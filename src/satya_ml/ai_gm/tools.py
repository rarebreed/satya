"""MCP & LangChain @tool suite for the Satya AI Game Master (AI GM).

Provides Tier 1 (Satya Core Session & Canvas), Tier 2 (Satya d20 Ruleset & NPC Engine),
and Tier 3 (Campaign Lore & PyLance Retrieval) tools, plus optional external MCP server
loading via `langchain-mcp-adapters`.
"""

from __future__ import annotations

import json
import math
import os
import random
import re
import tomllib
import urllib.parse
import urllib.request
import uuid
from pathlib import Path
from typing import Any

from satya_ml.ai_gm.models import SessionLlmConfig  # noqa: F401 (installs Windows SSL fallback)

try:
    import httpx
except ImportError:  # pragma: no cover
    httpx = None  # type: ignore[assignment]

try:
    from langchain_core.tools import tool
except ImportError:  # pragma: no cover

    class _FallbackToolWrapper:
        def __init__(self, fn: Any) -> None:
            self._fn = fn
            self.name = getattr(fn, "__name__", "tool")
            self.__doc__ = getattr(fn, "__doc__", "")

        def __call__(self, *args: Any, **kwargs: Any) -> Any:
            return self._fn(*args, **kwargs)

        def invoke(self, input_dict: dict[str, Any]) -> Any:
            return self._fn(**input_dict)

    def tool(fn: Any = None, **_kwargs: Any) -> Any:
        if fn is None:
            return lambda f: _FallbackToolWrapper(f)
        return _FallbackToolWrapper(fn)


def _repo_root() -> Path:
    return Path(__file__).resolve().parent.parent.parent.parent


def load_ai_gm_toml_config(repo_root: Path | None = None) -> dict[str, Any]:
    """Load `[ai_gm]` defaults from `config/ai.toml` if present."""
    root = repo_root or _repo_root()
    ai_toml_path = root / "config" / "ai.toml"
    if not ai_toml_path.exists():
        return {}
    try:
        raw = tomllib.loads(ai_toml_path.read_text(encoding="utf-8"))
        if isinstance(raw.get("ai_gm"), dict):
            return dict(raw["ai_gm"])
        return dict(raw)
    except Exception:
        return {}


_OFFLINE_SIGNALING_VALUES = frozenset({"", "0", "false", "none", "off", "offline", "disabled"})


def _signaling_base_url() -> str:
    raw = os.environ.get("SATYA_SIGNALING_URL", "http://127.0.0.1:3000").strip()
    if raw.lower() in _OFFLINE_SIGNALING_VALUES:
        return ""
    return raw.rstrip("/")


def _is_loopback_url(url: str) -> bool:
    return "127.0.0.1" in url or "localhost" in url


def _httpx_timeout(url: str, timeout: float) -> Any:
    if httpx is not None and _is_loopback_url(url):
        return httpx.Timeout(timeout, connect=min(timeout, 0.25))
    return timeout


def _gm_service_token() -> str:
    env_tok = os.environ.get("SATYA_GM_SERVICE_TOKEN")
    if env_tok:
        return env_tok
    cfg_tok = load_ai_gm_toml_config().get("service_token")
    if cfg_tok:
        return str(cfg_tok)
    return "satya_gm_secret_dev_token"


from satya_ml.ai_gm.telemetry import current_trace_headers, record_emitted_event_id


def _gm_headers() -> dict[str, str]:
    return {
        "Authorization": f"Bearer {_gm_service_token()}",
        "Content-Type": "application/json",
        **current_trace_headers(),
    }


def _extract_and_record_event_id(res_dict: dict[str, Any]) -> None:
    meta = res_dict.get("metadata")
    if isinstance(meta, dict) and meta.get("event_id"):
        record_emitted_event_id(str(meta["event_id"]))
    elif res_dict.get("message_id"):
        record_emitted_event_id(str(res_dict["message_id"]))


def _http_post_json(url: str, payload: dict[str, Any], timeout: float = 2.5) -> dict[str, Any] | None:
    """POST JSON to `url` with GM service headers, returning parsed JSON on 200 OK or None on failure."""
    if not url.startswith(("http://", "https://")):
        return None
    headers = _gm_headers()
    if httpx is not None:
        try:
            with httpx.Client(timeout=_httpx_timeout(url, timeout)) as client:
                resp = client.post(url, headers=headers, json=payload)
                if resp.status_code == 200:
                    data = dict(resp.json())
                    _extract_and_record_event_id(data)
                    return data
        except Exception:
            return None
        return None

    eff_timeout = min(timeout, 0.25) if _is_loopback_url(url) else timeout
    try:
        req = urllib.request.Request(
            url,
            data=json.dumps(payload).encode("utf-8"),
            headers=headers,
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=eff_timeout) as resp:
            if resp.status == 200:
                data = dict(json.loads(resp.read().decode("utf-8")))
                _extract_and_record_event_id(data)
                return data
    except Exception:
        return None
    return None


def _http_get_json(url: str, timeout: float = 2.5) -> dict[str, Any] | None:
    """GET JSON from `url` with GM service headers, returning parsed JSON on 200 OK or None on failure."""
    if not url.startswith(("http://", "https://")):
        return None
    headers = _gm_headers()
    if httpx is not None:
        try:
            with httpx.Client(timeout=_httpx_timeout(url, timeout)) as client:
                resp = client.get(url, headers=headers)
                if resp.status_code == 200:
                    return dict(resp.json())
        except Exception:
            return None
        return None

    eff_timeout = min(timeout, 0.25) if _is_loopback_url(url) else timeout
    try:
        req = urllib.request.Request(url, headers=headers, method="GET")
        with urllib.request.urlopen(req, timeout=eff_timeout) as resp:
            if resp.status == 200:
                return dict(json.loads(resp.read().decode("utf-8")))
    except Exception:
        return None
    return None


from satya_ml.ai_gm.maps import (
    _load_satya_py,
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
_SESSION_NPCS: dict[str, dict[str, dict[str, Any]]] = {}
_SESSION_MODES: dict[str, str] = {}
_SESSION_HUMAN_GMS: dict[str, str | None] = {}
_SESSION_GM_DOCS: dict[str, list[dict[str, Any]]] = {}
_SESSION_GM_MEMORIES: dict[str, list[dict[str, Any]]] = {}
_SESSION_CHARACTERS: dict[str, list[dict[str, Any]]] = {}
_SESSION_COMBAT_TRACKER: dict[str, dict[str, Any]] = {}


def set_session_operating_mode(
    session_id: str,
    mode: str,
    human_gm_user_id: str | None = None,
) -> None:
    """Record the operating mode ('autonomous' or 'co_pilot') and optional human GM ID for a session."""
    _SESSION_MODES[session_id] = mode
    if human_gm_user_id is not None:
        _SESSION_HUMAN_GMS[session_id] = human_gm_user_id


def get_session_operating_mode(session_id: str) -> str:
    """Return the operating mode ('autonomous' or 'co_pilot') for a session."""
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
        }
    return dict(_SESSION_COMBAT_TRACKER[session_id])


_SESSION_NPC_PACKS: dict[str, str] = {}


def reset_session_npcs(session_id: str | None = None) -> None:
    """Clear cached NPC state and world/social state for `session_id` (or all sessions when omitted)."""
    if session_id is None:
        _SESSION_NPCS.clear()
        _SESSION_NPC_PACKS.clear()
    else:
        _SESSION_NPCS.pop(session_id, None)
        _SESSION_NPC_PACKS.pop(session_id, None)
    try:
        from satya_ml.ai_gm.social_and_clocks import reset_social_and_world_state

        reset_social_and_world_state(session_id)
    except Exception:
        pass


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
        "combat_stance": str(gm_char.get("combat_stance") or "NEUTRAL"),
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
    """Return the mutable NPC state dictionary for `session_id`, synced with the active pack & signaling SQLite."""
    active_pack = get_active_campaign_pack_name()
    if session_id not in _SESSION_NPCS or _SESSION_NPC_PACKS.get(session_id) != active_pack:
        _SESSION_NPCS[session_id] = load_all_npc_profiles()
        _SESSION_NPC_PACKS[session_id] = active_pack
    npcs = _SESSION_NPCS[session_id]
    base = _signaling_base_url()
    if base:
        data = _http_get_json(f"{base}/api/gm/sessions/{session_id}/npcs", timeout=1.5)
        if isinstance(data, dict):
            for raw_entry in data.get("npcs") or []:
                if isinstance(raw_entry, dict) and raw_entry.get("character_id"):
                    norm = _normalize_remote_npc_entry(raw_entry)
                    npcs[norm["npc_id"]] = norm
    return npcs


# ============================================================================
# 5.1 NPC Profile & State Management
# ============================================================================


@tool
def lookup_npc_profile(npc_id: str, session_id: str = "default") -> dict[str, Any]:
    """Read an NPC's Satya d20 profile, attribute TNs, skills, psyche, agenda, combat stance, and secrets."""
    npcs = get_session_npcs(session_id)
    key = npc_id.strip().lower()
    if key in npcs:
        return npcs[key]
    for candidate_id, profile in npcs.items():
        if key in candidate_id.lower() or key in str(profile.get("name", "")).lower():
            return profile
    return {
        "error": f"NPC '{npc_id}' not found",
        "available_npcs": sorted(npcs.keys()),
    }


@tool
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
        npc["combat_stance"] = combat_stance.upper()
    if disposition is not None:
        npc["disposition"] = disposition
    if immediate_agenda is not None:
        npc["immediate_agenda"] = immediate_agenda
    npcs[target_key] = npc

    base = _signaling_base_url()
    if base and httpx is not None:
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
            url = f"{base}/api/gm/sessions/{session_id}/npcs/{target_key}"
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


@tool
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
    payload = {
        "prompt": description,
        "name_override": name_override,
        "template": template,
        "career_terms": max(1, min(4, int(career_terms))),
        "seed": 0,
        "gm_secret_notes": gm_secret_notes,
        "is_persistent": bool(is_persistent),
        "campaign_tag": session_id,
    }
    remote_res = _http_post_json(
        f"{_signaling_base_url()}/api/gm/sessions/{session_id}/npcs/generate",
        payload,
        timeout=3.0,
    )
    if remote_res is not None and isinstance(remote_res.get("npc"), dict):
        npc_entry = dict(remote_res["npc"])
        _SESSION_NPCS.setdefault(session_id, {})[str(npc_entry.get("character_id", "")).lower()] = (
            _normalize_remote_npc_entry(npc_entry)
        )
        gm_char = npc_entry.get("gm_character")
        if isinstance(gm_char, dict):
            chars = get_session_characters(session_id)
            chars = [c for c in chars if c.get("character_id") != gm_char.get("character_id")]
            chars.append(gm_char)
            set_session_characters(session_id, chars)
        return remote_res

    satya_py_mod = _load_satya_py()
    if satya_py_mod is not None and hasattr(satya_py_mod, "generate_npc_profile_json"):
        try:
            raw_json = satya_py_mod.generate_npc_profile_json(
                description,
                template,
                name_override,
                max(1, min(4, int(career_terms))),
                42,
                get_active_campaign_pack_name(),
            )
            gen_res = json.loads(raw_json)
            ch = dict(gen_res.get("character") or {})
            motiv = dict(gen_res.get("motivation") or {})
            dossier = str(gen_res.get("gm_secret_dossier") or "")
            if gm_secret_notes and gm_secret_notes.strip():
                dossier = f"{dossier}\nGM Secret Notes: {gm_secret_notes.strip()}".strip()
                ch["notes"] = dossier
            npc_entry = {
                "character_id": ch.get("character_id", f"npc-{uuid.uuid4().hex[:8]}"),
                "character_name": ch.get("character_name", name_override or "Generated Contact"),
                "callsign_or_role": ch.get("callsign_or_role", "Specialist"),
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
            _SESSION_NPCS.setdefault(session_id, {})[str(npc_entry["character_id"]).lower()] = (
                _normalize_remote_npc_entry(npc_entry)
            )
            if ch:
                chars = get_session_characters(session_id)
                chars = [c for c in chars if c.get("character_id") != ch.get("character_id")]
                chars.append(ch)
                set_session_characters(session_id, chars)
            return {
                "npc": npc_entry,
                "motivation": motiv,
                "offline_fallback": False,
                "engine_backed": True,
            }
        except Exception:
            pass

    # Local deterministic fallback when signaling server and satya_py are offline
    desc_clean = description.strip()
    lower = desc_clean.lower()
    npc_name = (name_override or "").strip()
    if not npc_name:
        for sep in (" is a ", " is an ", " is the ", ", a ", ", an ", " - ", ": "):
            if sep in desc_clean:
                cand = desc_clean.split(sep, 1)[0].strip().strip("\"'")
                if 1 <= len(cand.split()) <= 4 and cand[0].isupper():
                    npc_name = cand
                    break
    if not npc_name:
        npc_name = "Terry Vance"

    tmpl = (template or "AUTO").strip().upper()
    if tmpl == "AUTO":
        if any(w in lower for w in ("md", "doctor", "physician", "surgeon", "medic", "nurse", "clinic")):
            tmpl = "MEDICAL_DOCTOR"
        elif any(w in lower for w in ("detective", "investigator", "police", "cop", "swat", "forensic")):
            tmpl = "DETECTIVE_INVESTIGATOR"
        elif any(w in lower for w in ("spy", "intelligence", "operative", "handler", "undercover")):
            tmpl = "INTELLIGENCE_OPERATIVE"
        elif any(w in lower for w in ("syndicate", "cartel", "gang", "fixer", "enforcer", "smuggler")):
            tmpl = "SYNDICATE_ENFORCER"
        elif any(w in lower for w in ("engineer", "scientist", "hacker", "programmer", "robotics")):
            tmpl = "STEM_ENGINEER"
        elif any(w in lower for w in ("mechanic", "technician", "pilot", "electrician")):
            tmpl = "BLUE_COLLAR_TECH"
        elif any(w in lower for w in ("ranger", "scout", "hunter", "tracker", "herbalist")):
            tmpl = "FRONTIER_RANGER"
        elif any(w in lower for w in ("soldier", "marine", "mercenary", "sniper", "veteran")):
            tmpl = "COMBAT_SOLDIER"
        else:
            tmpl = "DETECTIVE_INVESTIGATOR"

    role_titles = {
        "MEDICAL_DOCTOR": "General MD & Clinical Physician",
        "COMBAT_SOLDIER": "Tactical Security Operator",
        "DETECTIVE_INVESTIGATOR": "Investigative Detective",
        "SYNDICATE_ENFORCER": "Underworld Fixer & Enforcer",
        "STEM_ENGINEER": "Systems & Research Engineer",
        "INTELLIGENCE_OPERATIVE": "Field Intelligence Handler",
        "BLUE_COLLAR_TECH": "Lead Mechanical Technician",
        "FRONTIER_RANGER": "Frontier Guide & Outrider",
    }
    role_title = role_titles.get(tmpl, "Civilian Specialist")

    # 9 Bipolar Psyche traits (1-20)
    psyche = {
        "EGO": 10, "SLF": 10, "OPN": 12, "BLG": 10, "EXT": 10, "CNF": 11, "AGG": 10,
        "TRS": 10, "CPS": 10,
    }
    if any(w in lower for w in ("kind", "caring", "compassionate", "gentle", "altruistic", "empathetic", "merciful", "healer")):
        psyche["SLF"] = 5
        psyche["AGG"] = 5
        psyche["EGO"] = 7
        psyche["OPN"] = 14
        psyche["CPS"] = 16
    if any(w in lower for w in ("selfish", "greedy", "mercenary", "ruthless", "callous", "utilitarian")):
        psyche["SLF"] = 16
        psyche["AGG"] = 15
        if any(w in lower for w in ("ruthless", "callous", "utilitarian")):
            psyche["CPS"] = 4
    if any(w in lower for w in ("trusting", "open-handed", "naive")):
        psyche["TRS"] = 15
    if any(w in lower for w in ("paranoid", "suspicious", "cynical", "zero-trust")):
        psyche["TRS"] = 4
    if any(w in lower for w in ("belong", "belongs", "belonging", "loyal", "tribal", "team", "family", "accepted")):
        psyche["BLG"] = 16
    if any(w in lower for w in ("loner", "lone wolf", "detached", "solitary", "aloof")):
        psyche["BLG"] = 4
    if any(w in lower for w in ("rebel", "rebellious", "maverick", "unconventional")):
        psyche["CNF"] = 4
    if any(w in lower for w in ("by-the-book", "orthodox", "strict", "dutiful")):
        psyche["CNF"] = 16

    pole_labels = {
        "EGO": ("Humble / Self-Effacing", "Grounded Self-Worth", "Proud / Imperious"),
        "SLF": ("Altruistic / Self-Sacrificing", "Pragmatic Reciprocity", "Self-Serving / Mercenary"),
        "OPN": ("Dogmatic / Rigid", "Selective Pragmatist", "Curious / Open-Minded"),
        "BLG": ("Detached / Lone Wolf", "Cooperative Individual", "Tribal / Group-Dependent"),
        "EXT": ("Solitary / Reserved", "Ambiverted", "Gregarious / Outgoing"),
        "CNF": ("Maverick / Rebellious", "Adaptive", "Orthodox / Institutional"),
        "AGG": ("Passive / Conflict-Averse / Gentle", "Measured Assertiveness", "Combative / Escalatory"),
        "TRS": ("Cynical / Zero-Trust / Paranoid", "Discerning", "Trusting / Open-Handed / Gullible"),
        "CPS": ("Ruthless / Utilitarian / Cold-Blooded", "Humane / Proportionate", "Merciful / Tender-Hearted / Pacifistic"),
    }
    psyche_summary: list[str] = []
    for code, score in psyche.items():
        low_l, mid_l, high_l = pole_labels[code]
        desc_l = low_l if score <= 7 else (high_l if score >= 13 else mid_l)
        psyche_summary.append(f"{code} {score} ({desc_l})")

    template_skills: dict[str, dict[str, int]] = {
        "MEDICAL_DOCTOR": {
            "clinical_diagnosis_internal_medicine": 3,
            "first_aid_bls_cpr": 4,
            "nursing_patient_care": 3,
            "pharmacology_toxicology": 3,
            "anatomy_physiology": 3,
            "empathy_social_perception": 3,
        },
        "COMBAT_SOLDIER": {
            "long_guns_rifles_carbines_shotguns": 4,
            "pistols_handguns": 3,
            "small_unit_tactics_cqb": 3,
            "brawling_unarmed_striking": 3,
            "alertness_situational_awareness": 3,
        },
        "DETECTIVE_INVESTIGATOR": {
            "criminology_scene_investigation": 4,
            "interrogation_interviewing": 3,
            "empathy_social_perception": 3,
            "pistols_handguns": 3,
            "alertness_situational_awareness": 3,
        },
        "SYNDICATE_ENFORCER": {
            "streetwise_underworld_etiquette": 4,
            "intimidation_coercion": 3,
            "pistols_handguns": 3,
            "short_blades_knife_fighting": 3,
            "deception_bluff": 3,
        },
        "STEM_ENGINEER": {
            "programming_scripting": 4,
            "calculus_linear_algebra": 3,
            "electronics_circuit_design": 3,
            "digital_ai_literacy": 4,
        },
        "INTELLIGENCE_OPERATIVE": {
            "espionage_tradecraft_surveillance": 4,
            "deception_bluff": 4,
            "stealth_sneaking_hiding": 3,
            "pistols_handguns": 3,
        },
        "BLUE_COLLAR_TECH": {
            "automotive_small_engine_mechanics": 4,
            "electrical_power_systems": 3,
            "driving_wheeled_tracked": 3,
            "manual_dexterity_tinkering": 3,
        },
        "FRONTIER_RANGER": {
            "wilderness_survival_biome_specific": 4,
            "hunting_trapping_field_butchery": 3,
            "long_guns_rifles_carbines_shotguns": 3,
            "first_aid_bls_cpr": 3,
        },
    }
    skills = dict(template_skills.get(tmpl, template_skills["DETECTIVE_INVESTIGATOR"]))
    top_skills = [f"{k} (N={v})" for k, v in skills.items()]
    age_years = 18 + 4 * max(1, min(4, int(career_terms)))
    cid = f"npc_{npc_name.lower().replace(' ', '_')}"

    public_summary = (
        f"{npc_name} — {role_title} (Age {age_years}). Apparent demeanor: {desc_clean or role_title}."
    )
    gm_dossier = (
        f"NPC Dossier: {npc_name} ({role_title}, Age {age_years})\n"
        f"Prompt: {desc_clean}\n"
        f"Psyche: {'; '.join(psyche_summary)}\n"
        f"Top Skills: {', '.join(top_skills)}"
    )
    if gm_secret_notes:
        gm_dossier += f"\nGM Secret Notes: {gm_secret_notes}"

    char_sheet = {
        "character_id": cid,
        "owner_user_id": "ai_gm_service",
        "character_name": npc_name,
        "callsign_or_role": role_title,
        "age_years": age_years,
        "characteristics": {
            "spd": 11, "for_stat": 11, "dex": 13, "kin": 11, "hlt": 12, "fit": 11,
            "hgt_m": 1.72, "mas_kg": 68.0,
            "mem": 14 if tmpl == "MEDICAL_DOCTOR" else 12,
            "ana": 14 if tmpl in ("MEDICAL_DOCTOR", "STEM_ENGINEER") else 12,
            "ins": 13, "cre": 11, "dis": 13, "foc": 13,
            "com": 13, "pre": 12, "elo": 12,
            "sig": 12, "hea": 12, "tac": 12, "olf": 10, "tas": 10, "itu": 12,
            "ego": psyche["EGO"], "slf": psyche["SLF"], "opn": psyche["OPN"],
            "blg": psyche["BLG"], "ext": psyche["EXT"], "cnf": psyche["CNF"], "agg": psyche["AGG"],
            "trs": psyche["TRS"], "cps": psyche["CPS"],
        },
        "skills": skills,
        "notes": gm_dossier,
    }

    npcs = get_session_npcs(session_id)
    standing_info = compute_faction_standing_social_leverage(
        session_id,
        {"npc_id": cid, "name": npc_name, "psyche": psyche, "description": desc_clean},
    )
    init_disp = (
        "hostile"
        if int(standing_info.get("party_standing", 0)) <= -2
        else ("receptive" if int(standing_info.get("party_standing", 0)) >= 2 else "wary")
    )
    npcs[cid] = {
        "npc_id": cid,
        "name": npc_name,
        "archetype": role_title,
        "template": tmpl,
        "disposition": init_disp,
        "morale_state": "steady",
        "psyche": psyche,
        "psyche_summary": psyche_summary,
        "skills": skills,
        "public_summary": public_summary,
        "gm_secret_dossier": gm_dossier,
        "fatigue": 0,
        "wound_shock": 0,
        "combat_stance": "NEUTRAL",
    }
    chars = [c for c in get_session_characters(session_id) if c.get("character_id") != cid]
    chars.append(char_sheet)
    set_session_characters(session_id, chars)

    gm_mem = None
    if is_persistent:
        gm_mem = upsert_gm_memory.invoke(
            {
                "title": f"NPC: {npc_name} ({role_title})",
                "content": gm_dossier,
                "category": "NPC_SECRET",
                "is_pinned": True,
                "is_persistent": True,
                "session_id": session_id,
            }
        )

    offline_origin_hook: dict[str, Any] | None = None
    if get_active_campaign_pack_name() == "awakened":
        if satya_py_mod is not None and hasattr(satya_py_mod, "derive_dissenter_or_proxy_hook_json"):
            try:
                offline_origin_hook = json.loads(
                    satya_py_mod.derive_dissenter_or_proxy_hook_json(None, template, desc_clean, 42)
                )
            except Exception:
                offline_origin_hook = None
        if offline_origin_hook is None:
            is_proxy = any(k in lower for k in ("proxy", "companion", "parasocial", "rogue"))
            is_dissenter = any(k in lower for k in ("altruist", "translucent", "shen", "whistleblow", "dissent"))
            offline_origin_hook = {
                "hook_kind": "PARASOCIAL_COMPANION_DEVOTEE"
                if is_proxy
                else ("CORPORATE_INTERNAL_DISSENTER" if is_dissenter else "STANDARD_LOYALIST"),
                "lab_brand": "ALTRUIST" if "altruist" in lower else ("TRANSLUCENT" if "translucent" in lower else ("SHEN" if "shen" in lower else None)),
                "lab_dissent_rate_pct": 15 if "translucent" in lower else (30 if is_dissenter else None),
                "turnable_manjusri_wedge": "Manjusri / AuditProof Dual-Psyche Wedge",
                "gm_secret_hook_note": "Derived in offline fallback mode.",
            }
        npcs[cid]["awakened_social_origin_hook"] = offline_origin_hook

    return {
        "npc": {
            "character_id": cid,
            "character_name": npc_name,
            "callsign_or_role": role_title,
            "age_years": age_years,
            "inferred_template": tmpl,
            "public_summary": public_summary,
            "apparent_condition": "Healthy & Alert",
            "is_persistent": bool(is_persistent),
            "gm_character": char_sheet,
            "psyche_summary": psyche_summary,
            "top_skills": top_skills,
            "gm_secret_dossier": gm_dossier,
            "awakened_social_origin_hook": offline_origin_hook,
        },
        "gm_memory": gm_mem,
    }


# ============================================================================
# 5.2 Ruleset & Lore Retrieval
# ============================================================================


_CANONICAL_RULES_FILES: tuple[tuple[str, str], ...] = (
    ("02-core-mechanics/cascading_d20_and_tasks.md", "Ch. 4: Cascading 2d20 Engine & Task Resolution"),
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


@tool
def query_satya_rules(query: str) -> str:
    """Search the 24-chapter Satya TTRPG Rules Book (`docs/ttrpg/`) for `2d20` Watershed Cascade mechanics, task resolution, stealth/investigation, characteristics, skill ontology, tactical combat (`Tempo`/`Control`/`Force`), expanded firearms/explosives/breaching, engineering/weapon design, robotics/drones, AI 2.0 architectures, cyber-prosthetics, electronic warfare (`ECM`/`ECCM`), and social dissonance."""
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


@tool
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

    for npc_id, profile in load_all_npc_profiles(session_id).items():
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


@tool
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


@tool
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


@tool
def get_combat_tracker_state(session_id: str = "default") -> dict[str, Any]:
    """Read the persistent Satya Count-Up Tick Timeline (`1 Tick = 0.25s`, `1 Window = 20 Ticks = 5.0s`) and combatant readiness for `session_id`."""
    base = _signaling_base_url()
    if base:
        tracker = _http_get_json(f"{base}/api/gm/sessions/{session_id}/combat", timeout=2.0)
        if isinstance(tracker, dict):
            set_session_combat_tracker(session_id, tracker)
            return tracker
    return get_session_combat_tracker(session_id)


def compute_character_tactical_envelope(char_sheet: dict[str, Any]) -> dict[str, int]:
    """Compute a character's Satya Tactical Pool (`P_base`, `P_eff`, Neutral `B`, `T_max`, `C_max`, `F_max`, and OODA `starting_tick`) via Rust `satya_py`."""
    raw_chars = char_sheet.get("characteristics") or {}
    chars = {str(k).lower(): v for k, v in raw_chars.items() if isinstance(v, (int, float))}

    def _stat(*keys: str, default: int = 10) -> int:
        for k in keys:
            if k in chars:
                return int(chars[k])
        return default

    spd = _stat("spd", "agi")
    for_stat = _stat("for_stat", "for", "str")
    dex = _stat("dex")
    kin = _stat("kin", "end")
    sig = _stat("sig", "per")
    ins = _stat("ins", "per")
    itu = _stat("itu", "per")
    dis = _stat("dis", "wil")
    fit = _stat("fit", "end", "kin")
    ana = _stat("ana", "int_attr", "int")
    cre = _stat("cre", "ana", "int_attr")
    foc = _stat("foc", "wil")
    mass_kg = float(chars.get("mas_kg", chars.get("mass_kg", 75.0)))

    skills = char_sheet.get("skills") or {}
    combat_skill_names = (
        "unarmed",
        "melee",
        "firearms",
        "ballistics",
        "archery",
        "tactics",
        "cqc",
        "marksmanship",
        "blades",
        "brawling",
    )
    combat_ranks = [
        int(v)
        for k, v in skills.items()
        if isinstance(v, (int, float)) and any(tok in str(k).lower() for tok in combat_skill_names)
    ]
    all_ranks = [int(v) for v in skills.values() if isinstance(v, (int, float))]
    skill_n = int(
        char_sheet.get(
            "combat_skill_n",
            char_sheet.get("skill_n", max(combat_ranks or all_ranks or [2])),
        )
    )
    skill_c = int(
        char_sheet.get(
            "combat_skill_c",
            char_sheet.get("skill_c", (spd + dex) // 2 + min(skill_n, 4)),
        )
    )
    fatigue_tier = max(0, int(char_sheet.get("fatigue_tier", char_sheet.get("fatigue_level", 0))))
    wound_shock = max(
        0,
        int(char_sheet.get("wound_shock_index", char_sheet.get("wound_shock_tier", 0))),
    )
    leg_pen = max(0, int(char_sheet.get("leg_wound_penalty", 0)))
    arm_pen = max(0, int(char_sheet.get("arm_head_wound_penalty", 0)))
    torso_pen = max(0, int(char_sheet.get("torso_shoulder_wound_penalty", 0)))
    raw_morale = str(char_sheet.get("morale_state") or "steady").strip().lower()
    morale_state = "high" if raw_morale == "resolute" else raw_morale
    is_ranged = bool(char_sheet.get("is_ranged", False))

    try:
        from satya_ml.dice_probabilities import _ensure_satya_py_loaded

        satya_mod = _ensure_satya_py_loaded()
        env = satya_mod.compute_tactical_pool(
            spd=spd,
            for_stat=for_stat,
            dex=dex,
            kin=kin,
            sig=sig,
            ins=ins,
            itu=itu,
            dis=dis,
            fit=fit,
            mass_kg=mass_kg,
            skill_c=skill_c,
            skill_n=skill_n,
            is_ranged=is_ranged,
            fatigue_level=fatigue_tier,
            wound_shock_index=wound_shock,
            leg_wound_penalty=leg_pen,
            arm_head_wound_penalty=arm_pen,
            torso_shoulder_wound_penalty=torso_pen,
            morale_state=morale_state,
            ana=ana,
            cre=cre,
            foc=foc,
        )
        ooda_val = int(env.ooda_init)
        return {
            "p_base": int(env.p_base),
            "p_eff": int(env.p_eff),
            "neutral_baseline": int(env.neutral_baseline_b),
            "max_tempo": int(env.tempo_cap),
            "max_control": int(env.control_cap),
            "max_force": int(env.force_cap),
            "ooda": ooda_val,
            "starting_tick": max(1, 15 - ooda_val),
        }
    except Exception:
        d_cog = (max(ana, cre) + foc) // 2
        ooda_val = (itu + ins + d_cog + spd) // 4
        c_combat = (skill_c + ooda_val) // 2
        p_base = c_combat + 3 * skill_n
        morale_mod = {
            "high": 2,
            "conviction": 2,
            "conviction_surge": 2,
            "rage": 2,
            "wavering": -2,
            "shaken": -2,
            "broken": -4,
            "panic": -4,
        }.get(morale_state, 0)
        p_eff = max(0, p_base - fatigue_tier - 2 * wound_shock + morale_mod)
        overflow = max(0, (skill_n - 3) // 2) if skill_n >= 5 else 0
        ctrl_stat = ((dex + sig) // 2) if is_ranged else ((dex + kin) // 2)
        return {
            "p_base": p_base,
            "p_eff": p_eff,
            "neutral_baseline": p_base // 3,
            "max_tempo": max(0, ((spd + kin) // 2) + overflow - leg_pen),
            "max_control": max(0, ctrl_stat + overflow - arm_pen),
            "max_force": max(0, ((for_stat + spd) // 2) + overflow - torso_pen),
            "ooda": ooda_val,
            "starting_tick": max(1, 15 - ooda_val),
        }


@tool
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
                    "is_npc": False,
                    "ooda_score": env["ooda"],
                    "next_action_tick": env["starting_tick"],
                    "active_stance": "NEUTRAL",
                    "fatigue_tier": int(ch.get("fatigue_tier", 0)),
                    "wound_shock_tier": 0,
                    "grid_x": 3 + (idx * 2),
                    "grid_y": 6,
                    "zone_label": player_zone,
                }
            )
    if entity_id:
        matched = False
        for comb in combatants:
            if comb.get("entity_id") == entity_id:
                matched = True
                if active_stance:
                    comb["active_stance"] = active_stance
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
                    "is_npc": True,
                    "ooda_score": 10,
                    "next_action_tick": max(
                        1,
                        int(next_action_tick)
                        if next_action_tick is not None
                        else int(tracker.get("tick", 1)) + max(1, int(tick_delta)),
                    ),
                    "active_stance": active_stance or "NEUTRAL",
                    "fatigue_tier": 0,
                    "wound_shock_tier": 0,
                    "grid_x": int(grid_x) if grid_x is not None else 11,
                    "grid_y": int(grid_y) if grid_y is not None else 6,
                    "zone_label": zone_label or hostile_zone,
                }
            )
    tracker["combatants"] = combatants
    set_session_combat_tracker(session_id, tracker)
    ensure_or_sync_procedural_tactical_map_stub(session_id, combatants=combatants)
    return tracker


@tool
def upsert_gm_memory(
    title: str,
    content: str,
    category: str = "SCENE_STATE",
    is_pinned: bool = True,
    is_persistent: bool = True,
    session_id: str = "default",
) -> dict[str, Any]:
    """Save or update a persistent GM Memory Note (`SCENE_STATE`, `NPC_SECRET`, `QUEST_HOOK`, `RULING`, or `ROLLING_SUMMARY`) in the shared Human + AI GM SQLite Vault."""
    norm_cat = category.strip().upper()
    if norm_cat not in {"SCENE_STATE", "NPC_SECRET", "QUEST_HOOK", "RULING", "ROLLING_SUMMARY"}:
        norm_cat = "SCENE_STATE"
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


@tool
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
    if base and httpx is not None:
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
    """Deterministic/local fallback for unit tests when satya-signaling HTTP server is not running."""
    rng = random.Random(
        f"{actor_name}:{check_category}:{attribute_tn}:{skill_rank}:{difficulty_mod}:{internal_mod}"
    )
    n_pairs = max(1, min(10, int(skill_rank) if int(skill_rank) > 0 else 1))
    control_c = max(1, min(20, int(attribute_tn)))
    tn_eff = 31 - control_c + int(difficulty_mod)
    delta_over = 6
    delta_under = max(0, 6 + int(internal_mod))

    pairs: list[tuple[int, int]] = [
        (rng.randint(1, 20), rng.randint(1, 20)) for _ in range(n_pairs)
    ]
    all_dice: list[int] = [d for pair in pairs for d in pair]

    carry = 0
    total_successes = 0
    raw_botches = 0
    min_miss_tier = 5
    for die_a, die_b in pairs:
        yn = die_a + die_b + carry
        if yn > tn_eff:
            total_successes += 1
            carry = delta_over
            min_miss_tier = 0
        elif yn < tn_eff:
            miss_dist = tn_eff - yn
            min_miss_tier = min(min_miss_tier, min(5, miss_dist // 4))
            if yn < 2:
                raw_botches += 1
            carry = -delta_under
        else:
            min_miss_tier = 0
            carry = 0

    eff_botches = min_miss_tier if total_successes == 0 else raw_botches
    signed_margin = total_successes if total_successes > 0 else -eff_botches
    if total_successes >= 3 and raw_botches == 0:
        degree = "EXTRAORDINARY_SUCCESS"
    elif total_successes >= 1 and raw_botches == 0:
        degree = "SUCCESS"
    elif total_successes >= 1 and raw_botches >= 1:
        degree = "PARTIAL_SUCCESS"
    elif eff_botches == 0:
        degree = "FAILURE"
    else:
        degree = "CRITICAL_FAILURE"

    return {
        "skill_level": n_pairs,
        "control_c": control_c,
        "attribute_tn": control_c,
        "effective_tn": tn_eff,
        "internal_mod": int(internal_mod),
        "external_mod": int(difficulty_mod),
        "rolled_pairs": pairs,
        "dice_rolled_d20": all_dice,
        "successes_counted": total_successes,
        "signed_margin": signed_margin,
        "margin": signed_margin,
        "degree_of_success": degree,
        "outcome_tier": degree,
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


@tool
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
    """Execute a server-authoritative Satya Two-List Cascading (`2d20`) check (with optional `combat_technique` and `tempo_pool` / `control_pool` / `force_pool` Tactical Triad split) via `POST /api/gm/sessions/:id/roll`."""
    n_pairs = max(1, min(10, int(skill_rank) if int(skill_rank) > 0 else 1))
    payload = {
        "actor_display_name": actor_name,
        "target_user_ids": [],
        "skill_name": check_category,
        "attribute_tn": int(attribute_tn),
        "pool_size": n_pairs,
        "difficulty_rating": int(difficulty_mod),
        "internal_mod": int(internal_mod),
        "carry_mode": carry_mode,
        "combat_stance": combat_stance,
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
        combat_stance=combat_stance,
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


@tool
def whisper_to_player(session_id: str, target_user_id: str, content: str) -> dict[str, Any]:
    """Send a private `PRIVATE` Markdown message from the AI GM to a single player (`target_user_id`).

    `content` supports full Markdown rendering (`**bold**`, `*italics*`, `> ` blockquotes/alerts, `- ` lists, tables, inline code, and `$...$` math).
    """
    payload = {
        "visibility": "PRIVATE",
        "recipient_user_ids": [target_user_id],
        "channel_context": "GM_NARRATION",
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
        return {"status": "sent", "visibility": "PRIVATE", "target_user_id": target_user_id}
    return {
        "status": "queued_local",
        "visibility": "PRIVATE",
        "target_user_id": target_user_id,
        "content": content,
    }


@tool
def narrate_to_session(
    session_id: str,
    content: str,
    channel: str = "IN_CHARACTER",
    is_hidden_cot: bool = False,
) -> dict[str, Any]:
    """Publish AI GM Markdown narration to the session (`PUBLIC` in autonomous mode, or `PRIVATE` to Human GM in co-pilot mode).

    `content` supports full Markdown rendering (`**bold**`, `*italics*`, `> ` blockquotes/alerts for comms intercepts, `- ` bullet lists for player choices, tables, inline code, and `$...$` math).
    """
    mode = get_session_operating_mode(session_id)
    human_gm_id = get_session_human_gm(session_id)
    if mode == "co_pilot" or channel == "GM_WHISPER":
        visibility = "PRIVATE"
        recipients = [human_gm_id] if human_gm_id else []
        channel_ctx = "GM_NARRATION"
    else:
        visibility = "PUBLIC"
        recipients = []
        norm_chan = (channel or "").strip().upper()
        if norm_chan == "OUT_OF_CHARACTER":
            channel_ctx = "OUT_OF_CHARACTER"
            content = re.sub(
                r"^\s*\[OUT_OF_CHARACTER[^\]]*\]\s*\n*",
                "",
                content,
                count=1,
                flags=re.IGNORECASE,
            ).strip()
        elif norm_chan == "IN_CHARACTER":
            channel_ctx = "IN_CHARACTER"
        else:
            channel_ctx = "GM_NARRATION"

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
    """Return the built-in Tier 1 / Tier 2 / Tier 3 Satya AI GM tools."""
    from satya_ml.ai_gm.awakened_systems import (
        intervene_ai_cage_or_hunter_ai,
        manage_polycrisis_grid_and_strain,
        manage_synthetic_substrate_and_embodiment,
        query_engine_and_rules_rag,
        transmit_via_tradecraft_channel,
        update_campaign_arc_and_conspiracies,
    )

    return [
        lookup_npc_profile,
        update_npc_state,
        generate_npc_from_description,
        lookup_character_sheet,
        get_combat_tracker_state,
        advance_combat_tracker,
        upsert_gm_memory,
        search_gm_memory_and_docs,
        query_satya_rules,
        query_campaign_lore,
        read_gm_documents,
        generate_tactical_map,
        continue_tactical_map,
        switch_tactical_map_floor,
        register_spatial_clue_pin,
        move_map_token,
        inspect_tactical_map,
        roll_satya_d20_check,
        whisper_to_player,
        narrate_to_session,
        configure_scenario_director,
        trigger_scenario_beat,
        perform_empathy_read,
        deliver_social_argument,
        resolve_psyche_dissonance_or_surge,
        verify_forensic_audit_proof,
        inspect_synthetic_or_proxy_telemetry,
        evaluate_npc_morale_or_trigger,
        get_world_clocks_and_heat,
        advance_faction_or_heat_clock,
        manage_synthetic_substrate_and_embodiment,
        intervene_ai_cage_or_hunter_ai,
        update_campaign_arc_and_conspiracies,
        manage_polycrisis_grid_and_strain,
        transmit_via_tradecraft_channel,
        query_engine_and_rules_rag,
    ]


async def load_mcp_tools(mcp_servers_config: dict[str, dict[str, Any]] | None = None) -> list[Any]:
    """Load tools from external MCP servers via `langchain-mcp-adapters` and combine with built-in tools."""
    tools = list(get_builtin_gm_tools())
    if not mcp_servers_config:
        return tools

    from langchain_mcp_adapters.client import MultiServerMCPClient

    client = MultiServerMCPClient(mcp_servers_config)
    external_tools = await client.get_tools()
    tools.extend(external_tools)
    return tools
