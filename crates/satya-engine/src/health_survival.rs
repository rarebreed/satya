//! # Health, Anatomical Trauma, Movement, Vehicles & Environmental Survival (`06-health-and-survival`)
//!
//! Canonical Rust implementation of `docs/ttrpg/06-health-and-survival/`:
//! - **`health_and_trauma.md`**:
//!   - **§1 Hit Locations & Called Shots**: `1d20` Master Anatomical Hit Location Table (13 sub-zones)
//!     and Called Shot $\Delta_{\text{ext}}$ (`+2` Broad Zone waived when $C_{\text{att}} - C_{\text{def}} \ge 3$,
//!     `+4` Specific Sub-Zone/Joint, `+6..=+8` Vital Pinpoint).
//!   - **§2 Short-Term Metabolic Damage**: Fatigue Threshold ($\text{FT} = \text{FIT} + \lfloor\text{DIS}/2\rfloor$),
//!     the 6 Fatigue Levels (`0–5`), Breath-Holding ($40\times\text{FIT}$ calm vs. $16\times\text{FIT}$ vigorous ticks),
//!     Asphyxiation/Drowning (`+1` fatigue per `20 ticks`), and Vascular Blood Chokes (`24–40 ticks`).
//!   - **§3 Long-Term Structural Trauma**: Structural Wound Severity Tiers (`Tiers 1–5`), Broken Bones &
//!     Dislocations (hairline fracture aggravation, ribcage exertion, femoral/brachial arterial hazard),
//!     Hemorrhaging Grades (`Grades 0–4`) on the `10.0-Unit` Blood Volume track, Hypovolemic Shock (`Classes I–IV`),
//!     and Contested `2d20` Wound Shock Resistance (`WSR`) Consciousness Saves.
//!   - **§4–§6 Status Effects, Mental Trauma & Medical Triage**: The 8 Physiological/Sensory Status Conditions,
//!     the 4 Acute Mental Trauma Tiers (`OODA Freeze`, `Panic/Feral Rage`, `Dissociation`, `Catatonia`),
//!     Field Analgesics, and Weekly `HLT` Healing Recovery Cascades.
//! - **`movement_vehicles_and_environment.md`**:
//!   - **§1 Movement Architecture**: Stride Factor ($\text{SF} = \text{HGT}/1.75$), Maximum Sprint Velocity
//!     ($V_{\text{sprint}} = (4.0 + 0.35\times\text{SPD}_{\text{eff}})\times\text{SF}$), 5 Combat Movement Gaits,
//!     Jumping/Climbing/Swimming/Zero-G EVA, and Overland Travel Paces & Terrain Multipliers.
//!   - **§2 Encumbrance & Load-Bearing (`Tiers 0–4`)**: Base Comfort Load
//!     ($L_{\text{base}} = (\text{MAS}/70.0)\times(\text{FOR}\times 1.2\text{ kg})$), Ergonomic Load Multipliers
//!     (`×0.8` to `×1.5` and Powered Exo-Rig offsets), and the 5 Encumbrance Tiers (`0–4`).
//!   - **§3 Vehicles, Chases & High-G / Crash Physics**: Vehicle Envelope Ceilings (`Tempo`, `Control`, `Force/Root`),
//!     Chase Range Bands, Collision Kinetic Trauma, and High-G `G-LOC` Checks.
//!   - **§4 Environmental Survival**: Hydration & Dehydration (`Stages 0–4` with Fatigue Floors),
//!     Caloric Deficit & Starvation, Thermal Extremes (`CLO` Suit Tiers, Hypothermia, Frostbite, Heatstroke),
//!     Sleep Deprivation (`24h/48h/72h+`), Altitude/Vacuum Hypoxia, 4-Parameter Toxins, and Ionizing Radiation (`Sv`).

use serde::{Deserialize, Serialize};

use crate::action_economy::EncumbranceTier;
use crate::characteristics::compute_wound_shock_resistance_score;
use crate::dice_probabilities::{
    ContestedCascadeRollOutcome, ContestedOutcomeKind, ContestedTieRule,
};
use crate::primitives::{ConditionImpact, TieredSaveEval, Triad};

// ============================================================================
// 1. Hit Locations & Called Shots (`health_and_trauma.md` §1)
// ============================================================================

/// Precision tier for a targeted / called shot (`health_and_trauma.md` §1.1).
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

/// Computes the External Difficulty modifier ($\Delta_{\text{ext, loc}}$) for a Called Shot (`health_and_trauma.md` §1.1).
///
/// For [`CalledShotZoneTier::BroadZone`], the `+2` penalty is waived (`0`) when
/// `attacker_control - defender_control >= 3`.
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

/// Canonical 13 Anatomical Hit Location sub-zones (`health_and_trauma.md` §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitLocationId {
    /// `1d20 = 1`: Foot / Ankle (L or R)
    FootAnkle,
    /// `1d20 = 2–3`: Calf / Shin (Tibia/Fibula) (L or R)
    CalfShin,
    /// `1d20 = 4`: Knee Joint (Patella/ACL) (L or R)
    KneeJoint,
    /// `1d20 = 5–6`: Thigh (Femur / Femoral) (L or R)
    ThighFemur,
    /// `1d20 = 7`: Pelvis / Hip Girdle / Groin
    PelvisHipGroin,
    /// `1d20 = 8–10`: Abdomen / Gut / Liver / Spleen
    AbdomenGutLiverSpleen,
    /// `1d20 = 11–13`: Chest / Ribcage / Lungs
    ChestRibcageLungs,
    /// `1d20 = 14`: Vital Thoracic (Heart / Spine / Aorta)
    VitalThoracicHeartSpineAorta,
    /// `1d20 = 15`: Shoulder / Clavicle / Scapula (L or R)
    ShoulderClavicleScapula,
    /// `1d20 = 16–17`: Upper Arm / Elbow / Forearm (L or R)
    UpperArmElbowForearm,
    /// `1d20 = 18`: Wrist / Hand / Fingers (L or R)
    WristHandFingers,
    /// `1d20 = 19`: Neck / Throat / Carotid / Cervical
    NeckThroatCarotidCervical,
    /// `1d20 = 20`: Head (Cranium / Face / Jaw / Eyes)
    HeadCraniumFaceJawEyes,
}

/// Canonical specification for an entry in the `1d20` Master Anatomical Hit Location Table (`health_and_trauma.md` §1.2).
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
    /// Returns the `Tempo / Control / Force` aspect ceiling slash flags as a [`Triad<bool>`].
    #[must_use]
    pub const fn slashes_triad(&self) -> Triad<bool> {
        Triad::new(
            self.slashes_tempo_cap,
            self.slashes_control_cap,
            self.slashes_force_cap,
        )
    }
}

/// Returns the canonical 13-entry Master Anatomical Hit Location Table (`health_and_trauma.md` §1.2).
#[must_use]
pub fn canonical_hit_location_table() -> Vec<HitLocationSpec> {
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
    vec![
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
    ]
}

/// Resolves a `1d20` roll (`1..=20`) into its canonical [`HitLocationSpec`] (`health_and_trauma.md` §1.2).
#[must_use]
pub fn resolve_hit_location_d20(d20_roll: i32) -> HitLocationSpec {
    let clamped = d20_roll.clamp(1, 20);
    canonical_hit_location_table()
        .into_iter()
        .find(|spec| clamped >= spec.d20_min && clamped <= spec.d20_max)
        .expect("1..=20 is exhaustively covered by canonical_hit_location_table")
}

// ============================================================================
// 2. Short-Term Metabolic Damage: Fatigue Levels (0–5) & Asphyxiation (§2)
// ============================================================================

/// Mechanical effects of a character's current Fatigue Level (`0..=5`, `health_and_trauma.md` §2.2).
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

/// Evaluates the exact mechanical penalties and physiological state for a `fatigue_level` (`0..=5`, `health_and_trauma.md` §2.2).
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
    let delta_under = 6 + delta_int;
    let tactical_pool_drain = -2 * lvl;
    let action_time_factor = 1.0 + 0.10 * f64::from(lvl);
    let spd_and_for_cap_penalty = if lvl >= 3 { 2 } else { 0 };
    let can_sprint = lvl < 4;
    let is_unconscious = lvl >= 5;

    FatigueLevelSpec {
        fatigue_level: lvl,
        condition_name,
        delta_int,
        delta_under,
        tactical_pool_drain,
        action_time_factor,
        spd_and_for_cap_penalty,
        can_sprint,
        is_unconscious,
        recovery_requirement,
    }
}

/// Outcome of evaluating Breath-Holding, Drowning/Asphyxiation, or a Vascular Blood Choke (`health_and_trauma.md` §2.3).
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
/// and vascular blood chokes (`24–40 ticks` to unconsciousness) (`health_and_trauma.md` §2.3).
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
        // Vascular blood choke compresses carotid arteries directly: unconsciousness in 24–40 ticks (6–10s).
        let choke_ko_ticks = (24 + (fit_clamped - 10).max(0)).clamp(24, 40);
        let is_unconscious = ticks >= choke_ko_ticks || base_fatigue >= 5;
        let fatigue_levels_gained = ticks.div_euclid(20);
        let resulting_fatigue_level = (base_fatigue + fatigue_levels_gained).min(5);
        let hypoxic_brain_death_risk = ticks >= choke_ko_ticks + 240;
        return AsphyxiationOutcome {
            breath_holding_capacity_ticks: 0,
            hypoxic_overrun_ticks: ticks,
            fatigue_levels_gained,
            resulting_fatigue_level,
            is_unconscious,
            hypoxic_brain_death_risk,
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
    let hypoxic_brain_death_risk = is_unconscious && ticks_unconscious >= 240;

    AsphyxiationOutcome {
        breath_holding_capacity_ticks,
        hypoxic_overrun_ticks,
        fatigue_levels_gained,
        resulting_fatigue_level,
        is_unconscious,
        hypoxic_brain_death_risk,
    }
}

// ============================================================================
// 3. Long-Term Damage: Structural Wounds, Fractures, Bleed & WSR Saves (§3)
// ============================================================================

/// The 5 Structural Wound Severity Tiers (`health_and_trauma.md` §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralWoundTier {
    /// **Tier 1: Superficial / Minor**: `-2` to Location's Ceiling, `+0` Shock (`+1` if `WSR < 10`), Capillary Bleed. Heals in `3–7 days`.
    Tier1Superficial,
    /// **Tier 2: Moderate Trauma**: `-5` to Location's Ceiling, `+1` Shock Tier, Venous Bleed / hairline fracture. Heals in `2–4 weeks`.
    Tier2Moderate,
    /// **Tier 3: Severe / Fracture**: `-10` (Limb Disabled), `+2` Shock Tiers, Complete Bone Fracture / Major Venous or Arterial Bleed. `6–12 weeks`.
    Tier3SevereFracture,
    /// **Tier 4: Critical / Maiming**: Ceiling Locked at `0`, `+3` Shock Tiers, Compound Fracture / Solid Organ Rupture / Arterial Severance. `3–6 months`.
    Tier4CriticalMaiming,
    /// **Tier 5: Catastrophic / Lethal**: Ceiling Locked at `0` (Destroyed), `+4` Shock Tiers, Traumatic Amputation / Transected Spine / Ruptured Heart.
    Tier5CatastrophicLethal,
}

impl StructuralWoundTier {
    /// Converts a numeric wound tier (`1..=5`) into [`StructuralWoundTier`].
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

    /// Numeric tier (`1..=5`).
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

    /// Aspect ceiling penalty ($T_{\max}, C_{\max}, F_{\max}$) inflicted on the struck location (`2`, `5`, `10`, or `20` for Locked at `0`).
    #[must_use]
    pub const fn aspect_ceiling_penalty(self) -> i32 {
        match self {
            Self::Tier1Superficial => 2,
            Self::Tier2Moderate => 5,
            Self::Tier3SevereFracture => 10,
            Self::Tier4CriticalMaiming | Self::Tier5CatastrophicLethal => 20,
        }
    }

    /// Whether the struck location's aspect ceiling is completely locked at `0` (`Tier 4` or `Tier 5`).
    #[must_use]
    pub const fn locks_ceiling_to_zero(self) -> bool {
        matches!(
            self,
            Self::Tier4CriticalMaiming | Self::Tier5CatastrophicLethal
        )
    }

    /// Systemic `wound_shock_index` tiers added by this wound (`health_and_trauma.md` §3.1).
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

/// Maps net penetrating/BABT trauma magnitude into a [`StructuralWoundTier`] (`health_and_trauma.md` §3.1).
///
/// - `1..=3`: Tier 1 (Superficial)
/// - `4..=6`: Tier 2 (Moderate)
/// - `7..=10`: Tier 3 (Severe / Fracture)
/// - `11..=14`: Tier 4 (Critical / Maiming)
/// - `15+`: Tier 5 (Catastrophic / Lethal)
#[must_use]
pub const fn classify_structural_wound_tier(effective_trauma: i32) -> Option<StructuralWoundTier> {
    if effective_trauma <= 0 {
        None
    } else if effective_trauma <= 3 {
        Some(StructuralWoundTier::Tier1Superficial)
    } else if effective_trauma <= 6 {
        Some(StructuralWoundTier::Tier2Moderate)
    } else if effective_trauma <= 10 {
        Some(StructuralWoundTier::Tier3SevereFracture)
    } else if effective_trauma <= 14 {
        Some(StructuralWoundTier::Tier4CriticalMaiming)
    } else {
        Some(StructuralWoundTier::Tier5CatastrophicLethal)
    }
}

/// Evaluates whether a **Tier 2 Hairline Fracture** snaps into a **Tier 3 Complete Fracture** under load (`health_and_trauma.md` §3.2.1):
///
/// If the character allocates $> 50\%$ of their base ceiling into the fractured limb's aspect and fails a
/// `HLT` check at $\Delta_{\text{ext}} = +4$ (`hlt_check_succeeded == false`), the bone snaps into a Tier 3 Complete Fracture!
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

/// Evaluates reducing ("popping back in") a **Joint Dislocation** (`health_and_trauma.md` §3.2.2):
///
/// Requires `20 ticks` and a *First Aid* / *Orthopedic Medicine* (`DEX/INS`) check; reduces the `-8`
/// dislocation lock down to a `-3` residual sprain penalty.
#[must_use]
pub const fn reduce_joint_dislocation(first_aid_succeeded: bool) -> (i32, i32) {
    let action_ticks = 20;
    let residual_ceiling_penalty = if first_aid_succeeded { 3 } else { 8 };
    (action_ticks, residual_ceiling_penalty)
}

/// The Four Hemorrhage Grades (plus `Grade0None`) on the `10.0-Unit` Blood Volume Track (`health_and_trauma.md` §3.3).
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
    /// Blood volume lost per Tick (`1 Tick = 0.25s`, `240 Ticks = 1 min`, `14,400 Ticks = 1 hr`) in Units (`10.0 Units` baseline).
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

    /// Whether this hemorrhage grade can slow or clot naturally without hemostatic packing/tourniquet/surgery (`Grades 0–2`).
    #[must_use]
    pub const fn can_clot_naturally(self) -> bool {
        matches!(
            self,
            Self::Grade0None | Self::Grade1Capillary | Self::Grade2Venous
        )
    }

    /// Ticks required to apply standard field hemorrhage control (`8` bandage, `20` pressure dressing, `24` CAT tourniquet).
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

/// Clinical Hypovolemic Shock Classes (`Classes I–IV`) from cumulative blood volume lost (`health_and_trauma.md` §3.3).
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

/// Complete state of a character's `10.0-Unit` Blood Volume Track after advancing by `elapsed_ticks` (`health_and_trauma.md` §3.3).
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

/// Advances blood loss over `elapsed_ticks` across active [`HemorrhageGrade`] sources and evaluates
/// Hypovolemic Shock progression (`health_and_trauma.md` §3.3).
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

    // Grade 4 Arterial Severance causes cerebral hypoperfusion unconsciousness in 16–24 ticks (4–6s).
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

/// Outcome of a Contested `2d20` **Systemic Wound Shock (`WSR`) Consciousness Save** (`health_and_trauma.md` §3.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WoundShockSaveOutcome {
    /// Derived Wound Shock Resistance $\text{WSR} = \text{HLT} + \lfloor\text{DIS}/2\rfloor$.
    pub wsr: i32,
    /// Composite controlling characteristic $C_P = \lfloor(\text{HLT} + \text{DIS})/2\rfloor$.
    pub controller_c_p: i32,
    /// Player Target Number $\text{TN}_P = 31 - C_P$.
    pub tn_player: i32,
    /// External difficulty $\Delta_{\text{ext}} = 2 \times \text{wound\_shock\_index}$.
    pub delta_ext: i32,
    /// Underlying contested `2d20` roll evaluation.
    pub contested_roll: ContestedCascadeRollOutcome,
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

/// Evaluates a Contested `2d20` **Systemic Wound Shock (`WSR`) Consciousness Save** (`health_and_trauma.md` §3.4):
/// - Player rolls `HLT / DIS` ($C_P = \lfloor(\text{HLT} + \text{DIS})/2\rfloor$, $\text{TN}_P = 31 - C_P$)
///   against GM 1-Pair Tiered Resistance ($\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$ with $\Delta_{\text{ext}} = +2 \times \text{wound\_shock\_index}$).
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
    let contested_roll = eval.contested_roll;

    let (
        effective_wound_shock_index,
        stays_conscious,
        stunned_ticks,
        drops_prone,
        triggers_morale_check,
        falls_unconscious,
    ) = match contested_roll.contested_kind {
        ContestedOutcomeKind::ExtraordinarySuccess => {
            ((shock - 1).max(0), true, 0, false, false, false)
        }
        ContestedOutcomeKind::CleanSuccess if contested_roll.net_signed_margin >= 2 => {
            ((shock - 1).max(0), true, 0, false, false, false)
        }
        ContestedOutcomeKind::CleanSuccess
        | ContestedOutcomeKind::SuccessWithFloorBreach
        | ContestedOutcomeKind::GrazingTieSuccess
        | ContestedOutcomeKind::LuckyBreakSuccess => (shock, true, 0, false, false, false),
        ContestedOutcomeKind::CleanFailure => {
            if shock >= 4 {
                (shock, false, 0, true, false, true)
            } else {
                (shock, true, 8, true, true, false)
            }
        }
        ContestedOutcomeKind::BotchFailure => (shock, false, 0, true, false, true),
    };

    WoundShockSaveOutcome {
        wsr,
        controller_c_p,
        tn_player,
        delta_ext,
        contested_roll,
        effective_wound_shock_index,
        stays_conscious,
        stunned_ticks,
        drops_prone,
        triggers_morale_check,
        falls_unconscious,
    }
}

// ============================================================================
// 4. Physiological, Sensory & Acute Mental Status Conditions (§4 – §6)
// ============================================================================

/// Canonical Acute Physiological & Sensory Status Conditions (`health_and_trauma.md` §4).
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
    /// **`Paralyzed`**: `Tempo = 0`, `Control = 0`, `Force = 0` for affected limbs.
    Paralyzed,
    /// **`Unconscious` / `Comatose`**: `Tempo = 0, Control = 0, Force = 0, AWR = 0`, collapses Prone.
    UnconsciousComatose,
}

/// Aggregated characteristic, ceiling, and clock modifiers from active [`PhysiologicalStatusCondition`]s (`health_and_trauma.md` §4).
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
}

impl AggregatedStatusEffects {
    /// Converts this status aggregation into a [`ConditionImpact`] primitive.
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
            is_unconscious: self.tempo_locked_zero && self.control_and_force_locked_zero,
        }
    }
}

/// Evaluates the combined mechanical impact of a slice of [`PhysiologicalStatusCondition`]s (`health_and_trauma.md` §4).
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
                out.delta_int_added += 4;
                out.fit_halved = true;
            }
            PhysiologicalStatusCondition::PinnedImmobilized => {
                out.tempo_locked_zero = true;
            }
            PhysiologicalStatusCondition::Paralyzed
            | PhysiologicalStatusCondition::UnconsciousComatose => {
                out.tempo_locked_zero = true;
                out.control_and_force_locked_zero = true;
            }
        }
    }

    out
}

/// The 4 Acute Mental Trauma Conditions (`health_and_trauma.md` §5.1).
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

/// Mechanical evaluation of an [`AcuteMentalTraumaTier`] (`health_and_trauma.md` §5.1–§5.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcuteMentalTraumaEffect {
    /// Mental trauma severity tier (`1..=4`).
    pub tier: AcuteMentalTraumaTier,
    /// Action clock freeze in ticks (`6..=12`, midpoint `9` for Tier 1).
    pub ooda_freeze_ticks: i32,
    /// Tactical Pool modifier (`-4` in Tier 2 Panic).
    pub tactical_pool_modifier: i32,
    /// Subtractive penalty to `ANA` and `FOC` (`-6` in Tier 2).
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

/// Evaluates the mechanical effects and long-term psychological scars of an [`AcuteMentalTraumaTier`] (`health_and_trauma.md` §5.1–§5.2).
#[must_use]
pub fn evaluate_acute_mental_trauma(
    tier: AcuteMentalTraumaTier,
    is_cornered: bool,
    agg_score: i32,
) -> AcuteMentalTraumaEffect {
    match tier {
        AcuteMentalTraumaTier::Tier1AcuteMentalShockOodaFreeze => AcuteMentalTraumaEffect {
            tier,
            ooda_freeze_ticks: 9,
            tactical_pool_modifier: 0,
            ana_foc_penalty: 0,
            dissociation_stat_penalty: 0,
            action_time_factor_added: 0.0,
            shifts_into_feral_rage: false,
            total_psychomotor_shutdown: false,
            permanent_psyche_drift_marks: 0,
            trauma_trigger_delta_morale: 0,
        },
        AcuteMentalTraumaTier::Tier2PanicFightOrFlight => AcuteMentalTraumaEffect {
            tier,
            ooda_freeze_ticks: 0,
            tactical_pool_modifier: -4,
            ana_foc_penalty: 6,
            dissociation_stat_penalty: 0,
            action_time_factor_added: 0.0,
            shifts_into_feral_rage: is_cornered && agg_score >= 13,
            total_psychomotor_shutdown: false,
            permanent_psyche_drift_marks: 0,
            trauma_trigger_delta_morale: 0,
        },
        AcuteMentalTraumaTier::Tier3DissociationShellShock => AcuteMentalTraumaEffect {
            tier,
            ooda_freeze_ticks: 0,
            tactical_pool_modifier: 0,
            ana_foc_penalty: 6,
            dissociation_stat_penalty: 6,
            action_time_factor_added: 0.30,
            shifts_into_feral_rage: false,
            total_psychomotor_shutdown: false,
            permanent_psyche_drift_marks: 2,
            trauma_trigger_delta_morale: 4,
        },
        AcuteMentalTraumaTier::Tier4Catatonia => AcuteMentalTraumaEffect {
            tier,
            ooda_freeze_ticks: 0,
            tactical_pool_modifier: 0,
            ana_foc_penalty: 0,
            dissociation_stat_penalty: 0,
            action_time_factor_added: 0.0,
            shifts_into_feral_rage: false,
            total_psychomotor_shutdown: true,
            permanent_psyche_drift_marks: 2,
            trauma_trigger_delta_morale: 4,
        },
    }
}

/// Evaluates administering field analgesics (morphine/ketamine/fentanyl lozenge) (`health_and_trauma.md` §6.1):
/// - Reduces `wound_shock_index` by `1` (mild dose) or `2` (strong dose) tiers.
/// - Imposes `-2` (mild) or `-4` (strong) sedation penalty on `FOC`, `ANA`, and `SPD`.
#[must_use]
pub const fn apply_field_analgesic(
    current_wound_shock_index: i32,
    strong_dose: bool,
) -> (i32, i32) {
    let shock_reduction = if strong_dose { 2 } else { 1 };
    let sedation_stat_penalty = if strong_dose { 4 } else { 2 };
    let raw_shock = current_wound_shock_index - shock_reduction;
    let effective_shock = if raw_shock < 0 { 0 } else { raw_shock };
    (effective_shock, sedation_stat_penalty)
}

/// Estimates healing recovery days for a stabilized [`StructuralWoundTier`] (`health_and_trauma.md` §6.3).
///
/// Returns `(min_days, max_days, adjusted_days_after_hlt_successes)`.
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

// ============================================================================
// 5. Movement Architecture, Combat Gaits, Special Modes & Overland (`movement_vehicles_and_environment.md` §1)
// ============================================================================

/// Baseline Stride Factor ($\text{SF}$) and Sprint Velocity ($V_{\text{sprint}}$) (`movement_vehicles_and_environment.md` §1.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SprintVelocityMetrics {
    /// Stride Factor $\text{SF} = \text{HGT} / 1.75\text{ m}$.
    pub stride_factor_sf: f64,
    /// Maximum Sprint Velocity $V_{\text{sprint}} = (4.0 + 0.35 \times \text{SPD}_{\text{eff}}) \times \text{SF}$ in meters per second (`m/s`).
    pub v_sprint_m_per_sec: f64,
    /// Maximum Sprint Velocity in kilometers per hour (`km/h`).
    pub v_sprint_km_per_hr: f64,
    /// Maximum Sprint Velocity in meters per Tick (`m/Tick`, where `1 second = 4 Ticks`).
    pub v_sprint_m_per_tick: f64,
}

/// Computes Stride Factor ($\text{SF}$) and Maximum Sprint Velocity ($V_{\text{sprint}}$) (`movement_vehicles_and_environment.md` §1.1).
#[must_use]
pub fn compute_stride_and_sprint_velocity(spd_eff: i32, hgt_m: f64) -> SprintVelocityMetrics {
    let spd = spd_eff.clamp(0, 25);
    let hgt = hgt_m.clamp(0.5, 3.0);
    let stride_factor_sf = hgt / 1.75;
    let v_sprint_m_per_sec = (4.0 + 0.35 * f64::from(spd)) * stride_factor_sf;
    let v_sprint_km_per_hr = v_sprint_m_per_sec * 3.6;
    let v_sprint_m_per_tick = v_sprint_m_per_sec / 4.0;

    SprintVelocityMetrics {
        stride_factor_sf,
        v_sprint_m_per_sec,
        v_sprint_km_per_hr,
        v_sprint_m_per_tick,
    }
}

/// The 5 Canonical Tactical Combat Movement Gaits (`movement_vehicles_and_environment.md` §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatMovementGait {
    /// **1. Prone / Low Crawl** (`8%` of $V_{\text{sprint}}$): `+4` low cover bonus vs. ranged; melee `Tempo Cap -4`; `+1` Exertion Mark / `20 ticks`.
    ProneLowCrawl,
    /// **2. Stealth Creep / Crouch-Walk** (`18%` of $V_{\text{sprint}}$): `+0` $\Delta_{\text{ext}}$ on Stealth; full `Control` uncapped.
    StealthCreepCrouchWalk,
    /// **3. Combat Glide / Tactical Walk** (`35%` of $V_{\text{sprint}}$): Simultaneous aimed fire at `+2` Recoil (`R`), `0` Exertion Marks.
    CombatGlideTacticalWalk,
    /// **4. Bounding Run / Hustle** (`65%` of $V_{\text{sprint}}$): Requires `Tempo >= B`; `+4` Recoil (`R`); caps ranged `Control` at $\lfloor C_{\max}/2\rfloor$; `+1` Exertion Mark / `20 ticks`.
    BoundingRunHustle,
    /// **5. All-Out Sprint (`Pure Tempo`)** (`100%` of $V_{\text{sprint}}$): Requires `Force = 0, Control <= 2`; cannot fire aimed shots; `+2` Exertion Marks / `20 ticks`.
    AllOutSprint,
}

impl CombatMovementGait {
    /// Fraction of $V_{\text{sprint}}$ (`0.08`, `0.18`, `0.35`, `0.65`, `1.00`).
    #[must_use]
    pub const fn sprint_fraction(self) -> f64 {
        match self {
            Self::ProneLowCrawl => 0.08,
            Self::StealthCreepCrouchWalk => 0.18,
            Self::CombatGlideTacticalWalk => 0.35,
            Self::BoundingRunHustle => 0.65,
            Self::AllOutSprint => 1.00,
        }
    }

    /// Extra Recoil (`R`) added when firing a ranged weapon while moving at this gait (`0`, `0`, `+2`, `+4`, or un-aimable).
    #[must_use]
    pub const fn ranged_recoil_adder(self) -> i32 {
        match self {
            Self::ProneLowCrawl | Self::StealthCreepCrouchWalk => 0,
            Self::CombatGlideTacticalWalk => 2,
            Self::BoundingRunHustle => 4,
            Self::AllOutSprint => 8,
        }
    }

    /// Exertion Marks accumulated per `20 Ticks` (`5 seconds`) of continuous movement in this gait.
    #[must_use]
    pub const fn exertion_marks_per_20_ticks(self) -> i32 {
        match self {
            Self::StealthCreepCrouchWalk | Self::CombatGlideTacticalWalk => 0,
            Self::ProneLowCrawl | Self::BoundingRunHustle => 1,
            Self::AllOutSprint => 2,
        }
    }
}

/// Complete combat movement and triad interaction metrics for a chosen [`CombatMovementGait`] (`movement_vehicles_and_environment.md` §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CombatGaitEvaluation {
    /// Chosen gait mode.
    pub gait: CombatMovementGait,
    /// Speed in meters per second (`m/s`).
    pub speed_m_per_sec: f64,
    /// Speed in meters per Tick (`m/Tick`, `1 Tick = 0.25s`).
    pub speed_m_per_tick: f64,
    /// Distance covered over `10 Ticks` (`2.5s`) in meters.
    pub distance_per_10_ticks_m: f64,
    /// Ranged weapon Recoil (`R`) adder (`+0`, `+2`, `+4`).
    pub ranged_recoil_adder: i32,
    /// Effective Ranged `Control Cap` after gait constraints ($\lfloor C_{\max}/2\rfloor$ in `BoundingRunHustle`, `2` in `AllOutSprint`).
    pub effective_ranged_control_cap: i32,
    /// Whether aimed ranged shots are permitted (`false` in `AllOutSprint`).
    pub allows_aimed_ranged_fire: bool,
    /// Base Exertion Marks accumulated per `20 Ticks` (`5 seconds`).
    pub exertion_marks_per_20_ticks: i32,
}

/// Evaluates velocity (`m/s`, `m/Tick`, distance per `10 Ticks`) and Tactical Triad constraints for a [`CombatMovementGait`] (`movement_vehicles_and_environment.md` §1.2).
#[must_use]
pub fn evaluate_combat_movement_gait(
    gait: CombatMovementGait,
    spd_eff: i32,
    hgt_m: f64,
    base_ranged_control_cap: i32,
) -> CombatGaitEvaluation {
    let sprint = compute_stride_and_sprint_velocity(spd_eff, hgt_m);
    let frac = gait.sprint_fraction();
    let speed_m_per_sec = sprint.v_sprint_m_per_sec * frac;
    let speed_m_per_tick = sprint.v_sprint_m_per_tick * frac;
    let distance_per_10_ticks_m = speed_m_per_tick * 10.0;
    let c_cap = base_ranged_control_cap.max(0);

    let (effective_ranged_control_cap, allows_aimed_ranged_fire) = match gait {
        CombatMovementGait::ProneLowCrawl
        | CombatMovementGait::StealthCreepCrouchWalk
        | CombatMovementGait::CombatGlideTacticalWalk => (c_cap, true),
        CombatMovementGait::BoundingRunHustle => (c_cap.div_euclid(2), true),
        CombatMovementGait::AllOutSprint => (c_cap.min(2), false),
    };

    CombatGaitEvaluation {
        gait,
        speed_m_per_sec,
        speed_m_per_tick,
        distance_per_10_ticks_m,
        ranged_recoil_adder: gait.ranged_recoil_adder(),
        effective_ranged_control_cap,
        allows_aimed_ranged_fire,
        exertion_marks_per_20_ticks: gait.exertion_marks_per_20_ticks(),
    }
}

/// Jump & Vault distances in meters (`movement_vehicles_and_environment.md` §1.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct JumpDistances {
    /// Running Broad Jump: $2.0 + 0.25\times\text{POW} + 0.3\times\text{Successes}$ (`4.5 m` baseline at `POW 10`).
    pub running_broad_jump_m: f64,
    /// Standing Broad Jump: `55%` of Running Broad Jump (`~2.475 m` at `POW 10`).
    pub standing_broad_jump_m: f64,
    /// Running High Jump / Waist-Chest Vault without hands: $0.6 + 0.06\times\text{POW} + 0.1\times\text{Successes}$ (`1.2 m` at `POW 10`).
    pub running_high_jump_no_hands_m: f64,
    /// Maximum Wall Scaling / Ledge Grab height (`running_high_jump_no_hands_m + 0.8 * HGT`).
    pub ledge_grab_pullup_height_m: f64,
}

/// Computes Running Broad Jump, Standing Broad Jump, Running High Jump, and Ledge-Grab Scaling heights (`movement_vehicles_and_environment.md` §1.3.1).
#[must_use]
pub fn compute_jump_distances(pow: f64, hgt_m: f64, cascade_successes: i32) -> JumpDistances {
    let p = pow.max(0.0);
    let s = f64::from(cascade_successes.max(0));
    let hgt = hgt_m.clamp(0.5, 3.0);

    let running_broad_jump_m = 2.0 + 0.25 * p + 0.3 * s;
    let standing_broad_jump_m = 0.55 * running_broad_jump_m;
    let running_high_jump_no_hands_m = 0.6 + 0.06 * p + 0.1 * s;
    let ledge_grab_pullup_height_m = running_high_jump_no_hands_m + 0.8 * hgt;

    JumpDistances {
        running_broad_jump_m,
        standing_broad_jump_m,
        running_high_jump_no_hands_m,
        ledge_grab_pullup_height_m,
    }
}

/// Evaluates surface swimming speed (`20%` of $V_{\text{sprint}}$) and water encumbrance hazards (`movement_vehicles_and_environment.md` §1.3.3).
///
/// Returns `(swim_speed_m_per_sec, swim_speed_m_per_tick, stay_afloat_delta_ext, sinks_at_1m_per_sec)`.
#[must_use]
pub fn evaluate_swimming_movement(
    spd_eff: i32,
    hgt_m: f64,
    load_encumbrance_tier: u8,
) -> (f64, f64, i32, bool) {
    let sprint = compute_stride_and_sprint_velocity(spd_eff, hgt_m);
    let swim_m_per_sec = 0.20 * sprint.v_sprint_m_per_sec;
    let swim_m_per_tick = 0.20 * sprint.v_sprint_m_per_tick;
    let stay_afloat_delta_ext = match load_encumbrance_tier {
        0 => 0,
        1 => 1,
        2 => 4,
        _ => 8,
    };
    let sinks_at_1m_per_sec = load_encumbrance_tier >= 3;
    (
        swim_m_per_sec,
        swim_m_per_tick,
        stay_afloat_delta_ext,
        sinks_at_1m_per_sec,
    )
}

/// Non-Combat Overland Travel Pace (`movement_vehicles_and_environment.md` §1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlandTravelPace {
    /// **Cautious Recon / Trail-Finding**: `3.0 km/h` (`50 m/min`), `24 km/day`; active Observation & Stealth without penalty.
    CautiousRecon,
    /// **Standard Marching / Walking Pace**: `5.0 km/h` (`83 m/min`), `40 km/day`; `+1` fatigue after `8 hrs` (`4 hrs` at Tier 2+ load).
    StandardMarching,
    /// **Forced March / Double-Time**: `7.0 km/h` (`117 m/min`), `56 km/day`; roll `FIT/DIS` every `2 hrs` ($\Delta_{\text{ext}} = +2 \times \text{Load Tier}$) or `+1` fatigue.
    ForcedMarchDoubleTime,
    /// **Sustained Cross-Country Run**: `11.0 km/h` (`183 m/min`); roll `FIT` every `15 min` or `+1` fatigue.
    SustainedCrossCountryRun,
}

/// Terrain multiplier to movement speed and exertion (`movement_vehicles_and_environment.md` §1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlandTerrainKind {
    /// Paved Road / Corridor / Dry Sidewalk (`×1.00`, `+0` exertion)
    PavedRoadCorridor,
    /// Broken Ground / Tall Grass / Light Woods / Stairs (`×0.75`, `+0` exertion)
    BrokenGroundLightWoods,
    /// Dense Jungle / Mud / Sand Dunes / Rubble (`×0.50`, `+1` Exertion Mark per interval)
    DenseJungleMudSandRubble,
    /// Deep Snow / Chest-Deep Marsh / Steep Scree (`×0.25`, `+2` Exertion Marks per interval)
    DeepSnowMarshSteepScree,
}

impl OverlandTerrainKind {
    /// Speed multiplier (`1.00`, `0.75`, `0.50`, `0.25`).
    #[must_use]
    pub const fn speed_multiplier(self) -> f64 {
        match self {
            Self::PavedRoadCorridor => 1.00,
            Self::BrokenGroundLightWoods => 0.75,
            Self::DenseJungleMudSandRubble => 0.50,
            Self::DeepSnowMarshSteepScree => 0.25,
        }
    }

    /// Extra Exertion Marks per travel interval (`0`, `0`, `1`, `2`).
    #[must_use]
    pub const fn extra_exertion_marks(self) -> i32 {
        match self {
            Self::PavedRoadCorridor | Self::BrokenGroundLightWoods => 0,
            Self::DenseJungleMudSandRubble => 1,
            Self::DeepSnowMarshSteepScree => 2,
        }
    }
}

/// Evaluates effective overland travel speed (`km/h`) and daily range (`km` over `8 hours`) (`movement_vehicles_and_environment.md` §1.4).
#[must_use]
pub fn evaluate_overland_travel(
    pace: OverlandTravelPace,
    terrain: OverlandTerrainKind,
) -> (f64, f64, i32) {
    let base_km_per_hr = match pace {
        OverlandTravelPace::CautiousRecon => 3.0,
        OverlandTravelPace::StandardMarching => 5.0,
        OverlandTravelPace::ForcedMarchDoubleTime => 7.0,
        OverlandTravelPace::SustainedCrossCountryRun => 11.0,
    };
    let eff_km_per_hr = base_km_per_hr * terrain.speed_multiplier();
    let daily_8hr_km = eff_km_per_hr * 8.0;
    (eff_km_per_hr, daily_8hr_km, terrain.extra_exertion_marks())
}

// ============================================================================
// 6. Encumbrance & Load-Bearing Architecture: Tiers 0–4 (`movement_vehicles_and_environment.md` §2)
// ============================================================================

/// Ergonomic distribution method for carried equipment (`movement_vehicles_and_environment.md` §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarryErgonomicMethod {
    /// **Tailored Clothing / Articulated Body Armor / Duty Belt**: `×0.8`
    TailoredArmorOrDutyBelt,
    /// **Standard Tactical Webbing / Balanced Internal-Frame Rucksack**: `×1.0`
    BalancedWebbingOrRucksack,
    /// **Slung Bag / Asymmetric Duffel / Unbalanced Pack**: `×1.3`
    SlungOrAsymmetricPack,
    /// **Carried in Hands / Fireman-Carrying a Body**: `×1.5`
    CarriedInHandsOrBody,
}

impl CarryErgonomicMethod {
    /// Canonical ergonomic mass multiplier (`0.8`, `1.0`, `1.3`, `1.5`).
    #[must_use]
    pub const fn ergonomic_factor(self) -> f64 {
        match self {
            Self::TailoredArmorOrDutyBelt => 0.8,
            Self::BalancedWebbingOrRucksack => 1.0,
            Self::SlungOrAsymmetricPack => 1.3,
            Self::CarriedInHandsOrBody => 1.5,
        }
    }
}

/// A carried item or load group with physical mass (`kg`) and [`CarryErgonomicMethod`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CarriedLoadItem {
    /// Physical mass in kilograms (`kg`).
    pub mass_kg: f64,
    /// How the weight is worn or carried on the body.
    pub carry_method: CarryErgonomicMethod,
}

/// The 5 Canonical Encumbrance Tiers (`Tiers 0–4`) from `movement_vehicles_and_environment.md` §2.3,
/// unified with [`crate::action_economy::EncumbranceTier`].
pub type LoadEncumbranceTier = EncumbranceTier;

/// Complete evaluation of a character's Base Comfort Load ($L_{\text{base}}$), Effective Load ($L_{\text{eff}}$),
/// and resulting [`LoadEncumbranceTier`] (`movement_vehicles_and_environment.md` §2.1–§2.3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EncumbranceEvaluation {
    /// Base Uninhibited Comfort Load $L_{\text{base}} = (\text{MAS}/70.0) \times (\text{FOR} \times 1.2\text{ kg})$.
    pub base_comfort_load_kg: f64,
    /// Unmodified total physical mass carried in kilograms.
    pub raw_carried_mass_kg: f64,
    /// Effective Load $L_{\text{eff}} = \max(0, \sum (\text{Mass}_i \times \text{Ergonomic}_i) - \text{Exo Offset})$.
    pub effective_load_kg: f64,
    /// Load Ratio $L_{\text{eff}} / L_{\text{base}}$.
    pub load_ratio: f64,
    /// Derived Encumbrance Tier (`Tier 0..=4`).
    pub tier: LoadEncumbranceTier,
    /// Penalty to Tempo Cap ($T_{\max}$).
    pub tempo_cap_penalty: i32,
    /// Penalty to `KIN`.
    pub kin_penalty: i32,
    /// Physical `action_time_factor` delta ($\Delta_{\text{enc}}$).
    pub action_time_factor_delta: f64,
    /// Whether the character can perform an All-Out Sprint (`false` at `Tier 3+`).
    pub can_sprint: bool,
    /// Maximum deadlift / drag limit in kilograms ($8.0 \times L_{\text{base}}$).
    pub max_deadlift_drag_limit_kg: f64,
}

/// Computes Base Comfort Load $L_{\text{base}} = (\text{MAS}/70.0) \times (\text{FOR} \times 1.2\text{ kg})$ (`movement_vehicles_and_environment.md` §2.1).
#[must_use]
pub fn compute_base_comfort_load_kg(mas_kg: f64, for_stat: i32) -> f64 {
    let mas = mas_kg.max(20.0);
    let f = f64::from(for_stat.clamp(1, 25));
    (mas / 70.0) * (f * 1.2)
}

/// Evaluates a character's carried items and powered exoskeleton offset against their Base Comfort Load ($L_{\text{base}}$) (`movement_vehicles_and_environment.md` §2.1–§2.3).
#[must_use]
pub fn evaluate_encumbrance_load(
    mas_kg: f64,
    for_stat: i32,
    items: &[CarriedLoadItem],
    exo_frame_offset_kg: f64,
) -> EncumbranceEvaluation {
    let base_comfort_load_kg = compute_base_comfort_load_kg(mas_kg, for_stat);
    let raw_carried_mass_kg: f64 = items.iter().map(|i| i.mass_kg.max(0.0)).sum();
    let weighted_sum: f64 = items
        .iter()
        .map(|i| i.mass_kg.max(0.0) * i.carry_method.ergonomic_factor())
        .sum();
    let effective_load_kg = (weighted_sum - exo_frame_offset_kg.max(0.0)).max(0.0);
    let load_ratio = if base_comfort_load_kg > 0.0 {
        effective_load_kg / base_comfort_load_kg
    } else {
        0.0
    };

    let tier = LoadEncumbranceTier::from_load_ratio(load_ratio);

    EncumbranceEvaluation {
        base_comfort_load_kg,
        raw_carried_mass_kg,
        effective_load_kg,
        load_ratio,
        tier,
        tempo_cap_penalty: tier.tempo_cap_penalty(),
        kin_penalty: tier.kin_penalty(),
        action_time_factor_delta: tier.action_time_factor_delta(),
        can_sprint: tier.tier_number() < 3,
        max_deadlift_drag_limit_kg: 8.0 * base_comfort_load_kg,
    }
}

// ============================================================================
// 7. Vehicles, High-Speed Chases & Crash / G-Force Physics (`movement_vehicles_and_environment.md` §3)
// ============================================================================

/// Chase Range Bands in a vehicular pursuit or aerial dogfight (`movement_vehicles_and_environment.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChaseRangeBand {
    /// **Bumper-to-Bumper / Contact** (`0–3 m`): Ramming, PIT maneuvers, sideswiping, boarding.
    BumperToBumper,
    /// **Close Range** (`~15 m`): CQB window fire, tight urban weaving.
    Close15m,
    /// **Medium Range** (`~50 m`): Standard highway/street pursuit distance.
    Medium50m,
    /// **Long Range** (`150m+`): High-speed standoff pursuit; requires visual/sensor lock.
    Long150mPlus,
    /// **Lost Visual / Escaped**: Pursuit broken unless reacquired by aerial drone or comms net.
    LostVisual,
}

impl ChaseRangeBand {
    /// Shifts the range band by `steps` (`-1` closes distance toward `BumperToBumper`, `+1` opens distance toward `LostVisual`).
    #[must_use]
    pub const fn shift(self, steps: i32) -> Self {
        let idx = match self {
            Self::BumperToBumper => 0,
            Self::Close15m => 1,
            Self::Medium50m => 2,
            Self::Long150mPlus => 3,
            Self::LostVisual => 4,
        };
        let raw = idx + steps;
        let next = if raw < 0 {
            0
        } else if raw > 4 {
            4
        } else {
            raw
        };
        match next {
            0 => Self::BumperToBumper,
            1 => Self::Close15m,
            2 => Self::Medium50m,
            3 => Self::Long150mPlus,
            _ => Self::LostVisual,
        }
    }
}

/// Mechanical Envelope Ceilings of a ground, maritime, aerial, or orbital vehicle (`movement_vehicles_and_environment.md` §3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleEnvelopeProfile {
    /// Display name of the vehicle.
    pub name: String,
    /// Top Speed / Acceleration ceiling (`Vehicle Tempo Cap`).
    pub vehicle_tempo_cap: i32,
    /// Handling, Braking & Cornering ceiling (`Vehicle Control Cap`).
    pub vehicle_control_cap: i32,
    /// Chassis Mass & Structural Root ceiling (`Vehicle Force / Root Cap`).
    pub vehicle_force_cap: i32,
    /// Vehicle mass in kilograms (`kg`).
    pub mass_kg: f64,
    /// Armor Protection Rating (`PR`) of the chassis.
    pub armor_pr: i32,
}

/// Evaluates a driver/pilot's Effective Vehicular Triad Ceilings (bounded by both pilot attributes and the
/// [`VehicleEnvelopeProfile`]) and resolves a chase exchange (`movement_vehicles_and_environment.md` §3.1–§3.2).
#[must_use]
pub fn evaluate_vehicle_chase_exchange(
    vehicle: &VehicleEnvelopeProfile,
    pilot_tempo: i32,
    pilot_control: i32,
    pilot_force: i32,
    opponent_tempo: i32,
    opponent_control: i32,
    opponent_force_root: i32,
    current_band: ChaseRangeBand,
    wants_to_close_distance: bool,
    attempts_pit_or_ram: bool,
) -> (i32, i32, i32, ChaseRangeBand, bool) {
    let eff_t = pilot_tempo.clamp(0, vehicle.vehicle_tempo_cap);
    let eff_c = pilot_control.clamp(0, vehicle.vehicle_control_cap);
    let eff_f = pilot_force.clamp(0, vehicle.vehicle_force_cap);

    let net_tempo = eff_t - opponent_tempo.max(0);
    let band_shift = if net_tempo >= 3 {
        if wants_to_close_distance { -1 } else { 1 }
    } else if net_tempo <= -3 {
        if wants_to_close_distance { 1 } else { -1 }
    } else {
        0
    };
    let next_band = current_band.shift(band_shift);

    // PIT maneuver or Ramming succeeds at BumperToBumper when pilot's Force + Control advantage overcomes opponent's Vehicle Root
    let leveraged_pit_force = eff_f + (eff_c - opponent_control.max(0)).max(0);
    let pit_or_ram_succeeded = attempts_pit_or_ram
        && next_band == ChaseRangeBand::BumperToBumper
        && leveraged_pit_force >= opponent_force_root.max(1);

    (eff_t, eff_c, eff_f, next_band, pit_or_ram_succeeded)
}

/// Evaluates **Collision Kinetic Trauma** and **High-G Blackout (`G-LOC`)** (`movement_vehicles_and_environment.md` §3.3).
///
/// Returns `(raw_collision_blunt_trauma, g_loc_delta_ext, suffers_tunnel_vision_dazzled, suffers_g_loc_unconscious)`.
#[must_use]
pub fn evaluate_collision_and_g_force(
    relative_velocity_m_per_sec: f64,
    striking_mass_kg: f64,
    target_mass_kg: f64,
    has_crash_harness_or_airbag: bool,
    sustained_g_load: f64,
    wears_anti_g_suit: bool,
    crew_hlt_fit_check_margin: i32,
) -> (i32, i32, bool, bool) {
    let mass_ratio = (striking_mass_kg.max(1.0) / target_mass_kg.max(1.0)).clamp(0.25, 4.0);
    let raw_kinetic = (relative_velocity_m_per_sec.max(0.0) * 0.5 * mass_ratio).round() as i32;
    let collision_trauma = if has_crash_harness_or_airbag {
        (raw_kinetic - 6).max(0)
    } else {
        raw_kinetic
    };

    let over_g = (sustained_g_load - 5.0).max(0.0);
    let suit_relief = if wears_anti_g_suit { 4 } else { 0 };
    let g_loc_delta_ext = if sustained_g_load > 5.0 {
        ((over_g * 2.0).round() as i32 - suit_relief).max(0)
    } else {
        0
    };

    let (suffers_tunnel_vision_dazzled, suffers_g_loc_unconscious) = if sustained_g_load <= 5.0 {
        (false, false)
    } else if crew_hlt_fit_check_margin < -1 {
        (true, true)
    } else if crew_hlt_fit_check_margin < 0 {
        (true, false)
    } else {
        (false, false)
    };

    (
        collision_trauma,
        g_loc_delta_ext,
        suffers_tunnel_vision_dazzled,
        suffers_g_loc_unconscious,
    )
}

// ============================================================================
// 8. Environment & Survival Systems (`movement_vehicles_and_environment.md` §4)
// ============================================================================

/// Climate & operational activity level governing Daily Water Requirement ($W_{\text{req}}$, `movement_vehicles_and_environment.md` §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HydrationEnvironment {
    /// **Temperate Climate, Low Activity**: `3.0 Liters / Day`
    TemperateLowActivity,
    /// **Heavy Physical Labor / Forced March**: `5.0 Liters / Day`
    HeavyLaborOrForcedMarch,
    /// **Hot / Arid Desert or Tropical Jungle (`> 35°C`) + Tactical Gear**: `9.0 Liters / Day` (`8.0–10.0 L/day`)
    HotAridDesertOrJungleTactical,
}

impl HydrationEnvironment {
    /// Canonical Daily Water Requirement ($W_{\text{req}}$) in Liters (`3.0`, `5.0`, `9.0`).
    #[must_use]
    pub const fn daily_water_requirement_liters(self) -> f64 {
        match self {
            Self::TemperateLowActivity => 3.0,
            Self::HeavyLaborOrForcedMarch => 5.0,
            Self::HotAridDesertOrJungleTactical => 9.0,
        }
    }
}

/// The Five Stages of Dehydration (`Stages 0–4`, `movement_vehicles_and_environment.md` §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DehydrationStage {
    /// **Stage 0 (`< 2%`)**: Euhydrated baseline (`0` $\Delta_{\text{int}}$, `0` fatigue floor).
    Stage0Euhydrated,
    /// **Stage 1: Thirsty (`2–4%`)**: `+2` $\Delta_{\text{int}}$, `-1 FIT`, `-1 FOC`, `+1` `fatigue_level` floor.
    Stage1Thirsty,
    /// **Stage 2: Parched (`5–7%`)**: `+4` $\Delta_{\text{int}}$, `-3 FIT`, `-2 SPD/HLT/ANA`, `-2 KIN` (`Dizzy`), `+2` `fatigue_level` floor.
    Stage2Parched,
    /// **Stage 3: Severe (`8–10%`)**: `+8` $\Delta_{\text{int}}$, `-6 FIT/HLT/ANA/FOC`, `+4` `fatigue_level` floor, roll `HLT` every `4 hrs` or `Unconscious`.
    Stage3Severe,
    /// **Stage 4: Lethal (`> 12%`)**: `+10` $\Delta_{\text{int}}$, Comatose (`All Caps = 0`), fatal within `1–6 hours` without IV resuscitation.
    Stage4Lethal,
}

/// Mechanical evaluation of a character's Hydration / Dehydration state (`movement_vehicles_and_environment.md` §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DehydrationEvaluation {
    /// Daily water requirement $W_{\text{req}}$ in Liters.
    pub daily_requirement_liters: f64,
    /// Unreplaced water deficit in Liters.
    pub water_deficit_liters: f64,
    /// Resulting Dehydration Stage (`0..=4`).
    pub stage: DehydrationStage,
    /// Internal difficulty modifier $\Delta_{\text{int}}$ (`0`, `2`, `4`, `8`, `10`).
    pub delta_int: i32,
    /// Minimum `fatigue_level` floor that cannot be removed by rest until rehydrated (`0`, `1`, `2`, `4`, `5`).
    pub fatigue_level_floor: i32,
    /// Penalty to `FIT` (`0`, `-1`, `-3`, `-6`).
    pub fit_penalty: i32,
    /// Penalty to `FOC` (`0`, `-1`, `0`, `-6`).
    pub foc_penalty: i32,
    /// Penalty to `SPD`, `HLT`, `ANA` (`0`, `0`, `-2`, `-6`).
    pub spd_hlt_ana_penalty: i32,
    /// Whether the character is Comatose (`All Caps = 0`) at Stage 4.
    pub is_comatose: bool,
}

/// Evaluates a character's [`DehydrationStage`] from their daily water requirement ($W_{\text{req}}$) and
/// unreplaced water deficit (`movement_vehicles_and_environment.md` §4.1):
///
/// Advances `+1` stage for every `25%` of $W_{\text{req}}$ missed (or every `2.0 Liters` of unreplaced sweat deficit).
#[must_use]
pub fn evaluate_dehydration(
    env: HydrationEnvironment,
    water_consumed_liters: f64,
    extra_sweat_deficit_liters: f64,
) -> DehydrationEvaluation {
    let req = env.daily_water_requirement_liters();
    let missed =
        (req - water_consumed_liters.max(0.0)).max(0.0) + extra_sweat_deficit_liters.max(0.0);
    let stages_by_pct = ((missed / (0.25 * req)) + 1e-9).floor() as i32;
    let stages_by_liters = ((missed / 2.0) + 1e-9).floor() as i32;
    let stage_num = stages_by_pct.max(stages_by_liters).clamp(0, 4);

    let (
        stage,
        delta_int,
        fatigue_level_floor,
        fit_penalty,
        foc_penalty,
        spd_hlt_ana_penalty,
        is_comatose,
    ) = match stage_num {
        0 => (DehydrationStage::Stage0Euhydrated, 0, 0, 0, 0, 0, false),
        1 => (DehydrationStage::Stage1Thirsty, 2, 1, 1, 1, 0, false),
        2 => (DehydrationStage::Stage2Parched, 4, 2, 3, 0, 2, false),
        3 => (DehydrationStage::Stage3Severe, 8, 4, 6, 6, 6, false),
        _ => (DehydrationStage::Stage4Lethal, 10, 5, 10, 10, 10, true),
    };

    DehydrationEvaluation {
        daily_requirement_liters: req,
        water_deficit_liters: missed,
        stage,
        delta_int,
        fatigue_level_floor,
        fit_penalty,
        foc_penalty,
        spd_hlt_ana_penalty,
        is_comatose,
    }
}

/// Enforces all active **Fatigue Floors** (from Dehydration, Starvation, and Sleep Deprivation) so that
/// rest cannot reduce `fatigue_level` below the highest active floor (`movement_vehicles_and_environment.md` §4.1).
#[must_use]
pub fn enforce_fatigue_floor(current_fatigue_level: i32, active_floors: &[i32]) -> i32 {
    let max_floor = active_floors.iter().copied().max().unwrap_or(0).clamp(0, 5);
    current_fatigue_level.max(max_floor).clamp(0, 5)
}

/// Evaluation of Short-Term Fasting and Prolonged Caloric Deficit / Starvation (`movement_vehicles_and_environment.md` §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StarvationEvaluation {
    /// Internal difficulty modifier $\Delta_{\text{int}}$.
    pub delta_int: i32,
    /// Minimum `fatigue_level` floor (`+1` at `48h+` without food).
    pub fatigue_level_floor: i32,
    /// Penalty to `FOC` and `FIT` from glycogen depletion (`-2` at `48h+`).
    pub glycogen_foc_fit_penalty: i32,
    /// Number of 3-day / `10,000 kcal` catabolic starvation intervals suffered (`Days 4+`).
    pub catabolic_intervals: i32,
    /// Cumulative Body Mass (`MAS`) lost in kilograms (`1.5 kg` per catabolic interval).
    pub mass_lost_kg: f64,
    /// Cumulative erosion to `FOR`, `FIT`, and `HLT` (`1` point per catabolic interval).
    pub for_fit_hlt_erosion: i32,
    /// Whether fatal organ failure occurs (`FOR <= 0` or `HLT <= 0`).
    pub fatal_organ_failure: bool,
}

/// Evaluates Short-Term Fasting (`Days 1–3`) and Prolonged Starvation (`Days 4–30+` or cumulative `10,000 kcal` deficits)
/// (`movement_vehicles_and_environment.md` §4.2).
#[must_use]
pub fn evaluate_starvation_and_caloric_deficit(
    hours_without_food: i32,
    cumulative_kcal_deficit: i32,
    dis: i32,
    base_for: i32,
    base_hlt: i32,
) -> StarvationEvaluation {
    let hrs = hours_without_food.max(0);
    let days = hrs.div_euclid(24);

    let (base_delta_int, fatigue_level_floor, glycogen_foc_fit_penalty) = if hrs < 24 {
        (0, 0, 0)
    } else if hrs < 48 {
        (if dis < 10 { 1 } else { 0 }, 0, 0)
    } else {
        (2, 1, 2)
    };

    let intervals_by_days = if days >= 4 { days.div_euclid(3) } else { 0 };
    let intervals_by_kcal = cumulative_kcal_deficit.max(0).div_euclid(10_000);
    let catabolic_intervals = intervals_by_days.max(intervals_by_kcal);

    let weeks = catabolic_intervals.div_euclid(2);
    let delta_int = base_delta_int + 2 * weeks;
    let mass_lost_kg = 1.5 * f64::from(catabolic_intervals);
    let for_fit_hlt_erosion = catabolic_intervals;
    let fatal_organ_failure =
        (base_for - for_fit_hlt_erosion) <= 0 || (base_hlt - for_fit_hlt_erosion) <= 0;

    StarvationEvaluation {
        delta_int,
        fatigue_level_floor,
        glycogen_foc_fit_penalty,
        catabolic_intervals,
        mass_lost_kg,
        for_fit_hlt_erosion,
        fatal_organ_failure,
    }
}

/// The 5 Canonical Clothing / Suit Insulation (`CLO`) Tiers (`movement_vehicles_and_environment.md` §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClothingSuitTier {
    /// **Light Streetwear / Tropical Uniform**: Comfort `+18°C` to `+32°C`, $\Delta_{\text{cold}} = 0$, $\Delta_{\text{heat}} = 0$.
    LightStreetwearTropical,
    /// **Cold-Weather Field Jacket & Layers**: Comfort `-5°C` to `+15°C`, $\Delta_{\text{cold}} = -4$, $\Delta_{\text{heat}} = +4$ (`> 28°C`).
    ColdWeatherFieldJacket,
    /// **Extreme Arctic Parka / ECWCS Level 7**: Comfort `-35°C` to `-5°C`, $\Delta_{\text{cold}} = -8$, $\Delta_{\text{heat}} = +8$ (`> 20°C`).
    ExtremeArcticParkaEcwcs7,
    /// **Sealed NBC / Heavy Ballistic Armor**: Comfort `+10°C` to `+22°C`, $\Delta_{\text{cold}} = -2$, $\Delta_{\text{heat}} = +6$ (blocks sweat evaporation).
    SealedNbcOrHeavyArmor,
    /// **Climate-Controlled EVA / Exo-Rig**: Comfort `-150°C` to `+80°C`, Immune while powered.
    ClimateControlledEvaExoRig,
}

impl ClothingSuitTier {
    /// Comfort zone `(min_temp_c, max_temp_c)` in degrees Celsius.
    #[must_use]
    pub const fn comfort_zone_celsius(self) -> (f64, f64) {
        match self {
            Self::LightStreetwearTropical => (18.0, 32.0),
            Self::ColdWeatherFieldJacket => (-5.0, 15.0),
            Self::ExtremeArcticParkaEcwcs7 => (-35.0, -5.0),
            Self::SealedNbcOrHeavyArmor => (10.0, 22.0),
            Self::ClimateControlledEvaExoRig => (-150.0, 80.0),
        }
    }

    /// Cold Protection Bonus ($\Delta_{\text{cold}}$ applied to $\Delta_{\text{ext}}$ vs. Cold).
    #[must_use]
    pub const fn cold_protection_delta_ext(self) -> i32 {
        match self {
            Self::LightStreetwearTropical => 0,
            Self::ColdWeatherFieldJacket => -4,
            Self::ExtremeArcticParkaEcwcs7 => -8,
            Self::SealedNbcOrHeavyArmor => -2,
            Self::ClimateControlledEvaExoRig => -20,
        }
    }

    /// Heat Retention Penalty ($\Delta_{\text{heat}}$ applied to $\Delta_{\text{ext}}$ in high heat).
    #[must_use]
    pub const fn heat_retention_delta_ext(self, ambient_temp_c: f64) -> i32 {
        match self {
            Self::LightStreetwearTropical | Self::ClimateControlledEvaExoRig => 0,
            Self::ColdWeatherFieldJacket => {
                if ambient_temp_c > 28.0 {
                    4
                } else {
                    0
                }
            }
            Self::ExtremeArcticParkaEcwcs7 => {
                if ambient_temp_c > 20.0 {
                    8
                } else {
                    0
                }
            }
            Self::SealedNbcOrHeavyArmor => 6,
        }
    }
}

/// Cold Exposure & Hypothermia stages (`movement_vehicles_and_environment.md` §4.3.A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothermiaStage {
    /// Normothermic (`Core ~37.0°C`)
    None,
    /// **1. Cold Stress & Shivering (`Core 36.0–35.0°C`)**: `DEX` and `TAC` `-3`, `+2` $\Delta_{\text{int}}$, `+1` `fatigue_level`.
    Stage1ColdStressShivering,
    /// **2. Mild-to-Moderate Hypothermia (`Core 34.9–32.0°C`)**: `DEX, TAC, SPD, KIN, ANA, FOC` `-6`, `+0.30` `action_time_factor`, `+6` $\Delta_{\text{int}}`.
    Stage2MildModerateHypothermia,
    /// **3. Severe Hypothermia (`Core < 32.0°C`)**: Delirium, `Unconsciousness`, and ventricular fibrillation risk.
    Stage3SevereHypothermia,
}

/// Heat Exposure, Heat Exhaustion & Heatstroke stages (`movement_vehicles_and_environment.md` §4.3.B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HyperthermiaStage {
    /// Normothermic (`Core ~37.0°C`)
    None,
    /// **Stage 1: Heat Cramps**: `-2` to $T_{\max}$ and $F_{\max}$, `+1` `fatigue_level`.
    Stage1HeatCramps,
    /// **Stage 2: Heat Exhaustion (`Core 38.5–39.9°C`)**: `Dizzy` (`-4 KIN`), `Nausea` (`+4` $\Delta_{\text{int}}$), `+3` `fatigue_level`.
    Stage2HeatExhaustion,
    /// **Stage 3: Heatstroke (`Core >= 40.0°C`)**: CNS delirium, seizures, `Coma`; inflicts `+1` Tier of systemic organ trauma every `15 minutes`.
    Stage3Heatstroke,
}

/// Evaluates Cold Exposure (`HypothermiaStage`) or Heat Exposure (`HyperthermiaStage`) penalties (`movement_vehicles_and_environment.md` §4.3).
///
/// Returns `(delta_int, dex_tac_penalty, all_stat_penalty, action_time_factor_added, added_fatigue_level, is_unconscious_or_coma)`.
#[must_use]
pub const fn evaluate_thermal_stage_penalties(
    hypothermia: HypothermiaStage,
    hyperthermia: HyperthermiaStage,
) -> (i32, i32, i32, f64, i32, bool) {
    match (hypothermia, hyperthermia) {
        (HypothermiaStage::Stage3SevereHypothermia, _)
        | (_, HyperthermiaStage::Stage3Heatstroke) => (10, 6, 6, 0.50, 5, true),
        (HypothermiaStage::Stage2MildModerateHypothermia, _) => (6, 6, 6, 0.30, 2, false),
        (_, HyperthermiaStage::Stage2HeatExhaustion) => (4, 0, 4, 0.0, 3, false),
        (HypothermiaStage::Stage1ColdStressShivering, _) => (2, 3, 0, 0.0, 1, false),
        (_, HyperthermiaStage::Stage1HeatCramps) => (0, 0, 0, 0.0, 1, false),
        (HypothermiaStage::None, HyperthermiaStage::None) => (0, 0, 0, 0.0, 0, false),
    }
}

/// Evaluation of Sleep Deprivation & Circadian Debt (`movement_vehicles_and_environment.md` §4.4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepDeprivationEvaluation {
    /// Total hours awake without required sleep (`6–8h` standard, `4h` minimum with `DIS` check).
    pub hours_awake: i32,
    /// Internal difficulty modifier $\Delta_{\text{int}}$ (`0`, `+2` at `24h`, `+4` at `48h`, `+8` at `72h+`).
    pub delta_int: i32,
    /// Penalty to `FOC`, `INS`, `ITU`, `OODA` (and `ANA`, `MEM` at `48h+`; all Mental & Sensory at `72h+`).
    pub mental_sensory_ooda_penalty: i32,
    /// Minimum `fatigue_level` floor (`0` at `<48h`, `+2` at `48h+`).
    pub fatigue_level_floor: i32,
    /// Whether being stationary for `> 5 minutes` requires a `DIS` check ($\Delta_{\text{ext}} = +6$) to avoid involuntary Microsleep (`Unconscious`).
    pub requires_microsleep_dis_check: bool,
}

/// Evaluates Sleep Deprivation penalties from `hours_awake` (`movement_vehicles_and_environment.md` §4.4.1).
#[must_use]
pub const fn evaluate_sleep_deprivation(hours_awake: i32) -> SleepDeprivationEvaluation {
    let hrs = if hours_awake < 0 { 0 } else { hours_awake };
    if hrs >= 72 {
        SleepDeprivationEvaluation {
            hours_awake: hrs,
            delta_int: 8,
            mental_sensory_ooda_penalty: 6,
            fatigue_level_floor: 2,
            requires_microsleep_dis_check: true,
        }
    } else if hrs >= 48 {
        SleepDeprivationEvaluation {
            hours_awake: hrs,
            delta_int: 4,
            mental_sensory_ooda_penalty: 3,
            fatigue_level_floor: 2,
            requires_microsleep_dis_check: false,
        }
    } else if hrs >= 24 {
        SleepDeprivationEvaluation {
            hours_awake: hrs,
            delta_int: 2,
            mental_sensory_ooda_penalty: 1,
            fatigue_level_floor: 0,
            requires_microsleep_dis_check: false,
        }
    } else {
        SleepDeprivationEvaluation {
            hours_awake: hrs,
            delta_int: 0,
            mental_sensory_ooda_penalty: 0,
            fatigue_level_floor: 0,
            requires_microsleep_dis_check: false,
        }
    }
}

/// Evaluation of Altitude Hypoxia and Hard Vacuum Decompression (`movement_vehicles_and_environment.md` §4.4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AltitudeVacuumEvaluation {
    /// Penalty to `Fitness (FIT)` (`-2` per `1,000 meters` above `2,000 m` when unacclimatized above `2,500 m`).
    pub fit_penalty: i32,
    /// Whether Acute Mountain Sickness (`Dizzy`, `Nausea`) and HAPE/HACE checks apply (`> 5,500 m` without $\text{O}_2$).
    pub acute_mountain_sickness_risk: bool,
    /// Useful consciousness window in ticks when exposed to Hard Vacuum (`48–64 Ticks` = `12–16s`).
    pub vacuum_useful_consciousness_ticks: i32,
    /// Whether the character has lost consciousness in Hard Vacuum.
    pub vacuum_unconscious: bool,
    /// Cumulative `HLT` cold/barotrauma erosion suffered after losing consciousness in Hard Vacuum (`-2 HLT` per `20 ticks`).
    pub vacuum_hlt_erosion: i32,
}

/// Evaluates High-Altitude Hypoxia (`> 2,500 m`) and Hard Vacuum (`0 atm`) exposure (`movement_vehicles_and_environment.md` §4.4.2).
#[must_use]
pub const fn evaluate_altitude_and_vacuum_hypoxia(
    altitude_meters: i32,
    has_supplemental_oxygen_or_acclimatized: bool,
    is_hard_vacuum: bool,
    ticks_in_vacuum: i32,
    fit: i32,
) -> AltitudeVacuumEvaluation {
    if is_hard_vacuum {
        let fit_c = if fit < 1 {
            1
        } else if fit > 20 {
            20
        } else {
            fit
        };
        let useful_ticks = 48 + ((fit_c - 10) * 16).div_euclid(10);
        let clamped_useful = if useful_ticks < 48 {
            48
        } else if useful_ticks > 64 {
            64
        } else {
            useful_ticks
        };
        let t = if ticks_in_vacuum < 0 {
            0
        } else {
            ticks_in_vacuum
        };
        let vacuum_unconscious = t >= clamped_useful;
        let overrun = if t > clamped_useful {
            t - clamped_useful
        } else {
            0
        };
        let vacuum_hlt_erosion = 2 * overrun.div_euclid(20);
        return AltitudeVacuumEvaluation {
            fit_penalty: 20,
            acute_mountain_sickness_risk: true,
            vacuum_useful_consciousness_ticks: clamped_useful,
            vacuum_unconscious,
            vacuum_hlt_erosion,
        };
    }

    if has_supplemental_oxygen_or_acclimatized || altitude_meters <= 2500 {
        return AltitudeVacuumEvaluation {
            fit_penalty: 0,
            acute_mountain_sickness_risk: false,
            vacuum_useful_consciousness_ticks: 0,
            vacuum_unconscious: false,
            vacuum_hlt_erosion: 0,
        };
    }

    let meters_above_2000 = altitude_meters - 2000;
    let fit_penalty = 2 * meters_above_2000.div_euclid(1000);
    let acute_mountain_sickness_risk = altitude_meters > 5500;

    AltitudeVacuumEvaluation {
        fit_penalty,
        acute_mountain_sickness_risk,
        vacuum_useful_consciousness_ticks: 0,
        vacuum_unconscious: false,
        vacuum_hlt_erosion: 0,
    }
}

/// Exposure vector for a Toxin, Venom, Chemical Agent, or Pathogen (`movement_vehicles_and_environment.md` §4.4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxinVector {
    /// Airborne gas, aerosol, smoke, or spore (`Inhalation`).
    Inhalation,
    /// Percutaneous skin/mucosa absorption (`Contact`).
    Contact,
    /// Swallowed in food or water (`Ingestion`).
    Ingestion,
    /// Dart, fang, needle, or contaminated blade (`InjectionOrWound`).
    InjectionOrWound,
}

/// Canonical 4-Parameter Toxin, Chemical Agent, or Pathogen profile (`movement_vehicles_and_environment.md` §4.4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToxinOrPathogenProfile {
    /// Agent name (`"Tear Gas (CS)"`, `"Sarin Nerve Agent"`, etc.).
    pub name: &'static str,
    /// Parameter 1: Transmission / Exposure Vector.
    pub vector: ToxinVector,
    /// Parameter 2: Onset delay in Ticks before symptoms take effect.
    pub onset_delay_ticks: i32,
    /// Parameter 3: Virulence ($\Delta_{\text{ext}}$ on the victim's `HLT` cascade save).
    pub virulence_delta_ext: i32,
    /// Parameter 4: Target physiological status effects inflicted on a failed `HLT` save.
    pub target_conditions: Vec<PhysiologicalStatusCondition>,
    /// Narrative description of target effects.
    pub effect_summary: &'static str,
}

/// Returns the canonical example Toxin & Chemical Agent profiles from `movement_vehicles_and_environment.md` §4.4.3.
#[must_use]
pub fn canonical_toxin_catalog() -> Vec<ToxinOrPathogenProfile> {
    vec![
        ToxinOrPathogenProfile {
            name: "Tear Gas / CS Riot Agent",
            vector: ToxinVector::Inhalation,
            onset_delay_ticks: 4,
            virulence_delta_ext: 2,
            target_conditions: vec![
                PhysiologicalStatusCondition::Blinded,
                PhysiologicalStatusCondition::DazzledBlurredVision,
                PhysiologicalStatusCondition::NauseaSickened,
            ],
            effect_summary: "Onset 4 ticks, Virulence +2, Effect: Blinded/Dazzled + Coughing",
        },
        ToxinOrPathogenProfile {
            name: "Neurotoxin / Sarin (GB) Nerve Agent",
            vector: ToxinVector::Inhalation,
            onset_delay_ticks: 12,
            virulence_delta_ext: 10,
            target_conditions: vec![
                PhysiologicalStatusCondition::Paralyzed,
                PhysiologicalStatusCondition::UnconsciousComatose,
            ],
            effect_summary: "Onset 12 ticks, Virulence +10, Effect: Paralysis + Asphyxiation + Convulsions",
        },
    ]
}

/// Acute Radiation Sickness (`ARS`) tiers by cumulative Sievert (`Sv`) dose (`movement_vehicles_and_environment.md` §4.4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadiationExposureTier {
    /// `< 0.5 Sv`: Subclinical (no acute symptoms).
    Subclinical,
    /// `0.5 – 1.5 Sv` (*Mild ARS*): `Nausea` and `+1` `fatigue_level` after `6–12 hours`; temporary `-2 HLT`.
    MildArs,
    /// `1.5 – 4.0 Sv` (*Moderate-Severe Hematopoietic ARS*): Vomiting within `2 hours`, hair loss, `HLT` halved against infection over `2–6 weeks`.
    ModerateSevereHematopoieticArs,
    /// `> 4.0 / 6.0+ Sv` (*Lethal GI / Neurovascular ARS*): Acute shock, internal mucosal hemorrhage, fatal within days without nanomedical regeneration.
    LethalGiNeurovascularArs,
}

/// Evaluates Ionizing Radiation (`Sv`) exposure tier and physiological penalties (`movement_vehicles_and_environment.md` §4.4.4).
///
/// Returns `(tier, hlt_penalty, hlt_halved_vs_infection, added_fatigue_level, inflicts_nausea, is_lethal_without_nanomed)`.
#[must_use]
pub fn evaluate_radiation_exposure(
    cumulative_sieverts_sv: f64,
) -> (RadiationExposureTier, i32, bool, i32, bool, bool) {
    let sv = cumulative_sieverts_sv.max(0.0);
    if sv < 0.5 {
        (
            RadiationExposureTier::Subclinical,
            0,
            false,
            0,
            false,
            false,
        )
    } else if sv <= 1.5 {
        (RadiationExposureTier::MildArs, 2, false, 1, true, false)
    } else if sv <= 4.0 {
        (
            RadiationExposureTier::ModerateSevereHematopoieticArs,
            4,
            true,
            2,
            true,
            false,
        )
    } else {
        (
            RadiationExposureTier::LethalGiNeurovascularArs,
            10,
            true,
            4,
            true,
            sv > 6.0,
        )
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

        // Broad zone +2 is waived when attacker Control exceeds defender Control by +3+
        assert_eq!(
            compute_called_shot_delta_ext(CalledShotZoneTier::BroadZone, 12, 9),
            0
        );
        assert_eq!(
            compute_called_shot_delta_ext(CalledShotZoneTier::BroadZone, 11, 9),
            2
        );
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
        assert_eq!(f0.delta_int, 0);
        assert_eq!(f0.delta_under, 6);
        assert!(f0.can_sprint);

        let f3 = evaluate_fatigue_level(3);
        assert_eq!(f3.delta_int, 6);
        assert_eq!(f3.delta_under, 12);
        assert_eq!(f3.tactical_pool_drain, -6);
        assert_eq!(f3.spd_and_for_cap_penalty, 2);
        assert!(f3.can_sprint);

        let f4 = evaluate_fatigue_level(4);
        assert!(!f4.can_sprint);
        assert!(!f4.is_unconscious);

        let f5 = evaluate_fatigue_level(5);
        assert!(f5.is_unconscious);

        // FIT 10 calm breath holding = 400 ticks (100s); vigorous = 160 ticks (40s)
        let calm = evaluate_breath_holding_and_asphyxiation(10, 0, true, false, 400, false);
        assert_eq!(calm.breath_holding_capacity_ticks, 400);
        assert_eq!(calm.fatigue_levels_gained, 0);
        assert!(!calm.is_unconscious);

        // Vigorous at FIT 10 (160 ticks capacity), after 260 ticks (100 ticks overrun -> +5 fatigue -> Unconscious!)
        let vig = evaluate_breath_holding_and_asphyxiation(10, 0, true, true, 260, false);
        assert_eq!(vig.breath_holding_capacity_ticks, 160);
        assert_eq!(vig.fatigue_levels_gained, 5);
        assert!(vig.is_unconscious);

        // Vascular blood choke at FIT 10 -> unconscious at 24 ticks (6 seconds)!
        let choke = evaluate_breath_holding_and_asphyxiation(10, 0, true, true, 28, true);
        assert!(choke.is_unconscious);
    }

    #[test]
    fn test_wounds_fractures_hemorrhage_and_wsr_saves() {
        // Hairline fracture aggravation when allocating > 50% ceiling and failing HLT check
        let (snapped, new_tier) = check_hairline_fracture_aggravation(7, 12, false);
        assert!(snapped);
        assert_eq!(new_tier, StructuralWoundTier::Tier3SevereFracture);

        // Grade 4 Arterial Severance loses 1.0 Unit per 4 ticks -> 5.0 Units in 20 ticks (Class IV Coma!)
        let bleed =
            evaluate_blood_loss_progression(0.0, &[HemorrhageGrade::Grade4ArterialSeverance], 20);
        assert!((bleed.total_units_lost - 5.0).abs() < 1e-6);
        assert_eq!(
            bleed.hypovolemic_class,
            HypovolemicShockClass::ClassIVLethalComa
        );
        assert!(bleed.immediate_unconsciousness_and_coma);
        assert_eq!(bleed.ticks_until_cardiac_arrest, Some(240));

        // WSR Save: HLT 12, DIS 12 -> C_P = 12 (TN_P = 19), Wound Shock 2 -> Delta_ext = +4 (TN_GM = 17)
        let save_win = evaluate_wound_shock_save(12, 12, 2, 2, 0, &[(12, 10), (12, 10)], (8, 7));
        assert!(save_win.stays_conscious);
        assert_eq!(save_win.effective_wound_shock_index, 1); // ExtraordinarySuccess reduces shock by -1!

        // CleanFailure (19 vs TN_P = 21, miss < 4): Stunned 8 ticks, drops Prone, triggers Morale Check
        let save_fail = evaluate_wound_shock_save(10, 10, 1, 2, 0, &[(10, 9)], (10, 9));
        assert!(save_fail.stays_conscious);
        assert_eq!(save_fail.stunned_ticks, 8);
        assert!(save_fail.drops_prone);
        assert!(save_fail.triggers_morale_check);

        // BotchFailure (10 vs TN_P = 21): Neurogenic syncope -> falls Unconscious!
        let save_botch = evaluate_wound_shock_save(10, 10, 1, 2, 0, &[(5, 5)], (10, 9));
        assert!(!save_botch.stays_conscious);
        assert!(save_botch.falls_unconscious);
    }

    #[test]
    fn test_movement_gaits_and_encumbrance_tiers_0_to_4() {
        // Human Average (SPD 10, HGT 1.75 m): V_sprint = 7.50 m/s (27.0 km/h, 1.875 m/Tick)
        let sp10 = compute_stride_and_sprint_velocity(10, 1.75);
        assert!((sp10.v_sprint_m_per_sec - 7.50).abs() < 1e-6);
        assert!((sp10.v_sprint_m_per_tick - 1.875).abs() < 1e-6);

        // Peak Human (SPD 18, HGT 1.75 m): V_sprint = 10.30 m/s (2.575 m/Tick)
        let sp18 = compute_stride_and_sprint_velocity(18, 1.75);
        assert!((sp18.v_sprint_m_per_sec - 10.30).abs() < 1e-6);

        // Bounding Run (65%): +4 Recoil, caps ranged Control at floor(C_max / 2)
        let bound =
            evaluate_combat_movement_gait(CombatMovementGait::BoundingRunHustle, 10, 1.75, 11);
        assert_eq!(bound.ranged_recoil_adder, 4);
        assert_eq!(bound.effective_ranged_control_cap, 5);
        assert_eq!(bound.exertion_marks_per_20_ticks, 1);

        // Encumbrance: Human Average (MAS 70 kg, FOR 10) -> L_base = 12.0 kg
        // Strong Operator (MAS 85 kg, FOR 14) -> L_base = 20.4 kg
        assert!((compute_base_comfort_load_kg(70.0, 10) - 12.0).abs() < 1e-6);
        assert!((compute_base_comfort_load_kg(85.0, 14) - 20.4).abs() < 1e-6);

        // 24 kg in balanced rucksack (x1.0) for 70 kg, FOR 10 -> ratio 2.0 -> Tier 2 (Tactical Load: -3 T_max, -2 KIN, +0.10 time factor)
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
        assert_eq!(enc_t2.tempo_cap_penalty, 3);
        assert_eq!(enc_t2.kin_penalty, 2);
        assert!((enc_t2.action_time_factor_delta - 0.10).abs() < 1e-6);

        // With -20 kg Valkyrie Exo-Rig offset -> effective load = 4.0 kg -> Tier 0 Unencumbered!
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
        // Hot desert (9.0 L/day need), drank 4.0 L -> 5.0 L deficit -> Stage 2 Parched (+4 Delta_int, +2 Fatigue Floor)
        let dehyd = evaluate_dehydration(
            HydrationEnvironment::HotAridDesertOrJungleTactical,
            4.0,
            0.0,
        );
        assert_eq!(dehyd.stage, DehydrationStage::Stage2Parched);
        assert_eq!(dehyd.delta_int, 4);
        assert_eq!(dehyd.fatigue_level_floor, 2);
        assert_eq!(enforce_fatigue_floor(0, &[dehyd.fatigue_level_floor]), 2);

        // 6 days without food -> 2 catabolic intervals (-3.0 kg mass, -2 FOR/FIT/HLT)
        let starv = evaluate_starvation_and_caloric_deficit(6 * 24, 18_000, 10, 10, 10);
        assert_eq!(starv.catabolic_intervals, 2);
        assert!((starv.mass_lost_kg - 3.0).abs() < 1e-6);
        assert_eq!(starv.for_fit_hlt_erosion, 2);

        // 72 hours awake -> +8 Delta_int, -6 mental/sensory, +2 fatigue floor, microsleep check
        let sleep = evaluate_sleep_deprivation(72);
        assert_eq!(sleep.delta_int, 8);
        assert_eq!(sleep.mental_sensory_ooda_penalty, 6);
        assert!(sleep.requires_microsleep_dis_check);

        // Hard vacuum at FIT 10 -> 48 ticks useful consciousness; at 68 ticks -> unconscious + -2 HLT erosion
        let vac = evaluate_altitude_and_vacuum_hypoxia(0, false, true, 68, 10);
        assert_eq!(vac.vacuum_useful_consciousness_ticks, 48);
        assert!(vac.vacuum_unconscious);
        assert_eq!(vac.vacuum_hlt_erosion, 2);
    }
}
