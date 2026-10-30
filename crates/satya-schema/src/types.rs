//! Core nested Avro types (`com.satya.types`) shared across all Satya event schemas and wire protocols.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
