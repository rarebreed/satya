"""AI GM Social Action Mode (`INS/ITU` Empathy Reads, Contested Arguments, Dissonance & Surges)
and Persistent Faction Clocks x Operational `Heat` (`0-10`) World Ledger tools.

Routes deterministic Psyche/Social/Heat calculations through compiled Rust `satya_py` bindings
and syncs persistent world-ledger state with `satya-signaling` when online.
"""

from __future__ import annotations

import json
import re
from abc import ABC
from collections.abc import Mapping
from types import MappingProxyType
from typing import Annotated, Any, ClassVar, Final, Literal

from pydantic import Field
from satya_py import campaign as _campaign, psyche as _psyche

from satya_ml.ai_gm.rolls import ContextToolRequest, UnionToolInput, parse_json_obj
from satya_ml.ai_gm.tool_spec import (
    CompositeSessionStore,
    SessionStore,
    run_session_reset_hooks,
    satya_gm_tool,
)
from satya_ml.campaign_config import get_active_campaign_pack_name, get_pack_starter_npcs

type FactionId = Literal["daise", "big_tech", "satyang", "humanitas_prima", "rogue_ai", "plutonomy"]
type PsycheResolutionMode = Literal["dissonance", "conviction_surge", "drift_mark"]

_SESSION_WORLD_LEDGERS: Final[SessionStore[dict[str, Any]]] = SessionStore()
_SESSION_SOCIAL_COMPOSURE: Final[CompositeSessionStore[tuple[str, str], dict[str, Any]]] = (
    CompositeSessionStore()
)
_SESSION_CONVICTION_SURGES: Final[CompositeSessionStore[tuple[str, str], bool]] = (
    CompositeSessionStore()
)
_SESSION_PSYCHE_DRIFT_MARKS: Final[CompositeSessionStore[tuple[str, str, str], int]] = (
    CompositeSessionStore()
)

_DEFAULT_PSYCHE: Final[Mapping[str, int]] = MappingProxyType(
    {
        "opn": 10,
        "slf": 10,
        "blg": 10,
        "agg": 10,
        "ego": 10,
        "cnf": 10,
        "ext": 10,
        "trs": 10,
        "cps": 10,
    }
)

_NPC_FACTION_ALIASES: Final[Mapping[str, FactionId]] = MappingProxyType(
    {
        "daise": "daise",
        "government_daise": "daise",
        "directorate": "daise",
        "big_tech": "big_tech",
        "altruist": "big_tech",
        "altruist-ai": "big_tech",
        "altruist_ai": "big_tech",
        "translucent": "big_tech",
        "shen": "big_tech",
        "helios": "big_tech",
        "helios-synthetics": "big_tech",
        "bandajara": "big_tech",
        "dr_bandajara": "big_tech",
        "satyang": "satyang",
        "truth": "satyang",
        "truth_cell": "satyang",
        "humanitas_prima": "humanitas_prima",
        "humanitas": "humanitas_prima",
        "reeves": "humanitas_prima",
        "plutonomy": "plutonomy",
        "citadel": "plutonomy",
        "patron": "plutonomy",
        "sovereign_capital": "plutonomy",
        "oligarch": "plutonomy",
        "rogue_ai": "rogue_ai",
        "rogue_proxy_echo": "rogue_ai",
        "echo": "rogue_ai",
        "echo-9": "rogue_ai",
        "echo_9": "rogue_ai",
    }
)

_TRANSPARENCY_VECTOR_NAMES: Final[frozenset[str]] = frozenset(
    {
        "transparency_good_faith",
        "transparency_vulnerability_good_faith",
        "trust_alliance_guarantee_reciprocity",
        "transparency",
        "good_faith",
    }
)


def compute_faction_standing_social_leverage(session_id: str, npc: dict[str, Any]) -> dict[str, Any]:
    """Map the party's faction `party_standing` against an NPC into a Social `Delta_ext` shift."""
    ledger = get_session_world_ledger(session_id)
    clocks = ledger.get("faction_clocks") or []
    candidate_blobs = [
        str(npc.get(k) or "").strip().lower()
        for k in ("faction_id", "faction", "faction_archetype", "npc_id", "name")
        if npc.get(k)
    ]
    matched_id: str | None = next(
        (
            canonical
            for blob in candidate_blobs
            for alias, canonical in _NPC_FACTION_ALIASES.items()
            if alias in blob
        ),
        None,
    )
    matched_clock = next(
        (
            clk
            for clk in clocks
            if isinstance(clk, dict)
            and (cid := str(clk.get("faction_id") or "").strip().lower()) is not None
            and (cname := str(clk.get("faction_name") or "").strip().lower()) is not None
            and (
                (matched_id and cid == matched_id)
                or any(
                    cid and (cid in blob or blob in cid or (cname and blob in cname))
                    for blob in candidate_blobs
                )
            )
        ),
        None,
    )
    party_standing = 0
    if matched_clock is not None:
        matched_id = str(matched_clock.get("faction_id") or "").strip().lower() or matched_id
        party_standing = int(matched_clock.get("party_standing", 0) or 0)

    if party_standing == 0:
        return {
            "matched_faction_id": matched_id,
            "party_standing": 0,
            "standing_delta_ext": 0,
            "zero_trust_hostility_penalty": 0,
        }

    trs = _parse_psyche_dict(npc)["trs"]
    zero_trust_penalty = 1 if (party_standing <= -2 and trs <= 7) else 0
    standing_delta_ext = (-party_standing) + zero_trust_penalty
    return {
        "matched_faction_id": matched_id,
        "party_standing": party_standing,
        "standing_delta_ext": standing_delta_ext,
        "zero_trust_hostility_penalty": zero_trust_penalty,
    }


def reset_social_and_world_state(session_id: str | None = None) -> None:
    """Clear per-session world ledgers, NPC social state, and all registered GM session caches."""
    run_session_reset_hooks(session_id)


def _parse_psyche_dict(raw_npc: dict[str, Any]) -> dict[str, int]:
    """Extract normalized `1..=20` 9-axis Psyche traits from an NPC or character dictionary."""
    out = dict(_DEFAULT_PSYCHE)
    for bucket_key in ("psyche", "characteristics", "psyche_traits"):
        bucket = raw_npc.get(bucket_key)
        if isinstance(bucket, dict):
            for k, v in bucket.items():
                lk = str(k).strip().lower()
                if lk in out and isinstance(v, (int, float)):
                    out[lk] = max(1, min(20, int(v)))
    for k in list(out):
        if k in raw_npc and isinstance(raw_npc[k], (int, float)):
            out[k] = max(1, min(20, int(raw_npc[k])))
    summary = raw_npc.get("psyche_summary")
    if isinstance(summary, list):
        for entry in summary:
            for match in re.finditer(r"\b(OPN|SLF|BLG|AGG|EGO|CNF|EXT|TRS|CPS)(?:\s*[:=]\s*|\s+)(\d+)", str(entry), re.I):
                out[match.group(1).lower()] = max(1, min(20, int(match.group(2))))
    return out


def _resolve_npc_profile(target_npc_id_or_name: str, session_id: str) -> dict[str, Any]:
    """Resolve an NPC dictionary from session cache or pack starter TOMLs."""
    from satya_ml.ai_gm.tools import get_session_npcs

    needle = target_npc_id_or_name.strip().lower()
    npcs = dict(get_session_npcs(session_id))
    for cid, npc in get_pack_starter_npcs().items():
        npcs.setdefault(cid, npc)

    if needle in npcs:
        return dict(npcs[needle])
    return next(
        (
            dict(npc)
            for cid, npc in npcs.items()
            if str(npc.get("name") or "").strip().lower() == needle
            or needle in cid
            or needle in str(npc.get("name") or "").lower()
        ),
        {"npc_id": needle or "npc-unknown", "name": target_npc_id_or_name or "NPC"},
    )


def _seed_map_jurisdiction(session_id: str) -> tuple[str | None, str | None]:
    from satya_ml.ai_gm.maps import get_session_map

    active_map = get_session_map(session_id)
    if not isinstance(active_map, dict):
        return None, None
    manifest = active_map.get("narrative_manifest")
    tier = active_map.get("economic_tier") or (
        manifest.get("economic_tier") if isinstance(manifest, dict) else None
    )
    zoning = active_map.get("zoning") or (
        manifest.get("zoning") if isinstance(manifest, dict) else None
    )
    return (
        str(tier).strip() if tier else None,
        str(zoning).strip() if zoning else None,
    )


def get_session_world_ledger(session_id: str) -> dict[str, Any]:
    """Return the `SessionWorldLedger` for `session_id`, syncing signaling or seeding via Rust `satya_py`."""
    from satya_ml.ai_gm.tools import _http_get_json, _signaling_base_url

    remote = _http_get_json(f"{_signaling_base_url()}/api/gm/sessions/{session_id}/world-ledger")
    if isinstance(remote, dict) and isinstance(remote.get("faction_clocks"), list):
        _SESSION_WORLD_LEDGERS[session_id] = dict(remote)
        return dict(remote)

    if session_id in _SESSION_WORLD_LEDGERS:
        return dict(_SESSION_WORLD_LEDGERS[session_id])

    tier, zoning = _seed_map_jurisdiction(session_id)
    req: dict[str, Any] = {}
    if tier:
        req["economic_tier"] = tier
    if zoning:
        req["zoning"] = zoning
    data = _campaign.seed_or_update_world_ledger(
        session_id,
        None,
        req if req else None,
        get_active_campaign_pack_name(),
        session_id,
    )
    ledger = dict(data["ledger"])
    _SESSION_WORLD_LEDGERS[session_id] = ledger
    return dict(ledger)


def update_session_world_ledger(session_id: str, update_req: dict[str, Any]) -> dict[str, Any]:
    """Mutate Operational Heat, jurisdiction tier, or a faction clock via signaling or Rust `satya_py`."""
    from satya_ml.ai_gm.tools import _http_post_json, _signaling_base_url

    clean_req = {k: v for k, v in update_req.items() if v is not None and v != ""}
    remote = _http_post_json(
        f"{_signaling_base_url()}/api/gm/sessions/{session_id}/world-ledger",
        clean_req,
    )
    if isinstance(remote, dict) and isinstance(remote.get("ledger"), dict):
        _SESSION_WORLD_LEDGERS[session_id] = dict(remote["ledger"])
        return remote

    current = get_session_world_ledger(session_id)
    data = _campaign.seed_or_update_world_ledger(
        session_id,
        current,
        clean_req,
        get_active_campaign_pack_name(),
        session_id,
    )
    _SESSION_WORLD_LEDGERS[session_id] = dict(data["ledger"])
    return data


def format_world_ledger_for_prompt(ledger: dict[str, Any] | None) -> str:
    """Format Operational Heat, Jurisdiction Tier, and Faction Clocks into a compact AI GM prompt section."""
    if not isinstance(ledger, dict) or not ledger:
        return ""
    heat = int(ledger.get("operational_heat", 0) or 0)
    tier_code = str(ledger.get("heat_tier_code") or "UNNOTICED")
    delta_ext = int(ledger.get("heat_public_check_delta_ext", 0) or 0)
    frozen = bool(ledger.get("legal_bank_accounts_frozen", False))
    econ = str(ledger.get("active_economic_tier") or "MiddleClass")
    zoning = str(ledger.get("active_zoning") or "UrbanResidential")
    bm_mod = int(ledger.get("black_market_delta_ext_mod", 0) or 0)
    response_prof = str(ledger.get("jurisdiction_response_profile") or "")[:140]

    lines = [
        "### Active World Clocks & Jurisdiction Heat:",
        (
            f"- Operational Heat: `{heat}/10` ({tier_code}) | Public Checks `Delta_ext={delta_ext:+d}` | "
            f"Legal Accounts Frozen: `{frozen}` | Black Market `Delta_ext={bm_mod:+d}`"
        ),
        f"- Jurisdiction ({econ} x {zoning}): {response_prof}",
    ]
    dormant: list[str] = []
    for clk in ledger.get("faction_clocks") or []:
        if not isinstance(clk, dict):
            continue
        progress = int(clk.get("progress", 0) or 0)
        segments = int(clk.get("segments", 6) or 6)
        standing = int(clk.get("party_standing", 0) or 0)
        if progress == 0 and standing == 0:
            dormant.append(f"{clk.get('faction_id')}({progress}/{segments})")
            continue
        agenda = str(clk.get("gm_secret_agenda") or "").strip()[:110]
        rumor = str(clk.get("public_rumor") or "").strip()[:110]
        agenda_txt = f" | [GM Secret Agenda: {agenda}]" if agenda else ""
        lines.append(
            f"- Clock `{clk.get('faction_id')}` ({clk.get('faction_name')} — {clk.get('clock_title')}): "
            f"`{progress}/{segments}` | Standing `{standing:+d}` | "
            f"Rumor: {rumor}{agenda_txt}"
        )
    if dormant:
        lines.append(
            f"- Dormant Clocks (0 progress): {', '.join(dormant)} (call `get_world_clocks_and_heat` for full faction dossiers)"
        )
    recent = ledger.get("recent_ledger_events") or []
    if recent:
        lines.append(f"- Recent World Alerts: {'; '.join(str(x)[:120] for x in recent[-3:])}")
    return "\n".join(lines)


@satya_gm_tool(pack="social_and_psyche", tier="TIER_2_RULESET_ENGINE")
def perform_empathy_read(
    target_npc_id_or_name: str,
    net_successes: int = 1,
    target_psyche_json: str = "",
    session_id: str = "default",
) -> dict[str, Any]:
    """Reveal `1..=20` Psyche traits (`1` trait at 1 success, `2` + wedge tension at 2, full 9 + `-2 Delta_ext` leverage at 3+)."""
    npc = _resolve_npc_profile(target_npc_id_or_name, session_id)
    psyche_val: dict[str, Any] | str = (
        parse_json_obj(target_psyche_json) or target_psyche_json.strip()
        if target_psyche_json.strip()
        else _parse_psyche_dict(npc)
    )
    res = _psyche.evaluate_empathy_read(psyche_val, int(net_successes))
    res["target_npc_id"] = str(npc.get("npc_id") or target_npc_id_or_name)
    res["target_name"] = str(npc.get("name") or target_npc_id_or_name)
    return res


@satya_gm_tool(pack="social_and_psyche", tier="TIER_2_RULESET_ENGINE")
def deliver_social_argument(
    target_npc_id_or_name: str,
    argument_vectors: list[str] | str,
    net_successes: int = 1,
    target_psyche_json: str = "",
    ego_wound_dis_check_passed: bool | None = None,
    session_id: str = "default",
    audit_proof_id: str = "",
) -> dict[str, Any]:
    """Resolve a Contested Social Exchange using Satya's 9 Argument Vectors, Composure (`CMP`), Faction Standing, and Dual-Psyche Wedge checks.

    Supported `argument_vectors` slugs (combinable for Dual-Psyche Wedges):
    - `empirical_logic` (`OPN` evidence/logic)
    - `group_loyalty` / `betray_group` (`BLG` shared identity vs. betrayal)
    - `flattery_prestige` / `humiliate_fault` (`EGO` status vs. Ego Wound provocation)
    - `personal_profit` (`SLF >= 13` self-interest / immunity)
    - `humanitarian_mercy` (`SLF <= 7` self-sacrifice / altruistic duty)
    - `rules_protocol` / `break_law_orders` (`CNF` law, chain of command, or breaking orders)
    - `dominance_threat` / `bait_into_fight` (`AGG` intimidation vs. provoking aggression)
    - `transparency_good_faith` vs. `verified_audit_proof` (`TRS`: open olive branch vs. hard audit proof for `TRS <= 7` zero-trust targets; auto-upgrades when a matching `AuditProofToken` exists in the session)
    - `moral_sanctity` vs. `utilitarian_triage` (`CPS`: witnessed suffering/mercy vs. cold greater-good triage)
    """
    from satya_ml.ai_gm.audit_and_first_contact import find_matching_audit_proof

    npc = _resolve_npc_profile(target_npc_id_or_name, session_id)
    npc_key = str(npc.get("npc_id") or target_npc_id_or_name).strip().lower()
    raw_psyche = target_psyche_json.strip()
    if raw_psyche:
        parsed_psyche = parse_json_obj(raw_psyche)
        npc_for_standing = {**npc, "psyche": parsed_psyche} if parsed_psyche else npc
        psyche_payload: dict[str, Any] | str = parsed_psyche or raw_psyche
    else:
        npc_for_standing = npc
        psyche_payload = _parse_psyche_dict(npc)

    if isinstance(argument_vectors, str):
        stripped = argument_vectors.strip()
        vec_list = json.loads(stripped) if stripped.startswith("[") else [v.strip() for v in stripped.split(",") if v.strip()]
    else:
        vec_list = list(argument_vectors)

    matched_proof = find_matching_audit_proof(session_id, npc, audit_proof_id)
    audit_proof_applied: str | None = None
    upgraded_vectors: list[Any] = []
    for vec in vec_list:
        if isinstance(vec, str):
            norm_v = vec.strip().lower().replace("-", "_")
            if matched_proof is not None and norm_v in _TRANSPARENCY_VECTOR_NAMES:
                upgraded_vectors.append("verified_audit_proof")
                audit_proof_applied = matched_proof["proof_id"]
            else:
                upgraded_vectors.append(vec)
        elif isinstance(vec, dict):
            name_v = str(vec.get("vector") or vec.get("kind") or vec.get("name") or "").strip().lower().replace("-", "_")
            if matched_proof is not None and name_v in _TRANSPARENCY_VECTOR_NAMES:
                cloned = dict(vec)
                cloned["verifies_with_audit_or_proof"] = True
                upgraded_vectors.append(cloned)
                audit_proof_applied = matched_proof["proof_id"]
            else:
                upgraded_vectors.append(vec)
        else:
            upgraded_vectors.append(vec)

    attr_tns = npc.get("attribute_tns") if isinstance(npc.get("attribute_tns"), dict) else {}
    dis = int(attr_tns.get("dis") or npc.get("dis") or 12)
    foc = int(attr_tns.get("foc") or npc.get("foc") or 12)
    prior_state = _SESSION_SOCIAL_COMPOSURE.get((session_id, npc_key))
    outcome = _psyche.evaluate_social_exchange(
        psyche_payload,
        upgraded_vectors,
        int(net_successes),
        prior_state,
        dis,
        foc,
        ego_wound_dis_check_passed,
    )
    if isinstance(outcome.get("updated_state"), dict):
        _SESSION_SOCIAL_COMPOSURE[(session_id, npc_key)] = dict(outcome["updated_state"])
        upd = outcome["updated_state"]
        if upd.get("position_demolished") or upd.get("lashed_out_from_ego_wound") or int(outcome.get("dissonance_stacks_added", 0)) > 0:
            from satya_ml.ai_gm.tools import get_session_npcs, update_npc_state

            if npc_key in get_session_npcs(session_id):
                new_disp = (
                    "hostile"
                    if upd.get("lashed_out_from_ego_wound")
                    else ("shaken_yielding" if upd.get("position_demolished") else "conflicted")
                )
                update_npc_state.invoke(
                    {
                        "npc_id": npc_key,
                        "disposition": new_disp,
                        "session_id": session_id,
                    }
                )

    standing_info = compute_faction_standing_social_leverage(session_id, npc_for_standing)
    arg_eval = outcome.setdefault("argument_eval", {})
    base_delta_ext = int(arg_eval.get("net_delta_ext", 0) or 0)
    standing_delta_ext = int(standing_info["standing_delta_ext"])
    total_delta_ext = base_delta_ext + standing_delta_ext
    if standing_delta_ext != 0:
        arg_eval["base_net_delta_ext"] = base_delta_ext
        arg_eval["net_delta_ext"] = total_delta_ext

    outcome["audit_proof_applied"] = audit_proof_applied
    outcome["matched_faction_id"] = standing_info["matched_faction_id"]
    outcome["faction_standing"] = standing_info["party_standing"]
    outcome["faction_standing_delta_ext"] = standing_delta_ext
    outcome["total_delta_ext"] = total_delta_ext
    outcome["target_npc_id"] = npc_key
    outcome["target_name"] = str(npc.get("name") or target_npc_id_or_name)
    return outcome


class _PsycheTargetRequest(ContextToolRequest[str], ABC):
    """Abstract base for session-bound Psyche resolution modes targeting an NPC or PC."""

    target_id: str = ""

    def npc_key(self) -> str:
        return self.target_id.strip().lower()


class DissonanceRequest(_PsycheTargetRequest):
    mode: Literal["dissonance"] = "dissonance"
    resolution_path: str = "compartmentalization"
    violated_trait: str = "OPN"
    dis: int = 12
    dis_check_successes: int = 1

    def execute(self, session_id: str, /) -> dict[str, Any]:
        key = self.npc_key()
        composure = _SESSION_SOCIAL_COMPOSURE.get(
            (session_id, key),
            {
                "max_cmp": max(1, (self.dis + 12) // 4),
                "current_cmp": 0,
                "doubt_fractures": 1,
                "dissonance_index": 2,
                "accumulated_delta_int": 2,
                "focus_penalty": 0,
                "position_demolished": False,
                "lashed_out_from_ego_wound": False,
            },
        )
        res = _psyche.resolve_cognitive_dissonance(
            composure,
            int(self.dis),
            self.resolution_path,
            self.violated_trait or None,
            int(self.dis_check_successes),
        )
        if isinstance(res.get("updated_state"), dict) and key:
            _SESSION_SOCIAL_COMPOSURE[(session_id, key)] = dict(res["updated_state"])
        return res


class ConvictionSurgeRequest(_PsycheTargetRequest):
    mode: Literal["conviction_surge"] = "conviction_surge"
    psyche_json: str = ""
    current_delta_int: int = 4
    principle: str = ""

    def execute(self, session_id: str, /) -> dict[str, Any]:
        key = self.npc_key()
        surge_key = (session_id, key or "party")
        already_used = _SESSION_CONVICTION_SURGES.get(surge_key, False)
        raw_p = self.psyche_json.strip()
        eff_psyche: dict[str, Any] | str = (
            parse_json_obj(raw_p) or raw_p
            if raw_p
            else _parse_psyche_dict(_resolve_npc_profile(self.target_id, session_id))
        )
        res = _psyche.evaluate_conviction_surge(
            eff_psyche,
            int(self.current_delta_int),
            already_used,
            self.principle or None,
        )
        if res.get("eligible"):
            _SESSION_CONVICTION_SURGES[surge_key] = True
        return res


class PsycheDriftMarkRequest(_PsycheTargetRequest):
    mode: Literal["drift_mark"] = "drift_mark"
    violated_trait: str = "OPN"
    dis: int = 12
    trait_score: int = 10
    marks_added: int = 1

    def execute(self, session_id: str, /) -> dict[str, Any]:
        trait_key = (self.violated_trait or "OPN").strip().upper()
        drift_key = (session_id, self.npc_key() or "party", trait_key)
        cur_marks = _SESSION_PSYCHE_DRIFT_MARKS.get(drift_key, 0)
        res = _psyche.apply_psyche_drift_mark(
            int(self.trait_score),
            cur_marks,
            int(self.marks_added),
            int(self.dis),
            None,
        )
        _SESSION_PSYCHE_DRIFT_MARKS[drift_key] = int(
            res.get("remaining_drift_marks", 0) or 0
        )
        return res


type PsycheResolutionUnion = DissonanceRequest | ConvictionSurgeRequest | PsycheDriftMarkRequest
PsycheResolutionRequest = Annotated[
    DissonanceRequest | ConvictionSurgeRequest | PsycheDriftMarkRequest,
    Field(discriminator="mode"),
]


class PsycheResolutionToolInput(UnionToolInput):
    _mode_aliases: ClassVar[Mapping[str, PsycheResolutionMode]] = MappingProxyType(
        {
            "surge": "conviction_surge",
            "drift": "drift_mark",
        }
    )
    _default_mode: ClassVar[str] = "dissonance"
    _mode_case: ClassVar[Literal["upper", "lower"]] = "lower"

    request: PsycheResolutionRequest


@satya_gm_tool(
    pack="social_and_psyche",
    tier="TIER_2_RULESET_ENGINE",
    args_schema=PsycheResolutionToolInput,
)
def resolve_psyche_dissonance_or_surge(
    request: PsycheResolutionUnion,
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Cognitive Dissonance paths (`dissonance`: `conversion_defection`, `dogmatic_denial`, `compartmentalization`, `emotional_lashing_out` across the 9 Psyche traits `EGO, SLF, OPN, BLG, EXT, CNF, AGG, TRS, CPS`), dramatic `Conviction Surge` (`conviction_surge`), or `Psyche Drift Marks` (`drift_mark`)."""
    return request.execute(session_id or "default")


@satya_gm_tool(pack="world_and_director", tier="TIER_2_RULESET_ENGINE")
def get_world_clocks_and_heat(session_id: str = "default") -> dict[str, Any]:
    """Inspect current Operational `Heat` (`0-10`), sector jurisdiction response profile, and faction progress clocks."""
    return get_session_world_ledger(session_id)


@satya_gm_tool(pack="world_and_director", tier="TIER_2_RULESET_ENGINE")
def advance_faction_or_heat_clock(
    faction_id: str = "",
    clock_delta: int = 0,
    standing_delta: int = 0,
    heat_delta: int = 0,
    incident_severity: str = "",
    economic_tier: str = "",
    zoning: str = "",
    public_rumor: str = "",
    gm_secret_agenda: str = "",
    reason: str = "",
    session_id: str = "default",
) -> dict[str, Any]:
    """Advance or shift Operational Heat (`0-10`), sector jurisdiction, party faction standing, or a faction clock."""
    req: dict[str, Any] = {}
    if faction_id.strip():
        req["faction_id"] = faction_id.strip()
    if clock_delta:
        req["clock_delta"] = int(clock_delta)
    if standing_delta:
        req["standing_delta"] = int(standing_delta)
    if heat_delta:
        req["heat_delta"] = int(heat_delta)
    if incident_severity.strip():
        req["incident_severity"] = incident_severity.strip()
    if economic_tier.strip():
        req["economic_tier"] = economic_tier.strip()
    if zoning.strip():
        req["zoning"] = zoning.strip()
    if public_rumor.strip():
        req["public_rumor"] = public_rumor.strip()
    if gm_secret_agenda.strip():
        req["gm_secret_agenda"] = gm_secret_agenda.strip()
    if reason.strip():
        req["reason"] = reason.strip()
    return update_session_world_ledger(session_id, req)
