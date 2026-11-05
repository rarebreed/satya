//! # `satya-py`
//!
//! PyO3 Python extension module (`satya_py`) exposing [`satya_engine`] (exact combinatorial & DP dice
//! probability distributions, Two-List `2d20` Watershed Cascade task resolution, Combat Tactical Pool
//! "Envelope & Fill" calculator, and canonical Rules Book catalog/search) and [`satya_schema`]
//! (Apache Avro Schema Registry, `CRC-64-AVRO` wire framing, and GDPR PII manifest inspection) to Python
//! and Marimo notebooks.

#![deny(missing_docs)]
#![allow(clippy::too_many_arguments)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use satya_engine::{
    BotchRule, CarryMode, CascadeRollOutcome, CascadeStepTrace, CascadeTaskParams,
    CenterDistanceMode, ComplementarySynergyResult, ContestedCascadeOdds,
    ContestedCascadeRollOutcome, ContestedCascadeTaskParams, ContestedTieRule, DistributionStats,
    ExtendedProjectClock, ExtendedProjectStepResult, GmResistanceMode, MarginAggregation,
    MoraleState, Pmf, SignedCascadeResult, TacticalPoolEnvelope, TacticalPoolParams,
    TacticalTriadEvaluation, TaskGateClass, TaskGateResolution, ToolDependency, ToolQualityTier,
    compute_contested_cascade_odds_with_mode,
};
use satya_schema::{
    PiiFieldDescriptor, RulesChapterDocument, RulesChapterSummary, RulesSearchHit, SchemaRegistry,
};
use std::collections::BTreeMap;
use std::str::FromStr;

fn to_py_err(msg: impl Into<String>) -> PyErr {
    PyValueError::new_err(msg.into())
}

/// Summary moments of a discrete probability mass function (`PMF`).
#[pyclass(name = "DistributionStats", get_all)]
#[derive(Debug, Clone)]
pub struct PyDistributionStats {
    /// Expected value $E[X] = \mu$.
    pub mean: f64,
    /// Variance $\text{Var}(X) = \sigma^2$.
    pub variance: f64,
    /// Standard deviation $\sigma = \sqrt{\text{Var}(X)}$.
    pub std: f64,
    /// Alias for standard deviation $\sigma$.
    pub std_dev: f64,
    /// Standardized third central moment (skewness $\gamma_1$).
    pub skewness: f64,
    /// Standardized fourth central moment minus 3 (excess kurtosis $\gamma_2$).
    pub excess_kurtosis: f64,
}

impl From<DistributionStats> for PyDistributionStats {
    fn from(s: DistributionStats) -> Self {
        Self {
            mean: s.mean,
            variance: s.variance,
            std: s.std,
            std_dev: s.std,
            skewness: s.skewness,
            excess_kurtosis: s.excess_kurtosis,
        }
    }
}

#[pymethods]
impl PyDistributionStats {
    fn __repr__(&self) -> String {
        format!(
            "DistributionStats(mean={:.4}, variance={:.4}, std={:.4}, skewness={:.4}, excess_kurtosis={:.4})",
            self.mean, self.variance, self.std, self.skewness, self.excess_kurtosis
        )
    }
}

/// Complete PMF and summary probabilities for a signed Two-List Cascading ("Watershed") roll.
#[pyclass(name = "SignedCascadeResult", get_all)]
#[derive(Debug, Clone)]
pub struct PySignedCascadeResult {
    /// Unified signed outcome PMF over `-n_pairs..=+n_pairs`.
    pub signed_margin_pmf: Pmf,
    /// Positive success count PMF (`0..=n_pairs`).
    pub success_count_pmf: Pmf,
    /// Botch count PMF (`0..=n_pairs`).
    pub botch_count_pmf: Pmf,
    /// Final step modified roll $Y_N$ PMF.
    pub final_pair_value_pmf: Pmf,
    /// Conditional overshoot PMF given zero successes.
    pub failure_overshoot_pmf: Pmf,
    /// Probability of scoring at least 1 success (`S >= 1`).
    pub prob_any_success: f64,
    /// Probability of a clean failure (`S == 0` and `B_eff == 0`).
    pub prob_clean_failure: f64,
    /// Probability of a botch failure (`S == 0` and `B_eff >= 1`).
    pub prob_botch_failure: f64,
    /// Probability of succeeding with a complication (`S >= 1` and `B_raw >= 1`).
    pub prob_success_with_complication: f64,
}

impl From<SignedCascadeResult> for PySignedCascadeResult {
    fn from(r: SignedCascadeResult) -> Self {
        Self {
            signed_margin_pmf: r.signed_margin_pmf,
            success_count_pmf: r.success_count_pmf,
            botch_count_pmf: r.botch_count_pmf,
            final_pair_value_pmf: r.final_pair_value_pmf,
            failure_overshoot_pmf: r.failure_overshoot_pmf,
            prob_any_success: r.prob_any_success,
            prob_clean_failure: r.prob_clean_failure,
            prob_botch_failure: r.prob_botch_failure,
            prob_success_with_complication: r.prob_success_with_complication,
        }
    }
}

/// Trace of a single sequential pair $n \in \{1, \dots, N\}$ in a Satya `2d20` cascade roll.
#[pyclass(name = "CascadeStepTrace", get_all)]
#[derive(Debug, Clone)]
pub struct PyCascadeStepTrace {
    /// 1-based pair index (`1..=N`).
    pub pair_index: usize,
    /// First die roll $A_n \in [1, K]$.
    pub die_a: i32,
    /// Second die roll $B_n \in [1, K]$.
    pub die_b: i32,
    /// Unmodified pair sum $A_n + B_n$.
    pub raw_sum: i32,
    /// Carry-over modifier entering step $n$.
    pub carry_in: i32,
    /// Effective pair total $Y_n = A_n + B_n + \text{carry\_in}$.
    pub effective_y: i32,
    /// Effective Roll-Over Target Number $\text{TN}_{\text{eff}}$ compared against $Y_n$.
    pub tn_eff: i32,
    /// Whether step $n$ achieved a success ($Y_n > \text{TN}_{\text{eff}}$).
    pub succeeded: bool,
    /// Whether step $n$ breached the physical floor ($Y_n < 2$).
    pub step_botch: bool,
    /// Non-negative miss distance from $\text{TN}_{\text{eff}}$.
    pub miss_distance: i32,
    /// Carry-over modifier passed to step $n + 1$.
    pub carry_out: i32,
}

impl From<CascadeStepTrace> for PyCascadeStepTrace {
    fn from(s: CascadeStepTrace) -> Self {
        Self {
            pair_index: s.pair_index,
            die_a: s.die_a,
            die_b: s.die_b,
            raw_sum: s.raw_sum,
            carry_in: s.carry_in,
            effective_y: s.effective_y,
            tn_eff: s.tn_eff,
            succeeded: s.succeeded,
            step_botch: s.step_botch,
            miss_distance: s.miss_distance,
            carry_out: s.carry_out,
        }
    }
}

/// Complete step-by-step evaluation result of a concrete Satya `2d20` cascade roll.
#[pyclass(name = "CascadeRollOutcome", get_all)]
#[derive(Debug, Clone)]
pub struct PyCascadeRollOutcome {
    /// Per-pair trace across all $N$ rolled pairs.
    pub steps: Vec<PyCascadeStepTrace>,
    /// Total successes scored across all $N$ pairs (`0..=N`).
    pub total_successes: i32,
    /// Raw floor breaches ($Y_n < 2$) across all $N$ pairs.
    pub raw_botches: i32,
    /// Effective botch severity (`0..=N`) under the active botch rule when `total_successes == 0`.
    pub effective_botches: i32,
    /// Closest-miss tier (`0..=5`) below $\text{TN}_{\text{eff}}$.
    pub min_miss_tier: i32,
    /// Unified signed outcome margin in `-N..=+N`.
    pub signed_margin: i32,
    /// Effective value $Y_N$ of the final pair in the chain.
    pub final_pair_y: i32,
    /// High-level cascade outcome classification (`"SUCCESS"`, `"SUCCESS_WITH_COMPLICATION"`, `"CLEAN_FAILURE"`, `"BOTCH_FAILURE"`).
    pub outcome_kind: String,
    /// Canonical Avro `OutcomeTier` string (`"EXTRAORDINARY_SUCCESS"`, `"SUCCESS"`, `"PARTIAL_SUCCESS"`, `"FAILURE"`, `"CRITICAL_FAILURE"`).
    pub outcome_tier: String,
}

impl From<CascadeRollOutcome> for PyCascadeRollOutcome {
    fn from(o: CascadeRollOutcome) -> Self {
        let outcome_tier = serde_json::to_value(o.outcome_tier)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| format!("{:?}", o.outcome_tier));
        let outcome_kind = serde_json::to_value(o.outcome_kind)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| format!("{:?}", o.outcome_kind));
        Self {
            steps: o.steps.into_iter().map(PyCascadeStepTrace::from).collect(),
            total_successes: o.total_successes,
            raw_botches: o.raw_botches,
            effective_botches: o.effective_botches,
            min_miss_tier: o.min_miss_tier,
            signed_margin: o.signed_margin,
            final_pair_y: o.final_pair_y,
            outcome_kind,
            outcome_tier,
        }
    }
}

/// Canonical Four-Factor Satya `2d20` Cascading Task configuration (`Skill N`, `Control C`, `Internal Mod`, `External Mod`).
#[pyclass(name = "CascadeTaskParams")]
#[derive(Debug, Clone)]
pub struct PyCascadeTaskParams {
    inner: CascadeTaskParams,
}

#[pymethods]
impl PyCascadeTaskParams {
    /// Creates a new `K = 20` (`2d20`) Satya task parameter set with $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$.
    #[new]
    #[pyo3(signature = (skill_level=3, control_c=10, internal_mod=0, external_mod=0, carry_mode="markov_step", botch_rule="closest_miss"))]
    pub fn new(
        skill_level: usize,
        control_c: i32,
        internal_mod: i32,
        external_mod: i32,
        carry_mode: &str,
        botch_rule: &str,
    ) -> PyResult<Self> {
        let cm = CarryMode::from_str(carry_mode).map_err(to_py_err)?;
        let br = BotchRule::from_str(botch_rule).map_err(to_py_err)?;
        let mut inner =
            CascadeTaskParams::new_k20(skill_level, control_c, internal_mod, external_mod);
        inner.carry_mode = cm;
        inner.botch_rule = br;
        Ok(Self { inner })
    }

    /// Skill Level $N$ (number of `2d20` pairs rolled).
    #[getter]
    pub fn skill_level(&self) -> usize {
        self.inner.skill_level
    }

    /// Controlling Characteristic $C \in [1, 20]$.
    #[getter]
    pub fn control_c(&self) -> i32 {
        self.inner.control_c
    }

    /// Internal modifier $\Delta_{\text{int}}$ (fatigue, wound shock, strain).
    #[getter]
    pub fn internal_mod(&self) -> i32 {
        self.inner.internal_mod
    }

    /// External modifier $\Delta_{\text{ext}}$ (environmental difficulty, range, tools).
    #[getter]
    pub fn external_mod(&self) -> i32 {
        self.inner.external_mod
    }

    /// Effective Roll-Over Target Number $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$.
    pub fn tn_eff(&self) -> i32 {
        self.inner.tn_eff()
    }

    /// Effective negative carry penalty $\delta_{\text{under}} = 6 + \Delta_{\text{int}}$.
    pub fn effective_delta_under(&self) -> i32 {
        self.inner.effective_delta_under()
    }

    /// Computes the exact analytical PMF and odds for this task configuration.
    pub fn exact_odds(&self) -> PySignedCascadeResult {
        PySignedCascadeResult::from(self.inner.exact_odds())
    }

    /// Evaluates a concrete list of rolled `(die_a, die_b)` `d20` pairs against this task.
    pub fn evaluate_pairs(&self, pairs: Vec<(i32, i32)>) -> PyCascadeRollOutcome {
        PyCascadeRollOutcome::from(self.inner.evaluate_pairs(&pairs))
    }
}

/// Exact combinatorial probability breakdown and net margin PMF for a Contested Cascade check.
#[pyclass(name = "ContestedCascadeOdds", get_all)]
#[derive(Debug, Clone)]
pub struct PyContestedCascadeOdds {
    /// Exact PMF of the net signed margin $\Delta M = M_P - M_{GM}$.
    pub net_signed_margin_pmf: Pmf,
    /// Exact PMF of the net success count $\Delta S = S_P - S_{GM}$.
    pub net_success_pmf: Pmf,
    /// Total probability of achieving any success tier (`ExtraordinarySuccess + CleanSuccess + PartialSuccess`).
    pub prob_total_win: f64,
    /// Probability of an unblemished win ($S_P > S_{GM}$ without complication).
    pub prob_clean_or_extraordinary_win: f64,
    /// Probability of `OutcomeTier::ExtraordinarySuccess`.
    pub prob_extraordinary_success: f64,
    /// Probability of standard `OutcomeTier::Success`.
    pub prob_clean_success: f64,
    /// Total probability of `OutcomeTier::PartialSuccess`.
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
    /// Summary moments of `net_signed_margin_pmf`.
    pub net_margin_stats: PyDistributionStats,
    /// Sarle's Bimodality Coefficient $\text{BC} = (\gamma_1^2 + 1) / (\kappa_2 + 3)$.
    pub bimodality_coefficient: f64,
}

impl From<ContestedCascadeOdds> for PyContestedCascadeOdds {
    fn from(o: ContestedCascadeOdds) -> Self {
        Self {
            net_signed_margin_pmf: o.net_signed_margin_pmf,
            net_success_pmf: o.net_success_pmf,
            prob_total_win: o.prob_total_win,
            prob_clean_or_extraordinary_win: o.prob_clean_or_extraordinary_win,
            prob_extraordinary_success: o.prob_extraordinary_success,
            prob_clean_success: o.prob_clean_success,
            prob_partial_success: o.prob_partial_success,
            prob_floor_breach_partial: o.prob_floor_breach_partial,
            prob_grazing_tie_partial: o.prob_grazing_tie_partial,
            prob_lucky_break_partial: o.prob_lucky_break_partial,
            prob_clean_failure: o.prob_clean_failure,
            prob_botch_failure: o.prob_botch_failure,
            net_margin_stats: PyDistributionStats::from(o.net_margin_stats),
            bimodality_coefficient: o.bimodality_coefficient,
        }
    }
}

/// Complete live roll outcome for a Contested Cascade task check.
#[pyclass(name = "ContestedCascadeRollOutcome", get_all)]
#[derive(Debug, Clone)]
pub struct PyContestedCascadeRollOutcome {
    /// Step-by-step cascade outcome for the Player.
    pub player_outcome: PyCascadeRollOutcome,
    /// Step-by-step cascade outcome for the GM / Opponent.
    pub gm_outcome: PyCascadeRollOutcome,
    /// Difference in raw positive successes ($S_P - S_{GM}$).
    pub net_successes: i32,
    /// Difference in unified signed margins ($M_P - M_{GM}$).
    pub net_signed_margin: i32,
    /// Detailed 7-way contested outcome classification (`SCREAMING_SNAKE_CASE`).
    pub contested_kind: String,
    /// Canonical 5-tier `OutcomeTier` (`SCREAMING_SNAKE_CASE`).
    pub outcome_tier: String,
    /// Task duration / Tick Cost (`TC`) multiplier per §5 of `cascading_d20_and_tasks.md`.
    pub duration_multiplier: f64,
    /// External difficulty modifier ($\Delta_{\text{ext}}$) granted to the next linked action (`-4`, `0`, or `+4`).
    pub next_linked_action_delta_ext: i32,
    /// Physical or mental Strain cost inflicted by a Grazing Tie (`1` on `GRAZING_TIE_SUCCESS`, `0` otherwise).
    pub grazing_strain_cost: i32,
}

impl From<ContestedCascadeRollOutcome> for PyContestedCascadeRollOutcome {
    fn from(o: ContestedCascadeRollOutcome) -> Self {
        let duration_multiplier = o.contested_kind.duration_multiplier(o.net_successes);
        let next_linked_action_delta_ext = o.contested_kind.next_linked_action_delta_ext();
        let grazing_strain_cost = o.contested_kind.grazing_strain_cost();
        Self {
            player_outcome: PyCascadeRollOutcome::from(o.player_outcome),
            gm_outcome: PyCascadeRollOutcome::from(o.gm_outcome),
            net_successes: o.net_successes,
            net_signed_margin: o.net_signed_margin,
            contested_kind: o.contested_kind.as_str().to_string(),
            outcome_tier: format!("{:?}", o.outcome_tier).to_uppercase(),
            duration_multiplier,
            next_linked_action_delta_ext,
            grazing_strain_cost,
        }
    }
}

/// Always-Contested Two-List Cascade task configuration (Player vs. 1-Pair Tiered GM Resistance or Full Cascade NPC).
#[pyclass(name = "ContestedCascadeTaskParams")]
#[derive(Debug, Clone)]
pub struct PyContestedCascadeTaskParams {
    inner: ContestedCascadeTaskParams,
}

#[pymethods]
impl PyContestedCascadeTaskParams {
    /// Creates a Contested Cascade task configuration (`1-Pair Tiered GM Resistance` by default).
    #[new]
    #[pyo3(signature = (
        player_skill_n,
        player_control_c,
        player_internal_mod=0,
        player_known_tool_mod=0,
        hidden_external_difficulty=0,
        gm_mode="tiered_single_pair",
        gm_pairs=1,
        tie_rule="hybrid_signed_margin",
        delta_over=6,
    ))]
    pub fn new(
        player_skill_n: usize,
        player_control_c: i32,
        player_internal_mod: i32,
        player_known_tool_mod: i32,
        hidden_external_difficulty: i32,
        gm_mode: &str,
        gm_pairs: usize,
        tie_rule: &str,
        delta_over: i32,
    ) -> Self {
        let parsed_tie = match tie_rule.trim().to_ascii_lowercase().as_str() {
            "strict_signed_margin" | "strict" => ContestedTieRule::StrictSignedMargin,
            "active_player_wins_ties" | "active" => ContestedTieRule::ActivePlayerWinsTies,
            _ => ContestedTieRule::HybridSignedMargin,
        };
        let mut inner = if gm_mode.trim().eq_ignore_ascii_case("full_cascade") {
            ContestedCascadeTaskParams::new_hybrid_environment(
                player_skill_n.clamp(1, 10),
                player_control_c.clamp(1, 20),
                player_internal_mod,
                player_known_tool_mod,
                gm_pairs.clamp(1, 10),
                hidden_external_difficulty,
                parsed_tie,
            )
        } else {
            ContestedCascadeTaskParams::new_tiered_environment(
                player_skill_n.clamp(1, 10),
                player_control_c.clamp(1, 20),
                player_internal_mod,
                player_known_tool_mod,
                hidden_external_difficulty,
                parsed_tie,
            )
        };
        inner.player.delta_over = delta_over;
        Self { inner }
    }

    /// Loads one of the 6 canonical worked task presets (`"sarah_lockpick"`, `"marcus_surgery"`, `"elena_hack"`, etc.).
    #[staticmethod]
    pub fn from_preset(preset_id: &str) -> Self {
        let p = satya_engine::get_canonical_task_preset(preset_id);
        Self {
            inner: p.to_contested_params(),
        }
    }

    /// Player's known personal target number $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$.
    pub fn tn_player(&self) -> i32 {
        self.inner.player.tn_eff()
    }

    /// GM's hidden resistance target number $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$.
    pub fn tn_gm(&self) -> i32 {
        self.inner.gm.tn_eff()
    }

    /// Computes the exact combinatorial [`PyContestedCascadeOdds`] for this contested task.
    pub fn exact_contested_odds(&self) -> PyContestedCascadeOdds {
        PyContestedCascadeOdds::from(self.inner.exact_contested_odds())
    }

    /// Evaluates concrete `(die_a, die_b)` `2d20` pairs for both the Player and the GM.
    pub fn evaluate_contested_pairs(
        &self,
        player_pairs: Vec<(i32, i32)>,
        gm_pairs: Vec<(i32, i32)>,
    ) -> PyContestedCascadeRollOutcome {
        PyContestedCascadeRollOutcome::from(
            self.inner.evaluate_contested_pairs(&player_pairs, &gm_pairs),
        )
    }
}

/// Resolved eligibility, effective pair pool $N$, known $\Delta_{\text{tool}}$, and hidden $\Delta_{\text{ext}}$ gate penalty.
#[pyclass(name = "TaskGateResolution", get_all)]
#[derive(Debug, Clone)]
pub struct PyTaskGateResolution {
    /// Whether the character is permitted to roll the task (`false` means `[Locked]`).
    pub allowed: bool,
    /// Effective number of `2d20` cascade pairs $N$ to roll (`0` if `[Locked]`).
    pub effective_skill_n: usize,
    /// Additional external difficulty penalty ($\Delta_{\text{ext}}$) from untrained or sibling defaulting.
    pub gate_delta_ext: i32,
    /// Known player tool modifier ($\Delta_{\text{tool}}$) shifting $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$.
    pub tool_delta_mod: i32,
    /// Whether any Floor Breach ($Y_n < 2$) automatically inflicts 1 step of equipment wear/degradation.
    pub improvised_wear_on_floor_breach: bool,
    /// Whether the character used Talent (`Class A`) or sibling-skill (`Class B`) defaulting.
    pub used_defaulting: bool,
    /// Human-readable reason if `allowed == false` (`[Locked]`).
    pub lock_reason: Option<String>,
}

impl From<TaskGateResolution> for PyTaskGateResolution {
    fn from(r: TaskGateResolution) -> Self {
        Self {
            allowed: r.allowed,
            effective_skill_n: r.effective_skill_n,
            gate_delta_ext: r.gate_delta_ext,
            tool_delta_mod: r.tool_delta_mod,
            improvised_wear_on_floor_breach: r.improvised_wear_on_floor_breach,
            used_defaulting: r.used_defaulting,
            lock_reason: r.lock_reason,
        }
    }
}

/// Mechanical synergy transferred from a preliminary analytical (`KNO` / `LOG`) setup roll
/// to a subsequent physical or technical execution roll per §6.1 of `cascading_d20_and_tasks.md`.
#[pyclass(name = "ComplementarySynergyResult", get_all)]
#[derive(Debug, Clone)]
pub struct PyComplementarySynergyResult {
    /// External difficulty modifier ($\Delta_{\text{ext}}$) applied to the execution task (`-4`, `-2`, `0`, or `+4`).
    pub delta_ext_mod: i32,
    /// Over-target step carry bonus $\delta_{\text{over}}$ for the execution task (`8` on `ExtraordinarySuccess`, `6` otherwise).
    pub delta_over_carry: i32,
    /// Canonical description of the setup synergy tier.
    pub summary: String,
}

impl From<ComplementarySynergyResult> for PyComplementarySynergyResult {
    fn from(s: ComplementarySynergyResult) -> Self {
        Self {
            delta_ext_mod: s.delta_ext_mod,
            delta_over_carry: s.delta_over_carry,
            summary: s.summary.to_string(),
        }
    }
}

/// Result of applying a single interval roll to an [`PyExtendedProjectClock`].
#[pyclass(name = "ExtendedProjectStepResult", get_all)]
#[derive(Debug, Clone)]
pub struct PyExtendedProjectStepResult {
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

impl From<ExtendedProjectStepResult> for PyExtendedProjectStepResult {
    fn from(r: ExtendedProjectStepResult) -> Self {
        Self {
            progress_delta: r.progress_delta,
            accumulated_successes: r.accumulated_successes,
            target_net_successes: r.target_net_successes,
            intervals_rolled: r.intervals_rolled,
            heat_generated: r.heat_generated,
            extra_material_pct: r.extra_material_pct,
            is_completed: r.is_completed,
        }
    }
}

/// Extended Project Progress Clock per §6.3 of `cascading_d20_and_tasks.md`.
#[pyclass(name = "ExtendedProjectClock")]
#[derive(Debug, Clone)]
pub struct PyExtendedProjectClock {
    inner: ExtendedProjectClock,
}

#[pymethods]
impl PyExtendedProjectClock {
    /// Creates a new [`PyExtendedProjectClock`] with `target_net_successes` (`10`, `20`, or `40`).
    #[new]
    pub fn new(project_name: String, target_net_successes: i32) -> Self {
        Self {
            inner: ExtendedProjectClock::new(project_name, target_net_successes),
        }
    }

    /// Returns `true` if `accumulated_successes >= target_net_successes`.
    pub fn is_completed(&self) -> bool {
        self.inner.is_completed()
    }

    /// Current accumulated net successes (`>= 0`).
    #[getter]
    pub fn accumulated_successes(&self) -> i32 {
        self.inner.accumulated_successes
    }

    /// Applies a single interval roll to the extended project clock.
    #[pyo3(signature = (positive_successes, signed_margin, had_floor_breach_complication=false, is_botch=false))]
    pub fn apply_interval(
        &mut self,
        positive_successes: i32,
        signed_margin: i32,
        had_floor_breach_complication: bool,
        is_botch: bool,
    ) -> PyExtendedProjectStepResult {
        PyExtendedProjectStepResult::from(self.inner.apply_interval(
            positive_successes,
            signed_margin,
            had_floor_breach_complication,
            is_botch,
        ))
    }
}

/// Computed Combat Tactical Pool ("Fill") and Characteristic Aspect Ceilings ("Envelope").
#[pyclass(name = "TacticalPoolEnvelope")]
#[derive(Debug, Clone)]
pub struct PyTacticalPoolEnvelope {
    /// Cognitive Decision Score ("Quick Thinking"): $D_{\text{cog}} = \lfloor(\max(\text{ANA}, \text{CRE}) + \text{FOC}) / 2\rfloor$.
    #[pyo3(get)]
    pub d_cog: i32,
    /// 4-Pillar OODA Initiative rating: $\lfloor(\text{ITU} + \text{INS} + D_{\text{cog}} + \text{SPD}) / 4\rfloor$.
    #[pyo3(get)]
    pub ooda_init: i32,
    /// Combat Readiness controller: $C_{\text{combat}} = \lfloor(C_{\text{skill}} + \text{OODA}) / 2\rfloor$.
    #[pyo3(get)]
    pub combat_readiness_c: i32,
    /// Base Tactical Pool before fatigue, shock, or morale: $P_{\text{base}} = C_{\text{combat}} + 3N$.
    #[pyo3(get)]
    pub p_base: i32,
    /// Effective Tactical Pool available to allocate: $P_{\text{eff}}$.
    #[pyo3(get)]
    pub p_eff: i32,
    /// Neutral per-aspect baseline: $B = \lfloor P_{\text{base}} / 3 \rfloor$.
    #[pyo3(get)]
    pub neutral_baseline_b: i32,
    /// Maximum points allocatable to **Tempo ($T_{\max}$)**.
    #[pyo3(get)]
    pub tempo_cap: i32,
    /// Maximum points allocatable to **Control ($C_{\max}$)**.
    #[pyo3(get)]
    pub control_cap: i32,
    /// Maximum points allocatable to **Force ($F_{\max}$)**.
    #[pyo3(get)]
    pub force_cap: i32,
    /// Mastery technique overflow bonus ($\lfloor(N - 3)/2\rfloor$ for $N \ge 5$).
    #[pyo3(get)]
    pub mastery_overflow: i32,
    /// Derived kinetic Power: $\text{POW} = (\text{FOR} \times \text{SPD}) / 10.0$.
    #[pyo3(get)]
    pub power_pow: f64,
    /// Derived Momentum & Mass Factor: $\text{MMF} = (\text{MAS} / 70.0) \times \text{SPD}$.
    #[pyo3(get)]
    pub momentum_mass_factor_mmf: f64,
    /// Derived Fatigue Threshold: $\text{FT} = \text{FIT} + \lfloor\text{DIS} / 2\rfloor$.
    #[pyo3(get)]
    pub fatigue_threshold_ft: i32,
    /// Action Time Factor multiplier: $1.0 + 0.10\cdot\text{Fatigue} + 0.25\cdot\text{Shock}$.
    #[pyo3(get)]
    pub action_time_factor: f64,
    /// Active psychological Morale Condition name.
    #[pyo3(get)]
    pub morale_state: String,
    /// Minimum Force allocation required by Morale/Rage.
    #[pyo3(get)]
    pub min_force_required: i32,
    /// Maximum Control allocation permitted by Morale/Rage + Ceiling.
    #[pyo3(get)]
    pub max_control_allowed: i32,
    /// Maximum Force allocation permitted by Morale/Rage + Ceiling.
    #[pyo3(get)]
    pub max_force_allowed: i32,
    /// Whether `Force` is constrained to not exceed `Control` ($F \le C$).
    #[pyo3(get)]
    pub force_must_not_exceed_control: bool,
    inner: TacticalPoolEnvelope,
}

impl From<TacticalPoolEnvelope> for PyTacticalPoolEnvelope {
    fn from(e: TacticalPoolEnvelope) -> Self {
        Self {
            d_cog: e.d_cog,
            ooda_init: e.ooda_init,
            combat_readiness_c: e.combat_readiness_c,
            p_base: e.p_base,
            p_eff: e.p_eff,
            neutral_baseline_b: e.neutral_baseline_b,
            tempo_cap: e.tempo_cap,
            control_cap: e.control_cap,
            force_cap: e.force_cap,
            mastery_overflow: e.mastery_overflow,
            power_pow: e.power_pow,
            momentum_mass_factor_mmf: e.momentum_mass_factor_mmf,
            fatigue_threshold_ft: e.fatigue_threshold_ft,
            action_time_factor: e.action_time_factor,
            morale_state: e.morale_state.as_str().to_string(),
            min_force_required: e.min_force_required,
            max_control_allowed: e.max_control_allowed,
            max_force_allowed: e.max_force_allowed,
            force_must_not_exceed_control: e.force_must_not_exceed_control,
            inner: e,
        }
    }
}

#[pymethods]
impl PyTacticalPoolEnvelope {
    /// Evaluates a `(tempo, control, force)` allocation against this envelope.
    pub fn evaluate_triad(
        &self,
        base_weapon_tc: i32,
        tempo: i32,
        control: i32,
        force: i32,
    ) -> PyTacticalTriadEvaluation {
        PyTacticalTriadEvaluation::from(satya_engine::evaluate_tactical_triad(
            &self.inner,
            base_weapon_tc,
            tempo,
            control,
            force,
        ))
    }
}

/// Evaluation of a specific `Tempo + Control + Force` Tactical Triad allocation.
#[pyclass(name = "TacticalTriadEvaluation", get_all)]
#[derive(Debug, Clone)]
pub struct PyTacticalTriadEvaluation {
    /// Points allocated to **Tempo (`T`)**.
    pub tempo: i32,
    /// Points allocated to **Control (`C`)**.
    pub control: i32,
    /// Points allocated to **Force (`F`)**.
    pub force: i32,
    /// Whether the allocation satisfies all Pool, Ceiling, and Morale/Rage constraints.
    pub valid: bool,
    /// Human-readable descriptions of any violated constraints.
    pub violations: Vec<String>,
    /// Effective Count-Up Action Tick Cost (`TC`).
    pub effective_tc: i32,
    /// Scaled melee Strike Impact ($\text{POW} \times (F / B)$).
    pub strike_impact: f64,
    /// Effective Rooted stability against throws, sweeps, and knockback ($F + \text{MMF}$).
    pub effective_root: f64,
    /// Exertion marks accumulated by this action (`0`, `1`, or `2`).
    pub exertion_marks_gained: i32,
    /// Canonical Tactical Archetype classification.
    pub archetype_label: String,
}

impl From<TacticalTriadEvaluation> for PyTacticalTriadEvaluation {
    fn from(e: TacticalTriadEvaluation) -> Self {
        Self {
            tempo: e.tempo,
            control: e.control,
            force: e.force,
            valid: e.valid,
            violations: e.violations,
            effective_tc: e.effective_tc,
            strike_impact: e.strike_impact,
            effective_root: e.effective_root,
            exertion_marks_gained: e.exertion_marks_gained,
            archetype_label: e.archetype_label,
        }
    }
}

/// Metadata summary for a canonical chapter in the Satya TTRPG Rules Book.
#[pyclass(name = "RulesChapterSummary", get_all)]
#[derive(Debug, Clone)]
pub struct PyRulesChapterSummary {
    /// Canonical chapter slug identifier.
    pub chapter_id: String,
    /// Book part/section grouping label.
    pub part: String,
    /// Human-readable chapter title.
    pub title: String,
    /// Relative path inside `docs/ttrpg/`.
    pub relative_path: String,
    /// Concise summary of the rules covered in this chapter.
    pub description: String,
    /// Top-level `##` and `###` section headings.
    pub headings: Vec<String>,
}

impl From<RulesChapterSummary> for PyRulesChapterSummary {
    fn from(s: RulesChapterSummary) -> Self {
        Self {
            chapter_id: s.chapter_id,
            part: s.part,
            title: s.title,
            relative_path: s.relative_path,
            description: s.description,
            headings: s.headings,
        }
    }
}

/// Full Satya TTRPG Rules Book chapter document including Markdown and pre-rendered HTML.
#[pyclass(name = "RulesChapterDocument", get_all)]
#[derive(Debug, Clone)]
pub struct PyRulesChapterDocument {
    /// Canonical chapter slug identifier.
    pub chapter_id: String,
    /// Book part/section grouping label.
    pub part: String,
    /// Human-readable chapter title.
    pub title: String,
    /// Relative path inside `docs/ttrpg/`.
    pub relative_path: String,
    /// Concise summary of the rules covered in this chapter.
    pub description: String,
    /// Top-level `##` and `###` section headings.
    pub headings: Vec<String>,
    /// Raw canonical Markdown source of the chapter.
    pub markdown: String,
    /// Pre-rendered safe HTML representation of the chapter.
    pub rendered_html: String,
}

impl From<RulesChapterDocument> for PyRulesChapterDocument {
    fn from(d: RulesChapterDocument) -> Self {
        Self {
            chapter_id: d.summary.chapter_id,
            part: d.summary.part,
            title: d.summary.title,
            relative_path: d.summary.relative_path,
            description: d.summary.description,
            headings: d.summary.headings,
            markdown: d.markdown,
            rendered_html: d.rendered_html,
        }
    }
}

/// Ranked section match returned when searching across the Satya TTRPG Rules Book.
#[pyclass(name = "RulesSearchHit", get_all)]
#[derive(Debug, Clone)]
pub struct PyRulesSearchHit {
    /// Canonical chapter slug identifier where the match occurred.
    pub chapter_id: String,
    /// Human-readable title of the matching chapter.
    pub chapter_title: String,
    /// Relative file path inside `docs/ttrpg/`.
    pub relative_path: String,
    /// Heading of the specific section that matched the query.
    pub section_heading: String,
    /// Markdown excerpt of the matching section.
    pub snippet: String,
    /// Relevance score computed from keyword and heading hits.
    pub score: usize,
}

impl From<RulesSearchHit> for PyRulesSearchHit {
    fn from(h: RulesSearchHit) -> Self {
        Self {
            chapter_id: h.chapter_id,
            chapter_title: h.chapter_title,
            relative_path: h.relative_path,
            section_heading: h.section_heading,
            snippet: h.snippet,
            score: h.score,
        }
    }
}

/// Python wrapper around [`satya_schema::SchemaRegistry`] providing `CRC-64-AVRO` fingerprinting,
/// 15-byte binary wire framing, and schema-driven GDPR PII manifest inspection.
#[pyclass(name = "SchemaRegistry")]
pub struct PySchemaRegistry {
    inner: SchemaRegistry,
}

#[pymethods]
impl PySchemaRegistry {
    /// Initializes a [`SchemaRegistry`] populated with all built-in Satya `.avsc` schemas.
    #[new]
    pub fn new() -> PyResult<Self> {
        let inner =
            SchemaRegistry::new_with_builtin_schemas().map_err(|e| to_py_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Returns a list of `(subject, version, fingerprint_hex)` for all registered schemas.
    pub fn list_schemas(&self) -> Vec<(String, u16, String)> {
        let mut out: Vec<(String, u16, String)> = self
            .inner
            .all_schemas()
            .into_iter()
            .map(|s| (s.subject.clone(), s.version, s.fingerprint_hex()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        out
    }

    /// Extracts all custom `satya.*` PII/GDPR field annotations across registered schemas as a JSON string.
    pub fn extract_pii_manifest(&self) -> PyResult<String> {
        let mut all_specs: BTreeMap<String, Vec<PiiFieldDescriptor>> = BTreeMap::new();
        for reg in self.inner.all_schemas() {
            let specs = reg.extract_pii_field_manifest();
            if !specs.is_empty() {
                all_specs.insert(format!("{}@v{}", reg.subject, reg.version), specs);
            }
        }
        serde_json::to_string_pretty(&all_specs).map_err(|e| to_py_err(e.to_string()))
    }
}

// ============================================================================
// Module-level Functions
// ============================================================================

/// Normalizes a map of outcome weights so that all probabilities sum to `1.0`.
#[pyfunction]
#[pyo3(signature = (pmf))]
pub fn normalize_pmf(pmf: Pmf) -> Pmf {
    satya_engine::normalize_pmf(&pmf)
}

/// Computes exact mean, variance, standard deviation, skewness, and excess kurtosis of a PMF.
#[pyfunction]
#[pyo3(signature = (pmf))]
pub fn compute_stats(pmf: Pmf) -> PyDistributionStats {
    PyDistributionStats::from(satya_engine::compute_stats(&pmf))
}

/// Computes the discrete convolution $P(Z = x + y)$ of two independent PMFs.
#[pyfunction]
#[pyo3(signature = (pmf_a, pmf_b))]
pub fn convolve_pmfs(pmf_a: Pmf, pmf_b: Pmf) -> Pmf {
    satya_engine::convolve_pmfs(&pmf_a, &pmf_b)
}

/// Uniform PMF for a single `sides`-sided die ($1 \dots \text{sides}$, or $-\text{sides} \dots -1$ if `negative=True`).
#[pyfunction]
#[pyo3(signature = (sides, negative=false))]
pub fn single_die_pmf(sides: i32, negative: bool) -> Pmf {
    satya_engine::single_die_pmf(sides, negative)
}

/// Exact PMF for the sum of `pos_dice` positive `dK` dice and `neg_dice` negative `dK` dice.
#[pyfunction]
#[pyo3(signature = (pos_dice, sides, neg_dice=0))]
pub fn sum_dice_pool_pmf(pos_dice: usize, sides: i32, neg_dice: usize) -> Pmf {
    satya_engine::sum_dice_pool_pmf(pos_dice, sides, neg_dice)
}

/// Exact PMF for counting successes in an $N\text{d}K$ pool.
#[pyfunction]
#[pyo3(signature = (pool_size, sides, target_num, roll_under=true, crit_on_one_double=false))]
pub fn count_successes_pmf(
    pool_size: usize,
    sides: i32,
    target_num: i32,
    roll_under: bool,
    crit_on_one_double: bool,
) -> Pmf {
    satya_engine::count_successes_pmf(pool_size, sides, target_num, roll_under, crit_on_one_double)
}

/// Exact PMF for a single exploding die (`dK`) up to `max_explosions`.
#[pyfunction]
#[pyo3(signature = (sides, explode_min=None, max_explosions=4))]
pub fn single_exploding_die_pmf(
    sides: i32,
    explode_min: Option<i32>,
    max_explosions: usize,
) -> Pmf {
    satya_engine::single_exploding_die_pmf(sides, explode_min, max_explosions)
}

/// Exact PMF for the maximum of `pool_size` exploding `dK` dice (Gumbel extreme-value distribution).
#[pyfunction]
#[pyo3(signature = (pool_size, sides, explode_min=None, max_explosions=4))]
pub fn exploding_pool_keep_highest_pmf(
    pool_size: usize,
    sides: i32,
    explode_min: Option<i32>,
    max_explosions: usize,
) -> Pmf {
    satya_engine::exploding_pool_keep_highest_pmf(pool_size, sides, explode_min, max_explosions)
}

/// Exact PMF of the `k_rank_from_lowest`-th order statistic ($X_{(k)}$, 1-indexed from lowest) of $N\text{d}K$.
#[pyfunction]
#[pyo3(signature = (pool_size, sides, k_rank_from_lowest))]
pub fn order_statistic_pmf(
    pool_size: usize,
    sides: i32,
    k_rank_from_lowest: usize,
) -> PyResult<Pmf> {
    satya_engine::try_order_statistic_pmf(pool_size, sides, k_rank_from_lowest).map_err(to_py_err)
}

/// Exact PMF when rolling $N\text{d}K$ and selecting the die closest to (`"closest"`) or furthest from (`"furthest"`) the midpoint.
#[pyfunction]
#[pyo3(signature = (pool_size, sides, mode="furthest"))]
pub fn center_distance_selection_pmf(pool_size: usize, sides: i32, mode: &str) -> PyResult<Pmf> {
    let m = match mode.trim().to_ascii_lowercase().as_str() {
        "closest" | "focus" => CenterDistanceMode::Closest,
        "furthest" | "chaos" => CenterDistanceMode::Furthest,
        other => {
            return Err(to_py_err(format!(
                "invalid center distance mode '{other}' (expected 'closest' or 'furthest')"
            )));
        }
    };
    Ok(satya_engine::center_distance_selection_pmf(
        pool_size, sides, m,
    ))
}

/// Exact PMF for a Regime-Switching (Median-Gated) pool of $N\text{d}K$.
#[pyfunction]
#[pyo3(signature = (pool_size, sides, gate_tn, roll_under=false))]
pub fn median_gated_bimodal_pmf(
    pool_size: usize,
    sides: i32,
    gate_tn: i32,
    roll_under: bool,
) -> PyResult<Pmf> {
    satya_engine::try_median_gated_bimodal_pmf(pool_size, sides, gate_tn, roll_under)
        .map_err(to_py_err)
}

/// Exact PMF for the product of $N\text{d}K$ ($Z = \prod_{i=1}^N X_i$).
#[pyfunction]
#[pyo3(signature = (pool_size, sides))]
pub fn multiplicative_pool_pmf(pool_size: usize, sides: i32) -> Pmf {
    satya_engine::multiplicative_pool_pmf(pool_size, sides)
}

/// Exact PMF for the Two-List Cascading ("Watershed" / "Recoil") mechanic, returning `(success_pmf, final_y_pmf)`.
#[pyfunction]
#[pyo3(signature = (num_pairs, sides, characteristic_c, delta_over=3, delta_under=3, mode="markov_step", count_under_as_success=false, invert_feedback=false))]
pub fn paired_list_cascade_pmfs(
    num_pairs: usize,
    sides: i32,
    characteristic_c: i32,
    delta_over: i32,
    delta_under: i32,
    mode: &str,
    count_under_as_success: bool,
    invert_feedback: bool,
) -> PyResult<(Pmf, Pmf)> {
    let cm = CarryMode::from_str(mode).map_err(to_py_err)?;
    Ok(satya_engine::paired_list_cascade_pmfs(
        num_pairs,
        sides,
        characteristic_c,
        delta_over,
        delta_under,
        cm,
        count_under_as_success,
        invert_feedback,
    ))
}

/// Exact PMF for the Signed Two-List Cascading ("Watershed" / "Recoil") mechanic over `-num_pairs..=+num_pairs`.
#[pyfunction]
#[pyo3(signature = (num_pairs, sides, characteristic_c, delta_over=3, delta_under=3, mode="markov_step", count_under_as_success=true, invert_feedback=false, botch_rule="closest_miss", margin_aggregation="piecewise_zero_floor"))]
pub fn paired_list_signed_margin_pmfs(
    num_pairs: usize,
    sides: i32,
    characteristic_c: i32,
    delta_over: i32,
    delta_under: i32,
    mode: &str,
    count_under_as_success: bool,
    invert_feedback: bool,
    botch_rule: &str,
    margin_aggregation: &str,
) -> PyResult<PySignedCascadeResult> {
    let cm = CarryMode::from_str(mode).map_err(to_py_err)?;
    let br = BotchRule::from_str(botch_rule).map_err(to_py_err)?;
    let ma = MarginAggregation::from_str(margin_aggregation).map_err(to_py_err)?;
    Ok(PySignedCascadeResult::from(
        satya_engine::paired_list_signed_margin_pmfs(
            num_pairs,
            sides,
            characteristic_c,
            delta_over,
            delta_under,
            cm,
            count_under_as_success,
            invert_feedback,
            br,
            ma,
        ),
    ))
}

/// Evaluates a concrete list of `(die_a, die_b)` `d20` pairs against a Four-Factor Satya task.
#[pyfunction]
#[pyo3(signature = (skill_level, control_c, internal_mod, external_mod, pairs, carry_mode="markov_step", botch_rule="closest_miss"))]
pub fn evaluate_cascade_roll(
    skill_level: usize,
    control_c: i32,
    internal_mod: i32,
    external_mod: i32,
    pairs: Vec<(i32, i32)>,
    carry_mode: &str,
    botch_rule: &str,
) -> PyResult<PyCascadeRollOutcome> {
    let params = PyCascadeTaskParams::new(
        skill_level,
        control_c,
        internal_mod,
        external_mod,
        carry_mode,
        botch_rule,
    )?;
    Ok(params.evaluate_pairs(pairs))
}

/// Computes the Combat Tactical Pool ("Envelope & Fill" — Tempo, Control, Force) for a character.
#[pyfunction]
#[pyo3(signature = (
    spd=10,
    for_stat=10,
    dex=10,
    kin=10,
    sig=10,
    ins=10,
    itu=10,
    dis=10,
    fit=10,
    mass_kg=70.0,
    skill_c=10,
    skill_n=3,
    is_ranged=false,
    fatigue_level=0,
    wound_shock_index=0,
    leg_wound_penalty=0,
    arm_head_wound_penalty=0,
    torso_shoulder_wound_penalty=0,
    morale_state="steady",
    ana=10,
    cre=10,
    foc=10
))]
pub fn compute_tactical_pool(
    spd: i32,
    for_stat: i32,
    dex: i32,
    kin: i32,
    sig: i32,
    ins: i32,
    itu: i32,
    dis: i32,
    fit: i32,
    mass_kg: f64,
    skill_c: i32,
    skill_n: usize,
    is_ranged: bool,
    fatigue_level: i32,
    wound_shock_index: i32,
    leg_wound_penalty: i32,
    arm_head_wound_penalty: i32,
    torso_shoulder_wound_penalty: i32,
    morale_state: &str,
    ana: i32,
    cre: i32,
    foc: i32,
) -> PyResult<PyTacticalPoolEnvelope> {
    let ms = MoraleState::from_str_name(morale_state).map_err(to_py_err)?;
    let params = TacticalPoolParams {
        spd,
        for_stat,
        dex,
        kin,
        sig,
        ins,
        itu,
        ana,
        cre,
        foc,
        dis,
        fit,
        mass_kg,
        skill_c,
        skill_n,
        is_ranged,
        fatigue_level,
        wound_shock_index,
        leg_wound_penalty,
        arm_head_wound_penalty,
        torso_shoulder_wound_penalty,
        morale_state: ms,
    };
    Ok(PyTacticalPoolEnvelope::from(
        satya_engine::compute_tactical_pool(&params),
    ))
}

/// Returns the ordered list of chapter metadata summaries in the Satya TTRPG Rules Book (`docs/ttrpg/`).
#[pyfunction]
pub fn list_rules_chapters() -> Vec<PyRulesChapterSummary> {
    satya_engine::list_rules_chapters()
        .into_iter()
        .map(PyRulesChapterSummary::from)
        .collect()
}

/// Looks up a single chapter in the Satya TTRPG Rules Book by its `chapter_id` slug.
#[pyfunction]
pub fn get_rules_chapter(chapter_id: &str) -> Option<PyRulesChapterDocument> {
    satya_engine::get_rules_chapter(chapter_id).map(PyRulesChapterDocument::from)
}

/// Searches across all chapters of the Satya TTRPG Rules Book (`docs/ttrpg/`) and returns ranked section hits.
#[pyfunction]
#[pyo3(signature = (query, max_results=5))]
pub fn search_rules_book(query: &str, max_results: usize) -> Vec<PyRulesSearchHit> {
    satya_engine::search_rules_book(query, max_results)
        .into_iter()
        .map(PyRulesSearchHit::from)
        .collect()
}

/// Renders a Satya TTRPG Markdown string into safe HTML.
#[pyfunction]
pub fn render_rules_markdown_to_html(markdown: &str) -> String {
    satya_engine::render_rules_markdown_to_html(markdown)
}

/// Classifies a single `(player, gm)` state realization into a contested outcome kind and net margin.
#[pyfunction]
#[pyo3(signature = (sp, bp, mp, sg, bg, mg, tie_rule="hybrid_signed_margin"))]
pub fn classify_contested_outcome(
    sp: i32,
    bp: i32,
    mp: i32,
    sg: i32,
    bg: i32,
    mg: i32,
    tie_rule: &str,
) -> PyResult<(String, i32)> {
    let tr = ContestedTieRule::from_str(tie_rule).map_err(to_py_err)?;
    let (kind, net_margin) = satya_engine::classify_contested_outcome(sp, bp, mp, sg, bg, mg, tr);
    Ok((kind.as_str().to_string(), net_margin))
}

/// Computes exact combinatorial probabilities for a Contested Two-List Cascade check.
#[pyfunction]
#[pyo3(signature = (
    player_skill_n,
    player_control_c=10,
    player_internal_mod=0,
    player_known_tool_mod=0,
    gm_pairs=1,
    hidden_external_difficulty=0,
    tie_rule="hybrid_signed_margin",
    carry_mode="markov_step",
    gm_mode="tiered_single_pair",
    gm_over_tier_step=4
))]
pub fn compute_contested_cascade_odds<'py>(
    py: Python<'py>,
    player_skill_n: usize,
    player_control_c: i32,
    player_internal_mod: i32,
    player_known_tool_mod: i32,
    gm_pairs: usize,
    hidden_external_difficulty: i32,
    tie_rule: &str,
    carry_mode: &str,
    gm_mode: &str,
    gm_over_tier_step: i32,
) -> PyResult<Bound<'py, PyDict>> {
    let cm = CarryMode::from_str(carry_mode).map_err(to_py_err)?;
    let tr = ContestedTieRule::from_str(tie_rule).map_err(to_py_err)?;
    let gm_res_mode = match gm_mode.trim().to_ascii_lowercase().as_str() {
        "tiered_single_pair" => GmResistanceMode::TieredSinglePair {
            step_size: gm_over_tier_step,
        },
        "full_cascade" => GmResistanceMode::FullCascade,
        other => {
            return Err(to_py_err(format!(
                "Unknown gm_mode: '{other}' (expected 'tiered_single_pair' or 'full_cascade')"
            )));
        }
    };

    let mut player = CascadeTaskParams::new_k20(
        player_skill_n,
        player_control_c,
        player_internal_mod,
        player_known_tool_mod,
    );
    player.carry_mode = cm;

    let mut gm = CascadeTaskParams::new_k20(1.max(gm_pairs), 10 + hidden_external_difficulty, 0, 0);
    gm.carry_mode = cm;

    let odds = compute_contested_cascade_odds_with_mode(&player, &gm, gm_res_mode, tr);

    let dict = PyDict::new(py);
    dict.set_item("net_signed_margin_pmf", odds.net_signed_margin_pmf)?;
    dict.set_item("net_success_pmf", odds.net_success_pmf)?;
    dict.set_item("prob_total_win", odds.prob_total_win)?;
    dict.set_item(
        "prob_clean_or_extraordinary_win",
        odds.prob_clean_or_extraordinary_win,
    )?;
    dict.set_item(
        "prob_extraordinary_success",
        odds.prob_extraordinary_success,
    )?;
    dict.set_item("prob_clean_success", odds.prob_clean_success)?;
    dict.set_item("prob_partial_success", odds.prob_partial_success)?;
    dict.set_item("prob_floor_breach_partial", odds.prob_floor_breach_partial)?;
    dict.set_item("prob_grazing_tie_partial", odds.prob_grazing_tie_partial)?;
    dict.set_item("prob_lucky_break_partial", odds.prob_lucky_break_partial)?;
    dict.set_item("prob_clean_failure", odds.prob_clean_failure)?;
    dict.set_item("prob_botch_failure", odds.prob_botch_failure)?;
    dict.set_item(
        "net_margin_stats",
        PyDistributionStats::from(odds.net_margin_stats),
    )?;
    dict.set_item("bimodality_coefficient", odds.bimodality_coefficient)?;
    Ok(dict)
}

/// Computes the Satya Combat Tactical Pool ("Envelope & Fill") interactive lab summary.
#[pyfunction]
#[pyo3(signature = (
    control_c,
    skill_level,
    delta_int=0,
    delta_ext=0,
    num_successes=2,
    weapon_reach=2,
    opponent_reach=2
))]
pub fn compute_tactical_pool_envelope<'py>(
    py: Python<'py>,
    control_c: i32,
    skill_level: i32,
    delta_int: i32,
    delta_ext: i32,
    num_successes: i32,
    weapon_reach: i32,
    opponent_reach: i32,
) -> PyResult<Bound<'py, PyDict>> {
    let res = satya_engine::compute_tactical_pool_envelope(
        control_c,
        skill_level,
        delta_int,
        delta_ext,
        num_successes,
        weapon_reach,
        opponent_reach,
    );
    let dict = PyDict::new(py);
    dict.set_item("base_envelope", res.base_envelope)?;
    dict.set_item("raw_pool", res.raw_pool)?;
    dict.set_item("effective_pool", res.effective_pool)?;
    dict.set_item("overflow_wasted", res.overflow_wasted)?;
    dict.set_item("reach_modifier", res.reach_modifier)?;
    dict.set_item("max_tempo", res.max_tempo)?;
    dict.set_item("max_control", res.max_control)?;
    dict.set_item("max_force", res.max_force)?;
    dict.set_item("backend", "satya_py (Rust PyO3)")?;
    Ok(dict)
}

/// Evaluates a Close-Quarters Tactical Triad (`Tempo`, `Control`, `Force`) contested exchange.
#[pyfunction]
#[pyo3(signature = (
    attacker_tempo,
    attacker_control,
    attacker_force,
    defender_tempo,
    defender_control,
    defender_force,
    attacker_root=12,
    defender_guard=4,
    armor_dr=3
))]
pub fn evaluate_tactical_triad_contest<'py>(
    py: Python<'py>,
    attacker_tempo: i32,
    attacker_control: i32,
    attacker_force: i32,
    defender_tempo: i32,
    defender_control: i32,
    defender_force: i32,
    attacker_root: i32,
    defender_guard: i32,
    armor_dr: i32,
) -> PyResult<Bound<'py, PyDict>> {
    let res = satya_engine::evaluate_tactical_triad_contest(
        attacker_tempo,
        attacker_control,
        attacker_force,
        defender_tempo,
        defender_control,
        defender_force,
        attacker_root,
        defender_guard,
        armor_dr,
    );
    let dict = PyDict::new(py);
    dict.set_item("net_tempo", res.net_tempo)?;
    dict.set_item("net_control", res.net_control)?;
    dict.set_item("seizes_initiative", res.seizes_initiative)?;
    dict.set_item("guard_broken", res.guard_broken)?;
    dict.set_item("throw_triggered", res.throw_triggered)?;
    dict.set_item("penetrating_force", res.penetrating_force)?;
    dict.set_item("next_tick_cost", res.next_tick_cost)?;
    dict.set_item("archetype_label", res.archetype_label)?;
    dict.set_item("outcome_summary", res.outcome_summary)?;
    dict.set_item("backend", "satya_py (Rust PyO3)")?;
    Ok(dict)
}

/// Computes non-linear skill level `N` (`0..10`) from study hours using Chapter 6's exponential curve.
#[pyfunction]
#[pyo3(signature = (
    hours_studied,
    difficulty_multiplier=1.0,
    instruction_quality=1.0,
    aptitude_bonus=0
))]
pub fn compute_lifepath_skill_progression<'py>(
    py: Python<'py>,
    hours_studied: f64,
    difficulty_multiplier: f64,
    instruction_quality: f64,
    aptitude_bonus: i32,
) -> PyResult<Bound<'py, PyDict>> {
    let res = satya_engine::compute_lifepath_skill_progression(
        hours_studied,
        difficulty_multiplier,
        instruction_quality,
        aptitude_bonus,
    );
    let dict = PyDict::new(py);
    dict.set_item("raw_hours", res.raw_hours)?;
    dict.set_item("effective_hours", res.effective_hours)?;
    dict.set_item("skill_level_n", res.skill_level_n)?;
    dict.set_item("next_level_n", res.next_level_n)?;
    dict.set_item("hours_to_next_level", res.hours_to_next_level)?;
    dict.set_item("cumulative_thresholds", res.cumulative_thresholds)?;
    Ok(dict)
}

/// Computes Cognitive Dissonance pressure and belief-shift outcome from Chapter 9.
#[pyfunction]
#[pyo3(signature = (
    argument_margin,
    target_conviction,
    evidence_tier,
    shared_values_bonus=0,
    public_audience_pressure=0
))]
pub fn compute_social_dissonance_check<'py>(
    py: Python<'py>,
    argument_margin: i32,
    target_conviction: i32,
    evidence_tier: i32,
    shared_values_bonus: i32,
    public_audience_pressure: i32,
) -> PyResult<Bound<'py, PyDict>> {
    let res = satya_engine::compute_social_dissonance_check(
        argument_margin,
        target_conviction,
        evidence_tier,
        shared_values_bonus,
        public_audience_pressure,
    );
    let dict = PyDict::new(py);
    dict.set_item("dissonance_pressure", res.dissonance_pressure)?;
    dict.set_item("resistance_threshold", res.resistance_threshold)?;
    dict.set_item("net_dissonance_shift", res.net_dissonance_shift)?;
    dict.set_item("social_outcome", res.social_outcome)?;
    Ok(dict)
}
/// Evaluates Step 1 of the Satya Task Resolution Procedure (§2.1 Task Gate Classifications & §2.2 Tool Dependency Tiers).
#[pyfunction]
#[pyo3(signature = (
    gate_class="class_a",
    trained_skill_n=0,
    talent_or_parent_n=0,
    default_penalty_ext=4,
    tool_dependency="none",
    tool_tier="standard_kit",
    min_skill_n=1,
))]
pub fn resolve_task_gate_and_tools(
    gate_class: &str,
    trained_skill_n: usize,
    talent_or_parent_n: usize,
    default_penalty_ext: i32,
    tool_dependency: &str,
    tool_tier: &str,
    min_skill_n: usize,
) -> PyTaskGateResolution {
    let gate = match gate_class.trim().to_ascii_lowercase().as_str() {
        "class_b" | "b" => TaskGateClass::ClassB,
        "class_c" | "c" => TaskGateClass::ClassC {
            min_skill_n: min_skill_n.max(1),
        },
        _ => TaskGateClass::ClassA,
    };
    let dep = match tool_dependency.trim().to_ascii_lowercase().as_str() {
        "mandatory" => ToolDependency::Mandatory,
        "situational" => ToolDependency::Situational,
        _ => ToolDependency::None,
    };
    let tier = ToolQualityTier::from_key(tool_tier);
    PyTaskGateResolution::from(satya_engine::resolve_task_gate_and_tools(
        gate,
        trained_skill_n,
        talent_or_parent_n,
        default_penalty_ext,
        dep,
        tier,
    ))
}

/// Computes a Composite Controlling Characteristic $C = \lfloor (C_1 + C_2) / 2 \rfloor$.
#[pyfunction]
pub fn compute_composite_characteristic(c1: i32, c2: i32) -> i32 {
    satya_engine::compute_composite_characteristic(c1, c2)
}

/// Computes the Gated Supporting Characteristic shortfall penalty ($+2$ to $\Delta_{\text{ext}}$ per point below `min_required_c`).
#[pyfunction]
pub fn compute_supporting_characteristic_shortfall_penalty(
    supporting_c: i32,
    min_required_c: i32,
) -> i32 {
    satya_engine::compute_supporting_characteristic_shortfall_penalty(supporting_c, min_required_c)
}

/// Evaluates a preliminary `KNO` / `LOG` complementary setup roll per §6.1 of `cascading_d20_and_tasks.md`.
#[pyfunction]
pub fn evaluate_complementary_setup_synergy(
    setup_total_successes: i32,
    setup_signed_margin: i32,
) -> PyComplementarySynergyResult {
    PyComplementarySynergyResult::from(satya_engine::evaluate_complementary_setup_synergy(
        setup_total_successes,
        setup_signed_margin,
    ))
}

/// Computes the effective cascade pool size $N_{\text{eff}}$ for a Simultaneous Dual-Skill Gate per §6.2.
#[pyfunction]
pub fn compute_dual_skill_pool(skill_n1: usize, skill_n2: usize) -> usize {
    satya_engine::compute_dual_skill_pool(skill_n1, skill_n2)
}

/// PyO3 module definition for `satya_py`.
#[pymodule]
pub fn satya_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDistributionStats>()?;
    m.add_class::<PySignedCascadeResult>()?;
    m.add_class::<PyCascadeStepTrace>()?;
    m.add_class::<PyCascadeRollOutcome>()?;
    m.add_class::<PyCascadeTaskParams>()?;
    m.add_class::<PyContestedCascadeOdds>()?;
    m.add_class::<PyContestedCascadeRollOutcome>()?;
    m.add_class::<PyContestedCascadeTaskParams>()?;
    m.add_class::<PyTaskGateResolution>()?;
    m.add_class::<PyComplementarySynergyResult>()?;
    m.add_class::<PyExtendedProjectStepResult>()?;
    m.add_class::<PyExtendedProjectClock>()?;
    m.add_class::<PyTacticalPoolEnvelope>()?;
    m.add_class::<PyTacticalTriadEvaluation>()?;
    m.add_class::<PyRulesChapterSummary>()?;
    m.add_class::<PyRulesChapterDocument>()?;
    m.add_class::<PyRulesSearchHit>()?;
    m.add_class::<PySchemaRegistry>()?;

    m.add_function(wrap_pyfunction!(normalize_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(compute_stats, m)?)?;
    m.add_function(wrap_pyfunction!(convolve_pmfs, m)?)?;
    m.add_function(wrap_pyfunction!(single_die_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(sum_dice_pool_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(count_successes_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(single_exploding_die_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(exploding_pool_keep_highest_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(order_statistic_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(center_distance_selection_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(median_gated_bimodal_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(multiplicative_pool_pmf, m)?)?;
    m.add_function(wrap_pyfunction!(paired_list_cascade_pmfs, m)?)?;
    m.add_function(wrap_pyfunction!(paired_list_signed_margin_pmfs, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_cascade_roll, m)?)?;
    m.add_function(wrap_pyfunction!(compute_tactical_pool, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_task_gate_and_tools, m)?)?;
    m.add_function(wrap_pyfunction!(compute_composite_characteristic, m)?)?;
    m.add_function(wrap_pyfunction!(
        compute_supporting_characteristic_shortfall_penalty,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(evaluate_complementary_setup_synergy, m)?)?;
    m.add_function(wrap_pyfunction!(compute_dual_skill_pool, m)?)?;
    m.add_function(wrap_pyfunction!(list_rules_chapters, m)?)?;
    m.add_function(wrap_pyfunction!(get_rules_chapter, m)?)?;
    m.add_function(wrap_pyfunction!(search_rules_book, m)?)?;
    m.add_function(wrap_pyfunction!(render_rules_markdown_to_html, m)?)?;
    m.add_function(wrap_pyfunction!(classify_contested_outcome, m)?)?;
    m.add_function(wrap_pyfunction!(compute_contested_cascade_odds, m)?)?;
    m.add_function(wrap_pyfunction!(compute_tactical_pool_envelope, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_tactical_triad_contest, m)?)?;
    m.add_function(wrap_pyfunction!(compute_lifepath_skill_progression, m)?)?;
    m.add_function(wrap_pyfunction!(compute_social_dissonance_check, m)?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satya_py_wrappers_cascade_tactical_pool_and_rules_book() {
        let params = PyCascadeTaskParams::new(4, 11, 2, 2, "markov_step", "closest_miss").unwrap();
        assert_eq!(params.tn_eff(), 22);
        assert_eq!(params.effective_delta_under(), 8);

        let odds = params.exact_odds();
        assert!(odds.prob_any_success > 0.5);

        let roll = params.evaluate_pairs(vec![(14, 11), (8, 7), (16, 15), (17, 12)]);
        assert_eq!(roll.total_successes, 3);
        assert_eq!(roll.outcome_tier, "EXTRAORDINARY_SUCCESS");

        let sarah_contested = PyContestedCascadeTaskParams::from_preset("sarah_lockpick");
        assert_eq!(sarah_contested.tn_player(), 19);
        assert_eq!(sarah_contested.tn_gm(), 17);
        let c_odds = sarah_contested.exact_contested_odds();
        assert!(c_odds.prob_total_win > 0.45);
        let c_roll = sarah_contested
            .evaluate_contested_pairs(vec![(11, 9), (7, 6), (12, 10)], vec![(9, 9)]);
        assert_eq!(c_roll.contested_kind, "CLEAN_SUCCESS");

        let gate_res = resolve_task_gate_and_tools("class_b", 0, 3, 4, "situational", "improvised", 1);
        assert!(gate_res.allowed);
        assert_eq!(gate_res.effective_skill_n, 2);
        assert_eq!(gate_res.tool_delta_mod, 4);

        assert_eq!(compute_dual_skill_pool(4, 2), 3);
        let syn = evaluate_complementary_setup_synergy(3, 3);
        assert_eq!(syn.delta_ext_mod, -4);
        assert_eq!(syn.delta_over_carry, 8);

        let mut clock = PyExtendedProjectClock::new("Antitoxin Synthesis".to_string(), 10);
        let step = clock.apply_interval(4, 4, true, false);
        assert_eq!(step.accumulated_successes, 4);
        assert_eq!(step.heat_generated, 1);
        assert_eq!(step.extra_material_pct, 25);

        let env = compute_tactical_pool(
            12, 12, 13, 13, 11, 12, 12, 12, 12, 75.0, 14, 4, false, 0, 0, 0, 0, 0, "steady", 12,
            12, 12,
        )
        .unwrap();
        assert!(env.p_eff > 0);
        let triad = env.evaluate_triad(8, 8, 8, 6);
        assert!(triad.valid);

        let chapters = list_rules_chapters();
        assert_eq!(chapters.len(), 14);

        let hits = search_rules_book("Tactical Triad", 3);
        assert!(!hits.is_empty());

        let reg = PySchemaRegistry::new().unwrap();
        assert!(!reg.list_schemas().is_empty());
        let pii_json = reg.extract_pii_manifest().unwrap();
        assert!(pii_json.contains("satya"));
    }
}
