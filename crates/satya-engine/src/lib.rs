//! # `satya-engine`
//!
//! Satya TTRPG game engine, exact combinatorial & dynamic-programming dice probability distributions,
//! Two-List Cascading ("Watershed" / "Recoil") task resolution, and odds calibration sweeps.
//!
//! ## Module Structure & `docs/ttrpg/` Design Reference Mapping
//! - [`primitives`]: Shared foundational containers (`Triad<T>`, `ConditionImpact`, `ConditionSource`,
//!   `TieredSaveEval`, `ContestedCheck<D>`) and declarative catalog macros (`define_coded_enum!`,
//!   `impl_static_catalog!`) (`docs/ttrpg/01-foundation/mathematical_notation_glossary.md`).
//! - [`dice_pools`]: Exact discrete probability mass functions (`Pmf`), statistical moments
//!   (`DistributionStats`, Sarle's $\text{BC}$), and comparative dice pool topology benchmarks
//!   (`docs/ttrpg/01-foundation/probability_and_calibration.md` §1).
//! - [`dice_probabilities`] & [`contested_cascade`]: Two-List Cascading (`2d20`) dynamic-programming engines,
//!   Four-Factor task evaluators (`CascadeTaskParams`, `CascadeRollOutcome`), and 1-Pair Tiered GM Resistance
//!   contested evaluators (`ContestedCascadeTaskParams`, `ContestedOutcomeKind`)
//!   (`docs/ttrpg/01-foundation/probability_and_calibration.md` §1–§6,
//!   `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` §1–§5).
//! - [`task_resolution`]: Task Gate Classifications (`ClassA`/`B`/`C`), Tool Dependency & Quality tiers,
//!   Composite/Supporting Characteristic helpers, Complementary Synergy (`KNO -> PHY`), Dual-Skill Gates,
//!   Extended Project Clocks, and the 22-entry Master Task Requirements catalog
//!   (`docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` §2–§8).
//! - [`action_economy`] & [`action_catalog_stealth`]: Temporal calibration (`1 Tick = 0.25s`), OODA Loop &
//!   Quick Thinking ($D_{\text{cog}}$), Count-Up Tick Clock, `CANONICAL_ACTION_CATALOG`, `action_time_factor`,
//!   and Multi-Sensory Stealth & Detection (`docs/ttrpg/02-core-mechanics/action_economy_and_ooda.md` §1–§6).
//! - [`characteristics`]: All 30 Physical, Mental, Social, Sensory, and Psyche characteristics on the `1–20`
//!   Controlling Scale, 10 derived metrics (`POW`, `MMF`, `OODA`, `D_cog`, `AWR`, `FT`, `WSR`, `CMP`, `C_study`),
//!   and 9 Domain Aptitudes ($A \in [-2, +3]$)
//!   (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §1–§10).
//! - [`skills_trees`] & [`skills_catalog`]: Skill taxonomy (`TAL`, `PHY`, `KNO`, `PHY/KNO`), `[Access]` gates,
//!   the 8 Hierarchical Skill Trees (`CANONICAL_SKILL_TREES`), tree defaulting, and the 5-Factor Skill Point (`SP`)
//!   step cost formula (`docs/ttrpg/03-character/skills_and_trees.md` §1–§3,
//!   `docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7).
//! - [`lifepath_economy`], [`lifepath_catalog`] & [`economy_gear`]: Chronological Lifepath creation (Stages 1–5),
//!   School Quality (`SQM`), `DIS + Mental + APT` advancement rolls, Socioeconomic Upkeep (`SQM 1–10`),
//!   Black-Market `[Access]` procurement, and Operational `Heat`
//!   (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1–§3).
//! - [`psyche_social`] & [`social_combat`]: Psyche Friction Checks, Long-Term Psyche Drift, Conviction Surges,
//!   Combat Morale, Argument Vectors, Social Composure, and Cognitive Dissonance resolution
//!   (`docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §1–§4).
//! - [`combat`], [`combat_techniques`], [`ranged_combat`] & [`ballistics`]: Tactical Triad ("Envelope & Fill" — `Tempo`, `Control`, `Force`),
//!   30-entry Canonical Combat Technique Catalog (`Control`, `Strike`, `Intercept`, `Maneuver`, `RangedShot`, `RangedManeuver`),
//!   Melee Weapons, Reach & Clinch Polarity Flip, Kuzushi Throws, Granular Ballistics & Firearms Engineering
//!   ($E_k, I_{\text{KE}}, I_{\text{cal}}, W_{\text{yaw}}, \text{Acc}, \text{Rel}$), Ranged Fire Modes, Recoil Bracing,
//!   Suppression, Armor `PR` vs. `BABT`, and Weapon Malfunctions
//!   (`docs/ttrpg/05-combat/tactical_triad_and_pools.md`, `melee_combat.md`, `ranged_combat_and_armor.md`).
//! - [`health_survival`], [`status_and_triage`], [`environment_vehicles`] & [`environmental_hazards`]:
//!   `1d20` Hit Locations & Called Shots, Fatigue Levels (`0–5`), Asphyxiation, Structural Wounds (`Tiers 1–5`),
//!   Fractures, Hemorrhaging & Hypovolemic Shock, `WSR` Saves, Physiological & Mental Trauma Conditions,
//!   Combat Movement Gaits, Encumbrance (`Tiers 0–4`), Vehicular Chases & High-G Physics, and Environmental
//!   Survival (`docs/ttrpg/06-health-and-survival/health_and_trauma.md`, `movement_vehicles_and_environment.md`).
//! - [`rules_book`] & [`rules_book_html`]: Canonical Satya TTRPG Rules Book catalog (`docs/ttrpg/SUMMARY.md`),
//!   Markdown/LaTeX/Mermaid-to-HTML renderer, and multi-chapter search.
//! - [`tactical_map`], [`tactical_map_geom`], [`tactical_map_inspect`], [`tactical_map_edges`], [`map_dressing`], [`barrier_ballistics`], [`map_interior_gen`],
//!   [`map_outdoor_gen`], [`map_outdoor_natural`] & [`map_sector_gen`]: Continuous metric (`f32` m / `i32` cm) battlemap geometry, LoS/cover, hover inspection, thin
//!   edge walls/doors/windows, multi-cell prop catalog with per-material ballistic penetration vs. concealment
//!   grading, and dressed procedural sectors across all 12 presets.
//! - [`walkthroughs`]: Canonical character lifepath builds (Mateo Vance, Dr. Priya Nair, Amara Okafor)
//!   (`docs/ttrpg/03-character/lifepaths_and_economy.md` §2.1–§2.3).

pub mod action_catalog_stealth;
pub mod action_economy;
pub mod ballistics;
pub mod barrier_ballistics;
pub mod character_audit;
pub mod characteristics;
pub mod combat;
pub mod combat_techniques;
pub mod contested_cascade;
pub mod dice_pools;
pub mod dice_probabilities;
pub mod economy_gear;
pub mod environment_vehicles;
pub mod environmental_hazards;
pub mod health_survival;
pub mod lifepath_catalog;
pub mod lifepath_economy;
pub mod map_dressing;
pub mod map_interior_gen;
pub mod map_outdoor_gen;
pub mod map_outdoor_natural;
pub mod map_sector_gen;
pub mod npc_generator;
pub mod primitives;
pub mod psyche_social;
pub mod ranged_combat;
pub mod rules_book;
pub mod rules_book_html;
pub mod skills_catalog;
pub mod skills_trees;
pub mod social_combat;
pub mod status_and_triage;
pub mod tactical_map;
pub mod tactical_map_edges;
pub mod tactical_map_geom;
pub mod tactical_map_inspect;
pub mod task_resolution;
pub mod walkthroughs;

pub use action_economy::*;
pub use barrier_ballistics::*;
pub use character_audit::*;
pub use characteristics::*;
pub use combat::*;
pub use dice_pools::{
    SARLE_BIMODAL_THRESHOLD, try_median_gated_bimodal_pmf, try_order_statistic_pmf,
};
pub use dice_probabilities::*;
pub use health_survival::*;
pub use lifepath_economy::*;
pub use map_dressing::*;
pub use npc_generator::*;
pub use primitives::*;
pub use psyche_social::*;
pub use rules_book::*;
pub use skills_trees::*;
pub use tactical_map::*;
pub use tactical_map_geom::*;
pub use task_resolution::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_domain;
