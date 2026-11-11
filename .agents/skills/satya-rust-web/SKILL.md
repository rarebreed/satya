---
name: satya-rust-web
description: >-
  Full-stack Leptos SSR and WebAssembly (wasm-bindgen, web-sys, wgpu) architecture,
  module map, fast-path task router, bind_window_callbacks! registration, and host
  testability guide for crates/satya-web. Activate this skill whenever working on
  crates/satya-web, Leptos SSR views, Wasm browser handlers, 3D WebGPU map/dice/GLB
  pipelines, the Character Wizard/Sheet/Vault, or frontend stylesheets.
---

# Satya Rust Web & WebAssembly Steering Skill (`satya-rust-web`)

Use alongside [`satya-rust-workspace`](../satya-rust-workspace/SKILL.md) whenever reading or modifying [`crates/satya-web`](../../../crates/satya-web). Deep architecture references live in [`docs/technical/satya-web/`](../../../docs/technical/satya-web/README.md) and [`docs/technical/rust-web-steering.md`](../../../docs/technical/rust-web-steering.md).

---

## 1. Token-Saving Fast-Path Task Router

Read **only** the specific source file(s) and companion doc for your task (all files fit in 1 `read_file` call):

| Task / Feature Area | Primary Source Files (`crates/satya-web/src/`) | Deep-Dive Doc (`docs/technical/satya-web/`) |
| :--- | :--- | :--- |
| **App Bootstrap, `window.satya*` Registry, Client State** | `lib.rs`, `wasm_client/mod.rs`, `wasm_client/state.rs`, `wasm_client/dom.rs` | [`README.md`](../../../docs/technical/satya-web/README.md) |
| **Header, Hamburger Menu, Themes & CSS Layout** | `views_header.rs`, `wasm_client/{menu.rs,options.rs}`, `styles_themes.css`, `styles.css` | [`01-ssr-shell-and-theming.md`](../../../docs/technical/satya-web/01-ssr-shell-and-theming.md) |
| **3D Tactical Map, Grid/LoS/Cover & SVG Fallback** | `views_notebook.rs`, `wasm_client/{map_3d.rs,map_3d_wgpu.rs,wgpu_shared.rs}`, `styles_stages.css` | [`02-3d-tactical-map-diorama.md`](../../../docs/technical/satya-web/02-3d-tactical-map-diorama.md) |
| **3D Polyhedral Dice (`d4`–`d20`) & Translucent WGSL** | `wasm_client/{dice_3d.rs,dice_3d_wgpu.rs,wgpu_shared.rs}`, `styles_stages.css` | [`03-3d-dice-and-glb-stage.md`](../../../docs/technical/satya-web/03-3d-dice-and-glb-stage.md) |
| **3D Binary `.glb` Miniature Parser/Exporter & Stage** | `wasm_client/{glb_model.rs,model_3d_wgpu.rs,wgpu_shared.rs}` | [`03-3d-dice-and-glb-stage.md`](../../../docs/technical/satya-web/03-3d-dice-and-glb-stage.md) |
| **6-Step Character Creation Wizard** | `wasm_client/{wizard.rs,wizard_views_stats.rs,wizard_views_lifepath.rs,wizard_views_skills.rs}`, `styles_notebook.css` | [`04-character-wizard-sheet-and-vault.md`](../../../docs/technical/satya-web/04-character-wizard-sheet-and-vault.md) |
| **Character Sheet, 13-Zone SVG Trauma & `.satyachar` Vault** | `wasm_client/{character_sheet.rs,character_sheet_views.rs,character_sheet_equip_views.rs,character_sheet_status_views.rs,character_sheet_vault.rs,character_sheet_record.rs}` | [`04-character-wizard-sheet-and-vault.md`](../../../docs/technical/satya-web/04-character-wizard-sheet-and-vault.md) |
| **Auth, Session Resume, `.satyasession` & Transcripts** | `views_gate.rs`, `wasm_client/{auth_session.rs,session_persistence.rs}` | [`05-session-webrtc-chat-and-gm-vault.md`](../../../docs/technical/satya-web/05-session-webrtc-chat-and-gm-vault.md) |
| **WebSocket Signaling, WebRTC Mesh, Prosody & Drawers** | `views_workspace.rs`, `wasm_client/webrtc.rs` | [`05-session-webrtc-chat-and-gm-vault.md`](../../../docs/technical/satya-web/05-session-webrtc-chat-and-gm-vault.md) |
| **Chat `/msg` + `/roll` DSL, `2d20` Cascade & Triad Pool** | `chat_cmd.rs`, `views_workspace.rs`, `wasm_client/{chat_and_ttrpg.rs,ttrpg_drawer_cascade.rs,ttrpg_drawer_triad.rs}` | [`05-session-webrtc-chat-and-gm-vault.md`](../../../docs/technical/satya-web/05-session-webrtc-chat-and-gm-vault.md) |
| **Combat Clock, Rules Book, GM Vault/Drive, Director, NPCs, Iggy** | `views_notebook_docs.rs`, `views_notebook_gm.rs`, `wasm_client/{notebook.rs,gm_docs.rs,gm_memories_and_director.rs,npcs.rs,telemetry.rs}` | [`05-session-webrtc-chat-and-gm-vault.md`](../../../docs/technical/satya-web/05-session-webrtc-chat-and-gm-vault.md) |

---

## 2. Core Workflow & Testability Rules

1. **One-Shot SSR + `bind_window_callbacks!`**:
   - `render_satya_app_html()` serializes `SatyaApp().to_html()` once on the server (`satya-signaling` `GET /`); `SatyaApp` is **not** client-hydrated by Leptos reactive signals.
   - Register every `window.satya*` handler in `bind_window_callbacks!` (`wasm_client/mod.rs`), which auto-generates `REGISTERED_WINDOW_CALLBACKS` verified against SSR HTML by `cargo test`.
2. **Single-Read File Budget (`< 40 KB` & `< 780` Lines)**:
   - Split growing modules along clean domain boundaries with flat `pub use companion::*;` re-exports so every `.rs`, `.css`, and `.md` file reads in a single tool call without truncation.
3. **Single Source of Truth with `satya-engine` (`SECONDS_PER_TICK = 0.25`)**:
   - Drive the 10-tab Notebook from `NOTEBOOK_TAB_SPECS`, Rules chapters from `rules_book_catalog()`, 30 techniques from `canonical_combat_technique_catalog()`, and firearms/gear/trauma enums (`CANONICAL_FIREARM_PRESETS`, `CANONICAL_FIELD_GEAR_PRESETS`, `BoneFractureStatus`) from `satya-engine`.
4. **Shared `dom.rs` & `wgpu_shared.rs` Primitives**:
   - Always use `escape_html`, `escape_js_single_quoted_attr`, `fetch_json`, `fetch_text`, `local_storage_*`, `read_first_file_bytes`, `prompt_save_bytes`, and `active_session_and_token_or_feedback` in `dom.rs`, and `OrbitOrthoCamera` + `init_wgpu_canvas_context` + `SHARED_GLB_DIORAMA_WGSL` in `wgpu_shared.rs`.
5. **Fast Host Verification (`< 2s` Incremental)**:
   - Keep HTML rendering (`format_*_html`), state transitions, CPU 3D geometry (`build_tactical_diorama_vertices`), and WGSL shaders (`naga` validation) host-testable:
     ```powershell
     cargo test --offline -p satya-web
     cargo clippy --offline -p satya-web --all-targets -- -D warnings
     ```
