# `satya-engine` Agent Coding & Architecture Rules

Activate [`satya-rust-engine`](../../.agents/skills/satya-rust-engine/SKILL.md) and [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-engine` or `docs/ttrpg/`.

1. **Shared Primitives, Static Tables, & Sign Conventions**:
   - Build on `Triad<T>`, `ContestedCheck<D>` + `TieredSaveEval`, `ConditionSource` + `ConditionImpact`, `define_coded_enum!`, `&'static [...]` `const` tables in `*_catalog.rs`, and `Index`/`IndexMut` on stat blocks (`src/primitives.rs`).
   - Preserve exact sign conventions: `Δ_int` positive = wider failure stumble penalty (`6 + Δ_int`); direct characteristic/pool modifiers negative = penalty; `Δ_ext` / `Δ_tool` positive = harder target number (`TN_P = 31 - C_P + Δ_tool`, `TN_GM = 21 - Δ_ext`).
2. **Continuous Metric Tactical Maps (`tactical_map*` & `EdgeSide`)**:
   - Spatial modules: `tactical_map.rs`, `tactical_map_geom.rs`, `tactical_map_inspect.rs`, `tactical_map_edges.rs`, `map_dressing.rs`, `barrier_ballistics.rs`, `map_interior_gen.rs`, `map_outdoor_gen.rs`, `map_sector_gen.rs`.
   - Use `f32` meters (`_m`) and `MapPropPlacement::footprint_m()` (`PropFootprintM`) for geometry, and `EdgeSide::North` / `EdgeSide::West` only (South = `(x, y + 1, North)`, East = `(x + 1, y, West)`).
3. **Bidirectional `docs/ttrpg/` <-> `satya-engine` Sync (5 Registries)**:
   - When adding, renaming, or splitting any `docs/ttrpg/` chapter, update `docs/ttrpg/SUMMARY.md`, `rules_book.rs` + `rules_book_html.rs`, `bin/build_rules_book.rs`, `src/satya_ml/build_rules_book.py` + `notebooks/satya_rules_book.py`, and `src/satya_ml/ai_gm/tools.py`, then run `cargo test --offline -p satya-engine`.
