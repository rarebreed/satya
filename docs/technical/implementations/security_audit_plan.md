# Satya Security Audit & Hardening Roadmap

**Date**: 2026-11-26  
**Scope**: Authentication, RBAC, authorization sequencing, campaign/session/character endpoints, input validation, and ports/sockets/WebSockets/TLS across `satya-signaling`, `satya-web`, `satya-storage`, and `satya_ml`.

---

## Summary of Audit Findings

| # | Severity | Wave | Status | Finding | Primary Files |
|---|----------|------|--------|---------|---------------|
| 1 | Critical | Wave 1 | **Done** | Unauthenticated IDOR + inline content serving on character assets | `crates/satya-signaling/src/characters.rs`, `crates/satya-web/src/wasm_client/` |
| 2 | Critical | Wave 1 | **Done** | RBAC, quota, rate-limit bypass & cross-GM memory overwrite in session import | `crates/satya-signaling/src/session_export.rs` |
| 3 | Critical | Wave 2 | **Done** | Hardcoded default Admin/GM credentials & shared dev service token | `crates/satya-signaling/src/auth.rs`, `gm_api.rs`, `crates/satya-web/src/views_gate.rs`, `config/ai.toml`, `crates/satya-storage/src/lib.rs`, `src/satya_ml/` |
| 4 | High | Wave 1 | **Done** | Zombie WebSockets survive logout, leave & session deletion | `crates/satya-signaling/src/http_handlers.rs`, `state.rs`, `ws.rs`, `campaigns.rs` |
| 5 | High | Wave 1 | **Done** | Private session roster leak via `POST /api/sessions/leave` | `crates/satya-signaling/src/http_handlers.rs` |
| 6 | High | Wave 1 | **Done** | TOCTOU races in join-request decision, cancellation & quotas | `crates/satya-signaling/src/auth.rs`, `campaigns.rs` |
| 7 | High | Wave 2 | **Done** | Missing brute-force rate limits, Argon2 async-runtime DoS & spoofable `X-Forwarded-For` | `crates/satya-signaling/src/http_handlers.rs`, `session_persistence.rs`, `rate_limit.rs`, `auth.rs`, `tls.rs`, `bin/satya-server.rs` |
| 8 | High | Wave 3 | **Done** | Port, socket & WebSocket hardening gaps (query tokens, `Origin`, frame/rate caps, TLS timeouts, `/maps`) | `crates/satya-signaling/src/ws.rs`, `campaigns.rs`, `tls.rs`, `lib.rs`, `src/satya_ml/ai_player/server.py`, `crates/satya-web/src/wasm_client/` |
| 9 | Medium | Wave 2 | **Done** | Non-expiring `auth_tokens` and `join_tokens` + missing revoke-all support | `crates/satya-signaling/src/auth.rs`, `crates/satya-storage/src/lib.rs` |
| 10 | Medium | Wave 2 | **Done** | Directory search `LIKE` wildcard injection, email existence oracle & private session join requests | `crates/satya-signaling/src/campaigns.rs` |
| 11 | Medium | Wave 2 | **Done** | Weak password policy & unbounded text inputs | `crates/satya-signaling/src/auth.rs`, `ws.rs`, `campaigns.rs`, `http_handlers.rs` |
| 12 | Medium | Wave 3 | **Done** | Missing HTTP security headers, incomplete session purge cleanup & `resolve_local_maps_dir` bug | `crates/satya-signaling/src/lib.rs`, `campaigns.rs`, `map_vault.rs` |
| 13 | High | Wave 4 | **Done** | Path traversal via unsanitized vault resource IDs/`short_id` & deterministic dev character HMAC key | `crates/satya-signaling/src/characters.rs`, `map_vault.rs`, `security.rs` |
| 14 | High | Wave 4 | **Done** | Raw `TcpStream` HTTP builders, self-port/BYOK SSRF, header CRLF & email header injection | `crates/satya-signaling/src/gm_api.rs`, `player_ai_api.rs`, `email_notifier.rs`, `security.rs` |
| 15 | High | Wave 4 | **Done** | Python FastAPI fail-closed bridge auth, `/debug/api/*` unauthenticated/CSRF/SSE DoS & `models.py` SSRF/key-leak | `src/satya_ml/signaling_client.py`, `agent_server.py`, `ai_gm/`, `ai_player/`, `debug_dashboard/` |
| 16 | High | Wave 4 | **Done** | Cross-session `gm_memories` IDOR/RAM cache poison, Observer escalation & AI player human-kick guard | `crates/satya-signaling/src/gm_persistence.rs`, `session_persistence.rs`, `player_ai_api.rs` |
| 17 | Medium | Wave 4 | **Done** | HTTP 8 MiB / 4 MiB body DoS, unbounded WS outbound channels, bounded event replay & Unicode `'İ'` panic | `crates/satya-signaling/src/lib.rs`, `state.rs`, `session_persistence.rs`, `gm_docs.rs`, `session_export.rs` |
| 18 | Medium | Wave 4 | **Done** | WebRTC camera/peer teardown leaks, unvalidated/flooding peer signaling, frontend & rules-book XSS escaping | `crates/satya-web/src/wasm_client/`, `crates/satya-rules-book/src/rules_book_html.rs` |

---

## Key Design Decisions & Verification Notes

- **`AccountRole::can_host_sessions()` Design**: In `crates/satya-schema/src/protocol.rs`, `can_host_sessions()` intentionally returns `true` for `Self::User | Self::Gm | Self::Admin` (all authenticated users may host or import sessions up to per-user quotas and rate limits). Keep this behavior unchanged unless product requirements explicitly narrow hosting privileges.
- **No Hardcoded Default Accounts in E2E Tests**: Never use seeded demo accounts or `"satya-truth-2026"` in `e2e/` Playwright tests; always register fresh ephemeral accounts using `secrets.token_urlsafe(12)`.
- **Existing Rust Tests vs. Dev-Only Seeding (Wave 2 #3 Pitfall)**: Many existing Rust unit/integration tests in `crates/satya-signaling/src/tests_persistence.rs`, `tests_core.rs`, `tests_campaigns.rs`, and `player_ai_spatial.rs` authenticate as `gm_sean`, `sara_valkyrie`, or `marcus_recon` with `"satya-truth-2026"`, and send `"satya_gm_secret_dev_token"` in `Authorization` headers. Gate demo account seeding and the fallback dev service token behind an explicit dev/test opt-in config flag (default `false` in production / `config/storage.toml`, enabled only via explicit opt-in such as `SATYA_ENABLE_DEMO_ACCOUNTS=1` or in test helpers / `#[cfg(test)]`) so unit tests remain green while production deployments never auto-seed accounts or accept a hardcoded default service token.
- **Verification Commands & Known Quirks**:
  - `crates/satya-web/src/wasm_client/map_2d_art_sprites.rs:371:30` has a pre-existing `clippy::clone-on-copy` in test code; run targeted Clippy checks for `satya-web`:
    - `cargo clippy --offline -p satya-web --lib -- -D warnings`
    - `cargo clippy --offline -p satya-web --target wasm32-unknown-unknown -- -D warnings`
    - `cargo clippy --offline -p satya-web --features e2e-hooks --target wasm32-unknown-unknown -- -D warnings`
  - Build the E2E server binary with `cargo build --offline --profile e2e -p satya-signaling --bin satya-server` (`target/e2e/satya-server.exe`) to avoid file locks if a debug server is running.
  - Rebuild the Wasm E2E bundle with `pixi run -e e2e build-wasm-e2e` (`pkg-e2e/`) and run Playwright with `pixi run -e e2e test-e2e-fast`.

---

## Wave 1 — Immediate Access Control Fixes (Done & Verified)

### #1 (Critical) — Unauthenticated IDOR + Inline Content Serving on Character Assets
- **Issue**: `get_character_asset_handler` previously checked ownership only when an `Authorization` header was present, allowing unauthenticated fetches by `(character_id, asset_id)`, and reflected unsanitized `mime_type`/`filename` inline without `X-Content-Type-Options: nosniff`.
- **Implemented Fix**:
  - `crates/satya-signaling/src/characters.rs`: Added `normalize_safe_asset_mime` (allowlisting `image/png`, `image/jpeg`, `image/webp`, `image/gif`, `application/pdf`, `model/gltf-binary`, `application/octet-stream`) and `sanitize_asset_filename`; enforced the allowlist in `upload_character_asset_handler`; unconditionally required `require_authenticated_user` + `owner_id == user.user_id || Admin` + `X-Content-Type-Options: nosniff` + `attachment` disposition fallback in `get_character_asset_handler`.
  - Wasm Client (`crates/satya-web/src/wasm_client/dom.rs`, `character_sheet_vault.rs`, `mod.rs`): Added `fetch_bytes`, replaced unauthenticated `<a href>` links with `<button onclick="window.satyaOpenCharacterAsset(...)">`, implemented `satya_open_character_asset`, and registered `"satyaOpenCharacterAsset"` in `for_all_window_callbacks!`.

### #2 (Critical) — RBAC, Quota, Rate-Limit Bypass & Cross-GM Memory Overwrite in Session Import
- **Issue**: `POST /api/sessions/import` lacked `can_host_sessions()`, `session_create` rate limits, and `max_total_sessions_per_gm` quotas; skipped `target_sid::` prefixing on `mem_npc_*` memories (`INSERT ... ON CONFLICT DO UPDATE`); trusted caller `campaign_tag`/`author_user_id`; and bypassed `sanitize_and_guard_gm_document`.
- **Implemented Fix**:
  - `crates/satya-signaling/src/session_export.rs`: Hardened `import_session_snapshot` and `import_session_snapshot_handler` with `can_host_sessions()`, `session_create` rate limiting, `max_total_sessions_per_gm` (`429 TOO_MANY_REQUESTS`), `403 FORBIDDEN` when overwriting another user's existing session, forced `target_sid::` prefixing on all `gm_documents` and `gm_memories` (including `mem_npc_*`), forced `campaign_tag`/`uploader_user_id`/`author_user_id`, and `sanitize_and_guard_gm_document` on imported docs.

### #4 (High) — Zombie WebSockets Surviving Logout, Leave & Session Deletion
- **Issue**: Logout, `POST /api/sessions/leave`, and `DELETE /api/sessions/:id` dropped outbound `tx` senders without terminating inbound WebSocket read loops (`ws_rx.next()`) on `/ws/session` and `/ws/notifications`.
- **Implemented Fix**:
  - `crates/satya-signaling/src/http_handlers.rs` (`logout_handler`), `state.rs` (`disconnect_account_sockets`), `ws.rs` (`handle_ws_client`), `campaigns.rs` (`handle_notifications_socket`): Added `oneshot` cancellation (`closed_tx`/`closed_rx`) + `tokio::select!` in both read loops, called `disconnect_account_sockets` on logout, and enforced roster membership in `handle_ws_client`.

### #5 (High) — Private Session Roster Leak via `POST /api/sessions/leave`
- **Issue**: Non-member callers to `POST /api/sessions/leave` received `remaining_roster` containing active session participants.
- **Implemented Fix**:
  - `crates/satya-signaling/src/http_handlers.rs` (`leave_session_handler`): Changed non-member responses to return an empty `remaining_roster`.

### #6 (High) — TOCTOU Races in Join-Request Decision, Cancellation & Quotas
- **Issue**: Non-atomic read-then-write updates in join-request approval and cancellation could race or overwrite transitioned states, and duplicate pending requests lacked a DB-level uniqueness constraint.
- **Implemented Fix**:
  - `crates/satya-signaling/src/auth.rs`: Added partial unique index `idx_join_req_unique_pending ON session_join_requests(session_id, requester_user_id) WHERE status = 'PENDING'`.
  - `crates/satya-signaling/src/campaigns.rs`: Mapped unique constraint violations to `409 CONFLICT` in `persist_and_notify_access_request`; guarded `decide_join_request_handler` with atomic `WHERE request_id = ? AND status = 'PENDING'` (plus rollback on `CreateSession` failure); guarded `cancel_join_request_handler` with `WHERE request_id = ?2 AND status = 'PENDING'`.

### Wave 1 Verification Summary
- Rust: `test_wave1_security_hardening_assets_import_leave_and_join_races` in `crates/satya-signaling/src/tests_persistence.rs` passed.
- Playwright E2E: `e2e/test_security_wave1.py` (`test_zombie_websockets_close_on_logout_and_session_delete` and `test_wave1_api_access_controls_in_live_server`) plus all existing E2E specs passed (`6 passed` via `pixi run -e e2e test-e2e-fast`).

---

## Wave 2 — Auth, Rate-Limit, Token TTL, Directory & Input Hardening (Done & Verified)

### #3 (Critical) — Hardcoded Default Admin/GM Credentials & Shared Dev Service Token
- **Where**:
  - `crates/satya-signaling/src/auth.rs` (`seed_default_users` around lines 485–522)
  - `crates/satya-web/src/views_gate.rs` (`DEMO_ACCOUNT_SPECS` lines 13–33 and usage lines 47–71)
  - `config/ai.toml` (`service_token = "satya_gm_secret_dev_token"` line 7)
  - `crates/satya-storage/src/lib.rs` (`AuthConfig` line 142, `AiGmConfig::default` line 337)
  - `crates/satya-signaling/src/gm_api.rs` (`verify_gm_service_header` lines 30–49)
  - `crates/satya-signaling/src/characters.rs` (line 222), `crates/satya-signaling/src/player_ai_api.rs`
  - `src/satya_ml/signaling_client.py` (`gm_service_token` lines 75–87), `src/satya_ml/agent_server.py` (line 56)
- **Issue**:
  - Fresh databases auto-seed `gm_sean` (`ADMIN`), `host_elena` (`GM`), `sara_valkyrie`, and `marcus_recon` with password `"satya-truth-2026"`.
  - `views_gate.rs` embeds `DEMO_ACCOUNT_SPECS` (including `"satya-truth-2026"`) into public SSR HTML quick-fill buttons.
  - `config/ai.toml` and `AiGmConfig::default()` / `gm_service_token()` ship a static `service_token = "satya_gm_secret_dev_token"` granting full GM-level read/write access on `/api/gm/sessions/*` and `/api/player-ai/sessions/*`, compared with non-constant-time `!=`.
- **Implementation Checklist**:
  1. Gate demo account seeding behind an explicit dev/test opt-in (`AuthConfig.seed_demo_accounts: bool`, defaulting to `false` in production / `config/storage.toml`, and controlled by `SATYA_ENABLE_DEMO_ACCOUNTS=1` or explicit test setup / `#[cfg(test)]`). Keep bootstrap admin/GM role assignment safe if those usernames are never pre-seeded in prod.
  2. Remove `DEMO_ACCOUNT_SPECS` quick-fill chips containing embedded passwords from default SSR in `crates/satya-web/src/views_gate.rs` (or gate behind an opt-in Cargo feature that is off by default, and update `crates/satya-web/src/tests.rs` assertions as needed).
  3. Require `SATYA_AI_GM_SERVICE_TOKEN` from the environment / secret manager in non-dev deployments instead of relying on `"satya_gm_secret_dev_token"` in `config/ai.toml`, `AiGmConfig`, and `src/satya_ml/signaling_client.py`. Reject requests in `verify_gm_service_header` when `expected` is empty or unconfigured.
  4. Replace `clean != expected` in `verify_gm_service_header` (and any other secret token comparison) with constant-time equality (using a constant-time byte comparison helper or `subtle::ConstantTimeEq`).

### #7 (High) — Missing Brute-Force Rate Limits, Argon2 Async-Runtime DoS & Spoofable `X-Forwarded-For`
- **Where**:
  - `crates/satya-signaling/src/http_handlers.rs` (`register_handler` line 38, `login_handler` lines 75–91, `join_session_handler` lines 402–433)
  - `crates/satya-signaling/src/session_persistence.rs` (`resume_session_handler`)
  - `crates/satya-signaling/src/rate_limit.rs` (`RateLimiter`, `extract_client_ip`)
  - `crates/satya-signaling/src/auth.rs` (`authenticate_user`, password hashing/verification)
  - `crates/satya-storage/src/lib.rs` (`QuotaConfig`)
  - `crates/satya-signaling/src/bin/satya-server.rs` (`axum::serve`), `crates/satya-signaling/src/tls.rs` (`serve_https`)
- **Issue**:
  - `/api/auth/login`, `/api/sessions/join`, and `/api/sessions/resume` have no rate limits, enabling password brute-forcing and CPU exhaustion because Argon2id verification executes synchronously on Tokio async worker threads.
  - `extract_client_ip` blindly trusts the leftmost `X-Forwarded-For` (or `X-Real-IP`) header, enabling callers to spoof random IPs on every request to bypass `max_registrations_per_ip_per_hour` and grow `RateLimiter::windows`.
  - `authenticate_user` returns distinct `"Unknown username"` vs `"Invalid password"` messages and skips Argon2 on unknown usernames (username enumeration oracle).
- **Implementation Checklist**:
  1. Add configurable rate limits to `QuotaConfig` (e.g., per-IP and per-username/session limits per window) and enforce them in `login_handler`, `join_session_handler`, and `resume_session_handler`, returning `429 TOO_MANY_REQUESTS` (`Retry-After`).
  2. Offload Argon2id hashing and verification in `crates/satya-signaling/src/auth.rs` onto `tokio::task::spawn_blocking` gated by a bounded `tokio::sync::Semaphore` so heavy login traffic cannot starve the Tokio executor.
  3. Update `authenticate_user` to return a uniform `"Invalid username or password"` error for both unknown usernames and wrong passwords, and run a dummy Argon2id verify against a precomputed constant hash when the username is missing so response timing stays uniform.
  4. Wire `ConnectInfo<SocketAddr>` in `crates/satya-signaling/src/bin/satya-server.rs` (`app.into_make_service_with_connect_info::<SocketAddr>()`) and inject peer `SocketAddr` in `tls.rs` `serve_https`. Extract `Option<ConnectInfo<SocketAddr>>` in HTTP handlers so `oneshot` tests without a TCP socket still pass. Update `extract_client_ip(headers, peer_ip)` to use `peer_ip` by default and only trust the rightmost proxy-appended hop from `X-Forwarded-For` when an explicit `SATYA_TRUSTED_PROXY_HOPS` setting (> 0) is configured.

### #9 (Medium) — Non-Expiring `auth_tokens` and `join_tokens`
- **Where**:
  - `crates/satya-signaling/src/auth.rs` (`issue_auth_token`, `verify_auth_token`, `logout_user`, `join_tokens` schema/issuance/verification)
  - `crates/satya-storage/src/lib.rs` (`AuthConfig` / `QuotaConfig`)
- **Issue**:
  - `satya_tok_*` rows in `auth_tokens` never expire (even across restarts), `join_tokens` have no TTL, and users have no way to revoke all active tokens.
- **Implementation Checklist**:
  1. Add `expires_at_us` to `auth_tokens` (via idempotent `ALTER TABLE` migration for existing SQLite databases), set a configurable default TTL (e.g., 7–30 days), and reject/delete expired tokens inside `verify_auth_token`.
  2. Add a short TTL (e.g., 5–15 minutes) to `join_tokens` and enforce expiry when redeeming/upgrading WebSocket connections.
  3. Prune expired `auth_tokens` and `join_tokens` at startup and periodically, and add a "revoke all sessions/tokens" operation (`POST /api/auth/logout-all` / `revoke_all_user_tokens`) that also invokes `disconnect_account_sockets`.

### #10 (Medium) — Directory Search `LIKE` Wildcard Injection, Email Existence Oracle & Private Session Join Requests
- **Where**:
  - `crates/satya-signaling/src/campaigns.rs` (`search_gm_directory_handler` and `create_join_request_handler`)
- **Issue**:
  - Handle prefix matching (`format!("{handle}%")`) does not escape SQLite `LIKE` wildcards (`%`, `_`, `\`), allowing `q=%` to enumerate accounts alongside `is_online` status.
  - Exact email searches return matching accounts even when the target has zero discoverable campaigns or sessions (email-registration oracle).
  - `create_join_request_handler` checks `allow_join_requests` without checking `is_discoverable`, letting attackers target private non-discoverable sessions (and trigger offline owner emails) if they guess `session_id`.
- **Implementation Checklist**:
  1. Escape `\`, `%`, and `_` in `search_gm_directory_handler` using `ESCAPE '\'`, and enforce a minimum trimmed handle query length (e.g., `>= 3` characters).
  2. For non-self directory queries (both handle and email lookups), suppress users who have zero discoverable campaigns and zero discoverable sessions.
  3. In `create_join_request_handler`, reject join requests when `is_discoverable == 0` unless the requester is already an authorized/invited participant.

### #11 (Medium) — Weak Password Policy & Unbounded Text Inputs
- **Where**:
  - `crates/satya-signaling/src/auth.rs` (`register_user_with_email` and `authenticate_user`)
  - `crates/satya-signaling/src/ws.rs` (chat `body` handling)
  - `crates/satya-signaling/src/campaigns.rs` (`create_campaign_handler`, session creation)
  - `crates/satya-signaling/src/http_handlers.rs` (`create_session_handler`)
- **Issue**:
  - Passwords permit as few as 4 characters and have no upper bound (Argon2 CPU/memory DoS on huge inputs).
  - Chat `body`, campaign/session `title` and `description`, `username`/`display_name`, and `session_id` lack consistent length and character-set bounds.
- **Implementation Checklist**:
  1. Enforce password bounds (`8..=128` chars) on registration, and reject passwords `> 128` chars before Argon2 verification on login/join. *(Note: inspect existing Rust tests that register users with short passwords such as `"pw"` or `"secret"` and update those test fixtures as needed.)*
  2. Enforce bounds across user/session inputs: `username` length/charset (e.g., `3..=32` safe chars), `display_name` (`1..=64`), `session_id` (`[A-Za-z0-9_-]{1,64}`), campaign/session `title` (`<= 128`) and `description` (`<= 2048`), and WS chat `body` (e.g., `<= 16 * 1024` bytes).

### Wave 2 Verification Plan
- **Rust integration tests** (`crates/satya-signaling/src/tests_persistence.rs` or a new `tests_security.rs` module):
  - Demo accounts are not seeded when dev-seeding flag is `false`, and `DEMO_ACCOUNT_SPECS` passwords do not appear in default SSR HTML.
  - Empty or invalid `service_token` is rejected (and comparison uses constant-time equality).
  - `/api/auth/login`, `/api/sessions/join`, and `/api/sessions/resume` enforce `429 TOO_MANY_REQUESTS` rate limits; spoofed `X-Forwarded-For` headers are ignored when untrusted proxy hops = 0.
  - `authenticate_user` returns identical errors for unknown usernames and bad passwords.
  - Expired `auth_tokens` and `join_tokens` are rejected and pruned; `logout-all` revokes all tokens and closes sockets.
  - Directory search `q=%` does not enumerate accounts; email search on non-discoverable accounts returns no oracle result; join requests to `is_discoverable == 0` sessions fail.
  - Passwords `< 8` or `> 128` chars, invalid `session_id` slugs, and oversized chat bodies/campaign descriptions are rejected.
- **Playwright E2E tests** (`e2e/test_security_wave2.py` using fresh ephemeral `secrets.token_urlsafe(12)` accounts):
  - Default login gate renders zero quick-fill demo credentials or hardcoded passwords.
  - Repeated failed logins hit `429`, and wrong-username vs wrong-password surface identical UI error text.
  - Directory search `q=%` returns empty; weak registration password (`< 8` chars) is rejected in live UI/API.

---

## Wave 3 — Socket, WebSocket, TLS, Static Routes & Response Headers (Done & Verified)

### #8 (High) — Port, Socket & WebSocket Hardening Gaps
- **Where**:
  - `crates/satya-signaling/src/ws.rs` (lines 40–58)
  - `crates/satya-signaling/src/campaigns.rs` (`ws_notifications_handler`)
  - `crates/satya-signaling/src/tls.rs` (`serve_https`, lines 100–142)
  - `crates/satya-signaling/src/lib.rs` (`/maps` static `ServeDir::new("data/maps")` route)
  - `src/satya_ml/ai_player/server.py` (line 5 `0.0.0.0:8091` LAN guidance)
  - `crates/satya-web/src/wasm_client/` (WebSocket connection URLs)
- **Issue**:
  - `/ws/notifications?auth_token=...` and `/ws/session?join_token=...` pass bearer/join tokens in URL query strings (exposing tokens to proxy/access logs and browser history).
  - Missing `Origin` validation on WebSocket upgrades.
  - No `.max_message_size` / `.max_frame_size`, per-user concurrent WebSocket cap, or per-socket inbound message rate limit.
  - `tls.rs` `serve_https` spawns unbounded tasks with no timeout on `acceptor.accept(tcp_stream)` and no connection `Semaphore` (slowloris DoS).
  - `/maps` mounts `data/maps` unauthenticated via `ServeDir`.
  - `src/satya_ml/ai_player/server.py` documents binding `0.0.0.0:8091` over plain HTTP (unencrypted service token + BYOK `X-Satya-Provider-Key` headers).
- **Implementation Checklist**:
  1. Replace query-string tokens on `/ws/session` and `/ws/notifications` with short-lived, single-use WebSocket tickets (e.g., `POST /api/ws/ticket`) or first-frame authentication; update the Wasm client WebSocket connectors in `crates/satya-web` accordingly.
  2. Validate the `Origin` header on WebSocket upgrade handlers against allowed hosts/origins.
  3. Set `.max_message_size(256 * 1024)` and `.max_frame_size(64 * 1024)` on `WebSocketUpgrade`, cap concurrent open sockets per account, and add per-socket inbound frame rate limiting.
  4. Wrap `acceptor.accept(tcp_stream)` in `tls.rs` `serve_https` with a `10s` `tokio::time::timeout` and guard accepted connections with a bounded `tokio::sync::Semaphore`.
  5. Remove `/maps` public static serving in `lib.rs` or gate it behind authentication/session ownership.
  6. Keep `ai_player/server.py` bound to loopback `127.0.0.1` by default and document requiring TLS or SSH tunneling for non-loopback hops.

### #12 (Medium) — Missing HTTP Security Headers, Incomplete Session Purge & Local Maps Path Bug
- **Where**:
  - `crates/satya-signaling/src/lib.rs` (`build_router`)
  - `crates/satya-signaling/src/campaigns.rs` (`purge_session_records`)
  - `crates/satya-signaling/src/map_vault.rs` (`resolve_local_maps_dir`, lines 36–41)
- **Issue**:
  - Responses lack `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, `Content-Security-Policy`, and `Strict-Transport-Security` (under TLS). Because `satya_tok_*` lives in `localStorage`, any XSS would yield persistent account takeover.
  - `purge_session_records` does not clear in-memory `state.ai_players` / `state.ai_rally_counters` or non-persistent `session_npcs` / `gm_documents` / `gm_memories` rows for the deleted session.
  - `resolve_local_maps_dir` reads `SATYA_LOCAL_MAPS_DIR` and discards the value, falling back to `default_local_satya_maps_dir()`.
- **Implementation Checklist**:
  1. Add a security headers middleware/layer in `build_router` (`X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, Wasm-compatible `Content-Security-Policy`, and `HSTS` when TLS is active). *(Verify Leptos SSR `onclick` attributes and `'wasm-unsafe-eval'` requirements before tightening `script-src`.)*
  2. Extend `purge_session_records` to clear `state.ai_players` and `state.ai_rally_counters` and delete non-persistent `session_npcs`, `gm_documents`, and `gm_memories` rows for the purged `session_id`.
  3. Fix `resolve_local_maps_dir` in `map_vault.rs` so non-empty `SATYA_LOCAL_MAPS_DIR` values are actually returned.

### Wave 3 Verification Plan
- Rust integration tests for WS ticket single-use & TTL, `Origin` check rejection, oversized WS frame handling, `/maps` auth/removal, security headers on SSR HTML & JSON APIs, full `purge_session_records` cleanup, and `resolve_local_maps_dir` env override.
- Playwright E2E test verifying live WS connection via short-lived tickets and presence of security response headers.

---

## Wave 4 — Extended Audit & Full-Stack Hardening (Done & Verified)

- **#13 Vault Path Traversal & HMAC Signing**: Enforced `validate_safe_resource_id` and sanitized `short_id` components in `crates/satya-signaling/src/characters.rs` and `map_vault.rs`, replaced deterministic fallback character HMAC key with an ephemeral per-process `OsRng` secret when `SATYA_CHARACTER_SIGNING_KEY` is unset, and verified signatures with `ct_eq_str`.
- **#14 SSRF, Header Injection & Raw HTTP Replacement**: Replaced raw `TcpStream` HTTP builders in `gm_api.rs` and `player_ai_api.rs` with pooled `reqwest::Client`, added `validate_provider_base_url` (blocking link-local/metadata/private/untrusted destinations), `validate_api_key_header_safe`, self-port loopback SSRF blocking in `validate_ai_player_service_url`, and CRLF/control-character sanitization in `email_notifier.rs`.
- **#15 Python Bridge & Debug Dashboard Hardening**:
  - Removed hardcoded dev token fallback in `src/satya_ml/signaling_client.py` and fixed `is_loopback_url` using `urllib.parse.urlparse(url).hostname`.
  - Updated `verify_service_auth` in `src/satya_ml/agent_server.py` to fail closed (`503`) when unconfigured and compare tokens with `hmac.compare_digest`; added `BodySizeLimitMiddleware` (4 MiB -> `413`) to `ai_gm/server.py` and `ai_player/server.py`.
  - Protected `/debug/api/*` in `src/satya_ml/debug_dashboard/router.py` with token auth + same-origin CSRF (`Origin`/`Sec-Fetch-Site`) checks, added CSP/`nosniff`/`X-Frame-Options` headers on `/debug/`, capped SSE subscribers (`16` -> `429`) in `hub.py`, and escaped interpolated fields + supported `X-Satya-Debug-Token` in `static/app.js`.
  - Hardened `src/satya_ml/ai_gm/models.py` (`_validate_base_url_safety`) to reject metadata/unspecified/non-loopback Ollama IPs and prevent forwarding server environment API keys to any non-official cloud host (including loopback).
- **#16 Cross-Session Authorization & Escalation Guards**: Scoped `gm_memories` ownership checks in `gm_persistence.rs` before in-memory mutation, blocked Observer -> Player/GM escalation in `session_persistence.rs` (`resume_session`), restricted `remove_session_ai_player_handler` to `ai-player-*` AI agents, and sanitized imported `gm_memories` / validated `active_map` in `session_export.rs`.
- **#17 Server DoS & Parser Resilience**: Added `DefaultBodyLimit::max(8 * 1024 * 1024)` in `lib.rs`, bounded WS outbound channels (`WS_OUTBOUND_CHANNEL_CAP = 256` with `try_send`), bounded HTTP/1 header timeout & `1024` connection semaphore in `tls.rs`, bounded `load_visible_events_from_db` (`LIMIT 2000`), and switched `strip_html_and_media_tags` to `.to_ascii_lowercase()` to prevent Unicode `'İ'` slice panics.
- **#18 WebRTC Media Privacy & Frontend XSS**:
  - Updated `crates/satya-web/src/wasm_client/webrtc.rs`, `auth_session.rs`, and `campaigns_directory.rs` to tear down active `MediaStream` tracks and `RtcPeerConnection`s on session switch, deletion, and WS reconnect exhaustion, prevent concurrent camera leaks after `getUserMedia`, validate signals against human `roster` peers, throttle duplicate offers (`< 1000ms`), and cap `pending_ice_candidates` at `64`.
  - Escaped token chip IDs (`escape_js_single_quoted_attr`), `active_stance`, telemetry fingerprints, and WebRTC track DOM IDs in `crates/satya-web/src/wasm_client/`, and added `is_safe_markdown_url` + `rel="noopener noreferrer"` in `crates/satya-rules-book/src/rules_book_html.rs`.

---

## Verification Results (2026-11-26)

All checks passed after the final code edits (with non-embedded docs updated afterward):

- **Rust Clippy (`-D warnings`)**:
  - `pixi run cargo clippy --offline -p satya-storage -p satya-rules-book -p satya-signaling --all-targets -- -D warnings` -> 0 errors, 0 warnings
  - `pixi run cargo clippy --offline -p satya-web --lib -- -D warnings` -> 0 errors, 0 warnings
  - `pixi run cargo clippy --offline -p satya-web --target wasm32-unknown-unknown -- -D warnings` -> 0 errors, 0 warnings
  - `pixi run cargo clippy --offline -p satya-web --features e2e-hooks --target wasm32-unknown-unknown -- -D warnings` -> 0 errors, 0 warnings
- **Rust Unit & Integration Tests**:
  - `satya-rules-book`: 3 passed, 0 failed
  - `satya-signaling`: 44 passed, 0 failed (`tests_core`, `tests_campaigns`, `tests_persistence`, `tests_map`, `tests_map_floors`, `tests_security`, TLS & spatial suites)
  - `satya-web`: 102 passed, 0 failed, 1 pre-existing ignored (`export_phase4_showcase_svgs`)
- **Python Type Check & Tests**:
  - `pixi run pyrefly check src/satya_ml/signaling_client.py src/satya_ml/agent_server.py src/satya_ml/ai_gm/server.py src/satya_ml/ai_gm/models.py src/satya_ml/ai_player/server.py src/satya_ml/debug_dashboard/router.py src/satya_ml/debug_dashboard/hub.py tests/test_security_ml.py tests/test_ai_debug_dashboard.py tests/test_wave2_regressions.py` -> 0 errors
  - `$env:PYTHONPATH="src"; pixi run python -m pytest tests/test_ai_debug_dashboard.py tests/test_ai_gm_multiplayer_and_npcs.py tests/test_ai_player_langgraph.py tests/test_phase1_2_awakened_hooks_and_maps.py tests/test_security_ml.py tests/test_wave2_regressions.py` -> 35 passed, 0 failed
  - `$env:PYTHONPATH="src"; pixi run python -m pytest tests/test_satya_ml.py -k "not test_all_versioned_avro_schemas_parse_with_fastavro"` -> 5 passed, 1 deselected *(pre-existing schema count assertion expecting 14 `.avsc` files where 16 exist on disk)*
- **Playwright End-to-End (`pixi run -e e2e test-e2e-fast`)**:
  - Built `target/e2e/satya-server.exe` (`--profile e2e`) and `pkg-e2e/` (`pixi run -e e2e build-wasm-e2e`)
  - 10 passed, 0 failed in 48.95s (`test_auth_resume.py`, `test_multiplayer.py`, `test_tabletop.py`, `test_security_wave1.py`, `test_security_wave2.py`, `test_security_wave3.py`)
