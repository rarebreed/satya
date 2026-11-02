"""CLI script and summary table generator for Satya's Two-List Cascading Dice Mechanic.

Summarizes exact probabilities of:
  - At least 1 Success (`P(Succ >= 1)`)
  - Botch Failure (`P(Succ == 0 & Botch >= 1)`)
  - Success with a Botch / Complication (`P(Succ >= 1 & Botch >= 1)`)
  - Clean Failure (`P(Succ == 0 & Botch == 0)`)

across die sizes K in {10, 20, 100}, average Controlling Characteristic C = K / 2,
average Skill Levels N in {3, 4, 5}, internal/external modifiers = 0, carry-over modes,
and margin of error/success modes.

Usage:
    python -m satya_ml.summarize_cascade_odds
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from typing import List, Literal

from satya_ml.dice_probabilities import compute_stats, paired_list_signed_margin_pmfs


@dataclass(frozen=True)
class CascadeSummaryRow:
    """Single row of the cascade odds summary table."""

    k_sides: int
    control_c: int
    tn_eff: int
    skill_level: int
    delta_over: int
    delta_under: int
    carry_mode: str
    botch_rule: str
    margin_mode: str
    pct_any_success: float
    pct_botch_failure: float
    pct_succ_and_botch: float
    pct_clean_failure: float
    mean_margin: float


def evaluate_parameter_grid(
    k_values: List[int] | None = None,
    skill_levels: List[int] | None = None,
    carry_modes: List[Literal["markov_step", "cumulative_snowball", "linear_margin"]] | None = None,
    botch_rules: List[Literal["closest_miss", "ceiling_breach", "overrun_k"]] | None = None,
    margin_modes: List[Literal["piecewise_zero_floor", "net_difference"]] | None = None,
    tn_calibration: Literal["midpoint_2c", "raw_2k_minus_c"] = "midpoint_2c",
) -> List[CascadeSummaryRow]:
    """Evaluate the cascade probability grid and return summary rows.

    Note on `tn_calibration` when `C = K / 2` and pairs are `2dK` (range `2..2K`, mean `K + 1`):
      - `"midpoint_2c"` (Recommended): Since two dice (`2dK`) are rolled per pair and `C` is on a
        `1..K` scale (average `C = K / 2`), each point of `C` is worth 2 points on `2dK`:
        `TN_eff = (2K + 1) - 2*C + Delta_ext`. At `C = K / 2` and `Delta_ext = 0`, `TN_eff = K + 1`
        (the exact mean of `2dK`).
      - `"raw_2k_minus_c"`: `TN_eff = (2K + 2) - C + Delta_ext`. At `C = K / 2`, `TN_eff = 1.5*K + 2`
        (top ~12.5% tail of `2dK`).
    """
    if k_values is None:
        k_values = [10, 20, 100]
    if skill_levels is None:
        skill_levels = [3, 4, 5]
    if carry_modes is None:
        carry_modes = ["markov_step", "cumulative_snowball"]
    if botch_rules is None:
        botch_rules = ["closest_miss", "ceiling_breach", "overrun_k"]
    if margin_modes is None:
        margin_modes = ["piecewise_zero_floor"]

    rows: List[CascadeSummaryRow] = []

    for k in k_values:
        control_c = k // 2
        if tn_calibration == "midpoint_2c":
            tn_eff = (2 * k + 1) - (2 * control_c)  # Equals K + 1 when C = K / 2
        else:
            tn_eff = (2 * k + 2) - control_c  # Equals 1.5*K + 2 when C = K / 2

        # Scale delta_over (bonus) and delta_under (penalty) proportionally to K (e.g., 30% of K)
        delta_over = max(1, (3 * k) // 10)
        delta_under = max(1, (3 * k) // 10)

        for skill_n in skill_levels:
            for c_mode in carry_modes:
                for b_rule in botch_rules:
                    for m_mode in margin_modes:
                        res = paired_list_signed_margin_pmfs(
                            num_pairs=skill_n,
                            sides=k,
                            characteristic_c=tn_eff,
                            delta_over=delta_over,
                            delta_under=delta_under,
                            mode=c_mode,
                            count_under_as_success=False,
                            botch_rule=b_rule,
                            margin_aggregation=m_mode,
                        )
                        stats = compute_stats(res.signed_margin_pmf)
                        rows.append(
                            CascadeSummaryRow(
                                k_sides=k,
                                control_c=control_c,
                                tn_eff=tn_eff,
                                skill_level=skill_n,
                                delta_over=delta_over,
                                delta_under=delta_under,
                                carry_mode=c_mode,
                                botch_rule=b_rule,
                                margin_mode=m_mode,
                                pct_any_success=res.prob_any_success * 100.0,
                                pct_botch_failure=res.prob_botch_failure * 100.0,
                                pct_succ_and_botch=res.prob_success_with_complication * 100.0,
                                pct_clean_failure=res.prob_clean_failure * 100.0,
                                mean_margin=stats.mean,
                            )
                        )
    return rows


def format_summary_table(rows: List[CascadeSummaryRow]) -> str:
    """Format a list of `CascadeSummaryRow` objects as an aligned ASCII/Markdown table."""
    header = (
        f"{'K':>4} | {'Ctrl(C)':>7} | {'TN_eff':>6} | {'Skill(N)':>8} | "
        f"{'+d_ovr':>6} | {'-d_und':>6} | {'Carry Mode':<19} | {'Botch Rule':<14} | "
        f"{'Margin Mode':<20} | {'P(Succ>=1)':>10} | {'P(BotchFail)':>12} | "
        f"{'P(Succ+Botch)':>13} | {'P(CleanFail)':>12} | {'E[Margin]':>9}"
    )
    sep = "-" * len(header)
    lines = [header, sep]
    for r in rows:
        lines.append(
            f"{r.k_sides:>4} | {r.control_c:>7} | {r.tn_eff:>6} | {r.skill_level:>8} | "
            f"{r.delta_over:>+6} | {-r.delta_under:>6} | {r.carry_mode:<19} | {r.botch_rule:<14} | "
            f"{r.margin_mode:<20} | {r.pct_any_success:>9.2f}% | {r.pct_botch_failure:>11.2f}% | "
            f"{r.pct_succ_and_botch:>12.2f}% | {r.pct_clean_failure:>11.2f}% | {r.mean_margin:>+9.2f}"
        )
    return "\n".join(lines)


def evaluate_k20_full_range(
    control_values: List[int] | None = None,
    skill_levels: List[int] | None = None,
    delta_over_values: List[int] | None = None,
    delta_under_values: List[int] | None = None,
    carry_modes: List[Literal["markov_step", "cumulative_snowball", "linear_margin"]] | None = None,
    botch_rules: List[Literal["closest_miss", "ceiling_breach", "overrun_k"]] | None = None,
    margin_modes: List[Literal["piecewise_zero_floor", "net_difference"]] | None = None,
) -> List[CascadeSummaryRow]:
    """Evaluate K=20 across the full min-to-max ranges for Skill Level, delta_over, and delta_under.

    Uses the Midpoint-Anchored target formula `TN_eff = 31 - C + Delta_ext` (where `1.5*K + 1 = 31`,
    so average `C = 10` with `Delta_ext = 0` yields `TN_eff = 21`, the 50% mean of `2d20`).
    """
    k = 20
    if control_values is None:
        control_values = [10]
    if skill_levels is None:
        skill_levels = [1, 2, 3, 4, 5, 6, 7, 8, 10]
    if delta_over_values is None:
        delta_over_values = [4, 6, 8]
    if delta_under_values is None:
        delta_under_values = [4, 6, 8, 10, 12, 16]
    if carry_modes is None:
        carry_modes = ["markov_step", "cumulative_snowball"]
    if botch_rules is None:
        botch_rules = ["closest_miss"]
    if margin_modes is None:
        margin_modes = ["piecewise_zero_floor"]

    rows: List[CascadeSummaryRow] = []
    for ctrl_c in control_values:
        tn_eff = (3 * k) // 2 + 1 - ctrl_c  # 31 - C for K=20
        for skill_n in skill_levels:
            for d_over in delta_over_values:
                for d_under in delta_under_values:
                    for c_mode in carry_modes:
                        for b_rule in botch_rules:
                            for m_mode in margin_modes:
                                res = paired_list_signed_margin_pmfs(
                                    num_pairs=skill_n,
                                    sides=k,
                                    characteristic_c=tn_eff,
                                    delta_over=d_over,
                                    delta_under=d_under,
                                    mode=c_mode,
                                    count_under_as_success=False,
                                    botch_rule=b_rule,
                                    margin_aggregation=m_mode,
                                )
                                stats = compute_stats(res.signed_margin_pmf)
                                rows.append(
                                    CascadeSummaryRow(
                                        k_sides=k,
                                        control_c=ctrl_c,
                                        tn_eff=tn_eff,
                                        skill_level=skill_n,
                                        delta_over=d_over,
                                        delta_under=d_under,
                                        carry_mode=c_mode,
                                        botch_rule=b_rule,
                                        margin_mode=m_mode,
                                        pct_any_success=res.prob_any_success * 100.0,
                                        pct_botch_failure=res.prob_botch_failure * 100.0,
                                        pct_succ_and_botch=res.prob_success_with_complication * 100.0,
                                        pct_clean_failure=res.prob_clean_failure * 100.0,
                                        mean_margin=stats.mean,
                                    )
                                )
    return rows


def compare_control_vs_skill_k20(
    delta_over: int = 6,
    delta_under: int = 6,
    carry_mode: Literal["markov_step", "cumulative_snowball"] = "markov_step",
    tn_formula: Literal["31_minus_c", "41_minus_2c"] = "31_minus_c",
) -> str:
    """Build a 2D pivot matrix of P(Succ >= 1), P(BotchFail), and E[Margin] for Control C vs Skill N."""
    k = 20
    controls = [2, 4, 6, 8, 10, 12, 14, 16, 18, 20]
    skills = [1, 2, 3, 4, 5, 6, 7, 8]

    lines: List[str] = []
    lines.append(
        f"\n=== 2D MATRIX: Control (C) vs. Skill (N) on K=20 ({carry_mode}, +d_over={delta_over}, -d_und={delta_under}, formula={tn_formula}) ==="
    )
    header = f"{'Ctrl(C)':>7} | {'TN_eff':>6} | " + " | ".join(f"Skill N={n:<2}" for n in skills) + " | Range (N=1..8)"
    lines.append(header)
    lines.append("-" * len(header))

    col_vals: dict[int, list[float]] = {n: [] for n in skills}

    for c in controls:
        tn = (31 - c) if tn_formula == "31_minus_c" else (41 - 2 * c)
        row_cells: List[str] = []
        row_probs: List[float] = []
        for n in skills:
            res = paired_list_signed_margin_pmfs(
                num_pairs=n,
                sides=k,
                characteristic_c=tn,
                delta_over=delta_over,
                delta_under=delta_under,
                mode=carry_mode,
                count_under_as_success=False,
                botch_rule="closest_miss",
                margin_aggregation="piecewise_zero_floor",
            )
            p = res.prob_any_success * 100.0
            row_probs.append(p)
            col_vals[n].append(p)
            row_cells.append(f"{p:>9.2f}%")
        span_n = row_probs[-1] - row_probs[0]
        lines.append(f"{c:>7} | {tn:>6} | " + " | ".join(row_cells) + f" | +{span_n:>6.2f}% (dN)")

    lines.append("-" * len(header))
    span_c_4_18 = [f"+{col_vals[n][-2] - col_vals[n][1]:>8.2f}%" for n in skills]
    span_c_2_20 = [f"+{col_vals[n][-1] - col_vals[n][0]:>8.2f}%" for n in skills]
    lines.append(f"{'dC 4..18':>7} | {'-14 TN':>6} | " + " | ".join(span_c_4_18) + " | (Playable C span)")
    lines.append(f"{'dC 2..20':>7} | {'-18 TN':>6} | " + " | ".join(span_c_2_20) + " | (Full C span)")
    return "\n".join(lines)


def compare_control_vs_skill_k10(
    delta_over: int = 3,
    delta_under: int = 3,
    carry_mode: Literal["markov_step", "cumulative_snowball"] = "markov_step",
    tn_formula: Literal["16_minus_c", "21_minus_2c"] = "16_minus_c",
) -> str:
    """Build a 2D pivot matrix of P(Succ >= 1) for Control C (1..10) vs Skill N (1..8) on K=10."""
    k = 10
    controls = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    skills = [1, 2, 3, 4, 5, 6, 7, 8]

    lines: List[str] = []
    lines.append(
        f"\n=== 2D MATRIX: Control (C) vs. Skill (N) on K=10 ({carry_mode}, +d_over={delta_over}, -d_und={delta_under}, formula={tn_formula}) ==="
    )
    header = f"{'Ctrl(C)':>7} | {'TN_eff':>6} | " + " | ".join(f"Skill N={n:<2}" for n in skills) + " | Range (N=1..8)"
    lines.append(header)
    lines.append("-" * len(header))

    col_vals: dict[int, list[float]] = {n: [] for n in skills}

    for c in controls:
        tn = (16 - c) if tn_formula == "16_minus_c" else (21 - 2 * c)
        row_cells: List[str] = []
        row_probs: List[float] = []
        for n in skills:
            res = paired_list_signed_margin_pmfs(
                num_pairs=n,
                sides=k,
                characteristic_c=tn,
                delta_over=delta_over,
                delta_under=delta_under,
                mode=carry_mode,
                count_under_as_success=False,
                botch_rule="closest_miss",
                margin_aggregation="piecewise_zero_floor",
            )
            p = res.prob_any_success * 100.0
            row_probs.append(p)
            col_vals[n].append(p)
            row_cells.append(f"{p:>9.2f}%")
        span_n = row_probs[-1] - row_probs[0]
        lines.append(f"{c:>7} | {tn:>6} | " + " | ".join(row_cells) + f" | +{span_n:>6.2f}% (dN)")

    lines.append("-" * len(header))
    span_c_2_9 = [f"+{col_vals[n][-2] - col_vals[n][1]:>8.2f}%" for n in skills]
    span_c_1_10 = [f"+{col_vals[n][-1] - col_vals[n][0]:>8.2f}%" for n in skills]
    lines.append(f"{'dC 2..9':>7} | {'-7 TN':>6} | " + " | ".join(span_c_2_9) + " | (Playable C span)")
    lines.append(f"{'dC 1..10':>7} | {'-9 TN':>6} | " + " | ".join(span_c_1_10) + " | (Full C span)")
    return "\n".join(lines)


def evaluate_k10_full_range(
    control_values: List[int] | None = None,
    skill_levels: List[int] | None = None,
    delta_over_values: List[int] | None = None,
    delta_under_values: List[int] | None = None,
    carry_modes: List[Literal["markov_step", "cumulative_snowball", "linear_margin"]] | None = None,
    botch_rules: List[Literal["closest_miss", "ceiling_breach", "overrun_k"]] | None = None,
    margin_modes: List[Literal["piecewise_zero_floor", "net_difference"]] | None = None,
) -> List[CascadeSummaryRow]:
    """Evaluate K=10 across the full min-to-max ranges for Skill Level, delta_over, and delta_under.

    Uses the Midpoint-Anchored target formula `TN_eff = 16 - C + Delta_ext` (where `1.5*K + 1 = 16`,
    so average `C = 5` with `Delta_ext = 0` yields `TN_eff = 11`, the 50% mean of `2d10`).
    """
    k = 10
    if control_values is None:
        control_values = [5]
    if skill_levels is None:
        skill_levels = [1, 2, 3, 4, 5, 6, 7, 8, 10]
    if delta_over_values is None:
        delta_over_values = [2, 3, 4, 5]
    if delta_under_values is None:
        delta_under_values = [2, 3, 5, 8]
    if carry_modes is None:
        carry_modes = ["markov_step", "cumulative_snowball"]
    if botch_rules is None:
        botch_rules = ["closest_miss"]
    if margin_modes is None:
        margin_modes = ["piecewise_zero_floor"]

    rows: List[CascadeSummaryRow] = []
    for ctrl_c in control_values:
        tn_eff = (3 * k) // 2 + 1 - ctrl_c  # 16 - C for K=10
        for skill_n in skill_levels:
            for d_over in delta_over_values:
                for d_under in delta_under_values:
                    for c_mode in carry_modes:
                        for b_rule in botch_rules:
                            for m_mode in margin_modes:
                                res = paired_list_signed_margin_pmfs(
                                    num_pairs=skill_n,
                                    sides=k,
                                    characteristic_c=tn_eff,
                                    delta_over=d_over,
                                    delta_under=d_under,
                                    mode=c_mode,
                                    count_under_as_success=False,
                                    botch_rule=b_rule,
                                    margin_aggregation=m_mode,
                                )
                                stats = compute_stats(res.signed_margin_pmf)
                                rows.append(
                                    CascadeSummaryRow(
                                        k_sides=k,
                                        control_c=ctrl_c,
                                        tn_eff=tn_eff,
                                        skill_level=skill_n,
                                        delta_over=d_over,
                                        delta_under=d_under,
                                        carry_mode=c_mode,
                                        botch_rule=b_rule,
                                        margin_mode=m_mode,
                                        pct_any_success=res.prob_any_success * 100.0,
                                        pct_botch_failure=res.prob_botch_failure * 100.0,
                                        pct_succ_and_botch=res.prob_success_with_complication * 100.0,
                                        pct_clean_failure=res.prob_clean_failure * 100.0,
                                        mean_margin=stats.mean,
                                    )
                                )
    return rows


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Summarize Satya Two-List Cascading Dice odds across K={10,20,100} and Skill={3,4,5}."
    )
    parser.add_argument(
        "--calibration",
        choices=["midpoint_2c", "raw_2k_minus_c", "both", "k20_full", "c_vs_n", "k10_all"],
        default="both",
        help="How C = K/2 maps to TN_eff on 2dK, 'k20_full', 'c_vs_n', or 'k10_all'.",
    )
    args = parser.parse_args()

    if args.calibration == "k10_all":
        print(compare_control_vs_skill_k10(3, 3, "markov_step", "16_minus_c"))
        print(compare_control_vs_skill_k10(3, 3, "cumulative_snowball", "16_minus_c"))
        print(compare_control_vs_skill_k10(3, 3, "markov_step", "21_minus_2c"))

        print(
            "\n=== K=10 SWEEP 1: Skill Level N = 1..10 at Baseline Deltas (+d_over=3, -d_under=2, 3, 5, 8) ==="
        )
        rows_k10_s1 = evaluate_k10_full_range(
            control_values=[5],
            skill_levels=[1, 2, 3, 4, 5, 6, 7, 8, 10],
            delta_over_values=[3],
            delta_under_values=[2, 3, 5, 8],
            carry_modes=["markov_step", "cumulative_snowball"],
            botch_rules=["closest_miss"],
            margin_modes=["piecewise_zero_floor"],
        )
        print(format_summary_table(rows_k10_s1))

        print(
            "\n=== K=10 SWEEP 2: Delta Over (2..5) x Delta Under (2..8) Grid at Skill N in {1, 3, 5, 8} ==="
        )
        rows_k10_s2 = evaluate_k10_full_range(
            control_values=[5],
            skill_levels=[1, 3, 5, 8],
            delta_over_values=[2, 3, 4, 5],
            delta_under_values=[2, 3, 5, 8],
            carry_modes=["markov_step"],
            botch_rules=["closest_miss", "ceiling_breach"],
            margin_modes=["piecewise_zero_floor"],
        )
        print(format_summary_table(rows_k10_s2))

        print(
            "\n=== K=10 SWEEP 3: Controlling Characteristic C in {2 (Min), 5 (Avg), 9 (Max)} x Skill N in {1, 3, 5, 8} ==="
        )
        rows_k10_s3 = evaluate_k10_full_range(
            control_values=[2, 5, 9],
            skill_levels=[1, 3, 5, 8],
            delta_over_values=[3],
            delta_under_values=[2, 3, 5, 8],
            carry_modes=["markov_step"],
            botch_rules=["closest_miss"],
            margin_modes=["piecewise_zero_floor"],
        )
        print(format_summary_table(rows_k10_s3))
        return

    if args.calibration == "c_vs_n":
        print(compare_control_vs_skill_k20(6, 6, "markov_step", "31_minus_c"))
        print(compare_control_vs_skill_k20(6, 6, "cumulative_snowball", "31_minus_c"))
        print(compare_control_vs_skill_k20(6, 6, "markov_step", "41_minus_2c"))
        return

    if args.calibration == "k20_full":
        print(
            "\n=== K=20 SWEEP 1: Skill Level N = 1..10 at Baseline Deltas (+d_over=6, -d_under=4, 6, 10, 16) ==="
        )
        rows_skill_delta = evaluate_k20_full_range(
            control_values=[10],
            skill_levels=[1, 2, 3, 4, 5, 6, 7, 8, 10],
            delta_over_values=[6],
            delta_under_values=[4, 6, 10, 16],
            carry_modes=["markov_step", "cumulative_snowball"],
            botch_rules=["closest_miss"],
            margin_modes=["piecewise_zero_floor"],
        )
        print(format_summary_table(rows_skill_delta))

        print(
            "\n=== K=20 SWEEP 2: Full Delta Over (4..10) x Delta Under (4..16) Grid at Skill N in {1, 3, 5, 8} ==="
        )
        rows_deltas = evaluate_k20_full_range(
            control_values=[10],
            skill_levels=[1, 3, 5, 8],
            delta_over_values=[4, 6, 8, 10],
            delta_under_values=[4, 6, 10, 16],
            carry_modes=["markov_step"],
            botch_rules=["closest_miss", "ceiling_breach"],
            margin_modes=["piecewise_zero_floor"],
        )
        print(format_summary_table(rows_deltas))

        print(
            "\n=== K=20 SWEEP 3: Controlling Characteristic C in {4 (Min), 10 (Avg), 18 (Max)} x Skill N in {1, 3, 5, 8} ==="
        )
        rows_ctrl = evaluate_k20_full_range(
            control_values=[4, 10, 18],
            skill_levels=[1, 3, 5, 8],
            delta_over_values=[6],
            delta_under_values=[4, 6, 12, 16],
            carry_modes=["markov_step"],
            botch_rules=["closest_miss"],
            margin_modes=["piecewise_zero_floor"],
        )
        print(format_summary_table(rows_ctrl))
        return

    if args.calibration in ("midpoint_2c", "both"):
        print(
            "\n=== CALIBRATION A: Midpoint-Anchored Roll-Over (TN_eff = (2K + 1) - 2C -> K + 1 at C = K/2) ==="
        )
        rows_mid = evaluate_parameter_grid(tn_calibration="midpoint_2c")
        print(format_summary_table(rows_mid))

    if args.calibration in ("raw_2k_minus_c", "both"):
        print(
            "\n=== CALIBRATION B: Unscaled Single-C Roll-Over (TN_eff = (2K + 2) - C -> 1.5K + 2 at C = K/2) ==="
        )
        rows_raw = evaluate_parameter_grid(tn_calibration="raw_2k_minus_c")
        print(format_summary_table(rows_raw))


if __name__ == "__main__":
    main()
