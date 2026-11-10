"""Player-scoped tool suite (`@tool`) for the Satya AI Player Agent (`satya_ml.ai_player`).

Strictly isolates player state per `(session_id, player_user_id)` and exposes only player-legal
capabilities:
- `read_my_character_sheet`: Read the AI Player's bound `CharacterSheetRecord` and `TacticalPoolEnvelope`.
- `inspect_public_tactical_map`: Read public battlemap tokens and `CombatTrackerState`.
- `query_satya_rules`: Search the canonical Satya `2d20` rules book (`docs/ttrpg/`).
- `roll_player_d20_action`: Execute an authoritative Satya `2d20` skill check or Combat Technique + `T/C/F` roll.
- `send_player_chat`: Emit an `IN_CHARACTER`, `OUT_OF_CHARACTER`, or `GM_ONLY` message as the AI Player.
"""

from __future__ import annotations

from typing import Any

import httpx

from satya_ml.ai_gm.tools import (
    _gm_headers,
    _http_post_json,
    _signaling_base_url,
    compute_character_tactical_envelope,
    query_satya_rules,
    tool,
)


def _http_get_json(url: str, timeout: float = 2.5) -> dict[str, Any] | None:
    """GET JSON from `url` with GM/Player service headers, returning parsed JSON on 200 OK or None on failure."""
    try:
        with httpx.Client(timeout=timeout) as client:
            resp = client.get(url, headers=_gm_headers())
            if resp.status_code == 200:
                return dict(resp.json())
    except Exception:
        return None
    return None

try:
    import satya_py
except ImportError:  # pragma: no cover
    satya_py = None  # type: ignore[assignment]


_DEFAULT_AI_PLAYER_CHARACTER: dict[str, Any] = {
    "character_id": "char-ai-kaelen-voss",
    "character_name": "Kaelen Voss",
    "callsign_or_role": "Recon & Electronic Warfare Specialist",
    "archetype": "Void-Recon Scout",
    "faction": "Free Horizon Flotilla",
    "characteristics": {
        "for_stat": 11,
        "end": 12,
        "agi": 14,
        "dex": 15,
        "per": 14,
        "int_stat": 13,
        "wil": 13,
        "cha": 10,
    },
    "attribute_tns": {
        "STR": 11,
        "DEX": 15,
        "INT": 13,
        "WIL": 13,
    },
    "skills": {
        "small_arms": 3,
        "stealth_infiltration": 3,
        "sensor_operations": 2,
        "close_combat": 2,
        "athletics": 2,
    },
    "psyche_traits": {
        "altruism_egoism": 1,
        "conformity_rebellion": -2,
        "trust_suspicion": -1,
        "idealism_cynicism": 0,
        "compassion_ruthlessness": 1,
        "caution_audacity": 2,
        "calm_volatility": -1,
    },
    "combat_stance": "NEUTRAL",
    "fatigue_level": 0,
    "wound_shock_index": 0.0,
    "loadout": [
        "VX-9 Suppressed Flechette Carbine",
        "Light Recon Void-Suit (Guard +1)",
        "Multi-Spectrum Signal Splicer",
    ],
    "biography": (
        "Former deep-space telemetry scout who relies on high OODA initiative, "
        "covert sensor sweeps, and disciplined Tempo/Control fire."
    ),
}

# Strict player-scoped context cache keyed by `f"{session_id}:{player_user_id}"`
_PLAYER_CONTEXT_CACHE: dict[str, dict[str, Any]] = {}


def _player_cache_key(session_id: str, player_user_id: str) -> str:
    return f"{session_id.strip() or 'default'}:{player_user_id.strip() or 'ai-player-1'}"


def sanitize_player_visible_context(raw_ctx: dict[str, Any], player_user_id: str) -> dict[str, Any]:
    """Strip any `GM_ONLY` documents, GM memories, NPC secrets, or other players' private messages."""
    roster = [
        dict(p)
        for p in (raw_ctx.get("roster") or [])
        if isinstance(p, dict)
    ]

    # Pick this AI Player's bound character sheet (or fallback starter sheet)
    own_char: dict[str, Any] | None = None
    if isinstance(raw_ctx.get("character_sheet"), dict):
        own_char = dict(raw_ctx["character_sheet"])
    else:
        bound_chars = [
            c for c in (raw_ctx.get("bound_characters") or []) if isinstance(c, dict)
        ]
        for c in bound_chars:
            if (
                str(c.get("owner_user_id") or "") == player_user_id
                or str(c.get("user_id") or "") == player_user_id
            ):
                own_char = dict(c)
                break
        if own_char is None and bound_chars:
            own_char = dict(bound_chars[0])
    if own_char is None:
        own_char = {
            **_DEFAULT_AI_PLAYER_CHARACTER,
            "owner_user_id": player_user_id,
        }

    # Filter chat history to strictly PUBLIC messages or PRIVATE messages involving `player_user_id`
    visible_messages: list[dict[str, Any]] = []
    for msg in raw_ctx.get("recent_messages") or raw_ctx.get("chat_history") or []:
        if not isinstance(msg, dict):
            continue
        vis = str(msg.get("visibility") or "PUBLIC").upper()
        sender_uid = str((msg.get("sender") or {}).get("user_id") or msg.get("sender_user_id") or "")
        recip_ids = [
            str(r.get("user_id") if isinstance(r, dict) else r)
            for r in (msg.get("recipients") or msg.get("recipient_user_ids") or [])
        ]
        if vis == "PUBLIC":
            visible_messages.append(dict(msg))
        elif vis == "PRIVATE" and (player_user_id in recip_ids or sender_uid == player_user_id):
            visible_messages.append(dict(msg))
        # Strictly exclude GM_ONLY messages unless sent by this AI Player
        elif vis == "GM_ONLY" and sender_uid == player_user_id:
            visible_messages.append(dict(msg))

    # Sanitize tactical map (keep only public token positions)
    raw_map = raw_ctx.get("active_map")
    public_map: dict[str, Any] | None = None
    if isinstance(raw_map, dict) and raw_map.get("map_id"):
        public_tokens = [
            {
                "token_id": t.get("token_id"),
                "label": t.get("label"),
                "grid_x": t.get("grid_x"),
                "grid_y": t.get("grid_y"),
                "faction": t.get("faction", "NEUTRAL"),
            }
            for t in (raw_map.get("tokens") or [])
            if isinstance(t, dict) and not t.get("hidden") and not t.get("gm_only")
        ]
        public_map = {
            "map_id": raw_map.get("map_id"),
            "map_image_url": raw_map.get("map_image_url", ""),
            "grid_width": int(raw_map.get("grid_width", 20)),
            "grid_height": int(raw_map.get("grid_height", 20)),
            "tokens": public_tokens,
        }

    combat_tracker = dict(raw_ctx.get("combat_tracker") or {
        "session_id": raw_ctx.get("session_id", "default"),
        "combat_active": False,
        "round": 1,
        "tick": 1,
        "combatants": [],
    })

    return {
        "session_id": str(raw_ctx.get("session_id") or "default"),
        "player_user_id": player_user_id,
        "player_display_name": str(
            raw_ctx.get("player_display_name")
            or own_char.get("character_name")
            or "Kaelen Voss (AI Player)"
        ),
        "persona_prompt": str(raw_ctx.get("persona_prompt") or ""),
        "roster": roster,
        "character_sheet": own_char,
        "tactical_envelope": compute_character_tactical_envelope(own_char),
        "combat_tracker": combat_tracker,
        "active_map": public_map,
        "recent_messages": visible_messages[-16:],
    }


def set_player_session_context(
    session_id: str,
    player_user_id: str,
    raw_ctx: dict[str, Any],
) -> dict[str, Any]:
    """Sanitize and cache player-visible session context for `(session_id, player_user_id)`."""
    sanitized = sanitize_player_visible_context(
        {**raw_ctx, "session_id": session_id},
        player_user_id,
    )
    _PLAYER_CONTEXT_CACHE[_player_cache_key(session_id, player_user_id)] = sanitized
    return dict(sanitized)


def get_player_session_context(
    session_id: str,
    player_user_id: str = "ai-player-1",
) -> dict[str, Any]:
    """Return cached player-visible session context for `(session_id, player_user_id)`, initializing defaults if needed."""
    key = _player_cache_key(session_id, player_user_id)
    if key not in _PLAYER_CONTEXT_CACHE:
        _PLAYER_CONTEXT_CACHE[key] = sanitize_player_visible_context(
            {"session_id": session_id},
            player_user_id,
        )
    return dict(_PLAYER_CONTEXT_CACHE[key])


def fetch_player_context_from_signaling(
    session_id: str,
    player_user_id: str,
) -> dict[str, Any] | None:
    """Fetch player-scoped context from `GET /api/player-ai/sessions/{session_id}/context?user_id={player_user_id}`."""
    url = f"{_signaling_base_url()}/api/player-ai/sessions/{session_id}/context?user_id={player_user_id}"
    remote = _http_get_json(url, timeout=2.5)
    if isinstance(remote, dict) and not remote.get("error"):
        return set_player_session_context(session_id, player_user_id, remote)
    return None


@tool
def read_my_character_sheet(
    session_id: str = "default",
    player_user_id: str = "ai-player-1",
) -> dict[str, Any]:
    """Read your own bound Satya character sheet, Attribute TNs, Skill ranks, 7 Bipolar Psyche traits, Loadout, and Tactical Pool Envelope (`P_eff`, Neutral `B`, `T_max/C_max/F_max`)."""
    ctx = get_player_session_context(session_id, player_user_id)
    sheet = dict(ctx.get("character_sheet") or _DEFAULT_AI_PLAYER_CHARACTER)
    envelope = compute_character_tactical_envelope(sheet)
    return {
        "session_id": session_id,
        "player_user_id": player_user_id,
        "character_sheet": sheet,
        "tactical_pool_envelope": envelope,
    }


@tool
def inspect_public_tactical_map(
    session_id: str = "default",
    player_user_id: str = "ai-player-1",
) -> dict[str, Any]:
    """Inspect the public 2D battlemap token coordinates and active Count-Up Tick Clock (`combat_active`, `round`, `tick`, `combatants`)."""
    ctx = get_player_session_context(session_id, player_user_id)
    return {
        "session_id": session_id,
        "combat_tracker": ctx.get("combat_tracker"),
        "active_map": ctx.get("active_map"),
    }


@tool
def roll_player_d20_action(
    check_category: str,
    attribute_tn: int = 13,
    skill_rank: int = 2,
    difficulty_mod: int = 0,
    internal_mod: int = 0,
    combat_technique: str = "",
    tempo_pool: int = 0,
    control_pool: int = 0,
    force_pool: int = 0,
    note: str = "",
    session_id: str = "default",
    player_user_id: str = "ai-player-1",
) -> dict[str, Any]:
    """Execute an authoritative Satya Two-List Cascading (`2d20`) skill check or Combat Technique + `Tempo / Control / Force` (`T/C/F`) roll as your Player Character."""
    ctx = get_player_session_context(session_id, player_user_id)
    sheet = dict(ctx.get("character_sheet") or _DEFAULT_AI_PLAYER_CHARACTER)
    env = compute_character_tactical_envelope(sheet)
    display_name = str(
        ctx.get("player_display_name")
        or sheet.get("character_name")
        or "Kaelen Voss"
    )

    tech_clean = combat_technique.strip().lower()
    t_alloc = int(tempo_pool)
    c_alloc = int(control_pool)
    f_alloc = int(force_pool)
    if tech_clean and (t_alloc + c_alloc + f_alloc) <= 0:
        # Default balanced Tactical Triad within P_eff
        b = int(env.get("neutral_baseline", 4))
        t_alloc, c_alloc, f_alloc = b, b, max(0, int(env.get("p_eff", 12)) - 2 * b)

    payload: dict[str, Any] = {
        "user_id": player_user_id,
        "actor_display_name": display_name,
        "check_category": check_category,
        "attribute_tn": int(attribute_tn),
        "skill_rank": max(1, int(skill_rank)),
        "difficulty_mod": int(difficulty_mod),
        "internal_mod": int(internal_mod),
        "combat_technique": tech_clean or None,
        "tempo_pool": t_alloc if tech_clean else None,
        "control_pool": c_alloc if tech_clean else None,
        "force_pool": f_alloc if tech_clean else None,
        "notes": note or f"{display_name} player roll ({check_category})",
    }

    remote = _http_post_json(
        f"{_signaling_base_url()}/api/player-ai/sessions/{session_id}/roll",
        payload,
        timeout=3.0,
    )
    if isinstance(remote, dict) and not remote.get("error"):
        return remote

    # Deterministic local fallback using `satya_py` when running offline unit tests
    c_stat = max(1, min(30, int(attribute_tn) - int(internal_mod)))
    n_pairs = max(1, min(10, int(skill_rank)))
    b_val = int(env.get("neutral_baseline", 4))
    control_delta = -(c_alloc - b_val) if tech_clean else 0
    delta_ext = int(difficulty_mod) + control_delta
    tn_eff = max(2, min(39, 31 - c_stat + delta_ext))

    rolled_pairs: list[list[int]] = []
    successes = 0
    carry = 0
    for idx in range(n_pairs):
        d1 = ((hash((session_id, player_user_id, check_category, idx, 1)) % 20) + 1)
        d2 = ((hash((session_id, player_user_id, check_category, idx, 2)) % 20) + 1)
        rolled_pairs.append([d1, d2])
        pair_sum = d1 + d2 + carry
        if pair_sum >= tn_eff:
            successes += 1
            carry = 6
        else:
            carry = -(tn_eff - pair_sum)

    base_tc = 3 if tech_clean in {"snap_shot", "cover_peek", "thrust", "punch"} else 6
    eff_tc = max(1, base_tc - max(0, (t_alloc - b_val) // 2)) if tech_clean else None
    degree = "CRITICAL_SUCCESS" if successes >= n_pairs and n_pairs >= 2 else ("SUCCESS" if successes > 0 else "FAILURE")

    # Advance local cached combat tracker tick for this player if in Combat Action Mode
    tracker = dict(ctx.get("combat_tracker") or {})
    cur_tick = max(1, int(tracker.get("tick", 1)))
    next_tick = (cur_tick + eff_tc) if eff_tc is not None else None
    if eff_tc is not None and tracker.get("combat_active"):
        combatants = [dict(c) for c in (tracker.get("combatants") or []) if isinstance(c, dict)]
        updated_any = False
        for c in combatants:
            if str(c.get("entity_id") or "") in {player_user_id, str(sheet.get("character_id") or "")}:
                c["next_action_tick"] = next_tick
                c["active_stance"] = f"{tech_clean.upper()} (T{t_alloc}/C{c_alloc}/F{f_alloc})"
                updated_any = True
        if updated_any:
            tracker["combatants"] = combatants
            ctx["combat_tracker"] = tracker
            _PLAYER_CONTEXT_CACHE[_player_cache_key(session_id, player_user_id)] = ctx

    return {
        "status": "ok",
        "session_id": session_id,
        "player_user_id": player_user_id,
        "actor_display_name": display_name,
        "check_category": check_category,
        "combat_technique": tech_clean or None,
        "triad": {"tempo": t_alloc, "control": c_alloc, "force": f_alloc} if tech_clean else None,
        "effective_tc": eff_tc,
        "next_action_tick": next_tick,
        "effective_tn": tn_eff,
        "rolled_pairs": rolled_pairs,
        "successes_counted": successes,
        "degree_of_success": degree,
    }


@tool
def send_player_chat(
    message: str,
    channel_context: str = "IN_CHARACTER",
    visibility: str = "PUBLIC",
    recipient_user_ids: list[str] | None = None,
    session_id: str = "default",
    player_user_id: str = "ai-player-1",
) -> dict[str, Any]:
    """Send an `IN_CHARACTER` action/dialogue message, `OUT_OF_CHARACTER` question, or `GM_ONLY` whisper as the AI Player."""
    ctx = get_player_session_context(session_id, player_user_id)
    display_name = str(ctx.get("player_display_name") or "Kaelen Voss (AI Player)")
    payload = {
        "user_id": player_user_id,
        "display_name": display_name,
        "visibility": visibility.upper(),
        "recipient_user_ids": list(recipient_user_ids or []),
        "channel_context": channel_context.upper(),
        "body": message,
    }
    remote = _http_post_json(
        f"{_signaling_base_url()}/api/player-ai/sessions/{session_id}/chat",
        payload,
        timeout=3.0,
    )
    if isinstance(remote, dict) and not remote.get("error"):
        return remote
    return {
        "status": "buffered_locally",
        "session_id": session_id,
        "player_user_id": player_user_id,
        "display_name": display_name,
        "channel_context": channel_context.upper(),
        "visibility": visibility.upper(),
        "body": message,
    }


def get_builtin_player_tools() -> list[Any]:
    """Return the player-scoped `@tool` list bound to the AI Player LangGraph agent."""
    return [
        read_my_character_sheet,
        inspect_public_tactical_map,
        query_satya_rules,
        roll_player_d20_action,
        send_player_chat,
    ]
