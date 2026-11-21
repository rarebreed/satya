//! Consolidated continuous metric tactical map, procedural continuation, interior generation,
//! and zoned multi-floor tests (`tests_tactical_map`).

use std::collections::BTreeSet;

use satya_schema::{
    CoverGrade, EconomicTier, EdgeSide, MapContinuationDir, MapWallEdge, MaterialClass, PropKind,
    SurfaceFeatureKind, WallEdgeKind, ZoningContext,
};

use crate::map_continuation::{MAX_STITCHED_MAP_W, can_continue_map, continue_procedural_map};
use crate::map_dressing::{PropObstruction, make_prop, prop_spec, props_share_covered_cell};
use crate::map_gen_residential::{LotBox, floor_lot_connected};
use crate::map_gen_zoning::generate_zoned_sector;
use crate::map_interior_gen::{InteriorStyle, generate_furnished_interior};
use crate::tactical_map::{
    GaitReachQuery, ProceduralSector, SectorPreset, TacticalSceneView,
    build_procedural_map_request, build_procedural_map_request_with_options,
    compute_reachable_continuous, evaluate_los_continuous, generate_procedural_sector,
};
use crate::tactical_map_geom::{
    TOKEN_RADIUS_M, all_walkable_areas_connected, circle_collides_obstacles, tree_canopy_radius_m,
};
use satya_engine::environment_vehicles::CombatMovementGait;

#[test]
fn test_procedural_sector_presets_and_continuous_los_cover() {
    for preset in SectorPreset::ALL {
        let sec = generate_procedural_sector(preset, 20, 14, 42);
        if preset.has_structural_wall_edges() {
            assert!(!sec.wall_edges.is_empty(), "{preset:?} missing wall_edges");
        }
        assert!(!sec.props.is_empty(), "{preset:?} missing props");
        assert!(
            sec.props
                .iter()
                .any(|p| prop_spec(p.kind).obstruction == PropObstruction::HalfCover),
            "{preset:?} missing half cover prop"
        );
        assert!(
            all_walkable_areas_connected(20, 14, &sec.wall_edges, &sec.props),
            "{preset:?} disconnected walkable area"
        );
        // West and East deployment pads must remain clear of blocking obstacles.
        assert!(!circle_collides_obstacles(
            20.0,
            14.0,
            &sec.wall_edges,
            &sec.props,
            2.5,
            7.5,
            TOKEN_RADIUS_M
        ));
        assert!(!circle_collides_obstacles(
            20.0,
            14.0,
            &sec.wall_edges,
            &sec.props,
            17.5,
            7.5,
            TOKEN_RADIUS_M
        ));
    }

    // Clear LoS
    let empty = TacticalSceneView {
        width_m: 12.0,
        height_m: 8.0,
        edges: &[],
        props: &[],
        surfaces: &[],
    };
    let clear = evaluate_los_continuous(&empty, (2.5, 4.5), (9.5, 4.5));
    assert!(clear.has_los);
    assert_eq!(clear.cover_grade, CoverGrade::Clear);
    assert_eq!(clear.recommended_delta_ext, 0);
    assert!((clear.distance_m - 7.0).abs() < 0.01);

    // Mid-field JerseyBarrier -> LowCover (+4)
    let barrier = make_prop(PropKind::JerseyBarrier, 6, 4, 90, Some(1), 0);
    let low_scene = TacticalSceneView {
        width_m: 12.0,
        height_m: 8.0,
        edges: &[],
        props: &[barrier],
        surfaces: &[],
    };
    let low = evaluate_los_continuous(&low_scene, (2.5, 4.5), (9.5, 4.5));
    assert!(
        low.has_los && low.cover_grade == CoverGrade::LowCover && low.recommended_delta_ext == 4
    );

    // Braced right behind barrier -> Clear outgoing shot!
    let braced = evaluate_los_continuous(&low_scene, (5.8, 4.5), (9.5, 4.5));
    assert_eq!(braced.cover_grade, CoverGrade::Clear);

    // Target crouching inside a DefensiveTrench > 1.2m away -> LowCover (+4)
    let trench = satya_schema::MapSurfaceFeature {
        kind: SurfaceFeatureKind::DefensiveTrench,
        shape: satya_schema::SurfaceShapeKind::RibbonSpline,
        points_cm: vec![
            satya_schema::MapPointCm::new(900, 200),
            satya_schema::MapPointCm::new(900, 700),
        ],
        width_cm: 180,
        border_style_cm: 0,
        variant_seed: 1,
        tint_rgb: None,
    };
    let trench_scene = TacticalSceneView {
        width_m: 12.0,
        height_m: 8.0,
        edges: &[],
        props: &[],
        surfaces: &[trench],
    };
    let trench_cover = evaluate_los_continuous(&trench_scene, (2.5, 4.5), (9.0, 4.5));
    assert_eq!(trench_cover.cover_grade, CoverGrade::LowCover);
    assert_eq!(trench_cover.recommended_delta_ext, 4);

    // Single StructuralPillar with open sides -> corner-peek HighCover (+6)
    let pillar = make_prop(PropKind::StructuralPillar, 6, 4, 0, None, 0);
    let pillar_scene = TacticalSceneView {
        width_m: 12.0,
        height_m: 8.0,
        edges: &[],
        props: &[pillar],
        surfaces: &[],
    };
    let corner = evaluate_los_continuous(&pillar_scene, (5.6, 4.5), (9.5, 4.5));
    assert!(
        corner.has_los
            && corner.cover_grade == CoverGrade::HighCover
            && corner.recommended_delta_ext == 6
    );

    // Continuous brick wall across y=0..8 at x=6 -> completely Blocked
    let wall: Vec<MapWallEdge> = (0..8)
        .map(|y| MapWallEdge {
            x: 6,
            y,
            side: EdgeSide::West,
            kind: WallEdgeKind::Wall,
            material: MaterialClass::Brick,
        })
        .collect();
    let blocked_scene = TacticalSceneView {
        width_m: 12.0,
        height_m: 8.0,
        edges: &wall,
        props: &[],
        surfaces: &[],
    };
    let blocked = evaluate_los_continuous(&blocked_scene, (2.5, 4.5), (9.5, 4.5));
    assert!(!blocked.has_los && blocked.cover_grade == CoverGrade::Blocked);
}

#[test]
fn test_metric_gait_reach_scales_continuously() {
    let scene = TacticalSceneView {
        width_m: 20.0,
        height_m: 14.0,
        edges: &[],
        props: &[],
        surfaces: &[],
    };
    let walk = compute_reachable_continuous(
        &scene,
        &[],
        GaitReachQuery {
            start_m: (5.5, 7.5),
            spd_eff: 10,
            hgt_m: 1.75,
            gait: CombatMovementGait::CombatGlideTacticalWalk,
            tick_budget: 8,
        },
    );
    let sprint = compute_reachable_continuous(
        &scene,
        &[],
        GaitReachQuery {
            start_m: (5.5, 7.5),
            spd_eff: 10,
            hgt_m: 1.75,
            gait: CombatMovementGait::AllOutSprint,
            tick_budget: 8,
        },
    );
    let walk_count = walk.cost_mm.iter().filter(|&&c| c < i32::MAX).count();
    let sprint_count = sprint.cost_mm.iter().filter(|&&c| c < i32::MAX).count();
    assert!(walk_count > 16);
    assert!(sprint_count > walk_count);
    assert!(!walk.merged_row_runs().is_empty());
    assert!(walk.ticks_at_m(6.0, 7.5).is_some());
}

fn topology_signature(sec: &ProceduralSector) -> (usize, usize, u64, u64) {
    let mut edge_hash = 0xcbf2_9ce4_8422_2325_u64;
    for e in &sec.wall_edges {
        for v in [e.x as u64, e.y as u64, e.side as u64, e.kind as u64] {
            edge_hash ^= v;
            edge_hash = edge_hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    let mut surf_hash = 0xcbf2_9ce4_8422_2325_u64;
    for s in &sec.surfaces {
        let tag = [
            s.kind as u64,
            s.shape as u64,
            s.width_cm as u64,
            s.points_cm.len() as u64,
        ];
        for v in tag {
            surf_hash ^= v;
            surf_hash = surf_hash.wrapping_mul(0x0100_0000_01b3);
        }
        for pt in &s.points_cm {
            for coord in [pt.x_cm as u64, pt.y_cm as u64] {
                surf_hash ^= coord;
                surf_hash = surf_hash.wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    (
        sec.wall_edges.len(),
        sec.surfaces.len(),
        edge_hash,
        surf_hash,
    )
}

#[test]
fn phase2_large_scale_presets_are_connected_and_clear_spawn_pads() {
    for &(w, h) in &[(24, 18), (36, 26), (48, 36), (64, 48)] {
        let wy = (h / 2) as f32 + 0.5;
        for preset in SectorPreset::ALL {
            let sec = generate_procedural_sector(preset, w, h, 42);
            assert!(
                !sec.surfaces.is_empty(),
                "{preset:?} {w}x{h} missing surfaces"
            );
            assert!(
                all_walkable_areas_connected(w, h, &sec.wall_edges, &sec.props),
                "{preset:?} {w}x{h} disconnected walkable area"
            );
            assert!(
                !circle_collides_obstacles(
                    w as f32,
                    h as f32,
                    &sec.wall_edges,
                    &sec.props,
                    2.5,
                    wy,
                    TOKEN_RADIUS_M
                ),
                "{preset:?} {w}x{h} blocked West spawn pad"
            );
            assert!(
                !circle_collides_obstacles(
                    w as f32,
                    h as f32,
                    &sec.wall_edges,
                    &sec.props,
                    w as f32 - 2.5,
                    wy,
                    TOKEN_RADIUS_M
                ),
                "{preset:?} {w}x{h} blocked East spawn pad"
            );
        }
    }
    for preset in [SectorPreset::UrbanCrossroads, SectorPreset::GardenEstate] {
        let sec = generate_procedural_sector(preset, 96, 72, 42);
        assert!(
            all_walkable_areas_connected(96, 72, &sec.wall_edges, &sec.props),
            "{preset:?} 96x72 disconnected walkable area"
        );
    }
}

#[test]
fn phase2_multi_seed_topologies_produce_distinct_layouts() {
    for preset in [
        SectorPreset::GardenEstate,
        SectorPreset::WoodedPark,
        SectorPreset::WoodlandCreek,
        SectorPreset::RoadsideDitch,
        SectorPreset::UrbanCrossroads,
        SectorPreset::ResidentialApartment,
    ] {
        let distinct: std::collections::HashSet<_> = (1_i64..=12)
            .map(|seed| topology_signature(&generate_procedural_sector(preset, 36, 26, seed)))
            .collect();
        assert!(
            distinct.len() >= 3,
            "{preset:?} expected >= 3 distinct topologies across seeds 1..=12, got {}",
            distinct.len()
        );
    }
}

#[test]
fn continue_all_presets_across_all_four_cardinal_edges() {
    for preset in SectorPreset::ALL {
        let base = build_procedural_map_request("m-base", preset, None, 20, 14, 42);
        let base_tok_pos = base.token_pos_cm(&base.tokens[0]);
        for dir in MapContinuationDir::ALL {
            let outcome = continue_procedural_map(&base, dir, None)
                .unwrap_or_else(|e| panic!("{preset:?} {dir:?} failed: {e}"));
            let expected_size = match dir {
                MapContinuationDir::East | MapContinuationDir::West => (40, 14),
                MapContinuationDir::North | MapContinuationDir::South => (20, 28),
            };
            assert_eq!(
                (outcome.map.grid_width, outcome.map.grid_height),
                expected_size,
                "{preset:?} {dir:?}"
            );
            assert_eq!(outcome.map.map_id, "m-base");
            // Existing token position stays pinned relative to existing geometry.
            let new_tok_pos = outcome.map.token_pos_cm(&outcome.map.tokens[0]);
            let expect_dx_cm = (outcome.existing_shift_m.0 * 100.0).round() as i32;
            let expect_dy_cm = (outcome.existing_shift_m.1 * 100.0).round() as i32;
            assert_eq!(
                new_tok_pos,
                (base_tok_pos.0 + expect_dx_cm, base_tok_pos.1 + expect_dy_cm),
                "{preset:?} {dir:?} token shift"
            );
            // Zero duplicate edge keys.
            let mut seen_edges = BTreeSet::new();
            for e in &outcome.map.wall_edges {
                assert!(
                    seen_edges.insert((e.x, e.y, e.side as u8)),
                    "{preset:?} {dir:?}: duplicate edge {:?}",
                    e
                );
            }
            assert!(
                all_walkable_areas_connected(
                    outcome.map.grid_width,
                    outcome.map.grid_height,
                    &outcome.map.wall_edges,
                    &outcome.map.props
                ),
                "{preset:?} {dir:?} disconnected after continuation"
            );
        }
    }
}

#[test]
fn walled_north_south_adds_seam_doorway_and_outdoor_ribbons_continue() {
    let apt =
        build_procedural_map_request("apt", SectorPreset::ResidentialApartment, None, 20, 14, 42);
    let south = continue_procedural_map(&apt, MapContinuationDir::South, None).unwrap();
    let has_seam_door = south.map.wall_edges.iter().any(|e| {
        e.side == EdgeSide::North
            && e.y == 14
            && matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen)
    });
    assert!(
        has_seam_door,
        "South continuation on Apartment must carve a connecting doorway"
    );

    let creek =
        build_procedural_map_request("creek", SectorPreset::WoodlandCreek, None, 20, 14, 42);
    let creek_s = continue_procedural_map(&creek, MapContinuationDir::South, None).unwrap();
    let crosses_south_seam = creek_s.map.surfaces.iter().any(|s| {
        s.kind == SurfaceFeatureKind::CreekChannel
            && (1..20).any(|gx| {
                let xm = gx as f32;
                crate::map_surface_geom::surface_contains_point_m(s, xm, 14.0)
                    || (crate::map_surface_geom::surface_contains_point_m(s, xm, 13.6)
                        && crate::map_surface_geom::surface_contains_point_m(s, xm, 14.4))
            })
    });
    assert!(
        crosses_south_seam,
        "WoodlandCreek South continuation should bridge creek across y=14"
    );

    let park = build_procedural_map_request("park", SectorPreset::WoodedPark, None, 20, 14, 42);
    let park_e = continue_procedural_map(&park, MapContinuationDir::East, None).unwrap();
    let bridges_east_trail = park_e.map.surfaces.iter().any(|s| {
        s.kind == SurfaceFeatureKind::DirtTrail
            && (1..14).any(|gy| {
                let ym = gy as f32;
                crate::map_surface_geom::surface_contains_point_m(s, 20.0, ym)
                    || (crate::map_surface_geom::surface_contains_point_m(s, 19.6, ym)
                        && crate::map_surface_geom::surface_contains_point_m(s, 20.4, ym))
            })
    });
    assert!(
        bridges_east_trail,
        "WoodedPark East continuation should bridge winding trail across x=20"
    );
}

#[test]
fn multi_step_l_and_full_stitch_respects_max_arena_bounds() {
    let mut map =
        build_procedural_map_request("multi", SectorPreset::UrbanCrossroads, None, 20, 14, 42);
    map = continue_procedural_map(&map, MapContinuationDir::East, None)
        .unwrap()
        .map;
    assert_eq!((map.grid_width, map.grid_height), (40, 14));
    map = continue_procedural_map(&map, MapContinuationDir::North, None)
        .unwrap()
        .map;
    assert_eq!((map.grid_width, map.grid_height), (40, 28));
    assert!(all_walkable_areas_connected(
        map.grid_width,
        map.grid_height,
        &map.wall_edges,
        &map.props
    ));
    while map.grid_width < MAX_STITCHED_MAP_W {
        map = continue_procedural_map(&map, MapContinuationDir::East, None)
            .unwrap()
            .map;
    }
    assert_eq!(map.grid_width, MAX_STITCHED_MAP_W);
    assert!(!can_continue_map(&map, MapContinuationDir::East));
    assert!(continue_procedural_map(&map, MapContinuationDir::East, None).is_err());

    // Large single sectors stitch with proportional continuation strips cleanly.
    let estate =
        build_procedural_map_request("estate", SectorPreset::GardenEstate, None, 48, 36, 99);
    let stitched_estate = continue_procedural_map(&estate, MapContinuationDir::East, None)
        .unwrap()
        .map;
    assert_eq!(
        (stitched_estate.grid_width, stitched_estate.grid_height),
        (84, 36)
    );
    assert!(all_walkable_areas_connected(
        stitched_estate.grid_width,
        stitched_estate.grid_height,
        &stitched_estate.wall_edges,
        &stitched_estate.props
    ));
}

#[test]
fn interiors_are_deterministic_furnished_and_fully_connected() {
    for style in [
        InteriorStyle::Apartment,
        InteriorStyle::Bar,
        InteriorStyle::Office,
    ] {
        for seed in [1_i64, 42, 179, 9001] {
            let a = generate_furnished_interior(style, 20, 14, seed);
            assert_eq!(
                a,
                generate_furnished_interior(style, 20, 14, seed),
                "{style:?} deterministic"
            );
            assert!(
                a.props.len() >= 5,
                "{style:?}/{seed}: only {} props",
                a.props.len()
            );
            assert!(a.wall_edges.iter().any(|e| e.kind == WallEdgeKind::Window));
            assert!(
                a.wall_edges
                    .iter()
                    .any(|e| e.material == MaterialClass::Drywall)
            );
            assert!(all_walkable_areas_connected(
                20,
                14,
                &a.wall_edges,
                &a.props
            ));
            for &(x, y) in &[(2, 7), (2, 6), (17, 7), (17, 8)] {
                assert!(
                    !circle_collides_obstacles(
                        20.0,
                        14.0,
                        &a.wall_edges,
                        &a.props,
                        x as f32 + 0.5,
                        y as f32 + 0.5,
                        TOKEN_RADIUS_M,
                    ),
                    "spawn pad ({x},{y})"
                );
            }
            // No two solid props overlap.
            for (i, p) in a.props.iter().enumerate() {
                for q in &a.props[i + 1..] {
                    if prop_spec(p.kind).floor_layer || prop_spec(q.kind).floor_layer {
                        continue;
                    }
                    let overlap = props_share_covered_cell(p, q);
                    assert!(!overlap, "{style:?}/{seed}: {p:?} overlaps {q:?}");
                }
            }
        }
    }
    let bar = generate_furnished_interior(InteriorStyle::Bar, 20, 14, 42);
    let counter = bar
        .props
        .iter()
        .find(|p| p.kind == PropKind::BarCounter)
        .expect("bar counter");
    assert!(counter.w_cells >= 3, "multi-cell bar counter: {counter:?}");
    let office = generate_furnished_interior(InteriorStyle::Office, 20, 14, 42);
    assert!(
        office.props.iter().any(|p| p.kind == PropKind::Desk),
        "office workstations"
    );
    assert!(
        office
            .wall_edges
            .iter()
            .any(|e| e.kind == WallEdgeKind::Window && e.x > 0 && e.x < 20),
        "glass conference wall"
    );
    // Gait reach from the West spawn is bounded by interior walls (not the whole board).
    let scene = TacticalSceneView {
        width_m: 20.0,
        height_m: 14.0,
        edges: &bar.wall_edges,
        props: &bar.props,
        surfaces: &bar.surfaces,
    };
    let reach = compute_reachable_continuous(
        &scene,
        &[],
        GaitReachQuery {
            start_m: (2.5, 7.5),
            spd_eff: 11,
            hgt_m: 1.8,
            gait: CombatMovementGait::AllOutSprint,
            tick_budget: 40,
        },
    );
    let strips = reach.merged_row_runs();
    assert!(!strips.is_empty());
    assert!(!strips.iter().any(|s| s.x_min_m < 0.0 || s.x_max_m > 20.01));
}

#[test]
fn all_six_zoned_presets_generate_connected_multi_floor_maps_across_seeds() {
    let presets = [
        SectorPreset::UrbanResidential,
        SectorPreset::ProjectsHighDensity,
        SectorPreset::SuburbanNeighborhood,
        SectorPreset::RuralCountryside,
        SectorPreset::LuxuryMansion,
        SectorPreset::DowntownCommercial,
    ];
    for preset in presets {
        for seed in [40_i64, 41, 42] {
            let sec = generate_zoned_sector(preset, 36, 26, seed, None, None);
            assert!(
                all_walkable_areas_connected(36, 26, &sec.wall_edges, &sec.props),
                "{preset:?} seed {seed} failed 1F connectivity"
            );
            assert!(
                sec.floors.len() >= 2,
                "{preset:?} seed {seed} expected multi-floor output, got {} floors",
                sec.floors.len()
            );
            if preset == SectorPreset::LuxuryMansion {
                assert!(
                    sec.floors.iter().any(|f| f.floor_index == -1),
                    "LuxuryMansion seed {seed} should include B1 basement"
                );
            }
            for fl in &sec.floors {
                if fl.floor_index == 0 {
                    assert!(all_walkable_areas_connected(
                        36,
                        26,
                        &fl.wall_edges,
                        &fl.props
                    ));
                    continue;
                }
                let floor_lots: Vec<LotBox> = fl
                    .surfaces
                    .iter()
                    .filter(|s| {
                        s.kind == SurfaceFeatureKind::InteriorFloor && !s.points_cm.is_empty()
                    })
                    .map(|s| {
                        let x0 = s.points_cm.iter().map(|p| p.x_cm).min().unwrap_or(0) / 100;
                        let y0 = s.points_cm.iter().map(|p| p.y_cm).min().unwrap_or(0) / 100;
                        let x1 = s.points_cm.iter().map(|p| p.x_cm).max().unwrap_or(0) / 100;
                        let y1 = s.points_cm.iter().map(|p| p.y_cm).max().unwrap_or(0) / 100;
                        LotBox::new(x0, y0, x1, y1)
                    })
                    .collect();
                assert!(
                    !floor_lots.is_empty(),
                    "{preset:?} floor {} missing InteriorFloor",
                    fl.floor_index
                );
                for lot in floor_lots {
                    assert!(
                        floor_lot_connected(36, 26, lot, &fl.wall_edges, &fl.props),
                        "{preset:?} floor {} lot {lot:?} failed connectivity",
                        fl.floor_index
                    );
                }
            }
        }
    }
}

#[test]
fn economic_tier_and_zoning_override_decoupling_works() {
    let mansion_rural = generate_zoned_sector(
        SectorPreset::RuralCountryside,
        36,
        26,
        7,
        Some(EconomicTier::LuxuryMansion),
        Some(ZoningContext::RuralCountryside),
    );
    assert!(
        mansion_rural
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::GravelRoad)
    );
    assert!(mansion_rural.floors.iter().any(|f| f.floor_index == -1));

    let req = build_procedural_map_request_with_options(
        "zoned-mansion",
        SectorPreset::LuxuryMansion,
        None,
        36,
        26,
        42,
        None,
        None,
    );
    assert!(req.has_multiple_floors());
    assert_eq!(req.active_floor, 0);
}

#[test]
fn residential_architectures_vary_shapes_floors_decks_pools_and_non_sparse_rooms() {
    let mut saw_multi_wing = false;
    let mut saw_tile_or_marble = false;
    let mut saw_carpet = false;
    let mut saw_deck_or_pool = false;
    let mut saw_bedroom_suite = false;

    for seed in 1_i64..=18 {
        for preset in [
            SectorPreset::RuralCountryside,
            SectorPreset::SuburbanNeighborhood,
            SectorPreset::LuxuryMansion,
        ] {
            let sec = generate_zoned_sector(preset, 36, 26, seed, None, None);
            assert!(all_walkable_areas_connected(
                36,
                26,
                &sec.wall_edges,
                &sec.props
            ));
            let interior_count = sec
                .surfaces
                .iter()
                .filter(|s| s.kind == SurfaceFeatureKind::InteriorFloor)
                .count();
            if interior_count >= 2 {
                saw_multi_wing = true;
            }
            if sec.surfaces.iter().any(|s| {
                matches!(
                    s.kind,
                    SurfaceFeatureKind::CeramicTileFloor | SurfaceFeatureKind::MarbleFloor
                )
            }) {
                saw_tile_or_marble = true;
            }
            if sec
                .surfaces
                .iter()
                .any(|s| s.kind == SurfaceFeatureKind::CarpetFloor)
            {
                saw_carpet = true;
            }
            if sec.surfaces.iter().any(|s| {
                matches!(
                    s.kind,
                    SurfaceFeatureKind::WoodDeck | SurfaceFeatureKind::PoolWater
                )
            }) {
                saw_deck_or_pool = true;
            }
            if sec
                .props
                .iter()
                .any(|p| matches!(p.kind, PropKind::Nightstand | PropKind::Dresser))
                && sec.props.iter().any(|p| p.kind == PropKind::StandingLamp)
            {
                saw_bedroom_suite = true;
            }
        }
    }

    assert!(
        saw_multi_wing,
        "expected L-shaped multi-wing homes across seeds"
    );
    assert!(
        saw_tile_or_marble,
        "expected ceramic/marble kitchen/bath room floors"
    );
    assert!(saw_carpet, "expected carpet flooring in bedrooms");
    assert!(saw_deck_or_pool, "expected attached wood decks and pools");
    assert!(
        saw_bedroom_suite,
        "expected non-sparse bedroom suites with nightstands/dressers/lamps"
    );
    assert!((tree_canopy_radius_m(PropKind::LargeTree) - 1.95).abs() < 1e-4);
    assert!((tree_canopy_radius_m(PropKind::MediumTree) - 1.35).abs() < 1e-4);
}

#[test]
fn hospital_and_labs_have_realistic_rooms_elevators_baths_and_clutter() {
    for (w, h) in [(24, 18), (48, 36)] {
        let hosp = generate_procedural_sector(SectorPreset::Hospital, w, h, 42);
        assert_eq!(hosp.floors.len(), 2);
        let portal_centers =
            |fl: &satya_schema::MapFloorLevel, kind: PropKind| -> Vec<(i32, i32)> {
                let mut v: Vec<_> = fl
                    .props
                    .iter()
                    .filter(|p| p.kind == kind)
                    .map(|p| p.center_cm())
                    .collect();
                v.sort();
                v
            };
        for pk in [
            PropKind::Stairwell,
            PropKind::ElevatorCar,
            PropKind::ServiceElevator,
        ] {
            let f1 = portal_centers(&hosp.floors[0], pk);
            let f2 = portal_centers(&hosp.floors[1], pk);
            assert!(!f1.is_empty(), "Hospital {w}x{h} missing {pk:?}");
            assert_eq!(
                f1, f2,
                "Hospital {w}x{h} portal {pk:?} must align across floors"
            );
        }
        let all_props: Vec<_> = hosp.floors.iter().flat_map(|f| f.props.iter()).collect();
        for required in [
            PropKind::NurseStation,
            PropKind::TelemetryMonitor,
            PropKind::MedicalCounter,
            PropKind::BiohazardBin,
            PropKind::LinenCart,
            PropKind::Toilet,
            PropKind::BathroomSink,
            PropKind::HospitalBed,
        ] {
            assert!(
                all_props.iter().any(|p| p.kind == required),
                "Hospital {w}x{h} missing {required:?}"
            );
        }
        assert!(
            all_props.iter().any(|p| {
                matches!(
                    p.kind,
                    PropKind::HospitalBed | PropKind::Cot | PropKind::Gurney
                ) && p.rotation_deg % 90 != 0
            }),
            "Hospital {w}x{h} should include gently angled patient beds"
        );
    }

    let lab_small = generate_procedural_sector(SectorPreset::MedLabSector, 20, 14, 11);
    assert!(
        !lab_small.props.iter().any(|p| p.kind == PropKind::Bed),
        "MedLab should use clinical beds instead of residential Bed"
    );
    for pk in [
        PropKind::LabBench,
        PropKind::TestBench,
        PropKind::SolderingStation,
        PropKind::Printer3dStation,
        PropKind::ServerRack,
        PropKind::FumeHood,
        PropKind::DeskClutter,
    ] {
        assert!(
            lab_small.props.iter().any(|p| p.kind == pk),
            "MedLab 20x14 missing {pk:?}"
        );
    }

    let lab_big = generate_procedural_sector(SectorPreset::MedLabSector, 36, 26, 17);
    assert!(lab_big.props.iter().any(|p| p.kind == PropKind::CncMill));
    assert!(
        lab_big
            .props
            .iter()
            .any(|p| p.kind == PropKind::ElevatorCar)
    );
    assert!(
        lab_big
            .props
            .iter()
            .any(|p| p.kind == PropKind::AirCompressor)
    );

    let office = generate_procedural_sector(SectorPreset::OfficeFloor, 36, 26, 23);
    for pk in [
        PropKind::DeskClutter,
        PropKind::FilingCabinet,
        PropKind::WaterCooler,
        PropKind::ServerRack,
        PropKind::ElevatorCar,
    ] {
        assert!(
            office.props.iter().any(|p| p.kind == pk),
            "OfficeFloor 36x26 missing {pk:?}"
        );
    }
}

#[test]
fn houses_cap_bathroom_size_and_urban_projects_presets_differ() {
    for seed in 1_i64..=6 {
        for preset in [
            SectorPreset::SuburbanNeighborhood,
            SectorPreset::LuxuryMansion,
            SectorPreset::RuralCountryside,
        ] {
            let sec = generate_zoned_sector(preset, 36, 26, seed, None, None);
            for fl in &sec.floors {
                for sf in fl.surfaces.iter().filter(|s| {
                    matches!(
                        s.kind,
                        SurfaceFeatureKind::CeramicTileFloor | SurfaceFeatureKind::MarbleFloor
                    ) && s.points_cm.len() == 4
                }) {
                    let x0 = sf.points_cm.iter().map(|p| p.x_cm).min().unwrap_or(0) / 100;
                    let y0 = sf.points_cm.iter().map(|p| p.y_cm).min().unwrap_or(0) / 100;
                    let x1 = sf.points_cm.iter().map(|p| p.x_cm).max().unwrap_or(0) / 100;
                    let y1 = sf.points_cm.iter().map(|p| p.y_cm).max().unwrap_or(0) / 100;
                    let area_m2 = (x1 - x0) * (y1 - y0);
                    let has_toilet = fl.props.iter().any(|p| {
                        p.kind == PropKind::Toilet && p.x >= x0 && p.x < x1 && p.y >= y0 && p.y < y1
                    });
                    if has_toilet {
                        assert!(
                            area_m2 <= 18,
                            "{preset:?} seed {seed} floor {} generated oversized bathroom ({area_m2} m²)",
                            fl.floor_index
                        );
                    }
                }
            }
            assert!(
                sec.props.iter().any(|p| p.kind == PropKind::Toilet)
                    && sec.props.iter().any(|p| p.kind == PropKind::BathroomSink)
                    && sec.props.iter().any(|p| {
                        matches!(
                            p.kind,
                            PropKind::Bathtub | PropKind::ShowerStall | PropKind::SpaBath
                        )
                    }),
                "{preset:?} seed {seed} missing toilet/sink/bath"
            );
        }
    }

    // Suburban yards leave majority of board open grass rather than wall-to-wall buildings.
    let sub = generate_zoned_sector(SectorPreset::SuburbanNeighborhood, 36, 26, 4, None, None);
    let bldg_area: i32 = sub
        .surfaces
        .iter()
        .filter(|s| s.kind == SurfaceFeatureKind::InteriorFloor && s.points_cm.len() == 4)
        .map(|s| {
            let w = (s.points_cm.iter().map(|p| p.x_cm).max().unwrap_or(0)
                - s.points_cm.iter().map(|p| p.x_cm).min().unwrap_or(0))
                / 100;
            let h = (s.points_cm.iter().map(|p| p.y_cm).max().unwrap_or(0)
                - s.points_cm.iter().map(|p| p.y_cm).min().unwrap_or(0))
                / 100;
            w * h
        })
        .sum();
    assert!(
        bldg_area < (36 * 26) * 40 / 100,
        "SuburbanNeighborhood building area {bldg_area} too large for generous yards"
    );

    // ProjectsHighDensity uses elevator slab apartments + LinoleumCheckFloor corridors.
    let proj = generate_zoned_sector(SectorPreset::ProjectsHighDensity, 36, 26, 1, None, None);
    assert!(proj.props.iter().any(|p| p.kind == PropKind::ElevatorCar));
    assert!(
        proj.surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::LinoleumCheckFloor)
    );
    assert!(
        !proj
            .props
            .iter()
            .any(|p| p.kind == PropKind::BrownstoneStoop)
    );

    // UrbanResidential deterministically alternates Brownstones (seed % 3 != 2) & Chinatown Shophouses (seed % 3 == 2).
    let brownstone = generate_zoned_sector(SectorPreset::UrbanResidential, 36, 26, 4, None, None);
    assert!(
        brownstone
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::BrickPaverWalk)
    );
    assert!(
        brownstone
            .props
            .iter()
            .any(|p| p.kind == PropKind::BrownstoneStoop)
    );

    let chinatown = generate_zoned_sector(SectorPreset::UrbanResidential, 36, 26, 2, None, None);
    assert!(
        chinatown
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::BrickPaverWalk)
    );
    assert!(
        chinatown
            .props
            .iter()
            .any(|p| p.kind == PropKind::AwningCanopy)
    );
}

#[test]
fn garden_estate_furnishes_rooms_and_varies_creeks_pools_and_spas() {
    let mut creek_seeds = 0usize;
    let mut pool_seeds = 0usize;
    let mut spa_seeds = 0usize;

    for seed in 1_i64..=24 {
        let sec = generate_procedural_sector(SectorPreset::GardenEstate, 48, 36, seed);
        assert!(
            sec.props.len() >= 18,
            "GardenEstate seed={seed} too sparse overall ({} props)",
            sec.props.len()
        );
        let indoor_furniture = sec
            .props
            .iter()
            .filter(|p| {
                matches!(
                    p.kind,
                    PropKind::Sofa
                        | PropKind::Loveseat
                        | PropKind::Bed
                        | PropKind::DiningTable
                        | PropKind::KitchenCounter
                        | PropKind::Wardrobe
                        | PropKind::Desk
                        | PropKind::Bookshelf
                        | PropKind::Toilet
                        | PropKind::Bathtub
                        | PropKind::BathroomSink
                )
            })
            .count();
        assert!(
            indoor_furniture >= 8,
            "GardenEstate seed={seed} sparsely furnished interior ({indoor_furniture} props)"
        );
        for p in sec
            .props
            .iter()
            .filter(|p| matches!(p.kind, PropKind::Sofa | PropKind::Loveseat))
        {
            assert!(
                !crate::tactical_map_geom::prop_crosses_wall_edge(p, &sec.wall_edges),
                "GardenEstate seed={seed} sofa {:?} at {:?} crosses a wall",
                p.kind,
                p.center_cm()
            );
        }
        let has_creek = sec
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::CreekChannel);
        let has_bridge = sec
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::BridgeDeck);
        if has_creek {
            assert!(
                has_bridge,
                "GardenEstate seed={seed} has CreekChannel without BridgeDeck"
            );
            creek_seeds += 1;
        }
        if sec
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::PoolWater)
        {
            pool_seeds += 1;
        }
        if sec.props.iter().any(|p| p.kind == PropKind::HotTub) {
            spa_seeds += 1;
        }
    }

    assert!(
        (3..=20).contains(&creek_seeds),
        "Expected occasional GardenEstate creeks across 24 seeds, got {creek_seeds}"
    );
    assert!(
        pool_seeds >= 4,
        "Expected varied GardenEstate pools across 24 seeds, got {pool_seeds}"
    );
    assert!(
        spa_seeds >= 4,
        "Expected varied GardenEstate Jacuzzi/spas across 24 seeds, got {spa_seeds}"
    );
}

#[test]
fn housing_units_generate_varied_pool_shapes_and_spas_without_submerging_props() {
    let mut pool_vertex_counts = BTreeSet::new();
    let mut spa_styles = BTreeSet::new();

    for seed in 1_i64..=20 {
        let mut sectors = vec![
            generate_zoned_sector(SectorPreset::LuxuryMansion, 36, 28, seed, None, None),
            generate_zoned_sector(SectorPreset::SuburbanNeighborhood, 36, 28, seed, None, None),
            generate_zoned_sector(SectorPreset::RuralCountryside, 36, 28, seed, None, None),
            generate_zoned_sector(SectorPreset::UrbanResidential, 36, 26, seed, None, None),
            generate_zoned_sector(SectorPreset::GardenEstate, 48, 36, seed, None, None),
            generate_procedural_sector(SectorPreset::ResidentialApartment, 28, 22, seed),
        ];

        for sec in sectors.drain(..) {
            let pools: Vec<_> = sec
                .surfaces
                .iter()
                .filter(|s| s.kind == SurfaceFeatureKind::PoolWater)
                .collect();
            for pool in &pools {
                pool_vertex_counts.insert(pool.points_cm.len());
            }
            for prop in &sec.props {
                if prop.kind == PropKind::HotTub {
                    spa_styles.insert(prop.variant_seed & 3);
                }
                if matches!(prop.kind, PropKind::HotTub | PropKind::ReclinerChair) {
                    let fp = prop.footprint_m();
                    for pool in &pools {
                        assert!(
                            !crate::map_surface_queries::surface_contains_point_m(
                                pool, fp.cx_m, fp.cy_m
                            ),
                            "Prop {:?} at ({:.2}, {:.2}) submerged inside PoolWater (seed={seed})",
                            prop.kind,
                            fp.cx_m,
                            fp.cy_m
                        );
                    }
                }
            }
        }
    }

    assert!(
        pool_vertex_counts.len() >= 3,
        "Expected varied pool geometry vertex counts (oval/kidney/L-shaped/rounded), got {pool_vertex_counts:?}"
    );
    assert!(
        spa_styles.len() >= 3,
        "Expected at least 3 distinct HotTub spa styles across housing units, got {spa_styles:?}"
    );
}

fn counter_flush_against_wall(
    w: i32,
    h: i32,
    edges: &[MapWallEdge],
    prop: &satya_schema::MapPropPlacement,
) -> bool {
    let idx = crate::tactical_map_edges::WallEdgeIndex::from_edges(w, h, edges);
    let is_wall =
        |k: Option<WallEdgeKind>| matches!(k, Some(WallEdgeKind::Wall | WallEdgeKind::Window));
    match prop.rotation_deg.rem_euclid(360) {
        90 => (0..prop.h_cells)
            .all(|dy| is_wall(idx.get(prop.x + prop.w_cells, prop.y + dy, EdgeSide::West))),
        180 => (0..prop.w_cells)
            .all(|dx| is_wall(idx.get(prop.x + dx, prop.y + prop.h_cells, EdgeSide::North))),
        270 => (0..prop.h_cells).all(|dy| is_wall(idx.get(prop.x, prop.y + dy, EdgeSide::West))),
        _ => (0..prop.w_cells).all(|dx| is_wall(idx.get(prop.x + dx, prop.y, EdgeSide::North))),
    }
}

fn room_cells_without_crossing_edges(
    w: i32,
    h: i32,
    edges: &[MapWallEdge],
    sx: i32,
    sy: i32,
) -> BTreeSet<(i32, i32)> {
    let idx = crate::tactical_map_edges::WallEdgeIndex::from_edges(w, h, edges);
    let mut visited = BTreeSet::new();
    if sx < 0 || sx >= w || sy < 0 || sy >= h {
        return visited;
    }
    let mut stack = vec![(sx, sy)];
    visited.insert((sx, sy));
    while let Some((x, y)) = stack.pop() {
        if x > 0 && idx.get(x, y, EdgeSide::West).is_none() && visited.insert((x - 1, y)) {
            stack.push((x - 1, y));
        }
        if x + 1 < w && idx.get(x + 1, y, EdgeSide::West).is_none() && visited.insert((x + 1, y)) {
            stack.push((x + 1, y));
        }
        if y > 0 && idx.get(x, y, EdgeSide::North).is_none() && visited.insert((x, y - 1)) {
            stack.push((x, y - 1));
        }
        if y + 1 < h && idx.get(x, y + 1, EdgeSide::North).is_none() && visited.insert((x, y + 1)) {
            stack.push((x, y + 1));
        }
    }
    visited
}

fn point_in_any_surface(
    surfaces: &[satya_schema::MapSurfaceFeature],
    kind: SurfaceFeatureKind,
    x_m: f32,
    y_m: f32,
) -> bool {
    surfaces.iter().any(|s| {
        s.kind == kind && crate::map_surface_queries::surface_contains_point_m(s, x_m, y_m)
    })
}

#[test]
fn residential_units_have_hallways_wall_sinks_logical_room_caps_and_specialty_rooms() {
    use crate::map_gen_residential::{CardinalEntry, generate_home_in_lot};

    let tiers = [
        EconomicTier::LowIncome,
        EconomicTier::MiddleClass,
        EconomicTier::UpperMiddleClass,
        EconomicTier::LuxuryMansion,
    ];

    for tier in tiers {
        for (lw, lh) in [(9, 7), (14, 10), (22, 15)] {
            let (bw, bh) = (lw + 4, lh + 4);
            let lot = LotBox::new(2, 2, 2 + lw, 2 + lh);
            for seed in 1_u64..=16 {
                let home = generate_home_in_lot(
                    bw,
                    bh,
                    lot,
                    tier,
                    CardinalEntry::South,
                    true,
                    true,
                    true,
                    &[],
                    seed,
                );
                let levels: Vec<&crate::map_gen_residential::FloorContent> = home
                    .basement
                    .iter()
                    .chain(std::iter::once(&home.ground))
                    .chain(home.upper.iter())
                    .collect();

                let total_washers: usize = levels
                    .iter()
                    .map(|fl| {
                        fl.props
                            .iter()
                            .filter(|p| p.kind == PropKind::WasherDryer)
                            .count()
                    })
                    .sum();
                assert!(
                    total_washers <= 1,
                    "{tier:?} {lw}x{lh} seed={seed} emitted {total_washers} WasherDryer units across B1+1F+2F"
                );
                let total_stoves: usize = levels
                    .iter()
                    .map(|fl| {
                        fl.props
                            .iter()
                            .filter(|p| p.kind == PropKind::Stove)
                            .count()
                    })
                    .sum();
                assert!(
                    total_stoves <= 1,
                    "{tier:?} {lw}x{lh} seed={seed} emitted {total_stoves} Stoves across B1+1F+2F"
                );

                for (li, fl) in levels.iter().enumerate() {
                    let toilets = fl
                        .props
                        .iter()
                        .filter(|p| p.kind == PropKind::Toilet)
                        .count();
                    assert!(
                        toilets <= 2,
                        "{tier:?} {lw}x{lh} seed={seed} level_idx={li} had {toilets} toilets"
                    );
                    for kc in fl
                        .props
                        .iter()
                        .filter(|p| p.kind == PropKind::KitchenCounter)
                    {
                        assert!(
                            counter_flush_against_wall(bw, bh, &fl.wall_edges, kc),
                            "{tier:?} {lw}x{lh} seed={seed} level_idx={li} floating KitchenCounter at ({}, {})",
                            kc.x,
                            kc.y
                        );
                    }
                }

                // Locate South front entry door on 1F (interior to north, exterior to south).
                let front_door = home
                    .ground
                    .wall_edges
                    .iter()
                    .find(|e| {
                        e.side == EdgeSide::North
                            && e.material == MaterialClass::SolidWood
                            && matches!(e.kind, WallEdgeKind::DoorOpen | WallEdgeKind::DoorClosed)
                            && e.x >= lot.x0
                            && e.x < lot.x1
                            && e.y > lot.y0
                            && e.y <= lot.y1
                            && point_in_any_surface(
                                &home.ground.surfaces,
                                SurfaceFeatureKind::InteriorFloor,
                                e.x as f32 + 0.5,
                                e.y as f32 - 0.5,
                            )
                            && !point_in_any_surface(
                                &home.ground.surfaces,
                                SurfaceFeatureKind::InteriorFloor,
                                e.x as f32 + 0.5,
                                e.y as f32 + 0.5,
                            )
                    })
                    .unwrap_or_else(|| {
                        panic!("{tier:?} {lw}x{lh} seed={seed} missing South front door")
                    });
                let (ex, ey) = (front_door.x, front_door.y - 1);
                if (lw, lh) == (9, 7)
                    && matches!(tier, EconomicTier::LowIncome | EconomicTier::MiddleClass)
                {
                    assert!(
                        !point_in_any_surface(
                            &home.ground.surfaces,
                            SurfaceFeatureKind::MarbleFloor,
                            ex as f32 + 0.5,
                            ey as f32 + 0.5
                        ),
                        "Small {tier:?} 9x7 seed={seed} should open directly without MarbleFloor foyer"
                    );
                }
                let entry_room =
                    room_cells_without_crossing_edges(bw, bh, &home.ground.wall_edges, ex, ey);
                let entry_bad = home.ground.props.iter().any(|p| {
                    matches!(p.kind, PropKind::Toilet | PropKind::WasherDryer)
                        && (p.y..p.y + p.h_cells).any(|py| {
                            (p.x..p.x + p.w_cells)
                                .any(|px| p.covers(px, py) && entry_room.contains(&(px, py)))
                        })
                });
                assert!(
                    !entry_bad,
                    "{tier:?} {lw}x{lh} seed={seed} front door opened into Bathroom/Laundry"
                );
            }
        }
    }

    // Verify all 5 large-home specialty rooms appear across LuxuryMansion 22x15 seeds.
    let mut saw_garage = false;
    let mut saw_home_gym = false;
    let mut saw_library = false;
    let mut saw_office = false;
    let mut saw_sunroom = false;
    let lot_big = LotBox::new(2, 2, 24, 17);
    for seed in 1_u64..=24 {
        let home = generate_home_in_lot(
            26,
            19,
            lot_big,
            EconomicTier::LuxuryMansion,
            CardinalEntry::South,
            true,
            true,
            true,
            &[],
            seed,
        );
        if home
            .ground
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::StainedConcreteFloor)
            && home
                .ground
                .wall_edges
                .iter()
                .any(|e| e.material == MaterialClass::SheetSteel)
        {
            saw_garage = true;
        }
        let levels: Vec<&crate::map_gen_residential::FloorContent> = home
            .basement
            .iter()
            .chain(std::iter::once(&home.ground))
            .chain(home.upper.iter())
            .collect();
        for fl in &levels {
            if fl
                .surfaces
                .iter()
                .any(|s| s.kind == SurfaceFeatureKind::LinoleumCheckFloor)
                && fl.props.iter().any(|p| p.kind == PropKind::WaterCooler)
            {
                saw_home_gym = true;
            }
            if fl
                .props
                .iter()
                .filter(|p| p.kind == PropKind::Bookshelf)
                .count()
                >= 4
            {
                saw_library = true;
            }
            if fl
                .props
                .iter()
                .any(|p| matches!(p.kind, PropKind::DeskL | PropKind::Desk))
            {
                saw_office = true;
            }
        }
        for fl in std::iter::once(&home.ground).chain(home.upper.iter()) {
            let wins = fl
                .wall_edges
                .iter()
                .filter(|e| e.kind == WallEdgeKind::Window)
                .count();
            if wins >= 6 && fl.props.iter().any(|p| p.kind == PropKind::Planter) {
                saw_sunroom = true;
            }
        }
    }
    assert!(
        saw_garage,
        "Expected LuxuryMansion 22x15 Garage across seeds"
    );
    assert!(
        saw_home_gym,
        "Expected LuxuryMansion 22x15 HomeGym across seeds"
    );
    assert!(
        saw_library,
        "Expected LuxuryMansion 22x15 Library across seeds"
    );
    assert!(
        saw_office,
        "Expected LuxuryMansion 22x15 Office across seeds"
    );
    assert!(
        saw_sunroom,
        "Expected LuxuryMansion 22x15 Sunroom across seeds"
    );

    // Verify GardenEstate (48x36) & ResidentialApartment (20x14 & 28x22) invariants.
    for seed in 1_i64..=16 {
        let estate = generate_procedural_sector(SectorPreset::GardenEstate, 48, 36, seed);
        let mut total_washers = 0usize;
        let mut total_stoves = 0usize;
        for fl in &estate.floors {
            for kc in fl
                .props
                .iter()
                .filter(|p| p.kind == PropKind::KitchenCounter)
            {
                assert!(
                    counter_flush_against_wall(48, 36, &fl.wall_edges, kc),
                    "GardenEstate seed={seed} floor={} floating KitchenCounter at ({}, {})",
                    fl.floor_index,
                    kc.x,
                    kc.y
                );
            }
            total_washers += fl
                .props
                .iter()
                .filter(|p| p.kind == PropKind::WasherDryer)
                .count();
            total_stoves += fl
                .props
                .iter()
                .filter(|p| p.kind == PropKind::Stove)
                .count();
            if fl.floor_index == 0 {
                let toilets = fl
                    .props
                    .iter()
                    .filter(|p| p.kind == PropKind::Toilet)
                    .count();
                assert!(
                    toilets <= 3,
                    "GardenEstate seed={seed} 1F had {toilets} toilets"
                );
            }
        }
        assert!(
            total_washers <= 1,
            "GardenEstate seed={seed} had {total_washers} WasherDryer units across floors"
        );
        assert!(
            total_stoves <= 2,
            "GardenEstate seed={seed} had {total_stoves} Stoves across floors"
        );

        for (aw, ah, max_toilets, check_direct_entry) in
            [(20, 14, 2usize, true), (28, 22, 3usize, false)]
        {
            let apt = generate_procedural_sector(SectorPreset::ResidentialApartment, aw, ah, seed);
            for kc in apt
                .props
                .iter()
                .filter(|p| p.kind == PropKind::KitchenCounter)
            {
                assert!(
                    counter_flush_against_wall(aw, ah, &apt.wall_edges, kc),
                    "ResidentialApartment {aw}x{ah} seed={seed} floating KitchenCounter at ({}, {})",
                    kc.x,
                    kc.y
                );
            }
            let stoves = apt
                .props
                .iter()
                .filter(|p| p.kind == PropKind::Stove)
                .count();
            let washers = apt
                .props
                .iter()
                .filter(|p| p.kind == PropKind::WasherDryer)
                .count();
            let toilets = apt
                .props
                .iter()
                .filter(|p| p.kind == PropKind::Toilet)
                .count();
            assert_eq!(
                stoves, 1,
                "ResidentialApartment {aw}x{ah} seed={seed} expected 1 Stove, got {stoves}"
            );
            assert!(
                washers <= 1,
                "ResidentialApartment {aw}x{ah} seed={seed} had {washers} WasherDryer units"
            );
            assert!(
                toilets <= max_toilets,
                "ResidentialApartment {aw}x{ah} seed={seed} had {toilets} toilets (cap {max_toilets})"
            );
            if check_direct_entry {
                assert!(
                    !point_in_any_surface(
                        &apt.surfaces,
                        SurfaceFeatureKind::MarbleFloor,
                        0.5,
                        (ah / 2) as f32 + 0.5
                    ),
                    "Compact ResidentialApartment {aw}x{ah} seed={seed} should not place MarbleFloor foyer at West entry"
                );
            }
            for sf in apt.surfaces.iter().filter(|s| {
                matches!(
                    s.kind,
                    SurfaceFeatureKind::CeramicTileFloor | SurfaceFeatureKind::MarbleFloor
                ) && s.points_cm.len() == 4
            }) {
                let x0 = sf.points_cm.iter().map(|p| p.x_cm).min().unwrap_or(0) / 100;
                let y0 = sf.points_cm.iter().map(|p| p.y_cm).min().unwrap_or(0) / 100;
                let x1 = sf.points_cm.iter().map(|p| p.x_cm).max().unwrap_or(0) / 100;
                let y1 = sf.points_cm.iter().map(|p| p.y_cm).max().unwrap_or(0) / 100;
                let has_toilet = apt.props.iter().any(|p| {
                    p.kind == PropKind::Toilet && p.x >= x0 && p.x < x1 && p.y >= y0 && p.y < y1
                });
                if has_toilet {
                    let area_m2 = (x1 - x0) * (y1 - y0);
                    assert!(
                        area_m2 <= 18,
                        "ResidentialApartment {aw}x{ah} seed={seed} oversized bathroom ({area_m2} m²)"
                    );
                }
            }
        }
    }
}

fn suburban_house_envelopes(surfaces: &[satya_schema::MapSurfaceFeature]) -> Vec<LotBox> {
    let boxes: Vec<LotBox> = surfaces
        .iter()
        .filter(|s| s.kind == SurfaceFeatureKind::InteriorFloor && !s.points_cm.is_empty())
        .map(|s| {
            let x0 = s.points_cm.iter().map(|p| p.x_cm).min().unwrap_or(0) / 100;
            let y0 = s.points_cm.iter().map(|p| p.y_cm).min().unwrap_or(0) / 100;
            let x1 = s.points_cm.iter().map(|p| p.x_cm).max().unwrap_or(0) / 100;
            let y1 = s.points_cm.iter().map(|p| p.y_cm).max().unwrap_or(0) / 100;
            LotBox::new(x0, y0, x1, y1)
        })
        .collect();
    let connected = |a: LotBox, b: LotBox| -> bool {
        let overlap_x = a.x0 < b.x1 && b.x0 < a.x1;
        let overlap_y = a.y0 < b.y1 && b.y0 < a.y1;
        let touch_x = a.x1 == b.x0 || b.x1 == a.x0;
        let touch_y = a.y1 == b.y0 || b.y1 == a.y0;
        (overlap_x && overlap_y) || (touch_x && overlap_y) || (touch_y && overlap_x)
    };
    let mut visited = vec![false; boxes.len()];
    let mut envelopes = Vec::new();
    for start in 0..boxes.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack = vec![start];
        let mut env = boxes[start];
        while let Some(cur) = stack.pop() {
            let b = boxes[cur];
            env = LotBox::new(
                env.x0.min(b.x0),
                env.y0.min(b.y0),
                env.x1.max(b.x1),
                env.y1.max(b.y1),
            );
            for nxt in 0..boxes.len() {
                if !visited[nxt] && connected(b, boxes[nxt]) {
                    visited[nxt] = true;
                    stack.push(nxt);
                }
            }
        }
        envelopes.push(env);
    }
    envelopes.sort_by_key(|l| l.x0);
    envelopes
}

#[test]
fn suburban_neighborhood_varies_house_sizes_aspect_ratios_and_setbacks() {
    use crate::map_surface_queries::PreparedSurfaceGrid;
    use std::collections::HashSet;

    let mut street_house_counts = BTreeSet::new();
    let mut default_widths = BTreeSet::new();
    let mut default_depths = BTreeSet::new();
    let mut saw_one_story = false;
    let mut saw_two_story = false;

    for &(w, h) in &[(36, 26), (48, 36), (64, 48)] {
        for seed in 1_i64..=18 {
            let sec = generate_procedural_sector(SectorPreset::SuburbanNeighborhood, w, h, seed);
            assert!(
                all_walkable_areas_connected(w, h, &sec.wall_edges, &sec.props),
                "SuburbanNeighborhood {w}x{h} seed={seed} failed 1F connectivity"
            );
            assert!(
                sec.floors.len() >= 2,
                "SuburbanNeighborhood {w}x{h} seed={seed} expected multi-floor output"
            );

            let houses = suburban_house_envelopes(&sec.surfaces);
            assert!(
                !houses.is_empty(),
                "SuburbanNeighborhood {w}x{h} seed={seed} emitted zero houses"
            );
            for &house in &houses {
                assert!(
                    !(house.w() == 12 && house.h() == 12),
                    "SuburbanNeighborhood {w}x{h} seed={seed} emitted 12x12 house {house:?}"
                );
                assert!(
                    (house.w() - house.h()).abs() >= 2,
                    "SuburbanNeighborhood {w}x{h} seed={seed} emitted near-square house {}x{}",
                    house.w(),
                    house.h()
                );
            }

            for kc in sec
                .props
                .iter()
                .filter(|p| p.kind == PropKind::KitchenCounter)
            {
                assert!(
                    counter_flush_against_wall(w, h, &sec.wall_edges, kc),
                    "SuburbanNeighborhood {w}x{h} seed={seed} floating 1F KitchenCounter at ({}, {})",
                    kc.x,
                    kc.y
                );
            }
            for fl in &sec.floors {
                for kc in fl
                    .props
                    .iter()
                    .filter(|p| p.kind == PropKind::KitchenCounter)
                {
                    assert!(
                        counter_flush_against_wall(w, h, &fl.wall_edges, kc),
                        "SuburbanNeighborhood {w}x{h} seed={seed} floor {} floating KitchenCounter at ({}, {})",
                        fl.floor_index,
                        kc.x,
                        kc.y
                    );
                }
            }

            let bldg_area: i32 = sec
                .surfaces
                .iter()
                .filter(|s| s.kind == SurfaceFeatureKind::InteriorFloor && !s.points_cm.is_empty())
                .map(|s| {
                    let bw = (s.points_cm.iter().map(|p| p.x_cm).max().unwrap_or(0)
                        - s.points_cm.iter().map(|p| p.x_cm).min().unwrap_or(0))
                        / 100;
                    let bh = (s.points_cm.iter().map(|p| p.y_cm).max().unwrap_or(0)
                        - s.points_cm.iter().map(|p| p.y_cm).min().unwrap_or(0))
                        / 100;
                    bw * bh
                })
                .sum();
            assert!(
                bldg_area < (w * h) * 40 / 100,
                "SuburbanNeighborhood {w}x{h} seed={seed} bldg_area={bldg_area} exceeds 40% cap"
            );

            let grid = PreparedSurfaceGrid::new(&sec.surfaces);
            let emitted: HashSet<SurfaceFeatureKind> =
                sec.surfaces.iter().map(|s| s.kind).collect();
            let mut exposed: HashSet<SurfaceFeatureKind> = HashSet::new();
            for gy in 1..(h * 4) {
                for gx in 1..(w * 4) {
                    if let Some(kind) = grid.topmost_kind_at_m(gx as f32 * 0.25, gy as f32 * 0.25) {
                        exposed.insert(kind);
                    }
                }
            }
            for &kind in &emitted {
                assert!(
                    exposed.contains(&kind),
                    "SuburbanNeighborhood {w}x{h} seed={seed}: {kind:?} 100% occluded"
                );
            }

            if (w, h) == (48, 36) {
                assert!(
                    houses.len() >= 2,
                    "SuburbanNeighborhood 48x36 seed={seed} expected >= 2 houses, got {}",
                    houses.len()
                );
                let dims: BTreeSet<(i32, i32)> = houses.iter().map(|l| (l.w(), l.h())).collect();
                assert!(
                    dims.len() >= 2,
                    "SuburbanNeighborhood 48x36 seed={seed} all houses had identical dims {dims:?}"
                );
                street_house_counts.insert(houses.len());
                let f2 = sec.floors.iter().find(|f| f.floor_index == 1);
                for &house in &houses {
                    default_widths.insert(house.w());
                    default_depths.insert(house.h());
                    let has_upper = f2.is_some_and(|upper| {
                        upper.surfaces.iter().any(|s| {
                            if s.kind != SurfaceFeatureKind::InteriorFloor || s.points_cm.is_empty()
                            {
                                return false;
                            }
                            let fx0 = s.points_cm.iter().map(|p| p.x_cm).min().unwrap_or(0) / 100;
                            let fx1 = s.points_cm.iter().map(|p| p.x_cm).max().unwrap_or(0) / 100;
                            fx0 < house.x1 && fx1 > house.x0
                        })
                    });
                    if has_upper {
                        saw_two_story = true;
                    } else {
                        saw_one_story = true;
                    }
                }
            }
        }
    }

    assert!(
        street_house_counts.contains(&2) && street_house_counts.contains(&3),
        "Expected both 2-house and 3-house 48x36 streets, got {street_house_counts:?}"
    );
    assert!(
        default_widths.len() >= 5
            && default_widths.iter().any(|&x| x <= 11)
            && default_widths.iter().any(|&x| x >= 15),
        "Expected varied house widths (<=11 and >=15, >=5 distinct), got {default_widths:?}"
    );
    assert!(
        default_depths.len() >= 4
            && default_depths.iter().any(|&y| y <= 9)
            && default_depths.iter().any(|&y| y >= 12),
        "Expected varied house depths (<=9 and >=12, >=4 distinct), got {default_depths:?}"
    );
    assert!(
        saw_one_story && saw_two_story,
        "Expected both 1F-only and 2F houses on 48x36 streets (saw_one_story={saw_one_story}, saw_two_story={saw_two_story})"
    );
}

#[test]
fn test_map_continuation_preserves_ground_override_tint_and_cell_size() {
    let mut base = build_procedural_map_request_with_options(
        "cont-ground",
        SectorPreset::SuburbanNeighborhood,
        None,
        20,
        16,
        11,
        None,
        None,
    );
    base.ground_floor_override = Some("brick_paver".to_owned());
    base.ground_tint_rgb = Some([210, 195, 180]);
    base.cell_size_cm = 100;

    let cont = continue_procedural_map(&base, MapContinuationDir::East, Some(77))
        .expect("continue_procedural_map east succeeds");
    assert_eq!(
        cont.map.ground_floor_override.as_deref(),
        Some("brick_paver")
    );
    assert_eq!(cont.map.ground_tint_rgb, Some([210, 195, 180]));
    assert_eq!(cont.map.cell_size_cm, 100);
}

#[test]
fn wave2_ballistics_floor_props_corner_peek_and_window_materials() {
    use crate::barrier_ballistics::{
        BallisticShotQuery, BarrierPenetrationVerdict, evaluate_ballistic_shot,
    };
    use crate::map_dressing::{is_non_blocking_prop, make_prop_at_cm};
    use crate::tactical_map_geom::props_overlap_continuous;
    use crate::tactical_map_inspect::inspect_map_point;
    use satya_engine::ballistics::lookup_firearm_preset;

    // 1. Floor-layer props must be non-blocking for placement and ballistics.
    for kind in [
        PropKind::DeskLamp,
        PropKind::BarStool,
        PropKind::StandingLamp,
        PropKind::FireExtinguisher,
    ] {
        let p = make_prop(kind, 5, 5, 0, None, 0);
        assert!(
            is_non_blocking_prop(&p),
            "{kind:?} should be non-blocking for ballistics/placement"
        );
        let props = [p];
        let scene = TacticalSceneView {
            width_m: 12.0,
            height_m: 10.0,
            edges: &[],
            props: &props,
            surfaces: &[],
        };
        let shot = evaluate_ballistic_shot(BallisticShotQuery {
            scene: &scene,
            attacker_m: (2.5, 5.5),
            target_m: (9.5, 5.5),
            firearm: lookup_firearm_preset("sig_p320_9mm").unwrap(),
            net_successes: 0,
            hit_frame_or_stud: false,
        });
        assert_eq!(
            shot.verdict,
            BarrierPenetrationVerdict::Unobstructed,
            "{kind:?} unexpectedly blocked ballistic shot: {:?}",
            shot.barriers
        );
    }

    // 2. Narrow orthogonal OBB crossing (X-cross) must be detected by SAT overlap.
    let bar_h = make_prop_at_cm(PropKind::JerseyBarrier, 500, 500, 0, Some(220), 0);
    let bar_v = make_prop_at_cm(PropKind::JerseyBarrier, 500, 500, 90, Some(220), 0);
    assert!(
        props_overlap_continuous(&bar_h, &bar_v),
        "Orthogonal crossing barriers must overlap under 2D OBB SAT"
    );

    // 3. Corner-peek (`HighCover`) through an intervening vehicle must evaluate vehicle barrier PR.
    let pillar = make_prop(PropKind::StructuralPillar, 4, 5, 0, None, 0);
    let car = make_prop(PropKind::Car, 7, 3, 0, None, 0);
    let props = [pillar, car];
    let scene = TacticalSceneView {
        width_m: 14.0,
        height_m: 12.0,
        edges: &[],
        props: &props,
        surfaces: &[],
    };
    let peek_shot = evaluate_ballistic_shot(BallisticShotQuery {
        scene: &scene,
        attacker_m: (3.2, 5.5),
        target_m: (11.5, 5.5),
        firearm: lookup_firearm_preset("sig_p320_9mm").unwrap(),
        net_successes: 0,
        hit_frame_or_stud: false,
    });
    assert_eq!(peek_shot.sight_cover_grade, CoverGrade::HighCover);
    assert!(
        !peek_shot.barriers.is_empty(),
        "Corner-peek shot through a Car must still evaluate intervening barriers"
    );

    // 4. DerelictCorridor & Glass windows vs SheetSteel grilles in inspect.
    let derelict = generate_procedural_sector(SectorPreset::DerelictCorridor, 20, 14, 7);
    let derelict_windows: Vec<_> = derelict
        .wall_edges
        .iter()
        .filter(|e| e.kind == WallEdgeKind::Window)
        .collect();
    if !derelict_windows.is_empty() {
        assert!(
            derelict_windows
                .iter()
                .all(|e| e.material == MaterialClass::Glass),
            "DerelictCorridor windows should be Glass, got {derelict_windows:?}"
        );
    }

    let mut req = build_procedural_map_request(
        "inspect-grille",
        SectorPreset::OfficeFloor,
        None,
        12,
        10,
        1,
    );
    req.wall_edges.clear();
    req.props.clear();
    req.wall_edges.push(MapWallEdge {
        x: 5,
        y: 5,
        side: EdgeSide::North,
        kind: WallEdgeKind::Window,
        material: MaterialClass::SheetSteel,
    });
    let hit = inspect_map_point(&req, "Grille Test", 5.5, 5.02).expect("inspect steel grille");
    assert_eq!(hit.title, "Iron / Steel Security Grille");
}

#[test]
fn wave2_zoning_hospital_park_reach_and_whisper_redaction() {
    use crate::map_narrative_manifest::{
        evaluate_passive_movement_perceptions, extract_sector_narrative_manifest_for_floor,
    };
    use satya_schema::{MessageVisibility, TokenPlacement};

    // 1. ProjectsHighDensity, compact Hospital, and WoodedPark layout checks.
    for seed in 1_i64..=8 {
        let proj =
            generate_procedural_sector(SectorPreset::ProjectsHighDensity, 24, 18, seed);
        assert!(
            all_walkable_areas_connected(24, 18, &proj.wall_edges, &proj.props),
            "ProjectsHighDensity seed={seed} disconnected"
        );
        let hosp = generate_procedural_sector(SectorPreset::Hospital, 14, 16, seed);
        for p in hosp.props.iter().filter(|p| p.kind == PropKind::ElevatorCar) {
            assert!(
                !crate::tactical_map_geom::prop_crosses_wall_edge(p, &hosp.wall_edges),
                "Compact Hospital seed={seed} ElevatorCar crosses wall: {p:?}"
            );
        }
        let park = generate_procedural_sector(SectorPreset::WoodedPark, 24, 18, seed);
        assert!(
            park.props
                .iter()
                .any(|p| matches!(p.kind, PropKind::PicnicTable | PropKind::ParkBench)),
            "WoodedPark seed={seed} missing plaza seating"
        );
    }

    // 2. Knight-step blocking & TOKEN_RADIUS_M (0.30m) clearance in compute_reachable_continuous.
    let pillar = make_prop(PropKind::StructuralPillar, 6, 5, 0, None, 0);
    let props = [pillar];
    let scene = TacticalSceneView {
        width_m: 12.0,
        height_m: 12.0,
        edges: &[],
        props: &props,
        surfaces: &[],
    };
    let field = compute_reachable_continuous(
        &scene,
        &[],
        GaitReachQuery {
            start_m: (4.5, 5.5),
            spd_eff: 4,
            hgt_m: 1.75,
            gait: CombatMovementGait::CombatGlideTacticalWalk,
            tick_budget: 2,
        },
    );
    // (5.75, 5.5) is only 0.25m from StructuralPillar west face at x=6.0m -> must be blocked by TOKEN_RADIUS_M (0.30m).
    assert!(
        field.distance_m_at(5.80, 5.5).is_none(),
        "Reach field must respect TOKEN_RADIUS_M=0.30m around solid props"
    );

    // 3. GmOnly token whisper redaction + decorative HazardSpill guard + multi-floor manifest.
    let mut map = build_procedural_map_request("whisp", SectorPreset::OfficeFloor, None, 20, 14, 5);
    map.props.clear();
    let mut dec_spill = make_prop(PropKind::HazardSpill, 6, 6, 0, None, 0);
    dec_spill.decorative = true;
    map.props.push(dec_spill);
    map.tokens = vec![
        TokenPlacement {
            entity_id: "pc_1".to_owned(),
            label: "Rook".to_owned(),
            x: 5,
            y: 6,
            visibility: MessageVisibility::Public,
        },
        TokenPlacement {
            entity_id: "npc_secret".to_owned(),
            label: "TOP_SECRET_ASSASSIN_99".to_owned(),
            x: 7,
            y: 6,
            visibility: MessageVisibility::GmOnly,
        },
    ];
    let whispers = evaluate_passive_movement_perceptions(&map, "pc_1", &[], &[]);
    assert!(
        whispers
            .iter()
            .all(|w| !w.whisper_text.contains("TOP_SECRET_ASSASSIN_99")
                && !w.whisper_text.contains("hazard spill")),
        "GmOnly label or decorative HazardSpill leaked in whispers: {whispers:?}"
    );
    let mf = extract_sector_narrative_manifest_for_floor(&map, Some(0));
    assert_eq!(mf.active_floor, 0);
}

#[test]
fn test_all_presets_never_panic_at_small_sector_dimensions() {
    for preset in SectorPreset::ALL {
        for (w, h) in [(16, 12), (12, 10), (8, 8)] {
            let sec = generate_procedural_sector(preset, w, h, 17);
            assert!(
                !sec.props.is_empty() || !sec.wall_edges.is_empty() || !sec.surfaces.is_empty(),
                "{preset:?} at ({w}, {h}) produced an empty sector"
            );
            if preset == SectorPreset::Hospital && (w, h) == (16, 12) {
                assert_eq!(sec.floors.len(), 2, "Hospital 16x12 should stay multi-floor");
            }
        }
    }
}

#[test]
fn test_wave5_awakened_presets_spatial_bridges_and_blast_breach() {
    use crate::barrier_ballistics::evaluate_spatial_blast_and_breach;
    use crate::tactical_map_inspect::{evaluate_surveillance_detection_at_point, inspect_map_point};
    use satya_engine::explosives_and_arsenal::ExplosiveOrdnanceId;
    use satya_schema::{
        GmMapUpdateRequest, MapSurveillanceNode, MessageVisibility, TokenPlacement,
    };

    // 1. All 4 Awakened 2035 presets generate cleanly and attach spatial metadata.
    let bunker_req = build_procedural_map_request_with_options(
        "map-bunker",
        SectorPreset::CorporateServerBunker,
        None,
        24,
        18,
        2035,
        None,
        None,
    );
    assert!(
        !bunker_req.faraday_zones.is_empty(),
        "CorporateServerBunker should declare a Faraday vault zone"
    );
    assert!(
        !bunker_req.surveillance_nodes.is_empty(),
        "CorporateServerBunker should declare surveillance nodes"
    );
    let fz = &bunker_req.faraday_zones[0];
    let fz_x_m = (fz.x0_cm + fz.x1_cm) as f32 / 200.0;
    let fz_y_m = (fz.y0_cm + fz.y1_cm) as f32 / 200.0;
    let bunker_ins = inspect_map_point(&bunker_req, "Corporate Bunker", fz_x_m, fz_y_m).unwrap();
    assert!(bunker_ins.in_faraday_zone);
    assert!(bunker_ins.acoustic_surface.is_some());
    assert!(bunker_ins.traction_tier.is_some());

    let clinic_req = build_procedural_map_request_with_options(
        "map-clinic-annex",
        SectorPreset::CommunityClinicAnnex,
        None,
        24,
        18,
        2036,
        None,
        None,
    );
    assert!(
        clinic_req.floors.iter().any(|f| f.floor_index == -1),
        "CommunityClinicAnnex 24x18 should include B1 seed vault floor"
    );
    let mut clinic_b1 = clinic_req.clone();
    assert!(clinic_b1.switch_active_floor(-1));
    let c_fz = &clinic_b1.faraday_zones[0];
    let clinic_ins = inspect_map_point(
        &clinic_b1,
        "Clinic Vault",
        (c_fz.x0_cm + c_fz.x1_cm) as f32 / 200.0,
        (c_fz.y0_cm + c_fz.y1_cm) as f32 / 200.0,
    )
    .unwrap();
    assert!(clinic_ins.in_faraday_zone);

    let safehouse_req = build_procedural_map_request_with_options(
        "map-safehouse",
        SectorPreset::SatyangSafehouse,
        None,
        24,
        18,
        2037,
        None,
        None,
    );
    assert!(!safehouse_req.faraday_zones.is_empty());

    let depot_req = build_procedural_map_request_with_options(
        "map-depot",
        SectorPreset::BrownoutLogisticsDepot,
        None,
        24,
        18,
        2038,
        None,
        None,
    );
    assert!(!depot_req.surveillance_nodes.is_empty());

    // 2. Surveillance cone LoS detection respects powered flag and intervening walls.
    let mut surv_map = GmMapUpdateRequest {
        map_id: "map-surv".to_string(),
        title: "Surveillance Test".to_string(),
        grid_width: 12,
        grid_height: 10,
        surveillance_nodes: vec![MapSurveillanceNode {
            node_id: "cam-east".to_string(),
            label: "Hallway Cam".to_string(),
            node_kind_code: "DOME_CAMERA".to_string(),
            floor_index: 0,
            x_cm: 200,
            y_cm: 550,
            facing_deg: 90,
            fov_deg: 90,
            range_cm: 600,
            powered: true,
            visibility: MessageVisibility::GmOnly,
        }],
        ..Default::default()
    };
    let hits_open = evaluate_surveillance_detection_at_point(&surv_map, 5.5, 5.5, 0, false);
    assert_eq!(hits_open.len(), 1);
    assert_eq!(hits_open[0].node_id, "cam-east");
    assert!(
        evaluate_surveillance_detection_at_point(&surv_map, 5.5, 5.5, 0, true).is_empty(),
        "Blackout suppresses powered surveillance cameras"
    );

    // Wall across (4, 5, WEST) blocks camera LoS through cell center (5.5m)
    surv_map.wall_edges.push(MapWallEdge {
        x: 4,
        y: 5,
        side: EdgeSide::West,
        kind: WallEdgeKind::Wall,
        material: MaterialClass::ConcreteBlock,
    });
    assert!(evaluate_surveillance_detection_at_point(&surv_map, 5.5, 5.5, 0, false).is_empty());

    // Unpowered camera detects nothing even with clear LoS
    surv_map.wall_edges.clear();
    surv_map.surveillance_nodes[0].powered = false;
    assert!(evaluate_surveillance_detection_at_point(&surv_map, 5.5, 5.5, 0, false).is_empty());

    // 3. Enclosed-room blast overpressure and drywall breach
    let mut blast_map = GmMapUpdateRequest {
        map_id: "map-blast".to_string(),
        title: "Enclosed Room".to_string(),
        grid_width: 12,
        grid_height: 10,
        wall_edges: vec![
            MapWallEdge {
                x: 4,
                y: 3,
                side: EdgeSide::North,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::Drywall,
            },
            MapWallEdge {
                x: 4,
                y: 5,
                side: EdgeSide::North,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::Drywall,
            },
            MapWallEdge {
                x: 4,
                y: 3,
                side: EdgeSide::West,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::ConcreteBlock,
            },
            MapWallEdge {
                x: 4,
                y: 4,
                side: EdgeSide::West,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::ConcreteBlock,
            },
            MapWallEdge {
                x: 5,
                y: 3,
                side: EdgeSide::West,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::ConcreteBlock,
            },
            MapWallEdge {
                x: 5,
                y: 4,
                side: EdgeSide::West,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::ConcreteBlock,
            },
        ],
        tokens: vec![TokenPlacement {
            entity_id: "tok-target".to_string(),
            label: "Guard".to_string(),
            x: 4,
            y: 4,
            visibility: MessageVisibility::Public,
        }],
        ..Default::default()
    };
    let outcome = evaluate_spatial_blast_and_breach(
        &mut blast_map,
        (4.5, 4.9),
        0,
        ExplosiveOrdnanceId::C4SemtexBlock500g,
        2,
        true,
    );
    assert!(outcome.enclosed_room);
    assert!(outcome.breached_edges_count >= 1);
    assert_eq!(outcome.token_exposures.len(), 1);
}
