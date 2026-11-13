//! # Natural Creek & Roadside Ditch Battlemap Builders (`map_outdoor_natural`)
//!
//! Companion module to [`crate::map_outdoor_gen`] implementing the organic terrain presets:
//! - [`build_woodland_creek`]: Meandering `1–2 m` stream (`CreekWater`) with flanking gravel shoals,
//!   a stone stepping-ford (`StonePath`), winding dirt foot-trails, scattered boulders, diagonal
//!   fallen logs (`FallenLog` at `45°` / `135°`), mulch rings, and woodland tree canopy.
//! - [`build_roadside_ditch`]: East–West gravel country lane (`GravelPath`) with dirt shoulders,
//!   a parallel sunken drainage ditch (`DrainageDitch`), masonry culvert bridge (`StonePath` +
//!   headwalls), an adjacent cottage garden (raised `GardenBed`s, `GardenEdging`, `MulchBed`,
//!   fenced perimeter with `DoorOpen` gate), diagonal fallen logs, and roadside trees.

use satya_schema::{EdgeSide, MaterialClass, PropKind, WallEdgeKind};

use crate::map_outdoor_gen::{
    OutdoorBuilder, Rect, SURF_CREEK, SURF_DIRT, SURF_DITCH, SURF_GRAVEL, SURF_STONE, SURF_TURF,
};

/// Populates a winding creek crossing, gravel bars, dirt trails, boulders, diagonal fallen logs,
/// and woodland canopy (`OutdoorStyle::WoodlandCreek`).
pub(crate) fn build_woodland_creek(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w, b.h);
    let mid_y = h / 2;
    let mid_x = w / 2;

    // 1. Meandering North-to-South woodland creek (with a diagonal bend across the center)
    let cx0 = (mid_x + b.rng.range(-2, 3)).clamp(4, (w - 5).max(4));
    let cx1 = (mid_x + b.rng.range(-2, 3)).clamp(4, (w - 5).max(4));
    let bow = if b.rng.chance(1, 2) { 2.4 } else { -2.4 };
    b.paint_meandering_ribbon((cx0, 0), (cx1, h - 1), SURF_CREEK, bow, w >= 14, false);

    // 2. Flanking gravel shoals along the creek banks + stepping-stone ford near mid_y
    for y in 0..h {
        for x in 1..w - 1 {
            if b.surf_at(x, y) != SURF_TURF {
                continue;
            }
            let near_creek =
                b.surf_at(x - 1, y) == SURF_CREEK || b.surf_at(x + 1, y) == SURF_CREEK;
            if near_creek && b.rng.chance(3, 5) {
                b.set_surf(x, y, SURF_GRAVEL);
            }
        }
    }
    let ford_y = (mid_y + b.rng.range(-1, 2)).clamp(2, (h - 3).max(2));
    for y in (ford_y - 1).max(1)..=(ford_y + 1).min(h - 2) {
        for x in 0..w {
            if b.surf_at(x, y) == SURF_CREEK {
                b.set_surf(x, y, SURF_STONE);
            }
        }
    }

    // 3. Organic West-to-East dirt footpath meeting the stone ford
    let w_y = (mid_y + b.rng.range(-2, 2)).clamp(2, (h - 3).max(2));
    let e_y = (mid_y + b.rng.range(-2, 2)).clamp(2, (h - 3).max(2));
    b.paint_meandering_ribbon((0, w_y), (mid_x - 1, ford_y), SURF_DIRT, 1.3, false, true);
    b.paint_meandering_ribbon((mid_x + 1, ford_y), (w - 1, e_y), SURF_DIRT, -1.2, false, true);

    // Reserve deployment pads and ford clearance before placing blocking logs/trees
    b.reserve_spawns_and_doors();
    for y in (ford_y - 1).max(1)..=(ford_y + 1).min(h - 2) {
        for x in (mid_x - 2).max(1)..=(mid_x + 2).min(w - 2) {
            if b.surf_at(x, y) == SURF_STONE {
                b.reserve(x, y);
            }
        }
    }

    // 4. Diagonal fallen logs (45°/135°), mossy boulders, mulch pockets & forest canopy
    let bounds = Rect { x0: 1, y0: 1, x1: w - 1, y1: h - 1 };
    for _ in 0..3 {
        let lx = b.rng.range(2, (w - 5).max(3));
        let ly = b.rng.range(2, (h - 5).max(3));
        b.paint_mulch_patch(lx + 1, ly + 1, 1);
    }

    let log_target = if w * h >= 240 { 3 } else if w * h >= 130 { 2 } else { 1 };
    let mut logs_placed = 0;
    for attempt in 0..28 {
        if logs_placed >= log_target {
            break;
        }
        let rot = if (attempt + logs_placed) % 2 == 0 { 45 } else { 135 };
        let lx = b.rng.range(1, (w - 3).max(2));
        let ly = b.rng.range(1, (h - 3).max(2));
        // Allow logs on turf/mulch/gravel or spanning across the creek away from the stone ford
        let on_ford = (ly..ly + 3).any(|yy| (lx..lx + 3).any(|xx| b.surf_at(xx, yy) == SURF_STONE));
        if !on_ford && b.try_place(PropKind::FallenLog, (lx, ly), rot, None, bounds).is_some() {
            logs_placed += 1;
        }
    }

    // Scatter shoreline & woodland boulders, trees, shrubs
    b.scatter_turf_prop(PropKind::Boulder, ((w * h) / 60).clamp(2, 6), bounds);
    b.scatter_trees(PropKind::LargeTree, ((w * h) / 72).clamp(1, 6), 5, bounds);
    b.scatter_trees(PropKind::MediumTree, ((w * h) / 40).clamp(2, 10), 3, bounds);
    b.scatter_turf_prop(PropKind::Bush, ((w * h) / 36).clamp(3, 12), bounds);
}

/// Populates a rural gravel lane, parallel drainage ditch, culvert crossing, diagonal fallen logs,
/// and a landscaped cottage garden (`OutdoorStyle::RoadsideDitch`).
pub(crate) fn build_roadside_ditch(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w, b.h);
    let mid_y = h / 2;

    // 1. Gravel country road along the North-ish corridor + soft dirt verges
    let road_y0 = (mid_y - 3).clamp(1, (h - 6).max(1));
    let road_y1 = (road_y0 + 3).min(h - 3);
    b.paint_rect(Rect { x0: 0, y0: road_y0, x1: w, y1: road_y1 }, SURF_GRAVEL);
    if road_y0 > 0 {
        b.paint_rect(Rect { x0: 0, y0: road_y0 - 1, x1: w, y1: road_y0 }, SURF_DIRT);
    }

    // 2. Sunken roadside drainage ditch parallel to the lane + stone culvert bridge near center
    let ditch_y = (road_y1 + 1).min(h - 2);
    let mut x = 0;
    while x < w {
        let dy = (ditch_y + ((x as f32 * 0.45).sin().round() as i32).clamp(-1, 1))
            .clamp(road_y1, h - 2);
        b.set_surf(x, dy, SURF_DITCH);
        x += 1;
    }

    let culv_x0 = (w / 2 - 1).clamp(3, (w - 5).max(3));
    let culv_x1 = (culv_x0 + 3).min(w - 2);
    for cy in road_y1..=(ditch_y + 1).min(h - 1) {
        for cx in culv_x0..culv_x1 {
            b.set_surf(cx, cy, SURF_STONE);
        }
    }
    // Masonry culvert parapet headwalls flanking the stone crossing
    b.put_edge(culv_x0, ditch_y, EdgeSide::West, WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete);
    b.put_edge(culv_x1, ditch_y, EdgeSide::West, WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete);

    // 3. Landscaped cottage garden plot South of the ditch when height allows
    let garden_y0 = ditch_y + 2;
    let garden_x0 = (culv_x0 - 2).max(2);
    let garden_x1 = (w - 2).max(garden_x0 + 3);
    if garden_y0 + 1 < h {
        let gate_x = (culv_x0 + 1).clamp(garden_x0, garden_x1 - 1);
        for gx in garden_x0..garden_x1 {
            let k = if gx == gate_x { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall };
            b.put_edge(gx, garden_y0, EdgeSide::North, k, MaterialClass::SolidWood);
        }
        for gy in garden_y0..h - 1 {
            b.put_edge(garden_x0, gy, EdgeSide::West, WallEdgeKind::Wall, MaterialClass::SolidWood);
        }
        b.paint_meandering_ribbon(
            (gate_x, garden_y0),
            ((garden_x0 + garden_x1) / 2, h - 2),
            SURF_STONE,
            0.8,
            false,
            true,
        );
        // Decorative mulch ring & raised garden bed + edging inside the fenced corner
        let bed_x = (garden_x1 - 3).clamp(garden_x0 + 1, w - 3);
        let bed_y = (garden_y0 + 1).min(h - 2);
        b.paint_mulch_patch(bed_x, bed_y, 1);
    }

    b.reserve_spawns_and_doors();
    // Keep the stone culvert deck clear for crossing
    for cx in culv_x0..culv_x1 {
        b.reserve(cx, ditch_y);
    }

    // 4. Decorative garden beds, edging, diagonal fallen logs, boulders & roadside trees
    let full = Rect { x0: 1, y0: 1, x1: w - 1, y1: h - 1 };
    if garden_y0 + 1 < h {
        let garden_box = Rect { x0: garden_x0, y0: garden_y0, x1: garden_x1, y1: h - 1 };
        let bx = (garden_x0 + 1).min(garden_x1 - 2);
        b.try_place(PropKind::GardenBed, (bx, garden_y0), 0, None, garden_box);
        if b.try_place(PropKind::GardenEdging, (bx, (garden_y0 + 1).min(h - 2)), 0, Some(2), garden_box).is_none() {
            b.try_place(PropKind::GardenEdging, (garden_x0, garden_y0), 0, Some(2), garden_box);
        }
        b.scatter_turf_prop(PropKind::ParkBench, 1, garden_box);
    }

    // Diagonal fallen logs near roadside verge or ditch bank
    let mut logs = 0;
    for attempt in 0..16 {
        if logs >= if w * h >= 200 { 2 } else { 1 } {
            break;
        }
        let rot = if attempt % 2 == 0 { 45 } else { 135 };
        let lx = b.rng.range(1, (w - 3).max(2));
        let ly = b.rng.range(1, (h - 3).max(2));
        if b.try_place(PropKind::FallenLog, (lx, ly), rot, None, full).is_some() {
            logs += 1;
        }
    }

    // Cover & canopy along roadside verge
    if road_y0 >= 3 {
        let north_verge = Rect { x0: 1, y0: 1, x1: w - 1, y1: road_y0 };
        b.paint_mulch_patch(w / 3, 1, 1);
        b.try_place(PropKind::GardenBed, ((w / 3).clamp(1, w - 3), 1), 0, None, north_verge);
        b.scatter_trees(PropKind::MediumTree, ((w - 2) / 6).clamp(1, 5), 3, north_verge);
        b.scatter_turf_prop(PropKind::Boulder, 2, north_verge);
    }
    b.scatter_trees(PropKind::LargeTree, ((w * h) / 96).clamp(1, 4), 5, full);
    b.scatter_trees(PropKind::MediumTree, ((w * h) / 56).clamp(2, 7), 3, full);
    b.scatter_turf_prop(PropKind::Bush, ((w * h) / 40).clamp(3, 10), full);
}
