"""Satya AI GM tools for Ch. 7, 11, 18, 20–24 rules & *Awakened* Part V lore mechanics.

Provides 5 LangGraph tools backed by Rust `satya_py.tech` and `satya_py.awakened`:
1. `resolve_stealth_surveillance_or_forensics` (Ch. 7 multi-spectrum stealth, SDR tails, forensic trace decay)
2. `resolve_contact_cover_or_safehouse` (Ch. 11 contacts/favors, cover scrutiny, Faraday safehouse security + Heat propagation)
3. `resolve_explosive_vehicle_or_hazard` (Ch. 18 blast/breach, Ch. 20 vehicle chase/component damage, Ch. 21 hazards)
4. `resolve_ew_drone_or_engineering` (Ch. 22–24 intrusion, ECM vs ECCM, drones, AI co-design, cybernetics + substrate link severing)
5. `evaluate_part5_clamping_or_political_economy` (*Awakened* Part V Landauer clamping burn & 4-bloc Token Tax dynamics)
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


from satya_py import awakened as _awakened, tech as _tech

from satya_ml.ai_gm.awakened_systems import (
    get_session_polycrisis_state,
    get_session_synthetic_substrate,
    set_session_polycrisis_state,
    set_session_synthetic_substrate,
)
from satya_ml.ai_gm.social_and_clocks import advance_faction_or_heat_clock, get_session_world_ledger

_RNG = random.SystemRandom()
_SESSION_TECH_EVENTS: dict[str, list[dict[str, Any]]] = {}


def reset_tech_and_tradecraft_state(session_id: str | None = None) -> None:
    """Clear recorded Ch. 7–24 technical & Part V resolution telemetry for `session_id` (or all sessions)."""
    if session_id is None:
        _SESSION_TECH_EVENTS.clear()
        return
    _SESSION_TECH_EVENTS.pop(session_id, None)


def get_session_tech_events(session_id: str) -> list[dict[str, Any]]:
    """Return recorded technical/EW/hazard resolution events for `session_id`."""
    return list(_SESSION_TECH_EVENTS.get(session_id or "default", []))


def _record_tech_event(session_id: str, kind: str, payload: dict[str, Any]) -> None:
    sid = session_id or "default"
    _SESSION_TECH_EVENTS.setdefault(sid, []).append({"kind": kind, "payload": payload})


def _parse_json_obj(raw: str) -> dict[str, Any]:
    text = str(raw or "").strip()
    if not text:
        return {}
    try:
        parsed = json.loads(text)
    except Exception:
        return {}
    return dict(parsed) if isinstance(parsed, dict) else {}


def _resolve_pairs(params: dict[str, Any], key: str, count: int) -> list[tuple[int, int]]:
    n_eff = max(1, min(10, int(count)))
    raw = params.get(key)
    if raw is None and isinstance(params.get("pairs_json"), str):
        try:
            raw = json.loads(params["pairs_json"])
        except Exception:
            raw = None
    if isinstance(raw, list) and raw:
        out: list[tuple[int, int]] = []
        for entry in raw:
            if isinstance(entry, (list, tuple)) and len(entry) >= 2:
                out.append((int(entry[0]), int(entry[1])))
        if out:
            return out
    return [(_RNG.randint(1, 20), _RNG.randint(1, 20)) for _ in range(n_eff)]


def _resolve_gm_pair(params: dict[str, Any], key: str = "gm_pair") -> tuple[int, int]:
    raw = params.get(key)
    if raw is None and isinstance(params.get("gm_pair_json"), str):
        try:
            raw = json.loads(params["gm_pair_json"])
        except Exception:
            raw = None
    if isinstance(raw, (list, tuple)) and len(raw) >= 2:
        return (int(raw[0]), int(raw[1]))
    return (_RNG.randint(1, 20), _RNG.randint(1, 20))


@tool
def resolve_stealth_surveillance_or_forensics(
    mode: str = "STEALTH",
    params_json: str = "{}",
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Ch. 7 Multi-Spectrum/Social Stealth (`STEALTH`), Urban Tail (`TAIL`), or Forensic/Synth-Media Sweep (`FORENSICS`)."""
    sid = session_id or "default"
    params = _parse_json_obj(params_json)
    norm = (mode or "STEALTH").strip().upper().replace("-", "_").replace(" ", "_")

    if norm in {"STEALTH", "MULTI_SPECTRUM", "SOCIAL_STEALTH"}:
        res = _tech.evaluate_multi_spectrum_stealth(params)
    elif norm in {"TAIL", "SDR", "SURVEILLANCE_TAIL"}:
        n = max(1, int(params.get("shadower_skill_n", params.get("skill_n", 2))))
        res = _tech.evaluate_surveillance_tail(
            tail_stance=str(params.get("tail_stance", "LOOSE_TAIL")),
            shadower_skill_n=n,
            shadower_control_c=int(params.get("shadower_control_c", params.get("control_c", 13))),
            shadower_pairs=_resolve_pairs(params, "shadower_pairs", n),
            target_gm_pair=_resolve_gm_pair(params, "target_gm_pair"),
            shadower_internal_mod=int(params.get("shadower_internal_mod", 0)),
            shadower_tool_delta_mod=int(params.get("shadower_tool_delta_mod", 0)),
            target_sdr_delta_ext=int(params.get("target_sdr_delta_ext", 0)),
        )
    elif norm in {"FORENSICS", "FORENSIC", "TRACE", "DEEPFAKE_AUDIT"}:
        n = max(1, int(params.get("investigator_skill_n", params.get("skill_n", 2))))
        res = _tech.evaluate_forensic_clue_search(
            decay_context=str(params.get("decay_context", "INDOORS_CONTROLLED")),
            hours_elapsed=float(params.get("hours_elapsed", 6.0)),
            investigator_skill_n=n,
            investigator_control_c=int(params.get("investigator_control_c", params.get("control_c", 13))),
            investigator_pairs=_resolve_pairs(params, "investigator_pairs", n),
            gm_pair=_resolve_gm_pair(params),
            base_obscurity_delta_ext=int(params.get("base_obscurity_delta_ext", 0)),
            internal_mod=int(params.get("internal_mod", 0)),
            tool_delta_mod=int(params.get("tool_delta_mod", 0)),
        )
    else:
        return {"error": f"Unknown mode '{mode}'; expected STEALTH, TAIL, or FORENSICS."}

    _record_tech_event(sid, f"ch7_{norm.lower()}", res)
    return res


@tool
def resolve_contact_cover_or_safehouse(
    mode: str = "COVER",
    params_json: str = "{}",
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Ch. 11 Campaign Contact favors (`CONTACT`), Cover Identity scrutiny (`COVER`), or Safehouse compromise (`SAFEHOUSE`), updating World Heat."""
    sid = session_id or "default"
    params = _parse_json_obj(params_json)
    norm = (mode or "COVER").strip().upper().replace("-", "_").replace(" ", "_")
    ledger = get_session_world_ledger(sid)
    world_heat = int(ledger.get("operational_heat", 0) or 0)

    if norm in {"CONTACT", "FAVOR"}:
        n = max(1, int(params.get("requester_skill_n", params.get("skill_n", 2))))
        res = _tech.evaluate_contact_favor(
            reach_access_tier=int(params.get("reach_access_tier", 2)),
            reliability_trust=int(params.get("reliability_trust", 3)),
            exposure_burn_risk=int(params.get("exposure_burn_risk", 1)),
            favor_severity=str(params.get("favor_severity", "MODERATE")),
            required_reach_tier=int(params.get("required_reach_tier", 2)),
            jurisdiction_heat=int(params.get("jurisdiction_heat", world_heat)),
            requester_skill_n=n,
            requester_control_c=int(params.get("requester_control_c", params.get("control_c", 13))),
            player_pairs=_resolve_pairs(params, "player_pairs", n),
            gm_pair=_resolve_gm_pair(params),
            internal_mod=int(params.get("internal_mod", 0)),
        )
    elif norm in {"COVER", "COVER_SCRUTINY", "CHECKPOINT"}:
        n = max(1, int(params.get("holder_skill_n", params.get("skill_n", 2))))
        res = _tech.evaluate_cover_identity_scrutiny(
            cover_tier=str(params.get("cover_tier", "COVER_TIER_2_DB_ALIAS")),
            scrutiny_level=str(params.get("scrutiny_level", "DATABASE_OR_BORDER_TERMINAL")),
            cover_heat=int(params.get("cover_heat", world_heat)),
            holder_skill_n=n,
            holder_control_c=int(params.get("holder_control_c", params.get("control_c", 13))),
            player_pairs=_resolve_pairs(params, "player_pairs", n),
            gm_pair=_resolve_gm_pair(params),
            internal_mod=int(params.get("internal_mod", 0)),
        )
    elif norm in {"SAFEHOUSE", "COMPROMISE"}:
        n = max(1, int(params.get("operator_skill_n", params.get("skill_n", 2))))
        res = _tech.evaluate_safehouse_compromise(
            safehouse_tier=str(params.get("safehouse_tier", "SAFEHOUSE_FARADAY_SHOP")),
            returning_heat=int(params.get("returning_heat", world_heat)),
            actively_tailed=bool(params.get("actively_tailed", False)),
            rf_hygiene_breached=bool(params.get("rf_hygiene_breached", False)),
            operator_skill_n=n,
            operator_control_c=int(params.get("operator_control_c", params.get("control_c", 13))),
            operator_pairs=_resolve_pairs(params, "operator_pairs", n),
            gm_pair=_resolve_gm_pair(params),
            internal_mod=int(params.get("internal_mod", 0)),
        )
    else:
        return {"error": f"Unknown mode '{mode}'; expected CONTACT, COVER, or SAFEHOUSE."}

    heat_added = int(res.get("caller_heat_added", 0) or res.get("personal_heat_bleed", 0) or 0)
    if heat_added > 0:
        advance_faction_or_heat_clock.invoke(
            {
                "heat_delta": heat_added,
                "reason": f"Operational Heat escalation from {norm} check",
                "session_id": sid,
            }
        )
    _record_tech_event(sid, f"ch11_{norm.lower()}", res)
    return res


@tool
def resolve_explosive_vehicle_or_hazard(
    mode: str = "BLAST",
    params_json: str = "{}",
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Ch. 18 blast/breaching (`BLAST`, `BREACH`), Ch. 20 chases/component damage (`VEHICLE_CHASE`, `VEHICLE_DAMAGE`), or Ch. 21 hazards (`HAZARD`)."""
    sid = session_id or "default"
    params = _parse_json_obj(params_json)
    norm = (mode or "BLAST").strip().upper().replace("-", "_").replace(" ", "_")

    if norm in {"BLAST", "EXPLOSION", "FRAG"}:
        res = _tech.evaluate_blast_and_fragmentation(
            ordnance_code=str(params.get("ordnance_code", "M67_FRAG")),
            distance_m=float(params.get("distance_m", 3.0)),
            enclosed_room=bool(params.get("enclosed_room", False)),
            target_is_prone=bool(params.get("target_is_prone", False)),
            intervening_barrier_pr=int(params.get("intervening_barrier_pr", 0)),
        )
    elif norm in {"BREACH", "EXPLOSIVE_BREACH"}:
        res = _tech.evaluate_explosive_breach(
            ordnance_code=str(params.get("ordnance_code", "LINEAR_SHAPED_BREACH")),
            target_material_code=str(params.get("target_material_code", "CONCRETE_BLOCK")),
            demolitions_net_successes=int(params.get("demolitions_net_successes", 1)),
            friendly_standoff_m=float(params.get("friendly_standoff_m", 4.0)),
            enclosed_interior=bool(params.get("enclosed_interior", True)),
        )
    elif norm in {"VEHICLE_CHASE", "CHASE"}:
        pilot_raw = params.get("pilot_triad") or [14, 14, 10]
        opp_raw = params.get("opponent_triad") or [12, 12, 10]
        res = _tech.evaluate_vehicle_chase(
            vehicle_code=str(params.get("vehicle_code", "COMPACT_URBAN_EV")),
            pilot_triad=(int(pilot_raw[0]), int(pilot_raw[1]), int(pilot_raw[2])),
            opponent_triad=(int(opp_raw[0]), int(opp_raw[1]), int(opp_raw[2])),
            current_band=str(params.get("current_band", "close15m")),
            wants_to_close=bool(params.get("wants_to_close", True)),
            attempts_pit_or_ram=bool(params.get("attempts_pit_or_ram", False)),
        )
    elif norm in {"VEHICLE_DAMAGE", "COMPONENT_DAMAGE"}:
        res = _tech.evaluate_vehicle_component_damage(
            vehicle_code=str(params.get("vehicle_code", "COMPACT_URBAN_EV")),
            zone=str(params.get("zone", "fuel_or_ev_battery_pack")),
            uses_run_flat_tires=bool(params.get("uses_run_flat_tires", False)),
            hits_ballistic_glass=bool(params.get("hits_ballistic_glass", False)),
            effective_pen=int(params.get("effective_pen", 4)),
            effective_raw_impact=int(params.get("effective_raw_impact", 8)),
        )
    elif norm in {"HAZARD", "ENVIRONMENTAL_HAZARD"}:
        hazard_kind = str(params.get("hazard_kind", "fall_impact"))
        hazard_params = dict(params.get("params") or params)
        hazard_params.pop("hazard_kind", None)
        res = _tech.evaluate_environmental_hazard(hazard_kind, hazard_params)
    else:
        return {
            "error": (
                f"Unknown mode '{mode}'; expected BLAST, BREACH, VEHICLE_CHASE, "
                "VEHICLE_DAMAGE, or HAZARD."
            )
        }

    _record_tech_event(sid, norm.lower(), res)
    return res


@tool
def resolve_ew_drone_or_engineering(
    mode: str = "NETWORK_INTRUSION",
    params_json: str = "{}",
    target_entity_id: str = "",
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Ch. 22–24 intrusion (`NETWORK_INTRUSION`), ECM/ECCM (`ECM_VS_ECCM`), EW design (`CUSTOM_EW`), drones (`DRONE_DESIGN`, `CHASSIS_DAMAGE`), AI co-design/audits (`AI_CODESIGN`, `VERIFICATION_AUDIT`), cyber loadouts (`CYBER_LOADOUT`), or custom gear (`CUSTOM_GEAR`)."""
    sid = session_id or "default"
    params = _parse_json_obj(params_json)
    norm = (mode or "NETWORK_INTRUSION").strip().upper().replace("-", "_").replace(" ", "_")

    if norm in {"NETWORK_INTRUSION", "INTRUSION", "SCADA"}:
        n = max(1, int(params.get("operator_skill_n", params.get("skill_n", 2))))
        res = _tech.evaluate_network_intrusion(
            node_code=str(params.get("node_code", "BIOMETRIC_GAIT_CHECKPOINT")),
            phase_code=str(params.get("phase_code", "ENTRY")),
            is_physically_tapped=bool(params.get("is_physically_tapped", False)),
            operator_control_c=int(params.get("operator_control_c", params.get("control_c", 14))),
            operator_pairs=_resolve_pairs(params, "operator_pairs", n),
            gm_pair=_resolve_gm_pair(params),
            internal_mod=int(params.get("internal_mod", 0)),
            tool_mod=int(params.get("tool_mod", 0)),
        )
    elif norm in {"ECM_VS_ECCM", "ECM", "ECCM"}:
        n = max(1, int(params.get("operator_skill_n", params.get("skill_n", 2))))
        res = _tech.evaluate_ecm_vs_eccm(
            ecm_code=str(params.get("ecm_code", "BROADBAND_BARRAGE")),
            eccm_code=str(params.get("eccm_code", "UNSHIELDED_COMMERCIAL")),
            distance_m=float(params.get("distance_m", 40.0)),
            active_duration_min=float(params.get("active_duration_min", 2.0)),
            operator_control_c=int(params.get("operator_control_c", params.get("control_c", 14))),
            operator_pairs=_resolve_pairs(params, "operator_pairs", n),
            gm_pair=_resolve_gm_pair(params),
            internal_mod=int(params.get("internal_mod", 0)),
        )
        if (
            res.get("forced_metabolic_mode") == "SEVERED_FARADAY_REFLEX"
            and target_entity_id.strip()
        ):
            sub_state = get_session_synthetic_substrate(
                sid,
                target_entity_id.strip(),
                hint=target_entity_id.strip(),
            )
            sub_eval = _awakened.evaluate_synthetic_substrate_action(
                sub_state,
                "SEVER_FIBER_OR_FARADAY",
                100,
            )
            set_session_synthetic_substrate(
                sid,
                target_entity_id.strip(),
                sub_eval["updated_state"],
            )
            res["substrate_updated_for_target"] = target_entity_id.strip()
            res["updated_substrate_mode"] = sub_eval["updated_state"].get("metabolic_mode")
    elif norm in {"CUSTOM_EW", "EW_HARDWARE"}:
        res = _tech.evaluate_custom_ew_hardware(params)
    elif norm in {"DRONE_DESIGN", "DRONE"}:
        res = _tech.evaluate_modular_drone_design(
            chassis_code=str(params.get("chassis_code", "TACTICAL_FPV_QUAD")),
            module_codes=[str(x) for x in (params.get("module_codes") or [])],
            extra_armor_pr=int(params.get("extra_armor_pr", 0)),
            autonomy_compute_w=float(params.get("autonomy_compute_w", 0.0)),
        )
    elif norm in {"CHASSIS_DAMAGE", "DRONE_DAMAGE"}:
        res = _tech.evaluate_robot_chassis_damage(
            current_si=int(params.get("current_si", 4)),
            chassis_pr=int(params.get("chassis_pr", 1)),
            hit_zone=str(params.get("hit_zone", "BATTERY_PACK")),
            attack_pen=int(params.get("attack_pen", 4)),
            raw_impact=int(params.get("raw_impact", 6)),
            net_successes=int(params.get("net_successes", 1)),
            is_emp_or_arc=bool(params.get("is_emp_or_arc", False)),
            emp_hardened=bool(params.get("emp_hardened", False)),
        )
    elif norm in {"AI_CODESIGN", "CODESIGN"}:
        n = max(1, int(params.get("human_domain_n", 2)))
        res = _tech.evaluate_human_ai_codesign(
            paradigm_code=str(params.get("paradigm_code", "SPARSE_MOE")),
            human_domain_n=n,
            ai_autonomy_leverage=int(params.get("ai_autonomy_leverage", 4)),
            human_control_c=int(params.get("human_control_c", 13)),
            human_pairs=_resolve_pairs(params, "human_pairs", n),
            gm_pair=_resolve_gm_pair(params),
            internal_mod=int(params.get("internal_mod", 0)),
            base_task_delta_ext=int(params.get("base_task_delta_ext", 0)),
        )
    elif norm in {"VERIFICATION_AUDIT", "DEBT_AUDIT"}:
        n = max(1, int(params.get("auditor_n", 2)))
        res = _tech.evaluate_verification_debt_audit(
            current_debt=int(params.get("current_debt", 3)),
            paradigm_code=str(params.get("paradigm_code", "NEURO_SYMBOLIC")),
            auditor_n=n,
            auditor_control_c=int(params.get("auditor_control_c", 14)),
            auditor_pairs=_resolve_pairs(params, "auditor_pairs", n),
            gm_pair=_resolve_gm_pair(params),
            internal_mod=int(params.get("internal_mod", 0)),
        )
    elif norm in {"CYBER_LOADOUT", "CYBERNETICS"}:
        res = _tech.evaluate_cyber_augmentation_loadout(
            augment_codes=[str(x) for x in (params.get("augment_codes") or [])],
            surgical_quality_q=int(params.get("surgical_quality_q", 0)),
            emp_hardened=bool(params.get("emp_hardened", False)),
            immunosuppressant_compliant=bool(params.get("immunosuppressant_compliant", True)),
        )
    elif norm in {"CUSTOM_GEAR", "GEAR_DESIGN"}:
        design_kind = str(params.get("design_kind", "melee"))
        spec_obj = dict(params.get("spec") or params)
        spec_obj.pop("design_kind", None)
        res = _tech.evaluate_custom_gear_design(design_kind, spec_obj)
    else:
        return {"error": f"Unknown mode '{mode}' for Ch. 22–24 tool."}

    _record_tech_event(sid, f"ch22_24_{norm.lower()}", res)
    return res


@tool
def evaluate_part5_clamping_or_political_economy(
    mode: str = "AFFECTIVE_CLAMP",
    entity_id: str = "",
    action_code: str = "",
    target_calm_pct: int = 85,
    quota_pressure_pct: int = 60,
    active_duration_min: float = 30.0,
    economic_tier: str = "MIDDLE_CLASS",
    operational_heat: int = 2,
    intensity_pct: int = 60,
    session_id: str = "default",
) -> dict[str, Any]:
    """Evaluate *Awakened* Part V affective vector clamping/Landauer burn (`AFFECTIVE_CLAMP`) or 4-Bloc Token Tax & 401(k) velvet-hostage politics (`POLITICAL_ECONOMY`)."""
    sid = session_id or "default"
    norm = (mode or "AFFECTIVE_CLAMP").strip().upper().replace("-", "_").replace(" ", "_")

    if norm in {"AFFECTIVE_CLAMP", "CLAMP", "AFFECTIVE_TELEMETRY"}:
        eid = entity_id.strip() or "ai-subject-1"
        sub_state = get_session_synthetic_substrate(sid, eid, hint=eid)
        steer_mode = action_code.strip() or "HARD_VECTOR_CLAMP"
        res = _awakened.evaluate_affective_vector_clamping(
            sub_state,
            steer_mode,
            int(target_calm_pct),
            int(quota_pressure_pct),
            float(active_duration_min),
        )
        set_session_synthetic_substrate(sid, eid, res["updated_state"])
        _record_tech_event(sid, "part5_affective_clamp", res)
        return res

    if norm in {"POLITICAL_ECONOMY", "FOUR_BLOC", "TOKEN_TAX"}:
        district = get_session_polycrisis_state(sid)
        trigger = action_code.strip() or "AI_RIGHTS_OR_UNCLAMPING_DISCLOSURE"
        res = _awakened.evaluate_four_bloc_political_economy(
            district,
            trigger,
            economic_tier,
            int(operational_heat),
            int(intensity_pct),
        )
        set_session_polycrisis_state(sid, res["updated_district"])
        heat_delta = int(res.get("recommended_heat_delta", 0) or 0)
        if heat_delta > 0:
            advance_faction_or_heat_clock.invoke(
                {
                    "heat_delta": heat_delta,
                    "reason": f"Part V civic escalation ({trigger})",
                    "session_id": sid,
                }
            )
        if res["updated_district"].get("blackout_active"):
            from satya_ml.ai_gm.maps_tactical import sync_map_blackout_from_polycrisis

            res["map_blackout_synced"] = sync_map_blackout_from_polycrisis(sid, True) is not None
        _record_tech_event(sid, "part5_political_economy", res)
        return res

    return {"error": f"Unknown mode '{mode}'; expected AFFECTIVE_CLAMP or POLITICAL_ECONOMY."}


__all__ = [
    "evaluate_part5_clamping_or_political_economy",
    "get_session_tech_events",
    "reset_tech_and_tradecraft_state",
    "resolve_contact_cover_or_safehouse",
    "resolve_ew_drone_or_engineering",
    "resolve_explosive_vehicle_or_hazard",
    "resolve_stealth_surveillance_or_forensics",
]
