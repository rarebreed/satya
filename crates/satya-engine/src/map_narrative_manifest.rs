//! # Tactical Sector Narrative Manifest & Passive Sensory Whispers (`satya-engine`)
//!
//! Translates raw procedural vector battlemaps (`GmMapUpdateRequest`: thin walls, `360°` props,
//! continuous ground surfaces, `floors`, and stitched `world_sectors`) into:
//! 1. [`SectorNarrativeManifest`]: semantic zones, multi-floor directory, `N/S/E/W` boundary exits,
//!    and verified obstacle-free spawn/walkable anchors for the AI GM.
//! 2. [`evaluate_passive_movement_perceptions`]: deterministic `SIG` / `HEA` / `OLF` / `ITU` passive
//!    perception checks against [`SpatialSensoryPin`] clues, `GmOnly` tokens, and `HazardSpill` props
//!    using continuous 2D metric distance and [`evaluate_los_continuous`].

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use satya_schema::{
    CharacterSheetRecord, CoverGrade, GmMapUpdateRequest, MapContinuationDir, MessageVisibility,
    PropKind, SurfaceFeatureKind,
};

use crate::map_dressing::{PropObstruction, effective_obstruction};
use crate::map_surface_geom::topmost_surface_at_m;
use crate::map_world_graph::{bootstrap_world_sectors_if_empty, preview_continuation_tooltip};
use crate::tactical_map::{TacticalSceneView, evaluate_los_continuous};
use crate::tactical_map_geom::{TOKEN_RADIUS_M, circle_collides_obstacles, prop_contains_point};

// ============================================================================
// §1 Sector Narrative Manifest DTOs
// ============================================================================

/// High-level functional cluster distilled from vector surfaces and placed props.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrativeZoneSummary {
    pub zone_id: String,
    pub category: String,
    pub label: String,
    pub floor_index: i32,
    pub center_m: (f32, f32),
    pub feature_count: usize,
    pub cover_prop_count: usize,
    pub concealment_prop_count: usize,
}

/// Cardinal boundary summary connecting the current sector to neighboring districts/sectors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardinalExitSummary {
    pub direction: String,
    pub preview_label: String,
    pub has_road: bool,
    pub has_water: bool,
    pub crossing_count: usize,
    pub walled_pct: u8,
}

/// Verified obstacle-free coordinate anchor safe for token placement or staging.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WalkableAnchorPoint {
    pub label: String,
    pub floor_index: i32,
    pub x_m: f32,
    pub y_m: f32,
    pub x_cm: i32,
    pub y_cm: i32,
}

/// Vertical floor directory entry summarizing a sector's stacked levels (`B1`, `1F`, `2F`, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloorSummary {
    pub floor_index: i32,
    pub label: String,
    pub elevation_cm: i32,
    pub prop_count: usize,
    pub wall_count: usize,
}

/// Compact semantic overview of an active vector battlemap for AI GM grounding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorNarrativeManifest {
    pub map_id: String,
    pub title: String,
    pub preset: String,
    pub width_m: f32,
    pub height_m: f32,
    pub active_floor: i32,
    pub floors: Vec<FloorSummary>,
    pub zones: Vec<NarrativeZoneSummary>,
    pub cardinal_exits: Vec<CardinalExitSummary>,
    pub walkable_anchors: Vec<WalkableAnchorPoint>,
    pub concise_brief: String,
}

// ============================================================================
// §2 Passive Sensory Pin & Whisper DTOs
// ============================================================================

/// Sensory modality evaluated during passive token movement checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpatialSensoryChannel {
    #[default]
    Sight,
    Hearing,
    Olfaction,
    Intuition,
}

impl SpatialSensoryChannel {
    #[must_use]
    pub const fn characteristic_code(self) -> &'static str {
        match self {
            Self::Sight => "SIG",
            Self::Hearing => "HEA",
            Self::Olfaction => "OLF",
            Self::Intuition => "ITU",
        }
    }

    #[must_use]
    pub fn parse_loose(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "HEARING" | "HEA" | "AUDIO" | "SOUND" => Self::Hearing,
            "OLFACTION" | "OLF" | "SMELL" | "SCENT" => Self::Olfaction,
            "INTUITION" | "ITU" | "GUT" | "HUNCH" => Self::Intuition,
            _ => Self::Sight,
        }
    }
}

fn default_pin_tn() -> i32 {
    12
}

fn default_pin_range_m() -> f32 {
    12.0
}

/// Hidden spatial clue or environmental tell attached to continuous battlemap coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialSensoryPin {
    pub pin_id: String,
    pub x_m: f32,
    pub y_m: f32,
    #[serde(default)]
    pub floor_index: i32,
    #[serde(default)]
    pub channel: SpatialSensoryChannel,
    #[serde(default = "default_pin_tn")]
    pub difficulty_tn: i32,
    #[serde(default = "default_pin_range_m")]
    pub max_range_m: f32,
    pub whisper_text: String,
    #[serde(default)]
    pub revealed_to: Vec<String>,
}

/// Private perception output emitted when an observer passes close enough (and beats effective TN).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PassivePerceptionWhisper {
    pub pin_id: String,
    pub observer_id: String,
    pub observer_name: String,
    pub channel: SpatialSensoryChannel,
    pub characteristic_code: String,
    pub distance_m: f32,
    pub has_los: bool,
    pub passive_score: i32,
    pub effective_tn: i32,
    pub margin: i32,
    pub whisper_text: String,
}

// ============================================================================
// §3 Sector Narrative Manifest Extraction
// ============================================================================

fn prop_semantic_zone(kind: PropKind) -> Option<(&'static str, &'static str)> {
    match kind {
        PropKind::HospitalBed
        | PropKind::SurgicalTable
        | PropKind::ImagingScanner
        | PropKind::PrivacyCurtain
        | PropKind::Cot
        | PropKind::Gurney
        | PropKind::MedicalCounter
        | PropKind::TelemetryMonitor
        | PropKind::BiohazardBin
        | PropKind::LinenCart => Some(("clinical_medical", "Clinical / Triage Bay")),
        PropKind::DisplayRack | PropKind::CheckoutCounter | PropKind::MallKiosk => {
            Some(("retail_concourse", "Retail Concourse / Counter"))
        }
        PropKind::ConveyorBelt
        | PropKind::IndustrialPress
        | PropKind::PalletRack
        | PropKind::Forklift
        | PropKind::ShippingContainer
        | PropKind::AirCompressor
        | PropKind::CncMill => Some(("industrial_logistics", "Industrial / Logistics Floor")),
        PropKind::LabBench
        | PropKind::SolderingStation
        | PropKind::TestBench
        | PropKind::Printer3dStation
        | PropKind::ServerRack
        | PropKind::FumeHood => Some(("lab_workshop", "Laboratory / Technical Bay")),
        PropKind::Desk
        | PropKind::DeskL
        | PropKind::DeskT
        | PropKind::Bookshelf
        | PropKind::DeskClutter
        | PropKind::FilingCabinet
        | PropKind::WaterCooler => Some(("office_ops", "Command / Office Suite")),
        PropKind::BarCounter
        | PropKind::BackBarShelf
        | PropKind::Booth
        | PropKind::PoolTable
        | PropKind::Jukebox => Some(("hospitality_lounge", "Bar / Hospitality Lounge")),
        PropKind::Sofa
        | PropKind::Loveseat
        | PropKind::ReclinerChair
        | PropKind::Bed
        | PropKind::SingleBed
        | PropKind::DiningTable
        | PropKind::KitchenCounter
        | PropKind::Stove
        | PropKind::Fridge
        | PropKind::Nightstand
        | PropKind::Dresser
        | PropKind::Wardrobe
        | PropKind::BathroomSink
        | PropKind::Toilet
        | PropKind::Bathtub
        | PropKind::ShowerStall
        | PropKind::SpaBath
        | PropKind::LinenCloset
        | PropKind::WasherDryer
        | PropKind::TvStand
        | PropKind::Stereo
        | PropKind::RunnerRug
        | PropKind::RoundRug
        | PropKind::DeskLamp
        | PropKind::StandingLamp
        | PropKind::FloorClothes
        | PropKind::FloorToys => Some(("residential_quarters", "Living Quarters / Unit")),
        PropKind::Car
        | PropKind::UtilityVan
        | PropKind::JerseyBarrier
        | PropKind::StreetDebris
        | PropKind::AwningCanopy => Some(("vehicle_perimeter", "Curbside / Vehicle Perimeter")),
        PropKind::CropRow
        | PropKind::PlanterTable
        | PropKind::LargeTree
        | PropKind::MediumTree
        | PropKind::Hedge
        | PropKind::Bush
        | PropKind::Boulder
        | PropKind::SmallFountain
        | PropKind::MediumFountain
        | PropKind::LargeFountain
        | PropKind::Gazebo
        | PropKind::BbqGrill
        | PropKind::GardenShed
        | PropKind::HotTub
        | PropKind::BrownstoneStoop => Some(("exterior_grounds", "Exterior / Greenbelt Grounds")),
        PropKind::FireExtinguisher => Some(("office_ops", "Safety & Utility Fixture")),
        PropKind::TrashCan | PropKind::GarbagePile | PropKind::Dumpster | PropKind::AcUnit => {
            Some(("utility_service", "Service / Utility Yard"))
        }
        PropKind::Stairwell | PropKind::ElevatorCar | PropKind::ServiceElevator => {
            Some(("vertical_stairwell", "Stairwell / Vertical Access"))
        }
        PropKind::NurseStation => Some(("nurse_station", "Nursing Station / Triage Hub")),
        PropKind::HazardSpill => Some(("hazard_spill", "Hazardous Spill Zone")),
        _ => None,
    }
}

fn surface_semantic_label(kind: SurfaceFeatureKind) -> &'static str {
    match kind {
        SurfaceFeatureKind::AsphaltRoad | SurfaceFeatureKind::GravelRoad => "Roadway Corridor",
        SurfaceFeatureKind::ConcreteWalk
        | SurfaceFeatureKind::StoneWalk
        | SurfaceFeatureKind::BrickPaverWalk => "Pedestrian Plaza / Walk",
        SurfaceFeatureKind::WoodDeck => "Timber Deck / Porch",
        SurfaceFeatureKind::PoolWater => "Swimming Pool Basin",
        SurfaceFeatureKind::InteriorFloor
        | SurfaceFeatureKind::CeramicTileFloor
        | SurfaceFeatureKind::MarbleFloor
        | SurfaceFeatureKind::CarpetFloor
        | SurfaceFeatureKind::StainedConcreteFloor
        | SurfaceFeatureKind::ClinicalVinylFloor
        | SurfaceFeatureKind::LinoleumCheckFloor
        | SurfaceFeatureKind::TerrazzoFloor
        | SurfaceFeatureKind::SteelPlateFloor => "Building Interior Wing",
        SurfaceFeatureKind::BridgeDeck => "Bridge / Overpass Span",
        SurfaceFeatureKind::CreekChannel | SurfaceFeatureKind::DrainageDitch => {
            "Waterway / Drainage Cut"
        }
        SurfaceFeatureKind::DefensiveTrench => "Excavated Trench / Earthwork",
        SurfaceFeatureKind::DirtTrail => "Unpaved Dirt Trail",
        SurfaceFeatureKind::LawnGrassZone
        | SurfaceFeatureKind::GardenBedZone
        | SurfaceFeatureKind::MulchZone => "Landscaped Grounds",
    }
}

fn find_clear_anchor_near(
    map: &GmMapUpdateRequest,
    target_x_m: f32,
    target_y_m: f32,
    floor_index: i32,
    label: &str,
) -> Option<WalkableAnchorPoint> {
    let (w_m, h_m) = map.board_size_m();
    let max_r: i32 = 6;
    for ring in 0_i32..=max_r {
        for dy in -ring..=ring {
            for dx in -ring..=ring {
                if dx.abs() != ring && dy.abs() != ring {
                    continue;
                }
                let x = (target_x_m + dx as f32).clamp(1.0, (w_m - 1.0).max(1.0));
                let y = (target_y_m + dy as f32).clamp(1.0, (h_m - 1.0).max(1.0));
                let x_m = (x * 100.0).round() / 100.0;
                let y_m = (y * 100.0).round() / 100.0;
                let in_water_or_hazard = matches!(
                    topmost_surface_at_m(&map.surfaces, x_m, y_m).map(|s| s.kind),
                    Some(
                        SurfaceFeatureKind::PoolWater
                            | SurfaceFeatureKind::CreekChannel
                            | SurfaceFeatureKind::DrainageDitch
                    )
                ) || map.props.iter().any(|p| {
                    !p.decorative
                        && matches!(
                            p.kind,
                            PropKind::HazardSpill | PropKind::CreekWater | PropKind::DrainageDitch
                        )
                        && prop_contains_point(p, x_m, y_m, false)
                });
                if !in_water_or_hazard
                    && !circle_collides_obstacles(
                        w_m,
                        h_m,
                        &map.wall_edges,
                        &map.props,
                        x_m,
                        y_m,
                        TOKEN_RADIUS_M,
                    )
                {
                    return Some(WalkableAnchorPoint {
                        label: label.to_string(),
                        floor_index,
                        x_m,
                        y_m,
                        x_cm: (x_m * 100.0).round() as i32,
                        y_cm: (y_m * 100.0).round() as i32,
                    });
                }
            }
        }
    }
    None
}

/// Distill an entire procedural vector [`GmMapUpdateRequest`] into semantic zones, exits, floors, and anchors.
#[must_use]
pub fn extract_sector_narrative_manifest(map: &GmMapUpdateRequest) -> SectorNarrativeManifest {
    let (width_m, height_m) = map.board_size_m();
    let floors: Vec<FloorSummary> = if map.floors.is_empty() {
        vec![FloorSummary {
            floor_index: map.active_floor,
            label: satya_schema::default_floor_badge(map.active_floor),
            elevation_cm: map.floor_elevation_cm(map.active_floor),
            prop_count: map.props.len(),
            wall_count: map.wall_edges.len(),
        }]
    } else {
        map.floors
            .iter()
            .map(|f| FloorSummary {
                floor_index: f.floor_index,
                label: f.label.clone(),
                elevation_cm: f.elevation_cm,
                prop_count: f.props.len(),
                wall_count: f.wall_edges.len(),
            })
            .collect()
    };

    struct Acc {
        category: String,
        label: String,
        sum_x: f32,
        sum_y: f32,
        count: usize,
        cover_props: usize,
        concealment_props: usize,
    }
    let mut grouped: BTreeMap<String, Acc> = BTreeMap::new();

    for p in &map.props {
        let Some((zone_key, zone_label)) = prop_semantic_zone(p.kind) else {
            continue;
        };
        let fp = p.footprint_m();
        let obs = effective_obstruction(p);
        let entry = grouped
            .entry(format!("prop_{zone_key}"))
            .or_insert_with(|| Acc {
                category: "Interior / Prop Cluster".to_string(),
                label: zone_label.to_string(),
                sum_x: 0.0,
                sum_y: 0.0,
                count: 0,
                cover_props: 0,
                concealment_props: 0,
            });
        entry.sum_x += fp.cx_m;
        entry.sum_y += fp.cy_m;
        entry.count += 1;
        match obs {
            PropObstruction::HalfCover | PropObstruction::FullObstacle => entry.cover_props += 1,
            PropObstruction::Concealment => entry.concealment_props += 1,
            PropObstruction::FloorLayer => {}
        }
    }

    for surf in &map.surfaces {
        if surf.points_cm.is_empty() {
            continue;
        }
        let (sx, sy) = surf
            .points_cm
            .iter()
            .fold((0.0_f32, 0.0_f32), |(ax, ay), pt| {
                let (mx, my) = pt.to_m();
                (ax + mx, ay + my)
            });
        let n = surf.points_cm.len() as f32;
        let key = format!("surf_{}", surf.kind.as_str().to_ascii_lowercase());
        let entry = grouped.entry(key).or_insert_with(|| Acc {
            category: "Terrain Surface".to_string(),
            label: surface_semantic_label(surf.kind).to_string(),
            sum_x: 0.0,
            sum_y: 0.0,
            count: 0,
            cover_props: 0,
            concealment_props: 0,
        });
        entry.sum_x += sx / n;
        entry.sum_y += sy / n;
        entry.count += 1;
    }

    let mut zones: Vec<NarrativeZoneSummary> = grouped
        .into_iter()
        .map(|(zone_id, acc)| NarrativeZoneSummary {
            zone_id,
            category: acc.category,
            label: acc.label,
            floor_index: map.active_floor,
            center_m: (
                ((acc.sum_x / acc.count as f32) * 100.0).round() / 100.0,
                ((acc.sum_y / acc.count as f32) * 100.0).round() / 100.0,
            ),
            feature_count: acc.count,
            cover_prop_count: acc.cover_props,
            concealment_prop_count: acc.concealment_props,
        })
        .collect();
    zones.sort_by(|a, b| {
        b.feature_count
            .cmp(&a.feature_count)
            .then_with(|| a.zone_id.cmp(&b.zone_id))
    });

    let sectors = bootstrap_world_sectors_if_empty(map);
    let dirs = [
        MapContinuationDir::North,
        MapContinuationDir::South,
        MapContinuationDir::East,
        MapContinuationDir::West,
    ];
    let cardinal_exits: Vec<CardinalExitSummary> = dirs
        .into_iter()
        .map(|dir| {
            let mut has_road = false;
            let mut has_water = false;
            let mut crossing_count = 0_usize;
            let mut max_walled = 0_u8;
            for cell in &sectors {
                let edge = cell.boundary.edge(dir);
                has_road |= edge.has_any_road();
                has_water |= edge.has_water();
                crossing_count += edge.crossing_trails_cm.len() + edge.crossing_roads_cm.len();
                max_walled = max_walled.max(edge.walled_pct);
            }
            CardinalExitSummary {
                direction: dir.as_str().to_string(),
                preview_label: preview_continuation_tooltip(map, dir),
                has_road,
                has_water,
                crossing_count,
                walled_pct: max_walled,
            }
        })
        .collect();

    let candidate_anchors = [
        ("west_entry", 2.0_f32, height_m * 0.5),
        ("center_concourse", width_m * 0.5, height_m * 0.5),
        ("east_approach", (width_m - 2.5).max(2.0), height_m * 0.5),
        ("north_flank", width_m * 0.5, 2.5),
        ("south_flank", width_m * 0.5, (height_m - 2.5).max(2.0)),
    ];
    let mut walkable_anchors = Vec::new();
    for (label, ax, ay) in candidate_anchors {
        if let Some(anchor) = find_clear_anchor_near(map, ax, ay, map.active_floor, label) {
            walkable_anchors.push(anchor);
        }
    }

    let zone_names: Vec<String> = zones
        .iter()
        .take(5)
        .map(|z| format!("{} @{:.1},{:.1}m", z.label, z.center_m.0, z.center_m.1))
        .collect();
    let floor_names: Vec<String> = floors.iter().map(|f| f.label.clone()).collect();
    let exit_names: Vec<String> = cardinal_exits
        .iter()
        .map(|e| format!("{}: {}", e.direction, e.preview_label))
        .collect();
    let preset = map
        .theme_preset
        .clone()
        .unwrap_or_else(|| "procedural_sector".to_string());
    let concise_brief = format!(
        "{} ({preset}, {:.0}m x {:.0}m, active_floor={}, floors=[{}]) | Zones: [{}] | Exits: [{}]",
        map.title,
        width_m,
        height_m,
        map.active_floor,
        floor_names.join(", "),
        if zone_names.is_empty() {
            "Open Floorplan".to_string()
        } else {
            zone_names.join("; ")
        },
        exit_names.join(" | "),
    );

    SectorNarrativeManifest {
        map_id: map.map_id.clone(),
        title: map.title.clone(),
        preset,
        width_m,
        height_m,
        active_floor: map.active_floor,
        floors,
        zones,
        cardinal_exits,
        walkable_anchors,
        concise_brief,
    }
}

// ============================================================================
// §4 Passive Movement Perception Evaluation (`SIG` / `HEA` / `OLF` / `ITU`)
// ============================================================================

fn observer_passive_score(
    sheet: Option<&CharacterSheetRecord>,
    channel: SpatialSensoryChannel,
) -> i32 {
    let Some(rec) = sheet else {
        return 10;
    };
    let base_stat = match channel {
        SpatialSensoryChannel::Sight => rec.characteristics.sig,
        SpatialSensoryChannel::Hearing => rec.characteristics.hea,
        SpatialSensoryChannel::Olfaction => rec.characteristics.olf,
        SpatialSensoryChannel::Intuition => rec.characteristics.itu,
    };
    let alertness = rec
        .skills
        .get("alertness_situational_awareness")
        .copied()
        .unwrap_or(0);
    let empathy = if channel == SpatialSensoryChannel::Intuition {
        rec.skills
            .get("empathy_social_perception")
            .copied()
            .unwrap_or(0)
    } else {
        0
    };
    base_stat + alertness.max(empathy).clamp(0, 5)
}

fn evaluate_single_pin(
    scene: &TacticalSceneView<'_>,
    obs_id: &str,
    obs_name: &str,
    obs_pos_m: (f32, f32),
    sheet: Option<&CharacterSheetRecord>,
    pin: &SpatialSensoryPin,
) -> Option<PassivePerceptionWhisper> {
    let los = evaluate_los_continuous(scene, obs_pos_m, (pin.x_m, pin.y_m));
    if los.distance_m > pin.max_range_m {
        return None;
    }
    if pin.channel == SpatialSensoryChannel::Sight && !los.has_los {
        return None;
    }
    let dist_penalty = ((los.distance_m - 4.0).max(0.0) / 4.0).floor() as i32;
    let obscurement_penalty = if pin.channel == SpatialSensoryChannel::Sight {
        match los.cover_grade {
            CoverGrade::Concealment | CoverGrade::LowCover => 2,
            CoverGrade::HighCover => 4,
            CoverGrade::Blocked => return None,
            CoverGrade::Clear => 0,
        }
    } else if !los.has_los {
        3
    } else {
        0
    };
    let effective_tn = pin.difficulty_tn + dist_penalty + obscurement_penalty;
    let passive_score = observer_passive_score(sheet, pin.channel);
    if passive_score < effective_tn {
        return None;
    }
    Some(PassivePerceptionWhisper {
        pin_id: pin.pin_id.clone(),
        observer_id: obs_id.to_string(),
        observer_name: obs_name.to_string(),
        channel: pin.channel,
        characteristic_code: pin.channel.characteristic_code().to_string(),
        distance_m: los.distance_m,
        has_los: los.has_los,
        passive_score,
        effective_tn,
        margin: passive_score - effective_tn,
        whisper_text: pin.whisper_text.clone(),
    })
}

fn synthesize_environment_pins(
    map: &GmMapUpdateRequest,
    exclude_entity: &str,
) -> Vec<SpatialSensoryPin> {
    let mut pins = Vec::new();
    for (idx, prop) in map.props.iter().enumerate() {
        if prop.kind == PropKind::HazardSpill {
            let fp = prop.footprint_m();
            pins.push(SpatialSensoryPin {
                pin_id: format!("env_hazard_{idx}"),
                x_m: fp.cx_m,
                y_m: fp.cy_m,
                floor_index: map.active_floor,
                channel: SpatialSensoryChannel::Olfaction,
                difficulty_tn: 10,
                max_range_m: 9.0,
                whisper_text:
                    "Sharp chemical / plasma residue hangs in the air near a nearby hazard spill."
                        .to_string(),
                revealed_to: Vec::new(),
            });
        }
    }
    for tok in &map.tokens {
        if tok.entity_id != exclude_entity && tok.visibility == MessageVisibility::GmOnly {
            let (tx, ty) = map.token_pos_m(tok);
            pins.push(SpatialSensoryPin {
                pin_id: format!("hidden_tok_{}", tok.entity_id),
                x_m: tx,
                y_m: ty,
                floor_index: map.token_floor(&tok.entity_id),
                channel: SpatialSensoryChannel::Hearing,
                difficulty_tn: 13,
                max_range_m: 7.5,
                whisper_text: format!(
                    "Faint shifting weight and gear rustle near ({tx:.1}m, {ty:.1}m) betray an unseen presence ({}).",
                    tok.label
                ),
                revealed_to: Vec::new(),
            });
        }
    }
    pins
}

/// Evaluates passive perception whispers triggered when `moved_entity_id` steps to its new position.
#[must_use]
pub fn evaluate_passive_movement_perceptions(
    map: &GmMapUpdateRequest,
    moved_entity_id: &str,
    observers: &[CharacterSheetRecord],
    custom_pins: &[SpatialSensoryPin],
) -> Vec<PassivePerceptionWhisper> {
    let Some(moved_tok) = map.tokens.iter().find(|t| t.entity_id == moved_entity_id) else {
        return Vec::new();
    };
    let moved_floor = map.token_floor(&moved_tok.entity_id);
    if moved_floor != map.active_floor {
        return Vec::new();
    }
    let scene = TacticalSceneView::from_map(map);
    let moved_pos = map.token_pos_m(moved_tok);
    let sheet_by_id: BTreeMap<&str, &CharacterSheetRecord> = observers
        .iter()
        .map(|o| (o.character_id.as_str(), o))
        .collect();

    let mut whispers = Vec::new();
    let mut seen_keys: BTreeSet<(String, String)> = BTreeSet::new();

    // 1. Evaluate moved token's passive senses against custom pins + environmental hazards + GmOnly threats.
    let env_pins = synthesize_environment_pins(map, moved_entity_id);
    let moved_sheet = sheet_by_id.get(moved_entity_id).copied();
    for pin in custom_pins.iter().chain(env_pins.iter()) {
        if pin.floor_index != moved_floor {
            continue;
        }
        if pin.revealed_to.iter().any(|id| id == moved_entity_id) {
            continue;
        }
        if let Some(w) = evaluate_single_pin(
            &scene,
            moved_entity_id,
            &moved_tok.label,
            moved_pos,
            moved_sheet,
            pin,
        ) && seen_keys.insert((w.observer_id.clone(), w.pin_id.clone()))
        {
            whispers.push(w);
        }
    }

    // 2. If the moved token itself is GmOnly, check whether stationary public observers detect it.
    if moved_tok.visibility == MessageVisibility::GmOnly {
        let threat_pin = SpatialSensoryPin {
            pin_id: format!("moving_threat_{}", moved_tok.entity_id),
            x_m: moved_pos.0,
            y_m: moved_pos.1,
            floor_index: moved_floor,
            channel: SpatialSensoryChannel::Sight,
            difficulty_tn: 12,
            max_range_m: 14.0,
            whisper_text: format!(
                "Movement catches your eye at ({:.1}m, {:.1}m): {} is repositioning nearby.",
                moved_pos.0, moved_pos.1, moved_tok.label
            ),
            revealed_to: Vec::new(),
        };
        for tok in &map.tokens {
            if tok.entity_id == moved_entity_id || tok.visibility == MessageVisibility::GmOnly {
                continue;
            }
            if map.token_floor(&tok.entity_id) != moved_floor {
                continue;
            }
            let obs_sheet = sheet_by_id.get(tok.entity_id.as_str()).copied();
            if let Some(w) = evaluate_single_pin(
                &scene,
                &tok.entity_id,
                &tok.label,
                map.token_pos_m(tok),
                obs_sheet,
                &threat_pin,
            ) && seen_keys.insert((w.observer_id.clone(), w.pin_id.clone()))
            {
                whispers.push(w);
            }
        }
    }

    whispers.sort_by(|a, b| {
        b.margin
            .cmp(&a.margin)
            .then_with(|| a.pin_id.cmp(&b.pin_id))
    });
    whispers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tactical_map::{SectorPreset, build_procedural_map_request_with_options};

    #[test]
    fn test_extract_sector_narrative_manifest_hospital_has_floors_zones_exits_and_anchors() {
        let map = build_procedural_map_request_with_options(
            "map-hosp",
            SectorPreset::Hospital,
            Some("St. Jude Regional Medical"),
            36,
            26,
            77,
            None,
            None,
        );
        let manifest = extract_sector_narrative_manifest(&map);
        assert_eq!(manifest.cardinal_exits.len(), 4);
        assert!(!manifest.floors.is_empty());
        assert!(!manifest.zones.is_empty());
        assert!(!manifest.walkable_anchors.is_empty());
        assert!(manifest.concise_brief.contains("St. Jude Regional Medical"));
    }

    #[test]
    fn test_passive_movement_perception_triggers_for_observant_pc_near_pin() {
        let mut map = build_procedural_map_request_with_options(
            "map-pin",
            SectorPreset::MedLabSector,
            None,
            24,
            18,
            15,
            None,
            None,
        );
        let manifest = extract_sector_narrative_manifest(&map);
        let anchor = &manifest.walkable_anchors[0];
        let tok_id = map.tokens[0].entity_id.clone();
        map.set_token_pos_cm(&tok_id, anchor.x_cm, anchor.y_cm);

        let mut pc = CharacterSheetRecord {
            character_id: tok_id.clone(),
            character_name: "Sara".to_string(),
            ..CharacterSheetRecord::default()
        };
        pc.characteristics.sig = 15;
        pc.skills
            .insert("alertness_situational_awareness".to_string(), 3);

        let pin = SpatialSensoryPin {
            pin_id: "pin-blood-swab".to_string(),
            x_m: anchor.x_m + 0.4,
            y_m: anchor.y_m,
            floor_index: map.active_floor,
            channel: SpatialSensoryChannel::Sight,
            difficulty_tn: 13,
            max_range_m: 8.0,
            whisper_text: "Micro-hairline fracture and wiped reagent swab under the trolley."
                .to_string(),
            revealed_to: Vec::new(),
        };

        let whispers = evaluate_passive_movement_perceptions(&map, &tok_id, &[pc], &[pin]);
        assert!(
            whispers
                .iter()
                .any(|w| w.pin_id == "pin-blood-swab" && w.characteristic_code == "SIG")
        );
    }

    #[test]
    fn test_walkable_anchors_avoid_pool_water_and_label_brick_paver_walk() {
        use satya_schema::{MapPointCm, MapSurfaceFeature, SurfaceFeatureKind, SurfaceShapeKind};

        let mut map = build_procedural_map_request_with_options(
            "map-pool-walk",
            SectorPreset::Warehouse,
            None,
            16,
            14,
            21,
            None,
            None,
        );
        map.wall_edges.clear();
        map.props.clear();
        map.surfaces = vec![
            MapSurfaceFeature {
                kind: SurfaceFeatureKind::PoolWater,
                shape: SurfaceShapeKind::Polygon,
                points_cm: vec![
                    MapPointCm::new(400, 400),
                    MapPointCm::new(1100, 400),
                    MapPointCm::new(1100, 1000),
                    MapPointCm::new(400, 1000),
                ],
                width_cm: 0,
                border_style_cm: 0,
                variant_seed: 1,
                tint_rgb: None,
            },
            MapSurfaceFeature {
                kind: SurfaceFeatureKind::BrickPaverWalk,
                shape: SurfaceShapeKind::Polygon,
                points_cm: vec![
                    MapPointCm::new(100, 100),
                    MapPointCm::new(350, 100),
                    MapPointCm::new(350, 350),
                    MapPointCm::new(100, 350),
                ],
                width_cm: 0,
                border_style_cm: 0,
                variant_seed: 2,
                tint_rgb: None,
            },
        ];

        assert_eq!(
            surface_semantic_label(SurfaceFeatureKind::BrickPaverWalk),
            "Pedestrian Plaza / Walk"
        );
        let center_anchor = find_clear_anchor_near(&map, 8.0, 7.0, 0, "poolside")
            .expect("should find clear dry anchor outside PoolWater");
        let top = crate::map_surface_geom::topmost_surface_at_m(
            &map.surfaces,
            center_anchor.x_m,
            center_anchor.y_m,
        );
        assert_ne!(
            top.map(|s| s.kind),
            Some(SurfaceFeatureKind::PoolWater),
            "anchor landed inside PoolWater at ({}, {})",
            center_anchor.x_m,
            center_anchor.y_m
        );
    }
}
