//! Versioned Avro [`SchemaRegistry`], 64-bit `CRC-64-AVRO` canonical fingerprinting,
//! and 15-byte binary [`WireFrameHeader`] framing.

use crate::events::{subjects, topics};
use crate::types::{EventMetadata, SessionMode};
use apache_avro::{Schema as AvroSchema, from_avro_datum, from_value, to_avro_datum, to_value};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::io::Cursor;
use thiserror::Error;

/// Magic leading byte (`0x53`, ASCII `'S'`) identifying a framed Satya wire message.
pub const WIRE_MAGIC_BYTE: u8 = 0x53; // 'S' for Satya
/// Total byte length of the binary [`WireFrameHeader`] (`1` magic + `4` schema_id + `2` version + `8` crc64).
pub const WIRE_HEADER_LEN: usize = 15; // 1 (magic) + 4 (schema_id) + 2 (version) + 8 (crc64)

/// Raw JSON schema definition (`v1`) for [`crate::events::SchemaRegistryEvent`].
pub const SCHEMA_REGISTRY_V1_AVSC: &str =
    include_str!("../../../schemas/v1/schema_registry_event.avsc");
/// Raw JSON schema definition (`v1`) for [`crate::events::SessionLifecycleEvent`].
pub const SESSION_LIFECYCLE_V1_AVSC: &str =
    include_str!("../../../schemas/v1/session_lifecycle_event.avsc");
/// Raw JSON schema definition (`v1`) for [`crate::events::WebRtcTelemetryEvent`].
pub const WEBRTC_TELEMETRY_V1_AVSC: &str =
    include_str!("../../../schemas/v1/webrtc_telemetry_event.avsc");
/// Raw JSON schema definition (`v1`) for [`crate::events::ChatMessageEvent`].
pub const CHAT_MESSAGE_V1_AVSC: &str = include_str!("../../../schemas/v1/chat_message_event.avsc");
/// Raw JSON schema definition (`v1`) for [`crate::events::InteractionDynamicsEvent`].
pub const INTERACTION_DYNAMICS_V1_AVSC: &str =
    include_str!("../../../schemas/v1/interaction_dynamics_event.avsc");
/// Raw JSON schema definition (`v1`) for [`crate::events::TtrpgMechanicsEvent`].
pub const TTRPG_MECHANICS_V1_AVSC: &str =
    include_str!("../../../schemas/v1/ttrpg_mechanics_event.avsc");
/// Evolved raw JSON schema definition (`v2`) for [`crate::events::ChatMessageEventV2`].
pub const CHAT_MESSAGE_V2_AVSC: &str = include_str!("../../../schemas/v2/chat_message_event.avsc");
/// Raw JSON schema definition (`v1`) for [`crate::events::TabletopCanvasEvent`].
pub const TABLETOP_CANVAS_V1_AVSC: &str =
    include_str!("../../../schemas/v1/tabletop_canvas_event.avsc");
/// Raw JSON schema definition (`v1`) for [`crate::events::AiGmTrajectoryEvent`].
pub const AI_GM_TRAJECTORY_V1_AVSC: &str =
    include_str!("../../../schemas/v1/ai_gm_trajectory_event.avsc");
/// Raw JSON schema definition (`v1`) for [`crate::events::SessionLogEvent`].
pub const SESSION_LOG_V1_AVSC: &str = include_str!("../../../schemas/v1/session_log_event.avsc");

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
        registry.register(
            7,
            subjects::CHAT_MESSAGE,
            topics::CHAT_MESSAGE,
            2,
            CHAT_MESSAGE_V2_AVSC,
        )?;
        registry.register(
            8,
            subjects::TABLETOP_CANVAS,
            topics::TABLETOP_CANVAS,
            1,
            TABLETOP_CANVAS_V1_AVSC,
        )?;
        registry.register(
            9,
            subjects::AI_GM_TRAJECTORY,
            topics::AI_GM_TRAJECTORY,
            1,
            AI_GM_TRAJECTORY_V1_AVSC,
        )?;
        registry.register(
            10,
            subjects::SESSION_LOG,
            topics::SYSTEM_LOGS,
            1,
            SESSION_LOG_V1_AVSC,
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

    /// Creates an [`EventMetadata`] populated with the exact version & CRC-64 fingerprint from the registry.
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

    /// Encodes a domain event into a framed wire payload (`[WireFrameHeader (15B) | Avro Binary Payload]`).
    pub fn encode_framed<T: Serialize>(
        &self,
        subject: &str,
        version: u16,
        event: &T,
    ) -> Result<Vec<u8>, SchemaError> {
        let reg = self.get_by_subject_version(subject, version)?;
        let header = WireFrameHeader {
            magic_byte: WIRE_MAGIC_BYTE,
            schema_id: reg.schema_id,
            schema_version: reg.version,
            fingerprint_crc64: reg.fingerprint_crc64,
        };

        let value = to_value(event)?;
        let avro_bytes = to_avro_datum(&reg.avro_schema, value)?;

        let mut out = Vec::with_capacity(WIRE_HEADER_LEN + avro_bytes.len());
        out.extend_from_slice(&header.to_bytes());
        out.extend_from_slice(&avro_bytes);
        Ok(out)
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
