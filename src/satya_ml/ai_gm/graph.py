"""LangGraph state machine (`StateGraph`) for the Satya AI Game Master (AI GM).

Implements the 4-node workflow:
`sync_session_context` -> `gm_supervisor_node` <-> `tool_execution_node` -> `publish_gm_response`
with per-session checkpointing (`MemorySaver`) and automatic `AiGmTrajectoryEvent` telemetry.
"""

from __future__ import annotations

import json
import time
import uuid
from typing import Annotated, Any, Literal, TypedDict

import httpx
from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage, ToolMessage
from langgraph.checkpoint.memory import MemorySaver
from langgraph.graph import END, START, StateGraph
from langgraph.graph.message import add_messages

from satya_ml.ai_gm.models import SessionLlmConfig, build_gm_chat_model
from satya_ml.ai_gm.tools import (
    _gm_headers,
    _signaling_base_url,
    get_builtin_gm_tools,
    get_session_map,
    get_session_npcs,
    set_session_map,
    set_session_operating_mode,
)


class NpcState(TypedDict, total=False):
    """Per-NPC tactical and narrative state tracked in `AiGmSessionState`."""

    npc_id: str
    name: str
    archetype: str
    attribute_tns: dict[str, int]
    skills: dict[str, int]
    combat_stance: str
    fatigue: int
    wound_shock: int
    disposition: str
    secrets: list[str]


class TacticalMapState(TypedDict, total=False):
    """Active 2D battlemap state tracked in `AiGmSessionState`."""

    map_id: str
    map_image_url: str
    grid_width: int
    grid_height: int
    tokens: list[dict[str, Any]]


class AiGmSessionState(TypedDict, total=False):
    """LangGraph session state persisted across turns per `session_id`."""

    session_id: str
    session_mode: str
    operating_mode: Literal["autonomous", "co_pilot"]
    human_gm_user_id: str | None
    messages: Annotated[list[BaseMessage], add_messages]
    roster: list[dict[str, Any]]
    current_scene: str
    combat_active: bool
    combat_tick_clock: dict[str, int]
    active_npcs: dict[str, NpcState]
    active_map: TacticalMapState | None
    trigger_event: dict[str, Any]
    tool_invocations: list[dict[str, Any]]
    turn_started_ms: float
    final_reply: str
    final_channel: str


AUTONOMOUS_SYSTEM_PROMPT = """You are the Satya AI Game Master (Autonomous GM Mode).
You are the sole Game Master for this Satya d20 TTRPG session.
Rules:
1. Never invent dice outcomes in prose. Always call `roll_satya_d20_check` for uncertain actions.
2. Satya d20 is a ROLL-UNDER system (`kept_die <= effective_tn` succeeds; 1 or margin >= +5 is CRITICAL_SUCCESS).
3. Use `lookup_npc_profile`, `query_satya_rules`, and `query_campaign_lore` to ground narration.
4. Keep responses atmospheric, concise (2-4 sentences), and end with a clear prompt for player action."""

CO_PILOT_SYSTEM_PROMPT = """You are the Satya AI GM Assistant operating in CO-PILOT MODE alongside a Human Game Master.
Rules:
1. NEVER address players publicly on `IN_CHARACTER` unless explicitly instructed.
2. Direct all rules lookups, NPC drafts, roll breakdowns, and tactical suggestions privately to the Human GM via `GM_WHISPER`.
3. Be concise, tactical, and cite Satya d20 rules accurately."""


def _classify_tool_tier(tool_name: str) -> tuple[str, str]:
    if tool_name in {
        "generate_tactical_map",
        "move_map_token",
        "inspect_tactical_map",
        "roll_satya_d20_check",
        "whisper_to_player",
        "narrate_to_session",
    }:
        return "TIER_1_SATYA_CORE", "satya-core"
    if tool_name in {"lookup_npc_profile", "update_npc_state", "query_satya_rules"}:
        return "TIER_2_RULESET_ENGINE", "satya-d20-rules"
    return "TIER_3_LORE_PROVIDER", "satya-campaign-lore"


def fetch_session_context_from_rust(session_id: str) -> dict[str, Any] | None:
    """Query `GET /api/gm/sessions/:id/context` from `satya-signaling` when reachable."""
    try:
        with httpx.Client(timeout=2.0) as client:
            resp = client.get(
                f"{_signaling_base_url()}/api/gm/sessions/{session_id}/context",
                headers=_gm_headers(),
            )
            if resp.status_code == 200:
                return dict(resp.json())
    except Exception:
        pass
    return None


def report_trajectory_to_rust(session_id: str, payload: dict[str, Any]) -> None:
    """Emit an `AiGmTrajectoryEvent` to `POST /api/gm/sessions/:id/trajectory` (Topic 8)."""
    try:
        with httpx.Client(timeout=2.0) as client:
            client.post(
                f"{_signaling_base_url()}/api/gm/sessions/{session_id}/trajectory",
                headers=_gm_headers(),
                json=payload,
            )
    except Exception:
        pass


def build_ai_gm_graph(
    llm_config: SessionLlmConfig | None = None,
    *,
    chat_model: Any | None = None,
    tools: list[Any] | None = None,
    checkpointer: Any | None = None,
) -> Any:
    """Build and compile the 4-node Satya AI GM LangGraph `StateGraph`."""
    cfg = llm_config or SessionLlmConfig()
    active_tools = list(tools) if tools is not None else get_builtin_gm_tools()
    tool_map = {t.name: t for t in active_tools}

    def sync_session_context(state: AiGmSessionState) -> dict[str, Any]:
        session_id = state.get("session_id", "default")
        trigger = state.get("trigger_event") or {}
        ctx = fetch_session_context_from_rust(session_id)

        roster = list(state.get("roster") or [])
        operating_mode: Literal["autonomous", "co_pilot"] = state.get("operating_mode", "autonomous")
        human_gm_user_id = state.get("human_gm_user_id") or trigger.get("human_gm_user_id")
        active_map = state.get("active_map") or get_session_map(session_id)

        if ctx is not None:
            roster = ctx.get("roster", roster)
            raw_mode = str(ctx.get("operating_mode", "")).upper()
            if raw_mode == "CO_PILOT":
                operating_mode = "co_pilot"
            elif raw_mode == "AUTONOMOUS":
                operating_mode = "autonomous"
            human_gm_user_id = ctx.get("human_gm_user_id", human_gm_user_id)
            if ctx.get("active_map"):
                active_map = ctx["active_map"]
                set_session_map(session_id, active_map)
        elif trigger.get("operating_mode"):
            raw_mode = str(trigger["operating_mode"]).lower()
            operating_mode = "co_pilot" if "co_pilot" in raw_mode else "autonomous"

        set_session_operating_mode(session_id, operating_mode, human_gm_user_id)
        npcs = get_session_npcs(session_id)

        new_messages: list[BaseMessage] = []
        existing_msgs = state.get("messages") or []
        if not existing_msgs:
            sys_prompt = (
                CO_PILOT_SYSTEM_PROMPT if operating_mode == "co_pilot" else AUTONOMOUS_SYSTEM_PROMPT
            )
            new_messages.append(SystemMessage(content=sys_prompt))

        if trigger:
            actor_name = (trigger.get("actor") or {}).get("display_name", "Player")
            content = trigger.get("content") or ""
            roll_summary = trigger.get("roll_summary")
            if roll_summary:
                prompt_text = (
                    f"[Session={session_id} | Mode={operating_mode}] {actor_name} rolled "
                    f"{roll_summary}. {content}".strip()
                )
            else:
                prompt_text = f"[Session={session_id} | Mode={operating_mode}] {actor_name}: {content}"
            new_messages.append(HumanMessage(content=prompt_text))

        return {
            "session_id": session_id,
            "operating_mode": operating_mode,
            "human_gm_user_id": human_gm_user_id,
            "roster": roster,
            "active_npcs": npcs,  # type: ignore[typeddict-item]
            "active_map": active_map,  # type: ignore[typeddict-item]
            "messages": new_messages,
            "tool_invocations": list(state.get("tool_invocations") or []),
            "turn_started_ms": state.get("turn_started_ms") or (time.perf_counter() * 1000.0),
        }

    def gm_supervisor_node(state: AiGmSessionState) -> dict[str, Any]:
        model = chat_model if chat_model is not None else build_gm_chat_model(cfg)
        if hasattr(model, "bind_tools") and active_tools:
            bound_model = model.bind_tools(active_tools)
        else:
            bound_model = model

        response = bound_model.invoke(state.get("messages") or [])
        if not isinstance(response, BaseMessage):
            response = AIMessage(content=str(response))
        return {"messages": [response]}

    def tool_execution_node(state: AiGmSessionState) -> dict[str, Any]:
        messages = state.get("messages") or []
        if not messages:
            return {}
        last_msg = messages[-1]
        tool_calls = getattr(last_msg, "tool_calls", None) or []
        if not tool_calls:
            return {}

        session_id = state.get("session_id", "default")
        tool_messages: list[ToolMessage] = []
        invocations = list(state.get("tool_invocations") or [])

        for tc in tool_calls:
            t_name = tc.get("name", "")
            t_args = dict(tc.get("args") or {})
            t_id = tc.get("id") or f"call-{uuid.uuid4().hex[:8]}"
            if "session_id" not in t_args and t_name in {
                "lookup_npc_profile",
                "update_npc_state",
                "generate_tactical_map",
                "move_map_token",
                "inspect_tactical_map",
                "roll_satya_d20_check",
                "whisper_to_player",
                "narrate_to_session",
                "query_campaign_lore",
            }:
                t_args["session_id"] = session_id

            started = time.perf_counter()
            is_err = False
            if t_name in tool_map:
                try:
                    raw_res = tool_map[t_name].invoke(t_args)
                    res_str = (
                        json.dumps(raw_res) if isinstance(raw_res, (dict, list)) else str(raw_res)
                    )
                except Exception as exc:
                    is_err = True
                    res_str = json.dumps({"error": str(exc)})
            else:
                is_err = True
                res_str = json.dumps({"error": f"Unknown tool '{t_name}'"})

            duration_ms = int(max(1.0, (time.perf_counter() - started) * 1000.0))
            tier, server_name = _classify_tool_tier(t_name)
            invocations.append(
                {
                    "server_tier": tier,
                    "server_name": server_name,
                    "tool_name": t_name,
                    "arguments_redacted": json.dumps(t_args),
                    "result_summary": res_str[:500],
                    "latency_ms": duration_ms,
                    "succeeded": not is_err,
                }
            )
            tool_messages.append(ToolMessage(content=res_str, tool_call_id=t_id, name=t_name))

        return {
            "messages": tool_messages,
            "tool_invocations": invocations,
            "active_npcs": get_session_npcs(session_id),  # type: ignore[typeddict-item]
            "active_map": get_session_map(session_id),  # type: ignore[typeddict-item]
        }

    def route_after_supervisor(
        state: AiGmSessionState,
    ) -> Literal["tool_execution_node", "publish_gm_response"]:
        messages = state.get("messages") or []
        if messages:
            last_msg = messages[-1]
            if getattr(last_msg, "tool_calls", None):
                return "tool_execution_node"
        return "publish_gm_response"

    def publish_gm_response(state: AiGmSessionState) -> dict[str, Any]:
        session_id = state.get("session_id", "default")
        operating_mode = state.get("operating_mode", "autonomous")
        human_gm_id = state.get("human_gm_user_id")
        messages = state.get("messages") or []
        reply_text = ""
        for msg in reversed(messages):
            if isinstance(msg, AIMessage) and msg.content:
                reply_text = (
                    msg.content if isinstance(msg.content, str) else json.dumps(msg.content)
                )
                break

        channel = "GM_WHISPER" if operating_mode == "co_pilot" else "IN_CHARACTER"
        invocations = list(state.get("tool_invocations") or [])
        already_narrated = any(
            inv.get("tool_name") in {"narrate_to_session", "whisper_to_player"}
            for inv in invocations
        )

        if reply_text and not already_narrated:
            visibility = "PRIVATE" if operating_mode == "co_pilot" else "PUBLIC"
            recipients = [human_gm_id] if (operating_mode == "co_pilot" and human_gm_id) else []
            try:
                with httpx.Client(timeout=2.5) as client:
                    client.post(
                        f"{_signaling_base_url()}/api/gm/sessions/{session_id}/chat",
                        headers=_gm_headers(),
                        json={
                            "visibility": visibility,
                            "recipient_user_ids": recipients,
                            "channel_context": "GM_NARRATION",
                            "body": reply_text,
                            "model_id": cfg.model_id,
                            "speaker_override": None,
                        },
                    )
            except Exception:
                pass

        started_ms = state.get("turn_started_ms") or (time.perf_counter() * 1000.0)
        latency_ms = int(max(1.0, (time.perf_counter() * 1000.0) - started_ms))
        trigger = state.get("trigger_event") or {}
        default_actor = {
            "user_id": "u-player-unknown",
            "username": "player",
            "display_name": "Player",
            "participant_kind": "HUMAN",
            "role": "PLAYER",
            "character_id": None,
            "ai_agent_info": None,
        }
        raw_actor = trigger.get("actor") or default_actor
        trigger_actor = {**default_actor, **raw_actor}

        trajectory_payload = {
            "turn_id": f"turn-{uuid.uuid4().hex[:10]}",
            "trace_id": uuid.uuid4().hex,
            "operating_mode": "CO_PILOT" if operating_mode == "co_pilot" else "AUTONOMOUS",
            "trigger_actor": trigger_actor,
            "trigger_event_id": str(trigger.get("event_id") or f"ev-{uuid.uuid4().hex[:8]}"),
            "trigger_summary": str(trigger.get("content") or "AI GM Turn")[:240],
            "llm_provider": cfg.provider,
            "reasoning_model_id": cfg.model_id,
            "vision_model_id": cfg.vision_model_id,
            "ruleset_engine_id": "satya-d20",
            "tool_invocations": invocations,
            "rag_sources_used": [
                inv["tool_name"]
                for inv in invocations
                if inv["tool_name"] in {"lookup_npc_profile", "query_satya_rules", "query_campaign_lore"}
            ],
            "emitted_event_ids": [],
            "prompt_tokens": max(32, len(str(trigger)) // 4),
            "completion_tokens": max(16, len(reply_text) // 4),
            "cached_tokens": 0,
            "total_turn_latency_ms": latency_ms,
            "turn_status": "COMPLETED",
        }
        report_trajectory_to_rust(session_id, trajectory_payload)

        return {
            "final_reply": reply_text,
            "final_channel": channel,
        }

    workflow = StateGraph(AiGmSessionState)
    workflow.add_node("sync_session_context", sync_session_context)
    workflow.add_node("gm_supervisor_node", gm_supervisor_node)
    workflow.add_node("tool_execution_node", tool_execution_node)
    workflow.add_node("publish_gm_response", publish_gm_response)

    workflow.add_edge(START, "sync_session_context")
    workflow.add_edge("sync_session_context", "gm_supervisor_node")
    workflow.add_conditional_edges(
        "gm_supervisor_node",
        route_after_supervisor,
        {
            "tool_execution_node": "tool_execution_node",
            "publish_gm_response": "publish_gm_response",
        },
    )
    workflow.add_edge("tool_execution_node", "gm_supervisor_node")
    workflow.add_edge("publish_gm_response", END)

    memory = checkpointer if checkpointer is not None else MemorySaver()
    return workflow.compile(checkpointer=memory)
