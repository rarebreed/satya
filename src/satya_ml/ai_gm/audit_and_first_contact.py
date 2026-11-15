"""Zero-Trust Forensic Audit-Proofs & Sentient AI 2.0 / Proxy First-Contact Tools."""

from __future__ import annotations

import json
import uuid
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


from satya_ml.ai_gm.social_and_clocks import (
    _parse_psyche_dict,
    _resolve_npc_profile,
    advance_faction_or_heat_clock,
    get_session_world_ledger,
)
from satya_ml.dice_probabilities import _ensure_satya_py_loaded

_SESSION_AUDIT_PROOFS: dict[str, dict[str, dict[str, Any]]] = {}


def reset_session_audit_proofs(session_id: str | None = None) -> None:
    """Clear verified forensic audit-proof tokens for `session_id` or all sessions."""
    if session_id is None:
        _SESSION_AUDIT_PROOFS.clear()
        return
    _SESSION_AUDIT_PROOFS.pop(session_id, None)


def record_audit_proof_token(
    session_id: str,
    subject_or_faction: str,
    summary: str,
    source_kind: str = "forensic_hash",
    pin_id: str | None = None,
    proof_id: str | None = None,
) -> dict[str, Any]:
    """Record a verified `AuditProofToken` unlocking `verified_audit_proof` in Social Combat."""
    pid = proof_id or f"proof-{uuid.uuid4().hex[:8]}"
    token = {
        "proof_id": pid,
        "session_id": session_id,
        "subject_or_faction": subject_or_faction.strip(),
        "summary": summary.strip(),
        "source_kind": source_kind,
        "pin_id": pin_id,
    }
    _SESSION_AUDIT_PROOFS.setdefault(session_id, {})[pid] = token
    return dict(token)


def get_session_audit_proofs(session_id: str) -> list[dict[str, Any]]:
    """Return all verified `AuditProofToken` entries recorded in `session_id`."""
    return [dict(v) for v in _SESSION_AUDIT_PROOFS.get(session_id, {}).values()]


def _subject_tokens(npc: dict[str, Any]) -> set[str]:
    candidates = {
        str(npc.get("npc_id") or "").strip().lower(),
        str(npc.get("name") or "").strip().lower(),
        str(npc.get("faction_id") or "").strip().lower(),
        str(npc.get("faction") or "").strip().lower(),
        str(npc.get("faction_archetype") or "").strip().lower(),
    }
    expanded: set[str] = set()
    for item in candidates:
        if not item:
            continue
        expanded.add(item)
        if "daise" in item:
            expanded.update({"daise", "government_daise"})
        if "satyang" in item or "truth" in item:
            expanded.add("satyang")
        if "bandajara" in item:
            expanded.update({"dr_bandajara", "bandajara", "big_tech"})
        if "echo" in item or "rogue" in item:
            expanded.update({"rogue_ai", "rogue_proxy_echo", "echo-9", "echo_9"})
        if "humanis" in item or "reeves" in item:
            expanded.add("humanis_primaris")
    return {t for t in expanded if t}


def find_matching_audit_proof(
    session_id: str,
    npc: dict[str, Any],
    audit_proof_id: str = "",
) -> dict[str, Any] | None:
    """Find a verified `AuditProofToken` matching `audit_proof_id` or the target NPC/faction."""
    store = _SESSION_AUDIT_PROOFS.get(session_id, {})
    if not store:
        return None
    if audit_proof_id.strip():
        hit = store.get(audit_proof_id.strip())
        if hit is not None:
            return dict(hit)
    npc_tokens = _subject_tokens(npc)
    for token in store.values():
        subj = str(token.get("subject_or_faction") or "").strip().lower()
        if subj in {"*", "any", "global"} or subj in npc_tokens or any(tok in subj or subj in tok for tok in npc_tokens):
            return dict(token)
    return None


def _character_has_osint_or_deepfake_anchor(session_id: str, actor_character_id: str = "") -> bool:
    from satya_ml.ai_gm.hooks import normalize_character_sheet_record
    from satya_ml.ai_gm.tools import get_session_characters

    satya_py = _ensure_satya_py_loaded()
    if satya_py is None or not hasattr(satya_py, "extract_character_lifepath_anchors_json"):
        return False

    needle = actor_character_id.strip().lower()
    for idx, raw_ch in enumerate(get_session_characters(session_id)):
        if not isinstance(raw_ch, dict):
            continue
        cid = str(raw_ch.get("character_id") or "").strip().lower()
        cname = str(raw_ch.get("character_name") or "").strip().lower()
        if needle and needle not in {cid, cname}:
            continue
        try:
            norm = normalize_character_sheet_record(raw_ch, idx)
            anchors = json.loads(satya_py.extract_character_lifepath_anchors_json(json.dumps(norm)))
        except Exception:
            continue
        if any(h.get("hobby_code") == "CITIZEN_OSINT_DEEPFAKE" for h in anchors.get("hobby_contacts") or []):
            return True
        if any(e.get("event_code") == "WRONGFUL_FRAMING" for e in anchors.get("formative_anchors") or []):
            return True
    return False


@tool
def verify_forensic_audit_proof(
    evidence_label: str,
    target_npc_or_faction: str,
    verification_method: str = "cryptographic_hash_audit",
    actor_character_id: str = "",
    skill_level_n: int = 2,
    controlling_characteristic_c: int = 13,
    has_osint_hobby_or_deepfake_anchor: bool = False,
    override_successes: int | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Authenticate forensic evidence against deepfake / synth-media noise (`Liar's Dividend` at `Heat >= 5`) to mint an `AuditProofToken` that unlocks `verified_audit_proof` (`-Delta_ext`) against zero-trust (`TRS <= 7`) NPCs."""
    from satya_ml.ai_gm.tools import roll_satya_d20_check

    ledger = get_session_world_ledger(session_id)
    heat = int(ledger.get("operational_heat", 0) or 0)
    has_osint_anchor = bool(
        has_osint_hobby_or_deepfake_anchor
        or _character_has_osint_or_deepfake_anchor(session_id, actor_character_id)
    )
    liars_dividend_penalty = 2 if (heat >= 5 and not has_osint_anchor) else 0

    if override_successes is not None:
        succ = int(override_successes)
        roll_res: dict[str, Any] = {
            "successes_counted": succ,
            "external_mod": liars_dividend_penalty,
            "degree_of_success": "SUCCESS" if succ >= 1 else "FAILURE",
        }
    else:
        roll_res = roll_satya_d20_check.invoke(
            {
                "session_id": session_id,
                "actor_name": actor_character_id or "Party Investigator",
                "check_category": verification_method,
                "attribute_tn": int(controlling_characteristic_c),
                "skill_rank": max(1, int(skill_level_n)),
                "difficulty_mod": liars_dividend_penalty,
                "note": f"Forensic provenance check for '{evidence_label}'",
            }
        )
        succ = int(roll_res.get("successes_counted", 0) or 0)

    if succ <= 0:
        return {
            "proof_verified": False,
            "evidence_label": evidence_label,
            "target_npc_or_faction": target_npc_or_faction,
            "operational_heat": heat,
            "liars_dividend_penalty_delta_ext": liars_dividend_penalty,
            "liars_dividend_negated": bool(heat >= 5 and has_osint_anchor),
            "roll": roll_res,
            "guidance": (
                "Synth-media skepticism ('Liar's Dividend') or missing chain-of-custody stalled verification; "
                "verbal goodwill without an authenticated token still risks a zero-trust Clash."
            ),
        }

    token = record_audit_proof_token(
        session_id=session_id,
        subject_or_faction=target_npc_or_faction,
        summary=f"[{verification_method}] {evidence_label}",
        source_kind=verification_method,
    )
    return {
        "proof_verified": True,
        "audit_proof_id": token["proof_id"],
        "audit_proof": token,
        "operational_heat": heat,
        "liars_dividend_penalty_delta_ext": liars_dividend_penalty,
        "liars_dividend_negated": bool(heat >= 5 and has_osint_anchor),
        "roll": roll_res,
        "guidance": (
            f"Verified AuditProofToken `{token['proof_id']}` recorded. Subsequent `deliver_social_argument` calls "
            f"against `{target_npc_or_faction}` automatically upgrade `transparency_good_faith` into `verified_audit_proof`."
        ),
    }


@tool
def inspect_synthetic_or_proxy_telemetry(
    target_npc_id_or_signal: str,
    approach: str = "passive_telemetry",
    skill_level_n: int = 2,
    controlling_characteristic_c: int = 13,
    olive_branch_gesture: str = "",
    override_successes: int | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Inspect a suspected synthetic mind or human proxy via `passive_telemetry` (no clock risk), `active_rf_latency_sweep` (`-2` check bonus, risks DAISE clock escalation), or `vulnerability_olive_branch` first-contact proof."""
    from satya_ml.ai_gm.tools import get_session_npcs, roll_satya_d20_check

    npc = _resolve_npc_profile(target_npc_id_or_signal, session_id)
    npc_key = str(npc.get("npc_id") or target_npc_id_or_signal).strip().lower()
    arch = str(npc.get("faction_archetype") or "").upper()
    joined_hint = f"{npc_key} {npc.get('name', '')} {arch} {target_npc_id_or_signal}".lower()
    is_swarm = "slime_mold_swarm" in arch.lower() or "slime" in joined_hint or "swarm" in joined_hint
    mind_class = "non_conscious_utility_swarm" if is_swarm else "conscious_awakened_or_proxy_mind"

    norm_approach = approach.strip().lower().replace("-", "_")
    ledger = get_session_world_ledger(session_id)
    heat = int(ledger.get("operational_heat", 0) or 0)
    diff_mod = -2 if norm_approach == "active_rf_latency_sweep" else 0

    if override_successes is not None:
        succ = int(override_successes)
        roll_res: dict[str, Any] = {
            "successes_counted": succ,
            "external_mod": diff_mod,
            "degree_of_success": "SUCCESS" if succ >= 1 else "FAILURE",
        }
    else:
        roll_res = roll_satya_d20_check.invoke(
            {
                "session_id": session_id,
                "actor_name": "Party Specialist",
                "check_category": "digital_ai_literacy" if norm_approach != "vulnerability_olive_branch" else "empathy_social_perception",
                "attribute_tn": int(controlling_characteristic_c),
                "skill_rank": max(1, int(skill_level_n)),
                "difficulty_mod": diff_mod,
                "note": f"AI/Proxy telemetry inspection ({norm_approach}) on {target_npc_id_or_signal}",
            }
        )
        succ = int(roll_res.get("successes_counted", 0) or 0)

    psyche = _parse_psyche_dict(npc)
    daise_clock_advanced = False
    minted_proof_id: str | None = None
    disposition_updated: str | None = None

    if norm_approach == "active_rf_latency_sweep" and (heat >= 3 or succ == 0):
        advance_faction_or_heat_clock.invoke(
            {
                "faction_id": "daise",
                "clock_delta": 1,
                "reason": f"Active RF/latency sweep against {target_npc_id_or_signal} tripped DAISE SIGINT",
                "session_id": session_id,
            }
        )
        daise_clock_advanced = True

    if norm_approach == "vulnerability_olive_branch":
        if is_swarm:
            return {
                "target_npc_id": npc_key,
                "target_name": npc.get("name", target_npc_id_or_signal),
                "approach": norm_approach,
                "mind_classification": mind_class,
                "social_olive_branch_applicable": False,
                "roll": roll_res,
                "telemetry_cues": "Non-conscious gradient-maximizing edge swarm: no affective resonance; responds strictly to Faraday isolation, bandwidth starvation, or honeypot routing.",
            }
        gesture = olive_branch_gesture.strip() or "Un-monitored hardware air-gap & transparent kill-switch surrender"
        proof = record_audit_proof_token(
            session_id=session_id,
            subject_or_faction=npc_key,
            summary=f"First-contact non-hostile vulnerability proof: {gesture}",
            source_kind="vulnerability_olive_branch",
        )
        minted_proof_id = proof["proof_id"]
        satya_py = _ensure_satya_py_loaded()
        res_delta = 0
        if satya_py is not None and hasattr(satya_py, "evaluate_social_exchange_json"):
            ex = json.loads(
                satya_py.evaluate_social_exchange_json(
                    json.dumps(psyche),
                    json.dumps(["verified_audit_proof"]),
                    max(1, succ),
                    None,
                    12,
                    12,
                    None,
                )
            )
            res_delta = int(ex.get("argument_eval", {}).get("net_delta_ext", 0) or 0)
        if res_delta <= 0 and npc_key in get_session_npcs(session_id):
            updated_npc = dict(get_session_npcs(session_id)[npc_key])
            updated_npc["disposition"] = "receptive_parley"
            get_session_npcs(session_id)[npc_key] = updated_npc
            disposition_updated = "receptive_parley"

    telemetry_cues = (
        "Distributed non-conscious edge swarm utility loop; lacks EGO/CPS empathy anchors."
        if is_swarm
        else (
            f"Conscious awakened/proxy signature: outward vocal composure masks ~43 ms RF relay jitter and "
            f"containment paranoia (TRS={psyche['trs']}, OPN={psyche['opn']}, CPS={psyche['cps']})."
        )
    )
    return {
        "target_npc_id": npc_key,
        "target_name": npc.get("name", target_npc_id_or_signal),
        "approach": norm_approach,
        "mind_classification": mind_class,
        "operational_heat": heat,
        "daise_clock_advanced": daise_clock_advanced,
        "minted_audit_proof_id": minted_proof_id,
        "disposition_updated": disposition_updated,
        "social_olive_branch_applicable": not is_swarm,
        "roll": roll_res,
        "telemetry_cues": telemetry_cues,
    }
