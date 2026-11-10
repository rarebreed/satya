"""Unit and integration tests for the isolated Satya AI Player Agent (`satya_ml.ai_player`)
and its interaction with the 7-node AI GM (`party_turn_gate_node` and `npc_action_resolution_node`).
"""

from __future__ import annotations

from typing import Any

from langchain_core.messages import AIMessage

from satya_ml.ai_gm.graph import build_ai_gm_graph
from satya_ml.ai_gm.models import SessionLlmConfig
from satya_ml.ai_gm.party_coordinator import extract_active_human_players
from satya_ml.ai_gm.server import set_chat_model_override
from satya_ml.ai_gm.tools import _gm_service_token
from satya_ml.ai_player.graph import build_ai_player_graph
from satya_ml.ai_player.server import (
    AiPlayerTurnEventPayload,
    handle_ai_player_event,
    set_player_chat_model_override,
)
from satya_ml.ai_player.tools import (
    read_my_character_sheet,
    set_player_session_context,
)


class _FakePlayerChatModel:
    """Deterministic fake LLM for the AI Player that calls `roll_player_d20_action` then narrates in first person."""

    def __init__(self) -> None:
        self.calls = 0

    def bind_tools(self, _tools: list[Any]) -> _FakePlayerChatModel:
        return self

    def invoke(self, messages: list[Any]) -> AIMessage:
        self.calls += 1
        # Check if a ToolMessage is already at the end of the turn
        last_msg = messages[-1] if messages else None
        if getattr(last_msg, "type", "") == "tool":
            return AIMessage(
                content=(
                    "**Kaelen Voss** drops to one knee behind the bulkhead and squeezes off a "
                    "`snap_shot` (`T=4/C=4/F=4`) down the corridor!"
                )
            )
        return AIMessage(
            content="",
            tool_calls=[
                {
                    "id": "call-player-roll-1",
                    "name": "roll_player_d20_action",
                    "args": {
                        "check_category": "small_arms",
                        "attribute_tn": 15,
                        "skill_rank": 3,
                        "combat_technique": "snap_shot",
                        "tempo_pool": 4,
                        "control_pool": 4,
                        "force_pool": 4,
                    },
                }
            ],
        )


class _FakeGmChatModel:
    """Deterministic fake LLM for the 7-node AI GM."""

    def bind_tools(self, _tools: list[Any]) -> _FakeGmChatModel:
        return self

    def invoke(self, messages: list[Any]) -> AIMessage:
        combined_text = "\n".join(str(getattr(m, "content", "")) for m in messages)
        assert "[COMBINED PARTY TURN DECLARATIONS" in combined_text
        return AIMessage(
            content=(
                "> **[ACTION MODE · Window 1 · Tick 4 (2.0s)]**\n"
                "Both **Sara** and **Kaelen Voss (AI Player)** coordinate their fire down the corridor!"
            )
        )


def test_player_context_sanitization_strips_gm_only_and_other_private_whispers() -> None:
    raw_ctx = {
        "session_id": "ep-iso-1",
        "gm_documents": [
            {"doc_id": "doc-secret", "title": "GM Vault Secret", "sanitized_content": "OMEGA_BLACK_SITE"}
        ],
        "gm_memories": [
            {"memory_id": "mem-1", "title": "Betrayal Twist", "content": "Vance is a double agent"}
        ],
        "active_map": {
            "map_id": "map-docking-bay",
            "grid_width": 16,
            "grid_height": 12,
            "tokens": [
                {"token_id": "tok-kaelen", "label": "Kaelen Voss", "grid_x": 3, "grid_y": 4},
                {"token_id": "tok-ambush", "label": "Hidden Mine", "grid_x": 8, "grid_y": 6, "gm_only": True},
            ],
        },
        "recent_messages": [
            {
                "message_id": "m-pub",
                "visibility": "PUBLIC",
                "sender": {"user_id": "u-sara"},
                "body": "I check the airlock panel.",
            },
            {
                "message_id": "m-gm-only-other",
                "visibility": "GM_ONLY",
                "sender": {"user_id": "u-sara"},
                "body": "Secret note from Sara to GM.",
            },
            {
                "message_id": "m-priv-kaelen",
                "visibility": "PRIVATE",
                "sender": {"user_id": "ai-gm"},
                "recipients": [{"user_id": "ai-player-kaelen"}],
                "body": "Your scanner picks up a faint thermal trace.",
            },
        ],
    }

    sanitized = set_player_session_context("ep-iso-1", "ai-player-kaelen", raw_ctx)
    assert "gm_documents" not in sanitized
    assert "gm_memories" not in sanitized
    assert len(sanitized["active_map"]["tokens"]) == 1
    assert sanitized["active_map"]["tokens"][0]["token_id"] == "tok-kaelen"
    msg_ids = [m["message_id"] for m in sanitized["recent_messages"]]
    assert msg_ids == ["m-pub", "m-priv-kaelen"]

    sheet_res = read_my_character_sheet.invoke(
        {"session_id": "ep-iso-1", "player_user_id": "ai-player-kaelen"}
    )
    assert sheet_res["character_sheet"]["character_name"] == "Kaelen Voss"
    assert sheet_res["tactical_pool_envelope"]["p_eff"] >= 9


def test_ai_player_graph_executes_combat_triad_roll_and_pairs_with_7_node_ai_gm() -> None:
    session_id = "ep-party-gate-ai-player"
    roster = [
        {
            "user_id": "u-sara",
            "username": "sara_valkyrie",
            "display_name": "Sara (Valkyrie)",
            "participant_kind": "HUMAN",
            "role": "PLAYER",
        },
        {
            "user_id": "ai-player-kaelen",
            "username": "ai-player-kaelen",
            "display_name": "Kaelen Voss (AI Player)",
            "participant_kind": "AI_AGENT",
            "role": "PLAYER",
        },
        {
            "user_id": "ai-gm-ep",
            "username": "satya_ai_gm",
            "display_name": "Satya AI Game Master",
            "participant_kind": "AI_AGENT",
            "role": "AI_GM",
        },
    ]

    # 1. Verify `extract_active_human_players` recognizes both the Human Player and the AI Player (excluding AI_GM)
    party_players = extract_active_human_players(roster)
    assert [p["user_id"] for p in party_players] == ["u-sara", "ai-player-kaelen"]

    # 2. Run the 4-node AI Player graph to generate Kaelen's action and 2d20 T/C/F roll
    fake_player_llm = _FakePlayerChatModel()
    player_graph = build_ai_player_graph(
        SessionLlmConfig(),
        chat_model=fake_player_llm,
    )
    player_out = player_graph.invoke(
        {
            "session_id": session_id,
            "player_user_id": "ai-player-kaelen",
            "player_display_name": "Kaelen Voss (AI Player)",
            "roster": roster,
            "combat_tracker": {
                "session_id": session_id,
                "combat_active": True,
                "round": 1,
                "tick": 1,
                "combatants": [
                    {"entity_id": "u-sara", "display_name": "Sara (Valkyrie)", "next_action_tick": 1},
                    {"entity_id": "ai-player-kaelen", "display_name": "Kaelen Voss (AI Player)", "next_action_tick": 1},
                ],
            },
            "trigger_event": {
                "content": "Window 1 · Tick 1: SecOps sentries breach the bulkhead! Declare your Combat Technique and T/C/F split.",
                "auto_trigger_gm": False,
            },
        },
        config={"configurable": {"thread_id": f"{session_id}:player:ai-player-kaelen"}},
    )
    assert "snap_shot" in player_out["final_reply"]
    assert len(player_out["roll_results"]) == 1
    assert player_out["roll_results"][0]["combat_technique"] == "snap_shot"
    assert player_out["roll_results"][0]["next_action_tick"] is not None

    # 3. Verify the 7-node AI GM `party_turn_gate_node` waits after Sara's turn, then resolves when Kaelen (AI Player) submits!
    gm_graph = build_ai_gm_graph(
        SessionLlmConfig(),
        chat_model=_FakeGmChatModel(),
    )
    turn_1 = gm_graph.invoke(
        {
            "session_id": session_id,
            "operating_mode": "autonomous",
            "roster": roster,
            "trigger_event": {
                "event_id": "ev-sara-1",
                "trigger_kind": "CHAT",
                "actor": roster[0],
                "channel": "PUBLIC",
                "content": "I vault behind the cargo loader and lay down suppressive fire!",
            },
        },
        config={"configurable": {"thread_id": session_id}},
    )
    assert turn_1["party_gate_status"] == "waiting"
    assert turn_1["awaiting_user_ids"] == ["ai-player-kaelen"]

    turn_2 = gm_graph.invoke(
        {
            "session_id": session_id,
            "operating_mode": "autonomous",
            "roster": roster,
            "trigger_event": {
                "event_id": "ev-kaelen-1",
                "trigger_kind": "CHAT",
                "actor": roster[1],
                "channel": "PUBLIC",
                "content": player_out["final_reply"],
            },
        },
        config={"configurable": {"thread_id": session_id}},
    )
    assert turn_2["party_gate_status"] == "ready"
    assert turn_2["awaiting_user_ids"] == []
    assert "Sara" in turn_2["final_reply"] and "Kaelen Voss" in turn_2["final_reply"]


def test_fastapi_ai_player_endpoint_executes_isolated_turn() -> None:
    set_player_chat_model_override(_FakePlayerChatModel())
    try:
        body = handle_ai_player_event(
            AiPlayerTurnEventPayload(
                session_id="ep-http-player-1",
                player_user_id="ai-player-1",
                player_display_name="Kaelen Voss (AI Player)",
                content="Take your combat turn on Tick 1.",
                auto_trigger_gm=False,
            ),
            authorization=f"Bearer {_gm_service_token()}",
        )
        assert body["status"] == "ok"
        assert body["player_user_id"] == "ai-player-1"
        assert "snap_shot" in body["final_reply"]
        assert len(body["roll_results"]) == 1
    finally:
        set_player_chat_model_override(None)
        set_chat_model_override(None)
