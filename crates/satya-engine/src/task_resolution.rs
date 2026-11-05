//! # Task Resolution, Gate Classes, Tool Tiers, Synergy & Project Clocks (`02-core-mechanics`)
//!
//! Canonical Rust implementation of Sections 2, 3, 6, 7, and 8 of
//! `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`:
//! - **Task Gate Classifications (`§2.1`)**: [`TaskGateClass`] (`ClassA`, `ClassB`, `ClassC { min_skill_n }`).
//! - **Tool Dependency & Equipment Quality Tiers (`§2.2`)**: [`ToolDependency`], [`ToolQualityTier`], and [`resolve_task_gate_and_tools`].
//! - **Composite & Supporting Characteristics (`§3.2`)**: [`compute_composite_characteristic`] and [`compute_supporting_characteristic_shortfall_penalty`].
//! - **Complementary Skills (`KNO -> PHY`, `§6.1`), Dual-Skill Gates (`§6.2`) & Extended Project Clocks (`§6.3`)**:
//!   [`ComplementarySynergyResult`], [`evaluate_complementary_setup_synergy`], [`compute_dual_skill_pool`], and [`ExtendedProjectClock`].
//! - **Canonical Worked Examples (`§7`) & 22-Entry Master Task Requirements Catalog (`§8`)**:
//!   [`CanonicalTaskPreset`], [`CANONICAL_TASK_PRESETS`], [`MasterTaskBenchmark`], and [`MASTER_TASK_BENCHMARKS`].

use serde::{Deserialize, Serialize};

use crate::dice_probabilities::{
    CascadeOutcomeKind, CascadeRollOutcome, CascadeTaskParams, ContestedCascadeRollOutcome,
    ContestedCascadeTaskParams, ContestedOutcomeKind, ContestedTieRule,
};

// ---------------------------------------------------------------------------
// 11. Task Gate Classifications (Class A / B / C) & Tool Dependency Tiers
// ---------------------------------------------------------------------------

/// Task Gate Classification per §2.1 of `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskGateClass {
    /// **Class A — Open / Talent Tasks (`N >= 0`)**:
    /// Actions any conscious character can attempt without formal training.
    /// - If `trained_skill_n == 0`, the character may either roll their raw Talent rating
    ///   $N_{\text{talent}} \in [1, 3]$ with $\Delta_{\text{ext}} = 0$, or default to $N = 1$ pair
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
    /// (`1` to `3`) AND appropriate tools if `[Tool: Mandatory]`. No Talent or sibling defaulting is permitted.
    ClassC {
        /// Minimum trained skill level $N_{\text{min}}$ required (`1`, `2`, or `3`).
        min_skill_n: usize,
    },
}

impl TaskGateClass {
    /// Canonical label (`"Class A"`, `"Class B"`, or `"Class C"`).
    pub const fn label(self) -> &'static str {
        match self {
            Self::ClassA => "Class A",
            Self::ClassB => "Class B",
            Self::ClassC { .. } => "Class C",
        }
    }
}

/// Tool Dependency classification per §2.2 of `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`.
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
    /// Canonical tag (`"[Tool: None]"`, `"[Tool: Situational]"`, `"[Tool: Mandatory]"`).
    pub const fn tag(self) -> &'static str {
        match self {
            Self::None => "[Tool: None]",
            Self::Situational => "[Tool: Situational]",
            Self::Mandatory => "[Tool: Mandatory]",
        }
    }
}

/// Equipment quality tier and known player tool modifier ($\Delta_{\text{tool}}$) per §2.2 of `cascading_d20_and_tasks.md`.
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
    /// Parses a string identifier into a [`ToolQualityTier`].
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskGateResolution {
    /// Whether the character is permitted to roll the task (`false` means `[Locked]`).
    pub allowed: bool,
    /// Effective number of `2d20` cascade pairs $N$ to roll (`0` if `[Locked]`).
    pub effective_skill_n: usize,
    /// Additional external difficulty penalty ($\Delta_{\text{ext}}$) from untrained or sibling defaulting.
    pub gate_delta_ext: i32,
    /// Known player tool modifier ($\Delta_{\text{tool}}$) shifting $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$.
    pub tool_delta_mod: i32,
    /// Whether any Floor Breach ($Y_n < 2$) automatically inflicts 1 step of equipment wear/degradation (`Improvised` tools).
    pub improvised_wear_on_floor_breach: bool,
    /// Whether the character used Talent (`Class A`) or sibling-skill (`Class B`) defaulting.
    pub used_defaulting: bool,
    /// Human-readable reason if `allowed == false` (`[Locked]`).
    pub lock_reason: Option<String>,
}

/// Evaluates Step 1 of the Satya Task Resolution Procedure (§2.1 Task Gate Classifications & §2.2 Tool Dependency Tiers).
///
/// - `gate_class`: `ClassA`, `ClassB`, or `ClassC { min_skill_n }`.
/// - `trained_skill_n`: Character's trained skill level $N$ in the required skill (`0..=10`).
/// - `talent_or_parent_n`: For `ClassA` when `trained_skill_n == 0`, the character's raw Talent rating $N_{\text{talent}} \in [0, 3]$;
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
                // Raw Talent rating N_talent in [1, 3] with Delta_ext = 0
                TaskGateResolution {
                    allowed: true,
                    effective_skill_n: talent_or_parent_n.min(3),
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
// 12. Controlling & Supporting Characteristic Formulas (§3.2)
// ---------------------------------------------------------------------------

/// Computes a Composite Controlling Characteristic $C = \lfloor (C_1 + C_2) / 2 \rfloor$
/// per §3.2 of `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`.
pub const fn compute_composite_characteristic(c1: i32, c2: i32) -> i32 {
    (c1 + c2).div_euclid(2)
}

/// Computes the Gated Supporting Characteristic external difficulty penalty ($\Delta_{\text{ext}}$)
/// per §3.2 of `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`:
/// - If a task specifies `Min C3 >= X` and the character has `supporting_c < min_required_c`,
///   each point of shortfall $k = X - C_3$ imposes a $+2$ external difficulty penalty ($+2k$ to $\Delta_{\text{ext}}$).
pub const fn compute_supporting_characteristic_shortfall_penalty(
    supporting_c: i32,
    min_required_c: i32,
) -> i32 {
    let shortfall = min_required_c - supporting_c;
    if shortfall > 0 {
        shortfall * 2
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// 13. Complementary Skills (KNO -> PHY), Dual-Skill Gate & Extended Clocks (§6)
// ---------------------------------------------------------------------------

/// Mechanical synergy transferred from a preliminary analytical (`KNO` / `LOG`) setup roll
/// to a subsequent physical or technical execution roll per §6.1 of `cascading_d20_and_tasks.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplementarySynergyResult {
    /// External difficulty modifier ($\Delta_{\text{ext}}$) applied to the execution task (`-4`, `-2`, `0`, or `+4`).
    pub delta_ext_mod: i32,
    /// Over-target step carry bonus $\delta_{\text{over}}$ for the execution task (`8` on `ExtraordinarySuccess`, `6` otherwise).
    pub delta_over_carry: i32,
    /// Canonical description of the setup synergy tier.
    pub summary: &'static str,
}

/// Evaluates a preliminary `KNO` / `LOG` complementary setup roll per §6.1 of `cascading_d20_and_tasks.md`:
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
    /// `delta_over = synergy.delta_over_carry`).
    pub const fn with_complementary_synergy(mut self, synergy: ComplementarySynergyResult) -> Self {
        self.external_mod += synergy.delta_ext_mod;
        self.delta_over = synergy.delta_over_carry;
        self
    }
}

impl ContestedCascadeTaskParams {
    /// Returns a copy of `self` with a [`ComplementarySynergyResult`] applied to the contested check:
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
/// per §6.2 of `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`:
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedProjectStepResult {
    /// Net progress delta added (or subtracted on a botch) during this interval.
    pub progress_delta: i32,
    /// Updated cumulative net successes on the project clock (`>= 0`).
    pub accumulated_successes: i32,
    /// Target net successes required to complete the project (`10`, `20`, or `40`).
    pub target_net_successes: i32,
    /// Total work intervals rolled so far.
    pub intervals_rolled: u32,
    /// Cumulative Heat / detection risk generated by floor-breach complications.
    pub heat_generated: i32,
    /// Cumulative extra material consumption percentage (`+25%` per floor-breach complication).
    pub extra_material_pct: i32,
    /// Whether the project has reached or exceeded `target_net_successes`.
    pub is_completed: bool,
}

/// Extended Project Progress Clock per §6.3 of `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`.
///
/// Tracks multi-interval engineering, research, surgical, decryption, or post-lifepath training projects:
/// - **Standard Project**: `10` net successes (`target_net_successes = 10`)
/// - **Complex Project**: `20` net successes (`target_net_successes = 20`)
/// - **Major Breakthrough / Overhaul**: `40` net successes (`target_net_successes = 40`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedProjectClock {
    /// Name or description of the extended project.
    pub project_name: String,
    /// Required cumulative net successes to finish (`10`, `20`, `40`, etc.).
    pub target_net_successes: i32,
    /// Current accumulated net successes (`>= 0`).
    pub accumulated_successes: i32,
    /// Number of work intervals rolled so far.
    pub intervals_rolled: u32,
    /// Cumulative Heat / alert level generated by floor-breach complications.
    pub heat_generated: i32,
    /// Cumulative extra material cost in percentage points (`+25` per floor-breach complication).
    pub extra_material_pct: i32,
}

impl ExtendedProjectClock {
    /// Creates a new [`ExtendedProjectClock`] with `target_net_successes` (clamped to `>= 1`).
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

    /// Returns `true` if `accumulated_successes >= target_net_successes`.
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

    /// Applies an uncontested [`CascadeRollOutcome`] to the project clock.
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

    /// Applies a [`ContestedCascadeRollOutcome`] to the project clock.
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
            -outcome
                .player_outcome
                .effective_botches
                .max(outcome.net_signed_margin.abs())
                .max(1)
        } else {
            progress
        };
        self.apply_interval(progress, setback_margin, had_complication, is_botch)
    }
}

// ---------------------------------------------------------------------------
// 14. Canonical Worked Examples (§7) & Master Task Requirements Catalog (§8)
// ---------------------------------------------------------------------------

/// Canonical worked example preset from §7 of `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md`
/// and the Satya combat/social chapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalTaskPreset {
    /// Preset identifier (`"sarah_lockpick"`, `"marcus_surgery"`, `"elena_hack"`, `"elena_probe"`, `"viktor_cleave"`, `"social_persuasion"`).
    pub id: &'static str,
    /// Display title for UI buttons and logs.
    pub title: &'static str,
    /// Skill name populated into the task roller.
    pub skill_name: &'static str,
    /// Effective player skill pool size $N_P$.
    pub player_skill_n: usize,
    /// Player Controlling Characteristic $C_P$.
    pub player_control_c: i32,
    /// Internal state modifier $\Delta_{\text{int}}$ (`+` = stress/fatigue penalty, `-` = focus/calm bonus).
    pub player_internal_mod: i32,
    /// Known player tool modifier $\Delta_{\text{tool}}$ shifting $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$.
    pub player_known_tool_mod: i32,
    /// Hidden external difficulty $\Delta_{\text{ext}}$ shifting $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$.
    pub hidden_external_difficulty: i32,
    /// Over-target step carry $\delta_{\text{over}}$ (`6` standard, `8` when boosted by Extraordinary `KNO -> PHY` synergy).
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

/// Canonical worked examples from §7 of `cascading_d20_and_tasks.md` and the core rules book.
pub static CANONICAL_TASK_PRESETS: &[CanonicalTaskPreset] = &[
    CanonicalTaskPreset {
        id: "sarah_lockpick",
        title: "Sarah's Lockpick (N=3, C=12, Δ_tool=0, Δ_int=+2, Δ_ext=+4)",
        skill_name: "Mechanical Lockpicking (Sarah)",
        player_skill_n: 3,
        player_control_c: 12,
        player_internal_mod: 2,
        player_known_tool_mod: 0,
        hidden_external_difficulty: 4,
        delta_over: 6,
    },
    CanonicalTaskPreset {
        id: "marcus_surgery",
        title: "Marcus's Trauma Surgery (N=4, C=14, Δ_tool=-2, Δ_int=-1, Δ_ext=+4)",
        skill_name: "Trauma Surgery (Marcus)",
        player_skill_n: 4,
        player_control_c: 14,
        player_internal_mod: -1,
        player_known_tool_mod: -2,
        hidden_external_difficulty: 4,
        delta_over: 6,
    },
    CanonicalTaskPreset {
        id: "elena_hack",
        title: "Elena's Intrusion Hack (N=3, C=15, Δ_tool=-2, Δ_int=0, Δ_ext=+2, δ_over=8)",
        skill_name: "Intrusion & Exploitation (Elena)",
        player_skill_n: 3,
        player_control_c: 15,
        player_internal_mod: 0,
        player_known_tool_mod: -2,
        hidden_external_difficulty: 2,
        delta_over: 8,
    },
    CanonicalTaskPreset {
        id: "elena_probe",
        title: "Elena's Sword Probe (N=5, C=16, Δ_tool=0, Δ_int=0, Δ_ext=+2)",
        skill_name: "Arming Sword Probe (Elena)",
        player_skill_n: 5,
        player_control_c: 16,
        player_internal_mod: 0,
        player_known_tool_mod: 0,
        hidden_external_difficulty: 2,
        delta_over: 6,
    },
    CanonicalTaskPreset {
        id: "viktor_cleave",
        title: "Viktor's Greatsword Cleave (N=4, C=13, Δ_tool=0, Δ_int=0, Δ_ext=+3)",
        skill_name: "Greatsword Cleave (Viktor)",
        player_skill_n: 4,
        player_control_c: 13,
        player_internal_mod: 0,
        player_known_tool_mod: 0,
        hidden_external_difficulty: 3,
        delta_over: 6,
    },
    CanonicalTaskPreset {
        id: "social_persuasion",
        title: "Counter-Interrogation Rhetoric (N=3, C=15, Δ_tool=0, Δ_int=+2, Δ_ext=+3)",
        skill_name: "Counter-Interrogation Rhetoric",
        player_skill_n: 3,
        player_control_c: 15,
        player_internal_mod: 2,
        player_known_tool_mod: 0,
        hidden_external_difficulty: 3,
        delta_over: 6,
    },
];

/// Returns all canonical worked task presets.
pub fn list_canonical_task_presets() -> &'static [CanonicalTaskPreset] {
    CANONICAL_TASK_PRESETS
}

/// Looks up a canonical worked task preset by `id` (falling back to `"sarah_lockpick"` if not found).
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
    /// Snake-case benchmark identifier.
    pub id: &'static str,
    /// Domain category (`"Physical"`, `"Subterfuge"`, `"Medical"`, `"Engineering"`, `"Computing"`, `"Investigative"`, `"Social"`, `"Survival"`).
    pub domain: &'static str,
    /// Representative task name.
    pub task_name: &'static str,
    /// Primary skill name (and optional dual-skill requirement).
    pub required_skill: &'static str,
    /// Task Gate Class (`ClassA`, `ClassB`, or `ClassC { min_skill_n }`).
    pub gate_class: TaskGateClass,
    /// Primary Controlling Characteristic code(s), e.g. `"STR"`, `"FIN + PER"`.
    pub controlling_characteristic: &'static str,
    /// Optional Gated Supporting Characteristic code and minimum value (`Min C3 >= X`).
    pub min_supporting_characteristic: Option<(&'static str, i32)>,
    /// Tool dependency tag (`None`, `Situational`, `Mandatory`).
    pub tool_dependency: ToolDependency,
    /// Base hidden external difficulty $\Delta_{\text{ext}}$.
    pub base_delta_ext: i32,
    /// Typical Tick Cost (`TC`) or interval duration string.
    pub tick_cost: &'static str,
}

/// Complete 22-entry Master Task Requirements catalog from §8 of `cascading_d20_and_tasks.md`.
pub static MASTER_TASK_BENCHMARKS: &[MasterTaskBenchmark] = &[
    MasterTaskBenchmark {
        id: "sprint_vault_gap",
        domain: "Athletic",
        task_name: "Sprint & Vault Gap",
        required_skill: "Acrobatics, Tumbling & Parkour",
        gate_class: TaskGateClass::ClassA,
        controlling_characteristic: "KIN + SPD",
        min_supporting_characteristic: None,
        tool_dependency: ToolDependency::None,
        base_delta_ext: 0,
        tick_cost: "6 TC",
    },
    MasterTaskBenchmark {
        id: "free_climb_sheer_wet_rock",
        domain: "Athletic",
        task_name: "Free-Climb Sheer Wet Rock",
        required_skill: "Climbing & Scaling",
        gate_class: TaskGateClass::ClassA,
        controlling_characteristic: "FOR + KIN",
        min_supporting_characteristic: Some(("FIT", 11)),
        tool_dependency: ToolDependency::Situational,
        base_delta_ext: 4,
        tick_cost: "15 TC / pitch",
    },
    MasterTaskBenchmark {
        id: "force_rusted_bulkhead_door",
        domain: "Athletic",
        task_name: "Force Rusted Bulkhead Door",
        required_skill: "Strength & Lifting",
        gate_class: TaskGateClass::ClassA,
        controlling_characteristic: "FOR + FIT",
        min_supporting_characteristic: Some(("HLT", 12)),
        tool_dependency: ToolDependency::Situational,
        base_delta_ext: 2,
        tick_cost: "8 TC",
    },
    MasterTaskBenchmark {
        id: "zero_g_eva_thruster_stabilization",
        domain: "Athletic",
        task_name: "Zero-G EVA Thruster Stabilization",
        required_skill: "Zero-G / EVA Operations",
        gate_class: TaskGateClass::ClassB,
        controlling_characteristic: "KIN + FOC",
        min_supporting_characteristic: Some(("KIN", 10)),
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 2,
        tick_cost: "5 TC",
    },
    MasterTaskBenchmark {
        id: "silent_infiltration_past_sentries",
        domain: "Fieldcraft",
        task_name: "Silent Infiltration Past Sentries",
        required_skill: "Stealth (Sneaking & Hiding)",
        gate_class: TaskGateClass::ClassA,
        controlling_characteristic: "KIN + ITU",
        min_supporting_characteristic: Some(("DIS", 10)),
        tool_dependency: ToolDependency::None,
        base_delta_ext: 2,
        tick_cost: "10 TC",
    },
    MasterTaskBenchmark {
        id: "pick_high_security_pin_lock",
        domain: "Fieldcraft",
        task_name: "Pick High-Security Pin Lock",
        required_skill: "Lockpicking & Physical Intrusion",
        gate_class: TaskGateClass::ClassB,
        controlling_characteristic: "DEX + TAC",
        min_supporting_characteristic: Some(("FOC", 9)),
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 4,
        tick_cost: "15 TC",
    },
    MasterTaskBenchmark {
        id: "bypass_biometric_capacitance_pad",
        domain: "Cyber",
        task_name: "Bypass Biometric Capacitance Pad",
        required_skill: "Lockpicking & Physical Intrusion",
        gate_class: TaskGateClass::ClassC { min_skill_n: 2 },
        controlling_characteristic: "ANA + DEX",
        min_supporting_characteristic: Some(("MEM", 12)),
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 4,
        tick_cost: "20 TC",
    },
    MasterTaskBenchmark {
        id: "disarm_shaped_charge_ied",
        domain: "Technical",
        task_name: "Disarm Shaped-Charge IED",
        required_skill: "Demolitions & Explosive Ordnance (EOD)",
        gate_class: TaskGateClass::ClassC { min_skill_n: 2 },
        controlling_characteristic: "DEX + FOC",
        min_supporting_characteristic: Some(("DIS", 13)),
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 4,
        tick_cost: "30 TC",
    },
    MasterTaskBenchmark {
        id: "staunch_arterial_bleed_in_combat",
        domain: "Medical",
        task_name: "Staunch Arterial Bleed in Combat",
        required_skill: "First Aid & Basic Life Support (BLS / CPR)",
        gate_class: TaskGateClass::ClassA,
        controlling_characteristic: "DEX + FOC",
        min_supporting_characteristic: Some(("DIS", 11)),
        tool_dependency: ToolDependency::Situational,
        base_delta_ext: 2,
        tick_cost: "12 TC",
    },
    MasterTaskBenchmark {
        id: "field_extraction_of_fragment_near_spine",
        domain: "Medical",
        task_name: "Field Extraction of Fragment Near Spine",
        required_skill: "Trauma, Orthopedic & General Surgery",
        gate_class: TaskGateClass::ClassC { min_skill_n: 2 },
        controlling_characteristic: "DEX + FOC",
        min_supporting_characteristic: Some(("DIS", 12)),
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 4,
        tick_cost: "15 min",
    },
    MasterTaskBenchmark {
        id: "synthesize_antitoxin_from_tissue_sample",
        domain: "Medical",
        task_name: "Synthesize Antitoxin from Tissue Sample",
        required_skill: "Pharmacology & Toxicology",
        gate_class: TaskGateClass::ClassC { min_skill_n: 2 },
        controlling_characteristic: "ANA + MEM",
        min_supporting_characteristic: None,
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 4,
        tick_cost: "2 hrs / interval",
    },
    MasterTaskBenchmark {
        id: "patch_ruptured_hydraulic_line",
        domain: "Technical",
        task_name: "Patch Ruptured Hydraulic Line",
        required_skill: "Automotive & Small Engine Mechanics",
        gate_class: TaskGateClass::ClassB,
        controlling_characteristic: "DEX + ANA",
        min_supporting_characteristic: Some(("FOR", 10)),
        tool_dependency: ToolDependency::Situational,
        base_delta_ext: 0,
        tick_cost: "5 min",
    },
    MasterTaskBenchmark {
        id: "reroute_damaged_micro_fission_coolant_loop",
        domain: "Technical",
        task_name: "Reroute Damaged Micro-Fission Coolant Loop",
        required_skill: "Thermodynamics & Fluid Dynamics",
        gate_class: TaskGateClass::ClassC { min_skill_n: 3 },
        controlling_characteristic: "ANA + DEX",
        min_supporting_characteristic: Some(("DIS", 13)),
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 6,
        tick_cost: "10 min / interval",
    },
    MasterTaskBenchmark {
        id: "fabricate_custom_suppressor_baffle",
        domain: "Technical",
        task_name: "Fabricate Custom Suppressor Baffle",
        required_skill: "Armourer & Gunsmithing",
        gate_class: TaskGateClass::ClassB,
        controlling_characteristic: "DEX + ANA",
        min_supporting_characteristic: None,
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 2,
        tick_cost: "1 hr / interval",
    },
    MasterTaskBenchmark {
        id: "packet_sniff_local_unencrypted_subnet",
        domain: "Cyber",
        task_name: "Packet-Sniff Local Unencrypted Subnet",
        required_skill: "Cybersecurity & Network Intrusion",
        gate_class: TaskGateClass::ClassB,
        controlling_characteristic: "ANA + INS",
        min_supporting_characteristic: None,
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 0,
        tick_cost: "10 TC",
    },
    MasterTaskBenchmark {
        id: "live_kernel_injection_on_air_gapped_node",
        domain: "Cyber",
        task_name: "Live Kernel Injection on Air-Gapped Node",
        required_skill: "Cybersecurity & Network Intrusion + Cryptography",
        gate_class: TaskGateClass::ClassC { min_skill_n: 2 },
        controlling_characteristic: "ANA + CRE",
        min_supporting_characteristic: Some(("MEM", 13)),
        tool_dependency: ToolDependency::Mandatory,
        base_delta_ext: 4,
        tick_cost: "15 TC",
    },
    MasterTaskBenchmark {
        id: "reconstruct_ballistic_trajectory_from_ricochets",
        domain: "Science",
        task_name: "Reconstruct Ballistic Trajectory from Ricochets",
        required_skill: "Forensics & Crime Scene Investigation",
        gate_class: TaskGateClass::ClassB,
        controlling_characteristic: "ANA + SIG",
        min_supporting_characteristic: Some(("MEM", 11)),
        tool_dependency: ToolDependency::Situational,
        base_delta_ext: 2,
        tick_cost: "15 min",
    },
    MasterTaskBenchmark {
        id: "spot_ambush_in_dense_urban_ruins",
        domain: "Fieldcraft",
        task_name: "Spot Ambush in Dense Urban Ruins",
        required_skill: "Alertness (Visual & Auditory Situational Awareness)",
        gate_class: TaskGateClass::ClassA,
        controlling_characteristic: "SIG + HEA",
        min_supporting_characteristic: None,
        tool_dependency: ToolDependency::None,
        base_delta_ext: 2,
        tick_cost: "6 TC",
    },
    MasterTaskBenchmark {
        id: "fast_talk_checkpoint_sentry",
        domain: "Social",
        task_name: "Fast-Talk Checkpoint Sentry",
        required_skill: "Deception & Bluff",
        gate_class: TaskGateClass::ClassA,
        controlling_characteristic: "ELO + CRE",
        min_supporting_characteristic: Some(("DIS", 10)),
        tool_dependency: ToolDependency::None,
        base_delta_ext: 2,
        tick_cost: "10 TC",
    },
    MasterTaskBenchmark {
        id: "break_hostile_subject_in_interrogation",
        domain: "Social",
        task_name: "Break Hostile Subject in Interrogation",
        required_skill: "Interrogation & Counter-Interrogation",
        gate_class: TaskGateClass::ClassB,
        controlling_characteristic: "INS + PRE",
        min_supporting_characteristic: Some(("DIS", 12)),
        tool_dependency: ToolDependency::None,
        base_delta_ext: 4,
        tick_cost: "30 min",
    },
    MasterTaskBenchmark {
        id: "rally_panicked_squad_under_artillery_fire",
        domain: "Social",
        task_name: "Rally Panicked Squad Under Artillery Fire",
        required_skill: "Leadership & Command",
        gate_class: TaskGateClass::ClassB,
        controlling_characteristic: "PRE + DIS",
        min_supporting_characteristic: None,
        tool_dependency: ToolDependency::None,
        base_delta_ext: 4,
        tick_cost: "3 TC",
    },
    MasterTaskBenchmark {
        id: "navigate_magnetic_anomaly_storm_by_dead_reckoning",
        domain: "Fieldcraft",
        task_name: "Navigate Magnetic Anomaly Storm by Dead Reckoning",
        required_skill: "Land Navigation & Cartography",
        gate_class: TaskGateClass::ClassB,
        controlling_characteristic: "ANA + SIG",
        min_supporting_characteristic: Some(("DIS", 11)),
        tool_dependency: ToolDependency::Situational,
        base_delta_ext: 4,
        tick_cost: "1 hr",
    },
];

/// Returns all 22 benchmark entries from the §8 Master Task Requirements Table.
pub fn list_master_task_benchmarks() -> &'static [MasterTaskBenchmark] {
    MASTER_TASK_BENCHMARKS
}

/// Looks up a [`MasterTaskBenchmark`] by its `id` or `task_name`.
pub fn get_master_task_benchmark(id_or_name: &str) -> Option<&'static MasterTaskBenchmark> {
    let q = id_or_name.trim();
    MASTER_TASK_BENCHMARKS
        .iter()
        .find(|b| b.id.eq_ignore_ascii_case(q) || b.task_name.eq_ignore_ascii_case(q))
}
