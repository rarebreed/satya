//! # Composable Multi-Tier Residential Home Generator (`map_gen_residential`)
//!
//! Decouples residential floorplan architecture ([`EconomicTier`]: `LowIncome`, `MiddleClass`,
//! `UpperMiddleClass`, `LuxuryMansion`) from land-use placement ([`crate::map_gen_zoning`]).
//! Supports multi-level generation (`B1` cellar/vault, `1F` ground floor, `2F` upper suites)
//! with vertically aligned [`PropKind::Stairwell`] shafts and 100% doorway connectivity.

use std::collections::VecDeque;

use satya_schema::{
    EconomicTier, EdgeSide, MapFloorLevel, MapPointCm, MapPropPlacement, MapSurfaceFeature,
    MapWallEdge, MaterialClass, PropKind, SurfaceFeatureKind, SurfaceShapeKind, WallEdgeKind,
};

use crate::map_dressing::{PropObstruction, make_prop, prop_spec};
use crate::map_interior_gen::{Furnisher, Rng, Role, Room, Shell, Side};
use crate::tactical_map_geom::TOKEN_RADIUS_M;
use crate::tactical_map_spatial::{SpatialObstacleIndex, SpatialWallFilter};

/// Integer cell bounding box `[x0, x1) × [y0, y1)` for a composable building lot or bay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotBox {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl LotBox {
    #[must_use]
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    #[must_use]
    pub const fn w(self) -> i32 {
        let d = self.x1 - self.x0;
        if d > 0 { d } else { 0 }
    }

    #[must_use]
    pub const fn h(self) -> i32 {
        let d = self.y1 - self.y0;
        if d > 0 { d } else { 0 }
    }

    #[must_use]
    pub const fn area(self) -> i32 {
        self.w() * self.h()
    }

    #[must_use]
    pub(crate) const fn as_room(self) -> Room {
        Room {
            x0: self.x0,
            y0: self.y0,
            x1: self.x1,
            y1: self.y1,
        }
    }

    #[must_use]
    pub fn polygon_feature(self, kind: SurfaceFeatureKind, corner_cm: i32, seed: u32) -> MapSurfaceFeature {
        MapSurfaceFeature {
            kind,
            shape: SurfaceShapeKind::Polygon,
            points_cm: vec![
                MapPointCm::new(self.x0 * 100, self.y0 * 100),
                MapPointCm::new(self.x1 * 100, self.y0 * 100),
                MapPointCm::new(self.x1 * 100, self.y1 * 100),
                MapPointCm::new(self.x0 * 100, self.y1 * 100),
            ],
            width_cm: 0,
            border_style_cm: corner_cm,
            variant_seed: seed,
        }
    }
}

/// Cardinal side of a [`LotBox`] facing the primary street, breezeway, or approach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalEntry {
    North,
    South,
    West,
    East,
}

/// Geometry generated for a single vertical floor level before site wrapping.
#[derive(Debug, Clone, Default)]
pub struct FloorContent {
    pub wall_edges: Vec<MapWallEdge>,
    pub props: Vec<MapPropPlacement>,
    pub surfaces: Vec<MapSurfaceFeature>,
}

/// Vertical shaft record keeping `B1`, `1F`, and `2F` stairs identically registered in `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StairAnchor {
    pub x: i32,
    pub y: i32,
    pub rot_deg: i32,
}

/// Multi-level output emitted by [`generate_home_in_lot`].
#[derive(Debug, Clone)]
pub struct GeneratedHome {
    pub lot: LotBox,
    pub basement: Option<FloorContent>,
    pub ground: FloorContent,
    pub upper: Option<FloorContent>,
    pub stair_anchors: Vec<StairAnchor>,
}

impl GeneratedHome {
    /// Converts upper/basement floor data into wire [`MapFloorLevel`] entries sorted ascending by `floor_index`.
    #[must_use]
    pub fn into_floor_levels(self, ground_merged: FloorContent, has_cellar: bool) -> Vec<MapFloorLevel> {
        let mut levels = Vec::new();
        if let Some(b1) = self.basement
            && has_cellar
        {
            levels.push(MapFloorLevel {
                floor_index: -1,
                label: "B1 · Cellar & Vault".to_string(),
                elevation_cm: -320,
                wall_edges: b1.wall_edges,
                props: b1.props,
                surfaces: b1.surfaces,
            });
        }
        levels.push(MapFloorLevel {
            floor_index: 0,
            label: "1F · Ground".to_string(),
            elevation_cm: 0,
            wall_edges: ground_merged.wall_edges,
            props: ground_merged.props,
            surfaces: ground_merged.surfaces,
        });
        if let Some(f2) = self.upper {
            levels.push(MapFloorLevel {
                floor_index: 1,
                label: "2F · Upper Level".to_string(),
                elevation_cm: 340,
                wall_edges: f2.wall_edges,
                props: f2.props,
                surfaces: f2.surfaces,
            });
        }
        levels.sort_by_key(|f| f.floor_index);
        levels
    }
}

/// Checks that every walkable `1.0m` cell inside `lot` forms one connected component from the first open cell
/// (or stairwell landing), ignoring regions outside `lot` for self-contained `B1`/`2F` levels.
#[must_use]
pub fn floor_lot_connected(
    board_w: i32,
    board_h: i32,
    lot: LotBox,
    edges: &[MapWallEdge],
    props: &[MapPropPlacement],
) -> bool {
    if lot.w() <= 0 || lot.h() <= 0 {
        return false;
    }
    let bw = board_w.max(lot.x1) as f32;
    let bh = board_h.max(lot.y1) as f32;
    let spatial = SpatialObstacleIndex::new(bw, bh, edges, props);
    let cols = (lot.w() * 2) as usize;
    let rows = (lot.h() * 2) as usize;
    let node_pos = |ix: usize, iy: usize| -> (f32, f32) {
        (
            lot.x0 as f32 + (ix as f32 + 0.5) * 0.5,
            lot.y0 as f32 + (iy as f32 + 0.5) * 0.5,
        )
    };
    let mut open = vec![false; cols * rows];
    let mut first = None;
    let stair = props.iter().find(|p| p.kind == PropKind::Stairwell);
    let mut preferred = None;
    for iy in 0..rows {
        for ix in 0..cols {
            let (px, py) = node_pos(ix, iy);
            if !spatial.any_full_obstacle_prop_intersects_circle(px, py, 0.20) {
                open[iy * cols + ix] = true;
                if first.is_none() {
                    first = Some((ix, iy));
                }
                if let Some(st) = stair
                    && st.covers(px.floor() as i32, py.floor() as i32)
                {
                    preferred = Some((ix, iy));
                }
            }
        }
    }
    let Some(start) = preferred.or(first) else {
        return false;
    };
    let mut visited = vec![false; cols * rows];
    let mut q = VecDeque::new();
    visited[start.1 * cols + start.0] = true;
    q.push_back(start);
    while let Some((cx, cy)) = q.pop_front() {
        for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (cx as isize + dx, cy as isize + dy);
            if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                continue;
            }
            let (ux, uy) = (nx as usize, ny as usize);
            let idx = uy * cols + ux;
            if !open[idx] || visited[idx] {
                continue;
            }
            let gcx = (lot.x0 * 2) as usize + cx;
            let gcy = (lot.y0 * 2) as usize + cy;
            let gux = (lot.x0 * 2) as usize + ux;
            let guy = (lot.y0 * 2) as usize + uy;
            if spatial.half_meter_step_blocked_by_wall_or_window(gcx, gcy, gux, guy) {
                continue;
            }
            visited[idx] = true;
            q.push_back((ux, uy));
        }
    }
    for gy in lot.y0..lot.y1 {
        for gx in lot.x0..lot.x1 {
            let cx = gx as f32 + 0.5;
            let cy = gy as f32 + 0.5;
            if spatial.circle_hits_wall(cx, cy, TOKEN_RADIUS_M, SpatialWallFilter::WallsAndWindowsOnly)
                || spatial.any_full_obstacle_prop_intersects_circle(cx, cy, TOKEN_RADIUS_M - 1e-4)
            {
                continue;
            }
            let bx = ((gx - lot.x0) * 2) as usize;
            let by = ((gy - lot.y0) * 2) as usize;
            let reached = [
                (bx, by),
                ((bx + 1).min(cols - 1), by),
                (bx, (by + 1).min(rows - 1)),
                ((bx + 1).min(cols - 1), (by + 1).min(rows - 1)),
            ]
            .into_iter()
            .any(|(sx, sy)| visited[sy * cols + sx]);
            if !reached {
                return false;
            }
        }
    }
    true
}

/// Places a perimeter around `lot` with an exterior door facing `entry_side`, optional rear door, and windows.
pub(crate) fn enclose_lot_shell(
    shell: &mut Shell,
    lot: LotBox,
    mat: MaterialClass,
    entry_side: CardinalEntry,
    rear_door: bool,
    window_step: i32,
) {
    for x in lot.x0..lot.x1 {
        let win_n = window_step > 0 && (x - lot.x0) % window_step == 1 && x + 1 < lot.x1;
        let kn = if win_n { WallEdgeKind::Window } else { WallEdgeKind::Wall };
        let km = if win_n { MaterialClass::Glass } else { mat };
        shell.put(x, lot.y0, EdgeSide::North, kn, km);
        shell.put(x, lot.y1, EdgeSide::North, kn, km);
    }
    for y in lot.y0..lot.y1 {
        let win_w = window_step > 0 && (y - lot.y0) % window_step == 1 && y + 1 < lot.y1;
        let kw = if win_w { WallEdgeKind::Window } else { WallEdgeKind::Wall };
        let km = if win_w { MaterialClass::Glass } else { mat };
        shell.put(lot.x0, y, EdgeSide::West, kw, km);
        shell.put(lot.x1, y, EdgeSide::West, kw, km);
    }
    let mx = lot.x0 + lot.w() / 2;
    let my = lot.y0 + lot.h() / 2;
    let door_k = if shell.rng.chance(2, 3) {
        WallEdgeKind::DoorOpen
    } else {
        WallEdgeKind::DoorClosed
    };
    match entry_side {
        CardinalEntry::North => {
            shell.put(mx, lot.y0, EdgeSide::North, door_k, MaterialClass::SolidWood);
            if rear_door {
                shell.put(mx, lot.y1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
            }
        }
        CardinalEntry::South => {
            shell.put(mx, lot.y1, EdgeSide::North, door_k, MaterialClass::SolidWood);
            if rear_door {
                shell.put(mx, lot.y0, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
            }
        }
        CardinalEntry::West => {
            shell.put(lot.x0, my, EdgeSide::West, door_k, MaterialClass::SolidWood);
            if rear_door {
                shell.put(lot.x1, my, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
            }
        }
        CardinalEntry::East => {
            shell.put(lot.x1, my, EdgeSide::West, door_k, MaterialClass::SolidWood);
            if rear_door {
                shell.put(lot.x0, my, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
            }
        }
    }
}

/// Trims blocking props until `floor_lot_connected` passes, and verifies no wall edge is orphaned.
pub(crate) fn trim_floor_props_to_connected(
    board_w: i32,
    board_h: i32,
    lot: LotBox,
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
) {
    let mut guard = 0;
    while !floor_lot_connected(board_w, board_h, lot, edges, props) && guard < 40 {
        guard += 1;
        match props
            .iter()
            .rposition(|p| prop_spec(p.kind).obstruction == PropObstruction::FullObstacle)
            .or_else(|| {
                props
                    .iter()
                    .rposition(|p| prop_spec(p.kind).obstruction == PropObstruction::HalfCover)
            }) {
            Some(i) => {
                props.remove(i);
            }
            None => break,
        }
    }
}

/// Computes an aligned stairwell shaft inside `lot` without colliding with perimeter walls.
fn pick_stair_anchor(lot: LotBox, rng: &mut Rng) -> Option<StairAnchor> {
    if lot.w() >= 6 && lot.h() >= 5 {
        let rot_deg = if lot.w() >= lot.h() + 3 { 90 } else { 0 };
        let (sw, sh) = if rot_deg == 90 { (3, 2) } else { (2, 3) };
        let sx = lot.x0 + 1 + rng.range(0, (lot.w() - sw - 1).max(1)) / 2;
        let sy = lot.y0 + 1 + rng.range(0, (lot.h() - sh - 1).max(1)) / 2;
        Some(StairAnchor {
            x: sx.clamp(lot.x0, lot.x1 - sw),
            y: sy.clamp(lot.y0, lot.y1 - sh),
            rot_deg,
        })
    } else if lot.w() >= 4 && lot.h() >= 4 {
        Some(StairAnchor {
            x: lot.x0,
            y: lot.y0,
            rot_deg: 0,
        })
    } else {
        None
    }
}

/// Reserves all cells covered by `stair` inside `furn` and pushes the [`PropKind::Stairwell`] placement.
fn stamp_stairwell(furn: &mut Furnisher, stair: StairAnchor) {
    let prop = make_prop(PropKind::Stairwell, stair.x, stair.y, stair.rot_deg, None, 0);
    for yy in prop.y..(prop.y + prop.h_cells) {
        for xx in prop.x..(prop.x + prop.w_cells) {
            if prop.covers(xx, yy) {
                furn.reserve_cell(xx, yy);
            }
        }
    }
    furn.props.push(prop);
}

/// Partition strategy that splits rooms strictly outside the hall containing `stair` so stairs never
/// get sliced by interior partition walls, while connecting every room via doorway into its parent.
fn build_interior_rooms_for_floor(
    shell: &mut Shell,
    lot: LotBox,
    stair: Option<StairAnchor>,
    tier: EconomicTier,
    floor_idx: i32,
) -> Vec<(Room, Role)> {
    let mut out = Vec::new();
    let full = lot.as_room();

    // Central spine / hall when wide or tall enough so wings branch cleanly around the stairwell.
    let (hall, wings): (Room, Vec<(Room, Side)>) = if lot.w() >= 10 && lot.h() >= 7 {
        let hall_w = if tier == EconomicTier::LuxuryMansion && lot.w() >= 16 { 4 } else { 3 };
        let hx0 = if let Some(st) = stair {
            let st_w = if st.rot_deg % 180 == 90 { 3 } else { 2 };
            st.x.clamp(lot.x0 + 3, (lot.x1 - 3 - hall_w).max(lot.x0 + 3))
                .min(lot.x1 - st_w - 1)
        } else {
            lot.x0 + (lot.w() - hall_w) / 2
        };
        let hx1 = (hx0 + hall_w).min(lot.x1 - 2).max(hx0 + 2);
        let door_w = lot.y0 + lot.h() / 2;
        let door_e = (lot.y0 + lot.h() / 2 - 1).max(lot.y0);
        shell.partition_v(hx0, lot.y0, lot.y1, &[door_w, (lot.y0 + 1).min(lot.y1 - 1)]);
        shell.partition_v(hx1, lot.y0, lot.y1, &[door_e, (lot.y1 - 2).max(lot.y0)]);
        let w_wing = Room { x0: lot.x0, y0: lot.y0, x1: hx0, y1: lot.y1 };
        let e_wing = Room { x0: hx1, y0: lot.y0, x1: lot.x1, y1: lot.y1 };
        (
            Room { x0: hx0, y0: lot.y0, x1: hx1, y1: lot.y1 },
            vec![(w_wing, Side::West), (e_wing, Side::East)],
        )
    } else if lot.w() >= 6 && lot.h() >= 5 {
        let sx = stair
            .map(|st| {
                let st_w = if st.rot_deg % 180 == 90 { 3 } else { 2 };
                (st.x + st_w).clamp(lot.x0 + 3, lot.x1 - 2)
            })
            .unwrap_or_else(|| lot.x0 + lot.w() / 2);
        let sx = sx.clamp(lot.x0 + 2, lot.x1 - 2);
        let door_y = lot.y0 + lot.h() / 2;
        shell.partition_v(sx, lot.y0, lot.y1, &[door_y]);
        (
            Room { x0: lot.x0, y0: lot.y0, x1: sx, y1: lot.y1 },
            vec![(Room { x0: sx, y0: lot.y0, x1: lot.x1, y1: lot.y1 }, Side::East)],
        )
    } else {
        return vec![(full, Role::Living)];
    };

    // Role palette tuned to EconomicTier and floor index (`-1` basement, `0` ground, `1` upper).
    let palette: &[Role] = match (tier, floor_idx) {
        (_, -1) => &[Role::Storage, Role::BarHall, Role::ServerCloset, Role::BackKitchen],
        (EconomicTier::LowIncome, 0) => &[Role::Kitchen, Role::Bedroom, Role::Bathroom],
        (EconomicTier::LowIncome, _) => &[Role::Bedroom, Role::Bathroom, Role::Living],
        (EconomicTier::MiddleClass, 0) => &[Role::Kitchen, Role::Bedroom, Role::Bathroom, Role::Office],
        (EconomicTier::MiddleClass, _) => &[Role::Bedroom, Role::Bedroom, Role::Bathroom, Role::Office],
        (EconomicTier::UpperMiddleClass, 0) => &[
            Role::Kitchen,
            Role::Office,
            Role::Bedroom,
            Role::Bathroom,
            Role::BackKitchen,
        ],
        (EconomicTier::UpperMiddleClass, _) => &[
            Role::Bedroom,
            Role::Office,
            Role::Bedroom,
            Role::Bathroom,
        ],
        (EconomicTier::LuxuryMansion, 0) => &[
            Role::BackKitchen,
            Role::Office,
            Role::Living,
            Role::Kitchen,
            Role::Bathroom,
            Role::Bedroom,
        ],
        (EconomicTier::LuxuryMansion, _) => &[
            Role::Bedroom,
            Role::Office,
            Role::Living,
            Role::Bathroom,
            Role::Bedroom,
            Role::Storage,
        ],
    };

    let hall_role = match (tier, floor_idx) {
        (_, -1) => Role::Storage,
        (EconomicTier::LuxuryMansion, _) => Role::Living,
        (EconomicTier::UpperMiddleClass, 1) => Role::Office,
        _ => Role::Living,
    };
    out.push((hall, hall_role));

    let max_depth = match tier {
        EconomicTier::LowIncome => 1,
        EconomicTier::MiddleClass => 2,
        EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion => 3,
    };
    let min_dim = if lot.w() >= 14 && lot.h() >= 10 { 4 } else { 3 };
    let mut role_cursor = 0usize;
    for (wing, _) in wings {
        let mut sub_rooms = Vec::new();
        shell.bsp(wing, 0, min_dim, max_depth, &mut sub_rooms);
        for sr in sub_rooms {
            let role = palette[role_cursor % palette.len()];
            role_cursor += 1;
            out.push((sr, role));
        }
    }
    out
}

/// Populates one architectural floor (`floor_idx`) inside `lot`, honoring `stair` alignment and extra reserves.
#[allow(clippy::too_many_arguments)]
fn build_single_home_floor(
    board_w: i32,
    board_h: i32,
    lot: LotBox,
    tier: EconomicTier,
    entry_side: CardinalEntry,
    floor_idx: i32,
    stair: Option<StairAnchor>,
    rear_door: bool,
    extra_reserves: &[(f32, f32)],
    seed: u64,
) -> FloorContent {
    let exterior_mat = if floor_idx < 0 {
        MaterialClass::ReinforcedConcrete
    } else {
        match tier {
            EconomicTier::LowIncome => MaterialClass::ConcreteBlock,
            EconomicTier::MiddleClass | EconomicTier::UpperMiddleClass => MaterialClass::Brick,
            EconomicTier::LuxuryMansion => MaterialClass::ReinforcedConcrete,
        }
    };
    let mut shell = Shell {
        w: board_w,
        h: board_h,
        rng: Rng(seed ^ ((floor_idx + 3) as u64).wrapping_mul(0x9E37_79B9)),
        edges: Vec::new(),
    };
    let win_step = if floor_idx < 0 {
        0
    } else if tier == EconomicTier::LowIncome {
        4
    } else {
        3
    };
    if floor_idx == 0 {
        enclose_lot_shell(&mut shell, lot, exterior_mat, entry_side, rear_door, win_step);
    } else {
        // Enclosed upper/cellar perimeter without external doors (connected strictly via Stairwell).
        for x in lot.x0..lot.x1 {
            let win = win_step > 0 && (x - lot.x0) % win_step == 1 && x + 1 < lot.x1;
            let (k, m) = if win {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, exterior_mat)
            };
            shell.put(x, lot.y0, EdgeSide::North, k, m);
            shell.put(x, lot.y1, EdgeSide::North, k, m);
        }
        for y in lot.y0..lot.y1 {
            let win = win_step > 0 && (y - lot.y0) % win_step == 1 && y + 1 < lot.y1;
            let (k, m) = if win {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, exterior_mat)
            };
            shell.put(lot.x0, y, EdgeSide::West, k, m);
            shell.put(lot.x1, y, EdgeSide::West, k, m);
        }
    }

    let rooms = build_interior_rooms_for_floor(&mut shell, lot, stair, tier, floor_idx);

    let mut furn = Furnisher::new(board_w, board_h, shell.rng.next());
    furn.reserve_doors(&shell.edges);
    for &(sx, sy) in extra_reserves {
        furn.reserve_cell(sx.floor() as i32, sy.floor() as i32);
    }
    if let Some(st) = stair {
        stamp_stairwell(&mut furn, st);
    }
    for &(room, role) in &rooms {
        furn.furnish(role, room);
    }
    if floor_idx == -1 {
        let _ = furn.against_any_wall(PropKind::SteelShelving, lot.as_room(), None);
        furn.scatter(PropKind::WoodCrate, lot.as_room(), 2, 0);
    } else if tier == EconomicTier::LuxuryMansion && lot.w() >= 14 {
        let _ = furn.against_any_wall(PropKind::StructuralPillar, lot.as_room(), None);
    }

    trim_floor_props_to_connected(board_w, board_h, lot, &shell.edges, &mut furn.props);

    let surf_seed = (seed as u32).wrapping_add((floor_idx + 11) as u32 * 37);
    let surfaces = vec![lot.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, surf_seed)];
    FloorContent {
        wall_edges: shell.edges,
        props: furn.props,
        surfaces,
    }
}

/// Composes a furnished residential structure scaled to `tier` inside `lot`, emitting ground floor
/// `1F` plus optional aligned upper floor (`2F`) and cellar (`B1`) levels.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn generate_home_in_lot(
    board_w: i32,
    board_h: i32,
    lot: LotBox,
    tier: EconomicTier,
    entry_side: CardinalEntry,
    allow_upper_floor: bool,
    allow_basement: bool,
    rear_door: bool,
    spawn_pads_m: &[(f32, f32)],
    seed: u64,
) -> GeneratedHome {
    let mut rng = Rng(seed ^ 0x5A17_C0DE);
    let stair = if allow_upper_floor || allow_basement {
        pick_stair_anchor(lot, &mut rng)
    } else {
        None
    };

    let ground = build_single_home_floor(
        board_w,
        board_h,
        lot,
        tier,
        entry_side,
        0,
        stair,
        rear_door,
        spawn_pads_m,
        seed,
    );
    let upper = if allow_upper_floor && stair.is_some() && lot.w() >= 5 && lot.h() >= 4 {
        Some(build_single_home_floor(
            board_w,
            board_h,
            lot,
            tier,
            entry_side,
            1,
            stair,
            false,
            &[],
            seed.wrapping_add(101),
        ))
    } else {
        None
    };
    let basement = if allow_basement && stair.is_some() && lot.w() >= 6 && lot.h() >= 5 {
        Some(build_single_home_floor(
            board_w,
            board_h,
            lot,
            tier,
            entry_side,
            -1,
            stair,
            false,
            &[],
            seed.wrapping_add(202),
        ))
    } else {
        None
    };

    GeneratedHome {
        lot,
        basement,
        ground,
        upper,
        stair_anchors: stair.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_economic_tiers_generate_connected_furnished_levels_and_aligned_stairs() {
        let lot = LotBox::new(4, 3, 24, 19);
        for tier in EconomicTier::ALL {
            let allow_b1 = tier == EconomicTier::LuxuryMansion;
            let home = generate_home_in_lot(
                28,
                22,
                lot,
                tier,
                CardinalEntry::South,
                true,
                allow_b1,
                true,
                &[],
                42,
            );
            assert!(!home.ground.wall_edges.is_empty(), "{tier:?} missing edges");
            assert!(!home.ground.props.is_empty(), "{tier:?} missing props");
            assert!(floor_lot_connected(28, 22, lot, &home.ground.wall_edges, &home.ground.props));
            let f2 = home.upper.as_ref().expect("2F level present");
            assert!(floor_lot_connected(28, 22, lot, &f2.wall_edges, &f2.props));
            let st1 = home.ground.props.iter().find(|p| p.kind == PropKind::Stairwell).unwrap();
            let st2 = f2.props.iter().find(|p| p.kind == PropKind::Stairwell).unwrap();
            assert_eq!(st1.center_cm(), st2.center_cm(), "{tier:?} stairs must align across floors");
            if allow_b1 {
                let b1 = home.basement.as_ref().expect("B1 cellar present");
                assert!(floor_lot_connected(28, 22, lot, &b1.wall_edges, &b1.props));
                let stb = b1.props.iter().find(|p| p.kind == PropKind::Stairwell).unwrap();
                assert_eq!(st1.center_cm(), stb.center_cm());
            }
        }
    }
}
