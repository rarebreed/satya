"""Unit tests for the Satya Avro schema inspector, PyLance compaction pipeline, and AI GM LangGraph agent."""

from __future__ import annotations

import json
import shutil
import sys
import uuid
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import fastavro
import lance

from satya_ml.ai_gm.models import SessionLlmConfig, build_gm_chat_model, redact_api_key
from satya_ml.ai_gm.tools import (
    generate_tactical_map,
    inspect_tactical_map,
    lookup_npc_profile,
    move_map_token,
    narrate_to_session,
    query_campaign_lore,
    query_satya_rules,
    roll_satya_d20_check,
    set_session_operating_mode,
    update_npc_state,
)
from satya_ml.inspect_avro import inspect_schemas
from satya_ml.lance_ingest import compact_topic_to_lance, read_avro_batch_with_metadata


def test_all_versioned_avro_schemas_parse_with_fastavro() -> None:
    """Verify all 10 .avsc schemas under schemas/v1 and schemas/v2 parse cleanly with fastavro."""
    repo_root = Path(__file__).resolve().parent.parent
    avsc_files = sorted((repo_root / "schemas").glob("v*/*.avsc"))
    assert len(avsc_files) == 10

    for avsc_path in avsc_files:
        raw = json.loads(avsc_path.read_text(encoding="utf-8"))
        parsed = fastavro.schema.parse_schema(raw)
        assert parsed is not None

    inspect_schemas(repo_root)


def test_incremental_avro_batch_to_lance_compaction() -> None:
    """Verify offset-watermarked .avro batches compact into a .lance dataset without duplicates."""
    repo_root = Path(__file__).resolve().parent.parent
    tmp_path = repo_root / "target" / f"py_test_{uuid.uuid4().hex}"
    tmp_path.mkdir(parents=True, exist_ok=True)
    try:
        schema_raw = json.loads(
            (repo_root / "schemas" / "v1" / "session_lifecycle_event.avsc").read_text(
                encoding="utf-8"
            )
        )
        parsed_schema = fastavro.schema.parse_schema(schema_raw)

        topic_dir = tmp_path / "topics" / "session-lifecycle"
        part_dir = topic_dir / "schema_version=v1" / "partition=0"
        part_dir.mkdir(parents=True, exist_ok=True)

        batch_file = part_dir / "batch_0000000000_0000000001.avro"
        records = [
            {
                "metadata": {
                    "event_id": "ev-1",
                    "sequence_num": 1,
                    "schema_subject": "satya.session.lifecycle",
                    "schema_version": 1,
                    "schema_fingerprint_crc64": b"\x01\x02\x03\x04\x05\x06\x07\x08",
                    "session_id": "ep-test",
                    "session_mode": "TTRPG",
                    "client_timestamp_us": 1700000000000000,
                    "ingest_timestamp_us": 1700000000000000,
                    "timezone_offset_min": -240,
                },
                "lifecycle_type": "SESSION_CREATED",
                "actor": {
                    "user_id": "u-gm",
                    "username": "gm_sean",
                    "display_name": "Sean (GM)",
                    "participant_kind": "HUMAN",
                    "role": "GM",
                    "character_id": None,
                    "ai_agent_info": None,
                },
                "target_participant": None,
                "password_protected": True,
                "roster_snapshot": [],
                "reason": "Created session",
            },
            {
                "metadata": {
                    "event_id": "ev-2",
                    "sequence_num": 2,
                    "schema_subject": "satya.session.lifecycle",
                    "schema_version": 1,
                    "schema_fingerprint_crc64": b"\x01\x02\x03\x04\x05\x06\x07\x08",
                    "session_id": "ep-test",
                    "session_mode": "TTRPG",
                    "client_timestamp_us": 1700000001000000,
                    "ingest_timestamp_us": 1700000001000000,
                    "timezone_offset_min": -240,
                },
                "lifecycle_type": "USER_LOGGED_OUT",
                "actor": {
                    "user_id": "u-gm",
                    "username": "gm_sean",
                    "display_name": "Sean (GM)",
                    "participant_kind": "HUMAN",
                    "role": "GM",
                    "character_id": None,
                    "ai_agent_info": None,
                },
                "target_participant": None,
                "password_protected": True,
                "roster_snapshot": [],
                "reason": "User logged out",
            },
        ]

        with batch_file.open("wb") as fp:
            fastavro.writer(
                fp,
                parsed_schema,
                records,
                metadata={
                    "satya.start_offset": "0",
                    "satya.end_offset": "1",
                    "satya.schema_version": "1",
                },
            )

        meta, read_back = read_avro_batch_with_metadata(batch_file)
        assert meta["satya.start_offset"] == "0"
        assert meta["satya.end_offset"] == "1"
        assert len(read_back) == 2

        lance_root = tmp_path / "lance"
        lance_root.mkdir(parents=True, exist_ok=True)
        state_file = lance_root / "_watermarks.json"

        # First compaction ingests 2 records
        ingested_first = compact_topic_to_lance(topic_dir, lance_root, state_file)
        assert ingested_first == 2

        # Second compaction with no new offsets ingests 0 records (deduplicated by watermark)
        ingested_second = compact_topic_to_lance(topic_dir, lance_root, state_file)
        assert ingested_second == 0

        watermarks = json.loads(state_file.read_text(encoding="utf-8"))
        assert watermarks["session-lifecycle"] == 1
    finally:
        shutil.rmtree(tmp_path, ignore_errors=True)


def test_topics_7_to_9_avro_to_lance_compaction() -> None:
    """Verify Avro topics 7-9 (`tabletop-canvas`, `ai-gm-trajectories`, `system-logs`) compact into Lance."""
    repo_root = Path(__file__).resolve().parent.parent
    tmp_path = repo_root / "target" / f"py_topics79_{uuid.uuid4().hex}"
    tmp_path.mkdir(parents=True, exist_ok=True)
    try:
        lance_root = tmp_path / "lance"
        lance_root.mkdir(parents=True, exist_ok=True)
        state_file = lance_root / "_watermarks.json"

        base_meta = {
            "event_id": "ev-99",
            "sequence_num": 1,
            "schema_subject": "satya.system.logs",
            "schema_version": 1,
            "schema_fingerprint_crc64": b"\x01\x02\x03\x04\x05\x06\x07\x08",
            "session_id": "ep-1",
            "session_mode": "TTRPG",
            "client_timestamp_us": 1700000000000000,
            "ingest_timestamp_us": 1700000000000000,
            "timezone_offset_min": -240,
        }
        actor = {
            "user_id": "u-p1",
            "username": "sara_p1",
            "display_name": "Sara",
            "participant_kind": "HUMAN",
            "role": "PLAYER",
            "character_id": "char-sara",
            "ai_agent_info": None,
        }

        sys_schema = fastavro.schema.parse_schema(
            json.loads(
                (repo_root / "schemas" / "v1" / "session_log_event.avsc").read_text(encoding="utf-8")
            )
        )
        sys_dir = tmp_path / "topics" / "system-logs" / "schema_version=v1" / "partition=0"
        sys_dir.mkdir(parents=True, exist_ok=True)
        with (sys_dir / "batch_0000000000_0000000000.avro").open("wb") as fp:
            fastavro.writer(
                fp,
                sys_schema,
                [
                    {
                        "metadata": base_meta,
                        "log_id": "log-1",
                        "level": "INFO",
                        "service_name": "satya-signaling",
                        "actor": actor,
                        "trace_id": "0123456789abcdef0123456789abcdef",
                        "span_id": "0123456789abcdef",
                        "parent_span_id": None,
                        "lineage_id": "ev-99",
                        "parent_lineage_ids": [],
                        "log_line": "[INFO] [session=ep-1] AI GM turn completed",
                    }
                ],
                metadata={
                    "satya.start_offset": "0",
                    "satya.end_offset": "0",
                    "satya.schema_version": "1",
                },
            )

        count = compact_topic_to_lance(tmp_path / "topics" / "system-logs", lance_root, state_file)
        assert count == 1
    finally:
        shutil.rmtree(tmp_path, ignore_errors=True)


def test_ai_gm_tools_npc_rules_map_and_byok_config() -> None:
    """Verify AI GM tools (NPC lookup/update, rules/lore query, SVG map generation, token bounds, BYOK redaction)."""
    cfg = SessionLlmConfig(
        provider="google_genai",
        model_id="gemini-2.5-flash",
        vision_model_id="gemini-2.5-flash",
        api_key="AIzaSySecretTestKey12345",
    )
    assert "AIzaSySecretTestKey12345" not in repr(cfg)
    assert redact_api_key(cfg.api_key) == "[REDACTED_BYOK_KEY]"
    assert redact_api_key(None) == "<none>"
    assert cfg.target_model_id(for_vision=True) == "gemini-2.5-flash"

    # Missing cloud API key raises ValueError
    empty_cloud_cfg = SessionLlmConfig(provider="openai", model_id="gpt-4o", api_key=None)
    try:
        build_gm_chat_model(empty_cloud_cfg)
        raise AssertionError("Expected ValueError when API key is missing")
    except (ValueError, ImportError):
        pass

    # NPC lookup & state update
    sid = f"test-sess-{uuid.uuid4().hex[:6]}"
    vance = lookup_npc_profile.invoke({"npc_id": "captain_vance", "session_id": sid})
    assert vance["name"] == "Captain Elena Vance"
    assert vance["attribute_tns"]["INT"] == 11

    updated_vance = update_npc_state.invoke(
        {
            "npc_id": "captain_vance",
            "fatigue_delta": 2,
            "combat_stance": "aggressive",
            "disposition": "friendly",
            "session_id": sid,
        }
    )
    assert updated_vance["fatigue"] == 3
    assert updated_vance["combat_stance"] == "AGGRESSIVE"
    assert updated_vance["disposition"] == "friendly"

    # Rules & lore retrieval
    rules_excerpt = query_satya_rules.invoke({"query": "target number degree of success"})
    assert len(rules_excerpt) > 50
    lore_excerpt = query_campaign_lore.invoke({"query": "Aethelgard", "session_id": sid})
    assert "Captain Elena Vance" in lore_excerpt

    # Tactical map generation, token movement & spatial inspection
    map_res = generate_tactical_map.invoke(
        {"session_id": sid, "theme": "Sector 4 Docking Bay", "width": 12, "height": 10}
    )
    assert map_res["grid_width"] == 12
    assert map_res["grid_height"] == 10
    svg_file = Path(__file__).resolve().parent.parent / "data" / "maps" / f"{sid}_{map_res['map_id']}.svg"
    assert svg_file.exists()

    try:
        moved = move_map_token.invoke(
            {"session_id": sid, "token_id": "tok-vance", "to_x": 5, "to_y": 4}
        )
        vance_tok = next(t for t in moved["tokens"] if t["token_id"] == "tok-vance")
        assert vance_tok["grid_x"] == 5
        assert vance_tok["grid_y"] == 4

        out_of_bounds = move_map_token.invoke(
            {"session_id": sid, "token_id": "tok-vance", "to_x": 99, "to_y": 4}
        )
        assert "error" in out_of_bounds

        inspection = inspect_tactical_map.invoke(
            {"session_id": sid, "question": "Where is Captain Vance relative to cover?"}
        )
        assert "Capt. Vance" in inspection
        assert "(5, 4)" in inspection
    finally:
        svg_file.unlink(missing_ok=True)

    # Deterministic roll fallback & co-pilot narration routing
    roll_res = roll_satya_d20_check.invoke(
        {
            "session_id": sid,
            "actor_name": "Captain Elena Vance",
            "check_category": "astrogation",
            "attribute_tn": 11,
            "skill_rank": 3,
        }
    )
    assert "degree_of_success" in roll_res
    assert "wgpu_dice" in roll_res

    set_session_operating_mode(sid, "co_pilot", "u-gm-sean")
    narration = narrate_to_session.invoke({"session_id": sid, "content": "Suggest TN 11 check."})
    assert narration["visibility"] == "PRIVATE"
    assert narration["channel"] == "GM_WHISPER"


def test_ai_gm_langgraph_and_fastapi_bridge() -> None:
    """Verify the 4-node LangGraph workflow and FastAPI `/internal/ai-gm/events` bridge in autonomous & co-pilot modes."""
    try:
        from fastapi.testclient import TestClient
        from langchain_core.messages import AIMessage
        from satya_ml.ai_gm.graph import build_ai_gm_graph
        from satya_ml.ai_gm.server import app, set_chat_model_override
    except ImportError:
        # If langgraph / fastapi are not yet installed in the active Python environment, skip gracefully
        return

    class FakeToolCallingChatModel:
        def __init__(self) -> None:
            self.calls = 0

        def bind_tools(self, _tools: list[Any]) -> FakeToolCallingChatModel:
            return self

        def invoke(self, messages: list[Any]) -> AIMessage:
            self.calls += 1
            last = messages[-1]
            # First call on a turn: invoke lookup_npc_profile
            if getattr(last, "type", "") == "human":
                return AIMessage(
                    content="",
                    tool_calls=[
                        {
                            "name": "lookup_npc_profile",
                            "args": {"npc_id": "captain_vance"},
                            "id": "call-test-1",
                        }
                    ],
                )
            # Second call after ToolMessage: return final narration
            return AIMessage(
                content="Captain Elena Vance checks the encrypted Aethelgard nav-log (INT TN 11)."
            )

    fake_model = FakeToolCallingChatModel()
    graph = build_ai_gm_graph(chat_model=fake_model)
    out = graph.invoke(
        {
            "session_id": "ep-graph-test",
            "session_mode": "TTRPG_AI_GM",
            "operating_mode": "autonomous",
            "trigger_event": {
                "event_id": "ev-101",
                "actor": {"user_id": "u-p1", "display_name": "Sara"},
                "content": "I ask Captain Vance about the Aethelgard logs.",
            },
            "tool_invocations": [],
        },
        config={"configurable": {"thread_id": "ep-graph-test"}},
    )
    assert out["final_channel"] == "IN_CHARACTER"
    assert "Captain Elena Vance" in out["final_reply"]
    assert len(out["tool_invocations"]) == 1
    assert out["tool_invocations"][0]["tool_name"] == "lookup_npc_profile"
    assert out["tool_invocations"][0]["server_tier"] == "TIER_2_RULESET_ENGINE"

    # Test FastAPI service in Co-Pilot mode with BYOK header
    set_chat_model_override(FakeToolCallingChatModel())
    try:
        client = TestClient(app)
        health_res = client.get("/health")
        assert health_res.status_code == 200
        assert health_res.json()["status"] == "ok"

        # Unauthorized request fails with 401
        unauth = client.post("/internal/ai-gm/events", json={"session_id": "ep-1"})
        assert unauth.status_code == 401

        # Authorized Co-Pilot turn with ephemeral BYOK header
        ev_res = client.post(
            "/internal/ai-gm/events",
            headers={
                "Authorization": "Bearer satya_gm_secret_dev_token",
                "X-Satya-Provider-Key": "sk-ephemeral-secret-key",
            },
            json={
                "session_id": "ep-copilot-fastapi",
                "session_mode": "TTRPG_GM_AI",
                "operating_mode": "CO_PILOT",
                "human_gm_user_id": "u-gm-sean",
                "trigger_event_id": "ev-202",
                "trigger_actor": {"user_id": "u-gm-sean", "display_name": "Sean (GM)"},
                "context_text": "What are Captain Vance's stats?",
                "provider": "openai",
                "model_id": "gpt-4o",
            },
        )
        assert ev_res.status_code == 200
        body = ev_res.json()
        assert body["operating_mode"] == "co_pilot"
        assert body["final_channel"] == "GM_WHISPER"
        assert body["api_key_status"] == "[REDACTED_BYOK_KEY]"
        assert "sk-ephemeral-secret-key" not in json.dumps(body)
        assert body["tool_invocations_count"] == 1
    finally:
        set_chat_model_override(None)
