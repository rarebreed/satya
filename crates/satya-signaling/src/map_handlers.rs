//! Tactical `1.0m` Map Generation, Token Movement, Wall-Collision Validation, and `GM_ONLY`
//! Visibility Filtering (`GET/POST /api/sessions/:session_id/map`, `POST /api/sessions/:session_id/map/move`).

use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use satya_engine::map_dressing::{place_prop_on_grid, remove_prop_at_on_grid};
use satya_engine::tactical_map::{
    SectorPreset, build_procedural_map_request, generate_procedural_sector,
};
use satya_schema::{
    CanvasEventType, EdgeSide, GenerateSessionMapRequest, GmMapUpdateRequest, MapWallEdge,
    MaterialClass, MessageVisibility, MoveMapTokenRequest, Participant, ParticipantKind,
    ServerWsPacket, SessionRole, TabletopCanvasEvent, TacticalTileKind, TokenPlacement,
    TokenVisualSpec, UpdateCombatTrackerRequest, WallEdgeKind, sanitize_portrait_url, subjects,
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

fn find_unblocked_spawn_cell(
    tiles: &[TacticalTileKind],
    width: i32,
    height: i32,
    preferred_x: i32,
    preferred_y: i32,
    occupied: &[(i32, i32)],
) -> (i32, i32) {
    let px = preferred_x.clamp(0, width - 1);
    let py = preferred_y.clamp(0, height - 1);
    let is_free = |gx: i32, gy: i32| -> bool {
        if gx < 0 || gy < 0 || gx >= width || gy >= height {
            return false;
        }
        let idx = (gy * width + gx) as usize;
        let tile = tiles.get(idx).copied().unwrap_or(TacticalTileKind::Floor);
        !tile.blocks_occupancy() && !occupied.contains(&(gx, gy))
    };

    if is_free(px, py) {
        return (px, py);
    }
    for radius in 1..=width.max(height) {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs().max(dy.abs()) != radius {
                    continue;
                }
                if is_free(px + dx, py + dy) {
                    return (px + dx, py + dy);
                }
            }
        }
    }
    (px, py)
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

/// Generates a procedural `1.0m` tactical sector map or edits/toggles a cell at `POST /api/sessions/:session_id/map`.
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

    // Branch A0b: GM prop placement or removal on active map (`edit_prop` / `clear_prop_at`)
    if req.edit_prop.is_some() || req.clear_prop_at.is_some() {
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
        let expected = (map.grid_width.max(1) * map.grid_height.max(1)) as usize;
        if map.tiles.len() != expected {
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
            map.tiles = sec.tiles;
            if map.wall_edges.is_empty() {
                map.wall_edges = sec.wall_edges;
            }
            if map.props.is_empty() {
                map.props = sec.props;
            }
        }
        if let Some((cx, cy)) = req.clear_prop_at {
            if cx < 0 || cy < 0 || cx >= map.grid_width || cy >= map.grid_height {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Cell ({cx}, {cy}) out of grid bounds") })),
                )
                    .into_response();
            }
            remove_prop_at_on_grid(
                &mut map.tiles,
                &mut map.props,
                map.grid_width,
                map.grid_height,
                cx,
                cy,
            );
        } else if let Some(prop) = req.edit_prop {
            let occupied: Vec<(i32, i32)> = map.tokens.iter().map(|t| (t.x, t.y)).collect();
            if let Err(msg) = place_prop_on_grid(
                &mut map.tiles,
                &mut map.props,
                &occupied,
                map.grid_width,
                map.grid_height,
                prop,
            ) {
                return (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response();
            }
        }
        let _ = state
            .auth
            .set_session_active_map(&session_id, map.clone())
            .await;
        if let Ok(canvas_meta) = state.storage.registry().build_metadata(
            state.next_seq(),
            subjects::TABLETOP_CANVAS,
            1,
            &session_id,
            ctx.session_mode,
        ) {
            let ev = TabletopCanvasEvent {
                metadata: canvas_meta,
                canvas_event_type: CanvasEventType::MapUpdated,
                actor: caller_part.clone(),
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
            build_procedural_map_request(
                &format!("map-{}", Uuid::now_v7()),
                p,
                req.title.as_deref(),
                req.grid_width.clamp(8, 40),
                req.grid_height.clamp(8, 40),
                req.seed.unwrap_or(42),
            )
        });
        let expected = (map.grid_width.max(1) * map.grid_height.max(1)) as usize;
        if map.tiles.len() != expected {
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
            map.tiles = sec.tiles;
            if map.wall_edges.is_empty() {
                map.wall_edges = sec.wall_edges;
            }
            if map.props.is_empty() {
                map.props = sec.props;
            }
        }
        if ex < 0 || ey < 0 || ex >= map.grid_width || ey >= map.grid_height {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Cell ({ex}, {ey}) out of grid bounds") })),
            )
                .into_response();
        }
        let idx = (ey * map.grid_width + ex) as usize;
        let current_tile = map.tiles[idx];
        let is_door_toggle = matches!(
            current_tile,
            TacticalTileKind::DoorClosed | TacticalTileKind::DoorOpen
        ) && matches!(
            req.set_tile_kind,
            None | Some(TacticalTileKind::DoorClosed | TacticalTileKind::DoorOpen)
        );
        if !caller_is_gm_or_solo && !is_door_toggle {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Only the Game Master can paint non-door terrain tiles" })),
            )
                .into_response();
        }
        let next_tile = req.set_tile_kind.unwrap_or(match current_tile {
            TacticalTileKind::DoorClosed => TacticalTileKind::DoorOpen,
            TacticalTileKind::DoorOpen => TacticalTileKind::DoorClosed,
            other => other,
        });
        map.tiles[idx] = next_tile;
        let _ = state
            .auth
            .set_session_active_map(&session_id, map.clone())
            .await;

        if let Ok(canvas_meta) = state.storage.registry().build_metadata(
            state.next_seq(),
            subjects::TABLETOP_CANVAS,
            1,
            &session_id,
            ctx.session_mode,
        ) {
            let ev = TabletopCanvasEvent {
                metadata: canvas_meta,
                canvas_event_type: CanvasEventType::MapUpdated,
                actor: caller_part.clone(),
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
    let width = req.grid_width.clamp(8, 40);
    let height = req.grid_height.clamp(8, 40);
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

    if req.populate_roster_tokens {
        map.tokens.clear();
        map.token_visuals.clear();
        let mut occupied: Vec<(i32, i32)> = Vec::new();
        let mid_y = height / 2;
        for (idx, p) in ctx.roster.iter().enumerate() {
            if p.role == SessionRole::AiGm || p.role == SessionRole::Observer {
                continue;
            }
            let pref_y = (mid_y - 2 + (idx as i32) * 2).clamp(1, height - 2);
            let (sx, sy) = find_unblocked_spawn_cell(&map.tiles, width, height, 2, pref_y, &occupied);
            occupied.push((sx, sy));
            map.tokens.push(TokenPlacement {
                entity_id: p.user_id.clone(),
                label: p.display_name.clone(),
                x: sx,
                y: sy,
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
            });
        }
        let (hx, hy) =
            find_unblocked_spawn_cell(&map.tiles, width, height, width - 3, mid_y, &occupied);
        map.tokens.push(TokenPlacement {
            entity_id: "npc-secops-1".to_string(),
            label: "SecOps Sentry".to_string(),
            x: hx,
            y: hy,
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
        });
    } else if let Some(prev) = ctx.active_map.as_ref() {
        let mut occupied = Vec::new();
        for mut tok in prev.tokens.clone() {
            let (nx, ny) =
                find_unblocked_spawn_cell(&map.tiles, width, height, tok.x, tok.y, &occupied);
            occupied.push((nx, ny));
            tok.x = nx;
            tok.y = ny;
            map.tokens.push(tok);
        }
        map.token_visuals = prev.token_visuals.clone();
    }

    let _ = state
        .auth
        .set_session_active_map(&session_id, map.clone())
        .await;

    if let Ok(canvas_meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TABLETOP_CANVAS,
        1,
        &session_id,
        ctx.session_mode,
    ) {
        let ev = TabletopCanvasEvent {
            metadata: canvas_meta,
            canvas_event_type: if ctx.active_map.is_some() {
                CanvasEventType::MapUpdated
            } else {
                CanvasEventType::MapGenerated
            },
            actor: caller_part.clone(),
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

    broadcast_role_filtered_map(&state, &session_id, &ctx.roster, &map).await;
    (
        StatusCode::OK,
        Json(filter_map_for_role(&map, caller_part.role)),
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
    let expected = (map.grid_width.max(1) * map.grid_height.max(1)) as usize;
    if map.tiles.len() != expected {
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
        map.tiles = sec.tiles;
        if map.wall_edges.is_empty() {
            map.wall_edges = sec.wall_edges;
        }
        if map.props.is_empty() {
            map.props = sec.props;
        }
    }

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
        if req.to_x < 0
            || req.to_y < 0
            || req.to_x >= map.grid_width
            || req.to_y >= map.grid_height
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!(
                        "Destination ({}, {}) is outside the {}x{} tactical grid",
                        req.to_x, req.to_y, map.grid_width, map.grid_height
                    )
                })),
            )
                .into_response();
        }
        let dest_tile = map.tile_at(req.to_x, req.to_y);
        if dest_tile.blocks_occupancy() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": format!(
                        "Destination ({}, {}) is blocked by {}",
                        req.to_x,
                        req.to_y,
                        dest_tile.label()
                    )
                })),
            )
                .into_response();
        }

        let requested_vis = req.visibility.unwrap_or(MessageVisibility::Public);
        let eff_vis = if can_role_see_gm_only_tokens(caller_part.role) {
            requested_vis
        } else {
            MessageVisibility::Public
        };

        let mut auto_facing: Option<i32> = req.facing_deg;
        if let Some(tok) = map.tokens.iter_mut().find(|t| t.entity_id == req.entity_id) {
            if auto_facing.is_none() && (tok.x != req.to_x || tok.y != req.to_y) {
                let dx = (req.to_x - tok.x) as f32;
                let dy = (req.to_y - tok.y) as f32;
                auto_facing = Some(((dx.atan2(-dy).to_degrees().round() as i32) + 360) % 360);
            }
            tok.x = req.to_x;
            tok.y = req.to_y;
            if let Some(lbl) = req.label.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                tok.label = lbl.to_string();
            }
            if req.visibility.is_some() && can_role_see_gm_only_tokens(caller_part.role) {
                tok.visibility = eff_vis;
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
                x: req.to_x,
                y: req.to_y,
                visibility: eff_vis,
            });
        }

        if let Some(vis_spec) = map
            .token_visuals
            .iter_mut()
            .find(|v| v.entity_id == req.entity_id)
        {
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
                elevation_cm: 0,
                stance_pose: req.stance_pose.unwrap_or_else(|| "idle".to_string()),
                portrait_url: req.portrait_url.as_deref().and_then(sanitize_portrait_url),
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
                        grid_x: Some(req.to_x),
                        grid_y: Some(req.to_y),
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

    if let Ok(canvas_meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TABLETOP_CANVAS,
        1,
        &session_id,
        ctx.session_mode,
    ) {
        let ev = TabletopCanvasEvent {
            metadata: canvas_meta,
            canvas_event_type: CanvasEventType::TokenMoved,
            actor: caller_part.clone(),
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

    broadcast_role_filtered_map(&state, &session_id, &ctx.roster, &map).await;
    (
        StatusCode::OK,
        Json(filter_map_for_role(&map, caller_part.role)),
    )
        .into_response()
}
