"""Player-scoped tool suite (`@tool`) for the Satya AI Player Agent (`satya_ml.ai_player`).

Strictly isolates player state per `(session_id, player_user_id)` and exposes only player-legal
capabilities:
- `read_my_character_sheet`: Read the AI Player's bound `CharacterSheetRecord` and `TacticalPoolEnvelope`.
- `inspect_public_tactical_map`: Read public battlemap tokens and `CombatTrackerState`.
- `move_player_map_token`: Move ONLY your own character miniature with wall-collision enforcement.
- `inspect_surroundings`: Inspect walkable zones, exits, nearby tokens/furniture, and cover/LoS.
- `lookup_public_lore_and_npcs`: Search public lore, factions, and public NPC bios (never secrets).
- `perform_player_empathy_read`: Run an `INS / ITU` check to unlock an NPC's Psyche motivations on net success.
- `update_my_motivation_journal`: Persist private player goals, unresolved clues, and party bonds.
- `query_satya_rules`: Search the canonical Satya `2d20` rules book (`docs/ttrpg/`).
- `roll_player_d20_action`: Execute an authoritative Satya `2d20` skill check or Combat Technique + `T/C/F` roll.
- `send_player_chat`: Emit an `IN_CHARACTER`, `OUT_OF_CHARACTER`, or `GM_ONLY` message as the AI Player.
"""

from __future__ import annotations

import hashlib
from typing import Any

from satya_ml.ai_gm.tools import (
    _gm_headers,
    _http_get_json,
    _http_post_json,
    _signaling_base_url,
    compute_character_tactical_envelope,
    query_satya_rules,
    tool,
)
from satya_ml.ai_player.motivation_ledger import (
    ensure_or_sync_motivation_ledger,
    format_motivation_ledger_for_prompt,
    inspect_surroundings,
    lookup_public_lore_and_npcs,
    move_player_map_token,
    perform_player_empathy_read,
    redact_public_npc_entry,
    redact_world_ledger_for_player,
    reset_player_motivation_ledgers,
    sanitize_map_dict_for_player,
    summarize_public_map_for_player,
    update_my_motivation_journal,
)
from satya_ml.campaign_config import get_pack_default_ai_player
from satya_ml.dice_probabilities import _ensure_satya_py_loaded


try:
    import satya_py
except ImportError:  # pragma: no cover
    satya_py = None  # type: ignore[assignment]


# Strict player-scoped context cache keyed by `f"{session_id}:{player_user_id}"`
_PLAYER_CONTEXT_CACHE: dict[str, dict[str, Any]] = {}


def reset_player_session_context(session_id: str | None = None) -> None:
    """Clear cached player-visible session contexts and `MotivationLedger` states."""
    reset_player_motivation_ledgers(session_id)
    if session_id is None:
        _PLAYER_CONTEXT_CACHE.clear()
        return
    prefix = f"{session_id.strip() or 'default'}:"
    for key in [k for k in _PLAYER_CONTEXT_CACHE if k.startswith(prefix)]:
        _PLAYER_CONTEXT_CACHE.pop(key, None)


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
            **get_pack_default_ai_player(),
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

    # Sanitize tactical map (preserve geometry/exits while strictly removing GM_ONLY tokens & spatial_pins)
    raw_map = raw_ctx.get("active_map")
    public_map = sanitize_map_dict_for_player(raw_map)
    if isinstance(public_map, dict) and isinstance(raw_map, dict):
        public_map.setdefault("map_image_url", raw_map.get("map_image_url", ""))

    public_npcs = [
        redact_public_npc_entry(npc)
        for npc in (raw_ctx.get("public_npcs") or [])
        if isinstance(npc, dict)
    ]
    world_ledger = redact_world_ledger_for_player(raw_ctx.get("world_ledger"))

    combat_tracker = dict(raw_ctx.get("combat_tracker") or {
        "session_id": raw_ctx.get("session_id", "default"),
        "combat_active": False,
        "round": 1,
        "tick": 1,
        "combatants": [],
    })

    default_name = str(get_pack_default_ai_player().get("character_name") or "Mateo Vance")
    return {
        "session_id": str(raw_ctx.get("session_id") or "default"),
        "player_user_id": player_user_id,
        "player_display_name": str(
            raw_ctx.get("player_display_name")
            or own_char.get("character_name")
            or f"{default_name} (AI Player)"
        ),
        "persona_prompt": str(raw_ctx.get("persona_prompt") or ""),
        "roster": roster,
        "character_sheet": own_char,
        "tactical_envelope": compute_character_tactical_envelope(own_char),
        "combat_tracker": combat_tracker,
        "active_map": public_map,
        "public_npcs": public_npcs,
        "world_ledger": world_ledger,
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
    sheet = dict(ctx.get("character_sheet") or get_pack_default_ai_player())
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
    default_sheet = get_pack_default_ai_player()
    sheet = dict(ctx.get("character_sheet") or default_sheet)
    env = compute_character_tactical_envelope(sheet)
    display_name = str(
        ctx.get("player_display_name")
        or sheet.get("character_name")
        or default_sheet.get("character_name")
        or "Mateo Vance"
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

    # Deterministic local fallback using compiled Rust `satya_py` when running offline unit tests
    satya_lib = _ensure_satya_py_loaded()
    c_stat = max(1, min(30, int(attribute_tn)))
    n_pairs = max(1, min(10, int(skill_rank)))
    b_val = int(env.get("neutral_baseline", 4))
    control_delta = -(c_alloc - b_val) if tech_clean else 0
    base_tc = 3 if tech_clean in {"snap_shot", "cover_peek", "thrust", "punch"} else 6
    eff_tc = max(1, base_tc - max(0, (t_alloc - b_val) // 2)) if tech_clean else None
    if tech_clean and hasattr(satya_lib, "evaluate_combat_technique_triad"):
        dom = (
            "ranged"
            if tech_clean
            in {
                "snap_shot",
                "aimed_shot",
                "braced_shot",
                "full_auto",
                "suppressive_fire",
                "cover_peek",
                "combat_sighted_shot",
            }
            else (
                "unarmed_martial_arts"
                if tech_clean in {"punch", "kick", "grab", "throw", "tackle"}
                else "armed_melee"
            )
        )
        try:
            triad_eval = satya_lib.evaluate_combat_technique_triad(
                tech_clean,
                t_alloc,
                c_alloc,
                f_alloc,
                dom,
                n_pairs,
            )
            if isinstance(triad_eval, dict):
                if triad_eval.get("effective_total_tc") is not None:
                    eff_tc = int(triad_eval["effective_total_tc"])
                if triad_eval.get("technique_delta_ext") is not None:
                    control_delta = int(triad_eval["technique_delta_ext"])
        except Exception:
            pass

    delta_ext = int(difficulty_mod) + control_delta
    if hasattr(satya_lib, "CascadeTaskParams"):
        tn_eff = int(
            satya_lib.CascadeTaskParams(
                n_pairs,
                c_stat,
                int(internal_mod),
                delta_ext,
                "markov_step",
                "closest_miss",
            ).tn_eff()
        )
    else:
        tn_eff = max(2, min(39, 31 - c_stat + delta_ext))

    rolled_pairs: list[list[int]] = []
    pair_tuples: list[tuple[int, int]] = []
    for idx in range(n_pairs):
        digest = hashlib.sha256(
            f"{session_id}|{player_user_id}|{check_category}|{idx}".encode("utf-8")
        ).digest()
        d1 = (int.from_bytes(digest[0:4], "big") % 20) + 1
        d2 = (int.from_bytes(digest[4:8], "big") % 20) + 1
        rolled_pairs.append([d1, d2])
        pair_tuples.append((d1, d2))

    outcome = satya_lib.evaluate_cascade_roll(
        n_pairs,
        c_stat,
        int(internal_mod),
        delta_ext,
        pair_tuples,
        "markov_step",
        "closest_miss",
    )
    successes = int(outcome.total_successes)
    outcome_tier = str(outcome.outcome_tier)
    degree = (
        "CRITICAL_SUCCESS"
        if successes >= n_pairs and n_pairs >= 2
        else ("SUCCESS" if successes > 0 else "FAILURE")
    )

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
        "total_successes": successes,
        "successes_counted": successes,
        "outcome_tier": outcome_tier,
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
    default_name = str(get_pack_default_ai_player().get("character_name") or "Mateo Vance")
    display_name = str(ctx.get("player_display_name") or f"{default_name} (AI Player)")
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
        move_player_map_token,
        inspect_surroundings,
        lookup_public_lore_and_npcs,
        perform_player_empathy_read,
        update_my_motivation_journal,
        query_satya_rules,
        roll_player_d20_action,
        send_player_chat,
    ]
