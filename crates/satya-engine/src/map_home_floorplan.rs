//! # Architectural Floorplan Graph & Multi-Wing Home Generator (`map_home_floorplan`)
//!
//! Circulation-first architectural generator for residential units (`B1`, `1F`, `2F`):
//! - Varied architectural footprints (`LShaped`, `OpenConceptFoyer`, `FarmhouseSunroom`,
//!   `CourtyardVilla`, `RowhouseOrApt`) with attached patios, multi-shape pools, and styled spas.
//! - Scale-aware entries: small cottages/apartments/rowhouses omit marble foyers and open
//!   directly into `Living`, `Dining`, or `Kitchen`, while larger affluent homes include a `Foyer`
//!   feeding a continuous `Hall` circulation spine with vertically aligned `Stairwell` shafts.
//! - Unified whole-home room program across `B1`/`1F`/`2F`: hard-caps kitchens (`<= 1`), laundry
//!   rooms (`<= 1` whole house), ground bathrooms (`1` full + at most `1` `HalfBath`), and upper
//!   baths (`<= 2`), while adding `Sunroom`, `Library`, `Office`, `Garage`, and `HomeGym` in larger
//!   homes and keeping every non-en-suite room directly accessible from `Hall`/`Living` without
//!   traversing private bedrooms or bathrooms.

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

/// Feature toggles separating standalone primary residences from attached estate guest/annex wings.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HomeGenConfig {
    pub(crate) allow_kitchen: bool,
    pub(crate) allow_laundry: bool,
    pub(crate) allow_garage: bool,
}

impl HomeGenConfig {
    pub(crate) const PRIMARY: Self = Self {
        allow_kitchen: true,
        allow_laundry: true,
        allow_garage: true,
    };

    pub(crate) const ANNEX: Self = Self {
        allow_kitchen: false,
        allow_laundry: false,
        allow_garage: false,
    };
}

/// Architectural silhouette, whole-home facility budget, and stair placement across `B1`/`1F`/`2F`.
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
    pub(crate) use_foyer: bool,
    pub(crate) laundry_floor: Option<i32>,
    pub(crate) has_upper: bool,
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

#[derive(Debug, Clone, Copy)]
struct RoomBay {
    room: Room,
    wing_id: u8,
    fixed_role: Option<Role>,
    is_entry_center: bool,
    is_compact: bool,
    min_depth: i32,
    exterior_touches_entry_or_side: bool,
    on_patio_or_corner: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpineMode {
    Horizontal { hy0: i32, hy1: i32 },
    Vertical { hx0: i32, hx1: i32 },
    Rowhouse,
    Compact,
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

const fn classify_spine(wing: LotBox) -> SpineMode {
    let (w, h) = (wing.w(), wing.h());
    if w >= 8 && h >= 7 && w >= h {
        let hall_h = if h >= 13 { 3 } else { 2 };
        let hy0 = wing.y0 + (h - hall_h) / 2;
        SpineMode::Horizontal {
            hy0,
            hy1: hy0 + hall_h,
        }
    } else if w >= 7 && h >= 8 {
        let hall_w = if w >= 13 { 3 } else { 2 };
        let hx0 = wing.x0 + (w - hall_w) / 2;
        SpineMode::Vertical {
            hx0,
            hx1: hx0 + hall_w,
        }
    } else if w >= 5 && h >= 7 {
        SpineMode::Rowhouse
    } else {
        SpineMode::Compact
    }
}

fn compute_spine_stair(wing: LotBox, entry_side: CardinalEntry) -> Option<StairAnchor> {
    match classify_spine(wing) {
        SpineMode::Horizontal { hy0, .. } => {
            let mx = wing.x0 + wing.w() / 2;
            let sx = match entry_side {
                CardinalEntry::North | CardinalEntry::South => {
                    if mx + 5 < wing.x1 {
                        mx + 2
                    } else if mx - 5 > wing.x0 {
                        mx - 5
                    } else {
                        (mx + 1).clamp(wing.x0 + 1, wing.x1 - 4)
                    }
                }
                CardinalEntry::West => mx.clamp(wing.x0 + 2, wing.x1 - 4),
                CardinalEntry::East => (mx - 3).clamp(wing.x0 + 1, wing.x1 - 5),
            };
            Some(StairAnchor {
                x: sx,
                y: hy0,
                rot_deg: 90,
            })
        }
        SpineMode::Vertical { hx0, .. } => {
            let my = wing.y0 + wing.h() / 2;
            let sy = match entry_side {
                CardinalEntry::West | CardinalEntry::East => {
                    if my + 5 < wing.y1 {
                        my + 2
                    } else if my - 5 > wing.y0 {
                        my - 5
                    } else {
                        (my + 1).clamp(wing.y0 + 1, wing.y1 - 4)
                    }
                }
                CardinalEntry::North => my.clamp(wing.y0 + 2, wing.y1 - 4),
                CardinalEntry::South => (my - 3).clamp(wing.y0 + 1, wing.y1 - 5),
            };
            Some(StairAnchor {
                x: hx0,
                y: sy,
                rot_deg: 0,
            })
        }
        SpineMode::Rowhouse => {
            let parlor_d = if wing.h() >= 9 { 4 } else { 3 };
            let (hx, sy) = match entry_side {
                CardinalEntry::North => {
                    let ry0 = wing.y0 + parlor_d;
                    let rh = wing.y1 - ry0;
                    (wing.x0, ry0 + (rh - 3).max(0) / 2)
                }
                CardinalEntry::South => {
                    let ry1 = wing.y1 - parlor_d;
                    let rh = ry1 - wing.y0;
                    (wing.x0, wing.y0 + (rh - 3).max(0) / 2)
                }
                CardinalEntry::West | CardinalEntry::East => {
                    let ry0 = if wing.h() >= 7 { wing.y0 + 3 } else { wing.y0 };
                    let hx = if entry_side == CardinalEntry::West {
                        wing.x0
                    } else {
                        wing.x1 - 2
                    };
                    (hx, ry0 + (wing.y1 - ry0 - 3).max(0) / 2)
                }
            };
            Some(StairAnchor {
                x: hx,
                y: sy,
                rot_deg: 0,
            })
        }
        SpineMode::Compact => (wing.w() >= 4 && wing.h() >= 4).then_some(StairAnchor {
            x: wing.x0,
            y: wing.y0,
            rot_deg: 0,
        }),
    }
}

/// Computes the silhouette, attached patio/deck/pool zone, and spine-aligned stair inside `lot`.
#[must_use]
pub(crate) fn plan_home_blueprint(
    lot: LotBox,
    tier: EconomicTier,
    entry_side: CardinalEntry,
    allow_upper_floor: bool,
    allow_basement: bool,
    cfg: HomeGenConfig,
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
                let deck_d = if lot.h() >= 14 { 4 } else { 3 };
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
            } else if lot.w() >= 13
                && matches!(entry_side, CardinalEntry::West | CardinalEntry::East)
            {
                let deck_w = if lot.w() >= 16 { 4 } else { 3 };
                match entry_side {
                    CardinalEntry::West => {
                        patio_box = Some(LotBox::new(lot.x1 - deck_w, lot.y0, lot.x1, lot.y1));
                        LotBox::new(lot.x0, lot.y0, lot.x1 - deck_w, lot.y1)
                    }
                    CardinalEntry::East => {
                        patio_box = Some(LotBox::new(lot.x0, lot.y0, lot.x0 + deck_w, lot.y1));
                        LotBox::new(lot.x0 + deck_w, lot.y0, lot.x1, lot.y1)
                    }
                    _ => lot,
                }
            } else {
                lot
            }
        }
        HomePlanKind::RowhouseOrApt => lot,
    };

    let stair = if allow_upper_floor || allow_basement {
        compute_spine_stair(main_wing, entry_side)
    } else {
        None
    };
    let has_upper =
        allow_upper_floor && stair.is_some() && main_wing.w() >= 5 && main_wing.h() >= 4;

    let total_area = main_wing.area() + arm_wing.map_or(0, LotBox::area);
    let use_foyer = cfg.allow_kitchen
        && total_area >= 108
        && main_wing.w() >= 12
        && main_wing.h() >= 8
        && match tier {
            EconomicTier::LowIncome => false,
            EconomicTier::MiddleClass => total_area >= 135 && rng.chance(1, 2),
            EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion => true,
        };

    let laundry_floor = if !cfg.allow_laundry {
        None
    } else if total_area >= 72 {
        Some(0)
    } else if allow_basement && stair.is_some() && main_wing.w() >= 6 && main_wing.h() >= 5 {
        Some(-1)
    } else if total_area >= 52 {
        Some(0)
    } else {
        None
    };

    let can_pool = patio_box.is_some_and(|p| p.w() >= 5 && p.h() >= 4);
    let can_spa = patio_box.is_some_and(|p| p.w() >= 3 && p.h() >= 3);
    let (pool_in_patio, hot_tub_in_patio) = match tier {
        EconomicTier::LuxuryMansion => {
            let pool = can_pool && rng.chance(3, 4);
            let tub = can_spa
                && (!pool || (patio_box.is_some_and(|p| p.area() >= 30) && rng.chance(1, 2)));
            (pool, tub)
        }
        EconomicTier::UpperMiddleClass => {
            let pool = can_pool && rng.chance(1, 2);
            let tub = can_spa && !pool && rng.chance(3, 5);
            (pool, tub)
        }
        EconomicTier::MiddleClass => {
            let pool = can_pool && rng.chance(1, 4);
            let tub = can_spa && !pool && rng.chance(1, 3);
            (pool, tub)
        }
        EconomicTier::LowIncome => (false, false),
    };

    HomeBlueprint {
        plan,
        main_wing,
        arm_wing,
        patio_box,
        pool_in_patio,
        hot_tub_in_patio,
        stair,
        hardwood_seed,
        use_foyer,
        laundry_floor,
        has_upper,
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

/// Carves realistic bay spans `[len_0, len_1, ...]` along a strip of length `len` & depth `depth`,
/// reserving at most `compact_budget` compact `2..3m` service bays (`Bathroom`/`HalfBath`/`Laundry`).
fn slice_strip_segments(
    len: i32,
    depth: i32,
    compact_budget: &mut i32,
    rng: &mut Rng,
) -> Vec<(i32, bool)> {
    if len <= 0 {
        return Vec::new();
    }
    if len < 4 {
        return vec![(len, depth <= 4 && len <= 3)];
    }

    let mut out = Vec::new();
    let mut rem = len;
    let mut trailing_compact: Option<i32> = None;

    if *compact_budget > 0 && rem >= 6 {
        let cw = if depth >= 5 { 2 } else { 3 };
        if rem - cw >= 3 {
            *compact_budget -= 1;
            rem -= cw;
            if rng.chance(1, 2) {
                out.push((cw, true));
            } else {
                trailing_compact = Some(cw);
            }
        }
    }
    if *compact_budget > 0 && rem >= 6 {
        let cw = if depth >= 5 { 2 } else { 3 };
        if rem - cw >= 4 {
            *compact_budget -= 1;
            rem -= cw;
            if trailing_compact.is_none() && rng.chance(1, 2) {
                trailing_compact = Some(cw);
            } else {
                out.push((cw, true));
            }
        }
    }

    let target_w = if !(3..=4).contains(&depth) { 5 } else { 4 };
    let count = ((rem + target_w / 2) / target_w).clamp(1, (rem / 3).max(1));
    let base = rem / count;
    let extra = rem % count;
    for i in 0..count {
        let seg = base + i32::from(i < extra);
        out.push((seg, seg <= 3 && depth <= 4));
    }
    if let Some(cw) = trailing_compact {
        out.push((cw, true));
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn push_strip_bays(
    out: &mut Vec<RoomBay>,
    horizontal_strip: bool,
    fixed0: i32,
    fixed1: i32,
    span0: i32,
    span1: i32,
    wing_id: u8,
    wing: LotBox,
    patio_box: Option<LotBox>,
    entry_side: CardinalEntry,
    compact_budget: &mut i32,
    rng: &mut Rng,
) {
    let depth = fixed1 - fixed0;
    let segs = slice_strip_segments(span1 - span0, depth, compact_budget, rng);
    let mut cur = span0;
    for (slen, is_compact) in segs {
        let room = if horizontal_strip {
            Room {
                x0: cur,
                y0: fixed0,
                x1: cur + slen,
                y1: fixed1,
            }
        } else {
            Room {
                x0: fixed0,
                y0: cur,
                x1: fixed1,
                y1: cur + slen,
            }
        };
        cur += slen;
        let touches_entry = match entry_side {
            CardinalEntry::North => room.y0 == wing.y0,
            CardinalEntry::South => room.y1 == wing.y1,
            CardinalEntry::West => room.x0 == wing.x0,
            CardinalEntry::East => room.x1 == wing.x1,
        };
        let touches_patio = patio_box.is_some_and(|p| {
            room.x1 == p.x0 || room.x0 == p.x1 || room.y1 == p.y0 || room.y0 == p.y1
        });
        let at_corner = (room.x0 == wing.x0 || room.x1 == wing.x1)
            && (room.y0 == wing.y0 || room.y1 == wing.y1);
        out.push(RoomBay {
            room,
            wing_id,
            fixed_role: None,
            is_entry_center: false,
            is_compact: is_compact || depth <= 2 && room.area() <= 10,
            min_depth: room.w().min(room.h()),
            exterior_touches_entry_or_side: touches_entry || at_corner,
            on_patio_or_corner: touches_patio || at_corner,
        });
    }
}

/// Partitions a wing along a continuous `Hall` spine (+ optional `Foyer` or direct social entry),
/// guaranteeing zero corridor-less pass-through layouts whenever dimensions permit.
#[allow(clippy::too_many_arguments)]
fn partition_wing_bays(
    wing: LotBox,
    wing_id: u8,
    seam_with_other: Option<LotBox>,
    stair: Option<StairAnchor>,
    patio_box: Option<LotBox>,
    entry_side: CardinalEntry,
    use_foyer: bool,
    floor_idx: i32,
    compact_budget: &mut i32,
    rng: &mut Rng,
) -> Vec<RoomBay> {
    let mut bays = Vec::new();
    let is_main = wing_id == 0;

    match classify_spine(wing) {
        SpineMode::Horizontal { hy0, hy1 } => {
            // Optional Foyer / Hall split at West/East end when entry is on West/East.
            let mut hall_x0 = wing.x0;
            let mut hall_x1 = wing.x1;
            if is_main
                && floor_idx == 0
                && use_foyer
                && matches!(entry_side, CardinalEntry::West | CardinalEntry::East)
            {
                let fw = 3;
                if entry_side == CardinalEntry::West && wing.x0 + fw + 3 <= wing.x1 {
                    let foyer = Room {
                        x0: wing.x0,
                        y0: hy0,
                        x1: wing.x0 + fw,
                        y1: hy1,
                    };
                    if stair.is_none_or(|st| stair_foot(st).0 >= foyer.x1) {
                        hall_x0 = foyer.x1;
                        bays.push(RoomBay {
                            room: foyer,
                            wing_id,
                            fixed_role: Some(Role::Foyer),
                            is_entry_center: true,
                            is_compact: false,
                            min_depth: foyer.w().min(foyer.h()),
                            exterior_touches_entry_or_side: true,
                            on_patio_or_corner: false,
                        });
                    }
                } else if entry_side == CardinalEntry::East && wing.x1 - fw - 3 >= wing.x0 {
                    let foyer = Room {
                        x0: wing.x1 - fw,
                        y0: hy0,
                        x1: wing.x1,
                        y1: hy1,
                    };
                    if stair.is_none_or(|st| stair_foot(st).2 <= foyer.x0) {
                        hall_x1 = foyer.x0;
                        bays.push(RoomBay {
                            room: foyer,
                            wing_id,
                            fixed_role: Some(Role::Foyer),
                            is_entry_center: true,
                            is_compact: false,
                            min_depth: foyer.w().min(foyer.h()),
                            exterior_touches_entry_or_side: true,
                            on_patio_or_corner: false,
                        });
                    }
                }
            }

            let hall_room = Room {
                x0: hall_x0,
                y0: hy0,
                x1: hall_x1,
                y1: hy1,
            };
            bays.push(RoomBay {
                room: hall_room,
                wing_id,
                fixed_role: Some(Role::Hall),
                is_entry_center: is_main
                    && floor_idx == 0
                    && !use_foyer
                    && matches!(entry_side, CardinalEntry::West | CardinalEntry::East),
                is_compact: false,
                min_depth: hall_room.h(),
                exterior_touches_entry_or_side: false,
                on_patio_or_corner: false,
            });

            // Build North [wing.y0..hy0] & South [hy1..wing.y1] strips with entry portal & seam hall connector.
            for (is_north, sy0, sy1) in [(true, wing.y0, hy0), (false, hy1, wing.y1)] {
                let is_entry_strip = is_main
                    && floor_idx == 0
                    && ((is_north && entry_side == CardinalEntry::North)
                        || (!is_north && entry_side == CardinalEntry::South));

                // Seam connector interval if attached L-wing touches this face.
                let seam_conn = seam_with_other.and_then(|o| {
                    let touches = if is_north {
                        o.y1 == wing.y0
                    } else {
                        o.y0 == wing.y1
                    };
                    if !touches {
                        return None;
                    }
                    let ox0 = wing.x0.max(o.x0);
                    let ox1 = wing.x1.min(o.x1);
                    (ox1 - ox0 >= 3).then(|| {
                        let cx0 = (ox0 + (ox1 - ox0) / 2 - 1).clamp(wing.x0, wing.x1 - 2);
                        let cx0 = if cx0 - wing.x0 == 1 { wing.x0 } else { cx0 };
                        let cx1 = if wing.x1 - (cx0 + 2) == 1 {
                            wing.x1
                        } else {
                            cx0 + 2
                        };
                        (cx0, cx1)
                    })
                });

                let mut blocked_spans: Vec<(i32, i32, Option<Role>, bool)> = Vec::new();
                if let Some((cx0, cx1)) = seam_conn {
                    blocked_spans.push((cx0, cx1, Some(Role::Hall), false));
                }

                if is_entry_strip {
                    let mx = wing.x0 + wing.w() / 2;
                    let target_w = if use_foyer {
                        if wing.w() >= 16 { 4 } else { 3 }
                    } else {
                        (wing.w() * 48 / 100).clamp(4, 7)
                    };
                    let mut ex0 = (mx - target_w / 2).clamp(wing.x0, wing.x1 - target_w);
                    let mut ex1 = ex0 + target_w;
                    // Avoid leaving narrow <3m slivers unless absorbed into seam connector.
                    if seam_conn.is_none_or(|(cx0, cx1)| ex1 <= cx0 || ex0 >= cx1) {
                        if ex0 - wing.x0 < 3 {
                            ex0 = wing.x0;
                        }
                        if wing.x1 - ex1 < 3 {
                            ex1 = wing.x1;
                        }
                        blocked_spans.push((
                            ex0,
                            ex1,
                            if use_foyer { Some(Role::Foyer) } else { None },
                            true,
                        ));
                    }
                }

                blocked_spans.sort_by_key(|s| s.0);
                // Resolve rare overlap between seam_conn and entry span by dropping non-entry merge overlap.
                let mut clean_spans: Vec<(i32, i32, Option<Role>, bool)> = Vec::new();
                for sp in blocked_spans {
                    if let Some(prev) = clean_spans.last()
                        && sp.0 < prev.1
                    {
                        continue;
                    }
                    clean_spans.push(sp);
                }

                let mut cur = wing.x0;
                for (bx0, bx1, frole, is_entry) in clean_spans {
                    if bx0 > cur {
                        push_strip_bays(
                            &mut bays,
                            true,
                            sy0,
                            sy1,
                            cur,
                            bx0,
                            wing_id,
                            wing,
                            patio_box,
                            entry_side,
                            compact_budget,
                            rng,
                        );
                    }
                    let special = Room {
                        x0: bx0,
                        y0: sy0,
                        x1: bx1,
                        y1: sy1,
                    };
                    bays.push(RoomBay {
                        room: special,
                        wing_id,
                        fixed_role: frole,
                        is_entry_center: is_entry,
                        is_compact: false,
                        min_depth: special.w().min(special.h()),
                        exterior_touches_entry_or_side: true,
                        on_patio_or_corner: false,
                    });
                    cur = bx1;
                }
                if wing.x1 > cur {
                    push_strip_bays(
                        &mut bays,
                        true,
                        sy0,
                        sy1,
                        cur,
                        wing.x1,
                        wing_id,
                        wing,
                        patio_box,
                        entry_side,
                        compact_budget,
                        rng,
                    );
                }
            }
        }
        SpineMode::Vertical { hx0, hx1 } => {
            let mut hall_y0 = wing.y0;
            let mut hall_y1 = wing.y1;
            if is_main
                && floor_idx == 0
                && use_foyer
                && matches!(entry_side, CardinalEntry::North | CardinalEntry::South)
            {
                let fh = 3;
                if entry_side == CardinalEntry::North && wing.y0 + fh + 3 <= wing.y1 {
                    let foyer = Room {
                        x0: hx0,
                        y0: wing.y0,
                        x1: hx1,
                        y1: wing.y0 + fh,
                    };
                    if stair.is_none_or(|st| stair_foot(st).1 >= foyer.y1) {
                        hall_y0 = foyer.y1;
                        bays.push(RoomBay {
                            room: foyer,
                            wing_id,
                            fixed_role: Some(Role::Foyer),
                            is_entry_center: true,
                            is_compact: false,
                            min_depth: foyer.w().min(foyer.h()),
                            exterior_touches_entry_or_side: true,
                            on_patio_or_corner: false,
                        });
                    }
                } else if entry_side == CardinalEntry::South && wing.y1 - fh - 3 >= wing.y0 {
                    let foyer = Room {
                        x0: hx0,
                        y0: wing.y1 - fh,
                        x1: hx1,
                        y1: wing.y1,
                    };
                    if stair.is_none_or(|st| stair_foot(st).3 <= foyer.y0) {
                        hall_y1 = foyer.y0;
                        bays.push(RoomBay {
                            room: foyer,
                            wing_id,
                            fixed_role: Some(Role::Foyer),
                            is_entry_center: true,
                            is_compact: false,
                            min_depth: foyer.w().min(foyer.h()),
                            exterior_touches_entry_or_side: true,
                            on_patio_or_corner: false,
                        });
                    }
                }
            }

            let hall_room = Room {
                x0: hx0,
                y0: hall_y0,
                x1: hx1,
                y1: hall_y1,
            };
            bays.push(RoomBay {
                room: hall_room,
                wing_id,
                fixed_role: Some(Role::Hall),
                is_entry_center: is_main
                    && floor_idx == 0
                    && !use_foyer
                    && matches!(entry_side, CardinalEntry::North | CardinalEntry::South),
                is_compact: false,
                min_depth: hall_room.w(),
                exterior_touches_entry_or_side: false,
                on_patio_or_corner: false,
            });

            for (is_west, sx0, sx1) in [(true, wing.x0, hx0), (false, hx1, wing.x1)] {
                let is_entry_strip = is_main
                    && floor_idx == 0
                    && ((is_west && entry_side == CardinalEntry::West)
                        || (!is_west && entry_side == CardinalEntry::East));
                let mut cur = wing.y0;
                if is_entry_strip {
                    let my = wing.y0 + wing.h() / 2;
                    let target_h = if use_foyer {
                        if wing.h() >= 16 { 4 } else { 3 }
                    } else {
                        (wing.h() * 48 / 100).clamp(4, 7)
                    };
                    let mut ey0 = (my - target_h / 2).clamp(wing.y0, wing.y1 - target_h);
                    let mut ey1 = ey0 + target_h;
                    if ey0 - wing.y0 < 3 {
                        ey0 = wing.y0;
                    }
                    if wing.y1 - ey1 < 3 {
                        ey1 = wing.y1;
                    }
                    if ey0 > cur {
                        push_strip_bays(
                            &mut bays,
                            false,
                            sx0,
                            sx1,
                            cur,
                            ey0,
                            wing_id,
                            wing,
                            patio_box,
                            entry_side,
                            compact_budget,
                            rng,
                        );
                    }
                    let entry_bay = Room {
                        x0: sx0,
                        y0: ey0,
                        x1: sx1,
                        y1: ey1,
                    };
                    bays.push(RoomBay {
                        room: entry_bay,
                        wing_id,
                        fixed_role: if use_foyer { Some(Role::Foyer) } else { None },
                        is_entry_center: true,
                        is_compact: false,
                        min_depth: entry_bay.w().min(entry_bay.h()),
                        exterior_touches_entry_or_side: true,
                        on_patio_or_corner: false,
                    });
                    cur = ey1;
                }
                // Check seam connector on North/South seam touching this strip if L-shaped.
                if let Some(o) = seam_with_other
                    && (o.y1 == wing.y0 || o.y0 == wing.y1)
                    && cur == wing.y0
                    && wing.y1 - cur >= 6
                {
                    let hconn = 2;
                    let conn_r = if o.y1 == wing.y0 {
                        cur += hconn;
                        Room {
                            x0: sx0,
                            y0: wing.y0,
                            x1: sx1,
                            y1: cur,
                        }
                    } else {
                        Room {
                            x0: sx0,
                            y0: wing.y0,
                            x1: sx1,
                            y1: cur,
                        }
                    };
                    if o.y1 == wing.y0 && conn_r.h() > 0 {
                        bays.push(RoomBay {
                            room: conn_r,
                            wing_id,
                            fixed_role: Some(Role::Hall),
                            is_entry_center: false,
                            is_compact: false,
                            min_depth: 2,
                            exterior_touches_entry_or_side: false,
                            on_patio_or_corner: false,
                        });
                    }
                }
                if wing.y1 > cur {
                    push_strip_bays(
                        &mut bays,
                        false,
                        sx0,
                        sx1,
                        cur,
                        wing.y1,
                        wing_id,
                        wing,
                        patio_box,
                        entry_side,
                        compact_budget,
                        rng,
                    );
                }
            }
        }
        SpineMode::Rowhouse => {
            let parlor_d = if wing.h() >= 9 { 4 } else { 3 };
            match entry_side {
                CardinalEntry::North => {
                    let split_y = wing.y0 + parlor_d;
                    let front = Room {
                        x0: wing.x0,
                        y0: wing.y0,
                        x1: wing.x1,
                        y1: split_y,
                    };
                    bays.push(RoomBay {
                        room: front,
                        wing_id,
                        fixed_role: None,
                        is_entry_center: true,
                        is_compact: false,
                        min_depth: parlor_d,
                        exterior_touches_entry_or_side: true,
                        on_patio_or_corner: false,
                    });
                    let hall = Room {
                        x0: wing.x0,
                        y0: split_y,
                        x1: wing.x0 + 2,
                        y1: wing.y1,
                    };
                    bays.push(RoomBay {
                        room: hall,
                        wing_id,
                        fixed_role: Some(Role::Hall),
                        is_entry_center: false,
                        is_compact: false,
                        min_depth: 2,
                        exterior_touches_entry_or_side: false,
                        on_patio_or_corner: false,
                    });
                    push_strip_bays(
                        &mut bays,
                        false,
                        wing.x0 + 2,
                        wing.x1,
                        split_y,
                        wing.y1,
                        wing_id,
                        wing,
                        patio_box,
                        entry_side,
                        compact_budget,
                        rng,
                    );
                }
                CardinalEntry::South => {
                    let split_y = wing.y1 - parlor_d;
                    let front = Room {
                        x0: wing.x0,
                        y0: split_y,
                        x1: wing.x1,
                        y1: wing.y1,
                    };
                    bays.push(RoomBay {
                        room: front,
                        wing_id,
                        fixed_role: None,
                        is_entry_center: true,
                        is_compact: false,
                        min_depth: parlor_d,
                        exterior_touches_entry_or_side: true,
                        on_patio_or_corner: false,
                    });
                    let hall = Room {
                        x0: wing.x0,
                        y0: wing.y0,
                        x1: wing.x0 + 2,
                        y1: split_y,
                    };
                    bays.push(RoomBay {
                        room: hall,
                        wing_id,
                        fixed_role: Some(Role::Hall),
                        is_entry_center: false,
                        is_compact: false,
                        min_depth: 2,
                        exterior_touches_entry_or_side: false,
                        on_patio_or_corner: false,
                    });
                    push_strip_bays(
                        &mut bays,
                        false,
                        wing.x0 + 2,
                        wing.x1,
                        wing.y0,
                        split_y,
                        wing_id,
                        wing,
                        patio_box,
                        entry_side,
                        compact_budget,
                        rng,
                    );
                }
                CardinalEntry::West | CardinalEntry::East => {
                    let (hx0, hx1, rx0, rx1) = if entry_side == CardinalEntry::West {
                        (wing.x0, wing.x0 + 2, wing.x0 + 2, wing.x1)
                    } else {
                        (wing.x1 - 2, wing.x1, wing.x0, wing.x1 - 2)
                    };
                    let (ry0, hall_is_entry) = if wing.h() >= 7 {
                        let front = Room {
                            x0: wing.x0,
                            y0: wing.y0,
                            x1: wing.x1,
                            y1: wing.y0 + 3,
                        };
                        bays.push(RoomBay {
                            room: front,
                            wing_id,
                            fixed_role: None,
                            is_entry_center: is_main && floor_idx == 0,
                            is_compact: false,
                            min_depth: 3,
                            exterior_touches_entry_or_side: true,
                            on_patio_or_corner: false,
                        });
                        (wing.y0 + 3, false)
                    } else {
                        (wing.y0, is_main && floor_idx == 0)
                    };
                    let hall = Room {
                        x0: hx0,
                        y0: ry0,
                        x1: hx1,
                        y1: wing.y1,
                    };
                    bays.push(RoomBay {
                        room: hall,
                        wing_id,
                        fixed_role: Some(Role::Hall),
                        is_entry_center: hall_is_entry,
                        is_compact: false,
                        min_depth: 2,
                        exterior_touches_entry_or_side: false,
                        on_patio_or_corner: false,
                    });
                    push_strip_bays(
                        &mut bays,
                        false,
                        rx0,
                        rx1,
                        ry0,
                        wing.y1,
                        wing_id,
                        wing,
                        patio_box,
                        entry_side,
                        compact_budget,
                        rng,
                    );
                }
            }
        }
        SpineMode::Compact => {
            let r = wing.as_room();
            // Always keep stair_foot(st) completely inside one bay without splitting through it.
            let min_sy = stair.map_or(r.y0 + 3, |st| stair_foot(st).3.max(r.y0 + 3));
            let min_sx = stair.map_or(r.x0 + 3, |st| stair_foot(st).2.max(r.x0 + 3));
            if r.w() >= 6 && r.h() >= 5 && min_sx <= r.x1 - 2 {
                let left = Room { x1: min_sx, ..r };
                let right = Room { x0: min_sx, ..r };
                let left_is_entry = entry_side != CardinalEntry::East;
                bays.push(RoomBay {
                    room: left,
                    wing_id,
                    fixed_role: None,
                    is_entry_center: left_is_entry,
                    is_compact: false,
                    min_depth: left.w().min(left.h()),
                    exterior_touches_entry_or_side: true,
                    on_patio_or_corner: false,
                });
                if right.h() >= 5 {
                    let my = right.y0 + right.h() / 2;
                    bays.push(RoomBay {
                        room: Room { y1: my, ..right },
                        wing_id,
                        fixed_role: None,
                        is_entry_center: !left_is_entry,
                        is_compact: (right.w() * (my - right.y0)) <= 10,
                        min_depth: right.w().min(my - right.y0),
                        exterior_touches_entry_or_side: !left_is_entry,
                        on_patio_or_corner: false,
                    });
                    bays.push(RoomBay {
                        room: Room { y0: my, ..right },
                        wing_id,
                        fixed_role: None,
                        is_entry_center: false,
                        is_compact: (right.w() * (right.y1 - my)) <= 10,
                        min_depth: right.w().min(right.y1 - my),
                        exterior_touches_entry_or_side: false,
                        on_patio_or_corner: false,
                    });
                } else {
                    bays.push(RoomBay {
                        room: right,
                        wing_id,
                        fixed_role: None,
                        is_entry_center: !left_is_entry,
                        is_compact: right.area() <= 10,
                        min_depth: right.w().min(right.h()),
                        exterior_touches_entry_or_side: !left_is_entry,
                        on_patio_or_corner: false,
                    });
                }
            } else if r.h() >= 5 && min_sy <= r.y1 - 2 {
                let top = Room { y1: min_sy, ..r };
                let bot = Room { y0: min_sy, ..r };
                let top_is_entry = entry_side != CardinalEntry::South;
                bays.push(RoomBay {
                    room: top,
                    wing_id,
                    fixed_role: None,
                    is_entry_center: top_is_entry,
                    is_compact: false,
                    min_depth: top.w().min(top.h()),
                    exterior_touches_entry_or_side: true,
                    on_patio_or_corner: false,
                });
                bays.push(RoomBay {
                    room: bot,
                    wing_id,
                    fixed_role: None,
                    is_entry_center: !top_is_entry,
                    is_compact: bot.area() <= 10,
                    min_depth: bot.w().min(bot.h()),
                    exterior_touches_entry_or_side: !top_is_entry,
                    on_patio_or_corner: false,
                });
            } else {
                bays.push(RoomBay {
                    room: r,
                    wing_id,
                    fixed_role: Some(Role::Living),
                    is_entry_center: true,
                    is_compact: false,
                    min_depth: r.w().min(r.h()),
                    exterior_touches_entry_or_side: true,
                    on_patio_or_corner: false,
                });
            }
        }
    }
    bays
}

fn pick_best_bay(
    bays: &[RoomBay],
    assigned: &[Option<Role>],
    mut score_fn: impl FnMut(usize, &RoomBay) -> Option<i32>,
) -> Option<usize> {
    (0..bays.len())
        .filter(|&i| assigned[i].is_none())
        .filter_map(|i| score_fn(i, &bays[i]).map(|s| (s, i)))
        .min_by_key(|&(s, _)| s)
        .map(|(_, i)| i)
}

fn assign_unified_floor_roles(
    bays: &[RoomBay],
    bp: &HomeBlueprint,
    tier: EconomicTier,
    cfg: HomeGenConfig,
    floor_idx: i32,
    rng: &mut Rng,
) -> Vec<(Room, Role, u8)> {
    let n = bays.len();
    let mut assigned: Vec<Option<Role>> = bays.iter().map(|b| b.fixed_role).collect();
    if n == 0 {
        return Vec::new();
    }

    let assign = |idx: usize, role: Role, out: &mut [Option<Role>]| {
        out[idx] = Some(role);
    };

    if floor_idx == -1 {
        let want_laundry = bp.laundry_floor == Some(-1);
        let has_bath = false;
        let mut did_laundry = false;
        let mut did_half_bath = has_bath;
        let mut idx_pool: Vec<usize> = (0..n).filter(|&i| assigned[i].is_none()).collect();
        idx_pool.sort_by_key(|&i| bays[i].room.area());

        if want_laundry
            && let Some(li) = pick_best_bay(bays, &assigned, |_, b| {
                (b.room.area() <= 14).then_some(b.room.area())
            })
        {
            assign(li, Role::Laundry, &mut assigned);
            did_laundry = true;
        }
        if !did_half_bath
            && n >= 4
            && let Some(hi) = pick_best_bay(bays, &assigned, |_, b| {
                (b.room.area() <= 12).then_some(b.room.area())
            })
        {
            assign(hi, Role::HalfBath, &mut assigned);
            did_half_bath = true;
        }
        if !did_laundry && want_laundry {
            // Skip if no compact room fit cleanly.
        }
        let _ = did_half_bath;
        let mut rest: Vec<usize> = (0..n).filter(|&i| assigned[i].is_none()).collect();
        rest.sort_by_key(|&i| -bays[i].room.area());
        let b_palette = [
            Role::BarHall,
            Role::HomeGym,
            Role::Storage,
            Role::ServerCloset,
        ];
        for (cur, idx) in rest.into_iter().enumerate() {
            assign(idx, b_palette[cur % b_palette.len()], &mut assigned);
        }
    } else if floor_idx == 1 {
        // Upper level: MasterBedroom + MasterBath + Bedrooms + max 1 shared Bathroom + Study/Library/Sunroom.
        if let Some(mb) = pick_best_bay(bays, &assigned, |_, b| {
            (b.min_depth >= 3).then_some(-b.room.area())
        })
        .or_else(|| pick_best_bay(bays, &assigned, |_, b| Some(-b.room.area())))
        {
            assign(mb, Role::MasterBedroom, &mut assigned);
        }
        if let Some(mbath) = pick_best_bay(bays, &assigned, |_, b| {
            (b.room.area() <= 16).then_some(if b.is_compact { 0 } else { 20 } + b.room.area())
        }) {
            assign(mbath, Role::MasterBath, &mut assigned);
        }
        let open_after_core = assigned.iter().filter(|r| r.is_none()).count();
        if open_after_core >= 3
            && let Some(b2) = pick_best_bay(bays, &assigned, |_, b| {
                (b.room.area() <= 14).then_some(if b.is_compact { 0 } else { 20 } + b.room.area())
            })
        {
            assign(b2, Role::Bathroom, &mut assigned);
        }
        let mut rest: Vec<usize> = (0..n).filter(|&i| assigned[i].is_none()).collect();
        rest.sort_by_key(|&i| -bays[i].room.area());
        let is_affluent = matches!(
            tier,
            EconomicTier::LuxuryMansion | EconomicTier::UpperMiddleClass
        );
        let mut tail = if is_affluent || rest.len() >= 4 {
            [Role::Library, Role::Office, Role::Sunroom, Role::HomeGym]
        } else {
            [Role::Bedroom, Role::Office, Role::Sunroom, Role::Office]
        };
        let tail_rot = (rng.next() as usize) % tail.len();
        tail.rotate_left(tail_rot);
        let mut closet_used = false;
        let mut bedrooms_assigned = 0usize;
        let mut si = 0usize;
        for idx in rest {
            let b = &bays[idx];
            if (b.is_compact || b.room.area() <= 9) && !closet_used {
                assign(idx, Role::Closet, &mut assigned);
                closet_used = true;
            } else if b.is_compact {
                assign(idx, Role::Office, &mut assigned);
            } else if bedrooms_assigned < 2 {
                assign(idx, Role::Bedroom, &mut assigned);
                bedrooms_assigned += 1;
            } else {
                let role = if si < tail.len() {
                    tail[si]
                } else if (si - tail.len()).is_multiple_of(2) {
                    Role::Living
                } else {
                    Role::Bedroom
                };
                assign(idx, role, &mut assigned);
                si += 1;
            }
        }
    } else {
        // Ground floor (1F): Logical whole-home program.
        let open_count = assigned.iter().filter(|r| r.is_none()).count();

        // 1. When `!use_foyer`, open front door directly into Living / Dining / Kitchen.
        let mut have_living = assigned.contains(&Some(Role::Living));
        let mut have_kitchen = false;
        let mut have_dining = false;
        if !bp.use_foyer
            && let Some(ei) = bays
                .iter()
                .position(|b| b.is_entry_center && assigned[bays_index_match(bays, b)].is_none())
        {
            let entry_role = if !cfg.allow_kitchen || open_count <= 2 {
                Role::Living
            } else {
                match rng.range(0, 10) {
                    0..=5 => Role::Living,
                    6..=7 => Role::Dining,
                    _ => Role::Kitchen,
                }
            };
            assign(ei, entry_role, &mut assigned);
            match entry_role {
                Role::Living => have_living = true,
                Role::Dining => have_dining = true,
                Role::Kitchen => have_kitchen = true,
                _ => {}
            }
        }

        // 2. Ensure Living room (largest >= 3m deep bay).
        if !have_living
            && let Some(li) = pick_best_bay(bays, &assigned, |_, b| {
                let pen = if b.min_depth < 3 { 1000 } else { 0 };
                Some(pen - b.room.area())
            })
        {
            assign(li, Role::Living, &mut assigned);
        }

        // 3. Full Bathroom (required on primary 1F >= 2 rooms).
        let total_bays_now = open_count
            + assigned
                .iter()
                .filter(|r| matches!(r, Some(Role::Foyer)))
                .count();
        let mut bath_count = 0usize;
        if (cfg.allow_kitchen || total_bays_now >= 3)
            && bays.len() >= 2
            && let Some(bi) = pick_best_bay(bays, &assigned, |_, b| {
                let pen = if b.room.area() <= 14 {
                    if b.is_compact { 0 } else { 30 }
                } else {
                    200
                };
                Some(pen + b.room.area())
            })
        {
            assign(bi, Role::Bathroom, &mut assigned);
            bath_count += 1;
        }

        // 4. Primary Kitchen (max 1 per home, omitted in Annex wings).
        if cfg.allow_kitchen
            && !have_kitchen
            && let Some(ki) = pick_best_bay(bays, &assigned, |_, b| {
                let depth_pen = if b.min_depth >= 3 { 0 } else { 500 };
                let size_pen = if b.room.area() >= 9 { 0 } else { 200 };
                Some(depth_pen + size_pen + (b.room.area() - 16).abs())
            })
        {
            assign(ki, Role::Kitchen, &mut assigned);
        }

        // 5. Dining room (when >= 3 open bays originally).
        if !have_dining
            && open_count >= 4
            && let Some(di) = pick_best_bay(bays, &assigned, |_, b| {
                let depth_pen = if b.min_depth >= 3 { 0 } else { 500 };
                Some(depth_pen + (b.room.area() - 15).abs())
            })
        {
            assign(di, Role::Dining, &mut assigned);
        }

        // 6. Optional Single Laundry Room on 1F (only when blueprint assigned Laundry to 1F).
        if bp.laundry_floor == Some(0)
            && open_count >= 5
            && let Some(li) = pick_best_bay(bays, &assigned, |_, b| {
                if b.room.area() > 14 {
                    return None;
                }
                Some(if b.is_compact { 0 } else { 30 } + b.room.area())
            })
        {
            assign(li, Role::Laundry, &mut assigned);
        }

        // 7. At most 1 HalfBath (powder room) ONLY on large ground floors (>= 7 non-hall bays).
        if cfg.allow_kitchen
            && open_count >= 7
            && bath_count < 2
            && let Some(hi) = pick_best_bay(bays, &assigned, |_, b| {
                if b.room.area() > 12 {
                    return None;
                }
                Some(if b.is_compact { 0 } else { 25 } + b.room.area())
            })
        {
            assign(hi, Role::HalfBath, &mut assigned);
        }

        // 8. Garage (exterior entry/corner bay >= 4x4 on large homes).
        let is_affluent = matches!(
            tier,
            EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion
        );
        if cfg.allow_garage
            && (open_count >= 7 || (is_affluent && open_count >= 6))
            && let Some(gi) = pick_best_bay(bays, &assigned, |_, b| {
                (b.exterior_touches_entry_or_side && b.room.w() >= 4 && b.room.h() >= 4)
                    .then_some(-b.room.area())
            })
        {
            assign(gi, Role::Garage, &mut assigned);
        }

        // 9. Assign remaining bays without repeating bathrooms/kitchens/laundries.
        let mut did_sunroom = false;
        let mut did_library = false;
        let mut did_office = false;
        let mut did_gym = false;
        let mut did_closet = false;
        let mut did_master = false;
        let mut bedroom_count = 0usize;

        let mut spec_order = [Role::Sunroom, Role::Library, Role::Office, Role::HomeGym];
        let spec_rot = (rng.next() as usize) % spec_order.len();
        spec_order.rotate_left(spec_rot);
        let mut rest: Vec<usize> = (0..n).filter(|&i| assigned[i].is_none()).collect();
        rest.sort_by_key(|&i| -bays[i].room.area());
        for idx in rest {
            let b = &bays[idx];
            if (b.is_compact || b.room.area() <= 9) && !did_closet && open_count >= 4 {
                assign(idx, Role::Closet, &mut assigned);
                did_closet = true;
                continue;
            }
            if b.min_depth <= 2 && b.room.area() <= 10 && !did_office {
                assign(idx, Role::Office, &mut assigned);
                did_office = true;
                continue;
            }
            let next_role = if !bp.has_upper && !did_master {
                did_master = true;
                bedroom_count += 1;
                Role::MasterBedroom
            } else if (!bp.has_upper && bedroom_count < 2)
                || (cfg.allow_kitchen && bedroom_count == 0 && b.min_depth >= 3)
            {
                bedroom_count += 1;
                Role::Bedroom
            } else {
                let chosen = if !did_sunroom && b.on_patio_or_corner && b.min_depth >= 3 {
                    Some(Role::Sunroom)
                } else {
                    spec_order.into_iter().find(|&cand| match cand {
                        Role::Sunroom => !did_sunroom && b.min_depth >= 3,
                        Role::Library => !did_library && (is_affluent || open_count >= 6),
                        Role::HomeGym => {
                            !did_gym
                                && (is_affluent && open_count >= 6 || open_count >= 7)
                                && b.min_depth >= 3
                        }
                        Role::Office => !did_office,
                        _ => false,
                    })
                };
                if let Some(spec) = chosen {
                    match spec {
                        Role::Sunroom => did_sunroom = true,
                        Role::Library => did_library = true,
                        Role::Office => did_office = true,
                        Role::HomeGym => did_gym = true,
                        _ => {}
                    }
                    spec
                } else if bedroom_count < 2 {
                    bedroom_count += 1;
                    Role::Bedroom
                } else if !did_closet {
                    did_closet = true;
                    Role::Closet
                } else {
                    Role::Living
                }
            };
            assign(idx, next_role, &mut assigned);
        }
    }

    let mut result: Vec<(Room, Role, u8)> = bays
        .iter()
        .enumerate()
        .map(|(i, b)| (b.room, assigned[i].unwrap_or(Role::Bedroom), b.wing_id))
        .collect();
    enforce_service_room_caps(&mut result, bp.stair, floor_idx, true);
    result
}

fn bays_index_match(bays: &[RoomBay], target: &RoomBay) -> usize {
    bays.iter()
        .position(|b| std::ptr::eq(b, target))
        .unwrap_or(0)
}

const fn max_service_room_area(role: Role) -> Option<i32> {
    match role {
        Role::MasterBath => Some(16),
        Role::Bathroom => Some(14),
        Role::HalfBath => Some(9),
        Role::Closet | Role::Laundry => Some(12),
        _ => None,
    }
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

/// Enforces realistic `<= 9..16 m²` caps on `Bathroom`, `MasterBath`, `HalfBath`, `Closet`, & `Laundry`.
fn enforce_service_room_caps(
    tagged: &mut Vec<(Room, Role, u8)>,
    stair: Option<StairAnchor>,
    floor_idx: i32,
    _is_main: bool,
) {
    if floor_idx < 0 {
        return;
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
        let has_closet = tagged.iter().any(|(_, cr, _)| *cr == Role::Closet);
        for split_v in try_split_v {
            if split_v && rm.w() >= 4 {
                let want = (cap / rm.h().max(1)).clamp(2, max_bath_dim);
                if let Some(sx) = pick_v_cut(rm, rm.x0 + want, 2, 2, stair) {
                    let a = Room { x1: sx, ..rm };
                    let b = Room { x0: sx, ..rm };
                    let (bath_rm, rest_rm) = if a.area() <= b.area() { (a, b) } else { (b, a) };
                    let rest_role = if rest_rm.area() >= 10 {
                        Role::Office
                    } else if !has_closet && rest_rm.area() >= 6 {
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
                        Role::Office
                    } else if !has_closet && rest_rm.area() >= 6 {
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
            if let Some(swap_i) = tagged.iter().position(|(other, orole, _)| {
                other.area() <= cap
                    && !matches!(
                        orole,
                        Role::Hall
                            | Role::Foyer
                            | Role::Living
                            | Role::Kitchen
                            | Role::Bathroom
                            | Role::HalfBath
                            | Role::MasterBath
                            | Role::Closet
                            | Role::Laundry
                    )
            }) {
                let small_role = tagged[swap_i].1;
                tagged[swap_i].1 = role;
                tagged[idx].1 = small_role;
            } else {
                tagged[idx].1 = Role::Office;
            }
        }
    }
}

const fn is_circulation_role(r: Role) -> bool {
    matches!(r, Role::Hall | Role::Foyer)
}

const fn is_social_role(r: Role) -> bool {
    matches!(
        r,
        Role::Hall
            | Role::Foyer
            | Role::Living
            | Role::Dining
            | Role::Kitchen
            | Role::Sunroom
            | Role::Library
    )
}

const fn pair_door_tier(a: Role, b: Role) -> u8 {
    let ca = is_circulation_role(a);
    let cb = is_circulation_role(b);
    if ca && cb {
        0
    } else if (ca && !matches!(b, Role::MasterBath)) || (cb && !matches!(a, Role::MasterBath)) {
        1
    } else if (matches!(a, Role::MasterBedroom | Role::Bedroom)
        && matches!(b, Role::MasterBath | Role::Closet))
        || (matches!(b, Role::MasterBedroom | Role::Bedroom)
            && matches!(a, Role::MasterBath | Role::Closet))
        || (is_social_role(a) && is_social_role(b))
    {
        2
    } else if ca || cb {
        3
    } else {
        99
    }
}

const fn is_open_passage_pair(a: Role, b: Role, has_foyer: bool) -> bool {
    matches!(
        (a, b),
        (Role::Hall, Role::Hall)
            | (Role::Hall, Role::Foyer)
            | (Role::Foyer, Role::Hall)
            | (Role::Kitchen, Role::Dining)
            | (Role::Dining, Role::Kitchen)
            | (Role::Dining, Role::Living)
            | (Role::Living, Role::Dining)
            | (Role::Foyer, Role::Living)
            | (Role::Living, Role::Foyer)
    ) || (!has_foyer
        && matches!(
            (a, b),
            (Role::Living, Role::Hall)
                | (Role::Hall, Role::Living)
                | (Role::Dining, Role::Hall)
                | (Role::Hall, Role::Dining)
        ))
}

fn connect_rooms_with_doors(shell: &mut Shell, tagged: &[(Room, Role, u8)], bp: &HomeBlueprint) {
    let mut borders: Vec<SharedBorder> = Vec::new();
    for i in 0..tagged.len() {
        for j in (i + 1)..tagged.len() {
            let ((ra, role_a, wa), (rb, role_b, wb)) = (tagged[i], tagged[j]);
            let same_wing = wa == wb;
            let weight = i32::from(pair_door_tier(role_a, role_b));
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
            pair_door_tier(tagged[b.a].1, tagged[b.b].1),
            u8::from(!b.same_wing),
            b.weight,
            -(b.end - b.start),
        )
    });
    let mut parent: Vec<usize> = (0..tagged.len()).collect();
    let mut carved = vec![false; borders.len()];
    // Carve Hall/Foyer spine and circulation-to-room connections before cross-wing fallbacks.
    for tier_limit in [2u8, 99u8] {
        for (bi, bd) in borders.iter().enumerate() {
            if pair_door_tier(tagged[bd.a].1, tagged[bd.b].1) > tier_limit {
                continue;
            }
            let (ra, rb) = (dsu_find(&mut parent, bd.a), dsu_find(&mut parent, bd.b));
            if ra != rb {
                parent[rb] = ra;
                carved[bi] = true;
            }
        }
    }

    // Ensure every room touching Hall/Foyer connects directly into the hallway rather than piggybacking.
    for i in 0..tagged.len() {
        if is_circulation_role(tagged[i].1) {
            continue;
        }
        let has_circ = borders.iter().enumerate().any(|(bi, bd)| {
            carved[bi]
                && ((bd.a == i && is_circulation_role(tagged[bd.b].1))
                    || (bd.b == i && is_circulation_role(tagged[bd.a].1)))
        });
        if !has_circ
            && let Some(bi) = borders.iter().position(|bd| {
                (bd.a == i && is_circulation_role(tagged[bd.b].1))
                    || (bd.b == i && is_circulation_role(tagged[bd.a].1))
            })
        {
            carved[bi] = true;
        }
    }

    let is_villa = bp.plan == HomePlanKind::CourtyardVilla;
    for (bi, bd) in borders.iter().enumerate() {
        if carved[bi] || bd.end - bd.start < 2 {
            continue;
        }
        let (ra, rb) = (tagged[bd.a].1, tagged[bd.b].1);
        if is_social_role(ra)
            && is_social_role(rb)
            && (is_open_passage_pair(ra, rb, bp.use_foyer) || is_villa || shell.rng.chance(1, 2))
        {
            carved[bi] = true;
        }
    }

    for (bi, bd) in borders.iter().enumerate() {
        if !carved[bi] {
            continue;
        }
        let (ra_role, rb_role) = (tagged[bd.a].1, tagged[bd.b].1);
        if is_open_passage_pair(ra_role, rb_role, bp.use_foyer) && bd.end - bd.start >= 2 {
            // Retain only if removing interior wall doesn't cut an external edge.
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
                | (Role::Living, Role::Library)
                | (Role::Library, Role::Living)
                | (Role::Kitchen, Role::Hall)
                | (Role::Hall, Role::Kitchen)
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

fn apply_exterior_specialty_openings(
    shell: &mut Shell,
    tagged: &[(Room, Role, u8)],
    bp: &HomeBlueprint,
    floor_idx: i32,
) {
    if floor_idx < 0 {
        return;
    }
    let wings: Vec<LotBox> = std::iter::once(bp.main_wing).chain(bp.arm_wing).collect();
    for &(room, role, wid) in tagged {
        let Some(&wing) = wings.get(usize::from(wid)) else {
            continue;
        };
        let other = if wid == 0 {
            bp.arm_wing
        } else {
            Some(bp.main_wing)
        };
        if role == Role::Sunroom {
            // Upgrade Sunroom exterior wall spans (excluding corner junctions & existing doors) to Glass windows.
            if room.y0 == wing.y0 {
                for x in (room.x0 + 1).max(wing.x0 + 1)..(room.x1 - 1).min(wing.x1 - 1) {
                    if !other.is_some_and(|o| is_seam_edge(x, wing.y0, EdgeSide::North, wing, o))
                        && !shell.edges.iter().any(|e| {
                            e.x == x
                                && e.y == wing.y0
                                && e.side == EdgeSide::North
                                && matches!(
                                    e.kind,
                                    WallEdgeKind::DoorOpen | WallEdgeKind::DoorClosed
                                )
                        })
                    {
                        shell.put(
                            x,
                            wing.y0,
                            EdgeSide::North,
                            WallEdgeKind::Window,
                            MaterialClass::Glass,
                        );
                    }
                }
            }
            if room.y1 == wing.y1 {
                for x in (room.x0 + 1).max(wing.x0 + 1)..(room.x1 - 1).min(wing.x1 - 1) {
                    if !other.is_some_and(|o| is_seam_edge(x, wing.y1, EdgeSide::North, wing, o))
                        && !shell.edges.iter().any(|e| {
                            e.x == x
                                && e.y == wing.y1
                                && e.side == EdgeSide::North
                                && matches!(
                                    e.kind,
                                    WallEdgeKind::DoorOpen | WallEdgeKind::DoorClosed
                                )
                        })
                    {
                        shell.put(
                            x,
                            wing.y1,
                            EdgeSide::North,
                            WallEdgeKind::Window,
                            MaterialClass::Glass,
                        );
                    }
                }
            }
            if room.x0 == wing.x0 {
                for y in (room.y0 + 1).max(wing.y0 + 1)..(room.y1 - 1).min(wing.y1 - 1) {
                    if !other.is_some_and(|o| is_seam_edge(wing.x0, y, EdgeSide::West, wing, o))
                        && !shell.edges.iter().any(|e| {
                            e.x == wing.x0
                                && e.y == y
                                && e.side == EdgeSide::West
                                && matches!(
                                    e.kind,
                                    WallEdgeKind::DoorOpen | WallEdgeKind::DoorClosed
                                )
                        })
                    {
                        shell.put(
                            wing.x0,
                            y,
                            EdgeSide::West,
                            WallEdgeKind::Window,
                            MaterialClass::Glass,
                        );
                    }
                }
            }
            if room.x1 == wing.x1 {
                for y in (room.y0 + 1).max(wing.y0 + 1)..(room.y1 - 1).min(wing.y1 - 1) {
                    if !other.is_some_and(|o| is_seam_edge(wing.x1, y, EdgeSide::West, wing, o))
                        && !shell.edges.iter().any(|e| {
                            e.x == wing.x1
                                && e.y == y
                                && e.side == EdgeSide::West
                                && matches!(
                                    e.kind,
                                    WallEdgeKind::DoorOpen | WallEdgeKind::DoorClosed
                                )
                        })
                    {
                        shell.put(
                            wing.x1,
                            y,
                            EdgeSide::West,
                            WallEdgeKind::Window,
                            MaterialClass::Glass,
                        );
                    }
                }
            }
        } else if floor_idx == 0 && role == Role::Garage {
            // Give Garage a 2m SheetSteel vehicle bay door on an unshared exterior wall.
            let mid_x = (room.x0 + room.x1) / 2;
            let mid_y = (room.y0 + room.y1) / 2;
            if room.y1 == wing.y1
                && room.w() >= 3
                && !other.is_some_and(|o| is_seam_edge(mid_x, wing.y1, EdgeSide::North, wing, o))
            {
                shell.put(
                    mid_x - 1,
                    wing.y1,
                    EdgeSide::North,
                    WallEdgeKind::DoorClosed,
                    MaterialClass::SheetSteel,
                );
                shell.put(
                    mid_x,
                    wing.y1,
                    EdgeSide::North,
                    WallEdgeKind::DoorClosed,
                    MaterialClass::SheetSteel,
                );
            } else if room.y0 == wing.y0
                && room.w() >= 3
                && !other.is_some_and(|o| is_seam_edge(mid_x, wing.y0, EdgeSide::North, wing, o))
            {
                shell.put(
                    mid_x - 1,
                    wing.y0,
                    EdgeSide::North,
                    WallEdgeKind::DoorClosed,
                    MaterialClass::SheetSteel,
                );
                shell.put(
                    mid_x,
                    wing.y0,
                    EdgeSide::North,
                    WallEdgeKind::DoorClosed,
                    MaterialClass::SheetSteel,
                );
            } else if room.x0 == wing.x0
                && room.h() >= 3
                && !other.is_some_and(|o| is_seam_edge(wing.x0, mid_y, EdgeSide::West, wing, o))
            {
                shell.put(
                    wing.x0,
                    mid_y - 1,
                    EdgeSide::West,
                    WallEdgeKind::DoorClosed,
                    MaterialClass::SheetSteel,
                );
                shell.put(
                    wing.x0,
                    mid_y,
                    EdgeSide::West,
                    WallEdgeKind::DoorClosed,
                    MaterialClass::SheetSteel,
                );
            } else if room.x1 == wing.x1
                && room.h() >= 3
                && !other.is_some_and(|o| is_seam_edge(wing.x1, mid_y, EdgeSide::West, wing, o))
            {
                shell.put(
                    wing.x1,
                    mid_y - 1,
                    EdgeSide::West,
                    WallEdgeKind::DoorClosed,
                    MaterialClass::SheetSteel,
                );
                shell.put(
                    wing.x1,
                    mid_y,
                    EdgeSide::West,
                    WallEdgeKind::DoorClosed,
                    MaterialClass::SheetSteel,
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
    cfg: HomeGenConfig,
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

    let mut compact_budget = if floor_idx < 0 {
        if bp.laundry_floor == Some(-1) { 2 } else { 1 }
    } else if floor_idx == 1 {
        2
    } else {
        let allow_1f_laundry = bp.laundry_floor == Some(0);
        if bp.main_wing.area() >= 84 {
            2 + i32::from(allow_1f_laundry)
        } else {
            1 + i32::from(allow_1f_laundry)
        }
    };

    let mut bays = partition_wing_bays(
        bp.main_wing,
        0,
        other_wing,
        bp.stair,
        bp.patio_box,
        entry_side,
        bp.use_foyer,
        floor_idx,
        &mut compact_budget,
        &mut shell.rng,
    );
    if let Some(arm) = other_wing {
        bays.extend(partition_wing_bays(
            arm,
            1,
            Some(bp.main_wing),
            None,
            bp.patio_box,
            entry_side,
            false,
            floor_idx,
            &mut compact_budget,
            &mut shell.rng,
        ));
    }

    let tagged_rooms = assign_unified_floor_roles(&bays, bp, tier, cfg, floor_idx, &mut shell.rng);
    connect_rooms_with_doors(&mut shell, &tagged_rooms, bp);
    apply_exterior_specialty_openings(&mut shell, &tagged_rooms, bp, floor_idx);
    if floor_idx == 0 {
        add_ground_entry_and_patio_doors(&mut shell, bp, &tagged_rooms, entry_side, rear_door);
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

fn pick_exterior_cell_for_roles(
    tagged: &[(Room, Role, u8)],
    wing: LotBox,
    side: CardinalEntry,
    ideal_coord: i32,
    patio_span: Option<(i32, i32)>,
    prefer_direct_social: bool,
) -> i32 {
    let role_prio = |r: Role| -> i32 {
        if prefer_direct_social {
            match r {
                Role::Living | Role::Dining | Role::Kitchen => 0,
                Role::Foyer | Role::Hall => 1,
                Role::Sunroom | Role::Library | Role::Office | Role::HomeGym => 2,
                Role::MasterBedroom | Role::Bedroom => 3,
                Role::Garage | Role::Closet | Role::Laundry => 4,
                Role::Bathroom | Role::HalfBath | Role::MasterBath => 5,
                _ => 3,
            }
        } else {
            match r {
                Role::Foyer | Role::Hall => 0,
                Role::Living | Role::Dining | Role::Sunroom => 1,
                Role::Kitchen | Role::Library | Role::Office | Role::HomeGym => 2,
                Role::MasterBedroom | Role::Bedroom => 3,
                Role::Garage | Role::Closet | Role::Laundry => 4,
                Role::Bathroom | Role::HalfBath | Role::MasterBath => 5,
                _ => 3,
            }
        }
    };
    let (lo, hi) = patio_span.unwrap_or(match side {
        CardinalEntry::North | CardinalEntry::South => (wing.x0, wing.x1),
        CardinalEntry::West | CardinalEntry::East => (wing.y0, wing.y1),
    });
    let mut best: Option<(i32, i32, i32)> = None;
    for &(room, role, wid) in tagged {
        if wid != 0 {
            continue;
        }
        let on_edge = match side {
            CardinalEntry::North => room.y0 == wing.y0,
            CardinalEntry::South => room.y1 == wing.y1,
            CardinalEntry::West => room.x0 == wing.x0,
            CardinalEntry::East => room.x1 == wing.x1,
        };
        if !on_edge {
            continue;
        }
        let (r0, r1) = match side {
            CardinalEntry::North | CardinalEntry::South => (room.x0.max(lo), room.x1.min(hi)),
            CardinalEntry::West | CardinalEntry::East => (room.y0.max(lo), room.y1.min(hi)),
        };
        for c in r0..r1 {
            let score = (role_prio(role), (c - ideal_coord).abs());
            if best.is_none_or(|b| (score.0, score.1) < (b.0, b.1)) {
                best = Some((score.0, score.1, c));
            }
        }
    }
    best.map_or(ideal_coord.clamp(lo, (hi - 1).max(lo)), |b| b.2)
}

fn add_ground_entry_and_patio_doors(
    shell: &mut Shell,
    bp: &HomeBlueprint,
    tagged: &[(Room, Role, u8)],
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
    let opp = match entry_side {
        CardinalEntry::North => CardinalEntry::South,
        CardinalEntry::South => CardinalEntry::North,
        CardinalEntry::West => CardinalEntry::East,
        CardinalEntry::East => CardinalEntry::West,
    };
    let front_coord = pick_exterior_cell_for_roles(
        tagged,
        w,
        entry_side,
        match entry_side {
            CardinalEntry::North | CardinalEntry::South => mx,
            CardinalEntry::West | CardinalEntry::East => my,
        },
        None,
        !bp.use_foyer,
    );
    match entry_side {
        CardinalEntry::North => shell.put(
            front_coord,
            w.y0,
            EdgeSide::North,
            dk,
            MaterialClass::SolidWood,
        ),
        CardinalEntry::South => shell.put(
            front_coord,
            w.y1,
            EdgeSide::North,
            dk,
            MaterialClass::SolidWood,
        ),
        CardinalEntry::West => shell.put(
            w.x0,
            front_coord,
            EdgeSide::West,
            dk,
            MaterialClass::SolidWood,
        ),
        CardinalEntry::East => shell.put(
            w.x1,
            front_coord,
            EdgeSide::West,
            dk,
            MaterialClass::SolidWood,
        ),
    }

    if rear_door {
        let rear_coord = pick_exterior_cell_for_roles(
            tagged,
            w,
            opp,
            match opp {
                CardinalEntry::North | CardinalEntry::South => mx,
                CardinalEntry::West | CardinalEntry::East => my,
            },
            None,
            false,
        );
        match opp {
            CardinalEntry::North => shell.put(
                rear_coord,
                w.y0,
                EdgeSide::North,
                WallEdgeKind::DoorOpen,
                MaterialClass::SolidWood,
            ),
            CardinalEntry::South => shell.put(
                rear_coord,
                w.y1,
                EdgeSide::North,
                WallEdgeKind::DoorOpen,
                MaterialClass::SolidWood,
            ),
            CardinalEntry::West => shell.put(
                w.x0,
                rear_coord,
                EdgeSide::West,
                WallEdgeKind::DoorOpen,
                MaterialClass::SolidWood,
            ),
            CardinalEntry::East => shell.put(
                w.x1,
                rear_coord,
                EdgeSide::West,
                WallEdgeKind::DoorOpen,
                MaterialClass::SolidWood,
            ),
        }
    }

    if let Some(p) = bp.patio_box {
        if p.y1 == w.y0 && p.x1 > w.x0 && p.x0 < w.x1 {
            let s0 = p.x0.max(w.x0);
            let s1 = p.x1.min(w.x1);
            let dx = pick_exterior_cell_for_roles(
                tagged,
                w,
                CardinalEntry::North,
                (s0 + s1) / 2,
                Some((s0, s1)),
                false,
            );
            shell.put(
                dx,
                w.y0,
                EdgeSide::North,
                WallEdgeKind::DoorOpen,
                MaterialClass::Glass,
            );
        } else if p.y0 == w.y1 && p.x1 > w.x0 && p.x0 < w.x1 {
            let s0 = p.x0.max(w.x0);
            let s1 = p.x1.min(w.x1);
            let dx = pick_exterior_cell_for_roles(
                tagged,
                w,
                CardinalEntry::South,
                (s0 + s1) / 2,
                Some((s0, s1)),
                false,
            );
            shell.put(
                dx,
                w.y1,
                EdgeSide::North,
                WallEdgeKind::DoorOpen,
                MaterialClass::Glass,
            );
        } else if p.x1 == w.x0 && p.y1 > w.y0 && p.y0 < w.y1 {
            let s0 = p.y0.max(w.y0);
            let s1 = p.y1.min(w.y1);
            let dy = pick_exterior_cell_for_roles(
                tagged,
                w,
                CardinalEntry::West,
                (s0 + s1) / 2,
                Some((s0, s1)),
                false,
            );
            shell.put(
                w.x0,
                dy,
                EdgeSide::West,
                WallEdgeKind::DoorOpen,
                MaterialClass::Glass,
            );
        } else if p.x0 == w.x1 && p.y1 > w.y0 && p.y0 < w.y1 {
            let s0 = p.y0.max(w.y0);
            let s1 = p.y1.min(w.y1);
            let dy = pick_exterior_cell_for_roles(
                tagged,
                w,
                CardinalEntry::East,
                (s0 + s1) / 2,
                Some((s0, s1)),
                false,
            );
            shell.put(
                w.x1,
                dy,
                EdgeSide::West,
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
    crate::map_pool_spa_gen::populate_patio_pool_or_spa(
        furn,
        patio,
        tier,
        bp.pool_in_patio,
        bp.hot_tub_in_patio,
        surfaces,
        rng,
    );
}
