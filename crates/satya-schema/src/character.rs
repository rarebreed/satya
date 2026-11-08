//! Canonical Character Sheet, Multimodal Asset, Anti-Cheat Provenance, and Avro Event types
//! (`com.satya.events.CharacterSheetEvent`, `schema_id = 11`, topic `character-sheets`).

use crate::types::{EventMetadata, Participant};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Lifecycle or portability operation recorded by a [`CharacterSheetEvent`] on the `character-sheets` topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacterEventType {
    #[default]
    CharacterCreated,
    CharacterUpdated,
    CharacterExported,
    CharacterImported,
    CharacterBoundToSession,
    CharacterAssetUploaded,
}

/// Multimodal asset classification attached to a character sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacterAssetKind {
    #[default]
    PortraitImage,
    TacticalToken,
    DossierPdf,
    #[serde(rename = "MODEL_3D_GLB", alias = "MODEL3D_GLB")]
    Model3dGlb,
}

impl CharacterAssetKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PortraitImage => "PORTRAIT_IMAGE",
            Self::TacticalToken => "TACTICAL_TOKEN",
            Self::DossierPdf => "DOSSIER_PDF",
            Self::Model3dGlb => "MODEL_3D_GLB",
        }
    }

    #[must_use]
    pub fn from_code_str(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "TACTICAL_TOKEN" | "TOKEN" => Self::TacticalToken,
            "DOSSIER_PDF" | "PDF" => Self::DossierPdf,
            "MODEL_3D_GLB" | "GLB" | "MODEL_3D" => Self::Model3dGlb,
            _ => Self::PortraitImage,
        }
    }
}

/// Metadata reference to a multimodal asset attached to a [`CharacterSheetRecord`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterAssetRef {
    pub asset_id: String,
    pub asset_kind: CharacterAssetKind,
    pub filename: String,
    pub mime_type: String,
    pub sha256_hex: String,
    pub size_bytes: i64,
    pub created_at_us: i64,
}

/// Two-layer anti-cheat & ruleset legality status for a character sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacterVerificationStatus {
    /// Signed by a Satya server authority AND passes all `satya-engine` Lifepath/SP/DAG legality checks.
    #[default]
    VerifiedOfficial,
    /// Created offline or customized without a matching server key, but passes 100% of `satya-engine` Lifepath/SP/DAG legality checks.
    LegalCustom,
    /// Cryptographic signature invalid OR violates `satya-engine` characteristic, SP budget, or 4-gate skill DAG constraints.
    TamperedOrOverBudget,
}

impl CharacterVerificationStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerifiedOfficial => "VERIFIED_OFFICIAL",
            Self::LegalCustom => "LEGAL_CUSTOM",
            Self::TamperedOrOverBudget => "TAMPERED_OR_OVER_BUDGET",
        }
    }

    #[must_use]
    pub fn from_code_str(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "VERIFIED_OFFICIAL" => Self::VerifiedOfficial,
            "LEGAL_CUSTOM" => Self::LegalCustom,
            _ => Self::TamperedOrOverBudget,
        }
    }
}

/// Serializable 30-characteristic profile (`1..=20` scale, plus `hgt_m` and `mas_kg`) stored inside [`CharacterSheetRecord`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterCharacteristicsRecord {
    pub spd: i32,
    pub for_stat: i32,
    pub dex: i32,
    pub kin: i32,
    pub hlt: i32,
    pub fit: i32,
    pub hgt_m: f64,
    pub mas_kg: f64,
    pub mem: i32,
    pub ana: i32,
    pub ins: i32,
    pub cre: i32,
    pub dis: i32,
    pub foc: i32,
    pub com: i32,
    pub pre: i32,
    pub elo: i32,
    pub sig: i32,
    pub hea: i32,
    pub tac: i32,
    pub olf: i32,
    pub tas: i32,
    pub itu: i32,
    pub ego: i32,
    pub slf: i32,
    pub opn: i32,
    pub blg: i32,
    pub ext: i32,
    pub cnf: i32,
    pub agg: i32,
}

impl Default for CharacterCharacteristicsRecord {
    fn default() -> Self {
        Self {
            spd: 10, for_stat: 10, dex: 10, kin: 10, hlt: 10, fit: 10,
            hgt_m: 1.75, mas_kg: 70.0,
            mem: 10, ana: 10, ins: 10, cre: 10, dis: 10, foc: 10,
            com: 10, pre: 10, elo: 10,
            sig: 10, hea: 10, tac: 10, olf: 10, tas: 10, itu: 10,
            ego: 10, slf: 10, opn: 10, blg: 10, ext: 10, cnf: 10, agg: 10,
        }
    }
}

/// Serializable 9-domain aptitude profile (`-2..=+3`) stored inside [`CharacterSheetRecord`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CharacterAptitudesRecord {
    pub apt_qnt: i32,
    pub apt_lng: i32,
    pub apt_bio: i32,
    pub apt_mec: i32,
    pub apt_cmp: i32,
    pub apt_kin: i32,
    pub apt_fin: i32,
    pub apt_art: i32,
    pub apt_soc: i32,
}

/// Per-zone anatomical trauma record (`1..=13`) stored inside [`CharacterSheetRecord`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnatomicalZoneRecord {
    pub location_code: String,
    pub wound_tier: i32,
    pub bone_status: String,
    pub bleed_grade: i32,
}

/// Complete persistent Character Sheet record (`com.satya.types.CharacterSheetRecord`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheetRecord {
    pub character_id: String,
    pub owner_user_id: String,
    pub version: i32,
    pub character_name: String,
    pub callsign_or_role: String,
    pub age_years: i32,
    pub origin_label: String,
    pub campaign_sqm: i32,
    pub operational_heat: i32,
    pub granted_access_tags: Vec<String>,
    pub stage_history: Vec<String>,
    pub characteristics: CharacterCharacteristicsRecord,
    pub aptitudes: CharacterAptitudesRecord,
    pub skills: BTreeMap<String, i32>,
    pub equipped_ranged_ids: Vec<String>,
    pub equipped_melee_codes: Vec<String>,
    pub equipped_armor_codes: Vec<String>,
    pub field_gear_ids: Vec<String>,
    pub hit_zones: Vec<AnatomicalZoneRecord>,
    pub fatigue_level: i32,
    pub exertion_marks: i32,
    pub wound_shock_index: i32,
    pub blood_units_lost: f64,
    pub active_status_conditions: Vec<String>,
    pub morale_state: String,
    pub mental_trauma: Option<String>,
    pub social_composure_lost: i32,
    pub social_wound: Option<String>,
    pub psyche_drift_marks: i32,
    pub notes: Option<String>,
    /// Optional JSON-serialized `LifepathBuildConfig` for full deterministic SP ledger auditing.
    pub lifepath_config_json: Option<String>,
    #[serde(default)]
    pub assets: Vec<CharacterAssetRef>,
    #[serde(default)]
    pub verification_status: CharacterVerificationStatus,
    #[serde(default = "default_true")]
    pub sp_budget_valid: bool,
    #[serde(default = "default_true")]
    pub dag_prereqs_valid: bool,
    #[serde(default)]
    pub audit_violations: Vec<String>,
    #[serde(default)]
    pub content_sha256: String,
    #[serde(default)]
    pub server_signature_hex: Option<String>,
    #[serde(default)]
    pub updated_at_us: i64,
}

fn default_true() -> bool {
    true
}

impl Default for CharacterSheetRecord {
    fn default() -> Self {
        Self {
            character_id: String::new(),
            owner_user_id: String::new(),
            version: 1,
            character_name: "Unnamed Operator".to_string(),
            callsign_or_role: "Field Specialist".to_string(),
            age_years: 28,
            origin_label: "Working-Class Standard (SQM 4)".to_string(),
            campaign_sqm: 5,
            operational_heat: 0,
            granted_access_tags: vec!["UNIVERSAL".to_string()],
            stage_history: Vec::new(),
            characteristics: CharacterCharacteristicsRecord::default(),
            aptitudes: CharacterAptitudesRecord::default(),
            skills: BTreeMap::new(),
            equipped_ranged_ids: Vec::new(),
            equipped_melee_codes: Vec::new(),
            equipped_armor_codes: Vec::new(),
            field_gear_ids: Vec::new(),
            hit_zones: Vec::new(),
            fatigue_level: 0,
            exertion_marks: 0,
            wound_shock_index: 0,
            blood_units_lost: 0.0,
            active_status_conditions: Vec::new(),
            morale_state: "STEADY".to_string(),
            mental_trauma: None,
            social_composure_lost: 0,
            social_wound: None,
            psyche_drift_marks: 0,
            notes: None,
            lifepath_config_json: None,
            assets: Vec::new(),
            verification_status: CharacterVerificationStatus::VerifiedOfficial,
            sp_budget_valid: true,
            dag_prereqs_valid: true,
            audit_violations: Vec::new(),
            content_sha256: String::new(),
            server_signature_hex: None,
            updated_at_us: 0,
        }
    }
}

/// Avro event (`schema_id = 11`) emitted to the `character-sheets` topic on every character create,
/// update, export, import, session bind, and multimodal asset upload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheetEvent {
    pub metadata: EventMetadata,
    pub character_event_type: CharacterEventType,
    pub actor: Participant,
    pub character: CharacterSheetRecord,
}

/// Request payload for `POST /api/characters`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveCharacterRequest {
    pub character: CharacterSheetRecord,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub bind_to_session: bool,
}

/// Response payload returned by `POST /api/characters` and `POST /api/characters/import`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveCharacterResponse {
    pub character: CharacterSheetRecord,
    pub local_file_path: String,
    pub satyachar_base64: String,
}

/// Summary item returned in `GET /api/characters`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSummaryItem {
    pub character_id: String,
    pub character_name: String,
    pub callsign_or_role: String,
    pub age_years: i32,
    pub origin_label: String,
    pub version: i32,
    pub verification_status: CharacterVerificationStatus,
    pub asset_count: usize,
    pub updated_at_us: i64,
}

/// Response payload for `GET /api/characters`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterListResponse {
    pub local_directory: String,
    pub characters: Vec<CharacterSheetRecord>,
}

/// Request payload for `POST /api/characters/import`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportCharacterRequest {
    pub satyachar_base64: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub bind_to_session: bool,
}

/// Request payload for `POST /api/characters/:id/bind-session`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindCharacterSessionRequest {
    pub session_id: String,
}

/// Request payload for `POST /api/characters/:id/assets`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UploadCharacterAssetRequest {
    pub asset_kind: CharacterAssetKind,
    pub filename: String,
    pub mime_type: String,
    pub data_base64: String,
    #[serde(default)]
    pub session_id: Option<String>,
}
