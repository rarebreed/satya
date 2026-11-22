"""Combat Tactical Triad, Count-Up Tick OODA, trauma, and gear helpers backed by Rust."""

from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Final, TypedDict, cast

from satya_py._native import (
    TacticalPoolEnvelope,
    TacticalTriadEvaluation,
    compute_ooda_reset_pivot_ticks,
    compute_starting_tick,
    compute_tactical_pool,
    compute_tactical_pool_envelope,
    evaluate_combat_technique_triad,
    evaluate_health_fatigue_and_bleed,
    evaluate_movement_and_encumbrance,
    evaluate_ranged_recoil_and_suppression,
    evaluate_tactical_triad_contest,
    evaluate_tactical_triad_from_dict,
    evaluate_two_component_action_cost,
    list_combat_techniques,
    resolve_canonical_armor_impact,
    resolve_hit_location_d20,
)

_COMBAT_SKILL_TOKENS: Final[tuple[str, ...]] = (
    "unarmed",
    "melee",
    "firearms",
    "ballistics",
    "archery",
    "tactics",
    "cqc",
    "marksmanship",
    "blades",
    "brawling",
)


class TacticalPoolDict(TypedDict):
    """Plain dictionary projection of Rust `TacticalPoolEnvelope`."""

    d_cog: int
    ooda_init: int
    combat_readiness_c: int
    p_base: int
    p_eff: int
    neutral_baseline_b: int
    tempo_cap: int
    control_cap: int
    force_cap: int
    mastery_overflow: int
    power_pow: float
    momentum_mass_factor_mmf: float
    fatigue_threshold_ft: int
    action_time_factor: float
    morale_state: str
    min_force_required: int
    max_control_allowed: int
    max_force_allowed: int
    force_must_not_exceed_control: bool


class CharacterTacticalEnvelope(TypedDict):
    """Compact tactical envelope and initial tick summary for character sheets."""

    p_base: int
    p_eff: int
    neutral_baseline: int
    max_tempo: int
    max_control: int
    max_force: int
    ooda: int
    starting_tick: int


def compute_tactical_pool_dict(
    spd: int = 12,
    for_stat: int = 12,
    dex: int = 13,
    kin: int = 13,
    sig: int = 11,
    ins: int = 12,
    itu: int = 12,
    dis: int = 12,
    fit: int = 12,
    mass_kg: float = 75.0,
    skill_c: int = 14,
    skill_n: int = 4,
    is_ranged: bool = False,
    fatigue_level: int = 0,
    wound_shock_index: int = 0,
    leg_wound_penalty: int = 0,
    arm_head_wound_penalty: int = 0,
    torso_shoulder_wound_penalty: int = 0,
    morale_state: str = "steady",
    ana: int = 12,
    cre: int = 12,
    foc: int = 12,
) -> TacticalPoolDict:
    """Compute a Satya Tactical Pool Envelope via Rust and return a plain dictionary."""
    env = compute_tactical_pool(
        spd=spd,
        for_stat=for_stat,
        dex=dex,
        kin=kin,
        sig=sig,
        ins=ins,
        itu=itu,
        dis=dis,
        fit=fit,
        mass_kg=mass_kg,
        skill_c=skill_c,
        skill_n=skill_n,
        is_ranged=is_ranged,
        fatigue_level=fatigue_level,
        wound_shock_index=wound_shock_index,
        leg_wound_penalty=leg_wound_penalty,
        arm_head_wound_penalty=arm_head_wound_penalty,
        torso_shoulder_wound_penalty=torso_shoulder_wound_penalty,
        morale_state=morale_state,
        ana=ana,
        cre=cre,
        foc=foc,
    )
    return cast(TacticalPoolDict, env.to_dict())


def compute_character_tactical_envelope(
    char_sheet: Mapping[str, Any],
) -> CharacterTacticalEnvelope:
    """Compute a normalized character sheet's Satya Tactical Pool and OODA `starting_tick` via Rust."""
    raw_chars = char_sheet.get("characteristics") or {}
    chars = {str(k).lower(): v for k, v in raw_chars.items() if isinstance(v, (int, float))}

    def _stat(*keys: str, default: int = 10) -> int:
        return next((int(chars[k]) for k in keys if k in chars), default)

    spd = _stat("spd", "agi")
    for_stat = _stat("for_stat", "for", "str")
    dex = _stat("dex")
    kin = _stat("kin", "end")
    sig = _stat("sig", "per")
    ins = _stat("ins", "per")
    itu = _stat("itu", "per")
    dis = _stat("dis", "wil")
    fit = _stat("fit", "end", "kin")
    ana = _stat("ana", "int_attr", "int")
    cre = _stat("cre", "ana", "int_attr")
    foc = _stat("foc", "wil")
    mass_kg = float(chars.get("mas_kg", chars.get("mass_kg", 75.0)))

    skills = char_sheet.get("skills") or {}
    combat_ranks = [
        int(v)
        for k, v in skills.items()
        if isinstance(v, (int, float)) and any(tok in str(k).lower() for tok in _COMBAT_SKILL_TOKENS)
    ]
    all_ranks = [int(v) for v in skills.values() if isinstance(v, (int, float))]
    skill_n = int(
        char_sheet.get(
            "combat_skill_n",
            char_sheet.get("skill_n", max(combat_ranks or all_ranks or [2])),
        )
    )
    skill_c = int(
        char_sheet.get(
            "combat_skill_c",
            char_sheet.get("skill_c", (spd + dex) // 2 + min(skill_n, 4)),
        )
    )
    fatigue_tier = max(0, int(char_sheet.get("fatigue_tier", char_sheet.get("fatigue_level", 0))))
    wound_shock = max(
        0,
        int(char_sheet.get("wound_shock_index", char_sheet.get("wound_shock_tier", 0))),
    )
    leg_pen = max(0, int(char_sheet.get("leg_wound_penalty", 0)))
    arm_pen = max(0, int(char_sheet.get("arm_head_wound_penalty", 0)))
    torso_pen = max(0, int(char_sheet.get("torso_shoulder_wound_penalty", 0)))
    raw_morale = str(char_sheet.get("morale_state") or "steady").strip().lower()
    morale_state = "high" if raw_morale == "resolute" else raw_morale
    is_ranged = bool(char_sheet.get("is_ranged", False))

    env = compute_tactical_pool(
        spd=spd,
        for_stat=for_stat,
        dex=dex,
        kin=kin,
        sig=sig,
        ins=ins,
        itu=itu,
        dis=dis,
        fit=fit,
        mass_kg=mass_kg,
        skill_c=skill_c,
        skill_n=skill_n,
        is_ranged=is_ranged,
        fatigue_level=fatigue_tier,
        wound_shock_index=wound_shock,
        leg_wound_penalty=leg_pen,
        arm_head_wound_penalty=arm_pen,
        torso_shoulder_wound_penalty=torso_pen,
        morale_state=morale_state,
        ana=ana,
        cre=cre,
        foc=foc,
    )
    ooda_val = int(env.ooda_init)
    return CharacterTacticalEnvelope(
        p_base=int(env.p_base),
        p_eff=int(env.p_eff),
        neutral_baseline=int(env.neutral_baseline_b),
        max_tempo=int(env.tempo_cap),
        max_control=int(env.control_cap),
        max_force=int(env.force_cap),
        ooda=ooda_val,
        starting_tick=max(1, 15 - ooda_val),
    )


__all__ = [
    "CharacterTacticalEnvelope",
    "TacticalPoolDict",
    "TacticalPoolEnvelope",
    "TacticalTriadEvaluation",
    "compute_character_tactical_envelope",
    "compute_ooda_reset_pivot_ticks",
    "compute_starting_tick",
    "compute_tactical_pool",
    "compute_tactical_pool_dict",
    "compute_tactical_pool_envelope",
    "evaluate_combat_technique_triad",
    "evaluate_health_fatigue_and_bleed",
    "evaluate_movement_and_encumbrance",
    "evaluate_ranged_recoil_and_suppression",
    "evaluate_tactical_triad_contest",
    "evaluate_tactical_triad_from_dict",
    "evaluate_two_component_action_cost",
    "list_combat_techniques",
    "resolve_canonical_armor_impact",
    "resolve_hit_location_d20",
]
