//! # 3D Diorama Extra Props & Translucent Canopies (`map_3d_props_extra`)
//!
//! Companion to [`crate::wasm_client::map_3d_dressing_geom`] that builds detailed 3D meshes for
//! the 23 extended residential, outdoor, and clutter [`PropKind`]s (`DeskLamp` .. `StreetDebris`),
//! and emits translucent overhanging canopies (`LargeTree`, `MediumTree`, `Gazebo` roof) in a
//! separate late layer drawn **after** character miniatures so tokens beneath canopies stay visible.

use crate::wasm_client::glb_model::GlbVertex;
use crate::wasm_client::map_3d::{TacticalMapUiState, metric_to_world_xz};
use crate::wasm_client::map_3d_dressing_geom::{lighten, push_obb};
use satya_engine::map_dressing::prop_spec;
use satya_engine::tactical_map_geom::tree_canopy_radius_m;
use satya_schema::{MapPropPlacement, PropKind};

/// Translucent foliage/gazebo canopy zone sentinel in `SHARED_GLB_DIORAMA_WGSL`.
const TRANSLUCENT_CANOPY_ZONE: f32 = -0.35;

/// Appends detailed opaque 3D geometry for `p` centered at world `(cx, cz)` if `p.kind` is one of
/// the 23 extended residential/outdoor/clutter props. Returns `false` if handled in the core matcher.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_extra_prop_3d(
    verts: &mut Vec<GlbVertex>,
    p: &MapPropPlacement,
    cx: f32,
    cz: f32,
    hx: f32,
    hz: f32,
    top: f32,
    cutaway: bool,
    cell_m: f32,
) -> bool {
    let fp = p.footprint_m();
    let rot = fp.rot_deg;
    let rgb = crate::wasm_client::map_3d_props_lab_urban::apply_prop_tint(
        prop_spec(p.kind).albedo_rgb,
        p.tint_rgb,
    );
    let (sn, cs) = (fp.sin_rot, fp.cos_rot);
    let rot_pt =
        |ox: f32, oz: f32| -> (f32, f32) { (cx + ox * cs - oz * sn, cz + ox * sn + oz * cs) };

    match p.kind {
        PropKind::DeskLamp => {
            push_obb(
                verts,
                cx,
                cz,
                0.76,
                0.80,
                0.12,
                0.12,
                rot,
                [0.24, 0.24, 0.26],
                0.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                0.80,
                1.02,
                0.03,
                0.03,
                rot,
                [0.70, 0.58, 0.28],
                0.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                1.00,
                1.15_f32.min(top + 0.72),
                0.14,
                0.14,
                rot,
                rgb,
                0.0,
            );
        }
        PropKind::StandingLamp => {
            let h = if cutaway { top.min(0.88) } else { top };
            push_obb(
                verts,
                cx,
                cz,
                0.0,
                0.04,
                0.20,
                0.20,
                rot,
                [0.22, 0.22, 0.24],
                0.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                0.04,
                (h - 0.22).max(0.24),
                0.035,
                0.035,
                rot,
                [0.64, 0.54, 0.30],
                0.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                (h - 0.26).max(0.24),
                h,
                0.22,
                0.22,
                rot,
                [0.96, 0.90, 0.70],
                0.0,
            );
        }
        PropKind::Nightstand | PropKind::Dresser => {
            let cap = (top * 0.08).clamp(0.03, 0.06);
            push_obb(verts, cx, cz, 0.0, top - cap, hx, hz * 0.88, rot, rgb, 0.0);
            push_obb(
                verts,
                cx,
                cz,
                top - cap,
                top,
                hx * 1.02,
                hz * 0.90,
                rot,
                lighten(rgb, 1.18),
                0.0,
            );
            let (dx, dz) = rot_pt(0.0, hz * 0.86);
            push_obb(
                verts,
                dx,
                dz,
                top * 0.48,
                top * 0.62,
                hx * 0.45,
                0.03,
                rot,
                [0.78, 0.66, 0.34],
                0.0,
            );
        }
        PropKind::BathroomSink => {
            push_obb(
                verts,
                cx,
                cz,
                0.0,
                top * 0.86,
                hx * 0.92,
                hz * 0.78,
                rot,
                [0.46, 0.36, 0.28],
                0.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                top * 0.86,
                top,
                hx * 0.96,
                hz * 0.84,
                rot,
                rgb,
                0.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                top,
                top + 0.02,
                hx * 0.56,
                hz * 0.44,
                rot,
                [0.68, 0.86, 0.94],
                0.0,
            );
        }
        PropKind::FireExtinguisher => {
            push_obb(
                verts,
                cx,
                cz,
                0.10,
                top.min(0.75),
                0.11,
                0.11,
                rot,
                rgb,
                0.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                top.min(0.75),
                top.min(0.84),
                0.05,
                0.05,
                rot,
                [0.16, 0.16, 0.18],
                0.0,
            );
        }
        PropKind::TvStand => {
            let cons_h = (top * 0.48).min(0.56);
            push_obb(
                verts,
                cx,
                cz,
                0.0,
                cons_h,
                hx * 0.98,
                hz * 0.72,
                rot,
                rgb,
                0.0,
            );
            let (tx, tz) = rot_pt(0.0, -hz * 0.18);
            push_obb(
                verts,
                tx,
                tz,
                cons_h,
                top,
                hx * 0.86,
                0.05,
                rot,
                [0.09, 0.10, 0.13],
                0.0,
            );
        }
        PropKind::Stereo => {
            push_obb(
                verts,
                cx,
                cz,
                0.0,
                top * 0.82,
                hx * 0.44,
                hz * 0.72,
                rot,
                rgb,
                0.0,
            );
            for sx in [-hx * 0.72, hx * 0.72] {
                let (px, pz) = rot_pt(sx, 0.0);
                push_obb(
                    verts,
                    px,
                    pz,
                    0.0,
                    top,
                    hx * 0.22,
                    hz * 0.76,
                    rot,
                    [0.15, 0.15, 0.17],
                    0.0,
                );
            }
        }
        PropKind::RunnerRug => {
            push_obb(verts, cx, cz, 0.002, 0.012, hx, hz * 0.82, rot, rgb, 0.0);
        }
        PropKind::RoundRug => {
            push_obb(
                verts,
                cx,
                cz,
                0.002,
                0.012,
                hx * 0.86,
                hz * 0.86,
                rot,
                rgb,
                0.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                0.003,
                0.014,
                hx * 0.86,
                hz * 0.86,
                rot + 45,
                rgb,
                0.0,
            );
        }
        PropKind::SmallFountain | PropKind::MediumFountain | PropKind::LargeFountain => {
            let tiers = match p.kind {
                PropKind::SmallFountain => 1,
                PropKind::MediumFountain => 2,
                _ => 3,
            };
            let br = hx.min(hz) * 0.95;
            push_obb(verts, cx, cz, 0.0, 0.34, br, br, rot, rgb, -3.0);
            push_obb(verts, cx, cz, 0.0, 0.34, br, br, rot + 45, rgb, -3.0);
            push_obb(
                verts,
                cx,
                cz,
                0.02,
                0.36,
                br * 0.84,
                br * 0.84,
                rot,
                [0.16, 0.62, 0.78],
                -6.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                0.34,
                top * 0.56,
                br * 0.28,
                br * 0.28,
                rot,
                lighten(rgb, 1.08),
                -3.0,
            );
            if tiers >= 2 {
                push_obb(
                    verts,
                    cx,
                    cz,
                    top * 0.54,
                    top * 0.72,
                    br * 0.44,
                    br * 0.44,
                    rot,
                    rgb,
                    -3.0,
                );
            }
            if tiers >= 3 {
                push_obb(
                    verts,
                    cx,
                    cz,
                    top * 0.72,
                    top,
                    br * 0.22,
                    br * 0.22,
                    rot,
                    lighten(rgb, 1.15),
                    -3.0,
                );
            }
        }
        PropKind::Gazebo => {
            let gr = hx.min(hz) * 0.92;
            push_obb(verts, cx, cz, 0.0, 0.10, gr, gr, rot, rgb, -3.0);
            push_obb(verts, cx, cz, 0.0, 0.10, gr, gr, rot + 45, rgb, -3.0);
            let post_h = if cutaway { 0.88 } else { 2.28 };
            for i in 0..8 {
                let a = (i as f32 / 8.0) * std::f32::consts::TAU + std::f32::consts::TAU / 16.0;
                let (px, pz) = rot_pt(a.cos() * gr * 0.90, a.sin() * gr * 0.90);
                push_obb(
                    verts,
                    px,
                    pz,
                    0.10,
                    post_h,
                    0.08,
                    0.08,
                    rot,
                    [0.36, 0.24, 0.14],
                    0.0,
                );
            }
        }
        PropKind::BbqGrill => {
            push_obb(
                verts,
                cx,
                cz,
                0.0,
                top * 0.72,
                hx * 0.76,
                hz * 0.66,
                rot,
                [0.16, 0.17, 0.19],
                0.0,
            );
            push_obb(
                verts,
                cx,
                cz,
                top * 0.72,
                top,
                hx * 0.58,
                hz * 0.54,
                rot,
                [0.22, 0.23, 0.25],
                0.0,
            );
        }
        PropKind::GardenShed => {
            let sh = if cutaway { top.min(0.86) } else { 2.10 };
            push_obb(verts, cx, cz, 0.0, sh, hx * 0.94, hz * 0.94, rot, rgb, 0.0);
            if !cutaway {
                push_obb(
                    verts,
                    cx,
                    cz,
                    sh,
                    sh + 0.25,
                    hx * 0.98,
                    hz * 0.98,
                    rot,
                    [0.32, 0.24, 0.18],
                    0.0,
                );
                push_obb(
                    verts,
                    cx,
                    cz,
                    sh + 0.25,
                    sh + 0.42,
                    hx * 0.55,
                    hz * 0.96,
                    rot,
                    [0.28, 0.21, 0.16],
                    0.0,
                );
            }
        }
        PropKind::TrashCan => {
            push_obb(verts, cx, cz, 0.0, top, 0.28, 0.28, rot, rgb, 0.0);
            push_obb(
                verts,
                cx,
                cz,
                0.0,
                top,
                0.28,
                0.28,
                rot + 45,
                lighten(rgb, 1.08),
                0.0,
            );
        }
        PropKind::GarbagePile => {
            for (ox, oz, w, h) in [
                (-hx * 0.42, 0.0, 0.28, top),
                (0.05, 0.08, 0.30, top * 0.85),
                (hx * 0.44, -0.06, 0.25, top * 0.65),
            ] {
                let (bx, bz) = rot_pt(ox, oz);
                push_obb(verts, bx, bz, 0.0, h, w, 0.26, rot, rgb, 0.0);
            }
        }
        PropKind::AcUnit => {
            push_obb(verts, cx, cz, 0.0, top, hx * 0.82, hz * 0.76, rot, rgb, 0.0);
            push_obb(
                verts,
                cx,
                cz,
                top,
                top + 0.03,
                0.24,
                0.24,
                rot,
                [0.18, 0.20, 0.23],
                0.0,
            );
        }
        PropKind::HotTub => {
            push_obb(verts, cx, cz, 0.0, top, hx * 0.95, hz * 0.95, rot, rgb, 0.0);
            push_obb(
                verts,
                cx,
                cz,
                top * 0.82,
                top + 0.02,
                hx * 0.76,
                hz * 0.76,
                rot,
                [0.16, 0.68, 0.84],
                -6.0,
            );
        }
        PropKind::FloorClothes | PropKind::FloorToys => {
            push_obb(
                verts,
                cx,
                cz,
                0.004,
                top.max(0.06),
                0.24 * cell_m,
                0.18 * cell_m,
                rot + 18,
                rgb,
                0.0,
            );
            let (ox, oz) = rot_pt(0.16, -0.10);
            push_obb(
                verts,
                ox,
                oz,
                0.004,
                top.max(0.05),
                0.14 * cell_m,
                0.14 * cell_m,
                rot + 42,
                lighten(rgb, 1.2),
                0.0,
            );
        }
        PropKind::StreetDebris => {
            push_obb(
                verts,
                cx,
                cz,
                0.003,
                0.05,
                hx * 0.68,
                hz * 0.36,
                rot + 12,
                rgb,
                -4.0,
            );
            let (dx, dz) = rot_pt(hx * 0.42, 0.10);
            push_obb(
                verts,
                dx,
                dz,
                0.003,
                0.07,
                0.18,
                0.14,
                rot + 55,
                [0.58, 0.56, 0.52],
                -4.0,
            );
        }
        _ => return false,
    }
    true
}

/// Emits enlarged, translucent overhead canopies (`LargeTree`, `MediumTree`, and `Gazebo` roof)
/// after opaque geometry & miniatures so tokens underneath remain clearly visible.
pub fn push_translucent_canopies_3d(verts: &mut Vec<GlbVertex>, ui: &TacticalMapUiState) {
    if ui.wall_cutaway {
        return;
    }
    let (bw_m, bh_m) = ui.map.board_size_m();
    for p in &ui.map.props {
        let fp = p.footprint_m();
        let [cx, cz] = metric_to_world_xz(fp.cx_m, fp.cy_m, bw_m, bh_m);
        match p.kind {
            PropKind::LargeTree => {
                let cr = tree_canopy_radius_m(p.kind);
                push_obb(
                    verts,
                    cx,
                    cz,
                    2.05,
                    3.85,
                    cr,
                    cr,
                    0,
                    [0.22, 0.40, 0.18],
                    TRANSLUCENT_CANOPY_ZONE,
                );
                push_obb(
                    verts,
                    cx,
                    cz,
                    2.20,
                    4.00,
                    cr * 0.94,
                    cr * 0.94,
                    45,
                    [0.26, 0.45, 0.21],
                    TRANSLUCENT_CANOPY_ZONE,
                );
                push_obb(
                    verts,
                    cx,
                    cz,
                    3.45,
                    4.65,
                    cr * 0.66,
                    cr * 0.66,
                    22,
                    [0.30, 0.52, 0.24],
                    TRANSLUCENT_CANOPY_ZONE,
                );
                push_obb(
                    verts,
                    cx + cr * 0.28,
                    cz - cr * 0.22,
                    2.45,
                    4.15,
                    cr * 0.54,
                    cr * 0.54,
                    15,
                    [0.28, 0.48, 0.22],
                    TRANSLUCENT_CANOPY_ZONE,
                );
            }
            PropKind::MediumTree => {
                let cr = tree_canopy_radius_m(p.kind);
                push_obb(
                    verts,
                    cx,
                    cz,
                    1.80,
                    3.35,
                    cr,
                    cr,
                    0,
                    [0.22, 0.43, 0.20],
                    TRANSLUCENT_CANOPY_ZONE,
                );
                push_obb(
                    verts,
                    cx,
                    cz,
                    1.95,
                    3.50,
                    cr * 0.92,
                    cr * 0.92,
                    45,
                    [0.27, 0.50, 0.24],
                    TRANSLUCENT_CANOPY_ZONE,
                );
                push_obb(
                    verts,
                    cx - cr * 0.14,
                    cz - cr * 0.14,
                    3.05,
                    3.95,
                    cr * 0.60,
                    cr * 0.60,
                    22,
                    [0.33, 0.56, 0.27],
                    TRANSLUCENT_CANOPY_ZONE,
                );
            }
            PropKind::Gazebo => {
                let gr = fp.hx_m.min(fp.hy_m) * 1.02;
                let rot = fp.rot_deg;
                push_obb(
                    verts,
                    cx,
                    cz,
                    2.28,
                    2.58,
                    gr,
                    gr,
                    rot,
                    [0.46, 0.34, 0.24],
                    TRANSLUCENT_CANOPY_ZONE,
                );
                push_obb(
                    verts,
                    cx,
                    cz,
                    2.28,
                    2.58,
                    gr,
                    gr,
                    rot + 45,
                    [0.46, 0.34, 0.24],
                    TRANSLUCENT_CANOPY_ZONE,
                );
                push_obb(
                    verts,
                    cx,
                    cz,
                    2.58,
                    2.88,
                    gr * 0.62,
                    gr * 0.62,
                    rot,
                    [0.52, 0.39, 0.28],
                    TRANSLUCENT_CANOPY_ZONE,
                );
                push_obb(
                    verts,
                    cx,
                    cz,
                    2.88,
                    3.10,
                    gr * 0.28,
                    gr * 0.28,
                    rot,
                    [0.58, 0.44, 0.30],
                    TRANSLUCENT_CANOPY_ZONE,
                );
            }
            PropKind::AwningCanopy => {
                crate::wasm_client::map_3d_props_lab_urban::push_awning_canopy_3d(
                    verts, p, cx, cz,
                );
            }
            _ => {}
        }
    }
}
