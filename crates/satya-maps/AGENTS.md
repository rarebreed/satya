# `satya-maps` Agent Coding & Architecture Rules

Activate [`satya-rust-engine`](../../.agents/skills/satya-rust-engine/SKILL.md) and [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-maps`.

1. **Module Map**:
   - **Spatial Core & Grounding**: `tactical_map.rs`, `tactical_map_geom.rs`, `tactical_map_inspect.rs`, `tactical_map_edges.rs`, `tactical_map_reach.rs`, `tactical_map_spatial.rs`, `map_dressing.rs`, `map_surface_{geom,queries}.rs`, `barrier_ballistics.rs` (re-exporting `MaterialBallisticProfile` / `material_ballistic_profile` from `satya_engine::ballistics`).
   - **Procedural Indoor, Outdoor & Sector Generators**: `map_interior_{gen,topologies}.rs`, `map_home_floorplan.rs`, `map_room_furnish.rs`, `map_outdoor_{gen,natural}.rs`, `map_estate_grammars.rs`, `map_sector_{gen,scifi}.rs`, `map_urban_grammars.rs`, `map_gen_{residential,commercial,zoning,civic_retail,agro_industrial}.rs`, `map_pool_spa_gen.rs`.
   - **Continuation, World Graph & Narrative Perceptions**: `map_continuation*.rs`, `map_world_{graph,transitions}.rs`, `map_narrative_manifest.rs` (`SectorNarrativeManifest` + `evaluate_passive_movement_perceptions`).
2. **Spatial & Architectural Realism Invariants**:
   - Use `f32` meters (`_m`) and `MapPropPlacement::footprint_m()` (`satya_schema::PropFootprintM`), and `EdgeSide::North` / `EdgeSide::West` only (`South = (x, y + 1, North)`, `East = (x + 1, y, West)`).
   - Resolve vector surfaces strictly by `SurfaceFeatureKind::layer_priority()`, never push order; assert non-zero exposed area via `topmost_surface_at_m` in generator tests and keep non-vehicle street props off `AsphaltRoad`.
   - Validate solid `Wall`/`Window` backing via `WallEdgeIndex` for wall-mounted fixtures; carve `Role::Hall` spines before room bays; omit foyers on compact dwellings; enforce whole-home/compound `WasherDryer <= 1`, `Kitchen <= 1` (`generate_annex_wing_in_lot`), and bathroom caps.
3. **Scoped Verification**:
   - Run `pixi run test-maps` and `pixi run check-wasm` when modifying spatial geometry or generators.
