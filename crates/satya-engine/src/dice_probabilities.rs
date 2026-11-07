//! Exact combinatorial and dynamic-programming probability distribution engines for Satya TTRPG dice mechanics.
//!
//! Canonical Rust source of truth (exposed to Python via the `satya_py` PyO3 crate) and calibrated per
//! `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1–§6),
//! `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1–§2.2),
//! `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§1–§5), and
//! `docs/ttrpg/02-core-mechanics/action_economy_and_ooda.md` (§3.1, §5.1).
//!
//! Provides:
//! - Exact probability mass functions ([`Pmf`]) and summary moments ([`DistributionStats`]: mean, variance, std, skewness, excess kurtosis).
//! - Additive & negative dice pools ([`sum_dice_pool_pmf`]), binomial & comparative roll-under `2d20` critical-`1` pools ([`count_successes_pmf`]).
//! - Exploding dice & Gumbel keep-highest pools ([`single_exploding_die_pmf`], [`exploding_pool_keep_highest_pmf`]).
//! - Order statistics ([`order_statistic_pmf`]), center-distance Focus vs. Chaos ([`center_distance_selection_pmf`]),
//!   median-gated bimodal pools ([`median_gated_bimodal_pmf`]), and multiplicative log-normal pools ([`multiplicative_pool_pmf`]).
//! - Two-List Cascading ("Watershed" / "Recoil") exact DP engines ([`paired_list_cascade_pmfs`], [`paired_list_signed_margin_pmfs`]).
//! - Game-engine live roll evaluator ([`CascadeTaskParams`], [`CascadeRollOutcome`], [`evaluate_cascade_roll`], [`evaluate_satya_cascade_task`])
//!   for resolving sequential `2dK` pairs with Four-Factor Task Calibration (`Skill N`, `Control C`, `Internal Mod`, `External Mod`).

use satya_schema::{
    CombatStance, EventMetadata, MechanicEventType, OutcomeTier, Participant, TtrpgMechanicsEvent,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use crate::define_coded_enum;
use crate::dice_pools::normalize_hash_pmf;
pub use crate::dice_pools::{
    CenterDistanceMode, DistributionStats, Pmf, center_distance_selection_pmf, compute_stats,
    convolve_pmfs, count_successes_pmf, exploding_pool_keep_highest_pmf, median_gated_bimodal_pmf,
    multiplicative_pool_pmf, normalize_pmf, order_statistic_pmf, single_die_pmf,
    single_exploding_die_pmf, sum_dice_pool_pmf, try_median_gated_bimodal_pmf,
    try_order_statistic_pmf,
};

pub use crate::contested_cascade::*;
// ---------------------------------------------------------------------------
// 8. Paired-List Cascading / Contagious Dice (The Watershed Mechanic)
// ---------------------------------------------------------------------------

define_coded_enum! {
    /// Carry-over propagation rule governing how `mod_{n+1}` is calculated from step `n` in the Two-List Cascade.
    ///
    /// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§2.1–§2.2),
    /// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1), and
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1–§3.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum CarryMode [count = 3] {
        /// 1-step Markov memory (`mod_{n+1}` depends only on whether step `n` rolled over or under `C`).
        #[default]
        MarkovStep => { code: "markov_step", label: "Markov Step" },
        /// Compounding momentum (`mod_{n+1} = mod_n + step_mod`).
        CumulativeSnowball => { code: "cumulative_snowball", label: "Cumulative Snowball" },
        /// Exact difference carry (`mod_{n+1} = Y_n - C`, or `C - Y_n` when `invert_feedback = true`).
        LinearMargin => { code: "linear_margin", label: "Linear Margin" },
    }
}

impl CarryMode {
    /// Computes the carry-over modifier $\text{mod}_{n+1}$ propagated from step $n$.
    ///
    /// An exact threshold tie ($Y_n == \text{TN}_P$) produces `0` successes and `0` carry-over
    /// (`step_delta = 0`), which is the calibrated tie behavior behind the `47.50%` / `60.68%` / `70.10%`
    /// success tables in `docs/ttrpg/01-foundation/probability_and_calibration.md` (§4.2–§4.3) and
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1).
    #[inline]
    pub const fn next_mod(
        self,
        mod_n: i32,
        yn: i32,
        characteristic_c: i32,
        eff_over: i32,
        eff_under: i32,
        sign: i32,
    ) -> i32 {
        let step_delta = if yn > characteristic_c {
            eff_over
        } else if yn < characteristic_c {
            eff_under
        } else {
            0
        };
        match self {
            Self::MarkovStep => step_delta,
            Self::CumulativeSnowball => mod_n + step_delta,
            Self::LinearMargin => sign * (yn - characteristic_c),
        }
    }
}

define_coded_enum! {
    /// Botch / complication evaluation rule for [`paired_list_signed_margin_pmfs`].
    ///
    /// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§3.1–§3.3),
    /// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1–§2.2), and
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.2–§3.3).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum BotchRule [count = 3] {
        /// Monotonic / Anti-Paradox rule: when `successes == 0`, failure severity is determined by the closest pair
        /// to `C_eff` across all `N` pairs, binned every `max(2, K / 5)` points (`0..=5`).
        #[default]
        ClosestMiss => { code: "closest_miss", label: "Closest Miss" },
        /// Physical dice bound breach: `Y_n > 2K` (in Roll-Under) or `Y_n < 2` (in Roll-Over).
        CeilingBreach => { code: "ceiling_breach", label: "Ceiling Breach" },
        /// Relative overrun: `Y_n > C_eff + K` (in Roll-Under) or `Y_n < C_eff - K` (in Roll-Over).
        OverrunK => { code: "overrun_k", label: "Overrun K" },
    }
}

impl BotchRule {
    /// Resolves effective botch severity from `(succ, raw_botch, min_miss_tier)`.
    #[inline]
    pub const fn effective_botches(self, succ: i32, raw_botch: i32, min_miss_tier: i32) -> i32 {
        if matches!(self, Self::ClosestMiss) && succ == 0 {
            min_miss_tier
        } else {
            raw_botch
        }
    }
}

define_coded_enum! {
    /// Rule for combining total successes and botches into the unified `-N..=+N` signed margin.
    ///
    /// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§3.2),
    /// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2), and
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.3).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum MarginAggregation [count = 2] {
        /// If `successes > 0`, margin is `+successes`; if `successes == 0`, margin is `-eff_botch`.
        #[default]
        PiecewiseZeroFloor => { code: "piecewise_zero_floor", label: "Piecewise Zero Floor" },
        /// Margin is `successes - eff_botch` across the whole chain.
        NetDifference => { code: "net_difference", label: "Net Difference" },
    }
}

impl MarginAggregation {
    /// Combines total successes and effective botches into a unified signed margin.
    #[inline]
    pub const fn compute(self, succ: i32, eff_botch: i32) -> i32 {
        match self {
            Self::PiecewiseZeroFloor => {
                if succ > 0 {
                    succ
                } else {
                    -eff_botch
                }
            }
            Self::NetDifference => succ - eff_botch,
        }
    }
}

/// Computes exact PMFs `(success_count_pmf, final_pair_value_pmf)` for the Two-List Cascading
/// ("Watershed" or "Recoil") Dice Mechanic.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§2–§4) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1–§3.2).
#[allow(clippy::too_many_arguments)]
pub fn paired_list_cascade_pmfs(
    num_pairs: usize,
    sides: i32,
    characteristic_c: i32,
    delta_over: i32,
    delta_under: i32,
    mode: CarryMode,
    count_under_as_success: bool,
    invert_feedback: bool,
) -> (Pmf, Pmf) {
    let res = paired_list_signed_margin_pmfs(
        num_pairs,
        sides,
        characteristic_c,
        delta_over,
        delta_under,
        mode,
        count_under_as_success,
        invert_feedback,
        BotchRule::ClosestMiss,
        MarginAggregation::PiecewiseZeroFloor,
    );
    (res.success_count_pmf, res.final_pair_value_pmf)
}

/// Complete outcome breakdown from [`paired_list_signed_margin_pmfs`] including Degrees of Failure and Degrees of Success.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§3–§4),
/// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2), and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1–§3.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedCascadeResult {
    pub signed_margin_pmf: Pmf,
    pub success_count_pmf: Pmf,
    pub botch_count_pmf: Pmf,
    pub final_pair_value_pmf: Pmf,
    pub failure_overshoot_pmf: Pmf,
    pub prob_any_success: f64,
    /// Probability of succeeding while also triggering at least 1 floor/ceiling breach (`successes >= 1 && botches >= 1`).
    pub prob_success_with_complication: f64,
    pub prob_clean_failure: f64,
    pub prob_botch_failure: f64,
}

#[inline]
fn evaluate_step_success_and_botch(
    y_val: i32,
    sides: i32,
    characteristic_c: i32,
    count_under_as_success: bool,
    botch_rule: BotchRule,
) -> (i32, i32, i32) {
    let (succ, miss_dist, bound_breach, overrun_breach) = if count_under_as_success {
        (
            i32::from(y_val < characteristic_c),
            (y_val - characteristic_c).max(0),
            i32::from(y_val > 2 * sides),
            i32::from(y_val > characteristic_c + sides),
        )
    } else {
        (
            i32::from(y_val > characteristic_c),
            (characteristic_c - y_val).max(0),
            i32::from(y_val < 2),
            i32::from(y_val < characteristic_c - sides),
        )
    };
    let botch = if succ == 1 {
        0
    } else {
        match botch_rule {
            BotchRule::CeilingBreach | BotchRule::ClosestMiss => bound_breach,
            BotchRule::OverrunK => overrun_breach,
        }
    };
    (succ, botch, miss_dist)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn run_cascade_dp(
    num_pairs: usize,
    sides: i32,
    characteristic_c: i32,
    delta_over: i32,
    delta_under: i32,
    mode: CarryMode,
    count_under_as_success: bool,
    invert_feedback: bool,
    botch_rule: BotchRule,
) -> (
    HashMap<(i32, i32, i32, i32), f64>,
    HashMap<i32, f64>,
    HashMap<i32, f64>,
) {
    let pair_pmf = sum_dice_pool_pmf(2, sides, 0);
    let sign = if invert_feedback { -1 } else { 1 };
    let eff_over = sign * delta_over;
    let eff_under = -sign * delta_under;
    let tier_step = (sides / 5).max(2);

    // DP state: (current_mod, successes_so_far, botches_so_far, min_miss_tier) -> prob
    let mut state_probs: HashMap<(i32, i32, i32, i32), f64> = HashMap::from([((0, 0, 0, 5), 1.0)]);
    let mut final_y_pmf: HashMap<i32, f64> = HashMap::new();
    let mut overshoot_raw: HashMap<i32, f64> = HashMap::new();

    for step in 1..=num_pairs {
        let is_last = step == num_pairs;
        let mut next_states: HashMap<(i32, i32, i32, i32), f64> = HashMap::new();
        for (&(mod_n, succ_so_far, botch_so_far, min_miss_so_far), &p_state) in &state_probs {
            for (&xn, &px) in &pair_pmf {
                let yn = xn + mod_n;
                let (sn, bn, mn) = evaluate_step_success_and_botch(
                    yn,
                    sides,
                    characteristic_c,
                    count_under_as_success,
                    botch_rule,
                );
                let new_succ = succ_so_far + sn;
                let new_botch = botch_so_far + bn;
                let new_min_miss = min_miss_so_far.min((mn / tier_step).min(5));
                let joint_p = p_state * px;

                if is_last {
                    *final_y_pmf.entry(yn).or_insert(0.0) += joint_p;
                    if new_succ == 0 {
                        let overshoot = if count_under_as_success {
                            yn - characteristic_c
                        } else {
                            characteristic_c - yn
                        };
                        *overshoot_raw.entry(overshoot).or_insert(0.0) += joint_p;
                    }
                }

                let next_mod = if is_last {
                    0
                } else {
                    mode.next_mod(mod_n, yn, characteristic_c, eff_over, eff_under, sign)
                };

                *next_states
                    .entry((next_mod, new_succ, new_botch, new_min_miss))
                    .or_insert(0.0) += joint_p;
            }
        }
        state_probs = next_states;
    }

    (state_probs, final_y_pmf, overshoot_raw)
}

/// Computes the full `-N..=+N` Signed Outcome Spectrum (Degrees of Failure & Success) for the Two-List Cascade.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§3–§4) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1–§3.3).
#[allow(clippy::too_many_arguments)]
pub fn paired_list_signed_margin_pmfs(
    num_pairs: usize,
    sides: i32,
    characteristic_c: i32,
    delta_over: i32,
    delta_under: i32,
    mode: CarryMode,
    count_under_as_success: bool,
    invert_feedback: bool,
    botch_rule: BotchRule,
    margin_aggregation: MarginAggregation,
) -> SignedCascadeResult {
    if num_pairs == 0 || sides <= 0 {
        return SignedCascadeResult {
            signed_margin_pmf: BTreeMap::new(),
            success_count_pmf: BTreeMap::new(),
            botch_count_pmf: BTreeMap::new(),
            final_pair_value_pmf: BTreeMap::new(),
            failure_overshoot_pmf: BTreeMap::new(),
            prob_any_success: 0.0,
            prob_success_with_complication: 0.0,
            prob_clean_failure: 0.0,
            prob_botch_failure: 0.0,
        };
    }

    let (state_probs, final_y_pmf, overshoot_raw) = run_cascade_dp(
        num_pairs,
        sides,
        characteristic_c,
        delta_over,
        delta_under,
        mode,
        count_under_as_success,
        invert_feedback,
        botch_rule,
    );

    let mut signed_margin_pmf: HashMap<i32, f64> = HashMap::new();
    let mut success_pmf: HashMap<i32, f64> = HashMap::new();
    let mut botch_pmf: HashMap<i32, f64> = HashMap::new();

    let mut prob_any_succ = 0.0_f64;
    let mut prob_succ_with_comp = 0.0_f64;
    let mut prob_clean_fail = 0.0_f64;
    let mut prob_botch_fail = 0.0_f64;

    for (&(_mod, succ, botch, min_miss_tier), &prob) in &state_probs {
        let eff_botch = botch_rule.effective_botches(succ, botch, min_miss_tier);
        *success_pmf.entry(succ).or_insert(0.0) += prob;
        *botch_pmf.entry(eff_botch).or_insert(0.0) += prob;

        let margin = margin_aggregation.compute(succ, eff_botch);
        *signed_margin_pmf.entry(margin).or_insert(0.0) += prob;

        if succ >= 1 {
            prob_any_succ += prob;
            if botch >= 1 {
                prob_succ_with_comp += prob;
            }
        } else if eff_botch == 0 {
            prob_clean_fail += prob;
        } else {
            prob_botch_fail += prob;
        }
    }

    SignedCascadeResult {
        signed_margin_pmf: normalize_hash_pmf(&signed_margin_pmf),
        success_count_pmf: normalize_hash_pmf(&success_pmf),
        botch_count_pmf: normalize_hash_pmf(&botch_pmf),
        final_pair_value_pmf: normalize_hash_pmf(&final_y_pmf),
        failure_overshoot_pmf: normalize_hash_pmf(&overshoot_raw),
        prob_any_success: prob_any_succ,
        prob_success_with_complication: prob_succ_with_comp,
        prob_clean_failure: prob_clean_fail,
        prob_botch_failure: prob_botch_fail,
    }
}

// ---------------------------------------------------------------------------
// 9. Game-Engine Live Task Resolution (Four-Factor Cascade Evaluator)
// ---------------------------------------------------------------------------

/// Four-Factor Task configuration for Satya's Two-List Cascading ("Watershed") resolution engine
/// as specified in `docs/ttrpg/01-foundation/probability_and_calibration.md` (§4) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§1, §3.1–§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeTaskParams {
    pub skill_level: usize,
    /// 2. Controlling Characteristic $C \in [1, 20]$ (human playable range `4..=18`, average `10`).
    pub control_c: i32,
    /// 3. Internal Character Modifier $\Delta_{\text{int}} \in [-2, +10]$ (Fatigue, Injury, Stress; added to `base_delta_under`).
    pub internal_mod: i32,
    /// 4. External Task Modifier $\Delta_{\text{ext}} \in [-10, +16]$ (Range, Darkness, Tool Quality; shifts $\text{TN}_{\text{eff}}$).
    pub external_mod: i32,
    pub sides: i32,
    /// Positive momentum bonus $+\delta_{\text{over}}$ on a successful pair (default `6` for `K=20`).
    pub delta_over: i32,
    /// Unfatigued baseline failure penalty $\delta_{\text{base}}$ (default `6` for `K=20`).
    pub base_delta_under: i32,
    pub carry_mode: CarryMode,
    pub botch_rule: BotchRule,
    pub margin_aggregation: MarginAggregation,
}

impl Default for CascadeTaskParams {
    fn default() -> Self {
        Self::new_k20(3, 10, 0, 0)
    }
}

impl CascadeTaskParams {
    /// Constructs a canonical `K = 20` (`2d20`) Four-Factor Task configuration per
    /// `docs/ttrpg/01-foundation/probability_and_calibration.md` (§4.1) and
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1):
    /// - $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$
    /// - $\delta_{\text{over}} = +6$
    /// - $\delta_{\text{under}} = \max(0, 6 + \Delta_{\text{int}})$
    pub const fn new_k20(
        skill_level: usize,
        control_c: i32,
        internal_mod: i32,
        external_mod: i32,
    ) -> Self {
        Self {
            skill_level,
            control_c,
            internal_mod,
            external_mod,
            sides: 20,
            delta_over: 6,
            base_delta_under: 6,
            carry_mode: CarryMode::MarkovStep,
            botch_rule: BotchRule::ClosestMiss,
            margin_aggregation: MarginAggregation::PiecewiseZeroFloor,
        }
    }

    /// Computes the Midpoint-Anchored Roll-Over target number:
    /// $$\text{TN}_{\text{eff}} = \lfloor 1.5K \rfloor + 1 - C + \Delta_{\text{ext}}$$
    /// For $K = 20$, $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$ (so $C = 10, \Delta_{\text{ext}} = 0 \implies \text{TN}_{\text{eff}} = 21$).
    pub const fn tn_eff(&self) -> i32 {
        (3 * self.sides) / 2 + 1 - self.control_c + self.external_mod
    }

    /// Computes the effective failure carry penalty magnitude $\delta_{\text{under}} = \max(0, \delta_{\text{base}} + \Delta_{\text{int}})$.
    pub fn effective_delta_under(&self) -> i32 {
        (self.base_delta_under + self.internal_mod).max(0)
    }

    pub fn exact_odds(&self) -> SignedCascadeResult {
        paired_list_signed_margin_pmfs(
            self.skill_level,
            self.sides,
            self.tn_eff(),
            self.delta_over,
            self.effective_delta_under(),
            self.carry_mode,
            false,
            false,
            self.botch_rule,
            self.margin_aggregation,
        )
    }

    pub fn evaluate_pairs(&self, pairs: &[(i32, i32)]) -> CascadeRollOutcome {
        evaluate_cascade_roll(
            pairs,
            self.sides,
            self.tn_eff(),
            self.delta_over,
            self.effective_delta_under(),
            self.carry_mode,
            false,
            false,
            self.botch_rule,
            self.margin_aggregation,
        )
    }

    pub fn evaluate_with_roller<F>(&self, mut roll_die: F) -> CascadeRollOutcome
    where
        F: FnMut(i32) -> i32,
    {
        let mut pairs = Vec::with_capacity(self.skill_level);
        for _ in 0..self.skill_level {
            let a = roll_die(self.sides).clamp(1, self.sides);
            let b = roll_die(self.sides).clamp(1, self.sides);
            pairs.push((a, b));
        }
        self.evaluate_pairs(&pairs)
    }
}

define_coded_enum! {
    /// High-level narrative outcome category of a resolved Two-List Cascade roll.
    ///
    /// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§3.2) and
    /// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.3, §5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum CascadeOutcomeKind [count = 4] {
        Success => { code: "SUCCESS", label: "Success" },
        SuccessWithComplication => { code: "SUCCESS_WITH_COMPLICATION", label: "Success with Complication" },
        CleanFailure => { code: "CLEAN_FAILURE", label: "Clean Failure" },
        BotchFailure => { code: "BOTCH_FAILURE", label: "Botch Failure" },
    }
}

impl CascadeOutcomeKind {
    /// Classifies a roll outcome from `(total_successes, raw_botches, effective_botches)`.
    #[inline]
    pub const fn classify(
        total_successes: i32,
        raw_botches: i32,
        effective_botches: i32,
    ) -> Self {
        if total_successes >= 1 {
            if raw_botches >= 1 {
                Self::SuccessWithComplication
            } else {
                Self::Success
            }
        } else if effective_botches == 0 {
            Self::CleanFailure
        } else {
            Self::BotchFailure
        }
    }

    #[inline]
    pub const fn to_outcome_tier(self, total_successes: i32) -> OutcomeTier {
        match self {
            Self::BotchFailure => OutcomeTier::CriticalFailure,
            Self::CleanFailure => OutcomeTier::Failure,
            Self::SuccessWithComplication => OutcomeTier::PartialSuccess,
            Self::Success => {
                if total_successes >= 3 {
                    OutcomeTier::ExtraordinarySuccess
                } else {
                    OutcomeTier::Success
                }
            }
        }
    }
}

/// Step-by-step trace for a single pair $(A_n, B_n)$ in a resolved Two-List Cascade roll.
///
/// Origin: `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1–§3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeStepTrace {
    pub pair_index: usize,
    pub die_a: i32,
    pub die_b: i32,
    pub raw_sum: i32,
    pub carry_in: i32,
    pub effective_y: i32,
    pub tn_eff: i32,
    pub succeeded: bool,
    pub step_botch: bool,
    pub miss_distance: i32,
    pub carry_out: i32,
}

/// Complete game-engine resolution of a rolled Two-List Cascade check.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§3–§4) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1–§3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeRollOutcome {
    pub steps: Vec<CascadeStepTrace>,
    pub total_successes: i32,
    pub raw_botches: i32,
    pub effective_botches: i32,
    /// Best (minimum) miss tier across all pairs (`0..=5`).
    pub min_miss_tier: i32,
    pub signed_margin: i32,
    pub final_pair_y: i32,
    pub outcome_kind: CascadeOutcomeKind,
    pub outcome_tier: OutcomeTier,
}

/// Evaluates a concrete list of rolled `(A_n, B_n)` dice pairs using the exact Two-List Cascading
/// ("Watershed" / "Recoil") state machine.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§2–§4) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.1–§3.3).
#[allow(clippy::too_many_arguments)]
pub fn evaluate_cascade_roll(
    pairs: &[(i32, i32)],
    sides: i32,
    characteristic_c: i32,
    delta_over: i32,
    delta_under: i32,
    mode: CarryMode,
    count_under_as_success: bool,
    invert_feedback: bool,
    botch_rule: BotchRule,
    margin_aggregation: MarginAggregation,
) -> CascadeRollOutcome {
    if pairs.is_empty() {
        return CascadeRollOutcome {
            steps: Vec::new(),
            total_successes: 0,
            raw_botches: 0,
            effective_botches: 0,
            min_miss_tier: 0,
            signed_margin: 0,
            final_pair_y: 0,
            outcome_kind: CascadeOutcomeKind::CleanFailure,
            outcome_tier: OutcomeTier::Failure,
        };
    }

    let sign = if invert_feedback { -1 } else { 1 };
    let eff_over = sign * delta_over;
    let eff_under = -sign * delta_under;
    let tier_step = (sides / 5).max(2);

    let mut steps = Vec::with_capacity(pairs.len());
    let mut mod_n = 0;
    let mut total_successes = 0;
    let mut raw_botches = 0;
    let mut min_miss_tier = 5;
    let mut final_pair_y = 0;

    for (idx, &(die_a, die_b)) in pairs.iter().enumerate() {
        let raw_sum = die_a + die_b;
        let carry_in = mod_n;
        let yn = raw_sum + carry_in;
        let (sn, bn, mn) = evaluate_step_success_and_botch(
            yn,
            sides,
            characteristic_c,
            count_under_as_success,
            botch_rule,
        );
        total_successes += sn;
        raw_botches += bn;
        min_miss_tier = min_miss_tier.min((mn / tier_step).min(5));
        final_pair_y = yn;

        let next_mod = mode.next_mod(mod_n, yn, characteristic_c, eff_over, eff_under, sign);
        steps.push(CascadeStepTrace {
            pair_index: idx + 1,
            die_a,
            die_b,
            raw_sum,
            carry_in,
            effective_y: yn,
            tn_eff: characteristic_c,
            succeeded: sn == 1,
            step_botch: bn == 1,
            miss_distance: mn,
            carry_out: next_mod,
        });
        mod_n = next_mod;
    }

    let effective_botches =
        botch_rule.effective_botches(total_successes, raw_botches, min_miss_tier);
    let signed_margin = margin_aggregation.compute(total_successes, effective_botches);
    let outcome_kind =
        CascadeOutcomeKind::classify(total_successes, raw_botches, effective_botches);
    let outcome_tier = outcome_kind.to_outcome_tier(total_successes);

    CascadeRollOutcome {
        steps,
        total_successes,
        raw_botches,
        effective_botches,
        min_miss_tier,
        signed_margin,
        final_pair_y,
        outcome_kind,
        outcome_tier,
    }
}

#[inline]
const fn stance_base_tc(combat_stance: Option<CombatStance>) -> i32 {
    match combat_stance {
        Some(CombatStance::Power) => 14,
        Some(CombatStance::Maneuver) => 11,
        Some(CombatStance::Stability) => 9,
        Some(CombatStance::RangedAimed) => 12,
        Some(CombatStance::RangedSnap) => 7,
        None => 10,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_cascade_mechanics_event(
    metadata: EventMetadata,
    actor: Participant,
    target_participants: Vec<Participant>,
    skill_name: String,
    control_c: i32,
    tn_eff: i32,
    dice_rolled_d20: Vec<i32>,
    successes_counted: i32,
    outcome_tier: OutcomeTier,
    combat_stance: Option<CombatStance>,
    fatigue_level: i32,
    wound_shock_index: f32,
    outcome_tc_mult: f32,
    notes: String,
) -> TtrpgMechanicsEvent {
    let base_tc = stance_base_tc(combat_stance);
    let time_factor =
        (1.0 + (fatigue_level as f32 * 0.1) + (wound_shock_index * 0.25)) * outcome_tc_mult;
    let action_count_cost = ((base_tc as f32) * time_factor).round() as i32;

    TtrpgMechanicsEvent {
        metadata,
        mechanic_type: if combat_stance.is_some() {
            MechanicEventType::CombatAction
        } else {
            MechanicEventType::TaskResolution
        },
        actor,
        target_participants,
        skill_name: Some(skill_name),
        attribute_tn: Some(control_c),
        difficulty_rating: Some(tn_eff),
        dice_rolled_d20,
        successes_counted,
        outcome_tier,
        action_time_factor: Some(time_factor),
        action_time_cost: Some(base_tc),
        action_count_cost: Some(action_count_cost),
        combat_stance,
        fatigue_level,
        wound_shock_index,
        notes: Some(notes),
    }
}

/// Evaluates a Two-List Cascading ("Watershed") `2d20` task roll per
/// `docs/ttrpg/01-foundation/probability_and_calibration.md` (§4) and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3), with action tick telemetry per
/// `docs/ttrpg/02-core-mechanics/action_economy_and_ooda.md` (§3.1, §5.1),
/// and returns both the [`TtrpgMechanicsEvent`] telemetry record and the step-by-step [`CascadeRollOutcome`].
#[allow(clippy::too_many_arguments)]
pub fn evaluate_satya_cascade_task(
    metadata: EventMetadata,
    actor: Participant,
    target_participants: Vec<Participant>,
    skill_name: String,
    params: &CascadeTaskParams,
    rolled_pairs: &[(i32, i32)],
    combat_stance: Option<CombatStance>,
    fatigue_level: i32,
    wound_shock_index: f32,
) -> (TtrpgMechanicsEvent, CascadeRollOutcome) {
    let outcome = params.evaluate_pairs(rolled_pairs);
    let dice_rolled_d20: Vec<i32> = rolled_pairs.iter().flat_map(|&(a, b)| [a, b]).collect();
    let event = build_cascade_mechanics_event(
        metadata,
        actor,
        target_participants,
        skill_name,
        params.control_c,
        params.tn_eff(),
        dice_rolled_d20,
        outcome.total_successes,
        outcome.outcome_tier,
        combat_stance,
        fatigue_level,
        wound_shock_index,
        1.0,
        format!(
            "cascade(mode={}, botch={}, tn_eff={}, signed_margin={:+})",
            params.carry_mode,
            params.botch_rule,
            params.tn_eff(),
            outcome.signed_margin
        ),
    );
    (event, outcome)
}

