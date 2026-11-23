//! Interactive 2D & 3D Battlemap Editor Mode (`map_editor`).
//!
//! Provides GM / Host / Solo authoring controls inside the `Maps` tab without breaking offline
//! or WebGPU/2D-SVG parity:
//! - Select, continuous drag-move (`0.05m` fine or `0.5m` snap), `360°` rotate, stretch (`w_cm`),
//!   tint (`#RRGGBB`), toggle `Visual Only (decorative)`, duplicate, and delete props/surfaces.
//! - Drag-paint grid walls, doors, windows, or erasures with explicit [`MaterialClass`].
//! - Draw `Rect`, multi-vertex `Polygon`, or `Ribbon` vector floor/ground zones with optional tint.
//! - Override or tint the active floor's base ground texture.
//! - 30-step per-floor Undo/Redo ring (`Ctrl+Z` / `Ctrl+Y`) synchronized atomically via
//!   `replace_floor_geometry` when connected to a session.

use crate::wasm_client::dom::{
    fetch_json, input_checked, input_value, select_value, set_button_disabled, set_class,
    set_display, set_input_checked, set_input_value, set_select_value, set_text,
};
use crate::wasm_client::map_2d_art_overlay::sync_battlemap_svg_host;
use crate::wasm_client::map_2d_camera::local_user_can_author_map;
use crate::wasm_client::map_3d::{
    apply_incoming_map_update, mark_map_wgpu_dirty, refresh_map_dom_and_wgpu, with_map_ui,
    with_map_ui_mut,
};
use crate::wasm_client::map_edge_edit::{
    EDGE_PICK_TOLERANCE_CELLS, apply_edge_kind, nearest_edge_at,
};
use crate::wasm_client::state::with_state;
use satya_maps::map_dressing::{make_prop_at_cm, rotated_aabb_half_extents_cm};
use satya_maps::map_surface_geom::surface_contains_point_m;
use satya_maps::tactical_map_geom::prop_contains_point;
use satya_schema::{
    EdgeSide, GenerateSessionMapRequest, GmMapUpdateRequest, MapFloorLevel, MapPointCm,
    MapPropPlacement, MapSurfaceFeature, MapWallEdge, MaterialClass, PropKind, SurfaceFeatureKind,
    SurfaceShapeKind, WallEdgeKind, default_floor_badge, format_rgb_hex, parse_hex_rgb,
};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

/// Maximum per-editor Undo/Redo snapshot history depth.
pub const MAX_EDITOR_UNDO: usize = 30;

/// Selectable base ground pattern overrides for Editor Mode (`"texture_key"`, `"Label"`).
pub const GROUND_OVERRIDE_OPTIONS: &[(&str, &str)] = &[
    ("", "Theme Default (Auto)"),
    ("grass_meadow", "Meadow Grass"),
    ("grass_lush", "Lush Suburban Lawn"),
    ("grass_dry", "Dry Field Grass"),
    ("hardwood_plank", "Warm Hardwood Plank"),
    ("hardwood_walnut", "Dark Walnut Hardwood"),
    ("hardwood_ash", "Pale Ash Hardwood"),
    ("clinical_vinyl", "Clinical Hospital Vinyl"),
    ("linoleum_check", "Checkered Linoleum"),
    ("terrazzo_stone", "Polished Terrazzo"),
    ("ceramic_tile", "Ceramic Bath Tile"),
    ("marble_floor", "Polished Marble"),
    ("carpet_plush", "Plush Woven Carpet"),
    ("concrete_slab", "Clean Concrete Slab"),
    ("stained_concrete", "Stained Utility Concrete"),
    ("asphalt_road", "Asphalt Roadway"),
    ("herringbone_brick", "Herringbone Brick Pavers"),
    ("flagstone_paver", "Flagstone Courtyard"),
    ("server_raised_tile", "Server Room Raised Steel"),
    ("wood_deck", "Timber Decking"),
    ("packed_dirt", "Packed Dirt"),
    ("gravel_shoal", "Natural Gravel"),
];

/// Primary tool modes in the interactive Battlemap Editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTool {
    #[default]
    SelectMove,
    AddProp,
    PaintWall,
    PaintSurface,
    GroundStyle,
    Erase,
}

impl EditorTool {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SelectMove => "select",
            Self::AddProp => "prop",
            Self::PaintWall => "wall",
            Self::PaintSurface => "surface",
            Self::GroundStyle => "ground",
            Self::Erase => "erase",
        }
    }

    #[must_use]
    pub fn parse_code(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "prop" => Self::AddProp,
            "wall" => Self::PaintWall,
            "surface" => Self::PaintSurface,
            "ground" => Self::GroundStyle,
            "erase" => Self::Erase,
            _ => Self::SelectMove,
        }
    }
}

/// Edge stroke kind for [`EditorTool::PaintWall`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallBrushMode {
    #[default]
    Wall,
    DoorClosed,
    DoorOpen,
    Window,
    EraseEdge,
}

impl WallBrushMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wall => "wall",
            Self::DoorClosed => "door_closed",
            Self::DoorOpen => "door_open",
            Self::Window => "window",
            Self::EraseEdge => "erase",
        }
    }

    #[must_use]
    pub fn parse_code(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "door_closed" => Self::DoorClosed,
            "door_open" => Self::DoorOpen,
            "window" => Self::Window,
            "erase" => Self::EraseEdge,
            _ => Self::Wall,
        }
    }

    #[must_use]
    pub const fn to_edge_kind(self) -> Option<WallEdgeKind> {
        match self {
            Self::Wall => Some(WallEdgeKind::Wall),
            Self::DoorClosed => Some(WallEdgeKind::DoorClosed),
            Self::DoorOpen => Some(WallEdgeKind::DoorOpen),
            Self::Window => Some(WallEdgeKind::Window),
            Self::EraseEdge => None,
        }
    }
}

/// Geometric drawing mode for [`EditorTool::PaintSurface`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SurfaceBrushShape {
    #[default]
    Rect,
    Polygon,
    Ribbon,
}

impl SurfaceBrushShape {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Polygon => "polygon",
            Self::Ribbon => "ribbon",
        }
    }

    #[must_use]
    pub fn parse_code(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "polygon" => Self::Polygon,
            "ribbon" => Self::Ribbon,
            _ => Self::Rect,
        }
    }
}

/// Currently selected object or ground zone in Editor Mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorSelection {
    #[default]
    None,
    Prop(usize),
    Surface(usize),
}

/// Result flags returned by Editor pointer handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EditorPointerOutcome {
    pub needs_redraw: bool,
    pub needs_server_sync: bool,
    pub needs_dom_sync: bool,
}

/// Captures a complete active-floor geometry & styling snapshot for Undo/Redo or atomic server sync.
#[must_use]
pub fn snapshot_active_floor(map: &GmMapUpdateRequest) -> MapFloorLevel {
    let existing = map
        .floors
        .iter()
        .find(|f| f.floor_index == map.active_floor);
    MapFloorLevel {
        floor_index: map.active_floor,
        label: existing.map_or_else(
            || default_floor_badge(map.active_floor),
            |f| f.label.clone(),
        ),
        elevation_cm: existing.map_or(map.active_floor * 320, |f| f.elevation_cm),
        wall_edges: map.wall_edges.clone(),
        props: map.props.clone(),
        surfaces: map.surfaces.clone(),
        ground_floor_override: map.ground_floor_override.clone(),
        ground_tint_rgb: map.ground_tint_rgb,
    }
}

/// Restores a previously captured [`MapFloorLevel`] onto `map` and keeps `map.floors` in sync.
pub fn restore_floor_snapshot(map: &mut GmMapUpdateRequest, snap: &MapFloorLevel) {
    if map.has_multiple_floors() && map.active_floor != snap.floor_index {
        let _ = map.switch_active_floor(snap.floor_index);
    }
    map.wall_edges = snap.wall_edges.clone();
    map.props = snap.props.clone();
    map.surfaces = snap.surfaces.clone();
    map.ground_floor_override = snap.ground_floor_override.clone();
    map.ground_tint_rgb = snap.ground_tint_rgb;
    map.tiles.clear();
    map.sync_active_floor_snapshot();
}

/// Bounded 30-step Undo/Redo ring buffer over per-floor geometry snapshots.
#[derive(Debug, Clone, Default)]
pub struct EditorHistory {
    pub undo_stack: Vec<MapFloorLevel>,
    pub redo_stack: Vec<MapFloorLevel>,
}

impl EditorHistory {
    pub fn push_undo(&mut self, snap: MapFloorLevel) {
        if self.undo_stack.len() >= MAX_EDITOR_UNDO {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(snap);
        self.redo_stack.clear();
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self, map: &mut GmMapUpdateRequest) -> bool {
        let Some(prev) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(snapshot_active_floor(map));
        restore_floor_snapshot(map, &prev);
        true
    }

    pub fn redo(&mut self, map: &mut GmMapUpdateRequest) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        if self.undo_stack.len() >= MAX_EDITOR_UNDO {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(snapshot_active_floor(map));
        restore_floor_snapshot(map, &next);
        true
    }
}

/// Full interactive state for the 2D & 3D Battlemap Editor Mode.
#[derive(Debug, Clone)]
pub struct MapEditorState {
    pub active: bool,
    pub tool: EditorTool,
    pub selection: EditorSelection,
    pub snap_half_m: bool,
    pub prop_kind: PropKind,
    pub prop_rotation_deg: i32,
    pub prop_decorative: bool,
    pub prop_tint_rgb: Option<[u8; 3]>,
    pub wall_mode: WallBrushMode,
    pub wall_material: MaterialClass,
    pub wall_drag_start: Option<(i32, i32)>,
    pub wall_drag_cur: Option<(i32, i32)>,
    pub wall_down_pt_m: Option<(f32, f32)>,
    pub surface_kind: SurfaceFeatureKind,
    pub surface_shape: SurfaceBrushShape,
    pub surface_width_cm: i32,
    pub surface_tint_rgb: Option<[u8; 3]>,
    pub surface_rect_start_cm: Option<(i32, i32)>,
    pub surface_rect_cur_cm: Option<(i32, i32)>,
    pub surface_draft_pts_cm: Vec<MapPointCm>,
    pub dragging_prop_idx: Option<usize>,
    pub drag_offset_cm: (i32, i32),
    pub drag_undo_pushed: bool,
    pub drag_changed: bool,
    pub history: EditorHistory,
}

impl Default for MapEditorState {
    fn default() -> Self {
        Self {
            active: false,
            tool: EditorTool::SelectMove,
            selection: EditorSelection::None,
            snap_half_m: false,
            prop_kind: PropKind::Sofa,
            prop_rotation_deg: 0,
            prop_decorative: false,
            prop_tint_rgb: None,
            wall_mode: WallBrushMode::Wall,
            wall_material: MaterialClass::Drywall,
            wall_drag_start: None,
            wall_drag_cur: None,
            wall_down_pt_m: None,
            surface_kind: SurfaceFeatureKind::ClinicalVinylFloor,
            surface_shape: SurfaceBrushShape::Rect,
            surface_width_cm: 180,
            surface_tint_rgb: None,
            surface_rect_start_cm: None,
            surface_rect_cur_cm: None,
            surface_draft_pts_cm: Vec::new(),
            dragging_prop_idx: None,
            drag_offset_cm: (0, 0),
            drag_undo_pushed: false,
            drag_changed: false,
            history: EditorHistory::default(),
        }
    }
}

fn clamp_prop_center_cm(
    map: &GmMapUpdateRequest,
    cx_cm: i32,
    cy_cm: i32,
    snap_half: bool,
) -> (i32, i32) {
    let (bw_cm, bh_cm) = map.board_size_cm();
    let (sx, sy) = if snap_half {
        (
            ((cx_cm as f32 / 50.0).round() as i32) * 50,
            ((cy_cm as f32 / 50.0).round() as i32) * 50,
        )
    } else {
        (cx_cm, cy_cm)
    };
    (
        sx.clamp(20, (bw_cm - 20).max(20)),
        sy.clamp(20, (bh_cm - 20).max(20)),
    )
}

fn snap_point_cm(map: &GmMapUpdateRequest, wx_m: f32, wy_m: f32, snap_half: bool) -> (i32, i32) {
    let (bw_cm, bh_cm) = map.board_size_cm();
    let raw_x = (wx_m * 100.0).round() as i32;
    let raw_y = (wy_m * 100.0).round() as i32;
    let (sx, sy) = if snap_half {
        (
            ((raw_x as f32 / 50.0).round() as i32) * 50,
            ((raw_y as f32 / 50.0).round() as i32) * 50,
        )
    } else {
        (raw_x, raw_y)
    };
    (sx.clamp(0, bw_cm.max(100)), sy.clamp(0, bh_cm.max(100)))
}

fn update_prop_coarse_bbox(p: &mut MapPropPlacement) {
    let (cx_cm, cy_cm) = p.center_cm();
    let (lw_cm, lh_cm) = p.local_span_cm();
    let (hx_f, hy_f) = rotated_aabb_half_extents_cm(lw_cm, lh_cm, p.rotation_deg);
    let hx_cm = hx_f.round() as i32;
    let hy_cm = hy_f.round() as i32;
    p.x = (cx_cm - hx_cm).div_euclid(100);
    p.y = (cy_cm - hy_cm).div_euclid(100);
    p.w_cells = (((hx_cm * 2) + 99) / 100).max(1);
    p.h_cells = (((hy_cm * 2) + 99) / 100).max(1);
}

fn move_prop_center_cm(map: &GmMapUpdateRequest, p: &mut MapPropPlacement, cx_cm: i32, cy_cm: i32) {
    let (lw_cm, lh_cm) = p.local_span_cm();
    p.span_w_cm = Some(lw_cm);
    p.span_h_cm = Some(lh_cm);
    p.center_x_cm = Some(cx_cm);
    p.center_y_cm = Some(cy_cm);
    update_prop_coarse_bbox(p);
    let _ = map;
}

fn nearest_lattice_vertex(map: &GmMapUpdateRequest, wx_m: f32, wy_m: f32) -> (i32, i32) {
    let cell_m = map.cell_size_m().max(0.25);
    (
        (wx_m / cell_m).round().clamp(0.0, map.grid_width as f32) as i32,
        (wy_m / cell_m).round().clamp(0.0, map.grid_height as f32) as i32,
    )
}

/// Generates grid edges along the dominant X or Y axis between lattice vertices `start` and `end`.
#[must_use]
pub fn rasterize_wall_stroke_edges(
    map: &GmMapUpdateRequest,
    start: (i32, i32),
    end: (i32, i32),
) -> Vec<satya_schema::MapEdgeRef> {
    let (x0, y0) = start;
    let (x1, y1) = end;
    let dx = x1 - x0;
    let dy = y1 - y0;
    let mut out = Vec::new();
    if dx == 0 && dy == 0 {
        return out;
    }
    if dx.abs() >= dy.abs() {
        let (a, b) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let y = y0.clamp(0, map.grid_height);
        for x in a.max(0)..b.min(map.grid_width) {
            out.push(satya_schema::MapEdgeRef {
                x,
                y,
                side: EdgeSide::North,
            });
        }
    } else {
        let (a, b) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        let x = x0.clamp(0, map.grid_width);
        for y in a.max(0)..b.min(map.grid_height) {
            out.push(satya_schema::MapEdgeRef {
                x,
                y,
                side: EdgeSide::West,
            });
        }
    }
    out
}

/// Picks the topmost/smallest prop covering `(wx_m, wy_m)`, preferring small tabletop items over desks.
#[must_use]
pub fn pick_prop_at_m(props: &[MapPropPlacement], wx_m: f32, wy_m: f32) -> Option<usize> {
    let mut best: Option<(usize, u8, i64)> = None;
    for (idx, p) in props.iter().enumerate() {
        let exact = prop_contains_point(p, wx_m, wy_m, true);
        let fp = p.footprint_m();
        let pad_hit = if exact {
            false
        } else {
            let (lx, ly) = fp.world_to_local(wx_m, wy_m);
            lx.abs() <= fp.hx_m + 0.14 && ly.abs() <= fp.hy_m + 0.14
        };
        if !exact && !pad_hit {
            continue;
        }
        let penalty = u8::from(!exact);
        let area_cm2 = (fp.span_w_m * 100.0) as i64 * (fp.span_h_m * 100.0) as i64;
        let replace = match best {
            None => true,
            Some((b_idx, b_pen, b_area)) => {
                (penalty, area_cm2) < (b_pen, b_area)
                    || ((penalty, area_cm2) == (b_pen, b_area) && idx > b_idx)
            }
        };
        if replace {
            best = Some((idx, penalty, area_cm2));
        }
    }
    best.map(|(i, _, _)| i)
}

/// Picks the topmost vector ground/floor surface covering `(wx_m, wy_m)`.
#[must_use]
pub fn pick_surface_at_m(surfaces: &[MapSurfaceFeature], wx_m: f32, wy_m: f32) -> Option<usize> {
    let mut best: Option<(usize, u8)> = None;
    for (idx, s) in surfaces.iter().enumerate() {
        let prio = s.kind.layer_priority();
        if let Some((_, best_prio)) = best
            && prio < best_prio
        {
            continue;
        }
        if surface_contains_point_m(s, wx_m, wy_m) {
            best = Some((idx, prio));
        }
    }
    best.map(|(i, _)| i)
}

/// Picks either the topmost prop or (if none) the topmost ground surface at `(wx_m, wy_m)`.
#[must_use]
pub fn pick_prop_or_surface_at(map: &GmMapUpdateRequest, wx_m: f32, wy_m: f32) -> EditorSelection {
    if let Some(p_idx) = pick_prop_at_m(&map.props, wx_m, wy_m) {
        EditorSelection::Prop(p_idx)
    } else if let Some(s_idx) = pick_surface_at_m(&map.surfaces, wx_m, wy_m) {
        EditorSelection::Surface(s_idx)
    } else {
        EditorSelection::None
    }
}

impl MapEditorState {
    pub fn clear_transient_drafts(&mut self) {
        self.wall_drag_start = None;
        self.wall_drag_cur = None;
        self.wall_down_pt_m = None;
        self.surface_rect_start_cm = None;
        self.surface_rect_cur_cm = None;
        self.surface_draft_pts_cm.clear();
        self.dragging_prop_idx = None;
        self.drag_undo_pushed = false;
        self.drag_changed = false;
    }

    pub fn validate_selection(&mut self, map: &GmMapUpdateRequest) {
        match self.selection {
            EditorSelection::Prop(i) if i >= map.props.len() => {
                self.selection = EditorSelection::None;
            }
            EditorSelection::Surface(i) if i >= map.surfaces.len() => {
                self.selection = EditorSelection::None;
            }
            _ => {}
        }
    }

    pub fn on_pointer_down(
        &mut self,
        map: &mut GmMapUpdateRequest,
        wx_m: f32,
        wy_m: f32,
    ) -> EditorPointerOutcome {
        if !wx_m.is_finite() || !wy_m.is_finite() {
            return EditorPointerOutcome::default();
        }
        let (bw_m, bh_m) = map.board_size_m();
        let wx = wx_m.clamp(0.0, bw_m);
        let wy = wy_m.clamp(0.0, bh_m);
        match self.tool {
            EditorTool::SelectMove => {
                self.dragging_prop_idx = None;
                self.drag_undo_pushed = false;
                self.drag_changed = false;
                let picked = pick_prop_or_surface_at(map, wx, wy);
                self.selection = picked;
                if let EditorSelection::Prop(idx) = picked
                    && let Some(p) = map.props.get(idx)
                {
                    let (pcx, pcy) = p.center_cm();
                    let click_x = (wx * 100.0).round() as i32;
                    let click_y = (wy * 100.0).round() as i32;
                    self.dragging_prop_idx = Some(idx);
                    self.drag_offset_cm = (pcx - click_x, pcy - click_y);
                }
                EditorPointerOutcome {
                    needs_redraw: true,
                    needs_server_sync: false,
                    needs_dom_sync: true,
                }
            }
            EditorTool::AddProp => {
                let (cx_cm, cy_cm) = clamp_prop_center_cm(
                    map,
                    (wx * 100.0).round() as i32,
                    (wy * 100.0).round() as i32,
                    self.snap_half_m,
                );
                self.history.push_undo(snapshot_active_floor(map));
                let seed = (map.seed.unwrap_or(42) as u32)
                    .wrapping_mul(31)
                    .wrapping_add(
                        (cx_cm as u32).wrapping_mul(17) ^ (cy_cm as u32).wrapping_mul(53),
                    );
                let mut prop = make_prop_at_cm(
                    self.prop_kind,
                    cx_cm,
                    cy_cm,
                    self.prop_rotation_deg,
                    None,
                    seed,
                );
                prop.tint_rgb = self.prop_tint_rgb;
                prop.decorative = self.prop_decorative;
                map.props.push(prop);
                self.selection = EditorSelection::Prop(map.props.len() - 1);
                map.sync_active_floor_snapshot();
                EditorPointerOutcome {
                    needs_redraw: true,
                    needs_server_sync: true,
                    needs_dom_sync: true,
                }
            }
            EditorTool::PaintWall => {
                let v = nearest_lattice_vertex(map, wx, wy);
                self.wall_drag_start = Some(v);
                self.wall_drag_cur = Some(v);
                self.wall_down_pt_m = Some((wx, wy));
                EditorPointerOutcome {
                    needs_redraw: true,
                    needs_server_sync: false,
                    needs_dom_sync: false,
                }
            }
            EditorTool::PaintSurface => match self.surface_shape {
                SurfaceBrushShape::Rect => {
                    let pt = snap_point_cm(map, wx, wy, self.snap_half_m);
                    self.surface_rect_start_cm = Some(pt);
                    self.surface_rect_cur_cm = Some(pt);
                    EditorPointerOutcome {
                        needs_redraw: true,
                        needs_server_sync: false,
                        needs_dom_sync: false,
                    }
                }
                SurfaceBrushShape::Polygon => {
                    let (px_cm, py_cm) = snap_point_cm(map, wx, wy, self.snap_half_m);
                    if self.surface_draft_pts_cm.len() >= 3 {
                        let f = self.surface_draft_pts_cm[0];
                        let dist = (((px_cm - f.x_cm) as f32) / 100.0)
                            .hypot(((py_cm - f.y_cm) as f32) / 100.0);
                        if dist <= 0.48 && self.finish_draft_surface(map) {
                            return EditorPointerOutcome {
                                needs_redraw: true,
                                needs_server_sync: true,
                                needs_dom_sync: true,
                            };
                        }
                    }
                    self.surface_draft_pts_cm
                        .push(MapPointCm::new(px_cm, py_cm));
                    EditorPointerOutcome {
                        needs_redraw: true,
                        needs_server_sync: false,
                        needs_dom_sync: true,
                    }
                }
                SurfaceBrushShape::Ribbon => {
                    let (px_cm, py_cm) = snap_point_cm(map, wx, wy, self.snap_half_m);
                    self.surface_draft_pts_cm
                        .push(MapPointCm::new(px_cm, py_cm));
                    EditorPointerOutcome {
                        needs_redraw: true,
                        needs_server_sync: false,
                        needs_dom_sync: true,
                    }
                }
            },
            EditorTool::GroundStyle => EditorPointerOutcome::default(),
            EditorTool::Erase => {
                if let Some(p_idx) = pick_prop_at_m(&map.props, wx, wy) {
                    self.history.push_undo(snapshot_active_floor(map));
                    map.props.remove(p_idx);
                    self.selection = EditorSelection::None;
                    map.sync_active_floor_snapshot();
                    return EditorPointerOutcome {
                        needs_redraw: true,
                        needs_server_sync: true,
                        needs_dom_sync: true,
                    };
                }
                let cell_m = map.cell_size_m().max(0.25);
                if let Some(edge_ref) = nearest_edge_at(
                    wx / cell_m,
                    wy / cell_m,
                    map.grid_width,
                    map.grid_height,
                    EDGE_PICK_TOLERANCE_CELLS.max(0.28),
                ) && map
                    .wall_edges
                    .iter()
                    .any(|e| e.x == edge_ref.x && e.y == edge_ref.y && e.side == edge_ref.side)
                {
                    self.history.push_undo(snapshot_active_floor(map));
                    apply_edge_kind(map, edge_ref, None);
                    map.sync_active_floor_snapshot();
                    return EditorPointerOutcome {
                        needs_redraw: true,
                        needs_server_sync: true,
                        needs_dom_sync: true,
                    };
                }
                if let Some(s_idx) = pick_surface_at_m(&map.surfaces, wx, wy) {
                    self.history.push_undo(snapshot_active_floor(map));
                    map.surfaces.remove(s_idx);
                    self.selection = EditorSelection::None;
                    map.sync_active_floor_snapshot();
                    return EditorPointerOutcome {
                        needs_redraw: true,
                        needs_server_sync: true,
                        needs_dom_sync: true,
                    };
                }
                EditorPointerOutcome::default()
            }
        }
    }

    pub fn on_pointer_move(&mut self, map: &mut GmMapUpdateRequest, wx_m: f32, wy_m: f32) -> bool {
        if !wx_m.is_finite() || !wy_m.is_finite() {
            return false;
        }
        let (bw_m, bh_m) = map.board_size_m();
        let wx = wx_m.clamp(0.0, bw_m);
        let wy = wy_m.clamp(0.0, bh_m);

        if self.tool == EditorTool::SelectMove
            && let Some(idx) = self.dragging_prop_idx
            && idx < map.props.len()
        {
            let target_x = (wx * 100.0).round() as i32 + self.drag_offset_cm.0;
            let target_y = (wy * 100.0).round() as i32 + self.drag_offset_cm.1;
            let (nx_cm, ny_cm) = clamp_prop_center_cm(map, target_x, target_y, self.snap_half_m);
            let (old_x, old_y) = map.props[idx].center_cm();
            if (nx_cm - old_x).abs() >= 3 || (ny_cm - old_y).abs() >= 3 {
                if !self.drag_undo_pushed {
                    self.history.push_undo(snapshot_active_floor(map));
                    self.drag_undo_pushed = true;
                }
                let mut prop = map.props[idx];
                move_prop_center_cm(map, &mut prop, nx_cm, ny_cm);
                map.props[idx] = prop;
                self.drag_changed = true;
                map.sync_active_floor_snapshot();
                return true;
            }
            return false;
        }
        if self.tool == EditorTool::PaintWall && self.wall_drag_start.is_some() {
            let v = nearest_lattice_vertex(map, wx, wy);
            if self.wall_drag_cur != Some(v) {
                self.wall_drag_cur = Some(v);
                return true;
            }
            return false;
        }
        if self.tool == EditorTool::PaintSurface
            && self.surface_shape == SurfaceBrushShape::Rect
            && self.surface_rect_start_cm.is_some()
        {
            let cur = snap_point_cm(map, wx, wy, self.snap_half_m);
            if self.surface_rect_cur_cm != Some(cur) {
                self.surface_rect_cur_cm = Some(cur);
                return true;
            }
        }
        false
    }

    pub fn on_pointer_up(
        &mut self,
        map: &mut GmMapUpdateRequest,
        wx_m: f32,
        wy_m: f32,
    ) -> EditorPointerOutcome {
        let (bw_m, bh_m) = map.board_size_m();
        let wx = wx_m.clamp(0.0, bw_m);
        let wy = wy_m.clamp(0.0, bh_m);

        if self.tool == EditorTool::SelectMove && self.dragging_prop_idx.take().is_some() {
            let changed = std::mem::take(&mut self.drag_changed);
            self.drag_undo_pushed = false;
            return EditorPointerOutcome {
                needs_redraw: changed,
                needs_server_sync: changed,
                needs_dom_sync: true,
            };
        }
        if self.tool == EditorTool::PaintWall
            && let Some(start) = self.wall_drag_start.take()
        {
            let end = self
                .wall_drag_cur
                .take()
                .unwrap_or_else(|| nearest_lattice_vertex(map, wx, wy));
            let down_pt = self.wall_down_pt_m.take().unwrap_or((wx, wy));
            let mut refs = rasterize_wall_stroke_edges(map, start, end);
            if refs.is_empty() {
                let cell_m = map.cell_size_m().max(0.25);
                if let Some(r) = nearest_edge_at(
                    down_pt.0 / cell_m,
                    down_pt.1 / cell_m,
                    map.grid_width,
                    map.grid_height,
                    EDGE_PICK_TOLERANCE_CELLS.max(0.35),
                ) {
                    refs.push(r);
                }
            }
            if refs.is_empty() {
                return EditorPointerOutcome {
                    needs_redraw: true,
                    needs_server_sync: false,
                    needs_dom_sync: false,
                };
            }
            self.history.push_undo(snapshot_active_floor(map));
            let target_kind = self.wall_mode.to_edge_kind();
            for r in refs {
                if let Some(k) = target_kind {
                    if let Some(slot) = map
                        .wall_edges
                        .iter_mut()
                        .find(|e| e.x == r.x && e.y == r.y && e.side == r.side)
                    {
                        slot.kind = k;
                        slot.material = self.wall_material;
                    } else {
                        map.wall_edges.push(MapWallEdge {
                            x: r.x,
                            y: r.y,
                            side: r.side,
                            kind: k,
                            material: self.wall_material,
                        });
                    }
                } else {
                    apply_edge_kind(map, r, None);
                }
            }
            map.sync_active_floor_snapshot();
            return EditorPointerOutcome {
                needs_redraw: true,
                needs_server_sync: true,
                needs_dom_sync: true,
            };
        }
        if self.tool == EditorTool::PaintSurface
            && self.surface_shape == SurfaceBrushShape::Rect
            && let Some(p0) = self.surface_rect_start_cm.take()
        {
            let p1 = self
                .surface_rect_cur_cm
                .take()
                .unwrap_or_else(|| snap_point_cm(map, wx, wy, self.snap_half_m));
            let (x0, x1) = (p0.0.min(p1.0), p0.0.max(p1.0));
            let (y0, y1) = (p0.1.min(p1.1), p0.1.max(p1.1));
            if x1 - x0 < 40 || y1 - y0 < 40 {
                return EditorPointerOutcome {
                    needs_redraw: true,
                    needs_server_sync: false,
                    needs_dom_sync: false,
                };
            }
            self.history.push_undo(snapshot_active_floor(map));
            map.surfaces.push(MapSurfaceFeature {
                kind: self.surface_kind,
                shape: SurfaceShapeKind::Polygon,
                points_cm: vec![
                    MapPointCm::new(x0, y0),
                    MapPointCm::new(x1, y0),
                    MapPointCm::new(x1, y1),
                    MapPointCm::new(x0, y1),
                ],
                width_cm: 0,
                border_style_cm: 0,
                variant_seed: (map.seed.unwrap_or(42) as u32)
                    .wrapping_add((map.surfaces.len() as u32).wrapping_mul(19)),
                tint_rgb: self.surface_tint_rgb,
            });
            self.selection = EditorSelection::Surface(map.surfaces.len() - 1);
            map.sync_active_floor_snapshot();
            return EditorPointerOutcome {
                needs_redraw: true,
                needs_server_sync: true,
                needs_dom_sync: true,
            };
        }
        EditorPointerOutcome::default()
    }

    pub fn finish_draft_surface(&mut self, map: &mut GmMapUpdateRequest) -> bool {
        let min_pts = match self.surface_shape {
            SurfaceBrushShape::Ribbon => 2,
            SurfaceBrushShape::Polygon => 3,
            SurfaceBrushShape::Rect => return false,
        };
        if self.surface_draft_pts_cm.len() < min_pts {
            return false;
        }
        let pts = std::mem::take(&mut self.surface_draft_pts_cm);
        self.history.push_undo(snapshot_active_floor(map));
        let (shape, width_cm) = match self.surface_shape {
            SurfaceBrushShape::Ribbon => (
                SurfaceShapeKind::RibbonSpline,
                self.surface_width_cm.clamp(40, 1200),
            ),
            _ => (SurfaceShapeKind::Polygon, 0),
        };
        map.surfaces.push(MapSurfaceFeature {
            kind: self.surface_kind,
            shape,
            points_cm: pts,
            width_cm,
            border_style_cm: 0,
            variant_seed: (map.seed.unwrap_or(42) as u32)
                .wrapping_add((map.surfaces.len() as u32).wrapping_mul(29)),
            tint_rgb: self.surface_tint_rgb,
        });
        self.selection = EditorSelection::Surface(map.surfaces.len() - 1);
        map.sync_active_floor_snapshot();
        true
    }

    pub fn rotate_selected_prop(&mut self, map: &mut GmMapUpdateRequest, delta_deg: i32) -> bool {
        let EditorSelection::Prop(idx) = self.selection else {
            return false;
        };
        let Some(cur) = map.props.get(idx).map(|p| p.rotation_deg) else {
            return false;
        };
        self.set_selected_prop_rotation(map, (cur + delta_deg).rem_euclid(360))
    }

    pub fn set_selected_prop_rotation(&mut self, map: &mut GmMapUpdateRequest, deg: i32) -> bool {
        let EditorSelection::Prop(idx) = self.selection else {
            return false;
        };
        if idx >= map.props.len() {
            return false;
        }
        self.history.push_undo(snapshot_active_floor(map));
        let mut p = map.props[idx];
        // Freeze local_span_cm before changing rotation across orthogonal 90/0 boundaries.
        let (lw_cm, lh_cm) = p.local_span_cm();
        let (cx_cm, cy_cm) = p.center_cm();
        p.span_w_cm = Some(lw_cm);
        p.span_h_cm = Some(lh_cm);
        p.center_x_cm = Some(cx_cm);
        p.center_y_cm = Some(cy_cm);
        p.rotation_deg = deg.rem_euclid(360);
        update_prop_coarse_bbox(&mut p);
        map.props[idx] = p;
        map.sync_active_floor_snapshot();
        true
    }

    pub fn stretch_selected_prop(&mut self, map: &mut GmMapUpdateRequest, delta_cm: i32) -> bool {
        let EditorSelection::Prop(idx) = self.selection else {
            return false;
        };
        if idx >= map.props.len() {
            return false;
        }
        self.history.push_undo(snapshot_active_floor(map));
        let mut p = map.props[idx];
        let (lw_cm, lh_cm) = p.local_span_cm();
        let (cx_cm, cy_cm) = p.center_cm();
        let next_w = (lw_cm + delta_cm).clamp(40, 1200);
        p.span_w_cm = Some(next_w);
        p.span_h_cm = Some(lh_cm);
        p.center_x_cm = Some(cx_cm);
        p.center_y_cm = Some(cy_cm);
        update_prop_coarse_bbox(&mut p);
        map.props[idx] = p;
        map.sync_active_floor_snapshot();
        true
    }

    pub fn set_selected_tint(
        &mut self,
        map: &mut GmMapUpdateRequest,
        tint: Option<[u8; 3]>,
    ) -> bool {
        match self.selection {
            EditorSelection::Prop(idx) if idx < map.props.len() => {
                self.history.push_undo(snapshot_active_floor(map));
                map.props[idx].tint_rgb = tint;
                map.sync_active_floor_snapshot();
                true
            }
            EditorSelection::Surface(idx) if idx < map.surfaces.len() => {
                self.history.push_undo(snapshot_active_floor(map));
                map.surfaces[idx].tint_rgb = tint;
                map.sync_active_floor_snapshot();
                true
            }
            _ => false,
        }
    }

    pub fn set_selected_prop_decorative(
        &mut self,
        map: &mut GmMapUpdateRequest,
        decorative: bool,
    ) -> bool {
        let EditorSelection::Prop(idx) = self.selection else {
            return false;
        };
        if idx >= map.props.len() {
            return false;
        }
        self.history.push_undo(snapshot_active_floor(map));
        map.props[idx].decorative = decorative;
        map.sync_active_floor_snapshot();
        true
    }

    pub fn set_selected_surface_kind(
        &mut self,
        map: &mut GmMapUpdateRequest,
        kind: SurfaceFeatureKind,
    ) -> bool {
        let EditorSelection::Surface(idx) = self.selection else {
            return false;
        };
        if idx >= map.surfaces.len() {
            return false;
        }
        self.history.push_undo(snapshot_active_floor(map));
        map.surfaces[idx].kind = kind;
        map.sync_active_floor_snapshot();
        true
    }

    pub fn duplicate_selected(&mut self, map: &mut GmMapUpdateRequest) -> bool {
        let EditorSelection::Prop(idx) = self.selection else {
            return false;
        };
        if idx >= map.props.len() {
            return false;
        }
        self.history.push_undo(snapshot_active_floor(map));
        let mut copy = map.props[idx];
        let (cx_cm, cy_cm) = copy.center_cm();
        let (nx_cm, ny_cm) = clamp_prop_center_cm(map, cx_cm + 60, cy_cm + 60, self.snap_half_m);
        move_prop_center_cm(map, &mut copy, nx_cm, ny_cm);
        map.props.push(copy);
        self.selection = EditorSelection::Prop(map.props.len() - 1);
        map.sync_active_floor_snapshot();
        true
    }

    pub fn delete_selected(&mut self, map: &mut GmMapUpdateRequest) -> bool {
        match self.selection {
            EditorSelection::Prop(idx) if idx < map.props.len() => {
                self.history.push_undo(snapshot_active_floor(map));
                map.props.remove(idx);
                self.selection = EditorSelection::None;
                map.sync_active_floor_snapshot();
                true
            }
            EditorSelection::Surface(idx) if idx < map.surfaces.len() => {
                self.history.push_undo(snapshot_active_floor(map));
                map.surfaces.remove(idx);
                self.selection = EditorSelection::None;
                map.sync_active_floor_snapshot();
                true
            }
            _ => false,
        }
    }

    pub fn set_ground_style(
        &mut self,
        map: &mut GmMapUpdateRequest,
        floor_override: Option<String>,
        tint_rgb: Option<[u8; 3]>,
    ) -> bool {
        let cleaned = floor_override.and_then(|s| {
            let t = s.trim().to_string();
            (!t.is_empty()).then_some(t)
        });
        if map.ground_floor_override == cleaned && map.ground_tint_rgb == tint_rgb {
            return false;
        }
        self.history.push_undo(snapshot_active_floor(map));
        map.ground_floor_override = cleaned;
        map.ground_tint_rgb = tint_rgb;
        map.sync_active_floor_snapshot();
        true
    }
}

thread_local! {
    static EDITOR_STATE: RefCell<MapEditorState> = RefCell::new(MapEditorState::default());
}

pub fn with_editor<F, R>(f: F) -> R
where
    F: FnOnce(&MapEditorState) -> R,
{
    EDITOR_STATE.with(|c| f(&c.borrow()))
}

pub fn with_editor_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut MapEditorState) -> R,
{
    EDITOR_STATE.with(|c| f(&mut c.borrow_mut()))
}

/// Returns `true` when Editor Mode is active and the viewer has map authoring permission.
#[must_use]
pub fn is_editor_active() -> bool {
    with_editor(|ed| ed.active) && local_user_can_author_map()
}

/// Clears transient drafts and selection when switching floors or generating a new map.
pub fn reset_editor_selection_and_drafts(clear_history: bool) {
    with_editor_mut(|ed| {
        ed.selection = EditorSelection::None;
        ed.clear_transient_drafts();
        if clear_history {
            ed.history.clear();
        }
    });
}

fn sync_active_floor_to_server() {
    let (auth_token, session_id) = with_state(|st| (st.auth_token.clone(), st.session_id.clone()));
    let (Some(token), Some(sid)) = (auth_token, session_id) else {
        return;
    };
    if !local_user_can_author_map() {
        return;
    }
    let (preset, floor_snap) = with_map_ui(|ui| {
        (
            ui.map
                .theme_preset
                .clone()
                .unwrap_or_else(|| "orbital_airlock".to_string()),
            snapshot_active_floor(&ui.map),
        )
    });
    let req = GenerateSessionMapRequest {
        preset,
        populate_roster_tokens: false,
        replace_floor_geometry: Some(floor_snap),
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

fn commit_outcome_and_refresh(out: EditorPointerOutcome) {
    if out.needs_redraw {
        with_map_ui_mut(|ui| {
            ui.ensure_vector_scene_populated();
            ui.recompute_overlays();
        });
        mark_map_wgpu_dirty();
        refresh_map_dom_and_wgpu();
    } else if out.needs_dom_sync {
        sync_map_editor_dom();
    }
    if out.needs_server_sync {
        sync_active_floor_to_server();
    }
}

/// Handles a left-button press inside the 2D SVG or 3D WebGPU canvas when Editor Mode is active.
pub fn handle_editor_pointer_down(wx_m: f32, wy_m: f32) {
    if !is_editor_active() {
        return;
    }
    let out = with_map_ui_mut(|ui| {
        ui.ensure_vector_scene_populated();
        with_editor_mut(|ed| ed.on_pointer_down(&mut ui.map, wx_m, wy_m))
    });
    commit_outcome_and_refresh(out);
}

/// Handles pointer motion inside the 2D SVG or 3D WebGPU canvas when Editor Mode is active.
pub fn handle_editor_pointer_move(wx_m: f32, wy_m: f32) -> bool {
    if !is_editor_active() {
        return false;
    }
    let moved = with_map_ui_mut(|ui| {
        ui.hovered_point_m = Some((wx_m, wy_m));
        with_editor_mut(|ed| ed.on_pointer_move(&mut ui.map, wx_m, wy_m))
    });
    if moved {
        mark_map_wgpu_dirty();
        sync_battlemap_svg_host();
        crate::wasm_client::map_3d::refresh_map_wgpu_if_active();
    }
    moved
}

/// Handles left-button release inside the 2D SVG or 3D WebGPU canvas when Editor Mode is active.
pub fn handle_editor_pointer_up(wx_m: f32, wy_m: f32) {
    if !is_editor_active() {
        return;
    }
    let out = with_map_ui_mut(|ui| with_editor_mut(|ed| ed.on_pointer_up(&mut ui.map, wx_m, wy_m)));
    commit_outcome_and_refresh(out);
}

/// Handles keyboard shortcuts while the `Maps` pane and Editor Mode are active (`Ctrl+Z`/`Y`, `R`, `Del`, `Esc`, `Enter`).
pub fn handle_editor_keydown(ev: &web_sys::KeyboardEvent) -> bool {
    let key = ev.key();
    if ev.ctrl_key() || ev.meta_key() {
        if !is_editor_active() {
            return false;
        }
        let low = key.to_ascii_lowercase();
        if low == "s" {
            ev.prevent_default();
            crate::wasm_client::map_vault::satya_save_battlemap(false);
            return true;
        }
        if low == "z" && !ev.shift_key() {
            ev.prevent_default();
            satya_editor_undo();
            return true;
        }
        if low == "y" || (low == "z" && ev.shift_key()) {
            ev.prevent_default();
            satya_editor_redo();
            return true;
        }
        return false;
    }
    if key.eq_ignore_ascii_case("e") {
        if local_user_can_author_map() {
            satya_toggle_map_editor();
            return true;
        }
        return false;
    }
    if !is_editor_active() {
        return false;
    }
    match key.as_str() {
        "r" | "R" => {
            let delta = if ev.shift_key() { -15 } else { 15 };
            let rotated_sel = with_editor(|ed| matches!(ed.selection, EditorSelection::Prop(_)));
            if rotated_sel {
                satya_editor_rotate_selected(delta);
            } else {
                with_editor_mut(|ed| {
                    ed.prop_rotation_deg = (ed.prop_rotation_deg + delta).rem_euclid(360);
                });
                sync_map_editor_dom();
                sync_battlemap_svg_host();
            }
            true
        }
        "Delete" | "Backspace" => {
            satya_editor_delete_selected();
            true
        }
        "Enter" => {
            if with_editor(|ed| !ed.surface_draft_pts_cm.is_empty()) {
                satya_finish_editor_surface();
                return true;
            }
            false
        }
        "Escape" => {
            let cleared = with_editor_mut(|ed| {
                let had_draft = !ed.surface_draft_pts_cm.is_empty()
                    || ed.wall_drag_start.is_some()
                    || ed.surface_rect_start_cm.is_some();
                ed.clear_transient_drafts();
                let had_sel = ed.selection != EditorSelection::None;
                ed.selection = EditorSelection::None;
                had_draft || had_sel
            });
            if cleared {
                sync_map_editor_dom();
                sync_battlemap_svg_host();
                mark_map_wgpu_dirty();
                crate::wasm_client::map_3d::refresh_map_wgpu_if_active();
            }
            true
        }
        _ => false,
    }
}

/// Synchronizes `#btn-map-editor-toggle`, `#map-editor-toolbar`, tool sub-panels, and `#map-editor-inspector`.
pub fn sync_map_editor_dom() {
    let can_author = local_user_can_author_map();
    if !can_author {
        with_editor_mut(|ed| ed.active = false);
    }
    set_display(
        "btn-map-editor-toggle",
        if can_author { "inline-flex" } else { "none" },
    );

    let (
        active,
        tool,
        snap,
        can_undo,
        can_redo,
        prop_kind,
        prop_rot,
        prop_dec,
        wall_mode,
        wall_mat,
        surf_kind,
        surf_shape,
        surf_w,
        draft_len,
        sel,
        sel_label,
        sel_rot,
        sel_dec,
        sel_tint_hex,
        sel_surf_kind,
        ground_ov,
        ground_tint_hex,
    ) = with_map_ui(|ui| {
        with_editor_mut(|ed| {
            ed.validate_selection(&ui.map);
            let (sel_label, sel_rot, sel_dec, sel_tint_hex, sel_surf_kind) = match ed.selection {
                EditorSelection::Prop(i) if i < ui.map.props.len() => {
                    let p = &ui.map.props[i];
                    let (sw, sh) = p.local_span_m();
                    (
                        format!(
                            "Object #{}: {} ({:.1}m×{:.1}m)",
                            i + 1,
                            p.kind.label(),
                            sw,
                            sh
                        ),
                        p.rotation_deg,
                        p.decorative,
                        p.tint_rgb
                            .map(format_rgb_hex)
                            .unwrap_or_else(|| "#7c8ea6".to_string()),
                        ui.map
                            .surfaces
                            .first()
                            .map_or(SurfaceFeatureKind::LawnGrassZone, |s| s.kind),
                    )
                }
                EditorSelection::Surface(i) if i < ui.map.surfaces.len() => {
                    let s = &ui.map.surfaces[i];
                    (
                        format!(
                            "Ground Zone #{}: {} ({} pts)",
                            i + 1,
                            s.kind.label(),
                            s.points_cm.len()
                        ),
                        0,
                        false,
                        s.tint_rgb
                            .map(format_rgb_hex)
                            .unwrap_or_else(|| "#7c8ea6".to_string()),
                        s.kind,
                    )
                }
                _ => (
                    String::new(),
                    0,
                    false,
                    "#7c8ea6".to_string(),
                    SurfaceFeatureKind::LawnGrassZone,
                ),
            };
            (
                ed.active && can_author,
                ed.tool,
                ed.snap_half_m,
                ed.history.can_undo(),
                ed.history.can_redo(),
                ed.prop_kind,
                ed.prop_rotation_deg,
                ed.prop_decorative,
                ed.wall_mode,
                ed.wall_material,
                ed.surface_kind,
                ed.surface_shape,
                ed.surface_width_cm,
                ed.surface_draft_pts_cm.len(),
                ed.selection,
                sel_label,
                sel_rot,
                sel_dec,
                sel_tint_hex,
                sel_surf_kind,
                ui.map.ground_floor_override.clone().unwrap_or_default(),
                ui.map
                    .ground_tint_rgb
                    .map(format_rgb_hex)
                    .unwrap_or_else(|| "#6b7280".to_string()),
            )
        })
    });

    set_text(
        "btn-map-editor-toggle",
        if active {
            "Editor: ON [E]"
        } else {
            "Editor [E]"
        },
    );
    set_class(
        "btn-map-editor-toggle",
        if active {
            "btn btn-sm btn-accent"
        } else {
            "btn btn-sm btn-secondary"
        },
    );
    set_display("map-editor-toolbar", if active { "flex" } else { "none" });
    if !active {
        return;
    }

    for (t, id) in [
        (EditorTool::SelectMove, "btn-editor-tool-select"),
        (EditorTool::AddProp, "btn-editor-tool-prop"),
        (EditorTool::PaintWall, "btn-editor-tool-wall"),
        (EditorTool::PaintSurface, "btn-editor-tool-surface"),
        (EditorTool::GroundStyle, "btn-editor-tool-ground"),
        (EditorTool::Erase, "btn-editor-tool-erase"),
    ] {
        set_class(
            id,
            if tool == t {
                "btn btn-sm btn-accent"
            } else {
                "btn btn-sm btn-secondary"
            },
        );
    }

    set_display(
        "map-editor-panel-prop",
        if tool == EditorTool::AddProp {
            "flex"
        } else {
            "none"
        },
    );
    set_display(
        "map-editor-panel-wall",
        if tool == EditorTool::PaintWall {
            "flex"
        } else {
            "none"
        },
    );
    set_display(
        "map-editor-panel-surface",
        if tool == EditorTool::PaintSurface {
            "flex"
        } else {
            "none"
        },
    );
    set_display(
        "map-editor-panel-ground",
        if tool == EditorTool::GroundStyle {
            "flex"
        } else {
            "none"
        },
    );

    set_input_checked("map-editor-snap-chk", snap);
    set_button_disabled("btn-editor-undo", !can_undo);
    set_button_disabled("btn-editor-redo", !can_redo);

    set_select_value("map-editor-prop-select", prop_kind.as_str());
    set_input_value("map-editor-prop-rot", &prop_rot.to_string());
    set_input_checked("map-editor-prop-decorative", prop_dec);

    set_select_value("map-editor-wall-kind", wall_mode.as_str());
    set_select_value("map-editor-wall-mat", wall_mat.as_str());

    set_select_value("map-editor-surface-kind", surf_kind.as_str());
    set_select_value("map-editor-surface-shape", surf_shape.as_str());
    set_input_value("map-editor-surface-width", &surf_w.to_string());

    set_select_value("map-editor-ground-override", &ground_ov);
    set_input_value("map-editor-ground-tint", &ground_tint_hex);

    let show_draft_btns =
        tool == EditorTool::PaintSurface && surf_shape != SurfaceBrushShape::Rect && draft_len > 0;
    set_display(
        "btn-editor-surface-finish",
        if show_draft_btns {
            "inline-flex"
        } else {
            "none"
        },
    );
    set_display(
        "btn-editor-surface-cancel",
        if show_draft_btns {
            "inline-flex"
        } else {
            "none"
        },
    );

    let hint = match tool {
        EditorTool::SelectMove => "Click prop/zone to select · Drag prop to move · Right-drag pan",
        EditorTool::AddProp => "Click map to place object · [R]/[Shift+R] rotate",
        EditorTool::PaintWall => "Drag across grid to draw wall/door/window line",
        EditorTool::PaintSurface => match surf_shape {
            SurfaceBrushShape::Rect => "Drag rectangle to paint ground/floor zone",
            SurfaceBrushShape::Polygon => {
                if draft_len == 0 {
                    "Click polygon corners · Click start or press Enter to finish"
                } else {
                    "Polygon draft active · Click start or Finish to commit"
                }
            }
            SurfaceBrushShape::Ribbon => {
                if draft_len == 0 {
                    "Click path control points · Press Enter or Finish to commit"
                } else {
                    "Path draft active · Add points then Finish"
                }
            }
        },
        EditorTool::GroundStyle => "Choose base ground texture override & tint for this floor",
        EditorTool::Erase => "Click any object, wall edge, or ground zone to remove",
    };
    set_text("map-editor-status-pill", hint);

    let has_sel = sel != EditorSelection::None && !sel_label.is_empty();
    set_display(
        "map-editor-inspector",
        if has_sel { "flex" } else { "none" },
    );
    if has_sel {
        set_text("map-editor-sel-label", &sel_label);
        let is_prop = matches!(sel, EditorSelection::Prop(_));
        set_display(
            "map-editor-sel-prop-controls",
            if is_prop { "inline-flex" } else { "none" },
        );
        set_display(
            "map-editor-sel-surface-wrap",
            if is_prop { "none" } else { "inline-flex" },
        );
        if is_prop {
            set_input_value("map-editor-sel-rot", &sel_rot.to_string());
            set_input_checked("map-editor-sel-decorative", sel_dec);
        } else {
            set_select_value("map-editor-sel-surface-kind", sel_surf_kind.as_str());
        }
        set_input_value("map-editor-sel-tint", &sel_tint_hex);
    }
}

/// Toggles interactive Battlemap Editor Mode (`#btn-map-editor-toggle` / `[E]` key).
#[wasm_bindgen]
pub fn satya_toggle_map_editor() {
    if !local_user_can_author_map() {
        return;
    }
    with_editor_mut(|ed| {
        ed.active = !ed.active;
        ed.clear_transient_drafts();
    });
    mark_map_wgpu_dirty();
    refresh_map_dom_and_wgpu();
}

/// Selects the active tool (`"select"`, `"prop"`, `"wall"`, `"surface"`, `"ground"`, `"erase"`).
#[wasm_bindgen]
pub fn satya_set_map_editor_tool(tool_code: String) {
    with_editor_mut(|ed| {
        ed.tool = EditorTool::parse_code(&tool_code);
        ed.clear_transient_drafts();
    });
    sync_map_editor_dom();
    sync_battlemap_svg_host();
    mark_map_wgpu_dirty();
    crate::wasm_client::map_3d::refresh_map_wgpu_if_active();
}

/// Undoes the last Editor operation on the active floor (`Ctrl+Z`).
#[wasm_bindgen]
pub fn satya_editor_undo() {
    let ok = with_map_ui_mut(|ui| {
        with_editor_mut(|ed| {
            let changed = ed.history.undo(&mut ui.map);
            if changed {
                ed.selection = EditorSelection::None;
                ed.clear_transient_drafts();
                ui.recompute_overlays();
            }
            changed
        })
    });
    if ok {
        mark_map_wgpu_dirty();
        refresh_map_dom_and_wgpu();
        sync_active_floor_to_server();
    }
}

/// Redoes the last undone Editor operation on the active floor (`Ctrl+Y`).
#[wasm_bindgen]
pub fn satya_editor_redo() {
    let ok = with_map_ui_mut(|ui| {
        with_editor_mut(|ed| {
            let changed = ed.history.redo(&mut ui.map);
            if changed {
                ed.selection = EditorSelection::None;
                ed.clear_transient_drafts();
                ui.recompute_overlays();
            }
            changed
        })
    });
    if ok {
        mark_map_wgpu_dirty();
        refresh_map_dom_and_wgpu();
        sync_active_floor_to_server();
    }
}

/// Reads all Editor brush & tool settings from DOM inputs into [`MapEditorState`].
#[wasm_bindgen]
pub fn satya_update_editor_brush_controls() {
    let snap = input_checked("map-editor-snap-chk");
    let prop_raw = select_value("map-editor-prop-select");
    let prop_rot_raw = input_value("map-editor-prop-rot");
    let prop_dec = input_checked("map-editor-prop-decorative");
    let prop_tint_raw = input_value("map-editor-prop-tint");
    let wall_k_raw = select_value("map-editor-wall-kind");
    let wall_m_raw = select_value("map-editor-wall-mat");
    let surf_k_raw = select_value("map-editor-surface-kind");
    let surf_s_raw = select_value("map-editor-surface-shape");
    let surf_w_raw = input_value("map-editor-surface-width");
    let surf_tint_raw = input_value("map-editor-surface-tint");

    with_editor_mut(|ed| {
        ed.snap_half_m = snap;
        if let Some(pk) = PropKind::from_code_str(&prop_raw) {
            ed.prop_kind = pk;
        }
        if let Ok(deg) = prop_rot_raw.trim().parse::<i32>() {
            ed.prop_rotation_deg = deg.rem_euclid(360);
        }
        ed.prop_decorative = prop_dec;
        if let Some(rgb) = parse_hex_rgb(&prop_tint_raw) {
            ed.prop_tint_rgb = Some(rgb);
        }
        ed.wall_mode = WallBrushMode::parse_code(&wall_k_raw);
        if let Some(mat) = MaterialClass::from_code_str(&wall_m_raw) {
            ed.wall_material = mat;
        }
        if let Some(sk) = SurfaceFeatureKind::from_code_str(&surf_k_raw) {
            ed.surface_kind = sk;
        }
        let next_shape = SurfaceBrushShape::parse_code(&surf_s_raw);
        if ed.surface_shape != next_shape {
            ed.surface_shape = next_shape;
            ed.surface_draft_pts_cm.clear();
            ed.surface_rect_start_cm = None;
            ed.surface_rect_cur_cm = None;
        }
        if let Ok(w_cm) = surf_w_raw.trim().parse::<i32>() {
            ed.surface_width_cm = w_cm.clamp(40, 1200);
        }
        if let Some(rgb) = parse_hex_rgb(&surf_tint_raw) {
            ed.surface_tint_rgb = Some(rgb);
        }
    });
    sync_map_editor_dom();
    sync_battlemap_svg_host();
}

/// Clears custom prop and surface brush tints back to untinted default material colors.
#[wasm_bindgen]
pub fn satya_clear_editor_brush_tint() {
    with_editor_mut(|ed| {
        ed.prop_tint_rgb = None;
        ed.surface_tint_rgb = None;
    });
    sync_map_editor_dom();
}

/// Commits the in-progress polygon or ribbon surface draft onto the active floor.
#[wasm_bindgen]
pub fn satya_finish_editor_surface() {
    let ok = with_map_ui_mut(|ui| with_editor_mut(|ed| ed.finish_draft_surface(&mut ui.map)));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Cancels any active wall drag or surface draft without mutating map geometry.
#[wasm_bindgen]
pub fn satya_cancel_editor_draft() {
    with_editor_mut(MapEditorState::clear_transient_drafts);
    sync_map_editor_dom();
    sync_battlemap_svg_host();
}

/// Applies `#map-editor-ground-override` and `#map-editor-ground-tint` to the active floor.
#[wasm_bindgen]
pub fn satya_apply_editor_ground_style() {
    let ov_raw = select_value("map-editor-ground-override");
    let tint_raw = input_value("map-editor-ground-tint");
    let tint = parse_hex_rgb(&tint_raw);
    let ok = with_map_ui_mut(|ui| {
        with_editor_mut(|ed| ed.set_ground_style(&mut ui.map, Some(ov_raw), tint))
    });
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Clears base ground override & tint back to the sector theme default.
#[wasm_bindgen]
pub fn satya_clear_editor_ground_style() {
    let ok =
        with_map_ui_mut(|ui| with_editor_mut(|ed| ed.set_ground_style(&mut ui.map, None, None)));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Rotates the currently selected object by `delta_deg` (`-15` or `+15`).
#[wasm_bindgen]
pub fn satya_editor_rotate_selected(delta_deg: i32) {
    let ok =
        with_map_ui_mut(|ui| with_editor_mut(|ed| ed.rotate_selected_prop(&mut ui.map, delta_deg)));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Sets the selected object's exact angle `0..359°` from `#map-editor-sel-rot`.
#[wasm_bindgen]
pub fn satya_editor_set_selected_rot() {
    let raw = input_value("map-editor-sel-rot");
    let Ok(deg) = raw.trim().parse::<i32>() else {
        return;
    };
    let ok =
        with_map_ui_mut(|ui| with_editor_mut(|ed| ed.set_selected_prop_rotation(&mut ui.map, deg)));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Stretches/shrinks the selected object's local width span by `delta_cm`.
#[wasm_bindgen]
pub fn satya_editor_stretch_selected(delta_cm: i32) {
    let ok =
        with_map_ui_mut(|ui| with_editor_mut(|ed| ed.stretch_selected_prop(&mut ui.map, delta_cm)));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Applies `#map-editor-sel-tint` onto the selected object or ground zone.
#[wasm_bindgen]
pub fn satya_editor_set_selected_tint() {
    let hex = input_value("map-editor-sel-tint");
    let Some(rgb) = parse_hex_rgb(&hex) else {
        return;
    };
    let ok =
        with_map_ui_mut(|ui| with_editor_mut(|ed| ed.set_selected_tint(&mut ui.map, Some(rgb))));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Removes any color tint from the currently selected object or ground zone.
#[wasm_bindgen]
pub fn satya_editor_clear_selected_tint() {
    let ok = with_map_ui_mut(|ui| with_editor_mut(|ed| ed.set_selected_tint(&mut ui.map, None)));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Toggles `Visual Only (decorative)` on the selected object from `#map-editor-sel-decorative`.
#[wasm_bindgen]
pub fn satya_editor_toggle_selected_decorative() {
    let dec = input_checked("map-editor-sel-decorative");
    let ok = with_map_ui_mut(|ui| {
        with_editor_mut(|ed| ed.set_selected_prop_decorative(&mut ui.map, dec))
    });
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Changes the [`SurfaceFeatureKind`] texture of the currently selected ground zone.
#[wasm_bindgen]
pub fn satya_editor_set_selected_surface_kind() {
    let raw = select_value("map-editor-sel-surface-kind");
    let Some(kind) = SurfaceFeatureKind::from_code_str(&raw) else {
        return;
    };
    let ok =
        with_map_ui_mut(|ui| with_editor_mut(|ed| ed.set_selected_surface_kind(&mut ui.map, kind)));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Duplicates the currently selected object offset `(+0.6m, +0.6m)`.
#[wasm_bindgen]
pub fn satya_editor_duplicate_selected() {
    let ok = with_map_ui_mut(|ui| with_editor_mut(|ed| ed.duplicate_selected(&mut ui.map)));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

/// Deletes the currently selected object or ground zone (`Delete` / `Backspace`).
#[wasm_bindgen]
pub fn satya_editor_delete_selected() {
    let ok = with_map_ui_mut(|ui| with_editor_mut(|ed| ed.delete_selected(&mut ui.map)));
    commit_outcome_and_refresh(EditorPointerOutcome {
        needs_redraw: ok,
        needs_server_sync: ok,
        needs_dom_sync: true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_maps::tactical_map::{SectorPreset, build_procedural_map_request};

    #[test]
    fn editor_select_drag_rotate_stretch_tint_decorative_duplicate_and_30_step_undo() {
        let mut map =
            build_procedural_map_request("ed", SectorPreset::OfficeFloor, None, 20, 14, 7);
        map.props.clear();
        map.wall_edges.clear();
        map.surfaces.clear();

        let mut ed = MapEditorState {
            active: true,
            tool: EditorTool::AddProp,
            prop_kind: PropKind::Desk,
            prop_rotation_deg: 0,
            ..Default::default()
        };
        let out = ed.on_pointer_down(&mut map, 5.0, 5.0);
        assert!(out.needs_server_sync);
        ed.prop_kind = PropKind::DeskClutter;
        ed.prop_decorative = true;
        ed.prop_tint_rgb = Some([200, 100, 50]);
        ed.on_pointer_down(&mut map, 5.0, 5.0);
        assert_eq!(map.props.len(), 2);
        assert_eq!(
            pick_prop_or_surface_at(&map, 5.0, 5.0),
            EditorSelection::Prop(1)
        );

        // 2. Drag move + 0.5m snap
        ed.tool = EditorTool::SelectMove;
        ed.snap_half_m = true;
        ed.on_pointer_down(&mut map, 5.0, 5.0);
        assert!(ed.on_pointer_move(&mut map, 7.62, 6.38));
        let up = ed.on_pointer_up(&mut map, 7.62, 6.38);
        assert!(up.needs_server_sync);
        assert_eq!(map.props[1].center_cm(), (750, 650));

        // 3. Rotate, stretch, tint clear, decorative toggle, duplicate, delete
        assert!(ed.rotate_selected_prop(&mut map, -15));
        assert_eq!(map.props[1].rotation_deg, 345);
        assert!(ed.stretch_selected_prop(&mut map, 50));
        assert_eq!(map.props[1].local_span_cm().0, 150);
        assert!(ed.set_selected_tint(&mut map, None));
        assert_eq!(map.props[1].tint_rgb, None);
        assert!(ed.set_selected_prop_decorative(&mut map, false));
        assert!(!map.props[1].decorative);
        assert!(ed.duplicate_selected(&mut map));
        assert_eq!(map.props.len(), 3);
        assert!(ed.delete_selected(&mut map));
        assert_eq!(map.props.len(), 2);

        // 4. Wall stroke & erase
        ed.tool = EditorTool::PaintWall;
        ed.wall_mode = WallBrushMode::Wall;
        ed.wall_material = MaterialClass::Brick;
        ed.on_pointer_down(&mut map, 2.0, 2.0);
        ed.on_pointer_move(&mut map, 5.0, 2.0);
        assert!(ed.on_pointer_up(&mut map, 5.0, 2.0).needs_server_sync);
        assert_eq!(map.wall_edges.len(), 3);
        assert!(
            map.wall_edges
                .iter()
                .all(|e| e.material == MaterialClass::Brick)
        );

        // 5. Rect, Polygon & Ribbon surface creation + Ground override
        ed.tool = EditorTool::PaintSurface;
        ed.surface_shape = SurfaceBrushShape::Rect;
        ed.surface_kind = SurfaceFeatureKind::TerrazzoFloor;
        ed.on_pointer_down(&mut map, 10.0, 4.0);
        ed.on_pointer_move(&mut map, 14.0, 8.0);
        assert!(ed.on_pointer_up(&mut map, 14.0, 8.0).needs_server_sync);
        assert_eq!(
            pick_prop_or_surface_at(&map, 12.0, 6.0),
            EditorSelection::Surface(0)
        );

        ed.surface_shape = SurfaceBrushShape::Ribbon;
        ed.surface_kind = SurfaceFeatureKind::BrickPaverWalk;
        ed.surface_width_cm = 160;
        ed.on_pointer_down(&mut map, 1.0, 10.0);
        ed.on_pointer_down(&mut map, 8.0, 10.0);
        assert!(ed.finish_draft_surface(&mut map));
        assert_eq!(map.surfaces.len(), 2);

        assert!(ed.set_ground_style(
            &mut map,
            Some("clinical_vinyl".to_string()),
            Some([220, 240, 255])
        ));
        assert_eq!(map.ground_floor_override.as_deref(), Some("clinical_vinyl"));

        // 6. 30-step Undo/Redo ring capacity & roundtrip
        for i in 0..35 {
            ed.set_ground_style(
                &mut map,
                Some("grass_lush".to_string()),
                Some([i as u8, 200, 100]),
            );
        }
        assert_eq!(ed.history.undo_stack.len(), MAX_EDITOR_UNDO);
        assert!(ed.history.undo(&mut map));
        assert_eq!(map.ground_tint_rgb, Some([33, 200, 100]));
        assert!(ed.history.redo(&mut map));
        assert_eq!(map.ground_tint_rgb, Some([34, 200, 100]));
    }
}
