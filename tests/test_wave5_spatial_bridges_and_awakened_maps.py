"""Wave 5 regression tests: spatial bridges, *Awakened* presets, blackout coupling, and player inspection."""

from __future__ import annotations

import uuid

from satya_ml.ai_gm import (
    compute_token_reachable_zone,
    continue_tactical_map,
    edit_tactical_map_feature,
    evaluate_ballistic_shot_on_map,
    generate_tactical_map,
    get_session_map,
    inspect_tactical_map_point,
    manage_polycrisis_grid_and_strain,
    reset_awakened_systems_state,
    set_session_map,
)
from satya_ml.ai_gm.awakened_systems import get_session_polycrisis_state
from satya_ml.ai_gm.graph import _classify_tool_tier
from satya_ml.ai_gm.maps import infer_sector_preset
from satya_ml.ai_gm.tools import get_builtin_gm_tools
from satya_ml.ai_player.motivation_ledger import inspect_surroundings
from satya_ml.ai_player.tools import (
    reset_player_session_context,
    set_player_session_context,
)
from satya_ml.campaign_config import get_active_campaign_pack, set_campaign_pack_override


def test_awakened_default_preset_and_infer_sector_preset_mappings() -> None:
    set_campaign_pack_override(None)
    try:
        pack = get_active_campaign_pack()
        assert pack["default_sector_preset"] == "community_clinic_annex"

        assert infer_sector_preset("Triage Annex") == "community_clinic_annex"
        assert infer_sector_preset("St. Jude Community Clinic") == "community_clinic_annex"
        assert infer_sector_preset("Corporate Server Bunker") == "corporate_server_bunker"
        assert infer_sector_preset("Satyang Safehouse") == "satyang_safehouse"
        assert infer_sector_preset("Brownout Logistics Depot") == "brownout_logistics_depot"

        # Existing regression aliases must remain intact.
        assert infer_sector_preset("Satyang District Hospital") == "hospital"
        assert infer_sector_preset("Logistics Pallet Warehouse") == "warehouse"
        assert infer_sector_preset("Rain-slick Backstreet Intersection") == "urban_crossroads"
        assert infer_sector_preset("subterranean bunker") == "derelict_corridor"
    finally:
        set_campaign_pack_override(None)


def test_awakened_four_presets_generate_continue_and_bridge_tools() -> None:
    set_campaign_pack_override(None)
    sid = f"wave5-gm-{uuid.uuid4().hex[:6]}"
    try:
        for theme_name, expected_preset in (
            ("St. Jude Community Clinic Annex", "community_clinic_annex"),
            ("Corporate Server Bunker QPU Hall", "corporate_server_bunker"),
            ("Satyang Safehouse Relay Cellar", "satyang_safehouse"),
            ("Brownout Logistics Depot", "brownout_logistics_depot"),
        ):
            generated = generate_tactical_map.invoke(
                {
                    "session_id": sid,
                    "theme": theme_name,
                    "width": 24,
                    "height": 18,
                    "seed": 2035,
                }
            )
            assert generated["theme_preset"] == expected_preset

        bunker = generate_tactical_map.invoke(
            {
                "session_id": sid,
                "theme": "Corporate Server Bunker",
                "width": 24,
                "height": 18,
                "seed": 2035,
            }
        )
        assert bunker.get("faraday_zones"), "CorporateServerBunker should expose Faraday zones"
        assert bunker.get("surveillance_nodes"), "CorporateServerBunker should expose surveillance nodes"

        fz = bunker["faraday_zones"][0]
        fz_x = (fz["x0_cm"] + fz["x1_cm"]) / 200.0
        fz_y = (fz["y0_cm"] + fz["y1_cm"]) / 200.0
        inspected = inspect_tactical_map_point.invoke(
            {"x_m": fz_x, "y_m": fz_y, "session_id": sid}
        )
        assert inspected["in_faraday_zone"] is True
        assert inspected["acoustic_surface"] is not None
        assert inspected["traction_tier"] is not None

        cont = continue_tactical_map.invoke(
            {"session_id": sid, "direction": "EAST", "seed": 2099}
        )
        assert cont["grid_width"] > bunker["grid_width"]

        set_edge = edit_tactical_map_feature.invoke(
            {
                "action": "SET_EDGE",
                "x": 5,
                "y": 5,
                "side": "NORTH",
                "edge_kind": "WALL",
                "material": "DRYWALL",
                "session_id": sid,
            }
        )
        assert set_edge["event_type"] == "MAP_FEATURE_EDITED"

        shot = evaluate_ballistic_shot_on_map.invoke(
            {
                "attacker_x_m": 5.5,
                "attacker_y_m": 4.2,
                "target_x_m": 5.5,
                "target_y_m": 5.8,
                "firearm_preset_id": "hk416_carbine_556",
                "net_successes": 2,
                "session_id": sid,
            }
        )
        assert "HK416" in shot["weapon_name"]
        assert "effective_delta_ext" in shot

        reach = compute_token_reachable_zone.invoke(
            {
                "x_m": fz_x,
                "y_m": fz_y,
                "spd_eff": 11,
                "gait_code": "COMBAT_GLIDE",
                "tick_budget": 4,
                "session_id": sid,
            }
        )
        assert reach["reachable_cell_count"] > 0
        assert isinstance(reach["strips"], list) and reach["strips"]

        edit_tactical_map_feature.invoke(
            {
                "action": "SET_EDGE",
                "x": 4,
                "y": 5,
                "side": "NORTH",
                "edge_kind": "DOOR_CLOSED",
                "material": "SOLID_WOOD",
                "session_id": sid,
            }
        )
        door_toggle = edit_tactical_map_feature.invoke(
            {
                "action": "TOGGLE_DOOR",
                "x": 4,
                "y": 5,
                "side": "NORTH",
                "session_id": sid,
            }
        )
        assert door_toggle["edit_summary"]["kind"] == "DOOR_OPEN"

        blast = edit_tactical_map_feature.invoke(
            {
                "action": "BLAST_OR_BREACH",
                "x_m": 5.5,
                "y_m": 4.9,
                "ordnance_code": "C4",
                "demolitions_net_successes": 2,
                "session_id": sid,
            }
        )
        assert blast["edit_summary"]["blast_outcome"]["breached_edges_count"] >= 1
    finally:
        set_campaign_pack_override(None)


def test_wave5_tool_tiers_and_polycrisis_blackout_sync() -> None:
    tool_names = {t.name for t in get_builtin_gm_tools()}
    for name in (
        "inspect_tactical_map_point",
        "evaluate_ballistic_shot_on_map",
        "compute_token_reachable_zone",
        "edit_tactical_map_feature",
    ):
        assert name in tool_names

    assert _classify_tool_tier("inspect_tactical_map_point") == ("TIER_1_SATYA_CORE", "satya-core")
    assert _classify_tool_tier("compute_token_reachable_zone") == ("TIER_1_SATYA_CORE", "satya-core")
    assert _classify_tool_tier("edit_tactical_map_feature") == ("TIER_1_SATYA_CORE", "satya-core")
    assert _classify_tool_tier("evaluate_ballistic_shot_on_map") == (
        "TIER_2_RULESET_ENGINE",
        "satya-d20-rules",
    )

    reset_awakened_systems_state()
    sid = f"wave5-blackout-{uuid.uuid4().hex[:6]}"
    generate_tactical_map.invoke(
        {
            "session_id": sid,
            "theme": "Corporate Server Bunker",
            "width": 20,
            "height": 16,
            "seed": 111,
        }
    )
    # Drive district strain over blackout threshold using repeated heavy unmasked draw.
    step_res: dict[str, object] = {}
    for _ in range(8):
        step_res = manage_polycrisis_grid_and_strain.invoke(
            {"power_mode": "UNMASKED_FULL_GRID_DRAW", "shard_draw_kw": 95, "session_id": sid}
        )
        if step_res.get("updated_state", {}).get("blackout_active"):  # type: ignore[union-attr]
            break

    state = get_session_polycrisis_state(sid)
    assert state["blackout_active"] is True
    assert step_res["map_lighting_updated"] == "darkness"
    cur_map = get_session_map(sid)
    assert isinstance(cur_map, dict)
    assert cur_map["lighting"] == "darkness"
    assert int(cur_map["ambient_light_delta_ext"]) >= 3
    assert all(n["powered"] is False for n in cur_map.get("surveillance_nodes", []))
    reset_awakened_systems_state()


def test_ai_player_inspect_surroundings_footing_and_surveillance_visibility() -> None:
    reset_player_session_context()
    sid = f"wave5-player-{uuid.uuid4().hex[:6]}"
    uid = "ai-player-mateo"
    try:
        generated = generate_tactical_map.invoke(
            {
                "session_id": sid,
                "theme": "Corporate Server Bunker",
                "width": 20,
                "height": 16,
                "seed": 77,
            }
        )
        map_with_mixed_cams = dict(generated)
        fz = map_with_mixed_cams["faraday_zones"][0]
        fz_x_cm = (fz["x0_cm"] + fz["x1_cm"]) // 2
        fz_y_cm = (fz["y0_cm"] + fz["y1_cm"]) // 2
        map_with_mixed_cams["surveillance_nodes"] = [
            {
                "node_id": "cam-visible",
                "label": "Lobby Security Dome",
                "node_kind_code": "DOME_CAMERA",
                "floor_index": 0,
                "x_cm": 350,
                "y_cm": 450,
                "facing_deg": 0,
                "fov_deg": 90,
                "range_cm": 700,
                "powered": True,
                "visibility": "PUBLIC",
            },
            {
                "node_id": "cam-hidden-gm",
                "label": "SECRET_COVERT_PINHOLE_99",
                "node_kind_code": "COVERT_MIC",
                "floor_index": 0,
                "x_cm": 450,
                "y_cm": 450,
                "facing_deg": 0,
                "fov_deg": 90,
                "range_cm": 700,
                "powered": True,
                "visibility": "GM_ONLY",
            },
        ]
        set_session_map(sid, map_with_mixed_cams)
        set_player_session_context(
            sid,
            uid,
            {
                "character_sheet": {"character_name": "Mateo Vance", "entity_id": uid},
                "active_map": map_with_mixed_cams,
            },
        )

        obs = inspect_surroundings.invoke(
            {
                "x_m": fz_x_cm / 100.0,
                "y_m": fz_y_cm / 100.0,
                "session_id": sid,
                "player_user_id": uid,
            }
        )
        footing = obs["footing_and_environment"]
        assert footing["in_faraday_zone"] is True
        assert footing["acoustic_surface"] is not None
        assert footing["traction_tier"] is not None

        vis_nodes = obs["visible_surveillance_nodes"]
        vis_ids = {n["node_id"] for n in vis_nodes}
        assert "cam-visible" in vis_ids
        assert "cam-hidden-gm" not in vis_ids
        assert "SECRET_COVERT_PINHOLE_99" not in str(obs)
    finally:
        reset_player_session_context()
