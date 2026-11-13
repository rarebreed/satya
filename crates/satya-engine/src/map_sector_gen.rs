//! # Upgraded Procedural Facility, Clinical, Transit & Urban Sector Generators (`map_sector_gen`)
//!
//! Upgrades the five classic `SectorPreset`s ([`FacilitySectorStyle`]: `UrbanCrossroads`, `DockingBay`,
//! `MedLab`, `DerelictCorridor`, `OrbitalAirlock`) from bare cell blocks into full [`ProceduralSector`]
//! layouts emitting thin architectural [`MapWallEdge`]s with realistic ballistic [`MaterialClass`]es and
//! multi-cell [`MapPropPlacement`]s, while preserving elevated catwalks, hazard zones, and spawn pads.

use satya_schema::{
    EdgeSide, MapPropPlacement, MapWallEdge, MaterialClass, PropKind, WallEdgeKind,
};
use serde::{Deserialize, Serialize};

use crate::map_dressing::{PropObstruction, make_prop, prop_spec};
use crate::tactical_map::ProceduralSector;
use crate::tactical_map_edges::WallEdgeIndex;
use crate::tactical_map_geom::{TOKEN_RADIUS_M, all_walkable_areas_connected, circle_collides_obstacles};

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

const SURF_BASE: u8 = 0;
const SURF_STONE: u8 = 1;
const SURF_ASPHALT: u8 = 2;
const SURF_CATWALK: u8 = 3;
const SURF_HAZARD: u8 = 4;

#[derive(Debug, Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range(&mut self, lo: i32, hi_excl: i32) -> i32 {
        if hi_excl <= lo {
            lo
        } else {
            lo + (self.next() % (hi_excl - lo) as u64) as i32
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl Rect {
    const fn h(self) -> i32 {
        self.y1 - self.y0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
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

struct SectorBuilder {
    w: i32,
    h: i32,
    rng: Rng,
    surf: Vec<u8>,
    edges: Vec<MapWallEdge>,
    edge_idx: WallEdgeIndex,
    blocked: Vec<bool>,
    props: Vec<MapPropPlacement>,
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
        }
    }

    const fn whole(&self) -> Rect {
        Rect { x0: 0, y0: 0, x1: self.w, y1: self.h }
    }

    const fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    fn set_surf(&mut self, x: i32, y: i32, code: u8) {
        if self.in_bounds(x, y) {
            self.surf[(y * self.w + x) as usize] = code;
        }
    }

    fn add_pillar(&mut self, x: i32, y: i32) {
        if self.in_bounds(x, y) && !self.blocked[(y * self.w + x) as usize] {
            let vseed = self.rng.next() as u32;
            self.blocked[(y * self.w + x) as usize] = true;
            self.props.push(make_prop(PropKind::StructuralPillar, x, y, 0, None, vseed));
        }
    }

    fn paint_surf(&mut self, r: Rect, code: u8) {
        for y in r.y0.max(0)..r.y1.min(self.h) {
            for x in r.x0.max(0)..r.x1.min(self.w) {
                self.surf[(y * self.w + x) as usize] = code;
            }
        }
    }

    fn put_edge(&mut self, pos: (i32, i32), side: EdgeSide, kind: WallEdgeKind, material: MaterialClass) {
        let e = MapWallEdge { x: pos.0, y: pos.1, side, kind, material };
        match self.edges.iter_mut().find(|o| o.x == e.x && o.y == e.y && o.side == e.side) {
            Some(o) => *o = e,
            None => self.edges.push(e),
        }
    }

    fn perimeter(&mut self, wall_mat: MaterialClass, door_mat: MaterialClass) {
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

    fn add_corner_pilasters(&mut self) {
        for &(px, py) in &[(0, 0), (self.w - 1, 0), (0, self.h - 1), (self.w - 1, self.h - 1)] {
            self.add_pillar(px, py);
        }
    }

    fn add_perimeter_windows(&mut self, spacing: i32) {
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

    fn enclose_rect(&mut self, r: Rect, wall_mat: MaterialClass) {
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

    fn reserve_spawns_and_doors(&mut self) {
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

    fn try_place(
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
            for yy in p.y..p.y + p.h_cells {
                for xx in p.x..p.x + p.w_cells {
                    self.blocked[(yy * self.w + xx) as usize] = true;
                }
            }
        }
        self.props.push(p);
        Some(p)
    }

    fn against_wall(&mut self, kind: PropKind, r: Rect, side: Side, len: Option<i32>) -> Option<MapPropPlacement> {
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

    fn against_any_wall(&mut self, kind: PropKind, r: Rect, len: Option<i32>) -> Option<MapPropPlacement> {
        let start = self.rng.range(0, 4) as usize;
        (0..4).find_map(|i| self.against_wall(kind, r, Side::ALL[(start + i) % 4], len))
    }

    fn scatter_in(&mut self, kind: PropKind, bounds: Rect, count: usize, clear_ring: bool) {
        let mut placed = 0;
        for _ in 0..(count * 16).max(16) {
            if placed >= count {
                break;
            }
            let rot = self.rng.range(0, 4) * 90;
            let probe = make_prop(kind, 0, 0, rot, None, 0);
            let pad = i32::from(clear_ring);
            let x = self.rng.range(bounds.x0 + pad, bounds.x1 - probe.w_cells - pad + 1);
            let y = self.rng.range(bounds.y0 + pad, bounds.y1 - probe.h_cells - pad + 1);
            let check = if clear_ring {
                self.can_place((x - 1, y - 1), (probe.w_cells + 2, probe.h_cells + 2), bounds)
            } else {
                true
            };
            if check && self.try_place(kind, (x, y), rot, None, bounds).is_some() {
                placed += 1;
            }
        }
    }

    fn emit_surface_props(&mut self) {
        let mut decals = Vec::new();
        for (code, kind) in [
            (SURF_ASPHALT, PropKind::AsphaltDrive),
            (SURF_STONE, PropKind::StonePath),
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
                return ProceduralSector { wall_edges: self.edges, props: self.props };
            }
            match self
                .props
                .iter()
                .rposition(|p| prop_spec(p.kind).obstruction == PropObstruction::FullObstacle)
            {
                Some(i) => {
                    self.props.remove(i);
                }
                None => return ProceduralSector { wall_edges: self.edges, props: self.props },
            }
        }
    }
}

fn build_urban_crossroads(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    let bw = (w / 4).clamp(2, 6);
    let bh = (h / 4).clamp(2, 4);
    let nw = Rect { x0: 0, y0: 0, x1: bw, y1: bh };
    let ne = Rect { x0: w - bw, y0: 0, x1: w, y1: bh };
    let sw = Rect { x0: 0, y0: h - bh, x1: bw, y1: h };
    let se = Rect { x0: w - bw, y0: h - bh, x1: w, y1: h };

    // Corner buildings + 1m concrete sidewalks around their street-facing sides
    for &(bx0, by0, bx1, by1) in &[
        (0, 0, (bw + 1).min(mid_x), (bh + 1).min(mid_y)),
        ((w - bw - 1).max(mid_x), 0, w, (bh + 1).min(mid_y)),
        (0, (h - bh - 1).max(mid_y + 1), (bw + 1).min(mid_x), h),
        ((w - bw - 1).max(mid_x), (h - bh - 1).max(mid_y + 1), w, h),
    ] {
        b.paint_surf(Rect { x0: bx0, y0: by0, x1: bx1, y1: by1 }, SURF_STONE);
    }

    for (i, r) in [nw, ne, sw, se].into_iter().enumerate() {
        let mat = if i % 2 == 0 { MaterialClass::Brick } else { MaterialClass::ConcreteBlock };
        b.enclose_rect(r, mat);
        // Street-facing entrance and storefront display windows
        let door_kind = if i == 0 || i == 3 { WallEdgeKind::DoorOpen } else { WallEdgeKind::DoorClosed };
        let door_y = r.y0 + r.h() / 2;
        let door_x = if i % 2 == 0 { r.x1 } else { r.x0 };
        b.put_edge((door_x, door_y), EdgeSide::West, door_kind, MaterialClass::SolidWood);
        let horiz_y = if i < 2 { r.y1 } else { r.y0 };
        for x in (r.x0 + 1)..(r.x1 - 1) {
            if (x - r.x0) % 2 == 1 {
                b.put_edge((x, horiz_y), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
            }
        }
    }

    b.reserve_spawns_and_doors();
    // Sidewalk trees and corner interior furnishing
    let whole = b.whole();
    let _ = b.try_place(PropKind::MediumTree, (bw, bh), 0, None, whole)
        .or_else(|| b.try_place(PropKind::MediumTree, (mid_x - 2, bh), 0, None, whole));
    if bw >= 3 && bh >= 3 {
        let _ = b.against_any_wall(PropKind::KitchenCounter, nw, Some(2));
        b.scatter_in(PropKind::RoundTable, nw, 2, false);
        let _ = b.against_any_wall(PropKind::SteelShelving, ne, None);
        let _ = b.against_any_wall(PropKind::Fridge, ne, None);
        let _ = b.against_any_wall(PropKind::Desk, sw, None);
        let _ = b.against_any_wall(PropKind::Bookshelf, sw, None);
        let _ = b.against_any_wall(PropKind::SteelShelving, se, None);
        let _ = b.against_any_wall(PropKind::DrumCluster, se, None);
    }

    // Parked street vehicles & urban cover
    if w >= 16 && h >= 12 {
        let lane_x = (mid_x - 3).max(bw + 1);
        let _ = b.try_place(PropKind::Car, (lane_x, bh.max(2)), 180, None, whole);
        let _ = b.try_place(PropKind::UtilityVan, (w - bw - 5, mid_y + 1), 90, None, whole);
    }
    let _ = b.try_place(PropKind::JerseyBarrier, (mid_x - 3, mid_y - 1), 0, Some(2), whole);
    let _ = b.try_place(PropKind::JerseyBarrier, (mid_x + 1, mid_y + 1), 0, Some(2), whole);
    let _ = b.try_place(PropKind::Dumpster, (w - bw - 3, 1), 0, None, whole);
    let _ = b.try_place(PropKind::ParkBench, (bw + 1, h - bh - 1), 0, None, whole);
    b.scatter_in(PropKind::Planter, Rect { x0: bw, y0: bh, x1: w - bw, y1: h - bh }, 2, true);
    b.scatter_in(PropKind::WoodCrate, whole, 2, true);
}

fn build_docking_bay(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    b.perimeter(MaterialClass::ConcreteBlock, MaterialClass::SheetSteel);
    b.add_corner_pilasters();

    // Elevated steel catwalk along North gantry + industrial fuel/chemical hazard spill
    let cat_y = if h >= 12 { 2 } else { 1 };
    for x in 3..(w - 3) {
        b.set_surf(x, cat_y, SURF_CATWALK);
    }
    b.set_surf(mid_x - 1, (h - 3).max(cat_y + 2), SURF_HAZARD);
    b.set_surf(mid_x, (h - 3).max(cat_y + 2), SURF_HAZARD);

    // East loading apron + glazed SW shipping/dispatch booth
    if w >= 16 && h >= 12 {
        b.paint_surf(Rect { x0: w - 6, y0: 3, x1: w - 1, y1: h - 1 }, SURF_ASPHALT);
    }
    let office = if w >= 14 && h >= 11 {
        let r = Rect { x0: 1, y0: h - 4, x1: 6, y1: h };
        b.enclose_rect(r, MaterialClass::Drywall);
        for x in 2..5 {
            b.put_edge((x, r.y0), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
        }
        b.put_edge((r.x1, h - 2), EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
        Some(r)
    } else {
        None
    };
    b.add_perimeter_windows(4);
    b.reserve_spawns_and_doors();

    let whole = b.whole();
    if let Some(off) = office {
        let _ = b.against_wall(PropKind::Desk, off, Side::West, None);
        let _ = b.against_wall(PropKind::SteelShelving, off, Side::South, None);
    }
    if w >= 16 && h >= 14 {
        let _ = b.try_place(PropKind::UtilityVan, (w - 6, 4), 90, None, whole);
    }
    let _ = b.try_place(PropKind::SteelShelving, (mid_x - 1, 4), 0, None, whole)
        .or_else(|| b.try_place(PropKind::SteelShelving, (4, 1), 0, None, whole));
    let _ = b.try_place(PropKind::SteelShelving, (mid_x + 1, h - 2), 0, None, whole);
    let _ = b.try_place(PropKind::JerseyBarrier, (mid_x - 4, mid_y + 1), 90, Some(2), whole);
    b.scatter_in(PropKind::WoodCrate, Rect { x0: 3, y0: cat_y + 1, x1: w - 3, y1: h - 1 }, 5, true);
    b.scatter_in(PropKind::DrumCluster, Rect { x0: 4, y0: cat_y + 1, x1: w - 3, y1: h - 1 }, 3, true);
}

fn build_med_lab(b: &mut SectorBuilder) {
    let (w, h, mid_y) = (b.w, b.h, b.h / 2);
    b.perimeter(MaterialClass::ConcreteBlock, MaterialClass::SolidWood);
    b.add_corner_pilasters();

    if w >= 12 {
        let x1 = (w / 3).clamp(3, w - 5);
        let x2 = (2 * w / 3).clamp(x1 + 2, w - 3);
        // West drywall partition: intake triage ward -> main surgical bay
        for y in 0..h {
            let (kind, mat) = if y == mid_y || y == 2 {
                (WallEdgeKind::DoorOpen, MaterialClass::Drywall)
            } else if (y - mid_y).abs() == 1 {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, MaterialClass::Drywall)
            };
            b.put_edge((x1, y), EdgeSide::West, kind, mat);
        }
        // East reinforced bio-containment bulkhead
        for y in 0..h {
            let (kind, mat) = if y == mid_y || y == (h - 3).max(1) {
                (WallEdgeKind::DoorClosed, MaterialClass::SheetSteel)
            } else if y == 3 || y == (h - 4).max(4) {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete)
            };
            b.put_edge((x2, y), EdgeSide::West, kind, mat);
        }
        b.set_surf(x2 + 1, (mid_y + 1).min(h - 2), SURF_HAZARD);
        b.set_surf((x2 + 2).min(w - 2), (mid_y + 1).min(h - 2), SURF_HAZARD);
        b.add_perimeter_windows(4);
        b.reserve_spawns_and_doors();

        let triage = Rect { x0: 0, y0: 0, x1, y1: h };
        let trauma = Rect { x0: x1, y0: 0, x1: x2, y1: h };
        let lab = Rect { x0: x2, y0: 0, x1: w, y1: h };

        let _ = b.against_any_wall(PropKind::Desk, triage, None);
        let _ = b.against_any_wall(PropKind::Loveseat, triage, None);
        let _ = b.against_any_wall(PropKind::Planter, triage, None);
        let _ = b.against_any_wall(PropKind::Fridge, trauma, None);
        let _ = b.against_any_wall(PropKind::SteelShelving, trauma, None);
        let _ = b.against_any_wall(PropKind::KitchenCounter, trauma, Some(3));
        b.scatter_in(PropKind::Bed, trauma, 2, true);
        let _ = b.against_any_wall(PropKind::Fridge, lab, None);
        let _ = b.against_any_wall(PropKind::Desk, lab, None);
        b.scatter_in(PropKind::DrumCluster, lab, 2, true);
    } else {
        let sx = w / 2;
        for y in 0..h {
            let kind = if y == mid_y { WallEdgeKind::DoorClosed } else { WallEdgeKind::Wall };
            b.put_edge((sx, y), EdgeSide::West, kind, MaterialClass::ConcreteBlock);
        }
        b.set_surf(sx + 1, (mid_y + 1).min(h - 2), SURF_HAZARD);
        b.add_perimeter_windows(3);
        b.reserve_spawns_and_doors();
        let whole = b.whole();
        b.scatter_in(PropKind::Bed, whole, 1, false);
        let _ = b.against_any_wall(PropKind::Fridge, whole, None);
    }
}

fn build_derelict_corridor(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    b.perimeter(MaterialClass::ReinforcedConcrete, MaterialClass::SheetSteel);
    b.add_corner_pilasters();

    let y_top = (h / 4).clamp(2, 4);
    let y_bot = (h - y_top).max(y_top + 3);
    if w >= 12 && h >= 10 {
        // Side conduit-bay partitions along North & South flanks with hatches & breached grilles
        for x in 0..w {
            let top_door = x == 2 || x == mid_x || x == w - 3;
            let bot_door = x == 3 || x == mid_x - 1 || x == w - 4;
            let top_kind = if top_door { WallEdgeKind::DoorOpen } else if x % 4 == 1 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
            let bot_kind = if bot_door { WallEdgeKind::DoorClosed } else if x % 4 == 2 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
            b.put_edge((x, y_top), EdgeSide::North, top_kind, MaterialClass::ConcreteBlock);
            b.put_edge((x, y_bot), EdgeSide::North, bot_kind, MaterialClass::ConcreteBlock);
        }
        for &div_x in &[w / 3, (2 * w) / 3] {
            for y in 0..y_top {
                b.put_edge((div_x, y), EdgeSide::West, WallEdgeKind::Wall, MaterialClass::SheetSteel);
            }
            for y in y_bot..h {
                b.put_edge((div_x, y), EdgeSide::West, WallEdgeKind::Wall, MaterialClass::SheetSteel);
            }
            // Transverse blast doors dividing the main spine into 3 airlock segments
            for y in y_top..y_bot {
                let kind = if y == mid_y {
                    WallEdgeKind::DoorOpen
                } else if y == (mid_y - 1).max(y_top) {
                    WallEdgeKind::DoorClosed
                } else {
                    WallEdgeKind::Wall
                };
                b.put_edge((div_x, y), EdgeSide::West, kind, MaterialClass::ReinforcedConcrete);
            }
        }
    } else {
        for y in 0..h {
            let kind = if y == mid_y { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall };
            b.put_edge((mid_x, y), EdgeSide::West, kind, MaterialClass::ReinforcedConcrete);
        }
    }

    b.set_surf((mid_x - 1).max(1), (mid_y + 1).min(h - 2), SURF_HAZARD);
    b.set_surf(mid_x, (mid_y + 1).min(h - 2), SURF_HAZARD);
    b.reserve_spawns_and_doors();

    let whole = b.whole();
    let spine = Rect { x0: 1, y0: y_top.min(h - 2), x1: w - 1, y1: y_bot.min(h - 1).max(y_top + 1) };
    let _ = b.against_any_wall(PropKind::SteelShelving, Rect { x0: 1, y0: 0, x1: w - 1, y1: y_top }, None)
        .or_else(|| b.against_any_wall(PropKind::SteelShelving, whole, None));
    let _ = b.against_any_wall(PropKind::Desk, Rect { x0: 1, y0: y_bot, x1: w - 1, y1: h }, None);
    b.scatter_in(PropKind::Boulder, spine, 2, true);
    b.scatter_in(PropKind::DrumCluster, whole, 4, true);
    b.scatter_in(PropKind::WoodCrate, whole, 4, true);
}

fn build_orbital_airlock(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    b.perimeter(MaterialClass::ArmorSteel, MaterialClass::ArmorSteel);

    // 4 structural Mag-Rail blast columns (`StructuralPillar`s for corner-peeking)
    if w >= 14 && h >= 10 {
        for &(px, py) in &[(mid_x - 4, 3), (mid_x + 3, 3), (mid_x - 4, h - 4), (mid_x + 3, h - 4)] {
            b.add_pillar(px, py);
        }
    } else {
        b.add_corner_pilasters();
    }

    // Elevated gantry pads & plasma vent hazard strip
    b.set_surf(mid_x + 1, (mid_y - 2).max(1), SURF_HAZARD);
    b.set_surf((mid_x + 2).min(w - 2), (mid_y - 2).max(1), SURF_HAZARD);

    let mut booths = Vec::new();
    if w >= 14 && h >= 12 {
        let bx0 = (mid_x - 3).max(4);
        let bx1 = (mid_x + 3).min(w - 4);
        let north_booth = Rect { x0: bx0, y0: 0, x1: bx1, y1: 3 };
        let south_booth = Rect { x0: bx0, y0: h - 3, x1: bx1, y1: h };
        b.enclose_rect(north_booth, MaterialClass::SheetSteel);
        b.enclose_rect(south_booth, MaterialClass::SheetSteel);
        for x in (bx0 + 1)..(bx1 - 1) {
            b.put_edge((x, 3), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
            b.put_edge((x, h - 3), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
        }
        b.put_edge((bx0, 1), EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        b.put_edge((bx1, h - 2), EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SheetSteel);
        booths.push(north_booth);
        booths.push(south_booth);
        for x in (bx0 + 1)..(bx1 - 1) {
            b.set_surf(x, 3, SURF_CATWALK);
        }
    }

    if w >= 16 && h >= 10 {
        for &vx in &[4, w - 4] {
            for y in 2..(h - 2) {
                let (kind, mat) = if y == mid_y {
                    (WallEdgeKind::DoorOpen, MaterialClass::ArmorSteel)
                } else if y == mid_y - 1 {
                    (WallEdgeKind::DoorClosed, MaterialClass::ArmorSteel)
                } else if (y - mid_y).abs() == 2 {
                    (WallEdgeKind::Window, MaterialClass::Glass)
                } else {
                    (WallEdgeKind::Wall, MaterialClass::ArmorSteel)
                };
                b.put_edge((vx, y), EdgeSide::West, kind, mat);
            }
        }
    }

    b.reserve_spawns_and_doors();
    let whole = b.whole();
    for booth in booths {
        let _ = b.against_any_wall(PropKind::Desk, booth, None);
        let _ = b.against_any_wall(PropKind::Wardrobe, booth, None);
    }
    let _ = b.try_place(PropKind::JerseyBarrier, (mid_x - 1, (mid_y - 1).max(3)), 90, Some(2), whole);
    let _ = b.try_place(PropKind::SteelShelving, (mid_x, h - 4), 0, None, whole)
        .or_else(|| b.against_any_wall(PropKind::SteelShelving, whole, None));
    b.scatter_in(PropKind::WoodCrate, Rect { x0: 4, y0: 3, x1: (w - 4).max(5), y1: (h - 3).max(4) }, 4, true);
    b.scatter_in(PropKind::DrumCluster, Rect { x0: 4, y0: 3, x1: (w - 4).max(5), y1: (h - 3).max(4) }, 2, true);
}

/// Generates an upgraded [`ProceduralSector`] with thin [`MapWallEdge`]s and multi-cell props.
#[must_use]
pub fn generate_facility_sector(
    style: FacilitySectorStyle,
    width: i32,
    height: i32,
    seed: i64,
) -> ProceduralSector {
    let (w, h) = (width.clamp(8, 48), height.clamp(8, 48));
    let salt = match style {
        FacilitySectorStyle::UrbanCrossroads => 0xC405,
        FacilitySectorStyle::DockingBay => 0xD0C4,
        FacilitySectorStyle::MedLab => 0x3ED1,
        FacilitySectorStyle::DerelictCorridor => 0xDEC0,
        FacilitySectorStyle::OrbitalAirlock => 0xA140,
    };
    let mut b = SectorBuilder::new(w, h, (seed as u64) ^ salt);
    match style {
        FacilitySectorStyle::UrbanCrossroads => build_urban_crossroads(&mut b),
        FacilitySectorStyle::DockingBay => build_docking_bay(&mut b),
        FacilitySectorStyle::MedLab => build_med_lab(&mut b),
        FacilitySectorStyle::DerelictCorridor => build_derelict_corridor(&mut b),
        FacilitySectorStyle::OrbitalAirlock => build_orbital_airlock(&mut b),
    }
    let mut sector = b.finalize();

    // Defensive minimum guarantees on tiny 8x8 boards: ensure at least one HalfCover and one FullObstacle.
    let mid_y = h / 2;
    let has_half_cover = sector
        .props
        .iter()
        .any(|p| prop_spec(p.kind).obstruction == PropObstruction::HalfCover);
    if !has_half_cover
        && let Some(p) = (1..w - 1).find_map(|x| {
            let y = 1;
            let blocked_pad =
                (x == 2 || x == 3 || x == w - 3 || x == w - 5) && (y - mid_y).abs() <= 1;
            (!blocked_pad
                && !circle_collides_obstacles(
                    w as f32,
                    h as f32,
                    &sector.wall_edges,
                    &sector.props,
                    x as f32 + 0.5,
                    y as f32 + 0.5,
                    TOKEN_RADIUS_M,
                ))
            .then(|| make_prop(PropKind::WoodCrate, x, y, 0, None, 1))
        })
    {
        sector.props.push(p);
    }
    let has_full_obstacle = sector
        .props
        .iter()
        .any(|p| prop_spec(p.kind).obstruction == PropObstruction::FullObstacle);
    if !has_full_obstacle
        && let Some((px, py)) = [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)]
            .into_iter()
            .chain((1..w - 1).map(|x| (x, 0)))
            .find(|&(x, y)| {
                !circle_collides_obstacles(
                    w as f32,
                    h as f32,
                    &sector.wall_edges,
                    &sector.props,
                    x as f32 + 0.5,
                    y as f32 + 0.5,
                    TOKEN_RADIUS_M,
                )
            })
    {
        sector.props.push(make_prop(PropKind::StructuralPillar, px, py, 0, None, 1));
    }
    sector
}

#[cfg(test)]
mod tests {
    use super::*;

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
                            let overlap = p.x < q.x + q.w_cells && q.x < p.x + p.w_cells && p.y < q.y + q.h_cells && q.y < p.y + p.h_cells;
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
