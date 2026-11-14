//! Continuous Vector Surface Spatial Queries (`map_surface_queries`)
//!
//! Companion module to [`crate::map_surface_geom`] providing distance-to-polyline checks,
//! closed-ring point containment, priority-ordered [`topmost_surface_at_m`] queries,
//! higher-surface masking, and road/water/channel protection checks.

use satya_schema::tactical_map_surfaces_dto::{
    MapSurfaceFeature, SurfaceFeatureKind, SurfaceShapeKind,
};

use crate::map_surface_geom::{sample_open_catmull_rom_m, sampled_closed_ring_m};

/// Computes minimum Euclidean distance in meters from `(px, py)` to a 2D polyline.
#[must_use]
pub fn distance_point_to_polyline_m(px: f32, py: f32, polyline: &[(f32, f32)]) -> f32 {
    if polyline.is_empty() {
        return f32::INFINITY;
    }
    if polyline.len() == 1 {
        return (px - polyline[0].0).hypot(py - polyline[0].1);
    }
    let mut best = f32::INFINITY;
    for w in polyline.windows(2) {
        let (d, _) = crate::tactical_map_geom::distance_point_to_segment(px, py, w[0], w[1]);
        if d < best {
            best = d;
        }
    }
    best
}

/// Ray-casting point-in-polygon check against a closed ring in meters.
#[must_use]
pub fn point_in_ring_m(px: f32, py: f32, ring: &[(f32, f32)]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (xi, yi) = ring[i];
        let (xj, yj) = ring[j];
        let crosses = (yi > py) != (yj > py);
        if crosses {
            let denom = (yj - yi).abs().max(1e-7) * if yj >= yi { 1.0 } else { -1.0 };
            let x_int = xi + (py - yi) * (xj - xi) / denom;
            if px < x_int {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Exact geometric containment test for any [`MapSurfaceFeature`] at world `(px_m, py_m)`.
#[must_use]
pub fn surface_contains_point_m(surface: &MapSurfaceFeature, px_m: f32, py_m: f32) -> bool {
    if surface.points_cm.is_empty() {
        return false;
    }
    let raw: Vec<(f32, f32)> = surface.points_cm.iter().map(|p| p.to_m()).collect();
    match surface.shape {
        SurfaceShapeKind::RibbonSpline => {
            let half_w = (surface.width_cm.max(10) as f32) / 200.0;
            let (mut min_x, mut min_y, mut max_x, mut max_y) =
                (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
            for &(x, y) in &raw {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            let pad = half_w + 1.5;
            if px_m < min_x - pad || px_m > max_x + pad || py_m < min_y - pad || py_m > max_y + pad {
                return false;
            }
            let cl = sample_open_catmull_rom_m(&raw, 6);
            distance_point_to_polyline_m(px_m, py_m, &cl) <= half_w
        }
        SurfaceShapeKind::OrganicClosed | SurfaceShapeKind::Polygon => {
            let ring = sampled_closed_ring_m(surface);
            if ring.len() < 3 {
                return false;
            }
            let (mut min_x, mut min_y, mut max_x, mut max_y) =
                (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
            for &(x, y) in &ring {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
            if px_m < min_x || px_m > max_x || py_m < min_y || py_m > max_y {
                return false;
            }
            point_in_ring_m(px_m, py_m, &ring)
        }
    }
}

/// Returns the topmost (highest [`SurfaceFeatureKind::layer_priority`], last-drawn tiebreaker)
/// surface covering `(px_m, py_m)`.
#[must_use]
pub fn topmost_surface_at_m<'a>(
    surfaces: &'a [MapSurfaceFeature],
    px_m: f32,
    py_m: f32,
) -> Option<&'a MapSurfaceFeature> {
    let mut best: Option<&'a MapSurfaceFeature> = None;
    let mut best_prio = 0u8;
    for s in surfaces {
        let prio = s.kind.layer_priority();
        if best.is_some() && prio < best_prio {
            continue;
        }
        if surface_contains_point_m(s, px_m, py_m) {
            best = Some(s);
            best_prio = prio;
        }
    }
    best
}

/// Returns `true` when `(px_m, py_m)` is covered by any surface strictly higher than
/// `min_priority` (used to mask ditch berms, road curbs, and low organic soil patches under
/// roads, walkways, or bridge overpasses).
#[must_use]
pub fn point_masked_by_higher_surface(
    surfaces: &[MapSurfaceFeature],
    min_priority: u8,
    px_m: f32,
    py_m: f32,
) -> bool {
    surfaces
        .iter()
        .any(|s| s.kind.layer_priority() > min_priority && surface_contains_point_m(s, px_m, py_m))
}

/// Returns `true` if any road, pavement, bridge, or water surface covers `(px_m, py_m)` within `pad_m`.
#[must_use]
pub fn point_near_protected_surface_m(
    surfaces: &[MapSurfaceFeature],
    px_m: f32,
    py_m: f32,
    pad_m: f32,
) -> bool {
    const DIRS: [(f32, f32); 5] = [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];
    surfaces.iter().any(|s| {
        (s.kind.is_road_or_pavement()
            || s.kind.is_water_or_channel()
            || s.kind == SurfaceFeatureKind::DirtTrail)
            && DIRS
                .iter()
                .any(|&(dx, dy)| surface_contains_point_m(s, px_m + dx * pad_m, py_m + dy * pad_m))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_surface_geom::{
        along_tangent_rot_deg, triangulate_simple_polygon_m, vehicle_forward_rot_deg,
    };
    use satya_schema::tactical_map_surfaces_dto::MapPointCm;

    #[test]
    fn catmull_rom_interpolates_control_points_smoothly() {
        let ctrl = [(0.0, 4.0), (5.0, 6.0), (11.0, 3.0), (18.0, 7.0)];
        let sampled = sample_open_catmull_rom_m(&ctrl, 8);
        assert!(sampled.len() > 20);
        assert!((sampled[0].0 - 0.0).abs() < 1e-4);
        assert!((sampled.last().unwrap().0 - 18.0).abs() < 1e-4);
        // Curve passes exactly through interior control knots.
        assert!((sampled[8].0 - 5.0).abs() < 1e-3 && (sampled[8].1 - 6.0).abs() < 1e-3);
        assert!((sampled[16].0 - 11.0).abs() < 1e-3 && (sampled[16].1 - 3.0).abs() < 1e-3);
    }

    #[test]
    fn concave_ear_clipping_covers_l_shape_exact_area() {
        let l_shape = [
            (0.0, 0.0),
            (6.0, 0.0),
            (6.0, 2.0),
            (2.0, 2.0),
            (2.0, 6.0),
            (0.0, 6.0),
        ];
        let (verts, tris) = triangulate_simple_polygon_m(&l_shape);
        assert_eq!(tris.len(), verts.len() - 2);
        let tri_area: f32 = tris
            .iter()
            .map(|&[i0, i1, i2]| {
                let (a, b, c) = (verts[i0], verts[i1], verts[i2]);
                0.5 * ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs()
            })
            .sum();
        assert!((tri_area - 20.0).abs() < 1e-3, "expected L-area 20, got {tri_area}");
    }

    #[test]
    fn surface_priority_picks_bridge_over_creek_and_road_over_mulch() {
        let mulch = MapSurfaceFeature {
            kind: SurfaceFeatureKind::MulchZone,
            shape: SurfaceShapeKind::OrganicClosed,
            points_cm: vec![
                MapPointCm::new(200, 200),
                MapPointCm::new(900, 200),
                MapPointCm::new(900, 900),
                MapPointCm::new(200, 900),
            ],
            width_cm: 0,
            border_style_cm: 0,
            variant_seed: 1,
        };
        let creek = MapSurfaceFeature {
            kind: SurfaceFeatureKind::CreekChannel,
            shape: SurfaceShapeKind::RibbonSpline,
            points_cm: vec![MapPointCm::new(0, 500), MapPointCm::new(1200, 500)],
            width_cm: 240,
            border_style_cm: 0,
            variant_seed: 2,
        };
        let bridge = MapSurfaceFeature {
            kind: SurfaceFeatureKind::BridgeDeck,
            shape: SurfaceShapeKind::Polygon,
            points_cm: vec![
                MapPointCm::new(400, 350),
                MapPointCm::new(700, 350),
                MapPointCm::new(700, 650),
                MapPointCm::new(400, 650),
            ],
            width_cm: 0,
            border_style_cm: 10,
            variant_seed: 3,
        };
        let list = [bridge.clone(), creek.clone(), mulch.clone()];
        assert_eq!(topmost_surface_at_m(&list, 5.5, 5.0).unwrap().kind, SurfaceFeatureKind::BridgeDeck);
        assert_eq!(topmost_surface_at_m(&list, 2.5, 5.0).unwrap().kind, SurfaceFeatureKind::CreekChannel);
        assert_eq!(topmost_surface_at_m(&list, 5.5, 8.0).unwrap().kind, SurfaceFeatureKind::MulchZone);
    }

    #[test]
    fn tangent_rotation_helpers_match_prop_footprint_conventions() {
        let east_deg = vehicle_forward_rot_deg(1.0, 0.0);
        let fp_east = crate::map_dressing::make_prop_at_cm(
            satya_schema::PropKind::Car,
            0,
            0,
            east_deg,
            None,
            0,
        )
        .footprint_m();
        let dir_east = fp_east.local_dir_to_world(0.0, 1.0);
        assert!((dir_east.0 - 1.0).abs() < 1e-4 && dir_east.1.abs() < 1e-4);

        // Diagonal tangent (~37 deg) rotates (+1, 0) bench length axis -> (0.8, 0.6)
        let diag_deg = along_tangent_rot_deg(0.8, 0.6);
        assert_ne!(diag_deg % 45, 0);
        let fp_diag = crate::map_dressing::make_prop_at_cm(
            satya_schema::PropKind::ParkBench,
            0,
            0,
            diag_deg,
            None,
            0,
        )
        .footprint_m();
        let dir_diag = fp_diag.local_dir_to_world(1.0, 0.0);
        assert!((dir_diag.0 - 0.8).abs() < 0.02 && (dir_diag.1 - 0.6).abs() < 0.02);
    }

    #[test]
    fn all_presets_expose_every_emitted_surface_kind_and_keep_street_props_off_asphalt() {
        use satya_schema::PropKind;
        use std::collections::HashSet;

        for preset in crate::tactical_map::SectorPreset::ALL {
            for &(w, h) in &[(20, 14), (24, 18), (36, 26)] {
                for seed in 1_i64..=12 {
                    let sec = crate::tactical_map::generate_procedural_sector(preset, w, h, seed);
                    let emitted: HashSet<SurfaceFeatureKind> =
                        sec.surfaces.iter().map(|s| s.kind).collect();
                    let mut exposed: HashSet<SurfaceFeatureKind> = HashSet::new();
                    for gy in 1..(h * 4) {
                        for gx in 1..(w * 4) {
                            if let Some(top) = topmost_surface_at_m(
                                &sec.surfaces,
                                gx as f32 * 0.25,
                                gy as f32 * 0.25,
                            ) {
                                exposed.insert(top.kind);
                            }
                        }
                    }
                    for &kind in &emitted {
                        assert!(
                            exposed.contains(&kind),
                            "{preset:?} {w}x{h} seed {seed}: emitted {kind:?} is 100% occluded under topmost_surface_at_m"
                        );
                    }

                    if matches!(
                        preset,
                        crate::tactical_map::SectorPreset::UrbanCrossroads
                            | crate::tactical_map::SectorPreset::WoodedPark
                            | crate::tactical_map::SectorPreset::GardenEstate
                            | crate::tactical_map::SectorPreset::WoodlandCreek
                            | crate::tactical_map::SectorPreset::RoadsideDitch
                    ) {
                        for p in &sec.props {
                            let (cx, cy) = p.center_m();
                            let top_kind =
                                topmost_surface_at_m(&sec.surfaces, cx, cy).map(|s| s.kind);
                            if matches!(
                                p.kind,
                                PropKind::LargeTree
                                    | PropKind::MediumTree
                                    | PropKind::Bush
                                    | PropKind::Hedge
                                    | PropKind::ParkBench
                                    | PropKind::Dumpster
                            ) {
                                assert_ne!(
                                    top_kind,
                                    Some(SurfaceFeatureKind::AsphaltRoad),
                                    "{preset:?} {w}x{h} seed {seed}: {:?} at ({cx}, {cy}) landed on AsphaltRoad",
                                    p.kind
                                );
                                assert_ne!(
                                    top_kind,
                                    Some(SurfaceFeatureKind::InteriorFloor),
                                    "{preset:?} {w}x{h} seed {seed}: {:?} at ({cx}, {cy}) landed inside InteriorFloor",
                                    p.kind
                                );
                            }
                            if matches!(
                                p.kind,
                                PropKind::Car | PropKind::UtilityVan | PropKind::JerseyBarrier
                            ) {
                                assert_ne!(
                                    top_kind,
                                    Some(SurfaceFeatureKind::InteriorFloor),
                                    "{preset:?} {w}x{h} seed {seed}: {:?} at ({cx}, {cy}) landed inside InteriorFloor",
                                    p.kind
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
