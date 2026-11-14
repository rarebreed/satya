//! Multi-topology procedural site grammars for `GardenEstate` ("Walled Estate & Courtyard").
//!
//! Replaces the single fixed rectangular manor + box wall with three seeded architectural
//! compound topologies across compact (`20×14`) through massive (`64×48` / `96×72`) arenas:
//! - [`EstateTopology::ForecourtTurnaroundManor`]: Outer street approach, gate-pier entry,
//!   circular turnaround drive around a sculptural parterre, grand manor & detached carriage house.
//! - [`EstateTopology::LWingCarriageCompound`]: Quadrant-mirrored L-wing mansion framing a paved
//!   terrace courtyard, opposite stable/carriage wing, parterres & sweeping driveway.
//! - [`EstateTopology::CloisteredPavilionVilla`]: Dual wing pavilions flanking a pillared
//!   cloister courtyard with sunken reflection garden, footbridge & perimeter shade orchard.

use crate::map_outdoor_gen::{OutdoorBuilder, RectM};
use satya_schema::{EdgeSide, MaterialClass, PropKind, SurfaceFeatureKind, WallEdgeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EstateTopology {
    ForecourtTurnaroundManor,
    LWingCarriageCompound,
    CloisteredPavilionVilla,
}

#[derive(Debug, Clone, Copy)]
struct BoxI {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl BoxI {
    const fn w(self) -> i32 { self.x1 - self.x0 }
    const fn h(self) -> i32 { self.y1 - self.y0 }
    fn rect_m(self, inset: f32) -> RectM {
        RectM { x0: self.x0 as f32 + inset, y0: self.y0 as f32 + inset, x1: self.x1 as f32 - inset, y1: self.y1 as f32 - inset }
    }
}

/// Entry point for [`crate::map_outdoor_gen::OutdoorStyle::GardenEstate`].
pub(crate) fn build_garden_estate(b: &mut OutdoorBuilder) {
    if b.w < 24 || b.h < 18 {
        build_compact_estate(b);
        return;
    }
    let topo = match b.rng.range(0, 3) {
        0 => EstateTopology::ForecourtTurnaroundManor,
        1 => EstateTopology::LWingCarriageCompound,
        _ => EstateTopology::CloisteredPavilionVilla,
    };
    match topo {
        EstateTopology::ForecourtTurnaroundManor => build_forecourt_turnaround(b),
        EstateTopology::LWingCarriageCompound => build_lwing_compound(b),
        EstateTopology::CloisteredPavilionVilla => build_cloistered_villa(b),
    }
}

fn rect_poly(x0: f32, y0: f32, x1: f32, y1: f32) -> [(f32, f32); 4] {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn ellipse_poly(cx: f32, cy: f32, rx: f32, ry: f32, steps: usize) -> Vec<(f32, f32)> {
    (0..steps)
        .map(|i| {
            let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
            (cx + a.cos() * rx, cy + a.sin() * ry)
        })
        .collect()
}

fn reserve_door_clearance(b: &mut OutdoorBuilder, x: i32, y: i32, side: EdgeSide) {
    match side {
        EdgeSide::North => {
            b.spawn_pads_m.push((x as f32 + 0.5, y as f32 + 0.5));
            if y > 0 { b.spawn_pads_m.push((x as f32 + 0.5, y as f32 - 0.5)); }
        }
        EdgeSide::West => {
            b.spawn_pads_m.push((x as f32 + 0.5, y as f32 + 0.5));
            if x > 0 { b.spawn_pads_m.push((x as f32 - 0.5, y as f32 + 0.5)); }
        }
    }
}

fn put_door(b: &mut OutdoorBuilder, x: i32, y: i32, side: EdgeSide, open: bool, mat: MaterialClass) {
    let kind = if open { WallEdgeKind::DoorOpen } else { WallEdgeKind::DoorClosed };
    b.put_edge(x, y, side, kind, mat);
    reserve_door_clearance(b, x, y, side);
}

fn place_rot_m(b: &mut OutdoorBuilder, kind: PropKind, x_m: f32, y_m: f32, rot_lo: i32, rot_hi: i32, allow_on_pavement: bool) {
    let rot = b.rng.range(rot_lo, rot_hi);
    b.try_place_cm(kind, OutdoorBuilder::cm(x_m), OutdoorBuilder::cm(y_m), rot, None, allow_on_pavement);
}

fn build_compound_wall(b: &mut OutdoorBuilder, comp: BoxI, north_gate_x0: i32, north_gate_w: i32, south_gate_x0: i32, south_gate_w: i32) {
    for x in comp.x0..comp.x1 {
        let is_grille = (x - comp.x0) % 4 == 1 || (x - comp.x0) % 4 == 2;
        let (kind, mat) = if is_grille { (WallEdgeKind::Window, MaterialClass::SheetSteel) } else { (WallEdgeKind::Wall, MaterialClass::Brick) };
        if north_gate_w > 0 && x >= north_gate_x0 && x < north_gate_x0 + north_gate_w {
            b.remove_edge(x, comp.y0, EdgeSide::North);
            reserve_door_clearance(b, x, comp.y0, EdgeSide::North);
        } else {
            b.put_edge(x, comp.y0, EdgeSide::North, kind, mat);
        }
        if south_gate_w > 0 && x >= south_gate_x0 && x < south_gate_x0 + south_gate_w {
            b.remove_edge(x, comp.y1, EdgeSide::North);
            reserve_door_clearance(b, x, comp.y1, EdgeSide::North);
        } else {
            b.put_edge(x, comp.y1, EdgeSide::North, kind, mat);
        }
    }
    let mid_y = (comp.y0 + comp.y1) / 2;
    for y in comp.y0..comp.y1 {
        if y == mid_y || y == mid_y - 1 {
            b.remove_edge(comp.x0, y, EdgeSide::West);
            b.remove_edge(comp.x1, y, EdgeSide::West);
            reserve_door_clearance(b, comp.x0, y, EdgeSide::West);
            reserve_door_clearance(b, comp.x1, y, EdgeSide::West);
        } else {
            let (kind, mat) = if (y - comp.y0) % 4 == 1 { (WallEdgeKind::Window, MaterialClass::SheetSteel) } else { (WallEdgeKind::Wall, MaterialClass::Brick) };
            b.put_edge(comp.x0, y, EdgeSide::West, kind, mat);
            b.put_edge(comp.x1, y, EdgeSide::West, kind, mat);
        }
    }
}

fn build_wing_shell(b: &mut OutdoorBuilder, r: BoxI, wall_mat: MaterialClass) {
    b.add_poly(SurfaceFeatureKind::InteriorFloor, &rect_poly(r.x0 as f32, r.y0 as f32, r.x1 as f32, r.y1 as f32), 0);
    for x in r.x0..r.x1 {
        let kn = if (x - r.x0) % 3 == 1 && x + 1 < r.x1 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
        b.put_edge(x, r.y0, EdgeSide::North, kn, wall_mat);
        b.put_edge(x, r.y1, EdgeSide::North, kn, wall_mat);
    }
    for y in r.y0..r.y1 {
        let kw = if (y - r.y0) % 3 == 1 && y + 1 < r.y1 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
        b.put_edge(r.x0, y, EdgeSide::West, kw, wall_mat);
        b.put_edge(r.x1, y, EdgeSide::West, kw, wall_mat);
    }
}

fn partition_manor_rooms(b: &mut OutdoorBuilder, r: BoxI) -> Vec<BoxI> {
    if r.w() < 6 || r.h() < 5 {
        return vec![r];
    }
    let sx = r.x0 + r.w() / 2;
    let sy = r.y0 + r.h() / 2;
    for y in r.y0..r.y1 {
        b.put_edge(sx, y, EdgeSide::West, WallEdgeKind::Wall, MaterialClass::Drywall);
    }
    if r.h() >= 7 {
        for x in r.x0..r.x1 {
            b.put_edge(x, sy, EdgeSide::North, WallEdgeKind::Wall, MaterialClass::Drywall);
        }
        put_door(b, sx, r.y0 + (sy - r.y0) / 2, EdgeSide::West, false, MaterialClass::SolidWood);
        put_door(b, sx, sy + (r.y1 - sy) / 2, EdgeSide::West, false, MaterialClass::SolidWood);
        put_door(b, r.x0 + (sx - r.x0) / 2, sy, EdgeSide::North, false, MaterialClass::SolidWood);
        put_door(b, sx + (r.x1 - sx) / 2, sy, EdgeSide::North, true, MaterialClass::SolidWood);
        vec![
            BoxI { x0: r.x0, y0: r.y0, x1: sx, y1: sy },
            BoxI { x0: sx, y0: r.y0, x1: r.x1, y1: sy },
            BoxI { x0: r.x0, y0: sy, x1: sx, y1: r.y1 },
            BoxI { x0: sx, y0: sy, x1: r.x1, y1: r.y1 },
        ]
    } else {
        put_door(b, sx, r.y0 + r.h() / 2, EdgeSide::West, false, MaterialClass::SolidWood);
        vec![BoxI { x0: r.x0, y0: r.y0, x1: sx, y1: r.y1 }, BoxI { x0: sx, y0: r.y0, x1: r.x1, y1: r.y1 }]
    }
}

fn furnish_estate_rooms(b: &mut OutdoorBuilder, rooms: &[BoxI]) {
    for (idx, &rm) in rooms.iter().enumerate() {
        let cx = ((rm.x0 + rm.x1) as f32 * 50.0) as i32;
        let cy = ((rm.y0 + rm.y1) as f32 * 50.0) as i32;
        let rot = (idx as i32 * 90) + b.rng.range(-12, 13);
        let center_kind = match idx % 4 { 0 => PropKind::Sofa, 1 => PropKind::DiningTable, 2 => PropKind::Desk, _ => PropKind::Bed };
        b.try_place_cm(center_kind, cx, cy, rot, None, true);
        if rm.w() >= 4 && rm.h() >= 4 {
            let accent = match idx % 3 { 0 => PropKind::Bookshelf, 1 => PropKind::KitchenCounter, _ => PropKind::Armchair };
            b.try_place_cm(accent, rm.x0 * 100 + 85, rm.y0 * 100 + 85, (idx as i32 * 90).rem_euclid(360), None, true);
            let third = if idx % 2 == 0 { PropKind::CoffeeTable } else { PropKind::Wardrobe };
            b.try_place_cm(third, rm.x1 * 100 - 85, rm.y1 * 100 - 85, ((idx as i32 * 90) + 180).rem_euclid(360), None, true);
        }
    }
}

fn build_carriage_house(b: &mut OutdoorBuilder, r: BoxI, bay_on_south: bool) {
    b.add_poly(SurfaceFeatureKind::InteriorFloor, &rect_poly(r.x0 as f32, r.y0 as f32, r.x1 as f32, r.y1 as f32), 0);
    for x in r.x0..r.x1 {
        b.put_edge(x, r.y0, EdgeSide::North, WallEdgeKind::Wall, MaterialClass::Brick);
        b.put_edge(x, r.y1, EdgeSide::North, WallEdgeKind::Wall, MaterialClass::Brick);
    }
    for y in r.y0..r.y1 {
        let kw = if y == r.y0 + 1 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
        b.put_edge(r.x0, y, EdgeSide::West, kw, MaterialClass::Brick);
        b.put_edge(r.x1, y, EdgeSide::West, kw, MaterialClass::Brick);
    }
    let bay_y = if bay_on_south { r.y1 } else { r.y0 };
    let mid_x = (r.x0 + r.x1) / 2;
    put_door(b, mid_x - 1, bay_y, EdgeSide::North, true, MaterialClass::SolidWood);
    put_door(b, mid_x, bay_y, EdgeSide::North, true, MaterialClass::SolidWood);
    put_door(b, r.x0, (r.y0 + r.y1) / 2, EdgeSide::West, false, MaterialClass::SolidWood);
    b.try_place_cm(PropKind::SteelShelving, r.x0 * 100 + 75, r.y0 * 100 + 75, 0, None, true);
    let crate_rot = b.rng.range(0, 45);
    b.try_place_cm(PropKind::WoodCrate, r.x1 * 100 - 75, r.y1 * 100 - 75, crate_rot, None, true);
}

fn dress_estate_grounds(b: &mut OutdoorBuilder, ground_rect: RectM, hedge_anchors: &[(f32, f32, i32)], tree_count: i32) {
    for &(hx, hy, rot) in hedge_anchors {
        b.try_place_cm(PropKind::Hedge, OutdoorBuilder::cm(hx), OutdoorBuilder::cm(hy), rot, Some(220), false);
    }
    let patch_count = ((tree_count / 3).clamp(3, 8)) as usize;
    for i in 0..patch_count {
        let px = b.rng.f32_range(ground_rect.x0 + 1.5, (ground_rect.x1 - 1.5).max(ground_rect.x0 + 2.0));
        let py = b.rng.f32_range(ground_rect.y0 + 1.5, (ground_rect.y1 - 1.5).max(ground_rect.y0 + 2.0));
        let kind = if i % 2 == 0 { SurfaceFeatureKind::GardenBedZone } else { SurfaceFeatureKind::MulchZone };
        let rx = b.rng.f32_range(1.1, 2.0);
        let ry = b.rng.f32_range(0.9, 1.7);
        b.try_add_organic_patch(kind, (px, py), (rx, ry), 0.35, 36);
    }
    b.scatter_trees(PropKind::LargeTree, tree_count / 2 + 1, 3.2, ground_rect);
    b.scatter_trees(PropKind::MediumTree, tree_count / 2 + 2, 2.4, ground_rect);
    b.scatter_turf_prop_rot(PropKind::Bush, tree_count, 35, ground_rect);
    b.scatter_turf_prop_rot(PropKind::ParkBench, (patch_count / 2 + 2) as i32, 30, ground_rect);
    b.scatter_turf_prop_rot(PropKind::Planter, (patch_count / 2 + 2) as i32, 45, ground_rect);
    b.scatter_turf_prop_rot(PropKind::Boulder, (patch_count / 3 + 1) as i32, 30, ground_rect);
}

/// Compact `20×14`-compatible estate with asymmetric manor, carriage apron, curved stone walk & hedges.
fn build_compact_estate(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let street_y = hf - 1.65;
    let dy = b.rng.f32_range(-0.2, 0.2);
    let street = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[(0.0, street_y + dy), (wf * 0.35, street_y + 0.15), (wf * 0.7, street_y - 0.15), (wf, street_y)],
        320,
    );
    let gate_x = (w / 3).max(4);
    b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[(gate_x as f32 + 1.0, street_y), (gate_x as f32 + 1.1, hf - 3.5), (gate_x as f32 + 1.4, (hf * 0.52).max(5.5)), (gate_x as f32 + 3.2, (hf * 0.42).max(4.5))],
        250,
    );
    let manor_right = b.rng.chance(1, 2);
    let mx0 = if manor_right { (w / 2).max(9) } else { 2 };
    let mx1 = if manor_right { w - 2 } else { (w / 2).max(8) };
    let manor = BoxI { x0: mx0, y0: 2, x1: mx1, y1: (h - 5).max(7) };
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[(gate_x as f32 + 1.5, hf - 3.8), (wf * 0.5, hf * 0.58), ((manor.x0 + manor.x1) as f32 * 0.5, manor.y1 as f32 + 0.4), ((manor.x0 + manor.x1) as f32 * 0.5, manor.y1 as f32 - 0.2)],
        160,
    );

    let comp = BoxI { x0: 1, y0: 1, x1: w - 1, y1: h - 3 };
    build_compound_wall(b, comp, w / 2, 2, gate_x, 2);
    build_wing_shell(b, manor, MaterialClass::Brick);
    put_door(b, (manor.x0 + manor.x1) / 2, manor.y1, EdgeSide::North, true, MaterialClass::SolidWood);
    put_door(b, manor.x0, (manor.y0 + manor.y1) / 2, EdgeSide::West, false, MaterialClass::SolidWood);
    let rooms = partition_manor_rooms(b, manor);
    furnish_estate_rooms(b, &rooms);

    b.place_vehicle_along_ribbon(PropKind::Car, street, 0.32, 0.2, 5);
    place_rot_m(b, PropKind::UtilityVan, gate_x as f32 + 1.2, hf - 4.4, 78, 103, true);
    dress_estate_grounds(b, comp.rect_m(0.8), &[(wf * 0.28, 3.2, 0), (wf * 0.72, 3.2, 0)], 4);
}

/// Topology 0: Street avenue, gate piers, elliptical turnaround around sculptural island, grand manor & carriage house.
fn build_forecourt_turnaround(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let gate_top = b.rng.chance(1, 2);
    let street_y = if gate_top { 2.0 } else { hf - 2.0 };
    let comp = BoxI { x0: 2, y0: if gate_top { 4 } else { 2 }, x1: w - 2, y1: if gate_top { h - 2 } else { h - 4 } };

    let (d0, d1, d2, d3) = (b.rng.f32_range(-0.3, 0.3), b.rng.f32_range(-0.35, 0.35), b.rng.f32_range(-0.35, 0.35), b.rng.f32_range(-0.25, 0.25));
    let street = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &[(0.0, street_y + d0), (wf * 0.33, street_y + d1), (wf * 0.67, street_y + d2), (wf, street_y + d3)], 380);

    let rear_pad = if comp.h() >= 18 { 3 } else { 1 };
    let manor_w = (comp.w() / 2).clamp(9, 26);
    let manor_h = (comp.h() * 34 / 100).clamp(4, 15);
    let mx0 = (w / 2 - manor_w / 2).clamp(comp.x0 + 3, comp.x1 - manor_w - 3);
    let (my0, my1) = if gate_top { (comp.y1 - manor_h - rear_pad, comp.y1 - rear_pad) } else { (comp.y0 + rear_pad, comp.y0 + rear_pad + manor_h) };
    let manor = BoxI { x0: mx0, y0: my0, x1: mx0 + manor_w, y1: my1 };

    let front_depth = if gate_top { (manor.y0 - comp.y0) as f32 } else { (comp.y1 - manor.y1) as f32 };
    let loop_w_cm = if front_depth < 9.0 { 210 } else { 280 };
    let half_loop = loop_w_cm as f32 / 200.0;
    let ry = ((front_depth - half_loop * 2.0 - 1.2) * 0.5).clamp(1.2, 2.8);
    let gate_x = (w / 2 + b.rng.range(-2, 3)).clamp(comp.x0 + 4, comp.x1 - 7);
    let gx_m = gate_x as f32 + 1.5;
    let island_off = (half_loop + ry + 0.55).min(front_depth * 0.52);
    let island_y = if gate_top { comp.y0 as f32 + island_off } else { comp.y1 as f32 - island_off };
    let rx = (comp.w() as f32 * 0.16).clamp(3.0, 5.8);
    let gate_edge_y = if gate_top { comp.y0 as f32 } else { comp.y1 as f32 };
    let loop_far_y = if gate_top {
        (island_y + ry + 0.4).min(manor.y0 as f32 - half_loop - 0.35)
    } else {
        (island_y - ry - 0.4).max(manor.y1 as f32 + half_loop + 0.35)
    };
    b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &[(gx_m, gate_edge_y), (gx_m - rx, island_y), (gx_m, loop_far_y), (gx_m + rx, island_y), (gx_m, gate_edge_y)], loop_w_cm);
    b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &[(gx_m, street_y), (gx_m, if gate_top { comp.y0 as f32 + 0.5 } else { comp.y1 as f32 - 0.5 })], 280);
    b.add_poly(SurfaceFeatureKind::GardenBedZone, &ellipse_poly(gx_m, island_y, (rx - half_loop - 0.25).max(1.2), (ry - 0.35).clamp(0.85, 1.45), 16), 120);

    let carriage_w = (comp.w() / 6).clamp(5, 8);
    let carriage_h = 5.min(comp.h() / 3).max(4);
    let carriage_right = b.rng.chance(1, 2);
    let cx0 = if carriage_right { comp.x1 - carriage_w - 1 } else { comp.x0 + 1 };
    let cy0 = if gate_top { comp.y0 + 2 } else { comp.y1 - carriage_h - 2 };
    let carriage = BoxI { x0: cx0, y0: cy0, x1: cx0 + carriage_w, y1: cy0 + carriage_h };
    b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[(gx_m + if carriage_right { rx } else { -rx }, island_y), ((carriage.x0 + carriage.x1) as f32 * 0.5, if gate_top { carriage.y1 as f32 + 1.0 } else { carriage.y0 as f32 - 1.0 })],
        240,
    );
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[(gx_m, loop_far_y), (mx0 as f32 - 1.2, (my0 + my1) as f32 * 0.5), ((manor.x0 + manor.x1) as f32 * 0.5, if gate_top { comp.y1 as f32 - 0.8 } else { comp.y0 as f32 + 0.8 }), (manor.x1 as f32 + 1.2, (my0 + my1) as f32 * 0.5)],
        170,
    );

    build_compound_wall(b, comp, if gate_top { gate_x } else { w / 2 - 1 }, if gate_top { 3 } else { 2 }, if !gate_top { gate_x } else { w / 2 - 1 }, if !gate_top { 3 } else { 2 });
    build_wing_shell(b, manor, MaterialClass::Brick);
    let front_y = if gate_top { manor.y0 } else { manor.y1 };
    let back_y = if gate_top { manor.y1 } else { manor.y0 };
    put_door(b, (manor.x0 + manor.x1) / 2, front_y, EdgeSide::North, true, MaterialClass::SolidWood);
    put_door(b, (manor.x0 + manor.x1) / 2, back_y, EdgeSide::North, false, MaterialClass::SolidWood);
    put_door(b, manor.x0, (manor.y0 + manor.y1) / 2, EdgeSide::West, false, MaterialClass::SolidWood);
    let rooms = partition_manor_rooms(b, manor);
    furnish_estate_rooms(b, &rooms);

    build_carriage_house(b, carriage, gate_top);
    b.try_place_cm(PropKind::Planter, OutdoorBuilder::cm(gx_m), OutdoorBuilder::cm(island_y), 0, None, true);
    b.place_vehicle_along_ribbon(PropKind::Car, street, 0.25, 0.2, 6);
    place_rot_m(b, PropKind::UtilityVan, gx_m - rx, island_y, 0, 360, true);
    dress_estate_grounds(b, comp.rect_m(1.0), &[(manor.x0 as f32 - 2.0, (manor.y0 + manor.y1) as f32 * 0.5, 90), (manor.x1 as f32 + 2.0, (manor.y0 + manor.y1) as f32 * 0.5, 90)], (w * h / 110).clamp(6, 16));
}

/// Topology 1: Quadrant-mirrored L-wing mansion, paved terrace court & detached stable/carriage wing.
fn build_lwing_compound(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let flip_x = b.rng.chance(1, 2);
    let comp = BoxI { x0: 2, y0: 2, x1: w - 2, y1: h - 3 };

    let street_y = hf - 1.5;
    let street = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &[(0.0, street_y), (wf * 0.4, street_y + 0.25), (wf * 0.8, street_y - 0.2), (wf, street_y)], 340);

    let bar_w = (comp.w() * 48 / 100).clamp(10, 24);
    let bar_h = (comp.h() * 34 / 100).clamp(5, 11);
    let wing_w = (bar_w / 2).clamp(5, 11);
    let wing_h = (comp.h() * 32 / 100).clamp(5, 11);

    let (bar, wing, car) = if !flip_x {
        (
            BoxI { x0: comp.x0 + 2, y0: comp.y0 + 2, x1: comp.x0 + 2 + bar_w, y1: comp.y0 + 2 + bar_h },
            BoxI { x0: comp.x0 + 2, y0: comp.y0 + 2 + bar_h, x1: comp.x0 + 2 + wing_w, y1: (comp.y0 + 2 + bar_h + wing_h).min(comp.y1 - 3) },
            BoxI { x0: comp.x1 - 7, y0: comp.y1 - 6, x1: comp.x1 - 2, y1: comp.y1 - 2 },
        )
    } else {
        (
            BoxI { x0: comp.x1 - 2 - bar_w, y0: comp.y0 + 2, x1: comp.x1 - 2, y1: comp.y0 + 2 + bar_h },
            BoxI { x0: comp.x1 - 2 - wing_w, y0: comp.y0 + 2 + bar_h, x1: comp.x1 - 2, y1: (comp.y0 + 2 + bar_h + wing_h).min(comp.y1 - 3) },
            BoxI { x0: comp.x0 + 2, y0: comp.y1 - 6, x1: comp.x0 + 7, y1: comp.y1 - 2 },
        )
    };

    let gate_x = ((car.x0 + car.x1) / 2 - 1).clamp(comp.x0 + 3, comp.x1 - 5);
    let drive_pts = [(gate_x as f32 + 1.5, street_y), (gate_x as f32 + 1.5, comp.y1 as f32 - 0.5), ((car.x0 + car.x1) as f32 * 0.5, car.y0 as f32 - 1.4), (wf * 0.5, (bar.y1 as f32 + 2.5).min(comp.y1 as f32 - 4.0))];
    b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &drive_pts, 270);

    let (tx0, tx1) = if !flip_x { (wing.x1 as f32 + 0.5, (bar.x1 as f32 + 1.5).min(comp.x1 as f32 - 2.0)) } else { ((bar.x0 as f32 - 1.5).max(comp.x0 as f32 + 2.0), wing.x0 as f32 - 0.5) };
    let (ty0, ty1) = (bar.y1 as f32 + 0.4, wing.y1 as f32 - 0.2);
    if tx1 - tx0 >= 3.0 && ty1 - ty0 >= 3.0 {
        b.add_poly(SurfaceFeatureKind::ConcreteWalk, &rect_poly(tx0, ty0, tx1, ty1), 85);
    }
    b.add_ribbon(SurfaceFeatureKind::StoneWalk, &[((tx0 + tx1) * 0.5, (ty0 + ty1) * 0.5), (wf * 0.52, (ty1 + comp.y1 as f32) * 0.5), ((car.x0 + car.x1) as f32 * 0.5, car.y0 as f32 - 1.0)], 175);

    build_compound_wall(b, comp, w / 2 - 1, 2, gate_x, 3);
    build_wing_shell(b, bar, MaterialClass::Brick);
    build_wing_shell(b, wing, MaterialClass::Brick);
    put_door(b, (wing.x0 + wing.x1) / 2, bar.y1, EdgeSide::North, true, MaterialClass::SolidWood);
    put_door(b, if !flip_x { wing.x1 } else { wing.x0 }, (wing.y0 + wing.y1) / 2, EdgeSide::West, true, MaterialClass::SolidWood);
    put_door(b, if !flip_x { bar.x1 - 2 } else { bar.x0 + 2 }, bar.y1, EdgeSide::North, false, MaterialClass::SolidWood);
    let mut rooms = partition_manor_rooms(b, bar);
    rooms.push(wing);
    furnish_estate_rooms(b, &rooms);
    build_carriage_house(b, car, false);

    b.try_place_cm(PropKind::RoundTable, OutdoorBuilder::cm((tx0 + tx1) * 0.5), OutdoorBuilder::cm((ty0 + ty1) * 0.5), 18, None, true);
    b.place_vehicle_along_ribbon(PropKind::Car, street, 0.72, 0.2, 8);
    place_rot_m(b, PropKind::Car, (car.x0 + car.x1) as f32 * 0.5, (car.y0 as f32 - 2.2).max(2.5), -25, 26, true);
    dress_estate_grounds(b, comp.rect_m(1.0), &[((tx0 + tx1) * 0.5, ty1 + 0.8, 0), (wf * 0.5, comp.y0 as f32 + 1.4, 0)], (w * h / 105).clamp(6, 16));
}

/// Topology 2: Twin opposed wing pavilions around a central cloistered garden courtyard & reflecting stream.
fn build_cloistered_villa(b: &mut OutdoorBuilder) {
    let (w, h) = (b.w, b.h);
    let wf = w as f32;
    let comp = BoxI { x0: 2, y0: 4, x1: w - 2, y1: h - 2 };
    let street = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &[(0.0, 2.0), (wf * 0.35, 2.3), (wf * 0.7, 1.75), (wf, 2.0)], 350);

    let west_w = (comp.w() / 4).clamp(5, 11);
    let east_w = (comp.w() / 5).clamp(5, 9);
    let v_pad = (comp.h() / 5).max(2);
    let west_villa = BoxI { x0: comp.x0 + 2, y0: comp.y0 + v_pad, x1: comp.x0 + 2 + west_w, y1: comp.y1 - v_pad };
    let east_pav = BoxI { x0: comp.x1 - 2 - east_w, y0: comp.y0 + v_pad + 1, x1: comp.x1 - 2, y1: comp.y1 - v_pad - 1 };

    let court = BoxI { x0: west_villa.x1, y0: west_villa.y0 + 1, x1: east_pav.x0, y1: west_villa.y1 - 1 };
    let mid_x = (court.x0 + court.x1) as f32 * 0.5;
    let mid_y = (court.y0 + court.y1) as f32 * 0.5;

    let gate_x = w / 2 - 1;
    b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &[(gate_x as f32 + 1.5, 2.0), (gate_x as f32 + 1.5, comp.y0 as f32 + 1.5), (mid_x, court.y0 as f32 - 0.5)], 280);
    let cx0 = court.x0 as f32 + 0.9;
    let cx1 = (court.x1 as f32 - 0.9).max(cx0 + 1.5);
    let cy0 = court.y0 as f32 + 0.9;
    let cy1 = (court.y1 as f32 - 0.9).max(cy0 + 1.5);
    for seg in [
        [(cx0, cy0), (cx1, cy0)],
        [(cx1, cy0), (cx1, cy1)],
        [(cx1, cy1), (cx0, cy1)],
        [(cx0, cy1), (cx0, cy0)],
        [(mid_x, cy0), (mid_x, (mid_y - 0.9).max(cy0 + 0.4))],
        [(mid_x, (mid_y + 0.9).min(cy1 - 0.4)), (mid_x, cy1)],
    ] {
        b.add_ribbon(SurfaceFeatureKind::StoneWalk, &seg, 185);
    }
    let basin_rx = ((court.w() as f32) * 0.25).clamp(1.3, 4.2);
    let basin_ry = ((court.h() as f32) * 0.24).clamp(1.3, 3.6);
    b.add_poly(SurfaceFeatureKind::GardenBedZone, &ellipse_poly(mid_x, mid_y, basin_rx, basin_ry, 18), 120);
    let rill = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &[(mid_x - basin_rx * 0.65, mid_y), (mid_x, mid_y - basin_ry * 0.35), (mid_x + basin_rx * 0.65, mid_y)], 190);
    b.add_bridge_deck_across(rill, 0.5, 1.8, 2.6);

    build_compound_wall(b, comp, gate_x, 3, w / 2 - 1, 2);
    build_wing_shell(b, west_villa, MaterialClass::Brick);
    build_wing_shell(b, east_pav, MaterialClass::ConcreteBlock);
    put_door(b, west_villa.x1, (west_villa.y0 + west_villa.y1) / 2, EdgeSide::West, true, MaterialClass::SolidWood);
    put_door(b, east_pav.x0, (east_pav.y0 + east_pav.y1) / 2, EdgeSide::West, true, MaterialClass::SolidWood);
    let mut rooms = partition_manor_rooms(b, west_villa);
    rooms.extend(partition_manor_rooms(b, east_pav));
    furnish_estate_rooms(b, &rooms);

    if court.w() >= 6 {
        for &(px, py) in &[
            (court.x0 as f32 + 1.0, court.y0 as f32 + 1.0),
            (court.x1 as f32 - 1.0, court.y0 as f32 + 1.0),
            (court.x0 as f32 + 1.0, court.y1 as f32 - 1.0),
            (court.x1 as f32 - 1.0, court.y1 as f32 - 1.0),
        ] {
            b.try_place_cm(PropKind::StructuralPillar, OutdoorBuilder::cm(px), OutdoorBuilder::cm(py), 0, None, true);
        }
    }
    b.place_vehicle_along_ribbon(PropKind::UtilityVan, street, 0.68, -0.2, 6);
    b.try_place_cm(PropKind::Car, OutdoorBuilder::cm(gate_x as f32 + 4.2), OutdoorBuilder::cm(comp.y0 as f32 + 1.5), 85, None, true);
    dress_estate_grounds(b, comp.rect_m(0.8), &[(mid_x, court.y0 as f32 - 0.8, 0), (mid_x, court.y1 as f32 + 0.8, 0)], (w * h / 100).clamp(6, 18));
}
