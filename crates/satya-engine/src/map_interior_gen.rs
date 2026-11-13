//! # Furnished Modern-Day Interior Generator (`map_interior_gen`)
//!
//! Deterministic, seeded generator for edge-walled interiors ([`InteriorStyle::Apartment`],
//! [`InteriorStyle::Bar`], [`InteriorStyle::Office`]):
//! 1. **Shell**: masonry perimeter edge walls with West/East entry doors and seeded exterior windows.
//! 2. **Rooms**: the apartment uses a BSP split into drywall rooms; the bar has a large hall plus back rooms;
//!    the office has an open bullpen, reception lounge, glass-walled conference room, breakroom & server closet.
//! 3. **Roles & furnishing**: rooms get roles and rule-based multi-cell props (sofas, coffee tables, desks,
//!    conference tables, kitchen counters, bar counters, bookshelves, steel shelving, lockers).
//! 4. **Safety**: door thresholds and spawn pads stay clear. If a tall prop ever disconnects the floor,
//!    tall props are removed until every open cell is reachable again.
//!
//! Props are emitted as continuous [`MapPropPlacement`] footprints with analytical collision and cover.

use satya_schema::{
    EdgeSide, MapPropPlacement, MapWallEdge, MaterialClass, PropKind, WallEdgeKind,
};
use serde::{Deserialize, Serialize};

use crate::map_dressing::{PropObstruction, make_prop, prop_spec};
#[cfg(test)]
use crate::map_dressing::props_share_covered_cell;
use crate::tactical_map::ProceduralSector;
use crate::tactical_map_edges::WallEdgeIndex;
use crate::tactical_map_geom::all_walkable_areas_connected;
#[cfg(test)]
use crate::tactical_map_geom::{TOKEN_RADIUS_M, circle_collides_obstacles};

/// Furnished interior layout family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteriorStyle {
    Apartment,
    Bar,
    Office,
}

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
        if hi_excl <= lo { lo } else { lo + (self.next() % (hi_excl - lo) as u64) as i32 }
    }
    fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next() % den < num
    }
}

/// Half-open cell rectangle `[x0, x1) × [y0, y1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Room {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl Room {
    const fn w(self) -> i32 { self.x1 - self.x0 }
    const fn h(self) -> i32 { self.y1 - self.y0 }
    const fn area(self) -> i32 { self.w() * self.h() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Living,
    Kitchen,
    Bedroom,
    Office,
    Bathroom,
    BarHall,
    Storage,
    Restroom,
    BackKitchen,
    Bullpen,
    ConferenceRoom,
    Breakroom,
    ServerCloset,
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
    /// Rotation that puts a prop's back against this wall.
    const fn rot(self) -> i32 {
        match self {
            Self::North => 0,
            Self::East => 90,
            Self::South => 180,
            Self::West => 270,
        }
    }
}

/// Unit vector a prop's **front** faces for `rotation_deg` (back faces North at `0`).
const fn front_dir(rotation_deg: i32) -> (i32, i32) {
    match rotation_deg.rem_euclid(360) {
        90 => (-1, 0),
        180 => (0, -1),
        270 => (1, 0),
        _ => (0, 1),
    }
}

struct Shell {
    w: i32,
    h: i32,
    rng: Rng,
    edges: Vec<MapWallEdge>,
}

impl Shell {
    fn put(&mut self, x: i32, y: i32, side: EdgeSide, kind: WallEdgeKind, material: MaterialClass) {
        let e = MapWallEdge { x, y, side, kind, material };
        match self.edges.iter_mut().find(|o| o.x == x && o.y == y && o.side == side) {
            Some(o) => *o = e,
            None => self.edges.push(e),
        }
    }

    fn door_kind(&mut self) -> WallEdgeKind {
        if self.rng.chance(1, 2) { WallEdgeKind::DoorOpen } else { WallEdgeKind::DoorClosed }
    }

    /// Styled vertical partition on West edges of column `x` for rows `y0..y1`, with doors at `door_rows`.
    fn partition_v_styled(&mut self, x: i32, y0: i32, y1: i32, door_rows: &[i32], wall: (WallEdgeKind, MaterialClass)) {
        for y in y0..y1 {
            if door_rows.contains(&y) {
                let kind = self.door_kind();
                self.put(x, y, EdgeSide::West, kind, MaterialClass::Drywall);
            } else {
                self.put(x, y, EdgeSide::West, wall.0, wall.1);
            }
        }
    }

    /// Styled horizontal partition on North edges of row `y` for columns `x0..x1`, with doors at `door_cols`.
    fn partition_h_styled(&mut self, y: i32, x0: i32, x1: i32, door_cols: &[i32], wall: (WallEdgeKind, MaterialClass)) {
        for x in x0..x1 {
            if door_cols.contains(&x) {
                let kind = self.door_kind();
                self.put(x, y, EdgeSide::North, kind, MaterialClass::Drywall);
            } else {
                self.put(x, y, EdgeSide::North, wall.0, wall.1);
            }
        }
    }

    /// Drywall partition on West edges of column `x` for rows `y0..y1`, with doors at `door_rows`.
    fn partition_v(&mut self, x: i32, y0: i32, y1: i32, door_rows: &[i32]) {
        self.partition_v_styled(x, y0, y1, door_rows, (WallEdgeKind::Wall, MaterialClass::Drywall));
    }

    /// Drywall partition on North edges of row `y` for columns `x0..x1`, with doors at `door_cols`.
    fn partition_h(&mut self, y: i32, x0: i32, x1: i32, door_cols: &[i32]) {
        self.partition_h_styled(y, x0, x1, door_cols, (WallEdgeKind::Wall, MaterialClass::Drywall));
    }

    fn perimeter_mat(&mut self, wall_mat: MaterialClass) {
        let (w, h, mid_y) = (self.w, self.h, self.h / 2);
        for x in 0..w {
            self.put(x, 0, EdgeSide::North, WallEdgeKind::Wall, wall_mat);
            self.put(x, h, EdgeSide::North, WallEdgeKind::Wall, wall_mat);
        }
        for y in 0..h {
            self.put(0, y, EdgeSide::West, WallEdgeKind::Wall, wall_mat);
            self.put(w, y, EdgeSide::West, WallEdgeKind::Wall, wall_mat);
        }
        self.put(0, mid_y, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SolidWood);
        self.put(w, mid_y, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SolidWood);
    }

    fn perimeter(&mut self) {
        self.perimeter_mat(MaterialClass::Brick);
    }

    /// Seeded exterior windows on perimeter walls, keeping clear of corners, doors, and partition junctions.
    fn windows(&mut self, spacing: i32) {
        let idx = WallEdgeIndex::from_edges(self.w, self.h, &self.edges);
        let (w, h) = (self.w, self.h);
        let off = self.rng.range(0, spacing);
        let mut picks: Vec<(i32, i32, EdgeSide)> = Vec::new();
        let partition_v = |x: i32, y: i32| idx.get(x, y, EdgeSide::West).is_some() && x > 0 && x < w;
        let partition_h = |x: i32, y: i32| idx.get(x, y, EdgeSide::North).is_some() && y > 0 && y < h;
        for x in 1..(w - 1) {
            if (x + off) % spacing == 0 {
                if !partition_v(x, 0) && !partition_v(x + 1, 0) {
                    picks.push((x, 0, EdgeSide::North));
                }
                if !partition_v(x, h - 1) && !partition_v(x + 1, h - 1) {
                    picks.push((x, h, EdgeSide::North));
                }
            }
        }
        for y in 1..(h - 1) {
            if (y + off) % spacing == 0 && (y - h / 2).abs() > 1 {
                if !partition_h(0, y) && !partition_h(0, y + 1) {
                    picks.push((0, y, EdgeSide::West));
                }
                if !partition_h(w - 1, y) && !partition_h(w - 1, y + 1) {
                    picks.push((w, y, EdgeSide::West));
                }
            }
        }
        for (x, y, side) in picks {
            self.put(x, y, side, WallEdgeKind::Window, MaterialClass::Glass);
        }
    }

    fn bsp(&mut self, r: Room, depth: u32, min: i32, max_depth: u32, rooms: &mut Vec<Room>) {
        let can_v = r.w() >= 2 * min;
        let can_h = r.h() >= 2 * min;
        if depth >= max_depth || (!can_v && !can_h) || (depth > 0 && r.area() <= min * min * 2) {
            rooms.push(r);
            return;
        }
        let vertical = match (can_v, can_h) {
            (true, true) if r.w() * 4 > r.h() * 5 => true,
            (true, true) if r.h() * 4 > r.w() * 5 => false,
            (true, true) => self.rng.chance(1, 2),
            (v, _) => v,
        };
        if vertical {
            let sx = self.rng.range(r.x0 + min, r.x1 - min + 1);
            let door = self.rng.range(r.y0, r.y1);
            self.partition_v(sx, r.y0, r.y1, &[door]);
            self.bsp(Room { x1: sx, ..r }, depth + 1, min, max_depth, rooms);
            self.bsp(Room { x0: sx, ..r }, depth + 1, min, max_depth, rooms);
        } else {
            let sy = self.rng.range(r.y0 + min, r.y1 - min + 1);
            let door = self.rng.range(r.x0, r.x1);
            self.partition_h(sy, r.x0, r.x1, &[door]);
            self.bsp(Room { y1: sy, ..r }, depth + 1, min, max_depth, rooms);
            self.bsp(Room { y0: sy, ..r }, depth + 1, min, max_depth, rooms);
        }
    }
}

struct Furnisher {
    w: i32,
    h: i32,
    blocked: Vec<bool>,
    props: Vec<MapPropPlacement>,
    rng: Rng,
}

impl Furnisher {
    fn free(&self, x: i32, y: i32, fw: i32, fh: i32, r: Room) -> bool {
        x >= r.x0
            && y >= r.y0
            && x + fw <= r.x1
            && y + fh <= r.y1
            && (y..y + fh).all(|yy| (x..x + fw).all(|xx| !self.blocked[(yy * self.w + xx) as usize]))
    }

    fn free_prop(&self, p: &MapPropPlacement, r: Room) -> bool {
        p.x >= r.x0
            && p.y >= r.y0
            && p.x + p.w_cells <= r.x1
            && p.y + p.h_cells <= r.y1
            && (p.y..p.y + p.h_cells).all(|yy| {
                (p.x..p.x + p.w_cells).all(|xx| !p.covers(xx, yy) || !self.blocked[(yy * self.w + xx) as usize])
            })
    }

    fn try_place(&mut self, kind: PropKind, x: i32, y: i32, rot: i32, len: Option<i32>, r: Room) -> Option<MapPropPlacement> {
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

    fn against_wall(&mut self, kind: PropKind, r: Room, side: Side, len: Option<i32>) -> Option<MapPropPlacement> {
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
            .find_map(|(x, y)| self.try_place(kind, x, y, rot, len, r))
    }

    fn against_any_wall(&mut self, kind: PropKind, r: Room, len: Option<i32>) -> Option<MapPropPlacement> {
        let start = self.rng.range(0, 4) as usize;
        (0..4).find_map(|i| self.against_wall(kind, r, Side::ALL[(start + i) % 4], len))
    }

    /// Places `kind` with the same rotation, centered in front of `anchor` after a `gap` of free cells.
    fn in_front(&mut self, anchor: MapPropPlacement, kind: PropKind, gap: i32, r: Room) -> Option<MapPropPlacement> {
        let rot = anchor.rotation_deg;
        let probe = make_prop(kind, 0, 0, rot, None, 0);
        let (fw, fh) = (probe.w_cells, probe.h_cells);
        let (x, y) = match front_dir(rot) {
            (0, 1) => (anchor.x + (anchor.w_cells - fw) / 2, anchor.y + anchor.h_cells + gap),
            (0, _) => (anchor.x + (anchor.w_cells - fw) / 2, anchor.y - gap - fh),
            (1, _) => (anchor.x + anchor.w_cells + gap, anchor.y + (anchor.h_cells - fh) / 2),
            _ => (anchor.x - gap - fw, anchor.y + (anchor.h_cells - fh) / 2),
        };
        self.try_place(kind, x, y, rot, None, r)
    }

    /// Scatters up to `count` props with a one-cell clearance ring inside `r`.
    fn scatter(&mut self, kind: PropKind, r: Room, count: usize, rot: i32) {
        let probe = make_prop(kind, 0, 0, rot, None, 0);
        let mut placed = 0;
        for _ in 0..(count * 12) {
            if placed >= count {
                break;
            }
            let x = self.rng.range(r.x0 + 1, r.x1 - probe.w_cells);
            let y = self.rng.range(r.y0 + 1, r.y1 - probe.h_cells);
            let ring = Room { x0: r.x0, y0: r.y0, x1: r.x1, y1: r.y1 };
            if self.free(x - 1, y - 1, probe.w_cells + 2, probe.h_cells + 2, ring)
                && self.try_place(kind, x, y, rot, None, r).is_some()
            {
                placed += 1;
            }
        }
    }

    fn furnish(&mut self, role: Role, r: Room) {
        use PropKind as P;
        match role {
            Role::Living => {
                let seat = self.against_any_wall(P::Sofa, r, None).or_else(|| self.against_any_wall(P::Loveseat, r, None));
                if let Some(s) = seat {
                    let _ = self.in_front(s, P::Rug, 0, r);
                    let _ = self.in_front(s, P::CoffeeTable, 1, r).or_else(|| self.in_front(s, P::CoffeeTable, 0, r));
                }
                let _ = self.against_any_wall(P::Armchair, r, None);
                let _ = self.against_any_wall(P::Bookshelf, r, None);
                let _ = self.against_any_wall(P::Planter, r, None);
            }
            Role::Kitchen | Role::BackKitchen => {
                let side = Side::ALL[self.rng.range(0, 4) as usize];
                let run = match side { Side::North | Side::South => r.w(), _ => r.h() };
                let len = (run - 2).clamp(2, 4);
                let _ = self.against_wall(P::KitchenCounter, r, side, Some(len));
                let _ = self.against_wall(P::Stove, r, side, None);
                let _ = self.against_wall(P::Fridge, r, side, None).or_else(|| self.against_any_wall(P::Fridge, r, None));
                if role == Role::Kitchen {
                    self.scatter(P::DiningTable, r, 1, 0);
                } else {
                    self.scatter(P::WoodCrate, r, 2, 0);
                }
            }
            Role::Bedroom => {
                let _ = self.against_any_wall(P::Bed, r, None);
                let _ = self.against_any_wall(P::Wardrobe, r, None);
                let _ = self.against_any_wall(P::Desk, r, None);
            }
            Role::Office => {
                let _ = self
                    .against_any_wall(P::DeskL, r, None)
                    .or_else(|| self.against_any_wall(P::Desk, r, None));
                let _ = self.against_any_wall(P::Desk, r, None);
                let _ = self.against_any_wall(P::Bookshelf, r, None);
                let _ = self.against_any_wall(P::Planter, r, None);
            }
            Role::Bathroom | Role::Restroom => {
                if role == Role::Bathroom {
                    let _ = self.against_any_wall(P::Bathtub, r, None);
                }
                let _ = self.against_any_wall(P::Toilet, r, None);
                if role == Role::Restroom {
                    let _ = self.against_any_wall(P::Toilet, r, None);
                }
            }
            Role::Storage => {
                let _ = self.against_any_wall(P::SteelShelving, r, None);
                for _ in 0..4 {
                    let _ = self.against_any_wall(P::WoodCrate, r, None);
                }
                let _ = self.against_any_wall(P::DrumCluster, r, None);
            }
            Role::BarHall => self.furnish_bar_hall(r),
            Role::Bullpen => {
                let side = if self.rng.chance(1, 2) { Side::North } else { Side::South };
                let lounge = self
                    .against_wall(P::Sofa, r, side, None)
                    .or_else(|| self.against_wall(P::Loveseat, r, side, None));
                if let Some(s) = lounge {
                    let _ = self.in_front(s, P::Rug, 0, r);
                    let _ = self
                        .in_front(s, P::CoffeeTable, 1, r)
                        .or_else(|| self.in_front(s, P::CoffeeTable, 0, r));
                }
                self.scatter(P::DeskT, r, if r.area() >= 60 { 2 } else { 1 }, 0);
                let _ = self.against_any_wall(P::DeskL, r, None);
                let _ = self.against_any_wall(P::DeskL, r, None);
                let desk_count = ((r.area() / 22).clamp(1, 4)) as usize;
                self.scatter(P::Desk, r, desk_count, 0);
                let _ = self.against_any_wall(P::Desk, r, None);
                let _ = self.against_any_wall(P::Bookshelf, r, None);
                let _ = self.against_any_wall(P::Planter, r, None);
            }
            Role::ConferenceRoom => {
                let cx = r.x0 + (r.w() - 2) / 2;
                let cy = r.y0 + (r.h() - 2) / 2;
                if r.w() >= 6 && r.h() >= 5 {
                    let _ = self.try_place(P::DiningTable, cx - 1, cy, 0, None, r);
                    let _ = self.try_place(P::DiningTable, cx + 1, cy, 0, None, r);
                } else {
                    let _ = self.try_place(P::DiningTable, cx, cy, 0, None, r);
                }
                let _ = self.against_any_wall(P::Bookshelf, r, None);
                let _ = self.against_any_wall(P::Planter, r, None);
            }
            Role::Breakroom => {
                let side = if r.w() >= r.h() { Side::North } else { Side::East };
                let run = match side {
                    Side::North | Side::South => r.w(),
                    _ => r.h(),
                };
                let _ = self.against_wall(P::KitchenCounter, r, side, Some((run - 2).clamp(2, 4)));
                let _ = self
                    .against_wall(P::Fridge, r, side, None)
                    .or_else(|| self.against_any_wall(P::Fridge, r, None));
                self.scatter(P::RoundTable, r, 2, 0);
            }
            Role::ServerCloset => {
                let _ = self.against_any_wall(P::SteelShelving, r, None);
                let _ = self.against_any_wall(P::SteelShelving, r, None);
                let _ = self.against_any_wall(P::Desk, r, None);
                let _ = self.against_any_wall(P::WoodCrate, r, None);
            }
        }
    }

    /// Back-bar shelf → bartender aisle → long bar counter → stool row, then booths, pool & bistro tables.
    fn furnish_bar_hall(&mut self, r: Room) {
        use PropKind as P;
        let len = (r.w() - 4).clamp(3, 8);
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
        for _ in 0..3 {
            let _ = self.against_wall(P::Booth, r, far, None);
        }
        self.scatter(P::PoolTable, r, 1, 0);
        self.scatter(P::RoundTable, r, 4, 0);
        let _ = self.against_any_wall(P::Jukebox, r, None);
        let _ = self.against_any_wall(P::Planter, r, None);
    }
}

/// Generates a furnished edge-walled interior sector (`width × height` cells, already clamped by caller).
#[must_use]
pub fn generate_furnished_interior(style: InteriorStyle, width: i32, height: i32, seed: i64) -> ProceduralSector {
    let (w, h) = (width, height);
    let mid_y = h / 2;
    let salt = match style {
        InteriorStyle::Apartment => 0xA9A7,
        InteriorStyle::Bar => 0xBA77,
        InteriorStyle::Office => 0x0FF1,
    };
    let mut shell = Shell { w, h, rng: Rng((seed as u64) ^ salt), edges: Vec::new() };
    match style {
        InteriorStyle::Office => shell.perimeter_mat(MaterialClass::ConcreteBlock),
        InteriorStyle::Apartment | InteriorStyle::Bar => shell.perimeter(),
    }

    let whole = Room { x0: 0, y0: 0, x1: w, y1: h };
    let mut rooms_roles: Vec<(Room, Role)> = Vec::new();
    match style {
        InteriorStyle::Apartment => {
            let mut rooms = Vec::new();
            shell.bsp(whole, 0, 4, 3, &mut rooms);
            rooms.sort_by_key(|r| -r.area());
            let n = rooms.len();
            for (i, r) in rooms.into_iter().enumerate() {
                let role = match i {
                    0 => Role::Living,
                    1 => Role::Kitchen,
                    _ if i == n - 1 && n >= 4 => Role::Bathroom,
                    _ if i % 2 == 0 => Role::Bedroom,
                    _ => Role::Office,
                };
                rooms_roles.push((r, role));
            }
            shell.windows(4);
        }
        InteriorStyle::Bar => {
            let strip_w = (w * 3 / 10).max(4);
            let sx = w - strip_w;
            let n_back: usize = if h >= 12 { 3 } else { 2 };
            let mut cuts = vec![0];
            for i in 1..n_back {
                let base = h * i as i32 / n_back as i32;
                cuts.push((base + shell.rng.range(-1, 2)).clamp(3, h - 3));
            }
            cuts.push(h);
            let mut doors = Vec::new();
            const BACK_ROLES: [Role; 4] = [Role::Storage, Role::Restroom, Role::BackKitchen, Role::Office];
            for i in 0..n_back {
                let (y0, y1) = (cuts[i], cuts[i + 1]);
                doors.push(shell.rng.range(y0 + 1, (y1 - 1).max(y0 + 2)));
                if i > 0 {
                    shell.partition_h(y0, sx, w, &[]);
                }
                rooms_roles.push((Room { x0: sx, y0, x1: w, y1 }, BACK_ROLES[i % BACK_ROLES.len()]));
            }
            shell.partition_v(sx, 0, h, &doors);
            rooms_roles.insert(0, (Room { x0: 0, y0: 0, x1: sx, y1: h }, Role::BarHall));
            shell.windows(3);
        }
        InteriorStyle::Office => {
            if w >= 12 {
                let sx = (w * 6 / 10).clamp(6, w - 4);
                let n_east: usize = if h >= 12 { 3 } else { 2 };
                let mut cuts = vec![0];
                for i in 1..n_east {
                    let base = h * i as i32 / n_east as i32;
                    cuts.push((base + shell.rng.range(-1, 2)).clamp(3, h - 3));
                }
                cuts.push(h);
                const EAST_ROLES: [Role; 3] = [Role::ConferenceRoom, Role::Breakroom, Role::ServerCloset];
                for i in 0..n_east {
                    let (y0, y1) = (cuts[i], cuts[i + 1]);
                    let door = shell.rng.range(y0 + 1, (y1 - 1).max(y0 + 2));
                    if i > 0 {
                        shell.partition_h_styled(y0, sx, w, &[], (WallEdgeKind::Wall, MaterialClass::Drywall));
                    }
                    if i == 0 {
                        // Glass-walled conference suite overlooking the bullpen with drywall corners & door.
                        shell.partition_v_styled(sx, y0, y1, &[door], (WallEdgeKind::Window, MaterialClass::Glass));
                        shell.put(sx, y0, EdgeSide::West, WallEdgeKind::Wall, MaterialClass::Drywall);
                        if y1 - y0 >= 3 {
                            let kind = if door == y1 - 1 { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall };
                            shell.put(sx, y1 - 1, EdgeSide::West, kind, MaterialClass::Drywall);
                        }
                    } else {
                        shell.partition_v(sx, y0, y1, &[door]);
                    }
                    rooms_roles.push((Room { x0: sx, y0, x1: w, y1 }, EAST_ROLES[i % EAST_ROLES.len()]));
                }
                rooms_roles.insert(0, (Room { x0: 0, y0: 0, x1: sx, y1: h }, Role::Bullpen));
            } else {
                let sx = (w / 2).clamp(4, w - 3);
                let door = mid_y.clamp(1, h - 2);
                shell.partition_v(sx, 0, h, &[door]);
                rooms_roles.push((Room { x0: 0, y0: 0, x1: sx, y1: h }, Role::Bullpen));
                rooms_roles.push((Room { x0: sx, y0: 0, x1: w, y1: h }, Role::ServerCloset));
            }
            shell.windows(3);
        }
    }

    // Reserve spawn pads and both sides of every door so thresholds stay walkable.
    let mut furn = Furnisher { w, h, blocked: vec![false; (w * h) as usize], props: Vec::new(), rng: Rng(shell.rng.next()) };
    let reserve = |f: &mut Furnisher, x: i32, y: i32| {
        if x >= 0 && y >= 0 && x < f.w && y < f.h {
            f.blocked[(y * f.w + x) as usize] = true;
        }
    };
    for &(sx, sy) in &[(2, mid_y), (2, (mid_y - 1).max(1)), (w - 3, mid_y), (w - 3, (mid_y + 1).min(h - 2))] {
        reserve(&mut furn, sx, sy);
    }
    for e in &shell.edges {
        if matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen) {
            match e.side {
                EdgeSide::West => {
                    reserve(&mut furn, e.x - 1, e.y);
                    reserve(&mut furn, e.x, e.y);
                }
                EdgeSide::North => {
                    reserve(&mut furn, e.x, e.y - 1);
                    reserve(&mut furn, e.x, e.y);
                }
            }
        }
    }
    for &(r, role) in &rooms_roles {
        furn.furnish(role, r);
    }

    // Connectivity safeguard: drop solid obstacles (tallest/newest first) until every open area is reachable.
    let mut props = furn.props;
    loop {
        if all_walkable_areas_connected(w, h, &shell.edges, &props) {
            return ProceduralSector { wall_edges: shell.edges, props };
        }
        match props
            .iter()
            .rposition(|p| prop_spec(p.kind).obstruction == PropObstruction::FullObstacle)
        {
            Some(i) => {
                props.remove(i);
            }
            None => return ProceduralSector { wall_edges: shell.edges, props },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tactical_map::{GaitReachQuery, TacticalSceneView, compute_reachable_continuous};
    use crate::environment_vehicles::CombatMovementGait;

    #[test]
    fn interiors_are_deterministic_furnished_and_fully_connected() {
        for style in [InteriorStyle::Apartment, InteriorStyle::Bar, InteriorStyle::Office] {
            for seed in [1_i64, 42, 179, 9001] {
                let a = generate_furnished_interior(style, 20, 14, seed);
                assert_eq!(a, generate_furnished_interior(style, 20, 14, seed), "{style:?} deterministic");
                assert!(a.props.len() >= 5, "{style:?}/{seed}: only {} props", a.props.len());
                assert!(a.wall_edges.iter().any(|e| e.kind == WallEdgeKind::Window));
                assert!(a.wall_edges.iter().any(|e| e.material == MaterialClass::Drywall));
                assert!(all_walkable_areas_connected(20, 14, &a.wall_edges, &a.props));
                for &(x, y) in &[(2, 7), (2, 6), (17, 7), (17, 8)] {
                    assert!(
                        !circle_collides_obstacles(
                            20.0,
                            14.0,
                            &a.wall_edges,
                            &a.props,
                            x as f32 + 0.5,
                            y as f32 + 0.5,
                            TOKEN_RADIUS_M,
                        ),
                        "spawn pad ({x},{y})"
                    );
                }
                // No two solid props overlap.
                for (i, p) in a.props.iter().enumerate() {
                    for q in &a.props[i + 1..] {
                        if prop_spec(p.kind).floor_layer || prop_spec(q.kind).floor_layer {
                            continue;
                        }
                        let overlap = props_share_covered_cell(p, q);
                        assert!(!overlap, "{style:?}/{seed}: {p:?} overlaps {q:?}");
                    }
                }
            }
        }
        let bar = generate_furnished_interior(InteriorStyle::Bar, 20, 14, 42);
        let counter = bar.props.iter().find(|p| p.kind == PropKind::BarCounter).expect("bar counter");
        assert!(counter.w_cells >= 3, "multi-cell bar counter: {counter:?}");
        let office = generate_furnished_interior(InteriorStyle::Office, 20, 14, 42);
        assert!(office.props.iter().any(|p| p.kind == PropKind::Desk), "office workstations");
        assert!(office.wall_edges.iter().any(|e| e.kind == WallEdgeKind::Window && e.x > 0 && e.x < 20), "glass conference wall");
        // Gait reach from the West spawn is bounded by interior walls (not the whole board).
        let scene = TacticalSceneView {
            width_m: 20.0,
            height_m: 14.0,
            edges: &bar.wall_edges,
            props: &bar.props,
        };
        let reach = compute_reachable_continuous(&scene, &[], GaitReachQuery {
            start_m: (2.5, 7.5), spd_eff: 11, hgt_m: 1.8,
            gait: CombatMovementGait::AllOutSprint, tick_budget: 40,
        });
        let strips = reach.merged_row_runs();
        assert!(!strips.is_empty());
        assert!(!strips.iter().any(|s| s.x_min_m < 0.0 || s.x_max_m > 20.01));
    }
}
