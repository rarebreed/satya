# Satya WebRTC, Chat & Event Streaming Platform — Walkthrough

## Summary of Changes

We have built the full-stack **WebRTC Audio/Video, Chat & Event Streaming application** in **Rust, WebAssembly (`wasm-bindgen` / `web-sys`), and Leptos**, managed via **Pixi** (`conda-forge` Rust + Python toolchain) and designed for downstream AI/ML training via **Apache Iggy**, **Apache Avro**, and **PyLance (`lance`)**. Every crate in the workspace enforces `#![deny(missing_docs)]` with full module, struct, enum, and function documentation.

### 1. Project & Workspace Configuration
- [Cargo.toml](Cargo.toml): Root Cargo workspace organizing four specialized crates (`satya-schema`, `satya-storage`, `satya-signaling`, `satya-web`), sharing dependencies including `leptos`, `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, and `web-sys`.
- [pixi.toml](pixi.toml): Manages both the `rust` (`>=1.85`) and `python` (`>=3.14.7`) toolchains plus `pylance`, `fastavro`, `pyarrow`, and `pytest`, with tasks for building the Wasm frontend (`build-wasm`), testing, serving (`serve` / `serve-release`), and running the Avro-to-PyLance ML compaction pipeline.
- [config/storage.toml](config/storage.toml): Declarative configuration for Apache Iggy stream connection (`127.0.0.1:8090` with automatic local broker fallback), micro-batch thresholds (`batch_max_messages`, `batch_max_interval_ms`), and per-topic `local` vs. `s3` storage sinks.

### 2. Versioned Nested Avro Schemas, Protocol DTOs & Evolution Engine (`satya-schema`)
- **Avro Schema Files**:
  - [schemas/v1/schema_registry_event.avsc](schemas/v1/schema_registry_event.avsc)
  - [schemas/v1/session_lifecycle_event.avsc](schemas/v1/session_lifecycle_event.avsc): Includes `SESSION_CREATED`, `PARTICIPANT_JOINED`, `PARTICIPANT_LEFT`, `PARTICIPANT_DISCONNECTED`, `USER_LOGGED_OUT`, `ROLE_CHANGED`, and `SESSION_ENDED`.
  - [schemas/v1/webrtc_telemetry_event.avsc](schemas/v1/webrtc_telemetry_event.avsc): Includes `SIGNALING_OFFER`, `SIGNALING_ANSWER`, `ICE_CANDIDATE`, `PEER_CONNECTED`, `PEER_DISCONNECTED`, `STATS_SNAPSHOT`, `TRACK_MUTE_CHANGE`, `DATA_CHANNEL_OPEN`, and `DATA_CHANNEL_CLOSED`.
  - [schemas/v1/chat_message_event.avsc](schemas/v1/chat_message_event.avsc) and [schemas/v2/chat_message_event.avsc](schemas/v2/chat_message_event.avsc)
  - [schemas/v1/interaction_dynamics_event.avsc](schemas/v1/interaction_dynamics_event.avsc)
  - [schemas/v1/ttrpg_mechanics_event.avsc](schemas/v1/ttrpg_mechanics_event.avsc)
- [crates/satya-schema/src/lib.rs](crates/satya-schema/src/lib.rs):
  - Defines the nested `Participant` and `EventMetadata` structs used across all events.
  - Defines shared HTTP and WebSocket protocol packets (`ClientWsPacket`, `ServerWsPacket`, `RegisterRequest`, `LoginRequest`, `LogoutRequest`, `AuthSuccessResponse`, `CreateSessionRequest`, `JoinSessionRequest`, `LeaveSessionRequest`, `SessionEnterResponse`, and `StorageMetricsViewResponse`) shared directly between the Axum server and the Rust/Wasm browser client.
  - Implements `SchemaRegistry`, 64-bit `crc64_avro_fingerprint`, and `WireFrameHeader` (15-byte header tracking `schema_id`, `schema_version`, and `fingerprint_crc64` for every message).

### 3. Apache Iggy & Offset-Tracked Micro-Batch Storage (`satya-storage`)
- [crates/satya-storage/src/lib.rs](crates/satya-storage/src/lib.rs):
  - Implements `EventLogRouter`, which streams framed Avro events to Apache Iggy and buffers events per topic up to `batch_max_messages`.
  - Flushes Snappy-compressed Avro Object Container segments (`batch_{start_offset:010}_{end_offset:010}.avro`) with embedded `satya.start_offset`, `satya.end_offset`, `satya.schema_version`, and `satya.schema_fingerprint_crc64` metadata to either `LocalFileSystem` or `AmazonS3` via `object_store`.
- [src/satya_ml/lance_ingest.py](src/satya_ml/lance_ingest.py) & [src/satya_ml/inspect_avro.py](src/satya_ml/inspect_avro.py):
  - Incremental compaction script (`pixi run compact-to-lance`) that reads flushed `.avro` batches, checks the high-water offset in `_watermarks.json`, converts nested records into a `pyarrow.Table`, and appends to `data/lance/<topic>.lance`.

### 4. Authentication, Session Disconnect & WebRTC Signaling Server (`satya-signaling` & `satya-web`)
- [crates/satya-signaling/src/auth.rs](crates/satya-signaling/src/auth.rs):
  - Argon2id-backed `AuthAndSessionManager` managing persistent user accounts (`user_id`, `username`, `password_hash`), password-protected sessions (`session_id` + `session_password`), explicit session disconnect (`leave_session`), and token-revoking user logout (`logout_user`).
- [crates/satya-signaling/src/lib.rs](crates/satya-signaling/src/lib.rs):
  - Serves the Leptos HTML shell at `GET /` and static compiled WebAssembly assets at `/pkg` (`ServeDir::new("pkg")`).
  - Provides REST endpoints for `/api/auth/register`, `/api/auth/login`, `/api/auth/logout`, `/api/sessions/create`, `/api/sessions/join`, `/api/sessions/leave`, `/api/storage/metrics`, and `/api/storage/flush`.
  - WebSocket signaling & message router (`/ws/session`) enforcing public broadcast vs. private recipient isolation (`PUBLIC`, `PRIVATE`, `GM_ONLY`, `HOST_ONLY`), WebRTC SDP/ICE relay, Satya d20 task resolution & combat action economy, autonomous AI GM narration, and disconnect event logging (`process_session_disconnect`).
- [crates/satya-web/src/lib.rs](crates/satya-web/src/lib.rs) & [crates/satya-web/src/wasm_client.rs](crates/satya-web/src/wasm_client.rs):
  - Pure Rust + Leptos + WebAssembly (`wasm-bindgen` / `web-sys`) frontend (`cdylib` + `rlib`).
  - Handles user login/registration/logout, password-protected session creation/join/disconnect, WebRTC `RTCPeerConnection` audio/video + `RTCDataChannel` mesh, Web Audio API `AnalyserNode` RMS prosody telemetry, targeted private/public chat, and live Apache Iggy topic/schema inspection without custom JavaScript runtime files.

---

## Validation & Test Results

All workspace unit, integration, and doc tests were executed via `pixi run test-rust` (`cargo test --workspace`) and linted with `cargo clippy --workspace --all-targets -- -D warnings`:

- **`satya-schema` (4 passed)**:
  - `test_all_builtin_schemas_register_and_fingerprint`: Verified all 7 schemas (`v1` across 6 topics + `v2` evolved `ChatMessageEvent`) parse and generate deterministic 64-bit `CRC-64-AVRO` fingerprints.
  - `test_nested_participant_and_schema_evolution_v1_to_v2`: Verified nested `Participant` serialization in `ChatMessageEvent` (`sender` and `recipients`) and verified live schema evolution (decoding a `v1` payload using the `v2` reader schema with default `sentiment_score` and `detected_intent` resolution).
  - `test_interaction_dynamics_and_ttrpg_mechanics_serialization`: Verified `InteractionDynamicsEvent` (with nested `speaker` and `overlapped_participants`) and `TtrpgMechanicsEvent` (Satya d20 pool vs. Attribute TN and combat stance action count cost).
  - `test_disconnect_and_logout_lifecycle_and_webrtc_events`: Verified Avro serialization of `LifecycleEventType::UserLoggedOut`, `LifecycleEventType::ParticipantDisconnected`, and `WebRtcEventKind::PeerDisconnected`, plus JSON round-tripping of `ClientWsPacket` and `ServerWsPacket`.
- **`satya-storage` (1 passed)**:
  - `test_micro_batch_flush_and_avro_header_watermarks`: Verified automatic batch flushing to `batch_0000000000_0000000002.avro` and confirmed the embedded `satya.start_offset`, `satya.end_offset`, and `satya.schema_version` header metadata and nested `Participant` records round-trip cleanly.
- **`satya-signaling` (3 passed)**:
  - `test_user_auth_session_password_and_private_message_isolation`: Verified Argon2id user registration/login (`user_id` persistence), password-protected session creation, rejection of invalid session passwords, and private message recipient isolation.
  - `test_axum_http_endpoints_and_leptos_render`: Verified the Leptos UI renders at `GET /` and `/api/storage/metrics` reports all 6 topics and their registered schema fingerprints.
  - `test_session_leave_and_user_logout_emit_disconnect_events`: Verified `POST /api/sessions/leave` and `POST /api/auth/logout` revoke tokens, update the active roster, and log `PARTICIPANT_LEFT`, `PEER_DISCONNECTED`, `SESSION_ENDED`, and `USER_LOGGED_OUT` Avro events.
- **`satya-web` (2 passed)**:
  - `test_wasm_client_enum_parsers_and_metrics_table_render`: Verified the Rust/Wasm enum parsers and Apache Iggy metrics table renderer.
  - `test_leptos_app_renders_wasm_bootstrap_and_disconnect_controls`: Verified `render_satya_app_html()` outputs the `/pkg/satya_web.js` Wasm module bootstrap and the session disconnect/logout controls.

---

## Pixi Task Reference

| Category | Pixi Command | Description |
| :--- | :--- | :--- |
| **Build & Quality** | `pixi run check` | Type-check all 4 Rust crates (`cargo check --workspace`) |
| | `pixi run fmt` | Format all Rust code (`cargo fmt --all`) |
| | `pixi run lint` | Run Clippy lints across all Rust targets (`cargo clippy --workspace --all-targets`) |
| | `pixi run build-wasm` | Compile `satya-web` Rust/Leptos frontend to WebAssembly in `pkg/` (`wasm-pack build crates/satya-web --target web --out-dir ../../pkg`) |
| | `pixi run build` / `pixi run build-release` | Build debug or optimized release workspace binaries |
| **Granular & Polyglot Tests** | `pixi run test-schema` | Run Avro schema, CRC-64-AVRO fingerprint, and `v1 -> v2` evolution tests |
| | `pixi run test-storage` | Run Apache Iggy WAL and offset-tracked `.avro` micro-batching tests |
| | `pixi run test-signaling` | Run Argon2id user auth, session password, disconnect/logout, and private routing tests |
| | `pixi run test-web` | Run Leptos SSR and Rust/Wasm client unit tests |
| | `pixi run test-rust` | Run all Rust unit, integration, and doc tests (`cargo test --workspace`) |
| | `pixi run test-py` | Run Python unit tests (`pytest`) |
| | `pixi run test` | Polyglot DAG task (`depends-on = ["test-rust", "test-py"]`) running both Rust and Python test suites |
| **Server Runtime** | `pixi run serve` | Automatically runs `build-wasm` and launches the Satya Server on `http://127.0.0.1:3000` |
| | `pixi run serve-release` | Automatically runs `build-wasm` and launches the optimized release server on `http://127.0.0.1:3000` |
| **ML & Data Pipeline** | `pixi run inspect-schemas` | Validate and list all versioned `.avsc` schemas and fields using `fastavro` |
| | `pixi run compact-to-lance` | Incrementally compact flushed `.avro` topic batches into `.lance` AI datasets |
| | `pixi run inspect-topics` | Inspect flushed `.avro` batch offset watermarks and `.lance` table row counts |
| | `pixi run ml-pipeline` | Pipeline DAG task (`depends-on = ["compact-to-lance", "inspect-topics"]`) |

---

## Building & Running the Application Locally

### Option A: Using Pixi (Recommended One-Command Workflow)
`pixi run serve` automatically runs `build-wasm` first (compiling `crates/satya-web` into `pkg/satya_web.js` and `pkg/satya_web_bg.wasm`) and then starts the Axum signaling server on port `3000`:

```powershell
pixi run serve
```

*(Or run the steps individually with `pixi run build-wasm` followed by `pixi run serve`.)*

### Option B: Using `wasm-pack` and `cargo` Directly
If you are running outside Pixi, ensure `wasm-pack` and the `wasm32-unknown-unknown` target are installed:

```powershell
# 1. One-time setup (if not already installed)
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# 2. Compile the satya-web crate to WebAssembly in ./pkg
wasm-pack build crates/satya-web --target web --out-dir ../../pkg

# 3. Start the Axum signaling & event server (serves / and /pkg/* on http://127.0.0.1:3000)
cargo run -p satya-signaling --bin satya-server
```

### Connecting Browser Clients & Testing the Pipeline
1. **Connect Browser Clients**:
   - Open two or more browser tabs at `http://127.0.0.1:3000`.
   - Use the **Quick Fill Demo Accounts** (or register new accounts) to log in as `gm_sean` in Tab 1 (click **Host: Create Session** with Session ID `harsh-reality-ep1` and password `orbital-drop-7`) and `sara_valkyrie` in Tab 2 (click **Join Existing Session** with the same Session ID and password).
   - Enable Camera/Mic, send Public or Private targeted messages, roll Satya d20 tasks, test **Disconnect Session** / **Logout**, and click **Flush Avro Batches Now** in the inspector panel to write `.avro` segments under `data/topics/`.
2. **Compact Avro Batches into PyLance Tables**:
   ```powershell
   pixi run ml-pipeline
   ```
