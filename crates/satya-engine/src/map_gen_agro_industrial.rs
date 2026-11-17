//! # Agricultural, Botanical, Manufacturing & Warehouse Sector Generators (`map_gen_agro_industrial`)
//!
//! Procedural vector generators for four specialized outdoor and industrial presets:
//! - [`SectorPreset::Farms`]: contoured crop fields ([`PropKind::CropRow`]), irrigation ditch &
//!   culvert [`SurfaceFeatureKind::BridgeDeck`], tractor [`SurfaceFeatureKind::DirtTrail`], farmhouse
//!   (reusing [`crate::map_gen_residential::generate_home_in_lot`]), barn & equipment yard.
//! - [`SectorPreset::BotanicalGarden`]: glazed conservatory ([`MaterialClass::Glass`], [`PropKind::PlanterTable`]),
//!   winding [`SurfaceFeatureKind::StoneWalk`] & [`SurfaceFeatureKind::DirtTrail`], organic flora beds,
//!   meandering [`SurfaceFeatureKind::CreekChannel`] stream, and footbridges.
//! - [`SectorPreset::ManufacturingPlant`]: assembly [`PropKind::ConveyorBelt`] runs,
//!   [`PropKind::IndustrialPress`] cells, [`PropKind::ElevatedCatwalk`], [`PropKind::HazardSpill`],
//!   glazed supervisor office, and exterior freight apron.
//! - [`SectorPreset::Warehouse`]: high-bay [`PropKind::PalletRack`] aisles, [`PropKind::Forklift`]
//!   lanes, [`PropKind::ShippingContainer`] dock bays, dispatch office, and sorting conveyor spur.

use satya_schema::{
    EconomicTier, EdgeSide, MapFloorLevel, MaterialClass, PropKind, SurfaceFeatureKind,
    WallEdgeKind,
};

use crate::map_gen_residential::{
    CardinalEntry, LotBox, generate_home_in_lot, trim_floor_props_to_connected,
};
use crate::map_outdoor_gen::{OutdoorBuilder, RectM};
use crate::map_outdoor_natural::{
    CurveProfile, CurveSpec, route_trail_across_bridge_m, synthesize_natural_curve_m,
};
use crate::map_surface_queries::prune_occluded_surfaces;
use crate::tactical_map::{ProceduralSector, SectorPreset};

fn safe_x_span(w: i32) -> (i32, i32) {
    let pad = if w >= 16 {
        4
    } else if w >= 12 {
        3
    } else {
        2
    };
    (pad, (w - pad).max(pad + 4).min(w - 1))
}

fn enclose_hall(
    b: &mut OutdoorBuilder,
    lot: LotBox,
    wall_mat: MaterialClass,
    win_step: i32,
    glass_windows: bool,
) {
    for x in lot.x0..lot.x1 {
        let is_pier = x == lot.x0 || x + 1 == lot.x1 || (x - lot.x0) % 3 == 0;
        let win = if glass_windows {
            !is_pier
        } else {
            win_step > 0 && (x - lot.x0) % win_step == 1 && x + 1 < lot.x1
        };
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        b.put_edge(x, lot.y0, EdgeSide::North, k, m);
        b.put_edge(x, lot.y1, EdgeSide::North, k, m);
    }
    for y in lot.y0..lot.y1 {
        let is_pier = y == lot.y0 || y + 1 == lot.y1 || (y - lot.y0) % 3 == 0;
        let win = if glass_windows {
            !is_pier
        } else {
            win_step > 0 && (y - lot.y0) % win_step == 1 && y + 1 < lot.y1
        };
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        b.put_edge(lot.x0, y, EdgeSide::West, k, m);
        b.put_edge(lot.x1, y, EdgeSide::West, k, m);
    }
}

fn carve_door_band_h(b: &mut OutdoorBuilder, y: i32, x0: i32, x1: i32, mat: MaterialClass) {
    for x in x0..x1 {
        b.put_edge(x, y, EdgeSide::North, WallEdgeKind::DoorOpen, mat);
    }
}

fn carve_door_band_v(b: &mut OutdoorBuilder, x: i32, y0: i32, y1: i32, mat: MaterialClass) {
    for y in y0..y1 {
        b.put_edge(x, y, EdgeSide::West, WallEdgeKind::DoorOpen, mat);
    }
}

/// Builds an interior glazed office box tucked inside `(ox0..ox1, oy0..oy1)` with a guaranteed doorway.
fn build_interior_office(b: &mut OutdoorBuilder, office: LotBox, on_west: bool, seed: u32) {
    if on_west {
        for x in office.x0..office.x1 {
            let is_door = x == office.x0 + office.w() / 2;
            let (k, m) = if is_door {
                (WallEdgeKind::DoorOpen, MaterialClass::SheetSteel)
            } else {
                (WallEdgeKind::Window, MaterialClass::Glass)
            };
            b.put_edge(x, office.y1, EdgeSide::North, k, m);
        }
        for y in office.y0..office.y1 {
            let is_door = y == office.y0 + 1;
            let (k, m) = if is_door {
                (WallEdgeKind::DoorOpen, MaterialClass::SheetSteel)
            } else {
                (WallEdgeKind::Window, MaterialClass::Glass)
            };
            b.put_edge(office.x1, y, EdgeSide::West, k, m);
        }
    }
    let desk_x = OutdoorBuilder::cm(office.x0 as f32 + 1.1);
    let desk_y = OutdoorBuilder::cm(office.y0 as f32 + 0.7);
    let _ = b.try_place_cm(PropKind::Desk, desk_x, desk_y, 0, None, true);
    if office.w() >= 6 {
        let _ = b.try_place_cm(
            PropKind::SteelShelving,
            OutdoorBuilder::cm(office.x1 as f32 - 1.2),
            OutdoorBuilder::cm(office.y0 as f32 + 0.6),
            0,
            None,
            true,
        );
    }
    let _ = seed;
}

fn generate_farms(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let (bx0, bx1) = safe_x_span(w);
    let (wf, hf) = (w as f32, h as f32);
    let lane_y = (hf * 0.66).min(hf - 4.0);
    let ditch_y = (hf - 1.85).max(lane_y + 2.2);

    // Tractor DirtTrail lane across South-mid & meandering DrainageDitch culvert below it.
    let trail_rect = RectM {
        x0: 1.0,
        y0: lane_y - 0.55,
        x1: wf - 1.0,
        y1: (ditch_y - 1.40).max(lane_y + 0.25),
    };
    let trail_y0 = lane_y + b.rng.f32_range(-0.35, 0.35);
    let trail_y1 = lane_y + b.rng.f32_range(-0.35, 0.35);
    let trail_ctrl = synthesize_natural_curve_m(
        &mut b.rng,
        (-0.5, trail_y0),
        (wf + 0.5, trail_y1),
        CurveSpec {
            profile: CurveProfile::ForestTrail,
            max_lateral_m: 0.75,
            ribbon_half_w_m: 1.0,
            clamp_rect: trail_rect,
        },
    );
    b.add_ribbon(SurfaceFeatureKind::DirtTrail, &trail_ctrl, 200);

    let ditch_rect = RectM {
        x0: 1.0,
        y0: (lane_y + 1.40).min(ditch_y - 0.35),
        x1: wf - 1.0,
        y1: (hf - 1.80).max(ditch_y + 0.15),
    };
    let ditch_y0 = ditch_y + b.rng.f32_range(-0.20, 0.20);
    let ditch_y1 = ditch_y + b.rng.f32_range(-0.20, 0.20);
    let ditch_ctrl = synthesize_natural_curve_m(
        &mut b.rng,
        (-0.5, ditch_y0),
        (wf + 0.5, ditch_y1),
        CurveSpec {
            profile: CurveProfile::DrainageSwale,
            max_lateral_m: 0.65,
            ribbon_half_w_m: 0.65,
            clamp_rect: ditch_rect,
        },
    );
    let ditch_idx = b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &ditch_ctrl, 130);
    let br_frac = b.rng.f32_range(0.35, 0.68);
    if let Some(bridge) = b.place_perpendicular_bridge(ditch_idx, br_frac, 2.5, 2.4) {
        let spur_start = (bridge.center_m.0, lane_y);
        let dot_in = (spur_start.0 - bridge.center_m.0) * bridge.span_dir.0
            + (spur_start.1 - bridge.center_m.1) * bridge.span_dir.1;
        let out_sign = if dot_in <= 0.0 { 1.0 } else { -1.0 };
        let spur_end = (
            (bridge.center_m.0 + bridge.span_dir.0 * out_sign * (bridge.half_span_m + 0.45))
                .clamp(1.2, wf - 1.2),
            (bridge.center_m.1 + bridge.span_dir.1 * out_sign * (bridge.half_span_m + 0.45))
                .clamp(1.2, hf - 0.35),
        );
        let spur_pts = route_trail_across_bridge_m(
            &mut b.rng,
            spur_start,
            bridge,
            spur_end,
            RectM {
                x0: 0.8,
                y0: lane_y - 0.5,
                x1: wf - 0.8,
                y1: hf - 0.35,
            },
            Some((&ditch_ctrl, 1.1)),
        );
        b.add_ribbon(SurfaceFeatureKind::DirtTrail, &spur_pts, 180);
    }

    // North-West Farmhouse + North-East Timber Barn.
    let bldg_y0 = 1;
    let bldg_y1 = (h * 38 / 100).clamp(5, (lane_y.floor() as i32 - 3).max(5));
    let bw = bx1 - bx0;
    let farm_w = ((bw * 44) / 100).clamp(5, 16);
    let barn_w = ((bw * 38) / 100).clamp(4, 14);
    let farm_lot = LotBox::new(bx0, bldg_y0, bx0 + farm_w, bldg_y1);
    let barn_lot = LotBox::new((bx1 - barn_w).max(farm_lot.x1 + 2), bldg_y0, bx1, bldg_y1);

    let tier = if seed % 2 == 0 {
        EconomicTier::MiddleClass
    } else {
        EconomicTier::LowIncome
    };
    let home = generate_home_in_lot(
        w,
        h,
        farm_lot,
        tier,
        CardinalEntry::South,
        true,
        false,
        true,
        &b.spawn_pads_m,
        seed as u64,
    );
    for e in home.ground.wall_edges {
        b.put_edge(e.x, e.y, e.side, e.kind, e.material);
    }
    b.props.extend(home.ground.props);
    b.surfaces.extend(home.ground.surfaces);

    // Timber Barn InteriorFloor + solid timber walls + wide barn doors.
    let barn_seed = (b.rng.next() & 0xffff_ffff) as u32;
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[
            (barn_lot.x0 as f32, barn_lot.y0 as f32),
            (barn_lot.x1 as f32, barn_lot.y0 as f32),
            (barn_lot.x1 as f32, barn_lot.y1 as f32),
            (barn_lot.x0 as f32, barn_lot.y1 as f32),
        ],
        0,
        barn_seed,
    );
    enclose_hall(&mut b, barn_lot, MaterialClass::SolidWood, 0, false);
    let barn_mid_x = barn_lot.x0 + barn_lot.w() / 2;
    carve_door_band_h(
        &mut b,
        barn_lot.y1,
        barn_mid_x - 1,
        (barn_mid_x + 1).min(barn_lot.x1),
        MaterialClass::SolidWood,
    );
    carve_door_band_h(
        &mut b,
        barn_lot.y0,
        barn_mid_x,
        (barn_mid_x + 1).min(barn_lot.x1),
        MaterialClass::SolidWood,
    );
    let _ = b.try_place_cm(
        PropKind::PalletRack,
        OutdoorBuilder::cm(barn_lot.x0 as f32 + 1.6),
        OutdoorBuilder::cm(barn_lot.y0 as f32 + 0.6),
        0,
        None,
        true,
    );
    let _ = b.try_place_cm(
        PropKind::WoodCrate,
        OutdoorBuilder::cm(barn_lot.x1 as f32 - 0.7),
        OutdoorBuilder::cm(barn_lot.y0 as f32 + 0.7),
        18,
        None,
        true,
    );
    let _ = b.try_place_cm(
        PropKind::DrumCluster,
        OutdoorBuilder::cm(barn_lot.x1 as f32 - 0.7),
        OutdoorBuilder::cm(barn_lot.y1 as f32 - 0.7),
        32,
        None,
        true,
    );

    // Gated fenced pasture corral + GravelRoad equipment pad between barn and tractor lane.
    let gx0 = (barn_lot.x0 as f32 + 0.4).max(farm_lot.x1 as f32 + 1.0);
    let gx1 = bx1 as f32 - 0.4;
    let gy0 = bldg_y1 as f32 + 0.35;
    let gy1 = (lane_y - 1.15).max(gy0 + 1.2);
    if gx1 - gx0 >= 2.5 && gy1 - gy0 >= 1.1 {
        let gseed = (b.rng.next() & 0xffff_ffff) as u32;
        b.add_polygon_surface(
            SurfaceFeatureKind::GravelRoad,
            &[(gx0, gy0), (gx1, gy0), (gx1, gy1), (gx0, gy1)],
            20,
            gseed,
        );
        let _ = b.try_place_cm(
            PropKind::Forklift,
            OutdoorBuilder::cm(gx0 + 1.3),
            OutdoorBuilder::cm((gy0 + gy1) * 0.5),
            24,
            None,
            true,
        );
    }

    // Disjoint contoured tilled CropRow field zone strictly between buildings and tractor lane on West half.
    let field_x0 = (bx0 as f32).max(2.0);
    let field_x1 = (gx0 - 0.8).max(field_x0 + 4.5).min(w as f32 - 5.0);
    let field_y0 = bldg_y1 as f32 + 0.55;
    let field_y1 = (lane_y - 1.15).max(field_y0 + 1.4);
    let row_angle = b.rng.range(12, 29);
    if field_x1 - field_x0 >= 3.5 && field_y1 - field_y0 >= 1.3 {
        let mseed = (b.rng.next() & 0xffff_ffff) as u32;
        b.add_polygon_surface(
            SurfaceFeatureKind::MulchZone,
            &[
                (field_x0, field_y0),
                (field_x1, field_y0 + 0.25),
                (field_x1 - 0.2, field_y1),
                (field_x0 + 0.25, field_y1 - 0.2),
            ],
            35,
            mseed,
        );
        let mut rx = field_x0 + 1.6;
        while rx + 1.2 <= field_x1 {
            let cy_f = (field_y0 + field_y1) * 0.5;
            let _ = b.try_place_cm(
                PropKind::CropRow,
                OutdoorBuilder::cm(rx),
                OutdoorBuilder::cm(cy_f),
                row_angle,
                Some(280),
                true,
            );
            rx += 3.6;
        }
    }
    // Exposed decorative GardenBedZone near farmhouse porch / orchard edge + orchard trees & corral fence.
    let _ = b.try_add_organic_patch(
        SurfaceFeatureKind::GardenBedZone,
        (1.8, 2.2),
        (1.1, 1.0),
        0.30,
        24,
    );
    let _ = b.try_add_organic_patch(
        SurfaceFeatureKind::MulchZone,
        (w as f32 - 2.0, 2.4),
        (1.2, 1.0),
        0.30,
        0,
    );
    let _ = b.try_place_cm(
        PropKind::UtilityVan,
        OutdoorBuilder::cm(w as f32 * 0.25),
        OutdoorBuilder::cm(lane_y),
        94,
        None,
        true,
    );
    b.scatter_trees(
        PropKind::LargeTree,
        3,
        3.2,
        RectM {
            x0: 1.0,
            y0: 1.0,
            x1: bx0 as f32 - 0.5,
            y1: lane_y - 1.5,
        },
    );
    b.scatter_trees(
        PropKind::MediumTree,
        4,
        2.8,
        RectM {
            x0: bx1 as f32 + 0.5,
            y0: 1.0,
            x1: w as f32 - 1.0,
            y1: ditch_y - 1.2,
        },
    );
    b.scatter_turf_prop_rot(
        PropKind::Bush,
        5,
        25,
        RectM {
            x0: 0.8,
            y0: 0.8,
            x1: w as f32 - 0.8,
            y1: ditch_y - 1.0,
        },
    );

    let mut sec = b.finalize_connected();
    if let Some(mut f2) = home.upper {
        trim_floor_props_to_connected(w, h, farm_lot, &f2.wall_edges, &mut f2.props);
        prune_occluded_surfaces(&mut f2.surfaces, w, h);
        sec.floors = vec![
            MapFloorLevel {
                floor_index: 0,
                label: "1F · Farmstead Grounds".to_string(),
                elevation_cm: 0,
                wall_edges: sec.wall_edges.clone(),
                props: sec.props.clone(),
                surfaces: sec.surfaces.clone(),
                ..Default::default()
            },
            MapFloorLevel {
                floor_index: 1,
                label: "2F · Farmhouse Upper Level".to_string(),
                elevation_cm: 330,
                wall_edges: f2.wall_edges,
                props: f2.props,
                surfaces: f2.surfaces,
                ..Default::default()
            },
        ];
    }
    sec
}

fn generate_botanical_garden(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let (bx0, bx1) = safe_x_span(w);
    let cw = ((bx1 - bx0) * 65 / 100).clamp(7, 22);
    let cx0 = bx0 + ((bx1 - bx0 - cw) / 2).max(0);
    let cx1 = cx0 + cw;
    let cy0 = 1;
    let cy1 = (h * 38 / 100).clamp(5, (h - 7).max(5));
    let cons_lot = LotBox::new(cx0, cy0, cx1, cy1);

    // Glazed Glass Conservatory building in North-center.
    let cseed = (b.rng.next() & 0xffff_ffff) as u32;
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[
            (cx0 as f32, cy0 as f32),
            (cx1 as f32, cy0 as f32),
            (cx1 as f32, cy1 as f32),
            (cx0 as f32, cy1 as f32),
        ],
        0,
        cseed,
    );
    enclose_hall(&mut b, cons_lot, MaterialClass::Brick, 1, true);
    let mid_x = cx0 + cw / 2;
    let mid_y = cy0 + cons_lot.h() / 2;
    carve_door_band_h(
        &mut b,
        cy1,
        mid_x - 1,
        (mid_x + 1).min(cx1),
        MaterialClass::Glass,
    );
    carve_door_band_h(
        &mut b,
        cy0,
        mid_x - 1,
        (mid_x + 1).min(cx1),
        MaterialClass::Glass,
    );
    carve_door_band_v(
        &mut b,
        cx0,
        mid_y,
        (mid_y + 1).min(cy1),
        MaterialClass::Glass,
    );
    carve_door_band_v(
        &mut b,
        cx1,
        mid_y,
        (mid_y + 1).min(cy1),
        MaterialClass::Glass,
    );

    // Conservatory specimen PlanterTables & benches inside glazed hall.
    let mut px = cx0 as f32 + 1.8;
    while px + 1.4 <= cx1 as f32 {
        if (px - (mid_x as f32)).abs() >= 1.6 {
            let _ = b.try_place_cm(
                PropKind::PlanterTable,
                OutdoorBuilder::cm(px),
                OutdoorBuilder::cm(cy0 as f32 + 1.3),
                12,
                None,
                true,
            );
            if cons_lot.h() >= 7 {
                let _ = b.try_place_cm(
                    PropKind::PlanterTable,
                    OutdoorBuilder::cm(px),
                    OutdoorBuilder::cm(cy1 as f32 - 1.3),
                    348,
                    None,
                    true,
                );
            }
        }
        px += 3.2;
    }

    // Meandering ornamental CreekChannel stream across South grounds + BridgeDeck footbridge(s).
    let (wf, hf) = (w as f32, h as f32);
    let stream_y = (hf * 0.78).max(cy1 as f32 + 3.2).min(hf - 1.9);
    let stream_rect = RectM {
        x0: 1.4,
        y0: (cy1 as f32 + 2.4).min(stream_y - 0.5),
        x1: wf - 1.4,
        y1: (hf - 1.9).max(stream_y + 0.3),
    };
    let creek_y0 = stream_y + b.rng.f32_range(-0.4, 0.4);
    let creek_y1 = stream_y + b.rng.f32_range(-0.4, 0.4);
    let creek_ctrl = synthesize_natural_curve_m(
        &mut b.rng,
        (-0.5, creek_y0),
        (wf + 0.5, creek_y1),
        CurveSpec {
            profile: CurveProfile::WildStream,
            max_lateral_m: if h >= 22 { 1.6 } else { 0.85 },
            ribbon_half_w_m: 0.98,
            clamp_rect: stream_rect,
        },
    );
    let creek_idx = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &creek_ctrl, 195);
    let main_frac = b.rng.f32_range(0.42, 0.62);
    let main_bridge = b.place_perpendicular_bridge(creek_idx, main_frac, 2.8, 2.8);
    if w >= 32 {
        let side_frac = b.rng.f32_range(0.18, 0.30);
        let _ = b.place_perpendicular_bridge(creek_idx, side_frac, 2.4, 2.7);
    }

    // Winding StoneWalk promenade linking East/West borders & conservatory, plus meandering DirtTrail spurs.
    let walk_y = ((cy1 as f32 + stream_y) * 0.5 - 0.25).clamp(cy1 as f32 + 1.1, stream_y - 1.4);
    let walk_rect = RectM {
        x0: 1.0,
        y0: cy1 as f32 + 0.9,
        x1: wf - 1.0,
        y1: (stream_y - 1.25).max(cy1 as f32 + 1.3),
    };
    let walk_y0 = (hf * 0.5 + b.rng.f32_range(-0.3, 0.4)).clamp(walk_rect.y0, walk_rect.y1);
    let walk_y1 = (hf * 0.5 + b.rng.f32_range(-0.4, 0.3)).clamp(walk_rect.y0, walk_rect.y1);
    let walk_ctrl = synthesize_natural_curve_m(
        &mut b.rng,
        (-0.5, walk_y0),
        (wf + 0.5, walk_y1),
        CurveSpec {
            profile: CurveProfile::GentleWalk,
            max_lateral_m: 0.9,
            ribbon_half_w_m: 0.78,
            clamp_rect: walk_rect,
        },
    );
    b.add_ribbon(SurfaceFeatureKind::StoneWalk, &walk_ctrl, 155);
    if let Some(br) = main_bridge {
        let end_x = (br.center_m.0 + b.rng.f32_range(-1.6, 1.6)).clamp(1.2, wf - 1.2);
        let garden_trail = route_trail_across_bridge_m(
            &mut b.rng,
            (br.center_m.0, walk_y),
            br,
            (end_x, hf - 0.35),
            RectM {
                x0: 0.8,
                y0: walk_y - 0.4,
                x1: wf - 0.8,
                y1: hf - 0.35,
            },
            Some((&creek_ctrl, 1.35)),
        );
        b.add_ribbon(SurfaceFeatureKind::DirtTrail, &garden_trail, 150);
    }
    b.add_ribbon(
        SurfaceFeatureKind::DirtTrail,
        &[
            (cx1 as f32 + 1.5, 1.5),
            (wf - 2.0, walk_y - 1.0),
            (wf - 2.5, 2.2),
        ],
        115,
    );

    // Organic GardenBedZone and MulchZone flowerbeds strictly clear of water/paths/conservatory.
    for &(gx, gy, rx, ry) in &[
        (2.0_f32, 2.4_f32, 1.3_f32, 1.1_f32),
        (w as f32 - 2.2, 2.5, 1.3, 1.1),
        (w as f32 * 0.30, cy1 as f32 + 1.6, 1.4, 1.0),
        (w as f32 * 0.70, cy1 as f32 + 1.6, 1.4, 1.0),
    ] {
        let _ = b.try_add_organic_patch(
            SurfaceFeatureKind::GardenBedZone,
            (gx, gy),
            (rx, ry),
            0.35,
            32,
        );
    }
    b.scatter_trees(
        PropKind::LargeTree,
        4,
        3.0,
        RectM {
            x0: 1.0,
            y0: 1.0,
            x1: w as f32 - 1.0,
            y1: h as f32 - 1.0,
        },
    );
    b.scatter_trees(
        PropKind::MediumTree,
        6,
        2.4,
        RectM {
            x0: 1.0,
            y0: 1.0,
            x1: w as f32 - 1.0,
            y1: h as f32 - 1.0,
        },
    );
    b.scatter_turf_prop_rot(
        PropKind::PlanterTable,
        3,
        18,
        RectM {
            x0: 1.2,
            y0: 1.2,
            x1: w as f32 - 1.2,
            y1: stream_y - 1.3,
        },
    );
    b.scatter_turf_prop_rot(
        PropKind::Bush,
        6,
        22,
        RectM {
            x0: 0.8,
            y0: 0.8,
            x1: w as f32 - 0.8,
            y1: h as f32 - 0.8,
        },
    );
    b.scatter_turf_prop_rot(
        PropKind::ParkBench,
        3,
        15,
        RectM {
            x0: 1.2,
            y0: cy1 as f32 + 0.6,
            x1: w as f32 - 1.2,
            y1: h as f32 - 1.2,
        },
    );
    b.scatter_turf_prop_rot(
        PropKind::Boulder,
        3,
        25,
        RectM {
            x0: 1.0,
            y0: stream_y - 1.6,
            x1: w as f32 - 1.0,
            y1: h as f32 - 0.8,
        },
    );

    b.finalize_connected()
}

fn build_industrial_hall_shell_and_apron(
    b: &mut OutdoorBuilder,
    w: i32,
    h: i32,
    dock_bays: usize,
) -> (LotBox, usize) {
    let (bx0, bx1) = safe_x_span(w);
    let by0 = 1;
    let by1 = (h - 4).max(by0 + 7);
    let lot = LotBox::new(bx0, by0, bx1, by1);

    // Exterior AsphaltRoad truck apron + ConcreteWalk dock walkway south of building.
    let road_y = h as f32 - 1.35;
    let road_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (-0.5, road_y - 0.1),
            (w as f32 * 0.5, road_y + 0.12),
            (w as f32 + 0.5, road_y - 0.1),
        ],
        220,
    );
    let walk_y = by1 as f32 + 0.65;
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[(bx0 as f32 + 0.6, walk_y), (bx1 as f32 - 0.6, walk_y)],
        110,
    );

    let fseed = (b.rng.next() & 0xffff_ffff) as u32;
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[
            (bx0 as f32, by0 as f32),
            (bx1 as f32, by0 as f32),
            (bx1 as f32, by1 as f32),
            (bx0 as f32, by1 as f32),
        ],
        0,
        fseed,
    );
    enclose_hall(b, lot, MaterialClass::ReinforcedConcrete, 4, false);

    // West & East personnel doors aligned with spawn pads, plus South freight bay doors.
    let mid_y = (h / 2).clamp(by0 + 2, by1 - 2);
    carve_door_band_v(b, bx0, mid_y, mid_y + 1, MaterialClass::SheetSteel);
    carve_door_band_v(b, bx1, mid_y, mid_y + 1, MaterialClass::SheetSteel);
    let bw = lot.w();
    for i in 0..dock_bays {
        let dx = bx0 + ((i as i32 + 1) * bw) / (dock_bays as i32 + 1);
        carve_door_band_h(
            b,
            by1,
            (dx - 1).max(bx0 + 1),
            (dx + 1).min(bx1 - 1),
            MaterialClass::SheetSteel,
        );
    }

    (lot, road_idx)
}

fn generate_manufacturing_plant(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let (lot, road_idx) = build_industrial_hall_shell_and_apron(&mut b, w, h, 2);

    // Glazed Supervisor Office in NW hall corner.
    let off_w = (lot.w() / 4).clamp(4, 8);
    let off_h = (lot.h() / 3).clamp(3, 5);
    let office = LotBox::new(lot.x0, lot.y0, lot.x0 + off_w, lot.y0 + off_h);
    build_interior_office(&mut b, office, true, seed as u32);

    // Supervisor ElevatedCatwalk deck + HazardSpill warning zones on factory slab.
    let cat_x = (office.x1 + 1).min(lot.x1 - 3);
    let _ = b.try_place_cm(
        PropKind::ElevatedCatwalk,
        OutdoorBuilder::cm(cat_x as f32 + 1.5),
        OutdoorBuilder::cm(lot.y0 as f32 + 1.5),
        0,
        Some(((lot.x1 - cat_x - 2).clamp(2, 6)) * 100),
        true,
    );
    let _ = b.try_place_cm(
        PropKind::HazardSpill,
        OutdoorBuilder::cm(lot.x1 as f32 - 2.2),
        OutdoorBuilder::cm(lot.y0 as f32 + 2.5),
        14,
        Some(200),
        true,
    );

    // Parallel assembly ConveyorBelt runs + IndustrialPress machines with >= 1.6m aisle clearance.
    let floor_top = (office.y1 as f32 + 1.2).min(lot.y1 as f32 - 4.0);
    let conv_len_cm = (((lot.w() - 4).clamp(3, 8)) * 100).min(700);
    let mid_x_cm = OutdoorBuilder::cm((lot.x0 + lot.x1) as f32 * 0.5);
    let _ = b.try_place_cm(
        PropKind::ConveyorBelt,
        mid_x_cm,
        OutdoorBuilder::cm(floor_top + 0.6),
        0,
        Some(conv_len_cm),
        true,
    );
    if lot.h() >= 12 {
        let _ = b.try_place_cm(
            PropKind::ConveyorBelt,
            mid_x_cm,
            OutdoorBuilder::cm(lot.y1 as f32 - 2.0),
            0,
            Some(conv_len_cm),
            true,
        );
    }

    let press_y = OutdoorBuilder::cm((floor_top + lot.y1 as f32 - 1.5) * 0.5);
    let _ = b.try_place_cm(
        PropKind::IndustrialPress,
        OutdoorBuilder::cm(lot.x0 as f32 + 2.5),
        press_y,
        0,
        None,
        true,
    );
    if lot.w() >= 16 {
        let _ = b.try_place_cm(
            PropKind::IndustrialPress,
            OutdoorBuilder::cm(lot.x1 as f32 - 2.5),
            press_y,
            180,
            None,
            true,
        );
    }
    let _ = b.try_place_cm(
        PropKind::PalletRack,
        OutdoorBuilder::cm(lot.x1 as f32 - 2.2),
        OutdoorBuilder::cm(lot.y0 as f32 + 0.6),
        0,
        None,
        true,
    );
    let _ = b.try_place_cm(
        PropKind::DrumCluster,
        OutdoorBuilder::cm(office.x1 as f32 + 0.8),
        OutdoorBuilder::cm(lot.y0 as f32 + 0.7),
        22,
        None,
        true,
    );

    // Exterior freight ShippingContainers + Forklift + UtilityVans cleanly off InteriorFloor & AsphaltRoad.
    let _ = b.try_place_cm(
        PropKind::ShippingContainer,
        OutdoorBuilder::cm(1.85),
        OutdoorBuilder::cm(3.6),
        87,
        None,
        true,
    );
    if h >= 24 {
        let _ = b.try_place_cm(
            PropKind::ShippingContainer,
            OutdoorBuilder::cm(w as f32 - 1.85),
            OutdoorBuilder::cm(4.2),
            273,
            None,
            true,
        );
    }
    let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, road_idx, 0.35, 0.0, 4);
    let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, road_idx, 0.72, 0.0, 5);
    let _ = b.try_place_cm(
        PropKind::Forklift,
        OutdoorBuilder::cm(w as f32 * 0.5 + 1.5),
        OutdoorBuilder::cm(lot.y1 as f32 + 0.65),
        14,
        None,
        true,
    );
    b.scatter_turf_prop_rot(
        PropKind::JerseyBarrier,
        2,
        15,
        RectM {
            x0: 1.0,
            y0: lot.y1 as f32 + 0.2,
            x1: w as f32 - 1.0,
            y1: h as f32 - 0.5,
        },
    );

    b.finalize_connected()
}

fn generate_warehouse(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let (lot, road_idx) = build_industrial_hall_shell_and_apron(&mut b, w, h, 3);

    // Dispatch Office in NW warehouse corner.
    let off_w = (lot.w() / 4).clamp(4, 7);
    let off_h = (lot.h() / 3).clamp(3, 5);
    let office = LotBox::new(lot.x0, lot.y0, lot.x0 + off_w, lot.y0 + off_h);
    build_interior_office(&mut b, office, true, seed as u32);

    // Ordered PalletRack rows separated by >= 2.8m forklift aisles leaving 2m perimeter aisles clear.
    let rack_y0 = lot.y0 as f32 + 1.8;
    let rack_y1 = (lot.y1 as f32 - 3.2).max(rack_y0 + 1.0);
    let rack_x0 = (office.x1 as f32 + 2.5).min(lot.x1 as f32 - 3.0);
    let rack_x1 = lot.x1 as f32 - 2.0;
    let mut ry = rack_y0;
    while ry <= rack_y1 {
        let mut rx = rack_x0;
        while rx + 1.8 <= rack_x1 {
            let _ = b.try_place_cm(
                PropKind::PalletRack,
                OutdoorBuilder::cm(rx + 1.0),
                OutdoorBuilder::cm(ry),
                0,
                None,
                true,
            );
            rx += 4.4;
        }
        ry += 3.0;
    }

    // Sorting ConveyorBelt spur + Forklift + WoodCrate staging near South loading docks.
    let conv_cx = OutdoorBuilder::cm(lot.x0 as f32 + 2.5);
    let conv_cy = OutdoorBuilder::cm((lot.y1 as f32 - 1.4).max(office.y1 as f32 + 0.8));
    let _ = b.try_place_cm(PropKind::ConveyorBelt, conv_cx, conv_cy, 0, Some(300), true);
    let _ = b.try_place_cm(
        PropKind::Forklift,
        OutdoorBuilder::cm((lot.x0 + lot.x1) as f32 * 0.5),
        OutdoorBuilder::cm(lot.y1 as f32 - 1.6),
        19,
        None,
        true,
    );
    let _ = b.try_place_cm(
        PropKind::WoodCrate,
        OutdoorBuilder::cm(lot.x0 as f32 + 1.2),
        OutdoorBuilder::cm(lot.y1 as f32 - 2.6),
        27,
        None,
        true,
    );
    let _ = b.try_place_cm(
        PropKind::DrumCluster,
        OutdoorBuilder::cm(lot.x1 as f32 - 1.2),
        OutdoorBuilder::cm(lot.y0 as f32 + 1.2),
        11,
        None,
        true,
    );

    // Exterior ShippingContainers in side freight yards + Freight Van + Forklift on truck apron.
    let _ = b.try_place_cm(
        PropKind::ShippingContainer,
        OutdoorBuilder::cm(1.85),
        OutdoorBuilder::cm(3.8),
        88,
        None,
        true,
    );
    if h >= 24 {
        let _ = b.try_place_cm(
            PropKind::ShippingContainer,
            OutdoorBuilder::cm(1.85),
            OutdoorBuilder::cm(9.2),
            266,
            None,
            true,
        );
    }
    if w >= 28 && h >= 24 {
        let _ = b.try_place_cm(
            PropKind::ShippingContainer,
            OutdoorBuilder::cm(w as f32 - 1.85),
            OutdoorBuilder::cm(4.2),
            274,
            None,
            true,
        );
    }
    let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, road_idx, 0.72, 0.0, 5);
    let _ = b.try_place_cm(
        PropKind::Forklift,
        OutdoorBuilder::cm(w as f32 * 0.55),
        OutdoorBuilder::cm(lot.y1 as f32 + 0.65),
        344,
        None,
        true,
    );

    b.finalize_connected()
}

/// Dispatches Phase 5 agricultural, botanical, manufacturing, and warehouse presets.
#[must_use]
pub fn generate_agro_or_industrial_sector(
    preset: SectorPreset,
    width: i32,
    height: i32,
    seed: i64,
) -> ProceduralSector {
    let w = width.clamp(
        crate::tactical_map::MIN_SECTOR_DIM,
        crate::tactical_map::MAX_SECTOR_DIM,
    );
    let h = height.clamp(
        crate::tactical_map::MIN_SECTOR_DIM,
        crate::tactical_map::MAX_SECTOR_DIM,
    );
    match preset {
        SectorPreset::Farms => generate_farms(w, h, seed),
        SectorPreset::BotanicalGarden => generate_botanical_garden(w, h, seed),
        SectorPreset::ManufacturingPlant => generate_manufacturing_plant(w, h, seed),
        SectorPreset::Warehouse => generate_warehouse(w, h, seed),
        _ => ProceduralSector::default(),
    }
}
