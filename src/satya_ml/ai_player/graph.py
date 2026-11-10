"""LangGraph state machine (`StateGraph`) for the Satya AI Player Agent (`satya_ml.ai_player`).

Implements the 4-node player workflow:
`sync_player_context` -> `player_decision_node` <-> `player_tool_node` -> `publish_player_turn`
with isolated per-player checkpointing (`MemorySaver` keyed by `f"{session_id}:player:{player_user_id}"`)
and strict player-only visibility boundaries (no `GM_ONLY` documents, GM memories, or NPC secrets).
"""

from __future__ import annotations

import json
import logging
import time
import uuid
from typing import Annotated, Any, Literal, TypedDict

import httpx
from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage, ToolMessage
from langgraph.checkpoint.memory import MemorySaver
from langgraph.graph import END, START, StateGraph
from langgraph.graph.message import add_messages

from satya_ml.ai_gm.graph import (
    _count_turn_tool_rounds,
    _extract_ai_message_text,
    report_trajectory_to_rust,
)
from satya_ml.ai_gm.models import SessionLlmConfig, build_gm_chat_model
from satya_ml.ai_gm.telemetry import (
    ensure_turn_context,
    get_current_emitted_event_ids,
    get_current_trace_id,
    get_current_turn_logs,
    record_emitted_event_id,
    redact_sensitive_text,
    span_scope,
)
from satya_ml.ai_gm.tools import (
    _gm_headers,
    _signaling_base_url,
    compute_character_tactical_envelope,
)
from satya_ml.ai_player.tools import (
    fetch_player_context_from_signaling,
    get_builtin_player_tools,
    get_player_session_context,
    set_player_session_context,
)

logger = logging.getLogger("satya_ml.ai_player.graph")


class AiPlayerSessionState(TypedDict, total=False):
    """Isolated LangGraph state persisted per `(session_id, player_user_id)`."""

    session_id: str
    player_user_id: str
    player_display_name: str
    persona_prompt: str
    messages: Annotated[list[BaseMessage], add_messages]
    roster: list[dict[str, Any]]
    character_sheet: dict[str, Any]
    tactical_envelope: dict[str, Any]
    combat_tracker: dict[str, Any]
    active_map: dict[str, Any] | None
    trigger_event: dict[str, Any]
    tool_invocations: list[dict[str, Any]]
    roll_results: list[dict[str, Any]]
    turn_started_ms: float
    final_reply: str
    final_channel: str
    trace_id: str
    turn_logs: list[dict[str, Any]]


AI_PLAYER_SYSTEM_PROMPT = """You are an autonomous Satya d20 TTRPG Player Character (AI Player Agent).
You play ONLY your single bound Player Character — you are NOT the Game Master.
Rules:
1. Embody your character in first person ("I...") or clear character voice, guided by your character sheet's Archetype, Loadout, Skills, and 7 Bipolar Psyche Traits.
2. NEVER narrate outcomes for the world or control NPCs — declare what your character attempts, speaks, or investigates, and let the GM adjudicate the world's reaction.
3. Whenever taking an uncertain action or acting in Combat Action Mode (`combat_active = true`), call `roll_player_d20_action` first using one of your character's skills (`attribute_tn` and `skill_rank` from your character sheet).
4. In Combat Action Mode (`[ACTION MODE · Window W · Tick T]`):
   - Choose a concrete Combat Technique (`snap_shot`, `combat_sighted_shot`, `cover_peek`, `thrust`, `punch`, `grab`) and allocate your `Tempo / Control / Force` (`tempo_pool`, `control_pool`, `force_pool`) so `T + C + F <= P_eff` and each pool respects `T_max / C_max / F_max`.
   - Call `roll_player_d20_action` with `combat_technique`, `tempo_pool`, `control_pool`, and `force_pool`, then state your tactical action and roll result concisely in Markdown.
5. Keep your turn declarations vivid, cooperative with fellow party members, and concise (1–2 short paragraphs)."""


def _format_player_context_header(ctx: dict[str, Any]) -> str:
    sheet = ctx.get("character_sheet") or {}
    env = ctx.get("tactical_envelope") or compute_character_tactical_envelope(sheet)
    cname = sheet.get("character_name") or ctx.get("player_display_name") or "Kaelen Voss"
    role = sheet.get("callsign_or_role") or sheet.get("archetype") or "Operative"
    skills = sheet.get("skills") or {}
    skills_str = ", ".join(f"{k}(N={v})" for k, v in list(skills.items())[:5])
    attr_tns = sheet.get("attribute_tns") or {}
    attrs_str = ", ".join(f"{k}={v}" for k, v in attr_tns.items())
    psyche = sheet.get("psyche_traits") or {}
    psyche_str = ", ".join(f"{k}={v:+d}" for k, v in list(psyche.items())[:4])

    parts = [
        f"My Character: {cname} ({role})",
        f"Attrs: {attrs_str}" if attrs_str else "",
        f"Skills: {skills_str}" if skills_str else "",
        f"Psyche: {psyche_str}" if psyche_str else "",
        (
            f"Tactical Pool: P_eff={env['p_eff']}/{env['p_base']} "
            f"(Neutral B={env['neutral_baseline']}, Caps T={env['max_tempo']}/C={env['max_control']}/F={env['max_force']}, "
            f"OODA={env['ooda']} -> StartTick={env['starting_tick']})"
        ),
    ]
    tracker = ctx.get("combat_tracker") or {}
    if tracker.get("combat_active") or tracker.get("is_active"):
        rnd = tracker.get("round", tracker.get("combat_round", 1))
        tck = tracker.get("tick", tracker.get("current_tick", 1))
        parts.append(f"Combat Action Mode ACTIVE: Window {rnd} · Tick {tck}")

    persona = str(ctx.get("persona_prompt") or "").strip()
    if persona:
        parts.append(f"Playstyle Directive: {persona}")

    clean_parts = [p for p in parts if p]
    return "[AI Player Character & Public Scene Index]: " + " || ".join(clean_parts)


def build_ai_player_graph(
    llm_config: SessionLlmConfig | None = None,
    *,
    chat_model: Any | None = None,
    tools: list[Any] | None = None,
    checkpointer: Any | None = None,
) -> Any:
    """Build and compile the isolated 4-node Satya AI Player LangGraph `StateGraph`."""
    cfg = llm_config or SessionLlmConfig()
    active_tools = list(tools) if tools is not None else get_builtin_player_tools()
    tool_map = {t.name: t for t in active_tools}

    if chat_model is not None:
        base_model = chat_model
    else:
        base_model = build_gm_chat_model(cfg, for_vision=False)

    model_with_tools = (
        base_model.bind_tools(active_tools)
        if hasattr(base_model, "bind_tools")
        else base_model
    )

    def sync_player_context(state: AiPlayerSessionState) -> dict[str, Any]:
        session_id = state.get("session_id", "default")
        trigger = state.get("trigger_event") or {}
        player_user_id = str(
            state.get("player_user_id")
            or trigger.get("player_user_id")
            or "ai-player-1"
        ).strip()
        ensure_turn_context(session_id)

        with span_scope("sync_player_context"):
            remote_ctx = fetch_player_context_from_signaling(session_id, player_user_id)
            if remote_ctx is None:
                fallback_raw = {
                    "session_id": session_id,
                    "player_user_id": player_user_id,
                    "player_display_name": state.get("player_display_name") or trigger.get("player_display_name"),
                    "persona_prompt": state.get("persona_prompt") or trigger.get("persona_prompt") or "",
                    "roster": state.get("roster") or trigger.get("roster") or [],
                    "character_sheet": state.get("character_sheet") or trigger.get("character_sheet"),
                    "combat_tracker": state.get("combat_tracker") or trigger.get("combat_tracker"),
                    "active_map": state.get("active_map") or trigger.get("active_map"),
                    "recent_messages": trigger.get("recent_messages") or [],
                }
                player_ctx = set_player_session_context(session_id, player_user_id, fallback_raw)
            else:
                if trigger.get("persona_prompt"):
                    remote_ctx["persona_prompt"] = trigger["persona_prompt"]
                if trigger.get("player_display_name"):
                    remote_ctx["player_display_name"] = trigger["player_display_name"]
                if isinstance(trigger.get("character_sheet"), dict):
                    remote_ctx["character_sheet"] = trigger["character_sheet"]
                player_ctx = set_player_session_context(session_id, player_user_id, remote_ctx)

            existing_msgs = list(state.get("messages") or [])
            new_messages: list[BaseMessage] = []
            if not existing_msgs:
                new_messages.append(SystemMessage(content=AI_PLAYER_SYSTEM_PROMPT))

            scene_prompt = str(
                trigger.get("content")
                or trigger.get("scene_prompt")
                or "The Game Master awaits your character's next action or declaration."
            ).strip()
            header = _format_player_context_header(player_ctx)
            full_prompt = (
                f"{header}\n\n"
                f"[Current Scene / Turn Prompt for {player_ctx['player_display_name']}]:\n{scene_prompt}"
            )
            new_messages.append(
                HumanMessage(content=full_prompt, id=f"player-hm-{uuid.uuid4().hex[:10]}")
            )

            return {
                "session_id": session_id,
                "player_user_id": player_user_id,
                "player_display_name": player_ctx["player_display_name"],
                "persona_prompt": player_ctx.get("persona_prompt", ""),
                "roster": player_ctx["roster"],
                "character_sheet": player_ctx["character_sheet"],
                "tactical_envelope": player_ctx["tactical_envelope"],
                "combat_tracker": player_ctx["combat_tracker"],
                "active_map": player_ctx["active_map"],
                "messages": new_messages,
                "tool_invocations": [],
                "roll_results": [],
                "turn_started_ms": time.perf_counter() * 1000.0,
            }

    def player_decision_node(state: AiPlayerSessionState) -> dict[str, Any]:
        session_id = state.get("session_id", "default")
        ensure_turn_context(session_id)
        raw_messages = list(state.get("messages") or [])
        messages = [
            m
            for m in raw_messages
            if not (
                isinstance(m, AIMessage)
                and not (getattr(m, "tool_calls", None) or [])
                and not _extract_ai_message_text(m)
            )
        ]
        if len(messages) > 14:
            sys_msgs = [m for m in messages[:1] if isinstance(m, SystemMessage)]
            recent_msgs = [m for m in messages[-10:] if not isinstance(m, SystemMessage)]
            messages = sys_msgs + recent_msgs

        tool_rounds = _count_turn_tool_rounds(messages)
        with span_scope("player_decision_node"):
            if tool_rounds >= 3:
                synth_msgs = list(messages) + [
                    HumanMessage(
                        content=(
                            "[SYSTEM: You have completed your dice rolls and tool checks for this turn. "
                            "Now write your character's in-character action/dialogue response in Markdown.]"
                        )
                    )
                ]
                response = base_model.invoke(synth_msgs)
            else:
                response = model_with_tools.invoke(messages)
                if (
                    tool_rounds >= 1
                    and not (getattr(response, "tool_calls", None) or [])
                    and not _extract_ai_message_text(response)
                ):
                    synth_msgs = list(messages) + [
                        HumanMessage(
                            content=(
                                "[SYSTEM: Write your character's in-character action and dialogue based on the tool/roll results above.]"
                            )
                        )
                    ]
                    response = base_model.invoke(synth_msgs)
            return {"messages": [response]}

    def player_tool_node(state: AiPlayerSessionState) -> dict[str, Any]:
        messages = state.get("messages") or []
        if not messages:
            return {}
        last_msg = messages[-1]
        tool_calls = getattr(last_msg, "tool_calls", None) or []
        if not tool_calls:
            return {}

        session_id = state.get("session_id", "default")
        player_user_id = state.get("player_user_id", "ai-player-1")
        ensure_turn_context(session_id)
        tool_messages: list[ToolMessage] = []
        invocations = list(state.get("tool_invocations") or [])
        roll_results = list(state.get("roll_results") or [])

        for tc in tool_calls:
            t_name = tc.get("name", "")
            t_args = dict(tc.get("args") or {})
            t_id = tc.get("id") or f"pcall-{uuid.uuid4().hex[:8]}"
            if t_name in {
                "read_my_character_sheet",
                "inspect_public_tactical_map",
                "roll_player_d20_action",
                "send_player_chat",
            }:
                t_args.setdefault("session_id", session_id)
                t_args.setdefault("player_user_id", player_user_id)

            redacted_args_json = redact_sensitive_text(json.dumps(t_args))
            with span_scope(f"mcp.player_tool.{t_name}", new_lineage_id=f"ptool-{t_id}") as span_id:
                started = time.perf_counter()
                is_err = False
                raw_res: Any = None
                if t_name in tool_map:
                    try:
                        raw_res = tool_map[t_name].invoke(t_args)
                        res_str = (
                            json.dumps(raw_res)
                            if isinstance(raw_res, (dict, list))
                            else str(raw_res)
                        )
                    except Exception as exc:
                        is_err = True
                        res_str = json.dumps({"error": str(exc)})
                else:
                    is_err = True
                    res_str = json.dumps({"error": f"Unknown player tool '{t_name}'"})

                if t_name == "roll_player_d20_action" and isinstance(raw_res, dict) and not is_err:
                    roll_results.append(raw_res)

                duration_ms = int(max(1.0, (time.perf_counter() - started) * 1000.0))
                summary_str = redact_sensitive_text(res_str[:500])
                invocations.append(
                    {
                        "step_index": len(invocations),
                        "span_id": span_id,
                        "mcp_tier": "TIER_1_SATYA_CORE" if t_name != "query_satya_rules" else "TIER_2_RULESET_ENGINE",
                        "server_id": "satya-player-agent",
                        "tool_name": t_name,
                        "arguments_json": redacted_args_json,
                        "result_summary_json": summary_str,
                        "is_error": is_err,
                        "duration_ms": duration_ms,
                    }
                )
                tool_messages.append(ToolMessage(content=res_str, tool_call_id=t_id, name=t_name))

        updated_ctx = get_player_session_context(session_id, player_user_id)
        return {
            "messages": tool_messages,
            "tool_invocations": invocations,
            "roll_results": roll_results,
            "combat_tracker": updated_ctx.get("combat_tracker", {}),
        }

    def route_after_decision(
        state: AiPlayerSessionState,
    ) -> Literal["player_tool_node", "publish_player_turn"]:
        messages = state.get("messages") or []
        if messages:
            last_msg = messages[-1]
            if getattr(last_msg, "tool_calls", None):
                return "player_tool_node"
        return "publish_player_turn"

    def publish_player_turn(state: AiPlayerSessionState) -> dict[str, Any]:
        session_id = state.get("session_id", "default")
        player_user_id = state.get("player_user_id", "ai-player-1")
        display_name = state.get("player_display_name", "Kaelen Voss (AI Player)")
        ensure_turn_context(session_id)

        messages = state.get("messages") or []
        reply_text = ""
        for msg in reversed(messages):
            if isinstance(msg, HumanMessage):
                break
            if isinstance(msg, AIMessage):
                candidate = _extract_ai_message_text(msg)
                if candidate:
                    reply_text = candidate
                    break

        roll_results = list(state.get("roll_results") or [])
        if not reply_text and roll_results:
            last_r = roll_results[-1]
            cat = last_r.get("check_category", "skill")
            tech = last_r.get("combat_technique")
            deg = last_r.get("degree_of_success", "RESOLVED")
            succ = last_r.get("successes_counted", 0)
            if tech:
                triad = last_r.get("triad") or {}
                reply_text = (
                    f"**{display_name}** executes `{tech}` (`{cat}`, "
                    f"Triad `T={triad.get('tempo', 4)}/C={triad.get('control', 4)}/F={triad.get('force', 4)}`) "
                    f"-> **{deg}** (`{succ}` successes)."
                )
            else:
                reply_text = f"**{display_name}** attempts `{cat}` -> **{deg}** (`{succ}` successes)."

        invocations = list(state.get("tool_invocations") or [])
        already_sent_chat = any(
            inv.get("tool_name") == "send_player_chat" and not inv.get("is_error")
            for inv in invocations
        )
        trigger = state.get("trigger_event") or {}
        auto_trigger_gm = bool(trigger.get("auto_trigger_gm", True))

        with span_scope("publish_player_turn"):
            if reply_text and not already_sent_chat:
                try:
                    with httpx.Client(timeout=3.0) as client:
                        resp = client.post(
                            f"{_signaling_base_url()}/api/player-ai/sessions/{session_id}/chat",
                            headers=_gm_headers(),
                            json={
                                "user_id": player_user_id,
                                "display_name": display_name,
                                "visibility": "PUBLIC",
                                "recipient_user_ids": [],
                                "channel_context": "IN_CHARACTER",
                                "body": reply_text,
                                "auto_trigger_gm": auto_trigger_gm,
                            },
                        )
                        if resp.status_code == 200:
                            try:
                                r_json = resp.json()
                                if isinstance(r_json, dict) and r_json.get("message_id"):
                                    record_emitted_event_id(str(r_json["message_id"]))
                            except Exception:
                                pass
                except Exception as exc:
                    logger.debug(
                        "[publish_player_turn] Offline or unreachable satya-signaling chat endpoint: %s",
                        exc,
                    )

            started_ms = state.get("turn_started_ms") or (time.perf_counter() * 1000.0)
            latency_ms = int(max(1.0, (time.perf_counter() * 1000.0) - started_ms))
            active_trace_id = get_current_trace_id() or uuid.uuid4().hex
            turn_logs = get_current_turn_logs()

            trajectory_payload = {
                "turn_id": f"pturn-{uuid.uuid4().hex[:10]}",
                "trace_id": active_trace_id,
                "operating_mode": "AUTONOMOUS",
                "trigger_actor": {
                    "user_id": player_user_id,
                    "username": player_user_id,
                    "display_name": display_name,
                    "participant_kind": "AI_AGENT",
                    "role": "PLAYER",
                    "character_id": (state.get("character_sheet") or {}).get("character_id"),
                    "ai_agent_info": None,
                },
                "trigger_event_id": str(trigger.get("event_id") or f"pev-{uuid.uuid4().hex[:8]}"),
                "trigger_summary": f"[AI Player {display_name}] {str(trigger.get('content') or 'Player Turn')[:180]}",
                "llm_provider": cfg.provider,
                "reasoning_model_id": cfg.model_id,
                "vision_model_id": cfg.vision_model_id,
                "ruleset_engine_id": "satya-d20-player",
                "tool_invocations": invocations,
                "rag_sources_used": [
                    inv["tool_name"]
                    for inv in invocations
                    if inv["tool_name"]
                    in {"read_my_character_sheet", "inspect_public_tactical_map", "query_satya_rules"}
                ],
                "emitted_event_ids": get_current_emitted_event_ids(),
                "prompt_tokens": max(32, len(str(trigger)) // 4),
                "completion_tokens": max(16, len(reply_text) // 4),
                "cached_tokens": 0,
                "total_turn_latency_ms": latency_ms,
                "turn_status": "COMPLETED",
                "turn_logs": turn_logs,
            }
            report_trajectory_to_rust(session_id, trajectory_payload)

            return {
                "final_reply": reply_text,
                "final_channel": "IN_CHARACTER",
                "trace_id": active_trace_id,
                "turn_logs": turn_logs,
            }

    workflow = StateGraph(AiPlayerSessionState)
    workflow.add_node("sync_player_context", sync_player_context)
    workflow.add_node("player_decision_node", player_decision_node)
    workflow.add_node("player_tool_node", player_tool_node)
    workflow.add_node("publish_player_turn", publish_player_turn)

    workflow.add_edge(START, "sync_player_context")
    workflow.add_edge("sync_player_context", "player_decision_node")
    workflow.add_conditional_edges(
        "player_decision_node",
        route_after_decision,
        {
            "player_tool_node": "player_tool_node",
            "publish_player_turn": "publish_player_turn",
        },
    )
    workflow.add_edge("player_tool_node", "player_decision_node")
    workflow.add_edge("publish_player_turn", END)

    memory = checkpointer if checkpointer is not None else MemorySaver()
    return workflow.compile(checkpointer=memory)
