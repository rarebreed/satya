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


def _signaling_base_url() -> str:
    return os.environ.get("SATYA_SIGNALING_URL", "http://127.0.0.1:3000").rstrip("/")


def _gm_service_token() -> str:
    return os.environ.get("SATYA_GM_SERVICE_TOKEN", "satya_gm_secret_dev_token")


def _gm_headers() -> dict[str, str]:
    return {
        "Authorization": f"Bearer {_gm_service_token()}",
        "Content-Type": "application/json",
    }


def _http_post_json(url: str, payload: dict[str, Any], timeout: float = 2.5) -> dict[str, Any] | None:
    """POST JSON to `url` with GM service headers, returning parsed JSON on 200 OK or None on failure."""
    headers = _gm_headers()
    if httpx is not None:
        try:
            with httpx.Client(timeout=timeout) as client:
                resp = client.post(url, headers=headers, json=payload)
                if resp.status_code == 200:
                    return dict(resp.json())
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
                return dict(json.loads(resp.read().decode("utf-8")))
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

# In-memory per-session caches for NPC states, operating mode, and human GM ID
_SESSION_NPCS: dict[str, dict[str, dict[str, Any]]] = {}
_SESSION_MODES: dict[str, str] = {}
_SESSION_HUMAN_GMS: dict[str, str | None] = {}


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


def load_all_npc_profiles(repo_root: Path | None = None) -> dict[str, dict[str, Any]]:
    """Load all NPC definitions from `data/npcs/*.toml`."""
    root = repo_root or _repo_root()
    npcs_dir = root / "data" / "npcs"
    profiles: dict[str, dict[str, Any]] = {}
    if not npcs_dir.exists():
        return profiles

    for toml_path in sorted(npcs_dir.glob("*.toml")):
        raw = tomllib.loads(toml_path.read_text(encoding="utf-8"))
        for key, npc_data in raw.get("npcs", {}).items():
            npc_id = str(npc_data.get("npc_id", key))
            profiles[npc_id] = dict(npc_data)
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


@tool
def query_satya_rules(query: str) -> str:
    """Search `docs/satya_d20_rules.md` for Satya d20 mechanics, dice pools, target numbers, stances, and degrees of success."""
    rules_path = _repo_root() / "docs" / "satya_d20_rules.md"
    if not rules_path.exists():
        return "Satya d20 ruleset document not found."

    text = rules_path.read_text(encoding="utf-8")
    sections = text.split("\n## ")
    keywords = [w.lower() for w in query.replace("?", " ").split() if len(w) >= 3]
    if not keywords:
        return text[:1500]

    scored: list[tuple[int, str]] = []
    for idx, section in enumerate(sections):
        header_and_body = section if idx == 0 else f"## {section}"
        lowered = header_and_body.lower()
        score = sum(lowered.count(kw) for kw in keywords)
        if score > 0:
            scored.append((score, header_and_body.strip()))

    if not scored:
        return text[:1500]

    scored.sort(key=lambda item: item[0], reverse=True)
    top_excerpts = "\n\n---\n\n".join(chunk for _, chunk in scored[:2])
    return top_excerpts[:2400]


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
) -> dict[str, Any]:
    """Deterministic/local fallback for unit tests when satya-signaling HTTP server is not running."""
    rng = random.Random(f"{actor_name}:{check_category}:{attribute_tn}:{skill_rank}:{difficulty_mod}")
    d20_base = rng.randint(1, 20)
    stance_mod = 1 if combat_stance == "AGGRESSIVE" else (-1 if combat_stance == "DEFENSIVE" else 0)
    net_pool = skill_rank + difficulty_mod + stance_mod
    pool_rolls = [rng.randint(1, 20) for _ in range(abs(net_pool))]
    all_dice = [d20_base, *pool_rolls]
    kept_die = min(all_dice) if net_pool >= 0 else max(all_dice)
    effective_tn = max(2, min(19, int(attribute_tn)))
    margin = effective_tn - kept_die
    if kept_die == 1 or margin >= 5:
        degree = "CRITICAL_SUCCESS"
    elif kept_die <= effective_tn:
        degree = "SUCCESS"
    elif kept_die == 20 or margin <= -5:
        degree = "CRITICAL_FAILURE"
    else:
        degree = "FAILURE"
    return {
        "d20_base": d20_base,
        "skill_pool_rolls": pool_rolls,
        "net_pool_size": net_pool,
        "kept_die": kept_die,
        "attribute_tn": attribute_tn,
        "effective_tn": effective_tn,
        "margin": margin,
        "degree_of_success": degree,
        "wgpu_dice": {
            "dice_notation": f"1d20+{abs(net_pool)}d20",
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
    skill_rank: int = 0,
    difficulty_mod: int = 0,
    combat_stance: str | None = None,
    note: str | None = None,
) -> dict[str, Any]:
    """Execute a server-authoritative Satya d20 roll-under check via `POST /api/gm/sessions/:id/roll`."""
    pool_size = max(1, 1 + int(skill_rank))
    difficulty_rating = max(1, 1 - int(difficulty_mod))
    payload = {
        "actor_display_name": actor_name,
        "target_user_ids": [],
        "skill_name": check_category,
        "attribute_tn": int(attribute_tn),
        "pool_size": pool_size,
        "difficulty_rating": difficulty_rating,
        "combat_stance": combat_stance,
        "fatigue_level": 0,
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
        skill_rank=int(skill_rank),
        difficulty_mod=int(difficulty_mod),
        combat_stance=combat_stance,
        note=note,
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
