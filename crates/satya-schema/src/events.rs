//! Domain event structs (`com.satya.events`) and canonical Apache Iggy subject/topic constants.

use crate::types::{EventMetadata, Participant};
use serde::{Deserialize, Serialize};

/// Canonical Avro schema subject names (`com.satya.events.*`).
pub mod subjects {
    /// Subject name for [`super::SchemaRegistryEvent`].
    pub const SCHEMA_REGISTRY: &str = "com.satya.events.SchemaRegistryEvent";
    /// Subject name for [`super::SessionLifecycleEvent`].
    pub const SESSION_LIFECYCLE: &str = "com.satya.events.SessionLifecycleEvent";
    /// Subject name for [`super::WebRtcTelemetryEvent`].
    pub const WEBRTC_TELEMETRY: &str = "com.satya.events.WebRtcTelemetryEvent";
    /// Subject name for [`super::ChatMessageEvent`] and [`super::ChatMessageEventV2`].
    pub const CHAT_MESSAGE: &str = "com.satya.events.ChatMessageEvent";
    /// Subject name for [`super::InteractionDynamicsEvent`].
    pub const INTERACTION_DYNAMICS: &str = "com.satya.events.InteractionDynamicsEvent";
    /// Subject name for [`super::TtrpgMechanicsEvent`].
    pub const TTRPG_MECHANICS: &str = "com.satya.events.TtrpgMechanicsEvent";
    /// Subject name for [`super::TabletopCanvasEvent`].
    pub const TABLETOP_CANVAS: &str = "com.satya.events.TabletopCanvasEvent";
    /// Subject name for [`super::AiGmTrajectoryEvent`].
    pub const AI_GM_TRAJECTORY: &str = "com.satya.events.AiGmTrajectoryEvent";
    /// Subject name for [`super::SessionLogEvent`].
    pub const SESSION_LOG: &str = "com.satya.events.SessionLogEvent";
}

/// Canonical Apache Iggy topic names corresponding to each Satya event stream.
pub mod topics {
    /// Topic storing [`super::SchemaRegistryEvent`] records.
    pub const SCHEMA_REGISTRY: &str = "schema-registry";
    /// Topic storing [`super::SessionLifecycleEvent`] records.
    pub const SESSION_LIFECYCLE: &str = "session-lifecycle";
    /// Topic storing [`super::WebRtcTelemetryEvent`] records.
    pub const WEBRTC_TELEMETRY: &str = "webrtc-telemetry";
    /// Topic storing [`super::ChatMessageEvent`] records.
    pub const CHAT_MESSAGE: &str = "chat-messages";
    /// Topic storing [`super::InteractionDynamicsEvent`] records.
    pub const INTERACTION_DYNAMICS: &str = "interaction-dynamics";
    /// Topic storing [`super::TtrpgMechanicsEvent`] records.
    pub const TTRPG_MECHANICS: &str = "ttrpg-mechanics";
    /// Topic storing [`super::TabletopCanvasEvent`] records.
    pub const TABLETOP_CANVAS: &str = "tabletop-canvas";
    /// Topic storing [`super::AiGmTrajectoryEvent`] records.
    pub const AI_GM_TRAJECTORY: &str = "ai-gm-trajectories";
    /// Topic storing [`super::SessionLogEvent`] records.
    pub const SYSTEM_LOGS: &str = "system-logs";
}

/// Schema evolution compatibility policy enforced for a registered subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityMode {
    /// Newer reader schemas can read data written by older writer schemas.
    Backward,
    /// Older reader schemas can read data written by newer writer schemas.
    Forward,
    /// Both backward and forward schema compatibility are guaranteed.
    Full,
    /// No schema evolution compatibility checks are enforced.
    None,
}

/// Audit event emitted to the `schema-registry` topic whenever a schema version is registered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaRegistryEvent {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// Numeric schema identifier in the registry.
    pub schema_id: i32,
    /// Fully qualified Avro schema subject name.
    pub subject: String,
    /// Version number of the schema within its subject.
    pub version: i32,
    /// Compatibility mode declared for this schema version.
    pub compatibility_mode: CompatibilityMode,
    /// Hex-encoded 64-bit `CRC-64-AVRO` fingerprint of the canonical schema form.
    pub fingerprint_crc64_hex: String,
    /// Raw `.avsc` JSON schema definition.
    pub avro_schema_json: String,
    /// Optional participant who registered the schema (`None` for system bootstrap).
    pub registered_by: Option<Participant>,
}

/// Specific authentication or room lifecycle transition recorded by a [`SessionLifecycleEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleEventType {
    /// A new user account was registered.
    UserRegistered,
    /// An existing user account authenticated with valid credentials.
    UserAuthenticated,
    /// An authenticated user explicitly logged out and revoked their token.
    UserLoggedOut,
    /// An authentication or session join attempt failed verification.
    AuthFailed,
    /// A host or GM created a new password-protected session.
    SessionCreated,
    /// A participant joined an existing password-protected session.
    ParticipantJoined,
    /// A participant explicitly left/disconnected from a session.
    ParticipantLeft,
    /// A participant's WebSocket connection closed or dropped.
    ParticipantDisconnected,
    /// A participant's role within the session was updated.
    RoleChanged,
    /// The session ended after the last human participant disconnected.
    SessionEnded,
}

/// Event emitted to the `session-lifecycle` topic tracking authentication, joins, disconnects, and logouts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionLifecycleEvent {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// Type of lifecycle transition that occurred.
    pub lifecycle_type: LifecycleEventType,
    /// Primary participant who initiated or experienced the lifecycle event.
    pub actor: Participant,
    /// Optional secondary target participant (e.g. for role changes).
    pub target_participant: Option<Participant>,
    /// Whether the session requires a password to join.
    pub password_protected: bool,
    /// Complete snapshot of the session participant roster immediately after the event.
    pub roster_snapshot: Vec<Participant>,
    /// Optional human-readable description or reason for the lifecycle event.
    pub reason: Option<String>,
}

/// Type of WebRTC signaling or media telemetry event recorded in [`WebRtcTelemetryEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebRtcEventKind {
    /// SDP offer transmitted to initiate or renegotiate a peer connection.
    SignalingOffer,
    /// SDP answer transmitted in response to a peer's offer.
    SignalingAnswer,
    /// Trickle ICE candidate exchanged between peers.
    IceCandidate,
    /// ICE connection state transition on a peer connection.
    IceStateChange,
    /// P2P `RTCDataChannel` opened between two peers.
    DataChannelOpen,
    /// P2P `RTCDataChannel` closed prior to peer teardown.
    DataChannelClosed,
    /// Peer connection closed due to participant session disconnect or logout.
    PeerDisconnected,
    /// Periodic network quality-of-experience (RTT, jitter, loss, bitrate) snapshot.
    StatsSnapshot,
    /// Local audio microphone or video camera mute state toggled.
    TrackMuteChange,
}

/// Network transport classification of a WebRTC ICE candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IceCandidateType {
    /// Local interface address candidate (`host`).
    Host,
    /// Server-reflexive NAT mapping discovered via STUN (`srflx`).
    Srflx,
    /// Peer-reflexive mapping discovered during connectivity checks (`prflx`).
    Prflx,
    /// Relayed candidate allocated via a TURN server (`relay`).
    Relay,
    /// Unspecified or unknown ICE candidate type.
    Unknown,
}

/// Event emitted to the `webrtc-telemetry` topic capturing WebRTC signaling, mute states, and QoE metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebRtcTelemetryEvent {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// Participant emitting or experiencing the WebRTC telemetry event.
    pub local_participant: Participant,
    /// Optional remote peer participant associated with the connection event.
    pub remote_participant: Option<Participant>,
    /// Specific WebRTC signaling or telemetry event kind.
    pub event_kind: WebRtcEventKind,
    /// Optional ICE candidate transport classification.
    pub ice_candidate_type: Option<IceCandidateType>,
    /// Optional round-trip time in milliseconds.
    pub rtt_ms: Option<f32>,
    /// Optional packet arrival jitter in milliseconds.
    pub jitter_ms: Option<f32>,
    /// Optional fractional packet loss rate (`0.0` to `1.0`).
    pub packet_loss_rate: Option<f32>,
    /// Optional estimated available outgoing bandwidth in bits per second.
    pub available_outgoing_bitrate_bps: Option<i64>,
    /// Whether the local participant's audio track is muted.
    pub audio_muted: bool,
    /// Whether the local participant's video track is muted.
    pub video_muted: bool,
}

/// Delivery audience and access control scope for a [`ChatMessageEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageVisibility {
    /// Broadcast to all participants in the session.
    Public,
    /// Whispered privately only to explicitly selected recipient `user_id`s.
    Private,
    /// Visible only to the human GM and/or autonomous AI GM (plus the sender).
    GmOnly,
    /// Visible only to the Meeting Host (plus the sender).
    HostOnly,
}

/// Semantic channel classification for a chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChannelContext {
    /// Spoken or acted in-character within the TTRPG world.
    InCharacter,
    /// Table-level out-of-character discussion among players.
    OutOfCharacter,
    /// World description or adjudication narrated by a human GM or AI GM.
    GmNarration,
    /// Public discussion channel in a Meeting session.
    MeetingPublic,
    /// Side-channel or private backchannel in a Meeting session.
    MeetingBackchannel,
    /// Automated platform or simulation system notice.
    SystemNotice,
}

/// Event (`v1`) emitted to the `chat-messages` topic for public and private targeted chat messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessageEvent {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// Unique UUIDv7 identifier for this chat message.
    pub message_id: String,
    /// Nested [`Participant`] record of the message sender.
    pub sender: Participant,
    /// Visibility and routing scope (`PUBLIC`, `PRIVATE`, `GM_ONLY`, `HOST_ONLY`).
    pub visibility: MessageVisibility,
    /// Resolved list of recipient [`Participant`] records who received the message.
    pub recipients: Vec<Participant>,
    /// Semantic channel context in which the message was sent.
    pub channel_context: ChannelContext,
    /// Optional `message_id` of an earlier message being replied to.
    pub reply_to_message_id: Option<String>,
    /// UTF-8 text body of the chat message.
    pub body: String,
    /// Total Unicode character count of `body`.
    pub char_count: i32,
    /// Total whitespace-delimited word count of `body`.
    pub word_count: i32,
    /// Time spent composing the message in milliseconds (for ML interaction analysis).
    pub composition_time_ms: i64,
    /// Whether the message has been edited after initial transmission.
    pub is_edited: bool,
}

/// Evolved v2 representation of [`ChatMessageEvent`] demonstrating backward/forward schema evolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessageEventV2 {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// Unique UUIDv7 identifier for this chat message.
    pub message_id: String,
    /// Nested [`Participant`] record of the message sender.
    pub sender: Participant,
    /// Visibility and routing scope.
    pub visibility: MessageVisibility,
    /// Resolved list of recipient [`Participant`] records.
    pub recipients: Vec<Participant>,
    /// Semantic channel context in which the message was sent.
    pub channel_context: ChannelContext,
    /// Optional `message_id` of an earlier message being replied to.
    pub reply_to_message_id: Option<String>,
    /// UTF-8 text body of the chat message.
    pub body: String,
    /// Total Unicode character count of `body`.
    pub char_count: i32,
    /// Total whitespace-delimited word count of `body`.
    pub word_count: i32,
    /// Time spent composing the message in milliseconds.
    pub composition_time_ms: i64,
    /// Whether the message has been edited after initial transmission.
    pub is_edited: bool,
    /// Optional ML-inferred sentiment score (`-1.0` to `1.0`), defaulting to `None` when evolving from `v1`.
    pub sentiment_score: Option<f32>,
    /// Optional ML-classified conversational intent label, defaulting to `None` when evolving from `v1`.
    pub detected_intent: Option<String>,
}

/// Classification of a conversational or voice-activity turn in [`InteractionDynamicsEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InteractionTurnType {
    /// Onset of vocal speech detected by Web Audio VAD.
    SpeechStart,
    /// Completion of a vocal speech segment with aggregated RMS prosody statistics.
    SpeechEnd,
    /// Simultaneous speech overlapping one or more active speakers.
    InterruptionOverlap,
    /// Extended conversational pause exceeding silence thresholds.
    LongSilence,
    /// Non-verbal reaction or gesture signal.
    ReactionGesture,
}

/// Event emitted to the `interaction-dynamics` topic capturing non-textual prosody, turn-taking, and vocal energy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionDynamicsEvent {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// Participant whose vocal or turn-taking dynamics were measured.
    pub speaker: Participant,
    /// Type of conversational turn or prosody event.
    pub turn_type: InteractionTurnType,
    /// Duration of the speech turn or silence interval in milliseconds.
    pub duration_ms: i64,
    /// Mean root-mean-square (RMS) vocal energy over the turn window.
    pub rms_energy_mean: f32,
    /// Peak root-mean-square (RMS) vocal energy over the turn window.
    pub rms_energy_peak: f32,
    /// Optional fundamental pitch frequency estimate in Hertz.
    pub pitch_hz_estimate: Option<f32>,
    /// Other participants who were speaking simultaneously during an overlap.
    pub overlapped_participants: Vec<Participant>,
    /// Silence duration immediately preceding this turn in milliseconds.
    pub preceding_silence_ms: i64,
    /// Share of total session speaking time held by `speaker` (`0.0` to `1.0`).
    pub dominance_ratio: f32,
}

/// Category of simulation rule evaluation recorded in [`TtrpgMechanicsEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MechanicEventType {
    /// Non-combat skill or attribute d20 pool task resolution.
    TaskResolution,
    /// Count-up combat action with stance, time factor, and action count cost.
    CombatAction,
    /// Character fatigue, wound shock, or psychological status transition.
    StatusChange,
    /// Autonomous AI Game Master world-state or adjudication decision.
    AiGmDecision,
}

/// Degree of success or failure produced by a Satya d20 pool evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutcomeTier {
    /// Zero successes with at least one natural 20 rolled.
    CriticalFailure,
    /// Fewer successes than required by the difficulty rating.
    Failure,
    /// At least one success and within 1 success of the difficulty rating.
    PartialSuccess,
    /// Met the required difficulty rating.
    Success,
    /// Exceeded the difficulty rating by 2+ successes or rolled multiple natural 1s.
    ExtraordinarySuccess,
}

/// Melee or ranged tactical stance chosen for a Satya combat action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CombatStance {
    /// Heavy melee commitment prioritizing impact force (`TC 14`, increases fatigue).
    Power,
    /// Agile melee commitment prioritizing positioning and defense (`TC 11`, increases fatigue).
    Maneuver,
    /// Grounded melee stance conserving balance and endurance (`TC 9`).
    Stability,
    /// Deliberate aimed ranged fire (`TC 12`).
    RangedAimed,
    /// Quick reactive ranged snap shot (`TC 7`).
    RangedSnap,
}

/// Event emitted to the `ttrpg-mechanics` topic capturing Satya d20 pool rolls and count-up combat economy metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtrpgMechanicsEvent {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// High-level category of TTRPG mechanic evaluated.
    pub mechanic_type: MechanicEventType,
    /// Participant performing the task or combat action.
    pub actor: Participant,
    /// Optional target participants affected by the action.
    pub target_participants: Vec<Participant>,
    /// Name of the Satya skill being tested (e.g. `"Trauma Surgery"`).
    pub skill_name: Option<String>,
    /// Controlling attribute Target Number (`1..=20`) for roll-under evaluation.
    pub attribute_tn: Option<i32>,
    /// Number of successes required to achieve a full [`OutcomeTier::Success`].
    pub difficulty_rating: Option<i32>,
    /// Individual `d20` outcomes rolled in the dice pool.
    pub dice_rolled_d20: Vec<i32>,
    /// Total successes counted (including open-ended natural 1 bonus successes).
    pub successes_counted: i32,
    /// Resulting degree of success or failure.
    pub outcome_tier: OutcomeTier,
    /// Multiplier applied to base time cost due to fatigue and wound shock.
    pub action_time_factor: Option<f32>,
    /// Unmodified base time cost (`TC`) of the action or combat stance.
    pub action_time_cost: Option<i32>,
    /// Final tick cost added to the character's count-up action economy clock.
    pub action_count_cost: Option<i32>,
    /// Optional combat stance selected for the action.
    pub combat_stance: Option<CombatStance>,
    /// Character's current accumulated fatigue level.
    pub fatigue_level: i32,
    /// Character's current wound/shock penalty index.
    pub wound_shock_index: f32,
    /// Optional adjudication notes attached to the roll.
    pub notes: Option<String>,
}

impl TtrpgMechanicsEvent {
    /// Evaluates a Satya d20 pool roll against an attribute Target Number (TN) and difficulty rating.
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_satya_task(
        metadata: EventMetadata,
        actor: Participant,
        target_participants: Vec<Participant>,
        skill_name: String,
        attribute_tn: i32,
        difficulty_rating: i32,
        dice_rolled_d20: Vec<i32>,
        combat_stance: Option<CombatStance>,
        fatigue_level: i32,
        wound_shock_index: f32,
    ) -> Self {
        let mut successes = 0;
        let mut nat_20s = 0;
        let mut nat_1s = 0;
        for &roll in &dice_rolled_d20 {
            if roll <= attribute_tn {
                successes += 1;
            }
            if roll == 1 {
                successes += 1;
                nat_1s += 1;
            } else if roll == 20 {
                nat_20s += 1;
            }
        }

        let outcome_tier = if successes == 0 && nat_20s > 0 {
            OutcomeTier::CriticalFailure
        } else if successes < difficulty_rating {
            if successes > 0 && successes + 1 == difficulty_rating {
                OutcomeTier::PartialSuccess
            } else {
                OutcomeTier::Failure
            }
        } else if successes >= difficulty_rating + 2 || nat_1s >= 2 {
            OutcomeTier::ExtraordinarySuccess
        } else {
            OutcomeTier::Success
        };

        let base_tc = match combat_stance {
            Some(CombatStance::Power) => 14,
            Some(CombatStance::Maneuver) => 11,
            Some(CombatStance::Stability) => 9,
            Some(CombatStance::RangedAimed) => 12,
            Some(CombatStance::RangedSnap) => 7,
            None => 10,
        };
        let time_factor = 1.0 + (fatigue_level as f32 * 0.1) + (wound_shock_index * 0.25);
        let action_count_cost = ((base_tc as f32) * time_factor).round() as i32;

        Self {
            metadata,
            mechanic_type: if combat_stance.is_some() {
                MechanicEventType::CombatAction
            } else {
                MechanicEventType::TaskResolution
            },
            actor,
            target_participants,
            skill_name: Some(skill_name),
            attribute_tn: Some(attribute_tn),
            difficulty_rating: Some(difficulty_rating),
            dice_rolled_d20,
            successes_counted: successes,
            outcome_tier,
            action_time_factor: Some(time_factor),
            action_time_cost: Some(base_tc),
            action_count_cost: Some(action_count_cost),
            combat_stance,
            fatigue_level,
            wound_shock_index,
            notes: None,
        }
    }
}

/// Type of visual mutation recorded in a [`TabletopCanvasEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CanvasEventType {
    /// A new tactical map grid was generated.
    MapGenerated,
    /// An existing tactical map grid or image was updated.
    MapUpdated,
    /// A participant or NPC token moved to new grid coordinates.
    TokenMoved,
    /// A new token was placed on the tactical map.
    TokenAdded,
    /// A token was removed from the tactical map.
    TokenRemoved,
    /// Fog-of-war visibility masks were updated.
    FogOfWarUpdated,
    /// A synchronized 3D `wgpu` physics dice roll was spawned on the tabletop canvas.
    WgpuDiceSpawned,
}

/// Grid coordinates and visibility of a character or NPC token on a tactical map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenPlacement {
    /// Unique identifier for the token entity (e.g. `user_id` or `npc_id`).
    pub entity_id: String,
    /// Human-readable label rendered on or below the token.
    pub label: String,
    /// Horizontal grid coordinate (`0..grid_width`).
    pub x: i32,
    /// Vertical grid coordinate (`0..grid_height`).
    pub y: i32,
    /// Visibility scope of the token (`PUBLIC` or `GM_ONLY` for hidden enemies).
    pub visibility: MessageVisibility,
}

/// Deterministic 3D `wgpu` dice physics specification and outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WgpuDiceSpec {
    /// Standard dice expression (e.g. `"3d20"`, `"2d6+3"`).
    pub dice_notation: String,
    /// Deterministic PRNG seed so all connected browsers render identical 3D dice trajectories.
    pub physics_seed: i64,
    /// Individual die face outcomes rolled.
    pub outcomes: Vec<i32>,
    /// Visibility scope of the 3D dice roll (`PUBLIC` or `GM_ONLY` / `PRIVATE`).
    pub visibility: MessageVisibility,
}

/// Event (`v1`, `schema_id = 8`) emitted to the `tabletop-canvas` topic capturing tactical map updates,
/// token movements, and synchronized 3D `wgpu` dice physics rolls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabletopCanvasEvent {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// Specific tabletop canvas mutation type.
    pub canvas_event_type: CanvasEventType,
    /// Participant (human GM, player, or AI GM) who triggered the canvas mutation.
    pub actor: Participant,
    /// Optional identifier of the active tactical map.
    pub map_id: Option<String>,
    /// Optional human-readable title of the tactical map.
    pub map_title: Option<String>,
    /// Optional URL or relative path to the rendered SVG/PNG map asset.
    pub image_url: Option<String>,
    /// Optional tactical grid width in cells.
    pub grid_width: Option<i32>,
    /// Optional tactical grid height in cells.
    pub grid_height: Option<i32>,
    /// Active token coordinate placements on the map.
    #[serde(default)]
    pub token_placements: Vec<TokenPlacement>,
    /// Optional 3D `wgpu` dice roll specification when `canvas_event_type` is [`CanvasEventType::WgpuDiceSpawned`].
    pub wgpu_dice_roll: Option<WgpuDiceSpec>,
    /// Optional terrain, cover, or environmental hazard notes.
    pub terrain_notes: Option<String>,
}

/// Operating mode of the AI Game Master within a TTRPG session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AiGmOperatingMode {
    /// Solo AI Game Master mode (`TTRPG_AI_GM`: AI GM + Players, no human GM).
    #[default]
    Autonomous,
    /// Co-Pilot Assistant mode (`TTRPG_GM_AI`: Human GM + AI Assistant + Players).
    CoPilot,
}

/// Architectural tier of an MCP tool server invoked by the AI Game Master.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpServerTier {
    /// Tier 1: Core Satya tabletop primitives (chat, whispers, map grid, 3D `wgpu` dice, session history).
    #[serde(rename = "TIER_1_SATYA_CORE")]
    Tier1SatyaCore,
    /// Tier 2: Pluggable game engine ruleset (e.g. `satya-d20`, `dnd5e`, `fitd`).
    #[serde(rename = "TIER_2_RULESET_ENGINE")]
    Tier2RulesetEngine,
    /// Tier 3: Campaign lore, NPC sheets, and world state (local Markdown/PyLance or Google Workspace).
    #[serde(rename = "TIER_3_LORE_PROVIDER")]
    Tier3LoreProvider,
}

/// Audit record of a single MCP `@tool` invocation within an [`AiGmTrajectoryEvent`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolInvocationRecord {
    /// Zero-based sequential index of this tool call within the LangGraph turn.
    pub step_index: i32,
    /// 16-character hex OpenTelemetry `span_id` for this tool call.
    pub span_id: String,
    /// MCP server tier classification.
    pub mcp_tier: McpServerTier,
    /// Identifier of the MCP server (e.g. `"satya-core-table"`, `"satya-d20-rules"`).
    pub server_id: String,
    /// Name of the invoked tool (e.g. `"lookup_npc_profile"`, `"roll_satya_d20_check"`).
    pub tool_name: String,
    /// Redacted JSON string of tool arguments (guaranteed free of API keys or tokens).
    pub arguments_json: String,
    /// Structured JSON or text summary returned by the tool.
    pub result_summary_json: String,
    /// Whether the tool invocation resulted in an error.
    pub is_error: bool,
    /// Execution latency of the tool call in milliseconds.
    pub duration_ms: i64,
}

/// Completion status of a LangGraph AI Game Master turn recorded in [`AiGmTrajectoryEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AiGmTurnStatus {
    /// Turn completed normally and emitted its response.
    Completed,
    /// Turn completed with one or more recoverable tool errors.
    PartialError,
    /// Turn was cancelled or overridden by a human GM.
    AbortedByGm,
    /// Turn failed due to an upstream LLM provider error or unreachable endpoint.
    ProviderError,
}

/// Event (`v1`, `schema_id = 9`) emitted to the `ai-gm-trajectories` topic once per completed
/// LangGraph AI GM turn, capturing tool calls, RAG sources, token counts, and W3C `trace_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiGmTrajectoryEvent {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// Unique UUIDv7 identifier for this LangGraph turn execution.
    pub turn_id: String,
    /// 32-character lowercase hex W3C OpenTelemetry `trace_id`.
    pub trace_id: String,
    /// Operating mode (`AUTONOMOUS` vs. `CO_PILOT`).
    pub operating_mode: AiGmOperatingMode,
    /// [`Participant`] record of the AI GM agent.
    pub ai_participant: Participant,
    /// Human [`Participant`] whose chat, whisper, or dice roll triggered this turn.
    pub trigger_actor: Participant,
    /// `event_id` or `message_id` of the triggering event.
    pub trigger_event_id: String,
    /// Text summary of the player/GM action that triggered the turn.
    pub trigger_summary: String,
    /// LLM provider identifier (e.g. `"ollama"`, `"google_genai"`, `"openai"`, `"anthropic"`).
    pub llm_provider: String,
    /// Reasoning model identifier (e.g. `"qwen2.5:14b"`, `"gemini-2.5-flash"`).
    pub reasoning_model_id: String,
    /// Optional multimodal vision model identifier used for map inspection.
    pub vision_model_id: Option<String>,
    /// Active game ruleset identifier (e.g. `"satya-d20"`).
    pub ruleset_engine_id: String,
    /// Ordered list of MCP/LangChain tool invocations executed during the turn.
    #[serde(default)]
    pub tool_invocations: Vec<McpToolInvocationRecord>,
    /// URIs or document IDs of lore/rules sources consulted during the turn.
    #[serde(default)]
    pub rag_sources_used: Vec<String>,
    /// `event_id`s of domain events (`ChatMessageEvent`, `TtrpgMechanicsEvent`, `TabletopCanvasEvent`) emitted during the turn.
    #[serde(default)]
    pub emitted_event_ids: Vec<String>,
    /// Total input/prompt tokens consumed across LLM calls in this turn.
    pub prompt_tokens: i32,
    /// Total output/completion tokens generated across LLM calls in this turn.
    pub completion_tokens: i32,
    /// Tokens served from a provider context cache (e.g. Gemini `CachedContent`).
    #[serde(default)]
    pub cached_tokens: i32,
    /// End-to-end turn execution latency in milliseconds.
    pub total_turn_latency_ms: i64,
    /// Final completion status of the turn.
    pub turn_status: AiGmTurnStatus,
}

/// Severity level for a [`SessionLogEvent`] in the `system-logs` topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogLevel {
    /// Fine-grained trace diagnostics.
    Trace,
    /// Debug diagnostics.
    Debug,
    /// Informational operational milestone.
    Info,
    /// Non-fatal warning condition.
    Warn,
    /// Operational or request error.
    Error,
    /// Critical unrecoverable failure.
    Fatal,
}

/// Event (`v1`, `schema_id = 10`) emitted to the `system-logs` topic storing embedding-ready
/// plain-text log lines prefixed with `[session=...] [trace=... span=...<-...] [lineage=...<-...]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionLogEvent {
    /// Common event metadata envelope.
    pub metadata: EventMetadata,
    /// Unique UUIDv7 identifier for this log entry.
    pub log_id: String,
    /// Log severity level.
    pub level: LogLevel,
    /// Emitting service name (e.g. `"satya-signaling"`, `"satya-ml-ai-gm"`).
    pub service_name: String,
    /// Optional [`Participant`] associated with this log entry for GDPR subject matching.
    pub actor: Option<Participant>,
    /// Optional 32-character hex W3C `trace_id`.
    pub trace_id: Option<String>,
    /// Optional 16-character hex W3C `span_id`.
    pub span_id: Option<String>,
    /// Optional 16-character hex caller `parent_span_id`.
    pub parent_span_id: Option<String>,
    /// Optional data artifact lineage ID produced or inspected at this step.
    pub lineage_id: Option<String>,
    /// Upstream data artifact lineage IDs consumed at this step.
    #[serde(default)]
    pub parent_lineage_ids: Vec<String>,
    /// Complete formatted plain-text log line ready for direct text embedding.
    pub log_line: String,
}
