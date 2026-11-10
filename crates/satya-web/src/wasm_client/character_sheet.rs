//! Interactive Character Sheet & Anatomical/Physiological/Psyche Status Tracker state and
//! `#[wasm_bindgen]` handlers for the `#notebook-pane-character` tab.
//!
//! HTML rendering is modularized into:
//! - [`crate::wasm_client::character_sheet_views`]: Top summary, all 30 Characteristics, 9 Domain Aptitudes,
//!   10 Derived Metrics, Tactical Pool Envelope, Innate Talents Table, Trained Skills Table, and Equipment Loadout.
//! - [`crate::wasm_client::character_sheet_status_views`]: Human-shaped Anatomical Hit Location SVG diagram,
//!   13-zone physical status editor, Damage/Fatigue/Psyche trackers, and writable Operator Field Notes.

use std::collections::BTreeMap;

pub use crate::wasm_client::character_sheet_status_views::*;
pub use crate::wasm_client::character_sheet_vault::*;
pub use crate::wasm_client::character_sheet_views::*;
use crate::wasm_client::chat_and_ttrpg::{append_system_notice, satya_update_cascade_preview};
use crate::wasm_client::dom::{
    get_element, input_value, roll_browser_die_i32, select_value, set_input_value, set_text,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use satya_engine::{
    AcuteMentalTraumaTier, AmmoCaliberId, AmmunitionProfile, AptitudeCode, ArmorProfileId,
    BarrelMountingType, CarriedLoadItem, CarryErgonomicMethod, CenterOfGravityProfile,
    CharacterAptitudes, CharacterCharacteristics, CharacteristicCode, ConditionImpact,
    ConditionSource, FirearmActionType, GranularFirearmSpec, HemorrhageGrade, HitLocationId,
    MeleeWeaponId, MoraleState, MuzzleDeviceType, PhysiologicalStatusCondition, SkillAccessTier,
    SocialWoundKind, StructuralWoundTier, TacticalPoolEnvelope, TacticalPoolParams, Triad,
    WeaponFeedType, accumulate_exertion_marks, build_amara_okafor_lifepath,
    build_mateo_vance_lifepath, build_priya_nair_lifepath, compute_lifepath_baseline_from_config,
    compute_tactical_pool, evaluate_acute_mental_trauma, evaluate_blood_loss_progression,
    evaluate_encumbrance_load, evaluate_fatigue_level, evaluate_physiological_status_conditions,
    initialize_universal_talents, resolve_hit_location_d20,
};
use wasm_bindgen::prelude::*;

/// Fracture or joint integrity state for an anatomical hit location (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoneFractureStatus {
    #[default]
    Intact,
    HairlineFracture,
    DislocatedJoint,
    CompleteFracture,
    CompoundShattered,
}

impl BoneFractureStatus {
    pub const ALL: [Self; 5] = [
        Self::Intact,
        Self::HairlineFracture,
        Self::DislocatedJoint,
        Self::CompleteFracture,
        Self::CompoundShattered,
    ];

    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Intact => "INTACT",
            Self::HairlineFracture => "HAIRLINE",
            Self::DislocatedJoint => "DISLOCATED",
            Self::CompleteFracture => "FRACTURED",
            Self::CompoundShattered => "SHATTERED",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Intact => "Intact",
            Self::HairlineFracture => "Hairline Fracture (Risk T3 under >50% Cap)",
            Self::DislocatedJoint => "Dislocated Joint (-10 Cap until reduced)",
            Self::CompleteFracture => "Broken / Complete Fracture (Limb Disabled)",
            Self::CompoundShattered => "Compound / Shattered (Cap Locked 0)",
        }
    }

    #[must_use]
    pub const fn short_badge(self) -> &'static str {
        match self {
            Self::Intact => "Intact",
            Self::HairlineFracture => "Hairline Fx",
            Self::DislocatedJoint => "Dislocated",
            Self::CompleteFracture => "Broken Bone",
            Self::CompoundShattered => "Shattered",
        }
    }

    #[must_use]
    pub fn from_code(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "HAIRLINE" => Self::HairlineFracture,
            "DISLOCATED" => Self::DislocatedJoint,
            "FRACTURED" | "BROKEN" => Self::CompleteFracture,
            "SHATTERED" | "COMPOUND" => Self::CompoundShattered,
            _ => Self::Intact,
        }
    }

    #[must_use]
    pub const fn extra_cap_penalty(self) -> i32 {
        match self {
            Self::Intact => 0,
            Self::HairlineFracture => 2,
            Self::DislocatedJoint | Self::CompleteFracture => 10,
            Self::CompoundShattered => 20,
        }
    }
}

/// Per-zone physical trauma state for one of the 13 canonical [`HitLocationId`] regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnatomicalZoneState {
    pub location: HitLocationId,
    /// Structural wound severity (`0` = Uninjured, `1..=5` = `Tier 1` .. `Tier 5`).
    pub wound_tier: u8,
    pub bone_status: BoneFractureStatus,
    pub bleed_grade: HemorrhageGrade,
}

impl AnatomicalZoneState {
    #[must_use]
    pub const fn new_healthy(location: HitLocationId) -> Self {
        Self {
            location,
            wound_tier: 0,
            bone_status: BoneFractureStatus::Intact,
            bleed_grade: HemorrhageGrade::Grade0None,
        }
    }

    #[must_use]
    pub const fn is_injured(&self) -> bool {
        self.wound_tier > 0
            || !matches!(self.bone_status, BoneFractureStatus::Intact)
            || !matches!(self.bleed_grade, HemorrhageGrade::Grade0None)
    }

    #[must_use]
    pub const fn effective_cap_penalty(&self) -> i32 {
        let wound_pen = if self.wound_tier == 0 {
            0
        } else {
            StructuralWoundTier::from_tier_number(self.wound_tier).aspect_ceiling_penalty()
        };
        let bone_pen = self.bone_status.extra_cap_penalty();
        if wound_pen > bone_pen {
            wound_pen
        } else {
            bone_pen
        }
    }
}

/// Carried field equipment item on the Character Sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldGearItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub mass_kg: f64,
    pub carry_method: CarryErgonomicMethod,
    pub tool_mod: i32,
    pub notes: String,
}

macro_rules! firearm_preset {
    (
        $id:literal, $name:literal,
        ($cal:ident, $aname:literal, $mm:expr, $b_gr:expr, $p_gr:expr, $opt_cm:expr, $e_j:expr, $dec_m:expr, $exp:expr, $sd:expr, $aq:expr, $spitz:expr),
        ($feed:ident, $cap:expr, $act:ident, $u_kg:expr, $l_kg:expr, $oal_cm:expr, $bbl_cm:expr, $bq:expr, $mnt:ident, $muz:ident, $cog:ident, $tau:expr, $mq:expr, $j_min:expr, $j_maj:expr, $strp:expr, $cost:expr)
    ) => {
        (
            $id,
            GranularFirearmSpec {
                name: $name,
                ammo: AmmunitionProfile {
                    caliber_id: AmmoCaliberId::$cal,
                    name: $aname,
                    caliber_mm: $mm,
                    bullet_mass_grains: $b_gr,
                    powder_charge_grains: $p_gr,
                    optimal_barrel_cm: $opt_cm,
                    reference_muzzle_energy_j: $e_j,
                    energy_decay_scale_m: $dec_m,
                    expansion_factor: $exp,
                    sectional_density_pen_bonus: $sd,
                    ammo_quality_q: $aq,
                    is_spitzer_early_yaw: $spitz,
                },
                feed_type: WeaponFeedType::$feed,
                capacity_rounds: $cap,
                action_type: FirearmActionType::$act,
                unloaded_weight_kg: $u_kg,
                loaded_weight_kg: $l_kg,
                overall_length_cm: $oal_cm,
                barrel_length_cm: $bbl_cm,
                barrel_quality_q: $bq,
                barrel_mounting: BarrelMountingType::$mnt,
                muzzle_device: MuzzleDeviceType::$muz,
                center_of_gravity: CenterOfGravityProfile::$cog,
                tolerance_tau: $tau,
                manufacturing_quality_q: $mq,
                clear_jam_minor_ticks: $j_min,
                clear_jam_major_ticks: $j_maj,
                strip_time_ticks: $strp,
                cost_usd: $cost,
            },
        )
    };
}

/// Canonical preset ranged firearms for the Character Sheet Equipment section.
pub const CANONICAL_FIREARM_PRESETS: &[(&str, GranularFirearmSpec)] = &[
    firearm_preset!(
        "hk416_carbine_556",
        "HK416A5 Tactical Carbine (5.56x45mm)",
        (
            Nato556x45,
            "5.56x45mm NATO M855A1",
            5.56,
            62.0,
            27.0,
            50.8,
            1795.0,
            520.0,
            1.15,
            2,
            0.5,
            true
        ),
        (
            DetachableBoxMagazine,
            30,
            ShortStrokeGasPiston,
            3.12,
            3.60,
            80.5,
            36.8,
            1.0,
            FreeFloating,
            FlashHider,
            ChamberNeutral,
            0.5,
            2,
            6,
            18,
            36,
            2800
        )
    ),
    firearm_preset!(
        "sig_p320_9mm",
        "SIG P320 / M18 Sidearm (9x19mm)",
        (
            Parabellum9x19,
            "9x19mm +P JHP (124 gr)",
            9.0,
            124.0,
            5.2,
            12.0,
            515.0,
            180.0,
            1.5,
            0,
            0.5,
            false
        ),
        (
            DetachableBoxMagazine,
            17,
            StrikerFired,
            0.76,
            0.98,
            18.8,
            9.9,
            0.5,
            StandardPistolMount,
            BareCrown,
            ChamberNeutral,
            0.0,
            1,
            5,
            14,
            20,
            750
        )
    ),
    firearm_preset!(
        "mp7_pdw_57",
        "MP7A2 / P90 Suppressed PDW (5.7x28mm)",
        (
            Fn57x28,
            "5.7x28mm SS190 AP",
            5.7,
            40.0,
            7.2,
            26.0,
            540.0,
            260.0,
            1.0,
            3,
            0.5,
            true
        ),
        (
            DetachableBoxMagazine,
            40,
            ShortStrokeGasPiston,
            1.90,
            2.30,
            41.5,
            18.0,
            1.0,
            FreeFloating,
            SoundSuppressor,
            RearBullpup,
            0.5,
            2,
            6,
            16,
            30,
            2200
        )
    ),
    firearm_preset!(
        "akm_762x39",
        "Kalashnikov AKM Assault Rifle (7.62x39mm)",
        (
            Soviet762x39,
            "7.62x39mm M43 FMJ",
            7.62,
            123.0,
            25.5,
            41.5,
            2100.0,
            420.0,
            1.0,
            2,
            0.0,
            false
        ),
        (
            DetachableBoxMagazine,
            30,
            LongStrokeGasPiston,
            3.30,
            4.10,
            88.0,
            41.5,
            0.0,
            ClampedNonFreeFloated,
            BareCrown,
            ModerateForward,
            -1.0,
            1,
            8,
            20,
            32,
            950
        )
    ),
    firearm_preset!(
        "benelli_m4_12ga",
        "Benelli M4 Combat Shotgun (12-Gauge)",
        (
            Gauge12,
            "12-Gauge 00 Buck / Breaching Slug",
            18.5,
            437.0,
            32.0,
            51.0,
            3150.0,
            140.0,
            1.35,
            1,
            0.5,
            false
        ),
        (
            TubularMagazine,
            7,
            ShortStrokeGasPiston,
            3.82,
            4.25,
            88.5,
            47.0,
            0.5,
            ClampedNonFreeFloated,
            BareCrown,
            ModerateForward,
            0.0,
            2,
            7,
            18,
            40,
            1950
        )
    ),
    firearm_preset!(
        "ai_atx_65cm",
        "Accuracy Int'l AT-X Precision Rifle (6.5mm CM)",
        (
            Creedmoor65,
            "6.5mm Creedmoor 140gr Match OTM",
            6.72,
            140.0,
            42.5,
            56.0,
            3165.0,
            950.0,
            1.4,
            3,
            1.0,
            false
        ),
        (
            DetachableBoxMagazine,
            10,
            BoltAction,
            5.40,
            5.80,
            112.0,
            61.0,
            1.5,
            FreeFloating,
            MuzzleBrakeCompensator,
            ChamberNeutral,
            1.0,
            2,
            6,
            20,
            80,
            5200
        )
    ),
];

/// Canonical preset field gear items (`(id, name, category, mass_kg, carry_method, tool_mod, notes)`).
pub const CANONICAL_FIELD_GEAR_PRESETS: &[(
    &str,
    &str,
    &str,
    f64,
    CarryErgonomicMethod,
    i32,
    &str,
)] = &[
    (
        "trauma_kit_pro",
        "Trauma & Vascular Surgery Field Kit",
        "Medical",
        3.2,
        CarryErgonomicMethod::BalancedWebbingOrRucksack,
        -1,
        "Class B surgical kit · Δ_tool -1 · Stops Grade 3–4 arterial bleeds",
    ),
    (
        "ifak_cat_tourniquet",
        "IFAK + 2x CAT Tourniquets & Hemostatic Gauze",
        "Medical",
        0.8,
        CarryErgonomicMethod::TailoredArmorOrDutyBelt,
        0,
        "20–24 Ticks field stabilization · Morphine auto-injector (-1 Shock)",
    ),
    (
        "comms_tac_mesh",
        "Encrypted Multi-Band Tactical Comms & HUD Link",
        "Comms/Optics",
        0.6,
        CarryErgonomicMethod::TailoredArmorOrDutyBelt,
        -1,
        "Zero-latency squad telemetry · Δ_tool -1 on SIGINT/Coordination",
    ),
    (
        "nvg_thermal_binocs",
        "Multi-Spectral NVG / Thermal Fusion Optics",
        "Comms/Optics",
        0.9,
        CarryErgonomicMethod::TailoredArmorOrDutyBelt,
        -2,
        "Negates darkness Δ_ext · Thermal/IR signature overlay",
    ),
    (
        "cyber_deck_analyzer",
        "Ruggedized Cyber-Deck & RF Logic Analyzer",
        "Technical",
        1.8,
        CarryErgonomicMethod::BalancedWebbingOrRucksack,
        -1,
        "Class B hardware interface · Δ_tool -1 on Cyber/Avionics",
    ),
    (
        "eva_mag_thruster_pack",
        "Zero-G EVA Cold-Gas Thruster & Mag-Tether Rig",
        "Survival/EVA",
        4.5,
        CarryErgonomicMethod::BalancedWebbingOrRucksack,
        -1,
        "Prevents Zero-G Adrift state · 120 m/s delta-v reserve",
    ),
];

/// Complete state for the interactive Character Sheet / Status notebook pane (`#notebook-pane-character`).
#[derive(Debug, Clone)]
pub struct CharacterSheetState {
    pub character_id: String,
    pub preset_id: String,
    pub character_name: String,
    pub callsign_or_role: String,
    pub age_years: u32,
    pub origin_label: String,
    pub campaign_sqm: u8,
    pub operational_heat: u8,
    pub granted_access_tags: Vec<SkillAccessTier>,
    pub stage_history: Vec<String>,
    pub section_filter: String,
    pub skill_tree_filter: String,
    pub skills_trained_only: bool,
    pub characteristics: CharacterCharacteristics,
    pub aptitudes: CharacterAptitudes,
    pub skills: BTreeMap<String, usize>,
    pub equipped_ranged_ids: Vec<String>,
    pub ballistic_range_m: f64,
    pub equipped_melee: Vec<MeleeWeaponId>,
    pub equipped_armor: Vec<ArmorProfileId>,
    pub field_gear: Vec<FieldGearItem>,
    pub hit_zones: [AnatomicalZoneState; 13],
    pub selected_hit_zone: usize,
    pub last_hit_location_roll_d20: Option<i32>,
    pub fatigue_level: i32,
    pub exertion_marks: i32,
    pub wound_shock_index: i32,
    pub blood_units_lost: f64,
    pub active_status_conditions: Vec<PhysiologicalStatusCondition>,
    pub morale_state: MoraleState,
    pub mental_trauma: Option<AcuteMentalTraumaTier>,
    pub social_composure_lost: i32,
    pub social_wound: Option<SocialWoundKind>,
    pub psyche_drift_marks: i32,
    pub conviction_surge_active: bool,
    pub notes: String,
    pub verification_status: satya_schema::CharacterVerificationStatus,
    pub audit_violations: Vec<String>,
    pub local_satya_file_path: Option<String>,
    pub assets: Vec<satya_schema::CharacterAssetRef>,
    pub saved_vault_list: Vec<satya_schema::CharacterSheetRecord>,
    pub vault_feedback: String,
}

impl Default for CharacterSheetState {
    fn default() -> Self {
        Self::from_preset("sara_valkyrie")
    }
}

impl CharacterSheetState {
    /// Builds a canonical operator preset (`"sara_valkyrie"`, `"mateo_vance"`, `"priya_nair"`, or `"amara_okafor"`).
    #[must_use]
    pub fn from_preset(preset_id: &str) -> Self {
        match preset_id {
            "mateo_vance" => Self::from_lifepath_profile(
                "mateo_vance",
                "Syndicate Enforcer & Urban Pointman",
                build_mateo_vance_lifepath(),
                vec!["sig_p320_9mm".to_string(), "akm_762x39".to_string()],
                vec![
                    MeleeWeaponId::CombatKnifeKarambitShiv,
                    MeleeWeaponId::UnarmedJabLowKickClinchKnee,
                ],
                vec![ArmorProfileId::ConcealableSoftKevlarVestNij3a],
                &["ifak_cat_tourniquet", "comms_tac_mesh"],
                "Syndicate safehouse keycard (Sector 9). Watching Dock 4 shipments.\nHigh APT-QNT (+3) ballistic intuition; keep Heat below 4.",
            ),
            "priya_nair" => Self::from_lifepath_profile(
                "priya_nair",
                "Corporate AI & Robotics Lead Scientist",
                build_priya_nair_lifepath(),
                vec!["mp7_pdw_57".to_string()],
                vec![MeleeWeaponId::ExpandableBatonTonfaClub],
                vec![ArmorProfileId::HeavyLeatherSynthWeaveJacket],
                &[
                    "cyber_deck_analyzer",
                    "comms_tac_mesh",
                    "nvg_thermal_binocs",
                ],
                "Holding Level-4 autonomous drone override keys for Sector 4 Airlock.\nPrimary mission: audit telemetry anomalies and secure robotics core.",
            ),
            "amara_okafor" => Self::from_lifepath_profile(
                "amara_okafor",
                "Frontier Recon Tracker & Anti-Poaching Ranger",
                build_amara_okafor_lifepath(),
                vec!["ai_atx_65cm".to_string(), "sig_p320_9mm".to_string()],
                vec![
                    MeleeWeaponId::MacheteWakizashiTacticalBlade,
                    MeleeWeaponId::CombatKnifeKarambitShiv,
                ],
                vec![ArmorProfileId::LightweightUhmwpePatrolCarrierNij3],
                &[
                    "trauma_kit_pro",
                    "ifak_cat_tourniquet",
                    "nvg_thermal_binocs",
                ],
                "Perimeter overwatch established at 150m elevation.\nHerbalism & field triage stocks full; tracking hostile patrol signatures.",
            ),
            _ => {
                let chars = CharacterCharacteristics {
                    spd: 14,
                    for_stat: 13,
                    dex: 14,
                    kin: 14,
                    hlt: 13,
                    fit: 12,
                    hgt_m: 1.78,
                    mas_kg: 72.0,
                    mem: 13,
                    ana: 15,
                    ins: 13,
                    cre: 11,
                    dis: 14,
                    foc: 14,
                    com: 11,
                    pre: 13,
                    elo: 11,
                    sig: 14,
                    hea: 12,
                    tac: 12,
                    olf: 10,
                    tas: 10,
                    itu: 13,
                    ego: 11,
                    slf: 7,
                    opn: 13,
                    blg: 14,
                    ext: 10,
                    cnf: 12,
                    agg: 11,
                };
                let mut aptitudes = CharacterAptitudes::default();
                aptitudes[AptitudeCode::AptBio] = 2;
                aptitudes[AptitudeCode::AptKin] = 2;
                aptitudes[AptitudeCode::AptFin] = 1;
                aptitudes[AptitudeCode::AptMec] = 1;
                let mut skills = initialize_universal_talents();
                for (id, n) in [
                    ("alertness_situational_awareness", 3),
                    ("running_sprinting", 2),
                    ("brawling_unarmed_striking", 2),
                    ("manual_dexterity_tinkering", 2),
                    ("empathy_social_perception", 2),
                    ("reading_comprehension", 2),
                    ("arithmetic", 2),
                    ("first_aid_bls_cpr", 3),
                    ("paramedicine_combat_trauma_tccc", 3),
                    ("trauma_orthopedic_general_surgery", 3),
                    ("zero_g_eva_operations", 4),
                    ("pistols_handguns", 3),
                    ("long_guns_rifles_carbines_shotguns", 3),
                    ("fully_automatic_burst_weapons", 3),
                    ("small_unit_tactics_cqb_doctrine", 4),
                    ("surveillance_reconnaissance_countersurveillance", 2),
                    ("camouflage_concealment", 2),
                ] {
                    skills.insert(id.to_string(), n);
                }
                Self {
                    character_id: "char-sara_valkyrie".to_string(),
                    preset_id: "sara_valkyrie".to_string(),
                    character_name: "Sara \"Valkyrie\" Lin".to_string(),
                    callsign_or_role: "Orbital Med-Assault Operator (Valkyrie Exo-Rig)".to_string(),
                    age_years: 26,
                    origin_label: "Professional / Upper-Middle Class".to_string(),
                    campaign_sqm: 6,
                    operational_heat: 1,
                    granted_access_tags: vec![
                        SkillAccessTier::Universal, SkillAccessTier::Academic,
                        SkillAccessTier::VocationalLicensed, SkillAccessTier::Restricted,
                    ],
                    stage_history: vec![
                        "Stage 4A: STEM Magnet High School -> B.S. Pre-Med & Combat Trauma Residency.".to_string(),
                        "Stage 5: 2 Terms Orbital SpecOps Pararescue & Med-Assault (Zero-G EVA N=4, Trauma Surgery N=3, Small-Unit Tactics N=4).".to_string(),
                    ],
                    section_filter: "ALL".to_string(),
                    skill_tree_filter: "ALL".to_string(),
                    skills_trained_only: true,
                    characteristics: chars,
                    aptitudes,
                    skills,
                    equipped_ranged_ids: vec!["hk416_carbine_556".to_string(), "sig_p320_9mm".to_string()],
                    ballistic_range_m: 25.0,
                    equipped_melee: vec![MeleeWeaponId::CombatKnifeKarambitShiv, MeleeWeaponId::UnarmedJabLowKickClinchKnee],
                    equipped_armor: vec![ArmorProfileId::ValkyrieReconExoCarapace2035, ArmorProfileId::BallisticCombatHelmetVisor],
                    field_gear: Self::gear_from_ids(&["trauma_kit_pro", "ifak_cat_tourniquet", "comms_tac_mesh", "eva_mag_thruster_pack"]),
                    hit_zones: HitLocationId::ALL.map(AnatomicalZoneState::new_healthy),
                    selected_hit_zone: HitLocationId::ChestRibcageLungs as usize,
                    last_hit_location_roll_d20: None,
                    fatigue_level: 1,
                    exertion_marks: 3,
                    wound_shock_index: 0,
                    blood_units_lost: 0.0,
                    active_status_conditions: Vec::new(),
                    morale_state: MoraleState::Steady,
                    mental_trauma: None,
                    social_composure_lost: 0,
                    social_wound: None,
                    psyche_drift_marks: 1,
                    conviction_surge_active: false,
                    notes: "Squad Med-Assault Lead — Sector 4 Orbital Airlock breach team.\n• Valkyrie Exo-Rig battery at 94% (+20.0 kg load offset).\n• Trauma Surgery Kit & 2x CAT tourniquets staged on chest rig.\n• Primary objective: secure decompression valve and extract VIP.".to_string(),
                    verification_status: satya_schema::CharacterVerificationStatus::VerifiedOfficial,
                    audit_violations: Vec::new(),
                    local_satya_file_path: None,
                    assets: Vec::new(),
                    saved_vault_list: Vec::new(),
                    vault_feedback: String::new(),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn from_lifepath_profile(
        preset_id: &str,
        role: &str,
        lp: satya_engine::LifepathCharacterProfile,
        ranged_ids: Vec<String>,
        melee: Vec<MeleeWeaponId>,
        armor: Vec<ArmorProfileId>,
        gear_ids: &[&str],
        notes: &str,
    ) -> Self {
        Self {
            character_id: format!("char-{preset_id}"),
            preset_id: preset_id.to_string(),
            character_name: lp.name,
            callsign_or_role: role.to_string(),
            age_years: lp.age_years,
            origin_label: lp.origin.label.to_string(),
            campaign_sqm: lp.campaign_sqm,
            operational_heat: lp.operational_heat,
            granted_access_tags: lp.granted_access_tags,
            stage_history: lp.stage_history,
            section_filter: "ALL".to_string(),
            skill_tree_filter: "ALL".to_string(),
            skills_trained_only: true,
            characteristics: lp.characteristics,
            aptitudes: lp.aptitudes,
            skills: lp.skills,
            equipped_ranged_ids: ranged_ids,
            ballistic_range_m: 25.0,
            equipped_melee: melee,
            equipped_armor: armor,
            field_gear: Self::gear_from_ids(gear_ids),
            hit_zones: HitLocationId::ALL.map(AnatomicalZoneState::new_healthy),
            selected_hit_zone: HitLocationId::ChestRibcageLungs as usize,
            last_hit_location_roll_d20: None,
            fatigue_level: 0,
            exertion_marks: 0,
            wound_shock_index: 0,
            blood_units_lost: 0.0,
            active_status_conditions: Vec::new(),
            morale_state: MoraleState::Steady,
            mental_trauma: None,
            social_composure_lost: 0,
            social_wound: None,
            psyche_drift_marks: 0,
            conviction_surge_active: false,
            notes: notes.to_string(),
            verification_status: satya_schema::CharacterVerificationStatus::VerifiedOfficial,
            audit_violations: Vec::new(),
            local_satya_file_path: None,
            assets: Vec::new(),
            saved_vault_list: Vec::new(),
            vault_feedback: String::new(),
        }
    }

    pub(crate) fn gear_from_ids(ids: &[&str]) -> Vec<FieldGearItem> {
        ids.iter()
            .filter_map(|&id| {
                CANONICAL_FIELD_GEAR_PRESETS
                    .iter()
                    .find(|(pid, ..)| *pid == id)
                    .map(
                        |&(pid, name, cat, mass_kg, carry_method, tool_mod, notes)| FieldGearItem {
                            id: pid.to_string(),
                            name: name.to_string(),
                            category: cat.to_string(),
                            mass_kg,
                            carry_method,
                            tool_mod,
                            notes: notes.to_string(),
                        },
                    )
            })
            .collect()
    }

    /// Computes localized `(Tempo, Control, Force)` ceiling penalties from all 13 anatomical hit zones.
    #[must_use]
    pub fn compute_anatomical_wound_cap_penalties(&self) -> Triad<i32> {
        let mut pen = Triad::ZERO;
        for z in &self.hit_zones {
            let cap_pen = z.effective_cap_penalty();
            if cap_pen > 0 {
                let spec = z.location.spec();
                if spec.slashes_tempo_cap {
                    pen.tempo = (pen.tempo + cap_pen).min(20);
                }
                if spec.slashes_control_cap {
                    pen.control = (pen.control + cap_pen).min(20);
                }
                if spec.slashes_force_cap {
                    pen.force = (pen.force + cap_pen).min(20);
                }
            }
        }
        pen
    }

    /// Collects all active non-zero [`HemorrhageGrade`] bleeds across the 13 anatomical hit zones.
    #[must_use]
    pub fn active_bleeds(&self) -> Vec<HemorrhageGrade> {
        self.hit_zones
            .iter()
            .filter_map(|z| (z.bleed_grade != HemorrhageGrade::Grade0None).then_some(z.bleed_grade))
            .collect()
    }

    /// Computes systemic wound shock contributed by anatomical wounds and blood loss, combined with `self.wound_shock_index`.
    #[must_use]
    pub fn effective_wound_shock_index(&self) -> i32 {
        let wsr = self
            .characteristics
            .compute_derived(0)
            .wsr_wound_shock_resistance;
        let zone_shock: i32 = self
            .hit_zones
            .iter()
            .filter(|z| z.wound_tier > 0)
            .map(|z| StructuralWoundTier::from_tier_number(z.wound_tier).wound_shock_added(wsr))
            .sum();
        let blood_eval =
            evaluate_blood_loss_progression(self.blood_units_lost, &self.active_bleeds(), 0);
        (self.wound_shock_index.max(zone_shock) + blood_eval.added_wound_shock_index).clamp(0, 5)
    }

    /// Computes effective Fatigue Level (`0..=5`) combining `self.fatigue_level` and hypovolemic blood loss fatigue.
    #[must_use]
    pub fn effective_fatigue_level(&self) -> i32 {
        let blood_eval =
            evaluate_blood_loss_progression(self.blood_units_lost, &self.active_bleeds(), 0);
        (self.fatigue_level + blood_eval.added_fatigue_level).clamp(0, 5)
    }

    /// Computes total carried items and powered exoskeleton offset for `evaluate_encumbrance_load`.
    #[must_use]
    pub fn compute_encumbrance(&self) -> satya_engine::EncumbranceEvaluation {
        let mut items = Vec::new();
        for r_id in &self.equipped_ranged_ids {
            if let Some((_, spec)) = CANONICAL_FIREARM_PRESETS
                .iter()
                .find(|(id, _)| *id == r_id.as_str())
            {
                items.push(CarriedLoadItem {
                    mass_kg: spec.loaded_weight_kg,
                    carry_method: CarryErgonomicMethod::BalancedWebbingOrRucksack,
                });
            }
        }
        for &m_id in &self.equipped_melee {
            let mass_kg = match m_id {
                MeleeWeaponId::UnarmedJabLowKickClinchKnee
                | MeleeWeaponId::UnarmedHaymakerHighKickThrow => 0.0,
                MeleeWeaponId::CombatKnifeKarambitShiv => 0.35,
                MeleeWeaponId::ExpandableBatonTonfaClub => 0.65,
                MeleeWeaponId::MacheteWakizashiTacticalBlade => 0.95,
                MeleeWeaponId::TwoHandedStaffSpearRiotShield => 3.40,
            };
            if mass_kg > 0.0 {
                items.push(CarriedLoadItem {
                    mass_kg,
                    carry_method: CarryErgonomicMethod::TailoredArmorOrDutyBelt,
                });
            }
        }
        let mut exo_offset_kg = 0.0;
        for &a_id in &self.equipped_armor {
            let spec = a_id.spec();
            items.push(CarriedLoadItem {
                mass_kg: spec.weight_kg,
                carry_method: CarryErgonomicMethod::TailoredArmorOrDutyBelt,
            });
            exo_offset_kg += spec.exo_load_offset_kg;
        }
        for g in &self.field_gear {
            items.push(CarriedLoadItem {
                mass_kg: g.mass_kg,
                carry_method: g.carry_method,
            });
        }
        evaluate_encumbrance_load(
            self.characteristics.mas_kg,
            self.characteristics.for_stat,
            &items,
            exo_offset_kg,
        )
    }

    /// Computes aggregated [`ConditionImpact`] from active physiological status conditions, mental trauma, and encumbrance.
    #[must_use]
    pub fn compute_combined_condition_impact(&self) -> ConditionImpact {
        let phys = evaluate_physiological_status_conditions(&self.active_status_conditions)
            .to_condition_impact();
        let mut combined = phys.combine(self.compute_encumbrance().to_condition_impact());
        if let Some(mt) = self.mental_trauma {
            combined = combined.combine(
                evaluate_acute_mental_trauma(mt, false, self.characteristics.agg)
                    .to_condition_impact(),
            );
        }
        if let Some(sw) = self.social_wound {
            combined.delta_int += match sw {
                SocialWoundKind::RhetoricalStrain => 1,
                SocialWoundKind::Doubt | SocialWoundKind::EgoWound => 2,
                SocialWoundKind::CognitiveDissonance => 4,
            };
        }
        if self.conviction_surge_active {
            combined.delta_int = (combined.delta_int - 6).max(-2);
            combined.tactical_pool_mod += 2;
        }
        combined
    }

    /// Computes total internal difficulty modifier $\Delta_{\text{int}}$ affecting `2d20` tasks.
    #[must_use]
    pub fn total_task_delta_int(&self) -> i32 {
        let fat_spec = evaluate_fatigue_level(self.effective_fatigue_level());
        let raw = fat_spec.delta_int
            + (2 * self.effective_wound_shock_index())
            + self.compute_combined_condition_impact().delta_int;
        if self.conviction_surge_active {
            raw.max(-2)
        } else {
            raw
        }
    }

    /// Computes the live [`TacticalPoolEnvelope`] (`P_eff / P_base`, `Tempo / Control / Force` caps) for this operator.
    #[must_use]
    pub fn compute_live_tactical_envelope(&self) -> TacticalPoolEnvelope {
        let c_dex_tac = self
            .characteristics
            .compute_controller(CharacteristicCode::DEX, Some(CharacteristicCode::TAC));
        compute_tactical_pool(&TacticalPoolParams {
            chars: self.characteristics.clone(),
            skill_c: c_dex_tac,
            skill_n: 3,
            is_ranged: true,
            fatigue_level: self.effective_fatigue_level(),
            wound_shock_index: self.effective_wound_shock_index(),
            wound_cap_penalties: self.compute_anatomical_wound_cap_penalties(),
            condition: self.compute_combined_condition_impact(),
            morale_state: self.morale_state,
        })
    }

    /// Formats the status pill text for `#char-sheet-status-pill`.
    #[must_use]
    pub fn format_status_pill_text(&self) -> String {
        let derived = self.characteristics.compute_derived(0);
        let env = self.compute_live_tactical_envelope();
        let (fat, shock) = (
            self.effective_fatigue_level(),
            self.effective_wound_shock_index(),
        );
        let state_tag = if env.caps == Triad::ZERO {
            "INCAPACITATED"
        } else if shock >= 2 || fat >= 3 {
            "IMPAIRED"
        } else {
            "Combat Ready"
        };
        format!(
            "{} (Age {}) · {} · OODA {} · D_cog {} · Pool {}/{} · Fatigue {} · Shock {} · Δ_int {:+}",
            self.character_name,
            self.age_years,
            state_tag,
            derived.ooda_initiative,
            derived.d_cog_quick_thinking,
            env.p_eff,
            env.p_base,
            fat,
            shock,
            self.total_task_delta_int()
        )
    }
}

/// Re-renders `#char-sheet-stage-container` and `#char-sheet-status-pill` from thread-local state.
pub fn render_character_sheet() {
    let notes_dom = input_value("char-sheet-notes-textarea");
    with_state_mut(|st| {
        if get_element("char-sheet-notes-textarea").is_some() {
            st.character_sheet.notes = notes_dom;
        }
    });
    let (html, pill_text, preset_id) = with_state(|st| {
        (
            format_character_sheet_html(&st.character_sheet),
            st.character_sheet.format_status_pill_text(),
            st.character_sheet.preset_id.clone(),
        )
    });
    set_text("char-sheet-status-pill", &pill_text);
    crate::wasm_client::dom::set_select_value("char-sheet-preset-select", &preset_id);
    if let Some(el) = get_element("char-sheet-stage-container") {
        el.set_inner_html(&html);
    }
}

/// Synchronizes the Character Creation Wizard (`st.wizard`) into `st.character_sheet`.
pub fn apply_wizard_to_character_sheet(st: &mut crate::wasm_client::state::SatyaWasmState) {
    st.wizard.sync_lifepath_skills();
    let baseline = compute_lifepath_baseline_from_config(
        &st.wizard.lifepath,
        &st.wizard.characteristics,
        &st.wizard.aptitudes,
    );
    let sheet = &mut st.character_sheet;
    let slug: String = st
        .wizard
        .character_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    sheet.character_id = format!(
        "char-{}",
        if slug.is_empty() {
            "custom_wizard"
        } else {
            &slug
        }
    );
    sheet.preset_id = "custom_wizard".to_string();
    sheet.character_name = st.wizard.character_name.clone();
    sheet.callsign_or_role = format!("Custom Lifepath Operator ({})", baseline.origin.label);
    sheet.age_years = baseline.age_years;
    sheet.origin_label = baseline.origin.label.to_string();
    sheet.campaign_sqm = baseline.campaign_sqm;
    sheet.operational_heat = baseline.operational_heat;
    sheet.granted_access_tags = baseline.granted_access_tags.clone();
    sheet.stage_history = baseline.stage_history.clone();
    sheet.characteristics = st.wizard.characteristics.clone();
    sheet.aptitudes = st.wizard.aptitudes.clone();
    sheet.skills = st.wizard.skills.clone();
    let rec = sheet.to_character_sheet_record("local-operator");
    sheet.verification_status = rec.verification_status;
    sheet.audit_violations = rec.audit_violations;
}

/// Loads a canonical operator preset (`"sara_valkyrie"`, `"mateo_vance"`, `"priya_nair"`, `"amara_okafor"`) into the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_load_preset(preset_id: String) {
    let target = if preset_id.trim().is_empty() {
        select_value("char-sheet-preset-select")
    } else {
        preset_id.trim().to_string()
    };
    if target == "custom_wizard" {
        with_state_mut(apply_wizard_to_character_sheet);
    } else {
        with_state_mut(|st| {
            let prev_filter = st.character_sheet.section_filter.clone();
            let prev_vault = st.character_sheet.saved_vault_list.clone();
            st.character_sheet = CharacterSheetState::from_preset(&target);
            st.character_sheet.section_filter = prev_filter;
            st.character_sheet.saved_vault_list = prev_vault;
        });
    }
    render_character_sheet();
}

/// Switches the visible section filter (`"ALL"`, `"STATS"`, `"SKILLS"`, `"EQUIPMENT"`, `"STATUS"`, `"NOTES"`) on the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_set_section(section: String) {
    with_state_mut(|st| {
        st.character_sheet.section_filter = section.trim().to_ascii_uppercase();
    });
    render_character_sheet();
}

/// Updates the Skill Table tree filter (`"ALL"` or `"TREE_1"`..`"TREE_8"`) and trained-only toggle on the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_set_skill_filter(tree_code: String, trained_only: bool) {
    with_state_mut(|st| {
        st.character_sheet.skill_tree_filter = tree_code.trim().to_ascii_uppercase();
        st.character_sheet.skills_trained_only = trained_only;
    });
    render_character_sheet();
}

/// Selects one of the 13 anatomical hit zones (`0..13`) on the Human-Shaped Hit Location SVG or table.
#[wasm_bindgen]
pub fn satya_char_sheet_select_zone(zone_idx: usize) {
    with_state_mut(|st| {
        if zone_idx < 13 {
            st.character_sheet.selected_hit_zone = zone_idx;
        }
    });
    render_character_sheet();
}

/// Rolls `1d20` on the Master Anatomical Hit Location Table (`HIT_LOCATION_TABLE`) and selects that zone on the human diagram.
#[wasm_bindgen]
pub fn satya_char_sheet_roll_hit_location() {
    let roll = roll_browser_die_i32(20);
    let spec = resolve_hit_location_d20(roll);
    with_state_mut(|st| {
        st.character_sheet.last_hit_location_roll_d20 = Some(roll);
        st.character_sheet.selected_hit_zone = spec.id as usize;
    });
    render_character_sheet();
    append_system_notice(&format!(
        "Rolled 1d20 Hit Location = {} -> {} (Called Shot +{} Δ_ext): {}",
        roll, spec.name, spec.called_shot_delta_ext, spec.mechanical_notes
    ));
}

/// Updates the physical trauma state (`"wound_tier"`, `"bone_status"`, `"bleed_grade"`, `"stabilize_bleeds"`, `"clear_all"`) of an anatomical hit zone.
#[wasm_bindgen]
pub fn satya_char_sheet_update_zone(zone_idx: usize, field: String, value: String) {
    let f = field.trim().to_ascii_lowercase();
    let v = value.trim();
    with_state_mut(|st| {
        if f == "clear_all" {
            st.character_sheet.hit_zones = HitLocationId::ALL.map(AnatomicalZoneState::new_healthy);
            st.character_sheet.wound_shock_index = 0;
            st.character_sheet.blood_units_lost = 0.0;
            return;
        }
        if f == "stabilize_bleeds" {
            for z in &mut st.character_sheet.hit_zones {
                z.bleed_grade = HemorrhageGrade::Grade0None;
            }
            return;
        }
        let Some(zone) = st.character_sheet.hit_zones.get_mut(zone_idx) else {
            return;
        };
        st.character_sheet.selected_hit_zone = zone_idx;
        match f.as_str() {
            "wound_tier" => zone.wound_tier = v.parse::<u8>().unwrap_or(0).min(5),
            "bone_status" => zone.bone_status = BoneFractureStatus::from_code(v),
            "bleed_grade" => {
                zone.bleed_grade = match v.parse::<u8>().unwrap_or(0) {
                    1 => HemorrhageGrade::Grade1Capillary,
                    2 => HemorrhageGrade::Grade2Venous,
                    3 => HemorrhageGrade::Grade3MajorVenousVisceral,
                    4 => HemorrhageGrade::Grade4ArterialSeverance,
                    _ => HemorrhageGrade::Grade0None,
                };
            }
            _ => {}
        }
    });
    render_character_sheet();
}

/// Adjusts or sets Damage, Fatigue, Exertion, Blood Loss, Morale, or Psyche status fields on the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_update_status(field: String, value: String) {
    let f = field.trim().to_ascii_lowercase();
    let v = value.trim();
    with_state_mut(|st| {
        let sheet = &mut st.character_sheet;
        let derived = sheet.characteristics.compute_derived(0);
        match f.as_str() {
            "fatigue_delta" => {
                sheet.fatigue_level =
                    (sheet.fatigue_level + v.parse::<i32>().unwrap_or(0)).clamp(0, 5);
            }
            "exertion_delta" => {
                let d = v.parse::<i32>().unwrap_or(0);
                if d <= -99 {
                    sheet.exertion_marks = 0;
                } else if d > 0 {
                    let prog = accumulate_exertion_marks(
                        sheet.exertion_marks,
                        d,
                        sheet.fatigue_level,
                        derived.ft_fatigue_threshold,
                    );
                    sheet.exertion_marks = prog.remaining_exertion_marks;
                    sheet.fatigue_level = prog.new_fatigue_level.clamp(0, 5);
                } else {
                    sheet.exertion_marks = (sheet.exertion_marks + d).max(0);
                }
            }
            "wound_shock_delta" => {
                sheet.wound_shock_index =
                    (sheet.wound_shock_index + v.parse::<i32>().unwrap_or(0)).clamp(0, 5);
            }
            "blood_loss_delta" => {
                sheet.blood_units_lost =
                    (sheet.blood_units_lost + v.parse::<f64>().unwrap_or(0.0)).clamp(0.0, 10.0);
            }
            "advance_bleed_ticks" => {
                let ticks = v.parse::<i32>().unwrap_or(20).max(1);
                let out = evaluate_blood_loss_progression(
                    sheet.blood_units_lost,
                    &sheet.active_bleeds(),
                    ticks,
                );
                sheet.blood_units_lost = out.total_units_lost;
            }
            "morale_state" => {
                if let Some(m) = MoraleState::from_code_str(v) {
                    sheet.morale_state = m;
                }
            }
            "mental_trauma" => {
                sheet.mental_trauma = match v {
                    "TIER_1" => Some(AcuteMentalTraumaTier::Tier1AcuteMentalShockOodaFreeze),
                    "TIER_2" => Some(AcuteMentalTraumaTier::Tier2PanicFightOrFlight),
                    "TIER_3" => Some(AcuteMentalTraumaTier::Tier3DissociationShellShock),
                    "TIER_4" => Some(AcuteMentalTraumaTier::Tier4Catatonia),
                    _ => None,
                };
            }
            "social_wound" => {
                sheet.social_wound = match v {
                    "STRAIN" => Some(SocialWoundKind::RhetoricalStrain),
                    "DOUBT" => Some(SocialWoundKind::Doubt),
                    "DISSONANCE" => Some(SocialWoundKind::CognitiveDissonance),
                    "EGO_WOUND" => Some(SocialWoundKind::EgoWound),
                    _ => None,
                };
            }
            "composure_delta" => {
                sheet.social_composure_lost = (sheet.social_composure_lost
                    + v.parse::<i32>().unwrap_or(0))
                .clamp(0, derived.cmp_social_composure_threshold);
            }
            "drift_delta" => {
                sheet.psyche_drift_marks =
                    (sheet.psyche_drift_marks + v.parse::<i32>().unwrap_or(0)).clamp(0, 10);
            }
            "toggle_surge" => sheet.conviction_surge_active = !sheet.conviction_surge_active,
            "toggle_condition" => {
                let cond_opt = match v {
                    "DAZZLED" => Some(PhysiologicalStatusCondition::DazzledBlurredVision),
                    "BLINDED" => Some(PhysiologicalStatusCondition::Blinded),
                    "TINNITUS" => Some(PhysiologicalStatusCondition::Tinnitus),
                    "DEAFENED" => Some(PhysiologicalStatusCondition::Deafened),
                    "STUNNED" => Some(PhysiologicalStatusCondition::StunnedDazed),
                    "CONCUSSED" => Some(PhysiologicalStatusCondition::Concussed),
                    "DIZZY" => Some(PhysiologicalStatusCondition::DizzyVertigo),
                    "NAUSEA" => Some(PhysiologicalStatusCondition::NauseaSickened),
                    "PINNED" => Some(PhysiologicalStatusCondition::PinnedImmobilized),
                    _ => None,
                };
                if let Some(cond) = cond_opt {
                    if let Some(pos) = sheet
                        .active_status_conditions
                        .iter()
                        .position(|&c| c == cond)
                    {
                        sheet.active_status_conditions.remove(pos);
                    } else {
                        sheet.active_status_conditions.push(cond);
                    }
                }
            }
            "sync_to_roller" => {
                let delta_int = sheet.total_task_delta_int();
                let shock = sheet.effective_wound_shock_index();
                set_input_value("ttrpg-internal-mod", &delta_int.to_string());
                set_input_value("ttrpg-wound-shock", &shock.to_string());
                satya_update_cascade_preview();
            }
            _ => {}
        }
    });
    render_character_sheet();
}

/// Modifies equipped Ranged Firearms, Melee Weapons, Armor Profiles, Field Gear, or Ballistic Range on the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_modify_equipment(action: String, item_id: String) {
    let act = action.trim().to_ascii_lowercase();
    let id = item_id.trim();
    with_state_mut(|st| {
        let sheet = &mut st.character_sheet;
        match act.as_str() {
            "set_range" => {
                sheet.ballistic_range_m = id.parse::<f64>().unwrap_or(25.0).clamp(5.0, 1000.0)
            }
            "add_ranged" => {
                let target = if id.is_empty() {
                    select_value("char-add-ranged-select")
                } else {
                    id.to_string()
                };
                if CANONICAL_FIREARM_PRESETS
                    .iter()
                    .any(|(pid, _)| *pid == target.as_str())
                    && !sheet.equipped_ranged_ids.contains(&target)
                {
                    sheet.equipped_ranged_ids.push(target);
                }
            }
            "remove_ranged" => sheet.equipped_ranged_ids.retain(|x| x != id),
            "add_melee" => {
                let target = if id.is_empty() {
                    select_value("char-add-melee-select")
                } else {
                    id.to_string()
                };
                if let Ok(idx) = target.parse::<usize>()
                    && let Some(&mw) = MeleeWeaponId::ALL.get(idx)
                    && !sheet.equipped_melee.contains(&mw)
                {
                    sheet.equipped_melee.push(mw);
                }
            }
            "remove_melee" => {
                if let Ok(idx) = id.parse::<usize>()
                    && let Some(&mw) = MeleeWeaponId::ALL.get(idx)
                {
                    sheet.equipped_melee.retain(|&x| x != mw);
                }
            }
            "add_armor" => {
                let target = if id.is_empty() {
                    select_value("char-add-armor-select")
                } else {
                    id.to_string()
                };
                if let Ok(idx) = target.parse::<usize>()
                    && let Some(&ar) = ArmorProfileId::ALL.get(idx)
                    && !sheet.equipped_armor.contains(&ar)
                {
                    sheet.equipped_armor.push(ar);
                }
            }
            "remove_armor" => {
                if let Ok(idx) = id.parse::<usize>()
                    && let Some(&ar) = ArmorProfileId::ALL.get(idx)
                {
                    sheet.equipped_armor.retain(|&x| x != ar);
                }
            }
            "add_gear" => {
                let target = if id.is_empty() {
                    select_value("char-add-gear-select")
                } else {
                    id.to_string()
                };
                if !sheet.field_gear.iter().any(|g| g.id == target) {
                    sheet
                        .field_gear
                        .extend(CharacterSheetState::gear_from_ids(&[&target]));
                }
            }
            "remove_gear" => sheet.field_gear.retain(|g| g.id != id),
            _ => {}
        }
    });
    render_character_sheet();
}

/// Saves or updates the writable Operator Field Notes (`#char-sheet-notes-textarea`) in [`CharacterSheetState`].
#[wasm_bindgen]
pub fn satya_char_sheet_save_notes(mode: String) {
    let current = input_value("char-sheet-notes-textarea");
    with_state_mut(|st| {
        if mode == "clear" {
            st.character_sheet.notes.clear();
        } else if mode == "stamp_status" {
            let pill = st.character_sheet.format_status_pill_text();
            st.character_sheet.notes = if current.trim().is_empty() {
                format!("[Status Log] {pill}")
            } else {
                format!("{}\n[Status Log] {pill}", current.trim_end())
            };
        } else {
            st.character_sheet.notes = current;
        }
    });
    if mode == "clear" || mode == "stamp_status" {
        render_character_sheet();
    }
}
