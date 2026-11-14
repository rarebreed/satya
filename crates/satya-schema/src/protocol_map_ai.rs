//! Tactical map state, AI GM / AI Player turn triggers, rules search, NPC, and storage metrics DTOs.
//!
//! Companion to [`crate::protocol`], re-exported flatly via `pub use crate::protocol_map_ai::*;`.

use crate::events::{
    AiGmOperatingMode, ChannelContext, CombatStance, MessageVisibility, TokenPlacement,
};
use crate::protocol::{GmMemoryNoteRecord, default_true_flag};
use crate::tactical_map_dto::*;
use crate::types::{Participant, SessionMode};
use serde::{Deserialize, Serialize};

pub(crate) fn default_control_c() -> i32 {
    10
}

pub(crate) fn default_skill_level_n() -> usize {
    3
}

/// HTTP request body for `POST /api/gm/sessions/:session_id/roll`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GmRollRequest {
    /// Display name of the NPC or environmental hazard performing the check.
    pub actor_display_name: String,
    /// Optional target participant `user_id`s affected by the roll.
    #[serde(default)]
    pub target_user_ids: Vec<String>,
    /// Name of the Satya skill being tested.
    pub skill_name: String,
    /// Controlling Characteristic $C \in [1, 20]$ (shifts $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$; also accepts `control_c`).
    #[serde(default = "default_control_c", alias = "control_c")]
    pub attribute_tn: i32,
    /// Skill Level $N \in [1, 10]$ (number of `2d20` pairs rolled; also accepts `skill_level` or `skill_level_n`).
    #[serde(
        default = "default_skill_level_n",
        alias = "skill_level",
        alias = "skill_level_n"
    )]
    pub pool_size: usize,
    /// Known Player Tool Modifier $\Delta_{\text{tool}} \in [-4, +12]$ (shifts $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$).
    #[serde(default, alias = "delta_tool", alias = "tool_mod")]
    pub known_tool_mod: i32,
    /// External Task Modifier $\Delta_{\text{ext}} \in [-10, +16]$ (shifts $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$ in contested mode; also accepts `external_mod` or `delta_ext`).
    #[serde(default, alias = "external_mod", alias = "delta_ext")]
    pub difficulty_rating: i32,
    /// Optional Internal Character Modifier $\Delta_{\text{int}} \in [-2, +10]$ (defaults to `fatigue_level` when omitted).
    #[serde(default)]
    pub internal_mod: Option<i32>,
    /// Optional over-target step carry $\delta_{\text{over}}$ (`6` standard, `8` when boosted by Extraordinary `KNO -> PHY` synergy).
    #[serde(default)]
    pub delta_over: Option<i32>,
    /// Optional GM resistance mode (`"tiered_single_pair"` default, `"full_cascade"`, or `"uncontested"`).
    #[serde(default)]
    pub gm_resistance_mode: Option<String>,
    /// Optional contested tie-breaking rule (`"hybrid_signed_margin"` default, `"strict_signed_margin"`, or `"active_player_wins_ties"`).
    #[serde(default)]
    pub tie_rule: Option<String>,
    /// Optional cascade carry-over mode (`"markov_step"`, `"cumulative_snowball"`, or `"linear_margin"`).
    #[serde(default)]
    pub carry_mode: Option<String>,
    /// Optional botch severity rule (`"closest_miss"`, `"ceiling_breach"`, or `"overrun_k"`).
    #[serde(default)]
    pub botch_rule: Option<String>,
    /// Optional Satya combat stance.
    #[serde(default)]
    pub combat_stance: Option<CombatStance>,
    /// Accumulated internal penalty / fatigue level $\Delta_{\text{int}}$ of the acting NPC.
    #[serde(default, alias = "delta_int")]
    pub fatigue_level: i32,
    /// Accumulated wound shock index of the acting NPC.
    #[serde(default)]
    pub wound_shock_index: f32,
    /// Optional canonical combat action/technique code from `COMBAT_TECHNIQUE_CATALOG` (e.g. `"punch"`, `"snap_shot"`).
    #[serde(default)]
    pub combat_technique: Option<String>,
    /// Optional Tactical Triad Tempo (`T`) pool allocation.
    #[serde(default)]
    pub tempo_pool: Option<i32>,
    /// Optional Tactical Triad Control (`C`) pool allocation.
    #[serde(default)]
    pub control_pool: Option<i32>,
    /// Optional Tactical Triad Force (`F`) pool allocation.
    #[serde(default)]
    pub force_pool: Option<i32>,
    /// Optional adjudication notes.
    #[serde(default)]
    pub notes: Option<String>,
}

/// HTTP request body for `POST /api/gm/sessions/:session_id/map` (and `ServerWsPacket::MapUpdated` state).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GmMapUpdateRequest {
    /// Unique identifier for the tactical map.
    pub map_id: String,
    /// Human-readable map title.
    pub title: String,
    /// URL or `data:image/svg+xml` fallback URI for the tactical map.
    pub image_url: String,
    /// Tactical grid width in cells (`1 cell = 1.0m` by default).
    pub grid_width: i32,
    /// Tactical grid height in cells (`1 cell = 1.0m` by default).
    pub grid_height: i32,
    /// Active character and NPC token coordinates on the grid.
    #[serde(default)]
    pub tokens: Vec<TokenPlacement>,
    /// Optional terrain, cover, or environmental hazard notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Physical cell edge scale in centimeters (`100 cm = 1.00 m`).
    #[serde(default = "default_cell_size_cm")]
    pub cell_size_cm: i32,
    /// Optional procedural sector preset identifier (`"orbital_airlock"`, `"docking_bay"`, etc.).
    #[serde(default)]
    pub theme_preset: Option<String>,
    /// Optional deterministic procedural generation seed.
    #[serde(default)]
    pub seed: Option<i64>,
    /// Optional legacy row-major terrain/cover tiles (empty for tile-free continuous vector maps).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tiles: Vec<TacticalTileKind>,
    /// Optional 3D `.glb` miniature model, color, facing, and pose overrides per `entity_id`.
    #[serde(default)]
    pub token_visuals: Vec<TokenVisualSpec>,
    /// Thin walls, doors, and windows on cell-boundary edges (empty for legacy cell-wall maps).
    #[serde(default)]
    pub wall_edges: Vec<MapWallEdge>,
    /// Multi-cell props whose mechanical footprint is already stamped into `tiles`.
    #[serde(default)]
    pub props: Vec<MapPropPlacement>,
    /// Smooth continuous vector ground splines and organic polygons (`AsphaltRoad`, `CreekChannel`, `MulchZone`, `GardenBedZone`, etc.).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surfaces: Vec<MapSurfaceFeature>,
    /// Multi-level architectural floors (`B1`, `1F`, `2F`, `Roof`) when present.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub floors: Vec<MapFloorLevel>,
    /// Currently active floor index (`0` = Ground / `1F`, `-1` = `B1`, `1` = `2F`).
    #[serde(default)]
    pub active_floor: i32,
}

impl Default for GmMapUpdateRequest {
    fn default() -> Self {
        Self {
            map_id: "map-default".to_string(),
            title: "Tactical Sector".to_string(),
            image_url: String::new(),
            grid_width: 20,
            grid_height: 14,
            tokens: Vec::new(),
            notes: None,
            cell_size_cm: default_cell_size_cm(),
            theme_preset: None,
            seed: None,
            tiles: Vec::new(),
            token_visuals: Vec::new(),
            wall_edges: Vec::new(),
            props: Vec::new(),
            surfaces: Vec::new(),
            floors: Vec::new(),
            active_floor: 0,
        }
    }
}

impl GmMapUpdateRequest {
    /// Physical tactical scale unit in meters (`1.0` default).
    #[must_use]
    pub fn cell_size_m(&self) -> f32 {
        (self.cell_size_cm.max(25) as f32) / 100.0
    }

    /// Physical board width and height in centimeters.
    #[must_use]
    pub fn board_size_cm(&self) -> (i32, i32) {
        let s = self.cell_size_cm.max(25);
        (self.grid_width.max(1) * s, self.grid_height.max(1) * s)
    }

    /// Physical board width and height in meters.
    #[must_use]
    pub fn board_size_m(&self) -> (f32, f32) {
        let (w_cm, h_cm) = self.board_size_cm();
        (w_cm as f32 / 100.0, h_cm as f32 / 100.0)
    }

    /// Looks up the legacy [`TacticalTileKind`] at `(x, y)` (`Floor` when `tiles` is empty or out of range).
    #[must_use]
    pub fn tile_at(&self, x: i32, y: i32) -> TacticalTileKind {
        if x < 0 || y < 0 || x >= self.grid_width || y >= self.grid_height {
            return TacticalTileKind::FullWall;
        }
        let idx = (y * self.grid_width + x) as usize;
        self.tiles
            .get(idx)
            .copied()
            .unwrap_or(TacticalTileKind::Floor)
    }

    /// Looks up the [`TokenVisualSpec`] for `entity_id`, if present.
    #[must_use]
    pub fn visual_for(&self, entity_id: &str) -> Option<&TokenVisualSpec> {
        self.token_visuals
            .iter()
            .find(|v| v.entity_id == entity_id)
    }

    /// Authoritative continuous center coordinate `(x_cm, y_cm)` for `tok`, falling back to the
    /// center of `(tok.x, tok.y)` when no sub-meter coordinate override is set on its visual spec.
    #[must_use]
    pub fn token_pos_cm(&self, tok: &TokenPlacement) -> (i32, i32) {
        let cell_cm = self.cell_size_cm.max(25);
        let vis = self.visual_for(&tok.entity_id);
        let x_cm = vis
            .and_then(|v| v.pos_x_cm)
            .unwrap_or(tok.x * cell_cm + cell_cm / 2);
        let y_cm = vis
            .and_then(|v| v.pos_y_cm)
            .unwrap_or(tok.y * cell_cm + cell_cm / 2);
        (x_cm, y_cm)
    }

    /// Authoritative continuous center coordinate `(x_m, y_m)` in meters for `tok`.
    #[must_use]
    pub fn token_pos_m(&self, tok: &TokenPlacement) -> (f32, f32) {
        let (x_cm, y_cm) = self.token_pos_cm(tok);
        (x_cm as f32 / 100.0, y_cm as f32 / 100.0)
    }

    /// Synchronizes both continuous centimeter coordinates (`pos_x_cm`, `pos_y_cm` on
    /// [`TokenVisualSpec`]) and legacy integer meter indices (`tok.x`, `tok.y` on [`TokenPlacement`])
    /// for `entity_id` so Avro v1 logs and UI stay consistent.
    pub fn set_token_pos_cm(&mut self, entity_id: &str, x_cm: i32, y_cm: i32) {
        let cell_cm = self.cell_size_cm.max(25);
        let max_gx = (self.grid_width - 1).max(0);
        let max_gy = (self.grid_height - 1).max(0);
        let gx = (x_cm / cell_cm).clamp(0, max_gx);
        let gy = (y_cm / cell_cm).clamp(0, max_gy);
        if let Some(tok) = self.tokens.iter_mut().find(|t| t.entity_id == entity_id) {
            tok.x = gx;
            tok.y = gy;
        }
        if let Some(vis) = self
            .token_visuals
            .iter_mut()
            .find(|v| v.entity_id == entity_id)
        {
            vis.pos_x_cm = Some(x_cm);
            vis.pos_y_cm = Some(y_cm);
        } else {
            self.token_visuals.push(TokenVisualSpec {
                entity_id: entity_id.to_string(),
                pos_x_cm: Some(x_cm),
                pos_y_cm: Some(y_cm),
                ..Default::default()
            });
        }
    }
}

/// Webhook payload dispatched from `satya-signaling` to the Python LangGraph service
/// at `POST /internal/ai-gm/events`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiGmTurnTriggerRequest {
    /// Active session room identifier.
    pub session_id: String,
    /// Session business mode (`TTRPG`).
    pub session_mode: SessionMode,
    /// Operating mode (`AUTONOMOUS` for `TTRPG_AI_GM`, `CO_PILOT` for `TTRPG_GM_AI`).
    pub operating_mode: AiGmOperatingMode,
    /// `user_id` of the human Game Master if operating in `CO_PILOT` mode.
    pub human_gm_user_id: Option<String>,
    /// `event_id` or `message_id` of the triggering chat message or mechanics roll.
    pub trigger_event_id: String,
    /// Triggering event category (`"CHAT"` or `"MECHANICS"`).
    pub trigger_kind: String,
    /// Human [`Participant`] who triggered the AI GM turn.
    pub trigger_actor: Participant,
    /// Visibility of the triggering message (`PUBLIC`, `PRIVATE`, `GM_ONLY`).
    pub visibility: MessageVisibility,
    /// Optional semantic channel context of the triggering message (`IN_CHARACTER`, `OUT_OF_CHARACTER`, etc.).
    #[serde(default)]
    pub channel_context: Option<ChannelContext>,
    /// Formatted text of the player's or GM's action/roll.
    pub context_text: String,
    /// Current session roster snapshot.
    pub roster: Vec<Participant>,
    /// Configured LLM provider (`"ollama"`, `"google_genai"`, `"openai"`, `"anthropic"`).
    pub provider: String,
    /// Configured reasoning model ID.
    pub model_id: String,
    /// Optional multimodal vision model ID.
    pub vision_model_id: Option<String>,
    /// Sampling temperature.
    pub temperature: f32,
    /// Persona identifier.
    pub persona_id: String,
    /// Optional custom LLM base URL.
    pub base_url: Option<String>,
    /// Optional 32-character lowercase hex W3C `trace_id` correlating this turn across Rust and Python.
    #[serde(default)]
    pub trace_id: Option<String>,
    /// Optional 16-character lowercase hex caller `span_id` from `satya-signaling`.
    #[serde(default)]
    pub parent_span_id: Option<String>,
    /// Optional upstream data artifact lineage ID (`trigger_event_id`).
    #[serde(default)]
    pub lineage_id: Option<String>,
}

/// Client-side view model of per-topic Apache Iggy & Avro storage metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicMetricsView {
    /// Apache Iggy topic name.
    pub topic: String,
    /// Numeric topic identifier.
    pub topic_id: u32,
    /// Configured storage sink backend (`"local"` or `"s3"`).
    pub storage_backend: String,
    /// Target filesystem path or `s3://` URI prefix.
    pub destination_uri: String,
    /// Total number of events ingested into this topic.
    pub total_messages_ingested: u64,
    /// Number of events currently buffered in memory awaiting batch flush.
    pub buffered_messages: usize,
    /// Total number of `.avro` Object Container batch files flushed for this topic.
    pub flushed_batches_count: u64,
    /// Highest Apache Iggy offset persisted to an `.avro` batch file, if any.
    pub last_flushed_offset: Option<u64>,
    /// Active Avro schema version number for this topic.
    pub active_schema_version: u16,
    /// Hex-encoded `CRC-64-AVRO` fingerprint of the active schema.
    pub active_schema_fingerprint_hex: String,
}

/// Wrapper response returned by `GET /api/storage/metrics`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageMetricsViewResponse {
    /// Metrics snapshots for all configured Apache Iggy topics.
    pub topics: Vec<TopicMetricsView>,
}

/// Metadata summary for a single chapter in the Satya TTRPG Rules Book (`docs/ttrpg/`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesChapterSummary {
    /// Canonical chapter slug identifier (e.g. `"satya_d20_rules"`, `"tasks"`, `"combat"`, `"melee_combat"`).
    pub chapter_id: String,
    /// Book part/section grouping label (e.g. `"Part II: Core 2d20 Watershed Resolution & Probability"`).
    pub part: String,
    /// Human-readable chapter title.
    pub title: String,
    /// Relative path inside `docs/ttrpg/` (e.g. `"combat/melee_combat.md"`).
    pub relative_path: String,
    /// Concise summary of the rules covered in this chapter.
    pub description: String,
    /// Top-level `##` and `###` section headings extracted from the chapter markdown.
    #[serde(default)]
    pub headings: Vec<String>,
}

/// Full Satya TTRPG Rules Book chapter payload returned by `GET /api/rules/:chapter_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesChapterDocument {
    /// Chapter metadata summary.
    pub summary: RulesChapterSummary,
    /// Raw canonical Markdown source of the chapter.
    pub markdown: String,
    /// Pre-rendered safe HTML representation of the chapter for direct display in web clients.
    pub rendered_html: String,
}

/// Ranked section match returned when searching across the Satya TTRPG Rules Book (`GET /api/rules?q=...`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesSearchHit {
    /// Canonical chapter slug identifier where the match occurred.
    pub chapter_id: String,
    /// Human-readable title of the matching chapter.
    pub chapter_title: String,
    /// Relative file path inside `docs/ttrpg/`.
    pub relative_path: String,
    /// Heading of the specific section that matched the query.
    pub section_heading: String,
    /// Markdown excerpt of the matching section.
    pub snippet: String,
    /// Relevance score computed from keyword and heading hits.
    pub score: usize,
}

/// HTTP response body returned by `GET /api/rules` listing all chapters and optional search hits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesBookCatalogResponse {
    /// Ordered list of all canonical chapters in the Satya TTRPG Rules Book.
    pub chapters: Vec<RulesChapterSummary>,
    /// Ranked section search hits when a `?q=` query parameter was supplied.
    #[serde(default)]
    pub search_hits: Vec<RulesSearchHit>,
}

fn default_npc_career_terms() -> usize {
    2
}

/// HTTP request body for `POST /api/sessions/:session_id/npcs/generate` (and `/api/gm/sessions/:session_id/npcs/generate`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateNpcRequest {
    /// Free-text description of the NPC (e.g., `"Terry is a kind general MD, but needs to feel like she belongs"`).
    pub prompt: String,
    /// Optional explicit name override.
    #[serde(default)]
    pub name_override: Option<String>,
    /// Optional archetype code (`"AUTO"`, `"MEDICAL_DOCTOR"`, `"COMBAT_SOLDIER"`, `"DETECTIVE_INVESTIGATOR"`, etc.).
    #[serde(default)]
    pub template: Option<String>,
    /// Number of 4-year adult career terms (`1..=4`, default `2`).
    #[serde(default = "default_npc_career_terms")]
    pub career_terms: usize,
    /// Optional deterministic seed (`0` hashes `prompt`).
    #[serde(default)]
    pub seed: u64,
    /// Optional extra GM-only secret note to attach to the NPC dossier.
    #[serde(default)]
    pub gm_secret_notes: Option<String>,
    /// Whether to persist the generated NPC and its GM memory note across restarts (`default = true`).
    #[serde(default = "default_true_flag")]
    pub is_persistent: bool,
    /// Optional campaign tag override scoping the persistent NPC memory.
    #[serde(default)]
    pub campaign_tag: Option<String>,
}

/// Role-filtered NPC summary entry returned in [`SessionNpcsResponse`] and [`GenerateNpcResponse`].
///
/// Players receive only the public fields (`character_id`, `character_name`, `callsign_or_role`,
/// `age_years`, `public_summary`, `apparent_condition`), whereas Game Masters and Solo-Mode players
/// also receive `gm_character`, `psyche_summary`, `top_skills`, and `gm_secret_dossier`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionNpcEntry {
    pub character_id: String,
    pub character_name: String,
    pub callsign_or_role: String,
    pub age_years: i32,
    pub public_summary: String,
    pub apparent_condition: String,
    #[serde(default)]
    pub is_persistent: bool,
    /// Populated only for `GM` or in Solo / AI-GM mode (`None` for non-GM players when a human GM runs the table).
    #[serde(default)]
    pub gm_character: Option<crate::character::CharacterSheetRecord>,
    /// Populated only for `GM` or in Solo / AI-GM mode.
    #[serde(default)]
    pub psyche_summary: Vec<String>,
    /// Populated only for `GM` or in Solo / AI-GM mode (`"Skill Name (N=3)"`).
    #[serde(default)]
    pub top_skills: Vec<String>,
    /// Populated only for `GM` or in Solo / AI-GM mode.
    #[serde(default)]
    pub gm_secret_dossier: Option<String>,
}

/// HTTP response body for `POST /api/sessions/:session_id/npcs/generate`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerateNpcResponse {
    pub npc: SessionNpcEntry,
    #[serde(default)]
    pub gm_memory: Option<GmMemoryNoteRecord>,
}

/// HTTP response body for `GET /api/sessions/:session_id/npcs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionNpcsResponse {
    pub session_id: String,
    pub campaign_tag: String,
    pub viewer_can_see_gm_secrets: bool,
    pub npcs: Vec<SessionNpcEntry>,
}

fn default_max_rally_turns() -> u32 {
    2
}

/// Configuration for spawning or updating an autonomous AI Player (`satya_ml.ai_player`) in a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiPlayerConfig {
    /// Unique `user_id` for the AI Player (e.g. `"ai-player-1"`).
    #[serde(default)]
    pub user_id: Option<String>,
    /// Display or character name shown in the session roster and chat (`"Kaelen Voss (AI Player)"`).
    #[serde(default)]
    pub display_name: Option<String>,
    /// Optional bound `character_id` from the session's character sheets.
    #[serde(default)]
    pub character_id: Option<String>,
    /// Playstyle or persona directive guiding the AI Player's decisions.
    #[serde(default)]
    pub persona_prompt: Option<String>,
    /// Optional LLM provider override (`"ollama"`, `"google_genai"`, `"openai"`, `"anthropic"`).
    #[serde(default)]
    pub provider: Option<String>,
    /// Optional reasoning model override.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Optional custom service URL for the AI Player worker (defaults to `ai_gm.service_url` or `SATYA_AI_PLAYER_URL`).
    #[serde(default)]
    pub service_url: Option<String>,
    /// Whether the AI Player automatically submits a turn when awaited by `party_turn_gate_node` or after a GM scene beat.
    #[serde(default = "default_true_flag")]
    pub auto_respond: bool,
    /// Maximum consecutive AI Player <-> AI GM rally exchanges before pausing for human input (`default = 2`).
    #[serde(default = "default_max_rally_turns")]
    pub max_rally_turns: u32,
}

/// HTTP request body for `POST /api/sessions/:session_id/ai-players/step` to trigger an AI Player turn.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepAiPlayerRequest {
    /// Target AI Player `user_id` (or `"all"` / omitted to step the first active AI Player).
    #[serde(default)]
    pub user_id: Option<String>,
    /// Optional custom prompt or tactical instruction for this step.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Whether this AI Player turn should also trigger the AI GM upon completion (`default = true`).
    #[serde(default = "default_true_flag")]
    pub auto_trigger_gm: bool,
}

/// HTTP request body for `POST /api/player-ai/sessions/:session_id/chat` called by `satya_ml.ai_player`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAiChatRequest {
    pub user_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub visibility: MessageVisibility,
    #[serde(default)]
    pub recipient_user_ids: Vec<String>,
    pub channel_context: ChannelContext,
    pub body: String,
    #[serde(default = "default_true_flag")]
    pub auto_trigger_gm: bool,
}

/// HTTP request body for `POST /api/player-ai/sessions/:session_id/roll` called by `satya_ml.ai_player`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerAiRollRequest {
    pub user_id: String,
    #[serde(default)]
    pub actor_display_name: Option<String>,
    #[serde(alias = "skill_name")]
    pub check_category: String,
    #[serde(default = "default_control_c", alias = "control_c")]
    pub attribute_tn: i32,
    #[serde(default = "default_skill_level_n", alias = "pool_size")]
    pub skill_rank: usize,
    #[serde(default, alias = "external_mod")]
    pub difficulty_mod: i32,
    #[serde(default)]
    pub internal_mod: i32,
    #[serde(default)]
    pub combat_technique: Option<String>,
    #[serde(default)]
    pub tempo_pool: Option<i32>,
    #[serde(default)]
    pub control_pool: Option<i32>,
    #[serde(default)]
    pub force_pool: Option<i32>,
    #[serde(default)]
    pub notes: Option<String>,
}
