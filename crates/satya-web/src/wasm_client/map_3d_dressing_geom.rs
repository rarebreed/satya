//! # 3D Diorama Geometry for Edge Walls & Multi-Cell Props (`map_3d_dressing_geom`)
//!
//! Companion to [`crate::wasm_client::map_3d_wgpu::build_tactical_diorama_vertices`]. It emits:
//! - **Thin edge walls** (`0.14m` thick) colored by [`MaterialClass`], with closed door slabs, open door
//!   leaves swung into the cell, and windows as waist-high sills (plus lintels in full-height mode).
//! - **Multi-cell props**: one body box per footprint, sized from the
//!   `satya_engine::map_dressing::PROP_CATALOG` height and albedo, with a lighter top cap. Trees get a
//!   canopy and cars a cabin. Tall props clamp to cutaway height when wall cutaway is on.

use crate::wasm_client::glb_model::GlbVertex;
use crate::wasm_client::map_3d::TacticalMapUiState;
use crate::wasm_client::map_3d_wgpu::push_box;
use satya_engine::map_dressing::prop_spec;
use satya_schema::{EdgeSide, MaterialClass, PropKind, WallEdgeKind};

const WALL_HALF_THICK_M: f32 = 0.07;

const fn material_rgb(m: MaterialClass) -> [f32; 3] {
    match m {
        MaterialClass::Brick => [0.55, 0.30, 0.24],
        MaterialClass::ConcreteBlock | MaterialClass::ReinforcedConcrete => [0.52, 0.53, 0.54],
        MaterialClass::SolidWood | MaterialClass::WoodFurniture => [0.50, 0.34, 0.20],
        MaterialClass::SheetSteel | MaterialClass::ArmorSteel | MaterialClass::Appliance => [0.45, 0.50, 0.55],
        _ => [0.80, 0.78, 0.72],
    }
}

fn lighten(c: [f32; 3], k: f32) -> [f32; 3] {
    [(c[0] * k).min(1.0), (c[1] * k).min(1.0), (c[2] * k).min(1.0)]
}

/// Appends edge-wall and prop geometry for `ui` (`wall_h_m` / `door_h_m` honor the cutaway toggle).
pub fn push_dressing_geometry(verts: &mut Vec<GlbVertex>, ui: &TacticalMapUiState, wall_h_m: f32, door_h_m: f32) {
    let (w, h) = (ui.map.grid_width.max(1), ui.map.grid_height.max(1));
    let s = ui.map.cell_size_m();
    // Grid lattice point `(gx, gy)` -> world `(wx, wz)`.
    let lattice = |gx: f32, gy: f32| ((gx - w as f32 * 0.5) * s, (gy - h as f32 * 0.5) * s);

    for e in &ui.map.wall_edges {
        let (ax, az) = lattice(e.x as f32, e.y as f32);
        // Edge runs +X for North edges and +Z for West edges.
        let (dir_x, dir_z) = match e.side {
            EdgeSide::North => (1.0, 0.0),
            EdgeSide::West => (0.0, 1.0),
        };
        let (mx, mz) = (ax + dir_x * s * 0.5, az + dir_z * s * 0.5);
        let half_len = s * 0.5 + WALL_HALF_THICK_M;
        let (hx, hz) = (dir_x * half_len + dir_z * WALL_HALF_THICK_M, dir_z * half_len + dir_x * WALL_HALF_THICK_M);
        let wall_rgb = material_rgb(e.material);
        match e.kind {
            WallEdgeKind::Wall => {
                push_box(verts, mx, mz, 0.0, wall_h_m, hx, hz, wall_rgb, 0.0);
                push_box(verts, mx, mz, wall_h_m, wall_h_m + 0.03, hx, hz, lighten(wall_rgb, 1.25), 0.0);
            }
            WallEdgeKind::DoorClosed => {
                let (dhx, dhz) = (dir_x * s * 0.46 + dir_z * 0.03, dir_z * s * 0.46 + dir_x * 0.03);
                push_box(verts, mx, mz, 0.0, door_h_m, dhx, dhz, [0.62, 0.40, 0.20], 0.0);
            }
            WallEdgeKind::DoorOpen => {
                // Leaf hinged at the edge start, swung 90° into the owning cell (+Z for North, +X for West).
                let leaf = s * 0.46;
                let (lx, lz) = (ax + dir_z * leaf + dir_x * 0.04, az + dir_x * leaf + dir_z * 0.04);
                let (lhx, lhz) = (dir_z * leaf + dir_x * 0.03, dir_x * leaf + dir_z * 0.03);
                push_box(verts, lx, lz, 0.0, door_h_m.min(0.9), lhx, lhz, [0.62, 0.40, 0.20], 0.0);
            }
            WallEdgeKind::Window => {
                push_box(verts, mx, mz, 0.0, 0.9_f32.min(wall_h_m), hx, hz, wall_rgb, 0.0);
                push_box(verts, mx, mz, 0.9_f32.min(wall_h_m), 0.94_f32.min(wall_h_m + 0.04), hx, hz, [0.70, 0.86, 0.95], 0.0);
                if wall_h_m > 2.0 {
                    push_box(verts, mx, mz, 2.05, wall_h_m, hx, hz, wall_rgb, 0.0);
                }
            }
        }
    }

    let cutaway = wall_h_m < 1.0;
    for p in &ui.map.props {
        let spec = prop_spec(p.kind);
        let (x0, z0) = lattice(p.x as f32, p.y as f32);
        let (x1, z1) = lattice((p.x + p.w_cells) as f32, (p.y + p.h_cells) as f32);
        let (cx, cz) = ((x0 + x1) * 0.5, (z0 + z1) * 0.5);
        let inset = 0.06 * s;
        let (hx, hz) = (((x1 - x0) * 0.5 - inset).max(0.05), ((z1 - z0) * 0.5 - inset).max(0.05));
        let mut top = spec.height_cm as f32 / 100.0;
        if cutaway {
            top = top.min(0.95);
        }
        let rgb = spec.albedo_rgb;
        match p.kind {
            PropKind::Rug => push_box(verts, cx, cz, 0.002, 0.012, hx, hz, rgb, 0.0),
            PropKind::StonePath | PropKind::AsphaltDrive => {
                let full_hx = (x1 - x0) * 0.5;
                let full_hz = (z1 - z0) * 0.5;
                push_box(verts, cx, cz, 0.002, 0.014, full_hx, full_hz, rgb, -1.0);
            }
            PropKind::LargeTree => {
                push_box(verts, cx, cz, 0.0, top.min(2.2), hx * 0.45, hz * 0.45, rgb, 0.0);
                if !cutaway {
                    push_box(verts, cx, cz, 2.0, 4.6, hx * 1.9, hz * 1.9, [0.24, 0.42, 0.20], 0.0);
                }
            }
            PropKind::MediumTree => {
                push_box(verts, cx, cz, 0.0, top.min(1.9), hx * 0.40, hz * 0.40, rgb, 0.0);
                if !cutaway {
                    push_box(verts, cx, cz, 1.75, 3.85, hx * 2.1, hz * 2.1, [0.22, 0.45, 0.22], 0.0);
                }
            }
            PropKind::Car => {
                push_box(verts, cx, cz, 0.15, 0.85, hx, hz, rgb, 0.0);
                push_box(verts, cx, cz, 0.85, top, hx * 0.86, hz * 0.55, [0.20, 0.24, 0.30], 0.0);
            }
            PropKind::UtilityVan => {
                push_box(verts, cx, cz, 0.16, 0.92, hx, hz, rgb, 0.0);
                push_box(verts, cx, cz, 0.92, top, hx * 0.90, hz * 0.78, lighten(rgb, 0.92), 0.0);
            }
            PropKind::BarStool | PropKind::Planter | PropKind::Bush => {
                push_box(verts, cx, cz, 0.0, top, hx * 0.62, hz * 0.62, rgb, 0.0);
            }
            _ => {
                let cap = (top * 0.08).clamp(0.02, 0.06);
                push_box(verts, cx, cz, 0.0, top - cap, hx, hz, rgb, 0.0);
                push_box(verts, cx, cz, top - cap, top, hx * 0.98, hz * 0.98, lighten(rgb, 1.22), 0.0);
            }
        }
    }
}
