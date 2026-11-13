//! 2D SVG Battlemap Camera (`BattlemapCamera2d`), Boundary Continuation Controls, and Map Authoring Guards.
//!
//! Companion to [`crate::wasm_client::map_2d_art`] and [`crate::wasm_client::map_2d_art_overlay`]:
//! drives smooth cursor-centered zoom and drag-pan via SVG `viewBox` updates (zero `#battlemap-art`
//! DOM rebuilds), inverts `preserveAspectRatio="xMidYMid meet"` letterboxing for sub-centimeter
//! pointer unprojection, detects deliberate drag-overscroll past map boundaries, and gates
//! directional `Continue [North/South/East/West]` controls to the Human GM / Host or solo players
//! in AI GM sessions (`!has_human_gm`).

use crate::wasm_client::dom::{get_element, set_display, set_text};
use crate::wasm_client::map_2d_art::{BATTLEMAP_CELL_PX, BATTLEMAP_PAD_PX};
use crate::wasm_client::map_3d::{with_map_ui, with_map_ui_mut};
use crate::wasm_client::map_3d_input::{continue_session_map_in_direction, handle_map_point_clicked};
use crate::wasm_client::map_3d_wgpu::{current_3d_map_zoom_and_pan, zoom_3d_map_camera_by};
use crate::wasm_client::map_hover_inspect::{satya_hover_map_point, satya_leave_map_hover};
use crate::wasm_client::state::with_state;
use satya_engine::tactical_map::can_continue_map;
use satya_schema::{
    GmMapUpdateRequest, MapContinuationDir, Participant, ParticipantKind, SessionRole,
};
use wasm_bindgen::prelude::*;

/// Minimum 2D battlemap zoom factor (`0.25x` fits up to `80m x 56m` stitched arenas).
pub const MIN_ZOOM_2D: f32 = 0.25;
/// Maximum 2D battlemap zoom factor (`6.0x` close-up inspection).
pub const MAX_ZOOM_2D: f32 = 6.0;
/// Reference single-sector width in meters (`zoom = 1.0` matches `20m x 14m` sector scale).
pub const REF_SECTOR_W_M: f32 = 20.0;
/// Reference single-sector height in meters (`zoom = 1.0` matches `20m x 14m` sector scale).
pub const REF_SECTOR_H_M: f32 = 14.0;
/// Outward pixel drag past a clamped boundary before triggering automatic procedural continuation.
pub const OVERSCROLL_TRIGGER_PX: f32 = 120.0;
const DRAG_THRESHOLD_PX: f32 = 4.0;
const WHEEL_ZOOM_STEP: f32 = 1.14;
const BUTTON_ZOOM_STEP: f32 = 1.20;

/// Bounding client rectangle `(left, top, width, height)` in CSS pixels.
#[derive(Debug, Clone, Copy)]
pub struct ClientRect2d {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

/// Host-testable 2D SVG battlemap viewport camera (`viewBox` zoom & metric pan).
#[derive(Debug, Clone)]
pub struct BattlemapCamera2d {
    /// Offset from board center `(bw_m * 0.5, bh_m * 0.5)` in meters.
    pub pan_m: (f32, f32),
    /// Scale relative to standard `20m x 14m` sector (`1.0` = `100%`).
    pub zoom: f32,
    pub dragging: bool,
    pub drag_moved: bool,
    pub drag_start_client: (f32, f32),
    pub drag_last_client: (f32, f32),
    pub overscroll_px: (f32, f32),
}

impl Default for BattlemapCamera2d {
    fn default() -> Self {
        Self {
            pan_m: (0.0, 0.0),
            zoom: 1.0,
            dragging: false,
            drag_moved: false,
            drag_start_client: (0.0, 0.0),
            drag_last_client: (0.0, 0.0),
            overscroll_px: (0.0, 0.0),
        }
    }
}

impl BattlemapCamera2d {
    /// Overview zoom that fits a `board_size_m` map completely without exceeding `1.0`.
    #[must_use]
    pub fn fit_zoom_for_board(board_size_m: (f32, f32)) -> f32 {
        f32::min(
            REF_SECTOR_W_M / board_size_m.0.max(1.0),
            REF_SECTOR_H_M / board_size_m.1.max(1.0),
        )
        .clamp(MIN_ZOOM_2D, 1.0)
    }

    /// Resets pan to center `(0, 0)` and zoom to fit the given board dimensions.
    pub fn reset_for_board(&mut self, board_size_m: (f32, f32)) {
        self.pan_m = (0.0, 0.0);
        self.zoom = Self::fit_zoom_for_board(board_size_m);
        self.clear_drag();
    }

    /// Clears active pointer drag and overscroll accumulators.
    pub fn clear_drag(&mut self) {
        self.dragging = false;
        self.drag_moved = false;
        self.overscroll_px = (0.0, 0.0);
    }

    /// Computes SVG `(min_x, min_y, width, height)` `viewBox` user units.
    #[must_use]
    pub fn view_box_tuple(
        &self,
        board_size_m: (f32, f32),
        cell_size_m: f32,
    ) -> (f32, f32, f32, f32) {
        let cell_m = cell_size_m.max(0.25);
        let px_per_m = BATTLEMAP_CELL_PX as f32 / cell_m;
        let pad = BATTLEMAP_PAD_PX as f32;
        let z = self.zoom.clamp(MIN_ZOOM_2D, MAX_ZOOM_2D);
        let base_w = REF_SECTOR_W_M * px_per_m + 2.0 * pad;
        let base_h = REF_SECTOR_H_M * px_per_m + 2.0 * pad;
        let vb_w = base_w / z;
        let vb_h = base_h / z;
        let cx_svg = (board_size_m.0 * 0.5 + self.pan_m.0) * px_per_m;
        let cy_svg = (board_size_m.1 * 0.5 + self.pan_m.1) * px_per_m;
        (cx_svg - 0.5 * vb_w, cy_svg - 0.5 * vb_h, vb_w, vb_h)
    }

    /// Formats the SVG `viewBox` attribute string for `#battlemap-art` and `#battlemap-overlay`.
    #[must_use]
    pub fn view_box_attr(&self, board_size_m: (f32, f32), cell_size_m: f32) -> String {
        let (vx, vy, vw, vh) = self.view_box_tuple(board_size_m, cell_size_m);
        format!("{vx:.2} {vy:.2} {vw:.2} {vh:.2}")
    }

    /// Inverts `preserveAspectRatio="xMidYMid meet"` from CSS client coordinates to metric `(x_m, y_m)`.
    #[must_use]
    pub fn unproject_client_to_metric(
        &self,
        client_x: f32,
        client_y: f32,
        rect: ClientRect2d,
        board_size_m: (f32, f32),
        cell_size_m: f32,
    ) -> Option<(f32, f32)> {
        if !client_x.is_finite()
            || !client_y.is_finite()
            || rect.width <= 1.0
            || rect.height <= 1.0
        {
            return None;
        }
        let cell_m = cell_size_m.max(0.25);
        let px_per_m = BATTLEMAP_CELL_PX as f32 / cell_m;
        let (vx, vy, vb_w, vb_h) = self.view_box_tuple(board_size_m, cell_m);
        let scale = f32::min(rect.width / vb_w, rect.height / vb_h).max(1e-6);
        let off_x = (rect.width - vb_w * scale) * 0.5;
        let off_y = (rect.height - vb_h * scale) * 0.5;
        let svg_x = vx + (client_x - rect.left - off_x) / scale;
        let svg_y = vy + (client_y - rect.top - off_y) / scale;
        Some((svg_x / px_per_m, svg_y / px_per_m))
    }

    /// Symmetric pan limits around board center in meters.
    #[must_use]
    pub fn max_pan_m(&self, board_size_m: (f32, f32)) -> (f32, f32) {
        (
            (board_size_m.0 * 0.5 + 3.5).max(6.0),
            (board_size_m.1 * 0.5 + 3.5).max(5.0),
        )
    }

    /// Clamps `pan_m` inside `[-max_pan, +max_pan]`.
    pub fn clamp_pan(&mut self, board_size_m: (f32, f32)) {
        let (mx, my) = self.max_pan_m(board_size_m);
        self.pan_m.0 = self.pan_m.0.clamp(-mx, mx);
        self.pan_m.1 = self.pan_m.1.clamp(-my, my);
    }

    /// Applies multiplicative `factor` around `client_pt` keeping the world point fixed under cursor.
    pub fn zoom_around_client_point(
        &mut self,
        factor: f32,
        client_pt: (f32, f32),
        rect: ClientRect2d,
        board_size_m: (f32, f32),
        cell_size_m: f32,
    ) {
        let (client_x, client_y) = client_pt;
        let cell_m = cell_size_m.max(0.25);
        let anchor = self.unproject_client_to_metric(
            client_x,
            client_y,
            rect,
            board_size_m,
            cell_m,
        );
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM_2D, MAX_ZOOM_2D);
        if let Some((mx, my)) = anchor
            && rect.width > 1.0
            && rect.height > 1.0
        {
            let px_per_m = BATTLEMAP_CELL_PX as f32 / cell_m;
            let (_, _, vb_w, vb_h) = self.view_box_tuple(board_size_m, cell_m);
            let scale = f32::min(rect.width / vb_w, rect.height / vb_h).max(1e-6);
            let dx_rel = client_x - rect.left - rect.width * 0.5;
            let dy_rel = client_y - rect.top - rect.height * 0.5;
            self.pan_m.0 = (mx - 0.5 * board_size_m.0) - dx_rel / (scale * px_per_m);
            self.pan_m.1 = (my - 0.5 * board_size_m.1) - dy_rel / (scale * px_per_m);
        }
        self.clamp_pan(board_size_m);
        self.overscroll_px = (0.0, 0.0);
    }

    /// Pans the camera by screen drag delta `(dx_px, dy_px)` and returns `Some(dir)` if deliberate
    /// drag-overscroll past a clamped boundary crosses [`OVERSCROLL_TRIGGER_PX`].
    pub fn pan_by_client_delta(
        &mut self,
        dx_px: f32,
        dy_px: f32,
        rect_w: f32,
        rect_h: f32,
        board_size_m: (f32, f32),
        cell_size_m: f32,
    ) -> Option<MapContinuationDir> {
        if rect_w <= 1.0 || rect_h <= 1.0 {
            return None;
        }
        let cell_m = cell_size_m.max(0.25);
        let px_per_m = BATTLEMAP_CELL_PX as f32 / cell_m;
        let (_, _, vb_w, vb_h) = self.view_box_tuple(board_size_m, cell_m);
        let scale = f32::min(rect_w / vb_w, rect_h / vb_h).max(1e-6);
        let px_to_m = 1.0 / (scale * px_per_m);

        // Dragging right (`dx_px > 0`) pulls the board right, shifting camera West (`-pan_m.0`).
        let raw_x = self.pan_m.0 - dx_px * px_to_m;
        let raw_y = self.pan_m.1 - dy_px * px_to_m;
        let (max_x, max_y) = self.max_pan_m(board_size_m);

        self.pan_m.0 = raw_x.clamp(-max_x, max_x);
        self.pan_m.1 = raw_y.clamp(-max_y, max_y);

        let eff_dx = -dx_px;
        let eff_dy = -dy_px;
        if raw_x < -max_x && eff_dx < 0.0 {
            self.overscroll_px.0 = (self.overscroll_px.0 + eff_dx).min(0.0);
        } else if raw_x > max_x && eff_dx > 0.0 {
            self.overscroll_px.0 = (self.overscroll_px.0 + eff_dx).max(0.0);
        } else {
            self.overscroll_px.0 = 0.0;
        }

        if raw_y < -max_y && eff_dy < 0.0 {
            self.overscroll_px.1 = (self.overscroll_px.1 + eff_dy).min(0.0);
        } else if raw_y > max_y && eff_dy > 0.0 {
            self.overscroll_px.1 = (self.overscroll_px.1 + eff_dy).max(0.0);
        } else {
            self.overscroll_px.1 = 0.0;
        }

        let dir = if self.overscroll_px.0.abs() >= OVERSCROLL_TRIGGER_PX
            && self.overscroll_px.0.abs() >= self.overscroll_px.1.abs()
        {
            Some(if self.overscroll_px.0 < 0.0 {
                MapContinuationDir::West
            } else {
                MapContinuationDir::East
            })
        } else if self.overscroll_px.1.abs() >= OVERSCROLL_TRIGGER_PX {
            Some(if self.overscroll_px.1 < 0.0 {
                MapContinuationDir::North
            } else {
                MapContinuationDir::South
            })
        } else {
            None
        };

        if dir.is_some() {
            self.clear_drag();
        }
        dir
    }

    /// Re-anchors `pan_m` after `map_continuation` stitches a strip onto `old_size_m`, preserving
    /// visual continuity (plus an optional step into the newly generated sector).
    pub fn adjust_after_stitch(
        &mut self,
        old_size_m: (f32, f32),
        new_size_m: (f32, f32),
        dir: MapContinuationDir,
        nudge_into_new: bool,
    ) {
        let added_w = (new_size_m.0 - old_size_m.0).max(0.0);
        let added_h = (new_size_m.1 - old_size_m.1).max(0.0);
        let (shift_x, shift_y) = match dir {
            MapContinuationDir::West => (added_w, 0.0),
            MapContinuationDir::North => (0.0, added_h),
            _ => (0.0, 0.0),
        };
        let (nudge_x, nudge_y) = if nudge_into_new {
            let (ux, uy) = dir.unit_step();
            (ux as f32 * 5.5, uy as f32 * 4.5)
        } else {
            (0.0, 0.0)
        };
        self.pan_m.0 = self.pan_m.0 + shift_x + 0.5 * (old_size_m.0 - new_size_m.0) + nudge_x;
        self.pan_m.1 = self.pan_m.1 + shift_y + 0.5 * (old_size_m.1 - new_size_m.1) + nudge_y;
        self.clamp_pan(new_size_m);
        self.clear_drag();
    }
}

/// Returns `true` when the viewer can authorize map edits or continuations (Human GM/Host, solo/co-op AI GM, or offline).
#[must_use]
pub fn can_author_map(self_part: Option<&Participant>, roster: &[Participant]) -> bool {
    if roster.is_empty() && self_part.is_none() {
        return true;
    }
    let has_human_gm = roster
        .iter()
        .any(|p| p.role == SessionRole::Gm && p.participant_kind == ParticipantKind::Human);
    let is_gm_or_host = self_part.is_some_and(|p| matches!(p.role, SessionRole::Gm | SessionRole::Host));
    is_gm_or_host || !has_human_gm
}

/// Checks whether the local browser user is permitted to continue or edit the active session map.
#[must_use]
pub fn local_user_can_author_map() -> bool {
    with_state(|st| {
        if st.session_id.is_none() {
            true
        } else {
            can_author_map(st.self_participant.as_ref(), &st.roster)
        }
    })
}

/// Computes cardinal boundary continuation buttons currently visible in 2D SVG mode.
#[must_use]
pub fn active_panned_continuation_dirs_2d(
    cam: &BattlemapCamera2d,
    map: &GmMapUpdateRequest,
    rect_aspect: Option<f32>,
) -> Vec<MapContinuationDir> {
    let board = map.board_size_m();
    let cell_m = map.cell_size_m();
    let px_per_m = BATTLEMAP_CELL_PX as f32 / cell_m;
    let (vx, vy, vb_w, vb_h) = cam.view_box_tuple(board, cell_m);
    let (vis_w_svg, vis_h_svg) = match rect_aspect.filter(|a| a.is_finite() && *a > 0.05) {
        Some(asp) if asp > vb_w / vb_h => (vb_h * asp, vb_h),
        Some(asp) => (vb_w, vb_w / asp),
        None => (vb_w, vb_h),
    };
    let cx_svg = vx + 0.5 * vb_w;
    let cy_svg = vy + 0.5 * vb_h;
    let vis_min_x = (cx_svg - 0.5 * vis_w_svg) / px_per_m;
    let vis_max_x = (cx_svg + 0.5 * vis_w_svg) / px_per_m;
    let vis_min_y = (cy_svg - 0.5 * vis_h_svg) / px_per_m;
    let vis_max_y = (cy_svg + 0.5 * vis_h_svg) / px_per_m;

    let mut out = Vec::with_capacity(4);
    if can_continue_map(map, MapContinuationDir::North)
        && vis_min_y <= 0.75
        && (cam.pan_m.1 <= -1.0 || (cam.zoom > 1.15 && vis_max_y < board.1 - 0.75 && cam.pan_m.1 < -0.1))
    {
        out.push(MapContinuationDir::North);
    }
    if can_continue_map(map, MapContinuationDir::South)
        && vis_max_y >= board.1 - 0.75
        && (cam.pan_m.1 >= 1.0 || (cam.zoom > 1.15 && vis_min_y > 0.75 && cam.pan_m.1 > 0.1))
    {
        out.push(MapContinuationDir::South);
    }
    if can_continue_map(map, MapContinuationDir::West)
        && vis_min_x <= 0.75
        && (cam.pan_m.0 <= -1.0 || (cam.zoom > 1.15 && vis_max_x < board.0 - 0.75 && cam.pan_m.0 < -0.1))
    {
        out.push(MapContinuationDir::West);
    }
    if can_continue_map(map, MapContinuationDir::East)
        && vis_max_x >= board.0 - 0.75
        && (cam.pan_m.0 >= 1.0 || (cam.zoom > 1.15 && vis_min_x > 0.75 && cam.pan_m.0 > 0.1))
    {
        out.push(MapContinuationDir::East);
    }
    out
}

/// Computes cardinal boundary continuation buttons currently visible from 3D diorama camera pan offsets.
#[must_use]
pub fn active_panned_continuation_dirs_3d(
    pan_xz: [f32; 2],
    map: &GmMapUpdateRequest,
) -> Vec<MapContinuationDir> {
    let mut out = Vec::with_capacity(4);
    if can_continue_map(map, MapContinuationDir::North) && pan_xz[1] <= -1.75 {
        out.push(MapContinuationDir::North);
    }
    if can_continue_map(map, MapContinuationDir::South) && pan_xz[1] >= 1.75 {
        out.push(MapContinuationDir::South);
    }
    if can_continue_map(map, MapContinuationDir::West) && pan_xz[0] <= -2.0 {
        out.push(MapContinuationDir::West);
    }
    if can_continue_map(map, MapContinuationDir::East) && pan_xz[0] >= 2.0 {
        out.push(MapContinuationDir::East);
    }
    out
}

/// Infers whether `new_map` is a seamless procedural continuation of `old_map` and along which edge.
#[must_use]
pub fn detect_map_stitch_dir(
    old_map: &GmMapUpdateRequest,
    new_map: &GmMapUpdateRequest,
    explicit_hint: Option<MapContinuationDir>,
) -> Option<MapContinuationDir> {
    let grew_w = new_map.grid_width > old_map.grid_width;
    let grew_h = new_map.grid_height > old_map.grid_height;
    if !grew_w && !grew_h {
        return None;
    }
    if let Some(hint) = explicit_hint {
        return Some(hint);
    }
    let shared_delta = old_map.tokens.iter().find_map(|ot| {
        let nt = new_map.tokens.iter().find(|t| t.entity_id == ot.entity_id)?;
        let (ox, oy) = old_map.token_pos_m(ot);
        let (nx, ny) = new_map.token_pos_m(nt);
        Some((nx - ox, ny - oy))
    });
    if grew_w && new_map.grid_height == old_map.grid_height {
        let shifted_right = shared_delta.is_some_and(|(dx, _)| dx >= 2.0);
        return Some(if shifted_right {
            MapContinuationDir::West
        } else {
            MapContinuationDir::East
        });
    }
    if grew_h && new_map.grid_width == old_map.grid_width {
        let shifted_down = shared_delta.is_some_and(|(_, dy)| dy >= 2.0);
        return Some(if shifted_down {
            MapContinuationDir::North
        } else {
            MapContinuationDir::South
        });
    }
    None
}

fn overlay_rect_dims() -> Option<ClientRect2d> {
    let el = get_element("battlemap-overlay")?;
    let r = el.get_bounding_client_rect();
    let (width, height) = (r.width() as f32, r.height() as f32);
    if width > 1.0 && height > 1.0 {
        Some(ClientRect2d {
            left: r.left() as f32,
            top: r.top() as f32,
            width,
            height,
        })
    } else {
        None
    }
}

/// Synchronizes the 2D `viewBox` attributes, `#map-zoom-readout` badge, and edge continuation buttons.
pub fn sync_map_camera_and_edge_controls_dom() {
    let rect_aspect = overlay_rect_dims().map(|r| r.width / r.height);
    let can_author = local_user_can_author_map();
    let (zoom_3d, pan_3d) = current_3d_map_zoom_and_pan();

    let (view_box, zoom_pct, battlemap_on, dirs) = with_map_ui(|ui| {
        let board = ui.map.board_size_m();
        let vb = ui.camera_2d.view_box_attr(board, ui.map.cell_size_m());
        let zoom_val = if ui.view_battlemap { ui.camera_2d.zoom } else { zoom_3d };
        let dirs = if !can_author || ui.continuation_in_flight {
            Vec::new()
        } else if ui.view_battlemap {
            active_panned_continuation_dirs_2d(&ui.camera_2d, &ui.map, rect_aspect)
        } else {
            active_panned_continuation_dirs_3d(pan_3d, &ui.map)
        };
        (vb, (zoom_val * 100.0).round() as i32, ui.view_battlemap, dirs)
    });

    if battlemap_on {
        if let Some(art) = get_element("battlemap-art") {
            let _ = art.set_attribute("viewBox", &view_box);
        }
        if let Some(overlay) = get_element("battlemap-overlay") {
            let _ = overlay.set_attribute("viewBox", &view_box);
        }
    }
    set_text("map-zoom-readout", &format!("{zoom_pct}%"));

    for (dir, id) in [
        (MapContinuationDir::North, "btn-map-continue-north"),
        (MapContinuationDir::South, "btn-map-continue-south"),
        (MapContinuationDir::West, "btn-map-continue-west"),
        (MapContinuationDir::East, "btn-map-continue-east"),
    ] {
        set_display(id, if dirs.contains(&dir) { "inline-flex" } else { "none" });
    }
}

/// Handles mouse-wheel zoom centered on cursor inside the 2D SVG battlemap.
#[wasm_bindgen]
pub fn satya_battlemap_wheel(delta_y: f32, client_x: f32, client_y: f32) {
    let factor = if delta_y < 0.0 {
        WHEEL_ZOOM_STEP
    } else {
        1.0 / WHEEL_ZOOM_STEP
    };
    let rect = overlay_rect_dims();
    with_map_ui_mut(|ui| {
        let board = ui.map.board_size_m();
        let cell_m = ui.map.cell_size_m();
        if let Some(r) = rect {
            ui.camera_2d.zoom_around_client_point(
                factor,
                (client_x, client_y),
                r,
                board,
                cell_m,
            );
        } else {
            ui.camera_2d.zoom = (ui.camera_2d.zoom * factor).clamp(MIN_ZOOM_2D, MAX_ZOOM_2D);
        }
    });
    sync_map_camera_and_edge_controls_dom();
}

/// Starts 2D SVG pointer press/drag tracking on `#battlemap-overlay`.
#[wasm_bindgen]
pub fn satya_battlemap_mouse_down(button: i32, client_x: f32, client_y: f32) {
    if !(0..=2).contains(&button) {
        return;
    }
    with_map_ui_mut(|ui| {
        ui.camera_2d.dragging = true;
        ui.camera_2d.drag_moved = false;
        ui.camera_2d.drag_start_client = (client_x, client_y);
        ui.camera_2d.drag_last_client = (client_x, client_y);
        ui.camera_2d.overscroll_px = (0.0, 0.0);
    });
}

/// Handles 2D SVG pointer drag-panning, boundary overscroll continuation, and hover coordinate tracking.
#[wasm_bindgen]
pub fn satya_battlemap_mouse_move(buttons: i32, client_x: f32, client_y: f32) {
    let Some(rect) = overlay_rect_dims() else {
        return;
    };
    let can_author = local_user_can_author_map();
    let (did_pan, overscroll_dir, hover_metric) = with_map_ui_mut(|ui| {
        if ui.camera_2d.dragging && (buttons & 7) == 0 {
            ui.camera_2d.clear_drag();
        }
        let board = ui.map.board_size_m();
        let cell_m = ui.map.cell_size_m();
        if ui.camera_2d.dragging {
            let (sx, sy) = ui.camera_2d.drag_start_client;
            if !ui.camera_2d.drag_moved
                && ((client_x - sx).abs() > DRAG_THRESHOLD_PX
                    || (client_y - sy).abs() > DRAG_THRESHOLD_PX)
            {
                ui.camera_2d.drag_moved = true;
            }
            if ui.camera_2d.drag_moved {
                let (lx, ly) = ui.camera_2d.drag_last_client;
                let dx = client_x - lx;
                let dy = client_y - ly;
                ui.camera_2d.drag_last_client = (client_x, client_y);
                let dir = ui
                    .camera_2d
                    .pan_by_client_delta(dx, dy, rect.width, rect.height, board, cell_m);
                return (true, dir.filter(|_| can_author), None);
            }
        }
        let metric = ui
            .camera_2d
            .unproject_client_to_metric(client_x, client_y, rect, board, cell_m);
        (false, None, metric)
    });

    if did_pan {
        sync_map_camera_and_edge_controls_dom();
    } else if let Some((fx, fy)) = hover_metric {
        satya_hover_map_point(fx, fy, client_x, client_y);
    }
    if let Some(dir) = overscroll_dir {
        continue_session_map_in_direction(dir);
    }
}

/// Finishes 2D SVG pointer interaction and dispatches left-click map action when not dragged.
#[wasm_bindgen]
pub fn satya_battlemap_mouse_up(button: i32, client_x: f32, client_y: f32) {
    let rect = overlay_rect_dims();
    let click_metric = with_map_ui_mut(|ui| {
        let should_click =
            button == 0 && (!ui.camera_2d.dragging || !ui.camera_2d.drag_moved);
        ui.camera_2d.clear_drag();
        if !should_click {
            return None;
        }
        let r = rect?;
        let board = ui.map.board_size_m();
        let cell_m = ui.map.cell_size_m();
        ui.camera_2d
            .unproject_client_to_metric(client_x, client_y, r, board, cell_m)
    });
    if let Some((fx, fy)) = click_metric {
        handle_map_point_clicked(fx, fy);
    }
}

/// Cancels active 2D drag/overscroll state when the pointer leaves `#battlemap-overlay`.
#[wasm_bindgen]
pub fn satya_battlemap_mouse_leave() {
    with_map_ui_mut(|ui| {
        ui.camera_2d.clear_drag();
    });
    satya_leave_map_hover();
}

fn zoom_active_map_by(factor: f32) {
    let battlemap_on = with_map_ui(|ui| ui.view_battlemap);
    if battlemap_on {
        let rect = overlay_rect_dims();
        with_map_ui_mut(|ui| {
            let board = ui.map.board_size_m();
            let cell_m = ui.map.cell_size_m();
            if let Some(r) = rect {
                ui.camera_2d.zoom_around_client_point(
                    factor,
                    (r.left + r.width * 0.5, r.top + r.height * 0.5),
                    r,
                    board,
                    cell_m,
                );
            } else {
                ui.camera_2d.zoom = (ui.camera_2d.zoom * factor).clamp(MIN_ZOOM_2D, MAX_ZOOM_2D);
            }
        });
    } else {
        zoom_3d_map_camera_by(factor);
    }
    sync_map_camera_and_edge_controls_dom();
}

/// Zooms in on the active tactical map (`#btn-map-zoom-in`).
#[wasm_bindgen]
pub fn satya_zoom_map_in() {
    zoom_active_map_by(BUTTON_ZOOM_STEP);
}

/// Zooms out on the active tactical map (`#btn-map-zoom-out`).
#[wasm_bindgen]
pub fn satya_zoom_map_out() {
    zoom_active_map_by(1.0 / BUTTON_ZOOM_STEP);
}

/// Triggers seamless procedural continuation along a cardinal map boundary (`NORTH`/`SOUTH`/`EAST`/`WEST`).
#[wasm_bindgen]
pub fn satya_continue_map_dir(dir_code: String) {
    if let Some(dir) = MapContinuationDir::from_code_str(&dir_code) {
        continue_session_map_in_direction(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_engine::tactical_map::{SectorPreset, build_procedural_map_request};

    #[test]
    fn viewbox_default_matches_legacy_20x14_and_fit_zoom_covers_80x56() {
        let cam = BattlemapCamera2d::default();
        assert_eq!(cam.view_box_tuple((20.0, 14.0), 1.0), (-12.0, -12.0, 1304.0, 920.0));
        let fit_max = BattlemapCamera2d::fit_zoom_for_board((80.0, 56.0));
        assert!((fit_max - 0.25).abs() < 1e-4, "expected 0.25x for 80x56, got {fit_max}");
    }

    #[test]
    fn unproject_inverts_letterboxed_aspect_ratios_and_cursor_zoom_preserves_anchor() {
        let mut cam = BattlemapCamera2d::default();
        let board = (20.0, 14.0);
        // Ultra-wide 1000x400 container with xMidYMid meet letterboxing
        let rect = ClientRect2d {
            left: 100.0,
            top: 50.0,
            width: 1000.0,
            height: 400.0,
        };
        let (cx, cy) = (rect.left + 680.0, rect.top + 135.0);
        let before = cam
            .unproject_client_to_metric(cx, cy, rect, board, 1.0)
            .expect("unproject before zoom");
        cam.zoom_around_client_point(2.0, (cx, cy), rect, board, 1.0);
        let after = cam
            .unproject_client_to_metric(cx, cy, rect, board, 1.0)
            .expect("unproject after zoom");
        assert!(
            (before.0 - after.0).abs() < 1e-3 && (before.1 - after.1).abs() < 1e-3,
            "anchor drifted across cursor zoom: {before:?} -> {after:?}"
        );
    }

    #[test]
    fn overscroll_sign_and_stitch_camera_shift_preserve_world_focus() {
        let mut cam = BattlemapCamera2d::default();
        let board = (20.0, 14.0);
        let (max_x, _) = cam.max_pan_m(board);
        cam.pan_m.0 = max_x;
        // Dragging mouse left (`dx_px = -70`) twice while clamped East triggers East continuation
        assert_eq!(cam.pan_by_client_delta(-70.0, 0.0, 800.0, 560.0, board, 1.0), None);
        assert_eq!(
            cam.pan_by_client_delta(-70.0, 0.0, 800.0, 560.0, board, 1.0),
            Some(MapContinuationDir::East)
        );

        // North overscroll: dragging mouse down (`dy_px = +70`) twice while clamped North
        cam.pan_m = (0.0, -cam.max_pan_m(board).1);
        assert_eq!(cam.pan_by_client_delta(0.0, 70.0, 800.0, 560.0, board, 1.0), None);
        assert_eq!(
            cam.pan_by_client_delta(0.0, 70.0, 800.0, 560.0, board, 1.0),
            Some(MapContinuationDir::North)
        );

        // West stitch keeps the original room camera anchor shifted (+20m West added -> world center +10m)
        let mut west_cam = BattlemapCamera2d {
            pan_m: (-3.0, 1.0),
            zoom: 1.5,
            ..Default::default()
        };
        west_cam.adjust_after_stitch((20.0, 14.0), (40.0, 14.0), MapContinuationDir::West, false);
        assert!((west_cam.pan_m.0 - 7.0).abs() < 1e-4, "expected +7.0m pan, got {}", west_cam.pan_m.0);
        assert!((west_cam.pan_m.1 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn authoring_permissions_allow_gm_and_solo_ai_gm_but_block_non_gm_player() {
        let gm = Participant::human("gm-1", "gm", "Sean (GM)", SessionRole::Gm);
        let p1 = Participant::human("p-1", "sara", "Sara", SessionRole::Player);
        let ai_gm = Participant::ai_gm("s-1", "qwen", "Arbiter");

        assert!(can_author_map(None, &[]));
        assert!(can_author_map(Some(&gm), &[gm.clone(), p1.clone()]));
        assert!(!can_author_map(Some(&p1), &[gm, p1.clone()]));
        assert!(can_author_map(Some(&p1), &[p1.clone(), ai_gm]));

        let map = build_procedural_map_request("m-1", SectorPreset::DockingBay, None, 20, 14, 77);
        let mut cam = BattlemapCamera2d::default();
        assert!(active_panned_continuation_dirs_2d(&cam, &map, None).is_empty());
        cam.pan_m.0 = 4.0;
        assert_eq!(
            active_panned_continuation_dirs_2d(&cam, &map, None),
            vec![MapContinuationDir::East]
        );
    }
}
