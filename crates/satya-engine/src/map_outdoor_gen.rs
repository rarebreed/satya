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
use crate::map_estate_grammars::build_garden_estate;
use crate::map_outdoor_natural::{build_roadside_ditch, build_wooded_park, build_woodland_creek};
use crate::map_surface_geom::{
    along_tangent_rot_deg, point_near_protected_surface_m, ribbon_macro_chord_at_fraction,
    ribbon_point_and_tangent_at_fraction, surface_contains_point_m, topmost_surface_at_m,
    vehicle_forward_rot_deg,
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct BridgeCrossingSpec {
    pub(crate) center_m: (f32, f32),
    #[allow(dead_code)]
    pub(crate) tangent_dir: (f32, f32),
    pub(crate) span_dir: (f32, f32),
    #[allow(dead_code)]
    pub(crate) half_width_m: f32,
    pub(crate) half_span_m: f32,
    pub(crate) abutment_neg_m: (f32, f32),
    pub(crate) abutment_pos_m: (f32, f32),
}

struct BankScanResult {
    reach_m: f32,
    penalty: f32,
}

fn scan_bridge_bank_reach(
    surfaces: &[MapSurfaceFeature],
    pos: (f32, f32),
    axis: (f32, f32),
    dir_sign: f32,
    max_scan_m: f32,
    board_m: (f32, f32),
) -> BankScanResult {
    let (bw, bh) = board_m;
    let steps = (max_scan_m / 0.20).ceil().max(1.0) as usize;
    let mut last_water_d = 0.0_f32;
    let mut dry_run_m = 0.0_f32;
    let mut penalty = 0.0_f32;
    let mut reached_dry = false;

    for s in 1..=steps {
        let d = s as f32 * 0.20;
        let px = pos.0 + axis.0 * dir_sign * d;
        let py = pos.1 + axis.1 * dir_sign * d;
        let in_soft_bounds = px >= 0.55 && px <= bw - 0.55 && py >= 0.55 && py <= bh - 0.55;
        let hits_building = surfaces
            .iter()
            .any(|surf| surf.kind.is_built_floor_or_deck() && surface_contains_point_m(surf, px, py));
        let in_water = surfaces
            .iter()
            .any(|surf| surf.kind.is_water_or_channel() && surface_contains_point_m(surf, px, py));

        if in_water {
            if !reached_dry {
                last_water_d = d;
                dry_run_m = 0.0;
            } else {
                penalty += 14.0;
                break;
            }
        } else {
            if (!in_soft_bounds || hits_building) && dry_run_m < 0.80 {
                penalty += 25.0;
            }
            dry_run_m += 0.20;
            if dry_run_m >= 0.80 {
                reached_dry = true;
            }
            if dry_run_m >= 2.00 {
                break;
            }
        }
    }

    if !reached_dry {
        penalty += 12.0;
    }
    BankScanResult {
        reach_m: last_water_d,
        penalty,
    }
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
    pub(crate) fn new(w: i32, h: i32, seed: i64) -> Self {
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
        let edge = MapWallEdge {
            x,
            y,
            side,
            kind,
            material,
        };
        self.edge_idx.set(x, y, side, Some(kind));
        match self
            .edges
            .iter_mut()
            .find(|o| o.x == x && o.y == y && o.side == side)
        {
            Some(existing) => *existing = edge,
            None => self.edges.push(edge),
        }
    }

    /// Removes any structural edge at `(x, y, side)` (open compound gate or breezeway).
    pub(crate) fn remove_edge(&mut self, x: i32, y: i32, side: EdgeSide) {
        self.edge_idx.set(x, y, side, None);
        self.edges
            .retain(|o| !(o.x == x && o.y == y && o.side == side));
    }

    /// Appends a smooth Catmull-Rom [`SurfaceShapeKind::RibbonSpline`] with an RNG-derived seed.
    pub(crate) fn add_ribbon(
        &mut self,
        kind: SurfaceFeatureKind,
        ctrl_m: &[(f32, f32)],
        width_cm: i32,
    ) -> usize {
        let s = (self.rng.next() & 0xffff_ffff) as u32;
        self.add_ribbon_spline(kind, ctrl_m, width_cm, s)
    }

    /// Appends a corner-rounded polygon [`SurfaceFeatureKind`] with an RNG-derived seed.
    pub(crate) fn add_poly(
        &mut self,
        kind: SurfaceFeatureKind,
        poly_m: &[(f32, f32)],
        corner_radius_cm: i32,
    ) -> usize {
        let s = (self.rng.next() & 0xffff_ffff) as u32;
        self.add_polygon_surface(kind, poly_m, corner_radius_cm, s)
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
            tint_rgb: None,
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
            tint_rgb: None,
        });
        idx
    }

    /// Places an elevated [`SurfaceFeatureKind::BridgeDeck`] spanning perpendicularly across
    /// `channel_idx` near `preferred_frac`, selecting a low-curvature reach and centering across
    /// water/ravine banks so guardrails span the water and open ends meet dry abutments.
    pub(crate) fn place_perpendicular_bridge(
        &mut self,
        channel_idx: usize,
        preferred_frac: f32,
        bridge_width_m: f32,
        min_span_m: f32,
    ) -> Option<BridgeCrossingSpec> {
        let ch = self.surfaces.get(channel_idx)?.clone();
        let half_win_m = (bridge_width_m * 0.75)
            .max(ch.width_cm as f32 / 150.0)
            .max(1.4);
        let (wf, hf) = (self.w as f32, self.h as f32);
        let hw = bridge_width_m * 0.5;
        let max_scan_m = min_span_m * 0.5 + 3.4;

        struct Candidate {
            score: f32,
            center: (f32, f32),
            tx: f32,
            ty: f32,
            sx: f32,
            sy: f32,
            hs: f32,
        }
        let mut best: Option<Candidate> = None;

        for i in 0..15 {
            let cand_frac = (preferred_frac - 0.32 + (i as f32 / 14.0) * 0.64).clamp(0.16, 0.84);
            let Some((pos, (tx, ty), sagitta_m)) =
                ribbon_macro_chord_at_fraction(&ch, cand_frac, half_win_m)
            else {
                continue;
            };
            let (sx, sy) = (-ty, tx);
            let scan_neg = scan_bridge_bank_reach(
                &self.surfaces,
                pos,
                (sx, sy),
                -1.0,
                max_scan_m,
                (wf, hf),
            );
            let scan_pos = scan_bridge_bank_reach(
                &self.surfaces,
                pos,
                (sx, sy),
                1.0,
                max_scan_m,
                (wf, hf),
            );
            let reach_cap = min_span_m * 0.5 + 0.65;
            let shift_s = (0.5 * (scan_pos.reach_m - scan_neg.reach_m)).clamp(-0.55, 0.55);
            let center = (pos.0 + sx * shift_s, pos.1 + sy * shift_s);
            let hs_desired = (0.5
                * (scan_neg.reach_m.min(reach_cap) + scan_pos.reach_m.min(reach_cap))
                + 0.60)
            .max(min_span_m * 0.5);
            let x_slack = (wf - 0.30 - center.0 - (tx * hw).abs())
                .min(center.0 - 0.30 - (tx * hw).abs());
            let y_slack = (hf - 0.30 - center.1 - (ty * hw).abs())
                .min(center.1 - 0.30 - (ty * hw).abs());
            let hs_cap_x = if sx.abs() > 1e-4 {
                (x_slack / sx.abs()).max(0.75)
            } else {
                hs_desired
            };
            let hs_cap_y = if sy.abs() > 1e-4 {
                (y_slack / sy.abs()).max(0.75)
            } else {
                hs_desired
            };
            let hs_cap = hs_cap_x.min(hs_cap_y);
            let hs = hs_desired.min(hs_cap);

            let mut hazard_penalty = scan_neg.penalty + scan_pos.penalty;
            if hs_desired > hs_cap {
                hazard_penalty += (hs_desired - hs_cap) * 30.0;
            }
            if x_slack < 0.0 || y_slack < 0.0 {
                hazard_penalty += 25.0;
            }
            let abut_neg = (center.0 - sx * hs, center.1 - sy * hs);
            let abut_pos = (center.0 + sx * hs, center.1 + sy * hs);
            for (ax, ay) in [abut_neg, abut_pos] {
                if ax < 0.35 || ax > wf - 0.35 || ay < 0.35 || ay > hf - 0.35 {
                    hazard_penalty += 20.0;
                }
                if self
                    .surfaces
                    .iter()
                    .any(|s| s.kind.is_built_floor_or_deck() && surface_contains_point_m(s, ax, ay))
                {
                    hazard_penalty += 30.0;
                }
                if self
                    .surfaces
                    .iter()
                    .any(|s| s.kind.is_water_or_channel() && surface_contains_point_m(s, ax, ay))
                {
                    hazard_penalty += 45.0;
                }
            }

            let score =
                sagitta_m * 3.5 + (cand_frac - preferred_frac).abs() * 0.9 + hazard_penalty;
            if best.as_ref().is_none_or(|b| score < b.score) {
                best = Some(Candidate {
                    score,
                    center,
                    tx,
                    ty,
                    sx,
                    sy,
                    hs,
                });
            }
        }

        let cand = best?;
        let (cx, cy) = cand.center;
        let (tx, ty, sx, sy, hs) = (cand.tx, cand.ty, cand.sx, cand.sy, cand.hs);
        // CCW corners with guardrails along [0]->[1] and [2]->[3] spanning across water,
        // leaving [1]->[2] and [3]->[0] open at the dry banks.
        let corners = [
            (cx + tx * hw - sx * hs, cy + ty * hw - sy * hs),
            (cx + tx * hw + sx * hs, cy + ty * hw + sy * hs),
            (cx - tx * hw + sx * hs, cy - ty * hw + sy * hs),
            (cx - tx * hw - sx * hs, cy - ty * hw - sy * hs),
        ];
        let seed = (self.rng.next() & 0xffff_ffff) as u32;
        self.add_polygon_surface(SurfaceFeatureKind::BridgeDeck, &corners, 14, seed);
        Some(BridgeCrossingSpec {
            center_m: cand.center,
            tangent_dir: (tx, ty),
            span_dir: (sx, sy),
            half_width_m: hw,
            half_span_m: hs,
            abutment_neg_m: (cx - sx * hs, cy - sy * hs),
            abutment_pos_m: (cx + sx * hs, cy + sy * hs),
        })
    }

    /// Appends an elevated [`SurfaceFeatureKind::BridgeDeck`] spanning perpendicularly across `channel_idx`
    /// near normalized arc fraction `frac`.
    pub(crate) fn add_bridge_deck_across(
        &mut self,
        channel_idx: usize,
        frac: f32,
        bridge_width_m: f32,
        bridge_span_m: f32,
    ) -> bool {
        self.place_perpendicular_bridge(channel_idx, frac, bridge_width_m, bridge_span_m)
            .is_some()
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
                    let ang = base_rot
                        + TAU * (i as f32 / n_pts as f32)
                        + self.rng.f32_range(-0.14, 0.14);
                    let wob = self.rng.f32_range(0.76, 1.18);
                    let px =
                        (cx_m + rx_m * scale * wob * ang.cos()).clamp(0.35, self.w as f32 - 0.35);
                    let py =
                        (cy_m + ry_m * scale * wob * ang.sin()).clamp(0.35, self.h as f32 - 0.35);
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
                    tint_rgb: None,
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
            (s.kind.is_built_floor_or_deck()
                || matches!(
                    s.kind,
                    SurfaceFeatureKind::MulchZone | SurfaceFeatureKind::GardenBedZone
                ))
                && surface_contains_point_m(s, px, py)
        })
    }

    fn prop_clears_surfaces(
        &self,
        prop: &MapPropPlacement,
        kind: PropKind,
        allow_on_pavement: bool,
    ) -> bool {
        if allow_on_pavement {
            return true;
        }
        let fp = prop.footprint_m();
        let hx = fp.span_w_m * 0.45;
        let hy = fp.span_h_m * 0.45;
        let pad = if matches!(
            kind,
            PropKind::GardenBed | PropKind::LargeTree | PropKind::MediumTree
        ) {
            0.35
        } else {
            0.22
        };
        for &(ox, oy) in &[(0.0, 0.0), (-hx, -hy), (hx, -hy), (-hx, hy), (hx, hy)] {
            let (wx, wy) = fp.local_offset_to_world(ox, oy);
            if point_near_protected_surface_m(&self.surfaces, wx, wy, pad) {
                return false;
            }
            if self
                .surfaces
                .iter()
                .any(|s| s.kind.is_built_floor_or_deck() && surface_contains_point_m(s, wx, wy))
            {
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
                if !prop_spec(existing.kind).floor_layer
                    && props_overlap_continuous(&cand, existing)
                {
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

    pub(crate) fn scatter_trees(
        &mut self,
        kind: PropKind,
        count: i32,
        min_dist_m: f32,
        bounds: RectM,
    ) {
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
        self.try_place_cm(
            kind,
            Self::cm(cx_m),
            Self::cm(cy_m),
            rot,
            None,
            allow_on_pavement,
        )
    }
    /// Places a street/plaza prop on pavement or turf while refusing asphalt, water channels, or interiors.
    pub(crate) fn try_place_off_road_cm(
        &mut self,
        kind: PropKind,
        cx_cm: i32,
        cy_cm: i32,
        rot_deg: i32,
        length_cm: Option<i32>,
    ) -> bool {
        let cx_m = cx_cm as f32 / 100.0;
        let cy_m = cy_cm as f32 / 100.0;
        if let Some(top) = topmost_surface_at_m(&self.surfaces, cx_m, cy_m)
            && (matches!(
                top.kind,
                SurfaceFeatureKind::AsphaltRoad | SurfaceFeatureKind::GravelRoad
            ) || top.kind.is_built_floor_or_deck()
                || top.kind.is_water_or_channel())
        {
            return false;
        }
        self.try_place_cm(kind, cx_cm, cy_cm, rot_deg, length_cm, true)
    }

    /// Prunes completely occluded surfaces and trims excess blocking props until 100% connected.
    pub(crate) fn finalize_connected(mut self) -> ProceduralSector {
        crate::map_surface_queries::prune_occluded_surfaces(&mut self.surfaces, self.w, self.h);
        loop {
            if all_walkable_areas_connected(self.w, self.h, &self.edges, &self.props) {
                return ProceduralSector {
                    wall_edges: self.edges,
                    props: self.props,
                    surfaces: self.surfaces,
                    floors: Vec::new(),
                    active_floor: 0,
                };
            }
            match self
                .props
                .iter()
                .rposition(|p| prop_spec(p.kind).obstruction == PropObstruction::FullObstacle)
            {
                Some(i) => {
                    self.props.remove(i);
                }
                None => {
                    return ProceduralSector {
                        wall_edges: self.edges,
                        props: self.props,
                        surfaces: self.surfaces,
                        floors: Vec::new(),
                        active_floor: 0,
                    };
                }
            }
        }
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
    let w = width.clamp(
        crate::tactical_map::MIN_SECTOR_DIM,
        crate::tactical_map::MAX_SECTOR_DIM,
    );
    let h = height.clamp(
        crate::tactical_map::MIN_SECTOR_DIM,
        crate::tactical_map::MAX_SECTOR_DIM,
    );
    let mut b = OutdoorBuilder::new(w, h, seed);

    match style {
        OutdoorStyle::WoodedPark => build_wooded_park(&mut b),
        OutdoorStyle::GardenEstate => build_garden_estate(&mut b),
        OutdoorStyle::WoodlandCreek => build_woodland_creek(&mut b),
        OutdoorStyle::RoadsideDitch => build_roadside_ditch(&mut b),
    }

    b.finalize_connected()
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
                assert!(
                    !sec.surfaces.is_empty(),
                    "{style:?}/{seed} missing vector surfaces"
                );
                assert!(
                    sec.surfaces
                        .iter()
                        .any(|s| s.shape == SurfaceShapeKind::RibbonSpline),
                    "{style:?}/{seed} should include smooth RibbonSpline terrain"
                );
                assert!(all_walkable_areas_connected(
                    20,
                    14,
                    &sec.wall_edges,
                    &sec.props
                ));
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
                    PropKind::LargeTree
                        | PropKind::MediumTree
                        | PropKind::Boulder
                        | PropKind::FallenLog
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

    #[test]
    fn woodland_creek_exposes_topmost_creek_water_and_ravine_across_seeds_and_scales() {
        for &(w, h) in &[(20, 14), (36, 26), (48, 36)] {
            for seed in 1..=12 {
                let sec = generate_outdoor_sector(OutdoorStyle::WoodlandCreek, w, h, seed);
                let mut creek_hits = 0usize;
                let mut ravine_hits = 0usize;
                let mut bridge_hits = 0usize;
                for gy in 1..(h * 2) {
                    for gx in 1..(w * 2) {
                        let px = gx as f32 * 0.5;
                        let py = gy as f32 * 0.5;
                        if let Some(top) = topmost_surface_at_m(&sec.surfaces, px, py) {
                            match top.kind {
                                SurfaceFeatureKind::CreekChannel => creek_hits += 1,
                                SurfaceFeatureKind::DefensiveTrench => ravine_hits += 1,
                                SurfaceFeatureKind::BridgeDeck => bridge_hits += 1,
                                _ => {}
                            }
                        }
                    }
                }
                assert!(
                    creek_hits >= 15,
                    "{w}x{h} seed {seed}: expected >= 15 exposed CreekChannel samples, got {creek_hits}"
                );
                assert!(
                    ravine_hits >= 15,
                    "{w}x{h} seed {seed}: expected >= 15 exposed DefensiveTrench ravine samples, got {ravine_hits}"
                );
                assert!(
                    bridge_hits >= 4,
                    "{w}x{h} seed {seed}: expected >= 4 exposed BridgeDeck samples, got {bridge_hits}"
                );
                assert!(
                    all_walkable_areas_connected(w, h, &sec.wall_edges, &sec.props),
                    "{w}x{h} seed {seed}: WoodlandCreek disconnected"
                );
            }
        }
    }

    #[test]
    fn fractal_ribbon_edges_modulate_natural_paths_while_keeping_asphalt_crisp() {
        use crate::map_surface_geom::extrude_surface_ribbon_m;

        let make_line = |kind: SurfaceFeatureKind, width_cm: i32| MapSurfaceFeature {
            kind,
            shape: SurfaceShapeKind::RibbonSpline,
            points_cm: vec![MapPointCm::new(200, 700), MapPointCm::new(1800, 700)],
            width_cm,
            border_style_cm: 0,
            variant_seed: 1337,
            tint_rgb: None,
        };

        for (kind, width_cm) in [
            (SurfaceFeatureKind::CreekChannel, 260),
            (SurfaceFeatureKind::DirtTrail, 180),
        ] {
            let feat = make_line(kind, width_cm);
            let mesh = extrude_surface_ribbon_m(&feat, 20.0, 14.0);
            assert!(
                mesh.centerline.len() > 20,
                "{kind:?} expected resampled centerline (>20 pts), got {}",
                mesh.centerline.len()
            );
            let mut min_w = f32::INFINITY;
            let mut max_w = 0.0_f32;
            let mut asym_hits = 0usize;
            for i in 0..mesh.centerline.len() {
                let lw = (mesh.left[i].1 - mesh.centerline[i].1).abs();
                let rw = (mesh.centerline[i].1 - mesh.right[i].1).abs();
                let total_w = lw + rw;
                min_w = min_w.min(total_w);
                max_w = max_w.max(total_w);
                if (lw - rw).abs() > 0.02 {
                    asym_hits += 1;
                }
            }
            assert!(
                max_w - min_w > 0.15,
                "{kind:?} expected organic fractal edge modulation (max_w={max_w}, min_w={min_w})"
            );
            assert!(
                asym_hits > 5,
                "{kind:?} expected independent left/right bank widths"
            );

            // Containment agrees with modulated edge offsets across interior points.
            let mid_x = mesh.centerline[mesh.centerline.len() / 2].0;
            let mid_y_top = mesh.left[mesh.centerline.len() / 2].1;
            let mid_y_bot = mesh.right[mesh.centerline.len() / 2].1;
            assert!(surface_contains_point_m(&feat, mid_x, mid_y_top - 0.05));
            assert!(!surface_contains_point_m(&feat, mid_x, mid_y_top + 0.05));
            assert!(surface_contains_point_m(&feat, mid_x, mid_y_bot + 0.05));
            assert!(!surface_contains_point_m(&feat, mid_x, mid_y_bot - 0.05));
        }

        let road = make_line(SurfaceFeatureKind::AsphaltRoad, 340);
        let road_mesh = extrude_surface_ribbon_m(&road, 20.0, 14.0);
        for i in 0..road_mesh.centerline.len() {
            let total_w = (road_mesh.left[i].1 - road_mesh.right[i].1).abs();
            assert!(
                (total_w - 3.40).abs() < 1e-3,
                "AsphaltRoad width must remain crisp at 3.40m, got {total_w}"
            );
        }
    }

    fn max_turn_angle_within_arc_m(pts: &[(f32, f32)], max_arc_m: f32) -> f32 {
        if pts.len() < 3 {
            return 0.0;
        }
        let mut cum = Vec::with_capacity(pts.len());
        let mut angles = Vec::with_capacity(pts.len() - 1);
        cum.push(0.0_f32);
        for w in pts.windows(2) {
            let dx = w[1].0 - w[0].0;
            let dy = w[1].1 - w[0].1;
            let len = dx.hypot(dy);
            cum.push(cum[cum.len() - 1] + len);
            angles.push(dy.atan2(dx));
        }
        let mut best_turn = 0.0_f32;
        for i in 0..angles.len() {
            let mut accum_turn = 0.0_f32;
            for j in i + 1..angles.len() {
                if cum[j + 1] - cum[i] > max_arc_m {
                    break;
                }
                let mut d = angles[j] - angles[j - 1];
                while d > std::f32::consts::PI {
                    d -= TAU;
                }
                while d < -std::f32::consts::PI {
                    d += TAU;
                }
                accum_turn += d;
                best_turn = best_turn.max(accum_turn.abs());
            }
        }
        best_turn
    }

    #[test]
    fn nature_curves_vary_shapes_on_20x14_and_include_rare_tight_bends() {
        use crate::map_surface_geom::sample_open_catmull_rom_m;
        use std::collections::HashSet;

        let mut tight_bends = 0usize;
        let mut total_curves = 0usize;

        for style in [OutdoorStyle::WoodlandCreek, OutdoorStyle::WoodedPark] {
            let target_kind = if style == OutdoorStyle::WoodlandCreek {
                SurfaceFeatureKind::CreekChannel
            } else {
                SurfaceFeatureKind::DirtTrail
            };
            let mut sigs_20x14: HashSet<u64> = HashSet::new();
            let mut pos_deflect = 0usize;
            let mut neg_deflect = 0usize;

            for &(w, h) in &[(20, 14), (36, 26)] {
                for seed in 1_i64..=60 {
                    let sec = generate_outdoor_sector(style, w, h, seed);
                    assert!(
                        all_walkable_areas_connected(w, h, &sec.wall_edges, &sec.props),
                        "{style:?} {w}x{h} seed {seed} disconnected"
                    );
                    let primary = sec
                        .surfaces
                        .iter()
                        .find(|s| {
                            s.kind == target_kind
                                && s.shape == SurfaceShapeKind::RibbonSpline
                                && s.points_cm.len() >= 3
                        })
                        .expect("expected multi-knot primary nature ribbon");

                    if (w, h) == (20, 14) {
                        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
                        for pt in &primary.points_cm {
                            hash ^= (pt.x_cm / 40) as u64;
                            hash = hash.wrapping_mul(0x0100_0000_01b3);
                            hash ^= (pt.y_cm / 40) as u64;
                            hash = hash.wrapping_mul(0x0100_0000_01b3);
                        }
                        sigs_20x14.insert(hash);
                    }

                    let a = primary.points_cm[0].to_m();
                    let b_pt = primary.points_cm[primary.points_cm.len() - 1].to_m();
                    let m = primary.points_cm[primary.points_cm.len() / 3].to_m();
                    let chord_len = (b_pt.0 - a.0).hypot(b_pt.1 - a.1).max(1.0);
                    let cross =
                        ((m.0 - a.0) * (b_pt.1 - a.1) - (m.1 - a.1) * (b_pt.0 - a.0)) / chord_len;
                    if cross > 0.30 {
                        pos_deflect += 1;
                    } else if cross < -0.30 {
                        neg_deflect += 1;
                    }

                    for s in &sec.surfaces {
                        if s.shape == SurfaceShapeKind::RibbonSpline
                            && matches!(
                                s.kind,
                                SurfaceFeatureKind::CreekChannel | SurfaceFeatureKind::DirtTrail
                            )
                            && s.points_cm.len() >= 6
                        {
                            let raw: Vec<(f32, f32)> =
                                s.points_cm.iter().map(|p| p.to_m()).collect();
                            let cl = sample_open_catmull_rom_m(&raw, 10);
                            total_curves += 1;
                            if max_turn_angle_within_arc_m(&cl, 4.2) > 95.0_f32.to_radians() {
                                tight_bends += 1;
                            }
                        }
                    }
                }
            }
            assert!(
                sigs_20x14.len() >= 8,
                "{style:?} 20x14 expected >= 8 distinct curve shapes, got {}",
                sigs_20x14.len()
            );
            assert!(
                pos_deflect >= 5 && neg_deflect >= 5,
                "{style:?} expected both left/right curve deflections (pos={pos_deflect}, neg={neg_deflect})"
            );
        }

        assert!(
            tight_bends >= 1,
            "expected >= 1 rare tight bend (>95 deg in <=4.2m) across 60 seeds"
        );
        assert!(
            tight_bends * 100 / total_curves.max(1) <= 25,
            "tight bends should remain rare (got {tight_bends}/{total_curves})"
        );
    }

    #[test]
    fn bridges_cross_channels_perpendicularly_with_rails_over_water_and_dry_abutments() {
        use crate::map_surface_geom::sample_open_catmull_rom_m;
        use crate::tactical_map::{SectorPreset, generate_procedural_sector};

        let mut bridges_checked = 0usize;

        for preset in [
            SectorPreset::WoodlandCreek,
            SectorPreset::RoadsideDitch,
            SectorPreset::BotanicalGarden,
            SectorPreset::Farms,
            SectorPreset::RuralCountryside,
        ] {
            for &(w, h) in &[(20, 14), (36, 26)] {
                for seed in 1_i64..=16 {
                    let sec = generate_procedural_sector(preset, w, h, seed);
                    let channels: Vec<&MapSurfaceFeature> = sec
                        .surfaces
                        .iter()
                        .filter(|s| {
                            s.shape == SurfaceShapeKind::RibbonSpline
                                && matches!(
                                    s.kind,
                                    SurfaceFeatureKind::CreekChannel
                                        | SurfaceFeatureKind::DrainageDitch
                                )
                        })
                        .collect();

                    for br in sec
                        .surfaces
                        .iter()
                        .filter(|s| s.kind == SurfaceFeatureKind::BridgeDeck)
                    {
                        assert_eq!(br.points_cm.len(), 4, "BridgeDeck must have 4 corners");
                        let p = [
                            br.points_cm[0].to_m(),
                            br.points_cm[1].to_m(),
                            br.points_cm[2].to_m(),
                            br.points_cm[3].to_m(),
                        ];
                        let center = (
                            0.25 * (p[0].0 + p[1].0 + p[2].0 + p[3].0),
                            0.25 * (p[0].1 + p[1].1 + p[2].1 + p[3].1),
                        );

                        // Locate underlying channel and arc fraction under bridge center.
                        let mut best_chan: Option<(&MapSurfaceFeature, bool, u8, f32, f32)> = None;
                        for &ch in &channels {
                            let raw: Vec<(f32, f32)> =
                                ch.points_cm.iter().map(|pt| pt.to_m()).collect();
                            let cl = sample_open_catmull_rom_m(&raw, 10);
                            if cl.len() < 2 {
                                continue;
                            }
                            let mut cum = Vec::with_capacity(cl.len());
                            cum.push(0.0_f32);
                            for seg in cl.windows(2) {
                                cum.push(
                                    cum[cum.len() - 1]
                                        + (seg[1].0 - seg[0].0).hypot(seg[1].1 - seg[0].1),
                                );
                            }
                            let total = (*cum.last().unwrap_or(&0.0)).max(1e-4);
                            let mut min_d = f32::INFINITY;
                            let mut min_s = 0.0_f32;
                            for i in 0..cl.len() - 1 {
                                let (d, t) = crate::tactical_map_geom::distance_point_to_segment(
                                    center.0, center.1, cl[i], cl[i + 1],
                                );
                                if d < min_d {
                                    min_d = d;
                                    min_s = cum[i] + t * (cum[i + 1] - cum[i]);
                                }
                            }
                            let frac = (min_s / total).clamp(0.0, 1.0);
                            let contains_center =
                                surface_contains_point_m(ch, center.0, center.1);
                            let prio = ch.kind.layer_priority();
                            let better = match best_chan {
                                None => true,
                                Some((_, best_contains, best_prio, best_d, _)) => {
                                    match (contains_center, best_contains) {
                                        (true, false) => true,
                                        (false, true) => false,
                                        (true, true) => {
                                            prio > best_prio
                                                || (prio == best_prio && min_d < best_d)
                                        }
                                        (false, false) => min_d < best_d,
                                    }
                                }
                            };
                            if better {
                                best_chan = Some((ch, contains_center, prio, min_d, frac));
                            }
                        }

                        let (ch, _, _, dist_to_cl, frac) =
                            best_chan.expect("BridgeDeck missing underlying channel");
                        assert!(
                            dist_to_cl < 2.0,
                            "{preset:?} {w}x{h} seed {seed}: bridge center far from channel ({dist_to_cl}m)"
                        );
                        let (_, t_chan, _) =
                            ribbon_macro_chord_at_fraction(ch, frac, 1.6).unwrap();

                        let rx = p[1].0 - p[0].0;
                        let ry = p[1].1 - p[0].1;
                        let rlen = rx.hypot(ry).max(1e-4);
                        let dot = (rx / rlen) * t_chan.0 + (ry / rlen) * t_chan.1;
                        assert!(
                            dot.abs() <= 0.25,
                            "{preset:?} {w}x{h} seed {seed}: guardrail axis not perpendicular to channel (|dot| = {})",
                            dot.abs()
                        );

                        let end_a = (0.5 * (p[1].0 + p[2].0), 0.5 * (p[1].1 + p[2].1));
                        let end_b = (0.5 * (p[3].0 + p[0].0), 0.5 * (p[3].1 + p[0].1));
                        for (label, end) in [("end_a", end_a), ("end_b", end_b)] {
                            let in_water = sec.surfaces.iter().any(|s| {
                                s.kind.is_water_or_channel()
                                    && surface_contains_point_m(s, end.0, end.1)
                            });
                            assert!(
                                !in_water,
                                "{preset:?} {w}x{h} seed {seed}: bridge {label} at ({:.2}, {:.2}) landed inside water/channel",
                                end.0, end.1
                            );
                        }
                        bridges_checked += 1;
                    }
                }
            }
        }
        assert!(
            bridges_checked >= 60,
            "expected >= 60 bridges checked across nature presets, got {bridges_checked}"
        );
    }
}
