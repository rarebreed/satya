//! Classical and exotic dice pool probability mass functions ([`Pmf`]), summary statistics
//! ([`DistributionStats`]), and exact combinatorial pool generators.
//!
//! # TTRPG Rules Origins
//! - `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1–§2.2)
//! - `docs/ttrpg/01-foundation/design_philosophy.md` (§5.4)
//! - `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1)
//! - `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§3.2)

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

/// Discrete probability mass function mapping outcome integer values to their exact probabilities in `[0.0, 1.0]`.
///
/// Stored in a [`BTreeMap`] so keys are always sorted in ascending order.
///
/// Origin: `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1) and
/// `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
pub type Pmf = BTreeMap<i32, f64>;

/// Summary moments of a discrete probability distribution.
///
/// Origin: `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2),
/// `docs/ttrpg/01-foundation/design_philosophy.md` (§5.4), and
/// `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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

impl DistributionStats {
    /// Sarle's uniform-distribution bimodality threshold $\text{BC}_{\text{uniform}} = 5/9 \approx 0.5556$
    /// (`docs/ttrpg/01-foundation/mathematical_notation_glossary.md` §2.2).
    pub const SARLE_BIMODAL_THRESHOLD: f64 = 5.0 / 9.0;

    /// Computes Sarle's Bimodality Coefficient $\text{BC} = (\gamma_1^2 + 1) / (\kappa_2 + 3)$
    /// (`docs/ttrpg/01-foundation/mathematical_notation_glossary.md` §2.2,
    /// `docs/ttrpg/01-foundation/design_philosophy.md` §5.4).
    #[must_use]
    pub fn bimodality_coefficient(&self) -> f64 {
        (self.skewness.powi(2) + 1.0) / ((self.excess_kurtosis + 3.0).max(1e-9))
    }
}

/// Sarle's uniform-distribution bimodality threshold $\text{BC}_{\text{uniform}} = 5/9 \approx 0.5556$
/// (`docs/ttrpg/01-foundation/mathematical_notation_glossary.md` §2.2).
pub const SARLE_BIMODAL_THRESHOLD: f64 = DistributionStats::SARLE_BIMODAL_THRESHOLD;

pub(crate) fn normalize_pmf_iter<'a, I>(entries: I) -> Pmf
where
    I: IntoIterator<Item = (&'a i32, &'a f64)> + Clone,
{
    let total: f64 = entries.clone().into_iter().map(|(_, &v)| v).sum();
    if total <= 0.0 {
        return BTreeMap::new();
    }
    entries
        .into_iter()
        .filter_map(|(&k, &v)| (v > 1e-15).then_some((k, v / total)))
        .collect()
}

/// Returns a sorted, normalized probability mass function with negligible entries (`<= 1e-15`) removed.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
pub fn normalize_pmf(pmf: &BTreeMap<i32, f64>) -> Pmf {
    normalize_pmf_iter(pmf)
}

pub(crate) fn normalize_hash_pmf(pmf: &HashMap<i32, f64>) -> Pmf {
    normalize_pmf_iter(pmf)
}

/// Computes exact mean, variance, standard deviation, skewness, and excess kurtosis for a [`Pmf`].
///
/// Origin: `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2) and
/// `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
        .map(|(&x, &p)| ((x as f64) - mean).powi(2) * p)
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

    let (m3, m4) = norm.iter().fold((0.0, 0.0), |(acc3, acc4), (&x, &p)| {
        let diff = (x as f64) - mean;
        (acc3 + diff.powi(3) * p, acc4 + diff.powi(4) * p)
    });

    DistributionStats {
        mean,
        variance,
        std,
        skewness: m3 / std.powi(3),
        excess_kurtosis: (m4 / variance.powi(2)) - 3.0,
    }
}

/// Computes the exact distribution of $X + Y$ for independent discrete random variables $X$ and $Y$.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
pub fn single_die_pmf(sides: i32, negative: bool) -> Pmf {
    if sides <= 0 {
        return BTreeMap::new();
    }
    let p = 1.0 / (sides as f64);
    let sign = if negative { -1 } else { 1 };
    (1..=sides).map(|face| (sign * face, p)).collect()
}

// ---------------------------------------------------------------------------
// 1. Additive & Negative Dice Pools (Central Limit Theorem Demonstration)
// ---------------------------------------------------------------------------

/// Exact [`Pmf`] of the sum of `pos_dice` `dK` minus `neg_dice` `dK`.
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
// 2. Success-Counting Dice Pools (Binomial & Comparative Roll-Under 2d20 Critical-1 Pool)
// ---------------------------------------------------------------------------

/// Exact [`Pmf`] of success counts in an `NdK` pool.
///
/// If `crit_on_one_double` is `true`, rolling a `1` on a hit counts as `2` successes—modeling classic
/// roll-under `2d20` critical-`1` success-counting pools for comparative distribution analysis
/// (`docs/ttrpg/01-foundation/probability_and_calibration.md` §1), whereas Satya itself is a Roll-Over
/// `2d20` Two-List Cascade system (`docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` §3.2).
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
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
pub fn center_distance_selection_pmf(
    pool_size: usize,
    sides: i32,
    mode: CenterDistanceMode,
) -> Pmf {
    if sides <= 0 || pool_size == 0 {
        return BTreeMap::new();
    }
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

struct MedianGateCtx {
    sides: i32,
    mid_idx: usize,
    gate_tn: i32,
    roll_under: bool,
    n_fact: u64,
    total_perms: f64,
    current_tuple: Vec<i32>,
    out: BTreeMap<i32, f64>,
}

impl MedianGateCtx {
    fn recurse(&mut self, face: i32, remaining: usize, mult_denom: u64) {
        if remaining == 0 {
            let prob = ((self.n_fact / mult_denom) as f64) / self.total_perms;
            let mn = self.current_tuple[0];
            let md = self.current_tuple[self.mid_idx];
            let mx = *self.current_tuple.last().unwrap_or(&mn);
            let chosen = if self.roll_under {
                if md <= self.gate_tn { mn } else { mx }
            } else if md >= self.gate_tn {
                mx
            } else {
                mn
            };
            *self.out.entry(chosen).or_insert(0.0) += prob;
            return;
        }
        if face > self.sides {
            return;
        }
        self.recurse(face + 1, remaining, mult_denom);
        for c in 1..=remaining {
            self.current_tuple.resize(self.current_tuple.len() + c, face);
            self.recurse(face + 1, remaining - c, mult_denom * factorial_u64(c));
            self.current_tuple.truncate(self.current_tuple.len() - c);
        }
    }
}

/// Fallible version of [`median_gated_bimodal_pmf`] validating that `pool_size` is an odd integer `>= 3`.
///
/// Origin: `docs/ttrpg/01-foundation/design_philosophy.md` (§5.4) and
/// `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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

    let mut ctx = MedianGateCtx {
        sides,
        mid_idx: pool_size / 2,
        gate_tn,
        roll_under,
        n_fact: factorial_u64(pool_size),
        total_perms: (sides as f64).powi(pool_size as i32),
        current_tuple: Vec::with_capacity(pool_size),
        out: BTreeMap::new(),
    };
    ctx.recurse(1, pool_size, 1);
    Ok(normalize_pmf(&ctx.out))
}

/// Roll an odd number of dice `pool_size` `dK` (`>= 3`) and branch on the median die vs. `gate_tn`:
/// - If `roll_under == false`: if `median >= gate_tn`, return `max(pool)`; else return `min(pool)`.
/// - If `roll_under == true`: if `median <= gate_tn`, return `min(pool)`; else return `max(pool)`.
///
/// Origin: `docs/ttrpg/01-foundation/design_philosophy.md` (§5.4) and
/// `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
///
/// Origin: `docs/ttrpg/01-foundation/probability_and_calibration.md` (§1).
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
