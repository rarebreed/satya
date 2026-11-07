//! # Health, Anatomical Trauma, Fatigue & Mental Conditions (06-health-and-survival/health_and_trauma.md)
//!
//! Canonical Rust implementation of `docs/ttrpg/06-health-and-survival/health_and_trauma.md`:
//! - **§1 Hit Locations & Called Shots (`§1.1–§1.2`)**: `1d20` Master Anatomical Hit Location Table (13 sub-zones)
//!   and Called Shot $\Delta_{\text{ext}}$ (`+2` Broad Zone waived when $C_{\text{att}} - C_{\text{def}} \ge 3$,
//!   `+4` Specific Sub-Zone/Joint, `+6` Cranium/Face/Jaw, `+8` Vital Pinpoint).
//! - **§2 Short-Term Metabolic Damage (`§2.1–§2.3`)**: Fatigue Threshold ($\text{FT} = \text{FIT} + \lfloor\text{DIS}/2\rfloor$),
//!   the 6 Fatigue Levels (`0–5`), Breath-Holding ($40\times\text{FIT}$ calm vs. $16\times\text{FIT}$ vigorous ticks),
//!   Asphyxiation/Drowning (`+1` fatigue per `20 ticks`), and Vascular Blood Chokes (`24–40 ticks`).
//! - **§3 Long-Term Structural Trauma (`§3.1–§3.4`)**: Structural Wound Severity Tiers (`Tiers 1–5`, trauma ranges
//!   `1–2`, `3–5`, `6–8`, `9–11`, `12+`), Broken Bones & Dislocations (hairline fracture aggravation, ribcage exertion,
//!   femoral/brachial arterial hazard), Hemorrhaging Grades (`Grades 0–4`) on the `10.0-Unit` Blood Volume track,
//!   Hypovolemic Shock (`Classes I–IV`), and Contested `2d20` Wound Shock Resistance (`WSR`) Consciousness Saves.
//! - **§4–§6 Status Effects, Mental Trauma & Medical Triage (`§4`, `§5.1–§5.2`, `§6.1–§6.3`)**: The Physiological/Sensory
//!   Status Conditions, the 4 Acute Mental Trauma Tiers (`OODA Freeze`, `Panic/Feral Rage`, `Dissociation`, `Catatonia`),
//!   Field Analgesics, and Weekly `HLT` Healing Recovery Cascades.
//! - **`movement_vehicles_and_environment.md`**: Implemented in and re-exported from [`crate::environment_vehicles`].

use serde::{Deserialize, Serialize};

use crate::characteristics::compute_wound_shock_resistance_score;
use crate::dice_probabilities::ContestedTieRule;
pub use crate::environment_vehicles::*;
use crate::impl_static_catalog;
pub use crate::status_and_triage::*;
use crate::primitives::{
    ConditionImpact, ConditionSource, ContestedCheck, StaticCatalog, TieredSaveEval, Triad,
};

// ============================================================================
// 1. Hit Locations & Called Shots (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.1–§1.2)
// ============================================================================

/// Precision tier for a targeted / called shot (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.1 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalledShotZoneTier {
    /// Default / Unaimed Hit (`1d20` roll on the Anatomical Hit Location Table, $\Delta_{\text{ext}} = 0$).
    UnaimedRandomD20,
    /// Broad Zone (Torso, Left/Right Leg, Left/Right Arm): $\Delta_{\text{ext}} = +2$
    /// (waived to `0` if attacker's `Control` exceeds defender's `Control` by `+3` or more).
    BroadZone,
    /// Specific Sub-Zone / Joint (Foot/Ankle, Knee, Pelvis, Shoulder, Wrist/Hand, Liver, Face/Jaw): $\Delta_{\text{ext}} = +4$.
    SpecificSubZoneOrJoint,
    /// Cranium / Face / Jaw sub-zone on the Head (`+6` $\Delta_{\text{ext}}$).
    HeadCraniumOrFace,
    /// Vital Pinpoint / Gap in Armor (Eye/Optic Slit, Carotid Artery, Cranial Ocular Box, Heart/Aorta/Spine): $\Delta_{\text{ext}} = +8$.
    VitalPinpointOrArmorGap,
}

/// Computes the External Difficulty modifier ($\Delta_{\text{ext, loc}}$) for a Called Shot
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.1 Table).
#[must_use]
pub const fn compute_called_shot_delta_ext(
    zone_tier: CalledShotZoneTier,
    attacker_control: i32,
    defender_control: i32,
) -> i32 {
    match zone_tier {
        CalledShotZoneTier::UnaimedRandomD20 => 0,
        CalledShotZoneTier::BroadZone => {
            if attacker_control - defender_control >= 3 {
                0
            } else {
                2
            }
        }
        CalledShotZoneTier::SpecificSubZoneOrJoint => 4,
        CalledShotZoneTier::HeadCraniumOrFace => 6,
        CalledShotZoneTier::VitalPinpointOrArmorGap => 8,
    }
}

/// Canonical 13 Anatomical Hit Location sub-zones (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.2 Table).
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitLocationId {
    FootAnkle = 0,
    CalfShin = 1,
    KneeJoint = 2,
    ThighFemur = 3,
    PelvisHipGroin = 4,
    AbdomenGutLiverSpleen = 5,
    ChestRibcageLungs = 6,
    VitalThoracicHeartSpineAorta = 7,
    ShoulderClavicleScapula = 8,
    UpperArmElbowForearm = 9,
    WristHandFingers = 10,
    NeckThroatCarotidCervical = 11,
    HeadCraniumFaceJawEyes = 12,
}

impl HitLocationId {
    pub const ALL: [Self; 13] = [
        Self::FootAnkle,
        Self::CalfShin,
        Self::KneeJoint,
        Self::ThighFemur,
        Self::PelvisHipGroin,
        Self::AbdomenGutLiverSpleen,
        Self::ChestRibcageLungs,
        Self::VitalThoracicHeartSpineAorta,
        Self::ShoulderClavicleScapula,
        Self::UpperArmElbowForearm,
        Self::WristHandFingers,
        Self::NeckThroatCarotidCervical,
        Self::HeadCraniumFaceJawEyes,
    ];
}

/// Canonical specification for an entry in the `1d20` Master Anatomical Hit Location Table
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.2 Table).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitLocationSpec {
    pub id: HitLocationId,
    /// Minimum `1d20` roll (`1..=20`).
    pub d20_min: i32,
    /// Maximum `1d20` roll (`1..=20`).
    pub d20_max: i32,
    pub name: &'static str,
    /// Base Called Shot $\Delta_{\text{ext}}$ (`+2`, `+4`, `+6`, or `+8`).
    pub called_shot_delta_ext: i32,
    pub slashes_tempo_cap: bool,
    pub slashes_control_cap: bool,
    pub slashes_force_cap: bool,
    pub major_arterial_hazard: bool,
    pub mechanical_notes: &'static str,
}

impl HitLocationSpec {
    #[must_use]
    pub const fn slashes_triad(&self) -> Triad<bool> {
        Triad::new(
            self.slashes_tempo_cap,
            self.slashes_control_cap,
            self.slashes_force_cap,
        )
    }
}

macro_rules! hl {
    ($id:ident, $min:expr, $max:expr, $name:literal, $ext:expr, ($t:expr, $c:expr, $f:expr), $art:expr, $notes:literal) => {
        HitLocationSpec {
            id: HitLocationId::$id,
            d20_min: $min,
            d20_max: $max,
            name: $name,
            called_shot_delta_ext: $ext,
            slashes_tempo_cap: $t,
            slashes_control_cap: $c,
            slashes_force_cap: $f,
            major_arterial_hazard: $art,
            mechanical_notes: $notes,
        }
    };
}

/// Canonical 13-entry Master Anatomical Hit Location Table (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.2 Table).
pub const HIT_LOCATION_TABLE: [HitLocationSpec; 13] = [
    hl!(FootAnkle, 1, 1, "Foot / Ankle (L or R)", 4, (true, false, false), false,
        "Mobility Anchor: Fractured tarsals or severed Achilles locks running/sprinting; -4 to Effective Root against sweeps."),
    hl!(CalfShin, 2, 3, "Calf / Shin (Tibia/Fibula) (L or R)", 2, (true, false, false), false,
        "Footwork Degradation: Muscle cramp or tibial fracture; Tier 3+ fracture prevents standing without support."),
    hl!(KneeJoint, 4, 4, "Knee Joint (Patella/ACL) (L or R)", 4, (true, false, false), false,
        "Joint Collapse: High susceptibility to Blunt/Leverage trauma (-4/-8/Locked 0 T_max & Root); Tier 2+ tear/dislocation forces immediate Prone check."),
    hl!(ThighFemur, 5, 6, "Thigh (Femur / Femoral) (L or R)", 2, (true, false, true), true,
        "Arterial Hazard: Deep Piercing/Slashing wounds risk Femoral Arterial Bleed; broken femur inflicts immediate Wound Shock +3."),
    hl!(PelvisHipGroin, 7, 7, "Pelvis / Hip Girdle / Groin", 4, (true, false, true), true,
        "Center of Mass Structural Lock: Pelvic fracture locks Tempo = 0 (cannot walk/stand) and inflicts excruciating neurogenic shock."),
    hl!(AbdomenGutLiverSpleen, 8, 10, "Abdomen / Gut / Liver / Spleen", 2, (false, false, true), false,
        "Visceral Shock & Nausea: Blunt liver/solar-plexus hit triggers Nausea / Winded; penetrating gut wound causes sepsis & internal bleed."),
    hl!(ChestRibcageLungs, 11, 13, "Chest / Ribcage / Lungs", 2, (false, false, true), false,
        "Respiratory Trauma: Cracked ribs slash Force Cap and impose -4 FIT (+1 Exertion Mark / 20 ticks); penetrating hit causes Pneumothorax."),
    hl!(VitalThoracicHeartSpineAorta, 14, 14, "Vital Thoracic (Heart / Spine / Aorta)", 8, (true, true, true), true,
        "Catastrophic Vitals: Penetrating trauma causes massive Arterial Hemorrhage; spinal hit causes immediate lower-body Paralysis."),
    hl!(ShoulderClavicleScapula, 15, 15, "Shoulder / Clavicle / Scapula (L or R)", 4, (false, true, true), true,
        "Shoulder Girdle Failure: Dislocated shoulder or broken collarbone disables lifting, two-handed bracing, and heavy swings with that arm."),
    hl!(UpperArmElbowForearm, 16, 17, "Upper Arm / Elbow / Forearm (L or R)", 2, (false, true, true), true,
        "Brachial & Lever Trauma: Slashes parry/aiming stability (Control) and striking power (Force); deep cuts risk Brachial Artery."),
    hl!(WristHandFingers, 18, 18, "Wrist / Hand / Fingers (L or R)", 4, (false, true, false), false,
        "Disarm & Manual Lock: Tier 2+ hit forces immediate FOR/DEX Disarm Check; fractured wrist/fingers prevent firing or gripping with that hand."),
    hl!(NeckThroatCarotidCervical, 19, 19, "Neck / Throat / Carotid / Cervical", 8, (true, true, true), true,
        "Lethal Vulnerability: Blunt strike crushes trachea (Asphyxiation + mutes speech ELO = 0); blade/bullet severs Carotid/Jugular or cervical spine."),
    hl!(HeadCraniumFaceJawEyes, 20, 20, "Head (Cranium / Face / Jaw / Eyes)", 6, (false, true, false), false,
        "Neurological & Sensory Apex: Blunt impact triggers Stunned / Concussed / Unconscious; eye hit causes Blinded; jaw hit impairs speech."),
];

impl_static_catalog!(HitLocationId => HitLocationSpec : HIT_LOCATION_TABLE);

#[must_use]
pub const fn canonical_hit_location_table() -> &'static [HitLocationSpec] {
    HitLocationId::CATALOG
}

/// Resolves a `1d20` roll (`1..=20`) into its canonical [`HitLocationSpec`]
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.2 Table).
#[must_use]
pub fn resolve_hit_location_d20(d20_roll: i32) -> HitLocationSpec {
    let clamped = d20_roll.clamp(1, 20);
    HIT_LOCATION_TABLE
        .iter()
        .find(|spec| clamped >= spec.d20_min && clamped <= spec.d20_max)
        .expect("1..=20 is exhaustively covered by HIT_LOCATION_TABLE")
        .clone()
}

// ============================================================================
// 2. Short-Term Metabolic Damage: Fatigue Levels (0–5) & Asphyxiation (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §2.1–§2.3)
// ============================================================================

/// Mechanical effects of a character's current Fatigue Level (`0..=5`, `docs/ttrpg/06-health-and-survival/health_and_trauma.md` §2.1–§2.2 Table).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FatigueLevelSpec {
    /// Numeric fatigue level (`0..=5`).
    pub fatigue_level: i32,
    pub condition_name: &'static str,
    /// Internal difficulty modifier $\Delta_{\text{int}} = 2 \times \text{fatigue\_level}$.
    pub delta_int: i32,
    /// Failure carry penalty $\delta_{\text{under}} = 6 + \Delta_{\text{int}}$.
    pub delta_under: i32,
    /// Tactical Pool drain $\Delta P = -2 \times \text{fatigue\_level}$.
    pub tactical_pool_drain: i32,
    /// Multiplier added to `action_time_factor` ($1.00 + 0.10 \times \text{fatigue\_level}$).
    pub action_time_factor: f64,
    /// Penalty applied to `SPD` and `FOR` aspect ceilings (`-2` at Fatigue Level `3+`, else `0`).
    pub spd_and_for_cap_penalty: i32,
    pub can_sprint: bool,
    pub is_unconscious: bool,
    pub recovery_requirement: &'static str,
}

/// Evaluates the exact mechanical penalties and physiological state for a `fatigue_level` (`0..=5`,
/// `docs/ttrpg/06-health-and-survival/health_and_trauma.md` §2.2 Table).
#[must_use]
pub fn evaluate_fatigue_level(fatigue_level: i32) -> FatigueLevelSpec {
    let lvl = fatigue_level.clamp(0, 5);
    let (condition_name, recovery_requirement) = match lvl {
        0 => ("Fresh / Rested", "None (normal baseline)"),
        1 => ("Winded / Warm", "5 minutes of sitting/hydration"),
        2 => ("Tired / Gassed", "15–30 minutes of rest + water"),
        3 => ("Exhausted", "2 hours rest + meal"),
        4 => ("Spent / Agonized", "8 hours sleep"),
        _ => ("Metabolic Collapse", "24+ hours bedrest"),
    };
    let delta_int = 2 * lvl;
    FatigueLevelSpec {
        fatigue_level: lvl,
        condition_name,
        delta_int,
        delta_under: 6 + delta_int,
        tactical_pool_drain: -2 * lvl,
        action_time_factor: 1.0 + 0.10 * f64::from(lvl),
        spd_and_for_cap_penalty: if lvl >= 3 { 2 } else { 0 },
        can_sprint: lvl < 4,
        is_unconscious: lvl >= 5,
        recovery_requirement,
    }
}

impl ConditionSource for FatigueLevelSpec {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.delta_int,
            tactical_pool_mod: self.tactical_pool_drain,
            action_time_factor_delta: self.action_time_factor - 1.0,
            triad_cap_penalty: Triad::new(
                self.spd_and_for_cap_penalty,
                0,
                self.spd_and_for_cap_penalty,
            ),
            triad_locked_zero: Triad::splat(self.is_unconscious),
            is_unconscious: self.is_unconscious,
            ..ConditionImpact::ZERO
        }
    }
}

/// Outcome of evaluating Breath-Holding, Drowning/Asphyxiation, or a Vascular Blood Choke
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsphyxiationOutcome {
    /// Maximum ticks the character can hold their breath before hypoxic fatigue accumulation begins
    /// ($40\times\text{FIT}$ calm, $16\times\text{FIT}$ vigorous, `0` if surprised without air or in a blood choke).
    pub breath_holding_capacity_ticks: i32,
    pub hypoxic_overrun_ticks: i32,
    pub fatigue_levels_gained: i32,
    /// Resulting total `fatigue_level` (`0..=5`).
    pub resulting_fatigue_level: i32,
    pub is_unconscious: bool,
    pub hypoxic_brain_death_risk: bool,
}

/// Evaluates breath-holding capacity, drowning/vacuum asphyxiation (`+1` fatigue per `20 ticks`),
/// and vascular blood chokes (`24–40 ticks` to unconsciousness)
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §2.3).
#[must_use]
pub fn evaluate_breath_holding_and_asphyxiation(
    fit: i32,
    starting_fatigue_level: i32,
    took_full_breath: bool,
    is_vigorous_activity: bool,
    elapsed_ticks: i32,
    is_vascular_blood_choke: bool,
) -> AsphyxiationOutcome {
    let fit_clamped = fit.clamp(1, 20);
    let ticks = elapsed_ticks.max(0);
    let base_fatigue = starting_fatigue_level.clamp(0, 5);

    if is_vascular_blood_choke {
        let choke_ko_ticks = (24 + (fit_clamped - 10).max(0)).clamp(24, 40);
        let is_unconscious = ticks >= choke_ko_ticks || base_fatigue >= 5;
        let fatigue_levels_gained = ticks.div_euclid(20);
        let resulting_fatigue_level = (base_fatigue + fatigue_levels_gained).min(5);
        return AsphyxiationOutcome {
            breath_holding_capacity_ticks: 0,
            hypoxic_overrun_ticks: ticks,
            fatigue_levels_gained,
            resulting_fatigue_level,
            is_unconscious,
            hypoxic_brain_death_risk: ticks >= choke_ko_ticks + 240,
        };
    }

    let breath_holding_capacity_ticks = if !took_full_breath {
        0
    } else if is_vigorous_activity {
        16 * fit_clamped
    } else {
        40 * fit_clamped
    };

    let hypoxic_overrun_ticks = (ticks - breath_holding_capacity_ticks).max(0);
    let fatigue_levels_gained = hypoxic_overrun_ticks.div_euclid(20);
    let resulting_fatigue_level = (base_fatigue + fatigue_levels_gained).min(5);
    let is_unconscious = resulting_fatigue_level >= 5;
    let ticks_to_fatigue_5 = (5 - base_fatigue).max(0) * 20;
    let ticks_unconscious = (hypoxic_overrun_ticks - ticks_to_fatigue_5).max(0);

    AsphyxiationOutcome {
        breath_holding_capacity_ticks,
        hypoxic_overrun_ticks,
        fatigue_levels_gained,
        resulting_fatigue_level,
        is_unconscious,
        hypoxic_brain_death_risk: is_unconscious && ticks_unconscious >= 240,
    }
}

// ============================================================================
// 3. Long-Term Damage: Structural Wounds, Fractures, Bleed & WSR Saves (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.1–§3.4)
// ============================================================================

/// The 5 Structural Wound Severity Tiers (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.1 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralWoundTier {
    /// **Tier 1: Superficial / Minor (`1–2` effective trauma)**: `-2` to Location's Ceiling, `+0` Shock (`+1` if `WSR < 10`), Capillary Bleed. Heals in `3–7 days`.
    Tier1Superficial,
    /// **Tier 2: Moderate Trauma (`3–5` effective trauma)**: `-5` to Location's Ceiling, `+1` Shock Tier, Venous Bleed / hairline fracture. Heals in `2–4 weeks`.
    Tier2Moderate,
    /// **Tier 3: Severe / Fracture (`6–8` effective trauma)**: `-10` (Limb Disabled), `+2` Shock Tiers, Complete Bone Fracture / Major Venous or Arterial Bleed. `6–12 weeks`.
    Tier3SevereFracture,
    /// **Tier 4: Critical / Maiming (`9–11` effective trauma)**: Ceiling Locked at `0`, `+3` Shock Tiers, Compound Fracture / Solid Organ Rupture / Arterial Severance. `3–6 months`.
    Tier4CriticalMaiming,
    /// **Tier 5: Catastrophic / Lethal (`12+` effective trauma)**: Ceiling Locked at `0` (Destroyed), `+4` Shock Tiers, Traumatic Amputation / Transected Spine / Ruptured Heart.
    Tier5CatastrophicLethal,
}

impl StructuralWoundTier {
    #[must_use]
    pub const fn from_tier_number(tier: u8) -> Self {
        match tier {
            ..=1 => Self::Tier1Superficial,
            2 => Self::Tier2Moderate,
            3 => Self::Tier3SevereFracture,
            4 => Self::Tier4CriticalMaiming,
            5.. => Self::Tier5CatastrophicLethal,
        }
    }

    #[must_use]
    pub const fn tier_number(self) -> u8 {
        match self {
            Self::Tier1Superficial => 1,
            Self::Tier2Moderate => 2,
            Self::Tier3SevereFracture => 3,
            Self::Tier4CriticalMaiming => 4,
            Self::Tier5CatastrophicLethal => 5,
        }
    }

    /// Aspect ceiling penalty ($T_{\max}, C_{\max}, F_{\max}$) inflicted on the struck location (`2`, `5`, `10`, or `20` for Locked at `0`,
    /// `docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.1 Table).
    #[must_use]
    pub const fn aspect_ceiling_penalty(self) -> i32 {
        match self {
            Self::Tier1Superficial => 2,
            Self::Tier2Moderate => 5,
            Self::Tier3SevereFracture => 10,
            Self::Tier4CriticalMaiming | Self::Tier5CatastrophicLethal => 20,
        }
    }

    /// Whether the struck location's aspect ceiling is completely locked at `0` (`Tier 4` or `Tier 5`,
    /// `docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.1 Table).
    #[must_use]
    pub const fn locks_ceiling_to_zero(self) -> bool {
        matches!(
            self,
            Self::Tier4CriticalMaiming | Self::Tier5CatastrophicLethal
        )
    }

    /// Systemic `wound_shock_index` tiers added by this wound (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.1 Table).
    #[must_use]
    pub const fn wound_shock_added(self, wsr: i32) -> i32 {
        match self {
            Self::Tier1Superficial => {
                if wsr < 10 {
                    1
                } else {
                    0
                }
            }
            Self::Tier2Moderate => 1,
            Self::Tier3SevereFracture => 2,
            Self::Tier4CriticalMaiming => 3,
            Self::Tier5CatastrophicLethal => 4,
        }
    }
}

/// Maps net penetrating/BABT trauma magnitude into a [`StructuralWoundTier`]
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.1 Table; `ranged_combat_and_armor.md` §4.2 Table).
///
/// Thresholds:
/// - `<= 0` -> `None`
/// - `1..=2` -> [`StructuralWoundTier::Tier1Superficial`]
/// - `3..=5` -> [`StructuralWoundTier::Tier2Moderate`]
/// - `6..=8` -> [`StructuralWoundTier::Tier3SevereFracture`]
/// - `9..=11` -> [`StructuralWoundTier::Tier4CriticalMaiming`]
/// - `12+` -> [`StructuralWoundTier::Tier5CatastrophicLethal`]
#[must_use]
pub const fn classify_structural_wound_tier(effective_trauma: i32) -> Option<StructuralWoundTier> {
    if effective_trauma <= 0 {
        None
    } else if effective_trauma <= 2 {
        Some(StructuralWoundTier::Tier1Superficial)
    } else if effective_trauma <= 5 {
        Some(StructuralWoundTier::Tier2Moderate)
    } else if effective_trauma <= 8 {
        Some(StructuralWoundTier::Tier3SevereFracture)
    } else if effective_trauma <= 11 {
        Some(StructuralWoundTier::Tier4CriticalMaiming)
    } else {
        Some(StructuralWoundTier::Tier5CatastrophicLethal)
    }
}

/// Evaluates whether a **Tier 2 Hairline Fracture** snaps into a **Tier 3 Complete Fracture** under load
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.2.1).
#[must_use]
pub const fn check_hairline_fracture_aggravation(
    allocated_aspect: i32,
    base_aspect_ceiling: i32,
    hlt_check_succeeded: bool,
) -> (bool, StructuralWoundTier) {
    let over_half_ceiling = base_aspect_ceiling > 0 && allocated_aspect * 2 > base_aspect_ceiling;
    if over_half_ceiling && !hlt_check_succeeded {
        (true, StructuralWoundTier::Tier3SevereFracture)
    } else {
        (false, StructuralWoundTier::Tier2Moderate)
    }
}

/// Evaluates reducing ("popping back in") a **Joint Dislocation**
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.2.2).
#[must_use]
pub const fn reduce_joint_dislocation(first_aid_succeeded: bool) -> (i32, i32) {
    (20, if first_aid_succeeded { 3 } else { 8 })
}

/// The Four Hemorrhage Grades (plus `Grade0None`) on the `10.0-Unit` Blood Volume Track
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.3 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HemorrhageGrade {
    /// **Grade 0: None / Contusion** (`0 Units`)
    Grade0None,
    /// **Grade 1: Capillary / Minor Ooze** (`0.25 Units / Hour` = `1 Unit / 57,600 Ticks`)
    Grade1Capillary,
    /// **Grade 2: Venous Hemorrhage** (`0.5 Units / Minute` = `1 Unit / 480 Ticks`)
    Grade2Venous,
    /// **Grade 3: Major Venous / Visceral** (`1.0 Unit / 20 Ticks` = `1 Unit / 5 Seconds`)
    Grade3MajorVenousVisceral,
    /// **Grade 4: Arterial Severance** (`1.0 Unit / 4 Ticks` = `1 Unit / 1 Second`)
    Grade4ArterialSeverance,
}

impl HemorrhageGrade {
    /// Blood volume lost per Tick (`1 Tick = 0.25s`, `240 Ticks = 1 min`, `14,400 Ticks = 1 hr`) in Units (`10.0 Units` baseline,
    /// `docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.3 Table).
    #[must_use]
    pub fn units_lost_per_tick(self) -> f64 {
        match self {
            Self::Grade0None => 0.0,
            Self::Grade1Capillary => 0.25 / 14_400.0,
            Self::Grade2Venous => 1.0 / 480.0,
            Self::Grade3MajorVenousVisceral => 1.0 / 20.0,
            Self::Grade4ArterialSeverance => 1.0 / 4.0,
        }
    }

    /// Whether this hemorrhage grade can slow or clot naturally without hemostatic packing/tourniquet/surgery (`Grades 0–2`,
    /// `docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.3 Table).
    #[must_use]
    pub const fn can_clot_naturally(self) -> bool {
        matches!(
            self,
            Self::Grade0None | Self::Grade1Capillary | Self::Grade2Venous
        )
    }

    /// Ticks required to apply standard field hemorrhage control (`8` bandage, `20` pressure dressing, `24` CAT tourniquet,
    /// `docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.3 Table).
    #[must_use]
    pub const fn field_stabilization_ticks(self) -> i32 {
        match self {
            Self::Grade0None => 0,
            Self::Grade1Capillary => 8,
            Self::Grade2Venous | Self::Grade3MajorVenousVisceral => 20,
            Self::Grade4ArterialSeverance => 24,
        }
    }
}

/// Clinical Hypovolemic Shock Classes (`Classes I–IV`) from cumulative blood volume lost
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.3 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypovolemicShockClass {
    /// `< 1.0 Units Lost` (`< 10%`): Euvolemic / Compensated
    None,
    /// `1.0–1.99 Units Lost` (`10–15%` Blood Loss): Mild tachycardia; `+1` `fatigue_level`.
    ClassIMild,
    /// `2.0–3.49 Units Lost` (`20–30%` Blood Loss): Pallor, anxiety; `+1` `wound_shock_index`, `+2` `fatigue_level`, `Dizzy` (`-2 KIN`).
    ClassIIModerate,
    /// `3.5–4.99 Units Lost` (`35–45%` Blood Loss): Severe hypotension; `+3` `wound_shock_index`, `+4` `fatigue_level`, roll `HLT/DIS` every `20 ticks` or `Unconscious`.
    ClassIIISevere,
    /// `5.0+ Units Lost` (`> 50%` Blood Loss): Immediate `Unconsciousness & Coma`; fatal cardiac arrest within `240 Ticks` (`1 minute`).
    ClassIVLethalComa,
}

/// Complete state of a character's `10.0-Unit` Blood Volume Track after advancing by `elapsed_ticks`
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BloodLossProgressionOutcome {
    /// Total cumulative blood units lost (`0.0..=10.0`).
    pub total_units_lost: f64,
    /// Remaining circulating blood volume (`0.0..=10.0`, baseline `10.0`).
    pub remaining_blood_units: f64,
    pub hypovolemic_class: HypovolemicShockClass,
    pub added_fatigue_level: i32,
    pub added_wound_shock_index: i32,
    /// `KIN` penalty from orthostatic dizziness (`-2` in Class II+).
    pub dizzy_kin_penalty: i32,
    pub requires_consciousness_check_every_20_ticks: bool,
    pub immediate_unconsciousness_and_coma: bool,
    pub ticks_until_cardiac_arrest: Option<i32>,
}

impl ConditionSource for BloodLossProgressionOutcome {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            fatigue_added: self.added_fatigue_level,
            wound_shock_added: self.added_wound_shock_index,
            triad_locked_zero: Triad::splat(self.immediate_unconsciousness_and_coma),
            is_unconscious: self.immediate_unconsciousness_and_coma,
            ..ConditionImpact::ZERO
        }
    }
}

/// Advances blood loss over `elapsed_ticks` across active [`HemorrhageGrade`] sources and evaluates
/// Hypovolemic Shock progression (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.3).
#[must_use]
pub fn evaluate_blood_loss_progression(
    prior_units_lost: f64,
    active_bleeds: &[HemorrhageGrade],
    elapsed_ticks: i32,
) -> BloodLossProgressionOutcome {
    let ticks = elapsed_ticks.max(0);
    let rate_per_tick: f64 = active_bleeds.iter().map(|g| g.units_lost_per_tick()).sum();
    let total_units_lost = (prior_units_lost.max(0.0) + rate_per_tick * f64::from(ticks)).min(10.0);
    let remaining_blood_units = (10.0 - total_units_lost).max(0.0);
    let has_arterial_severance = active_bleeds.contains(&HemorrhageGrade::Grade4ArterialSeverance);

    let (
        hypovolemic_class,
        added_fatigue_level,
        added_wound_shock_index,
        dizzy_kin_penalty,
        requires_consciousness_check_every_20_ticks,
        mut immediate_unconsciousness_and_coma,
        ticks_until_cardiac_arrest,
    ) = if total_units_lost >= 5.0 {
        (
            HypovolemicShockClass::ClassIVLethalComa,
            5,
            5,
            2,
            false,
            true,
            Some(240),
        )
    } else if total_units_lost >= 3.5 {
        (
            HypovolemicShockClass::ClassIIISevere,
            4,
            3,
            2,
            true,
            false,
            None,
        )
    } else if total_units_lost >= 2.0 {
        (
            HypovolemicShockClass::ClassIIModerate,
            2,
            1,
            2,
            false,
            false,
            None,
        )
    } else if total_units_lost >= 1.0 {
        (
            HypovolemicShockClass::ClassIMild,
            1,
            0,
            0,
            false,
            false,
            None,
        )
    } else {
        (HypovolemicShockClass::None, 0, 0, 0, false, false, None)
    };

    if has_arterial_severance && ticks >= 20 {
        immediate_unconsciousness_and_coma = true;
    }

    BloodLossProgressionOutcome {
        total_units_lost,
        remaining_blood_units,
        hypovolemic_class,
        added_fatigue_level,
        added_wound_shock_index,
        dizzy_kin_penalty,
        requires_consciousness_check_every_20_ticks,
        immediate_unconsciousness_and_coma,
        ticks_until_cardiac_arrest,
    }
}

/// Domain-specific Wound Shock Consciousness Save effect payload
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WoundShockSaveEffect {
    /// Derived Wound Shock Resistance $\text{WSR} = \text{HLT} + \lfloor\text{DIS}/2\rfloor$.
    pub wsr: i32,
    /// Composite controlling characteristic $C_P = \lfloor(\text{HLT} + \text{DIS})/2\rfloor$.
    pub controller_c_p: i32,
    /// Player Target Number $\text{TN}_P = 31 - C_P$.
    pub tn_player: i32,
    /// External difficulty $\Delta_{\text{ext}} = 2 \times \text{wound\_shock\_index}$.
    pub delta_ext: i32,
    pub effective_wound_shock_index: i32,
    pub stays_conscious: bool,
    pub stunned_ticks: i32,
    pub drops_prone: bool,
    pub triggers_morale_check: bool,
    pub falls_unconscious: bool,
}

/// Outcome of a Contested `2d20` **Systemic Wound Shock (`WSR`) Consciousness Save**
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.4).
pub type WoundShockSaveOutcome = ContestedCheck<WoundShockSaveEffect>;

/// Evaluates a Contested `2d20` **Systemic Wound Shock (`WSR`) Consciousness Save**
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.4).
#[must_use]
pub fn evaluate_wound_shock_save(
    hlt: i32,
    dis: i32,
    pairs_n: usize,
    wound_shock_index: i32,
    delta_int: i32,
    player_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
) -> WoundShockSaveOutcome {
    let hlt_c = hlt.clamp(1, 20);
    let dis_c = dis.clamp(1, 20);
    let wsr = compute_wound_shock_resistance_score(hlt_c, dis_c);
    let controller_c_p = (hlt_c + dis_c).div_euclid(2);
    let shock = wound_shock_index.clamp(0, 5);
    let delta_ext = 2 * shock;

    let eval = TieredSaveEval::evaluate(
        pairs_n.max(1),
        controller_c_p,
        delta_int,
        0,
        delta_ext,
        ContestedTieRule::HybridSignedMargin,
        player_pairs,
        gm_pair,
    );
    let tn_player = eval.player_tn;

    let (
        effective_wound_shock_index,
        stays_conscious,
        stunned_ticks,
        drops_prone,
        triggers_morale_check,
        falls_unconscious,
    ) = eval.classify_4way(
        |_| ((shock - 1).max(0), true, 0, false, false, false),
        |r| {
            let eff_shock = if r.net_successes >= 2 {
                (shock - 1).max(0)
            } else {
                shock
            };
            (eff_shock, true, 0, false, false, false)
        },
        |_| {
            if shock >= 4 {
                (shock, false, 0, true, false, true)
            } else {
                (shock, true, 8, true, true, false)
            }
        },
        |_| (shock, false, 0, true, false, true),
    );

    ContestedCheck::new(
        eval,
        WoundShockSaveEffect {
            wsr,
            controller_c_p,
            tn_player,
            delta_ext,
            effective_wound_shock_index,
            stays_conscious,
            stunned_ticks,
            drops_prone,
            triggers_morale_check,
            falls_unconscious,
        },
    )
}
