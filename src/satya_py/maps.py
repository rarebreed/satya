"""Continuous metric tactical map generation, LoS/cover, and spatial perception backed by Rust."""

from __future__ import annotations

import json
from collections.abc import Sequence
from typing import Any

from satya_py._native import (
    check_tactical_token_collision,
    compute_reachable_zone_json,
    continue_procedural_tactical_map_json,
    edit_tactical_map_feature_json,
    evaluate_ballistic_shot_on_map_json,
    evaluate_passive_movement_perceptions_json,
    evaluate_spatial_blast_and_breach_json,
    evaluate_tactical_los_and_cover,
    extract_sector_narrative_manifest_json,
    generate_procedural_tactical_sector_json,
    inspect_tactical_map_point,
    resolve_tactical_token_destination_cm,
    switch_tactical_map_floor_json,
)


def _as_json_str(map_state: dict[str, Any] | str) -> str:
    return map_state if isinstance(map_state, str) else json.dumps(map_state)


def generate_procedural_sector(
    map_id: str,
    preset_or_theme: str,
    title: str | None = None,
    width: int | None = None,
    height: int | None = None,
    seed: int = 42,
    economic_tier: str | None = None,
    zoning: str | None = None,
    scale: str | None = None,
) -> dict[str, Any]:
    """Generate a procedural multi-floor tactical sector and parse its JSON into a dictionary."""
    raw = generate_procedural_tactical_sector_json(
        map_id,
        preset_or_theme,
        title,
        width,
        height,
        seed,
        economic_tier,
        zoning,
        scale,
    )
    return dict(json.loads(raw))


def continue_procedural_map(
    map_state: dict[str, Any] | str,
    direction: str,
    seed: int | None = None,
    preset_override: str | None = None,
) -> dict[str, Any]:
    """Continue an existing tactical map into an adjacent cardinal sector."""
    raw = continue_procedural_tactical_map_json(
        _as_json_str(map_state),
        direction,
        seed,
        preset_override,
    )
    return dict(json.loads(raw))


def switch_map_floor(
    map_state: dict[str, Any] | str,
    target_floor_index: int,
) -> dict[str, Any]:
    """Switch the active vertical floor slice of a multi-story tactical map."""
    raw = switch_tactical_map_floor_json(_as_json_str(map_state), int(target_floor_index))
    return dict(json.loads(raw))


def extract_narrative_manifest(map_state: dict[str, Any] | str) -> dict[str, Any]:
    """Extract zones, cardinal exits, and semantic landmarks for a tactical map."""
    raw = extract_sector_narrative_manifest_json(_as_json_str(map_state))
    return dict(json.loads(raw))


def check_token_collision(
    map_state: dict[str, Any] | str,
    x_m: float,
    y_m: float,
    radius_m: float = 0.30,
    floor_index: int | None = None,
) -> bool:
    """Return ``True`` when a token disc at `(x_m, y_m)` hits bounds, solid walls, or solid OBB props."""
    return bool(
        check_tactical_token_collision(
            _as_json_str(map_state),
            float(x_m),
            float(y_m),
            float(radius_m),
            floor_index=floor_index,
        )
    )


def resolve_token_destination_cm(
    map_state: dict[str, Any] | str,
    moving_entity_id: str,
    target_x_cm: int,
    target_y_cm: int,
    floor_index: int | None = None,
) -> tuple[int, int]:
    """Resolve and validate continuous destination coordinates in centimeters for `moving_entity_id`."""
    return resolve_tactical_token_destination_cm(
        _as_json_str(map_state),
        moving_entity_id,
        int(target_x_cm),
        int(target_y_cm),
        floor_index=floor_index,
    )


def evaluate_los_and_cover(
    map_state: dict[str, Any] | str,
    x1_m: float,
    y1_m: float,
    x2_m: float,
    y2_m: float,
    floor_index: int | None = None,
) -> dict[str, Any]:
    """Evaluate continuous LoS obstruction and cover grade between `(x1_m, y1_m)` and `(x2_m, y2_m)`."""
    return evaluate_tactical_los_and_cover(
        _as_json_str(map_state),
        float(x1_m),
        float(y1_m),
        float(x2_m),
        float(y2_m),
        floor_index=floor_index,
    )


def inspect_map_point(
    map_state: dict[str, Any] | str,
    theme_name: str,
    x_m: float,
    y_m: float,
    floor_index: int | None = None,
) -> dict[str, Any] | None:
    """Inspect ground surface, walls, props, or tokens at `(x_m, y_m)`."""
    return inspect_tactical_map_point(
        _as_json_str(map_state),
        theme_name,
        float(x_m),
        float(y_m),
        floor_index=floor_index,
    )


def evaluate_passive_perceptions(
    map_state: dict[str, Any] | str,
    moved_entity_id: str,
    observers: Sequence[dict[str, Any]] | str,
    pins: Sequence[dict[str, Any]] | str | None = None,
) -> list[dict[str, Any]]:
    """Evaluate passive sensory pin triggers during token movement."""
    obs_json = observers if isinstance(observers, str) else json.dumps(list(observers))
    pins_json = None if pins is None else (pins if isinstance(pins, str) else json.dumps(list(pins)))
    raw = evaluate_passive_movement_perceptions_json(
        _as_json_str(map_state),
        moved_entity_id,
        obs_json,
        pins_json,
    )
    parsed = json.loads(raw)
    return list(parsed) if isinstance(parsed, list) else []


def evaluate_ballistic_shot(
    map_state: dict[str, Any] | str,
    x1_m: float,
    y1_m: float,
    x2_m: float,
    y2_m: float,
    firearm_preset_id: str = "hk416_carbine_556",
    net_successes: int = 0,
    hit_frame_or_stud: bool = False,
    floor_index: int | None = None,
) -> dict[str, Any]:
    """Evaluate continuous range, cover, multi-barrier perforation, and residual ballistic impact."""
    raw = evaluate_ballistic_shot_on_map_json(
        _as_json_str(map_state),
        float(x1_m),
        float(y1_m),
        float(x2_m),
        float(y2_m),
        firearm_preset_id,
        int(net_successes),
        bool(hit_frame_or_stud),
        floor_index,
    )
    return dict(json.loads(raw))


def compute_reachable_zone(
    map_state: dict[str, Any] | str,
    x_m: float,
    y_m: float,
    spd_eff: int = 10,
    hgt_m: float = 1.75,
    gait_code: str = "COMBAT_GLIDE",
    tick_budget: int = 4,
    floor_index: int | None = None,
    exclude_entity_id: str | None = None,
) -> dict[str, Any]:
    """Compute continuous `0.25m` 16-directional gait reachability strips for a tactical floor."""
    raw = compute_reachable_zone_json(
        _as_json_str(map_state),
        float(x_m),
        float(y_m),
        int(spd_eff),
        float(hgt_m),
        gait_code,
        int(tick_budget),
        floor_index,
        exclude_entity_id,
    )
    return dict(json.loads(raw))


def evaluate_spatial_blast_and_breach(
    map_state: dict[str, Any] | str,
    x_m: float,
    y_m: float,
    ordnance_code: str = "M67_FRAG",
    demolitions_net_successes: int = 0,
    apply_wall_breaches: bool = True,
    floor_index: int | None = None,
) -> dict[str, Any]:
    """Evaluate spatial blast exposure, indoor overpressure, and structural wall breaches."""
    raw = evaluate_spatial_blast_and_breach_json(
        _as_json_str(map_state),
        float(x_m),
        float(y_m),
        ordnance_code,
        int(demolitions_net_successes),
        bool(apply_wall_breaches),
        floor_index,
    )
    return dict(json.loads(raw))


def edit_map_feature(
    map_state: dict[str, Any] | str,
    action: dict[str, Any] | str,
    floor_index: int | None = None,
) -> dict[str, Any]:
    """Apply an authoritative vector map edit (`TOGGLE_DOOR`, `SET_EDGE`, `REMOVE_EDGE`, `PLACE_PROP`, `REMOVE_PROP`, `SET_BLACKOUT`, `BLAST_OR_BREACH`)."""
    act_json = action if isinstance(action, str) else json.dumps(action)
    raw = edit_tactical_map_feature_json(
        _as_json_str(map_state),
        act_json,
        floor_index,
    )
    return dict(json.loads(raw))


__all__ = [
    "check_tactical_token_collision",
    "check_token_collision",
    "compute_reachable_zone",
    "compute_reachable_zone_json",
    "continue_procedural_map",
    "continue_procedural_tactical_map_json",
    "edit_map_feature",
    "edit_tactical_map_feature_json",
    "evaluate_ballistic_shot",
    "evaluate_ballistic_shot_on_map_json",
    "evaluate_los_and_cover",
    "evaluate_passive_movement_perceptions_json",
    "evaluate_passive_perceptions",
    "evaluate_spatial_blast_and_breach",
    "evaluate_spatial_blast_and_breach_json",
    "evaluate_tactical_los_and_cover",
    "extract_narrative_manifest",
    "extract_sector_narrative_manifest_json",
    "generate_procedural_sector",
    "generate_procedural_tactical_sector_json",
    "inspect_map_point",
    "inspect_tactical_map_point",
    "resolve_tactical_token_destination_cm",
    "resolve_token_destination_cm",
    "switch_map_floor",
    "switch_tactical_map_floor_json",
]
