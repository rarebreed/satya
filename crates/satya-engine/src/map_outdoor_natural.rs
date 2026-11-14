//! # Natural Creek & Roadside Ditch Battlemap Builders (`map_outdoor_natural`)
//!
//! Companion module to [`crate::map_outdoor_gen`] authoring meandering hydrological channels,
//! highway curves, ditches, culvert bridge crossings, and spline-tangent roadside vehicles.

use satya_schema::{EdgeSide, MaterialClass, PropKind, SurfaceFeatureKind, WallEdgeKind};

use crate::map_outdoor_gen::{OutdoorBuilder, RectM};
use crate::map_surface_geom::{along_tangent_rot_deg, ribbon_point_and_tangent_at_fraction};

/// Winding `WoodlandCreek`: curved stream across the sector, dirt feeder trail, footbridge deck,
/// mossy bank boulders/logs at continuous angles, and road-protected tree mulch rings.
pub(crate) fn build_woodland_creek(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w as f32, b.h as f32);
    let mid_x = w * b.rng.f32_range(0.42, 0.58);
    let drift_a = b.rng.f32_range(-2.2, 2.2);
    let drift_b = -drift_a * b.rng.f32_range(0.7, 1.25);

    let creek_ctrl = [
        (mid_x + drift_a * 0.4, 0.0),
        (mid_x + drift_a, h * 0.30),
        (mid_x + drift_b, h * 0.68),
        (mid_x - drift_a * 0.35, h),
    ];
    let creek_idx = b.add_ribbon_spline(SurfaceFeatureKind::CreekChannel, &creek_ctrl, 250, 301);

    // Timber footbridge spanning the meandering creek near mid-board (with BridgeDeck priority 70).
    let bridge_frac = b.rng.f32_range(0.40, 0.60);
    let _ = b.add_bridge_deck_across(creek_idx, bridge_frac, 2.4, 3.6);

    if let Some((b_pos, (tx, ty))) = ribbon_point_and_tangent_at_fraction(&b.surfaces[creek_idx], bridge_frac) {
        let (sx, sy) = (-ty, tx);
        let west_pt = ((b_pos.0 - sx * 2.1).max(0.0), (b_pos.1 - sy * 2.1).clamp(1.2, h - 1.2));
        let east_pt = ((b_pos.0 + sx * 2.1).min(w), (b_pos.1 + sy * 2.1).clamp(1.2, h - 1.2));
        let wy = b.rng.f32_range(h * 0.35, h * 0.65);
        let ey = b.rng.f32_range(h * 0.35, h * 0.65);
        b.add_ribbon_spline(
            SurfaceFeatureKind::DirtTrail,
            &[(0.0, wy), west_pt],
            180,
            302,
        );
        b.add_ribbon_spline(
            SurfaceFeatureKind::DirtTrail,
            &[east_pt, (w, ey)],
            180,
            303,
        );
    }

    // Bankside fallen logs and boulders oriented smoothly along or askew the creek flow.
    for frac in [0.20_f32, 0.80] {
        if let Some((pos, (tx, ty))) = ribbon_point_and_tangent_at_fraction(&b.surfaces[creek_idx], frac) {
            let (nx, ny) = (-ty, tx);
            let side = if b.rng.chance(1, 2) { 1.0 } else { -1.0 };
            let px = pos.0 + nx * 1.95 * side;
            let py = pos.1 + ny * 1.95 * side;
            let rot = (along_tangent_rot_deg(tx, ty) + b.rng.range(-28, 29)).rem_euclid(360);
            let _ = b.try_place_cm(
                PropKind::FallenLog,
                OutdoorBuilder::cm(px),
                OutdoorBuilder::cm(py),
                rot,
                None,
                false,
            );
        }
    }

    let bounds = RectM { x0: 1.4, y0: 1.4, x1: w - 1.4, y1: h - 1.4 };
    b.scatter_trees(PropKind::LargeTree, ((w * h) / 52.0) as i32 + 3, 3.8, bounds);
    b.scatter_trees(PropKind::MediumTree, ((w * h) / 42.0) as i32 + 3, 2.8, bounds);
    b.scatter_turf_prop_rot(PropKind::Boulder, 7, 1, bounds);
    b.scatter_turf_prop_rot(PropKind::FallenLog, 2, 1, bounds);
    b.scatter_turf_prop_rot(PropKind::Bush, 9, 1, bounds);
}

/// Sweeping `RoadsideDitch`: smoothly curved `AsphaltRoad`, parallel gravel shoulder, meandering
/// flanking `DrainageDitch`, cross-ditch culvert `BridgeDeck`, tangent-parked vehicles & Jersey barriers,
/// and strict road-surface purity.
pub(crate) fn build_roadside_ditch(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w as f32, b.h as f32);
    let road_low = b.rng.chance(1, 2);
    let base_y = if road_low { h * 0.72 } else { h * 0.28 };
    let sign = if road_low { -1.0_f32 } else { 1.0_f32 };

    // Smooth curved asphalt roadway traversing West -> East with subtle highway sweep.
    let bow = b.rng.f32_range(-1.3, 1.3);
    let road_ctrl = [
        (0.0, (base_y + bow * 0.5).clamp(2.4, h - 2.4)),
        (w * 0.34, (base_y - bow).clamp(2.4, h - 2.4)),
        (w * 0.68, (base_y + bow * 0.85).clamp(2.4, h - 2.4)),
        (w, (base_y - bow * 0.4).clamp(2.4, h - 2.4)),
    ];
    let road_idx = b.add_ribbon_spline(SurfaceFeatureKind::AsphaltRoad, &road_ctrl, 420, 401);

    // Parallel gravel shoulder apron adjacent to the far shoulder of the road (lower priority than AsphaltRoad).
    let shoulder_ctrl: Vec<(f32, f32)> = road_ctrl
        .iter()
        .map(|&(x, y)| (x, (y + sign * 2.25).clamp(1.2, h - 1.2)))
        .collect();
    b.add_ribbon_spline(SurfaceFeatureKind::GravelRoad, &shoulder_ctrl, 140, 402);

    // Parallel winding drainage ditch safely outside the asphalt corridor.
    let ditch_offset = sign * 4.35;
    let ditch_ctrl: Vec<(f32, f32)> = road_ctrl
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            let wiggle = if i == 1 || i == 2 { b.rng.f32_range(-0.65, 0.65) } else { 0.0 };
            (x, (y + ditch_offset + wiggle).clamp(1.3, h - 1.3))
        })
        .collect();
    let ditch_idx = b.add_ribbon_spline(SurfaceFeatureKind::DrainageDitch, &ditch_ctrl, 175, 403);

    // Culvert crossing across the drainage ditch so foot movement and LoS cross cleanly without ditch berms.
    let culvert_frac = b.rng.f32_range(0.38, 0.62);
    let _ = b.add_bridge_deck_across(ditch_idx, culvert_frac, 2.5, 2.8);

    // Tangent-aligned vehicles & roadside concrete barriers along the asphalt curve (± 4°).
    let _ = b.place_vehicle_along_ribbon(PropKind::Car, road_idx, 0.33, -sign * 0.65, 4);
    let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, road_idx, 0.68, sign * 0.65, 5);
    let _ = b.place_along_ribbon(PropKind::JerseyBarrier, road_idx, 0.49, sign * 2.05, 5, true);
    let _ = b.place_along_ribbon(PropKind::JerseyBarrier, road_idx, 0.57, -sign * 2.05, 5, true);

    // Natural trees, shrubs, and curved planting patches only on the non-road verge & far bank.
    let tree_bounds = RectM { x0: 1.4, y0: 1.3, x1: w - 1.4, y1: h - 1.3 };
    for _ in 0..2 {
        let cx = b.rng.f32_range(3.5, w - 3.5);
        let cy = if road_low {
            b.rng.f32_range(1.6, (base_y - 4.0).max(2.0))
        } else {
            b.rng.f32_range((base_y + 4.0).min(h - 2.0), h - 1.6)
        };
        let rx = b.rng.f32_range(1.2, 1.8);
        let ry = b.rng.f32_range(0.85, 1.2);
        let _ = b.try_add_organic_patch(
            SurfaceFeatureKind::GardenBedZone,
            (cx, cy),
            (rx, ry),
            0.60,
            18,
        );
    }
    b.scatter_trees(PropKind::LargeTree, 3, 3.6, tree_bounds);
    b.scatter_trees(PropKind::MediumTree, 4, 2.8, tree_bounds);
    b.scatter_turf_prop_rot(PropKind::FallenLog, 2, 1, tree_bounds);
    b.scatter_turf_prop_rot(PropKind::Boulder, 5, 1, tree_bounds);
    b.scatter_turf_prop_rot(PropKind::Bush, 7, 1, tree_bounds);
}

/// Winding `WoodedPark`: bordering curved asphalt road, meandering park trail, flagstone spur,
/// tangent benches, curved planting beds, and mature tree glades.
pub(crate) fn build_wooded_park(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w as f32, b.h as f32);
    // Bordering curved roadside asphalt edge + park flagstone walk connecting to meandering trail.
    let road_north = b.rng.chance(1, 2);
    let road_y = if road_north { 1.15 } else { h - 1.15 };
    let road_bow = (road_y + b.rng.f32_range(-0.28, 0.28)).clamp(0.9, h - 0.9);
    let _ = b.add_ribbon_spline(
        SurfaceFeatureKind::AsphaltRoad,
        &[(0.0, road_y), (w * 0.5, road_bow), (w, road_y)],
        210,
        100,
    );

    let y0 = b.rng.f32_range(h * 0.30, h * 0.48);
    let y1 = b.rng.f32_range(h * 0.52, h * 0.72);
    let y2 = b.rng.f32_range(h * 0.25, h * 0.55);
    let y3 = b.rng.f32_range(h * 0.42, h * 0.68);
    let trail_ctrl = [
        (0.0, y0),
        (w * 0.28, y1),
        (w * 0.56, y2),
        (w * 0.80, y3),
        (w, b.rng.f32_range(h * 0.35, h * 0.65)),
    ];
    let trail_idx = b.add_ribbon_spline(SurfaceFeatureKind::DirtTrail, &trail_ctrl, 240, 101);

    if h >= 10.0 {
        let spur_start = ribbon_point_and_tangent_at_fraction(&b.surfaces[trail_idx], 0.48)
            .map_or((w * 0.5, h * 0.5), |(p, _)| p);
        let spur_dx1 = b.rng.f32_range(-1.2, 1.2);
        let spur_dx2 = b.rng.f32_range(-1.4, 1.4);
        let spur_ctrl = [
            spur_start,
            (spur_start.0 + spur_dx1, (spur_start.1 + road_y) * 0.5),
            (spur_start.0 + spur_dx2, road_y),
        ];
        b.add_ribbon_spline(SurfaceFeatureKind::StoneWalk, &spur_ctrl, 185, 102);
    }

    // Benches tangent to the winding park trail.
    for &(frac, side) in &[(0.28, 1.55_f32), (0.68, -1.55_f32)] {
        let _ = b.place_along_ribbon(PropKind::ParkBench, trail_idx, frac, side, 6, false);
    }

    // Scenic curved planting beds & mulch glades safely outside the trail.
    for _ in 0..3 {
        let cx = b.rng.f32_range(3.5, w - 3.5);
        let cy = b.rng.f32_range(2.2, h - 2.2);
        let rx = b.rng.f32_range(1.3, 2.0);
        let ry = b.rng.f32_range(0.95, 1.5);
        let _ = b.try_add_organic_patch(
            SurfaceFeatureKind::GardenBedZone,
            (cx, cy),
            (rx, ry),
            0.55,
            18,
        );
    }

    let bounds = RectM { x0: 1.4, y0: 1.4, x1: w - 1.4, y1: h - 1.4 };
    b.scatter_trees(PropKind::LargeTree, ((w * h) / 48.0) as i32 + 3, 4.0, bounds);
    b.scatter_trees(PropKind::MediumTree, ((w * h) / 40.0) as i32 + 3, 3.0, bounds);
    b.scatter_turf_prop_rot(PropKind::PicnicTable, 2, 1, bounds);
    b.scatter_turf_prop_rot(PropKind::FallenLog, 3, 1, bounds);
    b.scatter_turf_prop_rot(PropKind::Boulder, 6, 1, bounds);
    b.scatter_turf_prop_rot(PropKind::Bush, 9, 1, bounds);
}

/// Brick `GardenEstate`: asymmetric manor house wing, gated brick courtyard, sweeping stone walk,
/// organic planting beds, and roadside asphalt entry drive.
pub(crate) fn build_garden_estate(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w, b.h);
    let wall_x0 = 3;
    let wall_x1 = (w - 3).max(wall_x0 + 8);
    let wall_y0 = 1;
    let wall_y1 = (h - 1).max(wall_y0 + 7);
    let mid_y = h / 2;

    // Vary whether the main manor house sits on the West or East courtyard wing (keeps spawns open).
    let manor_east = b.rng.chance(1, 2);
    let house_w = (wall_x1 - wall_x0 - 6).clamp(5, 7);
    let house_h = (wall_y1 - wall_y0 - 3).clamp(5, 7);
    let (hx0, hx1) = if manor_east {
        (wall_x1 - house_w, wall_x1)
    } else {
        (wall_x0, wall_x0 + house_w)
    };
    let hy0 = wall_y0 + 1 + ((wall_y1 - wall_y0 - house_h) / 2).max(0);
    let hy1 = (hy0 + house_h).min(wall_y1 - 1);

    // Outer estate brick perimeter with open pedestrian gates on West & East (aligned with spawns) and South.
    for x in wall_x0..wall_x1 {
        let on_gate = x == (wall_x0 + wall_x1) / 2 || x == (wall_x0 + wall_x1) / 2 + 1;
        b.put_edge(
            x,
            wall_y0,
            EdgeSide::North,
            if on_gate { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall },
            MaterialClass::Brick,
        );
        b.put_edge(
            x,
            wall_y1,
            EdgeSide::North,
            if on_gate { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall },
            MaterialClass::Brick,
        );
    }
    for y in wall_y0..wall_y1 {
        let w_gate = y == mid_y || y == (mid_y - 1).max(wall_y0);
        b.put_edge(
            wall_x0,
            y,
            EdgeSide::West,
            if w_gate { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall },
            MaterialClass::Brick,
        );
        b.put_edge(
            wall_x1,
            y,
            EdgeSide::West,
            if w_gate { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall },
            MaterialClass::Brick,
        );
    }

    // Manor house enclosure + interior floor polygon.
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[
            (hx0 as f32, hy0 as f32),
            (hx1 as f32, hy0 as f32),
            (hx1 as f32, hy1 as f32),
            (hx0 as f32, hy1 as f32),
        ],
        0,
        201,
    );
    let door_y = (hy0 + hy1) / 2;
    let courtyard_face_x = if manor_east { hx0 } else { hx1 };
    let outer_face_x = if manor_east { hx1 } else { hx0 };
    for y in hy0..hy1 {
        let is_door = y == door_y || y == door_y - 1;
        b.put_edge(
            courtyard_face_x,
            y,
            EdgeSide::West,
            if is_door { WallEdgeKind::DoorOpen } else { WallEdgeKind::Window },
            MaterialClass::Brick,
        );
        let is_outer_door = y == mid_y || y == (mid_y - 1).max(hy0);
        b.put_edge(
            outer_face_x,
            y,
            EdgeSide::West,
            if is_outer_door { WallEdgeKind::DoorOpen } else { WallEdgeKind::Window },
            MaterialClass::Brick,
        );
    }
    for x in hx0..hx1 {
        b.put_edge(x, hy0, EdgeSide::North, WallEdgeKind::Window, MaterialClass::Brick);
        b.put_edge(x, hy1, EdgeSide::North, WallEdgeKind::Window, MaterialClass::Brick);
    }

    // Manor interior furnishings with subtle 1° placement rotation variation.
    let hcx = ((hx0 + hx1) as f32 * 0.5 * 100.0) as i32;
    let hcy = ((hy0 + hy1) as f32 * 0.5 * 100.0) as i32;
    let d_rot = b.rng.range(-4, 5);
    let _ = b.try_place_cm(PropKind::DiningTable, hcx, hcy, d_rot, None, true);
    let sofa_x = if manor_east { (hx1 - 1) * 100 } else { (hx0 + 1) * 100 };
    let s_rot = b.rng.range(-5, 6);
    let _ = b.try_place_cm(PropKind::Sofa, sofa_x, (hy0 + 1) * 100 + 40, s_rot, None, true);
    let bs_rot = b.rng.range(176, 185);
    let _ = b.try_place_cm(PropKind::Bookshelf, hcx, (hy1 - 1) * 100 + 25, bs_rot, None, true);

    // Sweeping flagstone garden walk connecting courtyard gate to manor doorway.
    let gate_pt = if manor_east {
        (wall_x0 as f32 + 0.2, mid_y as f32 + 0.3)
    } else {
        (wall_x1 as f32 - 0.2, mid_y as f32 + 0.3)
    };
    let door_pt = (
        courtyard_face_x as f32 + if manor_east { -0.3 } else { 0.3 },
        door_y as f32 + 0.1,
    );
    let mid_bow_y = ((gate_pt.1 + door_pt.1) * 0.5 + b.rng.f32_range(-1.8, 1.8))
        .clamp(wall_y0 as f32 + 1.8, wall_y1 as f32 - 1.8);
    let walk_idx = b.add_ribbon_spline(
        SurfaceFeatureKind::StoneWalk,
        &[
            gate_pt,
            ((gate_pt.0 + door_pt.0) * 0.5, mid_bow_y),
            door_pt,
        ],
        190,
        202,
    );

    // Courtyard curved flower beds with stone edging + ornamental trees & benches.
    let (cy_x0, cy_x1) = if manor_east {
        (wall_x0 as f32 + 1.4, hx0 as f32 - 1.2)
    } else {
        (hx1 as f32 + 1.2, wall_x1 as f32 - 1.4)
    };
    if cy_x1 - cy_x0 >= 3.5 {
        for frac_y in [0.24_f32, 0.76_f32] {
            let bx = b.rng.f32_range(cy_x0 + 1.2, cy_x1 - 1.2);
            let by = wall_y0 as f32 + (wall_y1 - wall_y0) as f32 * frac_y;
            let brx = b.rng.f32_range(1.2, 1.8);
            let bry = b.rng.f32_range(0.85, 1.3);
            let _ = b.try_add_organic_patch(
                SurfaceFeatureKind::GardenBedZone,
                (bx, by),
                (brx, bry),
                0.55,
                22,
            );
        }
        let _ = b.place_along_ribbon(PropKind::ParkBench, walk_idx, 0.52, 1.45, 6, false);
        let yard_bounds = RectM {
            x0: cy_x0,
            y0: wall_y0 as f32 + 1.2,
            x1: cy_x1,
            y1: wall_y1 as f32 - 1.2,
        };
        b.scatter_trees(PropKind::MediumTree, 3, 2.8, yard_bounds);
        b.scatter_turf_prop_rot(PropKind::Hedge, 3, 1, yard_bounds);
        b.scatter_turf_prop_rot(PropKind::Bush, 5, 1, yard_bounds);
    }

    // Roadside entry drive strip outside west/east edge + parked manor sedan with subtle jitter.
    if w >= 18 {
        let drive_x = if manor_east { w as f32 - 1.6 } else { 1.6 };
        let drive_drift = b.rng.f32_range(-0.35, 0.35);
        let drive_idx = b.add_ribbon_spline(
            SurfaceFeatureKind::AsphaltRoad,
            &[(drive_x, 0.0), (drive_x + drive_drift, h as f32)],
            240,
            203,
        );
        let _ = b.place_vehicle_along_ribbon(PropKind::Car, drive_idx, 0.22, 0.0, 4);
    }

    let outer_bounds = RectM { x0: 1.2, y0: 1.2, x1: w as f32 - 1.2, y1: h as f32 - 1.2 };
    b.scatter_turf_prop_rot(PropKind::Boulder, 3, 1, outer_bounds);
}
