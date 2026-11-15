//! Phase 4 unit & integration tests for [`crate::map_world_graph`], stitched world grids,
//! road-gated neighbor transitions, walkability, and multi-sector hover inspection.

use std::collections::BTreeSet;

use satya_schema::{EconomicTier, EdgeSide, MapContinuationDir, WallEdgeKind};

use crate::map_continuation::continue_procedural_map;
use crate::map_world_graph::preview_continuation_tooltip;
use crate::tactical_map::{SectorPreset, build_procedural_map_request};
use crate::tactical_map_geom::all_walkable_areas_connected;
use crate::tactical_map_inspect::inspect_map_point;

#[test]
fn downtown_commercial_continues_commercial_then_steps_down_to_residential() {
    let mut commercial_step1 = 0_usize;
    let mut residential_at_step3 = 0_usize;
    let seeds = [7_i64, 19, 42, 77, 108, 202, 313, 404, 515, 777];

    for &seed in &seeds {
        let mut map = build_procedural_map_request(
            "m-comm",
            SectorPreset::DowntownCommercial,
            None,
            20,
            14,
            seed,
        );
        assert_eq!(map.world_sectors.len(), 1);
        assert_eq!(map.world_sectors[0].sector_x, 0);
        assert_eq!(map.world_sectors[0].zone_depth, 0);

        map = continue_procedural_map(&map, MapContinuationDir::East, None)
            .unwrap()
            .map;
        let s1 = map
            .world_sectors
            .iter()
            .find(|c| c.sector_x == 1 && c.sector_y == 0)
            .expect("step 1 sector (1, 0)");
        if matches!(s1.preset.as_str(), "downtown_commercial" | "urban_crossroads") {
            commercial_step1 += 1;
        }

        map = continue_procedural_map(&map, MapContinuationDir::East, None)
            .unwrap()
            .map;
        map = continue_procedural_map(&map, MapContinuationDir::East, None)
            .unwrap()
            .map;
        let s3 = map
            .world_sectors
            .iter()
            .find(|c| c.sector_x == 3 && c.sector_y == 0)
            .expect("step 3 sector (3, 0)");
        assert_eq!(map.world_sectors.len(), 4);
        assert_eq!(s3.origin_x, 60);
        assert_eq!(s3.width, 20);
        // Any step starting from zone_depth >= 2 steps down; at step 3, check either step 2 or 3 stepped down.
        let any_stepped_down = map.world_sectors.iter().skip(1).any(|c| {
            matches!(c.preset.as_str(), "urban_residential" | "suburban_neighborhood")
                && c.economic_tier == Some(EconomicTier::UpperMiddleClass)
        });
        if any_stepped_down {
            residential_at_step3 += 1;
        }
        assert!(all_walkable_areas_connected(
            map.grid_width,
            map.grid_height,
            &map.wall_edges,
            &map.props
        ));
    }

    assert!(
        commercial_step1 >= 6,
        "Expected majority of step-1 continuations from DowntownCommercial to remain commercial, got {commercial_step1}/10"
    );
    assert!(
        residential_at_step3 >= 8,
        "Expected downtown commercial runs to step down into upper-middle residential by distance 2-3, got {residential_at_step3}/10"
    );
}

#[test]
fn wooded_park_road_border_transitions_to_residential_or_rural_while_non_road_stays_natural() {
    let mut road_oriented_matches = 0_usize;
    let mut tested_seeds = 0_usize;

    for seed in [11_i64, 22, 33, 42, 55, 77, 88, 99] {
        let park = build_procedural_map_request("m-park", SectorPreset::WoodedPark, None, 36, 26, seed);
        let root = &park.world_sectors[0];
        let north_road = root.boundary.north.road_parallel;
        let south_road = root.boundary.south.road_parallel;
        if north_road == south_road {
            continue;
        }
        tested_seeds += 1;
        let (road_dir, non_road_dir) = if north_road {
            (MapContinuationDir::North, MapContinuationDir::South)
        } else {
            (MapContinuationDir::South, MapContinuationDir::North)
        };

        let across_road = continue_procedural_map(&park, road_dir, None).unwrap().map;
        let added = across_road
            .world_sectors
            .iter()
            .find(|c| (c.sector_x, c.sector_y) == road_dir.unit_step())
            .expect("added neighbor across road");
        assert!(
            matches!(
                added.preset.as_str(),
                "urban_residential" | "suburban_neighborhood" | "rural_countryside"
            ),
            "seed {seed}: across-road neighbor was {}",
            added.preset
        );
        // When a road-facing neighbor has a parallel border road, orient_sector_to_shared_road aligns it to face the park.
        if added.boundary.edge(road_dir.opposite()).road_parallel {
            road_oriented_matches += 1;
        }

        let natural_side = continue_procedural_map(&park, non_road_dir, None).unwrap().map;
        let nat_added = natural_side
            .world_sectors
            .iter()
            .find(|c| (c.sector_x, c.sector_y) == non_road_dir.unit_step())
            .expect("added neighbor on non-road border");
        assert!(
            matches!(
                nat_added.preset.as_str(),
                "wooded_park" | "woodland_creek" | "rural_countryside"
            ),
            "seed {seed}: non-road side was {}",
            nat_added.preset
        );
    }

    assert!(tested_seeds >= 6, "Expected distinct N/S road border on WoodedPark seeds");
    assert_eq!(
        road_oriented_matches, tested_seeds,
        "Every neighbor across WoodedPark's road border must orient its street toward the shared border seam"
    );
}

#[test]
fn multi_step_2x2_and_west_north_shifts_preserve_world_sector_bounds_walkability_and_hover() {
    let mut map = build_procedural_map_request("m-grid", SectorPreset::WoodedPark, None, 20, 14, 42);
    map = continue_procedural_map(&map, MapContinuationDir::East, None).unwrap().map;
    map = continue_procedural_map(&map, MapContinuationDir::North, None).unwrap().map;
    map = continue_procedural_map(&map, MapContinuationDir::West, None).unwrap().map;

    assert_eq!((map.grid_width, map.grid_height), (60, 28));
    assert_eq!(map.world_sectors.len(), 6);

    // Every arena cell (0..60, 0..28) belongs to exactly one WorldSectorCell (100% disjoint tiling).
    let mut coords = BTreeSet::new();
    for cell in &map.world_sectors {
        assert!(coords.insert((cell.sector_x, cell.sector_y)), "duplicate world coord {cell:?}");
        assert!(cell.origin_x >= 0 && cell.origin_x + cell.width <= map.grid_width);
        assert!(cell.origin_y >= 0 && cell.origin_y + cell.height <= map.grid_height);
    }
    for gy in 0..map.grid_height {
        for gx in 0..map.grid_width {
            let matches_count = map.world_sectors.iter().filter(|c| c.contains_cell(gx, gy)).count();
            assert_eq!(matches_count, 1, "cell ({gx},{gy}) covered {matches_count} times");
        }
    }

    assert!(all_walkable_areas_connected(
        map.grid_width,
        map.grid_height,
        &map.wall_edges,
        &map.props
    ));

    // Verify no floating freestanding doors exist on outdoor seams without adjacent structural walls.
    let origin = map
        .world_sectors
        .iter()
        .find(|c| (c.sector_x, c.sector_y) == (0, 0))
        .expect("origin shifted cleanly to (20, 14)");
    assert_eq!((origin.origin_x, origin.origin_y), (20, 14));
    let north_seam_doors = map.wall_edges.iter().filter(|e| {
        e.side == EdgeSide::North
            && e.y == 14
            && e.x >= 20
            && e.x < 40
            && matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen)
    });
    for door in north_seam_doors {
        let has_neighbor_wall = map.wall_edges.iter().any(|w| {
            w.kind == WallEdgeKind::Wall
                && ((w.x - door.x).abs() <= 1 && (w.y - door.y).abs() <= 1)
        });
        assert!(has_neighbor_wall, "Unexpected freestanding door on outdoor seam: {door:?}");
    }

    // Hovering bare ground inside two distinct WorldSectorCells reports their respective local sector coordinates.
    let c_origin = inspect_map_point(&map, "Meadow Grass Turf", 20.5, 14.5).expect("origin ground");
    let c_west = inspect_map_point(&map, "Meadow Grass Turf", 0.5, 0.5).expect("nw ground");
    assert_ne!(c_origin.subtitle, c_west.subtitle);
    assert!(c_origin.subtitle.contains("(0, 0)") || c_origin.title.len() > 5);

    let tip = preview_continuation_tooltip(&map, MapContinuationDir::South);
    assert!(tip.starts_with("Continue South (+14m, 3 sectors) → "), "tip={tip}");
}

#[test]
fn phase5_sector_preset_groups_and_aliases_roundtrip() {
    use std::collections::BTreeSet;
    use crate::tactical_map::{SECTOR_PRESET_GROUPS, SectorPreset};

    let mut seen = BTreeSet::new();
    let mut flat = Vec::new();
    for grp in SECTOR_PRESET_GROUPS {
        assert!(!grp.label.is_empty());
        assert!(!grp.presets.is_empty());
        for &p in grp.presets {
            assert!(seen.insert(p.as_str()), "duplicate preset {} in SECTOR_PRESET_GROUPS", p.as_str());
            flat.push(p);
        }
    }
    assert_eq!(flat.len(), SectorPreset::ALL.len());
    assert_eq!(flat.len(), 25);
    for p in SectorPreset::ALL {
        assert!(seen.contains(p.as_str()), "missing preset {} from SECTOR_PRESET_GROUPS", p.as_str());
        assert_eq!(SectorPreset::from_code_str(p.as_str()), Some(p), "roundtrip failed for {}", p.as_str());
    }
    let checks = [
        ("hospital", SectorPreset::Hospital),
        ("medlab", SectorPreset::MedLabSector),
        ("medical", SectorPreset::MedLabSector),
        ("warehouse", SectorPreset::Warehouse),
        ("cargo", SectorPreset::DockingBay),
        ("plant", SectorPreset::ManufacturingPlant),
        ("factory", SectorPreset::ManufacturingPlant),
        ("plaza", SectorPreset::ShoppingPlaza),
        ("mall", SectorPreset::ShoppingMall),
        ("farm", SectorPreset::Farms),
        ("botanical", SectorPreset::BotanicalGarden),
    ];
    for (alias, expected) in checks {
        assert_eq!(SectorPreset::from_code_str(alias), Some(expected), "alias={alias}");
    }
}

#[test]
fn phase5_hospital_and_shopping_mall_multi_floor_and_props() {
    use satya_schema::{PropKind, SurfaceFeatureKind};
    use crate::map_gen_residential::{LotBox, floor_lot_connected};
    use crate::tactical_map::{SectorPreset, generate_procedural_sector};

    let hosp = generate_procedural_sector(SectorPreset::Hospital, 48, 36, 42);
    assert_eq!(hosp.floors.len(), 2);
    let f1_stairs: Vec<_> = hosp.floors[0]
        .props
        .iter()
        .filter(|p| p.kind == PropKind::Stairwell)
        .map(|p| p.center_cm())
        .collect();
    let f2_stairs: Vec<_> = hosp.floors[1]
        .props
        .iter()
        .filter(|p| p.kind == PropKind::Stairwell)
        .map(|p| p.center_cm())
        .collect();
    assert!(!f1_stairs.is_empty());
    assert_eq!(f1_stairs, f2_stairs, "Hospital 1F/2F stairwells must align");
    assert!(floor_lot_connected(
        48,
        36,
        LotBox::new(4, 1, 44, 32),
        &hosp.floors[1].wall_edges,
        &hosp.floors[1].props
    ));
    for kind in [
        PropKind::HospitalBed,
        PropKind::SurgicalTable,
        PropKind::ImagingScanner,
        PropKind::PrivacyCurtain,
    ] {
        assert!(
            hosp.floors.iter().any(|fl| fl.props.iter().any(|p| p.kind == kind)),
            "Hospital missing {kind:?}"
        );
    }

    let mall = generate_procedural_sector(SectorPreset::ShoppingMall, 48, 36, 42);
    assert_eq!(mall.floors.len(), 2);
    let m1_stairs: Vec<_> = mall.floors[0]
        .props
        .iter()
        .filter(|p| p.kind == PropKind::Stairwell)
        .map(|p| p.center_cm())
        .collect();
    let m2_stairs: Vec<_> = mall.floors[1]
        .props
        .iter()
        .filter(|p| p.kind == PropKind::Stairwell)
        .map(|p| p.center_cm())
        .collect();
    assert!(!m1_stairs.is_empty());
    assert_eq!(m1_stairs, m2_stairs, "ShoppingMall 1F/2F stairwells must align");
    assert!(floor_lot_connected(
        48,
        36,
        LotBox::new(0, 0, 48, 36),
        &mall.floors[1].wall_edges,
        &mall.floors[1].props
    ));
    assert!(mall.props.iter().any(|p| p.kind == PropKind::MallKiosk));
    assert!(mall.props.iter().any(|p| p.kind == PropKind::PlanterTable));
    assert!(mall.floors[1].surfaces.iter().any(|s| s.kind == SurfaceFeatureKind::BridgeDeck));
}

#[test]
fn phase5_agro_industrial_sectors_and_plaza_transition() {
    use satya_schema::{EconomicTier, MaterialClass, PropKind, SurfaceFeatureKind};
    use crate::tactical_map::generate_procedural_sector;

    let farms = generate_procedural_sector(SectorPreset::Farms, 48, 36, 42);
    assert_eq!(farms.floors.len(), 2);
    assert!(farms.props.iter().any(|p| p.kind == PropKind::CropRow));
    assert!(farms.surfaces.iter().any(|s| s.kind == SurfaceFeatureKind::DrainageDitch));
    assert!(farms.surfaces.iter().any(|s| s.kind == SurfaceFeatureKind::BridgeDeck));

    let bot = generate_procedural_sector(SectorPreset::BotanicalGarden, 48, 36, 42);
    assert!(bot.wall_edges.iter().any(|e| e.material == MaterialClass::Glass));
    assert!(bot.props.iter().any(|p| p.kind == PropKind::PlanterTable));
    assert!(bot.surfaces.iter().any(|s| s.kind == SurfaceFeatureKind::CreekChannel));
    assert!(bot.surfaces.iter().any(|s| s.kind == SurfaceFeatureKind::BridgeDeck));

    let plant = generate_procedural_sector(SectorPreset::ManufacturingPlant, 48, 36, 42);
    assert!(plant.props.iter().any(|p| p.kind == PropKind::ConveyorBelt));
    assert!(plant.props.iter().any(|p| p.kind == PropKind::IndustrialPress));

    let wh = generate_procedural_sector(SectorPreset::Warehouse, 48, 36, 42);
    assert!(wh.props.iter().any(|p| p.kind == PropKind::PalletRack));
    assert!(wh.props.iter().any(|p| p.kind == PropKind::Forklift));
    assert!(wh.props.iter().any(|p| p.kind == PropKind::ShippingContainer));

    let mut plaza = build_procedural_map_request("plaza", SectorPreset::ShoppingPlaza, None, 36, 26, 42);
    for _ in 0..3 {
        plaza = continue_procedural_map(&plaza, MapContinuationDir::East, None).unwrap().map;
        assert!(all_walkable_areas_connected(
            plaza.grid_width,
            plaza.grid_height,
            &plaza.wall_edges,
            &plaza.props
        ));
    }
    let reached_upper_mid_res = plaza.world_sectors.iter().any(|c| {
        matches!(c.preset.as_str(), "urban_residential" | "suburban_neighborhood")
            && c.economic_tier == Some(EconomicTier::UpperMiddleClass)
    });
    assert!(
        reached_upper_mid_res,
        "Expected ShoppingPlaza to transition to UpperMiddleClass residential within 3 steps, got: {:?}",
        plaza.world_sectors.iter().map(|c| (&c.preset, c.economic_tier)).collect::<Vec<_>>()
    );
}

