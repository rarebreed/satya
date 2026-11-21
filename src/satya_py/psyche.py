"""9-axis Psyche, social combat, cognitive dissonance & morale resolution backed by Rust."""

from __future__ import annotations

import json
from collections.abc import Sequence
from typing import Any

from satya_py._native import (
    apply_psyche_drift_mark_json,
    compute_social_dissonance_check,
    evaluate_combat_morale_roll_json,
    evaluate_combat_motivation_json,
    evaluate_conviction_surge_json,
    evaluate_emotional_trigger_json,
    evaluate_empathy_read_json,
    evaluate_psyche_friction_check,
    evaluate_social_exchange_json,
    resolve_cognitive_dissonance_json,
)


def _as_json_str(obj: Any) -> str:
    return obj if isinstance(obj, str) else json.dumps(obj)


def evaluate_empathy_read(
    target_psyche: dict[str, Any] | str,
    net_successes: int,
) -> dict[str, Any]:
    """Reveal graded 9-axis Psyche traits from an `INS/ITU` Empathy & Read Person check."""
    return dict(json.loads(evaluate_empathy_read_json(_as_json_str(target_psyche), int(net_successes))))


def evaluate_social_exchange(
    target_psyche: dict[str, Any] | str,
    vectors: Sequence[str] | str,
    net_successes: int,
    composure_state: dict[str, Any] | str | None = None,
    target_dis: int = 12,
    target_foc: int = 12,
    ego_wound_dis_check_passed: bool | None = None,
) -> dict[str, Any]:
    """Resolve persuasive vs coercive social combat exchange via Rust."""
    comp_json = None if composure_state is None else _as_json_str(composure_state)
    vec_json = vectors if isinstance(vectors, str) else json.dumps(list(vectors))
    raw = evaluate_social_exchange_json(
        _as_json_str(target_psyche),
        vec_json,
        int(net_successes),
        comp_json,
        int(target_dis),
        int(target_foc),
        ego_wound_dis_check_passed,
    )
    return dict(json.loads(raw))


def resolve_cognitive_dissonance(
    composure_state: dict[str, Any] | str,
    dis: int = 12,
    resolution_path: str = "compartmentalization",
    violated_trait: str | None = None,
    dis_check_successes: int = 1,
) -> dict[str, Any]:
    """Resolve cognitive dissonance via Compartmentalization, Catharsis, or Belief Shift."""
    raw = resolve_cognitive_dissonance_json(
        _as_json_str(composure_state),
        int(dis),
        resolution_path,
        violated_trait,
        int(dis_check_successes),
    )
    return dict(json.loads(raw))


def evaluate_conviction_surge(
    psyche: dict[str, Any] | str,
    current_delta_int: int,
    already_used_this_scene: bool = False,
    principle: str | None = None,
) -> dict[str, Any]:
    """Check and grant a Conviction Surge (`+2` pairs, `0` Fatigue) against strong Psyche poles."""
    raw = evaluate_conviction_surge_json(
        _as_json_str(psyche),
        int(current_delta_int),
        bool(already_used_this_scene),
        principle,
    )
    return dict(json.loads(raw))


def apply_psyche_drift_mark(
    trait_score: int,
    current_marks: int = 0,
    marks_added: int = 1,
    dis: int = 12,
    target_pole_score: int | None = None,
) -> dict[str, Any]:
    """Apply `3`-mark permanent Psyche polarity drift towards a competing pole."""
    raw = apply_psyche_drift_mark_json(
        int(trait_score),
        int(current_marks),
        int(marks_added),
        int(dis),
        target_pole_score,
    )
    return dict(json.loads(raw))


def evaluate_combat_motivation(
    target_psyche: dict[str, Any] | str,
    motivation_kind: str,
    command_intact: bool = True,
) -> dict[str, Any]:
    """Compute battlefield morale modifier and surrender resistance for an NPC motivation."""
    raw = evaluate_combat_motivation_json(
        _as_json_str(target_psyche),
        motivation_kind,
        bool(command_intact),
    )
    return dict(json.loads(raw))


def evaluate_emotional_trigger(
    target_psyche: dict[str, Any] | str,
    dis: int,
    current_morale: str,
    current_delta_int: int,
    trigger_kind: str,
    wound_tier: int = 1,
    roll_pairs: Sequence[Sequence[int]] | str | None = None,
) -> dict[str, Any]:
    """Evaluate an acute Psyche emotional trigger against composure/morale."""
    pairs_json = None if roll_pairs is None else _as_json_str(roll_pairs)
    raw = evaluate_emotional_trigger_json(
        _as_json_str(target_psyche),
        int(dis),
        current_morale,
        int(current_delta_int),
        trigger_kind,
        int(wound_tier),
        pairs_json,
    )
    return dict(json.loads(raw))


def evaluate_combat_morale_roll(
    target_psyche: dict[str, Any] | str,
    dis: int,
    motivation_kind: str,
    command_intact: bool,
    delta_int: int,
    situation: dict[str, Any] | str,
    is_veteran_or_sere: bool,
    current_morale: str,
    player_pairs: Sequence[Sequence[int]] | str,
    gm_pair: Sequence[int] | str,
) -> dict[str, Any]:
    """Resolve a contested battlefield morale check via Rust."""
    raw = evaluate_combat_morale_roll_json(
        _as_json_str(target_psyche),
        int(dis),
        motivation_kind,
        bool(command_intact),
        int(delta_int),
        _as_json_str(situation),
        bool(is_veteran_or_sere),
        current_morale,
        _as_json_str(player_pairs),
        _as_json_str(gm_pair),
    )
    return dict(json.loads(raw))


__all__ = [
    "apply_psyche_drift_mark",
    "apply_psyche_drift_mark_json",
    "compute_social_dissonance_check",
    "evaluate_combat_morale_roll",
    "evaluate_combat_morale_roll_json",
    "evaluate_combat_motivation",
    "evaluate_combat_motivation_json",
    "evaluate_conviction_surge",
    "evaluate_conviction_surge_json",
    "evaluate_emotional_trigger",
    "evaluate_emotional_trigger_json",
    "evaluate_empathy_read",
    "evaluate_empathy_read_json",
    "evaluate_psyche_friction_check",
    "evaluate_social_exchange",
    "evaluate_social_exchange_json",
    "resolve_cognitive_dissonance",
    "resolve_cognitive_dissonance_json",
]
