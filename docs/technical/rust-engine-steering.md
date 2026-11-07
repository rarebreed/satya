# Rust Domain & Game Engine Technical Steering Guide (`satya-engine`)

This document defines the architectural standards, type-design patterns, and anti-bloat rules for `crates/satya-engine` and dependent Rust crates (`satya-schema`, `satya-web`, `satya-py`, `satya-signaling`). Both human engineers and AI coding agents must follow these rules when adding or modifying mechanics.

---

## 1. Core Design Philosophy: Density Over Boilerplate

1. **Zero Parallel Domain Types**: Never introduce a second enum or struct representing the same domain concept in a different module (e.g., do not create a separate `LoadEncumbranceTier` when `EncumbranceTier` exists, or a separate `PsycheProfile` when `CharacterCharacteristics` holds all psyche traits).
2. **One Canonical Formula Per Mechanic**: Every mathematical formula from `docs/ttrpg/` must live in **one** `const fn` or pure function in its owning domain module (`characteristics.rs`, `dice_probabilities.rs`, `action_economy.rs`, `combat.rs`, `health_survival.rs`, `psyche_social.rs`, `lifepath_economy.rs`). Downstream calculators must call that canonical function rather than re-writing `(ana + foc + ins) - 20` or `(mas / 70.0) * (for_stat * 2.0)`.
3. **Data-Driven Tables Over Multi-Screen `match` Blocks**: When a mechanic maps an enum or integer tier (`0..=N`) to a tuple of modifiers or strings, use a `const` lookup table or declarative macro (`define_coded_enum!`, `skill_node!`, `action_spec!`) rather than 200-line multi-line `match` arms.

---

## 2. Shared Primitives & Generics (`src/primitives.rs`)

Always build on the shared primitives in [`crates/satya-engine/src/primitives.rs`](../../crates/satya-engine/src/primitives.rs) instead of ad-hoc fields:

### 2.1 `Triad<T>` for `Tempo / Control / Force`
Whenever a struct or function deals with the Tactical Triad (`Tempo`, `Control`, `Force`)—whether integer allocations (`Triad<i32>`), ceiling caps (`Triad<i32>`), or boolean status flags like limb slashes (`Triad<bool>`)—use `Triad<T>`:
```rust
pub use crate::primitives::Triad;

// Good: single Triad<i32> value with combinators
let effective = alloc.clamp_min(0).clamp_each(envelope.caps_triad());
let total = effective.sum();

// Bad: three separate fields or 6 positional arguments (att_t, att_c, att_f, def_t, def_c, def_f)
```

### 2.2 `ContestedCheck<D>` & `TieredSaveEval` for Contested Saves
All 1-Pair Tiered GM Resistance checks (`ContestedCascadeTaskParams::new_tiered_environment`, where $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$ vs. $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$) must use `TieredSaveEval::evaluate` and `classify_4way`, and return `ContestedCheck<D>` (which implements `Deref<Target = D>` and `#[serde(flatten)]`):
```rust
pub type CombatMoraleRollOutcome = ContestedCheck<MoraleCheckEffect>;
```
Never copy-paste `player_tn`, `gm_tn`, and `contested_roll` across domain outcome structs.

### 2.3 `ConditionImpact` & the `ConditionSource` Trait
Physiological conditions, fatigue, encumbrance, wounds, and environmental hazards (dehydration, starvation, thermal stress, sleep deprivation, hypoxia, toxins) all affect the same downstream engine variables ($\Delta_{\text{int}}$, $\Delta_{\text{ext}}$, `tactical_pool_mod`, `action_time_factor_delta`, `fatigue_floor`, `triad_cap_penalty`, `triad_locked_zero`, `is_unconscious`).
- Implement `ConditionSource` (`fn to_condition_impact(&self) -> ConditionImpact`) on every status/survival evaluation struct.
- Combine multiple active conditions via `ConditionImpact::combine` or `aggregate_condition_impacts`.

---

## 3. Enums, Static Catalogs & Indexing

### 3.1 Coded Enums (`define_coded_enum!`)
Any enum that needs a canonical string code, human-readable label, optional subtitle, `ALL` array, and case-insensitive `from_code_str` parser must be declared with `define_coded_enum!`:
```rust
define_coded_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SkillTreeId [count = 8] {
        Tree1MathCompPhysicalSciences => {
            code: "TREE_1_MATH_COMP_PHYS",
            label: "Tree 1: Mathematics, Computation & Physical Sciences",
            aliases: ["TREE_1", "MATH_COMP_PHYS"],
        },
        // ...
    }
}
```
Do not hand-write `as_str`, `label`, `full_name`, `subtitle`, `Display`, or `from_code_str` match blocks.

### 3.2 `const` Slices & Enum `.spec()` Methods (No Heap-Allocating Catalogs)
Static game tables (skills, actions, weapons, fire modes, armor, hit locations, task presets) must be stored as `pub const` arrays/slices (`&'static [Spec]`) generated via compact file-local declarative macros:
- **Never** construct a `Vec<Spec>` at runtime just to look up a single entry.
- Provide `pub const fn spec(self) -> &'static Spec` directly on the ID enum (`MeleeWeaponId::spec`, `ArmorId::spec`, `RangedFireModeId::spec`, `HitLocationRegion::spec`).

### 3.3 `Index<K>` / `IndexMut<K>` for Stat Blocks
`CharacterCharacteristics` and `CharacterAptitudes` implement `std::ops::Index` and `std::ops::IndexMut` for `CharacteristicCode`, `PsycheTraitId`, and `AptitudeCode`. Prefer `chars[CharacteristicCode::FOR]` or `apts[AptitudeCode::AptBio]` over verbose getter/setter chains.

---

## 4. Function Signatures & Module Organization

1. **Maximum 5–6 Positional Parameters**: If a function requires more than 6 inputs, group related parameters into a domain context struct or `Triad<i32>`.
2. **Separate Narrative Walkthroughs from Core Mechanics**:
   - Core rules modules (`combat.rs`, `psyche_social.rs`, `lifepath_economy.rs`, `health_survival.rs`) must contain **only** types, static tables, and mechanical resolution functions.
   - Named character walkthrough simulations (Viktor vs. Elena, Mateo vs. Sgt. Vance, Marcus vs. Nadia, Mateo/Priya/Amara lifepath builds) belong in [`crates/satya-engine/src/walkthroughs.rs`](../../crates/satya-engine/src/walkthroughs.rs).
3. **Separate Documentation Rendering from Mechanics**:
   - Markdown/LaTeX/Mermaid-to-HTML rendering belongs in `rules_book/markdown_html.rs`, separate from chapter search/catalog logic in `rules_book/mod.rs`.
4. **Target File Size Ceiling (`<= 1,600 lines`)**:
   - No single `.rs` file in `crates/satya-engine/src/` should exceed ~1,600 lines. If a module grows past that threshold, split it into focused submodules or compress repetitive tables with a declarative macro.
