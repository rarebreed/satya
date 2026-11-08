# `satya-signaling` Agent Coding & Architecture Rules

When reading, adding, or refactoring code in `crates/satya-signaling`, activate the [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) skill and follow these architectural rules (full guide in [`docs/technical/rust-workspace-steering.md`](../../docs/technical/rust-workspace-steering.md)):

1. **Pragmatic File Size Ceiling (`<= 45 KB`) & Modular Domain Split**:
   - Keep every `.rs` file in `crates/satya-signaling/src/` under `45 KB` (`< 46,080` bytes) so it fits inside a single agent `read_file` call without truncation.
   - Organize routes and domain state across focused modules with flat `pub use` re-exports in `src/lib.rs`:
     - `auth.rs`: Persistent SQLx SQLite (`users.db`) Argon2id user accounts, RBAC (`AccountRole`), password-protected session state (`ProtectedSession`), and ephemeral in-memory BYOK/OAuth vaults.
     - `state.rs`: Shared `SignalingAppState`, message visibility target resolution (`resolve_message_targets`), WebSocket fanout, and disconnect lifecycle processing.
     - `http_handlers.rs`: SSR shell (`/`), `/api/rules/*`, `/api/auth/*`, `/api/sessions/*`, and `/api/storage/*` admin endpoints.
     - `gm_api.rs`: AI Game Master bridge endpoints (`/api/sessions/:id/gm-config`, `/api/gm/sessions/:id/*`), Co-Pilot vs. Autonomous trigger filters, and FastAPI LangGraph turn dispatch.
     - `gm_docs.rs`, `gm_persistence.rs` & `npcs.rs`: `GM_ONLY` document vault (`/api/sessions/:id/docs`), persistent GM memory/notes, unified GM omnisearch (`/api/sessions/:id/lookup`), prompt-driven NPC generation & role-filtered NPC roster (`/api/sessions/:id/npcs`), HTML/script stripping, minor-safety filtering, and prompt-injection neutralization.
     - `characters.rs`: Character persistence (`/api/characters/*`), `.satyachar` portable bundle export/import, HMAC-SHA256 anti-cheat verification (`satya_engine::verify_character_sheet_legality`), and multimodal character assets.
     - `ws.rs`: Full-duplex `/ws/session` WebSocket upgrade, real-time chat, WebRTC SDP/ICE relay, prosody telemetry, and TTRPG task resolution.

2. **Strict Visibility, Security & Ephemeral Credential Boundaries**:
   - **Never Persist or Broadcast Ephemeral Secrets**: Ephemeral BYOK LLM API keys (`SessionAiGmVault::api_key`) and Google OAuth `drive.file` access tokens must remain strictly in memory—never write them to SQLite, Apache Iggy/Avro topics, or WebSocket broadcasts.
   - **Strict `GM_ONLY` & `PRIVATE` Isolation**: `GM_ONLY` documents, persistent GM memory notes, and private whispers must never leak to unauthorized player sockets or REST queries. Always verify caller identity via `verify_auth_token` or `verify_gm_service_header`.
   - **Sanitize Untrusted Uploads**: All player/GM document and note uploads must pass through `sanitize_and_guard_gm_document` (stripping HTML/scripts/images, rejecting explicit content, and neutralizing `<untrusted_player_document>` prompt-injection breakout tokens).

3. **Zero Duplicated Engine Math or Schema DTOs**:
   - Always import request/response DTOs and event structs from `satya-schema` and call authoritative calculators from `satya-engine` (`evaluate_satya_cascade_task`, `evaluate_satya_contested_cascade_task`, `verify_character_sheet_legality`, `list_rules_chapters`, `search_rules_book`).

4. **High-Signal Docstrings & Unit Suffixes (No `#![deny(missing_docs)]`)**:
   - Never enable `#![deny(missing_docs)]`. Include `//!` module headers and `///` docstrings on HTTP/WS handlers and state managers, and encode units in identifiers (`_ms`, `_us`, `_secs`, `_bytes`, `_sha256`, `_hex`).
