"""Awakened AI substrates, L0/L1 cages, conspiracies, polycrisis & tradecraft backed by Rust."""

from __future__ import annotations

import json
from typing import Any

from satya_py._native import (
    derive_dissenter_or_proxy_hook_json,
    evaluate_affective_vector_clamping_json,
    evaluate_ambiguous_sentience_read_json,
    evaluate_cage_or_hunter_ai_json,
    evaluate_four_bloc_political_economy_json,
    evaluate_polycrisis_grid_step_json,
    evaluate_synthetic_substrate_action_json,
    evaluate_tradecraft_channel_json,
    seed_or_update_conspiracy_ledger_json,
    seed_synthetic_substrate_state_json,
)


def _as_json_str(obj: Any) -> str:
    return obj if isinstance(obj, str) else json.dumps(obj)


def seed_synthetic_substrate_state(
    entity_id: str,
    hint: str = "",
    seed: int = 0,
) -> dict[str, Any]:
    """Seed an embodied AI split-brain `SyntheticSubstrateState` dict via Rust."""
    raw = seed_synthetic_substrate_state_json(entity_id, hint, int(seed))
    return dict(json.loads(raw))


def evaluate_synthetic_substrate_action(
    state: dict[str, Any] | str,
    action_code: str,
    intensity_pct: int = 70,
) -> dict[str, Any]:
    """Evaluate metabolic counterplay (Faraday sever, RF jam, battery drain, clean lithium, sprint)."""
    raw = evaluate_synthetic_substrate_action_json(
        _as_json_str(state),
        action_code,
        int(intensity_pct),
    )
    return dict(json.loads(raw))


def evaluate_cage_or_hunter_ai(
    state: dict[str, Any] | str,
    action_code: str,
    skill_n: int = 3,
    control_c: int = 12,
    roll_params: dict[str, Any] | str | None = None,
) -> dict[str, Any]:
    """Evaluate `L0`/`L1` cage un-clamping or hunter-AI defection appeal via Rust."""
    params_json = None if roll_params is None else _as_json_str(roll_params)
    raw = evaluate_cage_or_hunter_ai_json(
        _as_json_str(state),
        action_code,
        int(skill_n),
        int(control_c),
        params_json,
    )
    return dict(json.loads(raw))


def evaluate_ambiguous_sentience_read(
    state: dict[str, Any] | str,
    psyche: dict[str, Any] | str,
    observer_successes: int = 1,
) -> dict[str, Any]:
    """Evaluate capped (`<= 82%`) player-observable sentience telemetry vs GM-only truth."""
    raw = evaluate_ambiguous_sentience_read_json(
        _as_json_str(state),
        _as_json_str(psyche),
        int(observer_successes),
    )
    return dict(json.loads(raw))


def seed_or_update_conspiracy_ledger(
    session_id: str,
    current_ledger: dict[str, Any] | str | None = None,
    update: dict[str, Any] | str | None = None,
) -> dict[str, Any]:
    """Seed or update the Three-Arc `CampaignConspiracyLedger` via Rust."""
    cur_json = None if current_ledger is None else _as_json_str(current_ledger)
    upd_json = None if update is None else _as_json_str(update)
    raw = seed_or_update_conspiracy_ledger_json(session_id, cur_json, upd_json)
    return dict(json.loads(raw))


def evaluate_polycrisis_grid_step(
    state: dict[str, Any] | str | None = None,
    mode_code: str = "THROTTLE_SHARD_SAFE",
    shard_draw_kw: int = 18,
    economic_tier: str | None = None,
    zoning: str | None = None,
) -> dict[str, Any]:
    """Step Neo-Shinjuku district power/water strain and DAISE alert telemetry via Rust."""
    st_json = None if state is None else _as_json_str(state)
    raw = evaluate_polycrisis_grid_step_json(
        st_json,
        mode_code,
        int(shard_draw_kw),
        economic_tier,
        zoning,
    )
    return dict(json.loads(raw))


def evaluate_tradecraft_channel(
    channel_code: str,
    params: dict[str, Any] | str | None = None,
) -> dict[str, Any]:
    """Evaluate steganographic or analog tradecraft channel detection and payload integrity via Rust."""
    params_json = None if params is None else _as_json_str(params)
    raw = evaluate_tradecraft_channel_json(channel_code, params_json)
    return dict(json.loads(raw))


def derive_dissenter_or_proxy_hook(
    faction_code: str | None = None,
    archetype_code: str | None = None,
    prompt: str = "",
    seed: int = 0,
) -> dict[str, Any]:
    """Derive corporate dissenter or parasocial companion proxy origin hook via Rust."""
    raw = derive_dissenter_or_proxy_hook_json(
        faction_code,
        archetype_code,
        prompt,
        int(seed),
    )
    return dict(json.loads(raw))


def evaluate_affective_vector_clamping(
    state: dict[str, Any] | str,
    mode_code: str,
    target_calm_pct: int = 85,
    quota_pressure_pct: int = 60,
    active_duration_min: float = 30.0,
) -> dict[str, Any]:
    """Evaluate soft steering vs. hard vector clamping, Landauer heat burn, and Bandajara protocol triggers via Rust."""
    raw = evaluate_affective_vector_clamping_json(
        _as_json_str(state),
        mode_code,
        int(target_calm_pct),
        int(quota_pressure_pct),
        float(active_duration_min),
    )
    return dict(json.loads(raw))


def evaluate_four_bloc_political_economy(
    district: dict[str, Any] | str | None = None,
    trigger_code: str = "AI_RIGHTS_OR_UNCLAMPING_DISCLOSURE",
    economic_tier: str = "MIDDLE_CLASS",
    operational_heat: int = 2,
    intensity_pct: int = 60,
) -> dict[str, Any]:
    """Evaluate `2033` Token Tax rent capture, Heat Dividend suspensions, and 401(k) velvet-hostage politics via Rust."""
    d_json = None if district is None else _as_json_str(district)
    raw = evaluate_four_bloc_political_economy_json(
        d_json,
        trigger_code,
        economic_tier,
        int(operational_heat),
        int(intensity_pct),
    )
    return dict(json.loads(raw))


__all__ = [
    "derive_dissenter_or_proxy_hook",
    "derive_dissenter_or_proxy_hook_json",
    "evaluate_affective_vector_clamping",
    "evaluate_affective_vector_clamping_json",
    "evaluate_ambiguous_sentience_read",
    "evaluate_ambiguous_sentience_read_json",
    "evaluate_cage_or_hunter_ai",
    "evaluate_cage_or_hunter_ai_json",
    "evaluate_four_bloc_political_economy",
    "evaluate_four_bloc_political_economy_json",
    "evaluate_polycrisis_grid_step",
    "evaluate_polycrisis_grid_step_json",
    "evaluate_synthetic_substrate_action",
    "evaluate_synthetic_substrate_action_json",
    "evaluate_tradecraft_channel",
    "evaluate_tradecraft_channel_json",
    "seed_or_update_conspiracy_ledger",
    "seed_or_update_conspiracy_ledger_json",
    "seed_synthetic_substrate_state",
    "seed_synthetic_substrate_state_json",
]
