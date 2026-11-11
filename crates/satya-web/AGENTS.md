# `satya-web` Agent Coding & Architecture Rules

When reading, adding, or refactoring code in `crates/satya-web`, activate [`satya-rust-web`](../../.agents/skills/satya-rust-web/SKILL.md) and [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md).

## Technical Documentation Suite (`docs/technical/satya-web/`)
Read only the specific chapter relevant to your task instead of scanning the entire crate:
- [`docs/technical/satya-web/README.md`](../../docs/technical/satya-web/README.md): One-shot SSR + Wasm execution model, complete `thread_local!` & `localStorage` state map, REST/WS contract index, and fast-path file router.
- [`01-ssr-shell-and-theming.md`](../../docs/technical/satya-web/01-ssr-shell-and-theming.md): SSR component tree, collapsible/floating/resizable drawers, and 6-axis CSS theming (`styles_themes.css`, `styles.css`, `styles_stages.css`, `styles_notebook.css`).
- [`02-3d-tactical-map-diorama.md`](../../docs/technical/satya-web/02-3d-tactical-map-diorama.md): 3D Tactical Map (`map_3d.rs`, `map_3d_wgpu.rs`, `wgpu_shared.rs`), `1.0m` grid/elevation/cutaway geometry, gait reach, LoS/Cover laser, `.glb` tokens, ray-plane picking, and SVG fallback.
- [`03-3d-dice-and-glb-stage.md`](../../docs/technical/satya-web/03-3d-dice-and-glb-stage.md): 3D Polyhedral Dice (`dice_3d.rs`, `dice_3d_wgpu.rs`, SDF atlas, `SplitMix64` trajectory, 2-pass glass WGSL) & Binary glTF 2.0 (`.glb`) codec/stage (`glb_model.rs`, `model_3d_wgpu.rs`).
- [`04-character-wizard-sheet-and-vault.md`](../../docs/technical/satya-web/04-character-wizard-sheet-and-vault.md): 6-step Character Wizard (`wizard*.rs`), 13-zone SVG Character Sheet (`character_sheet*.rs`), and `.satyachar` Avro Vault (`character_sheet_vault.rs`, `character_sheet_record.rs`).
- [`05-session-webrtc-chat-and-gm-vault.md`](../../docs/technical/satya-web/05-session-webrtc-chat-and-gm-vault.md): Auth/Session snapshots (`auth_session.rs`, `session_persistence.rs`), WebRTC mesh & RMS prosody (`webrtc.rs`), `/roll` DSL (`chat_cmd.rs`), `2d20` Cascade/Triad drawer (`chat_and_ttrpg.rs`, `ttrpg_drawer_cascade.rs`, `ttrpg_drawer_triad.rs`), GM Vault (`gm_docs.rs`, `gm_memories_and_director.rs`), NPCs (`npcs.rs`), and Telemetry (`telemetry.rs`).

## Mandatory Architectural Invariants

1. **Single-Read File Size Budget (`< 40 KB` & `< 780` Lines) + Flat `pub use` Re-Exports**:
   - Keep every `.rs`, `.css`, and `.md` file compact enough to fit inside a single `read_file` tool call (`800`-line / `46,080`-byte limit) without multi-part reads.
   - Always re-export `pub use companion::*;` in parent modules and `wasm_client/mod.rs` when splitting files.
2. **Declarative `window.satya*` Registration (`bind_window_callbacks!`)**:
   - `SatyaApp` renders one-shot SSR HTML (`SatyaApp().to_html()`). Browser interactivity binds via `bind_window_callbacks!` in `src/wasm_client/mod.rs` (which also populates `REGISTERED_WINDOW_CALLBACKS`, verified against SSR HTML in `cargo test`). Never hand-write `Closure::new` + `Reflect::set` + `forget` blocks.
3. **Single Source of Truth with `satya-engine` (`1 Tick = 0.25s`)**:
   - Consume `satya-engine` catalogs and types (`NOTEBOOK_TAB_SPECS` [10 tabs], `rules_book_catalog()`, `canonical_combat_technique_catalog()` [30 techniques], `CANONICAL_FIREARM_PRESETS`, `CANONICAL_FIELD_GEAR_PRESETS`, `BoneFractureStatus`, `SECONDS_PER_TICK = 0.25`) — never duplicate domain math or tables in `satya-web`.
4. **Shared `dom.rs` & `wgpu_shared.rs` Helpers**:
   - Use `dom.rs` (`escape_html`, `escape_js_single_quoted_attr`, `fetch_json`, `fetch_text`, `local_storage_*`, `read_first_file_bytes`, `prompt_save_bytes`, `active_session_and_token_or_feedback`) and `wgpu_shared.rs` (`OrbitOrthoCamera`, `init_wgpu_canvas_context`, `create_depth_view`, `SHARED_GLB_DIORAMA_WGSL`).
5. **Host-Testable Formatters, State Transitions & Offline `naga` Shader Validation**:
   - Keep HTML formatters (`format_*_html -> String`), CPU 3D mesh builders (`build_tactical_diorama_vertices`), and WGSL shaders (`naga` dev-dependency) host-testable via `cargo test --offline -p satya-web`.
