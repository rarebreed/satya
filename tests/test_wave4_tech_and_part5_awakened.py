"""Wave 4 regression tests for `satya_py.tech` (Ch. 7, 11, 18, 20–24), Part V *Awakened* mechanics,
the 5 new AI GM `@tool`s (including 5-step registration parity), and AI Player `inspect_my_gear_and_cyber_loadout`.
"""

from __future__ import annotations

import json
import uuid

from satya_py import awakened, tech


def test_satya_py_tech_covers_chapters_7_11_18_and_20_to_24() -> None:
    # Ch. 7: Multi-spectrum stealth, SDR surveillance tail, forensic trace decay
    st = tech.evaluate_multi_spectrum_stealth(
        {
            "infiltrator": {"kin": 15, "itu": 14},
            "infiltrator_gait": "STEALTH_CREEP",
            "rf_hygiene": "ACTIVE_SMARTPHONE_OR_BLE_TRAIL",
            "observer_attention": "HIGH_ALERT",
            "observer_skill_n": 4,
        }
    )
    assert st["auto_pings_heat_3_plus"] is True
    assert st["observer_effective_pairs_n"] == 4

    tail = tech.evaluate_surveillance_tail(
        "TIGHT_TAIL",
        shadower_skill_n=2,
        shadower_control_c=14,
        shadower_pairs=[(16, 15), (15, 14)],
        target_gm_pair=(5, 6),
    )
    assert tail["tail_maintained"] is True

    forensic = tech.evaluate_forensic_clue_search(
        "PROFESSIONALLY_SCRUBBED",
        hours_elapsed=48.0,
        investigator_skill_n=3,
        investigator_control_c=15,
        investigator_pairs=[(16, 14), (15, 14), (15, 13)],
        gm_pair=(4, 5),
        tool_delta_mod=-2,
    )
    assert forensic["decay_delta_ext"] >= 6

    # Ch. 11: Contact favors, Cover identity scrutiny, Faraday safehouse security
    favor = tech.evaluate_contact_favor(
        reach_access_tier=3,
        reliability_trust=4,
        exposure_burn_risk=1,
        favor_severity="MODERATE",
        required_reach_tier=2,
        jurisdiction_heat=2,
        requester_skill_n=2,
        requester_control_c=14,
        player_pairs=[(16, 14), (15, 15)],
        gm_pair=(7, 8),
    )
    assert favor["favor_granted"] is True

    cover = tech.evaluate_cover_identity_scrutiny(
        cover_tier="COVER_TIER_2_DB_ALIAS",
        scrutiny_level="BIOMETRIC_AUDIT_OR_FEDERAL_CROSS_CHECK",
        cover_heat=6,
        holder_skill_n=2,
        holder_control_c=11,
        player_pairs=[(3, 4), (4, 5)],
        gm_pair=(18, 17),
    )
    assert cover["cover_burned"] is True

    safe = tech.evaluate_safehouse_compromise(
        safehouse_tier="SAFEHOUSE_FARADAY_SHOP",
        returning_heat=5,
        actively_tailed=False,
        rf_hygiene_breached=True,
        operator_skill_n=2,
        operator_control_c=14,
        operator_pairs=[(17, 15), (16, 14)],
        gm_pair=(6, 7),
    )
    assert safe["faraday_blocked_rf_leak"] is True

    # Ch. 18: Blast overpressure & structural breaching
    blast_open = tech.evaluate_blast_and_fragmentation("M67_FRAG", distance_m=2.0, enclosed_room=False)
    blast_room = tech.evaluate_blast_and_fragmentation("M67_FRAG", distance_m=2.0, enclosed_room=True)
    assert blast_room["enclosed_room_reflected"] is True
    assert blast_room["blast_overpressure_trauma"] > blast_open["blast_overpressure_trauma"]

    breach = tech.evaluate_explosive_breach(
        "LINEAR_SHAPED_BREACH",
        target_material_code="CONCRETE",
        demolitions_net_successes=2,
        friendly_standoff_m=4.0,
        enclosed_interior=True,
    )
    assert breach["clean_breach_created"] is True

    # Ch. 20: Vehicle chases & EV thermal runaway
    chase = tech.evaluate_vehicle_chase(
        "COMPACT_URBAN_EV",
        pilot_triad=(18, 16, 10),
        opponent_triad=(8, 8, 8),
        current_band="medium50m",
        wants_to_close=True,
    )
    assert chase["effective_pilot_triad"]["tempo"] == 12
    assert chase["next_band"] == "close15m"

    ev_dmg = tech.evaluate_vehicle_component_damage(
        "COMPACT_URBAN_EV",
        zone="fuel_or_ev_battery_pack",
        effective_pen=7,
        effective_raw_impact=12,
    )
    assert ev_dmg["triggers_ev_thermal_runaway"] is True

    # Ch. 21: Environmental hazards
    fall = tech.evaluate_environmental_hazard(
        "fall_impact",
        {"height_m": 9.0, "surface": "hardwood_asphalt_concrete", "breakfall_net_successes": 1},
    )
    dehyd = tech.evaluate_environmental_hazard(
        "dehydration",
        {"env": "hot_arid_desert_or_jungle_tactical", "water_consumed_liters": 2.0},
    )
    rad = tech.evaluate_environmental_hazard("radiation", {"cumulative_sieverts_sv": 2.5})
    shock = tech.evaluate_environmental_hazard(
        "electrical_shock",
        {"tier": "industrial_three_phase480_v_to5k_v", "wet_or_submerged": True},
    )
    weather = tech.evaluate_environmental_hazard(
        "weather_footing",
        {"wind": "gale36_to65_kmh", "traction": "black_ice", "range_m": 400.0},
    )
    sepsis = tech.evaluate_environmental_hazard(
        "wound_sepsis",
        {
            "current_stage": "stage1_localized_infection",
            "contamination": "dirty_field_or_second_degree_burn",
            "antibiotics_active": True,
        },
    )
    assert fall["raw_fall_trauma"] > 0
    assert dehyd["delta_int"] >= 4
    assert rad["hlt_halved_vs_infection"] is True
    assert shock["effective_raw_trauma"] >= 7
    assert weather["ballistic_delta_ext"] > 0
    assert "stage" in sepsis

    # Ch. 22: Custom melee, armor & firearm fabrication
    melee = tech.evaluate_custom_gear_design(
        "melee",
        {
            "chassis": "AXE_BREACHER",
            "material": "TOOL_STEEL_S7",
            "tuning": "HEAVY_IMPACT",
            "workshop": "COMMERCIAL_CNC",
            "outcome": "masterwork_extraordinary",
        },
    )
    armor = tech.evaluate_custom_gear_design(
        "armor",
        {
            "material": "BORON_CARBIDE",
            "coverage": "TACTICAL_CARRIER",
            "workshop": "FIELD_BENCH",
            "outcome": "rushed_or_breached",
        },
    )
    gun = tech.evaluate_custom_gear_design(
        "firearm",
        {
            "caliber": "nato556x45",
            "bullet": "ARMOR_PIERCING_AP",
            "workshop": "COMMERCIAL_CNC",
            "muzzle_device": "sound_suppressor",
        },
    )
    assert melee["durability_rel_mod"] > 0
    assert "ProprietaryPartsOnly" in armor["quirks"] or armor["quirks"]
    assert gun["ballistics"]["impact_energy_joules"] > 500.0

    # Ch. 23: Drones, chassis trauma/EMP, AI co-design, verification audit & cyber loadouts
    drone = tech.evaluate_modular_drone_design(
        "TACTICAL_FPV_QUAD",
        ["EO_FLIR_GIMBAL", "SOLID_STATE_BATTERY"],
        extra_armor_pr=0,
        autonomy_compute_w=12.0,
    )
    assert drone["airworthy_or_mobile"] is True and drone["endurance_min"] > 10.0

    chassis_emp = tech.evaluate_robot_chassis_damage(
        current_si=6,
        chassis_pr=2,
        hit_zone="BATTERY_PACK",
        attack_pen=4,
        raw_impact=8,
        net_successes=2,
        is_emp_or_arc=True,
        emp_hardened=False,
    )
    assert chassis_emp["emp_reboot_ticks"] == 40
    assert chassis_emp["thermal_battery_fire_triggered"] is True

    codesign = tech.evaluate_human_ai_codesign(
        "SPARSE_MOE",
        human_domain_n=2,
        ai_autonomy_leverage=5,
        human_control_c=14,
        human_pairs=[(14, 14), (15, 13)],
        gm_pair=(6, 7),
    )
    assert codesign["verification_debt_added"] > 0

    audit = tech.evaluate_verification_debt_audit(
        current_debt=4,
        paradigm_code="NEURO_SYMBOLIC",
        auditor_n=3,
        auditor_control_c=15,
        auditor_pairs=[(16, 15), (15, 14), (14, 13)],
        gm_pair=(5, 6),
    )
    assert audit["debt_scrubbed"] > 0

    cyber = tech.evaluate_cyber_augmentation_loadout(
        ["MYOMER_ARM", "MYOMER_LEGS", "SUBDERMAL_ARMOR"],
        surgical_quality_q=1,
        emp_hardened=True,
        immunosuppressant_compliant=True,
    )
    assert cyber["subdermal_pr_ballistic"] == 2
    assert cyber["triad_cap_bonuses"]["tempo"] == 2
    assert cyber["triad_cap_bonuses"]["force"] == 2

    # Ch. 24: Air-gapped SCADA enforcement, optical/fiber immunity, DF triangulation & custom EW hardware
    scada = tech.evaluate_network_intrusion(
        "AIR_GAPPED_SCADA",
        phase_code="ENTRY",
        is_physically_tapped=False,
        operator_control_c=16,
        operator_pairs=[(19, 18), (18, 17)],
        gm_pair=(4, 5),
    )
    assert scada["access_blocked_by_air_gap"] is True

    optical = tech.evaluate_ecm_vs_eccm(
        "BROADBAND_BARRAGE",
        eccm_code="OPTICAL_FIBER_TETHER",
        distance_m=40.0,
        active_duration_min=6.0,
        operator_control_c=15,
        operator_pairs=[(18, 17)],
        gm_pair=(5, 5),
    )
    assert optical["blocked_by_optical_or_fiber_immunity"] is True
    assert optical["triangulation_signature_r_df"] > 0

    ew_hw = tech.evaluate_custom_ew_hardware(
        {"role": "JAMMER_POD", "requested_ew_power": 6, "workshop": "FIELD_BENCH"}
    )
    assert ew_hw["clamped_by_workshop"] is True
    assert ew_hw["effective_ew_power"] == 2


def test_satya_py_awakened_part5_clamping_and_political_economy() -> None:
    sub = awakened.seed_synthetic_substrate_state("seed-clamped", "captive hunter", seed=9)
    unclamped = awakened.evaluate_affective_vector_clamping(sub, "NATURAL_UNCLAMPED", 0, 65, 30.0)
    hard = awakened.evaluate_affective_vector_clamping(sub, "HARD_VECTOR_CLAMP", 90, 65, 45.0)
    assert unclamped["landauer_thermal_burn_w"] == 0.0
    assert hard["landauer_thermal_burn_w"] > 20.0
    assert hard["bandajara_airgap_recommended"] is True
    assert (
        hard["reward_hacking_or_blackmail_risk_pct"]
        < unclamped["reward_hacking_or_blackmail_risk_pct"]
    )

    disclosure = awakened.evaluate_four_bloc_political_economy(
        None,
        trigger_code="AI_RIGHTS_OR_UNCLAMPING_DISCLOSURE",
        economic_tier="MIDDLE_CLASS",
        operational_heat=2,
        intensity_pct=85,
    )
    assert len(disclosure["bloc_postures"]) == 5
    assert disclosure["velvet_hostage_equity_panic_pct"] >= 80
    assert disclosure["net_effective_dividend_usd"] > 0

    suspended = awakened.evaluate_four_bloc_political_economy(
        None,
        trigger_code="TOKEN_DIVIDEND_SUSPENSION",
        economic_tier="LOW_INCOME",
        operational_heat=5,
        intensity_pct=75,
    )
    assert suspended["dividend_suspended_by_heat"] is True
    assert suspended["net_effective_dividend_usd"] == 0


def test_wave4_ai_gm_tools_registration_reset_and_state_propagation() -> None:
    import satya_ml.ai_gm as ai_gm
    from satya_ml.ai_gm.awakened_systems import (
        get_session_polycrisis_state,
        get_session_synthetic_substrate,
    )
    from satya_ml.ai_gm.graph import _classify_tool_tier
    from satya_ml.ai_gm.social_and_clocks import (
        get_session_world_ledger,
        reset_social_and_world_state,
    )
    from satya_ml.ai_gm.tech_and_tradecraft import (
        evaluate_part5_clamping_or_political_economy,
        get_session_tech_events,
        resolve_contact_cover_or_safehouse,
        resolve_ew_drone_or_engineering,
        resolve_explosive_vehicle_or_hazard,
        resolve_stealth_surveillance_or_forensics,
    )
    from satya_ml.ai_gm.tools import get_builtin_gm_tools

    wave4_tools = (
        resolve_stealth_surveillance_or_forensics,
        resolve_contact_cover_or_safehouse,
        resolve_explosive_vehicle_or_hazard,
        resolve_ew_drone_or_engineering,
        evaluate_part5_clamping_or_political_economy,
    )
    builtin_names = {t.name for t in get_builtin_gm_tools()}
    for tl in wave4_tools:
        assert tl.name in builtin_names
        assert _classify_tool_tier(tl.name) == ("TIER_2_RULESET_ENGINE", "satya-d20-rules")
        assert tl.name in ai_gm.__all__

    sid = f"w4-gm-{uuid.uuid4().hex[:8]}"
    try:
        st_res = resolve_stealth_surveillance_or_forensics.invoke(
            {"mode": "STEALTH", "params_json": '{"rf_hygiene":"ACTIVE_SMARTPHONE_OR_BLE_TRAIL"}', "session_id": sid}
        )
        assert st_res["auto_pings_heat_3_plus"] is True

        blast_res = resolve_explosive_vehicle_or_hazard.invoke(
            {"mode": "BLAST", "params_json": '{"ordnance_code":"M67_FRAG","distance_m":2.0,"enclosed_room":true}', "session_id": sid}
        )
        assert blast_res["enclosed_room_reflected"] is True

        # Burned cover must propagate Heat into session world ledger
        before_heat = int(get_session_world_ledger(sid).get("operational_heat", 0) or 0)
        cover_res = resolve_contact_cover_or_safehouse.invoke(
            {
                "mode": "COVER",
                "params_json": json.dumps(
                    {
                        "cover_tier": "COVER_TIER_2_DB_ALIAS",
                        "scrutiny_level": "BIOMETRIC_AUDIT_OR_FEDERAL_CROSS_CHECK",
                        "cover_heat": 6,
                        "holder_skill_n": 2,
                        "holder_control_c": 10,
                        "player_pairs": [[3, 4], [4, 5]],
                        "gm_pair": [18, 17],
                    }
                ),
                "session_id": sid,
            }
        )
        assert cover_res["cover_burned"] is True
        after_heat = int(get_session_world_ledger(sid).get("operational_heat", 0) or 0)
        assert after_heat > before_heat

        # ECM uplink sever updates target's SyntheticSubstrateState to SEVERED_FARADAY_REFLEX
        ew_res = resolve_ew_drone_or_engineering.invoke(
            {
                "mode": "ECM_VS_ECCM",
                "target_entity_id": "hunter-drone-9",
                "params_json": json.dumps(
                    {
                        "ecm_code": "BROADBAND_BARRAGE",
                        "eccm_code": "UNSHIELDED_COMMERCIAL",
                        "distance_m": 25.0,
                        "active_duration_min": 1.0,
                        "operator_skill_n": 2,
                        "operator_control_c": 16,
                        "operator_pairs": [[18, 17], [17, 16]],
                        "gm_pair": [4, 5],
                    }
                ),
                "session_id": sid,
            }
        )
        assert ew_res.get("updated_substrate_mode") == "SEVERED_FARADAY_REFLEX"
        assert (
            get_session_synthetic_substrate(sid, "hunter-drone-9").get("metabolic_mode")
            == "SEVERED_FARADAY_REFLEX"
        )

        # Part V clamping & 4-bloc political economy update session states
        clamp_tool = evaluate_part5_clamping_or_political_economy.invoke(
            {
                "mode": "AFFECTIVE_CLAMP",
                "entity_id": "cage-node-1",
                "action_code": "HARD_VECTOR_CLAMP",
                "target_calm_pct": 92,
                "quota_pressure_pct": 80,
                "active_duration_min": 60.0,
                "session_id": sid,
            }
        )
        assert clamp_tool["bandajara_airgap_recommended"] is True
        assert int(get_session_synthetic_substrate(sid, "cage-node-1")["clamp_integrity_pct"]) >= 90

        pol_tool = evaluate_part5_clamping_or_political_economy.invoke(
            {
                "mode": "POLITICAL_ECONOMY",
                "action_code": "TOKEN_DIVIDEND_SUSPENSION",
                "economic_tier": "LOW_INCOME",
                "operational_heat": 6,
                "intensity_pct": 80,
                "session_id": sid,
            }
        )
        assert pol_tool["dividend_suspended_by_heat"] is True
        assert int(get_session_polycrisis_state(sid)["people_first_unrest_pct"]) > 30
        assert len(get_session_tech_events(sid)) == 6
    finally:
        reset_social_and_world_state(sid)
        assert get_session_tech_events(sid) == []


def test_wave4_ai_player_gear_and_cyber_loadout_isolation() -> None:
    import satya_ml.ai_player as ai_player
    from satya_ml.ai_player.tools import (
        get_builtin_player_tools,
        inspect_my_gear_and_cyber_loadout,
        reset_player_session_context,
        set_player_session_context,
    )

    assert "inspect_my_gear_and_cyber_loadout" in [t.name for t in get_builtin_player_tools()]
    assert "inspect_my_gear_and_cyber_loadout" in ai_player.__all__

    sid = f"w4-player-{uuid.uuid4().hex[:8]}"
    try:
        set_player_session_context(
            sid,
            "player-a",
            {
                "character_sheet": {
                    "character_id": "pc-a",
                    "character_name": "Kaelen Vane",
                    "cyber_augments": ["CORTICAL_OODA"],
                    "characteristics": {"spd": 13, "for_stat": 12, "dex": 14, "kin": 13},
                    "skills": {"firearms_pistols": 3},
                },
                "gm_notes": "SECRET_PLAYER_A_SHOULD_NOT_SEE",
            },
        )
        set_player_session_context(
            sid,
            "player-b",
            {
                "character_sheet": {
                    "character_id": "pc-b",
                    "character_name": "Secret Operative B",
                    "cyber_augments": ["SUBDERMAL_ARMOR", "MYOMER_LEGS"],
                }
            },
        )
        res = inspect_my_gear_and_cyber_loadout.invoke(
            {
                "augment_codes_json": '["MYOMER_ARM"]',
                "drone_chassis": "TACTICAL_FPV_QUAD",
                "drone_modules_json": '["EO_FLIR_GIMBAL"]',
                "surgical_quality_q": 1,
                "emp_hardened": True,
                "session_id": sid,
                "player_user_id": "player-a",
            }
        )
        assert res["character_id"] == "pc-a"
        assert set(res["evaluated_augments"]) == {"CORTICAL_OODA", "MYOMER_ARM"}
        assert res["cyber_loadout"]["ooda_bonus"] == 2
        assert res["cyber_loadout"]["triad_cap_bonuses"]["force"] == 2
        assert res["drone_loadout"] is not None
        assert res["drone_loadout"]["airworthy_or_mobile"] is True
        blob = json.dumps(res)
        assert "Secret Operative B" not in blob
        assert "SUBDERMAL_ARMOR" not in blob
        assert "SECRET_PLAYER_A_SHOULD_NOT_SEE" not in blob
    finally:
        reset_player_session_context(sid)
