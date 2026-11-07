//! # Contested & Hybrid-Contested Two-List Cascade Resolution Engine
//! (docs/ttrpg/01-foundation/probability_and_calibration.md §6.1–§6.3;
//! docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md §4.1–§4.2, §5).
//!
//! Separated from [crate::dice_probabilities] so uncontested 2d20 Cascade DP/evaluation
//! and Contested / Hybrid-Contested Two-List Cascade resolution each fit comfortably in
//! a single read window.

use std::collections::HashMap;

use satya_schema::{CombatStance, EventMetadata, OutcomeTier, Participant, TtrpgMechanicsEvent};
use serde::{Deserialize, Serialize};

use crate::define_coded_enum;
use crate::dice_pools::{DistributionStats, Pmf, compute_stats, normalize_hash_pmf, sum_dice_pool_pmf};
use crate::dice_probabilities::{
    BotchRule, CarryMode, CascadeOutcomeKind, CascadeRollOutcome, CascadeStepTrace,
    CascadeTaskParams, MarginAggregation, build_cascade_mechanics_event, evaluate_cascade_roll,
    run_cascade_dp,
};
// ---------------------------------------------------------------------------
// 10. Contested & Hybrid-Contested Two-List Cascade Resolution Engine
// ---------------------------------------------------------------------------

define_coded_enum! {
    /// Tie-breaking and margin-interaction rule for Contested Cascade Task Resolution.
    ///
    /// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.1–§6.3) and
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.1–§4.2, §5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum ContestedTieRule [count = 3] {
        /// Recommended Hybrid Rule: positive ties succeed at a grazing cost; GM botches rescue player clean misses.
        #[default]
        HybridSignedMargin => { code: "hybrid_signed_margin", label: "Hybrid Signed Margin" },
        /// Strict Signed Margin ($M_P > M_{GM}$): positive ties fail; GM botches still rescue player clean misses.
        StrictSignedMargin => { code: "strict_signed_margin", label: "Strict Signed Margin" },
        /// Active Player Wins Positive Ties ($S_P \ge S_{GM} \ge 1$).
        ActivePlayerWinsTies => { code: "active_player_wins_ties", label: "Active Player Wins Ties" },
    }
}

define_coded_enum! {
    /// Specific sub-classification of how a Contested Cascade roll resolved.
    ///
    /// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.2–§6.3),
    /// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2), and
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.2, §5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ContestedOutcomeKind [count = 7] {
        /// Player strictly beat GM ($S_P > S_{GM}$) with net margin $\Delta M \ge 3$ and no uncancelled floor breach.
        ExtraordinarySuccess => { code: "EXTRAORDINARY_SUCCESS", label: "Extraordinary Success" },
        /// Player cleanly beat GM ($S_P > S_{GM}$) with no uncancelled floor breach.
        CleanSuccess => { code: "CLEAN_SUCCESS", label: "Clean Success" },
        /// Player beat GM ($S_P > S_{GM}$), but suffered a floor-breach complication ($B_P \ge 1$).
        SuccessWithFloorBreach => { code: "SUCCESS_WITH_FLOOR_BREACH", label: "Success with Floor Breach" },
        /// Player tied GM on positive successes ($S_P == S_{GM} \ge 1$); succeeds at a grazing cost or delay.
        GrazingTieSuccess => { code: "GRAZING_TIE_SUCCESS", label: "Grazing Tie Success" },
        /// Player rolled a Clean Failure ($S_P == 0, M_P == 0$), but the GM botched ($M_{GM} < 0$), creating an opportunistic opening.
        LuckyBreakSuccess => { code: "LUCKY_BREAK_SUCCESS", label: "Lucky Break Success" },
        /// Player failed to beat GM without botching ($M_P \ge 0$, or stalemate $0 == 0$).
        CleanFailure => { code: "CLEAN_FAILURE", label: "Clean Failure" },
        /// Player botched ($M_P < 0$) or failed while suffering a floor breach.
        BotchFailure => { code: "BOTCH_FAILURE", label: "Botch Failure" },
    }
}

impl ContestedOutcomeKind {
    /// Maps the detailed [`ContestedOutcomeKind`] to the canonical 5-tier [`OutcomeTier`].
    pub const fn to_outcome_tier(self) -> OutcomeTier {
        match self {
            Self::ExtraordinarySuccess => OutcomeTier::ExtraordinarySuccess,
            Self::CleanSuccess => OutcomeTier::Success,
            Self::SuccessWithFloorBreach | Self::GrazingTieSuccess | Self::LuckyBreakSuccess => {
                OutcomeTier::PartialSuccess
            }
            Self::CleanFailure => OutcomeTier::Failure,
            Self::BotchFailure => OutcomeTier::CriticalFailure,
        }
    }

    /// Computes the task duration / Tick Cost (`TC`) multiplier per §5 of `cascading_d20_and_tasks.md`:
    /// - `ExtraordinarySuccess`: Reduces task duration by `-15%` per net success above 1 (clamped to a `0.25` floor).
    /// - `GrazingTieSuccess`: Increases Tick Cost (`TC`) by `+25%` (`1.25`).
    /// - All other outcomes: `1.0`.
    pub fn duration_multiplier(self, net_successes: i32) -> f64 {
        match self {
            Self::ExtraordinarySuccess => {
                let net_above_one = (net_successes - 1).max(1) as f64;
                (1.0 - 0.15 * net_above_one).max(0.25)
            }
            Self::GrazingTieSuccess => 1.25,
            _ => 1.0,
        }
    }

    /// External difficulty modifier ($\Delta_{\text{ext}}$) granted to the next linked action per §5:
    /// - `ExtraordinarySuccess`: `-4` ($\Delta_{\text{ext}} - 4$ setup advantage).
    /// - `BotchFailure`: `+4` ($\Delta_{\text{ext}} + 4$ complication/setback).
    /// - All other outcomes: `0`.
    pub const fn next_linked_action_delta_ext(self) -> i32 {
        match self {
            Self::ExtraordinarySuccess => -4,
            Self::BotchFailure => 4,
            _ => 0,
        }
    }

    /// Physical or mental Strain inflicted by a Grazing Tie (`1` point on `GrazingTieSuccess`, `0` otherwise).
    pub const fn grazing_strain_cost(self) -> i32 {
        match self {
            Self::GrazingTieSuccess => 1,
            _ => 0,
        }
    }
}

/// Mode for evaluating the GM's opposing roll in a Contested Task.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.2–§6.3) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.1–§4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GmResistanceMode {
    /// **1-Pair Tiered GM Resistance Roll (Default for Environmental / Technical Tasks)**:
    /// The GM rolls a single hidden `2d20` pair ($X_{GM} = A_{GM} + B_{GM}$) against $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$.
    TieredSinglePair {
        /// Over-target step size per additional GM success (canonical default `4`, matching $K / 5 = 4$).
        step_size: i32,
    },
    /// **Full Multi-Pair Cascade (Used for Active NPC Opponents)**:
    /// The GM rolls an $N_{GM}$-pair sequential `2d20` cascade using the NPC's Skill Level $N_{GM}$ and Controlling Characteristic $C_{GM}$.
    FullCascade,
}

impl Default for GmResistanceMode {
    fn default() -> Self {
        Self::TieredSinglePair { step_size: 4 }
    }
}

/// Configuration for a Contested Two-List Cascade check (Player vs. GM Environment or Active Opponent).
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.1–§6.3),
/// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2), and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.1–§4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContestedCascadeTaskParams {
    /// Player's task parameters (uses player's known personal target $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$).
    pub player: CascadeTaskParams,
    /// GM's hidden resistance or NPC opponent parameters ($\text{TN}_{GM} = 31 - C_{GM}$, $N_{GM} \ge 1$).
    pub gm: CascadeTaskParams,
    /// GM evaluation mode (`TieredSinglePair { step_size: 4 }` for environmental tasks, `FullCascade` for NPCs).
    pub gm_mode: GmResistanceMode,
    /// Tie-breaking and margin interaction rule.
    pub tie_rule: ContestedTieRule,
}

impl ContestedCascadeTaskParams {
    /// Constructs a **1-Pair Tiered GM Resistance Environmental Task** (with canonical `4`-point over-tiers).
    pub const fn new_tiered_environment(
        player_skill_n: usize,
        player_control_c: i32,
        player_internal_mod: i32,
        player_known_tool_mod: i32,
        hidden_external_difficulty: i32,
        tie_rule: ContestedTieRule,
    ) -> Self {
        Self {
            player: CascadeTaskParams::new_k20(
                player_skill_n,
                player_control_c,
                player_internal_mod,
                player_known_tool_mod,
            ),
            gm: CascadeTaskParams::new_k20(1, 10 + hidden_external_difficulty, 0, 0),
            gm_mode: GmResistanceMode::TieredSinglePair { step_size: 4 },
            tie_rule,
        }
    }

    /// Constructs a **Full Dual-Cascade Contested Task** (e.g., against an active NPC with `gm_pairs` skill level).
    pub const fn new_hybrid_environment(
        player_skill_n: usize,
        player_control_c: i32,
        player_internal_mod: i32,
        player_known_tool_mod: i32,
        gm_pairs: usize,
        hidden_external_difficulty: i32,
        tie_rule: ContestedTieRule,
    ) -> Self {
        Self {
            player: CascadeTaskParams::new_k20(
                player_skill_n,
                player_control_c,
                player_internal_mod,
                player_known_tool_mod,
            ),
            gm: CascadeTaskParams::new_k20(gm_pairs, 10 + hidden_external_difficulty, 0, 0),
            gm_mode: GmResistanceMode::FullCascade,
            tie_rule,
        }
    }

    /// Evaluates concrete rolled pair sequences for both the Player and the GM.
    pub fn evaluate_contested_pairs(
        &self,
        player_pairs: &[(i32, i32)],
        gm_pairs: &[(i32, i32)],
    ) -> ContestedCascadeRollOutcome {
        let p_out = self.player.evaluate_pairs(player_pairs);
        let g_out = match self.gm_mode {
            GmResistanceMode::TieredSinglePair { step_size } => {
                evaluate_tiered_single_pair_gm_roll(
                    gm_pairs,
                    self.gm.sides,
                    self.gm.tn_eff(),
                    step_size,
                )
            }
            GmResistanceMode::FullCascade => self.gm.evaluate_pairs(gm_pairs),
        };
        let (contested_kind, net_margin) = classify_contested_outcome(
            p_out.total_successes,
            p_out.raw_botches,
            p_out.signed_margin,
            g_out.total_successes,
            g_out.raw_botches,
            g_out.signed_margin,
            self.tie_rule,
        );
        ContestedCascadeRollOutcome {
            net_successes: p_out.total_successes - g_out.total_successes,
            net_signed_margin: net_margin,
            contested_kind,
            outcome_tier: contested_kind.to_outcome_tier(),
            player_outcome: p_out,
            gm_outcome: g_out,
        }
    }

    /// Computes the exact combinatorial probability distribution ([`ContestedCascadeOdds`])
    /// for this contested task configuration.
    pub fn exact_contested_odds(&self) -> ContestedCascadeOdds {
        compute_contested_cascade_odds_with_mode(
            &self.player,
            &self.gm,
            self.gm_mode,
            self.tie_rule,
        )
    }
}

/// Complete live roll outcome for a Contested Cascade task check.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.2–§6.3),
/// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2), and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.2, §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContestedCascadeRollOutcome {
    /// Step-by-step cascade outcome for the Player.
    pub player_outcome: CascadeRollOutcome,
    /// Step-by-step cascade outcome for the GM / Opponent.
    pub gm_outcome: CascadeRollOutcome,
    /// Difference in raw positive successes ($S_P - S_{GM}$).
    pub net_successes: i32,
    /// Difference in unified signed margins ($M_P - M_{GM}$).
    pub net_signed_margin: i32,
    /// Detailed 7-way contested outcome classification.
    pub contested_kind: ContestedOutcomeKind,
    /// Canonical 5-tier [`OutcomeTier`] (`ExtraordinarySuccess`, `Success`, `PartialSuccess`, `Failure`, `CriticalFailure`).
    pub outcome_tier: OutcomeTier,
}

/// Exact combinatorial probability breakdown and net margin PMF for a Contested Cascade check.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.1–§6.3),
/// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2), and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.1–§4.2, §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContestedCascadeOdds {
    /// Exact PMF of the net signed margin $\Delta M = M_P - M_{GM}$.
    pub net_signed_margin_pmf: Pmf,
    /// Exact PMF of the net success count $\Delta S = S_P - S_{GM}$.
    pub net_success_pmf: Pmf,
    /// Total probability of achieving any success tier (`ExtraordinarySuccess + CleanSuccess + PartialSuccess`).
    pub prob_total_win: f64,
    /// Probability of an unblemished win ($S_P > S_{GM}$ without complication, or `ActivePlayerWinsTies`).
    pub prob_clean_or_extraordinary_win: f64,
    /// Probability of `OutcomeTier::ExtraordinarySuccess` ($\Delta M \ge 3, S_P > S_{GM}$).
    pub prob_extraordinary_success: f64,
    /// Probability of standard `OutcomeTier::Success`.
    pub prob_clean_success: f64,
    /// Total probability of `OutcomeTier::PartialSuccess` (`SuccessWithFloorBreach + GrazingTieSuccess + LuckyBreakSuccess`).
    pub prob_partial_success: f64,
    /// Sub-probability of `PartialSuccess` caused by a player floor breach ($B_P \ge 1$) on a winning roll.
    pub prob_floor_breach_partial: f64,
    /// Sub-probability of `PartialSuccess` caused by a positive tie ($S_P == S_{GM} \ge 1$).
    pub prob_grazing_tie_partial: f64,
    /// Sub-probability of `PartialSuccess` caused by a GM botch ($M_{GM} < 0$) rescuing a player's Clean Miss ($M_P == 0$).
    pub prob_lucky_break_partial: f64,
    /// Probability of `OutcomeTier::Failure` (`CleanFailure`).
    pub prob_clean_failure: f64,
    /// Probability of `OutcomeTier::CriticalFailure` (`BotchFailure`).
    pub prob_botch_failure: f64,
    /// Summary moments (mean, variance, std, skewness, excess kurtosis) of `net_signed_margin_pmf`.
    pub net_margin_stats: DistributionStats,
    /// Sarle's Bimodality Coefficient $\text{BC} = (\gamma_1^2 + 1) / (\kappa_2 + 3)$ of `net_signed_margin_pmf`.
    pub bimodality_coefficient: f64,
}

/// Classifies a single `(player, gm)` state realization into [`ContestedOutcomeKind`] and net margin $\Delta M$.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.2–§6.3) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.2, §5).
pub fn classify_contested_outcome(
    sp: i32,
    bp: i32,
    mp: i32,
    sg: i32,
    _bg: i32,
    mg: i32,
    tie_rule: ContestedTieRule,
) -> (ContestedOutcomeKind, i32) {
    let net_margin = mp - mg;
    let kind = match tie_rule {
        ContestedTieRule::HybridSignedMargin | ContestedTieRule::StrictSignedMargin => {
            if sp > sg {
                if bp >= 1 && mg >= 0 {
                    ContestedOutcomeKind::SuccessWithFloorBreach
                } else if net_margin >= 3 {
                    ContestedOutcomeKind::ExtraordinarySuccess
                } else {
                    ContestedOutcomeKind::CleanSuccess
                }
            } else if tie_rule == ContestedTieRule::HybridSignedMargin && sp == sg && sp >= 1 {
                ContestedOutcomeKind::GrazingTieSuccess
            } else if sp == 0 && mp == 0 && mg < 0 {
                ContestedOutcomeKind::LuckyBreakSuccess
            } else if mp < 0 || bp >= 1 {
                ContestedOutcomeKind::BotchFailure
            } else {
                ContestedOutcomeKind::CleanFailure
            }
        }
        ContestedTieRule::ActivePlayerWinsTies => {
            if sp >= sg && sp >= 1 {
                if bp >= 1 {
                    ContestedOutcomeKind::SuccessWithFloorBreach
                } else if (sp - sg) >= 3 {
                    ContestedOutcomeKind::ExtraordinarySuccess
                } else {
                    ContestedOutcomeKind::CleanSuccess
                }
            } else if mp < 0 || bp >= 1 {
                ContestedOutcomeKind::BotchFailure
            } else {
                ContestedOutcomeKind::CleanFailure
            }
        }
    };
    (kind, net_margin)
}

/// Computes the exact joint distribution over `(succ, raw_botch, signed_margin)` for a [`CascadeTaskParams`].
fn cascade_joint_outcome_pmf(params: &CascadeTaskParams) -> HashMap<(i32, i32, i32), f64> {
    let (state_probs, _, _) = run_cascade_dp(
        params.skill_level,
        params.sides,
        params.tn_eff(),
        params.delta_over,
        params.effective_delta_under(),
        params.carry_mode,
        false,
        false,
        params.botch_rule,
    );

    let mut joint: HashMap<(i32, i32, i32), f64> = HashMap::new();
    for (&(_mod, succ, raw_botch, min_miss_tier), &prob) in &state_probs {
        let eff_botch = params
            .botch_rule
            .effective_botches(succ, raw_botch, min_miss_tier);
        let signed_margin = params.margin_aggregation.compute(succ, eff_botch);
        *joint.entry((succ, raw_botch, signed_margin)).or_insert(0.0) += prob;
    }
    joint
}

/// Evaluates a single GM `2dK` resistance pair using the Tiered Over-Success & Under-Botch rule:
/// - If $X_{GM} > \text{TN}_{GM}$, $S_{GM} = 1 + \lfloor (X_{GM} - \text{TN}_{GM} - 1) / \text{step\_size} \rfloor$.
/// - If $X_{GM} \le \text{TN}_{GM}$, $S_{GM} = 0$ and $B_{GM} = \min(5, \lfloor (\text{TN}_{GM} - X_{GM}) / 4 \rfloor)$.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.2),
/// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2), and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.1–§4.2).
pub fn evaluate_tiered_single_pair_gm_roll(
    gm_pairs: &[(i32, i32)],
    sides: i32,
    tn_gm: i32,
    step_size: i32,
) -> CascadeRollOutcome {
    let Some(&(die_a, die_b)) = gm_pairs.first() else {
        return evaluate_cascade_roll(
            &[],
            sides,
            tn_gm,
            6,
            6,
            CarryMode::MarkovStep,
            false,
            false,
            BotchRule::ClosestMiss,
            MarginAggregation::PiecewiseZeroFloor,
        );
    };
    let raw_sum = die_a + die_b;
    let over_step = step_size.max(1);
    let under_step = (sides / 5).max(2);
    let (total_successes, effective_botches, miss_distance) = if raw_sum > tn_gm {
        (1 + (raw_sum - tn_gm - 1) / over_step, 0, 0)
    } else {
        let dist = tn_gm - raw_sum;
        (0, (dist / under_step).min(5), dist)
    };
    let signed_margin =
        MarginAggregation::PiecewiseZeroFloor.compute(total_successes, effective_botches);
    let outcome_kind = CascadeOutcomeKind::classify(total_successes, 0, effective_botches);
    let outcome_tier = outcome_kind.to_outcome_tier(total_successes);
    CascadeRollOutcome {
        steps: vec![CascadeStepTrace {
            pair_index: 1,
            die_a,
            die_b,
            raw_sum,
            carry_in: 0,
            effective_y: raw_sum,
            tn_eff: tn_gm,
            succeeded: total_successes >= 1,
            step_botch: effective_botches >= 1,
            miss_distance,
            carry_out: 0,
        }],
        total_successes,
        raw_botches: 0,
        effective_botches,
        min_miss_tier: effective_botches,
        signed_margin,
        final_pair_y: raw_sum,
        outcome_kind,
        outcome_tier,
    }
}

/// Computes the exact joint distribution over `(succ, raw_botch, signed_margin)` for a 1-Pair Tiered GM Resistance Roll.
fn tiered_single_pair_gm_joint_pmf(
    sides: i32,
    tn_gm: i32,
    step_size: i32,
) -> HashMap<(i32, i32, i32), f64> {
    let base_pmf = sum_dice_pool_pmf(2, sides, 0);
    let over_step = step_size.max(1);
    let under_step = (sides / 5).max(2);
    let mut joint: HashMap<(i32, i32, i32), f64> = HashMap::new();
    for (&x, &prob) in &base_pmf {
        let (succ, signed_margin) = if x > tn_gm {
            let s = 1 + (x - tn_gm - 1) / over_step;
            (s, s)
        } else {
            let b = ((tn_gm - x) / under_step).min(5);
            (0, -b)
        };
        *joint.entry((succ, 0, signed_margin)).or_insert(0.0) += prob;
    }
    joint
}

/// Computes exact contested probabilities ([`ContestedCascadeOdds`]) using `FullCascade` mode.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.1–§6.3) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.1–§4.2).
pub fn compute_contested_cascade_odds(
    player: &CascadeTaskParams,
    gm: &CascadeTaskParams,
    tie_rule: ContestedTieRule,
) -> ContestedCascadeOdds {
    compute_contested_cascade_odds_with_mode(player, gm, GmResistanceMode::FullCascade, tie_rule)
}

/// Computes exact contested probabilities ([`ContestedCascadeOdds`]) by convolving the joint `(succ, raw_botch, signed_margin)`
/// state distributions of `player` and `gm` under `gm_mode` (`TieredSinglePair` or `FullCascade`).
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.1–§6.3),
/// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2), and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.1–§4.2, §5).
pub fn compute_contested_cascade_odds_with_mode(
    player: &CascadeTaskParams,
    gm: &CascadeTaskParams,
    gm_mode: GmResistanceMode,
    tie_rule: ContestedTieRule,
) -> ContestedCascadeOdds {
    let p_joint = cascade_joint_outcome_pmf(player);
    let g_joint = match gm_mode {
        GmResistanceMode::TieredSinglePair { step_size } => {
            tiered_single_pair_gm_joint_pmf(gm.sides, gm.tn_eff(), step_size)
        }
        GmResistanceMode::FullCascade => cascade_joint_outcome_pmf(gm),
    };

    let mut net_margin_map: HashMap<i32, f64> = HashMap::new();
    let mut net_succ_map: HashMap<i32, f64> = HashMap::new();

    let mut prob_extraordinary_success = 0.0;
    let mut prob_clean_success = 0.0;
    let mut prob_floor_breach_partial = 0.0;
    let mut prob_grazing_tie_partial = 0.0;
    let mut prob_lucky_break_partial = 0.0;
    let mut prob_clean_failure = 0.0;
    let mut prob_botch_failure = 0.0;

    for (&(sp, bp, mp), &pp) in &p_joint {
        for (&(sg, bg, mg), &pg) in &g_joint {
            let jp = pp * pg;
            let (kind, net_m) = classify_contested_outcome(sp, bp, mp, sg, bg, mg, tie_rule);
            *net_margin_map.entry(net_m).or_insert(0.0) += jp;
            *net_succ_map.entry(sp - sg).or_insert(0.0) += jp;

            match kind {
                ContestedOutcomeKind::ExtraordinarySuccess => prob_extraordinary_success += jp,
                ContestedOutcomeKind::CleanSuccess => prob_clean_success += jp,
                ContestedOutcomeKind::SuccessWithFloorBreach => prob_floor_breach_partial += jp,
                ContestedOutcomeKind::GrazingTieSuccess => prob_grazing_tie_partial += jp,
                ContestedOutcomeKind::LuckyBreakSuccess => prob_lucky_break_partial += jp,
                ContestedOutcomeKind::CleanFailure => prob_clean_failure += jp,
                ContestedOutcomeKind::BotchFailure => prob_botch_failure += jp,
            }
        }
    }

    let net_signed_margin_pmf = normalize_hash_pmf(&net_margin_map);
    let net_success_pmf = normalize_hash_pmf(&net_succ_map);
    let stats = compute_stats(&net_signed_margin_pmf);
    let bimodality_coefficient = stats.bimodality_coefficient();

    let prob_clean_or_extraordinary_win = prob_extraordinary_success + prob_clean_success;
    let prob_partial_success =
        prob_floor_breach_partial + prob_grazing_tie_partial + prob_lucky_break_partial;
    let prob_total_win = prob_clean_or_extraordinary_win + prob_partial_success;

    ContestedCascadeOdds {
        net_signed_margin_pmf,
        net_success_pmf,
        prob_total_win,
        prob_clean_or_extraordinary_win,
        prob_extraordinary_success,
        prob_clean_success,
        prob_partial_success,
        prob_floor_breach_partial,
        prob_grazing_tie_partial,
        prob_lucky_break_partial,
        prob_clean_failure,
        prob_botch_failure,
        net_margin_stats: stats,
        bimodality_coefficient,
    }
}

/// Evaluates an Always-Contested Two-List Cascading (`2d20`) task roll per
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` and returns both the
/// [`TtrpgMechanicsEvent`] telemetry record (including §5 Tick Cost duration modifiers for
/// `ExtraordinarySuccess` and `GrazingTieSuccess`) and the [`ContestedCascadeRollOutcome`].
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.2–§6.3),
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.1–§4.2, §5), and
/// `docs/ttrpg/02-core-mechanics/action_economy_and_ooda.md` (§3.1, §5.1).
#[allow(clippy::too_many_arguments)]
pub fn evaluate_satya_contested_cascade_task(
    metadata: EventMetadata,
    actor: Participant,
    target_participants: Vec<Participant>,
    skill_name: String,
    params: &ContestedCascadeTaskParams,
    player_pairs: &[(i32, i32)],
    gm_pairs: &[(i32, i32)],
    combat_stance: Option<CombatStance>,
    fatigue_level: i32,
    wound_shock_index: f32,
) -> (TtrpgMechanicsEvent, ContestedCascadeRollOutcome) {
    let outcome = params.evaluate_contested_pairs(player_pairs, gm_pairs);
    let mut dice_rolled_d20: Vec<i32> = player_pairs.iter().flat_map(|&(a, b)| [a, b]).collect();
    dice_rolled_d20.extend(gm_pairs.iter().flat_map(|&(a, b)| [a, b]));

    let outcome_tc_mult = outcome
        .contested_kind
        .duration_multiplier(outcome.net_successes) as f32;
    let event = build_cascade_mechanics_event(
        metadata,
        actor,
        target_participants,
        skill_name,
        params.player.control_c,
        params.player.tn_eff(),
        dice_rolled_d20,
        outcome.player_outcome.total_successes,
        outcome.outcome_tier,
        combat_stance,
        fatigue_level,
        wound_shock_index,
        outcome_tc_mult,
        format!(
            "contested(kind={}, tn_p={}, tn_gm={}, s_p={}, s_gm={}, net_margin={:+}, tc_mult={:.2})",
            outcome.contested_kind,
            params.player.tn_eff(),
            params.gm.tn_eff(),
            outcome.player_outcome.total_successes,
            outcome.gm_outcome.total_successes,
            outcome.net_signed_margin,
            outcome_tc_mult
        ),
    );

    (event, outcome)
}
