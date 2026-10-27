# Satya WebRTC, Chat & Event Streaming Platform — Walkthrough

## Summary of Changes

We have built the initial full-stack **WebRTC Audio/Video, Chat & Event Streaming application** in **Rust, WebAssembly, and Leptos**, managed via **Pixi** (`conda-forge` Rust + Python toolchain) and designed for downstream AI/ML training via **Apache Iggy**, **Apache Avro**, and **PyLance (`lance`)**.

### 1. Project & Workspace Configuration
- [Cargo.toml](Cargo.toml): Root Cargo workspace organizing four specialized crates (`satya-schema`, `satya-storage`, `satya-signaling`, `satya-web`).
- [pixi.toml](pixi.toml): Manages both the `rust` (`>=1.85`) and `python` (`>=3.14.7`) toolchains plus `pylance`, `fastavro`, `pyarrow`, and `pytest`, with tasks for building, testing, serving, and running the Avro-to-PyLance ML compaction pipeline.
- [config/storage.toml](config/storage.toml): Declarative configuration for Apache Iggy stream connection (`127.0.0.1:8090` with automatic local broker fallback), micro-batch thresholds (`batch_max_messages`, `batch_max_interval_ms`), and per-topic `local` vs. `s3` storage sinks.

### 2. Versioned Nested Avro Schemas & Evolution Engine (`satya-schema`)
- **Avro Schema Files**:
  - [schemas/v1/schema_registry_event.avsc](schemas/v1/schema_registry_event.avsc)
  - [schemas/v1/session_lifecycle_event.avsc](schemas/v1/session_lifecycle_event.avsc)
  - [schemas/v1/webrtc_telemetry_event.avsc](schemas/v1/webrtc_telemetry_event.avsc)
  - [schemas/v1/chat_message_event.avsc](schemas/v1/chat_message_event.avsc) and [schemas/v2/chat_message_event.avsc](schemas/v2/chat_message_event.avsc)
  - [schemas/v1/interaction_dynamics_event.avsc](schemas/v1/interaction_dynamics_event.avsc)
  - [schemas/v1/ttrpg_mechanics_event.avsc](schemas/v1/ttrpg_mechanics_event.avsc)
- [crates/satya-schema/src/lib.rs](crates/satya-schema/src/lib.rs):
  - Defines the nested `Participant` and `EventMetadata` structs used across all events.
  - Implements `SchemaRegistry`, 64-bit `crc64_avro_fingerprint`, and `WireFrameHeader` (15-byte header tracking `schema_id`, `schema_version`, and `fingerprint_crc64` for every message).

### 3. Apache Iggy & Offset-Tracked Micro-Batch Storage (`satya-storage`)
- [crates/satya-storage/src/lib.rs](crates/satya-storage/src/lib.rs):
  - Implements `EventLogRouter`, which streams framed Avro events to Apache Iggy and buffers events per topic up to `batch_max_messages`.
  - Flushes Snappy-compressed Avro Object Container segments (`batch_{start_offset:010}_{end_offset:010}.avro`) with embedded `satya.start_offset`, `satya.end_offset`, `satya.schema_version`, and `satya.schema_fingerprint_crc64` metadata to either `LocalFileSystem` or `AmazonS3` via `object_store`.
- [src/satya_ml/lance_ingest.py](src/satya_ml/lance_ingest.py) & [src/satya_ml/inspect_avro.py](src/satya_ml/inspect_avro.py):
  - Incremental compaction script (`pixi run compact-to-lance`) that reads flushed `.avro` batches, checks the high-water offset in `_watermarks.json`, converts nested records into a `pyarrow.Table`, and appends to `data/lance/<topic>.lance`.

### 4. Authentication, Session Password & WebRTC Signaling Server (`satya-signaling` & `satya-web`)
- [crates/satya-signaling/src/auth.rs](crates/satya-signaling/src/auth.rs):
  - Argon2id-backed `AuthAndSessionManager` managing persistent user accounts (`user_id`, `username`, `password_hash`) and password-protected sessions (`session_id` + `session_password`).
- [crates/satya-signaling/src/lib.rs](crates/satya-signaling/src/lib.rs):
  - WebSocket signaling & message router enforcing public broadcast vs. private recipient isolation (`PUBLIC`, `PRIVATE`, `GM_ONLY`, `HOST_ONLY`), WebRTC SDP/ICE relay, Satya d20 task resolution & combat action economy, and autonomous AI GM narration.
- [crates/satya-web/src/lib.rs](crates/satya-web/src/lib.rs):
  - Full-stack Leptos UI for user login/registration, password-protected session creation/join, WebRTC audio/video + DataChannel mesh, Web Audio API RMS prosody telemetry, targeted private/public chat, and live Apache Iggy topic/schema inspection.

---

## Validation & Test Results

All workspace unit and integration tests were executed via `pixi run test-rust`:

- **`satya-schema` (3 passed)**:
  - `test_all_builtin_schemas_register_and_fingerprint`: Verified all 7 schemas (`v1` across 6 topics + `v2` evolved `ChatMessageEvent`) parse and generate deterministic 64-bit `CRC-64-AVRO` fingerprints.
  - `test_nested_participant_and_schema_evolution_v1_to_v2`: Verified nested `Participant` serialization in `ChatMessageEvent` (`sender` and `recipients`) and verified live schema evolution (decoding a `v1` payload using the `v2` reader schema with default `sentiment_score` and `detected_intent` resolution).
  - `test_interaction_dynamics_and_ttrpg_mechanics_serialization`: Verified `InteractionDynamicsEvent` (with nested `speaker` and `overlapped_participants`) and `TtrpgMechanicsEvent` (Satya d20 pool vs. Attribute TN and combat stance action count cost).
- **`satya-storage` (1 passed)**:
  - `test_micro_batch_flush_and_avro_header_watermarks`: Verified automatic batch flushing to `batch_0000000000_0000000002.avro` and confirmed the embedded `satya.start_offset`, `satya.end_offset`, and `satya.schema_version` header metadata and nested `Participant` records round-trip cleanly.
- **`satya-signaling` (2 passed)**:
  - `test_user_auth_session_password_and_private_message_isolation`: Verified Argon2id user registration/login (`user_id` persistence), password-protected session creation, rejection of invalid session passwords, and private message recipient isolation.
  - `test_axum_http_endpoints_and_leptos_render`: Verified the Leptos UI renders at `GET /` and `/api/storage/metrics` reports all 6 topics and their registered schema fingerprints.

---

## Pixi Task Reference

| Category | Pixi Command | Description |
| :--- | :--- | :--- |
| **Build & Quality** | `pixi run check` | Type-check all 4 Rust crates (`cargo check --workspace`) |
| | `pixi run fmt` | Format all Rust code (`cargo fmt --all`) |
| | `pixi run lint` | Run Clippy lints across all Rust targets (`cargo clippy --workspace --all-targets`) |
| | `pixi run build` / `pixi run build-release` | Build debug or optimized release binaries |
| **Granular & Polyglot Tests** | `pixi run test-schema` | Run Avro schema, CRC-64-AVRO fingerprint, and `v1 -> v2` evolution tests |
| | `pixi run test-storage` | Run Apache Iggy WAL and offset-tracked `.avro` micro-batching tests |
| | `pixi run test-signaling` | Run Argon2id user auth, session password, and private message routing tests |
| | `pixi run test-rust` | Run all Rust unit & integration tests |
| | `pixi run test-py` | Run Python unit tests (`pytest`) |
| | `pixi run test` | Polyglot DAG task (`depends-on = ["test-rust", "test-py"]`) running both Rust and Python test suites |
| **Server Runtime** | `pixi run serve` | Launch the Satya WebRTC, Chat & Apache Iggy Event Server on `http://127.0.0.1:3000` |
| | `pixi run serve-release` | Launch the optimized release server |
| **ML & Data Pipeline** | `pixi run inspect-schemas` | Validate and list all versioned `.avsc` schemas and fields using `fastavro` |
| | `pixi run compact-to-lance` | Incrementally compact flushed `.avro` topic batches into `.lance` AI datasets |
| | `pixi run inspect-topics` | Inspect flushed `.avro` batch offset watermarks and `.lance` table row counts |
| | `pixi run ml-pipeline` | Pipeline DAG task (`depends-on = ["compact-to-lance", "inspect-topics"]`) |

---

## Running the Application Locally

1. **Start the Server**:
   ```powershell
   pixi run serve
   ```
2. **Connect Browser Clients**:
   - Open two or more browser tabs at `http://127.0.0.1:3000`.
   - Use the **Quick Fill Demo Accounts** (or register new accounts) to log in as `gm_sean` in Tab 1 (click **Host: Create Session** with Session ID `harsh-reality-ep1` and password `orbital-drop-7`) and `sara_valkyrie` in Tab 2 (click **Join Existing Session** with the same Session ID and password).
   - Enable Camera/Mic, send Public or Private targeted messages, roll Satya d20 tasks, and click **Flush Avro Batches Now** in the inspector panel to write `.avro` segments under `data/topics/`.
3. **Compact Avro Batches into PyLance Tables**:
   ```powershell
   pixi run ml-pipeline
   ```
