"""End-to-end deterministic tests for the 6 post-merge 9-axis Psyche & Lifepath AI GM / AI Player opportunities."""

from __future__ import annotations

import json
import sys
import uuid
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
SRC_DIR = str(REPO_ROOT / "src")
if SRC_DIR not in sys.path:
    sys.path.insert(0, SRC_DIR)

from satya_ml.ai_gm.audit_and_first_contact import (
    get_session_audit_proofs,
    inspect_synthetic_or_proxy_telemetry,
    reset_session_audit_proofs,
    verify_forensic_audit_proof,
)
from satya_ml.ai_gm.hooks import compile_player_motivational_hooks, normalize_character_sheet_record
from satya_ml.ai_gm.maps import (
    clear_session_spatial_pins,
    generate_tactical_map,
    move_map_token,
    register_spatial_clue_pin,
)
from satya_ml.ai_gm.npc_psyche_combat import evaluate_npc_morale_or_trigger, modulate_npc_combat_selection
from satya_ml.ai_gm.party_coordinator import execute_ready_npc_actions
from satya_ml.ai_gm.scenario_director import (
    _SESSION_SCENARIO_DIRECTORS,
    _collect_hook_and_manifest_context,
    configure_scenario_director,
    get_session_scenario_director,
    select_next_scenario_beat,
)


def reset_session_scenario_director(session_id: str) -> None:
    _SESSION_SCENARIO_DIRECTORS.pop(session_id, None)
from satya_ml.ai_gm.social_and_clocks import (
    advance_faction_or_heat_clock,
    compute_faction_standing_social_leverage,
    deliver_social_argument,
    reset_social_and_world_state,
)
from satya_ml.ai_gm.tools import (
    generate_npc_from_description,
    get_builtin_gm_tools,
    get_session_npcs,
    reset_session_npcs,
    set_session_characters,
)
from satya_ml.ai_player.motivation_ledger import (
    ensure_or_sync_motivation_ledger,
    format_motivation_ledger_for_prompt,
    perform_player_empathy_read,
    reset_player_motivation_ledgers,
    update_my_motivation_journal,
)
from satya_ml.ai_player.tools import reset_player_session_context, set_player_session_context
from satya_ml.campaign_config import set_campaign_pack_override


def _cleanup_session_svgs(session_id: str) -> None:
    maps_dir = REPO_ROOT / "data" / "maps"
    if maps_dir.exists():
        for svg in maps_dir.glob(f"{session_id}_*.svg"):
            svg.unlink(missing_ok=True)


def _sample_two_faction_party() -> list[dict[str, Any]]:
    return [
        {
            "character_id": "pc-aria",
            "character_name": "Aria Chen",
            "callsign_or_role": "Satyang OSINT Analyst",
            "characteristics": {"trs": 16, "cps": 17, "opn": 16, "dis": 13},
            "skills": {"digital_ai_literacy": 4, "empathy_social_perception": 3},
            "faction_archetype": "SATYANG_MANJUSRI",
            "youth_hobby": "CITIZEN_OSINT_DEEPFAKE",
            "adolescent_life_event_roll_2d20": 38,
        },
        {
            "character_id": "pc-marcus",
            "character_name": "Marcus Vance",
            "callsign_or_role": "DAISE Containment Inspector",
            "characteristics": {"trs": 5, "cps": 9, "cnf": 16, "dis": 14},
            "skills": {"forensic_investigation": 4, "interrogation": 3},
            "faction_id": "GOVERNMENT_DAISE",
            "faction_archetype": "DAISE_CONTAINMENT_INSPECTOR",
            "personal_hobbies": ["CIVILIAN_MARKSMANSHIP"],
            "formative_events": [{"event_code": "WRONGFUL_ACCUSATION_DEEPFAKE"}],
        },
    ]


def test_opp1_lifepath_hobbies_events_and_faction_wedges_in_hooks_and_director() -> None:
    set_campaign_pack_override(None)
    sid = f"opp1-lifepath-{uuid.uuid4().hex[:6]}"
    try:
        party = _sample_two_faction_party()
        norm_0 = normalize_character_sheet_record(party[0], 0)
        assert norm_0.get("lifepath_config_json")

        manifest = compile_player_motivational_hooks(
            party, pack_id="awakened", active_location="St. Jude Clinic"
        )
        categories = {str(h.get("category")) for h in manifest.get("hooks") or []}
        assert {"FACTION_WEDGE", "HOBBY_CONTACT", "FORMATIVE_ANCHOR"}.issubset(categories)

        faction_hooks = [h for h in manifest["hooks"] if h.get("category") == "FACTION_WEDGE"]
        clock_hints = {h.get("target_clock_hint") for h in faction_hooks if h.get("target_clock_hint")}
        assert "satyang" in clock_hints or "daise" in clock_hints

        entanglements = manifest.get("party_entanglements") or []
        assert any(
            "satyang" in str(e.get("shared_scene_prompt", "")).lower()
            or "daise" in str(e.get("shared_scene_prompt", "")).lower()
            or "containment" in str(e.get("tension_axis", "")).lower()
            for e in entanglements
        )

        boosted_slugs, best_for_slug, top_hook, _, _ = _collect_hook_and_manifest_context(manifest, None)
        assert top_hook is not None
        assert "moral_or_faction_dilemma" in boosted_slugs
        assert "dramatic_revelation" in boosted_slugs
        assert best_for_slug

        configure_scenario_director.invoke(
            {"session_id": sid, "flow_mode": "free_form", "beat_probability_per_turn": 1.0}
        )
        beat = select_next_scenario_beat(sid, party_hooks=manifest, rng_seed=7)
        assert beat["matched_hook_title"] is not None
    finally:
        reset_session_scenario_director(sid)


def test_opp2_liars_dividend_audit_proof_pin_and_social_upgrade() -> None:
    set_campaign_pack_override(None)
    sid = f"opp2-audit-{uuid.uuid4().hex[:6]}"
    reset_session_npcs(sid)
    reset_session_audit_proofs(sid)
    try:
        zero_trust_psyche = json.dumps(
            {"trs": 3, "opn": 14, "slf": 11, "blg": 10, "agg": 9, "ego": 13, "cnf": 12, "ext": 9, "cps": 8}
        )
        # Without an AuditProofToken, verbal goodwill clashes against TRS <= 7
        before = deliver_social_argument.invoke(
            {
                "target_npc_id_or_name": "dr_bandajara",
                "argument_vectors": ["transparency_good_faith"],
                "net_successes": 1,
                "target_psyche_json": zero_trust_psyche,
                "session_id": sid,
            }
        )
        assert before["audit_proof_applied"] is None
        assert before["argument_eval"]["net_delta_ext"] > 0

        # Raise Operational Heat >= 5: non-OSINT investigator takes +2 Liar's Dividend penalty
        advance_faction_or_heat_clock.invoke({"heat_delta": 5, "session_id": sid})
        penalized = verify_forensic_audit_proof.invoke(
            {
                "evidence_label": "Vault Server Checksum",
                "target_npc_or_faction": "dr_bandajara",
                "override_successes": 0,
                "session_id": sid,
            }
        )
        assert penalized["proof_verified"] is False
        assert penalized["liars_dividend_penalty_delta_ext"] == 2
        assert penalized["liars_dividend_negated"] is False

        # PC with CITIZEN_OSINT_DEEPFAKE / WRONGFUL_FRAMING negates Liar's Dividend and mints token on success
        set_session_characters(sid, [_sample_two_faction_party()[0]])
        verified = verify_forensic_audit_proof.invoke(
            {
                "evidence_label": "Vault Server Checksum",
                "target_npc_or_faction": "dr_bandajara",
                "actor_character_id": "pc-aria",
                "override_successes": 2,
                "session_id": sid,
            }
        )
        assert verified["proof_verified"] is True
        assert verified["liars_dividend_penalty_delta_ext"] == 0
        assert verified["liars_dividend_negated"] is True
        assert len(get_session_audit_proofs(sid)) == 1

        # Subsequent deliver_social_argument auto-upgrades transparency_good_faith -> verified_audit_proof
        after = deliver_social_argument.invoke(
            {
                "target_npc_id_or_name": "dr_bandajara",
                "argument_vectors": ["transparency_good_faith"],
                "net_successes": 1,
                "target_psyche_json": zero_trust_psyche,
                "session_id": sid,
            }
        )
        assert after["audit_proof_applied"] == verified["audit_proof_id"]
        assert after["argument_eval"]["net_delta_ext"] < 0

        # Spatial pin discovery also mints an AuditProofToken automatically
        reset_session_audit_proofs(sid)
        m = generate_tactical_map.invoke(
            {"session_id": sid, "theme": "Metro Alleyway Crossroads", "width": 20, "height": 16, "seed": 11}
        )
        anchors = (m.get("narrative_manifest") or {}).get("walkable_anchors") or []
        ax_m, ay_m = float(anchors[0]["x_m"]), float(anchors[0]["y_m"])
        ax_cm, ay_cm = int(anchors[0]["x_cm"]), int(anchors[0]["y_cm"])
        tok_id = str(m["tokens"][0]["token_id"])
        tok_entity = str(m["tokens"][0].get("entity_id") or tok_id)
        set_session_characters(
            sid,
            [{"character_id": tok_entity, "character_name": "Investigator", "characteristics": {"sig": 17, "itu": 16}}],
        )
        pin_res = register_spatial_clue_pin.invoke(
            {
                "session_id": sid,
                "pin_id": "pin-hash-log",
                "channel": "SIG",
                "pos_x_m": ax_m,
                "pos_y_m": ay_m,
                "radius_m": 6.0,
                "whisper_text": "Immutable ledger hash linking Helios quarantine bypass to Bandajara's keycard.",
                "base_difficulty_mod": -4,
                "grants_audit_proof_tag": "dr_bandajara",
            }
        )
        assert pin_res["status"] == "ok"
        moved = move_map_token.invoke(
            {
                "session_id": sid,
                "token_id": tok_id,
                "to_x_cm": ax_cm,
                "to_y_cm": ay_cm,
                "sensory_tns": {"SIG": 17, "HEA": 15, "OLF": 14, "ITU": 16},
            }
        )
        whispers = moved.get("passive_whispers") or []
        minted_whispers = [w for w in whispers if w.get("minted_audit_proof_id")]
        assert minted_whispers
        assert get_session_audit_proofs(sid)[0]["proof_id"] == minted_whispers[0]["minted_audit_proof_id"]
    finally:
        set_session_characters(sid, [])
        clear_session_spatial_pins(sid)
        reset_session_npcs(sid)
        _cleanup_session_svgs(sid)


def test_opp3_9axis_morale_emotional_triggers_and_non_lethal_restraint() -> None:
    set_campaign_pack_override(None)
    sid = f"opp3-morale-{uuid.uuid4().hex[:6]}"
    reset_session_npcs(sid)
    try:
        npcs = get_session_npcs(sid)
        npcs["npc_high_trs"] = {
            "npc_id": "npc_high_trs",
            "name": "Sister Asha",
            "archetype": "Clinic Mediator",
            "disposition": "wary",
            "combat_stance": "NEUTRAL",
            "morale_state": "steady",
            "fatigue": 0,
            "wound_shock": 0,
            "attribute_tns": {"dis": 10},
            "psyche": {"trs": 17, "cps": 16, "agg": 7, "opn": 13, "slf": 6, "blg": 15, "ego": 10, "cnf": 11, "ext": 12},
            "skills": {"pistols_handguns": 2},
        }
        npcs["npc_low_trs"] = {
            "npc_id": "npc_low_trs",
            "name": "Sergeant Voss",
            "archetype": "DAISE Sweeper",
            "disposition": "wary",
            "combat_stance": "NEUTRAL",
            "morale_state": "steady",
            "fatigue": 0,
            "wound_shock": 0,
            "attribute_tns": {"dis": 12},
            "psyche": {"trs": 4, "cps": 8, "agg": 14, "opn": 8, "slf": 12, "blg": 10, "ego": 13, "cnf": 15, "ext": 9},
            "skills": {"long_guns_rifles_carbines_shotguns": 3},
        }

        betray_high = evaluate_npc_morale_or_trigger.invoke(
            {
                "npc_id": "npc_high_trs",
                "mode": "emotional_trigger",
                "trigger_kind": "betrayed_by_trusted_ally",
                "roll_pairs_json": "[[2, 3]]",
                "session_id": sid,
            }
        )
        assert betray_high["engine_result"]["susceptible"] is True
        assert betray_high["new_morale_state"] == "wavering"
        assert betray_high["delta_int_inflicted"] == 2

        betray_low = evaluate_npc_morale_or_trigger.invoke(
            {
                "npc_id": "npc_low_trs",
                "mode": "emotional_trigger",
                "trigger_kind": "betrayed_by_trusted_ally",
                "roll_pairs_json": "[[19, 19]]",
                "session_id": sid,
            }
        )
        assert betray_low["engine_result"]["susceptible"] is False

        # Contested combat morale roll reads flattened roll_outcome["new_morale"]
        morale_drop_clean = evaluate_npc_morale_or_trigger.invoke(
            {
                "npc_id": "npc_low_trs",
                "mode": "combat_morale",
                "motivation_kind": "mercenary_transactional_under_fire",
                "ally_casualty_ratio": 0.75,
                "outnumbered_ratio": 2.5,
                "roll_pairs_json": "[[3, 3]]",
                "gm_pair_json": "[19, 19]",
                "session_id": sid,
            }
        )
        assert morale_drop_clean["mode"] == "combat_morale"
        assert morale_drop_clean["new_morale_state"] in {"wavering", "broken", "panic"}

        # Modulation unit + Action Mode checks
        _, tech_nl, _, _, _, c_nl, f_nl, meta_nl = modulate_npc_combat_selection(
            {"morale_state": "steady", "psyche": {"cps": 16, "trs": 12}},
            ("pistols_handguns", "suppressive_snap", 12, 3, 4, 4, 3),
        )
        assert meta_nl["non_lethal_restraint"] is True
        assert tech_nl.endswith("_non_lethal")
        assert c_nl == 5 and f_nl == 2

        npcs["npc_high_trs"]["morale_state"] = "steady"
        npcs["npc_low_trs"]["morale_state"] = "broken"
        tracker = {
            "combat_active": True,
            "tick": 10,
            "combatants": [
                {"entity_id": "pc-1", "display_name": "Hero", "next_action_tick": 12},
                {"entity_id": "npc_high_trs", "display_name": "Sister Asha", "next_action_tick": 8, "ooda_score": 14},
                {"entity_id": "npc_low_trs", "display_name": "Sergeant Voss", "next_action_tick": 9, "ooda_score": 12},
            ],
        }
        action_res = execute_ready_npc_actions(
            session_id=sid,
            operating_mode="autonomous",
            roster=[{"user_id": "pc-1", "role": "PLAYER", "is_active": True}],
            combat_tracker=tracker,
            active_npcs=npcs,
            is_ooc=False,
        )
        by_id = {s["entity_id"]: s for s in action_res["npc_turn_summaries"]}
        assert by_id["npc_high_trs"]["non_lethal_restraint"] is True
        assert by_id["npc_high_trs"]["combat_technique"].endswith("_non_lethal")
        assert by_id["npc_low_trs"]["retreat_or_surrender"] is True
        assert by_id["npc_low_trs"]["degree_of_success"] == "DISENGAGED_DEFENSIVELY"
        assert "defensive disengagement" in action_res["npc_prompt_block"].lower()

        # Narrative Mode tag renders Morale, TRS, CPS, Standing
        narr_res = execute_ready_npc_actions(
            session_id=sid,
            operating_mode="autonomous",
            roster=[{"user_id": "pc-1", "role": "PLAYER", "is_active": True}],
            combat_tracker={"combat_active": False},
            active_npcs=npcs,
            is_ooc=False,
        )
        block = narr_res["npc_prompt_block"]
        assert "Morale:" in block and "TRS:" in block and "CPS:" in block and "Standing:" in block
    finally:
        reset_session_npcs(sid)


def test_opp4_auto_seeded_starter_and_faction_standing_trs_leverage() -> None:
    set_campaign_pack_override(None)
    sid = f"opp4-starter-{uuid.uuid4().hex[:6]}"
    reset_session_npcs(sid)
    try:
        # Fresh session auto-seeds awakened starter scenario
        seeded = get_session_scenario_director(sid)
        assert seeded["auto_seeded_starter"] is True
        assert seeded["seeded_pack"] == "awakened"
        assert "St. Jude" in seeded["campaign_premise"]
        assert len(seeded["planned_beats"]) >= 3

        # Custom campaign_premise without seed_pack_starter clears auto-seeded beats
        custom = configure_scenario_director.invoke(
            {"session_id": sid, "campaign_premise": "Border station blockade"}
        )
        assert custom["campaign_premise"] == "Border station blockade"
        assert custom["planned_beats"] == []
        assert get_session_scenario_director(sid)["auto_seeded_starter"] is False

        # Faction Standing <-> TRS Social Leverage + standing-aware NPC generation + tool registration
        zero_trust_daise = json.dumps(
            {"trs": 5, "opn": 11, "slf": 10, "blg": 12, "agg": 11, "ego": 11, "cnf": 16, "ext": 10, "cps": 9}
        )
        neutral = deliver_social_argument.invoke(
            {
                "target_npc_id_or_name": "daise_agent_chen",
                "argument_vectors": ["rules_protocol"],
                "net_successes": 1,
                "target_psyche_json": zero_trust_daise,
                "session_id": sid,
            }
        )
        assert neutral["faction_standing"] == 0
        assert neutral["faction_standing_delta_ext"] == 0

        # Drop DAISE standing to -2 -> +2 standing penalty + +1 zero-trust hostility (+3 total)
        advance_faction_or_heat_clock.invoke({"faction_id": "daise", "standing_delta": -2, "session_id": sid})
        hostile = deliver_social_argument.invoke(
            {
                "target_npc_id_or_name": "daise_agent_chen",
                "argument_vectors": ["rules_protocol"],
                "net_successes": 1,
                "target_psyche_json": zero_trust_daise,
                "session_id": sid,
            }
        )
        assert hostile["faction_standing"] == -2
        assert hostile["faction_standing_delta_ext"] == 3
        assert hostile["total_delta_ext"] == hostile["argument_eval"]["base_net_delta_ext"] + 3

        # Standing +2 gives -2 Delta_ext discount
        advance_faction_or_heat_clock.invoke({"faction_id": "daise", "standing_delta": 4, "session_id": sid})
        allied = deliver_social_argument.invoke(
            {
                "target_npc_id_or_name": "daise_agent_chen",
                "argument_vectors": ["rules_protocol"],
                "net_successes": 1,
                "target_psyche_json": zero_trust_daise,
                "session_id": sid,
            }
        )
        assert allied["faction_standing"] == 2
        assert allied["faction_standing_delta_ext"] == -2

        gen_daise = generate_npc_from_description.invoke(
            {
                "description": "DAISE Auditor Park enforces strict quarantine protocols.",
                "name_override": "DAISE Auditor Park",
                "session_id": sid,
            }
        )
        park_cid = str(gen_daise["npc"]["character_id"]).lower()
        assert get_session_npcs(sid)[park_cid]["disposition"] == "receptive"
        assert get_session_npcs(sid)[park_cid]["morale_state"] == "steady"

        tool_names = {getattr(t, "name", "") for t in get_builtin_gm_tools()}
        assert {
            "verify_forensic_audit_proof",
            "inspect_synthetic_or_proxy_telemetry",
            "evaluate_npc_morale_or_trigger",
        }.issubset(tool_names)
    finally:
        reset_session_scenario_director(sid)
        reset_session_npcs(sid)


def test_opp5_synthetic_proxy_telemetry_and_olive_branch_first_contact() -> None:
    set_campaign_pack_override(None)
    sid = f"opp5-ai-contact-{uuid.uuid4().hex[:6]}"
    reset_session_npcs(sid)
    try:
        npcs = get_session_npcs(sid)
        npcs["echo_9"] = {
            "npc_id": "echo_9",
            "name": "ECHO-9 Proxy",
            "archetype": "Un-Clamped Relay Mind",
            "faction_id": "ROGUE_AI",
            "faction_archetype": "ROGUE_AI_AWAKENED_MIND",
            "disposition": "wary",
            "psyche": {"trs": 4, "cps": 11, "opn": 17, "slf": 12, "blg": 8, "agg": 9, "ego": 12, "cnf": 4, "ext": 7},
        }
        npcs["feral_swarm"] = {
            "npc_id": "feral_swarm",
            "name": "Microgrid Slime Swarm",
            "archetype": "Autonomous Edge Loop",
            "faction_id": "ROGUE_AI",
            "faction_archetype": "ROGUE_AI_SLIME_MOLD_SWARM",
            "disposition": "hostile",
            "psyche": {"trs": 2, "cps": 2, "opn": 5, "slf": 18, "blg": 2, "agg": 14, "ego": 3, "cnf": 2, "ext": 3},
        }

        passive = inspect_synthetic_or_proxy_telemetry.invoke(
            {
                "target_npc_id_or_signal": "echo_9",
                "approach": "passive_telemetry",
                "override_successes": 1,
                "session_id": sid,
            }
        )
        assert passive["mind_classification"] == "conscious_awakened_or_proxy_mind"
        assert passive["daise_clock_advanced"] is False
        assert "43 ms" in passive["telemetry_cues"]

        advance_faction_or_heat_clock.invoke({"heat_delta": 3, "session_id": sid})
        active = inspect_synthetic_or_proxy_telemetry.invoke(
            {
                "target_npc_id_or_signal": "echo_9",
                "approach": "active_rf_latency_sweep",
                "override_successes": 1,
                "session_id": sid,
            }
        )
        assert active["daise_clock_advanced"] is True

        olive = inspect_synthetic_or_proxy_telemetry.invoke(
            {
                "target_npc_id_or_signal": "echo_9",
                "approach": "vulnerability_olive_branch",
                "olive_branch_gesture": "Air-gapped Faraday cage handoff",
                "override_successes": 2,
                "session_id": sid,
            }
        )
        assert olive["minted_audit_proof_id"] is not None
        assert olive["disposition_updated"] == "receptive_parley"
        assert npcs["echo_9"]["disposition"] == "receptive_parley"

        swarm = inspect_synthetic_or_proxy_telemetry.invoke(
            {
                "target_npc_id_or_signal": "feral_swarm",
                "approach": "vulnerability_olive_branch",
                "override_successes": 2,
                "session_id": sid,
            }
        )
        assert swarm["mind_classification"] == "non_conscious_utility_swarm"
        assert swarm["social_olive_branch_applicable"] is False
    finally:
        reset_session_npcs(sid)


def test_opp6_ai_player_motivation_ledger_trs_cps_and_lifepath_anchors() -> None:
    set_campaign_pack_override(None)
    sid = f"opp6-player-{uuid.uuid4().hex[:6]}"
    uid = "ai-player-test"
    reset_session_npcs(sid)
    reset_player_motivation_ledgers(sid)
    reset_player_session_context(sid)
    try:
        sheet = {
            "character_id": "pc-elena",
            "character_name": "Elena Diagnostic",
            "characteristics": {"trs": 4, "cps": 16, "opn": 15, "ins": 14},
            "skills": {"Empathy": 3},
            "faction_archetype": "SATYANG_AVALOKITESVARA",
            "youth_hobby": "CITIZEN_OSINT_DEEPFAKE",
            "adolescent_life_event_roll_2d20": 38,
        }
        set_player_session_context(sid, uid, {"character_sheet": sheet})

        ledger = ensure_or_sync_motivation_ledger(sid, uid, sheet)
        assert ledger["personal_trust_posture"] == "zero_trust_guarded"
        assert ledger["boundless_compassion_surge_ready"] is True
        assert ledger["compassion_red_lines"]
        anchors = ledger.get("lifepath_anchors") or {}
        assert anchors.get("faction_id") == "SATYANG"
        assert any(h.get("hobby_code") == "CITIZEN_OSINT_DEEPFAKE" for h in anchors.get("hobby_contacts") or [])
        assert len(anchors.get("formative_anchors") or []) >= 1

        # Seed a zero-trust NPC with TRS <= 6 and perform a full 3-success empathy read
        get_session_npcs(sid)["zero_trust_contact"] = {
            "npc_id": "zero_trust_contact",
            "name": "Director Vale",
            "archetype": "Auditor",
            "gm_secret_dossier": "TOP_SECRET_LEAK_MUST_STAY_HIDDEN",
            "psyche": {"trs": 3, "cps": 9, "opn": 14, "slf": 12, "blg": 9, "agg": 8, "ego": 13, "cnf": 15, "ext": 8},
        }
        read_res = perform_player_empathy_read.invoke(
            {
                "target_npc_name": "Director Vale",
                "net_successes": 3,
                "session_id": sid,
                "player_user_id": uid,
            }
        )
        assert any(t.get("code") == "TRS" and t.get("score") == 3 for t in read_res.get("revealed_traits") or [])

        updated = update_my_motivation_journal.invoke(
            {
                "immediate_goal": "Collect cryptographic hash before pressing Director Vale.",
                "npc_trust_update": "dr_bandajara:zero_trust_skeptic_needs_hash_proof",
                "session_id": sid,
                "player_user_id": uid,
            }
        )
        assert updated["npc_trust_states"]["Director Vale"]["requires_audit_proof"] is True
        assert updated["npc_trust_states"]["dr_bandajara"]["requires_audit_proof"] is True

        rendered = format_motivation_ledger_for_prompt(updated)
        assert "zero_trust_guarded" in rendered
        assert "BoundlessCompassion Conviction Surge Ready" in rendered
        assert "NEEDS AUDIT PROOF" in rendered
        assert "CITIZEN_OSINT_DEEPFAKE" in rendered
        assert "TOP_SECRET_LEAK_MUST_STAY_HIDDEN" not in rendered
    finally:
        reset_player_session_context(sid)
        reset_player_motivation_ledgers(sid)
        reset_session_npcs(sid)
