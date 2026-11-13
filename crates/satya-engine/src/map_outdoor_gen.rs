//! # Procedural Outdoor & Compound Battlemap Generator (`map_outdoor_gen`)
//!
//! Deterministic, seeded `1.0m`-grid outdoor generator supporting two modern-day presets:
//! - [`OutdoorStyle::WoodedPark`]: Winding flagstone trail (`StonePath`), octagonal central plaza with
//!   benches and centerpiece planter/oak, roadside asphalt pullout with parked `2×5` vehicles, trimmed
//!   hedges, picnic lawn, granite boulders, and minimum-spaced `2×2` / `1×1` trees + undergrowth.
//! - [`OutdoorStyle::GardenEstate`]: Paved West street/driveway with parked vehicles, thin masonry
//!   compound perimeter (`MapWallEdge`) with iron gates, landscaped courtyard (stone walks, trimmed
//!   hedges, patio seating, trees), and a windowed, furnished East pavilion/gatehouse.
//!
//! Walkable ground surfaces (`StonePath`, `AsphaltDrive`) are emitted as row-run floor-layer
//! [`MapPropPlacement`]s so both the 2D SVG composer (which merges contiguous regions seamlessly) and
//! the 3D WebGPU diorama render them from the existing `props` array without extra wire fields.

use satya_schema::{
    EdgeSide, MapPropPlacement, MapWallEdge, MaterialClass, PropKind, TacticalTileKind,
    WallEdgeKind,
};
use serde::{Deserialize, Serialize};

use crate::map_dressing::{make_prop, prop_spec, stamp_props_into_tiles};
use crate::tactical_map::ProceduralSector;
use crate::tactical_map_edges::WallEdgeIndex;

/// Outdoor tactical layout family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutdoorStyle {
    WoodedPark,
    GardenEstate,
}

const SURF_TURF: u8 = 0;
const SURF_STONE: u8 = 1;
const SURF_ASPHALT: u8 = 2;

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

    fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next() % den < num
    }
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

struct OutdoorBuilder {
    w: i32,
    h: i32,
    rng: Rng,
    surface: Vec<u8>,
    edges: Vec<MapWallEdge>,
    edge_idx: WallEdgeIndex,
    blocked: Vec<bool>,
    props: Vec<MapPropPlacement>,
    tree_centers: Vec<(i32, i32)>,
}

impl OutdoorBuilder {
    fn new(w: i32, h: i32, seed: u64) -> Self {
        Self {
            w,
            h,
            rng: Rng(seed),
            surface: vec![SURF_TURF; (w * h) as usize],
            edges: Vec::new(),
            edge_idx: WallEdgeIndex::from_edges(w, h, &[]),
            blocked: vec![false; (w * h) as usize],
            props: Vec::new(),
            tree_centers: Vec::new(),
        }
    }

    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    fn set_surf(&mut self, x: i32, y: i32, s: u8) {
        if self.in_bounds(x, y) {
            self.surface[(y * self.w + x) as usize] = s;
        }
    }

    fn surf_at(&self, x: i32, y: i32) -> u8 {
        if self.in_bounds(x, y) {
            self.surface[(y * self.w + x) as usize]
        } else {
            SURF_TURF
        }
    }

    fn paint_rect(&mut self, r: Rect, s: u8) {
        for y in r.y0.max(0)..r.y1.min(self.h) {
            for x in r.x0.max(0)..r.x1.min(self.w) {
                self.surface[(y * self.w + x) as usize] = s;
            }
        }
    }

    /// Paints a `2`-cell-wide stepped stone walkway between `a` and `b`.
    fn paint_walkway(&mut self, a: (i32, i32), b: (i32, i32)) {
        let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
        for i in 0..=steps {
            let x = a.0 + (b.0 - a.0) * i / steps;
            let y = a.1 + (b.1 - a.1) * i / steps;
            for dy in 0..2 {
                if self.surf_at(x, y + dy) == SURF_TURF {
                    self.set_surf(x, y + dy, SURF_STONE);
                }
            }
        }
    }

    fn put_edge(&mut self, x: i32, y: i32, side: EdgeSide, kind: WallEdgeKind, material: MaterialClass) {
        let e = MapWallEdge { x, y, side, kind, material };
        match self.edges.iter_mut().find(|o| o.x == x && o.y == y && o.side == side) {
            Some(o) => *o = e,
            None => self.edges.push(e),
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
            (self.w - 3, mid_y),
            (self.w - 3, (mid_y + 1).min(self.h - 2)),
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

    /// Checks that `[x, x + fw) × [y, y + fh)` fits inside `bounds`, has no blocked cells, and is not
    /// split internally by any thin wall/door/window edge.
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
        let (x, y) = pos;
        let vseed = self.rng.next() as u32;
        let p = make_prop(kind, x, y, rot_deg, len, vseed);
        let decal = prop_spec(kind).floor_layer;
        let ok = if decal {
            x >= bounds.x0 && y >= bounds.y0 && x + p.w_cells <= bounds.x1 && y + p.h_cells <= bounds.y1
        } else {
            self.can_place((x, y), (p.w_cells, p.h_cells), bounds)
        };
        if !ok {
            return None;
        }
        if !decal {
            for yy in y..y + p.h_cells {
                for xx in x..x + p.w_cells {
                    self.blocked[(yy * self.w + xx) as usize] = true;
                }
            }
        }
        self.props.push(p);
        Some(p)
    }

    fn tree_clear(&self, cx: i32, cy: i32, min_dist_cells: i32) -> bool {
        let d2 = min_dist_cells * min_dist_cells;
        self.tree_centers
            .iter()
            .all(|&(tx, ty)| (cx - tx) * (cx - tx) + (cy - ty) * (cy - ty) >= d2)
    }

    fn scatter_trees(&mut self, kind: PropKind, count: i32, min_dist_cells: i32, bounds: Rect) {
        let spec = prop_spec(kind);
        let (fw, fh) = (spec.base_w_cells, spec.base_h_cells);
        let mut placed = 0;
        for _ in 0..(count * 24).max(32) {
            if placed >= count {
                break;
            }
            let x = self.rng.range(bounds.x0, bounds.x1 - fw + 1);
            let y = self.rng.range(bounds.y0, bounds.y1 - fh + 1);
            let on_paved = (y..y + fh).any(|yy| (x..x + fw).any(|xx| self.surf_at(xx, yy) != SURF_TURF));
            if on_paved || !self.tree_clear(x + fw / 2, y + fh / 2, min_dist_cells) {
                continue;
            }
            let rot_deg = self.rng.range(0, 4) * 90;
            if self.try_place(kind, (x, y), rot_deg, None, bounds).is_some() {
                self.tree_centers.push((x + fw / 2, y + fh / 2));
                placed += 1;
            }
        }
    }

    fn scatter_turf_prop(&mut self, kind: PropKind, count: i32, bounds: Rect) {
        let mut placed = 0;
        for _ in 0..(count * 20).max(24) {
            if placed >= count {
                break;
            }
            let rot_deg = self.rng.range(0, 4) * 90;
            let probe = make_prop(kind, 0, 0, rot_deg, None, 0);
            let x = self.rng.range(bounds.x0, bounds.x1 - probe.w_cells + 1);
            let y = self.rng.range(bounds.y0, bounds.y1 - probe.h_cells + 1);
            let on_paved = (y..y + probe.h_cells)
                .any(|yy| (x..x + probe.w_cells).any(|xx| self.surf_at(xx, yy) != SURF_TURF));
            if on_paved {
                continue;
            }
            if self.try_place(kind, (x, y), rot_deg, None, bounds).is_some() {
                placed += 1;
            }
        }
    }

    /// Compresses `self.surface` into horizontal row-run floor props prepended before solid props.
    fn emit_surface_props(&mut self) {
        let mut decals = Vec::new();
        for (surf_code, kind) in [(SURF_ASPHALT, PropKind::AsphaltDrive), (SURF_STONE, PropKind::StonePath)] {
            for y in 0..self.h {
                let mut x = 0;
                while x < self.w {
                    if self.surf_at(x, y) != surf_code {
                        x += 1;
                        continue;
                    }
                    let x0 = x;
                    while x < self.w && self.surf_at(x, y) == surf_code {
                        x += 1;
                    }
                    let vseed = self.rng.next() as u32;
                    decals.push(make_prop(kind, x0, y, 0, Some(x - x0), vseed));
                }
            }
        }
        decals.append(&mut self.props);
        self.props = decals;
    }
}

fn build_wooded_park(b: &mut OutdoorBuilder) {
    let (w, h, mid_y) = (b.w, b.h, b.h / 2);
    let whole = Rect { x0: 0, y0: 0, x1: w, y1: h };

    // 1. Roadside pullout along North or South edge
    let north_road = b.rng.chance(1, 2);
    let has_road = h >= 12 && w >= 12;
    let (turf_y0, turf_y1) = if has_road {
        let ry0 = if north_road { 0 } else { h - 2 };
        b.paint_rect(Rect { x0: 0, y0: ry0, x1: w, y1: ry0 + 2 }, SURF_ASPHALT);
        if north_road { (2, h) } else { (0, h - 2) }
    } else {
        (0, h)
    };

    // 2. Winding West-to-East flagstone trail + octagonal central plaza
    let px = (w / 2 + b.rng.range(-1, 2)).clamp(4, (w - 5).max(4));
    let py_bias = if north_road { 1 } else { -1 };
    let py = (mid_y + py_bias).clamp(turf_y0 + 3, (turf_y1 - 4).max(turf_y0 + 3));
    for dy in -2_i32..=2 {
        for dx in -2_i32..=2 {
            if dx.abs() == 2 && dy.abs() == 2 {
                continue;
            }
            b.set_surf(px + dx, py + dy, SURF_STONE);
        }
    }
    let wy1 = (mid_y - 1 + b.rng.range(-1, 2)).clamp(turf_y0 + 1, (turf_y1 - 3).max(turf_y0 + 1));
    let wy2 = (mid_y - 1 + b.rng.range(-1, 2)).clamp(turf_y0 + 1, (turf_y1 - 3).max(turf_y0 + 1));
    b.paint_walkway((0, (mid_y - 1).max(0)), (w / 4, wy1));
    b.paint_walkway((w / 4, wy1), (px, py - 1));
    b.paint_walkway((px, py - 1), (3 * w / 4, wy2));
    b.paint_walkway((3 * w / 4, wy2), (w - 1, (mid_y - 1).max(0)));
    if has_road {
        let spur_y = if north_road { 2 } else { h - 3 };
        b.paint_walkway((px, py), (px, spur_y));
    }

    b.reserve_spawns_and_doors();

    // 3. Roadside vehicles, concrete barrier & bordering hedge runs
    if has_road {
        let ry0 = if north_road { 0 } else { h - 2 };
        let veh1 = if b.rng.chance(3, 5) { PropKind::Car } else { PropKind::UtilityVan };
        let rot_deg = if b.rng.chance(1, 2) { 90 } else { 270 };
        let vx1 = b.rng.range(2, (px - 4).max(3));
        let _ = b.try_place(veh1, (vx1, ry0), rot_deg, None, whole);
        if w >= 20 {
            let veh2 = if b.rng.chance(1, 2) { PropKind::UtilityVan } else { PropKind::Car };
            let vx2 = b.rng.range((px + 3).min(w - 6), (w - 5).max(px + 4));
            let _ = b.try_place(veh2, (vx2, ry0), 270, None, whole);
        } else {
            let _ = b.try_place(PropKind::JerseyBarrier, ((w - 4).max(0), ry0), 0, Some(3), whole);
        }
        let hy = if north_road { 2 } else { h - 3 };
        let hlen1 = (px - 4).clamp(2, 5);
        let _ = b.try_place(PropKind::Hedge, (2, hy), 0, Some(hlen1), whole);
        let hx2 = (px + 3).min(w - 4);
        let hlen2 = (w - 2 - hx2).clamp(2, 5);
        let _ = b.try_place(PropKind::Hedge, (hx2, hy), 0, Some(hlen2), whole);
    }

    // 4. Plaza centerpiece & inward-facing park benches
    if w >= 18 && h >= 14 && b.rng.chance(1, 2) {
        if b.try_place(PropKind::LargeTree, (px - 1, py - 1), 0, None, whole).is_some() {
            b.tree_centers.push((px, py));
        }
    } else {
        let _ = b.try_place(PropKind::Planter, (px, py), 0, None, whole);
    }
    let _ = b.try_place(PropKind::ParkBench, (px - 1, py - 3), 0, None, whole);
    let _ = b.try_place(PropKind::ParkBench, (px - 1, py + 3), 180, None, whole);

    // 5. Picnic grove, woodland trees, boulders & shrubs
    let turf = Rect { x0: 1, y0: turf_y0.max(1), x1: w - 1, y1: turf_y1.min(h - 1) };
    b.scatter_turf_prop(PropKind::PicnicTable, if w * h >= 320 { 2 } else { 1 }, turf);
    b.scatter_turf_prop(PropKind::ParkBench, 2, turf);
    b.scatter_trees(PropKind::LargeTree, ((w * h) / 68).clamp(2, 7), 5, turf);
    b.scatter_trees(PropKind::MediumTree, ((w * h) / 38).clamp(3, 12), 3, turf);
    b.scatter_turf_prop(PropKind::Boulder, ((w * h) / 85).clamp(2, 5), turf);
    b.scatter_turf_prop(PropKind::Bush, ((w * h) / 34).clamp(4, 14), turf);
}

fn build_garden_estate(b: &mut OutdoorBuilder) {
    let (w, h, mid_y) = (b.w, b.h, b.h / 2);
    let whole = Rect { x0: 0, y0: 0, x1: w, y1: h };

    // 1. West street / driveway approach
    let drive_w = if w >= 18 { 4 } else if w >= 12 { 3 } else { 2 };
    b.paint_rect(Rect { x0: 0, y0: 0, x1: drive_w, y1: h }, SURF_ASPHALT);

    // 2. Masonry compound perimeter walls with main West gate, side gate & East back gate
    let (cx0, cx1) = (drive_w, (w - 1).max(drive_w + 4).min(w));
    let (cy0, cy1) = (1, (h - 1).max(2));
    let wall_mat = if b.rng.chance(2, 3) { MaterialClass::Brick } else { MaterialClass::ConcreteBlock };
    let side_gate_x = (cx0 + 2).min(cx1 - 2);
    for x in cx0..cx1 {
        let n_kind = if x == side_gate_x { WallEdgeKind::DoorClosed } else { WallEdgeKind::Wall };
        let s_kind = if x == side_gate_x + 1 && cx1 - cx0 >= 7 {
            WallEdgeKind::DoorClosed
        } else {
            WallEdgeKind::Wall
        };
        b.put_edge(x, cy0, EdgeSide::North, n_kind, wall_mat);
        b.put_edge(x, cy1, EdgeSide::North, s_kind, wall_mat);
    }
    let gate_y2 = (mid_y - 1).max(cy0);
    for y in cy0..cy1 {
        let w_kind = if y == mid_y {
            WallEdgeKind::DoorOpen
        } else if y == gate_y2 && cy1 - cy0 >= 5 {
            WallEdgeKind::DoorClosed
        } else {
            WallEdgeKind::Wall
        };
        let w_mat = if w_kind == WallEdgeKind::Wall { wall_mat } else { MaterialClass::SheetSteel };
        b.put_edge(cx0, y, EdgeSide::West, w_kind, w_mat);

        let e_kind = if y == mid_y { WallEdgeKind::DoorClosed } else { WallEdgeKind::Wall };
        let e_mat = if e_kind == WallEdgeKind::Wall { wall_mat } else { MaterialClass::SheetSteel };
        b.put_edge(cx1, y, EdgeSide::West, e_kind, e_mat);
    }

    // 3. Windowed East pavilion / gatehouse sharing `cx1` when the compound is wide enough
    let pav = if cx1 - cx0 >= 9 && cy1 - cy0 >= 7 {
        let pw = ((cx1 - cx0) * 2 / 5).clamp(5, 9);
        let ph = ((cy1 - cy0) * 3 / 5).clamp(5, 9);
        let px0 = cx1 - pw;
        let py0 = (mid_y - ph / 2).clamp(cy0 + 1, (cy1 - ph - 1).max(cy0 + 1));
        let py1 = (py0 + ph).min(cy1);
        b.paint_rect(Rect { x0: px0, y0: py0, x1: cx1, y1: py1 }, SURF_STONE);
        // Stone patio terrace outside courtyard door
        b.paint_rect(
            Rect { x0: (px0 - 2).max(cx0), y0: (mid_y - 1).max(py0), x1: px0, y1: (mid_y + 2).min(py1) },
            SURF_STONE,
        );
        let side_door_x = px0 + pw / 2;
        for x in px0..cx1 {
            if py0 > cy0 {
                let k = if x == side_door_x {
                    WallEdgeKind::DoorClosed
                } else if (x - px0) % 3 == 1 {
                    WallEdgeKind::Window
                } else {
                    WallEdgeKind::Wall
                };
                let m = if k == WallEdgeKind::Window { MaterialClass::Glass } else { MaterialClass::Brick };
                b.put_edge(x, py0, EdgeSide::North, k, m);
            }
            if py1 < cy1 {
                let k = if (x - px0) % 3 == 1 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
                let m = if k == WallEdgeKind::Window { MaterialClass::Glass } else { MaterialClass::Brick };
                b.put_edge(x, py1, EdgeSide::North, k, m);
            }
        }
        for y in py0..py1 {
            let k = if y == mid_y {
                WallEdgeKind::DoorOpen
            } else if (y - py0) % 2 == 1 {
                WallEdgeKind::Window
            } else {
                WallEdgeKind::Wall
            };
            let m = match k {
                WallEdgeKind::Window => MaterialClass::Glass,
                WallEdgeKind::DoorOpen => MaterialClass::SolidWood,
                _ => MaterialClass::Brick,
            };
            b.put_edge(px0, y, EdgeSide::West, k, m);
        }
        // Optional interior drywall split when pavilion is tall enough (kept off East spawns `mid_y..=mid_y+1`)
        if py1 - py0 >= 7 && mid_y - py0 >= 3 {
            let split_y = py0 + 2;
            let door_x = px0 + 1;
            for x in px0..cx1 {
                let k = if x == door_x { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall };
                b.put_edge(x, split_y, EdgeSide::North, k, MaterialClass::Drywall);
            }
        }
        Some(Rect { x0: px0, y0: py0, x1: cx1, y1: py1 })
    } else {
        None
    };

    // 4. Courtyard stone walkways connecting main gate -> pavilion / East gate & side gate
    let walk_end_x = pav.map_or(cx1 - 1, |p| p.x0 - 1);
    b.paint_walkway((cx0, gate_y2), (walk_end_x, gate_y2));
    b.paint_walkway((side_gate_x, cy0), (side_gate_x, gate_y2));

    b.reserve_spawns_and_doors();

    // 5. Driveway vehicles (parked clear of the West entry gate & spawn pads)
    if drive_w >= 3 && h >= 12 {
        let v1 = if b.rng.chance(1, 2) { PropKind::UtilityVan } else { PropKind::Car };
        if mid_y > 5 {
            let _ = b.try_place(v1, (0, 0), 0, None, whole);
        }
        if h - (mid_y + 2) >= 5 {
            let v2 = if b.rng.chance(1, 2) { PropKind::Car } else { PropKind::UtilityVan };
            let _ = b.try_place(v2, (0, h - 5), 180, None, whole);
        }
    }

    // 6. Furnish pavilion interior
    if let Some(p) = pav {
        let _ = b.try_place(PropKind::Rug, (p.x0 + 1, (mid_y - 1).max(p.y0)), 0, None, p);
        let _ = b
            .try_place(PropKind::Sofa, (p.x0 + 1, p.y1 - 1), 180, None, p)
            .or_else(|| b.try_place(PropKind::Loveseat, (p.x0 + 1, p.y1 - 1), 180, None, p));
        let _ = b.try_place(PropKind::Desk, (p.x1 - 2, p.y0), 0, None, p);
        let _ = b.try_place(PropKind::Bookshelf, (p.x0, p.y0), 0, None, p);
        let _ = b.try_place(PropKind::SteelShelving, (p.x1 - 2, p.y1 - 1), 180, None, p);
    }

    // 7. Courtyard hedges, patio tables, service dumpster & garden trees
    let yard_x1 = pav.map_or(cx1, |p| p.x0);
    let yard = Rect { x0: cx0, y0: cy0, x1: yard_x1, y1: cy1 };
    let compound = Rect { x0: cx0, y0: cy0, x1: cx1, y1: cy1 };
    let hlen = ((yard_x1 - cx0 - 3) / 2).clamp(2, 4);
    let _ = b.try_place(PropKind::Hedge, (cx0 + 2, (gate_y2 - 2).max(cy0)), 0, Some(hlen), yard);
    let _ = b.try_place(PropKind::Hedge, (cx0 + 2, (mid_y + 2).min(cy1 - 1)), 0, Some(hlen), yard);
    let _ = b.try_place(PropKind::Dumpster, ((cx1 - 3).max(cx0), cy0), 0, None, compound);
    let _ = b.try_place(PropKind::DrumCluster, ((cx1 - 1).max(cx0), cy0), 0, None, compound);

    if let Some(p) = pav {
        let _ = b.try_place(PropKind::Planter, (p.x0 - 1, (mid_y - 2).max(cy0)), 0, None, yard);
        let _ = b.try_place(PropKind::Planter, (p.x0 - 1, (mid_y + 2).min(cy1 - 1)), 0, None, yard);
        let _ = b.try_place(PropKind::RoundTable, ((p.x0 - 2).max(cx0), (mid_y + 1).min(cy1 - 1)), 0, None, yard);
    }
    b.scatter_turf_prop(PropKind::PicnicTable, 1, yard);
    b.scatter_turf_prop(PropKind::ParkBench, 2, yard);
    b.scatter_trees(PropKind::LargeTree, ((yard.x1 - yard.x0) * (cy1 - cy0) / 55).clamp(1, 3), 5, yard);
    b.scatter_trees(PropKind::MediumTree, ((yard.x1 - yard.x0) * (cy1 - cy0) / 28).clamp(2, 6), 3, yard);
    b.scatter_turf_prop(PropKind::Bush, ((yard.x1 - yard.x0) * (cy1 - cy0) / 22).clamp(3, 8), yard);
}

fn all_walkable_connected(
    w: i32,
    h: i32,
    tiles: &[TacticalTileKind],
    edges: &WallEdgeIndex,
    start: (i32, i32),
) -> bool {
    let walkable = |x: i32, y: i32| !tiles[(y * w + x) as usize].blocks_occupancy();
    if !walkable(start.0, start.1) {
        return false;
    }
    let mut seen = vec![false; (w * h) as usize];
    let mut stack = vec![start];
    seen[(start.1 * w + start.0) as usize] = true;
    while let Some((x, y)) = stack.pop() {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h || seen[(ny * w + nx) as usize] || !walkable(nx, ny) {
                continue;
            }
            if edges.step_blocked((x, y), dx, dy, |k| matches!(k, WallEdgeKind::Wall | WallEdgeKind::Window)) {
                continue;
            }
            seen[(ny * w + nx) as usize] = true;
            stack.push((nx, ny));
        }
    }
    (0..h).all(|y| (0..w).all(|x| !walkable(x, y) || seen[(y * w + x) as usize]))
}

/// Generates a deterministic outdoor or walled-compound sector (`width × height` cells, `1.0m` scale).
#[must_use]
pub fn generate_outdoor_sector(style: OutdoorStyle, width: i32, height: i32, seed: i64) -> ProceduralSector {
    let (w, h) = (width.clamp(8, 48), height.clamp(8, 48));
    let salt = match style {
        OutdoorStyle::WoodedPark => 0x50A7_C177,
        OutdoorStyle::GardenEstate => 0xE57A_7E01,
    };
    let mut b = OutdoorBuilder::new(w, h, (seed as u64) ^ salt);
    match style {
        OutdoorStyle::WoodedPark => build_wooded_park(&mut b),
        OutdoorStyle::GardenEstate => build_garden_estate(&mut b),
    }
    b.emit_surface_props();

    let idx = WallEdgeIndex::from_edges(w, h, &b.edges);
    let mut props = b.props;
    let start = (2.min(w - 1), h / 2);
    loop {
        let mut tiles = vec![TacticalTileKind::Floor; (w * h) as usize];
        stamp_props_into_tiles(&mut tiles, w, h, &props);
        if all_walkable_connected(w, h, &tiles, &idx, start) {
            return ProceduralSector { tiles, wall_edges: b.edges, props };
        }
        match props.iter().rposition(|p| !prop_spec(p.kind).floor_layer) {
            Some(i) => {
                props.remove(i);
            }
            None => return ProceduralSector { tiles, wall_edges: b.edges, props },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outdoor_presets_are_deterministic_non_overlapping_and_fully_connected() {
        for style in [OutdoorStyle::WoodedPark, OutdoorStyle::GardenEstate] {
            for &(w, h) in &[(8, 8), (14, 10), (20, 14), (28, 20)] {
                for seed in [1_i64, 42, 179, 316, 9001] {
                    let s = generate_outdoor_sector(style, w, h, seed);
                    assert_eq!(s, generate_outdoor_sector(style, w, h, seed), "{style:?} {w}x{h}/{seed}");
                    assert_eq!(s.tiles.len(), (w * h) as usize);
                    assert!(s.props.iter().any(|p| p.kind == PropKind::StonePath), "stone walkway");
                    let idx = WallEdgeIndex::from_edges(w, h, &s.wall_edges);
                    assert!(
                        all_walkable_connected(w, h, &s.tiles, &idx, (2, h / 2)),
                        "{style:?} {w}x{h}/{seed} must keep every walkable cell connected"
                    );
                    if w >= 12 && h >= 10 {
                        let mid_y = h / 2;
                        for &(sx, sy) in &[(2, mid_y), (2, mid_y - 1), (w - 3, mid_y), (w - 3, mid_y + 1)] {
                            assert_eq!(s.tiles[(sy * w + sx) as usize], TacticalTileKind::Floor, "spawn ({sx},{sy})");
                        }
                    }
                    for (i, p) in s.props.iter().enumerate() {
                        assert!(p.x >= 0 && p.y >= 0 && p.x + p.w_cells <= w && p.y + p.h_cells <= h, "{p:?}");
                        for q in &s.props[i + 1..] {
                            if prop_spec(p.kind).floor_layer || prop_spec(q.kind).floor_layer {
                                continue;
                            }
                            let overlap = p.x < q.x + q.w_cells
                                && q.x < p.x + p.w_cells
                                && p.y < q.y + q.h_cells
                                && q.y < p.y + p.h_cells;
                            assert!(!overlap, "{style:?} {w}x{h}/{seed}: {p:?} overlaps {q:?}");
                        }
                    }
                }
            }
        }
        let park = generate_outdoor_sector(OutdoorStyle::WoodedPark, 20, 14, 42);
        assert!(park.wall_edges.is_empty(), "open park has no perimeter walls");
        assert!(park.props.iter().any(|p| matches!(p.kind, PropKind::LargeTree | PropKind::MediumTree)));
        assert!(park.props.iter().any(|p| p.kind == PropKind::Hedge && p.w_cells >= 2));
        assert!(park.props.iter().any(|p| matches!(p.kind, PropKind::Car | PropKind::UtilityVan)));

        let estate = generate_outdoor_sector(OutdoorStyle::GardenEstate, 20, 14, 42);
        assert!(!estate.wall_edges.is_empty(), "estate has compound + pavilion thin walls");
        assert!(estate.wall_edges.iter().any(|e| e.kind == WallEdgeKind::Window));
        assert!(estate.props.iter().any(|p| p.kind == PropKind::AsphaltDrive));
    }
}
