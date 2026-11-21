"""Campaign lifepaths, motivational hooks, NPC generation, campaign packs & world ledgers backed by Rust."""

from __future__ import annotations

import json
from collections.abc import Sequence
from typing import Any

from satya_py._native import (
    audit_canonical_lifepath_character,
    compile_party_motivational_hooks as _native_compile_party_motivational_hooks,
    compute_lifepath_skill_progression,
    evaluate_educational_advancement_roll,
    extract_character_lifepath_anchors as _native_extract_character_lifepath_anchors,
    generate_npc_profile,
    get_embedded_campaign_config,
    seed_or_update_world_ledger,
)

_DEFAULT_CHARACTERISTICS: dict[str, Any] = {
    "spd": 10,
    "for_stat": 10,
    "dex": 10,
    "kin": 10,
    "hlt": 10,
    "fit": 10,
    "hgt_m": 1.75,
    "mas_kg": 70.0,
    "mem": 10,
    "ana": 10,
    "ins": 10,
    "cre": 10,
    "dis": 10,
    "foc": 10,
    "com": 10,
    "pre": 10,
    "elo": 10,
    "sig": 10,
    "hea": 10,
    "tac": 10,
    "olf": 10,
    "tas": 10,
    "itu": 10,
    "ego": 10,
    "slf": 10,
    "opn": 10,
    "blg": 10,
    "ext": 10,
    "cnf": 10,
    "agg": 10,
    "trs": 10,
    "cps": 10,
}

_DEFAULT_APTITUDES: dict[str, int] = {
    "apt_qnt": 0,
    "apt_lng": 0,
    "apt_bio": 0,
    "apt_mec": 0,
    "apt_cmp": 0,
    "apt_kin": 0,
    "apt_fin": 0,
    "apt_art": 0,
    "apt_soc": 0,
}


def normalize_character_sheet_record(raw: dict[str, Any], index: int = 0) -> dict[str, Any]:
    """Normalize a partial or full Python character dict to satisfy `satya_schema::CharacterSheetRecord`."""
    raw_chars = dict(raw.get("characteristics") or {})
    merged_chars = dict(_DEFAULT_CHARACTERISTICS)
    for k, v in raw_chars.items():
        lk = str(k).lower()
        key = "for_stat" if lk in {"for", "for_stat", "str"} else lk
        if key in merged_chars and isinstance(v, (int, float)):
            merged_chars[key] = float(v) if key in {"hgt_m", "mas_kg"} else int(v)

    raw_apts = dict(raw.get("aptitudes") or {})
    merged_apts = dict(_DEFAULT_APTITUDES)
    for k, v in raw_apts.items():
        if k in merged_apts and isinstance(v, (int, float)):
            merged_apts[k] = int(v)

    skills = {
        str(k): int(v)
        for k, v in dict(raw.get("skills") or {}).items()
        if isinstance(v, (int, float))
    }
    cid = str(raw.get("character_id") or f"pc-{index}")
    cname = str(raw.get("character_name") or cid)
    lifepath_json = raw.get("lifepath_config_json")
    if not lifepath_json:
        if isinstance(raw.get("lifepath_config"), dict):
            lifepath_json = json.dumps(raw["lifepath_config"])
        else:
            lp_keys = (
                "youth_hobby",
                "personal_hobbies",
                "formative_events",
                "career_terms",
                "adolescent_life_event_roll_2d20",
                "adolescent_life_event_choice",
                "faction_archetype",
                "faction_id",
            )
            extracted_lp = {k: raw[k] for k in lp_keys if k in raw and raw[k] is not None}
            if extracted_lp:
                lifepath_json = json.dumps(extracted_lp)

    return {
        "character_id": cid,
        "owner_user_id": str(raw.get("owner_user_id") or cid),
        "version": int(raw.get("version", 1)),
        "character_name": cname,
        "callsign_or_role": str(raw.get("callsign_or_role") or "Field Specialist"),
        "age_years": int(raw.get("age_years", 28)),
        "origin_label": str(raw.get("origin_label") or "Working-Class Standard (SQM 4)"),
        "campaign_sqm": int(raw.get("campaign_sqm", 5)),
        "operational_heat": int(raw.get("operational_heat", 0)),
        "granted_access_tags": list(raw.get("granted_access_tags") or ["UNIVERSAL"]),
        "stage_history": list(raw.get("stage_history") or []),
        "characteristics": merged_chars,
        "aptitudes": merged_apts,
        "skills": skills,
        "equipped_ranged_ids": list(raw.get("equipped_ranged_ids") or []),
        "equipped_melee_codes": list(raw.get("equipped_melee_codes") or []),
        "equipped_armor_codes": list(raw.get("equipped_armor_codes") or []),
        "field_gear_ids": list(raw.get("field_gear_ids") or []),
        "hit_zones": list(raw.get("hit_zones") or []),
        "fatigue_level": int(raw.get("fatigue_level", 0)),
        "exertion_marks": int(raw.get("exertion_marks", 0)),
        "wound_shock_index": int(raw.get("wound_shock_index", 0)),
        "blood_units_lost": float(raw.get("blood_units_lost", 0.0)),
        "active_status_conditions": list(raw.get("active_status_conditions") or []),
        "morale_state": str(raw.get("morale_state") or "STEADY"),
        "mental_trauma": raw.get("mental_trauma"),
        "social_composure_lost": int(raw.get("social_composure_lost", 0)),
        "social_wound": raw.get("social_wound"),
        "psyche_drift_marks": int(raw.get("psyche_drift_marks", 0)),
        "notes": raw.get("notes"),
        "lifepath_config_json": lifepath_json,
        "faction_id": raw.get("faction_id") or None,
        "faction_archetype": raw.get("faction_archetype") or None,
    }


def compile_party_motivational_hooks(
    characters: Sequence[dict[str, Any]] | str,
    scene_context: dict[str, Any] | str | None = None,
    pack_id: str | None = None,
) -> dict[str, Any]:
    """Normalize party `CharacterSheetRecord` dicts and compile a `PartyHookManifest` via Rust."""
    payload: Any = (
        characters
        if isinstance(characters, str)
        else [normalize_character_sheet_record(dict(c), i) for i, c in enumerate(characters) if isinstance(c, dict)]
    )
    return _native_compile_party_motivational_hooks(payload, scene_context, pack_id)


def extract_character_lifepath_anchors(
    character: dict[str, Any] | str,
    index: int = 0,
) -> dict[str, Any]:
    """Extract hobby contacts, formative anchors & faction wedge hints from a character sheet."""
    payload: Any = (
        character
        if isinstance(character, str)
        else normalize_character_sheet_record(dict(character), index)
    )
    return _native_extract_character_lifepath_anchors(payload)


__all__ = [
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
