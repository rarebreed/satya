---
name: satya-rust-web
description: >-
  Full-stack Leptos SSR and WebAssembly (wasm-bindgen, web-sys, wgpu) architecture,
  complete module map, SSR/Wasm parity contracts, and host testability steering guide
  for crates/satya-web.
---

# Rust Web & WebAssembly Technical Steering Guide (`satya-web`)

This steering guide defines the module boundaries, file-size budgets, SSR↔Wasm parity invariants, and host-testability standards for [`crates/satya-web`](../../crates/satya-web). It extends [`docs/technical/rust-workspace-steering.md`](rust-workspace-steering.md) and pairs with [`docs/technical/rust-engine-steering.md`](rust-engine-steering.md) and the 6-part architecture reference in [`docs/technical/satya-web/README.md`](satya-web/README.md).

---

## 1. Complete Module Map (`< 40 KB` & `< 780` Lines Per File)

Every `.rs`, `.css`, and `.md` file is kept under `40 KB` raw and `< 780` lines so AI coding agents can read any file in a single `read_file` call without line (`800`) or byte (`46,080`) truncation.

### 1.1 Top-Level SSR Shell, Stylesheets & Slash-Command Parser (`crates/satya-web/src/`)
- [`lib.rs`](../../crates/satya-web/src/lib.rs): Root `SatyaApp` component, `render_satya_app_html()` one-shot SSR entrypoint, `WASM_BOOTSTRAP_LOADER` FOUC-free appearance loader, inline concatenation of the 4 stylesheets, and flat `pub use` re-exports.
- [`views_header.rs`](../../crates/satya-web/src/views_header.rs) & [`views_gate.rs`](../../crates/satya-web/src/views_gate.rs): `HeaderBar` (status pills, Hamburger Menu, table-driven 6-axis appearance controls, chat filters, BYOK AI GM & AI Player panels), `PasswordToggleButton`, and `AuthAndSessionGate` (`#auth-card`, `#session-card`).
- [`views_notebook.rs`](../../crates/satya-web/src/views_notebook.rs), [`views_notebook_docs.rs`](../../crates/satya-web/src/views_notebook_docs.rs) & [`views_notebook_gm.rs`](../../crates/satya-web/src/views_notebook_gm.rs): Left-column 10-tab Tabletop Notebook (`maps`, `dice`, `model`, `character`, `wizard`, `npcs`, `combat`, `rules`, `docs`, `iggy`).
- [`views_workspace.rs`](../../crates/satya-web/src/views_workspace.rs): Active 3-column workspace (`#notebook-column`, `#workspace-col-splitter`, `#center-column` with `#chat-card` + `#ttrpg-drawer`, and `#webrtc-dock-column` with `#webrtc-card` + `#right-drawer-rails`).
- [`chat_cmd.rs`](../../crates/satya-web/src/chat_cmd.rs): Pure Rust slash-command parser (`/msg`, `/w`, `/whisper`, `/roll`, `/r`, `/help`), fuzzy `@recipient` resolver, `|>` dice pipeline evaluator (`e`, `hi`, `lo`, `sum`, `gte`, `lte`), and `ChatInputHistory`.
- [`styles_themes.css`](../../crates/satya-web/src/styles_themes.css), [`styles.css`](../../crates/satya-web/src/styles.css), [`styles_stages.css`](../../crates/satya-web/src/styles_stages.css) & [`styles_notebook.css`](../../crates/satya-web/src/styles_notebook.css): 13 color palettes + 5 structural frames (`styles_themes.css`), core layout/WebRTC/chat/roller (`styles.css`), 3D WebGPU map/dice/model stages (`styles_stages.css`), and GM Vault/Rules Book/Wizard/Character Sheet (`styles_notebook.css`).
- [`tests.rs`](../../crates/satya-web/src/tests.rs): Host SSR parity, `REGISTERED_WINDOW_CALLBACKS` verification, DOM ID uniqueness, and formatter unit tests.

### 1.2 Browser WebAssembly Runtime (`crates/satya-web/src/wasm_client/`)
- [`mod.rs`](../../crates/satya-web/src/wasm_client/mod.rs): `#[wasm_bindgen(start)]` entrypoint, submodule re-exports, and `bind_window_callbacks!` (generating both `window.satya*` JS closures and `REGISTERED_WINDOW_CALLBACKS`).
- [`state.rs`](../../crates/satya-web/src/wasm_client/state.rs): `SatyaWasmState`, `PeerEntry`, `SECONDS_PER_TICK` (`0.25s`), and full `thread_local!` topology index.
- [`dom.rs`](../../crates/satya-web/src/wasm_client/dom.rs): Typed `web-sys` DOM helpers, `escape_html` / `escape_js_single_quoted_attr`, `fetch_json` / `fetch_text`, `local_storage_*`, `read_first_file_bytes`, `prompt_save_bytes`, `active_session_and_token_or_feedback`, and RNG helpers.
- [`options.rs`](../../crates/satya-web/src/wasm_client/options.rs) & [`menu.rs`](../../crates/satya-web/src/wasm_client/menu.rs): Pure enum parsers, appearance catalogs/normalizers, Hamburger Menu toggles, and BYOK AI GM / AI Player controls.
- [`auth_session.rs`](../../crates/satya-web/src/wasm_client/auth_session.rs) & [`session_persistence.rs`](../../crates/satya-web/src/wasm_client/session_persistence.rs): Auth/session lifecycle, pure `format_roster_and_recipients_html`, boot auto-restore, sequence-sorted `replay_session_history_snapshot`, `.satyasession` bundles, and `.md`/`.json` transcript export.
- [`webrtc.rs`](../../crates/satya-web/src/wasm_client/webrtc.rs): `/ws/session` signaling dispatch, P2P `RTCPeerConnection` + `satya-chat-dc` mesh, ICE queuing, `250ms` WebAudio RMS prosody monitor, and drawer/column-splitter state machine.
- [`chat_and_ttrpg.rs`](../../crates/satya-web/src/wasm_client/chat_and_ttrpg.rs), [`ttrpg_drawer_cascade.rs`](../../crates/satya-web/src/wasm_client/ttrpg_drawer_cascade.rs) & [`ttrpg_drawer_triad.rs`](../../crates/satya-web/src/wasm_client/ttrpg_drawer_triad.rs): Targeted Markdown chat, `2d20` Cascade odds + searchable skill combobox, and 30-technique `Tempo / Control / Force` Triad allocator.
- [`notebook.rs`](../../crates/satya-web/src/wasm_client/notebook.rs), [`gm_docs.rs`](../../crates/satya-web/src/wasm_client/gm_docs.rs), [`gm_memories_and_director.rs`](../../crates/satya-web/src/wasm_client/gm_memories_and_director.rs), [`npcs.rs`](../../crates/satya-web/src/wasm_client/npcs.rs) & [`telemetry.rs`](../../crates/satya-web/src/wasm_client/telemetry.rs): 10-tab hub, Combat Tracker (`0.25s/tick`), Rules Book reader/search, Google Drive `drive.file` Vault, GM Memories, 12-dial Scenario Director & Omnisearch, role-aware NPC roster/generator, and Admin Apache Iggy/Avro inspector.
- [`wgpu_shared.rs`](../../crates/satya-web/src/wasm_client/wgpu_shared.rs), [`map_3d.rs`](../../crates/satya-web/src/wasm_client/map_3d.rs), [`map_3d_wgpu.rs`](../../crates/satya-web/src/wasm_client/map_3d_wgpu.rs), [`dice_3d.rs`](../../crates/satya-web/src/wasm_client/dice_3d.rs), [`dice_3d_wgpu.rs`](../../crates/satya-web/src/wasm_client/dice_3d_wgpu.rs), [`glb_model.rs`](../../crates/satya-web/src/wasm_client/glb_model.rs) & [`model_3d_wgpu.rs`](../../crates/satya-web/src/wasm_client/model_3d_wgpu.rs): Shared 3D math/`OrbitOrthoCamera`/`wgpu` bootstrap, 3D Tactical Map diorama, 3D Polyhedral Dice, and Binary glTF 2.0 (`.glb`) stage.
- [`wizard.rs`](../../crates/satya-web/src/wasm_client/wizard.rs), [`wizard_views_stats.rs`](../../crates/satya-web/src/wasm_client/wizard_views_stats.rs), [`wizard_views_lifepath.rs`](../../crates/satya-web/src/wasm_client/wizard_views_lifepath.rs) & [`wizard_views_skills.rs`](../../crates/satya-web/src/wasm_client/wizard_views_skills.rs): 6-step Character Creation Wizard.
- [`character_sheet.rs`](../../crates/satya-web/src/wasm_client/character_sheet.rs), [`character_sheet_views.rs`](../../crates/satya-web/src/wasm_client/character_sheet_views.rs), [`character_sheet_equip_views.rs`](../../crates/satya-web/src/wasm_client/character_sheet_equip_views.rs), [`character_sheet_status_views.rs`](../../crates/satya-web/src/wasm_client/character_sheet_status_views.rs), [`character_sheet_vault.rs`](../../crates/satya-web/src/wasm_client/character_sheet_vault.rs) & [`character_sheet_record.rs`](../../crates/satya-web/src/wasm_client/character_sheet_record.rs): Interactive Operator Character Sheet, 13-zone SVG trauma/bleed/psyche tracker, and `.satyachar` Avro Vault.

---

## 2. Core Architectural & Verification Rules

1. **Declarative `window.satya*` Binding & Automated SSR Parity**:
   - Register all browser callbacks in `bind_window_callbacks!` (`wasm_client/mod.rs`). Host unit tests verify that every `window.satya*` handler emitted by SSR or dynamic HTML formatters exists in `REGISTERED_WINDOW_CALLBACKS` and that all SSR `id="..."` attributes are globally unique.
2. **Single Source of Truth with `satya-engine`**:
   - Never duplicate TTRPG domain math, firearm/gear presets, or status enums in `satya-web`; import them from `satya-engine` (`CANONICAL_FIREARM_PRESETS`, `CANONICAL_FIELD_GEAR_PRESETS`, `BoneFractureStatus`, `canonical_combat_technique_catalog()`, `rules_book_catalog()`, `1 Tick = 0.25s`).
3. **Host-First Testability (`cargo test --offline -p satya-web`)**:
   - Keep HTML formatters, state transitions, CPU 3D geometry builders (`build_tactical_diorama_vertices`), and WGSL shaders (`naga` validation) ungated on host targets so full regression verification runs in seconds without a headless browser or GPU.
