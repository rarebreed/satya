//! # Physiological & Sensory Status Conditions, Acute Mental Trauma & Medical Triage
//! (docs/ttrpg/06-health-and-survival/health_and_trauma.md §4, §5.1–§5.2, §6.1–§6.3).
//!
//! Separated from [crate::health_survival] so anatomical wounds/hemorrhage (§1–§3) and
//! systemic status conditions / mental trauma / medical recovery (§4–§6) each fit within
//! a single agent read window.

use serde::{Deserialize, Serialize};

use crate::health_survival::StructuralWoundTier;
use crate::primitives::{ConditionImpact, ConditionSource, Triad};
// ============================================================================
// 4. Physiological, Sensory & Acute Mental Status Conditions (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §4 – §6.3)
// ============================================================================

/// Fracture or joint integrity state for an anatomical hit location (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

impl PhysiologicalStatusCondition {
    pub const ALL: [Self; 11] = [
        Self::DazzledBlurredVision,
        Self::Blinded,
        Self::Tinnitus,
        Self::Deafened,
        Self::StunnedDazed,
        Self::Concussed,
        Self::DizzyVertigo,
        Self::NauseaSickened,
        Self::PinnedImmobilized,
        Self::Paralyzed,
        Self::UnconsciousComatose,
    ];

    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DazzledBlurredVision => "DAZZLED",
            Self::Blinded => "BLINDED",
            Self::Tinnitus => "TINNITUS",
            Self::Deafened => "DEAFENED",
            Self::StunnedDazed => "STUNNED",
            Self::Concussed => "CONCUSSED",
            Self::DizzyVertigo => "DIZZY",
            Self::NauseaSickened => "NAUSEA",
            Self::PinnedImmobilized => "PINNED",
            Self::Paralyzed => "PARALYZED",
            Self::UnconsciousComatose => "UNCONSCIOUS",
        }
    }

    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::DazzledBlurredVision => "Dazzled (SIG -5)",
            Self::Blinded => "Blinded (SIG=0)",
            Self::Tinnitus => "Tinnitus (HEA -6)",
            Self::Deafened => "Deafened (HEA=0)",
            Self::StunnedDazed => "Stunned (+8t)",
            Self::Concussed => "Concussed (Mental -4)",
            Self::DizzyVertigo => "Dizzy (KIN -6)",
            Self::NauseaSickened => "Nausea (+4 Δ_int)",
            Self::PinnedImmobilized => "Pinned (T=0)",
            Self::Paralyzed => "Paralyzed (T/C/F=0)",
            Self::UnconsciousComatose => "Unconscious (All=0)",
        }
    }

    #[must_use]
    pub fn from_code_str(code: &str) -> Option<Self> {
        match code.trim().to_ascii_uppercase().as_str() {
            "DAZZLED" => Some(Self::DazzledBlurredVision),
            "BLINDED" => Some(Self::Blinded),
            "TINNITUS" => Some(Self::Tinnitus),
            "DEAFENED" => Some(Self::Deafened),
            "STUNNED" => Some(Self::StunnedDazed),
            "CONCUSSED" => Some(Self::Concussed),
            "DIZZY" => Some(Self::DizzyVertigo),
            "NAUSEA" => Some(Self::NauseaSickened),
            "PINNED" => Some(Self::PinnedImmobilized),
            "PARALYZED" => Some(Self::Paralyzed),
            "UNCONSCIOUS" => Some(Self::UnconsciousComatose),
            _ => None,
        }
    }
}

/// Aggregated characteristic, ceiling, and clock modifiers from active [`PhysiologicalStatusCondition`]s
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §4 Table).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregatedStatusEffects {
    pub sig_locked_zero: bool,
    /// Subtractive penalty to `SIG` (`-5` when `Dazzled`).
    pub sig_penalty: i32,
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

impl AcuteMentalTraumaTier {
    pub const ALL: [Self; 4] = [
        Self::Tier1AcuteMentalShockOodaFreeze,
        Self::Tier2PanicFightOrFlight,
        Self::Tier3DissociationShellShock,
        Self::Tier4Catatonia,
    ];

    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Tier1AcuteMentalShockOodaFreeze => "TIER_1",
            Self::Tier2PanicFightOrFlight => "TIER_2",
            Self::Tier3DissociationShellShock => "TIER_3",
            Self::Tier4Catatonia => "TIER_4",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Tier1AcuteMentalShockOodaFreeze => "Tier 1 — OODA Freeze (+9t)",
            Self::Tier2PanicFightOrFlight => "Tier 2 — Panic / Fight-or-Flight (Pool -4)",
            Self::Tier3DissociationShellShock => "Tier 3 — Dissociation / Shell Shock (+0.30 ATF)",
            Self::Tier4Catatonia => "Tier 4 — Catatonia (Total Lock T/C/F = 0)",
        }
    }

    #[must_use]
    pub fn from_code_str(code: &str) -> Option<Self> {
        match code.trim().to_ascii_uppercase().as_str() {
            "TIER_1" => Some(Self::Tier1AcuteMentalShockOodaFreeze),
            "TIER_2" => Some(Self::Tier2PanicFightOrFlight),
            "TIER_3" => Some(Self::Tier3DissociationShellShock),
            "TIER_4" => Some(Self::Tier4Catatonia),
            _ => None,
        }
    }
}

/// Mechanical evaluation of an [`AcuteMentalTraumaTier`]
/// (`docs/ttrpg/06-health-and-survival/health_and_trauma.md` §5.1–§5.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcuteMentalTraumaEffect {
    pub tier: AcuteMentalTraumaTier,
    pub ooda_freeze_ticks: i32,
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
        AcuteMentalTraumaTier::Tier2PanicFightOrFlight => (
            0,
            -4,
            6,
            0,
            0.0,
            is_cornered && agg_score >= 13,
            false,
            0,
            0,
        ),
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
