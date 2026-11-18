"""Tests for Phase 1 & Phase 2: Awakened default campaign pack config, Rust procedural maps,
passive spatial perception whispers, HookWeaver motivational compilation, and ScenarioDirector conditioning.
"""

from __future__ import annotations

import sys
import uuid
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
SRC_DIR = str(REPO_ROOT / "src")
if SRC_DIR not in sys.path:
    sys.path.insert(0, SRC_DIR)

from satya_ml.ai_gm.hooks import (
    compile_player_motivational_hooks,
    format_party_hooks_for_prompt,
    normalize_character_sheet_record,
)
from satya_ml.ai_gm.maps import (
    clear_session_spatial_pins,
    continue_tactical_map,
    generate_tactical_map,
    infer_sector_preset,
    move_map_token,
    register_spatial_clue_pin,
    switch_tactical_map_floor,
)
from satya_ml.ai_gm.scenario_director import (
    SCENARIO_DIRECTOR_TASKS,
    configure_scenario_director,
    get_session_scenario_director,
    select_next_scenario_beat,
)
from satya_ml.ai_gm.tools import (
    _gm_service_token,
    generate_npc_from_description,
    load_all_npc_profiles,
    lookup_npc_profile,
    query_campaign_lore,
    reset_session_npcs,
    set_session_characters,
)
from satya_ml.ai_player.tools import read_my_character_sheet, reset_player_session_context
from satya_ml.campaign_config import (
    get_active_campaign_pack_name,
    get_pack_combat_spawn,
    get_pack_default_ai_player,
    get_pack_starter_scenario,
    set_campaign_pack_override,
)


def _cleanup_session_svgs(session_id: str) -> None:
    maps_dir = REPO_ROOT / "data" / "maps"
    if maps_dir.exists():
        for svg in maps_dir.glob(f"{session_id}_*.svg"):
            svg.unlink(missing_ok=True)


def test_awakened_campaign_pack_is_default_and_switchable() -> None:
    set_campaign_pack_override(None)
    reset_session_npcs()
    reset_player_session_context()
    sid = f"awakened-pack-{uuid.uuid4().hex[:6]}"
    try:
        assert get_active_campaign_pack_name() == "awakened"

        profiles = load_all_npc_profiles(sid)
        assert "dr_bandajara" in profiles
        assert "daise_agent_chen" in profiles
        assert "captain_vance" not in profiles

        bandajara = lookup_npc_profile.invoke({"npc_id": "dr_bandajara", "session_id": sid})
        assert bandajara["name"] == "Dr. Bandajara"
        assert "seed weight" in str(bandajara.get("immediate_agenda") or "").lower()

        lore_satyang = query_campaign_lore.invoke({"query": "Satyang", "session_id": sid})
        assert "Satyang" in lore_satyang
        assert "[Campaign Lore" in lore_satyang or "Manjusri" in lore_satyang

        lore_bandajara = query_campaign_lore.invoke({"query": "Bandajara seed weight", "session_id": sid})
        assert "Dr. Bandajara" in lore_bandajara
        assert "Agenda:" in lore_bandajara

        default_player = get_pack_default_ai_player()
        assert default_player["character_name"] == "Mateo Vance"
        sheet_res = read_my_character_sheet.invoke({"session_id": sid, "player_user_id": "ai-mateo"})
        assert sheet_res["character_sheet"]["character_name"] == "Mateo Vance"

        spawn = get_pack_combat_spawn()
        assert spawn["entity_id"] == "daise_breach_lead"
        assert infer_sector_preset("Rain-slick Backstreet Intersection") == "urban_crossroads"

        # Switching to optional Space Opera pack switches defaults cleanly
        set_campaign_pack_override("space_opera")
        reset_session_npcs()
        reset_player_session_context()
        assert get_active_campaign_pack_name() == "space_opera"
        assert "captain_vance" in load_all_npc_profiles(sid)
        assert get_pack_default_ai_player()["character_name"] == "Kaelen Voss"
        assert get_pack_combat_spawn()["entity_id"] == "secops_breach_lead"
        assert infer_sector_preset("Uncharted Deck Junction") == "orbital_airlock"
    finally:
        set_campaign_pack_override(None)
        reset_session_npcs()
        reset_player_session_context()


def test_real_rust_procedural_sector_floors_continuation_and_manifest() -> None:
    set_campaign_pack_override(None)
    sid = f"rust-sector-{uuid.uuid4().hex[:6]}"
    try:
        m = generate_tactical_map.invoke(
            {
                "session_id": sid,
                "theme": "Satyang District Hospital",
                "width": 24,
                "height": 20,
                "seed": 77,
                "economic_tier": "WorkingClass",
                "zoning": "InstitutionalMedical",
            }
        )
        assert m["grid_width"] == 24
        assert m["grid_height"] == 20
        assert m["theme_preset"] == "hospital"
        assert len(m["wall_edges"]) > 10
        assert len(m["props"]) > 5
        assert len(m["surfaces"]) > 0
        assert len(m["floors"]) >= 2

        manifest = m.get("narrative_manifest") or {}
        assert manifest.get("map_id") == m["map_id"]
        assert len(manifest.get("floors") or []) >= 2
        assert len(manifest.get("walkable_anchors") or []) >= 1
        assert len(manifest.get("cardinal_exits") or []) == 4
        assert manifest.get("concise_brief")

        current_floor = int(m["active_floor"])
        other_floor = next(
            int(f["floor_index"])
            for f in m["floors"]
            if int(f["floor_index"]) != current_floor
        )
        switched = switch_tactical_map_floor.invoke(
            {"session_id": sid, "target_floor_index": other_floor}
        )
        assert switched["event_type"] == "FLOOR_SWITCHED"
        assert int(switched["active_floor"]) == other_floor

        continued = continue_tactical_map.invoke(
            {"session_id": sid, "direction": "EAST", "seed": 88}
        )
        assert continued["event_type"] == "MAP_CONTINUED"
        assert len(continued.get("world_sectors") or []) >= 2
        assert (continued.get("narrative_manifest") or {}).get("concise_brief")
    finally:
        clear_session_spatial_pins(sid)
        _cleanup_session_svgs(sid)


def test_passive_spatial_clue_pins_whisper_and_one_shot() -> None:
    set_campaign_pack_override(None)
    sid = f"clue-pins-{uuid.uuid4().hex[:6]}"
    try:
        m = generate_tactical_map.invoke(
            {
                "session_id": sid,
                "theme": "Metro Alleyway Crossroads",
                "width": 24,
                "height": 20,
                "seed": 99,
            }
        )
        manifest = m.get("narrative_manifest") or {}
        anchors = list(manifest.get("walkable_anchors") or [])
        assert len(anchors) >= 1
        ax_m = float(anchors[0]["x_m"])
        ay_m = float(anchors[0]["y_m"])
        ax_cm = int(anchors[0]["x_cm"])
        ay_cm = int(anchors[0]["y_cm"])

        tok_id = str(m["tokens"][0]["token_id"])
        tok_entity = str(m["tokens"][0].get("entity_id") or tok_id)
        set_session_characters(
            sid,
            [
                {
                    "character_id": tok_entity,
                    "character_name": "Elena Diagnostic",
                    "characteristics": {"sig": 17, "hea": 15, "olf": 14, "itu": 16},
                    "skills": {"forensics": 3},
                }
            ],
        )

        pin_res = register_spatial_clue_pin.invoke(
            {
                "session_id": sid,
                "pin_id": "pin-airgap-shard",
                "channel": "SIG",
                "pos_x_m": ax_m,
                "pos_y_m": ay_m,
                "radius_m": 5.0,
                "whisper_text": "Hairline solder scratches hide a sub-dermal Manjusri micro-port behind the fuse plate.",
                "base_difficulty_mod": -4,
                "floor_index": int(m.get("active_floor", 0)),
                "requires_los": False,
                "one_shot": True,
            }
        )
        assert pin_res["status"] == "ok"

        moved_1 = move_map_token.invoke(
            {
                "session_id": sid,
                "token_id": tok_id,
                "to_x_cm": ax_cm,
                "to_y_cm": ay_cm,
                "sensory_tns": {"SIG": 17, "HEA": 15, "OLF": 14, "ITU": 16},
            }
        )
        whispers_1 = moved_1.get("passive_whispers") or []
        assert any(
            w.get("pin_id") == "pin-airgap-shard" and "Manjusri micro-port" in str(w.get("whisper_text"))
            for w in whispers_1
        )

        # Second move at same range should suppress the already-triggered one_shot pin
        moved_2 = move_map_token.invoke(
            {
                "session_id": sid,
                "token_id": tok_id,
                "to_x_cm": ax_cm,
                "to_y_cm": ay_cm,
                "sensory_tns": {"SIG": 17, "HEA": 15, "OLF": 14, "ITU": 16},
            }
        )
        whispers_2 = moved_2.get("passive_whispers") or []
        assert not any(w.get("pin_id") == "pin-airgap-shard" for w in whispers_2)
    finally:
        set_session_characters(sid, [])
        clear_session_spatial_pins(sid)
        _cleanup_session_svgs(sid)


def test_hook_weaver_and_scenario_director_gating_and_conditioning() -> None:
    set_campaign_pack_override(None)
    sid = f"hooks-dir-{uuid.uuid4().hex[:6]}"

    party = [
        {
            "character_id": "pc-mateo",
            "character_name": "Mateo Vance",
            "callsign_or_role": "Displaced Port Fixer",
            "operational_heat": 3,
            "granted_access_tags": ["DAISE_AUDIT_KEY", "CLINIC_BASEMENT"],
            "characteristics": {
                "opn": 18,
                "slf": 4,
                "cnf": 4,
                "agg": 7,
                "ego": 9,
                "blg": 15,
                "ext": 13,
                "ana": 15,
                "ins": 14,
            },
            "skills": {"digital_ai_literacy": 4, "electronics_repair": 3},
            "notes": "Owes clinic debt after refusing DAISE biometric registry.",
        },
        {
            "character_id": "pc-kors",
            "character_name": "Inspector Linnea Kors",
            "callsign_or_role": "Auditor Liaison",
            "characteristics": {
                "opn": 6,
                "slf": 16,
                "cnf": 17,
                "agg": 14,
                "ego": 15,
                "blg": 11,
                "ext": 8,
            },
            "skills": {"interrogation": 3, "small_arms": 3},
        },
    ]

    manifest = compile_player_motivational_hooks(party, active_location="Satyang Clinic Triage")
    assert manifest["pack_id"] == "awakened"
    assert len(manifest.get("hooks") or []) >= 2
    assert len(manifest.get("party_entanglements") or []) >= 1

    digest = format_party_hooks_for_prompt(manifest, max_hooks=4)
    assert "[Party Motivational Hooks & Entanglements]" in digest
    assert len(digest) <= 900

    # Zero-disabled director weight stays excluded even with active hooks
    configure_scenario_director.invoke(
        {
            "session_id": f"{sid}-weights",
            "flow_mode": "free_form",
            "event_weights": {
                slug: (5.0 if slug in {"combat_encounter", "discovery_or_clue"} else 0.0)
                for slug in SCENARIO_DIRECTOR_TASKS
            },
        }
    )
    sector_manifest = {
        "zones": [
            {"label": "Clinical / Triage Bay"},
            {"label": "Curbside / Vehicle Perimeter"},
        ],
        "cardinal_exits": [{"direction": "EAST", "preview_label": "Rail Overpass"}],
    }
    for seed in range(8):
        beat = select_next_scenario_beat(
            f"{sid}-weights",
            rng_seed=seed,
            party_hooks=manifest,
            narrative_manifest=sector_manifest,
        )
        assert beat["task_slug"] in {"combat_encounter", "discovery_or_clue"}
        assert "Active PC Motivational Hook to Pull:" in beat["mandate_text"]
        assert "Tactical Sector Affordances:" in beat["mandate_text"]
        assert "Clinical / Triage Bay" in beat["mandate_text"]

    # End-to-end graph gating with beat_probability_per_turn = 0.0 vs 1.0
    from langchain_core.messages import AIMessage
    from satya_ml.ai_gm.server import (
        AiGmTurnEventPayload,
        handle_ai_gm_event,
        set_chat_model_override,
    )

    class EchoModel:
        def bind_tools(self, _tools: list[Any]) -> EchoModel:
            return self

        def invoke(self, _messages: list[Any]) -> AIMessage:
            return AIMessage(content="The clinic rain hammers against the shuttered glass.")

    set_session_characters(sid, party)
    set_chat_model_override(EchoModel())
    try:
        configure_scenario_director.invoke(
            {
                "session_id": sid,
                "flow_mode": "free_form",
                "beat_probability_per_turn": 0.0,
                "beat_interval_turns": 1,
            }
        )
        res_off = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=sid,
                operating_mode="AUTONOMOUS",
                channel_context="IN_CHARACTER",
                context_text="I check the backroom generator.",
            ),
            authorization=f"Bearer {_gm_service_token()}",
        )
        assert res_off["active_director_mandate"] == ""
        assert get_session_scenario_director(sid)["turns_since_last_beat"] == 1

        configure_scenario_director.invoke(
            {
                "session_id": f"{sid}-prob1",
                "flow_mode": "free_form",
                "beat_probability_per_turn": 1.0,
                "beat_interval_turns": 3,
                "event_weights": {
                    slug: (10.0 if slug == "discovery_or_clue" else 0.0)
                    for slug in SCENARIO_DIRECTOR_TASKS
                },
            }
        )
        set_session_characters(f"{sid}-prob1", party)
        res_on = handle_ai_gm_event(
            AiGmTurnEventPayload(
                session_id=f"{sid}-prob1",
                operating_mode="AUTONOMOUS",
                channel_context="IN_CHARACTER",
                context_text="I pry open the fuse panel in triage.",
            ),
            authorization=f"Bearer {_gm_service_token()}",
        )
        assert "[SCENARIO DIRECTOR MANDATE" in res_on["active_director_mandate"]
        assert "Active PC Motivational Hook to Pull:" in res_on["active_director_mandate"]
    finally:
        set_chat_model_override(None)
        set_session_characters(sid, [])
        set_session_characters(f"{sid}-prob1", [])
        clear_session_spatial_pins(sid)
        _cleanup_session_svgs(sid)
        _cleanup_session_svgs(f"{sid}-prob1")


def test_phase2_python_ai_gm_9_psyche_factions_and_starter_scenario() -> None:
    set_campaign_pack_override(None)
    sid = f"phase2-py-{uuid.uuid4().hex[:6]}"
    try:
        # 1. Normalization preserves 9-trait Psyche (`trs`, `cps`) + optional faction tags
        norm = normalize_character_sheet_record(
            {
                "character_id": "pc-linnea",
                "character_name": "Linnea Cho",
                "faction_id": "SATYANG",
                "faction_archetype": "SATYANG_MANJUSRI",
                "characteristics": {
                    "opn": 16,
                    "slf": 5,
                    "cnf": 6,
                    "trs": 4,
                    "cps": 17,
                    "ana": 14,
                    "ins": 15,
                },
                "skills": {"philosophy_formal_logic_ethics": 3, "empathy_social_perception": 3},
            }
        )
        assert norm["characteristics"]["trs"] == 4
        assert norm["characteristics"]["cps"] == 17
        assert norm["faction_id"] == "SATYANG"
        assert norm["faction_archetype"] == "SATYANG_MANJUSRI"

        manifest = compile_player_motivational_hooks([norm], active_location="St. Jude Annex")
        assert manifest["pack_id"] == "awakened"
        assert manifest["hooks"]
        linnea_prof = manifest["character_profiles"].get("pc-linnea") or {}
        if linnea_prof:
            dom_poles = linnea_prof.get("dominant_poles") or []
            assert any("CPS=17" in p for p in dom_poles)
            assert any("TRS=4" in p for p in dom_poles)

        # 2. generate_npc_from_description advertises 9 Psyche traits & 14 Faction Archetypes
        tool_desc = str(getattr(generate_npc_from_description, "description", ""))
        assert "9 bipolar Psyche traits" in tool_desc
        assert "SATYANG_MANJUSRI" in tool_desc

        npc_res = generate_npc_from_description.invoke(
            {
                "description": "Archivist auditing black-site compute telemetry",
                "template": "SATYANG_MANJUSRI",
                "name_override": "Linnea Archive",
                "career_terms": 2,
                "is_persistent": False,
                "session_id": sid,
            }
        )
        npc = npc_res["npc"]
        assert npc["faction_id"] == "SATYANG"
        assert npc["faction_archetype"] == "SATYANG_MANJUSRI"
        assert len(npc["psyche_summary"]) == 9
        assert any(s.startswith("TRS ") for s in npc["psyche_summary"])
        assert any(s.startswith("CPS ") for s in npc["psyche_summary"])

        # 3. Starter Scenario + Preface/Breakdown lore discovery
        starter = get_pack_starter_scenario("awakened")
        assert "Cold Storage at St. Jude Annex" in starter["title"]
        assert len(starter["planned_beats"]) == 5

        cfg = configure_scenario_director.invoke(
            {
                "session_id": f"{sid}-ep1",
                "flow_mode": "scenario_driven",
                "seed_pack_starter": True,
            }
        )
        assert "St. Jude Mutual-Aid Clinic" in cfg["campaign_premise"]
        assert len(cfg["planned_beats"]) == 5

        preface_hit = query_campaign_lore.invoke(
            {"query": "Preface Truth Mathematics Compassion", "session_id": sid}
        )
        assert "preface.md" in preface_hit

        arc3_hit = query_campaign_lore.invoke(
            {"query": "Arc 3 Awakening Moral Horizon Cascade Fracture", "session_id": sid}
        )
        assert "three_arc_chronicle.md" in arc3_hit
    finally:
        set_session_characters(sid, [])
        reset_session_npcs(sid)
