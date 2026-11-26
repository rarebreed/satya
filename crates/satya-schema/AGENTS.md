# `satya-schema` Agent Coding & Architecture Rules

Activate [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-schema` or `schemas/v*/`.

1. **Avro & PyLance Schema Evolution (`CRC-64-AVRO` & Compatibility)**:
   - Apache Avro (for Apache Iggy WAL/batches) and PyLance (for AI/ML datasets) exist specifically to support **schema evolution**. Evolve schemas cleanly when needed: register new `schemas/v2+/` `.avsc` versions or update schemas with Avro defaults/aliases in `BUILTIN_SCHEMAS` (`src/registry.rs`), and keep historical `v1` compatibility tests + `CRC-64-AVRO` framing passing via `pixi run test-schema`.
   - Keep `crates/satya-schema` strictly free of `pyo3` dependencies so `satya-web` (`wasm32-unknown-unknown`), `satya-signaling`, `satya-storage`, and `satya-py` all share it cleanly.
   - Centralize shared wire DTOs (e.g., `GmSessionLogEntry`, `GmTrajectoryReportRequest`, `MoveMapTokenRequest`) and role-filtering methods (e.g., `GmMapUpdateRequest::filtered_for_role(SessionRole)`) here rather than duplicating DTO structs or filtering rules in downstream crates.
2. **Macros & Shared Wire Primitives**:
   - Declare `SCREAMING_SNAKE_CASE` wire enums via `define_wire_enum!` (`src/wire_enum.rs`) with `aliases` (for legacy string migrations such as `MaterialClass::ConcreteBlock` aliasing `"CONCRETE"` or `CombatStance`), `ALL`, `as_str`, `code`, `label`, and `from_code_str`.
   - When adding `PropKind` variants in `tactical_map_dressing_dto.rs`, update `[count = N]` and `PropKind::ALL.len()`, match `serde` `SCREAMING_SNAKE_CASE` on both digit and acronym boundaries (e.g., `Printer3dStation` → `"PRINTER3D_STATION"`, `SciFiCargoPod` → `"SCI_FI_CARGO_POD"`), and append matching `PropSpec` entries in identical enum order to `PROP_CATALOG` (`crates/satya-maps/src/map_dressing.rs`).
   - Use `MapPropPlacement::footprint_m()` (`PropFootprintM`) for rotated continuous metric prop transforms rather than re-deriving center/half-extents/rotation in downstream crates.
   - Register builtin Avro schemas in `BUILTIN_SCHEMAS` (`src/registry.rs`).
3. **Module Responsibility Map (`crates/satya-schema/src/`)**:
   - **Wire Enums, Avro Registry & GDPR**: `wire_enum.rs` (`define_wire_enum!`), `types.rs` / `events.rs` (`Avro` envelopes + `metadata: EventMetadata` + `satya.*` GDPR attributes), `registry.rs` (`BUILTIN_SCHEMAS` + `CRC-64-AVRO` framing), `gdpr.rs` (PII redaction).
   - **REST/WS Session, Vault & AI Protocol DTOs**: `protocol.rs` (auth, sessions, chat/mechanics WS packets, GM docs/memories, omnisearch), `protocol_map_ai.rs` (AI GM/Player configs, session NPCs, map authoring requests, telemetry/trajectory DTOs), `session_persistence.rs` (`.satyasession` bundles & replay history), `character.rs` (`CharacterSheetRecord` & `.satyachar` metadata), `world_clocks_dto.rs` (`SessionWorldLedger`, faction clocks, `OperationalHeat`).
   - **Continuous Tactical Map DTOs**: `tactical_map_dto.rs` (map state, tokens, edges), `tactical_map_dressing_dto.rs` (`PropKind`, `MapPropPlacement::footprint_m()`), `tactical_map_surfaces_dto.rs` (`SurfaceFeatureKind` & vector polygons), `tactical_map_floors_dto.rs` (multi-floor stacks & portals), `tactical_map_world_dto.rs` (sector continuation & world graph).
