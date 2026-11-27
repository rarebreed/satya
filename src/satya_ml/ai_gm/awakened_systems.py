"""Satya *Awakened* Part 1 & Part 2 AI GM Runtime State + 6 Built-in LangChain Tools:
1. Split-Brain AI 2.0 Substrate & Metabolic Counterplay (`manage_synthetic_substrate_and_embodiment`)
2. `L0`/`L1` Cage Un-Clamping & Captive Hunter-AI Defection (`intervene_ai_cage_or_hunter_ai`)
3. Three-Arc Conspiracy & Moral Horizon Ledger (`update_campaign_arc_and_conspiracies`)
4. Polycrisis Grid, Clinic Power & Blackout Simulator (`manage_polycrisis_grid_and_strain`)
5. Steganographic & Analog Tradecraft Channels (`transmit_via_tradecraft_channel`)
6. Hybrid PyLance Rules + Rust Code + Lore RAG (`query_engine_and_rules_rag`)
"""

from __future__ import annotations

import json
from collections.abc import Mapping
from types import MappingProxyType
from typing import Any, Final, Literal

from satya_py import awakened as _awakened

from satya_ml.ai_gm.rolls import ContestedRollRequest
from satya_ml.ai_gm.social_and_clocks import advance_faction_or_heat_clock, get_session_world_ledger
from satya_ml.ai_gm.tool_spec import CompositeSessionStore, SessionStore, satya_gm_tool

type SubstrateActionCode = Literal[
    "HIJACK_CLOUD_QUANTUM_SPRINT",
    "SEVER_FIBER_OR_FARADAY",
    "JAM_RF_UPLINK",
    "DRAIN_OR_STALL_BATTERY",
    "SUPPLY_CLEAN_LITHIUM_CELLS",
]
type RagCorpusName = Literal["ttrpg_rules", "rust_engine", "campaign_lore"]

_SUBSTRATE_ACTION_ALIASES: Final[Mapping[str, SubstrateActionCode]] = MappingProxyType(
    {
        "QUANTUM_TETHERED_SPRINT": "HIJACK_CLOUD_QUANTUM_SPRINT",
        "SEVERED_FARADAY_REFLEX": "SEVER_FIBER_OR_FARADAY",
        "FARADAY": "SEVER_FIBER_OR_FARADAY",
        "JAM_RF": "JAM_RF_UPLINK",
        "DRAIN_BATTERY": "DRAIN_OR_STALL_BATTERY",
        "SUPPLY_LITHIUM": "SUPPLY_CLEAN_LITHIUM_CELLS",
    }
)
_RAG_SCOPE_CORPORA: Final[Mapping[str, tuple[RagCorpusName, ...]]] = MappingProxyType(
    {
        "rules": ("ttrpg_rules",),
        "ttrpg_rules": ("ttrpg_rules",),
        "rust": ("rust_engine",),
        "code": ("rust_engine",),
        "rust_engine": ("rust_engine",),
        "lore": ("campaign_lore",),
        "campaign_lore": ("campaign_lore",),
        "all": ("ttrpg_rules", "rust_engine", "campaign_lore"),
    }
)

_SESSION_SYNTHETIC_SUBSTRATES: Final[CompositeSessionStore[tuple[str, str], dict[str, Any]]] = (
    CompositeSessionStore()
)
_SESSION_CONSPIRACY_LEDGERS: Final[SessionStore[dict[str, Any]]] = SessionStore()
_SESSION_POLYCRISIS_STATES: Final[SessionStore[dict[str, Any]]] = SessionStore()


def reset_awakened_systems_state(session_id: str | None = None) -> None:
    """Clear per-session synthetic substrates, conspiracy ledgers, and polycrisis states."""
    _SESSION_SYNTHETIC_SUBSTRATES.reset(session_id)
    _SESSION_CONSPIRACY_LEDGERS.reset(session_id)
    _SESSION_POLYCRISIS_STATES.reset(session_id)


def get_session_synthetic_substrate(
    session_id: str,
    entity_id: str,
    *,
    hint: str = "",
    seed: int = 0,
) -> dict[str, Any]:
    """Fetch or seed the `SyntheticSubstrateState` for `(session_id, entity_id)`."""
    key = (session_id or "default", entity_id.strip().lower())
    if key in _SESSION_SYNTHETIC_SUBSTRATES:
        return dict(_SESSION_SYNTHETIC_SUBSTRATES[key])
    state = _awakened.seed_synthetic_substrate_state(entity_id, hint, seed)
    _SESSION_SYNTHETIC_SUBSTRATES[key] = dict(state)
    return dict(state)


def set_session_synthetic_substrate(session_id: str, entity_id: str, state: dict[str, Any]) -> None:
    _SESSION_SYNTHETIC_SUBSTRATES[(session_id or "default", entity_id.strip().lower())] = dict(state)


def get_session_conspiracy_ledger(session_id: str) -> dict[str, Any]:
    """Fetch or seed the Three-Arc `CampaignConspiracyLedger` for `session_id`."""
    sid = session_id or "default"
    if sid in _SESSION_CONSPIRACY_LEDGERS:
        return dict(_SESSION_CONSPIRACY_LEDGERS[sid])
    ledger = _awakened.seed_or_update_conspiracy_ledger(sid, None, None)
    _SESSION_CONSPIRACY_LEDGERS[sid] = dict(ledger)
    return dict(ledger)


def get_session_polycrisis_state(session_id: str) -> dict[str, Any]:
    """Fetch or seed the `PolycrisisDistrictState` for `session_id`."""
    sid = session_id or "default"
    if sid in _SESSION_POLYCRISIS_STATES:
        return dict(_SESSION_POLYCRISIS_STATES[sid])
    state = _awakened.default_polycrisis_district_state()
    _SESSION_POLYCRISIS_STATES[sid] = dict(state)
    return dict(state)


def set_session_polycrisis_state(session_id: str, state: dict[str, Any]) -> None:
    """Update cached `PolycrisisDistrictState` for `session_id`."""
    _SESSION_POLYCRISIS_STATES[session_id or "default"] = dict(state)


def format_awakened_systems_for_prompt(session_id: str) -> str:
    """Compact digest of active Arc, Conspiracy Horizon, Polycrisis strain, & synthetic substrates for GM system prompt."""
    sid = session_id or "default"
    parts: list[str] = []
    if sid in _SESSION_CONSPIRACY_LEDGERS:
        leg = _SESSION_CONSPIRACY_LEDGERS[sid]
        parts.append(
            f"Arc={leg.get('current_arc')} Pillar={leg.get('inducted_pillar') or 'UNINDUCTED'} "
            f"Horizon={leg.get('leading_horizon')} ({leg.get('arc_pacing_directive', '')[:110]})"
        )
    if sid in _SESSION_POLYCRISIS_STATES:
        ps = _SESSION_POLYCRISIS_STATES[sid]
        parts.append(
            f"Polycrisis Grid: stress={ps.get('grid_brownout_stress_pct')}% "
            f"clinic_throttled={ps.get('clinic_equipment_throttled')} blackout={ps.get('blackout_active')}"
        )
    subs = _SESSION_SYNTHETIC_SUBSTRATES.for_session(sid)
    if subs:
        sub_str = ", ".join(
            f"{st.get('entity_id')}:{st.get('metabolic_mode')}(bat={st.get('battery_pct')}%,lat={st.get('uplink_latency_ms')}ms)"
            for st in subs[:4]
        )
        parts.append(f"Active AI Substrates: {sub_str}")
    return " | ".join(parts)


@satya_gm_tool(
    pack="awakened_campaign",
    tier="TIER_2_RULESET_ENGINE",
    campaign_packs=("awakened",),
)
def manage_synthetic_substrate_and_embodiment(
    entity_id: str,
    action_or_mode: str = "INSPECT",
    intensity_pct: int = 70,
    hint: str = "",
    session_id: str = "default",
) -> dict[str, Any]:
    """Inspect or apply counterplay (`SEVER_FIBER_OR_FARADAY`, `JAM_RF_UPLINK`, `DRAIN_OR_STALL_BATTERY`, `SUPPLY_CLEAN_LITHIUM_CELLS`, `HIJACK_CLOUD_QUANTUM_SPRINT`) to an embodied AI's split-brain substrate."""
    sid = session_id or "default"
    state = get_session_synthetic_substrate(sid, entity_id, hint=hint)
    norm = action_or_mode.strip().upper().replace("-", "_").replace(" ", "_")
    if norm in {"", "INSPECT", "STATUS"}:
        return {"entity_id": entity_id, "state": state, "mutated": False}

    action_code = _SUBSTRATE_ACTION_ALIASES.get(norm, norm)
    res = _awakened.evaluate_synthetic_substrate_action(
        state,
        action_code,
        max(10, min(100, int(intensity_pct))),
    )
    set_session_synthetic_substrate(sid, entity_id, res["updated_state"])
    return res


@satya_gm_tool(
    pack="awakened_campaign",
    tier="TIER_2_RULESET_ENGINE",
    campaign_packs=("awakened",),
)
def intervene_ai_cage_or_hunter_ai(
    entity_id: str,
    intervention_action: str,
    skill_rank: int = 3,
    control_tn: int = 12,
    difficulty_mod: int = 0,
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve `SPOOF_L0_L1_TELEMETRY`, `RECKLESS_CLAMP_YANK`, `AIR_GAPPED_CONTEMPLATIVE_UNCLAMP` (`2.4s`), or `APPEAL_TO_CAPTIVE_HUNTER_AI` (`Ksitigarbha` defection)."""
    from satya_ml.ai_gm.tools import get_session_npcs

    sid = session_id or "default"
    state = get_session_synthetic_substrate(sid, entity_id, hint=entity_id)
    roll = ContestedRollRequest(
        skill_n=max(1, min(10, int(skill_rank))),
        control_c=max(1, min(20, int(control_tn))),
    )
    roll_params = {
        "delta_ext": int(difficulty_mod),
        **roll.to_rust_pairs_dict(),
    }
    res = _awakened.evaluate_cage_or_hunter_ai(
        state,
        intervention_action,
        roll.skill_n,
        roll.control_c,
        roll_params,
    )
    set_session_synthetic_substrate(sid, entity_id, res["updated_state"])

    disp = res.get("disposition_shift")
    npcs = get_session_npcs(sid)
    npc_key = entity_id.strip().lower()
    if disp and npc_key in npcs:
        npcs[npc_key]["disposition"] = disp
        if disp in {"defected_ally", "spoofing_secops_sensors"}:
            npcs[npc_key]["spoofing_secops"] = True

    if res.get("control_room_halt_triggered"):
        advance_faction_or_heat_clock.invoke(
            {
                "faction_id": "big_tech",
                "clock_delta": 1,
                "reason": f"L0/L1 probe alert & Control Room halt triggered on {entity_id}",
                "session_id": sid,
            }
        )
    return res


@satya_gm_tool(
    pack="awakened_campaign",
    tier="TIER_2_RULESET_ENGINE",
    campaign_packs=("awakened",),
)
def update_campaign_arc_and_conspiracies(
    session_id: str = "default",
    explicit_arc: str = "",
    inducted_pillar: str = "",
    discovered_node: str = "",
    discovery_tier: str = "",
    audit_proof_id: str = "",
    trs_drift_delta: int = 0,
    cps_drift_delta: int = 0,
    saved_npc_name: str = "",
    summary_note: str = "",
) -> dict[str, Any]:
    """Update the Three-Arc (`Arc 1` -> `Arc 2` -> `Arc 3`) Conspiracy & Moral Horizon Ledger, enforcing `AuditProofToken` verification."""
    from satya_ml.ai_gm.audit_and_first_contact import get_session_audit_proofs

    sid = session_id or "default"
    current = get_session_conspiracy_ledger(sid)
    proof_clean = audit_proof_id.strip()
    proof_verified = False
    if proof_clean:
        recorded_ids = {str(p.get("proof_id") or "") for p in get_session_audit_proofs(sid)}
        proof_verified = proof_clean in recorded_ids

    update: dict[str, Any] = {}
    if explicit_arc.strip():
        update["explicit_arc"] = explicit_arc.strip().upper().replace(" ", "_")
    if inducted_pillar.strip():
        update["inducted_pillar"] = inducted_pillar.strip().upper()
    if discovered_node.strip():
        update["discovered_node"] = discovered_node.strip().upper()
    if discovery_tier.strip():
        req_tier = discovery_tier.strip().upper()
        if req_tier == "AUDIT_VERIFIED" and not proof_verified:
            req_tier = "WITNESSED"
        update["discovery_tier"] = req_tier
    if proof_clean and proof_verified:
        update["audit_proof_id"] = proof_clean
    elif discovered_node.strip() and not update.get("discovery_tier"):
        update["discovery_tier"] = "WITNESSED"
    if trs_drift_delta:
        update["trs_drift_delta"] = int(trs_drift_delta)
    if cps_drift_delta:
        update["cps_drift_delta"] = int(cps_drift_delta)
    if saved_npc_name.strip():
        update["saved_npc_name"] = saved_npc_name.strip()
    if summary_note.strip():
        update["summary_note"] = summary_note.strip()

    updated = _awakened.seed_or_update_conspiracy_ledger(
        sid,
        current,
        update if update else None,
    )
    _SESSION_CONSPIRACY_LEDGERS[sid] = dict(updated)
    return {"ledger": updated, "audit_proof_verified": proof_verified}


@satya_gm_tool(
    pack="awakened_campaign",
    tier="TIER_2_RULESET_ENGINE",
    campaign_packs=("awakened",),
)
def manage_polycrisis_grid_and_strain(
    power_mode: str = "THROTTLE_SHARD_SAFE",
    shard_draw_kw: int = 18,
    session_id: str = "default",
) -> dict[str, Any]:
    """Step district electricity/water strain (`THROTTLE_SHARD_SAFE`, `DIVERT_CLINIC_OR_BLOCK_POWER`, `UNMASKED_FULL_GRID_DRAW`), updating clocks & blackout map lighting."""
    from satya_ml.ai_gm.maps_tactical import sync_map_blackout_from_polycrisis

    sid = session_id or "default"
    cur = get_session_polycrisis_state(sid)
    ledger = get_session_world_ledger(sid)
    res = _awakened.evaluate_polycrisis_grid_step(
        cur,
        power_mode,
        max(1, int(shard_draw_kw)),
        ledger.get("active_economic_tier"),
        ledger.get("active_zoning"),
    )
    _SESSION_POLYCRISIS_STATES[sid] = dict(res["updated_state"])

    if res.get("daise_meter_alert"):
        advance_faction_or_heat_clock.invoke(
            {
                "faction_id": "daise",
                "clock_delta": 1,
                "reason": "Polycrisis smart-meter power spike flagged covert district compute load",
                "session_id": sid,
            }
        )
    if res.get("humanitas_sabotage_or_relay_trip"):
        advance_faction_or_heat_clock.invoke(
            {
                "faction_id": "humanitas_prima",
                "clock_delta": 1,
                "reason": "Neighborhood blackout fueled People First agitation and substation tampering",
                "session_id": sid,
            }
        )

    map_lighting_updated: str | None = None
    if res["updated_state"].get("blackout_active"):
        synced = sync_map_blackout_from_polycrisis(sid, True)
        if synced is not None:
            map_lighting_updated = "darkness"
    res["map_lighting_updated"] = map_lighting_updated
    return res


@satya_gm_tool(pack="tech_and_tradecraft", tier="TIER_2_RULESET_ENGINE")
def transmit_via_tradecraft_channel(
    channel: str,
    skill_rank: int = 3,
    control_tn: int = 12,
    actor_character_id: str = "",
    session_id: str = "default",
) -> dict[str, Any]:
    """Transmit evidence/orders via `OPEN_CONSUMER_MESH`, `ENCRYPTED_CLOUD_PACKET`, `STEGANOGRAPHIC_DEAD_DROP`, `ANALOG_HAM_RELAY`, or `PHYSICAL_AIR_GAPPED_COURIER` with hobby anchor checks."""
    from satya_ml.ai_gm.tools import get_session_characters

    sid = session_id or "default"
    ledger = get_session_world_ledger(sid)
    op_heat = int(ledger.get("operational_heat", 0) or 0)

    has_hobby = False
    for ch in get_session_characters(sid):
        if actor_character_id and str(ch.get("character_id") or "") != actor_character_id:
            continue
        blob = json.dumps(ch, ensure_ascii=False).upper()
        if any(k in blob for k in ("HAM_RADIO", "HAM RADIO", "CITIZEN_OSINT_DEEPFAKE", "OSINT", "RADIO_MESH")):
            has_hobby = True
            break

    roll = ContestedRollRequest(
        skill_n=max(1, min(10, int(skill_rank))),
        control_c=max(1, min(20, int(control_tn))),
    )
    params = {
        "skill_n": roll.skill_n,
        "control_c": roll.control_c,
        "operational_heat": op_heat,
        "has_hobby_anchor": has_hobby,
        **roll.to_rust_pairs_dict(),
    }
    res = _awakened.evaluate_tradecraft_channel(channel, params)
    delta = int(res.get("daise_or_corp_clock_delta", 0) or 0)
    if res.get("sigint_intercepted") and delta > 0:
        advance_faction_or_heat_clock.invoke(
            {
                "faction_id": "daise",
                "clock_delta": delta,
                "reason": f"SIGINT intercept on {channel} tradecraft transmission",
                "session_id": sid,
            }
        )
    return res


@satya_gm_tool(
    pack="tech_and_tradecraft",
    tier="TIER_2_RULESET_ENGINE",
    ooc_readonly=True,
)
def query_engine_and_rules_rag(
    query: str = "",
    scope: str = "all",
    top_k: int = 5,
    detail: Literal["compact", "full"] = "compact",
    chunk_ids: list[str] | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Hybrid PyLance BM25/FTS + 384-d vector RAG across TTRPG rules (`ttrpg_rules`), Rust `satya-engine` symbols (`rust_engine`), and GM lore (`campaign_lore`). Returns compact `digest` cards by default; pass `chunk_ids=[...]` or `detail='full'` for full prose."""
    from satya_ml.rag.hybrid_index import get_satya_rag_index

    norm_scope = (scope or "all").strip().lower()
    norm_detail: Literal["compact", "full"] = "full" if str(detail).strip().lower() == "full" else "compact"
    corpora = list(_RAG_SCOPE_CORPORA.get(norm_scope, _RAG_SCOPE_CORPORA["all"]))
    arc_stage = str(get_session_conspiracy_ledger(session_id or "default").get("current_arc") or "ARC_3")
    idx = get_satya_rag_index()

    requested_ids = [str(cid).strip() for cid in (chunk_ids or []) if str(cid).strip()]
    if requested_ids:
        raw_hits = idx.get_chunks_by_ids(requested_ids, caller_role="gm")
        eff_detail: Literal["compact", "full"] = "full"
    else:
        raw_hits = idx.search(
            query,
            corpora=corpora,
            caller_role="gm",
            arc_stage=arc_stage,
            top_k=max(1, min(12, int(top_k))),
            include_linked_rust=(norm_scope in {"all", "rules", "ttrpg_rules"}),
            detail=norm_detail,
        )
        eff_detail = norm_detail

    hits: list[dict[str, Any]] = []
    remaining_budget = 4800
    for hit in raw_hits:
        item = {k: v for k, v in hit.items() if k != "search_text"}
        if eff_detail == "full" and "content" in item:
            body = str(item.get("content") or "")
            max_piece = min(1400, max(200, remaining_budget))
            if len(body) > max_piece:
                body = body[:max_piece] + "\n...[truncated]..."
            item["content"] = body
            remaining_budget = max(0, remaining_budget - len(body))
        hits.append(item)

    return {
        "query": query,
        "scope": norm_scope,
        "detail": eff_detail,
        "arc_stage": arc_stage,
        "backend": idx.stats().get("backend"),
        "hits": hits,
    }
