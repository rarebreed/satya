"""Continuous metric tactical map generation, LoS/cover, and spatial perception backed by Rust."""

from __future__ import annotations

from satya_py._native import (
    check_tactical_token_collision,
    compute_reachable_zone,
    continue_procedural_map,
    edit_map_feature,
    evaluate_ballistic_shot,
    evaluate_passive_perceptions,
    evaluate_spatial_blast_and_breach,
    evaluate_tactical_los_and_cover,
    extract_narrative_manifest,
    generate_procedural_sector,
    inspect_tactical_map_point,
    list_non_solid_prop_kinds,
    normalize_tactical_map,
    resolve_tactical_token_destination_cm,
    switch_map_floor,
)

check_token_collision = check_tactical_token_collision
resolve_token_destination_cm = resolve_tactical_token_destination_cm
evaluate_los_and_cover = evaluate_tactical_los_and_cover
inspect_map_point = inspect_tactical_map_point

__all__ = [
    "check_tactical_token_collision",
    "check_token_collision",
    "compute_reachable_zone",
    "continue_procedural_map",
    "edit_map_feature",
    "evaluate_ballistic_shot",
    "evaluate_los_and_cover",
    "evaluate_passive_perceptions",
    "evaluate_spatial_blast_and_breach",
    "evaluate_tactical_los_and_cover",
    "extract_narrative_manifest",
    "generate_procedural_sector",
    "inspect_map_point",
    "inspect_tactical_map_point",
    "list_non_solid_prop_kinds",
    "normalize_tactical_map",
    "resolve_tactical_token_destination_cm",
    "resolve_token_destination_cm",
    "switch_map_floor",
]
