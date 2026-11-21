"""9-Axis Psyche Morale, Emotional Triggers, and Non-Lethal Combat Modulation for NPCs."""

from __future__ import annotations

from collections.abc import Mapping
from types import MappingProxyType
from typing import Annotated, Any, ClassVar, Literal, TypedDict

from pydantic import Field
from satya_py import psyche as _psyche

from satya_ml.ai_gm.rolls import ContestedRollRequest, ContextToolRequest, UnionToolInput
from satya_ml.ai_gm.social_and_clocks import _parse_psyche_dict, _resolve_npc_profile
from satya_ml.ai_gm.tool_spec import satya_gm_tool


def extract_npc_psyche_dict(npc: dict[str, Any]) -> dict[str, int]:
    """Extract normalized 9-axis `1..=20` Psyche traits (including `trs`, `cps`) from an NPC profile."""
    return _parse_psyche_dict(npc)


def _lookup_mutable_npc_key(npcs: dict[str, dict[str, Any]], needle: str) -> str | None:
    low = needle.strip().lower()
    if low in npcs:
        return low
    return next(
        (
            cid
            for cid, prof in npcs.items()
            if low == str(prof.get("name") or "").strip().lower()
            or low in cid
            or low in str(prof.get("name") or "").lower()
        ),
        None,
    )


class _MoraleContext(TypedDict):
    psyche: dict[str, int]
    dis: int
    current_morale: str
    fatigue: int


class _MoraleOutcome(TypedDict):
    new_morale: str
    delta_int_inflicted: int
    engine_result: dict[str, Any]


type NpcMoraleMode = Literal["emotional_trigger", "combat_morale"]


class EmotionalTriggerRequest(ContestedRollRequest, ContextToolRequest[_MoraleContext, _MoraleOutcome]):
    """Ch. 9 NPC emotional trigger check evaluated against 9-axis Psyche traits."""

    mode: Literal["emotional_trigger"] = "emotional_trigger"
    skill_n: int = Field(default=1, ge=1)
    trigger_kind: str = "betrayed_by_trusted_ally"
    wound_tier: int = 1

    def execute(self, context: _MoraleContext, /) -> _MoraleOutcome:
        pairs_raw = [list(pair) for pair in self.resolved_pairs(1)]
        engine_res = _psyche.evaluate_emotional_trigger(
            context["psyche"],
            context["dis"],
            context["current_morale"],
            context["fatigue"],
            self.trigger_kind,
            int(self.wound_tier),
            pairs_raw,
        )
        new_morale = str(engine_res.get("new_morale_state") or context["current_morale"]).lower()
        return _MoraleOutcome(
            new_morale=new_morale,
            delta_int_inflicted=int(engine_res.get("delta_int_inflicted", 0) or 0),
            engine_result=engine_res,
        )


class CombatMoraleRequest(ContestedRollRequest, ContextToolRequest[_MoraleContext, _MoraleOutcome]):
    """Contested battlefield morale check factoring command cohesion, casualties, and fatigue."""

    mode: Literal["combat_morale"] = "combat_morale"
    motivation_kind: str = "neutral_or_survival"
    command_intact: bool = True
    ally_casualty_ratio: float = 0.0
    outnumbered_ratio: float = 1.0
    leader_incapacitated_or_comms_cut: bool = False
    position_overrun_flanked_or_ambushed: bool = False
    is_veteran_or_sere: bool = False

    def execute(self, context: _MoraleContext, /) -> _MoraleOutcome:
        sit = {
            "ally_casualty_ratio": max(0.0, float(self.ally_casualty_ratio)),
            "outnumbered_ratio": max(0.0, float(self.outnumbered_ratio)),
            "leader_incapacitated_or_comms_cut": bool(self.leader_incapacitated_or_comms_cut),
            "position_overrun_flanked_or_ambushed": bool(self.position_overrun_flanked_or_ambushed),
        }
        n_p = 2 if self.is_veteran_or_sere else 1
        p_pairs = [list(pair) for pair in self.resolved_pairs(n_p)]
        g_pair = list(self.resolved_gm_pair())
        engine_res = _psyche.evaluate_combat_morale_roll(
            context["psyche"],
            context["dis"],
            self.motivation_kind,
            bool(self.command_intact),
            context["fatigue"],
            sit,
            bool(self.is_veteran_or_sere),
            context["current_morale"],
            p_pairs,
            g_pair,
        )
        roll_out = engine_res.get("roll_outcome") if isinstance(engine_res.get("roll_outcome"), dict) else {}
        new_morale = str(
            roll_out.get("new_morale")
            or (roll_out.get("effect") or {}).get("new_morale")
            or context["current_morale"]
        ).lower()
        return _MoraleOutcome(
            new_morale=new_morale,
            delta_int_inflicted=0,
            engine_result=engine_res,
        )


type NpcMoraleUnion = EmotionalTriggerRequest | CombatMoraleRequest
NpcMoraleRequest = Annotated[
    EmotionalTriggerRequest | CombatMoraleRequest,
    Field(discriminator="mode"),
]


class NpcMoraleToolInput(UnionToolInput):
    _mode_aliases: ClassVar[Mapping[str, NpcMoraleMode]] = MappingProxyType(
        {
            "TRIGGER": "emotional_trigger",
            "MORALE": "combat_morale",
        }
    )
    _default_mode: ClassVar[str] = "emotional_trigger"
    _mode_case: ClassVar[Literal["upper", "lower"]] = "lower"
    _outer_keys: ClassVar[tuple[str, ...]] = ("npc_id", "session_id")

    npc_id: str
    request: NpcMoraleRequest


@satya_gm_tool(
    pack="social_and_psyche",
    tier="TIER_2_RULESET_ENGINE",
    args_schema=NpcMoraleToolInput,
)
def evaluate_npc_morale_or_trigger(
    npc_id: str,
    request: NpcMoraleUnion,
    session_id: str = "default",
) -> dict[str, Any]:
    """Evaluate an NPC's `9`-axis Emotional Trigger (`emotional_trigger`) or Battlefield Morale Check (`combat_morale`) and update their session morale state."""
    from satya_ml.ai_gm.tools import get_session_npcs

    npcs = get_session_npcs(session_id)
    target_key = _lookup_mutable_npc_key(npcs, npc_id)
    npc = dict(npcs[target_key]) if target_key is not None else _resolve_npc_profile(npc_id, session_id)
    psyche = extract_npc_psyche_dict(npc)

    attr_tns = npc.get("attribute_tns") if isinstance(npc.get("attribute_tns"), dict) else {}
    dis = int(attr_tns.get("dis") or npc.get("dis") or 12)
    current_morale = str(npc.get("morale_state") or "steady").strip().lower()
    fatigue = max(0, int(npc.get("fatigue", 0) or 0))

    outcome = request.execute(
        _MoraleContext(
            psyche=psyche,
            dis=dis,
            current_morale=current_morale,
            fatigue=fatigue,
        )
    )
    new_morale = outcome["new_morale"]
    delta_int_inflicted = outcome["delta_int_inflicted"]

    npc["morale_state"] = new_morale
    if delta_int_inflicted > 0:
        npc["fatigue"] = fatigue + 1
    if new_morale in {"broken", "panic"}:
        npc["combat_stance"] = "DEFENSIVE"
        npc["disposition"] = "shaken_surrendering"
    elif new_morale == "rage":
        npc["combat_stance"] = "AGGRESSIVE"
        npc["disposition"] = "enraged"

    if target_key is not None:
        npcs[target_key] = npc

    return {
        "status": "evaluated",
        "npc_id": str(npc.get("npc_id") or npc_id),
        "npc_name": str(npc.get("name") or npc_id),
        "mode": request.mode,
        "previous_morale": current_morale,
        "new_morale_state": new_morale,
        "delta_int_inflicted": delta_int_inflicted,
        "disposition": npc.get("disposition", "wary"),
        "combat_stance": npc.get("combat_stance", "NEUTRAL"),
        "cps": psyche["cps"],
        "trs": psyche["trs"],
        "engine_result": outcome["engine_result"],
    }


def modulate_npc_combat_selection(
    npc_profile: dict[str, Any],
    base_selection: tuple[str, str, int, int, int, int, int],
) -> tuple[str, str, int, int, int, int, int, dict[str, Any]]:
    """Modulate an NPC's Tactical Triad and technique using `morale_state` and `CPS` restraint."""
    skill_name, technique, control_c, skill_n, t_pool, c_pool, f_pool = base_selection
    morale_state = str(npc_profile.get("morale_state") or "steady").strip().lower()
    psyche = extract_npc_psyche_dict(npc_profile)

    retreat_or_surrender = False
    hesitant_under_fire = False
    reckless_rage = False
    non_lethal_restraint = False

    if morale_state == "panic":
        technique = "withdrawal_disengage_cover"
        t_pool = max(2, t_pool)
        c_pool = 0
        f_pool = 0
        retreat_or_surrender = True
    elif morale_state == "broken":
        technique = "withdrawal_disengage_cover"
        t_pool = max(2, t_pool)
        c_pool = max(2, c_pool + 1)
        f_pool = 0
        retreat_or_surrender = True
    elif morale_state == "wavering":
        hesitant_under_fire = True
        if f_pool > 2:
            c_pool += f_pool - 2
            f_pool = 2
    elif morale_state == "rage":
        reckless_rage = True
        f_pool += 1
        c_pool = max(1, c_pool - 1)

    if not retreat_or_surrender and not reckless_rage and psyche["cps"] >= 14:
        non_lethal_restraint = True
        if f_pool > 1:
            f_pool -= 1
            c_pool += 1
        if not technique.endswith("_non_lethal"):
            technique = f"{technique}_non_lethal"

    meta = {
        "morale_state": morale_state,
        "trs": psyche["trs"],
        "cps": psyche["cps"],
        "retreat_or_surrender": retreat_or_surrender,
        "hesitant_under_fire": hesitant_under_fire,
        "reckless_rage": reckless_rage,
        "non_lethal_restraint": non_lethal_restraint,
    }
    return skill_name, technique, control_c, skill_n, t_pool, c_pool, f_pool, meta


def format_npc_psyche_combat_tag(npc_profile: dict[str, Any], party_standing: int = 0) -> str:
    """Format a compact Morale, `TRS`/`CPS`, Faction Archetype, Wedge, and Standing tag."""
    psyche = extract_npc_psyche_dict(npc_profile)
    morale = str(npc_profile.get("morale_state") or "steady").upper()
    arch = str(npc_profile.get("faction_archetype") or "").strip()
    wedge = str(
        npc_profile.get("social_combat_wedge_hint")
        or npc_profile.get("dual_psyche_wedge_hint")
        or ""
    ).strip()

    parts = [
        f"Morale: `{morale}`",
        f"TRS: `{psyche['trs']}`",
        f"CPS: `{psyche['cps']}`",
        f"Standing: `{int(party_standing):+d}`",
    ]
    if arch:
        parts.append(f"Archetype: `{arch}`")
    if wedge:
        short_wedge = wedge[:117].rstrip() + "..." if len(wedge) > 120 else wedge
        parts.append(f"Wedge: `{short_wedge}`")
    return ", ".join(parts)
