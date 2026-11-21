"""Satya AI GM tools for Ch. 7, 11, 18, 20–24 rules & *Awakened* Part V lore mechanics.

Provides 5 LangGraph tools backed by Rust ``satya_py.tech`` and ``satya_py.awakened``:
1. ``resolve_stealth_surveillance_or_forensics`` (Ch. 7 multi-spectrum stealth, SDR tails, forensic trace decay)
2. ``resolve_contact_cover_or_safehouse`` (Ch. 11 contacts/favors, cover scrutiny, Faraday safehouse security + Heat)
3. ``resolve_explosive_vehicle_or_hazard`` (Ch. 18 blast/breach, Ch. 20 vehicle chase/component damage, Ch. 21 hazards)
4. ``resolve_ew_drone_or_engineering`` (Ch. 22–24 intrusion, ECM/ECCM, drones, AI co-design, cybernetics, custom gear)
5. ``evaluate_part5_clamping_or_political_economy`` (*Awakened* Part V Landauer clamping burn & 4-bloc Token Tax)
"""

from __future__ import annotations

from abc import ABC
from collections.abc import Mapping
from types import MappingProxyType
from typing import Annotated, Any, ClassVar, Final, Literal, TypedDict

from pydantic import Field
from satya_py import awakened as _awakened, tech as _tech

from satya_ml.ai_gm.awakened_systems import (
    get_session_polycrisis_state,
    get_session_synthetic_substrate,
    set_session_polycrisis_state,
    set_session_synthetic_substrate,
)
from satya_ml.ai_gm.rolls import (
    ContestedRollRequest,
    ContextToolRequest,
    Executable,
    PassthroughRequest,
    ToolRequest,
    UnionToolInput,
)
from satya_ml.ai_gm.social_and_clocks import advance_faction_or_heat_clock, get_session_world_ledger
from satya_ml.ai_gm.tool_spec import SessionStore, satya_gm_tool


class TechEventRecord(TypedDict):
    """Telemetry record appended to `_SESSION_TECH_EVENTS`."""

    kind: str
    payload: dict[str, Any]


_SESSION_TECH_EVENTS: Final[SessionStore[list[TechEventRecord]]] = SessionStore()


def reset_tech_and_tradecraft_state(session_id: str | None = None) -> None:
    """Clear recorded Ch. 7–24 technical & Part V resolution telemetry for `session_id` (or all sessions)."""
    _SESSION_TECH_EVENTS.reset(session_id)


def get_session_tech_events(session_id: str) -> list[TechEventRecord]:
    """Return recorded technical/EW/hazard resolution events for `session_id`."""
    return list(_SESSION_TECH_EVENTS.get(session_id or "default", []))


def _record_tech_event(session_id: str, kind: str, payload: dict[str, Any]) -> None:
    sid = session_id or "default"
    _SESSION_TECH_EVENTS.setdefault(sid, []).append({"kind": kind, "payload": payload})


def _run_stateless[Req: Executable[dict[str, Any]]](
    request: Req,
    *,
    session_id: str,
    event_kind: str,
) -> dict[str, Any]:
    """Execute a parameterless request conforming to `Executable` and record its telemetry event."""
    res = request.execute()
    _record_tech_event(session_id, event_kind, res)
    return res


# ---------------------------------------------------------------------------
# Domain-Specific Request Abstract Bases (Extending Leaf `rolls.py` Bases)
# ---------------------------------------------------------------------------


class HeatContestedRequest(ContestedRollRequest, ContextToolRequest[int], ABC):
    """Abstract base for Ch. 11 contested checks evaluated against session operational Heat."""

    heat: int | None = None

    def effective_heat(self, world_heat: int) -> int:
        return world_heat if self.heat is None else self.heat


class OrdnanceRequest(ToolRequest, ABC):
    """Abstract base for Ch. 18 blast and explosive-breach requests."""

    ordnance_code: str = "M67_FRAG"


class VehicleRequest(ToolRequest, ABC):
    """Abstract base for Ch. 20 vehicle chase and component-damage requests."""

    vehicle_code: str = "COMPACT_URBAN_EV"


# ---------------------------------------------------------------------------
# Tool 1: Ch. 7 Multi-Spectrum Stealth, SDR Surveillance Tail & Forensics
# ---------------------------------------------------------------------------

type StealthForensicsMode = Literal["STEALTH", "TAIL", "FORENSICS"]


class StealthRequest(PassthroughRequest):
    mode: Literal["STEALTH"] = "STEALTH"

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_multi_spectrum_stealth(self.extra_payload())


class SurveillanceTailRequest(ContestedRollRequest, ToolRequest):
    mode: Literal["TAIL"] = "TAIL"
    tail_stance: str = "LOOSE_TAIL"
    tool_delta_mod: int = 0
    target_sdr_delta_ext: int = 0

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_surveillance_tail(
            tail_stance=self.tail_stance,
            shadower_skill_n=self.skill_n,
            shadower_control_c=self.control_c,
            shadower_pairs=self.resolved_pairs(),
            target_gm_pair=self.resolved_gm_pair(),
            shadower_internal_mod=self.internal_mod,
            shadower_tool_delta_mod=self.tool_delta_mod,
            target_sdr_delta_ext=self.target_sdr_delta_ext,
        )


class ForensicsRequest(ContestedRollRequest, ToolRequest):
    mode: Literal["FORENSICS"] = "FORENSICS"
    decay_context: str = "INDOORS_CONTROLLED"
    hours_elapsed: float = 6.0
    base_obscurity_delta_ext: int = 0
    tool_delta_mod: int = 0

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_forensic_clue_search(
            decay_context=self.decay_context,
            hours_elapsed=self.hours_elapsed,
            investigator_skill_n=self.skill_n,
            investigator_control_c=self.control_c,
            investigator_pairs=self.resolved_pairs(),
            gm_pair=self.resolved_gm_pair(),
            base_obscurity_delta_ext=self.base_obscurity_delta_ext,
            internal_mod=self.internal_mod,
            tool_delta_mod=self.tool_delta_mod,
        )


type StealthForensicsUnion = StealthRequest | SurveillanceTailRequest | ForensicsRequest
StealthForensicsRequest = Annotated[
    StealthRequest | SurveillanceTailRequest | ForensicsRequest,
    Field(discriminator="mode"),
]


class StealthForensicsToolInput(UnionToolInput):
    _mode_aliases: ClassVar[Mapping[str, StealthForensicsMode]] = MappingProxyType(
        {
            "MULTI_SPECTRUM": "STEALTH",
            "SOCIAL_STEALTH": "STEALTH",
            "SDR": "TAIL",
            "SURVEILLANCE_TAIL": "TAIL",
            "FORENSIC": "FORENSICS",
            "TRACE": "FORENSICS",
            "DEEPFAKE_AUDIT": "FORENSICS",
        }
    )
    _default_mode: ClassVar[str] = "STEALTH"
    request: StealthForensicsRequest


@satya_gm_tool(
    pack="tech_and_tradecraft",
    tier="TIER_2_RULESET_ENGINE",
    args_schema=StealthForensicsToolInput,
)
def resolve_stealth_surveillance_or_forensics(
    request: StealthForensicsUnion,
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Ch. 7 Multi-Spectrum/Social Stealth (`STEALTH`), Urban Tail (`TAIL`), or Forensic/Synth-Media Sweep (`FORENSICS`)."""
    sid = session_id or "default"
    return _run_stateless(request, session_id=sid, event_kind=f"ch7_{request.mode.lower()}")


# ---------------------------------------------------------------------------
# Tool 2: Ch. 11 Contacts/Favors, Cover Scrutiny & Safehouse Compromise
# ---------------------------------------------------------------------------

type ContactCoverMode = Literal["CONTACT", "COVER", "SAFEHOUSE"]


class ContactFavorRequest(HeatContestedRequest):
    mode: Literal["CONTACT"] = "CONTACT"
    reach_access_tier: int = 2
    reliability_trust: int = 3
    exposure_burn_risk: int = 1
    favor_severity: str = "MODERATE"
    required_reach_tier: int = 2

    def execute(self, world_heat: int, /) -> dict[str, Any]:
        return _tech.evaluate_contact_favor(
            reach_access_tier=self.reach_access_tier,
            reliability_trust=self.reliability_trust,
            exposure_burn_risk=self.exposure_burn_risk,
            favor_severity=self.favor_severity,
            required_reach_tier=self.required_reach_tier,
            jurisdiction_heat=self.effective_heat(world_heat),
            requester_skill_n=self.skill_n,
            requester_control_c=self.control_c,
            player_pairs=self.resolved_pairs(),
            gm_pair=self.resolved_gm_pair(),
            internal_mod=self.internal_mod,
        )


class CoverScrutinyRequest(HeatContestedRequest):
    mode: Literal["COVER"] = "COVER"
    cover_tier: str = "COVER_TIER_2_DB_ALIAS"
    scrutiny_level: str = "DATABASE_OR_BORDER_TERMINAL"

    def execute(self, world_heat: int, /) -> dict[str, Any]:
        return _tech.evaluate_cover_identity_scrutiny(
            cover_tier=self.cover_tier,
            scrutiny_level=self.scrutiny_level,
            cover_heat=self.effective_heat(world_heat),
            holder_skill_n=self.skill_n,
            holder_control_c=self.control_c,
            player_pairs=self.resolved_pairs(),
            gm_pair=self.resolved_gm_pair(),
            internal_mod=self.internal_mod,
        )


class SafehouseCompromiseRequest(HeatContestedRequest):
    mode: Literal["SAFEHOUSE"] = "SAFEHOUSE"
    safehouse_tier: str = "SAFEHOUSE_FARADAY_SHOP"
    actively_tailed: bool = False
    rf_hygiene_breached: bool = False

    def execute(self, world_heat: int, /) -> dict[str, Any]:
        return _tech.evaluate_safehouse_compromise(
            safehouse_tier=self.safehouse_tier,
            returning_heat=self.effective_heat(world_heat),
            actively_tailed=self.actively_tailed,
            rf_hygiene_breached=self.rf_hygiene_breached,
            operator_skill_n=self.skill_n,
            operator_control_c=self.control_c,
            operator_pairs=self.resolved_pairs(),
            gm_pair=self.resolved_gm_pair(),
            internal_mod=self.internal_mod,
        )


type ContactCoverUnion = ContactFavorRequest | CoverScrutinyRequest | SafehouseCompromiseRequest
ContactCoverSafehouseRequest = Annotated[
    ContactFavorRequest | CoverScrutinyRequest | SafehouseCompromiseRequest,
    Field(discriminator="mode"),
]


class ContactCoverSafehouseToolInput(UnionToolInput):
    _mode_aliases: ClassVar[Mapping[str, ContactCoverMode]] = MappingProxyType(
        {
            "FAVOR": "CONTACT",
            "COVER_SCRUTINY": "COVER",
            "CHECKPOINT": "COVER",
            "COMPROMISE": "SAFEHOUSE",
        }
    )
    _default_mode: ClassVar[str] = "COVER"
    request: ContactCoverSafehouseRequest


@satya_gm_tool(
    pack="tech_and_tradecraft",
    tier="TIER_2_RULESET_ENGINE",
    args_schema=ContactCoverSafehouseToolInput,
)
def resolve_contact_cover_or_safehouse(
    request: ContactCoverUnion,
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Ch. 11 Campaign Contact favors (`CONTACT`), Cover Identity scrutiny (`COVER`), or Safehouse compromise (`SAFEHOUSE`), updating World Heat."""
    sid = session_id or "default"
    world_heat = int(get_session_world_ledger(sid).get("operational_heat", 0) or 0)
    res = request.execute(world_heat)

    heat_added = int(res.get("caller_heat_added", 0) or res.get("personal_heat_bleed", 0) or 0)
    if heat_added > 0:
        advance_faction_or_heat_clock.invoke(
            {
                "heat_delta": heat_added,
                "reason": f"Operational Heat escalation from {request.mode} check",
                "session_id": sid,
            }
        )
    _record_tech_event(sid, f"ch11_{request.mode.lower()}", res)
    return res


# ---------------------------------------------------------------------------
# Tool 3: Ch. 18 Explosives/Breach, Ch. 20 Vehicles & Ch. 21 Hazards
# ---------------------------------------------------------------------------

type ExplosiveVehicleMode = Literal[
    "BLAST",
    "BREACH",
    "VEHICLE_CHASE",
    "VEHICLE_DAMAGE",
    "HAZARD",
]


class BlastRequest(OrdnanceRequest):
    mode: Literal["BLAST"] = "BLAST"
    distance_m: float = 3.0
    enclosed_room: bool = False
    target_is_prone: bool = False
    intervening_barrier_pr: int = 0

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_blast_and_fragmentation(
            ordnance_code=self.ordnance_code,
            distance_m=self.distance_m,
            enclosed_room=self.enclosed_room,
            target_is_prone=self.target_is_prone,
            intervening_barrier_pr=self.intervening_barrier_pr,
        )


class ExplosiveBreachRequest(OrdnanceRequest):
    mode: Literal["BREACH"] = "BREACH"
    ordnance_code: str = "LINEAR_SHAPED_BREACH"
    target_material_code: str = "CONCRETE_BLOCK"
    demolitions_net_successes: int = 1
    friendly_standoff_m: float = 4.0
    enclosed_interior: bool = True

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_explosive_breach(
            ordnance_code=self.ordnance_code,
            target_material_code=self.target_material_code,
            demolitions_net_successes=self.demolitions_net_successes,
            friendly_standoff_m=self.friendly_standoff_m,
            enclosed_interior=self.enclosed_interior,
        )


class VehicleChaseRequest(VehicleRequest):
    mode: Literal["VEHICLE_CHASE"] = "VEHICLE_CHASE"
    pilot_triad: tuple[int, int, int] = (14, 14, 10)
    opponent_triad: tuple[int, int, int] = (12, 12, 10)
    current_band: str = "close15m"
    wants_to_close: bool = True
    attempts_pit_or_ram: bool = False

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_vehicle_chase(
            vehicle_code=self.vehicle_code,
            pilot_triad=self.pilot_triad,
            opponent_triad=self.opponent_triad,
            current_band=self.current_band,
            wants_to_close=self.wants_to_close,
            attempts_pit_or_ram=self.attempts_pit_or_ram,
        )


class VehicleDamageRequest(VehicleRequest):
    mode: Literal["VEHICLE_DAMAGE"] = "VEHICLE_DAMAGE"
    zone: str = "fuel_or_ev_battery_pack"
    uses_run_flat_tires: bool = False
    hits_ballistic_glass: bool = False
    effective_pen: int = 4
    effective_raw_impact: int = 8

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_vehicle_component_damage(
            vehicle_code=self.vehicle_code,
            zone=self.zone,
            uses_run_flat_tires=self.uses_run_flat_tires,
            hits_ballistic_glass=self.hits_ballistic_glass,
            effective_pen=self.effective_pen,
            effective_raw_impact=self.effective_raw_impact,
        )


class EnvironmentalHazardRequest(PassthroughRequest):
    mode: Literal["HAZARD"] = "HAZARD"
    hazard_kind: str = "fall_impact"
    params: dict[str, Any] | None = None

    def execute(self) -> dict[str, Any]:
        hazard_params = (
            dict(self.params)
            if self.params is not None
            else self.extra_payload("hazard_kind", "params")
        )
        return _tech.evaluate_environmental_hazard(self.hazard_kind, hazard_params)


type ExplosiveVehicleHazardUnion = (
    BlastRequest
    | ExplosiveBreachRequest
    | VehicleChaseRequest
    | VehicleDamageRequest
    | EnvironmentalHazardRequest
)
ExplosiveVehicleHazardRequest = Annotated[
    BlastRequest
    | ExplosiveBreachRequest
    | VehicleChaseRequest
    | VehicleDamageRequest
    | EnvironmentalHazardRequest,
    Field(discriminator="mode"),
]


class ExplosiveVehicleHazardToolInput(UnionToolInput):
    _mode_aliases: ClassVar[Mapping[str, ExplosiveVehicleMode]] = MappingProxyType(
        {
            "EXPLOSION": "BLAST",
            "FRAG": "BLAST",
            "EXPLOSIVE_BREACH": "BREACH",
            "CHASE": "VEHICLE_CHASE",
            "COMPONENT_DAMAGE": "VEHICLE_DAMAGE",
            "ENVIRONMENTAL_HAZARD": "HAZARD",
        }
    )
    _default_mode: ClassVar[str] = "BLAST"
    request: ExplosiveVehicleHazardRequest


@satya_gm_tool(
    pack="tech_and_tradecraft",
    tier="TIER_2_RULESET_ENGINE",
    args_schema=ExplosiveVehicleHazardToolInput,
)
def resolve_explosive_vehicle_or_hazard(
    request: ExplosiveVehicleHazardUnion,
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Ch. 18 blast/breaching (`BLAST`, `BREACH`), Ch. 20 chases/component damage (`VEHICLE_CHASE`, `VEHICLE_DAMAGE`), or Ch. 21 hazards (`HAZARD`)."""
    sid = session_id or "default"
    return _run_stateless(request, session_id=sid, event_kind=request.mode.lower())


# ---------------------------------------------------------------------------
# Tool 4: Ch. 22–24 Intrusion, ECM/ECCM, Drones, AI Co-Design & Custom Gear
# ---------------------------------------------------------------------------

type EwEngineeringMode = Literal[
    "NETWORK_INTRUSION",
    "ECM_VS_ECCM",
    "CUSTOM_EW",
    "DRONE_DESIGN",
    "CHASSIS_DAMAGE",
    "AI_CODESIGN",
    "VERIFICATION_AUDIT",
    "CYBER_LOADOUT",
    "CUSTOM_GEAR",
]


class NetworkIntrusionRequest(ContestedRollRequest, ToolRequest):
    mode: Literal["NETWORK_INTRUSION"] = "NETWORK_INTRUSION"
    control_c: int = 14
    node_code: str = "BIOMETRIC_GAIT_CHECKPOINT"
    phase_code: str = "ENTRY"
    is_physically_tapped: bool = False
    tool_mod: int = 0

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_network_intrusion(
            node_code=self.node_code,
            phase_code=self.phase_code,
            is_physically_tapped=self.is_physically_tapped,
            operator_control_c=self.control_c,
            operator_pairs=self.resolved_pairs(),
            gm_pair=self.resolved_gm_pair(),
            internal_mod=self.internal_mod,
            tool_mod=self.tool_mod,
        )


class EcmVsEccmRequest(ContestedRollRequest, ToolRequest):
    mode: Literal["ECM_VS_ECCM"] = "ECM_VS_ECCM"
    control_c: int = 14
    ecm_code: str = "BROADBAND_BARRAGE"
    eccm_code: str = "UNSHIELDED_COMMERCIAL"
    distance_m: float = 40.0
    active_duration_min: float = 2.0

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_ecm_vs_eccm(
            ecm_code=self.ecm_code,
            eccm_code=self.eccm_code,
            distance_m=self.distance_m,
            active_duration_min=self.active_duration_min,
            operator_control_c=self.control_c,
            operator_pairs=self.resolved_pairs(),
            gm_pair=self.resolved_gm_pair(),
            internal_mod=self.internal_mod,
        )


class CustomEwHardwareRequest(PassthroughRequest):
    mode: Literal["CUSTOM_EW"] = "CUSTOM_EW"

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_custom_ew_hardware(self.extra_payload())


class DroneDesignRequest(ToolRequest):
    mode: Literal["DRONE_DESIGN"] = "DRONE_DESIGN"
    chassis_code: str = "TACTICAL_FPV_QUAD"
    module_codes: list[str] = Field(default_factory=list)
    extra_armor_pr: int = 0
    autonomy_compute_w: float = 0.0

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_modular_drone_design(
            chassis_code=self.chassis_code,
            module_codes=list(self.module_codes),
            extra_armor_pr=self.extra_armor_pr,
            autonomy_compute_w=self.autonomy_compute_w,
        )


class RobotChassisDamageRequest(ToolRequest):
    mode: Literal["CHASSIS_DAMAGE"] = "CHASSIS_DAMAGE"
    current_si: int = 4
    chassis_pr: int = 1
    hit_zone: str = "BATTERY_PACK"
    attack_pen: int = 4
    raw_impact: int = 6
    net_successes: int = 1
    is_emp_or_arc: bool = False
    emp_hardened: bool = False

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_robot_chassis_damage(
            current_si=self.current_si,
            chassis_pr=self.chassis_pr,
            hit_zone=self.hit_zone,
            attack_pen=self.attack_pen,
            raw_impact=self.raw_impact,
            net_successes=self.net_successes,
            is_emp_or_arc=self.is_emp_or_arc,
            emp_hardened=self.emp_hardened,
        )


class HumanAiCodesignRequest(ContestedRollRequest, ToolRequest):
    mode: Literal["AI_CODESIGN"] = "AI_CODESIGN"
    paradigm_code: str = "SPARSE_MOE"
    ai_autonomy_leverage: int = 4
    base_task_delta_ext: int = 0

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_human_ai_codesign(
            paradigm_code=self.paradigm_code,
            human_domain_n=self.skill_n,
            ai_autonomy_leverage=self.ai_autonomy_leverage,
            human_control_c=self.control_c,
            human_pairs=self.resolved_pairs(),
            gm_pair=self.resolved_gm_pair(),
            internal_mod=self.internal_mod,
            base_task_delta_ext=self.base_task_delta_ext,
        )


class VerificationDebtAuditRequest(ContestedRollRequest, ToolRequest):
    mode: Literal["VERIFICATION_AUDIT"] = "VERIFICATION_AUDIT"
    control_c: int = 14
    current_debt: int = 3
    paradigm_code: str = "NEURO_SYMBOLIC"

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_verification_debt_audit(
            current_debt=self.current_debt,
            paradigm_code=self.paradigm_code,
            auditor_n=self.skill_n,
            auditor_control_c=self.control_c,
            auditor_pairs=self.resolved_pairs(),
            gm_pair=self.resolved_gm_pair(),
            internal_mod=self.internal_mod,
        )


class CyberLoadoutRequest(ToolRequest):
    mode: Literal["CYBER_LOADOUT"] = "CYBER_LOADOUT"
    augment_codes: list[str] = Field(default_factory=list)
    surgical_quality_q: int = 0
    emp_hardened: bool = False
    immunosuppressant_compliant: bool = True

    def execute(self) -> dict[str, Any]:
        return _tech.evaluate_cyber_augmentation_loadout(
            augment_codes=list(self.augment_codes),
            surgical_quality_q=self.surgical_quality_q,
            emp_hardened=self.emp_hardened,
            immunosuppressant_compliant=self.immunosuppressant_compliant,
        )


class CustomGearDesignRequest(PassthroughRequest):
    mode: Literal["CUSTOM_GEAR"] = "CUSTOM_GEAR"
    design_kind: str = "melee"
    spec: dict[str, Any] | None = None

    def execute(self) -> dict[str, Any]:
        spec_obj = (
            dict(self.spec)
            if self.spec is not None
            else self.extra_payload("design_kind", "spec")
        )
        return _tech.evaluate_custom_gear_design(self.design_kind, spec_obj)


type EwDroneEngineeringUnion = (
    NetworkIntrusionRequest
    | EcmVsEccmRequest
    | CustomEwHardwareRequest
    | DroneDesignRequest
    | RobotChassisDamageRequest
    | HumanAiCodesignRequest
    | VerificationDebtAuditRequest
    | CyberLoadoutRequest
    | CustomGearDesignRequest
)
EwDroneEngineeringRequest = Annotated[
    NetworkIntrusionRequest
    | EcmVsEccmRequest
    | CustomEwHardwareRequest
    | DroneDesignRequest
    | RobotChassisDamageRequest
    | HumanAiCodesignRequest
    | VerificationDebtAuditRequest
    | CyberLoadoutRequest
    | CustomGearDesignRequest,
    Field(discriminator="mode"),
]


class EwDroneEngineeringToolInput(UnionToolInput):
    _mode_aliases: ClassVar[Mapping[str, EwEngineeringMode]] = MappingProxyType(
        {
            "INTRUSION": "NETWORK_INTRUSION",
            "SCADA": "NETWORK_INTRUSION",
            "ECM": "ECM_VS_ECCM",
            "ECCM": "ECM_VS_ECCM",
            "EW_HARDWARE": "CUSTOM_EW",
            "DRONE": "DRONE_DESIGN",
            "DRONE_DAMAGE": "CHASSIS_DAMAGE",
            "CODESIGN": "AI_CODESIGN",
            "DEBT_AUDIT": "VERIFICATION_AUDIT",
            "CYBERNETICS": "CYBER_LOADOUT",
            "GEAR_DESIGN": "CUSTOM_GEAR",
        }
    )
    _default_mode: ClassVar[str] = "NETWORK_INTRUSION"
    _outer_keys: ClassVar[tuple[str, ...]] = ("target_entity_id", "session_id")

    request: EwDroneEngineeringRequest
    target_entity_id: str = ""


@satya_gm_tool(
    pack="tech_and_tradecraft",
    tier="TIER_2_RULESET_ENGINE",
    args_schema=EwDroneEngineeringToolInput,
)
def resolve_ew_drone_or_engineering(
    request: EwDroneEngineeringUnion,
    target_entity_id: str = "",
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Ch. 22–24 intrusion (`NETWORK_INTRUSION`), ECM/ECCM (`ECM_VS_ECCM`), EW design (`CUSTOM_EW`), drones (`DRONE_DESIGN`, `CHASSIS_DAMAGE`), AI co-design/audits (`AI_CODESIGN`, `VERIFICATION_AUDIT`), cyber loadouts (`CYBER_LOADOUT`), or custom gear (`CUSTOM_GEAR`)."""
    sid = session_id or "default"
    res = _run_stateless(request, session_id=sid, event_kind=f"ch22_24_{request.mode.lower()}")
    target_clean = target_entity_id.strip()
    if (
        isinstance(request, EcmVsEccmRequest)
        and res.get("forced_metabolic_mode") == "SEVERED_FARADAY_REFLEX"
        and target_clean
    ):
        sub_state = get_session_synthetic_substrate(sid, target_clean, hint=target_clean)
        sub_eval = _awakened.evaluate_synthetic_substrate_action(
            sub_state,
            "SEVER_FIBER_OR_FARADAY",
            100,
        )
        set_session_synthetic_substrate(sid, target_clean, sub_eval["updated_state"])
        res["substrate_updated_for_target"] = target_clean
        res["updated_substrate_mode"] = sub_eval["updated_state"].get("metabolic_mode")
    return res


# ---------------------------------------------------------------------------
# Tool 5: Awakened Part V Affective Clamping & Four-Bloc Political Economy
# ---------------------------------------------------------------------------

type Part5Mode = Literal["AFFECTIVE_CLAMP", "POLITICAL_ECONOMY"]

_PART5_MODE_ALIASES: Final[Mapping[str, Part5Mode]] = MappingProxyType(
    {
        "AFFECTIVE_CLAMP": "AFFECTIVE_CLAMP",
        "CLAMP": "AFFECTIVE_CLAMP",
        "AFFECTIVE_TELEMETRY": "AFFECTIVE_CLAMP",
        "POLITICAL_ECONOMY": "POLITICAL_ECONOMY",
        "FOUR_BLOC": "POLITICAL_ECONOMY",
        "TOKEN_TAX": "POLITICAL_ECONOMY",
    }
)


@satya_gm_tool(
    pack="awakened_campaign",
    tier="TIER_2_RULESET_ENGINE",
    campaign_packs=("awakened",),
)
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
    canon = _PART5_MODE_ALIASES.get(norm)

    if canon == "AFFECTIVE_CLAMP":
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

    if canon == "POLITICAL_ECONOMY":
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
    "ContestedRollRequest",
    "HeatContestedRequest",
    "OrdnanceRequest",
    "PassthroughRequest",
    "TechEventRecord",
    "VehicleRequest",
    "evaluate_part5_clamping_or_political_economy",
    "get_session_tech_events",
    "reset_tech_and_tradecraft_state",
    "resolve_contact_cover_or_safehouse",
    "resolve_ew_drone_or_engineering",
    "resolve_explosive_vehicle_or_hazard",
    "resolve_stealth_surveillance_or_forensics",
]
