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

Use alongside [`satya-rust-workspace`](../satya-rust-workspace/SKILL.md) whenever reading or modifying [`crates/satya-web`](../../../crates/satya-web). Deep architecture references live in [`docs/technical/satya-web/`](../../../docs/technical/satya-web/README.md).

---

## 1. Token-Saving Fast-Path Task Router

Read **only** the specific source file(s) and companion doc for your task (using `rg -n` + `StartLine`/`EndLine` on larger files):

| Task / Feature Area | Primary Source Files (`crates/satya-web/src/`) | Deep-Dive Doc (`docs/technical/satya-web/`) |
| :--- | :--- | :--- |
| **App Bootstrap, `window.satya*` Registry, Client State** | `lib.rs`, `wasm_client/mod.rs`, `wasm_client/state.rs`, `wasm_client/{dom.rs,dom_tooltip.rs}` | [`README.md`](../../../docs/technical/satya-web/README.md) |
| **Header, Hamburger Menu, Drawers/Floating Panes & CSS Layout** | `views_header.rs`, `views_workspace.rs`, `wasm_client/{menu.rs,options.rs,workspace_drawers.rs}`, `styles_themes.css`, `styles.css` | [`01-ssr-shell-and-theming.md`](../../../docs/technical/satya-web/01-ssr-shell-and-theming.md) |
| **3D Tactical Map, Input Handlers & Hover Inspector** | `views_notebook.rs`, `views_notebook_map.rs`, `wasm_client/{map_3d.rs,map_3d_input.rs,map_3d_wgpu.rs,map_3d_diorama_mesh.rs,map_3d_dressing_geom.rs,map_3d_props_extra.rs,map_3d_props_lab_urban.rs,map_3d_surfaces_geom.rs,map_hover_inspect.rs,wgpu_shared.rs}`, `styles_stages.css` | [`02-3d-tactical-map-diorama.md`](../../../docs/technical/satya-web/02-3d-tactical-map-diorama.md) |
| **2D SVG Battlemap Art, Editor Mode, Camera & Continuation** | `views_notebook_map.rs`, `wasm_client/{map_2d_art*.rs,map_2d_camera.rs,map_2d_tokens.rs,map_edge_edit.rs,map_editor.rs,map_hover_inspect.rs}`, `styles_stages.css` | [`06-2d-battlemap-art.md`](../../../docs/technical/satya-web/06-2d-battlemap-art.md) |
| **3D Polyhedral Dice (`d4`–`d20`) & Translucent WGSL** | `wasm_client/{dice_3d.rs,dice_3d_wgpu.rs,wgpu_shared.rs}`, `styles_stages.css` | [`03-3d-dice-and-glb-stage.md`](../../../docs/technical/satya-web/03-3d-dice-and-glb-stage.md) |
| **3D Binary `.glb` Miniature Parser/Exporter & Stage** | `wasm_client/{glb_model.rs,model_3d_wgpu.rs,wgpu_shared.rs}` | [`03-3d-dice-and-glb-stage.md`](../../../docs/technical/satya-web/03-3d-dice-and-glb-stage.md) |
| **6-Step Character Creation Wizard** | `wasm_client/{wizard.rs,wizard_views_stats.rs,wizard_views_lifepath.rs,wizard_views_skills.rs}`, `styles_notebook.css` | [`04-character-wizard-sheet-and-vault.md`](../../../docs/technical/satya-web/04-character-wizard-sheet-and-vault.md) |
| **Character Sheet, 13-Zone SVG Trauma & `.satyachar` Vault** | `wasm_client/{character_sheet.rs,character_sheet_views.rs,character_sheet_equip_views.rs,character_sheet_status_views.rs,character_sheet_vault.rs,character_sheet_record.rs}` | [`04-character-wizard-sheet-and-vault.md`](../../../docs/technical/satya-web/04-character-wizard-sheet-and-vault.md) |
| **Auth, Session Resume, `.satyasession` & Transcripts** | `views_gate.rs`, `wasm_client/{auth_session.rs,session_persistence.rs}` | [`05-session-webrtc-chat-and-gm-vault.md`](../../../docs/technical/satya-web/05-session-webrtc-chat-and-gm-vault.md) |
| **WebSocket Signaling, WebRTC Mesh & Prosody** | `views_workspace.rs`, `wasm_client/{webrtc.rs,workspace_drawers.rs}` | [`05-session-webrtc-chat-and-gm-vault.md`](../../../docs/technical/satya-web/05-session-webrtc-chat-and-gm-vault.md) |
| **Chat `/msg` + `/roll` DSL, `2d20` Cascade & Triad Pool** | `chat_cmd.rs`, `views_workspace.rs`, `wasm_client/{chat_and_ttrpg.rs,ttrpg_drawer_cascade.rs,ttrpg_drawer_triad.rs}` | [`05-session-webrtc-chat-and-gm-vault.md`](../../../docs/technical/satya-web/05-session-webrtc-chat-and-gm-vault.md) |
| **Combat Clock, Rules Book, GM Vault/Drive, Director, NPCs, Iggy** | `views_notebook_docs.rs`, `views_notebook_gm.rs`, `wasm_client/{notebook.rs,gm_docs.rs,gm_docs_google.rs,gm_memories_and_director.rs,gm_omnisearch.rs,npcs.rs,telemetry.rs}` | [`05-session-webrtc-chat-and-gm-vault.md`](../../../docs/technical/satya-web/05-session-webrtc-chat-and-gm-vault.md) |

---

## 2. Core Workflow & Testability Rules

1. **One-Shot SSR, Unique DOM IDs, & `bind_window_callbacks!`**:
   - `render_satya_app_html()` serializes `SatyaApp().to_html()` once on the server (`satya-signaling` `GET /`); `SatyaApp` is **not** client-hydrated by Leptos reactive signals.
   - Register every `window.satya*` handler in `bind_window_callbacks!` (`wasm_client/mod.rs`), which populates `REGISTERED_WINDOW_CALLBACKS`. Include representative outputs of dynamic `format_*_html` emitters (e.g., `format_gm_documents_list_html`, `format_gm_memories_list_html`, `format_gm_omnisearch_hits_html`, `format_npc_card_html`) in `tests_contracts.rs::test_ssr_and_dynamic_window_callbacks_match_registered_bindings` so any missing callback binding or duplicate SSR DOM `id="..."` fails host tests immediately.
   - **Interactive Battlemap Editor Parity (`map_editor.rs`)**: Keep host-testable editor mutations (`EditorHistory` 30-step undo/redo, hit-testing, drag-move, wall stroke rasterization, rect/polygon/ribbon surfaces) in un-gated Rust tested by `cargo test -p satya-web`. In `#[cfg(target_arch = "wasm32")]`, guard `window`-level pointer move/up dispatches with a canvas-armed flag (`MapDioramaController::editor_pointer_active`) so UI toolbar clicks never fire phantom canvas edits; freeze `local_span_cm()` onto `span_w_cm`/`span_h_cm` before rotating props across `90°`/`0°` thresholds; hash `tint_rgb`, `decorative`, `ground_floor_override`, and `ground_tint_rgb` inside `battlemap_art_cache_key`; and place Editor `Ctrl+Z`/`Ctrl+Y` shortcut handling *before* the global `if ev.ctrl_key() || ev.meta_key()` early return in `handle_map_global_keydown`.
2. **Responsibility-Driven Module Boundaries**:
   - Split modules only along distinct UI/protocol responsibilities with flat `pub use companion::*;` re-exports in the parent feature file (avoiding duplicate glob re-exports in `wasm_client/mod.rs`); keep single-responsibility feature modules together regardless of line count rather than micro-splitting or line-golfing fluent/builder code.
3. **Single Source of Truth with `satya-engine` & `satya-schema`**:
   - Drive the 10-tab Notebook from `NOTEBOOK_TAB_SPECS`, Rules chapters from `rules_book_catalog()`, 30 techniques from `canonical_combat_technique_catalog()`, firearms/gear/trauma (`CANONICAL_FIREARM_PRESETS`, `CANONICAL_FIELD_GEAR_PRESETS`, `BoneFractureStatus`), and spatial/hover geometry (`tactical_map`, `tactical_map_inspect`, `MapPropPlacement::footprint_m()`) directly from Rust core crates (`SECONDS_PER_TICK = 0.25`).
4. **Shared `dom.rs`, `dom_tooltip.rs`, & `wgpu_shared.rs` Primitives**:
   - Use `dom.rs` (`escape_html`, `escape_js_single_quoted_attr`, `fetch_json`, `fetch_text`, `local_storage_*`, `read_first_file_bytes`, `prompt_save_bytes`, `active_session_and_token_or_feedback`), `dom_tooltip.rs` (`TooltipClampSpec`, `clamp_tooltip_within_rect`, `StickyHoldToggle`, `sync_floating_tooltip_dom`, `is_keyboard_typing_target`), and `wgpu_shared.rs` (`OrbitOrthoCamera`, `init_wgpu_canvas_context`, `SHARED_GLB_DIORAMA_WGSL`).
5. **Fast Host Verification (`< 2s` Incremental)**:
   - Keep HTML rendering (`format_*_html`), tooltip clamping math, CPU 3D geometry (`build_tactical_diorama_vertices`), and WGSL shaders (`naga` validation) host-testable:
     ```powershell
     cargo test --offline -p satya-web
     cargo clippy --offline -p satya-web --all-targets -- -D warnings
     ```
6. **Mandatory `wasm32` Verification (Host Builds Skip `cfg(target_arch = "wasm32")`)**:
   - Host builds skip every `#[cfg(target_arch = "wasm32")]` item (`*_wgpu_runtime` modules, `wgpu` pipelines, mouse/wheel handlers), so passing host tests does **not** mean `pixi run serve` (`wasm-pack build`) will compile, and **no host unit/integration test can catch wasm-only compile errors**; only a wasm32 compile can. Always also run `pixi run check-wasm` (also part of `prep`, `prepc`, `test-web`, `test`, and CI in `.github/workflows/ci.yml`), or in the agent sandbox, where `pixi` is not on `PATH`:
     ```powershell
     $env:PATH = 'c:\Users\hapah\Projects\satya\.pixi\envs\default\Library\bin;' + $env:PATH; cargo clippy --offline -p satya-web --target wasm32-unknown-unknown -- -D warnings
     ```
   - `pixi` is not on the sandbox `PATH`; prepend `.pixi\envs\default\Library\bin` so `cargo`/`rustc` resolve to the toolchain with `rust-std-wasm32-unknown-unknown`. `wasm-pack build` cannot run inside the standard sandbox because `%LOCALAPPDATA%\.wasm-pack` is outside workspace sandbox permissions; however, `cargo clippy` skips LLVM `--release` codegen, `wasm-bindgen`, and `wasm-opt`. When changing `Cargo.toml`, Wasm exports/imports, or large procedural/3D modules, also verify release codegen offline in the sandbox:
     ```powershell
     $env:PATH = 'c:\Users\hapah\Projects\satya\.pixi\envs\default\Library\bin;' + $env:PATH; cargo build --offline --release -p satya-web --target wasm32-unknown-unknown
     ```
     When modifying `pixi.toml` (`build-wasm` / `serve`) or `[package.metadata.wasm-pack]`, run `pixi run build-wasm` with `BypassSandbox: true` (user approval) to verify the full `wasm-bindgen` packaging step.
   - **Keep `wasm-opt` Disabled on Rust `1.99+`**: Binaryen's `wasm-opt -O` (`Precompute.cpp:838`) crashes (`0xc0000409`) on Rust `1.99+` LLVM-optimized Wasm binaries. Preserve **both** `wasm-opt = false` under `[package.metadata.wasm-pack.profile.release]` / `profiling` in `crates/satya-web/Cargo.toml` and `--no-opt` on `build-wasm` in `pixi.toml`.
   - **New `web-sys` APIs require Cargo features**: when calling a new DOM API (e.g. `Element::get_bounding_client_rect` → `DomRect`, `MouseEvent`, `WheelEvent`, `UiEvent`), add every type it touches to `[workspace.dependencies] web-sys` features in the root [`Cargo.toml`](../../../Cargo.toml).
   - **Gate host-only unused-binding suppressions**: when values are moved inside a `#[cfg(target_arch = "wasm32")]` block, write the host silencer as `#[cfg(not(target_arch = "wasm32"))] let _ = (a, b, c);`. An unconditional `let _ = ...` after the block causes `E0382` (use after move) on wasm only.
7. **Thin `wasm32` Shells, Host-Testable Cores**:
   - Inside `#[cfg(target_arch = "wasm32")]` modules, keep only `web-sys`/`wgpu` I/O (DOM reads, event wiring, GPU calls). Extract pure math and state transitions (e.g. client px + `DomRect` dims → NDC → grid cell, camera orbit deltas, controller state updates) into un-gated `fn`s taking plain numbers/structs, and unit-test them on the host.
   - Shrinking the wasm-only surface reduces the code that only `check-wasm` ever compiles and makes logic regressions catchable by `cargo test`.
8. **Resizable CSS Grid Tracks & Floating Overlay Panes (`styles.css`, `workspace_drawers.rs`)**:
   - `#workspace-col-splitter` persists inline `--col-left-track` on `#workspace-panel`, which overrides class-level `--col-*-track` definitions; collapsed/undocked states must override `grid-template-columns` directly with explicit `minmax(0, 1fr)` / `fr` tracks instead of relying solely on custom properties.
   - Use `display: contents` (`.is-undocked`) on column wrappers containing only `position: fixed` floating panes so grid tracks disappear without hiding the overlays; avoid `overflow: clip` on `.is-floating` cards (use `overflow: hidden` or `auto` so `resize: both` remains functional), and strip inline `left`/`top`/`right`/`width`/`height`/`z-index` on re-dock.
9. **Single Vertical Scroll Container per Notebook Tab (Precision Trackpad Scrolling)**:
   - Scrolling panes (e.g., `#notebook-pane-docs`, `#notebook-pane-npcs`) inherit vertical scrolling from `.notebook-pane { overflow-y: auto; overflow-x: hidden; overscroll-behavior-y: contain; scrollbar-gutter: stable; }`, whereas full-viewport tabs (`#notebook-pane-rules`, `#notebook-pane-wizard`, `#notebook-pane-character`, `#notebook-pane-dice`, `#notebook-pane-maps`) set `overflow: hidden` and delegate scrolling to their dedicated inner viewport.
   - Never put `overflow-y: auto; overscroll-behavior-y: contain;` on un-bounded inner list wrappers (`.gm-docs-list`) or `flex-shrink: 1` min-height traps inside `.notebook-pane` — precision trackpads trap wheel events inside the inner wrapper and block scroll chaining up to `.notebook-pane`. Reserve bounded `overflow-y: auto` only for explicitly `max-height`-capped drawers like `#rules-search-results`.
10. **Session-Scoped Client Cache & Demo Seeding Hygiene**:
   - Reset all room/vault caches (`gm_documents`, `gm_memories`, `gm_omnisearch_active`, `session_npcs`, `selected_npc_id`) in `cleanup_client_session_state`.
   - Gate offline demo seeding (`ensure_default_demo_npc_if_empty`) strictly on `st.session_id.is_none() && st.session_npcs.is_empty()`, overwrite client vectors unconditionally from authoritative server responses (`st.session_npcs = resp.npcs;` without `if !is_empty()` guards), deduplicate optimistic vs. server NPC records by ID and case-insensitive name, and call `refresh_gm_omnisearch_if_active()` after vault/NPC mutations.
