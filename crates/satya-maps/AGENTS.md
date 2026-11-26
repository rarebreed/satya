# `satya-maps` Agent Coding & Architecture Rules

Activate [`satya-maps`](../../.agents/skills/satya-maps/SKILL.md) and [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-maps`.

1. **Module Map**:
   - **Spatial Core & Grounding**: `tactical_map.rs`, `tactical_map_geom.rs`, `tactical_map_inspect.rs`, `tactical_map_edges.rs`, `tactical_map_reach.rs`, `tactical_map_spatial.rs`, `map_dressing.rs`, `map_surface_{geom,queries}.rs`, `barrier_ballistics.rs` (re-exporting `MaterialBallisticProfile` / `material_ballistic_profile` from `satya_engine::ballistics`).
   - **Procedural Indoor, Outdoor & Sector Generators**: `map_interior_{gen,topologies}.rs`, `map_home_floorplan.rs`, `map_room_furnish.rs`, `map_outdoor_{gen,natural}.rs`, `map_estate_grammars.rs`, `map_sector_{gen,scifi}.rs`, `map_urban_grammars.rs`, `map_gen_{residential,commercial,zoning,civic_retail,agro_industrial,awakened}.rs`, `map_pool_spa_gen.rs`.
   - **Continuation, World Graph & Narrative Perceptions**: `map_continuation*.rs`, `map_world_{graph,transitions}.rs`, `map_narrative_manifest.rs` (`SectorNarrativeManifest` + `evaluate_passive_movement_perceptions`).
2. **Spatial & Architectural Realism Invariants**:
   - Use `f32` meters (`_m`) and `MapPropPlacement::footprint_m()` (`satya_schema::PropFootprintM`), and `EdgeSide::North` / `EdgeSide::West` only (`South = (x, y + 1, North)`, `East = (x + 1, y, West)`).
   - Resolve vector surfaces strictly by `SurfaceFeatureKind::layer_priority()`, never push order; carve strictly disjoint surface zones, assert non-zero exposed area via `topmost_surface_at_m` in generator tests, and keep non-vehicle street props off `AsphaltRoad` (`Car | UtilityVan | JerseyBarrier` only, except `DockingBay`'s indoor loading apron).
   - Validate solid `Wall`/`Window` backing via `WallEdgeIndex` for wall-mounted fixtures; carve `Role::Hall` spines before room bays; omit foyers on compact dwellings; enforce whole-home/compound `WasherDryer <= 1`, `Kitchen <= 1` (`generate_annex_wing_in_lot`), and bathroom caps.
   - **Scale-Aware Density & Thematic Authenticity**: Never leave `>= 24×18` / `36×26` / `48×36` sectors with small fixed prop lists or empty partitioned bays; loop racks/crops across both axes, furnish every partitioned room/alcove, inset multi-cell props clear of partitions (`x <= wall_x - w_cells`), keep exterior props clear of `h/2` spawn pads, verify `B1`/`2F` via `floor_lot_connected`, and add dedicated `PropKind`s rather than mismatched stand-ins.
   - **Prop Alignment Discipline**: Keep props in well-maintained, high-traffic commercial, retail, civic, corporate, clinical, and formal conservatory spaces strictly orthogonal on **N–S / E–W axes (`0° / 90° / 180° / 270°`, `rot_jitter = 0`)**. Reserve slight/arbitrary off-axis rotations strictly for rural/natural areas, unmaintained or low-economic/chaotic urban/derelict zones, organic foliage/debris, tabletop `DeskClutter`, and vehicles aligned to curved roads.
3. **Scoped Verification**:
   - Run `pixi run test-maps` and `pixi run check-wasm` when modifying spatial geometry or generators.
