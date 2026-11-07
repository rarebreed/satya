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

## 4. Function Signatures, Module Splits & File Size Ceiling (`<= 45 KB`)

1. **Maximum 5–6 Positional Parameters**: If a function requires more than 6 inputs, group related parameters into a domain context struct or `Triad<i32>`.
2. **Hard File Size Ceiling (`<= 45 KB` / `< 46,080` bytes)**:
   - Every `.rs` file in `crates/satya-engine/src/` and every `.md` file in `docs/ttrpg/` must remain under `45 KB` (~1,000 lines) so an AI coding agent can read the entire file in a single `read_file` call without truncation.
   - **Split Files Rather Than Deleting Content**: When a Rust module or TTRPG markdown chapter approaches `45 KB`, split it into modular companion files along domain boundaries rather than cutting prose, worked examples, or reference tables.
3. **Separate Static Catalogs, Sub-Domains, Walkthroughs & HTML Rendering**:
   - **Static Reference Catalogs**: Store large `const` tables in dedicated `*_catalog.rs` modules ([`skills_catalog.rs`](../../crates/satya-engine/src/skills_catalog.rs), [`lifepath_catalog.rs`](../../crates/satya-engine/src/lifepath_catalog.rs), [`action_catalog_stealth.rs`](../../crates/satya-engine/src/action_catalog_stealth.rs)).
   - **Domain Companion Modules**: Keep core mechanics modularized by sub-domain (`combat.rs` + `ranged_combat.rs` + `ballistics.rs`, `psyche_social.rs` + `social_combat.rs`, `health_survival.rs` + `status_and_triage.rs`, `environment_vehicles.rs` + `environmental_hazards.rs`, `dice_probabilities.rs` + `contested_cascade.rs`).
   - **Preserve Flat Re-Exports When Splitting**: Whenever a module is split into a companion `.rs` file, re-export its public items (`pub use companion::*;`) in both the parent module and `crates/satya-engine/src/lib.rs` so downstream crates (`satya-web`, `satya-py`, `satya-signaling`) continue to compile without import-path breakage.
   - **Narrative Walkthroughs**: Named character walkthrough simulations (Viktor vs. Elena, Mateo vs. Sgt. Vance, Marcus vs. Nadia, Mateo/Priya/Amara lifepath builds) belong in [`crates/satya-engine/src/walkthroughs.rs`](../../crates/satya-engine/src/walkthroughs.rs).
   - **Documentation Rendering**: Markdown/LaTeX/Mermaid-to-HTML rendering belongs in [`rules_book_html.rs`](../../crates/satya-engine/src/rules_book_html.rs), separate from chapter catalog and search logic in [`rules_book.rs`](../../crates/satya-engine/src/rules_book.rs).
   - **Unit Test Suites**: Keep cross-module and domain test suites split across [`tests.rs`](../../crates/satya-engine/src/tests.rs) and [`tests_domain.rs`](../../crates/satya-engine/src/tests_domain.rs).
4. **Canonical Sign Conventions ($\Delta_{\text{int}}$ vs. $\Delta_{\text{ext}}$ / $\Delta_{\text{tool}}$)**:
   - **Internal Competence Modifiers ($\Delta_{\text{int}}$)** adjust effective competence ($C_{\text{eff}} = C + \Delta_{\text{int}}$):
     - **Negative** (`-2` Fatigue, `-4` Wound Shock, `-2` Off-Balance) = **Penalty** to character competence (increases $\text{TN}_P = 31 - C_{\text{eff}}$).
     - **Positive** (`+1` Focus/Preparation) = **Bonus** to character competence (decreases $\text{TN}_P$).
   - **External / Tool Modifiers ($\Delta_{\text{ext}}$, $\Delta_{\text{tool}}$)** adjust target numbers directly ($\text{TN}_{\text{eff}} = 31 - C_{\text{eff}} + \Delta_{\text{ext}}$ and $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$):
     - **Positive** (`+3` Darkness, `+6` High-Security Lock, `+2` Improvised Tool) = **Harder Task** (raises $\text{TN}_P$ or lowers $\text{TN}_{GM}$ so environmental resistance cascades more easily).
     - **Negative** (`-2` Superior Tool, `-4` Trivial Lock) = **Easier Task** (lowers $\text{TN}_P$ or raises $\text{TN}_{GM}$).
   - Never mix or invert $\Delta_{\text{int}}$ and $\Delta_{\text{ext}}$ signs when aggregating `ConditionImpact` or evaluating `TieredSaveEval`.

---

## 5. Bidirectional Synchronization Between `docs/ttrpg/` and `satya-engine`

1. **`docs/ttrpg/` is the Canonical Design Authority**:
   - If a formula, modifier, threshold, or table in `crates/satya-engine` ever diverges from `docs/ttrpg/`, `docs/ttrpg/` is the design source of truth unless the user explicitly directs a rule change (in which case both `docs/ttrpg/` and `crates/satya-engine` must be updated together).
2. **Reciprocal Traceability Links & High-Signal Docstrings**:
   - **Rust -> TTRPG (No `#![deny(missing_docs)]`)**: Require `//!` module headers and `///` docstrings on top-level `pub fn` calculators and non-obvious domain structs in `crates/satya-engine/src/`, citing the governing chapter path (`docs/ttrpg/...`), section (`§X.Y`), canonical formula, and any unit/sign conventions. Omit `///` comments on self-explanatory struct fields, enum variants, and simple accessors unless a field has a non-obvious unit (`kg`, `m/s`), scale (`1–20` controlling scale vs. `0.0–100.0` spectrum), or sign convention ($\Delta_{\text{int}}$ vs. $\Delta_{\text{ext}}$).
   - **TTRPG -> Rust**: Every markdown chapter in `docs/ttrpg/` begins with a `> **Rust Engine Implementation**:` callout linking to its implementing `crates/satya-engine/src/*.rs` module(s), and [`docs/ttrpg/SUMMARY.md`](../ttrpg/SUMMARY.md) maintains the master 1-to-1 Reciprocal Rust Engine Module Index.
3. **Markdown / LaTeX / Mermaid Rendering Invariants**:
   - `test_rules_book_catalog_search_and_tactical_pool_envelope` in [`crates/satya-engine/src/tests.rs`](../../crates/satya-engine/src/tests.rs) compiles every `docs/ttrpg/` chapter into HTML and asserts:
     - Zero raw `\text{...}` or `\frac{...}` LaTeX commands leaking outside `$...$` or `$$...$$` delimiters.
     - Zero double-escaped `&amp;amp;` entities.
     - All Mermaid node labels containing parentheses or brackets are quoted (`S1["Node Label (Extra Info)"]`).
     - Currency strings (`$500`, `$10,000`) are preserved as literal dollar amounts and not accidentally paired into inline LaTeX math.
   - **Windows Editing Pitfall**: Never use PowerShell double-quoted strings (`"..."`) to write or modify Markdown containing backticks (`` `r ``, `` `t ``, `` `f ``) or `$` math symbols, as PowerShell strips backtick escapes and interpolates `$var` as empty strings. Always use `edit_file` / `write_file` or UTF-8 Python scripts with LF (`\n`) line endings.
4. **Checklist When Adding, Renaming, or Splitting a `docs/ttrpg/` Chapter**:
   - Update [`docs/ttrpg/SUMMARY.md`](../ttrpg/SUMMARY.md)
   - Update `STATIC_CHAPTERS` in [`crates/satya-engine/src/rules_book.rs`](../../crates/satya-engine/src/rules_book.rs) and `resolve_markdown_link_chapter_id` in [`crates/satya-engine/src/rules_book_html.rs`](../../crates/satya-engine/src/rules_book_html.rs)
   - Update the relative link map in [`crates/satya-engine/src/bin/build_rules_book.rs`](../../crates/satya-engine/src/bin/build_rules_book.rs)
   - Update `BOOK_CHAPTER_SPECS` in [`src/satya_ml/build_rules_book.py`](../../src/satya_ml/build_rules_book.py) and `CHAPTER_SPECS` in [`notebooks/satya_rules_book.py`](../../notebooks/satya_rules_book.py)
   - Update `_CANONICAL_RULES_FILES` in [`src/satya_ml/ai_gm/tools.py`](../../src/satya_ml/ai_gm/tools.py)
   - Run `cargo test -p satya-engine` to verify chapter counts, HTML/LaTeX/Mermaid rendering, and search indexing.
