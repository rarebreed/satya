//! # Upgraded Procedural Facility, Clinical, Transit & Urban Sector Generators (`map_sector_gen`)
//!
//! Upgrades the five classic `SectorPreset`s ([`FacilitySectorStyle`]: `UrbanCrossroads`, `DockingBay`,
//! `MedLab`, `DerelictCorridor`, `OrbitalAirlock`) from bare cell blocks into full [`ProceduralSector`]
//! layouts emitting thin architectural [`MapWallEdge`]s, continuous vector [`MapSurfaceFeature`]s
//! (`AsphaltRoad`, `ConcreteWalk`, `InteriorFloor`), elevated catwalks, hazard spills, and
//! continuously oriented props.
//!
//! Sci-fi & clinical bay builders live in [`crate::map_sector_scifi`].

use satya_schema::{
    EdgeSide, MapPointCm, MapPropPlacement, MapSurfaceFeature, MapWallEdge, MaterialClass,
    PropKind, SurfaceFeatureKind, SurfaceShapeKind, WallEdgeKind,
};
use serde::{Deserialize, Serialize};

use crate::map_dressing::{PropObstruction, make_prop, prop_spec};
use crate::map_sector_scifi::{
    build_derelict_corridor, build_docking_bay, build_med_lab, build_orbital_airlock,
};
use crate::tactical_map::ProceduralSector;
use crate::tactical_map_edges::WallEdgeIndex;
use crate::tactical_map_geom::{
    all_walkable_areas_connected, prop_crosses_wall_edge, props_overlap_continuous,
};

/// Upgraded facility / urban layout family corresponding to the five classic sector presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FacilitySectorStyle {
    UrbanCrossroads,
    DockingBay,
    MedLab,
    DerelictCorridor,
    OrbitalAirlock,
}

pub(crate) const SURF_BASE: u8 = 0;
pub(crate) const SURF_CATWALK: u8 = 3;
pub(crate) const SURF_HAZARD: u8 = 4;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub(crate) fn range(&mut self, lo: i32, hi_excl: i32) -> i32 {
        if hi_excl <= lo {
            lo
        } else {
            lo + (self.next() % (hi_excl - lo) as u64) as i32
        }
    }

    pub(crate) fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next() % den < num
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    pub(crate) x0: i32,
    pub(crate) y0: i32,
    pub(crate) x1: i32,
    pub(crate) y1: i32,
}

impl Rect {
    pub(crate) const fn h(self) -> i32 {
        self.y1 - self.y0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    North,
    East,
    South,
    West,
}

impl Side {
    const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];
    const fn rot(self) -> i32 {
        match self {
            Self::North => 0,
            Self::East => 90,
            Self::South => 180,
            Self::West => 270,
        }
    }
}

pub(crate) struct SectorBuilder {
    pub(crate) w: i32,
    pub(crate) h: i32,
    pub(crate) rng: Rng,
    surf: Vec<u8>,
    edges: Vec<MapWallEdge>,
    edge_idx: WallEdgeIndex,
    blocked: Vec<bool>,
    props: Vec<MapPropPlacement>,
    surfaces: Vec<MapSurfaceFeature>,
}

/// Surface-layer placement filter for facility and street props in [`SectorBuilder`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SurfaceAllowance {
    pub(crate) allow_road: bool,
    pub(crate) allow_indoor: bool,
}

impl SurfaceAllowance {
    pub(crate) const EXTERIOR_OFF_ROAD: Self = Self { allow_road: false, allow_indoor: false };
    pub(crate) const EXTERIOR_ANY_ROAD: Self = Self { allow_road: true, allow_indoor: false };
}

impl SectorBuilder {
    fn new(w: i32, h: i32, seed: u64) -> Self {
        Self {
            w,
            h,
            rng: Rng(seed),
            surf: vec![SURF_BASE; (w * h) as usize],
            edges: Vec::new(),
            edge_idx: WallEdgeIndex::from_edges(w, h, &[]),
            blocked: vec![false; (w * h) as usize],
            props: Vec::new(),
            surfaces: Vec::new(),
        }
    }

    pub(crate) const fn whole(&self) -> Rect {
        Rect { x0: 0, y0: 0, x1: self.w, y1: self.h }
    }

    const fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    pub(crate) fn set_surf(&mut self, x: i32, y: i32, code: u8) {
        if self.in_bounds(x, y) {
            self.surf[(y * self.w + x) as usize] = code;
        }
    }

    pub(crate) fn add_polygon_surface(
        &mut self,
        kind: SurfaceFeatureKind,
        pts_m: &[(f32, f32)],
        corner_radius_cm: i32,
        variant_seed: u32,
    ) {
        self.surfaces.push(MapSurfaceFeature {
            kind,
            shape: SurfaceShapeKind::Polygon,
            points_cm: pts_m
                .iter()
                .map(|&(x, y)| MapPointCm::new((x * 100.0).round() as i32, (y * 100.0).round() as i32))
                .collect(),
            width_cm: 0,
            border_style_cm: corner_radius_cm,
            variant_seed,
        });
    }

    pub(crate) fn add_ribbon_spline(
        &mut self,
        kind: SurfaceFeatureKind,
        ctrl_m: &[(f32, f32)],
        width_cm: i32,
        variant_seed: u32,
    ) {
        self.surfaces.push(MapSurfaceFeature {
            kind,
            shape: SurfaceShapeKind::RibbonSpline,
            points_cm: ctrl_m
                .iter()
                .map(|&(x, y)| MapPointCm::new((x * 100.0).round() as i32, (y * 100.0).round() as i32))
                .collect(),
            width_cm,
            border_style_cm: 0,
            variant_seed,
        });
    }

    pub(crate) fn add_pillar(&mut self, x: i32, y: i32) {
        if self.in_bounds(x, y) && !self.blocked[(y * self.w + x) as usize] {
            let vseed = self.rng.next() as u32;
            self.blocked[(y * self.w + x) as usize] = true;
            self.props.push(make_prop(PropKind::StructuralPillar, x, y, 0, None, vseed));
        }
    }

    pub(crate) fn put_edge(&mut self, pos: (i32, i32), side: EdgeSide, kind: WallEdgeKind, material: MaterialClass) {
        let e = MapWallEdge { x: pos.0, y: pos.1, side, kind, material };
        match self.edges.iter_mut().find(|o| o.x == e.x && o.y == e.y && o.side == e.side) {
            Some(o) => *o = e,
            None => self.edges.push(e),
        }
    }

    pub(crate) fn perimeter(&mut self, wall_mat: MaterialClass, door_mat: MaterialClass) {
        let (w, h, mid_y) = (self.w, self.h, self.h / 2);
        for x in 0..w {
            self.put_edge((x, 0), EdgeSide::North, WallEdgeKind::Wall, wall_mat);
            self.put_edge((x, h), EdgeSide::North, WallEdgeKind::Wall, wall_mat);
        }
        for y in 0..h {
            self.put_edge((0, y), EdgeSide::West, WallEdgeKind::Wall, wall_mat);
            self.put_edge((w, y), EdgeSide::West, WallEdgeKind::Wall, wall_mat);
        }
        self.put_edge((0, mid_y), EdgeSide::West, WallEdgeKind::DoorOpen, door_mat);
        if mid_y >= 2 {
            self.put_edge((0, mid_y - 1), EdgeSide::West, WallEdgeKind::DoorClosed, door_mat);
        }
        self.put_edge((w, mid_y), EdgeSide::West, WallEdgeKind::DoorClosed, door_mat);
    }

    pub(crate) fn add_corner_pilasters(&mut self) {
        for &(px, py) in &[(0, 0), (self.w - 1, 0), (0, self.h - 1), (self.w - 1, self.h - 1)] {
            self.add_pillar(px, py);
        }
    }

    pub(crate) fn add_perimeter_windows(&mut self, spacing: i32) {
        let idx = WallEdgeIndex::from_edges(self.w, self.h, &self.edges);
        let (w, h, mid_y) = (self.w, self.h, self.h / 2);
        let off = self.rng.range(0, spacing.max(2));
        let mut picks = Vec::new();
        let part_v = |x: i32, y: i32| idx.get(x, y, EdgeSide::West).is_some() && x > 0 && x < w;
        let part_h = |x: i32, y: i32| idx.get(x, y, EdgeSide::North).is_some() && y > 0 && y < h;
        for x in 1..(w - 1) {
            if (x + off) % spacing == 0 {
                if !part_v(x, 0) && !part_v(x + 1, 0) {
                    picks.push((x, 0, EdgeSide::North));
                }
                if !part_v(x, h - 1) && !part_v(x + 1, h - 1) {
                    picks.push((x, h, EdgeSide::North));
                }
            }
        }
        for y in 1..(h - 1) {
            if (y + off) % spacing == 0 && (y - mid_y).abs() > 1 {
                if !part_h(0, y) && !part_h(0, y + 1) {
                    picks.push((0, y, EdgeSide::West));
                }
                if !part_h(w - 1, y) && !part_h(w - 1, y + 1) {
                    picks.push((w, y, EdgeSide::West));
                }
            }
        }
        for (x, y, side) in picks {
            self.put_edge((x, y), side, WallEdgeKind::Window, MaterialClass::Glass);
        }
    }

    pub(crate) fn enclose_rect(&mut self, r: Rect, wall_mat: MaterialClass) {
        for x in r.x0..r.x1 {
            self.put_edge((x, r.y0), EdgeSide::North, WallEdgeKind::Wall, wall_mat);
            self.put_edge((x, r.y1), EdgeSide::North, WallEdgeKind::Wall, wall_mat);
        }
        for y in r.y0..r.y1 {
            self.put_edge((r.x0, y), EdgeSide::West, WallEdgeKind::Wall, wall_mat);
            self.put_edge((r.x1, y), EdgeSide::West, WallEdgeKind::Wall, wall_mat);
        }
    }

    fn reserve(&mut self, x: i32, y: i32) {
        if self.in_bounds(x, y) {
            self.blocked[(y * self.w + x) as usize] = true;
        }
    }

    pub(crate) fn reserve_spawns_and_doors(&mut self) {
        let mid_y = self.h / 2;
        for &(sx, sy) in &[
            (2, mid_y),
            (2, (mid_y - 1).max(1)),
            (2, (mid_y + 1).min(self.h - 2)),
            (3, mid_y),
            (self.w - 3, mid_y),
            (self.w - 3, (mid_y + 1).min(self.h - 2)),
            ((self.w - 5).max(2), mid_y),
        ] {
            self.reserve(sx, sy);
        }
        let edges = self.edges.clone();
        for e in &edges {
            if matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen) {
                match e.side {
                    EdgeSide::West => {
                        self.reserve(e.x - 1, e.y);
                        self.reserve(e.x, e.y);
                    }
                    EdgeSide::North => {
                        self.reserve(e.x, e.y - 1);
                        self.reserve(e.x, e.y);
                    }
                }
            }
        }
        self.edge_idx = WallEdgeIndex::from_edges(self.w, self.h, &self.edges);
    }

    fn can_place(&self, pos: (i32, i32), size: (i32, i32), bounds: Rect) -> bool {
        let ((x, y), (fw, fh)) = (pos, size);
        if x < bounds.x0 || y < bounds.y0 || x + fw > bounds.x1 || y + fh > bounds.y1 {
            return false;
        }
        for yy in y..y + fh {
            for xx in x..x + fw {
                if !self.in_bounds(xx, yy) || self.blocked[(yy * self.w + xx) as usize] {
                    return false;
                }
                if xx > x && self.edge_idx.get(xx, yy, EdgeSide::West).is_some() {
                    return false;
                }
                if yy > y && self.edge_idx.get(xx, yy, EdgeSide::North).is_some() {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn try_place(
        &mut self,
        kind: PropKind,
        pos: (i32, i32),
        rot_deg: i32,
        len: Option<i32>,
        bounds: Rect,
    ) -> Option<MapPropPlacement> {
        let vseed = self.rng.next() as u32;
        let p = make_prop(kind, pos.0, pos.1, rot_deg, len, vseed);
        if !self.can_place((p.x, p.y), (p.w_cells, p.h_cells), bounds) {
            return None;
        }
        if !prop_spec(kind).floor_layer {
            if prop_crosses_wall_edge(&p, &self.edges) {
                return None;
            }
            for existing in &self.props {
                if !prop_spec(existing.kind).floor_layer && props_overlap_continuous(&p, existing) {
                    return None;
                }
            }
            for yy in p.y..p.y + p.h_cells {
                for xx in p.x..p.x + p.w_cells {
                    self.blocked[(yy * self.w + xx) as usize] = true;
                }
            }
        }
        self.props.push(p);
        Some(p)
    }

    pub(crate) fn against_wall(&mut self, kind: PropKind, r: Rect, side: Side, len: Option<i32>) -> Option<MapPropPlacement> {
        let rot = side.rot();
        let probe = make_prop(kind, 0, 0, rot, len, 0);
        let (fw, fh) = (probe.w_cells, probe.h_cells);
        let cands: Vec<(i32, i32)> = match side {
            Side::North => (r.x0..=r.x1 - fw).map(|x| (x, r.y0)).collect(),
            Side::South => (r.x0..=r.x1 - fw).map(|x| (x, r.y1 - fh)).collect(),
            Side::West => (r.y0..=r.y1 - fh).map(|y| (r.x0, y)).collect(),
            Side::East => (r.y0..=r.y1 - fh).map(|y| (r.x1 - fw, y)).collect(),
        };
        if cands.is_empty() {
            return None;
        }
        let start = self.rng.range(0, cands.len() as i32) as usize;
        (0..cands.len())
            .map(|i| cands[(start + i) % cands.len()])
            .find_map(|pos| self.try_place(kind, pos, rot, len, r))
    }

    pub(crate) fn against_any_wall(&mut self, kind: PropKind, r: Rect, len: Option<i32>) -> Option<MapPropPlacement> {
        let start = self.rng.range(0, 4) as usize;
        (0..4).find_map(|i| self.against_wall(kind, r, Side::ALL[(start + i) % 4], len))
    }

    pub(crate) fn prop_allows_surfaces(
        &self,
        p: &MapPropPlacement,
        allow: SurfaceAllowance,
    ) -> bool {
        let fp = p.footprint_m();
        let (hx, hy) = (fp.span_w_m * 0.42, fp.span_h_m * 0.42);
        for &(ox, oy) in &[(0.0, 0.0), (-hx, -hy), (hx, -hy), (-hx, hy), (hx, hy)] {
            let (wx, wy) = fp.local_offset_to_world(ox, oy);
            if let Some(sf) = crate::map_surface_queries::topmost_surface_at_m(&self.surfaces, wx, wy) {
                if !allow.allow_road && sf.kind == SurfaceFeatureKind::AsphaltRoad {
                    return false;
                }
                if !allow.allow_indoor && sf.kind == SurfaceFeatureKind::InteriorFloor {
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn try_place_filtered(
        &mut self,
        kind: PropKind,
        pos: (i32, i32),
        rot_deg: i32,
        len: Option<i32>,
        bounds: Rect,
        allow: SurfaceAllowance,
    ) -> Option<MapPropPlacement> {
        let probe = make_prop(kind, pos.0, pos.1, rot_deg, len, 0);
        if !self.prop_allows_surfaces(&probe, allow) {
            return None;
        }
        self.try_place(kind, pos, rot_deg, len, bounds)
    }

    fn scatter_impl(
        &mut self,
        kind: PropKind,
        bounds: Rect,
        count: usize,
        clear_ring: bool,
        allow: Option<SurfaceAllowance>,
    ) -> usize {
        let mut placed = 0;
        let attempts = if allow.is_some() { (count * 36).max(48) } else { (count * 16).max(16) };
        for _ in 0..attempts {
            if placed >= count {
                break;
            }
            let base_rot = self.rng.range(0, 4) * 90;
            let jitter = if clear_ring { self.rng.range(-14, 15) } else { self.rng.range(-6, 7) };
            let rot = (base_rot + jitter).rem_euclid(360);
            let probe = make_prop(kind, 0, 0, rot, None, 0);
            let pad = i32::from(clear_ring);
            let x = self.rng.range(bounds.x0 + pad, bounds.x1 - probe.w_cells - pad + 1);
            let y = self.rng.range(bounds.y0 + pad, bounds.y1 - probe.h_cells - pad + 1);
            let check = if clear_ring {
                self.can_place((x - 1, y - 1), (probe.w_cells + 2, probe.h_cells + 2), bounds)
            } else {
                true
            };
            let ok = check && match allow {
                Some(rule) => self.try_place_filtered(kind, (x, y), rot, None, bounds, rule).is_some(),
                None => self.try_place(kind, (x, y), rot, None, bounds).is_some(),
            };
            if ok {
                placed += 1;
            }
        }
        placed
    }

    pub(crate) fn scatter_in(&mut self, kind: PropKind, bounds: Rect, count: usize, clear_ring: bool) {
        self.scatter_impl(kind, bounds, count, clear_ring, None);
    }

    pub(crate) fn scatter_filtered(
        &mut self,
        kind: PropKind,
        bounds: Rect,
        count: usize,
        clear_ring: bool,
        allow: SurfaceAllowance,
    ) -> usize {
        self.scatter_impl(kind, bounds, count, clear_ring, Some(allow))
    }

    fn emit_surface_props(&mut self) {
        let mut decals = Vec::new();
        for (code, kind) in [
            (SURF_CATWALK, PropKind::ElevatedCatwalk),
            (SURF_HAZARD, PropKind::HazardSpill),
        ] {
            for y in 0..self.h {
                let mut x = 0;
                while x < self.w {
                    if self.surf[(y * self.w + x) as usize] != code {
                        x += 1;
                        continue;
                    }
                    let x0 = x;
                    while x < self.w && self.surf[(y * self.w + x) as usize] == code {
                        x += 1;
                    }
                    decals.push(make_prop(kind, x0, y, 0, Some(x - x0), self.rng.next() as u32));
                }
            }
        }
        decals.append(&mut self.props);
        self.props = decals;
    }

    fn finalize(mut self) -> ProceduralSector {
        self.emit_surface_props();
        let (w, h, mid_y) = (self.w, self.h, self.h / 2);
        let pads = [
            (2, mid_y),
            (2, (mid_y - 1).max(1)),
            (w - 3, mid_y),
            (w - 3, (mid_y + 1).min(h - 2)),
        ];
        self.props.retain(|p| {
            prop_spec(p.kind).floor_layer
                || !pads.iter().any(|&(px, py)| p.covers(px, py))
        });
        loop {
            if all_walkable_areas_connected(w, h, &self.edges, &self.props) {
                return ProceduralSector {
                    wall_edges: self.edges,
                    props: self.props,
                    surfaces: self.surfaces,
                    ..ProceduralSector::default()
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
                        ..ProceduralSector::default()
                    };
                }
            }
        }
    }
}

/// Generates an upgraded [`ProceduralSector`] with thin [`MapWallEdge`]s, vector surfaces, and props.
#[must_use]
pub fn generate_facility_sector(
    style: FacilitySectorStyle,
    width: i32,
    height: i32,
    seed: i64,
) -> ProceduralSector {
    let (w, h) = (
        width.clamp(crate::tactical_map::MIN_SECTOR_DIM, crate::tactical_map::MAX_SECTOR_DIM),
        height.clamp(crate::tactical_map::MIN_SECTOR_DIM, crate::tactical_map::MAX_SECTOR_DIM),
    );
    let salt = match style {
        FacilitySectorStyle::UrbanCrossroads => 0xC405,
        FacilitySectorStyle::DockingBay => 0xD0C4,
        FacilitySectorStyle::MedLab => 0x3ED1,
        FacilitySectorStyle::DerelictCorridor => 0xDEC0,
        FacilitySectorStyle::OrbitalAirlock => 0xA140,
    };
    let mut b = SectorBuilder::new(w, h, (seed as u64) ^ salt);
    match style {
        FacilitySectorStyle::UrbanCrossroads => crate::map_urban_grammars::build_urban_crossroads(&mut b),
        FacilitySectorStyle::DockingBay => build_docking_bay(&mut b),
        FacilitySectorStyle::MedLab => build_med_lab(&mut b),
        FacilitySectorStyle::DerelictCorridor => build_derelict_corridor(&mut b),
        FacilitySectorStyle::OrbitalAirlock => build_orbital_airlock(&mut b),
    }
    b.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tactical_map_geom::{TOKEN_RADIUS_M, circle_collides_obstacles};

    const ALL_STYLES: [FacilitySectorStyle; 5] = [
        FacilitySectorStyle::UrbanCrossroads,
        FacilitySectorStyle::DockingBay,
        FacilitySectorStyle::MedLab,
        FacilitySectorStyle::DerelictCorridor,
        FacilitySectorStyle::OrbitalAirlock,
    ];

    #[test]
    fn facility_sectors_are_deterministic_dressed_connected_and_valid() {
        for style in ALL_STYLES {
            for &(w, h) in &[(8, 8), (12, 10), (16, 12), (20, 14), (28, 20)] {
                for seed in [1_i64, 42, 77, 179, 9001] {
                    let s = generate_facility_sector(style, w, h, seed);
                    assert_eq!(s, generate_facility_sector(style, w, h, seed), "{style:?} deterministic");
                    assert!(!s.wall_edges.is_empty(), "{style:?} ({w}x{h}/{seed}) missing wall_edges");
                    assert!(!s.props.is_empty(), "{style:?} ({w}x{h}/{seed}) missing props");
                    assert!(!s.surfaces.is_empty(), "{style:?} ({w}x{h}/{seed}) missing vector surfaces");
                    assert!(
                        !s.props.iter().any(|p| matches!(p.kind, PropKind::AsphaltDrive | PropKind::StonePath)),
                        "{style:?} ({w}x{h}/{seed}) emitted legacy asphalt/stone floor strip"
                    );
                    assert!(
                        s.props.iter().any(|p| prop_spec(p.kind).obstruction == PropObstruction::HalfCover),
                        "{style:?} ({w}x{h}/{seed}) HalfCover"
                    );
                    assert!(
                        s.props.iter().any(|p| prop_spec(p.kind).obstruction == PropObstruction::FullObstacle),
                        "{style:?} ({w}x{h}/{seed}) FullObstacle"
                    );

                    let mid_y = h / 2;
                    for &(px, py) in &[(2, mid_y), (2, (mid_y - 1).max(1)), (w - 3, mid_y), (w - 3, (mid_y + 1).min(h - 2))] {
                        assert!(
                            !circle_collides_obstacles(
                                w as f32,
                                h as f32,
                                &s.wall_edges,
                                &s.props,
                                px as f32 + 0.5,
                                py as f32 + 0.5,
                                TOKEN_RADIUS_M,
                            ),
                            "{style:?} ({w}x{h}) pad ({px},{py}) blocked"
                        );
                    }
                    assert!(
                        all_walkable_areas_connected(w, h, &s.wall_edges, &s.props),
                        "{style:?} ({w}x{h}/{seed}) disconnected"
                    );

                    for (i, p) in s.props.iter().enumerate() {
                        for q in &s.props[i + 1..] {
                            if prop_spec(p.kind).floor_layer || prop_spec(q.kind).floor_layer {
                                continue;
                            }
                            let overlap = props_overlap_continuous(p, q);
                            assert!(!overlap, "{style:?} ({w}x{h}/{seed}): {p:?} overlaps {q:?}");
                        }
                    }
                }
            }
        }

        // Preserve docking_bay (20x14) boundary and spawn checks exercised by signaling tests
        let bay = generate_facility_sector(FacilitySectorStyle::DockingBay, 20, 14, 77);
        assert!(
            circle_collides_obstacles(20.0, 14.0, &bay.wall_edges, &bay.props, 0.5, 0.5, TOKEN_RADIUS_M),
            "(0,0) corner pilaster must block"
        );
        assert!(
            !circle_collides_obstacles(20.0, 14.0, &bay.wall_edges, &bay.props, 3.5, 7.5, TOKEN_RADIUS_M),
            "(3,7) open for move test"
        );
        assert!(
            !circle_collides_obstacles(20.0, 14.0, &bay.wall_edges, &bay.props, 15.5, 7.5, TOKEN_RADIUS_M),
            "(15,7) open for ambush test"
        );
        assert!(bay.props.iter().any(|p| p.kind == PropKind::ElevatedCatwalk));
    }
}
