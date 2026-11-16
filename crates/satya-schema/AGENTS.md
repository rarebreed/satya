# `satya-schema` Agent Coding & Architecture Rules

Activate [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-schema` or `schemas/v*/`.

1. **Immutable `v1` Avro Schema Freeze (`CRC-64-AVRO`)**:
   - **Never edit `schemas/v1/*.avsc` or the `v1` event structs in `src/events.rs`**. Every `v1` event keeps its exact field layout, `metadata: com.satya.types.EventMetadata` envelope, `satya.*` GDPR custom attributes, and `CRC-64-AVRO` canonical fingerprint.
   - Evolve wire events by adding `schemas/v2+/` schemas in `BUILTIN_SCHEMAS` (`src/registry.rs`), or evolve REST/WS/canvas DTOs via additive `#[serde(default)]` fields in `protocol.rs`, `tactical_map_dto.rs`, and `tactical_map_dressing_dto.rs`.
2. **Macros & Shared Wire Primitives**:
   - Declare `SCREAMING_SNAKE_CASE` wire enums via `define_wire_enum!` (`src/wire_enum.rs`) with `aliases`, `ALL`, `as_str`, `code`, `label`, and `from_code_str`.
   - Use `MapPropPlacement::footprint_m()` (`PropFootprintM`) for rotated continuous metric prop transforms rather than re-deriving center/half-extents/rotation in downstream crates.
   - Register builtin Avro schemas in `BUILTIN_SCHEMAS` (`src/registry.rs`).
3. **Module Responsibility Map (`crates/satya-schema/src/`)**:
   - **Wire Enums, Avro Registry & GDPR**: `wire_enum.rs` (`define_wire_enum!`), `types.rs` / `events.rs` (frozen `v1` Avro envelopes), `registry.rs` (`BUILTIN_SCHEMAS` + `CRC-64-AVRO` framing), `gdpr.rs` (PII redaction).
   - **REST/WS Session, Vault & AI Protocol DTOs**: `protocol.rs` (auth, sessions, chat/mechanics WS packets, GM docs/memories, omnisearch), `protocol_map_ai.rs` (AI GM/Player configs, session NPCs, map authoring requests), `session_persistence.rs` (`.satyasession` bundles & replay history), `character.rs` (`CharacterSheetRecord` & `.satyachar` metadata), `world_clocks_dto.rs` (`SessionWorldLedger`, faction clocks, `OperationalHeat`).
   - **Continuous Tactical Map DTOs**: `tactical_map_dto.rs` (map state, tokens, edges), `tactical_map_dressing_dto.rs` (`PropKind`, `MapPropPlacement::footprint_m()`), `tactical_map_surfaces_dto.rs` (`SurfaceFeatureKind` & vector polygons), `tactical_map_floors_dto.rs` (multi-floor stacks & portals), `tactical_map_world_dto.rs` (sector continuation & world graph).
