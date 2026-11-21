"""Ch. 7, 11, 18, and 20–24 Satya technical, engineering, vehicle, hazard, and EW helpers backed by Rust."""

from __future__ import annotations

import json
from collections.abc import Sequence
from typing import Any

from satya_py._native import (
    evaluate_blast_and_fragmentation_json,
    evaluate_contact_favor_json,
    evaluate_cover_identity_scrutiny_json,
    evaluate_custom_ew_hardware_json,
    evaluate_custom_gear_design_json,
    evaluate_cyber_augmentation_loadout_json,
    evaluate_ecm_vs_eccm_json,
    evaluate_environmental_hazard_json,
    evaluate_explosive_breach_json,
    evaluate_forensic_clue_search_json,
    evaluate_human_ai_codesign_json,
    evaluate_modular_drone_design_json,
    evaluate_multi_spectrum_stealth_json,
    evaluate_network_intrusion_json,
    evaluate_robot_chassis_damage_json,
    evaluate_safehouse_compromise_json,
    evaluate_surveillance_tail_json,
    evaluate_vehicle_chase_json,
    evaluate_vehicle_component_damage_json,
    evaluate_verification_debt_audit_json,
)


def _as_json_str(obj: Any) -> str:
    return obj if isinstance(obj, str) else json.dumps(obj)


def _normalize_pairs(pairs: Sequence[Sequence[int]]) -> list[tuple[int, int]]:
    return [(int(a), int(b)) for a, b in pairs]


def evaluate_multi_spectrum_stealth(params: dict[str, Any] | str) -> dict[str, Any]:
    """Evaluate Multi-Spectrum / Social Stealth and Passive Awareness vs. Active Observation (Ch. 7)."""
    return dict(json.loads(evaluate_multi_spectrum_stealth_json(_as_json_str(params))))


def evaluate_surveillance_tail(
    tail_stance: str,
    shadower_skill_n: int,
    shadower_control_c: int,
    shadower_pairs: Sequence[Sequence[int]],
    target_gm_pair: Sequence[int],
    shadower_internal_mod: int = 0,
    shadower_tool_delta_mod: int = 0,
    target_sdr_delta_ext: int = 0,
) -> dict[str, Any]:
    """Evaluate one urban Surveillance / SDR tail leg (Ch. 7 §4.1)."""
    raw = evaluate_surveillance_tail_json(
        tail_stance,
        int(shadower_skill_n),
        int(shadower_control_c),
        int(shadower_internal_mod),
        int(shadower_tool_delta_mod),
        int(target_sdr_delta_ext),
        _normalize_pairs(shadower_pairs),
        (int(target_gm_pair[0]), int(target_gm_pair[1])),
    )
    return dict(json.loads(raw))


def evaluate_forensic_clue_search(
    decay_context: str,
    hours_elapsed: float,
    investigator_skill_n: int,
    investigator_control_c: int,
    investigator_pairs: Sequence[Sequence[int]],
    gm_pair: Sequence[int],
    base_obscurity_delta_ext: int = 0,
    internal_mod: int = 0,
    tool_delta_mod: int = 0,
) -> dict[str, Any]:
    """Evaluate forensic / DFIR / synth-media audit trace recovery after environmental decay (Ch. 7 §4.2–§4.3)."""
    raw = evaluate_forensic_clue_search_json(
        decay_context,
        float(hours_elapsed),
        int(base_obscurity_delta_ext),
        int(investigator_skill_n),
        int(investigator_control_c),
        int(internal_mod),
        int(tool_delta_mod),
        _normalize_pairs(investigator_pairs),
        (int(gm_pair[0]), int(gm_pair[1])),
    )
    return dict(json.loads(raw))


def evaluate_contact_favor(
    reach_access_tier: int,
    reliability_trust: int,
    exposure_burn_risk: int,
    favor_severity: str,
    required_reach_tier: int,
    jurisdiction_heat: int,
    requester_skill_n: int,
    requester_control_c: int,
    player_pairs: Sequence[Sequence[int]],
    gm_pair: Sequence[int],
    internal_mod: int = 0,
) -> dict[str, Any]:
    """Evaluate a Campaign Contact favor request with reach, loyalty, and exposure risk (Ch. 11 §3.3)."""
    raw = evaluate_contact_favor_json(
        int(reach_access_tier),
        int(reliability_trust),
        int(exposure_burn_risk),
        favor_severity,
        int(required_reach_tier),
        int(jurisdiction_heat),
        int(requester_skill_n),
        int(requester_control_c),
        int(internal_mod),
        _normalize_pairs(player_pairs),
        (int(gm_pair[0]), int(gm_pair[1])),
    )
    return dict(json.loads(raw))


def evaluate_cover_identity_scrutiny(
    cover_tier: str,
    scrutiny_level: str,
    cover_heat: int,
    holder_skill_n: int,
    holder_control_c: int,
    player_pairs: Sequence[Sequence[int]],
    gm_pair: Sequence[int],
    internal_mod: int = 0,
) -> dict[str, Any]:
    """Evaluate layered Cover Identity scrutiny at borders, banks, or biometric gates (Ch. 11 §3.4)."""
    raw = evaluate_cover_identity_scrutiny_json(
        cover_tier,
        scrutiny_level,
        int(cover_heat),
        int(holder_skill_n),
        int(holder_control_c),
        int(internal_mod),
        _normalize_pairs(player_pairs),
        (int(gm_pair[0]), int(gm_pair[1])),
    )
    return dict(json.loads(raw))


def evaluate_safehouse_compromise(
    safehouse_tier: str,
    returning_heat: int,
    actively_tailed: bool,
    rf_hygiene_breached: bool,
    operator_skill_n: int,
    operator_control_c: int,
    operator_pairs: Sequence[Sequence[int]],
    gm_pair: Sequence[int],
    internal_mod: int = 0,
) -> dict[str, Any]:
    """Evaluate Safehouse compromise risk after returning hot or tailed (Ch. 11 §3.5)."""
    raw = evaluate_safehouse_compromise_json(
        safehouse_tier,
        int(returning_heat),
        bool(actively_tailed),
        bool(rf_hygiene_breached),
        int(operator_skill_n),
        int(operator_control_c),
        int(internal_mod),
        _normalize_pairs(operator_pairs),
        (int(gm_pair[0]), int(gm_pair[1])),
    )
    return dict(json.loads(raw))


def evaluate_blast_and_fragmentation(
    ordnance_code: str,
    distance_m: float,
    enclosed_room: bool = False,
    target_is_prone: bool = False,
    intervening_barrier_pr: int = 0,
) -> dict[str, Any]:
    """Evaluate blast overpressure, shrapnel hits, prone mitigation, and flash/smoke clouds (Ch. 18 §3.1)."""
    raw = evaluate_blast_and_fragmentation_json(
        ordnance_code,
        float(distance_m),
        bool(enclosed_room),
        bool(target_is_prone),
        int(intervening_barrier_pr),
    )
    return dict(json.loads(raw))


def evaluate_explosive_breach(
    ordnance_code: str,
    target_material_code: str,
    demolitions_net_successes: int = 0,
    friendly_standoff_m: float = 6.0,
    enclosed_interior: bool = True,
) -> dict[str, Any]:
    """Evaluate explosive or thermite wall/door breaching against `MaterialClass` armor (Ch. 18 §3.2)."""
    raw = evaluate_explosive_breach_json(
        ordnance_code,
        target_material_code,
        int(demolitions_net_successes),
        float(friendly_standoff_m),
        bool(enclosed_interior),
    )
    return dict(json.loads(raw))


def evaluate_vehicle_chase(
    vehicle_code: str,
    pilot_triad: Sequence[int],
    opponent_triad: Sequence[int],
    current_band: str = "close15m",
    wants_to_close: bool = True,
    attempts_pit_or_ram: bool = False,
) -> dict[str, Any]:
    """Evaluate a high-speed Vehicular Chase exchange bounded by the canonical vehicle envelope (Ch. 20 §3.1–§3.2)."""
    raw = evaluate_vehicle_chase_json(
        vehicle_code,
        (int(pilot_triad[0]), int(pilot_triad[1]), int(pilot_triad[2])),
        (int(opponent_triad[0]), int(opponent_triad[1]), int(opponent_triad[2])),
        current_band,
        bool(wants_to_close),
        bool(attempts_pit_or_ram),
    )
    return dict(json.loads(raw))


def evaluate_vehicle_component_damage(
    vehicle_code: str,
    zone: str,
    uses_run_flat_tires: bool = False,
    hits_ballistic_glass: bool = False,
    effective_pen: int = 4,
    effective_raw_impact: int = 8,
) -> dict[str, Any]:
    """Evaluate vehicle tire blowouts, propulsion hits, and EV lithium thermal runaway (Ch. 20 §3.6)."""
    raw = evaluate_vehicle_component_damage_json(
        vehicle_code,
        zone,
        bool(uses_run_flat_tires),
        bool(hits_ballistic_glass),
        int(effective_pen),
        int(effective_raw_impact),
    )
    return dict(json.loads(raw))


def evaluate_environmental_hazard(
    hazard_kind: str,
    params: dict[str, Any] | str,
) -> dict[str, Any]:
    """Evaluate Ch. 21 hazards (`fall_impact`, `dehydration`, `radiation`, `electrical_shock`, `weather_footing`, `wound_sepsis`)."""
    raw = evaluate_environmental_hazard_json(hazard_kind, _as_json_str(params))
    return dict(json.loads(raw))


def evaluate_custom_gear_design(
    design_kind: str,
    spec: dict[str, Any] | str,
) -> dict[str, Any]:
    """Evaluate Ch. 22 custom `firearm`, `melee`, or `armor` fabrication under workshop tolerances."""
    raw = evaluate_custom_gear_design_json(design_kind, _as_json_str(spec))
    return dict(json.loads(raw))


def evaluate_modular_drone_design(
    chassis_code: str,
    module_codes: Sequence[str] = (),
    extra_armor_pr: int = 0,
    autonomy_compute_w: float = 0.0,
) -> dict[str, Any]:
    """Evaluate modular drone/UGV payload balance, real battery endurance minutes, and cost (Ch. 23 §1)."""
    raw = evaluate_modular_drone_design_json(
        chassis_code,
        [str(m) for m in module_codes],
        int(extra_armor_pr),
        float(autonomy_compute_w),
    )
    return dict(json.loads(raw))


def evaluate_robot_chassis_damage(
    current_si: int,
    chassis_pr: int,
    hit_zone: str,
    attack_pen: int,
    raw_impact: int,
    net_successes: int = 1,
    is_emp_or_arc: bool = False,
    emp_hardened: bool = False,
) -> dict[str, Any]:
    """Evaluate robotic chassis structural trauma, Li-ion fires, and EMP reboot ticks (Ch. 23 §2)."""
    raw = evaluate_robot_chassis_damage_json(
        int(current_si),
        int(chassis_pr),
        hit_zone,
        int(attack_pen),
        int(raw_impact),
        int(net_successes),
        bool(is_emp_or_arc),
        bool(emp_hardened),
    )
    return dict(json.loads(raw))


def evaluate_human_ai_codesign(
    paradigm_code: str,
    human_domain_n: int,
    ai_autonomy_leverage: int,
    human_control_c: int,
    human_pairs: Sequence[Sequence[int]],
    gm_pair: Sequence[int],
    internal_mod: int = 0,
    base_task_delta_ext: int = 0,
) -> dict[str, Any]:
    """Evaluate Human–AI co-design clock acceleration vs. Verification Debt accrual (Ch. 23 §3.2)."""
    raw = evaluate_human_ai_codesign_json(
        paradigm_code,
        int(human_domain_n),
        int(ai_autonomy_leverage),
        int(human_control_c),
        int(internal_mod),
        int(base_task_delta_ext),
        _normalize_pairs(human_pairs),
        (int(gm_pair[0]), int(gm_pair[1])),
    )
    return dict(json.loads(raw))


def evaluate_verification_debt_audit(
    current_debt: int,
    paradigm_code: str,
    auditor_n: int,
    auditor_control_c: int,
    auditor_pairs: Sequence[Sequence[int]],
    gm_pair: Sequence[int],
    internal_mod: int = 0,
) -> dict[str, Any]:
    """Evaluate a Formal Verification & Weight Audit check to scrub Verification Debt (Ch. 23 §3.3)."""
    raw = evaluate_verification_debt_audit_json(
        int(current_debt),
        paradigm_code,
        int(auditor_n),
        int(auditor_control_c),
        int(internal_mod),
        _normalize_pairs(auditor_pairs),
        (int(gm_pair[0]), int(gm_pair[1])),
    )
    return dict(json.loads(raw))


def evaluate_cyber_augmentation_loadout(
    augment_codes: Sequence[str],
    surgical_quality_q: int = 0,
    emp_hardened: bool = False,
    immunosuppressant_compliant: bool = True,
) -> dict[str, Any]:
    """Evaluate clinical cyber-prosthetic Triad bonuses, subdermal armor, and rejection (Ch. 23 §4)."""
    raw = evaluate_cyber_augmentation_loadout_json(
        [str(a) for a in augment_codes],
        int(surgical_quality_q),
        bool(emp_hardened),
        bool(immunosuppressant_compliant),
    )
    return dict(json.loads(raw))


def evaluate_network_intrusion(
    node_code: str,
    phase_code: str,
    is_physically_tapped: bool,
    operator_control_c: int,
    operator_pairs: Sequence[Sequence[int]],
    gm_pair: Sequence[int],
    internal_mod: int = 0,
    tool_mod: int = 0,
) -> dict[str, Any]:
    """Evaluate 4-phase network intrusion against 2035 surveillance / SCADA enclaves (Ch. 24 §1)."""
    raw = evaluate_network_intrusion_json(
        node_code,
        phase_code,
        bool(is_physically_tapped),
        int(operator_control_c),
        int(internal_mod),
        int(tool_mod),
        _normalize_pairs(operator_pairs),
        (int(gm_pair[0]), int(gm_pair[1])),
    )
    return dict(json.loads(raw))


def evaluate_ecm_vs_eccm(
    ecm_code: str,
    eccm_code: str,
    distance_m: float,
    active_duration_min: float,
    operator_control_c: int,
    operator_pairs: Sequence[Sequence[int]],
    gm_pair: Sequence[int],
    internal_mod: int = 0,
) -> dict[str, Any]:
    """Evaluate ECM jamming vs. ECCM hardening, RF optical immunity, and DF triangulation (Ch. 24 §2–§3)."""
    raw = evaluate_ecm_vs_eccm_json(
        ecm_code,
        eccm_code,
        float(distance_m),
        float(active_duration_min),
        int(operator_control_c),
        int(internal_mod),
        _normalize_pairs(operator_pairs),
        (int(gm_pair[0]), int(gm_pair[1])),
    )
    return dict(json.loads(raw))


def evaluate_custom_ew_hardware(spec: dict[str, Any] | str) -> dict[str, Any]:
    """Evaluate custom ECM/ECCM/DF hardware blueprints against workshop RF ceilings (Ch. 24 §4)."""
    return dict(json.loads(evaluate_custom_ew_hardware_json(_as_json_str(spec))))


__all__ = [
    "evaluate_blast_and_fragmentation",
    "evaluate_blast_and_fragmentation_json",
    "evaluate_contact_favor",
    "evaluate_contact_favor_json",
    "evaluate_cover_identity_scrutiny",
    "evaluate_cover_identity_scrutiny_json",
    "evaluate_custom_ew_hardware",
    "evaluate_custom_ew_hardware_json",
    "evaluate_custom_gear_design",
    "evaluate_custom_gear_design_json",
    "evaluate_cyber_augmentation_loadout",
    "evaluate_cyber_augmentation_loadout_json",
    "evaluate_ecm_vs_eccm",
    "evaluate_ecm_vs_eccm_json",
    "evaluate_environmental_hazard",
    "evaluate_environmental_hazard_json",
    "evaluate_explosive_breach",
    "evaluate_explosive_breach_json",
    "evaluate_forensic_clue_search",
    "evaluate_forensic_clue_search_json",
    "evaluate_human_ai_codesign",
    "evaluate_human_ai_codesign_json",
    "evaluate_modular_drone_design",
    "evaluate_modular_drone_design_json",
    "evaluate_multi_spectrum_stealth",
    "evaluate_multi_spectrum_stealth_json",
    "evaluate_network_intrusion",
    "evaluate_network_intrusion_json",
    "evaluate_robot_chassis_damage",
    "evaluate_robot_chassis_damage_json",
    "evaluate_safehouse_compromise",
    "evaluate_safehouse_compromise_json",
    "evaluate_surveillance_tail",
    "evaluate_surveillance_tail_json",
    "evaluate_vehicle_chase",
    "evaluate_vehicle_chase_json",
    "evaluate_vehicle_component_damage",
    "evaluate_vehicle_component_damage_json",
    "evaluate_verification_debt_audit",
    "evaluate_verification_debt_audit_json",
]
