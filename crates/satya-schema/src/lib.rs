//! Versioned Apache Avro schemas, 64-bit `CRC-64-AVRO` canonical fingerprinting,
//! 15-byte wire framing headers, and shared client/server protocol types for the
//! Satya WebRTC, Chat, TTRPG Mechanics, and Apache Iggy Event Streaming platform.

#![deny(missing_docs)]

use apache_avro::{Schema as AvroSchema, from_avro_datum, from_value, to_avro_datum, to_value};
use chrono::Utc;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::io::Cursor;
use thiserror::Error;
use uuid::Uuid;

/// Magic leading byte (`0x53`, ASCII `'S'`) identifying a framed Satya wire message.
pub const WIRE_MAGIC_BYTE: u8 = 0x53; // 'S' for Satya
/// Total byte length of the binary [`WireFrameHeader`] (`1` magic + `4` schema_id + `2` version + `8` crc64).
pub const WIRE_HEADER_LEN: usize = 15; // 1 (magic) + 4 (schema_id) + 2 (version) + 8 (crc64)

/// Raw JSON schema definition (`v1`) for [`SchemaRegistryEvent`].
pub const SCHEMA_REGISTRY_V1_AVSC: &str =
    include_str!("../../../schemas/v1/schema_registry_event.avsc");
/// Raw JSON schema definition (`v1`) for [`SessionLifecycleEvent`].
pub const SESSION_LIFECYCLE_V1_AVSC: &str =
    include_str!("../../../schemas/v1/session_lifecycle_event.avsc");
/// Raw JSON schema definition (`v1`) for [`WebRtcTelemetryEvent`].
pub const WEBRTC_TELEMETRY_V1_AVSC: &str =
    include_str!("../../../schemas/v1/webrtc_telemetry_event.avsc");
/// Raw JSON schema definition (`v1`) for [`ChatMessageEvent`].
pub const CHAT_MESSAGE_V1_AVSC: &str = include_str!("../../../schemas/v1/chat_message_event.avsc");
/// Raw JSON schema definition (`v1`) for [`InteractionDynamicsEvent`].
pub const INTERACTION_DYNAMICS_V1_AVSC: &str =
    include_str!("../../../schemas/v1/interaction_dynamics_event.avsc");
/// Raw JSON schema definition (`v1`) for [`TtrpgMechanicsEvent`].
pub const TTRPG_MECHANICS_V1_AVSC: &str =
    include_str!("../../../schemas/v1/ttrpg_mechanics_event.avsc");
/// Evolved raw JSON schema definition (`v2`) for [`ChatMessageEventV2`].
pub const CHAT_MESSAGE_V2_AVSC: &str = include_str!("../../../schemas/v2/chat_message_event.avsc");

/// Errors returned during Avro schema registration, wire header parsing, or payload serialization/deserialization.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// Underlying Apache Avro parsing, validation, or encoding/decoding error.
    #[error("Avro error: {0}")]
    Avro(#[source] Box<apache_avro::Error>),
    /// Requested `(subject, version)` pair is not registered in the [`SchemaRegistry`].
    #[error("Schema not found for subject '{subject}' version {version}")]
    SubjectVersionNotFound {
        /// Fully qualified Avro schema subject name.
        subject: String,
        /// Requested schema version number.
        version: u16,
    },
    /// Requested numeric `schema_id` is not registered in the [`SchemaRegistry`].
    #[error("Schema ID {0} not found in registry")]
    SchemaIdNotFound(u32),
    /// Wire frame buffer is malformed or has an invalid magic byte.
    #[error("Invalid wire frame header: {0}")]
    InvalidWireHeader(String),
    /// Wire frame `CRC-64-AVRO` fingerprint does not match the registered writer schema.
    #[error("Schema fingerprint mismatch: expected {expected}, got {actual}")]
    FingerprintMismatch {
        /// Expected hex-encoded 64-bit fingerprint from the registry.
        expected: String,
        /// Actual hex-encoded 64-bit fingerprint found in the wire header.
        actual: String,
    },
}

impl From<apache_avro::Error> for SchemaError {
    fn from(err: apache_avro::Error) -> Self {
        Self::Avro(Box::new(err))
    }
}

/// Computes the standard 64-bit CRC-64-AVRO fingerprint over the canonical parsing form of a schema.
pub fn crc64_avro_fingerprint(canonical_form: &[u8]) -> [u8; 8] {
    const EMPTY: u64 = 0xc15d_213a_a4d7_a795;
    let mut fp: u64 = EMPTY;
    for &b in canonical_form {
        let idx = ((fp ^ (b as u64)) & 0xff) as usize;
        let mut entry = idx as u64;
        for _ in 0..8 {
            entry = (entry >> 1) ^ (EMPTY & (0u64.wrapping_sub(entry & 1)));
        }
        fp = (fp >> 8) ^ entry;
    }
    fp.to_be_bytes()
}

// ============================================================================
// Core Nested Avro Types (`com.satya.types`)
// ============================================================================

/// High-level business operating mode for a Satya session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionMode {
    /// Tabletop roleplaying simulation mode (GM, AI GM, and players).
    Ttrpg,
    /// Multi-participant audio/video and chat meeting mode (Host and attendees).
    Meeting,
}

/// Distinguishes human users from autonomous AI agent participants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParticipantKind {
    /// A human participant authenticated via username and password.
    Human,
    /// An autonomous AI agent participant (such as the Satya AI Game Master).
    AiAgent,
}

/// Role assigned to a [`Participant`] within a TTRPG or Meeting session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionRole {
    /// Human Game Master adjudicating a TTRPG session.
    Gm,
    /// Autonomous AI Game Master adjudicating a TTRPG session.
    AiGm,
    /// Player character participant in a TTRPG session.
    Player,
    /// Host of a Meeting session.
    Host,
    /// Standard attendee in a Meeting session.
    Participant,
    /// Read/listen-only observer in a session.
    Observer,
}

/// Metadata describing an autonomous AI agent participant's model and persona configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiAgentInfo {
    /// Identifier of the underlying foundation or local ML model.
    pub model_id: String,
    /// Identifier of the prompt/persona profile driving the AI agent.
    pub persona_id: String,
    /// Sampling temperature configured for the AI agent.
    pub temperature: f32,
}

/// Nested `Participant` record shared across all Satya event schemas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    /// Persistent UUIDv7 user identifier across sessions.
    pub user_id: String,
    /// Unique account username.
    pub username: String,
    /// Human-readable display or character name shown in the session UI.
    pub display_name: String,
    /// Whether this participant is a human user or an autonomous AI agent.
    pub participant_kind: ParticipantKind,
    /// Assigned role within the session.
    pub role: SessionRole,
    /// Optional character sheet identifier when participating in TTRPG mode.
    pub character_id: Option<String>,
    /// Optional AI model and persona metadata when `participant_kind` is [`ParticipantKind::AiAgent`].
    pub ai_agent_info: Option<AiAgentInfo>,
}

impl Participant {
    /// Constructs a human [`Participant`] record with the given identifiers and session role.
    pub fn human(
        user_id: impl Into<String>,
        username: impl Into<String>,
        display_name: impl Into<String>,
        role: SessionRole,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            username: username.into(),
            display_name: display_name.into(),
            participant_kind: ParticipantKind::Human,
            role,
            character_id: None,
            ai_agent_info: None,
        }
    }

    /// Constructs an autonomous AI Game Master [`Participant`] record for a TTRPG session.
    pub fn ai_gm(session_id: &str, model_id: &str, persona_id: &str) -> Self {
        Self {
            user_id: format!("ai-gm-{session_id}"),
            username: "satya_ai_gm".to_string(),
            display_name: "Satya AI Game Master".to_string(),
            participant_kind: ParticipantKind::AiAgent,
            role: SessionRole::AiGm,
            character_id: None,
            ai_agent_info: Some(AiAgentInfo {
                model_id: model_id.to_string(),
                persona_id: persona_id.to_string(),
                temperature: 0.35,
            }),
        }
    }
}

/// Nested `EventMetadata` envelope record embedded in every Satya event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique UUIDv7 identifier for this event instance.
    pub event_id: String,
    /// Monotonically increasing sequence number assigned by the signaling server.
    pub sequence_num: i64,
    /// Fully qualified Avro schema subject name (e.g. `"com.satya.events.ChatMessageEvent"`).
    pub schema_subject: String,
    /// Schema version number against which this event was created.
    pub schema_version: i32,
    /// 8-byte `CRC-64-AVRO` canonical schema fingerprint.
    #[serde(with = "serde_bytes_vec")]
    pub schema_fingerprint_crc64: Vec<u8>,
    /// Identifier of the session in which this event occurred.
    pub session_id: String,
    /// Operating mode (`TTRPG` or `MEETING`) of the session.
    pub session_mode: SessionMode,
    /// Client creation timestamp in microseconds since the Unix epoch.
    pub client_timestamp_us: i64,
    /// Server ingestion timestamp in microseconds since the Unix epoch.
    pub ingest_timestamp_us: i64,
    /// Client timezone offset from UTC in minutes.
    pub timezone_offset_min: i32,
}

impl EventMetadata {
    /// Creates a new [`EventMetadata`] envelope stamped with the current UTC microsecond timestamp.
    pub fn new(
        sequence_num: i64,
        schema_subject: impl Into<String>,
        schema_version: u16,
        fingerprint_crc64: [u8; 8],
        session_id: impl Into<String>,
        session_mode: SessionMode,
    ) -> Self {
        let now_us = Utc::now().timestamp_micros();
        Self {
            event_id: Uuid::now_v7().to_string(),
            sequence_num,
            schema_subject: schema_subject.into(),
            schema_version: schema_version as i32,
            schema_fingerprint_crc64: fingerprint_crc64.to_vec(),
            session_id: session_id.into(),
            session_mode,
            client_timestamp_us: now_us,
            ingest_timestamp_us: now_us,
            timezone_offset_min: 0,
        }
    }
}

mod serde_bytes_vec {
    use serde::{Deserializer, Serializer, de::Visitor};
    use std::fmt;

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a byte array or sequence of u8")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(v.to_vec())
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(v)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut out = Vec::new();
                while let Some(b) = seq.next_element::<u8>()? {
                    out.push(b);
                }
                Ok(out)
            }
        }

        deserializer.deserialize_bytes(BytesVisitor)
    }
}

// ============================================================================
// Domain Event Structs (`com.satya.events`)
// ============================================================================

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
                // Open-ended low roll critical success in roll-under or bonus success
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

// ============================================================================
// Shared Client <-> Server HTTP & WebSocket Protocol Types
// ============================================================================

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
    /// Optional reason string recorded in [`SessionLifecycleEvent`].
    pub reason: Option<String>,
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

// ============================================================================
// Schema Registry & Wire Framing Header
// ============================================================================

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
}

/// Parsed and fingerprinted Avro schema entry stored inside [`SchemaRegistry`].
#[derive(Debug, Clone)]
pub struct RegisteredSchema {
    /// Unique numeric identifier for this schema version.
    pub schema_id: u32,
    /// Fully qualified Avro schema subject name.
    pub subject: String,
    /// Target Apache Iggy topic name.
    pub topic: String,
    /// Schema version number (`1`, `2`, ...).
    pub version: u16,
    /// 8-byte `CRC-64-AVRO` fingerprint computed over `canonical_form`.
    pub fingerprint_crc64: [u8; 8],
    /// Original raw JSON `.avsc` definition.
    pub raw_json: String,
    /// Normalized Avro Parsing Canonical Form string.
    pub canonical_form: String,
    /// Parsed `apache_avro::Schema` instance.
    pub avro_schema: AvroSchema,
}

impl RegisteredSchema {
    /// Returns the 16-character lowercase hexadecimal representation of `fingerprint_crc64`.
    pub fn fingerprint_hex(&self) -> String {
        hex::encode(self.fingerprint_crc64)
    }
}

/// 15-byte binary wire framing header prepended to every Avro message in Apache Iggy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireFrameHeader {
    /// Leading magic byte (`0x53`).
    pub magic_byte: u8,
    /// Numeric schema identifier in the [`SchemaRegistry`].
    pub schema_id: u32,
    /// Schema version number used by the writer when encoding the payload.
    pub schema_version: u16,
    /// 8-byte `CRC-64-AVRO` fingerprint of the writer schema.
    pub fingerprint_crc64: [u8; 8],
}

impl WireFrameHeader {
    /// Serializes this wire header into its 15-byte big-endian binary representation.
    pub fn to_bytes(&self) -> [u8; WIRE_HEADER_LEN] {
        let mut buf = [0u8; WIRE_HEADER_LEN];
        buf[0] = self.magic_byte;
        buf[1..5].copy_from_slice(&self.schema_id.to_be_bytes());
        buf[5..7].copy_from_slice(&self.schema_version.to_be_bytes());
        buf[7..15].copy_from_slice(&self.fingerprint_crc64);
        buf
    }

    /// Parses a 15-byte binary wire header from the start of `bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SchemaError> {
        if bytes.len() < WIRE_HEADER_LEN {
            return Err(SchemaError::InvalidWireHeader(format!(
                "Buffer too short: {} < {}",
                bytes.len(),
                WIRE_HEADER_LEN
            )));
        }
        if bytes[0] != WIRE_MAGIC_BYTE {
            return Err(SchemaError::InvalidWireHeader(format!(
                "Invalid magic byte: 0x{:02x}",
                bytes[0]
            )));
        }
        let schema_id = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        let schema_version = u16::from_be_bytes([bytes[5], bytes[6]]);
        let mut fingerprint_crc64 = [0u8; 8];
        fingerprint_crc64.copy_from_slice(&bytes[7..15]);

        Ok(Self {
            magic_byte: bytes[0],
            schema_id,
            schema_version,
            fingerprint_crc64,
        })
    }
}

/// In-memory registry of versioned Avro schemas supporting `CRC-64-AVRO` fingerprinting
/// and writer-to-reader schema evolution.
#[derive(Debug, Clone)]
pub struct SchemaRegistry {
    by_id: HashMap<u32, RegisteredSchema>,
    by_subject_version: HashMap<(String, u16), u32>,
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new_with_builtin_schemas().expect("Builtin Avro schemas must be valid")
    }
}

impl SchemaRegistry {
    /// Creates a [`SchemaRegistry`] pre-populated with all built-in Satya `v1` and `v2` Avro schemas.
    pub fn new_with_builtin_schemas() -> Result<Self, SchemaError> {
        let mut registry = Self {
            by_id: HashMap::new(),
            by_subject_version: HashMap::new(),
        };

        registry.register(
            1,
            subjects::SCHEMA_REGISTRY,
            topics::SCHEMA_REGISTRY,
            1,
            SCHEMA_REGISTRY_V1_AVSC,
        )?;
        registry.register(
            2,
            subjects::SESSION_LIFECYCLE,
            topics::SESSION_LIFECYCLE,
            1,
            SESSION_LIFECYCLE_V1_AVSC,
        )?;
        registry.register(
            3,
            subjects::WEBRTC_TELEMETRY,
            topics::WEBRTC_TELEMETRY,
            1,
            WEBRTC_TELEMETRY_V1_AVSC,
        )?;
        registry.register(
            4,
            subjects::CHAT_MESSAGE,
            topics::CHAT_MESSAGE,
            1,
            CHAT_MESSAGE_V1_AVSC,
        )?;
        registry.register(
            5,
            subjects::INTERACTION_DYNAMICS,
            topics::INTERACTION_DYNAMICS,
            1,
            INTERACTION_DYNAMICS_V1_AVSC,
        )?;
        registry.register(
            6,
            subjects::TTRPG_MECHANICS,
            topics::TTRPG_MECHANICS,
            1,
            TTRPG_MECHANICS_V1_AVSC,
        )?;
        // Evolved v2 schema for ChatMessageEvent
        registry.register(
            7,
            subjects::CHAT_MESSAGE,
            topics::CHAT_MESSAGE,
            2,
            CHAT_MESSAGE_V2_AVSC,
        )?;

        Ok(registry)
    }

    /// Parses, fingerprints, and registers a versioned Avro schema under `(subject, version)` and `schema_id`.
    pub fn register(
        &mut self,
        schema_id: u32,
        subject: impl Into<String>,
        topic: impl Into<String>,
        version: u16,
        raw_json: &str,
    ) -> Result<&RegisteredSchema, SchemaError> {
        let subject = subject.into();
        let topic = topic.into();
        let avro_schema = AvroSchema::parse_str(raw_json)?;
        let canonical_form = avro_schema.canonical_form();
        let fingerprint_crc64 = crc64_avro_fingerprint(canonical_form.as_bytes());

        let entry = RegisteredSchema {
            schema_id,
            subject: subject.clone(),
            topic,
            version,
            fingerprint_crc64,
            raw_json: raw_json.to_string(),
            canonical_form,
            avro_schema,
        };

        self.by_subject_version
            .insert((subject, version), schema_id);
        self.by_id.insert(schema_id, entry);
        Ok(self.by_id.get(&schema_id).unwrap())
    }

    /// Looks up a registered schema by its `subject` name and `version` number.
    pub fn get_by_subject_version(
        &self,
        subject: &str,
        version: u16,
    ) -> Result<&RegisteredSchema, SchemaError> {
        let id = self
            .by_subject_version
            .get(&(subject.to_string(), version))
            .copied()
            .ok_or_else(|| SchemaError::SubjectVersionNotFound {
                subject: subject.to_string(),
                version,
            })?;
        self.get_by_id(id)
    }

    /// Looks up a registered schema by its numeric `schema_id`.
    pub fn get_by_id(&self, schema_id: u32) -> Result<&RegisteredSchema, SchemaError> {
        self.by_id
            .get(&schema_id)
            .ok_or(SchemaError::SchemaIdNotFound(schema_id))
    }

    /// Returns all registered schemas sorted in ascending order by `schema_id`.
    pub fn all_schemas(&self) -> Vec<&RegisteredSchema> {
        let mut list: Vec<&RegisteredSchema> = self.by_id.values().collect();
        list.sort_by_key(|s| s.schema_id);
        list
    }

    /// Creates a `EventMetadata` populated with the exact version & CRC-64 fingerprint from the registry.
    pub fn build_metadata(
        &self,
        sequence_num: i64,
        subject: &str,
        version: u16,
        session_id: impl Into<String>,
        session_mode: SessionMode,
    ) -> Result<EventMetadata, SchemaError> {
        let reg = self.get_by_subject_version(subject, version)?;
        Ok(EventMetadata::new(
            sequence_num,
            subject,
            version,
            reg.fingerprint_crc64,
            session_id,
            session_mode,
        ))
    }

    /// Serializes a domain event into a 15-byte wire header + Avro binary datum.
    pub fn encode_framed<T: Serialize>(
        &self,
        subject: &str,
        version: u16,
        record: &T,
    ) -> Result<Vec<u8>, SchemaError> {
        let reg = self.get_by_subject_version(subject, version)?;
        let avro_value = to_value(record)?;
        let datum_bytes = to_avro_datum(&reg.avro_schema, avro_value)?;

        let header = WireFrameHeader {
            magic_byte: WIRE_MAGIC_BYTE,
            schema_id: reg.schema_id,
            schema_version: reg.version,
            fingerprint_crc64: reg.fingerprint_crc64,
        };

        let mut framed = Vec::with_capacity(WIRE_HEADER_LEN + datum_bytes.len());
        framed.extend_from_slice(&header.to_bytes());
        framed.extend_from_slice(&datum_bytes);
        Ok(framed)
    }

    /// Decodes a framed Avro payload using the writer schema embedded in the wire header,
    /// optionally resolving into a newer/older `reader_version` for seamless schema evolution.
    pub fn decode_framed<T: DeserializeOwned>(
        &self,
        framed_bytes: &[u8],
        reader_version: Option<u16>,
    ) -> Result<(WireFrameHeader, T), SchemaError> {
        let header = WireFrameHeader::from_bytes(framed_bytes)?;
        let writer_reg = self.get_by_id(header.schema_id)?;

        if writer_reg.fingerprint_crc64 != header.fingerprint_crc64 {
            return Err(SchemaError::FingerprintMismatch {
                expected: writer_reg.fingerprint_hex(),
                actual: hex::encode(header.fingerprint_crc64),
            });
        }

        let mut cursor = Cursor::new(&framed_bytes[WIRE_HEADER_LEN..]);
        let avro_value = if let Some(target_ver) = reader_version {
            let reader_reg = self.get_by_subject_version(&writer_reg.subject, target_ver)?;
            from_avro_datum(
                &writer_reg.avro_schema,
                &mut cursor,
                Some(&reader_reg.avro_schema),
            )?
        } else {
            from_avro_datum(&writer_reg.avro_schema, &mut cursor, None)?
        };

        let decoded: T = from_value(&avro_value)?;
        Ok((header, decoded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_builtin_schemas_register_and_fingerprint() {
        let registry = SchemaRegistry::new_with_builtin_schemas().unwrap();
        let all = registry.all_schemas();
        assert_eq!(all.len(), 7);

        let chat_v1 = registry
            .get_by_subject_version(subjects::CHAT_MESSAGE, 1)
            .unwrap();
        let chat_v2 = registry
            .get_by_subject_version(subjects::CHAT_MESSAGE, 2)
            .unwrap();

        assert_ne!(chat_v1.fingerprint_crc64, chat_v2.fingerprint_crc64);
        assert_eq!(chat_v1.fingerprint_hex().len(), 16);
    }

    #[test]
    fn test_nested_participant_and_schema_evolution_v1_to_v2() {
        let registry = SchemaRegistry::default();
        let gm = Participant::human("u-gm-1", "sean_gm", "GM Sean", SessionRole::Gm);
        let player = Participant::human("u-pl-1", "sara_p", "Sara (Valkyrie)", SessionRole::Player);

        let metadata = registry
            .build_metadata(
                1,
                subjects::CHAT_MESSAGE,
                1,
                "harsh-reality-01",
                SessionMode::Ttrpg,
            )
            .unwrap();

        let msg_v1 = ChatMessageEvent {
            metadata,
            message_id: Uuid::now_v7().to_string(),
            sender: gm.clone(),
            visibility: MessageVisibility::Private,
            recipients: vec![player.clone()],
            channel_context: ChannelContext::GmNarration,
            reply_to_message_id: None,
            body: "You hear footsteps behind the bulkhead.".to_string(),
            char_count: 40,
            word_count: 6,
            composition_time_ms: 2150,
            is_edited: false,
        };

        let framed = registry
            .encode_framed(subjects::CHAT_MESSAGE, 1, &msg_v1)
            .unwrap();

        // 1. Decode using writer version (v1)
        let (hdr, roundtrip_v1): (_, ChatMessageEvent) =
            registry.decode_framed(&framed, None).unwrap();
        assert_eq!(hdr.schema_version, 1);
        assert_eq!(roundtrip_v1.sender.username, "sean_gm");
        assert_eq!(roundtrip_v1.recipients[0].display_name, "Sara (Valkyrie)");

        // 2. Evolve v1 message into v2 reader schema seamlessly
        let (hdr_v2, evolved_v2): (_, ChatMessageEventV2) =
            registry.decode_framed(&framed, Some(2)).unwrap();
        assert_eq!(hdr_v2.schema_version, 1); // Preserves original writer version!
        assert_eq!(evolved_v2.body, msg_v1.body);
        assert_eq!(evolved_v2.sentiment_score, None);
        assert_eq!(evolved_v2.detected_intent, None);
    }

    #[test]
    fn test_interaction_dynamics_and_ttrpg_mechanics_serialization() {
        let registry = SchemaRegistry::default();
        let speaker = Participant::human("u-1", "alex", "Alex", SessionRole::Host);
        let overlapped = Participant::human("u-2", "blair", "Blair", SessionRole::Participant);

        let dyn_meta = registry
            .build_metadata(
                10,
                subjects::INTERACTION_DYNAMICS,
                1,
                "meet-1",
                SessionMode::Meeting,
            )
            .unwrap();
        let dyn_event = InteractionDynamicsEvent {
            metadata: dyn_meta,
            speaker: speaker.clone(),
            turn_type: InteractionTurnType::InterruptionOverlap,
            duration_ms: 1840,
            rms_energy_mean: 0.42,
            rms_energy_peak: 0.88,
            pitch_hz_estimate: Some(165.5),
            overlapped_participants: vec![overlapped],
            preceding_silence_ms: 0,
            dominance_ratio: 0.61,
        };

        let framed_dyn = registry
            .encode_framed(subjects::INTERACTION_DYNAMICS, 1, &dyn_event)
            .unwrap();
        let (_, decoded_dyn): (_, InteractionDynamicsEvent) =
            registry.decode_framed(&framed_dyn, None).unwrap();
        assert_eq!(decoded_dyn.overlapped_participants.len(), 1);

        let mech_meta = registry
            .build_metadata(
                11,
                subjects::TTRPG_MECHANICS,
                1,
                "satya-1",
                SessionMode::Ttrpg,
            )
            .unwrap();
        let mech_event = TtrpgMechanicsEvent::evaluate_satya_task(
            mech_meta,
            speaker,
            vec![],
            "Lockpicking".to_string(),
            13,
            2,
            vec![4, 11, 17],
            Some(CombatStance::Maneuver),
            2,
            0.5,
        );
        let framed_mech = registry
            .encode_framed(subjects::TTRPG_MECHANICS, 1, &mech_event)
            .unwrap();
        let (_, decoded_mech): (_, TtrpgMechanicsEvent) =
            registry.decode_framed(&framed_mech, None).unwrap();
        assert_eq!(decoded_mech.successes_counted, 2);
        assert_eq!(decoded_mech.outcome_tier, OutcomeTier::Success);
    }

    #[test]
    fn test_disconnect_and_logout_lifecycle_and_webrtc_events() {
        let registry = SchemaRegistry::default();
        let player = Participant::human("u-pl-1", "sara_p", "Sara (Valkyrie)", SessionRole::Player);

        for lifecycle_type in [
            LifecycleEventType::ParticipantLeft,
            LifecycleEventType::ParticipantDisconnected,
            LifecycleEventType::UserLoggedOut,
        ] {
            let meta = registry
                .build_metadata(
                    20,
                    subjects::SESSION_LIFECYCLE,
                    1,
                    "harsh-reality-01",
                    SessionMode::Ttrpg,
                )
                .unwrap();
            let ev = SessionLifecycleEvent {
                metadata: meta,
                lifecycle_type,
                actor: player.clone(),
                target_participant: None,
                password_protected: true,
                roster_snapshot: vec![],
                reason: Some("User disconnected from session".to_string()),
            };
            let framed = registry
                .encode_framed(subjects::SESSION_LIFECYCLE, 1, &ev)
                .unwrap();
            let (_, decoded): (_, SessionLifecycleEvent) =
                registry.decode_framed(&framed, None).unwrap();
            assert_eq!(decoded.lifecycle_type, lifecycle_type);
        }

        let tel_meta = registry
            .build_metadata(
                21,
                subjects::WEBRTC_TELEMETRY,
                1,
                "harsh-reality-01",
                SessionMode::Ttrpg,
            )
            .unwrap();
        let tel_ev = WebRtcTelemetryEvent {
            metadata: tel_meta,
            local_participant: player,
            remote_participant: None,
            event_kind: WebRtcEventKind::PeerDisconnected,
            ice_candidate_type: None,
            rtt_ms: None,
            jitter_ms: None,
            packet_loss_rate: None,
            available_outgoing_bitrate_bps: None,
            audio_muted: true,
            video_muted: true,
        };
        let framed_tel = registry
            .encode_framed(subjects::WEBRTC_TELEMETRY, 1, &tel_ev)
            .unwrap();
        let (_, decoded_tel): (_, WebRtcTelemetryEvent) =
            registry.decode_framed(&framed_tel, None).unwrap();
        assert_eq!(decoded_tel.event_kind, WebRtcEventKind::PeerDisconnected);
    }
}
