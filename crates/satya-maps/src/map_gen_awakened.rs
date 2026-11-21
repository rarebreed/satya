//! # *Awakened* 2035 Procedural Venue Generators (`map_gen_awakened`)
//!
//! Four scenario-grounded 2035 near-future battlemap generators built strictly on existing
//! validated [`PropKind`] and [`SurfaceFeatureKind`] variants:
//! - [`SectorPreset::CommunityClinicAnnex`] (*Episode 1: Cold Storage at St. Jude Annex*, `1F` + `B1` vault)
//! - [`SectorPreset::CorporateServerBunker`] (Hardened AI 2.0 cryo-compute hall + biometric mantrap)
//! - [`SectorPreset::SatyangSafehouse`] (Covert copper-mesh shophouse & `B1` SIGINT sanctuary)
//! - [`SectorPreset::BrownoutLogisticsDepot`] (Logistics bay + fenced `500 kV` substation + wet-bulb shelter)

use satya_schema::{
    EdgeSide, MapFaradayZone, MapFloorLevel, MapPropPlacement, MapSurveillanceNode,
    MapWallEdge, MaterialClass, MessageVisibility, PropKind, SurfaceFeatureKind, WallEdgeKind,
};

use crate::map_dressing::{make_prop, prop_spec};
use crate::map_gen_residential::{LotBox, trim_floor_props_to_connected};
use crate::map_outdoor_gen::OutdoorBuilder;
use crate::map_surface_queries::prune_occluded_surfaces;
use crate::tactical_map::{ProceduralSector, SectorPreset};

#[expect(clippy::too_many_arguments)]
fn upsert_edge(
    edges: &mut Vec<MapWallEdge>,
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    side: EdgeSide,
    kind: WallEdgeKind,
    material: MaterialClass,
) {
    let valid = match side {
        EdgeSide::West => x >= 0 && x <= w && y >= 0 && y < h,
        EdgeSide::North => x >= 0 && x < w && y >= 0 && y <= h,
    };
    if !valid {
        return;
    }
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

fn enclose_lot(
    edges: &mut Vec<MapWallEdge>,
    w: i32,
    h: i32,
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
        upsert_edge(edges, w, h, x, lot.y0, EdgeSide::North, k, m);
        upsert_edge(edges, w, h, x, lot.y1, EdgeSide::North, k, m);
    }
    for y in lot.y0..lot.y1 {
        let win = win_step > 0 && (y - lot.y0) % win_step == 1 && y + 1 < lot.y1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        upsert_edge(edges, w, h, lot.x0, y, EdgeSide::West, k, m);
        upsert_edge(edges, w, h, lot.x1, y, EdgeSide::West, k, m);
    }
}

#[expect(clippy::too_many_arguments)]
fn build_v_partition(
    edges: &mut Vec<MapWallEdge>,
    w: i32,
    h: i32,
    x: i32,
    y0: i32,
    y1: i32,
    door_y: i32,
    open: bool,
    wall_mat: MaterialClass,
) {
    let dk = if open {
        WallEdgeKind::DoorOpen
    } else {
        WallEdgeKind::DoorClosed
    };
    for y in y0..y1 {
        if y == door_y {
            upsert_edge(edges, w, h, x, y, EdgeSide::West, dk, MaterialClass::SheetSteel);
        } else {
            upsert_edge(edges, w, h, x, y, EdgeSide::West, WallEdgeKind::Wall, wall_mat);
        }
    }
}

#[expect(clippy::too_many_arguments)]
fn build_h_partition(
    edges: &mut Vec<MapWallEdge>,
    w: i32,
    h: i32,
    y: i32,
    x0: i32,
    x1: i32,
    door_x: i32,
    open: bool,
    wall_mat: MaterialClass,
) {
    let dk = if open {
        WallEdgeKind::DoorOpen
    } else {
        WallEdgeKind::DoorClosed
    };
    for x in x0..x1 {
        if x == door_x {
            upsert_edge(edges, w, h, x, y, EdgeSide::North, dk, MaterialClass::SheetSteel);
        } else {
            upsert_edge(edges, w, h, x, y, EdgeSide::North, WallEdgeKind::Wall, wall_mat);
        }
    }
}

fn safe_interior_lot(w: i32, h: i32, has_south_street: bool) -> LotBox {
    let lx0 = if w >= 16 { 4 } else if w >= 12 { 3 } else { 1 };
    let lx1 = if w >= 16 { w - 4 } else if w >= 12 { w - 3 } else { w - 1 };
    let ly0 = if h >= 14 { 2 } else { 1 };
    let ly1 = if has_south_street && h >= 14 {
        h - 4
    } else {
        h - 1
    };
    LotBox::new(lx0, ly0, lx1.max(lx0 + 4), ly1.max(ly0 + 4))
}

fn blocks_any_door(edges: &[MapWallEdge], cand: &MapPropPlacement) -> bool {
    edges.iter().any(|e| {
        matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen)
            && match e.side {
                EdgeSide::West => cand.covers(e.x - 1, e.y) || cand.covers(e.x, e.y),
                EdgeSide::North => cand.covers(e.x, e.y - 1) || cand.covers(e.x, e.y),
            }
    })
}

fn try_push_prop(
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    spawn_pads: &[(f32, f32)],
    w: i32,
    h: i32,
    cand: MapPropPlacement,
) -> bool {
    let fp = cand.footprint_m();
    let (sw_cm, sh_cm) = cand.local_span_cm();
    let (hw_cm, hh_cm) = crate::map_dressing::rotated_aabb_half_extents_cm(
        sw_cm,
        sh_cm,
        cand.rotation_deg,
    );
    let (hw, hh) = (hw_cm / 100.0, hh_cm / 100.0);
    if fp.cx_m - hw < 0.05
        || fp.cy_m - hh < 0.05
        || fp.cx_m + hw > w as f32 - 0.05
        || fp.cy_m + hh > h as f32 - 0.05
    {
        return false;
    }
    if crate::tactical_map_geom::prop_crosses_wall_edge(&cand, edges) {
        return false;
    }
    if !prop_spec(cand.kind).floor_layer || cand.kind.is_vertical_portal() {
        if blocks_any_door(edges, &cand) {
            return false;
        }
        for &(sx, sy) in spawn_pads {
            if crate::tactical_map_geom::prop_intersects_circle(&cand, sx, sy, 0.85) {
                return false;
            }
        }
        for ex in props.iter() {
            if (!prop_spec(ex.kind).floor_layer || ex.kind.is_vertical_portal())
                && crate::tactical_map_geom::props_overlap_continuous(&cand, ex)
            {
                return false;
            }
        }
    }
    props.push(cand);
    true
}

fn generate_community_clinic_annex(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let has_street = h >= 14;
    let lot = safe_interior_lot(w, h, has_street);
    let s = (b.rng.next() & 0xffff_ffff) as u32;

    // Non-overlapping exterior ribbons/zones around `lot`.
    if w >= 12 && lot.x0 >= 3 {
        let alley = LotBox::new(0, lot.y0, lot.x0, lot.y1);
        b.surfaces.push(alley.polygon_feature(SurfaceFeatureKind::BrickPaverWalk, 0, s + 1));
    }
    if w >= 12 && lot.x1 + 3 <= w {
        let util = LotBox::new(lot.x1, lot.y0, w, lot.y1);
        b.surfaces.push(util.polygon_feature(SurfaceFeatureKind::StainedConcreteFloor, 0, s + 2));
    }
    if has_street {
        let walk = LotBox::new(0, lot.y1, w, (lot.y1 + 1).min(h));
        b.surfaces.push(walk.polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 3));
        if lot.y1 + 2 <= h {
            let road = LotBox::new(0, lot.y1 + 1, w, h);
            b.surfaces.push(road.polygon_feature(SurfaceFeatureKind::AsphaltRoad, 0, s + 4));
        }
    }

    // 1F interior zoning & shell.
    let split_x = lot.x0 + lot.w() / 2;
    let split_y = lot.y0 + lot.h() / 2;
    let has_two_wings = lot.w() >= 10 && lot.h() >= 7;
    if has_two_wings {
        b.surfaces.push(
            LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                .polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 5),
        );
        b.surfaces.push(
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::ClinicalVinylFloor, 0, s + 6),
        );
    } else {
        b.surfaces.push(lot.polygon_feature(SurfaceFeatureKind::ClinicalVinylFloor, 0, s + 6));
    }

    enclose_lot(&mut b.edges, w, h, lot, MaterialClass::Brick, 3);
    // Exterior doors: South triage entrance + West service alley door + East emergency exit.
    let front_door_x = lot.x0 + (lot.w() / 4).max(1);
    upsert_edge(
        &mut b.edges,
        w,
        h,
        front_door_x,
        lot.y1,
        EdgeSide::North,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    upsert_edge(
        &mut b.edges,
        w,
        h,
        lot.x0,
        lot.y0 + 1,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::SheetSteel,
    );
    upsert_edge(
        &mut b.edges,
        w,
        h,
        lot.x1,
        lot.y0 + 1,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::SheetSteel,
    );

    if has_two_wings {
        build_v_partition(
            &mut b.edges,
            w,
            h,
            split_x,
            lot.y0,
            lot.y1,
            lot.y0 + lot.h() / 2,
            true,
            MaterialClass::Drywall,
        );
        build_h_partition(
            &mut b.edges,
            w,
            h,
            split_y,
            split_x,
            lot.x1,
            split_x + (lot.x1 - split_x) / 2,
            false,
            MaterialClass::Drywall,
        );
    }

    // Aligned vertical circulation (`Stairwell` + `ServiceElevator` when room allows).
    let portal_x = if has_two_wings { split_x - 2 } else { lot.x0 };
    let portal_y = lot.y0;
    let has_multi_floor = w >= 20 && h >= 14 && has_two_wings;
    let _ = try_push_prop(
        &b.edges,
        &mut b.props,
        &b.spawn_pads_m,
        w,
        h,
        make_prop(PropKind::Stairwell, portal_x.max(lot.x0), portal_y, 0, None, s + 10),
    );
    let _ = try_push_prop(
        &b.edges,
        &mut b.props,
        &b.spawn_pads_m,
        w,
        h,
        make_prop(PropKind::ServiceElevator, split_x.min(lot.x1 - 3), lot.y0, 0, None, s + 11),
    );

    // 1F clinic furnishings (always including HalfCover props cleanly).
    let clinic_props = if has_two_wings {
        vec![
            make_prop(PropKind::NurseStation, lot.x0 + 1, lot.y1 - 3, 0, None, s + 20),
            make_prop(PropKind::ReclinerChair, lot.x0, lot.y0 + 2, 90, None, s + 21),
            make_prop(PropKind::WaterCooler, split_x - 1, lot.y1 - 1, 0, None, s + 22),
            make_prop(PropKind::HospitalBed, lot.x1 - 2, lot.y0, 90, None, s + 23),
            make_prop(PropKind::PrivacyCurtain, lot.x1 - 3, lot.y0 + 1, 0, None, s + 24),
            make_prop(PropKind::TelemetryMonitor, lot.x1 - 1, lot.y0 + 2, 0, None, s + 25),
            make_prop(PropKind::Cot, split_x, split_y + 1, 0, None, s + 26),
            make_prop(PropKind::MedicalCounter, lot.x1 - 2, lot.y1 - 1, 0, Some(2), s + 27),
            make_prop(PropKind::BiohazardBin, lot.x1 - 1, split_y + 1, 0, None, s + 28),
            make_prop(PropKind::LinenCart, split_x + 1, lot.y1 - 1, 0, None, s + 29),
        ]
    } else {
        vec![
            make_prop(PropKind::MedicalCounter, lot.x0 + 1, lot.y0 + 1, 0, Some(1), s + 20),
            make_prop(PropKind::Cot, lot.x1 - 2, lot.y1 - 2, 0, None, s + 21),
            make_prop(PropKind::BiohazardBin, lot.x0 + 1, lot.y1 - 2, 0, None, s + 22),
        ]
    };
    for cand in clinic_props {
        let _ = try_push_prop(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, cand);
    }

    // Exterior props: West service alley, East transformer pad, South street triage van.
    if w >= 12 && lot.x0 >= 3 {
        let _ = b.try_place_cm(PropKind::Dumpster, 120, (lot.y0 as f32 * 100.0) as i32 + 90, 90, None, true);
        let _ = b.try_place_cm(PropKind::StreetDebris, 160, (lot.y1 as f32 * 100.0) as i32 - 120, 15, None, true);
    }
    if w >= 12 && lot.x1 + 3 <= w {
        let _ = b.try_place_cm(PropKind::AcUnit, lot.x1 * 100 + 150, lot.y0 * 100 + 80, 0, None, true);
        let _ = b.try_place_cm(PropKind::AirCompressor, lot.x1 * 100 + 150, lot.y0 * 100 + 200, 90, None, true);
    }
    if has_street {
        let _ = b.try_place_cm(
            PropKind::JerseyBarrier,
            (front_door_x - 2).max(1) * 100 + 50,
            lot.y1 * 100 + 50,
            0,
            None,
            true,
        );
        if lot.y1 + 2 <= h {
            let _ = b.try_place_cm(
                PropKind::UtilityVan,
                (lot.x1 - 3).max(4) * 100 + 50,
                (lot.y1 + 2).min(h - 1) * 100,
                90,
                None,
                true,
            );
        }
        let _ = b.try_place_cm(
            PropKind::TrashCan,
            (front_door_x + 2).min(w - 2) * 100 + 50,
            lot.y1 * 100 + 50,
            0,
            None,
            true,
        );
    }

    trim_floor_props_to_connected(w, h, LotBox::new(0, 0, w, h), &b.edges, &mut b.props);
    prune_occluded_surfaces(&mut b.surfaces, w, h);

    let mut floors = Vec::new();
    let faraday_zones = if has_multi_floor {
        let mut b1_edges = Vec::new();
        enclose_lot(&mut b1_edges, w, h, lot, MaterialClass::ReinforcedConcrete, 0);
        build_v_partition(
            &mut b1_edges,
            w,
            h,
            split_x,
            lot.y0,
            lot.y1,
            lot.y0 + lot.h() / 2,
            false,
            MaterialClass::ArmorSteel,
        );
        let mut b1_props = Vec::new();
        let b1_candidates = [
            make_prop(PropKind::Stairwell, portal_x.max(lot.x0), portal_y, 0, None, s + 40),
            make_prop(PropKind::ServiceElevator, split_x.min(lot.x1 - 3), lot.y0, 0, None, s + 41),
            make_prop(PropKind::AirCompressor, lot.x0, lot.y1 - 1, 0, None, s + 42),
            make_prop(PropKind::SteelShelving, lot.x0, lot.y0 + 2, 0, None, s + 43),
            make_prop(PropKind::LabBench, lot.x0 + 1, lot.y1 - 2, 0, None, s + 44),
            make_prop(PropKind::HazardSpill, split_x - 2, lot.y1 - 2, 0, None, s + 45),
            make_prop(PropKind::ServerRack, lot.x1 - 1, lot.y0 + 1, 0, Some(2), s + 46),
            make_prop(PropKind::ServerRack, lot.x1 - 1, lot.y1 - 2, 0, None, s + 47),
            make_prop(PropKind::FumeHood, split_x + 1, lot.y1 - 1, 0, None, s + 48),
            make_prop(PropKind::TestBench, split_x + 1, lot.y0 + 2, 0, None, s + 49),
        ];
        for cand in b1_candidates {
            let _ = try_push_prop(&b1_edges, &mut b1_props, &b.spawn_pads_m, w, h, cand);
        }
        trim_floor_props_to_connected(w, h, lot, &b1_edges, &mut b1_props);
        let mut b1_surfaces = vec![
            LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                .polygon_feature(SurfaceFeatureKind::StainedConcreteFloor, 0, s + 50),
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::SteelPlateFloor, 0, s + 51),
        ];
        prune_occluded_surfaces(&mut b1_surfaces, w, h);
        floors.push(MapFloorLevel {
            floor_index: -1,
            label: "B1 · Cold Storage Vault".to_string(),
            elevation_cm: -320,
            wall_edges: b1_edges,
            props: b1_props,
            surfaces: b1_surfaces,
            ground_floor_override: Some("concrete_stained".to_string()),
            ground_tint_rgb: None,
        });
        floors.push(MapFloorLevel {
            floor_index: 0,
            label: "1F · Mutual-Aid Clinic".to_string(),
            elevation_cm: 0,
            wall_edges: b.edges.clone(),
            props: b.props.clone(),
            surfaces: b.surfaces.clone(),
            ground_floor_override: None,
            ground_tint_rgb: None,
        });
        vec![MapFaradayZone {
            zone_id: "faraday-clinic-b1-vault".to_string(),
            label: "B1 Cryogenic Seed Vault (Reinforced Faraday Bay)".to_string(),
            floor_index: -1,
            x0_cm: split_x * 100,
            y0_cm: lot.y0 * 100,
            x1_cm: lot.x1 * 100,
            y1_cm: lot.y1 * 100,
            attenuation_db: 90,
            forces_severed_faraday_reflex: true,
        }]
    } else {
        Vec::new()
    };

    let surveillance_nodes = vec![
        MapSurveillanceNode {
            node_id: "cam-clinic-vestibule".to_string(),
            label: "Clinic Entry Biometric Scanner".to_string(),
            node_kind_code: "BIOMETRIC_GAIT_CHECKPOINT".to_string(),
            floor_index: 0,
            x_cm: front_door_x * 100 + 50,
            y_cm: (lot.y1 - 1).max(lot.y0) * 100 + 50,
            facing_deg: 180,
            fov_deg: 110,
            range_cm: 900,
            powered: true,
            visibility: MessageVisibility::Public,
        },
        MapSurveillanceNode {
            node_id: "cam-clinic-street-alpr".to_string(),
            label: "Municipal ALPR Street Camera".to_string(),
            node_kind_code: "ALPR_TRAFFIC_CAMERA".to_string(),
            floor_index: 0,
            x_cm: (lot.x0 * 100 + 50).min(w * 100 - 150),
            y_cm: (lot.y1 * 100 + 50).min(h * 100 - 100),
            facing_deg: 135,
            fov_deg: 120,
            range_cm: 1600,
            powered: true,
            visibility: MessageVisibility::Public,
        },
    ];

    ProceduralSector {
        wall_edges: b.edges,
        props: b.props,
        surfaces: b.surfaces,
        floors,
        active_floor: 0,
        faraday_zones,
        surveillance_nodes,
    }
}

fn generate_corporate_server_bunker(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let has_apron = h >= 14;
    let lot = safe_interior_lot(w, h, has_apron);
    let s = (b.rng.next() & 0xffff_ffff) as u32;

    if w >= 12 && lot.x0 >= 3 {
        b.surfaces.push(
            LotBox::new(0, lot.y0, lot.x0, lot.y1)
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 1),
        );
    }
    if w >= 12 && lot.x1 + 3 <= w {
        b.surfaces.push(
            LotBox::new(lot.x1, lot.y0, w, lot.y1)
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 2),
        );
    }
    if has_apron {
        b.surfaces.push(
            LotBox::new(0, lot.y1, w, h)
                .polygon_feature(SurfaceFeatureKind::AsphaltRoad, 0, s + 3),
        );
    }

    let split_x = lot.x0 + (lot.w() * 2 / 5).max(2);
    let breaker_y = lot.y0 + (lot.h() / 2).max(2);
    let has_wings = lot.w() >= 10 && lot.h() >= 7;

    if has_wings {
        b.surfaces.push(
            LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                .polygon_feature(SurfaceFeatureKind::TerrazzoFloor, 0, s + 4),
        );
        b.surfaces.push(
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::SteelPlateFloor, 0, s + 5),
        );
    } else {
        b.surfaces.push(lot.polygon_feature(SurfaceFeatureKind::SteelPlateFloor, 0, s + 5));
    }

    enclose_lot(&mut b.edges, w, h, lot, MaterialClass::ReinforcedConcrete, 0);
    // West mantrap entry & East egress airlock + South loading hatch.
    upsert_edge(
        &mut b.edges,
        w,
        h,
        lot.x0,
        lot.y0 + lot.h() / 2,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::ArmorSteel,
    );
    upsert_edge(
        &mut b.edges,
        w,
        h,
        lot.x1,
        lot.y0 + lot.h() / 2,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::ArmorSteel,
    );
    upsert_edge(
        &mut b.edges,
        w,
        h,
        lot.x0 + lot.w() / 2,
        lot.y1,
        EdgeSide::North,
        WallEdgeKind::DoorOpen,
        MaterialClass::SheetSteel,
    );

    if has_wings {
        // Glazed control room / server hall divider.
        for y in lot.y0..lot.y1 {
            if y == breaker_y {
                upsert_edge(
                    &mut b.edges,
                    w,
                    h,
                    split_x,
                    y,
                    EdgeSide::West,
                    WallEdgeKind::DoorClosed,
                    MaterialClass::ArmorSteel,
                );
            } else {
                upsert_edge(
                    &mut b.edges,
                    w,
                    h,
                    split_x,
                    y,
                    EdgeSide::West,
                    WallEdgeKind::Window,
                    MaterialClass::Glass,
                );
            }
        }
        build_h_partition(
            &mut b.edges,
            w,
            h,
            breaker_y,
            split_x + 2,
            lot.x1,
            (split_x + 2).min(lot.x1 - 1),
            true,
            MaterialClass::ArmorSteel,
        );
    }

    let cands = if has_wings {
        vec![
            make_prop(PropKind::DeskL, lot.x0 + 1, lot.y0 + 1, 0, None, s + 10),
            make_prop(PropKind::TelemetryMonitor, lot.x0, lot.y1 - 1, 0, None, s + 11),
            make_prop(PropKind::FilingCabinet, split_x - 1, lot.y1 - 1, 0, None, s + 12),
            make_prop(PropKind::FireExtinguisher, split_x - 1, lot.y0, 0, None, s + 13),
            make_prop(PropKind::ServerRack, split_x + 1, lot.y0 + 1, 0, Some(2), s + 14),
            make_prop(PropKind::ServerRack, lot.x1 - 2, lot.y0 + 1, 0, Some(2), s + 15),
            make_prop(PropKind::ElevatedCatwalk, split_x + 2, lot.y0 + 3, 90, Some(3), s + 16),
            make_prop(PropKind::AirCompressor, lot.x1 - 1, breaker_y + 1, 0, None, s + 17),
            make_prop(PropKind::TestBench, split_x + 1, lot.y1 - 1, 0, None, s + 18),
            make_prop(PropKind::SolderingStation, lot.x1 - 2, lot.y1 - 1, 0, None, s + 19),
            make_prop(PropKind::SteelShelving, lot.x0 + 1, lot.y1 - 1, 0, None, s + 20),
        ]
    } else {
        vec![
            make_prop(PropKind::Desk, lot.x0 + 1, lot.y0 + 1, 0, None, s + 10),
            make_prop(PropKind::ServerRack, lot.x1 - 2, lot.y0 + 1, 0, None, s + 11),
            make_prop(PropKind::TestBench, lot.x0 + 1, lot.y1 - 2, 0, None, s + 12),
        ]
    };
    for cand in cands {
        let _ = try_push_prop(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, cand);
    }

    if has_apron {
        let _ = b.try_place_cm(PropKind::JerseyBarrier, (lot.x0 * 100 + 150).min(w * 100 - 150), lot.y1 * 100 + 80, 0, None, true);
        let _ = b.try_place_cm(PropKind::Car, (lot.x1 - 2).max(4) * 100, (lot.y1 + 2).min(h - 1) * 100, 90, None, true);
    }

    trim_floor_props_to_connected(w, h, LotBox::new(0, 0, w, h), &b.edges, &mut b.props);
    prune_occluded_surfaces(&mut b.surfaces, w, h);

    let fz = MapFaradayZone {
        zone_id: "faraday-qpu-hall".to_string(),
        label: "Cryogenic QPU Faraday Vault".to_string(),
        floor_index: 0,
        x0_cm: (if has_wings { split_x } else { lot.x0 }) * 100,
        y0_cm: lot.y0 * 100,
        x1_cm: lot.x1 * 100,
        y1_cm: lot.y1 * 100,
        attenuation_db: 95,
        forces_severed_faraday_reflex: true,
    };

    let surveillance_nodes = vec![
        MapSurveillanceNode {
            node_id: "node-bunker-biometric".to_string(),
            label: "Mantrap Biometric Gait Scanner".to_string(),
            node_kind_code: "BIOMETRIC_GAIT_CHECKPOINT".to_string(),
            floor_index: 0,
            x_cm: (lot.x0 + 1).min(w - 2) * 100 + 50,
            y_cm: (lot.y0 + lot.h() / 2) * 100 + 50,
            facing_deg: 270,
            fov_deg: 100,
            range_cm: 800,
            powered: true,
            visibility: MessageVisibility::Public,
        },
        MapSurveillanceNode {
            node_id: "node-bunker-scada".to_string(),
            label: "Air-Gapped Chiller SCADA Terminal".to_string(),
            node_kind_code: "AIR_GAPPED_SCADA".to_string(),
            floor_index: 0,
            x_cm: (lot.x1 - 1).max(lot.x0) * 100 + 50,
            y_cm: (lot.y1 - 1).max(lot.y0) * 100 + 50,
            facing_deg: 0,
            fov_deg: 360,
            range_cm: 500,
            powered: true,
            visibility: MessageVisibility::Public,
        },
        MapSurveillanceNode {
            node_id: "node-bunker-turret".to_string(),
            label: "Concealed Apron Thermal Sentry".to_string(),
            node_kind_code: "THERMAL_PERIMETER_TURRET".to_string(),
            floor_index: 0,
            x_cm: (lot.x0 + lot.w() / 2) * 100 + 50,
            y_cm: lot.y1 * 100 + 30,
            facing_deg: 180,
            fov_deg: 90,
            range_cm: 1400,
            powered: true,
            visibility: MessageVisibility::GmOnly,
        },
    ];

    ProceduralSector {
        wall_edges: b.edges,
        props: b.props,
        surfaces: b.surfaces,
        floors: Vec::new(),
        active_floor: 0,
        faraday_zones: vec![fz],
        surveillance_nodes,
    }
}

fn generate_satyang_safehouse(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let has_street = h >= 14;
    let lot = safe_interior_lot(w, h, has_street);
    let s = (b.rng.next() & 0xffff_ffff) as u32;

    if w >= 12 && lot.x0 >= 3 {
        b.surfaces.push(
            LotBox::new(0, lot.y0, lot.x0, lot.y1)
                .polygon_feature(SurfaceFeatureKind::BrickPaverWalk, 0, s + 1),
        );
    }
    if has_street {
        b.surfaces.push(
            LotBox::new(0, lot.y1, w, (lot.y1 + 1).min(h))
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 2),
        );
        if lot.y1 + 2 <= h {
            b.surfaces.push(
                LotBox::new(0, lot.y1 + 1, w, h)
                    .polygon_feature(SurfaceFeatureKind::AsphaltRoad, 0, s + 3),
            );
        }
    }

    let split_x = lot.x0 + lot.w() / 2;
    let has_wings = lot.w() >= 10 && lot.h() >= 7;
    if has_wings {
        b.surfaces.push(
            LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                .polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 4),
        );
        b.surfaces.push(
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::CarpetFloor, 0, s + 5),
        );
    } else {
        b.surfaces.push(lot.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 4));
    }

    enclose_lot(&mut b.edges, w, h, lot, MaterialClass::Brick, 3);
    let front_door_x = lot.x0 + 1;
    // Dual egress: South street entrance + North/West rear escape door.
    upsert_edge(
        &mut b.edges,
        w,
        h,
        front_door_x,
        lot.y1,
        EdgeSide::North,
        WallEdgeKind::DoorOpen,
        MaterialClass::SolidWood,
    );
    upsert_edge(
        &mut b.edges,
        w,
        h,
        lot.x0,
        lot.y0 + 1,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::SolidWood,
    );
    if has_wings {
        build_v_partition(
            &mut b.edges,
            w,
            h,
            split_x,
            lot.y0,
            lot.y1,
            lot.y0 + lot.h() / 2,
            false,
            MaterialClass::Drywall,
        );
    }

    let portal_x = lot.x1 - 2;
    let portal_y = lot.y0;
    let has_multi_floor = w >= 20 && h >= 14 && has_wings;
    let cands = if has_wings {
        vec![
            make_prop(PropKind::Stairwell, portal_x.max(split_x + 1), portal_y, 0, None, s + 10),
            make_prop(PropKind::Rug, lot.x0 + 1, lot.y0 + 1, 0, None, s + 11),
            make_prop(PropKind::Desk, lot.x0, lot.y0 + 1, 0, None, s + 12),
            make_prop(PropKind::Bookshelf, split_x - 1, lot.y0, 0, None, s + 13),
            make_prop(PropKind::Sofa, lot.x0 + 1, lot.y1 - 2, 0, None, s + 14),
            make_prop(PropKind::KitchenCounter, split_x + 1, lot.y0, 0, None, s + 15),
            make_prop(PropKind::Fridge, split_x, lot.y0, 0, None, s + 16),
            make_prop(PropKind::Cot, lot.x1 - 2, lot.y1 - 2, 0, None, s + 17),
            make_prop(PropKind::DiningTable, split_x + 1, lot.y1 - 2, 0, None, s + 18),
        ]
    } else {
        vec![
            make_prop(PropKind::Desk, lot.x0 + 1, lot.y0 + 1, 0, None, s + 12),
            make_prop(PropKind::Sofa, lot.x1 - 3, lot.y1 - 2, 0, None, s + 14),
        ]
    };
    for cand in cands {
        let _ = try_push_prop(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, cand);
    }

    if w >= 12 && lot.x0 >= 3 {
        let _ = b.try_place_cm(PropKind::Dumpster, 120, lot.y0 * 100 + 100, 90, None, true);
    }
    if has_street {
        let _ = b.try_place_cm(
            PropKind::AwningCanopy,
            front_door_x * 100 + 100,
            lot.y1 * 100 + 40,
            0,
            None,
            true,
        );
        if lot.y1 + 2 <= h {
            let _ = b.try_place_cm(
                PropKind::Car,
                (lot.x1 - 2).max(4) * 100,
                (lot.y1 + 2).min(h - 1) * 100,
                90,
                None,
                true,
            );
        }
    }

    trim_floor_props_to_connected(w, h, LotBox::new(0, 0, w, h), &b.edges, &mut b.props);
    prune_occluded_surfaces(&mut b.surfaces, w, h);

    let mut floors = Vec::new();
    let faraday_zone = if has_multi_floor {
        let mut b1_edges = Vec::new();
        enclose_lot(&mut b1_edges, w, h, lot, MaterialClass::ReinforcedConcrete, 0);
        build_v_partition(
            &mut b1_edges,
            w,
            h,
            split_x,
            lot.y0,
            lot.y1,
            lot.y0 + lot.h() / 2,
            false,
            MaterialClass::ReinforcedConcrete,
        );
        let mut b1_props = Vec::new();
        for cand in [
            make_prop(PropKind::Stairwell, portal_x.max(split_x + 1), portal_y, 0, None, s + 30),
            make_prop(PropKind::Rug, lot.x0 + 1, lot.y0 + 1, 0, None, s + 31),
            make_prop(PropKind::TestBench, lot.x0, lot.y0 + 1, 0, None, s + 32),
            make_prop(PropKind::SolderingStation, lot.x0, lot.y1 - 2, 0, None, s + 33),
            make_prop(PropKind::Printer3dStation, split_x - 2, lot.y1 - 2, 0, None, s + 34),
            make_prop(PropKind::ServerRack, split_x - 1, lot.y0 + 1, 0, None, s + 35),
            make_prop(PropKind::SteelShelving, lot.x1 - 2, lot.y1 - 1, 0, None, s + 36),
            make_prop(PropKind::WoodCrate, split_x + 1, lot.y1 - 2, 0, None, s + 37),
        ] {
            let _ = try_push_prop(&b1_edges, &mut b1_props, &b.spawn_pads_m, w, h, cand);
        }
        trim_floor_props_to_connected(w, h, lot, &b1_edges, &mut b1_props);
        let b1_surfaces = vec![
            lot.polygon_feature(SurfaceFeatureKind::StainedConcreteFloor, 0, s + 39),
        ];
        floors.push(MapFloorLevel {
            floor_index: -1,
            label: "B1 · Copper-Mesh Cellar".to_string(),
            elevation_cm: -300,
            wall_edges: b1_edges,
            props: b1_props,
            surfaces: b1_surfaces,
            ground_floor_override: Some("concrete_stained".to_string()),
            ground_tint_rgb: None,
        });
        floors.push(MapFloorLevel {
            floor_index: 0,
            label: "1F · Cover Shophouse".to_string(),
            elevation_cm: 0,
            wall_edges: b.edges.clone(),
            props: b.props.clone(),
            surfaces: b.surfaces.clone(),
            ground_floor_override: None,
            ground_tint_rgb: None,
        });
        MapFaradayZone {
            zone_id: "faraday-safehouse-cellar".to_string(),
            label: "B1 Copper-Mesh SIGINT Sanctuary".to_string(),
            floor_index: -1,
            x0_cm: lot.x0 * 100,
            y0_cm: lot.y0 * 100,
            x1_cm: split_x * 100,
            y1_cm: lot.y1 * 100,
            attenuation_db: 85,
            forces_severed_faraday_reflex: true,
        }
    } else {
        MapFaradayZone {
            zone_id: "faraday-safehouse-backroom".to_string(),
            label: "Copper-Mesh Shielded Backroom".to_string(),
            floor_index: 0,
            x0_cm: lot.x0 * 100,
            y0_cm: lot.y0 * 100,
            x1_cm: lot.x1 * 100,
            y1_cm: lot.y1 * 100,
            attenuation_db: 85,
            forces_severed_faraday_reflex: true,
        }
    };

    ProceduralSector {
        wall_edges: b.edges,
        props: b.props,
        surfaces: b.surfaces,
        floors,
        active_floor: 0,
        faraday_zones: vec![faraday_zone],
        surveillance_nodes: Vec::new(),
    }
}

fn generate_brownout_logistics_depot(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let has_street = h >= 14;
    let lot = safe_interior_lot(w, h, has_street);
    let s = (b.rng.next() & 0xffff_ffff) as u32;

    if has_street {
        b.surfaces.push(
            LotBox::new(0, lot.y1, w, (lot.y1 + 1).min(h))
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 1),
        );
        if lot.y1 + 2 <= h {
            b.surfaces.push(
                LotBox::new(0, lot.y1 + 1, w, h)
                    .polygon_feature(SurfaceFeatureKind::AsphaltRoad, 0, s + 2),
            );
        }
    }

    // Divide interior lot into West Logistics Warehouse and East Fenced 500 kV Substation Yard.
    let split_x = lot.x0 + (lot.w() * 3 / 5).max(2);
    let has_substation = lot.w() >= 10 && lot.h() >= 7;
    if has_substation {
        b.surfaces.push(
            LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                .polygon_feature(SurfaceFeatureKind::StainedConcreteFloor, 0, s + 3),
        );
        b.surfaces.push(
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::GravelRoad, 0, s + 4),
        );
        // Warehouse shell (`ConcreteBlock`) + Substation fenced perimeter (`SheetSteel` with wide gate).
        let wh_lot = LotBox::new(lot.x0, lot.y0, split_x, lot.y1);
        enclose_lot(&mut b.edges, w, h, wh_lot, MaterialClass::ConcreteBlock, 4);
        let sub_lot = LotBox::new(split_x, lot.y0, lot.x1, lot.y1);
        enclose_lot(&mut b.edges, w, h, sub_lot, MaterialClass::SheetSteel, 0);
        // Open loading bays & substation access gate.
        let door_y = lot.y0 + lot.h() / 2;
        upsert_edge(&mut b.edges, w, h, lot.x0, door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        upsert_edge(&mut b.edges, w, h, split_x, door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        upsert_edge(&mut b.edges, w, h, lot.x1, door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        upsert_edge(&mut b.edges, w, h, lot.x0 + 2, lot.y1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
    } else {
        b.surfaces.push(lot.polygon_feature(SurfaceFeatureKind::StainedConcreteFloor, 0, s + 3));
        enclose_lot(&mut b.edges, w, h, lot, MaterialClass::ConcreteBlock, 4);
        upsert_edge(
            &mut b.edges,
            w,
            h,
            lot.x0 + lot.w() / 2,
            lot.y1,
            EdgeSide::North,
            WallEdgeKind::DoorOpen,
            MaterialClass::SheetSteel,
        );
    }

    let cands = if has_substation {
        vec![
            make_prop(PropKind::PalletRack, lot.x0 + 1, lot.y0 + 1, 0, None, s + 10),
            make_prop(PropKind::ConveyorBelt, lot.x0 + 1, lot.y1 - 2, 0, Some(3), s + 11),
            make_prop(PropKind::Forklift, split_x - 2, lot.y0 + 1, 90, None, s + 12),
            make_prop(PropKind::WoodCrate, split_x - 2, lot.y1 - 2, 0, None, s + 13),
            make_prop(PropKind::Desk, lot.x0 + 1, lot.y0 + 3, 0, None, s + 14),
            make_prop(PropKind::FilingCabinet, lot.x0, lot.y1 - 1, 0, None, s + 15),
            make_prop(PropKind::IndustrialPress, split_x + 1, lot.y0 + 1, 0, None, s + 16),
            make_prop(PropKind::AirCompressor, lot.x1 - 2, lot.y0 + 1, 0, None, s + 17),
            make_prop(PropKind::AcUnit, lot.x1 - 2, lot.y1 - 2, 0, None, s + 18),
            make_prop(PropKind::JerseyBarrier, split_x + 1, lot.y1 - 1, 0, None, s + 19),
            make_prop(PropKind::HazardSpill, split_x + 2, lot.y0 + 2, 0, None, s + 20),
        ]
    } else {
        vec![
            make_prop(PropKind::PalletRack, lot.x0 + 1, lot.y0 + 1, 0, None, s + 10),
            make_prop(PropKind::WoodCrate, lot.x1 - 2, lot.y1 - 2, 0, None, s + 11),
            make_prop(PropKind::AcUnit, lot.x0 + 1, lot.y1 - 2, 0, None, s + 12),
        ]
    };
    for cand in cands {
        let _ = try_push_prop(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, cand);
    }

    if w >= 16 && h >= 14 {
        let _ = b.try_place_cm(PropKind::ShippingContainer, (w - 3) * 100 + 50, lot.y0 * 100 + 200, 90, None, true);
    }
    if has_street {
        let can_x = (lot.x0 + 2).min(w - 3) * 100;
        let _ = b.try_place_cm(PropKind::AwningCanopy, can_x, lot.y1 * 100 + 40, 0, None, true);
        let _ = b.try_place_cm(PropKind::ParkBench, can_x + 200, lot.y1 * 100 + 45, 0, None, true);
        let _ = b.try_place_cm(PropKind::WaterCooler, can_x - 100, lot.y1 * 100 + 45, 0, None, true);
        if lot.y1 + 2 <= h {
            let _ = b.try_place_cm(
                PropKind::UtilityVan,
                (lot.x1 - 3).max(5) * 100,
                (lot.y1 + 2).min(h - 1) * 100,
                90,
                None,
                true,
            );
        }
    }

    trim_floor_props_to_connected(w, h, LotBox::new(0, 0, w, h), &b.edges, &mut b.props);
    prune_occluded_surfaces(&mut b.surfaces, w, h);

    let surveillance_nodes = vec![
        MapSurveillanceNode {
            node_id: "cam-depot-alpr".to_string(),
            label: "Depot Gatehouse ALPR Camera".to_string(),
            node_kind_code: "ALPR_TRAFFIC_CAMERA".to_string(),
            floor_index: 0,
            x_cm: lot.x0 * 100 + 50,
            y_cm: (lot.y1 * 100 + 50).min(h * 100 - 100),
            facing_deg: 180,
            fov_deg: 120,
            range_cm: 1800,
            powered: true,
            visibility: MessageVisibility::Public,
        },
        MapSurveillanceNode {
            node_id: "arr-depot-acoustic".to_string(),
            label: "500 kV Substation Acoustic Array".to_string(),
            node_kind_code: "ACOUSTIC_GUNSHOT_ARRAY".to_string(),
            floor_index: 0,
            x_cm: (if has_substation { split_x + 1 } else { lot.x0 + 1 }) * 100 + 50,
            y_cm: (lot.y0 + 1) * 100 + 50,
            facing_deg: 0,
            fov_deg: 360,
            range_cm: 1800,
            powered: true,
            visibility: MessageVisibility::Public,
        },
    ];

    ProceduralSector {
        wall_edges: b.edges,
        props: b.props,
        surfaces: b.surfaces,
        floors: Vec::new(),
        active_floor: 0,
        faraday_zones: Vec::new(),
        surveillance_nodes,
    }
}

/// Generates one of the 4 canonical *Awakened* 2035 procedural tactical battlemaps.
#[must_use]
pub(crate) fn generate_awakened_sector(
    preset: SectorPreset,
    w: i32,
    h: i32,
    seed: i64,
) -> ProceduralSector {
    match preset {
        SectorPreset::CommunityClinicAnnex => generate_community_clinic_annex(w, h, seed),
        SectorPreset::CorporateServerBunker => generate_corporate_server_bunker(w, h, seed),
        SectorPreset::SatyangSafehouse => generate_satyang_safehouse(w, h, seed),
        SectorPreset::BrownoutLogisticsDepot => generate_brownout_logistics_depot(w, h, seed),
        _ => ProceduralSector::default(),
    }
}
