# `satya-signaling` Agent Coding & Architecture Rules

Activate [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-signaling`.

1. **Module Responsibility Map (`crates/satya-signaling/src/`)**:
   - **Server State, Auth & Real-Time Relay**: `state.rs` (`SignalingAppState` + WS fanout), `auth.rs` (SQLite Argon2id RBAC, ephemeral vaults, `session_world_ledger`), `http_handlers.rs` (SSR `/`, `/api/rules/*`, `/api/auth/*`, `/api/sessions/*`, `/api/storage/*`), `ws.rs` (`/ws/session`).
   - **Session Replay, Portable Archives & Characters**: `session_persistence.rs` (SQLite history rehydration), `session_export.rs` (`.satyasession` + `.md`/`.json` transcripts), `characters.rs` (`.satyachar` vault + HMAC legality).
   - **GM Vault, Lore, Omnisearch, NPCs & World Clocks**: `gm_docs.rs` (`GM_ONLY` vault + Google OAuth `drive.file`), `gm_persistence.rs` (persistent GM docs/memories + omnisearch), `campaign_lore.rs` (`config/campaign.toml` pack resolution + lore/NPC omnisearch), `npcs.rs` + `npcs_store.rs` (role-filtered NPCs, `PATCH`/`DELETE`, SQLite storage & dedup), `world_clocks.rs` (`SessionWorldLedger` REST + Heat/clock mutations).
   - **Tactical Maps & AI Service Bridges**: `map_handlers.rs` (`GET /map`, `POST /map/move`, role filtering), `map_authoring_handlers.rs` (`POST /map` generation + edge/prop authoring), `gm_api.rs` (AI GM REST bridge + FastAPI dispatch), `player_ai_api.rs` (AI Player context & turn dispatch), `player_ai_spatial.rs` (self-only AI Player map move + public lore/NPC lookup).
   - **Integration Test Suites**: `tests_core.rs`, `tests_map.rs`, `tests_map_floors.rs`, `tests_persistence.rs`.
2. **Multi-Table NPC / GM Vault Lifecycle, Omnisearch Dedup & `COLLATE NOCASE`**:
   - Session NPCs span `session_npcs`, `characters`, `session_characters` (`npc:<cid>`), and companion `mem_<character_id>` notes in `gm_memories`. Reuse existing same-name `character_id`s on regeneration (`generate_session_npc_handler`), exclude `npc_*` / `session_npcs` rows from `load_session_characters_from_db`, deduplicate companion `mem_<id>` / `mem_npc_*` hits when `SessionNpc` results are shown in `campaign_lore.rs`, cascade-sync `is_persistent` and `DELETE` across all linked tables + `gm_memories`, and compare `campaign_tag` case-insensitively (`COLLATE NOCASE`).
3. **Ephemeral Secrets, `GM_ONLY` Isolation, & Upload Sanitization**:
   - Keep BYOK LLM keys (`SessionAiGmVault::api_key`) and Google `drive.file` tokens strictly in memory—never write them to SQLite, Iggy/Avro logs, or WebSocket broadcasts.
   - Gate `GM_ONLY` docs/memories/tokens and `PRIVATE` whispers behind `verify_auth_token` or `verify_gm_service_header`, and route untrusted uploads through `sanitize_and_guard_gm_document`.
4. **Zero Duplicated Math or DTOs**:
   - Import wire DTOs from `satya-schema` and invoke spatial/mechanical calculators from `satya-engine` (`resolve_token_destination_cm`, `evaluate_satya_cascade_task`, `verify_character_sheet_legality`).
