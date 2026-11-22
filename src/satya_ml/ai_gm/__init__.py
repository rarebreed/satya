"""Satya AI Game Master (AI GM) LangGraph + Multi-Provider + MCP agent package."""

from __future__ import annotations

from satya_ml.ai_gm.awakened_systems import (
    intervene_ai_cage_or_hunter_ai,
    manage_polycrisis_grid_and_strain,
    manage_synthetic_substrate_and_embodiment,
    query_engine_and_rules_rag,
    reset_awakened_systems_state,
    transmit_via_tradecraft_channel,
    update_campaign_arc_and_conspiracies,
)
from satya_ml.ai_gm.audit_and_first_contact import (
    inspect_synthetic_or_proxy_telemetry,
    verify_forensic_audit_proof,
)
from satya_ml.ai_gm.maps import (
    continue_tactical_map,
    generate_tactical_map,
    get_session_map,
    inspect_tactical_map,
    move_map_token,
    register_spatial_clue_pin,
    render_tactical_svg,
    set_session_map,
    switch_tactical_map_floor,
)
from satya_ml.ai_gm.maps_tactical import (
    compute_token_reachable_zone,
    edit_tactical_map_feature,
    evaluate_ballistic_shot_on_map,
    inspect_tactical_map_point,
    sync_map_blackout_from_polycrisis,
)
from satya_ml.ai_gm.models import SessionLlmConfig, build_gm_chat_model, redact_api_key
from satya_ml.ai_gm.npc_psyche_combat import evaluate_npc_morale_or_trigger
from satya_ml.ai_gm.tech_and_tradecraft import (
    evaluate_part5_clamping_or_political_economy,
    reset_tech_and_tradecraft_state,
    resolve_contact_cover_or_safehouse,
    resolve_ew_drone_or_engineering,
    resolve_explosive_vehicle_or_hazard,
    resolve_stealth_surveillance_or_forensics,
)
from satya_ml.ai_gm.party_coordinator import (
    evaluate_party_turn_gate,
    execute_ready_npc_actions,
    extract_active_human_players,
)
from satya_ml.ai_gm.tool_registry import (
    CORE_GM_TOOL_NAMES,
    DEFERRED_TOOL_PACKS,
    GmToolRegistry,
    OOC_READONLY_TOOL_NAMES,
    load_gm_tools,
)
from satya_ml.ai_gm.tools import (
    delete_gm_memory,
    get_persisted_session_npcs,
    load_campaign_npc,
)

__all__ = [
    "CORE_GM_TOOL_NAMES",
    "DEFERRED_TOOL_PACKS",
    "GmToolRegistry",
    "OOC_READONLY_TOOL_NAMES",
    "SessionLlmConfig",
    "build_gm_chat_model",
    "compute_token_reachable_zone",
    "continue_tactical_map",
    "delete_gm_memory",
    "edit_tactical_map_feature",
    "evaluate_ballistic_shot_on_map",
    "evaluate_npc_morale_or_trigger",
    "evaluate_part5_clamping_or_political_economy",
    "evaluate_party_turn_gate",
    "execute_ready_npc_actions",
    "extract_active_human_players",
    "generate_tactical_map",
    "get_persisted_session_npcs",
    "get_session_map",
    "inspect_synthetic_or_proxy_telemetry",
    "inspect_tactical_map",
    "inspect_tactical_map_point",
    "intervene_ai_cage_or_hunter_ai",
    "load_campaign_npc",
    "load_gm_tools",
    "manage_polycrisis_grid_and_strain",
    "manage_synthetic_substrate_and_embodiment",
    "move_map_token",
    "query_engine_and_rules_rag",
    "redact_api_key",
    "register_spatial_clue_pin",
    "render_tactical_svg",
    "reset_awakened_systems_state",
    "reset_tech_and_tradecraft_state",
    "resolve_contact_cover_or_safehouse",
    "resolve_ew_drone_or_engineering",
    "resolve_explosive_vehicle_or_hazard",
    "resolve_stealth_surveillance_or_forensics",
    "set_session_map",
    "switch_tactical_map_floor",
    "sync_map_blackout_from_polycrisis",
    "transmit_via_tradecraft_channel",
    "update_campaign_arc_and_conspiracies",
    "verify_forensic_audit_proof",
]
