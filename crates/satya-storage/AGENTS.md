# `satya-storage` Agent Coding & Architecture Rules

When reading, adding, or refactoring code in `crates/satya-storage`, activate the [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) skill and follow these architectural rules (full guide in [`docs/technical/rust-workspace-steering.md`](../../docs/technical/rust-workspace-steering.md)):

1. **Pragmatic File Size Ceiling (`<= 45 KB`) & Companion Modules**:
   - Keep `.rs` files in `crates/satya-storage/src/` under `45 KB` (`< 46,080` bytes) so each file fits in a single agent `read_file` call without truncation.
   - When `src/lib.rs` grows large, split configuration structs (`StorageConfig`, `AuthConfig`, `AiGmConfig`, `IggyConfig`), WAL/batch router (`EventLogRouter`), or test suites (`src/tests.rs`) into companion modules and preserve flat `pub use companion::*;` re-exports in `src/lib.rs`.

2. **Single Source of Truth with `satya-schema` & `config/*.toml`**:
   - Consume topic/subject constants (`satya_schema::topics`, `satya_schema::subjects`), 15-byte wire framing (`WireFrameHeader`, `WIRE_HEADER_LEN`), and domain events directly from `satya-schema`. Never redefine schema IDs or topic strings locally.
   - Maintain backward-compatible `#[serde(default)]` deserialization on `StorageConfig` (`config/storage.toml`) and `AiGmConfig` (`config/ai.toml`) so unit and integration tests using `StorageConfig::default()` run deterministically offline without external services.

3. **Offset-Tracked Avro Container & Embedded Iggy WAL Invariants**:
   - Every published event must be encoded via `SchemaRegistry::encode_framed`, appended to the embedded Apache Iggy WAL (`partition_0.iggylog`), and flushed into Snappy-compressed `.avro` Object Container files (`batch_{start_offset:010}_{end_offset:010}.avro`) carrying `satya.stream`, `satya.topic`, `satya.partition_id`, `satya.start_offset`, `satya.end_offset`, `satya.schema_id`, `satya.schema_version`, and `satya.schema_fingerprint_crc64` user metadata.

4. **High-Signal Docstrings & Unit Suffixes (No `#![deny(missing_docs)]`)**:
   - Never enable `#![deny(missing_docs)]`.
   - Include a concise `//!` module header and `///` docstrings on public entrypoints (`EventLogRouter`, `StorageConfig`, `TopicMetricsSnapshot`), and encode units directly in field and parameter names (`_ms`, `_secs`, `_bytes`, `_hex`, `_count`, `_offset`).
