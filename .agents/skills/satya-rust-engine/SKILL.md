---
name: satya-rust-engine
description: >-
  Domain architecture, shared primitives (Triad, ContestedCheck, ConditionImpact,
  define_coded_enum!), sign conventions (Delta_int vs Delta_ext/Delta_tool), and
  bidirectional docs/ttrpg/ synchronization workflow for crates/satya-engine.
  Activate this skill whenever working on crates/satya-engine, TTRPG mechanics,
  character creation math, combat, ballistics, or docs/ttrpg/ markdown chapters.
---

# Satya Rust Game Engine, Campaign & Rules Book Skill (`satya-rust-engine`)

Use this skill alongside [`satya-rust-workspace`](../satya-rust-workspace/SKILL.md) whenever reading, adding, or refactoring code in [`crates/satya-engine`](../../../crates/satya-engine), [`crates/satya-campaign`](../../../crates/satya-campaign), [`crates/satya-rules-book`](../../../crates/satya-rules-book), or TTRPG design chapters in [`docs/ttrpg/`](../../../docs/ttrpg). (For [`crates/satya-maps`](../../../crates/satya-maps), activate [`satya-maps`](../satya-maps/SKILL.md).)

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

## 3. Continuous Metric Tactical Maps & Prop Sprites (`crates/satya-maps`)

- Moved to the dedicated [`satya-maps`](../satya-maps/SKILL.md) skill — activate `satya-maps` whenever modifying map geometry, procedural sector generators, `PropKind`s, or battlemap prop sprites.

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
