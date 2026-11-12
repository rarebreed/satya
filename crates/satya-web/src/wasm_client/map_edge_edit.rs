//! # Edge-Wall Interaction: Door Toggling & GM Wall Painting (`map_edge_edit`)
//!
//! Turns a continuous pointer position (in cell units, from the 2D battlemap overlay or the 3D canvas
//! ray-plane pick) into either an **edge** action or an ordinary cell click:
//! - `move_or_select`: a click within `0.2` cells of a door edge opens or closes it. Anyone may do this,
//!   synced via `POST /api/sessions/:id/map` with `edit_edge`.
//! - `edit_terrain`: a click near any edge cycles it `none → Wall → Closed Door → Open Door → Window → none`
//!   (GM only on the server).
//! - Any other click falls through to [`crate::wasm_client::map_3d::handle_map_grid_cell_clicked`].

use crate::wasm_client::dom::fetch_json;
use crate::wasm_client::map_3d::{
    apply_incoming_map_update, handle_map_grid_cell_clicked, mark_map_wgpu_dirty,
    refresh_map_dom_and_wgpu, with_map_ui_mut,
};
use crate::wasm_client::state::with_state;
use satya_schema::{
    EdgeSide, GenerateSessionMapRequest, GmMapUpdateRequest, MapEdgeRef, MapWallEdge,
    MaterialClass, WallEdgeKind,
};
use wasm_bindgen::prelude::*;

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

/// Handles a pointer click at continuous cell-space `(fx, fy)` (edge action or cell click fallback).
#[wasm_bindgen]
pub fn satya_click_map_point(fx: f32, fy: f32) {
    let action = with_map_ui_mut(|ui| {
        let r = nearest_edge_at(fx, fy, ui.map.grid_width, ui.map.grid_height, EDGE_PICK_TOLERANCE_CELLS)?;
        let cur = edge_kind_at(&ui.map, r);
        let next = match ui.interaction_mode.as_str() {
            "edit_terrain" => next_painted_edge(cur),
            "move_or_select" => match cur {
                Some(WallEdgeKind::DoorClosed) => Some(WallEdgeKind::DoorOpen),
                Some(WallEdgeKind::DoorOpen) => Some(WallEdgeKind::DoorClosed),
                _ => return None,
            },
            _ => return None,
        };
        apply_edge_kind(&mut ui.map, r, next);
        ui.recompute_overlays();
        Some((r, next, ui.map.theme_preset.clone().unwrap_or_default()))
    });
    match action {
        Some((r, next, preset)) => {
            mark_map_wgpu_dirty();
            refresh_map_dom_and_wgpu();
            post_edge_edit_if_in_session(preset, r, next);
        }
        None => handle_map_grid_cell_clicked(fx.floor() as i32, fy.floor() as i32),
    }
}

fn post_edge_edit_if_in_session(preset: String, r: MapEdgeRef, next: Option<WallEdgeKind>) {
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
}
