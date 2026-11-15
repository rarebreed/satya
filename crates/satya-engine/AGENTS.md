# `satya-engine` Agent Coding & Architecture Rules

Activate [`satya-rust-engine`](../../.agents/skills/satya-rust-engine/SKILL.md) and [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-engine` or `docs/ttrpg/`.

1. **Shared Primitives, Static Tables, & Sign Conventions**:
   - Build on `Triad<T>`, `ContestedCheck<D>` + `TieredSaveEval`, `ConditionSource` + `ConditionImpact`, `define_coded_enum!`, `&'static [...]` `const` tables in `*_catalog.rs`, and `Index`/`IndexMut` on stat blocks (`src/primitives.rs`).
   - Preserve exact sign conventions: `Δ_int` positive = wider failure stumble penalty (`6 + Δ_int`); direct characteristic/pool modifiers negative = penalty; `Δ_ext` / `Δ_tool` positive = harder target number (`TN_P = 31 - C_P + Δ_tool`, `TN_GM = 21 - Δ_ext`).
2. **Continuous Metric Tactical Maps (`tactical_map*` & `EdgeSide`)**:
   - Spatial & grounding modules: `tactical_map.rs`, `tactical_map_geom.rs`, `tactical_map_inspect.rs`, `tactical_map_edges.rs`, `map_dressing.rs`, `barrier_ballistics.rs`, `map_interior_gen.rs`, `map_outdoor_gen.rs`, `map_sector_gen.rs`, `map_gen_residential.rs`, `map_gen_commercial.rs`, `map_gen_zoning.rs`, `map_gen_civic_retail.rs`, `map_gen_agro_industrial.rs`, `map_continuation.rs`, `map_continuation_ribbons.rs`, `map_world_graph.rs`, `map_world_transitions.rs`, `map_narrative_manifest.rs` (`SectorNarrativeManifest` + `evaluate_passive_movement_perceptions`).
   - Use `f32` meters (`_m`) and `MapPropPlacement::footprint_m()` (`PropFootprintM`) for geometry, and `EdgeSide::North` / `EdgeSide::West` only (South = `(x, y + 1, North)`, East = `(x + 1, y, West)`).
   - Vector surfaces resolve by `SurfaceFeatureKind::layer_priority()`, never insertion order: never place lower-priority surfaces (`GardenBedZone`, `CreekChannel`, `AsphaltRoad`) inside solid higher-priority polygons (`StoneWalk`, `InteriorFloor`), verify non-zero exposed area via `topmost_surface_at_m` in generator tests, and keep non-vehicle street props off `AsphaltRoad` surfaces.
3. **Bidirectional `docs/ttrpg/` <-> `satya-engine` Sync (5 Registries)**:
   - When adding, renaming, or splitting any `docs/ttrpg/` chapter, update `docs/ttrpg/SUMMARY.md`, `rules_book.rs` + `rules_book_html.rs`, `bin/build_rules_book.rs`, `src/satya_ml/build_rules_book.py` + `notebooks/satya_rules_book.py`, and `src/satya_ml/ai_gm/tools.py`, then run `cargo test --offline -p satya-engine`.
4. **Motivational Hook Compilation & Pack-Aware NPCs (`hook_weaver.rs` & `npc_generator.rs`)**:
   - Compile `PartyHookManifest` (`compile_party_motivational_hooks`) and derive `NpcMotivationalProfile` + `generate_npc_from_prompt_with_pack` (`"awakened"` default vs. `"space_opera"`) from canonical `7` bipolar Psyche traits (`1..=20`), skills, access tags, and operational heat rather than hardcoding setting-specific flavor in Rust handlers.
