# Illustrated 2D SVG Battlemap (`map_2d_art*` & `map_2d_tokens`)

`crates/satya-web/src/wasm_client/map_2d_art*.rs` renders the top-down architectural 2D battlemap inside `#tabletop-map-svg-fallback` (`Mode: 2D Battlemap` via `#btn-map-view-mode`, persisted in `localStorage["satya.map.view_mode"]`, and automatic no-WebGPU fallback) from the exact same `GmMapUpdateRequest` payload that feeds the 3D WebGPU diorama and `satya-engine` pathing / LoS / barrier ballistics.

---

## 1. Module Split & Ownership (`crates/satya-web/src/wasm_client/`)

| Module | Responsibility |
| :--- | :--- |
| **`map_2d_art.rs`** | `BATTLEMAP_CELL_PX` (`64`), `BATTLEMAP_PAD_PX` (`12`), `BATTLEMAP_ART_THEMES` (`8` material themes + `PRESET_ART_THEME_MAP` covering all `12` presets), FNV-1a `battlemap_art_cache_key`, boundary/rect path mergers, and `format_battlemap_art_layer` (renders continuous `wall_edges`, surfaces, and `360°` props with `tiles == []`). |
| **`map_2d_art_surfaces.rs`** | Floor `<pattern>` defs (`Grass` multi-tone turf, `Hardwood`, etc.), landscaping `<pattern>`s (`#bm-gravel`, `#bm-dirt`, `#bm-mulch`, `#bm-creek`, `#bm-ditch`, `#bm-leaf-tex`), and Chaikin corner-smoothed contour extraction (`battlemap_smooth_region_path`) for meandering creeks, ditches, gravel bars, dirt trails, mulch beds, and flagstones. |
| **`map_2d_art_walls.rs`** | Coalesced `MapWallEdge` orthogonal segments (`Wall`, `DoorClosed`, `DoorOpen` swing arcs, `Window` sills/glazing) rendered cleanly along wall coordinates. |
| **`map_2d_art_furniture.rs`** | All `52` `PropKind` vector glyphs rendered at continuous `(center_x_cm, center_y_cm)` and full `360°` (`0..359°`) rotation (`Sofa`, `BarCounter`, `PoolTable`, `Car`, `UtilityVan`, `GardenBed`, `GardenEdging`, `StructuralPillar`, `ElevatedCatwalk`, `HazardSpill`, leaf-textured `LargeTree` / `MediumTree` canopies, etc.). |
| **`map_2d_art_props.rs`** | Deterministic cell hashing / blob path helpers for terrain scatter and legacy prop fallback contours. |
| **`map_2d_tokens.rs`** | Phase 4 `BattlemapInkStyle` filter finishes (`vellum`, `clean`, `blueprint`, `sketch`), `satya_set_battlemap_ink_style`, and `push_token_portrait_disc_svg` placed at continuous `(pos_x_cm, pos_y_cm)` coordinates. |
| **`map_2d_art_overlay.rs`** | Dynamic `#battlemap-overlay` (optional `1.0m`/`5.0m` visual metric grid, continuous `reachable_strips` contour runs, hover reticle + OBB/edge highlight, `360°` prop brush preview, analytical LoS ray, portrait discs) and `sync_battlemap_svg_host`. |
| **`map_2d_camera.rs`** | `BattlemapCamera2d` cursor-centered SVG `viewBox` zoom (`0.25x..6.0x`) & drag-pan, `preserveAspectRatio="xMidYMid meet"` client-to-metric unprojection, deliberate `>= 120px` boundary overscroll & `Continue [Dir]` overlay triggers, and GM/solo authoring permission checks. |
| **`map_hover_inspect.rs`** | Continuous Object & Ground Hover Inspector (`satyaHoverMapPoint`, `satyaLeaveMapHover`, `satyaToggleMapInspectMode`), fixed-width `#map-hover-coords-pill`, and docked top-right `#map-hover-tooltip` overlay pane (`Alt`-hold / `I`-toggle / `#btn-map-inspect-toggle`) with zero map relayout. |
| **`map_edge_edit.rs`** | Continuous metric pointer picking (`satyaClickMapPoint(fx, fy)`), `PROP_BRUSH_GROUPS` (`52` variants + Wall Edge + Eraser), GM `360°` prop placement/rotation/snap (`satyaRotateMapPropBrush` `+15°`, `satyaSetMapPropRotationDeg`, `satyaToggleMapGridSnap`), and server REST sync. |

---

## 2. Two-Layer SVG Stack, Camera Zoom/Pan & Boundary Continuation

`#tabletop-map-svg-fallback` fills `#tabletop-map-viewport` (`100% × 100%`) and contains `.battlemap-stack`, stacking two `<svg>` elements over an identical synchronized `viewBox` (`preserveAspectRatio="xMidYMid meet"`):

1. **`#battlemap-art` (Static Art Layer)**:
   - Keyed by `data-art-key="{battlemap_art_cache_key(ui)}"`, an FNV-1a digest over `grid_width`, `grid_height`, `seed`, `theme.id`, `ink_style.as_str()`, `wall_edges`, and `props` (continuous geometry, independent of any legacy tile array).
   - `sync_battlemap_svg_host()` skips regenerating `#battlemap-art` whenever `data-art-key` matches, replacing only `#battlemap-overlay.innerHTML` on pointer hover, token selection, token movement, or grid-toggle events.
   - Wheel zoom and drag-pan mutate only the `viewBox` attribute on `#battlemap-art` and `#battlemap-overlay` via `sync_map_camera_and_edge_controls_dom()` with zero DOM rebuilds (`#btn-map-zoom-out`, `#map-zoom-readout`, `#btn-map-zoom-in`, and `#btn-map-reset-cam` / double-click).
2. **`#battlemap-overlay` (Dynamic Interaction Layer)**:
   - Routes `onwheel`, `onmousedown`, `onmousemove`, `onmouseup`, and `onmouseleave` into `map_2d_camera.rs` (`satyaBattlemapWheel`, `satyaBattlemapMouse*`), unprojecting client pixels into continuous metric coordinates `(fx, fy)` across any zoom/pan or container aspect ratio.
   - When an authorized user (Human GM/Host, solo player in an AI GM session, or offline user) pans to a cardinal boundary or overscrolls `>= 120px` past it, directional `#btn-map-continue-{north,south,west,east}` controls trigger edge-matched procedural stitching (`satya_engine::tactical_map::continue_procedural_map`, up to `80m × 56m`).

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

## 5. GM 360° Prop Placement, Rotation, Grid Snap & Eraser

When `#map-click-mode` is `"edit_terrain"`, the subbar reveals `#map-prop-brush-select` (`PROP_BRUSH_GROUPS` covering all `52` `PropKind` variants including `StructuralPillar`, `ElevatedCatwalk`, `HazardSpill`, `Landscaping & Terrain`, `"Wall / Door / Window Edge"`, and `"__erase__"`) plus `#btn-map-prop-rotate` (`+15°` steps across full `0..359°`), `#map-prop-rot-deg` numeric degree input (`satyaSetMapPropRotationDeg`), and `#btn-map-snap-toggle` (`satyaToggleMapGridSnap`):
- **Continuous Ghost Preview**: `#battlemap-overlay` renders the rotated OBB or circle silhouette centered at `hovered_point_m` (snapped to `0.50m` only when snap is enabled, otherwise free continuous meters; `#0ea5e9` when valid, `#e11d48` when out of bounds, colliding, or erasing a prop).
- **Vector Scene Mutation (`satya_engine::tactical_map_geom`)**: Both the optimistic Wasm handler (`handle_map_point_clicked`) and `POST /api/sessions/:id/map` (`satya-signaling::map_handlers`, GM-only guarded) invoke `place_prop_on_map` / `remove_prop_at_point` with `tiles: []`. Updating or removing a prop immediately refreshes `#battlemap-art`, `#battlemap-overlay`, `ReachableField`, LoS, and 3D WebGPU geometry.

---

## 6. Object & Ground Hover Inspector (`map_hover_inspect.rs`)

Moving the pointer anywhere over `#battlemap-overlay` or `#wgpu-map-canvas` dispatches `satyaHoverMapPoint(fx, fy, clientX, clientY)` into `compute_hover_inspection` (`satya_engine::tactical_map_geom::inspect_map_point`), classifying the topmost entity in priority order:
1. **Tokens** (`0.42m` pick radius): label, stance pose, visibility, facing degrees, and metric coordinates.
2. **Wall / Door / Window Edges**: `MaterialClass` (`Brick`, `Drywall`, `ConcreteBlock`, `Glass`, etc.) + cover/breach rating.
3. **3D / Solid Props (incl. Sub-Zones)**: distinguishes `Car` / `UtilityVan` front `Engine Block` (`ArmorSteel` hard cover) from cabin, and `LargeTree` / `MediumTree` solid `Trunk` (`LiveWood`) from overhead `Canopy` (`Foliage`, `2.10m` elevation).
4. **Surface / Catwalk / Hazard Props**: `ElevatedCatwalk`, `HazardSpill`, `StonePath`, `GravelPath`, `DirtPath`, `MulchBed`, `CreekWater`, `Rug`, etc. with move-cost multipliers.
5. **Base Ground / Floor**: theme-aware roadway, grass, concrete slab, or interior decking.

Coordinates render inside the fixed-width toolbar pill `#map-hover-coords-pill` (`136px` monospace tabular-nums) and `#tabletop-map-los-hud` stays locked to a single non-wrapping row, while full material, cover, and active-shot ballistic details appear in the **docked top-right `#map-hover-tooltip` overlay pane** inside `#tabletop-map-viewport` whenever Inspect mode is active (`Alt`-hold, `I` hotkey, or `#btn-map-inspect-toggle`) — eliminating all hover-induced vertical/horizontal layout shifts.
