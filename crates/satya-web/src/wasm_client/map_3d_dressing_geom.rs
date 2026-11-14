//! # 3D Diorama Geometry for Edge Walls & Multi-Cell Props (`map_3d_dressing_geom`)
//!
//! Companion to [`crate::wasm_client::map_3d_wgpu::build_tactical_diorama_vertices`]. It emits:
//! - **Thin edge walls** (`0.14m` thick) colored by [`MaterialClass`], with closed door slabs, open door
//!   leaves swung into the cell, and windows as waist-high sills (plus lintels in full-height mode).
//! - **Multi-cell props**: one body box per footprint, sized from the
//!   `satya_engine::map_dressing::PROP_CATALOG` height and albedo, with a lighter top cap. Trees get a
//!   canopy and cars a cabin. Tall props clamp to cutaway height when wall cutaway is on.

use crate::wasm_client::glb_model::GlbVertex;
use crate::wasm_client::map_3d::{TacticalMapUiState, metric_to_world_xz};
use crate::wasm_client::map_3d_wgpu::{push_box, push_quad};
use satya_engine::map_dressing::prop_spec;
use satya_schema::{EdgeSide, MaterialClass, PropKind, WallEdgeKind};

const WALL_HALF_THICK_M: f32 = 0.07;

#[allow(clippy::too_many_arguments)]
fn push_obb(
    verts: &mut Vec<GlbVertex>,
    cx: f32,
    cz: f32,
    y0: f32,
    y1: f32,
    hl: f32,
    hw: f32,
    rot_deg: i32,
    rgb: [f32; 3],
    zone_id: f32,
) {
    let rad = (rot_deg as f32).to_radians();
    let (ux, uz) = (rad.cos(), rad.sin());
    let (vx, vz) = (-uz, ux);
    let col = [rgb[0], rgb[1], rgb[2], 0.0];
    let pt = |lx: f32, y: f32, lz: f32| [cx + lx * ux + lz * vx, y, cz + lx * uz + lz * vz];
    let b0 = pt(-hl, y0, -hw);
    let b1 = pt(hl, y0, -hw);
    let b2 = pt(hl, y0, hw);
    let b3 = pt(-hl, y0, hw);
    let t0 = pt(-hl, y1, -hw);
    let t1 = pt(hl, y1, -hw);
    let t2 = pt(hl, y1, hw);
    let t3 = pt(-hl, y1, hw);
    push_quad(verts, [b3, b2, t2, t3], col, zone_id);
    push_quad(verts, [b1, b0, t0, t1], col, zone_id);
    push_quad(verts, [b2, b1, t1, t2], col, zone_id);
    push_quad(verts, [b0, b3, t3, t0], col, zone_id);
    push_quad(verts, [t3, t2, t1, t0], col, zone_id);
}

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
    crate::wasm_client::map_3d_surfaces_geom::push_vector_surfaces_3d(verts, ui);
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
    let (bw_m, bh_m) = ui.map.board_size_m();
    let has_vector_surfaces = !ui.map.surfaces.is_empty();
    for p in &ui.map.props {
        if has_vector_surfaces
            && matches!(
                p.kind,
                PropKind::AsphaltDrive
                    | PropKind::StonePath
                    | PropKind::GravelPath
                    | PropKind::DirtPath
                    | PropKind::MulchBed
                    | PropKind::CreekWater
                    | PropKind::DrainageDitch
                    | PropKind::GardenBed
                    | PropKind::GardenEdging
            )
        {
            continue;
        }
        let spec = prop_spec(p.kind);
        let fp = p.footprint_m();
        let [cx, cz] = metric_to_world_xz(fp.cx_m, fp.cy_m, bw_m, bh_m);
        let (sw_m, sh_m) = (fp.span_w_m, fp.span_h_m);
        let (full_hx, full_hz) = (fp.hx_m, fp.hy_m);
        let inset = 0.06 * s;
        let (hx, hz) = ((full_hx - inset).max(0.05), (full_hz - inset).max(0.05));
        let rot = fp.rot_deg;
        let (sn, cs) = (fp.sin_rot, fp.cos_rot);
        let rot_pt = |ox: f32, oz: f32| -> (f32, f32) { (cx + ox * cs - oz * sn, cz + ox * sn + oz * cs) };
        let mut top = spec.height_cm as f32 / 100.0;
        if cutaway {
            top = top.min(0.95);
        }
        let rgb = spec.albedo_rgb;
        match p.kind {
            PropKind::Rug => push_obb(verts, cx, cz, 0.002, 0.012, hx, hz, rot, rgb, 0.0),
            PropKind::ElevatedCatwalk => {
                // Raised industrial mezzanine stays at 1.48m even in cutaway view so platform height stays clear.
                push_obb(verts, cx, cz, 0.0, 1.48, full_hx * 0.98, full_hz * 0.98, rot, [0.15, 0.21, 0.31], -1.0);
            }
            PropKind::HazardSpill => {
                push_obb(verts, cx, cz, 0.004, 0.028, hx, hz, rot, rgb, 0.0);
            }
            PropKind::StructuralPillar => {
                push_obb(verts, cx, cz, 0.0, wall_h_m, hx, hz, rot, rgb, 0.0);
                push_obb(verts, cx, cz, wall_h_m, wall_h_m + 0.03, hx * 0.96, hz * 0.96, rot, lighten(rgb, 1.25), 0.0);
            }
            PropKind::StonePath | PropKind::AsphaltDrive => {
                push_obb(verts, cx, cz, 0.002, 0.014, full_hx, full_hz, rot, rgb, -1.0);
            }
            PropKind::GravelPath => {
                push_obb(verts, cx, cz, 0.003, 0.018, full_hx, full_hz, rot, rgb, -3.0);
            }
            PropKind::DirtPath => {
                push_obb(verts, cx, cz, 0.002, 0.014, full_hx, full_hz, rot, rgb, -4.0);
            }
            PropKind::MulchBed => {
                push_obb(verts, cx, cz, 0.003, 0.020, full_hx, full_hz, rot, rgb, -5.0);
            }
            PropKind::CreekWater => {
                push_obb(verts, cx, cz, 0.002, 0.015, full_hx, full_hz, rot, [0.46, 0.43, 0.36], 0.0);
                push_obb(verts, cx, cz, 0.004, 0.026, full_hx * 0.96, full_hz * 0.96, rot, rgb, -6.0);
            }
            PropKind::DrainageDitch => {
                let (b1x, b1z) = rot_pt(0.0, -full_hz + 0.08 * s);
                let (b2x, b2z) = rot_pt(0.0, full_hz - 0.08 * s);
                push_obb(verts, b1x, b1z, 0.0, 0.11, full_hx, 0.08 * s, rot, [0.34, 0.27, 0.19], -4.0);
                push_obb(verts, b2x, b2z, 0.0, 0.11, full_hx, 0.08 * s, rot, [0.34, 0.27, 0.19], -4.0);
                push_obb(verts, cx, cz, 0.003, 0.022, full_hx, full_hz * 0.84, rot, rgb, -6.0);
            }
            PropKind::FallenLog => {
                let hl = (sw_m.max(sh_m) - 0.12).max(0.4) * 0.5;
                let r = 0.23 * s;
                let log_top = top.min(0.62);
                push_obb(verts, cx, cz, 0.02, log_top * 0.86, hl, r, rot, rgb, 0.0);
                push_obb(verts, cx, cz, log_top * 0.68, log_top, hl * 0.99, r * 0.68, rot, lighten(rgb, 1.15), 0.0);
                push_obb(verts, cx, cz, 0.06, log_top * 0.72, hl + 0.015, r * 0.66, rot, [0.76, 0.62, 0.44], 0.0);
                push_obb(verts, cx, cz, log_top - 0.01, log_top + 0.03, hl * 0.55, r * 0.45, rot, [0.34, 0.50, 0.24], -0.25);
            }
            PropKind::DeskL => {
                let (bx, bz) = rot_pt(0.0, -0.5);
                let (wx, wz) = rot_pt(-0.5, 0.5);
                push_obb(verts, bx, bz, 0.0, top - 0.06, (1.0 - inset).max(0.2), (0.5 - inset).max(0.2), rot, [0.32, 0.26, 0.22], 0.0);
                push_obb(verts, bx, bz, top - 0.06, top, (1.0 - inset).max(0.2), (0.5 - inset).max(0.2), rot, rgb, 0.0);
                push_obb(verts, wx, wz, 0.0, top - 0.06, (0.5 - inset).max(0.2), 0.5, rot, [0.32, 0.26, 0.22], 0.0);
                push_obb(verts, wx, wz, top - 0.06, top, (0.5 - inset).max(0.2), 0.5, rot, rgb, 0.0);
                let (chx, chz) = rot_pt(0.45, 0.45);
                push_obb(verts, chx, chz, 0.0, 0.42, 0.10 * s, 0.10 * s, rot, [0.18, 0.20, 0.23], 0.0);
                push_obb(verts, chx, chz, 0.42, 0.48, 0.24 * s, 0.24 * s, rot, [0.22, 0.24, 0.28], 0.0);
                push_obb(verts, chx, chz, 0.48, (top + 0.14).min(0.92), 0.22 * s, 0.08 * s, rot, [0.18, 0.20, 0.24], 0.0);
            }
            PropKind::DeskT => {
                let (bx, bz) = rot_pt(0.0, -0.5);
                let (px, pz) = rot_pt(0.0, 0.5);
                push_obb(verts, bx, bz, 0.0, top - 0.06, (1.5 - inset).max(0.3), (0.5 - inset).max(0.2), rot, [0.32, 0.26, 0.22], 0.0);
                push_obb(verts, bx, bz, top - 0.06, top, (1.5 - inset).max(0.3), (0.5 - inset).max(0.2), rot, rgb, 0.0);
                push_obb(verts, px, pz, 0.0, top - 0.06, (0.5 - inset).max(0.2), 0.5, rot, [0.32, 0.26, 0.22], 0.0);
                push_obb(verts, px, pz, top - 0.06, top, (0.5 - inset).max(0.2), 0.5, rot, rgb, 0.0);
                for ox in [-0.95_f32, 0.95] {
                    let (chx, chz) = rot_pt(ox, 0.45);
                    push_obb(verts, chx, chz, 0.0, 0.42, 0.10 * s, 0.10 * s, rot, [0.18, 0.20, 0.23], 0.0);
                    push_obb(verts, chx, chz, 0.42, 0.48, 0.24 * s, 0.24 * s, rot, [0.22, 0.24, 0.28], 0.0);
                }
            }
            PropKind::GardenBed => {
                push_obb(verts, cx, cz, 0.0, top, hx, hz, rot, [0.44, 0.29, 0.18], 0.0);
                push_obb(verts, cx, cz, 0.0, top + 0.02, hx * 0.86, hz * 0.86, rot, [0.30, 0.21, 0.14], -5.0);
                for (ox, oz, bloom) in [
                    (-hx * 0.45, -hz * 0.25, [0.88, 0.34, 0.44]),
                    (0.0, hz * 0.20, [0.95, 0.76, 0.28]),
                    (hx * 0.45, -hz * 0.15, [0.64, 0.46, 0.88]),
                ] {
                    let (bx, bz) = rot_pt(ox, oz);
                    push_obb(verts, bx, bz, top + 0.02, top + 0.18, hx * 0.22, hz * 0.32, rot, [0.24, 0.48, 0.21], -0.25);
                    push_obb(verts, bx, bz, top + 0.16, top + 0.24, 0.08 * s, 0.08 * s, rot, bloom, 0.0);
                }
            }
            PropKind::GardenEdging => {
                let steps = (sw_m.max(sh_m) * 2.0).round().max(2.0) as i32;
                let span = sw_m;
                for i in 0..steps {
                    let t = (i as f32 + 0.5) / steps as f32 - 0.5;
                    let (sx, sz) = rot_pt(t * span, 0.0);
                    let bh = if i % 2 == 0 { top } else { top * 0.88 };
                    push_obb(verts, sx, sz, 0.0, bh, (span / steps as f32) * 0.44, 0.18 * s, rot, rgb, 0.0);
                }
            }
            PropKind::LargeTree => {
                push_obb(verts, cx, cz, 0.0, 0.45, hx * 0.58, hz * 0.58, rot, rgb, 0.0);
                push_obb(verts, cx, cz, 0.0, top.min(2.3), hx * 0.42, hz * 0.42, rot, rgb, 0.0);
                if !cutaway {
                    let cr = hx.max(hz) * 1.85;
                    push_obb(verts, cx, cz, 2.0, 3.8, cr, cr, 0, [0.22, 0.40, 0.18], -0.25);
                    push_obb(verts, cx, cz, 2.15, 3.95, cr * 0.96, cr * 0.96, 45, [0.26, 0.45, 0.21], -0.25);
                    push_obb(verts, cx, cz, 3.4, 4.65, cr * 0.68, cr * 0.68, 22, [0.30, 0.52, 0.24], -0.25);
                    push_obb(verts, cx + cr * 0.28, cz - cr * 0.22, 2.4, 4.15, cr * 0.56, cr * 0.56, 15, [0.28, 0.48, 0.22], -0.25);
                    push_obb(verts, cx - cr * 0.26, cz + cr * 0.24, 2.3, 4.0, cr * 0.54, cr * 0.54, 60, [0.20, 0.38, 0.17], -0.25);
                }
            }
            PropKind::MediumTree => {
                push_obb(verts, cx, cz, 0.0, 0.38, hx * 0.52, hz * 0.52, rot, rgb, 0.0);
                push_obb(verts, cx, cz, 0.0, top.min(1.95), hx * 0.38, hz * 0.38, rot, rgb, 0.0);
                if !cutaway {
                    let cr = hx.max(hz) * 2.05;
                    push_obb(verts, cx, cz, 1.75, 3.35, cr, cr, 0, [0.22, 0.43, 0.20], -0.25);
                    push_obb(verts, cx, cz, 1.9, 3.45, cr * 0.94, cr * 0.94, 45, [0.27, 0.50, 0.24], -0.25);
                    push_obb(verts, cx - cr * 0.15, cz - cr * 0.15, 3.05, 3.95, cr * 0.64, cr * 0.64, 22, [0.33, 0.56, 0.27], -0.25);
                }
            }
            PropKind::Car => {
                push_obb(verts, cx, cz, 0.15, 0.85, hx, hz, rot, rgb, 0.0);
                push_obb(verts, cx, cz, 0.85, top, hx * 0.86, hz * 0.55, rot, [0.20, 0.24, 0.30], 0.0);
            }
            PropKind::UtilityVan => {
                push_obb(verts, cx, cz, 0.16, 0.92, hx, hz, rot, rgb, 0.0);
                push_obb(verts, cx, cz, 0.92, top, hx * 0.90, hz * 0.78, rot, lighten(rgb, 0.92), 0.0);
            }
            PropKind::Hedge => {
                push_obb(verts, cx, cz, 0.0, top, hx, hz, rot, rgb, -0.25);
            }
            PropKind::Bush => {
                push_obb(verts, cx, cz, 0.0, top * 0.92, hx * 0.66, hz * 0.66, rot, rgb, -0.25);
                push_obb(verts, cx, cz, 0.05, top, hx * 0.62, hz * 0.62, rot + 45, lighten(rgb, 1.12), -0.25);
            }
            PropKind::BarStool | PropKind::Planter => {
                push_obb(verts, cx, cz, 0.0, top, hx * 0.62, hz * 0.62, rot, rgb, 0.0);
            }
            _ => {
                let cap = (top * 0.08).clamp(0.02, 0.06);
                push_obb(verts, cx, cz, 0.0, top - cap, hx, hz, rot, rgb, 0.0);
                push_obb(verts, cx, cz, top - cap, top, hx * 0.98, hz * 0.98, rot, lighten(rgb, 1.22), 0.0);
            }
        }
    }
}
