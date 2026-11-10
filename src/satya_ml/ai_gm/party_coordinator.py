"""Multi-player turn readiness barrier (`party_turn_gate_node`) and autonomous NPC action/tick
resolution (`npc_action_resolution_node`) for the Satya LangGraph AI Game Master.

Ensures that:
1. In multi-player sessions (`>= 2` human `PLAYER`s), all active players in a Narrative round
   (or all players tied at the lowest `next_action_tick` in Combat Action Mode) declare their
   actions or rolls before the AI GM invokes the LLM or advances the Scenario Director.
2. Single-player sessions, `OUT_OF_CHARACTER` rules questions, `CO_PILOT` whispers, and
   proactive `SCENARIO_BEAT` triggers bypass the barrier with zero added latency.
3. In Combat Action Mode (`combat_active = true`), NPC combatants whose `next_action_tick` is
   due on the Count-Up Tick Clock automatically execute authoritative Satya `2d20` technique +
   `Tempo / Control / Force` (`T/C/F`) rolls and advance their `next_action_tick`.
"""

from __future__ import annotations

import time
import uuid
from typing import Any, Literal, TypedDict

from satya_ml.ai_gm.tools import (
    advance_combat_tracker,
    get_session_combat_tracker,
    roll_satya_d20_check,
)

_PASS_HOLD_PHRASES: frozenset[str] = frozenset(
    {
        "pass",
        "wait",
        "hold",
        "skip",
        "ready",
        "[pass]",
        "[wait]",
        "[hold]",
        "[ready]",
        "i pass",
        "i wait",
        "i hold",
        "hold action",
        "pass turn",
    }
)

_FORCE_RESOLVE_PHRASES: frozenset[str] = frozenset(
    {
        "!resolve",
        "/resolve",
        "!go",
        "[resolve]",
        "force resolve",
        "!proceed",
    }
)


class QueuedPlayerAction(TypedDict, total=False):
    """Single player's buffered action/roll declaration within an open party turn window."""

    user_id: str
    display_name: str
    action_text: str
    roll_summary: str | None
    is_pass: bool
    target_tick: int | None
    submitted_at_ms: float


class PartyGateDecision(TypedDict):
    """Decision returned by `evaluate_party_turn_gate`."""

    status: Literal["bypass", "waiting", "ready"]
    pending_round_actions: dict[str, dict[str, Any]]
    awaiting_user_ids: list[str]
    awaiting_display_names: list[str]
    waiting_notice_markdown: str
    combined_party_prompt_block: str


def extract_active_human_players(roster: list[dict[str, Any]] | None) -> list[dict[str, Any]]:
    """Return all active `PLAYER` participants (`HUMAN` and `AI_AGENT` test players) from `roster` (excluding GM, AI_GM, and OBSERVER)."""
    if not roster:
        return []
    players: list[dict[str, Any]] = []
    seen_uids: set[str] = set()
    for p in roster:
        if not isinstance(p, dict):
            continue
        uid = str(p.get("user_id") or "").strip()
        if not uid or uid in seen_uids:
            continue
        kind = str(p.get("participant_kind") or "HUMAN").strip().upper()
        role = str(p.get("role") or "PLAYER").strip().upper()
        if role != "PLAYER" or kind not in {"HUMAN", "AI_AGENT"}:
            continue
        seen_uids.add(uid)
        players.append(p)
    return players


extract_active_party_players = extract_active_human_players


def _is_pass_or_hold_text(content: str) -> bool:
    cleaned = content.strip().lower().rstrip(".!")
    return cleaned in _PASS_HOLD_PHRASES


def _is_force_resolve_text(content: str) -> tuple[bool, str]:
    raw = content.strip()
    lower = raw.lower()
    if lower in _FORCE_RESOLVE_PHRASES:
        return True, ""
    for prefix in ("!resolve ", "/resolve ", "!go ", "[resolve] "):
        if lower.startswith(prefix):
            return True, raw[len(prefix) :].strip()
    return False, raw


def _match_player_combatant(
    player: dict[str, Any],
    combatants: list[dict[str, Any]],
) -> dict[str, Any] | None:
    uid = str(player.get("user_id") or "").strip().lower()
    cid = str(player.get("character_id") or "").strip().lower()
    dname = str(player.get("display_name") or player.get("username") or "").strip().lower()
    for c in combatants:
        if not isinstance(c, dict):
            continue
        c_eid = str(c.get("entity_id") or "").strip().lower()
        c_name = str(c.get("display_name") or "").strip().lower()
        if (uid and c_eid == uid) or (cid and c_eid == cid) or (dname and c_name == dname):
            return c
    return None


def _is_npc_combatant(
    combatant: dict[str, Any],
    active_npcs: dict[str, dict[str, Any]],
    active_players: list[dict[str, Any]],
) -> bool:
    if bool(combatant.get("is_npc")):
        return True
    role_fac = str(combatant.get("role_or_faction") or "").strip().upper()
    if role_fac == "NPC":
        return True
    eid = str(combatant.get("entity_id") or "").strip().lower()
    dname = str(combatant.get("display_name") or "").strip().lower()
    for p in active_players:
        p_uid = str(p.get("user_id") or "").strip().lower()
        p_cid = str(p.get("character_id") or "").strip().lower()
        p_name = str(p.get("display_name") or "").strip().lower()
        if eid in {p_uid, p_cid} or (dname and dname == p_name):
            return False
    if eid.startswith(("npc_", "secops_", "hostile_", "enemy_")):
        return True
    if eid in active_npcs:
        return True
    for npc_id, prof in active_npcs.items():
        if eid == npc_id.lower() or (dname and dname == str(prof.get("name") or "").strip().lower()):
            return True
    return False


def evaluate_party_turn_gate(
    *,
    session_id: str,
    operating_mode: str,
    roster: list[dict[str, Any]] | None,
    trigger: dict[str, Any] | None,
    pending_round_actions: dict[str, dict[str, Any]] | None,
    combat_tracker: dict[str, Any] | None,
    is_ooc: bool,
    turn_barrier_timeout_sec: float = 120.0,
    now_ms: float | None = None,
) -> PartyGateDecision:
    """Evaluate whether all required players have declared their actions for the current round/tick.

    Returns a `PartyGateDecision` with `status` in `{"bypass", "waiting", "ready"}`.
    """
    current_ms = now_ms if now_ms is not None else (time.time() * 1000.0)
    pending: dict[str, dict[str, Any]] = {
        k: dict(v) for k, v in (pending_round_actions or {}).items() if isinstance(v, dict)
    }
    active_players = extract_active_human_players(roster)

    if not trigger:
        return {
            "status": "bypass",
            "pending_round_actions": pending,
            "awaiting_user_ids": [],
            "awaiting_display_names": [],
            "waiting_notice_markdown": "",
            "combined_party_prompt_block": "",
        }

    trigger_kind = str(trigger.get("trigger_kind") or trigger.get("kind") or "CHAT").strip().upper()
    vis = str(trigger.get("channel") or trigger.get("visibility") or "PUBLIC").strip().upper()
    force_beat = bool(trigger.get("force_scenario_beat")) or trigger_kind == "SCENARIO_BEAT"

    # Immediate bypasses: Co-Pilot mode, OOC/rules queries, proactive Scenario Beats, private whispers,
    # or single-player / zero-player sessions
    if (
        operating_mode == "co_pilot"
        or is_ooc
        or force_beat
        or (vis in {"PRIVATE", "GM_ONLY"} and not bool(trigger.get("queue_in_party_turn")))
        or len(active_players) <= 1
    ):
        awaiting_ids = [
            str(p.get("user_id"))
            for p in active_players
            if str(p.get("user_id")) not in pending
        ]
        awaiting_names = [
            str(p.get("display_name") or p.get("username") or p.get("user_id"))
            for p in active_players
            if str(p.get("user_id")) not in pending
        ]
        return {
            "status": "bypass",
            "pending_round_actions": pending,
            "awaiting_user_ids": awaiting_ids if len(active_players) > 1 else [],
            "awaiting_display_names": awaiting_names if len(active_players) > 1 else [],
            "waiting_notice_markdown": "",
            "combined_party_prompt_block": "",
        }

    actor = trigger.get("actor") or {}
    actor_uid = str(actor.get("user_id") or "").strip()
    actor_name = str(
        actor.get("display_name") or actor.get("username") or actor_uid or "Player"
    ).strip()
    raw_content = str(trigger.get("content") or "").strip()
    roll_summary = str(trigger.get("roll_summary") or "").strip() or None

    is_force_cmd, stripped_content = _is_force_resolve_text(raw_content)
    force_resolve = bool(trigger.get("force_resolve_turn")) or is_force_cmd
    effective_content = stripped_content if is_force_cmd else raw_content
    is_pass = _is_pass_or_hold_text(effective_content) and not roll_summary

    combat_is_active = bool(
        combat_tracker
        and (combat_tracker.get("combat_active") or combat_tracker.get("is_active"))
    )
    combatants = list((combat_tracker or {}).get("combatants") or [])

    # Map each active player to their combatant tick if Combat Action Mode is active
    player_ticks: dict[str, int] = {}
    if combat_is_active and combatants:
        for p in active_players:
            p_uid = str(p.get("user_id") or "")
            matched_c = _match_player_combatant(p, combatants)
            if matched_c is not None:
                player_ticks[p_uid] = int(matched_c.get("next_action_tick", 1))

    # Record or merge the triggering player's declaration if they provided content, a roll, or a pass
    if actor_uid and (effective_content or roll_summary or is_pass or not force_resolve):
        existing = pending.get(actor_uid)
        target_tick = player_ticks.get(actor_uid)
        if existing is not None:
            prev_text = str(existing.get("action_text") or "").strip()
            if effective_content and not is_pass and effective_content not in prev_text:
                merged_text = (
                    f"{prev_text}; {effective_content}" if prev_text else effective_content
                )
            else:
                merged_text = prev_text or effective_content
            merged_roll = roll_summary or existing.get("roll_summary")
            pending[actor_uid] = {
                "user_id": actor_uid,
                "display_name": actor_name,
                "action_text": merged_text,
                "roll_summary": merged_roll,
                "is_pass": bool(existing.get("is_pass") and is_pass and not merged_roll),
                "target_tick": target_tick,
                "submitted_at_ms": float(existing.get("submitted_at_ms") or current_ms),
            }
        elif effective_content or roll_summary or is_pass:
            pending[actor_uid] = {
                "user_id": actor_uid,
                "display_name": actor_name,
                "action_text": "*(holds / passes action)*" if is_pass else effective_content,
                "roll_summary": roll_summary,
                "is_pass": is_pass,
                "target_tick": target_tick,
                "submitted_at_ms": current_ms,
            }

    # Determine which players are required before resolving this beat/tick
    if combat_is_active and player_ticks:
        min_player_tick = min(player_ticks.values())
        required_players = [
            p
            for p in active_players
            if player_ticks.get(str(p.get("user_id") or ""), min_player_tick) <= min_player_tick
        ]
    else:
        min_player_tick = None
        required_players = list(active_players)

    awaiting_players = [
        p for p in required_players if str(p.get("user_id") or "") not in pending
    ]
    awaiting_ids = [str(p.get("user_id") or "") for p in awaiting_players]
    awaiting_names = [
        str(p.get("display_name") or p.get("username") or p.get("user_id"))
        for p in awaiting_players
    ]

    # Check if the round barrier timeout has elapsed
    oldest_ms = min(
        (float(entry.get("submitted_at_ms") or current_ms) for entry in pending.values()),
        default=current_ms,
    )
    timed_out = (
        bool(pending)
        and turn_barrier_timeout_sec > 0
        and (current_ms - oldest_ms) >= (turn_barrier_timeout_sec * 1000.0)
    )

    if awaiting_ids and not force_resolve and not timed_out:
        total_req = len(required_players)
        ready_req = total_req - len(awaiting_ids)
        actor_entry = pending.get(actor_uid) or {}
        action_preview = str(
            actor_entry.get("roll_summary") or actor_entry.get("action_text") or raw_content
        ).strip()
        if len(action_preview) > 140:
            action_preview = action_preview[:137] + "..."
        ready_names = [
            str(pending[ str(p.get("user_id")) ].get("display_name") or p.get("display_name"))
            for p in required_players
            if str(p.get("user_id")) in pending
        ]
        if combat_is_active and min_player_tick is not None:
            actor_tick = player_ticks.get(actor_uid, min_player_tick)
            tick_note = (
                f" *(queued for Tick {actor_tick})*"
                if actor_tick > min_player_tick
                else f" *(Tick {min_player_tick})*"
            )
            notice = (
                f"> **[Action Mode Tick Queue · Tick {min_player_tick} — {ready_req}/{total_req} Ready]**\n"
                f"> Recorded declaration for **{actor_name}**{tick_note}: `{action_preview}`\n"
                f"> - **Ready on Tick {min_player_tick}**: {', '.join(f'**{n}**' for n in ready_names) if ready_names else '*(none yet)*'}\n"
                f"> - **Waiting on**: {', '.join(f'**{n}**' for n in awaiting_names)} "
                f"*(declare your technique/action, roll in the Task Resolution drawer, or type `pass` / `hold`)*"
            )
        else:
            notice = (
                f"> **[Party Turn Queue — {ready_req}/{total_req} Players Ready]**\n"
                f"> Recorded declaration for **{actor_name}**: *{action_preview}*\n"
                f"> - **Ready**: {', '.join(f'**{n}**' for n in ready_names)}\n"
                f"> - **Waiting on**: {', '.join(f'**{n}**' for n in awaiting_names)} "
                f"*(describe what your character does, roll a `2d20` check, or send `pass` / `hold` to proceed)*"
            )
        return {
            "status": "waiting",
            "pending_round_actions": pending,
            "awaiting_user_ids": awaiting_ids,
            "awaiting_display_names": awaiting_names,
            "waiting_notice_markdown": notice,
            "combined_party_prompt_block": "",
        }

    # Barrier is satisfied (all required players declared, or force_resolve / timed_out)!
    # Synthesize the combined party turn block in roster / tick order.
    resolved_uids: set[str] = set()
    declaration_lines: list[str] = []

    ordered_players = list(required_players)
    for p in active_players:
        if p not in ordered_players and str(p.get("user_id") or "") in pending:
            # In Combat Action Mode, only consume higher-tick players if force_resolve is set
            # or if we are in Narrative Mode.
            if not combat_is_active or force_resolve:
                ordered_players.append(p)

    for idx, p in enumerate(ordered_players, start=1):
        p_uid = str(p.get("user_id") or "")
        p_name = str(p.get("display_name") or p.get("username") or p_uid)
        entry = pending.get(p_uid)
        tick_prefix = (
            f" [Tick {player_ticks[p_uid]}]"
            if (combat_is_active and p_uid in player_ticks)
            else ""
        )
        if entry is not None:
            resolved_uids.add(p_uid)
            act_txt = str(entry.get("action_text") or "").strip()
            r_sum = str(entry.get("roll_summary") or "").strip()
            if r_sum and act_txt and r_sum not in act_txt:
                declaration_lines.append(
                    f"{idx}. **{p_name}**{tick_prefix}: {act_txt} | **Roll**: {r_sum}"
                )
            elif r_sum:
                declaration_lines.append(f"{idx}. **{p_name}**{tick_prefix} rolled: {r_sum}")
            else:
                declaration_lines.append(f"{idx}. **{p_name}**{tick_prefix}: {act_txt}")
        else:
            declaration_lines.append(
                f"{idx}. **{p_name}**{tick_prefix}: *(no action declared before turn resolution — holds position)*"
            )

    remaining_pending = {
        uid: data for uid, data in pending.items() if uid not in resolved_uids
    }

    mode_header = (
        f"[COMBINED PARTY TURN DECLARATIONS — Session={session_id} | Action Mode Tick {min_player_tick}]"
        if (combat_is_active and min_player_tick is not None)
        else f"[COMBINED PARTY TURN DECLARATIONS — Session={session_id} | All {len(ordered_players)} Players Ready]"
    )
    combined_block = (
        f"{mode_header}\n"
        "Every active player has declared their action for this beat. Adjudicate and weave ALL of the following player actions together in your response:\n"
        + "\n".join(declaration_lines)
    )

    return {
        "status": "ready",
        "pending_round_actions": remaining_pending,
        "awaiting_user_ids": [],
        "awaiting_display_names": [],
        "waiting_notice_markdown": "",
        "combined_party_prompt_block": combined_block,
    }


def _select_npc_combat_technique_and_triad(
    npc_profile: dict[str, Any],
    combatant: dict[str, Any],
) -> tuple[str, str, int, int, int, int, int]:
    """Select `(skill_name, combat_technique, attribute_tn, skill_rank, tempo, control, force)` for an NPC combatant."""
    skills = npc_profile.get("skills") or {}
    attr_tns = npc_profile.get("attribute_tns") or {}
    chars = (npc_profile.get("gm_character") or {}).get("characteristics") or {}
    stance = str(
        combatant.get("active_stance") or npc_profile.get("combat_stance") or "NEUTRAL"
    ).upper()

    # Determine best combat skill & rank
    ranged_keys = (
        "small_arms",
        "pistols_handguns",
        "long_guns_rifles_carbines_shotguns",
        "heavy_weapons_ordnance",
    )
    melee_keys = (
        "close_combat",
        "brawling_unarmed_striking",
        "short_blades_knife_fighting",
        "grappling_wrestling_clinch",
    )
    best_ranged_skill = "small_arms"
    best_ranged_n = 0
    for rk in ranged_keys:
        if int(skills.get(rk, 0)) > best_ranged_n:
            best_ranged_skill = rk
            best_ranged_n = int(skills[rk])

    best_melee_skill = "close_combat"
    best_melee_n = 0
    for mk in melee_keys:
        if int(skills.get(mk, 0)) > best_melee_n:
            best_melee_skill = mk
            best_melee_n = int(skills[mk])

    dex_c = int(chars.get("dex") or attr_tns.get("DEX") or 12)
    str_c = int(chars.get("for_stat") or attr_tns.get("STR") or 12)

    if best_ranged_n >= best_melee_n and best_ranged_n > 0:
        skill_name = best_ranged_skill
        skill_n = max(1, best_ranged_n)
        control_c = dex_c
        if "AIMED" in stance or "DEFENSIVE" in stance:
            technique = "combat_sighted_shot"
            t, c, f = 3, 5, 3
        else:
            technique = "snap_shot"
            t, c, f = 4, 4, 3
    elif best_melee_n > 0:
        skill_name = best_melee_skill
        skill_n = max(1, best_melee_n)
        control_c = max(dex_c, str_c)
        if "AGGRESSIVE" in stance or "POWER" in stance:
            technique = "thrust"
            t, c, f = 4, 3, 4
        else:
            technique = "punch"
            t, c, f = 4, 4, 3
    else:
        # Generic tactical NPC fallback
        skill_name = "small_arms"
        skill_n = 2
        control_c = 12
        technique = "snap_shot"
        t, c, f = 4, 4, 3

    return skill_name, technique, control_c, skill_n, t, c, f


def execute_ready_npc_actions(
    *,
    session_id: str,
    operating_mode: str,
    roster: list[dict[str, Any]] | None,
    combat_tracker: dict[str, Any] | None,
    active_npcs: dict[str, dict[str, Any]] | None,
    is_ooc: bool,
    trigger: dict[str, Any] | None = None,
    max_npc_actions_per_turn: int = 3,
) -> dict[str, Any]:
    """Execute autonomous NPC actions on the Count-Up Tick Clock (Action Mode) or build the Scene NPC Reaction directive.

    Returns a dictionary containing:
    - `npc_turn_summaries`: list of structured NPC action/roll dicts
    - `npc_prompt_block`: Markdown/prompt directive injected for `gm_supervisor_node`
    - `combat_tracker`: updated `CombatTrackerState` dict
    - `tool_invocations`: list of `McpToolInvocationRecord`-compatible dicts for trajectory telemetry
    """
    tracker = dict(combat_tracker or get_session_combat_tracker(session_id) or {})
    npcs = dict(active_npcs or {})
    trig = trigger or {}
    trigger_kind = str(trig.get("trigger_kind") or trig.get("kind") or "CHAT").strip().upper()
    is_scenario_beat_init = trigger_kind == "SCENARIO_BEAT" or bool(trig.get("force_scenario_beat"))

    if operating_mode == "co_pilot" or is_ooc or is_scenario_beat_init:
        return {
            "npc_turn_summaries": [],
            "npc_prompt_block": "",
            "combat_tracker": tracker,
            "tool_invocations": [],
        }

    active_players = extract_active_human_players(roster)
    combat_is_active = bool(tracker.get("combat_active") or tracker.get("is_active"))

    if combat_is_active:
        combatants = list(tracker.get("combatants") or [])
        npc_combatants = [
            c
            for c in combatants
            if isinstance(c, dict) and _is_npc_combatant(c, npcs, active_players)
        ]
        pc_combatants = [
            c
            for c in combatants
            if isinstance(c, dict) and not _is_npc_combatant(c, npcs, active_players)
        ]
        if not npc_combatants:
            return {
                "npc_turn_summaries": [],
                "npc_prompt_block": "",
                "combat_tracker": tracker,
                "tool_invocations": [],
            }

        current_tick = max(1, int(tracker.get("tick", tracker.get("current_tick", 1))))
        min_pc_tick = min(
            (int(c.get("next_action_tick", current_tick)) for c in pc_combatants),
            default=current_tick,
        )
        horizon_tick = max(current_tick, min_pc_tick)

        ready_npcs = [
            c for c in npc_combatants if int(c.get("next_action_tick", 1)) <= horizon_tick
        ]
        ready_npcs.sort(
            key=lambda c: (int(c.get("next_action_tick", 1)), -int(c.get("ooda_score", 10)))
        )

        summaries: list[dict[str, Any]] = []
        invocations: list[dict[str, Any]] = []
        summary_lines: list[str] = []

        for npc_comb in ready_npcs[: max(1, max_npc_actions_per_turn)]:
            eid = str(npc_comb.get("entity_id") or "npc_combatant")
            dname = str(npc_comb.get("display_name") or eid)
            act_tick = max(1, int(npc_comb.get("next_action_tick", current_tick)))
            fatigue = int(npc_comb.get("fatigue_level", npc_comb.get("fatigue_tier", 0)))

            # Match profile in `active_npcs`
            matched_prof: dict[str, Any] = {}
            if eid in npcs:
                matched_prof = npcs[eid]
            else:
                for cand_id, prof in npcs.items():
                    if (
                        eid.lower() in cand_id.lower()
                        or dname.lower() in str(prof.get("name") or "").lower()
                    ):
                        matched_prof = prof
                        break

            skill_name, technique, control_c, skill_n, t_pool, c_pool, f_pool = (
                _select_npc_combat_technique_and_triad(matched_prof, npc_comb)
            )
            base_tc = 6
            neutral_b = 3
            eff_tc = max(2, base_tc - max(0, (t_pool - neutral_b) // 2))
            next_tick = act_tick + eff_tc

            t0 = time.perf_counter()
            roll_args = {
                "session_id": session_id,
                "actor_name": dname,
                "check_category": skill_name,
                "attribute_tn": control_c,
                "skill_rank": skill_n,
                "difficulty_mod": 0,
                "internal_mod": fatigue,
                "combat_technique": technique,
                "tempo_pool": t_pool,
                "control_pool": c_pool,
                "force_pool": f_pool,
                "note": f"Autonomous NPC Action Mode turn at Tick {act_tick}",
            }
            roll_res = roll_satya_d20_check.invoke(roll_args)
            dur_ms = int(max(1.0, (time.perf_counter() - t0) * 1000.0))

            stance_str = f"{technique.upper()} (T{t_pool}/C{c_pool}/F{f_pool})"
            tick_advance = max(0, act_tick - current_tick)
            tracker = advance_combat_tracker.invoke(
                {
                    "tick_delta": tick_advance if tick_advance > 0 else 1,
                    "entity_id": eid,
                    "active_stance": stance_str,
                    "next_action_tick": next_tick,
                    "session_id": session_id,
                }
            )
            current_tick = max(1, int(tracker.get("tick", tracker.get("current_tick", act_tick))))

            deg = str(
                roll_res.get("degree_of_success")
                or roll_res.get("outcome_tier")
                or "RESOLVED"
            )
            succ = int(roll_res.get("successes_counted", 0))
            margin = int(roll_res.get("signed_margin", roll_res.get("margin", 0)))
            tn_eff = int(roll_res.get("effective_tn", 31 - control_c))
            pairs = roll_res.get("rolled_pairs") or roll_res.get("dice_rolled_d20") or []

            npc_summary = {
                "entity_id": eid,
                "display_name": dname,
                "acted_on_tick": act_tick,
                "next_action_tick": next_tick,
                "effective_tc": eff_tc,
                "skill_name": skill_name,
                "combat_technique": technique,
                "triad": {"tempo": t_pool, "control": c_pool, "force": f_pool},
                "effective_tn": tn_eff,
                "rolled_pairs": pairs,
                "successes_counted": succ,
                "signed_margin": margin,
                "degree_of_success": deg,
            }
            summaries.append(npc_summary)
            summary_lines.append(
                f"- **{dname}** (`{eid}`) acted on **Tick {act_tick}** using `{technique}` "
                f"(`{skill_name}` N={skill_n}, C={control_c}, Triad `T={t_pool}/C={c_pool}/F={f_pool}`, `Effective TC = {eff_tc} ticks` -> `NextTick = {next_tick}`): "
                f"rolled `{pairs}` vs `TN_eff = {tn_eff}` -> **{deg}** (`{succ}` successes, signed margin `{margin:+d}`)."
            )
            invocations.append(
                {
                    "step_index": len(invocations),
                    "span_id": uuid.uuid4().hex[:16],
                    "mcp_tier": "TIER_1_SATYA_CORE",
                    "server_id": "satya-core",
                    "tool_name": "roll_satya_d20_check",
                    "arguments_json": f'{{"actor_name": "{dname}", "combat_technique": "{technique}", "triad": "T{t_pool}/C{c_pool}/F{f_pool}"}}',
                    "result_summary_json": f"{deg} (successes={succ}, margin={margin:+d}, next_tick={next_tick})",
                    "is_error": False,
                    "duration_ms": dur_ms,
                    "server_tier": "TIER_1_SATYA_CORE",
                    "server_name": "satya-core",
                    "arguments_redacted": f'{{"actor_name": "{dname}", "combat_technique": "{technique}"}}',
                    "result_summary": f"{deg} (successes={succ}, margin={margin:+d})",
                    "latency_ms": dur_ms,
                    "succeeded": True,
                }
            )

        if not summary_lines:
            return {
                "npc_turn_summaries": [],
                "npc_prompt_block": "",
                "combat_tracker": tracker,
                "tool_invocations": [],
            }

        prompt_block = (
            "[NPC ACTION MODE RESOLUTION — Server-Authoritative Count-Up Tick Rolls]\n"
            "The following NPC combatants reached their `Next Action Tick` on the Count-Up Clock and executed their techniques. "
            "Weave these exact dice outcomes and updated `NextTick` values into your Action Mode narration:\n"
            + "\n".join(summary_lines)
        )
        return {
            "npc_turn_summaries": summaries,
            "npc_prompt_block": prompt_block,
            "combat_tracker": tracker,
            "tool_invocations": invocations,
        }

    # Narrative / Scene Mode (`combat_active == False`): build active NPC reaction directive
    if not npcs:
        return {
            "npc_turn_summaries": [],
            "npc_prompt_block": "",
            "combat_tracker": tracker,
            "tool_invocations": [],
        }

    npc_roster_lines: list[str] = []
    for npc_id, prof in list(npcs.items())[:5]:
        name = str(prof.get("name") or npc_id)
        arch = str(prof.get("archetype") or "NPC")
        disp = str(prof.get("disposition") or "neutral")
        stance = str(prof.get("combat_stance") or "NEUTRAL")
        fat = int(prof.get("fatigue", 0))
        ws = int(prof.get("wound_shock", 0))
        skills_str = ", ".join(f"{k}:{v}" for k, v in list((prof.get("skills") or {}).items())[:3])
        npc_roster_lines.append(
            f"- **{name}** (`{npc_id}`, {arch}) — Disposition: `{disp}`, Stance: `{stance}`, "
            f"Fatigue: `{fat}`, Wound Shock: `{ws}`, Top Skills: `{skills_str}`"
        )

    narrative_directive = (
        "[SCENE NPC REACTION & INTENT DIRECTIVE]:\n"
        "Available NPCs in session state (have any NPC present in the current scene actively react to the players' actions, "
        "pursue their own agenda, and call `roll_satya_d20_check` / `update_npc_state` if an NPC contests a player or takes an uncertain action):\n"
        + "\n".join(npc_roster_lines)
    )
    return {
        "npc_turn_summaries": [],
        "npc_prompt_block": narrative_directive,
        "combat_tracker": tracker,
        "tool_invocations": [],
    }
