# `satya-schema` Agent Coding & Architecture Rules

When reading, adding, or refactoring code in `crates/satya-schema` or Avro schemas in `schemas/v*/`, activate the [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) skill and follow these architectural rules:

1. **High-Signal Docstrings (No `#![deny(missing_docs)]`)**:
   - Never enable `#![deny(missing_docs)]` in `src/lib.rs`.
   - Include a concise `//!` module header at the top of each `.rs` file and `///` docstrings on top-level `pub struct`, `pub enum`, and `pub fn` entrypoints.
   - Omit repetitive `///` comments on self-explanatory struct fields and enum variants unless a field has a non-obvious scale, wire encoding, or sign convention (`_int` vs. `_ext`).
   - Encode physical/time units directly in field names (`_ms`, `_us`, `_bps`, `_m`, `_kg`, `_bytes`, `_hex`, `_base64`).

2. **Declarative Schema & Enum Registration (`define_schema_enum!` & `BUILTIN_SCHEMAS`)**:
   - Define builtin Avro schemas as a single `BUILTIN_SCHEMAS` table in `src/registry.rs` (`(schema_id, subject, topic, version, include_str!(...))`) rather than repetitive `registry.register(...)` calls.
   - Use compact declarative macros for schema enums that need string conversion helpers (`as_str`, `from_code_str`).

3. **Avro `.avsc` <-> Rust Serde Parity & GDPR Annotations**:
   - Every event schema in `schemas/v*/` must include the standard `metadata: com.satya.types.EventMetadata` envelope and `satya.*` GDPR custom attributes (`satya.subject_role`, `satya.gdpr_subject_key`, `satya.pii_owner`, `satya.erasure_action`, `satya.dsar_third_party_mask`) while preserving exact `CRC-64-AVRO` canonical fingerprints.
   - Keep DTOs and domain events logically grouped across `types.rs`, `events.rs`, `character.rs`, `protocol.rs`, `registry.rs`, and `gdpr.rs` with flat `pub use` re-exports in `src/lib.rs`.
