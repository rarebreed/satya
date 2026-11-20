//! # Expanded Arsenal, Shotgun Specialty Loads, Muscle/Less-Lethal, Explosives & Tactical Gear
//! (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §1–§4).
//!
//! Companion module to [crate::ballistics] (`17` canonical firearms across `17` calibers) and
//! [crate::barrier_ballistics] (`MaterialClass` barriers). Re-exported through [crate::combat] so all
//! downstream callers via `satya_engine::*` receive the full Phase 3 arsenal surface automatically.

use satya_schema::MaterialClass;
use serde::{Deserialize, Serialize};

use crate::barrier_ballistics::material_ballistic_profile;
use crate::dice_probabilities::{ContestedOutcomeKind, ContestedTieRule};
use crate::economy_gear::BlackMarketRestrictionClass;
use crate::primitives::{
    ConditionImpact, ConditionSource, ContestedCheck, TieredSaveEval, Triad,
};
use crate::ranged_combat::WeaponDamageType;
use crate::{define_coded_enum, impl_static_catalog};

// ============================================================================
// 1. Specialty 12-Gauge Shotgun Loads (§1.3)
// ============================================================================

define_coded_enum! {
    /// Five canonical 12-Gauge specialty ammunition loadings
    /// (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §1.3).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[repr(usize)]
    pub enum ShotgunLoadId [count = 5] {
        Buckshot00 => {
            code: "SHOTGUN_00_BUCK",
            label: "12-Gauge 00 Buckshot (9-Pellet Lead)",
            aliases: ["00_BUCK", "BUCKSHOT"],
        },
        RifledFosterSlug => {
            code: "SHOTGUN_FOSTER_SLUG",
            label: "12-Gauge 1 oz Rifled Foster / Sabot Slug",
            aliases: ["FOSTER_SLUG", "SLUG"],
        },
        BreacherFrangibleLockBuster => {
            code: "SHOTGUN_BREACHER_FRANGIBLE",
            label: "12-Gauge Sintered Zinc Frangible Breaching Shell",
            aliases: ["BREACHER_SHELL", "LOCK_BUSTER"],
        },
        DragonsBreathIncendiary => {
            code: "SHOTGUN_DRAGONS_BREATH",
            label: "12-Gauge Magnesium Dragon's Breath Incendiary",
            aliases: ["DRAGONS_BREATH"],
        },
        BeanbagLessLethal => {
            code: "SHOTGUN_BEANBAG_LESS_LETHAL",
            label: "12-Gauge Kevlar-Sock Beanbag Less-Lethal",
            aliases: ["BEANBAG"],
        },
    }
}

/// Physical & terminal specification of a 12-Gauge specialty shell (`arsenal_explosives_and_tactical_gear.md` §1.3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ShotgunLoadSpec {
    pub id: ShotgunLoadId,
    pub damage_type: WeaponDamageType,
    pub raw_impact: i32,
    pub base_pen: i32,
    pub effective_range_m: f64,
    /// Close-range pattern aim shift (`-2` $\Delta_{\text{ext}}$ at `<= 15 m` for buckshot; `0` for slugs).
    pub close_range_delta_ext_mod: i32,
    pub reliability_mod: i32,
    pub causes_ignition_check: bool,
    pub cost_per_10_usd: u32,
}

const SHOTGUN_LOAD_CATALOG_ARRAY: [ShotgunLoadSpec; 5] = [
    ShotgunLoadSpec {
        id: ShotgunLoadId::Buckshot00,
        damage_type: WeaponDamageType::Ballistic,
        raw_impact: 11,
        base_pen: 3,
        effective_range_m: 35.0,
        close_range_delta_ext_mod: -2,
        reliability_mod: 0,
        causes_ignition_check: false,
        cost_per_10_usd: 15,
    },
    ShotgunLoadSpec {
        id: ShotgunLoadId::RifledFosterSlug,
        damage_type: WeaponDamageType::Ballistic,
        raw_impact: 10,
        base_pen: 5,
        effective_range_m: 100.0,
        close_range_delta_ext_mod: 0,
        reliability_mod: 0,
        causes_ignition_check: false,
        cost_per_10_usd: 22,
    },
    ShotgunLoadSpec {
        id: ShotgunLoadId::BreacherFrangibleLockBuster,
        damage_type: WeaponDamageType::Blunt,
        raw_impact: 8,
        base_pen: 6,
        effective_range_m: 3.0,
        close_range_delta_ext_mod: -2,
        reliability_mod: 0,
        causes_ignition_check: false,
        cost_per_10_usd: 45,
    },
    ShotgunLoadSpec {
        id: ShotgunLoadId::DragonsBreathIncendiary,
        damage_type: WeaponDamageType::Ballistic,
        raw_impact: 6,
        base_pen: 1,
        effective_range_m: 20.0,
        close_range_delta_ext_mod: -4,
        reliability_mod: -2,
        causes_ignition_check: true,
        cost_per_10_usd: 75,
    },
    ShotgunLoadSpec {
        id: ShotgunLoadId::BeanbagLessLethal,
        damage_type: WeaponDamageType::Blunt,
        raw_impact: 7,
        base_pen: 0,
        effective_range_m: 25.0,
        close_range_delta_ext_mod: 0,
        reliability_mod: -1,
        causes_ignition_check: false,
        cost_per_10_usd: 40,
    },
];

impl_static_catalog!(ShotgunLoadId => ShotgunLoadSpec : SHOTGUN_LOAD_CATALOG_ARRAY);

/// Canonical 5-entry 12-Gauge specialty shell catalog.
pub const SHOTGUN_LOAD_CATALOG: &[ShotgunLoadSpec] = &SHOTGUN_LOAD_CATALOG_ARRAY;

/// Return the canonical 5-entry 12-Gauge specialty shell catalog.
#[must_use]
pub const fn canonical_shotgun_load_catalog() -> &'static [ShotgunLoadSpec] {
    SHOTGUN_LOAD_CATALOG
}

// ============================================================================
// 2. Muscle-Powered & Less-Lethal Systems (§2)
// ============================================================================

define_coded_enum! {
    /// Six canonical muscle-powered & less-lethal ranged platforms
    /// (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[repr(usize)]
    pub enum NonFirearmWeaponId [count = 6] {
        CompoundBow70Lb => {
            code: "COMPOUND_BOW_70LB",
            label: "70-lb Compound Hunting Bow (Broadhead)",
            aliases: ["COMPOUND_BOW", "BOW"],
        },
        TacticalCrossbow => {
            code: "TACTICAL_CROSSBOW",
            label: "Tactical Recurve / Compound Crossbow",
            aliases: ["CROSSBOW"],
        },
        PcpAirRifleSuppressed => {
            code: "PCP_AIR_RIFLE",
            label: "Pre-Charged Pneumatic (.357 PCP) Suppressed Air Rifle",
            aliases: ["PCP_AIRGUN"],
        },
        TaserCewDualProbe => {
            code: "TASER_CEW",
            label: "Conducted Energy Weapon (TASER X10 Dual-Dart)",
            aliases: ["TASER", "CEW"],
        },
        OcPepperSprayStream => {
            code: "OC_PEPPER_SPRAY",
            label: "High-Pressure OC Oleoresin Capsicum Stream",
            aliases: ["PEPPER_SPRAY", "OC_SPRAY"],
        },
        Baton40MmSpongeRound => {
            code: "SPONGE_BATON_40MM",
            label: "40mm eXact iMpact Sponge Baton Launcher",
            aliases: ["40MM_SPONGE", "SPONGE_ROUND"],
        },
    }
}

/// Mechanical profile for a muscle-powered or less-lethal launcher (`arsenal_explosives_and_tactical_gear.md` §2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct NonFirearmWeaponSpec {
    pub id: NonFirearmWeaponId,
    pub damage_type: WeaponDamageType,
    pub raw_impact: i32,
    pub base_pen: i32,
    pub effective_range_m: f64,
    pub base_tc: i32,
    pub reload_ticks: i32,
    pub min_for_req: i32,
    /// Negative values indicate acoustic stealth reduction (`HEA` penalty to listeners).
    pub acoustic_stealth_mod_hea: i32,
    pub mass_kg: f64,
    pub cost_usd: u32,
    pub access_class: BlackMarketRestrictionClass,
}

const NON_FIREARM_WEAPON_CATALOG_ARRAY: [NonFirearmWeaponSpec; 6] = [
    NonFirearmWeaponSpec {
        id: NonFirearmWeaponId::CompoundBow70Lb,
        damage_type: WeaponDamageType::Edged,
        raw_impact: 7,
        base_pen: 3,
        effective_range_m: 65.0,
        base_tc: 11,
        reload_ticks: 10,
        min_for_req: 11,
        acoustic_stealth_mod_hea: -10,
        mass_kg: 1.95,
        cost_usd: 950,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
    },
    NonFirearmWeaponSpec {
        id: NonFirearmWeaponId::TacticalCrossbow,
        damage_type: WeaponDamageType::IcepickOrFlechette,
        raw_impact: 7,
        base_pen: 4,
        effective_range_m: 75.0,
        base_tc: 9,
        reload_ticks: 24,
        min_for_req: 9,
        acoustic_stealth_mod_hea: -9,
        mass_kg: 3.10,
        cost_usd: 1_200,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
    },
    NonFirearmWeaponSpec {
        id: NonFirearmWeaponId::PcpAirRifleSuppressed,
        damage_type: WeaponDamageType::Ballistic,
        raw_impact: 5,
        base_pen: 2,
        effective_range_m: 90.0,
        base_tc: 10,
        reload_ticks: 12,
        min_for_req: 8,
        acoustic_stealth_mod_hea: -11,
        mass_kg: 3.40,
        cost_usd: 1_450,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
    },
    NonFirearmWeaponSpec {
        id: NonFirearmWeaponId::TaserCewDualProbe,
        damage_type: WeaponDamageType::Blunt,
        raw_impact: 2,
        base_pen: 1,
        effective_range_m: 10.0,
        base_tc: 7,
        reload_ticks: 14,
        min_for_req: 6,
        acoustic_stealth_mod_hea: -6,
        mass_kg: 0.42,
        cost_usd: 1_100,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
    },
    NonFirearmWeaponSpec {
        id: NonFirearmWeaponId::OcPepperSprayStream,
        damage_type: WeaponDamageType::Blunt,
        raw_impact: 0,
        base_pen: 0,
        effective_range_m: 4.5,
        base_tc: 6,
        reload_ticks: 0,
        min_for_req: 5,
        acoustic_stealth_mod_hea: -8,
        mass_kg: 0.14,
        cost_usd: 35,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
    },
    NonFirearmWeaponSpec {
        id: NonFirearmWeaponId::Baton40MmSpongeRound,
        damage_type: WeaponDamageType::Blunt,
        raw_impact: 8,
        base_pen: 0,
        effective_range_m: 45.0,
        base_tc: 9,
        reload_ticks: 16,
        min_for_req: 9,
        acoustic_stealth_mod_hea: -2,
        mass_kg: 2.30,
        cost_usd: 1_850,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
    },
];

impl_static_catalog!(NonFirearmWeaponId => NonFirearmWeaponSpec : NON_FIREARM_WEAPON_CATALOG_ARRAY);

/// Canonical 6-entry Muscle-Powered & Less-Lethal catalog.
pub const NON_FIREARM_WEAPON_CATALOG: &[NonFirearmWeaponSpec] = &NON_FIREARM_WEAPON_CATALOG_ARRAY;

/// Return the canonical 6-entry Muscle-Powered & Less-Lethal catalog.
#[must_use]
pub const fn canonical_non_firearm_weapon_catalog() -> &'static [NonFirearmWeaponSpec] {
    NON_FIREARM_WEAPON_CATALOG
}

/// Physiological outcome of a less-lethal hit (TASER, OC spray, 40mm sponge, or beanbag)
/// (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §2.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessLethalExposureEffect {
    pub weapon: NonFirearmWeaponId,
    pub mitigated_by_ppe_or_armor: bool,
    pub total_hidden_delta_ext: i32,
    pub delta_int_penalty: i32,
    pub incapacitated_or_paralyzed: bool,
    pub blinded_or_nauseated: bool,
    pub knocked_prone_or_winded: bool,
    pub lock_or_stun_ticks: i32,
    pub summary: &'static str,
}

impl ConditionSource for LessLethalExposureEffect {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.delta_int_penalty,
            wound_shock_added: if self.knocked_prone_or_winded && !self.mitigated_by_ppe_or_armor {
                1
            } else {
                0
            },
            clock_freeze_ticks: self.lock_or_stun_ticks.min(20),
            triad_locked_zero: Triad::splat(self.incapacitated_or_paralyzed),
            ..ConditionImpact::ZERO
        }
    }
}

/// Resolve a defender's `HLT/DIS` or `FIT/DIS` resistance save against a less-lethal attack
/// (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §2.1).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_less_lethal_exposure(
    weapon: NonFirearmWeaponId,
    target_armor_pr_ballistic: i32,
    target_sealed_goggles_or_respirator: bool,
    attacker_net_successes: i32,
    defender_hlt_or_fit_c: i32,
    defender_save_n: usize,
    internal_mod: i32,
    defender_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<LessLethalExposureEffect> {
    let base_severity = match weapon {
        NonFirearmWeaponId::TaserCewDualProbe | NonFirearmWeaponId::OcPepperSprayStream => 4,
        NonFirearmWeaponId::Baton40MmSpongeRound => 3,
        _ => 1,
    };
    let total_hidden_delta_ext = base_severity + (attacker_net_successes.max(0) * 2);

    let eval = TieredSaveEval::evaluate(
        defender_save_n,
        defender_hlt_or_fit_c,
        internal_mod,
        0,
        total_hidden_delta_ext,
        tie_rule,
        defender_pairs,
        gm_pair,
    );

    let ppe_blocked = match weapon {
        NonFirearmWeaponId::TaserCewDualProbe => target_armor_pr_ballistic >= 2,
        NonFirearmWeaponId::OcPepperSprayStream => target_sealed_goggles_or_respirator,
        _ => false,
    };

    let effect = if ppe_blocked {
        LessLethalExposureEffect {
            weapon,
            mitigated_by_ppe_or_armor: true,
            total_hidden_delta_ext,
            delta_int_penalty: 0,
            incapacitated_or_paralyzed: false,
            blinded_or_nauseated: false,
            knocked_prone_or_winded: false,
            lock_or_stun_ticks: 0,
            summary: "PPE / Armor Defeated Exposure: probes/spray failed to reach skin or eyes.",
        }
    } else {
        eval.classify_4way(
            |_| LessLethalExposureEffect {
                weapon,
                mitigated_by_ppe_or_armor: false,
                total_hidden_delta_ext,
                delta_int_penalty: 0,
                incapacitated_or_paralyzed: false,
                blinded_or_nauseated: false,
                knocked_prone_or_winded: false,
                lock_or_stun_ticks: 0,
                summary: "Fought Through cleanly: zero incapacitation or loss of tempo.",
            },
            |roll| match roll.contested_kind {
                ContestedOutcomeKind::CleanSuccess => LessLethalExposureEffect {
                    weapon,
                    mitigated_by_ppe_or_armor: false,
                    total_hidden_delta_ext,
                    delta_int_penalty: 1,
                    incapacitated_or_paralyzed: false,
                    blinded_or_nauseated: false,
                    knocked_prone_or_winded: false,
                    lock_or_stun_ticks: 0,
                    summary: "Brief Stagger: resisted primary incapacitation (+1 Delta_int shock).",
                },
                _ => LessLethalExposureEffect {
                    weapon,
                    mitigated_by_ppe_or_armor: false,
                    total_hidden_delta_ext,
                    delta_int_penalty: 2,
                    incapacitated_or_paralyzed: false,
                    blinded_or_nauseated: weapon == NonFirearmWeaponId::OcPepperSprayStream,
                    knocked_prone_or_winded: weapon == NonFirearmWeaponId::Baton40MmSpongeRound,
                    lock_or_stun_ticks: 4,
                    summary: "Partial Less-Lethal Disruption: +2 Delta_int, tearing/winded (+4 Ticks OODA stutter).",
                },
            },
            |_| LessLethalExposureEffect {
                weapon,
                mitigated_by_ppe_or_armor: false,
                total_hidden_delta_ext,
                delta_int_penalty: 4,
                incapacitated_or_paralyzed: false,
                blinded_or_nauseated: weapon == NonFirearmWeaponId::OcPepperSprayStream,
                knocked_prone_or_winded: weapon != NonFirearmWeaponId::OcPepperSprayStream,
                lock_or_stun_ticks: 8,
                summary: "Severe Less-Lethal Impairment: +4 Delta_int, blinded or winded knockdown (8 Ticks freeze).",
            },
            |_| LessLethalExposureEffect {
                weapon,
                mitigated_by_ppe_or_armor: false,
                total_hidden_delta_ext,
                delta_int_penalty: 6,
                incapacitated_or_paralyzed: true,
                blinded_or_nauseated: weapon == NonFirearmWeaponId::OcPepperSprayStream,
                knocked_prone_or_winded: true,
                lock_or_stun_ticks: 20,
                summary: "Full Less-Lethal Incapacitation: 5s (20 Ticks) neuromuscular lock or choking blindness!",
            },
        )
    };

    ContestedCheck::new(eval, effect)
}

// ============================================================================
// 3. Explosives, Blast Overpressure, Fragmentation Cones & Breaching (§3)
// ============================================================================

define_coded_enum! {
    /// Seven canonical grenades, smoke/incendiary charges, and breaching demolitions
    /// (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §3).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[repr(usize)]
    pub enum ExplosiveOrdnanceId [count = 7] {
        M67FragmentationGrenade => {
            code: "M67_FRAG",
            label: "M67 Fragmentation Hand Grenade (Comp B)",
            aliases: ["M67"],
        },
        Mk3A2ConcussionGrenade => {
            code: "MK3A2_CONCUSSION",
            label: "Mk3A2 Offensive Blast / Concussion Grenade",
            aliases: ["MK3A2"],
        },
        M84FlashbangStunGrenade => {
            code: "M84_FLASHBANG",
            label: "M84 Diversionary Flashbang Stun Grenade",
            aliases: ["M84", "FLASHBANG"],
        },
        SmokeScreeningGrenadeIr => {
            code: "M18_IR_SMOKE",
            label: "M83/M18 Multispectral IR-Blocking Smoke Grenade",
            aliases: ["SMOKE_GRENADE", "IR_SMOKE"],
        },
        ThermiteIncendiaryCharge => {
            code: "AN_M14_THERMITE",
            label: "AN-M14 TH3 Thermite Incendiary Charge",
            aliases: ["THERMITE"],
        },
        C4SemtexBlock500g => {
            code: "C4_SEMTEX_500G",
            label: "500g C4 / Semtex Moldable High-Explosive Block",
            aliases: ["C4", "SEMTEX"],
        },
        LinearShapedBreachingCharge => {
            code: "LINEAR_SHAPED_BREACH",
            label: "Linear Shaped Copper-Lined Door/Wall Breaching Charge",
            aliases: ["SHAPED_CHARGE", "DET_CORD_BREACH"],
        },
    }
}

/// Physical parameters of an explosive or pyrotechnic ordnance item (`arsenal_explosives_and_tactical_gear.md` §3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ExplosiveOrdnanceSpec {
    pub id: ExplosiveOrdnanceId,
    pub tnt_equiv_kg: f64,
    pub peak_blast_trauma: i32,
    pub frag_radius_m: f64,
    pub base_frag_hits_3m: i32,
    pub frag_pen: i32,
    pub frag_raw_impact: i32,
    pub breach_pr: i32,
    pub flash_stun_delta_ext: i32,
    pub mass_kg: f64,
    pub cost_usd: u32,
    pub access_class: BlackMarketRestrictionClass,
}

const EXPLOSIVE_ORDNANCE_CATALOG_ARRAY: [ExplosiveOrdnanceSpec; 7] = [
    ExplosiveOrdnanceSpec {
        id: ExplosiveOrdnanceId::M67FragmentationGrenade,
        tnt_equiv_kg: 0.24,
        peak_blast_trauma: 9,
        frag_radius_m: 15.0,
        base_frag_hits_3m: 4,
        frag_pen: 3,
        frag_raw_impact: 6,
        breach_pr: 4,
        flash_stun_delta_ext: 4,
        mass_kg: 0.40,
        cost_usd: 150,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
    },
    ExplosiveOrdnanceSpec {
        id: ExplosiveOrdnanceId::Mk3A2ConcussionGrenade,
        tnt_equiv_kg: 0.23,
        peak_blast_trauma: 11,
        frag_radius_m: 2.0,
        base_frag_hits_3m: 0,
        frag_pen: 0,
        frag_raw_impact: 0,
        breach_pr: 5,
        flash_stun_delta_ext: 6,
        mass_kg: 0.44,
        cost_usd: 180,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
    },
    ExplosiveOrdnanceSpec {
        id: ExplosiveOrdnanceId::M84FlashbangStunGrenade,
        tnt_equiv_kg: 0.01,
        peak_blast_trauma: 1,
        frag_radius_m: 7.0,
        base_frag_hits_3m: 0,
        frag_pen: 0,
        frag_raw_impact: 0,
        breach_pr: 0,
        flash_stun_delta_ext: 6,
        mass_kg: 0.24,
        cost_usd: 120,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
    },
    ExplosiveOrdnanceSpec {
        id: ExplosiveOrdnanceId::SmokeScreeningGrenadeIr,
        tnt_equiv_kg: 0.0,
        peak_blast_trauma: 0,
        frag_radius_m: 12.0,
        base_frag_hits_3m: 0,
        frag_pen: 0,
        frag_raw_impact: 0,
        breach_pr: 0,
        flash_stun_delta_ext: 0,
        mass_kg: 0.54,
        cost_usd: 95,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
    },
    ExplosiveOrdnanceSpec {
        id: ExplosiveOrdnanceId::ThermiteIncendiaryCharge,
        tnt_equiv_kg: 0.0,
        peak_blast_trauma: 0,
        frag_radius_m: 2.0,
        base_frag_hits_3m: 0,
        frag_pen: 10,
        frag_raw_impact: 10,
        breach_pr: 10,
        flash_stun_delta_ext: 0,
        mass_kg: 0.90,
        cost_usd: 450,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
    },
    ExplosiveOrdnanceSpec {
        id: ExplosiveOrdnanceId::C4SemtexBlock500g,
        tnt_equiv_kg: 0.67,
        peak_blast_trauma: 14,
        frag_radius_m: 12.0,
        base_frag_hits_3m: 3,
        frag_pen: 4,
        frag_raw_impact: 7,
        breach_pr: 10,
        flash_stun_delta_ext: 8,
        mass_kg: 0.55,
        cost_usd: 800,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
    },
    ExplosiveOrdnanceSpec {
        id: ExplosiveOrdnanceId::LinearShapedBreachingCharge,
        tnt_equiv_kg: 0.16,
        peak_blast_trauma: 8,
        frag_radius_m: 3.0,
        base_frag_hits_3m: 1,
        frag_pen: 4,
        frag_raw_impact: 6,
        breach_pr: 10,
        flash_stun_delta_ext: 6,
        mass_kg: 0.45,
        cost_usd: 1_100,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
    },
];

impl_static_catalog!(ExplosiveOrdnanceId => ExplosiveOrdnanceSpec : EXPLOSIVE_ORDNANCE_CATALOG_ARRAY);

/// Canonical 7-entry Explosives & Ordnance catalog.
pub const EXPLOSIVE_ORDNANCE_CATALOG: &[ExplosiveOrdnanceSpec] = &EXPLOSIVE_ORDNANCE_CATALOG_ARRAY;

/// Return the canonical 7-entry Explosives & Ordnance catalog.
#[must_use]
pub const fn canonical_explosive_ordnance_catalog() -> &'static [ExplosiveOrdnanceSpec] {
    EXPLOSIVE_ORDNANCE_CATALOG
}

/// Blast overpressure, fragmentation cone, flash/tinnitus, and cover evaluation for a target at `distance_m`
/// (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlastExposureEvaluation {
    pub ordnance: ExplosiveOrdnanceId,
    pub distance_m: f64,
    pub enclosed_room_reflected: bool,
    pub blast_overpressure_trauma: i32,
    pub frag_hit_count: i32,
    pub frag_pen: i32,
    pub frag_raw_impact: i32,
    pub stun_save_delta_ext: i32,
    pub causes_eardrum_tinnitus: bool,
    pub blinded_ticks: i32,
    pub creates_ir_blocking_smoke_cloud: bool,
    pub thermal_melt_active: bool,
}

impl ConditionSource for BlastExposureEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        let delta_int = if self.blinded_ticks > 0 {
            4
        } else if self.causes_eardrum_tinnitus {
            2
        } else {
            0
        };
        let clock_freeze_ticks = if self.blinded_ticks > 0 {
            8
        } else if self.blast_overpressure_trauma >= 6 {
            4
        } else {
            0
        };
        ConditionImpact {
            delta_int,
            wound_shock_added: (self.blast_overpressure_trauma / 4).min(3),
            clock_freeze_ticks,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluate blast overpressure (including `1.5x` indoor pressure reflection), shrapnel cone hits,
/// prone mitigation, hard cover blocking, and flash/smoke effects (`arsenal_explosives_and_tactical_gear.md` §3.1).
#[must_use]
pub fn evaluate_blast_and_fragmentation(
    ordnance: ExplosiveOrdnanceId,
    distance_m: f64,
    enclosed_room: bool,
    target_is_prone: bool,
    intervening_barrier_pr: i32,
) -> BlastExposureEvaluation {
    let spec = ordnance.spec();
    let d = distance_m.max(0.0);
    let dist_eff = d.max(0.5);

    let creates_ir_blocking_smoke_cloud =
        ordnance == ExplosiveOrdnanceId::SmokeScreeningGrenadeIr && d <= spec.frag_radius_m;
    let thermal_melt_active =
        ordnance == ExplosiveOrdnanceId::ThermiteIncendiaryCharge && d <= spec.frag_radius_m;

    let mut blast_overpressure_trauma = if spec.peak_blast_trauma <= 0 {
        0
    } else {
        let raw = ((spec.peak_blast_trauma as f64) * (1.5 / (1.0 + dist_eff * 0.65))).round() as i32;
        raw.clamp(0, spec.peak_blast_trauma)
    };

    if enclosed_room && blast_overpressure_trauma > 0 && d <= spec.frag_radius_m.max(6.0) {
        blast_overpressure_trauma =
            ((blast_overpressure_trauma as f64) * 1.5).round() as i32;
    }
    if target_is_prone && blast_overpressure_trauma > 0 {
        blast_overpressure_trauma = (blast_overpressure_trauma - 2).max(1);
    }
    if intervening_barrier_pr >= 6 {
        blast_overpressure_trauma /= 2;
    }

    let mut frag_hit_count = if spec.base_frag_hits_3m <= 0 || d > spec.frag_radius_m {
        0
    } else if d <= 3.0 {
        spec.base_frag_hits_3m
    } else {
        let falloff = 1.0 - ((d - 3.0) / (spec.frag_radius_m - 3.0).max(1.0));
        ((spec.base_frag_hits_3m as f64) * falloff).ceil().max(1.0) as i32
    };

    if target_is_prone && frag_hit_count > 0 {
        frag_hit_count /= 2;
    }
    if intervening_barrier_pr >= 6 {
        frag_hit_count = 0;
    } else if intervening_barrier_pr >= 3 && frag_hit_count > 0 {
        frag_hit_count /= 2;
    }

    let stun_save_delta_ext = if d <= spec.frag_radius_m.max(5.0) {
        spec.flash_stun_delta_ext + if enclosed_room { 4 } else { 0 }
    } else {
        0
    };
    let blinded_ticks = if ordnance == ExplosiveOrdnanceId::M84FlashbangStunGrenade
        && d <= spec.frag_radius_m
        && intervening_barrier_pr == 0
    {
        if enclosed_room { 32 } else { 20 }
    } else {
        0
    };
    let causes_eardrum_tinnitus =
        blast_overpressure_trauma >= 4 || (spec.flash_stun_delta_ext >= 6 && d <= spec.frag_radius_m);

    BlastExposureEvaluation {
        ordnance,
        distance_m: d,
        enclosed_room_reflected: enclosed_room,
        blast_overpressure_trauma,
        frag_hit_count,
        frag_pen: if frag_hit_count > 0 { spec.frag_pen } else { 0 },
        frag_raw_impact: if frag_hit_count > 0 {
            spec.frag_raw_impact
        } else {
            0
        },
        stun_save_delta_ext,
        causes_eardrum_tinnitus,
        blinded_ticks,
        creates_ir_blocking_smoke_cloud,
        thermal_melt_active,
    }
}

/// Explosive or thermite tactical breach check against a physical barrier [`MaterialClass`]
/// (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExplosiveBreachEvaluation {
    pub ordnance: ExplosiveOrdnanceId,
    pub target_material: MaterialClass,
    pub barrier_pr: i32,
    pub effective_breach_pr: i32,
    pub clean_breach_created: bool,
    pub safe_standoff_m: f64,
    pub friendly_caught_in_backblast: bool,
    pub interior_defender_stun_ticks: i32,
}

/// Evaluate explosive or thermite door/wall breaching against `target_material`
/// (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §3.2).
#[must_use]
pub fn evaluate_explosive_breach(
    ordnance: ExplosiveOrdnanceId,
    target_material: MaterialClass,
    demolitions_net_successes: i32,
    friendly_standoff_m: f64,
    enclosed_interior: bool,
) -> ExplosiveBreachEvaluation {
    let spec = ordnance.spec();
    let barrier_pr = material_ballistic_profile(target_material).barrier_pr;
    let effective_breach_pr = spec.breach_pr + demolitions_net_successes.max(0);
    let clean_breach_created = effective_breach_pr >= barrier_pr;
    let safe_standoff_m = match ordnance {
        ExplosiveOrdnanceId::LinearShapedBreachingCharge => 3.0,
        ExplosiveOrdnanceId::ThermiteIncendiaryCharge => 2.0,
        ExplosiveOrdnanceId::C4SemtexBlock500g => 8.0,
        _ => 6.0,
    };
    let friendly_caught_in_backblast =
        friendly_standoff_m < safe_standoff_m && spec.peak_blast_trauma > 0;
    let interior_defender_stun_ticks = if clean_breach_created && spec.peak_blast_trauma > 0 {
        if enclosed_interior { 12 } else { 6 }
    } else {
        0
    };

    ExplosiveBreachEvaluation {
        ordnance,
        target_material,
        barrier_pr,
        effective_breach_pr,
        clean_breach_created,
        safe_standoff_m,
        friendly_caught_in_backblast,
        interior_defender_stun_ticks,
    }
}

// ============================================================================
// 4. Optics, Ballistic Computers & Tactical Field Gear (§4)
// ============================================================================

define_coded_enum! {
    /// Eight canonical optics, ballistic computers, laser aimers, and breaching field gear
    /// (`docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md` §4).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[repr(usize)]
    pub enum TacticalOpticOrGearId [count = 8] {
        MicroRedDotReflex => {
            code: "OPTIC_MICRO_RDS",
            label: "Micro Red-Dot Reflex Sight (RMR / Acro)",
            aliases: ["RED_DOT", "MICRO_RDS"],
        },
        Lpvo1To8VariableOptic => {
            code: "OPTIC_LPVO_1_8X",
            label: "1-8x24 Low-Power Variable Optic (LPVO FFP)",
            aliases: ["LPVO"],
        },
        PrecisionLongRangeScope5To25 => {
            code: "OPTIC_PRECISION_5_25X",
            label: "5-25x56 First-Focal-Plane Precision Riflescope",
            aliases: ["LONG_RANGE_SCOPE"],
        },
        SmartBallisticComputerScope2035 => {
            code: "OPTIC_2035_BALLISTIC_AI",
            label: "2035 Laser-Rangefinding Smart Ballistic Computer Scope",
            aliases: ["BALLISTIC_SCOPE", "SMART_SCOPE"],
        },
        ClipOnThermalFlirWeaponSight => {
            code: "OPTIC_CLIPON_FLIR",
            label: "Clip-On Cooled Thermal / FLIR Weapon Sight",
            aliases: ["THERMAL_SIGHT", "FLIR_CLIPON"],
        },
        IrLaserDesignatorPeq => {
            code: "GEAR_IR_PEQ_LASER",
            label: "IR Aiming Laser & Illuminator Module (PEQ/NGAL)",
            aliases: ["IR_LASER", "PEQ15"],
        },
        BipodOrShootingTripod => {
            code: "GEAR_BIPOD_TRIPOD",
            label: "Carbon Lock-Leg Bipod / Precision Shooting Tripod",
            aliases: ["BIPOD", "TRIPOD"],
        },
        HydraulicBreachingKitAndMirror => {
            code: "GEAR_HYDRAULIC_BREACH",
            label: "Compact Hydraulic Door Spreader + Under-Door Scope",
            aliases: ["HYDRAULIC_BREACHER"],
        },
    }
}

/// Mechanical specification for a tactical optic, stabilizer, or breaching kit (`arsenal_explosives_and_tactical_gear.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TacticalOpticOrGearSpec {
    pub id: TacticalOpticOrGearId,
    /// Aimed-shot or precision task modifier ($\Delta_{\text{tool}}$, negative = helps player).
    pub aim_delta_tool_mod: i32,
    /// CQB / Snap-shot modifier ($\Delta_{\text{tool}}$ at close range).
    pub cqb_snap_delta_mod: i32,
    pub recoil_reduction: i32,
    pub emp_vulnerable: bool,
    pub grants_flir: bool,
    pub mass_kg: f64,
    pub cost_usd: u32,
    pub access_class: BlackMarketRestrictionClass,
    pub notes: &'static str,
}

const TACTICAL_OPTICS_GEAR_CATALOG_ARRAY: [TacticalOpticOrGearSpec; 8] = [
    TacticalOpticOrGearSpec {
        id: TacticalOpticOrGearId::MicroRedDotReflex,
        aim_delta_tool_mod: -2,
        cqb_snap_delta_mod: -2,
        recoil_reduction: 0,
        emp_vulnerable: true,
        grants_flir: false,
        mass_kg: 0.05,
        cost_usd: 550,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
        notes: "Parallax-free single focal plane; -2 Delta_tool on Snap & medium shots.",
    },
    TacticalOpticOrGearSpec {
        id: TacticalOpticOrGearId::Lpvo1To8VariableOptic,
        aim_delta_tool_mod: -2,
        cqb_snap_delta_mod: -1,
        recoil_reduction: 0,
        emp_vulnerable: false,
        grants_flir: false,
        mass_kg: 0.58,
        cost_usd: 1_900,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
        notes: "Etched glass reticle works without battery/EMP; Switch Optic selector takes 2 Ticks.",
    },
    TacticalOpticOrGearSpec {
        id: TacticalOpticOrGearId::PrecisionLongRangeScope5To25,
        aim_delta_tool_mod: -3,
        cqb_snap_delta_mod: 4,
        recoil_reduction: 0,
        emp_vulnerable: false,
        grants_flir: false,
        mass_kg: 0.96,
        cost_usd: 3_400,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
        notes: "Eliminates mechanical sight error out to 1500m; +4 eyebox penalty in <25m CQB.",
    },
    TacticalOpticOrGearSpec {
        id: TacticalOpticOrGearId::SmartBallisticComputerScope2035,
        aim_delta_tool_mod: -4,
        cqb_snap_delta_mod: 2,
        recoil_reduction: 0,
        emp_vulnerable: true,
        grants_flir: false,
        mass_kg: 1.15,
        cost_usd: 6_800,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
        notes: "Auto-compensates range drop & crosswind via LRF/anemometer; EMP degrades to -1.",
    },
    TacticalOpticOrGearSpec {
        id: TacticalOpticOrGearId::ClipOnThermalFlirWeaponSight,
        aim_delta_tool_mod: -2,
        cqb_snap_delta_mod: 1,
        recoil_reduction: 0,
        emp_vulnerable: true,
        grants_flir: true,
        mass_kg: 0.72,
        cost_usd: 8_500,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
        notes: "Zero-hold Thermal/FLIR overlay defeats darkness/smoke unless target is thermally baffled.",
    },
    TacticalOpticOrGearSpec {
        id: TacticalOpticOrGearId::IrLaserDesignatorPeq,
        aim_delta_tool_mod: -1,
        cqb_snap_delta_mod: -2,
        recoil_reduction: 0,
        emp_vulnerable: true,
        grants_flir: false,
        mass_kg: 0.21,
        cost_usd: 2_200,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
        notes: "Passive hip-fire aim under NVGs (-2 CQB), but visible beam exposes shooter to enemy NVGs.",
    },
    TacticalOpticOrGearSpec {
        id: TacticalOpticOrGearId::BipodOrShootingTripod,
        aim_delta_tool_mod: -2,
        cqb_snap_delta_mod: 2,
        recoil_reduction: 3,
        emp_vulnerable: false,
        grants_flir: false,
        mass_kg: 0.45,
        cost_usd: 420,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
        notes: "Reduces Effective Recoil by -3 and grants -2 aim stability once braced in prone/cover.",
    },
    TacticalOpticOrGearSpec {
        id: TacticalOpticOrGearId::HydraulicBreachingKitAndMirror,
        aim_delta_tool_mod: -2,
        cqb_snap_delta_mod: 0,
        recoil_reduction: 0,
        emp_vulnerable: false,
        grants_flir: false,
        mass_kg: 7.40,
        cost_usd: 3_600,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
        notes: "Silent zero-overpressure mechanical door spread + under-door fiber inspection (-2 Observe).",
    },
];

impl_static_catalog!(TacticalOpticOrGearId => TacticalOpticOrGearSpec : TACTICAL_OPTICS_GEAR_CATALOG_ARRAY);

/// Canonical 8-entry Optics & Tactical Gear catalog.
pub const TACTICAL_OPTICS_GEAR_CATALOG: &[TacticalOpticOrGearSpec] =
    &TACTICAL_OPTICS_GEAR_CATALOG_ARRAY;

/// Return the canonical 8-entry Optics & Tactical Gear catalog.
#[must_use]
pub const fn canonical_tactical_optics_gear_catalog() -> &'static [TacticalOpticOrGearSpec] {
    TACTICAL_OPTICS_GEAR_CATALOG
}
