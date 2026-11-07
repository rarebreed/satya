---
name: satya-rust-web
description: >-
  Full-stack Leptos SSR and WebAssembly (wasm-bindgen, web-sys, wgpu) architecture,
  module map, bind_window_callbacks! registration, and DOM/catalog deduplication
  guide for crates/satya-web.
---

# Rust Web & WebAssembly Technical Steering Guide (`satya-web`)

This document defines the module architecture, SSR/Wasm parity invariants, and anti-duplication rules for [`crates/satya-web`](../../crates/satya-web). It extends the workspace-wide rules in [`docs/technical/rust-workspace-steering.md`](rust-workspace-steering.md) and pairs with the engine rules in [`docs/technical/rust-engine-steering.md`](rust-engine-steering.md).

---

## 1. Module Map & Responsibilities (`<= 45 KB` Per File)

Every `.rs` and `.css` file in `crates/satya-web/src/` must remain under `45 KB` (`< 46,080` bytes):

### 1.1 Top-Level SSR Shell & Chat Parser (`crates/satya-web/src/`)
- [`lib.rs`](../../crates/satya-web/src/lib.rs): Root `SatyaApp` component, `render_satya_app_html()` SSR entrypoint, `WASM_BOOTSTRAP_LOADER`, stylesheet concatenation (`include_str!("styles.css")` + `include_str!("styles_notebook.css")`), and flat `pub use` re-exports.
- [`views_gate.rs`](../../crates/satya-web/src/views_gate.rs): Top navigation bar (`HeaderBar`), Hamburger Menu settings, shared `PasswordToggleButton`, and pre-session authentication/session gate (`AuthAndSessionGate`).
- [`views_notebook.rs`](../../crates/satya-web/src/views_notebook.rs) & [`views_notebook_docs.rs`](../../crates/satya-web/src/views_notebook_docs.rs): Left-column Tabbed Tabletop Notebook (`NotebookColumn` and `NotebookReferencePanes`) rendering all 9 panes (`maps`, `dice`, `model`, `character`, `wizard`, `combat`, `rules`, `docs`, `iggy`).
- [`views_workspace.rs`](../../crates/satya-web/src/views_workspace.rs): Active 3-column session workspace (`ActiveSessionWorkspace`) combining `NotebookColumn`, Center column (`#chat-card` + `#ttrpg-drawer`), and Right column (`#webrtc-card` + `#right-drawer-rails`).
- [`chat_cmd.rs`](../../crates/satya-web/src/chat_cmd.rs): Slash-command parser (`/msg`, `/w`, `/whisper`, `/roll`, `/r`) and dice pipeline evaluator (`4d10|>hi(3)`, `keep_lowest`, `>=TN`).
- [`styles.css`](../../crates/satya-web/src/styles.css) & [`styles_notebook.css`](../../crates/satya-web/src/styles_notebook.css): Core layout/WebRTC/chat/3D-dice styles (`styles.css`) and GM Docs/Rules Book/Character Wizard styles (`styles_notebook.css`).
- [`tests.rs`](../../crates/satya-web/src/tests.rs): SSR HTML structure and `wasm_client` formatter unit tests.

### 1.2 Browser WebAssembly Runtime (`crates/satya-web/src/wasm_client/`)
- [`mod.rs`](../../crates/satya-web/src/wasm_client/mod.rs): `#[wasm_bindgen(start)]` entrypoint, submodule re-exports, and the declarative `bind_window_callbacks!` registration table.
- [`state.rs`](../../crates/satya-web/src/wasm_client/state.rs): Thread-local `SatyaWasmState` (`with_state`, `with_state_mut`) and WebRTC `PeerEntry` registry.
- [`dom.rs`](../../crates/satya-web/src/wasm_client/dom.rs): Typed `web-sys` DOM query/mutation helpers (`input_value`, `set_text`, `set_html`, `set_display`, `toggle_element_display`, `fetch_json`, `roll_browser_die_i32`, `roll_browser_die_i64`) and inline SVG constants.
- [`options.rs`](../../crates/satya-web/src/wasm_client/options.rs): Pure Rust enum parsers (`parse_session_mode`, `parse_session_role`, `parse_visibility`, `parse_combat_stance`) and mode-specific `<select>` option tables.
- [`menu.rs`](../../crates/satya-web/src/wasm_client/menu.rs): Hamburger menu dropdown, pane visibility toggles, password visibility toggles, and BYOK AI GM configuration.
- [`auth_session.rs`](../../crates/satya-web/src/wasm_client/auth_session.rs): Registration, login, logout, demo presets, session create/join/leave, and participant roster rendering.
- [`webrtc.rs`](../../crates/satya-web/src/wasm_client/webrtc.rs): WebSocket signaling, P2P WebRTC Audio/Video + DataChannel mesh, prosody telemetry, and collapsible/floating drawer controls (`sync_drawer_rail_and_aria`, `sync_center_drawer_card`).
- [`chat_and_ttrpg.rs`](../../crates/satya-web/src/wasm_client/chat_and_ttrpg.rs): Targeted Session Chat, recipient selector, message filtering, and Satya Two-List Cascading (`2d20`) task resolution UI.
- [`notebook.rs`](../../crates/satya-web/src/wasm_client/notebook.rs): Tab switching (`NOTEBOOK_TAB_SPECS`), 3D dice triggers, 3D model stage, Character Sheet presets, Combat Tracker, Map renderer, and Rules Book viewer/search.
- [`gm_docs.rs`](../../crates/satya-web/src/wasm_client/gm_docs.rs): `GM_ONLY` Document Vault & Google Drive (`drive.file`) OAuth 2.0 / Picker integration.
- [`dice_3d.rs`](../../crates/satya-web/src/wasm_client/dice_3d.rs) & [`dice_3d_wgpu.rs`](../../crates/satya-web/src/wasm_client/dice_3d_wgpu.rs): Procedural polyhedral meshes (`d4`–`d20`), SDF numeral atlas, deterministic rigid-body physics, and WebGPU (`wgpu`) / WGSL 2-pass renderer.
- [`wizard.rs`](../../crates/satya-web/src/wasm_client/wizard.rs), [`wizard_views_stats.rs`](../../crates/satya-web/src/wasm_client/wizard_views_stats.rs), [`wizard_views_lifepath.rs`](../../crates/satya-web/src/wasm_client/wizard_views_lifepath.rs), & [`wizard_views_skills.rs`](../../crates/satya-web/src/wasm_client/wizard_views_skills.rs): Interactive 6-step Character Creation Wizard state, handlers, and HTML stage formatters.
- [`telemetry.rs`](../../crates/satya-web/src/wasm_client/telemetry.rs): Admin-only Apache Iggy Topic & Avro Schema Inspector (`#admin-inspector-card`).

---

## 2. Core Architectural Rules for `satya-web`

1. **Register All Browser Callbacks via `bind_window_callbacks!`**:
   - Never hand-write `let c = Closure::<dyn Fn(...)>::new(...); bind("...", c.as_ref()); c.forget();` blocks in `wasm_client/mod.rs`. Add a single declarative entry to `bind_window_callbacks!` in [`wasm_client/mod.rs`](../../crates/satya-web/src/wasm_client/mod.rs).
   - Keep `onclick="window.satya..."` / `onchange="window.satya..."` SSR attributes and `on:click` / `on:change` Leptos handlers in sync so both static SSR HTML and hydrated/Wasm views work identically.
2. **Drive UI Options & Tabs from Single-Source-of-Truth Catalogs**:
   - Notebook tab buttons in [`views_notebook.rs`](../../crates/satya-web/src/views_notebook.rs) and tab switching in [`notebook.rs`](../../crates/satya-web/src/wasm_client/notebook.rs) must both iterate `NOTEBOOK_TAB_SPECS` / `NOTEBOOK_TABS`.
   - Rules Book chapters in `<select id="rules-chapter-select">` must be generated directly from `satya_engine::rules_book_catalog()`.
   - Character Wizard steps (`1..=6`) must consume `satya-engine` catalogs (`CANONICAL_CHARACTERISTICS`, `CANONICAL_APTITUDES`, `CANONICAL_SKILL_TREES`, `CAREER_TRACK_CATALOG`, etc.) and canonical derived-stat functions (`compute_d_cog_score`, `compute_ooda_score`, `compute_pow_striking_power`, etc.) rather than duplicating formulas in `satya-web`.
3. **Use Shared DOM & RNG Helpers in `wasm_client/dom.rs`**:
   - Always use `toggle_element_display`, `set_display`, `set_text`, `set_html`, `input_value`, `select_value`, `roll_browser_die_i32`, and `roll_browser_die_i64` from [`wasm_client/dom.rs`](../../crates/satya-web/src/wasm_client/dom.rs) instead of ad-hoc `web_sys::window().unwrap().document()...` chains or inline `js_sys::Math::random()` arithmetic.
4. **Separate Pure HTML Formatters from `web-sys` Mutation Handlers**:
   - Keep HTML rendering functions pure (`format_*_html(...) -> String`) so they can be unit-tested natively on the host (`cargo test -p satya-web`) without a headless browser.
