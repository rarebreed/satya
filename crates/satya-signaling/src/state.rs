//! Shared application state (`SignalingAppState`), message target routing, WebSocket fanout,
//! and session disconnect lifecycle processing for the Satya signaling server.

use crate::auth::{AuthAndSessionManager, SessionDisconnectSummary};
use crate::email_notifier::{EmailNotifier, build_email_notifier};
use crate::player_ai_api::SessionAiPlayerEntry;
use crate::rate_limit::RateLimiter;
use satya_schema::{
    AccountWsPacket, LifecycleEventType, MessageVisibility, Participant, ServerWsPacket,
    SessionLifecycleEvent, SessionMode, SessionRole, WebRtcEventKind, WebRtcTelemetryEvent,
    subjects,
};
use satya_storage::{EventLogRouter, StorageConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicI64, AtomicU64, Ordering},
};
use tokio::sync::{RwLock, mpsc};

/// Hard ceiling on consecutive autonomous AI-to-AI rally rounds before pausing for human input.
pub const MAX_AUTONOMOUS_RALLY_ROUNDS: u32 = 10;

/// Per-session autonomous AI Player rally scheduler state.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SessionRallyState {
    /// Complete AI GM narration rounds dispatched so far since last human turn or reset.
    pub rounds_used: u32,
    /// Maximum rounds allowed before pausing (`1..=MAX_AUTONOMOUS_RALLY_ROUNDS`).
    pub max_rounds: u32,
    /// Whether autonomous rally progression is paused by an Observer/GM/Host.
    pub paused: bool,
}

impl Default for SessionRallyState {
    fn default() -> Self {
        Self {
            rounds_used: 0,
            max_rounds: 3,
            paused: false,
        }
    }
}

/// Monotonic connection token paired with a WebSocket outbound channel (`(conn_id, tx)`).
pub type SessionSocketEntry = (u64, mpsc::UnboundedSender<String>);

/// Active WebSocket map keyed by `session_id -> (user_id -> (conn_id, tx))`.
pub type SessionSocketMap = HashMap<String, HashMap<String, SessionSocketEntry>>;

/// Shared application state for the Satya HTTP and WebSocket signaling server.
pub struct SignalingAppState {
    /// Persistent Argon2id user authentication and password-protected session manager.
    pub auth: AuthAndSessionManager,
    /// Multi-topic Apache Iggy and Avro container batch storage router.
    pub storage: Arc<EventLogRouter>,
    /// Active default campaign pack configured via `config/ai.toml` / `config/campaign.toml`.
    pub campaign_pack: String,
    /// Monotonically increasing sequence counter for emitted Avro event envelopes.
    pub sequence_counter: AtomicI64,
    /// Monotonically increasing WebSocket connection identifier to prevent reload disconnect races.
    pub next_conn_id: AtomicU64,
    /// Active WebSocket connections per `session_id -> (user_id -> (conn_id, tx))`.
    pub active_sockets: RwLock<SessionSocketMap>,
    /// Account-level notification WebSocket connections keyed by `user_id -> (conn_id -> tx)`.
    pub account_sockets: RwLock<HashMap<String, HashMap<u64, mpsc::UnboundedSender<String>>>>,
    /// Sliding-window anti-bot rate limiter.
    pub rate_limiter: RateLimiter,
    /// Outbound email notifier for offline GM join-request alerts.
    pub email_notifier: Arc<dyn EmailNotifier>,
    /// Configured AI Player agents per `session_id -> (player_user_id -> SessionAiPlayerEntry)`.
    pub ai_players: RwLock<HashMap<String, HashMap<String, SessionAiPlayerEntry>>>,
    /// Per-session autonomous AI rally scheduler state (`session_id -> SessionRallyState`).
    pub ai_rally_counters: RwLock<HashMap<String, SessionRallyState>>,
}

impl SignalingAppState {
    /// Initializes a new [`SignalingAppState`] with persistent user accounts and topic storage sinks.
    pub async fn new(
        config: StorageConfig,
        base_dir: impl Into<PathBuf>,
    ) -> anyhow::Result<Arc<Self>> {
        let base_dir = base_dir.into();
        let campaign_pack = config.ai_gm.campaign_pack.clone();
        let auth =
            AuthAndSessionManager::new_with_quotas(&base_dir, &config.auth, config.quotas.clone())
                .await?;
        let start_seq = auth.max_persisted_sequence_num().await.saturating_add(1);
        let email_notifier = build_email_notifier(&config.email, auth.db_pool().clone());
        let storage = Arc::new(EventLogRouter::new(config, &base_dir).await?);

        Ok(Arc::new(Self {
            auth,
            storage,
            campaign_pack,
            sequence_counter: AtomicI64::new(start_seq),
            next_conn_id: AtomicU64::new(1),
            active_sockets: RwLock::new(HashMap::new()),
            account_sockets: RwLock::new(HashMap::new()),
            rate_limiter: RateLimiter::new(),
            email_notifier,
            ai_players: RwLock::new(HashMap::new()),
            ai_rally_counters: RwLock::new(HashMap::new()),
        }))
    }

    /// Returns `true` if `user_id` currently has an open `/ws/notifications` or `/ws/session` socket.
    pub async fn is_user_online(&self, user_id: &str) -> bool {
        let has_acct_ws = self
            .account_sockets
            .read()
            .await
            .get(user_id)
            .is_some_and(|m| !m.is_empty());
        if has_acct_ws {
            return true;
        }
        let guard = self.active_sockets.read().await;
        guard.values().any(|room| room.contains_key(user_id))
    }

    /// Pushes an [`AccountWsPacket`] to every active `/ws/notifications` socket belonging to `user_id`.
    pub async fn send_account_notification(&self, user_id: &str, pkt: &AccountWsPacket) {
        let Ok(payload) = serde_json::to_string(pkt) else {
            return;
        };
        let guard = self.account_sockets.read().await;
        if let Some(conns) = guard.get(user_id) {
            for tx in conns.values() {
                let _ = tx.send(payload.clone());
            }
        }
    }

    /// Drops all active `/ws/notifications` outbound channels for `user_id` (closing those sockets).
    pub async fn disconnect_account_sockets(&self, user_id: &str) {
        self.account_sockets.write().await.remove(user_id);
    }

    /// Pushes a [`ServerWsPacket`] to `owner_user_id` across any active `/ws/session` room they are in.
    pub async fn send_user_live_session_packet(&self, user_id: &str, pkt: &ServerWsPacket) {
        let Ok(payload) = serde_json::to_string(pkt) else {
            return;
        };
        let guard = self.active_sockets.read().await;
        for room_map in guard.values() {
            if let Some((_, tx)) = room_map.get(user_id) {
                let _ = tx.send(payload.clone());
            }
        }
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
                if let Some((_, tx)) = room_map.get(uid) {
                    let _ = tx.send(payload_json.to_string());
                }
            }
        }
    }

    /// Broadcasts a serialized JSON WebSocket packet to every connected socket in `session_id`.
    pub async fn broadcast_to_session(&self, session_id: &str, payload_json: &str) {
        let guard = self.active_sockets.read().await;
        if let Some(room_map) = guard.get(session_id) {
            for (_, tx) in room_map.values() {
                let _ = tx.send(payload_json.to_string());
            }
        }
    }

    /// Broadcasts an out-of-band `SYSTEM_NOTICE` chat packet to all connected human participants
    /// (including `Observer`s) in `session_id` without failing silently when AI Agents have no socket.
    pub async fn broadcast_system_notice_to_humans(
        &self,
        session_id: &str,
        session_mode: SessionMode,
        sender: &Participant,
        roster: &[Participant],
        notice_body: &str,
    ) {
        let human_ids: Vec<String> = roster
            .iter()
            .filter(|p| p.participant_kind == satya_schema::ParticipantKind::Human)
            .map(|p| p.user_id.clone())
            .collect();
        if human_ids.is_empty() {
            return;
        }
        let Ok(meta) = self.storage.registry().build_metadata(
            self.next_seq(),
            subjects::CHAT_MESSAGE,
            1,
            session_id,
            session_mode,
        ) else {
            return;
        };
        let ev = satya_schema::ChatMessageEvent {
            metadata: meta,
            message_id: uuid::Uuid::now_v7().to_string(),
            sender: sender.clone(),
            visibility: MessageVisibility::Public,
            recipients: roster
                .iter()
                .filter(|p| p.participant_kind == satya_schema::ParticipantKind::Human)
                .cloned()
                .collect(),
            channel_context: satya_schema::ChannelContext::SystemNotice,
            reply_to_message_id: None,
            body: notice_body.to_string(),
            char_count: notice_body.chars().count() as i32,
            word_count: notice_body.split_whitespace().count() as i32,
            composition_time_ms: 0,
            is_edited: false,
        };
        let _ = self.storage.log_chat_message(&ev).await;
        self.auth.record_chat_event_to_db(session_id, &ev).await;
        let pkt_str = serde_json::json!({
            "type": "CHAT_EVENT",
            "event": ev
        })
        .to_string();
        self.send_to_users(session_id, &human_ids, &pkt_str).await;
    }

    /// Removes the socket for `(session_id, user_id)` ONLY if its active `conn_id` matches `expected_conn_id`.
    /// Returns `true` if this socket was still the active connection (or `false` if superseded by a newer tab/reconnect).
    pub async fn unregister_ws_socket(
        &self,
        session_id: &str,
        user_id: &str,
        expected_conn_id: u64,
    ) -> bool {
        let mut guard = self.active_sockets.write().await;
        let Some(room_map) = guard.get_mut(session_id) else {
            return true;
        };
        match room_map.get(user_id) {
            Some((current_conn_id, _)) if *current_conn_id != expected_conn_id => false,
            Some(_) => {
                room_map.remove(user_id);
                true
            }
            None => true,
        }
    }

    /// Returns `true` if `(session_id, user_id)` currently has an open WebSocket connection.
    pub async fn has_active_ws_socket(&self, session_id: &str, user_id: &str) -> bool {
        let guard = self.active_sockets.read().await;
        guard
            .get(session_id)
            .is_some_and(|room_map| room_map.contains_key(user_id))
    }

    /// Cleans up active WebSocket state, logs `WebRtcTelemetryEvent` (`PEER_DISCONNECTED`) and
    /// `SessionLifecycleEvent` (`PARTICIPANT_LEFT` / `PARTICIPANT_DISCONNECTED`, plus `SESSION_ENDED`
    /// if no human or active AI Player participants remain), and broadcasts `PEER_DISCONNECTED` + `ROSTER_SYNC` to peers.
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

        // 3. If no human participants or active autonomous AI Players remain, also log SESSION_ENDED
        if summary.remaining_human_count == 0
            && !summary.has_active_ai_players
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
