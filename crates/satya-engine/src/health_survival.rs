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
    /// `1d20 = 1`: Foot / Ankle (L or R)
    FootAnkle = 0,
    /// `1d20 = 2–3`: Calf / Shin (Tibia/Fibula) (L or R)
    CalfShin = 1,
    /// `1d20 = 4`: Knee Joint (Patella/ACL) (L or R)
    KneeJoint = 2,
    /// `1d20 = 5–6`: Thigh (Femur / Femoral) (L or R)
    ThighFemur = 3,
    /// `1d20 = 7`: Pelvis / Hip Girdle / Groin
    PelvisHipGroin = 4,
    /// `1d20 = 8–10`: Abdomen / Gut / Liver / Spleen
    AbdomenGutLiverSpleen = 5,
    /// `1d20 = 11–13`: Chest / Ribcage / Lungs
    ChestRibcageLungs = 6,
    /// `1d20 = 14`: Vital Thoracic (Heart / Spine / Aorta)
    VitalThoracicHeartSpineAorta = 7,
    /// `1d20 = 15`: Shoulder / Clavicle / Scapula (L or R)
    ShoulderClavicleScapula = 8,
    /// `1d20 = 16–17`: Upper Arm / Elbow / Forearm (L or R)
    UpperArmElbowForearm = 9,
    /// `1d20 = 18`: Wrist / Hand / Fingers (L or R)
    WristHandFingers = 10,
    /// `1d20 = 19`: Neck / Throat / Carotid / Cervical
    NeckThroatCarotidCervical = 11,
    /// `1d20 = 20`: Head (Cranium / Face / Jaw / Eyes)
    HeadCraniumFaceJawEyes = 12,
}

impl HitLocationId {
    /// All 13 canonical hit location identifiers in declaration order (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.2 Table).
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
    /// Canonical hit location identifier.
    pub id: HitLocationId,
    /// Minimum `1d20` roll (`1..=20`).
    pub d20_min: i32,
    /// Maximum `1d20` roll (`1..=20`).
    pub d20_max: i32,
    /// Display name from §1.2.
    pub name: &'static str,
    /// Base Called Shot $\Delta_{\text{ext}}$ (`+2`, `+4`, `+6`, or `+8`).
    pub called_shot_delta_ext: i32,
    /// Whether this location slashes **Tempo Cap ($T_{\max}$)**.
    pub slashes_tempo_cap: bool,
    /// Whether this location slashes **Control Cap ($C_{\max}$)**.
    pub slashes_control_cap: bool,
    /// Whether this location slashes **Force Cap ($F_{\max}$)**.
    pub slashes_force_cap: bool,
    /// Whether deep penetrating trauma to this location risks major arterial hemorrhage (Femoral, Brachial, Carotid, Aorta/Heart).
    pub major_arterial_hazard: bool,
    /// Summary of specific anatomical and mechanical effects from §1.2.
    pub mechanical_notes: &'static str,
}

impl HitLocationSpec {
    /// Returns the `Tempo / Control / Force` aspect ceiling slash flags as a [`Triad<bool>`]
    /// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.2 Table).
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

/// Returns the canonical 13-entry Master Anatomical Hit Location Table
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §1.2 Table).
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
    /// Canonical condition name (`"Fresh / Rested"`, `"Winded / Warm"`, etc.).
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
    /// Whether the character is still capable of All-Out Sprinting (`false` at Fatigue `4+`).
    pub can_sprint: bool,
    /// Whether the character collapses into Metabolic Unconsciousness (`true` at Fatigue `5`).
    pub is_unconscious: bool,
    /// Required rest & hydration duration to recover `-1` level.
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
    /// Ticks elapsed beyond breath-holding capacity.
    pub hypoxic_overrun_ticks: i32,
    /// Additional `fatigue_level` steps gained from oxygen deprivation (`+1` per `20 ticks` past breath capacity).
    pub fatigue_levels_gained: i32,
    /// Resulting total `fatigue_level` (`0..=5`).
    pub resulting_fatigue_level: i32,
    /// Whether the character has lost consciousness from `Fatigue 5` or a Vascular Blood Choke (`>= 24 ticks` minimum, `32 ticks` typical).
    pub is_unconscious: bool,
    /// Whether irreversible hypoxic brain death has occurred (`>= 240 ticks` / `1 minute` after hypoxic unconsciousness).
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
    /// Converts a numeric wound tier (`1..=5`) into [`StructuralWoundTier`] (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.1 Table).
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

    /// Numeric tier (`1..=5`, `docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.1 Table).
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
    /// Current clinical Hypovolemic Shock classification.
    pub hypovolemic_class: HypovolemicShockClass,
    /// Additional `fatigue_level` imposed by blood loss (`0`, `1`, `2`, `4`, or `5`).
    pub added_fatigue_level: i32,
    /// Additional `wound_shock_index` imposed by blood loss (`0`, `1`, `3`, or `5`).
    pub added_wound_shock_index: i32,
    /// `KIN` penalty from orthostatic dizziness (`-2` in Class II+).
    pub dizzy_kin_penalty: i32,
    /// Whether the character must roll `HLT/DIS` every `20 ticks` (`5s`) to stay conscious (`Class III`).
    pub requires_consciousness_check_every_20_ticks: bool,
    /// Whether the character is immediately `Unconscious & Comatose` (`Class IV`, `>= 5.0 Units` lost, or Grade 4 Arterial `>= 20 ticks`).
    pub immediate_unconsciousness_and_coma: bool,
    /// Remaining ticks until fatal cardiac arrest once `Class IV` (`>= 5.0 Units` lost) is reached (`240 Ticks = 1 minute`).
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
    /// Effective `wound_shock_index` after applying `-1` reduction on `ExtraordinarySuccess`.
    pub effective_wound_shock_index: i32,
    /// Whether the character remains conscious.
    pub stays_conscious: bool,
    /// Ticks the character is `Stunned` (`8 ticks` on `CleanFailure`, `0` on success).
    pub stunned_ticks: i32,
    /// Whether the character is forced `Prone` (`true` on any failure).
    pub drops_prone: bool,
    /// Whether the character must make an immediate **Morale Check** (`true` on `CleanFailure`).
    pub triggers_morale_check: bool,
    /// Whether neurogenic syncope knocks the character **Unconscious** (`BotchFailure` or any failure at `wound_shock_index >= 4`).
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

// ============================================================================
// 4. Physiological, Sensory & Acute Mental Status Conditions (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §4 – §6.3)
// ============================================================================

/// Canonical Acute Physiological & Sensory Status Conditions (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §4 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysiologicalStatusCondition {
    /// **`Dazzled` / `Blurred Vision`**: `SIG` `-4` to `-6` (default `-5`), Ranged `Control Cap -3`, `+2` $\Delta_{\text{ext}}$ on visual tasks.
    DazzledBlurredVision,
    /// **`Blinded`**: `SIG = 0`, Ranged `Control = 0`, Melee `Control Cap` halved, `-4` `Tempo Cap`.
    Blinded,
    /// **`Tinnitus`**: `HEA -6`, `+2` penalty to `OODA` Initiative.
    Tinnitus,
    /// **`Deafened`**: `HEA = 0`, cannot hear callouts/footsteps, `+2` penalty to `OODA` Initiative.
    Deafened,
    /// **`Stunned` / `Dazed`**: Advances clock by `+4..=+12` ticks (`8` default) with `Guard = 0`.
    StunnedDazed,
    /// **`Concussed`**: `-4` to `ANA`, `FOC`, `INS`, `MEM` (and $D_{\text{cog}}$) and `+0.25` `action_time_factor`.
    Concussed,
    /// **`Dizzy` / `Vertigo` / `Disoriented`**: `KIN -6`, `Effective Root` halved; moving faster than a walk requires `KIN` ($\Delta_{\text{ext}} = +4$) or Prone.
    DizzyVertigo,
    /// **`Nausea` / `Sickened` / `Retching`**: `+4` $\Delta_{\text{int}}$, `FIT` halved; failed `HLT` check under exertion spends `12 ticks` retching (`T=0, C=0`).
    NauseaSickened,
    /// **`Pinned` / `Immobilized`**: `Tempo = 0` (cannot translate in space).
    PinnedImmobilized,
    /// **`Paralyzed`**: `Tempo = 0`, `Control = 0`, `Force = 0` for affected limbs (conscious, `AWR > 0`).
    Paralyzed,
    /// **`Unconscious` / `Comatose`**: `Tempo = 0, Control = 0, Force = 0, AWR = 0`, collapses Prone.
    UnconsciousComatose,
}

/// Aggregated characteristic, ceiling, and clock modifiers from active [`PhysiologicalStatusCondition`]s
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §4 Table).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregatedStatusEffects {
    /// Whether `Sight (SIG)` is locked to `0` (`Blinded`).
    pub sig_locked_zero: bool,
    /// Subtractive penalty to `SIG` (`-5` when `Dazzled`).
    pub sig_penalty: i32,
    /// Whether `Hearing (HEA)` is locked to `0` (`Deafened`).
    pub hea_locked_zero: bool,
    /// Subtractive penalty to `HEA` (`-6` when `Tinnitus`).
    pub hea_penalty: i32,
    /// Subtractive penalty to `OODA` Initiative (`-2` from `Tinnitus` / `Deafened`).
    pub ooda_penalty: i32,
    /// Subtractive penalty to `ANA`, `FOC`, `INS`, `MEM` (`-4` when `Concussed`).
    pub cognitive_stat_penalty: i32,
    /// Subtractive penalty to `KIN` (`-6` when `Dizzy/Vertigo`).
    pub kin_penalty: i32,
    /// Whether `Fitness (FIT)` is halved (`Nausea/Sickened`).
    pub fit_halved: bool,
    /// Whether `Effective Root` is halved (`Dizzy/Vertigo`).
    pub effective_root_halved: bool,
    /// Additional Internal Difficulty modifier $\Delta_{\text{int}}$ (`+4` when `Nausea/Sickened`).
    pub delta_int_added: i32,
    /// Additional `action_time_factor` penalty (`+0.25` when `Concussed`).
    pub action_time_factor_added: f64,
    /// Immediate clock freeze in ticks (`+8` ticks when `Stunned`).
    pub clock_freeze_ticks: i32,
    /// Subtractive penalty to `Tempo Cap ($T_{\max}$)` (`-4` when `Blinded`).
    pub tempo_cap_penalty: i32,
    /// Subtractive penalty to Ranged `Control Cap` (`-3` when `Dazzled`).
    pub ranged_control_cap_penalty: i32,
    /// Whether Melee `Control Cap` is halved (`Blinded`).
    pub melee_control_cap_halved: bool,
    /// Whether `Tempo` is locked at `0` (`Pinned`, `Paralyzed`, or `Unconscious`).
    pub tempo_locked_zero: bool,
    /// Whether `Control` and `Force` are locked at `0` (`Paralyzed` or `Unconscious`).
    pub control_and_force_locked_zero: bool,
    /// Whether `Awareness (AWR)` is locked at `0` and the character is unconscious (`Unconscious/Comatose`, distinct from conscious `Paralyzed`).
    pub awr_locked_zero: bool,
}

impl AggregatedStatusEffects {
    /// Converts this status aggregation into a [`ConditionImpact`] primitive
    /// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §4 Table).
    #[must_use]
    pub const fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.delta_int_added,
            delta_ext: 0,
            tactical_pool_mod: 0,
            action_time_factor_delta: self.action_time_factor_added,
            fatigue_added: 0,
            fatigue_floor: 0,
            wound_shock_added: 0,
            clock_freeze_ticks: self.clock_freeze_ticks,
            triad_cap_penalty: Triad::new(
                self.tempo_cap_penalty,
                self.ranged_control_cap_penalty,
                0,
            ),
            triad_locked_zero: Triad::new(
                self.tempo_locked_zero,
                self.control_and_force_locked_zero,
                self.control_and_force_locked_zero,
            ),
            is_unconscious: self.awr_locked_zero,
        }
    }
}

impl ConditionSource for AggregatedStatusEffects {
    fn to_condition_impact(&self) -> ConditionImpact {
        Self::to_condition_impact(self)
    }
}

/// Evaluates the combined mechanical impact of a slice of [`PhysiologicalStatusCondition`]s
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §4 Table).
#[must_use]
pub fn evaluate_physiological_status_conditions(
    conditions: &[PhysiologicalStatusCondition],
) -> AggregatedStatusEffects {
    let mut out = AggregatedStatusEffects {
        sig_locked_zero: false,
        sig_penalty: 0,
        hea_locked_zero: false,
        hea_penalty: 0,
        ooda_penalty: 0,
        cognitive_stat_penalty: 0,
        kin_penalty: 0,
        fit_halved: false,
        effective_root_halved: false,
        delta_int_added: 0,
        action_time_factor_added: 0.0,
        clock_freeze_ticks: 0,
        tempo_cap_penalty: 0,
        ranged_control_cap_penalty: 0,
        melee_control_cap_halved: false,
        tempo_locked_zero: false,
        control_and_force_locked_zero: false,
        awr_locked_zero: false,
    };

    for &cond in conditions {
        match cond {
            PhysiologicalStatusCondition::DazzledBlurredVision => {
                out.sig_penalty = out.sig_penalty.max(5);
                out.ranged_control_cap_penalty = out.ranged_control_cap_penalty.max(3);
            }
            PhysiologicalStatusCondition::Blinded => {
                out.sig_locked_zero = true;
                out.melee_control_cap_halved = true;
                out.tempo_cap_penalty = out.tempo_cap_penalty.max(4);
            }
            PhysiologicalStatusCondition::Tinnitus => {
                out.hea_penalty = out.hea_penalty.max(6);
                out.ooda_penalty = out.ooda_penalty.max(2);
            }
            PhysiologicalStatusCondition::Deafened => {
                out.hea_locked_zero = true;
                out.ooda_penalty = out.ooda_penalty.max(2);
            }
            PhysiologicalStatusCondition::StunnedDazed => {
                out.clock_freeze_ticks = out.clock_freeze_ticks.max(8);
            }
            PhysiologicalStatusCondition::Concussed => {
                out.cognitive_stat_penalty = out.cognitive_stat_penalty.max(4);
                out.action_time_factor_added = out.action_time_factor_added.max(0.25);
            }
            PhysiologicalStatusCondition::DizzyVertigo => {
                out.kin_penalty = out.kin_penalty.max(6);
                out.effective_root_halved = true;
            }
            PhysiologicalStatusCondition::NauseaSickened => {
                out.delta_int_added = out.delta_int_added.max(4);
                out.fit_halved = true;
            }
            PhysiologicalStatusCondition::PinnedImmobilized => {
                out.tempo_locked_zero = true;
            }
            PhysiologicalStatusCondition::Paralyzed => {
                out.tempo_locked_zero = true;
                out.control_and_force_locked_zero = true;
            }
            PhysiologicalStatusCondition::UnconsciousComatose => {
                out.tempo_locked_zero = true;
                out.control_and_force_locked_zero = true;
                out.awr_locked_zero = true;
            }
        }
    }

    out
}

/// The 4 Acute Mental Trauma Conditions (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §5.1 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcuteMentalTraumaTier {
    /// **Tier 1: Acute Mental Shock (`OODA Freeze`)**: Freezes `6–12 Ticks` (`1.5–3.0s`), passive `Control` capped at baseline $B$.
    Tier1AcuteMentalShockOodaFreeze,
    /// **Tier 2: Panic / Fight-or-Flight Override**: Pool `-4`, `ANA` and `FOC` `-6`; `100%` `Pure Tempo` flight or blind Feral Rage (`Control <= P_eff / 4`) if cornered with high `AGG`.
    Tier2PanicFightOrFlight,
    /// **Tier 3: Dissociation / Shell Shock ("Thousand-Yard Stare")**: Immune to further Fear/Panic, `-6` to `ITU`, `INS`, `FOC`, `ELO`, `+0.30` `action_time_factor`.
    Tier3DissociationShellShock,
    /// **Tier 4: Catatonia (Psychomotor Shutdown)**: Total lock (`Tempo = 0, Control = 0, Force = 0, ELO = 0`).
    Tier4Catatonia,
}

/// Mechanical evaluation of an [`AcuteMentalTraumaTier`]
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §5.1–§5.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcuteMentalTraumaEffect {
    /// Mental trauma severity tier (`1..=4`).
    pub tier: AcuteMentalTraumaTier,
    /// Action clock freeze in ticks (`6..=12`, midpoint `9` for Tier 1).
    pub ooda_freeze_ticks: i32,
    /// Tactical Pool modifier (`-4` in Tier 2 Panic).
    pub tactical_pool_modifier: i32,
    /// Subtractive penalty to `ANA` and `FOC` (`-6` in Tier 2, `0` in Tier 3).
    pub ana_foc_penalty: i32,
    /// Subtractive penalty to `ITU`, `INS`, `FOC`, and `ELO` (`-6` in Tier 3).
    pub dissociation_stat_penalty: i32,
    /// Additional `action_time_factor` penalty (`+0.30` in Tier 3).
    pub action_time_factor_added: f64,
    /// Whether the character shifts into cornered **Feral Rage** (`Control <= P_eff / 4`) instead of fleeing (`Tier 2` + `is_cornered && agg >= 13`).
    pub shifts_into_feral_rage: bool,
    /// Whether all physical and verbal output (`Tempo, Control, Force, ELO`) is locked at `0` (`Tier 4 Catatonia`).
    pub total_psychomotor_shutdown: bool,
    /// Permanent **Psyche Drift Marks** inflicted on the relevant `Psyche` axis upon surviving (`2` marks for Tier 3 or Tier 4, `§5.2`).
    pub permanent_psyche_drift_marks: i32,
    /// Situational **Trauma Trigger** morale penalty (`+4` $\Delta_{\text{morale}}$ for Tier 3 or Tier 4, `§5.2`).
    pub trauma_trigger_delta_morale: i32,
}

/// Evaluates the mechanical effects and long-term psychological scars of an [`AcuteMentalTraumaTier`]
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §5.1 Table, §5.2).
#[must_use]
pub fn evaluate_acute_mental_trauma(
    tier: AcuteMentalTraumaTier,
    is_cornered: bool,
    agg_score: i32,
) -> AcuteMentalTraumaEffect {
    let (
        ooda_freeze_ticks,
        tactical_pool_modifier,
        ana_foc_penalty,
        dissociation_stat_penalty,
        action_time_factor_added,
        shifts_into_feral_rage,
        total_psychomotor_shutdown,
        permanent_psyche_drift_marks,
        trauma_trigger_delta_morale,
    ) = match tier {
        AcuteMentalTraumaTier::Tier1AcuteMentalShockOodaFreeze => {
            (9, 0, 0, 0, 0.0, false, false, 0, 0)
        }
        AcuteMentalTraumaTier::Tier2PanicFightOrFlight => {
            (0, -4, 6, 0, 0.0, is_cornered && agg_score >= 13, false, 0, 0)
        }
        AcuteMentalTraumaTier::Tier3DissociationShellShock => {
            (0, 0, 0, 6, 0.30, false, false, 2, 4)
        }
        AcuteMentalTraumaTier::Tier4Catatonia => (0, 0, 0, 0, 0.0, false, true, 2, 4),
    };
    AcuteMentalTraumaEffect {
        tier,
        ooda_freeze_ticks,
        tactical_pool_modifier,
        ana_foc_penalty,
        dissociation_stat_penalty,
        action_time_factor_added,
        shifts_into_feral_rage,
        total_psychomotor_shutdown,
        permanent_psyche_drift_marks,
        trauma_trigger_delta_morale,
    }
}

impl ConditionSource for AcuteMentalTraumaEffect {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            tactical_pool_mod: self.tactical_pool_modifier,
            action_time_factor_delta: self.action_time_factor_added,
            clock_freeze_ticks: self.ooda_freeze_ticks,
            triad_locked_zero: Triad::splat(self.total_psychomotor_shutdown),
            is_unconscious: self.total_psychomotor_shutdown,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates administering field analgesics (morphine/ketamine/fentanyl lozenge)
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §6.1–§6.2).
#[must_use]
pub const fn apply_field_analgesic(
    current_wound_shock_index: i32,
    strong_dose: bool,
) -> (i32, i32) {
    let shock_reduction = if strong_dose { 2 } else { 1 };
    let sedation_stat_penalty = if strong_dose { 4 } else { 2 };
    let raw_shock = current_wound_shock_index - shock_reduction;
    (
        if raw_shock < 0 { 0 } else { raw_shock },
        sedation_stat_penalty,
    )
}

/// Estimates healing recovery days for a stabilized [`StructuralWoundTier`]
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §6.3).
#[must_use]
pub const fn evaluate_wound_recovery_timeline_days(
    tier: StructuralWoundTier,
    hlt_recovery_successes: i32,
) -> (i32, i32, i32) {
    let succ = if hlt_recovery_successes > 0 {
        hlt_recovery_successes
    } else {
        0
    };
    match tier {
        StructuralWoundTier::Tier1Superficial => {
            let adjusted = 7 - succ;
            (3, 7, if adjusted < 3 { 3 } else { adjusted })
        }
        StructuralWoundTier::Tier2Moderate => {
            let adjusted = 28 - 2 * succ;
            (14, 28, if adjusted < 14 { 14 } else { adjusted })
        }
        StructuralWoundTier::Tier3SevereFracture => {
            let adjusted = 84 - 4 * succ;
            (42, 84, if adjusted < 42 { 42 } else { adjusted })
        }
        StructuralWoundTier::Tier4CriticalMaiming => {
            let adjusted = 180 - 7 * succ;
            (90, 180, if adjusted < 90 { 90 } else { adjusted })
        }
        StructuralWoundTier::Tier5CatastrophicLethal => (180, 365, 365),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hit_locations_and_called_shots() {
        let table = canonical_hit_location_table();
        assert_eq!(table.len(), 13);
        assert_eq!(resolve_hit_location_d20(1).id, HitLocationId::FootAnkle);
        assert_eq!(resolve_hit_location_d20(4).id, HitLocationId::KneeJoint);
        assert_eq!(
            resolve_hit_location_d20(14).id,
            HitLocationId::VitalThoracicHeartSpineAorta
        );
        assert_eq!(
            resolve_hit_location_d20(20).id,
            HitLocationId::HeadCraniumFaceJawEyes
        );
        assert_eq!(compute_called_shot_delta_ext(CalledShotZoneTier::BroadZone, 12, 9), 0);
        assert_eq!(compute_called_shot_delta_ext(CalledShotZoneTier::BroadZone, 11, 9), 2);
        assert_eq!(
            compute_called_shot_delta_ext(CalledShotZoneTier::SpecificSubZoneOrJoint, 14, 8),
            4
        );
        assert_eq!(
            compute_called_shot_delta_ext(CalledShotZoneTier::VitalPinpointOrArmorGap, 14, 8),
            8
        );
    }

    #[test]
    fn test_fatigue_levels_and_asphyxiation() {
        let f0 = evaluate_fatigue_level(0);
        assert_eq!((f0.delta_int, f0.delta_under), (0, 6));
        assert!(f0.can_sprint);

        let f3 = evaluate_fatigue_level(3);
        assert_eq!(
            (f3.delta_int, f3.delta_under, f3.tactical_pool_drain, f3.spd_and_for_cap_penalty),
            (6, 12, -6, 2)
        );
        assert!(f3.can_sprint);

        let f4 = evaluate_fatigue_level(4);
        assert!(!f4.can_sprint && !f4.is_unconscious);
        assert!(evaluate_fatigue_level(5).is_unconscious);

        let calm = evaluate_breath_holding_and_asphyxiation(10, 0, true, false, 400, false);
        assert_eq!((calm.breath_holding_capacity_ticks, calm.fatigue_levels_gained), (400, 0));
        assert!(!calm.is_unconscious);

        let vig = evaluate_breath_holding_and_asphyxiation(10, 0, true, true, 260, false);
        assert_eq!((vig.breath_holding_capacity_ticks, vig.fatigue_levels_gained), (160, 5));
        assert!(vig.is_unconscious);

        let choke = evaluate_breath_holding_and_asphyxiation(10, 0, true, true, 28, true);
        assert!(choke.is_unconscious);
    }

    #[test]
    fn test_wounds_fractures_hemorrhage_and_wsr_saves() {
        assert_eq!(classify_structural_wound_tier(0), None);
        assert_eq!(
            classify_structural_wound_tier(1),
            Some(StructuralWoundTier::Tier1Superficial)
        );
        assert_eq!(
            classify_structural_wound_tier(2),
            Some(StructuralWoundTier::Tier1Superficial)
        );
        assert_eq!(
            classify_structural_wound_tier(3),
            Some(StructuralWoundTier::Tier2Moderate)
        );
        assert_eq!(
            classify_structural_wound_tier(5),
            Some(StructuralWoundTier::Tier2Moderate)
        );
        assert_eq!(
            classify_structural_wound_tier(6),
            Some(StructuralWoundTier::Tier3SevereFracture)
        );
        assert_eq!(
            classify_structural_wound_tier(8),
            Some(StructuralWoundTier::Tier3SevereFracture)
        );
        assert_eq!(
            classify_structural_wound_tier(9),
            Some(StructuralWoundTier::Tier4CriticalMaiming)
        );
        assert_eq!(
            classify_structural_wound_tier(11),
            Some(StructuralWoundTier::Tier4CriticalMaiming)
        );
        assert_eq!(
            classify_structural_wound_tier(12),
            Some(StructuralWoundTier::Tier5CatastrophicLethal)
        );

        let (snapped, new_tier) = check_hairline_fracture_aggravation(7, 12, false);
        assert!(snapped && new_tier == StructuralWoundTier::Tier3SevereFracture);

        let bleed =
            evaluate_blood_loss_progression(0.0, &[HemorrhageGrade::Grade4ArterialSeverance], 20);
        assert!((bleed.total_units_lost - 5.0).abs() < 1e-6);
        assert_eq!(bleed.hypovolemic_class, HypovolemicShockClass::ClassIVLethalComa);
        assert!(bleed.immediate_unconsciousness_and_coma);
        assert_eq!(bleed.ticks_until_cardiac_arrest, Some(240));

        let save_win = evaluate_wound_shock_save(12, 12, 2, 2, 0, &[(12, 10), (12, 10)], (8, 7));
        assert!(save_win.stays_conscious && save_win.effective_wound_shock_index == 1);

        let save_fail = evaluate_wound_shock_save(10, 10, 1, 2, 0, &[(10, 9)], (10, 9));
        assert!(
            save_fail.stays_conscious
                && save_fail.stunned_ticks == 8
                && save_fail.drops_prone
                && save_fail.triggers_morale_check
        );

        let save_botch = evaluate_wound_shock_save(10, 10, 1, 2, 0, &[(5, 5)], (10, 9));
        assert!(!save_botch.stays_conscious && save_botch.falls_unconscious);
    }

    #[test]
    fn test_status_conditions_and_acute_mental_trauma() {
        let nausea_dup = evaluate_physiological_status_conditions(&[
            PhysiologicalStatusCondition::NauseaSickened,
            PhysiologicalStatusCondition::NauseaSickened,
        ]);
        assert_eq!(nausea_dup.delta_int_added, 4);

        let paralyzed =
            evaluate_physiological_status_conditions(&[PhysiologicalStatusCondition::Paralyzed]);
        assert!(paralyzed.tempo_locked_zero && paralyzed.control_and_force_locked_zero);
        assert!(!paralyzed.awr_locked_zero);
        assert!(!paralyzed.to_condition_impact().is_unconscious);

        let unconscious = evaluate_physiological_status_conditions(&[
            PhysiologicalStatusCondition::UnconsciousComatose,
        ]);
        assert!(unconscious.awr_locked_zero);
        assert!(unconscious.to_condition_impact().is_unconscious);

        let t3 = evaluate_acute_mental_trauma(
            AcuteMentalTraumaTier::Tier3DissociationShellShock,
            false,
            10,
        );
        assert_eq!((t3.ana_foc_penalty, t3.dissociation_stat_penalty), (0, 6));
    }

    #[test]
    fn test_movement_gaits_and_encumbrance_tiers_0_to_4() {
        let sp10 = compute_stride_and_sprint_velocity(10, 1.75);
        assert!((sp10.v_sprint_m_per_sec - 7.50).abs() < 1e-6);
        assert!((sp10.v_sprint_m_per_tick - 1.875).abs() < 1e-6);

        let sp18 = compute_stride_and_sprint_velocity(18, 1.75);
        assert!((sp18.v_sprint_m_per_sec - 10.30).abs() < 1e-6);

        let bound =
            evaluate_combat_movement_gait(CombatMovementGait::BoundingRunHustle, 10, 1.75, 11);
        assert_eq!(
            (bound.ranged_recoil_adder, bound.effective_ranged_control_cap, bound.exertion_marks_per_20_ticks),
            (4, 5, 1)
        );

        let climb_ladder =
            evaluate_climbing_movement(true, LoadEncumbranceTier::Tier2TacticalFightingLoad, 2);
        assert_eq!(climb_ladder, (0.5, 2.0, false, 0));
        let climb_wall =
            evaluate_climbing_movement(false, LoadEncumbranceTier::Tier2TacticalFightingLoad, 2);
        assert_eq!(climb_wall, (0.15, 0.6, true, 6));

        let eva = evaluate_zero_g_maneuver(140.0, 70.0, false);
        assert!((eva.0 - 2.0).abs() < 1e-6 && eva.1);

        assert!((compute_base_comfort_load_kg(70.0, 10) - 12.0).abs() < 1e-6);
        assert!((compute_base_comfort_load_kg(85.0, 14) - 20.4).abs() < 1e-6);

        let enc_t2 = evaluate_encumbrance_load(
            70.0,
            10,
            &[CarriedLoadItem {
                mass_kg: 24.0,
                carry_method: CarryErgonomicMethod::BalancedWebbingOrRucksack,
            }],
            0.0,
        );
        assert_eq!(enc_t2.tier, LoadEncumbranceTier::Tier2TacticalFightingLoad);
        assert_eq!((enc_t2.tempo_cap_penalty, enc_t2.kin_penalty), (3, 2));
        assert!((enc_t2.action_time_factor_delta - 0.10).abs() < 1e-6);

        let enc_exo = evaluate_encumbrance_load(
            70.0,
            10,
            &[CarriedLoadItem {
                mass_kg: 24.0,
                carry_method: CarryErgonomicMethod::BalancedWebbingOrRucksack,
            }],
            20.0,
        );
        assert_eq!(enc_exo.tier, LoadEncumbranceTier::Tier0Unencumbered);
    }

    #[test]
    fn test_environmental_survival_dehydration_starvation_thermal_sleep_vacuum() {
        let dehyd = evaluate_dehydration(
            HydrationEnvironment::HotAridDesertOrJungleTactical,
            4.0,
            0.0,
        );
        assert_eq!(dehyd.stage, DehydrationStage::Stage2Parched);
        assert_eq!((dehyd.delta_int, dehyd.fatigue_level_floor), (4, 2));
        assert_eq!(enforce_fatigue_floor(0, &[dehyd.fatigue_level_floor]), 2);

        let dehyd_sweat = evaluate_dehydration(
            HydrationEnvironment::TemperateLowActivity,
            2.25,
            2.0,
        );
        assert_eq!(dehyd_sweat.stage, DehydrationStage::Stage2Parched);

        let starv = evaluate_starvation_and_caloric_deficit(6 * 24, 18_000, 10, 10, 10);
        assert_eq!((starv.catabolic_intervals, starv.for_fit_hlt_erosion), (2, 2));
        assert!((starv.mass_lost_kg - 3.0).abs() < 1e-6);

        let cramps = evaluate_thermal_stress(
            HypothermiaStage::None,
            HyperthermiaStage::Stage1HeatCramps,
        );
        assert_eq!(cramps.tempo_and_force_cap_penalty, 2);
        assert_eq!(
            cramps.to_condition_impact().triad_cap_penalty,
            Triad::new(2, 0, 2)
        );

        assert_eq!(
            evaluate_frostbite_exposure(-15.0, true, false),
            Some(StructuralWoundTier::Tier1Superficial)
        );
        assert_eq!(
            evaluate_frostbite_exposure(-30.0, true, false),
            Some(StructuralWoundTier::Tier3SevereFracture)
        );

        let sleep = evaluate_sleep_deprivation(72);
        assert_eq!((sleep.delta_int, sleep.mental_sensory_ooda_penalty), (8, 6));
        assert!(sleep.requires_microsleep_dis_check);

        let alt_2600 = evaluate_altitude_and_vacuum_hypoxia(2600, false, false, 0, 10);
        assert_eq!(alt_2600.fit_penalty, 2);

        let vac = evaluate_altitude_and_vacuum_hypoxia(0, false, true, 68, 10);
        assert_eq!((vac.vacuum_useful_consciousness_ticks, vac.vacuum_hlt_erosion), (48, 2));
        assert!(vac.vacuum_unconscious);
    }
}
