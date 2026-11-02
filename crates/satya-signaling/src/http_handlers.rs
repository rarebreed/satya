//! HTTP REST handlers for Leptos SSR shell rendering, user authentication (`/api/auth/*`),
//! session creation/joining/leaving (`/api/sessions/*`), and admin storage metrics (`/api/storage/*`).

use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
};
use satya_schema::{
    AccountRole, AiGmProviderConfig, CreateSessionRequest, JoinSessionRequest, LeaveSessionRequest,
    LifecycleEventType, LoginRequest, LogoutRequest, Participant, RegisterRequest,
    SessionEnterResponse, SessionLifecycleEvent, SessionMode, SessionRole, subjects,
};
use satya_storage::TopicMetricsSnapshot;
use satya_web::render_satya_app_html;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

/// Renders the Leptos SSR HTML application shell at `GET /`.
pub async fn index_handler() -> Html<String> {
    Html(render_satya_app_html())
}

/// Registers a new user account at `POST /api/auth/register`.
pub async fn register_handler(
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

/// Authenticates an existing user account at `POST /api/auth/login`.
pub async fn login_handler(
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

/// Revokes the caller's auth token and disconnects them from any active sessions at `POST /api/auth/logout`.
pub async fn logout_handler(
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

/// Creates a new password-protected session at `POST /api/sessions/create`.
pub async fn create_session_handler(
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

/// Joins an existing password-protected session at `POST /api/sessions/join`.
pub async fn join_session_handler(
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

/// Explicitly leaves an active session at `POST /api/sessions/leave`.
pub async fn leave_session_handler(
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
    /// Metrics snapshots for all Apache Iggy topics.
    pub topics: Vec<TopicMetricsSnapshot>,
}

pub(crate) async fn verify_admin_header(
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

/// Returns per-topic Apache Iggy & Avro storage metrics at `GET /api/storage/metrics` (requires `AccountRole::Admin`).
pub async fn storage_metrics_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_admin_header(&state, &headers).await {
        return err_resp.into_response();
    }
    let topics = state.storage.metrics_snapshot().await;
    (StatusCode::OK, Json(StorageMetricsResponse { topics })).into_response()
}

/// Flushes buffered Avro container batches across all topics at `POST /api/storage/flush` (requires `AccountRole::Admin`).
pub async fn storage_flush_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_admin_header(&state, &headers).await {
        return err_resp.into_response();
    }
    let flushed = state.storage.flush_all_topics().await.unwrap_or_default();
    (StatusCode::OK, Json(json!({ "flushed_files": flushed }))).into_response()
}
