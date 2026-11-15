//! Faction clock, Operational Heat, and jurisdiction economic-tier ledger DTOs.
//!
//! Companion to [`crate::protocol`], re-exported flatly via `pub use crate::world_clocks_dto::*;`.

use serde::{Deserialize, Serialize};

use crate::tactical_map_floors_dto::{EconomicTier, ZoningContext};

fn default_clock_segments() -> u8 {
    6
}

fn default_pack_awakened() -> String {
    "awakened".to_string()
}

/// Progress clock, party standing, and secret GM agenda for a setting faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionClockRecord {
    pub faction_id: String,
    pub faction_name: String,
    pub clock_title: String,
    #[serde(default)]
    pub progress: u8,
    #[serde(default = "default_clock_segments")]
    pub segments: u8,
    #[serde(default)]
    pub party_standing: i32,
    #[serde(default)]
    pub public_rumor: String,
    /// Exposed strictly to `GM` / `AiGm` / Solo-mode viewers; stripped for players.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gm_secret_agenda: Option<String>,
}

/// Session-persistent world state coupling Operational `Heat` (`0..=10`), neighborhood
/// [`EconomicTier`] × [`ZoningContext`], and active faction momentum clocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWorldLedger {
    pub session_id: String,
    #[serde(default)]
    pub campaign_tag: String,
    #[serde(default = "default_pack_awakened")]
    pub pack_id: String,
    #[serde(default)]
    pub operational_heat: u8,
    #[serde(default)]
    pub heat_tier_code: String,
    #[serde(default)]
    pub heat_public_check_delta_ext: i32,
    #[serde(default)]
    pub legal_bank_accounts_frozen: bool,
    #[serde(default)]
    pub heat_description: String,
    #[serde(default)]
    pub active_economic_tier: EconomicTier,
    #[serde(default)]
    pub active_zoning: ZoningContext,
    #[serde(default)]
    pub jurisdiction_response_profile: String,
    #[serde(default)]
    pub black_market_delta_ext_mod: i32,
    #[serde(default)]
    pub faction_clocks: Vec<FactionClockRecord>,
    #[serde(default)]
    pub recent_ledger_events: Vec<String>,
    #[serde(default)]
    pub updated_at_us: i64,
}

/// Additive request payload to mutate Operational Heat, active sector jurisdiction, or a faction clock.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateWorldLedgerRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heat_delta: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_heat: Option<u8>,
    /// Named incident severity (`"loud_firefight"`, `"cyber_breach"`, `"public_spectacle"`, `"minor"`, `"lay_low"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incident_severity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub economic_tier: Option<EconomicTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoning: Option<ZoningContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_delta: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing_delta: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_rumor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gm_secret_agenda: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Response returned after updating or seeding a [`SessionWorldLedger`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLedgerUpdateResponse {
    pub ledger: SessionWorldLedger,
    #[serde(default)]
    pub triggered_events: Vec<String>,
}
