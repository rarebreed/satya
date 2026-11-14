---
name: satya-rust-engine
description: >-
  Domain architecture, shared primitives (Triad, ContestedCheck, ConditionImpact,
  define_coded_enum!), sign conventions (Delta_int vs Delta_ext/Delta_tool), and
  bidirectional docs/ttrpg/ synchronization workflow for crates/satya-engine.
  Activate this skill whenever working on crates/satya-engine, TTRPG mechanics,
  character creation math, combat, ballistics, or docs/ttrpg/ markdown chapters.
---

# Satya Rust Game Engine Steering Skill (`satya-rust-engine`)

Use this skill alongside [`satya-rust-workspace`](../satya-rust-workspace/SKILL.md) whenever reading, adding, or refactoring code in [`crates/satya-engine`](../../../crates/satya-engine) or TTRPG design chapters in [`docs/ttrpg/`](../../../docs/ttrpg).

---

## 1. Shared Primitives (`crates/satya-engine/src/primitives.rs`)

Always build on the shared primitives in [`crates/satya-engine/src/primitives.rs`](../../../crates/satya-engine/src/primitives.rs) instead of ad-hoc fields:
- **Tactical Triad (`Triad<T>`)**: Use `Triad<T>` (`tempo`, `control`, `force`) for allocations (`Triad<i32>`), ceiling caps (`Triad<i32>`), and limb-slash flags (`Triad<bool>`). Never pass `(att_t, att_c, att_f, def_t, def_c, def_f)` as 6 separate integers when `Triad<i32>` applies.
- **Contested Saves (`ContestedCheck<D>` & `TieredSaveEval`)**: All 1-Pair Tiered GM Resistance checks (`ContestedCascadeTaskParams::new_tiered_environment`, where $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$ vs. $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$) must use `TieredSaveEval::evaluate` + `classify_4way` and return `ContestedCheck<D>` (`#[serde(flatten)] pub effect: D` + `Deref<Target = D>`).
- **Conditions (`ConditionSource` & `ConditionImpact`)**: Implement `ConditionSource` (`fn to_condition_impact(&self) -> ConditionImpact`) on every status, fatigue, encumbrance, or environmental evaluation struct, and combine them via `ConditionImpact::combine` or `aggregate_condition_impacts`.
- **Coded Enums (`define_coded_enum!`)**: Declare domain enums with `define_coded_enum!` to generate `ALL`, `as_str`, `label`, `full_name`, `subtitle`, `Display`, and alias-aware `from_code_str` (use `satya_schema::define_wire_enum!` for `SCREAMING_SNAKE_CASE` wire enums).

---

## 2. Static `const` Catalogs & `Index` Accessors

- Store static game tables (`CANONICAL_SKILL_TREES`, `CANONICAL_ACTION_CATALOG`, `MELEE_WEAPON_CATALOG`, `RANGED_FIRE_MODES`, `ARMOR_CATALOG`, `HIT_LOCATION_TABLE`, `UNIVERSAL_TALENTS`) as `pub const` arrays/slices via declarative macros in dedicated `*_catalog.rs` modules (`skills_catalog.rs`, `lifepath_catalog.rs`, `action_catalog_stealth.rs`).
- Attach `pub const fn spec(self) -> &'static ...` directly to ID enums (`MeleeWeaponId`, `RangedFireModeId`, `ArmorId`, `HitLocationRegion`).
- Use `Index` / `IndexMut` (`chars[CharacteristicCode::FOR]`, `chars[PsycheTraitId::Ego]`, `apts[AptitudeCode::AptBio]`) for dynamic stat access.
- Call the canonical derived-stat functions in `characteristics.rs` (`compute_d_cog_score`, `compute_ooda_score`, `compute_pow_striking_power`, `compute_mmf_momentum_factor`, `compute_fatigue_threshold_score`, `compute_wound_shock_resistance_score`, `compute_social_composure_score`, `compute_general_awareness_score`).

---

## 3. Continuous Metric Tactical Maps & Geometry

- **Spatial Modules**: `tactical_map.rs` (procedural sectors, `evaluate_los_continuous`, `compute_reachable_continuous`), `tactical_map_geom.rs` (circle/OBB/segment math, continuous destination resolution), `tactical_map_inspect.rs` (`migrate_legacy_tiles_to_vector`, `all_walkable_areas_connected`, `inspect_map_point`), `tactical_map_edges.rs`, `map_dressing.rs`, `barrier_ballistics.rs`, `map_interior_gen.rs`, `map_outdoor_gen.rs`, and `map_sector_gen.rs`.
- **Metric Units (`_m` / `_cm`) & `PropFootprintM`**: Use `f32` meters (`_m`) for all raycast/OBB/circle math and `i32` centimeters (`_cm`) on wire DTOs (`GmMapUpdateRequest`, `TokenVisualSpec`, `MapPropPlacement`). Extract OBB center/half-extents/rotation directly via `p.footprint_m()` (`satya_schema::PropFootprintM`) rather than re-implementing rotation transforms.
- **`EdgeSide` Canonical Representation**: `EdgeSide` has **only** `North` and `West` variants (`South` of cell `(x, y)` is `(x, y + 1, EdgeSide::North)`, and `East` of cell `(x, y)` is `(x + 1, y, EdgeSide::West)`).
- **Priority-Sorted Vector Surfaces (Never Painter's Order)**: 2D SVG (`map_2d_art_surfaces.rs`), 3D WebGPU (`map_3d_surfaces_geom.rs`), and `topmost_surface_at_m` (`map_surface_queries.rs`) resolve strictly by `SurfaceFeatureKind::layer_priority()` (`MulchZone=14 < GardenBedZone=16 < DirtTrail=24 < DefensiveTrench=28 < GravelRoad=30 < DrainageDitch=32 < CreekChannel=34 < StoneWalk=38 < ConcreteWalk=45 < AsphaltRoad=50 < InteriorFloor=60 < BridgeDeck=70`), **never** by `Vec` push order. Never place a lower-priority surface inside a solid polygon or wider concentric centerline of a higher-priority kind; use perimeter walk ribbons or carved/disjoint floor polygons so inner features stay exposed.
- **Verify Topmost Exposure, Not Just `Vec` Membership**: Generator tests must sample `topmost_surface_at_m(&sector.surfaces, x_m, y_m)` across seeds and scales to assert every signature/non-underlay `MapSurfaceFeature` has non-zero exposed area (mere `surfaces.iter().any(...)` misses 100%-occluded features).
- **Surface-Aware Prop Placement (`OutdoorBuilder` vs. `SectorBuilder`)**: `OutdoorBuilder::try_place_cm(..., allow_on_pavement = false)` silently rejects props near `is_road_or_pavement()` or `is_water_or_channel()` (including `CreekChannel`, `DrainageDitch`, and `DefensiveTrench`) — offset rim props outside `±(channel_width_cm / 200.0 + clearance_m)` and assert expected prop presence in tests. When placing/scattering non-vehicle street props in `SectorBuilder`, check `topmost_surface_at_m` so trees, planters, benches, and crates never land on `AsphaltRoad` lanes or inside `InteriorFloor` storefronts.

---

## 4. Canonical Sign Conventions ($\Delta_{\text{int}}$ vs. $\Delta_{\text{ext}}$ / $\Delta_{\text{tool}}$ vs. Characteristic Penalties)

- **Internal Failure-Carry / Strain Modifiers ($\Delta_{\text{int}}$ in `ConditionImpact::delta_int` and `CascadeTaskParams::internal_mod`)** adjust the failure-carry stumble magnitude ($\delta_{\text{under}} = \max(0, 6 + \Delta_{\text{int}})$):
  - **Positive** (`+2` per Fatigue level, `+4` Nausea, `+2` Ego Wound) = **Penalty** (widens failure stumble).
  - **Negative** (`-2` Focused / Conviction Surge) = **Bonus** (reduces failure stumble).
- **Direct Characteristic / Pool Modifiers** (`fit_penalty`, `hlt_penalty`, `delta_mot`, `tactical_pool_mod`) adjust $C$ or $P_{\text{eff}}$ directly:
  - **Negative** (`-2`, `-6`) = **Penalty**; **Positive** (`+1`, `+2`) = **Bonus**.
- **External / Tool Modifiers ($\Delta_{\text{ext}}$, $\Delta_{\text{tool}}$)** adjust target numbers directly ($\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$ and $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$):
  - **Positive** (`+3` Darkness, `+6` High-Security Lock, `+2` Improvised Tool) = **Harder Task** (raises $\text{TN}_P$ or lowers $\text{TN}_{GM}$).
  - **Negative** (`-2` Superior Tool, `-4` Trivial Task) = **Easier Task**.

---

## 5. Bidirectional `docs/ttrpg/` <-> `satya-engine` Synchronization Checklist

Whenever adding, renaming, or splitting a `docs/ttrpg/` markdown chapter or changing a canonical formula:
1. Update [`docs/ttrpg/SUMMARY.md`](../../../docs/ttrpg/SUMMARY.md) (Table of Contents + Reciprocal Rust Engine Module Index) and ensure the chapter has a `> **Rust Engine Implementation**:` callout at the top.
2. Update `STATIC_CHAPTERS` in [`crates/satya-engine/src/rules_book.rs`](../../../crates/satya-engine/src/rules_book.rs) and `resolve_markdown_link_chapter_id` in [`crates/satya-engine/src/rules_book_html.rs`](../../../crates/satya-engine/src/rules_book_html.rs).
3. Update the relative link map in [`crates/satya-engine/src/bin/build_rules_book.rs`](../../../crates/satya-engine/src/bin/build_rules_book.rs).
4. Update `BOOK_CHAPTER_SPECS` in [`src/satya_ml/build_rules_book.py`](../../../src/satya_ml/build_rules_book.py) and `CHAPTER_SPECS` in [`notebooks/satya_rules_book.py`](../../../notebooks/satya_rules_book.py).
5. Update `_CANONICAL_RULES_FILES` in [`src/satya_ml/ai_gm/tools.py`](../../../src/satya_ml/ai_gm/tools.py).
6. Run `cargo test --offline -p satya-engine` to verify `test_rules_book_catalog_search_and_tactical_pool_envelope` (ensuring zero raw LaTeX leaks, zero `&amp;amp;` double-escaping, quoted Mermaid labels, and valid currency strings).
