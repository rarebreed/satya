---
name: satya-rust-engine
description: >-
  Domain architecture, shared primitives (Triad, ContestedCheck, ConditionImpact,
  define_coded_enum!), sign conventions (Delta_int vs Delta_ext/Delta_tool), and
  bidirectional docs/ttrpg/ synchronization workflow for crates/satya-engine.
  Activate this skill whenever working on crates/satya-engine, TTRPG mechanics,
  character creation math, combat, ballistics, or docs/ttrpg/ markdown chapters.
---

# Satya Rust Game Engine, Maps, Campaign & Rules Book Skill (`satya-rust-engine`)

Use this skill alongside [`satya-rust-workspace`](../satya-rust-workspace/SKILL.md) whenever reading, adding, or refactoring code in [`crates/satya-engine`](../../../crates/satya-engine), [`crates/satya-maps`](../../../crates/satya-maps), [`crates/satya-campaign`](../../../crates/satya-campaign), [`crates/satya-rules-book`](../../../crates/satya-rules-book), or TTRPG design chapters in [`docs/ttrpg/`](../../../docs/ttrpg).

---

## 1. Shared Primitives (`crates/satya-engine/src/primitives.rs`)

Always build on the shared primitives in [`crates/satya-engine/src/primitives.rs`](../../../crates/satya-engine/src/primitives.rs) instead of ad-hoc fields:
- **Tactical Triad (`Triad<T>`)**: Use `Triad<T>` (`tempo`, `control`, `force`) for allocations (`Triad<i32>`), ceiling caps (`Triad<i32>`), and limb-slash flags (`Triad<bool>`). Never pass `(att_t, att_c, att_f, def_t, def_c, def_f)` as 6 separate integers when `Triad<i32>` applies.
- **Contested Saves (`ContestedCheck<D>` & `TieredSaveEval`)**: All 1-Pair Tiered GM Resistance checks (`ContestedCascadeTaskParams::new_tiered_environment`, where $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$ vs. $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$) must use `TieredSaveEval::evaluate` + `classify_4way` and return `ContestedCheck<D>` (`#[serde(flatten)] pub effect: D` + `Deref<Target = D>`).
- **Conditions (`ConditionSource` & `ConditionImpact`)**: Implement `ConditionSource` (`fn to_condition_impact(&self) -> ConditionImpact`) on every status, fatigue, encumbrance, or environmental evaluation struct, and combine them via `ConditionImpact::combine` or `aggregate_condition_impacts`.
- **Coded Enums (`define_coded_enum!`)**: Declare every domain enum exposed over FFI/JSON/CLI (including movement gaits and traction tiers) with `define_coded_enum!` to generate `ALL`, `as_str`, `label`, `full_name`, `subtitle`, `Display`, and alias-aware `from_code_str` (use `satya_schema::define_wire_enum!` for `SCREAMING_SNAKE_CASE` wire enums) so downstream crates never hand-write ad-hoc string matchers.

---

## 2. Static `const` Catalogs & `Index` Accessors

- Store static game tables (`CANONICAL_SKILL_TREES`, `CANONICAL_ACTION_CATALOG`, `MELEE_WEAPON_CATALOG`, `RANGED_FIRE_MODES`, `ARMOR_CATALOG`, `HIT_LOCATION_TABLE`, `UNIVERSAL_TALENTS`) as `pub const` arrays/slices via declarative macros in dedicated `*_catalog.rs` modules (`skills_catalog.rs`, `lifepath_catalog.rs`, `action_catalog_stealth.rs`).
- Attach `pub const fn spec(self) -> &'static ...` directly to ID enums (`MeleeWeaponId`, `RangedFireModeId`, `ArmorId`, `HitLocationRegion`).
- Use `Index` / `IndexMut` (`chars[CharacteristicCode::FOR]`, `chars[PsycheTraitId::Ego]`, `apts[AptitudeCode::AptBio]`) for dynamic stat access.
- Call the canonical derived-stat functions in `characteristics.rs` (`compute_d_cog_score`, `compute_ooda_score`, `compute_pow_striking_power`, `compute_mmf_momentum_factor`, `compute_fatigue_threshold_score`, `compute_wound_shock_resistance_score`, `compute_social_composure_score`, `compute_general_awareness_score`).

---

## 3. Continuous Metric Tactical Maps & Geometry (`crates/satya-maps`)

- **Spatial Core**: `tactical_map.rs` (procedural sectors, `evaluate_los_continuous`, `compute_reachable_continuous`), `tactical_map_{geom,inspect,edges,reach,spatial}.rs`, `map_dressing.rs`, `map_surface_{geom,queries}.rs`, and `barrier_ballistics.rs` (all in [`crates/satya-maps`](../../../crates/satya-maps)).
- **Procedural Generators**: `map_interior_{gen,topologies}.rs`, `map_home_floorplan.rs`, `map_room_furnish.rs`, `map_outdoor_{gen,natural}.rs`, `map_estate_grammars.rs`, `map_sector_{gen,scifi}.rs`, `map_urban_grammars.rs`, and `map_gen_{residential,commercial,zoning,civic_retail,agro_industrial}.rs`.
- **Continuation & World Graph**: `map_continuation*.rs`, `map_world_{graph,transitions}.rs`, and `map_narrative_manifest.rs`.
- **Metric Units (`_m` / `_cm`) & `PropFootprintM`**: Use `f32` meters (`_m`) for all raycast/OBB/circle math and `i32` centimeters (`_cm`) on wire DTOs (`GmMapUpdateRequest`, `TokenVisualSpec`, `MapPropPlacement`). Extract OBB center/half-extents/rotation directly via `p.footprint_m()` (`satya_schema::PropFootprintM`) rather than re-implementing rotation transforms.
- **`EdgeSide` Canonical Representation**: `EdgeSide` has **only** `North` and `West` variants (`South` of cell `(x, y)` is `(x, y + 1, EdgeSide::North)`, and `East` of cell `(x, y)` is `(x + 1, y, EdgeSide::West)`).
- **Compass Bearing & Ray-Vertex Convention**: Surveillance cones and facing angles in `satya-maps` follow map compass degrees (`0° = North/-y`, `90° = East/+x`, `180° = South/+y`, `270° = West/-x`). Aim LoS/surveillance ray tests through cell centers (e.g., `(x + 0.5, y + 0.5)`) rather than exact integer edge endpoints (`y = 5.0`) to avoid boundary-vertex grazing.
- **Priority-Sorted Vector Surfaces (Never Painter's Order)**: 2D SVG (`map_2d_art_surfaces.rs`), 3D WebGPU (`map_3d_surfaces_geom.rs`), and `topmost_surface_at_m` (`map_surface_queries.rs`) resolve strictly by `SurfaceFeatureKind::layer_priority()` (`LawnGrassZone=12 < MulchZone=14 < GardenBedZone=16 < DirtTrail=24 < DefensiveTrench=28 < GravelRoad=30 < DrainageDitch=32 < CreekChannel=34 < StoneWalk=38 < BrickPaverWalk=42 < ConcreteWalk=45 < AsphaltRoad=50 < WoodDeck=56 < PoolWater=58 < InteriorFloor=60 < StainedConcreteFloor=61 < CeramicTileFloor=62 < MarbleFloor=63 < CarpetFloor=64 < ClinicalVinylFloor=65 < LinoleumCheckFloor=66 < TerrazzoFloor=67 < SteelPlateFloor=68 < BridgeDeck=74`), **never** by `Vec` push order. Never place a lower-priority surface inside a solid polygon or wider concentric centerline of a higher-priority kind (e.g., carve disjoint `ClinicalVinylFloor` strips around `CeramicTileFloor` en-suite bathrooms instead of painting one solid overlay rectangle).
- **Verify Topmost Exposure, Not Just `Vec` Membership**: Generator tests must sample `topmost_surface_at_m(&sector.surfaces, x_m, y_m)` across seeds and scales to assert every signature/non-underlay `MapSurfaceFeature` has non-zero exposed area (mere `surfaces.iter().any(...)` misses 100%-occluded features).
- **Surface-Aware Prop Placement (`OutdoorBuilder` vs. `SectorBuilder`)**: `OutdoorBuilder::try_place_cm(..., allow_on_pavement = false)` silently rejects props near `is_road_or_pavement()` or `is_water_or_channel()` (including `CreekChannel`, `DrainageDitch`, and `DefensiveTrench`) — offset rim props outside `±(channel_width_cm / 200.0 + clearance_m)` and assert expected prop presence in tests. When placing/scattering non-vehicle street props in `SectorBuilder`, check `topmost_surface_at_m` so trees, planters, benches, and crates never land on `AsphaltRoad` lanes or inside `InteriorFloor` storefronts.
- **Wall-Flush Props vs. Ballistics & Visual-Only Bypass**: Wall slicing checks (`prop_slices_wall_segment` in `tactical_map_geom.rs`) inset footprints by `WALL_SKIN_EPS_M` (`0.03m`) so flush wall-backed props are permitted, while ballistic `intersect_segment_prop` remains exact. Always route gameplay obstruction through `effective_obstruction(p)` so `p.decorative == true` ("Visual Only") props never block movement, cover, or LoS. Multi-floor validation (`floor_lot_connected` in `map_gen_residential.rs`) and cross-floor token moves must check `PropKind::is_vertical_portal()` (`Stairwell`, `ElevatorCar`, `ServiceElevator`).
- **Wall-Edge-Backed Indoor Furnishing (`WallEdgeIndex`)**: Never infer walls solely from `Room` bounding coordinates. Whenever open-concept passages or doorways delete or pierce `Room` edges, `Furnisher::reserve_doors(&edges)` must populate `WallEdgeIndex`, and wall-dependent fixtures (`KitchenCounter`, `Stove`, `Fridge`, `MedicalCounter`, `BathroomSink`, `Toilet`, `ShowerStall`, `Bathtub`, `SpaBath`, `WasherDryer`, `LinenCloset`, `FumeHood`) must require `WallEdgeKind::Wall | Window` behind every occupied cell along their backed face (`has_backed_wall`) instead of falling back to open edges. General wall furniture (`Sofa`, `TvStand`, `Bed`, `Desk`) should try solid-backed walls first before falling back; kitchen centers use freestanding `BarCounter` islands or `RoundTable`s, never `KitchenCounter`s.
- **Architectural Realism Invariants (`Hospital`, `MedLabSector`, Homes, Zoning)**:
  - *Hospitals & Labs*: Include partitioned patient suites with compact `CeramicTileFloor` en-suite bathrooms, corridor `NurseStation`s, aligned multi-floor stairs + elevators, tabletop `DeskClutter`, and scientific/fabrication rigs rather than residential furniture.
  - *Residences, Apartments & Zoning*: Carve continuous `Role::Hall` spines with aligned `Stairwell` shafts (`B1`/`1F`/`2F`) before subdividing room bays so navigation never traverses bedrooms, bathrooms, or laundries. Omit `Role::Foyer` on small houses, rowhouses, and compact apartments (`< 24×18`), opening front doors directly into `Living`, `Dining`, or `Kitchen`. Budget facilities at the whole-building/compound level (`HomeGenConfig::PRIMARY` vs. `generate_annex_wing_in_lot` for estate guest wings): max `1` `WasherDryer` across `B1+1F+2F`, max `1` `Kitchen` per primary residence, `1` full `Bathroom` + at most `1` `HalfBath` (`>= 7` ground bays) on `1F`, `<= 2` baths (`MasterBath` + shared `Bathroom`) on `2F`, cap `Bathroom <= 14 m²` / `MasterBath <= 18 m²`, keep `Role::Office` single-desk, and rotate specialty rooms (`Sunroom`, `Library`, `Office`, `Garage`, `HomeGym`) on larger homes without repeating. Give `MiddleClass`+ suburban houses smaller footprints with `LawnGrassZone` yards; keep `ProjectsHighDensity` visually and structurally distinct from `UrbanResidential` (`BrownstoneStoop` vs. `AwningCanopy`).
  - *Walkway Continuity & Outward Ribbon Endcaps*: Keep compound walkways closed/joined into loops or spurs (stopping lower-priority `AsphaltRoad` driveways short of stone/brick plazas) and verify semicircle endcaps satisfy `mid · outward >= 0` (`±TAU` flip in `cap_arc_angles_and_radii`) so axis-aligned ribbons never fold into V-notches.
  - *Mandatory Structural Layout Assertions*: Do **not** rely solely on `all_walkable_areas_connected` or `!prop_crosses_wall_edge` in map generator tests — both pass when fixtures float in open spaces or entries open into bathrooms. New/updated generator tests must assert: (a) every `KitchenCounter` has `Wall | Window` edges along its full backed span according to `rotation_deg`, (b) edge-bounded single-room flood-fills from front entry doors never contain `Toilet`/`WasherDryer` (and omit `MarbleFloor` foyers on small dwellings), and (c) whole-building counts respect `WasherDryer <= 1`, `Stove <= 1`, and per-floor bath caps.
- **6-Step Full-Stack Checklist for New `PropKind` / `SurfaceFeatureKind` / WebP Textures**:
  1. `crates/satya-schema`: Add variant to `define_wire_enum!` (`count = N`); verify `serde` `SCREAMING_SNAKE_CASE` matches the primary wire code on digit boundaries (e.g. `Printer3dStation` -> `"PRINTER3D_STATION"` primary + `"PRINTER_3D_STATION"` alias).
  2. `crates/satya-maps/src/map_dressing.rs`: Register `PropSpec` in `PROP_CATALOG`.
  3. `src/satya_ml/texture_gen/catalog.py`, `crates/satya-web/src/wasm_client/map_2d_art_textures.rs` (`BATTLEMAP_TEXTURE_SPECS`), & `tests/test_texture_gen_seamless.py`: Register matching WebP tile filenames (enforced by `test_battlemap_2d_texture_catalog_matches_rust_wasm_registry` in `tests/test_schema_contracts.py`), generate/seam-check, and run `pixi run verify-textures`.
  4. `crates/satya-web/src/wasm_client/`: Implement 2D SVG glyph (`map_2d_art*.rs` + `BATTLEMAP_TEXTURE_SPECS`), 3D mesh (`map_3d*.rs`), and add the variant uniquely to `PROP_BRUSH_GROUPS` (`map_edge_edit.rs`).
  5. `crates/satya-web` tests verify all `PropKind::ALL` variants render balanced `<g>`/`</g>` SVG and unique brush membership.
  6. When adding or evolving map/Avro DTOs in `crates/satya-schema`, keep `#[serde(default)]` (or Avro `v2+` / default-backed schema evolution) and verify `v1` wire compatibility with `pixi run test-schema`.

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

> [!NOTE]
> **`docs/ttrpg-examples/` Boundary**: Verbose human-only prose play tutorials live separately in [`docs/ttrpg-examples/`](../../../docs/ttrpg-examples/AGENTS.md) and are intentionally excluded from `STATIC_CHAPTERS` and PyLance RAG. Do **not** read, grep, or sync `docs/ttrpg-examples/` unless the user explicitly requests it.

> Only the 25 `docs/ttrpg/*.md` chapters are embedded at compile time (`include_str!` in [`crates/satya-rules-book/src/lib.rs`](../../../crates/satya-rules-book/src/lib.rs)); `docs/campaign/` and `docs/ttrpg-examples/` are **not** embedded in Rust. `pixi run build-book` (`docs/book/` HTML generation) triggers a `--release` `build-py` compile and is optional/on-demand unless requested or structurally needed.

Whenever adding, renaming, or splitting a `docs/ttrpg/` markdown chapter or changing a canonical formula:
1. Update [`docs/ttrpg/SUMMARY.md`](../../../docs/ttrpg/SUMMARY.md) (Table of Contents + Reciprocal Rust Engine Module Index) and ensure the chapter has a `> **Rust Engine Implementation**:` callout at the top.
2. Update `STATIC_CHAPTERS` in [`crates/satya-rules-book/src/lib.rs`](../../../crates/satya-rules-book/src/lib.rs) and `resolve_markdown_link_chapter_id` in [`crates/satya-rules-book/src/rules_book_html.rs`](../../../crates/satya-rules-book/src/rules_book_html.rs).
3. Update the relative link map in [`crates/satya-rules-book/src/bin/build_rules_book.rs`](../../../crates/satya-rules-book/src/bin/build_rules_book.rs).
4. Update `BOOK_CHAPTER_SPECS` in [`src/satya_ml/build_rules_book.py`](../../../src/satya_ml/build_rules_book.py) and `CHAPTER_SPECS` in [`notebooks/satya_rules_book.py`](../../../notebooks/satya_rules_book.py).
5. Update `_CANONICAL_RULES_FILES` in [`src/satya_ml/ai_gm/tools.py`](../../../src/satya_ml/ai_gm/tools.py).
6. Run the targeted test `pixi run test-rules-book` (~1s) to verify `test_rules_book_catalog_search_and_rendering` (ensuring zero raw LaTeX leaks, zero `&amp;amp;` double-escaping, quoted Mermaid labels, valid currency strings, and intact external cross-links) without running unrelated procedural map or engine suites.
