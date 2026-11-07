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

3. **Single Source of Truth & Module Boundaries**:
   - Never duplicate a domain enum or mathematical formula across modules. Call the canonical derived-stat functions in `characteristics.rs` (`compute_d_cog_score`, `compute_ooda_score`, `compute_pow_striking_power`, `compute_mmf_momentum_factor`, `compute_fatigue_threshold_score`, `compute_wound_shock_resistance_score`, `compute_social_composure_score`, `compute_general_awareness_score`).
   - Put hardcoded narrative walkthroughs and sample NPC builders in `src/walkthroughs.rs`, not in core mechanics modules.
   - Keep individual `.rs` files under ~1,600 lines and honor `#![deny(missing_docs)]`.
