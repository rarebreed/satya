//! # Architectural Floorplan Graph & Multi-Wing Home Generator (`map_home_floorplan`)
//!
//! Replaces single-corridor BSP boxes with varied architectural floorplans (`LShaped`,
//! `OpenConceptFoyer`, `FarmhouseSunroom`, `CourtyardVilla`, `RowhouseOrApt`), weighted spanning-tree
//! doorway carving (so hallways stay natural and bathrooms/closets act as true private leaf rooms),
//! room-specific flooring (`CeramicTileFloor`, `MarbleFloor`, `CarpetFloor`), attached outdoor
//! `WoodDeck` / `StoneWalk` patios, and `PoolWater` / `HotTub` amenities for affluent homes.

use satya_schema::{
    EconomicTier, EdgeSide, MapSurfaceFeature, MaterialClass, PropKind, SurfaceFeatureKind,
    WallEdgeKind,
};

use crate::map_dressing::make_prop;
use crate::map_gen_residential::{
    CardinalEntry, FloorContent, LotBox, StairAnchor, trim_floor_props_to_connected,
};
use crate::map_interior_gen::{Furnisher, Rng, Role, Room, Shell, emit_room_floor_surface};
use crate::map_surface_queries::prune_occluded_surfaces;

/// Distinct residential floorplan grammar selected per house & seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HomePlanKind {
    LShaped,
    OpenConceptFoyer,
    FarmhouseSunroom,
    CourtyardVilla,
    RowhouseOrApt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotchCorner {
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

/// Architectural silhouette and stair placement shared across `B1`, `1F`, and `2F` of one home.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HomeBlueprint {
    pub(crate) plan: HomePlanKind,
    pub(crate) main_wing: LotBox,
    pub(crate) arm_wing: Option<LotBox>,
    pub(crate) patio_box: Option<LotBox>,
    pub(crate) pool_in_patio: bool,
    pub(crate) hot_tub_in_patio: bool,
    pub(crate) stair: Option<StairAnchor>,
    pub(crate) hardwood_seed: u32,
}

#[derive(Debug, Clone, Copy)]
struct SharedBorder {
    a: usize,
    b: usize,
    vertical: bool,
    fixed_coord: i32,
    start: i32,
    end: i32,
    same_wing: bool,
    weight: i32,
}

fn dsu_find(p: &mut [usize], mut x: usize) -> usize {
    while p[x] != x {
        p[x] = p[p[x]];
        x = p[x];
    }
    x
}

const fn stair_foot(st: StairAnchor) -> (i32, i32, i32, i32) {
    let (sw, sh) = if st.rot_deg.rem_euclid(180) == 90 {
        (3, 2)
    } else {
        (2, 3)
    };
    (st.x, st.y, st.x + sw, st.y + sh)
}

fn cut_avoids_stair_v(r: Room, sx: i32, stair: Option<StairAnchor>) -> bool {
    let Some(st) = stair else { return true };
    let (tx0, ty0, tx1, ty1) = stair_foot(st);
    r.y1 <= ty0 || r.y0 >= ty1 || sx <= tx0 || sx >= tx1
}

fn cut_avoids_stair_h(r: Room, sy: i32, stair: Option<StairAnchor>) -> bool {
    let Some(st) = stair else { return true };
    let (tx0, ty0, tx1, ty1) = stair_foot(st);
    r.x1 <= tx0 || r.x0 >= tx1 || sy <= ty0 || sy >= ty1
}

fn pick_v_cut(
    r: Room,
    target: i32,
    min_a: i32,
    min_b: i32,
    stair: Option<StairAnchor>,
) -> Option<i32> {
    let (lo, hi) = (r.x0 + min_a, r.x1 - min_b);
    (lo <= hi)
        .then(|| {
            (lo..=hi)
                .filter(|&sx| cut_avoids_stair_v(r, sx, stair))
                .min_by_key(|&sx| (sx - target).abs())
        })
        .flatten()
}

fn pick_h_cut(
    r: Room,
    target: i32,
    min_a: i32,
    min_b: i32,
    stair: Option<StairAnchor>,
) -> Option<i32> {
    let (lo, hi) = (r.y0 + min_a, r.y1 - min_b);
    (lo <= hi)
        .then(|| {
            (lo..=hi)
                .filter(|&sy| cut_avoids_stair_h(r, sy, stair))
                .min_by_key(|&sy| (sy - target).abs())
        })
        .flatten()
}

/// Computes the silhouette, attached patio/deck/pool zone, and aligned stair shaft inside `lot`.
#[must_use]
pub(crate) fn plan_home_blueprint(
    lot: LotBox,
    tier: EconomicTier,
    entry_side: CardinalEntry,
    allow_vertical_shaft: bool,
    seed: u64,
) -> HomeBlueprint {
    let mut rng = Rng(seed ^ 0x4F10_0F91);
    let hardwood_seed = (rng.next() & 0xffff_ffff) as u32;
    let can_l = lot.w() >= 11 && lot.h() >= 9;
    let can_villa = lot.w() >= 14
        && lot.h() >= 10
        && matches!(
            tier,
            EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion
        );

    let roll = rng.range(0, 100);
    let plan = if lot.w() < 9 || lot.h() < 7 {
        HomePlanKind::RowhouseOrApt
    } else if can_villa && roll < 26 {
        HomePlanKind::CourtyardVilla
    } else if can_l && roll < 64 {
        HomePlanKind::LShaped
    } else if roll < 84 {
        HomePlanKind::OpenConceptFoyer
    } else {
        HomePlanKind::FarmhouseSunroom
    };

    let mut arm_wing = None;
    let mut patio_box = None;
    let main_wing = match plan {
        HomePlanKind::LShaped => {
            let notch = match (entry_side, rng.chance(1, 2)) {
                (CardinalEntry::South, true) | (CardinalEntry::West, true) => {
                    NotchCorner::NorthEast
                }
                (CardinalEntry::South, false) | (CardinalEntry::East, true) => {
                    NotchCorner::NorthWest
                }
                (CardinalEntry::North, true) | (CardinalEntry::West, false) => {
                    NotchCorner::SouthEast
                }
                (CardinalEntry::North, false) | (CardinalEntry::East, false) => {
                    NotchCorner::SouthWest
                }
            };
            let split_x =
                (lot.x0 + lot.w() * rng.range(48, 60) / 100).clamp(lot.x0 + 5, lot.x1 - 4);
            let split_y =
                (lot.y0 + lot.h() * rng.range(46, 60) / 100).clamp(lot.y0 + 4, lot.y1 - 4);
            let (m, a, n) = match notch {
                NotchCorner::NorthEast => (
                    LotBox::new(lot.x0, split_y, lot.x1, lot.y1),
                    LotBox::new(lot.x0, lot.y0, split_x, split_y),
                    LotBox::new(split_x, lot.y0, lot.x1, split_y),
                ),
                NotchCorner::NorthWest => (
                    LotBox::new(lot.x0, split_y, lot.x1, lot.y1),
                    LotBox::new(split_x, lot.y0, lot.x1, split_y),
                    LotBox::new(lot.x0, lot.y0, split_x, split_y),
                ),
                NotchCorner::SouthEast => (
                    LotBox::new(lot.x0, lot.y0, lot.x1, split_y),
                    LotBox::new(lot.x0, split_y, split_x, lot.y1),
                    LotBox::new(split_x, split_y, lot.x1, lot.y1),
                ),
                NotchCorner::SouthWest => (
                    LotBox::new(lot.x0, lot.y0, lot.x1, split_y),
                    LotBox::new(split_x, split_y, lot.x1, lot.y1),
                    LotBox::new(lot.x0, split_y, split_x, lot.y1),
                ),
            };
            arm_wing = Some(a);
            patio_box = Some(n);
            m
        }
        HomePlanKind::CourtyardVilla
        | HomePlanKind::OpenConceptFoyer
        | HomePlanKind::FarmhouseSunroom => {
            if lot.h() >= 11 && !matches!(entry_side, CardinalEntry::West | CardinalEntry::East) {
                let deck_d = if plan == HomePlanKind::CourtyardVilla {
                    3
                } else {
                    2
                };
                match entry_side {
                    CardinalEntry::South => {
                        patio_box = Some(LotBox::new(lot.x0, lot.y0, lot.x1, lot.y0 + deck_d));
                        LotBox::new(lot.x0, lot.y0 + deck_d, lot.x1, lot.y1)
                    }
                    CardinalEntry::North => {
                        patio_box = Some(LotBox::new(lot.x0, lot.y1 - deck_d, lot.x1, lot.y1));
                        LotBox::new(lot.x0, lot.y0, lot.x1, lot.y1 - deck_d)
                    }
                    _ => lot,
                }
            } else {
                lot
            }
        }
        HomePlanKind::RowhouseOrApt => lot,
    };

    let stair = if allow_vertical_shaft && main_wing.w() >= 6 && main_wing.h() >= 5 {
        let rot_deg = if main_wing.w() >= main_wing.h() + 3 {
            90
        } else {
            0
        };
        let (sw, sh) = if rot_deg == 90 { (3, 2) } else { (2, 3) };
        let sx = (main_wing.x0 + (main_wing.w() - sw) / 2).clamp(main_wing.x0, main_wing.x1 - sw);
        let sy = (main_wing.y0 + (main_wing.h() - sh) / 2).clamp(main_wing.y0, main_wing.y1 - sh);
        Some(StairAnchor {
            x: sx,
            y: sy,
            rot_deg,
        })
    } else if allow_vertical_shaft && main_wing.w() >= 4 && main_wing.h() >= 4 {
        Some(StairAnchor {
            x: main_wing.x0,
            y: main_wing.y0,
            rot_deg: 0,
        })
    } else {
        None
    };

    let affluent = matches!(
        tier,
        EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion
    );
    let pool_in_patio = affluent && patio_box.is_some_and(|p| p.w() >= 5 && p.h() >= 4);
    let hot_tub_in_patio =
        affluent && !pool_in_patio && patio_box.is_some_and(|p| p.w() >= 3 && p.h() >= 3);

    HomeBlueprint {
        plan,
        main_wing,
        arm_wing,
        patio_box,
        pool_in_patio,
        hot_tub_in_patio,
        stair,
        hardwood_seed,
    }
}

fn is_seam_edge(x: i32, y: i32, side: EdgeSide, a: LotBox, b: LotBox) -> bool {
    match side {
        EdgeSide::North => {
            (y == a.y1 && y == b.y0 || y == b.y1 && y == a.y0)
                && x >= a.x0.max(b.x0)
                && x < a.x1.min(b.x1)
        }
        EdgeSide::West => {
            (x == a.x1 && x == b.x0 || x == b.x1 && x == a.x0)
                && y >= a.y0.max(b.y0)
                && y < a.y1.min(b.y1)
        }
    }
}

fn enclose_wing_perimeter(
    shell: &mut Shell,
    wing: LotBox,
    other_wing: Option<LotBox>,
    mat: MaterialClass,
    win_step: i32,
) {
    for x in wing.x0..wing.x1 {
        let win = win_step > 0 && (x - wing.x0) % win_step == 1 && x + 1 < wing.x1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, mat)
        };
        if !other_wing.is_some_and(|o| is_seam_edge(x, wing.y0, EdgeSide::North, wing, o)) {
            shell.put(x, wing.y0, EdgeSide::North, k, m);
        }
        if !other_wing.is_some_and(|o| is_seam_edge(x, wing.y1, EdgeSide::North, wing, o)) {
            shell.put(x, wing.y1, EdgeSide::North, k, m);
        }
    }
    for y in wing.y0..wing.y1 {
        let win = win_step > 0 && (y - wing.y0) % win_step == 1 && y + 1 < wing.y1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, mat)
        };
        if !other_wing.is_some_and(|o| is_seam_edge(wing.x0, y, EdgeSide::West, wing, o)) {
            shell.put(wing.x0, y, EdgeSide::West, k, m);
        }
        if !other_wing.is_some_and(|o| is_seam_edge(wing.x1, y, EdgeSide::West, wing, o)) {
            shell.put(wing.x1, y, EdgeSide::West, k, m);
        }
    }
}

fn subdivide_cluster(
    r: Room,
    stair: Option<StairAnchor>,
    depth_left: u8,
    rng: &mut Rng,
    out: &mut Vec<Room>,
) {
    if depth_left == 0 || r.area() <= 14 || (r.w() < 5 && r.h() < 5) {
        out.push(r);
        return;
    }
    let split_v = r.w() >= 6 && (r.h() < 5 || (r.w() > r.h() && rng.chance(2, 3)));
    if split_v && let Some(sx) = pick_v_cut(r, r.x0 + r.w() * rng.range(42, 62) / 100, 2, 3, stair)
    {
        subdivide_cluster(Room { x1: sx, ..r }, stair, depth_left - 1, rng, out);
        subdivide_cluster(Room { x0: sx, ..r }, stair, depth_left - 1, rng, out);
        return;
    }
    if r.h() >= 5
        && let Some(sy) = pick_h_cut(r, r.y0 + r.h() * rng.range(40, 62) / 100, 2, 3, stair)
    {
        subdivide_cluster(Room { y1: sy, ..r }, stair, depth_left - 1, rng, out);
        subdivide_cluster(Room { y0: sy, ..r }, stair, depth_left - 1, rng, out);
        return;
    }
    out.push(r);
}

fn split_along_strip_cols(r: Room, stair: Option<StairAnchor>, target_w: i32, out: &mut Vec<Room>) {
    if r.w() <= target_w + 1 {
        out.push(r);
        return;
    }
    if let Some(sx) = pick_v_cut(r, r.x0 + (r.w() / 2).max(2), 2, 2, stair) {
        split_along_strip_cols(Room { x1: sx, ..r }, stair, target_w, out);
        split_along_strip_cols(Room { x0: sx, ..r }, stair, target_w, out);
    } else {
        out.push(r);
    }
}

fn max_service_room_area(role: Role) -> Option<i32> {
    match role {
        Role::MasterBath => Some(18),
        Role::Bathroom => Some(14),
        Role::Closet | Role::Laundry => Some(12),
        _ => None,
    }
}

/// Splits any oversized `Bathroom`, `MasterBath`, `Closet`, or `Laundry` room (`> 14..18 m²`)
/// into a human-scaled bath/service compartment plus a `Bedroom`/`Office`/`Closet`.
fn enforce_service_room_caps(
    tagged: &mut Vec<(Room, Role, u8)>,
    stair: Option<StairAnchor>,
    floor_idx: i32,
    is_main: bool,
) {
    if floor_idx < 0 {
        return;
    }
    // Guarantee ground-floor homes with >= 2 rooms include at least one Bathroom before capping size.
    let has_bath = tagged
        .iter()
        .any(|(_, rl, _)| matches!(rl, Role::Bathroom | Role::MasterBath));
    if is_main && floor_idx == 0 && tagged.len() >= 2 && !has_bath {
        if let Some(cand_i) = tagged
            .iter()
            .enumerate()
            .filter(|(_, (rm, rl, _))| {
                rm.area() <= 14
                    && !matches!(
                        rl,
                        Role::Hall | Role::Foyer | Role::Living | Role::Kitchen | Role::MasterBedroom
                    )
            })
            .min_by_key(|(_, (rm, _, _))| rm.area())
            .map(|(i, _)| i)
        {
            tagged[cand_i].1 = Role::Bathroom;
        } else if let Some((split_i, _)) = tagged
            .iter()
            .enumerate()
            .filter(|(_, (rm, rl, _))| {
                (rm.w() >= 4 || rm.h() >= 4) && !matches!(rl, Role::Hall | Role::Foyer)
            })
            .min_by_key(|(_, (rm, rl, _))| {
                let pen = if matches!(rl, Role::Living | Role::Kitchen) {
                    100
                } else {
                    0
                };
                pen + rm.area()
            })
        {
            let (rm, rl, wid) = tagged[split_i];
            if rm.w() >= 4
                && let Some(sx) = pick_v_cut(rm, rm.x0 + 2, 2, 2, stair)
                    .or_else(|| pick_v_cut(rm, rm.x1 - 2, 2, 2, stair))
            {
                let a = Room { x1: sx, ..rm };
                let b = Room { x0: sx, ..rm };
                let (bath, rest) = if a.area() <= b.area() { (a, b) } else { (b, a) };
                tagged[split_i] = (rest, rl, wid);
                tagged.push((bath, Role::Bathroom, wid));
            } else if rm.h() >= 4
                && let Some(sy) = pick_h_cut(rm, rm.y0 + 2, 2, 2, stair)
                    .or_else(|| pick_h_cut(rm, rm.y1 - 2, 2, 2, stair))
            {
                let a = Room { y1: sy, ..rm };
                let b = Room { y0: sy, ..rm };
                let (bath, rest) = if a.area() <= b.area() { (a, b) } else { (b, a) };
                tagged[split_i] = (rest, rl, wid);
                tagged.push((bath, Role::Bathroom, wid));
            } else if let Some(cand_i) = tagged
                .iter()
                .enumerate()
                .filter(|(_, (_, cr, _))| !matches!(cr, Role::Hall | Role::Foyer))
                .min_by_key(|(_, (rm, _, _))| rm.area())
                .map(|(i, _)| i)
            {
                tagged[cand_i].1 = Role::Bathroom;
            }
        }
    }

    for _pass in 0..8 {
        let Some(idx) = tagged.iter().position(|(rm, role, _)| {
            max_service_room_area(*role).is_some_and(|cap| rm.area() > cap)
        }) else {
            break;
        };
        let (rm, role, wid) = tagged[idx];
        let cap = max_service_room_area(role).unwrap_or(14);
        let max_bath_dim = if role == Role::MasterBath { 4 } else { 3 };
        let try_split_v = if rm.w() >= rm.h() {
            [true, false]
        } else {
            [false, true]
        };
        let mut did_split = false;
        for split_v in try_split_v {
            if split_v && rm.w() >= 4 {
                let want = (cap / rm.h().max(1)).clamp(2, max_bath_dim);
                if let Some(sx) = pick_v_cut(rm, rm.x0 + want, 2, 2, stair) {
                    let a = Room { x1: sx, ..rm };
                    let b = Room { x0: sx, ..rm };
                    let (bath_rm, rest_rm) = if a.area() <= b.area() { (a, b) } else { (b, a) };
                    let rest_role = if rest_rm.area() >= 10 {
                        Role::Bedroom
                    } else if rest_rm.area() >= 6 {
                        Role::Closet
                    } else {
                        Role::Hall
                    };
                    tagged[idx] = (bath_rm, role, wid);
                    tagged.push((rest_rm, rest_role, wid));
                    did_split = true;
                    break;
                }
            } else if !split_v && rm.h() >= 4 {
                let want = (cap / rm.w().max(1)).clamp(2, max_bath_dim);
                if let Some(sy) = pick_h_cut(rm, rm.y0 + want, 2, 2, stair) {
                    let a = Room { y1: sy, ..rm };
                    let b = Room { y0: sy, ..rm };
                    let (bath_rm, rest_rm) = if a.area() <= b.area() { (a, b) } else { (b, a) };
                    let rest_role = if rest_rm.area() >= 10 {
                        Role::Bedroom
                    } else if rest_rm.area() >= 6 {
                        Role::Closet
                    } else {
                        Role::Hall
                    };
                    tagged[idx] = (bath_rm, role, wid);
                    tagged.push((rest_rm, rest_role, wid));
                    did_split = true;
                    break;
                }
            }
        }
        if !did_split {
            // If stair blocks splitting an oversized room, swap role with a compact non-bath room or turn into Bedroom.
            if let Some(swap_i) = tagged.iter().position(|(other, orole, _)| {
                other.area() <= cap
                    && !matches!(
                        orole,
                        Role::Hall
                            | Role::Foyer
                            | Role::Bathroom
                            | Role::MasterBath
                            | Role::Closet
                            | Role::Laundry
                    )
            }) {
                let small_role = tagged[swap_i].1;
                tagged[swap_i].1 = role;
                tagged[idx].1 = small_role;
            } else {
                tagged[idx].1 = if rm.area() >= 12 {
                    Role::Bedroom
                } else {
                    Role::Office
                };
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn partition_wing_rooms(
    wing: LotBox,
    stair: Option<StairAnchor>,
    floor_idx: i32,
    entry_side: CardinalEntry,
    is_main: bool,
    plan: HomePlanKind,
    rng: &mut Rng,
) -> Vec<(Room, Role, u8)> {
    let wing_id = u8::from(!is_main);
    let r = wing.as_room();
    let mut rooms: Vec<Room> = Vec::new();
    let mut preset_hall: Vec<Room> = Vec::new();

    if r.w() <= 5 || r.h() <= 4 {
        rooms.push(r);
    } else if floor_idx >= 0
        && r.w() >= 9
        && r.h() >= 7
        && let Some(sx1) = pick_v_cut(r, r.x0 + r.w() * rng.range(38, 52) / 100, 4, 4, stair)
    {
        let (pub_r, priv_r) = if entry_side == CardinalEntry::East {
            (Room { x0: sx1, ..r }, Room { x1: sx1, ..r })
        } else {
            (Room { x1: sx1, ..r }, Room { x0: sx1, ..r })
        };
        let pub_depth = if pub_r.area() >= 42 { 3 } else { 2 };
        subdivide_cluster(pub_r, stair, pub_depth, rng, &mut rooms);

        let hy0 = priv_r.y0 + priv_r.h() / 2 - 1;
        let hy1 = hy0 + 2;
        if priv_r.w() >= 5
            && hy0 >= priv_r.y0 + 2
            && priv_r.y1 - hy1 >= 2
            && cut_avoids_stair_h(priv_r, hy0, stair)
            && cut_avoids_stair_h(priv_r, hy1, stair)
        {
            let hall_r = Room {
                y0: hy0,
                y1: hy1,
                ..priv_r
            };
            preset_hall.push(hall_r);
            rooms.push(hall_r);
            let n_strip = Room { y1: hy0, ..priv_r };
            let s_strip = Room { y0: hy1, ..priv_r };
            if n_strip.h() <= 5 {
                split_along_strip_cols(n_strip, stair, 4, &mut rooms);
            } else {
                subdivide_cluster(n_strip, stair, 2, rng, &mut rooms);
            }
            if s_strip.h() <= 5 {
                split_along_strip_cols(s_strip, stair, 4, &mut rooms);
            } else {
                subdivide_cluster(s_strip, stair, 2, rng, &mut rooms);
            }
        } else {
            let priv_depth = if priv_r.area() >= 42 { 3 } else { 2 };
            subdivide_cluster(priv_r, stair, priv_depth, rng, &mut rooms);
        }
    } else if r.w() >= 10 && r.h() >= 6 {
        if let Some(sx1) = (r.w() >= r.h())
            .then(|| pick_v_cut(r, r.x0 + r.w() * rng.range(38, 52) / 100, 4, 4, stair))
            .flatten()
        {
            let d = if r.area() >= 64 { 3 } else { 2 };
            subdivide_cluster(Room { x1: sx1, ..r }, stair, d, rng, &mut rooms);
            subdivide_cluster(Room { x0: sx1, ..r }, stair, d, rng, &mut rooms);
        } else if let Some(sy1) = pick_h_cut(r, r.y0 + r.h() / 2, 3, 3, stair) {
            let d = if r.area() >= 64 { 3 } else { 2 };
            subdivide_cluster(Room { y1: sy1, ..r }, stair, d, rng, &mut rooms);
            subdivide_cluster(Room { y0: sy1, ..r }, stair, d, rng, &mut rooms);
        } else {
            rooms.push(r);
        }
    } else if let Some(sx) = pick_v_cut(r, r.x0 + r.w() / 2, 3, 3, stair) {
        subdivide_cluster(Room { x1: sx, ..r }, stair, 2, rng, &mut rooms);
        subdivide_cluster(Room { x0: sx, ..r }, stair, 2, rng, &mut rooms);
    } else if let Some(sy) = pick_h_cut(r, r.y0 + r.h() / 2, 3, 3, stair) {
        rooms.push(Room { y1: sy, ..r });
        rooms.push(Room { y0: sy, ..r });
    } else {
        rooms.push(r);
    }

    let mut tagged: Vec<(Room, Role, u8)> = assign_architectural_roles(
        &rooms, wing, entry_side, floor_idx, is_main, plan, stair, rng,
    )
    .into_iter()
    .map(|(rm, rl)| {
        let final_rl = if preset_hall.contains(&rm) {
            Role::Hall
        } else {
            rl
        };
        (rm, final_rl, wing_id)
    })
    .collect();
    enforce_service_room_caps(&mut tagged, stair, floor_idx, is_main);
    tagged
}

#[allow(clippy::too_many_arguments)]
fn assign_architectural_roles(
    rooms: &[Room],
    wing: LotBox,
    entry_side: CardinalEntry,
    floor_idx: i32,
    is_main: bool,
    plan: HomePlanKind,
    stair: Option<StairAnchor>,
    rng: &mut Rng,
) -> Vec<(Room, Role)> {
    let n = rooms.len();
    let mut roles = vec![Role::Living; n];
    if n == 0 {
        return Vec::new();
    }
    if floor_idx == -1 {
        const B_ROLES: [Role; 5] = [
            Role::Storage,
            Role::BarHall,
            Role::Laundry,
            Role::ServerCloset,
            Role::BackKitchen,
        ];
        for (i, r) in roles.iter_mut().enumerate() {
            *r = B_ROLES[i % B_ROLES.len()];
        }
        return rooms.iter().copied().zip(roles).collect();
    }

    let touches_entry = |rm: Room| match entry_side {
        CardinalEntry::North => rm.y0 == wing.y0,
        CardinalEntry::South => rm.y1 == wing.y1,
        CardinalEntry::West => rm.x0 == wing.x0,
        CardinalEntry::East => rm.x1 == wing.x1,
    };
    let touches_stair = |rm: Room| {
        stair.is_some_and(|st| {
            let (tx0, ty0, tx1, ty1) = stair_foot(st);
            rm.x0 <= tx0 && rm.x1 >= tx1 && rm.y0 <= ty0 && rm.y1 >= ty1
        })
    };

    let mut by_area: Vec<usize> = (0..n).collect();
    by_area.sort_by_key(|&i| -(rooms[i].area()));
    let mut assigned = vec![false; n];

    if floor_idx == 0 && is_main {
        if let Some(fi) = (0..n)
            .filter(|&i| !touches_stair(rooms[i]) && touches_entry(rooms[i]))
            .min_by_key(|&i| (rooms[i].area() - 10).abs())
            && n >= 3
            && rooms[fi].area() <= 18
        {
            roles[fi] = Role::Foyer;
            assigned[fi] = true;
        }
        if let Some(&li) = by_area
            .iter()
            .find(|&&i| !assigned[i] && touches_stair(rooms[i]))
            .or_else(|| by_area.iter().find(|&&i| !assigned[i]))
        {
            roles[li] = Role::Living;
            assigned[li] = true;
        }
        let large_palette: &[Role] = match plan {
            HomePlanKind::FarmhouseSunroom => &[
                Role::Kitchen,
                Role::Sunroom,
                Role::Dining,
                Role::MasterBedroom,
                Role::Bedroom,
                Role::Office,
            ],
            HomePlanKind::CourtyardVilla => &[
                Role::Dining,
                Role::Kitchen,
                Role::Sunroom,
                Role::MasterBedroom,
                Role::Bedroom,
                Role::Office,
            ],
            _ => &[
                Role::Kitchen,
                Role::Dining,
                Role::MasterBedroom,
                Role::Bedroom,
                Role::Office,
            ],
        };
        let compact_service = [Role::Bathroom, Role::Closet, Role::Laundry, Role::MasterBath];
        let (mut big_cur, mut small_cur) = (0usize, 0usize);
        for &idx in &by_area {
            if assigned[idx] {
                continue;
            }
            if rooms[idx].area() <= 14 || rooms[idx].w() <= 2 || rooms[idx].h() <= 2 {
                roles[idx] = compact_service[small_cur % compact_service.len()];
                small_cur += 1;
            } else {
                roles[idx] = large_palette[big_cur % large_palette.len()];
                big_cur += 1;
            }
        }
    } else {
        let arm_large = [
            Role::MasterBedroom,
            Role::Bedroom,
            Role::Sunroom,
            Role::Office,
            Role::Dining,
        ];
        let arm_compact = [Role::MasterBath, Role::Bathroom, Role::Closet, Role::Laundry];
        let (mut large_cur, mut comp_cur) = (0usize, 0usize);
        for (cur, &idx) in by_area.iter().enumerate() {
            if cur == 0 || (touches_stair(rooms[idx]) && rooms[idx].area() >= 10) {
                roles[idx] = if touches_stair(rooms[idx]) || rng.chance(1, 2) {
                    Role::Living
                } else {
                    Role::Office
                };
            } else if rooms[idx].area() <= 14 || rooms[idx].w() <= 2 || rooms[idx].h() <= 2 {
                roles[idx] = arm_compact[comp_cur % arm_compact.len()];
                comp_cur += 1;
            } else {
                roles[idx] = arm_large[large_cur % arm_large.len()];
                large_cur += 1;
            }
        }
    }
    rooms.iter().copied().zip(roles).collect()
}

const fn is_circulation_role(r: Role) -> bool {
    matches!(r, Role::Hall | Role::Foyer | Role::Living)
}

const fn role_door_weight(r: Role) -> i32 {
    match r {
        Role::Hall | Role::Foyer | Role::Living => 1,
        Role::Bathroom | Role::MasterBath | Role::Closet | Role::Laundry => 90,
        Role::Bedroom | Role::MasterBedroom | Role::Office => 30,
        _ => 8,
    }
}

const fn pair_door_tier(a: Role, b: Role) -> u8 {
    let ca = is_circulation_role(a);
    let cb = is_circulation_role(b);
    if ca && cb {
        0
    } else if (ca && matches!(b, Role::Bedroom | Role::MasterBedroom | Role::Office | Role::Dining | Role::Kitchen | Role::Sunroom))
        || (cb && matches!(a, Role::Bedroom | Role::MasterBedroom | Role::Office | Role::Dining | Role::Kitchen | Role::Sunroom))
    {
        1
    } else if (ca && matches!(b, Role::Bathroom | Role::MasterBath | Role::Closet | Role::Laundry))
        || (cb && matches!(a, Role::Bathroom | Role::MasterBath | Role::Closet | Role::Laundry))
        || (matches!(a, Role::Dining | Role::Kitchen | Role::Sunroom)
            && matches!(b, Role::Dining | Role::Kitchen | Role::Sunroom))
    {
        2
    } else if (matches!(a, Role::MasterBedroom | Role::Bedroom)
        && matches!(b, Role::MasterBath | Role::Bathroom | Role::Closet))
        || (matches!(b, Role::MasterBedroom | Role::Bedroom)
            && matches!(a, Role::MasterBath | Role::Bathroom | Role::Closet))
    {
        3
    } else {
        4
    }
}

const fn is_open_concept_pair(a: Role, b: Role) -> bool {
    matches!(
        (a, b),
        (Role::Kitchen, Role::Dining)
            | (Role::Dining, Role::Kitchen)
            | (Role::Dining, Role::Living)
            | (Role::Living, Role::Dining)
            | (Role::Foyer, Role::Living)
            | (Role::Living, Role::Foyer)
    )
}

fn connect_rooms_with_doors(shell: &mut Shell, tagged: &[(Room, Role, u8)], is_villa: bool) {
    let mut borders: Vec<SharedBorder> = Vec::new();
    for i in 0..tagged.len() {
        for j in (i + 1)..tagged.len() {
            let ((ra, role_a, wa), (rb, role_b, wb)) = (tagged[i], tagged[j]);
            let same_wing = wa == wb;
            let weight = role_door_weight(role_a).max(role_door_weight(role_b));
            if ra.x1 == rb.x0 || rb.x1 == ra.x0 {
                let fx = if ra.x1 == rb.x0 { ra.x1 } else { rb.x1 };
                let (s, e) = (ra.y0.max(rb.y0), ra.y1.min(rb.y1));
                if e > s {
                    for y in s..e {
                        shell.put(
                            fx,
                            y,
                            EdgeSide::West,
                            WallEdgeKind::Wall,
                            MaterialClass::Drywall,
                        );
                    }
                    borders.push(SharedBorder {
                        a: i,
                        b: j,
                        vertical: true,
                        fixed_coord: fx,
                        start: s,
                        end: e,
                        same_wing,
                        weight,
                    });
                }
            }
            if ra.y1 == rb.y0 || rb.y1 == ra.y0 {
                let fy = if ra.y1 == rb.y0 { ra.y1 } else { rb.y1 };
                let (s, e) = (ra.x0.max(rb.x0), ra.x1.min(rb.x1));
                if e > s {
                    for x in s..e {
                        shell.put(
                            x,
                            fy,
                            EdgeSide::North,
                            WallEdgeKind::Wall,
                            MaterialClass::Drywall,
                        );
                    }
                    borders.push(SharedBorder {
                        a: i,
                        b: j,
                        vertical: false,
                        fixed_coord: fy,
                        start: s,
                        end: e,
                        same_wing,
                        weight,
                    });
                }
            }
        }
    }

    borders.sort_by_key(|b| {
        (
            u8::from(!b.same_wing),
            pair_door_tier(tagged[b.a].1, tagged[b.b].1),
            b.weight,
            -(b.end - b.start),
        )
    });
    let mut parent: Vec<usize> = (0..tagged.len()).collect();
    let mut carved = vec![false; borders.len()];
    for pass in [true, false] {
        for (bi, bd) in borders.iter().enumerate() {
            if bd.same_wing != pass {
                continue;
            }
            let (ra, rb) = (dsu_find(&mut parent, bd.a), dsu_find(&mut parent, bd.b));
            if ra != rb {
                parent[rb] = ra;
                carved[bi] = true;
            }
        }
    }

    // Ensure every Bedroom / MasterBedroom touching Hall/Living/Foyer has direct circulation access.
    for i in 0..tagged.len() {
        if !matches!(tagged[i].1, Role::Bedroom | Role::MasterBedroom) {
            continue;
        }
        let has_circ = borders.iter().enumerate().any(|(bi, bd)| {
            if !carved[bi] {
                return false;
            }
            let other = if bd.a == i {
                Some(bd.b)
            } else if bd.b == i {
                Some(bd.a)
            } else {
                None
            };
            other.is_some_and(|o| is_circulation_role(tagged[o].1))
        });
        if !has_circ
            && let Some(bi) = borders.iter().position(|bd| {
                let other = if bd.a == i {
                    Some(bd.b)
                } else if bd.b == i {
                    Some(bd.a)
                } else {
                    None
                };
                other.is_some_and(|o| is_circulation_role(tagged[o].1))
            })
        {
            carved[bi] = true;
        }
    }

    for (bi, bd) in borders.iter().enumerate() {
        if carved[bi] || bd.end - bd.start < 2 {
            continue;
        }
        let social = |r| {
            matches!(
                r,
                Role::Foyer | Role::Living | Role::Dining | Role::Kitchen | Role::Sunroom | Role::Hall
            )
        };
        if social(tagged[bd.a].1) && social(tagged[bd.b].1) && (is_villa || shell.rng.chance(1, 2))
        {
            carved[bi] = true;
        }
    }

    for (bi, bd) in borders.iter().enumerate() {
        if !carved[bi] {
            continue;
        }
        let (ra_role, rb_role) = (tagged[bd.a].1, tagged[bd.b].1);
        if is_open_concept_pair(ra_role, rb_role) && bd.end - bd.start >= 2 {
            // Open passage with zero interior wall dividing connected social spaces.
            shell.edges.retain(|e| {
                if bd.vertical {
                    !(e.side == EdgeSide::West
                        && e.x == bd.fixed_coord
                        && e.y >= bd.start
                        && e.y < bd.end)
                } else {
                    !(e.side == EdgeSide::North
                        && e.y == bd.fixed_coord
                        && e.x >= bd.start
                        && e.x < bd.end)
                }
            });
            continue;
        }
        let open_arch = matches!(
            (ra_role, rb_role),
            (Role::Living, Role::Sunroom)
                | (Role::Sunroom, Role::Living)
                | (Role::Hall, Role::Living)
                | (Role::Living, Role::Hall)
                | (Role::Hall, Role::Foyer)
                | (Role::Foyer, Role::Hall)
        );
        let kind = if open_arch {
            WallEdgeKind::DoorOpen
        } else {
            shell.door_kind()
        };
        let mat = if open_arch {
            MaterialClass::SolidWood
        } else {
            MaterialClass::Drywall
        };
        let mid = bd.start + (bd.end - bd.start) / 2;
        if bd.vertical {
            shell.put(bd.fixed_coord, mid, EdgeSide::West, kind, mat);
            if open_arch && bd.end - bd.start >= 3 {
                shell.put(
                    bd.fixed_coord,
                    mid - 1,
                    EdgeSide::West,
                    WallEdgeKind::DoorOpen,
                    mat,
                );
            }
        } else {
            shell.put(mid, bd.fixed_coord, EdgeSide::North, kind, mat);
            if open_arch && bd.end - bd.start >= 3 {
                shell.put(
                    mid - 1,
                    bd.fixed_coord,
                    EdgeSide::North,
                    WallEdgeKind::DoorOpen,
                    mat,
                );
            }
        }
    }
}

/// Builds one floor (`B1`, `1F`, or `2F`) inside a `HomeBlueprint`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_blueprint_floor(
    board_w: i32,
    board_h: i32,
    bp: &HomeBlueprint,
    tier: EconomicTier,
    entry_side: CardinalEntry,
    floor_idx: i32,
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
        rng: Rng(seed ^ ((floor_idx + 5) as u64).wrapping_mul(0x9E37_79B9)),
        edges: Vec::new(),
    };
    let win_step = if floor_idx < 0 {
        0
    } else if tier == EconomicTier::LowIncome {
        4
    } else {
        3
    };
    let other_wing = if floor_idx < 0 { None } else { bp.arm_wing };

    enclose_wing_perimeter(&mut shell, bp.main_wing, other_wing, exterior_mat, win_step);
    if let Some(arm) = other_wing {
        enclose_wing_perimeter(&mut shell, arm, Some(bp.main_wing), exterior_mat, win_step);
    }

    let mut tagged_rooms = partition_wing_rooms(
        bp.main_wing,
        bp.stair,
        floor_idx,
        entry_side,
        true,
        bp.plan,
        &mut shell.rng,
    );
    if let Some(arm) = other_wing {
        tagged_rooms.extend(partition_wing_rooms(
            arm,
            None,
            floor_idx,
            entry_side,
            false,
            bp.plan,
            &mut shell.rng,
        ));
    }

    connect_rooms_with_doors(
        &mut shell,
        &tagged_rooms,
        bp.plan == HomePlanKind::CourtyardVilla,
    );
    if floor_idx == 0 {
        add_ground_entry_and_patio_doors(&mut shell, bp, entry_side, rear_door);
    }

    let mut furn = Furnisher::new(board_w, board_h, shell.rng.next());
    furn.reserve_doors(&shell.edges);
    for &(sx, sy) in extra_reserves {
        furn.reserve_cell(sx.floor() as i32, sy.floor() as i32);
    }
    if let Some(st) = bp.stair {
        let prop = make_prop(PropKind::Stairwell, st.x, st.y, st.rot_deg, None, 0);
        for yy in prop.y..(prop.y + prop.h_cells) {
            for xx in prop.x..(prop.x + prop.w_cells) {
                if prop.covers(xx, yy) {
                    furn.reserve_cell(xx, yy);
                }
            }
        }
        furn.props.push(prop);
    }
    for &(room, role, _) in &tagged_rooms {
        furn.furnish(role, room);
    }
    if floor_idx == -1 {
        let _ = furn.against_any_wall(PropKind::SteelShelving, bp.main_wing.as_room(), None);
        furn.scatter(PropKind::WoodCrate, bp.main_wing.as_room(), 2, 0);
    } else if tier == EconomicTier::LuxuryMansion && bp.main_wing.w() >= 14 {
        let _ = furn.against_any_wall(PropKind::StructuralPillar, bp.main_wing.as_room(), None);
    }

    let mut surfaces =
        vec![
            bp.main_wing
                .polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, bp.hardwood_seed),
        ];
    if let Some(arm) = other_wing {
        surfaces.push(arm.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, bp.hardwood_seed));
    }
    if floor_idx >= 0 {
        // Ensure each wing retains at least one exposed hardwood InteriorFloor room so prune never drops it.
        for wing_id in [0u8, 1u8] {
            let mut overlays: Vec<(usize, MapSurfaceFeature)> = Vec::new();
            let mut has_wood = false;
            for (idx, &(room, role, wid)) in tagged_rooms.iter().enumerate() {
                if wid != wing_id {
                    continue;
                }
                if let Some(sf) = emit_room_floor_surface(room, role, tier, &mut shell.rng) {
                    overlays.push((idx, sf));
                } else {
                    has_wood = true;
                }
            }
            if !has_wood && !overlays.is_empty() {
                overlays.remove(0);
            }
            surfaces.extend(overlays.into_iter().map(|(_, s)| s));
        }
    }

    if floor_idx == 0 {
        add_attached_patio_and_exterior(&mut furn, bp, tier, &mut surfaces, &mut shell.rng);
    }

    trim_floor_props_to_connected(
        board_w,
        board_h,
        bp.main_wing,
        &shell.edges,
        &mut furn.props,
    );
    if let Some(arm) = other_wing {
        trim_floor_props_to_connected(board_w, board_h, arm, &shell.edges, &mut furn.props);
    }
    prune_occluded_surfaces(&mut surfaces, board_w, board_h);

    FloorContent {
        wall_edges: shell.edges,
        props: furn.props,
        surfaces,
    }
}

fn add_ground_entry_and_patio_doors(
    shell: &mut Shell,
    bp: &HomeBlueprint,
    entry_side: CardinalEntry,
    rear_door: bool,
) {
    let w = bp.main_wing;
    let (mx, my) = (w.x0 + w.w() / 2, w.y0 + w.h() / 2);
    let dk = if shell.rng.chance(2, 3) {
        WallEdgeKind::DoorOpen
    } else {
        WallEdgeKind::DoorClosed
    };
    match entry_side {
        CardinalEntry::North => {
            shell.put(mx, w.y0, EdgeSide::North, dk, MaterialClass::SolidWood);
            if rear_door {
                shell.put(
                    mx,
                    w.y1,
                    EdgeSide::North,
                    WallEdgeKind::DoorOpen,
                    MaterialClass::SolidWood,
                );
            }
        }
        CardinalEntry::South => {
            shell.put(mx, w.y1, EdgeSide::North, dk, MaterialClass::SolidWood);
            if rear_door {
                shell.put(
                    mx,
                    w.y0,
                    EdgeSide::North,
                    WallEdgeKind::DoorOpen,
                    MaterialClass::SolidWood,
                );
            }
        }
        CardinalEntry::West => {
            shell.put(w.x0, my, EdgeSide::West, dk, MaterialClass::SolidWood);
            if rear_door {
                shell.put(
                    w.x1,
                    my,
                    EdgeSide::West,
                    WallEdgeKind::DoorOpen,
                    MaterialClass::SolidWood,
                );
            }
        }
        CardinalEntry::East => {
            shell.put(w.x1, my, EdgeSide::West, dk, MaterialClass::SolidWood);
            if rear_door {
                shell.put(
                    w.x0,
                    my,
                    EdgeSide::West,
                    WallEdgeKind::DoorOpen,
                    MaterialClass::SolidWood,
                );
            }
        }
    }
    if let Some(p) = bp.patio_box {
        if p.y1 == w.y0 && p.x1 > w.x0 && p.x0 < w.x1 {
            shell.put(
                (p.x0.max(w.x0) + p.x1.min(w.x1)) / 2,
                w.y0,
                EdgeSide::North,
                WallEdgeKind::DoorOpen,
                MaterialClass::Glass,
            );
        } else if p.y0 == w.y1 && p.x1 > w.x0 && p.x0 < w.x1 {
            shell.put(
                (p.x0.max(w.x0) + p.x1.min(w.x1)) / 2,
                w.y1,
                EdgeSide::North,
                WallEdgeKind::DoorOpen,
                MaterialClass::Glass,
            );
        }
        if let Some(arm) = bp.arm_wing {
            let dy = (p.y0.max(arm.y0) + p.y1.min(arm.y1)) / 2;
            if p.x0 == arm.x1 && p.y1 > arm.y0 && p.y0 < arm.y1 {
                shell.put(
                    arm.x1,
                    dy,
                    EdgeSide::West,
                    WallEdgeKind::DoorOpen,
                    MaterialClass::Glass,
                );
            } else if p.x1 == arm.x0 && p.y1 > arm.y0 && p.y0 < arm.y1 {
                shell.put(
                    arm.x0,
                    dy,
                    EdgeSide::West,
                    WallEdgeKind::DoorOpen,
                    MaterialClass::Glass,
                );
            }
        }
    }
}

fn add_attached_patio_and_exterior(
    furn: &mut Furnisher,
    bp: &HomeBlueprint,
    tier: EconomicTier,
    surfaces: &mut Vec<MapSurfaceFeature>,
    rng: &mut Rng,
) {
    let Some(patio) = bp.patio_box else { return };
    let patio_kind = if bp.pool_in_patio && matches!(tier, EconomicTier::LuxuryMansion) {
        SurfaceFeatureKind::StoneWalk
    } else {
        SurfaceFeatureKind::WoodDeck
    };
    surfaces.push(patio.polygon_feature(patio_kind, 22, rng.next() as u32));
    let pr = patio.as_room();

    if bp.pool_in_patio && patio.w() >= 5 && patio.h() >= 4 {
        let (pw, ph) = ((patio.w() - 2).clamp(3, 6), (patio.h() - 2).clamp(2, 4));
        let (px0, py0) = (patio.x0 + 1, patio.y0 + 1);
        let pool = LotBox::new(px0, py0, px0 + pw, py0 + ph);
        surfaces.push(pool.polygon_feature(SurfaceFeatureKind::PoolWater, 32, rng.next() as u32));
        for yy in pool.y0..pool.y1 {
            for xx in pool.x0..pool.x1 {
                furn.reserve_cell(xx, yy);
            }
        }
    } else if bp.hot_tub_in_patio {
        let _ = furn.try_place(
            PropKind::HotTub,
            patio.x0 + (patio.w() - 2).max(0) / 2,
            patio.y0 + (patio.h() - 2).max(0) / 2,
            0,
            None,
            pr,
        );
    }

    let _ = furn.against_any_wall(PropKind::BbqGrill, pr, None);
    let _ = furn
        .against_any_wall(PropKind::RoundTable, pr, None)
        .or_else(|| furn.against_any_wall(PropKind::PicnicTable, pr, None));
    let _ = furn.against_any_wall(PropKind::AcUnit, pr, None);
    let _ = furn.against_any_wall(PropKind::TrashCan, pr, None);
}
