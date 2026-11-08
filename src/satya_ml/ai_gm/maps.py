"""Procedural 2D Tactical Map SVG Generator, Token Movement, and Multimodal Vision Inspector.

Implements Phase 3 of the Satya AI GM architecture:
- `generate_tactical_map`: Generates a coordinate-labeled top-down SVG battlemap under `data/maps/`
  with cover bulkheads, airlock doors, hazard zones, and token badges, and broadcasts `MAP_UPDATED`
  via `POST /api/gm/sessions/:id/map`.
- `move_map_token`: Validates grid bounds, updates token `(grid_x, grid_y)` coordinates, re-renders
  the SVG battlemap on disk, and broadcasts `MAP_UPDATED`.
- `inspect_tactical_map`: Evaluates spatial geometry, pairwise distances, and cover lines of sight,
  and optionally invokes the session's multimodal vision model (`build_gm_chat_model(cfg, for_vision=True)`)
  when a vision model endpoint is reachable.
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


def normalize_token(tok: dict[str, Any]) -> dict[str, Any]:
    """Normalize a token placement dictionary to include both `grid_x`/`grid_y` and `x`/`y` keys."""
    gx = int(tok.get("grid_x", tok.get("x", 0)))
    gy = int(tok.get("grid_y", tok.get("y", 0)))
    return {
        "token_id": str(tok.get("token_id", f"tok-{uuid.uuid4().hex[:6]}")),
        "label": str(tok.get("label", tok.get("token_id", "Token"))),
        "actor": tok.get("actor"),
        "grid_x": gx,
        "grid_y": gy,
        "x": gx,
        "y": gy,
        "color_hex": str(tok.get("color_hex", "#38bdf8")),
        "is_hidden": bool(tok.get("is_hidden", False)),
        "status_badges": list(tok.get("status_badges", [])),
    }


def render_tactical_svg(
    map_id: str,
    theme: str,
    width: int,
    height: int,
    tokens: list[dict[str, Any]],
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

    # Procedural central cover bulkhead
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

    # Map title banner
    safe_theme = (
        theme[:48]
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
    )
    lines.append(
        f'  <text x="20" y="{svg_h - 10}" fill="#94a3b8" font-family="monospace" font-size="12">'
        f"SATYA TACTICAL GRID [{map_id}] // {safe_theme} ({width}x{height})</text>"
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
    wire_payload = {
        "map_id": map_state["map_id"],
        "title": map_state.get("title", map_state.get("theme", "Tactical Map")),
        "image_url": map_state.get("image_url", map_state.get("map_image_url", "")),
        "grid_width": int(map_state["grid_width"]),
        "grid_height": int(map_state["grid_height"]),
        "tokens": wire_tokens,
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
    """Generate a top-down tactical SVG battlemap (`data/maps/`) and broadcast `MAP_UPDATED` to the session."""
    width = max(4, min(int(width), 32))
    height = max(4, min(int(height), 32))
    map_id = f"map-{uuid.uuid4().hex[:8]}"
    raw_tokens = list(initial_tokens or [])
    if not raw_tokens:
        raw_tokens = [
            {
                "token_id": "tok-vance",
                "label": "Capt. Vance",
                "grid_x": 2,
                "grid_y": height // 2,
                "color_hex": "#38bdf8",
            },
            {
                "token_id": "tok-secops-1",
                "label": "SecOps Sentry",
                "grid_x": width - 3,
                "grid_y": height // 2,
                "color_hex": "#f87171",
            },
        ]
    tokens = [normalize_token(t) for t in raw_tokens]

    maps_dir = _repo_root() / "data" / "maps"
    maps_dir.mkdir(parents=True, exist_ok=True)
    filename = f"{session_id}_{map_id}.svg"
    svg_path = maps_dir / filename
    svg_path.write_text(render_tactical_svg(map_id, theme, width, height, tokens), encoding="utf-8")

    map_state: dict[str, Any] = {
        "event_type": "MAP_GENERATED",
        "map_id": map_id,
        "title": theme,
        "theme": theme,
        "image_url": f"/maps/{filename}",
        "map_image_url": f"/maps/{filename}",
        "grid_width": width,
        "grid_height": height,
        "tokens": tokens,
        "notes": f"Procedural SVG battlemap for {theme}",
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
    """Move a token to `(to_x, to_y)` on the session's active tactical map, re-render the SVG, and broadcast `MAP_UPDATED`."""
    map_state = get_session_map(session_id)
    if map_state is None:
        return {"error": f"No active tactical map for session '{session_id}'."}

    grid_w = int(map_state.get("grid_width", 16))
    grid_h = int(map_state.get("grid_height", 12))
    if not (0 <= int(to_x) < grid_w and 0 <= int(to_y) < grid_h):
        return {
            "error": f"Target coordinates ({to_x}, {to_y}) out of bounds for {grid_w}x{grid_h} grid."
        }

    tokens = [normalize_token(t) for t in map_state.get("tokens", [])]
    found = False
    for tok in tokens:
        if tok.get("token_id") == token_id or str(tok.get("label", "")).lower() == token_id.lower():
            tok["grid_x"] = int(to_x)
            tok["grid_y"] = int(to_y)
            tok["x"] = int(to_x)
            tok["y"] = int(to_y)
            found = True
            break

    if not found:
        tokens.append(
            normalize_token(
                {
                    "token_id": token_id,
                    "label": token_id,
                    "grid_x": int(to_x),
                    "grid_y": int(to_y),
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
                render_tactical_svg(map_id, theme, grid_w, grid_h, tokens),
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
    """Inspect the active tactical map geometry, token positions, distances, cover lines of sight, and optional VLM analysis."""
    map_state = get_session_map(session_id)
    if map_state is None:
        return f"No active tactical map for session '{session_id}'."

    width = int(map_state.get("grid_width", 16))
    height = int(map_state.get("grid_height", 12))
    tokens = map_state.get("tokens", [])
    obstacle_x_range = (max(2, width // 2 - 1), max(2, width // 2))
    obstacle_y = max(2, height // 2 - 1)

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
            dist = round(math.hypot(x1 - x2, y1 - y2), 1)
            # Check if central bulkhead lies between the two tokens
            crosses_cover = (
                min(x1, x2) <= obstacle_x_range[0]
                and max(x1, x2) >= obstacle_x_range[1]
                and min(y1, y2) <= obstacle_y <= max(y1, y2)
            )
            los_tag = " [PARTIAL COVER / OBSTRUCTED]" if crosses_cover else " [CLEAR LOS]"
            pairwise.append(
                f"{t1.get('label', t1.get('token_id'))} <-> {t2.get('label', t2.get('token_id'))}: {dist} cells{los_tag}"
            )

    geometric_summary = (
        f"Map '{map_state.get('map_id')}' ({width}x{height} grid, url={map_state.get('image_url', map_state.get('map_image_url'))}):\n"
        f"Central cover bulkhead at x={obstacle_x_range[0]}..{obstacle_x_range[1]}, y={obstacle_y}.\n"
        f"Tokens:\n" + "\n".join(token_summaries) + "\n"
        f"Pairwise Distances & Line of Sight: {', '.join(pairwise) if pairwise else 'N/A'}\n"
        f"Query context: {question}"
    )

    # Optional VLM invocation when a session vision model is configured and a raster PNG/JPEG is present
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
    """Stubbed hook for future Procedural Environment & Tactical Map Generation.

    When Action Mode (`combat_active == True`) is initiated or combatant coordinates update,
    initializes a default 16x12 SVG tactical grid if no active map exists yet, and synchronizes
    player and NPC token placements (`(grid_x, grid_y)` and tactical zone labels) with
    `satya-signaling` (`POST /api/gm/sessions/:id/map`).
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
                    "grid_x": gx,
                    "grid_y": gy,
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
                matched["grid_x"] = max(0, min(width - 1, int(gx)))
                matched["x"] = matched["grid_x"]
                changed = True
            if gy is not None and int(gy) != matched.get("grid_y"):
                matched["grid_y"] = max(0, min(height - 1, int(gy)))
                matched["y"] = matched["grid_y"]
                changed = True
        elif eid or dname:
            role_str = str(c.get("role_or_faction") or "").lower()
            is_hostile = bool(c.get("is_npc")) or "npc" in role_str or "hostile" in role_str
            nx = max(0, min(width - 1, int(gx if gx is not None else (12 if is_hostile else 3))))
            ny = max(0, min(height - 1, int(gy if gy is not None else (4 + idx % 5))))
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

