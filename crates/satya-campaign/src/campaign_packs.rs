//! Canonical single-source `config/campaign.toml` configuration model & embedded fallback.
//!
//! Embeds `config/campaign.toml` once at compile time (`DEFAULT_CAMPAIGN_TOML`) so both Rust
//! services (`satya-signaling`) and Python orchestrators (`satya_py` -> `satya_ml.campaign_config`)
//! share one authoritative source for active pack defaults without duplicated fallback tables.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Embedded copy of the workspace `config/campaign.toml`.
pub const DEFAULT_CAMPAIGN_TOML: &str = include_str!("../../../config/campaign.toml");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StarterScenarioSpec {
    #[serde(default)]
    pub episode_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub campaign_premise: String,
    #[serde(default)]
    pub planned_beats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PackTokenSpec {
    #[serde(default)]
    pub token_id: String,
    #[serde(default)]
    pub entity_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub model_preset: String,
    #[serde(default)]
    pub color_hex: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PackCombatSpawnSpec {
    #[serde(default)]
    pub entity_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub role_or_faction: String,
    #[serde(default)]
    pub ooda_score: i32,
    #[serde(default)]
    pub zone_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PackZoneLabelsSpec {
    #[serde(default)]
    pub hostile: String,
    #[serde(default)]
    pub player: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PackAiPlayerSpec {
    #[serde(default)]
    pub character_id: String,
    #[serde(default)]
    pub character_name: String,
    #[serde(default)]
    pub callsign_or_role: String,
    #[serde(default)]
    pub archetype: String,
    #[serde(default)]
    pub faction: String,
    #[serde(default)]
    pub characteristics: BTreeMap<String, i32>,
    #[serde(default)]
    pub attribute_tns: BTreeMap<String, i32>,
    #[serde(default)]
    pub skills: BTreeMap<String, i32>,
    #[serde(default)]
    pub combat_stance: String,
    #[serde(default)]
    pub fatigue_level: i32,
    #[serde(default)]
    pub wound_shock_index: f64,
    #[serde(default)]
    pub loadout: Vec<String>,
    #[serde(default)]
    pub biography: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CampaignPackSpec {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub default_sector_preset: String,
    #[serde(default)]
    pub allow_scifi_sector_presets: bool,
    #[serde(default)]
    pub lore_dirs: Vec<String>,
    #[serde(default)]
    pub npc_toml_file: String,
    #[serde(default)]
    pub factions: Vec<String>,
    #[serde(default)]
    pub starter_scenario: StarterScenarioSpec,
    #[serde(default)]
    pub default_ally_token: PackTokenSpec,
    #[serde(default)]
    pub default_hostile_token: PackTokenSpec,
    #[serde(default)]
    pub default_combat_spawn: PackCombatSpawnSpec,
    #[serde(default)]
    pub default_zone_labels: PackZoneLabelsSpec,
    #[serde(default)]
    pub default_ai_player: PackAiPlayerSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignConfigFile {
    #[serde(default = "default_active_pack_string")]
    pub active_pack: String,
    #[serde(default)]
    pub packs: BTreeMap<String, CampaignPackSpec>,
}

fn default_active_pack_string() -> String {
    "awakened".to_string()
}

/// Parses a `campaign.toml` payload into [`CampaignConfigFile`].
pub fn parse_campaign_config_toml(raw: &str) -> Result<CampaignConfigFile, String> {
    toml::from_str::<CampaignConfigFile>(raw).map_err(|e| format!("Invalid campaign.toml: {e}"))
}

/// Returns the parsed compile-time embedded `config/campaign.toml` singleton.
pub fn embedded_campaign_config() -> &'static CampaignConfigFile {
    static EMBEDDED: OnceLock<CampaignConfigFile> = OnceLock::new();
    EMBEDDED.get_or_init(|| {
        parse_campaign_config_toml(DEFAULT_CAMPAIGN_TOML)
            .expect("embedded config/campaign.toml must parse cleanly")
    })
}

/// Returns the default `active_pack` identifier configured in embedded `config/campaign.toml`.
#[must_use]
pub fn default_active_pack_name() -> &'static str {
    embedded_campaign_config().active_pack.as_str()
}

/// Looks up a pack definition from the embedded `config/campaign.toml` (falling back to the active pack).
#[must_use]
pub fn embedded_pack_spec(pack_name: &str) -> Option<&'static CampaignPackSpec> {
    let cfg = embedded_campaign_config();
    let key = pack_name.trim().to_ascii_lowercase();
    cfg.packs
        .get(&key)
        .or_else(|| cfg.packs.get(cfg.active_pack.as_str()))
        .or_else(|| cfg.packs.values().next())
}

/// Loads and parses a `config/campaign.toml` file from disk on non-wasm targets.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn load_campaign_config_from_path(path: &std::path::Path) -> Option<CampaignConfigFile> {
    let raw = std::fs::read_to_string(path).ok()?;
    parse_campaign_config_toml(&raw).ok()
}

#[cfg(test)]
mod pack_tests {
    use super::*;

    #[test]
    fn embedded_campaign_config_parses_awakened_and_space_opera_packs() {
        let cfg = embedded_campaign_config();
        assert_eq!(cfg.active_pack, "awakened");
        let awakened = cfg.packs.get("awakened").expect("awakened pack");
        assert_eq!(awakened.default_sector_preset, "community_clinic_annex");
        assert!(!awakened.allow_scifi_sector_presets);
        assert_eq!(awakened.default_ai_player.character_name, "Mateo Vance");
        assert!(!awakened.starter_scenario.planned_beats.is_empty());

        let space_opera = cfg.packs.get("space_opera").expect("space_opera pack");
        assert_eq!(space_opera.default_sector_preset, "orbital_airlock");
        assert!(space_opera.allow_scifi_sector_presets);
        assert_eq!(space_opera.default_ai_player.character_name, "Kaelen Voss");
    }
}
