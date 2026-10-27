use apache_avro::{Schema as AvroSchema, from_avro_datum, from_value, to_avro_datum, to_value};
use chrono::Utc;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::io::Cursor;
use thiserror::Error;
use uuid::Uuid;

pub const WIRE_MAGIC_BYTE: u8 = 0x53; // 'S' for Satya
pub const WIRE_HEADER_LEN: usize = 15; // 1 (magic) + 4 (schema_id) + 2 (version) + 8 (crc64)

pub const SCHEMA_REGISTRY_V1_AVSC: &str =
    include_str!("../../../schemas/v1/schema_registry_event.avsc");
pub const SESSION_LIFECYCLE_V1_AVSC: &str =
    include_str!("../../../schemas/v1/session_lifecycle_event.avsc");
pub const WEBRTC_TELEMETRY_V1_AVSC: &str =
    include_str!("../../../schemas/v1/webrtc_telemetry_event.avsc");
pub const CHAT_MESSAGE_V1_AVSC: &str = include_str!("../../../schemas/v1/chat_message_event.avsc");
pub const INTERACTION_DYNAMICS_V1_AVSC: &str =
    include_str!("../../../schemas/v1/interaction_dynamics_event.avsc");
pub const TTRPG_MECHANICS_V1_AVSC: &str =
    include_str!("../../../schemas/v1/ttrpg_mechanics_event.avsc");
pub const CHAT_MESSAGE_V2_AVSC: &str = include_str!("../../../schemas/v2/chat_message_event.avsc");

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("Avro error: {0}")]
    Avro(#[source] Box<apache_avro::Error>),
    #[error("Schema not found for subject '{subject}' version {version}")]
    SubjectVersionNotFound { subject: String, version: u16 },
    #[error("Schema ID {0} not found in registry")]
    SchemaIdNotFound(u32),
    #[error("Invalid wire frame header: {0}")]
    InvalidWireHeader(String),
    #[error("Schema fingerprint mismatch: expected {expected}, got {actual}")]
    FingerprintMismatch { expected: String, actual: String },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionMode {
    Ttrpg,
    Meeting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParticipantKind {
    Human,
    AiAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionRole {
    Gm,
    AiGm,
    Player,
    Host,
    Participant,
    Observer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiAgentInfo {
    pub model_id: String,
    pub persona_id: String,
    pub temperature: f32,
}

/// Nested `Participant` record shared across all Satya event schemas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub participant_kind: ParticipantKind,
    pub role: SessionRole,
    pub character_id: Option<String>,
    pub ai_agent_info: Option<AiAgentInfo>,
}

impl Participant {
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
    pub event_id: String,
    pub sequence_num: i64,
    pub schema_subject: String,
    pub schema_version: i32,
    #[serde(with = "serde_bytes_vec")]
    pub schema_fingerprint_crc64: Vec<u8>,
    pub session_id: String,
    pub session_mode: SessionMode,
    pub client_timestamp_us: i64,
    pub ingest_timestamp_us: i64,
    pub timezone_offset_min: i32,
}

impl EventMetadata {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityMode {
    Backward,
    Forward,
    Full,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaRegistryEvent {
    pub metadata: EventMetadata,
    pub schema_id: i32,
    pub subject: String,
    pub version: i32,
    pub compatibility_mode: CompatibilityMode,
    pub fingerprint_crc64_hex: String,
    pub avro_schema_json: String,
    pub registered_by: Option<Participant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifecycleEventType {
    UserRegistered,
    UserAuthenticated,
    AuthFailed,
    SessionCreated,
    ParticipantJoined,
    ParticipantLeft,
    RoleChanged,
    SessionEnded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionLifecycleEvent {
    pub metadata: EventMetadata,
    pub lifecycle_type: LifecycleEventType,
    pub actor: Participant,
    pub target_participant: Option<Participant>,
    pub password_protected: bool,
    pub roster_snapshot: Vec<Participant>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebRtcEventKind {
    SignalingOffer,
    SignalingAnswer,
    IceCandidate,
    IceStateChange,
    DataChannelOpen,
    DataChannelClosed,
    StatsSnapshot,
    TrackMuteChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IceCandidateType {
    Host,
    Srflx,
    Prflx,
    Relay,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebRtcTelemetryEvent {
    pub metadata: EventMetadata,
    pub local_participant: Participant,
    pub remote_participant: Option<Participant>,
    pub event_kind: WebRtcEventKind,
    pub ice_candidate_type: Option<IceCandidateType>,
    pub rtt_ms: Option<f32>,
    pub jitter_ms: Option<f32>,
    pub packet_loss_rate: Option<f32>,
    pub available_outgoing_bitrate_bps: Option<i64>,
    pub audio_muted: bool,
    pub video_muted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageVisibility {
    Public,
    Private,
    GmOnly,
    HostOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChannelContext {
    InCharacter,
    OutOfCharacter,
    GmNarration,
    MeetingPublic,
    MeetingBackchannel,
    SystemNotice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessageEvent {
    pub metadata: EventMetadata,
    pub message_id: String,
    pub sender: Participant,
    pub visibility: MessageVisibility,
    pub recipients: Vec<Participant>,
    pub channel_context: ChannelContext,
    pub reply_to_message_id: Option<String>,
    pub body: String,
    pub char_count: i32,
    pub word_count: i32,
    pub composition_time_ms: i64,
    pub is_edited: bool,
}

/// Evolved v2 representation of `ChatMessageEvent` demonstrating backward/forward schema evolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessageEventV2 {
    pub metadata: EventMetadata,
    pub message_id: String,
    pub sender: Participant,
    pub visibility: MessageVisibility,
    pub recipients: Vec<Participant>,
    pub channel_context: ChannelContext,
    pub reply_to_message_id: Option<String>,
    pub body: String,
    pub char_count: i32,
    pub word_count: i32,
    pub composition_time_ms: i64,
    pub is_edited: bool,
    pub sentiment_score: Option<f32>,
    pub detected_intent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InteractionTurnType {
    SpeechStart,
    SpeechEnd,
    InterruptionOverlap,
    LongSilence,
    ReactionGesture,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionDynamicsEvent {
    pub metadata: EventMetadata,
    pub speaker: Participant,
    pub turn_type: InteractionTurnType,
    pub duration_ms: i64,
    pub rms_energy_mean: f32,
    pub rms_energy_peak: f32,
    pub pitch_hz_estimate: Option<f32>,
    pub overlapped_participants: Vec<Participant>,
    pub preceding_silence_ms: i64,
    pub dominance_ratio: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MechanicEventType {
    TaskResolution,
    CombatAction,
    StatusChange,
    AiGmDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutcomeTier {
    CriticalFailure,
    Failure,
    PartialSuccess,
    Success,
    ExtraordinarySuccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CombatStance {
    Power,
    Maneuver,
    Stability,
    RangedAimed,
    RangedSnap,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtrpgMechanicsEvent {
    pub metadata: EventMetadata,
    pub mechanic_type: MechanicEventType,
    pub actor: Participant,
    pub target_participants: Vec<Participant>,
    pub skill_name: Option<String>,
    pub attribute_tn: Option<i32>,
    pub difficulty_rating: Option<i32>,
    pub dice_rolled_d20: Vec<i32>,
    pub successes_counted: i32,
    pub outcome_tier: OutcomeTier,
    pub action_time_factor: Option<f32>,
    pub action_time_cost: Option<i32>,
    pub action_count_cost: Option<i32>,
    pub combat_stance: Option<CombatStance>,
    pub fatigue_level: i32,
    pub wound_shock_index: f32,
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
// Schema Registry & Wire Framing Header
// ============================================================================

pub mod subjects {
    pub const SCHEMA_REGISTRY: &str = "com.satya.events.SchemaRegistryEvent";
    pub const SESSION_LIFECYCLE: &str = "com.satya.events.SessionLifecycleEvent";
    pub const WEBRTC_TELEMETRY: &str = "com.satya.events.WebRtcTelemetryEvent";
    pub const CHAT_MESSAGE: &str = "com.satya.events.ChatMessageEvent";
    pub const INTERACTION_DYNAMICS: &str = "com.satya.events.InteractionDynamicsEvent";
    pub const TTRPG_MECHANICS: &str = "com.satya.events.TtrpgMechanicsEvent";
}

pub mod topics {
    pub const SCHEMA_REGISTRY: &str = "schema-registry";
    pub const SESSION_LIFECYCLE: &str = "session-lifecycle";
    pub const WEBRTC_TELEMETRY: &str = "webrtc-telemetry";
    pub const CHAT_MESSAGE: &str = "chat-messages";
    pub const INTERACTION_DYNAMICS: &str = "interaction-dynamics";
    pub const TTRPG_MECHANICS: &str = "ttrpg-mechanics";
}

#[derive(Debug, Clone)]
pub struct RegisteredSchema {
    pub schema_id: u32,
    pub subject: String,
    pub topic: String,
    pub version: u16,
    pub fingerprint_crc64: [u8; 8],
    pub raw_json: String,
    pub canonical_form: String,
    pub avro_schema: AvroSchema,
}

impl RegisteredSchema {
    pub fn fingerprint_hex(&self) -> String {
        hex::encode(self.fingerprint_crc64)
    }
}

/// 15-byte binary wire framing header prepended to every Avro message in Apache Iggy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireFrameHeader {
    pub magic_byte: u8,
    pub schema_id: u32,
    pub schema_version: u16,
    pub fingerprint_crc64: [u8; 8],
}

impl WireFrameHeader {
    pub fn to_bytes(&self) -> [u8; WIRE_HEADER_LEN] {
        let mut buf = [0u8; WIRE_HEADER_LEN];
        buf[0] = self.magic_byte;
        buf[1..5].copy_from_slice(&self.schema_id.to_be_bytes());
        buf[5..7].copy_from_slice(&self.schema_version.to_be_bytes());
        buf[7..15].copy_from_slice(&self.fingerprint_crc64);
        buf
    }

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

    pub fn get_by_id(&self, schema_id: u32) -> Result<&RegisteredSchema, SchemaError> {
        self.by_id
            .get(&schema_id)
            .ok_or(SchemaError::SchemaIdNotFound(schema_id))
    }

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
}
