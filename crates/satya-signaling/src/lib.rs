//! Axum HTTP + WebSocket signaling server, session lifecycle orchestration,
//! targeted chat routing, and Apache Iggy Avro event ingestion for Satya.
#![deny(missing_docs)]

pub mod auth;

use crate::auth::{AuthAndSessionManager, JoinTokenClaims, SessionDisconnectSummary};
use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use rand::Rng;
use satya_schema::{
    AccountRole, AiGmOperatingMode, AiGmProviderConfig, AiGmTrajectoryEvent, AiGmTurnStatus,
    AiGmTurnTriggerRequest, CanvasEventType, ChannelContext, ChatMessageEvent, ClientWsPacket,
    CreateSessionRequest, GmChatRequest, GmMapUpdateRequest, GmRollRequest, IceCandidateType,
    InteractionDynamicsEvent, JoinSessionRequest, LeaveSessionRequest, LifecycleEventType,
    LogLevel, LoginRequest, LogoutRequest, McpToolInvocationRecord, MechanicEventType,
    MessageVisibility, Participant, ParticipantKind, RegisterRequest, ServerWsPacket,
    SessionEnterResponse, SessionLifecycleEvent, SessionLogEvent, SessionMode, SessionRole,
    TabletopCanvasEvent, TtrpgMechanicsEvent, WebRtcEventKind, WebRtcTelemetryEvent, WgpuDiceSpec,
    subjects,
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

/// Constructs the Axum [`Router`] with all HTTP REST endpoints, static `/pkg` and `/maps` asset serving,
/// Backend GM REST endpoints (`/api/gm/sessions/:id/*`), and the `/ws/session` WebSocket upgrade route.
pub fn build_router(state: Arc<SignalingAppState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .nest_service("/pkg", tower_http::services::ServeDir::new("pkg"))
        .nest_service("/maps", tower_http::services::ServeDir::new("data/maps"))
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/logout", post(logout_handler))
        .route("/api/sessions/create", post(create_session_handler))
        .route("/api/sessions/join", post(join_session_handler))
        .route("/api/sessions/leave", post(leave_session_handler))
        .route(
            "/api/sessions/:session_id/gm-config",
            post(update_session_gm_config_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/context",
            get(gm_session_context_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/chat",
            post(gm_session_chat_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/roll",
            post(gm_session_roll_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/map",
            post(gm_session_map_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/trajectory",
            post(gm_session_trajectory_handler),
        )
        .route("/api/storage/metrics", get(storage_metrics_handler))
        .route("/api/storage/flush", post(storage_flush_handler))
        .route("/ws/session", get(ws_session_handler))
        .with_state(state)
}

async fn index_handler() -> Html<String> {
    Html(render_satya_app_html())
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

async fn logout_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    body: Option<Json<LogoutRequest>>,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };

    let req = body.map(|Json(r)| r).unwrap_or_default();
    let reason = req
        .reason
        .unwrap_or_else(|| "User logged out of session".to_string());

    let Some((user, disconnected_sessions)) = state.auth.logout_user(auth_hdr).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or already revoked auth token" })),
        )
            .into_response();
    };

    let mut disconnected_ids = Vec::new();
    let mut last_mode = SessionMode::Ttrpg;
    for summary in &disconnected_sessions {
        disconnected_ids.push(summary.session_id.clone());
        last_mode = summary.session_mode;
        state
            .process_session_disconnect(summary, LifecycleEventType::ParticipantLeft, &reason)
            .await;
    }

    let lifecycle_session_id = req
        .session_id
        .or_else(|| disconnected_ids.first().cloned())
        .unwrap_or_else(|| "auth-session".to_string());

    if let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::SESSION_LIFECYCLE,
        1,
        &lifecycle_session_id,
        last_mode,
    ) {
        let actor = Participant::human(
            &user.user_id,
            &user.username,
            &user.display_name,
            SessionRole::Participant,
        );
        let ev = SessionLifecycleEvent {
            metadata: meta,
            lifecycle_type: LifecycleEventType::UserLoggedOut,
            actor,
            target_participant: None,
            password_protected: true,
            roster_snapshot: Vec::new(),
            reason: Some(reason),
        };
        let _ = state.storage.log_session_lifecycle(&ev).await;
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "logged_out",
            "user_id": user.user_id,
            "username": user.username,
            "disconnected_sessions": disconnected_ids,
        })),
    )
        .into_response()
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
        Ok((join_token, self_participant, mut roster)) => {
            if req.session_mode == SessionMode::Ttrpg && req.enable_ai_gm {
                let ai_cfg = req.ai_gm_config.unwrap_or_else(|| {
                    let defaults = &state.storage.config().ai_gm;
                    AiGmProviderConfig {
                        provider: defaults.default_provider.clone(),
                        model_id: defaults.reasoning_model.clone(),
                        vision_model_id: Some(defaults.vision_model.clone()),
                        temperature: defaults.temperature,
                        persona_id: defaults.persona_id.clone(),
                        api_key: None,
                        base_url: Some(defaults.ollama_base_url.clone()),
                    }
                });
                if let Ok((_, updated_roster)) = state
                    .auth
                    .configure_session_ai_gm(&req.session_id, &user.user_id, ai_cfg)
                    .await
                {
                    roster = updated_roster;
                }
            }

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

async fn leave_session_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<LeaveSessionRequest>,
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

    let reason = req
        .reason
        .unwrap_or_else(|| "User disconnected from session".to_string());

    if let Some(summary) = state
        .auth
        .leave_session(&req.session_id, &user.user_id)
        .await
    {
        state
            .process_session_disconnect(&summary, LifecycleEventType::ParticipantLeft, &reason)
            .await;

        (
            StatusCode::OK,
            Json(json!({
                "status": "disconnected",
                "session_id": summary.session_id,
                "user_id": user.user_id,
                "remaining_roster": summary.remaining_roster,
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::OK,
            Json(json!({
                "status": "already_disconnected",
                "session_id": req.session_id,
                "user_id": user.user_id,
                "remaining_roster": state.auth.get_session_roster(&req.session_id).await,
            })),
        )
            .into_response()
    }
}

/// JSON response returned by `GET /api/storage/metrics` containing per-topic telemetry.
#[derive(Debug, Serialize)]
pub struct StorageMetricsResponse {
    /// Metrics snapshots for all 6 Apache Iggy topics.
    pub topics: Vec<TopicMetricsSnapshot>,
}

async fn verify_admin_header(
    state: &SignalingAppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        ));
    };
    let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        ));
    };
    if user.account_role != AccountRole::Admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(
                json!({ "error": "Admin account role required to access storage telemetry and batch sinks" }),
            ),
        ));
    }
    Ok(())
}

async fn storage_metrics_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_admin_header(&state, &headers).await {
        return err_resp.into_response();
    }
    let topics = state.storage.metrics_snapshot().await;
    (StatusCode::OK, Json(StorageMetricsResponse { topics })).into_response()
}

async fn storage_flush_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_admin_header(&state, &headers).await {
        return err_resp.into_response();
    }
    let flushed = state.storage.flush_all_topics().await.unwrap_or_default();
    (StatusCode::OK, Json(json!({ "flushed_files": flushed }))).into_response()
}

fn verify_gm_service_header(
    state: &SignalingAppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header for GM service" })),
        ));
    };
    let clean = auth_hdr.strip_prefix("Bearer ").unwrap_or(auth_hdr).trim();
    let expected = state.storage.config().ai_gm.service_token.trim();
    if clean.is_empty() || clean != expected {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid AI GM service_token" })),
        ));
    }
    Ok(())
}

async fn update_session_gm_config_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<AiGmProviderConfig>,
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
        .configure_session_ai_gm(&session_id, &user.user_id, req)
        .await
    {
        Ok((ai_participant, roster)) => {
            if let Ok(sync_msg) = serde_json::to_string(&ServerWsPacket::RosterSync {
                roster: roster.clone(),
            }) {
                state.broadcast_to_session(&session_id, &sync_msg).await;
            }
            (
                StatusCode::OK,
                Json(json!({
                    "status": "configured",
                    "session_id": session_id,
                    "ai_gm_participant": ai_participant,
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

async fn gm_session_context_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };
    (StatusCode::OK, Json(ctx)).into_response()
}

async fn gm_session_chat_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<GmChatRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };

    let default_cfg = &state.storage.config().ai_gm;
    let mut sender = ctx.ai_gm_participant.unwrap_or_else(|| {
        Participant::ai_gm(
            &session_id,
            &format!(
                "{}/{}",
                default_cfg.default_provider, default_cfg.reasoning_model
            ),
            &default_cfg.persona_id,
        )
    });
    if let Some(speaker) = req
        .speaker_override
        .as_ref()
        .filter(|s| !s.trim().is_empty())
    {
        sender.display_name = speaker.trim().to_string();
    }
    if let Some(mid) = req.model_id.as_ref().filter(|s| !s.trim().is_empty())
        && let Some(info) = sender.ai_agent_info.as_mut()
    {
        info.model_id = mid.trim().to_string();
    }

    let (recipients, delivery_ids) = SignalingAppState::resolve_message_targets(
        &sender,
        req.visibility,
        &req.recipient_user_ids,
        &ctx.roster,
    );

    let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::CHAT_MESSAGE,
        1,
        &session_id,
        ctx.session_mode,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to build EventMetadata" })),
        )
            .into_response();
    };

    let chat_event = ChatMessageEvent {
        metadata: meta,
        message_id: Uuid::now_v7().to_string(),
        sender,
        visibility: req.visibility,
        recipients,
        channel_context: req.channel_context,
        reply_to_message_id: None,
        body: req.body.clone(),
        char_count: req.body.chars().count() as i32,
        word_count: req.body.split_whitespace().count() as i32,
        composition_time_ms: 50,
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

    (StatusCode::OK, Json(chat_event)).into_response()
}

async fn gm_session_roll_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<GmRollRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };

    let default_cfg = &state.storage.config().ai_gm;
    let mut actor = ctx.ai_gm_participant.unwrap_or_else(|| {
        Participant::ai_gm(
            &session_id,
            &format!(
                "{}/{}",
                default_cfg.default_provider, default_cfg.reasoning_model
            ),
            &default_cfg.persona_id,
        )
    });
    if !req.actor_display_name.trim().is_empty() {
        actor.display_name = req.actor_display_name.trim().to_string();
    }

    let target_participants: Vec<Participant> = ctx
        .roster
        .iter()
        .filter(|p| req.target_user_ids.contains(&p.user_id))
        .cloned()
        .collect();

    let count = req.pool_size.clamp(1, 12);
    let (rolls, physics_seed): (Vec<i32>, i64) = {
        let mut rng = rand::thread_rng();
        let outcomes = (0..count).map(|_| rng.gen_range(1..=20)).collect();
        let seed: i64 = rng.r#gen::<i64>().abs();
        (outcomes, seed)
    };

    let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TTRPG_MECHANICS,
        1,
        &session_id,
        ctx.session_mode,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to build EventMetadata" })),
        )
            .into_response();
    };

    let mut mech_ev = TtrpgMechanicsEvent::evaluate_satya_task(
        meta,
        actor.clone(),
        target_participants,
        req.skill_name.clone(),
        req.attribute_tn,
        req.difficulty_rating,
        rolls.clone(),
        req.combat_stance,
        req.fatigue_level,
        req.wound_shock_index,
    );
    mech_ev.mechanic_type = MechanicEventType::AiGmDecision;
    mech_ev.notes = req.notes.clone();

    let _ = state.storage.log_ttrpg_mechanics(&mech_ev).await;

    // Also emit Topic 7 (`tabletop-canvas`) WgpuDiceSpawned event & broadcast 3D dice seed
    let dice_spec = WgpuDiceSpec {
        dice_notation: format!("{count}d20"),
        physics_seed,
        outcomes: rolls,
        visibility: MessageVisibility::Public,
    };
    if let Ok(canvas_meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TABLETOP_CANVAS,
        1,
        &session_id,
        ctx.session_mode,
    ) {
        let canvas_ev = TabletopCanvasEvent {
            metadata: canvas_meta,
            canvas_event_type: CanvasEventType::WgpuDiceSpawned,
            actor: actor.clone(),
            map_id: ctx.active_map.as_ref().map(|m| m.map_id.clone()),
            map_title: ctx.active_map.as_ref().map(|m| m.title.clone()),
            image_url: ctx.active_map.as_ref().map(|m| m.image_url.clone()),
            grid_width: ctx.active_map.as_ref().map(|m| m.grid_width),
            grid_height: ctx.active_map.as_ref().map(|m| m.grid_height),
            token_placements: Vec::new(),
            wgpu_dice_roll: Some(dice_spec.clone()),
            terrain_notes: req.notes,
        };
        let _ = state.storage.log_tabletop_canvas(&canvas_ev).await;
    }

    if let Ok(wgpu_pkt) = serde_json::to_string(&ServerWsPacket::WgpuDiceRoll {
        actor,
        spec: dice_spec,
    }) {
        state.broadcast_to_session(&session_id, &wgpu_pkt).await;
    }

    let pkt_str = json!({
        "type": "TTRPG_MECHANICS_EVENT",
        "event": mech_ev
    })
    .to_string();
    state.broadcast_to_session(&session_id, &pkt_str).await;

    (StatusCode::OK, Json(mech_ev)).into_response()
}

async fn gm_session_map_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<GmMapUpdateRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };

    let _ = state
        .auth
        .set_session_active_map(&session_id, req.clone())
        .await;

    let default_cfg = &state.storage.config().ai_gm;
    let actor = ctx.ai_gm_participant.unwrap_or_else(|| {
        Participant::ai_gm(
            &session_id,
            &format!(
                "{}/{}",
                default_cfg.default_provider, default_cfg.reasoning_model
            ),
            &default_cfg.persona_id,
        )
    });

    let Ok(canvas_meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TABLETOP_CANVAS,
        1,
        &session_id,
        ctx.session_mode,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to build EventMetadata" })),
        )
            .into_response();
    };

    let canvas_ev = TabletopCanvasEvent {
        metadata: canvas_meta,
        canvas_event_type: if ctx.active_map.is_some() {
            CanvasEventType::MapUpdated
        } else {
            CanvasEventType::MapGenerated
        },
        actor,
        map_id: Some(req.map_id.clone()),
        map_title: Some(req.title.clone()),
        image_url: Some(req.image_url.clone()),
        grid_width: Some(req.grid_width),
        grid_height: Some(req.grid_height),
        token_placements: req.tokens.clone(),
        wgpu_dice_roll: None,
        terrain_notes: req.notes.clone(),
    };
    let _ = state.storage.log_tabletop_canvas(&canvas_ev).await;

    if let Ok(map_pkt) = serde_json::to_string(&ServerWsPacket::MapUpdated { map: Box::new(req) }) {
        state.broadcast_to_session(&session_id, &map_pkt).await;
    }

    (StatusCode::OK, Json(canvas_ev)).into_response()
}

/// Request payload for `POST /api/gm/sessions/:session_id/trajectory`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmTrajectoryReportRequest {
    /// Unique UUIDv7 identifier for the completed LangGraph turn.
    pub turn_id: String,
    /// 32-character hex W3C `trace_id`.
    pub trace_id: String,
    /// Operating mode (`AUTONOMOUS` or `CO_PILOT`).
    pub operating_mode: AiGmOperatingMode,
    /// Triggering participant.
    pub trigger_actor: Participant,
    /// `event_id` or `message_id` that triggered the turn.
    pub trigger_event_id: String,
    /// Summary of the triggering action.
    pub trigger_summary: String,
    /// LLM provider identifier.
    pub llm_provider: String,
    /// Reasoning model ID.
    pub reasoning_model_id: String,
    /// Optional vision model ID.
    #[serde(default)]
    pub vision_model_id: Option<String>,
    /// Ruleset engine ID (e.g. `"satya-d20"`).
    #[serde(default = "default_ruleset_id")]
    pub ruleset_engine_id: String,
    /// Ordered tool invocations executed during the turn.
    #[serde(default)]
    pub tool_invocations: Vec<McpToolInvocationRecord>,
    /// RAG/lore sources consulted.
    #[serde(default)]
    pub rag_sources_used: Vec<String>,
    /// Emitted event IDs.
    #[serde(default)]
    pub emitted_event_ids: Vec<String>,
    /// Prompt token count.
    #[serde(default)]
    pub prompt_tokens: i32,
    /// Completion token count.
    #[serde(default)]
    pub completion_tokens: i32,
    /// Cached token count.
    #[serde(default)]
    pub cached_tokens: i32,
    /// Total turn latency in milliseconds.
    pub total_turn_latency_ms: i64,
    /// Completion status of the turn.
    pub turn_status: AiGmTurnStatus,
}

fn default_ruleset_id() -> String {
    "satya-d20".to_string()
}

async fn gm_session_trajectory_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<GmTrajectoryReportRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };

    let default_cfg = &state.storage.config().ai_gm;
    let ai_participant = ctx.ai_gm_participant.unwrap_or_else(|| {
        Participant::ai_gm(
            &session_id,
            &format!("{}/{}", req.llm_provider, req.reasoning_model_id),
            &default_cfg.persona_id,
        )
    });

    let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::AI_GM_TRAJECTORY,
        1,
        &session_id,
        ctx.session_mode,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to build EventMetadata" })),
        )
            .into_response();
    };

    let traj_ev = AiGmTrajectoryEvent {
        metadata: meta,
        turn_id: req.turn_id,
        trace_id: req.trace_id,
        operating_mode: req.operating_mode,
        ai_participant,
        trigger_actor: req.trigger_actor,
        trigger_event_id: req.trigger_event_id,
        trigger_summary: req.trigger_summary,
        llm_provider: req.llm_provider,
        reasoning_model_id: req.reasoning_model_id,
        vision_model_id: req.vision_model_id,
        ruleset_engine_id: req.ruleset_engine_id,
        tool_invocations: req.tool_invocations,
        rag_sources_used: req.rag_sources_used,
        emitted_event_ids: req.emitted_event_ids,
        prompt_tokens: req.prompt_tokens,
        completion_tokens: req.completion_tokens,
        cached_tokens: req.cached_tokens,
        total_turn_latency_ms: req.total_turn_latency_ms,
        turn_status: req.turn_status,
    };

    let _ = state.storage.log_ai_gm_trajectory(&traj_ev).await;
    (StatusCode::OK, Json(traj_ev)).into_response()
}

/// Query string parameters required when upgrading `/ws/session?join_token=...`.
#[derive(Debug, Deserialize)]
pub struct WsConnectQuery {
    /// Single-session join token returned by `/api/sessions/create` or `/api/sessions/join`.
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

                    if session_mode == SessionMode::Ttrpg
                        && self_participant.participant_kind == ParticipantKind::Human
                        && let Some(ai_gm) = current_roster
                            .iter()
                            .find(|p| p.role == SessionRole::AiGm)
                            .cloned()
                        && should_trigger_ai_gm_for_chat(
                            &current_roster,
                            &ai_gm,
                            visibility,
                            &recipient_user_ids,
                            channel_context,
                            &body,
                        )
                    {
                        trigger_ai_gm_chat_reply(
                            &state,
                            &session_id,
                            session_mode,
                            &ai_gm,
                            &self_participant,
                            &chat_event.message_id,
                            "CHAT",
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

                    // Have AI GM narrate the task resolution outcome when in Autonomous mode (`TTRPG_AI_GM`)
                    if let Some(ai_gm) = current_roster
                        .iter()
                        .find(|p| p.role == SessionRole::AiGm)
                        .cloned()
                        && should_trigger_ai_gm_for_roll(&current_roster)
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
                            &mech_ev.metadata.event_id,
                            "MECHANICS",
                            MessageVisibility::Public,
                            &narration,
                            &current_roster,
                        )
                        .await;
                    }
                }
            }
            ClientWsPacket::LeaveSession { reason } => {
                let disc_reason =
                    reason.unwrap_or_else(|| "User disconnected from session".to_string());
                if let Some(summary) = state.auth.leave_session(&session_id, &user_id).await {
                    state
                        .process_session_disconnect(
                            &summary,
                            LifecycleEventType::ParticipantLeft,
                            &disc_reason,
                        )
                        .await;
                }
                break;
            }
        }
    }

    forward_task.abort();
    if let Some(summary) = state.auth.leave_session(&session_id, &user_id).await {
        state
            .process_session_disconnect(
                &summary,
                LifecycleEventType::ParticipantDisconnected,
                "WebSocket connection closed",
            )
            .await;
    } else {
        let mut guard = state.active_sockets.write().await;
        if let Some(room_map) = guard.get_mut(&session_id) {
            room_map.remove(&user_id);
        }
    }
}

/// Evaluates whether a human chat message should trigger the AI GM according to the session's
/// operating topology (`TTRPG_GM_AI` Co-Pilot vs. `TTRPG_AI_GM` Autonomous):
/// - **Co-Pilot (`TTRPG_GM_AI`, human `SessionRole::Gm` present)**: Only triggers when directly addressed
///   (`@satya_ai_gm` / `@ai_gm`) or when a `GM_ONLY` or `PRIVATE` message targets the AI GM.
/// - **Autonomous (`TTRPG_AI_GM`, no human `SessionRole::Gm`)**: Triggers on all `IN_CHARACTER` messages,
///   `GM_ONLY` whispers, `PRIVATE` messages targeting the AI GM, or messages mentioning `gm`.
pub fn should_trigger_ai_gm_for_chat(
    roster: &[Participant],
    ai_gm: &Participant,
    visibility: MessageVisibility,
    recipient_user_ids: &[String],
    channel_context: ChannelContext,
    body: &str,
) -> bool {
    let has_human_gm = roster
        .iter()
        .any(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm);
    let lower = body.to_lowercase();
    let targets_ai_privately =
        visibility == MessageVisibility::Private && recipient_user_ids.contains(&ai_gm.user_id);
    let directly_mentions_ai = lower.contains("@satya_ai_gm") || lower.contains("@ai_gm");

    if has_human_gm {
        // Co-Pilot Mode (`TTRPG_GM_AI`): do not interrupt public player chat unless explicitly addressed
        visibility == MessageVisibility::GmOnly || targets_ai_privately || directly_mentions_ai
    } else {
        // Autonomous Solo AI GM Mode (`TTRPG_AI_GM`)
        visibility == MessageVisibility::GmOnly
            || targets_ai_privately
            || channel_context == ChannelContext::InCharacter
            || lower.contains("gm")
    }
}

/// Evaluates whether a `RollTtrpgTask` event should automatically trigger an AI GM narration turn.
/// In `TTRPG_AI_GM` (Autonomous mode, no human GM), the AI GM adjudicates every roll automatically;
/// in `TTRPG_GM_AI` (Co-Pilot mode, human GM present), the human GM adjudicates public player rolls.
pub fn should_trigger_ai_gm_for_roll(roster: &[Participant]) -> bool {
    !roster
        .iter()
        .any(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm)
}

/// Dispatches an [`AiGmTurnTriggerRequest`] over HTTP/1.1 to the Python FastAPI LangGraph service
/// at `{service_url}/internal/ai-gm/events`, passing the ephemeral BYOK key in `X-Satya-Provider-Key`
/// if present.
pub async fn post_ai_gm_event_to_fastapi(
    service_url: &str,
    service_token: &str,
    ephemeral_api_key: Option<&str>,
    payload: &AiGmTurnTriggerRequest,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let trimmed = service_url
        .trim()
        .strip_prefix("http://")
        .or_else(|| service_url.trim().strip_prefix("https://"))
        .unwrap_or(service_url.trim());
    let host_port = trimmed.split('/').next().unwrap_or("127.0.0.1:8090");
    let connect_addr = if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{host_port}:80")
    };

    let body_bytes = serde_json::to_vec(payload)?;
    let mut key_hdr = String::new();
    if let Some(k) = ephemeral_api_key.filter(|s| !s.trim().is_empty()) {
        key_hdr = format!("X-Satya-Provider-Key: {}\r\n", k.trim());
    }

    let req_bytes = format!(
        "POST /internal/ai-gm/events HTTP/1.1\r\n\
         Host: {host_port}\r\n\
         Authorization: Bearer {service_token}\r\n\
         {key_hdr}\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body_bytes.len()
    );

    let mut stream = tokio::time::timeout(
        std::time::Duration::from_millis(1500),
        tokio::net::TcpStream::connect(&connect_addr),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Connection to AI GM service at {connect_addr} timed out"))??;

    stream.write_all(req_bytes.as_bytes()).await?;
    stream.write_all(&body_bytes).await?;
    stream.flush().await?;

    let mut resp_buf = vec![0u8; 512];
    let n = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        stream.read(&mut resp_buf),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!("Timed out awaiting response from AI GM service at {connect_addr}")
    })??;

    let status_line = String::from_utf8_lossy(&resp_buf[..n]);
    if !status_line.starts_with("HTTP/1.1 2") && !status_line.starts_with("HTTP/1.0 2") {
        anyhow::bail!(
            "AI GM service at {connect_addr} returned non-2xx response: {}",
            status_line.lines().next().unwrap_or("empty response")
        );
    }

    Ok(())
}

/// Triggers an AI Game Master turn for `session_id`:
/// 1. Attempts to dispatch the turn to the Python FastAPI LangGraph service (`POST /internal/ai-gm/events`).
/// 2. If the FastAPI service is offline:
///    - When `ai_gm.fail_fast_when_offline == true` (end-to-end runtime configuration in `config/storage.toml`),
///      logs an error to `system-logs` (`SessionLogEvent`) and emits a fail-fast `SYSTEM_NOTICE` error to the session.
///    - When `ai_gm.fail_fast_when_offline == false` (`StorageConfig::default()` used in unit/integration tests),
///      executes the deterministic in-process reply so tests run deterministically without an external daemon.
#[allow(clippy::too_many_arguments)]
pub async fn trigger_ai_gm_chat_reply(
    state: &Arc<SignalingAppState>,
    session_id: &str,
    session_mode: SessionMode,
    ai_gm: &Participant,
    triggering_player: &Participant,
    trigger_event_id: &str,
    trigger_kind: &str,
    visibility: MessageVisibility,
    context_text: &str,
    roster: &[Participant],
) {
    let ai_cfg = state.storage.config().ai_gm.clone();
    if !ai_cfg.enabled {
        return;
    }

    let (ctx_opt, vault_opt) = match state.auth.get_session_gm_context(session_id).await {
        Some((c, v)) => (Some(c), v),
        None => (None, None),
    };

    let human_gm_user_id = ctx_opt
        .as_ref()
        .and_then(|c| c.human_gm_user_id.clone())
        .or_else(|| {
            roster
                .iter()
                .find(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm)
                .map(|p| p.user_id.clone())
        });
    let operating_mode = if human_gm_user_id.is_some() {
        AiGmOperatingMode::CoPilot
    } else {
        AiGmOperatingMode::Autonomous
    };

    let provider = vault_opt
        .as_ref()
        .map(|v| v.provider.clone())
        .unwrap_or_else(|| ai_cfg.default_provider.clone());
    let model_id = vault_opt
        .as_ref()
        .map(|v| v.model_id.clone())
        .unwrap_or_else(|| ai_cfg.reasoning_model.clone());
    let vision_model_id = vault_opt
        .as_ref()
        .and_then(|v| v.vision_model_id.clone())
        .or_else(|| Some(ai_cfg.vision_model.clone()));
    let temperature = vault_opt
        .as_ref()
        .map(|v| v.temperature)
        .unwrap_or(ai_cfg.temperature);
    let persona_id = vault_opt
        .as_ref()
        .map(|v| v.persona_id.clone())
        .unwrap_or_else(|| ai_cfg.persona_id.clone());
    let base_url = vault_opt
        .as_ref()
        .and_then(|v| v.base_url.clone())
        .or_else(|| Some(ai_cfg.ollama_base_url.clone()));
    let ephemeral_key = vault_opt.as_ref().and_then(|v| v.api_key.clone());

    let trigger_payload = AiGmTurnTriggerRequest {
        session_id: session_id.to_string(),
        session_mode,
        operating_mode,
        human_gm_user_id: human_gm_user_id.clone(),
        trigger_event_id: trigger_event_id.to_string(),
        trigger_kind: trigger_kind.to_string(),
        trigger_actor: triggering_player.clone(),
        visibility,
        context_text: context_text.to_string(),
        roster: roster.to_vec(),
        provider,
        model_id,
        vision_model_id,
        temperature,
        persona_id,
        base_url,
    };

    match post_ai_gm_event_to_fastapi(
        &ai_cfg.service_url,
        &ai_cfg.service_token,
        ephemeral_key.as_deref(),
        &trigger_payload,
    )
    .await
    {
        Ok(()) => return,
        Err(err) => {
            if ai_cfg.fail_fast_when_offline {
                let err_msg = format!(
                    "[AI GM Service Error] FastAPI AI GM service at {} is unreachable ({err}). Ensure `pixi run serve-ai-gm` is running.",
                    ai_cfg.service_url
                );
                tracing::error!(session_id = %session_id, error = %err, "AI GM service unreachable (fail_fast_when_offline=true)");

                if let Ok(log_meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::SESSION_LOG,
                    1,
                    session_id,
                    session_mode,
                ) {
                    let log_ev = SessionLogEvent {
                        metadata: log_meta,
                        log_id: Uuid::now_v7().to_string(),
                        level: LogLevel::Error,
                        service_name: "satya-signaling".to_string(),
                        actor: Some(triggering_player.clone()),
                        trace_id: None,
                        span_id: None,
                        parent_span_id: None,
                        lineage_id: Some(trigger_event_id.to_string()),
                        parent_lineage_ids: vec![],
                        log_line: format!("[ERROR] [session={session_id}] {err_msg}"),
                    };
                    let _ = state.storage.log_session_log(&log_ev).await;
                }

                // Emit fail-fast system notice to the triggering player / GM
                let req_ids = vec![triggering_player.user_id.clone()];
                let (recipients, delivery_ids) = SignalingAppState::resolve_message_targets(
                    ai_gm,
                    MessageVisibility::Private,
                    &req_ids,
                    roster,
                );
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
                        visibility: MessageVisibility::Private,
                        recipients,
                        channel_context: ChannelContext::SystemNotice,
                        reply_to_message_id: None,
                        body: err_msg.clone(),
                        char_count: err_msg.chars().count() as i32,
                        word_count: err_msg.split_whitespace().count() as i32,
                        composition_time_ms: 0,
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
                return;
            }
        }
    }

    // Deterministic fallback for unit/integration tests (`fail_fast_when_offline == false`)
    let (reply_vis, target_uid) = if operating_mode == AiGmOperatingMode::CoPilot {
        (
            MessageVisibility::Private,
            human_gm_user_id.unwrap_or_else(|| triggering_player.user_id.clone()),
        )
    } else if visibility == MessageVisibility::GmOnly || visibility == MessageVisibility::Private {
        (
            MessageVisibility::Private,
            triggering_player.user_id.clone(),
        )
    } else {
        (MessageVisibility::Public, triggering_player.user_id.clone())
    };
    let req_ids = vec![target_uid];
    let (recipients, delivery_ids) =
        SignalingAppState::resolve_message_targets(ai_gm, reply_vis, &req_ids, roster);

    let response_body = if operating_mode == AiGmOperatingMode::CoPilot {
        format!(
            "[Co-Pilot Whisper to GM] Processed request from {}: {context_text}",
            triggering_player.display_name
        )
    } else if visibility == MessageVisibility::GmOnly {
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
    use satya_schema::GmContextResponse;

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

        // 2. Verify /api/storage/metrics requires ADMIN role (401 unauthenticated, 403 for USER, 200 for ADMIN)
        let res_unauth = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/storage/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res_unauth.status(), StatusCode::UNAUTHORIZED);

        // Regular user (sara_valkyrie) is seeded with AccountRole::User -> 403 FORBIDDEN
        let sara_auth = state
            .auth
            .authenticate_user("sara_valkyrie", "satya-truth-2026")
            .await
            .unwrap();
        assert_eq!(sara_auth.account_role, AccountRole::User);
        let res_forbidden = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/storage/metrics")
                    .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res_forbidden.status(), StatusCode::FORBIDDEN);

        // Bootstrap admin (gm_sean) is seeded with AccountRole::Admin -> 200 OK
        let admin_auth = state
            .auth
            .authenticate_user("gm_sean", "satya-truth-2026")
            .await
            .unwrap();
        assert_eq!(admin_auth.account_role, AccountRole::Admin);
        let res_metrics = app
            .oneshot(
                Request::builder()
                    .uri("/api/storage/metrics")
                    .header("Authorization", format!("Bearer {}", admin_auth.auth_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res_metrics.status(), StatusCode::OK);

        let metrics = state.storage.metrics_snapshot().await;
        assert_eq!(metrics.len(), 9);
        let schema_reg_topic = metrics
            .iter()
            .find(|m| m.topic == "schema-registry")
            .unwrap();
        assert_eq!(schema_reg_topic.total_messages_ingested, 10);
        assert_eq!(schema_reg_topic.flushed_batches_count, 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_gm_backend_api_byok_vault_and_copilot_vs_autonomous_modes() {
        use axum::body::{Body, to_bytes};
        use axum::http::Request;
        use satya_schema::{CombatStance, TokenPlacement};
        use tower::ServiceExt;

        let temp_dir = std::env::temp_dir().join(format!("satya_gm_api_test_{}", Uuid::new_v4()));
        let state = SignalingAppState::new(StorageConfig::default(), &temp_dir)
            .await
            .unwrap();
        let app = build_router(state.clone());

        let gm_auth = state
            .auth
            .authenticate_user("gm_sean", "satya-truth-2026")
            .await
            .unwrap();
        let sara_auth = state
            .auth
            .authenticate_user("sara_valkyrie", "satya-truth-2026")
            .await
            .unwrap();

        let gm_user = state
            .auth
            .verify_auth_token(&gm_auth.auth_token)
            .await
            .unwrap();
        let sara_user = state
            .auth
            .verify_auth_token(&sara_auth.auth_token)
            .await
            .unwrap();

        // 1. Create a Co-Pilot session (`TTRPG_GM_AI`: Human GM + AI GM + Player)
        state
            .auth
            .create_session(
                &gm_user,
                "ep1-copilot",
                "orbital-7",
                SessionMode::Ttrpg,
                SessionRole::Gm,
                None,
                true,
            )
            .await
            .unwrap();
        let (_, _, sara_part, copilot_roster) = state
            .auth
            .join_session(
                &sara_user,
                "ep1-copilot",
                "orbital-7",
                SessionRole::Player,
                None,
            )
            .await
            .unwrap();

        let ai_gm_part = copilot_roster
            .iter()
            .find(|p| p.role == SessionRole::AiGm)
            .unwrap()
            .clone();

        // Verify Co-Pilot vs Autonomous trigger filter rules:
        // In Co-Pilot mode (Human GM present), normal public IC chat does NOT trigger AI GM
        assert!(!should_trigger_ai_gm_for_chat(
            &copilot_roster,
            &ai_gm_part,
            MessageVisibility::Public,
            &[],
            ChannelContext::InCharacter,
            "I open the cargo bay door.",
        ));
        // Addressing @satya_ai_gm or sending GM_ONLY DOES trigger AI GM in Co-Pilot mode
        assert!(should_trigger_ai_gm_for_chat(
            &copilot_roster,
            &ai_gm_part,
            MessageVisibility::Public,
            &[],
            ChannelContext::InCharacter,
            "@satya_ai_gm look up Captain Vance's stats",
        ));
        assert!(should_trigger_ai_gm_for_chat(
            &copilot_roster,
            &ai_gm_part,
            MessageVisibility::GmOnly,
            &[],
            ChannelContext::InCharacter,
            "Roll 3d20 for the two sentries",
        ));
        assert!(!should_trigger_ai_gm_for_roll(&copilot_roster));

        // 2. Configure Ephemeral BYOK Credentials via POST /api/sessions/:id/gm-config
        let cfg_req = Request::builder()
            .method("POST")
            .uri("/api/sessions/ep1-copilot/gm-config")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
            .body(Body::from(
                serde_json::to_string(&AiGmProviderConfig {
                    provider: "google_genai".to_string(),
                    model_id: "gemini-2.5-flash".to_string(),
                    vision_model_id: Some("gemini-2.5-flash".to_string()),
                    temperature: 0.3,
                    persona_id: "harsh-reality-arbiter".to_string(),
                    api_key: Some("ephemeral-secret-key-xyz999".to_string()),
                    base_url: None,
                })
                .unwrap(),
            ))
            .unwrap();
        let cfg_res = app.clone().oneshot(cfg_req).await.unwrap();
        assert_eq!(cfg_res.status(), StatusCode::OK);

        // Verify GET /api/gm/sessions/:id/context requires service_token (401 without token)
        let unauth_ctx = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/gm/sessions/ep1-copilot/context")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauth_ctx.status(), StatusCode::UNAUTHORIZED);

        // Authorized GET /api/gm/sessions/:id/context returns CO_PILOT mode and redacts api_key!
        let ctx_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/gm/sessions/ep1-copilot/context")
                    .header("Authorization", "Bearer satya_gm_secret_dev_token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ctx_res.status(), StatusCode::OK);
        let ctx_bytes = to_bytes(ctx_res.into_body(), usize::MAX).await.unwrap();
        let ctx_str = String::from_utf8(ctx_bytes.to_vec()).unwrap();
        assert!(!ctx_str.contains("ephemeral-secret-key-xyz999"));
        let gm_ctx: GmContextResponse = serde_json::from_str(&ctx_str).unwrap();
        assert_eq!(gm_ctx.operating_mode, AiGmOperatingMode::CoPilot);
        assert_eq!(
            gm_ctx.human_gm_user_id.as_deref(),
            Some(gm_user.user_id.as_str())
        );
        assert_eq!(
            gm_ctx
                .ai_gm_participant
                .unwrap()
                .ai_agent_info
                .unwrap()
                .model_id,
            "google_genai/gemini-2.5-flash"
        );

        // 3. Test POST /api/gm/sessions/:id/roll, /chat, /map, and /trajectory
        let roll_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/gm/sessions/ep1-copilot/roll")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "Bearer satya_gm_secret_dev_token")
                    .body(Body::from(
                        serde_json::to_string(&GmRollRequest {
                            actor_display_name: "Station SecOps Sentry".to_string(),
                            target_user_ids: vec![sara_part.user_id.clone()],
                            skill_name: "Suppressive Fire".to_string(),
                            attribute_tn: 13,
                            pool_size: 3,
                            difficulty_rating: 2,
                            combat_stance: Some(CombatStance::RangedSnap),
                            fatigue_level: 1,
                            wound_shock_index: 0.0,
                            notes: Some("Sentry reacts to corridor movement".to_string()),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(roll_res.status(), StatusCode::OK);

        let map_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/gm/sessions/ep1-copilot/map")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "Bearer satya_gm_secret_dev_token")
                    .body(Body::from(
                        serde_json::to_string(&GmMapUpdateRequest {
                            map_id: "sector-4-airlock".to_string(),
                            title: "Sector 4 Cargo Airlock".to_string(),
                            image_url: "/maps/ep1-copilot/sector-4-airlock.svg".to_string(),
                            grid_width: 20,
                            grid_height: 15,
                            tokens: vec![TokenPlacement {
                                entity_id: sara_part.user_id.clone(),
                                label: "Sara".to_string(),
                                x: 5,
                                y: 7,
                                visibility: MessageVisibility::Public,
                            }],
                            notes: Some("Low-gravity cargo bay".to_string()),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(map_res.status(), StatusCode::OK);

        let chat_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/gm/sessions/ep1-copilot/chat")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "Bearer satya_gm_secret_dev_token")
                    .body(Body::from(
                        serde_json::to_string(&GmChatRequest {
                            visibility: MessageVisibility::Private,
                            recipient_user_ids: vec![gm_user.user_id.clone()],
                            channel_context: ChannelContext::GmNarration,
                            body: "[Co-Pilot] Captain Vance TN 14 Leadership, wary disposition."
                                .to_string(),
                            model_id: None,
                            speaker_override: None,
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(chat_res.status(), StatusCode::OK);

        let traj_res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/gm/sessions/ep1-copilot/trajectory")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "Bearer satya_gm_secret_dev_token")
                    .body(Body::from(
                        serde_json::to_string(&GmTrajectoryReportRequest {
                            turn_id: "turn-copilot-1".to_string(),
                            trace_id: "0123456789abcdef0123456789abcdef".to_string(),
                            operating_mode: AiGmOperatingMode::CoPilot,
                            trigger_actor: sara_part.clone(),
                            trigger_event_id: "msg-1".to_string(),
                            trigger_summary: "Look up Captain Vance".to_string(),
                            llm_provider: "google_genai".to_string(),
                            reasoning_model_id: "gemini-2.5-flash".to_string(),
                            vision_model_id: None,
                            ruleset_engine_id: "satya-d20".to_string(),
                            tool_invocations: vec![],
                            rag_sources_used: vec!["data/npcs/sector_4_npcs.toml".to_string()],
                            emitted_event_ids: vec![],
                            prompt_tokens: 410,
                            completion_tokens: 65,
                            cached_tokens: 0,
                            total_turn_latency_ms: 310,
                            turn_status: AiGmTurnStatus::Completed,
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(traj_res.status(), StatusCode::OK);

        // 4. Verify ephemeral BYOK key is scrubbed when sponsoring user (GM) leaves the session
        let (_, vault_before) = state
            .auth
            .get_session_gm_context("ep1-copilot")
            .await
            .unwrap();
        assert_eq!(
            vault_before.unwrap().api_key.as_deref(),
            Some("ephemeral-secret-key-xyz999")
        );
        state
            .auth
            .leave_session("ep1-copilot", &gm_user.user_id)
            .await;
        let (ctx_after_gm_left, vault_after) = state
            .auth
            .get_session_gm_context("ep1-copilot")
            .await
            .unwrap();
        assert!(vault_after.unwrap().api_key.is_none());
        // With no human GM remaining, session automatically transitions to AUTONOMOUS mode!
        assert_eq!(
            ctx_after_gm_left.operating_mode,
            AiGmOperatingMode::Autonomous
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_session_leave_and_user_logout_emit_disconnect_events() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let temp_dir =
            std::env::temp_dir().join(format!("satya_disconnect_test_{}", Uuid::new_v4()));
        let state = SignalingAppState::new(StorageConfig::default(), &temp_dir)
            .await
            .unwrap();
        let app = build_router(state.clone());

        let gm_auth = state
            .auth
            .register_user("gm_disc", "strong-pass-1", "GM Sean")
            .await
            .unwrap();
        let p1_auth = state
            .auth
            .register_user("p1_disc", "strong-pass-2", "Player Sara")
            .await
            .unwrap();

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

        let (gm_join_tok, _, _) = state
            .auth
            .create_session(
                &gm_user,
                "ep-disconnect",
                "room-secret",
                SessionMode::Ttrpg,
                SessionRole::Gm,
                None,
                true,
            )
            .await
            .unwrap();
        let (p1_join_tok, _, _, roster_before) = state
            .auth
            .join_session(
                &p1_user,
                "ep-disconnect",
                "room-secret",
                SessionRole::Player,
                None,
            )
            .await
            .unwrap();
        assert_eq!(roster_before.len(), 3); // GM + Player 1 + AI GM

        // 1. Player 1 disconnects from the session via POST /api/sessions/leave
        let leave_req = Request::builder()
            .method("POST")
            .uri("/api/sessions/leave")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
            .body(Body::from(
                serde_json::to_string(&json!({
                    "session_id": "ep-disconnect",
                    "reason": "Player Sara clicked Disconnect"
                }))
                .unwrap(),
            ))
            .unwrap();
        let leave_res = app.clone().oneshot(leave_req).await.unwrap();
        assert_eq!(leave_res.status(), StatusCode::OK);

        // Player 1's join token is invalidated and roster now has 2 participants (GM + AI GM)
        assert!(state.auth.verify_join_token(&p1_join_tok).await.is_none());
        assert!(state.auth.verify_join_token(&gm_join_tok).await.is_some());
        let roster_after_leave = state.auth.get_session_roster("ep-disconnect").await;
        assert_eq!(roster_after_leave.len(), 2);

        // 2. GM logs out completely via POST /api/auth/logout while still in the session
        let logout_req = Request::builder()
            .method("POST")
            .uri("/api/auth/logout")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
            .body(Body::from(
                serde_json::to_string(&json!({
                    "session_id": "ep-disconnect",
                    "reason": "GM logged out"
                }))
                .unwrap(),
            ))
            .unwrap();
        let logout_res = app.oneshot(logout_req).await.unwrap();
        assert_eq!(logout_res.status(), StatusCode::OK);

        // GM's auth token and join token are both revoked
        assert!(
            state
                .auth
                .verify_auth_token(&gm_auth.auth_token)
                .await
                .is_none()
        );
        assert!(state.auth.verify_join_token(&gm_join_tok).await.is_none());

        // Verify disconnect & logout events were logged to `session-lifecycle` and `webrtc-telemetry`
        let metrics = state.storage.metrics_snapshot().await;
        let lifecycle_metrics = metrics
            .iter()
            .find(|m| m.topic == "session-lifecycle")
            .unwrap();
        // Events: P1 ParticipantLeft (1) + GM ParticipantLeft (2) + SessionEnded (3, since 0 humans left) + GM UserLoggedOut (4)
        assert_eq!(lifecycle_metrics.total_messages_ingested, 4);

        let webrtc_metrics = metrics
            .iter()
            .find(|m| m.topic == "webrtc-telemetry")
            .unwrap();
        // Events: P1 PeerDisconnected (1) + GM PeerDisconnected (2)
        assert_eq!(webrtc_metrics.total_messages_ingested, 2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_ai_gm_fail_fast_when_offline_emits_system_log_and_notice() {
        let mut cfg = StorageConfig::default();
        let temp_dir = std::env::temp_dir().join(format!("satya_sig_failfast_{}", Uuid::now_v7()));
        for topic_cfg in cfg.topics.values_mut() {
            topic_cfg.local_path = temp_dir
                .join(&topic_cfg.local_path)
                .to_string_lossy()
                .to_string();
        }
        cfg.auth.database_url = temp_dir
            .join("users_failfast.db")
            .to_string_lossy()
            .to_string();
        // Enable fail-fast behavior and point to an unreachable local port
        cfg.ai_gm.fail_fast_when_offline = true;
        cfg.ai_gm.service_url = "http://127.0.0.1:59999".to_string();

        let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
        let gm = Participant::human("u-gm-ff", "gm_sean", "Sean (GM)", SessionRole::Gm);
        let ai_gm =
            Participant::ai_gm("ep-failfast", "ollama/qwen2.5:14b", "harsh-reality-arbiter");
        let roster = vec![gm.clone(), ai_gm.clone()];

        trigger_ai_gm_chat_reply(
            &state,
            "ep-failfast",
            SessionMode::Ttrpg,
            &ai_gm,
            &gm,
            "ev-trigger-1",
            "CHAT",
            MessageVisibility::GmOnly,
            "Check Captain Vance's leadership TN",
            &roster,
        )
        .await;

        let metrics = state.storage.metrics_snapshot().await;
        let sys_logs = metrics.iter().find(|m| m.topic == "system-logs").unwrap();
        assert_eq!(sys_logs.total_messages_ingested, 1);

        let chat_logs = metrics.iter().find(|m| m.topic == "chat-messages").unwrap();
        assert_eq!(chat_logs.total_messages_ingested, 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
