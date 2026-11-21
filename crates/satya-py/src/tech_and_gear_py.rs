//! PyO3 bindings for Satya Ch. 7, 11, 18, 20–24 (`satya_py.tech`):
//! multi-spectrum stealth, surveillance tail craft & forensic trace decay,
//! contacts/cover/safehouses, blast/breaching, vehicle chases & component trauma,
//! environmental hazards, custom firearm/melee/armor engineering, modular drones/chassis trauma/cyber-prosthetics,
//! and 2035 electronic warfare (`SURVEILLANCE` -> `ECM` vs. `ECCM` -> custom EW hardware).

use pyo3::prelude::*;
use satya_engine::action_catalog_stealth::{
    ActiveSensoryChannel, AcousticSurfaceKind, AmbientNoiseMask, DigitalRfHygiene,
    ForensicDecayContext, IlluminationCondition, MultiSpectrumStealthParams, ObservationMode,
    ObserverAttentionState, SocialStealthWardrobeFit, StealthGait, TailStance,
    ThermalStealthProfile, VisualCountermeasure, evaluate_forensic_clue_search,
    evaluate_multi_spectrum_stealth_modifiers, evaluate_surveillance_tail_exchange,
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
    evaluate_contact_favor_request, evaluate_cover_identity_scrutiny,
    evaluate_safehouse_compromise_check,
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
    ChaseRangeBand, VEHICLE_CATALOG, VehicleComponentZone, VehicleId,
    evaluate_vehicle_chase_exchange, evaluate_vehicle_component_damage,
};
use satya_engine::environmental_hazards::{
    ElectricalHazardTier, HydrationEnvironment, LandingSurfaceClass, SepsisStage,
    SurfaceTractionTier, WindIntensityTier, WoundContaminationLevel, evaluate_acute_radiation,
    evaluate_dehydration, evaluate_electrical_shock, evaluate_fall_impact,
    evaluate_weather_and_footing, evaluate_wound_sepsis,
};
use satya_engine::explosives_and_arsenal::{
    ExplosiveOrdnanceId, evaluate_blast_and_fragmentation, evaluate_explosive_breach,
};
use satya_engine::primitives::Triad;
use satya_engine::robotics_and_ai_design::{
    AiArchitectureParadigm, ChassisHitZone, CyberAugmentId, DroneChassisClass, DroneModuleId,
    evaluate_cyber_augmentation_loadout, evaluate_human_ai_codesign_session,
    evaluate_modular_drone_design, evaluate_robot_chassis_damage, evaluate_verification_debt_audit,
};
use satya_schema::MaterialClass;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::psyche_social_py::to_py_err;

fn normalize_upper(raw: &str) -> String {
    raw.trim().to_ascii_uppercase().replace(['-', ' '], "_")
}

fn parse_serde_enum_ci<T: DeserializeOwned>(raw: &str, kind_label: &str) -> PyResult<T> {
    let trimmed = raw.trim();
    for candidate in [
        trimmed.to_string(),
        normalize_upper(trimmed),
        trimmed.to_ascii_lowercase().replace(['-', ' '], "_"),
    ] {
        if let Ok(parsed) = serde_json::from_value::<T>(Value::String(candidate)) {
            return Ok(parsed);
        }
    }
    Err(to_py_err(format!("Unknown {kind_label} `{raw}`")))
}

fn parse_coded_or_serde<T: DeserializeOwned>(
    raw: &str,
    from_code: fn(&str) -> Option<T>,
    kind_label: &str,
) -> PyResult<T> {
    let trimmed = raw.trim();
    let upper = normalize_upper(trimmed);
    let lower = upper.to_ascii_lowercase();
    if let Some(v) = from_code(&upper)
        .or_else(|| from_code(&lower))
        .or_else(|| from_code(trimmed))
    {
        return Ok(v);
    }
    parse_serde_enum_ci::<T>(trimmed, kind_label)
}

fn normalize_material_code(raw: &str) -> String {
    let norm = normalize_upper(raw);
    if norm == "CONCRETE" {
        "CONCRETE_BLOCK".to_string()
    } else {
        norm
    }
}

fn encode_json<T: serde::Serialize>(val: &T) -> PyResult<String> {
    serde_json::to_string(val).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

fn parse_characteristics_value(val: Option<&Value>) -> PyResult<CharacterCharacteristics> {
    let mut base = serde_json::to_value(CharacterCharacteristics::default())
        .map_err(|e| to_py_err(format!("Default characteristics encode failed: {e}")))?;
    if let Some(src) = val
        && !src.is_null()
    {
        let src_obj = src
            .as_object()
            .ok_or_else(|| to_py_err("Characteristics override must be a JSON object"))?;
        if let Some(dst_obj) = base.as_object_mut() {
            for (k, v) in src_obj {
                let key = if k == "for" { "for_stat" } else { k.as_str() };
                dst_obj.insert(key.to_string(), v.clone());
            }
        }
    }
    serde_json::from_value(base)
        .map_err(|e| to_py_err(format!("Invalid CharacterCharacteristics JSON: {e}")))
}

fn default_zero() -> i32 {
    0
}
fn default_zero_f64() -> f64 {
    0.0
}
fn default_two() -> usize {
    2
}

#[derive(Debug, Deserialize)]
struct PyMultiSpectrumStealthInput {
    #[serde(default)]
    infiltrator: Option<Value>,
    #[serde(default)]
    infiltrator_gait: Option<String>,
    #[serde(default)]
    infiltrator_encumbrance: Option<String>,
    #[serde(default = "default_zero")]
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
    #[serde(default = "default_zero")]
    cover_identity_pr_bonus: i32,
    #[serde(default)]
    observer: Option<Value>,
    #[serde(default)]
    observation_mode: Option<String>,
    #[serde(default)]
    observer_attention: Option<String>,
    #[serde(default = "default_two")]
    observer_skill_n: usize,
    #[serde(default = "default_zero_f64")]
    watch_hours_elapsed: f64,
    #[serde(default)]
    observer_channel: Option<String>,
    #[serde(default)]
    illumination: Option<String>,
    #[serde(default)]
    countermeasure: Option<String>,
    #[serde(default = "default_zero")]
    sensor_sweep_tool_delta_mod: i32,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyFallImpactParams {
    height_m: f64,
    surface: String,
    breakfall_net_successes: i32,
    clean_water_dive: bool,
}

impl Default for PyFallImpactParams {
    fn default() -> Self {
        Self {
            height_m: 3.0,
            surface: "hardwood_asphalt_concrete".to_string(),
            breakfall_net_successes: 0,
            clean_water_dive: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyDehydrationParams {
    env: String,
    water_consumed_liters: f64,
    extra_sweat_deficit_liters: f64,
}

impl Default for PyDehydrationParams {
    fn default() -> Self {
        Self {
            env: "temperate_low_activity".to_string(),
            water_consumed_liters: 3.0,
            extra_sweat_deficit_liters: 0.0,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyRadiationParams {
    cumulative_sieverts_sv: f64,
}

impl Default for PyRadiationParams {
    fn default() -> Self {
        Self {
            cumulative_sieverts_sv: 0.0,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyElectricalShockParams {
    tier: String,
    wet_or_submerged: bool,
    insulated_ppe: bool,
    hlt: i32,
    dis: i32,
    player_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
}

impl Default for PyElectricalShockParams {
    fn default() -> Self {
        Self {
            tier: "taser_cew_pulsed".to_string(),
            wet_or_submerged: false,
            insulated_ppe: false,
            hlt: 10,
            dis: 10,
            player_pairs: vec![(10, 10)],
            gm_pair: (10, 10),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyWeatherFootingParams {
    wind: String,
    traction: String,
    range_m: f64,
    has_crampons: bool,
}

impl Default for PyWeatherFootingParams {
    fn default() -> Self {
        Self {
            wind: "calm_under_15_kmh".to_string(),
            traction: "dry_solid".to_string(),
            range_m: 100.0,
            has_crampons: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyWoundSepsisParams {
    current_stage: String,
    contamination: String,
    antibiotics_active: bool,
    hlt: i32,
    player_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
}

impl Default for PyWoundSepsisParams {
    fn default() -> Self {
        Self {
            current_stage: "stage0_clean".to_string(),
            contamination: "clean_irrigated".to_string(),
            antibiotics_active: false,
            hlt: 10,
            player_pairs: vec![(10, 10)],
            gm_pair: (10, 10),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyCustomMeleeInput {
    chassis: String,
    material: String,
    tuning: String,
    workshop: String,
    outcome: String,
}

impl Default for PyCustomMeleeInput {
    fn default() -> Self {
        Self {
            chassis: "KNIFE".to_string(),
            material: "SPRING_CARBON".to_string(),
            tuning: "BALANCED".to_string(),
            workshop: "HOME_GARAGE".to_string(),
            outcome: "clean_standard".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyCustomArmorInput {
    material: String,
    coverage: String,
    workshop: String,
    outcome: String,
}

impl Default for PyCustomArmorInput {
    fn default() -> Self {
        Self {
            material: "PARA_ARAMID".to_string(),
            coverage: "TACTICAL_CARRIER".to_string(),
            workshop: "COMMERCIAL_CNC".to_string(),
            outcome: "clean_standard".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyCustomFirearmInput {
    caliber: String,
    bullet: String,
    handload_quality_q: f64,
    action_type: String,
    feed_type: String,
    capacity_rounds: i32,
    barrel_length_cm: f64,
    overall_length_cm: f64,
    unloaded_weight_kg: f64,
    barrel_mounting: String,
    muzzle_device: String,
    center_of_gravity: String,
    requested_tolerance_tau: f64,
    requested_barrel_quality_q: f64,
    requested_mfg_quality_q: i32,
    workshop: String,
    complexity: String,
    build_outcome: String,
    eval_range_m: f64,
}

impl Default for PyCustomFirearmInput {
    fn default() -> Self {
        Self {
            caliber: "parabellum9x19".to_string(),
            bullet: "FMJ_BALL".to_string(),
            handload_quality_q: 0.0,
            action_type: "striker_fired".to_string(),
            feed_type: "detachable_box_magazine".to_string(),
            capacity_rounds: 15,
            barrel_length_cm: 12.0,
            overall_length_cm: 20.0,
            unloaded_weight_kg: 0.85,
            barrel_mounting: "standard_pistol_mount".to_string(),
            muzzle_device: "bare_crown".to_string(),
            center_of_gravity: "chamber_neutral".to_string(),
            requested_tolerance_tau: 0.0,
            requested_barrel_quality_q: 0.0,
            requested_mfg_quality_q: 0,
            workshop: "HOME_GARAGE".to_string(),
            complexity: "CUSTOM_SUBASSEMBLY".to_string(),
            build_outcome: "clean_standard".to_string(),
            eval_range_m: 25.0,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyCustomEwInput {
    role: String,
    requested_ew_power: i32,
    rf_stealth_baffled: bool,
    battery_wh: f64,
    workshop: String,
    complexity: String,
    build_outcome: String,
}

impl Default for PyCustomEwInput {
    fn default() -> Self {
        Self {
            role: "PASSIVE_DF_SNIFFER".to_string(),
            requested_ew_power: 3,
            rf_stealth_baffled: true,
            battery_wh: 220.0,
            workshop: "COMMERCIAL_CNC".to_string(),
            complexity: "CUSTOM_SUBASSEMBLY".to_string(),
            build_outcome: "clean_standard".to_string(),
        }
    }
}

/// Evaluates Multi-Spectrum / Social Stealth and Passive Awareness vs. Active Observation (Ch. 7).
#[pyfunction]
pub fn evaluate_multi_spectrum_stealth_json(params_json: &str) -> PyResult<String> {
    let raw: PyMultiSpectrumStealthInput = if params_json.trim().is_empty() {
        serde_json::from_str("{}")
            .map_err(|e| to_py_err(format!("Default stealth parse failed: {e}")))?
    } else {
        serde_json::from_str(params_json)
            .map_err(|e| to_py_err(format!("Invalid stealth params JSON: {e}")))?
    };

    let params = MultiSpectrumStealthParams {
        infiltrator: parse_characteristics_value(raw.infiltrator.as_ref())?,
        infiltrator_gait: match raw.infiltrator_gait.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "infiltrator_gait")?,
            None => StealthGait::StealthCreep,
        },
        infiltrator_encumbrance: match raw.infiltrator_encumbrance.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "infiltrator_encumbrance")?,
            None => EncumbranceTier::Tier0Unencumbered,
        },
        infiltrator_camouflage_bonus_delta_ext: raw.infiltrator_camouflage_bonus_delta_ext,
        acoustic_surface: match raw.acoustic_surface.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "acoustic_surface")?,
            None => AcousticSurfaceKind::ConcreteAsphaltOrTile,
        },
        ambient_noise: match raw.ambient_noise.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "ambient_noise")?,
            None => AmbientNoiseMask::QuietSuburbanNight,
        },
        thermal_profile: match raw.thermal_profile.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "thermal_profile")?,
            None => ThermalStealthProfile::UnshieldedStandard,
        },
        rf_hygiene: match raw.rf_hygiene.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "rf_hygiene")?,
            None => DigitalRfHygiene::AirGappedFaradayPouch,
        },
        social_wardrobe_fit: match raw.social_wardrobe_fit.as_deref() {
            Some(s) if !s.trim().is_empty() => {
                Some(parse_serde_enum_ci::<SocialStealthWardrobeFit>(s, "social_wardrobe_fit")?)
            }
            _ => None,
        },
        cover_identity_pr_bonus: raw.cover_identity_pr_bonus,
        observer: parse_characteristics_value(raw.observer.as_ref())?,
        observation_mode: match raw.observation_mode.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "observation_mode")?,
            None => ObservationMode::PassiveAwareness,
        },
        observer_attention: match raw.observer_attention.as_deref() {
            Some(s) => parse_coded_or_serde(
                s,
                ObserverAttentionState::from_code_str,
                "observer_attention",
            )?,
            None => ObserverAttentionState::RoutinePatrolBored,
        },
        observer_skill_n: raw.observer_skill_n,
        watch_hours_elapsed: raw.watch_hours_elapsed,
        observer_channel: match raw.observer_channel.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "observer_channel")?,
            None => ActiveSensoryChannel::VisualSigFoc,
        },
        illumination: match raw.illumination.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "illumination")?,
            None => IlluminationCondition::DimLightDuskShadows,
        },
        countermeasure: match raw.countermeasure.as_deref() {
            Some(s) => parse_serde_enum_ci(s, "countermeasure")?,
            None => VisualCountermeasure::None,
        },
        sensor_sweep_tool_delta_mod: raw.sensor_sweep_tool_delta_mod,
    };

    encode_json(&evaluate_multi_spectrum_stealth_modifiers(&params))
}

/// Evaluates one urban tail / Surveillance Detection Route (`SDR`) leg (Ch. 7 §4.1).
#[pyfunction]
#[pyo3(signature = (tail_stance, shadower_skill_n, shadower_control_c, shadower_internal_mod, shadower_tool_delta_mod, target_sdr_delta_ext, shadower_pairs, target_gm_pair))]
pub fn evaluate_surveillance_tail_json(
    tail_stance: &str,
    shadower_skill_n: usize,
    shadower_control_c: i32,
    shadower_internal_mod: i32,
    shadower_tool_delta_mod: i32,
    target_sdr_delta_ext: i32,
    shadower_pairs: Vec<(i32, i32)>,
    target_gm_pair: (i32, i32),
) -> PyResult<String> {
    let stance: TailStance = parse_serde_enum_ci(tail_stance, "tail_stance")?;
    let res = evaluate_surveillance_tail_exchange(
        stance,
        shadower_skill_n,
        shadower_control_c,
        shadower_internal_mod,
        shadower_tool_delta_mod,
        target_sdr_delta_ext,
        &shadower_pairs,
        target_gm_pair,
        ContestedTieRule::HybridSignedMargin,
    );
    encode_json(&res)
}

/// Evaluates trace decay and forensic / DFIR / deepfake audit sweeps (Ch. 7 §4.2–§4.3).
#[pyfunction]
#[pyo3(signature = (decay_context, hours_elapsed, base_obscurity_delta_ext, investigator_skill_n, investigator_control_c, internal_mod, tool_delta_mod, investigator_pairs, gm_pair))]
pub fn evaluate_forensic_clue_search_json(
    decay_context: &str,
    hours_elapsed: f64,
    base_obscurity_delta_ext: i32,
    investigator_skill_n: usize,
    investigator_control_c: i32,
    internal_mod: i32,
    tool_delta_mod: i32,
    investigator_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
) -> PyResult<String> {
    let ctx: ForensicDecayContext = parse_serde_enum_ci(decay_context, "decay_context")?;
    let res = evaluate_forensic_clue_search(
        ctx,
        hours_elapsed,
        base_obscurity_delta_ext,
        investigator_skill_n,
        investigator_control_c,
        internal_mod,
        tool_delta_mod,
        &investigator_pairs,
        gm_pair,
        ContestedTieRule::HybridSignedMargin,
    );
    encode_json(&res)
}

/// Evaluates a Campaign Contact favor request with reach/loyalty/exposure modifiers (Ch. 11 §3.3).
#[pyfunction]
#[pyo3(signature = (reach_access_tier, reliability_trust, exposure_burn_risk, favor_severity, required_reach_tier, jurisdiction_heat, requester_skill_n, requester_control_c, internal_mod, player_pairs, gm_pair))]
pub fn evaluate_contact_favor_json(
    reach_access_tier: u8,
    reliability_trust: u8,
    exposure_burn_risk: u8,
    favor_severity: &str,
    required_reach_tier: u8,
    jurisdiction_heat: u8,
    requester_skill_n: usize,
    requester_control_c: i32,
    internal_mod: i32,
    player_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
) -> PyResult<String> {
    let contact = ContactProfile {
        reach_access_tier,
        reliability_trust,
        exposure_burn_risk,
    };
    let favor: FavorSeverityTier = parse_serde_enum_ci(favor_severity, "favor_severity")?;
    let res = evaluate_contact_favor_request(
        contact,
        favor,
        required_reach_tier,
        jurisdiction_heat,
        requester_skill_n,
        requester_control_c,
        internal_mod,
        &player_pairs,
        gm_pair,
        ContestedTieRule::HybridSignedMargin,
    );
    encode_json(&res)
}

/// Evaluates layered Cover Identity checkpoint or biometric scrutiny (Ch. 11 §3.4).
#[pyfunction]
#[pyo3(signature = (cover_tier, scrutiny_level, cover_heat, holder_skill_n, holder_control_c, internal_mod, player_pairs, gm_pair))]
pub fn evaluate_cover_identity_scrutiny_json(
    cover_tier: &str,
    scrutiny_level: &str,
    cover_heat: u8,
    holder_skill_n: usize,
    holder_control_c: i32,
    internal_mod: i32,
    player_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
) -> PyResult<String> {
    let tier = parse_coded_or_serde(cover_tier, CoverIdentityTier::from_code_str, "cover_tier")?;
    let scrutiny: CoverScrutinyLevel = parse_serde_enum_ci(scrutiny_level, "scrutiny_level")?;
    let res = evaluate_cover_identity_scrutiny(
        tier,
        scrutiny,
        cover_heat,
        holder_skill_n,
        holder_control_c,
        internal_mod,
        &player_pairs,
        gm_pair,
        ContestedTieRule::HybridSignedMargin,
    );
    encode_json(&res)
}

/// Evaluates whether returning hot or tailed compromises a Safehouse, enforcing Faraday shielding (Ch. 11 §3.5).
#[pyfunction]
#[pyo3(signature = (safehouse_tier, returning_heat, actively_tailed, rf_hygiene_breached, operator_skill_n, operator_control_c, internal_mod, operator_pairs, gm_pair))]
pub fn evaluate_safehouse_compromise_json(
    safehouse_tier: &str,
    returning_heat: u8,
    actively_tailed: bool,
    rf_hygiene_breached: bool,
    operator_skill_n: usize,
    operator_control_c: i32,
    internal_mod: i32,
    operator_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
) -> PyResult<String> {
    let tier = parse_coded_or_serde(safehouse_tier, SafehouseTier::from_code_str, "safehouse_tier")?;
    let res = evaluate_safehouse_compromise_check(
        tier,
        returning_heat,
        actively_tailed,
        rf_hygiene_breached,
        operator_skill_n,
        operator_control_c,
        internal_mod,
        &operator_pairs,
        gm_pair,
        ContestedTieRule::HybridSignedMargin,
    );
    encode_json(&res)
}

/// Evaluates explosive blast overpressure (`1.5x` indoor reflection), shrapnel hits, and flash/smoke effects (Ch. 18 §3.1).
#[pyfunction]
#[pyo3(signature = (ordnance_code, distance_m, enclosed_room=false, target_is_prone=false, intervening_barrier_pr=0))]
pub fn evaluate_blast_and_fragmentation_json(
    ordnance_code: &str,
    distance_m: f64,
    enclosed_room: bool,
    target_is_prone: bool,
    intervening_barrier_pr: i32,
) -> PyResult<String> {
    let ordnance = parse_coded_or_serde(
        ordnance_code,
        ExplosiveOrdnanceId::from_code_str,
        "explosive ordnance",
    )?;
    encode_json(&evaluate_blast_and_fragmentation(
        ordnance,
        distance_m,
        enclosed_room,
        target_is_prone,
        intervening_barrier_pr,
    ))
}

/// Evaluates shaped-charge / thermite / C4 structural breaching against a barrier [`MaterialClass`] (Ch. 18 §3.2).
#[pyfunction]
#[pyo3(signature = (ordnance_code, target_material_code, demolitions_net_successes=0, friendly_standoff_m=6.0, enclosed_interior=true))]
pub fn evaluate_explosive_breach_json(
    ordnance_code: &str,
    target_material_code: &str,
    demolitions_net_successes: i32,
    friendly_standoff_m: f64,
    enclosed_interior: bool,
) -> PyResult<String> {
    let ordnance = parse_coded_or_serde(
        ordnance_code,
        ExplosiveOrdnanceId::from_code_str,
        "explosive ordnance",
    )?;
    let mat_norm = normalize_material_code(target_material_code);
    let material: MaterialClass = parse_serde_enum_ci(&mat_norm, "target_material")?;
    encode_json(&evaluate_explosive_breach(
        ordnance,
        material,
        demolitions_net_successes,
        friendly_standoff_m,
        enclosed_interior,
    ))
}

fn resolve_vehicle_spec(
    vehicle_code: &str,
) -> PyResult<&'static satya_engine::environment_vehicles::VehicleCatalogSpec> {
    if vehicle_code.trim().is_empty() {
        return Ok(&VEHICLE_CATALOG[1]);
    }
    let id = parse_coded_or_serde(vehicle_code, VehicleId::from_code_str, "vehicle_code")?;
    Ok(id.spec())
}

/// Evaluates a high-speed Vehicular Chase exchange bounded by the canonical vehicle envelope (Ch. 20 §3.1–§3.2).
#[pyfunction]
#[pyo3(signature = (vehicle_code, pilot_triad, opponent_triad, current_band="close15m", wants_to_close=true, attempts_pit_or_ram=false))]
pub fn evaluate_vehicle_chase_json(
    vehicle_code: &str,
    pilot_triad: (i32, i32, i32),
    opponent_triad: (i32, i32, i32),
    current_band: &str,
    wants_to_close: bool,
    attempts_pit_or_ram: bool,
) -> PyResult<String> {
    let spec = resolve_vehicle_spec(vehicle_code)?;
    let profile = spec.to_envelope_profile();
    let band: ChaseRangeBand = parse_serde_enum_ci(current_band, "current_band")?;
    let outcome = evaluate_vehicle_chase_exchange(
        &profile,
        Triad::new(pilot_triad.0, pilot_triad.1, pilot_triad.2),
        Triad::new(opponent_triad.0, opponent_triad.1, opponent_triad.2),
        band,
        wants_to_close,
        attempts_pit_or_ram,
    );
    encode_json(&outcome)
}

/// Evaluates vehicle component trauma, tire blowouts, and EV battery thermal runaway (Ch. 20 §3.6).
#[pyfunction]
#[pyo3(signature = (vehicle_code, zone, uses_run_flat_tires=false, hits_ballistic_glass=false, effective_pen=4, effective_raw_impact=8))]
pub fn evaluate_vehicle_component_damage_json(
    vehicle_code: &str,
    zone: &str,
    uses_run_flat_tires: bool,
    hits_ballistic_glass: bool,
    effective_pen: i32,
    effective_raw_impact: i32,
) -> PyResult<String> {
    let spec = resolve_vehicle_spec(vehicle_code)?;
    let zone_enum: VehicleComponentZone = parse_serde_enum_ci(zone, "vehicle_component_zone")?;
    encode_json(&evaluate_vehicle_component_damage(
        spec,
        zone_enum,
        uses_run_flat_tires,
        hits_ballistic_glass,
        effective_pen,
        effective_raw_impact,
    ))
}

/// Evaluates Ch. 21 environmental hazards (`fall_impact`, `dehydration`, `radiation`, `electrical_shock`, `weather_footing`, `wound_sepsis`).
#[pyfunction]
#[pyo3(signature = (hazard_kind, params_json="{}"))]
pub fn evaluate_environmental_hazard_json(hazard_kind: &str, params_json: &str) -> PyResult<String> {
    let norm_kind = normalize_upper(hazard_kind);
    let raw_json = if params_json.trim().is_empty() {
        "{}"
    } else {
        params_json
    };
    match norm_kind.as_str() {
        "FALL" | "FALL_IMPACT" => {
            let p: PyFallImpactParams = serde_json::from_str(raw_json)
                .map_err(|e| to_py_err(format!("Invalid fall_impact JSON: {e}")))?;
            let surface: LandingSurfaceClass = parse_serde_enum_ci(&p.surface, "landing_surface")?;
            encode_json(&evaluate_fall_impact(
                p.height_m,
                surface,
                p.breakfall_net_successes,
                p.clean_water_dive,
            ))
        }
        "DEHYDRATION" | "HYDRATION" => {
            let p: PyDehydrationParams = serde_json::from_str(raw_json)
                .map_err(|e| to_py_err(format!("Invalid dehydration JSON: {e}")))?;
            let env: HydrationEnvironment = parse_serde_enum_ci(&p.env, "hydration_environment")?;
            encode_json(&evaluate_dehydration(
                env,
                p.water_consumed_liters,
                p.extra_sweat_deficit_liters,
            ))
        }
        "RADIATION" | "ACUTE_RADIATION" => {
            let p: PyRadiationParams = serde_json::from_str(raw_json)
                .map_err(|e| to_py_err(format!("Invalid radiation JSON: {e}")))?;
            encode_json(&evaluate_acute_radiation(p.cumulative_sieverts_sv))
        }
        "SHOCK" | "ELECTRICAL_SHOCK" | "ELECTRICITY" => {
            let p: PyElectricalShockParams = serde_json::from_str(raw_json)
                .map_err(|e| to_py_err(format!("Invalid electrical_shock JSON: {e}")))?;
            let tier: ElectricalHazardTier = parse_serde_enum_ci(&p.tier, "electrical_hazard_tier")?;
            encode_json(&evaluate_electrical_shock(
                tier,
                p.wet_or_submerged,
                p.insulated_ppe,
                p.hlt,
                p.dis,
                &p.player_pairs,
                p.gm_pair,
                ContestedTieRule::HybridSignedMargin,
            ))
        }
        "WEATHER" | "WEATHER_FOOTING" | "WIND" | "TRACTION" => {
            let p: PyWeatherFootingParams = serde_json::from_str(raw_json)
                .map_err(|e| to_py_err(format!("Invalid weather_footing JSON: {e}")))?;
            let wind: WindIntensityTier = parse_serde_enum_ci(&p.wind, "wind_intensity")?;
            let traction: SurfaceTractionTier =
                parse_serde_enum_ci(&p.traction, "surface_traction")?;
            encode_json(&evaluate_weather_and_footing(
                wind,
                traction,
                p.range_m,
                p.has_crampons,
            ))
        }
        "SEPSIS" | "WOUND_SEPSIS" => {
            let p: PyWoundSepsisParams = serde_json::from_str(raw_json)
                .map_err(|e| to_py_err(format!("Invalid wound_sepsis JSON: {e}")))?;
            let stage: SepsisStage = parse_serde_enum_ci(&p.current_stage, "sepsis_stage")?;
            let contam: WoundContaminationLevel =
                parse_serde_enum_ci(&p.contamination, "wound_contamination")?;
            encode_json(&evaluate_wound_sepsis(
                stage,
                contam,
                p.antibiotics_active,
                p.hlt,
                &p.player_pairs,
                p.gm_pair,
                ContestedTieRule::HybridSignedMargin,
            ))
        }
        _ => Err(to_py_err(format!(
            "Unknown hazard_kind `{hazard_kind}`; expected fall_impact, dehydration, radiation, electrical_shock, weather_footing, or wound_sepsis"
        ))),
    }
}

/// Evaluates Ch. 22 custom firearm, melee weapon, or body armor engineering against workshop tolerances.
#[pyfunction]
#[pyo3(signature = (design_kind, spec_json="{}"))]
pub fn evaluate_custom_gear_design_json(design_kind: &str, spec_json: &str) -> PyResult<String> {
    let norm_kind = normalize_upper(design_kind);
    let raw_json = if spec_json.trim().is_empty() {
        "{}"
    } else {
        spec_json
    };
    match norm_kind.as_str() {
        "MELEE" | "CUSTOM_MELEE" => {
            let p: PyCustomMeleeInput = serde_json::from_str(raw_json)
                .map_err(|e| to_py_err(format!("Invalid custom melee JSON: {e}")))?;
            let chassis =
                parse_coded_or_serde(&p.chassis, MeleeChassisClass::from_code_str, "melee_chassis")?;
            let material = parse_coded_or_serde(
                &p.material,
                MeleeMaterialGrade::from_code_str,
                "melee_material",
            )?;
            let tuning = parse_coded_or_serde(
                &p.tuning,
                MeleeErgonomicTuning::from_code_str,
                "melee_tuning",
            )?;
            let workshop = parse_coded_or_serde(
                &p.workshop,
                WorkshopFacilityTier::from_code_str,
                "workshop_tier",
            )?;
            let outcome: BuildOutcomeQuality = parse_serde_enum_ci(&p.outcome, "build_outcome")?;
            encode_json(&evaluate_custom_melee_design(
                chassis, material, tuning, workshop, outcome,
            ))
        }
        "ARMOR" | "CUSTOM_ARMOR" => {
            let p: PyCustomArmorInput = serde_json::from_str(raw_json)
                .map_err(|e| to_py_err(format!("Invalid custom armor JSON: {e}")))?;
            let material = parse_coded_or_serde(
                &p.material,
                ArmorMaterialArchitecture::from_code_str,
                "armor_material",
            )?;
            let coverage =
                parse_coded_or_serde(&p.coverage, ArmorCoverageTier::from_code_str, "armor_coverage")?;
            let workshop = parse_coded_or_serde(
                &p.workshop,
                WorkshopFacilityTier::from_code_str,
                "workshop_tier",
            )?;
            let outcome: BuildOutcomeQuality = parse_serde_enum_ci(&p.outcome, "build_outcome")?;
            encode_json(&evaluate_custom_armor_design(
                material, coverage, workshop, outcome,
            ))
        }
        "FIREARM" | "CUSTOM_FIREARM" | "GUN" => {
            let p: PyCustomFirearmInput = serde_json::from_str(raw_json)
                .map_err(|e| to_py_err(format!("Invalid custom firearm JSON: {e}")))?;
            let bp = CustomFirearmBlueprint {
                name: "Custom Build",
                caliber: parse_serde_enum_ci::<AmmoCaliberId>(&p.caliber, "caliber")?,
                bullet: parse_coded_or_serde(
                    &p.bullet,
                    BulletConstructionKind::from_code_str,
                    "bullet",
                )?,
                handload_quality_q: p.handload_quality_q,
                action_type: parse_serde_enum_ci::<FirearmActionType>(
                    &p.action_type,
                    "action_type",
                )?,
                feed_type: parse_serde_enum_ci::<WeaponFeedType>(&p.feed_type, "feed_type")?,
                capacity_rounds: p.capacity_rounds,
                barrel_length_cm: p.barrel_length_cm,
                overall_length_cm: p.overall_length_cm,
                unloaded_weight_kg: p.unloaded_weight_kg,
                barrel_mounting: parse_serde_enum_ci::<BarrelMountingType>(
                    &p.barrel_mounting,
                    "barrel_mounting",
                )?,
                muzzle_device: parse_serde_enum_ci::<MuzzleDeviceType>(
                    &p.muzzle_device,
                    "muzzle_device",
                )?,
                center_of_gravity: parse_serde_enum_ci::<CenterOfGravityProfile>(
                    &p.center_of_gravity,
                    "center_of_gravity",
                )?,
                requested_tolerance_tau: p.requested_tolerance_tau,
                requested_barrel_quality_q: p.requested_barrel_quality_q,
                requested_mfg_quality_q: p.requested_mfg_quality_q,
                workshop: parse_coded_or_serde(
                    &p.workshop,
                    WorkshopFacilityTier::from_code_str,
                    "workshop_tier",
                )?,
                complexity: parse_coded_or_serde(
                    &p.complexity,
                    ProjectClockComplexity::from_code_str,
                    "complexity",
                )?,
                build_outcome: parse_serde_enum_ci::<BuildOutcomeQuality>(
                    &p.build_outcome,
                    "build_outcome",
                )?,
            };
            encode_json(&build_custom_firearm_design(&bp, p.eval_range_m))
        }
        _ => Err(to_py_err(format!(
            "Unknown design_kind `{design_kind}`; expected melee, armor, or firearm"
        ))),
    }
}

/// Evaluates modular drone / UGV payload utilization, endurance minutes, armor, and cost (Ch. 23 §1).
#[pyfunction]
#[pyo3(signature = (chassis_code, module_codes=vec![], extra_armor_pr=0, autonomy_compute_w=0.0))]
pub fn evaluate_modular_drone_design_json(
    chassis_code: &str,
    module_codes: Vec<String>,
    extra_armor_pr: i32,
    autonomy_compute_w: f64,
) -> PyResult<String> {
    let chassis =
        parse_coded_or_serde(chassis_code, DroneChassisClass::from_code_str, "drone_chassis")?;
    let mut modules = Vec::with_capacity(module_codes.len());
    for raw in module_codes {
        modules.push(parse_coded_or_serde(
            &raw,
            DroneModuleId::from_code_str,
            "drone_module",
        )?);
    }
    encode_json(&evaluate_modular_drone_design(
        chassis,
        &modules,
        extra_armor_pr,
        autonomy_compute_w,
    ))
}

/// Resolves component damage, battery thermal fires, and EMP reboot ticks on a drone/UGV chassis (Ch. 23 §2).
#[pyfunction]
#[pyo3(signature = (current_si, chassis_pr, hit_zone, attack_pen, raw_impact, net_successes=1, is_emp_or_arc=false, emp_hardened=false))]
pub fn evaluate_robot_chassis_damage_json(
    current_si: i32,
    chassis_pr: i32,
    hit_zone: &str,
    attack_pen: i32,
    raw_impact: i32,
    net_successes: i32,
    is_emp_or_arc: bool,
    emp_hardened: bool,
) -> PyResult<String> {
    let zone = parse_coded_or_serde(hit_zone, ChassisHitZone::from_code_str, "chassis_hit_zone")?;
    encode_json(&evaluate_robot_chassis_damage(
        current_si,
        chassis_pr,
        zone,
        attack_pen,
        raw_impact,
        net_successes,
        is_emp_or_arc,
        emp_hardened,
    ))
}

/// Resolves Human–AI Co-Design engineering velocity vs. Verification Debt $D_{\text{verify}}$ (Ch. 23 §3.2).
#[pyfunction]
#[pyo3(signature = (paradigm_code, human_domain_n, ai_autonomy_leverage, human_control_c, internal_mod, base_task_delta_ext, human_pairs, gm_pair))]
pub fn evaluate_human_ai_codesign_json(
    paradigm_code: &str,
    human_domain_n: i32,
    ai_autonomy_leverage: i32,
    human_control_c: i32,
    internal_mod: i32,
    base_task_delta_ext: i32,
    human_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
) -> PyResult<String> {
    let paradigm = parse_coded_or_serde(
        paradigm_code,
        AiArchitectureParadigm::from_code_str,
        "ai_architecture_paradigm",
    )?;
    let res = evaluate_human_ai_codesign_session(
        paradigm,
        human_domain_n,
        ai_autonomy_leverage,
        human_control_c,
        internal_mod,
        base_task_delta_ext,
        &human_pairs,
        gm_pair,
        ContestedTieRule::HybridSignedMargin,
    );
    encode_json(&res)
}

/// Resolves a Formal Verification & Weight Audit check to scrub Verification Debt (Ch. 23 §3.3).
#[pyfunction]
#[pyo3(signature = (current_debt, paradigm_code, auditor_n, auditor_control_c, internal_mod, auditor_pairs, gm_pair))]
pub fn evaluate_verification_debt_audit_json(
    current_debt: i32,
    paradigm_code: &str,
    auditor_n: i32,
    auditor_control_c: i32,
    internal_mod: i32,
    auditor_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
) -> PyResult<String> {
    let paradigm = parse_coded_or_serde(
        paradigm_code,
        AiArchitectureParadigm::from_code_str,
        "ai_architecture_paradigm",
    )?;
    let res = evaluate_verification_debt_audit(
        current_debt,
        paradigm,
        auditor_n,
        auditor_control_c,
        internal_mod,
        &auditor_pairs,
        gm_pair,
        ContestedTieRule::HybridSignedMargin,
    );
    encode_json(&res)
}

/// Evaluates clinical cyber-prosthetic / neural augmentation Triad gains, subdermal armor, and rejection (Ch. 23 §4).
#[pyfunction]
#[pyo3(signature = (augment_codes, surgical_quality_q=0, emp_hardened=false, immunosuppressant_compliant=true))]
pub fn evaluate_cyber_augmentation_loadout_json(
    augment_codes: Vec<String>,
    surgical_quality_q: i32,
    emp_hardened: bool,
    immunosuppressant_compliant: bool,
) -> PyResult<String> {
    let mut augments = Vec::with_capacity(augment_codes.len());
    for raw in augment_codes {
        augments.push(parse_coded_or_serde(
            &raw,
            CyberAugmentId::from_code_str,
            "cyber_augment",
        )?);
    }
    encode_json(&evaluate_cyber_augmentation_loadout(
        &augments,
        surgical_quality_q,
        emp_hardened,
        immunosuppressant_compliant,
    ))
}

/// Evaluates a 2035 surveillance / SCADA intrusion step, enforcing hard physical-tap air-gap rules (Ch. 24 §1).
#[pyfunction]
#[pyo3(signature = (node_code, phase_code, is_physically_tapped, operator_control_c, internal_mod, tool_mod, operator_pairs, gm_pair))]
pub fn evaluate_network_intrusion_json(
    node_code: &str,
    phase_code: &str,
    is_physically_tapped: bool,
    operator_control_c: i32,
    internal_mod: i32,
    tool_mod: i32,
    operator_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
) -> PyResult<String> {
    let node = parse_coded_or_serde(
        node_code,
        SurveillanceNodeKind::from_code_str,
        "surveillance_node",
    )?;
    let phase = parse_coded_or_serde(
        phase_code,
        NetworkIntrusionPhase::from_code_str,
        "network_intrusion_phase",
    )?;
    let res = evaluate_network_or_sensor_intrusion(
        node,
        phase,
        is_physically_tapped,
        operator_control_c,
        internal_mod,
        tool_mod,
        &operator_pairs,
        gm_pair,
        ContestedTieRule::HybridSignedMargin,
    );
    encode_json(&res)
}

/// Evaluates ECM jamming vs. ECCM link hardening, RF optical/fiber immunity, and DF triangulation (Ch. 24 §2–§3).
#[pyfunction]
#[pyo3(signature = (ecm_code, eccm_code, distance_m, active_duration_min, operator_control_c, internal_mod, operator_pairs, gm_pair))]
pub fn evaluate_ecm_vs_eccm_json(
    ecm_code: &str,
    eccm_code: &str,
    distance_m: f64,
    active_duration_min: f64,
    operator_control_c: i32,
    internal_mod: i32,
    operator_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
) -> PyResult<String> {
    let ecm = parse_coded_or_serde(ecm_code, EcmMethodId::from_code_str, "ecm_method")?;
    let eccm = parse_coded_or_serde(eccm_code, EccmHardeningId::from_code_str, "eccm_hardening")?;
    let res = evaluate_ecm_vs_eccm_contest(
        ecm,
        eccm,
        distance_m,
        active_duration_min,
        operator_control_c,
        internal_mod,
        &operator_pairs,
        gm_pair,
        ContestedTieRule::HybridSignedMargin,
    );
    encode_json(&res)
}

/// Evaluates custom ECM/ECCM/DF hardware blueprints against WorkshopFacilityTier precision ceilings (Ch. 24 §4).
#[pyfunction]
#[pyo3(signature = (spec_json="{}"))]
pub fn evaluate_custom_ew_hardware_json(spec_json: &str) -> PyResult<String> {
    let raw_json = if spec_json.trim().is_empty() {
        "{}"
    } else {
        spec_json
    };
    let p: PyCustomEwInput = serde_json::from_str(raw_json)
        .map_err(|e| to_py_err(format!("Invalid custom EW hardware JSON: {e}")))?;
    let bp = CustomEwHardwareBlueprint {
        role: parse_coded_or_serde(&p.role, EwHardwareRole::from_code_str, "ew_hardware_role")?,
        requested_ew_power: p.requested_ew_power,
        rf_stealth_baffled: p.rf_stealth_baffled,
        battery_wh: p.battery_wh,
        workshop: parse_coded_or_serde(
            &p.workshop,
            WorkshopFacilityTier::from_code_str,
            "workshop_tier",
        )?,
        complexity: parse_coded_or_serde(
            &p.complexity,
            ProjectClockComplexity::from_code_str,
            "complexity",
        )?,
        build_outcome: parse_serde_enum_ci::<BuildOutcomeQuality>(
            &p.build_outcome,
            "build_outcome",
        )?,
    };
    encode_json(&evaluate_custom_ew_hardware_design(&bp))
}

/// Registers Ch. 7, 11, 18, and 20–24 technical/engineering/EW/hazard PyO3 functions on `satya_py._native`.
pub fn register_tech_and_gear_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(evaluate_multi_spectrum_stealth_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_surveillance_tail_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_forensic_clue_search_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_contact_favor_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_cover_identity_scrutiny_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_safehouse_compromise_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_blast_and_fragmentation_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_explosive_breach_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_vehicle_chase_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_vehicle_component_damage_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_environmental_hazard_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_custom_gear_design_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_modular_drone_design_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_robot_chassis_damage_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_human_ai_codesign_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_verification_debt_audit_json, m)?)?;
    m.add_function(wrap_pyfunction!(
        evaluate_cyber_augmentation_loadout_json,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(evaluate_network_intrusion_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_ecm_vs_eccm_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_custom_ew_hardware_json, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tech_and_gear_py_covers_chapters_7_11_18_and_20_to_24() {
        // Ch. 7 Stealth, Tail & Forensics
        let stealth: Value = serde_json::from_str(
            &evaluate_multi_spectrum_stealth_json(
                r#"{"infiltrator":{"kin":15,"itu":14},"infiltrator_gait":"STEALTH_CREEP","rf_hygiene":"ACTIVE_SMARTPHONE_OR_BLE_TRAIL","observer_attention":"HIGH_ALERT","observer_skill_n":4}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(stealth["auto_pings_heat_3_plus"], true);
        assert_eq!(stealth["observer_effective_pairs_n"], 4);

        let tail: Value = serde_json::from_str(
            &evaluate_surveillance_tail_json(
                "TIGHT_TAIL",
                2,
                14,
                0,
                0,
                0,
                vec![(16, 15), (14, 14)],
                (5, 6),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(tail["tail_maintained"], true);

        let forensic: Value = serde_json::from_str(
            &evaluate_forensic_clue_search_json(
                "PROFESSIONALLY_SCRUBBED",
                48.0,
                2,
                3,
                15,
                0,
                -2,
                vec![(16, 14), (15, 13), (14, 14)],
                (4, 5),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(forensic["decay_delta_ext"].as_i64().unwrap() >= 6);

        // Ch. 11 Contacts, Cover Identity & Safehouse
        let contact: Value = serde_json::from_str(
            &evaluate_contact_favor_json(
                2,
                3,
                1,
                "MODERATE",
                2,
                2,
                2,
                14,
                0,
                vec![(15, 14), (16, 15)],
                (7, 8),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(contact["favor_granted"].is_boolean());

        let cover: Value = serde_json::from_str(
            &evaluate_cover_identity_scrutiny_json(
                "COVER_TIER_2_DB_ALIAS",
                "BIOMETRIC_AUDIT_OR_FEDERAL_CROSS_CHECK",
                6,
                2,
                11,
                0,
                vec![(4, 5), (3, 4)],
                (18, 17),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(cover["cover_burned"], true);

        let safe: Value = serde_json::from_str(
            &evaluate_safehouse_compromise_json(
                "SAFEHOUSE_FARADAY_SHOP",
                5,
                false,
                true,
                2,
                14,
                0,
                vec![(16, 15), (15, 15)],
                (6, 7),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(safe["faraday_blocked_rf_leak"], true);

        // Ch. 18 Explosives & Breaching
        let blast: Value = serde_json::from_str(
            &evaluate_blast_and_fragmentation_json("M67_FRAG", 2.0, true, false, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(blast["enclosed_room_reflected"], true);
        let breach: Value = serde_json::from_str(
            &evaluate_explosive_breach_json("LINEAR_SHAPED_BREACH", "CONCRETE", 2, 4.0, true)
                .unwrap(),
        )
        .unwrap();
        assert!(breach["effective_breach_pr"].as_i64().unwrap() > 0);

        // Ch. 20 Vehicles
        let chase: Value = serde_json::from_str(
            &evaluate_vehicle_chase_json(
                "COMPACT_URBAN_EV",
                (18, 16, 10),
                (10, 10, 10),
                "medium50m",
                true,
                false,
            )
            .unwrap(),
        )
        .unwrap();
        assert!(chase["next_band"].is_string());

        let v_dmg: Value = serde_json::from_str(
            &evaluate_vehicle_component_damage_json(
                "COMPACT_URBAN_EV",
                "fuel_or_ev_battery_pack",
                false,
                false,
                7,
                12,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(v_dmg["triggers_ev_thermal_runaway"], true);

        // Ch. 21 Environmental Hazards
        for (kind, body) in [
            ("fall_impact", r#"{"height_m":10.0,"surface":"hardwood_asphalt_concrete"}"#),
            ("dehydration", r#"{"env":"hot_arid_desert_or_jungle_tactical","water_consumed_liters":2.0}"#),
            ("radiation", r#"{"cumulative_sieverts_sv":2.5}"#),
            ("electrical_shock", r#"{"tier":"industrial_three_phase480_v_to5k_v","wet_or_submerged":true}"#),
            ("weather_footing", r#"{"wind":"gale36_to65_kmh","traction":"black_ice","range_m":400.0}"#),
            ("wound_sepsis", r#"{"current_stage":"stage1_localized_infection","contamination":"dirty_field_or_second_degree_burn","antibiotics_active":true}"#),
        ] {
            let v: Value =
                serde_json::from_str(&evaluate_environmental_hazard_json(kind, body).unwrap())
                    .unwrap();
            assert!(v.is_object());
        }

        // Ch. 22 Custom Engineering
        for (kind, body) in [
            ("melee", r#"{"chassis":"AXE_BREACHER","material":"TOOL_STEEL_S7","tuning":"HEAVY_IMPACT","workshop":"COMMERCIAL_CNC","outcome":"masterwork_extraordinary"}"#),
            ("armor", r#"{"material":"BORON_CARBIDE","coverage":"TACTICAL_CARRIER","workshop":"FIELD_BENCH","outcome":"rushed_or_breached"}"#),
            ("firearm", r#"{"caliber":"nato556x45","bullet":"ARMOR_PIERCING_AP","workshop":"COMMERCIAL_CNC","muzzle_device":"sound_suppressor"}"#),
        ] {
            let v: Value =
                serde_json::from_str(&evaluate_custom_gear_design_json(kind, body).unwrap())
                    .unwrap();
            assert!(v.is_object());
        }

        // Ch. 23 Robotics, Drones, AI Co-Design & Cybernetics
        let drone: Value = serde_json::from_str(
            &evaluate_modular_drone_design_json(
                "TACTICAL_FPV_QUAD",
                vec!["EO_FLIR_GIMBAL".to_string(), "SOLID_STATE_BATTERY".to_string()],
                1,
                12.0,
            )
            .unwrap(),
        )
        .unwrap();
        assert!(drone["endurance_min"].as_f64().unwrap() > 10.0);

        let chassis_dmg: Value = serde_json::from_str(
            &evaluate_robot_chassis_damage_json(6, 2, "BATTERY_PACK", 4, 8, 2, true, false)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(chassis_dmg["emp_reboot_ticks"], 40);

        let codesign: Value = serde_json::from_str(
            &evaluate_human_ai_codesign_json(
                "SPARSE_MOE",
                2,
                5,
                14,
                0,
                0,
                vec![(14, 14), (15, 13)],
                (6, 7),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(codesign["verification_debt_added"].as_i64().unwrap() > 0);

        let audit: Value = serde_json::from_str(
            &evaluate_verification_debt_audit_json(
                4,
                "NEURO_SYMBOLIC",
                3,
                15,
                0,
                vec![(16, 15), (15, 14), (14, 12)],
                (5, 6),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(audit["debt_scrubbed"].as_i64().unwrap() > 0);

        let cyber: Value = serde_json::from_str(
            &evaluate_cyber_augmentation_loadout_json(
                vec!["MYOMER_ARM".to_string(), "SUBDERMAL_ARMOR".to_string()],
                1,
                true,
                true,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(cyber["subdermal_pr_ballistic"], 2);

        // Ch. 24 Electronic Warfare
        let scada: Value = serde_json::from_str(
            &evaluate_network_intrusion_json(
                "AIR_GAPPED_SCADA",
                "ENTRY",
                false,
                15,
                0,
                0,
                vec![(18, 17)],
                (4, 5),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(scada["access_blocked_by_air_gap"], true);

        let optical: Value = serde_json::from_str(
            &evaluate_ecm_vs_eccm_json(
                "BROADBAND_BARRAGE",
                "OPTICAL_FIBER_TETHER",
                40.0,
                5.0,
                15,
                0,
                vec![(18, 17)],
                (5, 5),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(optical["blocked_by_optical_or_fiber_immunity"], true);

        let ew_hw: Value = serde_json::from_str(
            &evaluate_custom_ew_hardware_json(
                r#"{"role":"JAMMER_POD","requested_ew_power":6,"workshop":"FIELD_BENCH"}"#,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(ew_hw["clamped_by_workshop"], true);
    }
}
