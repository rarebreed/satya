"""9-Axis Psyche (`TRS`/`CPS`) & Lifepath Anchors helpers for Embodied AI Player Ledgers."""

from __future__ import annotations

from typing import Any

from satya_py.campaign import (
    extract_character_lifepath_anchors,
    normalize_character_sheet_record,
)


def extract_anchors_for_player(character_sheet: dict[str, Any] | None) -> dict[str, Any]:
    """Call Rust ``satya_py.campaign.extract_character_lifepath_anchors`` for the AI Player sheet."""
    if not isinstance(character_sheet, dict) or not character_sheet:
        return {}
    norm = normalize_character_sheet_record(character_sheet)
    return extract_character_lifepath_anchors(norm)


def sync_character_9axis_and_lifepath_anchors(
    ledger: dict[str, Any],
    character_sheet: dict[str, Any] | None,
) -> None:
    """Populate `TRS`/`CPS` red lines, `BoundlessCompassion` surge readiness, and bound lifepath anchors."""
    scores = dict(ledger.get("psyche_scores") or {})
    trs = int(scores.get("trs", 10) or 10)
    cps = int(scores.get("cps", 10) or 10)
    scores["trs"] = trs
    scores["cps"] = cps
    ledger["psyche_scores"] = scores

    if trs <= 6:
        ledger["personal_trust_posture"] = "zero_trust_guarded"
    elif trs >= 14:
        ledger["personal_trust_posture"] = "high_trust_open"
    else:
        ledger["personal_trust_posture"] = "discerning"

    red_lines = list(ledger.get("compassion_red_lines") or [])
    if cps >= 14:
        line = "Refuse collateral cruelty to humans or sentient synthetics; prefer non-lethal restraint and triage."
        if line not in red_lines:
            red_lines.append(line)
    elif cps <= 6:
        line = "Pragmatic unsentimental triage: prioritize mission survival over emotional sentimentality."
        if line not in red_lines:
            red_lines.append(line)
    ledger["compassion_red_lines"] = red_lines
    ledger["boundless_compassion_surge_ready"] = bool(
        cps >= 15 and not ledger.get("conviction_surge_used_this_scene", False)
    )
    ledger.setdefault("npc_trust_states", {})

    anchors = extract_anchors_for_player(character_sheet)
    if anchors:
        ledger["lifepath_anchors"] = anchors


def record_empathy_trust_observation(
    ledger: dict[str, Any],
    npc_name: str,
    revealed_traits: list[dict[str, Any]],
    wedge_detected: bool,
) -> dict[str, Any] | None:
    """Record revealed `TRS` / `CPS` telemetry from `perform_player_empathy_read` into `npc_trust_states`."""
    states = dict(ledger.get("npc_trust_states") or {})
    trs_score = None
    cps_score = None
    for t in revealed_traits:
        code = str(t.get("code") or "").upper()
        if code == "TRS":
            trs_score = int(t.get("score", 10))
        elif code == "CPS":
            cps_score = int(t.get("score", 10))

    if trs_score is None and cps_score is None and not wedge_detected:
        return None

    requires_proof = trs_score is not None and trs_score <= 6
    stance = (
        "zero_trust_skeptic_requires_audit_proof"
        if requires_proof
        else ("open_reciprocal" if (trs_score or 10) >= 14 else "cautious_observation")
    )
    entry = dict(states.get(npc_name) or {})
    entry.update(
        {
            "trust_stance": stance,
            "trs_score": trs_score,
            "cps_score": cps_score,
            "requires_audit_proof": requires_proof,
            "dual_psyche_wedge_detected": bool(wedge_detected),
        }
    )
    states[npc_name] = entry
    ledger["npc_trust_states"] = states
    return entry


def apply_npc_trust_journal_entry(ledger: dict[str, Any], npc_trust_update: str) -> None:
    """Parse `'npc_id:stance_or_note'` updates from `update_my_motivation_journal`."""
    raw = npc_trust_update.strip()
    if not raw:
        return
    if ":" in raw:
        npc_key, note = raw.split(":", 1)
    else:
        npc_key, note = raw, "updated"
    npc_key = npc_key.strip()
    note = note.strip()
    states = dict(ledger.get("npc_trust_states") or {})
    cur = dict(states.get(npc_key) or {})
    cur["trust_stance"] = note
    cur["requires_audit_proof"] = (
        "proof" in note.lower() or "hash" in note.lower() or "skeptic" in note.lower()
    )
    states[npc_key] = cur
    ledger["npc_trust_states"] = states


def format_9axis_and_lifepath_prompt_lines(ledger: dict[str, Any]) -> list[str]:
    """Render 9-axis `TRS`/`CPS` rules, NPC trust states, and bound lifepath anchors for AI Player prompts."""
    lines: list[str] = []
    scores = ledger.get("psyche_scores") or {}
    posture = ledger.get("personal_trust_posture")
    if posture:
        lines.append(
            f"- Trust & Compassion Posture: TRS={scores.get('trs', 10)} ({posture}), CPS={scores.get('cps', 10)}"
        )
    if ledger.get("boundless_compassion_surge_ready"):
        lines.append(
            "- BoundlessCompassion Conviction Surge Ready (`CPS >= 15`): Claim `+2` pairs (`0` Fatigue) when intervening to shield innocents or save a life."
        )
    red_lines = ledger.get("compassion_red_lines") or []
    if red_lines:
        lines.append(f"- Compassion Red Lines: {'; '.join(str(r) for r in red_lines[:2])}")

    npc_trust = ledger.get("npc_trust_states") or {}
    if npc_trust:
        parts = []
        for name, info in list(npc_trust.items())[:4]:
            tag = " [NEEDS AUDIT PROOF]" if info.get("requires_audit_proof") else ""
            parts.append(f"{name}={info.get('trust_stance', 'observed')}{tag}")
        lines.append(f"- NPC Trust Assessments: {'; '.join(parts)}")

    anchors = ledger.get("lifepath_anchors") or {}
    if anchors.get("faction_id"):
        lines.append(
            f"- Faction Identity: `{anchors.get('faction_archetype') or anchors.get('faction_id')}`"
            + (f" ({anchors['faction_wedge_hint']})" if anchors.get("faction_wedge_hint") else "")
        )
    hobbies = anchors.get("hobby_contacts") or []
    if hobbies:
        h_str = "; ".join(
            f"{h.get('hobby_code')} -> {h.get('grounded_contact_role')}" for h in hobbies[:2]
        )
        lines.append(f"- Bound Hobby Contacts: {h_str}")
    formative = anchors.get("formative_anchors") or []
    if formative:
        f_str = "; ".join(f"{e.get('event_code')}: {e.get('narrative_hook')}" for e in formative[:2])
        lines.append(f"- Formative Life Echoes: {f_str}")
    return lines
