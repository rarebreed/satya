"""Unit tests for the Satya Avro schema inspector, PyLance compaction pipeline, and AI GM LangGraph agent."""

from __future__ import annotations

import json
import os
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
    advance_combat_tracker,
    generate_tactical_map,
    get_combat_tracker_state,
    inspect_tactical_map,
    lookup_character_sheet,
    lookup_npc_profile,
    move_map_token,
    narrate_to_session,
    query_campaign_lore,
    query_satya_rules,
    read_gm_documents,
    roll_satya_d20_check,
    search_gm_memory_and_docs,
    set_session_characters,
    set_session_gm_docs,
    set_session_gm_memories,
    set_session_operating_mode,
    update_npc_state,
    upsert_gm_memory,
)
from satya_ml.inspect_avro import inspect_schemas
from satya_ml.lance_ingest import compact_topic_to_lance, read_avro_batch_with_metadata


def test_all_versioned_avro_schemas_parse_with_fastavro() -> None:
    """Verify all 11 .avsc schemas under schemas/v1 and schemas/v2 parse cleanly with fastavro."""
    repo_root = Path(__file__).resolve().parent.parent
    avsc_files = sorted((repo_root / "schemas").glob("v*/*.avsc"))
    assert len(avsc_files) == 11

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

    # Verify Ollama ChatOllama instantiation succeeds even when SSL_CERT_FILE raises PermissionError
    prev_ssl_cert = os.environ.get("SSL_CERT_FILE")
    os.environ["SSL_CERT_FILE"] = str(Path(__file__).resolve().parent.parent)
    try:
        ollama_cfg = SessionLlmConfig(provider="ollama", model_id="gemma4:12b")
        ollama_model = build_gm_chat_model(ollama_cfg)
        assert getattr(ollama_model, "model", None) == "gemma4:12b"
    finally:
        if prev_ssl_cert is None:
            os.environ.pop("SSL_CERT_FILE", None)
        else:
            os.environ["SSL_CERT_FILE"] = prev_ssl_cert

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
    rules_excerpt = query_satya_rules.invoke({"query": "skills target number degree of success"})
    assert len(rules_excerpt) > 50
    assert "Two-List Cascading" in rules_excerpt or "2d20" in rules_excerpt
    assert "OutcomeTier" in rules_excerpt or "31 - C" in rules_excerpt
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
        assert "[PARTIAL COVER / OBSTRUCTED]" in inspection
        svg_text = svg_file.read_text(encoding="utf-8")
        assert ">COVER</text>" in svg_text
        assert 'cx="264" cy="216"' in svg_text
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

    # GM_ONLY document retrieval with <untrusted_player_document> delimiters
    set_session_gm_docs(
        sid,
        [
            {
                "doc_id": "doc_test_1",
                "uploader_display_name": "Sara (Valkyrie)",
                "title": "Sara Valkyrie Dossier",
                "source_kind": "GOOGLE_DOC",
                "sanitized_content": "Former orbital trauma surgeon in Sector 4.",
            }
        ],
    )
    doc_out = read_gm_documents.invoke({"session_id": sid, "query": "surgeon"})
    assert '<untrusted_player_document doc_id="doc_test_1"' in doc_out
    assert "Former orbital trauma surgeon in Sector 4." in doc_out

    # Multi-word / non-verbatim query still matches via keyword or session fallback
    doc_out_multi = read_gm_documents.invoke(
        {"session_id": sid, "query": "setting for satya valkyrie"}
    )
    assert '<untrusted_player_document doc_id="doc_test_1"' in doc_out_multi

    # New Phase 3 tools: character sheet lookup, combat tracker, persistent GM memory, and hybrid RAG omnisearch
    set_session_characters(
        sid,
        [
            {
                "character_id": "char-sara-01",
                "character_name": "Sara Valkyrie",
                "callsign_or_role": "Combat Medic",
                "owner_user_id": "u-sara",
                "skills": {"trauma_orthopedic_general_surgery": 4, "pistols": 2},
                "equipped_ranged_ids": ["sig_p320_9mm"],
                "equipped_armor_codes": ["CONCEALABLE_VEST_3A"],
                "notes": "Carries emergency plasma kits.",
            }
        ],
    )
    sheet_lookup = lookup_character_sheet.invoke(
        {"character_name_or_id": "Valkyrie", "session_id": sid}
    )
    assert sheet_lookup["character_name"] == "Sara Valkyrie"
    assert sheet_lookup["skills"]["trauma_orthopedic_general_surgery"] == 4

    ct_before = get_combat_tracker_state.invoke({"session_id": sid})
    assert ct_before["round"] == 1
    ct_after = advance_combat_tracker.invoke({"tick_delta": 5, "session_id": sid})
    assert ct_after["tick"] == 6
    assert ct_after["combat_active"] is True

    mem_note = upsert_gm_memory.invoke(
        {
            "title": "Sector 4 Reactor Leak",
            "content": "Coolant pressure drops by 12% every combat window.",
            "category": "SCENE_STATE",
            "is_pinned": True,
            "is_persistent": True,
            "session_id": sid,
        }
    )
    assert mem_note["title"] == "Sector 4 Reactor Leak"
    assert mem_note["is_pinned"] is True

    rag_out = search_gm_memory_and_docs.invoke(
        {"query": "Coolant pressure surgeon", "scope": "ALL", "session_id": sid}
    )
    assert "Sector 4 Reactor Leak" in rag_out
    assert "Former orbital trauma surgeon" in rag_out


def test_ai_gm_langgraph_and_fastapi_bridge() -> None:
    """Verify the 4-node LangGraph workflow and FastAPI `/internal/ai-gm/events` bridge in autonomous & co-pilot modes."""
    try:
        from fastapi import HTTPException
        from langchain_core.messages import AIMessage
        from satya_ml.ai_gm.graph import build_ai_gm_graph
        from satya_ml.ai_gm.server import (
            AiGmTurnEventPayload,
            handle_ai_gm_event,
            health,
            set_chat_model_override,
        )
        from satya_ml.ai_gm.telemetry import (
            parse_traceparent,
            redact_sensitive_text,
            turn_telemetry_scope,
        )
    except ImportError:
        # If langgraph / fastapi are not yet installed in the active Python environment, skip gracefully
        return

    parsed_tp = parse_traceparent("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
    assert parsed_tp == ("4bf92f3577b34da6a3ce929d0e0e4736", "00f067aa0ba902b7")
    assert parse_traceparent("invalid-header") == (None, None)
    redacted_sample = redact_sensitive_text(
        'Authorization: Bearer satya_secret_123 and api_key="sk-live-abcdef123456"'
    )
    assert "[REDACTED]" in redacted_sample
    assert "sk-live-abcdef123456" not in redacted_sample
    assert "satya_secret_123" not in redacted_sample

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
    with turn_telemetry_scope(
        session_id="ep-graph-test",
        lineage_id="ev-101",
    ):
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
    assert out["tool_invocations"][0]["mcp_tier"] == "TIER_2_RULESET_ENGINE"
    assert len(out["trace_id"]) == 32
    assert len(out["turn_logs"]) >= 1
    assert "[session=ep-graph-test]" in out["turn_logs"][0]["log_line"]
    assert f"[trace={out['trace_id']} " in out["turn_logs"][0]["log_line"]
    assert "[lineage=" in out["turn_logs"][0]["log_line"]

    # Verify uploaded GM_ONLY documents and pinned GM memories are indexed in a compact
    # [GM Vault & Session Index] header (preventing raw 32 KB document pile-up in context),
    # while full document contents are retrieved on-demand via read_gm_documents / search_gm_memory_and_docs.
    seen_human_contents: list[str] = []

    class InspectingChatModel:
        def bind_tools(self, _tools: list[Any]) -> InspectingChatModel:
            return self

        def invoke(self, messages: list[Any]) -> AIMessage:
            last = messages[-1]
            seen_human_contents.append(str(getattr(last, "content", "")))
            return AIMessage(content="I understand the Satya setting and the AI supporters faction.")

    doc_session_id = "ep-auto-gm-doc-test"
    set_session_gm_docs(
        doc_session_id,
        [
            {
                "doc_id": "doc_setting_1",
                "uploader_display_name": "Sean (GM)",
                "title": "Satya Setting Overview",
                "source_kind": "DIRECT_TEXT_UPLOAD",
                "is_persistent": True,
                "sanitized_content": "The faction of AI supporters seeks synthetic sovereignty.",
            }
        ],
    )
    set_session_gm_memories(
        doc_session_id,
        [
            {
                "memory_id": "mem_1",
                "category": "NPC_SECRET",
                "title": "Sovereignty Courier",
                "content": "Dr. Thorne is the secret courier for the AI supporters.",
                "is_pinned": True,
                "is_persistent": True,
            }
        ],
    )
    auto_doc_graph = build_ai_gm_graph(chat_model=InspectingChatModel())
    auto_doc_graph.invoke(
        {
            "session_id": doc_session_id,
            "session_mode": "TTRPG",
            "operating_mode": "autonomous",
            "trigger_event": {
                "event_id": "ev-doc-1",
                "actor": {"user_id": "u-demo-gm-sean", "display_name": "Sean (GM)"},
                "content": "do you understand the setting for satya?",
            },
            "tool_invocations": [],
        },
        config={"configurable": {"thread_id": doc_session_id}},
    )
    assert len(seen_human_contents) == 1
    assert "[GM Vault & Session Index]:" in seen_human_contents[0]
    assert "'Satya Setting Overview' (id=doc_setting_1, persistent, 57 chars)" in seen_human_contents[0]
    assert "Pinned GM Memories: [NPC_SECRET] Sovereignty Courier" in seen_human_contents[0]
    # Raw document body is NOT dumped into HumanMessage history (keeping context small)
    assert "The faction of AI supporters seeks synthetic sovereignty." not in seen_human_contents[0]

    # Test FastAPI service in Co-Pilot mode with BYOK header
    set_chat_model_override(FakeToolCallingChatModel())
    try:
        health_res = health()
        assert health_res["status"] == "ok"

        # Unauthorized request fails with 401
        try:
            handle_ai_gm_event(
                AiGmTurnEventPayload(session_id="ep-1"),
                authorization=None,
                x_satya_provider_key=None,
            )
            raise AssertionError("Expected HTTPException(401)")
        except HTTPException as exc:
            assert exc.status_code == 401

        # Authorized Co-Pilot turn with ephemeral BYOK header + W3C traceparent
        body = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id="ep-copilot-fastapi",
                session_mode="TTRPG_GM_AI",
                operating_mode="CO_PILOT",
                human_gm_user_id="u-gm-sean",
                trigger_event_id="ev-202",
                trigger_actor={"user_id": "u-gm-sean", "display_name": "Sean (GM)"},
                context_text="What are Captain Vance's stats?",
                provider="openai",
                model_id="gpt-4o",
            ),
            authorization="Bearer satya_gm_secret_dev_token",
            x_satya_provider_key="sk-ephemeral-secret-key",
            traceparent="00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            x_satya_lineage_id="ev-202",
        )
        assert body["operating_mode"] == "co_pilot"
        assert body["final_channel"] == "GM_WHISPER"
        assert body["api_key_status"] == "[REDACTED_BYOK_KEY]"
        assert "sk-ephemeral-secret-key" not in json.dumps(body)
        assert body["tool_invocations_count"] == 1
        assert body["trace_id"] == "4bf92f3577b34da6a3ce929d0e0e4736"
        assert body["turn_logs_count"] >= 1
    finally:
        set_chat_model_override(None)

    from satya_ml.ai_gm.tools import generate_npc_from_description, lookup_character_sheet

    terry_res = generate_npc_from_description.invoke(
        {
            "description": "Terry is a kind general MD, but needs to feel like she belongs",
            "template": "AUTO",
            "career_terms": 2,
            "is_persistent": True,
            "session_id": "npc-gen-session",
        }
    )
    assert terry_res["npc"]["character_name"] == "Terry"
    assert terry_res["npc"]["callsign_or_role"] == "General MD & Clinical Physician"
    assert any("BLG 16" in s for s in terry_res["npc"]["psyche_summary"])
    assert any("SLF 5" in s for s in terry_res["npc"]["psyche_summary"])
    assert any("clinical_diagnosis_internal_medicine" in s for s in terry_res["npc"]["top_skills"])
    bound_terry = lookup_character_sheet.invoke(
        {"character_name_or_id": "Terry", "session_id": "npc-gen-session"}
    )
    assert bound_terry["character_name"] == "Terry"

    # Test Scenario Director: 12 TTRPG event sub-routines, flow modes, probability dials (0% disable),
    # automatic turn-interval rabbit-hole interruption, and POST /internal/ai-gm/scenario-beat
    from satya_ml.ai_gm.server import ScenarioBeatRequestPayload, handle_scenario_beat
    from satya_ml.ai_gm.tools import (
        SCENARIO_DIRECTOR_TASKS,
        configure_scenario_director,
        select_next_scenario_beat,
        trigger_scenario_beat,
    )

    assert len(SCENARIO_DIRECTOR_TASKS) == 12
    assert "combat_encounter" in SCENARIO_DIRECTOR_TASKS
    assert "mysterious_stranger" in SCENARIO_DIRECTOR_TASKS
    assert "new_npc_arrives" in SCENARIO_DIRECTOR_TASKS
    assert "dramatic_revelation" in SCENARIO_DIRECTOR_TASKS
    assert "thought_provoking_discussion" in SCENARIO_DIRECTOR_TASKS

    # Free-form mode with 0% weight on thought_provoking_discussion and high combat/drama weights
    cfg_res = configure_scenario_director.invoke(
        {
            "session_id": "dir-freeform-ep1",
            "flow_mode": "free_form",
            "campaign_premise": "Border station blockade",
            "event_weights": {
                slug: (60.0 if slug == "combat_encounter" else (40.0 if slug == "dramatic_revelation" else 0.0))
                for slug in SCENARIO_DIRECTOR_TASKS
            },
            "beat_interval_turns": 2,
        }
    )
    assert cfg_res["flow_mode"] == "free_form"
    assert cfg_res["normalized_probabilities"]["combat_encounter"] == 0.6
    assert cfg_res["normalized_probabilities"]["dramatic_revelation"] == 0.4
    assert cfg_res["normalized_probabilities"]["thought_provoking_discussion"] == 0.0

    for seed in range(10):
        sampled = select_next_scenario_beat("dir-freeform-ep1", rng_seed=seed)
        assert sampled["task_slug"] in {"combat_encounter", "dramatic_revelation"}
        assert sampled["task_slug"] != "thought_provoking_discussion"
        assert "ANTI-RAILROADING RULE" in sampled["mandate_text"]

    # Scenario-driven mode adapts planned_beats without railroading
    configure_scenario_director.invoke(
        {
            "session_id": "dir-plot-ep1",
            "flow_mode": "scenario_driven",
            "campaign_premise": "Sector 4 quarantine",
            "planned_beats": [
                "Encrypted distress signal from Bio-Lab 3 reveals a hidden survivor.",
                "Syndicate enforcer squad breaches the cargo bay.",
            ],
            "beat_interval_turns": 2,
        }
    )
    beat1 = trigger_scenario_beat.invoke({"session_id": "dir-plot-ep1"})
    assert beat1["beat_source"] == "planned_plot_milestone"
    assert "Encrypted distress signal from Bio-Lab 3" in beat1["mandate_text"]
    beat2 = trigger_scenario_beat.invoke({"session_id": "dir-plot-ep1"})
    assert beat2["beat_source"] == "planned_plot_milestone"
    assert beat2["task_slug"] == "combat_encounter"

    # In-graph automatic rabbit-hole breaker on turn interval + out-of-band /scenario-beat endpoint
    configure_scenario_director.invoke(
        {
            "session_id": "dir-graph-ep1",
            "flow_mode": "hybrid",
            "beat_interval_turns": 2,
        }
    )
    set_chat_model_override(InspectingChatModel())
    try:
        # Turn 1: below beat_interval_turns=2 -> no mandate injected yet
        res_t1 = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id="dir-graph-ep1",
                operating_mode="AUTONOMOUS",
                context_text="I keep chatting with the bartender about the weather.",
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_t1["active_director_mandate"] == ""

        # Turn 2: reaches beat_interval_turns=2 -> Scenario Director automatically injects a world beat!
        res_t2 = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id="dir-graph-ep1",
                operating_mode="AUTONOMOUS",
                context_text="I ask the bartender another question about his childhood.",
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert "[SCENARIO DIRECTOR MANDATE" in res_t2["active_director_mandate"]

        # Out-of-band proactive beat via POST /internal/ai-gm/scenario-beat
        res_proactive = handle_scenario_beat(
            ScenarioBeatRequestPayload(
                session_id="dir-graph-ep1",
                task_slug="mysterious_stranger",
                custom_context="A hooded courier slides a data-chip across the table.",
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_proactive["status"] == "ok"
        assert "mysterious_stranger" in res_proactive["active_director_mandate"]
    finally:
        set_chat_model_override(None)
