"""Hook Weaver & Party Motivational Manifest compiler for the Satya AI GM (`satya_ml.ai_gm.hooks`).

Wraps the deterministic Rust compiler (`satya_py.compile_party_motivational_hooks_json`)
to surface PC Psyche polarity tensions, Skill Spotlights, Operational Heat, Credential Access,
and Party Entanglement Wedges into `sync_session_context` and `ScenarioDirector`.
"""

from __future__ import annotations

import json
from typing import Any

from satya_ml.campaign_config import get_active_campaign_pack_name


def _load_satya_py() -> Any | None:
    try:
        from satya_ml.dice_probabilities import _ensure_satya_py_loaded

        return _ensure_satya_py_loaded()
    except Exception:
        return None


def _python_fallback_hooks(
    characters: list[dict[str, Any]],
    pack_id: str,
    active_location: str | None,
) -> dict[str, Any]:
    """Compact deterministic fallback if compiled `satya_py` bindings are unavailable."""
    hooks: list[dict[str, Any]] = []
    for idx, ch in enumerate(characters):
        if not isinstance(ch, dict):
            continue
        cid = str(ch.get("character_id") or f"pc-{idx}")
        cname = str(ch.get("character_name") or cid)
        chars = ch.get("characteristics") or {}
        if isinstance(chars, dict):
            cps = int(chars.get("cps", chars.get("CPS", 10)) or 10)
            trs = int(chars.get("trs", chars.get("TRS", 10)) or 10)
            if cps >= 15 or cps <= 6:
                hooks.append(
                    {
                        "hook_id": f"{cid}-psyche-cps",
                        "category": "IDENTITY_PSYCHE",
                        "character_id": cid,
                        "character_name": cname,
                        "title": f"{cname}: Compassion Edge (CPS={cps})",
                        "setup": (
                            f"Place vulnerable civilians or an un-clamped AI in harm's way, testing {cname}'s Compassion (CPS={cps})."
                        ),
                        "emotional_stake": f"Contrasts {cname}'s humanitarian empathy against cold operational expediency.",
                        "preferred_vectors": ["MoralSanctityWitnessedSufferingMercy"],
                        "pressure_lever": f"CPS={cps}",
                        "salience": 0.72,
                    }
                )
            elif trs >= 15 or trs <= 6:
                hooks.append(
                    {
                        "hook_id": f"{cid}-psyche-trs",
                        "category": "IDENTITY_PSYCHE",
                        "character_id": cid,
                        "character_name": cname,
                        "title": f"{cname}: Trust Fault Line (TRS={trs})",
                        "setup": (
                            f"Introduce a whistleblower or rival offer demanding a leap of faith against {cname}'s Trust (TRS={trs})."
                        ),
                        "emotional_stake": f"Tests {cname}'s institutional credence or paranoid vigilance.",
                        "preferred_vectors": ["TransparencyVulnerabilityGoodFaith"],
                        "pressure_lever": f"TRS={trs}",
                        "salience": 0.68,
                    }
                )
        skills = ch.get("skills") or {}
        if isinstance(skills, dict) and skills:
            top_skill, top_rank = max(
                ((str(k), int(v)) for k, v in skills.items() if isinstance(v, (int, float))),
                key=lambda item: item[1],
                default=("", 0),
            )
            if top_skill and top_rank >= 2:
                hooks.append(
                    {
                        "hook_id": f"{cid}-skill-{top_skill}",
                        "category": "SKILL_SPOTLIGHT",
                        "character_id": cid,
                        "character_name": cname,
                        "title": f"{cname}: Spotlight {top_skill} (N={top_rank})",
                        "setup": f"Present an immediate complication solvable through {top_skill} (N={top_rank}).",
                        "emotional_stake": f"Tests {cname}'s defining competence under pressure.",
                        "preferred_vectors": ["EmpiricalEvidenceLogicNovelIdeas"],
                        "pressure_lever": top_skill,
                        "salience": round(0.55 + min(0.35, top_rank * 0.07), 3),
                    }
                )
    return {
        "pack_id": pack_id,
        "active_location": active_location,
        "character_profiles": {},
        "hooks": hooks,
        "party_entanglements": [],
    }


_DEFAULT_CHARACTERISTICS: dict[str, Any] = {
    "spd": 10, "for_stat": 10, "dex": 10, "kin": 10, "hlt": 10, "fit": 10,
    "hgt_m": 1.75, "mas_kg": 70.0,
    "mem": 10, "ana": 10, "ins": 10, "cre": 10, "dis": 10, "foc": 10,
    "com": 10, "pre": 10, "elo": 10,
    "sig": 10, "hea": 10, "tac": 10, "olf": 10, "tas": 10, "itu": 10,
    "ego": 10, "slf": 10, "opn": 10, "blg": 10, "ext": 10, "cnf": 10, "agg": 10,
    "trs": 10, "cps": 10,
}

_DEFAULT_APTITUDES: dict[str, int] = {
    "apt_qnt": 0, "apt_lng": 0, "apt_bio": 0, "apt_mec": 0, "apt_cmp": 0,
    "apt_kin": 0, "apt_fin": 0, "apt_art": 0, "apt_soc": 0,
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


def compile_player_motivational_hooks(
    characters: list[dict[str, Any]],
    pack_id: str | None = None,
    active_location: str | None = None,
) -> dict[str, Any]:
    """Compile bound PC CharacterSheetRecords into a `PartyHookManifest` dictionary."""
    resolved_pack = (pack_id or get_active_campaign_pack_name()).strip().lower() or "awakened"
    clean_chars = [
        normalize_character_sheet_record(dict(c), i)
        for i, c in enumerate(characters or [])
        if isinstance(c, dict)
    ]
    if not clean_chars:
        return {
            "pack_id": resolved_pack,
            "active_location": active_location,
            "character_profiles": {},
            "hooks": [],
            "party_entanglements": [],
        }

    mod = _load_satya_py()
    if mod is not None and hasattr(mod, "compile_party_motivational_hooks_json"):
        try:
            scene_ctx = (
                {"location": str(active_location), "active_location": str(active_location)}
                if active_location
                else {}
            )
            raw_json = mod.compile_party_motivational_hooks_json(
                json.dumps(clean_chars),
                json.dumps(scene_ctx) if scene_ctx else None,
                resolved_pack,
            )
            parsed = json.loads(raw_json)
            if isinstance(parsed, dict):
                return parsed
        except Exception:
            pass
    return _python_fallback_hooks(clean_chars, resolved_pack, active_location)


def format_party_hooks_for_prompt(manifest: dict[str, Any], max_hooks: int = 4) -> str:
    """Format a compact (`<= 900` chars) digest of compiled PC hooks & party wedges."""
    hooks = sorted(
        list(manifest.get("hooks") or []),
        key=lambda h: float(h.get("salience", 0.0)),
        reverse=True,
    )
    entanglements = list(manifest.get("party_entanglements") or [])
    if not hooks and not entanglements:
        return ""

    parts: list[str] = ["[Party Motivational Hooks & Entanglements]"]
    for h in hooks[: max(1, int(max_hooks))]:
        cat = str(h.get("category", "HOOK"))
        title = str(h.get("title", "Hook"))
        setup = str(h.get("setup", "")).strip()
        parts.append(f"- [{cat}] {title}: {setup}")
    for ent in entanglements[:2]:
        parts.append(
            f"- [PARTY_WEDGE] {ent.get('title', 'Friction')} ({ent.get('tension_axis', '')}): "
            f"{ent.get('shared_scene_prompt', '')}"
        )
    digest = "\n".join(parts)
    if len(digest) > 900:
        return digest[:897].rstrip() + "..."
    return digest
