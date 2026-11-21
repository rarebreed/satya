"""Tests for turn-scoped lazy tool-pack loading (`tool_registry.py`) in the Satya AI GM."""

from __future__ import annotations

import json
import uuid
from typing import Any

import pytest
from langchain_core.messages import AIMessage, BaseMessage
from langchain_core.tools import StructuredTool
from langgraph.checkpoint.memory import MemorySaver

import satya_ml.ai_gm as ai_gm
from satya_ml.ai_gm.graph import (
    AUTONOMOUS_SYSTEM_PROMPT,
    _classify_tool_tier,
    build_ai_gm_graph,
)
from satya_ml.ai_gm.tool_registry import (
    CORE_GM_TOOL_NAMES,
    DEFERRED_TOOL_PACKS,
    LOAD_GM_TOOLS_NAME,
    OOC_READONLY_TOOL_NAMES,
    GmToolRegistry,
    load_gm_tools,
)
from satya_ml.ai_gm.tools import (
    get_builtin_gm_tools,
    lookup_npc_profile,
    reset_session_npcs,
    set_session_combat_tracker,
)
from satya_ml.campaign_config import set_campaign_pack_override


class _RecordingChatModel:
    """Deterministic fake chat model capturing bound tool names and system prompts per LLM call."""

    def __init__(self, handler: Any) -> None:
        self._handler = handler
        self.bound_history: list[list[str]] = []
        self.system_prompts: list[str] = []
        self.call_index = 0

    def bind_tools(self, tools: list[Any]) -> _BoundProxy:
        names = [str(getattr(t, "name", "")) for t in tools]
        self.bound_history.append(names)
        return _BoundProxy(self, names)

    def invoke(self, messages: list[BaseMessage], *, bound_names: list[str] | None = None) -> AIMessage:
        if messages and getattr(messages[0], "type", "") == "system":
            self.system_prompts.append(str(messages[0].content))
        idx = self.call_index
        self.call_index += 1
        return self._handler(idx, messages, bound_names or [])


class _BoundProxy:
    def __init__(self, parent: _RecordingChatModel, bound_names: list[str]) -> None:
        self._parent = parent
        self._bound_names = bound_names

    def invoke(self, messages: list[BaseMessage]) -> AIMessage:
        return self._parent.invoke(messages, bound_names=self._bound_names)


def test_builtin_tool_partition_and_exports() -> None:
    builtin_tools = get_builtin_gm_tools()
    builtin_names = [t.name for t in builtin_tools]
    assert len(builtin_names) == 45
    assert len(set(builtin_names)) == 45

    core_set = set(CORE_GM_TOOL_NAMES)
    assert len(CORE_GM_TOOL_NAMES) == 10
    assert core_set.issubset(set(builtin_names))

    deferred_names: list[str] = []
    for spec in DEFERRED_TOOL_PACKS.values():
        deferred_names.extend(spec.tool_names)

    assert len(deferred_names) == 35
    assert len(set(deferred_names)) == 35
    assert core_set.isdisjoint(set(deferred_names))
    assert (core_set | set(deferred_names)) == set(builtin_names)

    assert _classify_tool_tier(LOAD_GM_TOOLS_NAME) == ("TIER_1_SATYA_CORE", "satya-core")
    assert {
        "CORE_GM_TOOL_NAMES",
        "DEFERRED_TOOL_PACKS",
        "GmToolRegistry",
        "OOC_READONLY_TOOL_NAMES",
        "load_gm_tools",
    }.issubset(set(ai_gm.__all__))


def test_default_narrative_turn_binds_11_tools_and_filters_campaign_packs() -> None:
    set_campaign_pack_override("awakened")
    sid_awk = f"lazy-awk-{uuid.uuid4().hex[:8]}"
    sid_so = f"lazy-so-{uuid.uuid4().hex[:8]}"
    try:
        model_awk = _RecordingChatModel(lambda _i, _m, _b: AIMessage(content="Scene steady."))
        graph_awk = build_ai_gm_graph(chat_model=model_awk, checkpointer=MemorySaver())
        graph_awk.invoke(
            {
                "session_id": sid_awk,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "I scan the server alcove.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid_awk}},
        )
        assert len(model_awk.bound_history) == 1
        assert len(model_awk.bound_history[0]) == 11
        assert set(model_awk.bound_history[0]) == set(CORE_GM_TOOL_NAMES) | {LOAD_GM_TOOLS_NAME}
        assert "awakened_campaign" in model_awk.system_prompts[0]

        set_campaign_pack_override("space_opera")
        model_so = _RecordingChatModel(lambda _i, _m, _b: AIMessage(content="Airlock hums."))
        graph_so = build_ai_gm_graph(chat_model=model_so, checkpointer=MemorySaver())
        graph_so.invoke(
            {
                "session_id": sid_so,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "I inspect the airlock.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid_so}},
        )
        assert len(model_so.bound_history[0]) == 11
        assert "awakened_campaign" not in model_so.system_prompts[0]
    finally:
        set_campaign_pack_override(None)
        reset_session_npcs(sid_awk)
        reset_session_npcs(sid_so)


def test_ooc_rules_turn_binds_8_readonly_tools() -> None:
    sid = f"lazy-ooc-{uuid.uuid4().hex[:8]}"
    try:
        model = _RecordingChatModel(lambda _i, _m, _b: AIMessage(content="Tick_start = max(1, 15 - OODA)."))
        graph = build_ai_gm_graph(chat_model=model, checkpointer=MemorySaver())
        out = graph.invoke(
            {
                "session_id": sid,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "channel_context": "OUT_OF_CHARACTER",
                    "content": "How does combat tick cost work?",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid}},
        )
        assert out["final_channel"] == "OUT_OF_CHARACTER"
        assert len(model.bound_history) == 1
        assert len(model.bound_history[0]) == 8
        assert set(model.bound_history[0]) == set(OOC_READONLY_TOOL_NAMES) | {LOAD_GM_TOOLS_NAME}
    finally:
        reset_session_npcs(sid)


def test_active_combat_and_combat_beat_auto_bind_16_tools() -> None:
    sid_active = f"lazy-combat-{uuid.uuid4().hex[:8]}"
    sid_beat = f"lazy-beat-{uuid.uuid4().hex[:8]}"
    try:
        set_session_combat_tracker(
            sid_active,
            {
                "session_id": sid_active,
                "combat_active": True,
                "round": 1,
                "tick": 1,
                "combatants": [
                    {
                        "entity_id": "u1",
                        "display_name": "Kaelen",
                        "is_npc": False,
                        "next_action_tick": 5,
                    }
                ],
            },
        )
        model_active = _RecordingChatModel(lambda _i, _m, _b: AIMessage(content="Take cover!"))
        graph_active = build_ai_gm_graph(chat_model=model_active, checkpointer=MemorySaver())
        graph_active.invoke(
            {
                "session_id": sid_active,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "I snap a shot downrange.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid_active}},
        )
        expected_combat = (
            set(CORE_GM_TOOL_NAMES)
            | set(DEFERRED_TOOL_PACKS["combat"].tool_names)
            | {LOAD_GM_TOOLS_NAME}
        )
        assert len(model_active.bound_history) == 1
        assert len(model_active.bound_history[0]) == 16
        assert set(model_active.bound_history[0]) == expected_combat

        model_beat = _RecordingChatModel(lambda _i, _m, _b: AIMessage(content="Breach team enters!"))
        graph_beat = build_ai_gm_graph(chat_model=model_beat, checkpointer=MemorySaver())
        out_beat = graph_beat.invoke(
            {
                "session_id": sid_beat,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "trigger_kind": "SCENARIO_BEAT",
                    "force_scenario_beat": True,
                    "task_slug": "combat_encounter",
                    "content": "Trigger ambush.",
                    "actor": {"user_id": "scenario-director", "display_name": "Scenario Director"},
                },
            },
            config={"configurable": {"thread_id": sid_beat}},
        )
        assert out_beat["combat_active"] is True
        assert len(model_beat.bound_history[0]) == 16
        assert set(model_beat.bound_history[0]) == expected_combat
    finally:
        reset_session_npcs(sid_active)
        reset_session_npcs(sid_beat)


def test_lazy_load_roundtrip_and_turn_end_expiry() -> None:
    sid = f"lazy-rt-{uuid.uuid4().hex[:8]}"
    try:
        def _handler(_idx: int, messages: list[BaseMessage], bound_names: list[str]) -> AIMessage:
            last = messages[-1]
            last_type = getattr(last, "type", "")
            last_name = getattr(last, "name", "")
            if last_type == "human" and "Turn 1" in str(last.content):
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": LOAD_GM_TOOLS_NAME,
                            "args": {"packs": ["tech_and_tradecraft"]},
                            "id": "call-load-tech",
                        }
                    ],
                )
            if last_type == "tool" and last_name == LOAD_GM_TOOLS_NAME:
                assert "resolve_explosive_vehicle_or_hazard" in bound_names
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": "resolve_explosive_vehicle_or_hazard",
                            "args": {
                                "mode": "BLAST",
                                "params_json": '{"ordnance_code":"M67_FRAG","distance_m":2.0,"enclosed_room":true}',
                            },
                            "id": "call-blast-1",
                        }
                    ],
                )
            if last_type == "tool" and last_name == "resolve_explosive_vehicle_or_hazard":
                return AIMessage(content="The frag blast echoes inside the enclosed bay.")
            return AIMessage(content="Turn 2 continues cleanly with base tools.")

        model = _RecordingChatModel(_handler)
        saver = MemorySaver()
        graph = build_ai_gm_graph(chat_model=model, checkpointer=saver)

        out1 = graph.invoke(
            {
                "session_id": sid,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "Turn 1: I toss a frag grenade into the server room.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid}},
        )
        assert "enclosed bay" in out1["final_reply"]
        assert len(model.bound_history) == 3
        assert len(model.bound_history[0]) == 11
        assert len(model.bound_history[1]) == 18
        assert len(model.bound_history[2]) == 18
        assert [inv["tool_name"] for inv in out1["tool_invocations"]] == [
            LOAD_GM_TOOLS_NAME,
            "resolve_explosive_vehicle_or_hazard",
        ]

        out2 = graph.invoke(
            {
                "session_id": sid,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "Turn 2: I check the corridor.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid}},
        )
        assert "Turn 2 continues" in out2["final_reply"]
        assert len(model.bound_history) == 4
        assert len(model.bound_history[3]) == 11
        assert "resolve_explosive_vehicle_or_hazard" not in model.bound_history[3]
        assert out2["loaded_tool_packs"] == []
        assert out2["loaded_tool_names"] == []
    finally:
        reset_session_npcs(sid)


def test_discovery_round_budget_fairness_and_cap() -> None:
    sid_fair = f"lazy-fair-{uuid.uuid4().hex[:8]}"
    sid_cap = f"lazy-cap-{uuid.uuid4().hex[:8]}"
    try:
        # 1 load_gm_tools round + 3 real tool execution rounds all succeed in a single turn
        fair_steps: list[int] = [0]

        def _fair_handler(_idx: int, _messages: list[BaseMessage], _bound: list[str]) -> AIMessage:
            step = fair_steps[0]
            fair_steps[0] += 1
            if step == 0:
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": LOAD_GM_TOOLS_NAME,
                            "args": {"packs": ["world_and_director"]},
                            "id": "call-fair-load",
                        }
                    ],
                )
            if step in {1, 2, 3}:
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": "get_world_clocks_and_heat",
                            "args": {},
                            "id": f"call-fair-exec-{step}",
                        }
                    ],
                )
            return AIMessage(content="All 3 execution rounds finished after 1 discovery round.")

        model_fair = _RecordingChatModel(_fair_handler)
        graph_fair = build_ai_gm_graph(chat_model=model_fair, checkpointer=MemorySaver())
        out_fair = graph_fair.invoke(
            {
                "session_id": sid_fair,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "Check heat repeatedly.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid_fair}},
        )
        assert "All 3 execution rounds finished" in out_fair["final_reply"]
        assert [inv["tool_name"] for inv in out_fair["tool_invocations"]] == [
            LOAD_GM_TOOLS_NAME,
            "get_world_clocks_and_heat",
            "get_world_clocks_and_heat",
            "get_world_clocks_and_heat",
        ]

        # After 2 load_gm_tools rounds in one turn, load_gm_tools is removed from bound tools
        cap_steps: list[int] = [0]

        def _cap_handler(_idx: int, _messages: list[BaseMessage], bound_names: list[str]) -> AIMessage:
            step = cap_steps[0]
            cap_steps[0] += 1
            if step == 0:
                assert LOAD_GM_TOOLS_NAME in bound_names
                return AIMessage(
                    content="",
                    tool_calls=[
                        {"name": LOAD_GM_TOOLS_NAME, "args": {"packs": ["combat"]}, "id": "cap-l1"}
                    ],
                )
            if step == 1:
                assert LOAD_GM_TOOLS_NAME in bound_names
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": LOAD_GM_TOOLS_NAME,
                            "args": {"packs": ["social_and_psyche"]},
                            "id": "cap-l2",
                        }
                    ],
                )
            assert LOAD_GM_TOOLS_NAME not in bound_names
            return AIMessage(content="Loader capped after 2 discovery rounds.")

        model_cap = _RecordingChatModel(_cap_handler)
        graph_cap = build_ai_gm_graph(chat_model=model_cap, checkpointer=MemorySaver())
        out_cap = graph_cap.invoke(
            {
                "session_id": sid_cap,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "Load twice.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid_cap}},
        )
        assert "Loader capped" in out_cap["final_reply"]
        assert len(model_cap.bound_history) == 3
        assert LOAD_GM_TOOLS_NAME not in model_cap.bound_history[2]
    finally:
        reset_session_npcs(sid_fair)
        reset_session_npcs(sid_cap)


def test_registry_query_forgiving_normalization_and_standalone_tool() -> None:
    set_campaign_pack_override("awakened")
    try:
        registry = GmToolRegistry.from_tools(get_builtin_gm_tools())

        res_query = registry.activate_tools(query="explosive blast")
        assert res_query["status"] == "ok"
        assert "tech_and_tradecraft" in res_query["loaded_packs"]
        assert "resolve_explosive_vehicle_or_hazard" in res_query["loaded_tool_names"]

        res_str_pack = registry.activate_tools(packs="social_and_psyche")
        assert res_str_pack["status"] == "ok"
        assert "social_and_psyche" in res_str_pack["loaded_packs"]
        assert "perform_empathy_read" in res_str_pack["loaded_tool_names"]

        res_str_tool = registry.activate_tools(tool_names="perform_empathy_read")
        assert res_str_tool["status"] == "ok"
        assert res_str_tool["loaded_tool_names"] == ["perform_empathy_read"]

        res_unknown = registry.activate_tools(packs="nonexistent_pack")
        assert res_unknown["status"] == "no_match"
        assert "nonexistent_pack" in res_unknown["unmatched"]
        assert set(res_unknown["available_packs"]) == set(DEFERRED_TOOL_PACKS.keys())

        standalone = load_gm_tools.invoke({"query": "explosive blast"})
        assert standalone["status"] == "ok"
        assert "resolve_explosive_vehicle_or_hazard" in standalone["loaded_tool_names"]
    finally:
        set_campaign_pack_override(None)


def test_direct_call_to_unloaded_deferred_tool_still_executes() -> None:
    sid = f"lazy-direct-{uuid.uuid4().hex[:8]}"
    try:
        def _handler(_idx: int, messages: list[BaseMessage], bound_names: list[str]) -> AIMessage:
            last = messages[-1]
            if getattr(last, "type", "") == "tool":
                payload = json.loads(str(last.content))
                assert payload["operational_heat"] >= 0
                return AIMessage(content="Heat ledger inspected directly.")
            assert "get_world_clocks_and_heat" not in bound_names
            return AIMessage(
                content="",
                tool_calls=[
                    {
                        "name": "get_world_clocks_and_heat",
                        "args": {},
                        "id": "call-direct-deferred",
                    }
                ],
            )

        model = _RecordingChatModel(_handler)
        graph = build_ai_gm_graph(chat_model=model, checkpointer=MemorySaver())
        out = graph.invoke(
            {
                "session_id": sid,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "What is our Heat level?",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid}},
        )
        assert "Heat ledger inspected directly." in out["final_reply"]
        assert out["tool_invocations"][0]["is_error"] is False
    finally:
        reset_session_npcs(sid)


def test_eager_mode_small_custom_tools_and_external_mcp_packs() -> None:
    sid_eager = f"lazy-eager-{uuid.uuid4().hex[:8]}"
    sid_small = f"lazy-small-{uuid.uuid4().hex[:8]}"
    sid_ext = f"lazy-ext-{uuid.uuid4().hex[:8]}"
    try:
        model_eager = _RecordingChatModel(lambda _i, _m, _b: AIMessage(content="Eager ok."))
        graph_eager = build_ai_gm_graph(
            chat_model=model_eager,
            checkpointer=MemorySaver(),
            lazy_tool_loading=False,
        )
        graph_eager.invoke(
            {
                "session_id": sid_eager,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "Hello.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid_eager}},
        )
        assert len(model_eager.bound_history[0]) == 45
        assert LOAD_GM_TOOLS_NAME not in model_eager.bound_history[0]
        assert model_eager.system_prompts[0] == AUTONOMOUS_SYSTEM_PROMPT

        model_small = _RecordingChatModel(lambda _i, _m, _b: AIMessage(content="Small ok."))
        graph_small = build_ai_gm_graph(
            chat_model=model_small,
            tools=[lookup_npc_profile],
            checkpointer=MemorySaver(),
        )
        graph_small.invoke(
            {
                "session_id": sid_small,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "Hello.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid_small}},
        )
        assert model_small.bound_history[0] == ["lookup_npc_profile"]
        assert model_small.system_prompts[0] == AUTONOMOUS_SYSTEM_PROMPT

        def _fetch_campaign_doc(doc_name: str) -> str:
            """Fetch a campaign Google Doc by title."""
            return f"gdrive:{doc_name}"

        def _ping_custom_webhook(target: str) -> str:
            """Send a custom external MCP webhook."""
            return f"webhook:{target}"

        gdrive_tool = StructuredTool.from_function(
            func=_fetch_campaign_doc,
            name="fetch_gdrive_doc",
            description="Fetch a campaign Google Doc by title.",
            metadata={"mcp_server_id": "gdrive"},
        )
        untagged_tool = StructuredTool.from_function(
            func=_ping_custom_webhook,
            name="ping_custom_webhook",
            description="Send a custom external MCP webhook.",
        )

        def _ext_handler(_idx: int, messages: list[BaseMessage], bound_names: list[str]) -> AIMessage:
            last = messages[-1]
            last_type = getattr(last, "type", "")
            last_name = getattr(last, "name", "")
            if last_type == "human":
                assert "fetch_gdrive_doc" not in bound_names
                assert "ping_custom_webhook" not in bound_names
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": LOAD_GM_TOOLS_NAME,
                            "args": {"packs": ["mcp_gdrive", "external_mcp"]},
                            "id": "call-ext-load",
                        }
                    ],
                )
            if last_type == "tool" and last_name == LOAD_GM_TOOLS_NAME:
                assert "fetch_gdrive_doc" in bound_names
                assert "ping_custom_webhook" in bound_names
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": "fetch_gdrive_doc",
                            "args": {"doc_name": "Chrysalis"},
                            "id": "call-ext-exec",
                        }
                    ],
                )
            return AIMessage(content="Fetched external doc.")

        model_ext = _RecordingChatModel(_ext_handler)
        graph_ext = build_ai_gm_graph(
            chat_model=model_ext,
            tools=get_builtin_gm_tools() + [gdrive_tool, untagged_tool],
            checkpointer=MemorySaver(),
        )
        out_ext = graph_ext.invoke(
            {
                "session_id": sid_ext,
                "operating_mode": "autonomous",
                "trigger_event": {
                    "content": "Read Google Doc Chrysalis.",
                    "actor": {"user_id": "u1", "display_name": "Kaelen"},
                },
            },
            config={"configurable": {"thread_id": sid_ext}},
        )
        assert "Fetched external doc." in out_ext["final_reply"]
        assert "mcp_gdrive" in model_ext.system_prompts[0]
        assert "external_mcp" in model_ext.system_prompts[0]
        assert len(model_ext.bound_history[0]) == 11
        assert len(model_ext.bound_history[1]) == 13
    finally:
        reset_session_npcs(sid_eager)
        reset_session_npcs(sid_small)
        reset_session_npcs(sid_ext)


def test_satya_gm_tool_spec_registry_parity() -> None:
    """Verify `@satya_gm_tool` single-source metadata derives all packs, OOC tools, tiers, and scoping."""
    from pathlib import Path
    from satya_ml.ai_gm.tool_spec import (
        classify_registered_tool_tier,
        get_core_tool_names,
        get_ooc_readonly_tool_names,
        get_registered_tool_specs,
        is_registered_session_scoped,
    )

    specs = get_registered_tool_specs(include_meta=False)
    builtin_tools = get_builtin_gm_tools()
    assert len(specs) == 45
    assert set(specs) == {t.name for t in builtin_tools}
    assert set(CORE_GM_TOOL_NAMES) == set(get_core_tool_names())
    assert set(OOC_READONLY_TOOL_NAMES) == set(get_ooc_readonly_tool_names())
    assert len(OOC_READONLY_TOOL_NAMES) == 7

    for pack_id, pack_spec in DEFERRED_TOOL_PACKS.items():
        expected = {n for n, sp in specs.items() if sp.pack == pack_id}
        assert set(pack_spec.tool_names) == expected

    for name, sp in specs.items():
        assert _classify_tool_tier(name) == classify_registered_tool_tier(name)
        assert _classify_tool_tier(name) == (sp.tier, sp.tier_label)

    assert is_registered_session_scoped("query_satya_rules") is False
    assert is_registered_session_scoped(LOAD_GM_TOOLS_NAME) is False
    assert is_registered_session_scoped("lookup_npc_profile") is True

    root = Path(__file__).resolve().parent.parent / "src" / "satya_ml"
    fallback_files = [
        p.relative_to(root).as_posix()
        for p in root.rglob("*.py")
        if "_FallbackToolWrapper" in p.read_text(encoding="utf-8")
    ]
    assert fallback_files == ["ai_gm/tool_spec.py"]

