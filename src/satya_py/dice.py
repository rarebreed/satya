"""Probability mass functions, 2d20xN cascades & task gates backed by Rust."""

from __future__ import annotations

import random
from collections.abc import Sequence
from typing import Final, TypedDict, cast

from satya_py._native import (
    CascadeRollOutcome,
    CascadeStepTrace,
    CascadeTaskParams,
    ComplementarySynergyResult,
    ContestedCascadeOdds,
    ContestedCascadeRollOutcome,
    ContestedCascadeTaskParams,
    DistributionStats,
    ExtendedProjectClock,
    ExtendedProjectStepResult,
    SignedCascadeResult,
    TaskGateResolution,
    center_distance_selection_pmf,
    classify_contested_outcome,
    compute_composite_characteristic,
    compute_contested_cascade_odds,
    compute_dual_skill_pool,
    compute_stats,
    compute_supporting_characteristic_shortfall_penalty,
    convolve_pmfs,
    count_successes_pmf,
    evaluate_cascade_roll,
    evaluate_complementary_setup_synergy,
    exploding_pool_keep_highest_pmf,
    median_gated_bimodal_pmf,
    multiplicative_pool_pmf,
    normalize_pmf,
    order_statistic_pmf,
    paired_list_cascade_pmfs,
    paired_list_signed_margin_pmfs,
    resolve_task_gate_and_tools,
    single_die_pmf,
    single_exploding_die_pmf,
    sum_dice_pool_pmf,
)

_RNG: Final[random.SystemRandom] = random.SystemRandom()


class CascadeStepDict(TypedDict):
    """Per-pair trace record produced from Rust `CascadeStepTrace`."""

    pair_index: int
    die_a: int
    die_b: int
    raw_sum: int
    carry_in: int
    effective_y: int
    tn_eff: int
    succeeded: bool
    step_botch: bool
    miss_distance: int
    carry_out: int


class CascadeOutcomeDict(TypedDict):
    """Plain dictionary representation of Rust `CascadeRollOutcome`."""

    steps: list[CascadeStepDict]
    total_successes: int
    raw_botches: int
    effective_botches: int
    min_miss_tier: int
    signed_margin: int
    final_pair_y: int
    outcome_kind: str
    outcome_tier: str


class LocalCascadeRollDict(CascadeOutcomeDict):
    """Full roll + summary envelope returned by `roll_cascade_local`."""

    skill_level: int
    control_c: int
    attribute_tn: int
    effective_tn: int
    internal_mod: int
    external_mod: int
    rolled_pairs: list[list[int]]
    dice_rolled_d20: list[int]
    successes_counted: int
    margin: int
    degree_of_success: str


def roll_d20_pair(rng: random.Random | None = None) -> tuple[int, int]:
    """Draw a single `(die_a, die_b)` `2d20` pair using `rng` or the canonical `SystemRandom` CSPRNG."""
    roller = rng if rng is not None else _RNG
    return (roller.randint(1, 20), roller.randint(1, 20))


def roll_d20_pairs(count: int, rng: random.Random | None = None) -> list[tuple[int, int]]:
    """Draw `max(1, min(10, count))` pairs of `(die_a, die_b)` `2d20` dice."""
    n = max(1, min(10, int(count)))
    roller = rng if rng is not None else _RNG
    return [(roller.randint(1, 20), roller.randint(1, 20)) for _ in range(n)]


def _format_cascade_outcome_dict(out: CascadeRollOutcome) -> CascadeOutcomeDict:
    return cast(CascadeOutcomeDict, out.to_dict())


def evaluate_cascade_pairs_dict(
    skill_level: int,
    control_c: int,
    internal_mod: int,
    external_mod: int,
    pairs: Sequence[tuple[int, int]],
    carry_mode: str = "markov_step",
    botch_rule: str = "closest_miss",
) -> CascadeOutcomeDict:
    """Evaluate concrete `(die_a, die_b)` pairs via Rust and return a plain dictionary."""
    clean_pairs = [(int(a), int(b)) for a, b in pairs]
    out = evaluate_cascade_roll(
        int(skill_level),
        int(control_c),
        int(internal_mod),
        int(external_mod),
        clean_pairs,
        carry_mode,
        botch_rule,
    )
    return _format_cascade_outcome_dict(out)


def roll_cascade_local(
    skill_rank: int,
    control_c: int,
    difficulty_mod: int = 0,
    internal_mod: int = 0,
    carry_mode: str = "markov_step",
    botch_rule: str = "closest_miss",
    *,
    rng: random.Random | None = None,
    pairs: Sequence[tuple[int, int]] | None = None,
) -> LocalCascadeRollDict:
    """Draw `N` `2d20` pairs and evaluate them deterministically with Rust ``CascadeTaskParams``."""
    n_pairs = max(1, min(10, int(skill_rank) if int(skill_rank) > 0 else 1))
    c_stat = max(1, min(20, int(control_c)))
    params = CascadeTaskParams(
        n_pairs,
        c_stat,
        int(internal_mod),
        int(difficulty_mod),
        carry_mode,
        botch_rule,
    )
    tn_eff = int(params.tn_eff())
    if pairs is None:
        pair_tuples = roll_d20_pairs(n_pairs, rng=rng)
    else:
        pair_tuples = [(int(a), int(b)) for a, b in list(pairs)[:n_pairs]]

    out = params.evaluate_pairs(pair_tuples)
    res = _format_cascade_outcome_dict(out)
    rolled_pairs = [[a, b] for a, b in pair_tuples]
    all_dice = [d for pair in pair_tuples for d in pair]
    return LocalCascadeRollDict(
        steps=res["steps"],
        total_successes=res["total_successes"],
        raw_botches=res["raw_botches"],
        effective_botches=res["effective_botches"],
        min_miss_tier=res["min_miss_tier"],
        signed_margin=res["signed_margin"],
        final_pair_y=res["final_pair_y"],
        outcome_kind=res["outcome_kind"],
        outcome_tier=res["outcome_tier"],
        skill_level=n_pairs,
        control_c=c_stat,
        attribute_tn=c_stat,
        effective_tn=tn_eff,
        internal_mod=int(internal_mod),
        external_mod=int(difficulty_mod),
        rolled_pairs=rolled_pairs,
        dice_rolled_d20=all_dice,
        successes_counted=int(out.total_successes),
        margin=int(out.signed_margin),
        degree_of_success=str(out.outcome_tier),
    )


__all__ = [
    "CascadeOutcomeDict",
    "CascadeRollOutcome",
    "CascadeStepDict",
    "CascadeStepTrace",
    "CascadeTaskParams",
    "ComplementarySynergyResult",
    "ContestedCascadeOdds",
    "ContestedCascadeRollOutcome",
    "ContestedCascadeTaskParams",
    "DistributionStats",
    "ExtendedProjectClock",
    "ExtendedProjectStepResult",
    "LocalCascadeRollDict",
    "SignedCascadeResult",
    "TaskGateResolution",
    "center_distance_selection_pmf",
    "classify_contested_outcome",
    "compute_composite_characteristic",
    "compute_contested_cascade_odds",
    "compute_dual_skill_pool",
    "compute_stats",
    "compute_supporting_characteristic_shortfall_penalty",
    "convolve_pmfs",
    "count_successes_pmf",
    "evaluate_cascade_pairs_dict",
    "evaluate_cascade_roll",
    "evaluate_complementary_setup_synergy",
    "exploding_pool_keep_highest_pmf",
    "median_gated_bimodal_pmf",
    "multiplicative_pool_pmf",
    "normalize_pmf",
    "order_statistic_pmf",
    "paired_list_cascade_pmfs",
    "paired_list_signed_margin_pmfs",
    "resolve_task_gate_and_tools",
    "roll_cascade_local",
    "roll_d20_pair",
    "roll_d20_pairs",
    "single_die_pmf",
    "single_exploding_die_pmf",
    "sum_dice_pool_pmf",
]
