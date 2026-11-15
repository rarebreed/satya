"""Phase 3 & Phase 4 verification: Social Action Mode, World Clocks x Heat x EconomicTier,
Embodied AI Player spatial/curiosity/MotivationLedger isolation, and Awakened multi-turn playtest.
"""

from __future__ import annotations

import json
from typing import Any

from langchain_core.messages import AIMessage

from satya_ml.ai_gm.graph import build_ai_gm_graph
from satya_ml.ai_gm.maps import generate_tactical_map, register_spatial_clue_pin
from satya_ml.ai_gm.models import SessionLlmConfig
from satya_ml.ai_gm.scenario_director import (
    SCENARIO_DIRECTOR_TASKS,
    select_next_scenario_beat,
    set_session_scenario_director,
)
from satya_ml.ai_gm.social_and_clocks import (
    advance_faction_or_heat_clock,
    deliver_social_argument,
    get_world_clocks_and_heat,
    perform_empathy_read,
    resolve_psyche_dissonance_or_surge,
)
from satya_ml.ai_gm.tools import get_session_npcs, reset_session_npcs
from satya_ml.ai_player.graph import build_ai_player_graph
from satya_ml.ai_player.motivation_ledger import (
    inspect_surroundings,
    lookup_public_lore_and_npcs,
    move_player_map_token,
    perform_player_empathy_read,
    update_my_motivation_journal,
)
from satya_ml.ai_player.tools import (
    reset_player_session_context,
    roll_player_d20_action,
    set_player_session_context,
)
from satya_ml.campaign_config import get_active_campaign_pack_name, get_pack_default_ai_player


def test_phase3_social_combat_empathy_read_and_conviction_surge() -> None:
    sid = "p3-social-1"
    reset_session_npcs(sid)
    assert get_active_campaign_pack_name() == "awakened"

    # 1. Empathy Read thresholds (1, 2, 3+ net successes) on Awakened starter NPC
    r1 = perform_empathy_read.invoke({"target_npc_id_or_name": "daise_agent_chen", "net_successes": 1, "session_id": sid})
    assert len(r1["revealed_traits"]) == 1
    assert r1["detects_psyche_wedge_tension"] is False

    r2 = perform_empathy_read.invoke({"target_npc_id_or_name": "daise_agent_chen", "net_successes": 2, "session_id": sid})
    assert len(r2["revealed_traits"]) == 2
    assert r2["detects_psyche_wedge_tension"] is True

    r3 = perform_empathy_read.invoke({"target_npc_id_or_name": "daise_agent_chen", "net_successes": 3, "session_id": sid})
    assert len(r3["revealed_traits"]) == 9
    assert r3["leverage_delta_ext_bonus"] == -2

    # 2. Contested Social Exchange hitting Agent Chen's BLG (13) vs CNF (16) Dual-Psyche Wedge
    ex1 = deliver_social_argument.invoke(
        {
            "target_npc_id_or_name": "daise_agent_chen",
            "argument_vectors": ["group_loyalty", "break_law_orders"],
            "net_successes": 2,
            "session_id": sid,
        }
    )
    assert ex1["argument_eval"]["triggers_dual_psyche_wedge"] is True
    assert ex1["dissonance_stacks_added"] == 2
    assert ex1["updated_state"]["dissonance_index"] >= 2
    assert get_session_npcs(sid)["daise_agent_chen"]["disposition"] == "conflicted"

    # Second & third logical/empathetic Doubt exchanges reach doubt_fractures == 3 -> position_demolished!
    deliver_social_argument.invoke(
        {
            "target_npc_id_or_name": "daise_agent_chen",
            "argument_vectors": ["group_loyalty", "break_law_orders"],
            "net_successes": 2,
            "session_id": sid,
        }
    )
    ex3 = deliver_social_argument.invoke(
        {
            "target_npc_id_or_name": "daise_agent_chen",
            "argument_vectors": ["group_loyalty", "break_law_orders"],
            "net_successes": 3,
            "session_id": sid,
        }
    )
    assert ex3["updated_state"]["doubt_fractures"] >= 3
    assert ex3["updated_state"]["position_demolished"] is True
    assert get_session_npcs(sid)["daise_agent_chen"]["disposition"] == "shaken_yielding"

    # Ego Wound provocation on high-EGO/AGG Harlan Reeves with failed DIS check triggers lashed_out_from_ego_wound
    ex_ego = deliver_social_argument.invoke(
        {
            "target_npc_id_or_name": "humanis_harlan_reeves",
            "argument_vectors": ["humiliate_fault"],
            "net_successes": 2,
            "ego_wound_dis_check_passed": False,
            "session_id": sid,
        }
    )
    assert ex_ego["updated_state"]["lashed_out_from_ego_wound"] is True
    assert get_session_npcs(sid)["humanis_harlan_reeves"]["disposition"] == "hostile"

    # 3. Dissonance resolution, Conviction Surge (Delta_int +4 -> -2), and Psyche Drift Mark
    diss = resolve_psyche_dissonance_or_surge.invoke(
        {
            "mode": "dissonance",
            "target_id": "daise_agent_chen",
            "resolution_path": "compartmentalization",
            "dis": 14,
            "dis_check_successes": 1,
            "session_id": sid,
        }
    )
    assert diss["residual_delta_int_until_rest"] == 2

    surge = resolve_psyche_dissonance_or_surge.invoke(
        {
            "mode": "conviction_surge",
            "target_id": "dr_bandajara",
            "current_delta_int": 4,
            "principle": "iconoclastic_truth",
            "session_id": sid,
        }
    )
    assert surge["eligible"] is True
    assert surge["adjusted_delta_int"] == -2
    assert surge["surge_delta_under"] == 4
    assert surge["tactical_pool_bonus"] == 2

    drift = resolve_psyche_dissonance_or_surge.invoke(
        {
            "mode": "drift_mark",
            "trait_score": 16,
            "marks_added": 3,
            "dis": 12,
            "session_id": sid,
        }
    )
    assert drift["drift_threshold"] == 3
    assert drift["shifted"] is True
    assert drift["new_score"] == 15


def test_phase3_world_clocks_economic_tier_heat_and_director_conditioning() -> None:
    sid = "p3-world-clocks"
    reset_session_npcs(sid)

    initial = get_world_clocks_and_heat.invoke({"session_id": sid})
    assert initial["operational_heat"] == 0
    assert initial["heat_tier_code"] == "HEAT_0_CLEAN"
    assert len(initial["faction_clocks"]) >= 3

    lux = advance_faction_or_heat_clock.invoke(
        {
            "faction_id": "daise",
            "clock_delta": 3,
            "economic_tier": "LUXURY_MANSION",
            "zoning": "URBAN_RESIDENTIAL",
            "incident_severity": "loud_firefight",
            "reason": "Corporate penthouse server extraction",
            "session_id": sid,
        }
    )
    lux_ledger = lux["ledger"]
    assert lux_ledger["operational_heat"] >= 3
    assert lux_ledger["active_economic_tier"] == "LUXURY_MANSION"
    lux_bm = lux_ledger["black_market_delta_ext_mod"]
    lux_profile = lux_ledger["jurisdiction_response_profile"]

    imp = advance_faction_or_heat_clock.invoke(
        {
            "faction_id": "daise",
            "clock_delta": 2,
            "heat_delta": 4,
            "economic_tier": "LOW_INCOME",
            "zoning": "PROJECTS_HIGH_DENSITY",
            "reason": "Relocated to outer clinic tenement",
            "session_id": sid,
        }
    )
    imp_ledger = imp["ledger"]
    assert imp_ledger["operational_heat"] >= 6
    assert imp_ledger["legal_bank_accounts_frozen"] is True
    assert imp_ledger["heat_public_check_delta_ext"] > 0
    assert imp_ledger["black_market_delta_ext_mod"] != lux_bm
    assert imp_ledger["jurisdiction_response_profile"] != lux_profile

    # Disabled weights (0.0) stay strictly unselected even under high Heat & near-full clocks
    weights = {slug: 0.0 for slug in SCENARIO_DIRECTOR_TASKS}
    weights["ticking_clock_escalation"] = 15.0
    weights["moral_or_faction_dilemma"] = 10.0
    set_session_scenario_director(
        sid,
        {"flow_mode": "free_form", "event_weights": weights},
    )

    for seed in range(20):
        beat = select_next_scenario_beat(sid, rng_seed=seed, world_ledger=imp_ledger)
        assert beat["task_slug"] in {"ticking_clock_escalation", "moral_or_faction_dilemma"}
        assert "World Clocks & Heat:" in beat["mandate_text"]


def test_phase4_ai_player_spatial_embodiment_isolation_and_motivation_ledger() -> None:
    sid = "p4-spatial-iso"
    uid = "ai-player-mateo"
    reset_session_npcs(sid)
    reset_player_session_context(sid)

    tactical_map = generate_tactical_map.invoke(
        {
            "session_id": sid,
            "theme": "urban clinic alleyway",
            "width": 16,
            "height": 12,
            "seed": 17,
            "initial_tokens": [
                {
                    "token_id": uid,
                    "entity_id": uid,
                    "label": "Mateo Vance",
                    "x": 2,
                    "y": 2,
                    "faction": "PLAYER",
                },
                {
                    "token_id": "tok-bandajara",
                    "entity_id": "dr_bandajara",
                    "label": "Dr. Bandajara",
                    "x": 5,
                    "y": 4,
                    "faction": "ALLY",
                },
                {
                    "token_id": "tok-gm-ambush",
                    "entity_id": "npc-hidden-sniper",
                    "label": "GM_SECRET_AMBUSH_SNIPER",
                    "x": 9,
                    "y": 8,
                    "faction": "HOSTILE",
                    "gm_only": True,
                },
            ],
        }
    )
    register_spatial_clue_pin.invoke(
        {
            "session_id": sid,
            "pin_id": "pin-secret",
            "channel": "SIG",
            "pos_x_m": 6.0,
            "pos_y_m": 6.0,
            "radius_m": 3.0,
            "whisper_text": "GM_PIN_HIDDEN_BOMB",
        }
    )
    tactical_map["narrative_manifest"] = {
        **(tactical_map.get("narrative_manifest") or {}),
        "spatial_pins": [{"pin_id": "pin-secret", "secret_text": "GM_PIN_HIDDEN_BOMB"}],
    }

    ledger_with_secret = get_world_clocks_and_heat.invoke({"session_id": sid})
    ledger_with_secret["faction_clocks"][0]["gm_secret_agenda"] = "GM_CLOCK_COVERT_RAID"

    raw_ctx = {
        "session_id": sid,
        "character_sheet": get_pack_default_ai_player(),
        "active_map": tactical_map,
        "public_npcs": [
            {
                "character_id": "dr_bandajara",
                "character_name": "Dr. Bandajara",
                "callsign_or_role": "Haunted Whistleblower",
                "faction_id": "Manjusri Contact",
                "disposition": "guarded",
                "public_summary": "Clutching an analog notebook.",
                "gm_secret_dossier": "GM_SECRET_DOSSIER_ALPHA",
                "immediate_agenda": "GM_SECRET_AGENDA_BETA",
                "core_drive": "GM_SECRET_DRIVE",
                "dual_psyche_wedge_hint": "GM_SECRET_WEDGE",
            }
        ],
        "world_ledger": ledger_with_secret,
    }
    sanitized = set_player_session_context(sid, uid, raw_ctx)
    serialized = json.dumps(sanitized)
    for forbidden in (
        "GM_SECRET_AMBUSH_SNIPER",
        "GM_PIN_HIDDEN_BOMB",
        "GM_SECRET_DOSSIER_ALPHA",
        "GM_SECRET_AGENDA_BETA",
        "GM_SECRET_DRIVE",
        "GM_SECRET_WEDGE",
        "GM_CLOCK_COVERT_RAID",
    ):
        assert forbidden not in serialized

    surr = inspect_surroundings.invoke({"session_id": sid, "player_user_id": uid})
    assert surr["map_id"] == tactical_map["map_id"]
    assert all(tok["label"] != "GM_SECRET_AMBUSH_SNIPER" for tok in surr["visible_tokens"])

    moved = move_player_map_token.invoke(
        {"target_x": 4.5, "target_y": 3.5, "session_id": sid, "player_user_id": uid}
    )
    assert moved["status"] in {"moved", "moved_offline"}
    assert all(t.get("label") != "GM_SECRET_AMBUSH_SNIPER" for t in moved["active_map"]["tokens"])

    pub_hits = lookup_public_lore_and_npcs.invoke(
        {"query": "Bandajara", "scope": "ALL", "session_id": sid, "player_user_id": uid}
    )
    assert pub_hits["hits"]
    assert "GM_SECRET_DOSSIER_ALPHA" not in json.dumps(pub_hits)

    fail_read = perform_player_empathy_read.invoke(
        {"target_npc_name": "Dr. Bandajara", "net_successes": 0, "session_id": sid, "player_user_id": uid}
    )
    assert fail_read["revealed_traits"] == []

    good_read = perform_player_empathy_read.invoke(
        {"target_npc_name": "Dr. Bandajara", "net_successes": 2, "session_id": sid, "player_user_id": uid}
    )
    assert len(good_read["revealed_traits"]) == 2
    assert good_read["detects_psyche_wedge_tension"] is True
    assert "GM_SECRET_DOSSIER_ALPHA" not in json.dumps(good_read)

    journal = update_my_motivation_journal.invoke(
        {
            "immediate_goal": "Get Dr. Bandajara out of camera sight before DAISE arrives.",
            "add_clue": "Bandajara hides an air-gapped medical drive in his coat.",
            "party_bond_note": "Protect Sister Lin's clinic regulars.",
            "psyche_reflection": "My low CNF=6 bristles at warrantless surveillance.",
            "session_id": sid,
            "player_user_id": uid,
        }
    )
    assert "Bandajara" in journal["immediate_goal"]
    assert len(journal["unresolved_clues"]) == 1
    assert "Dr. Bandajara" in journal["known_npc_reads"]

    # Authoritative satya_py 2d20 roll returns both total_successes & successes_counted cleanly
    roll_res = roll_player_d20_action.invoke(
        {
            "check_category": "small_arms",
            "attribute_tn": 14,
            "skill_rank": 2,
            "combat_technique": "snap_shot",
            "tempo_pool": 4,
            "control_pool": 4,
            "force_pool": 4,
            "session_id": sid,
            "player_user_id": uid,
        }
    )
    assert roll_res["total_successes"] == roll_res["successes_counted"]
    assert roll_res["outcome_tier"]


class _PlaytestPlayerLlm:
    """Multi-turn scripted AI Player exercising spatial, curiosity, empathy, and roll tools."""

    def __init__(self) -> None:
        self.turn_stage = 0
        self.observed_prompts: list[str] = []

    def bind_tools(self, _tools: list[Any]) -> _PlaytestPlayerLlm:
        return self

    def invoke(self, messages: list[Any]) -> AIMessage:
        prompt_blob = "\n".join(str(getattr(m, "content", "")) for m in messages)
        self.observed_prompts.append(prompt_blob)
        last = messages[-1] if messages else None
        if getattr(last, "type", "") == "tool":
            return AIMessage(
                content=(
                    "**Mateo Vance**: I edge along the wet brick wall toward the loading bay (`4.5m, 3.5m`), "
                    "keep my eyes on Dr. Bandajara's tremor, and tap the diagnostic console quietly. "
                    "*I didn't survive the port layoffs by trusting federal compliance tablets — talk to me straight, Doc.*"
                )
            )
        self.turn_stage += 1
        if self.turn_stage == 1:
            return AIMessage(
                content="",
                tool_calls=[
                    {"id": "p-surr", "name": "inspect_surroundings", "args": {}},
                    {"id": "p-move", "name": "move_player_map_token", "args": {"target_x": 4.5, "target_y": 3.5}},
                    {"id": "p-emp", "name": "perform_player_empathy_read", "args": {"target_npc_name": "Dr. Bandajara", "net_successes": 2}},
                    {
                        "id": "p-jrn",
                        "name": "update_my_motivation_journal",
                        "args": {
                            "add_clue": "DAISE cruisers are sweeping the outer block.",
                            "immediate_goal": "Move Bandajara to the interior storage door.",
                        },
                    },
                ],
            )
        return AIMessage(
            content="",
            tool_calls=[
                {
                    "id": "p-roll",
                    "name": "roll_player_d20_action",
                    "args": {
                        "check_category": "electronics_repair",
                        "attribute_tn": 13,
                        "skill_rank": 3,
                        "note": "Spoofing alley camera loop",
                    },
                }
            ],
        )


class _PlaytestGmLlm:
    """Scripted AI GM using Social Action Mode and World Clocks / Heat tools."""

    def __init__(self) -> None:
        self.observed_prompts: list[str] = []

    def bind_tools(self, _tools: list[Any]) -> _PlaytestGmLlm:
        return self

    def invoke(self, messages: list[Any]) -> AIMessage:
        self.observed_prompts.append("\n".join(str(getattr(m, "content", "")) for m in messages))
        last = messages[-1] if messages else None
        if getattr(last, "type", "") == "tool":
            return AIMessage(
                content=(
                    "**Dr. Bandajara** lowers his coat collar as **Mateo** slides beside the crate.\n\n"
                    "> **[WORLD ALERT · DAISE Enforcement Clock 2/6 · Heat 2/10]**\n"
                    "> Distant patrol strobe sweeps the tenement courtyard.\n\n"
                    "- Slip through the east service door toward Sister Lin's triage hallway.\n"
                    "- Press **Agent Chen** at the gate before he scans the medical drive."
                )
            )
        return AIMessage(
            content="",
            tool_calls=[
                {
                    "id": "gm-soc",
                    "name": "deliver_social_argument",
                    "args": {
                        "target_npc_id_or_name": "daise_agent_chen",
                        "argument_vectors": ["group_loyalty", "break_law_orders"],
                        "net_successes": 2,
                    },
                },
                {
                    "id": "gm-clk",
                    "name": "advance_faction_or_heat_clock",
                    "args": {
                        "faction_id": "daise",
                        "clock_delta": 2,
                        "heat_delta": 2,
                        "public_rumor": "Unmarked DAISE cruisers were spotted two blocks from St. Jude Clinic.",
                        "gm_secret_agenda": "GM_PLAYTEST_COVERT_SWEEP",
                        "reason": "Clinic alley camera loop glitch",
                    },
                },
            ],
        )


def test_phase4_awakened_multi_turn_ai_gm_and_ai_player_experience() -> None:
    sid = "p4-awakened-playtest"
    uid = "ai-player-mateo"
    reset_session_npcs(sid)
    reset_player_session_context(sid)
    assert get_active_campaign_pack_name() == "awakened"

    tactical_map = generate_tactical_map.invoke(
        {
            "session_id": sid,
            "theme": "urban clinic courtyard",
            "width": 16,
            "height": 12,
            "seed": 33,
            "initial_tokens": [
                {"token_id": uid, "entity_id": uid, "label": "Mateo Vance", "x": 2, "y": 2, "faction": "PLAYER"},
                {"token_id": "tok-bandajara", "entity_id": "dr_bandajara", "label": "Dr. Bandajara", "x": 5, "y": 4, "faction": "ALLY"},
                {"token_id": "tok-hidden", "entity_id": "npc-drone", "label": "GM_AMBUSH_DRONE", "x": 11, "y": 9, "faction": "HOSTILE", "gm_only": True},
            ],
        }
    )
    tactical_map["narrative_manifest"] = {
        **(tactical_map.get("narrative_manifest") or {}),
        "spatial_pins": [{"pin_id": "pin-basement", "secret_text": "SECRET_SERVER_CAGE_BELOW"}],
    }
    roster = [
        {"user_id": uid, "username": "mateo_vance", "display_name": "Mateo Vance (AI Player)", "participant_kind": "AI_AGENT", "role": "PLAYER"},
        {"user_id": "ai-gm-1", "username": "satya_ai_gm", "display_name": "Satya AI Game Master", "participant_kind": "AI_AGENT", "role": "AI_GM"},
    ]

    player_llm = _PlaytestPlayerLlm()
    gm_llm = _PlaytestGmLlm()
    player_graph = build_ai_player_graph(SessionLlmConfig(), chat_model=player_llm)
    gm_graph = build_ai_gm_graph(SessionLlmConfig(), chat_model=gm_llm)

    # Turn 1: Embodied AI Player explores, moves, reads Bandajara, and logs MotivationLedger
    p_turn1 = player_graph.invoke(
        {
            "session_id": sid,
            "player_user_id": uid,
            "player_display_name": "Mateo Vance (AI Player)",
            "roster": roster,
            "active_map": tactical_map,
            "world_ledger": get_world_clocks_and_heat.invoke({"session_id": sid}),
            "trigger_event": {
                "content": "Rain slicks the alley outside St. Jude Clinic. Dr. Bandajara waves from beneath the awning while sirens murmur a block away.",
                "auto_trigger_gm": False,
            },
        },
        config={"configurable": {"thread_id": f"{sid}:player:{uid}"}},
    )
    assert "Mateo Vance" in p_turn1["final_reply"]
    assert "Dr. Bandajara" in p_turn1["motivation_ledger"]["known_npc_reads"]
    assert p_turn1["motivation_ledger"]["unresolved_clues"] == ["DAISE cruisers are sweeping the outer block."]

    # AI GM resolves Turn 1 player declaration, drives Social Action Mode + DAISE Clock & Heat
    gm_turn1 = gm_graph.invoke(
        {
            "session_id": sid,
            "operating_mode": "autonomous",
            "roster": roster,
            "active_map": tactical_map,
            "trigger_event": {
                "event_id": "ev-mateo-1",
                "trigger_kind": "CHAT",
                "actor": roster[0],
                "channel": "PUBLIC",
                "content": p_turn1["final_reply"],
            },
        },
        config={"configurable": {"thread_id": sid}},
    )
    assert gm_turn1["world_ledger"]["operational_heat"] >= 2
    assert any(clk["faction_id"] == "daise" and clk["progress"] >= 2 for clk in gm_turn1["world_ledger"]["faction_clocks"])
    assert "Active World Clocks & Jurisdiction Heat" in gm_llm.observed_prompts[-1]

    # Turn 2: Player receives updated world ledger & map state; journal persists; GM secrets remain isolated
    p_turn2 = player_graph.invoke(
        {
            "session_id": sid,
            "player_user_id": uid,
            "player_display_name": "Mateo Vance (AI Player)",
            "roster": roster,
            "active_map": tactical_map,
            "world_ledger": gm_turn1["world_ledger"],
            "trigger_event": {
                "content": gm_turn1["final_reply"],
                "auto_trigger_gm": False,
            },
        },
        config={"configurable": {"thread_id": f"{sid}:player:{uid}"}},
    )
    assert len(p_turn2["roll_results"]) == 1
    assert p_turn2["motivation_ledger"]["unresolved_clues"] == ["DAISE cruisers are sweeping the outer block."]
    all_player_prompts = "\n".join(player_llm.observed_prompts)
    assert "My Inner Motivation Ledger:" in all_player_prompts
    assert "Public Heat" in all_player_prompts
    assert "GM_AMBUSH_DRONE" not in all_player_prompts
    assert "SECRET_SERVER_CAGE_BELOW" not in all_player_prompts
    assert "GM_PLAYTEST_COVERT_SWEEP" not in all_player_prompts
