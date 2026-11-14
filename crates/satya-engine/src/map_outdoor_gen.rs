//! # Procedural Outdoor & Compound Battlemap Generator (`map_outdoor_gen`)
//!
//! Emits smooth continuous vector terrain (`MapSurfaceFeature`), thin perimeter/house walls
//! (`MapWallEdge`), and true `1°`-oriented (`0..360°`) cover props for all outdoor styles.
//!
//! Natural creek and roadside ditch builders live in [`crate::map_outdoor_natural`].

use std::f32::consts::TAU;

use satya_schema::{
    EdgeSide, MapPointCm, MapPropPlacement, MapSurfaceFeature, MapWallEdge, MaterialClass,
    PropKind, SurfaceFeatureKind, SurfaceShapeKind, WallEdgeKind,
};

use crate::map_dressing::{
    PropObstruction, make_prop_at_cm, normalize_prop_rotation_deg, prop_spec,
};
use crate::map_outdoor_natural::{
    build_garden_estate, build_roadside_ditch, build_wooded_park, build_woodland_creek,
};
use crate::map_surface_geom::{
    along_tangent_rot_deg, point_near_protected_surface_m, ribbon_point_and_tangent_at_fraction,
    surface_contains_point_m, vehicle_forward_rot_deg,
};
use crate::tactical_map::ProceduralSector;
use crate::tactical_map_edges::WallEdgeIndex;
use crate::tactical_map_geom::{
    TOKEN_RADIUS_M, all_walkable_areas_connected, prop_crosses_wall_edge, prop_intersects_circle,
    props_overlap_continuous,
};

/// Outdoor procedural style routed from `SectorPreset::outdoor_style()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutdoorStyle {
    WoodedPark,
    GardenEstate,
    WoodlandCreek,
    RoadsideDitch,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        let mut x = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        self.0 = x;
        x ^ (x >> 31)
    }

    pub(crate) fn range(&mut self, lo: i32, hi_excl: i32) -> i32 {
        if hi_excl <= lo {
            return lo;
        }
        lo + (self.next() % (hi_excl - lo) as u64) as i32
    }

    pub(crate) fn chance(&mut self, num: u64, den: u64) -> bool {
        den > 0 && (self.next() % den) < num
    }

    pub(crate) fn f32_range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next() & 0x00ff_ffff) as f32 / 16_777_215.0;
        lo + (hi - lo) * u
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RectM {
    pub(crate) x0: f32,
    pub(crate) y0: f32,
    pub(crate) x1: f32,
    pub(crate) y1: f32,
}

pub(crate) struct OutdoorBuilder {
    pub(crate) w: i32,
    pub(crate) h: i32,
    pub(crate) rng: Rng,
    pub(crate) edges: Vec<MapWallEdge>,
    pub(crate) edge_idx: WallEdgeIndex,
    pub(crate) props: Vec<MapPropPlacement>,
    pub(crate) surfaces: Vec<MapSurfaceFeature>,
    pub(crate) spawn_pads_m: Vec<(f32, f32)>,
    pub(crate) tree_centers_m: Vec<(f32, f32)>,
}

impl OutdoorBuilder {
    fn new(w: i32, h: i32, seed: i64) -> Self {
        let mid_y = (h / 2) as f32;
        let sara_y = (h / 2 - 1).max(1) as f32;
        let spawn_pads_m = vec![
            (2.5, sara_y + 0.5),
            (2.5, mid_y + 0.5),
            ((w - 3) as f32 + 0.5, mid_y + 0.5),
            ((w - 3) as f32 + 0.5, ((h / 2 + 1).min(h - 2)) as f32 + 0.5),
        ];
        Self {
            w,
            h,
            rng: Rng((seed as u64) ^ 0x51a7_9e37_79b9_f00d),
            edges: Vec::new(),
            edge_idx: WallEdgeIndex::from_edges(w, h, &[]),
            props: Vec::new(),
            surfaces: Vec::new(),
            spawn_pads_m,
            tree_centers_m: Vec::new(),
        }
    }

    #[inline]
    pub(crate) fn cm(v_m: f32) -> i32 {
        (v_m * 100.0).round() as i32
    }

    pub(crate) fn put_edge(
        &mut self,
        x: i32,
        y: i32,
        side: EdgeSide,
        kind: WallEdgeKind,
        material: MaterialClass,
    ) {
        let valid = match side {
            EdgeSide::West => x >= 0 && x <= self.w && y >= 0 && y < self.h,
            EdgeSide::North => x >= 0 && x < self.w && y >= 0 && y <= self.h,
        };
        if !valid {
            return;
        }
        let edge = MapWallEdge { x, y, side, kind, material };
        self.edge_idx.set(x, y, side, Some(kind));
        match self.edges.iter_mut().find(|o| o.x == x && o.y == y && o.side == side) {
            Some(existing) => *existing = edge,
            None => self.edges.push(edge),
        }
    }

    /// Appends a smooth Catmull-Rom [`SurfaceShapeKind::RibbonSpline`] feature.
    pub(crate) fn add_ribbon_spline(
        &mut self,
        kind: SurfaceFeatureKind,
        ctrl_m: &[(f32, f32)],
        width_cm: i32,
        variant_seed: u32,
    ) -> usize {
        let idx = self.surfaces.len();
        self.surfaces.push(MapSurfaceFeature {
            kind,
            shape: SurfaceShapeKind::RibbonSpline,
            points_cm: ctrl_m
                .iter()
                .map(|&(x, y)| MapPointCm::new(Self::cm(x), Self::cm(y)))
                .collect(),
            width_cm,
            border_style_cm: 0,
            variant_seed,
        });
        idx
    }

    /// Appends a polygonal or corner-rounded surface region (`InteriorFloor`, `BridgeDeck`, etc.).
    pub(crate) fn add_polygon_surface(
        &mut self,
        kind: SurfaceFeatureKind,
        pts_m: &[(f32, f32)],
        corner_radius_cm: i32,
        variant_seed: u32,
    ) -> usize {
        let idx = self.surfaces.len();
        self.surfaces.push(MapSurfaceFeature {
            kind,
            shape: SurfaceShapeKind::Polygon,
            points_cm: pts_m
                .iter()
                .map(|&(x, y)| MapPointCm::new(Self::cm(x), Self::cm(y)))
                .collect(),
            width_cm: 0,
            border_style_cm: corner_radius_cm,
            variant_seed,
        });
        idx
    }

    /// Appends an elevated [`SurfaceFeatureKind::BridgeDeck`] spanning perpendicularly across `channel_idx`
    /// at normalized arc fraction `frac`.
    pub(crate) fn add_bridge_deck_across(
        &mut self,
        channel_idx: usize,
        frac: f32,
        bridge_width_m: f32,
        bridge_span_m: f32,
    ) -> bool {
        let Some(ch) = self.surfaces.get(channel_idx).cloned() else {
            return false;
        };
        let Some((pos, (tx, ty))) = ribbon_point_and_tangent_at_fraction(&ch, frac) else {
            return false;
        };
        // Span axis runs perpendicular to channel tangent: (-ty, tx).
        let (sx, sy) = (-ty, tx);
        let hw = bridge_width_m * 0.5;
        let hs = bridge_span_m * 0.5;
        let corners = [
            (pos.0 - tx * hw - sx * hs, pos.1 - ty * hw - sy * hs),
            (pos.0 + tx * hw - sx * hs, pos.1 + ty * hw - sy * hs),
            (pos.0 + tx * hw + sx * hs, pos.1 + ty * hw + sy * hs),
            (pos.0 - tx * hw + sx * hs, pos.1 - ty * hw + sy * hs),
        ];
        let seed = (self.rng.next() & 0xffff_ffff) as u32;
        self.add_polygon_surface(SurfaceFeatureKind::BridgeDeck, &corners, 14, seed);
        true
    }

    /// Generates an organic undulating loop (`MulchZone` / `GardenBedZone`) that strictly stays off
    /// higher-priority roads, walkways, interior floors, and water channels by `pad_m`.
    pub(crate) fn try_add_organic_patch(
        &mut self,
        kind: SurfaceFeatureKind,
        center_m: (f32, f32),
        radii_m: (f32, f32),
        pad_m: f32,
        border_style_cm: i32,
    ) -> bool {
        let (cx_m, cy_m) = center_m;
        let (rx_m, ry_m) = radii_m;
        let n_pts = 8usize;
        let base_rot = self.rng.f32_range(0.0, TAU);
        for &scale in &[1.0_f32, 0.78, 0.62] {
            let mut ring = Vec::with_capacity(n_pts);
            let mut blocked = self.patch_point_blocked(cx_m, cy_m, pad_m);
            if !blocked {
                for i in 0..n_pts {
                    let ang = base_rot + TAU * (i as f32 / n_pts as f32) + self.rng.f32_range(-0.14, 0.14);
                    let wob = self.rng.f32_range(0.76, 1.18);
                    let px = (cx_m + rx_m * scale * wob * ang.cos()).clamp(0.35, self.w as f32 - 0.35);
                    let py = (cy_m + ry_m * scale * wob * ang.sin()).clamp(0.35, self.h as f32 - 0.35);
                    if self.patch_point_blocked(px, py, pad_m) {
                        blocked = true;
                        break;
                    }
                    ring.push((px, py));
                }
            }
            if !blocked && ring.len() >= 5 {
                let seed = (self.rng.next() & 0xffff_ffff) as u32;
                self.surfaces.push(MapSurfaceFeature {
                    kind,
                    shape: SurfaceShapeKind::OrganicClosed,
                    points_cm: ring
                        .into_iter()
                        .map(|(x, y)| MapPointCm::new(Self::cm(x), Self::cm(y)))
                        .collect(),
                    width_cm: 0,
                    border_style_cm,
                    variant_seed: seed,
                });
                return true;
            }
        }
        false
    }

    fn patch_point_blocked(&self, px: f32, py: f32, pad_m: f32) -> bool {
        if point_near_protected_surface_m(&self.surfaces, px, py, pad_m) {
            return true;
        }
        self.surfaces.iter().any(|s| {
            s.kind == SurfaceFeatureKind::InteriorFloor && surface_contains_point_m(s, px, py)
        })
    }

    fn prop_clears_surfaces(&self, prop: &MapPropPlacement, kind: PropKind, allow_on_pavement: bool) -> bool {
        if allow_on_pavement {
            return true;
        }
        let fp = prop.footprint_m();
        let hx = fp.span_w_m * 0.45;
        let hy = fp.span_h_m * 0.45;
        let pad = if matches!(kind, PropKind::GardenBed | PropKind::LargeTree | PropKind::MediumTree) {
            0.35
        } else {
            0.22
        };
        for &(ox, oy) in &[(0.0, 0.0), (-hx, -hy), (hx, -hy), (-hx, hy), (hx, hy)] {
            let (wx, wy) = fp.local_offset_to_world(ox, oy);
            if point_near_protected_surface_m(&self.surfaces, wx, wy, pad) {
                return false;
            }
            if self.surfaces.iter().any(|s| {
                s.kind == SurfaceFeatureKind::InteriorFloor && surface_contains_point_m(s, wx, wy)
            }) {
                return false;
            }
        }
        true
    }

    /// Attempts continuous `1°` prop placement in integer centimeters with edge/prop/spawn/road guards.
    pub(crate) fn try_place_cm(
        &mut self,
        kind: PropKind,
        cx_cm: i32,
        cy_cm: i32,
        rot_deg: i32,
        length_cm: Option<i32>,
        allow_on_pavement: bool,
    ) -> bool {
        let seed = (self.rng.next() & 0xffff_ffff) as u32;
        let rot = normalize_prop_rotation_deg(kind, rot_deg);
        let cand = make_prop_at_cm(kind, cx_cm, cy_cm, rot, length_cm, seed);
        let fp = cand.footprint_m();
        let (hw_cm, hh_cm) = crate::map_dressing::rotated_aabb_half_extents_cm(
            (fp.span_w_m * 100.0) as i32,
            (fp.span_h_m * 100.0) as i32,
            rot,
        );
        let (hw, hh) = (hw_cm / 100.0, hh_cm / 100.0);
        if fp.cx_m - hw < 0.05
            || fp.cy_m - hh < 0.05
            || fp.cx_m + hw > self.w as f32 - 0.05
            || fp.cy_m + hh > self.h as f32 - 0.05
        {
            return false;
        }
        let spec = prop_spec(kind);
        if !spec.floor_layer {
            if prop_crosses_wall_edge(&cand, &self.edges) {
                return false;
            }
            for &(sx, sy) in &self.spawn_pads_m {
                if prop_intersects_circle(&cand, sx, sy, TOKEN_RADIUS_M + 0.55) {
                    return false;
                }
            }
            for existing in &self.props {
                if !prop_spec(existing.kind).floor_layer && props_overlap_continuous(&cand, existing) {
                    return false;
                }
            }
        }
        if !self.prop_clears_surfaces(&cand, kind, allow_on_pavement) {
            return false;
        }
        self.props.push(cand);
        true
    }

    pub(crate) fn scatter_trees(&mut self, kind: PropKind, count: i32, min_dist_m: f32, bounds: RectM) {
        let mut placed = 0;
        let mut tries = 0;
        while placed < count && tries < count * 28 {
            tries += 1;
            let cx_m = self.rng.f32_range(bounds.x0, bounds.x1);
            let cy_m = self.rng.f32_range(bounds.y0, bounds.y1);
            if self
                .tree_centers_m
                .iter()
                .any(|&(tx, ty)| (tx - cx_m).hypot(ty - cy_m) < min_dist_m)
            {
                continue;
            }
            let rot = self.rng.range(0, 360);
            if self.try_place_cm(kind, Self::cm(cx_m), Self::cm(cy_m), rot, None, false) {
                self.tree_centers_m.push((cx_m, cy_m));
                let radius = if kind == PropKind::LargeTree {
                    self.rng.f32_range(1.15, 1.55)
                } else {
                    self.rng.f32_range(0.85, 1.25)
                };
                let ry = radius * self.rng.f32_range(0.82, 1.18);
                let _ = self.try_add_organic_patch(
                    SurfaceFeatureKind::MulchZone,
                    (cx_m, cy_m),
                    (radius, ry),
                    0.35,
                    0,
                );
                placed += 1;
            }
        }
    }

    /// Scatters natural/turf props across `bounds` with continuous `rot_step_deg` resolution (supports `1°`).
    pub(crate) fn scatter_turf_prop_rot(
        &mut self,
        kind: PropKind,
        count: i32,
        rot_step_deg: i32,
        bounds: RectM,
    ) {
        let step = rot_step_deg.max(1);
        let buckets = (360 / step).max(1);
        let mut placed = 0;
        let mut tries = 0;
        while placed < count && tries < count * 22 {
            tries += 1;
            let cx_m = self.rng.f32_range(bounds.x0, bounds.x1);
            let cy_m = self.rng.f32_range(bounds.y0, bounds.y1);
            let rot = (self.rng.range(0, buckets) * step) % 360;
            if self.try_place_cm(kind, Self::cm(cx_m), Self::cm(cy_m), rot, None, false) {
                placed += 1;
            }
        }
    }

    /// Places a tangent-aligned vehicle (`Car` or `UtilityVan`) along `ribbon_idx` shoulder/lane.
    pub(crate) fn place_vehicle_along_ribbon(
        &mut self,
        kind: PropKind,
        ribbon_idx: usize,
        frac: f32,
        lateral_offset_m: f32,
        jitter_deg: i32,
    ) -> bool {
        let Some(surf) = self.surfaces.get(ribbon_idx).cloned() else {
            return false;
        };
        let Some((pos, (tx, ty))) = ribbon_point_and_tangent_at_fraction(&surf, frac) else {
            return false;
        };
        let (nx, ny) = (-ty, tx);
        let cx_m = pos.0 + nx * lateral_offset_m;
        let cy_m = pos.1 + ny * lateral_offset_m;
        let base_rot = vehicle_forward_rot_deg(tx, ty);
        let rot = (base_rot + self.rng.range(-jitter_deg, jitter_deg + 1)).rem_euclid(360);
        self.try_place_cm(kind, Self::cm(cx_m), Self::cm(cy_m), rot, None, true)
    }

    /// Places a bench or barrier aligned parallel to `ribbon_idx` tangent + subtle natural offset.
    pub(crate) fn place_along_ribbon(
        &mut self,
        kind: PropKind,
        ribbon_idx: usize,
        frac: f32,
        lateral_offset_m: f32,
        jitter_deg: i32,
        allow_on_pavement: bool,
    ) -> bool {
        let Some(surf) = self.surfaces.get(ribbon_idx).cloned() else {
            return false;
        };
        let Some((pos, (tx, ty))) = ribbon_point_and_tangent_at_fraction(&surf, frac) else {
            return false;
        };
        let (nx, ny) = (-ty, tx);
        let cx_m = pos.0 + nx * lateral_offset_m;
        let cy_m = pos.1 + ny * lateral_offset_m;
        let base_rot = along_tangent_rot_deg(tx, ty);
        let rot = (base_rot + self.rng.range(-jitter_deg, jitter_deg + 1)).rem_euclid(360);
        self.try_place_cm(kind, Self::cm(cx_m), Self::cm(cy_m), rot, None, allow_on_pavement)
    }
}

/// Generates an outdoor tactical [`ProceduralSector`] with smooth vector terrain surfaces and `360°` props.
#[must_use]
pub fn generate_outdoor_sector(
    style: OutdoorStyle,
    width: i32,
    height: i32,
    seed: i64,
) -> ProceduralSector {
    let w = width.clamp(8, 48);
    let h = height.clamp(8, 48);
    let mut b = OutdoorBuilder::new(w, h, seed);

    match style {
        OutdoorStyle::WoodedPark => build_wooded_park(&mut b),
        OutdoorStyle::GardenEstate => build_garden_estate(&mut b),
        OutdoorStyle::WoodlandCreek => build_woodland_creek(&mut b),
        OutdoorStyle::RoadsideDitch => build_roadside_ditch(&mut b),
    }

    let mut props = b.props;
    loop {
        if all_walkable_areas_connected(w, h, &b.edges, &props) {
            return ProceduralSector {
                wall_edges: b.edges,
                props,
                surfaces: b.surfaces,
            };
        }
        match props
            .iter()
            .rposition(|p| prop_spec(p.kind).obstruction == PropObstruction::FullObstacle)
        {
            Some(i) => {
                props.remove(i);
            }
            None => {
                return ProceduralSector {
                    wall_edges: b.edges,
                    props,
                    surfaces: b.surfaces,
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_surface_geom::topmost_surface_at_m;

    #[test]
    fn outdoor_sectors_emit_smooth_surfaces_and_continuous_non45_rotations() {
        for style in [
            OutdoorStyle::WoodedPark,
            OutdoorStyle::GardenEstate,
            OutdoorStyle::WoodlandCreek,
            OutdoorStyle::RoadsideDitch,
        ] {
            for seed in [7_i64, 42, 108] {
                let sec = generate_outdoor_sector(style, 20, 14, seed);
                assert_eq!(sec, generate_outdoor_sector(style, 20, 14, seed));
                assert!(!sec.surfaces.is_empty(), "{style:?}/{seed} missing vector surfaces");
                assert!(
                    sec.surfaces.iter().any(|s| s.shape == SurfaceShapeKind::RibbonSpline),
                    "{style:?}/{seed} should include smooth RibbonSpline terrain"
                );
                assert!(all_walkable_areas_connected(20, 14, &sec.wall_edges, &sec.props));
                // Old floor strip grid props (`CreekWater`, `MulchBed`, etc.) must not be emitted by outdoor gen.
                assert!(
                    !sec.props.iter().any(|p| matches!(
                        p.kind,
                        PropKind::CreekWater
                            | PropKind::DrainageDitch
                            | PropKind::MulchBed
                            | PropKind::DirtPath
                            | PropKind::GravelPath
                            | PropKind::StonePath
                            | PropKind::AsphaltDrive
                            | PropKind::GardenBed
                            | PropKind::GardenEdging
                    )),
                    "{style:?}/{seed} emitted legacy stair-step surface prop"
                );
                // Continuous 1° orientations: not all props are multiples of 45°.
                assert!(
                    sec.props.iter().any(|p| p.rotation_deg % 45 != 0),
                    "{style:?}/{seed} had all props snapped to 45° multiples"
                );
            }
        }
    }

    #[test]
    fn roadside_ditch_keeps_asphalt_road_free_of_mulch_and_garden_beds() {
        for seed in 1..=12 {
            let sec = generate_outdoor_sector(OutdoorStyle::RoadsideDitch, 20, 14, seed);
            let road_surfaces: Vec<&MapSurfaceFeature> = sec
                .surfaces
                .iter()
                .filter(|s| s.kind == SurfaceFeatureKind::AsphaltRoad)
                .collect();
            assert!(!road_surfaces.is_empty(), "seed {seed} missing AsphaltRoad");

            // Sample every 0.5m along the board: any point inside AsphaltRoad must resolve topmost
            // to AsphaltRoad or BridgeDeck, NEVER MulchZone or GardenBedZone.
            for y in 1..27 {
                for x in 1..39 {
                    let px = x as f32 * 0.5;
                    let py = y as f32 * 0.5;
                    if road_surfaces
                        .iter()
                        .any(|r| crate::map_surface_geom::surface_contains_point_m(r, px, py))
                    {
                        let top = topmost_surface_at_m(&sec.surfaces, px, py).unwrap();
                        assert!(
                            matches!(
                                top.kind,
                                SurfaceFeatureKind::AsphaltRoad | SurfaceFeatureKind::BridgeDeck
                            ),
                            "seed {seed}: unexpected {:?} atop road at ({px}, {py})",
                            top.kind
                        );
                    }
                }
            }
            // No tree or boulder sits on an asphalt surface.
            for p in &sec.props {
                if matches!(
                    p.kind,
                    PropKind::LargeTree | PropKind::MediumTree | PropKind::Boulder | PropKind::FallenLog
                ) {
                    let (px, py) = p.center_m();
                    assert!(
                        topmost_surface_at_m(&sec.surfaces, px, py)
                            .is_none_or(|s| !s.kind.is_road_or_pavement()),
                        "seed {seed}: natural prop {:?} landed on road/pavement at ({px}, {py})",
                        p.kind
                    );
                }
            }
        }
    }
}
