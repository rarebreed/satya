# `satya-web` Agent Coding & Architecture Rules

When reading, adding, or refactoring code in `crates/satya-web`, activate the [`satya-rust-web`](../../.agents/skills/satya-rust-web/SKILL.md) and [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) skills and follow these mandatory architectural rules (full guide in [`docs/technical/rust-web-steering.md`](../../docs/technical/rust-web-steering.md) and [`docs/technical/rust-workspace-steering.md`](../../docs/technical/rust-workspace-steering.md)):

1. **Hard File Size Ceiling (`<= 45 KB` / `< 46,080` bytes) & Flat Re-Exports**:
   - Every `.rs` and `.css` file in `crates/satya-web/src/` MUST remain under `45 KB` (~1,000 lines) so it fits inside a single AI agent `read_file` call without truncation.
   - Keep SSR views split across `lib.rs` (app shell), `views_gate.rs` (header + auth/session gate), `views_notebook.rs` + `views_notebook_docs.rs` (left 10-tab Tabletop Notebook including `NPC List`), `views_workspace.rs` (center chat/task roller + right WebRTC dock), and `tests.rs`.
   - Keep large `wasm_client/` domains split into companion modules (`notebook.rs` + `gm_docs.rs` + `npcs.rs`, `dice_3d.rs` + `dice_3d_wgpu.rs`, `wizard.rs` + `wizard_views_stats.rs` + `wizard_views_lifepath.rs` + `wizard_views_skills.rs`) and re-export `pub use companion::*;` in both the parent module and `wasm_client/mod.rs`.
   - Keep CSS split between `styles.css` (core layout, gate, chat, WebRTC, 3D dice/model) and `styles_notebook.css` (GM Docs, Rules Book `.satya-rules-article`, and Character Wizard `.wiz-*`).

2. **Declarative `window.satya*` Registration (`bind_window_callbacks!`)**:
   - Register all `window.satya*` browser callbacks via a single 1-line entry in the `bind_window_callbacks!` macro inside `src/wasm_client/mod.rs`. Never hand-write `Closure::new` + `bind` + `forget` blocks.

3. **Single Source of Truth with `satya-engine` & Static UI Tables**:
   - Drive Notebook tab buttons (`views_notebook.rs`) and tab switching (`wasm_client/notebook.rs`) from `NOTEBOOK_TAB_SPECS` / `NOTEBOOK_TABS`.
   - Drive Rules Book `<select id="rules-chapter-select">` options directly from `satya_engine::rules_book_catalog()`.
   - Never duplicate `satya-engine` formulas or catalogs in `satya-web`; always call `satya_engine` functions (`evaluate_satya_cascade_task`, `compute_exact_cascade_distribution`, `compute_d_cog_score`, `compute_ooda_score`, `compute_pow_striking_power`, `format_rules_markdown_to_html`, etc.).

4. **Shared DOM, Drawer & RNG Helpers (`wasm_client/dom.rs` & `webrtc.rs`)**:
   - Use `toggle_element_display`, `set_display`, `set_text`, `set_html`, `input_value`, `select_value`, `roll_browser_die_i32`, and `roll_browser_die_i64` in `wasm_client/dom.rs` rather than raw `web_sys` chains or inline `js_sys::Math::random()` math.
   - Use `sync_drawer_rail_and_aria` and `sync_center_drawer_card` in `wasm_client/webrtc.rs` for collapsible drawer state synchronization.

5. **Pure Host-Testable HTML Formatters**:
   - Keep complex HTML generation in pure functions (`format_*_html(...) -> String`) separate from DOM event handlers so all UI views and state transitions can be tested via `cargo test -p satya-web`.
