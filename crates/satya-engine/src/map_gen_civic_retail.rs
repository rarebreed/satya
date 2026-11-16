//! # Specialized Civic & Commercial Facility Generators (`map_gen_civic_retail`)
//!
//! Procedural vector generators for three signature civic and retail presets:
//! - [`SectorPreset::Hospital`]: multi-floor (`1F`/`2F`) hospital campus with exterior ambulance bay,
//!   canopy sidewalk, triage waiting room, patient wards (`HospitalBed`, `PrivacyCurtain`), surgical
//!   suite (`SurgicalTable`), diagnostic imaging (`ImagingScanner`), and aligned [`PropKind::Stairwell`].
//! - [`SectorPreset::ShoppingPlaza`]: curving landscaped parking lot, shaded outdoor concrete
//!   promenade with patio seating & [`PropKind::MallKiosk`], and `3–6` varied store units.
//! - [`SectorPreset::ShoppingMall`]: multi-floor (`1F`/`2F`) enclosed galleria with concourse kiosks,
//!   [`PropKind::PlanterTable`] atrium greenery, department anchors, boutiques, food court, and a `2F`
//!   [`SurfaceFeatureKind::BridgeDeck`] skybridge overlook.

use satya_schema::{
    EdgeSide, MapFloorLevel, MapPropPlacement, MapSurfaceFeature, MapWallEdge, MaterialClass,
    PropKind, StoreArchetype, SurfaceFeatureKind, WallEdgeKind,
};

use crate::map_dressing::{PropObstruction, make_prop, prop_spec};
use crate::map_gen_commercial::generate_store_unit;
use crate::map_gen_residential::{
    CardinalEntry, LotBox, floor_lot_connected, trim_floor_props_to_connected,
};
use crate::map_outdoor_gen::{OutdoorBuilder, RectM};
use crate::map_surface_geom::topmost_surface_at_m;
use crate::map_surface_queries::prune_occluded_surfaces;
use crate::tactical_map::{ProceduralSector, SectorPreset};

fn safe_x_span(w: i32) -> (i32, i32) {
    let pad = if w >= 16 { 4 } else if w >= 12 { 3 } else { 2 };
    (pad, (w - pad).max(pad + 4).min(w - 1))
}

fn enclose_campus_shell(
    b: &mut OutdoorBuilder,
    lot: LotBox,
    wall_mat: MaterialClass,
    win_step: i32,
) {
    for x in lot.x0..lot.x1 {
        let win = win_step > 0 && (x - lot.x0) % win_step == 1 && x + 1 < lot.x1;
        let (k, m) = if win { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, wall_mat) };
        b.put_edge(x, lot.y0, EdgeSide::North, k, m);
        b.put_edge(x, lot.y1, EdgeSide::North, k, m);
    }
    for y in lot.y0..lot.y1 {
        let win = win_step > 0 && (y - lot.y0) % win_step == 1 && y + 1 < lot.y1;
        let (k, m) = if win { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, wall_mat) };
        b.put_edge(lot.x0, y, EdgeSide::West, k, m);
        b.put_edge(lot.x1, y, EdgeSide::West, k, m);
    }
}

fn edge_upsert(edges: &mut Vec<MapWallEdge>, x: i32, y: i32, side: EdgeSide, kind: WallEdgeKind, material: MaterialClass) {
    let e = MapWallEdge { x, y, side, kind, material };
    match edges.iter_mut().find(|o| o.x == x && o.y == y && o.side == side) {
        Some(slot) => *slot = e,
        None => edges.push(e),
    }
}

fn enclose_upper_shell(edges: &mut Vec<MapWallEdge>, lot: LotBox, wall_mat: MaterialClass, win_step: i32) {
    for x in lot.x0..lot.x1 {
        let win = win_step > 0 && (x - lot.x0) % win_step == 1 && x + 1 < lot.x1;
        let (k, m) = if win { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, wall_mat) };
        edge_upsert(edges, x, lot.y0, EdgeSide::North, k, m);
        edge_upsert(edges, x, lot.y1, EdgeSide::North, k, m);
    }
    for y in lot.y0..lot.y1 {
        let win = win_step > 0 && (y - lot.y0) % win_step == 1 && y + 1 < lot.y1;
        let (k, m) = if win { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, wall_mat) };
        edge_upsert(edges, lot.x0, y, EdgeSide::West, k, m);
        edge_upsert(edges, lot.x1, y, EdgeSide::West, k, m);
    }
}

fn h_partition_on_edges(
    edges: &mut Vec<MapWallEdge>,
    y: i32,
    x0: i32,
    x1: i32,
    doors: &[i32],
    wall_mat: MaterialClass,
) {
    for x in x0..x1 {
        if doors.contains(&x) {
            edge_upsert(edges, x, y, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        } else {
            edge_upsert(edges, x, y, EdgeSide::North, WallEdgeKind::Wall, wall_mat);
        }
    }
}

fn v_partition_on_edges(
    edges: &mut Vec<MapWallEdge>,
    x: i32,
    y0: i32,
    y1: i32,
    doors: &[i32],
    wall_mat: MaterialClass,
) {
    for y in y0..y1 {
        if doors.contains(&y) {
            edge_upsert(edges, x, y, EdgeSide::West, WallEdgeKind::DoorClosed, wall_mat);
        } else {
            edge_upsert(edges, x, y, EdgeSide::West, WallEdgeKind::Wall, wall_mat);
        }
    }
}

/// Checks AABB wall-crossing + solid prop overlap and tries to place an indoor prop cleanly inside `bay`.
fn place_indoor_checked(
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    bay: LotBox,
    cand: MapPropPlacement,
) -> bool {
    if !prop_spec(cand.kind).floor_layer
        && (cand.x < bay.x0
            || cand.y < bay.y0
            || cand.x + cand.w_cells > bay.x1
            || cand.y + cand.h_cells > bay.y1
            || crate::tactical_map_geom::prop_crosses_wall_edge(&cand, edges))
    {
        return false;
    }
    if !prop_spec(cand.kind).floor_layer
        && props.iter().any(|e| {
            (!prop_spec(e.kind).floor_layer || e.kind == PropKind::Stairwell)
                && crate::tactical_map_geom::props_overlap_continuous(&cand, e)
        })
    {
        return false;
    }
    props.push(cand);
    true
}

fn fill_hospital_floor(
    lot: LotBox,
    edges: &mut Vec<MapWallEdge>,
    props: &mut Vec<MapPropPlacement>,
    stair_xy: (i32, i32),
    is_upper: bool,
    seed: u64,
) {
    let bw = lot.w();
    let bh = lot.h();
    let mid_y = (lot.y0 + bh / 2).clamp(lot.y0 + 3, lot.y1 - 4);
    let corr_y0 = (mid_y - 1).max(lot.y0 + 2);
    let corr_y1 = (corr_y0 + 2).min(lot.y1 - 2);
    let tx0 = (lot.x0 + bw / 2 - 1).max(lot.x0 + 3);
    let tx1 = (tx0 + 2).min(lot.x1 - 3);

    // North wing split into 3 medical bays; South wing split flanking central ER/lobby transept.
    let nx_a = lot.x0 + bw / 3;
    let nx_b = lot.x0 + (2 * bw) / 3;
    h_partition_on_edges(edges, corr_y0, lot.x0, lot.x1, &[lot.x0 + 1, (nx_a + nx_b) / 2, lot.x1 - 2], MaterialClass::Drywall);
    v_partition_on_edges(edges, nx_a, lot.y0, corr_y0, &[lot.y0 + 1], MaterialClass::Drywall);
    v_partition_on_edges(edges, nx_b, lot.y0, corr_y0, &[corr_y0 - 1], MaterialClass::Drywall);

    h_partition_on_edges(edges, corr_y1, lot.x0, tx0, &[lot.x0 + 1], MaterialClass::Drywall);
    h_partition_on_edges(edges, corr_y1, tx1, lot.x1, &[lot.x1 - 2], MaterialClass::Drywall);
    v_partition_on_edges(edges, tx0, corr_y1, lot.y1, &[corr_y1], MaterialClass::Drywall);
    v_partition_on_edges(edges, tx1, corr_y1, lot.y1, &[corr_y1], MaterialClass::Drywall);

    // Sub-divide wide North wards or South wings slightly when campus is large.
    if bw >= 26 && bh >= 16 {
        let extra_x = nx_a + (nx_b - nx_a) / 2;
        v_partition_on_edges(edges, extra_x, lot.y0, corr_y0, &[corr_y0 - 1], MaterialClass::Drywall);
    }

    // Aligned elevator/stairwell shaft inside the North-central nurse hub.
    let stair_prop = make_prop(PropKind::Stairwell, stair_xy.0, stair_xy.1, 0, None, seed as u32);
    props.push(stair_prop);

    // Dress waiting/triage (NW), ward beds + curtains (N-mid & NE), surgical suite (SW), imaging suite (SE).
    let s = seed as u32;
    let triage = LotBox::new(lot.x0, lot.y0, nx_a, corr_y0);
    let ward_mid = LotBox::new(nx_a, lot.y0, nx_b, corr_y0);
    let ward_ne = LotBox::new(nx_b, lot.y0, lot.x1, corr_y0);
    let sw_wing = LotBox::new(lot.x0, corr_y1, tx0, lot.y1);
    let se_wing = LotBox::new(tx1, corr_y1, lot.x1, lot.y1);

    if is_upper {
        let _ = place_indoor_checked(edges, props, triage, make_prop(PropKind::HospitalBed, triage.x0, triage.y0, 0, None, s + 1));
        let _ = place_indoor_checked(edges, props, triage, make_prop(PropKind::PrivacyCurtain, triage.x0, triage.y0 + 1, 0, Some(triage.w().min(3)), s + 2));
        let _ = place_indoor_checked(edges, props, triage, make_prop(PropKind::Armchair, triage.x1 - 1, triage.y0, 90, None, s + 3));
    } else {
        let _ = place_indoor_checked(edges, props, triage, make_prop(PropKind::CheckoutCounter, triage.x0 + 1, triage.y0 + 1, 0, Some(2), s + 1));
        let _ = place_indoor_checked(edges, props, triage, make_prop(PropKind::Sofa, triage.x0, triage.y0, 0, None, s + 2));
        let _ = place_indoor_checked(edges, props, triage, make_prop(PropKind::Armchair, triage.x1 - 1, triage.y0, 180, None, s + 3));
    }

    // Patient beds + privacy curtains along NE ward & center bays.
    for (idx, bay) in [ward_mid, ward_ne].iter().enumerate() {
        let mut bx = bay.x0;
        while bx + 2 <= bay.x1 {
            let bed = make_prop(PropKind::HospitalBed, bx, bay.y0, 90, None, s + 10 + idx as u32);
            if place_indoor_checked(edges, props, *bay, bed) && bx + 2 < bay.x1 {
                let curt = make_prop(PropKind::PrivacyCurtain, bx + 1, bay.y0, 90, Some(bay.h().clamp(2, 3)), s + 20 + idx as u32);
                let _ = place_indoor_checked(edges, props, *bay, curt);
            }
            bx += 3;
        }
        let _ = place_indoor_checked(edges, props, *bay, make_prop(PropKind::SteelShelving, bay.x0, bay.y1 - 1, 0, Some(2), s + 31));
    }

    // Surgical operating wing (SW).
    let sx = sw_wing.x0 + (sw_wing.w() / 2 - 1).max(0);
    let sy = sw_wing.y0 + (sw_wing.h() / 2 - 1).max(0);
    let _ = place_indoor_checked(edges, props, sw_wing, make_prop(PropKind::SurgicalTable, sx, sy, 0, None, s + 40));
    if sw_wing.w() >= 6 {
        let _ = place_indoor_checked(edges, props, sw_wing, make_prop(PropKind::SurgicalTable, sw_wing.x0, sw_wing.y0, 0, None, s + 41));
        let _ = place_indoor_checked(edges, props, sw_wing, make_prop(PropKind::PrivacyCurtain, sw_wing.x0 + 2, sw_wing.y0, 90, Some(2), s + 42));
    }
    let _ = place_indoor_checked(edges, props, sw_wing, make_prop(PropKind::SteelShelving, sw_wing.x0, sw_wing.y1 - 1, 0, Some(2), s + 43));

    // Diagnostic imaging wing (SE).
    let ix = se_wing.x0 + (se_wing.w() / 2 - 1).max(0);
    let iy = se_wing.y0 + (se_wing.h() / 2 - 1).max(0);
    let _ = place_indoor_checked(edges, props, se_wing, make_prop(PropKind::ImagingScanner, ix, iy, 0, None, s + 50));
    let _ = place_indoor_checked(edges, props, se_wing, make_prop(PropKind::Desk, se_wing.x1 - 2, se_wing.y0, 0, None, s + 51));
    if is_upper && se_wing.w() >= 6 {
        let _ = place_indoor_checked(edges, props, se_wing, make_prop(PropKind::HospitalBed, se_wing.x0, se_wing.y1 - 1, 0, None, s + 52));
    }

    // Reception / crash-cart cover along main E-W corridor margins.
    let _ = place_indoor_checked(edges, props, lot, make_prop(PropKind::Planter, tx0 - 2, corr_y0, 0, None, s + 60));
    let _ = place_indoor_checked(edges, props, lot, make_prop(PropKind::CheckoutCounter, tx1, corr_y0, 0, Some(2), s + 61));
}

fn generate_hospital(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let (bx0, bx1) = safe_x_span(w);
    let by0 = 1;
    let by1 = (h - 4).max(by0 + 7);
    let lot = LotBox::new(bx0, by0, bx1, by1);
    let bw = lot.w();

    // Exterior ambulance bay AsphaltRoad & covered ER ConcreteWalk canopy (strictly South of building).
    let road_y = h as f32 - 1.35;
    let ctrl_road = [
        (-0.5, road_y - 0.15),
        (w as f32 * 0.25, road_y + b.rng.f32_range(-0.25, 0.25)),
        (w as f32 * 0.50, road_y - 0.20),
        (w as f32 * 0.75, road_y + b.rng.f32_range(-0.25, 0.25)),
        (w as f32 + 0.5, road_y - 0.10),
    ];
    let road_idx = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &ctrl_road, 210);
    let walk_y = by1 as f32 + 0.65;
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[(bx0 as f32 + 1.0, walk_y), (w as f32 * 0.5, walk_y + 0.08), (bx1 as f32 - 1.0, walk_y)],
        115,
    );

    // Building 1F InteriorFloor polygon strictly inside campus walls.
    let floor_seed = (b.rng.next() & 0xffff_ffff) as u32;
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[(bx0 as f32, by0 as f32), (bx1 as f32, by0 as f32), (bx1 as f32, by1 as f32), (bx0 as f32, by1 as f32)],
        0,
        floor_seed,
    );

    enclose_campus_shell(&mut b, lot, MaterialClass::ReinforcedConcrete, 3);
    let mid_y = (by0 + lot.h() / 2).clamp(by0 + 3, by1 - 4);
    let corr_y0 = (mid_y - 1).max(by0 + 2);
    let tx0 = (bx0 + bw / 2 - 1).max(bx0 + 3);
    // Wide ER double sliding glass doors + West/East campus doors near center spawn axis.
    b.put_edge(tx0, by1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    b.put_edge(tx0 + 1, by1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    b.put_edge(bx0, corr_y0, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    b.put_edge(bx0, corr_y0 + 1, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    b.put_edge(bx1, corr_y0, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    b.put_edge(bx1, corr_y0 + 1, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    b.put_edge(bx0 + 1, by0, EdgeSide::North, WallEdgeKind::DoorClosed, MaterialClass::SheetSteel);

    let stair_xy = ((bx0 + bw / 3 + 1).min(bx1 - 3), (by0 + 1).min(corr_y0 - 3).max(by0));
    let mut f1_props = Vec::new();
    fill_hospital_floor(lot, &mut b.edges, &mut f1_props, stair_xy, false, seed as u64);
    trim_floor_props_to_connected(w, h, lot, &b.edges, &mut f1_props);
    for p in f1_props {
        let (cx, cy) = p.center_cm();
        let len = prop_spec(p.kind).stretchable.then_some(p.local_span_cm().0);
        if prop_spec(p.kind).floor_layer {
            b.props.push(p);
        } else {
            let _ = b.try_place_cm(p.kind, cx, cy, p.rotation_deg, len, true);
        }
    }

    // Exterior ambulances, cars, landscaped flowerbeds & campus trees strictly outside building/road.
    let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, road_idx, 0.42, 0.0, 5);
    let _ = b.place_vehicle_along_ribbon(PropKind::Car, road_idx, 0.70, 0.0, 6);
    let _ = b.try_add_organic_patch(SurfaceFeatureKind::GardenBedZone, (2.0, 2.3), (1.3, 1.1), 0.30, 28);
    let _ = b.try_add_organic_patch(SurfaceFeatureKind::GardenBedZone, (w as f32 - 2.0, 2.3), (1.3, 1.1), 0.30, 28);
    b.scatter_trees(PropKind::MediumTree, 4, 2.8, RectM { x0: 1.2, y0: 1.2, x1: bx0 as f32 - 0.5, y1: by1 as f32 });
    b.scatter_trees(PropKind::MediumTree, 4, 2.8, RectM { x0: bx1 as f32 + 0.5, y0: 1.2, x1: w as f32 - 1.2, y1: by1 as f32 });
    b.scatter_turf_prop_rot(PropKind::ParkBench, 2, 15, RectM { x0: 1.2, y0: 1.2, x1: bx0 as f32 - 0.4, y1: h as f32 - 3.5 });

    // Build 2F · Inpatient & Surgical Ward with aligned stairwell.
    let mut f2_edges = Vec::new();
    let mut f2_props = Vec::new();
    enclose_upper_shell(&mut f2_edges, lot, MaterialClass::ReinforcedConcrete, 3);
    fill_hospital_floor(lot, &mut f2_edges, &mut f2_props, stair_xy, true, (seed as u64).wrapping_add(77));
    trim_floor_props_to_connected(w, h, lot, &f2_edges, &mut f2_props);
    let mut f2_surfaces = vec![lot.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, floor_seed.wrapping_add(19))];
    prune_occluded_surfaces(&mut f2_surfaces, w, h);

    let mut sec = b.finalize_connected();
    let f1_floor = MapFloorLevel {
        floor_index: 0,
        label: "1F · Emergency & Trauma Wing".to_string(),
        elevation_cm: 0,
        wall_edges: sec.wall_edges.clone(),
        props: sec.props.clone(),
        surfaces: sec.surfaces.clone(),
    };
    let f2_floor = MapFloorLevel {
        floor_index: 1,
        label: "2F · Inpatient & Surgical Ward".to_string(),
        elevation_cm: 360,
        wall_edges: f2_edges,
        props: f2_props,
        surfaces: f2_surfaces,
    };
    sec.floors = vec![f1_floor, f2_floor];
    sec.active_floor = 0;
    sec
}

fn generate_shopping_plaza(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let (bx0, bx1) = safe_x_span(w);
    let bw = bx1 - bx0;
    let by0 = 1;
    let store_h = ((h * 38) / 100).clamp(5, (h - 7).max(5));
    let by1 = (by0 + store_h).min(h - 6);

    // Shaded outdoor concrete canopy promenade immediately in front of storefronts.
    let prom_y = by1 as f32 + 1.05;
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[(1.0, prom_y), (w as f32 * 0.5, prom_y + 0.05), (w as f32 - 1.0, prom_y)],
        200,
    );

    // Curving parking drive aisle across the South lot with generous landscaped islands left clear.
    let lot_y = (by1 as f32 + 3.45).min(h as f32 - 1.45);
    let wob = b.rng.f32_range(-0.35, 0.35);
    let park_ctrl = [
        (-0.5, lot_y - wob),
        (w as f32 * 0.25, lot_y + wob),
        (w as f32 * 0.50, lot_y - wob * 0.6),
        (w as f32 * 0.75, lot_y + wob * 0.8),
        (w as f32 + 0.5, lot_y),
    ];
    let road_idx = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &park_ctrl, 250);

    // Partition storefront strip into 3-6 varied StoreArchetypes reusing map_gen_commercial.
    let store_count = (bw / 7).clamp(3, 6);
    let archetypes = [
        StoreArchetype::CafeRestaurant,
        StoreArchetype::ConvenienceBodega,
        StoreArchetype::Pharmacy,
        StoreArchetype::ApparelBoutique,
        StoreArchetype::Electronics,
        StoreArchetype::DepartmentAnchor,
    ];
    let offset = (seed.unsigned_abs() as usize) % archetypes.len();
    let mut cur_x = bx0;
    for i in 0..store_count {
        let next_x = if i + 1 == store_count {
            bx1
        } else {
            bx0 + ((i + 1) * bw) / store_count
        };
        let bay = LotBox::new(cur_x, by0, next_x, by1);
        cur_x = next_x;
        let arch = archetypes[(offset + i as usize) % archetypes.len()];
        let unit = generate_store_unit(
            w,
            h,
            bay,
            arch,
            CardinalEntry::South,
            true,
            &b.spawn_pads_m,
            (seed as u64).wrapping_add(i as u64 * 131),
        );
        for e in unit.wall_edges {
            b.put_edge(e.x, e.y, e.side, e.kind, e.material);
        }
        b.surfaces.extend(unit.surfaces);
        b.props.extend(unit.props);
    }

    // Promenade furniture: cafe patio tables, benches, and outdoor MallKiosk cart strictly on promenade.
    let kiosk_cx = OutdoorBuilder::cm(w as f32 * 0.5 + b.rng.f32_range(-1.5, 1.5));
    let kiosk_rot = b.rng.range(-12, 13);
    let prom_cy = OutdoorBuilder::cm(prom_y - 0.15);
    let _ = b.try_place_off_road_cm(PropKind::MallKiosk, kiosk_cx, prom_cy, kiosk_rot, None);
    let patio_x = OutdoorBuilder::cm(bx0 as f32 + 2.5);
    let _ = b.try_place_off_road_cm(PropKind::RoundTable, patio_x, prom_cy, 15, None);
    let _ = b.try_place_off_road_cm(PropKind::ParkBench, OutdoorBuilder::cm(bx1 as f32 - 2.5), prom_cy, 180, None);

    // Angled parking vehicles along parking lane.
    for (idx, frac) in [0.18_f32, 0.38, 0.62, 0.82].iter().enumerate() {
        let veh = if idx % 3 == 2 { PropKind::UtilityVan } else { PropKind::Car };
        let _ = b.place_vehicle_along_ribbon(veh, road_idx, *frac, 0.0, 22);
    }

    // Landscaped turf islands + trees on flank margins strictly outside roads, promenade & interiors.
    let _ = b.try_add_organic_patch(SurfaceFeatureKind::GardenBedZone, (1.9, 2.6), (1.3, 1.2), 0.30, 28);
    let _ = b.try_add_organic_patch(SurfaceFeatureKind::GardenBedZone, (w as f32 - 1.9, 2.6), (1.3, 1.2), 0.30, 28);
    b.scatter_trees(PropKind::MediumTree, 4, 2.5, RectM { x0: 1.0, y0: 1.0, x1: bx0 as f32 - 0.5, y1: prom_y - 1.2 });
    b.scatter_trees(PropKind::MediumTree, 4, 2.5, RectM { x0: bx1 as f32 + 0.5, y0: 1.0, x1: w as f32 - 1.0, y1: prom_y - 1.2 });
    b.scatter_turf_prop_rot(PropKind::Bush, 4, 25, RectM { x0: 0.8, y0: 0.8, x1: w as f32 - 0.8, y1: prom_y - 1.0 });

    b.finalize_connected()
}

fn build_mall_concourse_shell(edges: &mut Vec<MapWallEdge>, w: i32, h: i32, conc_y0: i32, conc_y1: i32, is_upper: bool) {
    let mid_x = w / 2;
    for x in 0..w {
        let is_portal = !is_upper && (x == mid_x || x == mid_x - 1);
        let (k, m) = if is_portal {
            (WallEdgeKind::DoorOpen, MaterialClass::Glass)
        } else if x % 3 == 1 && x + 1 < w {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete)
        };
        edge_upsert(edges, x, 0, EdgeSide::North, k, m);
        edge_upsert(edges, x, h, EdgeSide::North, k, m);
    }
    for y in 0..h {
        let is_portal = !is_upper && y >= conc_y0 && y < conc_y1 && (y == h / 2 || y == h / 2 - 1);
        let (k, m) = if is_portal {
            (WallEdgeKind::DoorOpen, MaterialClass::Glass)
        } else if y % 3 == 1 && y + 1 < h {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete)
        };
        edge_upsert(edges, 0, y, EdgeSide::West, k, m);
        edge_upsert(edges, w, y, EdgeSide::West, k, m);
    }
}

fn generate_shopping_mall(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let conc_half = if h >= 28 { 3 } else { 2 };
    let conc_y0 = (h / 2 - conc_half).clamp(4, (h - 7).max(4));
    let conc_y1 = (h / 2 + conc_half).clamp(conc_y0 + 3, (h - 4).max(conc_y0 + 3));
    let trans_half = if w >= 36 { 3 } else { 2 };
    let cx0 = (w / 2 - trans_half).clamp(4, w - 7);
    let cx1 = (w / 2 + trans_half).clamp(cx0 + 3, w - 4);

    let full_lot = LotBox::new(0, 0, w, h);
    let floor_seed = (seed as u32).wrapping_add(0x4A11);
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[(0.0, 0.0), (w as f32, 0.0), (w as f32, h as f32), (0.0, h as f32)],
        0,
        floor_seed,
    );

    // Partition NW, NE, SW, SE store wings (splitting wide quadrants into anchor + boutique bays).
    let archetypes = [
        StoreArchetype::DepartmentAnchor,
        StoreArchetype::ApparelBoutique,
        StoreArchetype::CafeRestaurant,
        StoreArchetype::Electronics,
        StoreArchetype::Pharmacy,
        StoreArchetype::Grocery,
    ];
    let mut wing_lots: Vec<(LotBox, CardinalEntry, StoreArchetype)> = Vec::new();
    let quadrants = [
        (0, cx0, 0, conc_y0, CardinalEntry::South),
        (cx1, w, 0, conc_y0, CardinalEntry::South),
        (0, cx0, conc_y1, h, CardinalEntry::North),
        (cx1, w, conc_y1, h, CardinalEntry::North),
    ];
    for (qi, &(qx0, qx1, qy0, qy1, entry)) in quadrants.iter().enumerate() {
        let qw = qx1 - qx0;
        if qw >= 14 {
            let split = qx0 + qw / 2;
            wing_lots.push((LotBox::new(qx0, qy0, split, qy1), entry, archetypes[(qi * 2) % archetypes.len()]));
            wing_lots.push((LotBox::new(split, qy0, qx1, qy1), entry, archetypes[(qi * 2 + 1) % archetypes.len()]));
        } else if qw >= 4 && (qy1 - qy0) >= 4 {
            wing_lots.push((LotBox::new(qx0, qy0, qx1, qy1), entry, archetypes[qi % archetypes.len()]));
        }
    }

    for (bi, &(bay, entry, arch)) in wing_lots.iter().enumerate() {
        let unit = generate_store_unit(w, h, bay, arch, entry, false, &b.spawn_pads_m, (seed as u64).wrapping_add(bi as u64 * 97));
        for e in unit.wall_edges {
            b.put_edge(e.x, e.y, e.side, e.kind, e.material);
        }
        b.props.extend(unit.props);
    }

    // Apply enclosing 0..w, 0..h galleria perimeter shell on top so portal doors and glazed entries stay exact.
    build_mall_concourse_shell(&mut b.edges, w, h, conc_y0, conc_y1, false);

    // Central mall atrium stairwell + MallKiosk + PlanterTable seating strictly inside concourse/transept.
    let stair_x = (cx0 + 1).clamp(4, (w - 6).max(4));
    let stair_y = conc_y0;
    let stair_1f = make_prop(PropKind::Stairwell, stair_x, stair_y, 0, None, floor_seed);
    b.props.push(stair_1f);

    let cy_mid_cm = OutdoorBuilder::cm((conc_y0 + conc_y1) as f32 * 0.5);
    let _ = b.try_place_cm(PropKind::MallKiosk, OutdoorBuilder::cm((cx0 + cx1) as f32 * 0.5 + 1.5), cy_mid_cm, 12, None, true);
    let _ = b.try_place_cm(PropKind::PlanterTable, OutdoorBuilder::cm((cx0 + 1) as f32), OutdoorBuilder::cm(conc_y1 as f32 - 0.6), 0, Some(200), true);
    if w >= 28 {
        let _ = b.try_place_cm(PropKind::MallKiosk, OutdoorBuilder::cm(w as f32 * 0.28), cy_mid_cm, 348, None, true);
        let _ = b.try_place_cm(PropKind::MallKiosk, OutdoorBuilder::cm(w as f32 * 0.72), cy_mid_cm, 15, None, true);
        let _ = b.try_place_cm(PropKind::SmallFountain, OutdoorBuilder::cm(w as f32 * 0.40), cy_mid_cm, 0, None, true);
        let _ = b.try_place_cm(PropKind::ParkBench, OutdoorBuilder::cm(w as f32 * 0.60), OutdoorBuilder::cm(conc_y0 as f32 + 0.7), 0, None, true);
        let _ = b.try_place_cm(PropKind::TrashCan, OutdoorBuilder::cm(w as f32 * 0.62), OutdoorBuilder::cm(conc_y0 as f32 + 0.7), 0, None, true);
    }

    let mut sec = b.finalize_connected();

    // Build 2F · Mezzanine Gallery with aligned stairwell, skylight BridgeDeck span, and mezzanine shops.
    let mut f2_edges = Vec::new();
    let mut f2_props = Vec::new();
    for (bi, &(bay, entry, arch)) in wing_lots.iter().enumerate() {
        let f2_arch = match arch {
            StoreArchetype::DepartmentAnchor => StoreArchetype::ApparelBoutique,
            StoreArchetype::Grocery => StoreArchetype::CafeRestaurant,
            other => other,
        };
        let unit = generate_store_unit(w, h, bay, f2_arch, entry, false, &[], (seed as u64).wrapping_add(500 + bi as u64 * 59));
        for e in unit.wall_edges {
            edge_upsert(&mut f2_edges, e.x, e.y, e.side, e.kind, e.material);
        }
        f2_props.extend(unit.props);
    }
    build_mall_concourse_shell(&mut f2_edges, w, h, conc_y0, conc_y1, true);
    f2_props.push(make_prop(PropKind::Stairwell, stair_x, stair_y, 0, None, floor_seed + 1));
    let kiosk_2f = make_prop(PropKind::MallKiosk, (w / 2 - 1).clamp(3, w - 4), conc_y0 + 1, 0, None, floor_seed + 2);
    let _ = place_indoor_checked(&f2_edges, &mut f2_props, full_lot, kiosk_2f);
    let planter_2f = make_prop(PropKind::PlanterTable, cx0, conc_y1 - 1, 0, Some(2), floor_seed + 3);
    let _ = place_indoor_checked(&f2_edges, &mut f2_props, full_lot, planter_2f);
    trim_floor_props_to_connected(w, h, full_lot, &f2_edges, &mut f2_props);

    let mut f2_surfaces = vec![
        full_lot.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, floor_seed + 7),
        MapSurfaceFeature {
            kind: SurfaceFeatureKind::BridgeDeck,
            shape: satya_schema::SurfaceShapeKind::Polygon,
            points_cm: vec![
                satya_schema::MapPointCm::new(cx0 * 100, conc_y0 * 100),
                satya_schema::MapPointCm::new(cx1 * 100, conc_y0 * 100),
                satya_schema::MapPointCm::new(cx1 * 100, conc_y1 * 100),
                satya_schema::MapPointCm::new(cx0 * 100, conc_y1 * 100),
            ],
            width_cm: 0,
            border_style_cm: 12,
            variant_seed: floor_seed + 8,
        },
    ];
    prune_occluded_surfaces(&mut f2_surfaces, w, h);
    debug_assert!(floor_lot_connected(w, h, full_lot, &f2_edges, &f2_props));

    let f1_floor = MapFloorLevel {
        floor_index: 0,
        label: "1F · Galleria Concourse".to_string(),
        elevation_cm: 0,
        wall_edges: sec.wall_edges.clone(),
        props: sec.props.clone(),
        surfaces: sec.surfaces.clone(),
    };
    let f2_floor = MapFloorLevel {
        floor_index: 1,
        label: "2F · Mezzanine Gallery".to_string(),
        elevation_cm: 420,
        wall_edges: f2_edges,
        props: f2_props,
        surfaces: f2_surfaces,
    };
    sec.floors = vec![f1_floor, f2_floor];
    sec.active_floor = 0;
    sec
}

/// Dispatches Phase 5 civic and commercial facility sector presets (`Hospital`, `ShoppingPlaza`, `ShoppingMall`).
#[must_use]
pub fn generate_civic_or_retail_sector(
    preset: SectorPreset,
    width: i32,
    height: i32,
    seed: i64,
) -> ProceduralSector {
    let w = width.clamp(crate::tactical_map::MIN_SECTOR_DIM, crate::tactical_map::MAX_SECTOR_DIM);
    let h = height.clamp(crate::tactical_map::MIN_SECTOR_DIM, crate::tactical_map::MAX_SECTOR_DIM);
    match preset {
        SectorPreset::Hospital => generate_hospital(w, h, seed),
        SectorPreset::ShoppingPlaza => generate_shopping_plaza(w, h, seed),
        SectorPreset::ShoppingMall => generate_shopping_mall(w, h, seed),
        _ => ProceduralSector::default(),
    }
}

/// Ensures at least one non-blocking surface check exists cleanly (used by sibling test verification).
#[allow(dead_code)]
fn has_exposed_half_cover(sec: &ProceduralSector) -> bool {
    let _ = topmost_surface_at_m;
    sec.props.iter().any(|p| prop_spec(p.kind).obstruction == PropObstruction::HalfCover)
}
