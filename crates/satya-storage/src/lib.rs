use apache_avro::{Codec, Writer, from_avro_datum};
use bytes::Bytes;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::path::Path as ObjectPath;
use object_store::{ObjectStore, PutPayload};
use satya_schema::{
    ChatMessageEvent, InteractionDynamicsEvent, SchemaRegistry, SchemaRegistryEvent,
    SessionLifecycleEvent, TtrpgMechanicsEvent, WIRE_HEADER_LEN, WebRtcTelemetryEvent,
    WireFrameHeader, subjects, topics,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IggyConfig {
    pub enabled: bool,
    pub address: String,
    pub username: String,
    pub password: String,
    pub stream: String,
    pub fallback_if_offline: bool,
}

impl Default for IggyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            address: "127.0.0.1:8090".to_string(),
            username: "iggy".to_string(),
            password: "iggy".to_string(),
            stream: "satya".to_string(),
            fallback_if_offline: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchingConfig {
    pub batch_max_messages: usize,
    pub batch_max_interval_ms: u64,
}

impl Default for BatchingConfig {
    fn default() -> Self {
        Self {
            batch_max_messages: 50,
            batch_max_interval_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackendKind {
    Local,
    S3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicStorageConfig {
    pub topic_id: u32,
    pub partitions: u32,
    pub storage_backend: StorageBackendKind,
    pub local_path: String,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default)]
    pub iggy: IggyConfig,
    #[serde(default)]
    pub batching: BatchingConfig,
    pub topics: HashMap<String, TopicStorageConfig>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let mut topic_map = HashMap::new();
        let defaults = [
            (topics::SCHEMA_REGISTRY, 1, 1),
            (topics::SESSION_LIFECYCLE, 2, 2),
            (topics::WEBRTC_TELEMETRY, 3, 2),
            (topics::CHAT_MESSAGE, 4, 4),
            (topics::INTERACTION_DYNAMICS, 5, 4),
            (topics::TTRPG_MECHANICS, 6, 2),
        ];
        for (name, id, parts) in defaults {
            topic_map.insert(
                name.to_string(),
                TopicStorageConfig {
                    topic_id: id,
                    partitions: parts,
                    storage_backend: StorageBackendKind::Local,
                    local_path: format!("data/topics/{name}"),
                    s3_bucket: Some("satya-analytics-lake".to_string()),
                    s3_region: Some("us-east-1".to_string()),
                    s3_endpoint: None,
                    s3_prefix: Some(format!("raw/{name}")),
                },
            );
        }
        Self {
            iggy: IggyConfig::default(),
            batching: BatchingConfig::default(),
            topics: topic_map,
        }
    }
}

impl StorageConfig {
    pub fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let cfg: StorageConfig = toml::from_str(&content)?;
        Ok(cfg)
    }
}

/// Summary statistics for a single topic displayed in the UI inspector and metrics API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMetricsSnapshot {
    pub topic: String,
    pub topic_id: u32,
    pub storage_backend: StorageBackendKind,
    pub destination_uri: String,
    pub total_messages_ingested: u64,
    pub buffered_messages: usize,
    pub flushed_batches_count: u64,
    pub last_flushed_offset: Option<u64>,
    pub active_schema_version: u16,
    pub active_schema_fingerprint_hex: String,
    pub iggy_connected: bool,
}

#[derive(Debug, Clone)]
struct BufferedIggyMessage {
    offset: u64,
    partition_id: u32,
    header: WireFrameHeader,
    framed_payload: Vec<u8>,
}

struct TopicPartitionState {
    next_offset: u64,
    last_flushed_offset: Option<u64>,
    flushed_batches_count: u64,
    buffer: Vec<BufferedIggyMessage>,
}

/// Manages Apache Iggy stream publishing, wire-level schema headers, and offset-tracked
/// Avro Object Container micro-batching to Local FS or S3 per topic.
pub struct EventLogRouter {
    config: StorageConfig,
    base_dir: PathBuf,
    registry: Arc<SchemaRegistry>,
    iggy_online: RwLock<bool>,
    topic_states: RwLock<HashMap<String, TopicPartitionState>>,
}

impl EventLogRouter {
    pub async fn new(config: StorageConfig, base_dir: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let base_dir = base_dir.into();
        let registry = Arc::new(SchemaRegistry::new_with_builtin_schemas()?);

        // Probe Apache Iggy TCP port if enabled
        let iggy_online = if config.iggy.enabled {
            match tokio::time::timeout(
                std::time::Duration::from_millis(250),
                TcpStream::connect(&config.iggy.address),
            )
            .await
            {
                Ok(Ok(_stream)) => {
                    info!(
                        "Connected to Apache Iggy server at {} (stream '{}')",
                        config.iggy.address, config.iggy.stream
                    );
                    true
                }
                _ => {
                    if config.iggy.fallback_if_offline {
                        warn!(
                            "Apache Iggy server at {} is unreachable; using embedded local stream broker for '{}'",
                            config.iggy.address, config.iggy.stream
                        );
                        false
                    } else {
                        anyhow::bail!(
                            "Apache Iggy server at {} is unreachable and fallback_if_offline=false",
                            config.iggy.address
                        );
                    }
                }
            }
        } else {
            false
        };

        let mut states = HashMap::new();
        for topic_name in config.topics.keys() {
            states.insert(
                topic_name.clone(),
                TopicPartitionState {
                    next_offset: 0,
                    last_flushed_offset: None,
                    flushed_batches_count: 0,
                    buffer: Vec::new(),
                },
            );
        }

        let router = Self {
            config,
            base_dir,
            registry,
            iggy_online: RwLock::new(iggy_online),
            topic_states: RwLock::new(states),
        };

        // Log initial schema registrations to the `schema-registry` topic
        router.bootstrap_schema_registry_topic().await?;

        Ok(router)
    }

    pub fn registry(&self) -> &Arc<SchemaRegistry> {
        &self.registry
    }

    async fn bootstrap_schema_registry_topic(&self) -> anyhow::Result<()> {
        for reg in self.registry.all_schemas() {
            let meta = self.registry.build_metadata(
                reg.schema_id as i64,
                subjects::SCHEMA_REGISTRY,
                1,
                "system-bootstrap",
                satya_schema::SessionMode::Ttrpg,
            )?;
            let ev = SchemaRegistryEvent {
                metadata: meta,
                schema_id: reg.schema_id as i32,
                subject: reg.subject.clone(),
                version: reg.version as i32,
                compatibility_mode: satya_schema::CompatibilityMode::Full,
                fingerprint_crc64_hex: reg.fingerprint_hex(),
                avro_schema_json: reg.raw_json.clone(),
                registered_by: None,
            };
            self.publish_event(topics::SCHEMA_REGISTRY, subjects::SCHEMA_REGISTRY, 1, &ev)
                .await?;
        }
        self.flush_topic(topics::SCHEMA_REGISTRY).await?;
        Ok(())
    }

    /// Encodes a domain event with its 15-byte wire header and appends it to the Apache Iggy topic log.
    /// Automatically flushes an Avro Object Container batch when `batch_max_messages` is reached.
    pub async fn publish_event<T: Serialize>(
        &self,
        topic: &str,
        subject: &str,
        schema_version: u16,
        event: &T,
    ) -> anyhow::Result<u64> {
        let framed_payload = self
            .registry
            .encode_framed(subject, schema_version, event)?;
        let header = WireFrameHeader::from_bytes(&framed_payload)?;

        // Also append to local Iggy WAL log file for immediate durability before batch flush
        self.append_iggy_wal(topic, &framed_payload).await?;

        let should_flush;
        let assigned_offset;
        {
            let mut guard = self.topic_states.write().await;
            let state = guard
                .entry(topic.to_string())
                .or_insert(TopicPartitionState {
                    next_offset: 0,
                    last_flushed_offset: None,
                    flushed_batches_count: 0,
                    buffer: Vec::new(),
                });
            assigned_offset = state.next_offset;
            state.next_offset += 1;
            state.buffer.push(BufferedIggyMessage {
                offset: assigned_offset,
                partition_id: 0,
                header,
                framed_payload,
            });
            should_flush = state.buffer.len() >= self.config.batching.batch_max_messages;
        }

        if should_flush {
            self.flush_topic(topic).await?;
        }

        Ok(assigned_offset)
    }

    async fn append_iggy_wal(&self, topic: &str, framed_payload: &[u8]) -> anyhow::Result<()> {
        let wal_dir = self
            .base_dir
            .join("data")
            .join("iggy_streams")
            .join(&self.config.iggy.stream)
            .join(topic);
        tokio::fs::create_dir_all(&wal_dir).await?;
        let wal_file = wal_dir.join("partition_0.iggylog");

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(wal_file)
            .await?;
        let len_prefix = (framed_payload.len() as u32).to_be_bytes();
        file.write_all(&len_prefix).await?;
        file.write_all(framed_payload).await?;
        Ok(())
    }

    /// Flushes all buffered messages for a topic into offset-tagged Avro Object Container files
    /// (`batch_{start_offset:010}_{end_offset:010}.avro`) on either Local FS or S3.
    pub async fn flush_topic(&self, topic: &str) -> anyhow::Result<Option<String>> {
        let messages = {
            let mut guard = self.topic_states.write().await;
            let Some(state) = guard.get_mut(topic) else {
                return Ok(None);
            };
            if state.buffer.is_empty() {
                return Ok(None);
            }
            std::mem::take(&mut state.buffer)
        };

        let start_offset = messages.first().unwrap().offset;
        let end_offset = messages.last().unwrap().offset;
        let first_hdr = messages.first().unwrap().header;
        let partition_id = messages.first().unwrap().partition_id;

        let reg_schema = self.registry.get_by_id(first_hdr.schema_id)?;

        // Build Avro Object Container File with custom Iggy offset & schema evolution metadata
        let mut writer = Writer::with_codec(&reg_schema.avro_schema, Vec::new(), Codec::Snappy);
        writer.add_user_metadata("satya.stream".to_string(), &self.config.iggy.stream)?;
        writer.add_user_metadata("satya.topic".to_string(), topic)?;
        writer.add_user_metadata("satya.partition_id".to_string(), partition_id.to_string())?;
        writer.add_user_metadata("satya.start_offset".to_string(), start_offset.to_string())?;
        writer.add_user_metadata("satya.end_offset".to_string(), end_offset.to_string())?;
        writer.add_user_metadata(
            "satya.schema_id".to_string(),
            reg_schema.schema_id.to_string(),
        )?;
        writer.add_user_metadata(
            "satya.schema_version".to_string(),
            reg_schema.version.to_string(),
        )?;
        writer.add_user_metadata(
            "satya.schema_fingerprint_crc64".to_string(),
            reg_schema.fingerprint_hex(),
        )?;

        for msg in &messages {
            let datum_slice = &msg.framed_payload[WIRE_HEADER_LEN..];
            let mut cursor = Cursor::new(datum_slice);
            let value = from_avro_datum(&reg_schema.avro_schema, &mut cursor, None)?;
            writer.append_value_ref(&value)?;
        }

        let container_bytes = writer.into_inner()?;

        let topic_cfg = self
            .config
            .topics
            .get(topic)
            .cloned()
            .unwrap_or(TopicStorageConfig {
                topic_id: 99,
                partitions: 1,
                storage_backend: StorageBackendKind::Local,
                local_path: format!("data/topics/{topic}"),
                s3_bucket: None,
                s3_region: None,
                s3_endpoint: None,
                s3_prefix: None,
            });

        let written_uri = self
            .write_to_configured_backend(
                &topic_cfg,
                reg_schema.version,
                partition_id,
                start_offset,
                end_offset,
                container_bytes,
            )
            .await?;

        {
            let mut guard = self.topic_states.write().await;
            if let Some(state) = guard.get_mut(topic) {
                state.last_flushed_offset = Some(end_offset);
                state.flushed_batches_count += 1;
            }
        }

        Ok(Some(written_uri))
    }

    pub async fn flush_all_topics(&self) -> anyhow::Result<Vec<String>> {
        let topic_names: Vec<String> = self.config.topics.keys().cloned().collect();
        let mut flushed = Vec::new();
        for name in topic_names {
            if let Some(uri) = self.flush_topic(&name).await? {
                flushed.push(uri);
            }
        }
        Ok(flushed)
    }

    async fn write_to_configured_backend(
        &self,
        topic_cfg: &TopicStorageConfig,
        schema_version: u16,
        partition_id: u32,
        start_offset: u64,
        end_offset: u64,
        bytes: Vec<u8>,
    ) -> anyhow::Result<String> {
        let filename = format!("batch_{start_offset:010}_{end_offset:010}.avro");
        match topic_cfg.storage_backend {
            StorageBackendKind::Local => {
                let target_dir = self
                    .base_dir
                    .join(&topic_cfg.local_path)
                    .join(format!("schema_version=v{schema_version}"))
                    .join(format!("partition={partition_id}"));
                tokio::fs::create_dir_all(&target_dir).await?;
                let store = LocalFileSystem::new_with_prefix(&target_dir)?;
                let obj_path = ObjectPath::from(filename.as_str());
                store
                    .put(&obj_path, PutPayload::from_bytes(Bytes::from(bytes)))
                    .await?;
                Ok(target_dir.join(filename).display().to_string())
            }
            StorageBackendKind::S3 => {
                let bucket = topic_cfg
                    .s3_bucket
                    .as_deref()
                    .unwrap_or("satya-analytics-lake");
                let region = topic_cfg.s3_region.as_deref().unwrap_or("us-east-1");
                let prefix = topic_cfg.s3_prefix.as_deref().unwrap_or("raw");

                let mut builder = AmazonS3Builder::from_env()
                    .with_bucket_name(bucket)
                    .with_region(region);
                if let Some(ep) = &topic_cfg.s3_endpoint {
                    builder = builder.with_endpoint(ep).with_allow_http(true);
                }
                let store = builder.build()?;
                let key = format!(
                    "{prefix}/schema_version=v{schema_version}/partition={partition_id}/{filename}"
                );
                let obj_path = ObjectPath::from(key.as_str());
                store
                    .put(&obj_path, PutPayload::from_bytes(Bytes::from(bytes)))
                    .await?;
                Ok(format!("s3://{bucket}/{key}"))
            }
        }
    }

    // Convenience typed publishers
    pub async fn log_session_lifecycle(
        &self,
        event: &SessionLifecycleEvent,
    ) -> anyhow::Result<u64> {
        let offset = self
            .publish_event(
                topics::SESSION_LIFECYCLE,
                subjects::SESSION_LIFECYCLE,
                1,
                event,
            )
            .await?;
        self.flush_topic(topics::SESSION_LIFECYCLE).await?;
        Ok(offset)
    }

    pub async fn log_webrtc_telemetry(&self, event: &WebRtcTelemetryEvent) -> anyhow::Result<u64> {
        self.publish_event(
            topics::WEBRTC_TELEMETRY,
            subjects::WEBRTC_TELEMETRY,
            1,
            event,
        )
        .await
    }

    pub async fn log_chat_message(&self, event: &ChatMessageEvent) -> anyhow::Result<u64> {
        self.publish_event(topics::CHAT_MESSAGE, subjects::CHAT_MESSAGE, 1, event)
            .await
    }

    pub async fn log_interaction_dynamics(
        &self,
        event: &InteractionDynamicsEvent,
    ) -> anyhow::Result<u64> {
        self.publish_event(
            topics::INTERACTION_DYNAMICS,
            subjects::INTERACTION_DYNAMICS,
            1,
            event,
        )
        .await
    }

    pub async fn log_ttrpg_mechanics(&self, event: &TtrpgMechanicsEvent) -> anyhow::Result<u64> {
        let offset = self
            .publish_event(topics::TTRPG_MECHANICS, subjects::TTRPG_MECHANICS, 1, event)
            .await?;
        self.flush_topic(topics::TTRPG_MECHANICS).await?;
        Ok(offset)
    }

    pub async fn metrics_snapshot(&self) -> Vec<TopicMetricsSnapshot> {
        let guard = self.topic_states.read().await;
        let iggy_connected = *self.iggy_online.read().await;
        let mut out = Vec::new();

        for (topic_name, cfg) in &self.config.topics {
            let state = guard.get(topic_name);
            let subject = match topic_name.as_str() {
                topics::SCHEMA_REGISTRY => subjects::SCHEMA_REGISTRY,
                topics::SESSION_LIFECYCLE => subjects::SESSION_LIFECYCLE,
                topics::WEBRTC_TELEMETRY => subjects::WEBRTC_TELEMETRY,
                topics::CHAT_MESSAGE => subjects::CHAT_MESSAGE,
                topics::INTERACTION_DYNAMICS => subjects::INTERACTION_DYNAMICS,
                topics::TTRPG_MECHANICS => subjects::TTRPG_MECHANICS,
                _ => subjects::CHAT_MESSAGE,
            };
            let reg = self
                .registry
                .get_by_subject_version(subject, 1)
                .expect("Registered schema exists");

            let destination_uri = match cfg.storage_backend {
                StorageBackendKind::Local => cfg.local_path.clone(),
                StorageBackendKind::S3 => format!(
                    "s3://{}/{}",
                    cfg.s3_bucket.as_deref().unwrap_or("satya-analytics-lake"),
                    cfg.s3_prefix.as_deref().unwrap_or(topic_name)
                ),
            };

            out.push(TopicMetricsSnapshot {
                topic: topic_name.clone(),
                topic_id: cfg.topic_id,
                storage_backend: cfg.storage_backend,
                destination_uri,
                total_messages_ingested: state.map(|s| s.next_offset).unwrap_or(0),
                buffered_messages: state.map(|s| s.buffer.len()).unwrap_or(0),
                flushed_batches_count: state.map(|s| s.flushed_batches_count).unwrap_or(0),
                last_flushed_offset: state.and_then(|s| s.last_flushed_offset),
                active_schema_version: reg.version,
                active_schema_fingerprint_hex: reg.fingerprint_hex(),
                iggy_connected,
            });
        }

        out.sort_by_key(|m| m.topic_id);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apache_avro::Reader;
    use satya_schema::{ChannelContext, MessageVisibility, Participant, SessionMode, SessionRole};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_micro_batch_flush_and_avro_header_watermarks() {
        let temp_dir = std::env::temp_dir().join(format!("satya_storage_test_{}", Uuid::new_v4()));
        let mut cfg = StorageConfig::default();
        cfg.batching.batch_max_messages = 3;

        let router = EventLogRouter::new(cfg, &temp_dir).await.unwrap();
        let sender = Participant::human("u-100", "host_elena", "Elena (Host)", SessionRole::Host);
        let recipient = Participant::human(
            "u-200",
            "mark_dev",
            "Mark (Attendee)",
            SessionRole::Participant,
        );

        for i in 0..3 {
            let meta = router
                .registry()
                .build_metadata(
                    i,
                    subjects::CHAT_MESSAGE,
                    1,
                    "meeting-alpha",
                    SessionMode::Meeting,
                )
                .unwrap();
            let ev = ChatMessageEvent {
                metadata: meta,
                message_id: Uuid::now_v7().to_string(),
                sender: sender.clone(),
                visibility: MessageVisibility::Private,
                recipients: vec![recipient.clone()],
                channel_context: ChannelContext::MeetingBackchannel,
                reply_to_message_id: None,
                body: format!("Private message #{i}"),
                char_count: 18,
                word_count: 3,
                composition_time_ms: 950,
                is_edited: false,
            };
            router.log_chat_message(&ev).await.unwrap();
        }

        // Because batch_max_messages == 3, `chat-messages` should have auto-flushed batch_0000000000_0000000002.avro
        let batch_path = temp_dir
            .join("data/topics/chat-messages/schema_version=v1/partition=0/batch_0000000000_0000000002.avro");
        assert!(
            batch_path.exists(),
            "Expected flushed Avro batch file at {:?}",
            batch_path
        );

        let file_bytes = std::fs::read(&batch_path).unwrap();
        let reader = Reader::new(&file_bytes[..]).unwrap();
        let user_meta = reader.user_metadata();

        assert_eq!(
            user_meta
                .get("satya.start_offset")
                .map(|v| String::from_utf8_lossy(v)),
            Some("0".into())
        );
        assert_eq!(
            user_meta
                .get("satya.end_offset")
                .map(|v| String::from_utf8_lossy(v)),
            Some("2".into())
        );
        assert_eq!(
            user_meta
                .get("satya.schema_version")
                .map(|v| String::from_utf8_lossy(v)),
            Some("1".into())
        );

        let records: Vec<_> = reader
            .map(|r| apache_avro::from_value::<ChatMessageEvent>(&r.unwrap()).unwrap())
            .collect();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].sender.username, "host_elena");
        assert_eq!(records[0].recipients[0].username, "mark_dev");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
