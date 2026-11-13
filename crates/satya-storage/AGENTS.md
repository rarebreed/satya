# `satya-storage` Agent Coding & Architecture Rules

Activate [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-storage`.

1. **Single Source of Truth with `satya-schema` & Offline `StorageConfig::default()`**:
   - Consume `satya_schema::{topics, subjects}`, 15-byte `WireFrameHeader` (`WIRE_HEADER_LEN`), and domain events directly from `satya-schema`—never duplicate schema IDs or topic strings locally.
   - Keep `#[serde(default)]` deserialization on `StorageConfig` (`config/storage.toml`) and `AiGmConfig` (`config/ai.toml`) so `StorageConfig::default()` runs deterministically offline without external services.
2. **Offset-Tracked Embedded Iggy WAL & Snappy `.avro` Batch Invariants**:
   - Encode every published event with `SchemaRegistry::encode_framed`, append to the embedded Apache Iggy WAL (`partition_0.iggylog`), and flush into Snappy `.avro` Object Containers (`batch_{start_offset:010}_{end_offset:010}.avro`) carrying `satya.stream`, `satya.topic`, `satya.partition_id`, `satya.start_offset`, `satya.end_offset`, `satya.schema_id`, `satya.schema_version`, and `satya.schema_fingerprint_crc64` file metadata.
