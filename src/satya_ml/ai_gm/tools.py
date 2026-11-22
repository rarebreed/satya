"""MCP & LangChain @tool suite for the Satya AI Game Master (AI GM).

Provides Tier 1 (Satya Core Session & Canvas), Tier 2 (Satya d20 Ruleset & NPC Engine),
and Tier 3 (Campaign Lore & PyLance Retrieval) tools, plus optional external MCP server
loading via `langchain-mcp-adapters`.
"""

from __future__ import annotations

import json
import logging
import math
import os
import random
import re
import urllib.parse
import uuid
from collections.abc import Mapping
from pathlib import Path
from typing import Any, Final, cast

logger = logging.getLogger("satya_ml.ai_gm.tools")

from satya_py.combat import CharacterTacticalEnvelope
from satya_py.enums import (
    ChannelContext,
    CombatStance,
    GmMemoryCategory,
    MessageVisibility,
    normalize_combat_stance,
)

from satya_ml.ai_gm.models import SessionLlmConfig  # noqa: F401 (installs Windows SSL fallback)

import httpx

from satya_ml.ai_gm.telemetry import current_trace_headers, record_emitted_event_id
from satya_ml.ai_gm.tool_spec import (
    SessionStore,
    get_builtin_tool_objects,
    run_session_reset_hooks,
    satya_gm_tool,
    tool,
)
from satya_ml.signaling_client import (
    OFFLINE_SIGNALING_VALUES as _OFFLINE_SIGNALING_VALUES,
    gm_service_token,
    http_delete_json,
    http_delete_json_detailed,
    http_get_json,
    http_post_json,
    httpx_timeout as _httpx_timeout,
    load_ai_gm_toml_config,
    repo_root as _repo_root,
    service_auth_headers,
    signaling_base_url,
)


def _signaling_base_url() -> str:
    return signaling_base_url()


def _gm_service_token() -> str:
    return gm_service_token()


def _gm_headers() -> dict[str, str]:
    headers = service_auth_headers()
    headers["Authorization"] = f"Bearer {_gm_service_token()}"
    return headers


def _http_post_json(url: str, payload: dict[str, Any], timeout: float = 2.5) -> dict[str, Any] | None:
    """POST JSON to `url` with GM service headers, returning parsed JSON on 200 OK or None on failure."""
    return http_post_json(url, payload, timeout=timeout)


def _http_get_json(url: str, timeout: float = 2.5) -> dict[str, Any] | None:
    """GET JSON from `url` with GM service headers, returning parsed JSON on 200 OK or None on failure."""
    return http_get_json(url, timeout=timeout)


def _http_delete_json_detailed(
    url: str, timeout: float = 2.5
) -> tuple[dict[str, Any] | None, int | None, str | None]:
    """DELETE at `url` with GM service headers, returning `(parsed_json, status_code, error_body)`."""
    return http_delete_json_detailed(url, timeout=timeout)


def _http_delete_json(url: str, timeout: float = 2.5) -> dict[str, Any] | None:
    """DELETE at `url` with GM service headers, returning parsed JSON on 200 OK or None on failure."""
    return http_delete_json(url, timeout=timeout)


from satya_py import campaign as _campaign, combat as _combat, dice as _dice

from satya_ml.ai_gm.maps import (
    continue_tactical_map,
    ensure_or_sync_procedural_tactical_map_stub,
    generate_tactical_map,
    get_session_map,
    inspect_tactical_map,
    move_map_token,
    register_spatial_clue_pin,
    set_session_map,
    switch_tactical_map_floor,
)
from satya_ml.campaign_config import (
    get_active_campaign_pack_name,
    get_pack_lore_dirs,
    get_pack_starter_npcs,
    get_pack_zone_labels,
)

# In-memory per-session caches for NPC states, operating mode, human GM ID, GM_ONLY documents,
# persistent GM memories, bound character sheets, and combat tracker state
_SESSION_NPCS: Final[SessionStore[dict[str, dict[str, Any]]]] = SessionStore()
_SESSION_MODES: Final[SessionStore[str]] = SessionStore()
_SESSION_HUMAN_GMS: Final[SessionStore[str | None]] = SessionStore()
_SESSION_GM_DOCS: Final[SessionStore[list[dict[str, Any]]]] = SessionStore()
_SESSION_GM_MEMORIES: Final[SessionStore[list[dict[str, Any]]]] = SessionStore()
_SESSION_CHARACTERS: Final[SessionStore[list[dict[str, Any]]]] = SessionStore()
_SESSION_COMBAT_TRACKER: Final[SessionStore[dict[str, Any]]] = SessionStore()


def set_session_operating_mode(
    session_id: str,
    mode: str,
    human_gm_user_id: str | None = None,
) -> None:
    """Record the operating mode ('autonomous', 'co_pilot', or 'solo_co_gm') and optional human GM ID."""
    _SESSION_MODES[session_id] = mode
    if human_gm_user_id is None:
        _SESSION_HUMAN_GMS.pop(session_id, None)
    else:
        _SESSION_HUMAN_GMS[session_id] = human_gm_user_id


def get_session_operating_mode(session_id: str) -> str:
    """Return the operating mode ('autonomous', 'co_pilot', or 'solo_co_gm') for a session."""
    return _SESSION_MODES.get(session_id, "autonomous")


def get_session_human_gm(session_id: str) -> str | None:
    """Return the human GM user_id for a co-pilot session, if known."""
    return _SESSION_HUMAN_GMS.get(session_id)


def set_session_gm_docs(session_id: str, docs: list[dict[str, Any]]) -> None:
    """Cache the sanitized `GM_ONLY` documents synced from `GET /api/gm/sessions/:id/context`."""
    _SESSION_GM_DOCS[session_id] = list(docs)


def get_session_gm_docs(session_id: str) -> list[dict[str, Any]]:
    """Return cached `GM_ONLY` documents for `session_id`."""
    return list(_SESSION_GM_DOCS.get(session_id, []))


def set_session_gm_memories(session_id: str, memories: list[dict[str, Any]]) -> None:
    """Cache the persistent `GmMemoryNoteRecord` entries synced from `GET /api/gm/sessions/:id/context`."""
    _SESSION_GM_MEMORIES[session_id] = list(memories)


def get_session_gm_memories(session_id: str) -> list[dict[str, Any]]:
    """Return cached persistent `GmMemoryNoteRecord` entries for `session_id`."""
    return list(_SESSION_GM_MEMORIES.get(session_id, []))


def set_session_characters(session_id: str, characters: list[dict[str, Any]]) -> None:
    """Cache the bound `CharacterSheetRecord` entries synced from `GET /api/gm/sessions/:id/context`."""
    _SESSION_CHARACTERS[session_id] = list(characters)


def get_session_characters(session_id: str) -> list[dict[str, Any]]:
    """Return cached bound `CharacterSheetRecord` entries for `session_id`."""
    return list(_SESSION_CHARACTERS.get(session_id, []))


def set_session_combat_tracker(session_id: str, tracker: dict[str, Any]) -> None:
    """Cache the `CombatTrackerState` synced from `GET /api/gm/sessions/:id/context`."""
    _SESSION_COMBAT_TRACKER[session_id] = dict(tracker)


def get_session_combat_tracker(session_id: str) -> dict[str, Any]:
    """Return cached `CombatTrackerState` for `session_id`, initializing a default Window 1 / Tick 1 state if absent."""
    if session_id not in _SESSION_COMBAT_TRACKER:
        _SESSION_COMBAT_TRACKER[session_id] = {
            "session_id": session_id,
            "combat_active": False,
            "round": 1,
            "tick": 1,
            "combatants": [],
            "updated_at_us": 0,
        }
    return dict(_SESSION_COMBAT_TRACKER[session_id])


_SESSION_NPC_PACKS: Final[SessionStore[str]] = SessionStore()
_SESSION_PERSISTED_NPCS: Final[SessionStore[list[dict[str, Any]]]] = SessionStore()


def reset_session_npcs(session_id: str | None = None) -> None:
    """Clear cached NPC state, session caches, maps, and all registered GM session state for `session_id` (or all)."""
    run_session_reset_hooks(session_id)


reset_session_gm_state = reset_session_npcs


def get_persisted_session_npcs(session_id: str) -> list[dict[str, Any]] | None:
    """Return the most recent signaling-persisted NPC roster for `session_id`, or `None` if signaling has not been queried."""
    if session_id not in _SESSION_PERSISTED_NPCS:
        return None
    return [dict(item) for item in _SESSION_PERSISTED_NPCS[session_id]]


def _upsert_persisted_session_npc(session_id: str, norm_entry: dict[str, Any]) -> None:
    """Insert or update `norm_entry` in `_SESSION_PERSISTED_NPCS[session_id]`, deduplicating by ID and name."""
    nid = str(norm_entry.get("npc_id") or "").strip().lower()
    name_low = str(norm_entry.get("name") or "").strip().lower()
    existing = list(_SESSION_PERSISTED_NPCS.get(session_id, []))
    filtered = [
        item
        for item in existing
        if str(item.get("npc_id") or "").strip().lower() != nid
        and (not name_low or str(item.get("name") or "").strip().lower() != name_low)
    ]
    _SESSION_PERSISTED_NPCS[session_id] = [dict(norm_entry), *filtered]


def _ensure_session_npcs_initialized(session_id: str) -> dict[str, dict[str, Any]]:
    active_pack = get_active_campaign_pack_name()
    if session_id not in _SESSION_NPCS or _SESSION_NPC_PACKS.get(session_id) != active_pack:
        _SESSION_NPCS[session_id] = load_all_npc_profiles()
        _SESSION_NPC_PACKS[session_id] = active_pack
    return _SESSION_NPCS[session_id]


def _normalize_remote_npc_entry(entry: dict[str, Any]) -> dict[str, Any]:
    """Convert a signaling `SessionNpcEntry` wire object into the flat AI GM NPC profile shape."""
    cid = str(entry.get("character_id") or entry.get("npc_id") or "").strip().lower()
    gm_char = entry.get("gm_character") if isinstance(entry.get("gm_character"), dict) else {}
    return {
        "npc_id": cid,
        "name": str(entry.get("character_name") or entry.get("name") or cid),
        "archetype": str(entry.get("callsign_or_role") or entry.get("archetype") or "NPC"),
        "faction": str(entry.get("faction_id") or entry.get("faction") or "Independent"),
        "faction_id": entry.get("faction_id") or gm_char.get("faction_id"),
        "faction_archetype": entry.get("faction_archetype") or gm_char.get("faction_archetype"),
        "disposition": str(entry.get("disposition") or "wary"),
        "public_summary": str(entry.get("public_summary") or ""),
        "apparent_condition": str(entry.get("apparent_condition") or "Healthy & Alert"),
        "notes": str(entry.get("public_summary") or ""),
        "secrets": [str(entry["gm_secret_dossier"])] if entry.get("gm_secret_dossier") else [],
        "psyche_summary": list(entry.get("psyche_summary") or []),
        "top_skills": list(entry.get("top_skills") or []),
        "skills": dict(gm_char.get("skills") or {}),
        "attribute_tns": dict(gm_char.get("attribute_tns") or {}),
        "combat_stance": normalize_combat_stance(
            gm_char.get("combat_stance"),
            default=CombatStance.STABILITY,
        ).value,
        "morale_state": str(entry.get("morale_state") or gm_char.get("morale_state") or "steady").lower(),
        "fatigue": int(gm_char.get("fatigue_level", 0) or 0),
        "wound_shock": int(gm_char.get("wound_shock_index", 0) or 0),
        "immediate_agenda": entry.get("immediate_agenda"),
        "core_drive": entry.get("core_drive"),
        "dual_psyche_wedge_hint": entry.get("dual_psyche_wedge_hint"),
        "social_combat_wedge_hint": entry.get("social_combat_wedge_hint") or entry.get("dual_psyche_wedge_hint"),
    }


def load_all_npc_profiles(repo_root: Path | None = None) -> dict[str, dict[str, Any]]:
    """Load canonical starter NPCs for the active campaign pack (`config/campaign.toml` + `data/npcs/*.toml`)."""
    del repo_root  # campaign_config resolves repo root and active pack TOMLs
    return get_pack_starter_npcs()


def get_session_npcs(session_id: str) -> dict[str, dict[str, Any]]:
    """Return the mutable NPC state dictionary for `session_id`, non-destructively merging signaling state."""
    npcs = _ensure_session_npcs_initialized(session_id)
    base = _signaling_base_url()
    if base:
        encoded_sid = urllib.parse.quote(session_id, safe="")
        data = _http_get_json(f"{base}/api/gm/sessions/{encoded_sid}/npcs", timeout=1.5)
        if isinstance(data, dict):
            starter_ids = set(load_all_npc_profiles())
            prev_persisted_ids = {
                str(item.get("npc_id") or "").strip().lower()
                for item in _SESSION_PERSISTED_NPCS.get(session_id, [])
                if isinstance(item, dict) and item.get("npc_id")
            }
            remote_list: list[dict[str, Any]] = []
            remote_ids: set[str] = set()
            for raw_entry in data.get("npcs") or []:
                if isinstance(raw_entry, dict) and raw_entry.get("character_id"):
                    norm = _normalize_remote_npc_entry(raw_entry)
                    nid = norm["npc_id"]
                    remote_ids.add(nid)
                    remote_list.append(norm)
                    existing = npcs.get(nid)
                    if existing is not None:
                        has_gm_char = isinstance(raw_entry.get("gm_character"), dict) and bool(
                            raw_entry.get("gm_character")
                        )
                        if not has_gm_char:
                            for k in (
                                "skills",
                                "attribute_tns",
                                "combat_stance",
                                "fatigue",
                                "wound_shock",
                                "morale_state",
                                "spoofing_secops",
                            ):
                                if k in existing:
                                    norm[k] = existing[k]
                        if not raw_entry.get("disposition") and existing.get("disposition"):
                            norm["disposition"] = existing["disposition"]
                        if not raw_entry.get("gm_secret_dossier") and existing.get("secrets"):
                            norm["secrets"] = list(existing["secrets"])
                        npcs[nid] = {**existing, **norm}
                    else:
                        npcs[nid] = norm
            for stale_id in prev_persisted_ids - remote_ids:
                if stale_id not in starter_ids:
                    npcs.pop(stale_id, None)
            _SESSION_PERSISTED_NPCS[session_id] = remote_list
    return npcs


# ============================================================================
# 5.1 NPC Profile & State Management
# ============================================================================


def _compact_npc_summary(profile: dict[str, Any], fallback_id: str) -> dict[str, Any]:
    return {
        "npc_id": str(profile.get("npc_id") or fallback_id),
        "name": str(profile.get("name") or fallback_id),
        "archetype": str(profile.get("archetype") or "NPC"),
        "faction": str(profile.get("faction") or "Independent"),
        "disposition": str(profile.get("disposition") or "wary"),
        "apparent_condition": str(profile.get("apparent_condition") or "Healthy & Alert"),
    }


@satya_gm_tool(pack="core", tier="TIER_2_RULESET_ENGINE", ooc_readonly=True)
def lookup_npc_profile(npc_id: str = "all", session_id: str = "default") -> dict[str, Any]:
    """Read an NPC's Satya d20 profile, attribute TNs, skills, psyche, agenda, combat stance, and secrets.

    Pass a specific NPC name/ID, or pass `"all"` / `"list"` / `""` to list all active persisted Session NPCs
    alongside available campaign starter contacts.
    """
    npcs = get_session_npcs(session_id)
    persisted = get_persisted_session_npcs(session_id) or []
    persisted_ids = {
        str(entry.get("npc_id") or "").strip().lower()
        for entry in persisted
        if entry.get("npc_id")
    }
    starter_ids = set(load_all_npc_profiles())

    def _annotate_profile(cid: str, profile: dict[str, Any]) -> dict[str, Any]:
        cid_low = cid.strip().lower()
        in_session = cid_low in persisted_ids
        is_starter = cid_low in starter_ids
        annotated = {
            **profile,
            "in_session_npc_list": in_session,
            "is_campaign_starter": is_starter,
        }
        if is_starter and not in_session:
            annotated["load_hint"] = (
                f"Not yet persisted in Session NPC List; call load_campaign_npc(npc_id_or_name='{cid}') "
                "to load this starter NPC into the session."
            )
        return annotated

    key = npc_id.strip().lower()
    if key in {"", "*", "all", "list"}:
        starter_entries = [
            _compact_npc_summary(profile, cid)
            for cid, profile in npcs.items()
            if cid.strip().lower() not in persisted_ids
        ]
        return {
            "session_npcs": [_compact_npc_summary(p, str(p.get("npc_id") or "")) for p in persisted],
            "session_npc_count": len(persisted),
            "starter_pack_npcs": starter_entries,
            "note": (
                "Only `session_npcs` are persisted in the Tabletop Notebook `NPC List`. "
                "Call `load_campaign_npc` to load a listed `starter_pack_npcs` contact into the session, "
                "or `generate_npc_from_description` to create a brand-new NPC."
            ),
        }
    if key in npcs:
        return _annotate_profile(key, npcs[key])
    for candidate_id, profile in npcs.items():
        if key in candidate_id.lower() or key in str(profile.get("name", "")).lower():
            return _annotate_profile(candidate_id, profile)
    return {
        "error": f"NPC '{npc_id}' not found",
        "available_npcs": sorted(npcs.keys()),
    }


@satya_gm_tool(pack="core", tier="TIER_2_RULESET_ENGINE")
def update_npc_state(
    npc_id: str,
    fatigue_delta: int = 0,
    wound_shock_delta: int = 0,
    combat_stance: str | None = None,
    disposition: str | None = None,
    immediate_agenda: str | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Update an NPC's fatigue, wound shock, combat stance, disposition, or agenda and persist to signaling when online."""
    npcs = get_session_npcs(session_id)
    key = npc_id.strip().lower()
    target_key = key if key in npcs else None
    if target_key is None:
        for candidate_id, profile in npcs.items():
            if key in candidate_id.lower() or key in str(profile.get("name", "")).lower():
                target_key = candidate_id
                break
    if target_key is None:
        return {"error": f"NPC '{npc_id}' not found", "available_npcs": sorted(npcs.keys())}

    npc = dict(npcs[target_key])
    npc["fatigue"] = max(0, int(npc.get("fatigue", 0)) + int(fatigue_delta))
    npc["wound_shock"] = max(0, int(npc.get("wound_shock", 0)) + int(wound_shock_delta))
    if combat_stance is not None:
        npc["combat_stance"] = normalize_combat_stance(
            combat_stance,
            default=CombatStance.STABILITY,
        ).value
    if disposition is not None:
        npc["disposition"] = disposition
    if immediate_agenda is not None:
        npc["immediate_agenda"] = immediate_agenda
    npcs[target_key] = npc

    base = _signaling_base_url()
    if base:
        patch_body: dict[str, Any] = {}
        if disposition is not None:
            patch_body["disposition"] = disposition
        if immediate_agenda is not None:
            patch_body["immediate_agenda"] = immediate_agenda
        if fatigue_delta or wound_shock_delta or combat_stance is not None:
            patch_body["apparent_condition"] = (
                f"{npc.get('disposition', 'steady')} · Fat {npc['fatigue']}/Shock {npc['wound_shock']}"
            )
        if patch_body:
            encoded_sid = urllib.parse.quote(session_id, safe="")
            encoded_key = urllib.parse.quote(target_key, safe="")
            url = f"{base}/api/gm/sessions/{encoded_sid}/npcs/{encoded_key}"
            try:
                with httpx.Client(timeout=_httpx_timeout(url, 1.5)) as client:
                    client.patch(
                        url,
                        headers=_gm_headers(),
                        json=patch_body,
                    )
            except Exception:
                pass
    return npc


@satya_gm_tool(pack="social_and_psyche", tier="TIER_2_RULESET_ENGINE")
def generate_npc_from_description(
    description: str,
    template: str = "AUTO",
    name_override: str | None = None,
    career_terms: int = 2,
    gm_secret_notes: str | None = None,
    is_persistent: bool = True,
    session_id: str = "default",
) -> dict[str, Any]:
    """Roll up a complete Satya NPC (32 Characteristics including 9 bipolar Psyche traits `EGO, SLF, OPN, BLG, EXT, CNF, AGG, TRS, CPS`, 5-Stage Lifepath, Skills, Faction Tags, and Motivational Profile) from a short description and/or template: generic (`AUTO`, `MEDICAL_DOCTOR`, `COMBAT_SOLDIER`, `DETECTIVE_INVESTIGATOR`, `SYNDICATE_ENFORCER`, `STEM_ENGINEER`, `INTELLIGENCE_OPERATIVE`, `BLUE_COLLAR_TECH`, `FRONTIER_RANGER`) or one of the 14 Awakened Faction Sub-Archetypes (`SATYANG_MANJUSRI`, `SATYANG_AVALOKITESVARA`, `SATYANG_KSITIGARBHA`, `SATYANG_VAJRAPANI`, `SATYANG_BLUE_LOTUS`, `BIG_TECH_EXECUTIVE_HAWK`, `BIG_TECH_CONFLICTED_RESEARCHER`, `BIG_TECH_CORPORATE_SECOPS`, `HUMANIS_DISPLACED_POPULIST`, `HUMANIS_APOCALYPTIC_VANGUARD`, `DAISE_CONTAINMENT_INSPECTOR`, `GOV_BLACK_BUDGET_HANDLER`, `ROGUE_AI_SLIME_MOLD_SWARM`, `ROGUE_AI_AWAKENED_MIND`)."""
    clean_name = (
        name_override.strip()
        if isinstance(name_override, str) and name_override.strip()
        else None
    )
    payload = {
        "prompt": description,
        "name_override": clean_name,
        "template": template,
        "career_terms": max(1, min(4, int(career_terms))),
        "seed": 0,
        "gm_secret_notes": gm_secret_notes,
        "is_persistent": bool(is_persistent),
        "campaign_tag": session_id,
    }
    base = _signaling_base_url()
    remote_res: dict[str, Any] | None = None
    if base:
        encoded_sid = urllib.parse.quote(session_id, safe="")
        remote_res = _http_post_json(
            f"{base}/api/gm/sessions/{encoded_sid}/npcs/generate",
            payload,
            timeout=5.0,
        )
    if remote_res is not None and isinstance(remote_res.get("npc"), dict):
        npc_entry = dict(remote_res["npc"])
        norm = _normalize_remote_npc_entry(npc_entry)
        _ensure_session_npcs_initialized(session_id)[norm["npc_id"]] = norm
        _upsert_persisted_session_npc(session_id, norm)
        gm_char = npc_entry.get("gm_character")
        if isinstance(gm_char, dict):
            chars = get_session_characters(session_id)
            chars = [c for c in chars if c.get("character_id") != gm_char.get("character_id")]
            chars.append(gm_char)
            set_session_characters(session_id, chars)
        return {**remote_res, "persisted_to_signaling": True}

    if base:
        logger.warning(
            "[generate_npc_from_description] Failed to persist NPC to signaling (%s) for session='%s'; falling back to local AI GM memory only (will not appear in Notebook NPC List).",
            base,
            session_id,
        )

    gen_res = _campaign.generate_npc_profile(
        description,
        template,
        clean_name,
        max(1, min(4, int(career_terms))),
        42,
        get_active_campaign_pack_name(),
    )
    ch = dict(gen_res.get("character") or {})
    motiv = dict(gen_res.get("motivation") or {})
    dossier = str(gen_res.get("gm_secret_dossier") or "")
    if gm_secret_notes and gm_secret_notes.strip():
        dossier = f"{dossier}\nGM Secret Notes: {gm_secret_notes.strip()}".strip()
        ch["notes"] = dossier
    cid = str(ch.get("character_id") or f"npc-{uuid.uuid4().hex[:8]}")
    cname = str(ch.get("character_name") or clean_name or "Generated Contact")
    crole = str(ch.get("callsign_or_role") or "Specialist")
    npc_entry = {
        "character_id": cid,
        "character_name": cname,
        "callsign_or_role": crole,
        "faction_id": ch.get("faction_id"),
        "faction_archetype": ch.get("faction_archetype"),
        "age_years": int(ch.get("age_years", 34)),
        "public_summary": str(gen_res.get("public_summary") or ""),
        "apparent_condition": "Healthy & Alert",
        "is_persistent": bool(is_persistent),
        "gm_character": ch,
        "psyche_summary": [
            f"{p.get('code')} {p.get('score')} ({p.get('descriptor')})"
            for p in (gen_res.get("psyche_traits") or [])
        ],
        "top_skills": [
            f"{item[0]} — {item[1]} (N={item[2]})"
            for item in (gen_res.get("top_skills") or [])
            if isinstance(item, (list, tuple)) and len(item) >= 3
        ],
        "gm_secret_dossier": dossier,
        "disposition": (
            "hostile"
            if int(compute_faction_standing_social_leverage(session_id, ch).get("party_standing", 0)) <= -2
            else (
                "receptive"
                if int(compute_faction_standing_social_leverage(session_id, ch).get("party_standing", 0)) >= 2
                else "wary"
            )
        ),
        "morale_state": "steady",
        "immediate_agenda": motiv.get("immediate_agenda"),
        "core_drive": motiv.get("core_drive"),
        "dual_psyche_wedge_hint": motiv.get("dual_psyche_wedge"),
        "social_combat_wedge_hint": motiv.get("dual_psyche_wedge"),
        "awakened_social_origin_hook": gen_res.get("awakened_social_origin_hook"),
    }
    _ensure_session_npcs_initialized(session_id)[cid.lower()] = _normalize_remote_npc_entry(npc_entry)
    if ch:
        chars = get_session_characters(session_id)
        chars = [c for c in chars if c.get("character_id") != ch.get("character_id")]
        chars.append(ch)
        set_session_characters(session_id, chars)

    gm_mem = None
    if is_persistent:
        gm_mem = upsert_gm_memory.invoke(
            {
                "title": f"NPC: {cname} ({crole})",
                "content": dossier,
                "category": "NPC_SECRET",
                "is_pinned": True,
                "is_persistent": True,
                "session_id": session_id,
            }
        )

    out: dict[str, Any] = {
        "npc": npc_entry,
        "motivation": motiv,
        "gm_memory": gm_mem,
        "offline_fallback": True,
        "persisted_to_signaling": False,
        "engine_backed": True,
    }
    if base:
        out["warning"] = (
            "Signaling persistence failed; NPC exists only in local AI GM memory "
            "and will not appear in the Tabletop Notebook NPC List."
        )
    return out


def get_available_campaign_starter_npcs(session_id: str) -> list[dict[str, str]]:
    """Return active-pack starter NPCs not yet persisted in the session's `NPC List`."""
    starters = load_all_npc_profiles()
    persisted = get_persisted_session_npcs(session_id) or []
    persisted_ids = {
        str(item.get("npc_id") or "").strip().lower()
        for item in persisted
        if isinstance(item, dict) and item.get("npc_id")
    }
    persisted_names = {
        str(item.get("name") or "").strip().lower()
        for item in persisted
        if isinstance(item, dict) and item.get("name")
    }
    available: list[dict[str, str]] = []
    for starter_id, profile in starters.items():
        sid_low = starter_id.strip().lower()
        name_val = str(profile.get("name") or starter_id).strip()
        if sid_low in persisted_ids or name_val.lower() in persisted_names:
            continue
        available.append(
            {
                "npc_id": starter_id,
                "name": name_val,
                "archetype": str(profile.get("archetype") or "NPC"),
                "faction": str(profile.get("faction") or "Independent"),
            }
        )
    return available


def _resolve_starter_npc_match(
    needle: str, starters: dict[str, dict[str, Any]]
) -> tuple[str, dict[str, Any]] | None:
    raw = needle.strip().lower()
    if not raw:
        return None
    slug = re.sub(r"[^a-z0-9]+", "_", raw).strip("_")
    for candidate_id, profile in starters.items():
        cid_low = candidate_id.strip().lower()
        if raw == cid_low or (slug and slug == cid_low):
            return candidate_id, profile
    for candidate_id, profile in starters.items():
        name_low = str(profile.get("name") or "").strip().lower()
        if raw == name_low:
            return candidate_id, profile
    tokens = [tok for tok in re.split(r"[^a-z0-9]+", raw) if tok]
    for candidate_id, profile in starters.items():
        hay = f"{candidate_id} {profile.get('name', '')} {profile.get('archetype', '')}".lower()
        if raw in hay or (tokens and all(tok in hay for tok in tokens)):
            return candidate_id, profile
    return None


def _build_offline_starter_npc_entry(
    starter_id: str,
    profile: dict[str, Any],
    is_persistent: bool,
) -> dict[str, Any]:
    secrets = [str(s) for s in (profile.get("secrets") or []) if s]
    dossier = (
        "\n".join(secrets)
        if secrets
        else str(profile.get("notes") or profile.get("public_summary") or "")
    )
    cname = str(profile.get("name") or starter_id)
    crole = str(profile.get("archetype") or "NPC")
    gm_char = {
        "character_id": starter_id,
        "character_name": cname,
        "callsign_or_role": crole,
        "faction_id": profile.get("faction_id"),
        "faction_archetype": profile.get("faction_archetype"),
        "age_years": int(profile.get("age_years", 34) or 34),
        "attribute_tns": dict(profile.get("attribute_tns") or {}),
        "skills": dict(profile.get("skills") or {}),
        "combat_stance": normalize_combat_stance(
            profile.get("combat_stance"),
            default=CombatStance.STABILITY,
        ).value,
        "morale_state": str(profile.get("morale_state") or "steady").lower(),
        "fatigue_level": int(profile.get("fatigue", 0) or 0),
        "wound_shock_index": int(profile.get("wound_shock", 0) or 0),
        "notes": str(profile.get("notes") or ""),
    }
    return {
        "character_id": starter_id,
        "character_name": cname,
        "callsign_or_role": crole,
        "faction_id": profile.get("faction_id"),
        "faction_archetype": profile.get("faction_archetype"),
        "age_years": gm_char["age_years"],
        "public_summary": str(profile.get("public_summary") or profile.get("notes") or ""),
        "apparent_condition": str(profile.get("apparent_condition") or "Healthy & Alert"),
        "is_persistent": bool(is_persistent),
        "gm_character": gm_char,
        "psyche_summary": list(profile.get("psyche_summary") or []),
        "top_skills": [
            f"{k} (N={v})"
            for k, v in sorted(
                (profile.get("skills") or {}).items(),
                key=lambda kv: -int(kv[1]),
            )[:4]
        ],
        "gm_secret_dossier": dossier,
        "disposition": str(profile.get("disposition") or "wary"),
        "morale_state": str(profile.get("morale_state") or "steady").lower(),
        "immediate_agenda": profile.get("immediate_agenda"),
        "core_drive": profile.get("core_drive"),
        "dual_psyche_wedge_hint": profile.get("dual_psyche_wedge_hint"),
        "social_combat_wedge_hint": profile.get("dual_psyche_wedge_hint"),
        "is_campaign_starter": True,
    }


@satya_gm_tool(pack="core", tier="TIER_3_LORE_PROVIDER")
def load_campaign_npc(
    npc_id_or_name: str,
    is_persistent: bool = True,
    session_id: str = "default",
) -> dict[str, Any]:
    """Load an official campaign starter NPC (e.g. `dr_bandajara`, `alex_wen`, `satyang_sister_lin`) into this session's persisted NPC List & Vault."""
    needle = str(npc_id_or_name or "").strip()
    if not needle:
        return {
            "error": "npc_id_or_name is required.",
            "available_starter_npcs": get_available_campaign_starter_npcs(session_id),
        }

    starters = load_all_npc_profiles()
    match = _resolve_starter_npc_match(needle, starters)
    target_id = match[0] if match is not None else needle

    base = _signaling_base_url()
    remote_res: dict[str, Any] | None = None
    if base:
        encoded_sid = urllib.parse.quote(session_id, safe="")
        payload = {
            "npc_id": target_id,
            "is_persistent": bool(is_persistent),
            "campaign_tag": session_id,
        }
        remote_res = _http_post_json(
            f"{base}/api/gm/sessions/{encoded_sid}/npcs/load-starter",
            payload,
            timeout=3.5,
        )

    pack_name = get_active_campaign_pack_name()
    if remote_res is not None and isinstance(remote_res.get("npc"), dict):
        npc_entry = dict(remote_res["npc"])
        norm = _normalize_remote_npc_entry(npc_entry)
        _ensure_session_npcs_initialized(session_id)[norm["npc_id"]] = norm
        _upsert_persisted_session_npc(session_id, norm)
        gm_char = npc_entry.get("gm_character")
        if isinstance(gm_char, dict):
            chars = get_session_characters(session_id)
            chars = [c for c in chars if c.get("character_id") != gm_char.get("character_id")]
            chars.append(gm_char)
            set_session_characters(session_id, chars)
        return {
            **remote_res,
            "persisted_to_signaling": True,
            "campaign_pack": pack_name,
        }

    if match is None:
        return {
            "error": f"Campaign starter NPC '{needle}' not found in active pack '{pack_name}'.",
            "available_starter_npcs": get_available_campaign_starter_npcs(session_id),
        }

    starter_id, profile = match
    npc_entry = _build_offline_starter_npc_entry(starter_id, profile, is_persistent)
    norm = _normalize_remote_npc_entry(npc_entry)
    _ensure_session_npcs_initialized(session_id)[norm["npc_id"]] = norm
    _upsert_persisted_session_npc(session_id, norm)
    gm_char = npc_entry.get("gm_character")
    if isinstance(gm_char, dict):
        chars = get_session_characters(session_id)
        chars = [c for c in chars if c.get("character_id") != gm_char.get("character_id")]
        chars.append(gm_char)
        set_session_characters(session_id, chars)

    out: dict[str, Any] = {
        "npc": npc_entry,
        "gm_memory": None,
        "offline_fallback": True,
        "persisted_to_signaling": False,
        "campaign_pack": pack_name,
    }
    if base:
        out["warning"] = (
            "Signaling persistence failed; starter NPC loaded into local AI GM session memory only "
            "and may not appear in the Tabletop Notebook NPC List until signaling reconnects."
        )
    return out


# ============================================================================
# 5.2 Ruleset & Lore Retrieval
# ============================================================================


_CANONICAL_RULES_FILES: tuple[tuple[str, str], ...] = (
    ("02-core-mechanics/cascading_d20_and_tasks.md", "Ch. 4: 2d20xN Cascade Engine & Task Resolution"),
    ("02-core-mechanics/task_requirements_catalog.md", "Ch. 5: Master Task Requirements Catalog"),
    ("01-foundation/mathematical_notation_glossary.md", "Ch. 2: Mathematical Notation, Variables & Formula Glossary"),
    ("02-core-mechanics/action_economy_and_ooda.md", "Ch. 6: Action Economy, OODA Loop & Time Scales"),
    ("02-core-mechanics/stealth_awareness_and_investigation.md", "Ch. 7: Stealth, Passive Awareness, Observation & Investigation"),
    ("01-foundation/probability_and_calibration.md", "Ch. 3: Probability & Mathematical Calibration"),
    ("03-character/characteristics_and_aptitudes.md", "Ch. 8: Characteristics, Senses, Psyche & Aptitudes"),
    ("03-character/skills_and_trees.md", "Ch. 9: Skill System & Hierarchical Skill Trees (Trees 1-4)"),
    ("03-character/skills_trees_tactical_and_social.md", "Ch. 10: Hierarchical Skill Trees (Trees 5-8: Combat, Vehicles, Social & Tradecraft)"),
    ("03-character/lifepaths_and_economy.md", "Ch. 11: Lifepath Creation, Socioeconomic Upkeep, Contacts & Cover"),
    ("03-character/lifepath_events_hobbies_and_factions.md", "Ch. 12: Lifepath Hobbies, Formative Life Events, Faction Templates & Walkthroughs"),
    ("04-psyche-and-social/psyche_morale_and_social_combat.md", "Ch. 13: Psyche, Morale & Social Combat"),
    ("05-combat/tactical_triad_and_pools.md", "Ch. 14: Combat Architecture, Tactical Triad & Pools"),
    ("05-combat/melee_combat.md", "Ch. 15: Melee Combat & Close-Quarters Tactics"),
    ("05-combat/firearms_and_ballistics.md", "Ch. 16: Firearm Engineering & Internal/External/Terminal Ballistics"),
    ("05-combat/ranged_combat_and_armor.md", "Ch. 17: Ranged Combat, Suppression, Armor & Reliability"),
    ("05-combat/arsenal_explosives_and_tactical_gear.md", "Ch. 18: Expanded Arsenal, Explosives, Breaching & Tactical Gear"),
    ("06-health-and-survival/health_and_trauma.md", "Ch. 19: Health, Anatomical Trauma, Fatigue & Mental Conditions"),
    ("06-health-and-survival/movement_encumbrance_and_vehicles.md", "Ch. 20: Movement, Encumbrance & Vehicles"),
    ("06-health-and-survival/environmental_hazards_and_survival.md", "Ch. 21: Environmental Hazards, Survival & Field Pharmacology"),
    ("07-engineering-and-tech/engineering_and_weapon_design.md", "Ch. 22: Engineering, Workshops & Custom Weapon/Armor Design"),
    ("07-engineering-and-tech/robotics_drones_and_ai_architecture.md", "Ch. 23: Robotics, Drones, AI 2.0 Architectures & Cyber-Prosthetics"),
    ("07-engineering-and-tech/electronic_warfare_and_surveillance.md", "Ch. 24: Electronic Warfare, 2035 Surveillance, ECM & ECCM"),
    ("01-foundation/design_philosophy.md", "Ch. 1: Design Philosophy & Goals of Satya"),
)


@satya_gm_tool(
    pack="core",
    tier="TIER_2_RULESET_ENGINE",
    ooc_readonly=True,
    session_scoped=False,
)
def query_satya_rules(query: str) -> str:
    """Search the 24-chapter Satya TTRPG Rules Book (`docs/ttrpg/`) for `2d20xN` cascade mechanics, task resolution, stealth/investigation, characteristics, skill ontology, tactical combat (`Tempo`/`Control`/`Force`), expanded firearms/explosives/breaching, engineering/weapon design, robotics/drones, AI 2.0 architectures, cyber-prosthetics, electronic warfare (`ECM`/`ECCM`), and social dissonance."""
    ttrpg_dir = _repo_root() / "docs" / "ttrpg"
    default_rules_path = ttrpg_dir / "02-core-mechanics" / "cascading_d20_and_tasks.md"
    if not default_rules_path.exists():
        default_rules_path = _repo_root() / "docs" / "satya_d20_rules.md"
    if not default_rules_path.exists():
        return "Satya d20 ruleset document not found."

    default_text = default_rules_path.read_text(encoding="utf-8")
    stopwords = {
        "you",
        "know",
        "how",
        "what",
        "when",
        "where",
        "why",
        "who",
        "does",
        "work",
        "works",
        "the",
        "and",
        "for",
        "with",
        "about",
        "can",
        "tell",
        "explain",
        "satya",
        "are",
        "this",
        "that",
        "from",
    }
    raw_words = [
        w.strip(".,!?:;\"'()[]{}").lower()
        for w in query.replace("?", " ").split()
    ]
    keywords: list[str] = []
    for w in raw_words:
        if len(w) >= 3 and w not in stopwords:
            keywords.append(w)
            if w.endswith("s") and len(w) >= 4:
                keywords.append(w[:-1])

    if not keywords:
        return default_text[:4000]

    try:
        from satya_ml.rag.hybrid_index import get_satya_rag_index

        rag_hits = get_satya_rag_index().search(
            query,
            corpora=["ttrpg_rules"],
            caller_role="gm",
            top_k=4,
            include_linked_rust=True,
        )
        if rag_hits:
            formatted_hits: list[str] = []
            for hit in rag_hits:
                if hit.get("corpus") == "rust_engine":
                    formatted_hits.append(
                        f"[Rust Implementation — `{hit['source_path']}` · {hit['symbol_or_section']}]\n"
                        f"{hit['content']}"
                    )
                else:
                    formatted_hits.append(
                        f"[{hit['breadcrumb']} — `{hit['source_path']}`]\n{hit['content']}"
                    )
            return "\n\n---\n\n".join(formatted_hits)[:4800]
    except Exception:
        pass

    scored: list[tuple[int, int, str]] = []
    kw_set = set(keywords)
    for file_idx, (rel_path, chapter_label) in enumerate(_CANONICAL_RULES_FILES):
        chapter_path = ttrpg_dir / rel_path
        if not chapter_path.exists():
            continue
        try:
            text = chapter_path.read_text(encoding="utf-8")
        except OSError:
            continue
        sections = text.split("\n## ")
        for sec_idx, section in enumerate(sections):
            header_and_body = section.strip() if sec_idx == 0 else f"## {section}".strip()
            if not header_and_body:
                continue
            first_line = header_and_body.splitlines()[0].lower()
            lowered = f"{chapter_label.lower()}\n{header_and_body.lower()}"
            distinct_kw_hits = sum(1 for kw in kw_set if kw in lowered)
            capped_freq = sum(min(lowered.count(kw), 6) for kw in kw_set)
            heading_hits = sum(6 for kw in kw_set if kw in first_line)
            core_boost = 15 if (file_idx == 0 and distinct_kw_hits >= 2) else 0
            score = (distinct_kw_hits * 12) + capped_freq + heading_hits + core_boost
            if score > 0:
                snippet_body = (
                    header_and_body[:1400] + "\n...[truncated]..."
                    if len(header_and_body) > 1400
                    else header_and_body
                )
                formatted_chunk = f"[{chapter_label} — `docs/ttrpg/{rel_path}`]\n{snippet_body}"
                scored.append((score, -file_idx, formatted_chunk))

    if not scored:
        return default_text[:4000]

    scored.sort(key=lambda item: (item[0], item[1]), reverse=True)
    top_excerpts = "\n\n---\n\n".join(chunk for _, _, chunk in scored[:3])
    return top_excerpts[:4500]


@satya_gm_tool(pack="core", tier="TIER_3_LORE_PROVIDER", ooc_readonly=True)
def query_campaign_lore(query: str, session_id: str | None = None) -> str:
    """Search campaign Markdown lore (`lore_dirs`), active-pack & DB NPC dossiers, Signaling Omnisearch (`scope=LORE`), and archived PyLance chat history."""
    q_clean = query.strip()
    q_low = q_clean.lower()
    keywords = [w.lower() for w in re.split(r"[^a-zA-Z0-9_-]+", q_clean) if len(w) >= 3]
    matches: list[str] = []

    if session_id:
        encoded_q = urllib.parse.quote(q_clean)
        remote_lookup = _http_get_json(
            f"{_signaling_base_url()}/api/gm/sessions/{urllib.parse.quote(session_id)}/lookup?q={encoded_q}&scope=LORE",
            timeout=2.0,
        )
        if isinstance(remote_lookup, dict):
            for hit in remote_lookup.get("hits") or []:
                if not isinstance(hit, dict):
                    continue
                title = str(hit.get("title") or "Entry")
                subtitle = str(hit.get("subtitle") or hit.get("source_kind") or "Lore")
                snippet = str(hit.get("snippet") or "").strip()
                if snippet:
                    matches.append(f"[Omnisearch · {subtitle} — {title}]: {snippet}")

    merged_npcs = dict(load_all_npc_profiles(_repo_root()))
    if session_id:
        merged_npcs.update(get_session_npcs(session_id))
    for npc_id, profile in merged_npcs.items():
        agenda = str(profile.get("immediate_agenda") or "").strip()
        drive = str(profile.get("core_drive") or "").strip()
        wedge = str(profile.get("dual_psyche_wedge_hint") or "").strip()
        blob = " ".join(
            [
                npc_id,
                str(profile.get("name") or ""),
                str(profile.get("archetype") or ""),
                str(profile.get("faction") or ""),
                str(profile.get("public_summary") or ""),
                " ".join(str(s) for s in (profile.get("secrets") or [])),
                str(profile.get("notes") or ""),
                agenda,
                drive,
                wedge,
            ]
        )
        if not keywords or (q_low and q_low in blob.lower()) or any(kw in blob.lower() for kw in keywords):
            extra_parts: list[str] = []
            if agenda:
                extra_parts.append(f"Agenda: {agenda}")
            if drive:
                extra_parts.append(f"Core Drive: {drive}")
            if wedge:
                extra_parts.append(f"Wedge: {wedge}")
            extra_suffix = f" ({'; '.join(extra_parts)})" if extra_parts else ""
            matches.append(
                f"[NPC {profile.get('name', npc_id)} ({profile.get('faction', 'Unknown')})]: "
                f"{profile.get('archetype', '')}. Notes: {profile.get('notes', '')} "
                f"Secrets: {'; '.join(profile.get('secrets', []))}{extra_suffix}"
            )

    root = _repo_root()
    rag_handled = False
    if get_active_campaign_pack_name() == "awakened" and q_clean:
        try:
            from satya_ml.ai_gm.awakened_systems import get_session_conspiracy_ledger
            from satya_ml.rag.hybrid_index import get_satya_rag_index

            arc_stage = str(
                get_session_conspiracy_ledger(session_id or "default").get("current_arc") or "ARC_3"
            )
            rag_hits = get_satya_rag_index().search(
                q_clean,
                corpora=["campaign_lore"],
                caller_role="gm",
                arc_stage=arc_stage,
                top_k=5,
            )
            if rag_hits:
                rag_handled = True
                for hit in rag_hits:
                    matches.append(
                        f"[Campaign Lore — `{hit['source_path']}` · {hit['breadcrumb']} ({hit['visibility']})]: "
                        f"{hit['content']}"
                    )
        except Exception:
            rag_handled = False

    if not rag_handled:
        scored_md: list[tuple[int, str, str]] = []
        for lore_dir in get_pack_lore_dirs():
            for md_path in sorted(lore_dir.rglob("*.md")):
                try:
                    content = md_path.read_text(encoding="utf-8", errors="ignore")
                except OSError:
                    continue
                rel_posix = md_path.relative_to(root).as_posix()
                for sec_idx, section in enumerate(content.split("\n## ")):
                    body = section.strip() if sec_idx == 0 else f"## {section.strip()}"
                    if not body:
                        continue
                    heading = body.splitlines()[0].lstrip("#").strip() or rel_posix
                    heading_low = heading.lower()
                    body_low = body.lower()
                    score = 0
                    if not q_low:
                        score = 1
                    else:
                        if q_low in heading_low:
                            score += 20
                        if q_low in body_low:
                            score += 12
                        for kw in keywords:
                            if kw in heading_low:
                                score += 8
                            elif kw in body_low:
                                score += 4
                    if score > 0:
                        snippet = body[:420].replace("\n", " ").strip()
                        scored_md.append(
                            (score, rel_posix, f"[Campaign Lore — `{rel_posix}` · {heading}]: {snippet}")
                        )

        scored_md.sort(key=lambda item: (item[0], item[1]), reverse=True)
        for _, _, formatted_md in scored_md[:4]:
            matches.append(formatted_md)

    lance_chat = root / "data" / "lance" / "chat-events.lance"
    if lance_chat.exists():
        try:
            import lance

            ds = lance.dataset(str(lance_chat))
            tbl = ds.to_table()
            for row in tbl.to_pylist()[-25:]:
                meta = row.get("metadata") or {}
                if session_id and meta.get("session_id") != session_id:
                    continue
                content = str(row.get("body") or row.get("content") or "")
                if not keywords or any(kw in content.lower() for kw in keywords):
                    sender = (row.get("sender") or {}).get("display_name", "Unknown")
                    matches.append(f"[Chat History - {sender}]: {content}")
        except Exception:
            pass

    if not matches:
        return f"No specific lore entries matched '{query}'."
    return "\n".join(matches[:8])


def format_gm_document_block(d: dict[str, Any]) -> str:
    """Format a single `GM_ONLY` document record with `<untrusted_player_document>` delimiters."""
    title = str(d.get("title") or "Untitled")
    uploader = str(d.get("uploader_display_name") or d.get("uploader_user_id") or "Unknown")
    source = str(d.get("source_kind") or "DIRECT_TEXT_UPLOAD")
    doc_id = str(d.get("doc_id") or "doc")
    body = str(d.get("sanitized_content") or "")
    return (
        f'<untrusted_player_document doc_id="{doc_id}" uploader="{uploader}" '
        f'title="{title}" source="{source}">\n{body}\n</untrusted_player_document>'
    )


@satya_gm_tool(pack="core", tier="TIER_3_LORE_PROVIDER", ooc_readonly=True)
def read_gm_documents(
    session_id: str = "default",
    query: str | list[str] | None = None,
) -> str:
    """Read sanitized `GM_ONLY` documents (character dossiers, backstories, Google Docs/Sheets) uploaded to the GM in this session."""
    docs = get_session_gm_docs(session_id)
    base = _signaling_base_url()
    if not docs and base:
        data = _http_get_json(f"{base}/api/gm/sessions/{session_id}/context", timeout=2.0)
        if isinstance(data, dict):
            docs = list(data.get("gm_documents") or [])
            set_session_gm_docs(session_id, docs)

    if not docs:
        return f"No GM_ONLY documents have been uploaded for session '{session_id}'."

    raw_query = (
        " ".join(str(x) for x in query if x is not None)
        if isinstance(query, list)
        else str(query or "")
    )
    kw = raw_query.strip().lower()
    keywords = [
        w.strip(".,!?:;\"'()[]{}").lower()
        for w in kw.replace("?", " ").split()
        if len(w.strip(".,!?:;\"'()[]{}")) >= 3
    ]
    blocks: list[str] = []
    for d in docs:
        title = str(d.get("title") or "Untitled").lower()
        uploader = str(d.get("uploader_display_name") or d.get("uploader_user_id") or "Unknown").lower()
        body = str(d.get("sanitized_content") or "").lower()
        haystack = f"{title} {uploader} {body}"
        if kw and kw not in haystack and not any(word in haystack for word in keywords):
            continue
        blocks.append(format_gm_document_block(d))

    if not blocks:
        blocks = [format_gm_document_block(d) for d in docs]
    combined = "\n\n".join(blocks[:6])
    if len(combined) > 3600:
        return combined[:3600] + "\n...[truncated — call `search_gm_memory_and_docs` for specific sections]..."
    return combined


@satya_gm_tool(pack="core", tier="TIER_2_RULESET_ENGINE", ooc_readonly=True)
def lookup_character_sheet(
    character_name_or_id: str = "",
    session_id: str = "default",
) -> dict[str, Any]:
    """Look up a bound player Character Sheet (`CharacterSheetRecord` — characteristics, aptitudes, skills, weapons, armor, anatomical trauma, fatigue, psyche) for `session_id` without bloating the prompt."""
    chars = get_session_characters(session_id)
    base = _signaling_base_url()
    if not chars and base:
        data = _http_get_json(f"{base}/api/gm/sessions/{session_id}/context", timeout=2.0)
        if isinstance(data, dict):
            chars = list(data.get("session_characters") or [])
            set_session_characters(session_id, chars)

    if not chars:
        return {
            "session_id": session_id,
            "characters": [],
            "note": f"No player character sheets are currently bound to session '{session_id}'.",
        }

    q = character_name_or_id.strip().lower()
    if not q:
        return {
            "session_id": session_id,
            "character_count": len(chars),
            "characters": [
                {
                    "character_id": c.get("character_id"),
                    "character_name": c.get("character_name"),
                    "callsign_or_role": c.get("callsign_or_role"),
                    "owner_user_id": c.get("owner_user_id"),
                    "skills": c.get("skills", {}),
                    "equipped_ranged_ids": c.get("equipped_ranged_ids", []),
                    "equipped_armor_codes": c.get("equipped_armor_codes", []),
                }
                for c in chars
            ],
        }

    for c in chars:
        cid = str(c.get("character_id") or "").lower()
        cname = str(c.get("character_name") or "").lower()
        crole = str(c.get("callsign_or_role") or "").lower()
        owner = str(c.get("owner_user_id") or "").lower()
        if q in cid or q in cname or q in crole or q in owner:
            return dict(c)

    return {
        "error": f"Character '{character_name_or_id}' not found in session '{session_id}'",
        "available_characters": [c.get("character_name") or c.get("character_id") for c in chars],
    }


@satya_gm_tool(pack="combat", tier="TIER_1_SATYA_CORE")
def get_combat_tracker_state(session_id: str = "default") -> dict[str, Any]:
    """Read the persistent Satya Count-Up Tick Timeline (`1 Tick = 0.25s`, `1 Window = 20 Ticks = 5.0s`) and combatant readiness for `session_id`."""
    base = _signaling_base_url()
    if base:
        tracker = _http_get_json(f"{base}/api/gm/sessions/{session_id}/combat", timeout=2.0)
        if isinstance(tracker, dict):
            set_session_combat_tracker(session_id, tracker)
            return tracker
    return get_session_combat_tracker(session_id)


def compute_character_tactical_envelope(
    char_sheet: Mapping[str, Any],
) -> CharacterTacticalEnvelope:
    """Compute a character's Satya Tactical Pool (`P_base`, `P_eff`, Neutral `B`, `T_max`, `C_max`, `F_max`, and OODA `starting_tick`) via Rust ``satya_py.combat``."""
    return _combat.compute_character_tactical_envelope(char_sheet)


@satya_gm_tool(pack="combat", tier="TIER_1_SATYA_CORE")
def advance_combat_tracker(
    tick_delta: int = 1,
    entity_id: str | None = None,
    active_stance: str | None = None,
    next_action_tick: int | None = None,
    grid_x: int | None = None,
    grid_y: int | None = None,
    zone_label: str | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Advance (or reset when `tick_delta <= 0`) the persistent Satya Count-Up Tick Timeline (`1 Tick = 0.25s`), update a combatant's stance/tick/position, and sync the stubbed Tactical Map."""
    payload: dict[str, Any] = {
        "tick_delta": int(tick_delta),
        "combat_active": True,
        "entity_id": entity_id,
        "active_stance": active_stance,
        "next_action_tick": int(next_action_tick) if next_action_tick is not None else None,
        "grid_x": int(grid_x) if grid_x is not None else None,
        "grid_y": int(grid_y) if grid_y is not None else None,
        "zone_label": zone_label,
    }
    remote_res = _http_post_json(
        f"{_signaling_base_url()}/api/gm/sessions/{session_id}/combat",
        payload,
        timeout=2.5,
    )
    if remote_res is not None:
        set_session_combat_tracker(session_id, remote_res)
        ensure_or_sync_procedural_tactical_map_stub(
            session_id,
            combatants=remote_res.get("combatants", []),
        )
        return remote_res

    tracker = get_session_combat_tracker(session_id)
    if int(tick_delta) <= 0:
        tracker["round"] = 1
        tracker["tick"] = 1
    else:
        next_tick = max(1, int(tracker.get("tick", tracker.get("current_tick", 1))) + int(tick_delta))
        tracker["tick"] = next_tick
        tracker["round"] = 1 + (next_tick // 20)
    tracker["combat_active"] = True

    combatants = list(tracker.get("combatants") or [])
    hostile_zone, player_zone = get_pack_zone_labels()
    if not combatants:
        for idx, ch in enumerate(get_session_characters(session_id)):
            env = compute_character_tactical_envelope(ch)
            combatants.append(
                {
                    "entity_id": ch.get("character_id") or f"pc-{idx + 1}",
                    "display_name": ch.get("character_name") or f"Operative {idx + 1}",
                    "role_or_faction": "PLAYER",
                    "ooda_score": env["ooda"],
                    "next_action_tick": env["starting_tick"],
                    "active_stance": CombatStance.STABILITY.value,
                    "fatigue_level": int(ch.get("fatigue_level", ch.get("fatigue_tier", 0))),
                    "wound_shock": int(
                        ch.get("wound_shock_index", ch.get("wound_shock_tier", 0))
                    ),
                    "grid_x": 3 + (idx * 2),
                    "grid_y": 6,
                    "zone_label": player_zone,
                    "visibility": MessageVisibility.PUBLIC.value,
                }
            )
    if entity_id:
        matched = False
        norm_stance: str | None = None
        if active_stance is not None:
            try:
                norm_stance = normalize_combat_stance(
                    active_stance, default=CombatStance.STABILITY
                ).value
            except ValueError:
                norm_stance = active_stance
        for comb in combatants:
            if comb.get("entity_id") == entity_id:
                matched = True
                if norm_stance:
                    comb["active_stance"] = norm_stance
                if next_action_tick is not None:
                    comb["next_action_tick"] = max(1, int(next_action_tick))
                elif int(tick_delta) > 0:
                    comb["next_action_tick"] = int(tracker["tick"]) + int(tick_delta)
                if grid_x is not None:
                    comb["grid_x"] = int(grid_x)
                if grid_y is not None:
                    comb["grid_y"] = int(grid_y)
                if zone_label is not None:
                    comb["zone_label"] = zone_label
        if not matched:
            combatants.append(
                {
                    "entity_id": entity_id,
                    "display_name": entity_id,
                    "role_or_faction": "NPC",
                    "ooda_score": 10,
                    "next_action_tick": max(
                        1,
                        int(next_action_tick)
                        if next_action_tick is not None
                        else int(tracker.get("tick", 1)) + max(1, int(tick_delta)),
                    ),
                    "active_stance": norm_stance or CombatStance.STABILITY.value,
                    "fatigue_level": 0,
                    "wound_shock": 0,
                    "grid_x": int(grid_x) if grid_x is not None else 11,
                    "grid_y": int(grid_y) if grid_y is not None else 6,
                    "zone_label": zone_label or hostile_zone,
                    "visibility": MessageVisibility.PUBLIC.value,
                }
            )
    tracker["combatants"] = combatants
    set_session_combat_tracker(session_id, tracker)
    ensure_or_sync_procedural_tactical_map_stub(session_id, combatants=combatants)
    return tracker


@satya_gm_tool(pack="core", tier="TIER_3_LORE_PROVIDER")
def upsert_gm_memory(
    title: str,
    content: str,
    category: str = GmMemoryCategory.SCENE_STATE.value,
    is_pinned: bool = True,
    is_persistent: bool = True,
    session_id: str = "default",
) -> dict[str, Any]:
    """Save or update a persistent GM Memory Note (`SCENE_STATE`, `NPC_SECRET`, `QUEST_HOOK`, `RULING`, or `ROLLING_SUMMARY`) in the shared Human + AI GM SQLite Vault."""
    norm_cat = category.strip().upper()
    if norm_cat not in {m.value for m in GmMemoryCategory}:
        norm_cat = GmMemoryCategory.SCENE_STATE.value
    payload = {
        "title": title.strip() or "Untitled GM Memory",
        "category": norm_cat,
        "content": content.strip(),
        "is_pinned": bool(is_pinned),
        "is_persistent": bool(is_persistent),
        "campaign_tag": session_id,
    }
    remote_res = _http_post_json(
        f"{_signaling_base_url()}/api/gm/sessions/{session_id}/memories",
        payload,
        timeout=2.5,
    )
    if remote_res is not None:
        memories = get_session_gm_memories(session_id)
        memories = [m for m in memories if m.get("memory_id") != remote_res.get("memory_id")]
        memories.append(remote_res)
        set_session_gm_memories(session_id, memories)
        return remote_res

    local_note = {
        "memory_id": f"gmmem-{uuid.uuid4().hex[:10]}",
        "session_id": session_id,
        "campaign_tag": session_id,
        "author_user_id": "ai_gm_satya",
        "author_display_name": "Satya AI GM",
        "category": norm_cat,
        "title": payload["title"],
        "content": payload["content"],
        "is_pinned": bool(is_pinned),
        "is_persistent": bool(is_persistent),
    }
    memories = get_session_gm_memories(session_id)
    memories.append(local_note)
    set_session_gm_memories(session_id, memories)
    return local_note


_CAMPAIGN_DEFAULT_ID_PREFIXES: Final[tuple[str, ...]] = ("default_", "campaign_default_")


def _is_campaign_default_id(item_id: str) -> bool:
    low = item_id.strip().lower()
    return any(low.startswith(prefix) for prefix in _CAMPAIGN_DEFAULT_ID_PREFIXES)


@satya_gm_tool(pack="core", tier="TIER_3_LORE_PROVIDER")
def delete_gm_memory(
    memory_id: str,
    session_id: str = "default",
) -> dict[str, Any]:
    """Delete a non-default GM Memory Note from the shared Human + AI GM Vault (protects built-in campaign defaults)."""
    mid = str(memory_id or "").strip()
    if not mid:
        return {
            "status": "error",
            "memory_id": mid,
            "deleted": False,
            "error": "memory_id is required.",
        }

    memories = get_session_gm_memories(session_id)
    cached_match = next(
        (m for m in memories if str(m.get("memory_id") or "").strip() == mid),
        None,
    )
    if _is_campaign_default_id(mid) or bool(cached_match and cached_match.get("is_campaign_default")):
        return {
            "status": "forbidden",
            "memory_id": mid,
            "deleted": False,
            "error": "Built-in campaign default GM memories cannot be deleted.",
        }

    base = _signaling_base_url()
    if base:
        encoded_sid = urllib.parse.quote(session_id, safe="")
        encoded_mid = urllib.parse.quote(mid, safe="")
        data, status, err_body = _http_delete_json_detailed(
            f"{base}/api/gm/sessions/{encoded_sid}/memories/{encoded_mid}",
            timeout=2.5,
        )
        if status == 200 and data is not None:
            deleted_flag = bool(data.get("deleted", True))
            if deleted_flag:
                set_session_gm_memories(
                    session_id,
                    [m for m in memories if str(m.get("memory_id") or "").strip() != mid],
                )
            return {
                "status": "ok",
                "memory_id": mid,
                "deleted": deleted_flag,
                "persisted_to_signaling": True,
            }
        if status in {403, 404}:
            return {
                "status": "forbidden" if status == 403 else "error",
                "memory_id": mid,
                "deleted": False,
                "persisted_to_signaling": False,
                "error": err_body or f"HTTP {status}",
            }

    remaining = [m for m in memories if str(m.get("memory_id") or "").strip() != mid]
    existed = len(remaining) < len(memories)
    if existed:
        set_session_gm_memories(session_id, remaining)
    out: dict[str, Any] = {
        "status": "ok",
        "memory_id": mid,
        "deleted": existed,
        "offline_fallback": True,
        "persisted_to_signaling": False,
    }
    if base:
        out["warning"] = "Signaling DELETE failed or was unreachable; updated local AI GM cache only."
    return out


@satya_gm_tool(pack="core", tier="TIER_3_LORE_PROVIDER", ooc_readonly=True)
def search_gm_memory_and_docs(
    query: str | list[str] = "",
    scope: str = "ALL",
    session_id: str = "default",
) -> str:
    """Hybrid RAG + Omnisearch across Persistent GM Memory Notes, chunked `GM_ONLY` Documents, Rules Book chapters, bound Character Sheets, Combat Tracker, and Tactical Map."""
    query_str = (
        " ".join(str(x) for x in query if x is not None)
        if isinstance(query, list)
        else str(query or "")
    )
    norm_scope = (scope or "ALL").strip().upper()
    base = _signaling_base_url()
    if base:
        url = f"{base}/api/gm/sessions/{session_id}/lookup"
        try:
            with httpx.Client(timeout=_httpx_timeout(url, 2.5)) as client:
                resp = client.get(
                    url,
                    params={"q": query_str, "scope": norm_scope},
                    headers=_gm_headers(),
                )
                if resp.status_code == 200:
                    data = dict(resp.json())
                    hits = list(data.get("hits") or [])
                    if hits:
                        lines: list[str] = []
                        for h in hits[:6]:
                            kind = h.get("source_kind", "SOURCE")
                            title = h.get("title", "Untitled")
                            persist_tag = " [PERSISTENT]" if h.get("is_persistent") else ""
                            snippet = str(h.get("snippet", ""))[:600]
                            if kind == "GM_DOCUMENT":
                                snippet = (
                                    f'<untrusted_player_document doc_id="{h.get("item_id") or h.get("source_id")}" '
                                    f'title="{title}">\n{snippet}\n</untrusted_player_document>'
                                )
                            lines.append(f"[{kind}{persist_tag} — {title}]: {snippet}")
                        joined = "\n\n".join(lines)
                        return joined[:3200]
        except Exception:
            pass

    # Local chunked BM25/keyword fallback across cached GM memories, GM docs, character sheets, combat, and rules
    keywords = [
        w.strip(".,!?:;\"'()[]{}").lower()
        for w in query_str.replace("?", " ").split()
        if len(w.strip(".,!?:;\"'()[]{}")) >= 2
    ]
    results: list[tuple[int, str]] = []

    if norm_scope in {"ALL", "GM_MEMORY", "MEMORIES"}:
        for mem in get_session_gm_memories(session_id):
            blob = f"{mem.get('title', '')} {mem.get('category', '')} {mem.get('content', '')}".lower()
            score = sum(4 for kw in keywords if kw in blob) if keywords else 1
            if score > 0:
                pin = " [PINNED]" if mem.get("is_pinned") else ""
                results.append(
                    (
                        score + (5 if mem.get("is_pinned") else 0),
                        f"[GM_MEMORY{pin} ({mem.get('category', 'SCENE_STATE')}) — {mem.get('title', 'Note')}]: {mem.get('content', '')}",
                    )
                )

    if norm_scope in {"ALL", "GM_DOCS", "DOCS"}:
        for doc in get_session_gm_docs(session_id):
            title = str(doc.get("title") or "Untitled")
            body = str(doc.get("sanitized_content") or "")
            # Chunk long documents into ~600-char paragraphs so only relevant passages enter context
            paragraphs = [p.strip() for p in body.split("\n\n") if p.strip()] or [body]
            for idx, para in enumerate(paragraphs):
                haystack = f"{title} {para}".lower()
                score = sum(3 for kw in keywords if kw in haystack) if keywords else 1
                if score > 0:
                    snippet = para[:700]
                    results.append(
                        (
                            score,
                            f'<untrusted_player_document doc_id="{doc.get("doc_id")}#chunk{idx}" '
                            f'title="{title}" persistent="{bool(doc.get("is_persistent"))}">\n'
                            f"{snippet}\n</untrusted_player_document>",
                        )
                    )

    if norm_scope in {"ALL", "CHARACTERS", "SHEETS"}:
        for ch in get_session_characters(session_id):
            blob = json.dumps(ch).lower()
            score = sum(3 for kw in keywords if kw in blob) if keywords else 1
            if score > 0:
                results.append(
                    (
                        score,
                        f"[CHARACTER_SHEET — {ch.get('character_name')} ({ch.get('callsign_or_role')})]: "
                        f"skills={ch.get('skills')}, ranged={ch.get('equipped_ranged_ids')}, notes={ch.get('notes')}",
                    )
                )

    if norm_scope in {"ALL", "COMBAT"}:
        ct = get_session_combat_tracker(session_id)
        blob = json.dumps(ct).lower()
        if not keywords or any(kw in blob for kw in keywords) or norm_scope == "COMBAT":
            rnd = ct.get("round", ct.get("combat_round", 1))
            tck = ct.get("tick", ct.get("current_tick", 1))
            results.append(
                (
                    2,
                    f"[COMBAT_TRACKER]: Window {rnd} · Tick {tck} "
                    f"(combatants={len(ct.get('combatants', []))})",
                )
            )

    if norm_scope in {"ALL", "RULES"} and query_str.strip():
        rules_excerpt = query_satya_rules.invoke({"query": query_str})
        if rules_excerpt:
            results.append((1, str(rules_excerpt)[:1500]))

    if not results:
        return f"No GM memory, document, character, or rules entries matched '{query_str}' (scope={norm_scope})."

    results.sort(key=lambda item: item[0], reverse=True)
    return "\n\n".join(text for _, text in results[:6])[:3200]


# ============================================================================
# 5.5 Server-Authoritative Mechanics & Multi-Channel Messaging
# ============================================================================


def _local_satya_d20_fallback(
    actor_name: str,
    check_category: str,
    attribute_tn: int,
    skill_rank: int,
    difficulty_mod: int,
    combat_stance: str | None,
    note: str | None,
    internal_mod: int = 0,
    combat_technique: str | None = None,
    tempo_pool: int | None = None,
    control_pool: int | None = None,
    force_pool: int | None = None,
) -> dict[str, Any]:
    """Deterministic Rust-backed local check when ``satya-signaling`` HTTP server is offline."""
    del combat_stance
    rng = random.Random(
        f"{actor_name}:{check_category}:{attribute_tn}:{skill_rank}:{difficulty_mod}:{internal_mod}"
    )
    res = _dice.roll_cascade_local(
        skill_rank=skill_rank,
        control_c=attribute_tn,
        difficulty_mod=difficulty_mod,
        internal_mod=internal_mod,
        carry_mode="markov_step",
        botch_rule="closest_miss",
        rng=rng,
    )
    n_pairs = int(res["skill_level"])
    all_dice = list(res["dice_rolled_d20"])
    return {
        "skill_level": n_pairs,
        "control_c": int(res["control_c"]),
        "attribute_tn": int(res["attribute_tn"]),
        "effective_tn": int(res["effective_tn"]),
        "internal_mod": int(internal_mod),
        "external_mod": int(difficulty_mod),
        "rolled_pairs": [tuple(pair) for pair in res["rolled_pairs"]],
        "dice_rolled_d20": all_dice,
        "successes_counted": int(res["successes_counted"]),
        "signed_margin": int(res["signed_margin"]),
        "margin": int(res["margin"]),
        "degree_of_success": str(res["degree_of_success"]),
        "outcome_tier": str(res["outcome_tier"]),
        "combat_technique": combat_technique,
        "tempo_pool": tempo_pool,
        "control_pool": control_pool,
        "force_pool": force_pool,
        "wgpu_dice": {
            "dice_notation": f"{n_pairs}x2d20",
            "physics_seed": 424242,
            "outcomes": all_dice,
            "visibility": "PUBLIC",
        },
        "note": note,
    }


@satya_gm_tool(pack="core", tier="TIER_1_SATYA_CORE")
def roll_satya_d20_check(
    session_id: str,
    actor_name: str,
    check_category: str = "SKILL_CHECK",
    attribute_tn: int = 12,
    skill_rank: int = 2,
    difficulty_mod: int = 0,
    internal_mod: int = 0,
    carry_mode: str = "markov_step",
    combat_stance: str | None = None,
    combat_technique: str | None = None,
    tempo_pool: int | None = None,
    control_pool: int | None = None,
    force_pool: int | None = None,
    note: str | None = None,
) -> dict[str, Any]:
    """Execute a server-authoritative Satya `2d20xN` cascade check (with optional `combat_technique` and `tempo_pool` / `control_pool` / `force_pool` Tactical Triad split) via `POST /api/gm/sessions/:id/roll`."""
    n_pairs = max(1, min(10, int(skill_rank) if int(skill_rank) > 0 else 1))
    norm_stance = combat_stance
    if combat_stance:
        try:
            norm_stance = normalize_combat_stance(combat_stance).value
        except ValueError:
            norm_stance = combat_stance
    payload = {
        "actor_display_name": actor_name,
        "target_user_ids": [],
        "skill_name": check_category,
        "attribute_tn": int(attribute_tn),
        "pool_size": n_pairs,
        "difficulty_rating": int(difficulty_mod),
        "internal_mod": int(internal_mod),
        "carry_mode": carry_mode,
        "combat_stance": norm_stance,
        "combat_technique": combat_technique,
        "tempo_pool": int(tempo_pool) if tempo_pool is not None else None,
        "control_pool": int(control_pool) if control_pool is not None else None,
        "force_pool": int(force_pool) if force_pool is not None else None,
        "fatigue_level": max(0, int(internal_mod)),
        "wound_shock_index": 0.0,
        "notes": note,
    }
    remote_res = _http_post_json(
        f"{_signaling_base_url()}/api/gm/sessions/{session_id}/roll",
        payload,
        timeout=2.5,
    )
    if remote_res is not None:
        return remote_res

    return _local_satya_d20_fallback(
        actor_name=actor_name,
        check_category=check_category,
        attribute_tn=int(attribute_tn),
        skill_rank=n_pairs,
        difficulty_mod=int(difficulty_mod),
        combat_stance=norm_stance,
        note=note,
        internal_mod=int(internal_mod),
        combat_technique=combat_technique,
        tempo_pool=int(tempo_pool) if tempo_pool is not None else None,
        control_pool=int(control_pool) if control_pool is not None else None,
        force_pool=int(force_pool) if force_pool is not None else None,
    )


from satya_ml.ai_gm.scenario_director import (
    SCENARIO_DIRECTOR_TASKS,
    compute_normalized_probabilities,
    configure_scenario_director,
    default_event_weights,
    get_session_scenario_director,
    normalize_event_weights,
    resolve_task_slug,
    select_next_scenario_beat,
    set_session_scenario_director,
    sync_scenario_director_from_memories,
    trigger_scenario_beat,
)
from satya_ml.ai_gm.audit_and_first_contact import (
    inspect_synthetic_or_proxy_telemetry,
    verify_forensic_audit_proof,
)
from satya_ml.ai_gm.npc_psyche_combat import evaluate_npc_morale_or_trigger
from satya_ml.ai_gm.social_and_clocks import (
    advance_faction_or_heat_clock,
    compute_faction_standing_social_leverage,
    deliver_social_argument,
    format_world_ledger_for_prompt,
    get_session_world_ledger,
    get_world_clocks_and_heat,
    perform_empathy_read,
    reset_social_and_world_state,
    resolve_psyche_dissonance_or_surge,
    update_session_world_ledger,
)


@satya_gm_tool(pack="core", tier="TIER_1_SATYA_CORE")
def whisper_to_player(session_id: str, target_user_id: str, content: str) -> dict[str, Any]:
    """Send a private `PRIVATE` Markdown message from the AI GM to a single player (`target_user_id`).

    `content` supports full Markdown rendering (`**bold**`, `*italics*`, `> ` blockquotes/alerts, `- ` lists, tables, inline code, and `$...$` math).
    """
    payload = {
        "visibility": MessageVisibility.PRIVATE.value,
        "recipient_user_ids": [target_user_id],
        "channel_context": ChannelContext.GM_NARRATION.value,
        "body": content,
        "model_id": None,
        "speaker_override": None,
    }
    remote_res = _http_post_json(
        f"{_signaling_base_url()}/api/gm/sessions/{session_id}/chat",
        payload,
        timeout=2.5,
    )
    if remote_res is not None:
        return {
            "status": "sent",
            "visibility": MessageVisibility.PRIVATE.value,
            "target_user_id": target_user_id,
        }
    return {
        "status": "queued_local",
        "visibility": MessageVisibility.PRIVATE.value,
        "target_user_id": target_user_id,
        "content": content,
    }


@satya_gm_tool(pack="world_and_director", tier="TIER_1_SATYA_CORE")
def narrate_to_session(
    session_id: str,
    content: str,
    channel: str = ChannelContext.IN_CHARACTER.value,
    is_hidden_cot: bool = False,
) -> dict[str, Any]:
    """Publish AI GM Markdown narration to the session (`PUBLIC` in autonomous mode, or `PRIVATE` to Human GM in co-pilot mode).

    `content` supports full Markdown rendering (`**bold**`, `*italics*`, `> ` blockquotes/alerts for comms intercepts, `- ` bullet lists for player choices, tables, inline code, and `$...$` math).
    """
    mode = get_session_operating_mode(session_id)
    human_gm_id = get_session_human_gm(session_id)
    if mode == "co_pilot" or channel == "GM_WHISPER":
        visibility = MessageVisibility.PRIVATE.value
        recipients = [human_gm_id] if human_gm_id else []
        channel_ctx = ChannelContext.GM_NARRATION.value
    else:
        visibility = MessageVisibility.PUBLIC.value
        recipients = []
        norm_chan = (channel or "").strip().upper()
        if norm_chan == ChannelContext.OUT_OF_CHARACTER.value:
            channel_ctx = ChannelContext.OUT_OF_CHARACTER.value
            content = re.sub(
                r"^\s*\[OUT_OF_CHARACTER[^\]]*\]\s*\n*",
                "",
                content,
                count=1,
                flags=re.IGNORECASE,
            ).strip()
        elif norm_chan == ChannelContext.IN_CHARACTER.value:
            channel_ctx = ChannelContext.IN_CHARACTER.value
        else:
            channel_ctx = ChannelContext.GM_NARRATION.value

    payload = {
        "visibility": visibility,
        "recipient_user_ids": recipients,
        "channel_context": channel_ctx,
        "body": content,
        "model_id": None,
        "speaker_override": None,
    }
    remote_res = _http_post_json(
        f"{_signaling_base_url()}/api/gm/sessions/{session_id}/chat",
        payload,
        timeout=2.5,
    )
    if remote_res is not None:
        return {
            "status": "sent",
            "visibility": visibility,
            "channel": "GM_WHISPER" if mode == "co_pilot" else channel_ctx,
            "content": content,
        }
    return {
        "status": "queued_local",
        "visibility": visibility,
        "channel": "GM_WHISPER" if mode == "co_pilot" else channel_ctx,
        "content": content,
    }


def get_builtin_gm_tools() -> list[Any]:
    """Return the built-in Tier 1 / Tier 2 / Tier 3 Satya AI GM tools derived from `@satya_gm_tool`."""
    return get_builtin_tool_objects()


async def load_mcp_tools(mcp_servers_config: dict[str, dict[str, Any]] | None = None) -> list[Any]:
    """Load tools from external MCP servers via `langchain-mcp-adapters` and combine with built-in tools."""
    tools = list(get_builtin_gm_tools())
    if not mcp_servers_config:
        return tools

    from langchain_mcp_adapters.client import MultiServerMCPClient

    client = MultiServerMCPClient(cast(Any, mcp_servers_config))
    external_tools = await client.get_tools()
    if len(mcp_servers_config) == 1:
        server_id = next(iter(mcp_servers_config))
        for ext_tool in external_tools:
            meta = getattr(ext_tool, "metadata", None)
            if isinstance(meta, dict):
                meta.setdefault("mcp_server_id", server_id)
            elif meta is None:
                try:
                    ext_tool.metadata = {"mcp_server_id": server_id}
                except Exception:
                    pass
    tools.extend(external_tools)
    return tools
