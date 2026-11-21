"""AI GM granular spatial inspection, ballistics, gait reachability, and map-editing tools."""

from __future__ import annotations

from typing import Any

from langchain_core.tools import tool

from satya_py import maps as _maps


def _gm_maps_mod() -> Any:
    from satya_ml.ai_gm import maps as gm_maps

    return gm_maps


def _merge_edited_map(cur: dict[str, Any], edited: dict[str, Any], blackout_flag: bool | None = None) -> dict[str, Any]:
    updated = dict(cur)
    for key in (
        "wall_edges",
        "props",
        "surfaces",
        "floors",
        "active_floor",
        "ambient_light_delta_ext",
        "faraday_zones",
        "surveillance_nodes",
    ):
        if key in edited:
            updated[key] = edited[key]
    if blackout_flag is not None:
        updated["lighting"] = "darkness" if blackout_flag else "normal"
    return updated


def sync_map_blackout_from_polycrisis(session_id: str, blackout_active: bool) -> dict[str, Any] | None:
    """Propagate *Awakened* polycrisis grid blackouts onto the active map's lighting and cameras."""
    gm_maps = _gm_maps_mod()
    sid = session_id or "default"
    cur = gm_maps.get_session_map(sid)
    if not isinstance(cur, dict):
        return None
    res = _maps.edit_map_feature(
        gm_maps._build_rust_map_json(cur),
        {
            "action": "SET_BLACKOUT",
            "blackout_active": bool(blackout_active),
        },
    )
    edited_map = res.get("updated_map")
    if not isinstance(edited_map, dict):
        return None
    updated = _merge_edited_map(cur, edited_map, blackout_flag=bool(blackout_active))
    return gm_maps._write_and_sync_map_state(sid, updated)


@tool
def inspect_tactical_map_point(
    x_m: float,
    y_m: float,
    floor_index: int | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Inspect terrain, acoustic stealth surface, traction tier, Faraday shielding, cover, and light at `(x_m, y_m)`."""
    gm_maps = _gm_maps_mod()
    sid = session_id or "default"
    cur = gm_maps.get_session_map(sid)
    if not isinstance(cur, dict):
        return {"error": f"No tactical map exists for session '{sid}'."}
    try:
        hit = _maps.inspect_map_point(
            gm_maps._build_rust_map_json(cur),
            str(cur.get("theme", cur.get("title", "Tactical Map"))),
            float(x_m),
            float(y_m),
            floor_index=floor_index,
        )
    except Exception as exc:
        return {"error": f"Failed to inspect tactical map point: {exc}"}
    if not isinstance(hit, dict):
        return {
            "map_id": cur.get("map_id"),
            "floor_index": floor_index if floor_index is not None else int(cur.get("active_floor", 0)),
            "category": "NONE",
            "world_x_m": float(x_m),
            "world_y_m": float(y_m),
            "ambient_light_delta_ext": int(cur.get("ambient_light_delta_ext", 0) or 0),
        }
    hit["map_id"] = cur.get("map_id")
    hit["floor_index"] = floor_index if floor_index is not None else int(cur.get("active_floor", 0))
    return hit


@tool
def evaluate_ballistic_shot_on_map(
    attacker_x_m: float,
    attacker_y_m: float,
    target_x_m: float,
    target_y_m: float,
    firearm_preset_id: str = "hk416_carbine_556",
    net_successes: int = 0,
    hit_frame_or_stud: bool = False,
    floor_index: int | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Evaluate range, cover grade, multi-barrier wall/prop perforation, and residual terminal ballistics on the active map."""
    gm_maps = _gm_maps_mod()
    sid = session_id or "default"
    cur = gm_maps.get_session_map(sid)
    if not isinstance(cur, dict):
        return {"error": f"No tactical map exists for session '{sid}'."}
    try:
        res = _maps.evaluate_ballistic_shot(
            gm_maps._build_rust_map_json(cur),
            float(attacker_x_m),
            float(attacker_y_m),
            float(target_x_m),
            float(target_y_m),
            firearm_preset_id=firearm_preset_id,
            net_successes=int(net_successes),
            hit_frame_or_stud=bool(hit_frame_or_stud),
            floor_index=floor_index,
        )
    except Exception as exc:
        return {"error": f"Ballistic shot evaluation failed: {exc}"}
    res["map_id"] = cur.get("map_id")
    return res


@tool
def compute_token_reachable_zone(
    entity_id: str = "",
    x_m: float | None = None,
    y_m: float | None = None,
    spd_eff: int = 10,
    hgt_m: float = 1.75,
    gait_code: str = "COMBAT_GLIDE",
    tick_budget: int = 4,
    floor_index: int | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Compute continuous `0.25m` Dijkstra gait reachability strips for a token or map position."""
    gm_maps = _gm_maps_mod()
    sid = session_id or "default"
    cur = gm_maps.get_session_map(sid)
    if not isinstance(cur, dict):
        return {"error": f"No tactical map exists for session '{sid}'."}

    resolved_x = x_m
    resolved_y = y_m
    resolved_floor = floor_index
    eid = (entity_id or "").strip()
    if (resolved_x is None or resolved_y is None) and eid:
        for tok in cur.get("tokens") or []:
            if not isinstance(tok, dict):
                continue
            tid = str(tok.get("entity_id") or tok.get("token_id") or "")
            label = str(tok.get("label") or "")
            if eid == tid or eid.lower() == label.lower():
                px_cm = tok.get("pos_x_cm")
                py_cm = tok.get("pos_y_cm")
                if px_cm is not None and py_cm is not None:
                    resolved_x = float(px_cm) / 100.0
                    resolved_y = float(py_cm) / 100.0
                else:
                    gx = int(tok.get("grid_x", tok.get("x", 0)) or 0)
                    gy = int(tok.get("grid_y", tok.get("y", 0)) or 0)
                    resolved_x = gx + 0.5
                    resolved_y = gy + 0.5
                if resolved_floor is None and "floor_index" in tok:
                    resolved_floor = int(tok["floor_index"])
                eid = tid or eid
                break

    if resolved_x is None or resolved_y is None:
        return {"error": "Provide `(x_m, y_m)` or a valid `entity_id` present on the tactical map."}

    try:
        res = _maps.compute_reachable_zone(
            gm_maps._build_rust_map_json(cur),
            float(resolved_x),
            float(resolved_y),
            spd_eff=int(spd_eff),
            hgt_m=float(hgt_m),
            gait_code=gait_code,
            tick_budget=max(1, int(tick_budget)),
            floor_index=resolved_floor,
            exclude_entity_id=eid or None,
        )
    except Exception as exc:
        return {"error": f"Reachability query failed: {exc}"}
    res["map_id"] = cur.get("map_id")
    if eid:
        res["entity_id"] = eid
    return res


@tool
def edit_tactical_map_feature(
    action: str,
    x: int | None = None,
    y: int | None = None,
    side: str = "NORTH",
    edge_kind: str = "WALL",
    material: str = "DRYWALL",
    prop_kind: str = "",
    x_m: float | None = None,
    y_m: float | None = None,
    rotation_deg: int = 0,
    length_cm: int | None = None,
    blackout_active: bool = True,
    ambient_light_delta_ext: int | None = None,
    ordnance_code: str = "M67_FRAG",
    demolitions_net_successes: int = 0,
    apply_wall_breaches: bool = True,
    floor_index: int | None = None,
    session_id: str = "default",
) -> dict[str, Any]:
    """Mutate vector doors/walls, continuous props, district blackout state, or explosive breaches on the active map."""
    gm_maps = _gm_maps_mod()
    sid = session_id or "default"
    cur = gm_maps.get_session_map(sid)
    if not isinstance(cur, dict):
        return {"error": f"No tactical map exists for session '{sid}'."}

    norm_action = (action or "").strip().upper().replace("-", "_").replace(" ", "_")
    payload: dict[str, Any] = {"action": norm_action}
    if norm_action in {"TOGGLE_DOOR", "SET_EDGE", "REMOVE_EDGE"}:
        if x is None or y is None:
            return {"error": f"Action '{norm_action}' requires integer grid cell coordinates `(x, y)`."}
        payload.update({"x": int(x), "y": int(y), "side": side})
        if norm_action == "SET_EDGE":
            payload["kind"] = edge_kind
            payload["material"] = material
    elif norm_action == "PLACE_PROP":
        if not prop_kind.strip() or x_m is None or y_m is None:
            return {"error": "Action 'PLACE_PROP' requires `prop_kind`, `x_m`, and `y_m`."}
        payload.update(
            {
                "prop_kind": prop_kind.strip(),
                "x_m": float(x_m),
                "y_m": float(y_m),
                "rotation_deg": int(rotation_deg),
            }
        )
        if length_cm is not None:
            payload["length_cm"] = int(length_cm)
    elif norm_action == "REMOVE_PROP":
        if x_m is None or y_m is None:
            return {"error": "Action 'REMOVE_PROP' requires `x_m` and `y_m`."}
        payload.update({"x_m": float(x_m), "y_m": float(y_m)})
    elif norm_action == "SET_BLACKOUT":
        payload["blackout_active"] = bool(blackout_active)
        if ambient_light_delta_ext is not None:
            payload["ambient_light_delta_ext"] = int(ambient_light_delta_ext)
    elif norm_action == "BLAST_OR_BREACH":
        if x_m is None or y_m is None:
            return {"error": "Action 'BLAST_OR_BREACH' requires `x_m` and `y_m`."}
        payload.update(
            {
                "x_m": float(x_m),
                "y_m": float(y_m),
                "ordnance_code": ordnance_code,
                "demolitions_net_successes": int(demolitions_net_successes),
                "apply_wall_breaches": bool(apply_wall_breaches),
            }
        )
    else:
        return {"error": f"Unknown map feature action '{action}'."}

    try:
        res = _maps.edit_map_feature(
            gm_maps._build_rust_map_json(cur),
            payload,
            floor_index=floor_index,
        )
    except Exception as exc:
        return {"error": f"Map feature edit failed: {exc}"}

    edited_map = res.get("updated_map")
    if not isinstance(edited_map, dict):
        return {"error": "Rust map edit returned no updated_map payload."}

    blackout_flag = bool(blackout_active) if norm_action == "SET_BLACKOUT" else None
    updated = _merge_edited_map(cur, edited_map, blackout_flag=blackout_flag)
    gm_maps._write_and_sync_map_state(sid, updated)
    return {
        "event_type": "MAP_FEATURE_EDITED",
        "map_id": updated.get("map_id"),
        "action": res.get("action", norm_action),
        "edited_floor_index": res.get("edited_floor_index", updated.get("active_floor", 0)),
        "edit_summary": res.get("edit_summary", {}),
    }


__all__ = [
    "compute_token_reachable_zone",
    "edit_tactical_map_feature",
    "evaluate_ballistic_shot_on_map",
    "inspect_tactical_map_point",
    "sync_map_blackout_from_polycrisis",
]
