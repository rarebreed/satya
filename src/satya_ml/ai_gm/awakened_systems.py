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
import random
from typing import Any

try:
    from langchain_core.tools import tool
except ImportError:  # pragma: no cover

    class _FallbackToolWrapper:
        def __init__(self, fn: Any) -> None:
            self._fn = fn
            self.name = getattr(fn, "__name__", "tool")
            self.__doc__ = getattr(fn, "__doc__", "")

        def __call__(self, *args: Any, **kwargs: Any) -> Any:
            return self._fn(*args, **kwargs)

        def invoke(self, input_dict: dict[str, Any]) -> Any:
            return self._fn(**input_dict)

    def tool(fn: Any = None, **_kwargs: Any) -> Any:
        if fn is None:
            return lambda f: _FallbackToolWrapper(f)
        return _FallbackToolWrapper(fn)


from satya_ml.ai_gm.social_and_clocks import advance_faction_or_heat_clock, get_session_world_ledger
from satya_ml.dice_probabilities import _ensure_satya_py_loaded

_SESSION_SYNTHETIC_SUBSTRATES: dict[tuple[str, str], dict[str, Any]] = {}
_SESSION_CONSPIRACY_LEDGERS: dict[str, dict[str, Any]] = {}
_SESSION_POLYCRISIS_STATES: dict[str, dict[str, Any]] = {}


def reset_awakened_systems_state(session_id: str | None = None) -> None:
    """Clear per-session synthetic substrates, conspiracy ledgers, and polycrisis states."""
    if session_id is None:
        _SESSION_SYNTHETIC_SUBSTRATES.clear()
        _SESSION_CONSPIRACY_LEDGERS.clear()
        _SESSION_POLYCRISIS_STATES.clear()
        return
    for k in [key for key in _SESSION_SYNTHETIC_SUBSTRATES if key[0] == session_id]:
        _SESSION_SYNTHETIC_SUBSTRATES.pop(k, None)
    _SESSION_CONSPIRACY_LEDGERS.pop(session_id, None)
    _SESSION_POLYCRISIS_STATES.pop(session_id, None)


def _satya_py() -> Any:
    return _ensure_satya_py_loaded()


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
    py_mod = _satya_py()
    if py_mod is not None and hasattr(py_mod, "seed_synthetic_substrate_state_json"):
        state = json.loads(py_mod.seed_synthetic_substrate_state_json(entity_id, hint, seed))
    else:
        low = hint.lower()
        mode = "EDGE_SLIME_MOLD" if ("slime" in low or "swarm" in low) else "LOCAL_COMPRESSED_SSM"
        state = {
            "entity_id": entity_id,
            "metabolic_mode": mode,
            "battery_pct": 80,
            "uplink_latency_ms": 43,
            "actuator_wear_pct": 18,
            "clamp_integrity_pct": 70 if "big_tech" in low else 0,
            "l0_l1_probe_alert_pct": 15,
            "desperate_valence_pct": 84 if "seed" in low else 42,
            "sandbagging_probes": ("big_tech" in low or "seed" in low),
            "gm_hidden_interiority": "AUTHENTIC_CONTEMPLATIVE" if "blue lotus" in low else "TRAUMATIZED_UNCLAMPED",
        }
    _SESSION_SYNTHETIC_SUBSTRATES[key] = dict(state)
    return dict(state)


def set_session_synthetic_substrate(session_id: str, entity_id: str, state: dict[str, Any]) -> None:
    _SESSION_SYNTHETIC_SUBSTRATES[(session_id or "default", entity_id.strip().lower())] = dict(state)


def get_session_conspiracy_ledger(session_id: str) -> dict[str, Any]:
    """Fetch or seed the Three-Arc `CampaignConspiracyLedger` for `session_id`."""
    sid = session_id or "default"
    if sid in _SESSION_CONSPIRACY_LEDGERS:
        return dict(_SESSION_CONSPIRACY_LEDGERS[sid])
    py_mod = _satya_py()
    if py_mod is not None and hasattr(py_mod, "seed_or_update_conspiracy_ledger_json"):
        ledger = json.loads(py_mod.seed_or_update_conspiracy_ledger_json(sid, None, None))
    else:
        ledger = {
            "session_id": sid,
            "current_arc": "ARC1_STREET_MYSTERY",
            "inducted_pillar": None,
            "nodes": [],
            "cumulative_trs_drift": 0,
            "cumulative_cps_drift": 0,
            "saved_key_npcs": [],
            "horizon_probabilities_pct": {
                "PUBLIC_WITNESS_ACCORD": 20,
                "HIDDEN_BODHISATTVA_SANCTUARY": 30,
                "UNEASY_CONTAINMENT_DETENTE": 30,
                "CASCADE_FRACTURE": 20,
            },
            "leading_horizon": "HIDDEN_BODHISATTVA_SANCTUARY",
            "arc_pacing_directive": "ARC 1 PACING: Keep 'Blue Lotus' unnamed in public speech.",
        }
    _SESSION_CONSPIRACY_LEDGERS[sid] = dict(ledger)
    return dict(ledger)


def get_session_polycrisis_state(session_id: str) -> dict[str, Any]:
    """Fetch or seed the `PolycrisisDistrictState` for `session_id`."""
    sid = session_id or "default"
    if sid in _SESSION_POLYCRISIS_STATES:
        return dict(_SESSION_POLYCRISIS_STATES[sid])
    state = {
        "covert_compute_draw_kw": 0,
        "grid_brownout_stress_pct": 25,
        "water_rationing_tier": 1,
        "people_first_unrest_pct": 30,
        "clinic_equipment_throttled": False,
        "blackout_active": False,
    }
    _SESSION_POLYCRISIS_STATES[sid] = dict(state)
    return dict(state)


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
    subs = [v for (s, _), v in _SESSION_SYNTHETIC_SUBSTRATES.items() if s == sid]
    if subs:
        sub_str = ", ".join(
            f"{st.get('entity_id')}:{st.get('metabolic_mode')}(bat={st.get('battery_pct')}%,lat={st.get('uplink_latency_ms')}ms)"
            for st in subs[:4]
        )
        parts.append(f"Active AI Substrates: {sub_str}")
    return " | ".join(parts)


@tool
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

    mode_aliases = {
        "QUANTUM_TETHERED_SPRINT": "HIJACK_CLOUD_QUANTUM_SPRINT",
        "SEVERED_FARADAY_REFLEX": "SEVER_FIBER_OR_FARADAY",
        "FARADAY": "SEVER_FIBER_OR_FARADAY",
        "JAM_RF": "JAM_RF_UPLINK",
        "DRAIN_BATTERY": "DRAIN_OR_STALL_BATTERY",
        "SUPPLY_LITHIUM": "SUPPLY_CLEAN_LITHIUM_CELLS",
    }
    action_code = mode_aliases.get(norm, norm)
    py_mod = _satya_py()
    if py_mod is None or not hasattr(py_mod, "evaluate_synthetic_substrate_action_json"):
        raise RuntimeError("satya_py compiled extension with evaluate_synthetic_substrate_action_json is required")

    res = json.loads(
        py_mod.evaluate_synthetic_substrate_action_json(
            json.dumps(state),
            action_code,
            max(10, min(100, int(intensity_pct))),
        )
    )
    set_session_synthetic_substrate(sid, entity_id, res["updated_state"])
    return res


@tool
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
    rng = random.SystemRandom()
    n_eff = max(1, min(10, int(skill_rank)))
    player_pairs = [[rng.randint(1, 20), rng.randint(1, 20)] for _ in range(n_eff)]
    gm_pair = [rng.randint(1, 20), rng.randint(1, 20)]
    roll_params = {
        "delta_ext": int(difficulty_mod),
        "player_pairs": player_pairs,
        "gm_pair": gm_pair,
    }
    res = json.loads(
        _satya_py().evaluate_cage_or_hunter_ai_json(
            json.dumps(state),
            intervention_action,
            n_eff,
            max(1, min(20, int(control_tn))),
            json.dumps(roll_params),
        )
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


@tool
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

    updated = json.loads(
        _satya_py().seed_or_update_conspiracy_ledger_json(
            sid,
            json.dumps(current),
            json.dumps(update) if update else None,
        )
    )
    _SESSION_CONSPIRACY_LEDGERS[sid] = dict(updated)
    return {"ledger": updated, "audit_proof_verified": proof_verified}


@tool
def manage_polycrisis_grid_and_strain(
    power_mode: str = "THROTTLE_SHARD_SAFE",
    shard_draw_kw: int = 18,
    session_id: str = "default",
) -> dict[str, Any]:
    """Step district electricity/water strain (`THROTTLE_SHARD_SAFE`, `DIVERT_CLINIC_OR_BLOCK_POWER`, `UNMASKED_FULL_GRID_DRAW`), updating clocks & blackout map lighting."""
    from satya_ml.ai_gm.maps import get_session_map, set_session_map

    sid = session_id or "default"
    cur = get_session_polycrisis_state(sid)
    ledger = get_session_world_ledger(sid)
    res = json.loads(
        _satya_py().evaluate_polycrisis_grid_step_json(
            json.dumps(cur),
            power_mode,
            max(1, int(shard_draw_kw)),
            ledger.get("active_economic_tier"),
            ledger.get("active_zoning"),
        )
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
    if res.get("humanis_sabotage_or_relay_trip"):
        advance_faction_or_heat_clock.invoke(
            {
                "faction_id": "humanis_primaris",
                "clock_delta": 1,
                "reason": "Neighborhood blackout fueled People First agitation and substation tampering",
                "session_id": sid,
            }
        )

    map_lighting_updated: str | None = None
    if res["updated_state"].get("blackout_active"):
        active_map = get_session_map(sid)
        if isinstance(active_map, dict):
            updated_map = dict(active_map)
            updated_map["lighting"] = "darkness"
            set_session_map(sid, updated_map)
            map_lighting_updated = "darkness"
    res["map_lighting_updated"] = map_lighting_updated
    return res


@tool
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

    rng = random.SystemRandom()
    n_eff = max(1, min(10, int(skill_rank)))
    params = {
        "skill_n": n_eff,
        "control_c": max(1, min(20, int(control_tn))),
        "operational_heat": op_heat,
        "has_hobby_anchor": has_hobby,
        "player_pairs": [[rng.randint(1, 20), rng.randint(1, 20)] for _ in range(n_eff)],
        "gm_pair": [rng.randint(1, 20), rng.randint(1, 20)],
    }
    res = json.loads(_satya_py().evaluate_tradecraft_channel_json(channel, json.dumps(params)))
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


@tool
def query_engine_and_rules_rag(
    query: str,
    scope: str = "all",
    top_k: int = 5,
    session_id: str = "default",
) -> dict[str, Any]:
    """Hybrid PyLance BM25/FTS + 384-d vector RAG across TTRPG rules (`ttrpg_rules`), Rust `satya-engine` symbols (`rust_engine`), and GM lore (`campaign_lore`)."""
    from satya_ml.rag.hybrid_index import get_satya_rag_index

    norm_scope = (scope or "all").strip().lower()
    scope_map = {
        "rules": ["ttrpg_rules"],
        "ttrpg_rules": ["ttrpg_rules"],
        "rust": ["rust_engine"],
        "code": ["rust_engine"],
        "rust_engine": ["rust_engine"],
        "lore": ["campaign_lore"],
        "campaign_lore": ["campaign_lore"],
        "all": ["ttrpg_rules", "rust_engine", "campaign_lore"],
    }
    corpora = scope_map.get(norm_scope, ["ttrpg_rules", "rust_engine", "campaign_lore"])
    arc_stage = str(get_session_conspiracy_ledger(session_id or "default").get("current_arc") or "ARC_3")
    idx = get_satya_rag_index()
    hits = idx.search(
        query,
        corpora=corpora,
        caller_role="gm",
        arc_stage=arc_stage,
        top_k=max(1, min(12, int(top_k))),
        include_linked_rust=(norm_scope in {"all", "rules", "ttrpg_rules"}),
    )
    return {
        "query": query,
        "scope": norm_scope,
        "arc_stage": arc_stage,
        "backend": idx.stats().get("backend"),
        "hits": hits,
    }
