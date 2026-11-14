//! # Continuous Metric Tactical Map, Procedural Sectors, Analytical LoS/Cover & Gait Reach
//!
//! Deterministic spatial geometry engine shared across `satya-engine`, `satya-signaling`, and `satya-web`:
//! - **Procedural Metric Sector Templates**: [`SectorPreset`] (`ResidentialApartment`, `OfficeFloor`,
//!   `CornerBar`, `UrbanCrossroads`, `WoodedPark`, `GardenEstate`, `WoodlandCreek`, `RoadsideDitch`,
//!   `DockingBay`, `MedLabSector`, `DerelictCorridor`, `OrbitalAirlock`) — all emitting thin
//!   [`MapWallEdge`]s and `360°` oriented multi-cell props (no tile matrix).
//! - **Analytical Raycast & Corner-Peek Cover**: [`evaluate_los_continuous`] computing metric range
//!   (`distance_m`), [`CoverGrade`] (`Clear`, `Concealment`, `LowCover`, `HighCover`, `Blocked`), and
//!   recommended $\Delta_{\text{ext}}$.
//! - **Continuous `0.25m` Any-Angle Gait Reachability**: [`compute_reachable_continuous`] coupling
//!   [`CombatMovementGait`] and [`compute_stride_and_sprint_velocity`] to `16`-directional `0.25m`
//!   Dijkstra pathing merged into smooth [`ReachableStrip`] contours.

use satya_schema::tactical_map_dto::MapScalePreset;
use satya_schema::{
    CoverGrade, GmMapUpdateRequest, MapPropPlacement, MapSurfaceFeature, MapWallEdge,
    MessageVisibility, SurfaceFeatureKind, TokenPlacement, TokenVisualSpec, WallEdgeKind,
};
use serde::{Deserialize, Serialize};

use crate::define_coded_enum;
use crate::map_dressing::PropObstruction;
use crate::map_interior_gen::{InteriorStyle, generate_furnished_interior};
use crate::map_outdoor_gen::{OutdoorStyle, generate_outdoor_sector};
use crate::map_sector_gen::{FacilitySectorStyle, generate_facility_sector};
use crate::map_surface_geom::topmost_surface_at_m;
pub use crate::map_continuation::*;
pub use crate::tactical_map_edges::{EdgeObstruction, WallEdgeIndex};
use crate::tactical_map_geom::{
    circle_collides_obstacles, intersect_segment_edge, intersect_segment_prop,
};
pub use crate::tactical_map_reach::*;

/// Borrowed view of everything continuous spatial rules need: board extents in meters,
/// thin wall/door/window edges, `360°` oriented props, and vector ground surfaces.
#[derive(Debug, Clone, Copy)]
pub struct TacticalSceneView<'a> {
    pub width_m: f32,
    pub height_m: f32,
    pub edges: &'a [MapWallEdge],
    pub props: &'a [MapPropPlacement],
    pub surfaces: &'a [MapSurfaceFeature],
}

impl<'a> TacticalSceneView<'a> {
    /// Constructs a [`TacticalSceneView`] from a wire [`GmMapUpdateRequest`].
    #[must_use]
    pub fn from_map(map: &'a GmMapUpdateRequest) -> Self {
        let (width_m, height_m) = map.board_size_m();
        Self {
            width_m,
            height_m,
            edges: &map.wall_edges,
            props: &map.props,
            surfaces: &map.surfaces,
        }
    }
}

/// Builds the [`WallEdgeIndex`] for legacy helpers needing per-cell edge lookups.
#[must_use]
pub fn wall_edge_index_for_map(map: &GmMapUpdateRequest) -> WallEdgeIndex {
    WallEdgeIndex::from_edges(map.grid_width, map.grid_height, &map.wall_edges)
}

define_coded_enum! {
    /// Canonical procedural sector layout presets for `1.0m` tactical battlemaps.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum SectorPreset [count = 12] {
        #[default]
        ResidentialApartment => {
            code: "residential_apartment",
            label: "Residential Apartment",
            subtitle: "Furnished multi-room apartment with drywall partitions, doors & windows",
            aliases: ["apartment", "flat", "house", "residential"],
        },
        OfficeFloor => {
            code: "office_floor",
            label: "Corporate Office Suite",
            subtitle: "Open-plan office bullpen with glass conference room, reception lounge & server closet",
            aliases: ["office", "corporate", "agency", "commercial", "suite"],
        },
        CornerBar => {
            code: "corner_bar",
            label: "Neighborhood Corner Bar",
            subtitle: "Neighborhood bar with a long counter, booths, pool table & back rooms",
            aliases: ["bar", "pub", "tavern", "saloon"],
        },
        UrbanCrossroads => {
            code: "urban_crossroads",
            label: "Urban Street Crossroads",
            subtitle: "Four-way city intersection with furnished corner storefronts, sidewalks & vehicle cover",
            aliases: ["urban", "street", "city", "crossroads"],
        },
        WoodedPark => {
            code: "wooded_park",
            label: "Wooded Park & Trail",
            subtitle: "Tree-lined municipal park with flagstone plaza, hedges, boulders & roadside pullout",
            aliases: ["park", "woods", "forest", "trail", "clearing"],
        },
        GardenEstate => {
            code: "garden_estate",
            label: "Walled Estate & Courtyard",
            subtitle: "Gated compound with driveway vehicles, landscaped courtyard, hedges & furnished pavilion",
            aliases: ["estate", "compound", "courtyard", "garden", "yard", "villa"],
        },
        WoodlandCreek => {
            code: "woodland_creek",
            label: "Woodland Creek & Ravine",
            subtitle: "Meandering forest creek with gravel shoals, fallen-log & stone crossings, boulders & dense canopy",
            aliases: ["creek", "stream", "brook", "river", "ravine"],
        },
        RoadsideDitch => {
            code: "roadside_ditch",
            label: "Country Lane & Drainage Ditch",
            subtitle: "Gravel country lane bordered by a sunken drainage ditch, culvert bridge, garden beds & timber cover",
            aliases: ["ditch", "trench", "lane", "country_road", "culvert"],
        },
        DockingBay => {
            code: "docking_bay",
            label: "Industrial Logistics & Loading Bay",
            subtitle: "Warehouse cargo deck with elevated catwalk, shipping office, utility van & hazmat spill",
            aliases: ["cargo", "hangar", "bay", "warehouse", "industrial", "plant"],
        },
        MedLabSector => {
            code: "med_lab",
            label: "Trauma Clinic & Bio-Containment Lab",
            subtitle: "Partitioned triage ward, surgical bay & quarantine containment lab with bio-spill",
            aliases: ["medlab", "medical", "lab", "clinic", "hospital"],
        },
        DerelictCorridor => {
            code: "derelict_corridor",
            label: "Breached Utility Tunnel & Sub-Level",
            subtitle: "Segmented concrete service spine with flanking conduit bays, blast doors & chemical rupture",
            aliases: ["corridor", "derelict", "breach", "tunnel", "bunker"],
        },
        OrbitalAirlock => {
            code: "orbital_airlock",
            label: "Pressurized Transfer Airlock",
            subtitle: "Armored transfer chamber with dual airlock vestibules, security booths & vent hazard",
            aliases: ["airlock", "sector4_airlock", "station"],
        },
    }
}

impl SectorPreset {
    /// Furnished edge-wall interior style (`None` for outdoor/facility presets).
    #[must_use]
    pub const fn interior_style(self) -> Option<InteriorStyle> {
        match self {
            Self::ResidentialApartment => Some(InteriorStyle::Apartment),
            Self::OfficeFloor => Some(InteriorStyle::Office),
            Self::CornerBar => Some(InteriorStyle::Bar),
            _ => None,
        }
    }

    /// Multi-cell outdoor / compound style (`None` for indoor/facility presets).
    #[must_use]
    pub const fn outdoor_style(self) -> Option<OutdoorStyle> {
        match self {
            Self::WoodedPark => Some(OutdoorStyle::WoodedPark),
            Self::GardenEstate => Some(OutdoorStyle::GardenEstate),
            Self::WoodlandCreek => Some(OutdoorStyle::WoodlandCreek),
            Self::RoadsideDitch => Some(OutdoorStyle::RoadsideDitch),
            _ => None,
        }
    }

    /// Whether this preset generates architectural [`MapWallEdge`] boundaries.
    #[must_use]
    pub const fn has_structural_wall_edges(self) -> bool {
        !matches!(self, Self::WoodedPark | Self::WoodlandCreek | Self::RoadsideDitch)
    }

    /// Upgraded facility / clinical / transit / urban sector style (`None` for residential/outdoor presets).
    #[must_use]
    pub const fn facility_style(self) -> Option<FacilitySectorStyle> {
        match self {
            Self::UrbanCrossroads => Some(FacilitySectorStyle::UrbanCrossroads),
            Self::DockingBay => Some(FacilitySectorStyle::DockingBay),
            Self::MedLabSector => Some(FacilitySectorStyle::MedLab),
            Self::DerelictCorridor => Some(FacilitySectorStyle::DerelictCorridor),
            Self::OrbitalAirlock => Some(FacilitySectorStyle::OrbitalAirlock),
            _ => None,
        }
    }

    /// Natural default battlemap dimensions `(width_m, height_m)` for this sector type when [`MapScalePreset::Auto`] is selected.
    #[must_use]
    pub const fn default_dimensions(self) -> (i32, i32) {
        match self {
            Self::ResidentialApartment
            | Self::CornerBar
            | Self::DerelictCorridor
            | Self::OrbitalAirlock => (24, 18),
            Self::OfficeFloor
            | Self::UrbanCrossroads
            | Self::WoodedPark
            | Self::WoodlandCreek
            | Self::RoadsideDitch
            | Self::MedLabSector => (36, 26),
            Self::GardenEstate | Self::DockingBay => (48, 36),
        }
    }

    /// Resolves clamped single-sector `(width_m, height_m)` for `scale` (`Auto` delegates to [`Self::default_dimensions`]).
    #[must_use]
    pub fn resolve_scale_dimensions(self, scale: MapScalePreset) -> (i32, i32) {
        let (w, h) = scale.resolve_dimensions(self.default_dimensions());
        (
            w.clamp(MIN_SECTOR_DIM, MAX_SECTOR_DIM),
            h.clamp(MIN_SECTOR_DIM, MAX_SECTOR_DIM),
        )
    }
}

/// Minimum allowed width or height (cells = meters) for a single generated sector.
pub const MIN_SECTOR_DIM: i32 = 8;
/// Maximum allowed width or height (cells = meters) for a single generated sector.
pub const MAX_SECTOR_DIM: i32 = 96;

/// Complete procedural sector: thin edge walls, `360°` oriented props, and vector ground surfaces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProceduralSector {
    pub wall_edges: Vec<MapWallEdge>,
    pub props: Vec<MapPropPlacement>,
    pub surfaces: Vec<MapSurfaceFeature>,
}

/// Generates the complete [`ProceduralSector`] for `preset`, emitting thin edge walls and props.
#[must_use]
pub fn generate_procedural_sector(preset: SectorPreset, width: i32, height: i32, seed: i64) -> ProceduralSector {
    let (w, h) = (
        width.clamp(MIN_SECTOR_DIM, MAX_SECTOR_DIM),
        height.clamp(MIN_SECTOR_DIM, MAX_SECTOR_DIM),
    );
    if let Some(style) = preset.interior_style() {
        return generate_furnished_interior(style, w, h, seed);
    }
    if let Some(style) = preset.outdoor_style() {
        return generate_outdoor_sector(style, w, h, seed);
    }
    if let Some(style) = preset.facility_style() {
        return generate_facility_sector(style, w, h, seed);
    }
    ProceduralSector::default()
}

/// Complete Line-of-Sight, metric range, and cover evaluation between two continuous points in meters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalLosEvaluation {
    pub attacker_m: (f32, f32),
    pub target_m: (f32, f32),
    /// Euclidean distance in meters.
    pub distance_m: f32,
    pub has_los: bool,
    pub cover_grade: CoverGrade,
    /// Recommended Satya Ranged External Modifier (`+0`, `+3`, `+4`, `+6`, or `+16` when blocked).
    pub recommended_delta_ext: i32,
    /// Sampled sightline segment `(start_m, target_m)` for 2D/3D overlay drawing.
    pub ray_points_m: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Copy, Default)]
struct RayObstruction {
    blocked: bool,
    half_cover: bool,
    concealment: bool,
}

fn eval_continuous_ray(
    scene: &TacticalSceneView<'_>,
    origin_m: (f32, f32),
    target_m: (f32, f32),
) -> RayObstruction {
    let mut obs = RayObstruction::default();
    let dx = target_m.0 - origin_m.0;
    let dy = target_m.1 - origin_m.1;
    let seg_len = (dx * dx + dy * dy).sqrt();
    if seg_len <= 1e-4 {
        return obs;
    }

    for edge in scene.edges {
        if let Some(t) = intersect_segment_edge(origin_m, target_m, edge) {
            let hit_dist = t * seg_len;
            match edge.kind {
                WallEdgeKind::Wall | WallEdgeKind::DoorClosed => {
                    obs.blocked = true;
                    return obs;
                }
                WallEdgeKind::Window => {
                    if hit_dist > 0.85 {
                        obs.half_cover = true;
                    }
                }
                WallEdgeKind::DoorOpen => {}
            }
        }
    }

    for p in scene.props {
        if let Some(hit) = intersect_segment_prop(p, origin_m, target_m) {
            let enter_dist = hit.t_enter * seg_len;
            let braced_skip = hit.obstruction != PropObstruction::FullObstacle
                && enter_dist <= 0.85
                && hit.t_exit < 0.99;
            if braced_skip {
                continue;
            }
            match hit.obstruction {
                PropObstruction::FullObstacle => {
                    obs.blocked = true;
                    return obs;
                }
                PropObstruction::HalfCover => obs.half_cover = true,
                PropObstruction::Concealment => obs.concealment = true,
                PropObstruction::FloorLayer => {}
            }
        }
    }

    if !obs.half_cover
        && seg_len > 1.2
        && topmost_surface_at_m(scene.surfaces, target_m.0, target_m.1)
            .is_some_and(|s| s.kind == SurfaceFeatureKind::DefensiveTrench)
        && !topmost_surface_at_m(scene.surfaces, origin_m.0, origin_m.1)
            .is_some_and(|s| s.kind == SurfaceFeatureKind::DefensiveTrench)
    {
        obs.half_cover = true;
    }
    obs
}

/// Analytical continuous Line-of-Sight, corner-peek, and cover evaluation between `attacker_m` and `target_m`.
#[must_use]
pub fn evaluate_los_continuous(
    scene: &TacticalSceneView<'_>,
    attacker_m: (f32, f32),
    target_m: (f32, f32),
) -> TacticalLosEvaluation {
    let dx = target_m.0 - attacker_m.0;
    let dy = target_m.1 - attacker_m.1;
    let raw_dist = (dx * dx + dy * dy).sqrt();
    let distance_m = (raw_dist * 100.0).round() / 100.0;

    if raw_dist <= 0.05 {
        return TacticalLosEvaluation {
            attacker_m,
            target_m,
            distance_m: 0.0,
            has_los: true,
            cover_grade: CoverGrade::Clear,
            recommended_delta_ext: 0,
            ray_points_m: Vec::new(),
        };
    }

    let primary_obs = eval_continuous_ray(scene, attacker_m, target_m);
    if !primary_obs.blocked {
        let grade = if primary_obs.half_cover {
            CoverGrade::LowCover
        } else if primary_obs.concealment {
            CoverGrade::Concealment
        } else {
            CoverGrade::Clear
        };
        return TacticalLosEvaluation {
            attacker_m,
            target_m,
            distance_m,
            has_los: true,
            cover_grade: grade,
            recommended_delta_ext: grade.recommended_delta_ext(),
            ray_points_m: vec![attacker_m, target_m],
        };
    }

    // Perpendicular ±0.65m / ±0.85m step-out lean rays (plus fallback orthogonal ±0.65m).
    let ux = dx / raw_dist;
    let uy = dy / raw_dist;
    let leans = [
        (-uy * 0.65, ux * 0.65),
        (uy * 0.65, -ux * 0.65),
        (-uy * 0.85, ux * 0.85),
        (uy * 0.85, -ux * 0.85),
        (-0.65, 0.0),
        (0.65, 0.0),
        (0.0, -0.65),
        (0.0, 0.65),
    ];
    for &(ox, oy) in &leans {
        let peek = (attacker_m.0 + ox, attacker_m.1 + oy);
        if circle_collides_obstacles(scene.width_m, scene.height_m, scene.edges, scene.props, peek.0, peek.1, 0.18) {
            continue;
        }
        let lean_clear = !eval_continuous_ray(scene, attacker_m, peek).blocked;
        if !lean_clear {
            continue;
        }
        if !eval_continuous_ray(scene, peek, target_m).blocked {
            return TacticalLosEvaluation {
                attacker_m,
                target_m,
                distance_m,
                has_los: true,
                cover_grade: CoverGrade::HighCover,
                recommended_delta_ext: CoverGrade::HighCover.recommended_delta_ext(),
                ray_points_m: vec![peek, target_m],
            };
        }
    }

    TacticalLosEvaluation {
        attacker_m,
        target_m,
        distance_m,
        has_los: false,
        cover_grade: CoverGrade::Blocked,
        recommended_delta_ext: CoverGrade::Blocked.recommended_delta_ext(),
        ray_points_m: vec![attacker_m, target_m],
    }
}

/// Builds a complete vector [`GmMapUpdateRequest`] with continuous centimeter token coordinates,
/// thin structural [`MapWallEdge`]s, `360°` oriented props, vector ground surfaces, and `tiles: Vec::new()`.
#[must_use]
pub fn build_procedural_map_request(
    map_id: &str,
    preset: SectorPreset,
    title_override: Option<&str>,
    width: i32,
    height: i32,
    seed: i64,
) -> GmMapUpdateRequest {
    let w = width.clamp(MIN_SECTOR_DIM, MAX_SECTOR_DIM);
    let h = height.clamp(MIN_SECTOR_DIM, MAX_SECTOR_DIM);
    let ProceduralSector { wall_edges, props, surfaces } = generate_procedural_sector(preset, w, h, seed);
    let mid_y = h / 2;
    let title = title_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| preset.full_name())
        .to_string();

    let sara_y = (mid_y - 1).max(1);
    let tokens = vec![
        TokenPlacement {
            entity_id: "tok-sara".to_string(),
            label: "Sara (Valkyrie)".to_string(),
            x: 2,
            y: sara_y,
            visibility: MessageVisibility::Public,
        },
        TokenPlacement {
            entity_id: "tok-marcus".to_string(),
            label: "Marcus (Recon)".to_string(),
            x: 2,
            y: mid_y,
            visibility: MessageVisibility::Public,
        },
        TokenPlacement {
            entity_id: "tok-secops-1".to_string(),
            label: "SecOps Sentry".to_string(),
            x: w - 3,
            y: mid_y,
            visibility: MessageVisibility::Public,
        },
    ];
    let token_visuals = vec![
        TokenVisualSpec {
            entity_id: "tok-sara".to_string(),
            model_preset: "valkyrie_rig".to_string(),
            color_hex: "#38bdf8".to_string(),
            facing_deg: 90,
            elevation_cm: 0,
            stance_pose: "aimed".to_string(),
            portrait_url: None,
            pos_x_cm: Some(250),
            pos_y_cm: Some(sara_y * 100 + 50),
        },
        TokenVisualSpec {
            entity_id: "tok-marcus".to_string(),
            model_preset: "recon_mk4".to_string(),
            color_hex: "#22d3ee".to_string(),
            facing_deg: 90,
            elevation_cm: 0,
            stance_pose: "idle".to_string(),
            portrait_url: None,
            pos_x_cm: Some(250),
            pos_y_cm: Some(mid_y * 100 + 50),
        },
        TokenVisualSpec {
            entity_id: "tok-secops-1".to_string(),
            model_preset: "secops_sentry".to_string(),
            color_hex: "#f87171".to_string(),
            facing_deg: 270,
            elevation_cm: 0,
            stance_pose: "aimed".to_string(),
            portrait_url: None,
            pos_x_cm: Some((w - 3) * 100 + 50),
            pos_y_cm: Some(mid_y * 100 + 50),
        },
    ];

    GmMapUpdateRequest {
        map_id: map_id.to_string(),
        title,
        image_url: String::new(),
        grid_width: w,
        grid_height: h,
        tokens,
        notes: Some(format!(
            "{} · Continuous Metric Arena ({}m x {}m, Seed {})",
            preset.subtitle(),
            w,
            h,
            seed
        )),
        cell_size_cm: 100,
        theme_preset: Some(preset.as_str().to_string()),
        seed: Some(seed),
        tiles: Vec::new(),
        token_visuals,
        wall_edges,
        props,
        surfaces,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment_vehicles::CombatMovementGait;
    use crate::map_dressing::{PropObstruction, make_prop, prop_spec};
    use crate::tactical_map_geom::{TOKEN_RADIUS_M, all_walkable_areas_connected};
    use satya_schema::{EdgeSide, MaterialClass, PropKind};

    #[test]
    fn test_procedural_sector_presets_and_continuous_los_cover() {
        for preset in SectorPreset::ALL {
            let sec = generate_procedural_sector(preset, 20, 14, 42);
            if preset.has_structural_wall_edges() {
                assert!(!sec.wall_edges.is_empty(), "{preset:?} missing wall_edges");
            }
            assert!(!sec.props.is_empty(), "{preset:?} missing props");
            assert!(
                sec.props.iter().any(|p| prop_spec(p.kind).obstruction == PropObstruction::HalfCover),
                "{preset:?} missing half cover prop"
            );
            assert!(
                all_walkable_areas_connected(20, 14, &sec.wall_edges, &sec.props),
                "{preset:?} disconnected walkable area"
            );
            // West and East deployment pads must remain clear of blocking obstacles.
            assert!(!circle_collides_obstacles(20.0, 14.0, &sec.wall_edges, &sec.props, 2.5, 7.5, TOKEN_RADIUS_M));
            assert!(!circle_collides_obstacles(20.0, 14.0, &sec.wall_edges, &sec.props, 17.5, 7.5, TOKEN_RADIUS_M));
        }

        // Clear LoS
        let empty = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &[], props: &[], surfaces: &[] };
        let clear = evaluate_los_continuous(&empty, (2.5, 4.5), (9.5, 4.5));
        assert!(clear.has_los);
        assert_eq!(clear.cover_grade, CoverGrade::Clear);
        assert_eq!(clear.recommended_delta_ext, 0);
        assert!((clear.distance_m - 7.0).abs() < 0.01);

        // Mid-field JerseyBarrier -> LowCover (+4)
        let barrier = make_prop(PropKind::JerseyBarrier, 6, 4, 90, Some(1), 0);
        let low_scene = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &[], props: &[barrier], surfaces: &[] };
        let low = evaluate_los_continuous(&low_scene, (2.5, 4.5), (9.5, 4.5));
        assert!(low.has_los && low.cover_grade == CoverGrade::LowCover && low.recommended_delta_ext == 4);

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
        let pillar_scene = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &[], props: &[pillar], surfaces: &[] };
        let corner = evaluate_los_continuous(&pillar_scene, (5.6, 4.5), (9.5, 4.5));
        assert!(corner.has_los && corner.cover_grade == CoverGrade::HighCover && corner.recommended_delta_ext == 6);

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
        let blocked_scene = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &wall, props: &[], surfaces: &[] };
        let blocked = evaluate_los_continuous(&blocked_scene, (2.5, 4.5), (9.5, 4.5));
        assert!(!blocked.has_los && blocked.cover_grade == CoverGrade::Blocked);
    }

    #[test]
    fn test_metric_gait_reach_scales_continuously() {
        let scene = TacticalSceneView { width_m: 20.0, height_m: 14.0, edges: &[], props: &[], surfaces: &[] };
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
                assert!(!sec.surfaces.is_empty(), "{preset:?} {w}x{h} missing surfaces");
                assert!(
                    all_walkable_areas_connected(w, h, &sec.wall_edges, &sec.props),
                    "{preset:?} {w}x{h} disconnected walkable area"
                );
                assert!(
                    !circle_collides_obstacles(w as f32, h as f32, &sec.wall_edges, &sec.props, 2.5, wy, TOKEN_RADIUS_M),
                    "{preset:?} {w}x{h} blocked West spawn pad"
                );
                assert!(
                    !circle_collides_obstacles(w as f32, h as f32, &sec.wall_edges, &sec.props, w as f32 - 2.5, wy, TOKEN_RADIUS_M),
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
}
