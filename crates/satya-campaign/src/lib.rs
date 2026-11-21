//! # `satya-campaign`
//!
//! *Awakened* 2035 setting-specific campaign systems, AI 2.0 split-brain substrate & conspiracy
//! arcs, 5 Faction / 14 Archetype templates, party motivational hook weaving, prompt-driven NPC
//! generation, and role-redacted world clocks (`docs/campaign/`).

pub mod awakened_ai_substrate;
pub mod awakened_campaign_arcs;
pub mod faction_templates;
pub mod hook_weaver;
pub mod hook_weaver_lifepath;
pub mod npc_generator;
pub mod world_clocks;

pub use awakened_ai_substrate::*;
pub use awakened_campaign_arcs::*;
pub use faction_templates::*;
pub use hook_weaver::*;
pub use hook_weaver_lifepath::*;
pub use npc_generator::*;
pub use world_clocks::*;

#[cfg(test)]
mod tests;
