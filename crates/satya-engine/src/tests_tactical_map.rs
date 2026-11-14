//! Consolidated continuous metric tactical map, procedural continuation, interior generation,
//! and zoned multi-floor tests (`tests_tactical_map`).

use std::collections::BTreeSet;

use satya_schema::{
    CoverGrade, EconomicTier, EdgeSide, MapContinuationDir, MapWallEdge, MaterialClass, PropKind,
    SurfaceFeatureKind, WallEdgeKind, ZoningContext,
};

use crate::environment_vehicles::CombatMovementGait;
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
    TOKEN_RADIUS_M, all_walkable_areas_connected, circle_collides_obstacles,
};

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
        low.has_los
            && low.cover_grade == CoverGrade::LowCover
            && low.recommended_delta_ext == 4
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
    (sec.wall_edges.len(), sec.surfaces.len(), edge_hash, surf_hash)
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
    let apt = build_procedural_map_request(
        "apt",
        SectorPreset::ResidentialApartment,
        None,
        20,
        14,
        42,
    );
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
    let estate = build_procedural_map_request("estate", SectorPreset::GardenEstate, None, 48, 36, 99);
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
    assert!(
        counter.w_cells >= 3,
        "multi-cell bar counter: {counter:?}"
    );
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
