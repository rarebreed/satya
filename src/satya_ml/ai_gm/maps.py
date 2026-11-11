"""Procedural `1.0m` Tactical Map Generator, Token Movement, Cover/LoS Raycaster, and VLM Inspector.

Implements the AI GM tactical spatial pipeline synchronized with `satya-engine` and `satya-web` (`wgpu` 3D/2D diorama):
- `generate_tactical_map`: Generates a `1.0m`-per-cell structured tactical grid (`tiles`, `token_visuals`,
  `cell_size_cm = 100`) alongside a coordinate-labeled top-down SVG battlemap under `data/maps/`,
  and broadcasts `MAP_UPDATED` via `POST /api/gm/sessions/:id/map`.
- `move_map_token`: Validates grid bounds and wall/closed-door occupancy, updates token `(grid_x, grid_y)`
  and facing orientation, re-renders the SVG battlemap on disk, and broadcasts `MAP_UPDATED`.
- `inspect_tactical_map`: Evaluates `1.0m` metric pairwise distances, supercover raycast lines of sight,
  and cover `Δ_ext` (`+0` Clear, `+4` Low Cover, `+6` High Cover, `+16` Blocked), and optionally invokes
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

_BLOCKING_TILES = {"FULL_WALL", "DOOR_CLOSED"}
_COVER_TILES = {"HALF_COVER", "FULL_WALL", "DOOR_CLOSED"}


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
    """Map a free-text scene theme to one of the 5 canonical `satya-engine` `SectorPreset` codes."""
    low = (theme or "").lower()
    if any(k in low for k in ("dock", "cargo", "hangar", "bay")):
        return "docking_bay"
    if any(k in low for k in ("med", "trauma", "lab", "clinic", "ward")):
        return "med_lab"
    if any(k in low for k in ("urban", "street", "road", "city", "crossroad")):
        return "urban_crossroads"
    if any(k in low for k in ("corridor", "derelict", "breach", "tunnel")):
        return "derelict_corridor"
    return "orbital_airlock"


def build_procedural_tiles(width: int, height: int, preset: str = "orbital_airlock") -> list[str]:
    """Generate a row-major `width * height` `1.0m` `TacticalTileKind` array matching `satya-engine` conventions."""
    tiles = ["FLOOR"] * (width * height)

    def set_tile(gx: int, gy: int, kind: str) -> None:
        if 0 <= gx < width and 0 <= gy < height:
            tiles[gy * width + gx] = kind

    # Outer bulkhead perimeter with West/East airlock doors
    if width >= 8 and height >= 8:
        for gx in range(width):
            set_tile(gx, 0, "FULL_WALL")
            set_tile(gx, height - 1, "FULL_WALL")
        for gy in range(height):
            set_tile(0, gy, "FULL_WALL")
            set_tile(width - 1, gy, "FULL_WALL")
        door_y = height // 2
        set_tile(0, door_y, "DOOR_OPEN")
        set_tile(width - 1, door_y, "DOOR_CLOSED")

    # Central tactical cover cluster (`HALF_COVER` so miniatures can brace at `(mid_x, mid_y)`)
    mid_x = max(2, width // 2 - 1)
    mid_y = max(2, height // 2 - 1)
    set_tile(mid_x, mid_y, "HALF_COVER")
    if mid_x + 1 < width - 1:
        set_tile(mid_x + 1, mid_y, "HALF_COVER")

    # Preset-specific secondary tactical features (flanking cover, bulkheads, hazard zones)
    if width >= 10 and height >= 8:
        if preset == "docking_bay":
            set_tile(max(2, width // 3), max(2, height - 3), "HALF_COVER")
            set_tile(min(width - 3, (width * 2) // 3), 2, "HAZARD_ZONE")
        elif preset == "med_lab":
            set_tile(max(2, width // 3), 2, "FULL_WALL")
            set_tile(max(2, width // 3), 3, "DOOR_CLOSED")
            set_tile(min(width - 3, (width * 2) // 3), max(2, height - 3), "HAZARD_ZONE")
        elif preset == "urban_crossroads":
            set_tile(2, 2, "ELEVATED_PLATFORM")
            set_tile(width - 3, height - 3, "HALF_COVER")
        elif preset == "derelict_corridor":
            set_tile(min(width - 3, width // 2 + 2), max(2, height - 3), "HAZARD_ZONE")
        else:
            set_tile(max(2, width // 3), 2, "HALF_COVER")
            set_tile(min(width - 3, (width * 2) // 3), max(2, height - 3), "HALF_COVER")

    return tiles


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
    """Normalize a token placement dictionary to include both `grid_x`/`grid_y`, `x`/`y`, and 3D `.glb` fields."""
    gx = int(tok.get("grid_x", tok.get("x", 0)))
    gy = int(tok.get("grid_y", tok.get("y", 0)))
    tid = str(tok.get("token_id", tok.get("entity_id", f"tok-{uuid.uuid4().hex[:6]}")))
    lbl = str(tok.get("label", tid))
    hidden = bool(tok.get("is_hidden", False))
    return {
        "token_id": tid,
        "entity_id": str(tok.get("entity_id", tid)),
        "label": lbl,
        "actor": tok.get("actor"),
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


def _evaluate_pair_los_and_cover(
    tiles: list[str],
    width: int,
    height: int,
    x1: int,
    y1: int,
    x2: int,
    y2: int,
) -> tuple[float, str, int]:
    """Supercover raycast returning `(distance_m, cover_tag, recommended_delta_ext)` on a `1.0m` grid."""
    dist_m = round(math.hypot(x1 - x2, y1 - y2), 1)
    if (x1, y1) == (x2, y2):
        return (0.0, " [CLEAR LOS]", 0)

    steps = max(abs(x2 - x1), abs(y2 - y1)) * 4
    blocked = False
    low_cover = False
    for step in range(1, steps):
        t = step / float(steps)
        rx = x1 + 0.5 + (x2 - x1) * t
        ry = y1 + 0.5 + (y2 - y1) * t
        gx = int(math.floor(rx))
        gy = int(math.floor(ry))
        if (gx, gy) in {(x1, y1), (x2, y2)}:
            continue
        if 0 <= gx < width and 0 <= gy < height:
            kind = tiles[gy * width + gx]
            if kind in _BLOCKING_TILES:
                blocked = True
                break
            if kind == "HALF_COVER":
                low_cover = True

    # Also treat standing directly on or behind a HALF_COVER cell toward the opponent as partial cover
    for cx, cy in ((x1, y1), (x2, y2)):
        if 0 <= cx < width and 0 <= cy < height and tiles[cy * width + cx] == "HALF_COVER":
            low_cover = True

    if blocked:
        return (dist_m, " [PARTIAL COVER / OBSTRUCTED] (Blocked LoS Δ_ext +16)", 16)
    if low_cover:
        return (dist_m, " [PARTIAL COVER / OBSTRUCTED] (Low Cover Δ_ext +4)", 4)
    return (dist_m, " [CLEAR LOS]", 0)


def render_tactical_svg(
    map_id: str,
    theme: str,
    width: int,
    height: int,
    tokens: list[dict[str, Any]],
    tiles: list[str] | None = None,
) -> str:
    """Render a coordinate-labeled 2D tactical battlemap SVG with cover obstacles, airlocks, and token badges."""
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

    # Coordinate axis labels along top row and left column
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

    # Central cover bulkhead label (`COVER`)
    mid_x = max(2, width // 2 - 1) * cell_px
    mid_y = max(2, height // 2 - 1) * cell_px
    lines.append(
        f'  <rect x="{mid_x}" y="{mid_y}" width="{cell_px * 2}" height="{cell_px}" '
        f'fill="#1e293b" stroke="#64748b" stroke-width="2" rx="4"/>'
    )
    lines.append(
        f'  <text x="{mid_x + cell_px}" y="{mid_y + cell_px // 2 + 4}" text-anchor="middle" '
        f'fill="#94a3b8" font-family="monospace" font-size="9">COVER</text>'
    )

    # Airlock door markers on West and East bulkheads
    door_y = (height // 2) * cell_px
    lines.append(
        f'  <rect x="2" y="{door_y + 8}" width="8" height="{cell_px - 16}" fill="#0ea5e9" rx="2"/>'
    )
    lines.append(
        f'  <rect x="{svg_w - 10}" y="{door_y + 8}" width="8" height="{cell_px - 16}" fill="#f59e0b" rx="2"/>'
    )

    safe_theme = (
        theme[:48]
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
    )
    lines.append(
        f'  <text x="20" y="{svg_h - 10}" fill="#94a3b8" font-family="monospace" font-size="12">'
        f"SATYA TACTICAL GRID [{map_id}] // {safe_theme} ({width}m x {height}m · 1.0m/cell)</text>"
    )

    for tok in tokens:
        if tok.get("is_hidden"):
            continue
        gx = int(tok.get("grid_x", tok.get("x", 0)))
        gy = int(tok.get("grid_y", tok.get("y", 0)))
        cx = gx * cell_px + cell_px // 2
        cy = gy * cell_px + cell_px // 2
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
    """Generate a `1.0m`-per-cell 3D/2D tactical battlemap (`tiles` + `data/maps/*.svg`) and broadcast `MAP_UPDATED`."""
    width = max(4, min(int(width), 32))
    height = max(4, min(int(height), 32))
    map_id = f"map-{uuid.uuid4().hex[:8]}"
    preset = infer_sector_preset(theme)
    tiles = build_procedural_tiles(width, height, preset)

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
        render_tactical_svg(map_id, theme, width, height, tokens, tiles),
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
        "image_url": f"/maps/{filename}",
        "map_image_url": f"/maps/{filename}",
        "grid_width": width,
        "grid_height": height,
        "tokens": tokens,
        "notes": f"1.0m Tactical Diorama & SVG battlemap for {theme} ({preset})",
    }
    set_session_map(session_id, map_state)
    _sync_map_with_signaling(session_id, map_state)
    return map_state


@tool
def move_map_token(
    session_id: str,
    token_id: str,
    to_x: int,
    to_y: int,
) -> dict[str, Any]:
    """Move a token to `(to_x, to_y)` on the session's `1.0m` tactical map, validating bounds and walls."""
    map_state = get_session_map(session_id)
    if map_state is None:
        return {"error": f"No active tactical map for session '{session_id}'."}

    grid_w = int(map_state.get("grid_width", 16))
    grid_h = int(map_state.get("grid_height", 12))
    tx = int(to_x)
    ty = int(to_y)
    if not (0 <= tx < grid_w and 0 <= ty < grid_h):
        return {
            "error": f"Target coordinates ({to_x}, {to_y}) out of bounds for {grid_w}x{grid_h} grid."
        }

    tiles = list(map_state.get("tiles") or [])
    if len(tiles) == grid_w * grid_h:
        dest_kind = tiles[ty * grid_w + tx]
        if dest_kind in _BLOCKING_TILES:
            return {
                "error": f"Target cell ({tx}, {ty}) is blocked by {dest_kind}."
            }

    tokens = [normalize_token(t) for t in map_state.get("tokens", [])]
    found = False
    for tok in tokens:
        if tok.get("token_id") == token_id or str(tok.get("label", "")).lower() == token_id.lower():
            old_x = int(tok.get("grid_x", tx))
            old_y = int(tok.get("grid_y", ty))
            if (old_x, old_y) != (tx, ty):
                tok["facing_deg"] = int(round(math.degrees(math.atan2(tx - old_x, -(ty - old_y))))) % 360
            tok["grid_x"] = tx
            tok["grid_y"] = ty
            tok["x"] = tx
            tok["y"] = ty
            found = True
            break

    if not found:
        tokens.append(
            normalize_token(
                {
                    "token_id": token_id,
                    "label": token_id,
                    "grid_x": tx,
                    "grid_y": ty,
                    "color_hex": "#a78bfa",
                }
            )
        )

    map_id = str(map_state.get("map_id", "map"))
    theme = str(map_state.get("theme", map_state.get("title", "Tactical Map")))
    maps_dir = _repo_root() / "data" / "maps"
    filename = f"{session_id}_{map_id}.svg"
    svg_path = maps_dir / filename
    if maps_dir.exists():
        try:
            svg_path.write_text(
                render_tactical_svg(map_id, theme, grid_w, grid_h, tokens, tiles or None),
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
    """Inspect the active `1.0m` tactical map geometry, token positions, metric distances, cover `Δ_ext`, and optional VLM analysis."""
    map_state = get_session_map(session_id)
    if map_state is None:
        return f"No active tactical map for session '{session_id}'."

    width = int(map_state.get("grid_width", 16))
    height = int(map_state.get("grid_height", 12))
    tokens = map_state.get("tokens", [])
    preset = str(map_state.get("theme_preset") or infer_sector_preset(str(map_state.get("theme", ""))))
    tiles = list(map_state.get("tiles") or [])
    if len(tiles) != width * height:
        tiles = build_procedural_tiles(width, height, preset)

    obstacle_x_range = (max(2, width // 2 - 1), max(2, width // 2))
    obstacle_y = max(2, height // 2 - 1)
    cover_cells = [
        f"({ idx % width }, { idx // width }):{kind}"
        for idx, kind in enumerate(tiles)
        if kind in {"HALF_COVER", "DOOR_CLOSED", "HAZARD_ZONE", "ELEVATED_PLATFORM"}
    ][:12]

    token_summaries = [
        f"- {t.get('label', t.get('token_id'))} (`{t.get('token_id')}`) at ({t.get('grid_x', t.get('x'))}, {t.get('grid_y', t.get('y'))})"
        for t in tokens
    ]
    pairwise: list[str] = []
    for i in range(len(tokens)):
        for j in range(i + 1, len(tokens)):
            t1, t2 = tokens[i], tokens[j]
            x1 = int(t1.get("grid_x", t1.get("x", 0)))
            y1 = int(t1.get("grid_y", t1.get("y", 0)))
            x2 = int(t2.get("grid_x", t2.get("x", 0)))
            y2 = int(t2.get("grid_y", t2.get("y", 0)))
            dist_m, los_tag, _delta = _evaluate_pair_los_and_cover(
                tiles, width, height, x1, y1, x2, y2
            )
            pairwise.append(
                f"{t1.get('label', t1.get('token_id'))} <-> {t2.get('label', t2.get('token_id'))}: {dist_m}m ({dist_m} cells){los_tag}"
            )

    geometric_summary = (
        f"Map '{map_state.get('map_id')}' ({width}x{height} 1.0m grid, preset={preset}, url={map_state.get('image_url', map_state.get('map_image_url'))}):\n"
        f"Central cover bulkhead at x={obstacle_x_range[0]}..{obstacle_x_range[1]}, y={obstacle_y}. Tactical features: {', '.join(cover_cells)}.\n"
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
    """Ensure a `1.0m` procedural tactical grid is active during Action Mode (`combat_active == True`)
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
            gx = int(c.get("grid_x") if c.get("grid_x") is not None else default_x)
            gy = int(c.get("grid_y") if c.get("grid_y") is not None else default_y)
            zone = str(c.get("zone_label") or ("Entry Bulkhead" if is_hostile else "Core Sector"))
            initial_tokens.append(
                {
                    "token_id": str(c.get("entity_id") or f"comb-{idx}"),
                    "entity_id": str(c.get("entity_id") or f"comb-{idx}"),
                    "label": str(c.get("display_name") or f"Combatant {idx + 1}"),
                    "grid_x": max(1, min(14, gx)),
                    "grid_y": max(1, min(10, gy)),
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
                matched["grid_x"] = max(1, min(width - 2, int(gx)))
                matched["x"] = matched["grid_x"]
                changed = True
            if gy is not None and int(gy) != matched.get("grid_y"):
                matched["grid_y"] = max(1, min(height - 2, int(gy)))
                matched["y"] = matched["grid_y"]
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
