"""Probability mass functions, Watershed 2d20 cascades & task gates backed by Rust."""

from __future__ import annotations

import random
from collections.abc import Sequence
from typing import Any

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


def _format_cascade_outcome_dict(out: CascadeRollOutcome) -> dict[str, Any]:
    return {
        "steps": [
            {
                "pair_index": int(s.pair_index),
                "die_a": int(s.die_a),
                "die_b": int(s.die_b),
                "raw_sum": int(s.raw_sum),
                "carry_in": int(s.carry_in),
                "effective_y": int(s.effective_y),
                "tn_eff": int(s.tn_eff),
                "succeeded": bool(s.succeeded),
                "step_botch": bool(s.step_botch),
                "miss_distance": int(s.miss_distance),
                "carry_out": int(s.carry_out),
            }
            for s in out.steps
        ],
        "total_successes": int(out.total_successes),
        "raw_botches": int(out.raw_botches),
        "effective_botches": int(out.effective_botches),
        "min_miss_tier": int(out.min_miss_tier),
        "signed_margin": int(out.signed_margin),
        "final_pair_y": int(out.final_pair_y),
        "outcome_kind": str(out.outcome_kind),
        "outcome_tier": str(out.outcome_tier),
    }


def evaluate_cascade_pairs_dict(
    skill_level: int,
    control_c: int,
    internal_mod: int,
    external_mod: int,
    pairs: Sequence[tuple[int, int]],
    carry_mode: str = "markov_step",
    botch_rule: str = "closest_miss",
) -> dict[str, Any]:
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
) -> dict[str, Any]:
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
        roller = rng if rng is not None else random.SystemRandom()
        pair_tuples = [(roller.randint(1, 20), roller.randint(1, 20)) for _ in range(n_pairs)]
    else:
        pair_tuples = [(int(a), int(b)) for a, b in list(pairs)[:n_pairs]]

    out = params.evaluate_pairs(pair_tuples)
    res = _format_cascade_outcome_dict(out)
    rolled_pairs = [[a, b] for a, b in pair_tuples]
    all_dice = [d for pair in pair_tuples for d in pair]
    res.update(
        {
            "skill_level": n_pairs,
            "control_c": c_stat,
            "attribute_tn": c_stat,
            "effective_tn": tn_eff,
            "internal_mod": int(internal_mod),
            "external_mod": int(difficulty_mod),
            "rolled_pairs": rolled_pairs,
            "dice_rolled_d20": all_dice,
            "successes_counted": int(out.total_successes),
            "margin": int(out.signed_margin),
            "degree_of_success": str(out.outcome_tier),
        }
    )
    return res


__all__ = [
    "CascadeRollOutcome",
    "CascadeStepTrace",
    "CascadeTaskParams",
    "ComplementarySynergyResult",
    "ContestedCascadeOdds",
    "ContestedCascadeRollOutcome",
    "ContestedCascadeTaskParams",
    "DistributionStats",
    "ExtendedProjectClock",
    "ExtendedProjectStepResult",
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
    "single_die_pmf",
    "single_exploding_die_pmf",
    "sum_dice_pool_pmf",
]
