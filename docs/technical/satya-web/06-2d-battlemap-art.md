# Illustrated 2D SVG Battlemap (`map_2d_art*` & `map_2d_tokens`)

`crates/satya-web/src/wasm_client/map_2d_art*.rs` renders the top-down architectural 2D battlemap inside `#tabletop-map-svg-fallback` (`Mode: 2D Battlemap` via `#btn-map-view-mode`, persisted in `localStorage["satya.map.view_mode"]`, and automatic no-WebGPU fallback) from the exact same `GmMapUpdateRequest` payload that feeds the 3D WebGPU diorama and `satya-engine` pathing / LoS / barrier ballistics.

---

## 1. Module Split & Ownership (`crates/satya-web/src/wasm_client/`)

| Module | Responsibility |
| :--- | :--- |
| **`map_2d_art.rs`** | `BATTLEMAP_CELL_PX` (`64`), `BATTLEMAP_PAD_PX` (`12`), `BATTLEMAP_ART_THEMES` (`8` material themes + `PRESET_ART_THEME_MAP`), FNV-1a `battlemap_art_cache_key`, boundary/rect path mergers, and `format_battlemap_art_layer`. |
| **`map_2d_art_walls.rs`** | Coalesced `MapWallEdge` orthogonal segments (`Wall`, `DoorClosed`, `DoorOpen` swing arcs, `Window` sills/glazing) rendered cleanly on grid boundaries. |
| **`map_2d_art_furniture.rs`** | All `39` multi-cell `PropKind` vector glyphs (`Sofa`, `BarCounter`, `PoolTable`, `Car`, `UtilityVan`, `LargeTree` / `MediumTree` overhanging canopies, etc.). |
| **`map_2d_art_props.rs`** | Legacy single-cell fallback cover props (`BattlemapPropKind`) and deterministic cell hashing / blob path helpers. |
| **`map_2d_tokens.rs`** | Phase 4 `BattlemapInkStyle` filter finishes (`vellum`, `clean`, `blueprint`, `sketch`), `satya_set_battlemap_ink_style`, and `push_token_portrait_disc_svg` (portrait medallions + preset busts). |
| **`map_2d_art_overlay.rs`** | Dynamic `#battlemap-overlay` (`1.0m`/`5.0m` grid, reach envelope, hover reticle / prop footprint preview, LoS ray, portrait discs) and `sync_battlemap_svg_host`. |
| **`map_edge_edit.rs`** | Fractional cell-space picking (`satyaClickMapPoint`), `PROP_BRUSH_GROUPS`, GM prop placement/rotation/erasure (`satyaUpdateMapPropBrush`, `satyaRotateMapPropBrush`), and server REST sync. |

---

## 2. Two-Layer SVG Stack & Cache Invalidation

`#tabletop-map-svg-fallback` contains `.battlemap-stack`, stacking two `<svg>` elements over an identical `-12 -12 (w*64 + 24) (h*64 + 24)` `viewBox`:

1. **`#battlemap-art` (Static Art Layer)**:
   - Keyed by `data-art-key="{battlemap_art_cache_key(ui)}"`, an FNV-1a digest over `grid_width`, `grid_height`, `seed`, `theme.id`, `ink_style.as_str()`, `tiles`, `wall_edges`, and `props`.
   - `sync_battlemap_svg_host()` skips regenerating `#battlemap-art` whenever `data-art-key` matches, replacing only `#battlemap-overlay.innerHTML` on pointer hover, token selection, token movement, or grid-toggle events.
2. **`#battlemap-overlay` (Dynamic Interaction Layer)**:
   - Binds single pointer handlers on `<svg id="battlemap-overlay">` converting client pixels to fractional cell coordinates `(fx, fy)` (`window.satyaClickMapPoint(fx, fy)`, `window.satyaHoverMapCell(x, y)`) with zero per-cell DOM listeners.

---

## 3. Paper / Ink Filter Styles (`BattlemapInkStyle`)

Selectable via `#map-ink-style-select` (visible in 2D Battlemap mode, persisted in `localStorage["satya.map.ink_style"]`), `BattlemapInkStyle` reconfigures SVG `<defs>` filters and finishing layers without touching `satya-engine` mechanics:

| Style ID (`as_str`) | Label | `#bm-ink` Wobble Scale | Color Grading & Paper Finish |
| :--- | :--- | :---: | :--- |
| **`vellum`** *(default)* | Modern Vellum | `3.5` | Warm `#2b2723` board margin + `0.70` multiply `#bm-grain` paper tooth. |
| **`clean`** | Crisp Vector | `0.0` (`<feOffset/>`) | Zero displacement wobble, crisp vector shadows, no paper grain overlay. |
| **`blueprint`** | Tactical Blueprint | `0.8` | Prussian-blue `feColorMatrix` (`#bm-tint`), `#dbeafe` grid lines, and double drafting border. |
| **`sketch`** | Aged Field Sketch | `4.8` | Heavy hand-drawn displacement, warm sepia `feColorMatrix`, `#f0dfb8` multiply wash, and `0.85` tooth grain. |

---

## 4. Token Portrait Discs (`push_token_portrait_disc_svg`)

Each token in `ui.map.tokens` renders as a layered medallion inside `#battlemap-overlay`:
- **Compass & Selection Rings**: Directional facing wedge rotated by `facing_deg`, translucent `.55` opacity for `MessageVisibility::GmOnly` ambush tokens, and dashed amber ring (`r="29"`) when selected.
- **Clipped Portrait Well**: `<clipPath id="bm-tok-clip-{idx}">` (`r="17"`) frames either:
  - an allowlisted `<image href="..." preserveAspectRatio="xMidYMid slice"/>` from `TokenVisualSpec.portrait_url` (validated by `satya_schema::sanitize_portrait_url`, permitting `/api/characters/*`, `/maps/*`, `/pkg/*`, `/assets/*`, `https://`, and raster `png`/`jpeg`/`webp` base64 URIs while blocking `javascript:` and `data:image/svg+xml`), or
  - a distinct top-down/bust SVG operator silhouette matched to `model_preset` (`valkyrie_rig`, `recon_mk4`, `secops_sentry`, `operative_light`, `custom_glb`).
- **Callsign Badge**: High-contrast corner initials pill (`2`-letter abbreviation from `tok.label`) ensures immediate legibility even when portraits are similar colors.

---

## 5. GM Multi-Cell Prop Placement, Rotation & Eraser

When `#map-click-mode` is `"edit_terrain"`, the subbar reveals `#map-prop-brush-select` (`PROP_BRUSH_GROUPS` covering all `39` `PropKind` variants, `"Tile / Wall Cycle"`, and `"__erase__"`) plus `#btn-map-prop-rotate` (`0° → 90° → 180° → 270°`):
- **Hover Footprint Preview**: `#battlemap-overlay` draws a dashed rectangle across `(p.x..p.x + p.w_cells, p.y..p.y + p.h_cells)` (`#0ea5e9` when in bounds, `#e11d48` when out of bounds or erasing a prop).
- **Single-Source Tile Stamping (`satya_engine::map_dressing`)**: Both the optimistic Wasm handler (`handle_map_prop_brush_click`) and `POST /api/sessions/:id/map` (`satya-signaling::map_handlers` Branch A0b, GM-only guarded) invoke `place_prop_on_grid` / `remove_prop_at_on_grid`. Removing or replacing a prop cleanly resets old `FullWall` / `HalfCover` / `Concealment` stamps to `Floor` before re-stamping remaining props, and refreshes both 2D SVG and 3D WebGPU geometry.
