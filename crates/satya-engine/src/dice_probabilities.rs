//! Exact combinatorial and dynamic-programming probability distribution engines for Satya TTRPG dice mechanics.
//!
//! Canonical Rust source of truth (exposed to Python via the `satya_py` PyO3 crate) and calibrated per `docs/ttrpg/probability.md`.
//!
//! Provides:
//! - Exact probability mass functions ([`Pmf`]) and summary moments ([`DistributionStats`]: mean, variance, std, skewness, excess kurtosis).
//! - Additive & negative dice pools ([`sum_dice_pool_pmf`]), binomial & Satya d20 critical-1 pools ([`count_successes_pmf`]).
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
use std::fmt;
use std::str::FromStr;

/// Discrete probability mass function mapping outcome integer values to their exact probabilities in `[0.0, 1.0]`.
///
/// Stored in a [`BTreeMap`] so keys are always sorted in ascending order.
pub type Pmf = BTreeMap<i32, f64>;

/// Summary moments of a discrete probability distribution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DistributionStats {
    /// Expected value $E[X] = \sum x P(X = x)$.
    pub mean: f64,
    /// Variance $\text{Var}(X) = E[(X - \mu)^2]$.
    pub variance: f64,
    /// Standard deviation $\sigma = \sqrt{\text{Var}(X)}$.
    pub std: f64,
    /// Standardized third central moment $E[(X - \mu)^3] / \sigma^3$.
    pub skewness: f64,
    /// Standardized fourth central moment minus 3: $(E[(X - \mu)^4] / \sigma^4) - 3$.
    pub excess_kurtosis: f64,
}

/// Returns a sorted, normalized probability mass function with negligible entries (`<= 1e-15`) removed.
pub fn normalize_pmf(pmf: &BTreeMap<i32, f64>) -> Pmf {
    let total: f64 = pmf.values().copied().sum();
    if total <= 0.0 {
        return BTreeMap::new();
    }
    let mut out = BTreeMap::new();
    for (&k, &v) in pmf {
        if v > 1e-15 {
            out.insert(k, v / total);
        }
    }
    out
}

fn normalize_hash_pmf(pmf: &HashMap<i32, f64>) -> Pmf {
    let total: f64 = pmf.values().copied().sum();
    if total <= 0.0 {
        return BTreeMap::new();
    }
    let mut out = BTreeMap::new();
    for (&k, &v) in pmf {
        if v > 1e-15 {
            out.insert(k, v / total);
        }
    }
    out
}

/// Computes exact mean, variance, standard deviation, skewness, and excess kurtosis for a [`Pmf`].
pub fn compute_stats(pmf: &Pmf) -> DistributionStats {
    let norm = normalize_pmf(pmf);
    if norm.is_empty() {
        return DistributionStats {
            mean: 0.0,
            variance: 0.0,
            std: 0.0,
            skewness: 0.0,
            excess_kurtosis: 0.0,
        };
    }

    let mean: f64 = norm.iter().map(|(&x, &p)| (x as f64) * p).sum();
    let variance: f64 = norm
        .iter()
        .map(|(&x, &p)| {
            let diff = (x as f64) - mean;
            diff * diff * p
        })
        .sum();
    let std = variance.max(0.0).sqrt();
    if std < 1e-12 {
        return DistributionStats {
            mean,
            variance: 0.0,
            std: 0.0,
            skewness: 0.0,
            excess_kurtosis: 0.0,
        };
    }

    let m3: f64 = norm
        .iter()
        .map(|(&x, &p)| {
            let diff = (x as f64) - mean;
            diff.powi(3) * p
        })
        .sum();
    let m4: f64 = norm
        .iter()
        .map(|(&x, &p)| {
            let diff = (x as f64) - mean;
            diff.powi(4) * p
        })
        .sum();

    let skewness = m3 / std.powi(3);
    let excess_kurtosis = (m4 / variance.powi(2)) - 3.0;

    DistributionStats {
        mean,
        variance,
        std,
        skewness,
        excess_kurtosis,
    }
}

/// Computes the exact distribution of $X + Y$ for independent discrete random variables $X$ and $Y$.
pub fn convolve_pmfs(pmf_a: &Pmf, pmf_b: &Pmf) -> Pmf {
    let mut out: BTreeMap<i32, f64> = BTreeMap::new();
    for (&x, &px) in pmf_a {
        for (&y, &py) in pmf_b {
            *out.entry(x + y).or_insert(0.0) += px * py;
        }
    }
    normalize_pmf(&out)
}

/// Returns the uniform [`Pmf`] for a single die `dK` (`1..=sides`), or `-dK` (`-sides..=-1`) when `negative` is `true`.
pub fn single_die_pmf(sides: i32, negative: bool) -> Pmf {
    let mut out = BTreeMap::new();
    if sides <= 0 {
        return out;
    }
    let p = 1.0 / (sides as f64);
    if negative {
        for face in 1..=sides {
            out.insert(-face, p);
        }
    } else {
        for face in 1..=sides {
            out.insert(face, p);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 1. Additive & Negative Dice Pools (Central Limit Theorem Demonstration)
// ---------------------------------------------------------------------------

/// Exact [`Pmf`] of the sum of `pos_dice` `dK` minus `neg_dice` `dK`.
pub fn sum_dice_pool_pmf(pos_dice: usize, sides: i32, neg_dice: usize) -> Pmf {
    let mut result = BTreeMap::from([(0, 1.0)]);
    let pos_die = single_die_pmf(sides, false);
    let neg_die = single_die_pmf(sides, true);
    for _ in 0..pos_dice {
        result = convolve_pmfs(&result, &pos_die);
    }
    for _ in 0..neg_dice {
        result = convolve_pmfs(&result, &neg_die);
    }
    result
}

// ---------------------------------------------------------------------------
// 2. Success-Counting Dice Pools (Binomial & Satya d20 Critical-1 Pool)
// ---------------------------------------------------------------------------

/// Exact [`Pmf`] of success counts in an `NdK` pool.
///
/// If `crit_on_one_double` is `true` (as in Satya's roll-under `d20` rules), rolling a `1` on a hit counts as `2` successes.
pub fn count_successes_pmf(
    pool_size: usize,
    sides: i32,
    target_num: i32,
    roll_under: bool,
    crit_on_one_double: bool,
) -> Pmf {
    let mut die_success_pmf: BTreeMap<i32, f64> = BTreeMap::new();
    if sides <= 0 {
        return die_success_pmf;
    }
    let p = 1.0 / (sides as f64);
    for face in 1..=sides {
        let is_hit = if roll_under {
            face <= target_num
        } else {
            face >= target_num
        };
        if crit_on_one_double && face == 1 && is_hit {
            *die_success_pmf.entry(2).or_insert(0.0) += p;
        } else if is_hit {
            *die_success_pmf.entry(1).or_insert(0.0) += p;
        } else {
            *die_success_pmf.entry(0).or_insert(0.0) += p;
        }
    }

    let mut result = BTreeMap::from([(0, 1.0)]);
    for _ in 0..pool_size {
        result = convolve_pmfs(&result, &die_success_pmf);
    }
    result
}

// ---------------------------------------------------------------------------
// 3. Exploding Dice (Single, Summed Pool, and Keep-Highest Gumbel Pool)
// ---------------------------------------------------------------------------

/// Exact [`Pmf`] of a single exploding `dK` (re-roll and add when `face >= explode_min`, up to `max_explosions` times).
pub fn single_exploding_die_pmf(
    sides: i32,
    explode_min: Option<i32>,
    max_explosions: usize,
) -> Pmf {
    if sides <= 0 {
        return BTreeMap::new();
    }
    let exp_min = explode_min.unwrap_or(sides);
    let p = 1.0 / (sides as f64);
    let mut active: BTreeMap<i32, f64> = BTreeMap::from([(0, 1.0)]);
    let mut finished: BTreeMap<i32, f64> = BTreeMap::new();

    for step in 0..=max_explosions {
        let mut next_active: BTreeMap<i32, f64> = BTreeMap::new();
        let is_last = step == max_explosions;
        for (&current_sum, &prob) in &active {
            for face in 1..=sides {
                let new_sum = current_sum + face;
                if face >= exp_min && !is_last {
                    *next_active.entry(new_sum).or_insert(0.0) += prob * p;
                } else {
                    *finished.entry(new_sum).or_insert(0.0) += prob * p;
                }
            }
        }
        active = next_active;
        if active.is_empty() {
            break;
        }
    }

    normalize_pmf(&finished)
}

/// Exact [`Pmf`] of taking the maximum of `pool_size` independent exploding dice (Gumbel extreme value limit).
pub fn exploding_pool_keep_highest_pmf(
    pool_size: usize,
    sides: i32,
    explode_min: Option<i32>,
    max_explosions: usize,
) -> Pmf {
    let single = single_exploding_die_pmf(sides, explode_min, max_explosions);
    let mut cdf_prev = 0.0_f64;
    let mut out = BTreeMap::new();
    let n_exp = pool_size as i32;
    for (&v, &prob) in &single {
        let cdf_curr = cdf_prev + prob;
        out.insert(v, cdf_curr.powi(n_exp) - cdf_prev.powi(n_exp));
        cdf_prev = cdf_curr;
    }
    normalize_pmf(&out)
}

// ---------------------------------------------------------------------------
// 4. Order Statistics: Roll N, Keep k-th Lowest/Highest (Discrete Beta Family)
// ---------------------------------------------------------------------------

fn binomial_coeff(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut result = 1.0_f64;
    for i in 0..k {
        result *= (n - i) as f64;
        result /= (i + 1) as f64;
    }
    result
}

/// Fallible version of [`order_statistic_pmf`] that validates `1 <= k_rank_from_lowest <= pool_size`.
pub fn try_order_statistic_pmf(
    pool_size: usize,
    sides: i32,
    k_rank_from_lowest: usize,
) -> Result<Pmf, String> {
    if k_rank_from_lowest < 1 || k_rank_from_lowest > pool_size {
        return Err(format!(
            "k_rank_from_lowest must be in 1..={pool_size}, got {k_rank_from_lowest}"
        ));
    }
    if sides <= 0 {
        return Err(format!("sides must be >= 1, got {sides}"));
    }

    let cdf = |x: i32| -> f64 {
        if x <= 0 {
            return 0.0;
        }
        if x >= sides {
            return 1.0;
        }
        let p = (x as f64) / (sides as f64);
        let q = 1.0 - p;
        let mut sum = 0.0;
        for j in k_rank_from_lowest..=pool_size {
            sum += binomial_coeff(pool_size, j) * p.powi(j as i32) * q.powi((pool_size - j) as i32);
        }
        sum
    };

    let mut out = BTreeMap::new();
    for face in 1..=sides {
        out.insert(face, cdf(face) - cdf(face - 1));
    }
    Ok(normalize_pmf(&out))
}

/// Exact [`Pmf`] of the `k`-th order statistic (`1` = lowest, `pool_size` = highest) of `NdK`.
///
/// # Panics
/// Panics if `k_rank_from_lowest` is outside `1..=pool_size` or `sides <= 0`.
pub fn order_statistic_pmf(pool_size: usize, sides: i32, k_rank_from_lowest: usize) -> Pmf {
    try_order_statistic_pmf(pool_size, sides, k_rank_from_lowest)
        .expect("invalid order_statistic_pmf arguments")
}

// ---------------------------------------------------------------------------
// 5. Distance-from-Center Selection: Focus vs. Neutral vs. Chaos (U-Shaped)
// ---------------------------------------------------------------------------

/// Selection mode for [`center_distance_selection_pmf`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CenterDistanceMode {
    /// Pick the die closest to the midpoint $(K+1)/2$ (Focus / bell-like concentration).
    Closest,
    /// Pick the die furthest from the midpoint $(K+1)/2$ (Chaos / U-shaped bimodal distribution).
    Furthest,
}

/// Roll `pool_size` `dK` and pick the die closest to or furthest from the center $(K+1)/2$.
///
/// Ties in distance between a low face $f$ and its mirror $K + 1 - f$ are split 50/50 so symmetry is preserved.
pub fn center_distance_selection_pmf(
    pool_size: usize,
    sides: i32,
    mode: CenterDistanceMode,
) -> Pmf {
    if sides <= 0 || pool_size == 0 {
        return BTreeMap::new();
    }
    // Twice the distance from (sides + 1) / 2 is an exact integer: |2 * face - (sides + 1)|
    let mut dist_groups: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for face in 1..=sides {
        let dist_twice = (2 * face - (sides + 1)).abs();
        dist_groups.entry(dist_twice).or_default().push(face);
    }

    let ordered_buckets: Vec<&Vec<i32>> = match mode {
        CenterDistanceMode::Closest => dist_groups.values().collect(),
        CenterDistanceMode::Furthest => dist_groups.values().rev().collect(),
    };

    let mut out = BTreeMap::new();
    let mut remaining_faces = sides as f64;
    let sides_f = sides as f64;
    let n_exp = pool_size as i32;

    for faces_in_bucket in ordered_buckets {
        let b = faces_in_bucket.len() as f64;
        let p_bucket =
            (remaining_faces / sides_f).powi(n_exp) - ((remaining_faces - b) / sides_f).powi(n_exp);
        for &f in faces_in_bucket {
            out.insert(f, p_bucket / b);
        }
        remaining_faces -= b;
    }

    normalize_pmf(&out)
}

// ---------------------------------------------------------------------------
// 6. Median-Gated Branching Pool (True Two-Humped Bimodal Distribution)
// ---------------------------------------------------------------------------

fn factorial_u64(n: usize) -> u64 {
    (1..=(n as u64)).product::<u64>().max(1)
}

/// Fallible version of [`median_gated_bimodal_pmf`] validating that `pool_size` is an odd integer `>= 3`.
pub fn try_median_gated_bimodal_pmf(
    pool_size: usize,
    sides: i32,
    gate_tn: i32,
    roll_under: bool,
) -> Result<Pmf, String> {
    if pool_size.is_multiple_of(2) || pool_size < 3 {
        return Err(format!(
            "pool_size must be an odd integer >= 3 (e.g., 3 or 5), got {pool_size}"
        ));
    }
    if sides <= 0 {
        return Err(format!("sides must be >= 1, got {sides}"));
    }

    let mut out: BTreeMap<i32, f64> = BTreeMap::new();
    let mid_idx = pool_size / 2;
    let total_perms = (sides as f64).powi(pool_size as i32);
    let n_fact = factorial_u64(pool_size);

    #[allow(clippy::too_many_arguments)]
    fn recurse(
        face: i32,
        sides: i32,
        mid_idx: usize,
        gate_tn: i32,
        roll_under: bool,
        n_fact: u64,
        total_perms: f64,
        remaining: usize,
        current_tuple: &mut Vec<i32>,
        mult_denom: u64,
        out: &mut BTreeMap<i32, f64>,
    ) {
        if remaining == 0 {
            let perm_count = (n_fact / mult_denom) as f64;
            let prob = perm_count / total_perms;
            let mn = current_tuple[0];
            let md = current_tuple[mid_idx];
            let mx = *current_tuple.last().unwrap_or(&mn);
            let chosen = if roll_under {
                if md <= gate_tn { mn } else { mx }
            } else if md >= gate_tn {
                mx
            } else {
                mn
            };
            *out.entry(chosen).or_insert(0.0) += prob;
            return;
        }
        if face > sides {
            return;
        }
        // Option 0: 0 dice show `face`
        recurse(
            face + 1,
            sides,
            mid_idx,
            gate_tn,
            roll_under,
            n_fact,
            total_perms,
            remaining,
            current_tuple,
            mult_denom,
            out,
        );
        // Option c >= 1 dice show `face`
        for c in 1..=remaining {
            for _ in 0..c {
                current_tuple.push(face);
            }
            recurse(
                face + 1,
                sides,
                mid_idx,
                gate_tn,
                roll_under,
                n_fact,
                total_perms,
                remaining - c,
                current_tuple,
                mult_denom * factorial_u64(c),
                out,
            );
            for _ in 0..c {
                current_tuple.pop();
            }
        }
    }

    let mut current_tuple = Vec::with_capacity(pool_size);
    recurse(
        1,
        sides,
        mid_idx,
        gate_tn,
        roll_under,
        n_fact,
        total_perms,
        pool_size,
        &mut current_tuple,
        1,
        &mut out,
    );
    Ok(normalize_pmf(&out))
}

/// Roll an odd number of dice `pool_size` `dK` (`>= 3`) and branch on the median die vs. `gate_tn`:
/// - If `roll_under == false`: if `median >= gate_tn`, return `max(pool)`; else return `min(pool)`.
/// - If `roll_under == true`: if `median <= gate_tn`, return `min(pool)`; else return `max(pool)`.
///
/// # Panics
/// Panics if `pool_size` is even or `< 3`.
pub fn median_gated_bimodal_pmf(
    pool_size: usize,
    sides: i32,
    gate_tn: i32,
    roll_under: bool,
) -> Pmf {
    try_median_gated_bimodal_pmf(pool_size, sides, gate_tn, roll_under)
        .expect("invalid median_gated_bimodal_pmf arguments")
}

// ---------------------------------------------------------------------------
// 7. Multiplicative Dice Pool (Log-Normal Heavy-Tailed Distribution)
// ---------------------------------------------------------------------------

/// Exact [`Pmf`] of multiplying `pool_size` `dK` together ($d_1 \times d_2 \times \dots \times d_N$).
pub fn multiplicative_pool_pmf(pool_size: usize, sides: i32) -> Pmf {
    if sides <= 0 {
        return BTreeMap::new();
    }
    let mut result = BTreeMap::from([(1, 1.0)]);
    let p = 1.0 / (sides as f64);
    for _ in 0..pool_size {
        let mut nxt: BTreeMap<i32, f64> = BTreeMap::new();
        for (&val, &prob) in &result {
            for face in 1..=sides {
                *nxt.entry(val * face).or_insert(0.0) += prob * p;
            }
        }
        result = nxt;
    }
    normalize_pmf(&result)
}

// ---------------------------------------------------------------------------
// 8. Paired-List Cascading / Contagious Dice (The Watershed Mechanic)
// ---------------------------------------------------------------------------

/// Carry-over propagation rule governing how `mod_{n+1}` is calculated from step `n` in the Two-List Cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CarryMode {
    /// 1-step Markov memory (`mod_{n+1}` depends only on whether step `n` rolled over or under `C`).
    #[default]
    MarkovStep,
    /// Compounding momentum (`mod_{n+1} = mod_n + step_mod`).
    CumulativeSnowball,
    /// Exact difference carry (`mod_{n+1} = Y_n - C`, or `C - Y_n` when `invert_feedback = true`).
    LinearMargin,
}

impl CarryMode {
    /// Canonical snake_case identifier matching Python's `Literal["markov_step", "cumulative_snowball", "linear_margin"]`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MarkovStep => "markov_step",
            Self::CumulativeSnowball => "cumulative_snowball",
            Self::LinearMargin => "linear_margin",
        }
    }

    /// Computes the carry-over modifier $\text{mod}_{n+1}$ propagated from step $n$.
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

impl fmt::Display for CarryMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CarryMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "markov_step" => Ok(Self::MarkovStep),
            "cumulative_snowball" => Ok(Self::CumulativeSnowball),
            "linear_margin" => Ok(Self::LinearMargin),
            other => Err(format!("Unknown CarryMode: '{other}'")),
        }
    }
}

/// Botch / complication evaluation rule for [`paired_list_signed_margin_pmfs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BotchRule {
    /// Monotonic / Anti-Paradox rule: when `successes == 0`, failure severity is determined by the closest pair
    /// to `C_eff` across all `N` pairs, binned every `max(2, K / 5)` points (`0..=5`).
    #[default]
    ClosestMiss,
    /// Physical dice bound breach: `Y_n > 2K` (in Roll-Under) or `Y_n < 2` (in Roll-Over).
    CeilingBreach,
    /// Relative overrun: `Y_n > C_eff + K` (in Roll-Under) or `Y_n < C_eff - K` (in Roll-Over).
    OverrunK,
}

impl BotchRule {
    /// Canonical snake_case identifier matching Python's `Literal["closest_miss", "ceiling_breach", "overrun_k"]`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ClosestMiss => "closest_miss",
            Self::CeilingBreach => "ceiling_breach",
            Self::OverrunK => "overrun_k",
        }
    }

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

impl fmt::Display for BotchRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BotchRule {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "closest_miss" => Ok(Self::ClosestMiss),
            "ceiling_breach" => Ok(Self::CeilingBreach),
            "overrun_k" => Ok(Self::OverrunK),
            other => Err(format!("Unknown BotchRule: '{other}'")),
        }
    }
}

/// Rule for combining total successes and botches into the unified `-N..=+N` signed margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MarginAggregation {
    /// If `successes > 0`, margin is `+successes`; if `successes == 0`, margin is `-eff_botch`.
    #[default]
    PiecewiseZeroFloor,
    /// Margin is `successes - eff_botch` across the whole chain.
    NetDifference,
}

impl MarginAggregation {
    /// Canonical snake_case identifier matching Python's `Literal["piecewise_zero_floor", "net_difference"]`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PiecewiseZeroFloor => "piecewise_zero_floor",
            Self::NetDifference => "net_difference",
        }
    }

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

impl fmt::Display for MarginAggregation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MarginAggregation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "piecewise_zero_floor" => Ok(Self::PiecewiseZeroFloor),
            "net_difference" => Ok(Self::NetDifference),
            other => Err(format!("Unknown MarginAggregation: '{other}'")),
        }
    }
}

/// Computes exact PMFs `(success_count_pmf, final_pair_value_pmf)` for the Two-List Cascading
/// ("Watershed" or "Recoil") Dice Mechanic.
///
/// Two lists of `num_pairs` dice (`sides`-sided, `dK`) are rolled in parallel:
/// - At each index $n \in \{1, \dots, N\}$, the base pair sum is $X_n = A_n + B_n \in [2, 2K]$.
/// - The effective value is $Y_n = X_n + \text{mod}_n$ (where $\text{mod}_1 = 0$).
/// - $Y_n$ is compared to `characteristic_c` to determine whether step $n$ succeeded and what
///   modifier $\text{mod}_{n+1}$ propagates to step $n+1$.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedCascadeResult {
    /// Unified PMF over `-num_pairs..=+num_pairs`:
    /// - `> 0`: Margin of Success (`+1` = bare minimum, `+2..=+N` = higher degrees of success).
    /// - `0`: Clean Failure (`0` successes, `0` botches).
    /// - `< 0`: Degree of Failure (`-1` = minor setback, `-2` = severe, `-3..=-N` = catastrophic).
    pub signed_margin_pmf: Pmf,
    /// PMF of raw success counts `0..=num_pairs`.
    pub success_count_pmf: Pmf,
    /// PMF of effective botch/overrun counts.
    pub botch_count_pmf: Pmf,
    /// PMF of the final pair's effective value $Y_N$.
    pub final_pair_value_pmf: Pmf,
    /// Conditional PMF of $Y_N - C_{\text{eff}}$ (or $C_{\text{eff}} - Y_N$ in Roll-Over) given `successes == 0`.
    pub failure_overshoot_pmf: Pmf,
    /// Total probability of achieving at least 1 success (`successes >= 1`).
    pub prob_any_success: f64,
    /// Probability of succeeding while also triggering at least 1 floor/ceiling breach (`successes >= 1 && botches >= 1`).
    pub prob_success_with_complication: f64,
    /// Probability of failing with 0 botches (`successes == 0 && eff_botch == 0`).
    pub prob_clean_failure: f64,
    /// Probability of failing with 1 or more botches (`successes == 0 && eff_botch >= 1`).
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
    if count_under_as_success {
        let succ = i32::from(y_val < characteristic_c);
        let miss_dist = (y_val - characteristic_c).max(0);
        let mut botch = match botch_rule {
            BotchRule::CeilingBreach | BotchRule::ClosestMiss => i32::from(y_val > 2 * sides),
            BotchRule::OverrunK => i32::from(y_val > characteristic_c + sides),
        };
        if succ == 1 {
            botch = 0;
        }
        (succ, botch, miss_dist)
    } else {
        let succ = i32::from(y_val > characteristic_c);
        let miss_dist = (characteristic_c - y_val).max(0);
        let mut botch = match botch_rule {
            BotchRule::CeilingBreach | BotchRule::ClosestMiss => i32::from(y_val < 2),
            BotchRule::OverrunK => i32::from(y_val < characteristic_c - sides),
        };
        if succ == 1 {
            botch = 0;
        }
        (succ, botch, miss_dist)
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn run_cascade_dp(
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
/// as specified in `docs/ttrpg/probability.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeTaskParams {
    /// 1. Skill Level $N \in [1, 10]$ (number of `2dK` pairs rolled).
    pub skill_level: usize,
    /// 2. Controlling Characteristic $C \in [1, 20]$ (human playable range `4..=18`, average `10`).
    pub control_c: i32,
    /// 3. Internal Character Modifier $\Delta_{\text{int}} \in [-2, +10]$ (Fatigue, Injury, Stress; added to `base_delta_under`).
    pub internal_mod: i32,
    /// 4. External Task Modifier $\Delta_{\text{ext}} \in [-10, +16]$ (Range, Darkness, Tool Quality; shifts $\text{TN}_{\text{eff}}$).
    pub external_mod: i32,
    /// Die size $K$ (default `20` for `2d20`).
    pub sides: i32,
    /// Positive momentum bonus $+\delta_{\text{over}}$ on a successful pair (default `6` for `K=20`).
    pub delta_over: i32,
    /// Unfatigued baseline failure penalty $\delta_{\text{base}}$ (default `6` for `K=20`).
    pub base_delta_under: i32,
    /// Carry-over mode (`MarkovStep` standard resolution vs. `CumulativeSnowball` high-stakes resolution).
    pub carry_mode: CarryMode,
    /// Botch evaluation rule (default `ClosestMiss`).
    pub botch_rule: BotchRule,
    /// Margin aggregation mode (default `PiecewiseZeroFloor`).
    pub margin_aggregation: MarginAggregation,
}

impl Default for CascadeTaskParams {
    fn default() -> Self {
        Self::new_k20(3, 10, 0, 0)
    }
}

impl CascadeTaskParams {
    /// Constructs a canonical `K = 20` (`2d20`) Four-Factor Task configuration per `docs/ttrpg/probability.md`:
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

    /// Computes the exact probability distribution ([`SignedCascadeResult`]) for this task configuration.
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

    /// Evaluates a concrete sequence of rolled `(die_a, die_b)` pairs against this task configuration.
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

    /// Generates `self.skill_level` pairs using `roll_die(sides) -> 1..=sides` and resolves the cascade.
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

/// High-level narrative outcome category of a resolved Two-List Cascade roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CascadeOutcomeKind {
    /// At least 1 success and 0 floor-breach complications (`Succ >= 1 && Botch == 0`).
    Success,
    /// At least 1 success, but also triggered at least 1 floor-breach complication (`Succ >= 1 && Botch >= 1`).
    SuccessWithComplication,
    /// Zero successes and zero effective botches (`Succ == 0 && EffBotch == 0`).
    CleanFailure,
    /// Zero successes and at least 1 effective botch (`Succ == 0 && EffBotch >= 1`).
    BotchFailure,
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

    /// Maps this outcome classification and `total_successes` to the canonical 5-tier [`OutcomeTier`].
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeStepTrace {
    /// 1-based pair index ($n \in 1..=N$).
    pub pair_index: usize,
    /// Die outcome from List A ($A_n \in 1..=K$).
    pub die_a: i32,
    /// Die outcome from List B ($B_n \in 1..=K$).
    pub die_b: i32,
    /// Unmodified pair sum $X_n = A_n + B_n$.
    pub raw_sum: i32,
    /// Carry-over modifier entering step $n$ ($\text{Carry}_1 = 0$).
    pub carry_in: i32,
    /// Effective pair total $Y_n = X_n + \text{Carry}_n$.
    pub effective_y: i32,
    /// Target number $\text{TN}_{\text{eff}}$ compared against $Y_n$.
    pub tn_eff: i32,
    /// Whether this pair scored a success.
    pub succeeded: bool,
    /// Whether this pair triggered a floor/ceiling breach botch.
    pub step_botch: bool,
    /// Non-negative miss distance from $\text{TN}_{\text{eff}}$ (`0` on success or exact tie).
    pub miss_distance: i32,
    /// Carry-over modifier propagated to step $n + 1$.
    pub carry_out: i32,
}

/// Complete game-engine resolution of a rolled Two-List Cascade check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeRollOutcome {
    /// Ordered trace of each evaluated pair $(A_n, B_n)$ for $n = 1..=N$.
    pub steps: Vec<CascadeStepTrace>,
    /// Total number of pairs that succeeded (`0..=N`).
    pub total_successes: i32,
    /// Total raw floor/ceiling breach botches across all steps (`0..=N`).
    pub raw_botches: i32,
    /// Effective botch severity (`min_miss_tier` when `botch_rule == ClosestMiss && total_successes == 0`, else `raw_botches`).
    pub effective_botches: i32,
    /// Best (minimum) miss tier across all pairs (`0..=5`).
    pub min_miss_tier: i32,
    /// Unified signed margin in `-N..=+N`.
    pub signed_margin: i32,
    /// Effective value $Y_N$ of the final pair in the chain.
    pub final_pair_y: i32,
    /// Four-way cascade outcome classification (`Success`, `SuccessWithComplication`, `CleanFailure`, `BotchFailure`).
    pub outcome_kind: CascadeOutcomeKind,
    /// Canonical [`OutcomeTier`] mapping for `TtrpgMechanicsEvent` telemetry.
    pub outcome_tier: OutcomeTier,
}

/// Evaluates a concrete list of rolled `(A_n, B_n)` dice pairs using the exact Two-List Cascading
/// ("Watershed" / "Recoil") state machine.
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
fn build_cascade_mechanics_event(
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

/// Evaluates a Two-List Cascading ("Watershed") `2d20` task roll per `docs/ttrpg/probability.md`
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

// ---------------------------------------------------------------------------
// 10. Contested & Hybrid-Contested Two-List Cascade Resolution Engine
// ---------------------------------------------------------------------------

/// Tie-breaking and margin-interaction rule for Contested Cascade Task Resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContestedTieRule {
    /// Recommended Hybrid Rule:
    /// - Clean Win (`Success` / `ExtraordinarySuccess`): $S_P > S_{GM}$ and $B_P == 0$ (or GM botch cancels $B_P$).
    /// - Grazing Win (`PartialSuccess`): Positive tie $S_P == S_{GM} \ge 1$ succeeds at a minor cost/delay.
    /// - Lucky Break (`PartialSuccess`): Player rolls a Clean Miss ($S_P == 0, M_P == 0$) while the GM Botches ($M_{GM} < 0$).
    /// - Floor-Breach Win (`PartialSuccess`): $S_P > S_{GM}$ with $B_P \ge 1$ and $M_{GM} \ge 0$.
    #[default]
    HybridSignedMargin,
    /// Strict Signed Margin ($M_P > M_{GM}$):
    /// Positive ties ($S_P == S_{GM} \ge 1$) count as `Failure` (stalemate/blocked), while a GM Botch ($M_{GM} < 0$)
    /// against a player Clean Miss ($M_P == 0$) still triggers a Lucky Break (`PartialSuccess`).
    StrictSignedMargin,
    /// Active Player Wins Positive Ties ($S_P \ge S_{GM} \ge 1$):
    /// Positive ties count as a full `Success` (no grazing complication), and GM botches do not rescue $S_P == 0$.
    ActivePlayerWinsTies,
}

impl ContestedTieRule {
    /// Canonical string identifier.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::HybridSignedMargin => "hybrid_signed_margin",
            Self::StrictSignedMargin => "strict_signed_margin",
            Self::ActivePlayerWinsTies => "active_player_wins_ties",
        }
    }
}

impl fmt::Display for ContestedTieRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ContestedTieRule {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "hybrid_signed_margin" => Ok(Self::HybridSignedMargin),
            "strict_signed_margin" => Ok(Self::StrictSignedMargin),
            "active_player_wins_ties" => Ok(Self::ActivePlayerWinsTies),
            other => Err(format!("Unknown ContestedTieRule: '{other}'")),
        }
    }
}

/// Specific sub-classification of how a Contested Cascade roll resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContestedOutcomeKind {
    /// Player strictly beat GM ($S_P > S_{GM}$) with net margin $\Delta M \ge 3$ and no uncancelled floor breach.
    ExtraordinarySuccess,
    /// Player cleanly beat GM ($S_P > S_{GM}$) with no uncancelled floor breach.
    CleanSuccess,
    /// Player beat GM ($S_P > S_{GM}$), but suffered a floor-breach complication ($B_P \ge 1$).
    SuccessWithFloorBreach,
    /// Player tied GM on positive successes ($S_P == S_{GM} \ge 1$); succeeds at a grazing cost or delay.
    GrazingTieSuccess,
    /// Player rolled a Clean Failure ($S_P == 0, M_P == 0$), but the GM botched ($M_{GM} < 0$), creating an opportunistic opening.
    LuckyBreakSuccess,
    /// Player failed to beat GM without botching ($M_P \ge 0$, or stalemate $0 == 0$).
    CleanFailure,
    /// Player botched ($M_P < 0$) or failed while suffering a floor breach.
    BotchFailure,
}

impl ContestedOutcomeKind {
    /// Canonical `SCREAMING_SNAKE_CASE` string identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExtraordinarySuccess => "EXTRAORDINARY_SUCCESS",
            Self::CleanSuccess => "CLEAN_SUCCESS",
            Self::SuccessWithFloorBreach => "SUCCESS_WITH_FLOOR_BREACH",
            Self::GrazingTieSuccess => "GRAZING_TIE_SUCCESS",
            Self::LuckyBreakSuccess => "LUCKY_BREAK_SUCCESS",
            Self::CleanFailure => "CLEAN_FAILURE",
            Self::BotchFailure => "BOTCH_FAILURE",
        }
    }

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

impl fmt::Display for ContestedOutcomeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Mode for evaluating the GM's opposing roll in a Contested Task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GmResistanceMode {
    /// **1-Pair Tiered GM Resistance Roll (Default for Environmental / Technical Tasks)**:
    /// The GM rolls a single hidden `2d20` pair ($X_{GM} = A_{GM} + B_{GM}$) against $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$:
    /// - If $X_{GM} > \text{TN}_{GM}$, scores $S_{GM} = 1 + \lfloor (X_{GM} - \text{TN}_{GM} - 1) / \text{step\_size} \rfloor$ Resistance Successes (`step_size = 4` by default).
    /// - If $X_{GM} \le \text{TN}_{GM}$, scores $S_{GM} = 0$ successes and $B_{GM} = \min(5, \lfloor (\text{TN}_{GM} - X_{GM}) / 4 \rfloor)$ botch tiers ($M_{GM} = -B_{GM}$).
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
    /// Constructs a **1-Pair Tiered GM Resistance Environmental Task** (with canonical `4`-point over-tiers) where:
    /// - The Player rolls `player_skill_n` pairs against their known personal target $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$.
    /// - The GM secretly rolls `1` `2d20` pair against $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$ ($C_{GM} = 10 + \Delta_{\text{ext}}$),
    ///   scoring `1` success plus `+1` per `4` points above $\text{TN}_{GM}$ (or `-1` botch per `4` points below $\text{TN}_{GM}$).
    pub const fn new_tiered_environment(
        player_skill_n: usize,
        player_control_c: i32,
        player_internal_mod: i32,
        player_known_tool_mod: i32,
        hidden_external_difficulty: i32,
        tie_rule: ContestedTieRule,
    ) -> Self {
        let player = CascadeTaskParams::new_k20(
            player_skill_n,
            player_control_c,
            player_internal_mod,
            player_known_tool_mod,
        );
        let gm_control_c = 10 + hidden_external_difficulty;
        let gm = CascadeTaskParams::new_k20(1, gm_control_c, 0, 0);
        Self {
            player,
            gm,
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
        let player = CascadeTaskParams::new_k20(
            player_skill_n,
            player_control_c,
            player_internal_mod,
            player_known_tool_mod,
        );
        let gm_control_c = 10 + hidden_external_difficulty;
        let gm = CascadeTaskParams::new_k20(gm_pairs, gm_control_c, 0, 0);
        Self {
            player,
            gm,
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
        let net_successes = p_out.total_successes - g_out.total_successes;
        ContestedCascadeRollOutcome {
            player_outcome: p_out,
            gm_outcome: g_out,
            net_successes,
            net_signed_margin: net_margin,
            contested_kind,
            outcome_tier: contested_kind.to_outcome_tier(),
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
    match tie_rule {
        ContestedTieRule::HybridSignedMargin => {
            if sp > sg {
                // Player has strictly more successes than GM.
                // If player had a floor breach (bp >= 1), it causes a complication UNLESS GM also botched (mg < 0).
                if bp >= 1 && mg >= 0 {
                    (ContestedOutcomeKind::SuccessWithFloorBreach, net_margin)
                } else if net_margin >= 3 {
                    (ContestedOutcomeKind::ExtraordinarySuccess, net_margin)
                } else {
                    (ContestedOutcomeKind::CleanSuccess, net_margin)
                }
            } else if sp == sg && sp >= 1 {
                // Positive Tie: Player and GM achieved the same number of positive successes.
                (ContestedOutcomeKind::GrazingTieSuccess, net_margin)
            } else if sp == 0 && mp == 0 && mg < 0 {
                // Player had a Clean Failure (0 successes, 0 botches), but GM botched!
                (ContestedOutcomeKind::LuckyBreakSuccess, net_margin)
            } else if mp < 0 || (sp < sg && bp >= 1) {
                (ContestedOutcomeKind::BotchFailure, net_margin)
            } else {
                (ContestedOutcomeKind::CleanFailure, net_margin)
            }
        }
        ContestedTieRule::StrictSignedMargin => {
            if sp > sg {
                if bp >= 1 && mg >= 0 {
                    (ContestedOutcomeKind::SuccessWithFloorBreach, net_margin)
                } else if net_margin >= 3 {
                    (ContestedOutcomeKind::ExtraordinarySuccess, net_margin)
                } else {
                    (ContestedOutcomeKind::CleanSuccess, net_margin)
                }
            } else if sp == 0 && mp == 0 && mg < 0 {
                (ContestedOutcomeKind::LuckyBreakSuccess, net_margin)
            } else if mp < 0 || (sp <= sg && bp >= 1) {
                (ContestedOutcomeKind::BotchFailure, net_margin)
            } else {
                (ContestedOutcomeKind::CleanFailure, net_margin)
            }
        }
        ContestedTieRule::ActivePlayerWinsTies => {
            if sp >= sg && sp >= 1 {
                if bp >= 1 {
                    (ContestedOutcomeKind::SuccessWithFloorBreach, net_margin)
                } else if (sp - sg) >= 3 {
                    (ContestedOutcomeKind::ExtraordinarySuccess, net_margin)
                } else {
                    (ContestedOutcomeKind::CleanSuccess, net_margin)
                }
            } else if mp < 0 || bp >= 1 {
                (ContestedOutcomeKind::BotchFailure, net_margin)
            } else {
                (ContestedOutcomeKind::CleanFailure, net_margin)
            }
        }
    }
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
        let s = 1 + (raw_sum - tn_gm - 1) / over_step;
        (s, 0, 0)
    } else {
        let dist = tn_gm - raw_sum;
        let b = (dist / under_step).min(5);
        (0, b, dist)
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
pub fn compute_contested_cascade_odds(
    player: &CascadeTaskParams,
    gm: &CascadeTaskParams,
    tie_rule: ContestedTieRule,
) -> ContestedCascadeOdds {
    compute_contested_cascade_odds_with_mode(player, gm, GmResistanceMode::FullCascade, tie_rule)
}

/// Computes exact contested probabilities ([`ContestedCascadeOdds`]) by convolving the joint `(succ, raw_botch, signed_margin)`
/// state distributions of `player` and `gm` under `gm_mode` (`TieredSinglePair` or `FullCascade`).
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
    let net_margin_stats = compute_stats(&net_signed_margin_pmf);
    let denom = net_margin_stats.excess_kurtosis + 3.0;
    let bimodality_coefficient = if denom > 1e-9 {
        (net_margin_stats.skewness.powi(2) + 1.0) / denom
    } else {
        0.0
    };

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
        net_margin_stats,
        bimodality_coefficient,
    }
}

/// Evaluates an Always-Contested Two-List Cascading (`2d20`) task roll per
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` and returns both the
/// [`TtrpgMechanicsEvent`] telemetry record (including §5 Tick Cost duration modifiers for
/// `ExtraordinarySuccess` and `GrazingTieSuccess`) and the [`ContestedCascadeRollOutcome`].
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
