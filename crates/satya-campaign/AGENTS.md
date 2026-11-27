# `satya-campaign` Agent Coding & Architecture Rules

Activate [`satya-rust-engine`](../../.agents/skills/satya-rust-engine/SKILL.md) and [`satya-rust-workspace`](../../.agents/skills/satya-rust-workspace/SKILL.md) before modifying `crates/satya-campaign`.

1. **Owned Modules (`docs/campaign/` & *Awakened* 2035 Setting)**:
   - `awakened_ai_substrate.rs`: Split-brain AI 2.0 substrates, `L0`/`L1` probe/clamp liberation, captive hunter-AI defection, and Bayesian sentience ambiguity (`SyntheticMetabolicMode` is re-exported from `satya_engine::robotics_and_ai_design`).
   - `awakened_campaign_arcs.rs`: Three-Arc conspiracy ledger, polycrisis grid/water strain, tradecraft channels, and dissenter/proxy hooks.
   - `faction_templates.rs`: 6 *Awakened* Factions (`AwakenedFactionId`) and 16 Archetypes (`FactionArchetypeId`).
   - `hook_weaver.rs` & `hook_weaver_lifepath.rs`: `PartyHookManifest`, `compile_party_hooks`, and lifepath/hobby/event/faction wedge compilation (parsing `LifepathBuildConfig.faction_archetype: Option<String>` via `FactionArchetypeId::from_code_str`).
   - `npc_generator.rs`: Pack-aware (`"awakened"` default vs. `"space_opera"`) prompt/template NPC generation and `NpcMotivationalProfile` derived from the 9 bipolar Psyche traits (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`, `TRS`, `CPS`).
   - `world_clocks.rs`: Persistent faction progress clocks, `OperationalHeat` (`0..=10`) x `EconomicTier` / `ZoningContext` modifiers, and role-based redaction (`SessionWorldLedger`, `filter_world_ledger_for_viewer`).
2. **Scoped Verification**:
   - Run `pixi run test-campaign` (and `pixi run check-wasm` when editing types consumed by `satya-web`).
