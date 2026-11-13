# `satya-signaling` Agent Coding & Architecture Rules

Activate [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-signaling`.

1. **Complete Module Inventory (`crates/satya-signaling/src/`)**:
   - **Production (`14` modules)**: `auth.rs` (SQLite Argon2id RBAC + ephemeral vaults), `characters.rs` (`.satyachar` + HMAC legality), `gm_api.rs` (AI GM REST bridge + FastAPI dispatch), `gm_docs.rs` (`GM_ONLY` vault + Google OAuth `drive.file`), `gm_persistence.rs` (persistent GM notes + omnisearch), `http_handlers.rs` (SSR `/`, `/api/rules/*`, `/api/auth/*`, `/api/sessions/*`, `/api/storage/*`), `map_handlers.rs` (`GET /map`, `POST /map/move`, role filtering, `log_map_canvas_event`), `map_authoring_handlers.rs` (`POST /map` generation + edge/prop authoring), `npcs.rs` (role-filtered NPCs), `player_ai_api.rs`, `session_export.rs` (`.satyasession` + `.md`/`.json` export), `session_persistence.rs`, `state.rs` (`SignalingAppState` + WS fanout), `ws.rs` (`/ws/session`).
   - **Tests (`3` modules)**: `tests_core.rs`, `tests_map.rs`, `tests_persistence.rs`.
2. **Ephemeral Secrets, `GM_ONLY` Isolation, & Upload Sanitization**:
   - Keep BYOK LLM keys (`SessionAiGmVault::api_key`) and Google `drive.file` tokens strictly in memory—never write them to SQLite, Iggy/Avro logs, or WebSocket broadcasts.
   - Gate `GM_ONLY` docs/memories/tokens and `PRIVATE` whispers behind `verify_auth_token` or `verify_gm_service_header`, and route untrusted uploads through `sanitize_and_guard_gm_document`.
3. **Zero Duplicated Math or DTOs**:
   - Import wire DTOs from `satya-schema` and invoke spatial/mechanical calculators from `satya-engine` (`resolve_token_destination_cm`, `evaluate_satya_cascade_task`, `verify_character_sheet_legality`).
