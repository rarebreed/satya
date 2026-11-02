import marimo

__generated_with = "0.25.2"
app = marimo.App(width="full")


@app.cell
def _():
    import pathlib
    import sys

    import marimo as mo
    import plotly.graph_objects as go
    from plotly.subplots import make_subplots

    repo_root = pathlib.Path.cwd()
    if (repo_root / "src").exists():
        sys.path.insert(0, str(repo_root / "src"))
    elif (repo_root.parent / "src").exists():
        sys.path.insert(0, str(repo_root.parent / "src"))

    from satya_ml.dice_probabilities import (
        compute_stats,
        paired_list_cascade_pmfs,
        paired_list_signed_margin_pmfs,
        plot_cascade_final_pair_and_trap,
        plot_clt_sum_pools,
        plot_exploding_dice,
        plot_four_factor_task_comparison,
        plot_multiplicative_lognormal,
        plot_order_statistics_and_chaos,
        plot_paired_list_watershed,
        plot_positive_vs_negative_feedback,
        plot_signed_margin_spectrum,
        plot_success_counting_and_satya_d20,
    )

    return (
        compute_stats,
        go,
        make_subplots,
        mo,
        paired_list_signed_margin_pmfs,
        plot_cascade_final_pair_and_trap,
        plot_clt_sum_pools,
        plot_exploding_dice,
        plot_four_factor_task_comparison,
        plot_multiplicative_lognormal,
        plot_order_statistics_and_chaos,
        plot_paired_list_watershed,
        plot_positive_vs_negative_feedback,
        plot_signed_margin_spectrum,
        plot_success_counting_and_satya_d20,
    )


@app.cell
def _(mo):
    mo.md("""
    # Satya CARPG: Dice Mechanics & Probability Distribution Topologies

    This reactive **Marimo** notebook demonstrates and visualizes how different tabletop and **Computer-Assisted RPG (CARPG)** dice mechanics alter the **actual probability distribution family** (skewness, kurtosis, modality, and tail behavior) rather than merely shifting the mean or scaling the variance of a Gaussian bell curve.

    All probability distributions are computed **exactly** (zero Monte Carlo sampling noise) via combinatorial and dynamic-programming engines in `src/satya_ml/dice_probabilities.py` and rendered interactively with **Plotly**.
    """)
    return


@app.cell
def _(mo, plot_clt_sum_pools):
    mo.vstack(
        [
            mo.md(
                """
                ## 1. Additive & Negative Dice Pools: Why the Central Limit Theorem Forces a Bell Curve
                - **$1\\text{d}6$**: Flat **Uniform** distribution ($\\text{kurtosis} = -1.27$).
                - **$2\\text{d}6$**: Symmetric **Triangular** distribution ($\\text{kurtosis} = -0.63$).
                - **$4\\text{d}6$ and $3\\text{d}6 - 2\\text{d}6$ (Negative Dice)**: Rapidly converge to a **Gaussian (Normal)** distribution ($\\text{skewness} = 0$, $\\text{excess kurtosis} \\to 0$).
                """
            ),
            mo.ui.plotly(plot_clt_sum_pools()),
        ]
    )
    return


@app.cell
def _(mo, plot_success_counting_and_satya_d20):
    mo.vstack(
        [
            mo.md(
                """
                ## 2. Success-Counting Pools ($N\\text{d}K \\ge T$) & Satya's Roll-Under d20 Pool
                - **Small Pool + Extreme Target Number ($p = 0.2$ or $p = 0.8$)**: Strongly skewed (Poisson-like).
                - **Moderate/Large Pool ($12\\text{d}10 \\ge 6$)**: Collapses back into a symmetric Gaussian bell curve.
                - **Right Subplot (Satya d20 Pool)**: Shows Satya's roll-under $\\text{d}20 \\le \\text{TN}$ mechanic where a natural `1` counts as **2 successes**.
                """
            ),
            mo.ui.plotly(plot_success_counting_and_satya_d20()),
        ]
    )
    return


@app.cell
def _(mo, plot_exploding_dice):
    mo.vstack(
        [
            mo.md(
                """
                ## 3. Exploding Dice: Geometric Staircases & Gumbel Extreme Value Tails
                - **Left Subplot (Single Exploding d6)**: Introduces strong **positive skewness**, an exponentially decaying right tail, and zero-probability gaps at multiples of $6$ ($6, 12, 18\\dots$).
                - **Right Subplot (Keep-Highest of $N$ Exploding Dice)**: Converges to a permanently asymmetric **Gumbel (Extreme Value Type I)** distribution.
                """
            ),
            mo.ui.plotly(plot_exploding_dice()),
        ]
    )
    return


@app.cell
def _(mo, plot_order_statistics_and_chaos):
    mo.vstack(
        [
            mo.md(
                """
                ## 4. Non-Additive Selection: Discrete Beta Order Statistics & Bimodal U-Curves
                - **Left Subplot (Roll $5\\text{d}20$, Keep $k$-th Die)**: Maps to a discretized $\\text{Beta}(k, N-k+1)$ distribution bounded on $1..20$.
                - **Right Subplot (Center-Distance & Median-Gated Selection)**:
                  - **Focused (`closest` to $10.5$)**: Sharp tent peak in the middle.
                  - **Chaotic (`furthest` from $10.5$)**: Inverts the bell curve into a **bimodal U-shaped distribution** where $1$ and $20$ are the dominant peaks.
                  - **Median-Gated $3\\text{d}20$**: Uses the middle die as a gate between $\\max$ and $\\min$, creating **two distinct humps** with a valley in the middle.
                """
            ),
            mo.ui.plotly(plot_order_statistics_and_chaos()),
        ]
    )
    return


@app.cell
def _(mo, plot_paired_list_watershed):
    mo.vstack(
        [
            mo.md(
                """
                ## 5. Two-List Cascading / Contagious Dice: Characteristic $C$ as a Watershed Repeller
                Two lists of $N$ dice are rolled ($A_1\\dots A_N$ and $B_1\\dots B_N$). At each index $n$, the paired sum $Y_n = A_n + B_n + \\text{mod}_n$ is compared to the controlling characteristic $C$:
                - If $Y_n < C$, we **subtract** $\\delta_{\\text{under}}$ from pair $n+1$ (pushing $n+1$ *further under* $C$).
                - If $Y_n > C$, we **add** $\\delta_{\\text{over}}$ to pair $n+1$ (pushing $n+1$ *further over* $C$).
                """
            ),
            mo.ui.plotly(plot_paired_list_watershed(num_pairs=5, sides=6)),
        ]
    )
    return


@app.cell
def _(mo, plot_cascade_final_pair_and_trap):
    mo.vstack(
        [
            mo.md(
                """
                ## 6. Two-List Cascade Final Value ($Y_N$) & The Linear Margin Trap
                - **Left Subplot**: Non-linear step carry ($\\pm 4$) splits the final pair $Y_N$ into **two distinct humps** straddling $C=7$.
                - **Right Subplot (The Linear Margin Trap)**: Carrying the exact linear margin $(Y_n - C)$ yields **Lévy's Arcsine U-curve** for success counts (gold bars), but telescopes into a Gaussian bell curve if you only inspect the final value $Y_N$ (red line).
                """
            ),
            mo.ui.plotly(plot_cascade_final_pair_and_trap(num_pairs=5, sides=6)),
        ]
    )
    return


@app.cell
def _(mo, plot_positive_vs_negative_feedback):
    mo.vstack(
        [
            mo.md(
                """
                ## 7. Flipping the Rule (`invert_feedback=True`): Recoil Spike & Homeostatic Attractor
                When rolling **under** $C$ adds $+\\delta$ to step $n+1$ and rolling **over** $C$ subtracts $-\\delta$ from step $n+1$, $C$ flips from a **Watershed Repeller** into a **Homeostatic Attractor**:
                - **Left Subplot**: Streaks are suppressed, compressing success counts into a razor-sharp **Sub-Binomial Spike**.
                - **Right Subplot**: In `cumulative_snowball` mode, $C=11$ acts like a magnet, pulling the $2\\text{d}6$ peak away from $7$ and locking it directly onto $C=11$!
                """
            ),
            mo.ui.plotly(plot_positive_vs_negative_feedback(num_pairs=5, sides=6)),
        ]
    )
    return


@app.cell
def _(mo, plot_multiplicative_lognormal):
    mo.vstack(
        [
            mo.md(
                """
                ## 8. Multiplicative Dice Pools ($d_1 \\times d_2 \\times d_3$): Log-Normal Heavy Tails
                Multiplying dice invokes the **Multiplicative Central Limit Theorem ($\\to$ Log-Normal)**, compressing low rolls and stretching a heavy tail to the right.
                """
            ),
            mo.ui.plotly(plot_multiplicative_lognormal()),
        ]
    )
    return


@app.cell(hide_code=True, expand_output=True)
def _(mo, plot_four_factor_task_comparison):
    mo.vstack(
        [
            mo.md(
                """
                ## 9. Four-Factor Task Resolution (Roll-Over: High Roll & High Characteristic Are Both Good!)
                Uses a **Fixed Universal Target $T_0 = 2K + 2$** ($22$ on $2\\text{d}10$), so each pair succeeds when:
                $$(A_n + B_n + \\text{mod}_n) + C - \\Delta_{\\text{ext}} > 22 \\quad \\Longleftrightarrow \\quad A_n + B_n + \\text{mod}_n > \\text{TN}_{\\text{eff}} \\;\\; (\\text{where } \\text{TN}_{\\text{eff}} = 22 - C + \\Delta_{\\text{ext}})$$
                - **Rolling OVER $\\text{TN}_{\\text{eff}}$ (Success)**: Score $+1$ Success and **add $+\\delta_{\\text{over}}$** (Skill bonus) to pair $n+1$.
                - **Rolling UNDER $\\text{TN}_{\\text{eff}}$ (Miss)**: Score $0$ Successes and **subtract $-\\delta_{\\text{under}}$** (Base Penalty + Fatigue) from pair $n+1$.
                """
            ),
            mo.ui.plotly(
                plot_four_factor_task_comparison(
                    characteristic_c=13,
                    external_mod=3,
                    sides=10,
                    num_pairs=5,
                )
            ),
        ]
    )
    return


@app.cell(hide_code=True, expand_output=True)
def _(mo, plot_signed_margin_spectrum):
    mo.vstack(
        [
            mo.md(
                """
                ## 10. Degrees of Failure & Success: Full `-N` to `+N` Signed Outcome Spectrum (Roll-Over)
                - **Anti-Paradox Closest-Miss Rule (`min_n(TN_eff - Y_n)` when `Successes == 0`)**:
                  When a character fails (`0` successes), failure severity is determined by **how close their *highest* (best) pair got to $\\text{TN}_{\\text{eff}}$** across all $N$ pairs. Because rolling more pairs $N$ can only *reduce* the minimum shortfall, **higher Skill ($N$ and $+\\delta_{\\text{over}}$) strictly reduces both failure probability and failure severity!**
                """
            ),
            mo.ui.plotly(
                plot_signed_margin_spectrum(
                    characteristic_c=13,
                    external_mod=3,
                    sides=10,
                    num_pairs=5,
                )
            ),
        ]
    )
    return


@app.cell
def _(mo):
    num_pairs_slider = mo.ui.slider(
        start=2, stop=8, step=1, value=5, label="1. Skill — Chain Length (N Pairs)"
    )
    characteristic_slider = mo.ui.slider(
        start=4, stop=25, step=1, value=13, label="2. Controlling Characteristic (C — Higher is Better!)"
    )
    internal_mod_slider = mo.ui.slider(
        start=-2, stop=6, step=1, value=1, label="3. Internal Character Mod (Fatigue / Shock -> adds to miss penalty)"
    )
    external_mod_slider = mo.ui.slider(
        start=-5, stop=10, step=1, value=3, label="4. External Task Mod (Difficulty -> raises TN_eff)"
    )
    delta_over_slider = mo.ui.slider(
        start=0, stop=6, step=1, value=3, label="Cascade Bonus: delta_over (Added to n+1 when > TN_eff)"
    )
    delta_under_slider = mo.ui.slider(
        start=0, stop=6, step=1, value=2, label="Base Cascade Penalty: delta_under (Subtracted from n+1 when < TN_eff)"
    )
    die_sides_dropdown = mo.ui.dropdown(
        options={
            "d6 (2d6 avg=7, Fixed T_0=14)": 6,
            "d8 (2d8 avg=9, Fixed T_0=18)": 8,
            "d10 (2d10 avg=11, Fixed T_0=22)": 10,
            "d12 (2d12 avg=13, Fixed T_0=26)": 12,
            "d20 (2d20 avg=21, Fixed T_0=42)": 20,
        },
        value="d10 (2d10 avg=11, Fixed T_0=22)",
        label="Die Size (dK)",
    )
    mode_dropdown = mo.ui.dropdown(
        options=["markov_step", "cumulative_snowball", "linear_margin"],
        value="markov_step",
        label="Propagation Mode",
    )
    botch_rule_dropdown = mo.ui.dropdown(
        options={
            "Closest Miss (min_n(|Y_n - TN|) — Anti-Paradox)": "closest_miss",
            "Floor/Ceiling Breach (Y_n < 2 or Y_n > 2K)": "ceiling_breach",
            "Overrun Margin (|Y_n - TN| > K)": "overrun_k",
        },
        value="Closest Miss (min_n(|Y_n - TN|) — Anti-Paradox)",
        label="Botch / Failure Severity Rule",
    )
    margin_agg_dropdown = mo.ui.dropdown(
        options={
            "Piecewise (+Successes if >0, else -Botches)": "piecewise_zero_floor",
            "Net Difference (Successes - Botches)": "net_difference",
        },
        value="Piecewise (+Successes if >0, else -Botches)",
        label="Signed Margin Formula",
    )
    roll_under_checkbox = mo.ui.checkbox(
        value=False, label="Legacy Roll-Under Mode (Unchecked = Standard Roll-Over!)"
    )
    invert_feedback_checkbox = mo.ui.checkbox(
        value=False, label="Invert Feedback (Recoil / Attractor Mode)"
    )

    mo.vstack(
        [
            mo.md("## 11. Reactive 4-Factor Sandbox (Roll-Over Default: High Roll & High Characteristic Are Good!)"),
            mo.hstack(
                [
                    mo.vstack(
                        [
                            mo.md("**4 Task Factors**"),
                            num_pairs_slider,
                            characteristic_slider,
                            internal_mod_slider,
                            external_mod_slider,
                        ]
                    ),
                    mo.vstack(
                        [
                            mo.md("**Cascade Step Levers (`delta_over` / `delta_under`)**"),
                            delta_over_slider,
                            delta_under_slider,
                            die_sides_dropdown,
                        ]
                    ),
                    mo.vstack(
                        [
                            mo.md("**Resolution & Botch Rules**"),
                            mode_dropdown,
                            botch_rule_dropdown,
                            margin_agg_dropdown,
                            roll_under_checkbox,
                            invert_feedback_checkbox,
                        ]
                    ),
                ],
                justify="space-around",
            ),
        ]
    )
    return (
        botch_rule_dropdown,
        characteristic_slider,
        delta_over_slider,
        delta_under_slider,
        die_sides_dropdown,
        external_mod_slider,
        internal_mod_slider,
        invert_feedback_checkbox,
        margin_agg_dropdown,
        mode_dropdown,
        num_pairs_slider,
        roll_under_checkbox,
    )


@app.cell(hide_code=True)
def _(
    botch_rule_dropdown,
    characteristic_slider,
    compute_stats,
    delta_over_slider,
    delta_under_slider,
    die_sides_dropdown,
    external_mod_slider,
    go,
    internal_mod_slider,
    invert_feedback_checkbox,
    make_subplots,
    margin_agg_dropdown,
    mo,
    mode_dropdown,
    num_pairs_slider,
    paired_list_signed_margin_pmfs,
    roll_under_checkbox,
):
    sides_k = die_sides_dropdown.value
    fixed_t0 = 2 * sides_k + 2
    if roll_under_checkbox.value:
        # Roll-Under: Y_n < C - Ext
        threshold_eff = characteristic_slider.value - external_mod_slider.value
        eff_delta_over = max(0, delta_under_slider.value + internal_mod_slider.value)
        eff_delta_under = delta_over_slider.value
        threshold_label = f"C_eff={threshold_eff}"
    else:
        # Roll-Over (Default): (Y_n + C - Ext > T_0) <=> (Y_n > T_0 - C + Ext)
        threshold_eff = fixed_t0 - characteristic_slider.value + external_mod_slider.value
        eff_delta_over = delta_over_slider.value
        eff_delta_under = max(0, delta_under_slider.value + internal_mod_slider.value)
        threshold_label = f"TN_eff={threshold_eff} (T_0={fixed_t0} - C={characteristic_slider.value} + Ext={external_mod_slider.value})"

    res_live = paired_list_signed_margin_pmfs(
        num_pairs=num_pairs_slider.value,
        sides=sides_k,
        characteristic_c=threshold_eff,
        delta_over=eff_delta_over,
        delta_under=eff_delta_under,
        mode=mode_dropdown.value,
        count_under_as_success=roll_under_checkbox.value,
        invert_feedback=invert_feedback_checkbox.value,
        botch_rule=botch_rule_dropdown.value,
        margin_aggregation=margin_agg_dropdown.value,
    )

    m_stats = compute_stats(res_live.signed_margin_pmf)
    y_stats = compute_stats(res_live.final_pair_value_pmf)

    margin_xs = list(range(-max(5, num_pairs_slider.value), num_pairs_slider.value + 1))
    margin_ys = [res_live.signed_margin_pmf.get(m, 0.0) * 100 for m in margin_xs]
    bar_colors = [
        "#fb7185" if m < 0 else ("#fbbf24" if m == 0 else "#34d399")
        for m in margin_xs
    ]

    fig_live = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            f"Signed Margin (-{max(5, num_pairs_slider.value)}..+{num_pairs_slider.value}) | {threshold_label}",
            f"Final Pair Effective Value Y_{num_pairs_slider.value} (Higher = Better in Roll-Over)",
        ),
    )
    fig_live.add_trace(
        go.Bar(
            x=margin_xs,
            y=margin_ys,
            name="Signed Margin (%)",
            marker_color=bar_colors,
        ),
        row=1,
        col=1,
    )
    fig_live.add_trace(
        go.Bar(
            x=list(res_live.final_pair_value_pmf.keys()),
            y=[v * 100 for v in res_live.final_pair_value_pmf.values()],
            name="Final Pair Y_N (%)",
            marker_color="#38bdf8",
        ),
        row=1,
        col=2,
    )
    fig_live.update_xaxes(
        title_text="Net Signed Margin (Red <0 = Botch Failure, Amber 0 = Clean Miss, Green >0 = Success)",
        dtick=1,
        row=1,
        col=1,
    )
    fig_live.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig_live.update_xaxes(title_text="Final Pair Value (Y_N)", row=1, col=2)
    fig_live.update_yaxes(title_text="Probability (%)", row=1, col=2)
    fig_live.update_layout(
        template="plotly_dark",
        paper_bgcolor="#0f172a",
        plot_bgcolor="#1e293b",
        font=dict(color="#e2e8f0"),
        height=460,
        showlegend=False,
    )

    mo.vstack(
        [
            mo.md(
                f"""
                **Outcome Breakdown (`{threshold_label}`, `delta_over = +{eff_delta_over}`, `delta_under = -{eff_delta_under}`):**
                - **Total Success (`>= +1`)**: `{res_live.prob_any_success * 100:.1f}%` *(including **Success at a Cost / Complication**: `{res_live.prob_success_with_complication * 100:.1f}%`)*
                - **Clean Failure (`0` — No Success, No Botch)**: `{res_live.prob_clean_failure * 100:.1f}%`
                - **Botch / Setback Failure (`<= -1` — Severe/Catastrophic)**: `{res_live.prob_botch_failure * 100:.1f}%`
                - **Signed Margin Moments** — Mean: `{m_stats.mean:+.2f}` | Std: `{m_stats.std:.2f}` | Skewness: `{m_stats.skewness:+.2f}` | Excess Kurtosis: `{m_stats.excess_kurtosis:+.2f}`
                """
            ),
            mo.ui.plotly(fig_live),
        ]
    )
    return


if __name__ == "__main__":
    app.run()
