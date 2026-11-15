"""AI GM Social Action Mode (`INS/ITU` Empathy Reads, Contested Arguments, Dissonance & Surges)
and Persistent Faction Clocks x Operational `Heat` (`0-10`) World Ledger tools.

Routes deterministic Psyche/Social/Heat calculations through compiled Rust `satya_py` bindings
and syncs persistent world-ledger state with `satya-signaling` when online.
"""

from __future__ import annotations

import json
import re
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


from satya_ml.campaign_config import get_active_campaign_pack_name, get_pack_starter_npcs
from satya_ml.dice_probabilities import _ensure_satya_py_loaded

_SESSION_WORLD_LEDGERS: dict[str, dict[str, Any]] = {}
_SESSION_SOCIAL_COMPOSURE: dict[tuple[str, str], dict[str, Any]] = {}

_DEFAULT_PSYCHE: dict[str, int] = {
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


def reset_social_and_world_state(session_id: str | None = None) -> None:
    """Clear in-memory fallback world ledgers and NPC social composure states."""
    if session_id is None:
        _SESSION_WORLD_LEDGERS.clear()
        _SESSION_SOCIAL_COMPOSURE.clear()
        return
    _SESSION_WORLD_LEDGERS.pop(session_id, None)
    for key in [k for k in _SESSION_SOCIAL_COMPOSURE if k[0] == session_id]:
        _SESSION_SOCIAL_COMPOSURE.pop(key, None)


def _satya_py() -> Any:
    return _ensure_satya_py_loaded()


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
    starter = get_pack_starter_npcs()
    for cid, npc in starter.items():
        npcs.setdefault(cid, npc)

    if needle in npcs:
        return dict(npcs[needle])
    for cid, npc in npcs.items():
        if str(npc.get("name") or "").strip().lower() == needle or needle in cid or needle in str(npc.get("name") or "").lower():
            return dict(npc)
    return {"npc_id": needle or "npc-unknown", "name": target_npc_id_or_name or "NPC"}


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
    resp_json = _satya_py().seed_or_update_world_ledger_json(
        session_id,
        None,
        json.dumps(req) if req else None,
        get_active_campaign_pack_name(),
        session_id,
    )
    data = json.loads(resp_json)
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
    resp_json = _satya_py().seed_or_update_world_ledger_json(
        session_id,
        json.dumps(current),
        json.dumps(clean_req),
        get_active_campaign_pack_name(),
        session_id,
    )
    data = json.loads(resp_json)
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
    response_prof = str(ledger.get("jurisdiction_response_profile") or "")

    lines = [
        "### Active World Clocks & Jurisdiction Heat:",
        (
            f"- Operational Heat: `{heat}/10` ({tier_code}) | Public Checks `Delta_ext={delta_ext:+d}` | "
            f"Legal Accounts Frozen: `{frozen}` | Black Market `Delta_ext={bm_mod:+d}`"
        ),
        f"- Jurisdiction ({econ} x {zoning}): {response_prof}",
    ]
    for clk in ledger.get("faction_clocks") or []:
        if not isinstance(clk, dict):
            continue
        agenda = str(clk.get("gm_secret_agenda") or "").strip()
        agenda_txt = f" | [GM Secret Agenda: {agenda}]" if agenda else ""
        lines.append(
            f"- Clock `{clk.get('faction_id')}` ({clk.get('faction_name')} — {clk.get('clock_title')}): "
            f"`{clk.get('progress', 0)}/{clk.get('segments', 6)}` | Standing `{int(clk.get('party_standing', 0)):+d}` | "
            f"Rumor: {clk.get('public_rumor', '')}{agenda_txt}"
        )
    recent = ledger.get("recent_ledger_events") or []
    if recent:
        lines.append(f"- Recent World Alerts: {'; '.join(str(x) for x in recent[-3:])}")
    return "\n".join(lines)


@tool
def perform_empathy_read(
    target_npc_id_or_name: str,
    net_successes: int = 1,
    target_psyche_json: str = "",
    session_id: str = "default",
) -> dict[str, Any]:
    """Reveal `1..=20` Psyche traits (`1` trait at 1 success, `2` + wedge tension at 2, full 9 + `-2 Delta_ext` leverage at 3+)."""
    npc = _resolve_npc_profile(target_npc_id_or_name, session_id)
    psyche_str = target_psyche_json.strip() or json.dumps(_parse_psyche_dict(npc))
    res = json.loads(_satya_py().evaluate_empathy_read_json(psyche_str, int(net_successes)))
    res["target_npc_id"] = str(npc.get("npc_id") or target_npc_id_or_name)
    res["target_name"] = str(npc.get("name") or target_npc_id_or_name)
    return res


@tool
def deliver_social_argument(
    target_npc_id_or_name: str,
    argument_vectors: list[str] | str,
    net_successes: int = 1,
    target_psyche_json: str = "",
    ego_wound_dis_check_passed: bool | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve a Contested Social Exchange using Satya's 9 Argument Vectors, Composure (`CMP`), and Dual-Psyche Wedge checks.

    Supported `argument_vectors` slugs (combinable for Dual-Psyche Wedges):
    - `empirical_logic` (`OPN` evidence/logic)
    - `group_loyalty` / `betray_group` (`BLG` shared identity vs. betrayal)
    - `flattery_prestige` / `humiliate_fault` (`EGO` status vs. Ego Wound provocation)
    - `personal_profit` (`SLF >= 13` self-interest / immunity)
    - `humanitarian_mercy` (`SLF <= 7` self-sacrifice / altruistic duty)
    - `rules_protocol` / `break_law_orders` (`CNF` law, chain of command, or breaking orders)
    - `dominance_threat` / `bait_into_fight` (`AGG` intimidation vs. provoking aggression)
    - `transparency_good_faith` vs. `verified_audit_proof` (`TRS`: open olive branch vs. hard audit proof for `TRS <= 7` zero-trust targets)
    - `moral_sanctity` vs. `utilitarian_triage` (`CPS`: witnessed suffering/mercy vs. cold greater-good triage)
    """
    npc = _resolve_npc_profile(target_npc_id_or_name, session_id)
    npc_key = str(npc.get("npc_id") or target_npc_id_or_name).strip().lower()
    psyche_str = target_psyche_json.strip() or json.dumps(_parse_psyche_dict(npc))

    if isinstance(argument_vectors, str):
        stripped = argument_vectors.strip()
        if stripped.startswith("["):
            vec_list = json.loads(stripped)
        else:
            vec_list = [v.strip() for v in stripped.split(",") if v.strip()]
    else:
        vec_list = list(argument_vectors)

    attr_tns = npc.get("attribute_tns") if isinstance(npc.get("attribute_tns"), dict) else {}
    dis = int(attr_tns.get("dis") or npc.get("dis") or 12)
    foc = int(attr_tns.get("foc") or npc.get("foc") or 12)
    prior_state = _SESSION_SOCIAL_COMPOSURE.get((session_id, npc_key))
    outcome = json.loads(
        _satya_py().evaluate_social_exchange_json(
            psyche_str,
            json.dumps(vec_list),
            int(net_successes),
            json.dumps(prior_state) if prior_state else None,
            dis,
            foc,
            ego_wound_dis_check_passed,
        )
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

    outcome["target_npc_id"] = npc_key
    outcome["target_name"] = str(npc.get("name") or target_npc_id_or_name)
    return outcome


@tool
def resolve_psyche_dissonance_or_surge(
    mode: str = "dissonance",
    target_id: str = "",
    resolution_path: str = "compartmentalization",
    violated_trait: str = "OPN",
    dis: int = 12,
    dis_check_successes: int = 1,
    psyche_json: str = "",
    current_delta_int: int = 4,
    principle: str = "",
    trait_score: int = 10,
    marks_added: int = 1,
    session_id: str = "default",
) -> dict[str, Any]:
    """Resolve Cognitive Dissonance paths (`dissonance`: `conversion_defection`, `dogmatic_denial`, `compartmentalization`, `emotional_lashing_out` across the 9 Psyche traits `EGO, SLF, OPN, BLG, EXT, CNF, AGG, TRS, CPS`), dramatic `Conviction Surge` (`conviction_surge`: `altruistic_or_tribal_sacrifice`, `boundless_compassion` for `CPS >= 15`, `unbowed_defiance`, `iconoclastic_truth`), or `Psyche Drift Marks` (`drift_mark`)."""
    norm_mode = mode.strip().lower().replace("-", "_")
    npc_key = target_id.strip().lower()
    if norm_mode == "dissonance":
        composure = _SESSION_SOCIAL_COMPOSURE.get(
            (session_id, npc_key),
            {
                "max_cmp": max(1, (dis + 12) // 4),
                "current_cmp": 0,
                "doubt_fractures": 1,
                "dissonance_index": 2,
                "accumulated_delta_int": 2,
                "focus_penalty": 0,
                "position_demolished": False,
                "lashed_out_from_ego_wound": False,
            },
        )
        res = json.loads(
            _satya_py().resolve_cognitive_dissonance_json(
                json.dumps(composure),
                int(dis),
                resolution_path,
                violated_trait or None,
                int(dis_check_successes),
            )
        )
        if isinstance(res.get("updated_state"), dict) and npc_key:
            _SESSION_SOCIAL_COMPOSURE[(session_id, npc_key)] = dict(res["updated_state"])
        return res

    if norm_mode in {"conviction_surge", "surge"}:
        eff_psyche = psyche_json.strip() or json.dumps(_parse_psyche_dict(_resolve_npc_profile(target_id, session_id)))
        return json.loads(
            _satya_py().evaluate_conviction_surge_json(
                eff_psyche,
                int(current_delta_int),
                False,
                principle or None,
            )
        )

    if norm_mode in {"drift_mark", "drift"}:
        return json.loads(
            _satya_py().apply_psyche_drift_mark_json(
                int(trait_score),
                0,
                int(marks_added),
                int(dis),
                None,
            )
        )

    raise ValueError(f"Unsupported mode '{mode}'; expected 'dissonance', 'conviction_surge', or 'drift_mark'.")


@tool
def get_world_clocks_and_heat(session_id: str = "default") -> dict[str, Any]:
    """Inspect current Operational `Heat` (`0-10`), sector jurisdiction response profile, and faction progress clocks."""
    return get_session_world_ledger(session_id)


@tool
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
