//! # `satya-engine`
//!
//! Core Satya TTRPG game mechanics engine, exact combinatorial & dynamic-programming dice probability
//! distributions, `2d20xN` Cascade ("Recoil" inverted) task resolution, lifepath & character
//! auditing, Tactical Triad combat, granular ballistics, health/trauma, and 2035 engineering.
//!
//! Companion workspace crates extracted from the engine:
//! - `satya-rules-book`: Embedded `docs/ttrpg/*.md` catalog, search, and Markdown/LaTeX/Mermaid HTML renderer.
//! - `satya-maps`: Continuous metric (`f32` m / `i32` cm) battlemap geometry, barrier ballistics, and procedural generators.
//! - `satya-campaign`: *Awakened* 2035 AI substrates, conspiracy arcs, faction templates, hook weaver, NPC generator, and world clocks.
//!
//! ## Module Structure & `docs/ttrpg/` Design Reference Mapping
//! - [`primitives`]: Shared foundational containers (`Triad<T>`, `ConditionImpact`, `ConditionSource`,
//!   `TieredSaveEval`, `ContestedCheck<D>`) and declarative catalog macros (`define_coded_enum!`,
//!   `impl_static_catalog!`) (`docs/ttrpg/01-foundation/mathematical_notation_glossary.md`).
//! - [`dice_pools`]: Exact discrete probability mass functions (`Pmf`), statistical moments
//!   (`DistributionStats`, Sarle's $\text{BC}$), and comparative dice pool topology benchmarks
//!   (`docs/ttrpg/01-foundation/probability_and_calibration.md` §1).
//! - [`dice_probabilities`] & [`contested_cascade`]: `2d20xN` Cascade dynamic-programming engines,
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
//! - [`characteristics`]: All 35 Physical, Mental, 6-trait Social (`COM`, `PRE`, `ELO`, `SAV`, `BEA`, `WRM`), Sensory, and 9 Psyche characteristics on the `1–20`
//!   Controlling Scale, 10 derived metrics (`POW`, `MMF`, `OODA`, `D_cog`, `AWR`, `FT`, `WSR`, `CMP`, `C_study`),
//!   and 9 Domain Aptitudes ($A \in [-2, +3]$)
//!   (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §1–§10).
//! - [`skills_trees`] & [`skills_catalog`]: Skill taxonomy (`TAL`, `PHY`, `KNO`, `PHY/KNO`), `[Access]` gates,
//!   the 8 Hierarchical Skill Trees (`CANONICAL_SKILL_TREES`), tree defaulting, and the 5-Factor Skill Point (`SP`)
//!   step cost formula (`docs/ttrpg/03-character/skills_and_trees.md` §1–§3,
//!   `docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7).
//! - [`lifepath_economy`], [`lifepath_catalog`], [`lifepath_hobbies_events`], & [`economy_gear`]: Chronological Lifepath creation (Stages 1–5),
//!   School Quality (`SQM`), `DIS + Mental + APT` advancement rolls, `12` Hobbies (`-15%` Experience SP club discount),
//!   `12` `2d20` Formative Life Events, Socioeconomic Upkeep (`SQM 1–10`),
//!   Black-Market `[Access]` procurement, and Operational `Heat`
//!   (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1–§3, `lifepath_events_hobbies_and_factions.md` §1–§2).
//! - [`psyche_social`] & [`social_combat`]: Psyche Friction Checks, Long-Term Psyche Drift, Conviction Surges,
//!   Combat Morale, Argument Vectors, Social Composure, and Cognitive Dissonance resolution
//!   (`docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §1–§4).
//! - [`combat`], [`combat_techniques`], [`ranged_combat`], [`ballistics`] & [`explosives_and_arsenal`]: Tactical Triad ("Envelope & Fill" — `Tempo`, `Control`, `Force`),
//!   Multi-Opponent splitting/flanking/elevation/object targeting, 30-entry Canonical Combat Technique Catalog,
//!   22-entry Melee & Shield Catalog, 17-entry Granular Firearms & Ammunition Physics ($E_k, I_{\text{KE}}, I_{\text{cal}}, W_{\text{yaw}}, \text{Acc}, \text{Rel}$),
//!   Specialty Shotgun Loads, Muscle/Less-Lethal Systems, Explosives & Breaching, Optics/Gear, Armor `PR` vs. `BABT`, and Malfunctions
//!   (`docs/ttrpg/05-combat/tactical_triad_and_pools.md`, `melee_combat.md`, `ranged_combat_and_armor.md`, `arsenal_explosives_and_tactical_gear.md`).
//! - [`health_survival`], [`status_and_triage`], [`environment_vehicles`] & [`environmental_hazards`]:
//!   `1d20` Hit Locations & Called Shots, Fatigue Levels (`0–5`), Asphyxiation, Structural Wounds (`Tiers 1–5`),
//!   Fractures, Hemorrhaging & Hypovolemic Shock, `WSR` Saves, Physiological & Mental Trauma Conditions,
//!   Combat Movement Gaits, Encumbrance (`Tiers 0–4`), Vehicular Chases & High-G Physics, and Environmental
//!   Survival (`docs/ttrpg/06-health-and-survival/health_and_trauma.md`, `movement_encumbrance_and_vehicles.md`, `environmental_hazards_and_survival.md`).
//! - [`engineering_design`], [`robotics_and_ai_design`] & [`electronic_warfare`]: Workshop Tiers (`FieldBenchImprovised` to
//!   `AerospaceCleanroomLab`), Project Clocks & Prototype Quirks, Custom Firearm/Ammo/Melee/Armor Designers,
//!   Modular Drones & UGVs, Chassis Trauma & EMP, AI 2.0 Architecture Engineering & Verification Debt,
//!   Clinical Cyber-Prosthetics & Myomer/Neural Augmentations, and 2035 Surveillance, Network Intrusion,
//!   ECM vs. ECCM & Custom EW Hardware
//!   (`docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md`, `robotics_drones_and_ai_architecture.md`, `electronic_warfare_and_surveillance.md`).
//! - [`walkthroughs`]: Canonical character lifepath builds (Mateo Vance, Dr. Priya Nair, Amara Okafor, Linnea Cho)
//!   (`docs/ttrpg/03-character/lifepaths_and_economy.md` §2.1–§2.3, `lifepath_events_hobbies_and_factions.md` §4).

pub mod action_catalog_stealth;
pub mod action_economy;
pub mod ballistics;
pub mod character_audit;
pub mod characteristics;
pub mod combat;
pub mod combat_techniques;
pub mod contested_cascade;
pub mod dice_pools;
pub mod dice_probabilities;
pub mod economy_gear;
pub mod electronic_warfare;
pub mod engineering_design;
pub mod environment_vehicles;
pub mod environmental_hazards;
pub mod explosives_and_arsenal;
pub mod health_survival;
pub mod lifepath_catalog;
pub mod lifepath_economy;
pub mod lifepath_hobbies_events;
pub mod primitives;
pub mod psyche_social;
pub mod ranged_combat;
pub mod robotics_and_ai_design;
pub mod skills_catalog;
pub mod skills_trees;
pub mod social_combat;
pub mod status_and_triage;
pub mod task_resolution;
pub mod walkthroughs;

pub use action_economy::*;
pub use character_audit::*;
pub use characteristics::*;
pub use combat::*;
pub use dice_pools::{
    SARLE_BIMODAL_THRESHOLD, try_median_gated_bimodal_pmf, try_order_statistic_pmf,
};
pub use dice_probabilities::*;
pub use electronic_warfare::*;
pub use engineering_design::*;
pub use health_survival::*;
pub use lifepath_economy::*;
pub use lifepath_hobbies_events::*;
pub use primitives::*;
pub use psyche_social::*;
pub use robotics_and_ai_design::*;
pub use satya_schema::CodedEnum;
pub use skills_trees::*;
pub use task_resolution::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_domain;
#[cfg(test)]
mod tests_phase2;
