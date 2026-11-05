//! # `satya-engine`
//!
//! Satya TTRPG game engine, exact combinatorial & dynamic-programming dice probability distributions,
//! Two-List Cascading ("Watershed" / "Recoil") task resolution, and odds calibration sweeps.
//!
//! ## Module Structure
//! - [`dice_probabilities`]: Exact PMF combinatorial/DP engines, summary moments (`DistributionStats`),
//!   and Four-Factor Two-List Cascade game-engine task evaluators (`CascadeTaskParams`, `CascadeRollOutcome`,
//!   `evaluate_cascade_roll`, `evaluate_satya_cascade_task`).
//! - [`task_resolution`]: Task Gate Classifications (`ClassA`/`B`/`C`), Tool Dependency & Quality tiers,
//!   Composite/Supporting Characteristic helpers, Complementary Synergy (`KNO -> PHY`), Dual-Skill Gates,
//!   Extended Project Clocks, and the 22-entry Master Task Requirements catalog.
//! - [`psyche_social`]: Psyche Friction Checks, Long-Term Psyche Drift, Conviction Surges, Combat Morale,
//!   Argument Vectors, Social Composure, and Cognitive Dissonance resolution.
//! - [`combat`]: Tactical Triad extensions, Melee Weapons, Reach & Clinch Polarity Flip, Kuzushi Throws,
//!   Ranged Fire Modes, Recoil Bracing, Suppression, Armor `PR` vs. `BABT`, and Weapon Malfunctions.
//! - [`rules_book`]: Canonical Satya TTRPG Rules Book catalog, Markdown-to-HTML renderer, multi-chapter search,
//!   and Combat Tactical Pool ("Envelope & Fill" — Tempo, Control, Force) calculator.

#![deny(missing_docs)]

#[allow(missing_docs)]
pub mod action_economy;
#[allow(missing_docs)]
pub mod characteristics;
pub mod combat;
pub mod dice_probabilities;
#[allow(missing_docs)]
pub mod lifepath_economy;
pub mod psyche_social;
pub mod rules_book;
#[allow(missing_docs)]
pub mod skills_trees;
pub mod task_resolution;

pub use action_economy::*;
pub use characteristics::*;
pub use combat::*;
pub use dice_probabilities::*;
pub use lifepath_economy::*;
pub use psyche_social::*;
pub use rules_book::*;
pub use skills_trees::*;
pub use task_resolution::*;

#[cfg(test)]
mod tests;
