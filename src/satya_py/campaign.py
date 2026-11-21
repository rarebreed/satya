"""Campaign lifepaths, motivational hooks, NPC generation, campaign packs & world ledgers backed by Rust."""

from __future__ import annotations

from satya_py._native import (
    audit_canonical_lifepath_character,
    compile_party_motivational_hooks,
    compute_lifepath_skill_progression,
    evaluate_educational_advancement_roll,
    extract_character_lifepath_anchors,
    generate_npc_profile,
    get_embedded_campaign_config,
    normalize_character_sheet_record,
    seed_or_update_world_ledger,
)
from satya_py.schema import CharacterSheetRecordDict

__all__ = [
    "CharacterSheetRecordDict",
    "audit_canonical_lifepath_character",
    "compile_party_motivational_hooks",
    "compute_lifepath_skill_progression",
    "evaluate_educational_advancement_roll",
    "extract_character_lifepath_anchors",
    "generate_npc_profile",
    "get_embedded_campaign_config",
    "normalize_character_sheet_record",
    "seed_or_update_world_ledger",
]
