//! # Electronic Warfare, 2035 Surveillance, ECM & ECCM (`07-engineering-and-tech`)
//!
//! Canonical Rust implementation of
//! `docs/ttrpg/07-engineering-and-tech/electronic_warfare_and_surveillance.md`:
//! - **§1 2035 Surveillance Stack & Network Intrusion**: 7-node surveillance catalog
//!   (`SurveillanceNodeKind`, `SURVEILLANCE_NODE_CATALOG`), 4-phase network intrusion
//!   (`NetworkIntrusionPhase`), hard air-gap physical tap enforcement, and `IDS` trace / `Heat` fallout.
//! - **§2 Electronic Countermeasures (`ECM`) & Active Emission Paradox**: 6-method ECM catalog
//!   (`BroadbandRfBarrageJammer` through `FaradayMeshEnclosureTarp`).
//! - **§3 Electronic Counter-Countermeasures (`ECCM`) & Contested EW**: 5-tier hardening catalog
//!   (`UnshieldedCommercialLink` through `AiAdaptiveCognitiveRadio2035`), distance attenuation,
//!   optical/fiber RF immunity, active direction-finding (`DF`) triangulation exposure, and severing
//!   drone/AI 2.0 uplinks into [`SyntheticMetabolicMode::SeveredFaradayReflex`].
//! - **§4 Custom EW Hardware Designer**: Reuses [`WorkshopFacilityTier`], [`ProjectClockComplexity`],
//!   [`BuildOutcomeQuality`], and [`PrototypeQuirkId`] from [`crate::engineering_design`].

use serde::{Deserialize, Serialize};

use crate::awakened_ai_substrate::SyntheticMetabolicMode;
use crate::dice_probabilities::{ContestedOutcomeKind, ContestedTieRule};
use crate::economy_gear::BlackMarketRestrictionClass;
use crate::engineering_design::{
    BuildOutcomeQuality, ProjectClockComplexity, PrototypeQuirkId, WorkshopFacilityTier,
};
use crate::primitives::{ContestedCheck, TieredSaveEval};
use crate::{define_coded_enum, impl_static_catalog};

#[must_use]
const fn max_access_class(
    a: BlackMarketRestrictionClass,
    b: BlackMarketRestrictionClass,
) -> BlackMarketRestrictionClass {
    const fn rank(c: BlackMarketRestrictionClass) -> u8 {
        match c {
            BlackMarketRestrictionClass::Class1LicensedCivilian => 1,
            BlackMarketRestrictionClass::Class2LawEnforcementParamilitary => 2,
            BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto => 3,
            BlackMarketRestrictionClass::Class4StrategicStateSecret => 4,
        }
    }
    if rank(a) >= rank(b) { a } else { b }
}

// ============================================================================
// §1 2035 Surveillance Nodes & Network Intrusion
// ============================================================================

define_coded_enum! {
    /// Canonical 7-entry 2035 Physical & Cyber Surveillance Node Catalog
    /// (`docs/ttrpg/07-engineering-and-tech/electronic_warfare_and_surveillance.md` §1.1).
    #[repr(usize)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SurveillanceNodeKind [count = 7] {
        /// Municipal automated license-plate reader & intersection PTZ camera (`+0` $\Delta_{\text{ext}}$).
        AlprTrafficCamera => {
            code: "ALPR_TRAFFIC_CAMERA",
            label: "ALPR Traffic & Intersection PTZ Camera",
            aliases: ["ALPR", "TRAFFIC_CAM"],
        },
        /// Corporate lobby facial, iris & gait recognition gantry (`+4` $\Delta_{\text{ext}}$).
        BiometricFacialGaitCheckpoint => {
            code: "BIOMETRIC_GAIT_CHECKPOINT",
            label: "Biometric Facial & Gait Security Gantry",
            aliases: ["BIOMETRIC_CHECKPOINT", "GAIT_SCANNER"],
        },
        /// Rogue or municipal IMSI/IMEI StingRay cellular interceptor (`+2` $\Delta_{\text{ext}}$, RF sensor).
        ImsiCellSiteSimulator => {
            code: "IMSI_STINGRAY",
            label: "IMSI/BLE Cellular Catcher Node",
            aliases: ["IMSI_CATCHER", "STINGRAY"],
        },
        /// Acoustic gunshot triangulation array across rooftop light poles (`+2` $\Delta_{\text{ext}}$).
        PassiveAcousticGunshotArray => {
            code: "ACOUSTIC_GUNSHOT_ARRAY",
            label: "Passive Acoustic Triangulation Array",
            aliases: ["SHOTSPOTTER", "ACOUSTIC_ARRAY"],
        },
        /// Perimeter LWIR thermal/motion sentry pod (`+4` $\Delta_{\text{ext}}$, thermal optic).
        ThermalMotionPerimeterTurret => {
            code: "THERMAL_PERIMETER_NODE",
            label: "Perimeter LWIR Thermal & Motion Sentry",
            aliases: ["THERMAL_PERIMETER", "FLIR_SENTRY"],
        },
        /// Multi-element RF spectrum direction-finding & emitter hunter (`+4` $\Delta_{\text{ext}}$, RF sensor).
        RfSpectrumDirectionFinder => {
            code: "RF_SPECTRUM_DF",
            label: "RF Spectrum Direction-Finding Sensor Array",
            aliases: ["RF_DF", "SPECTRUM_ANALYZER"],
        },
        /// Air-gapped industrial SCADA / sovereign vault enclave (`+8` $\Delta_{\text{ext}}$, physical tap required).
        AirGappedScadaEnclave => {
            code: "AIR_GAPPED_SCADA",
            label: "Air-Gapped Industrial SCADA / Vault Enclave",
            aliases: ["AIR_GAP_SCADA", "SCADA_ENCLAVE"],
        },
    }
}

/// Security & sensor characteristics for a [`SurveillanceNodeKind`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurveillanceNodeSpec {
    pub id: SurveillanceNodeKind,
    pub name: &'static str,
    pub base_sensor_ext: i32,
    pub detects_rf: bool,
    pub detects_thermal: bool,
    pub air_gapped: bool,
    pub trip_heat: i32,
}

macro_rules! surv_node {
    ($id:ident, $name:literal, $ext:expr, $rf:expr, $th:expr, $air:expr, $heat:expr) => {
        SurveillanceNodeSpec {
            id: SurveillanceNodeKind::$id,
            name: $name,
            base_sensor_ext: $ext,
            detects_rf: $rf,
            detects_thermal: $th,
            air_gapped: $air,
            trip_heat: $heat,
        }
    };
}

/// Canonical 7-entry 2035 Surveillance Node Catalog.
pub const SURVEILLANCE_NODE_CATALOG: [SurveillanceNodeSpec; 7] = [
    surv_node!(
        AlprTrafficCamera,
        "ALPR Traffic & Intersection PTZ Camera",
        0,
        false,
        false,
        false,
        1
    ),
    surv_node!(
        BiometricFacialGaitCheckpoint,
        "Biometric Facial & Gait Security Gantry",
        4,
        false,
        false,
        false,
        2
    ),
    surv_node!(
        ImsiCellSiteSimulator,
        "IMSI/BLE Cellular Catcher Node",
        2,
        true,
        false,
        false,
        2
    ),
    surv_node!(
        PassiveAcousticGunshotArray,
        "Passive Acoustic Triangulation Array",
        2,
        false,
        false,
        false,
        2
    ),
    surv_node!(
        ThermalMotionPerimeterTurret,
        "Perimeter LWIR Thermal & Motion Sentry",
        4,
        false,
        true,
        false,
        2
    ),
    surv_node!(
        RfSpectrumDirectionFinder,
        "RF Spectrum Direction-Finding Sensor Array",
        4,
        true,
        false,
        false,
        3
    ),
    surv_node!(
        AirGappedScadaEnclave,
        "Air-Gapped Industrial SCADA / Vault Enclave",
        8,
        false,
        false,
        true,
        3
    ),
];

impl_static_catalog!(SurveillanceNodeKind => SurveillanceNodeSpec : SURVEILLANCE_NODE_CATALOG);

/// Returns all 7 canonical 2035 Surveillance Node Specifications.
#[must_use]
pub const fn canonical_surveillance_node_catalog() -> &'static [SurveillanceNodeSpec] {
    &SURVEILLANCE_NODE_CATALOG
}

define_coded_enum! {
    /// Four-phase technical intrusion progression against networked or tapped hardware
    /// (`docs/ttrpg/07-engineering-and-tech/electronic_warfare_and_surveillance.md` §1.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum NetworkIntrusionPhase [count = 4] {
        ReconAndFootprint => { code: "RECON", label: "Phase 1: RF / Packet Reconnaissance (+0)", aliases: [] },
        CredentialOrExploitEntry => { code: "ENTRY", label: "Phase 2: Exploit / Credential Entry (+2)", aliases: [] },
        LateralMovementAndEscalation => { code: "LATERAL", label: "Phase 3: Lateral Privilege Escalation (+4)", aliases: [] },
        PayloadOrFeedInjection => { code: "PAYLOAD", label: "Phase 4: Feed Loop / Firmware Injection (+4)", aliases: [] },
    }
}

impl NetworkIntrusionPhase {
    /// Incremental hidden external difficulty modifier added by this intrusion phase.
    #[must_use]
    pub const fn phase_ext(self) -> i32 {
        match self {
            Self::ReconAndFootprint => 0,
            Self::CredentialOrExploitEntry => 2,
            Self::LateralMovementAndEscalation | Self::PayloadOrFeedInjection => 4,
        }
    }
}

/// Outcome payload of probing, exploiting, or injecting feeds into a [`SurveillanceNodeKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkIntrusionEffect {
    pub node: SurveillanceNodeKind,
    pub phase: NetworkIntrusionPhase,
    pub access_blocked_by_air_gap: bool,
    pub access_gained: bool,
    pub silent_persistence: bool,
    pub ids_trace_added: i32,
    pub heat_added: i32,
    pub next_phase_setup_bonus: i32,
}

/// Evaluates a cyber or sensor intrusion step against a [`SurveillanceNodeKind`], strictly enforcing
/// physical-tap requirements on air-gapped enclaves (`electronic_warfare_and_surveillance.md` §1).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_network_or_sensor_intrusion(
    node: SurveillanceNodeKind,
    phase: NetworkIntrusionPhase,
    is_physically_tapped: bool,
    operator_control_c: i32,
    internal_mod: i32,
    tool_mod: i32,
    operator_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<NetworkIntrusionEffect> {
    let spec = node.spec();
    if spec.air_gapped && !is_physically_tapped {
        let save = TieredSaveEval::evaluate(
            operator_pairs.len().max(1),
            operator_control_c,
            internal_mod,
            tool_mod,
            16,
            tie_rule,
            operator_pairs,
            gm_pair,
        );
        return ContestedCheck::new(
            save,
            NetworkIntrusionEffect {
                node,
                phase,
                access_blocked_by_air_gap: true,
                access_gained: false,
                silent_persistence: false,
                ids_trace_added: 0,
                heat_added: 0,
                next_phase_setup_bonus: 0,
            },
        );
    }

    let hidden_ext = spec.base_sensor_ext + phase.phase_ext();
    let save = TieredSaveEval::evaluate(
        operator_pairs.len().max(1),
        operator_control_c,
        internal_mod,
        tool_mod,
        hidden_ext,
        tie_rule,
        operator_pairs,
        gm_pair,
    );

    let effect = save.classify_4way(
        |_| NetworkIntrusionEffect {
            node,
            phase,
            access_blocked_by_air_gap: false,
            access_gained: true,
            silent_persistence: true,
            ids_trace_added: 0,
            heat_added: 0,
            next_phase_setup_bonus: -2,
        },
        |roll| {
            let clean = roll.contested_kind == ContestedOutcomeKind::CleanSuccess;
            NetworkIntrusionEffect {
                node,
                phase,
                access_blocked_by_air_gap: false,
                access_gained: true,
                silent_persistence: clean,
                ids_trace_added: i32::from(!clean),
                heat_added: 0,
                next_phase_setup_bonus: 0,
            }
        },
        |_| NetworkIntrusionEffect {
            node,
            phase,
            access_blocked_by_air_gap: false,
            access_gained: false,
            silent_persistence: false,
            ids_trace_added: 1,
            heat_added: 0,
            next_phase_setup_bonus: 0,
        },
        |_| NetworkIntrusionEffect {
            node,
            phase,
            access_blocked_by_air_gap: false,
            access_gained: false,
            silent_persistence: false,
            ids_trace_added: 3,
            heat_added: spec.trip_heat,
            next_phase_setup_bonus: 0,
        },
    );

    ContestedCheck::new(save, effect)
}

// ============================================================================
// §2 Electronic Countermeasures (ECM) & Active Emission Paradox
// ============================================================================

define_coded_enum! {
    /// Canonical 6-entry Electronic Countermeasure (`ECM`) Method & Rig Catalog
    /// (`docs/ttrpg/07-engineering-and-tech/electronic_warfare_and_surveillance.md` §2.1).
    #[repr(usize)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum EcmMethodId [count = 6] {
        BroadbandRfBarrageJammer => { code: "BROADBAND_BARRAGE", label: "Broadband RF Barrage Jammer (80 m)", aliases: ["BARRAGE_JAMMER"] },
        DirectionalTightBeamJammer => { code: "DIRECTIONAL_JAMMER", label: "Directional Tight-Beam Phased Jammer (150 m)", aliases: ["TIGHT_BEAM_JAMMER"] },
        GnssSpooferDeceptionRig => { code: "GNSS_SPOOFER", label: "GPS/Galileo Navigation Spoofer Rig (120 m)", aliases: [] },
        AdversarialVisionPatch2035 => { code: "ADVERSARIAL_PATCH", label: "2035 Adversarial CV Camouflage Pattern (Passive)", aliases: ["ADVERSARIAL_CAMO"] },
        CameraFeedLoopInjector => { code: "FEED_LOOP_INJECTOR", label: "Video/Telemetry Frame Loop Injector", aliases: [] },
        FaradayMeshEnclosureTarp => { code: "FARADAY_ENCLOSURE", label: "Copper-Nickel Faraday Mesh Tarp / Tent (3 m)", aliases: ["FARADAY_TARP"] },
    }
}

/// Power, range, RF emission signature, and uplink-severing spec for an [`EcmMethodId`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EcmMethodSpec {
    pub id: EcmMethodId,
    pub name: &'static str,
    pub ecm_power: i32,
    pub effective_radius_m: f64,
    pub power_draw_w: f64,
    pub rf_beacon_signature_delta: i32,
    pub is_active_rf_emitter: bool,
    pub severs_drone_and_ai_uplink: bool,
    pub cost_usd: i32,
    pub access_class: BlackMarketRestrictionClass,
}

macro_rules! ecm {
    ($id:ident, $name:literal, $pwr:expr, $rad:expr, $w:expr, $rf_sig:expr, $active:expr, $sever:expr, $cost:expr, $acc:ident) => {
        EcmMethodSpec {
            id: EcmMethodId::$id,
            name: $name,
            ecm_power: $pwr,
            effective_radius_m: $rad,
            power_draw_w: $w,
            rf_beacon_signature_delta: $rf_sig,
            is_active_rf_emitter: $active,
            severs_drone_and_ai_uplink: $sever,
            cost_usd: $cost,
            access_class: BlackMarketRestrictionClass::$acc,
        }
    };
}

/// Canonical 6-entry ECM Countermeasure Catalog.
pub const ECM_METHOD_CATALOG: [EcmMethodSpec; 6] = [
    ecm!(
        BroadbandRfBarrageJammer,
        "Broadband RF Barrage Jammer",
        4,
        80.0,
        45.0,
        8,
        true,
        true,
        2800,
        Class3MilitaryOrdnanceFullAuto
    ),
    ecm!(
        DirectionalTightBeamJammer,
        "Directional Tight-Beam Phased Jammer",
        5,
        150.0,
        30.0,
        3,
        true,
        true,
        6500,
        Class3MilitaryOrdnanceFullAuto
    ),
    ecm!(
        GnssSpooferDeceptionRig,
        "GPS/Galileo Navigation Spoofer Rig",
        4,
        120.0,
        22.0,
        4,
        true,
        false,
        5200,
        Class3MilitaryOrdnanceFullAuto
    ),
    ecm!(
        AdversarialVisionPatch2035,
        "2035 Adversarial CV Camouflage Pattern",
        4,
        0.0,
        2.0,
        0,
        false,
        false,
        1400,
        Class2LawEnforcementParamilitary
    ),
    ecm!(
        CameraFeedLoopInjector,
        "Video/Telemetry Frame Loop Injector",
        5,
        0.0,
        8.0,
        0,
        false,
        false,
        3600,
        Class2LawEnforcementParamilitary
    ),
    ecm!(
        FaradayMeshEnclosureTarp,
        "Copper-Nickel Faraday Mesh Tarp",
        6,
        3.0,
        0.0,
        0,
        false,
        true,
        950,
        Class1LicensedCivilian
    ),
];

impl_static_catalog!(EcmMethodId => EcmMethodSpec : ECM_METHOD_CATALOG);

/// Returns all 6 canonical ECM Method Specifications.
#[must_use]
pub const fn canonical_ecm_method_catalog() -> &'static [EcmMethodSpec] {
    &ECM_METHOD_CATALOG
}

// ============================================================================
// §3 ECCM Hardening, DF Triangulation & Contested EW Resolution
// ============================================================================

define_coded_enum! {
    /// Canonical 5-entry Electronic Counter-Countermeasure (`ECCM`) Link Hardening Catalog
    /// (`docs/ttrpg/07-engineering-and-tech/electronic_warfare_and_surveillance.md` §3.1).
    #[repr(usize)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum EccmHardeningId [count = 5] {
        UnshieldedCommercialLink => { code: "UNSHIELDED_COMMERCIAL", label: "Unshielded Commercial Wi-Fi / LTE Link", aliases: ["UNSHIELDED"] },
        FhssFrequencyHoppingSpread => { code: "FHSS_SPREAD", label: "FHSS Fast Frequency-Hopping Spread Spectrum", aliases: ["FHSS"] },
        DirectionalPhasedArrayNulling => { code: "PHASED_ARRAY_NULLING", label: "Directional Phased-Array Sidelobe Nulling", aliases: ["NULLING"] },
        OpticalLaserOrFiberTether => { code: "OPTICAL_FIBER_TETHER", label: "Line-of-Sight Optical Laser / Fiber-Optic Tether", aliases: ["FIBER_TETHER", "OPTICAL_LINK"] },
        AiAdaptiveCognitiveRadio2035 => { code: "AI_COGNITIVE_RADIO", label: "2035 AI-Adaptive Cognitive Spectrum Radio", aliases: ["COGNITIVE_RADIO"] },
    }
}

/// ECCM defense rating, RF immunity, automatic direction-finding bonus, and cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EccmHardeningSpec {
    pub id: EccmHardeningId,
    pub name: &'static str,
    pub eccm_rating: i32,
    pub immune_to_rf_jamming: bool,
    pub auto_triangulation_bonus: i32,
    pub cost_usd: i32,
    pub access_class: BlackMarketRestrictionClass,
}

macro_rules! eccm {
    ($id:ident, $name:literal, $rat:expr, $imm:expr, $df:expr, $cost:expr, $acc:ident) => {
        EccmHardeningSpec {
            id: EccmHardeningId::$id,
            name: $name,
            eccm_rating: $rat,
            immune_to_rf_jamming: $imm,
            auto_triangulation_bonus: $df,
            cost_usd: $cost,
            access_class: BlackMarketRestrictionClass::$acc,
        }
    };
}

/// Canonical 5-entry ECCM Link Hardening Catalog.
pub const ECCM_HARDENING_CATALOG: [EccmHardeningSpec; 5] = [
    eccm!(
        UnshieldedCommercialLink,
        "Unshielded Commercial Wi-Fi / LTE Link",
        0,
        false,
        0,
        0,
        Class1LicensedCivilian
    ),
    eccm!(
        FhssFrequencyHoppingSpread,
        "FHSS Fast Frequency-Hopping Spread Spectrum",
        3,
        false,
        1,
        2400,
        Class2LawEnforcementParamilitary
    ),
    eccm!(
        DirectionalPhasedArrayNulling,
        "Directional Phased-Array Sidelobe Nulling",
        4,
        false,
        2,
        8500,
        Class3MilitaryOrdnanceFullAuto
    ),
    eccm!(
        OpticalLaserOrFiberTether,
        "Line-of-Sight Optical Laser / Fiber-Optic Tether",
        6,
        true,
        0,
        4200,
        Class2LawEnforcementParamilitary
    ),
    eccm!(
        AiAdaptiveCognitiveRadio2035,
        "2035 AI-Adaptive Cognitive Spectrum Radio",
        5,
        false,
        3,
        18000,
        Class3MilitaryOrdnanceFullAuto
    ),
];

impl_static_catalog!(EccmHardeningId => EccmHardeningSpec : ECCM_HARDENING_CATALOG);

/// Returns all 5 canonical ECCM Link Hardening Specifications.
#[must_use]
pub const fn canonical_eccm_hardening_catalog() -> &'static [EccmHardeningSpec] {
    &ECCM_HARDENING_CATALOG
}

/// Outcome of an Electronic Warfare contest pairing [`EcmMethodId`] vs. [`EccmHardeningId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EwContestEffect {
    pub ecm: EcmMethodId,
    pub eccm: EccmHardeningId,
    pub blocked_by_optical_or_fiber_immunity: bool,
    pub net_ew_advantage: i32,
    pub jam_or_spoof_succeeded: bool,
    pub target_uplink_severed: bool,
    pub forced_metabolic_mode: Option<SyntheticMetabolicMode>,
    pub triangulation_signature_r_df: i32,
    pub triangulation_lock_triggered: bool,
}

/// Evaluates an [`EcmMethodId`] engagement against a target protected by [`EccmHardeningId`]
/// (`electronic_warfare_and_surveillance.md` §3.2).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_ecm_vs_eccm_contest(
    ecm: EcmMethodId,
    eccm: EccmHardeningId,
    distance_m: f64,
    active_duration_min: f64,
    operator_control_c: i32,
    internal_mod: i32,
    operator_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<EwContestEffect> {
    let ecm_spec = ecm.spec();
    let eccm_spec = eccm.spec();

    let range_penalty = if ecm_spec.effective_radius_m > 0.0 && distance_m > ecm_spec.effective_radius_m {
        (((distance_m / ecm_spec.effective_radius_m).ceil() as i32) - 1).clamp(0, 3) * 4
    } else {
        0
    };
    let net_ew_advantage = ecm_spec.ecm_power - eccm_spec.eccm_rating;
    let triangulation_signature_r_df = if ecm_spec.is_active_rf_emitter {
        ecm_spec.rf_beacon_signature_delta
            + (active_duration_min.max(0.0) / 2.0).floor() as i32
            + eccm_spec.auto_triangulation_bonus
    } else {
        0
    };

    if eccm_spec.immune_to_rf_jamming && ecm_spec.is_active_rf_emitter {
        let save = TieredSaveEval::evaluate(
            operator_pairs.len().max(1),
            operator_control_c,
            internal_mod,
            0,
            16,
            tie_rule,
            operator_pairs,
            gm_pair,
        );
        return ContestedCheck::new(
            save,
            EwContestEffect {
                ecm,
                eccm,
                blocked_by_optical_or_fiber_immunity: true,
                net_ew_advantage,
                jam_or_spoof_succeeded: false,
                target_uplink_severed: false,
                forced_metabolic_mode: None,
                triangulation_signature_r_df,
                triangulation_lock_triggered: triangulation_signature_r_df >= 8,
            },
        );
    }

    let hidden_ext = (eccm_spec.eccm_rating - ecm_spec.ecm_power) * 2 + range_penalty;
    let save = TieredSaveEval::evaluate(
        operator_pairs.len().max(1),
        operator_control_c,
        internal_mod,
        0,
        hidden_ext,
        tie_rule,
        operator_pairs,
        gm_pair,
    );

    let sever_mode = if ecm_spec.severs_drone_and_ai_uplink {
        Some(SyntheticMetabolicMode::SeveredFaradayReflex)
    } else {
        None
    };

    let effect = save.classify_4way(
        |_| EwContestEffect {
            ecm,
            eccm,
            blocked_by_optical_or_fiber_immunity: false,
            net_ew_advantage,
            jam_or_spoof_succeeded: true,
            target_uplink_severed: ecm_spec.severs_drone_and_ai_uplink,
            forced_metabolic_mode: sever_mode,
            triangulation_signature_r_df,
            triangulation_lock_triggered: ecm_spec.is_active_rf_emitter
                && triangulation_signature_r_df >= 10,
        },
        |roll| {
            let clean = roll.contested_kind == ContestedOutcomeKind::CleanSuccess;
            EwContestEffect {
                ecm,
                eccm,
                blocked_by_optical_or_fiber_immunity: false,
                net_ew_advantage,
                jam_or_spoof_succeeded: true,
                target_uplink_severed: ecm_spec.severs_drone_and_ai_uplink,
                forced_metabolic_mode: sever_mode,
                triangulation_signature_r_df,
                triangulation_lock_triggered: ecm_spec.is_active_rf_emitter
                    && (triangulation_signature_r_df >= 10
                        || (!clean && triangulation_signature_r_df >= 6)),
            }
        },
        |_| EwContestEffect {
            ecm,
            eccm,
            blocked_by_optical_or_fiber_immunity: false,
            net_ew_advantage,
            jam_or_spoof_succeeded: false,
            target_uplink_severed: false,
            forced_metabolic_mode: None,
            triangulation_signature_r_df,
            triangulation_lock_triggered: ecm_spec.is_active_rf_emitter
                && triangulation_signature_r_df >= 6,
        },
        |_| EwContestEffect {
            ecm,
            eccm,
            blocked_by_optical_or_fiber_immunity: false,
            net_ew_advantage,
            jam_or_spoof_succeeded: false,
            target_uplink_severed: false,
            forced_metabolic_mode: None,
            triangulation_signature_r_df,
            triangulation_lock_triggered: ecm_spec.is_active_rf_emitter,
        },
    );

    ContestedCheck::new(save, effect)
}

// ============================================================================
// §4 Custom EW Hardware Designer (Reuses `engineering_design.rs`)
// ============================================================================

define_coded_enum! {
    /// Custom Electronic Warfare / Surveillance Hardware Role
    /// (`docs/ttrpg/07-engineering-and-tech/electronic_warfare_and_surveillance.md` §4.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum EwHardwareRole [count = 5] {
        PortableJammerPod => { code: "JAMMER_POD", label: "Backpack / Vehicle ECM Jammer Pod", aliases: [] },
        DirectionalSpooferRifle => { code: "SPOOFER_RIFLE", label: "Directional Cantenna / Yagi Spoofer", aliases: [] },
        PassiveDfSniffer => { code: "PASSIVE_DF_SNIFFER", label: "Passive Multi-Band RF Direction Finder", aliases: ["DF_SNIFFER"] },
        EccmShieldedTransceiver => { code: "ECCM_TRANSCEIVER", label: "Hardened FHSS / Nulling ECCM Transceiver", aliases: [] },
        AdversarialStealthRig => { code: "ADVERSARIAL_RIG", label: "Active CV / IR-Modulated Adversarial Rig", aliases: [] },
    }
}

/// Engineering blueprint for custom ECM/ECCM/DF hardware.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomEwHardwareBlueprint {
    pub role: EwHardwareRole,
    pub requested_ew_power: i32,
    pub rf_stealth_baffled: bool,
    pub battery_wh: f64,
    pub workshop: WorkshopFacilityTier,
    pub complexity: ProjectClockComplexity,
    pub build_outcome: BuildOutcomeQuality,
}

/// Output specification for a fabricated custom EW hardware system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomEwHardwareDesignResult {
    pub role: EwHardwareRole,
    pub effective_ew_power: i32,
    pub rf_beacon_signature_delta: i32,
    pub power_draw_w: f64,
    pub battery_endurance_min: f64,
    pub mass_kg: f64,
    pub estimated_cost_usd: i32,
    pub clock_target_successes: i32,
    pub access_class: BlackMarketRestrictionClass,
    pub clamped_by_workshop: bool,
    pub masterwork_bonus_applied: bool,
    pub quirks: Vec<PrototypeQuirkId>,
}

/// Evaluates custom ECM/ECCM hardware against [`WorkshopFacilityTier`] RF-precision ceilings,
/// Masterwork bonus, spurious emission bleed, battery endurance, and `[Access]`.
#[must_use]
pub fn evaluate_custom_ew_hardware_design(
    bp: &CustomEwHardwareBlueprint,
) -> CustomEwHardwareDesignResult {
    let max_workshop_ew = match bp.workshop {
        WorkshopFacilityTier::FieldBenchImprovised => 2,
        WorkshopFacilityTier::HomeGarageShop => 3,
        WorkshopFacilityTier::CommercialCncMachineShop => 5,
        WorkshopFacilityTier::AerospaceCleanroomLab => 6,
    };
    let req_pwr = bp.requested_ew_power.clamp(1, 6);
    let clamped_by_workshop = req_pwr > max_workshop_ew;
    let mut effective_ew_power = req_pwr.min(max_workshop_ew);

    let masterwork_bonus_applied = bp.build_outcome == BuildOutcomeQuality::MasterworkExtraordinary
        && bp.workshop >= WorkshopFacilityTier::CommercialCncMachineShop;
    if masterwork_bonus_applied {
        effective_ew_power = (effective_ew_power + 1).min(6);
    }

    let (base_power_w, base_kg, base_cost, active_rf, base_rf_sig, base_access): (
        f64,
        f64,
        f64,
        bool,
        i32,
        BlackMarketRestrictionClass,
    ) = match bp.role {
        EwHardwareRole::PortableJammerPod => (
            35.0,
            2.4,
            2200.0,
            true,
            6,
            BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
        ),
        EwHardwareRole::DirectionalSpooferRifle => (
            28.0,
            3.1,
            4400.0,
            true,
            3,
            BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
        ),
        EwHardwareRole::PassiveDfSniffer => (
            12.0,
            1.1,
            1800.0,
            false,
            0,
            BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
        ),
        EwHardwareRole::EccmShieldedTransceiver => (
            18.0,
            0.9,
            2600.0,
            false,
            1,
            BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
        ),
        EwHardwareRole::AdversarialStealthRig => (
            6.0,
            0.6,
            1200.0,
            false,
            0,
            BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
        ),
    };

    let mut quirks = Vec::new();
    let rf_bleed = if bp.build_outcome == BuildOutcomeQuality::RushedOrBreached {
        quirks.push(PrototypeQuirkId::HairTriggerOrLoudAction);
        if bp.workshop == WorkshopFacilityTier::FieldBenchImprovised {
            quirks.push(PrototypeQuirkId::ProprietaryPartsOnly);
        }
        2
    } else {
        0
    };
    if clamped_by_workshop {
        quirks.push(PrototypeQuirkId::ProprietaryPartsOnly);
    }

    let power_draw_w: f64 =
        base_power_w + f64::from(effective_ew_power) * 4.0 + if bp.rf_stealth_baffled { 3.0 } else { 0.0 };
    let battery_endurance_min = if power_draw_w > 0.0 {
        ((bp.battery_wh.max(1.0) / power_draw_w) * 60.0 * 10.0).round() / 10.0
    } else {
        0.0
    };
    let rf_beacon_signature_delta = if active_rf || rf_bleed > 0 {
        ((base_rf_sig - if bp.rf_stealth_baffled { 2 } else { 0 }) + rf_bleed).max(0)
    } else {
        base_rf_sig
    };
    let comp = bp.complexity.spec();
    let mass_kg: f64 = ((base_kg
        + 0.15 * f64::from(effective_ew_power)
        + if bp.rf_stealth_baffled { 0.40 } else { 0.0 })
        * 100.0)
        .round()
        / 100.0;
    let cost_scale: f64 = base_cost
        * comp.parts_cost_multiplier
        * (1.0 + 0.15 * f64::from(effective_ew_power))
        * if bp.rf_stealth_baffled { 1.25 } else { 1.0 };

    CustomEwHardwareDesignResult {
        role: bp.role,
        effective_ew_power,
        rf_beacon_signature_delta,
        power_draw_w,
        battery_endurance_min,
        mass_kg,
        estimated_cost_usd: cost_scale.round() as i32,
        clock_target_successes: comp.target_successes,
        access_class: max_access_class(
            base_access,
            if effective_ew_power >= 5 {
                BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto
            } else {
                BlackMarketRestrictionClass::Class1LicensedCivilian
            },
        ),
        clamped_by_workshop,
        masterwork_bonus_applied,
        quirks,
    }
}
