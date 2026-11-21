# `satya-engine` Agent Coding & Architecture Rules

Activate [`satya-rust-engine`](../../.agents/skills/satya-rust-engine/SKILL.md) and [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-engine` or `docs/ttrpg/`.

1. **Shared Primitives, Static Tables, & Sign Conventions**:
   - Build on `Triad<T>`, `ContestedCheck<D>` + `TieredSaveEval`, `ConditionSource` + `ConditionImpact`, `define_coded_enum!`, `&'static [...]` `const` tables in `*_catalog.rs`, and `Index`/`IndexMut` on stat blocks (`src/primitives.rs`).
   - Preserve exact sign conventions: `Δ_int` positive = wider failure stumble penalty (`6 + Δ_int`); direct characteristic/pool modifiers negative = penalty; `Δ_ext` / `Δ_tool` positive = harder target number (`TN_P = 31 - C_P + Δ_tool`, `TN_GM = 21 - Δ_ext`).
2. **Boundary with Extracted Sibling Crates (`satya-rules-book`, `satya-maps`, `satya-campaign`)**:
   - `satya-engine` must stay **setting-agnostic** and **acyclic** (depends only on `satya-schema`, `serde`, `serde_json`).
   - Do **not** depend on `satya-maps`, `satya-campaign`, or `satya-rules-book`. Keep `MaterialBallisticProfile` / `MATERIAL_BALLISTIC_PROFILES` (`src/ballistics.rs`), `SyntheticMetabolicMode` (`src/robotics_and_ai_design.rs`), and `LifepathBuildConfig.faction_archetype: Option<String>` (`src/lifepath_economy.rs`) setting- and geometry-free.
   - Tactical map geometry/generators live in [`crates/satya-maps`](../satya-maps/AGENTS.md); *Awakened* 2035 substrates/arcs/factions/hooks/NPCs/clocks live in [`crates/satya-campaign`](../satya-campaign/AGENTS.md); embedded `docs/ttrpg/` catalog/HTML rendering lives in [`crates/satya-rules-book`](../satya-rules-book/AGENTS.md).
3. **Bidirectional `docs/ttrpg/` <-> Rust Sync (5 Registries)**:
   - When adding, renaming, or splitting any `docs/ttrpg/` chapter, update `docs/ttrpg/SUMMARY.md`, `crates/satya-rules-book/src/lib.rs` + `rules_book_html.rs`, `crates/satya-rules-book/src/bin/build_rules_book.rs`, `src/satya_ml/build_rules_book.py` + `notebooks/satya_rules_book.py`, and `src/satya_ml/ai_gm/tools.py`, then run `pixi run test-rules-book` (~1s); treat `pixi run build-book` as optional unless regenerating `docs/book/` HTML is requested or needed for changed chapters/links.
