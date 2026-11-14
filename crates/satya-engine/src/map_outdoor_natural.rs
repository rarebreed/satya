//! Multi-topology natural outdoor sector grammars for `WoodlandCreek`, `RoadsideDitch`, and `WoodedPark`.
//!
//! Emits curved Catmull-Rom road/creek/ditch ribbons, organic `GardenBedZone` / `MulchZone` contours,
//! `360°` angled props, and multiple seeded route & clearing topologies for boards from `20×14` up to `96×72`.

use crate::map_outdoor_gen::{OutdoorBuilder, RectM};
use satya_schema::{PropKind, SurfaceFeatureKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreekTopology {
    NorthSouthMeander,
    BraidedConfluence,
    DiagonalSweepRavine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoadsideTopology {
    EastWestCountryCurve,
    NorthSouthRuralLane,
    SweepingBendWithPullout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParkTopology {
    WindingCrossTrail,
    ForkedGlade,
    LoopTrailWithClearing,
}

fn full_rect(b: &OutdoorBuilder, inset: f32) -> RectM {
    RectM { x0: inset, y0: inset, x1: b.w as f32 - inset, y1: b.h as f32 - inset }
}

fn place_rot_m(b: &mut OutdoorBuilder, kind: PropKind, x_m: f32, y_m: f32, lo: i32, hi: i32, allow_on_pavement: bool) {
    let rot = b.rng.range(lo, hi);
    b.try_place_cm(kind, OutdoorBuilder::cm(x_m), OutdoorBuilder::cm(y_m), rot, None, allow_on_pavement);
}

fn scatter_organic_beds(b: &mut OutdoorBuilder, count: i32, bounds: RectM, include_garden: bool) {
    for i in 0..count {
        let px = b.rng.f32_range(bounds.x0 + 1.0, (bounds.x1 - 1.0).max(bounds.x0 + 1.5));
        let py = b.rng.f32_range(bounds.y0 + 1.0, (bounds.y1 - 1.0).max(bounds.y0 + 1.5));
        let kind = if include_garden && i % 2 == 1 { SurfaceFeatureKind::GardenBedZone } else { SurfaceFeatureKind::MulchZone };
        let rx = b.rng.f32_range(1.0, 2.2);
        let ry = b.rng.f32_range(0.85, 1.85);
        b.try_add_organic_patch(kind, (px, py), (rx, ry), 0.35, 36);
    }
}

fn shift_shoal_pts(pts: &[(f32, f32)], amp_m: f32) -> Vec<(f32, f32)> {
    pts.iter()
        .enumerate()
        .map(|(i, &(x, y))| (x + if i.is_multiple_of(2) { amp_m } else { -amp_m }, y))
        .collect()
}

/// Winding `WoodlandCreek`: incised ravine gorge, cobble shoal bars, live creek(s), rim boulders & footbridge(s).
pub(crate) fn build_woodland_creek(b: &mut OutdoorBuilder) {
    let (wf, hf) = (b.w as f32, b.h as f32);
    let topo = if b.w < 24 || b.h < 18 {
        CreekTopology::NorthSouthMeander
    } else {
        match b.rng.range(0, 3) {
            0 => CreekTopology::NorthSouthMeander,
            1 => CreekTopology::BraidedConfluence,
            _ => CreekTopology::DiagonalSweepRavine,
        }
    };

    let creek_w_cm = (255 + (b.w - 20).max(0) * 3).clamp(255, 350);
    let shoal_w_cm = creek_w_cm + 135;
    let ravine_w_cm = creek_w_cm + (260 + (b.w - 20).max(0) * 4).clamp(260, 380);
    let bridge_span_m = (ravine_w_cm as f32 / 100.0 + 0.55).clamp(4.8, 7.6);

    let primary_creek = match topo {
        CreekTopology::NorthSouthMeander => {
            let cx = wf * b.rng.f32_range(0.42, 0.58);
            let amp = (wf * 0.14).clamp(1.8, 5.2);
            let pts = [(cx - amp * 0.35, -0.4), (cx + amp * 0.75, hf * 0.28), (cx - amp * 0.85, hf * 0.62), (cx + amp * 0.30, hf + 0.4)];
            b.add_ribbon(SurfaceFeatureKind::DefensiveTrench, &pts, ravine_w_cm);
            let shoal_pts = shift_shoal_pts(&pts, 0.45);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &shoal_pts, shoal_w_cm);
            let creek = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &pts, creek_w_cm);
            if b.h >= 24 {
                let flank_x = if b.rng.chance(1, 2) { -0.4 } else { wf + 0.4 };
                b.add_ribbon(
                    SurfaceFeatureKind::DrainageDitch,
                    &[(flank_x, hf * 0.40), (cx - amp * 1.05, hf * 0.52), (cx - amp * 0.85, hf * 0.62)],
                    195,
                );
            }
            let ty0 = hf * b.rng.f32_range(0.28, 0.44);
            let ty1 = hf * b.rng.f32_range(0.55, 0.74);
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(-0.3, ty0), (wf * 0.32, hf * 0.48), (wf * 0.68, hf * 0.56), (wf + 0.3, ty1)], 185);
            b.add_bridge_deck_across(creek, 0.48, 2.3, bridge_span_m);
            if b.h >= 24 {
                b.add_bridge_deck_across(creek, 0.80, 2.0, bridge_span_m * 0.85);
            }
            creek
        }
        CreekTopology::BraidedConfluence => {
            let jx = wf * b.rng.f32_range(0.45, 0.58);
            let jy = hf * b.rng.f32_range(0.46, 0.62);
            let main_pts = [(wf * 0.34, -0.4), (wf * 0.46, jy * 0.45), (jx, jy), (jx - wf * 0.08, hf * 0.82), (jx + wf * 0.06, hf + 0.4)];
            let trib_pts = [(wf * 0.82, -0.4), (wf * 0.72, jy * 0.52), (jx, jy)];
            let trib_ravine_cm = (ravine_w_cm * 3) / 4;
            let trib_creek_cm = (creek_w_cm * 3) / 4;
            b.add_ribbon(SurfaceFeatureKind::DefensiveTrench, &main_pts, ravine_w_cm);
            b.add_ribbon(SurfaceFeatureKind::DefensiveTrench, &trib_pts, trib_ravine_cm);
            let main_shoal = shift_shoal_pts(&main_pts, 0.45);
            let trib_shoal = shift_shoal_pts(&trib_pts, 0.35);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &main_shoal, shoal_w_cm);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &trib_shoal, creek_w_cm + 80);
            let main_idx = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &main_pts, creek_w_cm);
            let trib_idx = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &trib_pts, trib_creek_cm);
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(-0.3, hf * 0.62), (wf * 0.36, hf * 0.58), (wf * 0.68, hf * 0.36), (wf + 0.3, hf * 0.32)], 190);
            b.add_bridge_deck_across(main_idx, 0.54, 2.3, bridge_span_m);
            b.add_bridge_deck_across(trib_idx, 0.45, 2.0, bridge_span_m * 0.8);
            main_idx
        }
        CreekTopology::DiagonalSweepRavine => {
            let flip = b.rng.chance(1, 2);
            let x_a = if flip { wf * 0.78 } else { wf * 0.22 };
            let x_b = if flip { wf * 0.22 } else { wf * 0.78 };
            let pts = [(x_a, -0.4), (x_a * 0.65 + x_b * 0.35 + wf * 0.08, hf * 0.34), (x_a * 0.32 + x_b * 0.68 - wf * 0.08, hf * 0.68), (x_b, hf + 0.4)];
            b.add_ribbon(SurfaceFeatureKind::DefensiveTrench, &pts, ravine_w_cm);
            let gully_x0 = if flip { -0.4 } else { wf + 0.4 };
            b.add_ribbon(
                SurfaceFeatureKind::DrainageDitch,
                &[(gully_x0, hf * 0.16), (pts[1].0 * 0.6 + gully_x0.clamp(0.0, wf) * 0.4, hf * 0.25), pts[1]],
                200,
            );
            let shoal_pts = shift_shoal_pts(&pts, 0.48);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &shoal_pts, shoal_w_cm);
            let creek = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &pts, creek_w_cm);
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(-0.3, hf * if flip { 0.34 } else { 0.66 }), (wf * 0.48, hf * 0.5), (wf + 0.3, hf * if flip { 0.66 } else { 0.34 })], 190);
            b.add_bridge_deck_across(creek, 0.5, 2.3, bridge_span_m);
            creek
        }
    };

    let rim_offset = ravine_w_cm as f32 / 200.0 + 0.95;
    let rim_boulders = (b.h / 5).clamp(4, 10) as usize;
    for i in 0..rim_boulders {
        let frac = 0.10 + 0.80 * ((i as f32 + 0.35) / rim_boulders as f32);
        let side = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
        b.place_along_ribbon(PropKind::Boulder, primary_creek, frac, rim_offset * side, 35, false);
    }
    let rim_logs = (b.h / 7).clamp(2, 6) as usize;
    for i in 0..rim_logs {
        let frac = 0.16 + 0.68 * ((i as f32 + 0.5) / rim_logs as f32);
        let side = if i.is_multiple_of(2) { -1.0 } else { 1.0 };
        b.place_along_ribbon(PropKind::FallenLog, primary_creek, frac, rim_offset * side, 22, false);
    }

    let area_scale = (b.w * b.h / 80).clamp(3, 18);
    scatter_organic_beds(b, (area_scale / 2 + 2).clamp(3, 10), full_rect(b, 1.4), true);
    b.scatter_trees(PropKind::LargeTree, area_scale + 2, 3.2, full_rect(b, 1.5));
    b.scatter_trees(PropKind::MediumTree, area_scale + 4, 2.4, full_rect(b, 1.3));
    b.scatter_turf_prop_rot(PropKind::Boulder, area_scale, 30, full_rect(b, 1.4));
    b.scatter_turf_prop_rot(PropKind::Bush, area_scale + 4, 30, full_rect(b, 1.2));
}

/// Sweeping `RoadsideDitch`: curved `AsphaltRoad`, parallel gravel berm, drainage ditch, culvert bridge & cover.
pub(crate) fn build_roadside_ditch(b: &mut OutdoorBuilder) {
    let (wf, hf) = (b.w as f32, b.h as f32);
    let topo = if b.w < 24 || b.h < 18 {
        RoadsideTopology::EastWestCountryCurve
    } else {
        match b.rng.range(0, 3) {
            0 => RoadsideTopology::EastWestCountryCurve,
            1 => RoadsideTopology::NorthSouthRuralLane,
            _ => RoadsideTopology::SweepingBendWithPullout,
        }
    };

    let (road_idx, ditch_idx) = match topo {
        RoadsideTopology::EastWestCountryCurve => {
            let north_side = b.rng.chance(1, 2);
            let ry = if north_side { hf * 0.28 } else { hf * 0.72 };
            let dy = if north_side { ry + 3.0 } else { ry - 3.0 };
            let bend = b.rng.f32_range(-0.9, 0.9);
            let road_pts = [(-0.4, ry + bend * 0.4), (wf * 0.34, ry + bend), (wf * 0.68, ry - bend), (wf + 0.4, ry - bend * 0.3)];
            let ditch_pts = [(-0.4, dy + bend * 0.35), (wf * 0.35, dy + bend * 0.9), (wf * 0.67, dy - bend * 0.9), (wf + 0.4, dy - bend * 0.25)];
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &road_pts, 460);
            let road = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &road_pts, 350);
            let ditch = b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &ditch_pts, 200);
            let cross_x = wf * b.rng.f32_range(0.40, 0.62);
            let spur_y1 = if north_side { hf - 0.5 } else { 0.5 };
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(cross_x, ry), (cross_x + 0.6, dy), (cross_x + 1.8, spur_y1)], 185);
            b.add_bridge_deck_across(ditch, 0.5, 2.3, 2.7);
            (road, ditch)
        }
        RoadsideTopology::NorthSouthRuralLane => {
            let west_side = b.rng.chance(1, 2);
            let rx = if west_side { wf * 0.32 } else { wf * 0.68 };
            let dx = if west_side { rx + 3.3 } else { rx - 3.3 };
            let sway = b.rng.f32_range(-1.4, 1.4);
            let road_pts = [(rx, -0.4), (rx + sway, hf * 0.34), (rx - sway, hf * 0.68), (rx + sway * 0.3, hf + 0.4)];
            let ditch_pts = [(dx, -0.4), (dx + sway * 0.9, hf * 0.34), (dx - sway * 0.9, hf * 0.68), (dx + sway * 0.3, hf + 0.4)];
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &road_pts, 480);
            let road = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &road_pts, 360);
            let ditch = b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &ditch_pts, 210);
            let cross_y = hf * b.rng.f32_range(0.42, 0.58);
            let edge_x = if west_side { wf - 0.4 } else { 0.4 };
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(rx, cross_y), (dx, cross_y + 0.5), (edge_x, cross_y + 2.0)], 190);
            b.add_bridge_deck_across(ditch, 0.5, 2.4, 2.8);
            (road, ditch)
        }
        RoadsideTopology::SweepingBendWithPullout => {
            let flip = b.rng.chance(1, 2);
            let (p0, p1, p2, p3) = if flip {
                ((-0.4, hf * 0.25), (wf * 0.38, hf * 0.28), (wf * 0.68, hf * 0.54), (wf * 0.78, hf + 0.4))
            } else {
                ((-0.4, hf * 0.75), (wf * 0.38, hf * 0.72), (wf * 0.68, hf * 0.44), (wf * 0.78, -0.4))
            };
            let road_pts = [p0, p1, p2, p3];
            let off_y = if flip { 3.2 } else { -3.2 };
            let ditch_pts = [(p0.0, p0.1 + off_y), (p1.0 - 1.2, p1.1 + off_y), (p2.0 - 2.2, p2.1 + off_y * 0.7), (p3.0 - 3.2, p3.1)];
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &road_pts, 520);
            b.add_poly(SurfaceFeatureKind::GravelRoad, &[(p1.0 - 2.6, p1.1 - off_y * 0.55), (p1.0 + 3.0, p1.1 - off_y * 0.55), (p1.0 + 2.4, p1.1 - off_y * 1.45), (p1.0 - 2.0, p1.1 - off_y * 1.45)], 90);
            let road = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &road_pts, 380);
            let ditch = b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &ditch_pts, 210);
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(p2.0, p2.1), (p2.0 - 2.5, p2.1 + off_y), (wf * 0.18, hf * 0.5)], 185);
            b.add_bridge_deck_across(ditch, 0.52, 2.3, 2.8);
            (road, ditch)
        }
    };

    b.place_vehicle_along_ribbon(PropKind::Car, road_idx, 0.30, 0.15, 7);
    b.place_vehicle_along_ribbon(PropKind::UtilityVan, road_idx, 0.72, -0.2, 8);
    b.place_along_ribbon(PropKind::JerseyBarrier, road_idx, 0.48, -2.05, 4, true);
    b.place_along_ribbon(PropKind::FallenLog, ditch_idx, 0.25, 1.4, 22, false);

    let area_scale = (b.w * b.h / 90).clamp(3, 16);
    scatter_organic_beds(b, (area_scale / 2 + 2).clamp(3, 9), full_rect(b, 1.4), true);
    b.scatter_trees(PropKind::MediumTree, area_scale + 2, 2.6, full_rect(b, 1.4));
    b.scatter_trees(PropKind::LargeTree, area_scale / 2 + 1, 3.6, full_rect(b, 1.6));
    b.scatter_turf_prop_rot(PropKind::WoodCrate, 3, 25, full_rect(b, 2.0));
    b.scatter_turf_prop_rot(PropKind::Bush, area_scale + 3, 35, full_rect(b, 1.2));
}

/// Winding `WoodedPark`: bordering road, meandering & branching trails, stone plaza, benches & canopy.
pub(crate) fn build_wooded_park(b: &mut OutdoorBuilder) {
    let (wf, hf) = (b.w as f32, b.h as f32);
    let topo = if b.w < 24 || b.h < 18 {
        ParkTopology::WindingCrossTrail
    } else {
        match b.rng.range(0, 3) {
            0 => ParkTopology::WindingCrossTrail,
            1 => ParkTopology::ForkedGlade,
            _ => ParkTopology::LoopTrailWithClearing,
        }
    };

    let road = match topo {
        ParkTopology::WindingCrossTrail => {
            let south_road = b.w < 24 || b.rng.chance(1, 2);
            let ry = if south_road { hf - 1.75 } else { 1.75 };
            let r_idx = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &[(-0.4, ry), (wf * 0.35, ry - 0.25), (wf * 0.7, ry + 0.2), (wf + 0.4, ry)], 330);
            let trail_mid_y = hf * 0.46;
            let ty_l = trail_mid_y + b.rng.f32_range(-1.2, 1.2);
            let ty_r = trail_mid_y + b.rng.f32_range(-1.0, 1.0);
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(-0.3, ty_l), (wf * 0.32, trail_mid_y - 1.6), (wf * 0.68, trail_mid_y + 1.4), (wf + 0.3, ty_r)], 195);
            b.add_ribbon(SurfaceFeatureKind::StoneWalk, &[(wf * 0.48, ry), (wf * 0.50, (ry + trail_mid_y) * 0.5), (wf * 0.50, trail_mid_y)], 180);
            b.add_poly(SurfaceFeatureKind::StoneWalk, &[(wf * 0.42, trail_mid_y - 1.5), (wf * 0.58, trail_mid_y - 1.5), (wf * 0.60, trail_mid_y + 1.5), (wf * 0.40, trail_mid_y + 1.5)], 110);
            r_idx
        }
        ParkTopology::ForkedGlade => {
            let r_idx = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &[(-0.4, 1.85), (wf * 0.45, 2.15), (wf + 0.4, 1.70)], 340);
            let (fork_x, fork_y) = (wf * 0.44, hf * 0.54);
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(-0.3, hf * 0.52), (wf * 0.22, hf * 0.58), (fork_x, fork_y), (wf + 0.3, hf * 0.34)], 200);
            b.add_ribbon(SurfaceFeatureKind::StoneWalk, &[(wf * 0.46, 1.85), (fork_x, fork_y), (wf * 0.64, hf * 0.76), (wf + 0.3, hf * 0.78)], 185);
            b.add_poly(SurfaceFeatureKind::StoneWalk, &[(fork_x - 2.3, fork_y - 2.0), (fork_x + 2.3, fork_y - 2.0), (fork_x + 2.3, fork_y + 2.0), (fork_x - 2.3, fork_y + 2.0)], 130);
            r_idx
        }
        ParkTopology::LoopTrailWithClearing => {
            let ry = hf - 1.85;
            let r_idx = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &[(-0.4, ry), (wf * 0.38, ry - 0.2), (wf + 0.4, ry + 0.15)], 340);
            let (cx, cy) = (wf * 0.5, hf * 0.44);
            let rx = (wf * 0.26).clamp(4.2, 9.0);
            let ry_l = (hf * 0.25).clamp(3.2, 7.0);
            b.add_ribbon(SurfaceFeatureKind::StoneWalk, &[(cx, ry), (cx, cy + ry_l)], 190);
            b.add_ribbon(SurfaceFeatureKind::StoneWalk, &[(cx, cy - ry_l), (cx + rx, cy), (cx, cy + ry_l), (cx - rx, cy), (cx, cy - ry_l)], 180);
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(-0.3, cy), (cx - rx, cy)], 185);
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &[(cx + rx, cy), (wf + 0.3, cy)], 185);
            b.add_poly(SurfaceFeatureKind::GardenBedZone, &[(cx - 2.0, cy - 1.5), (cx + 2.0, cy - 1.5), (cx + 2.0, cy + 1.5), (cx - 2.0, cy + 1.5)], 115);
            r_idx
        }
    };

    b.place_vehicle_along_ribbon(PropKind::Car, road, 0.26, 0.15, 6);
    place_rot_m(b, PropKind::PicnicTable, wf * 0.50, hf * 0.47, 0, 90, true);
    place_rot_m(b, PropKind::ParkBench, wf * 0.43, hf * 0.46, -25, 26, true);

    let area_scale = (b.w * b.h / 85).clamp(3, 18);
    scatter_organic_beds(b, (area_scale / 2 + 2).clamp(3, 10), full_rect(b, 1.4), true);
    b.scatter_trees(PropKind::LargeTree, area_scale + 2, 3.3, full_rect(b, 1.5));
    b.scatter_trees(PropKind::MediumTree, area_scale + 4, 2.4, full_rect(b, 1.3));
    b.scatter_turf_prop_rot(PropKind::ParkBench, (area_scale / 2).clamp(2, 6), 30, full_rect(b, 2.0));
    b.scatter_turf_prop_rot(PropKind::Boulder, area_scale / 2 + 1, 30, full_rect(b, 1.6));
    b.scatter_turf_prop_rot(PropKind::Bush, area_scale + 4, 30, full_rect(b, 1.2));
}
