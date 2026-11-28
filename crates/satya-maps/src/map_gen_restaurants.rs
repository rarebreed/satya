//! # Restaurant Sector Generators (`map_gen_restaurants`)
//!
//! Three dedicated food-service tactical presets with proper wall-aligned [`PropKind::Booth`] seating,
//! continuous full-length [`PropKind::BarCounter`] / [`PropKind::FastFoodCounter`] runs, back-of-house
//! commercial kitchens, outdoor [`WallEdgeKind::LowWallStone`] patio walls, periodic [`PropKind::StructuralPillar`]
//! grids on large spans, beverage/snack vending machines, and public Men's / Women's / Family restrooms:
//! - [`SectorPreset::FastFoodRestaurant`]: Single-story (`1F`) quick-service restaurant with a wrapping
//!   U-shaped drive-thru lane, [`PropKind::DriveThruMenuBoard`], [`PropKind::SodaFountain`], and
//!   [`PropKind::DeepFryerStation`].
//! - [`SectorPreset::BistroDiner`]: Casual streetfront café/diner (`1F`, optional `2F` loft when `w >= 26 && h >= 20`)
//!   with stone patio wall seating, diner bar counter + stools, booths, and kitchen.
//! - [`SectorPreset::FineDiningRestaurant`]: Large upscale dining hall (`1F`, plus `2F` Private Dining & Banquet
//!   mezzanine when `w >= 32 && h >= 24`) with courtyard entrance, [`PropKind::HostStand`], cocktail bar,
//!   [`PropKind::BuffetCounter`], full 3-room restrooms, and periodic structural pillars.

use satya_schema::{
    EdgeSide, MapFloorLevel, MapPropPlacement, MapSurfaceFeature, MapWallEdge, MaterialClass,
    PropKind, SurfaceFeatureKind, WallEdgeKind,
};

use crate::map_dressing::{make_prop, prop_spec};
use crate::map_gen_residential::{
    CardinalEntry, LotBox, floor_lot_connected, trim_floor_props_to_connected,
};
use crate::map_outdoor_gen::{OutdoorBuilder, RectM};
use crate::map_restroom_gen::build_public_restroom_suite_in_lot;
use crate::map_room_furnish::place_structural_pillar_grid;
use crate::map_surface_queries::prune_occluded_surfaces;
use crate::tactical_map::{ProceduralSector, SectorPreset};

fn upsert_edge(
    edges: &mut Vec<MapWallEdge>,
    x: i32,
    y: i32,
    side: EdgeSide,
    kind: WallEdgeKind,
    material: MaterialClass,
) {
    let e = MapWallEdge {
        x,
        y,
        side,
        kind,
        material,
    };
    match edges
        .iter_mut()
        .find(|o| o.x == x && o.y == y && o.side == side)
    {
        Some(slot) => *slot = e,
        None => edges.push(e),
    }
}

fn enclose_rect_shell(
    edges: &mut Vec<MapWallEdge>,
    lot: LotBox,
    wall_mat: MaterialClass,
    win_step: i32,
) {
    for x in lot.x0..lot.x1 {
        let win = win_step > 0 && (x - lot.x0) % win_step == 1 && x + 1 < lot.x1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        upsert_edge(edges, x, lot.y0, EdgeSide::North, k, m);
        upsert_edge(edges, x, lot.y1, EdgeSide::North, k, m);
    }
    for y in lot.y0..lot.y1 {
        let win = win_step > 0 && (y - lot.y0) % win_step == 1 && y + 1 < lot.y1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        upsert_edge(edges, lot.x0, y, EdgeSide::West, k, m);
        upsert_edge(edges, lot.x1, y, EdgeSide::West, k, m);
    }
}

fn h_partition(
    edges: &mut Vec<MapWallEdge>,
    y: i32,
    x0: i32,
    x1: i32,
    doors: &[i32],
    mat: MaterialClass,
    glass_partition: bool,
) {
    for x in x0..x1 {
        if doors.contains(&x) {
            upsert_edge(
                edges,
                x,
                y,
                EdgeSide::North,
                WallEdgeKind::DoorOpen,
                MaterialClass::SolidWood,
            );
        } else {
            let (k, m) = if glass_partition && (x - x0) % 2 == 1 {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, mat)
            };
            upsert_edge(edges, x, y, EdgeSide::North, k, m);
        }
    }
}

fn v_partition(
    edges: &mut Vec<MapWallEdge>,
    x: i32,
    y0: i32,
    y1: i32,
    doors: &[i32],
    mat: MaterialClass,
    glass_partition: bool,
) {
    for y in y0..y1 {
        if doors.contains(&y) {
            upsert_edge(
                edges,
                x,
                y,
                EdgeSide::West,
                WallEdgeKind::DoorClosed,
                MaterialClass::SolidWood,
            );
        } else {
            let (k, m) = if glass_partition && (y - y0) % 2 == 1 {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, mat)
            };
            upsert_edge(edges, x, y, EdgeSide::West, k, m);
        }
    }
}

fn place_checked(
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    bay: LotBox,
    cand: MapPropPlacement,
) -> bool {
    if cand.x < bay.x0
        || cand.y < bay.y0
        || cand.x + cand.w_cells > bay.x1
        || cand.y + cand.h_cells > bay.y1
    {
        return false;
    }
    if prop_spec(cand.kind).floor_layer {
        props.push(cand);
        return true;
    }
    let blocks_door = edges.iter().any(|e| {
        e.kind.is_door()
            && match e.side {
                EdgeSide::West => cand.covers(e.x - 1, e.y) || cand.covers(e.x, e.y),
                EdgeSide::North => cand.covers(e.x, e.y - 1) || cand.covers(e.x, e.y),
            }
    });
    if blocks_door || crate::tactical_map_geom::prop_crosses_wall_edge(&cand, edges) {
        return false;
    }
    let overlap = props.iter().any(|p| {
        !prop_spec(p.kind).floor_layer && crate::tactical_map_geom::props_overlap_continuous(&cand, p)
    });
    if overlap {
        return false;
    }
    props.push(cand);
    true
}

/// Furnishes a commercial back-of-house kitchen bay (`DeepFryerStation`, `CommercialGrillHood`,
/// scaled cookline appliances, central stainless prep islands, and perimeter counters).
fn furnish_commercial_kitchen(
    bay: LotBox,
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    surfaces: &mut Vec<MapSurfaceFeature>,
    seed: u32,
) {
    surfaces.push(bay.polygon_feature(SurfaceFeatureKind::LinoleumCheckFloor, 0, seed));
    if bay.w() < 3 || bay.h() < 2 {
        return;
    }
    // 1. North cookline: repeat fryer/grill/cold-storage/counter stations across wide kitchens.
    let mut kx = bay.x0;
    let mut ki = 0u32;
    while kx + 2 <= bay.x1 - 2 {
        let kind = match ki % 4 {
            0 => PropKind::DeepFryerStation,
            1 => PropKind::CommercialGrillHood,
            2 => PropKind::KitchenCounter,
            _ => PropKind::RefrigeratedDisplayCase,
        };
        let len = (kind == PropKind::KitchenCounter).then_some(2);
        let _ = place_checked(
            edges,
            props,
            bay,
            make_prop(kind, kx, bay.y0, 0, len, seed + 1 + ki),
        );
        kx += 3;
        ki += 1;
    }
    if bay.w() >= 6 {
        let _ = place_checked(
            edges,
            props,
            bay,
            make_prop(PropKind::RefrigeratedDisplayCase, bay.x1 - 2, bay.y0, 0, None, seed + 20),
        );
    } else {
        let _ = place_checked(
            edges,
            props,
            bay,
            make_prop(PropKind::Fridge, bay.x1 - 1, bay.y0, 0, None, seed + 20),
        );
    }

    // 2. Interior stainless CommercialPrepTable island rows (maintaining >= 2m aisles).
    if bay.h() >= 3 {
        let mut py = bay.y0 + 2;
        let mut pi = 0u32;
        let max_py = if bay.h() >= 5 { bay.y1 - 2 } else { bay.y0 + 2 };
        while py <= max_py {
            let mut px = bay.x0 + 2;
            while px + 3 <= bay.x1 - 2 {
                let prep_len = (bay.x1 - 2 - px).clamp(2, 4);
                let _ = place_checked(
                    edges,
                    props,
                    bay,
                    make_prop(
                        PropKind::CommercialPrepTable,
                        px,
                        py,
                        0,
                        Some(prep_len),
                        seed + 30 + pi,
                    ),
                );
                px += prep_len + 2;
                pi += 1;
            }
            py += 3;
        }
    }

    // 3. South & West service/prep counters (`KitchenCounter`), `StorageShelf`, `TrashCan`, `FireExtinguisher`.
    let mut sx = bay.x0 + 2;
    let mut si = 0u32;
    while sx + 2 <= bay.x1 - 2 {
        let clen = (bay.x1 - 2 - sx).clamp(2, 3);
        let _ = place_checked(
            edges,
            props,
            bay,
            make_prop(PropKind::KitchenCounter, sx, bay.y1 - 1, 180, Some(clen), seed + 50 + si),
        );
        sx += clen + 1;
        si += 1;
    }
    if bay.h() >= 5 {
        let _ = place_checked(
            edges,
            props,
            bay,
            make_prop(PropKind::SteelShelving, bay.x0, bay.y0 + 2, 270, None, seed + 60),
        );
    }
    let _ = place_checked(
        edges,
        props,
        bay,
        make_prop(PropKind::TrashCan, bay.x0, bay.y1 - 2, 0, None, seed + 61),
    );
    let _ = place_checked(
        edges,
        props,
        bay,
        make_prop(
            PropKind::FireExtinguisher,
            bay.x1 - 1,
            bay.y1 - 1,
            0,
            None,
            seed + 62,
        ),
    );
}

/// Emits a `LowWallStone` patio divider border along `x0..x1` at row `y` with open gaps at `gaps`.
fn put_stone_patio_border_h(
    b: &mut OutdoorBuilder,
    y: i32,
    x0: i32,
    x1: i32,
    gaps: &[i32],
) {
    for x in x0..x1 {
        if !gaps.contains(&x) {
            b.put_edge(
                x,
                y,
                EdgeSide::North,
                WallEdgeKind::LowWallStone,
                MaterialClass::ConcreteBlock,
            );
        }
    }
}

/// Generates a single-story (`1F` only) quick-service fast food restaurant with a wrapping drive-thru lane.
fn generate_fast_food_restaurant(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let s = (seed & 0xffff_ffff) as u32;
    let has_wrap_lane = w >= 20 && h >= 14;
    let pad_x = if has_wrap_lane {
        5.min((w - 10).max(3))
    } else {
        3.min((w - 6).max(2))
    };
    let bx0 = pad_x;
    let bx1 = (w - pad_x).max(bx0 + 8);
    let by0 = if has_wrap_lane { 3 } else { 1 };
    let by1 = (h - 4).max(by0 + 6);
    let lot = LotBox::new(bx0, by0, bx1, by1);

    // Main streetfront AsphaltRoad at south + optional wrapping U-shaped drive-thru lane.
    let road_y = h as f32 - 1.45;
    let main_road = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (-0.5, road_y),
            (w as f32 * 0.5, road_y + ((seed.rem_euclid(5)) as f32 - 2.0) * 0.04),
            (w as f32 + 0.5, road_y),
        ],
        215,
    );
    let dt_road = if has_wrap_lane {
        let dt_w = (bx0 as f32 - 1.8).max(1.25);
        let dt_n = (by0 as f32 - 1.55).max(1.1);
        let dt_e = (bx1 as f32 + 1.8).min(w as f32 - 1.25);
        Some(b.add_ribbon(
            SurfaceFeatureKind::AsphaltRoad,
            &[
                (dt_w, road_y - 0.3),
                (dt_w, dt_n + 0.3),
                (bx0 as f32 * 0.5 + bx1 as f32 * 0.5, dt_n),
                (dt_e, dt_n + 0.3),
                (dt_e, road_y - 0.3),
            ],
            145,
        ))
    } else {
        None
    };
    let walk_y = by1 as f32 + 0.70;
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[(bx0 as f32 + 0.3, walk_y), (bx1 as f32 - 0.3, walk_y)],
        105,
    );

    // North BOH strip (`by0..split_y`): NW Kitchen, NE Public Restroom Suite (`CardinalEntry::South`), South Dining Hall.
    let split_lo = by0 + 2;
    let split_hi = (by1 - 4).max(split_lo);
    let split_y = (by0 + ((by1 - by0) * 40) / 100).clamp(split_lo, split_hi);
    let kx_lo = (bx0 + 3).min(bx1);
    let kx_hi = (bx1 - 4).max(kx_lo);
    let kitchen_x1 = if lot.w() >= 15 {
        (bx1 - 7.min(lot.w() / 2)).clamp(kx_lo, kx_hi)
    } else {
        bx1
    };
    let kitchen_bay = LotBox::new(bx0, by0, kitchen_x1, split_y);
    let rest_bay = LotBox::new(kitchen_x1, by0, bx1, split_y);
    let dining_bay = LotBox::new(bx0, split_y, bx1, by1);

    let mut edges = Vec::new();
    enclose_rect_shell(&mut edges, lot, MaterialClass::Brick, 2);
    let fd_lo = bx0 + 1;
    let fd_hi = (bx1 - 2).max(fd_lo);
    let front_door_x = (bx0 + lot.w() / 2 - 1).clamp(fd_lo, fd_hi);
    for dx in [front_door_x, (front_door_x + 1).min(bx1 - 1)] {
        upsert_edge(&mut edges, dx, by1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    }
    let mid_y = (h / 2).clamp(by0 + 1, by1 - 1);
    upsert_edge(&mut edges, bx0, mid_y, EdgeSide::West, WallEdgeKind::Window, MaterialClass::Glass);

    // Kitchen front partition + Restroom divider.
    h_partition(
        &mut edges,
        split_y,
        bx0,
        kitchen_x1,
        &[kitchen_bay.x0 + 1],
        MaterialClass::Drywall,
        false,
    );
    if kitchen_x1 < bx1 {
        v_partition(&mut edges, kitchen_x1, by0, split_y, &[], MaterialClass::Drywall, false);
    }

    let mut int_props = Vec::new();
    // Dining room floor (`InteriorFloor`), kitchen (`LinoleumCheckFloor`), restrooms (`CeramicTileFloor`).
    b.surfaces.push(dining_bay.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 20));
    if kitchen_x1 < bx1 {
        build_public_restroom_suite_in_lot(
            rest_bay,
            CardinalEntry::South,
            w >= 28 && h >= 20,
            MaterialClass::Brick,
            (seed as u64).wrapping_add(77),
            &mut edges,
            &mut int_props,
            &mut b.surfaces,
        );
    }
    furnish_commercial_kitchen(kitchen_bay, &edges, &mut int_props, &mut b.surfaces, s + 40);

    // Long FastFoodCounter under kitchen service pass + SodaFountain beside counter + Vending machines + Booths!
    let counter_len = (kitchen_x1 - (bx0 + 2)).clamp(3, 8);
    let counter_x = (bx0 + 2).min((kitchen_x1 - counter_len).max(bx0));
    let _ = place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(
            PropKind::FastFoodCounter,
            counter_x,
            split_y,
            180,
            Some(counter_len),
            s + 60,
        ),
    );
    let _ = place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(
            PropKind::SodaFountain,
            (counter_x + counter_len + 1).min(dining_bay.x1 - 2),
            split_y,
            0,
            None,
            s + 61,
        ),
    ) || place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(
            PropKind::SodaFountain,
            dining_bay.x1 - 2,
            by1 - 1,
            180,
            None,
            s + 61,
        ),
    );
    let _ = place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(PropKind::VendingMachine, dining_bay.x1 - 1, split_y + 2, 90, None, s + 62),
    );
    let _ = place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(
            PropKind::SnackVendingMachine,
            dining_bay.x1 - 1,
            (split_y + 3).min(by1 - 2),
            90,
            None,
            s + 63,
        ),
    );
    // Wall-aligned Booths along West window wall (`rot = 0` puts table end flush to West wall).
    let mut by = split_y + 1;
    let mut bi = 0u32;
    while by + 2 <= by1 {
        let _ = place_checked(
            &edges,
            &mut int_props,
            dining_bay,
            make_prop(PropKind::Booth, dining_bay.x0, by, 0, None, s + 70 + bi),
        );
        by += 3;
        bi += 1;
    }
    // Multi-row RoundTables across the dining floor.
    let mut ty = split_y + 3;
    let mut ti = 0u32;
    while ty < by1 - 2 {
        let mut tx = dining_bay.x0 + 4;
        while tx < dining_bay.x1 - 2 {
            let _ = place_checked(
                &edges,
                &mut int_props,
                dining_bay,
                make_prop(PropKind::RoundTable, tx, ty, 0, None, s + 80 + ti),
            );
            tx += 4;
            ti += 1;
        }
        ty += 4;
    }
    let _ = place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(PropKind::TrashCan, front_door_x.saturating_sub(1).max(dining_bay.x0), by1 - 1, 0, None, s + 89),
    );

    for e in edges {
        b.put_edge(e.x, e.y, e.side, e.kind, e.material);
    }
    b.props.extend(int_props);

    // Drive-thru menu board, outdoor picnic seating, vehicles, dumpster, lamps & shrubs.
    if has_wrap_lane {
        let placed_menu = b.try_place_cm(
            PropKind::DriveThruMenuBoard,
            OutdoorBuilder::cm(bx0 as f32 + 2.5),
            OutdoorBuilder::cm(by0 as f32 - 0.65),
            0,
            None,
            true,
        ) || b.try_place_cm(
            PropKind::DriveThruMenuBoard,
            OutdoorBuilder::cm(bx0 as f32 - 0.65),
            OutdoorBuilder::cm(by0 as f32 + 2.0),
            270,
            None,
            true,
        );
        let _ = placed_menu;
        let _ = b.try_place_cm(
            PropKind::TrashCan,
            OutdoorBuilder::cm(bx0 as f32 - 0.65),
            OutdoorBuilder::cm(by1 as f32 - 1.5),
            270,
            None,
            true,
        );
        if let Some(dt_idx) = dt_road {
            let _ = b.place_vehicle_along_ribbon(PropKind::Car, dt_idx, 0.24, 0.0, 6);
            let _ = b.place_vehicle_along_ribbon(PropKind::PickupTruck, dt_idx, 0.62, 0.0, 6);
        }
    }
    let _ = b.place_vehicle_along_ribbon(PropKind::PickupTruck, main_road, 0.80, 0.0, 12);
    let _ = b.try_place_off_road_cm(
        PropKind::PicnicTable,
        OutdoorBuilder::cm(bx0 as f32 + 1.5),
        OutdoorBuilder::cm(walk_y + 0.05),
        0,
        None,
    );
    let _ = b.try_place_off_road_cm(
        PropKind::Dumpster,
        OutdoorBuilder::cm(bx1 as f32 + 1.1),
        OutdoorBuilder::cm(by0 as f32 + 1.2),
        90,
        None,
    );
    let _ = b.try_place_off_road_cm(
        PropKind::StreetLamp,
        OutdoorBuilder::cm(bx0 as f32 + 0.6),
        OutdoorBuilder::cm(walk_y),
        0,
        None,
    );
    b.scatter_turf_prop_rot(
        PropKind::Bush,
        4,
        25,
        RectM {
            x0: 0.8,
            y0: 0.8,
            x1: w as f32 - 0.8,
            y1: road_y - 1.2,
        },
    );
    prune_occluded_surfaces(&mut b.surfaces, w, h);
    b.finalize_connected()
}

/// Generates a streetfront Bistro / Diner (`1F`, plus optional `2F` café mezzanine when `w >= 26 && h >= 20`).
fn generate_bistro_diner(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let s = (seed & 0xffff_ffff) as u32;
    let bx0 = if w >= 24 { 4 } else { 2 };
    let bx1 = (w - bx0).max(bx0 + 8);
    let by0 = 1;
    let by1 = (h - 5).max(by0 + 7);
    let lot = LotBox::new(bx0, by0, bx1, by1);
    let has_upper = w >= 26 && h >= 20;

    let road_y = h as f32 - 1.35;
    let road_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (-0.5, road_y),
            (w as f32 * 0.5, road_y + 0.1),
            (w as f32 + 0.5, road_y),
        ],
        210,
    );
    let patio_y = by1 as f32 + 1.05;
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[(bx0 as f32 + 0.4, patio_y), (bx1 as f32 - 0.4, patio_y)],
        130,
    );

    // Outdoor LowWallStone patio rail guarding sidewalk dining along South terrace.
    let door_cx = bx0 + lot.w() / 2;
    let wall_y = (by1 + 2).min(h - 2);
    put_stone_patio_border_h(&mut b, wall_y, bx0, bx1, &[door_cx - 1, door_cx, door_cx + 1]);

    // 1F zones: North-West Kitchen, North-East Public Restroom Suite, South Dining + Diner Bar Counter.
    let sy_lo = by0 + 2;
    let sy_hi = (by1 - 4).max(sy_lo);
    let split_y = (by0 + (lot.h() * 38) / 100).clamp(sy_lo, sy_hi);
    let kx_lo = (bx0 + 3).min(bx1);
    let kx_hi = (bx1 - 4).max(kx_lo);
    let kitchen_x1 = (bx0 + lot.w() / 2).clamp(kx_lo, kx_hi);
    let kitchen_bay = LotBox::new(bx0, by0, kitchen_x1, split_y);
    let rest_bay = LotBox::new(kitchen_x1, by0, bx1, split_y);
    let dining_bay = LotBox::new(bx0, split_y, bx1, by1);

    let mut edges = Vec::new();
    enclose_rect_shell(&mut edges, lot, MaterialClass::Brick, 2);
    for dx in [door_cx - 1, door_cx] {
        upsert_edge(&mut edges, dx, by1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    }
    h_partition(
        &mut edges,
        split_y,
        bx0,
        kitchen_x1,
        &[bx0 + 1],
        MaterialClass::Drywall,
        false,
    );
    v_partition(&mut edges, kitchen_x1, by0, split_y, &[], MaterialClass::Drywall, false);

    let mut int_props = Vec::new();
    b.surfaces.push(dining_bay.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 10));
    furnish_commercial_kitchen(kitchen_bay, &edges, &mut int_props, &mut b.surfaces, s + 20);
    build_public_restroom_suite_in_lot(
        rest_bay,
        CardinalEntry::South,
        w >= 32 && h >= 22,
        MaterialClass::Brick,
        (seed as u64).wrapping_add(199),
        &mut edges,
        &mut int_props,
        &mut b.surfaces,
    );

    // Stairwell (if 2F loft enabled) placed neatly near South-East dining corner.
    let stair_x = (bx1 - 3).max(bx0 + 4);
    let stair_y = split_y + 1;
    if has_upper {
        int_props.push(make_prop(PropKind::Stairwell, stair_x, stair_y, 0, None, s + 30));
    }

    // Long continuous BarCounter (with BarStools in front & EspressoCounter/BackBarShelf behind)
    let bar_len = ((kitchen_x1 - bx0).max(4) - 1).clamp(3, 6);
    let bar_x = bx0 + 2;
    let bar_y = split_y + 1;
    if place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(PropKind::BarCounter, bar_x, bar_y, 0, Some(bar_len), s + 31),
    ) {
        for i in 0..bar_len {
            let _ = place_checked(
                &edges,
                &mut int_props,
                dining_bay,
                make_prop(PropKind::BarStool, bar_x + i, bar_y + 1, 0, None, s + 40 + i as u32),
            );
        }
        let _ = place_checked(
            &edges,
            &mut int_props,
            dining_bay,
            make_prop(PropKind::EspressoCounter, bar_x, split_y, 0, Some(2), s + 48),
        );
    }

    // HostStand by entrance + Booth rows along West & East window walls!
    let _ = place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(
            PropKind::HostStand,
            (door_cx - 2).max(bx0 + 1),
            by1 - 1,
            180,
            None,
            s + 50,
        ),
    );
    let mut by = split_y + 1;
    let mut bi = 0u32;
    while by + 2 <= by1 {
        let _ = place_checked(
            &edges,
            &mut int_props,
            dining_bay,
            make_prop(PropKind::Booth, bx0, by, 0, None, s + 60 + bi),
        );
        if !has_upper || by > stair_y + 3 {
            let _ = place_checked(
                &edges,
                &mut int_props,
                dining_bay,
                make_prop(PropKind::Booth, bx1 - 2, by, 180, None, s + 70 + bi),
            );
        }
        by += 3;
        bi += 1;
    }
    let mut ty = bar_y + 3;
    let mut ti = 0u32;
    while ty < by1 - 1 {
        for tx in ((bx0 + 3)..(bx1 - 3)).step_by(4) {
            let kind = if (ti + tx as u32).is_multiple_of(2) {
                PropKind::DiningTable
            } else {
                PropKind::RoundTable
            };
            let _ = place_checked(
                &edges,
                &mut int_props,
                dining_bay,
                make_prop(kind, tx, ty, 0, None, s + 80 + ti),
            );
            ti += 1;
        }
        ty += 4;
    }

    for e in edges {
        b.put_edge(e.x, e.y, e.side, e.kind, e.material);
    }
    b.props.extend(int_props);

    // Streetfront patio café tables, planters, lamps, vehicles.
    for px in [(bx0 as f32 + 2.2), (bx1 as f32 - 2.2)] {
        let _ = b.try_place_cm(
            PropKind::PatioUmbrellaTable,
            OutdoorBuilder::cm(px),
            OutdoorBuilder::cm(patio_y - 0.1),
            0,
            None,
            true,
        );
    }
    let _ = b.try_place_cm(
        PropKind::PlanterTable,
        OutdoorBuilder::cm(door_cx as f32 - 2.0),
        OutdoorBuilder::cm(patio_y - 0.4),
        0,
        None,
        true,
    );
    let _ = b.try_place_off_road_cm(
        PropKind::StreetLamp,
        OutdoorBuilder::cm(bx0 as f32 + 0.5),
        OutdoorBuilder::cm(patio_y + 0.2),
        0,
        None,
    );
    let _ = b.place_vehicle_along_ribbon(PropKind::Car, road_idx, 0.25, 0.0, 12);
    let _ = b.place_vehicle_along_ribbon(PropKind::PickupTruck, road_idx, 0.78, 0.0, 12);
    b.scatter_turf_prop_rot(
        PropKind::Bush,
        4,
        25,
        RectM {
            x0: 0.8,
            y0: 0.8,
            x1: w as f32 - 0.8,
            y1: road_y - 1.2,
        },
    );
    prune_occluded_surfaces(&mut b.surfaces, w, h);
    let mut sec = b.finalize_connected();

    if has_upper {
        let mut f2_edges = Vec::new();
        enclose_rect_shell(&mut f2_edges, lot, MaterialClass::Brick, 2);
        let mut f2_props = vec![make_prop(PropKind::Stairwell, stair_x, stair_y, 0, None, s + 300)];
        let mut f2_surfaces = vec![lot.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 301)];
        let _ = place_checked(
            &f2_edges,
            &mut f2_props,
            lot,
            make_prop(PropKind::EspressoCounter, bx0 + 1, by0, 0, Some(3), s + 302),
        );
        let _ = place_checked(
            &f2_edges,
            &mut f2_props,
            lot,
            make_prop(PropKind::BarCounter, bx0 + 1, by0 + 2, 0, Some(3), s + 303),
        );
        for bi in 0..3 {
            let _ = place_checked(
                &f2_edges,
                &mut f2_props,
                lot,
                make_prop(PropKind::BarStool, bx0 + 1 + bi, by0 + 3, 0, None, s + 310 + bi as u32),
            );
        }
        let mut wy = by0 + 1;
        while wy + 2 < by1 {
            let _ = place_checked(
                &f2_edges,
                &mut f2_props,
                lot,
                make_prop(PropKind::Booth, bx0, wy, 0, None, s + 320 + wy as u32),
            );
            let mut tx = bx0 + 5;
            let mut col = 0u32;
            while tx < bx1 - 3 {
                let kind = if col.is_multiple_of(2) {
                    PropKind::RoundTable
                } else {
                    PropKind::DiningTable
                };
                let _ = place_checked(
                    &f2_edges,
                    &mut f2_props,
                    lot,
                    make_prop(kind, tx, wy, 0, None, s + 340 + wy as u32 + col),
                );
                tx += 5;
                col += 1;
            }
            wy += 4;
        }
        place_structural_pillar_grid(bx0, by0, bx1, by1, 6, 5, &f2_edges, &mut f2_props, s + 360);
        trim_floor_props_to_connected(w, h, lot, &f2_edges, &mut f2_props);
        prune_occluded_surfaces(&mut f2_surfaces, w, h);
        debug_assert!(floor_lot_connected(w, h, lot, &f2_edges, &f2_props));
        sec.floors = vec![
            MapFloorLevel {
                floor_index: 0,
                label: "1F · Main Bistro Dining".to_string(),
                elevation_cm: 0,
                wall_edges: sec.wall_edges.clone(),
                props: sec.props.clone(),
                surfaces: sec.surfaces.clone(),
                ..Default::default()
            },
            MapFloorLevel {
                floor_index: 1,
                label: "2F · Mezzanine Cafe Loft".to_string(),
                elevation_cm: 360,
                wall_edges: f2_edges,
                props: f2_props,
                surfaces: f2_surfaces,
                ..Default::default()
            },
        ];
        sec.active_floor = 0;
    }

    sec
}

/// Generates a large upscale Fine Dining Restaurant (`1F` Grand Hall + `2F` Private Dining & Banquet when `w >= 32 && h >= 24`).
fn generate_fine_dining_restaurant(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let s = (seed & 0xffff_ffff) as u32;
    let bx0 = if w >= 22 { 4 } else { 3.min((w - 5).max(1)) };
    let bx1 = (w - bx0).max(bx0 + 6);
    let by0 = 1;
    let by1 = (h - 5).max(by0 + 6);
    let lot = LotBox::new(bx0, by0, bx1, by1);
    let has_upper = w >= 32 && h >= 24;

    let road_y = h as f32 - 1.35;
    let road_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (-0.5, road_y),
            (w as f32 * 0.5, road_y + ((seed.rem_euclid(5)) as f32 - 2.0) * 0.04),
            (w as f32 + 0.5, road_y),
        ],
        220,
    );
    let terrace_y = by1 as f32 + 1.05;
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[(bx0 as f32, terrace_y), (bx1 as f32, terrace_y)],
        150,
    );
    let door_cx = bx0 + lot.w() / 2;
    let patio_wall_y = (by1 + 2).min(h - 2);
    put_stone_patio_border_h(
        &mut b,
        patio_wall_y,
        bx0,
        bx1,
        &[door_cx - 1, door_cx, door_cx + 1],
    );

    // BOH North Strip (`by0..split_y`): NW Kitchen, NE Restroom Suite (`Men`+`Women`+`Family`!).
    let sy_lo = by0 + 2;
    let sy_hi = (by1 - 4).max(sy_lo);
    let split_y = (by0 + (lot.h() * 34) / 100).clamp(sy_lo, sy_hi);
    let kx_lo = (bx0 + 3).min(bx1);
    let kx_hi = (bx1 - 5).max(kx_lo);
    let kitchen_x1 = (bx0 + (lot.w() * 55) / 100).clamp(kx_lo, kx_hi);
    let kitchen_bay = LotBox::new(bx0, by0, kitchen_x1, split_y);
    let rest_bay = LotBox::new(kitchen_x1, by0, bx1, split_y);
    let dining_bay = LotBox::new(bx0, split_y, bx1, by1);

    let mut edges = Vec::new();
    enclose_rect_shell(&mut edges, lot, MaterialClass::Brick, 2);
    for dx in [door_cx - 1, door_cx] {
        upsert_edge(&mut edges, dx, by1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    }
    h_partition(
        &mut edges,
        split_y,
        bx0,
        kitchen_x1,
        &[bx0 + 1, kitchen_x1 - 2],
        MaterialClass::Drywall,
        false,
    );
    v_partition(&mut edges, kitchen_x1, by0, split_y, &[], MaterialClass::Drywall, false);

    let mut int_props = Vec::new();
    b.surfaces.push(dining_bay.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 10));
    furnish_commercial_kitchen(kitchen_bay, &edges, &mut int_props, &mut b.surfaces, s + 20);
    build_public_restroom_suite_in_lot(
        rest_bay,
        CardinalEntry::South,
        true,
        MaterialClass::Brick,
        (seed as u64).wrapping_add(501),
        &mut edges,
        &mut int_props,
        &mut b.surfaces,
    );

    let stair_x = (bx1 - 3).max(bx0 + 5);
    let stair_y = split_y + 1;
    if has_upper {
        int_props.push(make_prop(PropKind::Stairwell, stair_x, stair_y, 0, None, s + 30));
    }

    // Periodic structural column grid across main dining hall!
    place_structural_pillar_grid(bx0, split_y, bx1, by1, 7, 5, &edges, &mut int_props, s + 40);

    // HostStand by south grand entry + BuffetCounter + Cocktail Bar + GrandPiano!
    let _ = place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(PropKind::HostStand, door_cx - 2, by1 - 1, 180, None, s + 50),
    );
    let _ = place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(PropKind::PlanterTable, door_cx + 1, by1 - 2, 0, None, s + 51),
    );
    let bar_len = (lot.w() / 4).clamp(4, 7);
    let bar_x = bx0 + 3;
    let bar_y = split_y + 2;
    if place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(PropKind::BarCounter, bar_x, bar_y, 0, Some(bar_len), s + 52),
    ) {
        for i in 0..bar_len {
            let _ = place_checked(
                &edges,
                &mut int_props,
                dining_bay,
                make_prop(PropKind::BarStool, bar_x + i, bar_y + 1, 0, None, s + 60 + i as u32),
            );
        }
        let _ = place_checked(
            &edges,
            &mut int_props,
            dining_bay,
            make_prop(PropKind::BackBarShelf, bar_x, split_y, 0, Some(bar_len.min(4)), s + 70),
        );
    }
    let _ = place_checked(
        &edges,
        &mut int_props,
        dining_bay,
        make_prop(
            PropKind::BuffetCounter,
            (kitchen_x1 - 6).max(bar_x + bar_len + 1),
            split_y,
            0,
            Some(3),
            s + 72,
        ),
    );
    if w >= 32 {
        let _ = place_checked(
            &edges,
            &mut int_props,
            dining_bay,
            make_prop(PropKind::Sofa, bx1 - 3, by1 - 3, 270, None, s + 74),
        );
    }

    // Perimeter window Booths (`West rot = 0`, `East rot = 180`) + Banquet/Dining tables.
    let mut by = split_y + 1;
    let mut bi = 0u32;
    while by + 2 < by1 {
        let _ = place_checked(
            &edges,
            &mut int_props,
            dining_bay,
            make_prop(PropKind::Booth, bx0, by, 0, None, s + 80 + bi),
        );
        if by > stair_y + 3 || !has_upper {
            let _ = place_checked(
                &edges,
                &mut int_props,
                dining_bay,
                make_prop(PropKind::Booth, bx1 - 2, by, 180, None, s + 90 + bi),
            );
        }
        by += 3;
        bi += 1;
    }
    for (ri, ty) in ((split_y + 4)..(by1 - 2)).step_by(3).enumerate() {
        for (ci, tx) in ((bx0 + 3)..(bx1 - 3)).step_by(4).enumerate() {
            let kind = if (ri + ci) % 2 == 0 {
                PropKind::DiningTable
            } else {
                PropKind::RoundTable
            };
            let _ = place_checked(
                &edges,
                &mut int_props,
                dining_bay,
                make_prop(kind, tx, ty, 0, None, s + 110 + (ri * 8 + ci) as u32),
            );
        }
    }

    for e in edges {
        b.put_edge(e.x, e.y, e.side, e.kind, e.material);
    }
    b.props.extend(int_props);

    let _ = b.try_place_cm(
        PropKind::SmallFountain,
        OutdoorBuilder::cm(door_cx as f32 + 3.2),
        OutdoorBuilder::cm(terrace_y - 0.1),
        0,
        None,
        true,
    );
    let _ = b.try_place_cm(
        PropKind::PatioUmbrellaTable,
        OutdoorBuilder::cm(bx0 as f32 + 2.2),
        OutdoorBuilder::cm(terrace_y - 0.1),
        0,
        None,
        true,
    );
    let _ = b.try_place_cm(
        PropKind::PatioUmbrellaTable,
        OutdoorBuilder::cm(bx1 as f32 - 2.2),
        OutdoorBuilder::cm(terrace_y - 0.1),
        0,
        None,
        true,
    );
    let _ = b.place_vehicle_along_ribbon(PropKind::Car, road_idx, 0.25, 0.0, 12);
    let _ = b.place_vehicle_along_ribbon(PropKind::Car, road_idx, 0.72, 0.0, 12);
    b.scatter_trees(
        PropKind::MediumTree,
        4,
        2.6,
        RectM {
            x0: 0.8,
            y0: 0.8,
            x1: bx0 as f32 - 0.5,
            y1: road_y - 1.2,
        },
    );
    b.scatter_turf_prop_rot(
        PropKind::Hedge,
        4,
        0,
        RectM {
            x0: bx1 as f32 + 0.4,
            y0: 1.0,
            x1: w as f32 - 0.8,
            y1: road_y - 1.2,
        },
    );
    prune_occluded_surfaces(&mut b.surfaces, w, h);
    let mut sec = b.finalize_connected();

    // 2F Private Dining Rooms & Banquet Hall on very large Fine Dining restaurants (`w >= 32 && h >= 24`).
    if has_upper {
        let mut f2_edges = Vec::new();
        enclose_rect_shell(&mut f2_edges, lot, MaterialClass::Brick, 2);
        let vip_w = (lot.w() / 3).clamp(7, 11);
        let vip_x1 = bx0 + vip_w;
        let pdr_h = (lot.h() / 2).clamp(5, 10);
        let pdr_y1 = by0 + pdr_h;
        // Private VIP Dining West + North-Central Banquet Room separated with glass partitions & doors.
        v_partition(&mut f2_edges, vip_x1, by0, by1, &[by0 + 2, by1 - 3], MaterialClass::Drywall, true);
        h_partition(&mut f2_edges, pdr_y1, vip_x1, stair_x - 1, &[vip_x1 + 2], MaterialClass::Drywall, true);

        let mut f2_props = vec![make_prop(PropKind::Stairwell, stair_x, stair_y, 0, None, s + 500)];
        let vip_north = LotBox::new(bx0, by0, vip_x1, by1);
        let banquet_top = LotBox::new(vip_x1, by0, stair_x - 1, pdr_y1);
        let mezzanine = LotBox::new(vip_x1, pdr_y1, bx1, by1);
        let mut f2_surfaces = vec![
            lot.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 501),
            banquet_top.polygon_feature(SurfaceFeatureKind::CarpetFloor, 0, s + 502),
        ];

        // VIP Private Dining Room (ConferenceTables, Sideboards, Buffet & Booths).
        let mut vy = by0 + 2;
        let mut vi = 0u32;
        while vy + 2 < by1 {
            let _ = place_checked(
                &f2_edges,
                &mut f2_props,
                vip_north,
                make_prop(PropKind::Booth, bx0, vy, 0, None, s + 510 + vi),
            );
            let _ = place_checked(
                &f2_edges,
                &mut f2_props,
                vip_north,
                make_prop(
                    if vi.is_multiple_of(2) {
                        PropKind::ConferenceTable
                    } else {
                        PropKind::DiningTable
                    },
                    bx0 + 3,
                    vy,
                    0,
                    None,
                    s + 515 + vi,
                ),
            );
            vy += 4;
            vi += 1;
        }
        let _ = place_checked(
            &f2_edges,
            &mut f2_props,
            vip_north,
            make_prop(PropKind::BuffetCounter, bx0 + 1, by0, 0, Some(2), s + 518),
        );
        // Second Private Banquet Dining Room (`banquet_top`).
        for (bi, tx) in ((vip_x1 + 2)..(stair_x - 3)).step_by(5).enumerate() {
            let _ = place_checked(
                &f2_edges,
                &mut f2_props,
                banquet_top,
                make_prop(PropKind::ConferenceTable, tx, by0 + 2, 0, None, s + 520 + bi as u32),
            );
            let _ = place_checked(
                &f2_edges,
                &mut f2_props,
                banquet_top,
                make_prop(PropKind::BuffetCounter, (tx - 1).max(vip_x1 + 1), by0, 0, Some(3), s + 525 + bi as u32),
            );
        }
        // Mezzanine Cocktail Bar, Lounge Tables & Vending Pantry (`mezzanine`).
        let _ = place_checked(
            &f2_edges,
            &mut f2_props,
            mezzanine,
            make_prop(PropKind::BarCounter, vip_x1 + 1, pdr_y1 + 1, 0, Some(4), s + 530),
        );
        for bi in 0..4 {
            let _ = place_checked(
                &f2_edges,
                &mut f2_props,
                mezzanine,
                make_prop(PropKind::BarStool, vip_x1 + 1 + bi, pdr_y1 + 2, 0, None, s + 532 + bi as u32),
            );
        }
        let _ = place_checked(
            &f2_edges,
            &mut f2_props,
            mezzanine,
            make_prop(PropKind::VendingMachine, bx1 - 1, pdr_y1, 90, None, s + 540),
        );
        let _ = place_checked(
            &f2_edges,
            &mut f2_props,
            mezzanine,
            make_prop(PropKind::SnackVendingMachine, bx1 - 1, pdr_y1 + 1, 90, None, s + 541),
        );
        let mut my = pdr_y1 + 4;
        let mut mi = 0u32;
        while my < by1 - 2 {
            for mx in ((vip_x1 + 2)..(bx1 - 3)).step_by(5) {
                let kind = if mi.is_multiple_of(2) {
                    PropKind::RoundTable
                } else {
                    PropKind::DiningTable
                };
                let _ = place_checked(
                    &f2_edges,
                    &mut f2_props,
                    mezzanine,
                    make_prop(kind, mx, my, 0, None, s + 542 + mi),
                );
                mi += 1;
            }
            my += 4;
        }
        place_structural_pillar_grid(bx0, by0, bx1, by1, 7, 5, &f2_edges, &mut f2_props, s + 550);
        trim_floor_props_to_connected(w, h, lot, &f2_edges, &mut f2_props);
        prune_occluded_surfaces(&mut f2_surfaces, w, h);
        debug_assert!(floor_lot_connected(w, h, lot, &f2_edges, &f2_props));

        sec.floors = vec![
            MapFloorLevel {
                floor_index: 0,
                label: "1F · Grand Dining Room".to_string(),
                elevation_cm: 0,
                wall_edges: sec.wall_edges.clone(),
                props: sec.props.clone(),
                surfaces: sec.surfaces.clone(),
                ..Default::default()
            },
            MapFloorLevel {
                floor_index: 1,
                label: "2F · Private Dining & Banquet".to_string(),
                elevation_cm: 400,
                wall_edges: f2_edges,
                props: f2_props,
                surfaces: f2_surfaces,
                ..Default::default()
            },
        ];
        sec.active_floor = 0;
    }

    sec
}

/// Dispatches all three dedicated restaurant sector presets (`FastFoodRestaurant`, `BistroDiner`, `FineDiningRestaurant`).
#[must_use]
pub fn generate_restaurant_sector(
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
        SectorPreset::FastFoodRestaurant => generate_fast_food_restaurant(w, h, seed),
        SectorPreset::BistroDiner => generate_bistro_diner(w, h, seed),
        SectorPreset::FineDiningRestaurant => generate_fine_dining_restaurant(w, h, seed),
        _ => ProceduralSector::default(),
    }
}
