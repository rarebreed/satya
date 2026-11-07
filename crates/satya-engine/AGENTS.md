# `satya-engine` Agent Coding & Architecture Rules

When reading, adding, or refactoring code in `crates/satya-engine`, follow these mandatory architectural rules (full guide in `docs/technical/rust-engine-steering.md`):

1. **Use Shared Primitives (`src/primitives.rs`)**:
   - **Tactical Triad**: Always use `Triad<T>` (`tempo`, `control`, `force`) for allocations (`Triad<i32>`), caps (`Triad<i32>`), and limb-slash flags (`Triad<bool>`). Do not pass `(att_t, att_c, att_f, def_t, def_c, def_f)` as 6 separate integers when `Triad<i32>` applies.
   - **Contested Saves**: Always use `TieredSaveEval::evaluate` + `classify_4way` and return `ContestedCheck<D>` (`eval: TieredSaveEval` + `#[serde(flatten)] pub effect: D`, which implements `Deref<Target = D>`). Never create a new `*Outcome` struct that manually duplicates `player_tn`, `gm_tn`, and `contested_roll`.
   - **Status & Environmental Conditions**: Implement `ConditionSource` (`fn to_condition_impact(&self) -> ConditionImpact`) on every status, fatigue, encumbrance, or environmental evaluation type, and compose them with `ConditionImpact::combine` or `aggregate_condition_impacts`.
   - **Coded Enums**: Use `define_coded_enum!` for any enum requiring `ALL`, `as_str`, `label`, `full_name`, `subtitle`, and alias-aware `from_code_str`.

2. **Static `const` Catalogs & `Index` Accessors**:
   - Define static tables (`CANONICAL_SKILL_TREES`, `CANONICAL_ACTION_CATALOG`, `MELEE_WEAPON_CATALOG`, `RANGED_FIRE_MODES`, `ARMOR_CATALOG`, `HIT_LOCATION_TABLE`, `UNIVERSAL_TALENTS`) as `pub const` arrays/slices using compact declarative macros—never build a `Vec` on the heap just to look up an entry.
   - Attach `pub const fn spec(self) -> &'static ...` directly to ID enums (`MeleeWeaponId`, `RangedFireModeId`, `ArmorId`, `HitLocationRegion`).
   - Use `Index` / `IndexMut` (`chars[CharacteristicCode::FOR]`, `chars[PsycheTraitId::Ego]`, `apts[AptitudeCode::AptBio]`) for dynamic stat access.

3. **Single Source of Truth, File Size Ceiling (`<= 45 KB`), & Module Splits**:
   - Never duplicate a domain enum or mathematical formula across modules. Call the canonical derived-stat functions in `characteristics.rs` (`compute_d_cog_score`, `compute_ooda_score`, `compute_pow_striking_power`, `compute_mmf_momentum_factor`, `compute_fatigue_threshold_score`, `compute_wound_shock_resistance_score`, `compute_social_composure_score`, `compute_general_awareness_score`).
   - **Hard File Size Ceiling (`<= 45 KB` / `< 46,080` bytes)**: Every `.rs` file in `crates/satya-engine/src/` and every `.md` chapter in `docs/ttrpg/` MUST remain under `45 KB` (~1,000 lines) so it fits inside a single AI agent `read_file` call without byte/line truncation.
   - **Split Files Rather Than Deleting Verbiage or Logic**: When a Rust module or TTRPG markdown chapter approaches `45 KB`, **split it into focused companion files** rather than stripping prose, worked examples, or reference tables:
     - Separate static data tables into `src/*_catalog.rs` (`skills_catalog.rs`, `lifepath_catalog.rs`, `action_catalog_stealth.rs`).
     - Separate distinct sub-domains into companion modules (`combat.rs` + `ranged_combat.rs` + `ballistics.rs`, `psyche_social.rs` + `social_combat.rs`, `health_survival.rs` + `status_and_triage.rs`, `environment_vehicles.rs` + `environmental_hazards.rs`, `dice_probabilities.rs` + `contested_cascade.rs`, `rules_book.rs` + `rules_book_html.rs`).
     - **Preserve Flat Re-Exports When Splitting**: Whenever you split a `.rs` module into a companion file, re-export its public items (`pub use companion::*;`) in both the parent module and `src/lib.rs` so downstream crates (`satya-web`, `satya-py`, `satya-signaling`) continue to compile without path churn.
     - Keep narrative walkthroughs in `src/walkthroughs.rs` and test suites in `src/tests.rs` + `src/tests_domain.rs`.
     - Honor `#![deny(missing_docs)]` across all public items.

4. **Canonical Sign Conventions ($\Delta_{\text{int}}$ vs. $\Delta_{\text{ext}}$ / $\Delta_{\text{tool}}$)**:
   - **Internal Competence Modifiers ($\Delta_{\text{int}}$)** adjust effective competence ($C_{\text{eff}} = C + \Delta_{\text{int}}$): **negative** values (`-2` Fatigue, `-4` Wound Shock) **penalize** the character (raising $\text{TN}_P = 31 - C_{\text{eff}}$); **positive** values buff competence.
   - **External / Tool Modifiers ($\Delta_{\text{ext}}$, $\Delta_{\text{tool}}$)** adjust target numbers directly ($\text{TN}_{\text{eff}} = 31 - C_{\text{eff}} + \Delta_{\text{ext}}$ and $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$): **positive** values (`+3` Darkness, `+6` High-Security Lock, `+2` Improvised Tool) **increase difficulty** (raising $\text{TN}_P$ or lowering $\text{TN}_{GM}$); **negative** values (`-2` Superior Tool, `-4` Trivial Task) **reduce difficulty**. Never invert these two sign conventions.

5. **Bidirectional `docs/ttrpg/` <-> `satya-engine` Synchronization & Rendering Invariants**:
   - **`docs/ttrpg/` is the Canonical Design Reference**: Whenever formulas, thresholds, tables, or worked examples change in `docs/ttrpg/*.md` or `crates/satya-engine/src/*.rs`, update both sides in lockstep and verify with `cargo test -p satya-engine`.
   - **Mandatory Origin Citations in Rust Docstrings**: Every module (`//!`) and public struct/enum/function (`///`) in `crates/satya-engine/src/` must cite its exact `docs/ttrpg/<dir>/<chapter>.md` section (`§X.Y`) and canonical formula.
   - **Reciprocal Rust Links in TTRPG Chapters**: Every chapter in `docs/ttrpg/` must include a `> **Rust Engine Implementation**:` callout at the top linking to its implementing `crates/satya-engine/src/*.rs` module(s) and key entry-point functions, and `docs/ttrpg/SUMMARY.md` must keep its Reciprocal Rust Engine Module Index table in sync.
   - **Rules Book HTML / LaTeX / Mermaid Invariants**: `test_rules_book_catalog_search_and_tactical_pool_envelope` parses all chapters in `docs/ttrpg/` and asserts that no raw `\text{` or `\frac{` LaTeX commands leak outside `$...$`/`$$...$$`, no `&amp;amp;` double-escaping occurs, all Mermaid node labels with parentheses are quoted (`ID["Label (Detail)"]`), and literal dollar amounts (`$500`) are not accidentally paired into inline math. On Windows, never use PowerShell double-quoted strings (`"..."`) to edit markdown containing backticks or `$` variables (use `edit_file`/`write_file` or UTF-8 Python with `\n` line endings).
   - **Adding or Splitting a TTRPG Chapter**: Whenever a `docs/ttrpg/` markdown file is added, renamed, or split, update all 5 chapter registries in the same commit:
     1. `docs/ttrpg/SUMMARY.md` (Table of Contents + Reciprocal Rust Engine Module Index)
     2. `crates/satya-engine/src/rules_book.rs` (`STATIC_CHAPTERS` array) & `crates/satya-engine/src/rules_book_html.rs` (`resolve_markdown_link_chapter_id`)
     3. `crates/satya-engine/src/bin/build_rules_book.rs` (HTML filename link mapping)
     4. `src/satya_ml/build_rules_book.py` (`BOOK_CHAPTER_SPECS`) & `notebooks/satya_rules_book.py` (`CHAPTER_SPECS`)
     5. `src/satya_ml/ai_gm/tools.py` (`_CANONICAL_RULES_FILES`)
