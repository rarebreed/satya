pub mod auth;

use crate::auth::{AuthAndSessionManager, JoinTokenClaims};
use axum::{
    Json, Router,
    extract::{
        Query, State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use rand::Rng;
use satya_schema::{
    ChannelContext, ChatMessageEvent, CombatStance, IceCandidateType, InteractionDynamicsEvent,
    InteractionTurnType, LifecycleEventType, MessageVisibility, Participant, ParticipantKind,
    SessionLifecycleEvent, SessionMode, SessionRole, TtrpgMechanicsEvent, WebRtcEventKind,
    WebRtcTelemetryEvent, subjects,
};
use satya_storage::{EventLogRouter, StorageConfig, TopicMetricsSnapshot};
use satya_web::render_satya_app_html;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

pub struct SignalingAppState {
    pub auth: AuthAndSessionManager,
    pub storage: Arc<EventLogRouter>,
    pub sequence_counter: AtomicI64,
    /// Active WebSocket connections per `session_id -> (user_id -> tx)`
    pub active_sockets: RwLock<HashMap<String, HashMap<String, mpsc::UnboundedSender<String>>>>,
}

impl SignalingAppState {
    pub async fn new(
        config: StorageConfig,
        base_dir: impl Into<PathBuf>,
    ) -> anyhow::Result<Arc<Self>> {
        let base_dir = base_dir.into();
        let auth = AuthAndSessionManager::new(&base_dir)?;
        let storage = Arc::new(EventLogRouter::new(config, &base_dir).await?);

        Ok(Arc::new(Self {
            auth,
            storage,
            sequence_counter: AtomicI64::new(1),
            active_sockets: RwLock::new(HashMap::new()),
        }))
    }

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

    pub async fn broadcast_to_session(&self, session_id: &str, payload_json: &str) {
        let guard = self.active_sockets.read().await;
        if let Some(room_map) = guard.get(session_id) {
            for tx in room_map.values() {
                let _ = tx.send(payload_json.to_string());
            }
        }
    }
}

pub fn build_router(state: Arc<SignalingAppState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/sessions/create", post(create_session_handler))
        .route("/api/sessions/join", post(join_session_handler))
        .route("/api/storage/metrics", get(storage_metrics_handler))
        .route("/api/storage/flush", post(storage_flush_handler))
        .route("/ws/session", get(ws_session_handler))
        .with_state(state)
}

async fn index_handler() -> Html<String> {
    Html(render_satya_app_html())
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub display_name: String,
}

async fn register_handler(
    State(state): State<Arc<SignalingAppState>>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    match state
        .auth
        .register_user(&req.username, &req.password, &req.display_name)
        .await
    {
        Ok(resp) => (StatusCode::OK, Json(json!(resp))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

async fn login_handler(
    State(state): State<Arc<SignalingAppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match state
        .auth
        .authenticate_user(&req.username, &req.password)
        .await
    {
        Ok(resp) => (StatusCode::OK, Json(json!(resp))).into_response(),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub session_id: String,
    pub session_password: String,
    pub session_mode: SessionMode,
    pub role: SessionRole,
    pub display_name: Option<String>,
    pub enable_ai_gm: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionEnterResponse {
    pub session_id: String,
    pub session_mode: SessionMode,
    pub join_token: String,
    pub self_participant: Participant,
    pub roster: Vec<Participant>,
}

async fn create_session_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };

    match state
        .auth
        .create_session(
            &user,
            &req.session_id,
            &req.session_password,
            req.session_mode,
            req.role,
            req.display_name,
            req.enable_ai_gm,
        )
        .await
    {
        Ok((join_token, self_participant, roster)) => {
            if let Ok(meta) = state.storage.registry().build_metadata(
                state.next_seq(),
                subjects::SESSION_LIFECYCLE,
                1,
                &req.session_id,
                req.session_mode,
            ) {
                let ev = SessionLifecycleEvent {
                    metadata: meta,
                    lifecycle_type: LifecycleEventType::SessionCreated,
                    actor: self_participant.clone(),
                    target_participant: None,
                    password_protected: true,
                    roster_snapshot: roster.clone(),
                    reason: Some("Host created password-protected session".to_string()),
                };
                let _ = state.storage.log_session_lifecycle(&ev).await;
            }

            (
                StatusCode::OK,
                Json(json!(SessionEnterResponse {
                    session_id: req.session_id,
                    session_mode: req.session_mode,
                    join_token,
                    self_participant,
                    roster,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct JoinSessionRequest {
    pub session_id: String,
    pub session_password: String,
    pub role: SessionRole,
    pub display_name: Option<String>,
}

async fn join_session_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<JoinSessionRequest>,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };

    match state
        .auth
        .join_session(
            &user,
            &req.session_id,
            &req.session_password,
            req.role,
            req.display_name,
        )
        .await
    {
        Ok((join_token, session_mode, self_participant, roster)) => {
            if let Ok(meta) = state.storage.registry().build_metadata(
                state.next_seq(),
                subjects::SESSION_LIFECYCLE,
                1,
                &req.session_id,
                session_mode,
            ) {
                let ev = SessionLifecycleEvent {
                    metadata: meta,
                    lifecycle_type: LifecycleEventType::ParticipantJoined,
                    actor: self_participant.clone(),
                    target_participant: None,
                    password_protected: true,
                    roster_snapshot: roster.clone(),
                    reason: Some(
                        "Authenticated user joined with valid session password".to_string(),
                    ),
                };
                let _ = state.storage.log_session_lifecycle(&ev).await;
            }

            (
                StatusCode::OK,
                Json(json!(SessionEnterResponse {
                    session_id: req.session_id,
                    session_mode,
                    join_token,
                    self_participant,
                    roster,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Serialize)]
pub struct StorageMetricsResponse {
    pub topics: Vec<TopicMetricsSnapshot>,
}

async fn storage_metrics_handler(
    State(state): State<Arc<SignalingAppState>>,
) -> Json<StorageMetricsResponse> {
    let topics = state.storage.metrics_snapshot().await;
    Json(StorageMetricsResponse { topics })
}

async fn storage_flush_handler(State(state): State<Arc<SignalingAppState>>) -> impl IntoResponse {
    let flushed = state.storage.flush_all_topics().await.unwrap_or_default();
    Json(json!({ "flushed_files": flushed }))
}

#[derive(Debug, Deserialize)]
pub struct WsConnectQuery {
    pub join_token: String,
}

async fn ws_session_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsConnectQuery>,
    State(state): State<Arc<SignalingAppState>>,
) -> impl IntoResponse {
    let Some(claims) = state.auth.verify_join_token(&query.join_token).await else {
        return (StatusCode::UNAUTHORIZED, "Invalid join_token").into_response();
    };
    ws.on_upgrade(move |socket| handle_ws_client(socket, claims, state))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
enum ClientWsPacket {
    SendChat {
        visibility: MessageVisibility,
        #[serde(default)]
        recipient_user_ids: Vec<String>,
        channel_context: ChannelContext,
        body: String,
        #[serde(default)]
        composition_time_ms: i64,
    },
    WebrtcSignal {
        target_user_id: String,
        signal_kind: WebRtcEventKind,
        payload: String,
    },
    WebrtcTelemetry {
        target_user_id: Option<String>,
        event_kind: WebRtcEventKind,
        audio_muted: bool,
        video_muted: bool,
    },
    InteractionDynamics {
        turn_type: InteractionTurnType,
        duration_ms: i64,
        rms_energy_mean: f32,
        rms_energy_peak: f32,
        pitch_hz_estimate: Option<f32>,
        preceding_silence_ms: i64,
    },
    RollTtrpgTask {
        skill_name: String,
        attribute_tn: i32,
        pool_size: usize,
        difficulty_rating: i32,
        combat_stance: Option<CombatStance>,
        fatigue_level: i32,
    },
}

async fn handle_ws_client(
    socket: WebSocket,
    claims: JoinTokenClaims,
    state: Arc<SignalingAppState>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let session_id = claims.session_id.clone();
    let session_mode = claims.session_mode;
    let self_participant = claims.participant.clone();
    let user_id = self_participant.user_id.clone();

    {
        let mut guard = state.active_sockets.write().await;
        guard
            .entry(session_id.clone())
            .or_default()
            .insert(user_id.clone(), tx);
    }

    // Broadcast updated roster to all connected peers in this session
    let roster = state.auth.get_session_roster(&session_id).await;
    let sync_msg = json!({
        "type": "ROSTER_SYNC",
        "roster": roster
    })
    .to_string();
    state.broadcast_to_session(&session_id, &sync_msg).await;

    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(WsMessage::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_rx.next().await {
        let WsMessage::Text(text) = msg else {
            continue;
        };
        let Ok(packet) = serde_json::from_str::<ClientWsPacket>(&text) else {
            continue;
        };

        let current_roster = state.auth.get_session_roster(&session_id).await;

        match packet {
            ClientWsPacket::SendChat {
                visibility,
                recipient_user_ids,
                channel_context,
                body,
                composition_time_ms,
            } => {
                let (recipients, delivery_ids) = SignalingAppState::resolve_message_targets(
                    &self_participant,
                    visibility,
                    &recipient_user_ids,
                    &current_roster,
                );

                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::CHAT_MESSAGE,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let word_count = body.split_whitespace().count() as i32;
                    let char_count = body.chars().count() as i32;
                    let chat_event = ChatMessageEvent {
                        metadata: meta,
                        message_id: Uuid::now_v7().to_string(),
                        sender: self_participant.clone(),
                        visibility,
                        recipients: recipients.clone(),
                        channel_context,
                        reply_to_message_id: None,
                        body: body.clone(),
                        char_count,
                        word_count,
                        composition_time_ms,
                        is_edited: false,
                    };

                    let _ = state.storage.log_chat_message(&chat_event).await;
                    let pkt_str = json!({
                        "type": "CHAT_EVENT",
                        "event": chat_event
                    })
                    .to_string();
                    state
                        .send_to_users(&session_id, &delivery_ids, &pkt_str)
                        .await;

                    // If an AI GM is in this TTRPG session and receives a GM_ONLY or PUBLIC IC message,
                    // have the AI GM synthesize an immediate in-world response!
                    if session_mode == SessionMode::Ttrpg
                        && self_participant.participant_kind == ParticipantKind::Human
                        && (visibility == MessageVisibility::GmOnly
                            || body.to_lowercase().contains("gm"))
                        && let Some(ai_gm) = current_roster
                            .iter()
                            .find(|p| p.role == SessionRole::AiGm)
                            .cloned()
                    {
                        trigger_ai_gm_chat_reply(
                            &state,
                            &session_id,
                            session_mode,
                            &ai_gm,
                            &self_participant,
                            visibility,
                            &body,
                            &current_roster,
                        )
                        .await;
                    }
                }
            }
            ClientWsPacket::WebrtcSignal {
                target_user_id,
                signal_kind,
                payload,
            } => {
                let remote_p = current_roster
                    .iter()
                    .find(|p| p.user_id == target_user_id)
                    .cloned();

                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::WEBRTC_TELEMETRY,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let tel_ev = WebRtcTelemetryEvent {
                        metadata: meta,
                        local_participant: self_participant.clone(),
                        remote_participant: remote_p,
                        event_kind: signal_kind,
                        ice_candidate_type: Some(IceCandidateType::Host),
                        rtt_ms: Some(4.2),
                        jitter_ms: Some(0.8),
                        packet_loss_rate: Some(0.0),
                        available_outgoing_bitrate_bps: Some(2_500_000),
                        audio_muted: false,
                        video_muted: false,
                    };
                    let _ = state.storage.log_webrtc_telemetry(&tel_ev).await;
                }

                let relay_json = json!({
                    "type": "WEBRTC_SIGNAL",
                    "from_participant": self_participant,
                    "signal_kind": signal_kind,
                    "payload": payload
                })
                .to_string();
                state
                    .send_to_users(&session_id, &[target_user_id], &relay_json)
                    .await;
            }
            ClientWsPacket::WebrtcTelemetry {
                target_user_id,
                event_kind,
                audio_muted,
                video_muted,
            } => {
                let remote_p = target_user_id
                    .and_then(|uid| current_roster.iter().find(|p| p.user_id == uid).cloned());
                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::WEBRTC_TELEMETRY,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let tel_ev = WebRtcTelemetryEvent {
                        metadata: meta,
                        local_participant: self_participant.clone(),
                        remote_participant: remote_p,
                        event_kind,
                        ice_candidate_type: Some(IceCandidateType::Host),
                        rtt_ms: Some(3.5),
                        jitter_ms: Some(0.5),
                        packet_loss_rate: Some(0.0),
                        available_outgoing_bitrate_bps: Some(3_000_000),
                        audio_muted,
                        video_muted,
                    };
                    let _ = state.storage.log_webrtc_telemetry(&tel_ev).await;
                }
            }
            ClientWsPacket::InteractionDynamics {
                turn_type,
                duration_ms,
                rms_energy_mean,
                rms_energy_peak,
                pitch_hz_estimate,
                preceding_silence_ms,
            } => {
                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::INTERACTION_DYNAMICS,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let dyn_ev = InteractionDynamicsEvent {
                        metadata: meta,
                        speaker: self_participant.clone(),
                        turn_type,
                        duration_ms,
                        rms_energy_mean,
                        rms_energy_peak,
                        pitch_hz_estimate,
                        overlapped_participants: Vec::new(),
                        preceding_silence_ms,
                        dominance_ratio: 1.0 / (current_roster.len().max(1) as f32),
                    };
                    let _ = state.storage.log_interaction_dynamics(&dyn_ev).await;
                }
            }
            ClientWsPacket::RollTtrpgTask {
                skill_name,
                attribute_tn,
                pool_size,
                difficulty_rating,
                combat_stance,
                fatigue_level,
            } => {
                let count = pool_size.clamp(1, 12);
                let rolls: Vec<i32> = {
                    let mut rng = rand::thread_rng();
                    (0..count).map(|_| rng.gen_range(1..=20)).collect()
                };

                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::TTRPG_MECHANICS,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let mech_ev = TtrpgMechanicsEvent::evaluate_satya_task(
                        meta,
                        self_participant.clone(),
                        Vec::new(),
                        skill_name.clone(),
                        attribute_tn,
                        difficulty_rating,
                        rolls,
                        combat_stance,
                        fatigue_level,
                        0.0,
                    );
                    let _ = state.storage.log_ttrpg_mechanics(&mech_ev).await;

                    let pkt_str = json!({
                        "type": "TTRPG_MECHANICS_EVENT",
                        "event": mech_ev
                    })
                    .to_string();
                    state.broadcast_to_session(&session_id, &pkt_str).await;

                    // Have AI GM narrate the task resolution outcome if present
                    if let Some(ai_gm) = current_roster
                        .iter()
                        .find(|p| p.role == SessionRole::AiGm)
                        .cloned()
                    {
                        let narration = format!(
                            "[Satya Simulation] {} attempted '{}' (TN {}, Diff {}) -> {:?} ({} successes). Action Count Cost: {} ticks.",
                            self_participant.display_name,
                            skill_name,
                            attribute_tn,
                            difficulty_rating,
                            mech_ev.outcome_tier,
                            mech_ev.successes_counted,
                            mech_ev.action_count_cost.unwrap_or(10)
                        );
                        trigger_ai_gm_chat_reply(
                            &state,
                            &session_id,
                            session_mode,
                            &ai_gm,
                            &self_participant,
                            MessageVisibility::Public,
                            &narration,
                            &current_roster,
                        )
                        .await;
                    }
                }
            }
        }
    }

    forward_task.abort();
    {
        let mut guard = state.active_sockets.write().await;
        if let Some(room_map) = guard.get_mut(&session_id) {
            room_map.remove(&user_id);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn trigger_ai_gm_chat_reply(
    state: &Arc<SignalingAppState>,
    session_id: &str,
    session_mode: SessionMode,
    ai_gm: &Participant,
    triggering_player: &Participant,
    visibility: MessageVisibility,
    context_text: &str,
    roster: &[Participant],
) {
    let reply_vis = if visibility == MessageVisibility::GmOnly {
        MessageVisibility::Private
    } else {
        MessageVisibility::Public
    };
    let req_ids = vec![triggering_player.user_id.clone()];
    let (recipients, delivery_ids) =
        SignalingAppState::resolve_message_targets(ai_gm, reply_vis, &req_ids, roster);

    let response_body = if visibility == MessageVisibility::GmOnly {
        format!(
            "[Whisper to {}] Copy that. Environmental telemetry shows elevated heat signatures in Sector 4—proceed with caution.",
            triggering_player.display_name
        )
    } else {
        format!("AI GM Adjudication: {context_text}")
    };

    if let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::CHAT_MESSAGE,
        1,
        session_id,
        session_mode,
    ) {
        let ev = ChatMessageEvent {
            metadata: meta,
            message_id: Uuid::now_v7().to_string(),
            sender: ai_gm.clone(),
            visibility: reply_vis,
            recipients,
            channel_context: ChannelContext::GmNarration,
            reply_to_message_id: None,
            body: response_body.clone(),
            char_count: response_body.chars().count() as i32,
            word_count: response_body.split_whitespace().count() as i32,
            composition_time_ms: 45,
            is_edited: false,
        };
        let _ = state.storage.log_chat_message(&ev).await;
        let pkt_str = json!({
            "type": "CHAT_EVENT",
            "event": ev
        })
        .to_string();
        state
            .send_to_users(session_id, &delivery_ids, &pkt_str)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_auth_session_password_and_private_message_isolation() {
        let temp_dir =
            std::env::temp_dir().join(format!("satya_signaling_test_{}", Uuid::new_v4()));
        let state = SignalingAppState::new(StorageConfig::default(), &temp_dir)
            .await
            .unwrap();

        // 1. Register and authenticate users
        let gm_auth = state
            .auth
            .register_user("gm_test", "strong-pass-1", "GM Arbiter")
            .await
            .unwrap();
        let p1_auth = state
            .auth
            .register_user("player_one", "strong-pass-2", "Player One")
            .await
            .unwrap();
        let p2_auth = state
            .auth
            .register_user("player_two", "strong-pass-3", "Player Two")
            .await
            .unwrap();

        // Re-login returns the exact same persistent user_id
        let gm_relogin = state
            .auth
            .authenticate_user("gm_test", "strong-pass-1")
            .await
            .unwrap();
        assert_eq!(gm_auth.user_id, gm_relogin.user_id);

        let gm_user = state
            .auth
            .verify_auth_token(&gm_auth.auth_token)
            .await
            .unwrap();
        let p1_user = state
            .auth
            .verify_auth_token(&p1_auth.auth_token)
            .await
            .unwrap();
        let p2_user = state
            .auth
            .verify_auth_token(&p2_auth.auth_token)
            .await
            .unwrap();

        // 2. Host/GM creates a password-protected session
        let (_, gm_part, _) = state
            .auth
            .create_session(
                &gm_user,
                "camp-alpha",
                "secret-room-pw",
                SessionMode::Ttrpg,
                SessionRole::Gm,
                None,
                true,
            )
            .await
            .unwrap();

        // 3. Joining with wrong session password fails
        let bad_join = state
            .auth
            .join_session(
                &p1_user,
                "camp-alpha",
                "wrong-password",
                SessionRole::Player,
                None,
            )
            .await;
        assert!(bad_join.is_err());

        // 4. Joining with valid session password succeeds
        let (_, _, p1_part, _) = state
            .auth
            .join_session(
                &p1_user,
                "camp-alpha",
                "secret-room-pw",
                SessionRole::Player,
                None,
            )
            .await
            .unwrap();
        let (_, _, p2_part, roster) = state
            .auth
            .join_session(
                &p2_user,
                "camp-alpha",
                "secret-room-pw",
                SessionRole::Player,
                None,
            )
            .await
            .unwrap();

        // Roster has GM + Player 1 + Player 2 + AI GM = 4 participants
        assert_eq!(roster.len(), 4);

        // 5. Private message from Player 1 to Player 2 ONLY targets Player 2 (and sender Player 1),
        // excluding GM and AI GM from delivery
        let (recip_records, delivery_ids) = SignalingAppState::resolve_message_targets(
            &p1_part,
            MessageVisibility::Private,
            std::slice::from_ref(&p2_part.user_id),
            &roster,
        );
        assert_eq!(recip_records.len(), 1);
        assert_eq!(recip_records[0].user_id, p2_part.user_id);
        assert!(delivery_ids.contains(&p1_part.user_id));
        assert!(delivery_ids.contains(&p2_part.user_id));
        assert!(!delivery_ids.contains(&gm_part.user_id));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_axum_http_endpoints_and_leptos_render() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let temp_dir = std::env::temp_dir().join(format!("satya_router_test_{}", Uuid::new_v4()));
        let state = SignalingAppState::new(StorageConfig::default(), &temp_dir)
            .await
            .unwrap();
        let app = build_router(state.clone());

        // 1. Verify Leptos HTML SSR shell renders at GET /
        let res = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        // 2. Verify /api/storage/metrics returns all 6 topics with CRC-64-AVRO fingerprints
        let res_metrics = app
            .oneshot(
                Request::builder()
                    .uri("/api/storage/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res_metrics.status(), StatusCode::OK);

        let metrics = state.storage.metrics_snapshot().await;
        assert_eq!(metrics.len(), 6);
        let schema_reg_topic = metrics
            .iter()
            .find(|m| m.topic == "schema-registry")
            .unwrap();
        assert_eq!(schema_reg_topic.total_messages_ingested, 7);
        assert_eq!(schema_reg_topic.flushed_batches_count, 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
