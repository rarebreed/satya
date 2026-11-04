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
import tomllib
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


def _signaling_base_url() -> str:
    return os.environ.get("SATYA_SIGNALING_URL", "http://127.0.0.1:3000").rstrip("/")


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
    headers = _gm_headers()
    if httpx is not None:
        try:
            with httpx.Client(timeout=timeout) as client:
                resp = client.post(url, headers=headers, json=payload)
                if resp.status_code == 200:
                    data = dict(resp.json())
                    _extract_and_record_event_id(data)
                    return data
        except Exception:
            return None
        return None

    try:
        req = urllib.request.Request(
            url,
            data=json.dumps(payload).encode("utf-8"),
            headers=headers,
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            if resp.status == 200:
                data = dict(json.loads(resp.read().decode("utf-8")))
                _extract_and_record_event_id(data)
                return data
    except Exception:
        return None
    return None


from satya_ml.ai_gm.maps import (
    generate_tactical_map,
    get_session_map,
    inspect_tactical_map,
    move_map_token,
    set_session_map,
)

# In-memory per-session caches for NPC states, operating mode, human GM ID, and GM_ONLY documents
_SESSION_NPCS: dict[str, dict[str, dict[str, Any]]] = {}
_SESSION_MODES: dict[str, str] = {}
_SESSION_HUMAN_GMS: dict[str, str | None] = {}
_SESSION_GM_DOCS: dict[str, list[dict[str, Any]]] = {}

_DEFAULT_STARTER_NPCS: dict[str, dict[str, Any]] = {
    "captain_vance": {
        "npc_id": "captain_vance",
        "name": "Captain Elena Vance",
        "archetype": "Veteran Aethelgard Fleet Commander",
        "faction": "Aethelgard Colonial Navy",
        "attribute_tns": {"STR": 13, "DEX": 12, "INT": 11, "WIL": 10},
        "skills": {"astrogation": 3, "command": 3, "tactics": 2},
        "combat_stance": "DEFENSIVE",
        "fatigue": 1,
        "wound_shock": 0,
        "disposition": "wary",
        "notes": "Commands the orbital relay station over Aethelgard Sector 4.",
        "secrets": [
            "Possesses the encrypted Aethelgard nav-log detailing the Ghost Drift anomaly."
        ],
    },
    "secops_sentry": {
        "npc_id": "secops_sentry",
        "name": "Station SecOps Sentry",
        "archetype": "Armored Bulkhead Patrol Guard",
        "faction": "Sector 4 Security Operations",
        "attribute_tns": {"STR": 11, "DEX": 12, "INT": 14, "WIL": 13},
        "skills": {"small_arms": 2, "close_combat": 2, "surveillance": 1},
        "combat_stance": "NEUTRAL",
        "fatigue": 0,
        "wound_shock": 0,
        "disposition": "suspicious",
        "notes": "Patrols the Sector 4 Docking Bay airlocks.",
        "secrets": ["Keycard overrides the West Bulkhead containment lock."],
    },
    "dr_aris_thorne": {
        "npc_id": "dr_aris_thorne",
        "name": "Dr. Aris Thorne",
        "archetype": "Xenobiology & Telemetry Specialist",
        "faction": "Aethelgard Science Directorate",
        "attribute_tns": {"STR": 15, "DEX": 13, "INT": 10, "WIL": 11},
        "skills": {"xenobiology": 3, "sensor_analysis": 3, "medicine": 2},
        "combat_stance": "DEFENSIVE",
        "fatigue": 2,
        "wound_shock": 0,
        "disposition": "cooperative",
        "notes": "Investigating signal harmonics from the Aethelgard deep-core relay.",
        "secrets": ["Secretly transmitted telemetry samples to an off-world courier."],
    },
}


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


def load_all_npc_profiles(repo_root: Path | None = None) -> dict[str, dict[str, Any]]:
    """Load all NPC definitions from `data/npcs/*.toml`, falling back to built-in Sector 4 starter NPCs."""
    root = repo_root or _repo_root()
    npcs_dir = root / "data" / "npcs"
    profiles: dict[str, dict[str, Any]] = {
        npc_id: {
            **data,
            "attribute_tns": dict(data.get("attribute_tns", {})),
            "skills": dict(data.get("skills", {})),
            "secrets": list(data.get("secrets", [])),
        }
        for npc_id, data in _DEFAULT_STARTER_NPCS.items()
    }
    if not npcs_dir.exists():
        return profiles

    for toml_path in sorted(npcs_dir.glob("*.toml")):
        try:
            raw = tomllib.loads(toml_path.read_text(encoding="utf-8"))
            for key, npc_data in raw.get("npcs", {}).items():
                npc_id = str(npc_data.get("npc_id", key))
                profiles[npc_id] = dict(npc_data)
        except OSError:
            continue
    return profiles


def get_session_npcs(session_id: str) -> dict[str, dict[str, Any]]:
    """Return the mutable NPC state dictionary for `session_id`, seeding from disk if needed."""
    if session_id not in _SESSION_NPCS:
        _SESSION_NPCS[session_id] = load_all_npc_profiles()
    return _SESSION_NPCS[session_id]


# ============================================================================
# 5.1 NPC Profile & State Management
# ============================================================================


@tool
def lookup_npc_profile(npc_id: str, session_id: str = "default") -> dict[str, Any]:
    """Read an NPC's Satya d20 profile, attribute TNs, skills, combat stance, fatigue, wound shock, and secrets."""
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
    session_id: str = "default",
) -> dict[str, Any]:
    """Update an NPC's fatigue, wound shock, combat stance, or disposition in the active session."""
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
    npcs[target_key] = npc
    return npc


# ============================================================================
# 5.2 Ruleset & Lore Retrieval
# ============================================================================


_CANONICAL_RULES_FILES: tuple[tuple[str, str], ...] = (
    ("02-core-mechanics/cascading_d20_and_tasks.md", "Ch. 3: Cascading 2d20 Engine & Task Resolution"),
    ("02-core-mechanics/action_economy_and_ooda.md", "Ch. 4: Action Economy, OODA Loop, Time Scales & Stealth"),
    ("01-foundation/probability_and_calibration.md", "Ch. 2: Probability & Mathematical Calibration"),
    ("03-character/characteristics_and_aptitudes.md", "Ch. 5: Characteristics, Senses, Psyche & Aptitudes"),
    ("03-character/skills_and_trees.md", "Ch. 6: Skill System & Hierarchical Skill Trees"),
    ("03-character/lifepaths_and_economy.md", "Ch. 7: Lifepath Creation, Socioeconomic Upkeep & Heat"),
    ("04-psyche-and-social/psyche_morale_and_social_combat.md", "Ch. 8: Psyche, Morale & Social Combat"),
    ("05-combat/tactical_triad_and_pools.md", "Ch. 9: Combat Architecture, Tactical Triad & Pools"),
    ("05-combat/melee_combat.md", "Ch. 10: Melee Combat & Close-Quarters Tactics"),
    ("05-combat/ranged_combat_and_armor.md", "Ch. 11: Ranged Combat, Suppression, Armor & Reliability"),
    ("06-health-and-survival/health_and_trauma.md", "Ch. 12: Health, Anatomical Trauma, Fatigue & Mental Conditions"),
    ("06-health-and-survival/movement_vehicles_and_environment.md", "Ch. 13: Movement, Encumbrance, Vehicles & Survival"),
    ("01-foundation/design_philosophy.md", "Ch. 1: Design Philosophy & Goals of Satya"),
)


@tool
def query_satya_rules(query: str) -> str:
    """Search the 13-chapter Satya TTRPG Rules Book (`docs/ttrpg/`) for `2d20` Watershed Cascade mechanics, task resolution, characteristics, skill ontology, tactical combat (`Tempo`/`Control`/`Force`), and social dissonance."""
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
    """Search campaign NPC lore (`data/npcs/*.toml`) and archived PyLance session history (`data/lance/chat-events.lance`)."""
    keywords = [w.lower() for w in query.replace("?", " ").split() if len(w) >= 3]
    matches: list[str] = []

    for npc_id, profile in load_all_npc_profiles().items():
        blob = f"{npc_id} {profile.get('name', '')} {profile.get('archetype', '')} {profile.get('faction', '')} {' '.join(profile.get('secrets', []))} {profile.get('notes', '')}"
        if not keywords or any(kw in blob.lower() for kw in keywords):
            matches.append(
                f"[NPC {profile.get('name', npc_id)} ({profile.get('faction', 'Unknown')})]: "
                f"{profile.get('archetype', '')}. Notes: {profile.get('notes', '')} "
                f"Secrets: {'; '.join(profile.get('secrets', []))}"
            )

    lance_chat = _repo_root() / "data" / "lance" / "chat-events.lance"
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
def read_gm_documents(session_id: str = "default", query: str | None = None) -> str:
    """Read sanitized `GM_ONLY` documents (character dossiers, backstories, Google Docs/Sheets) uploaded to the GM in this session."""
    docs = get_session_gm_docs(session_id)
    if not docs and httpx is not None:
        try:
            with httpx.Client(timeout=2.0) as client:
                resp = client.get(
                    f"{_signaling_base_url()}/api/gm/sessions/{session_id}/context",
                    headers=_gm_headers(),
                )
                if resp.status_code == 200:
                    data = dict(resp.json())
                    docs = list(data.get("gm_documents") or [])
                    set_session_gm_docs(session_id, docs)
        except Exception:
            pass

    if not docs:
        return f"No GM_ONLY documents have been uploaded for session '{session_id}'."

    kw = (query or "").strip().lower()
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
    return "\n\n".join(blocks[:6])


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
    note: str | None = None,
) -> dict[str, Any]:
    """Execute a server-authoritative Satya Two-List Cascading (`2d20`) check via `POST /api/gm/sessions/:id/roll`."""
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
    )


@tool
def whisper_to_player(session_id: str, target_user_id: str, content: str) -> dict[str, Any]:
    """Send a private `PRIVATE` message from the AI GM to a single player (`target_user_id`)."""
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
    """Publish AI GM narration to the session (`PUBLIC` in autonomous mode, or `PRIVATE` to Human GM in co-pilot mode)."""
    mode = get_session_operating_mode(session_id)
    human_gm_id = get_session_human_gm(session_id)
    if mode == "co_pilot" or channel == "GM_WHISPER":
        visibility = "PRIVATE"
        recipients = [human_gm_id] if human_gm_id else []
        channel_ctx = "GM_NARRATION"
    else:
        visibility = "PUBLIC"
        recipients = []
        channel_ctx = "IN_CHARACTER" if channel == "IN_CHARACTER" else "GM_NARRATION"

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
    return [
        lookup_npc_profile,
        update_npc_state,
        query_satya_rules,
        query_campaign_lore,
        read_gm_documents,
        generate_tactical_map,
        move_map_token,
        inspect_tactical_map,
        roll_satya_d20_check,
        whisper_to_player,
        narrate_to_session,
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
