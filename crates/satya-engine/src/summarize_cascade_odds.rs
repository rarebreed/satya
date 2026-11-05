//! Odds summary table generator and calibration sweeps for Satya's Two-List Cascading Dice Mechanic.
//!
//! Ported from `src/satya_ml/summarize_cascade_odds.py` and calibrated per `docs/ttrpg/probability.md`.
//!
//! Summarizes exact probabilities of:
//! - At least 1 Success (`P(Succ >= 1)`)
//! - Botch Failure (`P(Succ == 0 & Botch >= 1)`)
//! - Success with a Botch / Complication (`P(Succ >= 1 & Botch >= 1)`)
//! - Clean Failure (`P(Succ == 0 & Botch == 0)`)
//! - Expected Signed Margin (`E[Margin]`)
//!
//! across die sizes $K \in \{10, 20, 100\}$, Controlling Characteristic $C$, Skill Levels $N$,
//! carry-over modes, botch rules, and margin aggregation modes.

use crate::dice_probabilities::{
    BotchRule, CarryMode, CascadeTaskParams, ContestedCascadeTaskParams, ContestedTieRule,
    MarginAggregation, compute_stats, paired_list_signed_margin_pmfs,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// Single row of the cascade odds summary table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CascadeSummaryRow {
    /// Die size $K$ (e.g., `10`, `20`, `100`).
    pub k_sides: i32,
    /// Controlling Characteristic $C$ (`1..=K`).
    pub control_c: i32,
    /// Effective roll-over target number $\text{TN}_{\text{eff}}$ on `2dK`.
    pub tn_eff: i32,
    /// Skill Level $N$ (number of `2dK` pairs rolled).
    pub skill_level: usize,
    /// Positive carry-over bonus $+\delta_{\text{over}}$ on a successful pair.
    pub delta_over: i32,
    /// Negative carry-over penalty magnitude $\delta_{\text{under}}$ on a missed pair.
    pub delta_under: i32,
    /// Carry-over propagation rule (`markov_step`, `cumulative_snowball`, `linear_margin`).
    pub carry_mode: CarryMode,
    /// Botch evaluation rule (`closest_miss`, `ceiling_breach`, `overrun_k`).
    pub botch_rule: BotchRule,
    /// Margin aggregation rule (`piecewise_zero_floor`, `net_difference`).
    pub margin_mode: MarginAggregation,
    /// Percentage probability (`0.0..=100.0`) of achieving at least 1 success (`P(Succ >= 1)`).
    pub pct_any_success: f64,
    /// Percentage probability (`0.0..=100.0`) of failing with at least 1 botch (`P(BotchFail)`).
    pub pct_botch_failure: f64,
    /// Percentage probability (`0.0..=100.0`) of succeeding while also triggering a botch (`P(Succ+Botch)`).
    pub pct_succ_and_botch: f64,
    /// Percentage probability (`0.0..=100.0`) of failing with 0 botches (`P(CleanFail)`).
    pub pct_clean_failure: f64,
    /// Expected value $E[\text{Margin}]$ of the `-N..=+N` signed margin distribution.
    pub mean_margin: f64,
}

/// Target number calibration mode when $C = K / 2$ and pairs are `2dK` (range `2..=2K`, mean `K + 1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TnCalibration {
    /// Recommended midpoint calibration: $\text{TN}_{\text{eff}} = (2K + 1) - 2C + \Delta_{\text{ext}}$
    /// (equals $K + 1$, the exact mean of `2dK`, when $C = K/2$ and $\Delta_{\text{ext}} = 0$).
    #[default]
    Midpoint2c,
    /// Unscaled single-$C$ calibration: $\text{TN}_{\text{eff}} = (2K + 2) - C + \Delta_{\text{ext}}$
    /// (equals $1.5K + 2$ when $C = K/2$).
    Raw2kMinusC,
}

/// Target number formula selector for $K = 20$ Control vs. Skill 2D matrices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TnFormulaK20 {
    /// Midpoint-anchored single-step slope: $\text{TN}_{\text{eff}} = 31 - C$.
    #[default]
    #[serde(rename = "31_minus_c")]
    Formula31MinusC,
    /// Double-step slope: $\text{TN}_{\text{eff}} = 41 - 2C$.
    #[serde(rename = "41_minus_2c")]
    Formula41Minus2c,
}

impl TnFormulaK20 {
    /// Canonical string representation (`"31_minus_c"` or `"41_minus_2c"`).
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Formula31MinusC => "31_minus_c",
            Self::Formula41Minus2c => "41_minus_2c",
        }
    }
}

impl fmt::Display for TnFormulaK20 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Target number formula selector for $K = 10$ Control vs. Skill 2D matrices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TnFormulaK10 {
    /// Midpoint-anchored single-step slope: $\text{TN}_{\text{eff}} = 16 - C$.
    #[default]
    #[serde(rename = "16_minus_c")]
    Formula16MinusC,
    /// Double-step slope: $\text{TN}_{\text{eff}} = 21 - 2C$.
    #[serde(rename = "21_minus_2c")]
    Formula21Minus2c,
}

impl TnFormulaK10 {
    /// Canonical string representation (`"16_minus_c"` or `"21_minus_2c"`).
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Formula16MinusC => "16_minus_c",
            Self::Formula21Minus2c => "21_minus_2c",
        }
    }
}

impl fmt::Display for TnFormulaK10 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// CLI calibration sweep preset matching `--calibration` in `src/satya_ml/summarize_cascade_odds.py`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationPreset {
    /// Only Calibration A (`midpoint_2c` across $K \in \{10, 20, 100\}$).
    Midpoint2c,
    /// Only Calibration B (`raw_2k_minus_c` across $K \in \{10, 20, 100\}$).
    Raw2kMinusC,
    /// Both Calibration A and Calibration B (`both`, default).
    #[default]
    Both,
    /// Full $K = 20$ sweeps (`k20_full`).
    K20Full,
    /// 2D Control ($C$) vs. Skill ($N$) matrices on $K = 20$ (`c_vs_n`).
    CvsN,
    /// Complete $K = 10$ matrices and sweeps (`k10_all`).
    K10All,
    /// Contested & Hybrid-Contested Task Resolution calibration sweeps (`contested`).
    Contested,
}

impl FromStr for CalibrationPreset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "midpoint_2c" => Ok(Self::Midpoint2c),
            "raw_2k_minus_c" => Ok(Self::Raw2kMinusC),
            "both" => Ok(Self::Both),
            "k20_full" => Ok(Self::K20Full),
            "c_vs_n" => Ok(Self::CvsN),
            "k10_all" => Ok(Self::K10All),
            "contested" => Ok(Self::Contested),
            other => Err(format!(
                "Invalid calibration preset '{other}'. Expected one of: midpoint_2c, raw_2k_minus_c, both, k20_full, c_vs_n, k10_all, contested"
            )),
        }
    }
}

/// Evaluates the cascade probability grid across `K`, `Skill Level`, `CarryMode`, `BotchRule`, and `MarginAggregation`.
///
/// Any `None` slice parameter uses the canonical defaults from `src/satya_ml/summarize_cascade_odds.py`:
/// - `k_values`: `&[10, 20, 100]`
/// - `skill_levels`: `&[3, 4, 5]`
/// - `carry_modes`: `&[CarryMode::MarkovStep, CarryMode::CumulativeSnowball]`
/// - `botch_rules`: `&[BotchRule::ClosestMiss, BotchRule::CeilingBreach, BotchRule::OverrunK]`
/// - `margin_modes`: `&[MarginAggregation::PiecewiseZeroFloor]`
pub fn evaluate_parameter_grid(
    k_values: Option<&[i32]>,
    skill_levels: Option<&[usize]>,
    carry_modes: Option<&[CarryMode]>,
    botch_rules: Option<&[BotchRule]>,
    margin_modes: Option<&[MarginAggregation]>,
    tn_calibration: TnCalibration,
) -> Vec<CascadeSummaryRow> {
    let k_vals = k_values.unwrap_or(&[10, 20, 100]);
    let s_levels = skill_levels.unwrap_or(&[3, 4, 5]);
    let c_modes = carry_modes.unwrap_or(&[CarryMode::MarkovStep, CarryMode::CumulativeSnowball]);
    let b_rules = botch_rules.unwrap_or(&[
        BotchRule::ClosestMiss,
        BotchRule::CeilingBreach,
        BotchRule::OverrunK,
    ]);
    let m_modes = margin_modes.unwrap_or(&[MarginAggregation::PiecewiseZeroFloor]);

    let mut rows = Vec::new();

    for &k in k_vals {
        let control_c = k / 2;
        let tn_eff = match tn_calibration {
            TnCalibration::Midpoint2c => (2 * k + 1) - (2 * control_c),
            TnCalibration::Raw2kMinusC => (2 * k + 2) - control_c,
        };

        let delta_over = ((3 * k) / 10).max(1);
        let delta_under = ((3 * k) / 10).max(1);

        for &skill_n in s_levels {
            for &c_mode in c_modes {
                for &b_rule in b_rules {
                    for &m_mode in m_modes {
                        let res = paired_list_signed_margin_pmfs(
                            skill_n,
                            k,
                            tn_eff,
                            delta_over,
                            delta_under,
                            c_mode,
                            false,
                            false,
                            b_rule,
                            m_mode,
                        );
                        let stats = compute_stats(&res.signed_margin_pmf);
                        rows.push(CascadeSummaryRow {
                            k_sides: k,
                            control_c,
                            tn_eff,
                            skill_level: skill_n,
                            delta_over,
                            delta_under,
                            carry_mode: c_mode,
                            botch_rule: b_rule,
                            margin_mode: m_mode,
                            pct_any_success: res.prob_any_success * 100.0,
                            pct_botch_failure: res.prob_botch_failure * 100.0,
                            pct_succ_and_botch: res.prob_success_with_complication * 100.0,
                            pct_clean_failure: res.prob_clean_failure * 100.0,
                            mean_margin: stats.mean,
                        });
                    }
                }
            }
        }
    }

    rows
}

/// Formats a slice of [`CascadeSummaryRow`] items as an aligned ASCII/Markdown table identical to Python's `format_summary_table`.
pub fn format_summary_table(rows: &[CascadeSummaryRow]) -> String {
    let header = format!(
        "{:>4} | {:>7} | {:>6} | {:>8} | {:>6} | {:>6} | {:<19} | {:<14} | {:<20} | {:>10} | {:>12} | {:>13} | {:>12} | {:>9}",
        "K",
        "Ctrl(C)",
        "TN_eff",
        "Skill(N)",
        "+d_ovr",
        "-d_und",
        "Carry Mode",
        "Botch Rule",
        "Margin Mode",
        "P(Succ>=1)",
        "P(BotchFail)",
        "P(Succ+Botch)",
        "P(CleanFail)",
        "E[Margin]"
    );
    let sep = "-".repeat(header.len());
    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(header);
    lines.push(sep);

    for r in rows {
        lines.push(format!(
            "{:>4} | {:>7} | {:>6} | {:>8} | {:>+6} | {:>6} | {:<19} | {:<14} | {:<20} | {:>9.2}% | {:>11.2}% | {:>12.2}% | {:>11.2}% | {:>+9.2}",
            r.k_sides,
            r.control_c,
            r.tn_eff,
            r.skill_level,
            r.delta_over,
            -r.delta_under,
            r.carry_mode.as_str(),
            r.botch_rule.as_str(),
            r.margin_mode.as_str(),
            r.pct_any_success,
            r.pct_botch_failure,
            r.pct_succ_and_botch,
            r.pct_clean_failure,
            r.mean_margin
        ));
    }

    lines.join("\n")
}

/// Evaluates $K = 20$ across ranges for Control $C$, Skill Level $N$, $\delta_{\text{over}}$, and $\delta_{\text{under}}$.
///
/// Uses the Midpoint-Anchored target formula $\text{TN}_{\text{eff}} = 31 - C$.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_k20_full_range(
    control_values: Option<&[i32]>,
    skill_levels: Option<&[usize]>,
    delta_over_values: Option<&[i32]>,
    delta_under_values: Option<&[i32]>,
    carry_modes: Option<&[CarryMode]>,
    botch_rules: Option<&[BotchRule]>,
    margin_modes: Option<&[MarginAggregation]>,
) -> Vec<CascadeSummaryRow> {
    let k = 20;
    let ctrl_vals = control_values.unwrap_or(&[10]);
    let s_levels = skill_levels.unwrap_or(&[1, 2, 3, 4, 5, 6, 7, 8, 10]);
    let d_over_vals = delta_over_values.unwrap_or(&[4, 6, 8]);
    let d_under_vals = delta_under_values.unwrap_or(&[4, 6, 8, 10, 12, 16]);
    let c_modes = carry_modes.unwrap_or(&[CarryMode::MarkovStep, CarryMode::CumulativeSnowball]);
    let b_rules = botch_rules.unwrap_or(&[BotchRule::ClosestMiss]);
    let m_modes = margin_modes.unwrap_or(&[MarginAggregation::PiecewiseZeroFloor]);

    let mut rows = Vec::new();
    for &ctrl_c in ctrl_vals {
        let tn_eff = (3 * k) / 2 + 1 - ctrl_c; // 31 - C for K=20
        for &skill_n in s_levels {
            for &d_over in d_over_vals {
                for &d_under in d_under_vals {
                    for &c_mode in c_modes {
                        for &b_rule in b_rules {
                            for &m_mode in m_modes {
                                let res = paired_list_signed_margin_pmfs(
                                    skill_n, k, tn_eff, d_over, d_under, c_mode, false, false,
                                    b_rule, m_mode,
                                );
                                let stats = compute_stats(&res.signed_margin_pmf);
                                rows.push(CascadeSummaryRow {
                                    k_sides: k,
                                    control_c: ctrl_c,
                                    tn_eff,
                                    skill_level: skill_n,
                                    delta_over: d_over,
                                    delta_under: d_under,
                                    carry_mode: c_mode,
                                    botch_rule: b_rule,
                                    margin_mode: m_mode,
                                    pct_any_success: res.prob_any_success * 100.0,
                                    pct_botch_failure: res.prob_botch_failure * 100.0,
                                    pct_succ_and_botch: res.prob_success_with_complication * 100.0,
                                    pct_clean_failure: res.prob_clean_failure * 100.0,
                                    mean_margin: stats.mean,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    rows
}

/// Builds a 2D pivot matrix of `P(Succ >= 1)` for Control $C \in \{2, 4, \dots, 20\}$ vs. Skill $N \in \{1..=8\}$ on $K = 20$.
pub fn compare_control_vs_skill_k20(
    delta_over: i32,
    delta_under: i32,
    carry_mode: CarryMode,
    tn_formula: TnFormulaK20,
) -> String {
    let k = 20;
    let controls = [2, 4, 6, 8, 10, 12, 14, 16, 18, 20];
    let skills: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    let mut lines = Vec::new();
    lines.push(format!(
        "\n=== 2D MATRIX: Control (C) vs. Skill (N) on K=20 ({}, +d_over={delta_over}, -d_und={delta_under}, formula={}) ===",
        carry_mode.as_str(),
        tn_formula.as_str()
    ));

    let skill_headers: Vec<String> = skills.iter().map(|n| format!("Skill N={n:<2}")).collect();
    let header = format!(
        "{:>7} | {:>6} | {} | Range (N=1..8)",
        "Ctrl(C)",
        "TN_eff",
        skill_headers.join(" | ")
    );
    let sep = "-".repeat(header.len());
    lines.push(header);
    lines.push(sep.clone());

    let mut col_vals: BTreeMap<usize, Vec<f64>> = skills.iter().map(|&n| (n, Vec::new())).collect();

    for c in controls {
        let tn = match tn_formula {
            TnFormulaK20::Formula31MinusC => 31 - c,
            TnFormulaK20::Formula41Minus2c => 41 - 2 * c,
        };
        let mut row_cells = Vec::with_capacity(skills.len());
        let mut row_probs = Vec::with_capacity(skills.len());
        for &n in &skills {
            let res = paired_list_signed_margin_pmfs(
                n,
                k,
                tn,
                delta_over,
                delta_under,
                carry_mode,
                false,
                false,
                BotchRule::ClosestMiss,
                MarginAggregation::PiecewiseZeroFloor,
            );
            let p = res.prob_any_success * 100.0;
            row_probs.push(p);
            if let Some(col) = col_vals.get_mut(&n) {
                col.push(p);
            }
            row_cells.push(format!("{p:>9.2}%"));
        }
        let span_n = row_probs.last().copied().unwrap_or(0.0) - row_probs[0];
        lines.push(format!(
            "{c:>7} | {tn:>6} | {} | +{span_n:>6.2}% (dN)",
            row_cells.join(" | ")
        ));
    }

    lines.push(sep);
    let span_c_4_18: Vec<String> = skills
        .iter()
        .map(|n| {
            let v = &col_vals[n];
            format!("+{:>8.2}%", v[v.len() - 2] - v[1])
        })
        .collect();
    let span_c_2_20: Vec<String> = skills
        .iter()
        .map(|n| {
            let v = &col_vals[n];
            format!("+{:>8.2}%", v[v.len() - 1] - v[0])
        })
        .collect();
    lines.push(format!(
        "{:>7} | {:>6} | {} | (Playable C span)",
        "dC 4..18",
        "-14 TN",
        span_c_4_18.join(" | ")
    ));
    lines.push(format!(
        "{:>7} | {:>6} | {} | (Full C span)",
        "dC 2..20",
        "-18 TN",
        span_c_2_20.join(" | ")
    ));
    lines.join("\n")
}

/// Builds a 2D pivot matrix of `P(Succ >= 1)` for Control $C \in \{1..=10\}$ vs. Skill $N \in \{1..=8\}$ on $K = 10$.
pub fn compare_control_vs_skill_k10(
    delta_over: i32,
    delta_under: i32,
    carry_mode: CarryMode,
    tn_formula: TnFormulaK10,
) -> String {
    let k = 10;
    let controls = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let skills: [usize; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    let mut lines = Vec::new();
    lines.push(format!(
        "\n=== 2D MATRIX: Control (C) vs. Skill (N) on K=10 ({}, +d_over={delta_over}, -d_und={delta_under}, formula={}) ===",
        carry_mode.as_str(),
        tn_formula.as_str()
    ));

    let skill_headers: Vec<String> = skills.iter().map(|n| format!("Skill N={n:<2}")).collect();
    let header = format!(
        "{:>7} | {:>6} | {} | Range (N=1..8)",
        "Ctrl(C)",
        "TN_eff",
        skill_headers.join(" | ")
    );
    let sep = "-".repeat(header.len());
    lines.push(header);
    lines.push(sep.clone());

    let mut col_vals: BTreeMap<usize, Vec<f64>> = skills.iter().map(|&n| (n, Vec::new())).collect();

    for c in controls {
        let tn = match tn_formula {
            TnFormulaK10::Formula16MinusC => 16 - c,
            TnFormulaK10::Formula21Minus2c => 21 - 2 * c,
        };
        let mut row_cells = Vec::with_capacity(skills.len());
        let mut row_probs = Vec::with_capacity(skills.len());
        for &n in &skills {
            let res = paired_list_signed_margin_pmfs(
                n,
                k,
                tn,
                delta_over,
                delta_under,
                carry_mode,
                false,
                false,
                BotchRule::ClosestMiss,
                MarginAggregation::PiecewiseZeroFloor,
            );
            let p = res.prob_any_success * 100.0;
            row_probs.push(p);
            if let Some(col) = col_vals.get_mut(&n) {
                col.push(p);
            }
            row_cells.push(format!("{p:>9.2}%"));
        }
        let span_n = row_probs.last().copied().unwrap_or(0.0) - row_probs[0];
        lines.push(format!(
            "{c:>7} | {tn:>6} | {} | +{span_n:>6.2}% (dN)",
            row_cells.join(" | ")
        ));
    }

    lines.push(sep);
    let span_c_2_9: Vec<String> = skills
        .iter()
        .map(|n| {
            let v = &col_vals[n];
            format!("+{:>8.2}%", v[v.len() - 2] - v[1])
        })
        .collect();
    let span_c_1_10: Vec<String> = skills
        .iter()
        .map(|n| {
            let v = &col_vals[n];
            format!("+{:>8.2}%", v[v.len() - 1] - v[0])
        })
        .collect();
    lines.push(format!(
        "{:>7} | {:>6} | {} | (Playable C span)",
        "dC 2..9",
        "-7 TN",
        span_c_2_9.join(" | ")
    ));
    lines.push(format!(
        "{:>7} | {:>6} | {} | (Full C span)",
        "dC 1..10",
        "-9 TN",
        span_c_1_10.join(" | ")
    ));
    lines.join("\n")
}

/// Evaluates $K = 10$ across ranges for Control $C$, Skill Level $N$, $\delta_{\text{over}}$, and $\delta_{\text{under}}$.
///
/// Uses the Midpoint-Anchored target formula $\text{TN}_{\text{eff}} = 16 - C$.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_k10_full_range(
    control_values: Option<&[i32]>,
    skill_levels: Option<&[usize]>,
    delta_over_values: Option<&[i32]>,
    delta_under_values: Option<&[i32]>,
    carry_modes: Option<&[CarryMode]>,
    botch_rules: Option<&[BotchRule]>,
    margin_modes: Option<&[MarginAggregation]>,
) -> Vec<CascadeSummaryRow> {
    let k = 10;
    let ctrl_vals = control_values.unwrap_or(&[5]);
    let s_levels = skill_levels.unwrap_or(&[1, 2, 3, 4, 5, 6, 7, 8, 10]);
    let d_over_vals = delta_over_values.unwrap_or(&[2, 3, 4, 5]);
    let d_under_vals = delta_under_values.unwrap_or(&[2, 3, 5, 8]);
    let c_modes = carry_modes.unwrap_or(&[CarryMode::MarkovStep, CarryMode::CumulativeSnowball]);
    let b_rules = botch_rules.unwrap_or(&[BotchRule::ClosestMiss]);
    let m_modes = margin_modes.unwrap_or(&[MarginAggregation::PiecewiseZeroFloor]);

    let mut rows = Vec::new();
    for &ctrl_c in ctrl_vals {
        let tn_eff = (3 * k) / 2 + 1 - ctrl_c; // 16 - C for K=10
        for &skill_n in s_levels {
            for &d_over in d_over_vals {
                for &d_under in d_under_vals {
                    for &c_mode in c_modes {
                        for &b_rule in b_rules {
                            for &m_mode in m_modes {
                                let res = paired_list_signed_margin_pmfs(
                                    skill_n, k, tn_eff, d_over, d_under, c_mode, false, false,
                                    b_rule, m_mode,
                                );
                                let stats = compute_stats(&res.signed_margin_pmf);
                                rows.push(CascadeSummaryRow {
                                    k_sides: k,
                                    control_c: ctrl_c,
                                    tn_eff,
                                    skill_level: skill_n,
                                    delta_over: d_over,
                                    delta_under: d_under,
                                    carry_mode: c_mode,
                                    botch_rule: b_rule,
                                    margin_mode: m_mode,
                                    pct_any_success: res.prob_any_success * 100.0,
                                    pct_botch_failure: res.prob_botch_failure * 100.0,
                                    pct_succ_and_botch: res.prob_success_with_complication * 100.0,
                                    pct_clean_failure: res.prob_clean_failure * 100.0,
                                    mean_margin: stats.mean,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    rows
}

/// Generates a comprehensive comparison report for Hybrid Contested vs. Uncontested task resolution.
pub fn format_contested_calibration_report() -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(
        "=== CONTESTED SWEEP 1: 1-Pair Tiered GM Resistance (4-pt Over-Tiers) Across Skill Levels (C_P=10, Delta_ext=0) ==="
            .to_string(),
    );
    lines.push(format!(
        "{:>4} | {:>11} | {:>9} | {:>9} | {:>9} | {:>10} | {:>10} | {:>10} | {:>9} | {:>9} | {:>8} | {:>6}",
        "N_P",
        "Uncontested",
        "TotalWin%",
        "CleanWin%",
        "ExtrWin%",
        "GrazeTie%",
        "LuckyBrk%",
        "FlrBrch%",
        "CleanFail",
        "BotchFail",
        "E[dM]",
        "BC"
    ));
    lines.push("-".repeat(134));

    for n_p in [1, 2, 3, 4, 5, 6, 7, 8] {
        let uncontested = CascadeTaskParams::new_k20(n_p, 10, 0, 0).exact_odds();
        let contested = ContestedCascadeTaskParams::new_tiered_environment(
            n_p,
            10,
            0,
            0,
            0,
            ContestedTieRule::HybridSignedMargin,
        )
        .exact_contested_odds();

        lines.push(format!(
            "{:>4} | {:>10.2}% | {:>8.2}% | {:>8.2}% | {:>8.2}% | {:>9.2}% | {:>9.2}% | {:>9.2}% | {:>8.2}% | {:>8.2}% | {:>+8.2} | {:>6.3}",
            n_p,
            uncontested.prob_any_success * 100.0,
            contested.prob_total_win * 100.0,
            contested.prob_clean_or_extraordinary_win * 100.0,
            contested.prob_extraordinary_success * 100.0,
            contested.prob_grazing_tie_partial * 100.0,
            contested.prob_lucky_break_partial * 100.0,
            contested.prob_floor_breach_partial * 100.0,
            contested.prob_clean_failure * 100.0,
            contested.prob_botch_failure * 100.0,
            contested.net_margin_stats.mean,
            contested.bimodality_coefficient,
        ));
    }

    lines.push(
        "\n=== CONTESTED SWEEP 2: Hiding Difficulty (Delta_ext on Player TN vs. Hidden in GM Roll at N_P=3, C_P=10) ==="
            .to_string(),
    );
    lines.push(format!(
        "{:<20} | {:>5} | {:>11} | {:>11} | {:>14} | {:>14} | {:>14} | {:>14}",
        "Difficulty Tier",
        "d_ext",
        "Uncont(S>=1)",
        "GM(N=1)Cln",
        "GM(6ptTier)Cln",
        "GM(6ptTier)Tot",
        "GM(4ptTier)Cln",
        "GM(4ptTier)Tot"
    ));
    lines.push("-".repeat(120));

    let base_2d20 = crate::dice_probabilities::sum_dice_pool_pmf(2, 20, 0);

    for &(tier_name, d_ext) in &[
        ("Trivial / Routine", -8),
        ("Easy", -4),
        ("Standard Benchmark", 0),
        ("Challenging", 4),
        ("Hard", 8),
        ("Extreme", 12),
    ] {
        let unc = CascadeTaskParams::new_k20(3, 10, 0, d_ext).exact_odds();
        let unc_s1 = unc.prob_any_success * 100.0;

        let gm1 = ContestedCascadeTaskParams::new_hybrid_environment(
            3,
            10,
            0,
            0,
            1,
            d_ext,
            ContestedTieRule::HybridSignedMargin,
        )
        .exact_contested_odds();

        let mut gm_6pt_params = ContestedCascadeTaskParams::new_tiered_environment(
            3,
            10,
            0,
            0,
            d_ext,
            ContestedTieRule::HybridSignedMargin,
        );
        gm_6pt_params.gm_mode =
            crate::dice_probabilities::GmResistanceMode::TieredSinglePair { step_size: 6 };
        let gm_6pt = gm_6pt_params.exact_contested_odds();

        let gm_4pt = ContestedCascadeTaskParams::new_tiered_environment(
            3,
            10,
            0,
            0,
            d_ext,
            ContestedTieRule::HybridSignedMargin,
        )
        .exact_contested_odds();

        lines.push(format!(
            "{:<20} | {:>+5} | {:>10.2}% | {:>10.2}% | {:>13.2}% | {:>13.2}% | {:>13.2}% | {:>13.2}%",
            tier_name,
            d_ext,
            unc_s1,
            gm1.prob_clean_or_extraordinary_win * 100.0,
            gm_6pt.prob_clean_or_extraordinary_win * 100.0,
            gm_6pt.prob_total_win * 100.0,
            gm_4pt.prob_clean_or_extraordinary_win * 100.0,
            gm_4pt.prob_total_win * 100.0,
        ));
    }

    lines.push(
        "\n=== CONTESTED SWEEP 3: Epistemic Uncertainty — P(Player Loses | Player Rolled k Successes) at C_P=10, N_P=3 ==="
            .to_string(),
    );
    lines.push(format!(
        "{:<26} | {:>12} | {:>14} | {:>14} | {:>14}",
        "Hidden GM Config", "P(GM>=1|S=1)", "P(GM>1 |S=1)", "P(GM>=2|S=2)", "P(GM>=3|S=3)"
    ));
    lines.push("-".repeat(92));

    for &(label, n_gm, d_ext) in &[
        ("Standard (N_GM=1, d_ext=0)", 1, 0),
        ("Hard (N_GM=1, d_ext=+8)", 1, 8),
        ("Standard-2 (N_GM=2, d_ext=0)", 2, 0),
        ("Challenging-2 (N_GM=2, d_ext=+4)", 2, 4),
        ("Hard-2 (N_GM=2, d_ext=+8)", 2, 8),
        ("Peer-3 (N_GM=3, d_ext=0)", 3, 0),
        ("Elite-3 (N_GM=3, d_ext=+4)", 3, 4),
        ("Master-4 (N_GM=4, d_ext=+4)", 4, 4),
    ] {
        let gm_params = CascadeTaskParams::new_k20(n_gm, 10 + d_ext, 0, 0);
        let gm_odds = gm_params.exact_odds();
        let p_gm_ge_1: f64 = gm_odds
            .success_count_pmf
            .iter()
            .filter(|&(&s, _)| s >= 1)
            .map(|(_, &p)| p * 100.0)
            .sum();
        let p_gm_gt_1: f64 = gm_odds
            .success_count_pmf
            .iter()
            .filter(|&(&s, _)| s > 1)
            .map(|(_, &p)| p * 100.0)
            .sum();
        let p_gm_ge_2: f64 = gm_odds
            .success_count_pmf
            .iter()
            .filter(|&(&s, _)| s >= 2)
            .map(|(_, &p)| p * 100.0)
            .sum();
        let p_gm_ge_3: f64 = gm_odds
            .success_count_pmf
            .iter()
            .filter(|&(&s, _)| s >= 3)
            .map(|(_, &p)| p * 100.0)
            .sum();

        lines.push(format!(
            "{:<26} | {:>11.2}% | {:>13.2}% | {:>13.2}% | {:>13.2}%",
            label, p_gm_ge_1, p_gm_gt_1, p_gm_ge_2, p_gm_ge_3
        ));
    }

    lines.push(
        "\n=== CONTESTED SWEEP 4: Epistemic Uncertainty of 1-Pair Tiered GM Roll (6-pt & 4-pt Over-Tiers) ==="
            .to_string(),
    );
    for &step_size in &[6, 4] {
        lines.push(format!(
            "\n--- 1-Pair Tiered GM Roll (Over-Tier Step = {} pts: S_GM = 1 + (X_GM - TN_GM - 1) / {}) ---",
            step_size, step_size
        ));
        lines.push(format!(
            "{:<22} | {:>5} | {:>6} | {:>11} | {:>11} | {:>11} | {:>11} | {:>11} | {:>13}",
            "Difficulty Tier",
            "d_ext",
            "TN_GM",
            "P(GM>=1)",
            "P(GM>=2)",
            "P(GM>=3)",
            "P(GM>=4)",
            "P(GM>=5)",
            "P(GM Botch<0)"
        ));
        lines.push("-".repeat(120));

        for &(tier_name, d_ext) in &[
            ("Trivial / Routine", -8),
            ("Easy", -4),
            ("Standard Benchmark", 0),
            ("Challenging", 4),
            ("Hard", 8),
            ("Extreme", 12),
            ("Heroic", 16),
        ] {
            let tn_gm = 21 - d_ext;
            let mut p_ge = [0.0_f64; 6]; // indices 1..=5
            let mut p_botch = 0.0_f64;

            for (&x_gm, &pg) in &base_2d20 {
                if x_gm > tn_gm {
                    let s = 1 + (x_gm - tn_gm - 1) / step_size;
                    for k in 1..=5 {
                        if s >= k as i32 {
                            p_ge[k] += pg * 100.0;
                        }
                    }
                } else {
                    let b = ((tn_gm - x_gm) / 4).min(5);
                    if b >= 1 {
                        p_botch += pg * 100.0;
                    }
                }
            }

            lines.push(format!(
                "{:<22} | {:>+5} | {:>6} | {:>10.2}% | {:>10.2}% | {:>10.2}% | {:>10.2}% | {:>10.2}% | {:>12.2}%",
                tier_name,
                d_ext,
                tn_gm,
                p_ge[1],
                p_ge[2],
                p_ge[3],
                p_ge[4],
                p_ge[5],
                p_botch,
            ));
        }
    }

    lines.join("\n")
}
/// Generates the complete formatted CLI report for a given [`CalibrationPreset`], matching `python -m satya_ml.summarize_cascade_odds`.
pub fn run_calibration_report(preset: CalibrationPreset) -> String {
    let mut sections: Vec<String> = Vec::new();

    match preset {
        CalibrationPreset::Contested => {
            sections.push(format_contested_calibration_report());
        }
        CalibrationPreset::K10All => {
            sections.push(compare_control_vs_skill_k10(
                3,
                3,
                CarryMode::MarkovStep,
                TnFormulaK10::Formula16MinusC,
            ));
            sections.push(compare_control_vs_skill_k10(
                3,
                3,
                CarryMode::CumulativeSnowball,
                TnFormulaK10::Formula16MinusC,
            ));
            sections.push(compare_control_vs_skill_k10(
                3,
                3,
                CarryMode::MarkovStep,
                TnFormulaK10::Formula21Minus2c,
            ));

            sections.push(
                "\n=== K=10 SWEEP 1: Skill Level N = 1..10 at Baseline Deltas (+d_over=3, -d_under=2, 3, 5, 8) ==="
                    .to_string(),
            );
            let rows_k10_s1 = evaluate_k10_full_range(
                Some(&[5]),
                Some(&[1, 2, 3, 4, 5, 6, 7, 8, 10]),
                Some(&[3]),
                Some(&[2, 3, 5, 8]),
                Some(&[CarryMode::MarkovStep, CarryMode::CumulativeSnowball]),
                Some(&[BotchRule::ClosestMiss]),
                Some(&[MarginAggregation::PiecewiseZeroFloor]),
            );
            sections.push(format_summary_table(&rows_k10_s1));

            sections.push(
                "\n=== K=10 SWEEP 2: Delta Over (2..5) x Delta Under (2..8) Grid at Skill N in {1, 3, 5, 8} ==="
                    .to_string(),
            );
            let rows_k10_s2 = evaluate_k10_full_range(
                Some(&[5]),
                Some(&[1, 3, 5, 8]),
                Some(&[2, 3, 4, 5]),
                Some(&[2, 3, 5, 8]),
                Some(&[CarryMode::MarkovStep]),
                Some(&[BotchRule::ClosestMiss, BotchRule::CeilingBreach]),
                Some(&[MarginAggregation::PiecewiseZeroFloor]),
            );
            sections.push(format_summary_table(&rows_k10_s2));

            sections.push(
                "\n=== K=10 SWEEP 3: Controlling Characteristic C in {2 (Min), 5 (Avg), 9 (Max)} x Skill N in {1, 3, 5, 8} ==="
                    .to_string(),
            );
            let rows_k10_s3 = evaluate_k10_full_range(
                Some(&[2, 5, 9]),
                Some(&[1, 3, 5, 8]),
                Some(&[3]),
                Some(&[2, 3, 5, 8]),
                Some(&[CarryMode::MarkovStep]),
                Some(&[BotchRule::ClosestMiss]),
                Some(&[MarginAggregation::PiecewiseZeroFloor]),
            );
            sections.push(format_summary_table(&rows_k10_s3));
        }
        CalibrationPreset::CvsN => {
            sections.push(compare_control_vs_skill_k20(
                6,
                6,
                CarryMode::MarkovStep,
                TnFormulaK20::Formula31MinusC,
            ));
            sections.push(compare_control_vs_skill_k20(
                6,
                6,
                CarryMode::CumulativeSnowball,
                TnFormulaK20::Formula31MinusC,
            ));
            sections.push(compare_control_vs_skill_k20(
                6,
                6,
                CarryMode::MarkovStep,
                TnFormulaK20::Formula41Minus2c,
            ));
        }
        CalibrationPreset::K20Full => {
            sections.push(
                "\n=== K=20 SWEEP 1: Skill Level N = 1..10 at Baseline Deltas (+d_over=6, -d_under=4, 6, 10, 16) ==="
                    .to_string(),
            );
            let rows_skill_delta = evaluate_k20_full_range(
                Some(&[10]),
                Some(&[1, 2, 3, 4, 5, 6, 7, 8, 10]),
                Some(&[6]),
                Some(&[4, 6, 10, 16]),
                Some(&[CarryMode::MarkovStep, CarryMode::CumulativeSnowball]),
                Some(&[BotchRule::ClosestMiss]),
                Some(&[MarginAggregation::PiecewiseZeroFloor]),
            );
            sections.push(format_summary_table(&rows_skill_delta));

            sections.push(
                "\n=== K=20 SWEEP 2: Full Delta Over (4..10) x Delta Under (4..16) Grid at Skill N in {1, 3, 5, 8} ==="
                    .to_string(),
            );
            let rows_deltas = evaluate_k20_full_range(
                Some(&[10]),
                Some(&[1, 3, 5, 8]),
                Some(&[4, 6, 8, 10]),
                Some(&[4, 6, 10, 16]),
                Some(&[CarryMode::MarkovStep]),
                Some(&[BotchRule::ClosestMiss, BotchRule::CeilingBreach]),
                Some(&[MarginAggregation::PiecewiseZeroFloor]),
            );
            sections.push(format_summary_table(&rows_deltas));

            sections.push(
                "\n=== K=20 SWEEP 3: Controlling Characteristic C in {4 (Min), 10 (Avg), 18 (Max)} x Skill N in {1, 3, 5, 8} ==="
                    .to_string(),
            );
            let rows_ctrl = evaluate_k20_full_range(
                Some(&[4, 10, 18]),
                Some(&[1, 3, 5, 8]),
                Some(&[6]),
                Some(&[4, 6, 12, 16]),
                Some(&[CarryMode::MarkovStep]),
                Some(&[BotchRule::ClosestMiss]),
                Some(&[MarginAggregation::PiecewiseZeroFloor]),
            );
            sections.push(format_summary_table(&rows_ctrl));
        }
        CalibrationPreset::Midpoint2c
        | CalibrationPreset::Raw2kMinusC
        | CalibrationPreset::Both => {
            if matches!(
                preset,
                CalibrationPreset::Midpoint2c | CalibrationPreset::Both
            ) {
                sections.push(
                    "\n=== CALIBRATION A: Midpoint-Anchored Roll-Over (TN_eff = (2K + 1) - 2C -> K + 1 at C = K/2) ==="
                        .to_string(),
                );
                let rows_mid = evaluate_parameter_grid(
                    None,
                    None,
                    None,
                    None,
                    None,
                    TnCalibration::Midpoint2c,
                );
                sections.push(format_summary_table(&rows_mid));
            }
            if matches!(
                preset,
                CalibrationPreset::Raw2kMinusC | CalibrationPreset::Both
            ) {
                sections.push(
                    "\n=== CALIBRATION B: Unscaled Single-C Roll-Over (TN_eff = (2K + 2) - C -> 1.5K + 2 at C = K/2) ==="
                        .to_string(),
                );
                let rows_raw = evaluate_parameter_grid(
                    None,
                    None,
                    None,
                    None,
                    None,
                    TnCalibration::Raw2kMinusC,
                );
                sections.push(format_summary_table(&rows_raw));
            }
        }
    }

    sections.join("\n")
}
