"""Exact probability distribution engines and Plotly visualizations for Satya TTRPG dice mechanics.

Provides exact combinatorial / dynamic-programming probability mass functions (PMFs),
summary statistics (mean, std, skewness, excess kurtosis), and interactive Plotly figures
that can be rendered in Jupyter notebooks or serialized to JSON for a Satya Web UI widget.
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass
import math
from typing import TYPE_CHECKING, Any, Dict, List, Literal, Tuple

try:
    import satya_py as _satya_py

    HAS_SATYA_PY = True
except ImportError:
    _satya_py = None  # type: ignore[assignment]
    HAS_SATYA_PY = False

if TYPE_CHECKING:
    import plotly.graph_objects as go

PMF = Dict[int, float]


def using_satya_py_backend() -> bool:
    """Return True if the compiled Rust `satya_py` PyO3 extension module is active."""
    return HAS_SATYA_PY


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
    invert_feedback: bool = False,
) -> Tuple[PMF, PMF]:
    """Compute exact PMFs for the Two-List Cascading ("Watershed" or "Recoil") Dice Mechanic.

    Two lists of `num_pairs` dice (`sides`-sided, dK) are rolled in parallel:
        List A = [A_1, A_2, ..., A_N]
        List B = [B_1, B_2, ..., B_N]

    At each index n (from 1 to N):
      1. The base pair sum is X_n = A_n + B_n (ranging from 2 to 2*sides, mean = sides + 1).
      2. The effective value is Y_n = X_n + mod_n (where mod_1 = 0 on the first pair).
      3. Y_n is compared to the controlling characteristic `characteristic_c` (C) to determine
          both whether step n succeeded and what modifier `mod_{n+1}` propagates to step n+1.

    Args:
        num_pairs: Length (N) of each dice list (i.e., total number of paired rolls A_n + B_n
            evaluated in sequence).
        sides: Number of faces (K) on each individual die in List A and List B (e.g., 6 for d6,
            10 for d10, 20 for d20). Each pair A_n + B_n has range [2, 2*sides] and mean `sides + 1`.
        characteristic_c: The controlling characteristic / target threshold (C) against which each
            effective pair total Y_n is compared.
        delta_over: Magnitude of modifier applied to step n+1 when `Y_n > characteristic_c`.
            When `invert_feedback=False`, adds `+delta_over`; when `invert_feedback=True`,
            subtracts `-delta_over`. Ignored when `mode="linear_margin"`. Defaults to 3.
        delta_under: Magnitude of modifier applied to step n+1 when `Y_n < characteristic_c`.
            When `invert_feedback=False`, subtracts `-delta_under`; when `invert_feedback=True`,
            adds `+delta_under`. Ignored when `mode="linear_margin"`. Defaults to 3.
        mode: Propagation rule governing how `mod_{n+1}` is calculated from step n:
            - `"markov_step"` (default): 1-step memory (`mod_{n+1}` depends only on step n).
            - `"cumulative_snowball"`: Compounding momentum (`mod_{n+1} = mod_n + step_mod`).
            - `"linear_margin"`: Exact difference carry (`mod_{n+1} = Y_n - C`, or `C - Y_n` when
              `invert_feedback=True`).
        count_under_as_success: Controls which side of `characteristic_c` counts as a "success" in
            the returned `success_count_pmf`:
            - `False` (default, Roll-Over): `Y_n > characteristic_c` counts as 1 success.
            - `True` (Roll-Under): `Y_n < characteristic_c` counts as 1 success.
        invert_feedback: Reverses the sign of the carry rule:
            - `False` (default, Positive Feedback / Watershed Repeller):
              `Y_n > C` adds `+delta_over` to n+1; `Y_n < C` subtracts `-delta_under` from n+1.
              Pushes subsequent rolls *away* from C (amplifying streaks -> Bimodal U-curve).
            - `True` (Negative Feedback / Recoil & Homeostatic Attractor):
              `Y_n < C` adds `+delta_under` to n+1; `Y_n > C` subtracts `-delta_over` from n+1.
              Pushes subsequent rolls *back toward* C (suppressing streaks -> Sub-Binomial spike
              and pulling `Y_N` toward `characteristic_c`).

    Returns:
        A 2-tuple of normalized probability mass functions `(success_count_pmf, final_pair_value_pmf)`:
        - `success_count_pmf`: `Dict[int, float]` mapping `{k: P(total successes == k)}` for
          `k` in `0..=num_pairs`.
        - `final_pair_value_pmf`: `Dict[int, float]` mapping `{y: P(Y_N == y)}` for the effective
          value of the final pair in the chain.
    """
    pair_pmf = sum_dice_pool_pmf(pos_dice=2, sides=sides)
    sign = -1 if invert_feedback else 1
    eff_over = sign * delta_over
    eff_under = -sign * delta_under

    # DP state: (current_mod, successes_so_far, last_y) -> probability
    # First pair (n = 1) has mod_1 = 0
    state_probs: Dict[Tuple[int, int, int], float] = defaultdict(float)
    for x1, px in pair_pmf.items():
        y1 = x1
        is_hit = (y1 < characteristic_c) if count_under_as_success else (y1 > characteristic_c)
        succ = 1 if is_hit else 0
        if mode == "linear_margin":
            next_mod = sign * (y1 - characteristic_c)
        else:
            if y1 > characteristic_c:
                next_mod = eff_over
            elif y1 < characteristic_c:
                next_mod = eff_under
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
                        next_mod = eff_over
                    elif yn < characteristic_c:
                        next_mod = eff_under
                    else:
                        next_mod = 0
                elif mode == "cumulative_snowball":
                    if yn > characteristic_c:
                        next_mod = mod_n + eff_over
                    elif yn < characteristic_c:
                        next_mod = mod_n + eff_under
                    else:
                        next_mod = mod_n
                elif mode == "linear_margin":
                    next_mod = sign * (yn - characteristic_c)
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


@dataclass(frozen=True)
class SignedCascadeResult:
    """Complete outcome breakdown including Degrees of Failure and Degrees of Success.

    Attributes:
        signed_margin_pmf: Unified PMF over `-num_pairs .. +num_pairs`.
            - `> 0`: Margin of Success (`+1` = bare minimum, `+2..+N` = higher degrees of success).
            - `0`: Clean Failure (`0` successes, `0` botches).
            - `< 0`: Degree of Failure (`-1` = minor setback, `-2` = severe, `-3..-N` = catastrophic).
        success_count_pmf: PMF of raw success counts `0..num_pairs`.
        botch_count_pmf: PMF of raw botch/overrun counts `0..num_pairs`.
        final_pair_value_pmf: PMF of the final pair's effective value `Y_N`.
        failure_overshoot_pmf: Conditional PMF of `Y_N - C_eff` (or `C_eff - Y_N` in roll-over)
            given that the task failed (`successes == 0`).
        prob_any_success: Total probability of achieving at least 1 success (`successes >= 1`).
        prob_success_with_complication: Probability of succeeding while also triggering at least
            1 botch (`successes >= 1` and `botches >= 1` -> "Success at a Cost").
        prob_clean_failure: Probability of failing with 0 botches (`successes == 0` and `botches == 0`).
        prob_botch_failure: Probability of failing with 1 or more botches (`successes == 0` and `botches >= 1`).
    """

    signed_margin_pmf: PMF
    success_count_pmf: PMF
    botch_count_pmf: PMF
    final_pair_value_pmf: PMF
    failure_overshoot_pmf: PMF
    prob_any_success: float
    prob_success_with_complication: float
    prob_clean_failure: float
    prob_botch_failure: float


def paired_list_signed_margin_pmfs(
    num_pairs: int,
    sides: int,
    characteristic_c: int,
    delta_over: int = 3,
    delta_under: int = 3,
    mode: Literal["markov_step", "cumulative_snowball", "linear_margin"] = "markov_step",
    count_under_as_success: bool = True,
    invert_feedback: bool = False,
    botch_rule: Literal["closest_miss", "ceiling_breach", "overrun_k"] = "closest_miss",
    margin_aggregation: Literal["piecewise_zero_floor", "net_difference"] = "piecewise_zero_floor",
) -> SignedCascadeResult:
    """Compute the full `-N .. +N` Signed Outcome Spectrum (Degrees of Failure & Success).

    Uses only the existing cascade levers (`num_pairs`, `sides`, `characteristic_c`,
    `delta_over`, `delta_under`, `mode`) by partitioning each pair roll `Y_n` into three zones:
      1. Controlled / Success Zone:
         - Roll-Under (`count_under_as_success=True`): `Y_n < characteristic_c`
         - Roll-Over (`count_under_as_success=False`): `Y_n > characteristic_c`
      2. Contained Miss Zone (0 successes, 0 botches):
         - Roll missed the success zone, but did not breach the botch threshold.
      3. Botch / Overrun Zone (+1 botch):
        - `"closest_miss"` (default, Monotonic / Anti-Paradox):
          When `successes == 0`, failure severity is determined by the *closest* pair to `C_eff`
          across all `N` pairs: `min_miss = min_n (Y_n - C_eff)` (binned every 2 points:
          `0..1 -> 0 Clean Miss`, `2..3 -> -1 Minor`, `4..5 -> -2 Severe`, `6+ -> -3..`).
          Because adding more pairs `N` can only *reduce* the minimum of `N` rolls, rolling more
          dice **strictly reduces** both failure probability and failure severity!
        - `"ceiling_breach"`: `Y_n > 2 * sides` (in Roll-Under) or `Y_n < 2` (in Roll-Over).
          Can only occur when a carried penalty pushes a subsequent pair beyond the natural
          bounds `[2, 2*sides]` of the dice!
        - `"overrun_k"`: `Y_n > characteristic_c + sides` (in Roll-Under) or
          `Y_n < characteristic_c - sides` (in Roll-Over). Scales blunder risk when `C_eff` is
          very low relative to the task difficulty.

    Args:
        num_pairs: Length (N) of each of the two dice lists A and B.
        sides: Die size (K) for each die in lists A and B.
        characteristic_c: Effective controlling characteristic `C_eff = C - Delta_ext`.
        delta_over: Magnitude applied to step n+1 when `Y_n > characteristic_c` (in Roll-Under,
            this is the Fatigue / Instability penalty `+delta_over`).
        delta_under: Magnitude applied to step n+1 when `Y_n < characteristic_c` (in Roll-Under,
            this is the Skill technique bonus `-delta_under`).
        mode: `"markov_step"`, `"cumulative_snowball"`, or `"linear_margin"`.
        count_under_as_success: `True` for Roll-Under (`Y_n < C_eff` is success), `False` for Roll-Over.
        invert_feedback: `False` for Watershed Repeller, `True` for Recoil Attractor.
        botch_rule: `"ceiling_breach"` (`Y_n > 2K`) or `"overrun_k"` (`Y_n > C_eff + K`).
        margin_aggregation:
            - `"piecewise_zero_floor"`: If `successes > 0`, margin is `+successes`; if
              `successes == 0`, margin is `-botches` (`0` = clean miss, `-1..-N` = botch severity).
            - `"net_difference"`: Margin is `successes - botches` across the whole chain.

    Returns:
        `SignedCascadeResult` containing `signed_margin_pmf` (`-N..+N`) and full breakdown stats.
    """
    pair_pmf = sum_dice_pool_pmf(pos_dice=2, sides=sides)
    sign = -1 if invert_feedback else 1
    eff_over = sign * delta_over
    eff_under = -sign * delta_under

    def is_success_and_botch(y_val: int) -> Tuple[int, int, int]:
        if count_under_as_success:
            succ = 1 if y_val < characteristic_c else 0
            miss_dist = max(0, y_val - characteristic_c)
            if botch_rule in ("ceiling_breach", "closest_miss"):
                botch = 1 if y_val > (2 * sides) else 0
            elif botch_rule == "overrun_k":
                botch = 1 if y_val > (characteristic_c + sides) else 0
            else:
                raise ValueError(f"Unknown botch_rule: {botch_rule}")
            if succ == 1:
                botch = 0
            return succ, botch, miss_dist
        else:
            succ = 1 if y_val > characteristic_c else 0
            miss_dist = max(0, characteristic_c - y_val)
            if botch_rule in ("ceiling_breach", "closest_miss"):
                botch = 1 if y_val < 2 else 0
            elif botch_rule == "overrun_k":
                botch = 1 if y_val < (characteristic_c - sides) else 0
            else:
                raise ValueError(f"Unknown botch_rule: {botch_rule}")
            if succ == 1:
                botch = 0
            return succ, botch, miss_dist

    tier_step = max(2, sides // 5)

    # DP state for steps 1..N-1: (current_mod, successes_so_far, botches_so_far, min_miss_tier) -> prob
    state_probs: Dict[Tuple[int, int, int, int], float] = defaultdict(float)
    final_y_pmf: Dict[int, float] = defaultdict(float)
    overshoot_raw: Dict[int, float] = defaultdict(float)

    for x1, px in pair_pmf.items():
        y1 = x1
        s1, b1, m1 = is_success_and_botch(y1)
        m1_tier = min(5, m1 // tier_step)
        if mode == "linear_margin":
            next_mod = sign * (y1 - characteristic_c)
        else:
            if y1 > characteristic_c:
                next_mod = eff_over
            elif y1 < characteristic_c:
                next_mod = eff_under
            else:
                next_mod = 0
        state_probs[(next_mod, s1, b1, m1_tier)] += px
        if num_pairs == 1:
            final_y_pmf[y1] += px
            if s1 == 0:
                overshoot_raw[m1] += px

    for step in range(2, num_pairs + 1):
        is_last = step == num_pairs
        next_states: Dict[Tuple[int, int, int, int], float] = defaultdict(float)
        for (mod_n, succ_so_far, botch_so_far, min_miss_so_far), p_state in state_probs.items():
            for xn, px in pair_pmf.items():
                yn = xn + mod_n
                sn, bn, mn = is_success_and_botch(yn)
                new_succ = succ_so_far + sn
                new_botch = botch_so_far + bn
                new_min_miss = min(min_miss_so_far, min(5, mn // tier_step))
                joint_p = p_state * px

                if is_last:
                    final_y_pmf[yn] += joint_p
                    if new_succ == 0:
                        overshoot = (
                            (yn - characteristic_c)
                            if count_under_as_success
                            else (characteristic_c - yn)
                        )
                        overshoot_raw[overshoot] += joint_p

                if mode == "markov_step":
                    if yn > characteristic_c:
                        next_mod = eff_over
                    elif yn < characteristic_c:
                        next_mod = eff_under
                    else:
                        next_mod = 0
                elif mode == "cumulative_snowball":
                    if yn > characteristic_c:
                        next_mod = mod_n + eff_over
                    elif yn < characteristic_c:
                        next_mod = mod_n + eff_under
                    else:
                        next_mod = mod_n
                elif mode == "linear_margin":
                    next_mod = sign * (yn - characteristic_c)
                else:
                    raise ValueError(f"Unknown mode: {mode}")

                next_states[(next_mod, new_succ, new_botch, new_min_miss)] += joint_p
        state_probs = next_states

    signed_margin_pmf: Dict[int, float] = defaultdict(float)
    success_pmf: Dict[int, float] = defaultdict(float)
    botch_pmf: Dict[int, float] = defaultdict(float)

    prob_any_succ = 0.0
    prob_succ_with_comp = 0.0
    prob_clean_fail = 0.0
    prob_botch_fail = 0.0

    for (_mod, succ, botch, min_miss_tier), prob in state_probs.items():
        eff_botch = min_miss_tier if (botch_rule == "closest_miss" and succ == 0) else botch
        success_pmf[succ] += prob
        botch_pmf[eff_botch] += prob

        if margin_aggregation == "piecewise_zero_floor":
            margin = succ if succ > 0 else -eff_botch
        elif margin_aggregation == "net_difference":
            margin = succ - eff_botch
        else:
            raise ValueError(f"Unknown margin_aggregation: {margin_aggregation}")

        signed_margin_pmf[margin] += prob

        if succ >= 1:
            prob_any_succ += prob
            if botch >= 1:
                prob_succ_with_comp += prob
        else:
            if eff_botch == 0:
                prob_clean_fail += prob
            else:
                prob_botch_fail += prob

    return SignedCascadeResult(
        signed_margin_pmf=normalize_pmf(signed_margin_pmf),
        success_count_pmf=normalize_pmf(success_pmf),
        botch_count_pmf=normalize_pmf(botch_pmf),
        final_pair_value_pmf=normalize_pmf(final_y_pmf),
        failure_overshoot_pmf=normalize_pmf(overshoot_raw),
        prob_any_success=prob_any_succ,
        prob_success_with_complication=prob_succ_with_comp,
        prob_clean_failure=prob_clean_fail,
        prob_botch_failure=prob_botch_fail,
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


# ---------------------------------------------------------------------------
# 11. Satya Rules Book Interactive Simulators (Tactical Pool, Lifepath, Social)
# ---------------------------------------------------------------------------


def compute_tactical_pool_envelope(
    control_c: int,
    skill_level: int,
    delta_int: int = 0,
    delta_ext: int = 0,
    num_successes: int = 2,
    weapon_reach: int = 2,
    opponent_reach: int = 2,
) -> Dict[str, Any]:
    """Compute the Satya Combat Tactical Pool ('Envelope & Fill') from Chapters 7 & 8.

    Delegates to `satya_py.py_compute_tactical_pool` when the Rust PyO3 extension is installed.
    """
    if HAS_SATYA_PY and _satya_py is not None and hasattr(_satya_py, "compute_tactical_pool"):
        env = _satya_py.compute_tactical_pool(
            skill_c=int(control_c),
            skill_n=max(0, int(skill_level)),
            fatigue_level=max(0, -int(delta_int)),
        )
        reach_delta = int(weapon_reach) - int(opponent_reach)
        reach_mod = max(-2, min(3, reach_delta))
        return {
            "base_envelope": int(env.total_pool_b),
            "raw_pool": int(env.total_pool_b) + max(0, int(num_successes)) + reach_mod,
            "effective_pool": int(env.total_pool_b),
            "overflow_wasted": max(0, int(num_successes) + reach_mod),
            "reach_modifier": reach_mod,
            "max_tempo": int(env.max_tempo),
            "max_control": int(env.max_control),
            "max_force": int(env.max_force),
            "backend": "satya_py (Rust PyO3)",
        }

    c_clamped = max(1, min(20, int(control_c)))
    n_clamped = max(0, min(10, int(skill_level)))
    s_clamped = max(0, int(num_successes))
    base_envelope = max(
        1,
        ((c_clamped + 2 * n_clamped) // 3) + int(delta_int) - max(0, int(delta_ext) // 2),
    )
    reach_delta = int(weapon_reach) - int(opponent_reach)
    reach_mod = max(-2, min(3, reach_delta))
    raw_pool = max(0, s_clamped * 3 + max(0, n_clamped // 2) + reach_mod)
    effective_pool = min(raw_pool, base_envelope)
    overflow = max(0, raw_pool - base_envelope)
    return {
        "base_envelope": base_envelope,
        "raw_pool": raw_pool,
        "effective_pool": effective_pool,
        "overflow_wasted": overflow,
        "reach_modifier": reach_mod,
        "backend": "satya_ml (Python)",
    }


def evaluate_tactical_triad_contest(
    attacker_tempo: int,
    attacker_control: int,
    attacker_force: int,
    defender_tempo: int,
    defender_control: int,
    defender_force: int,
    attacker_root: int = 12,
    defender_guard: int = 4,
    armor_dr: int = 3,
) -> Dict[str, Any]:
    """Evaluate a Close-Quarters Tactical Triad (`Tempo`, `Control`, `Force`) exchange from Chapter 8."""
    archetype_label = "Custom Triad"
    if HAS_SATYA_PY and _satya_py is not None and hasattr(_satya_py, "compute_tactical_pool"):
        env = _satya_py.compute_tactical_pool(skill_c=14, skill_n=5)
        eval_res = env.evaluate_triad(
            int(attacker_tempo),
            int(attacker_control),
            int(attacker_force),
            10,
        )
        archetype_label = str(eval_res.archetype_label)

    atk_t = max(0, int(attacker_tempo))
    atk_c = max(0, int(attacker_control))
    atk_f = max(0, int(attacker_force))
    def_t = max(0, int(defender_tempo))
    def_c = max(0, int(defender_control))
    def_f = max(0, int(defender_force))

    net_tempo = atk_t - def_t
    net_control = atk_c - def_c
    seizes_initiative = net_tempo >= 2 and net_control >= 0
    guard_broken = (atk_c + max(0, atk_f // 2)) > (def_c + max(0, int(defender_guard)))
    unbalance = max(0, (atk_c + atk_f) - (def_c + def_f))
    throw_triggered = unbalance >= max(1, int(attacker_root) // 3)
    raw_impact = atk_f + (max(0, net_control) if guard_broken else 0)
    penetrating_force = max(0, raw_impact - max(0, int(armor_dr)) - (0 if guard_broken else def_c))
    next_tick_cost = max(3, 10 - min(5, atk_t) + max(0, def_c // 2))

    if throw_triggered and penetrating_force >= 3:
        summary = "Decisive Takedown & Armor Breach (Prone + Shock)"
    elif throw_triggered:
        summary = "Kuzushi Throw / Sweep (Target Off-Balance / Prone)"
    elif guard_broken and penetrating_force > 0:
        summary = "Guard Broken & Penetrating Strike"
    elif seizes_initiative:
        summary = "Initiative Seized (Tempo Advantage on Next Exchange)"
    elif penetrating_force > 0:
        summary = "Glancing Penetrating Blow"
    else:
        summary = "Deflected on Guard / Bind Equilibrium"

    return {
        "net_tempo": net_tempo,
        "net_control": net_control,
        "seizes_initiative": seizes_initiative,
        "guard_broken": guard_broken,
        "throw_triggered": throw_triggered,
        "penetrating_force": penetrating_force,
        "next_tick_cost": next_tick_cost,
        "archetype_label": archetype_label,
        "outcome_summary": summary,
        "backend": "satya_py (Rust PyO3)" if HAS_SATYA_PY else "satya_ml (Python)",
    }


def compute_lifepath_skill_progression(
    hours_studied: float,
    difficulty_multiplier: float = 1.0,
    instruction_quality: float = 1.0,
    aptitude_bonus: int = 0,
) -> Dict[str, Any]:
    """Compute non-linear skill level `N` (`0..10`) from study hours using Chapter 6's exponential curve.

    Each rank `N >= 1` requires `100 * (2 ** (N - 1)) * difficulty_multiplier` effective hours.
    """
    eff_hours = max(0.0, float(hours_studied)) * max(0.25, float(instruction_quality)) * (
        1.0 + 0.08 * int(aptitude_bonus)
    )
    diff = max(0.5, float(difficulty_multiplier))
    thresholds: List[Tuple[int, float]] = []
    cumulative = 0.0
    achieved_level = 0
    for lvl in range(1, 11):
        step_req = 100.0 * (2 ** (lvl - 1)) * diff
        cumulative += step_req
        thresholds.append((lvl, round(cumulative, 1)))
        if eff_hours >= cumulative:
            achieved_level = lvl

    next_level = min(10, achieved_level + 1)
    next_threshold = thresholds[next_level - 1][1] if achieved_level < 10 else thresholds[-1][1]
    hours_to_next = max(0.0, round(next_threshold - eff_hours, 1))
    return {
        "raw_hours": round(float(hours_studied), 1),
        "effective_hours": round(eff_hours, 1),
        "skill_level_n": achieved_level,
        "next_level_n": next_level,
        "hours_to_next_level": hours_to_next,
        "cumulative_thresholds": dict(thresholds),
    }


def compute_social_dissonance_check(
    argument_margin: int,
    target_conviction: int,
    evidence_tier: int,
    shared_values_bonus: int = 0,
    public_audience_pressure: int = 0,
) -> Dict[str, Any]:
    """Compute Cognitive Dissonance pressure and belief-shift outcome from Chapter 9 (`social_interactions.md`)."""
    margin = int(argument_margin)
    conviction = max(1, min(20, int(target_conviction)))
    evidence = max(0, min(5, int(evidence_tier)))
    shared = max(-3, min(5, int(shared_values_bonus)))
    audience = max(-3, min(5, int(public_audience_pressure)))

    dissonance_pressure = max(0, margin * 2 + evidence * 3 + shared * 2 + audience)
    resistance_threshold = conviction + max(0, (conviction - 10) // 2)
    net_shift = dissonance_pressure - resistance_threshold

    if margin <= 0 and net_shift < -4:
        state = "Backfire / Entrenchment (Target hardens belief +2 Conviction)"
    elif net_shift < 0:
        state = "Rationalized Dismissal (Target absorbs tension without shifting)"
    elif net_shift < 5:
        state = "Seed of Doubt (+1 Dissonance Fatigue; opens follow-up concession)"
    elif net_shift < 10:
        state = "Tactical Concession (Target yields immediate behavioral compliance)"
    else:
        state = "Paradigm Fracture / Core Belief Shift (Lasting alignment change)"

    return {
        "dissonance_pressure": dissonance_pressure,
        "resistance_threshold": resistance_threshold,
        "net_dissonance_shift": net_shift,
        "social_outcome": state,
    }


# ---------------------------------------------------------------------------
# 10. Contested & Hybrid-Contested Two-List Cascade Resolution Engine
# ---------------------------------------------------------------------------

ContestedTieRuleLiteral = Literal[
    "hybrid_signed_margin",
    "strict_signed_margin",
    "active_player_wins_ties",
]


def classify_contested_outcome(
    sp: int,
    bp: int,
    mp: int,
    sg: int,
    bg: int,
    mg: int,
    tie_rule: ContestedTieRuleLiteral = "hybrid_signed_margin",
) -> Tuple[str, int]:
    """Classify a single `(player, gm)` state realization into a contested outcome kind and net margin."""
    _ = bg
    net_margin = mp - mg
    if tie_rule == "hybrid_signed_margin":
        if sp > sg:
            if bp >= 1 and mg >= 0:
                return ("SUCCESS_WITH_FLOOR_BREACH", net_margin)
            if net_margin >= 3:
                return ("EXTRAORDINARY_SUCCESS", net_margin)
            return ("CLEAN_SUCCESS", net_margin)
        if sp == sg and sp >= 1:
            return ("GRAZING_TIE_SUCCESS", net_margin)
        if sp == 0 and mp == 0 and mg < 0:
            return ("LUCKY_BREAK_SUCCESS", net_margin)
        if mp < 0 or (sp < sg and bp >= 1):
            return ("BOTCH_FAILURE", net_margin)
        return ("CLEAN_FAILURE", net_margin)

    if tie_rule == "strict_signed_margin":
        if sp > sg:
            if bp >= 1 and mg >= 0:
                return ("SUCCESS_WITH_FLOOR_BREACH", net_margin)
            if net_margin >= 3:
                return ("EXTRAORDINARY_SUCCESS", net_margin)
            return ("CLEAN_SUCCESS", net_margin)
        if sp == 0 and mp == 0 and mg < 0:
            return ("LUCKY_BREAK_SUCCESS", net_margin)
        if mp < 0 or (sp <= sg and bp >= 1):
            return ("BOTCH_FAILURE", net_margin)
        return ("CLEAN_FAILURE", net_margin)

    # active_player_wins_ties
    if sp >= sg and sp >= 1:
        if bp >= 1:
            return ("SUCCESS_WITH_FLOOR_BREACH", net_margin)
        if (sp - sg) >= 3:
            return ("EXTRAORDINARY_SUCCESS", net_margin)
        return ("CLEAN_SUCCESS", net_margin)
    if mp < 0 or bp >= 1:
        return ("BOTCH_FAILURE", net_margin)
    return ("CLEAN_FAILURE", net_margin)


def _cascade_joint_outcome_pmf(
    n_pairs: int,
    sides: int,
    tn_eff: int,
    delta_over: int,
    delta_under: int,
    carry_mode: str = "markov_step",
    botch_rule: str = "closest_miss",
    margin_aggregation: str = "piecewise_zero_floor",
) -> Dict[Tuple[int, int, int], float]:
    """Compute the exact joint PMF over `(succ, raw_botch, signed_margin)` for a cascade configuration."""
    base_pmf = sum_dice_pool_pmf(2, sides, 0)
    tier_step = max(2, sides // 5)
    state_probs: Dict[Tuple[int, int, int, int], float] = {(0, 0, 0, 5): 1.0}

    for step in range(n_pairs):
        next_states: Dict[Tuple[int, int, int, int], float] = defaultdict(float)
        for (mod_n, succ_so_far, botch_so_far, min_miss_tier), p_state in state_probs.items():
            for x, p_roll in base_pmf.items():
                yn = x + mod_n
                sn, bn, mn = _evaluate_step_success_and_botch(
                    yn=yn,
                    sides=sides,
                    characteristic_c=tn_eff,
                    count_under_as_success=False,
                    botch_rule=botch_rule,
                    strict_comparison=True,
                )
                new_succ = succ_so_far + sn
                new_botch = botch_so_far + bn
                miss_tier = min(5, mn // tier_step)
                new_min_miss = min(min_miss_tier, miss_tier)
                jp = p_state * p_roll

                if step == n_pairs - 1:
                    next_mod = 0
                elif carry_mode == "markov_step":
                    next_mod = delta_over if yn > tn_eff else (-delta_under if yn < tn_eff else 0)
                elif carry_mode == "cumulative_snowball":
                    next_mod = (
                        mod_n + delta_over
                        if yn > tn_eff
                        else (mod_n - delta_under if yn < tn_eff else mod_n)
                    )
                else:
                    next_mod = yn - tn_eff

                next_states[(next_mod, new_succ, new_botch, new_min_miss)] += jp
        state_probs = next_states

    joint: Dict[Tuple[int, int, int], float] = defaultdict(float)
    for (_mod, succ, raw_botch, min_miss_tier), prob in state_probs.items():
        eff_botch = min_miss_tier if (botch_rule == "closest_miss" and succ == 0) else raw_botch
        if margin_aggregation == "piecewise_zero_floor":
            signed_margin = succ if succ > 0 else -eff_botch
        else:
            signed_margin = succ - eff_botch
        joint[(succ, raw_botch, signed_margin)] += prob
    return dict(joint)


def _tiered_single_pair_gm_joint_pmf(
    sides: int,
    tn_gm: int,
    step_size: int = 4,
    strict_comparison: bool = True,
) -> Dict[Tuple[int, int, int], float]:
    """Compute the exact joint PMF over `(succ, raw_botch, signed_margin)` for a 1-Pair Tiered GM roll."""
    base_pmf = sum_dice_pool_pmf(2, sides, 0)
    under_step = max(2, sides // 5)
    over_step = max(1, step_size)
    joint: Dict[Tuple[int, int, int], float] = defaultdict(float)

    for x, p_roll in base_pmf.items():
        if strict_comparison:
            is_succ = x > tn_gm
            over_margin = x - tn_gm - 1
            under_margin = tn_gm - x
        else:
            is_succ = x >= tn_gm
            over_margin = x - tn_gm
            under_margin = tn_gm - x - 1

        if is_succ:
            succ = 1 + max(0, over_margin) // over_step
            joint[(succ, 0, succ)] += p_roll
        else:
            botch = min(5, max(0, under_margin) // under_step)
            joint[(0, botch, -botch)] += p_roll

    return dict(joint)


def compute_contested_cascade_odds(
    player_skill_n: int,
    player_control_c: int = 10,
    player_internal_mod: int = 0,
    player_known_tool_mod: int = 0,
    gm_pairs: int = 1,
    hidden_external_difficulty: int = 0,
    tie_rule: ContestedTieRuleLiteral = "hybrid_signed_margin",
    carry_mode: str = "markov_step",
    gm_mode: Literal["tiered_single_pair", "full_cascade"] = "tiered_single_pair",
    gm_over_tier_step: int = 4,
) -> Dict[str, Any]:
    """Compute exact combinatorial probabilities for a Contested Two-List Cascade check."""
    tn_p = 31 - player_control_c + player_known_tool_mod
    d_under_p = max(0, 6 + player_internal_mod)
    tn_gm = 21 - hidden_external_difficulty

    p_joint = _cascade_joint_outcome_pmf(
        n_pairs=player_skill_n,
        sides=20,
        tn_eff=tn_p,
        delta_over=6,
        delta_under=d_under_p,
        carry_mode=carry_mode,
    )
    if gm_mode == "tiered_single_pair":
        g_joint = _tiered_single_pair_gm_joint_pmf(
            sides=20,
            tn_gm=tn_gm,
            step_size=gm_over_tier_step,
        )
    else:
        g_joint = _cascade_joint_outcome_pmf(
            n_pairs=gm_pairs,
            sides=20,
            tn_eff=tn_gm,
            delta_over=6,
            delta_under=6,
            carry_mode=carry_mode,
        )

    net_margin_map: Dict[int, float] = defaultdict(float)
    net_succ_map: Dict[int, float] = defaultdict(float)
    probs: Dict[str, float] = defaultdict(float)

    for (sp, bp, mp), pp in p_joint.items():
        for (sg, bg, mg), pg in g_joint.items():
            jp = pp * pg
            kind, net_m = classify_contested_outcome(sp, bp, mp, sg, bg, mg, tie_rule)
            net_margin_map[net_m] += jp
            net_succ_map[sp - sg] += jp
            probs[kind] += jp

    net_signed_margin_pmf = normalize_pmf(net_margin_map)
    net_success_pmf = normalize_pmf(net_succ_map)
    stats = compute_stats(net_signed_margin_pmf)
    denom = stats.excess_kurtosis + 3.0
    bc = (stats.skewness**2 + 1.0) / denom if denom > 1e-9 else 0.0

    p_extr = probs["EXTRAORDINARY_SUCCESS"]
    p_clean = probs["CLEAN_SUCCESS"]
    p_flr = probs["SUCCESS_WITH_FLOOR_BREACH"]
    p_graze = probs["GRAZING_TIE_SUCCESS"]
    p_lucky = probs["LUCKY_BREAK_SUCCESS"]
    p_cfail = probs["CLEAN_FAILURE"]
    p_bfail = probs["BOTCH_FAILURE"]

    return {
        "net_signed_margin_pmf": net_signed_margin_pmf,
        "net_success_pmf": net_success_pmf,
        "prob_total_win": p_extr + p_clean + p_flr + p_graze + p_lucky,
        "prob_clean_or_extraordinary_win": p_extr + p_clean,
        "prob_extraordinary_success": p_extr,
        "prob_clean_success": p_clean,
        "prob_partial_success": p_flr + p_graze + p_lucky,
        "prob_floor_breach_partial": p_flr,
        "prob_grazing_tie_partial": p_graze,
        "prob_lucky_break_partial": p_lucky,
        "prob_clean_failure": p_cfail,
        "prob_botch_failure": p_bfail,
        "net_margin_stats": stats,
        "bimodality_coefficient": bc,
    }






