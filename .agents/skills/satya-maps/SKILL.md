---
name: satya-maps
description: >-
  Continuous metric tactical maps, procedural sector generators, prop density and
  N-S/E-W alignment discipline, priority-sorted disjoint surfaces, WallEdgeIndex
  architectural realism, and the full-stack PropKind + Nano Banana 2 3-variant
  prop sprite pipeline for crates/satya-maps, crates/satya-schema, crates/satya-web,
  and src/satya_ml/texture_gen. Activate whenever working on map generators, props,
  surfaces, LoS/ballistics geometry, or painted battlemap sprites.
---

# Satya Continuous Metric Tactical Maps & Prop Sprite Pipeline (`satya-maps`)

Use alongside [`satya-rust-workspace`](../satya-rust-workspace/SKILL.md) whenever modifying [`crates/satya-maps`](../../../crates/satya-maps), battlemap art in [`crates/satya-web`](../../../crates/satya-web), or battlemap sprite/texture tooling in [`src/satya_ml/texture_gen/`](../../../src/satya_ml/texture_gen).

---

## 1. Module Map (`crates/satya-maps/src/`)

- **Spatial Core & Ballistics**: `tactical_map.rs` (procedural sectors, `evaluate_los_continuous`, `compute_reachable_continuous`), `tactical_map_{geom,inspect,edges,reach,spatial}.rs`, `map_dressing.rs`, `map_surface_{geom,queries}.rs`, `barrier_ballistics.rs`.
- **Procedural Sector Generators**: `map_interior_{gen,topologies}.rs`, `map_home_floorplan.rs`, `map_room_furnish.rs`, `map_outdoor_{gen,natural}.rs`, `map_estate_grammars.rs`, `map_sector_{gen,scifi}.rs`, `map_urban_grammars.rs`, `map_gen_{residential,commercial,zoning,civic_retail,agro_industrial,awakened}.rs`, `map_pool_spa_gen.rs`.
- **Continuation, World Graph & Narrative**: `map_continuation*.rs`, `map_world_{graph,transitions}.rs`, `map_narrative_manifest.rs` (`prop_semantic_zone`, `SectorNarrativeManifest`, `evaluate_passive_movement_perceptions`).

---

## 2. Core Spatial, Surface & Architectural Rules

- **Metric Units (`_m` / `_cm`) & `PropFootprintM`**: Use `f32` meters (`_m`) for raycast/OBB/circle math and `i32` centimeters (`_cm`) on wire DTOs (`GmMapUpdateRequest`, `TokenVisualSpec`, `MapPropPlacement`). Extract OBB geometry via `p.footprint_m()` (`satya_schema::PropFootprintM`) rather than re-implementing rotation transforms.
- **`EdgeSide` Canonical Representation**: `EdgeSide` has **only** `North` and `West` variants (`South` of `(x, y)` is `(x, y + 1, EdgeSide::North)`, `East` is `(x + 1, y, EdgeSide::West)`).
- **Compass Bearing & Ray-Vertex Convention**: Surveillance cones/facing use map compass degrees (`0° = North/-y`, `90° = East/+x`, `180° = South/+y`, `270° = West/-x`). Aim LoS rays through cell centers `(x + 0.5, y + 0.5)` to avoid boundary-vertex grazing.
- **Priority-Sorted Disjoint Vector Surfaces**: 2D SVG (`map_2d_art_surfaces.rs`), 3D WebGPU (`map_3d_surfaces_geom.rs`), and `topmost_surface_at_m` (`map_surface_queries.rs`) resolve strictly by `SurfaceFeatureKind::layer_priority()`, **never** by `Vec` push order. Never overlap a higher-priority floor polygon across an entire wing (e.g., carve disjoint rectangles for `ClinicalVinylFloor` / `CeramicTileFloor` / `CarpetFloor`, `FaradayCopperFloor` / `SubstationGrating` / `GravelRoad`, and `BrickPaverWalk` / `ConcreteWalk` / `FloodedAsphalt`).
- **Verify Topmost Exposure, Not Just `Vec` Membership**: Generator tests must probe `topmost_surface_at_m(&sector.surfaces, x_m, y_m)` across cells so every emitted `1F`/`B1`/`2F` surface has non-zero exposed area.
- **Wall-Flush Props vs. Ballistics & Visual-Only Bypass**: `prop_slices_wall_segment` insets footprints by `WALL_SKIN_EPS_M` (`0.03m`), while `intersect_segment_prop` is exact. Route gameplay obstruction through `effective_obstruction(p)` (`p.decorative == true` never blocks movement, cover, or LoS). Check multi-floor transitions with `PropKind::is_vertical_portal()` (`Stairwell`, `ElevatorCar`, `ServiceElevator`).
- **Wall-Edge-Backed Indoor Furnishing (`WallEdgeIndex`)**: Populate `WallEdgeIndex` via `Furnisher::reserve_doors(&edges)`, and require `WallEdgeKind::Wall | Window` behind every occupied cell of wall-dependent fixtures (`KitchenCounter`, `Stove`, `Fridge`, `MedicalCounter`, `BathroomSink`, `Toilet`, `ShowerStall`, `Bathtub`, `SpaBath`, `WasherDryer`, `LinenCloset`, `FumeHood`).
- **Architectural Realism (`Hospital`, Homes, Zoning, Walkways)**:
  - Carve `Role::Hall` spines with aligned `Stairwell` shafts (`B1`/`1F`/`2F`) before subdividing room bays; omit foyers on compact dwellings (`< 24×18`); enforce whole-home `WasherDryer <= 1`, `Stove <= 1`, bathroom caps, and single-desk `Role::Office`.
  - Keep walkways joined cleanly with outward endcaps (`mid · outward >= 0`), and assert backed-wall fixture checks, entry-room flood-fill sanity, and facility caps in residential tests.

---

## 3. Sector Density, Prop Alignment & Placement Discipline

- **Scale-Aware Density & Thematic Authenticity**:
  - Never leave `>= 24×18`, `36×26`, or `48×36` sectors with small fixed 8–12 prop lists or empty partitioned bays. Subdivide large halls into functional sub-rooms (dispatch, SecOps, diagnostics, breaker bays, restrooms), loop repetitive fixtures along **both** X and Y axes (`PalletRack`, `CryoComputeCluster`, `CropRow`), and loop over **every** partitioned alcove/wing (`DerelictCorridor`, patient wards).
  - Add dedicated `PropKind`s + sprites rather than using out-of-place stand-ins (`Wardrobe` in orbital airlocks, `Boulder` as starship wreckage, `BarHall` pool tables/jukeboxes in `CafeRestaurant`).
- **Prop Alignment Discipline (Orthogonal `N–S / E–W` vs. Controlled Jitter)**:
  - **Well-Maintained & High-Traffic Spaces Must Stay Orthogonal (`0° / 90° / 180° / 270°`, `rot_jitter = 0`)**: Fixtures in retail stores (`ApparelBoutique`, `Grocery`, `Pharmacy`, `DepartmentAnchor`), mall & plaza concourses (`MallKiosk`, `PatioUmbrellaTable`, `RoundTable`, benches), cafes, offices, hospitals, clinics, corporate bunkers, labs, and formal conservatory interiors must align cleanly to `N–S` or `E–W` axes.
  - **Controlled Off-Axis Rotation Permitted Only In**:
    1. **Rural or natural areas** (`Farms`, `RuralCountryside`, `WoodedPark`, `WoodlandCreek`, `RoadsideDitch`, organic pasture/orchard gear, `CropRow`, trees, bushes, boulders, fallen logs);
    2. **Unmaintained, low-economic, or chaotic urban/derelict zones** (`LowIncome` / `Destitute` back alleys, `GarbagePile`, `StreetDebris`, `DerelictCorridor` `HullDebris` & scattered pods);
    3. **Small tabletop clutter** (`DeskClutter`) and **roadside vehicles/barriers** matched to a curved ribbon tangent.
- **Placement Clearance & Multi-Floor Test Invariants**:
  - Inset multi-cell props by their full `w_cells`/`h_cells` before interior partitions (`x <= wall_x - w_cells`) so they aren't silently dropped by `try_push_prop`. Keep wide promenade props (`BusShelter`, `PatioUmbrellaTable`) clear of neighbors.
  - Keep exterior props clear of spawn pads (`(2.5, h/2 ± 0.5)` and `(w - 2.5, h/2 + 0.5..+1.5)`) and keep non-vehicle street props off `AsphaltRoad` (`Car | UtilityVan | JerseyBarrier` only, except `DockingBay`'s indoor apron) via `SurfaceAllowance::EXTERIOR_OFF_ROAD` or `try_place_off_road_cm`.
  - Check `1F` connectivity via `all_walkable_areas_connected`, check enclosed `B1`/`2F` levels via `floor_lot_connected`, and assert scale-appropriate minimum prop counts + required theme props in `crates/satya-maps/src/tests_tactical_map.rs`.

---

## 4. Full-Stack Checklist for New `PropKind`s, Sprites & WebP Textures

1. **Schema (`crates/satya-schema/src/tactical_map_dressing_dto.rs`)**: Append variant to `PropKind` via `define_wire_enum!` and update `[count = N]` + `PropKind::ALL.len()`. Ensure `serde` `SCREAMING_SNAKE_CASE` matches the primary code across digit/acronym boundaries (e.g. `Printer3dStation` → `"PRINTER3D_STATION"`, `SciFiCargoPod` → `"SCI_FI_CARGO_POD"`).
2. **Engine Catalog & Narrative (`crates/satya-maps`)**: Append matching `PropSpec` in identical enum order to `PROP_CATALOG` (`map_dressing.rs`), and add the variant to `prop_semantic_zone` (`map_narrative_manifest.rs`).
3. **2D SVG, 3D Mesh & Editor Palette (`crates/satya-web/src/wasm_client/`)**: Add 2D fallback SVG glyph (`map_2d_art_props*.rs` + dispatch in `map_2d_art_furniture.rs`), 3D diorama mesh (`map_3d_props*.rs` + dispatch in `map_3d_dressing_geom.rs`), and unique `PROP_BRUSH_GROUPS` entry (`map_edge_edit.rs`).
4. **Painted Sprite Registry (`crates/satya-web/src/wasm_client/map_2d_art_sprites.rs`)**: Register `PropKind => "stem" (W x H[, tile=true])` in `prop_sprite_specs!` and update `all_<N>_prop_sprite_variants_exist_and_toggle_with_texture_mode`. (For terrain tiles, sync `src/satya_ml/texture_gen/catalog.py`, `map_2d_art_textures.rs`, `tests/test_texture_gen_seamless.py`, and run `pixi run verify-textures`).
5. **Python Sprite Catalog (`src/satya_ml/texture_gen/`)**: Add `PropSpriteSpec` (strictly 90° bird's-eye `#FF00FF` magenta prompt + `3` `variant_directives`) to `prop_catalog_packs.py` (or thematic module), register batch IDs in `prop_catalog.py`, `prop_cli.py`, and `__init__.py`, and update `tests/test_prop_sprite_matting.py`. Avoid magenta/pink prop pigments unless overriding `key_color="green"`.
6. **Generate Raw Sprites & Bake Locally (`Nano Banana 2`)**:
   - Step A (**Outside Sandbox**, network only):
     ```powershell
     $env:PYTHONPATH='src'; .\.pixi\envs\default\python.exe -m satya_ml.texture_gen.prop_cli --batch <batch> --variants 3 --workers 6 --fetch-raw-only --skip-existing
     ```
   - Step B (**Inside Sandbox**, matte + contact sheet + verification):
     ```powershell
     $env:PYTHONPATH='src'; .\.pixi\envs\default\python.exe -m satya_ml.texture_gen.prop_cli --batch <batch> --variants 3 --process-raw-dir data/prop_sprites_raw --contact-sheet <artifact_dir>/props_<batch>_preview.png
     $env:PYTHONPATH='src'; .\.pixi\envs\default\python.exe -m satya_ml.texture_gen.prop_cli --batch <batch> --variants 3 --verify-only
     ```
     `--verify-only` enforces `corner_alpha_max < 16`, `0.08 <= fg_coverage <= 0.96`, and `edge_spill_frac <= 0.08`. Delete any bad `_raw.png` and re-fetch with `--only <id>` if needed, then visually inspect the contact sheet.
7. **Full Verification**:
   ```powershell
   cargo test --offline -p satya-schema; if ($LASTEXITCODE -eq 0) { cargo test --offline -p satya-maps }; if ($LASTEXITCODE -eq 0) { cargo test --offline -p satya-web --lib }; if ($LASTEXITCODE -eq 0) { cargo clippy --offline -p satya-web --target wasm32-unknown-unknown -- -D warnings }; if ($LASTEXITCODE -eq 0) { .\.pixi\envs\default\python.exe -m pytest tests/test_prop_sprite_matting.py }
   ```
