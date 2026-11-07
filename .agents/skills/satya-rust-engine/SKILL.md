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

Use this skill alongside [`satya-rust-workspace`](../satya-rust-workspace/SKILL.md) whenever reading, adding, or refactoring code in [`crates/satya-engine`](../../../crates/satya-engine) or TTRPG design chapters in [`docs/ttrpg/`](../../../docs/ttrpg). Full reference: [`docs/technical/rust-engine-steering.md`](../../../docs/technical/rust-engine-steering.md).

---

## 1. Shared Primitives (`crates/satya-engine/src/primitives.rs`)

Always build on the shared primitives in [`crates/satya-engine/src/primitives.rs`](../../../crates/satya-engine/src/primitives.rs) instead of ad-hoc fields:
- **Tactical Triad (`Triad<T>`)**: Use `Triad<T>` (`tempo`, `control`, `force`) for allocations (`Triad<i32>`), ceiling caps (`Triad<i32>`), and limb-slash flags (`Triad<bool>`). Never pass `(att_t, att_c, att_f, def_t, def_c, def_f)` as 6 separate integers when `Triad<i32>` applies.
- **Contested Saves (`ContestedCheck<D>` & `TieredSaveEval`)**: All 1-Pair Tiered GM Resistance checks (`ContestedCascadeTaskParams::new_tiered_environment`, where $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$ vs. $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$) must use `TieredSaveEval::evaluate` + `classify_4way` and return `ContestedCheck<D>` (`#[serde(flatten)] pub effect: D` + `Deref<Target = D>`).
- **Conditions (`ConditionSource` & `ConditionImpact`)**: Implement `ConditionSource` (`fn to_condition_impact(&self) -> ConditionImpact`) on every status, fatigue, encumbrance, or environmental evaluation struct, and combine them via `ConditionImpact::combine` or `aggregate_condition_impacts`.
- **Coded Enums (`define_coded_enum!`)**: Declare domain enums with `define_coded_enum!` to generate `ALL`, `as_str`, `label`, `full_name`, `subtitle`, `Display`, and alias-aware `from_code_str`.

---

## 2. Static `const` Catalogs & `Index` Accessors

- Store static game tables (`CANONICAL_SKILL_TREES`, `CANONICAL_ACTION_CATALOG`, `MELEE_WEAPON_CATALOG`, `RANGED_FIRE_MODES`, `ARMOR_CATALOG`, `HIT_LOCATION_TABLE`, `UNIVERSAL_TALENTS`) as `pub const` arrays/slices via declarative macros in dedicated `*_catalog.rs` modules (`skills_catalog.rs`, `lifepath_catalog.rs`, `action_catalog_stealth.rs`).
- Attach `pub const fn spec(self) -> &'static ...` directly to ID enums (`MeleeWeaponId`, `RangedFireModeId`, `ArmorId`, `HitLocationRegion`).
- Use `Index` / `IndexMut` (`chars[CharacteristicCode::FOR]`, `chars[PsycheTraitId::Ego]`, `apts[AptitudeCode::AptBio]`) for dynamic stat access.
- Call the canonical derived-stat functions in `characteristics.rs` (`compute_d_cog_score`, `compute_ooda_score`, `compute_pow_striking_power`, `compute_mmf_momentum_factor`, `compute_fatigue_threshold_score`, `compute_wound_shock_resistance_score`, `compute_social_composure_score`, `compute_general_awareness_score`).

---

## 3. Canonical Sign Conventions ($\Delta_{\text{int}}$ vs. $\Delta_{\text{ext}}$ / $\Delta_{\text{tool}}$ vs. Characteristic Penalties)

- **Internal Failure-Carry / Strain Modifiers ($\Delta_{\text{int}}$ in `ConditionImpact::delta_int` and `CascadeTaskParams::internal_mod`)** adjust the failure-carry stumble magnitude ($\delta_{\text{under}} = \max(0, 6 + \Delta_{\text{int}})$):
  - **Positive** (`+2` per Fatigue level, `+4` Nausea, `+2` Ego Wound) = **Penalty** (widens failure stumble).
  - **Negative** (`-2` Focused / Conviction Surge) = **Bonus** (reduces failure stumble).
- **Direct Characteristic / Pool Modifiers** (`fit_penalty`, `hlt_penalty`, `delta_mot`, `tactical_pool_mod`) adjust $C$ or $P_{\text{eff}}$ directly:
  - **Negative** (`-2`, `-6`) = **Penalty**; **Positive** (`+1`, `+2`) = **Bonus**.
- **External / Tool Modifiers ($\Delta_{\text{ext}}$, $\Delta_{\text{tool}}$)** adjust target numbers directly ($\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$ and $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$):
  - **Positive** (`+3` Darkness, `+6` High-Security Lock, `+2` Improvised Tool) = **Harder Task** (raises $\text{TN}_P$ or lowers $\text{TN}_{GM}$).
  - **Negative** (`-2` Superior Tool, `-4` Trivial Task) = **Easier Task**.

---

## 4. Bidirectional `docs/ttrpg/` <-> `satya-engine` Synchronization Checklist

Whenever adding, renaming, or splitting a `docs/ttrpg/` markdown chapter or changing a canonical formula:
1. Update [`docs/ttrpg/SUMMARY.md`](../../../docs/ttrpg/SUMMARY.md) (Table of Contents + Reciprocal Rust Engine Module Index) and ensure the chapter has a `> **Rust Engine Implementation**:` callout at the top.
2. Update `STATIC_CHAPTERS` in [`crates/satya-engine/src/rules_book.rs`](../../../crates/satya-engine/src/rules_book.rs) and `resolve_markdown_link_chapter_id` in [`crates/satya-engine/src/rules_book_html.rs`](../../../crates/satya-engine/src/rules_book_html.rs).
3. Update the relative link map in [`crates/satya-engine/src/bin/build_rules_book.rs`](../../../crates/satya-engine/src/bin/build_rules_book.rs).
4. Update `BOOK_CHAPTER_SPECS` in [`src/satya_ml/build_rules_book.py`](../../../src/satya_ml/build_rules_book.py) and `CHAPTER_SPECS` in [`notebooks/satya_rules_book.py`](../../../notebooks/satya_rules_book.py).
5. Update `_CANONICAL_RULES_FILES` in [`src/satya_ml/ai_gm/tools.py`](../../../src/satya_ml/ai_gm/tools.py).
6. Run `cargo test --offline -p satya-engine` to verify `test_rules_book_catalog_search_and_tactical_pool_envelope` (ensuring zero raw LaTeX leaks, zero `&amp;amp;` double-escaping, quoted Mermaid labels, and valid currency strings).
