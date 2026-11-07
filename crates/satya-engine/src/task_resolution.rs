//! # Task Resolution, Gate Classes, Tool Tiers, Synergy & Project Clocks (`02-core-mechanics`)
//!
//! Canonical Rust implementation of Sections 2, 3, 6, 7, and 8 of
//! `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`:
//! - **Task Gate Classifications (`§2.1`)**: [`TaskGateClass`] (`ClassA`, `ClassB`, `ClassC { min_skill_n }`).
//! - **Tool Dependency & Equipment Quality Tiers (`§2.2`)**: [`ToolDependency`], [`ToolQualityTier`], and [`resolve_task_gate_and_tools`].
//! - **Composite & Supporting Characteristics (`§3.1–§3.2`)**: [`compute_composite_characteristic`] and [`compute_supporting_characteristic_shortfall_penalty`].
//! - **Complementary Skills (`KNO -> PHY`, `§6.1`), Dual-Skill Gates (`§6.2`) & Extended Project Clocks (`§6.3`)**:
//!   [`ComplementarySynergyResult`], [`evaluate_complementary_setup_synergy`], [`compute_dual_skill_pool`], and [`ExtendedProjectClock`].
//! - **Canonical Worked Examples (`§7.1–§7.3`) & 22-Entry Master Task Requirements Catalog (`§8`)**:
//!   [`CanonicalTaskPreset`], [`CANONICAL_TASK_PRESETS`], [`MasterTaskBenchmark`], and [`MASTER_TASK_BENCHMARKS`].

use serde::{Deserialize, Serialize};

use crate::characteristics::{CharacterCharacteristics, CharacteristicCode};
use crate::dice_probabilities::{
    CascadeOutcomeKind, CascadeRollOutcome, CascadeTaskParams, ContestedCascadeRollOutcome,
    ContestedCascadeTaskParams, ContestedOutcomeKind, ContestedTieRule,
};

// ---------------------------------------------------------------------------
// 11. Task Gate Classifications (Class A / B / C) & Tool Dependency Tiers
// ---------------------------------------------------------------------------

/// Task Gate Classification per `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§2.1)
/// and `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskGateClass {
    /// **Class A — Open / Talent Tasks (`N >= 0`)**:
    /// Actions any conscious character can attempt without formal training.
    /// - If `trained_skill_n == 0`, the character may either roll their Talent rating
    ///   $N_{\text{talent}} \in [1, 10]$ with $\Delta_{\text{ext}} = 0$, or default to $N = 1$ pair
    ///   with a $+4$ external difficulty penalty ($\Delta_{\text{ext}} = +4$).
    ClassA,
    /// **Class B — Trained / Defaultable Tasks (`N >= 1`, or Default from Related Skill)**:
    /// Technical, scholarly, or tactical actions requiring procedural familiarity.
    /// - Cannot be attempted at `trained_skill_n == 0` unless defaulting from a closely related
    ///   parent or sibling skill at $N_{\text{parent}} \ge 2$, which grants
    ///   $N = \max(1, N_{\text{parent}} - 1)$ pairs at an external difficulty penalty of
    ///   $\Delta_{\text{ext}} \in [+4, +6]$ (canonical default `+4`).
    ClassB,
    /// **Class C — Hard-Gated Specialist Tasks (`N >= min_skill_n`, `[Locked]`)**:
    /// High-complexity specialist procedures. Strictly requires `trained_skill_n >= min_skill_n`
    /// (`1..=4`) AND appropriate tools if `[Tool: Mandatory]`. No Talent or sibling defaulting is permitted.
    ClassC {
        min_skill_n: usize,
    },
}

impl TaskGateClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ClassA => "Class A",
            Self::ClassB => "Class B",
            Self::ClassC { .. } => "Class C",
        }
    }
}

/// Tool Dependency classification per `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDependency {
    /// `[Tool: None]` — Purely physical, perceptual, social, or cognitive tasks requiring no equipment.
    None,
    /// `[Tool: Situational]` — Can be attempted bare-handed or with improvised gear at a heavy $\Delta_{\text{tool}}$ penalty.
    Situational,
    /// `[Tool: Mandatory]` — Physically impossible without appropriate instruments (`[Locked]` if `NoTools`).
    Mandatory,
}

impl ToolDependency {
    pub const fn tag(self) -> &'static str {
        match self {
            Self::None => "[Tool: None]",
            Self::Situational => "[Tool: Situational]",
            Self::Mandatory => "[Tool: Mandatory]",
        }
    }
}

/// Equipment quality tier and known player tool modifier ($\Delta_{\text{tool}}$) per
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolQualityTier {
    /// Automated diagnostic/fabrication rig or dedicated AI co-processor (`-4` to $\Delta_{\text{tool}}$).
    AiAssistedRig,
    /// Precision-calibrated, custom-fitted, or high-grade military/laboratory kit (`-2` to $\Delta_{\text{tool}}$).
    Masterwork,
    /// Proper, well-maintained professional tools (`0` to $\Delta_{\text{tool}}$).
    #[default]
    StandardKit,
    /// Degraded, uncalibrated, or mismatched commercial tools (`+2` to $\Delta_{\text{tool}}$).
    WornSubstandard,
    /// Makeshift tools (`+4` to $\Delta_{\text{tool}}$; triggers automatic 1-step wear/degradation on any Floor Breach $Y_n < 2$).
    Improvised,
    /// Heavily makeshift field tools (`+6` to $\Delta_{\text{tool}}$; triggers automatic 1-step wear/degradation on any Floor Breach $Y_n < 2$).
    ImprovisedSevere,
    /// Bare-handed / no tools (`+8` to $\Delta_{\text{tool}}$ for `[Tool: Situational]`; `0` for `[Tool: None]`; `[Locked]` for `[Tool: Mandatory]`).
    NoTools,
    /// Bare-handed in extreme conditions (`+12` to $\Delta_{\text{tool}}$ for `[Tool: Situational]`; `0` for `[Tool: None]`; `[Locked]` for `[Tool: Mandatory]`).
    NoToolsSevere,
}

impl ToolQualityTier {
    pub fn from_key(key: &str) -> Self {
        match key.trim().to_ascii_lowercase().as_str() {
            "ai_assisted" | "ai_assisted_rig" | "ai" => Self::AiAssistedRig,
            "masterwork" | "precision" => Self::Masterwork,
            "standard" | "standard_kit" | "" => Self::StandardKit,
            "worn" | "worn_substandard" | "substandard" => Self::WornSubstandard,
            "improvised" => Self::Improvised,
            "improvised_severe" => Self::ImprovisedSevere,
            "no_tools" | "bare_handed" | "none" => Self::NoTools,
            "no_tools_severe" => Self::NoToolsSevere,
            _ => Self::StandardKit,
        }
    }
}

/// Resolved eligibility, effective pair pool $N$, known $\Delta_{\text{tool}}$, and hidden $\Delta_{\text{ext}}$ gate penalty.
///
/// Origin: `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§2.1–§2.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskGateResolution {
    pub allowed: bool,
    pub effective_skill_n: usize,
    /// Additional external difficulty penalty ($\Delta_{\text{ext}}$) from untrained or sibling defaulting.
    pub gate_delta_ext: i32,
    /// Known player tool modifier ($\Delta_{\text{tool}}$) shifting $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$.
    pub tool_delta_mod: i32,
    pub improvised_wear_on_floor_breach: bool,
    pub used_defaulting: bool,
    pub lock_reason: Option<String>,
}

/// Evaluates Step 1 of the Satya Task Resolution Procedure (`docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`
/// §2.1 Task Gate Classifications & §2.2 Tool Dependency Tiers).
///
/// - `gate_class`: `ClassA`, `ClassB`, or `ClassC { min_skill_n }`.
/// - `trained_skill_n`: Character's trained skill level $N$ in the required skill (`0..=10`).
/// - `talent_or_parent_n`: For `ClassA` when `trained_skill_n == 0`, the character's Talent rating $N_{\text{talent}} \in [0, 10]$;
///   for `ClassB` when `trained_skill_n == 0`, the character's related parent/sibling skill level $N_{\text{parent}}$.
/// - `default_penalty_ext`: External difficulty penalty when defaulting in `ClassB` (clamped to `4..=6`, canonical `+4`).
/// - `tool_dependency`: `[Tool: None]`, `[Tool: Situational]`, or `[Tool: Mandatory]`.
/// - `tool_tier`: Equipment quality tier (`AiAssistedRig` through `NoToolsSevere`).
pub fn resolve_task_gate_and_tools(
    gate_class: TaskGateClass,
    trained_skill_n: usize,
    talent_or_parent_n: usize,
    default_penalty_ext: i32,
    tool_dependency: ToolDependency,
    tool_tier: ToolQualityTier,
) -> TaskGateResolution {
    // 1. Check Tool Dependency lock first
    let is_no_tools = matches!(
        tool_tier,
        ToolQualityTier::NoTools | ToolQualityTier::NoToolsSevere
    );
    if tool_dependency == ToolDependency::Mandatory && is_no_tools {
        return TaskGateResolution {
            allowed: false,
            effective_skill_n: 0,
            gate_delta_ext: 0,
            tool_delta_mod: 0,
            improvised_wear_on_floor_breach: false,
            used_defaulting: false,
            lock_reason: Some(
                "[Locked] Task has [Tool: Mandatory] and cannot be attempted without tools."
                    .to_string(),
            ),
        };
    }

    let tool_delta_mod = match (tool_dependency, tool_tier) {
        (_, ToolQualityTier::AiAssistedRig) => -4,
        (_, ToolQualityTier::Masterwork) => -2,
        (_, ToolQualityTier::StandardKit) => 0,
        (_, ToolQualityTier::WornSubstandard) => 2,
        (_, ToolQualityTier::Improvised) => 4,
        (_, ToolQualityTier::ImprovisedSevere) => 6,
        (ToolDependency::None, ToolQualityTier::NoTools | ToolQualityTier::NoToolsSevere) => 0,
        (ToolDependency::Situational, ToolQualityTier::NoTools) => 8,
        (ToolDependency::Situational, ToolQualityTier::NoToolsSevere) => 12,
        (ToolDependency::Mandatory, ToolQualityTier::NoTools | ToolQualityTier::NoToolsSevere) => {
            12
        }
    };
    let improvised_wear_on_floor_breach = matches!(
        tool_tier,
        ToolQualityTier::Improvised | ToolQualityTier::ImprovisedSevere
    );

    // 2. Evaluate Task Gate Class (Class A, B, C)
    match gate_class {
        TaskGateClass::ClassA => {
            if trained_skill_n >= 1 {
                TaskGateResolution {
                    allowed: true,
                    effective_skill_n: trained_skill_n,
                    gate_delta_ext: 0,
                    tool_delta_mod,
                    improvised_wear_on_floor_breach,
                    used_defaulting: false,
                    lock_reason: None,
                }
            } else if talent_or_parent_n >= 1 {
                // Talent rating N_talent in [1, 10] with Delta_ext = 0
                TaskGateResolution {
                    allowed: true,
                    effective_skill_n: talent_or_parent_n.min(10),
                    gate_delta_ext: 0,
                    tool_delta_mod,
                    improvised_wear_on_floor_breach,
                    used_defaulting: true,
                    lock_reason: None,
                }
            } else {
                // Untrained Class A default: N = 1 pair with Delta_ext = +4
                TaskGateResolution {
                    allowed: true,
                    effective_skill_n: 1,
                    gate_delta_ext: 4,
                    tool_delta_mod,
                    improvised_wear_on_floor_breach,
                    used_defaulting: true,
                    lock_reason: None,
                }
            }
        }
        TaskGateClass::ClassB => {
            if trained_skill_n >= 1 {
                TaskGateResolution {
                    allowed: true,
                    effective_skill_n: trained_skill_n,
                    gate_delta_ext: 0,
                    tool_delta_mod,
                    improvised_wear_on_floor_breach,
                    used_defaulting: false,
                    lock_reason: None,
                }
            } else if talent_or_parent_n >= 2 {
                // Default from related parent/sibling skill at N_parent >= 2:
                // N = max(1, N_parent - 1) pairs with Delta_ext in [+4, +6]
                let eff_n = (talent_or_parent_n - 1).max(1);
                let pen = default_penalty_ext.clamp(4, 6);
                TaskGateResolution {
                    allowed: true,
                    effective_skill_n: eff_n,
                    gate_delta_ext: pen,
                    tool_delta_mod,
                    improvised_wear_on_floor_breach,
                    used_defaulting: true,
                    lock_reason: None,
                }
            } else {
                TaskGateResolution {
                    allowed: false,
                    effective_skill_n: 0,
                    gate_delta_ext: 0,
                    tool_delta_mod,
                    improvised_wear_on_floor_breach,
                    used_defaulting: false,
                    lock_reason: Some(
                        "[Locked] Class B task requires trained skill N >= 1 or related parent/sibling skill N_parent >= 2."
                            .to_string(),
                    ),
                }
            }
        }
        TaskGateClass::ClassC { min_skill_n } => {
            let req = min_skill_n.max(1);
            if trained_skill_n >= req {
                TaskGateResolution {
                    allowed: true,
                    effective_skill_n: trained_skill_n,
                    gate_delta_ext: 0,
                    tool_delta_mod,
                    improvised_wear_on_floor_breach,
                    used_defaulting: false,
                    lock_reason: None,
                }
            } else {
                TaskGateResolution {
                    allowed: false,
                    effective_skill_n: 0,
                    gate_delta_ext: 0,
                    tool_delta_mod,
                    improvised_wear_on_floor_breach,
                    used_defaulting: false,
                    lock_reason: Some(format!(
                        "[Locked] Class C specialist task requires trained skill N >= {req} (current N = {trained_skill_n}); no defaulting permitted."
                    )),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 12. Controlling & Supporting Characteristic Formulas (§3.1–§3.2)
// ---------------------------------------------------------------------------

/// Computes a Composite Controlling Characteristic $C = \lfloor (C_1 + C_2) / 2 \rfloor$
/// per `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1–§3.2).
pub const fn compute_composite_characteristic(c1: i32, c2: i32) -> i32 {
    (c1 + c2).div_euclid(2)
}

/// Computes the Gated Supporting Characteristic external difficulty penalty ($\Delta_{\text{ext}}$)
/// per `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1–§3.2):
/// - If a task specifies `Min C3 >= X` and the character has `supporting_c < min_required_c`,
///   each point of shortfall $k = X - C_3$ imposes a $+2$ external difficulty penalty ($+2k$ to $\Delta_{\text{ext}}$).
pub const fn compute_supporting_characteristic_shortfall_penalty(
    supporting_c: i32,
    min_required_c: i32,
) -> i32 {
    let shortfall = min_required_c - supporting_c;
    if shortfall > 0 { shortfall * 2 } else { 0 }
}

// ---------------------------------------------------------------------------
// 13. Complementary Skills (KNO -> PHY), Dual-Skill Gate & Extended Clocks (§6.1–§6.3)
// ---------------------------------------------------------------------------

/// Mechanical synergy transferred from a preliminary analytical (`KNO` / `LOG`) setup roll
/// to a subsequent physical or technical execution roll per
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplementarySynergyResult {
    /// External difficulty modifier ($\Delta_{\text{ext}}$) applied to the execution task (`-4`, `-2`, `0`, or `+4`).
    pub delta_ext_mod: i32,
    /// Over-target step carry bonus $\delta_{\text{over}}$ for the execution task (`8` on `ExtraordinarySuccess`, `6` otherwise).
    pub delta_over_carry: i32,
    pub summary: &'static str,
}

/// Evaluates a preliminary `KNO` / `LOG` complementary setup roll per
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§6.1):
/// - `3+` successes (`ExtraordinarySuccess` and `setup_signed_margin >= 3`): `-4` to $\Delta_{\text{ext}}$ AND $\delta_{\text{over}}$ boosted from `+6` to `+8`.
/// - `1–2` successes (`setup_total_successes >= 1`): `-2` to $\Delta_{\text{ext}}$ and standard $\delta_{\text{over}} = 6$.
/// - `0` successes (`CleanFailure`, `setup_signed_margin == 0`): `0` modifier and $\delta_{\text{over}} = 6$.
/// - `< 0` (`BotchFailure`, `setup_signed_margin < 0`): `+4` penalty to $\Delta_{\text{ext}}$ (flawed theory / misread schematics).
pub const fn evaluate_complementary_setup_synergy(
    setup_total_successes: i32,
    setup_signed_margin: i32,
) -> ComplementarySynergyResult {
    if setup_total_successes >= 3 && setup_signed_margin >= 3 {
        ComplementarySynergyResult {
            delta_ext_mod: -4,
            delta_over_carry: 8,
            summary: "Extraordinary Setup (3+ succ): Delta_ext -4, delta_over boosted to +8",
        }
    } else if setup_total_successes >= 1 {
        ComplementarySynergyResult {
            delta_ext_mod: -2,
            delta_over_carry: 6,
            summary: "Solid Setup (1-2 succ): Delta_ext -2, delta_over +6",
        }
    } else if setup_signed_margin < 0 {
        ComplementarySynergyResult {
            delta_ext_mod: 4,
            delta_over_carry: 6,
            summary: "Botched Setup (<0): Flawed theory imposes Delta_ext +4 penalty",
        }
    } else {
        ComplementarySynergyResult {
            delta_ext_mod: 0,
            delta_over_carry: 6,
            summary: "Clean Miss Setup (0): No synergy modifier",
        }
    }
}

impl CascadeTaskParams {
    /// Returns a copy of `self` with a [`ComplementarySynergyResult`] applied (`external_mod += synergy.delta_ext_mod`,
    /// `delta_over = synergy.delta_over_carry`) per `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§6.1).
    pub const fn with_complementary_synergy(mut self, synergy: ComplementarySynergyResult) -> Self {
        self.external_mod += synergy.delta_ext_mod;
        self.delta_over = synergy.delta_over_carry;
        self
    }
}

impl ContestedCascadeTaskParams {
    /// Returns a copy of `self` with a [`ComplementarySynergyResult`] applied to the contested check per
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§6.1):
    /// - Adjusts the GM's hidden resistance difficulty by `synergy.delta_ext_mod` ($\Delta_{\text{ext}} \mathrel{+}= \text{delta\_ext\_mod}$,
    ///   shifting $C_{GM}$ by `synergy.delta_ext_mod` so $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$ updates accordingly).
    /// - Sets the player's over-target step carry $\delta_{\text{over}}$ to `synergy.delta_over_carry` (`+8` on Extraordinary setup).
    pub const fn with_complementary_synergy(mut self, synergy: ComplementarySynergyResult) -> Self {
        self.player.delta_over = synergy.delta_over_carry;
        self.gm.control_c += synergy.delta_ext_mod;
        self
    }
}

/// Computes the effective cascade pool size $N_{\text{eff}}$ for a Simultaneous Dual-Skill Gate
/// per `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§6.2):
/// $$N_{\text{eff}} = \min(N_1, N_2) + \mathbb{I}\big(\max(N_1, N_2) \ge \min(N_1, N_2) + 2\big)$$
pub const fn compute_dual_skill_pool(skill_n1: usize, skill_n2: usize) -> usize {
    let min_n = if skill_n1 < skill_n2 {
        skill_n1
    } else {
        skill_n2
    };
    let max_n = if skill_n1 > skill_n2 {
        skill_n1
    } else {
        skill_n2
    };
    let bonus = if max_n >= min_n + 2 { 1 } else { 0 };
    min_n + bonus
}

/// Result of applying a single interval roll to an [`ExtendedProjectClock`].
///
/// Origin: `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§6.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedProjectStepResult {
    pub progress_delta: i32,
    /// Updated cumulative net successes on the project clock (`>= 0`).
    pub accumulated_successes: i32,
    pub target_net_successes: i32,
    pub intervals_rolled: u32,
    /// Cumulative Heat / detection risk generated by floor-breach complications.
    pub heat_generated: i32,
    /// Cumulative extra material consumption percentage (`+25%` per floor-breach complication).
    pub extra_material_pct: i32,
    pub is_completed: bool,
}

/// Extended Project Progress Clock per `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§6.3).
///
/// Tracks multi-interval engineering, research, surgical, decryption, or post-lifepath training projects:
/// - **Standard Project**: `10` net successes (`target_net_successes = 10`)
/// - **Complex Project**: `20` net successes (`target_net_successes = 20`)
/// - **Major Breakthrough / Overhaul**: `40` net successes (`target_net_successes = 40`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedProjectClock {
    pub project_name: String,
    pub target_net_successes: i32,
    /// Current accumulated net successes (`>= 0`).
    pub accumulated_successes: i32,
    pub intervals_rolled: u32,
    /// Cumulative Heat / alert level generated by floor-breach complications.
    pub heat_generated: i32,
    /// Cumulative extra material cost in percentage points (`+25` per floor-breach complication).
    pub extra_material_pct: i32,
}

impl ExtendedProjectClock {
    pub fn new(project_name: impl Into<String>, target_net_successes: i32) -> Self {
        Self {
            project_name: project_name.into(),
            target_net_successes: target_net_successes.max(1),
            accumulated_successes: 0,
            intervals_rolled: 0,
            heat_generated: 0,
            extra_material_pct: 0,
        }
    }

    pub const fn is_completed(&self) -> bool {
        self.accumulated_successes >= self.target_net_successes
    }

    /// Applies an interval result given raw player successes, signed margin, floor-breach flag, and botch flag:
    /// - Positive successes add to `accumulated_successes`.
    /// - `SuccessWithComplication` / `SuccessWithFloorBreach` advances the clock but adds `+1` Heat and `+25%` extra materials.
    /// - `BotchFailure` (`is_botch == true` or `signed_margin < 0`) reduces `accumulated_successes` by `|signed_margin|` (minimum `0`).
    pub fn apply_interval(
        &mut self,
        positive_successes: i32,
        signed_margin: i32,
        had_floor_breach_complication: bool,
        is_botch: bool,
    ) -> ExtendedProjectStepResult {
        self.intervals_rolled += 1;
        let before = self.accumulated_successes;
        if is_botch || signed_margin < 0 {
            let setback = signed_margin.abs().max(1);
            self.accumulated_successes = (self.accumulated_successes - setback).max(0);
        } else if positive_successes > 0 {
            self.accumulated_successes += positive_successes;
            if had_floor_breach_complication {
                self.heat_generated += 1;
                self.extra_material_pct += 25;
            }
        }
        let delta = self.accumulated_successes - before;
        ExtendedProjectStepResult {
            progress_delta: delta,
            accumulated_successes: self.accumulated_successes,
            target_net_successes: self.target_net_successes,
            intervals_rolled: self.intervals_rolled,
            heat_generated: self.heat_generated,
            extra_material_pct: self.extra_material_pct,
            is_completed: self.is_completed(),
        }
    }

    pub fn apply_uncontested_roll(
        &mut self,
        outcome: &CascadeRollOutcome,
    ) -> ExtendedProjectStepResult {
        let is_botch = outcome.outcome_kind == CascadeOutcomeKind::BotchFailure;
        let had_complication = outcome.outcome_kind == CascadeOutcomeKind::SuccessWithComplication;
        self.apply_interval(
            outcome.total_successes,
            outcome.signed_margin,
            had_complication,
            is_botch,
        )
    }

    /// Applies a [`ContestedCascadeRollOutcome`] to the project clock per
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§5.1 & §6.3).
    pub fn apply_contested_roll(
        &mut self,
        outcome: &ContestedCascadeRollOutcome,
    ) -> ExtendedProjectStepResult {
        let is_botch = outcome.contested_kind == ContestedOutcomeKind::BotchFailure;
        let had_complication =
            outcome.contested_kind == ContestedOutcomeKind::SuccessWithFloorBreach;
        let progress = match outcome.contested_kind {
            ContestedOutcomeKind::ExtraordinarySuccess
            | ContestedOutcomeKind::CleanSuccess
            | ContestedOutcomeKind::SuccessWithFloorBreach => outcome.net_successes.max(1),
            ContestedOutcomeKind::GrazingTieSuccess | ContestedOutcomeKind::LuckyBreakSuccess => 1,
            ContestedOutcomeKind::CleanFailure | ContestedOutcomeKind::BotchFailure => 0,
        };
        let setback_margin = if is_botch {
            let eff_setback = if outcome.gm_outcome.signed_margin < 0 {
                (outcome.player_outcome.effective_botches - outcome.gm_outcome.effective_botches)
                    .max(1)
            } else {
                outcome.player_outcome.effective_botches.max(1)
            };
            -eff_setback
        } else {
            progress
        };
        self.apply_interval(progress, setback_margin, had_complication, is_botch)
    }
}

// ---------------------------------------------------------------------------
// 14. Canonical Worked Examples (§7.1–§7.3) & Master Task Requirements Catalog (§8)
// ---------------------------------------------------------------------------

/// Canonical worked example preset from `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§7.1–§7.3)
/// and the Satya combat/social chapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalTaskPreset {
    pub id: &'static str,
    pub title: &'static str,
    pub skill_name: &'static str,
    pub player_skill_n: usize,
    pub player_control_c: i32,
    /// Internal state modifier $\Delta_{\text{int}}$ (`+` = stress/fatigue penalty, `-` = focus/calm bonus).
    pub player_internal_mod: i32,
    /// Known player tool modifier $\Delta_{\text{tool}}$ shifting $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$.
    pub player_known_tool_mod: i32,
    /// Hidden external difficulty $\Delta_{\text{ext}}$ shifting $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$.
    pub hidden_external_difficulty: i32,
    pub delta_over: i32,
}

impl CanonicalTaskPreset {
    /// Converts this preset into its [`ContestedCascadeTaskParams`] (using `1-Pair Tiered GM Resistance` and `HybridSignedMargin`).
    pub const fn to_contested_params(&self) -> ContestedCascadeTaskParams {
        let mut params = ContestedCascadeTaskParams::new_tiered_environment(
            self.player_skill_n,
            self.player_control_c,
            self.player_internal_mod,
            self.player_known_tool_mod,
            self.hidden_external_difficulty,
            ContestedTieRule::HybridSignedMargin,
        );
        params.player.delta_over = self.delta_over;
        params
    }

    /// Converts this preset into an uncontested [`CascadeTaskParams`] where `external_mod` combines
    /// `player_known_tool_mod + hidden_external_difficulty`.
    pub const fn to_uncontested_params(&self) -> CascadeTaskParams {
        let mut params = CascadeTaskParams::new_k20(
            self.player_skill_n,
            self.player_control_c,
            self.player_internal_mod,
            self.player_known_tool_mod + self.hidden_external_difficulty,
        );
        params.delta_over = self.delta_over;
        params
    }
}

macro_rules! task_preset {
    ($id:literal, $title:literal, $skill:literal, ($n:expr, $c:expr, $int:expr, $tool:expr, $ext:expr, $over:expr)) => {
        CanonicalTaskPreset {
            id: $id,
            title: $title,
            skill_name: $skill,
            player_skill_n: $n,
            player_control_c: $c,
            player_internal_mod: $int,
            player_known_tool_mod: $tool,
            hidden_external_difficulty: $ext,
            delta_over: $over,
        }
    };
}

/// Canonical worked examples from `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§7.1–§7.3) and the core rules book.
pub static CANONICAL_TASK_PRESETS: &[CanonicalTaskPreset] = &[
    task_preset!("sarah_lockpick", "Sarah's Lockpick (N=3, C=12, Δ_tool=0, Δ_int=+2, Δ_ext=+4)", "Mechanical Lockpicking (Sarah)", (3, 12, 2, 0, 4, 6)),
    task_preset!("marcus_surgery", "Marcus's Trauma Surgery (N=4, C=14, Δ_tool=-2, Δ_int=-1, Δ_ext=+4)", "Trauma Surgery (Marcus)", (4, 14, -1, -2, 4, 6)),
    task_preset!("elena_hack", "Elena's Intrusion Hack (N=3, C=15, Δ_tool=-2, Δ_int=0, Δ_ext=+2, δ_over=8)", "Intrusion & Exploitation (Elena)", (3, 15, 0, -2, 2, 8)),
    task_preset!("elena_probe", "Elena's Sword Probe (N=5, C=16, Δ_tool=0, Δ_int=0, Δ_ext=+2)", "Arming Sword Probe (Elena)", (5, 16, 0, 0, 2, 6)),
    task_preset!("viktor_cleave", "Viktor's Greatsword Cleave (N=4, C=13, Δ_tool=0, Δ_int=0, Δ_ext=+3)", "Greatsword Cleave (Viktor)", (4, 13, 0, 0, 3, 6)),
    task_preset!("social_persuasion", "Counter-Interrogation Rhetoric (N=3, C=15, Δ_tool=0, Δ_int=+2, Δ_ext=+3)", "Counter-Interrogation Rhetoric", (3, 15, 2, 0, 3, 6)),
];

pub fn list_canonical_task_presets() -> &'static [CanonicalTaskPreset] {
    CANONICAL_TASK_PRESETS
}

pub fn get_canonical_task_preset(id: &str) -> &'static CanonicalTaskPreset {
    let key = id.trim();
    CANONICAL_TASK_PRESETS
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(key))
        .unwrap_or(&CANONICAL_TASK_PRESETS[0])
}

/// Structured entry from the §8 Master Task Requirements Table in
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterTaskBenchmark {
    pub id: &'static str,
    pub domain: &'static str,
    pub task_name: &'static str,
    pub required_skill: &'static str,
    pub gate_class: TaskGateClass,
    pub primary_char: CharacteristicCode,
    pub secondary_char: Option<CharacteristicCode>,
    pub controlling_characteristic: &'static str,
    pub min_supporting_characteristic: Option<(CharacteristicCode, i32)>,
    pub tool_dependency: ToolDependency,
    /// Base hidden external difficulty $\Delta_{\text{ext}}$.
    pub base_delta_ext: i32,
    pub tick_cost: &'static str,
}

impl MasterTaskBenchmark {
    /// Compute the composite Controlling Characteristic $C = \lfloor(C_1 + C_2)/2\rfloor$ for `chars`
    /// (`docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` §3.1).
    #[must_use]
    pub fn compute_controller(&self, chars: &CharacterCharacteristics) -> i32 {
        chars.compute_controller(self.primary_char, self.secondary_char)
    }

    /// Compute the Supporting Characteristic ($C_3$) shortfall penalty $\Delta_{\text{int}} = \max(0, C_{3,\min} - C_3)$
    /// (`docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` §3.2).
    #[must_use]
    pub fn supporting_shortfall_penalty(&self, chars: &CharacterCharacteristics) -> i32 {
        match self.min_supporting_characteristic {
            Some((c3, min_req)) => {
                compute_supporting_characteristic_shortfall_penalty(chars[c3], min_req)
            }
            None => 0,
        }
    }
}

macro_rules! task_benchmark {
    ($id:literal, $domain:literal, $name:literal, $skill:literal, $gate:expr, ($pchar:ident, $schar:ident), $supp:expr, $tool:ident, $ext:expr, $tc:literal) => {
        MasterTaskBenchmark {
            id: $id,
            domain: $domain,
            task_name: $name,
            required_skill: $skill,
            gate_class: $gate,
            primary_char: CharacteristicCode::$pchar,
            secondary_char: Some(CharacteristicCode::$schar),
            controlling_characteristic: concat!(stringify!($pchar), " + ", stringify!($schar)),
            min_supporting_characteristic: $supp,
            tool_dependency: ToolDependency::$tool,
            base_delta_ext: $ext,
            tick_cost: $tc,
        }
    };
}

/// Complete 22-entry Master Task Requirements catalog from `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§8).
pub static MASTER_TASK_BENCHMARKS: &[MasterTaskBenchmark] = &[
    task_benchmark!("sprint_vault_gap", "Athletic", "Sprint & Vault Gap", "Acrobatics, Tumbling & Parkour", TaskGateClass::ClassA, (KIN, SPD), None, None, 0, "6 TC"),
    task_benchmark!("free_climb_sheer_wet_rock", "Athletic", "Free-Climb Sheer Wet Rock", "Climbing & Scaling", TaskGateClass::ClassA, (FOR, KIN), Some((CharacteristicCode::FIT, 11)), Situational, 4, "15 TC / pitch"),
    task_benchmark!("force_rusted_bulkhead_door", "Athletic", "Force Rusted Bulkhead Door", "Strength & Lifting", TaskGateClass::ClassA, (FOR, FIT), Some((CharacteristicCode::HLT, 12)), Situational, 2, "8 TC"),
    task_benchmark!("zero_g_eva_thruster_stabilization", "Athletic", "Zero-G EVA Thruster Stabilization", "Zero-G / EVA Operations", TaskGateClass::ClassB, (KIN, FOC), Some((CharacteristicCode::KIN, 10)), Mandatory, 2, "5 TC"),
    task_benchmark!("silent_infiltration_past_sentries", "Fieldcraft", "Silent Infiltration Past Sentries", "Stealth (Sneaking & Hiding)", TaskGateClass::ClassA, (KIN, ITU), Some((CharacteristicCode::DIS, 10)), None, 2, "10 TC"),
    task_benchmark!("pick_high_security_pin_lock", "Fieldcraft", "Pick High-Security Pin Lock", "Lockpicking & Physical Intrusion", TaskGateClass::ClassB, (DEX, TAC), Some((CharacteristicCode::FOC, 9)), Mandatory, 4, "15 TC"),
    task_benchmark!("bypass_biometric_capacitance_pad", "Cyber", "Bypass Biometric Capacitance Pad", "Lockpicking & Physical Intrusion", TaskGateClass::ClassC { min_skill_n: 2 }, (ANA, DEX), Some((CharacteristicCode::MEM, 12)), Mandatory, 4, "20 TC"),
    task_benchmark!("disarm_shaped_charge_ied", "Technical", "Disarm Shaped-Charge IED", "Demolitions & Explosive Ordnance (EOD)", TaskGateClass::ClassC { min_skill_n: 2 }, (DEX, FOC), Some((CharacteristicCode::DIS, 13)), Mandatory, 4, "30 TC"),
    task_benchmark!("staunch_arterial_bleed_in_combat", "Medical", "Staunch Arterial Bleed in Combat", "First Aid & Basic Life Support (BLS / CPR)", TaskGateClass::ClassA, (DEX, FOC), Some((CharacteristicCode::DIS, 11)), Situational, 2, "12 TC"),
    task_benchmark!("field_extraction_of_fragment_near_spine", "Medical", "Field Extraction of Fragment Near Spine", "Trauma, Orthopedic & General Surgery", TaskGateClass::ClassC { min_skill_n: 2 }, (DEX, FOC), Some((CharacteristicCode::DIS, 12)), Mandatory, 4, "15 min"),
    task_benchmark!("synthesize_antitoxin_from_tissue_sample", "Medical", "Synthesize Antitoxin from Tissue Sample", "Pharmacology & Toxicology", TaskGateClass::ClassC { min_skill_n: 2 }, (ANA, MEM), None, Mandatory, 4, "2 hrs / interval"),
    task_benchmark!("patch_ruptured_hydraulic_line", "Technical", "Patch Ruptured Hydraulic Line", "Automotive & Small Engine Mechanics", TaskGateClass::ClassB, (DEX, ANA), Some((CharacteristicCode::FOR, 10)), Situational, 0, "5 min"),
    task_benchmark!("reroute_damaged_micro_fission_coolant_loop", "Technical", "Reroute Damaged Micro-Fission Coolant Loop", "Thermodynamics & Fluid Dynamics", TaskGateClass::ClassC { min_skill_n: 3 }, (ANA, DEX), Some((CharacteristicCode::DIS, 13)), Mandatory, 6, "10 min / interval"),
    task_benchmark!("fabricate_custom_suppressor_baffle", "Technical", "Fabricate Custom Suppressor Baffle", "Armourer & Gunsmithing", TaskGateClass::ClassB, (DEX, ANA), None, Mandatory, 2, "1 hr / interval"),
    task_benchmark!("packet_sniff_local_unencrypted_subnet", "Cyber", "Packet-Sniff Local Unencrypted Subnet", "Cybersecurity & Network Intrusion", TaskGateClass::ClassB, (ANA, INS), None, Mandatory, 0, "10 TC"),
    task_benchmark!("live_kernel_injection_on_air_gapped_node", "Cyber", "Live Kernel Injection on Air-Gapped Node", "Cybersecurity & Network Intrusion + Cryptography", TaskGateClass::ClassC { min_skill_n: 2 }, (ANA, CRE), Some((CharacteristicCode::MEM, 13)), Mandatory, 4, "15 TC"),
    task_benchmark!("reconstruct_ballistic_trajectory_from_ricochets", "Science", "Reconstruct Ballistic Trajectory from Ricochets", "Forensics & Crime Scene Investigation", TaskGateClass::ClassB, (ANA, SIG), Some((CharacteristicCode::MEM, 11)), Situational, 2, "15 min"),
    task_benchmark!("spot_ambush_in_dense_urban_ruins", "Fieldcraft", "Spot Ambush in Dense Urban Ruins", "Alertness (Visual & Auditory Situational Awareness)", TaskGateClass::ClassA, (SIG, HEA), None, None, 2, "6 TC"),
    task_benchmark!("fast_talk_checkpoint_sentry", "Social", "Fast-Talk Checkpoint Sentry", "Deception & Bluff", TaskGateClass::ClassA, (ELO, CRE), Some((CharacteristicCode::DIS, 10)), None, 2, "10 TC"),
    task_benchmark!("break_hostile_subject_in_interrogation", "Social", "Break Hostile Subject in Interrogation", "Interrogation & Counter-Interrogation", TaskGateClass::ClassB, (INS, PRE), Some((CharacteristicCode::DIS, 12)), None, 4, "30 min"),
    task_benchmark!("rally_panicked_squad_under_artillery_fire", "Social", "Rally Panicked Squad Under Artillery Fire", "Leadership & Command", TaskGateClass::ClassB, (PRE, DIS), None, None, 4, "3 TC"),
    task_benchmark!("navigate_magnetic_anomaly_storm_by_dead_reckoning", "Fieldcraft", "Navigate Magnetic Anomaly Storm by Dead Reckoning", "Land Navigation & Cartography", TaskGateClass::ClassB, (ANA, SIG), Some((CharacteristicCode::DIS, 11)), Situational, 4, "1 hr"),
];

pub fn list_master_task_benchmarks() -> &'static [MasterTaskBenchmark] {
    MASTER_TASK_BENCHMARKS
}

pub fn get_master_task_benchmark(id_or_name: &str) -> Option<&'static MasterTaskBenchmark> {
    let q = id_or_name.trim();
    if let Some(found) = MASTER_TASK_BENCHMARKS
        .iter()
        .find(|b| b.id.eq_ignore_ascii_case(q) || b.task_name.eq_ignore_ascii_case(q))
    {
        return Some(found);
    }
    let canonical_id = match q.to_ascii_lowercase().as_str() {
        "sprint_vault" | "vault_gap" => "sprint_vault_gap",
        "free_climb" | "climb_sheer_wet_rock" => "free_climb_sheer_wet_rock",
        "force_door" | "force_bulkhead_door" => "force_rusted_bulkhead_door",
        "zero_g_eva" | "eva_thruster_stabilization" => "zero_g_eva_thruster_stabilization",
        "silent_infiltration" | "stealth_sentries" => "silent_infiltration_past_sentries",
        "pick_lock" | "lockpick" | "sarah_lockpick" => "pick_high_security_pin_lock",
        "bypass_biometric" | "biometric_pad" => "bypass_biometric_capacitance_pad",
        "disarm_ied" | "eod_disarm" => "disarm_shaped_charge_ied",
        "staunch_bleed" | "first_aid_arterial" => "staunch_arterial_bleed_in_combat",
        "field_surgery" | "marcus_surgery" | "trauma_surgery" => {
            "field_extraction_of_fragment_near_spine"
        }
        "synthesize_antitoxin" => "synthesize_antitoxin_from_tissue_sample",
        "patch_hydraulic" | "hydraulic_line" => "patch_ruptured_hydraulic_line",
        "reroute_coolant" | "micro_fission_coolant" => "reroute_damaged_micro_fission_coolant_loop",
        "fabricate_suppressor" | "suppressor_baffle" => "fabricate_custom_suppressor_baffle",
        "packet_sniff" | "sniff_subnet" => "packet_sniff_local_unencrypted_subnet",
        "kernel_injection" | "elena_hack" | "air_gapped_injection" => {
            "live_kernel_injection_on_air_gapped_node"
        }
        "ballistic_trajectory" | "forensics_ricochet" => {
            "reconstruct_ballistic_trajectory_from_ricochets"
        }
        "spot_ambush" | "urban_ambush" => "spot_ambush_in_dense_urban_ruins",
        "fast_talk" | "checkpoint_sentry" => "fast_talk_checkpoint_sentry",
        "interrogation" | "break_hostile_subject" => "break_hostile_subject_in_interrogation",
        "rally_squad" | "rally_panicked_squad" => "rally_panicked_squad_under_artillery_fire",
        "dead_reckoning" | "navigate_magnetic_storm" => {
            "navigate_magnetic_anomaly_storm_by_dead_reckoning"
        }
        _ => return None,
    };
    MASTER_TASK_BENCHMARKS
        .iter()
        .find(|b| b.id == canonical_id)
}
