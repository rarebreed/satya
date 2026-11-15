"""Embodied AI Player Motivation Ledger, Self-Only Spatial Navigation, Public Lore Lookup,
and Mechanically Gated Empathy Read tools (`satya_ml.ai_player.motivation_ledger`).

Every function strictly preserves human-GM secret isolation: `GM_ONLY` map tokens,
`spatial_pins`, `GmDocument`, `GmMemory`, NPC `gm_secret_dossier`, and faction
`gm_secret_agenda` are stripped before inspection, formatting, or tool return.
"""

from __future__ import annotations

import copy
import json
import math
import re
import urllib.parse
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


from satya_ml.campaign_config import get_pack_starter_npcs
from satya_ml.dice_probabilities import _ensure_satya_py_loaded

_PLAYER_MOTIVATION_LEDGERS: dict[tuple[str, str], dict[str, Any]] = {}

_PSYCHE_POLE_HINTS: dict[str, tuple[str, str, str]] = {
    "opn": ("OPN (Openness)", "Dogmatic / Pragmatic Traditionalist", "Idealistic / Novelty-Seeking Truth-Seeker"),
    "slf": ("SLF (Selfishness)", "Altruistic / Self-Sacrificing Protector", "Self-Preserving / Opportunistic"),
    "blg": ("BLG (Belonging)", "Lonely Outsider / Detachment", "Fiercely Loyal to Crew & Community"),
    "agg": ("AGG (Aggression)", "Conciliatory / Harm-Averse Diplomat", "Confrontational / Escalation-Prone"),
    "ego": ("EGO (Pride/Status)", "Self-Effacing / Humble", "Proud / Legacy- and Reputation-Driven"),
    "cnf": ("CNF (Conformity)", "Iconoclastic / Anti-Authoritarian Rebel", "Protocol-Bound / Dutifully Lawful"),
    "ext": ("EXT (Extraversion)", "Reserved / Guarded Observer", "Expressive / Socially Energized"),
    "trs": ("TRS (Trust)", "Cynical / Zero-Trust / Paranoid", "Trusting / Open-Handed"),
    "cps": ("CPS (Compassion)", "Ruthless / Utilitarian", "Merciful / Tender-Hearted Protector"),
}


def reset_player_motivation_ledgers(session_id: str | None = None) -> None:
    """Clear persisted AI Player `MotivationLedger` states for `session_id` or all sessions."""
    if session_id is None:
        _PLAYER_MOTIVATION_LEDGERS.clear()
        return
    for key in [k for k in _PLAYER_MOTIVATION_LEDGERS if k[0] == session_id]:
        _PLAYER_MOTIVATION_LEDGERS.pop(key, None)


def redact_public_npc_entry(raw_npc: dict[str, Any]) -> dict[str, Any]:
    """Strip all GM-only dossier, secret agenda, core drive, wedge, and raw stat fields."""
    return {
        "npc_id": str(raw_npc.get("character_id") or raw_npc.get("npc_id") or ""),
        "name": str(raw_npc.get("character_name") or raw_npc.get("name") or "Unknown NPC"),
        "role": str(raw_npc.get("callsign_or_role") or raw_npc.get("archetype") or "NPC"),
        "faction": str(raw_npc.get("faction_id") or raw_npc.get("faction") or "Independent"),
        "disposition": str(raw_npc.get("disposition") or "wary"),
        "public_summary": str(raw_npc.get("public_summary") or raw_npc.get("notes") or ""),
        "apparent_condition": str(raw_npc.get("apparent_condition") or "Healthy & Alert"),
    }


def redact_world_ledger_for_player(raw_ledger: dict[str, Any] | None) -> dict[str, Any]:
    """Strip `gm_secret_agenda` from every faction clock in a world ledger snapshot."""
    if not isinstance(raw_ledger, dict) or not raw_ledger:
        return {}
    clean = copy.deepcopy(raw_ledger)
    clean_clocks: list[dict[str, Any]] = []
    for clk in clean.get("faction_clocks") or []:
        if not isinstance(clk, dict):
            continue
        item = {k: v for k, v in clk.items() if k != "gm_secret_agenda"}
        clean_clocks.append(item)
    clean["faction_clocks"] = clean_clocks
    return clean


def sanitize_map_dict_for_player(raw_map: dict[str, Any] | None) -> dict[str, Any] | None:
    """Preserve public map geometry/exits while strictly removing `GM_ONLY` tokens and `spatial_pins`."""
    if not isinstance(raw_map, dict) or not raw_map.get("map_id"):
        return None
    cell_cm = max(25, int(raw_map.get("cell_size_cm", 100) or 100))
    public_tokens: list[dict[str, Any]] = []
    hidden_ids: set[str] = set()
    for t in raw_map.get("tokens") or []:
        if not isinstance(t, dict):
            continue
        vis = str(t.get("visibility") or "PUBLIC").upper()
        if t.get("hidden") or t.get("is_hidden") or t.get("gm_only") or vis == "GM_ONLY":
            for id_key in ("entity_id", "token_id"):
                if t.get(id_key):
                    hidden_ids.add(str(t[id_key]))
            continue
        eid = str(t.get("entity_id") or t.get("token_id") or "")
        gx = int(t.get("x") if t.get("x") is not None else t.get("grid_x", 0))
        gy = int(t.get("y") if t.get("y") is not None else t.get("grid_y", 0))
        public_tokens.append(
            {
                "token_id": str(t.get("token_id") or eid),
                "entity_id": eid,
                "label": str(t.get("label") or eid),
                "x": gx,
                "y": gy,
                "grid_x": gx,
                "grid_y": gy,
                "pos_x_cm": t.get("pos_x_cm") if t.get("pos_x_cm") is not None else gx * cell_cm + cell_cm // 2,
                "pos_y_cm": t.get("pos_y_cm") if t.get("pos_y_cm") is not None else gy * cell_cm + cell_cm // 2,
                "faction": str(t.get("faction") or "NEUTRAL"),
                "visibility": "PUBLIC",
            }
        )

    clean_visuals = [
        dict(v)
        for v in (raw_map.get("token_visuals") or [])
        if isinstance(v, dict) and str(v.get("entity_id") or "") not in hidden_ids
    ]
    raw_manifest = raw_map.get("narrative_manifest")
    clean_manifest = (
        {k: copy.deepcopy(v) for k, v in raw_manifest.items() if k != "spatial_pins"}
        if isinstance(raw_manifest, dict)
        else None
    )
    out: dict[str, Any] = {
        "map_id": str(raw_map.get("map_id")),
        "title": str(raw_map.get("title") or "Tactical Sector"),
        "theme": str(raw_map.get("theme") or raw_map.get("theme_preset") or "urban_crossroads"),
        "theme_preset": raw_map.get("theme_preset"),
        "seed": raw_map.get("seed", 42),
        "grid_width": int(raw_map.get("grid_width", 16) or 16),
        "grid_height": int(raw_map.get("grid_height", 12) or 12),
        "cell_size_cm": cell_cm,
        "active_floor": int(raw_map.get("active_floor", 0) or 0),
        "tokens": public_tokens,
        "token_visuals": clean_visuals,
    }
    for geom_key in ("tiles", "wall_edges", "surfaces", "props", "floors", "world_sectors", "image_url", "notes"):
        if raw_map.get(geom_key) is not None:
            out[geom_key] = copy.deepcopy(raw_map[geom_key])
    if clean_manifest is not None:
        out["narrative_manifest"] = clean_manifest
    return out


def _extract_psyche_scores(sheet: dict[str, Any] | None) -> dict[str, int]:
    scores = {k: 10 for k in _PSYCHE_POLE_HINTS}
    if not isinstance(sheet, dict):
        return scores
    for container_key in ("characteristics", "psyche_traits", "psyche"):
        bucket = sheet.get(container_key)
        if not isinstance(bucket, dict):
            continue
        for k, v in bucket.items():
            lk = str(k).strip().lower()
            if lk not in scores or not isinstance(v, (int, float)):
                continue
            iv = int(v)
            # Support legacy signed deltas (-10..+10) cleanly while defaulting to canonical 1..=20
            if container_key == "psyche_traits" and -10 <= iv <= 10 and "characteristics" not in sheet:
                scores[lk] = max(1, min(20, 10 + iv))
            else:
                scores[lk] = max(1, min(20, iv))
    return scores


def ensure_or_sync_motivation_ledger(
    session_id: str,
    player_user_id: str,
    character_sheet: dict[str, Any] | None,
    persona_prompt: str = "",
) -> dict[str, Any]:
    """Initialize or refresh the persistent per-player `MotivationLedger` without clobbering runtime notes."""
    key = (session_id, player_user_id)
    cname = str((character_sheet or {}).get("character_name") or "Operative")
    psyche_scores = _extract_psyche_scores(character_sheet)
    extremes: list[dict[str, Any]] = []
    for axis, score in psyche_scores.items():
        if score <= 6 or score >= 15:
            label, low_pole, high_pole = _PSYCHE_POLE_HINTS[axis]
            extremes.append(
                {
                    "axis": axis.upper(),
                    "label": label,
                    "score": score,
                    "pole_hint": low_pole if score <= 6 else high_pole,
                }
            )
    extremes.sort(key=lambda x: abs(int(x["score"]) - 10), reverse=True)

    from satya_ml.ai_player.psyche_anchors import (
        format_9axis_and_lifepath_prompt_lines,
        sync_character_9axis_and_lifepath_anchors,
    )

    existing = _PLAYER_MOTIVATION_LEDGERS.get(key)
    if existing is not None:
        existing["character_name"] = cname
        existing["psyche_scores"] = psyche_scores
        if extremes:
            existing["psyche_extremes"] = extremes
        sync_character_9axis_and_lifepath_anchors(existing, character_sheet)
        return copy.deepcopy(existing)

    bg_goal = str((character_sheet or {}).get("background_summary") or "").strip()
    mots = (character_sheet or {}).get("motivations")
    if isinstance(mots, list) and mots:
        first_m = mots[0]
        default_arc = (
            str(first_m.get("description") or first_m.get("title") or "")
            if isinstance(first_m, dict)
            else str(first_m)
        )
    elif bg_goal:
        default_arc = bg_goal[:180]
    elif persona_prompt.strip():
        default_arc = persona_prompt.strip()[:180]
    else:
        default_arc = "Uncover who triggered the current crisis and protect innocent bystanders."

    ledger: dict[str, Any] = {
        "session_id": session_id,
        "player_user_id": player_user_id,
        "character_name": cname,
        "psyche_scores": psyche_scores,
        "psyche_extremes": extremes,
        "immediate_goal": "Assess the immediate scene, establish safe tactical footing, and learn what the people here are hiding.",
        "personal_arc_quest": default_arc,
        "unresolved_clues": [],
        "resolved_clues": [],
        "party_bonds_and_tensions": [],
        "psyche_strain_notes": [],
        "known_npc_reads": {},
    }
    sync_character_9axis_and_lifepath_anchors(ledger, character_sheet)
    _PLAYER_MOTIVATION_LEDGERS[key] = ledger
    return copy.deepcopy(ledger)


def format_motivation_ledger_for_prompt(ledger: dict[str, Any] | None) -> str:
    """Format an AI Player's private inner `MotivationLedger` for context injection."""
    if not isinstance(ledger, dict) or not ledger:
        return ""
    lines = ["My Inner Motivation Ledger:"]
    lines.append(f"- Immediate Goal: {ledger.get('immediate_goal')}")
    lines.append(f"- Personal Arc: {ledger.get('personal_arc_quest')}")
    ext = ledger.get("psyche_extremes") or []
    if ext:
        ext_str = "; ".join(f"{e['axis']}={e['score']} ({e['pole_hint']})" for e in ext[:4])
        lines.append(f"- Defining 1-20 Psyche Poles: {ext_str}")
    clues = ledger.get("unresolved_clues") or []
    if clues:
        lines.append(f"- Unresolved Clues: {'; '.join(str(c) for c in clues[:5])}")
    bonds = ledger.get("party_bonds_and_tensions") or []
    if bonds:
        lines.append(f"- Party Bonds & Tensions: {'; '.join(str(b) for b in bonds[-3:])}")
    reads = ledger.get("known_npc_reads") or {}
    if reads:
        read_parts = [f"{npc}: {', '.join(str(t) for t in traits[:3])}" for npc, traits in list(reads.items())[:4]]
        lines.append(f"- Known NPC Empathy Reads: {'; '.join(read_parts)}")
    from satya_ml.ai_player.psyche_anchors import format_9axis_and_lifepath_prompt_lines as _fmt_9axis

    lines.extend(_fmt_9axis(ledger))
    return "\n".join(lines)


def _find_self_token(
    tokens: list[dict[str, Any]],
    player_user_id: str,
    character_sheet: dict[str, Any] | None,
) -> dict[str, Any] | None:
    cid = str((character_sheet or {}).get("character_id") or "")
    cname = str((character_sheet or {}).get("character_name") or "").lower()
    for tok in tokens:
        if not isinstance(tok, dict):
            continue
        eid = str(tok.get("entity_id") or "")
        tid = str(tok.get("token_id") or "")
        lbl = str(tok.get("label") or "").lower()
        if eid == player_user_id or tid == player_user_id:
            return tok
        if cid and (eid == cid or tid == cid):
            return tok
        if cname and cname in lbl:
            return tok
    return None


def summarize_public_map_for_player(
    active_map: dict[str, Any] | None,
    player_user_id: str,
    character_sheet: dict[str, Any] | None = None,
) -> str:
    """Summarize player coordinates, nearby visible tokens, rooms/zones, and cardinal exits."""
    clean_map = sanitize_map_dict_for_player(active_map)
    if not clean_map:
        return ""
    cell_cm = max(25, int(clean_map.get("cell_size_cm", 100) or 100))
    tokens = clean_map.get("tokens") or []
    self_tok = _find_self_token(tokens, player_user_id, character_sheet)
    parts = [f"Map '{clean_map['title']}' ({clean_map['grid_width']}x{clean_map['grid_height']}m, Floor {clean_map['active_floor']})"]
    if self_tok is not None:
        sx = float(self_tok.get("pos_x_cm", 50)) / 100.0
        sy = float(self_tok.get("pos_y_cm", 50)) / 100.0
        parts.append(f"My Pos: ({sx:.1f}m, {sy:.1f}m)")
        others: list[str] = []
        for t in tokens:
            if t is self_tok:
                continue
            tx = float(t.get("pos_x_cm", 50)) / 100.0
            ty = float(t.get("pos_y_cm", 50)) / 100.0
            dist = math.hypot(tx - sx, ty - sy)
            others.append(f"{t.get('label')}@({tx:.1f},{ty:.1f}, {dist:.1f}m)")
        if others:
            parts.append("Visible Tokens: " + ", ".join(others[:6]))
    manifest = clean_map.get("narrative_manifest") or {}
    if isinstance(manifest, dict):
        zones = [str(z.get("label") or z.get("category") or "") for z in (manifest.get("zones") or [])[:4] if isinstance(z, dict)]
        zones = [z for z in zones if z]
        if zones:
            parts.append("Zones: " + ", ".join(zones))
        exits = [
            f"{e.get('direction')}->{e.get('preview_label')}"
            for e in (manifest.get("cardinal_exits") or [])[:4]
            if isinstance(e, dict) and e.get("direction") and e.get("preview_label")
        ]
        if exits:
            parts.append("Exits: " + ", ".join(exits))
    return " | ".join(parts)


@tool
def move_player_map_token(
    target_x: float,
    target_y: float,
    to_floor: int | None = None,
    stance_pose: str = "",
    session_id: str = "default",
    player_user_id: str = "ai-player-1",
) -> dict[str, Any]:
    """Move ONLY your own character miniature to `(target_x, target_y)` meters with wall-collision enforcement."""
    from satya_ml.ai_player.tools import (
        _http_post_json,
        _signaling_base_url,
        get_player_session_context,
        set_player_session_context,
    )

    ctx = get_player_session_context(session_id, player_user_id)
    sheet = ctx.get("character_sheet") or {}
    raw_map = ctx.get("active_map")
    clean_map = sanitize_map_dict_for_player(raw_map)
    self_tok = _find_self_token((clean_map or {}).get("tokens") or [], player_user_id, sheet)
    entity_id = str((self_tok or {}).get("entity_id") or player_user_id)

    req_x_cm = int(round(float(target_x) * 100.0))
    req_y_cm = int(round(float(target_y) * 100.0))
    payload: dict[str, Any] = {
        "entity_id": entity_id,
        "to_x": int(math.floor(float(target_x))),
        "to_y": int(math.floor(float(target_y))),
        "to_x_cm": req_x_cm,
        "to_y_cm": req_y_cm,
    }
    if to_floor is not None:
        payload["to_floor"] = int(to_floor)
    if stance_pose.strip():
        payload["stance_pose"] = stance_pose.strip()

    remote = _http_post_json(
        f"{_signaling_base_url()}/api/player-ai/sessions/{session_id}/map/move?user_id={urllib.parse.quote(player_user_id)}",
        payload,
        timeout=3.0,
    )
    if isinstance(remote, dict) and remote.get("map_id"):
        sanitized = sanitize_map_dict_for_player(remote)
        if sanitized:
            ctx["active_map"] = sanitized
            set_player_session_context(session_id, player_user_id, ctx)
            return {"status": "moved", "entity_id": entity_id, "x_m": target_x, "y_m": target_y, "active_map": sanitized}

    if not clean_map:
        return {"error": "No active tactical map in session context."}

    satya_py = _ensure_satya_py_loaded()
    final_x_cm, final_y_cm = req_x_cm, req_y_cm
    if hasattr(satya_py, "resolve_tactical_token_destination_cm") and clean_map.get("wall_edges") is not None:
        try:
            final_x_cm, final_y_cm = satya_py.resolve_tactical_token_destination_cm(
                json.dumps(clean_map),
                entity_id,
                req_x_cm,
                req_y_cm,
            )
        except Exception as exc:
            return {"error": f"Movement blocked: {exc}"}

    cell_cm = max(25, int(clean_map.get("cell_size_cm", 100) or 100))
    gx = max(0, min(int(clean_map["grid_width"]) - 1, final_x_cm // cell_cm))
    gy = max(0, min(int(clean_map["grid_height"]) - 1, final_y_cm // cell_cm))
    updated = False
    for tok in clean_map["tokens"]:
        if tok.get("entity_id") == entity_id or tok.get("token_id") == entity_id:
            tok["x"] = gx
            tok["y"] = gy
            tok["grid_x"] = gx
            tok["grid_y"] = gy
            tok["pos_x_cm"] = final_x_cm
            tok["pos_y_cm"] = final_y_cm
            updated = True
            break
    if not updated:
        clean_map["tokens"].append(
            {
                "token_id": entity_id,
                "entity_id": entity_id,
                "label": str(ctx.get("player_display_name") or entity_id),
                "x": gx,
                "y": gy,
                "grid_x": gx,
                "grid_y": gy,
                "pos_x_cm": final_x_cm,
                "pos_y_cm": final_y_cm,
                "visibility": "PUBLIC",
            }
        )
    ctx["active_map"] = clean_map
    set_player_session_context(session_id, player_user_id, ctx)
    return {
        "status": "moved_offline",
        "entity_id": entity_id,
        "x_m": round(final_x_cm / 100.0, 2),
        "y_m": round(final_y_cm / 100.0, 2),
        "grid_x": gx,
        "grid_y": gy,
        "active_map": clean_map,
    }


@tool
def inspect_surroundings(
    focus_target: str = "",
    x_m: float | None = None,
    y_m: float | None = None,
    session_id: str = "default",
    player_user_id: str = "ai-player-1",
) -> dict[str, Any]:
    """Inspect nearby rooms, props, cover/LoS, and visible tokens on the `Player`-filtered tactical map."""
    from satya_ml.ai_player.tools import get_player_session_context

    ctx = get_player_session_context(session_id, player_user_id)
    clean_map = sanitize_map_dict_for_player(ctx.get("active_map"))
    if not clean_map:
        return {"error": "No public tactical map available in current scene."}

    sheet = ctx.get("character_sheet") or {}
    tokens = clean_map.get("tokens") or []
    self_tok = _find_self_token(tokens, player_user_id, sheet)
    px_m = float(x_m) if x_m is not None else (float((self_tok or {}).get("pos_x_cm", 250)) / 100.0)
    py_m = float(y_m) if y_m is not None else (float((self_tok or {}).get("pos_y_cm", 250)) / 100.0)

    satya_py = _ensure_satya_py_loaded()
    map_json = json.dumps(clean_map)
    point_info = None
    manifest: dict[str, Any] = dict(clean_map.get("narrative_manifest") or {})
    if hasattr(satya_py, "inspect_tactical_map_point"):
        try:
            point_info = satya_py.inspect_tactical_map_point(
                map_json,
                str(clean_map.get("theme") or "urban_crossroads"),
                px_m,
                py_m,
            )
        except Exception:
            point_info = None
    if not manifest and hasattr(satya_py, "extract_sector_narrative_manifest_json"):
        try:
            manifest = json.loads(satya_py.extract_sector_narrative_manifest_json(map_json))
        except Exception:
            manifest = {}
    manifest.pop("spatial_pins", None)

    visible_contacts: list[dict[str, Any]] = []
    for tok in tokens:
        if tok is self_tok:
            continue
        lbl = str(tok.get("label") or tok.get("entity_id") or "")
        if focus_target.strip() and focus_target.strip().lower() not in lbl.lower():
            continue
        tx_m = float(tok.get("pos_x_cm", 150)) / 100.0
        ty_m = float(tok.get("pos_y_cm", 150)) / 100.0
        los_cover = None
        if hasattr(satya_py, "evaluate_tactical_los_and_cover"):
            try:
                los_cover = satya_py.evaluate_tactical_los_and_cover(map_json, px_m, py_m, tx_m, ty_m)
            except Exception:
                los_cover = None
        visible_contacts.append(
            {
                "entity_id": tok.get("entity_id"),
                "label": lbl,
                "x_m": round(tx_m, 2),
                "y_m": round(ty_m, 2),
                "distance_m": round(math.hypot(tx_m - px_m, ty_m - py_m), 2),
                "los_and_cover": los_cover,
            }
        )

    return {
        "map_id": clean_map.get("map_id"),
        "title": clean_map.get("title"),
        "observer_m": {"x_m": round(px_m, 2), "y_m": round(py_m, 2)},
        "point_under_cursor": point_info,
        "zones": manifest.get("zones", [])[:6],
        "cardinal_exits": manifest.get("cardinal_exits", [])[:4],
        "visible_tokens": visible_contacts,
    }


@tool
def lookup_public_lore_and_npcs(
    query: str,
    scope: str = "ALL",
    session_id: str = "default",
    player_user_id: str = "ai-player-1",
) -> dict[str, Any]:
    """Search Satya public rules, visible NPC public summaries, and public faction rumors (never GM secrets)."""
    from satya_ml.ai_player.tools import _http_get_json, _signaling_base_url, get_player_session_context

    q_param = urllib.parse.quote(query.strip())
    s_param = urllib.parse.quote(scope.strip() or "ALL")
    u_param = urllib.parse.quote(player_user_id.strip())
    remote = _http_get_json(
        f"{_signaling_base_url()}/api/player-ai/sessions/{session_id}/lookup?user_id={u_param}&q={q_param}&scope={s_param}"
    )
    if isinstance(remote, dict) and isinstance(remote.get("hits"), list):
        return remote

    ctx = get_player_session_context(session_id, player_user_id)
    satya_py = _ensure_satya_py_loaded()
    norm_scope = (scope or "ALL").strip().upper()
    q_low = query.strip().lower()
    hits: list[dict[str, Any]] = []

    if norm_scope in {"ALL", "RULES", "RULES_BOOK"} and hasattr(satya_py, "search_rules_book"):
        for r in satya_py.search_rules_book(query.strip() or "cascade", 4):
            hits.append(
                {
                    "source_kind": "RULES_BOOK",
                    "item_id": r.chapter_id,
                    "title": f"{r.chapter_title} — {r.section_heading}",
                    "snippet": str(r.snippet)[:360],
                }
            )

    if norm_scope in {"ALL", "NPC", "NPCS", "SESSION_NPC", "CHARACTERS", "LORE"}:
        seen: set[str] = set()
        candidates = list(ctx.get("public_npcs") or [])
        for cid, npc in get_pack_starter_npcs().items():
            candidates.append({"character_id": cid, **npc})
        for raw in candidates:
            pub = redact_public_npc_entry(raw)
            cid = pub["npc_id"] or pub["name"]
            if cid in seen:
                continue
            seen.add(cid)
            hay = f"{pub['name']} {pub['role']} {pub['faction']} {pub['public_summary']}".lower()
            if not q_low or any(w in hay for w in q_low.split() if len(w) >= 2):
                hits.append(
                    {
                        "source_kind": "SESSION_NPC",
                        "item_id": cid,
                        "title": pub["name"],
                        "subtitle": f"{pub['role']} · {pub['faction']} · {pub['apparent_condition']}",
                        "snippet": pub["public_summary"][:360],
                    }
                )

    if norm_scope in {"ALL", "CLOCKS", "FACTIONS", "WORLD", "HEAT", "LORE"}:
        ledger = redact_world_ledger_for_player(ctx.get("world_ledger"))
        for clk in ledger.get("faction_clocks") or []:
            title = f"{clk.get('faction_name')} — {clk.get('clock_title')}"
            hay = f"{title} {clk.get('public_rumor', '')}".lower()
            if not q_low or any(w in hay for w in q_low.split() if len(w) >= 2):
                hits.append(
                    {
                        "source_kind": "CAMPAIGN_LORE",
                        "item_id": f"faction-clock:{clk.get('faction_id')}",
                        "title": title,
                        "subtitle": f"Faction Clock ({clk.get('progress')}/{clk.get('segments')})",
                        "snippet": str(clk.get("public_rumor") or ""),
                    }
                )

    return {"session_id": session_id, "query": query, "scope": norm_scope, "hits": hits[:12]}


def _lookup_npc_psyche_only_for_empathy(
    target_npc_name: str,
    session_id: str,
) -> dict[str, int]:
    """Read target NPC `1..=20` Psyche traits ONLY for canonical `evaluate_empathy_read_json` resolution."""
    from satya_ml.ai_gm.tools import get_session_npcs

    needle = target_npc_name.strip().lower()
    pool = dict(get_session_npcs(session_id))
    for cid, npc in get_pack_starter_npcs().items():
        pool.setdefault(cid, npc)
    matched: dict[str, Any] | None = None
    for cid, npc in pool.items():
        if needle == cid or needle in cid or needle in str(npc.get("name") or "").lower():
            matched = npc
            break
    out = {k: 10 for k in _PSYCHE_POLE_HINTS}
    if matched is None:
        return out
    for bucket_key in ("psyche", "characteristics", "psyche_traits"):
        bucket = matched.get(bucket_key)
        if isinstance(bucket, dict):
            for k, v in bucket.items():
                lk = str(k).strip().lower()
                if lk in out and isinstance(v, (int, float)):
                    out[lk] = max(1, min(20, int(v)))
    for entry in matched.get("psyche_summary") or []:
        for m in re.finditer(r"\b(OPN|SLF|BLG|AGG|EGO|CNF|EXT|TRS|CPS)(?:\s*[:=]\s*|\s+)(\d+)", str(entry), re.I):
            out[m.group(1).lower()] = max(1, min(20, int(m.group(2))))
    return out


@tool
def perform_player_empathy_read(
    target_npc_name: str,
    net_successes: int | None = None,
    session_id: str = "default",
    player_user_id: str = "ai-player-1",
) -> dict[str, Any]:
    """Attempt an `INS / ITU` Empathy & Read Person check on an NPC; unlocks Psyche traits ONLY on net success."""
    from satya_ml.ai_player.tools import get_player_session_context, roll_player_d20_action

    ctx = get_player_session_context(session_id, player_user_id)
    sheet = ctx.get("character_sheet") or {}
    attr_tns = sheet.get("attribute_tns") or {}
    skills = sheet.get("skills") or {}
    roll_res: dict[str, Any] | None = None

    if net_successes is None:
        roll_res = roll_player_d20_action.invoke(
            {
                "check_category": "INS / ITU Empathy & Read Person",
                "attribute_tn": int(attr_tns.get("ins", 12) or 12),
                "skill_rank": max(1, int(skills.get("Empathy", skills.get("Intuition", 2)) or 2)),
                "note": f"Reading {target_npc_name}'s demeanor and intentions",
                "session_id": session_id,
                "player_user_id": player_user_id,
            }
        )
        raw_suc = roll_res.get("total_successes")
        if raw_suc is None:
            raw_suc = roll_res.get("successes_counted", 0)
        eff_successes = int(raw_suc or 0)
    else:
        eff_successes = int(net_successes)

    if eff_successes <= 0:
        return {
            "target_npc": target_npc_name,
            "net_successes": eff_successes,
            "revealed_traits": [],
            "detects_psyche_wedge_tension": False,
            "leverage_delta_ext_bonus": 0,
            "roll": roll_res,
            "note": "Check failed to reveal hidden Psyche motivations.",
        }

    psyche = _lookup_npc_psyche_only_for_empathy(target_npc_name, session_id)
    satya_py = _ensure_satya_py_loaded()
    res = json.loads(satya_py.evaluate_empathy_read_json(json.dumps(psyche), eff_successes))
    ledger = ensure_or_sync_motivation_ledger(session_id, player_user_id, sheet)
    trait_labels = [f"{t['code']}={t['score']}" for t in res.get("revealed_traits") or []]
    mutable_ledger = _PLAYER_MOTIVATION_LEDGERS[(session_id, player_user_id)]
    mutable_ledger["known_npc_reads"][target_npc_name] = trait_labels
    from satya_ml.ai_player.psyche_anchors import (
        apply_npc_trust_journal_entry,
        record_empathy_trust_observation,
    )

    record_empathy_trust_observation(
        mutable_ledger,
        target_npc_name,
        list(res.get("revealed_traits") or []),
        bool(res.get("detects_psyche_wedge_tension", False)),
    )
    res["target_npc"] = target_npc_name
    if roll_res is not None:
        res["roll"] = roll_res
    res["ledger_updated"] = ledger.get("character_name")
    return res


@tool
def update_my_motivation_journal(
    immediate_goal: str = "",
    add_clue: str = "",
    resolve_clue: str = "",
    party_bond_note: str = "",
    psyche_reflection: str = "",
    session_id: str = "default",
    player_user_id: str = "ai-player-1",
    npc_trust_update: str = "",
) -> dict[str, Any]:
    """Record a discovered clue, update your immediate goal, log a party bond, or record an NPC trust stance (`npc_id:stance`) in your `MotivationLedger`."""
    from satya_ml.ai_player.tools import get_player_session_context

    ctx = get_player_session_context(session_id, player_user_id)
    ensure_or_sync_motivation_ledger(
        session_id,
        player_user_id,
        ctx.get("character_sheet"),
        str(ctx.get("persona_prompt") or ""),
    )
    ledger = _PLAYER_MOTIVATION_LEDGERS[(session_id, player_user_id)]
    if immediate_goal.strip():
        ledger["immediate_goal"] = immediate_goal.strip()
    if add_clue.strip() and add_clue.strip() not in ledger["unresolved_clues"]:
        ledger["unresolved_clues"].append(add_clue.strip())
    if resolve_clue.strip():
        needle = resolve_clue.strip().lower()
        remaining = []
        for c in ledger["unresolved_clues"]:
            if needle in str(c).lower():
                ledger["resolved_clues"].append(c)
            else:
                remaining.append(c)
        ledger["unresolved_clues"] = remaining
        if resolve_clue.strip() not in ledger["resolved_clues"]:
            ledger["resolved_clues"].append(resolve_clue.strip())
    if party_bond_note.strip():
        ledger["party_bonds_and_tensions"].append(party_bond_note.strip())
    if psyche_reflection.strip():
        ledger["psyche_strain_notes"].append(psyche_reflection.strip())
    if npc_trust_update.strip():
        from satya_ml.ai_player.psyche_anchors import apply_npc_trust_journal_entry

        apply_npc_trust_journal_entry(ledger, npc_trust_update)
    return copy.deepcopy(ledger)
