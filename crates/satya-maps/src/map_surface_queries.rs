//! Continuous Vector Surface Spatial Queries (`map_surface_queries`)
//!
//! Companion module to [`crate::map_surface_geom`] providing distance-to-polyline checks,
//! closed-ring point containment, priority-ordered [`topmost_surface_at_m`] queries,
//! higher-surface masking, and road/water/channel protection checks.

use satya_schema::tactical_map_surfaces_dto::{
    MapSurfaceFeature, SurfaceFeatureKind, SurfaceShapeKind,
};

use crate::map_surface_geom::{
    local_ribbon_half_width_m, ribbon_edge_roughness_spec, sample_open_catmull_rom_m,
    sampled_closed_ring_m, visit_open_catmull_rom_segments,
};

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
    let (mut min_x_cm, mut min_y_cm, mut max_x_cm, mut max_y_cm) =
        (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for pt in &surface.points_cm {
        min_x_cm = min_x_cm.min(pt.x_cm);
        min_y_cm = min_y_cm.min(pt.y_cm);
        max_x_cm = max_x_cm.max(pt.x_cm);
        max_y_cm = max_y_cm.max(pt.y_cm);
    }
    let half_w = (surface.width_cm.max(10) as f32) / 200.0;
    let (rough_rel, rough_max_amp) = ribbon_edge_roughness_spec(surface.kind);
    let quick_pad_m = match surface.shape {
        SurfaceShapeKind::RibbonSpline => half_w + rough_max_amp + 1.5,
        SurfaceShapeKind::OrganicClosed => 1.5,
        SurfaceShapeKind::Polygon => 0.1,
    };
    if px_m < min_x_cm as f32 / 100.0 - quick_pad_m
        || px_m > max_x_cm as f32 / 100.0 + quick_pad_m
        || py_m < min_y_cm as f32 / 100.0 - quick_pad_m
        || py_m > max_y_cm as f32 / 100.0 + quick_pad_m
    {
        return false;
    }

    match surface.shape {
        SurfaceShapeKind::RibbonSpline => {
            let subdivs = if surface.points_cm.len() >= 8 { 4 } else { 6 };
            let mut best_dist = f32::INFINITY;
            let mut best_arc = 0.0_f32;
            let mut best_left = true;
            let mut arc_m = 0.0_f32;
            visit_open_catmull_rom_segments(&surface.points_cm, subdivs, |a, b| {
                let (d, t) = crate::tactical_map_geom::distance_point_to_segment(px_m, py_m, a, b);
                if rough_rel <= 0.0 {
                    if d < best_dist {
                        best_dist = d;
                    }
                } else {
                    let seg_len = (b.0 - a.0).hypot(b.1 - a.1);
                    if d < best_dist {
                        best_dist = d;
                        best_arc = arc_m + t * seg_len;
                        let qx = a.0 + (b.0 - a.0) * t;
                        let qy = a.1 + (b.1 - a.1) * t;
                        best_left = (b.0 - a.0) * (py_m - qy) - (b.1 - a.1) * (px_m - qx) >= 0.0;
                    }
                    arc_m += seg_len;
                }
            });
            if rough_rel <= 0.0 {
                best_dist <= half_w
            } else {
                let local_hw = local_ribbon_half_width_m(
                    half_w,
                    surface.kind,
                    surface.variant_seed,
                    best_left,
                    best_arc,
                    1.0,
                );
                best_dist <= local_hw
            }
        }
        SurfaceShapeKind::OrganicClosed | SurfaceShapeKind::Polygon => {
            let ring = sampled_closed_ring_m(surface);
            if ring.len() < 3 {
                return false;
            }
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            );
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

pub(crate) enum PreSampledSurfaceShape {
    Empty,
    Ring(Vec<(f32, f32)>),
    Ribbon {
        cl: Vec<(f32, f32)>,
        cum_arc: Vec<f32>,
        half_w: f32,
        rough_rel: f32,
        variant_seed: u32,
    },
}

pub(crate) struct PreSampledSurfaceEntry {
    pub(crate) kind: SurfaceFeatureKind,
    pub(crate) prio: u8,
    pub(crate) min_x: f32,
    pub(crate) min_y: f32,
    pub(crate) max_x: f32,
    pub(crate) max_y: f32,
    pub(crate) shape: PreSampledSurfaceShape,
}

impl PreSampledSurfaceEntry {
    pub(crate) fn from_feature(s: &MapSurfaceFeature) -> Self {
        let prio = s.kind.layer_priority();
        if s.points_cm.is_empty() {
            return Self {
                kind: s.kind,
                prio,
                min_x: 0.0,
                min_y: 0.0,
                max_x: -1.0,
                max_y: -1.0,
                shape: PreSampledSurfaceShape::Empty,
            };
        }
        match s.shape {
            SurfaceShapeKind::RibbonSpline => {
                let raw: Vec<(f32, f32)> = s.points_cm.iter().map(|p| p.to_m()).collect();
                let subdivs = if s.points_cm.len() >= 8 { 4 } else { 6 };
                let cl = sample_open_catmull_rom_m(&raw, subdivs);
                let half_w = (s.width_cm.max(10) as f32) / 200.0;
                let (rough_rel, rough_max_amp) = ribbon_edge_roughness_spec(s.kind);
                let pad = half_w + rough_max_amp;
                let (mut min_x, mut min_y, mut max_x, mut max_y) = (
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                );
                for &(x, y) in &cl {
                    min_x = min_x.min(x - pad);
                    min_y = min_y.min(y - pad);
                    max_x = max_x.max(x + pad);
                    max_y = max_y.max(y + pad);
                }
                let mut cum_arc = Vec::with_capacity(cl.len());
                cum_arc.push(0.0_f32);
                for w in cl.windows(2) {
                    let seg = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
                    cum_arc.push(cum_arc[cum_arc.len() - 1] + seg);
                }
                Self {
                    kind: s.kind,
                    prio,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    shape: PreSampledSurfaceShape::Ribbon {
                        cl,
                        cum_arc,
                        half_w,
                        rough_rel,
                        variant_seed: s.variant_seed,
                    },
                }
            }
            SurfaceShapeKind::OrganicClosed | SurfaceShapeKind::Polygon => {
                let ring = sampled_closed_ring_m(s);
                if ring.len() < 3 {
                    return Self {
                        kind: s.kind,
                        prio,
                        min_x: 0.0,
                        min_y: 0.0,
                        max_x: -1.0,
                        max_y: -1.0,
                        shape: PreSampledSurfaceShape::Empty,
                    };
                }
                let (mut min_x, mut min_y, mut max_x, mut max_y) = (
                    f32::INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NEG_INFINITY,
                );
                for &(x, y) in &ring {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
                Self {
                    kind: s.kind,
                    prio,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    shape: PreSampledSurfaceShape::Ring(ring),
                }
            }
        }
    }

    #[inline]
    fn contains(&self, px_m: f32, py_m: f32) -> bool {
        if px_m < self.min_x || px_m > self.max_x || py_m < self.min_y || py_m > self.max_y {
            return false;
        }
        match &self.shape {
            PreSampledSurfaceShape::Empty => false,
            PreSampledSurfaceShape::Ring(ring) => point_in_ring_m(px_m, py_m, ring),
            PreSampledSurfaceShape::Ribbon {
                cl,
                cum_arc,
                half_w,
                rough_rel,
                variant_seed,
            } => {
                if *rough_rel <= 0.0 {
                    return distance_point_to_polyline_m(px_m, py_m, cl) <= *half_w;
                }
                if cl.len() < 2 {
                    return cl
                        .first()
                        .is_some_and(|&(x, y)| (px_m - x).hypot(py_m - y) <= *half_w);
                }
                let mut best_dist = f32::INFINITY;
                let mut best_arc = 0.0_f32;
                let mut best_left = true;
                for i in 0..cl.len() - 1 {
                    let (a, b) = (cl[i], cl[i + 1]);
                    let (d, t) =
                        crate::tactical_map_geom::distance_point_to_segment(px_m, py_m, a, b);
                    if d < best_dist {
                        best_dist = d;
                        let seg_len = cum_arc[i + 1] - cum_arc[i];
                        best_arc = cum_arc[i] + t * seg_len;
                        let qx = a.0 + (b.0 - a.0) * t;
                        let qy = a.1 + (b.1 - a.1) * t;
                        best_left = (b.0 - a.0) * (py_m - qy) - (b.1 - a.1) * (px_m - qx) >= 0.0;
                    }
                }
                let local_hw = local_ribbon_half_width_m(
                    *half_w,
                    self.kind,
                    *variant_seed,
                    best_left,
                    best_arc,
                    1.0,
                );
                best_dist <= local_hw
            }
        }
    }
}

/// Pre-sampled surface index accelerating dense grid queries (`prune_occluded_surfaces`, test grids).
pub struct PreparedSurfaceGrid {
    entries: Vec<PreSampledSurfaceEntry>,
}

impl PreparedSurfaceGrid {
    /// Pre-samples all vector surface contours and tight bounding boxes once.
    #[must_use]
    pub fn new(surfaces: &[MapSurfaceFeature]) -> Self {
        Self {
            entries: surfaces
                .iter()
                .map(PreSampledSurfaceEntry::from_feature)
                .collect(),
        }
    }

    /// Returns `true` if surface `idx` covers world point `(px_m, py_m)`.
    #[must_use]
    pub fn contains_surface(&self, idx: usize, px_m: f32, py_m: f32) -> bool {
        self.entries
            .get(idx)
            .is_some_and(|e| e.contains(px_m, py_m))
    }

    /// Returns the topmost [`SurfaceFeatureKind`] at `(px_m, py_m)` matching [`topmost_surface_at_m`].
    #[must_use]
    pub fn topmost_kind_at_m(&self, px_m: f32, py_m: f32) -> Option<SurfaceFeatureKind> {
        let mut best: Option<SurfaceFeatureKind> = None;
        let mut best_prio = 0u8;
        for entry in &self.entries {
            if best.is_some() && entry.prio < best_prio {
                continue;
            }
            if entry.contains(px_m, py_m) {
                best = Some(entry.kind);
                best_prio = entry.prio;
            }
        }
        best
    }
}

/// Drops any surface patch that has zero exposed `0.25m` sample points inside `[0, w] x [0, h]`
/// (for instance, a cramped decorative bed completely covered by a higher-priority building slab).
pub fn prune_occluded_surfaces(surfaces: &mut Vec<MapSurfaceFeature>, w: i32, h: i32) {
    let max_gx = (w * 4 - 1).max(1);
    let max_gy = (h * 4 - 1).max(1);
    let grid = PreparedSurfaceGrid::new(surfaces);
    let mut keep = vec![false; surfaces.len()];
    for (idx, entry) in grid.entries.iter().enumerate() {
        if entry.max_x < entry.min_x || entry.max_y < entry.min_y {
            continue;
        }
        let gx_min = ((entry.min_x * 4.0).floor() as i32).clamp(1, max_gx);
        let gx_max = ((entry.max_x * 4.0).ceil() as i32).clamp(gx_min, max_gx);
        let gy_min = ((entry.min_y * 4.0).floor() as i32).clamp(1, max_gy);
        let gy_max = ((entry.max_y * 4.0).ceil() as i32).clamp(gy_min, max_gy);

        let mut exposed = false;
        for gy in gy_min..=gy_max {
            let py = gy as f32 * 0.25;
            for gx in gx_min..=gx_max {
                let px = gx as f32 * 0.25;
                if grid.contains_surface(idx, px, py)
                    && grid.topmost_kind_at_m(px, py) == Some(entry.kind)
                {
                    exposed = true;
                    break;
                }
            }
            if exposed {
                break;
            }
        }
        keep[idx] = exposed;
    }
    let mut i = 0;
    surfaces.retain(|_| {
        let k = keep[i];
        i += 1;
        k
    });
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
        assert!(
            (tri_area - 20.0).abs() < 1e-3,
            "expected L-area 20, got {tri_area}"
        );
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
            tint_rgb: None,
        };
        let creek = MapSurfaceFeature {
            kind: SurfaceFeatureKind::CreekChannel,
            shape: SurfaceShapeKind::RibbonSpline,
            points_cm: vec![MapPointCm::new(0, 500), MapPointCm::new(1200, 500)],
            width_cm: 240,
            border_style_cm: 0,
            variant_seed: 2,
            tint_rgb: None,
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
            tint_rgb: None,
        };
        let list = [bridge.clone(), creek.clone(), mulch.clone()];
        assert_eq!(
            topmost_surface_at_m(&list, 5.5, 5.0).unwrap().kind,
            SurfaceFeatureKind::BridgeDeck
        );
        assert_eq!(
            topmost_surface_at_m(&list, 2.5, 5.0).unwrap().kind,
            SurfaceFeatureKind::CreekChannel
        );
        assert_eq!(
            topmost_surface_at_m(&list, 5.5, 8.0).unwrap().kind,
            SurfaceFeatureKind::MulchZone
        );
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
    fn ribbon_caps_bulge_outward_across_cardinal_and_diagonal_axes() {
        use crate::map_surface_geom::{extrude_ribbon_strip_m, signed_area_2d};

        let half_w = 0.925_f32;
        let cases = [
            ((5.0, 5.0), (12.0, 5.0)),
            ((12.0, 5.0), (5.0, 5.0)),
            ((5.0, 5.0), (5.0, 12.0)),
            ((5.0, 12.0), (5.0, 5.0)),
            ((4.0, 4.0), (10.0, 9.0)),
        ];
        for (a, b) in cases {
            let mesh = extrude_ribbon_strip_m(&[a, b], half_w, true, true);
            let ring = mesh.to_closed_ring_m();
            let area = signed_area_2d(&ring).abs();
            let seg_len = (b.0 - a.0).hypot(b.1 - a.1);
            let rect_area = seg_len * 2.0 * half_w;
            assert!(
                area > rect_area + 2.0 * half_w * half_w,
                "ribbon {a:?}->{b:?} endcap inverted (area={area:.2}, rect={rect_area:.2})"
            );
            let tx = (b.0 - a.0) / seg_len;
            let ty = (b.1 - a.1) / seg_len;
            let start_ext = ring
                .iter()
                .map(|&(px, py)| (a.0 - px) * tx + (a.1 - py) * ty)
                .fold(f32::NEG_INFINITY, f32::max);
            let end_ext = ring
                .iter()
                .map(|&(px, py)| (px - b.0) * tx + (py - b.1) * ty)
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                start_ext >= 0.9 * half_w,
                "start cap did not bulge outward for {a:?}->{b:?}: {start_ext}"
            );
            assert!(
                end_ext >= 0.9 * half_w,
                "end cap did not bulge outward for {a:?}->{b:?}: {end_ext}"
            );
        }
    }

    #[test]
    fn all_presets_expose_every_emitted_surface_kind_and_keep_street_props_off_asphalt() {
        use satya_schema::PropKind;
        use std::collections::HashSet;

        for preset in crate::tactical_map::SectorPreset::ALL {
            for &(w, h) in &[(20, 14), (24, 18), (36, 26)] {
                for seed in 1_i64..=12 {
                    let sec = crate::tactical_map::generate_procedural_sector(preset, w, h, seed);
                    let grid = PreparedSurfaceGrid::new(&sec.surfaces);
                    let emitted: HashSet<SurfaceFeatureKind> =
                        sec.surfaces.iter().map(|s| s.kind).collect();
                    let mut exposed: HashSet<SurfaceFeatureKind> = HashSet::new();
                    for gy in 1..(h * 4) {
                        for gx in 1..(w * 4) {
                            if let Some(kind) =
                                grid.topmost_kind_at_m(gx as f32 * 0.25, gy as f32 * 0.25)
                            {
                                exposed.insert(kind);
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
                            | crate::tactical_map::SectorPreset::UrbanResidential
                            | crate::tactical_map::SectorPreset::ProjectsHighDensity
                            | crate::tactical_map::SectorPreset::SuburbanNeighborhood
                            | crate::tactical_map::SectorPreset::RuralCountryside
                            | crate::tactical_map::SectorPreset::LuxuryMansion
                            | crate::tactical_map::SectorPreset::DowntownCommercial
                            | crate::tactical_map::SectorPreset::Hospital
                            | crate::tactical_map::SectorPreset::ShoppingPlaza
                            | crate::tactical_map::SectorPreset::FastFoodRestaurant
                            | crate::tactical_map::SectorPreset::BistroDiner
                            | crate::tactical_map::SectorPreset::FineDiningRestaurant
                            | crate::tactical_map::SectorPreset::Farms
                            | crate::tactical_map::SectorPreset::BotanicalGarden
                            | crate::tactical_map::SectorPreset::ManufacturingPlant
                            | crate::tactical_map::SectorPreset::Warehouse
                    ) {
                        for p in &sec.props {
                            let (cx, cy) = p.center_m();
                            let top_kind = grid.topmost_kind_at_m(cx, cy);
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
                                assert!(
                                    !top_kind
                                        .is_some_and(SurfaceFeatureKind::is_built_floor_or_deck),
                                    "{preset:?} {w}x{h} seed {seed}: {:?} at ({cx}, {cy}) landed inside built floor/deck/pool ({top_kind:?})",
                                    p.kind
                                );
                            }
                            if matches!(
                                p.kind,
                                PropKind::Car
                                    | PropKind::UtilityVan
                                    | PropKind::PickupTruck
                                    | PropKind::JerseyBarrier
                            ) {
                                assert!(
                                    !top_kind
                                        .is_some_and(SurfaceFeatureKind::is_built_floor_or_deck),
                                    "{preset:?} {w}x{h} seed {seed}: {:?} at ({cx}, {cy}) landed inside built floor/deck/pool ({top_kind:?})",
                                    p.kind
                                );
                            }
                        }
                    }

                    for fl in &sec.floors {
                        let fl_grid = PreparedSurfaceGrid::new(&fl.surfaces);
                        let fl_emitted: HashSet<SurfaceFeatureKind> =
                            fl.surfaces.iter().map(|s| s.kind).collect();
                        let mut fl_exposed: HashSet<SurfaceFeatureKind> = HashSet::new();
                        for gy in 1..(h * 4) {
                            for gx in 1..(w * 4) {
                                if let Some(kind) =
                                    fl_grid.topmost_kind_at_m(gx as f32 * 0.25, gy as f32 * 0.25)
                                {
                                    fl_exposed.insert(kind);
                                }
                            }
                        }
                        for &kind in &fl_emitted {
                            assert!(
                                fl_exposed.contains(&kind),
                                "{preset:?} {w}x{h} seed {seed} floor {}: emitted {kind:?} 100% occluded",
                                fl.floor_index
                            );
                        }
                    }
                }
            }
        }
    }
}
