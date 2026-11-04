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
use satya_engine::{
    BotchRule, CarryMode, CascadeRollOutcome, CascadeStepTrace, CascadeTaskParams,
    CenterDistanceMode, DistributionStats, MarginAggregation, MoraleState, Pmf,
    SignedCascadeResult, TacticalPoolEnvelope, TacticalPoolParams, TacticalTriadEvaluation,
    TnFormulaK20, format_summary_table,
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

/// Computed Combat Tactical Pool ("Fill") and Characteristic Aspect Ceilings ("Envelope").
#[pyclass(name = "TacticalPoolEnvelope")]
#[derive(Debug, Clone)]
pub struct PyTacticalPoolEnvelope {
    /// OODA Initiative rating: $\lfloor(\text{SPD} + \text{INS} + \text{ITU}) / 3\rfloor$.
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
pub fn normalize_pmf(weights: Pmf) -> Pmf {
    satya_engine::normalize_pmf(&weights)
}

/// Computes exact mean, variance, standard deviation, skewness, and excess kurtosis of a PMF.
#[pyfunction]
pub fn compute_stats(pmf: Pmf) -> PyDistributionStats {
    PyDistributionStats::from(satya_engine::compute_stats(&pmf))
}

/// Computes the discrete convolution $P(Z = x + y)$ of two independent PMFs.
#[pyfunction]
pub fn convolve_pmfs(pmf_a: Pmf, pmf_b: Pmf) -> Pmf {
    satya_engine::convolve_pmfs(&pmf_a, &pmf_b)
}

/// Uniform PMF for a single $k$-sided die ($1 \dots k$, or $-k \dots -1$ if `negative=True`).
#[pyfunction]
#[pyo3(signature = (k, negative=false))]
pub fn single_die_pmf(k: i32, negative: bool) -> Pmf {
    satya_engine::single_die_pmf(k, negative)
}

/// Exact PMF for the sum of `n_pos` positive `dK` dice and `n_neg` negative `dK` dice.
#[pyfunction]
#[pyo3(signature = (n_pos, k, n_neg=0))]
pub fn sum_dice_pool_pmf(n_pos: usize, k: i32, n_neg: usize) -> Pmf {
    satya_engine::sum_dice_pool_pmf(n_pos, k, n_neg)
}

/// Exact PMF for counting successes in an $N\text{d}K$ pool.
#[pyfunction]
#[pyo3(signature = (n, k, target, roll_under=true, crit_double_on_one=false))]
pub fn count_successes_pmf(
    n: usize,
    k: i32,
    target: i32,
    roll_under: bool,
    crit_double_on_one: bool,
) -> Pmf {
    satya_engine::count_successes_pmf(n, k, target, roll_under, crit_double_on_one)
}

/// Exact PMF for a single exploding die (`dK`) up to `max_explosions`.
#[pyfunction]
#[pyo3(signature = (k, explode_at=None, max_explosions=4))]
pub fn single_exploding_die_pmf(k: i32, explode_at: Option<i32>, max_explosions: usize) -> Pmf {
    satya_engine::single_exploding_die_pmf(k, explode_at, max_explosions)
}

/// Exact PMF for the maximum of $N$ exploding `dK` dice (Gumbel extreme-value distribution).
#[pyfunction]
#[pyo3(signature = (n, k, explode_at=None, max_explosions=4))]
pub fn exploding_pool_keep_highest_pmf(
    n: usize,
    k: i32,
    explode_at: Option<i32>,
    max_explosions: usize,
) -> Pmf {
    satya_engine::exploding_pool_keep_highest_pmf(n, k, explode_at, max_explosions)
}

/// Exact PMF of the $r$-th order statistic ($X_{(r)}$, 1-indexed from lowest) of $N\text{d}K$.
#[pyfunction]
pub fn order_statistic_pmf(n: usize, k: i32, rank: usize) -> Pmf {
    satya_engine::order_statistic_pmf(n, k, rank)
}

/// Exact PMF when rolling $N\text{d}K$ and selecting the die closest to (`"closest"`) or furthest from (`"furthest"`) the midpoint.
#[pyfunction]
#[pyo3(signature = (n, k, mode="closest"))]
pub fn center_distance_selection_pmf(n: usize, k: i32, mode: &str) -> PyResult<Pmf> {
    let m = match mode.trim().to_ascii_lowercase().as_str() {
        "closest" | "focus" => CenterDistanceMode::Closest,
        "furthest" | "chaos" => CenterDistanceMode::Furthest,
        other => {
            return Err(to_py_err(format!(
                "invalid center distance mode '{other}' (expected 'closest' or 'furthest')"
            )));
        }
    };
    Ok(satya_engine::center_distance_selection_pmf(n, k, m))
}

/// Exact PMF for a Regime-Switching (Median-Gated) pool of $N\text{d}K$.
#[pyfunction]
#[pyo3(signature = (n, k, threshold, roll_under=false))]
pub fn median_gated_bimodal_pmf(
    n: usize,
    k: i32,
    threshold: i32,
    roll_under: bool,
) -> PyResult<Pmf> {
    satya_engine::try_median_gated_bimodal_pmf(n, k, threshold, roll_under).map_err(to_py_err)
}

/// Exact PMF for the product of $N\text{d}K$ ($Z = \prod_{i=1}^N X_i$).
#[pyfunction]
pub fn multiplicative_pool_pmf(n: usize, k: i32) -> Pmf {
    satya_engine::multiplicative_pool_pmf(n, k)
}

/// Exact PMF for the Two-List Cascading ("Watershed" / "Recoil") mechanic, returning `(success_pmf, final_y_pmf)`.
#[pyfunction]
#[pyo3(signature = (n_pairs, k, threshold, delta_over, delta_under, carry_mode="markov_step", count_under_as_success=false, invert_feedback=false))]
pub fn paired_list_cascade_pmfs(
    n_pairs: usize,
    k: i32,
    threshold: i32,
    delta_over: i32,
    delta_under: i32,
    carry_mode: &str,
    count_under_as_success: bool,
    invert_feedback: bool,
) -> PyResult<(Pmf, Pmf)> {
    let cm = CarryMode::from_str(carry_mode).map_err(to_py_err)?;
    Ok(satya_engine::paired_list_cascade_pmfs(
        n_pairs,
        k,
        threshold,
        delta_over,
        delta_under,
        cm,
        count_under_as_success,
        invert_feedback,
    ))
}

/// Exact PMF for the Signed Two-List Cascading ("Watershed" / "Recoil") mechanic over `-n_pairs..=+n_pairs`.
#[pyfunction]
#[pyo3(signature = (n_pairs, k, threshold, delta_over, delta_under, carry_mode="markov_step", count_under_as_success=false, invert_feedback=false, botch_rule="closest_miss", margin_aggregation="piecewise_zero_floor"))]
pub fn paired_list_signed_margin_pmfs(
    n_pairs: usize,
    k: i32,
    threshold: i32,
    delta_over: i32,
    delta_under: i32,
    carry_mode: &str,
    count_under_as_success: bool,
    invert_feedback: bool,
    botch_rule: &str,
    margin_aggregation: &str,
) -> PyResult<PySignedCascadeResult> {
    let cm = CarryMode::from_str(carry_mode).map_err(to_py_err)?;
    let br = BotchRule::from_str(botch_rule).map_err(to_py_err)?;
    let ma = MarginAggregation::from_str(margin_aggregation).map_err(to_py_err)?;
    Ok(PySignedCascadeResult::from(
        satya_engine::paired_list_signed_margin_pmfs(
            n_pairs,
            k,
            threshold,
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

/// Formats the full `K = 20` (`2d20`) parameter-range calibration report as a table string.
#[pyfunction]
#[pyo3(signature = (control_values=None, skill_levels=None))]
pub fn evaluate_k20_full_range(
    control_values: Option<Vec<i32>>,
    skill_levels: Option<Vec<usize>>,
) -> String {
    let rows = satya_engine::evaluate_k20_full_range(
        control_values.as_deref(),
        skill_levels.as_deref(),
        None,
        None,
        None,
        None,
        None,
    );
    format_summary_table(&rows)
}

/// Formats the `K = 20` (`2d20`) Controlling Characteristic ($C$) vs. Skill Level ($N$) comparison matrix.
#[pyfunction]
#[pyo3(signature = (base_delta_over=6, base_delta_under=6, carry_mode="markov_step", tn_formula="31_minus_c"))]
pub fn compare_control_vs_skill_k20(
    base_delta_over: i32,
    base_delta_under: i32,
    carry_mode: &str,
    tn_formula: &str,
) -> PyResult<String> {
    let cm = CarryMode::from_str(carry_mode).map_err(to_py_err)?;
    let tf = match tn_formula.trim().to_ascii_lowercase().as_str() {
        "31_minus_c" | "31-c" => TnFormulaK20::Formula31MinusC,
        "41_minus_2c" | "41-2c" => TnFormulaK20::Formula41Minus2c,
        other => {
            return Err(to_py_err(format!(
                "invalid tn_formula '{other}' (expected '31_minus_c' or '41_minus_2c')"
            )));
        }
    };
    Ok(satya_engine::compare_control_vs_skill_k20(
        base_delta_over,
        base_delta_under,
        cm,
        tf,
    ))
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
    morale_state="steady"
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

/// PyO3 module definition for `satya_py`.
#[pymodule]
pub fn satya_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDistributionStats>()?;
    m.add_class::<PySignedCascadeResult>()?;
    m.add_class::<PyCascadeStepTrace>()?;
    m.add_class::<PyCascadeRollOutcome>()?;
    m.add_class::<PyCascadeTaskParams>()?;
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
    m.add_function(wrap_pyfunction!(evaluate_k20_full_range, m)?)?;
    m.add_function(wrap_pyfunction!(compare_control_vs_skill_k20, m)?)?;
    m.add_function(wrap_pyfunction!(compute_tactical_pool, m)?)?;
    m.add_function(wrap_pyfunction!(list_rules_chapters, m)?)?;
    m.add_function(wrap_pyfunction!(get_rules_chapter, m)?)?;
    m.add_function(wrap_pyfunction!(search_rules_book, m)?)?;
    m.add_function(wrap_pyfunction!(render_rules_markdown_to_html, m)?)?;

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

        let env = compute_tactical_pool(
            12, 12, 13, 13, 11, 12, 12, 12, 12, 75.0, 14, 4, false, 0, 0, 0, 0, 0, "steady",
        )
        .unwrap();
        assert!(env.p_eff > 0);
        let triad = env.evaluate_triad(8, 8, 8, 6);
        assert!(triad.valid);

        let chapters = list_rules_chapters();
        assert_eq!(chapters.len(), 9);

        let hits = search_rules_book("Tactical Triad", 3);
        assert!(!hits.is_empty());

        let reg = PySchemaRegistry::new().unwrap();
        assert!(!reg.list_schemas().is_empty());
        let pii_json = reg.extract_pii_manifest().unwrap();
        assert!(pii_json.contains("satya"));
    }
}
