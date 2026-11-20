//! # Engineering, Workshops & Custom Weapon/Armor Design (`07-engineering-and-tech`)
//!
//! Canonical Rust implementation of
//! `docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md`:
//! - **§1 Workshops, Project Clocks & Prototype Quirks**: `WorkshopFacilityTier`,
//!   `ProjectClockComplexity`, and `PrototypeQuirkId`.
//! - **§2 Custom Firearm & Ammunition Designer**: `BulletConstructionKind`,
//!   `build_custom_ammunition_profile`, and `build_custom_firearm_design` wrapping
//!   [`crate::ballistics::evaluate_granular_firearm_ballistics`] with zero duplicate physics math.
//! - **§3 Custom Melee Weapon & Body Armor Designer**: `evaluate_custom_melee_design`
//!   and `evaluate_custom_armor_design`.

use serde::{Deserialize, Serialize};

use crate::ballistics::{
    AmmoCaliberId, AmmunitionProfile, BarrelMountingType, CenterOfGravityProfile,
    FirearmActionType, GranularBallisticsEvaluation, GranularFirearmSpec, MuzzleDeviceType,
    WeaponFeedType, evaluate_granular_firearm_ballistics,
};
use crate::economy_gear::BlackMarketRestrictionClass;
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
// §1 Workshops, Project Clocks & Prototype Quirks
// ============================================================================

define_coded_enum! {
    /// Workshop or Manufacturing Facility Tier governing machining tolerance ceiling ($\tau_{\max}$),
    /// manufacturing quality ceiling ($Q_{\text{mfg},\max}$), and $\Delta_{\text{tool}}$ modifier
    /// (`docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md` §1.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum WorkshopFacilityTier [count = 4] {
        /// Improvised vice, rotary tool, and hand files (`+4` $\Delta_{\text{tool}}$, $\tau_{\max}=-0.5$, $Q_{\text{mfg},\max}=-1$).
        FieldBenchImprovised => {
            code: "FIELD_BENCH",
            label: "Field Bench / Improvised Hand Tools",
            aliases: ["FIELD_BENCH_IMPROVISED", "IMPROVISED"],
        },
        /// Home garage lathe, mill, TIG welder, and 3D printer (`0` $\Delta_{\text{tool}}$, $\tau_{\max}=0.0$, $Q_{\text{mfg},\max}=0$).
        HomeGarageShop => {
            code: "HOME_GARAGE",
            label: "Home Garage Machine Shop",
            aliases: ["HOME_GARAGE_SHOP", "GARAGE"],
        },
        /// Commercial 5-axis CNC, EDM, heat-treat kiln, and clean bench (`-2` $\Delta_{\text{tool}}$, $\tau_{\max}=1.0$, $Q_{\text{mfg},\max}=1$).
        CommercialCncMachineShop => {
            code: "COMMERCIAL_CNC",
            label: "Commercial 5-Axis CNC Machine Shop",
            aliases: ["COMMERCIAL_CNC_MACHINE_SHOP", "CNC_SHOP"],
        },
        /// Aerospace cleanroom, metrology CMM, DMLS metal printer, and RF chamber (`-4` $\Delta_{\text{tool}}$, $\tau_{\max}=1.5$, $Q_{\text{mfg},\max}=2$).
        AerospaceCleanroomLab => {
            code: "AEROSPACE_LAB",
            label: "Aerospace Cleanroom & Metrology Lab",
            aliases: ["AEROSPACE_CLEANROOM_LAB", "CLEANROOM"],
        },
    }
}

/// Complete capability specification for a [`WorkshopFacilityTier`]
/// (`docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md` §1.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkshopFacilityProfile {
    pub id: WorkshopFacilityTier,
    pub name: &'static str,
    pub tool_delta_mod: i32,
    pub max_tolerance_tau: f64,
    pub max_barrel_quality_q: f64,
    pub max_mfg_quality_q: i32,
    pub setup_cost_usd: i32,
}

macro_rules! wf {
    ($id:ident, $name:literal, $tool:expr, $tau:expr, $bbl:expr, $mfg:expr, $cost:expr) => {
        WorkshopFacilityProfile {
            id: WorkshopFacilityTier::$id,
            name: $name,
            tool_delta_mod: $tool,
            max_tolerance_tau: $tau,
            max_barrel_quality_q: $bbl,
            max_mfg_quality_q: $mfg,
            setup_cost_usd: $cost,
        }
    };
}

/// Canonical 4-tier Workshop & Facility catalog.
pub const WORKSHOP_FACILITY_CATALOG: [WorkshopFacilityProfile; 4] = [
    wf!(
        FieldBenchImprovised,
        "Field Bench / Improvised Hand Tools",
        4,
        -0.5,
        -1.0,
        -1,
        400
    ),
    wf!(
        HomeGarageShop,
        "Home Garage Machine Shop",
        0,
        0.0,
        0.0,
        0,
        4500
    ),
    wf!(
        CommercialCncMachineShop,
        "Commercial 5-Axis CNC Machine Shop",
        -2,
        1.0,
        1.0,
        1,
        45000
    ),
    wf!(
        AerospaceCleanroomLab,
        "Aerospace Cleanroom & Metrology Lab",
        -4,
        1.5,
        1.5,
        2,
        450000
    ),
];

impl_static_catalog!(WorkshopFacilityTier => WorkshopFacilityProfile : WORKSHOP_FACILITY_CATALOG);

/// Returns all 4 canonical Workshop Facility Tiers.
#[must_use]
pub const fn canonical_workshop_facility_catalog() -> &'static [WorkshopFacilityProfile] {
    &WORKSHOP_FACILITY_CATALOG
}

define_coded_enum! {
    /// Engineering Extended Project Clock Complexity Tier
    /// (`docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md` §1.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ProjectClockComplexity [count = 4] {
        /// `4` net successes, `2 h` intervals (`1.0×` parts cost): trigger job, glass bedding, rail swap.
        FieldModification => {
            code: "FIELD_MODIFICATION",
            label: "Field Modification / Conversion (4 Successes)",
            aliases: ["MODIFICATION", "MINOR_MOD"],
        },
        /// `8` net successes, `8 h` intervals (`1.35×` parts cost): custom upper, suppressor, armor carrier.
        CustomSubassembly => {
            code: "CUSTOM_SUBASSEMBLY",
            label: "Custom Subassembly / Upper (8 Successes)",
            aliases: ["SUBASSEMBLY"],
        },
        /// `14` net successes, `24 h` intervals (`2.0×` parts cost): ground-up custom firearm or drone.
        FullCustomPrototype => {
            code: "FULL_PROTOTYPE",
            label: "Full Custom Prototype (14 Successes)",
            aliases: ["FULL_CUSTOM_PROTOTYPE", "PROTOTYPE"],
        },
        /// `22` net successes, `72 h` intervals (`3.5×` parts cost): frontier ballistic/neural/stealth architecture.
        NextGenBreakthrough => {
            code: "NEXT_GEN_BREAKTHROUGH",
            label: "Next-Gen R&D Breakthrough (22 Successes)",
            aliases: ["BREAKTHROUGH", "RND"],
        },
    }
}

/// Clock cadence & cost specification for a [`ProjectClockComplexity`] tier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectClockComplexitySpec {
    pub id: ProjectClockComplexity,
    pub name: &'static str,
    pub target_successes: i32,
    pub interval_hours: i32,
    pub parts_cost_multiplier: f64,
    pub required_skill_n_min: i32,
}

pub const PROJECT_COMPLEXITY_CATALOG: [ProjectClockComplexitySpec; 4] = [
    ProjectClockComplexitySpec {
        id: ProjectClockComplexity::FieldModification,
        name: "Field Modification / Conversion",
        target_successes: 4,
        interval_hours: 2,
        parts_cost_multiplier: 1.0,
        required_skill_n_min: 1,
    },
    ProjectClockComplexitySpec {
        id: ProjectClockComplexity::CustomSubassembly,
        name: "Custom Subassembly / Upper",
        target_successes: 8,
        interval_hours: 8,
        parts_cost_multiplier: 1.35,
        required_skill_n_min: 2,
    },
    ProjectClockComplexitySpec {
        id: ProjectClockComplexity::FullCustomPrototype,
        name: "Full Custom Prototype",
        target_successes: 14,
        interval_hours: 24,
        parts_cost_multiplier: 2.0,
        required_skill_n_min: 3,
    },
    ProjectClockComplexitySpec {
        id: ProjectClockComplexity::NextGenBreakthrough,
        name: "Next-Gen R&D Breakthrough",
        target_successes: 22,
        interval_hours: 72,
        parts_cost_multiplier: 3.5,
        required_skill_n_min: 4,
    },
];

impl_static_catalog!(ProjectClockComplexity => ProjectClockComplexitySpec : PROJECT_COMPLEXITY_CATALOG);

/// Returns all 4 canonical Project Clock Complexity Tiers.
#[must_use]
pub const fn canonical_project_complexity_catalog() -> &'static [ProjectClockComplexitySpec] {
    &PROJECT_COMPLEXITY_CATALOG
}

define_coded_enum! {
    /// Hardware prototype quirk acquired from workshop clamping, rushed tolerances, or Floor Breaches
    /// (`docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md` §1.3).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum PrototypeQuirkId [count = 5] {
        /// Chamber requires tuned handloads or match cases; non-match ammo suffers `-1` `Rel`.
        FinickyAmmoTolerance => {
            code: "FINICKY_AMMO",
            label: "Finicky Chamber / Ammo Tolerance (-1 Rel on Bulk Ammo)",
            aliases: ["FINICKY_AMMO_TOLERANCE"],
        },
        /// Rapid fire (>10 rds within 60s) walks POI: `-1` `Acc` until cooled.
        ThermalDriftBarrel => {
            code: "THERMAL_DRIFT",
            label: "Thermal Drift / Harmonics Shift (-1 Acc After Rapid Fire)",
            aliases: ["THERMAL_DRIFT_BARREL"],
        },
        /// Hand-fitted bespoke fasteners; field spare sourcing suffers `+4` $\Delta_{\text{ext}}$.
        ProprietaryPartsOnly => {
            code: "PROPRIETARY_PARTS",
            label: "Proprietary Hand-Fitted Parts (+4 Delta_ext Field Repair)",
            aliases: ["PROPRIETARY_PARTS_ONLY"],
        },
        /// Extra barrel/chassis reinforcement adds `+0.6 kg` mass and `+1` Snap Shot `TC`.
        OverweightForwardBalance => {
            code: "OVERWEIGHT_FORWARD",
            label: "Overweight Forward Balance (+0.6 kg, +1 Snap TC)",
            aliases: ["OVERWEIGHT_FORWARD_BALANCE"],
        },
        /// Lightened sear / unbaffled port adds `+2` `HEA` signature and accidental-discharge risk on Floor Breach.
        HairTriggerOrLoudAction => {
            code: "HAIR_TRIGGER_LOUD",
            label: "Hair-Trigger / Harsh Report (+2 HEA Acoustic Signature)",
            aliases: ["HAIR_TRIGGER_OR_LOUD_ACTION"],
        },
    }
}

/// Completed fabrication outcome tier governing Masterwork bonuses vs. Prototype Quirks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuildOutcomeQuality {
    /// Zero Floor Breaches and at least one `Extraordinary` interval on `CommercialCncMachineShop+` (`+1 Q_mfg`).
    MasterworkExtraordinary,
    /// Clean completion with zero lingering Floor Breaches.
    #[default]
    CleanStandard,
    /// Completed via `PartialSuccess`, `GrazingTie`, or rushed past a Floor Breach (`+Quirk`).
    RushedOrBreached,
}

// ============================================================================
// §2 Custom Firearm & Ammunition Designer (Wraps `ballistics.rs`)
// ============================================================================

define_coded_enum! {
    /// Custom Bullet & Propellant Load Construction (`docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md` §2.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum BulletConstructionKind [count = 6] {
        /// Standard copper-jacketed FMJ ball (`1.0×` expansion, `0` Pen mod, `0.0` $Q_{\text{ammo}}$).
        #[default]
        FmjBall => {
            code: "FMJ_BALL",
            label: "FMJ Full Metal Jacket",
            aliases: ["FMJ"],
        },
        /// Controlled-expansion Jacketed Hollow Point (`1.45×` expansion, `-1` SD Pen, `+0.25` $Q_{\text{ammo}}$).
        HollowPointJhp => {
            code: "HOLLOW_POINT_JHP",
            label: "Jacketed Hollow Point (JHP)",
            aliases: ["JHP"],
        },
        /// Boat-tail Open-Tip Match precision handload (`1.20×` expansion, `+1.0` $Q_{\text{ammo}}$, `1.02×` energy).
        MatchOtm => {
            code: "MATCH_OTM",
            label: "Match Open-Tip BTHP (OTM)",
            aliases: ["OTM", "MATCH"],
        },
        /// Hardened tungsten/steel-core Armor Piercing (`0.85×` expansion, `+2` SD Pen, `+0.5` $Q_{\text{ammo}}$).
        ArmorPiercingAp => {
            code: "ARMOR_PIERCING_AP",
            label: "Hardened Core Armor-Piercing (AP)",
            aliases: ["AP"],
        },
        /// Heavy-for-caliber subsonic charge (`1.10×` expansion, `-1` SD Pen, supersonic->subsonic energy scaling).
        SubsonicHeavy => {
            code: "SUBSONIC_HEAVY",
            label: "Subsonic Heavy Suppressor Load",
            aliases: ["SUBSONIC"],
        },
        /// Hot overpressure `+P` load (`1.15×` expansion, `+1` SD Pen, `1.15×` energy, `-1` chamber wear `Rel`).
        OverpressurePlusP => {
            code: "OVERPRESSURE_PLUS_P",
            label: "Overpressure +P Hot Handload",
            aliases: ["PLUS_P", "OVERPRESSURE"],
        },
    }
}

/// Canonical baseline cartridge physics for all 17 [`AmmoCaliberId`] calibers from `ballistics.rs`.
#[must_use]
pub const fn base_caliber_profile(caliber: AmmoCaliberId) -> AmmunitionProfile {
    match caliber {
        AmmoCaliberId::Rimfire22Lr => AmmunitionProfile {
            caliber_id: caliber,
            name: ".22 LR Rimfire",
            caliber_mm: 5.6,
            bullet_mass_grains: 40.0,
            powder_charge_grains: 1.8,
            optimal_barrel_cm: 40.0,
            reference_muzzle_energy_j: 180.0,
            energy_decay_scale_m: 130.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 0,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
        AmmoCaliberId::Acp380 => AmmunitionProfile {
            caliber_id: caliber,
            name: ".380 ACP (9x17mm)",
            caliber_mm: 9.0,
            bullet_mass_grains: 95.0,
            powder_charge_grains: 3.2,
            optimal_barrel_cm: 9.5,
            reference_muzzle_energy_j: 275.0,
            energy_decay_scale_m: 140.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 0,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
        AmmoCaliberId::Parabellum9x19 => AmmunitionProfile {
            caliber_id: caliber,
            name: "9x19mm Parabellum",
            caliber_mm: 9.0,
            bullet_mass_grains: 124.0,
            powder_charge_grains: 5.2,
            optimal_barrel_cm: 12.0,
            reference_muzzle_energy_j: 515.0,
            energy_decay_scale_m: 180.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 0,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
        AmmoCaliberId::Acp45 => AmmunitionProfile {
            caliber_id: caliber,
            name: ".45 ACP",
            caliber_mm: 11.43,
            bullet_mass_grains: 230.0,
            powder_charge_grains: 5.6,
            optimal_barrel_cm: 12.7,
            reference_muzzle_energy_j: 505.0,
            energy_decay_scale_m: 170.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 0,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
        AmmoCaliberId::Auto10mm => AmmunitionProfile {
            caliber_id: caliber,
            name: "10mm Auto",
            caliber_mm: 10.0,
            bullet_mass_grains: 180.0,
            powder_charge_grains: 9.2,
            optimal_barrel_cm: 12.7,
            reference_muzzle_energy_j: 880.0,
            energy_decay_scale_m: 210.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 1,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
        AmmoCaliberId::Hk46x30 => AmmunitionProfile {
            caliber_id: caliber,
            name: "4.6x30mm HK",
            caliber_mm: 4.6,
            bullet_mass_grains: 31.0,
            powder_charge_grains: 6.8,
            optimal_barrel_cm: 18.0,
            reference_muzzle_energy_j: 520.0,
            energy_decay_scale_m: 240.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 2,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: true,
        },
        AmmoCaliberId::Fn57x28 => AmmunitionProfile {
            caliber_id: caliber,
            name: "5.7x28mm FN",
            caliber_mm: 5.7,
            bullet_mass_grains: 40.0,
            powder_charge_grains: 7.2,
            optimal_barrel_cm: 26.0,
            reference_muzzle_energy_j: 540.0,
            energy_decay_scale_m: 260.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 2,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: true,
        },
        AmmoCaliberId::Magnum357 => AmmunitionProfile {
            caliber_id: caliber,
            name: ".357 Magnum",
            caliber_mm: 9.1,
            bullet_mass_grains: 158.0,
            powder_charge_grains: 14.5,
            optimal_barrel_cm: 15.0,
            reference_muzzle_energy_j: 980.0,
            energy_decay_scale_m: 230.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 1,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
        AmmoCaliberId::Blackout300Subsonic => AmmunitionProfile {
            caliber_id: caliber,
            name: ".300 AAC Blackout Subsonic",
            caliber_mm: 7.62,
            bullet_mass_grains: 220.0,
            powder_charge_grains: 10.5,
            optimal_barrel_cm: 22.9,
            reference_muzzle_energy_j: 680.0,
            energy_decay_scale_m: 320.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 2,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
        AmmoCaliberId::Blackout300Supersonic => AmmunitionProfile {
            caliber_id: caliber,
            name: ".300 AAC Blackout Supersonic",
            caliber_mm: 7.62,
            bullet_mass_grains: 110.0,
            powder_charge_grains: 19.5,
            optimal_barrel_cm: 22.9,
            reference_muzzle_energy_j: 1840.0,
            energy_decay_scale_m: 460.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 2,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: true,
        },
        AmmoCaliberId::Nato556x45 => AmmunitionProfile {
            caliber_id: caliber,
            name: "5.56x45mm NATO",
            caliber_mm: 5.56,
            bullet_mass_grains: 62.0,
            powder_charge_grains: 27.0,
            optimal_barrel_cm: 50.8,
            reference_muzzle_energy_j: 1795.0,
            energy_decay_scale_m: 520.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 2,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: true,
        },
        AmmoCaliberId::Soviet762x39 => AmmunitionProfile {
            caliber_id: caliber,
            name: "7.62x39mm",
            caliber_mm: 7.62,
            bullet_mass_grains: 123.0,
            powder_charge_grains: 25.5,
            optimal_barrel_cm: 41.5,
            reference_muzzle_energy_j: 2100.0,
            energy_decay_scale_m: 420.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 2,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
        AmmoCaliberId::Creedmoor65 => AmmunitionProfile {
            caliber_id: caliber,
            name: "6.5mm Creedmoor",
            caliber_mm: 6.72,
            bullet_mass_grains: 140.0,
            powder_charge_grains: 42.5,
            optimal_barrel_cm: 61.0,
            reference_muzzle_energy_j: 3165.0,
            energy_decay_scale_m: 950.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 3,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: true,
        },
        AmmoCaliberId::Nato762x51 => AmmunitionProfile {
            caliber_id: caliber,
            name: "7.62x51mm NATO",
            caliber_mm: 7.62,
            bullet_mass_grains: 147.0,
            powder_charge_grains: 45.0,
            optimal_barrel_cm: 56.0,
            reference_muzzle_energy_j: 3350.0,
            energy_decay_scale_m: 820.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 2,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: true,
        },
        AmmoCaliberId::Gauge12 => AmmunitionProfile {
            caliber_id: caliber,
            name: "12-Gauge (2-3/4 in)",
            caliber_mm: 18.5,
            bullet_mass_grains: 437.0,
            powder_charge_grains: 32.0,
            optimal_barrel_cm: 51.0,
            reference_muzzle_energy_j: 3150.0,
            energy_decay_scale_m: 140.0,
            expansion_factor: 1.35,
            sectional_density_pen_bonus: 1,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
        AmmoCaliberId::Lapua338 => AmmunitionProfile {
            caliber_id: caliber,
            name: ".338 Lapua Magnum",
            caliber_mm: 8.6,
            bullet_mass_grains: 250.0,
            powder_charge_grains: 90.0,
            optimal_barrel_cm: 69.0,
            reference_muzzle_energy_j: 6600.0,
            energy_decay_scale_m: 1350.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 4,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: true,
        },
        AmmoCaliberId::Bmg50 => AmmunitionProfile {
            caliber_id: caliber,
            name: ".50 BMG (12.7x99mm)",
            caliber_mm: 12.7,
            bullet_mass_grains: 660.0,
            powder_charge_grains: 235.0,
            optimal_barrel_cm: 91.0,
            reference_muzzle_energy_j: 18050.0,
            energy_decay_scale_m: 1700.0,
            expansion_factor: 1.0,
            sectional_density_pen_bonus: 5,
            ammo_quality_q: 0.0,
            is_spitzer_early_yaw: false,
        },
    }
}

/// Constructs a custom [`AmmunitionProfile`] by applying [`BulletConstructionKind`] and
/// handload consistency $Q_{\text{ammo}} \in [-1.5, +1.0]$ on top of a baseline caliber.
#[must_use]
pub fn build_custom_ammunition_profile(
    caliber: AmmoCaliberId,
    bullet: BulletConstructionKind,
    handload_quality_q: f64,
) -> AmmunitionProfile {
    let mut profile = base_caliber_profile(caliber);
    let already_subsonic = matches!(
        caliber,
        AmmoCaliberId::Blackout300Subsonic | AmmoCaliberId::Rimfire22Lr | AmmoCaliberId::Acp45
    );
    let (exp_mult, pen_delta, e_mult, powder_mult, q_shift, disable_spitzer) = match bullet {
        BulletConstructionKind::FmjBall => (1.0, 0, 1.0, 1.0, 0.0, false),
        BulletConstructionKind::HollowPointJhp => (1.45, -1, 1.0, 1.0, 0.25, true),
        BulletConstructionKind::MatchOtm => (1.20, 0, 1.02, 1.0, 1.0, false),
        BulletConstructionKind::ArmorPiercingAp => (0.85, 2, 1.05, 1.0, 0.5, false),
        BulletConstructionKind::SubsonicHeavy => (
            1.10,
            -1,
            if already_subsonic { 1.0 } else { 0.58 },
            if already_subsonic { 1.0 } else { 0.65 },
            0.25,
            true,
        ),
        BulletConstructionKind::OverpressurePlusP => (1.15, 1, 1.15, 1.12, 0.0, false),
    };
    profile.expansion_factor = (profile.expansion_factor * exp_mult).clamp(0.75, 2.0);
    profile.sectional_density_pen_bonus += pen_delta;
    profile.reference_muzzle_energy_j *= e_mult;
    profile.powder_charge_grains *= powder_mult;
    profile.ammo_quality_q = (handload_quality_q + q_shift).clamp(-1.5, 1.0);
    if disable_spitzer {
        profile.is_spitzer_early_yaw = false;
    }
    profile
}

/// Complete engineering blueprint for a custom-designed firearm
/// (`docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md` §2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CustomFirearmBlueprint {
    pub name: &'static str,
    pub caliber: AmmoCaliberId,
    pub bullet: BulletConstructionKind,
    pub handload_quality_q: f64,
    pub action_type: FirearmActionType,
    pub feed_type: WeaponFeedType,
    pub capacity_rounds: i32,
    pub barrel_length_cm: f64,
    pub overall_length_cm: f64,
    pub unloaded_weight_kg: f64,
    pub barrel_mounting: BarrelMountingType,
    pub muzzle_device: MuzzleDeviceType,
    pub center_of_gravity: CenterOfGravityProfile,
    pub requested_tolerance_tau: f64,
    pub requested_barrel_quality_q: f64,
    pub requested_mfg_quality_q: i32,
    pub workshop: WorkshopFacilityTier,
    pub complexity: ProjectClockComplexity,
    pub build_outcome: BuildOutcomeQuality,
}

/// Evaluated custom firearm build including [`GranularFirearmSpec`], [`GranularBallisticsEvaluation`],
/// workshop clamping/masterwork outcomes, quirks, project clock length, and estimated cost.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CustomFirearmDesignResult {
    pub spec: GranularFirearmSpec,
    pub ballistics: GranularBallisticsEvaluation,
    pub clamped_by_workshop: bool,
    pub masterwork_bonus_applied: bool,
    pub quirks: Vec<PrototypeQuirkId>,
    pub required_gunsmith_n_min: i32,
    pub project_clock_target_successes: i32,
    pub estimated_cost_usd: i32,
    pub access_class: BlackMarketRestrictionClass,
}

/// Evaluates a [`CustomFirearmBlueprint`] at target evaluation distance `eval_range_m`, enforcing
/// [`WorkshopFacilityTier`] limits and calling [`evaluate_granular_firearm_ballistics`].
pub fn build_custom_firearm_design(
    bp: &CustomFirearmBlueprint,
    eval_range_m: f64,
) -> CustomFirearmDesignResult {
    let wf = bp.workshop.spec();
    let comp = bp.complexity.spec();
    let ammo = build_custom_ammunition_profile(bp.caliber, bp.bullet, bp.handload_quality_q);

    let tau_clamped = bp
        .requested_tolerance_tau
        .clamp(-1.5, wf.max_tolerance_tau);
    let barrel_q_clamped = bp
        .requested_barrel_quality_q
        .clamp(-2.0, wf.max_barrel_quality_q);
    let mut mfg_q = bp
        .requested_mfg_quality_q
        .clamp(-2, wf.max_mfg_quality_q);

    let clamped_by_workshop = bp.requested_tolerance_tau > wf.max_tolerance_tau + 1e-6
        || bp.requested_barrel_quality_q > wf.max_barrel_quality_q + 1e-6
        || bp.requested_mfg_quality_q > wf.max_mfg_quality_q;

    let mut quirks = Vec::new();
    if clamped_by_workshop && bp.requested_tolerance_tau > wf.max_tolerance_tau + 1e-6 {
        quirks.push(PrototypeQuirkId::FinickyAmmoTolerance);
    }
    if bp.build_outcome == BuildOutcomeQuality::RushedOrBreached {
        quirks.push(PrototypeQuirkId::ThermalDriftBarrel);
        if bp.workshop == WorkshopFacilityTier::FieldBenchImprovised {
            quirks.push(PrototypeQuirkId::ProprietaryPartsOnly);
        }
    } else if tau_clamped >= 1.25 {
        quirks.push(PrototypeQuirkId::FinickyAmmoTolerance);
    }

    if bp.bullet == BulletConstructionKind::OverpressurePlusP {
        mfg_q = (mfg_q - 1).max(-2);
    }

    let masterwork_bonus_applied = bp.build_outcome == BuildOutcomeQuality::MasterworkExtraordinary
        && bp.workshop >= WorkshopFacilityTier::CommercialCncMachineShop;
    if masterwork_bonus_applied {
        mfg_q = (mfg_q + 1).min(2);
    }

    let cap = bp.capacity_rounds.max(1);
    let extra_mass = if quirks.contains(&PrototypeQuirkId::OverweightForwardBalance) {
        0.6
    } else {
        0.0
    };
    let round_mass_kg = (ammo.bullet_mass_grains * 2.1 + ammo.powder_charge_grains) / 15432.0;
    let mag_overhead_kg = match bp.feed_type {
        WeaponFeedType::BeltFed => 0.45,
        WeaponFeedType::DrumOrHelical => 0.35,
        WeaponFeedType::DetachableBoxMagazine => 0.14,
        _ => 0.0,
    };
    let unloaded_weight_kg = (bp.unloaded_weight_kg.max(0.4) + extra_mass).max(0.4);
    let loaded_weight_kg = unloaded_weight_kg + mag_overhead_kg + round_mass_kg * f64::from(cap);

    let (clear_jam_minor_ticks, clear_jam_major_ticks, strip_time_ticks) = match bp.action_type {
        FirearmActionType::BoltAction | FirearmActionType::RevolverCylinderGap => (6, 18, 48),
        FirearmActionType::PumpAction | FirearmActionType::SingleAction => (5, 16, 28),
        FirearmActionType::OpenBoltOrAutoSwitch => (8, 28, 52),
        _ => (6, 20, 36),
    };

    let base_platform_cost = match bp.caliber {
        AmmoCaliberId::Rimfire22Lr | AmmoCaliberId::Acp380 => 420.0,
        AmmoCaliberId::Parabellum9x19
        | AmmoCaliberId::Acp45
        | AmmoCaliberId::Auto10mm
        | AmmoCaliberId::Magnum357 => 680.0,
        AmmoCaliberId::Gauge12 | AmmoCaliberId::Soviet762x39 => 920.0,
        AmmoCaliberId::Hk46x30
        | AmmoCaliberId::Fn57x28
        | AmmoCaliberId::Nato556x45
        | AmmoCaliberId::Blackout300Subsonic
        | AmmoCaliberId::Blackout300Supersonic => 1450.0,
        AmmoCaliberId::Nato762x51 | AmmoCaliberId::Creedmoor65 => 2400.0,
        AmmoCaliberId::Lapua338 => 4800.0,
        AmmoCaliberId::Bmg50 => 9200.0,
    };
    let muzzle_cost = match bp.muzzle_device {
        MuzzleDeviceType::BareCrown => 0.0,
        MuzzleDeviceType::FlashHider => 90.0,
        MuzzleDeviceType::MuzzleBrakeCompensator => 180.0,
        MuzzleDeviceType::SoundSuppressor => 850.0,
    };
    let precision_mult = 1.0 + 0.25 * (tau_clamped.max(0.0) + barrel_q_clamped.max(0.0));
    let estimated_cost_usd = ((base_platform_cost * comp.parts_cost_multiplier * precision_mult)
        + muzzle_cost)
        .round() as i32;

    let access_class = if matches!(
        bp.action_type,
        FirearmActionType::OpenBoltOrAutoSwitch
    ) || bp.feed_type == WeaponFeedType::BeltFed
        || bp.muzzle_device == MuzzleDeviceType::SoundSuppressor
        || bp.bullet == BulletConstructionKind::ArmorPiercingAp
        || matches!(bp.caliber, AmmoCaliberId::Lapua338 | AmmoCaliberId::Bmg50)
    {
        BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto
    } else if matches!(
        bp.caliber,
        AmmoCaliberId::Hk46x30
            | AmmoCaliberId::Fn57x28
            | AmmoCaliberId::Blackout300Subsonic
            | AmmoCaliberId::Blackout300Supersonic
            | AmmoCaliberId::Nato556x45
            | AmmoCaliberId::Soviet762x39
            | AmmoCaliberId::Creedmoor65
            | AmmoCaliberId::Nato762x51
    ) || cap > 15
    {
        BlackMarketRestrictionClass::Class2LawEnforcementParamilitary
    } else {
        BlackMarketRestrictionClass::Class1LicensedCivilian
    };

    let spec = GranularFirearmSpec {
        name: bp.name,
        ammo,
        feed_type: bp.feed_type,
        capacity_rounds: cap,
        action_type: bp.action_type,
        unloaded_weight_kg,
        loaded_weight_kg,
        overall_length_cm: bp.overall_length_cm.max(bp.barrel_length_cm + 6.0),
        barrel_length_cm: bp.barrel_length_cm.max(4.0),
        barrel_quality_q: barrel_q_clamped,
        barrel_mounting: bp.barrel_mounting,
        muzzle_device: bp.muzzle_device,
        center_of_gravity: bp.center_of_gravity,
        tolerance_tau: tau_clamped,
        manufacturing_quality_q: mfg_q,
        clear_jam_minor_ticks,
        clear_jam_major_ticks,
        strip_time_ticks,
        cost_usd: estimated_cost_usd,
    };

    let ballistics = evaluate_granular_firearm_ballistics(&spec, eval_range_m);

    CustomFirearmDesignResult {
        spec,
        ballistics,
        clamped_by_workshop,
        masterwork_bonus_applied,
        quirks,
        required_gunsmith_n_min: comp.required_skill_n_min,
        project_clock_target_successes: comp.target_successes,
        estimated_cost_usd,
        access_class,
    }
}

// ============================================================================
// §3 Custom Melee Weapon & Body Armor Designer
// ============================================================================

define_coded_enum! {
    /// Custom Melee Weapon or Shield Chassis Form Factor.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum MeleeChassisClass [count = 6] {
        Knife => { code: "KNIFE", label: "Combat Knife / Dagger", aliases: ["DAGGER"] },
        BatonOrClub => { code: "BATON", label: "Baton / Tonfa / Heavy Club", aliases: ["CLUB"] },
        SwordOrMachete => { code: "SWORD", label: "Tactical Machete / Sword", aliases: ["MACHETE"] },
        AxeOrBreacher => { code: "AXE_BREACHER", label: "Tomahawk / Breaching Axe", aliases: ["AXE"] },
        Polearm => { code: "POLEARM", label: "Reinforced Staff / Spear", aliases: ["STAFF", "SPEAR"] },
        BallisticOrRiotShield => { code: "SHIELD", label: "Ballistic / Riot Shield", aliases: ["RIOT_SHIELD"] },
    }
}

define_coded_enum! {
    /// Metallurgy & Composite Grade for Custom Melee Weapons & Shields.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum MeleeMaterialGrade [count = 5] {
        MildScrapSteel => { code: "MILD_SCRAP", label: "Mild Scrap Steel", aliases: ["SCRAP"] },
        #[default]
        SpringCarbonSteel => { code: "SPRING_CARBON", label: "1095 / 5160 Spring Carbon Steel", aliases: ["CARBON_STEEL"] },
        ToolSteelS7 => { code: "TOOL_STEEL_S7", label: "S7 / CPM-3V Shock Tool Steel", aliases: ["TOOL_STEEL"] },
        TitaniumCarbideEdge => { code: "TITANIUM_CARBIDE", label: "Titanium-Carbide Non-Magnetic Edge", aliases: ["TITANIUM"] },
        CarbonPolyComposite => { code: "CARBON_COMPOSITE", label: "Carbon-Aramid Structural Composite", aliases: ["COMPOSITE"] },
    }
}

define_coded_enum! {
    /// Ergonomic Balance & Mass Distribution Tuning for Custom Melee Weapons.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum MeleeErgonomicTuning [count = 3] {
        #[default]
        Balanced => { code: "BALANCED", label: "Neutral Guard Balance", aliases: [] },
        SpeedLightened => { code: "SPEED_LIGHTENED", label: "Skeletonized Fast Recovery (-1 TC)", aliases: ["LIGHTENED"] },
        HeavyImpact => { code: "HEAVY_IMPACT", label: "Forward-Weighted Impact (+1 Trauma, +1 TC)", aliases: ["HEAVY"] },
    }
}

/// Output specification for a custom-forged melee weapon or shield.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomMeleeDesignResult {
    pub min_reach: i32,
    pub max_reach: i32,
    pub base_tc: i32,
    pub guard_bonus: i32,
    pub pow_multiplier: f64,
    pub flat_trauma_or_bleed_bonus: i32,
    pub block_sweep_force_bonus: i32,
    pub has_keep_out_property: bool,
    pub weight_kg: f64,
    pub durability_rel_mod: i32,
    pub cost_usd: i32,
    pub clock_target_successes: i32,
    pub quirks: Vec<PrototypeQuirkId>,
}

/// Evaluates a custom melee weapon or shield design (`engineering_and_weapon_design.md` §3.1).
#[must_use]
pub fn evaluate_custom_melee_design(
    chassis: MeleeChassisClass,
    material: MeleeMaterialGrade,
    tuning: MeleeErgonomicTuning,
    workshop: WorkshopFacilityTier,
    outcome: BuildOutcomeQuality,
) -> CustomMeleeDesignResult {
    let (
        min_reach,
        max_reach,
        base_tc,
        base_guard,
        base_pow,
        base_trauma,
        base_blk,
        keep_out,
        base_kg,
        base_cost,
        base_succ,
    ): (i32, i32, i32, i32, f64, i32, i32, bool, f64, f64, i32) = match chassis {
            MeleeChassisClass::Knife => (0, 1, 5, 0, 0.9, 2, 0, false, 0.35, 180.0, 4),
            MeleeChassisClass::BatonOrClub => (1, 1, 6, 1, 1.0, 1, 1, false, 0.65, 220.0, 4),
            MeleeChassisClass::SwordOrMachete => (1, 2, 7, 1, 1.25, 3, 1, false, 1.05, 450.0, 8),
            MeleeChassisClass::AxeOrBreacher => (1, 2, 8, 0, 1.35, 4, 1, false, 1.35, 380.0, 8),
            MeleeChassisClass::Polearm => (2, 3, 9, 2, 1.2, 2, 2, true, 1.60, 320.0, 8),
            MeleeChassisClass::BallisticOrRiotShield => (0, 1, 10, 4, 0.85, 0, 5, false, 4.20, 1200.0, 8),
        };

    let (mat_trauma, mat_weight_mult, mat_durability, mat_cost_mult): (i32, f64, i32, f64) =
        match material {
        MeleeMaterialGrade::MildScrapSteel => (-1, 1.15, -2, 0.5),
        MeleeMaterialGrade::SpringCarbonSteel => (0, 1.00, 0, 1.0),
        MeleeMaterialGrade::ToolSteelS7 => (1, 1.00, 2, 2.2),
        MeleeMaterialGrade::TitaniumCarbideEdge => (0, 0.72, 1, 4.0),
        MeleeMaterialGrade::CarbonPolyComposite => (-1, 0.62, 1, 2.8),
    };

    let (tc_shift, trauma_shift, guard_shift, weight_mult): (i32, i32, i32, f64) = match tuning {
        MeleeErgonomicTuning::Balanced => (0, 0, 0, 1.0),
        MeleeErgonomicTuning::SpeedLightened => (-1, -1, 1, 0.88),
        MeleeErgonomicTuning::HeavyImpact => (1, 1, -1, 1.18),
    };

    let mut durability_rel_mod = mat_durability + workshop.spec().max_mfg_quality_q;
    let mut quirks = Vec::new();
    if outcome == BuildOutcomeQuality::MasterworkExtraordinary {
        durability_rel_mod += 1;
    } else if outcome == BuildOutcomeQuality::RushedOrBreached {
        durability_rel_mod -= 1;
        quirks.push(PrototypeQuirkId::OverweightForwardBalance);
    }
    if workshop == WorkshopFacilityTier::FieldBenchImprovised
        && matches!(
            material,
            MeleeMaterialGrade::TitaniumCarbideEdge | MeleeMaterialGrade::ToolSteelS7
        )
    {
        quirks.push(PrototypeQuirkId::ProprietaryPartsOnly);
    }

    CustomMeleeDesignResult {
        min_reach,
        max_reach,
        base_tc: (base_tc + tc_shift).clamp(4, 14),
        guard_bonus: (base_guard + guard_shift).max(0),
        pow_multiplier: base_pow,
        flat_trauma_or_bleed_bonus: (base_trauma + mat_trauma + trauma_shift).max(0),
        block_sweep_force_bonus: base_blk,
        has_keep_out_property: keep_out,
        weight_kg: ((base_kg * mat_weight_mult * weight_mult) * 100.0).round() / 100.0,
        durability_rel_mod,
        cost_usd: (base_cost * mat_cost_mult).round() as i32,
        clock_target_successes: base_succ,
        quirks,
    }
}

define_coded_enum! {
    /// Body Armor Laminate / Plate Architecture (`engineering_and_weapon_design.md` §3.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ArmorMaterialArchitecture [count = 5] {
        SynthLeather => { code: "SYNTH_LEATHER", label: "Heavy Moto Synth-Leather", aliases: [] },
        ParaAramidSoftWeave => { code: "PARA_ARAMID", label: "Kevlar / Twaron Soft Aramid Weave", aliases: ["ARAMID"] },
        UhmwpePlate => { code: "UHMWPE_PLATE", label: "Dyneema UHMWPE Hard Rifle Plate", aliases: ["UHMWPE"] },
        BoronCarbideCeramic => { code: "BORON_CARBIDE", label: "Boron-Carbide / SiC NIJ IV Strike Face", aliases: ["CERAMIC"] },
        StfNanotubeCarapace => { code: "STF_NANOTUBE", label: "2035 Shear-Thickening Nanotube Carapace", aliases: ["NANOTUBE"] },
    }
}

define_coded_enum! {
    /// Body Armor Coverage & Rig Form Factor.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ArmorCoverageTier [count = 3] {
        ConcealableTorso => { code: "CONCEALABLE", label: "Concealable Under-Shirt Vest", aliases: ["VEST"] },
        TacticalCarrier => { code: "TACTICAL_CARRIER", label: "Modular Tactical Plate Carrier", aliases: ["CARRIER"] },
        FullBodyHeavy => { code: "FULL_BODY_HEAVY", label: "Articulated Full-Limb Heavy Suit", aliases: ["FULL_BODY"] },
    }
}

/// Output specification for a custom-tailored body armor system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomArmorDesignResult {
    pub pr_edged: i32,
    pub pr_icepick_flechette: i32,
    pub pr_ballistic: i32,
    pub blunt_trauma_dampening_btd: i32,
    pub weight_kg: f64,
    pub dex_encumbrance_penalty: i32,
    pub cost_usd: i32,
    pub clock_target_successes: i32,
    pub access_class: BlackMarketRestrictionClass,
    pub quirks: Vec<PrototypeQuirkId>,
}

/// Evaluates a custom body armor assembly (`engineering_and_weapon_design.md` §3.2).
#[must_use]
pub fn evaluate_custom_armor_design(
    material: ArmorMaterialArchitecture,
    coverage: ArmorCoverageTier,
    workshop: WorkshopFacilityTier,
    outcome: BuildOutcomeQuality,
) -> CustomArmorDesignResult {
    let (base_edg, base_ice, base_bal, base_btd, base_kg, base_cost, access): (
        i32,
        i32,
        i32,
        i32,
        f64,
        f64,
        BlackMarketRestrictionClass,
    ) = match material {
        ArmorMaterialArchitecture::SynthLeather => (
            2,
            1,
            0,
            1,
            1.8,
            240.0,
            BlackMarketRestrictionClass::Class1LicensedCivilian,
        ),
        ArmorMaterialArchitecture::ParaAramidSoftWeave => (
            4,
            2,
            3,
            3,
            2.2,
            780.0,
            BlackMarketRestrictionClass::Class1LicensedCivilian,
        ),
        ArmorMaterialArchitecture::UhmwpePlate => (
            6,
            5,
            6,
            5,
            4.5,
            1850.0,
            BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
        ),
        ArmorMaterialArchitecture::BoronCarbideCeramic => (
            7,
            7,
            8,
            6,
            6.8,
            3200.0,
            BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
        ),
        ArmorMaterialArchitecture::StfNanotubeCarapace => (
            8,
            8,
            8,
            8,
            5.4,
            18000.0,
            BlackMarketRestrictionClass::Class4StrategicStateSecret,
        ),
    };

    let (cov_kg_mult, cov_cost_mult, dex_pen, succ): (f64, f64, i32, i32) = match coverage {
        ArmorCoverageTier::ConcealableTorso => (0.85, 1.0, 0, 4),
        ArmorCoverageTier::TacticalCarrier => (1.25, 1.4, -1, 8),
        ArmorCoverageTier::FullBodyHeavy => (2.35, 2.8, -3, 14),
    };

    let masterwork_btd = if outcome == BuildOutcomeQuality::MasterworkExtraordinary {
        1
    } else {
        0
    };
    let mut quirks = Vec::new();
    let workshop_bal_penalty = if matches!(
        material,
        ArmorMaterialArchitecture::BoronCarbideCeramic
            | ArmorMaterialArchitecture::StfNanotubeCarapace
    ) && workshop < WorkshopFacilityTier::CommercialCncMachineShop
    {
        quirks.push(PrototypeQuirkId::ProprietaryPartsOnly);
        -1
    } else {
        0
    };
    if outcome == BuildOutcomeQuality::RushedOrBreached {
        quirks.push(PrototypeQuirkId::OverweightForwardBalance);
    }

    CustomArmorDesignResult {
        pr_edged: base_edg,
        pr_icepick_flechette: base_ice,
        pr_ballistic: (base_bal + workshop_bal_penalty).max(0),
        blunt_trauma_dampening_btd: base_btd + masterwork_btd,
        weight_kg: ((base_kg * cov_kg_mult) * 100.0).round() / 100.0,
        dex_encumbrance_penalty: dex_pen,
        cost_usd: (base_cost * cov_cost_mult).round() as i32,
        clock_target_successes: succ,
        access_class: max_access_class(
            access,
            if coverage == ArmorCoverageTier::FullBodyHeavy {
                BlackMarketRestrictionClass::Class2LawEnforcementParamilitary
            } else {
                BlackMarketRestrictionClass::Class1LicensedCivilian
            },
        ),
        quirks,
    }
}
