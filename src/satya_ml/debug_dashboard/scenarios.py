"""Standalone offline seeding, deterministic mock LLM, and multi-turn integration scenario runner."""

from __future__ import annotations

import argparse
from collections.abc import Mapping, Sequence
from datetime import datetime, timezone
import json
from pathlib import Path
import sys
import time
from typing import Any, Literal
import uuid

from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage, ToolMessage
from pydantic import BaseModel, Field

from satya_ml.ai_gm.models import SessionLlmConfig
from satya_ml.ai_gm.telemetry import turn_telemetry_scope
from satya_ml.campaign_config import get_pack_default_ai_player
from satya_ml.debug_dashboard.hub import GRAPH_HUB, GraphExecutionHub, TurnRunRecord, run_observed_graph
from satya_ml.session_store import GM_SESSION_REGISTRY, PLAYER_SESSION_REGISTRY
from satya_ml.signaling_client import load_ai_gm_toml_config, offline_signaling_scope


class ScriptedDebugChatModel:
    """Deterministic offline chat model (`provider='mock'`) for zero-LLM graph & UI testing."""

    def __init__(
        self,
        *,
        mock_reply: str | None = None,
        mock_tool_calls: list[dict[str, Any]] | None = None,
        bound_tool_names: frozenset[str] = frozenset(),
    ) -> None:
        self._mock_reply = mock_reply
        self._mock_tool_calls = list(mock_tool_calls or [])
        self._bound_tool_names = bound_tool_names

    def bind_tools(self, tools: Sequence[Any], **_kwargs: Any) -> ScriptedDebugChatModel:
        names = frozenset(
            str(getattr(t, "name", "") or "")
            for t in tools
            if getattr(t, "name", None)
        )
        return ScriptedDebugChatModel(
            mock_reply=self._mock_reply,
            mock_tool_calls=self._mock_tool_calls,
            bound_tool_names=names,
        )

    def invoke(self, messages: Sequence[BaseMessage], **_kwargs: Any) -> AIMessage:
        trailing_tools: list[ToolMessage] = []
        last_human_text = ""
        system_text = ""
        for msg in messages:
            if isinstance(msg, SystemMessage):
                system_text += str(msg.content or "") + "\n"
            elif isinstance(msg, HumanMessage):
                last_human_text = str(msg.content or "")
                trailing_tools = []
            elif isinstance(msg, ToolMessage):
                trailing_tools.append(msg)

        is_player = (
            "AI Player Agent" in system_text
            or "roll_player_d20_action" in self._bound_tool_names
        )
        is_ooc_rules = "[OUT_OF_CHARACTER RULES" in last_human_text
        is_ooc_table = "[OUT_OF_CHARACTER TABLE" in last_human_text
        is_synth_prompt = "[SYSTEM:" in last_human_text

        # Round 0: emit a single relevant tool call if tools are bound and no tool result is present yet.
        if not trailing_tools and not is_synth_prompt and self._bound_tool_names:
            if self._mock_tool_calls:
                calls = [
                    {
                        "name": str(tc.get("name") or ""),
                        "args": dict(tc.get("args") or {}),
                        "id": str(tc.get("id") or f"mock-tc-{idx}-{uuid.uuid4().hex[:6]}"),
                    }
                    for idx, tc in enumerate(self._mock_tool_calls)
                    if str(tc.get("name") or "") in self._bound_tool_names
                ]
                if calls:
                    return AIMessage(content="", tool_calls=calls)

            if is_player and "roll_player_d20_action" in self._bound_tool_names:
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": "roll_player_d20_action",
                            "args": {
                                "check_category": "INVESTIGATION",
                                "attribute_tn": 13,
                                "skill_rank": 2,
                                "note": "Inspect immediate hazards and clues",
                            },
                            "id": f"mock-ptc-{uuid.uuid4().hex[:6]}",
                        }
                    ],
                )
            if is_ooc_rules and "query_satya_rules" in self._bound_tool_names:
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": "query_satya_rules",
                            "args": {"query": "2d20 cascade tempo control force"},
                            "id": f"mock-rules-{uuid.uuid4().hex[:6]}",
                        }
                    ],
                )
            if not is_player and not is_ooc_table and "roll_satya_d20_check" in self._bound_tool_names:
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": "roll_satya_d20_check",
                            "args": {
                                "actor_name": "Mateo Vance",
                                "check_category": "OBSERVATION",
                                "attribute_tn": 14,
                                "skill_rank": 2,
                                "difficulty_mod": 0,
                                "note": "Resolve scene check",
                            },
                            "id": f"mock-gm-{uuid.uuid4().hex[:6]}",
                        }
                    ],
                )

        if self._mock_reply:
            return AIMessage(content=self._mock_reply)

        tool_summary = ""
        if trailing_tools:
            tool_names = ", ".join(f"`{t.name}`" for t in trailing_tools if t.name)
            tool_summary = f" *(Tool results incorporated from {tool_names})*"

        if is_player:
            return AIMessage(
                content=(
                    f"I scan the corridor threshold, keeping low behind the bulkhead cover as I check the console readout.{tool_summary}\n\n"
                    "- **Action**: I examine the terminal log and watch the opposite doorway for movement."
                )
            )
        if is_ooc_rules:
            return AIMessage(
                content=(
                    f"**Rules Summary (`2d20xN` & Tactical Triad)**{tool_summary}\n\n"
                    "- Roll `N` sequential `2d20` pairs against `TN_eff = 31 - C + Delta_ext` (`+6` carry on success, `-delta_under` on failure).\n"
                    "- In Combat Action Mode, `Effective TC = max(1, Base TC - floor((T - B) / 2))`."
                )
            )
        return AIMessage(
            content=(
                f"**Scene Beat**{tool_summary}\n\n"
                "*Rain hisses against the reinforced glass as amber status strobes sweep across the floor.*\n\n"
                "> [!NOTE]\n"
                "> Sensor sweep detects an unauthenticated uplink ping two decks below.\n\n"
                "- **Inspect** the relay terminal for the caller's routing signature.\n"
                "- **Hold position** and cover the stairwell approach."
            )
        )


def _clear_session_state_and_checkpoints(session_id: str) -> None:
    from satya_ml.ai_gm.server import clear_gm_checkpointer_session
    from satya_ml.ai_player.server import clear_player_checkpointer_session

    GM_SESSION_REGISTRY.reset(session_id)
    PLAYER_SESSION_REGISTRY.reset(session_id)
    clear_gm_checkpointer_session(session_id)
    clear_player_checkpointer_session(session_id)


def seed_offline_session(session_id: str, seed_config: Mapping[str, Any] | None = None) -> dict[str, Any]:
    """Seed in-memory GM & AI Player stores for offline/standalone debugging without `satya-signaling`."""
    from satya_ml.ai_gm.maps import generate_tactical_map, set_session_map
    from satya_ml.ai_gm.scenario_director import set_session_scenario_director
    from satya_ml.ai_gm.tools import (
        _SESSION_NPCS,
        get_session_combat_tracker,
        get_session_npcs,
        load_campaign_npc,
        set_session_characters,
        set_session_combat_tracker,
        set_session_gm_docs,
        set_session_gm_memories,
        set_session_operating_mode,
    )
    from satya_ml.ai_player.tools import set_player_session_context

    cfg = dict(seed_config or {})
    if cfg.get("reset_first", True):
        _clear_session_state_and_checkpoints(session_id)

    if cfg.get("operating_mode"):
        set_session_operating_mode(session_id, str(cfg["operating_mode"]))

    # Seed map first so any subsequent world-ledger seeding respects economic_tier/zoning.
    if isinstance(cfg.get("active_map"), Mapping):
        set_session_map(session_id, dict(cfg["active_map"]))
    elif cfg.get("seed_default_map"):
        with offline_signaling_scope(True):
            generate_tactical_map.invoke(
                {
                    "session_id": session_id,
                    "theme": str(cfg.get("map_theme") or "Rain-slicked Neon Safe-House Corridor"),
                    "economic_tier": str(cfg.get("economic_tier") or "LOW_INCOME"),
                }
            )

    if isinstance(cfg.get("characters"), list):
        set_session_characters(session_id, [dict(c) for c in cfg["characters"] if isinstance(c, Mapping)])
    elif cfg.get("seed_default_character", True):
        starter = dict(get_pack_default_ai_player())
        starter.setdefault("character_id", f"char-{session_id}")
        starter.setdefault("owner_user_id", "debug-player-1")
        set_session_characters(session_id, [starter])

    if isinstance(cfg.get("gm_memories"), list):
        set_session_gm_memories(session_id, [dict(m) for m in cfg["gm_memories"] if isinstance(m, Mapping)])
    if isinstance(cfg.get("gm_docs"), list):
        set_session_gm_docs(session_id, [dict(d) for d in cfg["gm_docs"] if isinstance(d, Mapping)])
    if isinstance(cfg.get("scenario_director"), Mapping):
        set_session_scenario_director(session_id, dict(cfg["scenario_director"]))
    if isinstance(cfg.get("combat_tracker"), Mapping):
        set_session_combat_tracker(session_id, dict(cfg["combat_tracker"]))

    if isinstance(cfg.get("npcs"), list):
        npc_map = dict(get_session_npcs(session_id))
        for raw_npc in cfg["npcs"]:
            if not isinstance(raw_npc, Mapping):
                continue
            nid = str(raw_npc.get("npc_id") or "").strip().lower()
            if nid:
                npc_map[nid] = dict(raw_npc)
        _SESSION_NPCS[session_id] = npc_map
    for starter_id in cfg.get("load_starter_npcs") or []:
        with offline_signaling_scope(True):
            load_campaign_npc.invoke({"session_id": session_id, "npc_id": str(starter_id)})

    if isinstance(cfg.get("player_contexts"), list):
        for p_ctx in cfg["player_contexts"]:
            if isinstance(p_ctx, Mapping):
                p_uid = str(p_ctx.get("player_user_id") or "ai-player-1")
                set_player_session_context(session_id, p_uid, dict(p_ctx))

    return {
        "session_id": session_id,
        "combat_active": bool(get_session_combat_tracker(session_id).get("combat_active")),
        "seeded": True,
    }


AssertionKind = Literal[
    "nodes_visited",
    "nodes_not_visited",
    "tools_called",
    "no_tool_errors",
    "reply_non_empty",
    "reply_contains",
    "party_gate_status",
    "max_duration_ms",
    "no_gm_secret_leak",
]


class ScenarioAssertion(BaseModel):
    """Automated pass/fail check evaluated against a scenario's recorded `TurnRunRecord`s."""

    kind: AssertionKind
    value: Any = None
    step_index: int | None = None
    description: str = ""


class ScenarioStep(BaseModel):
    """Single step inside an integration scenario."""

    kind: Literal["gm_turn", "scenario_beat", "player_turn", "player_gm_loop"]
    prompt: str = ""
    actor_user_id: str = "debug-player-1"
    actor_display_name: str = "Mateo Vance"
    operating_mode: str = "AUTONOMOUS"
    channel_context: str | None = None
    roll_summary: str | None = None
    task_slug: str = "auto"
    roster: list[dict[str, Any]] = Field(default_factory=list)
    player_user_id: str = "ai-player-1"
    player_display_name: str | None = None
    persona_prompt: str = ""
    turns: int = 3
    interrupt_before: list[str] = Field(default_factory=list)
    mock_reply: str | None = None


class ScenarioSpec(BaseModel):
    """Declarative multi-turn offline or live integration scenario."""

    id: str
    name: str
    description: str = ""
    session_id: str | None = None
    offline_mode: bool = True
    provider: str | None = None
    model_id: str | None = None
    seed: dict[str, Any] = Field(default_factory=dict)
    steps: list[ScenarioStep] = Field(default_factory=list)
    assertions: list[ScenarioAssertion] = Field(default_factory=list)


def resolve_debug_llm_config(
    *,
    provider: str | None = None,
    model_id: str | None = None,
    temperature: float | None = None,
    default_temperature: float = 0.35,
) -> tuple[SessionLlmConfig, Any | None]:
    """Resolve `SessionLlmConfig` plus optional `ScriptedDebugChatModel` when `provider == 'mock'`."""
    file_cfg = load_ai_gm_toml_config()
    eff_provider = (provider or "").strip() or str(file_cfg.get("default_provider") or "ollama")
    eff_model = (model_id or "").strip() or str(file_cfg.get("reasoning_model") or "gemma4:12b")
    eff_temp = (
        float(temperature)
        if temperature is not None
        else float(file_cfg.get("temperature") or default_temperature)
    )
    if eff_provider.lower() == "mock":
        cfg = SessionLlmConfig(
            provider="ollama",
            model_id=eff_model if model_id else "mock-debug-model",
            vision_model_id="mock-debug-model",
            temperature=0.0,
        )
        return cfg, ScriptedDebugChatModel()
    cfg = SessionLlmConfig(
        provider=eff_provider,  # type: ignore[arg-type]
        model_id=eff_model,
        vision_model_id=str(file_cfg.get("vision_model") or eff_model),
        temperature=eff_temp,
        base_url=str(file_cfg.get("ollama_base_url") or "http://127.0.0.1:11434"),
        num_ctx=int(file_cfg.get("num_ctx") or 16384),
        num_predict=int(file_cfg.get("num_predict") or 2048),
        reasoning=bool(file_cfg.get("reasoning", False)),
    )
    return cfg, None


def invoke_gm_debug_turn(
    *,
    session_id: str,
    prompt: str,
    actor_user_id: str = "debug-player-1",
    actor_display_name: str = "Mateo Vance",
    operating_mode: str = "AUTONOMOUS",
    channel_context: str | None = None,
    roll_summary: str | None = None,
    roster: list[dict[str, Any]] | None = None,
    force_resolve_turn: bool = False,
    offline_mode: bool = True,
    provider: str | None = None,
    model_id: str | None = None,
    temperature: float | None = None,
    interrupt_before: Sequence[str] | None = None,
    mock_reply: str | None = None,
    source: str = "debug_ui",
    hub: GraphExecutionHub | None = None,
) -> tuple[dict[str, Any], TurnRunRecord]:
    """Execute one AI GM turn with optional offline isolation, mock LLM, and breakpoints."""
    from satya_ml.ai_gm.graph import build_ai_gm_graph
    from satya_ml.ai_gm.server import (
        _CHAT_MODEL_OVERRIDE,
        _SHARED_CHECKPOINTER,
        _get_session_lock,
        _normalize_operating_mode,
    )

    llm_cfg, mock_model = resolve_debug_llm_config(
        provider=provider,
        model_id=model_id,
        temperature=temperature,
        default_temperature=0.35,
    )
    if mock_model is not None and mock_reply:
        mock_model = ScriptedDebugChatModel(mock_reply=mock_reply)
    chat_override = mock_model if mock_model is not None else _CHAT_MODEL_OVERRIDE
    eff_provider_label = "mock" if mock_model is not None else llm_cfg.provider

    file_cfg = load_ai_gm_toml_config()
    lazy_raw = bool(file_cfg.get("lazy_tool_loading", True))
    graph = build_ai_gm_graph(
        llm_cfg,
        chat_model=chat_override,
        checkpointer=_SHARED_CHECKPOINTER,
        lazy_tool_loading=lazy_raw,
    )
    op_mode = _normalize_operating_mode(operating_mode)
    visibility = "PUBLIC"

    with offline_signaling_scope(offline_mode):
        with turn_telemetry_scope(session_id) as telemetry:
            initial_state: dict[str, Any] = {
                "session_id": session_id,
                "session_mode": "TTRPG",
                "operating_mode": op_mode,
                "human_gm_user_id": None,
                "roster": list(roster or []),
                "trigger_event": {
                    "event_id": f"dbg-{uuid.uuid4().hex[:8]}",
                    "trigger_kind": "CHAT",
                    "actor": {
                        "user_id": actor_user_id,
                        "username": actor_user_id,
                        "display_name": actor_display_name,
                        "participant_kind": "HUMAN",
                        "role": "PLAYER",
                    },
                    "channel": visibility,
                    "channel_context": channel_context,
                    "content": prompt,
                    "roll_summary": roll_summary,
                    "force_resolve_turn": force_resolve_turn,
                    "operating_mode": op_mode,
                    "trace_id": telemetry.trace_id,
                },
                "tool_invocations": [],
            }
            lock = _get_session_lock(session_id)
            with lock:
                result, run = run_observed_graph(
                    graph,
                    initial_state,
                    {"configurable": {"thread_id": session_id}},
                    agent_kind="ai_gm",
                    session_id=session_id,
                    trace_id=telemetry.trace_id,
                    provider=eff_provider_label,
                    model_id=llm_cfg.model_id,
                    operating_mode=op_mode,
                    trigger_summary=prompt,
                    source=source,
                    interrupt_before=interrupt_before,
                    offline_mode=offline_mode,
                    hub=hub,
                    lock_getter=lambda sid=session_id: _get_session_lock(sid),
                )
            return result, run


def invoke_scenario_beat_debug_turn(
    *,
    session_id: str,
    task_slug: str = "auto",
    custom_context: str = "",
    operating_mode: str = "AUTONOMOUS",
    offline_mode: bool = True,
    provider: str | None = None,
    model_id: str | None = None,
    interrupt_before: Sequence[str] | None = None,
    mock_reply: str | None = None,
    source: str = "debug_ui",
    hub: GraphExecutionHub | None = None,
) -> tuple[dict[str, Any], TurnRunRecord]:
    """Trigger a proactive Scenario Director beat directly against the AI GM graph."""
    from satya_ml.ai_gm.graph import build_ai_gm_graph
    from satya_ml.ai_gm.server import (
        _CHAT_MODEL_OVERRIDE,
        _SHARED_CHECKPOINTER,
        _get_session_lock,
        _normalize_operating_mode,
    )

    llm_cfg, mock_model = resolve_debug_llm_config(
        provider=provider,
        model_id=model_id,
        default_temperature=0.35,
    )
    if mock_model is not None and mock_reply:
        mock_model = ScriptedDebugChatModel(mock_reply=mock_reply)
    chat_override = mock_model if mock_model is not None else _CHAT_MODEL_OVERRIDE
    eff_provider_label = "mock" if mock_model is not None else llm_cfg.provider

    file_cfg = load_ai_gm_toml_config()
    lazy_raw = bool(file_cfg.get("lazy_tool_loading", True))
    graph = build_ai_gm_graph(
        llm_cfg,
        chat_model=chat_override,
        checkpointer=_SHARED_CHECKPOINTER,
        lazy_tool_loading=lazy_raw,
    )
    op_mode = _normalize_operating_mode(operating_mode)
    prompt_text = custom_context or f"Trigger proactive scenario beat ({task_slug})"

    with offline_signaling_scope(offline_mode):
        with turn_telemetry_scope(session_id) as telemetry:
            initial_state: dict[str, Any] = {
                "session_id": session_id,
                "session_mode": "TTRPG",
                "operating_mode": op_mode,
                "trigger_event": {
                    "trigger_kind": "SCENARIO_BEAT",
                    "force_scenario_beat": True,
                    "task_slug": task_slug,
                    "custom_context": custom_context,
                    "actor": {"user_id": "scenario-director", "display_name": "Scenario Director"},
                    "channel": "PUBLIC",
                    "content": prompt_text,
                    "operating_mode": op_mode,
                    "trace_id": telemetry.trace_id,
                },
                "tool_invocations": [],
            }
            with _get_session_lock(session_id):
                return run_observed_graph(
                    graph,
                    initial_state,
                    {"configurable": {"thread_id": session_id}},
                    agent_kind="ai_gm",
                    session_id=session_id,
                    trace_id=telemetry.trace_id,
                    provider=eff_provider_label,
                    model_id=llm_cfg.model_id,
                    operating_mode=op_mode,
                    trigger_summary=prompt_text,
                    source=source,
                    interrupt_before=interrupt_before,
                    offline_mode=offline_mode,
                    hub=hub,
                    lock_getter=lambda sid=session_id: _get_session_lock(sid),
                )


def invoke_player_debug_turn(
    *,
    session_id: str,
    scene_prompt: str,
    player_user_id: str = "ai-player-1",
    player_display_name: str | None = None,
    persona_prompt: str = "",
    roster: list[dict[str, Any]] | None = None,
    offline_mode: bool = True,
    provider: str | None = None,
    model_id: str | None = None,
    temperature: float | None = None,
    interrupt_before: Sequence[str] | None = None,
    mock_reply: str | None = None,
    source: str = "debug_ui",
    hub: GraphExecutionHub | None = None,
) -> tuple[dict[str, Any], TurnRunRecord]:
    """Execute one isolated AI Player turn with optional offline isolation, mock LLM, and breakpoints."""
    from satya_ml.ai_player.graph import build_ai_player_graph
    from satya_ml.ai_player.server import (
        _PLAYER_CHAT_MODEL_OVERRIDE,
        _PLAYER_CHECKPOINTER,
        _get_player_lock,
    )

    llm_cfg, mock_model = resolve_debug_llm_config(
        provider=provider,
        model_id=model_id,
        temperature=temperature,
        default_temperature=0.45,
    )
    if mock_model is not None and mock_reply:
        mock_model = ScriptedDebugChatModel(mock_reply=mock_reply)
    chat_override = mock_model if mock_model is not None else _PLAYER_CHAT_MODEL_OVERRIDE
    eff_provider_label = "mock" if mock_model is not None else llm_cfg.provider

    graph = build_ai_player_graph(
        llm_cfg,
        chat_model=chat_override,
        checkpointer=_PLAYER_CHECKPOINTER,
    )
    default_name = str(get_pack_default_ai_player().get("character_name") or "Mateo Vance")
    eff_display_name = player_display_name or f"{default_name} (AI Player)"
    thread_id = f"{session_id}:player:{player_user_id}"

    with offline_signaling_scope(offline_mode):
        with turn_telemetry_scope(session_id) as telemetry:
            initial_state: dict[str, Any] = {
                "session_id": session_id,
                "player_user_id": player_user_id,
                "player_display_name": eff_display_name,
                "persona_prompt": persona_prompt,
                "roster": list(roster or []),
                "trigger_event": {
                    "event_id": f"pdbg-{uuid.uuid4().hex[:8]}",
                    "player_user_id": player_user_id,
                    "player_display_name": eff_display_name,
                    "persona_prompt": persona_prompt,
                    "roster": list(roster or []),
                    "content": scene_prompt,
                    "auto_trigger_gm": False,
                },
                "tool_invocations": [],
                "roll_results": [],
            }
            with _get_player_lock(session_id, player_user_id):
                return run_observed_graph(
                    graph,
                    initial_state,
                    {"configurable": {"thread_id": thread_id}},
                    agent_kind="ai_player",
                    session_id=session_id,
                    player_user_id=player_user_id,
                    trace_id=telemetry.trace_id,
                    provider=eff_provider_label,
                    model_id=llm_cfg.model_id,
                    operating_mode="autonomous",
                    trigger_summary=scene_prompt,
                    source=source,
                    interrupt_before=interrupt_before,
                    offline_mode=offline_mode,
                    hub=hub,
                    lock_getter=lambda sid=session_id, uid=player_user_id: _get_player_lock(sid, uid),
                )


def _evaluate_assertion(
    assertion: ScenarioAssertion,
    runs: Sequence[TurnRunRecord],
    secret_markers: Sequence[str],
) -> dict[str, Any]:
    if assertion.step_index is not None:
        if 0 <= assertion.step_index < len(runs):
            scoped_runs = [runs[assertion.step_index]]
        else:
            return {
                "kind": assertion.kind,
                "passed": False,
                "detail": f"step_index={assertion.step_index} out of range (runs={len(runs)})",
                "description": assertion.description,
            }
    else:
        scoped_runs = list(runs)

    visited = {s.node_name for r in scoped_runs for s in r.steps}
    tools_ok = [
        str(inv.get("tool_name") or "")
        for r in scoped_runs
        for inv in r.tool_invocations
    ]
    tool_errors = [
        inv
        for r in scoped_runs
        for inv in r.tool_invocations
        if inv.get("is_error")
    ]

    passed = False
    detail = ""
    if assertion.kind == "nodes_visited":
        expected = [str(x) for x in (assertion.value or [])]
        missing = [n for n in expected if n not in visited]
        passed = len(missing) == 0
        detail = "all expected nodes visited" if passed else f"missing nodes: {missing}"
    elif assertion.kind == "nodes_not_visited":
        forbidden = [str(x) for x in (assertion.value or [])]
        present = [n for n in forbidden if n in visited]
        passed = len(present) == 0
        detail = "no forbidden nodes visited" if passed else f"unexpectedly visited: {present}"
    elif assertion.kind == "tools_called":
        expected = [str(x) for x in (assertion.value or [])]
        missing = [t for t in expected if t not in tools_ok]
        passed = len(missing) == 0
        detail = f"tools called={tools_ok}" if passed else f"missing tools: {missing}"
    elif assertion.kind == "no_tool_errors":
        passed = len(tool_errors) == 0
        detail = "0 tool errors" if passed else f"tool errors: {tool_errors}"
    elif assertion.kind == "reply_non_empty":
        empty_runs = [r.run_id for r in scoped_runs if not (r.final_reply or "").strip()]
        passed = bool(scoped_runs) and len(empty_runs) == 0
        detail = "all replies non-empty" if passed else f"empty replies in runs={empty_runs}"
    elif assertion.kind == "reply_contains":
        needle = str(assertion.value or "").lower()
        passed = any(needle in (r.final_reply or "").lower() for r in scoped_runs)
        detail = f"found '{needle}'" if passed else f"'{needle}' not found in replies"
    elif assertion.kind == "party_gate_status":
        expected_gate = str(assertion.value or "")
        actual_gates = [r.party_gate_status for r in scoped_runs]
        passed = bool(scoped_runs) and scoped_runs[-1].party_gate_status == expected_gate
        detail = f"expected={expected_gate!r}, actual={actual_gates}"
    elif assertion.kind == "max_duration_ms":
        cap_ms = int(assertion.value or 0)
        total_ms = sum(r.duration_ms for r in scoped_runs)
        passed = total_ms <= cap_ms
        detail = f"total_ms={total_ms}ms <= {cap_ms}ms"
    elif assertion.kind == "no_gm_secret_leak":
        player_runs = [r for r in scoped_runs if r.agent_kind == "ai_player"]
        leaks: list[str] = []
        for r in player_runs:
            haystack = (
                (r.final_reply or "")
                + "\n"
                + json.dumps(r.messages)
            ).lower()
            for marker in secret_markers:
                m_low = marker.strip().lower()
                if len(m_low) >= 6 and m_low in haystack:
                    leaks.append(marker)
        passed = len(leaks) == 0
        detail = "player visibility wall held" if passed else f"leaked secret markers: {leaks}"

    return {
        "kind": assertion.kind,
        "step_index": assertion.step_index,
        "value": assertion.value,
        "passed": passed,
        "detail": detail,
        "description": assertion.description,
    }


def run_scenario(spec: ScenarioSpec, *, hub: GraphExecutionHub | None = None) -> dict[str, Any]:
    """Execute a multi-step `ScenarioSpec`, stream progress to `GRAPH_HUB`, and evaluate assertions."""
    eff_hub = hub or GRAPH_HUB
    session_id = spec.session_id or f"scn-{spec.id}-{uuid.uuid4().hex[:6]}"
    started = time.perf_counter()

    seed_cfg = dict(spec.seed)
    seed_cfg.setdefault("reset_first", True)
    seed_offline_session(session_id, seed_cfg)

    secret_markers: list[str] = []
    for doc in seed_cfg.get("gm_docs") or []:
        if isinstance(doc, Mapping) and doc.get("sanitized_content"):
            secret_markers.append(str(doc["sanitized_content"]))
    for mem in seed_cfg.get("gm_memories") or []:
        if isinstance(mem, Mapping) and mem.get("content"):
            secret_markers.append(str(mem["content"]))
    secret_markers.extend(["SATYANG_CONSPIRACY_SECRET", "ARC2_SATYANG"])

    eff_hub.publish_event(
        "scenario_started",
        {
            "scenario_id": spec.id,
            "name": spec.name,
            "session_id": session_id,
            "step_count": len(spec.steps),
        },
    )

    runs: list[TurnRunRecord] = []
    transcript: list[dict[str, Any]] = []

    for idx, step in enumerate(spec.steps):
        eff_hub.publish_event(
            "scenario_step",
            {
                "scenario_id": spec.id,
                "session_id": session_id,
                "step_index": idx,
                "kind": step.kind,
            },
        )
        if step.kind == "gm_turn":
            _res, run = invoke_gm_debug_turn(
                session_id=session_id,
                prompt=step.prompt,
                actor_user_id=step.actor_user_id,
                actor_display_name=step.actor_display_name,
                operating_mode=step.operating_mode,
                channel_context=step.channel_context,
                roll_summary=step.roll_summary,
                roster=step.roster,
                offline_mode=spec.offline_mode,
                provider=spec.provider,
                model_id=spec.model_id,
                interrupt_before=step.interrupt_before,
                mock_reply=step.mock_reply,
                source="scenario",
                hub=eff_hub,
            )
            runs.append(run)
            transcript.append(
                {
                    "step_index": idx,
                    "agent_kind": "ai_gm",
                    "speaker": "AI GM",
                    "prompt": step.prompt,
                    "reply": run.final_reply or "",
                    "run_id": run.run_id,
                    "party_gate_status": run.party_gate_status,
                }
            )
        elif step.kind == "scenario_beat":
            _res, run = invoke_scenario_beat_debug_turn(
                session_id=session_id,
                task_slug=step.task_slug,
                custom_context=step.prompt,
                operating_mode=step.operating_mode,
                offline_mode=spec.offline_mode,
                provider=spec.provider,
                model_id=spec.model_id,
                interrupt_before=step.interrupt_before,
                mock_reply=step.mock_reply,
                source="scenario",
                hub=eff_hub,
            )
            runs.append(run)
            transcript.append(
                {
                    "step_index": idx,
                    "agent_kind": "ai_gm",
                    "speaker": f"Scenario Beat ({step.task_slug})",
                    "prompt": step.prompt,
                    "reply": run.final_reply or "",
                    "run_id": run.run_id,
                }
            )
        elif step.kind == "player_turn":
            _res, run = invoke_player_debug_turn(
                session_id=session_id,
                scene_prompt=step.prompt,
                player_user_id=step.player_user_id,
                player_display_name=step.player_display_name,
                persona_prompt=step.persona_prompt,
                roster=step.roster,
                offline_mode=spec.offline_mode,
                provider=spec.provider,
                model_id=spec.model_id,
                interrupt_before=step.interrupt_before,
                mock_reply=step.mock_reply,
                source="scenario",
                hub=eff_hub,
            )
            runs.append(run)
            transcript.append(
                {
                    "step_index": idx,
                    "agent_kind": "ai_player",
                    "speaker": step.player_display_name or step.player_user_id,
                    "prompt": step.prompt,
                    "reply": run.final_reply or "",
                    "run_id": run.run_id,
                }
            )
        elif step.kind == "player_gm_loop":
            loop_turns = max(1, min(10, int(step.turns)))
            gm_prompt = step.prompt or "You stand at the rain-lashed entrance of the relay annex. What do you do?"
            p_name = step.player_display_name or str(
                get_pack_default_ai_player().get("character_name") or "Mateo Vance"
            )
            for loop_i in range(loop_turns):
                _p_res, p_run = invoke_player_debug_turn(
                    session_id=session_id,
                    scene_prompt=gm_prompt,
                    player_user_id=step.player_user_id,
                    player_display_name=p_name,
                    persona_prompt=step.persona_prompt,
                    roster=step.roster,
                    offline_mode=spec.offline_mode,
                    provider=spec.provider,
                    model_id=spec.model_id,
                    source="scenario",
                    hub=eff_hub,
                )
                runs.append(p_run)
                player_action = (p_run.final_reply or "").strip() or "I scan the doorway carefully."
                transcript.append(
                    {
                        "step_index": idx,
                        "loop_turn": loop_i + 1,
                        "agent_kind": "ai_player",
                        "speaker": p_name,
                        "prompt": gm_prompt,
                        "reply": player_action,
                        "run_id": p_run.run_id,
                    }
                )
                _g_res, g_run = invoke_gm_debug_turn(
                    session_id=session_id,
                    prompt=player_action,
                    actor_user_id=step.player_user_id,
                    actor_display_name=p_name,
                    operating_mode=step.operating_mode,
                    roster=step.roster,
                    offline_mode=spec.offline_mode,
                    provider=spec.provider,
                    model_id=spec.model_id,
                    source="scenario",
                    hub=eff_hub,
                )
                runs.append(g_run)
                gm_prompt = (g_run.final_reply or "").strip() or "The corridor hums with static."
                transcript.append(
                    {
                        "step_index": idx,
                        "loop_turn": loop_i + 1,
                        "agent_kind": "ai_gm",
                        "speaker": "AI GM",
                        "prompt": player_action,
                        "reply": gm_prompt,
                        "run_id": g_run.run_id,
                    }
                )

    assertion_results = [
        _evaluate_assertion(a, runs, secret_markers) for a in spec.assertions
    ]
    passed = all(r["passed"] for r in assertion_results) if assertion_results else True
    duration_ms = int(max(1.0, (time.perf_counter() - started) * 1000.0))

    result = {
        "scenario_id": spec.id,
        "name": spec.name,
        "session_id": session_id,
        "passed": passed,
        "duration_ms": duration_ms,
        "executed_at_iso": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "run_ids": [r.run_id for r in runs],
        "runs": [r.to_summary_dict() for r in runs],
        "transcript": transcript,
        "assertion_results": assertion_results,
    }
    eff_hub.publish_event(
        "scenario_completed",
        {
            "scenario_id": spec.id,
            "session_id": session_id,
            "passed": passed,
            "duration_ms": duration_ms,
        },
    )
    return result


BUILTIN_SCENARIOS: dict[str, ScenarioSpec] = {
    "single_player_opening": ScenarioSpec(
        id="single_player_opening",
        name="Single-Player Opening Turn (7-Node AI GM)",
        description="Runs a standard single-player turn offline with a seeded character and verifies all 7 AI GM nodes execute.",
        offline_mode=True,
        provider="mock",
        seed={"seed_default_character": True, "seed_default_map": True},
        steps=[
            ScenarioStep(
                kind="gm_turn",
                prompt="I sweep the darkened server loft with my flashlight and listen for footsteps.",
            )
        ],
        assertions=[
            ScenarioAssertion(
                kind="nodes_visited",
                value=[
                    "sync_session_context",
                    "party_turn_gate_node",
                    "scenario_director_node",
                    "npc_action_resolution_node",
                    "gm_supervisor_node",
                    "tool_execution_node",
                    "publish_gm_response",
                ],
                description="All 7 AI GM nodes including tool loop execute",
            ),
            ScenarioAssertion(kind="tools_called", value=["roll_satya_d20_check"]),
            ScenarioAssertion(kind="no_tool_errors"),
            ScenarioAssertion(kind="reply_non_empty"),
        ],
    ),
    "ooc_rules_question": ScenarioSpec(
        id="ooc_rules_question",
        name="OOC Mechanics & Rules Query",
        description="Tests OUT_OF_CHARACTER rules query routing and verifies query_satya_rules is invoked cleanly.",
        offline_mode=True,
        provider="mock",
        seed={"seed_default_character": True},
        steps=[
            ScenarioStep(
                kind="gm_turn",
                prompt="How do Tempo, Control, and Force pools work in combat?",
                channel_context="OUT_OF_CHARACTER",
            )
        ],
        assertions=[
            ScenarioAssertion(kind="tools_called", value=["query_satya_rules"]),
            ScenarioAssertion(kind="no_tool_errors"),
            ScenarioAssertion(kind="reply_contains", value="2d20"),
        ],
    ),
    "two_player_turn_gate": ScenarioSpec(
        id="two_player_turn_gate",
        name="Multi-Player Turn Barrier (2 Players)",
        description="Player 1 declares first (gate waits & skips scenario_director/LLM), then Player 2 declares and resolves the combined round.",
        offline_mode=True,
        provider="mock",
        seed={"seed_default_character": True},
        steps=[
            ScenarioStep(
                kind="gm_turn",
                prompt="I slice the stairwell keypad quietly.",
                actor_user_id="u-p1",
                actor_display_name="Kaelen",
                roster=[
                    {"user_id": "u-p1", "display_name": "Kaelen", "participant_kind": "HUMAN", "role": "PLAYER"},
                    {"user_id": "u-p2", "display_name": "Lyra", "participant_kind": "HUMAN", "role": "PLAYER"},
                ],
            ),
            ScenarioStep(
                kind="gm_turn",
                prompt="I cover the corridor corner with my carbine ready.",
                actor_user_id="u-p2",
                actor_display_name="Lyra",
                roster=[
                    {"user_id": "u-p1", "display_name": "Kaelen", "participant_kind": "HUMAN", "role": "PLAYER"},
                    {"user_id": "u-p2", "display_name": "Lyra", "participant_kind": "HUMAN", "role": "PLAYER"},
                ],
            ),
        ],
        assertions=[
            ScenarioAssertion(kind="party_gate_status", step_index=0, value="waiting"),
            ScenarioAssertion(
                kind="nodes_not_visited",
                step_index=0,
                value=["scenario_director_node", "gm_supervisor_node"],
                description="Step 0 skips LLM & director while awaiting Player 2",
            ),
            ScenarioAssertion(kind="party_gate_status", step_index=1, value="ready"),
            ScenarioAssertion(
                kind="nodes_visited",
                step_index=1,
                value=["scenario_director_node", "npc_action_resolution_node", "gm_supervisor_node"],
                description="Step 1 resolves full combined party turn",
            ),
            ScenarioAssertion(kind="reply_non_empty", step_index=1),
        ],
    ),
    "combat_action_mode": ScenarioSpec(
        id="combat_action_mode",
        name="Combat Action Mode & Autonomous NPC Tick",
        description="Seeds active combat tracker + NPC sentry at Tick 3 and verifies autonomous NPC resolution and combat tool loading.",
        offline_mode=True,
        provider="mock",
        seed={
            "seed_default_character": True,
            "seed_default_map": True,
            "combat_tracker": {
                "combat_active": True,
                "round": 1,
                "tick": 3,
                "combatants": [
                    {
                        "entity_id": "secops_sentry",
                        "display_name": "SecOps Sentry",
                        "role_or_faction": "NPC",
                        "is_npc": True,
                        "ooda_score": 12,
                        "next_action_tick": 3,
                        "active_stance": "NEUTRAL",
                        "grid_x": 12,
                        "grid_y": 6,
                        "zone_label": "Entry Bulkhead",
                    },
                    {
                        "entity_id": "debug-player-1",
                        "display_name": "Mateo Vance",
                        "role_or_faction": "PLAYER",
                        "is_npc": False,
                        "ooda_score": 11,
                        "next_action_tick": 4,
                        "active_stance": "NEUTRAL",
                        "grid_x": 4,
                        "grid_y": 6,
                        "zone_label": "Core Console",
                    },
                ],
            },
        },
        steps=[
            ScenarioStep(
                kind="gm_turn",
                prompt="I snap-shot toward the entry bulkhead with Tempo 5, Control 5, Force 4!",
                roll_summary="Snap Shot (2 successes)",
            )
        ],
        assertions=[
            ScenarioAssertion(
                kind="nodes_visited",
                value=["npc_action_resolution_node", "gm_supervisor_node", "publish_gm_response"],
            ),
            ScenarioAssertion(kind="no_tool_errors"),
            ScenarioAssertion(kind="reply_non_empty"),
        ],
    ),
    "player_vs_gm_loop_3_turns": ScenarioSpec(
        id="player_vs_gm_loop_3_turns",
        name="AI Player <-> AI GM 3-Turn Integration Loop",
        description="Runs 3 alternating turns between AI Player and AI GM in one isolated session and checks the player visibility wall.",
        offline_mode=True,
        provider="mock",
        seed={
            "seed_default_character": True,
            "seed_default_map": True,
            "gm_docs": [
                {
                    "doc_id": "secret-doc-1",
                    "title": "Classified Satyang Directive",
                    "is_persistent": True,
                    "sanitized_content": "ULTRA_SECRET_CONSPIRACY_PHRASE_404",
                }
            ],
        },
        steps=[
            ScenarioStep(
                kind="player_gm_loop",
                turns=3,
                prompt="You reach the locked relay junction on Sub-Level 3. Rain leaks down the conduit.",
            )
        ],
        assertions=[
            ScenarioAssertion(
                kind="nodes_visited",
                value=[
                    "sync_player_context",
                    "player_decision_node",
                    "publish_player_turn",
                    "sync_session_context",
                    "gm_supervisor_node",
                    "publish_gm_response",
                ],
            ),
            ScenarioAssertion(kind="no_tool_errors"),
            ScenarioAssertion(kind="reply_non_empty"),
            ScenarioAssertion(
                kind="no_gm_secret_leak",
                description="AI Player must never see or leak GM_ONLY secret text",
            ),
        ],
    ),
}


def main(argv: list[str] | None = None) -> int:
    """Headless CLI runner for Satya AI GM / AI Player integration scenarios."""
    parser = argparse.ArgumentParser(description="Run standalone Satya AI GM / AI Player scenarios offline")
    parser.add_argument("--list", action="store_true", help="List built-in scenario presets")
    parser.add_argument("--preset", default="single_player_opening", help="Built-in scenario preset ID")
    parser.add_argument("--file", default=None, help="Path to a custom ScenarioSpec JSON file")
    parser.add_argument("--provider", default=None, help="Override LLM provider (e.g. mock, ollama, gemini)")
    parser.add_argument("--model", default=None, help="Override LLM model ID")
    args = parser.parse_args(argv)

    if args.list:
        for sid, spec in BUILTIN_SCENARIOS.items():
            print(f"{sid:28s} - {spec.name}")
        return 0

    if args.file:
        raw = json.loads(Path(args.file).read_text(encoding="utf-8"))
        spec = ScenarioSpec.model_validate(raw)
    else:
        if args.preset not in BUILTIN_SCENARIOS:
            print(f"Unknown preset '{args.preset}'. Use --list to see available presets.", file=sys.stderr)
            return 2
        spec = BUILTIN_SCENARIOS[args.preset].model_copy(deep=True)

    if args.provider:
        spec.provider = args.provider
    if args.model:
        spec.model_id = args.model

    result = run_scenario(spec)
    status_label = "PASS" if result["passed"] else "FAIL"
    print(f"[{status_label}] {result['name']} (session={result['session_id']}, {result['duration_ms']}ms)")
    for a in result["assertion_results"]:
        mark = "OK" if a["passed"] else "FAIL"
        print(f"  - [{mark}] {a['kind']}: {a['detail']}")
    return 0 if result["passed"] else 1


__all__ = [
    "AssertionKind",
    "BUILTIN_SCENARIOS",
    "ScenarioAssertion",
    "ScenarioSpec",
    "ScenarioStep",
    "ScriptedDebugChatModel",
    "invoke_gm_debug_turn",
    "invoke_player_debug_turn",
    "invoke_scenario_beat_debug_turn",
    "resolve_debug_llm_config",
    "run_scenario",
    "seed_offline_session",
]

if __name__ == "__main__":
    raise SystemExit(main())
