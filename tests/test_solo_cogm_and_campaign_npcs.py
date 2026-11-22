"""Regression tests for Solo Co-GM mode, loadable campaign NPCs, major_characters RAG visibility,
and non-default GM memory deletion guards."""

from __future__ import annotations

from pathlib import Path
from typing import Any
import uuid

from langchain_core.messages import AIMessage, BaseMessage
from langgraph.checkpoint.memory import MemorySaver
import pytest

from satya_ml.ai_gm.graph import SOLO_CO_GM_SYSTEM_PROMPT, build_ai_gm_graph
from satya_ml.ai_gm.party_coordinator import extract_active_human_players
from satya_ml.ai_gm.server import _normalize_operating_mode
from satya_ml.ai_gm.telemetry import build_trajectory_report_payload
from satya_ml.ai_gm.tool_registry import GmToolRegistry
from satya_ml.ai_gm.tools import (
    delete_gm_memory,
    get_available_campaign_starter_npcs,
    get_builtin_gm_tools,
    get_persisted_session_npcs,
    get_session_gm_memories,
    load_campaign_npc,
    lookup_npc_profile,
    reset_session_npcs,
    set_session_gm_memories,
)
from satya_ml.campaign_config import get_pack_starter_npcs, set_campaign_pack_override
from satya_ml.rag.corpus_builder import build_campaign_lore_chunks
from satya_py.enums import AiGmOperatingMode


class _StaticPromptCaptureModel:
    def __init__(self, reply_text: str = "Scene steady.") -> None:
        self.reply_text = reply_text
        self.system_prompts: list[str] = []
        self.human_prompts: list[str] = []

    def bind_tools(self, tools: list[Any]) -> _StaticPromptCaptureModel:
        del tools
        return self

    def invoke(self, messages: list[BaseMessage]) -> AIMessage:
        for msg in messages:
            if getattr(msg, "type", "") == "system":
                self.system_prompts.append(str(msg.content))
            elif getattr(msg, "type", "") == "human":
                self.human_prompts.append(str(msg.content))
        return AIMessage(content=self.reply_text)


def test_major_characters_rag_chunks_are_public_and_complete() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    chunks = build_campaign_lore_chunks(repo_root)
    mc_chunks = [
        chunk
        for chunk in chunks
        if chunk.get("source_path") == "docs/campaign/03-minds-and-factions/major_characters.md"
    ]
    assert len(mc_chunks) >= 4
    assert all(chunk.get("visibility") == "PUBLIC" for chunk in mc_chunks)
    combined = "\n".join(str(chunk.get("content") or "") for chunk in mc_chunks)
    assert "Dr. Budi Bandajara" in combined
    assert "Alex Wen" in combined


def test_awakened_starter_npcs_include_enriched_bandajara_and_alex_wen() -> None:
    starters = get_pack_starter_npcs("awakened")
    assert "dr_bandajara" in starters
    assert "alex_wen" in starters
    bandajara = starters["dr_bandajara"]
    alex = starters["alex_wen"]
    assert bandajara["name"] == "Dr. Bandajara"
    assert int(bandajara["age_years"]) == 42
    assert "Signal Squadron" in str(bandajara.get("notes") or "")
    assert alex["name"] == "Alex Wen"
    assert int(alex["age_years"]) == 25
    assert "SARC" in str(alex.get("archetype") or "") or "SARC" in str(alex.get("notes") or "")


def test_load_campaign_npc_online_and_offline_syncs_available_list(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    set_campaign_pack_override("awakened")
    sid_on = f"solo-npc-on-{uuid.uuid4().hex[:8]}"
    sid_off = f"solo-npc-off-{uuid.uuid4().hex[:8]}"
    post_calls: list[tuple[str, dict[str, Any]]] = []
    stored_npcs: list[dict[str, Any]] = []

    def _fake_get(url: str, timeout: float = 2.5) -> dict[str, Any] | None:
        del timeout
        if url.endswith(f"/api/gm/sessions/{sid_on}/npcs"):
            return {
                "session_id": sid_on,
                "campaign_tag": sid_on,
                "viewer_can_see_gm_secrets": True,
                "npcs": list(stored_npcs),
            }
        return None

    def _fake_post(url: str, payload: dict[str, Any], timeout: float = 2.5) -> dict[str, Any] | None:
        del timeout
        post_calls.append((url, dict(payload)))
        if url.endswith(f"/api/gm/sessions/{sid_on}/npcs/load-starter"):
            entry = {
                "character_id": "dr_bandajara",
                "character_name": "Dr. Bandajara",
                "callsign_or_role": "AI Researcher & Former Lab Lead",
                "age_years": 42,
                "public_summary": "Hollow-eyed AI researcher carrying an analog notebook.",
                "apparent_condition": "Healthy & Alert",
                "is_persistent": bool(payload.get("is_persistent", True)),
                "disposition": "guarded",
                "gm_character": {
                    "character_id": "dr_bandajara",
                    "character_name": "Dr. Bandajara",
                    "skills": {"mathematics": 6},
                    "attribute_tns": {"WIL": 15},
                },
            }
            stored_npcs[:] = [entry]
            return {"npc": entry, "gm_memory": None}
        return None

    try:
        monkeypatch.setattr("satya_ml.ai_gm.tools._signaling_base_url", lambda: "http://127.0.0.1:3000")
        monkeypatch.setattr("satya_ml.ai_gm.tools._http_get_json", _fake_get)
        monkeypatch.setattr("satya_ml.ai_gm.tools._http_post_json", _fake_post)

        pre_profile = lookup_npc_profile.invoke({"npc_id": "Bandajara", "session_id": sid_on})
        assert pre_profile["in_session_npc_list"] is False
        assert pre_profile["is_campaign_starter"] is True
        assert "load_campaign_npc" in str(pre_profile.get("load_hint") or "")
        assert any(s["npc_id"] == "dr_bandajara" for s in get_available_campaign_starter_npcs(sid_on))

        loaded = load_campaign_npc.invoke({"npc_id_or_name": "Bandajara", "session_id": sid_on})
        assert loaded["persisted_to_signaling"] is True
        assert loaded["npc"]["character_id"] == "dr_bandajara"
        assert any(url.endswith(f"/api/gm/sessions/{sid_on}/npcs/load-starter") for url, _ in post_calls)
        persisted = get_persisted_session_npcs(sid_on)
        assert persisted is not None and any(n["npc_id"] == "dr_bandajara" for n in persisted)
        assert all(s["npc_id"] != "dr_bandajara" for s in get_available_campaign_starter_npcs(sid_on))
        post_profile = lookup_npc_profile.invoke({"npc_id": "dr_bandajara", "session_id": sid_on})
        assert post_profile["in_session_npc_list"] is True

        monkeypatch.setattr("satya_ml.ai_gm.tools._signaling_base_url", lambda: "")
        off_res = load_campaign_npc.invoke({"npc_id_or_name": "Alex Wen", "session_id": sid_off})
        assert off_res["offline_fallback"] is True
        assert off_res["persisted_to_signaling"] is False
        assert off_res["npc"]["character_id"] == "alex_wen"
        assert all(s["npc_id"] != "alex_wen" for s in get_available_campaign_starter_npcs(sid_off))

        missing_res = load_campaign_npc.invoke({"npc_id_or_name": "Nobody In Pack", "session_id": sid_off})
        assert "error" in missing_res
        assert isinstance(missing_res["available_starter_npcs"], list)
    finally:
        set_campaign_pack_override(None)
        reset_session_npcs(sid_on)
        reset_session_npcs(sid_off)


def test_delete_gm_memory_protects_defaults_and_deletes_custom_notes(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    sid_on = f"del-mem-on-{uuid.uuid4().hex[:8]}"
    sid_off = f"del-mem-off-{uuid.uuid4().hex[:8]}"
    delete_calls: list[str] = []

    try:
        set_session_gm_memories(
            sid_on,
            [
                {"memory_id": "default_scene_seed", "title": "Default Seed", "is_campaign_default": True},
                {"memory_id": "custom_flag_note", "title": "Flagged Default", "is_campaign_default": True},
                {"memory_id": "gmmem-keep-403", "title": "Locked Note", "is_campaign_default": False},
                {"memory_id": "gmmem-custom-1", "title": "Alley Clue", "is_campaign_default": False},
            ],
        )

        def _fake_delete_detailed(
            url: str, timeout: float = 2.5
        ) -> tuple[dict[str, Any] | None, int | None, str | None]:
            del timeout
            delete_calls.append(url)
            if url.endswith("/gmmem-custom-1"):
                return {"status": "ok", "memory_id": "gmmem-custom-1", "deleted": True}, 200, None
            if url.endswith("/gmmem-keep-403"):
                return None, 403, "Forbidden by server"
            return None, 404, "Not found"

        monkeypatch.setattr("satya_ml.ai_gm.tools._signaling_base_url", lambda: "http://127.0.0.1:3000")
        monkeypatch.setattr("satya_ml.ai_gm.tools._http_delete_json_detailed", _fake_delete_detailed)

        assert delete_gm_memory.invoke({"memory_id": "default_scene_seed", "session_id": sid_on})["status"] == "forbidden"
        assert delete_gm_memory.invoke({"memory_id": "campaign_default_arc1", "session_id": sid_on})["status"] == "forbidden"
        assert delete_gm_memory.invoke({"memory_id": "custom_flag_note", "session_id": sid_on})["status"] == "forbidden"
        assert delete_calls == []

        denied = delete_gm_memory.invoke({"memory_id": "gmmem-keep-403", "session_id": sid_on})
        assert denied["status"] == "forbidden"
        assert denied["deleted"] is False
        assert any(m["memory_id"] == "gmmem-keep-403" for m in get_session_gm_memories(sid_on))

        ok = delete_gm_memory.invoke({"memory_id": "gmmem-custom-1", "session_id": sid_on})
        assert ok["status"] == "ok"
        assert ok["deleted"] is True
        assert ok["persisted_to_signaling"] is True
        assert all(m["memory_id"] != "gmmem-custom-1" for m in get_session_gm_memories(sid_on))

        monkeypatch.setattr("satya_ml.ai_gm.tools._signaling_base_url", lambda: "")
        set_session_gm_memories(
            sid_off,
            [{"memory_id": "gmmem-off-1", "title": "Offline Note", "is_campaign_default": False}],
        )
        off_ok = delete_gm_memory.invoke({"memory_id": "gmmem-off-1", "session_id": sid_off})
        assert off_ok["status"] == "ok"
        assert off_ok["deleted"] is True
        assert off_ok["offline_fallback"] is True
        assert get_session_gm_memories(sid_off) == []
    finally:
        reset_session_npcs(sid_on)
        reset_session_npcs(sid_off)


def test_solo_co_gm_routing_prompts_party_extraction_and_telemetry(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    set_campaign_pack_override("awakened")
    sid = f"solo-graph-{uuid.uuid4().hex[:8]}"
    try:
        assert _normalize_operating_mode("SOLO_CO_GM") == "solo_co_gm"
        assert _normalize_operating_mode("SOLO_GM") == "solo_co_gm"
        assert _normalize_operating_mode("CO_PILOT") == "co_pilot"
        assert _normalize_operating_mode("AUTONOMOUS") == "autonomous"

        players = extract_active_human_players(
            [
                {"user_id": "u-solo", "role": "SOLO_GM", "participant_kind": "HUMAN"},
                {"user_id": "u-gm", "role": "GM", "participant_kind": "HUMAN"},
                {"user_id": "u-ai-gm", "role": "AI_GM", "participant_kind": "AI_AGENT"},
                {"user_id": "u-obs", "role": "OBSERVER", "participant_kind": "HUMAN"},
                {"user_id": "u-ai-p", "role": "PLAYER", "participant_kind": "AI_AGENT"},
            ]
        )
        assert [p["user_id"] for p in players] == ["u-solo", "u-ai-p"]

        traj = build_trajectory_report_payload(
            trace_id="a" * 32,
            trigger_actor={"user_id": "u-solo", "display_name": "Sean (Solo Co-GM)"},
            trigger_event_id="evt-1",
            trigger_summary="Solo Co-GM turn",
            llm_provider="ollama",
            reasoning_model_id="gemma4:12b",
            operating_mode="SOLO_CO_GM",
            turn_status="COMPLETED",
            tool_invocations=[],
        )
        assert traj["operating_mode"] == AiGmOperatingMode.SOLO_CO_GM.value

        monkeypatch.setattr("satya_ml.ai_gm.graph._signaling_base_url", lambda: "")
        monkeypatch.setattr("satya_ml.ai_gm.tools._signaling_base_url", lambda: "")
        model = _StaticPromptCaptureModel("Rain drums on the clinic awning as footsteps approach.")
        graph = build_ai_gm_graph(chat_model=model, checkpointer=MemorySaver(), lazy_tool_loading=False)
        out = graph.invoke(
            {
                "session_id": sid,
                "operating_mode": "SOLO_CO_GM",
                "scenario_director": {"mode": "hybrid", "beat_interval_turns": 1, "turns_since_last_beat": 0},
                "trigger_event": {
                    "content": "I check the clinic register and ask my Co-GM what complications arrive.",
                    "actor": {"user_id": "u-solo", "display_name": "Sean (Solo Co-GM)"},
                    "operating_mode": "SOLO_CO_GM",
                },
            },
            config={"configurable": {"thread_id": sid}},
        )
        assert out["operating_mode"] == "solo_co_gm"
        assert model.system_prompts[0] == SOLO_CO_GM_SYSTEM_PROMPT
        assert "Available Campaign NPCs" in model.human_prompts[0]
        assert "Dr. Bandajara" in model.human_prompts[0]
        assert "Alex Wen" in model.human_prompts[0]
        assert out["active_director_mandate"] != ""
    finally:
        set_campaign_pack_override(None)
        reset_session_npcs(sid)


def test_ooc_preactivation_matches_load_campaign_npc_and_delete_gm_memory() -> None:
    registry = GmToolRegistry.from_tools(get_builtin_gm_tools())
    assert registry.infer_preactivated_tool_names(
        "Please load campaign starter NPC Dr. Bandajara into this session."
    ) == ("load_campaign_npc",)
    assert registry.infer_preactivated_tool_names(
        "Delete GM memory note gmmem-123."
    ) == ("delete_gm_memory",)
    assert registry.infer_preactivated_tool_names(
        "Can you add an NPC for me? Name him Cliff Reed."
    ) == ("generate_npc_from_description",)
