# Implementation Plan: 2D SVG Battlemap Trackpad Pinch/Wheel Zoom, Pan & Reset View

> [!NOTE]
> **Status**: Planned (deferred for later implementation). Covers cursor-anchored laptop trackpad pinch/spread zoom, mouse-wheel zoom, two-finger / drag pan, and `Reset View` parity for `Mode: 2D Battlemap` (`#tabletop-map-svg-fallback`).

---

## 1. Problem Statement & Root Cause

On a laptop trackpad, pinching or spreading two fingers over **`Mode: 2D Battlemap`** zooms the **entire browser page** instead of the tactical map, whereas **`Mode: 3D Diorama`** zooms only the map camera:

1. **How browsers emit trackpad pinch**: On Windows Precision Touchpads, macOS trackpads, and Linux Wayland/libinput, a two-finger pinch/spread fires synthetic `WheelEvent`s with `ctrlKey == true` and fractional `deltaY` (and on Safari macOS, `gesturestart` / `gesturechange` events). Unless an active `{ passive: false }` listener calls `preventDefault()`, the browser treats `ctrlKey + wheel` as **full-page zoom**.
2. **Why `Mode: 3D Diorama` works**: [`attach_map_listeners_if_needed`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_3d_wgpu.rs#L580-L589) attaches a `wheel` listener on `#wgpu-map-canvas` (`touch-action: none` in [`styles_stages.css`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/styles_stages.css#L430-L436)) that calls `ev.prevent_default()` and `c.camera.zoom_by_factor(...)`.
3. **Why `Mode: 2D Battlemap` zooms the page**: [`format_tactical_map_svg_markup`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_2d_art_overlay.rs#L182-L197) only binds inline `onclick`, `onmousemove`, and `onmouseleave` on `#battlemap-overlay` with `touch-action: manipulation` ([`styles_stages.css`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/styles_stages.css#L460-L466)), and [`apply_battlemap_view_chrome`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_2d_art_overlay.rs#L215-L222) disables `#btn-map-reset-cam` (`Reset View`).

---

## 2. Target Architecture & Coordinate Identity

### 2.1 CSS `transform: translate(...) scale(...)` on `.battlemap-stack` (Zero SVG Re-Rasterization Cost)

Rather than mutating `<svg viewBox="...">` on every `60–120 Hz` trackpad frame (which forces the browser to re-run heavy `<feTurbulence>` + `<feDisplacementMap>` SVG filters `#bm-ink`, `#bm-prop`, `#bm-grain`, and `#bm-mottle` in [`map_2d_art.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_2d_art.rs#L447-L454)), transform the shared wrapper `<div id="battlemap-stack" class="battlemap-stack">` inside the clipping viewport `#tabletop-map-svg-fallback` (`overflow: hidden`):

```css
transform-origin: 0 0;
transform: translate({pan_x_px:.2}px, {pan_y_px:.2}px) scale({zoom:.4});
```

- **Exact `pick_js` coordinate invariance**: `#battlemap-overlay` already computes cell/edge coordinates from `var r = this.getBoundingClientRect()` ([`map_2d_art_overlay.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_2d_art_overlay.rs#L189-L191)):
  ```js
  fx = ((event.clientX - r.left) * tw / r.width - pad) / c
  fy = ((event.clientY - r.top)  * th / r.height - pad) / c
  ```
  Because `getBoundingClientRect()` returns the **post-transform** `r.left`, `r.top`, `r.width = w0 * zoom`, and `r.height = h0 * zoom`, `(event.clientX - r.left) * tw / r.width` remains **mathematically exact at every `zoom` and `(pan_x_px, pan_y_px)`** with zero change to cell or `0.20m` sub-cell edge/door picking (`satyaClickMapPoint`).
- **Crisp vector re-rasterization at high zoom**: Remove permanent `will-change: transform` from `.tabletop-map-svg-fallback svg.battlemap-art` ([`styles_stages.css`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/styles_stages.css#L457-L459)) so Chromium/WebKit re-rasterize vector ink strokes, grid hairlines, and token labels sharply at the target zoom scale instead of upscaling a `1x` bitmap texture.

### 2.2 Pure-Rust `Battlemap2dViewport` State & Cursor-Anchored Math

Add a pure-Rust, host-testable `Battlemap2dViewport` struct (stored on [`TacticalMapUiState`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_3d.rs#L40-L58)):

| Field | Type | Default / Bounds | Description |
| :--- | :--- | :--- | :--- |
| `zoom` | `f32` | `1.0` (`[1.0, 6.0]`) | Uniform scale factor (`1.0` fits `100%` of `#tabletop-map-svg-fallback` width) |
| `pan_x_px` | `f32` | `0.0` | Horizontal CSS-pixel offset of `#battlemap-stack` relative to host left |
| `pan_y_px` | `f32` | `0.0` | Vertical CSS-pixel offset of `#battlemap-stack` relative to host top |

- **Cursor-anchored zoom (`zoom_at_cursor`)**:
  Given cursor offset `(cx_px, cy_px)` relative to `#tabletop-map-svg-fallback`'s top-left (`client_x - host_rect.left()`, `client_y - host_rect.top()`), unscaled stack size `(base_w_px, base_h_px)`, and `new_zoom = (old_zoom * factor).clamp(1.0, 6.0)`:
  $$x_{\text{local}} = \frac{\text{cx\_px} - \text{pan\_x\_0}}{\text{old\_zoom}}, \qquad y_{\text{local}} = \frac{\text{cy\_px} - \text{pan\_y\_0}}{\text{old\_zoom}}$$
  $$\text{pan\_x\_1} = \text{cx\_px} - x_{\text{local}} \cdot \text{new\_zoom}, \qquad \text{pan\_y\_1} = \text{cy\_px} - y_{\text{local}} \cdot \text{new\_zoom}$$
- **Bounds clamping (`clamp_pan`)**:
  Clamp `pan_x_px` to `[base_w_px * (1.0 - zoom), 0.0]` and `pan_y_px` to `[base_h_px * (1.0 - zoom), 0.0]` so at `zoom == 1.0` the map snaps cleanly to `(0, 0)` and at `zoom > 1.0` every edge/corner is reachable without ever panning the map off-screen.

---

## 3. Input & Gesture Matrix

| Input / Gesture | Detection | Action |
| :--- | :--- | :--- |
| **Trackpad 2-finger pinch / spread** (or `Ctrl + wheel`) | `wheel` on `#tabletop-map-svg-fallback` with `ev.ctrl_key() == true` | `ev.prevent_default()` (via `{ passive: false }`); smooth exponential zoom anchored at cursor: `factor = (-ev.delta_y() as f32 * 0.012).exp()` |
| **Safari macOS trackpad pinch** | `gesturestart` / `gesturechange` on `#tabletop-map-svg-fallback` | `ev.prevent_default()` so WebKit never triggers Safari page zoom |
| **Trackpad 2-finger pan** | `wheel` with `!ev.ctrl_key()` and `ev.delta_x().abs() > 0.5` when `zoom > 1.001` | `ev.prevent_default()`; pan by `(-delta_x, -delta_y)` clamped to map bounds |
| **Mouse scroll wheel** | `wheel` with `!ev.ctrl_key()` and `ev.delta_x().abs() <= 0.5` | `ev.prevent_default()`; cursor-anchored zoom with clamped delta (`factor = (-dy_px.clamp(-60.0, 60.0) * 0.0022).exp()`, ~`12%` per wheel notch, matching `#wgpu-map-canvas`) |
| **Click-drag / Shift-drag / Middle- or Right-drag pan** | `mousedown` -> window `mousemove` -> `mouseup` | `4px` deadzone matching [`map_3d_wgpu.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_3d_wgpu.rs#L631-L664): stationary click (`<= 4px`, `button == 0`, `!shift_key`) dispatches `satyaClickMapPoint(fx, fy)`; dragging `> 4px` (or middle/right/`Shift` drag) pans `#battlemap-stack` and suppresses the synthetic click |
| **Double-click or `Reset View` (`#btn-map-reset-cam`)** | `dblclick` on `#tabletop-map-svg-fallback` or clicking `Reset View` | Resets `viewport_2d` to `zoom = 1.0, pan_x_px = 0.0, pan_y_px = 0.0` |

---

## 4. Ordered File-by-File Changes

1. **[`Cargo.toml`](file:///c:/Users/hapah/Projects/satya/Cargo.toml) (`[workspace.dependencies] web-sys`)**:
   - Add `"AddEventListenerOptions"` to the `web-sys` `features` list (pure local `cfg` gate on the already-cached `web-sys` crate — required so `add_event_listener_with_callback_and_add_event_listener_options` can register `{ passive: false }` `wheel` listeners; passive `wheel` listeners ignore `preventDefault()` in Chromium/Firefox/Safari).

2. **[`crates/satya-web/src/styles_stages.css`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/styles_stages.css) (`lines 437–466`)**:
   - Change `.tabletop-map-svg-fallback` from `overflow: auto;` to `overflow: hidden; position: relative; overscroll-behavior: contain; touch-action: none;`.
   - Set `.battlemap-stack { transform-origin: 0 0; }` and remove permanent `will-change: transform;` from `.tabletop-map-svg-fallback svg.battlemap-art` so zoomed SVG vectors re-rasterize crisply.
   - Change `.tabletop-map-svg-fallback svg.battlemap-overlay` from `touch-action: manipulation;` to `touch-action: none;` (with `cursor: grab` / `.panning { cursor: grabbing }` when zoomed).

3. **`crates/satya-web/src/wasm_client/map_2d_zoom.rs` (new focused companion module, re-exported from [`wasm_client/mod.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/mod.rs) and [`map_2d_art.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_2d_art.rs))**:
   - Define `Battlemap2dViewport` (`zoom`, `pan_x_px`, `pan_y_px`, `reset()`, `zoom_at_cursor(...)`, `pan_by(...)`, `clamp_pan(...)`, `css_transform_style()`).
   - Implement `apply_battlemap_stack_transform()` (updates `style` on `#battlemap-stack` in place without rebuilding `#battlemap-art` or `#battlemap-overlay`).
   - Implement one-time idempotent `attach_battlemap_2d_listeners_if_needed()` (`#[cfg(target_arch = "wasm32")]`) attaching `{ passive: false }` `wheel`, Safari `gesturestart`/`gesturechange`, `mousedown`, `contextmenu`, `dblclick`, and window `mousemove`/`mouseup` drag-vs-click (`4px` threshold) listeners on `#tabletop-map-svg-fallback`.

4. **[`crates/satya-web/src/wasm_client/map_2d_art_overlay.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_2d_art_overlay.rs)**:
   - Give `.battlemap-stack` `id="battlemap-stack"` and emit `style="{ui.viewport_2d.css_transform_style()}"` in `format_tactical_map_svg_markup` so innerHTML rebuilds preserve the current zoom/pan.
   - Guard inline `onclick` in `format_tactical_map_svg_markup` (or route left-click release through the `4px` drag-threshold handler when Wasm listeners are attached, keeping a `dataset.dragged !== '1'` guard so SSR/offline tests retain the inline `satyaClickMapPoint(fx,fy)` contract).
   - Call `attach_battlemap_2d_listeners_if_needed()` inside `sync_battlemap_svg_host()` and `apply_battlemap_view_chrome(true)`.
   - In `apply_battlemap_view_chrome`, keep `#btn-map-reset-cam` **enabled** when `battlemap_on == true` (only disable `"btn-map-toggle-proj"` and `"btn-map-wall-cutaway"`).

5. **[`crates/satya-web/src/wasm_client/map_3d.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_3d.rs) & [`map_3d_wgpu.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_3d_wgpu.rs)**:
   - Add `pub viewport_2d: Battlemap2dViewport` to `TacticalMapUiState` (`Default` -> `Battlemap2dViewport::default()`), and reset `viewport_2d` in `satya_generate_procedural_map` when generating a new map.
   - Update `satya_reset_map_camera()` in [`map_3d_wgpu.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-web/src/wasm_client/map_3d_wgpu.rs#L809-L820) so it resets `ui.viewport_2d` and calls `apply_battlemap_stack_transform()` (in addition to resetting the 3D camera on `wasm32`).

6. **[`docs/technical/satya-web/02-3d-tactical-map-diorama.md`](file:///c:/Users/hapah/Projects/satya/docs/technical/satya-web/02-3d-tactical-map-diorama.md)**:
   - Document `Battlemap2dViewport` and the 2D SVG trackpad pinch/wheel/pan controls alongside the 3D `OrbitOrthoCamera` section.

---

## 5. Verification Plan

1. **Host Unit Tests (`crates/satya-web/src/wasm_client/map_2d_zoom.rs`)**:
   - `cursor_anchored_zoom_preserves_point_under_cursor`: verify for arbitrary `(cx_px, cy_px)` inside `(800, 560)` that `(cx_px - pan_x_0) / zoom_0 == (cx_px - pan_x_1) / zoom_1` before bounds clamping, and that zooming out back to `1.0` clamps `(pan_x_px, pan_y_px)` to `(0.0, 0.0)`.
   - `pan_clamping_keeps_battlemap_within_viewport`: verify extreme positive/negative pan deltas never exceed `[base_w * (1 - zoom), 0]` × `[base_h * (1 - zoom), 0]`.
   - `svg_markup_preserves_stack_id_transform_and_callback_contracts`: verify `format_tactical_map_svg_markup` emits `id="battlemap-stack"` with the active CSS transform and passes `test_ssr_and_dynamic_window_callbacks_match_registered_bindings`.
2. **Workspace Gate**:
   ```powershell
   cargo test --offline -p satya-web
   cargo clippy --offline -p satya-web --all-targets -- -D warnings
   ```
3. **Manual Laptop Trackpad Check**:
   - Switch to `Mode: 2D Battlemap`, pinch-spread and pinch-close on a laptop trackpad: map zooms smoothly `1.0x..6.0x` anchored at the pointer while browser page zoom stays at `100%`.
   - Two-finger scroll or click-drag while zoomed in pans across rooms; clicking a token, door edge, or target cell at `3x` zoom hits the exact cell/edge under the crosshair; `Reset View` snaps back to `1.0x`.
