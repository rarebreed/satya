"""Plotly visualizations and Python facade for Satya TTRPG dice mechanics.

All exact combinatorial and dynamic-programming probability mass functions (PMFs),
summary statistics (mean, std, skewness, excess kurtosis), Two-List Cascading ("Watershed")
engines, and Rules Book interactive simulators are implemented in Rust in
`crates/satya-engine` and exposed to Python via the `satya_py` PyO3 extension (`crates/satya-py`).
"""

from __future__ import annotations

from pathlib import Path
import shutil
import sys
from typing import TYPE_CHECKING, Dict, Literal

if TYPE_CHECKING:
    import plotly.graph_objects as go


def _ensure_satya_py_loaded():
    """Import `satya_py`, syncing `src/satya_py.pyd` from the newest `target/` DLL if needed."""
    src_dir = Path(__file__).resolve().parent.parent
    repo_root = src_dir.parent
    pyd_path = src_dir / "satya_py.pyd"
    candidates = [
        repo_root / "target" / profile / "satya_py.dll"
        for profile in ("release", "debug")
    ]
    existing = [p for p in candidates if p.exists()]
    if existing:
        newest = max(existing, key=lambda p: p.stat().st_mtime)
        if not pyd_path.exists() or newest.stat().st_mtime > pyd_path.stat().st_mtime:
            try:
                shutil.copy2(newest, pyd_path)
            except OSError:
                pass
    if str(src_dir) not in sys.path:
        sys.path.insert(0, str(src_dir))
    import satya_py as _mod

    return _mod


_satya_py = _ensure_satya_py_loaded()
HAS_SATYA_PY = True

from satya_py import (  # noqa: E402
    DistributionStats,
    SignedCascadeResult,
    center_distance_selection_pmf,
    classify_contested_outcome,
    compute_contested_cascade_odds,
    compute_lifepath_skill_progression,
    compute_social_dissonance_check,
    compute_stats,
    compute_tactical_pool_envelope,
    convolve_pmfs,
    count_successes_pmf,
    evaluate_tactical_triad_contest,
    exploding_pool_keep_highest_pmf,
    median_gated_bimodal_pmf,
    multiplicative_pool_pmf,
    normalize_pmf,
    order_statistic_pmf,
    paired_list_cascade_pmfs,
    paired_list_signed_margin_pmfs,
    single_die_pmf,
    single_exploding_die_pmf,
    sum_dice_pool_pmf,
)

PMF = Dict[int, float]
ContestedTieRuleLiteral = Literal[
    "hybrid_signed_margin",
    "strict_signed_margin",
    "active_player_wins_ties",
]


def using_satya_py_backend() -> bool:
    """Return True if the compiled Rust `satya_py` PyO3 extension module is active."""
    return HAS_SATYA_PY



# ---------------------------------------------------------------------------
# Plotly Figure Builders (Notebook & Satya Web Widget Ready)
# ---------------------------------------------------------------------------

SATYA_PALETTE = [
    "#38bdf8",  # cyan
    "#f472b6",  # pink
    "#a78bfa",  # violet
    "#34d399",  # emerald
    "#fbbf24",  # amber
    "#fb7185",  # rose
]


def _require_plotly():
    """Import and return (plotly.graph_objects, plotly.subplots.make_subplots)."""
    try:
        import plotly.graph_objects as go
        from plotly.subplots import make_subplots

        return go, make_subplots
    except ImportError as exc:
        raise ImportError(
            "Plotly is required for chart rendering. Run `pixi install` to install plotly & jupyterlab."
        ) from exc


def _apply_satya_theme(fig: "go.Figure", title: str, height: int = 460) -> "go.Figure":
    fig.update_layout(
        title=dict(text=title, font=dict(size=18, color="#f8fafc")),
        template="plotly_dark",
        paper_bgcolor="#0f172a",
        plot_bgcolor="#1e293b",
        font=dict(family="Inter, system-ui, sans-serif", color="#e2e8f0"),
        height=height,
        margin=dict(l=55, r=35, t=70, b=55),
        legend=dict(
            bgcolor="rgba(15, 23, 42, 0.75)",
            bordercolor="#334155",
            borderwidth=1,
        ),
        hovermode="x unified",
    )
    return fig


def plot_clt_sum_pools() -> "go.Figure":
    """Compare 1d6 (Uniform), 2d6 (Triangular), 4d6 (Gaussian), and 3d6 - 2d6 (Signed Gaussian).

    Plots both raw sums and normalized z-scores to show why adding/subtracting independent dice
    always collapses to a Gaussian bell curve.
    """
    go, make_subplots = _require_plotly()
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            "Raw Sum Distributions (Mean & Variance Shift)",
            "Standardized Shape (Z-Score: Collapses to Normal)",
        ),
    )

    configs = [
        ("1d6 (Uniform)", sum_dice_pool_pmf(1, 6), SATYA_PALETTE[0]),
        ("2d6 (Triangular)", sum_dice_pool_pmf(2, 6), SATYA_PALETTE[1]),
        ("4d6 (Normal via CLT)", sum_dice_pool_pmf(4, 6), SATYA_PALETTE[2]),
        ("3d6 - 2d6 (Negative Dice)", sum_dice_pool_pmf(3, 6, neg_dice=2), SATYA_PALETTE[3]),
    ]

    for name, pmf, color in configs:
        stats = compute_stats(pmf)
        xs = list(pmf.keys())
        ys = [pmf[x] * 100 for x in xs]
        z_xs = [(x - stats.mean) / stats.std for x in xs]
        # Scale density by std so standardized curves are comparable
        z_ys = [pmf[x] * stats.std for x in xs]

        label = f"{name} (kurt={stats.excess_kurtosis:+.2f})"
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=label,
                line=dict(color=color, width=2.5),
            ),
            row=1,
            col=1,
        )
        fig.add_trace(
            go.Scatter(
                x=z_xs,
                y=z_ys,
                mode="lines+markers",
                name=label,
                line=dict(color=color, width=2.5),
                showlegend=False,
            ),
            row=1,
            col=2,
        )

    fig.update_xaxes(title_text="Roll Total", row=1, col=1)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(title_text="Standard Deviations from Mean (Z)", row=1, col=2)
    fig.update_yaxes(title_text="Standardized Density", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "1. Additive & Negative Dice Pools: Rapid Convergence to Gaussian (CLT)",
    )


def plot_exploding_dice() -> "go.Figure":
    """Visualize Single Exploding Die staircase vs. Variable Explosion Thresholds & Keep-Highest Gumbel."""
    go, make_subplots = _require_plotly()
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            "Single Exploding d6: Thresholds Control Right-Tail Skew",
            "Keep-Highest of N Exploding d6 (Gumbel Extreme Value)",
        ),
    )

    for idx, (label, exp_min) in enumerate(
        [("Explode on 6+", 6), ("Explode on 5-6", 5), ("No Explosion (1d6)", 7)]
    ):
        pmf = single_exploding_die_pmf(6, explode_min=exp_min, max_explosions=3)
        stats = compute_stats(pmf)
        xs = [x for x in pmf.keys() if x <= 24]
        ys = [pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Bar(
                x=xs,
                y=ys,
                name=f"{label} (skew={stats.skewness:+.2f})",
                marker_color=SATYA_PALETTE[idx],
                opacity=0.8,
            ),
            row=1,
            col=1,
        )

    for idx, pool_n in enumerate([1, 3, 5]):
        pmf = exploding_pool_keep_highest_pmf(pool_n, 6, explode_min=6, max_explosions=3)
        stats = compute_stats(pmf)
        xs = [x for x in pmf.keys() if x <= 24]
        ys = [pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=f"Max of {pool_n}d6! (skew={stats.skewness:+.2f})",
                line=dict(color=SATYA_PALETTE[idx + 2], width=2.5),
            ),
            row=1,
            col=2,
        )

    fig.update_layout(barmode="group")
    fig.update_xaxes(title_text="Roll Total (note zero-prob gaps at 6, 12, 18 on 6+)", row=1, col=1)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(title_text="Highest Exploding Die Total", row=1, col=2)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "2. Exploding Dice: Geometric Staircases & Gumbel Heavy Tails",
    )


def plot_order_statistics_and_chaos() -> "go.Figure":
    """Visualize Order Statistics (Discrete Beta) and Focus vs. Chaos (U-Shaped Bimodal)."""
    go, make_subplots = _require_plotly()
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            "Order Statistics on 5d20: Pick k-th Die (Discrete Beta Family)",
            "Center-Distance Selection on 3d20: Focus vs. Chaos (U-Shape)",
        ),
    )

    order_configs = [
        ("Keep 1st (Lowest - Right Skew)", 5, 1, SATYA_PALETTE[5]),
        ("Keep 2nd (Moderate Low Skew)", 5, 2, SATYA_PALETTE[4]),
        ("Keep 3rd (Median - Symmetric Beta)", 5, 3, SATYA_PALETTE[0]),
        ("Keep 4th (Moderate High Skew)", 5, 4, SATYA_PALETTE[2]),
        ("Keep 5th (Highest - Left Skew)", 5, 5, SATYA_PALETTE[3]),
    ]
    for label, n, k, color in order_configs:
        pmf = order_statistic_pmf(n, 20, k)
        stats = compute_stats(pmf)
        xs = list(pmf.keys())
        ys = [pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=f"{label} (skew={stats.skewness:+.2f})",
                line=dict(color=color, width=2.5),
            ),
            row=1,
            col=1,
        )

    chaos_configs = [
        ("Focused (3d20 Keep Closest to Center)", center_distance_selection_pmf(3, 20, "closest"), SATYA_PALETTE[0]),
        ("Neutral (1d20 Uniform)", single_die_pmf(20), SATYA_PALETTE[4]),
        ("Chaotic (3d20 Keep Furthest - Bimodal U)", center_distance_selection_pmf(3, 20, "furthest"), SATYA_PALETTE[1]),
        ("Median-Gated 3d20 (TN 11 - Two Humps)", median_gated_bimodal_pmf(3, 20, gate_tn=11), SATYA_PALETTE[2]),
    ]
    for label, pmf, color in chaos_configs:
        stats = compute_stats(pmf)
        xs = list(pmf.keys())
        ys = [pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=f"{label} (kurt={stats.excess_kurtosis:+.2f})",
                line=dict(color=color, width=2.5),
            ),
            row=1,
            col=2,
        )

    fig.update_xaxes(title_text="d20 Result (1..20)", row=1, col=1)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(title_text="d20 Result (1..20)", row=1, col=2)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "3. Non-Additive Mechanics: Discrete Beta Order Statistics & Bimodal U-Curves",
    )


def plot_paired_list_watershed(num_pairs: int = 5, sides: int = 6) -> "go.Figure":
    """Visualize the User's Two-List Cascading Dice Mechanic (The Watershed Repeller).

    Left subplot: How increasing feedback delta morphs a Bell Curve -> Flat Plateau -> Bimodal U-Curve.
    Right subplot: How changing the Controlling Characteristic C morphs J-Curve -> U-Curve -> Reverse-J.
    """
    go, make_subplots = _require_plotly()
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            f"Effect of Feedback Delta at C=7 ({num_pairs} Pairs of 2d{sides})",
            f"Effect of Characteristic C at Delta=3 ({num_pairs} Pairs of 2d{sides})",
        ),
    )

    deltas = [
        ("delta = 0 (Independent / Bell Curve)", 0, SATYA_PALETTE[0]),
        ("delta = 2 (Critical Flattening / Plateau)", 2, SATYA_PALETTE[4]),
        ("delta = 3 (Bimodal Watershed U-Curve)", 3, SATYA_PALETTE[1]),
        ("delta = 4 (Strong Streak Lock-In)", 4, SATYA_PALETTE[2]),
    ]
    for label, d, color in deltas:
        succ_pmf, _ = paired_list_cascade_pmfs(
            num_pairs=num_pairs,
            sides=sides,
            characteristic_c=sides + 1,
            delta_over=d,
            delta_under=d,
            mode="markov_step",
        )
        stats = compute_stats(succ_pmf)
        xs = list(succ_pmf.keys())
        ys = [succ_pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=f"{label} (kurt={stats.excess_kurtosis:+.2f})",
                line=dict(color=color, width=3),
            ),
            row=1,
            col=1,
        )

    c_values = [
        ("C = 9 (Hard Over / Right-Skewed J)", 9, SATYA_PALETTE[5]),
        ("C = 7 (Even Match / Bimodal U-Curve)", 7, SATYA_PALETTE[1]),
        ("C = 5 (Easy Over / Left-Skewed Reverse-J)", 5, SATYA_PALETTE[3]),
    ]
    for label, c_val, color in c_values:
        succ_pmf, _ = paired_list_cascade_pmfs(
            num_pairs=num_pairs,
            sides=sides,
            characteristic_c=c_val,
            delta_over=3,
            delta_under=3,
            mode="markov_step",
        )
        stats = compute_stats(succ_pmf)
        xs = list(succ_pmf.keys())
        ys = [succ_pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=f"{label} (skew={stats.skewness:+.2f})",
                line=dict(color=color, width=3),
            ),
            row=1,
            col=2,
        )

    fig.update_xaxes(title_text=f"Number of Pairs Rolling Over C (out of {num_pairs})", dtick=1, row=1, col=1)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(title_text=f"Number of Pairs Rolling Over C (out of {num_pairs})", dtick=1, row=1, col=2)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "4. Two-List Cascading Dice: Characteristic C as an Unstable Watershed Divide",
    )


def plot_cascade_final_pair_and_trap(num_pairs: int = 5, sides: int = 6) -> "go.Figure":
    """Compare Final Pair Value Y_N across Markov Step, Asymmetric Carry, and the Linear Telescope Trap."""
    go, make_subplots = _require_plotly()
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            "Final Pair Value Y_N: Non-Linear Step Splits Into Two Humps",
            "The Linear Margin Trap: (Y_n - C) Telescopes Back to Gaussian!",
        ),
    )

    _, base_y = paired_list_cascade_pmfs(num_pairs, sides, characteristic_c=7, delta_over=0, delta_under=0)
    _, markov_y = paired_list_cascade_pmfs(
        num_pairs, sides, characteristic_c=7, delta_over=4, delta_under=4, mode="markov_step"
    )
    _, asym_y = paired_list_cascade_pmfs(
        num_pairs, sides, characteristic_c=7, delta_over=4, delta_under=1, mode="markov_step"
    )
    _, snow_y = paired_list_cascade_pmfs(
        num_pairs, sides, characteristic_c=7, delta_over=2, delta_under=2, mode="cumulative_snowball"
    )

    for label, pmf, color in [
        ("No Carry (2d6 Triangular)", base_y, SATYA_PALETTE[0]),
        ("Markov Step (+4 / -4 Bimodal Split)", markov_y, SATYA_PALETTE[1]),
        ("Asymmetric Mastery (+4 Over / -1 Under)", asym_y, SATYA_PALETTE[3]),
        ("Cumulative Snowball (+2 / -2 per step)", snow_y, SATYA_PALETTE[2]),
    ]:
        xs = list(pmf.keys())
        ys = [pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(x=xs, y=ys, mode="lines+markers", name=label, line=dict(color=color, width=2.5)),
            row=1,
            col=1,
        )

    # Right subplot: Linear Margin Carry (Y_n - C)
    linear_succ, linear_final_y = paired_list_cascade_pmfs(
        num_pairs, sides, characteristic_c=7, mode="linear_margin"
    )
    fig.add_trace(
        go.Scatter(
            x=list(linear_final_y.keys()),
            y=[linear_final_y[x] * 100 for x in linear_final_y.keys()],
            mode="lines+markers",
            name="Final Y_N with Linear (Y_n - C) -> Gaussian Bell!",
            line=dict(color=SATYA_PALETTE[5], width=2.5),
        ),
        row=1,
        col=2,
    )
    fig.add_trace(
        go.Bar(
            x=list(linear_succ.keys()),
            y=[linear_succ[x] * 100 for x in linear_succ.keys()],
            name="Count of Pairs > C with Linear (Y_n - C) -> Levy Arcsine U-Curve!",
            marker_color=SATYA_PALETTE[4],
            opacity=0.65,
        ),
        row=1,
        col=2,
    )

    fig.update_xaxes(title_text="Final Pair Effective Value (Y_N)", row=1, col=1)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(title_text="Value / Success Count", row=1, col=2)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "5. Two-List Cascade Details: Bimodal Final Value vs. The Linear Margin Trap",
    )


def plot_success_counting_and_satya_d20() -> "go.Figure":
    """Compare Binomial success-counting skewness at small p vs. CLT convergence & Satya d20 crit-1 pool."""
    go, make_subplots = _require_plotly()
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            "Binomial Success Pools: Skewed at Extreme TN, Normal at Large N",
            "Satya d20 Roll-Under Pool (Nat 1 = 2 Successes)",
        ),
    )

    binom_configs = [
        ("3d10 >= 9 (p=0.2, Right-Skewed)", count_successes_pmf(3, 10, 9, roll_under=False), SATYA_PALETTE[5]),
        ("3d10 >= 6 (p=0.5, Symmetric)", count_successes_pmf(3, 10, 6, roll_under=False), SATYA_PALETTE[0]),
        ("3d10 >= 3 (p=0.8, Left-Skewed)", count_successes_pmf(3, 10, 3, roll_under=False), SATYA_PALETTE[3]),
        ("12d10 >= 6 (Large N -> Gaussian)", count_successes_pmf(12, 10, 6, roll_under=False), SATYA_PALETTE[2]),
    ]
    for label, pmf, color in binom_configs:
        stats = compute_stats(pmf)
        xs = list(pmf.keys())
        ys = [pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=f"{label} (skew={stats.skewness:+.2f})",
                line=dict(color=color, width=2.5),
            ),
            row=1,
            col=1,
        )

    satya_configs = [
        ("Satya 2d20 <= TN 8 (Crit 1 = 2x)", count_successes_pmf(2, 20, 8, roll_under=True, crit_on_one_double=True), SATYA_PALETTE[0]),
        ("Satya 4d20 <= TN 10 (Crit 1 = 2x)", count_successes_pmf(4, 20, 10, roll_under=True, crit_on_one_double=True), SATYA_PALETTE[1]),
        ("Satya 6d20 <= TN 13 (Crit 1 = 2x)", count_successes_pmf(6, 20, 13, roll_under=True, crit_on_one_double=True), SATYA_PALETTE[3]),
    ]
    for label, pmf, color in satya_configs:
        stats = compute_stats(pmf)
        xs = list(pmf.keys())
        ys = [pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=f"{label} (skew={stats.skewness:+.2f})",
                line=dict(color=color, width=2.5),
            ),
            row=1,
            col=2,
        )

    fig.update_xaxes(title_text="Successes Counted", dtick=1, row=1, col=1)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(title_text="Successes Counted", dtick=1, row=1, col=2)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "6. Success-Counting Pools: Binomial Skewness vs. Satya d20 Critical-1 Pool",
    )


def plot_multiplicative_lognormal() -> "go.Figure":
    """Compare Multiplicative Dice (d1 * d2 * d3 -> Log-Normal) vs. Additive Dice."""
    go, make_subplots = _require_plotly()
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            "Product of 2d6 (d6 x d6) vs. Sum of 2d6",
            "Product of 3d6 (d6 x d6 x d6): Heavy Log-Normal Right Tail",
        ),
    )

    prod_2d6 = multiplicative_pool_pmf(2, 6)
    sum_2d6 = sum_dice_pool_pmf(2, 6)
    stats_p2 = compute_stats(prod_2d6)
    stats_s2 = compute_stats(sum_2d6)

    fig.add_trace(
        go.Bar(
            x=list(prod_2d6.keys()),
            y=[prod_2d6[x] * 100 for x in prod_2d6.keys()],
            name=f"d6 * d6 Product (skew={stats_p2.skewness:+.2f})",
            marker_color=SATYA_PALETTE[1],
            opacity=0.85,
        ),
        row=1,
        col=1,
    )
    fig.add_trace(
        go.Scatter(
            x=list(sum_2d6.keys()),
            y=[sum_2d6[x] * 100 for x in sum_2d6.keys()],
            mode="lines+markers",
            name=f"d6 + d6 Sum (skew={stats_s2.skewness:+.2f})",
            line=dict(color=SATYA_PALETTE[0], width=3),
        ),
        row=1,
        col=1,
    )

    prod_3d6 = multiplicative_pool_pmf(3, 6)
    stats_p3 = compute_stats(prod_3d6)
    fig.add_trace(
        go.Bar(
            x=list(prod_3d6.keys()),
            y=[prod_3d6[x] * 100 for x in prod_3d6.keys()],
            name=f"d6 * d6 * d6 (skew={stats_p3.skewness:+.2f}, kurt={stats_p3.excess_kurtosis:+.2f})",
            marker_color=SATYA_PALETTE[2],
        ),
        row=1,
        col=2,
    )

    fig.update_xaxes(title_text="Outcome Value", row=1, col=1)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(title_text="Product Value (1..216)", row=1, col=2)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "7. Multiplicative Dice Pools: Log-Normal Heavy Right Tails",
    )


def plot_positive_vs_negative_feedback(num_pairs: int = 5, sides: int = 6) -> "go.Figure":
    """Compare Positive Feedback (Watershed Repeller) vs. Negative Feedback (Recoil Attractor).

    When rolling UNDER C adds +delta and rolling OVER C subtracts -delta (`invert_feedback=True`):
    - Left subplot: Streaks are suppressed -> Success count compresses into a razor-sharp Sub-Binomial spike.
    - Right subplot: In cumulative mode, C acts as a homeostatic magnet, pulling the final pair's
      peak away from the dice average (7) and locking it onto C (e.g., C = 11)!
    """
    go, make_subplots = _require_plotly()
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            f"Success Counts at C=7: Watershed U-Curve vs. Recoil Spike ({num_pairs}x 2d{sides})",
            "Cumulative Mode at C=11: Negative Feedback Locks Peak onto C!",
        ),
    )

    pos_succ, _ = paired_list_cascade_pmfs(
        num_pairs, sides, characteristic_c=7, delta_over=3, delta_under=3, invert_feedback=False
    )
    ind_succ, _ = paired_list_cascade_pmfs(
        num_pairs, sides, characteristic_c=7, delta_over=0, delta_under=0
    )
    neg_succ, _ = paired_list_cascade_pmfs(
        num_pairs, sides, characteristic_c=7, delta_over=3, delta_under=3, invert_feedback=True
    )

    for label, pmf, color in [
        ("Over Adds / Under Subtracts (Watershed U-Curve)", pos_succ, SATYA_PALETTE[1]),
        ("No Carry (Standard Binomial Bell)", ind_succ, SATYA_PALETTE[0]),
        ("Under Adds / Over Subtracts (Recoil Spike)", neg_succ, SATYA_PALETTE[3]),
    ]:
        stats = compute_stats(pmf)
        xs = list(pmf.keys())
        ys = [pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=f"{label} (std={stats.std:.2f})",
                line=dict(color=color, width=3),
            ),
            row=1,
            col=1,
        )

    # Right subplot: Base 2d6 (mean 7) vs. Cumulative Negative Feedback with C = 11
    _, base_y = paired_list_cascade_pmfs(num_pairs, sides, characteristic_c=11, delta_over=0, delta_under=0)
    _, neg_cum_y = paired_list_cascade_pmfs(
        num_pairs,
        sides,
        characteristic_c=11,
        delta_over=2,
        delta_under=2,
        mode="cumulative_snowball",
        invert_feedback=True,
    )
    _, pos_cum_y = paired_list_cascade_pmfs(
        num_pairs,
        sides,
        characteristic_c=11,
        delta_over=2,
        delta_under=2,
        mode="cumulative_snowball",
        invert_feedback=False,
    )

    for label, pmf, color in [
        ("Base 2d6 (Natural Peak at 7)", base_y, SATYA_PALETTE[0]),
        ("Under C=11 Adds +2 (Attractor: Peak Pulled to 11!)", neg_cum_y, SATYA_PALETTE[3]),
        ("Over C=11 Adds +2 / Under Subtracts -2 (Repelled Downward)", pos_cum_y, SATYA_PALETTE[5]),
    ]:
        xs = list(pmf.keys())
        ys = [pmf[x] * 100 for x in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=label,
                line=dict(color=color, width=2.5),
            ),
            row=1,
            col=2,
        )

    fig.update_xaxes(title_text=f"Pairs Rolling Under/Over C=7 (out of {num_pairs})", dtick=1, row=1, col=1)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(title_text="Final Pair Value (Y_N)", row=1, col=2)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "8. Flipping the Rule (Under C Adds / Over C Subtracts): Recoil Spike & Homeostatic Attractor",
    )


def plot_four_factor_task_comparison(
    characteristic_c: int = 13,
    external_mod: int = 3,
    sides: int = 10,
    num_pairs: int = 5,
) -> "go.Figure":
    """Compare Fresh Expert, Fatigued Expert, and Fresh Novice on the same 4-Factor Task (Roll-Over).

    Models a Roll-Over Cascade on 2d`sides` where:
      - Fixed Universal Target T_0 = 2 * sides + 2 (e.g., 22 on 2d10), so
        effective dice target TN_eff = (2 * sides + 2) - characteristic_c + external_mod
        (equivalently: A_n + B_n + mod_n + C - external_mod > 22)
      - Rolling OVER TN_eff (> TN_eff) counts as a success and ADDS `+delta_over` (skill bonus)
      - Rolling UNDER TN_eff (< TN_eff) counts as a miss and SUBTRACTS `-delta_under` (fatigue penalty)
    """
    go, make_subplots = _require_plotly()
    tn_eff = (2 * sides + 2) - characteristic_c + external_mod
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            f"Roll-Over Successes (C={characteristic_c}, Ext={external_mod} -> TN_eff={tn_eff})",
            "Final Pair Effective Value Y_N (Higher = Better!)",
        ),
    )

    profiles = [
        ("Fresh Expert (Skill Bonus=+3, Fatigue Penalty=-1)", 3, 1, SATYA_PALETTE[3]),
        ("Fatigued Expert (Skill Bonus=+3, Fatigue Penalty=-4)", 3, 4, SATYA_PALETTE[1]),
        ("Fresh Novice (Skill Bonus=+0, Fatigue Penalty=-1)", 0, 1, SATYA_PALETTE[0]),
    ]

    for label, skill_bon, fatigue_pen, color in profiles:
        succ_pmf, final_y_pmf = paired_list_cascade_pmfs(
            num_pairs=num_pairs,
            sides=sides,
            characteristic_c=tn_eff,
            delta_over=skill_bon,
            delta_under=fatigue_pen,
            mode="markov_step",
            count_under_as_success=False,
        )
        s_stats = compute_stats(succ_pmf)
        fig.add_trace(
            go.Scatter(
                x=list(succ_pmf.keys()),
                y=[succ_pmf[k] * 100 for k in succ_pmf.keys()],
                mode="lines+markers",
                name=f"{label} (skew={s_stats.skewness:+.2f}, kurt={s_stats.excess_kurtosis:+.2f})",
                line=dict(color=color, width=3),
            ),
            row=1,
            col=1,
        )
        fig.add_trace(
            go.Scatter(
                x=list(final_y_pmf.keys()),
                y=[final_y_pmf[k] * 100 for k in final_y_pmf.keys()],
                mode="lines+markers",
                name=label,
                line=dict(color=color, width=2.5),
                showlegend=False,
            ),
            row=1,
            col=2,
        )

    fig.update_xaxes(title_text=f"Successful Pairs > TN_eff (out of {num_pairs})", dtick=1, row=1, col=1)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(title_text="Final Pair Value (Y_N)", row=1, col=2)
    fig.update_yaxes(title_text="Probability (%)", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "9. Four-Factor Task Resolution (Roll-Over): Fresh Expert vs. Fatigued Expert vs. Fresh Novice",
    )


def plot_signed_margin_spectrum(
    characteristic_c: int = 13,
    external_mod: int = 3,
    sides: int = 10,
    num_pairs: int = 5,
) -> "go.Figure":
    """Plot the full `-N .. +N` Signed Margin Spectrum (Degrees of Failure & Success) in Roll-Over mode."""
    go, make_subplots = _require_plotly()
    tn_eff = (2 * sides + 2) - characteristic_c + external_mod
    fig = make_subplots(
        rows=1,
        cols=2,
        subplot_titles=(
            f"Signed Margin (-{num_pairs}..+{num_pairs}) at TN_eff={tn_eff} (Floor Breach < 2)",
            "Closest-Miss Rule Across Skill Tiers (Novice N=2 vs. Journeyman N=4 vs. Master N=6)",
        ),
    )

    profiles = [
        ("Fresh Expert (Skill=+3, Fatigue=-1)", 3, 1, SATYA_PALETTE[3]),
        ("Fatigued Expert (Skill=+3, Fatigue=-4)", 3, 4, SATYA_PALETTE[1]),
        ("Fatigued Novice (Skill=+0, Fatigue=-4)", 0, 4, SATYA_PALETTE[5]),
    ]

    for label, skill_bon, fatigue_pen, color in profiles:
        res = paired_list_signed_margin_pmfs(
            num_pairs=num_pairs,
            sides=sides,
            characteristic_c=tn_eff,
            delta_over=skill_bon,
            delta_under=fatigue_pen,
            mode="cumulative_snowball",
            count_under_as_success=False,
            botch_rule="ceiling_breach",
        )
        xs = list(res.signed_margin_pmf.keys())
        ys = [res.signed_margin_pmf[k] * 100 for k in xs]
        fig.add_trace(
            go.Scatter(
                x=xs,
                y=ys,
                mode="lines+markers",
                name=f"{label} (Botch Fail={res.prob_botch_failure*100:.1f}%)",
                line=dict(color=color, width=3),
            ),
            row=1,
            col=1,
        )

    # Right subplot: Closest-Miss Rule across increasing Skill (N=2 Novice vs N=4 Journeyman vs N=6 Master)
    hard_tn_eff = 14
    skill_tiers = [
        ("Novice (N=2 pairs, Skill=+0)", 2, 0, SATYA_PALETTE[5]),
        ("Journeyman (N=4 pairs, Skill=+2)", 4, 2, SATYA_PALETTE[4]),
        ("Master (N=6 pairs, Skill=+4)", 6, 4, SATYA_PALETTE[3]),
    ]

    for label, n_p, skill_bon, color in skill_tiers:
        res_tier = paired_list_signed_margin_pmfs(
            num_pairs=n_p,
            sides=sides,
            characteristic_c=hard_tn_eff,
            delta_over=skill_bon,
            delta_under=3,
            mode="markov_step",
            count_under_as_success=False,
            botch_rule="closest_miss",
        )
        xs = list(res_tier.signed_margin_pmf.keys())
        ys = [res_tier.signed_margin_pmf[k] * 100 for k in xs]
        fig.add_trace(
            go.Bar(
                x=xs,
                y=ys,
                name=f"{label} (Botch={res_tier.prob_botch_failure*100:.1f}%)",
                marker_color=color,
                opacity=0.82,
            ),
            row=1,
            col=2,
        )

    fig.update_layout(barmode="group")
    fig.update_xaxes(
        title_text="Net Signed Margin (<0 = Failure Severity, 0 = Clean Miss, >0 = Success Margin)",
        dtick=1,
        row=1,
        col=1,
    )
    fig.update_yaxes(title_text="Probability (%)", row=1, col=1)
    fig.update_xaxes(
        title_text="Net Signed Margin (<0 = Failure Severity, 0 = Clean Miss, >0 = Success Margin)",
        dtick=1,
        row=1,
        col=2,
    )
    fig.update_yaxes(title_text="Probability (%)", row=1, col=2)
    return _apply_satya_theme(
        fig,
        "10. Degrees of Failure & Success: Full -N to +N Signed Outcome Spectrum",
    )







