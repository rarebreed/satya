//! # Edge-Wall Interaction: Door Toggling & GM Wall Painting (`map_edge_edit`)
//!
//! Turns a continuous pointer position (in cell units, from the 2D battlemap overlay or the 3D canvas
//! ray-plane pick) into either an **edge** action or an ordinary cell click:
//! - `move_or_select`: a click within `0.2` cells of a door edge opens or closes it. Anyone may do this,
//!   synced via `POST /api/sessions/:id/map` with `edit_edge`.
//! - `edit_terrain`: a click near any edge cycles it `none → Wall → Closed Door → Open Door → Window → none`
//!   (GM only on the server).
//! - Any other click falls through to [`crate::wasm_client::map_3d::handle_map_grid_cell_clicked`].

use crate::wasm_client::dom::{fetch_json, input_value, select_value};
use crate::wasm_client::map_3d::{
    apply_incoming_map_update, handle_map_point_clicked, mark_map_wgpu_dirty,
    refresh_map_dom_and_wgpu, with_map_ui_mut,
};
use crate::wasm_client::state::with_state;
use satya_engine::map_dressing::{make_prop_at_cm, normalize_prop_rotation_deg};
use satya_engine::tactical_map_geom::{place_prop_on_map, remove_prop_at_point};
use satya_schema::{
    EdgeSide, GenerateSessionMapRequest, GmMapUpdateRequest, MapEdgeRef, MapPropPlacement,
    MapWallEdge, MaterialClass, PropKind, WallEdgeKind,
};
use wasm_bindgen::prelude::*;

/// Grouped [`PropKind`] catalog displayed in `#map-prop-brush-select` when `edit_terrain` mode is active.
pub const PROP_BRUSH_GROUPS: &[(&str, &[PropKind])] = &[
    (
        "Living & Office",
        &[
            PropKind::Sofa,
            PropKind::Loveseat,
            PropKind::Armchair,
            PropKind::CoffeeTable,
            PropKind::Rug,
            PropKind::DiningTable,
            PropKind::RoundTable,
            PropKind::Booth,
            PropKind::Bed,
            PropKind::Desk,
            PropKind::DeskL,
            PropKind::DeskT,
            PropKind::Bookshelf,
            PropKind::Wardrobe,
            PropKind::Bathtub,
            PropKind::Toilet,
            PropKind::Planter,
        ],
    ),
    (
        "Kitchen & Bar",
        &[
            PropKind::KitchenCounter,
            PropKind::Stove,
            PropKind::Fridge,
            PropKind::BarCounter,
            PropKind::BackBarShelf,
            PropKind::BarStool,
            PropKind::PoolTable,
            PropKind::Jukebox,
        ],
    ),
    (
        "Industrial & Tactical",
        &[
            PropKind::WoodCrate,
            PropKind::DrumCluster,
            PropKind::SteelShelving,
            PropKind::Dumpster,
            PropKind::JerseyBarrier,
            PropKind::StructuralPillar,
            PropKind::ElevatedCatwalk,
        ],
    ),
    (
        "Outdoors & Vehicles",
        &[
            PropKind::LargeTree,
            PropKind::MediumTree,
            PropKind::Hedge,
            PropKind::Bush,
            PropKind::Boulder,
            PropKind::FallenLog,
            PropKind::ParkBench,
            PropKind::PicnicTable,
            PropKind::Car,
            PropKind::UtilityVan,
        ],
    ),
    (
        "Landscaping & Terrain",
        &[
            PropKind::GardenBed,
            PropKind::GardenEdging,
            PropKind::StonePath,
            PropKind::GravelPath,
            PropKind::DirtPath,
            PropKind::MulchBed,
            PropKind::CreekWater,
            PropKind::DrainageDitch,
            PropKind::AsphaltDrive,
            PropKind::HazardSpill,
        ],
    ),
];

/// Pointer tolerance (in cells) for grabbing an edge instead of the cell interior.
pub const EDGE_PICK_TOLERANCE_CELLS: f32 = 0.2;

/// Nearest in-bounds edge to continuous cell-space point `(fx, fy)` if within `tol` cells.
#[must_use]
pub fn nearest_edge_at(fx: f32, fy: f32, w: i32, h: i32, tol: f32) -> Option<MapEdgeRef> {
    let (gx, gy) = (fx.floor() as i32, fy.floor() as i32);
    let (ux, uy) = (fx - gx as f32, fy - gy as f32);
    let cands = [
        (ux, MapEdgeRef { x: gx, y: gy, side: EdgeSide::West }),
        (1.0 - ux, MapEdgeRef { x: gx + 1, y: gy, side: EdgeSide::West }),
        (uy, MapEdgeRef { x: gx, y: gy, side: EdgeSide::North }),
        (1.0 - uy, MapEdgeRef { x: gx, y: gy + 1, side: EdgeSide::North }),
    ];
    let (dist, r) = cands.into_iter().min_by(|a, b| a.0.total_cmp(&b.0))?;
    let in_bounds = match r.side {
        EdgeSide::North => (0..w).contains(&r.x) && (0..=h).contains(&r.y),
        EdgeSide::West => (0..=w).contains(&r.x) && (0..h).contains(&r.y),
    };
    (dist <= tol && in_bounds).then_some(r)
}

/// Current edge kind at `r` on `map`.
#[must_use]
pub fn edge_kind_at(map: &GmMapUpdateRequest, r: MapEdgeRef) -> Option<WallEdgeKind> {
    map.wall_edges
        .iter()
        .find(|e| e.x == r.x && e.y == r.y && e.side == r.side)
        .map(|e| e.kind)
}

/// Sets (`Some`) or removes (`None`) the edge at `r`, preserving an existing material.
pub fn apply_edge_kind(map: &mut GmMapUpdateRequest, r: MapEdgeRef, kind: Option<WallEdgeKind>) {
    let pos = map.wall_edges.iter().position(|e| e.x == r.x && e.y == r.y && e.side == r.side);
    match (pos, kind) {
        (Some(i), Some(k)) => map.wall_edges[i].kind = k,
        (Some(i), None) => {
            map.wall_edges.remove(i);
        }
        (None, Some(k)) => map.wall_edges.push(MapWallEdge {
            x: r.x,
            y: r.y,
            side: r.side,
            kind: k,
            material: if k == WallEdgeKind::Window { MaterialClass::Glass } else { MaterialClass::Drywall },
        }),
        (None, None) => {}
    }
}

/// GM paint cycle: `none → Wall → DoorClosed → DoorOpen → Window → none`.
#[must_use]
pub const fn next_painted_edge(cur: Option<WallEdgeKind>) -> Option<WallEdgeKind> {
    match cur {
        None => Some(WallEdgeKind::Wall),
        Some(WallEdgeKind::Wall) => Some(WallEdgeKind::DoorClosed),
        Some(WallEdgeKind::DoorClosed) => Some(WallEdgeKind::DoorOpen),
        Some(WallEdgeKind::DoorOpen) => Some(WallEdgeKind::Window),
        Some(WallEdgeKind::Window) => None,
    }
}

/// Computes the next clockwise brush rotation (`+15°` increments across full `0..=359°`).
#[must_use]
pub fn next_brush_rotation_deg(_brush: &str, current_deg: i32) -> i32 {
    (current_deg.rem_euclid(360) + 15) % 360
}

/// Updates `#map-prop-brush-select` in `edit_terrain` mode (`"__wall_edge__"`/`""` edge paint, `"__erase__"`, or `PropKind`).
#[wasm_bindgen]
pub fn satya_update_map_prop_brush() {
    let raw = select_value("map-prop-brush-select");
    with_map_ui_mut(|ui| {
        if let Some(kind) = PropKind::from_code_str(&raw) {
            ui.edit_prop_rotation_deg = normalize_prop_rotation_deg(kind, ui.edit_prop_rotation_deg);
        }
        ui.edit_prop_brush = raw;
    });
    refresh_map_dom_and_wgpu();
}

/// Rotates the active prop brush placement clockwise by `+15°` (full `0..359°` orientation).
#[wasm_bindgen]
pub fn satya_rotate_map_prop_brush() {
    with_map_ui_mut(|ui| {
        ui.edit_prop_rotation_deg =
            next_brush_rotation_deg(&ui.edit_prop_brush, ui.edit_prop_rotation_deg);
    });
    refresh_map_dom_and_wgpu();
}

/// Applies an exact numeric `0..=359°` rotation from `#map-prop-rot-deg`.
#[wasm_bindgen]
pub fn satya_set_map_prop_rotation_deg() {
    let raw = input_value("map-prop-rot-deg");
    let parsed = raw.trim().parse::<i32>().unwrap_or(0).rem_euclid(360);
    with_map_ui_mut(|ui| {
        ui.edit_prop_rotation_deg = parsed;
    });
    refresh_map_dom_and_wgpu();
}

/// Toggles optional `0.50m` snap vs. free continuous prop placement (`#btn-map-snap-toggle`).
#[wasm_bindgen]
pub fn satya_toggle_map_grid_snap() {
    with_map_ui_mut(|ui| {
        ui.snap_to_half_meter = !ui.snap_to_half_meter;
    });
    refresh_map_dom_and_wgpu();
}

/// Applies the active prop brush (`__erase__` or `PropKind`) at continuous point `(fx_m, fy_m)`.
pub fn handle_map_prop_brush_point(fx_m: f32, fy_m: f32) {
    struct PendingPropEdit {
        preset: String,
        edit_prop: Option<MapPropPlacement>,
        clear_prop_at_cm: Option<(i32, i32)>,
    }
    let pending = with_map_ui_mut(|ui| {
        let (bw_m, bh_m) = ui.map.board_size_m();
        if !fx_m.is_finite() || !fy_m.is_finite() || fx_m < 0.0 || fy_m < 0.0 || fx_m >= bw_m || fy_m >= bh_m {
            return None;
        }
        ui.ensure_vector_scene_populated();
        let preset = ui
            .map
            .theme_preset
            .clone()
            .unwrap_or_else(|| "orbital_airlock".to_string());
        if ui.edit_prop_brush == "__erase__" {
            remove_prop_at_point(&mut ui.map.props, fx_m, fy_m)?;
            ui.recompute_overlays();
            let cx_cm = (fx_m * 100.0).round() as i32;
            let cy_cm = (fy_m * 100.0).round() as i32;
            return Some(PendingPropEdit {
                preset,
                edit_prop: None,
                clear_prop_at_cm: Some((cx_cm, cy_cm)),
            });
        }
        let kind = PropKind::from_code_str(&ui.edit_prop_brush)?;
        let (px_m, py_m) = if ui.snap_to_half_meter {
            ((fx_m * 2.0).round() * 0.5, (fy_m * 2.0).round() * 0.5)
        } else {
            (fx_m, fy_m)
        };
        let cx_cm = (px_m * 100.0).round() as i32;
        let cy_cm = (py_m * 100.0).round() as i32;
        let occupied: Vec<(f32, f32)> = ui.map.tokens.iter().map(|t| ui.map.token_pos_m(t)).collect();
        let seed = (ui.map.seed.unwrap_or(42) as u32)
            .wrapping_mul(31)
            .wrapping_add((cx_cm as u32).wrapping_mul(17) ^ (cy_cm as u32).wrapping_mul(53));
        let prop = make_prop_at_cm(kind, cx_cm, cy_cm, ui.edit_prop_rotation_deg, None, seed);
        let placed = place_prop_on_map(
            &mut ui.map.props,
            &ui.map.wall_edges,
            &occupied,
            (bw_m, bh_m),
            prop,
        )
        .ok()?;
        ui.recompute_overlays();
        Some(PendingPropEdit {
            preset,
            edit_prop: Some(placed),
            clear_prop_at_cm: None,
        })
    });
    if let Some(p) = pending {
        mark_map_wgpu_dirty();
        refresh_map_dom_and_wgpu();
        post_prop_edit_if_in_session(p.preset, p.edit_prop, p.clear_prop_at_cm);
    }
}

/// Applies the active prop brush at cell center `(gx, gy)`.
pub fn handle_map_prop_brush_click(gx: i32, gy: i32) {
    handle_map_prop_brush_point(gx as f32 + 0.5, gy as f32 + 0.5);
}

/// Handles a pointer click at continuous metric `(fx_m, fy_m)` (prop brush, edge action, or free point click).
#[wasm_bindgen]
pub fn satya_click_map_point(fx_m: f32, fy_m: f32) {
    let (is_edit, brush) = crate::wasm_client::map_3d::with_map_ui(|ui| {
        (ui.interaction_mode == "edit_terrain", ui.edit_prop_brush.clone())
    });
    if is_edit && !brush.is_empty() && brush != "__wall_edge__" {
        handle_map_prop_brush_point(fx_m, fy_m);
        return;
    }
    if !is_edit {
        handle_map_point_clicked(fx_m, fy_m);
        return;
    }
    let action = with_map_ui_mut(|ui| {
        let cell_m = ui.map.cell_size_m().max(0.25);
        let r = nearest_edge_at(
            fx_m / cell_m,
            fy_m / cell_m,
            ui.map.grid_width,
            ui.map.grid_height,
            EDGE_PICK_TOLERANCE_CELLS,
        )?;
        let cur = edge_kind_at(&ui.map, r);
        let next = next_painted_edge(cur);
        apply_edge_kind(&mut ui.map, r, next);
        ui.recompute_overlays();
        Some((r, next, ui.map.theme_preset.clone().unwrap_or_default()))
    });
    if let Some((r, next, preset)) = action {
        mark_map_wgpu_dirty();
        refresh_map_dom_and_wgpu();
        post_edge_edit_if_in_session(preset, r, next);
    }
}

fn post_prop_edit_if_in_session(
    preset: String,
    edit_prop: Option<MapPropPlacement>,
    clear_prop_at_cm: Option<(i32, i32)>,
) {
    let (auth_token, session_id) = with_state(|st| (st.auth_token.clone(), st.session_id.clone()));
    let (Some(token), Some(sid)) = (auth_token, session_id) else {
        return;
    };
    let req = GenerateSessionMapRequest {
        preset,
        populate_roster_tokens: false,
        edit_prop,
        clear_prop_at_cm,
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

pub(crate) fn post_edge_edit_if_in_session(preset: String, r: MapEdgeRef, next: Option<WallEdgeKind>) {
    let (auth_token, session_id) = with_state(|st| (st.auth_token.clone(), st.session_id.clone()));
    let (Some(token), Some(sid)) = (auth_token, session_id) else {
        return;
    };
    let req = GenerateSessionMapRequest {
        preset,
        populate_roster_tokens: false,
        edit_edge: Some(r),
        set_edge_kind: next,
        clear_edge: next.is_none(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_picking_and_paint_cycle() {
        let r = nearest_edge_at(3.05, 2.5, 10, 8, 0.2).unwrap();
        assert_eq!((r.x, r.y, r.side), (3, 2, EdgeSide::West));
        let r = nearest_edge_at(3.5, 2.93, 10, 8, 0.2).unwrap();
        assert_eq!((r.x, r.y, r.side), (3, 3, EdgeSide::North));
        assert!(nearest_edge_at(3.5, 2.5, 10, 8, 0.2).is_none(), "cell interior");
        assert!(nearest_edge_at(9.95, 2.5, 10, 8, 0.2).is_some(), "east border edge x = w");

        let mut map = GmMapUpdateRequest::default();
        let r = MapEdgeRef { x: 2, y: 2, side: EdgeSide::North };
        let mut cur = None;
        for expect in [Some(WallEdgeKind::Wall), Some(WallEdgeKind::DoorClosed), Some(WallEdgeKind::DoorOpen), Some(WallEdgeKind::Window), None] {
            cur = next_painted_edge(cur);
            apply_edge_kind(&mut map, r, cur);
            assert_eq!(edge_kind_at(&map, r), expect);
        }
        assert!(map.wall_edges.is_empty());
    }

    #[test]
    fn prop_brush_groups_cover_entire_prop_catalog_uniquely() {
        let mut seen = std::collections::BTreeSet::new();
        for &(_, kinds) in PROP_BRUSH_GROUPS {
            for &k in kinds {
                assert!(seen.insert(k.as_str()), "duplicate brush entry {}", k.as_str());
            }
        }
        assert_eq!(
            seen.len(),
            PropKind::ALL.len(),
            "PROP_BRUSH_GROUPS must include all PropKind variants"
        );
    }

    #[test]
    fn free_360_brush_rotation_and_continuous_prop_placement_erase() {
        assert_eq!(next_brush_rotation_deg("FALLEN_LOG", 0), 15);
        assert_eq!(next_brush_rotation_deg("SOFA", 33), 48);
        assert_eq!(next_brush_rotation_deg("DESK_L", 345), 0);
        assert_eq!(next_brush_rotation_deg("DESK_T", 180), 195);

        let mut props = Vec::new();
        let placed = place_prop_on_map(
            &mut props,
            &[],
            &[],
            (20.0, 14.0),
            make_prop_at_cm(PropKind::Sofa, 640, 520, 33, None, 7),
        )
        .expect("33-degree continuous placement succeeds");
        assert_eq!(placed.rotation_deg, 33);
        assert_eq!(props.len(), 1);
        let removed = remove_prop_at_point(&mut props, 6.40, 5.20);
        assert!(removed.is_some());
        assert!(props.is_empty());
    }
}
