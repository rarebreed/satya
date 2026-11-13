"""Procedural Continuous Vector Tactical Map Generator, Point-and-Click Movement, Analytical LoS Raycaster & VLM Inspector.

Implements the AI GM tactical spatial pipeline synchronized with `satya-engine` and `satya-web` (`wgpu` 3D/2D diorama):
- `generate_tactical_map`: Generates a continuous metric vector battlemap (`wall_edges`, 360°-rotatable `props` with
  `center_x_cm`/`center_y_cm`/`span_w_cm`/`span_h_cm`, `tiles: []`, `token_visuals`, `cell_size_cm = 100` visual scale)
  alongside a coordinate-labeled top-down SVG battlemap under `data/maps/`, and broadcasts `MAP_UPDATED` via
  `POST /api/gm/sessions/:id/map`.
- `move_map_token`: Validates continuous metric board bounds, wall/closed-door segment collisions, and rotated OBB prop
  occupancy (`0.30m` token radius), updates `pos_x_cm`/`pos_y_cm` (and derived `grid_x`/`grid_y`) and facing orientation,
  re-renders the SVG battlemap on disk, and broadcasts `MAP_UPDATED`.
- `inspect_tactical_map`: Evaluates continuous metric pairwise distances, analytical 2D segment/OBB raycast lines of sight,
  and cover `Δ_ext` (`+0` Clear, `+3` Concealment, `+4` Low Cover, `+6` High Cover, `+16` Blocked), and optionally invokes
  the session's multimodal vision model (`build_gm_chat_model(cfg, for_vision=True)`).
"""

from __future__ import annotations

import base64
import json
import math
import os
import urllib.request
import uuid
from pathlib import Path
from typing import Any

from satya_ml.ai_gm.models import SessionLlmConfig, build_gm_chat_model

try:
    import httpx
except ImportError:  # pragma: no cover
    httpx = None  # type: ignore[assignment]

try:
    from langchain_core.messages import HumanMessage
    from langchain_core.tools import tool
except ImportError:  # pragma: no cover
    HumanMessage = None  # type: ignore[assignment,misc]

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


_SESSION_MAPS: dict[str, dict[str, Any]] = {}
_SESSION_LLM_CONFIGS: dict[str, SessionLlmConfig] = {}


def _load_satya_py() -> Any | None:
    try:
        from satya_ml.dice_probabilities import _ensure_satya_py_loaded

        return _ensure_satya_py_loaded()
    except Exception:
        return None


_SATYA_PY = _load_satya_py()


def using_satya_py_map_backend() -> bool:
    """Return ``True`` when the compiled Rust ``satya_py`` spatial map backend is active."""
    return _SATYA_PY is not None and hasattr(_SATYA_PY, "evaluate_tactical_los_and_cover")

_TOKEN_RADIUS_M = 0.30
_NON_SOLID_PROP_KINDS = {
    "RUG",
    "MULCH_BED",
    "GARDEN_EDGING",
    "CREEK_WATER",
    "DRAINAGE_DITCH",
    "GRAVEL_PATH",
    "DIRT_PATH",
    "STONE_PATH",
    "ASPHALT_DRIVE",
    "ELEVATED_CATWALK",
    "HAZARD_SPILL",
}
_CONCEALMENT_PROP_KINDS = {"BUSH", "MEDIUM_TREE", "LARGE_TREE"}


def _repo_root() -> Path:
    return Path(__file__).resolve().parent.parent.parent.parent


def _signaling_base_url() -> str:
    return os.environ.get("SATYA_SIGNALING_URL", "http://127.0.0.1:3000").rstrip("/")


def _gm_service_token() -> str:
    return os.environ.get("SATYA_GM_SERVICE_TOKEN", "satya_gm_secret_dev_token")


def _gm_headers() -> dict[str, str]:
    return {
        "Authorization": f"Bearer {_gm_service_token()}",
        "Content-Type": "application/json",
    }


def get_session_map(session_id: str) -> dict[str, Any] | None:
    """Return the current tactical map state for `session_id`, if any."""
    return _SESSION_MAPS.get(session_id)


def set_session_map(session_id: str, map_state: dict[str, Any] | None) -> None:
    """Update the cached tactical map state for `session_id`."""
    if map_state is None:
        _SESSION_MAPS.pop(session_id, None)
    else:
        _SESSION_MAPS[session_id] = map_state


def set_session_llm_config(session_id: str, cfg: SessionLlmConfig) -> None:
    """Cache the active `SessionLlmConfig` for `session_id` so `inspect_tactical_map` can select the vision model."""
    _SESSION_LLM_CONFIGS[session_id] = cfg


def get_session_llm_config(session_id: str) -> SessionLlmConfig | None:
    """Return the cached `SessionLlmConfig` for `session_id`, if configured."""
    return _SESSION_LLM_CONFIGS.get(session_id)


def infer_sector_preset(theme: str) -> str:
    """Map a free-text scene theme to one of the 12 canonical `satya-engine` `SectorPreset` codes."""
    low = (theme or "").lower()
    if any(k in low for k in ("creek", "stream", "brook", "river", "ravine")):
        return "woodland_creek"
    if any(k in low for k in ("ditch", "trench", "culvert", "country lane")):
        return "roadside_ditch"
    if any(k in low for k in ("estate", "compound", "courtyard", "garden", "yard", "villa", "mansion")):
        return "garden_estate"
    if any(k in low for k in ("park", "woods", "forest", "trail", "clearing", "grove")):
        return "wooded_park"
    if any(k in low for k in ("bar", "pub", "tavern", "club", "lounge", "saloon")):
        return "corner_bar"
    if any(k in low for k in ("office", "corporate", "agency", "cubicle", "suite", "commercial")):
        return "office_floor"
    if any(k in low for k in ("apartment", "flat", "safehouse", "house", "home", "residential", "motel")):
        return "residential_apartment"
    if any(k in low for k in ("dock", "cargo", "hangar", "bay", "warehouse", "factory", "plant")):
        return "docking_bay"
    if any(k in low for k in ("med", "trauma", "lab", "clinic", "ward")):
        return "med_lab"
    if any(k in low for k in ("urban", "street", "road", "city", "crossroad")):
        return "urban_crossroads"
    if any(k in low for k in ("corridor", "derelict", "breach", "tunnel")):
        return "derelict_corridor"
    return "orbital_airlock"


def _make_prop(
    prop_id: str,
    kind: str,
    center_x_cm: int,
    center_y_cm: int,
    span_w_cm: int,
    span_h_cm: int,
    rotation_deg: int = 0,
    cover_height_cm: int = 95,
    material: str = "REINFORCED_CONCRETE",
    label: str | None = None,
) -> dict[str, Any]:
    origin_x = max(0, (center_x_cm - span_w_cm // 2) // 100)
    origin_y = max(0, (center_y_cm - span_h_cm // 2) // 100)
    w_cells = max(1, (span_w_cm + 99) // 100)
    h_cells = max(1, (span_h_cm + 99) // 100)
    return {
        "id": prop_id,
        "kind": kind,
        "x": origin_x,
        "y": origin_y,
        "w_cells": w_cells,
        "h_cells": h_cells,
        "center_x_cm": center_x_cm,
        "center_y_cm": center_y_cm,
        "span_w_cm": span_w_cm,
        "span_h_cm": span_h_cm,
        "rotation_deg": rotation_deg % 360,
        "cover_height_cm": cover_height_cm,
        "material": material,
        "label": label or kind.replace("_", " ").title(),
    }


def build_procedural_vector_scene(
    width: int,
    height: int,
    preset: str = "orbital_airlock",
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Generate thin perimeter/partition `wall_edges` and 360°-oriented `props` (zero tile grid)."""
    wall_edges: list[dict[str, Any]] = []
    props: list[dict[str, Any]] = []

    if width >= 6 and height >= 6:
        for gx in range(width):
            wall_edges.append(
                {"x": gx, "y": 0, "side": "NORTH", "kind": "WALL", "material": "CONCRETE_BLOCK"}
            )
            wall_edges.append(
                {"x": gx, "y": height, "side": "NORTH", "kind": "WALL", "material": "CONCRETE_BLOCK"}
            )
        door_y = height // 2
        for gy in range(height):
            wall_edges.append(
                {
                    "x": 0,
                    "y": gy,
                    "side": "WEST",
                    "kind": "DOOR_OPEN" if gy == door_y else "WALL",
                    "material": "SHEET_STEEL" if gy == door_y else "CONCRETE_BLOCK",
                }
            )
            wall_edges.append(
                {
                    "x": width,
                    "y": gy,
                    "side": "WEST",
                    "kind": "DOOR_CLOSED" if gy == door_y else "WALL",
                    "material": "SHEET_STEEL" if gy == door_y else "CONCRETE_BLOCK",
                }
            )

    # Structural corner column in the NW quadrant (`(0, 0)` cell corner, 90x90cm)
    props.append(
        _make_prop(
            "prop-nw-pillar",
            "STRUCTURAL_PILLAR",
            50,
            50,
            90,
            90,
            rotation_deg=0,
            cover_height_cm=240,
            material="REINFORCED_CONCRETE",
            label="Corner Pillar",
        )
    )

    # Central tactical cover barrier angled 15 deg
    mid_x = max(2, width // 2 - 1)
    mid_y = max(2, height // 2 - 1)
    props.append(
        _make_prop(
            "prop-center-cover",
            "JERSEY_BARRIER",
            mid_x * 100 + 200,
            mid_y * 100 + 75,
            140,
            80,
            rotation_deg=15,
            cover_height_cm=95,
            material="REINFORCED_CONCRETE",
            label="COVER",
        )
    )

    if width >= 10 and height >= 8:
        if preset == "docking_bay":
            props.append(
                _make_prop(
                    "prop-bay-crate",
                    "WOOD_CRATE",
                    max(2, width // 3) * 100 + 50,
                    max(2, height - 3) * 100 + 50,
                    95,
                    95,
                    rotation_deg=30,
                    cover_height_cm=110,
                    material="SOLID_WOOD",
                )
            )
            props.append(
                _make_prop(
                    "prop-bay-spill",
                    "HAZARD_SPILL",
                    min(width - 3, (width * 2) // 3) * 100 + 50,
                    250,
                    160,
                    110,
                    rotation_deg=20,
                    cover_height_cm=0,
                    material="FOLIAGE",
                )
            )
        elif preset == "med_lab":
            wall_edges.append(
                {
                    "x": max(2, width // 3),
                    "y": 2,
                    "side": "WEST",
                    "kind": "WALL",
                    "material": "DRYWALL",
                }
            )
            wall_edges.append(
                {
                    "x": max(2, width // 3),
                    "y": 3,
                    "side": "WEST",
                    "kind": "DOOR_CLOSED",
                    "material": "SHEET_STEEL",
                }
            )
            props.append(
                _make_prop(
                    "prop-med-spill",
                    "HAZARD_SPILL",
                    min(width - 3, (width * 2) // 3) * 100 + 50,
                    max(2, height - 3) * 100 + 50,
                    150,
                    100,
                    rotation_deg=35,
                    cover_height_cm=0,
                    material="FOLIAGE",
                )
            )
        elif preset == "urban_crossroads":
            props.append(
                _make_prop(
                    "prop-urban-catwalk",
                    "ELEVATED_CATWALK",
                    250,
                    250,
                    180,
                    120,
                    rotation_deg=0,
                    cover_height_cm=0,
                    material="SHEET_STEEL",
                )
            )
        else:
            props.append(
                _make_prop(
                    "prop-flank-crate",
                    "WOOD_CRATE",
                    max(2, width // 3) * 100 + 50,
                    250,
                    95,
                    95,
                    rotation_deg=25,
                    cover_height_cm=95,
                    material="SOLID_WOOD",
                )
            )

    return wall_edges, props


def _infer_model_preset(label: str, is_hidden: bool = False) -> str:
    low = label.lower()
    if "valkyrie" in low or "sara" in low:
        return "valkyrie_rig"
    if "recon" in low or "marcus" in low or "vance" in low:
        return "recon_mk4"
    if is_hidden or "sentry" in low or "secops" in low or "hostile" in low:
        return "secops_sentry"
    return "operative_light"


def normalize_token(tok: dict[str, Any]) -> dict[str, Any]:
    """Normalize a token placement dictionary to include continuous `pos_x_cm`/`pos_y_cm`, derived cell coords, and `.glb` fields."""
    if tok.get("pos_x_cm") is not None and tok.get("pos_y_cm") is not None:
        px_cm = int(tok["pos_x_cm"])
        py_cm = int(tok["pos_y_cm"])
    else:
        gx_in = int(tok.get("grid_x", tok.get("x", 0)))
        gy_in = int(tok.get("grid_y", tok.get("y", 0)))
        px_cm = gx_in * 100 + 50
        py_cm = gy_in * 100 + 50
    gx = px_cm // 100
    gy = py_cm // 100
    tid = str(tok.get("token_id", tok.get("entity_id", f"tok-{uuid.uuid4().hex[:6]}")))
    lbl = str(tok.get("label", tid))
    hidden = bool(tok.get("is_hidden", False))
    return {
        "token_id": tid,
        "entity_id": str(tok.get("entity_id", tid)),
        "label": lbl,
        "actor": tok.get("actor"),
        "pos_x_cm": px_cm,
        "pos_y_cm": py_cm,
        "grid_x": gx,
        "grid_y": gy,
        "x": gx,
        "y": gy,
        "color_hex": str(tok.get("color_hex", "#38bdf8")),
        "model_preset": str(tok.get("model_preset") or _infer_model_preset(lbl, hidden)),
        "facing_deg": int(tok.get("facing_deg", 270 if "sentry" in lbl.lower() else 90)),
        "is_hidden": hidden,
        "status_badges": list(tok.get("status_badges", [])),
    }


def _wall_edge_segment_m(edge: dict[str, Any]) -> tuple[float, float, float, float]:
    ex = float(edge.get("x", 0))
    ey = float(edge.get("y", 0))
    side = str(edge.get("side", "NORTH")).upper()
    if side == "WEST":
        return (ex, ey, ex, ey + 1.0)
    return (ex, ey, ex + 1.0, ey)


def _point_to_segment_dist_m(px: float, py: float, ax: float, ay: float, bx: float, by: float) -> float:
    vx = bx - ax
    vy = by - ay
    seg_len2 = vx * vx + vy * vy
    if seg_len2 <= 1e-9:
        return math.hypot(px - ax, py - ay)
    t = max(0.0, min(1.0, ((px - ax) * vx + (py - ay) * vy) / seg_len2))
    return math.hypot(px - (ax + t * vx), py - (ay + t * vy))


def _segments_intersect_m(
    ax: float, ay: float, bx: float, by: float, cx: float, cy: float, dx: float, dy: float
) -> bool:
    def cross(px: float, py: float, qx: float, qy: float, rx: float, ry: float) -> float:
        return (qx - px) * (ry - py) - (qy - py) * (rx - px)

    d1 = cross(cx, cy, dx, dy, ax, ay)
    d2 = cross(cx, cy, dx, dy, bx, by)
    d3 = cross(ax, ay, bx, by, cx, cy)
    d4 = cross(ax, ay, bx, by, dx, dy)
    if ((d1 > 1e-6 and d2 < -1e-6) or (d1 < -1e-6 and d2 > 1e-6)) and (
        (d3 > 1e-6 and d4 < -1e-6) or (d3 < -1e-6 and d4 > 1e-6)
    ):
        return True
    return False


def _prop_center_span_rot_m(prop: dict[str, Any]) -> tuple[float, float, float, float, float]:
    if prop.get("center_x_cm") is not None and prop.get("center_y_cm") is not None:
        cx_m = float(prop["center_x_cm"]) / 100.0
        cy_m = float(prop["center_y_cm"]) / 100.0
    else:
        ox = float(prop.get("x", prop.get("origin_x", 0)))
        oy = float(prop.get("y", prop.get("origin_y", 0)))
        sw = float(prop.get("w_cells", prop.get("span_w", 1)))
        sh = float(prop.get("h_cells", prop.get("span_h", 1)))
        cx_m = ox + sw * 0.5
        cy_m = oy + sh * 0.5
    sw_m = float(prop.get("span_w_cm", int(prop.get("w_cells", prop.get("span_w", 1))) * 100)) / 100.0
    sh_m = float(prop.get("span_h_cm", int(prop.get("h_cells", prop.get("span_h", 1))) * 100)) / 100.0
    rot_rad = math.radians(float(prop.get("rotation_deg", 0)))
    return (cx_m, cy_m, sw_m * 0.5, sh_m * 0.5, rot_rad)


def _circle_hits_prop_obb(px_m: float, py_m: float, radius_m: float, prop: dict[str, Any]) -> bool:
    cx, cy, hx, hy, rot = _prop_center_span_rot_m(prop)
    dx = px_m - cx
    dy = py_m - cy
    c = math.cos(-rot)
    s = math.sin(-rot)
    lx = dx * c - dy * s
    ly = dx * s + dy * c
    qx = max(-hx, min(hx, lx))
    qy = max(-hy, min(hy, ly))
    return math.hypot(lx - qx, ly - qy) < radius_m


def _segment_hits_prop_obb(
    x1_m: float, y1_m: float, x2_m: float, y2_m: float, prop: dict[str, Any]
) -> bool:
    cx, cy, hx, hy, rot = _prop_center_span_rot_m(prop)
    c = math.cos(-rot)
    s = math.sin(-rot)
    dx1, dy1 = x1_m - cx, y1_m - cy
    dx2, dy2 = x2_m - cx, y2_m - cy
    lx1, ly1 = dx1 * c - dy1 * s, dx1 * s + dy1 * c
    lx2, ly2 = dx2 * c - dy2 * s, dx2 * s + dy2 * c

    t0, t1 = 0.0, 1.0
    vx, vy = lx2 - lx1, ly2 - ly1
    for p, q in ((-vx, lx1 + hx), (vx, hx - lx1), (-vy, ly1 + hy), (vy, hy - ly1)):
        if abs(p) <= 1e-8:
            if q < 0.0:
                return False
        else:
            r = q / p
            if p < 0.0:
                if r > t1:
                    return False
                if r > t0:
                    t0 = r
            else:
                if r < t0:
                    return False
                if r < t1:
                    t1 = r
    return t0 <= t1


def _evaluate_pair_los_and_cover(
    wall_edges: list[dict[str, Any]],
    props: list[dict[str, Any]],
    x1_m: float,
    y1_m: float,
    x2_m: float,
    y2_m: float,
) -> tuple[float, str, int]:
    """Analytical 2D segment/OBB raycast returning `(distance_m, cover_tag, recommended_delta_ext)`."""
    dist_m = round(math.hypot(x1_m - x2_m, y1_m - y2_m), 1)
    if dist_m <= 1e-4:
        return (0.0, " [CLEAR LOS]", 0)

    if using_satya_py_map_backend():
        try:
            scene_json = json.dumps(
                {
                    "map_id": "los",
                    "title": "los",
                    "image_url": "",
                    "grid_width": 48,
                    "grid_height": 48,
                    "wall_edges": wall_edges,
                    "props": props,
                }
            )
            ev_ab = _SATYA_PY.evaluate_tactical_los_and_cover(
                scene_json, x1_m, y1_m, x2_m, y2_m
            )
            ev_ba = _SATYA_PY.evaluate_tactical_los_and_cover(
                scene_json, x2_m, y2_m, x1_m, y1_m
            )
            best = (
                ev_ab
                if int(ev_ab.get("recommended_delta_ext", 0))
                >= int(ev_ba.get("recommended_delta_ext", 0))
                else ev_ba
            )
            delta = int(best.get("recommended_delta_ext", 0))
            grade = str(best.get("cover_grade", "CLEAR")).upper()
            dist_py = round(float(best.get("distance_m", dist_m)), 1)
            if grade == "BLOCKED":
                return (dist_py, f" [PARTIAL COVER / OBSTRUCTED] (Blocked LoS Δ_ext +{delta})", delta)
            if grade == "HIGH_COVER":
                return (dist_py, f" [PARTIAL COVER / OBSTRUCTED] (High Cover Δ_ext +{delta})", delta)
            if grade == "LOW_COVER":
                return (dist_py, f" [PARTIAL COVER / OBSTRUCTED] (Low Cover Δ_ext +{delta})", delta)
            if grade == "CONCEALMENT":
                return (dist_py, f" [CONCEALMENT] (Foliage/Screen Δ_ext +{delta})", delta)
            return (dist_py, " [CLEAR LOS]", 0)
        except Exception:
            pass

    blocked = False
    low_cover = False
    concealment = False

    for edge in wall_edges:
        kind = str(edge.get("kind", "WALL")).upper()
        if kind == "DOOR_OPEN":
            continue
        ax, ay, bx, by = _wall_edge_segment_m(edge)
        if _segments_intersect_m(x1_m, y1_m, x2_m, y2_m, ax, ay, bx, by):
            if kind in {"WALL", "DOOR_CLOSED"}:
                blocked = True
                break
            if kind == "WINDOW":
                low_cover = True

    if not blocked:
        for prop in props:
            kind = str(prop.get("kind", "")).upper()
            if kind in _NON_SOLID_PROP_KINDS:
                continue
            if _segment_hits_prop_obb(x1_m, y1_m, x2_m, y2_m, prop):
                ch = int(prop.get("cover_height_cm", 95))
                if kind in _CONCEALMENT_PROP_KINDS:
                    concealment = True
                elif ch >= 170:
                    blocked = True
                    break
                elif ch >= 60:
                    low_cover = True

    if blocked:
        return (dist_m, " [PARTIAL COVER / OBSTRUCTED] (Blocked LoS Δ_ext +16)", 16)
    if low_cover:
        return (dist_m, " [PARTIAL COVER / OBSTRUCTED] (Low Cover Δ_ext +4)", 4)
    if concealment:
        return (dist_m, " [CONCEALMENT] (Foliage/Screen Δ_ext +3)", 3)
    return (dist_m, " [CLEAR LOS]", 0)


def render_tactical_svg(
    map_id: str,
    theme: str,
    width: int,
    height: int,
    tokens: list[dict[str, Any]],
    tiles: list[str] | None = None,
    wall_edges: list[dict[str, Any]] | None = None,
    props: list[dict[str, Any]] | None = None,
) -> str:
    """Render a continuous-vector 2D tactical battlemap SVG with thin walls, rotated OBB props, and token badges."""
    cell_px = 48
    svg_w = width * cell_px
    svg_h = height * cell_px

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_w} {svg_h}" width="{svg_w}" height="{svg_h}">',
        "  <defs>",
        '    <pattern id="grid" width="48" height="48" patternUnits="userSpaceOnUse">',
        '      <path d="M 48 0 L 0 0 0 48" fill="none" stroke="#1e293b" stroke-width="1"/>',
        "    </pattern>",
        "  </defs>",
        f'  <rect width="{svg_w}" height="{svg_h}" fill="#090d16"/>',
        f'  <rect width="{svg_w}" height="{svg_h}" fill="url(#grid)"/>',
        f'  <rect x="3" y="3" width="{svg_w - 6}" height="{svg_h - 6}" fill="none" stroke="#334155" stroke-width="3"/>',
    ]

    # Legacy tiles fallback if non-empty
    if tiles and len(tiles) == width * height:
        for gy in range(height):
            for gx in range(width):
                kind = tiles[gy * width + gx]
                if kind == "FLOOR":
                    continue
                px = gx * cell_px
                py = gy * cell_px
                if kind == "FULL_WALL":
                    lines.append(
                        f'  <rect x="{px}" y="{py}" width="{cell_px}" height="{cell_px}" fill="#1e293b" stroke="#475569" stroke-width="1"/>'
                    )
                elif kind == "HALF_COVER":
                    lines.append(
                        f'  <rect x="{px + 4}" y="{py + 4}" width="{cell_px - 8}" height="{cell_px - 8}" fill="#1e3a5f" stroke="#38bdf8" stroke-width="1.5" rx="3"/>'
                    )
                elif kind == "HAZARD_ZONE":
                    lines.append(
                        f'  <rect x="{px + 2}" y="{py + 2}" width="{cell_px - 4}" height="{cell_px - 4}" fill="#4c0519" fill-opacity="0.65" stroke="#f43f5e" stroke-width="1"/>'
                    )

    # Render 360-degree continuous props (sorted so surfaces draw below solids)
    sorted_props = sorted(
        list(props or []),
        key=lambda p: 0 if str(p.get("kind", "")).upper() in _NON_SOLID_PROP_KINDS else 1,
    )
    has_cover_label = False
    for prop in sorted_props:
        cx_m, cy_m, hx_m, hy_m, rot_rad = _prop_center_span_rot_m(prop)
        px_c = round(cx_m * cell_px, 1)
        py_c = round(cy_m * cell_px, 1)
        pw = round(hx_m * 2.0 * cell_px, 1)
        ph = round(hy_m * 2.0 * cell_px, 1)
        rot_deg = round(math.degrees(rot_rad), 1)
        kind = str(prop.get("kind", "")).upper()
        if kind == "HAZARD_SPILL":
            fill, stroke = "#4c0519", "#f43f5e"
        elif kind == "ELEVATED_CATWALK":
            fill, stroke = "#1e293b", "#64748b"
        elif kind == "STRUCTURAL_PILLAR":
            fill, stroke = "#334155", "#94a3b8"
        elif kind in _NON_SOLID_PROP_KINDS:
            fill, stroke = "#1e293b", "#475569"
        else:
            fill, stroke = "#1e3a5f", "#38bdf8"
        lines.append(
            f'  <g transform="translate({px_c},{py_c}) rotate({rot_deg})">'
            f'<rect x="{-pw * 0.5:.1f}" y="{-ph * 0.5:.1f}" width="{pw:.1f}" height="{ph:.1f}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="1.5" rx="3"/>'
        )
        raw_lbl = str(prop.get("label") or "")
        if "COVER" in raw_lbl.upper() or kind == "JERSEY_BARRIER":
            has_cover_label = True
            lines.append(
                '    <text x="0" y="3" text-anchor="middle" fill="#cbd5e1" font-family="monospace" font-size="9">COVER</text>'
            )
        lines.append("  </g>")

    if not has_cover_label and sorted_props:
        lines.append(
            f'  <text x="{svg_w // 2}" y="{svg_h // 2}" text-anchor="middle" fill="#94a3b8" font-family="monospace" font-size="9">COVER</text>'
        )

    # Render thin vector wall / door / window edges
    for edge in wall_edges or []:
        ax, ay, bx, by = _wall_edge_segment_m(edge)
        x1, y1 = round(ax * cell_px, 1), round(ay * cell_px, 1)
        x2, y2 = round(bx * cell_px, 1), round(by * cell_px, 1)
        kind = str(edge.get("kind", "WALL")).upper()
        if kind == "DOOR_OPEN":
            stroke, dash = "#0ea5e9", ' stroke-dasharray="4 3"'
        elif kind == "DOOR_CLOSED":
            stroke, dash = "#f59e0b", ""
        elif kind == "WINDOW":
            stroke, dash = "#38bdf8", ' stroke-dasharray="6 2"'
        else:
            stroke, dash = "#64748b", ""
        lines.append(
            f'  <line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="4"{dash} stroke-linecap="round"/>'
        )

    # Meter scale rulers along top row and left column
    for gx in range(width):
        cx = gx * cell_px + cell_px // 2
        lines.append(
            f'  <text x="{cx}" y="12" text-anchor="middle" fill="#475569" font-family="monospace" font-size="9">{gx}</text>'
        )
    for gy in range(height):
        cy = gy * cell_px + cell_px // 2 + 3
        lines.append(
            f'  <text x="10" y="{cy}" text-anchor="middle" fill="#475569" font-family="monospace" font-size="9">{gy}</text>'
        )

    safe_theme = (
        theme[:48]
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
    )
    lines.append(
        f'  <text x="20" y="{svg_h - 10}" fill="#94a3b8" font-family="monospace" font-size="12">'
        f"SATYA TACTICAL VECTOR MAP [{map_id}] // {safe_theme} ({width}m x {height}m · 1.0m scale)</text>"
    )

    for tok in tokens:
        if tok.get("is_hidden"):
            continue
        px_cm = int(tok.get("pos_x_cm", int(tok.get("grid_x", tok.get("x", 0))) * 100 + 50))
        py_cm = int(tok.get("pos_y_cm", int(tok.get("grid_y", tok.get("y", 0))) * 100 + 50))
        cx = round(px_cm * cell_px / 100.0)
        cy = round(py_cm * cell_px / 100.0)
        color = str(tok.get("color_hex", "#38bdf8"))
        label = (
            str(tok.get("label", tok.get("token_id", "?")))[:10]
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
        )
        lines.append(
            f'  <circle cx="{cx}" cy="{cy}" r="16" fill="{color}" stroke="#f8fafc" stroke-width="2"/>'
        )
        lines.append(
            f'  <text x="{cx}" y="{cy + 4}" text-anchor="middle" fill="#090d16" '
            f'font-family="monospace" font-weight="bold" font-size="10">{label}</text>'
        )
        badges = tok.get("status_badges") or []
        if badges:
            badge_txt = str(badges[0])[:8].replace("&", "&amp;").replace("<", "&lt;")
            lines.append(
                f'  <text x="{cx}" y="{cy - 19}" text-anchor="middle" fill="#fbbf24" '
                f'font-family="monospace" font-size="8">[{badge_txt}]</text>'
            )

    lines.append("</svg>")
    return "\n".join(lines)


def _sync_map_with_signaling(session_id: str, map_state: dict[str, Any]) -> None:
    wire_tokens = [
        {
            "entity_id": str(t.get("entity_id", t.get("token_id", "tok"))),
            "label": t["label"],
            "x": int(t.get("grid_x", t.get("x", 0))),
            "y": int(t.get("grid_y", t.get("y", 0))),
            "visibility": "GM_ONLY" if t.get("is_hidden", False) else "PUBLIC",
        }
        for t in map_state.get("tokens", [])
    ]
    wire_visuals = [
        {
            "entity_id": str(t.get("entity_id", t.get("token_id", "tok"))),
            "model_preset": str(t.get("model_preset") or _infer_model_preset(t["label"])),
            "color_hex": str(t.get("color_hex", "#38bdf8")),
            "facing_deg": int(t.get("facing_deg", 90)),
            "elevation_cm": 0,
            "stance_pose": "aimed",
            "pos_x_cm": int(t.get("pos_x_cm", int(t.get("grid_x", t.get("x", 0))) * 100 + 50)),
            "pos_y_cm": int(t.get("pos_y_cm", int(t.get("grid_y", t.get("y", 0))) * 100 + 50)),
        }
        for t in map_state.get("tokens", [])
    ]
    wire_payload = {
        "map_id": map_state["map_id"],
        "title": map_state.get("title", map_state.get("theme", "Tactical Map")),
        "image_url": map_state.get("image_url", map_state.get("map_image_url", "")),
        "grid_width": int(map_state["grid_width"]),
        "grid_height": int(map_state["grid_height"]),
        "cell_size_cm": int(map_state.get("cell_size_cm", 100)),
        "theme_preset": map_state.get("theme_preset", infer_sector_preset(str(map_state.get("theme", "")))),
        "seed": int(map_state.get("seed", 42)),
        "tiles": list(map_state.get("tiles", [])),
        "wall_edges": list(map_state.get("wall_edges", [])),
        "props": list(map_state.get("props", [])),
        "tokens": wire_tokens,
        "token_visuals": wire_visuals,
        "notes": map_state.get("notes"),
    }
    url = f"{_signaling_base_url()}/api/gm/sessions/{session_id}/map"
    headers = _gm_headers()
    if httpx is not None:
        try:
            with httpx.Client(timeout=2.0) as client:
                client.post(url, headers=headers, json=wire_payload)
                return
        except Exception:
            return
    try:
        req = urllib.request.Request(
            url,
            data=json.dumps(wire_payload).encode("utf-8"),
            headers=headers,
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=2.0):
            pass
    except Exception:
        pass


@tool
def generate_tactical_map(
    session_id: str,
    theme: str,
    width: int = 16,
    height: int = 12,
    initial_tokens: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    """Generate a continuous vector 3D/2D tactical battlemap (`wall_edges` + 360° `props` + `data/maps/*.svg`) and broadcast `MAP_UPDATED`."""
    width = max(4, min(int(width), 32))
    height = max(4, min(int(height), 32))
    map_id = f"map-{uuid.uuid4().hex[:8]}"
    preset = infer_sector_preset(theme)
    wall_edges, props = build_procedural_vector_scene(width, height, preset)
    tiles: list[str] = []

    raw_tokens = list(initial_tokens or [])
    if not raw_tokens:
        raw_tokens = [
            {
                "token_id": "tok-vance",
                "label": "Capt. Vance",
                "grid_x": 2,
                "grid_y": height // 2,
                "color_hex": "#38bdf8",
                "model_preset": "recon_mk4",
                "facing_deg": 90,
            },
            {
                "token_id": "tok-secops-1",
                "label": "SecOps Sentry",
                "grid_x": width - 3,
                "grid_y": height // 2,
                "color_hex": "#f87171",
                "model_preset": "secops_sentry",
                "facing_deg": 270,
            },
        ]
    tokens = [normalize_token(t) for t in raw_tokens]

    maps_dir = _repo_root() / "data" / "maps"
    maps_dir.mkdir(parents=True, exist_ok=True)
    filename = f"{session_id}_{map_id}.svg"
    svg_path = maps_dir / filename
    svg_path.write_text(
        render_tactical_svg(map_id, theme, width, height, tokens, tiles, wall_edges, props),
        encoding="utf-8",
    )

    map_state: dict[str, Any] = {
        "event_type": "MAP_GENERATED",
        "map_id": map_id,
        "title": theme,
        "theme": theme,
        "theme_preset": preset,
        "cell_size_cm": 100,
        "seed": 42,
        "tiles": tiles,
        "wall_edges": wall_edges,
        "props": props,
        "image_url": f"/maps/{filename}",
        "map_image_url": f"/maps/{filename}",
        "grid_width": width,
        "grid_height": height,
        "tokens": tokens,
        "notes": f"Continuous Vector Tactical Diorama & SVG battlemap for {theme} ({preset})",
    }
    set_session_map(session_id, map_state)
    _sync_map_with_signaling(session_id, map_state)
    return map_state


@tool
def move_map_token(
    session_id: str,
    token_id: str,
    to_x: int = 0,
    to_y: int = 0,
    to_x_cm: int | None = None,
    to_y_cm: int | None = None,
) -> dict[str, Any]:
    """Move a token continuously to `(to_x_cm, to_y_cm)` (or cell `(to_x, to_y)` center) validating bounds, walls, and OBB obstacles."""
    map_state = get_session_map(session_id)
    if map_state is None:
        return {"error": f"No active tactical map for session '{session_id}'."}

    grid_w = int(map_state.get("grid_width", 16))
    grid_h = int(map_state.get("grid_height", 12))
    tx_cm = int(to_x_cm) if to_x_cm is not None else int(to_x) * 100 + 50
    ty_cm = int(to_y_cm) if to_y_cm is not None else int(to_y) * 100 + 50

    margin_cm = int(round(_TOKEN_RADIUS_M * 100))
    if not (margin_cm <= tx_cm <= grid_w * 100 - margin_cm and margin_cm <= ty_cm <= grid_h * 100 - margin_cm):
        return {
            "error": f"Target coordinates ({tx_cm}cm, {ty_cm}cm) out of bounds for {grid_w}m x {grid_h}m board."
        }

    tx_m = tx_cm / 100.0
    ty_m = ty_cm / 100.0
    wall_edges = list(map_state.get("wall_edges") or [])
    props = list(map_state.get("props") or [])

    py_checked = False
    if using_satya_py_map_backend():
        try:
            collides = bool(
                _SATYA_PY.check_tactical_token_collision(
                    json.dumps(map_state), tx_m, ty_m, _TOKEN_RADIUS_M
                )
            )
            py_checked = True
            if collides:
                return {
                    "error": f"Target position ({tx_m:.2f}m, {ty_m:.2f}m) collides with a wall or solid obstacle."
                }
        except Exception:
            py_checked = False

    if not py_checked:
        for edge in wall_edges:
            kind = str(edge.get("kind", "WALL")).upper()
            if kind in {"WALL", "DOOR_CLOSED", "WINDOW"}:
                ax, ay, bx, by = _wall_edge_segment_m(edge)
                if _point_to_segment_dist_m(tx_m, ty_m, ax, ay, bx, by) < _TOKEN_RADIUS_M:
                    return {
                        "error": f"Target position ({tx_m:.2f}m, {ty_m:.2f}m) is blocked by {kind} edge."
                    }

        for prop in props:
            kind = str(prop.get("kind", "")).upper()
            if kind in _NON_SOLID_PROP_KINDS:
                continue
            if _circle_hits_prop_obb(tx_m, ty_m, _TOKEN_RADIUS_M, prop):
                return {
                    "error": f"Target position ({tx_m:.2f}m, {ty_m:.2f}m) overlaps solid obstacle {kind}."
                }

    tx_cell = max(0, min(grid_w - 1, tx_cm // 100))
    ty_cell = max(0, min(grid_h - 1, ty_cm // 100))
    tokens = [normalize_token(t) for t in map_state.get("tokens", [])]
    found = False
    for tok in tokens:
        if tok.get("token_id") == token_id or str(tok.get("label", "")).lower() == token_id.lower():
            old_x_cm = int(tok.get("pos_x_cm", tx_cm))
            old_y_cm = int(tok.get("pos_y_cm", ty_cm))
            if (old_x_cm, old_y_cm) != (tx_cm, ty_cm):
                tok["facing_deg"] = int(round(math.degrees(math.atan2(tx_cm - old_x_cm, -(ty_cm - old_y_cm))))) % 360
            tok["pos_x_cm"] = tx_cm
            tok["pos_y_cm"] = ty_cm
            tok["grid_x"] = tx_cell
            tok["grid_y"] = ty_cell
            tok["x"] = tx_cell
            tok["y"] = ty_cell
            found = True
            break

    if not found:
        tokens.append(
            normalize_token(
                {
                    "token_id": token_id,
                    "label": token_id,
                    "pos_x_cm": tx_cm,
                    "pos_y_cm": ty_cm,
                    "grid_x": tx_cell,
                    "grid_y": ty_cell,
                    "color_hex": "#a78bfa",
                }
            )
        )

    map_id = str(map_state.get("map_id", "map"))
    theme = str(map_state.get("theme", map_state.get("title", "Tactical Map")))
    tiles = list(map_state.get("tiles") or [])
    maps_dir = _repo_root() / "data" / "maps"
    filename = f"{session_id}_{map_id}.svg"
    svg_path = maps_dir / filename
    if maps_dir.exists():
        try:
            svg_path.write_text(
                render_tactical_svg(map_id, theme, grid_w, grid_h, tokens, tiles or None, wall_edges, props),
                encoding="utf-8",
            )
        except OSError:
            pass

    updated = dict(map_state)
    updated["event_type"] = "TOKEN_MOVED"
    updated["tokens"] = tokens
    set_session_map(session_id, updated)
    _sync_map_with_signaling(session_id, updated)
    return updated


@tool
def inspect_tactical_map(session_id: str, question: str) -> str:
    """Inspect the active continuous tactical map geometry, token positions, metric distances, cover `Δ_ext`, and optional VLM analysis."""
    map_state = get_session_map(session_id)
    if map_state is None:
        return f"No active tactical map for session '{session_id}'."

    width = int(map_state.get("grid_width", 16))
    height = int(map_state.get("grid_height", 12))
    tokens = map_state.get("tokens", [])
    preset = str(map_state.get("theme_preset") or infer_sector_preset(str(map_state.get("theme", ""))))
    wall_edges = list(map_state.get("wall_edges") or [])
    props = list(map_state.get("props") or [])
    if not wall_edges and not props:
        wall_edges, props = build_procedural_vector_scene(width, height, preset)

    prop_notes = [
        f"{p.get('kind')}@({p.get('center_x_cm', 0) / 100.0:.2f}m,{p.get('center_y_cm', 0) / 100.0:.2f}m {p.get('rotation_deg', 0)}deg)"
        for p in props[:12]
    ]

    token_summaries = [
        f"- {t.get('label', t.get('token_id'))} (`{t.get('token_id')}`) at ({t.get('grid_x', t.get('x'))}, {t.get('grid_y', t.get('y'))}) [{int(t.get('pos_x_cm', 0)) / 100.0:.2f}m, {int(t.get('pos_y_cm', 0)) / 100.0:.2f}m]"
        for t in tokens
    ]
    pairwise: list[str] = []
    for i in range(len(tokens)):
        for j in range(i + 1, len(tokens)):
            t1, t2 = tokens[i], tokens[j]
            x1_m = float(t1.get("pos_x_cm", int(t1.get("grid_x", t1.get("x", 0))) * 100 + 50)) / 100.0
            y1_m = float(t1.get("pos_y_cm", int(t1.get("grid_y", t1.get("y", 0))) * 100 + 50)) / 100.0
            x2_m = float(t2.get("pos_x_cm", int(t2.get("grid_x", t2.get("x", 0))) * 100 + 50)) / 100.0
            y2_m = float(t2.get("pos_y_cm", int(t2.get("grid_y", t2.get("y", 0))) * 100 + 50)) / 100.0
            dist_m, los_tag, _delta = _evaluate_pair_los_and_cover(
                wall_edges, props, x1_m, y1_m, x2_m, y2_m
            )
            pairwise.append(
                f"{t1.get('label', t1.get('token_id'))} <-> {t2.get('label', t2.get('token_id'))}: {dist_m}m{los_tag}"
            )

    geometric_summary = (
        f"Map '{map_state.get('map_id')}' ({width}m x {height}m continuous vector, preset={preset}, url={map_state.get('image_url', map_state.get('map_image_url'))}):\n"
        f"Walls: {len(wall_edges)} edges. Tactical props: {', '.join(prop_notes)}.\n"
        f"Tokens:\n" + "\n".join(token_summaries) + "\n"
        f"Pairwise Distances & Line of Sight: {', '.join(pairwise) if pairwise else 'N/A'}\n"
        f"Query context: {question}"
    )

    cfg = get_session_llm_config(session_id)
    if cfg is not None and cfg.vision_model_id and HumanMessage is not None:
        img_rel = str(map_state.get("image_url", "")).lstrip("/")
        img_path = _repo_root() / "data" / img_rel
        if img_path.exists() and img_path.suffix.lower() in {".png", ".jpg", ".jpeg", ".webp"}:
            try:
                b64_img = base64.b64encode(img_path.read_bytes()).decode("ascii")
                mime = "image/png" if img_path.suffix.lower() == ".png" else "image/jpeg"
                vlm = build_gm_chat_model(cfg, for_vision=True)
                vlm_resp = vlm.invoke(
                    [
                        HumanMessage(
                            content=[
                                {"type": "text", "text": f"{geometric_summary}\nAnswer concisely: {question}"},
                                {"type": "image_url", "image_url": {"url": f"data:{mime};base64,{b64_img}"}},
                            ]
                        )
                    ]
                )
                vlm_text = getattr(vlm_resp, "content", "")
                if vlm_text:
                    return f"{geometric_summary}\nVLM Spatial Observation: {vlm_text}"
            except Exception:
                pass

    return geometric_summary


def ensure_or_sync_procedural_tactical_map_stub(
    session_id: str,
    theme: str = "Action Mode Tactical Sector",
    combatants: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    """Ensure a continuous vector tactical map is active during Action Mode (`combat_active == True`)
    and synchronize player and NPC combatant placements with `satya-signaling`.
    """
    existing = get_session_map(session_id)
    raw_combatants = list(combatants or [])
    if existing is None:
        initial_tokens: list[dict[str, Any]] = []
        for idx, c in enumerate(raw_combatants):
            role_str = str(c.get("role_or_faction") or "").lower()
            is_hostile = "npc" in role_str or "hostile" in role_str or "enemy" in role_str
            default_x = 12 - (idx % 3) * 2 if is_hostile else 3 + (idx % 3) * 2
            default_y = 5 + (idx // 3) * 2
            gx = max(1, min(14, int(c.get("grid_x") if c.get("grid_x") is not None else default_x)))
            gy = max(1, min(10, int(c.get("grid_y") if c.get("grid_y") is not None else default_y)))
            px_cm = int(c.get("pos_x_cm") if c.get("pos_x_cm") is not None else gx * 100 + 50)
            py_cm = int(c.get("pos_y_cm") if c.get("pos_y_cm") is not None else gy * 100 + 50)
            zone = str(c.get("zone_label") or ("Entry Bulkhead" if is_hostile else "Core Sector"))
            initial_tokens.append(
                {
                    "token_id": str(c.get("entity_id") or f"comb-{idx}"),
                    "entity_id": str(c.get("entity_id") or f"comb-{idx}"),
                    "label": str(c.get("display_name") or f"Combatant {idx + 1}"),
                    "pos_x_cm": px_cm,
                    "pos_y_cm": py_cm,
                    "grid_x": px_cm // 100,
                    "grid_y": py_cm // 100,
                    "color_hex": "#f87171" if is_hostile else "#38bdf8",
                    "status_badges": [zone[:8]],
                }
            )
        generated = dict(
            generate_tactical_map.invoke(
                {
                    "session_id": session_id,
                    "theme": theme,
                    "width": 16,
                    "height": 12,
                    "initial_tokens": initial_tokens or None,
                }
            )
        )
        generated["procedural_generator_stub"] = True
        set_session_map(session_id, generated)
        return generated

    tokens = [normalize_token(t) for t in existing.get("tokens", [])]
    width = int(existing.get("grid_width", 16))
    height = int(existing.get("grid_height", 12))
    changed = not bool(existing.get("procedural_generator_stub"))
    for idx, c in enumerate(raw_combatants):
        eid = str(c.get("entity_id") or "")
        dname = str(c.get("display_name") or eid or f"Combatant {idx + 1}")
        gx = c.get("grid_x")
        gy = c.get("grid_y")
        matched = None
        for tok in tokens:
            if tok.get("token_id") == eid or str(tok.get("label", "")).lower() == dname.lower():
                matched = tok
                break
        if matched is not None:
            if gx is not None and int(gx) != matched.get("grid_x"):
                ng = max(1, min(width - 2, int(gx)))
                matched["grid_x"] = ng
                matched["x"] = ng
                matched["pos_x_cm"] = ng * 100 + 50
                changed = True
            if gy is not None and int(gy) != matched.get("grid_y"):
                ng = max(1, min(height - 2, int(gy)))
                matched["grid_y"] = ng
                matched["y"] = ng
                matched["pos_y_cm"] = ng * 100 + 50
                changed = True
        elif eid or dname:
            role_str = str(c.get("role_or_faction") or "").lower()
            is_hostile = bool(c.get("is_npc")) or "npc" in role_str or "hostile" in role_str
            nx = max(1, min(width - 2, int(gx if gx is not None else (12 if is_hostile else 3))))
            ny = max(1, min(height - 2, int(gy if gy is not None else (4 + idx % 5))))
            tokens.append(
                normalize_token(
                    {
                        "token_id": eid or f"tok-{idx}",
                        "label": dname,
                        "pos_x_cm": nx * 100 + 50,
                        "pos_y_cm": ny * 100 + 50,
                        "grid_x": nx,
                        "grid_y": ny,
                        "color_hex": "#f87171" if is_hostile else "#38bdf8",
                    }
                )
            )
            changed = True

    if changed:
        existing = dict(existing)
        existing["tokens"] = tokens
        existing["procedural_generator_stub"] = True
        set_session_map(session_id, existing)
        _sync_map_with_signaling(session_id, existing)
    return existing
