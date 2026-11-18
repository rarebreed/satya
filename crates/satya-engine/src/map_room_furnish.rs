//! # Architectural Room Furnishing & Flooring Overlays (`map_room_furnish`)
//!
//! Companion module to [`crate::map_interior_gen`] and [`crate::map_home_floorplan`].
//! Assigns semantic room roles (`Foyer`, `Dining`, `MasterBedroom`, `Closet`, `MasterBath`,
//! `Sunroom`, etc.), places non-sparse cohesive furniture groupings (`Bed` + flanking
//! `Nightstand`s + opposite `Dresser` + lamps + rugs, `Sofa` facing `TvStand` + `Stereo`,
//! `Bathtub` + `BathroomSink` + `Toilet`), and emits room-appropriate vector floor surfaces
//! (`CeramicTileFloor`, `MarbleFloor`, `CarpetFloor`).

use satya_schema::{
    EconomicTier, EdgeSide, MapPointCm, MapPropPlacement, MapSurfaceFeature, MapWallEdge, PropKind,
    SurfaceFeatureKind, SurfaceShapeKind, WallEdgeKind,
};

use crate::map_dressing::{make_prop, prop_spec};
use crate::map_interior_gen::{Rng, Room, Side};
use crate::tactical_map_edges::WallEdgeIndex;

/// Semantic functional role of an interior room or semi-outdoor zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Living,
    Dining,
    Kitchen,
    MasterBedroom,
    Bedroom,
    Closet,
    Office,
    Library,
    Bathroom,
    HalfBath,
    MasterBath,
    Foyer,
    Sunroom,
    Garage,
    HomeGym,
    Hall,
    Laundry,
    BarHall,
    Storage,
    Restroom,
    BackKitchen,
    Bullpen,
    ConferenceRoom,
    Breakroom,
    ServerCloset,
}

/// Unit vector a prop's **front** faces for `rotation_deg` (back faces North at `0°`).
const fn front_dir(rotation_deg: i32) -> (i32, i32) {
    match rotation_deg.rem_euclid(360) {
        90 => (-1, 0),
        180 => (0, -1),
        270 => (1, 0),
        _ => (0, 1),
    }
}

#[inline]
const fn opposite_side(side: Side) -> Side {
    match side {
        Side::North => Side::South,
        Side::South => Side::North,
        Side::East => Side::West,
        Side::West => Side::East,
    }
}

#[inline]
const fn side_from_rot(rot: i32) -> Side {
    match rot.rem_euclid(360) {
        90 => Side::East,
        180 => Side::South,
        270 => Side::West,
        _ => Side::North,
    }
}

const fn requires_solid_wall(kind: PropKind) -> bool {
    matches!(
        kind,
        PropKind::KitchenCounter
            | PropKind::Stove
            | PropKind::Fridge
            | PropKind::MedicalCounter
            | PropKind::BathroomSink
            | PropKind::Toilet
            | PropKind::ShowerStall
            | PropKind::Bathtub
            | PropKind::SpaBath
            | PropKind::WasherDryer
            | PropKind::LinenCloset
            | PropKind::FumeHood
    )
}

pub(crate) struct Furnisher {
    pub(crate) w: i32,
    pub(crate) h: i32,
    pub(crate) blocked: Vec<bool>,
    pub(crate) props: Vec<MapPropPlacement>,
    pub(crate) rng: Rng,
    pub(crate) wall_edges: Option<WallEdgeIndex>,
}

impl Furnisher {
    pub(crate) fn new(w: i32, h: i32, seed: u64) -> Self {
        Self {
            w,
            h,
            blocked: vec![false; (w.max(1) * h.max(1)) as usize],
            props: Vec::new(),
            rng: Rng(seed),
            wall_edges: None,
        }
    }

    pub(crate) fn reserve_cell(&mut self, x: i32, y: i32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.blocked[(y * self.w + x) as usize] = true;
        }
    }

    pub(crate) fn reserve_doors(&mut self, edges: &[MapWallEdge]) {
        self.wall_edges = Some(WallEdgeIndex::from_edges(self.w, self.h, edges));
        for e in edges {
            if matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen) {
                match e.side {
                    EdgeSide::West => {
                        self.reserve_cell(e.x - 1, e.y);
                        self.reserve_cell(e.x, e.y);
                    }
                    EdgeSide::North => {
                        self.reserve_cell(e.x, e.y - 1);
                        self.reserve_cell(e.x, e.y);
                    }
                }
            }
        }
    }

    fn free(&self, x: i32, y: i32, fw: i32, fh: i32, r: Room) -> bool {
        x >= r.x0
            && y >= r.y0
            && x + fw <= r.x1
            && y + fh <= r.y1
            && (y..y + fh)
                .all(|yy| (x..x + fw).all(|xx| !self.blocked[(yy * self.w + xx) as usize]))
    }

    fn free_prop(&self, p: &MapPropPlacement, r: Room) -> bool {
        p.x >= r.x0
            && p.y >= r.y0
            && p.x + p.w_cells <= r.x1
            && p.y + p.h_cells <= r.y1
            && (p.y..p.y + p.h_cells).all(|yy| {
                (p.x..p.x + p.w_cells)
                    .all(|xx| !p.covers(xx, yy) || !self.blocked[(yy * self.w + xx) as usize])
            })
    }

    pub(crate) fn try_place(
        &mut self,
        kind: PropKind,
        x: i32,
        y: i32,
        rot: i32,
        len: Option<i32>,
        r: Room,
    ) -> Option<MapPropPlacement> {
        let seed = self.rng.next() as u32;
        let p = make_prop(kind, x, y, rot, len, seed);
        let decal = prop_spec(kind).floor_layer;
        let fits = if decal {
            x >= r.x0 && y >= r.y0 && x + p.w_cells <= r.x1 && y + p.h_cells <= r.y1
        } else {
            self.free_prop(&p, r)
        };
        if !fits {
            return None;
        }
        if !decal {
            for yy in y..y + p.h_cells {
                for xx in x..x + p.w_cells {
                    if p.covers(xx, yy) {
                        self.blocked[(yy * self.w + xx) as usize] = true;
                    }
                }
            }
        }
        self.props.push(p);
        Some(p)
    }

    /// Places a walkable floor scatter decal (`FloorClothes`, `FloorToys`, `RunnerRug`, `RoundRug`)
    /// only where open cells are unoccupied by solid props or doorway thresholds.
    pub(crate) fn try_place_open_decal(
        &mut self,
        kind: PropKind,
        x: i32,
        y: i32,
        rot: i32,
        len: Option<i32>,
        r: Room,
    ) -> Option<MapPropPlacement> {
        let probe = make_prop(kind, x, y, rot, len, 0);
        if !self.free_prop(&probe, r) {
            return None;
        }
        self.try_place(kind, x, y, rot, len, r)
    }

    fn has_backed_wall(&self, x: i32, y: i32, fw: i32, fh: i32, side: Side) -> bool {
        let Some(ref idx) = self.wall_edges else {
            return true;
        };
        let is_solid =
            |k: Option<WallEdgeKind>| matches!(k, Some(WallEdgeKind::Wall | WallEdgeKind::Window));
        match side {
            Side::North => (0..fw).all(|dx| is_solid(idx.get(x + dx, y, EdgeSide::North))),
            Side::South => (0..fw).all(|dx| is_solid(idx.get(x + dx, y + fh, EdgeSide::North))),
            Side::West => (0..fh).all(|dy| is_solid(idx.get(x, y + dy, EdgeSide::West))),
            Side::East => (0..fh).all(|dy| is_solid(idx.get(x + fw, y + dy, EdgeSide::West))),
        }
    }

    fn against_wall_inner(
        &mut self,
        kind: PropKind,
        r: Room,
        side: Side,
        len: Option<i32>,
        require_wall: bool,
    ) -> Option<MapPropPlacement> {
        let rot = side.rot();
        let probe = make_prop(kind, 0, 0, rot, len, 0);
        let (fw, fh) = (probe.w_cells, probe.h_cells);
        let mut cands: Vec<(i32, i32)> = match side {
            Side::North => (r.x0..=r.x1 - fw).map(|x| (x, r.y0)).collect(),
            Side::South => (r.x0..=r.x1 - fw).map(|x| (x, r.y1 - fh)).collect(),
            Side::West => (r.y0..=r.y1 - fh).map(|y| (r.x0, y)).collect(),
            Side::East => (r.y0..=r.y1 - fh).map(|y| (r.x1 - fw, y)).collect(),
        };
        if require_wall {
            cands.retain(|&(x, y)| self.has_backed_wall(x, y, fw, fh, side));
        }
        if cands.is_empty() {
            return None;
        }
        let start = self.rng.range(0, cands.len() as i32) as usize;
        (0..cands.len())
            .map(|i| cands[(start + i) % cands.len()])
            .find_map(|(x, y)| self.try_place(kind, x, y, rot, len, r))
    }

    pub(crate) fn against_wall(
        &mut self,
        kind: PropKind,
        r: Room,
        side: Side,
        len: Option<i32>,
    ) -> Option<MapPropPlacement> {
        self.against_wall_inner(kind, r, side, len, true)
            .or_else(|| {
                (!requires_solid_wall(kind))
                    .then(|| self.against_wall_inner(kind, r, side, len, false))
                    .flatten()
            })
    }

    pub(crate) fn against_any_wall(
        &mut self,
        kind: PropKind,
        r: Room,
        len: Option<i32>,
    ) -> Option<MapPropPlacement> {
        let start = self.rng.range(0, 4) as usize;
        (0..4)
            .find_map(|i| self.against_wall_inner(kind, r, Side::ALL[(start + i) % 4], len, true))
            .or_else(|| {
                (!requires_solid_wall(kind))
                    .then(|| {
                        (0..4).find_map(|i| {
                            self.against_wall_inner(kind, r, Side::ALL[(start + i) % 4], len, false)
                        })
                    })
                    .flatten()
            })
    }

    /// Places a `1×1` prop flanking an anchor prop along its shared wall (`Nightstand` beside `Bed`,
    /// `Stereo` beside `TvStand`, `DeskLamp` beside `Desk`).
    fn beside_anchor(
        &mut self,
        anchor: MapPropPlacement,
        kind: PropKind,
        r: Room,
        both_sides: bool,
    ) {
        let rot = anchor.rotation_deg;
        let candidates: [(i32, i32); 2] = match side_from_rot(rot) {
            Side::North => [
                (anchor.x - 1, anchor.y),
                (anchor.x + anchor.w_cells, anchor.y),
            ],
            Side::South => [
                (anchor.x - 1, anchor.y + anchor.h_cells - 1),
                (anchor.x + anchor.w_cells, anchor.y + anchor.h_cells - 1),
            ],
            Side::West => [
                (anchor.x, anchor.y - 1),
                (anchor.x, anchor.y + anchor.h_cells),
            ],
            Side::East => [
                (anchor.x + anchor.w_cells - 1, anchor.y - 1),
                (anchor.x + anchor.w_cells - 1, anchor.y + anchor.h_cells),
            ],
        };
        for &(cx, cy) in &candidates {
            if self.try_place(kind, cx, cy, rot, None, r).is_some() && !both_sides {
                break;
            }
        }
    }

    /// Attempts to place a `1×1` accent prop in any open corner of `r`.
    pub(crate) fn in_corner(&mut self, kind: PropKind, r: Room) -> Option<MapPropPlacement> {
        if r.w() < 2 || r.h() < 2 {
            return None;
        }
        let corners = [
            (r.x0, r.y0, 0),
            (r.x1 - 1, r.y0, 90),
            (r.x1 - 1, r.y1 - 1, 180),
            (r.x0, r.y1 - 1, 270),
        ];
        let start = self.rng.range(0, 4) as usize;
        (0..4).find_map(|i| {
            let (cx, cy, rot) = corners[(start + i) % 4];
            self.try_place(kind, cx, cy, rot, None, r)
        })
    }

    /// Places `kind` with the same rotation, centered in front of `anchor` after `gap` free cells.
    pub(crate) fn in_front(
        &mut self,
        anchor: MapPropPlacement,
        kind: PropKind,
        gap: i32,
        r: Room,
    ) -> Option<MapPropPlacement> {
        let rot = anchor.rotation_deg;
        let probe = make_prop(kind, 0, 0, rot, None, 0);
        let (fw, fh) = (probe.w_cells, probe.h_cells);
        let (x, y) = match front_dir(rot) {
            (0, 1) => (
                anchor.x + (anchor.w_cells - fw) / 2,
                anchor.y + anchor.h_cells + gap,
            ),
            (0, _) => (anchor.x + (anchor.w_cells - fw) / 2, anchor.y - gap - fh),
            (1, _) => (
                anchor.x + anchor.w_cells + gap,
                anchor.y + (anchor.h_cells - fh) / 2,
            ),
            _ => (anchor.x - gap - fw, anchor.y + (anchor.h_cells - fh) / 2),
        };
        self.try_place(kind, x, y, rot, None, r)
    }

    /// Places a rug centered in open floor inside `r`.
    fn center_rug(&mut self, kind: PropKind, r: Room) {
        let probe = make_prop(kind, 0, 0, 0, None, 0);
        if r.w() >= probe.w_cells && r.h() >= probe.h_cells {
            let rx = r.x0 + (r.w() - probe.w_cells) / 2;
            let ry = r.y0 + (r.h() - probe.h_cells) / 2;
            let rot = if kind == PropKind::RunnerRug && r.h() > r.w() {
                90
            } else {
                0
            };
            let _ = self.try_place(kind, rx, ry, rot, None, r);
        }
    }

    /// Scatters a small decorative clutter item (`FloorClothes` or `FloorToys`) onto an open floor cell.
    fn floor_clutter(&mut self, kind: PropKind, r: Room) {
        for _ in 0..8 {
            let x = self.rng.range(r.x0, r.x1);
            let y = self.rng.range(r.y0, r.y1);
            let rot = self.rng.range(0, 360);
            if self
                .try_place_open_decal(kind, x, y, rot, None, r)
                .is_some()
            {
                break;
            }
        }
    }

    /// Scatters up to `count` props with a one-cell clearance ring inside `r` and natural `1°` jitter.
    pub(crate) fn scatter(&mut self, kind: PropKind, r: Room, count: usize, rot: i32) {
        let jitter_range = match kind {
            PropKind::RoundTable
            | PropKind::Armchair
            | PropKind::CoffeeTable
            | PropKind::WoodCrate => 8,
            PropKind::PoolTable => 2,
            _ => 0,
        };
        let mut placed = 0;
        for _ in 0..(count * 12) {
            if placed >= count || r.w() < 3 || r.h() < 3 {
                break;
            }
            let rot_deg = if jitter_range > 0 {
                (rot + self.rng.range(-jitter_range, jitter_range + 1)).rem_euclid(360)
            } else {
                rot
            };
            let probe = make_prop(kind, 0, 0, rot_deg, None, 0);
            if r.w() <= probe.w_cells + 1 || r.h() <= probe.h_cells + 1 {
                break;
            }
            let x = self.rng.range(r.x0 + 1, r.x1 - probe.w_cells);
            let y = self.rng.range(r.y0 + 1, r.y1 - probe.h_cells);
            let ring = Room {
                x0: r.x0,
                y0: r.y0,
                x1: r.x1,
                y1: r.y1,
            };
            if self.free(x - 1, y - 1, probe.w_cells + 2, probe.h_cells + 2, ring)
                && self.try_place(kind, x, y, rot_deg, None, r).is_some()
            {
                placed += 1;
            }
        }
    }

    fn attach_desk_clutter(&mut self, anchor: MapPropPlacement, r: Room) {
        let seed = self.rng.next() as u32;
        let rot = (anchor.rotation_deg + self.rng.range(-12, 13)).rem_euclid(360);
        let p = make_prop(PropKind::DeskClutter, anchor.x, anchor.y, rot, None, seed);
        if p.x >= r.x0 && p.y >= r.y0 && p.x + p.w_cells <= r.x1 && p.y + p.h_cells <= r.y1 {
            self.props.push(p);
        }
    }

    pub(crate) fn furnish(&mut self, role: Role, r: Room) {
        use PropKind as P;
        let area_scale = (r.area() / 36).clamp(1, 5) as usize;
        match role {
            Role::Foyer => {
                let rug_kind = if r.w() >= 3 && r.h() >= 3 && self.rng.chance(1, 2) {
                    P::RoundRug
                } else {
                    P::RunnerRug
                };
                self.center_rug(rug_kind, r);
                let _ = self.in_corner(P::StandingLamp, r);
                let _ = self.in_corner(P::Planter, r);
                if r.w() >= 4 || r.h() >= 4 {
                    let _ = self
                        .against_any_wall(P::Armchair, r, None)
                        .or_else(|| self.against_any_wall(P::RoundTable, r, None));
                }
            }
            Role::Living => {
                let seat = self
                    .against_any_wall(P::Sofa, r, None)
                    .or_else(|| self.against_any_wall(P::Loveseat, r, None));
                if let Some(s) = seat {
                    let seat_side = side_from_rot(s.rotation_deg);
                    let _ = self.in_front(s, P::Rug, 0, r);
                    let _ = self
                        .in_front(s, P::CoffeeTable, 1, r)
                        .or_else(|| self.in_front(s, P::CoffeeTable, 0, r));
                    let tv_side = opposite_side(seat_side);
                    let tv = self
                        .against_wall(P::TvStand, r, tv_side, Some(2))
                        .or_else(|| self.against_any_wall(P::TvStand, r, None));
                    if let Some(t) = tv {
                        self.beside_anchor(t, P::Stereo, r, false);
                    }
                } else {
                    let _ = self.against_any_wall(P::TvStand, r, None);
                }
                if area_scale >= 2 {
                    let _ = self.against_any_wall(P::Loveseat, r, None);
                    self.scatter(P::Armchair, r, area_scale.min(3), 0);
                } else {
                    let _ = self.against_any_wall(P::Armchair, r, None);
                }
                let _ = self.in_corner(P::StandingLamp, r);
                let _ = self.against_any_wall(P::Bookshelf, r, None);
                let _ = self.against_any_wall(P::Planter, r, None);
            }
            Role::Dining => {
                self.center_rug(
                    if r.w() >= 4 && r.h() >= 4 {
                        P::Rug
                    } else {
                        P::RoundRug
                    },
                    r,
                );
                let placed_table = if r.w() >= 4 && r.h() >= 4 {
                    let cx = r.x0 + (r.w() - 2) / 2;
                    let cy = r.y0 + (r.h() - 2) / 2;
                    self.try_place(P::DiningTable, cx, cy, 0, None, r).is_some()
                } else {
                    false
                };
                if !placed_table {
                    let cx = r.x0 + r.w() / 2;
                    let cy = r.y0 + r.h() / 2;
                    if self.try_place(P::RoundTable, cx, cy, 0, None, r).is_none() {
                        let _ = self
                            .against_any_wall(P::DiningTable, r, None)
                            .or_else(|| self.against_any_wall(P::RoundTable, r, None));
                    }
                }
                let _ = self
                    .against_any_wall(P::Dresser, r, None)
                    .or_else(|| self.against_any_wall(P::Bookshelf, r, None));
                let _ = self.in_corner(P::StandingLamp, r);
                let _ = self.in_corner(P::Planter, r);
            }
            Role::Kitchen | Role::BackKitchen => {
                let start_side = self.rng.range(0, 4) as usize;
                let mut counter_side = Side::ALL[start_side];
                let mut placed_counter = false;
                'wall_loop: for i in 0..4 {
                    let s = Side::ALL[(start_side + i) % 4];
                    let run = match s {
                        Side::North | Side::South => r.w(),
                        _ => r.h(),
                    };
                    let max_len = (run - 1).clamp(2, 4);
                    for len in (2..=max_len).rev() {
                        if self
                            .against_wall(P::KitchenCounter, r, s, Some(len))
                            .is_some()
                        {
                            counter_side = s;
                            placed_counter = true;
                            break 'wall_loop;
                        }
                    }
                }
                if !placed_counter
                    && let Some(p) = self.against_any_wall(P::KitchenCounter, r, None)
                {
                    counter_side = side_from_rot(p.rotation_deg);
                }
                let _ = self
                    .against_wall(P::Stove, r, counter_side, None)
                    .or_else(|| self.against_any_wall(P::Stove, r, None));
                let _ = self
                    .against_wall(P::Fridge, r, counter_side, None)
                    .or_else(|| self.against_any_wall(P::Fridge, r, None));
                if role == Role::Kitchen {
                    if r.area() >= 18 {
                        let _ = self
                            .against_any_wall(P::LinenCloset, r, None)
                            .or_else(|| self.against_any_wall(P::MedicalCounter, r, Some(2)));
                    }
                    let mut placed_island = false;
                    if r.w() >= 6 && r.h() >= 6 {
                        let cx = r.x0 + (r.w() - 2) / 2;
                        let cy = r.y0 + (r.h() - 2) / 2;
                        if self.free(cx - 1, cy - 1, 4, 4, r)
                            && let Some(bar) = self.try_place(P::BarCounter, cx, cy, 0, Some(2), r)
                        {
                            placed_island = true;
                            let _ = self.try_place(P::BarStool, bar.x, bar.y + 1, 0, None, r);
                            let _ = self.try_place(P::BarStool, bar.x + 1, bar.y + 1, 0, None, r);
                        }
                    }
                    if !placed_island && r.area() >= 20 {
                        self.scatter(P::RoundTable, r, 1, 0);
                    }
                    let _ = self.in_corner(P::TrashCan, r);
                    let _ = self.in_corner(P::FireExtinguisher, r);
                } else {
                    self.scatter(P::WoodCrate, r, (area_scale * 2).min(6), 0);
                    let _ = self.in_corner(P::FireExtinguisher, r);
                }
            }
            Role::MasterBedroom | Role::Bedroom => {
                self.furnish_bedroom(r, role == Role::MasterBedroom)
            }
            Role::Closet => {
                let _ = self
                    .against_any_wall(P::Wardrobe, r, None)
                    .or_else(|| self.against_any_wall(P::Dresser, r, None))
                    .or_else(|| self.against_any_wall(P::SteelShelving, r, None));
                self.center_rug(P::RunnerRug, r);
                self.floor_clutter(P::FloorClothes, r);
            }
            Role::Laundry => {
                let _ = self.against_any_wall(P::WasherDryer, r, None);
                let _ = self.against_any_wall(P::MedicalCounter, r, Some(r.w().clamp(1, 2)));
                let _ = self.against_any_wall(P::LinenCloset, r, None);
                self.floor_clutter(P::FloorClothes, r);
                let _ = self.in_corner(P::FireExtinguisher, r);
            }
            Role::Sunroom => {
                self.center_rug(P::RoundRug, r);
                let _ = self
                    .against_any_wall(P::Loveseat, r, None)
                    .or_else(|| self.against_any_wall(P::Armchair, r, None));
                self.scatter(P::RoundTable, r, 1, 0);
                let _ = self.in_corner(P::Planter, r);
                let _ = self.in_corner(P::Planter, r);
                let _ = self.in_corner(P::StandingLamp, r);
            }
            Role::Hall => {
                let rot = if r.h() > r.w() { 90 } else { 0 };
                let len = (r.w().max(r.h()) - 1).clamp(2, 5);
                let probe = make_prop(P::RunnerRug, 0, 0, rot, Some(len), 0);
                let rx = r.x0 + (r.w() - probe.w_cells).max(0) / 2;
                let ry = r.y0 + (r.h() - probe.h_cells).max(0) / 2;
                let _ = self.try_place(P::RunnerRug, rx, ry, rot, Some(len), r);
                let _ = self.in_corner(P::Planter, r);
            }
            Role::Office => {
                self.center_rug(
                    if r.w() >= 4 && r.h() >= 4 {
                        P::Rug
                    } else {
                        P::RoundRug
                    },
                    r,
                );
                let desk = self
                    .against_any_wall(P::DeskL, r, None)
                    .or_else(|| self.against_any_wall(P::Desk, r, None));
                if let Some(d) = desk {
                    self.beside_anchor(d, P::DeskLamp, r, false);
                    self.attach_desk_clutter(d, r);
                }
                let _ = self.against_any_wall(P::Bookshelf, r, None);
                if r.area() >= 18 {
                    let _ = self.against_any_wall(P::Bookshelf, r, None);
                }
                let _ = self.against_any_wall(P::FilingCabinet, r, None);
                let _ = self
                    .against_any_wall(P::Armchair, r, None)
                    .or_else(|| self.against_any_wall(P::Loveseat, r, None));
                let _ = self.in_corner(P::StandingLamp, r);
                let _ = self.in_corner(P::TrashCan, r);
                let _ = self.in_corner(P::Planter, r);
            }
            Role::Library => {
                self.center_rug(
                    if r.w() >= 4 && r.h() >= 4 {
                        P::Rug
                    } else {
                        P::RoundRug
                    },
                    r,
                );
                let shelf_count = (r.area() / 8).clamp(2, 5);
                for _ in 0..shelf_count {
                    let _ = self.against_any_wall(P::Bookshelf, r, None);
                }
                let lounge = self
                    .against_any_wall(P::Loveseat, r, None)
                    .or_else(|| self.against_any_wall(P::Armchair, r, None));
                if let Some(l) = lounge {
                    let _ = self.in_front(l, P::CoffeeTable, 0, r);
                }
                let _ = self.against_any_wall(P::Armchair, r, None);
                if r.area() >= 16
                    && let Some(d) = self.against_any_wall(P::Desk, r, None)
                {
                    self.beside_anchor(d, P::DeskLamp, r, false);
                    self.attach_desk_clutter(d, r);
                }
                let _ = self.in_corner(P::StandingLamp, r);
                let _ = self.in_corner(P::Planter, r);
            }
            Role::Garage => {
                let shelves = if r.area() >= 20 { 2 } else { 1 };
                for _ in 0..shelves {
                    let _ = self.against_any_wall(P::SteelShelving, r, None);
                }
                let _ = self
                    .against_any_wall(P::TestBench, r, None)
                    .or_else(|| self.against_any_wall(P::SolderingStation, r, None));
                if r.area() >= 18 {
                    let _ = self.against_any_wall(P::Printer3dStation, r, None);
                }
                let _ = self.against_any_wall(P::AirCompressor, r, None);
                let crates = if r.area() >= 20 { 2 } else { 1 };
                for _ in 0..crates {
                    let _ = self.against_any_wall(P::WoodCrate, r, None);
                }
                let _ = self.in_corner(P::TrashCan, r);
                let _ = self.in_corner(P::FireExtinguisher, r);
            }
            Role::HomeGym => {
                self.center_rug(P::RunnerRug, r);
                let mat2_x = r.x0 + 1;
                let mat2_y = r.y0 + 1;
                if r.w() >= 4 && r.h() >= 4 {
                    let _ = self.try_place(P::RunnerRug, mat2_x, mat2_y, 0, None, r);
                }
                let bench_count = if r.area() >= 18 { 2 } else { 1 };
                for idx in 0..bench_count {
                    if let Some(bench) = self.against_any_wall(P::Cot, r, None)
                        && idx == 0
                    {
                        self.beside_anchor(bench, P::TelemetryMonitor, r, false);
                    }
                }
                if let Some(tv) = self.against_any_wall(P::TvStand, r, None) {
                    self.beside_anchor(tv, P::Stereo, r, false);
                }
                let _ = self.in_corner(P::WaterCooler, r);
                let _ = self.against_any_wall(P::LinenCloset, r, None);
                let _ = self.in_corner(P::Planter, r);
            }
            Role::HalfBath => {
                let _ = self.against_any_wall(P::Toilet, r, None);
                let _ = self.against_any_wall(P::BathroomSink, r, None);
                if r.w() >= 3 && r.h() >= 3 {
                    self.center_rug(P::RunnerRug, r);
                }
                let _ = self.in_corner(P::TrashCan, r);
                let _ = self.in_corner(P::Planter, r);
            }
            Role::Bathroom | Role::MasterBath | Role::Restroom => {
                // Guarantee core sanitary fixtures first so tight layouts never drop toilet or vanity.
                let _ = self.against_any_wall(P::Toilet, r, None);
                if role == Role::Restroom {
                    let _ = self.against_any_wall(P::Toilet, r, None);
                }
                let _ = self.against_any_wall(
                    P::BathroomSink,
                    r,
                    if role == Role::MasterBath && r.w() >= 4 {
                        Some(2)
                    } else {
                        None
                    },
                );
                if role != Role::Restroom {
                    if role == Role::MasterBath {
                        if r.w() >= 3 && r.h() >= 3 {
                            let kind = if self.rng.chance(1, 2) {
                                P::SpaBath
                            } else {
                                P::Bathtub
                            };
                            let _ = self
                                .against_any_wall(kind, r, None)
                                .or_else(|| self.against_any_wall(P::ShowerStall, r, None));
                            if self.rng.chance(2, 3) {
                                let _ = self.against_any_wall(P::ShowerStall, r, None);
                            }
                        } else {
                            let _ = self
                                .against_any_wall(P::Bathtub, r, None)
                                .or_else(|| self.against_any_wall(P::ShowerStall, r, None));
                        }
                    } else if r.area() <= 8 || self.rng.chance(1, 2) {
                        let _ = self
                            .against_any_wall(P::ShowerStall, r, None)
                            .or_else(|| self.against_any_wall(P::Bathtub, r, None));
                    } else {
                        let _ = self
                            .against_any_wall(P::Bathtub, r, None)
                            .or_else(|| self.against_any_wall(P::ShowerStall, r, None));
                    }
                    let _ = self.against_any_wall(P::LinenCloset, r, None);
                }
                self.center_rug(P::RunnerRug, r);
                let _ = self.in_corner(P::TrashCan, r);
                if role == Role::MasterBath {
                    let _ = self.in_corner(P::Planter, r);
                }
            }
            Role::Storage => {
                for _ in 0..area_scale.min(3) {
                    let _ = self.against_any_wall(P::SteelShelving, r, None);
                }
                for _ in 0..(area_scale * 3 + 1).min(10) {
                    let _ = self.against_any_wall(P::WoodCrate, r, None);
                }
                let _ = self.against_any_wall(P::DrumCluster, r, None);
                let _ = self.in_corner(P::FireExtinguisher, r);
            }
            Role::BarHall => self.furnish_bar_hall(r),
            Role::Bullpen => {
                let side = if self.rng.chance(1, 2) {
                    Side::North
                } else {
                    Side::South
                };
                let lounge = self
                    .against_wall(P::Sofa, r, side, None)
                    .or_else(|| self.against_wall(P::Loveseat, r, side, None));
                if let Some(s) = lounge {
                    let _ = self.in_front(s, P::Rug, 0, r);
                    let _ = self
                        .in_front(s, P::CoffeeTable, 1, r)
                        .or_else(|| self.in_front(s, P::CoffeeTable, 0, r));
                }
                let prev = self.props.len();
                self.scatter(P::DeskT, r, (r.area() / 48).clamp(1, 4) as usize, 0);
                let _ = self.against_any_wall(P::DeskL, r, None);
                let _ = self.against_any_wall(P::DeskL, r, None);
                let desk_count = ((r.area() / 22).clamp(1, 8)) as usize;
                self.scatter(P::Desk, r, desk_count, 0);
                let desks_added: Vec<MapPropPlacement> = self.props[prev..].to_vec();
                for d in desks_added {
                    self.attach_desk_clutter(d, r);
                }
                let _ = self.against_any_wall(P::FilingCabinet, r, None);
                let _ = self.against_any_wall(P::FilingCabinet, r, None);
                let _ = self.against_any_wall(P::Bookshelf, r, None);
                let _ = self.in_corner(P::WaterCooler, r);
                let _ = self.in_corner(P::TrashCan, r);
                let _ = self.in_corner(P::FireExtinguisher, r);
                self.scatter(P::Planter, r, (r.area() / 60).clamp(1, 4) as usize, 0);
            }
            Role::ConferenceRoom => {
                let cx = r.x0 + (r.w() - 2) / 2;
                let cy = r.y0 + (r.h() - 2) / 2;
                if r.w() >= 6 && r.h() >= 5 {
                    let _ = self.try_place(P::DiningTable, cx - 1, cy, 0, None, r);
                    let _ = self.try_place(P::DiningTable, cx + 1, cy, 0, None, r);
                } else if let Some(dt) = self.try_place(P::DiningTable, cx, cy, 0, None, r) {
                    self.attach_desk_clutter(dt, r);
                }
                if area_scale >= 2 {
                    self.scatter(P::Armchair, r, 2, 0);
                }
                let _ = self.against_any_wall(P::Bookshelf, r, None);
                let _ = self.in_corner(P::WaterCooler, r);
                let _ = self.in_corner(P::TrashCan, r);
                let _ = self.against_any_wall(P::Planter, r, None);
            }
            Role::Breakroom => {
                let side = if r.w() >= r.h() {
                    Side::North
                } else {
                    Side::East
                };
                let run = match side {
                    Side::North | Side::South => r.w(),
                    _ => r.h(),
                };
                let _ = self.against_wall(P::KitchenCounter, r, side, Some((run - 2).clamp(2, 4)));
                let _ = self
                    .against_wall(P::Fridge, r, side, None)
                    .or_else(|| self.against_any_wall(P::Fridge, r, None));
                let _ = self.in_corner(P::WaterCooler, r);
                let _ = self.in_corner(P::TrashCan, r);
                let _ = self.in_corner(P::FireExtinguisher, r);
                let prev = self.props.len();
                self.scatter(P::RoundTable, r, (r.area() / 24).clamp(2, 6) as usize, 0);
                let tbls: Vec<MapPropPlacement> = self.props[prev..].to_vec();
                for t in tbls.into_iter().take(2) {
                    self.attach_desk_clutter(t, r);
                }
            }
            Role::ServerCloset => {
                for _ in 0..(area_scale * 2).clamp(2, 6) {
                    let _ = self.against_any_wall(P::ServerRack, r, None);
                }
                let _ = self.against_any_wall(P::AirCompressor, r, None);
                let _ = self.against_any_wall(P::TestBench, r, None);
                if let Some(d) = self.against_any_wall(P::Desk, r, None) {
                    self.attach_desk_clutter(d, r);
                }
                let _ = self.in_corner(P::FireExtinguisher, r);
            }
        }
    }

    fn furnish_bedroom(&mut self, r: Room, is_master: bool) {
        use PropKind as P;
        let bed_kind = if !is_master && r.area() <= 14 && self.rng.chance(1, 2) {
            P::SingleBed
        } else {
            P::Bed
        };
        let bed = self
            .against_any_wall(bed_kind, r, None)
            .or_else(|| self.against_any_wall(P::Bed, r, None));
        if let Some(b) = bed {
            let bed_side = side_from_rot(b.rotation_deg);
            self.beside_anchor(b, P::Nightstand, r, is_master);
            if r.w() >= 3 && r.h() >= 3 {
                let _ = self.in_front(b, P::RoundRug, 0, r);
            }
            let opp = opposite_side(bed_side);
            let _ = self
                .against_wall(P::Dresser, r, opp, None)
                .or_else(|| self.against_any_wall(P::Dresser, r, None));
        } else {
            let _ = self.against_any_wall(P::Dresser, r, None);
        }
        let _ = self.against_any_wall(P::Nightstand, r, None);
        if r.area() >= 14 {
            let _ = self.against_any_wall(P::Wardrobe, r, None);
        }
        if is_master {
            if let Some(tv) = self.against_any_wall(P::TvStand, r, None) {
                self.beside_anchor(tv, P::Stereo, r, false);
            }
            let _ = self.against_any_wall(P::Armchair, r, None);
        } else if let Some(d) = self.against_any_wall(P::Desk, r, None) {
            self.beside_anchor(d, P::DeskLamp, r, false);
            self.attach_desk_clutter(d, r);
        }
        let _ = self.in_corner(P::StandingLamp, r);
        let _ = self.in_corner(P::Planter, r);
        if self.rng.chance(1, 2) {
            let clutter = if !is_master && self.rng.chance(1, 2) {
                P::FloorToys
            } else {
                P::FloorClothes
            };
            self.floor_clutter(clutter, r);
        }
    }

    pub(crate) fn furnish_patio(&mut self, r: Room) {
        use PropKind as P;
        self.scatter(P::RoundTable, r, (r.area() / 32).clamp(1, 4) as usize, 0);
        self.scatter(P::ParkBench, r, (r.area() / 48).clamp(1, 3) as usize, 0);
        self.scatter(P::Planter, r, (r.area() / 36).clamp(1, 4) as usize, 0);
        let _ = self.against_any_wall(P::Bush, r, None);
    }

    fn furnish_bar_hall(&mut self, r: Room) {
        use PropKind as P;
        let len = (r.w() - 4).clamp(3, 10);
        let bx = r.x0 + 1 + self.rng.range(0, (r.w() - len - 2).max(1));
        let north = self.rng.chance(1, 2);
        let (shelf_y, counter_y, stool_y, rot) = if north {
            (r.y0, r.y0 + 2, r.y0 + 3, 0)
        } else {
            (r.y1 - 1, r.y1 - 3, r.y1 - 4, 180)
        };
        if r.h() >= 8 {
            let _ = self.try_place(P::BackBarShelf, bx, shelf_y, rot, Some(len), r);
            let aisle_y = (shelf_y + counter_y) / 2;
            for x in bx..bx + len {
                self.blocked[(aisle_y * self.w + x) as usize] = true;
            }
            let _ = self.try_place(P::BarCounter, bx, counter_y, rot, Some(len), r);
            for x in bx..bx + len {
                let _ = self.try_place(P::BarStool, x, stool_y, rot, None, r);
            }
        }
        let far = if north { Side::South } else { Side::North };
        for _ in 0..(r.w() / 5).clamp(3, 6) {
            let _ = self.against_wall(P::Booth, r, far, None);
        }
        self.scatter(P::PoolTable, r, (r.area() / 90).clamp(1, 3) as usize, 0);
        self.scatter(P::RoundTable, r, (r.area() / 32).clamp(4, 10) as usize, 0);
        let _ = self.against_any_wall(P::Jukebox, r, None);
        let _ = self.in_corner(P::FireExtinguisher, r);
        let _ = self.against_any_wall(P::Planter, r, None);
    }
}

/// Emits a room-specific floor finish overlay (`CeramicTileFloor`, `MarbleFloor`, `SteelPlateFloor`,
/// or `CarpetFloor`), returning `None` for rooms that retain the wing's hardwood `InteriorFloor` base.
pub(crate) fn emit_room_floor_surface(
    r: Room,
    role: Role,
    tier: EconomicTier,
    rng: &mut Rng,
) -> Option<MapSurfaceFeature> {
    if r.w() <= 0 || r.h() <= 0 {
        return None;
    }
    let kind = match role {
        Role::MasterBath => Some(SurfaceFeatureKind::MarbleFloor),
        Role::HalfBath => {
            if matches!(
                tier,
                EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion
            ) {
                Some(SurfaceFeatureKind::MarbleFloor)
            } else {
                Some(SurfaceFeatureKind::CeramicTileFloor)
            }
        }
        Role::Garage => Some(SurfaceFeatureKind::StainedConcreteFloor),
        Role::HomeGym => Some(SurfaceFeatureKind::LinoleumCheckFloor),
        Role::ServerCloset => Some(SurfaceFeatureKind::SteelPlateFloor),
        Role::Bathroom
        | Role::Restroom
        | Role::Kitchen
        | Role::BackKitchen
        | Role::Sunroom
        | Role::Laundry => Some(SurfaceFeatureKind::CeramicTileFloor),
        Role::Foyer => {
            if matches!(
                tier,
                EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion
            ) {
                Some(SurfaceFeatureKind::MarbleFloor)
            } else if rng.chance(2, 3) {
                Some(SurfaceFeatureKind::CeramicTileFloor)
            } else {
                None
            }
        }
        Role::MasterBedroom | Role::Bedroom | Role::Closet => {
            if rng.chance(3, 4) {
                Some(SurfaceFeatureKind::CarpetFloor)
            } else {
                None
            }
        }
        // Library, Living, Dining, Office & Hall keep base InteriorFloor hardwood.
        _ => None,
    }?;
    // Encode warm terracotta vs cool ceramic in variant_seed bit 0 for kitchens & sunrooms.
    let raw_seed = (rng.next() & 0xffff_fffe) as u32;
    let variant_seed = match role {
        Role::Kitchen | Role::BackKitchen | Role::Sunroom => raw_seed | 1,
        _ => raw_seed,
    };
    Some(MapSurfaceFeature {
        kind,
        shape: SurfaceShapeKind::Polygon,
        points_cm: vec![
            MapPointCm::new(r.x0 * 100, r.y0 * 100),
            MapPointCm::new(r.x1 * 100, r.y0 * 100),
            MapPointCm::new(r.x1 * 100, r.y1 * 100),
            MapPointCm::new(r.x0 * 100, r.y1 * 100),
        ],
        width_cm: 0,
        border_style_cm: 0,
        variant_seed,
        tint_rgb: None,
    })
}
