//! Interactive `1.0m` Tactical Map State, Grid Coordinate Math, LoS/Cover & Gait Reach Overlays,
//! and DOM / Stage Refresh Synchronization (`#notebook-pane-maps`).
//!
//! Companion modules:
//! - [`crate::wasm_client::map_3d_input`] handles click/movement/generation actions and REST sync.
//! - [`crate::wasm_client::map_3d_wgpu`] drives the live `wgpu` 3D Diorama / 2D Ortho canvas
//!   (`#wgpu-map-canvas`) and `.glb` character miniatures.
//! - [`crate::wasm_client::map_2d_art`] renders the illustrated 2D battlemap (`#tabletop-map-svg-fallback`),
//!   which is both the user-selectable `Mode: 2D Battlemap` view and the automatic no-WebGPU fallback.

pub use crate::wasm_client::map_2d_art::*;
pub use crate::wasm_client::map_3d_input::*;
pub use crate::wasm_client::map_3d_wgpu::*;
pub use crate::wasm_client::map_edge_edit::*;

use crate::wasm_client::dom::{
    escape_html, get_element, set_class, set_display, set_html, set_input_value, set_select_value,
    set_text,
};
use satya_engine::ballistics::lookup_firearm_preset;
use satya_engine::barrier_ballistics::{
    BallisticShotEvaluation, BallisticShotQuery, BarrierPenetrationVerdict, evaluate_ballistic_shot,
};
use satya_engine::environment_vehicles::CombatMovementGait;
use satya_engine::tactical_map::{
    GaitReachQuery, ReachableField, ReachableStrip, SectorPreset, TacticalLosEvaluation,
    TacticalSceneView, build_procedural_map_request, compute_reachable_continuous,
    evaluate_los_continuous,
};
use satya_engine::tactical_map_geom::{
    MapHoverInspection, TOKEN_PICK_RADIUS_M, migrate_legacy_tiles_to_vector,
};
use satya_schema::{CoverGrade, GmMapUpdateRequest, MessageVisibility, TokenPlacement};
use std::cell::RefCell;
use wasm_bindgen::JsCast;

/// Client-side interactive state for the `Maps` notebook tab.
#[derive(Debug, Clone)]
pub struct TacticalMapUiState {
    pub map: GmMapUpdateRequest,
    pub selected_entity_id: Option<String>,
    pub hovered_cell: Option<(i32, i32)>,
    /// Continuous cursor coordinate in meters (`0.0..board_w_m`, `0.0..board_h_m`).
    pub hovered_point_m: Option<(f32, f32)>,
    /// Live Object & Ground Hover Inspector result at `hovered_point_m`.
    pub hover_inspection: Option<MapHoverInspection>,
    /// Pinned `[I]` / `#btn-map-inspect-toggle` floating inspector card state.
    pub inspect_mode_pinned: bool,
    /// Temporary `Alt`-key hold state for floating inspector card.
    pub alt_inspect_held: bool,
    /// Optional `0.50m` increment snapping when placing props in `edit_terrain` mode.
    pub snap_to_half_meter: bool,
    pub gait: CombatMovementGait,
    pub tick_budget: u32,
    /// `"move_or_select"`, `"measure_los"`, or `"edit_terrain"`.
    pub interaction_mode: String,
    pub selected_weapon_id: String,
    pub wall_cutaway: bool,
    pub last_los: Option<TacticalLosEvaluation>,
    pub last_ballistic: Option<BallisticShotEvaluation>,
    pub reachable_field: Option<ReachableField>,
    pub reachable_strips: Vec<ReachableStrip>,
    pub seed_counter: i64,
    /// `true` shows the illustrated 2D battlemap instead of the WebGPU diorama (`#btn-map-view-mode`).
    pub view_battlemap: bool,
    /// Draws the visual `1.0m` / `5.0m` scale grid overlay on the 2D battlemap (`#btn-map-grid-overlay`).
    pub show_grid_overlay: bool,
    /// Paper/ink SVG filter style for the 2D battlemap (`#map-ink-style-select`).
    pub ink_style: crate::wasm_client::map_2d_tokens::BattlemapInkStyle,
    /// Ground & prop material mode (`Painted` WebP tiles vs. `ProceduralVector` SVG fills).
    pub texture_mode: crate::wasm_client::map_2d_art_textures::BattlemapTextureMode,
    /// Active brush selection in `edit_terrain` mode (`"__wall_edge__"`/`""` = edge cycle, `"__erase__"` = eraser, else `PropKind`).
    pub edit_prop_brush: String,
    /// Clockwise placement rotation (`0..=359°`, `+15°` steps or direct numeric input) when placing props.
    pub edit_prop_rotation_deg: i32,
    /// 2D SVG battlemap viewport zoom/pan camera and drag state.
    pub camera_2d: crate::wasm_client::map_2d_camera::BattlemapCamera2d,
    /// Direction of an in-flight local continuation click/overscroll so the incoming map nudges into the new area.
    pub pending_stitch_nudge: Option<satya_schema::MapContinuationDir>,
    /// True while a `/map` procedural continuation POST request is in flight.
    pub continuation_in_flight: bool,
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
        let camera_2d = crate::wasm_client::map_2d_camera::BattlemapCamera2d::default();
        let mut st = Self {
            map,
            selected_entity_id: selected,
            hovered_cell: None,
            hovered_point_m: None,
            hover_inspection: None,
            inspect_mode_pinned: false,
            alt_inspect_held: false,
            snap_to_half_meter: false,
            gait: CombatMovementGait::CombatGlideTacticalWalk,
            tick_budget: 8,
            interaction_mode: "move_or_select".to_string(),
            selected_weapon_id: "hk416_carbine_556".to_string(),
            wall_cutaway: true,
            last_los: None,
            last_ballistic: None,
            reachable_field: None,
            reachable_strips: Vec::new(),
            seed_counter: 42,
            view_battlemap: false,
            show_grid_overlay: true,
            ink_style: crate::wasm_client::map_2d_tokens::BattlemapInkStyle::default(),
            texture_mode: crate::wasm_client::map_2d_art_textures::BattlemapTextureMode::default(),
            edit_prop_brush: String::new(),
            edit_prop_rotation_deg: 0,
            camera_2d,
            pending_stitch_nudge: None,
            continuation_in_flight: false,
        };
        st.recompute_overlays();
        st
    }
}

impl TacticalMapUiState {
    pub fn ensure_vector_scene_populated(&mut self) {
        if !self.map.tiles.is_empty() {
            migrate_legacy_tiles_to_vector(
                &self.map.tiles,
                self.map.grid_width,
                self.map.grid_height,
                &mut self.map.wall_edges,
                &mut self.map.props,
            );
            self.map.tiles.clear();
        }
    }

    pub fn recompute_overlays(&mut self) {
        self.ensure_vector_scene_populated();
        if self.selected_entity_id.is_none()
            || !self
                .map
                .tokens
                .iter()
                .any(|t| Some(&t.entity_id) == self.selected_entity_id.as_ref())
        {
            self.selected_entity_id = self
                .map
                .tokens
                .iter()
                .find(|t| self.map.token_floor(&t.entity_id) == self.map.active_floor)
                .or_else(|| self.map.tokens.first())
                .map(|t| t.entity_id.clone());
        }

        let (board_w_m, board_h_m) = self.map.board_size_m();
        let cell_m = self.map.cell_size_m();
        if let Some(sel_id) = self.selected_entity_id.as_deref()
            && self.map.token_floor(sel_id) == self.map.active_floor
            && let Some(sel_tok) = self.map.tokens.iter().find(|t| t.entity_id == sel_id)
        {
            let start_m = self.map.token_pos_m(sel_tok);
            let occupied: Vec<(f32, f32)> = self
                .map
                .tokens
                .iter()
                .filter(|t| {
                    t.entity_id != sel_id
                        && self.map.token_floor(&t.entity_id) == self.map.active_floor
                })
                .map(|t| self.map.token_pos_m(t))
                .collect();
            let scene = TacticalSceneView {
                width_m: board_w_m,
                height_m: board_h_m,
                edges: &self.map.wall_edges,
                props: &self.map.props,
                surfaces: &self.map.surfaces,
            };
            let field = compute_reachable_continuous(
                &scene,
                &occupied,
                GaitReachQuery {
                    start_m,
                    spd_eff: 11,
                    hgt_m: 1.80,
                    gait: self.gait,
                    tick_budget: self.tick_budget,
                },
            );
            self.reachable_strips = field.merged_row_runs();
            self.reachable_field = Some(field);

            let target_m = self
                .hovered_point_m
                .or_else(|| {
                    self.hovered_cell
                        .map(|(gx, gy)| ((gx as f32 + 0.5) * cell_m, (gy as f32 + 0.5) * cell_m))
                })
                .or_else(|| {
                    self.map
                        .tokens
                        .iter()
                        .find(|t| {
                            t.entity_id != sel_id
                                && self.map.token_floor(&t.entity_id) == self.map.active_floor
                        })
                        .map(|t| self.map.token_pos_m(t))
                });
            let last_los = target_m.map(|tm| evaluate_los_continuous(&scene, start_m, tm));
            let firearm = lookup_firearm_preset(&self.selected_weapon_id)
                .or_else(|| lookup_firearm_preset("hk416_carbine_556"));
            self.last_ballistic = target_m.zip(firearm).map(|(tm, f)| {
                evaluate_ballistic_shot(BallisticShotQuery {
                    scene: &scene,
                    attacker_m: start_m,
                    target_m: tm,
                    firearm: f,
                    net_successes: 0,
                    hit_frame_or_stud: false,
                })
            });
            self.last_los = last_los;
        } else {
            self.reachable_field = None;
            self.reachable_strips.clear();
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

/// Converts continuous metric `(x_m, y_m)` coordinates to centered 3D world-space `[wx, wz]` in meters.
#[must_use]
pub fn metric_to_world_xz(x_m: f32, y_m: f32, board_w_m: f32, board_h_m: f32) -> [f32; 2] {
    [x_m - board_w_m * 0.5, y_m - board_h_m * 0.5]
}

/// Converts 3D world-space `(wx, wz)` in meters back to board-local `(x_m, y_m)` if inside bounds.
#[must_use]
pub fn world_xz_to_metric(wx: f32, wz: f32, board_w_m: f32, board_h_m: f32) -> Option<(f32, f32)> {
    let x_m = wx + board_w_m * 0.5;
    let y_m = wz + board_h_m * 0.5;
    if x_m >= 0.0 && y_m >= 0.0 && x_m <= board_w_m && y_m <= board_h_m {
        Some((x_m, y_m))
    } else {
        None
    }
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
        .map(|t| {
            let (tx_m, ty_m) = ui.map.token_pos_m(t);
            format!("{} ({tx_m:.2}m, {ty_m:.2}m)", escape_html(&t.label))
        })
        .unwrap_or_else(|| "None selected".to_string());

    let Some(los) = ui.last_los.as_ref() else {
        return format!(
            r#"<span class="map-los-hud-main"><strong>Selected:</strong> {sel_label} · <strong>Reach ({ticks}t):</strong> Free metric point-and-click · Hover or click anywhere to move or measure Range &amp; Cover.</span>"#,
            ticks = ui.tick_budget
        );
    };

    let target_name = ui
        .map
        .tokens
        .iter()
        .map(|t| (t, ui.map.token_pos_m(t)))
        .find(|(_, (px, py))| (px - los.target_m.0).hypot(py - los.target_m.1) <= TOKEN_PICK_RADIUS_M)
        .map(|(t, (px, py))| format!("{} @ ({px:.2}m, {py:.2}m)", escape_html(&t.label)))
        .unwrap_or_else(|| {
            let (tx, ty) = los.target_m;
            format!("Point ({tx:.2}m, {ty:.2}m)")
        });

    let pill_cls = match los.cover_grade {
        CoverGrade::Clear => "pill pill-active",
        CoverGrade::Concealment | CoverGrade::LowCover | CoverGrade::HighCover => "pill pill-iggy",
        CoverGrade::Blocked => "pill pill-muted",
    };
    let reach_note = ui
        .reachable_field
        .as_ref()
        .and_then(|f| f.ticks_at_m(los.target_m.0, los.target_m.1))
        .map(|t| format!(" · Reach: {t}t"))
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
            format!(r#"<span class="{b_cls}">{}</span>"#, escape_html(&b.summary_label))
        })
        .unwrap_or_default();

    format!(
        r#"<span class="map-los-hud-main"><strong>{sel_label}</strong> → <strong>{target_name}</strong> · Range: <span class="mono">{:.2}m</span>{reach_note}</span><span class="map-los-hud-pills"><span class="{pill_cls}">{}</span>{bal_html}</span>"#,
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

/// Synchronizes all DOM elements inside `#notebook-pane-maps` (pill, token chips, LoS HUD, 2D battlemap,
/// and `wgpu` canvas). In `Mode: 2D Battlemap` the WebGPU diorama is not rendered.
pub fn refresh_map_dom_and_wgpu() {
    ensure_battlemap_view_pref_loaded();
    let svg_visible = get_element("tabletop-map-svg-fallback")
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        .is_some_and(|h| h.style().get_property_value("display").unwrap_or_default() != "none");

    let (
        pill_txt,
        tokens_html,
        hud_html,
        battlemap_on,
        sel_preset,
        sel_color,
        sel_wpn,
        edit_on,
        prop_brush,
        prop_rot,
        snap_on,
        inspect_pinned,
        floor_html,
        has_multi_floors,
    ) = with_map_ui(|ui| {
        let (bw_m, bh_m) = ui.map.board_size_m();
        let pill = format!("{} ({bw_m:.0}m × {bh_m:.0}m · Free Metric)", ui.map.title);
        let chips = format_selectable_token_chips_html(&ui.map, ui.selected_entity_id.as_deref());
        let hud = format_map_tactical_hud_html(ui);
        let floor_bar =
            format_map_floor_switcher_html(&ui.map, ui.selected_entity_id.as_deref());
        let (preset, color) = ui
            .selected_entity_id
            .as_deref()
            .and_then(|id| ui.map.tokens.iter().find(|t| t.entity_id == id))
            .map(|tok| {
                let (p, c, _) = resolve_token_visual(&ui.map, tok);
                (p, c)
            })
            .unwrap_or_else(|| ("valkyrie_rig".to_string(), "#38bdf8".to_string()));
        (
            pill,
            chips,
            hud,
            ui.view_battlemap,
            preset,
            color,
            ui.selected_weapon_id.clone(),
            ui.interaction_mode == "edit_terrain",
            ui.edit_prop_brush.clone(),
            ui.edit_prop_rotation_deg,
            ui.snap_to_half_meter,
            ui.inspect_mode_pinned,
            floor_bar,
            ui.map.has_multiple_floors(),
        )
    });

    set_text("canvas-map-pill", &pill_txt);
    set_class("canvas-map-pill", "pill pill-active");
    set_display("tabletop-map-placeholder", "none");
    set_html("map-floor-switcher", &floor_html);
    set_display(
        "map-floor-switcher",
        if has_multi_floors { "flex" } else { "none" },
    );
    set_select_value("map-weapon-select", &sel_wpn);
    set_select_value("map-token-model-select", &sel_preset);
    set_input_value("map-token-color", &sel_color);
    set_display("map-prop-brush-wrap", if edit_on { "inline-flex" } else { "none" });
    set_select_value("map-prop-brush-select", &prop_brush);
    let show_rot = edit_on && satya_schema::PropKind::from_code_str(&prop_brush).is_some();
    set_display("btn-map-prop-rotate", if show_rot { "inline-flex" } else { "none" });
    set_display("map-prop-rot-wrap", if show_rot { "inline-flex" } else { "none" });
    set_text("btn-map-prop-rotate", &format!("Rot {prop_rot}°"));
    set_input_value("map-prop-rot-deg", &prop_rot.to_string());
    set_display("btn-map-snap-toggle", if edit_on { "inline-flex" } else { "none" });
    set_text(
        "btn-map-snap-toggle",
        if snap_on { "Snap: 0.5m" } else { "Snap: Free" },
    );
    set_text(
        "btn-map-inspect-toggle",
        if inspect_pinned {
            "Inspect: ON [I]"
        } else {
            "Inspect [Alt/I]"
        },
    );
    set_html("tabletop-map-los-hud", &hud_html);
    set_html("tabletop-token-list", &tokens_html);
    crate::wasm_client::map_hover_inspect::refresh_inspect_popup_if_visible();

    if battlemap_on || svg_visible {
        sync_battlemap_svg_host();
    }
    if !battlemap_on {
        refresh_map_wgpu_if_active();
    }
    crate::wasm_client::map_2d_camera::sync_map_camera_and_edge_controls_dom();
}

/// Formats the floating floor switcher pill bar + cross-floor move buttons for multi-floor maps.
#[must_use]
pub fn format_map_floor_switcher_html(map: &GmMapUpdateRequest, selected_id: Option<&str>) -> String {
    if !map.has_multiple_floors() {
        return String::new();
    }
    let mut levels = map.floors.clone();
    levels.sort_by_key(|f| f.floor_index);
    let mut out = String::new();
    for fl in &levels {
        let count = map
            .tokens
            .iter()
            .filter(|t| map.token_floor(&t.entity_id) == fl.floor_index)
            .count();
        let active_cls = if fl.floor_index == map.active_floor {
            " active"
        } else {
            ""
        };
        let badge = escape_html(&fl.short_badge());
        let label = escape_html(&fl.label);
        let count_txt = if count > 0 {
            format!(" · {count}")
        } else {
            String::new()
        };
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                r#"<button type="button" class="map-floor-pill{active_cls}" title="{label}" onclick="window.satyaSelectMapFloor({})">{badge}{count_txt}</button>"#,
                fl.floor_index
            ),
        );
    }
    if let Some(sel_id) = selected_id
        && map.tokens.iter().any(|t| t.entity_id == sel_id)
    {
        let cur_fl = map.token_floor(sel_id);
        for fl in &levels {
            if fl.floor_index == cur_fl {
                continue;
            }
            let badge = escape_html(&fl.short_badge());
            let label = escape_html(&fl.label);
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    r#"<button type="button" class="map-floor-move-btn" title="Move selected miniature to {label}" onclick="window.satyaMoveTokenToFloor({})">Move → {badge}</button>"#,
                    fl.floor_index
                ),
            );
        }
    }
    out
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
    let multi_floor = map.has_multiple_floors();
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
            let (px_cm, py_cm) = map.token_pos_cm(tok);
            let cell_cx = tok.x * map.cell_size_cm.max(25) + map.cell_size_cm.max(25) / 2;
            let cell_cy = tok.y * map.cell_size_cm.max(25) + map.cell_size_cm.max(25) / 2;
            let coord_label = if (px_cm - cell_cx).abs() <= 2 && (py_cm - cell_cy).abs() <= 2 {
                format!("({}, {})", tok.x, tok.y)
            } else {
                format!("({:.2}m, {:.2}m)", px_cm as f32 / 100.0, py_cm as f32 / 100.0)
            };
            let floor_prefix = if multi_floor {
                format!("[{}] ", satya_schema::default_floor_badge(map.token_floor(&tok.entity_id)))
            } else {
                String::new()
            };
            format!(
                r#"<button type="button" class="btn-chip {active_cls}" onclick="window.satyaSelectMapToken('{safe_id}')">{floor_prefix}{} {coord_label}{vis_suffix}</button>"#,
                escape_html(&tok.label),
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Ingests an authoritative [`GmMapUpdateRequest`] from `ServerWsPacket::MapUpdated` or REST history.
pub fn apply_incoming_map_update(incoming: &GmMapUpdateRequest) {
    let (old_size, new_size, stitch_dir, nudge, size_changed) = with_map_ui_mut(|ui| {
        let old_size = ui.map.board_size_m();
        let new_size = incoming.board_size_m();
        let size_changed =
            incoming.grid_width != ui.map.grid_width || incoming.grid_height != ui.map.grid_height;
        let explicit_nudge = ui.pending_stitch_nudge.take();
        ui.continuation_in_flight = false;
        let stitch_dir = crate::wasm_client::map_2d_camera::detect_map_stitch_dir(
            &ui.map,
            incoming,
            explicit_nudge,
        );
        if let Some(dir) = stitch_dir {
            ui.camera_2d
                .adjust_after_stitch(old_size, new_size, dir, explicit_nudge.is_some());
        } else if size_changed {
            ui.camera_2d.reset_for_board(new_size);
        }
        ui.map = incoming.clone();
        ui.recompute_overlays();
        (
            old_size,
            new_size,
            stitch_dir,
            explicit_nudge.is_some(),
            size_changed,
        )
    });
    if let Some(dir) = stitch_dir {
        crate::wasm_client::map_3d_wgpu::shift_3d_camera_on_stitch(old_size, new_size, dir, nudge);
    } else if size_changed {
        crate::wasm_client::map_3d_wgpu::reset_3d_map_camera_for_board(new_size);
    }
    mark_map_wgpu_dirty();
    if !incoming.image_url.is_empty()
        && incoming.tiles.is_empty()
        && let Some(img) = get_element("tabletop-map-img")
    {
        let _ = img.set_attribute("src", &incoming.image_url);
    }
    refresh_map_dom_and_wgpu();
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_engine::tactical_map_geom::resolve_token_destination_cm;

    #[test]
    fn test_world_xz_and_grid_cell_roundtrip_and_svg_hud_formatters() {
        for gy in 0..14 {
            for gx in 0..20 {
                let [wx, wz] = grid_cell_to_world_xz(gx, gy, 20, 14, 1.0);
                assert_eq!(world_xz_to_grid_cell(wx, wz, 20, 14, 1.0), Some((gx, gy)));
            }
        }
        let [wx, wz] = metric_to_world_xz(7.35, 4.18, 20.0, 14.0);
        let round = world_xz_to_metric(wx, wz, 20.0, 14.0).expect("in bounds");
        assert!((round.0 - 7.35).abs() < 1e-4 && (round.1 - 4.18).abs() < 1e-4);

        let mut ui = TacticalMapUiState::default();
        assert!(ui.map.tiles.is_empty());
        let hud = format_map_tactical_hud_html(&ui);
        assert!(hud.contains("Sara (Valkyrie)"));
        let svg = format_tactical_map_svg_markup(&ui);
        assert!(svg.starts_with(r#"<div class="battlemap-stack">"#));
        assert!(svg.ends_with("</svg></div>"));

        // Continuous off-grid point movement
        let sel = ui.selected_entity_id.clone().unwrap();
        let ok = resolve_token_destination_cm(&ui.map, &sel, 263, 418).is_ok();
        assert!(ok, "continuous metric movement should succeed in open deck");

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

        // Verify multi-floor Luxurious Mansion formats #map-floor-switcher & token badges
        let mansion = build_procedural_map_request("mansion", SectorPreset::LuxuryMansion, None, 24, 18, 42);
        let sel_id = mansion.tokens.first().map(|t| t.entity_id.as_str());
        let switcher_html = format_map_floor_switcher_html(&mansion, sel_id);
        assert!(switcher_html.contains("satyaSelectMapFloor("), "{switcher_html}");
        assert!(switcher_html.contains("satyaMoveTokenToFloor("), "{switcher_html}");
        let chips_html = format_selectable_token_chips_html(&mansion, sel_id);
        assert!(chips_html.contains("[1F]"), "{chips_html}");
    }
}
