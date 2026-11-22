"""Rust <-> Python `satya-schema` DTO, Avro v2, TypedDict, Pydantic, and wire-enum contract tests."""

from __future__ import annotations

import json
import uuid
from typing import Any, get_args

from satya_ml.ai_gm.maps import (
    _NON_SOLID_PROP_KINDS,
    SectorPresetId,
    _build_rust_map_payload,
)
from satya_ml.ai_gm.server import AiGmTurnEventPayload
from satya_ml.ai_gm.tools import get_session_combat_tracker, reset_session_npcs
from satya_py import _native, campaign, schema


def test_character_sheet_and_nested_typed_dicts_match_rust_and_avro_v2() -> None:
    reg = schema.SchemaRegistry()

    # 1. CharacterSheetRecord (41 fields across TypedDict, Rust Default, and Avro v2)
    rust_char_fields = set(schema.schema_default_record("CharacterSheetRecord").keys())
    avro_char_fields = set(reg.extract_record_fields("character-sheets", 2, "CharacterSheetRecord"))
    py_char_fields = set(schema.CharacterSheetRecordDict.__annotations__.keys())
    assert len(rust_char_fields) == 41
    assert py_char_fields == rust_char_fields == avro_char_fields

    # 2. CharacterCharacteristicsRecord (32 fields)
    rust_stat_fields = set(schema.schema_default_record("CharacterCharacteristicsRecord").keys())
    avro_stat_fields = set(
        reg.extract_record_fields("character-sheets", 2, "CharacterCharacteristicsRecord")
    )
    py_stat_fields = set(schema.CharacterCharacteristicsRecordDict.__annotations__.keys())
    assert len(rust_stat_fields) == 32
    assert py_stat_fields == rust_stat_fields == avro_stat_fields

    # 3. CharacterAptitudesRecord (9 fields)
    rust_apt_fields = set(schema.schema_default_record("CharacterAptitudesRecord").keys())
    avro_apt_fields = set(
        reg.extract_record_fields("character-sheets", 2, "CharacterAptitudesRecord")
    )
    py_apt_fields = set(schema.CharacterAptitudesRecordDict.__annotations__.keys())
    assert len(rust_apt_fields) == 9
    assert py_apt_fields == rust_apt_fields == avro_apt_fields

    # 4. AnatomicalZoneRecord (4 fields)
    zone = schema.validate_schema_dto(
        "AnatomicalZoneRecord",
        {"location_code": "TORSO", "wound_tier": 1, "bone_status": "INTACT", "bleed_grade": 0},
    )
    avro_zone_fields = set(reg.extract_record_fields("character-sheets", 2, "AnatomicalZoneRecord"))
    py_zone_fields = set(schema.AnatomicalZoneRecordDict.__annotations__.keys())
    assert len(zone) == 4
    assert py_zone_fields == set(zone.keys()) == avro_zone_fields

    # 5. CharacterAssetRef (7 fields)
    asset = schema.validate_schema_dto(
        "CharacterAssetRef",
        {
            "asset_id": "asset-1",
            "asset_kind": "PORTRAIT_IMAGE",
            "filename": "portrait.png",
            "mime_type": "image/png",
            "sha256_hex": "00" * 32,
            "size_bytes": 1024,
            "created_at_us": 123456,
        },
    )
    avro_asset_fields = set(reg.extract_record_fields("character-sheets", 2, "CharacterAssetRef"))
    py_asset_fields = set(schema.CharacterAssetRefDict.__annotations__.keys())
    assert len(asset) == 7
    assert py_asset_fields == set(asset.keys()) == avro_asset_fields


def test_normalize_character_sheet_record_aliases_defaults_and_roundtrip() -> None:
    empty_norm = schema.normalize_character_sheet_record({}, index=4)
    assert empty_norm["character_id"] == "pc-4"
    assert empty_norm["owner_user_id"] == "pc-4"
    assert empty_norm["character_name"] == "pc-4"
    assert empty_norm["granted_access_tags"] == ["UNIVERSAL"]
    assert empty_norm["verification_status"] == "VERIFIED_OFFICIAL"
    assert empty_norm["sp_budget_valid"] is True
    assert empty_norm["dag_prereqs_valid"] is True
    assert len(empty_norm["characteristics"]) == 32
    assert len(empty_norm["aptitudes"]) == 9
    assert schema.validate_schema_dto("CharacterSheetRecord", empty_norm) == empty_norm

    aliased = schema.normalize_character_sheet_record(
        {
            "character_id": "pc-alias",
            "characteristics": {"for": 13, "str": 15, "mass_kg": 68.5, "trs": 4, "cps": 17},
            "aptitudes": {"apt_qnt": 2},
            "fatigue_tier": 2,
            "wound_shock_tier": 1,
            "youth_hobby": "CITIZEN_OSINT_DEEPFAKE",
            "faction_id": "",
            "faction_archetype": "SATYANG_MANJUSRI",
            "notes": None,
        },
        index=1,
    )
    assert aliased["characteristics"]["for_stat"] == 15
    assert aliased["characteristics"]["mas_kg"] == 68.5
    assert aliased["characteristics"]["trs"] == 4
    assert aliased["characteristics"]["cps"] == 17
    assert aliased["aptitudes"]["apt_qnt"] == 2
    assert aliased["fatigue_level"] == 2
    assert aliased["wound_shock_index"] == 1
    assert aliased["faction_id"] is None
    assert aliased["faction_archetype"] == "SATYANG_MANJUSRI"
    assert "CITIZEN_OSINT_DEEPFAKE" in str(aliased["lifepath_config_json"])
    assert schema.validate_schema_dto("CharacterSheetRecord", aliased) == aliased

    with_obj_lp = schema.normalize_character_sheet_record(
        {"lifepath_config": {"career_terms": ["CLINICAL_MEDIC"]}},
        index=9,
    )
    assert json.loads(str(with_obj_lp["lifepath_config_json"])) == {"career_terms": ["CLINICAL_MEDIC"]}
    assert schema.validate_schema_dto("CharacterSheetRecord", with_obj_lp) == with_obj_lp


def test_ai_gm_turn_trigger_request_matches_fastapi_payload() -> None:
    rust_payload = schema.schema_default_record("AiGmTurnTriggerRequest")
    assert set(rust_payload.keys()).issubset(set(AiGmTurnEventPayload.model_fields.keys()))
    validated = AiGmTurnEventPayload.model_validate(rust_payload)
    assert validated.session_id == rust_payload["session_id"]
    assert validated.trigger_event_id == rust_payload["trigger_event_id"]


def test_outgoing_python_http_dtos_validate_against_rust_satya_schema() -> None:
    sid = f"schema-contract-{uuid.uuid4().hex[:8]}"
    try:
        # 1. GmMapUpdateRequest built from Python map state
        map_payload = _build_rust_map_payload(
            {
                "map_id": "map-contract",
                "theme": "Community Clinic Annex",
                "grid_width": 14,
                "grid_height": 12,
                "active_floor": 0,
                "ground_floor_override": "clinical_vinyl",
                "ground_tint_rgb": [210, 220, 225],
                "floors": [
                    {"floor_index": 0, "label": "1F", "wall_edges": [], "props": [], "surfaces": []},
                    {"floor_index": 1, "label": "2F", "wall_edges": [], "props": [], "surfaces": []},
                ],
                "faraday_zones": [
                    {
                        "zone_id": "fz-1",
                        "label": "Cold Vault",
                        "floor_index": 0,
                        "x0_cm": 100,
                        "y0_cm": 100,
                        "x1_cm": 400,
                        "y1_cm": 400,
                    }
                ],
                "surveillance_nodes": [
                    {
                        "node_id": "cam-1",
                        "label": "Triage Cam",
                        "node_kind_code": "DOME_CAMERA",
                        "floor_index": 0,
                        "x_cm": 500,
                        "y_cm": 200,
                    }
                ],
                "tokens": [
                    {
                        "token_id": "tok-spy",
                        "label": "Shadow Agent",
                        "pos_x_cm": 325,
                        "pos_y_cm": 475,
                        "floor_index": 1,
                        "visibility": "GM_ONLY",
                    }
                ],
            }
        )
        validated_map = schema.validate_schema_dto("GmMapUpdateRequest", map_payload)
        assert validated_map["tokens"][0]["visibility"] == "GM_ONLY"
        assert validated_map["ground_floor_override"] == "clinical_vinyl"
        assert validated_map["ground_tint_rgb"] == [210, 220, 225]
        assert len(validated_map["faraday_zones"]) == 1
        assert len(validated_map["surveillance_nodes"]) == 1

        # 2. CombatTrackerState offline seed
        tracker = get_session_combat_tracker(sid)
        assert schema.validate_schema_dto("CombatTrackerState", tracker)["session_id"] == sid

        # 3. GmRollRequest, GmChatRequest, UpsertGmMemoryRequest, UpdateCombatTrackerRequest
        schema.validate_schema_dto(
            "GmRollRequest",
            {
                "actor_display_name": "SecOps Sentry",
                "target_user_ids": [],
                "skill_name": "firearms_pistols",
                "attribute_tn": 12,
                "pool_size": 3,
                "difficulty_rating": 2,
                "internal_mod": 0,
                "carry_mode": "markov_step",
                "combat_stance": "RANGED_SNAP",
                "combat_technique": "snap_shot",
                "tempo_pool": 4,
                "control_pool": 4,
                "force_pool": 2,
                "fatigue_level": 0,
                "wound_shock_index": 0,
                "notes": "Snap shot",
            },
        )
        schema.validate_schema_dto(
            "GmChatRequest",
            {
                "visibility": "PUBLIC",
                "recipient_user_ids": [],
                "channel_context": "GM_NARRATION",
                "body": "Rain hammers the clinic awning.",
            },
        )
        schema.validate_schema_dto(
            "UpsertGmMemoryRequest",
            {
                "category": "SCENE_STATE",
                "title": "Vault Breaker",
                "content": "Sub-basement relay tripped.",
                "is_pinned": True,
                "is_persistent": True,
            },
        )
        schema.validate_schema_dto(
            "UpdateCombatTrackerRequest",
            {
                "tick_delta": 2,
                "combat_active": True,
                "entity_id": "pc-1",
                "active_stance": "STABILITY",
                "next_action_tick": 6,
                "grid_x": 4,
                "grid_y": 5,
            },
        )

        # 4. World ledger + NPC requests
        ledger_resp = campaign.seed_or_update_world_ledger(sid, pack_id="awakened")
        schema.validate_schema_dto("SessionWorldLedger", ledger_resp["ledger"])
        schema.validate_schema_dto(
            "UpdateWorldLedgerRequest",
            {"heat_delta": 1, "faction_id": "satyang", "clock_delta": 1, "reason": "Relay signal"},
        )
        schema.validate_schema_dto(
            "UpdateSessionNpcRequest",
            {"disposition": "guarded", "apparent_condition": "Fatigued"},
        )
        schema.validate_schema_dto(
            "GenerateNpcRequest",
            {"prompt": "Clinic triage nurse", "career_terms": 2, "is_persistent": False},
        )

        # 5. Player AI requests
        schema.validate_schema_dto(
            "PlayerAiChatRequest",
            {
                "user_id": "ai-player-1",
                "display_name": "Mateo Vance (AI Player)",
                "visibility": "PUBLIC",
                "recipient_user_ids": [],
                "channel_context": "IN_CHARACTER",
                "body": "Checking the circuit breaker.",
                "auto_trigger_gm": True,
            },
        )
        schema.validate_schema_dto(
            "PlayerAiRollRequest",
            {
                "user_id": "ai-player-1",
                "actor_display_name": "Mateo Vance",
                "check_category": "electronics_repair",
                "attribute_tn": 13,
                "skill_rank": 3,
                "difficulty_mod": 0,
                "internal_mod": 0,
                "notes": "Inspect relay",
            },
        )
        schema.validate_schema_dto(
            "PlayerAiDoorRequest",
            {"user_id": "ai-player-1", "cell_x": 4, "cell_y": 5, "side": "NORTH"},
        )
    finally:
        reset_session_npcs(sid)


def test_wire_enum_and_prop_catalog_parity() -> None:
    assert set(get_args(SectorPresetId.__value__)) == set(schema.list_wire_enum_codes("SectorPreset"))
    assert _NON_SOLID_PROP_KINDS == set(schema.list_wire_enum_codes("NonSolidPropKind"))
    assert "DRONE_DOCK_PAD" in _NON_SOLID_PROP_KINDS

    for enum_name in (
        "PropKind",
        "MaterialClass",
        "WallEdgeKind",
        "EdgeSide",
        "TacticalTileKind",
        "CoverGrade",
        "MapScalePreset",
        "EconomicTier",
        "ZoningContext",
        "StoreArchetype",
        "SurfaceFeatureKind",
        "MapContinuationDir",
        "SectorPreset",
        "NonSolidPropKind",
    ):
        codes = schema.list_wire_enum_codes(enum_name)
        assert codes and all(isinstance(c, str) and c for c in codes), enum_name


def test_native_ffi_accepts_partial_character_dicts_without_python_prenormalization() -> None:
    partial_char: dict[str, Any] = {
        "character_name": "Linnea Partial",
        "characteristics": {"for": 12, "trs": 4, "cps": 17, "sig": 16},
        "youth_hobby": "CITIZEN_OSINT_DEEPFAKE",
        "faction_archetype": "SATYANG_MANJUSRI",
    }

    hooks = _native.compile_party_motivational_hooks([partial_char], None, "awakened")
    assert "pc-0" in hooks["character_profiles"]
    assert hooks["hooks"]

    anchors = _native.extract_character_lifepath_anchors(partial_char, 3)
    assert anchors["faction_id"] == "SATYANG"
    assert anchors["faction_archetype"] == "SATYANG_MANJUSRI"
    assert any(h["hobby_code"] == "CITIZEN_OSINT_DEEPFAKE" for h in anchors["hobby_contacts"])

    sector = _native.generate_procedural_sector("contract-sec", "hospital", "Clinic", seed=17)
    whispers = _native.evaluate_passive_perceptions(sector, "pc-0", [partial_char], None)
    assert isinstance(whispers, list)


def test_python_strenums_match_rust_wire_enum_registry() -> None:
    from satya_py import enums

    assert {m.value for m in enums.WireEnumName} == set(schema.list_wire_enum_names())
    assert {m.value for m in enums.SchemaDtoName} == set(schema.list_schema_dto_names())

    for rust_enum_name, py_enum_cls in enums.RUST_ENUM_PARITY_MAP.items():
        rust_codes = schema.list_wire_enum_codes(rust_enum_name)
        py_codes = [member.value for member in py_enum_cls]
        assert py_codes == rust_codes, (
            f"Enum drift for {rust_enum_name}: py={py_codes} vs rust={rust_codes}"
        )


def test_telemetry_trajectory_and_offline_combat_dtos_validate_against_rust(
    monkeypatch: Any,
) -> None:
    from satya_ml.ai_gm.telemetry import (
        build_tool_invocation_record,
        build_trajectory_report_payload,
    )
    from satya_ml.ai_gm.tools import advance_combat_tracker, reset_session_npcs
    from satya_py.enums import LogLevel, SchemaDtoName

    tool_rec = build_tool_invocation_record(
        {
            "mcp_tier": "TIER_1_SATYA_CORE",
            "server_id": "satya-core",
            "tool_name": "advance_combat_tracker",
            "arguments_json": '{"tick_delta": 1}',
            "result_summary_json": '{"tick": 2}',
            "is_error": False,
            "duration_ms": 4,
        },
        step_index=0,
    )
    schema.validate_schema_dto(SchemaDtoName.MCP_TOOL_INVOCATION_RECORD, tool_rec)

    log_entry = {
        "level": LogLevel.INFO.value,
        "span_id": "0123456789abcdef",
        "parent_span_id": None,
        "lineage_id": "lin-contract-1",
        "parent_lineage_ids": [],
        "log_line": "[session=contract] telemetry contract verified",
    }
    schema.validate_schema_dto(SchemaDtoName.GM_SESSION_LOG_ENTRY, log_entry)

    traj = build_trajectory_report_payload(
        operating_mode="autonomous",
        trigger_actor={
            "user_id": "u-contract",
            "username": "contract",
            "display_name": "Contract Tester",
            "participant_kind": "HUMAN",
            "role": "PLAYER",
            "character_id": None,
            "ai_agent_info": None,
        },
        trigger_event_id="ev-contract-1",
        trigger_summary="Contract test turn",
        llm_provider="ollama",
        reasoning_model_id="gemma4:12b",
        vision_model_id="gemma4:12b",
        tool_invocations=[tool_rec],
        rag_tool_names={"query_satya_rules"},
        emitted_event_ids=["ev-out-1"],
        total_turn_latency_ms=12,
        turn_logs=[log_entry],
    )
    schema.validate_schema_dto(SchemaDtoName.GM_TRAJECTORY_REPORT_REQUEST, traj)

    move_req = {
        "entity_id": "pc-1",
        "label": "Mateo",
        "to_x": 4,
        "to_y": 5,
        "to_x_cm": 450,
        "to_y_cm": 550,
    }
    schema.validate_schema_dto(SchemaDtoName.MOVE_MAP_TOKEN_REQUEST, move_req)

    sid = f"combat-contract-{uuid.uuid4().hex[:8]}"
    monkeypatch.setattr("satya_ml.ai_gm.tools._signaling_base_url", lambda: "")
    try:
        tracker = advance_combat_tracker.invoke(
            {"tick_delta": 1, "entity_id": "npc-contract", "session_id": sid}
        )
        validated_tracker = schema.validate_schema_dto(
            SchemaDtoName.COMBAT_TRACKER_STATE, tracker
        )
        assert any(c["entity_id"] == "npc-contract" for c in validated_tracker["combatants"])
        assert all(c["active_stance"] == "STABILITY" for c in validated_tracker["combatants"])
    finally:
        reset_session_npcs(sid)


def test_build_rules_book_triad_evaluator_matches_rust_native() -> None:
    from satya_ml.build_rules_book import evaluate_tactical_triad_python
    from satya_py import combat

    env_obj = combat.compute_tactical_pool(
        skill_c=14, skill_n=4, morale_state="steady", ana=13, cre=12, foc=12
    )
    native_eval = env_obj.evaluate_triad(base_weapon_tc=8, tempo=7, control=6, force=5)
    py_eval = evaluate_tactical_triad_python(
        env_obj.to_dict(), base_weapon_tc=8, tempo=7, control=6, force=5
    )

    assert py_eval["valid"] == native_eval.valid
    assert py_eval["effective_tc"] == native_eval.effective_tc
    assert py_eval["strike_impact"] == native_eval.strike_impact
    assert py_eval["effective_root"] == native_eval.effective_root
    assert py_eval["exertion_marks_gained"] == native_eval.exertion_marks_gained
    assert py_eval["archetype_label"] == native_eval.archetype_label


def test_battlemap_2d_texture_catalog_matches_rust_wasm_registry() -> None:
    import re
    from pathlib import Path
    from satya_ml.texture_gen.catalog import BATTLEMAP_2D_TEXTURE_CATALOG

    repo_root = Path(__file__).resolve().parent.parent
    rust_src = (
        repo_root / "crates" / "satya-web" / "src" / "wasm_client" / "map_2d_art_textures.rs"
    ).read_text(encoding="utf-8")
    rust_files = re.findall(r'asset_file:\s*"([^"]+)"', rust_src)
    py_files = [spec.filename for spec in BATTLEMAP_2D_TEXTURE_CATALOG]

    assert len(py_files) == 50
    assert len(rust_files) == 50
    assert set(py_files) == set(rust_files)
