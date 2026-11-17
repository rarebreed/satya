//! # Medical, Laboratory, Bath, Elevator & Streetscape 3D Prop Meshes (`map_3d_props_lab_urban`)
//!
//! Companion to [`crate::wasm_client::map_3d_dressing_geom`] and [`crate::wasm_client::map_3d_props_extra`]
//! building multi-part 3D geometry for the 28 medical, lab/fabrication, vertical access,
//! bathroom/laundry, office, and urban streetscape [`PropKind`] variants, plus overhanging
//! striped [`PropKind::AwningCanopy`] translucent canopies.

use crate::wasm_client::glb_model::GlbVertex;
use crate::wasm_client::map_3d_dressing_geom::{lighten, push_obb};
use satya_engine::map_dressing::prop_spec;
use satya_engine::tactical_map_geom::prop_contains_point;
use satya_schema::{MapPropPlacement, PropKind};

/// Translucent canopy zone sentinel in `SHARED_GLB_DIORAMA_WGSL`.
const TRANSLUCENT_CANOPY_ZONE: f32 = -0.35;

/// Blends an optional `#RRGGBB` prop tint into a base linear RGB albedo.
#[must_use]
pub(crate) fn apply_prop_tint(rgb: [f32; 3], tint: Option<[u8; 3]>) -> [f32; 3] {
    if let Some([tr, tg, tb]) = tint {
        let t = [tr as f32 / 255.0, tg as f32 / 255.0, tb as f32 / 255.0];
        [
            rgb[0] * 0.55 + t[0] * 0.45,
            rgb[1] * 0.55 + t[1] * 0.45,
            rgb[2] * 0.55 + t[2] * 0.45,
        ]
    } else {
        rgb
    }
}

fn is_work_surface_prop(kind: PropKind) -> bool {
    matches!(
        kind,
        PropKind::Desk
            | PropKind::DeskL
            | PropKind::DeskT
            | PropKind::LabBench
            | PropKind::DiningTable
            | PropKind::RoundTable
            | PropKind::CoffeeTable
            | PropKind::BarCounter
            | PropKind::NurseStation
            | PropKind::MedicalCounter
            | PropKind::KitchenCounter
            | PropKind::CheckoutCounter
            | PropKind::SolderingStation
            | PropKind::TestBench
            | PropKind::PlanterTable
    )
}

fn clutter_base_y_m(p: &MapPropPlacement, all_props: &[MapPropPlacement]) -> f32 {
    let (cx_m, cy_m) = p.center_m();
    if all_props
        .iter()
        .any(|other| is_work_surface_prop(other.kind) && prop_contains_point(other, cx_m, cy_m, false))
    {
        0.76
    } else {
        0.0
    }
}

/// Appends detailed opaque 3D geometry for medical, lab, elevator, bath, office, and urban props.
/// Returns `false` if `p.kind` belongs to another renderer module.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_lab_urban_prop_3d(
    verts: &mut Vec<GlbVertex>,
    p: &MapPropPlacement,
    all_props: &[MapPropPlacement],
    cx: f32,
    cz: f32,
    hx: f32,
    hz: f32,
    top: f32,
    cutaway: bool,
) -> bool {
    let fp = p.footprint_m();
    let rot = fp.rot_deg;
    let rgb = apply_prop_tint(prop_spec(p.kind).albedo_rgb, p.tint_rgb);
    let (sn, cs) = (fp.sin_rot, fp.cos_rot);
    let rot_pt =
        |ox: f32, oz: f32| -> (f32, f32) { (cx + ox * cs - oz * sn, cz + ox * sn + oz * cs) };

    match p.kind {
        PropKind::Cot => {
            for lx in [-hx * 0.80, hx * 0.80] {
                for lz in [-hz * 0.72, hz * 0.72] {
                    let (px, pz) = rot_pt(lx, lz);
                    push_obb(verts, px, pz, 0.0, 0.30, 0.03, 0.03, rot, [0.34, 0.38, 0.42], 0.0);
                }
            }
            push_obb(verts, cx, cz, 0.28, 0.38, hx * 0.96, hz * 0.92, rot, rgb, 0.0);
            let (px, pz) = rot_pt(-hx * 0.62, 0.0);
            push_obb(verts, px, pz, 0.38, 0.46, hx * 0.20, hz * 0.65, rot, [0.88, 0.90, 0.92], 0.0);
        }
        PropKind::Gurney => {
            for lx in [-hx * 0.75, hx * 0.75] {
                for lz in [-hz * 0.68, hz * 0.68] {
                    let (px, pz) = rot_pt(lx, lz);
                    push_obb(verts, px, pz, 0.0, 0.20, 0.04, 0.04, rot, [0.22, 0.25, 0.28], 0.0);
                }
            }
            push_obb(verts, cx, cz, 0.20, 0.56, hx * 0.54, hz * 0.52, rot, [0.58, 0.64, 0.70], 0.0);
            push_obb(verts, cx, cz, 0.56, 0.74, hx * 0.96, hz * 0.90, rot, rgb, 0.0);
            let (px, pz) = rot_pt(-hx * 0.64, 0.0);
            push_obb(verts, px, pz, 0.74, 0.84, hx * 0.18, hz * 0.68, rot, [0.96, 0.98, 1.0], 0.0);
            for sz in [-hz * 0.88, hz * 0.88] {
                let (rx, rz) = rot_pt(0.0, sz);
                push_obb(verts, rx, rz, 0.66, 0.88, hx * 0.74, 0.025, rot, [0.12, 0.56, 0.84], 0.0);
            }
        }
        PropKind::ReclinerChair => {
            push_obb(verts, cx, cz, 0.0, 0.44, hx * 0.88, hz * 0.86, rot, rgb, 0.0);
            let (bx, bz) = rot_pt(0.0, -hz * 0.54);
            push_obb(verts, bx, bz, 0.44, top, hx * 0.88, (hz * 0.28).max(0.10), rot, lighten(rgb, 0.88), 0.0);
            for sx in [-hx * 0.78, hx * 0.78] {
                let (ax, az) = rot_pt(sx, 0.05);
                push_obb(verts, ax, az, 0.44, 0.62, 0.10, hz * 0.72, rot, lighten(rgb, 0.82), 0.0);
            }
        }
        PropKind::MedicalCounter => {
            push_obb(verts, cx, cz, 0.0, 0.78, hx, hz, rot, [0.76, 0.80, 0.85], 0.0);
            push_obb(verts, cx, cz, 0.78, top.min(0.88), hx * 1.02, hz * 1.02, rot, rgb, 0.0);
            let (sx, sz) = rot_pt(hx * 0.45, -hz * 0.15);
            push_obb(verts, sx, sz, 0.86, 0.90, 0.22, 0.18, rot, [0.58, 0.64, 0.70], 0.0);
            push_obb(verts, sx, sz, 0.88, (top + 0.16).min(1.06), 0.03, 0.03, rot, [0.78, 0.84, 0.88], 0.0);
        }
        PropKind::NurseStation => {
            push_obb(verts, cx, cz, 0.0, 0.76, hx, hz, rot, rgb, 0.0);
            let (rx, rz) = rot_pt(0.0, -hz * 0.72);
            push_obb(verts, rx, rz, 0.76, top.min(1.12), hx * 0.96, 0.12, rot, lighten(rgb, 1.10), 0.0);
            for mx in [-hx * 0.48, 0.0, hx * 0.48] {
                let (px, pz) = rot_pt(mx, -hz * 0.26);
                push_obb(verts, px, pz, 0.76, (top + 0.18).min(1.14), 0.18, 0.03, rot, [0.16, 0.20, 0.26], 0.0);
                push_obb(verts, px, pz, 0.82, (top + 0.14).min(1.10), 0.15, 0.035, rot, [0.22, 0.75, 0.96], 0.0);
            }
        }
        PropKind::TelemetryMonitor => {
            push_obb(verts, cx, cz, 0.0, 0.08, hx * 0.64, hz * 0.64, rot, [0.24, 0.28, 0.34], 0.0);
            push_obb(verts, cx, cz, 0.08, (top * 0.72).max(0.65), 0.04, 0.04, rot, [0.68, 0.72, 0.78], 0.0);
            let (sx, sz) = rot_pt(-0.08, -0.04);
            push_obb(
                verts,
                sx,
                sz,
                (top * 0.62).max(0.55),
                top.min(1.38),
                0.18,
                0.08,
                rot,
                [0.12, 0.16, 0.22],
                0.0,
            );
            push_obb(
                verts,
                sx,
                sz,
                (top * 0.66).max(0.60),
                (top - 0.04).max(0.66),
                0.14,
                0.09,
                rot,
                [0.16, 0.78, 0.36],
                0.0,
            );
        }
        PropKind::BiohazardBin => {
            let (bx, bz) = rot_pt(-hx * 0.46, 0.0);
            push_obb(verts, bx, bz, 0.0, top * 0.88, hx * 0.42, hz * 0.78, rot, rgb, 0.0);
            push_obb(verts, bx, bz, top * 0.88, top, hx * 0.44, hz * 0.80, rot, [0.95, 0.84, 0.18], 0.0);
            let (lx, lz) = rot_pt(hx * 0.46, 0.0);
            push_obb(verts, lx, lz, 0.0, top * 0.92, hx * 0.42, hz * 0.78, rot, [0.18, 0.42, 0.78], 0.0);
        }
        PropKind::LinenCart => {
            push_obb(verts, cx, cz, 0.08, 0.16, hx, hz, rot, [0.60, 0.66, 0.72], 0.0);
            push_obb(verts, cx, cz, 0.16, top * 0.62, hx * 0.90, hz * 0.86, rot, rgb, 0.0);
            push_obb(verts, cx, cz, top * 0.62, top, hx * 0.86, hz * 0.82, rot, [0.68, 0.84, 0.95], 0.0);
        }
        PropKind::ElevatorCar | PropKind::ServiceElevator => {
            let wall_top = if cutaway { top.min(0.95) } else { 2.35 };
            for sx in [-hx * 0.92, hx * 0.92] {
                let (wx, wz) = rot_pt(sx, 0.0);
                push_obb(verts, wx, wz, 0.0, wall_top, 0.07, hz, rot, [0.42, 0.46, 0.52], 0.0);
            }
            let (bx, bz) = rot_pt(0.0, -hz * 0.92);
            push_obb(verts, bx, bz, 0.0, wall_top, hx, 0.07, rot, [0.40, 0.44, 0.50], 0.0);
            push_obb(verts, cx, cz, 0.0, 0.04, hx * 0.94, hz * 0.94, rot, rgb, -1.0);
            let (dx, dz) = rot_pt(0.0, hz * 0.88);
            push_obb(verts, dx, dz, 0.04, wall_top * 0.92, hx * 0.72, 0.03, rot, [0.68, 0.73, 0.79], 0.0);
            let indicator_col = if p.kind == PropKind::ServiceElevator {
                [0.95, 0.76, 0.16]
            } else {
                [0.24, 0.80, 0.96]
            };
            push_obb(verts, dx, dz, wall_top * 0.78, wall_top * 0.90, 0.22, 0.05, rot, indicator_col, 0.0);
        }
        PropKind::LabBench => {
            for lx in [-hx * 0.82, hx * 0.82] {
                for lz in [-hz * 0.74, hz * 0.74] {
                    let (px, pz) = rot_pt(lx, lz);
                    push_obb(verts, px, pz, 0.0, 0.76, 0.04, 0.04, rot, [0.22, 0.26, 0.30], 0.0);
                }
            }
            push_obb(verts, cx, cz, 0.76, top.min(0.86), hx, hz, rot, rgb, 0.0);
            let (rx, rz) = rot_pt(0.0, -hz * 0.55);
            push_obb(verts, rx, rz, 0.84, (top + 0.28).min(1.14), hx * 0.82, 0.12, rot, [0.30, 0.36, 0.44], 0.0);
            let (vx, vz) = rot_pt(hx * 0.42, 0.06);
            push_obb(verts, vx, vz, 0.86, 1.02, 0.06, 0.06, rot, [0.24, 0.78, 0.95], 0.0);
        }
        PropKind::DeskClutter => {
            let y0 = clutter_base_y_m(p, all_props);
            let (p1x, p1z) = rot_pt(-hx * 0.36, -hz * 0.18);
            push_obb(verts, p1x, p1z, y0, y0 + 0.03, 0.12, 0.16, rot - 12, [0.94, 0.94, 0.92], 0.0);
            let (p2x, p2z) = rot_pt(0.02, hz * 0.20);
            push_obb(verts, p2x, p2z, y0, y0 + 0.04, 0.16, 0.09, rot + 6, [0.24, 0.28, 0.34], 0.0);
            let (mx, mz) = rot_pt(hx * 0.46, hz * 0.18);
            push_obb(verts, mx, mz, y0, y0 + 0.09, 0.045, 0.045, rot, rgb, 0.0);
        }
        PropKind::SolderingStation => {
            push_obb(verts, cx, cz, 0.0, 0.74, hx * 0.92, hz * 0.86, rot, [0.20, 0.24, 0.28], 0.0);
            push_obb(verts, cx, cz, 0.74, 0.78, hx, hz, rot, [0.16, 0.52, 0.30], 0.0);
            let (ux, uz) = rot_pt(-hx * 0.52, -hz * 0.35);
            push_obb(verts, ux, uz, 0.78, top.min(0.98), 0.16, 0.14, rot, rgb, 0.0);
            let (lx, lz) = rot_pt(-hx * 0.52, -hz * 0.20);
            push_obb(verts, lx, lz, 0.84, 0.93, 0.08, 0.02, rot, [0.24, 0.82, 0.98], 0.0);
        }
        PropKind::TestBench => {
            push_obb(verts, cx, cz, 0.0, 0.76, hx, hz, rot, rgb, 0.0);
            let (ox, oz) = rot_pt(-hx * 0.45, -hz * 0.22);
            push_obb(verts, ox, oz, 0.76, (top + 0.20).min(1.12), 0.28, 0.20, rot, [0.14, 0.18, 0.24], 0.0);
            push_obb(verts, ox, oz, 0.80, (top + 0.14).min(1.06), 0.20, 0.21, rot, [0.18, 0.86, 0.42], 0.0);
            let (wx, wz) = rot_pt(hx * 0.40, -hz * 0.24);
            push_obb(verts, wx, wz, 0.76, (top + 0.14).min(1.04), 0.24, 0.18, rot, [0.22, 0.28, 0.36], 0.0);
        }
        PropKind::Printer3dStation => {
            push_obb(verts, cx, cz, 0.0, 0.74, hx, hz, rot, [0.28, 0.32, 0.38], 0.0);
            push_obb(verts, cx, cz, 0.74, top.min(1.26), hx * 0.66, hz * 0.66, rot, rgb, 0.0);
            push_obb(verts, cx, cz, 0.76, 0.84, hx * 0.48, hz * 0.48, rot, [0.16, 0.68, 0.92], 0.0);
            let (sx, sz) = rot_pt(hx * 0.56, 0.0);
            push_obb(verts, sx, sz, 0.94, top.min(1.18), 0.08, 0.16, rot, [0.52, 0.28, 0.82], 0.0);
        }
        PropKind::AirCompressor => {
            push_obb(verts, cx, cz, 0.0, 0.14, hx * 0.75, hz * 0.70, rot, [0.18, 0.20, 0.24], 0.0);
            push_obb(verts, cx, cz, 0.14, top * 0.68, hx * 0.90, hz * 0.64, rot, rgb, 0.0);
            push_obb(verts, cx, cz, top * 0.68, top, hx * 0.42, hz * 0.46, rot, [0.28, 0.32, 0.36], 0.0);
        }
        PropKind::CncMill => {
            push_obb(verts, cx, cz, 0.0, top, hx * 0.88, hz * 0.92, rot, rgb, 0.0);
            push_obb(verts, cx, cz, 0.56, (top * 0.82).max(0.64), hx * 0.66, hz * 0.95, rot, [0.14, 0.20, 0.28], 0.0);
            let (px, pz) = rot_pt(hx * 0.84, hz * 0.58);
            push_obb(verts, px, pz, 0.72, top.min(1.24), 0.12, 0.14, rot, [0.14, 0.18, 0.24], 0.0);
            push_obb(verts, px, pz, 0.84, top.min(1.12), 0.09, 0.15, rot, [0.22, 0.78, 0.96], 0.0);
        }
        PropKind::ServerRack => {
            push_obb(verts, cx, cz, 0.0, top, hx, hz, rot, rgb, 0.0);
            push_obb(verts, cx, cz, 0.08, top * 0.92, hx * 0.86, hz * 1.03, rot, [0.14, 0.19, 0.26], 0.0);
            let (lx, lz) = rot_pt(0.0, hz * 1.04);
            push_obb(verts, lx, lz, top * 0.72, top * 0.82, hx * 0.55, 0.02, rot, [0.20, 0.86, 0.98], 0.0);
        }
        PropKind::FumeHood => {
            push_obb(verts, cx, cz, 0.0, 0.78, hx, hz, rot, [0.68, 0.72, 0.78], 0.0);
            push_obb(verts, cx, cz, 0.78, top, hx, hz * 0.94, rot, rgb, 0.0);
            let (gx, gz) = rot_pt(0.0, hz * 0.94);
            push_obb(verts, gx, gz, 0.82, (top * 0.88).max(0.86), hx * 0.84, 0.03, rot, [0.58, 0.84, 0.96], 0.0);
            push_obb(verts, cx, cz, top, (top + 0.18).min(if cutaway { 0.95 } else { 2.30 }), 0.18, 0.18, rot, [0.54, 0.60, 0.66], 0.0);
        }
        PropKind::ShowerStall => {
            push_obb(verts, cx, cz, 0.0, 0.08, hx, hz, rot, rgb, 0.0);
            push_obb(verts, cx, cz, 0.02, 0.09, hx * 0.86, hz * 0.86, rot, [0.62, 0.82, 0.92], -6.0);
            let wall_h = if cutaway { top.min(0.82) } else { top };
            for (ox, oz, w, d) in [
                (-hx * 0.94, 0.0, 0.03, hz),
                (0.0, -hz * 0.94, hx, 0.03),
                (hx * 0.94, 0.0, 0.03, hz),
            ] {
                let (px, pz) = rot_pt(ox, oz);
                push_obb(verts, px, pz, 0.08, wall_h, w, d, rot, [0.76, 0.90, 0.98], 0.0);
            }
        }
        PropKind::SpaBath => {
            push_obb(verts, cx, cz, 0.0, top, hx, hz, rot, rgb, 0.0);
            push_obb(verts, cx, cz, top * 0.62, top + 0.015, hx * 0.76, hz * 0.76, rot, [0.20, 0.70, 0.86], -6.0);
        }
        PropKind::LinenCloset => {
            push_obb(verts, cx, cz, 0.0, top, hx, hz, rot, rgb, 0.0);
            push_obb(verts, cx, cz, 0.06, top * 0.94, hx * 0.92, hz * 1.03, rot, lighten(rgb, 0.90), 0.0);
        }
        PropKind::WasherDryer => {
            let cap = 0.06;
            push_obb(verts, cx, cz, 0.0, top - cap, hx, hz, rot, rgb, 0.0);
            push_obb(verts, cx, cz, top - cap, top, hx * 1.01, hz * 1.01, rot, lighten(rgb, 1.08), 0.0);
            for sx in [-hx * 0.48, hx * 0.48] {
                let (dx, dz) = rot_pt(sx, hz * 0.98);
                push_obb(verts, dx, dz, 0.24, top * 0.74, (hx * 0.28).max(0.14), 0.03, rot, [0.18, 0.22, 0.28], 0.0);
            }
        }
        PropKind::SingleBed => {
            push_obb(verts, cx, cz, 0.0, 0.22, hx, hz, rot, [0.46, 0.30, 0.18], 0.0);
            push_obb(verts, cx, cz, 0.22, 0.48, hx * 0.94, hz * 0.94, rot, rgb, 0.0);
            let (hx_p, hz_p) = rot_pt(0.0, -hz * 0.90);
            push_obb(verts, hx_p, hz_p, 0.0, top.min(0.76), hx, 0.05, rot, [0.40, 0.26, 0.15], 0.0);
            let (px, pz) = rot_pt(0.0, -hz * 0.62);
            push_obb(verts, px, pz, 0.48, 0.58, hx * 0.64, 0.16, rot, [0.96, 0.97, 0.99], 0.0);
        }
        PropKind::FilingCabinet => {
            push_obb(verts, cx, cz, 0.0, top, hx, hz, rot, rgb, 0.0);
            let (dx, dz) = rot_pt(0.0, hz * 0.98);
            push_obb(verts, dx, dz, 0.12, top * 0.90, hx * 0.84, 0.03, rot, lighten(rgb, 1.15), 0.0);
        }
        PropKind::WaterCooler => {
            push_obb(verts, cx, cz, 0.0, 0.76, hx * 0.82, hz * 0.82, rot, [0.88, 0.91, 0.94], 0.0);
            push_obb(verts, cx, cz, 0.76, top, hx * 0.56, hz * 0.56, rot, rgb, -6.0);
        }
        PropKind::BrownstoneStoop => {
            let steps = 4;
            for i in 0..steps {
                let frac = (i as f32 + 0.5) / steps as f32 - 0.5;
                let h = 0.16 + (i as f32 + 1.0) * 0.14;
                let (sx, sz) = rot_pt(0.0, -frac * (hz * 1.8));
                push_obb(
                    verts,
                    sx,
                    sz,
                    0.0,
                    h,
                    hx * 0.84,
                    (hz * 0.24).max(0.14),
                    rot,
                    if i % 2 == 0 { rgb } else { lighten(rgb, 1.10) },
                    -3.0,
                );
            }
            for side in [-hx * 0.90, hx * 0.90] {
                let (rx, rz) = rot_pt(side, 0.0);
                push_obb(verts, rx, rz, 0.0, 0.84, 0.06, hz * 0.95, rot, [0.22, 0.20, 0.22], 0.0);
            }
        }
        PropKind::AwningCanopy => {
            // Subtle sidewalk curb bar; full overhead striped canopy renders in push_awning_canopy_3d.
            let (bx, bz) = rot_pt(0.0, -hz * 0.88);
            push_obb(verts, bx, bz, 0.0, 0.06, hx, 0.04, rot, [0.22, 0.24, 0.28], 0.0);
        }
        _ => return false,
    }
    true
}

/// Renders an overhead striped storefront [`PropKind::AwningCanopy`] inside the translucent canopy pass.
pub(crate) fn push_awning_canopy_3d(
    verts: &mut Vec<GlbVertex>,
    p: &MapPropPlacement,
    cx: f32,
    cz: f32,
) {
    if p.kind != PropKind::AwningCanopy {
        return;
    }
    let fp = p.footprint_m();
    let rot = fp.rot_deg;
    let rgb = apply_prop_tint(prop_spec(p.kind).albedo_rgb, p.tint_rgb);
    let stripes = ((fp.span_w_m / 0.35).round() as i32).max(4);
    let step_x = (fp.hx_m * 1.92) / stripes as f32;
    let (sn, cs) = (fp.sin_rot, fp.cos_rot);
    for i in 0..stripes {
        let ox = -fp.hx_m * 0.96 + (i as f32 + 0.5) * step_x;
        let px = cx + ox * cs;
        let pz = cz + ox * sn;
        let col = if i % 2 == 0 { rgb } else { [0.96, 0.93, 0.86] };
        push_obb(
            verts,
            px,
            pz,
            2.35,
            2.60,
            (step_x * 0.50).max(0.08),
            fp.hy_m * 0.96,
            rot,
            col,
            TRANSLUCENT_CANOPY_ZONE,
        );
    }
}
