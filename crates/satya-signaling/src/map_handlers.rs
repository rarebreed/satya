//! Tactical `1.0m` Map Generation, Token Movement, Wall-Collision Validation, and `GM_ONLY`
//! Visibility Filtering (`GET/POST /api/sessions/:session_id/map`, `POST /api/sessions/:session_id/map/move`).

pub use crate::map_authoring_handlers::*;

use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use satya_engine::tactical_map::{
    SectorPreset, build_procedural_map_request, generate_procedural_sector,
};
use satya_engine::tactical_map_geom::{
    TOKEN_RADIUS_M, circle_collides_obstacles, migrate_legacy_tiles_to_vector,
    resolve_token_destination_cm,
};
use satya_schema::{
    CanvasEventType, GmMapUpdateRequest, MessageVisibility, MoveMapTokenRequest, Participant,
    ParticipantKind, ServerWsPacket, SessionMode, SessionRole, TabletopCanvasEvent, TokenPlacement,
    TokenVisualSpec, UpdateCombatTrackerRequest, sanitize_portrait_url, subjects,
};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

/// Returns `true` when `role` is authorized to view `MessageVisibility::GmOnly` map tokens.
#[must_use]
pub fn can_role_see_gm_only_tokens(role: SessionRole) -> bool {
    matches!(role, SessionRole::Gm | SessionRole::Host | SessionRole::AiGm)
}

/// Filters `MessageVisibility::GmOnly` tokens (and their `token_visuals`) from `map` when `role` is a non-GM player or observer.
#[must_use]
pub fn filter_map_for_role(map: &GmMapUpdateRequest, role: SessionRole) -> GmMapUpdateRequest {
    if can_role_see_gm_only_tokens(role) {
        return map.clone();
    }
    let mut filtered = map.clone();
    filtered
        .tokens
        .retain(|tok| tok.visibility != MessageVisibility::GmOnly);
    let visible_ids: HashSet<&str> = filtered
        .tokens
        .iter()
        .map(|t| t.entity_id.as_str())
        .collect();
    filtered
        .token_visuals
        .retain(|v| visible_ids.contains(v.entity_id.as_str()));
    filtered
}

/// Broadcasts `ServerWsPacket::MapUpdated` to connected sockets in `session_id`, delivering the full
/// map to `GM` / `Host` / `AiGm` sockets and stripping `MessageVisibility::GmOnly` tokens for `Player` / `Observer` sockets.
pub async fn broadcast_role_filtered_map(
    state: &SignalingAppState,
    session_id: &str,
    roster: &[Participant],
    map: &GmMapUpdateRequest,
) {
    let gm_ids: HashSet<String> = roster
        .iter()
        .filter(|p| can_role_see_gm_only_tokens(p.role))
        .map(|p| p.user_id.clone())
        .collect();

    let connected_ids: Vec<String> = {
        let guard = state.active_sockets.read().await;
        guard
            .get(session_id)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    };
    if connected_ids.is_empty() {
        return;
    }

    let mut gm_targets = Vec::new();
    let mut player_targets = Vec::new();
    for uid in connected_ids {
        if gm_ids.contains(&uid) {
            gm_targets.push(uid);
        } else {
            player_targets.push(uid);
        }
    }

    if !gm_targets.is_empty()
        && let Ok(gm_pkt) = serde_json::to_string(&ServerWsPacket::MapUpdated {
            map: Box::new(map.clone()),
        })
    {
        state.send_to_users(session_id, &gm_targets, &gm_pkt).await;
    }

    if !player_targets.is_empty() {
        let public_map = filter_map_for_role(map, SessionRole::Player);
        if let Ok(pub_pkt) = serde_json::to_string(&ServerWsPacket::MapUpdated {
            map: Box::new(public_map),
        }) {
            state
                .send_to_users(session_id, &player_targets, &pub_pkt)
                .await;
        }
    }
}

pub(crate) fn ensure_vector_map_ready(map: &mut GmMapUpdateRequest) {
    if map.cell_size_cm <= 0 {
        map.cell_size_cm = 100;
    }
    if !map.tiles.is_empty() {
        migrate_legacy_tiles_to_vector(
            &map.tiles,
            map.grid_width.max(1),
            map.grid_height.max(1),
            &mut map.wall_edges,
            &mut map.props,
        );
        map.tiles.clear();
        return;
    }
    if map.wall_edges.is_empty() && map.props.is_empty() && map.floors.is_empty() {
        let p = map
            .theme_preset
            .as_deref()
            .and_then(SectorPreset::from_code_str)
            .unwrap_or_default();
        let sec = generate_procedural_sector(
            p,
            map.grid_width.max(8),
            map.grid_height.max(8),
            map.seed.unwrap_or(42),
        );
        map.wall_edges = sec.wall_edges;
        map.props = sec.props;
        if map.surfaces.is_empty() {
            map.surfaces = sec.surfaces;
        }
        if map.floors.is_empty() && !sec.floors.is_empty() {
            map.floors = sec.floors;
            map.active_floor = sec.active_floor;
        }
    }
}

pub(crate) fn find_unblocked_spawn_pos_cm(
    map: &GmMapUpdateRequest,
    preferred_x_cm: i32,
    preferred_y_cm: i32,
    occupied_m: &[(f32, f32)],
) -> (i32, i32) {
    let cell_cm = map.cell_size_cm.max(25);
    let (board_w_m, board_h_m) = map.board_size_m();
    let pref_x_m = (preferred_x_cm as f32 / 100.0).clamp(0.45, (board_w_m - 0.45).max(0.45));
    let pref_y_m = (preferred_y_cm as f32 / 100.0).clamp(0.45, (board_h_m - 0.45).max(0.45));
    let is_free = |cx_m: f32, cy_m: f32| -> bool {
        !circle_collides_obstacles(
            board_w_m,
            board_h_m,
            &map.wall_edges,
            &map.props,
            cx_m,
            cy_m,
            TOKEN_RADIUS_M,
        ) && occupied_m
            .iter()
            .all(|&(ox, oy)| (cx_m - ox).hypot(cy_m - oy) >= TOKEN_RADIUS_M * 2.1)
    };
    if is_free(pref_x_m, pref_y_m) {
        return (
            (pref_x_m * 100.0).round() as i32,
            (pref_y_m * 100.0).round() as i32,
        );
    }
    for ring in 1..=(map.grid_width.max(map.grid_height) * 2) {
        let r_m = ring as f32 * 0.50;
        for angle_idx in 0..16 {
            let rad = (angle_idx as f32) * (std::f32::consts::TAU / 16.0);
            let cx_m = pref_x_m + r_m * rad.cos();
            let cy_m = pref_y_m + r_m * rad.sin();
            if is_free(cx_m, cy_m) {
                return (
                    (cx_m * 100.0).round() as i32,
                    (cy_m * 100.0).round() as i32,
                );
            }
        }
    }
    let gx = (preferred_x_cm / cell_cm).clamp(0, (map.grid_width - 1).max(0));
    let gy = (preferred_y_cm / cell_cm).clamp(0, (map.grid_height - 1).max(0));
    (gx * cell_cm + cell_cm / 2, gy * cell_cm + cell_cm / 2)
}

pub(crate) async fn log_map_canvas_event(
    state: &SignalingAppState,
    session_id: &str,
    session_mode: SessionMode,
    canvas_event_type: CanvasEventType,
    actor: &Participant,
    map: &GmMapUpdateRequest,
) {
    if let Ok(canvas_meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TABLETOP_CANVAS,
        1,
        session_id,
        session_mode,
    ) {
        let ev = TabletopCanvasEvent {
            metadata: canvas_meta,
            canvas_event_type,
            actor: actor.clone(),
            map_id: Some(map.map_id.clone()),
            map_title: Some(map.title.clone()),
            image_url: Some(map.image_url.clone()),
            grid_width: Some(map.grid_width),
            grid_height: Some(map.grid_height),
            token_placements: map.tokens.clone(),
            wgpu_dice_roll: None,
            terrain_notes: map.notes.clone(),
        };
        let _ = state.storage.log_tabletop_canvas(&ev).await;
    }
}

/// Returns the active role-filtered tactical map at `GET /api/sessions/:session_id/map`.
pub async fn get_session_map_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    let is_gm_service = crate::gm_api::verify_gm_service_header(&state, &headers).is_ok();
    let caller_user = if is_gm_service {
        None
    } else {
        let Some(u) = state.auth.verify_auth_token(auth_hdr).await else {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid or expired auth token" })),
            )
                .into_response();
        };
        Some(u)
    };

    let Some((ctx, _)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };

    let caller_role = if is_gm_service {
        SessionRole::AiGm
    } else if let Some(u) = caller_user.as_ref() {
        let Some(p) = ctx.roster.iter().find(|p| p.user_id == u.user_id) else {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Caller is not an active participant in this session" })),
            )
                .into_response();
        };
        p.role
    } else {
        SessionRole::Player
    };

    let Some(active_map) = ctx.active_map else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "No active tactical map in session" })),
        )
            .into_response();
    };

    (
        StatusCode::OK,
        Json(filter_map_for_role(&active_map, caller_role)),
    )
        .into_response()
}

/// Moves, spawns, recolors, or removes a miniature token at `POST /api/sessions/:session_id/map/move`,
/// enforcing grid bounds, wall/closed-door occupancy rules, and player vs. GM token ownership.
pub async fn session_move_map_token_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<MoveMapTokenRequest>,
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
    let owns_token = req.entity_id == user.user_id
        || caller_part
            .character_id
            .as_deref()
            .is_some_and(|cid| cid == req.entity_id);

    if !caller_is_gm_or_solo && !owns_token {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Players may only move their own character miniature while a Human GM leads the session" })),
        )
            .into_response();
    }

    let mut map = ctx.active_map.unwrap_or_else(|| {
        build_procedural_map_request(
            &format!("map-{}", Uuid::now_v7()),
            SectorPreset::OrbitalAirlock,
            None,
            20,
            14,
            42,
        )
    });
    ensure_vector_map_ready(&mut map);

    if req.remove {
        if !caller_is_gm_or_solo {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Only the Game Master can remove miniatures from the map" })),
            )
                .into_response();
        }
        map.tokens.retain(|t| t.entity_id != req.entity_id);
        map.token_visuals.retain(|v| v.entity_id != req.entity_id);
    } else {
        let (req_x_cm, req_y_cm) = req.target_cm(map.cell_size_cm);
        let (board_w_cm, board_h_cm) = map.board_size_cm();
        let (board_w_m, board_h_m) = map.board_size_m();
        if req_x_cm < 0 || req_y_cm < 0 || req_x_cm >= board_w_cm || req_y_cm >= board_h_cm {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!(
                        "Destination ({}cm, {}cm) is outside the {:.1}m x {:.1}m tactical map",
                        req_x_cm, req_y_cm, board_w_m, board_h_m
                    )
                })),
            )
                .into_response();
        }
        if let Some(tf) = req.to_floor
            && !map.has_floor(tf)
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Floor {tf} does not exist on active map") })),
            )
                .into_response();
        }
        let target_floor = req
            .to_floor
            .unwrap_or_else(|| map.token_floor(&req.entity_id));
        let (final_x_cm, final_y_cm) = match if target_floor == map.active_floor {
            resolve_token_destination_cm(&map, &req.entity_id, req_x_cm, req_y_cm)
        } else {
            let mut floor_view = map.clone();
            floor_view.switch_active_floor(target_floor);
            resolve_token_destination_cm(&floor_view, &req.entity_id, req_x_cm, req_y_cm)
        } {
            Ok(coords) => coords,
            Err(msg) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": msg })),
                )
                    .into_response();
            }
        };

        let cell_cm = map.cell_size_cm.max(25);
        let final_gx = (final_x_cm / cell_cm).clamp(0, (map.grid_width - 1).max(0));
        let final_gy = (final_y_cm / cell_cm).clamp(0, (map.grid_height - 1).max(0));

        let requested_vis = req.visibility.unwrap_or(MessageVisibility::Public);
        let eff_vis = if can_role_see_gm_only_tokens(caller_part.role) {
            requested_vis
        } else {
            MessageVisibility::Public
        };

        let mut auto_facing: Option<i32> = req.facing_deg;
        let existing_pos = map
            .tokens
            .iter()
            .find(|t| t.entity_id == req.entity_id)
            .map(|t| map.token_pos_cm(t));
        if let Some((prev_x_cm, prev_y_cm)) = existing_pos {
            if auto_facing.is_none() && (prev_x_cm != final_x_cm || prev_y_cm != final_y_cm) {
                let dx = (final_x_cm - prev_x_cm) as f32;
                let dy = (final_y_cm - prev_y_cm) as f32;
                auto_facing = Some(((dx.atan2(-dy).to_degrees().round() as i32) + 360) % 360);
            }
            if let Some(tok) = map.tokens.iter_mut().find(|t| t.entity_id == req.entity_id) {
                tok.x = final_gx;
                tok.y = final_gy;
                if let Some(lbl) = req.label.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    tok.label = lbl.to_string();
                }
                if req.visibility.is_some() && can_role_see_gm_only_tokens(caller_part.role) {
                    tok.visibility = eff_vis;
                }
            }
        } else {
            let lbl = req
                .label
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    if req.entity_id == user.user_id {
                        caller_part.display_name.as_str()
                    } else {
                        req.entity_id.as_str()
                    }
                })
                .to_string();
            map.tokens.push(TokenPlacement {
                entity_id: req.entity_id.clone(),
                label: lbl,
                x: final_gx,
                y: final_gy,
                visibility: eff_vis,
            });
        }

        let target_elev_cm = map.floor_elevation_cm(target_floor);
        if let Some(vis_spec) = map
            .token_visuals
            .iter_mut()
            .find(|v| v.entity_id == req.entity_id)
        {
            vis_spec.pos_x_cm = Some(final_x_cm);
            vis_spec.pos_y_cm = Some(final_y_cm);
            if req.to_floor.is_some() {
                vis_spec.floor_index = target_floor;
                vis_spec.elevation_cm = target_elev_cm;
            }
            if let Some(f) = auto_facing {
                vis_spec.facing_deg = f.rem_euclid(360);
            }
            if let Some(mp) = req
                .model_preset
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                vis_spec.model_preset = mp.to_string();
            }
            if let Some(ch) = req
                .color_hex
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                vis_spec.color_hex = ch.to_string();
            }
            if let Some(sp) = req
                .stance_pose
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                vis_spec.stance_pose = sp.to_string();
            }
            if let Some(raw_url) = req.portrait_url.as_deref() {
                vis_spec.portrait_url = if raw_url.trim().is_empty() {
                    None
                } else {
                    sanitize_portrait_url(raw_url)
                };
            }
        } else {
            map.token_visuals.push(TokenVisualSpec {
                entity_id: req.entity_id.clone(),
                model_preset: req
                    .model_preset
                    .unwrap_or_else(|| "valkyrie_rig".to_string()),
                color_hex: req.color_hex.unwrap_or_else(|| "#38bdf8".to_string()),
                facing_deg: auto_facing.unwrap_or(90).rem_euclid(360),
                elevation_cm: target_elev_cm,
                floor_index: target_floor,
                stance_pose: req.stance_pose.unwrap_or_else(|| "idle".to_string()),
                portrait_url: req.portrait_url.as_deref().and_then(sanitize_portrait_url),
                pos_x_cm: Some(final_x_cm),
                pos_y_cm: Some(final_y_cm),
            });
        }

        // Keep CombatTrackerState `(grid_x, grid_y)` synchronized if combatant is tracked
        if ctx
            .combat_tracker
            .as_ref()
            .is_some_and(|ct| ct.combatants.iter().any(|c| c.entity_id == req.entity_id))
        {
            let _ = state
                .auth
                .update_session_combat_tracker(
                    &session_id,
                    UpdateCombatTrackerRequest {
                        tick_delta: None,
                        combat_active: None,
                        entity_id: Some(req.entity_id.clone()),
                        active_stance: None,
                        next_action_tick: None,
                        fatigue_level: None,
                        wound_shock: None,
                        grid_x: Some(final_gx),
                        grid_y: Some(final_gy),
                        zone_label: None,
                    },
                )
                .await;
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
        CanvasEventType::TokenMoved,
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
