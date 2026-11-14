//! # Continuous Vector Surface 3D Geometry (`map_3d_surfaces_geom`)
//!
//! Companion to [`crate::wasm_client::map_3d_dressing_geom`] that extrudes smooth
//! [`MapSurfaceFeature`] splines (`RibbonSpline`), `OrganicClosed` regions, and rounded
//! `Polygon`s into WebGPU diorama vertices with strict priority-layered elevation,
//! road-intersection curb masking, ditch/trench berm masking under bridges, and curved
//! flower-bed stone edging.

use satya_engine::map_surface_geom::{
    extrude_surface_ribbon_m, point_masked_by_higher_surface, sample_open_catmull_rom_m,
    sampled_closed_ring_m, surface_contains_point_m, triangulate_simple_polygon_m,
};
use satya_schema::{MapSurfaceFeature, SurfaceFeatureKind, SurfaceShapeKind};

use crate::wasm_client::glb_model::GlbVertex;
use crate::wasm_client::map_3d::{TacticalMapUiState, metric_to_world_xz};
use crate::wasm_client::map_3d_wgpu::push_quad;

/// Emits a horizontal triangle at elevation `y`, enforcing `+Y` CCW winding so
/// downward-winding ear-clipped or strip triangles are never backface-culled.
#[inline]
fn push_tri_up(
    verts: &mut Vec<GlbVertex>,
    a_xz: [f32; 2],
    mut b_xz: [f32; 2],
    mut c_xz: [f32; 2],
    y: f32,
    col: [f32; 4],
    zone_id: f32,
) {
    let ny = (b_xz[1] - a_xz[1]) * (c_xz[0] - a_xz[0])
        - (b_xz[0] - a_xz[0]) * (c_xz[1] - a_xz[1]);
    if ny.abs() < 1e-6 {
        return;
    }
    if ny < 0.0 {
        std::mem::swap(&mut b_xz, &mut c_xz);
    }
    let pts = [
        [a_xz[0], y, a_xz[1]],
        [b_xz[0], y, b_xz[1]],
        [c_xz[0], y, c_xz[1]],
    ];
    let bary = [[1.0_f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for (ci, &pos) in pts.iter().enumerate() {
        verts.push(GlbVertex {
            position: pos,
            normal: [0.0, 1.0, 0.0],
            color: col,
            bary_zone: [bary[ci][0], bary[ci][1], bary[ci][2], zone_id],
        });
    }
}

/// Emits a vertical quad wall from `a_xz` to `b_xz` (`y0` -> `y1`).
#[allow(clippy::too_many_arguments)]
fn push_wall_quad(
    verts: &mut Vec<GlbVertex>,
    a_xz: [f32; 2],
    b_xz: [f32; 2],
    y0: f32,
    y1: f32,
    col: [f32; 4],
    zone_id: f32,
) {
    if (b_xz[0] - a_xz[0]).hypot(b_xz[1] - a_xz[1]) < 1e-4 {
        return;
    }
    push_quad(
        verts,
        [
            [a_xz[0], y0, a_xz[1]],
            [b_xz[0], y0, b_xz[1]],
            [b_xz[0], y1, b_xz[1]],
            [a_xz[0], y1, a_xz[1]],
        ],
        col,
        zone_id,
    );
}

/// Emits a 3D prism segment along `(ax_m, ay_m) -> (bx_m, by_m)` with half-width `hw_m`.
#[allow(clippy::too_many_arguments)]
fn push_segment_prism(
    verts: &mut Vec<GlbVertex>,
    ax_m: f32,
    ay_m: f32,
    bx_m: f32,
    by_m: f32,
    hw_m: f32,
    y0: f32,
    y1: f32,
    bw_m: f32,
    bh_m: f32,
    rgb: [f32; 3],
    zone_id: f32,
) {
    let dx = bx_m - ax_m;
    let dy = by_m - ay_m;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-4 {
        return;
    }
    // CCW quad in metric / +Y world XZ so top triangles face +Y and wall quads face outward.
    let nx = (dy / len) * hw_m;
    let ny = (-dx / len) * hw_m;
    let q0 = metric_to_world_xz(ax_m - nx, ay_m - ny, bw_m, bh_m);
    let q1 = metric_to_world_xz(ax_m + nx, ay_m + ny, bw_m, bh_m);
    let q2 = metric_to_world_xz(bx_m + nx, by_m + ny, bw_m, bh_m);
    let q3 = metric_to_world_xz(bx_m - nx, by_m - ny, bw_m, bh_m);
    let col = [rgb[0], rgb[1], rgb[2], 0.0];
    push_tri_up(verts, q0, q1, q2, y1, col, zone_id);
    push_tri_up(verts, q0, q2, q3, y1, col, zone_id);
    if y1 - y0 > 0.004 {
        push_wall_quad(verts, q0, q1, y0, y1, col, zone_id);
        push_wall_quad(verts, q1, q2, y0, y1, col, zone_id);
        push_wall_quad(verts, q2, q3, y0, y1, col, zone_id);
        push_wall_quad(verts, q3, q0, y0, y1, col, zone_id);
    }
}

fn surface_style(kind: SurfaceFeatureKind) -> ([f32; 3], f32) {
    match kind {
        SurfaceFeatureKind::AsphaltRoad => ([0.21, 0.22, 0.24], -7.0),
        SurfaceFeatureKind::ConcreteWalk => ([0.64, 0.63, 0.60], -7.0),
        SurfaceFeatureKind::StoneWalk => ([0.66, 0.62, 0.56], -3.0),
        SurfaceFeatureKind::GravelRoad => ([0.58, 0.56, 0.52], -4.0),
        SurfaceFeatureKind::DirtTrail => ([0.48, 0.37, 0.26], -4.0),
        SurfaceFeatureKind::MulchZone => ([0.31, 0.20, 0.13], -5.0),
        SurfaceFeatureKind::GardenBedZone => ([0.25, 0.36, 0.18], -5.0),
        SurfaceFeatureKind::CreekChannel => ([0.18, 0.44, 0.56], -6.0),
        SurfaceFeatureKind::DrainageDitch => ([0.30, 0.25, 0.18], -4.0),
        SurfaceFeatureKind::DefensiveTrench => ([0.36, 0.27, 0.18], -4.0),
        SurfaceFeatureKind::BridgeDeck => ([0.46, 0.32, 0.19], -3.0),
        SurfaceFeatureKind::InteriorFloor => ([0.38, 0.28, 0.19], -1.0),
    }
}

/// Emits top-surface strip triangles for `surface` (`RibbonSpline`) at elevation `y`.
fn push_ribbon_surface_fill(
    verts: &mut Vec<GlbVertex>,
    surface: &MapSurfaceFeature,
    extra_half_w_m: f32,
    y: f32,
    rgb: [f32; 3],
    zone_id: f32,
    board_m: (f32, f32),
) {
    let (bw_m, bh_m) = board_m;
    let mut widened = surface.clone();
    if extra_half_w_m.abs() > 1e-4 {
        widened.width_cm = (widened.width_cm + (extra_half_w_m * 200.0).round() as i32).max(40);
    }
    let mesh = extrude_surface_ribbon_m(&widened, bw_m, bh_m);
    let (pts_m, indices) = mesh.build_strip_triangles_m();
    let col = [rgb[0], rgb[1], rgb[2], 0.0];
    for &[i0, i1, i2] in indices.as_chunks::<3>().0 {
        let a = pts_m[i0 as usize];
        let b = pts_m[i1 as usize];
        let c = pts_m[i2 as usize];
        push_tri_up(
            verts,
            metric_to_world_xz(a[0], a[1], bw_m, bh_m),
            metric_to_world_xz(b[0], b[1], bw_m, bh_m),
            metric_to_world_xz(c[0], c[1], bw_m, bh_m),
            y,
            col,
            zone_id,
        );
    }
}

/// Emits raised road curbs + dashed yellow centerline along an `AsphaltRoad` spline.
fn push_road_details_3d(
    verts: &mut Vec<GlbVertex>,
    surface: &MapSurfaceFeature,
    surf_idx: usize,
    all_surfaces: &[MapSurfaceFeature],
    y: f32,
    bw_m: f32,
    bh_m: f32,
) {
    let mesh = extrude_surface_ribbon_m(surface, bw_m, bh_m);
    let road_prio = surface.kind.layer_priority();
    let point_blocked = |px: f32, py: f32| -> bool {
        if point_masked_by_higher_surface(all_surfaces, road_prio + 1, px, py) {
            return true;
        }
        all_surfaces.iter().enumerate().any(|(j, other)| {
            j != surf_idx
                && other.kind == SurfaceFeatureKind::AsphaltRoad
                && surface_contains_point_m(other, px, py)
        })
    };

    for edge in [&mesh.left, &mesh.right] {
        for seg in edge.windows(2) {
            let mx = (seg[0].0 + seg[1].0) * 0.5;
            let my = (seg[0].1 + seg[1].1) * 0.5;
            if point_blocked(mx, my) {
                continue;
            }
            push_segment_prism(
                verts,
                seg[0].0,
                seg[0].1,
                seg[1].0,
                seg[1].1,
                0.055,
                y,
                y + 0.036,
                bw_m,
                bh_m,
                [0.62, 0.61, 0.58],
                -7.0,
            );
        }
    }

    if surface.width_cm >= 320 {
        let ctrl_m: Vec<(f32, f32)> = surface.points_cm.iter().map(|p| p.to_m()).collect();
        let line = sample_open_catmull_rom_m(&ctrl_m, 14);
        let mut accum_m = 0.0_f32;
        for seg in line.windows(2) {
            let slen = (seg[1].0 - seg[0].0).hypot(seg[1].1 - seg[0].1);
            let mid = ((seg[0].0 + seg[1].0) * 0.5, (seg[0].1 + seg[1].1) * 0.5);
            let dash_on = (accum_m % 1.1) < 0.62;
            accum_m += slen;
            if !dash_on || point_masked_by_higher_surface(all_surfaces, road_prio + 1, mid.0, mid.1) {
                continue;
            }
            push_segment_prism(
                verts,
                seg[0].0,
                seg[0].1,
                seg[1].0,
                seg[1].1,
                0.045,
                y + 0.002,
                y + 0.007,
                bw_m,
                bh_m,
                [0.92, 0.84, 0.42],
                0.0,
            );
        }
    }
}

#[derive(Clone, Copy)]
struct ChannelBermStyle {
    height_m: f32,
    rgb: [f32; 3],
    zone_id: f32,
}

/// Emits raised berms or inner rocky cutbanks along a channel spline (masked beneath bridges/roads).
fn push_channel_berms_3d(
    verts: &mut Vec<GlbVertex>,
    surface: &MapSurfaceFeature,
    all_surfaces: &[MapSurfaceFeature],
    y: f32,
    board_m: (f32, f32),
    style: ChannelBermStyle,
) {
    let (bw_m, bh_m) = board_m;
    let mesh = extrude_surface_ribbon_m(surface, bw_m, bh_m);
    let min_mask_prio = surface.kind.layer_priority() + 1;
    for edge in [&mesh.left, &mesh.right] {
        for seg in edge.windows(2) {
            let mx = (seg[0].0 + seg[1].0) * 0.5;
            let my = (seg[0].1 + seg[1].1) * 0.5;
            if point_masked_by_higher_surface(all_surfaces, min_mask_prio, mx, my) {
                continue;
            }
            push_segment_prism(
                verts,
                seg[0].0,
                seg[0].1,
                seg[1].0,
                seg[1].1,
                0.11,
                y,
                y + style.height_m,
                bw_m,
                bh_m,
                style.rgb,
                style.zone_id,
            );
        }
    }
}

/// Emits stone edging bricks & flower tufts for `GardenBedZone` / `MulchZone`.
fn push_garden_edging_and_blooms_3d(
    verts: &mut Vec<GlbVertex>,
    ring: &[(f32, f32)],
    surface: &MapSurfaceFeature,
    y: f32,
    bw_m: f32,
    bh_m: f32,
) {
    if ring.len() < 3 {
        return;
    }
    if surface.border_style_cm > 0 {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            push_segment_prism(
                verts,
                a.0,
                a.1,
                b.0,
                b.1,
                0.05,
                y,
                y + 0.085,
                bw_m,
                bh_m,
                [0.64, 0.58, 0.50],
                -3.0,
            );
        }
    }
    if surface.kind == SurfaceFeatureKind::GardenBedZone {
        let mut cx = 0.0_f32;
        let mut cy = 0.0_f32;
        for &(px, py) in ring {
            cx += px;
            cy += py;
        }
        cx /= ring.len() as f32;
        cy /= ring.len() as f32;
        let palette = [
            [0.88, 0.34, 0.44],
            [0.95, 0.76, 0.28],
            [0.66, 0.46, 0.88],
            [0.94, 0.52, 0.32],
        ];
        let step = (ring.len() / 5).max(1);
        for (k, &pt) in ring.iter().step_by(step).take(5).enumerate() {
            let tx = cx + (pt.0 - cx) * 0.52;
            let ty = cy + (pt.1 - cy) * 0.52;
            if !surface_contains_point_m(surface, tx, ty) {
                continue;
            }
            let bloom = palette[(k + surface.variant_seed as usize) % palette.len()];
            push_segment_prism(verts, tx - 0.12, ty, tx + 0.12, ty, 0.12, y, y + 0.14, bw_m, bh_m, [0.24, 0.48, 0.21], -0.25);
            push_segment_prism(verts, tx - 0.06, ty, tx + 0.06, ty, 0.06, y + 0.14, y + 0.21, bw_m, bh_m, bloom, 0.0);
        }
    }
}

/// Emits an elevated `BridgeDeck` surface at `y = 0.12m` with support skirts & open-ended side rails.
fn push_bridge_deck_3d(
    verts: &mut Vec<GlbVertex>,
    surface: &MapSurfaceFeature,
    bw_m: f32,
    bh_m: f32,
) {
    let ring = sampled_closed_ring_m(surface);
    if ring.len() < 3 {
        return;
    }
    let (verts_m, tris) = triangulate_simple_polygon_m(&ring);
    let col = [0.46, 0.32, 0.19, 0.0];
    let deck_y = 0.12_f32;
    for t in &tris {
        let a = metric_to_world_xz(verts_m[t[0]].0, verts_m[t[0]].1, bw_m, bh_m);
        let b = metric_to_world_xz(verts_m[t[1]].0, verts_m[t[1]].1, bw_m, bh_m);
        let c = metric_to_world_xz(verts_m[t[2]].0, verts_m[t[2]].1, bw_m, bh_m);
        push_tri_up(verts, a, b, c, deck_y, col, -3.0);
    }
    // Side walls from y=0.0 up to y=0.12 around perimeter
    let skirt_col = [0.34, 0.23, 0.14, 0.0];
    for i in 0..ring.len() {
        let a = metric_to_world_xz(ring[i].0, ring[i].1, bw_m, bh_m);
        let b = metric_to_world_xz(ring[(i + 1) % ring.len()].0, ring[(i + 1) % ring.len()].1, bw_m, bh_m);
        push_wall_quad(verts, a, b, 0.0, deck_y, skirt_col, -3.0);
    }
    // Bridge side guard rails along [0]->[1] and [2]->[3] (leaving entry/exit ends [1]->[2] & [3]->[0] open)
    if surface.points_cm.len() == 4 {
        let p = [
            surface.points_cm[0].to_m(),
            surface.points_cm[1].to_m(),
            surface.points_cm[2].to_m(),
            surface.points_cm[3].to_m(),
        ];
        for (a, b) in [(p[0], p[1]), (p[2], p[3])] {
            push_segment_prism(
                verts,
                a.0,
                a.1,
                b.0,
                b.1,
                0.045,
                deck_y,
                deck_y + 0.12,
                bw_m,
                bh_m,
                [0.54, 0.39, 0.24],
                -3.0,
            );
        }
    }
}

/// Emits smooth continuous vector surfaces from `ui.map.surfaces` in strict priority order.
pub fn push_vector_surfaces_3d(verts: &mut Vec<GlbVertex>, ui: &TacticalMapUiState) {
    if ui.map.surfaces.is_empty() {
        return;
    }
    let (bw_m, bh_m) = ui.map.board_size_m();
    let mut order: Vec<(usize, &MapSurfaceFeature)> = ui.map.surfaces.iter().enumerate().collect();
    order.sort_by_key(|(idx, s)| (s.kind.layer_priority(), *idx));

    for (rank, &(orig_idx, s)) in order.iter().enumerate() {
        let prio = s.kind.layer_priority();
        let y = 0.003 + (prio as f32) * 0.00032 + (rank as f32) * 0.00004;
        let (rgb, zone_id) = surface_style(s.kind);

        if s.kind == SurfaceFeatureKind::BridgeDeck {
            push_bridge_deck_3d(verts, s, bw_m, bh_m);
            continue;
        }

        match s.shape {
            SurfaceShapeKind::RibbonSpline => {
                if s.kind == SurfaceFeatureKind::CreekChannel {
                    push_ribbon_surface_fill(verts, s, 0.42, y, [0.46, 0.45, 0.42], -3.0, (bw_m, bh_m));
                    push_ribbon_surface_fill(verts, s, 0.0, y + 0.0045, rgb, zone_id, (bw_m, bh_m));
                    let mut bank = s.clone();
                    bank.width_cm += 84;
                    push_channel_berms_3d(
                        verts,
                        &bank,
                        &ui.map.surfaces,
                        y,
                        (bw_m, bh_m),
                        ChannelBermStyle {
                            height_m: 0.09,
                            rgb: [0.42, 0.39, 0.34],
                            zone_id: -3.0,
                        },
                    );
                } else {
                    push_ribbon_surface_fill(verts, s, 0.0, y, rgb, zone_id, (bw_m, bh_m));
                }
                if s.kind == SurfaceFeatureKind::AsphaltRoad {
                    push_road_details_3d(verts, s, orig_idx, &ui.map.surfaces, y, bw_m, bh_m);
                } else if matches!(
                    s.kind,
                    SurfaceFeatureKind::DrainageDitch | SurfaceFeatureKind::DefensiveTrench
                ) {
                    let height_m = if s.kind == SurfaceFeatureKind::DefensiveTrench {
                        0.18
                    } else {
                        0.09
                    };
                    push_channel_berms_3d(
                        verts,
                        s,
                        &ui.map.surfaces,
                        y,
                        (bw_m, bh_m),
                        ChannelBermStyle {
                            height_m,
                            rgb: [0.37, 0.30, 0.21],
                            zone_id: -4.0,
                        },
                    );
                }
            }
            SurfaceShapeKind::OrganicClosed | SurfaceShapeKind::Polygon => {
                let ring = sampled_closed_ring_m(s);
                let (verts_m, tris) = triangulate_simple_polygon_m(&ring);
                let col = [rgb[0], rgb[1], rgb[2], 0.0];
                for t in &tris {
                    let a = metric_to_world_xz(verts_m[t[0]].0, verts_m[t[0]].1, bw_m, bh_m);
                    let b = metric_to_world_xz(verts_m[t[1]].0, verts_m[t[1]].1, bw_m, bh_m);
                    let c = metric_to_world_xz(verts_m[t[2]].0, verts_m[t[2]].1, bw_m, bh_m);
                    push_tri_up(verts, a, b, c, y, col, zone_id);
                }
                if matches!(
                    s.kind,
                    SurfaceFeatureKind::GardenBedZone | SurfaceFeatureKind::MulchZone
                ) {
                    push_garden_edging_and_blooms_3d(verts, &ring, s, y, bw_m, bh_m);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::map_3d_wgpu::build_tactical_diorama_vertices;
    use satya_engine::tactical_map::{SectorPreset, build_procedural_map_request};

    #[test]
    fn vector_surfaces_build_upward_finite_3d_geometry_across_outdoor_and_urban_presets() {
        for preset in [
            SectorPreset::WoodedPark,
            SectorPreset::GardenEstate,
            SectorPreset::WoodlandCreek,
            SectorPreset::RoadsideDitch,
            SectorPreset::UrbanCrossroads,
        ] {
            let ui = TacticalMapUiState {
                map: build_procedural_map_request("v3d", preset, None, 20, 14, 42),
                ..Default::default()
            };
            assert!(!ui.map.surfaces.is_empty(), "{preset:?} missing vector surfaces");
            let verts = build_tactical_diorama_vertices(&ui);
            assert!(verts.len() > 200, "{preset:?} emitted too few 3D vertices");
            for v in &verts {
                assert!(
                    v.position.iter().all(|c| c.is_finite()) && v.normal.iter().all(|c| c.is_finite()),
                    "{preset:?} emitted non-finite vertex"
                );
            }
            if matches!(preset, SectorPreset::WoodlandCreek | SectorPreset::RoadsideDitch) {
                assert!(
                    verts.iter().any(|v| (v.position[1] - 0.12).abs() < 1e-4),
                    "{preset:?} expected elevated 0.12m BridgeDeck geometry"
                );
            }
        }
    }
}
