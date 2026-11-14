//! Shared client-to-server and server-to-client HTTP REST & WebSocket protocol DTOs.

use crate::events::{
    AiGmOperatingMode, ChannelContext, ChatMessageEvent, CombatStance, InteractionTurnType,
    MessageVisibility, TtrpgMechanicsEvent, WebRtcEventKind, WgpuDiceSpec,
};
use crate::protocol_map_ai::{default_control_c, default_skill_level_n};
pub use crate::protocol_map_ai::*;
pub use crate::tactical_map_dto::*;
use crate::types::{Participant, SessionMode, SessionRole};
use serde::{Deserialize, Serialize};

/// HTTP request body for `POST /api/auth/register`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegisterRequest {
    /// Desired unique account username.
    pub username: String,
    /// Plaintext account password (hashed with Argon2id on the server).
    pub password: String,
    /// Initial display or character name.
    pub display_name: String,
}

/// HTTP request body for `POST /api/auth/login`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginRequest {
    /// Account username to authenticate.
    pub username: String,
    /// Account password to verify against the stored Argon2id hash.
    pub password: String,
}

/// HTTP request body for `POST /api/auth/logout`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LogoutRequest {
    /// Optional active `session_id` to associate with the logout lifecycle event.
    pub session_id: Option<String>,
    /// Optional reason string recorded in [`crate::events::SessionLifecycleEvent`].
    pub reason: Option<String>,
}

/// Platform-wide account role controlling access to administrative UI panes and storage APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountRole {
    /// Standard platform user with access to sessions, WebRTC, chat, and TTRPG mechanics.
    #[default]
    User,
    /// Platform administrator with access to the Apache Iggy / Avro Inspector UI and storage flush/metrics APIs.
    Admin,
}

impl AccountRole {
    /// Returns the uppercase database string representation (`"USER"` or `"ADMIN"`).
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::User => "USER",
            Self::Admin => "ADMIN",
        }
    }

    /// Parses a database role string (`"ADMIN"` or `"USER"`) into an [`AccountRole`].
    pub fn from_db_str(raw: &str) -> Self {
        match raw.trim().to_uppercase().as_str() {
            "ADMIN" => Self::Admin,
            _ => Self::User,
        }
    }
}

/// HTTP response body returned upon successful user registration or login.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthSuccessResponse {
    /// Bearer token authorizing subsequent session creation, join, leave, or logout calls.
    pub auth_token: String,
    /// Persistent UUIDv7 identifier of the authenticated user account.
    pub user_id: String,
    /// Canonical lowercase username of the authenticated user account.
    pub username: String,
    /// Default display name associated with the user account.
    pub display_name: String,
    /// Platform-level account role (`USER` or `ADMIN`).
    #[serde(default)]
    pub account_role: AccountRole,
}

/// Per-session AI Game Master model provider and ephemeral BYOK credential configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiGmProviderConfig {
    /// Model provider identifier (`"ollama"`, `"google_genai"`, `"openai"`, or `"anthropic"`).
    pub provider: String,
    /// Primary reasoning and tool-calling model identifier (e.g. `"qwen2.5:14b"`, `"gemini-2.5-flash"`).
    pub model_id: String,
    /// Optional multimodal vision model identifier for tactical map inspection.
    #[serde(default)]
    pub vision_model_id: Option<String>,
    /// Sampling temperature for the AI GM (`0.0..=1.5`).
    #[serde(default = "default_gm_temperature")]
    pub temperature: f32,
    /// Persona/prompt identifier (e.g. `"harsh-reality-arbiter"`).
    #[serde(default = "default_gm_persona")]
    pub persona_id: String,
    /// Optional ephemeral player-supplied API key (held exclusively in memory for the session; never logged to Avro).
    #[serde(default)]
    pub api_key: Option<String>,
    /// Optional custom base URL (e.g. for local Ollama or custom OpenAI-compatible gateway).
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_gm_temperature() -> f32 {
    0.35
}

fn default_gm_persona() -> String {
    "harsh-reality-arbiter".to_string()
}

/// HTTP request body for `POST /api/sessions/create`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    /// Unique identifier for the session room to create.
    pub session_id: String,
    /// Password required for other participants to join this session.
    pub session_password: String,
    /// Operating mode (`TTRPG` or `MEETING`) for the session.
    pub session_mode: SessionMode,
    /// Role claimed by the session creator.
    pub role: SessionRole,
    /// Optional display or character name override for this session.
    pub display_name: Option<String>,
    /// Whether to include an autonomous AI Game Master participant in a TTRPG session.
    pub enable_ai_gm: bool,
    /// Optional initial AI GM provider, model, and ephemeral BYOK credential configuration.
    #[serde(default)]
    pub ai_gm_config: Option<AiGmProviderConfig>,
}

/// HTTP request body for `POST /api/sessions/join`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinSessionRequest {
    /// Identifier of the existing session room to join.
    pub session_id: String,
    /// Password for the target session room.
    pub session_password: String,
    /// Role claimed by the joining participant.
    pub role: SessionRole,
    /// Optional display or character name override for this session.
    pub display_name: Option<String>,
}

/// HTTP request body for `POST /api/sessions/leave`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaveSessionRequest {
    /// Identifier of the session from which the user is disconnecting.
    pub session_id: String,
    /// Optional human-readable disconnect reason.
    pub reason: Option<String>,
}

/// HTTP response body returned when a user creates, joins, or resumes a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEnterResponse {
    /// Identifier of the active session room.
    pub session_id: String,
    /// Operating mode (`TTRPG` or `MEETING`) of the session.
    pub session_mode: SessionMode,
    /// Single-session token used to authenticate the `/ws/session` WebSocket upgrade.
    pub join_token: String,
    /// Nested [`Participant`] record representing the caller in this session.
    pub self_participant: Participant,
    /// Current participant roster in the session.
    pub roster: Vec<Participant>,
    /// Optional visibility-filtered session history snapshot (chat, mechanics rolls, active map, combat tracker, bound character).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<crate::session_persistence::SessionHistorySnapshot>,
}

/// Client-to-server WebSocket packet sent over `/ws/session`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClientWsPacket {
    /// Transmits a public, private, GM-only, or Host-only chat message.
    SendChat {
        /// Visibility scope for routing the message.
        visibility: MessageVisibility,
        /// Explicit recipient `user_id`s when `visibility` is [`MessageVisibility::Private`].
        #[serde(default)]
        recipient_user_ids: Vec<String>,
        /// Semantic channel context of the message.
        channel_context: ChannelContext,
        /// UTF-8 message body text.
        body: String,
        /// Time spent typing the message in milliseconds.
        #[serde(default)]
        composition_time_ms: i64,
    },
    /// Relays a WebRTC SDP offer/answer or ICE candidate to a specific peer `target_user_id`.
    WebrtcSignal {
        /// Destination peer's `user_id`.
        target_user_id: String,
        /// Signaling event kind (`SIGNALING_OFFER`, `SIGNALING_ANSWER`, or `ICE_CANDIDATE`).
        signal_kind: WebRtcEventKind,
        /// JSON-serialized SDP or ICE candidate payload.
        payload: String,
    },
    /// Logs a WebRTC DataChannel state change or audio/video track mute toggle.
    WebrtcTelemetry {
        /// Optional remote peer `user_id` associated with the event.
        target_user_id: Option<String>,
        /// WebRTC telemetry event kind.
        event_kind: WebRtcEventKind,
        /// Current microphone mute state.
        audio_muted: bool,
        /// Current camera video mute state.
        video_muted: bool,
    },
    /// Logs a voice-activity detection (VAD) or prosody turn sample to `interaction-dynamics`.
    InteractionDynamics {
        /// Type of vocal turn or silence interval.
        turn_type: InteractionTurnType,
        /// Duration of the vocal turn in milliseconds.
        duration_ms: i64,
        /// Mean RMS vocal energy during the turn.
        rms_energy_mean: f32,
        /// Peak RMS vocal energy during the turn.
        rms_energy_peak: f32,
        /// Optional estimated fundamental pitch in Hertz.
        pitch_hz_estimate: Option<f32>,
        /// Preceding silence duration in milliseconds.
        preceding_silence_ms: i64,
    },
    /// Requests server-authoritative evaluation of a Satya Two-List Cascading ("Watershed") `2d20` skill or combat task.
    RollTtrpgTask {
        /// Name of the skill being tested (e.g. `"Trauma Surgery"`).
        skill_name: String,
        /// Skill Level $N \in [1, 10]$ (number of `2d20` pairs rolled; trained average `3`).
        #[serde(
            default = "default_skill_level_n",
            alias = "pool_size",
            alias = "skill_level_n"
        )]
        skill_level: usize,
        /// Controlling Characteristic $C \in [1, 20]$ (shifts $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$; human average `10`).
        #[serde(default = "default_control_c", alias = "attribute_tn")]
        control_c: i32,
        /// Internal Character Modifier $\Delta_{\text{int}} \in [-2, +10]$ (Fatigue, Injury, Stress; increases failure carry penalty $\delta_{\text{under}} = 6 + \Delta_{\text{int}}$).
        #[serde(default, alias = "delta_int")]
        internal_mod: i32,
        /// Known Player Tool Modifier $\Delta_{\text{tool}} \in [-4, +12]$ (shifts $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$).
        #[serde(default, alias = "delta_tool", alias = "tool_mod")]
        known_tool_mod: i32,
        /// External Task Modifier $\Delta_{\text{ext}} \in [-10, +16]$ (shifts $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$ in contested mode, or $\text{TN}_{\text{eff}}$ in uncontested mode).
        #[serde(default, alias = "difficulty_rating", alias = "delta_ext")]
        external_mod: i32,
        /// Optional over-target step carry $\delta_{\text{over}}$ (`6` standard, `8` when boosted by Extraordinary `KNO -> PHY` synergy).
        #[serde(default)]
        delta_over: Option<i32>,
        /// Optional GM resistance mode (`"tiered_single_pair"` default, `"full_cascade"`, or `"uncontested"`).
        #[serde(default)]
        gm_resistance_mode: Option<String>,
        /// Optional contested tie-breaking rule (`"hybrid_signed_margin"` default, `"strict_signed_margin"`, or `"active_player_wins_ties"`).
        #[serde(default)]
        tie_rule: Option<String>,
        /// Optional cascade carry-over mode (`"markov_step"`, `"cumulative_snowball"`, or `"linear_margin"`).
        #[serde(default)]
        carry_mode: Option<String>,
        /// Optional botch severity rule (`"closest_miss"`, `"ceiling_breach"`, or `"overrun_k"`).
        #[serde(default)]
        botch_rule: Option<String>,
        /// Optional combat stance selected for the action.
        #[serde(default)]
        combat_stance: Option<CombatStance>,
        /// Current character fatigue level (scales Action Time Factor).
        #[serde(default)]
        fatigue_level: i32,
        /// Current character wound shock index (scales Action Time Factor).
        #[serde(default)]
        wound_shock_index: f32,
        /// Optional canonical combat action/technique code from `COMBAT_TECHNIQUE_CATALOG` (e.g. `"punch"`, `"snap_shot"`, `"takedown"`).
        #[serde(default)]
        combat_technique: Option<String>,
        /// Optional Tactical Triad Tempo (`T`) pool allocation.
        #[serde(default)]
        tempo_pool: Option<i32>,
        /// Optional Tactical Triad Control (`C`) pool allocation.
        #[serde(default)]
        control_pool: Option<i32>,
        /// Optional Tactical Triad Force (`F`) pool allocation.
        #[serde(default)]
        force_pool: Option<i32>,
    },
    /// Notifies the server that the participant is intentionally disconnecting from the session.
    LeaveSession {
        /// Optional disconnect reason string.
        #[serde(default)]
        reason: Option<String>,
    },
}

/// Server-to-client WebSocket packet broadcast or targeted over `/ws/session`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServerWsPacket {
    /// Synchronizes the complete active session participant roster.
    RosterSync {
        /// Updated list of active participants in the session.
        roster: Vec<Participant>,
    },
    /// Notifies remaining peers that a participant disconnected from the session.
    PeerDisconnected {
        /// Nested [`Participant`] record of the departed user.
        participant: Participant,
        /// Optional human-readable disconnect reason.
        #[serde(default)]
        reason: Option<String>,
    },
    /// Delivers a public or private [`ChatMessageEvent`] to authorized recipients.
    ChatEvent {
        /// Logged chat message event payload.
        event: Box<ChatMessageEvent>,
    },
    /// Broadcasts a resolved [`TtrpgMechanicsEvent`] dice roll and action economy outcome.
    TtrpgMechanicsEvent {
        /// Logged TTRPG mechanics event payload.
        event: Box<TtrpgMechanicsEvent>,
    },
    /// Delivers a relayed WebRTC SDP offer/answer or ICE candidate from a peer.
    WebrtcSignal {
        /// Peer participant who sent the signaling frame.
        from_participant: Participant,
        /// Signaling event kind (`SIGNALING_OFFER`, `SIGNALING_ANSWER`, or `ICE_CANDIDATE`).
        signal_kind: WebRtcEventKind,
        /// JSON-serialized SDP or ICE candidate payload.
        payload: String,
    },
    /// Broadcasts a new or updated tactical map to all participants in the session.
    MapUpdated {
        /// Updated tactical map state and token placements.
        map: Box<GmMapUpdateRequest>,
    },
    /// Broadcasts a deterministic 3D `wgpu` physics dice roll seed and outcome.
    WgpuDiceRoll {
        /// Participant who rolled the dice.
        actor: Participant,
        /// Deterministic 3D physics seed, dice expression, and outcomes.
        spec: WgpuDiceSpec,
    },
    /// Delivers a sanitized `GM_ONLY` document snapshot strictly to the session's Human GM (and the uploader).
    GmDocumentAdded {
        /// Sanitized `GM_ONLY` document record.
        document: Box<GmDocumentRecord>,
    },
}

/// Source format of a `GM_ONLY` document submitted to the session Game Master.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GmDocumentSource {
    /// Plain-text, Markdown, or CSV file (`<= 32 KB`) uploaded directly.
    #[default]
    DirectTextUpload,
    /// Google Docs document imported via the single-file `drive.file` Picker/API.
    GoogleDoc,
    /// Google Sheets spreadsheet imported as a tabular CSV/Markdown snapshot via `drive.file`.
    GoogleSheet,
}

/// Sanitized, text-only `GM_ONLY` document snapshot stored in the session vault for the Human GM and AI GM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GmDocumentRecord {
    /// Unique identifier (`doc_<uuid>`) for this document snapshot.
    pub doc_id: String,
    /// Session ID to which this document belongs.
    pub session_id: String,
    /// Persistent `user_id` of the participant who submitted the document.
    pub uploader_user_id: String,
    /// Display name of the participant who submitted the document.
    pub uploader_display_name: String,
    /// Sanitized human-readable document title.
    pub title: String,
    /// Origin of the document (`DIRECT_TEXT_UPLOAD`, `GOOGLE_DOC`, or `GOOGLE_SHEET`).
    pub source_kind: GmDocumentSource,
    /// Optional opaque Google Drive file ID (never exposes personal email addresses).
    #[serde(default)]
    pub google_file_id: Option<String>,
    /// Strictly enforced [`MessageVisibility::GmOnly`] visibility (`Player -> GM` only).
    pub visibility: MessageVisibility,
    /// Sanitized plain-text/Markdown/CSV content (all HTML, images, scripts, and binary payloads stripped).
    pub sanitized_content: String,
    /// Character count of `sanitized_content`.
    pub char_count: usize,
    /// Whether prompt-injection tags or unsafe control sequences were scrubbed during ingestion.
    #[serde(default)]
    pub safety_flagged: bool,
    /// Whether this document is persisted in the SQLite GM Vault across server restarts and campaign sessions.
    #[serde(default)]
    pub is_persistent: bool,
    /// Optional campaign/session scope tag grouping persistent documents across sessions (defaults to `session_id`).
    #[serde(default)]
    pub campaign_tag: Option<String>,
    /// Ingestion timestamp in UTC microseconds since Unix epoch.
    pub created_at_us: i64,
}

/// HTTP request body for `POST /api/auth/google` linking an authenticated user's Google identity
/// and optional ephemeral `drive.file` access token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleAuthLinkRequest {
    /// Opaque Google OIDC subject identifier (`sub`) or local dev Google identifier.
    pub google_sub: String,
    /// Optional display label for the linked Google identity (never broadcast to other players).
    #[serde(default)]
    pub google_display_hint: Option<String>,
    /// Optional short-lived OAuth 2.0 `drive.file` access token held strictly in memory.
    #[serde(default)]
    pub access_token: Option<String>,
}

/// HTTP response body for `POST /api/auth/google` and `GET /api/auth/google/status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleAuthStatusResponse {
    /// Authenticated Satya `user_id`.
    pub user_id: String,
    /// Whether a Google identity is linked to this Satya account.
    pub google_linked: bool,
    /// Opaque Google subject identifier (`sub`), if linked.
    pub google_sub: Option<String>,
    /// Whether an active in-memory `drive.file` access token is available for single-file Picker imports.
    pub has_ephemeral_drive_token: bool,
    /// Enforced OAuth scope (`"https://www.googleapis.com/auth/drive.file"`).
    pub enforced_scope: String,
    /// Optional server-configured Google Cloud OAuth 2.0 Web Client ID for `drive.file`.
    #[serde(default)]
    pub google_oauth_client_id: Option<String>,
    /// Optional server-configured Google Picker API Key for `google.picker.PickerBuilder`.
    #[serde(default)]
    pub google_picker_api_key: Option<String>,
}

/// HTTP request body for `POST /api/sessions/:session_id/docs` submitting a `GM_ONLY` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitGmDocumentRequest {
    /// Human-readable title for the character sheet, backstory, or spreadsheet.
    pub title: String,
    /// Document source type (`DIRECT_TEXT_UPLOAD`, `GOOGLE_DOC`, or `GOOGLE_SHEET`).
    #[serde(default)]
    pub source_kind: GmDocumentSource,
    /// Plain-text/Markdown/CSV content (required for `DIRECT_TEXT_UPLOAD`; optional if fetched by `google_file_id`).
    #[serde(default)]
    pub raw_content: Option<String>,
    /// Optional Google Doc or Sheet file ID selected via Google Picker (`drive.file`).
    #[serde(default)]
    pub google_file_id: Option<String>,
    /// Optional single-use OAuth `drive.file` access token (never logged or persisted).
    #[serde(default)]
    pub google_access_token: Option<String>,
    /// Whether to persist this document in SQLite across server restarts and matching `campaign_tag` / `session_id` sessions.
    #[serde(default)]
    pub is_persistent: bool,
    /// Optional campaign tag scoping persistent documents (defaults to `session_id` when omitted).
    #[serde(default)]
    pub campaign_tag: Option<String>,
}

/// HTTP request body for `POST /api/sessions/:session_id/docs/:doc_id/persist` toggling persistence on a `GM_ONLY` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToggleGmDocumentPersistenceRequest {
    /// Target persistence state (`true` to persist in SQLite, `false` to keep session-ephemeral only).
    pub is_persistent: bool,
    /// Optional campaign/session tag override (defaults to the document's existing `campaign_tag` or `session_id`).
    #[serde(default)]
    pub campaign_tag: Option<String>,
}

/// Semantic category for a persistent Game Master memory note (`gm_memories` table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GmMemoryCategory {
    #[default]
    SceneState,
    NpcSecret,
    QuestHook,
    Ruling,
    RollingSummary,
}

impl GmMemoryCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SceneState => "SCENE_STATE",
            Self::NpcSecret => "NPC_SECRET",
            Self::QuestHook => "QUEST_HOOK",
            Self::Ruling => "RULING",
            Self::RollingSummary => "ROLLING_SUMMARY",
        }
    }

    pub fn from_code_str(raw: &str) -> Self {
        match raw.trim().to_uppercase().as_str() {
            "NPC_SECRET" | "NPC" | "SECRET" => Self::NpcSecret,
            "QUEST_HOOK" | "QUEST" | "HOOK" => Self::QuestHook,
            "RULING" | "RULE" => Self::Ruling,
            "ROLLING_SUMMARY" | "SUMMARY" => Self::RollingSummary,
            _ => Self::SceneState,
        }
    }
}

/// Persistent Game Master memory note stored per `session_id` / `campaign_tag` for both Human GM and AI GM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GmMemoryNoteRecord {
    pub memory_id: String,
    pub session_id: String,
    pub campaign_tag: String,
    pub author_user_id: String,
    pub author_display_name: String,
    #[serde(default)]
    pub category: GmMemoryCategory,
    pub title: String,
    pub content: String,
    /// When `true`, this concise fact is pinned directly into the AI GM's active turn context header.
    #[serde(default)]
    pub is_pinned: bool,
    /// When `true`, persisted to SQLite (`gm_memories`) across restarts and campaign sessions.
    #[serde(default = "default_true_flag")]
    pub is_persistent: bool,
    pub updated_at_us: i64,
}

pub(crate) fn default_true_flag() -> bool {
    true
}

/// HTTP request body for `POST /api/sessions/:session_id/memories` (or `/api/gm/sessions/:session_id/memories`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpsertGmMemoryRequest {
    #[serde(default)]
    pub memory_id: Option<String>,
    #[serde(default)]
    pub campaign_tag: Option<String>,
    #[serde(default)]
    pub category: GmMemoryCategory,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default = "default_true_flag")]
    pub is_persistent: bool,
}

/// HTTP response body for `GET /api/sessions/:session_id/memories`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionGmMemoriesResponse {
    pub session_id: String,
    pub campaign_tag: String,
    pub memories: Vec<GmMemoryNoteRecord>,
}

/// Per-combatant readiness and stance entry on the Satya Count-Up Tick timeline (`1 Tick = 0.5s`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatantTickEntry {
    pub entity_id: String,
    pub display_name: String,
    pub role_or_faction: String,
    pub ooda_score: i32,
    pub next_action_tick: u32,
    #[serde(default)]
    pub active_stance: Option<String>,
    #[serde(default)]
    pub fatigue_level: i32,
    #[serde(default)]
    pub wound_shock: i32,
    /// Optional tactical map X grid coordinate stub (`0..grid_width`).
    #[serde(default)]
    pub grid_x: Option<i32>,
    /// Optional tactical map Y grid coordinate stub (`0..grid_height`).
    #[serde(default)]
    pub grid_y: Option<i32>,
    /// Optional named tactical zone or cover position stub (e.g. `"Core Console"`, `"Entry Bulkhead"`).
    #[serde(default)]
    pub zone_label: Option<String>,
}

/// Authoritative Satya Count-Up Tick Combat Tracker state for a session (`1 Tick = 0.5s`, `1 Window = 20 Ticks`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatTrackerState {
    pub session_id: String,
    pub combat_active: bool,
    pub round: u32,
    pub tick: u32,
    #[serde(default)]
    pub combatants: Vec<CombatantTickEntry>,
    pub updated_at_us: i64,
}

impl Default for CombatTrackerState {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            combat_active: false,
            round: 1,
            tick: 0,
            combatants: Vec::new(),
            updated_at_us: 0,
        }
    }
}

/// HTTP request body for `POST /api/sessions/:session_id/combat` (or `/api/gm/sessions/:session_id/combat`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCombatTrackerRequest {
    /// Tick delta to advance (`> 0` advances clock; `Some(0)` resets to Window 1, Tick 1; `None` keeps current tick).
    #[serde(default)]
    pub tick_delta: Option<i32>,
    #[serde(default)]
    pub combat_active: Option<bool>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub active_stance: Option<String>,
    #[serde(default)]
    pub next_action_tick: Option<u32>,
    #[serde(default)]
    pub fatigue_level: Option<i32>,
    #[serde(default)]
    pub wound_shock: Option<i32>,
    /// Optional tactical map X grid coordinate stub (`0..grid_width`).
    #[serde(default)]
    pub grid_x: Option<i32>,
    /// Optional tactical map Y grid coordinate stub (`0..grid_height`).
    #[serde(default)]
    pub grid_y: Option<i32>,
    /// Optional named tactical zone or cover position stub.
    #[serde(default)]
    pub zone_label: Option<String>,
}

/// Source domain of a unified GM Omnisearch hit (`GET /api/sessions/:session_id/lookup`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GmLookupSourceKind {
    RulesBook,
    GmDocument,
    GmMemory,
    CharacterSheet,
    CombatTracker,
    TacticalMap,
}

/// Single ranked search or state lookup hit returned by `GET /api/sessions/:session_id/lookup`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GmLookupHit {
    pub source_kind: GmLookupSourceKind,
    pub item_id: String,
    pub title: String,
    pub subtitle: String,
    pub snippet: String,
    pub is_persistent: bool,
    pub score: usize,
}

/// Unified response returned by `GET /api/sessions/:session_id/lookup` for both Human GM Omnisearch and AI GM RAG/MCP lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GmOmnisearchResponse {
    pub session_id: String,
    pub campaign_tag: String,
    pub query: String,
    pub scope: String,
    pub hits: Vec<GmLookupHit>,
}

/// HTTP response body for `GET /api/sessions/:session_id/docs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionGmDocsResponse {
    /// Session identifier.
    pub session_id: String,
    /// List of `GM_ONLY` documents visible to the caller (all docs for GM/Host; own submissions only for Players).
    pub documents: Vec<GmDocumentRecord>,
}

/// HTTP response body for `GET /api/gm/sessions/:session_id/context`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GmContextResponse {
    /// Active session room identifier.
    pub session_id: String,
    /// Business operating mode (`TTRPG` or `MEETING`).
    pub session_mode: SessionMode,
    /// Active AI GM operating topology (`AUTONOMOUS` for `TTRPG_AI_GM`, `CO_PILOT` for `TTRPG_GM_AI`).
    pub operating_mode: AiGmOperatingMode,
    /// [`Participant`] record of the AI GM in this session, if enabled.
    pub ai_gm_participant: Option<Participant>,
    /// `user_id` of the human Game Master in the session (`Some(...)` in `TTRPG_GM_AI` Co-Pilot mode).
    pub human_gm_user_id: Option<String>,
    /// Current session participant roster.
    pub roster: Vec<Participant>,
    /// Currently active tactical map in the session, if any.
    pub active_map: Option<GmMapUpdateRequest>,
    /// Sanitized `GM_ONLY` documents submitted to the GM in this session.
    #[serde(default)]
    pub gm_documents: Vec<GmDocumentRecord>,
    /// Persistent and session-scoped GM memory notes for this session/campaign.
    #[serde(default)]
    pub gm_memories: Vec<GmMemoryNoteRecord>,
    /// Authoritative Satya Count-Up Tick Combat Tracker state (`round`, `tick`, `combatants`).
    #[serde(default)]
    pub combat_tracker: Option<CombatTrackerState>,
    /// Character sheets currently bound to participants in this session.
    #[serde(default)]
    pub session_characters: Vec<crate::character::CharacterSheetRecord>,
    /// Campaign tag grouping persistent GM documents and memories for this session.
    #[serde(default)]
    pub campaign_tag: Option<String>,
}

/// HTTP request body for `POST /api/gm/sessions/:session_id/chat`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GmChatRequest {
    /// Delivery visibility scope (`PUBLIC`, `PRIVATE`, `GM_ONLY`).
    pub visibility: MessageVisibility,
    /// Target recipient `user_id`s when `visibility` is [`MessageVisibility::Private`].
    #[serde(default)]
    pub recipient_user_ids: Vec<String>,
    /// Channel context (`GM_NARRATION` or `IN_CHARACTER`).
    #[serde(default = "default_gm_channel_context")]
    pub channel_context: ChannelContext,
    /// UTF-8 narration or NPC dialogue text.
    pub body: String,
    /// Optional model identifier override for `AiAgentInfo`.
    #[serde(default)]
    pub model_id: Option<String>,
    /// Optional NPC display name override when speaking `IN_CHARACTER`.
    #[serde(default)]
    pub speaker_override: Option<String>,
}

fn default_gm_channel_context() -> ChannelContext {
    ChannelContext::GmNarration
}
