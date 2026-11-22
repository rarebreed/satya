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
import uuid
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import Any, Final, Literal

try:
    import httpx
except ImportError:  # pragma: no cover
    httpx = None  # type: ignore[assignment]

from satya_py import maps as _maps, schema as _schema

from satya_ml.ai_gm.models import SessionLlmConfig, build_gm_chat_model
from satya_ml.campaign_config import (
    get_active_campaign_pack,
    get_pack_default_tokens,
    get_pack_zone_labels,
)
from satya_ml.ai_gm.tool_spec import SessionStore, satya_gm_tool
from satya_ml.signaling_client import (
    OFFLINE_SIGNALING_VALUES as _OFFLINE_SIGNALING_VALUES,
    http_post_json,
    repo_root as _repo_root,
    service_auth_headers,
    signaling_base_url,
)

try:
    from langchain_core.messages import HumanMessage
except ImportError:  # pragma: no cover
    HumanMessage = None  # type: ignore[assignment,misc]


type SensoryChannel = Literal["SIGHT", "HEARING", "OLFACTION", "INTUITION"]
type SectorPresetId = Literal[
    "community_clinic_annex",
    "corporate_server_bunker",
    "satyang_safehouse",
    "brownout_logistics_depot",
    "woodland_creek",
    "roadside_ditch",
    "med_lab",
    "hospital",
    "shopping_mall",
    "shopping_plaza",
    "botanical_garden",
    "luxury_mansion",
    "projects_high_density",
    "suburban_neighborhood",
    "rural_countryside",
    "farms",
    "warehouse",
    "manufacturing_plant",
    "downtown_commercial",
    "urban_residential",
    "garden_estate",
    "wooded_park",
    "corner_bar",
    "office_floor",
    "residential_apartment",
    "docking_bay",
    "urban_crossroads",
    "derelict_corridor",
    "orbital_airlock",
]

_SESSION_MAPS: Final[SessionStore[dict[str, Any]]] = SessionStore()
_SESSION_LLM_CONFIGS: Final[SessionStore[SessionLlmConfig]] = SessionStore()


def using_satya_py_map_backend() -> bool:
    """Return ``True``; all tactical map collision, LoS, and procedural generation runs via Rust ``satya_py.maps``."""
    return True


_TOKEN_RADIUS_M: Final[float] = 0.30
# Sourced from Rust `satya_maps::map_dressing::PROP_CATALOG` (zero Python enum list drift).
_NON_SOLID_PROP_KINDS: Final[frozenset[str]] = frozenset(_maps.list_non_solid_prop_kinds())
_SCIFI_SECTOR_PRESET_REMAP: Final[Mapping[str, SectorPresetId]] = MappingProxyType(
    {
        "orbital_airlock": "corporate_server_bunker",
        "docking_bay": "brownout_logistics_depot",
        "derelict_corridor": "satyang_safehouse",
        "med_lab": "community_clinic_annex",
    }
)


def clear_session_map_caches(session_id: str | None = None) -> None:
    """Clear in-memory tactical map, LLM config, and spatial pin caches for `session_id` (or all)."""
    _SESSION_MAPS.reset(session_id)
    _SESSION_LLM_CONFIGS.reset(session_id)
    _SESSION_SPATIAL_PINS.reset(session_id)


def _signaling_base_url() -> str:
    return signaling_base_url()


def _gm_service_token() -> str:
    from satya_ml.ai_gm.tools import _gm_service_token as _shared_gm_service_token

    return _shared_gm_service_token()


def _gm_headers() -> dict[str, str]:
    headers = service_auth_headers(include_trace=False)
    headers["Authorization"] = f"Bearer {_gm_service_token()}"
    return headers


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


@dataclass(frozen=True, slots=True)
class _SectorPresetRule:
    """Ordered phrase/word matcher rule mapping scene text to a `satya-engine` `SectorPreset`."""

    preset_id: SectorPresetId
    phrases: tuple[str, ...]
    whole_words: tuple[str, ...] = ()
    secondary_phrases: tuple[str, ...] = ()

    def matches(self, low: str, words: set[str]) -> bool:
        """Return ``True`` when scene text satisfies primary and optional secondary terms."""
        primary_hit = any(k in low for k in self.phrases) or any(w in words for w in self.whole_words)
        if not primary_hit:
            return False
        return not self.secondary_phrases or any(k in low for k in self.secondary_phrases)


# Specific phrases and setting venues appear ahead of generic keywords.
_SECTOR_PRESET_RULES: Final[tuple[_SectorPresetRule, ...]] = (
    _SectorPresetRule("community_clinic_annex", ("community_clinic_annex", "community clinic", "clinic annex", "free clinic", "triage annex", "street clinic")),
    _SectorPresetRule("corporate_server_bunker", ("corporate_server_bunker", "server bunker", "corporate bunker", "data bunker", "data center", "black site", "server vault", "faraday vault"), ("scif",)),
    _SectorPresetRule(
        "satyang_safehouse",
        (
            "satyang_safehouse",
            "satyang safehouse",
            "satyang cell",
            "satyang relay",
            "satyang hideout",
            "satyang enclave",
            "awakened safehouse",
            "cell safehouse",
            "dissident safehouse",
            "mesh safehouse",
            "relay safehouse",
        ),
    ),
    _SectorPresetRule("brownout_logistics_depot", ("brownout_logistics_depot", "brownout", "ration depot", "microgrid depot")),
    _SectorPresetRule("woodland_creek", ("creek", "stream", "brook", "river", "ravine")),
    _SectorPresetRule("roadside_ditch", ("ditch", "trench", "culvert", "country lane")),
    _SectorPresetRule("med_lab", ("med_lab", "med-lab", "medlab", "biolab")),
    _SectorPresetRule("hospital", ("hospital", "medical center", "emergency room", "inpatient", "operating room", "triage")),
    _SectorPresetRule("shopping_mall", ("shopping_mall", "shopping mall", "enclosed mall", "galleria", "food court")),
    _SectorPresetRule("shopping_plaza", ("shopping_plaza", "shopping plaza", "outdoor plaza", "retail plaza", "lifestyle center", "plaza")),
    _SectorPresetRule("botanical_garden", ("botanical_garden", "botanical garden", "botanical", "conservatory", "arboretum", "greenhouse")),
    _SectorPresetRule("luxury_mansion", ("luxury mansion", "luxury_mansion", "manor", "penthouse estate", "mansion")),
    _SectorPresetRule("projects_high_density", ("projects_high_density", "projects", "public housing", "tenement", "housing block")),
    _SectorPresetRule("suburban_neighborhood", ("suburban_neighborhood", "suburb", "suburban", "cul-de-sac", "neighborhood")),
    _SectorPresetRule("rural_countryside", ("rural_countryside", "rural", "countryside", "farmhouse", "homestead")),
    _SectorPresetRule("farms", ("farms", "farmland", "cropland", "orchard", "pasture", "acreage", "barn"), ("farm",)),
    _SectorPresetRule("brownout_logistics_depot", ("warehouse", "distribution center", "logistics", "pallet rack", "freight depot"), (), ("brownout", "blackout", "microgrid", "ration depot", "logistics depot")),
    _SectorPresetRule("warehouse", ("warehouse", "distribution center", "logistics", "pallet rack", "freight depot")),
    _SectorPresetRule("manufacturing_plant", ("manufacturing_plant", "manufacturing plant", "manufacturing", "factory", "assembly line", "foundry", "stamping", "industrial plant"), ("plant",)),
    _SectorPresetRule("downtown_commercial", ("downtown_commercial", "downtown commercial", "retail strip", "shopping strip", "storefront", "strip mall", "commercial center")),
    _SectorPresetRule("urban_residential", ("urban_residential", "urban residential", "townhouse", "brownstone", "rowhouse", "condo")),
    _SectorPresetRule("garden_estate", ("estate", "compound", "courtyard", "garden", "yard", "villa")),
    _SectorPresetRule("wooded_park", ("park", "woods", "forest", "trail", "clearing", "grove")),
    _SectorPresetRule("corner_bar", ("bar", "pub", "tavern", "club", "lounge", "saloon")),
    _SectorPresetRule("office_floor", ("office", "corporate", "agency", "cubicle", "suite", "commercial")),
    _SectorPresetRule("residential_apartment", ("apartment", "flat", "safehouse", "house", "home", "residential", "motel")),
    _SectorPresetRule("docking_bay", ("dock", "cargo", "hangar", "bay")),
    _SectorPresetRule("med_lab", ("med", "trauma", "lab", "clinic", "ward")),
    _SectorPresetRule("urban_crossroads", ("urban", "street", "road", "city", "crossroad")),
    _SectorPresetRule("derelict_corridor", ("corridor", "derelict", "breach", "tunnel", "bunker")),
    _SectorPresetRule("orbital_airlock", ("orbital", "airlock", "station")),
)


def infer_sector_preset(theme: str) -> str:
    """Map a free-text scene theme to one of the 29 canonical `satya-engine` `SectorPreset` codes."""
    low = (theme or "").lower()
    words = set(low.replace("-", " ").replace("_", " ").split())
    return next(
        (rule.preset_id for rule in _SECTOR_PRESET_RULES if rule.matches(low, words)),
        str(get_active_campaign_pack().get("default_sector_preset") or "urban_crossroads"),
    )


def _infer_model_preset(label: str, is_hidden: bool = False) -> str:
    return _maps.infer_token_model_preset(label, is_hidden)


def normalize_token(tok: Mapping[str, Any]) -> dict[str, Any]:
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
    vis = str(tok.get("visibility") or ("GM_ONLY" if tok.get("gm_only") else "PUBLIC")).upper()
    hidden = bool(tok.get("is_hidden") or tok.get("hidden") or tok.get("gm_only") or vis == "GM_ONLY")
    if hidden:
        vis = "GM_ONLY"
    out: dict[str, Any] = {
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
        "floor_index": int(tok.get("floor_index", 0)),
        "elevation_cm": int(tok.get("elevation_cm", 0)),
        "is_hidden": hidden,
        "gm_only": bool(tok.get("gm_only") or hidden),
        "visibility": vis,
        "status_badges": list(tok.get("status_badges", [])),
    }
    if tok.get("faction") is not None:
        out["faction"] = str(tok.get("faction"))
    if tok.get("stance_pose") is not None:
        out["stance_pose"] = str(tok.get("stance_pose"))
    if tok.get("portrait_url") is not None:
        out["portrait_url"] = str(tok.get("portrait_url"))
    return out


def _wall_edge_segment_m(edge: dict[str, Any]) -> tuple[float, float, float, float]:
    ex = float(edge.get("x", 0))
    ey = float(edge.get("y", 0))
    side = str(edge.get("side", "NORTH")).upper()
    if side == "WEST":
        return (ex, ey, ex, ey + 1.0)
    return (ex, ey, ex + 1.0, ey)


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
    rot_deg = float(prop.get("rotation_deg", 0))
    if prop.get("span_w_cm") is not None and prop.get("span_h_cm") is not None:
        sw_m = float(prop["span_w_cm"]) / 100.0
        sh_m = float(prop["span_h_cm"]) / 100.0
    else:
        w_cells = int(prop.get("w_cells", prop.get("span_w", 1)))
        h_cells = int(prop.get("h_cells", prop.get("span_h", 1)))
        sw_m = float(w_cells if w_cells > 0 else 1)
        sh_m = float(h_cells if h_cells > 0 else 1)
        if int(rot_deg) % 180 == 90:
            sw_m, sh_m = sh_m, sw_m
    rot_rad = math.radians(rot_deg)
    return (cx_m, cy_m, sw_m * 0.5, sh_m * 0.5, rot_rad)


def _evaluate_pair_los_and_cover(
    wall_edges: list[dict[str, Any]],
    props: list[dict[str, Any]],
    x1_m: float,
    y1_m: float,
    x2_m: float,
    y2_m: float,
) -> tuple[float, str, int]:
    """Evaluate 2D segment/OBB LoS & cover via Rust ``satya_py.maps.evaluate_los_and_cover``."""
    dist_m = round(math.hypot(x1_m - x2_m, y1_m - y2_m), 1)
    if dist_m <= 1e-4:
        return (0.0, " [CLEAR LOS]", 0)

    scene = {
        "map_id": "los",
        "title": "los",
        "image_url": "",
        "grid_width": max(240, int(max(x1_m, x2_m)) + 8),
        "grid_height": max(180, int(max(y1_m, y2_m)) + 8),
        "wall_edges": wall_edges,
        "props": props,
    }
    ev_ab = _maps.evaluate_los_and_cover(scene, x1_m, y1_m, x2_m, y2_m)
    ev_ba = _maps.evaluate_los_and_cover(scene, x2_m, y2_m, x1_m, y1_m)
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


def _merge_tokens_with_visuals(map_state: dict[str, Any]) -> list[dict[str, Any]]:
    """Merge `token_visuals[]` metadata (floor, elevation, continuous cm coords, pose, preset) into `tokens`."""
    active_floor = int(map_state.get("active_floor", 0))
    vis_by_eid = {
        str(v.get("entity_id") or ""): v
        for v in (map_state.get("token_visuals") or [])
        if isinstance(v, dict) and v.get("entity_id")
    }
    merged: list[dict[str, Any]] = []
    for raw in map_state.get("tokens") or []:
        if not isinstance(raw, dict):
            continue
        tok = dict(raw)
        eid = str(tok.get("entity_id") or tok.get("token_id") or "")
        vis_rec = vis_by_eid.get(eid) or {}
        for k in (
            "floor_index",
            "elevation_cm",
            "pos_x_cm",
            "pos_y_cm",
            "stance_pose",
            "portrait_url",
            "model_preset",
            "color_hex",
            "facing_deg",
        ):
            if tok.get(k) is None and vis_rec.get(k) is not None:
                tok[k] = vis_rec[k]
        if tok.get("floor_index") is None:
            tok["floor_index"] = active_floor
        merged.append(normalize_token(tok))
    return merged


def _build_wire_tokens_and_visuals(
    map_state: dict[str, Any],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    active_floor = int(map_state.get("active_floor", 0))
    merged = _merge_tokens_with_visuals(map_state)
    norm = _schema.normalize_tactical_map(
        {
            "active_floor": active_floor,
            "tokens": merged,
            "token_visuals": list(map_state.get("token_visuals") or []),
        }
    )
    return list(norm.get("tokens") or []), list(norm.get("token_visuals") or [])


def _sync_map_with_signaling(session_id: str, map_state: dict[str, Any]) -> None:
    wire_payload = _build_rust_map_payload(map_state)
    base = _signaling_base_url()
    if not base:
        return
    http_post_json(
        f"{base}/api/gm/sessions/{session_id}/map",
        wire_payload,
        timeout=2.0,
        include_trace=False,
        record_event_id=False,
        client_cls=httpx.Client if httpx is not None else None,
    )


_SESSION_SPATIAL_PINS: Final[SessionStore[list[dict[str, Any]]]] = SessionStore()

_CHANNEL_ALIAS_MAP: Final[Mapping[str, SensoryChannel]] = MappingProxyType(
    {
        "SIG": "SIGHT",
        "SIGHT": "SIGHT",
        "VISUAL": "SIGHT",
        "HEA": "HEARING",
        "HEARING": "HEARING",
        "SOUND": "HEARING",
        "AUDITORY": "HEARING",
        "OLF": "OLFACTION",
        "OLFACTION": "OLFACTION",
        "SMELL": "OLFACTION",
        "SCENT": "OLFACTION",
        "ITU": "INTUITION",
        "INTUITION": "INTUITION",
        "INSTINCT": "INTUITION",
    }
)


def get_session_spatial_pins(session_id: str) -> list[dict[str, Any]]:
    """Return all registered spatial sensory clue pins for `session_id`."""
    return list(_SESSION_SPATIAL_PINS.get(session_id, []))


def clear_session_spatial_pins(session_id: str | None = None) -> None:
    """Clear spatial clue pins for `session_id` (or all sessions when omitted)."""
    _SESSION_SPATIAL_PINS.reset(session_id)


def _build_rust_map_payload(map_state: dict[str, Any]) -> dict[str, Any]:
    """Build a canonical `GmMapUpdateRequest`-compatible payload dict via Rust ``satya_py.schema``."""
    payload = dict(map_state)
    if not payload.get("theme_preset"):
        theme = str(payload.get("theme") or payload.get("title") or "").strip()
        payload["theme_preset"] = infer_sector_preset(theme) if theme else "urban_crossroads"
    if payload.get("seed") is None:
        payload["seed"] = 42
    if payload.get("tokens"):
        payload["tokens"] = _merge_tokens_with_visuals(map_state)
    return _schema.normalize_tactical_map(payload)


def _extract_narrative_manifest(map_state: dict[str, Any]) -> dict[str, Any]:
    """Compute semantic `SectorNarrativeManifest` dict via Rust ``satya_py.maps``."""
    try:
        parsed = _maps.extract_narrative_manifest(_build_rust_map_payload(map_state))
        if isinstance(parsed, dict):
            return parsed
    except Exception:
        pass
    return {}


def _extract_floor_geometry(
    map_state: dict[str, Any],
    floor_index: int | None = None,
    fallback_walls: list[dict[str, Any]] | None = None,
    fallback_props: list[dict[str, Any]] | None = None,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Return `(wall_edges, props)` for `floor_index`, using `floors` when present."""
    active_floor = int(map_state.get("active_floor", 0))
    target_floor = active_floor if floor_index is None else int(floor_index)
    for fl in list(map_state.get("floors") or []):
        if isinstance(fl, dict) and int(fl.get("floor_index", 0)) == target_floor:
            return (
                list(fl.get("wall_edges") or []),
                list(fl.get("props") or []),
            )
    return (
        list(fallback_walls if fallback_walls is not None else (map_state.get("wall_edges") or [])),
        list(fallback_props if fallback_props is not None else (map_state.get("props") or [])),
    )


def _check_rust_collision(
    map_payload: dict[str, Any] | str,
    x_m: float,
    y_m: float,
    floor_index: int | None = None,
) -> bool:
    """Invoke ``satya_py.maps.check_token_collision`` forwarding `floor_index`."""
    return _maps.check_token_collision(
        map_payload,
        x_m,
        y_m,
        _TOKEN_RADIUS_M,
        floor_index=floor_index,
    )


def _find_unblocked_token_pos_cm(
    map_payload: dict[str, Any] | str,
    grid_w: int,
    grid_h: int,
    pref_x_cm: int,
    pref_y_cm: int,
    floor_index: int | None = None,
) -> tuple[int, int]:
    """Adjust `(pref_x_cm, pref_y_cm)` to a collision-free continuous coordinate inside the sector."""
    min_cm = 55
    max_x = max(min_cm + 10, grid_w * 100 - 55)
    max_y = max(min_cm + 10, grid_h * 100 - 55)
    px = max(min_cm, min(max_x, int(pref_x_cm)))
    py = max(min_cm, min(max_y, int(pref_y_cm)))
    try:
        if not _check_rust_collision(map_payload, px / 100.0, py / 100.0, floor_index):
            return px, py
        for radius_steps in range(1, max(grid_w, grid_h) * 2):
            delta = radius_steps * 50
            for dx, dy in (
                (delta, 0), (-delta, 0), (0, delta), (0, -delta),
                (delta, delta), (-delta, delta), (delta, -delta), (-delta, -delta),
            ):
                cx = max(min_cm, min(max_x, px + dx))
                cy = max(min_cm, min(max_y, py + dy))
                if not _check_rust_collision(map_payload, cx / 100.0, cy / 100.0, floor_index):
                    return cx, cy
    except Exception:
        pass
    return px, py


def _write_and_sync_map_state(session_id: str, map_state: dict[str, Any]) -> dict[str, Any]:
    """Write SVG preview on disk, attach narrative manifest, cache in memory, and sync signaling."""
    grid_w = int(map_state.get("grid_width", 16))
    grid_h = int(map_state.get("grid_height", 12))
    map_id = str(map_state.get("map_id", "map"))
    theme = str(map_state.get("theme", map_state.get("title", "Tactical Map")))
    tokens = list(map_state.get("tokens") or [])
    tiles = list(map_state.get("tiles") or [])
    wall_edges = list(map_state.get("wall_edges") or [])
    props = list(map_state.get("props") or [])

    maps_dir = _repo_root() / "data" / "maps"
    maps_dir.mkdir(parents=True, exist_ok=True)
    filename = f"{session_id}_{map_id}.svg"
    svg_path = maps_dir / filename
    try:
        svg_path.write_text(
            render_tactical_svg(map_id, theme, grid_w, grid_h, tokens, tiles, wall_edges, props),
            encoding="utf-8",
        )
    except OSError:
        pass

    manifest = _extract_narrative_manifest(map_state)
    if manifest:
        map_state["narrative_manifest"] = manifest
    set_session_map(session_id, map_state)
    _sync_map_with_signaling(session_id, map_state)
    return map_state


@satya_gm_tool(pack="tactical_maps", tier="TIER_1_SATYA_CORE")
def generate_tactical_map(
    session_id: str,
    theme: str,
    width: int | None = None,
    height: int | None = None,
    initial_tokens: list[dict[str, Any]] | None = None,
    seed: int = 42,
    economic_tier: str | None = None,
    zoning: str | None = None,
    scale: str | None = None,
) -> dict[str, Any]:
    """Generate a continuous vector 3D/2D tactical battlemap (backed by `satya-maps` procedural multi-floor sectors) and broadcast `MAP_UPDATED`."""
    map_id = f"map-{uuid.uuid4().hex[:8]}"
    preset = infer_sector_preset(theme)
    if not bool(get_active_campaign_pack().get("allow_scifi_sector_presets", True)):
        preset = _SCIFI_SECTOR_PRESET_REMAP.get(preset, preset)

    req_w = int(width) if width is not None else None
    req_h = int(height) if height is not None else None
    engine_map = _maps.generate_procedural_sector(
        map_id,
        preset,
        theme,
        req_w,
        req_h,
        int(seed),
        economic_tier,
        zoning,
        scale,
    )
    w = int(engine_map.get("grid_width", req_w or 24))
    h = int(engine_map.get("grid_height", req_h or 20))
    wall_edges = list(engine_map.get("wall_edges") or [])
    props = list(engine_map.get("props") or [])
    surfaces = list(engine_map.get("surfaces") or [])
    floors = list(engine_map.get("floors") or [])
    active_floor = int(engine_map.get("active_floor", 0))
    world_sectors = list(engine_map.get("world_sectors") or [])
    ambient_light_delta_ext = int(engine_map.get("ambient_light_delta_ext", 0) or 0)
    faraday_zones = list(engine_map.get("faraday_zones") or [])
    surveillance_nodes = list(engine_map.get("surveillance_nodes") or [])
    ground_floor_override = engine_map.get("ground_floor_override")
    ground_tint_rgb = engine_map.get("ground_tint_rgb")
    notes = str(
        engine_map.get("notes")
        or f"Continuous Vector Tactical Diorama & SVG battlemap for {theme} ({preset})"
    )

    tiles: list[str] = []
    raw_tokens: Sequence[Mapping[str, Any]] = (
        list(initial_tokens) if initial_tokens else get_pack_default_tokens(w, h)
    )
    tokens = _merge_tokens_with_visuals({"active_floor": active_floor, "tokens": list(raw_tokens)})

    probe_json = _build_rust_map_payload(
        {
            "map_id": map_id,
            "title": theme,
            "grid_width": w,
            "grid_height": h,
            "wall_edges": wall_edges,
            "props": props,
            "surfaces": surfaces,
            "floors": floors,
            "active_floor": active_floor,
            "tokens": [],
        }
    )
    for tok in tokens:
        px_cm = int(tok.get("pos_x_cm", 250))
        py_cm = int(tok.get("pos_y_cm", 250))
        tok_floor = int(tok.get("floor_index", active_floor))
        tok["floor_index"] = tok_floor
        tok["elevation_cm"] = next(
            (
                int(fl.get("elevation_cm", tok_floor * 300))
                for fl in floors
                if isinstance(fl, dict) and int(fl.get("floor_index", 0)) == tok_floor
            ),
            tok_floor * 300,
        )
        nx_cm, ny_cm = _find_unblocked_token_pos_cm(
            probe_json, w, h, px_cm, py_cm, floor_index=tok_floor
        )
        tok["pos_x_cm"] = nx_cm
        tok["pos_y_cm"] = ny_cm
        tok["grid_x"] = max(0, min(w - 1, nx_cm // 100))
        tok["grid_y"] = max(0, min(h - 1, ny_cm // 100))
        tok["x"] = tok["grid_x"]
        tok["y"] = tok["grid_y"]
        tok.setdefault("entity_id", tok.get("token_id"))

    _, wire_visuals = _build_wire_tokens_and_visuals(
        {"active_floor": active_floor, "tokens": tokens}
    )
    filename = f"{session_id}_{map_id}.svg"
    map_state: dict[str, Any] = {
        "event_type": "MAP_GENERATED",
        "map_id": map_id,
        "title": theme,
        "theme": theme,
        "theme_preset": preset,
        "cell_size_cm": 100,
        "seed": int(seed),
        "tiles": tiles,
        "wall_edges": wall_edges,
        "props": props,
        "surfaces": surfaces,
        "floors": floors,
        "active_floor": active_floor,
        "world_sectors": world_sectors,
        "ambient_light_delta_ext": ambient_light_delta_ext,
        "faraday_zones": faraday_zones,
        "surveillance_nodes": surveillance_nodes,
        "image_url": f"/maps/{filename}",
        "map_image_url": f"/maps/{filename}",
        "grid_width": w,
        "grid_height": h,
        "tokens": tokens,
        "token_visuals": wire_visuals,
        "notes": notes,
    }
    if ground_floor_override is not None:
        map_state["ground_floor_override"] = ground_floor_override
    if ground_tint_rgb is not None:
        map_state["ground_tint_rgb"] = ground_tint_rgb
    return _write_and_sync_map_state(session_id, map_state)


@satya_gm_tool(pack="tactical_maps", tier="TIER_1_SATYA_CORE")
def continue_tactical_map(
    session_id: str,
    direction: str,
    preset_override: str | None = None,
    seed: int = 0,
) -> dict[str, Any]:
    """Continue the active macro-world tactical map seamlessly into the neighboring NORTH, SOUTH, EAST, or WEST sector."""
    cur = get_session_map(session_id)
    if cur is None:
        return {"error": f"No active tactical map for session '{session_id}'."}
    working = dict(cur)
    working["tokens"] = _merge_tokens_with_visuals(working)
    preset_override_arg = (
        infer_sector_preset(preset_override)
        if (preset_override and preset_override.strip())
        else None
    )
    if preset_override_arg:
        working["theme_preset"] = preset_override_arg
    try:
        cont = _maps.continue_procedural_map(
            _build_rust_map_payload(working),
            direction.strip().upper(),
            int(seed) if seed else None,
            preset_override_arg,
        )
    except Exception as exc:
        return {"error": f"Failed to continue map '{direction}': {exc}"}

    updated = dict(cur)
    updated["event_type"] = "MAP_CONTINUED"
    updated["grid_width"] = int(cont.get("grid_width", cur.get("grid_width", 24)))
    updated["grid_height"] = int(cont.get("grid_height", cur.get("grid_height", 20)))
    updated["wall_edges"] = list(cont.get("wall_edges") or [])
    updated["props"] = list(cont.get("props") or [])
    updated["surfaces"] = list(cont.get("surfaces") or [])
    updated["floors"] = list(cont.get("floors") or [])
    updated["active_floor"] = int(cont.get("active_floor", 0))
    updated["world_sectors"] = list(cont.get("world_sectors") or [])
    updated["ambient_light_delta_ext"] = int(
        cont.get("ambient_light_delta_ext", cur.get("ambient_light_delta_ext", 0)) or 0
    )
    updated["faraday_zones"] = list(cont.get("faraday_zones") or [])
    updated["surveillance_nodes"] = list(cont.get("surveillance_nodes") or [])
    if "token_visuals" in cont and isinstance(cont.get("token_visuals"), list):
        updated["token_visuals"] = list(cont["token_visuals"])
    if "tokens" in cont and isinstance(cont.get("tokens"), list):
        updated["tokens"] = _merge_tokens_with_visuals(cont)
    else:
        updated["tokens"] = _merge_tokens_with_visuals(updated)
    if "ground_floor_override" in cont:
        updated["ground_floor_override"] = cont["ground_floor_override"]
    else:
        updated.pop("ground_floor_override", None)
    if "ground_tint_rgb" in cont:
        updated["ground_tint_rgb"] = cont["ground_tint_rgb"]
    else:
        updated.pop("ground_tint_rgb", None)
    updated["theme_preset"] = str(cont.get("theme_preset") or cur.get("theme_preset", "urban_crossroads"))
    updated["notes"] = str(cont.get("notes") or cur.get("notes", ""))
    return _write_and_sync_map_state(session_id, updated)


@satya_gm_tool(pack="tactical_maps", tier="TIER_1_SATYA_CORE")
def switch_tactical_map_floor(session_id: str, target_floor_index: int) -> dict[str, Any]:
    """Switch the active vertical floor (`floor_index` such as `-1`, `0`, `1`) on a multi-story procedural map."""
    cur = get_session_map(session_id)
    if cur is None:
        return {"error": f"No active tactical map for session '{session_id}'."}
    try:
        switched = _maps.switch_map_floor(
            _build_rust_map_payload(cur),
            int(target_floor_index),
        )
    except Exception as exc:
        return {"error": f"Failed to switch floor to {target_floor_index}: {exc}"}

    updated = dict(cur)
    updated["event_type"] = "FLOOR_SWITCHED"
    updated["active_floor"] = int(switched.get("active_floor", target_floor_index))
    updated["wall_edges"] = list(switched.get("wall_edges") or [])
    updated["props"] = list(switched.get("props") or [])
    updated["surfaces"] = list(switched.get("surfaces") or [])
    updated["floors"] = list(switched.get("floors") or [])
    updated["tokens"] = _merge_tokens_with_visuals(cur)
    updated["token_visuals"] = _build_wire_tokens_and_visuals(
        {
            "active_floor": updated["active_floor"],
            "tokens": updated["tokens"],
            "token_visuals": cur.get("token_visuals") or [],
        }
    )[1]
    return _write_and_sync_map_state(session_id, updated)


@satya_gm_tool(pack="tactical_maps", tier="TIER_1_SATYA_CORE")
def register_spatial_clue_pin(
    session_id: str,
    pin_id: str,
    channel: str,
    pos_x_m: float,
    pos_y_m: float,
    radius_m: float,
    whisper_text: str,
    base_difficulty_mod: int = 0,
    floor_index: int = 0,
    requires_los: bool = False,
    one_shot: bool = True,
    grants_audit_proof_tag: str = "",
) -> dict[str, Any]:
    """Place a hidden spatial sensory clue (`SIG`/`HEA`/`OLF`/`ITU`) that whispers automatically during token movement (and optionally mints a verified `AuditProofToken`)."""
    norm_channel = _CHANNEL_ALIAS_MAP.get(str(channel or "SIGHT").strip().upper(), "SIGHT")
    pin = {
        "pin_id": str(pin_id).strip() or f"pin-{uuid.uuid4().hex[:6]}",
        "x_m": float(pos_x_m),
        "y_m": float(pos_y_m),
        "floor_index": int(floor_index),
        "channel": norm_channel,
        "difficulty_tn": max(2, 10 + int(base_difficulty_mod)),
        "max_range_m": max(1.0, float(radius_m)),
        "whisper_text": str(whisper_text),
        "revealed_to": [],
        "requires_los": bool(requires_los),
        "one_shot": bool(one_shot),
        "grants_audit_proof_tag": str(grants_audit_proof_tag or "").strip(),
    }
    pins = [p for p in _SESSION_SPATIAL_PINS.get(session_id, []) if p.get("pin_id") != pin["pin_id"]]
    pins.append(pin)
    _SESSION_SPATIAL_PINS[session_id] = pins
    return {
        "status": "ok",
        "session_id": session_id,
        "pin": pin,
        "pin_count": len(pins),
    }


@satya_gm_tool(pack="combat", tier="TIER_1_SATYA_CORE")
def move_map_token(
    session_id: str,
    token_id: str,
    to_x: int = 0,
    to_y: int = 0,
    to_x_cm: int | None = None,
    to_y_cm: int | None = None,
    sensory_tns: dict[str, int] | None = None,
    to_floor: int | None = None,
) -> dict[str, Any]:
    """Move a token continuously to `(to_x_cm, to_y_cm)` (or cell `(to_x, to_y)` center) on its floor or `to_floor`, validating bounds, walls, OBB obstacles, and passive sensory clue whispers."""
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

    active_floor = int(map_state.get("active_floor", 0))
    tokens = _merge_tokens_with_visuals(map_state)
    matched_tok = next(
        (
            tok
            for tok in tokens
            if tok.get("token_id") == token_id
            or tok.get("entity_id") == token_id
            or str(tok.get("label", "")).lower() == token_id.lower()
        ),
        None,
    )
    target_floor = (
        int(to_floor)
        if to_floor is not None
        else int(matched_tok.get("floor_index", active_floor) if matched_tok is not None else active_floor)
    )
    target_elev_cm = next(
        (
            int(fl.get("elevation_cm", target_floor * 300))
            for fl in (map_state.get("floors") or [])
            if isinstance(fl, dict) and int(fl.get("floor_index", 0)) == target_floor
        ),
        target_floor * 300,
    )

    tx_m = tx_cm / 100.0
    ty_m = ty_cm / 100.0
    wall_edges, props = _extract_floor_geometry(map_state, target_floor)
    rust_map_json = _build_rust_map_payload(map_state)

    if _check_rust_collision(rust_map_json, tx_m, ty_m, floor_index=target_floor):
        return {
            "error": f"Target position ({tx_m:.2f}m, {ty_m:.2f}m) on floor {target_floor} collides with a wall or solid obstacle."
        }

    tx_cell = max(0, min(grid_w - 1, tx_cm // 100))
    ty_cell = max(0, min(grid_h - 1, ty_cm // 100))
    moved_entity_id = token_id
    if matched_tok is not None:
        old_x_cm = int(matched_tok.get("pos_x_cm", tx_cm))
        old_y_cm = int(matched_tok.get("pos_y_cm", ty_cm))
        if (old_x_cm, old_y_cm) != (tx_cm, ty_cm):
            matched_tok["facing_deg"] = int(round(math.degrees(math.atan2(tx_cm - old_x_cm, -(ty_cm - old_y_cm))))) % 360
        matched_tok["pos_x_cm"] = tx_cm
        matched_tok["pos_y_cm"] = ty_cm
        matched_tok["grid_x"] = tx_cell
        matched_tok["grid_y"] = ty_cell
        matched_tok["x"] = tx_cell
        matched_tok["y"] = ty_cell
        matched_tok["floor_index"] = target_floor
        matched_tok["elevation_cm"] = target_elev_cm
        moved_entity_id = str(matched_tok.get("entity_id") or matched_tok.get("token_id") or token_id)
    else:
        tokens.append(
            normalize_token(
                {
                    "token_id": token_id,
                    "entity_id": token_id,
                    "label": token_id,
                    "pos_x_cm": tx_cm,
                    "pos_y_cm": ty_cm,
                    "grid_x": tx_cell,
                    "grid_y": ty_cell,
                    "floor_index": target_floor,
                    "elevation_cm": target_elev_cm,
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
    updated.pop("passive_whispers", None)
    updated["event_type"] = "TOKEN_MOVED"
    updated["tokens"] = tokens
    updated["token_visuals"] = _build_wire_tokens_and_visuals(
        {
            "active_floor": active_floor,
            "tokens": tokens,
            "token_visuals": map_state.get("token_visuals") or [],
        }
    )[1]

    pins = list(_SESSION_SPATIAL_PINS.get(session_id) or [])
    try:
        from satya_ml.ai_gm.tools import get_session_characters

        observers: list[dict[str, Any]] = [
            dict(c) for c in get_session_characters(session_id) if isinstance(c, Mapping)
        ]
        if sensory_tns:
            matched_idx = next(
                (
                    i
                    for i, o in enumerate(observers)
                    if str(o.get("character_id") or f"pc-{i}") == moved_entity_id
                ),
                None,
            )
            if matched_idx is None:
                matched_obs: dict[str, Any] = {
                    "character_id": moved_entity_id,
                    "character_name": moved_entity_id,
                    "characteristics": {},
                }
                observers.append(matched_obs)
            else:
                matched_obs = dict(observers[matched_idx])
                matched_obs["characteristics"] = dict(matched_obs.get("characteristics") or {})
                observers[matched_idx] = matched_obs
            chars_map = matched_obs["characteristics"]
            for sk, sv in sensory_tns.items():
                if isinstance(sv, (int, float)):
                    chars_map[str(sk).lower()] = int(sv)

        updated_rust_json = _build_rust_map_payload(updated)
        try:
            whispers = _maps.evaluate_passive_perceptions(
                updated_rust_json, moved_entity_id, observers, pins or None
            )
        except Exception:
            whispers = _maps.evaluate_passive_perceptions(
                updated_rust_json, moved_entity_id, [], pins or None
            )
        if whispers:
            updated["passive_whispers"] = whispers
            triggered_ids = {str(w.get("pin_id")): str(w.get("observer_id")) for w in whispers}
            pin_by_id = {str(p.get("pin_id")): p for p in _SESSION_SPATIAL_PINS.get(session_id, [])}
            for w in whispers:
                pin = pin_by_id.get(str(w.get("pin_id")))
                proof_tag = str(pin.get("grants_audit_proof_tag") or "").strip() if pin else ""
                if proof_tag and pin is not None:
                    existing_pid = str(pin.get("minted_audit_proof_id") or "").strip()
                    if not existing_pid:
                        from satya_ml.ai_gm.audit_and_first_contact import record_audit_proof_token

                        token = record_audit_proof_token(
                            session_id=session_id,
                            subject_or_faction=proof_tag,
                            summary=str(w.get("whisper_text") or pin.get("whisper_text") or proof_tag),
                            source_kind="spatial_pin_discovery",
                            pin_id=str(pin.get("pin_id")),
                        )
                        existing_pid = token["proof_id"]
                        pin["minted_audit_proof_id"] = existing_pid
                    w["minted_audit_proof_id"] = existing_pid
                    w["grants_audit_proof_tag"] = proof_tag
            for pin in _SESSION_SPATIAL_PINS.get(session_id, []):
                pid = str(pin.get("pin_id"))
                if pid in triggered_ids and pin.get("one_shot", True):
                    obs_id = triggered_ids[pid]
                    rev = list(pin.get("revealed_to") or [])
                    if obs_id not in rev:
                        rev.append(obs_id)
                        pin["revealed_to"] = rev
    except Exception:
        pass

    set_session_map(session_id, updated)
    _sync_map_with_signaling(session_id, updated)
    return updated


@satya_gm_tool(pack="tactical_maps", tier="TIER_1_SATYA_CORE")
def inspect_tactical_map(session_id: str, question: str) -> str:
    """Inspect the active continuous tactical map geometry, floors, zones, exits, token positions, metric distances, cover `Δ_ext`, and optional VLM analysis."""
    map_state = get_session_map(session_id)
    if map_state is None:
        return f"No active tactical map for session '{session_id}'."

    width = int(map_state.get("grid_width", 16))
    height = int(map_state.get("grid_height", 12))
    tokens = _merge_tokens_with_visuals(map_state)
    preset = str(map_state.get("theme_preset") or infer_sector_preset(str(map_state.get("theme", ""))))
    wall_edges = list(map_state.get("wall_edges") or [])
    props = list(map_state.get("props") or [])

    prop_notes = [
        f"{p.get('kind')}@({p.get('center_x_cm', 0) / 100.0:.2f}m,{p.get('center_y_cm', 0) / 100.0:.2f}m {p.get('rotation_deg', 0)}deg)"
        for p in props[:12]
    ]

    active_floor = int(map_state.get("active_floor", 0))
    token_summaries = [
        f"- {t.get('label', t.get('token_id'))} (`{t.get('token_id')}`) [Floor {int(t.get('floor_index', active_floor))}] at ({t.get('grid_x', t.get('x'))}, {t.get('grid_y', t.get('y'))}) [{int(t.get('pos_x_cm', 0)) / 100.0:.2f}m, {int(t.get('pos_y_cm', 0)) / 100.0:.2f}m]"
        for t in tokens
    ]
    pairwise: list[str] = []
    for i in range(len(tokens)):
        for j in range(i + 1, len(tokens)):
            t1, t2 = tokens[i], tokens[j]
            t1_floor = int(t1.get("floor_index", active_floor))
            t2_floor = int(t2.get("floor_index", active_floor))
            x1_m = float(t1.get("pos_x_cm", int(t1.get("grid_x", t1.get("x", 0))) * 100 + 50)) / 100.0
            y1_m = float(t1.get("pos_y_cm", int(t1.get("grid_y", t1.get("y", 0))) * 100 + 50)) / 100.0
            x2_m = float(t2.get("pos_x_cm", int(t2.get("grid_x", t2.get("x", 0))) * 100 + 50)) / 100.0
            y2_m = float(t2.get("pos_y_cm", int(t2.get("grid_y", t2.get("y", 0))) * 100 + 50)) / 100.0
            l1 = t1.get("label", t1.get("token_id"))
            l2 = t2.get("label", t2.get("token_id"))
            if t1_floor != t2_floor:
                dist_m = round(math.hypot(x1_m - x2_m, y1_m - y2_m), 1)
                pairwise.append(
                    f"{l1} <-> {l2}: {dist_m}m [DIFFERENT FLOOR: Floor {t1_floor} vs Floor {t2_floor}]"
                )
            else:
                f_walls, f_props = _extract_floor_geometry(map_state, t1_floor, wall_edges, props)
                dist_m, los_tag, _delta = _evaluate_pair_los_and_cover(
                    f_walls, f_props, x1_m, y1_m, x2_m, y2_m
                )
                pairwise.append(f"{l1} <-> {l2}: {dist_m}m{los_tag}")

    manifest = dict(map_state.get("narrative_manifest") or {})
    if not manifest:
        manifest = _extract_narrative_manifest(map_state)
    floor_count = len(map_state.get("floors") or [])
    sector_count = len(map_state.get("world_sectors") or [])
    extra_lines: list[str] = []
    if floor_count > 1 or sector_count > 1:
        extra_lines.append(
            f"Floors: active={active_floor} (total={max(1, floor_count)}) | Linked World Sectors: {max(1, sector_count)}"
        )
    if manifest.get("concise_brief"):
        extra_lines.append(f"Narrative Manifest: {manifest['concise_brief']}")
    extra_block = ("\n" + "\n".join(extra_lines)) if extra_lines else ""

    geometric_summary = (
        f"Map '{map_state.get('map_id')}' ({width}m x {height}m continuous vector, preset={preset}, url={map_state.get('image_url', map_state.get('map_image_url'))}):\n"
        f"Walls: {len(wall_edges)} edges. Tactical props: {', '.join(prop_notes)}.\n"
        f"Tokens:\n" + "\n".join(token_summaries) + "\n"
        f"Pairwise Distances & Line of Sight: {', '.join(pairwise) if pairwise else 'N/A'}"
        f"{extra_block}\n"
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
    hostile_zone, player_zone = get_pack_zone_labels()
    if existing is None:
        initial_tokens: list[dict[str, Any]] = []
        for idx, c in enumerate(raw_combatants):
            role_str = str(c.get("role_or_faction") or "").lower()
            is_hostile = bool(c.get("is_npc")) or "npc" in role_str or "hostile" in role_str or "enemy" in role_str
            default_x = 12 - (idx % 3) * 2 if is_hostile else 3 + (idx % 3) * 2
            default_y = 5 + (idx // 3) * 2
            gx = max(1, min(14, int(c.get("grid_x") if c.get("grid_x") is not None else default_x)))
            gy = max(1, min(14, int(c.get("grid_y") if c.get("grid_y") is not None else default_y)))
            px_cm = int(c.get("pos_x_cm") if c.get("pos_x_cm") is not None else gx * 100 + 50)
            py_cm = int(c.get("pos_y_cm") if c.get("pos_y_cm") is not None else gy * 100 + 50)
            zone = str(c.get("zone_label") or (hostile_zone if is_hostile else player_zone))
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
                    "height": 16,
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
    active_floor = int(existing.get("active_floor", 0))
    changed = not bool(existing.get("procedural_generator_stub"))
    for idx, c in enumerate(raw_combatants):
        eid = str(c.get("entity_id") or "")
        dname = str(c.get("display_name") or eid or f"Combatant {idx + 1}")
        gx = c.get("grid_x")
        gy = c.get("grid_y")
        c_px_cm = c.get("pos_x_cm")
        c_py_cm = c.get("pos_y_cm")
        c_floor = c.get("floor_index")
        matched = None
        for tok in tokens:
            if (
                tok.get("token_id") == eid
                or tok.get("entity_id") == eid
                or str(tok.get("label", "")).lower() == dname.lower()
            ):
                matched = tok
                break
        if matched is not None:
            if c_px_cm is not None and int(c_px_cm) != matched.get("pos_x_cm"):
                px = max(55, min(width * 100 - 55, int(c_px_cm)))
                matched["pos_x_cm"] = px
                matched["grid_x"] = px // 100
                matched["x"] = px // 100
                changed = True
            elif gx is not None and int(gx) != matched.get("grid_x"):
                ng = max(1, min(width - 2, int(gx)))
                matched["grid_x"] = ng
                matched["x"] = ng
                matched["pos_x_cm"] = ng * 100 + 50
                changed = True
            if c_py_cm is not None and int(c_py_cm) != matched.get("pos_y_cm"):
                py = max(55, min(height * 100 - 55, int(c_py_cm)))
                matched["pos_y_cm"] = py
                matched["grid_y"] = py // 100
                matched["y"] = py // 100
                changed = True
            elif gy is not None and int(gy) != matched.get("grid_y"):
                ng = max(1, min(height - 2, int(gy)))
                matched["grid_y"] = ng
                matched["y"] = ng
                matched["pos_y_cm"] = ng * 100 + 50
                changed = True
            if c_floor is not None and int(c_floor) != matched.get("floor_index"):
                matched["floor_index"] = int(c_floor)
                changed = True
        elif eid or dname:
            role_str = str(c.get("role_or_faction") or "").lower()
            is_hostile = bool(c.get("is_npc")) or "npc" in role_str or "hostile" in role_str
            nx = max(1, min(width - 2, int(gx if gx is not None else (12 if is_hostile else 3))))
            ny = max(1, min(height - 2, int(gy if gy is not None else (4 + idx % 5))))
            px = max(55, min(width * 100 - 55, int(c_px_cm))) if c_px_cm is not None else nx * 100 + 50
            py = max(55, min(height * 100 - 55, int(c_py_cm))) if c_py_cm is not None else ny * 100 + 50
            tokens.append(
                normalize_token(
                    {
                        "token_id": eid or f"tok-{idx}",
                        "entity_id": eid or f"tok-{idx}",
                        "label": dname,
                        "pos_x_cm": px,
                        "pos_y_cm": py,
                        "grid_x": px // 100,
                        "grid_y": py // 100,
                        "floor_index": int(c_floor) if c_floor is not None else active_floor,
                        "visibility": c.get("visibility", "PUBLIC"),
                        "is_hidden": bool(c.get("is_hidden", False)),
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


from satya_ml.ai_gm.maps_tactical import (  # noqa: E402
    compute_token_reachable_zone,
    edit_tactical_map_feature,
    evaluate_ballistic_shot_on_map,
    inspect_tactical_map_point,
    sync_map_blackout_from_polycrisis,
)

__all_tactical_tools__ = [
    "compute_token_reachable_zone",
    "edit_tactical_map_feature",
    "evaluate_ballistic_shot_on_map",
    "inspect_tactical_map_point",
    "sync_map_blackout_from_polycrisis",
]
