# `satya-schema` Agent Coding & Architecture Rules

Activate [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-schema` or `schemas/v*/`.

1. **Immutable `v1` Avro Schema Freeze (`CRC-64-AVRO`)**:
   - **Never edit `schemas/v1/*.avsc` or the `v1` event structs in `src/events.rs`**. Every `v1` event keeps its exact field layout, `metadata: com.satya.types.EventMetadata` envelope, `satya.*` GDPR custom attributes, and `CRC-64-AVRO` canonical fingerprint.
   - Evolve wire events by adding `schemas/v2+/` schemas in `BUILTIN_SCHEMAS` (`src/registry.rs`), or evolve REST/WS/canvas DTOs via additive `#[serde(default)]` fields in `protocol.rs`, `tactical_map_dto.rs`, and `tactical_map_dressing_dto.rs`.
2. **Macros & Shared Wire Primitives**:
   - Declare `SCREAMING_SNAKE_CASE` wire enums via `define_wire_enum!` (`src/wire_enum.rs`) with `aliases`, `ALL`, `as_str`, `code`, `label`, and `from_code_str`.
   - Use `MapPropPlacement::footprint_m()` (`PropFootprintM`) for rotated continuous metric prop transforms rather than re-deriving center/half-extents/rotation in downstream crates.
   - Register builtin Avro schemas in `BUILTIN_SCHEMAS` (`src/registry.rs`).
3. **Complete Module Inventory (`crates/satya-schema/src/`)**:
   - `wire_enum.rs`, `types.rs`, `events.rs`, `protocol.rs`, `protocol_map_ai.rs`, `tactical_map_dto.rs`, `tactical_map_dressing_dto.rs`, `tactical_map_surfaces_dto.rs`, `tactical_map_floors_dto.rs`, `character.rs`, `session_persistence.rs`, `registry.rs`, `gdpr.rs`.
