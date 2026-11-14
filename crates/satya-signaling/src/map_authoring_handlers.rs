//! Tactical Map Procedural Sector Generation & Edge/Prop Authoring Handler
//! (`POST /api/sessions/:session_id/map`).
//!
//! Companion module to [`crate::map_handlers`], re-exported flatly so all router bindings
//! stay unchanged.

use crate::map_handlers::{
    broadcast_role_filtered_map, ensure_vector_map_ready, filter_map_for_role,
    find_unblocked_spawn_pos_cm, log_map_canvas_event,
};
use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use satya_engine::map_dressing::make_prop;
use satya_engine::tactical_map::{SectorPreset, build_procedural_map_request};
use satya_engine::tactical_map_geom::{place_prop_on_map, remove_prop_at_point};
use satya_schema::{
    CanvasEventType, EdgeSide, GenerateSessionMapRequest, MapWallEdge, MaterialClass,
    MessageVisibility, ParticipantKind, PropKind, SessionRole, TacticalTileKind, TokenPlacement,
    TokenVisualSpec, WallEdgeKind,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Generates a procedural `1.0m` tactical sector map or edits/toggles a wall edge, prop, or legacy cell
/// at `POST /api/sessions/:session_id/map`.
pub async fn session_generate_or_edit_map_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<GenerateSessionMapRequest>,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };

    let Some((ctx, _)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };
    let Some(caller_part) = ctx.roster.iter().find(|p| p.user_id == user.user_id).cloned() else {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Caller is not an active participant in this session" })),
        )
            .into_response();
    };

    let has_human_gm = ctx.roster.iter().any(|p| {
        p.participant_kind == ParticipantKind::Human
            && matches!(p.role, SessionRole::Gm | SessionRole::Host)
    });
    let caller_is_gm_or_solo =
        matches!(caller_part.role, SessionRole::Gm | SessionRole::Host) || !has_human_gm;

    // Branch A0: Single edge-wall edit (door toggle by anyone, or GM wall/window painting)
    if let Some(edge_ref) = req.edit_edge {
        let Some(mut map) = ctx.active_map.clone() else {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "No active map to edit" })),
            )
                .into_response();
        };
        ensure_vector_map_ready(&mut map);
        let in_bounds = match edge_ref.side {
            EdgeSide::North => {
                (0..map.grid_width).contains(&edge_ref.x) && (0..=map.grid_height).contains(&edge_ref.y)
            }
            EdgeSide::West => {
                (0..=map.grid_width).contains(&edge_ref.x) && (0..map.grid_height).contains(&edge_ref.y)
            }
        };
        if !in_bounds {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Edge out of grid bounds" })),
            )
                .into_response();
        }
        let existing = map
            .wall_edges
            .iter()
            .position(|e| e.x == edge_ref.x && e.y == edge_ref.y && e.side == edge_ref.side);
        let current_kind = existing.map(|i| map.wall_edges[i].kind);
        let is_door_toggle = !req.clear_edge
            && matches!(current_kind, Some(WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen))
            && matches!(
                req.set_edge_kind,
                None | Some(WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen)
            );
        if !caller_is_gm_or_solo && !is_door_toggle {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Only the Game Master can paint walls and windows" })),
            )
                .into_response();
        }
        let next_kind = if req.clear_edge {
            None
        } else {
            req.set_edge_kind.or(match current_kind {
                Some(WallEdgeKind::DoorClosed) => Some(WallEdgeKind::DoorOpen),
                Some(WallEdgeKind::DoorOpen) => Some(WallEdgeKind::DoorClosed),
                other => other,
            })
        };
        match (existing, next_kind) {
            (Some(i), Some(kind)) => map.wall_edges[i].kind = kind,
            (Some(i), None) => {
                map.wall_edges.remove(i);
            }
            (None, Some(kind)) => map.wall_edges.push(MapWallEdge {
                x: edge_ref.x,
                y: edge_ref.y,
                side: edge_ref.side,
                kind,
                material: if kind == WallEdgeKind::Window {
                    MaterialClass::Glass
                } else {
                    MaterialClass::Drywall
                },
            }),
            (None, None) => {}
        }
        let _ = state
            .auth
            .set_session_active_map(&session_id, map.clone())
            .await;
        broadcast_role_filtered_map(&state, &session_id, &ctx.roster, &map).await;
        return (
            StatusCode::OK,
            Json(filter_map_for_role(&map, caller_part.role)),
        )
            .into_response();
    }

    // Branch A0b: GM prop placement or removal on active map (`edit_prop` / `clear_prop_at` / `clear_prop_at_cm`)
    if req.edit_prop.is_some() || req.clear_prop_at.is_some() || req.clear_prop_at_cm.is_some() {
        let Some(mut map) = ctx.active_map.clone() else {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "No active map to edit" })),
            )
                .into_response();
        };
        if !caller_is_gm_or_solo {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Only the Game Master can place or remove map props" })),
            )
                .into_response();
        }
        ensure_vector_map_ready(&mut map);
        let (board_w_cm, board_h_cm) = map.board_size_cm();
        let (board_w_m, board_h_m) = map.board_size_m();
        if let Some((cx_cm, cy_cm)) = req.clear_prop_at_cm {
            if cx_cm < 0 || cy_cm < 0 || cx_cm >= board_w_cm || cy_cm >= board_h_cm {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Point ({cx_cm}cm, {cy_cm}cm) out of map bounds") })),
                )
                    .into_response();
            }
            remove_prop_at_point(&mut map.props, cx_cm as f32 / 100.0, cy_cm as f32 / 100.0);
        } else if let Some((cx, cy)) = req.clear_prop_at {
            if cx < 0 || cy < 0 || cx >= map.grid_width || cy >= map.grid_height {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Cell ({cx}, {cy}) out of grid bounds") })),
                )
                    .into_response();
            }
            remove_prop_at_point(&mut map.props, cx as f32 + 0.5, cy as f32 + 0.5);
        } else if let Some(prop) = req.edit_prop {
            let occupied_m: Vec<(f32, f32)> =
                map.tokens.iter().map(|t| map.token_pos_m(t)).collect();
            if let Err(msg) = place_prop_on_map(
                &mut map.props,
                &map.wall_edges,
                &occupied_m,
                (board_w_m, board_h_m),
                prop,
            ) {
                return (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response();
            }
        }
        let _ = state
            .auth
            .set_session_active_map(&session_id, map.clone())
            .await;
        log_map_canvas_event(
            &state,
            &session_id,
            ctx.session_mode,
            CanvasEventType::MapUpdated,
            &caller_part,
            &map,
        )
        .await;
        broadcast_role_filtered_map(&state, &session_id, &ctx.roster, &map).await;
        return (
            StatusCode::OK,
            Json(filter_map_for_role(&map, caller_part.role)),
        )
            .into_response();
    }

    // Branch A: Single-cell edit (door toggle or terrain paint)
    if let (Some(ex), Some(ey)) = (req.edit_cell_x, req.edit_cell_y) {
        let mut map = ctx.active_map.clone().unwrap_or_else(|| {
            let p = SectorPreset::from_code_str(&req.preset).unwrap_or(SectorPreset::OrbitalAirlock);
            let (raw_w, raw_h) = req
                .scale
                .map(|s| p.resolve_scale_dimensions(s))
                .unwrap_or((req.grid_width, req.grid_height));
            build_procedural_map_request(
                &format!("map-{}", Uuid::now_v7()),
                p,
                req.title.as_deref(),
                raw_w.clamp(
                    satya_engine::tactical_map::MIN_SECTOR_DIM,
                    satya_engine::tactical_map::MAX_SECTOR_DIM,
                ),
                raw_h.clamp(
                    satya_engine::tactical_map::MIN_SECTOR_DIM,
                    satya_engine::tactical_map::MAX_SECTOR_DIM,
                ),
                req.seed.unwrap_or(42),
            )
        });
        ensure_vector_map_ready(&mut map);
        if ex < 0 || ey < 0 || ex >= map.grid_width || ey >= map.grid_height {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Cell ({ex}, {ey}) out of grid bounds") })),
            )
                .into_response();
        }
        let adjacent_door = map.wall_edges.iter().position(|e| {
            matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen)
                && ((e.side == EdgeSide::North && e.x == ex && (e.y == ey || e.y == ey + 1))
                    || (e.side == EdgeSide::West && (e.x == ex || e.x == ex + 1) && e.y == ey))
        });
        let is_door_toggle = adjacent_door.is_some()
            && matches!(
                req.set_tile_kind,
                None | Some(TacticalTileKind::DoorClosed | TacticalTileKind::DoorOpen)
            );
        if !caller_is_gm_or_solo && !is_door_toggle {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Only the Game Master can paint non-door terrain props" })),
            )
                .into_response();
        }
        if let Some(door_idx) = adjacent_door
            && matches!(
                req.set_tile_kind,
                None | Some(TacticalTileKind::DoorClosed | TacticalTileKind::DoorOpen)
            )
        {
            map.wall_edges[door_idx].kind = match req.set_tile_kind {
                Some(TacticalTileKind::DoorClosed) => WallEdgeKind::DoorClosed,
                Some(TacticalTileKind::DoorOpen) => WallEdgeKind::DoorOpen,
                _ => match map.wall_edges[door_idx].kind {
                    WallEdgeKind::DoorClosed => WallEdgeKind::DoorOpen,
                    WallEdgeKind::DoorOpen => WallEdgeKind::DoorClosed,
                    other => other,
                },
            };
        } else if let Some(kind) = req.set_tile_kind {
            let (board_w_m, board_h_m) = map.board_size_m();
            let occupied_m: Vec<(f32, f32)> =
                map.tokens.iter().map(|t| map.token_pos_m(t)).collect();
            let seed = (ex * 73 + ey * 139) as u32;
            match kind {
                TacticalTileKind::Floor => {
                    remove_prop_at_point(&mut map.props, ex as f32 + 0.5, ey as f32 + 0.5);
                }
                TacticalTileKind::HalfCover => {
                    let _ = place_prop_on_map(
                        &mut map.props,
                        &map.wall_edges,
                        &occupied_m,
                        (board_w_m, board_h_m),
                        make_prop(PropKind::WoodCrate, ex, ey, 0, None, seed),
                    );
                }
                TacticalTileKind::FullWall => {
                    let _ = place_prop_on_map(
                        &mut map.props,
                        &map.wall_edges,
                        &occupied_m,
                        (board_w_m, board_h_m),
                        make_prop(PropKind::StructuralPillar, ex, ey, 0, None, seed),
                    );
                }
                TacticalTileKind::HazardZone => {
                    let _ = place_prop_on_map(
                        &mut map.props,
                        &map.wall_edges,
                        &occupied_m,
                        (board_w_m, board_h_m),
                        make_prop(PropKind::HazardSpill, ex, ey, 0, None, seed),
                    );
                }
                TacticalTileKind::ElevatedPlatform => {
                    let _ = place_prop_on_map(
                        &mut map.props,
                        &map.wall_edges,
                        &occupied_m,
                        (board_w_m, board_h_m),
                        make_prop(PropKind::ElevatedCatwalk, ex, ey, 0, None, seed),
                    );
                }
                _ => {}
            }
        }
        let _ = state
            .auth
            .set_session_active_map(&session_id, map.clone())
            .await;

        log_map_canvas_event(
            &state,
            &session_id,
            ctx.session_mode,
            CanvasEventType::MapUpdated,
            &caller_part,
            &map,
        )
        .await;
        broadcast_role_filtered_map(&state, &session_id, &ctx.roster, &map).await;
        return (
            StatusCode::OK,
            Json(filter_map_for_role(&map, caller_part.role)),
        )
            .into_response();
    }

    // Branch B2: Seamless edge-matched procedural continuation stitching onto active map
    if let Some(dir) = req.continue_direction {
        if !caller_is_gm_or_solo {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Only the Game Master can continue the tactical sector while a Human GM leads the session" })),
            )
                .into_response();
        }
        let mut base_map = ctx.active_map.clone().unwrap_or_else(|| {
            if req.preset.trim().is_empty() {
                satya_engine::tactical_map::default_starter_vector_map(req.seed.unwrap_or(1337))
            } else {
                let preset = SectorPreset::from_code_str(&req.preset).unwrap_or_default();
                build_procedural_map_request(
                    &format!("map-{}", preset.as_str()),
                    preset,
                    req.title.as_deref(),
                    satya_engine::tactical_map::CONTINUATION_STRIP_W,
                    satya_engine::tactical_map::CONTINUATION_STRIP_H,
                    req.seed.unwrap_or(1337),
                )
            }
        });
        ensure_vector_map_ready(&mut base_map);
        if base_map.theme_preset.is_none() && !req.preset.trim().is_empty() {
            base_map.theme_preset = Some(req.preset.clone());
        }
        let map = match satya_engine::tactical_map::continue_procedural_map(
            &base_map,
            dir,
            req.seed,
        ) {
            Ok(outcome) => outcome.map,
            Err(err) => {
                return (StatusCode::BAD_REQUEST, Json(json!({ "error": err }))).into_response();
            }
        };

        let _ = state
            .auth
            .set_session_active_map(&session_id, map.clone())
            .await;

        log_map_canvas_event(
            &state,
            &session_id,
            ctx.session_mode,
            CanvasEventType::MapUpdated,
            &caller_part,
            &map,
        )
        .await;

        broadcast_role_filtered_map(&state, &session_id, &ctx.roster, &map).await;
        return (
            StatusCode::OK,
            Json(filter_map_for_role(&map, caller_part.role)),
        )
            .into_response();
    }

    // Branch B: Full procedural `1.0m` sector generation
    if !caller_is_gm_or_solo {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the Game Master can generate a new sector map while a Human GM leads the session" })),
        )
            .into_response();
    }

    let preset =
        SectorPreset::from_code_str(&req.preset).unwrap_or(SectorPreset::OrbitalAirlock);
    let (raw_w, raw_h) = req
        .scale
        .map(|s| preset.resolve_scale_dimensions(s))
        .unwrap_or((req.grid_width, req.grid_height));
    let width = raw_w.clamp(
        satya_engine::tactical_map::MIN_SECTOR_DIM,
        satya_engine::tactical_map::MAX_SECTOR_DIM,
    );
    let height = raw_h.clamp(
        satya_engine::tactical_map::MIN_SECTOR_DIM,
        satya_engine::tactical_map::MAX_SECTOR_DIM,
    );
    let seed = req.seed.unwrap_or(42);
    let mut map = build_procedural_map_request(
        &format!("map-{}-{}", preset.as_str(), Uuid::now_v7()),
        preset,
        req.title.as_deref(),
        width,
        height,
        seed,
    );
    map.cell_size_cm = req.cell_size_cm.clamp(50, 250);
    let cell_cm = map.cell_size_cm;

    if req.populate_roster_tokens {
        map.tokens.clear();
        map.token_visuals.clear();
        let mut occupied_m: Vec<(f32, f32)> = Vec::new();
        let mid_y = height / 2;
        for (idx, p) in ctx.roster.iter().enumerate() {
            if p.role == SessionRole::AiGm || p.role == SessionRole::Observer {
                continue;
            }
            let pref_y = (mid_y - 2 + (idx as i32) * 2).clamp(1, height - 2);
            let (sx_cm, sy_cm) = find_unblocked_spawn_pos_cm(
                &map,
                2 * cell_cm + cell_cm / 2,
                pref_y * cell_cm + cell_cm / 2,
                &occupied_m,
            );
            occupied_m.push((sx_cm as f32 / 100.0, sy_cm as f32 / 100.0));
            map.tokens.push(TokenPlacement {
                entity_id: p.user_id.clone(),
                label: p.display_name.clone(),
                x: (sx_cm / cell_cm).clamp(0, width - 1),
                y: (sy_cm / cell_cm).clamp(0, height - 1),
                visibility: MessageVisibility::Public,
            });
            map.token_visuals.push(TokenVisualSpec {
                entity_id: p.user_id.clone(),
                model_preset: if idx % 2 == 0 {
                    "valkyrie_rig".to_string()
                } else {
                    "recon_mk4".to_string()
                },
                color_hex: "#38bdf8".to_string(),
                facing_deg: 90,
                elevation_cm: 0,
                stance_pose: "aimed".to_string(),
                portrait_url: None,
                pos_x_cm: Some(sx_cm),
                pos_y_cm: Some(sy_cm),
            });
        }
        let (hx_cm, hy_cm) = find_unblocked_spawn_pos_cm(
            &map,
            (width - 3) * cell_cm + cell_cm / 2,
            mid_y * cell_cm + cell_cm / 2,
            &occupied_m,
        );
        map.tokens.push(TokenPlacement {
            entity_id: "npc-secops-1".to_string(),
            label: "SecOps Sentry".to_string(),
            x: (hx_cm / cell_cm).clamp(0, width - 1),
            y: (hy_cm / cell_cm).clamp(0, height - 1),
            visibility: MessageVisibility::Public,
        });
        map.token_visuals.push(TokenVisualSpec {
            entity_id: "npc-secops-1".to_string(),
            model_preset: "secops_sentry".to_string(),
            color_hex: "#f87171".to_string(),
            facing_deg: 270,
            elevation_cm: 0,
            stance_pose: "aimed".to_string(),
            portrait_url: None,
            pos_x_cm: Some(hx_cm),
            pos_y_cm: Some(hy_cm),
        });
    } else if let Some(prev) = ctx.active_map.as_ref() {
        let mut occupied_m: Vec<(f32, f32)> = Vec::new();
        map.token_visuals = prev.token_visuals.clone();
        for mut tok in prev.tokens.clone() {
            let (px_cm, py_cm) = prev.token_pos_cm(&tok);
            let (nx_cm, ny_cm) =
                find_unblocked_spawn_pos_cm(&map, px_cm, py_cm, &occupied_m);
            occupied_m.push((nx_cm as f32 / 100.0, ny_cm as f32 / 100.0));
            tok.x = (nx_cm / cell_cm).clamp(0, width - 1);
            tok.y = (ny_cm / cell_cm).clamp(0, height - 1);
            let tid = tok.entity_id.clone();
            map.tokens.push(tok);
            map.set_token_pos_cm(&tid, nx_cm, ny_cm);
        }
    }

    let _ = state
        .auth
        .set_session_active_map(&session_id, map.clone())
        .await;

    log_map_canvas_event(
        &state,
        &session_id,
        ctx.session_mode,
        if ctx.active_map.is_some() {
            CanvasEventType::MapUpdated
        } else {
            CanvasEventType::MapGenerated
        },
        &caller_part,
        &map,
    )
    .await;

    broadcast_role_filtered_map(&state, &session_id, &ctx.roster, &map).await;
    (
        StatusCode::OK,
        Json(filter_map_for_role(&map, caller_part.role)),
    )
        .into_response()
}
