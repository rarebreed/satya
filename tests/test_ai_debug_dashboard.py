"""Tests for the FastAPI + SSE Live Graph Dashboard, Breakpoint Debugger, Trace Store & Scenario Runner."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from fastapi.testclient import TestClient
from langchain_core.messages import AIMessage
import pytest


@pytest.fixture(autouse=True)
def _debug_bridge_token(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("SATYA_AI_GM_SERVICE_TOKEN", "satya_gm_secret_dev_token")

from satya_ml.ai_gm.graph import build_ai_gm_graph
from satya_ml.ai_gm.models import SessionLlmConfig
from satya_ml.ai_gm.server import app as gm_app
from satya_ml.ai_gm.tools import reset_session_npcs
from satya_ml.ai_player.server import app as player_app
from satya_ml.debug_dashboard import (
    GRAPH_HUB,
    GraphExecutionHub,
    run_observed_graph,
)
from satya_ml.signaling_client import offline_signaling_scope, signaling_base_url


class _SimpleToolThenReplyModel:
    def __init__(self, bound: bool = False) -> None:
        self._bound = bound

    def bind_tools(self, _tools: Any) -> _SimpleToolThenReplyModel:
        return _SimpleToolThenReplyModel(bound=True)

    def invoke(self, messages: list[Any]) -> AIMessage:
        from langchain_core.messages import ToolMessage

        if any(isinstance(m, ToolMessage) for m in messages):
            return AIMessage(content="Observed narrative reply after tool call.")
        return AIMessage(
            content="",
            tool_calls=[
                {
                    "name": "roll_satya_d20_check",
                    "args": {
                        "actor_name": "Tester",
                        "check_category": "OBSERVATION",
                        "attribute_tn": 14,
                        "skill_rank": 2,
                        "difficulty_mod": 0,
                        "note": "Observed graph test",
                    },
                    "id": "obs-tc-1",
                }
            ],
        )


def test_offline_signaling_scope_restores_env(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("SATYA_SIGNALING_URL", "http://127.0.0.1:3000")
    assert signaling_base_url() == "http://127.0.0.1:3000"

    with offline_signaling_scope(True):
        assert signaling_base_url() == ""
        assert os.environ["SATYA_SIGNALING_URL"] == "http://127.0.0.1:3000"

    assert signaling_base_url() == "http://127.0.0.1:3000"


def test_observed_graph_tracks_all_gm_nodes_and_tool_loop() -> None:
    session_id = "dbg-obs-gm-1"
    reset_session_npcs(session_id)
    hub = GraphExecutionHub(trace_dir=None, auto_load_traces=False)
    cfg = SessionLlmConfig(provider="ollama", model_id="test-obs")
    graph = build_ai_gm_graph(
        cfg,
        chat_model=_SimpleToolThenReplyModel(),
        lazy_tool_loading=True,
    )
    initial_state = {
        "session_id": session_id,
        "session_mode": "TTRPG",
        "operating_mode": "autonomous",
        "roster": [],
        "trigger_event": {
            "event_id": "ev-obs-1",
            "trigger_kind": "CHAT",
            "actor": {"user_id": "u1", "display_name": "Tester"},
            "channel": "PUBLIC",
            "content": "I inspect the relay terminal.",
        },
        "tool_invocations": [],
    }
    with offline_signaling_scope(True):
        result, run = run_observed_graph(
            graph,
            initial_state,
            {"configurable": {"thread_id": session_id}},
            agent_kind="ai_gm",
            session_id=session_id,
            provider="mock",
            model_id="test-obs",
            trigger_summary="I inspect the relay terminal.",
            source="debug_ui",
            hub=hub,
        )

    assert result["final_reply"] == "Observed narrative reply after tool call."
    assert run.status == "completed"
    visited = [s.node_name for s in run.steps]
    assert visited == [
        "sync_session_context",
        "party_turn_gate_node",
        "scenario_director_node",
        "npc_action_resolution_node",
        "gm_supervisor_node",
        "tool_execution_node",
        "gm_supervisor_node",
        "publish_gm_response",
    ]
    assert all(s.status == "completed" for s in run.steps)
    assert len(run.tool_invocations) == 1
    assert run.tool_invocations[0]["tool_name"] == "roll_satya_d20_check"
    assert any(s.state_delta for s in run.steps)


def test_debug_ui_routes_topology_and_session_lifecycle() -> None:
    client = TestClient(gm_app, headers={"Authorization": "Bearer satya_gm_secret_dev_token"})
    GRAPH_HUB.clear_traces(delete_files=False)

    html_resp = client.get("/debug")
    assert html_resp.status_code == 200
    assert "SATYA // LANGGRAPH DEBUGGER" in html_resp.text

    js_resp = client.get("/debug/static/app.js")
    assert js_resp.status_code == 200
    assert "EventSource" in js_resp.text

    topo_resp = client.get("/debug/api/topology")
    assert topo_resp.status_code == 200
    topo = topo_resp.json()
    gm_nodes = {n["id"] for n in topo["graphs"]["ai_gm"]["nodes"]}
    player_nodes = {n["id"] for n in topo["graphs"]["ai_player"]["nodes"]}
    assert {
        "sync_session_context",
        "party_turn_gate_node",
        "scenario_director_node",
        "npc_action_resolution_node",
        "gm_supervisor_node",
        "tool_execution_node",
        "publish_gm_response",
    }.issubset(gm_nodes)
    assert {
        "sync_player_context",
        "player_decision_node",
        "player_tool_node",
        "publish_player_turn",
    }.issubset(player_nodes)

    sid = "dbg-lifecycle-1"
    seed_resp = client.post(
        f"/debug/api/sessions/{sid}/seed",
        json={"seed_config": {"seed_default_character": True, "seed_default_map": True}},
    )
    assert seed_resp.status_code == 200
    assert seed_resp.json()["seeded"] is True

    state_resp = client.get(f"/debug/api/sessions/{sid}/state")
    assert state_resp.status_code == 200
    snap = state_resp.json()
    assert len(snap["characters"]) == 1
    assert snap["active_map"] is not None

    reset_resp = client.post(f"/debug/api/sessions/{sid}/reset")
    assert reset_resp.status_code == 200

    after_reset = client.get(f"/debug/api/sessions/{sid}/state").json()
    assert after_reset["characters"] == []
    assert after_reset["active_map"] is None

    # Standalone AI Player server also mounts /debug cleanly
    player_client = TestClient(
        player_app,
        headers={"Authorization": "Bearer satya_gm_secret_dev_token"},
    )
    assert player_client.get("/debug/api/topology").status_code == 200


def test_interrupt_before_pause_step_patch_and_resume() -> None:
    client = TestClient(gm_app, headers={"Authorization": "Bearer satya_gm_secret_dev_token"})
    sid = "dbg-breakpoint-1"
    client.post(f"/debug/api/sessions/{sid}/reset")

    invoke_resp = client.post(
        "/debug/api/invoke/gm",
        json={
            "session_id": sid,
            "prompt": "Check the bulkhead terminal.",
            "provider": "mock",
            "offline_mode": True,
            "interrupt_before": ["gm_supervisor_node"],
        },
    )
    assert invoke_resp.status_code == 200
    run_data = invoke_resp.json()["run"]
    run_id = run_data["run_id"]
    assert run_data["status"] == "paused"
    assert run_data["next_nodes"] == ["gm_supervisor_node"]
    assert [s["node_name"] for s in run_data["steps"]] == [
        "sync_session_context",
        "party_turn_gate_node",
        "scenario_director_node",
        "npc_action_resolution_node",
    ]

    # Single-step executes gm_supervisor_node and pauses before tool_execution_node
    step_resp = client.post(
        f"/debug/api/runs/{run_id}/resume",
        json={"step_only": True},
    )
    assert step_resp.status_code == 200
    stepped = step_resp.json()["run"]
    assert stepped["status"] == "paused"
    assert stepped["next_nodes"] == ["tool_execution_node"]
    assert stepped["steps"][-1]["node_name"] == "gm_supervisor_node"

    # Patch state and continue to completion
    resume_resp = client.post(
        f"/debug/api/runs/{run_id}/resume",
        json={
            "step_only": False,
            "state_patch": {"active_director_mandate": "Patched Mandate"},
        },
    )
    assert resume_resp.status_code == 200
    finished = resume_resp.json()["run"]
    assert finished["status"] == "completed"
    assert finished["next_nodes"] == []
    assert finished["steps"][-1]["node_name"] == "publish_gm_response"
    assert finished["state_preview"]["active_director_mandate"] == "Patched Mandate"

    # Also verify /abort on a paused run
    paused2 = client.post(
        "/debug/api/invoke/player",
        json={
            "session_id": sid,
            "scene_prompt": "Peer down the hallway.",
            "provider": "mock",
            "offline_mode": True,
            "interrupt_before": ["player_decision_node"],
        },
    ).json()["run"]
    assert paused2["status"] == "paused"
    assert paused2["next_nodes"] == ["player_decision_node"]
    abort_resp = client.post(f"/debug/api/runs/{paused2['run_id']}/abort")
    assert abort_resp.status_code == 200
    assert abort_resp.json()["run"]["status"] == "aborted"


def test_trace_persistence_redaction_and_reload(tmp_path: Path) -> None:
    trace_dir = tmp_path / "debug_runs"
    hub = GraphExecutionHub(trace_dir=trace_dir, max_persisted_traces=2, auto_load_traces=False)
    cfg = SessionLlmConfig(provider="ollama", model_id="trace-model")
    graph = build_ai_gm_graph(
        cfg,
        chat_model=_SimpleToolThenReplyModel(),
        lazy_tool_loading=True,
    )

    for idx in range(3):
        sid = f"dbg-trace-{idx}"
        reset_session_npcs(sid)
        with offline_signaling_scope(True):
            _res, run = run_observed_graph(
                graph,
                {
                    "session_id": sid,
                    "session_mode": "TTRPG",
                    "operating_mode": "autonomous",
                    "roster": [],
                    "trigger_event": {
                        "event_id": f"ev-tr-{idx}",
                        "trigger_kind": "CHAT",
                        "actor": {"user_id": "u1", "display_name": "Tester"},
                        "channel": "PUBLIC",
                        "content": f"Check terminal with api_key=super_secret_token_{idx}",
                    },
                    "tool_invocations": [],
                },
                {"configurable": {"thread_id": sid}},
                agent_kind="ai_gm",
                session_id=sid,
                provider="mock",
                model_id="trace-model",
                trigger_summary=f"Bearer top_secret_bearer_{idx}",
                hub=hub,
            )
        assert run.persisted_path is not None

    # Retention cap = 2 files
    files = sorted(trace_dir.glob("*.json"))
    assert len(files) == 2
    raw_text = files[-1].read_text(encoding="utf-8")
    assert "super_secret_token_" not in raw_text
    assert "top_secret_bearer_" not in raw_text
    assert "[REDACTED]" in raw_text

    # Reload into fresh hub
    reloaded = GraphExecutionHub(trace_dir=trace_dir, max_persisted_traces=10, auto_load_traces=True)
    summaries = reloaded.list_runs(limit=10)
    assert len(summaries) == 2
    parsed = json.loads(raw_text)
    assert parsed["agent_kind"] == "ai_gm"
    assert len(parsed["steps"]) == 8


def test_builtin_scenarios_and_player_gm_loop() -> None:
    client = TestClient(gm_app, headers={"Authorization": "Bearer satya_gm_secret_dev_token"})
    presets_resp = client.get("/debug/api/scenarios")
    assert presets_resp.status_code == 200
    preset_ids = {s["id"] for s in presets_resp.json()["scenarios"]}
    assert {
        "single_player_opening",
        "ooc_rules_question",
        "two_player_turn_gate",
        "combat_action_mode",
        "player_vs_gm_loop_3_turns",
    }.issubset(preset_ids)

    for pid in (
        "single_player_opening",
        "ooc_rules_question",
        "two_player_turn_gate",
        "combat_action_mode",
        "player_vs_gm_loop_3_turns",
    ):
        res = client.post(
            "/debug/api/scenarios/run",
            json={"preset_id": pid, "provider": "mock", "offline_mode": True},
        )
        assert res.status_code == 200
        body = res.json()
        assert body["passed"] is True, f"Scenario {pid} failed: {body['assertion_results']}"


@pytest.mark.anyio
async def test_hub_threadsafe_sse_pubsub() -> None:
    hub = GraphExecutionHub(trace_dir=None, auto_load_traces=False)
    sub_id, queue = hub.subscribe()
    try:
        hub.publish_event("node_started", {"run_id": "r1", "node": "sync_session_context"})
        evt = await queue.get()
        assert evt["type"] == "node_started"
        assert evt["data"]["run_id"] == "r1"
    finally:
        hub.unsubscribe(sub_id)


def test_debug_ui_disabled_on_non_loopback_host(monkeypatch: pytest.MonkeyPatch) -> None:
    client = TestClient(gm_app, headers={"Authorization": "Bearer satya_gm_secret_dev_token"})
    monkeypatch.delenv("SATYA_AI_DEBUG_UI", raising=False)
    monkeypatch.setenv("SATYA_AI_GM_HOST", "0.0.0.0")
    assert client.get("/debug/api/topology").status_code == 403

    monkeypatch.setenv("SATYA_AI_DEBUG_UI", "1")
    assert client.get("/debug/api/topology").status_code == 200


def test_debug_ui_serves_animated_brain_icon() -> None:
    client = TestClient(gm_app, headers={"Authorization": "Bearer satya_gm_secret_dev_token"})
    html_res = client.get("/debug/")
    assert html_res.status_code == 200
    assert "/debug/static/debug-brain.svg" in html_res.text

    icon_res = client.get("/debug/static/debug-brain.svg")
    assert icon_res.status_code == 200
    assert icon_res.headers["content-type"].startswith("image/svg+xml")
    assert b"<svg" in icon_res.content
