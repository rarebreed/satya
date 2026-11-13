//! Interactive `1.0m` Tactical Map State, Grid Coordinate Math, LoS/Cover & Gait Reach Overlays,
//! and Server REST Synchronization (`#notebook-pane-maps`).
//!
//! Companion modules:
//! - [`crate::wasm_client::map_3d_wgpu`] drives the live `wgpu` 3D Diorama / 2D Ortho canvas
//!   (`#wgpu-map-canvas`) and `.glb` character miniatures.
//! - [`crate::wasm_client::map_2d_art`] renders the illustrated 2D battlemap (`#tabletop-map-svg-fallback`),
//!   which is both the user-selectable `Mode: 2D Battlemap` view and the automatic no-WebGPU fallback.

pub use crate::wasm_client::map_2d_art::*;
pub use crate::wasm_client::map_3d_wgpu::*;
pub use crate::wasm_client::map_edge_edit::*;

use crate::wasm_client::dom::{
    escape_html, fetch_json, get_element, set_class, set_display, set_html, set_input_value,
    set_select_value, set_text,
};
use crate::wasm_client::state::with_state;
use satya_engine::ballistics::lookup_firearm_preset;
use satya_engine::barrier_ballistics::{
    BallisticShotEvaluation, BallisticShotQuery, BarrierPenetrationVerdict, evaluate_ballistic_shot,
};
use satya_engine::environment_vehicles::CombatMovementGait;
use satya_engine::map_dressing::{prop_effective_material_at, solid_prop_at};
use satya_engine::tactical_map::{
    GaitReachQuery, ProceduralSector, ReachableCell, SectorPreset, TacticalGridView,
    TacticalLosEvaluation, build_procedural_map_request, compute_reachable_on_grid,
    evaluate_los_on_grid, generate_procedural_sector, wall_edge_index_for_map,
};
use satya_schema::{
    CoverGrade, GenerateSessionMapRequest, GmMapUpdateRequest, MessageVisibility,
    MoveMapTokenRequest, TacticalTileKind, TokenPlacement, TokenVisualSpec,
};
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Client-side interactive state for the `Maps` notebook tab.
#[derive(Debug, Clone)]
pub struct TacticalMapUiState {
    pub map: GmMapUpdateRequest,
    pub selected_entity_id: Option<String>,
    pub hovered_cell: Option<(i32, i32)>,
    pub gait: CombatMovementGait,
    pub tick_budget: u32,
    /// `"move_or_select"`, `"measure_los"`, or `"edit_terrain"`.
    pub interaction_mode: String,
    pub selected_weapon_id: String,
    pub wall_cutaway: bool,
    pub last_los: Option<TacticalLosEvaluation>,
    pub last_ballistic: Option<BallisticShotEvaluation>,
    pub reachable_cells: Vec<ReachableCell>,
    pub seed_counter: i64,
    /// `true` shows the illustrated 2D battlemap instead of the WebGPU diorama (`#btn-map-view-mode`).
    pub view_battlemap: bool,
    /// Draws the `1.0m` / `5.0m` grid overlay on the 2D battlemap (`#btn-map-grid-overlay`).
    pub show_grid_overlay: bool,
}

impl Default for TacticalMapUiState {
    fn default() -> Self {
        let map = build_procedural_map_request(
            "map-starter",
            SectorPreset::default(),
            None,
            20,
            14,
            42,
        );
        let selected = map.tokens.first().map(|t| t.entity_id.clone());
        let mut st = Self {
            map,
            selected_entity_id: selected,
            hovered_cell: None,
            gait: CombatMovementGait::CombatGlideTacticalWalk,
            tick_budget: 8,
            interaction_mode: "move_or_select".to_string(),
            selected_weapon_id: "hk416_carbine_556".to_string(),
            wall_cutaway: true,
            last_los: None,
            last_ballistic: None,
            reachable_cells: Vec::new(),
            seed_counter: 42,
            view_battlemap: false,
            show_grid_overlay: true,
        };
        st.recompute_overlays();
        st
    }
}

impl TacticalMapUiState {
    pub fn ensure_tiles_populated(&mut self) {
        let expected = (self.map.grid_width.max(1) * self.map.grid_height.max(1)) as usize;
        if self.map.tiles.len() != expected {
            let preset = self
                .map
                .theme_preset
                .as_deref()
                .and_then(SectorPreset::from_code_str)
                .unwrap_or_default();
            let ProceduralSector { tiles, wall_edges, props } = generate_procedural_sector(
                preset,
                self.map.grid_width.max(8),
                self.map.grid_height.max(8),
                self.map.seed.unwrap_or(42),
            );
            self.map.tiles = tiles;
            if self.map.wall_edges.is_empty() {
                self.map.wall_edges = wall_edges;
            }
            if self.map.props.is_empty() {
                self.map.props = props;
            }
        }
    }

    pub fn recompute_overlays(&mut self) {
        self.ensure_tiles_populated();
        if self.selected_entity_id.is_none()
            || !self
                .map
                .tokens
                .iter()
                .any(|t| Some(&t.entity_id) == self.selected_entity_id.as_ref())
        {
            self.selected_entity_id = self.map.tokens.first().map(|t| t.entity_id.clone());
        }

        let cell_m = self.map.cell_size_m();
        if let Some(sel_id) = self.selected_entity_id.as_deref()
            && let Some(sel_tok) = self.map.tokens.iter().find(|t| t.entity_id == sel_id)
        {
            let occupied: Vec<(i32, i32)> = self
                .map
                .tokens
                .iter()
                .filter(|t| t.entity_id != sel_id)
                .map(|t| (t.x, t.y))
                .collect();
            let edges = wall_edge_index_for_map(&self.map);
            let grid = TacticalGridView {
                tiles: &self.map.tiles,
                width: self.map.grid_width,
                height: self.map.grid_height,
                edges: &edges,
            };
            self.reachable_cells = compute_reachable_on_grid(
                &grid,
                &occupied,
                GaitReachQuery {
                    start_xy: (sel_tok.x, sel_tok.y),
                    cell_size_m: cell_m,
                    spd_eff: 11,
                    hgt_m: 1.80,
                    gait: self.gait,
                    tick_budget: self.tick_budget,
                },
            );

            let target_xy = self.hovered_cell.or_else(|| {
                self.map
                    .tokens
                    .iter()
                    .find(|t| t.entity_id != sel_id)
                    .map(|t| (t.x, t.y))
            });
            let last_los = target_xy
                .map(|txy| evaluate_los_on_grid(&grid, cell_m, (sel_tok.x, sel_tok.y), txy));
            let firearm = lookup_firearm_preset(&self.selected_weapon_id)
                .or_else(|| lookup_firearm_preset("hk416_carbine_556"));
            self.last_ballistic = target_xy.zip(firearm).map(|(txy, f)| {
                evaluate_ballistic_shot(BallisticShotQuery {
                    grid: &grid,
                    props: &self.map.props,
                    cell_size_m: cell_m,
                    attacker_xy: (sel_tok.x, sel_tok.y),
                    target_xy: txy,
                    firearm: f,
                    net_successes: 0,
                    hit_frame_or_stud: false,
                })
            });
            self.last_los = last_los;
        } else {
            self.reachable_cells.clear();
            self.last_los = None;
            self.last_ballistic = None;
        }
    }
}

thread_local! {
    static MAP_UI_STATE: RefCell<TacticalMapUiState> =
        RefCell::new(TacticalMapUiState::default());
}

pub fn with_map_ui<F, R>(f: F) -> R
where
    F: FnOnce(&TacticalMapUiState) -> R,
{
    MAP_UI_STATE.with(|c| f(&c.borrow()))
}

pub fn with_map_ui_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut TacticalMapUiState) -> R,
{
    MAP_UI_STATE.with(|c| f(&mut c.borrow_mut()))
}

/// Converts grid coordinates `(gx, gy)` to centered world-space `(wx, wz)` in meters.
#[must_use]
pub fn grid_cell_to_world_xz(
    gx: i32,
    gy: i32,
    grid_w: i32,
    grid_h: i32,
    cell_size_m: f32,
) -> [f32; 2] {
    let wx = ((gx as f32 + 0.5) - (grid_w as f32 * 0.5)) * cell_size_m;
    let wz = ((gy as f32 + 0.5) - (grid_h as f32 * 0.5)) * cell_size_m;
    [wx, wz]
}

/// Converts world-space `(wx, wz)` in meters back into `(gx, gy)` grid indices if inside bounds.
#[must_use]
pub fn world_xz_to_grid_cell(
    wx: f32,
    wz: f32,
    grid_w: i32,
    grid_h: i32,
    cell_size_m: f32,
) -> Option<(i32, i32)> {
    let scale = cell_size_m.max(0.25);
    let gx = ((wx / scale) + (grid_w as f32 * 0.5)).floor() as i32;
    let gy = ((wz / scale) + (grid_h as f32 * 0.5)).floor() as i32;
    if gx >= 0 && gy >= 0 && gx < grid_w && gy < grid_h {
        Some((gx, gy))
    } else {
        None
    }
}

pub fn parse_movement_gait_code(code: &str) -> CombatMovementGait {
    match code.trim() {
        "prone_crawl" => CombatMovementGait::ProneLowCrawl,
        "stealth_creep" => CombatMovementGait::StealthCreepCrouchWalk,
        "bounding_run" => CombatMovementGait::BoundingRunHustle,
        "all_out_sprint" => CombatMovementGait::AllOutSprint,
        _ => CombatMovementGait::CombatGlideTacticalWalk,
    }
}

/// Resolves `(model_preset, color_hex, facing_deg)` for a token on `map`.
#[must_use]
pub fn resolve_token_visual(map: &GmMapUpdateRequest, tok: &TokenPlacement) -> (String, String, f32) {
    if let Some(v) = map.visual_for(&tok.entity_id) {
        return (
            v.model_preset.clone(),
            v.color_hex.clone(),
            v.facing_deg as f32,
        );
    }
    let low = tok.label.to_ascii_lowercase();
    let is_hostile =
        low.contains("sentry") || low.contains("secops") || low.contains("hostile") || tok.visibility == MessageVisibility::GmOnly;
    let preset = if low.contains("valkyrie") || low.contains("sara") {
        "valkyrie_rig"
    } else if low.contains("recon") || low.contains("marcus") {
        "recon_mk4"
    } else if is_hostile {
        "secops_sentry"
    } else {
        "operative_light"
    };
    let color = if is_hostile { "#f87171" } else { "#38bdf8" };
    let facing = if is_hostile { 270.0 } else { 90.0 };
    (preset.to_string(), color.to_string(), facing)
}

/// Formats the tactical Range, Line-of-Sight, Cover $\Delta_{\text{ext}}$, and Gait Reach readout bar HTML.
#[must_use]
pub fn format_map_tactical_hud_html(ui: &TacticalMapUiState) -> String {
    let sel_label = ui
        .selected_entity_id
        .as_deref()
        .and_then(|id| ui.map.tokens.iter().find(|t| t.entity_id == id))
        .map(|t| format!("{} ({}, {})", escape_html(&t.label), t.x, t.y))
        .unwrap_or_else(|| "None selected".to_string());

    let reach_count = ui.reachable_cells.len();
    let Some(los) = ui.last_los.as_ref() else {
        return format!(
            r#"<span><strong>Selected:</strong> {sel_label} · <strong>Reach ({ticks}t):</strong> {reach_count} cells (`1.0m`/cell) · Hover or click any cell to measure Range &amp; Cover.</span>"#,
            ticks = ui.tick_budget
        );
    };

    let target_name = ui
        .map
        .tokens
        .iter()
        .find(|t| (t.x, t.y) == los.target_xy)
        .map(|t| format!("{} @ ({}, {})", escape_html(&t.label), t.x, t.y))
        .unwrap_or_else(|| {
            let (tx, ty) = los.target_xy;
            match solid_prop_at(&ui.map.props, tx, ty) {
                Some(p) => format!(
                    "Cell ({tx}, {ty}) [{} · {}]",
                    p.kind.label(),
                    prop_effective_material_at(p, tx, ty).label()
                ),
                None => format!("Cell ({tx}, {ty}) [{}]", ui.map.tile_at(tx, ty).label()),
            }
        });

    let pill_cls = match los.cover_grade {
        CoverGrade::Clear => "pill pill-active",
        CoverGrade::Concealment | CoverGrade::LowCover | CoverGrade::HighCover => "pill pill-iggy",
        CoverGrade::Blocked => "pill pill-muted",
    };
    let reach_note = ui
        .reachable_cells
        .iter()
        .find(|r| (r.x, r.y) == los.target_xy)
        .map(|r| format!(" · Move: {:.1}m ({}t)", r.distance_cost_m, r.ticks_required))
        .unwrap_or_default();

    let bal_html = ui
        .last_ballistic
        .as_ref()
        .map(|b| {
            let b_cls = match b.verdict {
                BarrierPenetrationVerdict::Unobstructed
                | BarrierPenetrationVerdict::ConcealmentOnly
                | BarrierPenetrationVerdict::OvermatchedCover => "pill pill-active",
                BarrierPenetrationVerdict::PenetratesWithLoss => "pill pill-iggy",
                BarrierPenetrationVerdict::StoppedByBarrier => "pill pill-muted",
            };
            format!(r#" <span class="{b_cls}">{}</span>"#, escape_html(&b.summary_label))
        })
        .unwrap_or_default();

    format!(
        r#"<span><strong>{sel_label}</strong> → <strong>{target_name}</strong> · Range: <span class="mono">{:.2}m</span>{reach_note}</span> <span class="{pill_cls}">{}</span>{bal_html}"#,
        los.distance_m,
        los.cover_grade.label()
    )
}

/// Default SSR tactical SVG battlemap markup (`20m x 14m` Sector 4 Orbital Airlock).
#[must_use]
pub fn default_tactical_map_svg_html() -> String {
    format_tactical_map_svg_markup(&TacticalMapUiState::default())
}

/// Default SSR tactical Range / Cover / Gait Reach readout HTML.
#[must_use]
pub fn default_tactical_map_hud_html() -> String {
    format_map_tactical_hud_html(&TacticalMapUiState::default())
}

/// Default SSR selectable token chip bar HTML.
#[must_use]
pub fn default_tactical_map_tokens_html() -> String {
    let ui = TacticalMapUiState::default();
    format_selectable_token_chips_html(&ui.map, ui.selected_entity_id.as_deref())
}

/// Handles clicking a `(gx, gy)` grid cell in either the 2D SVG fallback or JS automation.
#[wasm_bindgen]
pub fn satya_click_map_cell(gx: i32, gy: i32) {
    handle_map_grid_cell_clicked(gx, gy);
}

/// Synchronizes all DOM elements inside `#notebook-pane-maps` (pill, token chips, LoS HUD, 2D battlemap,
/// and `wgpu` canvas). In `Mode: 2D Battlemap` the WebGPU diorama is not rendered.
pub fn refresh_map_dom_and_wgpu() {
    ensure_battlemap_view_pref_loaded();
    let svg_visible = get_element("tabletop-map-svg-fallback")
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        .is_some_and(|h| h.style().get_property_value("display").unwrap_or_default() != "none");

    let (pill_txt, tokens_html, hud_html, battlemap_on, sel_preset, sel_color, sel_wpn) =
        with_map_ui(|ui| {
            let pill = format!(
                "{} ({}m × {}m · 1.0m Grid)",
                ui.map.title, ui.map.grid_width, ui.map.grid_height
            );
            let chips = format_selectable_token_chips_html(&ui.map, ui.selected_entity_id.as_deref());
            let hud = format_map_tactical_hud_html(ui);
            let (preset, color) = ui
                .selected_entity_id
                .as_deref()
                .and_then(|id| ui.map.tokens.iter().find(|t| t.entity_id == id))
                .map(|tok| {
                    let (p, c, _) = resolve_token_visual(&ui.map, tok);
                    (p, c)
                })
                .unwrap_or_else(|| ("valkyrie_rig".to_string(), "#38bdf8".to_string()));
            (pill, chips, hud, ui.view_battlemap, preset, color, ui.selected_weapon_id.clone())
        });

    set_text("canvas-map-pill", &pill_txt);
    set_class("canvas-map-pill", "pill pill-active");
    set_display("tabletop-map-placeholder", "none");
    set_select_value("map-weapon-select", &sel_wpn);
    set_select_value("map-token-model-select", &sel_preset);
    set_input_value("map-token-color", &sel_color);
    set_html("tabletop-map-los-hud", &hud_html);
    set_html("tabletop-token-list", &tokens_html);

    if battlemap_on || svg_visible {
        sync_battlemap_svg_host();
    }
    if !battlemap_on {
        refresh_map_wgpu_if_active();
    }
}

/// Formats clickable token chips for `#tabletop-token-list` allowing 1-click selection of any miniature.
#[must_use]
pub fn format_selectable_token_chips_html(
    map: &GmMapUpdateRequest,
    selected_id: Option<&str>,
) -> String {
    if map.tokens.is_empty() {
        return r#"<span class="pill pill-muted">No tokens on map</span>"#.to_string();
    }
    map.tokens
        .iter()
        .map(|tok| {
            let vis_suffix = if tok.visibility != MessageVisibility::Public {
                format!(" · {:?}", tok.visibility)
            } else {
                String::new()
            };
            let active_cls = if selected_id == Some(tok.entity_id.as_str()) {
                "pill pill-active"
            } else {
                "pill pill-iggy"
            };
            let safe_id = escape_html(&tok.entity_id);
            format!(
                r#"<button type="button" class="btn-chip {active_cls}" onclick="window.satyaSelectMapToken('{safe_id}')">{} ({}, {}){}</button>"#,
                escape_html(&tok.label),
                tok.x,
                tok.y,
                vis_suffix
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Ingests an authoritative [`GmMapUpdateRequest`] from `ServerWsPacket::MapUpdated` or REST history.
pub fn apply_incoming_map_update(incoming: &GmMapUpdateRequest) {
    with_map_ui_mut(|ui| {
        ui.map = incoming.clone();
        ui.recompute_overlays();
    });
    mark_map_wgpu_dirty();
    if !incoming.image_url.is_empty()
        && incoming.tiles.is_empty()
        && let Some(img) = get_element("tabletop-map-img")
    {
        let _ = img.set_attribute("src", &incoming.image_url);
    }
    refresh_map_dom_and_wgpu();
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

/// Generates a new procedural `1.0m` tactical sector map from `#map-preset-select` (syncing to server if in a session).
#[wasm_bindgen]
pub fn satya_generate_procedural_map() {
    let raw_preset = crate::wasm_client::dom::select_value("map-preset-select");
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
                let pref_y = (mid_y - 2 + (idx as i32) * 2).clamp(1, built.grid_height - 2);
                let (gx, gy) = if built.tile_at(2, pref_y).blocks_occupancy() {
                    (1..built.grid_width - 1)
                        .flat_map(|x| (1..built.grid_height - 1).map(move |y| (x, y)))
                        .filter(|&(x, y)| !built.tile_at(x, y).blocks_occupancy())
                        .min_by_key(|&(x, y)| (x - 2).abs() + (y - pref_y).abs())
                        .unwrap_or((2, mid_y))
                } else {
                    (2, pref_y)
                };
                built.tokens.push(TokenPlacement {
                    entity_id: p.user_id.clone(),
                    label: p.display_name.clone(),
                    x: gx,
                    y: gy,
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
                });
            }
            built.tokens.push(TokenPlacement {
                entity_id: "npc-secops-1".to_string(),
                label: "SecOps Sentry".to_string(),
                x: built.grid_width - 3,
                y: mid_y,
                visibility: MessageVisibility::Public,
            });
            built.token_visuals.push(TokenVisualSpec {
                entity_id: "npc-secops-1".to_string(),
                model_preset: "secops_sentry".to_string(),
                color_hex: "#f87171".to_string(),
                facing_deg: 270,
                elevation_cm: 0,
                stance_pose: "aimed".to_string(),
            });
        }
        ui.map = built;
        ui.recompute_overlays();
        s
    });

    mark_map_wgpu_dirty();
    refresh_map_dom_and_wgpu();
    post_generate_map_if_in_session(preset.as_str(), seed, None);
}

fn post_generate_map_if_in_session(
    preset_code: &str,
    seed: i64,
    cell_edit: Option<(i32, i32, Option<TacticalTileKind>)>,
) {
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
        populate_roster_tokens: cell_edit.is_none(),
        edit_cell_x: cell_edit.map(|(x, _, _)| x),
        edit_cell_y: cell_edit.map(|(_, y, _)| y),
        set_tile_kind: cell_edit.and_then(|(_, _, k)| k),
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
    let wpn_str = crate::wasm_client::dom::select_value("map-weapon-select");
    let gait_str = crate::wasm_client::dom::select_value("map-gait-select");
    let mode_str = crate::wasm_client::dom::select_value("map-click-mode");
    let ticks = crate::wasm_client::dom::input_value("map-ticks-input")
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
    let model_preset = crate::wasm_client::dom::select_value("map-token-model-select");
    let color_hex = crate::wasm_client::dom::input_value("map-token-color");
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
        if let Some(vis) = ui
            .map
            .token_visuals
            .iter_mut()
            .find(|v| v.entity_id == sel_id)
        {
            vis.model_preset = p_eff.clone();
            vis.color_hex = c_eff.clone();
        } else {
            ui.map.token_visuals.push(TokenVisualSpec {
                entity_id: sel_id.clone(),
                model_preset: p_eff.clone(),
                color_hex: c_eff.clone(),
                facing_deg: 90,
                elevation_cm: 0,
                stance_pose: "idle".to_string(),
            });
        }
        sync_req = Some(MoveMapTokenRequest {
            entity_id: sel_id,
            label: Some(tok.label),
            to_x: tok.x,
            to_y: tok.y,
            facing_deg: None,
            model_preset: Some(p_eff),
            color_hex: Some(c_eff),
            stance_pose: None,
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

/// Handles left-clicking a `(gx, gy)` cell on the tactical map canvas (selecting tokens, moving
/// the selected miniature with automatic facing rotation, measuring LoS, or toggling airlock doors/cover).
pub fn handle_map_grid_cell_clicked(gx: i32, gy: i32) {
    type PendingTileEdit = (String, i64, (i32, i32, Option<TacticalTileKind>));
    let mut move_req: Option<MoveMapTokenRequest> = None;
    let mut tile_edit: Option<PendingTileEdit> = None;

    with_map_ui_mut(|ui| {
        if gx < 0 || gy < 0 || gx >= ui.map.grid_width || gy >= ui.map.grid_height {
            return;
        }
        ui.ensure_tiles_populated();

        if ui.interaction_mode == "measure_los" {
            ui.hovered_cell = Some((gx, gy));
            ui.recompute_overlays();
            return;
        }

        if ui.interaction_mode == "edit_terrain" {
            let idx = (gy * ui.map.grid_width + gx) as usize;
            let next = match ui.map.tiles[idx] {
                TacticalTileKind::Floor => TacticalTileKind::Concealment,
                TacticalTileKind::Concealment => TacticalTileKind::HalfCover,
                TacticalTileKind::HalfCover => TacticalTileKind::FullWall,
                TacticalTileKind::FullWall => TacticalTileKind::DoorClosed,
                TacticalTileKind::DoorClosed => TacticalTileKind::DoorOpen,
                TacticalTileKind::DoorOpen => TacticalTileKind::HazardZone,
                TacticalTileKind::HazardZone => TacticalTileKind::ElevatedPlatform,
                TacticalTileKind::ElevatedPlatform => TacticalTileKind::Floor,
            };
            ui.map.tiles[idx] = next;
            ui.recompute_overlays();
            let preset_str = ui
                .map
                .theme_preset
                .clone()
                .unwrap_or_else(|| "orbital_airlock".to_string());
            tile_edit = Some((preset_str, ui.map.seed.unwrap_or(42), (gx, gy, Some(next))));
            return;
        }

        // 1. Clicking an existing token selects it
        if let Some(clicked_tok) = ui.map.tokens.iter().find(|t| t.x == gx && t.y == gy) {
            ui.selected_entity_id = Some(clicked_tok.entity_id.clone());
            ui.recompute_overlays();
            return;
        }

        // 2. Clicking a closed/open door toggles the airlock door directly
        let cur_tile = ui.map.tile_at(gx, gy);
        if matches!(
            cur_tile,
            TacticalTileKind::DoorClosed | TacticalTileKind::DoorOpen
        ) {
            let toggled = if cur_tile == TacticalTileKind::DoorClosed {
                TacticalTileKind::DoorOpen
            } else {
                TacticalTileKind::DoorClosed
            };
            let idx = (gy * ui.map.grid_width + gx) as usize;
            if idx < ui.map.tiles.len() {
                ui.map.tiles[idx] = toggled;
            }
            ui.recompute_overlays();
            let preset_str = ui
                .map
                .theme_preset
                .clone()
                .unwrap_or_else(|| "orbital_airlock".to_string());
            tile_edit = Some((
                preset_str,
                ui.map.seed.unwrap_or(42),
                (gx, gy, Some(toggled)),
            ));
            return;
        }

        // 3. Move selected token to an unblocked cell and orient facing toward movement direction
        if cur_tile.blocks_occupancy() {
            return;
        }
        let Some(sel_id) = ui.selected_entity_id.clone() else {
            return;
        };
        let Some(tok_idx) = ui.map.tokens.iter().position(|t| t.entity_id == sel_id) else {
            return;
        };
        let (old_x, old_y) = (ui.map.tokens[tok_idx].x, ui.map.tokens[tok_idx].y);
        let dx = (gx - old_x) as f32;
        let dy = (gy - old_y) as f32;
        // `0 deg = North (-Z / -dy)`, `90 deg = East (+X)`, `180 deg = South (+Z / +dy)`
        let facing = ((dx.atan2(-dy).to_degrees().round() as i32) + 360) % 360;

        ui.map.tokens[tok_idx].x = gx;
        ui.map.tokens[tok_idx].y = gy;
        let label = ui.map.tokens[tok_idx].label.clone();
        let vis = ui.map.tokens[tok_idx].visibility;

        let (preset, color) = if let Some(v) = ui
            .map
            .token_visuals
            .iter_mut()
            .find(|v| v.entity_id == sel_id)
        {
            v.facing_deg = facing;
            (v.model_preset.clone(), v.color_hex.clone())
        } else {
            let (p, c, _) = resolve_token_visual(&ui.map, &ui.map.tokens[tok_idx]);
            ui.map.token_visuals.push(TokenVisualSpec {
                entity_id: sel_id.clone(),
                model_preset: p.clone(),
                color_hex: c.clone(),
                facing_deg: facing,
                elevation_cm: 0,
                stance_pose: "idle".to_string(),
            });
            (p, c)
        };

        ui.hovered_cell = Some((gx, gy));
        ui.recompute_overlays();
        move_req = Some(MoveMapTokenRequest {
            entity_id: sel_id,
            label: Some(label),
            to_x: gx,
            to_y: gy,
            facing_deg: Some(facing),
            model_preset: Some(preset),
            color_hex: Some(color),
            stance_pose: None,
            visibility: Some(vis),
            remove: false,
        });
    });

    mark_map_wgpu_dirty();
    refresh_map_dom_and_wgpu();
    if let Some((preset_code, seed, edit)) = tile_edit {
        post_generate_map_if_in_session(&preset_code, seed, Some(edit));
    }
    if let Some(req) = move_req {
        post_move_token_if_in_session(req);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_xz_and_grid_cell_roundtrip_and_svg_hud_formatters() {
        for gy in 0..14 {
            for gx in 0..20 {
                let [wx, wz] = grid_cell_to_world_xz(gx, gy, 20, 14, 1.0);
                assert_eq!(world_xz_to_grid_cell(wx, wz, 20, 14, 1.0), Some((gx, gy)));
            }
        }
        let mut ui = TacticalMapUiState::default();
        let hud = format_map_tactical_hud_html(&ui);
        assert!(hud.contains("Sara (Valkyrie)"));
        let svg = format_tactical_map_svg_markup(&ui);
        assert!(svg.starts_with(r#"<div class="battlemap-stack">"#));
        assert!(svg.ends_with("</svg></div>"));

        // Verify weapon-graded barrier HUD across Apartment drywall/furniture & Estate masonry/foliage
        ui.map = build_procedural_map_request("apt", SectorPreset::ResidentialApartment, None, 20, 14, 42);
        ui.selected_weapon_id = "sig_p320_9mm".to_string();
        ui.recompute_overlays();
        assert!(ui.last_ballistic.is_some());
        let hud_9mm = format_map_tactical_hud_html(&ui);
        ui.selected_weapon_id = "scar_h_762x51".to_string();
        ui.recompute_overlays();
        let hud_762 = format_map_tactical_hud_html(&ui);
        assert_ne!(hud_9mm, hud_762);
    }
}
