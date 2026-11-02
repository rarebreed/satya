//! Shared application state (`SignalingAppState`), message target routing, WebSocket fanout,
//! and session disconnect lifecycle processing for the Satya signaling server.

use crate::auth::{AuthAndSessionManager, SessionDisconnectSummary};
use satya_schema::{
    LifecycleEventType, MessageVisibility, Participant, ServerWsPacket, SessionLifecycleEvent,
    SessionRole, WebRtcEventKind, WebRtcTelemetryEvent, subjects,
};
use satya_storage::{EventLogRouter, StorageConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};
use tokio::sync::{RwLock, mpsc};

/// Shared application state for the Satya HTTP and WebSocket signaling server.
pub struct SignalingAppState {
    /// Persistent Argon2id user authentication and password-protected session manager.
    pub auth: AuthAndSessionManager,
    /// Multi-topic Apache Iggy and Avro container batch storage router.
    pub storage: Arc<EventLogRouter>,
    /// Monotonically increasing sequence counter for emitted Avro event envelopes.
    pub sequence_counter: AtomicI64,
    /// Active WebSocket connections per `session_id -> (user_id -> tx)`.
    pub active_sockets: RwLock<HashMap<String, HashMap<String, mpsc::UnboundedSender<String>>>>,
}

impl SignalingAppState {
    /// Initializes a new [`SignalingAppState`] with persistent user accounts and topic storage sinks.
    pub async fn new(
        config: StorageConfig,
        base_dir: impl Into<PathBuf>,
    ) -> anyhow::Result<Arc<Self>> {
        let base_dir = base_dir.into();
        let auth = AuthAndSessionManager::new(&base_dir, &config.auth).await?;
        let storage = Arc::new(EventLogRouter::new(config, &base_dir).await?);

        Ok(Arc::new(Self {
            auth,
            storage,
            sequence_counter: AtomicI64::new(1),
            active_sockets: RwLock::new(HashMap::new()),
        }))
    }

    /// Returns the next monotonically increasing sequence number for event metadata.
    pub fn next_seq(&self) -> i64 {
        self.sequence_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Resolves which `Participant` records are stored in `ChatMessageEvent.recipients`
    /// and which `user_id`s should receive the live WebSocket frame.
    pub fn resolve_message_targets(
        sender: &Participant,
        visibility: MessageVisibility,
        requested_recipient_ids: &[String],
        roster: &[Participant],
    ) -> (Vec<Participant>, Vec<String>) {
        let mut recipient_records = Vec::new();
        let mut delivery_user_ids = vec![sender.user_id.clone()];

        match visibility {
            MessageVisibility::Public => {
                for p in roster {
                    recipient_records.push(p.clone());
                    if !delivery_user_ids.contains(&p.user_id) {
                        delivery_user_ids.push(p.user_id.clone());
                    }
                }
            }
            MessageVisibility::Private => {
                for p in roster {
                    if requested_recipient_ids.contains(&p.user_id) {
                        recipient_records.push(p.clone());
                        if !delivery_user_ids.contains(&p.user_id) {
                            delivery_user_ids.push(p.user_id.clone());
                        }
                    }
                }
            }
            MessageVisibility::GmOnly => {
                for p in roster {
                    if matches!(p.role, SessionRole::Gm | SessionRole::AiGm) {
                        recipient_records.push(p.clone());
                        if !delivery_user_ids.contains(&p.user_id) {
                            delivery_user_ids.push(p.user_id.clone());
                        }
                    }
                }
            }
            MessageVisibility::HostOnly => {
                for p in roster {
                    if p.role == SessionRole::Host {
                        recipient_records.push(p.clone());
                        if !delivery_user_ids.contains(&p.user_id) {
                            delivery_user_ids.push(p.user_id.clone());
                        }
                    }
                }
            }
        }

        (recipient_records, delivery_user_ids)
    }

    /// Sends a serialized JSON WebSocket packet to a targeted subset of `user_id`s in `session_id`.
    pub async fn send_to_users(
        &self,
        session_id: &str,
        target_user_ids: &[String],
        payload_json: &str,
    ) {
        let guard = self.active_sockets.read().await;
        if let Some(room_map) = guard.get(session_id) {
            for uid in target_user_ids {
                if let Some(tx) = room_map.get(uid) {
                    let _ = tx.send(payload_json.to_string());
                }
            }
        }
    }

    /// Broadcasts a serialized JSON WebSocket packet to every connected socket in `session_id`.
    pub async fn broadcast_to_session(&self, session_id: &str, payload_json: &str) {
        let guard = self.active_sockets.read().await;
        if let Some(room_map) = guard.get(session_id) {
            for tx in room_map.values() {
                let _ = tx.send(payload_json.to_string());
            }
        }
    }

    /// Cleans up active WebSocket state, logs `WebRtcTelemetryEvent` (`PEER_DISCONNECTED`) and
    /// `SessionLifecycleEvent` (`PARTICIPANT_LEFT` / `PARTICIPANT_DISCONNECTED`, plus `SESSION_ENDED`
    /// if no human participants remain), and broadcasts `PEER_DISCONNECTED` + `ROSTER_SYNC` to peers.
    pub async fn process_session_disconnect(
        &self,
        summary: &SessionDisconnectSummary,
        lifecycle_type: LifecycleEventType,
        reason: &str,
    ) {
        {
            let mut guard = self.active_sockets.write().await;
            if let Some(room_map) = guard.get_mut(&summary.session_id) {
                room_map.remove(&summary.departed_participant.user_id);
            }
        }

        // 1. Log WebRTC peer disconnect telemetry event
        if let Ok(meta) = self.storage.registry().build_metadata(
            self.next_seq(),
            subjects::WEBRTC_TELEMETRY,
            1,
            &summary.session_id,
            summary.session_mode,
        ) {
            let tel_ev = WebRtcTelemetryEvent {
                metadata: meta,
                local_participant: summary.departed_participant.clone(),
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
            let _ = self.storage.log_webrtc_telemetry(&tel_ev).await;
        }

        // 2. Log SessionLifecycleEvent (PARTICIPANT_LEFT or PARTICIPANT_DISCONNECTED)
        if let Ok(meta) = self.storage.registry().build_metadata(
            self.next_seq(),
            subjects::SESSION_LIFECYCLE,
            1,
            &summary.session_id,
            summary.session_mode,
        ) {
            let ev = SessionLifecycleEvent {
                metadata: meta,
                lifecycle_type,
                actor: summary.departed_participant.clone(),
                target_participant: None,
                password_protected: true,
                roster_snapshot: summary.remaining_roster.clone(),
                reason: Some(reason.to_string()),
            };
            let _ = self.storage.log_session_lifecycle(&ev).await;
        }

        // 3. If no human participants remain in the session, also log SESSION_ENDED
        if summary.remaining_human_count == 0
            && let Ok(meta) = self.storage.registry().build_metadata(
                self.next_seq(),
                subjects::SESSION_LIFECYCLE,
                1,
                &summary.session_id,
                summary.session_mode,
            )
        {
            let ended_ev = SessionLifecycleEvent {
                metadata: meta,
                lifecycle_type: LifecycleEventType::SessionEnded,
                actor: summary.departed_participant.clone(),
                target_participant: None,
                password_protected: true,
                roster_snapshot: summary.remaining_roster.clone(),
                reason: Some("Last human participant disconnected from session".to_string()),
            };
            let _ = self.storage.log_session_lifecycle(&ended_ev).await;
        }

        // 4. Broadcast PEER_DISCONNECTED and updated ROSTER_SYNC to remaining connected peers
        if let Ok(peer_disc_msg) = serde_json::to_string(&ServerWsPacket::PeerDisconnected {
            participant: summary.departed_participant.clone(),
            reason: Some(reason.to_string()),
        }) {
            self.broadcast_to_session(&summary.session_id, &peer_disc_msg)
                .await;
        }

        if let Ok(sync_msg) = serde_json::to_string(&ServerWsPacket::RosterSync {
            roster: summary.remaining_roster.clone(),
        }) {
            self.broadcast_to_session(&summary.session_id, &sync_msg)
                .await;
        }
    }
}
