//! Continuous Tactical Map Click, Token Selection/Movement, Procedural Sector Generation,
//! and Server REST Synchronization (`map_3d_input`).
//!
//! Companion to [`crate::wasm_client::map_3d`], re-exported flatly so all `window.satya*`
//! bindings stay unchanged.

use crate::wasm_client::dom::{fetch_json, input_value, select_value, set_input_value};
use crate::wasm_client::map_3d::{
    apply_incoming_map_update, mark_map_wgpu_dirty, parse_movement_gait_code,
    refresh_map_dom_and_wgpu, resolve_token_visual, with_map_ui, with_map_ui_mut,
};
use crate::wasm_client::state::with_state;
use satya_engine::ballistics::lookup_firearm_preset;
use satya_engine::tactical_map::{SectorPreset, build_procedural_map_request};
use satya_engine::tactical_map_geom::{TOKEN_PICK_RADIUS_M, resolve_token_destination_cm};
use satya_schema::{
    GenerateSessionMapRequest, GmMapUpdateRequest, MessageVisibility, MoveMapTokenRequest,
    TokenPlacement, TokenVisualSpec,
};
use wasm_bindgen::prelude::*;

/// Handles clicking a `(gx, gy)` grid cell in either the 2D SVG fallback or JS automation.
#[wasm_bindgen]
pub fn satya_click_map_cell(gx: i32, gy: i32) {
    handle_map_grid_cell_clicked(gx, gy);
}

/// Selects a token miniature by `entity_id` and updates movement reach and LoS overlays.
#[wasm_bindgen]
pub fn satya_select_map_token(entity_id: String) {
    with_map_ui_mut(|ui| {
        if ui.map.tokens.iter().any(|t| t.entity_id == entity_id) {
            ui.selected_entity_id = Some(entity_id);
            ui.recompute_overlays();
        }
    });
    mark_map_wgpu_dirty();
    refresh_map_dom_and_wgpu();
}

/// Generates a new procedural continuous vector tactical sector from `#map-preset-select` (syncing if in a session).
#[wasm_bindgen]
pub fn satya_generate_procedural_map() {
    let raw_preset = select_value("map-preset-select");
    let preset = SectorPreset::from_code_str(&raw_preset).unwrap_or_default();

    let seed = with_map_ui_mut(|ui| {
        ui.seed_counter = ui.seed_counter.wrapping_add(137);
        let s = ui.seed_counter;
        let mut built = build_procedural_map_request(
            &format!("map-{}-{s}", preset.as_str()),
            preset,
            None,
            20,
            14,
            s,
        );
        // Bind live session roster participants if connected
        let roster = with_state(|st| st.roster.clone());
        if !roster.is_empty() {
            built.tokens.clear();
            built.token_visuals.clear();
            let mid_y = built.grid_height / 2;
            for (idx, p) in roster.iter().enumerate() {
                if p.participant_kind == satya_schema::ParticipantKind::AiAgent
                    && p.role == satya_schema::SessionRole::AiGm
                {
                    continue;
                }
                let pref_y_cm = ((mid_y - 2 + (idx as i32) * 2).clamp(1, built.grid_height - 2)) * 100 + 50;
                let (px_cm, py_cm) = resolve_token_destination_cm(&built, &p.user_id, 250, pref_y_cm)
                    .unwrap_or((250, pref_y_cm));
                built.tokens.push(TokenPlacement {
                    entity_id: p.user_id.clone(),
                    label: p.display_name.clone(),
                    x: px_cm / 100,
                    y: py_cm / 100,
                    visibility: MessageVisibility::Public,
                });
                built.token_visuals.push(TokenVisualSpec {
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
                    pos_x_cm: Some(px_cm),
                    pos_y_cm: Some(py_cm),
                });
            }
            let npc_pref_x = (built.grid_width - 3) * 100 + 50;
            let npc_pref_y = mid_y * 100 + 50;
            let (nx_cm, ny_cm) =
                resolve_token_destination_cm(&built, "npc-secops-1", npc_pref_x, npc_pref_y)
                    .unwrap_or((npc_pref_x, npc_pref_y));
            built.tokens.push(TokenPlacement {
                entity_id: "npc-secops-1".to_string(),
                label: "SecOps Sentry".to_string(),
                x: nx_cm / 100,
                y: ny_cm / 100,
                visibility: MessageVisibility::Public,
            });
            built.token_visuals.push(TokenVisualSpec {
                entity_id: "npc-secops-1".to_string(),
                model_preset: "secops_sentry".to_string(),
                color_hex: "#f87171".to_string(),
                facing_deg: 270,
                elevation_cm: 0,
                stance_pose: "aimed".to_string(),
                portrait_url: None,
                pos_x_cm: Some(nx_cm),
                pos_y_cm: Some(ny_cm),
            });
        }
        ui.camera_2d.reset_for_board(built.board_size_m());
        ui.pending_stitch_nudge = None;
        ui.continuation_in_flight = false;
        ui.map = built;
        ui.hovered_point_m = None;
        ui.hover_inspection = None;
        ui.recompute_overlays();
        s
    });

    mark_map_wgpu_dirty();
    refresh_map_dom_and_wgpu();
    post_generate_map_if_in_session(preset.as_str(), seed);
}

/// Continues the active tactical arena seamlessly past the given cardinal boundary (`NORTH`/`SOUTH`/`EAST`/`WEST`).
pub fn continue_session_map_in_direction(dir: satya_schema::MapContinuationDir) {
    if !crate::wasm_client::map_2d_camera::local_user_can_author_map() {
        return;
    }
    let (auth_token, session_id) = with_state(|st| (st.auth_token.clone(), st.session_id.clone()));
    let has_session = auth_token.is_some() && session_id.is_some();

    let next_action = with_map_ui_mut(|ui| {
        if ui.continuation_in_flight || !satya_engine::tactical_map::can_continue_map(&ui.map, dir) {
            return None;
        }
        ui.seed_counter = ui.seed_counter.wrapping_add(193);
        let seed = ui.seed_counter;
        let preset = ui
            .map
            .theme_preset
            .clone()
            .unwrap_or_else(|| SectorPreset::default().as_str().to_string());
        if has_session {
            ui.continuation_in_flight = true;
            ui.pending_stitch_nudge = Some(dir);
            Some((seed, preset, None))
        } else {
            let outcome =
                satya_engine::tactical_map::continue_procedural_map(&ui.map, dir, Some(seed))
                    .ok()?;
            ui.pending_stitch_nudge = Some(dir);
            Some((seed, preset, Some(outcome.map)))
        }
    });

    let Some((seed, preset, offline_map)) = next_action else {
        return;
    };
    if let Some(stitched) = offline_map {
        apply_incoming_map_update(&stitched);
        return;
    }

    crate::wasm_client::map_2d_camera::sync_map_camera_and_edge_controls_dom();
    let (Some(token), Some(sid)) = (auth_token, session_id) else {
        return;
    };
    let req = GenerateSessionMapRequest {
        preset,
        seed: Some(seed),
        continue_direction: Some(dir),
        ..Default::default()
    };
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_json::<_, GmMapUpdateRequest>(
            "POST",
            &format!("/api/sessions/{sid}/map"),
            Some(&token),
            Some(&req),
        )
        .await
        {
            Ok(updated) => apply_incoming_map_update(&updated),
            Err(_) => {
                with_map_ui_mut(|ui| {
                    ui.continuation_in_flight = false;
                    ui.pending_stitch_nudge = None;
                });
                crate::wasm_client::map_2d_camera::sync_map_camera_and_edge_controls_dom();
            }
        }
    });
}

pub(crate) fn post_generate_map_if_in_session(preset_code: &str, seed: i64) {
    let (auth_token, session_id) = with_state(|st| (st.auth_token.clone(), st.session_id.clone()));
    let (Some(token), Some(sid)) = (auth_token, session_id) else {
        return;
    };
    let req = GenerateSessionMapRequest {
        preset: preset_code.to_string(),
        title: None,
        grid_width: 20,
        grid_height: 14,
        cell_size_cm: 100,
        seed: Some(seed),
        populate_roster_tokens: true,
        ..Default::default()
    };
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(updated) = fetch_json::<_, GmMapUpdateRequest>(
            "POST",
            &format!("/api/sessions/{sid}/map"),
            Some(&token),
            Some(&req),
        )
        .await
        {
            apply_incoming_map_update(&updated);
        }
    });
}

fn post_move_token_if_in_session(req: MoveMapTokenRequest) {
    let (auth_token, session_id) = with_state(|st| (st.auth_token.clone(), st.session_id.clone()));
    let (Some(token), Some(sid)) = (auth_token, session_id) else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(updated) = fetch_json::<_, GmMapUpdateRequest>(
            "POST",
            &format!("/api/sessions/{sid}/map/move"),
            Some(&token),
            Some(&req),
        )
        .await
        {
            apply_incoming_map_update(&updated);
        }
    });
}

/// Updates weapon (`#map-weapon-select`), movement gait (`#map-gait-select`), tick budget (`#map-ticks-input`),
/// or click mode (`#map-click-mode`).
#[wasm_bindgen]
pub fn satya_update_map_tactical_controls() {
    let wpn_str = select_value("map-weapon-select");
    let gait_str = select_value("map-gait-select");
    let mode_str = select_value("map-click-mode");
    let ticks = input_value("map-ticks-input")
        .parse::<u32>()
        .unwrap_or(8)
        .clamp(1, 40);
    with_map_ui_mut(|ui| {
        if !wpn_str.is_empty() && lookup_firearm_preset(&wpn_str).is_some() {
            ui.selected_weapon_id = wpn_str;
        }
        if !gait_str.is_empty() {
            ui.gait = parse_movement_gait_code(&gait_str);
        }
        if !mode_str.is_empty() {
            ui.interaction_mode = mode_str;
        }
        ui.tick_budget = ticks;
        ui.recompute_overlays();
    });
    mark_map_wgpu_dirty();
    refresh_map_dom_and_wgpu();
}

/// Updates the `.glb` miniature preset or faction hex color of the currently selected map token.
#[wasm_bindgen]
pub fn satya_update_selected_token_visual() {
    let model_preset = select_value("map-token-model-select");
    let color_hex = input_value("map-token-color");
    let mut sync_req: Option<MoveMapTokenRequest> = None;

    with_map_ui_mut(|ui| {
        let Some(sel_id) = ui.selected_entity_id.clone() else {
            return;
        };
        let Some(tok) = ui.map.tokens.iter().find(|t| t.entity_id == sel_id).cloned() else {
            return;
        };
        let p_eff = if model_preset.is_empty() {
            "valkyrie_rig".to_string()
        } else {
            model_preset.clone()
        };
        let c_eff = if color_hex.is_empty() {
            "#38bdf8".to_string()
        } else {
            color_hex.clone()
        };
        let mut existing_portrait: Option<String> = None;
        let (px_cm, py_cm) = ui.map.token_pos_cm(&tok);
        if let Some(vis) = ui
            .map
            .token_visuals
            .iter_mut()
            .find(|v| v.entity_id == sel_id)
        {
            vis.model_preset = p_eff.clone();
            vis.color_hex = c_eff.clone();
            vis.pos_x_cm = Some(px_cm);
            vis.pos_y_cm = Some(py_cm);
            existing_portrait = vis.portrait_url.clone();
        } else {
            ui.map.token_visuals.push(TokenVisualSpec {
                entity_id: sel_id.clone(),
                model_preset: p_eff.clone(),
                color_hex: c_eff.clone(),
                facing_deg: 90,
                elevation_cm: 0,
                stance_pose: "idle".to_string(),
                portrait_url: None,
                pos_x_cm: Some(px_cm),
                pos_y_cm: Some(py_cm),
            });
        }
        sync_req = Some(MoveMapTokenRequest {
            entity_id: sel_id,
            label: Some(tok.label),
            to_x: tok.x,
            to_y: tok.y,
            to_x_cm: Some(px_cm),
            to_y_cm: Some(py_cm),
            facing_deg: None,
            model_preset: Some(p_eff),
            color_hex: Some(c_eff),
            stance_pose: None,
            portrait_url: existing_portrait,
            visibility: Some(tok.visibility),
            remove: false,
        });
    });

    mark_map_wgpu_dirty();
    refresh_map_dom_and_wgpu();
    if let Some(req) = sync_req {
        post_move_token_if_in_session(req);
    }
}

/// Pushes the active map Cover / Concealment / Overmatch $\Delta_{\text{ext}}$ (`+0` Clear, `+2` Overmatched
/// Furniture, `+3` Concealment, `+4` Low Cover, `+6` High Cover / Penetrable Blind Fire, `+16` Blocked)
/// directly into `#ttrpg-external-mod` in the Center-Bottom `2d20` Task Resolution drawer.
#[wasm_bindgen]
pub fn satya_apply_map_cover_to_roller() {
    let delta_ext = with_map_ui(|ui| {
        ui.last_ballistic
            .as_ref()
            .map(|b| b.effective_delta_ext)
            .or_else(|| ui.last_los.as_ref().map(|l| l.recommended_delta_ext))
            .unwrap_or(0)
            .clamp(-10, 16)
    });
    set_input_value("ttrpg-external-mod", &delta_ext.to_string());
    crate::wasm_client::chat_and_ttrpg::satya_update_cascade_preview();
}

/// Handles clicking a `(gx, gy)` cell center by forwarding into continuous [`handle_map_point_clicked`].
pub fn handle_map_grid_cell_clicked(gx: i32, gy: i32) {
    let (xm, ym) = with_map_ui(|ui| {
        let cell_m = ui.map.cell_size_m().max(0.25);
        ((gx as f32 + 0.5) * cell_m, (gy as f32 + 0.5) * cell_m)
    });
    handle_map_point_clicked(xm, ym);
}

/// Handles point-and-click at metric `(x_m, y_m)` across `move_or_select`, `measure_los`, and `edit_terrain`.
pub fn handle_map_point_clicked(x_m: f32, y_m: f32) {
    let is_edit = with_map_ui(|ui| ui.interaction_mode == "edit_terrain");
    if is_edit {
        crate::wasm_client::map_edge_edit::satya_click_map_point(x_m, y_m);
        return;
    }

    let mut move_req: Option<MoveMapTokenRequest> = None;
    let mut door_sync: Option<(String, satya_schema::MapEdgeRef, satya_schema::WallEdgeKind)> = None;

    with_map_ui_mut(|ui| {
        let (bw_m, bh_m) = ui.map.board_size_m();
        if !x_m.is_finite() || !y_m.is_finite() || x_m < 0.0 || y_m < 0.0 || x_m >= bw_m || y_m >= bh_m {
            return;
        }
        ui.ensure_vector_scene_populated();
        let cell_m = ui.map.cell_size_m().max(0.25);
        let cell = (
            (x_m / cell_m).floor().clamp(0.0, (ui.map.grid_width - 1) as f32) as i32,
            (y_m / cell_m).floor().clamp(0.0, (ui.map.grid_height - 1) as f32) as i32,
        );

        if ui.interaction_mode == "measure_los" {
            ui.hovered_point_m = Some((x_m, y_m));
            ui.hovered_cell = Some(cell);
            ui.hover_inspection =
                crate::wasm_client::map_hover_inspect::compute_hover_inspection(ui, x_m, y_m);
            ui.recompute_overlays();
            return;
        }

        // 1. Clicking near an existing token disc selects it
        if let Some((clicked_id, _)) = ui
            .map
            .tokens
            .iter()
            .filter_map(|t| {
                let (px, py) = ui.map.token_pos_m(t);
                let d = (px - x_m).hypot(py - y_m);
                (d <= TOKEN_PICK_RADIUS_M).then(|| (t.entity_id.clone(), d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
        {
            ui.selected_entity_id = Some(clicked_id);
            ui.recompute_overlays();
            return;
        }

        // 2. Clicking directly on a DoorClosed/DoorOpen wall edge toggles the door
        if let Some(edge_ref) = crate::wasm_client::map_edge_edit::nearest_edge_at(
            x_m / cell_m,
            y_m / cell_m,
            ui.map.grid_width,
            ui.map.grid_height,
            crate::wasm_client::map_edge_edit::EDGE_PICK_TOLERANCE_CELLS,
        ) && let Some(edge) = ui
            .map
            .wall_edges
            .iter_mut()
            .find(|e| e.x == edge_ref.x && e.y == edge_ref.y && e.side == edge_ref.side)
            && matches!(
                edge.kind,
                satya_schema::WallEdgeKind::DoorClosed | satya_schema::WallEdgeKind::DoorOpen
            )
        {
            edge.kind = if edge.kind == satya_schema::WallEdgeKind::DoorClosed {
                satya_schema::WallEdgeKind::DoorOpen
            } else {
                satya_schema::WallEdgeKind::DoorClosed
            };
            let next_kind = edge.kind;
            ui.recompute_overlays();
            let preset_str = ui
                .map
                .theme_preset
                .clone()
                .unwrap_or_else(|| "orbital_airlock".to_string());
            door_sync = Some((preset_str, edge_ref, next_kind));
            return;
        }

        // 3. Continuous point-and-click movement with body-disc slide/snap around walls & props
        let Some(sel_id) = ui.selected_entity_id.clone() else {
            return;
        };
        let target_x_cm = (x_m * 100.0).round() as i32;
        let target_y_cm = (y_m * 100.0).round() as i32;
        let Ok((nx_cm, ny_cm)) =
            resolve_token_destination_cm(&ui.map, &sel_id, target_x_cm, target_y_cm)
        else {
            return;
        };
        let Some(tok_idx) = ui.map.tokens.iter().position(|t| t.entity_id == sel_id) else {
            return;
        };
        let (old_x_m, old_y_m) = ui.map.token_pos_m(&ui.map.tokens[tok_idx]);
        let dx = nx_cm as f32 / 100.0 - old_x_m;
        let dy = ny_cm as f32 / 100.0 - old_y_m;
        // `0 deg = North (-Z / -dy)`, `90 deg = East (+X)`, `180 deg = South (+Z / +dy)`
        let facing = ((dx.atan2(-dy).to_degrees().round() as i32) + 360) % 360;

        let nx_cell = (nx_cm / 100).clamp(0, ui.map.grid_width - 1);
        let ny_cell = (ny_cm / 100).clamp(0, ui.map.grid_height - 1);
        ui.map.tokens[tok_idx].x = nx_cell;
        ui.map.tokens[tok_idx].y = ny_cell;
        let label = ui.map.tokens[tok_idx].label.clone();
        let vis = ui.map.tokens[tok_idx].visibility;
        let (p_default, c_default, _) = resolve_token_visual(&ui.map, &ui.map.tokens[tok_idx]);
        ui.map.set_token_pos_cm(&sel_id, nx_cm, ny_cm);

        let (preset, color, existing_portrait) = if let Some(v) = ui
            .map
            .token_visuals
            .iter_mut()
            .find(|v| v.entity_id == sel_id)
        {
            v.facing_deg = facing;
            (v.model_preset.clone(), v.color_hex.clone(), v.portrait_url.clone())
        } else {
            ui.map.token_visuals.push(TokenVisualSpec {
                entity_id: sel_id.clone(),
                model_preset: p_default.clone(),
                color_hex: c_default.clone(),
                facing_deg: facing,
                elevation_cm: 0,
                stance_pose: "idle".to_string(),
                portrait_url: None,
                pos_x_cm: Some(nx_cm),
                pos_y_cm: Some(ny_cm),
            });
            (p_default, c_default, None)
        };

        let nm = (nx_cm as f32 / 100.0, ny_cm as f32 / 100.0);
        ui.hovered_point_m = Some(nm);
        ui.hovered_cell = Some((nx_cell, ny_cell));
        ui.hover_inspection =
            crate::wasm_client::map_hover_inspect::compute_hover_inspection(ui, nm.0, nm.1);
        ui.recompute_overlays();
        move_req = Some(MoveMapTokenRequest {
            entity_id: sel_id,
            label: Some(label),
            to_x: nx_cell,
            to_y: ny_cell,
            to_x_cm: Some(nx_cm),
            to_y_cm: Some(ny_cm),
            facing_deg: Some(facing),
            model_preset: Some(preset),
            color_hex: Some(color),
            stance_pose: None,
            portrait_url: existing_portrait,
            visibility: Some(vis),
            remove: false,
        });
    });

    mark_map_wgpu_dirty();
    refresh_map_dom_and_wgpu();
    if let Some((preset_code, edge_ref, next_kind)) = door_sync {
        crate::wasm_client::map_edge_edit::post_edge_edit_if_in_session(
            preset_code,
            edge_ref,
            Some(next_kind),
        );
    }
    if let Some(req) = move_req {
        post_move_token_if_in_session(req);
    }
}
