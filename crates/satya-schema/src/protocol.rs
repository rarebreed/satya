//! Shared client-to-server and server-to-client HTTP REST & WebSocket protocol DTOs.

use crate::events::{
    AiGmOperatingMode, ChannelContext, ChatMessageEvent, CombatStance, InteractionTurnType,
    MessageVisibility, TokenPlacement, TtrpgMechanicsEvent, WebRtcEventKind, WgpuDiceSpec,
};
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

/// HTTP response body returned when a user creates or joins a session.
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
    /// Requests server-authoritative evaluation of a Satya d20 skill or combat task.
    RollTtrpgTask {
        /// Name of the skill being tested.
        skill_name: String,
        /// Controlling attribute Target Number (`1..=20`).
        attribute_tn: i32,
        /// Number of `d20`s rolled in the pool.
        pool_size: usize,
        /// Required number of successes.
        difficulty_rating: i32,
        /// Optional combat stance selected for the action.
        combat_stance: Option<CombatStance>,
        /// Current character fatigue level.
        fatigue_level: i32,
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
    /// Controlling attribute Target Number (`1..=20`).
    pub attribute_tn: i32,
    /// Number of `d20`s in the dice pool.
    pub pool_size: usize,
    /// Required number of successes.
    pub difficulty_rating: i32,
    /// Optional Satya combat stance.
    #[serde(default)]
    pub combat_stance: Option<CombatStance>,
    /// Accumulated fatigue level of the acting NPC.
    #[serde(default)]
    pub fatigue_level: i32,
    /// Accumulated wound shock index of the acting NPC.
    #[serde(default)]
    pub wound_shock_index: f32,
    /// Optional adjudication notes.
    #[serde(default)]
    pub notes: Option<String>,
}

/// HTTP request body for `POST /api/gm/sessions/:session_id/map`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GmMapUpdateRequest {
    /// Unique identifier for the tactical map.
    pub map_id: String,
    /// Human-readable map title.
    pub title: String,
    /// URL or relative path to the rendered SVG/PNG map image.
    pub image_url: String,
    /// Tactical grid width in cells.
    pub grid_width: i32,
    /// Tactical grid height in cells.
    pub grid_height: i32,
    /// Active character and NPC token coordinates on the grid.
    #[serde(default)]
    pub tokens: Vec<TokenPlacement>,
    /// Optional terrain, cover, or environmental hazard notes.
    #[serde(default)]
    pub notes: Option<String>,
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
    /// `user_id` of the human GM if operating in `CO_PILOT` mode.
    pub human_gm_user_id: Option<String>,
    /// `event_id` or `message_id` of the triggering chat message or mechanics roll.
    pub trigger_event_id: String,
    /// Triggering event category (`"CHAT"` or `"MECHANICS"`).
    pub trigger_kind: String,
    /// Human [`Participant`] who triggered the AI GM turn.
    pub trigger_actor: Participant,
    /// Visibility of the triggering message (`PUBLIC`, `PRIVATE`, `GM_ONLY`).
    pub visibility: MessageVisibility,
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
