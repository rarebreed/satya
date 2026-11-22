//! PyO3 bindings for Satya Ch. 7, 11, 18, 20–24 (`satya_py.tech`):
//! multi-spectrum stealth, surveillance tail craft & forensic trace decay,
//! contacts/cover/safehouses, blast/breaching, vehicle chases & component trauma,
//! environmental hazards, custom firearm/melee/armor engineering, modular drones/chassis trauma/cyber-prosthetics,
//! and 2035 electronic warfare (`SURVEILLANCE` -> `ECM` vs. `ECCM` -> custom EW hardware).

use pyo3::prelude::*;
use satya_engine::action_catalog_stealth::{
    AcousticSurfaceKind, ActiveSensoryChannel, AmbientNoiseMask, DigitalRfHygiene,
    ForensicDecayContext, IlluminationCondition, MultiSpectrumStealthParams, ObservationMode,
    ObserverAttentionState, SocialStealthWardrobeFit, StealthGait, TailStance,
    ThermalStealthProfile, VisualCountermeasure, evaluate_multi_spectrum_stealth_modifiers,
    evaluate_surveillance_tail_exchange,
};
use satya_engine::action_economy::EncumbranceTier;
use satya_engine::ballistics::{
    AmmoCaliberId, BarrelMountingType, CenterOfGravityProfile, FirearmActionType, MuzzleDeviceType,
    WeaponFeedType,
};
use satya_engine::characteristics::CharacterCharacteristics;
use satya_engine::contested_cascade::ContestedTieRule;
use satya_engine::economy_gear::{
    ContactProfile, CoverIdentityTier, CoverScrutinyLevel, FavorSeverityTier, SafehouseTier,
    evaluate_contact_favor_request, evaluate_safehouse_compromise_check,
};
use satya_engine::electronic_warfare::{
    CustomEwHardwareBlueprint, EccmHardeningId, EcmMethodId, EwHardwareRole,
    NetworkIntrusionPhase, SurveillanceNodeKind, evaluate_custom_ew_hardware_design,
    evaluate_ecm_vs_eccm_contest, evaluate_network_or_sensor_intrusion,
};
use satya_engine::engineering_design::{
    ArmorCoverageTier, ArmorMaterialArchitecture, BuildOutcomeQuality, BulletConstructionKind,
    CustomFirearmBlueprint, MeleeChassisClass, MeleeErgonomicTuning, MeleeMaterialGrade,
    ProjectClockComplexity, WorkshopFacilityTier, build_custom_firearm_design,
    evaluate_custom_armor_design, evaluate_custom_melee_design,
};
use satya_engine::environment_vehicles::{
    ChaseRangeBand, VehicleComponentZone, VehicleId, evaluate_vehicle_chase_exchange,
};
use satya_engine::environmental_hazards::{
    ElectricalHazardTier, HydrationEnvironment, LandingSurfaceClass, SepsisStage,
    SurfaceTractionTier, WindIntensityTier, WoundContaminationLevel, evaluate_acute_radiation,
    evaluate_dehydration, evaluate_electrical_shock, evaluate_fall_impact,
    evaluate_weather_and_footing, evaluate_wound_sepsis,
};
use satya_engine::explosives_and_arsenal::ExplosiveOrdnanceId;
use satya_engine::primitives::Triad;
use satya_engine::robotics_and_ai_design::{
    AiArchitectureParadigm, ChassisHitZone, CyberAugmentId, DroneChassisClass, DroneModuleId,
    evaluate_human_ai_codesign_session,
};
use satya_schema::MaterialClass;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::py_bridge::{
    PyEnum, PyJsonDict, PyJsonInput, PySerdeEnum, merge_object_defaults, normalize_code,
    parse_coded_enum, parse_serde_enum_ci, py_to_json_value, to_py_err, to_py_object,
    unwrap_py_serde_enum_or,
};

const fn def_usize<const N: usize>() -> usize {
    N
}
fn d_hardwood() -> String {
    "hardwood_asphalt_concrete".to_string()
}
fn d_temperate() -> String {
    "temperate_low_activity".to_string()
}
fn d_taser() -> String {
    "taser_cew_pulsed".to_string()
}
fn d_calm() -> String {
    "calm_under_15_kmh".to_string()
}
fn d_dry() -> String {
    "dry_solid".to_string()
}
fn d_stage0() -> String {
    "stage0_clean".to_string()
}
fn d_clean() -> String {
    "clean_irrigated".to_string()
}
fn d_pair10() -> (i32, i32) {
    (10, 10)
}
fn d_pairs10() -> Vec<(i32, i32)> {
    vec![(10, 10)]
}
fn d_f64_3() -> f64 {
    3.0
}
fn d_f64_12() -> f64 {
    12.0
}
fn d_f64_20() -> f64 {
    20.0
}
fn d_f64_25() -> f64 {
    25.0
}
fn d_f64_100() -> f64 {
    100.0
}
fn d_f64_085() -> f64 {
    0.85
}
fn d_f64_220() -> f64 {
    220.0
}
fn d_i32_3() -> i32 {
    3
}
fn d_i32_10() -> i32 {
    10
}
fn d_i32_15() -> i32 {
    15
}
fn d_true() -> bool {
    true
}
fn d_knife() -> String {
    "KNIFE".to_string()
}
fn d_spring() -> String {
    "SPRING_CARBON".to_string()
}
fn d_bal() -> String {
    "BALANCED".to_string()
}
fn d_garage() -> String {
    "HOME_GARAGE".to_string()
}
fn d_cnc() -> String {
    "COMMERCIAL_CNC".to_string()
}
fn d_clean_build() -> String {
    "clean_standard".to_string()
}
fn d_aramid() -> String {
    "PARA_ARAMID".to_string()
}
fn d_carrier() -> String {
    "TACTICAL_CARRIER".to_string()
}
fn d_9mm() -> String {
    "parabellum9x19".to_string()
}
fn d_fmj() -> String {
    "FMJ_BALL".to_string()
}
fn d_striker() -> String {
    "striker_fired".to_string()
}
fn d_boxmag() -> String {
    "detachable_box_magazine".to_string()
}
fn d_pmount() -> String {
    "standard_pistol_mount".to_string()
}
fn d_crown() -> String {
    "bare_crown".to_string()
}
fn d_cog() -> String {
    "chamber_neutral".to_string()
}
fn d_subassy() -> String {
    "CUSTOM_SUBASSEMBLY".to_string()
}
fn d_sniffer() -> String {
    "PASSIVE_DF_SNIFFER".to_string()
}

fn opt_serde<T: serde::de::DeserializeOwned>(
    val: Option<&str>,
    label: &str,
    default: T,
) -> PyResult<T> {
    match val {
        Some(s) => parse_serde_enum_ci(s, label),
        None => Ok(default),
    }
}

fn inject_kind_tag(
    obj_opt: Option<PyJsonInput<'_>>,
    tag: &str,
    context: &str,
) -> PyResult<Value> {
    let mut val = match obj_opt {
        Some(bound) => {
            let v = py_to_json_value(&bound, context)?;
            if v.is_null() {
                Value::Object(Map::new())
            } else {
                v
            }
        }
        None => Value::Object(Map::new()),
    };
    let map = val
        .as_object_mut()
        .ok_or_else(|| to_py_err(format!("{context} payload must be a mapping or JSON object")))?;
    map.insert("kind".to_string(), Value::String(tag.to_string()));
    Ok(val)
}

#[derive(Debug, Deserialize)]
struct PyMultiSpectrumStealthInput {
    #[serde(default)]
    infiltrator: Option<Value>,
    #[serde(default)]
    infiltrator_gait: Option<String>,
    #[serde(default)]
    infiltrator_encumbrance: Option<String>,
    #[serde(default)]
    infiltrator_camouflage_bonus_delta_ext: i32,
    #[serde(default)]
    acoustic_surface: Option<String>,
    #[serde(default)]
    ambient_noise: Option<String>,
    #[serde(default)]
    thermal_profile: Option<String>,
    #[serde(default)]
    rf_hygiene: Option<String>,
    #[serde(default)]
    social_wardrobe_fit: Option<String>,
    #[serde(default)]
    cover_identity_pr_bonus: i32,
    #[serde(default)]
    observer: Option<Value>,
    #[serde(default)]
    observation_mode: Option<String>,
    #[serde(default)]
    observer_attention: Option<String>,
    #[serde(default = "def_usize::<2>")]
    observer_skill_n: usize,
    #[serde(default)]
    watch_hours_elapsed: f64,
    #[serde(default)]
    observer_channel: Option<String>,
    #[serde(default)]
    illumination: Option<String>,
    #[serde(default)]
    countermeasure: Option<String>,
    #[serde(default)]
    sensor_sweep_tool_delta_mod: i32,
}

/// Serde internally tagged union replacing procedural hazard_kind branches.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
enum PyHazardRequest {
    FallImpact {
        #[serde(default = "d_f64_3")]
        height_m: f64,
        #[serde(default = "d_hardwood")]
        surface: String,
        #[serde(default)]
        breakfall_net_successes: i32,
        #[serde(default)]
        clean_water_dive: bool,
    },
    Dehydration {
        #[serde(default = "d_temperate")]
        env: String,
        #[serde(default = "d_f64_3")]
        water_consumed_liters: f64,
        #[serde(default)]
        extra_sweat_deficit_liters: f64,
    },
    Radiation {
        #[serde(default)]
        cumulative_sieverts_sv: f64,
    },
    ElectricalShock {
        #[serde(default = "d_taser")]
        tier: String,
        #[serde(default)]
        wet_or_submerged: bool,
        #[serde(default)]
        insulated_ppe: bool,
        #[serde(default = "d_i32_10")]
        hlt: i32,
        #[serde(default = "d_i32_10")]
        dis: i32,
        #[serde(default = "d_pairs10")]
        player_pairs: Vec<(i32, i32)>,
        #[serde(default = "d_pair10")]
        gm_pair: (i32, i32),
    },
    WeatherFooting {
        #[serde(default = "d_calm")]
        wind: String,
        #[serde(default = "d_dry")]
        traction: String,
        #[serde(default = "d_f64_100")]
        range_m: f64,
        #[serde(default)]
        has_crampons: bool,
    },
    WoundSepsis {
        #[serde(default = "d_stage0")]
        current_stage: String,
        #[serde(default = "d_clean")]
        contamination: String,
        #[serde(default)]
        antibiotics_active: bool,
        #[serde(default = "d_i32_10")]
        hlt: i32,
        #[serde(default = "d_pairs10")]
        player_pairs: Vec<(i32, i32)>,
        #[serde(default = "d_pair10")]
        gm_pair: (i32, i32),
    },
}

/// Serde internally tagged union replacing procedural design_kind branches.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
enum PyCustomGearRequest {
    Melee {
        #[serde(default = "d_knife")]
        chassis: String,
        #[serde(default = "d_spring")]
        material: String,
        #[serde(default = "d_bal")]
        tuning: String,
        #[serde(default = "d_garage")]
        workshop: String,
        #[serde(default = "d_clean_build")]
        outcome: String,
    },
    Armor {
        #[serde(default = "d_aramid")]
        material: String,
        #[serde(default = "d_carrier")]
        coverage: String,
        #[serde(default = "d_cnc")]
        workshop: String,
        #[serde(default = "d_clean_build")]
        outcome: String,
    },
    Firearm {
        #[serde(default = "d_9mm")]
        caliber: String,
        #[serde(default = "d_fmj")]
        bullet: String,
        #[serde(default)]
        handload_quality_q: f64,
        #[serde(default = "d_striker")]
        action_type: String,
        #[serde(default = "d_boxmag")]
        feed_type: String,
        #[serde(default = "d_i32_15")]
        capacity_rounds: i32,
        #[serde(default = "d_f64_12")]
        barrel_length_cm: f64,
        #[serde(default = "d_f64_20")]
        overall_length_cm: f64,
        #[serde(default = "d_f64_085")]
        unloaded_weight_kg: f64,
        #[serde(default = "d_pmount")]
        barrel_mounting: String,
        #[serde(default = "d_crown")]
        muzzle_device: String,
        #[serde(default = "d_cog")]
        center_of_gravity: String,
        #[serde(default)]
        requested_tolerance_tau: f64,
        #[serde(default)]
        requested_barrel_quality_q: f64,
        #[serde(default)]
        requested_mfg_quality_q: i32,
        #[serde(default = "d_garage")]
        workshop: String,
        #[serde(default = "d_subassy")]
        complexity: String,
        #[serde(default = "d_clean_build")]
        build_outcome: String,
        #[serde(default = "d_f64_25")]
        eval_range_m: f64,
    },
}

#[derive(Debug, Deserialize)]
struct PyCustomEwInput {
    #[serde(default = "d_sniffer")]
    role: String,
    #[serde(default = "d_i32_3")]
    requested_ew_power: i32,
    #[serde(default = "d_true")]
    rf_stealth_baffled: bool,
    #[serde(default = "d_f64_220")]
    battery_wh: f64,
    #[serde(default = "d_cnc")]
    workshop: String,
    #[serde(default = "d_subassy")]
    complexity: String,
    #[serde(default = "d_clean_build")]
    build_outcome: String,
}

/// Evaluates Multi-Spectrum / Social Stealth and Passive Awareness vs. Active Observation (Ch. 7).
#[pyfunction]
#[pyo3(signature = (params=None))]
pub fn evaluate_multi_spectrum_stealth(
    py: Python<'_>,
    params: Option<PyJsonInput<'_>>,
) -> PyResult<PyJsonDict> {
    let raw_val = inject_kind_tag(params, "STEALTH", "stealth params")?;
    let raw: PyMultiSpectrumStealthInput = serde_json::from_value(raw_val)
        .map_err(|e| to_py_err(format!("Invalid stealth params: {e}")))?;

    let eval_params = MultiSpectrumStealthParams {
        infiltrator: merge_object_defaults::<CharacterCharacteristics>(
            raw.infiltrator.as_ref(),
            &[("for", "for_stat")],
            "characteristics",
        )?,
        infiltrator_gait: opt_serde(
            raw.infiltrator_gait.as_deref(),
            "infiltrator_gait",
            StealthGait::StealthCreep,
        )?,
        infiltrator_encumbrance: opt_serde(
            raw.infiltrator_encumbrance.as_deref(),
            "infiltrator_encumbrance",
            EncumbranceTier::Tier0Unencumbered,
        )?,
        infiltrator_camouflage_bonus_delta_ext: raw.infiltrator_camouflage_bonus_delta_ext,
        acoustic_surface: match raw.acoustic_surface.as_deref() {
            Some(s) => parse_coded_enum::<AcousticSurfaceKind>(s, "acoustic_surface")?,
            None => AcousticSurfaceKind::ConcreteAsphaltOrTile,
        },
        ambient_noise: opt_serde(
            raw.ambient_noise.as_deref(),
            "ambient_noise",
            AmbientNoiseMask::QuietSuburbanNight,
        )?,
        thermal_profile: opt_serde(
            raw.thermal_profile.as_deref(),
            "thermal_profile",
            ThermalStealthProfile::UnshieldedStandard,
        )?,
        rf_hygiene: opt_serde(
            raw.rf_hygiene.as_deref(),
            "rf_hygiene",
            DigitalRfHygiene::AirGappedFaradayPouch,
        )?,
        social_wardrobe_fit: match raw.social_wardrobe_fit.as_deref() {
            Some(s) if !s.trim().is_empty() => {
                Some(parse_serde_enum_ci::<SocialStealthWardrobeFit>(s, "social_wardrobe_fit")?)
            }
            _ => None,
        },
        cover_identity_pr_bonus: raw.cover_identity_pr_bonus,
        observer: merge_object_defaults::<CharacterCharacteristics>(
            raw.observer.as_ref(),
            &[("for", "for_stat")],
            "characteristics",
        )?,
        observation_mode: opt_serde(
            raw.observation_mode.as_deref(),
            "observation_mode",
            ObservationMode::PassiveAwareness,
        )?,
        observer_attention: match raw.observer_attention.as_deref() {
            Some(s) => parse_coded_enum::<ObserverAttentionState>(s, "observer_attention")?,
            None => ObserverAttentionState::RoutinePatrolBored,
        },
        observer_skill_n: raw.observer_skill_n,
        watch_hours_elapsed: raw.watch_hours_elapsed,
        observer_channel: opt_serde(
            raw.observer_channel.as_deref(),
            "observer_channel",
            ActiveSensoryChannel::VisualSigFoc,
        )?,
        illumination: opt_serde(
            raw.illumination.as_deref(),
            "illumination",
            IlluminationCondition::DimLightDuskShadows,
        )?,
        countermeasure: opt_serde(
            raw.countermeasure.as_deref(),
            "countermeasure",
            VisualCountermeasure::None,
        )?,
        sensor_sweep_tool_delta_mod: raw.sensor_sweep_tool_delta_mod,
    };

    to_py_object(py, &evaluate_multi_spectrum_stealth_modifiers(&eval_params))
}

/// Evaluates one urban tail / Surveillance Detection Route (`SDR`) leg (Ch. 7 §4.1).
#[pyfunction]
#[pyo3(signature = (tail_stance, shadower_skill_n, shadower_control_c, shadower_pairs, target_gm_pair, shadower_internal_mod=0, shadower_tool_delta_mod=0, target_sdr_delta_ext=0))]
pub fn evaluate_surveillance_tail(
    py: Python<'_>,
    tail_stance: PySerdeEnum<TailStance>,
    shadower_skill_n: usize,
    shadower_control_c: i32,
    shadower_pairs: Vec<(i32, i32)>,
    target_gm_pair: (i32, i32),
    shadower_internal_mod: i32,
    shadower_tool_delta_mod: i32,
    target_sdr_delta_ext: i32,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &evaluate_surveillance_tail_exchange(
            tail_stance.0,
            shadower_skill_n,
            shadower_control_c,
            shadower_internal_mod,
            shadower_tool_delta_mod,
            target_sdr_delta_ext,
            &shadower_pairs,
            target_gm_pair,
            ContestedTieRule::HybridSignedMargin,
        ),
    )
}

/// Evaluates trace decay and forensic / DFIR / deepfake audit sweeps (Ch. 7 §4.2–§4.3).
#[pyfunction]
#[pyo3(signature = (decay_context, hours_elapsed, investigator_skill_n, investigator_control_c, investigator_pairs, gm_pair, base_obscurity_delta_ext=0, internal_mod=0, tool_delta_mod=0))]
pub fn evaluate_forensic_clue_search(
    py: Python<'_>,
    decay_context: PySerdeEnum<ForensicDecayContext>,
    hours_elapsed: f64,
    investigator_skill_n: usize,
    investigator_control_c: i32,
    investigator_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
    base_obscurity_delta_ext: i32,
    internal_mod: i32,
    tool_delta_mod: i32,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &satya_engine::action_catalog_stealth::evaluate_forensic_clue_search(
            decay_context.0,
            hours_elapsed,
            base_obscurity_delta_ext,
            investigator_skill_n,
            investigator_control_c,
            internal_mod,
            tool_delta_mod,
            &investigator_pairs,
            gm_pair,
            ContestedTieRule::HybridSignedMargin,
        ),
    )
}

/// Evaluates a Campaign Contact favor request with reach/loyalty/exposure modifiers (Ch. 11 §3.3).
#[pyfunction]
#[pyo3(signature = (reach_access_tier, reliability_trust, exposure_burn_risk, favor_severity, required_reach_tier, jurisdiction_heat, requester_skill_n, requester_control_c, player_pairs, gm_pair, internal_mod=0))]
pub fn evaluate_contact_favor(
    py: Python<'_>,
    reach_access_tier: u8,
    reliability_trust: u8,
    exposure_burn_risk: u8,
    favor_severity: PySerdeEnum<FavorSeverityTier>,
    required_reach_tier: u8,
    jurisdiction_heat: u8,
    requester_skill_n: usize,
    requester_control_c: i32,
    player_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
    internal_mod: i32,
) -> PyResult<PyJsonDict> {
    let contact = ContactProfile {
        reach_access_tier,
        reliability_trust,
        exposure_burn_risk,
    };
    to_py_object(
        py,
        &evaluate_contact_favor_request(
            contact,
            favor_severity.0,
            required_reach_tier,
            jurisdiction_heat,
            requester_skill_n,
            requester_control_c,
            internal_mod,
            &player_pairs,
            gm_pair,
            ContestedTieRule::HybridSignedMargin,
        ),
    )
}

/// Evaluates layered Cover Identity checkpoint or biometric scrutiny (Ch. 11 §3.4).
#[pyfunction]
#[pyo3(signature = (cover_tier, scrutiny_level, cover_heat, holder_skill_n, holder_control_c, player_pairs, gm_pair, internal_mod=0))]
pub fn evaluate_cover_identity_scrutiny(
    py: Python<'_>,
    cover_tier: PyEnum<CoverIdentityTier>,
    scrutiny_level: PySerdeEnum<CoverScrutinyLevel>,
    cover_heat: u8,
    holder_skill_n: usize,
    holder_control_c: i32,
    player_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
    internal_mod: i32,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &satya_engine::economy_gear::evaluate_cover_identity_scrutiny(
            cover_tier.0,
            scrutiny_level.0,
            cover_heat,
            holder_skill_n,
            holder_control_c,
            internal_mod,
            &player_pairs,
            gm_pair,
            ContestedTieRule::HybridSignedMargin,
        ),
    )
}

/// Evaluates whether returning hot or tailed compromises a Safehouse, enforcing Faraday shielding (Ch. 11 §3.5).
#[pyfunction]
#[pyo3(signature = (safehouse_tier, returning_heat, actively_tailed, rf_hygiene_breached, operator_skill_n, operator_control_c, operator_pairs, gm_pair, internal_mod=0))]
pub fn evaluate_safehouse_compromise(
    py: Python<'_>,
    safehouse_tier: PyEnum<SafehouseTier>,
    returning_heat: u8,
    actively_tailed: bool,
    rf_hygiene_breached: bool,
    operator_skill_n: usize,
    operator_control_c: i32,
    operator_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
    internal_mod: i32,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &evaluate_safehouse_compromise_check(
            safehouse_tier.0,
            returning_heat,
            actively_tailed,
            rf_hygiene_breached,
            operator_skill_n,
            operator_control_c,
            internal_mod,
            &operator_pairs,
            gm_pair,
            ContestedTieRule::HybridSignedMargin,
        ),
    )
}

/// Evaluates explosive blast overpressure (`1.5x` indoor reflection), shrapnel hits, and flash/smoke effects (Ch. 18 §3.1).
#[pyfunction]
#[pyo3(signature = (ordnance_code, distance_m, enclosed_room=false, target_is_prone=false, intervening_barrier_pr=0))]
pub fn evaluate_blast_and_fragmentation(
    py: Python<'_>,
    ordnance_code: PyEnum<ExplosiveOrdnanceId>,
    distance_m: f64,
    enclosed_room: bool,
    target_is_prone: bool,
    intervening_barrier_pr: i32,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &satya_engine::explosives_and_arsenal::evaluate_blast_and_fragmentation(
            ordnance_code.0,
            distance_m,
            enclosed_room,
            target_is_prone,
            intervening_barrier_pr,
        ),
    )
}

/// Evaluates shaped-charge / thermite / C4 structural breaching against a barrier [`MaterialClass`] (Ch. 18 §3.2).
#[pyfunction]
#[pyo3(signature = (ordnance_code, target_material_code, demolitions_net_successes=0, friendly_standoff_m=6.0, enclosed_interior=true))]
pub fn evaluate_explosive_breach(
    py: Python<'_>,
    ordnance_code: PyEnum<ExplosiveOrdnanceId>,
    target_material_code: PyEnum<MaterialClass>,
    demolitions_net_successes: i32,
    friendly_standoff_m: f64,
    enclosed_interior: bool,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &satya_engine::explosives_and_arsenal::evaluate_explosive_breach(
            ordnance_code.0,
            target_material_code.0,
            demolitions_net_successes,
            friendly_standoff_m,
            enclosed_interior,
        ),
    )
}

/// Evaluates a high-speed Vehicular Chase exchange bounded by the canonical vehicle envelope (Ch. 20 §3.1–§3.2).
#[pyfunction]
#[pyo3(signature = (vehicle_code, pilot_triad, opponent_triad, current_band=None, wants_to_close=true, attempts_pit_or_ram=false))]
pub fn evaluate_vehicle_chase(
    py: Python<'_>,
    vehicle_code: PyEnum<VehicleId>,
    pilot_triad: (i32, i32, i32),
    opponent_triad: (i32, i32, i32),
    current_band: Option<PySerdeEnum<ChaseRangeBand>>,
    wants_to_close: bool,
    attempts_pit_or_ram: bool,
) -> PyResult<PyJsonDict> {
    let spec = vehicle_code.0.spec();
    let band = unwrap_py_serde_enum_or(current_band, ChaseRangeBand::Close15m);
    to_py_object(
        py,
        &evaluate_vehicle_chase_exchange(
            &spec.to_envelope_profile(),
            Triad::new(pilot_triad.0, pilot_triad.1, pilot_triad.2),
            Triad::new(opponent_triad.0, opponent_triad.1, opponent_triad.2),
            band,
            wants_to_close,
            attempts_pit_or_ram,
        ),
    )
}

/// Evaluates vehicle component trauma, tire blowouts, and EV battery thermal runaway (Ch. 20 §3.6).
#[pyfunction]
#[pyo3(signature = (vehicle_code, zone, uses_run_flat_tires=false, hits_ballistic_glass=false, effective_pen=4, effective_raw_impact=8))]
pub fn evaluate_vehicle_component_damage(
    py: Python<'_>,
    vehicle_code: PyEnum<VehicleId>,
    zone: PySerdeEnum<VehicleComponentZone>,
    uses_run_flat_tires: bool,
    hits_ballistic_glass: bool,
    effective_pen: i32,
    effective_raw_impact: i32,
) -> PyResult<PyJsonDict> {
    let spec = vehicle_code.0.spec();
    to_py_object(
        py,
        &satya_engine::environment_vehicles::evaluate_vehicle_component_damage(
            spec,
            zone.0,
            uses_run_flat_tires,
            hits_ballistic_glass,
            effective_pen,
            effective_raw_impact,
        ),
    )
}

/// Evaluates Ch. 21 environmental hazards (`fall_impact`, `dehydration`, `radiation`, `electrical_shock`, `weather_footing`, `wound_sepsis`).
#[pyfunction]
#[pyo3(signature = (hazard_kind, params=None))]
pub fn evaluate_environmental_hazard(
    py: Python<'_>,
    hazard_kind: &str,
    params: Option<PyJsonInput<'_>>,
) -> PyResult<PyJsonDict> {
    let canonical_tag = match normalize_code(hazard_kind).as_str() {
        "FALL" | "FALL_IMPACT" => "FALL_IMPACT",
        "DEHYDRATION" | "HYDRATION" => "DEHYDRATION",
        "RADIATION" | "ACUTE_RADIATION" => "RADIATION",
        "SHOCK" | "ELECTRICAL_SHOCK" | "ELECTRICITY" => "ELECTRICAL_SHOCK",
        "WEATHER" | "WEATHER_FOOTING" | "WIND" | "TRACTION" => "WEATHER_FOOTING",
        "SEPSIS" | "WOUND_SEPSIS" => "WOUND_SEPSIS",
        _ => {
            return Err(to_py_err(format!(
                "Unknown hazard_kind `{hazard_kind}`; expected fall_impact, dehydration, radiation, electrical_shock, weather_footing, or wound_sepsis"
            )));
        }
    };
    let tagged = inject_kind_tag(params, canonical_tag, "environmental hazard params")?;
    let req: PyHazardRequest = serde_json::from_value(tagged)
        .map_err(|e| to_py_err(format!("Invalid {canonical_tag} params: {e}")))?;

    match req {
        PyHazardRequest::FallImpact {
            height_m,
            surface,
            breakfall_net_successes,
            clean_water_dive,
        } => {
            let s = parse_serde_enum_ci::<LandingSurfaceClass>(&surface, "landing_surface")?;
            to_py_object(
                py,
                &evaluate_fall_impact(height_m, s, breakfall_net_successes, clean_water_dive),
            )
        }
        PyHazardRequest::Dehydration {
            env,
            water_consumed_liters,
            extra_sweat_deficit_liters,
        } => {
            let e = parse_serde_enum_ci::<HydrationEnvironment>(&env, "hydration_environment")?;
            to_py_object(
                py,
                &evaluate_dehydration(e, water_consumed_liters, extra_sweat_deficit_liters),
            )
        }
        PyHazardRequest::Radiation {
            cumulative_sieverts_sv,
        } => to_py_object(py, &evaluate_acute_radiation(cumulative_sieverts_sv)),
        PyHazardRequest::ElectricalShock {
            tier,
            wet_or_submerged,
            insulated_ppe,
            hlt,
            dis,
            player_pairs,
            gm_pair,
        } => {
            let t = parse_serde_enum_ci::<ElectricalHazardTier>(&tier, "electrical_hazard_tier")?;
            to_py_object(
                py,
                &evaluate_electrical_shock(
                    t,
                    wet_or_submerged,
                    insulated_ppe,
                    hlt,
                    dis,
                    &player_pairs,
                    gm_pair,
                    ContestedTieRule::HybridSignedMargin,
                ),
            )
        }
        PyHazardRequest::WeatherFooting {
            wind,
            traction,
            range_m,
            has_crampons,
        } => {
            let w = parse_serde_enum_ci::<WindIntensityTier>(&wind, "wind_intensity")?;
            let tr = parse_coded_enum::<SurfaceTractionTier>(&traction, "surface_traction")?;
            to_py_object(
                py,
                &evaluate_weather_and_footing(w, tr, range_m, has_crampons),
            )
        }
        PyHazardRequest::WoundSepsis {
            current_stage,
            contamination,
            antibiotics_active,
            hlt,
            player_pairs,
            gm_pair,
        } => {
            let st = parse_serde_enum_ci::<SepsisStage>(&current_stage, "sepsis_stage")?;
            let ct =
                parse_serde_enum_ci::<WoundContaminationLevel>(&contamination, "wound_contamination")?;
            to_py_object(
                py,
                &evaluate_wound_sepsis(
                    st,
                    ct,
                    antibiotics_active,
                    hlt,
                    &player_pairs,
                    gm_pair,
                    ContestedTieRule::HybridSignedMargin,
                ),
            )
        }
    }
}

/// Evaluates Ch. 22 custom firearm, melee weapon, or body armor engineering against workshop tolerances.
#[pyfunction]
#[pyo3(signature = (design_kind, spec=None))]
pub fn evaluate_custom_gear_design(
    py: Python<'_>,
    design_kind: &str,
    spec: Option<PyJsonInput<'_>>,
) -> PyResult<PyJsonDict> {
    let canonical_tag = match normalize_code(design_kind).as_str() {
        "MELEE" | "CUSTOM_MELEE" => "MELEE",
        "ARMOR" | "CUSTOM_ARMOR" => "ARMOR",
        "FIREARM" | "CUSTOM_FIREARM" | "GUN" => "FIREARM",
        _ => {
            return Err(to_py_err(format!(
                "Unknown design_kind `{design_kind}`; expected melee, armor, or firearm"
            )));
        }
    };
    let tagged = inject_kind_tag(spec, canonical_tag, "custom gear spec")?;
    let req: PyCustomGearRequest = serde_json::from_value(tagged)
        .map_err(|e| to_py_err(format!("Invalid custom {canonical_tag} spec: {e}")))?;

    match req {
        PyCustomGearRequest::Melee {
            chassis,
            material,
            tuning,
            workshop,
            outcome,
        } => to_py_object(
            py,
            &evaluate_custom_melee_design(
                parse_coded_enum::<MeleeChassisClass>(&chassis, "melee_chassis")?,
                parse_coded_enum::<MeleeMaterialGrade>(&material, "melee_material")?,
                parse_coded_enum::<MeleeErgonomicTuning>(&tuning, "melee_tuning")?,
                parse_coded_enum::<WorkshopFacilityTier>(&workshop, "workshop_tier")?,
                parse_serde_enum_ci::<BuildOutcomeQuality>(&outcome, "build_outcome")?,
            ),
        ),
        PyCustomGearRequest::Armor {
            material,
            coverage,
            workshop,
            outcome,
        } => to_py_object(
            py,
            &evaluate_custom_armor_design(
                parse_coded_enum::<ArmorMaterialArchitecture>(&material, "armor_material")?,
                parse_coded_enum::<ArmorCoverageTier>(&coverage, "armor_coverage")?,
                parse_coded_enum::<WorkshopFacilityTier>(&workshop, "workshop_tier")?,
                parse_serde_enum_ci::<BuildOutcomeQuality>(&outcome, "build_outcome")?,
            ),
        ),
        PyCustomGearRequest::Firearm {
            caliber,
            bullet,
            handload_quality_q,
            action_type,
            feed_type,
            capacity_rounds,
            barrel_length_cm,
            overall_length_cm,
            unloaded_weight_kg,
            barrel_mounting,
            muzzle_device,
            center_of_gravity,
            requested_tolerance_tau,
            requested_barrel_quality_q,
            requested_mfg_quality_q,
            workshop,
            complexity,
            build_outcome,
            eval_range_m,
        } => {
            let bp = CustomFirearmBlueprint {
                name: "Custom Build",
                caliber: parse_serde_enum_ci::<AmmoCaliberId>(&caliber, "caliber")?,
                bullet: parse_coded_enum::<BulletConstructionKind>(&bullet, "bullet")?,
                handload_quality_q,
                action_type: parse_serde_enum_ci::<FirearmActionType>(&action_type, "action_type")?,
                feed_type: parse_serde_enum_ci::<WeaponFeedType>(&feed_type, "feed_type")?,
                capacity_rounds,
                barrel_length_cm,
                overall_length_cm,
                unloaded_weight_kg,
                barrel_mounting: parse_serde_enum_ci::<BarrelMountingType>(
                    &barrel_mounting,
                    "barrel_mounting",
                )?,
                muzzle_device: parse_serde_enum_ci::<MuzzleDeviceType>(
                    &muzzle_device,
                    "muzzle_device",
                )?,
                center_of_gravity: parse_serde_enum_ci::<CenterOfGravityProfile>(
                    &center_of_gravity,
                    "center_of_gravity",
                )?,
                requested_tolerance_tau,
                requested_barrel_quality_q,
                requested_mfg_quality_q,
                workshop: parse_coded_enum::<WorkshopFacilityTier>(&workshop, "workshop_tier")?,
                complexity: parse_coded_enum::<ProjectClockComplexity>(&complexity, "complexity")?,
                build_outcome: parse_serde_enum_ci::<BuildOutcomeQuality>(
                    &build_outcome,
                    "build_outcome",
                )?,
            };
            to_py_object(py, &build_custom_firearm_design(&bp, eval_range_m))
        }
    }
}

/// Evaluates modular drone / UGV payload utilization, endurance minutes, armor, and cost (Ch. 23 §1).
#[pyfunction]
#[pyo3(signature = (chassis_code, module_codes=None, extra_armor_pr=0, autonomy_compute_w=0.0))]
pub fn evaluate_modular_drone_design(
    py: Python<'_>,
    chassis_code: PyEnum<DroneChassisClass>,
    module_codes: Option<Vec<String>>,
    extra_armor_pr: i32,
    autonomy_compute_w: f64,
) -> PyResult<PyJsonDict> {
    let raw_mods = module_codes.unwrap_or_default();
    let mut modules = Vec::with_capacity(raw_mods.len());
    for raw in raw_mods {
        modules.push(parse_coded_enum::<DroneModuleId>(&raw, "drone_module")?);
    }
    to_py_object(
        py,
        &satya_engine::robotics_and_ai_design::evaluate_modular_drone_design(
            chassis_code.0,
            &modules,
            extra_armor_pr,
            autonomy_compute_w,
        ),
    )
}

/// Resolves component damage, battery thermal fires, and EMP reboot ticks on a drone/UGV chassis (Ch. 23 §2).
#[pyfunction]
#[pyo3(signature = (current_si, chassis_pr, hit_zone, attack_pen, raw_impact, net_successes=1, is_emp_or_arc=false, emp_hardened=false))]
pub fn evaluate_robot_chassis_damage(
    py: Python<'_>,
    current_si: i32,
    chassis_pr: i32,
    hit_zone: PyEnum<ChassisHitZone>,
    attack_pen: i32,
    raw_impact: i32,
    net_successes: i32,
    is_emp_or_arc: bool,
    emp_hardened: bool,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &satya_engine::robotics_and_ai_design::evaluate_robot_chassis_damage(
            current_si,
            chassis_pr,
            hit_zone.0,
            attack_pen,
            raw_impact,
            net_successes,
            is_emp_or_arc,
            emp_hardened,
        ),
    )
}

/// Resolves Human–AI Co-Design engineering velocity vs. Verification Debt $D_{\text{verify}}$ (Ch. 23 §3.2).
#[pyfunction]
#[pyo3(signature = (paradigm_code, human_domain_n, ai_autonomy_leverage, human_control_c, human_pairs, gm_pair, internal_mod=0, base_task_delta_ext=0))]
pub fn evaluate_human_ai_codesign(
    py: Python<'_>,
    paradigm_code: PyEnum<AiArchitectureParadigm>,
    human_domain_n: i32,
    ai_autonomy_leverage: i32,
    human_control_c: i32,
    human_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
    internal_mod: i32,
    base_task_delta_ext: i32,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &evaluate_human_ai_codesign_session(
            paradigm_code.0,
            human_domain_n,
            ai_autonomy_leverage,
            human_control_c,
            internal_mod,
            base_task_delta_ext,
            &human_pairs,
            gm_pair,
            ContestedTieRule::HybridSignedMargin,
        ),
    )
}

/// Resolves a Formal Verification & Weight Audit check to scrub Verification Debt (Ch. 23 §3.3).
#[pyfunction]
#[pyo3(signature = (current_debt, paradigm_code, auditor_n, auditor_control_c, auditor_pairs, gm_pair, internal_mod=0))]
pub fn evaluate_verification_debt_audit(
    py: Python<'_>,
    current_debt: i32,
    paradigm_code: PyEnum<AiArchitectureParadigm>,
    auditor_n: i32,
    auditor_control_c: i32,
    auditor_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
    internal_mod: i32,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &satya_engine::robotics_and_ai_design::evaluate_verification_debt_audit(
            current_debt,
            paradigm_code.0,
            auditor_n,
            auditor_control_c,
            internal_mod,
            &auditor_pairs,
            gm_pair,
            ContestedTieRule::HybridSignedMargin,
        ),
    )
}

/// Evaluates clinical cyber-prosthetic / neural augmentation Triad gains, subdermal armor, and rejection (Ch. 23 §4).
#[pyfunction]
#[pyo3(signature = (augment_codes, surgical_quality_q=0, emp_hardened=false, immunosuppressant_compliant=true))]
pub fn evaluate_cyber_augmentation_loadout(
    py: Python<'_>,
    augment_codes: Vec<String>,
    surgical_quality_q: i32,
    emp_hardened: bool,
    immunosuppressant_compliant: bool,
) -> PyResult<PyJsonDict> {
    let mut augments = Vec::with_capacity(augment_codes.len());
    for raw in augment_codes {
        augments.push(parse_coded_enum::<CyberAugmentId>(&raw, "cyber_augment")?);
    }
    to_py_object(
        py,
        &satya_engine::robotics_and_ai_design::evaluate_cyber_augmentation_loadout(
            &augments,
            surgical_quality_q,
            emp_hardened,
            immunosuppressant_compliant,
        ),
    )
}

/// Evaluates a 2035 surveillance / SCADA intrusion step, enforcing hard physical-tap air-gap rules (Ch. 24 §1).
#[pyfunction]
#[pyo3(signature = (node_code, phase_code, is_physically_tapped, operator_control_c, operator_pairs, gm_pair, internal_mod=0, tool_mod=0))]
pub fn evaluate_network_intrusion(
    py: Python<'_>,
    node_code: PyEnum<SurveillanceNodeKind>,
    phase_code: PyEnum<NetworkIntrusionPhase>,
    is_physically_tapped: bool,
    operator_control_c: i32,
    operator_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
    internal_mod: i32,
    tool_mod: i32,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &evaluate_network_or_sensor_intrusion(
            node_code.0,
            phase_code.0,
            is_physically_tapped,
            operator_control_c,
            internal_mod,
            tool_mod,
            &operator_pairs,
            gm_pair,
            ContestedTieRule::HybridSignedMargin,
        ),
    )
}

/// Evaluates ECM jamming vs. ECCM link hardening, RF optical/fiber immunity, and DF triangulation (Ch. 24 §2–§3).
#[pyfunction]
#[pyo3(signature = (ecm_code, eccm_code, distance_m, active_duration_min, operator_control_c, operator_pairs, gm_pair, internal_mod=0))]
pub fn evaluate_ecm_vs_eccm(
    py: Python<'_>,
    ecm_code: PyEnum<EcmMethodId>,
    eccm_code: PyEnum<EccmHardeningId>,
    distance_m: f64,
    active_duration_min: f64,
    operator_control_c: i32,
    operator_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
    internal_mod: i32,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &evaluate_ecm_vs_eccm_contest(
            ecm_code.0,
            eccm_code.0,
            distance_m,
            active_duration_min,
            operator_control_c,
            internal_mod,
            &operator_pairs,
            gm_pair,
            ContestedTieRule::HybridSignedMargin,
        ),
    )
}

/// Evaluates custom ECM/ECCM/DF hardware blueprints against WorkshopFacilityTier precision ceilings (Ch. 24 §4).
#[pyfunction]
#[pyo3(signature = (spec=None))]
pub fn evaluate_custom_ew_hardware(
    py: Python<'_>,
    spec: Option<PyJsonInput<'_>>,
) -> PyResult<PyJsonDict> {
    let val = inject_kind_tag(spec, "EW", "custom EW hardware spec")?;
    let p: PyCustomEwInput = serde_json::from_value(val)
        .map_err(|e| to_py_err(format!("Invalid custom EW hardware spec: {e}")))?;
    let bp = CustomEwHardwareBlueprint {
        role: parse_coded_enum::<EwHardwareRole>(&p.role, "ew_hardware_role")?,
        requested_ew_power: p.requested_ew_power,
        rf_stealth_baffled: p.rf_stealth_baffled,
        battery_wh: p.battery_wh,
        workshop: parse_coded_enum::<WorkshopFacilityTier>(&p.workshop, "workshop_tier")?,
        complexity: parse_coded_enum::<ProjectClockComplexity>(&p.complexity, "complexity")?,
        build_outcome: parse_serde_enum_ci::<BuildOutcomeQuality>(&p.build_outcome, "build_outcome")?,
    };
    to_py_object(py, &evaluate_custom_ew_hardware_design(&bp))
}

/// Registers Ch. 7, 11, 18, and 20–24 technical/engineering/EW/hazard PyO3 functions on `satya_py._native`.
pub fn register_tech_and_gear_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(evaluate_multi_spectrum_stealth, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_surveillance_tail, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_forensic_clue_search, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_contact_favor, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_cover_identity_scrutiny, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_safehouse_compromise, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_blast_and_fragmentation, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_explosive_breach, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_vehicle_chase, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_vehicle_component_damage, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_environmental_hazard, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_custom_gear_design, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_modular_drone_design, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_robot_chassis_damage, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_human_ai_codesign, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_verification_debt_audit, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_cyber_augmentation_loadout, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_network_intrusion, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_ecm_vs_eccm, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_custom_ew_hardware, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::py_bridge::py_to_value;
    use pyo3::types::PyString;

    #[test]
    fn test_tech_and_gear_py_covers_chapters_7_11_18_and_20_to_24() {
        Python::initialize();
        Python::attach(|py| {
            let py_s = |s: &str| PyString::new(py, s).into_any();

            let stealth = evaluate_multi_spectrum_stealth(
                py,
                Some(py_s(
                    r#"{"infiltrator":{"kin":15,"itu":14},"infiltrator_gait":"STEALTH_CREEP","rf_hygiene":"ACTIVE_SMARTPHONE_OR_BLE_TRAIL","observer_attention":"HIGH_ALERT","observer_skill_n":4}"#,
                )),
            )
            .unwrap();
            let stealth_v = py_to_value(stealth.bind(py)).unwrap();
            assert_eq!(stealth_v["auto_pings_heat_3_plus"], true);
            assert_eq!(stealth_v["observer_effective_pairs_n"], 4);

            let tail = evaluate_surveillance_tail(
                py,
                PySerdeEnum(TailStance::TightTail),
                2,
                14,
                vec![(16, 15), (14, 14)],
                (5, 6),
                0,
                0,
                0,
            )
            .unwrap();
            assert_eq!(py_to_value(tail.bind(py)).unwrap()["tail_maintained"], true);

            let forensic = evaluate_forensic_clue_search(
                py,
                PySerdeEnum(ForensicDecayContext::ProfessionallyScrubbed),
                48.0,
                3,
                15,
                vec![(16, 14), (15, 13), (14, 14)],
                (4, 5),
                2,
                0,
                -2,
            )
            .unwrap();
            assert!(
                py_to_value(forensic.bind(py)).unwrap()["decay_delta_ext"]
                    .as_i64()
                    .unwrap()
                    >= 6
            );

            let contact = evaluate_contact_favor(
                py,
                2,
                3,
                1,
                PySerdeEnum(FavorSeverityTier::Moderate),
                2,
                2,
                2,
                14,
                vec![(15, 14), (16, 15)],
                (7, 8),
                0,
            )
            .unwrap();
            assert!(py_to_value(contact.bind(py)).unwrap()["favor_granted"].is_boolean());

            let cover = evaluate_cover_identity_scrutiny(
                py,
                PyEnum(CoverIdentityTier::Tier2DatabaseSeededAlias),
                PySerdeEnum(CoverScrutinyLevel::BiometricAuditOrFederalCrossCheck),
                6,
                2,
                11,
                vec![(4, 5), (3, 4)],
                (18, 17),
                0,
            )
            .unwrap();
            assert_eq!(py_to_value(cover.bind(py)).unwrap()["cover_burned"], true);

            let safe = evaluate_safehouse_compromise(
                py,
                PyEnum(SafehouseTier::HardenedFaradayWorkshop),
                5,
                false,
                true,
                2,
                14,
                vec![(16, 15), (15, 15)],
                (6, 7),
                0,
            )
            .unwrap();
            assert_eq!(
                py_to_value(safe.bind(py)).unwrap()["faraday_blocked_rf_leak"],
                true
            );

            let blast = evaluate_blast_and_fragmentation(
                py,
                PyEnum(ExplosiveOrdnanceId::M67FragmentationGrenade),
                2.0,
                true,
                false,
                0,
            )
            .unwrap();
            assert_eq!(
                py_to_value(blast.bind(py)).unwrap()["enclosed_room_reflected"],
                true
            );
            let breach = evaluate_explosive_breach(
                py,
                PyEnum(ExplosiveOrdnanceId::LinearShapedBreachingCharge),
                PyEnum(MaterialClass::ConcreteBlock),
                2,
                4.0,
                true,
            )
            .unwrap();
            assert!(
                py_to_value(breach.bind(py)).unwrap()["effective_breach_pr"]
                    .as_i64()
                    .unwrap()
                    > 0
            );

            let chase = evaluate_vehicle_chase(
                py,
                PyEnum(VehicleId::CompactUrbanEv),
                (18, 16, 10),
                (10, 10, 10),
                Some(PySerdeEnum(ChaseRangeBand::Medium50m)),
                true,
                false,
            )
            .unwrap();
            assert!(py_to_value(chase.bind(py)).unwrap()["next_band"].is_string());

            let v_dmg = evaluate_vehicle_component_damage(
                py,
                PyEnum(VehicleId::CompactUrbanEv),
                PySerdeEnum(VehicleComponentZone::FuelOrEvBatteryPack),
                false,
                false,
                7,
                12,
            )
            .unwrap();
            assert_eq!(
                py_to_value(v_dmg.bind(py)).unwrap()["triggers_ev_thermal_runaway"],
                true
            );

            for (kind, body) in [
                ("fall_impact", r#"{"height_m":10.0,"surface":"hardwood_asphalt_concrete"}"#),
                ("dehydration", r#"{"env":"hot_arid_desert_or_jungle_tactical","water_consumed_liters":2.0}"#),
                ("radiation", r#"{"cumulative_sieverts_sv":2.5}"#),
                ("electrical_shock", r#"{"tier":"industrial_three_phase480_v_to5k_v","wet_or_submerged":true}"#),
                ("weather_footing", r#"{"wind":"gale36_to65_kmh","traction":"black_ice","range_m":400.0}"#),
                ("wound_sepsis", r#"{"current_stage":"stage1_localized_infection","contamination":"dirty_field_or_second_degree_burn","antibiotics_active":true}"#),
            ] {
                let v = evaluate_environmental_hazard(py, kind, Some(py_s(body))).unwrap();
                assert!(py_to_value(v.bind(py)).unwrap().is_object());
            }

            for (kind, body) in [
                ("melee", r#"{"chassis":"AXE_BREACHER","material":"TOOL_STEEL_S7","tuning":"HEAVY_IMPACT","workshop":"COMMERCIAL_CNC","outcome":"masterwork_extraordinary"}"#),
                ("armor", r#"{"material":"BORON_CARBIDE","coverage":"TACTICAL_CARRIER","workshop":"FIELD_BENCH","outcome":"rushed_or_breached"}"#),
                ("firearm", r#"{"caliber":"nato556x45","bullet":"ARMOR_PIERCING_AP","workshop":"COMMERCIAL_CNC","muzzle_device":"sound_suppressor"}"#),
            ] {
                let v = evaluate_custom_gear_design(py, kind, Some(py_s(body))).unwrap();
                assert!(py_to_value(v.bind(py)).unwrap().is_object());
            }

            let drone = evaluate_modular_drone_design(
                py,
                PyEnum(DroneChassisClass::TacticalQuadrotorFpv),
                Some(vec![
                    "EO_FLIR_GIMBAL".to_string(),
                    "SOLID_STATE_BATTERY".to_string(),
                ]),
                1,
                12.0,
            )
            .unwrap();
            assert!(
                py_to_value(drone.bind(py)).unwrap()["endurance_min"]
                    .as_f64()
                    .unwrap()
                    > 10.0
            );

            let chassis_dmg = evaluate_robot_chassis_damage(
                py,
                6,
                2,
                PyEnum(ChassisHitZone::BatteryPackOrPowerbus),
                4,
                8,
                2,
                true,
                false,
            )
            .unwrap();
            assert_eq!(py_to_value(chassis_dmg.bind(py)).unwrap()["emp_reboot_ticks"], 40);

            let codesign = evaluate_human_ai_codesign(
                py,
                PyEnum(AiArchitectureParadigm::SparseMoETransformer),
                2,
                5,
                14,
                vec![(14, 14), (15, 13)],
                (6, 7),
                0,
                0,
            )
            .unwrap();
            assert!(
                py_to_value(codesign.bind(py)).unwrap()["verification_debt_added"]
                    .as_i64()
                    .unwrap()
                    > 0
            );

            let audit = evaluate_verification_debt_audit(
                py,
                4,
                PyEnum(AiArchitectureParadigm::NeuroSymbolicVerifier),
                3,
                15,
                vec![(16, 15), (15, 14), (14, 12)],
                (5, 6),
                0,
            )
            .unwrap();
            assert!(
                py_to_value(audit.bind(py)).unwrap()["debt_scrubbed"]
                    .as_i64()
                    .unwrap()
                    > 0
            );

            let cyber = evaluate_cyber_augmentation_loadout(
                py,
                vec!["MYOMER_ARM".to_string(), "SUBDERMAL_ARMOR".to_string()],
                1,
                true,
                true,
            )
            .unwrap();
            assert_eq!(
                py_to_value(cyber.bind(py)).unwrap()["subdermal_pr_ballistic"],
                2
            );

            let scada = evaluate_network_intrusion(
                py,
                PyEnum(SurveillanceNodeKind::AirGappedScadaEnclave),
                PyEnum(NetworkIntrusionPhase::CredentialOrExploitEntry),
                false,
                15,
                vec![(18, 17)],
                (4, 5),
                0,
                0,
            )
            .unwrap();
            assert_eq!(
                py_to_value(scada.bind(py)).unwrap()["access_blocked_by_air_gap"],
                true
            );

            let optical = evaluate_ecm_vs_eccm(
                py,
                PyEnum(EcmMethodId::BroadbandRfBarrageJammer),
                PyEnum(EccmHardeningId::OpticalLaserOrFiberTether),
                40.0,
                5.0,
                15,
                vec![(18, 17)],
                (5, 5),
                0,
            )
            .unwrap();
            assert_eq!(
                py_to_value(optical.bind(py)).unwrap()["blocked_by_optical_or_fiber_immunity"],
                true
            );

            let ew_hw = evaluate_custom_ew_hardware(
                py,
                Some(py_s(
                    r#"{"role":"JAMMER_POD","requested_ew_power":6,"workshop":"FIELD_BENCH"}"#,
                )),
            )
            .unwrap();
            assert_eq!(
                py_to_value(ew_hw.bind(py)).unwrap()["clamped_by_workshop"],
                true
            );
        });
    }
}
