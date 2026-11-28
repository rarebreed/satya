//! HTTP REST handlers for Leptos SSR shell rendering, user authentication (`/api/auth/*`),
//! session creation/joining/leaving (`/api/sessions/*`), and admin storage metrics (`/api/storage/*`).

use crate::email_notifier::OutboundEmail;
use crate::rate_limit::extract_client_ip;
use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse},
};
use satya_rules_book::{build_rules_catalog_response, get_rules_chapter};
use satya_schema::{
    AccountRecoveryRequest, AccountRecoveryResponse, AccountRole, AccountSecurityEvent,
    AccountSecurityEventType, AdminSetUserRoleRequest, AiGmProviderConfig, AuthSuccessResponse,
    CampaignDirectoryEvent, CampaignDirectoryEventType, CreateSessionRequest, JoinSessionRequest,
    LeaveSessionRequest, LifecycleEventType, LoginRequest, LogoutRequest, Participant,
    PasswordResetConfirmRequest, PasswordResetConfirmResponse, RegisterRequest, SessionAccessEvent,
    SessionAccessEventType, SessionEnterResponse, SessionLifecycleEvent, SessionMode, SessionRole,
    UpdateProfileRequest, subjects,
};
use satya_storage::TopicMetricsSnapshot;
use satya_web::render_satya_app_html;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

fn too_many_requests_response(msg: String, retry_secs: u64) -> axum::response::Response {
    let mut resp = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": msg,
            "retry_after_secs": retry_secs
        })),
    )
        .into_response();
    if let Ok(val) = HeaderValue::from_str(&retry_secs.to_string()) {
        resp.headers_mut().insert(header::RETRY_AFTER, val);
    }
    resp
}

fn current_wasm_pkg_version() -> String {
    let wasm_path = std::path::PathBuf::from(crate::resolve_pkg_dir()).join("satya_web_bg.wasm");
    let Ok(meta) = std::fs::metadata(&wasm_path) else {
        return String::new();
    };
    let mtime_secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{mtime_secs}-{}", meta.len())
}

/// Renders the Leptos SSR HTML application shell at `GET /`.
pub async fn index_handler() -> Html<String> {
    let ver = current_wasm_pkg_version();
    Html(render_satya_app_html().replace("__SATYA_PKG_VER__", &ver))
}

/// Registers a new user account at `POST /api/auth/register`.
pub async fn register_handler(
    State(state): State<Arc<SignalingAppState>>,
    peer: Option<ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let ip = extract_client_ip(&headers, peer.map(|c| c.0.ip()));
    let reg_limit = state.storage.config().quotas.max_registrations_per_ip_per_hour;
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record("register_ip", &ip, reg_limit, Duration::from_secs(3600))
        .await
    {
        return too_many_requests_response(
            format!("Registration rate limit exceeded; retry in {retry_secs}s"),
            retry_secs,
        );
    }

    match state
        .auth
        .register_user_with_email(
            &req.username,
            &req.password,
            &req.display_name,
            req.email.as_deref(),
        )
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
    peer: Option<ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let ip = extract_client_ip(&headers, peer.map(|c| c.0.ip()));
    let quotas = &state.storage.config().quotas;
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record(
            "login_ip",
            &ip,
            quotas.max_logins_per_ip_per_min,
            Duration::from_secs(60),
        )
        .await
    {
        return too_many_requests_response(
            format!("Login rate limit exceeded; retry in {retry_secs}s"),
            retry_secs,
        );
    }

    let norm_user = req.username.trim().to_lowercase();
    if !norm_user.is_empty()
        && let Err(retry_secs) = state
            .rate_limiter
            .check_and_record(
                "login_user",
                &norm_user,
                quotas.max_logins_per_username_per_min,
                Duration::from_secs(60),
            )
            .await
    {
        return too_many_requests_response(
            format!("Too many login attempts for this account; retry in {retry_secs}s"),
            retry_secs,
        );
    }

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
    state.disconnect_account_sockets(&user.user_id).await;

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

/// Revokes ALL active auth tokens and join tokens for the authenticated user and closes all sockets (`POST /api/auth/logout-all`).
pub async fn logout_all_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
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

    let disconnected_sessions = state.auth.revoke_all_user_tokens(&user.user_id).await;
    state.disconnect_account_sockets(&user.user_id).await;
    state.disconnect_user_session_sockets(&user.user_id).await;

    let mut disconnected_ids = Vec::new();
    for summary in &disconnected_sessions {
        disconnected_ids.push(summary.session_id.clone());
        state
            .process_session_disconnect(
                summary,
                LifecycleEventType::ParticipantLeft,
                "All sessions revoked by user",
            )
            .await;
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "revoked_all",
            "user_id": user.user_id,
            "disconnected_sessions": disconnected_ids,
        })),
    )
        .into_response()
}

fn sha256_hex(raw: &str) -> String {
    hex::encode(crate::characters::sha256_digest(raw.as_bytes()))
}

fn anonymous_security_actor() -> Participant {
    Participant::human(
        "anonymous",
        "anonymous",
        "Anonymous Requester",
        SessionRole::Observer,
    )
}

struct AccountSecurityAudit<'a> {
    event_type: AccountSecurityEventType,
    actor: Participant,
    identifier_sha256: Option<String>,
    email_sha256: Option<String>,
    client_ip_sha256: Option<String>,
    request_id: Option<String>,
    delivery_channel: Option<String>,
    revoked_session_count: i32,
    reason: Option<&'a str>,
}

async fn emit_account_security_event(state: &SignalingAppState, audit: AccountSecurityAudit<'_>) {
    if let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::ACCOUNT_SECURITY,
        1,
        "account-security",
        SessionMode::Ttrpg,
    ) {
        let ev = AccountSecurityEvent {
            metadata: meta,
            event_type: audit.event_type,
            actor: audit.actor,
            identifier_sha256: audit.identifier_sha256,
            email_sha256: audit.email_sha256,
            client_ip_sha256: audit.client_ip_sha256,
            request_id: audit.request_id,
            delivery_channel: audit.delivery_channel,
            revoked_session_count: audit.revoked_session_count,
            reason: audit.reason.map(str::to_string),
        };
        let _ = state.storage.log_account_security(&ev).await;
    }
}

/// Initiates username reminder & single-use password reset email delivery (`POST /api/auth/recovery/request`).
///
/// Always returns a uniform `200 OK` response for syntactically valid non-empty identifiers to
/// prevent account enumeration regardless of whether a matching account or email exists.
pub async fn request_account_recovery_handler(
    State(state): State<Arc<SignalingAppState>>,
    peer: Option<ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    Json(req): Json<AccountRecoveryRequest>,
) -> impl IntoResponse {
    let ip = extract_client_ip(&headers, peer.map(|c| c.0.ip()));
    let client_ip_sha256 = Some(sha256_hex(&ip));
    let quotas = &state.storage.config().quotas;
    let trimmed = req.identifier.trim();
    let norm_id_opt = (!trimmed.is_empty() && trimmed.len() <= 254)
        .then(|| trimmed.strip_prefix('@').unwrap_or(trimmed).to_lowercase());
    let identifier_sha256 = norm_id_opt.as_deref().map(sha256_hex);

    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record(
            "recovery_req_ip",
            &ip,
            quotas.max_recovery_requests_per_ip_per_hour,
            Duration::from_secs(3600),
        )
        .await
    {
        emit_account_security_event(
            &state,
            AccountSecurityAudit {
                event_type: AccountSecurityEventType::RecoveryRateLimited,
                actor: anonymous_security_actor(),
                identifier_sha256,
                email_sha256: None,
                client_ip_sha256,
                request_id: None,
                delivery_channel: None,
                revoked_session_count: 0,
                reason: Some("per_ip_limit"),
            },
        )
        .await;
        return too_many_requests_response(
            format!("Recovery request rate limit exceeded; retry in {retry_secs}s"),
            retry_secs,
        );
    }

    let Some(norm_id) = norm_id_opt.filter(|_| !trimmed.chars().any(char::is_control)) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Please enter a valid email address or username" })),
        )
            .into_response();
    };

    let id_window_secs = quotas.recovery_email_window_mins.max(1).saturating_mul(60);
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record(
            "recovery_req_id",
            &norm_id,
            quotas.max_recovery_requests_per_identifier_per_hour,
            Duration::from_secs(id_window_secs),
        )
        .await
    {
        emit_account_security_event(
            &state,
            AccountSecurityAudit {
                event_type: AccountSecurityEventType::RecoveryRateLimited,
                actor: anonymous_security_actor(),
                identifier_sha256,
                email_sha256: None,
                client_ip_sha256,
                request_id: None,
                delivery_channel: None,
                revoked_session_count: 0,
                reason: Some("per_identifier_limit"),
            },
        )
        .await;
        return too_many_requests_response(
            format!("Too many recovery requests for this account; retry in {retry_secs}s"),
            retry_secs,
        );
    }

    match state.auth.request_password_reset(trimmed).await {
        Ok(crate::auth::PasswordResetRequestOutcome::Issued(issued)) => {
            let app_url = state
                .storage
                .config()
                .email
                .public_app_url
                .trim_end_matches('/');
            let reset_url = format!("{app_url}/?reset_token={}", issued.raw_token);
            let request_id = format!("rec_{}", uuid::Uuid::now_v7());
            let email_msg = OutboundEmail {
                to_email: issued.to_email,
                subject: "[Satya] Account Recovery & Password Reset".to_string(),
                body_text: format!(
                    "Hello {},\n\n\
                    We received a request to recover your Satya account.\n\n\
                    Your username is: @{}\n\n\
                    To reset your password, open this link (valid for {} minutes, single-use):\n\
                    {}\n\n\
                    Or paste this reset token into the Account Recovery panel:\n\
                    {}\n\n\
                    If you did not request this, you can safely ignore this email; your password has not been changed.\n",
                    issued.account.display_name,
                    issued.account.username,
                    issued.ttl_mins,
                    reset_url,
                    issued.raw_token,
                ),
                request_id: request_id.clone(),
            };
            let delivery_channel = match state.email_notifier.send_email(&email_msg).await {
                Ok(res) if res.provider == "local_outbox" => "LOCAL_OUTBOX",
                Ok(_) => "EMAIL",
                Err(_) => "EMAIL_FAILED",
            };
            let actor = Participant::human(
                &issued.account.user_id,
                &issued.account.username,
                &issued.account.display_name,
                SessionRole::Participant,
            );
            emit_account_security_event(
                &state,
                AccountSecurityAudit {
                    event_type: AccountSecurityEventType::RecoveryRequested,
                    actor,
                    identifier_sha256,
                    email_sha256: Some(issued.email_sha256),
                    client_ip_sha256,
                    request_id: Some(request_id),
                    delivery_channel: Some(delivery_channel.to_string()),
                    revoked_session_count: 0,
                    reason: None,
                },
            )
            .await;
        }
        Ok(crate::auth::PasswordResetRequestOutcome::EmailRateLimited {
            account,
            email_sha256,
            ..
        }) => {
            let actor = Participant::human(
                &account.user_id,
                &account.username,
                &account.display_name,
                SessionRole::Participant,
            );
            emit_account_security_event(
                &state,
                AccountSecurityAudit {
                    event_type: AccountSecurityEventType::RecoveryRateLimited,
                    actor,
                    identifier_sha256,
                    email_sha256: Some(email_sha256),
                    client_ip_sha256,
                    request_id: None,
                    delivery_channel: None,
                    revoked_session_count: 0,
                    reason: Some("per_email_window_limit"),
                },
            )
            .await;
        }
        Ok(crate::auth::PasswordResetRequestOutcome::NoMatch) | Err(_) => {
            emit_account_security_event(
                &state,
                AccountSecurityAudit {
                    event_type: AccountSecurityEventType::RecoveryIgnored,
                    actor: anonymous_security_actor(),
                    identifier_sha256,
                    email_sha256: None,
                    client_ip_sha256,
                    request_id: None,
                    delivery_channel: None,
                    revoked_session_count: 0,
                    reason: Some("no_matching_account_or_email"),
                },
            )
            .await;
        }
    }

    (
        StatusCode::OK,
        Json(AccountRecoveryResponse {
            status: "accepted".to_string(),
            message: "If an account with an email on file matches that email or username, recovery instructions (including your username and a password reset link) have been sent.".to_string(),
        }),
    )
        .into_response()
}

/// Validates a single-use password reset token, updates the user's password hash, and revokes all
/// active auth/join tokens and live WebSocket sessions (`POST /api/auth/recovery/confirm`).
pub async fn confirm_password_reset_handler(
    State(state): State<Arc<SignalingAppState>>,
    peer: Option<ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    Json(req): Json<PasswordResetConfirmRequest>,
) -> impl IntoResponse {
    let ip = extract_client_ip(&headers, peer.map(|c| c.0.ip()));
    let client_ip_sha256 = Some(sha256_hex(&ip));
    let limit = state
        .storage
        .config()
        .quotas
        .max_recovery_confirms_per_ip_per_min;
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record("recovery_confirm_ip", &ip, limit, Duration::from_secs(60))
        .await
    {
        emit_account_security_event(
            &state,
            AccountSecurityAudit {
                event_type: AccountSecurityEventType::RecoveryRateLimited,
                actor: anonymous_security_actor(),
                identifier_sha256: None,
                email_sha256: None,
                client_ip_sha256,
                request_id: None,
                delivery_channel: None,
                revoked_session_count: 0,
                reason: Some("confirm_per_ip_limit"),
            },
        )
        .await;
        return too_many_requests_response(
            format!("Too many password reset attempts; retry in {retry_secs}s"),
            retry_secs,
        );
    }

    match state
        .auth
        .confirm_password_reset(&req.token, &req.new_password)
        .await
    {
        Ok((user, disconnected_sessions)) => {
            state.disconnect_account_sockets(&user.user_id).await;
            state.disconnect_user_session_sockets(&user.user_id).await;
            for summary in &disconnected_sessions {
                state
                    .process_session_disconnect(
                        summary,
                        LifecycleEventType::ParticipantLeft,
                        "Password reset completed",
                    )
                    .await;
            }
            let actor = Participant::human(
                &user.user_id,
                &user.username,
                &user.display_name,
                SessionRole::Participant,
            );
            let email_sha256 = user
                .email
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|em| sha256_hex(&em.to_lowercase()));
            emit_account_security_event(
                &state,
                AccountSecurityAudit {
                    event_type: AccountSecurityEventType::PasswordResetCompleted,
                    actor,
                    identifier_sha256: None,
                    email_sha256,
                    client_ip_sha256,
                    request_id: None,
                    delivery_channel: None,
                    revoked_session_count: disconnected_sessions.len() as i32,
                    reason: Some("password_reset_completed"),
                },
            )
            .await;
            (
                StatusCode::OK,
                Json(PasswordResetConfirmResponse {
                    status: "reset".to_string(),
                    username: user.username.clone(),
                    message: format!(
                        "Password updated for @{}. You can now log in with your new password.",
                        user.username
                    ),
                }),
            )
                .into_response()
        }
        Err(e) => {
            let err_str = e.to_string();
            let reason_code = if err_str.contains("Password must") {
                "password_policy_violation"
            } else {
                "invalid_or_expired_token"
            };
            emit_account_security_event(
                &state,
                AccountSecurityAudit {
                    event_type: AccountSecurityEventType::PasswordResetFailed,
                    actor: anonymous_security_actor(),
                    identifier_sha256: None,
                    email_sha256: None,
                    client_ip_sha256,
                    request_id: None,
                    delivery_channel: None,
                    revoked_session_count: 0,
                    reason: Some(reason_code),
                },
            )
            .await;
            (StatusCode::BAD_REQUEST, Json(json!({ "error": err_str }))).into_response()
        }
    }
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

    if !user.account_role.can_host_sessions() {
        let actor = Participant::human(&user.user_id, &user.username, &user.display_name, req.role);
        if let Ok(meta) = state.storage.registry().build_metadata(
            state.next_seq(),
            subjects::SESSION_ACCESS,
            1,
            &req.session_id,
            req.session_mode,
        ) {
            let ev = SessionAccessEvent {
                metadata: meta,
                event_type: SessionAccessEventType::QuotaRejected,
                actor,
                target_participant: None,
                request_id: None,
                campaign_id: req.campaign_id.clone(),
                requested_role: Some(req.role),
                delivery_channel: None,
                reason: Some("USER account role is not authorized to host sessions".to_string()),
            };
            let _ = state.storage.log_session_access(&ev).await;
        }
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "GM or Admin account role required to create sessions"
            })),
        )
            .into_response();
    }

    let create_limit = state.storage.config().quotas.max_creates_per_user_per_hour;
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record(
            "session_create",
            &user.user_id,
            create_limit,
            Duration::from_secs(3600),
        )
        .await
    {
        let actor = Participant::human(&user.user_id, &user.username, &user.display_name, req.role);
        if let Ok(meta) = state.storage.registry().build_metadata(
            state.next_seq(),
            subjects::SESSION_ACCESS,
            1,
            &req.session_id,
            req.session_mode,
        ) {
            let ev = SessionAccessEvent {
                metadata: meta,
                event_type: SessionAccessEventType::RateLimitRejected,
                actor,
                target_participant: None,
                request_id: None,
                campaign_id: req.campaign_id.clone(),
                requested_role: Some(req.role),
                delivery_channel: None,
                reason: Some(format!(
                    "Session creation rate limit exceeded (retry_after_secs={retry_secs})"
                )),
            };
            let _ = state.storage.log_session_access(&ev).await;
        }
        return too_many_requests_response(
            format!("Session creation rate limit exceeded; retry in {retry_secs}s"),
            retry_secs,
        );
    }

    let params = crate::auth::CreateSessionParams {
        session_id: &req.session_id,
        session_password: &req.session_password,
        session_mode: req.session_mode,
        role: req.role,
        display_name_override: req.display_name,
        enable_ai_gm: req.enable_ai_gm,
        campaign_id: req.campaign_id.as_deref(),
        title: req.title.clone(),
        is_discoverable: req.is_discoverable,
        allow_join_requests: req.allow_join_requests,
    };

    match state.auth.create_session_with_params(&user, params).await {
        Ok((join_token, self_participant, mut roster)) => {
            if req.session_mode == SessionMode::Ttrpg && req.enable_ai_gm {
                let mut ai_cfg = req.ai_gm_config.unwrap_or_else(|| {
                    let defaults = &state.storage.config().ai_gm;
                    AiGmProviderConfig {
                        provider: defaults.default_provider.clone(),
                        model_id: defaults.reasoning_model.clone(),
                        vision_model_id: Some(defaults.vision_model.clone()),
                        temperature: defaults.temperature,
                        persona_id: defaults.persona_id.clone(),
                        api_key: None,
                        base_url: Some(defaults.ollama_base_url.clone()),
                        clear_api_key: false,
                    }
                });
                let ai_defaults = &state.storage.config().ai_gm;
                if let Some(ref raw_key) = ai_cfg.api_key
                    && !raw_key.trim().is_empty()
                {
                    if let Ok(clean_key) = crate::security::validate_api_key_header_safe(raw_key) {
                        ai_cfg.api_key = Some(clean_key);
                    } else {
                        ai_cfg.api_key = None;
                    }
                }
                if let Some(ref raw_url) = ai_cfg.base_url
                    && !raw_url.trim().is_empty()
                {
                    match crate::security::validate_provider_base_url(
                        &ai_cfg.provider,
                        raw_url,
                        &ai_defaults.ollama_base_url,
                        &ai_defaults.allowed_provider_base_urls,
                    ) {
                        Ok(safe_url) => ai_cfg.base_url = Some(safe_url),
                        Err(_) => ai_cfg.base_url = None,
                    }
                }
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

            if req.campaign_id.is_some()
                && let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::CAMPAIGN_DIRECTORY,
                    1,
                    &req.session_id,
                    req.session_mode,
                )
            {
                let ev = CampaignDirectoryEvent {
                    metadata: meta,
                    event_type: CampaignDirectoryEventType::SessionAssignedToCampaign,
                    actor: self_participant.clone(),
                    owner_user_id: Some(user.user_id.clone()),
                    campaign_id: req.campaign_id.clone(),
                    campaign_title: req.title.clone(),
                    campaign_slug: None,
                    search_query_sha256: None,
                    result_count: 1,
                    discoverable: req.is_discoverable,
                };
                let _ = state.storage.log_campaign_directory(&ev).await;
            }

            let history = state
                .auth
                .build_session_history_snapshot(&req.session_id, &self_participant)
                .await;

            (
                StatusCode::OK,
                Json(json!(SessionEnterResponse {
                    session_id: req.session_id,
                    session_mode: req.session_mode,
                    join_token,
                    self_participant,
                    roster,
                    history: Some(history),
                })),
            )
                .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.starts_with("Quota exceeded:") {
                let actor =
                    Participant::human(&user.user_id, &user.username, &user.display_name, req.role);
                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::SESSION_ACCESS,
                    1,
                    &req.session_id,
                    req.session_mode,
                ) {
                    let ev = SessionAccessEvent {
                        metadata: meta,
                        event_type: SessionAccessEventType::QuotaRejected,
                        actor,
                        target_participant: None,
                        request_id: None,
                        campaign_id: req.campaign_id.clone(),
                        requested_role: Some(req.role),
                        delivery_channel: None,
                        reason: Some(msg.clone()),
                    };
                    let _ = state.storage.log_session_access(&ev).await;
                }
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::BAD_REQUEST
            };
            (status, Json(json!({ "error": msg }))).into_response()
        }
    }
}

/// Joins an existing session at `POST /api/sessions/join` via password OR user-bound `approval_token`.
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

    let join_limit = state
        .storage
        .config()
        .quotas
        .max_session_joins_per_user_per_min;
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record(
            "session_join",
            &user.user_id,
            join_limit,
            Duration::from_secs(60),
        )
        .await
    {
        return too_many_requests_response(
            format!("Session join rate limit exceeded; retry in {retry_secs}s"),
            retry_secs,
        );
    }
    let join_sess_key = format!("{}:{}", user.user_id, req.session_id.trim());
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record(
            "session_join_pair",
            &join_sess_key,
            join_limit.min(10),
            Duration::from_secs(60),
        )
        .await
    {
        return too_many_requests_response(
            format!("Session join rate limit exceeded for this room; retry in {retry_secs}s"),
            retry_secs,
        );
    }

    match state
        .auth
        .join_session_with_approval(
            &user,
            &req.session_id,
            &req.session_password,
            req.approval_token.as_deref(),
            req.role,
            req.display_name,
        )
        .await
    {
        Ok((join_token, session_mode, self_participant, roster, consumed_req_id)) => {
            if let Some(req_id) = consumed_req_id
                && let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::SESSION_ACCESS,
                    1,
                    &req.session_id,
                    session_mode,
                )
            {
                let acc_ev = SessionAccessEvent {
                    metadata: meta,
                    event_type: SessionAccessEventType::JoinRequestConsumed,
                    actor: self_participant.clone(),
                    target_participant: None,
                    request_id: Some(req_id),
                    campaign_id: None,
                    requested_role: Some(req.role),
                    delivery_channel: Some("APPROVAL_TOKEN".to_string()),
                    reason: Some("Approved join token consumed".to_string()),
                };
                let _ = state.storage.log_session_access(&acc_ev).await;
            }

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
                    reason: Some(if req.approval_token.as_deref().is_some_and(|s| !s.trim().is_empty()) {
                        "Authenticated user joined with GM approval grant".to_string()
                    } else {
                        "Authenticated user joined with valid session password".to_string()
                    }),
                };
                let _ = state.storage.log_session_lifecycle(&ev).await;
            }

            let history = state
                .auth
                .build_session_history_snapshot(&req.session_id, &self_participant)
                .await;

            (
                StatusCode::OK,
                Json(json!(SessionEnterResponse {
                    session_id: req.session_id,
                    session_mode,
                    join_token,
                    self_participant,
                    roster,
                    history: Some(history),
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
                "remaining_roster": Vec::<Participant>::new(),
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

/// Optional query parameters for `GET /api/rules`.
#[derive(Debug, Default, Deserialize)]
pub struct RulesCatalogQuery {
    /// Optional keyword search query across all chapters in `docs/ttrpg/`.
    pub q: Option<String>,
    /// Maximum number of ranked search hits to return (default `8`).
    pub limit: Option<usize>,
}

/// Returns the canonical Satya TTRPG Rules Book chapter catalog and optional `?q=` search hits at `GET /api/rules`.
pub async fn list_rules_handler(Query(params): Query<RulesCatalogQuery>) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(8).clamp(1, 25);
    let catalog = build_rules_catalog_response(params.q.as_deref(), limit);
    (StatusCode::OK, Json(catalog)).into_response()
}

/// Returns a single Satya TTRPG Rules Book chapter (metadata, raw Markdown, and rendered HTML) at `GET /api/rules/:chapter_id`.
pub async fn get_rules_chapter_handler(Path(chapter_id): Path<String>) -> impl IntoResponse {
    match get_rules_chapter(&chapter_id) {
        Some(doc) => (StatusCode::OK, Json(json!(doc))).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Rules chapter '{}' not found", chapter_id)
            })),
        )
            .into_response(),
    }
}

/// Updates the caller's display name and/or private email at `PATCH /api/auth/profile`.
pub async fn update_profile_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    let clean = auth_hdr.strip_prefix("Bearer ").unwrap_or(auth_hdr).trim();
    let Some(user) = state.auth.verify_auth_token(clean).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };

    match state
        .auth
        .update_user_profile(&user.user_id, req.display_name.as_deref(), req.email.as_deref())
        .await
    {
        Ok(updated) => (
            StatusCode::OK,
            Json(json!(AuthSuccessResponse {
                auth_token: clean.to_string(),
                user_id: updated.user_id,
                username: updated.username,
                display_name: updated.display_name,
                account_role: updated.account_role,
                email: updated.email,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Promotes or demotes a user's [`AccountRole`] (`USER` / `GM` / `ADMIN`) at `POST /api/admin/users/role`
/// (requires [`AccountRole::Admin`]).
pub async fn admin_set_user_role_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<AdminSetUserRoleRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_admin_header(&state, &headers).await {
        return err_resp.into_response();
    }
    match state
        .auth
        .set_user_account_role(&req.username, req.account_role)
        .await
    {
        Ok(updated) => (
            StatusCode::OK,
            Json(json!({
                "status": "updated",
                "user_id": updated.user_id,
                "username": updated.username,
                "account_role": updated.account_role,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
