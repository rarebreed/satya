"""Exact probability distribution engines and Plotly visualizations for Satya TTRPG dice mechanics.

Provides exact combinatorial / dynamic-programming probability mass functions (PMFs),
summary statistics (mean, std, skewness, excess kurtosis), and interactive Plotly figures
that can be rendered in Jupyter notebooks or serialized to JSON for a Satya Web UI widget.
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
import math
from typing import TYPE_CHECKING, Dict, List, Literal, Tuple

if TYPE_CHECKING:
    import plotly.graph_objects as go

PMF = Dict[int, float]


@dataclass(frozen=True)
class DistributionStats:
    """Summary moments of a discrete probability distribution."""

    mean: float
    variance: float
    std: float
    skewness: float
    excess_kurtosis: float


def normalize_pmf(pmf: Dict[int, float]) -> PMF:
    """Return a sorted, normalized probability mass function."""
    total = sum(pmf.values())
    if total <= 0:
        return {}
    return {k: pmf[k] / total for k in sorted(pmf.keys()) if pmf[k] > 1e-15}


def compute_stats(pmf: PMF) -> DistributionStats:
    """Compute exact mean, variance, standard deviation, skewness, and excess kurtosis."""
    pmf = normalize_pmf(pmf)
    mean = sum(x * p for x, p in pmf.items())
    variance = sum(((x - mean) ** 2) * p for x, p in pmf.items())
    std = math.sqrt(max(variance, 0.0))
    if std < 1e-12:
        return DistributionStats(mean, 0.0, 0.0, 0.0, 0.0)
    m3 = sum(((x - mean) ** 3) * p for x, p in pmf.items())
    m4 = sum(((x - mean) ** 4) * p for x, p in pmf.items())
    skewness = m3 / (std**3)
    excess_kurtosis = (m4 / (variance**2)) - 3.0
    return DistributionStats(
        mean=mean,
        variance=variance,
        std=std,
        skewness=skewness,
        excess_kurtosis=excess_kurtosis,
    )


def convolve_pmfs(pmf_a: PMF, pmf_b: PMF) -> PMF:
    """Compute the exact distribution of X + Y for independent discrete variables X and Y."""
    out: Dict[int, float] = defaultdict(float)
    for x, px in pmf_a.items():
        for y, py in pmf_b.items():
            out[x + y] += px * py
    return normalize_pmf(out)


def single_die_pmf(sides: int, negative: bool = False) -> PMF:
    """Return uniform PMF for a single die dK (or -dK if negative=True)."""
    p = 1.0 / sides
    if negative:
        return {-face: p for face in range(sides, 0, -1)}
    return {face: p for face in range(1, sides + 1)}


# ---------------------------------------------------------------------------
# 1. Additive & Negative Dice Pools (Central Limit Theorem Demonstration)
# ---------------------------------------------------------------------------


def sum_dice_pool_pmf(pos_dice: int, sides: int, neg_dice: int = 0) -> PMF:
    """Exact PMF of the sum of `pos_dice` dK minus `neg_dice` dK."""
    result: PMF = {0: 1.0}
    pos_die = single_die_pmf(sides, negative=False)
    neg_die = single_die_pmf(sides, negative=True)
    for _ in range(pos_dice):
        result = convolve_pmfs(result, pos_die)
    for _ in range(neg_dice):
        result = convolve_pmfs(result, neg_die)
    return result


# ---------------------------------------------------------------------------
# 2. Success-Counting Dice Pools (Binomial & Satya d20 Critical-1 Pool)
# ---------------------------------------------------------------------------


def count_successes_pmf(
    pool_size: int,
    sides: int,
    target_num: int,
    roll_under: bool = True,
    crit_on_one_double: bool = False,
) -> PMF:
    """Exact PMF of success counts in an NdK pool.

    If `crit_on_one_double` is True (as in Satya's d20 rules), rolling a 1 counts as 2 successes.
    """
    die_success_pmf: Dict[int, float] = defaultdict(float)
    for face in range(1, sides + 1):
        is_hit = (face <= target_num) if roll_under else (face >= target_num)
        if crit_on_one_double and face == 1 and is_hit:
            die_success_pmf[2] += 1.0 / sides
        elif is_hit:
            die_success_pmf[1] += 1.0 / sides
        else:
            die_success_pmf[0] += 1.0 / sides

    result: PMF = {0: 1.0}
    for _ in range(pool_size):
        result = convolve_pmfs(result, die_success_pmf)
    return result


# ---------------------------------------------------------------------------
# 3. Exploding Dice (Single, Summed Pool, and Keep-Highest Gumbel Pool)
# ---------------------------------------------------------------------------


def single_exploding_die_pmf(
    sides: int,
    explode_min: int | None = None,
    max_explosions: int = 4,
) -> PMF:
    """Exact PMF of a single exploding dK (re-roll and add when face >= explode_min)."""
    if explode_min is None:
        explode_min = sides
    p = 1.0 / sides
    # Base state: (accumulated_sum, can_still_explode)
    active: Dict[int, float] = {0: 1.0}
    finished: Dict[int, float] = defaultdict(float)

    for step in range(max_explosions + 1):
        next_active: Dict[int, float] = defaultdict(float)
        is_last = step == max_explosions
        for current_sum, prob in active.items():
            for face in range(1, sides + 1):
                new_sum = current_sum + face
                if face >= explode_min and not is_last:
                    next_active[new_sum] += prob * p
                else:
                    finished[new_sum] += prob * p
        active = next_active
        if not active:
            break

    return normalize_pmf(finished)


def exploding_pool_keep_highest_pmf(
    pool_size: int,
    sides: int,
    explode_min: int | None = None,
    max_explosions: int = 4,
) -> PMF:
    """Exact PMF of taking the maximum of `pool_size` independent exploding dice (Gumbel limit)."""
    single = single_exploding_die_pmf(sides, explode_min=explode_min, max_explosions=max_explosions)
    values = sorted(single.keys())
    cdf_prev = 0.0
    out: Dict[int, float] = {}
    for v in values:
        cdf_curr = cdf_prev + single[v]
        out[v] = (cdf_curr**pool_size) - (cdf_prev**pool_size)
        cdf_prev = cdf_curr
    return normalize_pmf(out)


# ---------------------------------------------------------------------------
# 4. Order Statistics: Roll N, Keep k-th Lowest/Highest (Discrete Beta Family)
# ---------------------------------------------------------------------------


def order_statistic_pmf(pool_size: int, sides: int, k_rank_from_lowest: int) -> PMF:
    """Exact PMF of the k-th order statistic (1 = lowest, pool_size = highest) of NdK."""
    if not (1 <= k_rank_from_lowest <= pool_size):
        raise ValueError(f"k_rank_from_lowest must be in 1..={pool_size}")

    def cdf(x: int) -> float:
        if x <= 0:
            return 0.0
        if x >= sides:
            return 1.0
        p = x / sides
        return sum(
            math.comb(pool_size, j) * (p**j) * ((1.0 - p) ** (pool_size - j))
            for j in range(k_rank_from_lowest, pool_size + 1)
        )

    out: Dict[int, float] = {}
    for face in range(1, sides + 1):
        out[face] = cdf(face) - cdf(face - 1)
    return normalize_pmf(out)


# ---------------------------------------------------------------------------
# 5. Distance-from-Center Selection: Focus vs. Neutral vs. Chaos (U-Shaped)
# ---------------------------------------------------------------------------


def center_distance_selection_pmf(
    pool_size: int,
    sides: int,
    mode: Literal["closest", "furthest"] = "furthest",
) -> PMF:
    """Roll `pool_size` dK and pick the die closest to or furthest from the center (K+1)/2.

    Ties in distance between a low face `f` and its mirror `K + 1 - f` are split 50/50
    so symmetry is preserved.
    """
    center = (sides + 1) / 2.0
    # Group faces by exact distance from center
    dist_groups: Dict[float, List[int]] = defaultdict(list)
    for face in range(1, sides + 1):
        dist = round(abs(face - center), 6)
        dist_groups[dist].append(face)

    ordered_dists = sorted(dist_groups.keys(), reverse=(mode == "furthest"))
    # Order-statistic approach over distance buckets:
    # A pool picks from bucket `d` if all dice fall in bucket `d` or worse buckets,
    # and at least one die lands in bucket `d`.
    out: Dict[int, float] = defaultdict(float)
    remaining_faces = sides
    for d in ordered_dists:
        faces_in_bucket = dist_groups[d]
        b = len(faces_in_bucket)
        # Probability all dice are in `remaining_faces`, minus all dice in `remaining_faces - b`
        p_bucket = ((remaining_faces / sides) ** pool_size) - (
            ((remaining_faces - b) / sides) ** pool_size
        )
        for f in faces_in_bucket:
            out[f] = p_bucket / b
        remaining_faces -= b

    return normalize_pmf(out)


# ---------------------------------------------------------------------------
# 6. Median-Gated Branching Pool (True Two-Humped Bimodal Distribution)
# ---------------------------------------------------------------------------


def median_gated_bimodal_pmf(
    pool_size: int,
    sides: int,
    gate_tn: int,
    roll_under: bool = False,
) -> PMF:
    """Roll an odd number of dice `pool_size` dK.

    Compare the median die against `gate_tn`:
    - If `roll_under=False`: if median >= gate_tn, return max(pool); else return min(pool).
    - If `roll_under=True`: if median <= gate_tn, return min(pool); else return max(pool).
    """
    if pool_size % 2 == 0 or pool_size < 3:
        raise ValueError("pool_size must be an odd integer >= 3 (e.g., 3 or 5)")

    # For clarity and speed on odd pools (3d20, 5d12, etc.), use exact dynamic programming
    # over sorted tuple counts or direct enumeration for pool_size <= 5.
    out: Dict[int, float] = defaultdict(float)
    mid_idx = pool_size // 2

    # DP over faces 1..sides: track (count_rolled, min_face, median_face, max_face)
    # Since pool_size is small (3 or 5), we can iterate compositions of `pool_size` across `sides`.
    def recurse(face: int, remaining: int, current_tuple: List[int], mult_denom: int) -> None:
        if remaining == 0:
            perm_count = math.factorial(pool_size) // mult_denom
            prob = perm_count / (sides**pool_size)
            mn = current_tuple[0]
            md = current_tuple[mid_idx]
            mx = current_tuple[-1]
            if roll_under:
                chosen = mn if md <= gate_tn else mx
            else:
                chosen = mx if md >= gate_tn else mn
            out[chosen] += prob
            return
        if face > sides:
            return
        # Option 0: 0 dice show `face`
        recurse(face + 1, remaining, current_tuple, mult_denom)
        # Option c >= 1 dice show `face`
        for c in range(1, remaining + 1):
            recurse(
                face + 1,
                remaining - c,
                current_tuple + [face] * c,
                mult_denom * math.factorial(c),
            )

    recurse(1, pool_size, [], 1)
    return normalize_pmf(out)


# ---------------------------------------------------------------------------
# 7. Multiplicative Dice Pool (Log-Normal Heavy-Tailed Distribution)
# ---------------------------------------------------------------------------


def multiplicative_pool_pmf(pool_size: int, sides: int) -> PMF:
    """Exact PMF of multiplying `pool_size` dK together (d1 * d2 * ... * dN)."""
    result: Dict[int, float] = {1: 1.0}
    p = 1.0 / sides
    for _ in range(pool_size):
        nxt: Dict[int, float] = defaultdict(float)
        for val, prob in result.items():
            for face in range(1, sides + 1):
                nxt[val * face] += prob * p
        result = nxt
    return normalize_pmf(result)


# ---------------------------------------------------------------------------
# 8. Paired-List Cascading / Contagious Dice (The Watershed Mechanic)
# ---------------------------------------------------------------------------


def paired_list_cascade_pmfs(
    num_pairs: int,
    sides: int,
    characteristic_c: int,
    delta_over: int = 3,
    delta_under: int = 3,
    mode: Literal["markov_step", "cumulative_snowball", "linear_margin"] = "markov_step",
    count_under_as_success: bool = False,
) -> Tuple[PMF, PMF]:
    """Exact PMFs for the Two-List Cascading Dice Mechanic.

    Two lists of `num_pairs` dK are rolled: A = [A_1..A_N], B = [B_1..B_N].
    At step n, base sum X_n = A_n + B_n, and effective value Y_n = X_n + mod_n.
    Y_n is compared to `characteristic_c` (C):
      - If Y_n > C: we add to step n+1 (+delta_over)
      - If Y_n < C: we subtract from step n+1 (-delta_under)
      - If Y_n == C: modifier is 0 (or unchanged in cumulative mode)

    Returns:
      (success_count_pmf, final_pair_value_pmf)
    """
    pair_pmf = sum_dice_pool_pmf(pos_dice=2, sides=sides)

    # DP state: (current_mod, successes_so_far, last_y) -> probability
    # First pair (n = 1) has mod_1 = 0
    state_probs: Dict[Tuple[int, int, int], float] = defaultdict(float)
    for x1, px in pair_pmf.items():
        y1 = x1
        is_hit = (y1 < characteristic_c) if count_under_as_success else (y1 > characteristic_c)
        succ = 1 if is_hit else 0
        if mode == "linear_margin":
            next_mod = y1 - characteristic_c
        else:
            if y1 > characteristic_c:
                next_mod = delta_over
            elif y1 < characteristic_c:
                next_mod = -delta_under
            else:
                next_mod = 0
        state_probs[(next_mod, succ, y1)] += px

    for _step in range(2, num_pairs + 1):
        next_states: Dict[Tuple[int, int, int], float] = defaultdict(float)
        for (mod_n, succ_so_far, _prev_y), p_state in state_probs.items():
            for xn, px in pair_pmf.items():
                yn = xn + mod_n
                is_hit = (
                    (yn < characteristic_c) if count_under_as_success else (yn > characteristic_c)
                )
                new_succ = succ_so_far + (1 if is_hit else 0)

                if mode == "markov_step":
                    if yn > characteristic_c:
                        next_mod = delta_over
                    elif yn < characteristic_c:
                        next_mod = -delta_under
                    else:
                        next_mod = 0
                elif mode == "cumulative_snowball":
                    if yn > characteristic_c:
                        next_mod = mod_n + delta_over
                    elif yn < characteristic_c:
                        next_mod = mod_n - delta_under
                    else:
                        next_mod = mod_n
                elif mode == "linear_margin":
                    next_mod = yn - characteristic_c
                else:
                    raise ValueError(f"Unknown mode: {mode}")

                next_states[(next_mod, new_succ, yn)] += p_state * px
        state_probs = next_states

    success_pmf: Dict[int, float] = defaultdict(float)
    final_y_pmf: Dict[int, float] = defaultdict(float)
    for (_mod, succ, final_y), prob in state_probs.items():
        success_pmf[succ] += prob
        final_y_pmf[final_y] += prob

    return normalize_pmf(success_pmf), normalize_pmf(final_y_pmf)


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

