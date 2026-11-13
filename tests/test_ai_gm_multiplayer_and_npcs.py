"""Unit and integration tests for multi-player turn readiness barriers (`party_turn_gate_node`)
and autonomous NPC action/tick resolution (`npc_action_resolution_node`) in `satya_ml.ai_gm`.
"""

from __future__ import annotations

from typing import Any


def test_multiplayer_party_turn_barrier_and_ooc_bypass() -> None:
    """Verify that in a multi-player session, the AI GM waits until all active human players
    have declared their actions (or passed/forced) before invoking the LLM or advancing the
    Scenario Director, while allowing OOC rules queries to bypass immediately without losing queued actions.
    """
    try:
        from langchain_core.messages import AIMessage
        from satya_ml.ai_gm.server import (
            AiGmTurnEventPayload,
            handle_ai_gm_event,
            set_chat_model_override,
        )
        from satya_ml.ai_gm.tools import (
            configure_scenario_director,
            get_session_scenario_director,
        )
    except ImportError:
        return

    seen_prompts: list[str] = []

    class PartyRecordingModel:
        def __init__(self) -> None:
            self.calls = 0

        def bind_tools(self, _tools: list[Any]) -> PartyRecordingModel:
            return self

        def invoke(self, messages: list[Any]) -> AIMessage:
            self.calls += 1
            last_txt = str(getattr(messages[-1], "content", ""))
            seen_prompts.append(last_txt)
            if "OUT_OF_CHARACTER RULES / TABLE QUESTION" in last_txt:
                return AIMessage(
                    content="OOC Answer: Allocate your Tactical Pool `P_eff = T + C + F` in the drawer."
                )
            return AIMessage(
                content="**GM Narration**: Kaelen pries open the bulkhead while Lyra covers the corridor; Captain Vance steps back warily."
            )

    model = PartyRecordingModel()
    set_chat_model_override(model)
    session_id = "mp-narrative-barrier-ep1"
    configure_scenario_director.invoke(
        {
            "session_id": session_id,
            "flow_mode": "hybrid",
            "beat_interval_turns": 3,
        }
    )

    multi_roster = [
        {
            "user_id": "u-p1",
            "username": "kaelen",
            "display_name": "Kaelen",
            "participant_kind": "HUMAN",
            "role": "PLAYER",
        },
        {
            "user_id": "u-p2",
            "username": "lyra",
            "display_name": "Lyra",
            "participant_kind": "HUMAN",
            "role": "PLAYER",
        },
        {
            "user_id": "ai-gm-1",
            "username": "satya_ai_gm",
            "display_name": "Satya AI GM",
            "participant_kind": "AI_AGENT",
            "role": "AI_GM",
        },
    ]

    try:
        # 1. Player 1 (Kaelen) declares an in-character action -> Barrier stays open ("waiting"), 0 LLM calls!
        res_p1 = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=session_id,
                operating_mode="AUTONOMOUS",
                channel_context="IN_CHARACTER",
                trigger_actor={"user_id": "u-p1", "display_name": "Kaelen"},
                context_text="I pry open the maintenance bulkhead with my crowbar.",
                roster=multi_roster,
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_p1["party_gate_status"] == "waiting"
        assert res_p1["awaiting_user_ids"] == ["u-p2"]
        assert res_p1["awaiting_display_names"] == ["Lyra"]
        assert "Party Turn Queue — 1/2 Players Ready" in res_p1["final_reply"]
        assert "Lyra" in res_p1["final_reply"]
        assert model.calls == 0
        # Scenario Director turn counter must NOT advance while waiting on other players
        assert get_session_scenario_director(session_id)["turns_since_last_beat"] == 0

        # 2. Player 1 also rolls a 2d20 check while still waiting on Player 2 -> merges into Player 1's queued declaration
        res_p1_roll = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=session_id,
                operating_mode="AUTONOMOUS",
                channel_context="IN_CHARACTER",
                trigger_kind="MECHANICS",
                trigger_actor={"user_id": "u-p1", "display_name": "Kaelen"},
                roll_summary="Athletics N=2, C=13 -> SUCCESS (2 successes, margin +2)",
                context_text="Rolled Athletics to force the hatch.",
                roster=multi_roster,
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_p1_roll["party_gate_status"] == "waiting"
        assert res_p1_roll["awaiting_user_ids"] == ["u-p2"]
        assert model.calls == 0

        # 3. Player 1 asks an OUT_OF_CHARACTER rules question while waiting on Player 2 ->
        #    bypasses the barrier immediately, answers on OUT_OF_CHARACTER, and preserves Kaelen's queued action!
        res_ooc = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=session_id,
                operating_mode="AUTONOMOUS",
                channel_context="OUT_OF_CHARACTER",
                trigger_actor={"user_id": "u-p1", "display_name": "Kaelen"},
                context_text="How do I use my Tempo, Control and Force pool here?",
                roster=multi_roster,
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_ooc["party_gate_status"] == "bypass"
        assert res_ooc["final_channel"] == "OUT_OF_CHARACTER"
        assert res_ooc["awaiting_user_ids"] == ["u-p2"]
        assert model.calls == 1
        assert get_session_scenario_director(session_id)["turns_since_last_beat"] == 0

        # 4. Player 2 (Lyra) declares her action -> Barrier is satisfied ("ready")!
        #    LLM is invoked once with BOTH players' actions + Kaelen's roll + Scene NPC reaction directive.
        res_p2 = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=session_id,
                operating_mode="AUTONOMOUS",
                channel_context="IN_CHARACTER",
                trigger_actor={"user_id": "u-p2", "display_name": "Lyra"},
                context_text="I kneel behind the console and aim my carbine down the corridor.",
                roster=multi_roster,
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_p2["party_gate_status"] == "ready"
        assert res_p2["awaiting_user_ids"] == []
        assert res_p2["final_channel"] == "IN_CHARACTER"
        assert model.calls == 2
        assert get_session_scenario_director(session_id)["turns_since_last_beat"] == 1

        combined_prompt = seen_prompts[-1]
        assert "[COMBINED PARTY TURN DECLARATIONS" in combined_prompt
        assert "I pry open the maintenance bulkhead with my crowbar." in combined_prompt
        assert "Athletics N=2, C=13 -> SUCCESS" in combined_prompt
        assert "I kneel behind the console and aim my carbine down the corridor." in combined_prompt
        assert "[SCENE NPC REACTION & INTENT DIRECTIVE]" in combined_prompt
        assert "Captain Elena Vance" in combined_prompt

        # 5. Next Round: Player 1 acts, Player 2 sends `pass` -> resolves round 2 cleanly
        res_r2_p1 = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=session_id,
                operating_mode="AUTONOMOUS",
                channel_context="IN_CHARACTER",
                trigger_actor={"user_id": "u-p1", "display_name": "Kaelen"},
                context_text="I step through the open bulkhead with my flashlight raised.",
                roster=multi_roster,
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_r2_p1["party_gate_status"] == "waiting"
        assert res_r2_p1["awaiting_user_ids"] == ["u-p2"]

        res_r2_p2 = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=session_id,
                operating_mode="AUTONOMOUS",
                channel_context="IN_CHARACTER",
                trigger_actor={"user_id": "u-p2", "display_name": "Lyra"},
                context_text="pass",
                roster=multi_roster,
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_r2_p2["party_gate_status"] == "ready"
        assert res_r2_p2["awaiting_user_ids"] == []
        assert model.calls == 3
        assert "*(holds / passes action)*" in seen_prompts[-1]
    finally:
        set_chat_model_override(None)


def test_action_mode_tick_barrier_and_autonomous_npc_resolution() -> None:
    """Verify Combat Action Mode (`combat_active = true`) tick-ordered player gating
    and autonomous NPC Count-Up Tick execution (`npc_action_resolution_node`).
    """
    try:
        from langchain_core.messages import AIMessage
        from satya_ml.ai_gm.server import (
            AiGmTurnEventPayload,
            handle_ai_gm_event,
            set_chat_model_override,
        )
        from satya_ml.ai_gm.tools import (
            get_session_combat_tracker,
            set_session_combat_tracker,
        )
    except ImportError:
        return

    seen_combat_prompts: list[str] = []

    class CombatInspectingModel:
        def __init__(self) -> None:
            self.calls = 0

        def bind_tools(self, _tools: list[Any]) -> CombatInspectingModel:
            return self

        def invoke(self, messages: list[Any]) -> AIMessage:
            self.calls += 1
            seen_combat_prompts.append(str(getattr(messages[-1], "content", "")))
            return AIMessage(
                content="> **[ACTION MODE · Window 1 · Tick 4 (2.0s)]**\nKaelen's snap shot grazes the bulkhead as the Station SecOps Sentry returns fire!"
            )

    combat_model = CombatInspectingModel()
    set_chat_model_override(combat_model)
    session_id = "mp-combat-tick-npc-ep1"

    roster = [
        {
            "user_id": "u-p1",
            "username": "kaelen",
            "display_name": "Kaelen",
            "participant_kind": "HUMAN",
            "role": "PLAYER",
        },
        {
            "user_id": "u-p2",
            "username": "lyra",
            "display_name": "Lyra",
            "participant_kind": "HUMAN",
            "role": "PLAYER",
        },
    ]

    # Initialize Action Mode:
    # - Kaelen (u-p1) is at NextTick = 4 (lowest player tick)
    # - Lyra (u-p2) is at NextTick = 9 (later tick)
    # - Station SecOps Sentry (secops_sentry, NPC) is at NextTick = 3 (ready to act!)
    set_session_combat_tracker(
        session_id,
        {
            "session_id": session_id,
            "combat_active": True,
            "round": 1,
            "tick": 3,
            "combatants": [
                {
                    "entity_id": "secops_sentry",
                    "display_name": "Station SecOps Sentry",
                    "role_or_faction": "NPC",
                    "is_npc": True,
                    "ooda_score": 12,
                    "next_action_tick": 3,
                    "active_stance": "NEUTRAL",
                    "fatigue_level": 0,
                    "wound_shock": 0,
                    "grid_x": 12,
                    "grid_y": 6,
                    "zone_label": "Entry Bulkhead",
                },
                {
                    "entity_id": "u-p1",
                    "display_name": "Kaelen",
                    "role_or_faction": "PLAYER",
                    "is_npc": False,
                    "ooda_score": 11,
                    "next_action_tick": 4,
                    "active_stance": "NEUTRAL",
                    "fatigue_level": 0,
                    "wound_shock": 0,
                    "grid_x": 4,
                    "grid_y": 6,
                    "zone_label": "Core Console",
                },
                {
                    "entity_id": "u-p2",
                    "display_name": "Lyra",
                    "role_or_faction": "PLAYER",
                    "is_npc": False,
                    "ooda_score": 10,
                    "next_action_tick": 9,
                    "active_stance": "NEUTRAL",
                    "fatigue_level": 0,
                    "wound_shock": 0,
                    "grid_x": 5,
                    "grid_y": 8,
                    "zone_label": "Core Console",
                },
            ],
        },
    )

    try:
        # 1. If Lyra (NextTick=9) submits her action early while Kaelen (NextTick=4) hasn't acted yet,
        #    her declaration is queued for Tick 9 and the gate waits for Kaelen on Tick 4!
        res_early_lyra = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=session_id,
                operating_mode="AUTONOMOUS",
                channel_context="IN_CHARACTER",
                trigger_actor={"user_id": "u-p2", "display_name": "Lyra"},
                context_text="On Tick 9 I will fire a Deliberate Aimed Shot at the sentry.",
                roster=roster,
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_early_lyra["party_gate_status"] == "waiting"
        assert res_early_lyra["awaiting_user_ids"] == ["u-p1"]
        assert "Action Mode Tick Queue · Tick 4" in res_early_lyra["final_reply"]
        assert "queued for Tick 9" in res_early_lyra["final_reply"]
        assert combat_model.calls == 0

        # 2. Now Kaelen (NextTick=4) declares his Snap Shot -> Tick 4 barrier is satisfied!
        #    `npc_action_resolution_node` automatically executes the ready NPC (`secops_sentry` at Tick 3),
        #    rolls its 2d20 technique + T/C/F check, advances its `next_action_tick`, and passes the
        #    `[NPC ACTION MODE RESOLUTION]` block to `gm_supervisor_node`.
        res_kaelen = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=session_id,
                operating_mode="AUTONOMOUS",
                channel_context="IN_CHARACTER",
                trigger_actor={"user_id": "u-p1", "display_name": "Kaelen"},
                context_text="I fire a Snap Shot (T4/C4/F3) at the Station SecOps Sentry!",
                roster=roster,
            ),
            authorization="Bearer satya_gm_secret_dev_token",
        )
        assert res_kaelen["party_gate_status"] == "ready"
        assert res_kaelen["awaiting_user_ids"] == []
        assert combat_model.calls == 1
        assert len(res_kaelen["npc_turn_summaries"]) == 1

        npc_turn = res_kaelen["npc_turn_summaries"][0]
        assert npc_turn["entity_id"] == "secops_sentry"
        assert npc_turn["acted_on_tick"] == 3
        assert npc_turn["next_action_tick"] > 3
        assert npc_turn["combat_technique"] in {"snap_shot", "combat_sighted_shot"}

        prompt_sent = seen_combat_prompts[-1]
        assert "[COMBINED PARTY TURN DECLARATIONS" in prompt_sent
        assert "**Kaelen** [Tick 4]: I fire a Snap Shot" in prompt_sent
        assert "[NPC ACTION MODE RESOLUTION — Server-Authoritative Count-Up Tick Rolls]" in prompt_sent
        assert "**Station SecOps Sentry** (`secops_sentry`) acted on **Tick 3**" in prompt_sent

        # Verify the combat tracker updated the NPC's next_action_tick
        updated_tracker = get_session_combat_tracker(session_id)
        sentry_entry = next(
            c for c in updated_tracker["combatants"] if c["entity_id"] == "secops_sentry"
        )
        assert sentry_entry["next_action_tick"] == npc_turn["next_action_tick"]
    finally:
        set_chat_model_override(None)


def test_tactical_map_vector_wall_collision_and_los_cover() -> None:
    """Verify continuous vector scene (`tiles == []`), 360-deg OBB prop collision, sub-meter movement, and LoS cover `Δ_ext` in `maps.py`."""
    import sys
    from pathlib import Path

    repo_root = Path(__file__).resolve().parent.parent
    src_dir = str(repo_root / "src")
    if src_dir not in sys.path:
        sys.path.insert(0, src_dir)

    from satya_ml.ai_gm.maps import (
        ensure_or_sync_procedural_tactical_map_stub,
        generate_tactical_map,
        inspect_tactical_map,
        move_map_token,
    )

    sid = "t-map-1m-suite"
    m = generate_tactical_map.invoke(
        {"session_id": sid, "theme": "Sector 4 Docking Bay", "width": 12, "height": 10}
    )
    svg_file = repo_root / "data" / "maps" / f"{sid}_{m['map_id']}.svg"
    assert m["cell_size_cm"] == 100
    assert m["theme_preset"] == "docking_bay"
    assert m["tiles"] == []
    assert len(m["wall_edges"]) > 0
    assert len(m["props"]) > 0
    assert svg_file.exists()

    try:
        # Move to integer cell center -> pos_x_cm = 550, pos_y_cm = 450
        mv = move_map_token.invoke(
            {"session_id": sid, "token_id": "tok-vance", "to_x": 5, "to_y": 4}
        )
        vance_tok = next(t for t in mv["tokens"] if t["token_id"] == "tok-vance")
        assert vance_tok["grid_x"] == 5
        assert vance_tok["grid_y"] == 4
        assert vance_tok["pos_x_cm"] == 550
        assert vance_tok["pos_y_cm"] == 450

        # Sub-meter continuous move
        mv_sub = move_map_token.invoke(
            {"session_id": sid, "token_id": "tok-vance", "to_x_cm": 425, "to_y_cm": 460}
        )
        vance_sub = next(t for t in mv_sub["tokens"] if t["token_id"] == "tok-vance")
        assert vance_sub["pos_x_cm"] == 425
        assert vance_sub["pos_y_cm"] == 460

        # Return to (550, 450) for LoS check across central angled cover
        move_map_token.invoke(
            {"session_id": sid, "token_id": "tok-vance", "to_x_cm": 550, "to_y_cm": 450}
        )

        # Reject solid obstacle overlap (corner pillar at (50cm, 50cm)) and out-of-bounds
        pillar_hit = move_map_token.invoke(
            {"session_id": sid, "token_id": "tok-vance", "to_x": 0, "to_y": 0}
        )
        assert "error" in pillar_hit

        oob = move_map_token.invoke(
            {"session_id": sid, "token_id": "tok-vance", "to_x": 99, "to_y": 4}
        )
        assert "error" in oob

        insp = inspect_tactical_map.invoke(
            {"session_id": sid, "question": "Where is Captain Vance relative to cover?"}
        )
        assert "Capt. Vance" in insp
        assert "(5, 4)" in insp
        assert "[PARTIAL COVER / OBSTRUCTED]" in insp
        assert "Δ_ext +4" in insp

        svg_text = svg_file.read_text(encoding="utf-8")
        assert ">COVER</text>" in svg_text
        assert 'cx="264" cy="216"' in svg_text
        assert "rotate(" in svg_text
    finally:
        svg_file.unlink(missing_ok=True)

    stub = ensure_or_sync_procedural_tactical_map_stub(
        "t-stub-sync-suite",
        theme="Sector 4 Med-Lab",
        combatants=[
            {"entity_id": "u-sara", "display_name": "Sara", "role_or_faction": "PLAYER", "grid_x": 3, "grid_y": 6},
            {"entity_id": "npc-1", "display_name": "Sentry", "role_or_faction": "NPC", "grid_x": 12, "grid_y": 6},
        ],
    )
    stub_svg = repo_root / "data" / "maps" / f"t-stub-sync-suite_{stub['map_id']}.svg"
    stub_svg.unlink(missing_ok=True)
    assert stub["theme_preset"] == "med_lab"
    assert stub["tiles"] == []
    assert len(stub["wall_edges"]) > 0
    assert len(stub["props"]) > 0

