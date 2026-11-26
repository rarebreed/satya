//! GM-owned Campaign management, privacy-preserving GM directory search,
//! Request-to-Join lifecycle (live WebSocket + offline email + user-bound approval grants),
//! quota endpoints, and account-level `/ws/notifications`.

use crate::auth::{ProvisionCampaignSessionParams, UserAccount};
use crate::characters::sha256_digest;
use crate::email_notifier::OutboundEmail;
use crate::session_persistence::{
    session_mode_from_db, session_mode_to_db, session_role_from_db, session_role_to_db,
};
use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{
        Path, Query, State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use satya_schema::{
    AccountRole, AccountWsPacket, CampaignDirectoryEvent, CampaignDirectoryEventType,
    CampaignListResponse, CampaignSummary, CreateCampaignRequest, CreateCampaignSessionRequestBody,
    CreateJoinRequestBody, DecideJoinRequestBody, DirectoryCampaignEntry, DirectoryGmEntry,
    DirectoryMembershipStatus, DirectorySearchResponse, DirectorySessionEntry, JoinRequestKind,
    JoinRequestRecord, JoinRequestStatus, JoinRequestsListResponse, Participant, ServerWsPacket,
    SessionAccessEvent, SessionAccessEventType, SessionMode, SessionRole, UpdateCampaignRequest,
    subjects,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

async fn verify_bearer_user(
    state: &SignalingAppState,
    headers: &HeaderMap,
) -> Result<UserAccount, (StatusCode, Json<serde_json::Value>)> {
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
    Ok(user)
}

fn actor_from_user(user: &UserAccount, role: SessionRole) -> Participant {
    Participant::human(&user.user_id, &user.username, &user.display_name, role)
}

fn slugify_campaign_title(raw: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in raw.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
        if slug.len() >= 48 {
            break;
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "campaign".to_string()
    } else {
        trimmed
    }
}

fn hash_query_sha256(query: &str) -> String {
    let normalized = query.trim().to_lowercase();
    hex::encode(sha256_digest(normalized.as_bytes()))
}

fn join_request_status_to_db(status: JoinRequestStatus) -> &'static str {
    status.as_str()
}

fn join_request_status_from_db(raw: &str) -> JoinRequestStatus {
    JoinRequestStatus::from_code_str(raw).unwrap_or(JoinRequestStatus::Pending)
}

struct CampaignDirAudit<'a> {
    event_type: CampaignDirectoryEventType,
    actor: Participant,
    owner_user_id: Option<String>,
    campaign_id: Option<String>,
    campaign_title: Option<String>,
    campaign_slug: Option<String>,
    search_query_sha256: Option<String>,
    result_count: i32,
    discoverable: bool,
    scope_session_id: &'a str,
}

async fn emit_campaign_directory_event(state: &SignalingAppState, audit: CampaignDirAudit<'_>) {
    if let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::CAMPAIGN_DIRECTORY,
        1,
        audit.scope_session_id,
        SessionMode::Ttrpg,
    ) {
        let ev = CampaignDirectoryEvent {
            metadata: meta,
            event_type: audit.event_type,
            actor: audit.actor,
            owner_user_id: audit.owner_user_id,
            campaign_id: audit.campaign_id,
            campaign_title: audit.campaign_title,
            campaign_slug: audit.campaign_slug,
            search_query_sha256: audit.search_query_sha256,
            result_count: audit.result_count,
            discoverable: audit.discoverable,
        };
        let _ = state.storage.log_campaign_directory(&ev).await;
    }
}

struct SessionAccessAudit<'a> {
    event_type: SessionAccessEventType,
    actor: Participant,
    target_participant: Option<Participant>,
    session_id: &'a str,
    session_mode: SessionMode,
    request_id: Option<String>,
    campaign_id: Option<String>,
    requested_role: Option<SessionRole>,
    delivery_channel: Option<String>,
    reason: Option<String>,
}

async fn emit_session_access_event(state: &SignalingAppState, audit: SessionAccessAudit<'_>) {
    if let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::SESSION_ACCESS,
        1,
        audit.session_id,
        audit.session_mode,
    ) {
        let ev = SessionAccessEvent {
            metadata: meta,
            event_type: audit.event_type,
            actor: audit.actor,
            target_participant: audit.target_participant,
            request_id: audit.request_id,
            campaign_id: audit.campaign_id,
            requested_role: audit.requested_role,
            delivery_channel: audit.delivery_channel,
            reason: audit.reason,
        };
        let _ = state.storage.log_session_access(&ev).await;
    }
}

async fn expire_stale_join_requests(state: &SignalingAppState, now_us: i64) {
    let _ = sqlx::query(
        r#"
        UPDATE session_join_requests
        SET status = 'EXPIRED', updated_at_us = ?1
        WHERE status IN ('PENDING', 'APPROVED') AND expires_at_us <= ?1
        "#,
    )
    .bind(now_us)
    .execute(state.auth.db_pool())
    .await;
}

async fn count_pending_incoming(state: &SignalingAppState, owner_user_id: &str) -> usize {
    let now_us = Utc::now().timestamp_micros();
    expire_stale_join_requests(state, now_us).await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session_join_requests WHERE owner_user_id = ?1 AND status = 'PENDING'",
    )
    .bind(owner_user_id)
    .fetch_one(state.auth.db_pool())
    .await
    .unwrap_or(0);
    count.max(0) as usize
}

/// Returns the caller's live quota usage and configured limits at `GET /api/auth/quotas`.
pub async fn get_account_quotas_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let usage = state.auth.compute_account_quota_usage(&user).await;
    (StatusCode::OK, Json(json!(usage))).into_response()
}

/// Creates a GM-owned campaign at `POST /api/campaigns` (requires `AccountRole::Gm` or `Admin`).
pub async fn create_campaign_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateCampaignRequest>,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let actor = actor_from_user(&user, SessionRole::Gm);
    if !user.account_role.can_host_sessions() {
        emit_session_access_event(
            &state,
            SessionAccessAudit {
                event_type: SessionAccessEventType::QuotaRejected,
                actor,
                target_participant: None,
                session_id: "campaigns",
                session_mode: SessionMode::Ttrpg,
                request_id: None,
                campaign_id: None,
                requested_role: None,
                delivery_channel: None,
                reason: Some("USER account role cannot create campaigns".to_string()),
            },
        )
        .await;
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "GM or Admin account role required to create campaigns" })),
        )
            .into_response();
    }

    let create_limit = state.storage.config().quotas.max_creates_per_user_per_hour;
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record(
            "campaign_create",
            &user.user_id,
            create_limit,
            Duration::from_secs(3600),
        )
        .await
    {
        emit_session_access_event(
            &state,
            SessionAccessAudit {
                event_type: SessionAccessEventType::RateLimitRejected,
                actor,
                target_participant: None,
                session_id: "campaigns",
                session_mode: SessionMode::Ttrpg,
                request_id: None,
                campaign_id: None,
                requested_role: None,
                delivery_channel: None,
                reason: Some(format!("Campaign creation rate limit (retry_after={retry_secs}s)")),
            },
        )
        .await;
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": format!("Campaign creation rate limit exceeded; retry in {retry_secs}s"),
                "retry_after_secs": retry_secs
            })),
        )
            .into_response();
    }

    let title = req.title.trim().to_string();
    if title.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Campaign title cannot be empty" })),
        )
            .into_response();
    }

    let quota = state.auth.compute_account_quota_usage(&user).await;
    if quota.campaigns_used >= quota.campaigns_limit {
        let msg = format!(
            "Quota exceeded: maximum {} active campaigns per GM reached",
            quota.campaigns_limit
        );
        emit_session_access_event(
            &state,
            SessionAccessAudit {
                event_type: SessionAccessEventType::QuotaRejected,
                actor,
                target_participant: None,
                session_id: "campaigns",
                session_mode: SessionMode::Ttrpg,
                request_id: None,
                campaign_id: None,
                requested_role: None,
                delivery_channel: None,
                reason: Some(msg.clone()),
            },
        )
        .await;
        return (StatusCode::TOO_MANY_REQUESTS, Json(json!({ "error": msg }))).into_response();
    }

    let base_slug = slugify_campaign_title(req.slug.as_deref().unwrap_or(&title));
    let mut slug = base_slug.clone();
    for suffix in 2..=50 {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM campaigns WHERE owner_user_id = ?1 AND slug = ?2 COLLATE NOCASE",
        )
        .bind(&user.user_id)
        .bind(&slug)
        .fetch_one(state.auth.db_pool())
        .await
        .unwrap_or(0);
        if exists == 0 {
            break;
        }
        slug = format!("{base_slug}-{suffix}");
    }

    let campaign_id = format!("camp_{}", Uuid::now_v7());
    let setting_pack = req
        .setting_pack
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&state.campaign_pack)
        .to_string();
    let description = req.description.trim().to_string();
    let now_us = Utc::now().timestamp_micros();

    let insert_res = sqlx::query(
        r#"
        INSERT INTO campaigns (
            campaign_id, owner_user_id, slug, title, description,
            setting_pack, is_discoverable, is_archived, created_at_us, updated_at_us
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9)
        "#,
    )
    .bind(&campaign_id)
    .bind(&user.user_id)
    .bind(&slug)
    .bind(&title)
    .bind(&description)
    .bind(&setting_pack)
    .bind(if req.is_discoverable { 1_i64 } else { 0_i64 })
    .bind(now_us)
    .bind(now_us)
    .execute(state.auth.db_pool())
    .await;

    if let Err(e) = insert_res {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Failed to create campaign: {e}") })),
        )
            .into_response();
    }

    let summary = CampaignSummary {
        campaign_id: campaign_id.clone(),
        owner_user_id: user.user_id.clone(),
        slug: slug.clone(),
        title: title.clone(),
        description,
        setting_pack,
        is_discoverable: req.is_discoverable,
        is_archived: false,
        session_count: 0,
        created_at_us: now_us,
        updated_at_us: now_us,
    };

    emit_campaign_directory_event(
        &state,
        CampaignDirAudit {
            event_type: CampaignDirectoryEventType::CampaignCreated,
            actor,
            owner_user_id: Some(user.user_id.clone()),
            campaign_id: Some(campaign_id),
            campaign_title: Some(title),
            campaign_slug: Some(slug),
            search_query_sha256: None,
            result_count: 1,
            discoverable: req.is_discoverable,
            scope_session_id: "directory",
        },
    )
    .await;

    (StatusCode::OK, Json(json!(summary))).into_response()
}

/// Lists campaigns owned by the authenticated user at `GET /api/campaigns`.
pub async fn list_my_campaigns_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    let rows = sqlx::query(
        r#"
        SELECT c.campaign_id, c.owner_user_id, c.slug, c.title, c.description,
               c.setting_pack, c.is_discoverable, c.is_archived, c.created_at_us, c.updated_at_us,
               (SELECT COUNT(*) FROM saved_sessions s WHERE s.campaign_id = c.campaign_id) AS session_count
        FROM campaigns c
        WHERE c.owner_user_id = ?1
        ORDER BY c.is_archived ASC, c.updated_at_us DESC
        "#,
    )
    .bind(&user.user_id)
    .fetch_all(state.auth.db_pool())
    .await
    .unwrap_or_default();

    let mut campaigns = Vec::with_capacity(rows.len());
    for row in rows {
        let Ok(campaign_id) = row.try_get::<String, _>("campaign_id") else {
            continue;
        };
        campaigns.push(CampaignSummary {
            campaign_id,
            owner_user_id: row.try_get("owner_user_id").unwrap_or_default(),
            slug: row.try_get("slug").unwrap_or_default(),
            title: row.try_get("title").unwrap_or_default(),
            description: row.try_get("description").unwrap_or_default(),
            setting_pack: row
                .try_get("setting_pack")
                .unwrap_or_else(|_| "awakened".to_string()),
            is_discoverable: row.try_get::<i64, _>("is_discoverable").unwrap_or(1) != 0,
            is_archived: row.try_get::<i64, _>("is_archived").unwrap_or(0) != 0,
            session_count: row.try_get::<i64, _>("session_count").unwrap_or(0).max(0) as usize,
            created_at_us: row.try_get("created_at_us").unwrap_or(0),
            updated_at_us: row.try_get("updated_at_us").unwrap_or(0),
        });
    }

    emit_campaign_directory_event(
        &state,
        CampaignDirAudit {
            event_type: CampaignDirectoryEventType::CampaignsListed,
            actor: actor_from_user(&user, SessionRole::Gm),
            owner_user_id: Some(user.user_id.clone()),
            campaign_id: None,
            campaign_title: None,
            campaign_slug: None,
            search_query_sha256: None,
            result_count: campaigns.len() as i32,
            discoverable: true,
            scope_session_id: "directory",
        },
    )
    .await;

    (
        StatusCode::OK,
        Json(json!(CampaignListResponse { campaigns })),
    )
        .into_response()
}

/// Updates campaign metadata or visibility/archive state at `PATCH /api/campaigns/:campaign_id`.
pub async fn update_campaign_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(campaign_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<UpdateCampaignRequest>,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let cid = campaign_id.trim().to_string();

    let Some(row) = sqlx::query(
        "SELECT owner_user_id, slug, title, description, setting_pack, is_discoverable, is_archived, created_at_us FROM campaigns WHERE campaign_id = ?1",
    )
    .bind(&cid)
    .fetch_optional(state.auth.db_pool())
    .await
    .ok()
    .flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Campaign '{cid}' not found") })),
        )
            .into_response();
    };

    let owner_user_id: String = row.try_get("owner_user_id").unwrap_or_default();
    if owner_user_id != user.user_id && user.account_role != AccountRole::Admin {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the campaign owner or Admin can modify this campaign" })),
        )
            .into_response();
    }

    let slug: String = row.try_get("slug").unwrap_or_default();
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| row.try_get("title").unwrap_or_default());
    let description = req
        .description
        .as_deref()
        .map(str::trim)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| row.try_get("description").unwrap_or_default());
    let setting_pack: String = row.try_get("setting_pack").unwrap_or_default();
    let is_discoverable = req
        .is_discoverable
        .unwrap_or_else(|| row.try_get::<i64, _>("is_discoverable").unwrap_or(1) != 0);
    let was_archived = row.try_get::<i64, _>("is_archived").unwrap_or(0) != 0;
    let is_archived = req.is_archived.unwrap_or(was_archived);
    let created_at_us: i64 = row.try_get("created_at_us").unwrap_or(0);
    let now_us = Utc::now().timestamp_micros();

    let _ = sqlx::query(
        r#"
        UPDATE campaigns
        SET title = ?1, description = ?2, is_discoverable = ?3, is_archived = ?4, updated_at_us = ?5
        WHERE campaign_id = ?6
        "#,
    )
    .bind(&title)
    .bind(&description)
    .bind(if is_discoverable { 1_i64 } else { 0_i64 })
    .bind(if is_archived { 1_i64 } else { 0_i64 })
    .bind(now_us)
    .bind(&cid)
    .execute(state.auth.db_pool())
    .await;

    let session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM saved_sessions WHERE campaign_id = ?1")
            .bind(&cid)
            .fetch_one(state.auth.db_pool())
            .await
            .unwrap_or(0);

    let ev_type = if !was_archived && is_archived {
        CampaignDirectoryEventType::CampaignArchived
    } else {
        CampaignDirectoryEventType::CampaignUpdated
    };
    emit_campaign_directory_event(
        &state,
        CampaignDirAudit {
            event_type: ev_type,
            actor: actor_from_user(&user, SessionRole::Gm),
            owner_user_id: Some(owner_user_id.clone()),
            campaign_id: Some(cid.clone()),
            campaign_title: Some(title.clone()),
            campaign_slug: Some(slug.clone()),
            search_query_sha256: None,
            result_count: session_count as i32,
            discoverable: is_discoverable,
            scope_session_id: "directory",
        },
    )
    .await;

    (
        StatusCode::OK,
        Json(json!(CampaignSummary {
            campaign_id: cid,
            owner_user_id,
            slug,
            title,
            description,
            setting_pack,
            is_discoverable,
            is_archived,
            session_count: session_count.max(0) as usize,
            created_at_us,
            updated_at_us: now_us,
        })),
    )
        .into_response()
}

/// Query parameters for `DELETE /api/campaigns/:campaign_id`.
#[derive(Debug, Default, Deserialize)]
pub struct DeleteCampaignQuery {
    /// When `true`, deletes child sessions belonging to the campaign as well.
    #[serde(default)]
    pub force: bool,
}

/// Deletes a GM-owned campaign (and optionally its sessions when `?force=true`) at `DELETE /api/campaigns/:campaign_id`.
pub async fn delete_campaign_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(campaign_id): Path<String>,
    Query(params): Query<DeleteCampaignQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let cid = campaign_id.trim().to_string();

    let Some(row) =
        sqlx::query("SELECT owner_user_id, slug, title FROM campaigns WHERE campaign_id = ?1")
            .bind(&cid)
            .fetch_optional(state.auth.db_pool())
            .await
            .ok()
            .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Campaign '{cid}' not found") })),
        )
            .into_response();
    };
    let owner_user_id: String = row.try_get("owner_user_id").unwrap_or_default();
    let slug: String = row.try_get("slug").unwrap_or_default();
    let title: String = row.try_get("title").unwrap_or_default();

    if owner_user_id != user.user_id && user.account_role != AccountRole::Admin {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the campaign owner or Admin can delete this campaign" })),
        )
            .into_response();
    }

    let session_ids: Vec<String> =
        sqlx::query_scalar("SELECT session_id FROM saved_sessions WHERE campaign_id = ?1")
            .bind(&cid)
            .fetch_all(state.auth.db_pool())
            .await
            .unwrap_or_default();

    if !session_ids.is_empty() && !params.force {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "error": format!(
                    "Campaign '{title}' still contains {} session(s); delete sessions first or pass ?force=true",
                    session_ids.len()
                )
            })),
        )
            .into_response();
    }

    for sid in &session_ids {
        purge_session_records(&state, sid).await;
    }
    let _ = sqlx::query("DELETE FROM campaigns WHERE campaign_id = ?1")
        .bind(&cid)
        .execute(state.auth.db_pool())
        .await;

    emit_campaign_directory_event(
        &state,
        CampaignDirAudit {
            event_type: CampaignDirectoryEventType::CampaignDeleted,
            actor: actor_from_user(&user, SessionRole::Gm),
            owner_user_id: Some(owner_user_id),
            campaign_id: Some(cid.clone()),
            campaign_title: Some(title),
            campaign_slug: Some(slug),
            search_query_sha256: None,
            result_count: session_ids.len() as i32,
            discoverable: false,
            scope_session_id: "directory",
        },
    )
    .await;

    (
        StatusCode::OK,
        Json(json!({
            "status": "deleted",
            "campaign_id": cid,
            "deleted_sessions": session_ids.len(),
        })),
    )
        .into_response()
}

async fn purge_session_records(state: &SignalingAppState, sid: &str) {
    state.auth.sessions.write().await.remove(sid);
    state
        .auth
        .join_tokens
        .write()
        .await
        .retain(|_, claims| claims.session_id != sid);
    state.active_sockets.write().await.remove(sid);
    for sql in [
        "DELETE FROM session_join_requests WHERE session_id = ?1",
        "DELETE FROM session_participants WHERE session_id = ?1",
        "DELETE FROM session_event_history WHERE session_id = ?1",
        "DELETE FROM session_combat_state WHERE session_id = ?1",
        "DELETE FROM session_world_ledger WHERE session_id = ?1",
        "DELETE FROM session_ai_players WHERE session_id = ?1",
        "DELETE FROM saved_sessions WHERE session_id = ?1",
    ] {
        let _ = sqlx::query(sql)
            .bind(sid)
            .execute(state.auth.db_pool())
            .await;
    }
}

/// Deletes an owned saved session to reclaim GM quota at `DELETE /api/sessions/:session_id`.
pub async fn delete_session_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let sid = session_id.trim().to_string();

    let Some(row) = sqlx::query(
        "SELECT COALESCE(NULLIF(owner_user_id, ''), host_user_id) AS owner_user_id, campaign_id, title FROM saved_sessions WHERE session_id = ?1",
    )
    .bind(&sid)
    .fetch_optional(state.auth.db_pool())
    .await
    .ok()
    .flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{sid}' not found") })),
        )
            .into_response();
    };
    let owner_user_id: String = row.try_get("owner_user_id").unwrap_or_default();
    let campaign_id: Option<String> = row.try_get("campaign_id").ok().flatten();
    let title: Option<String> = row.try_get("title").ok().flatten();

    if owner_user_id != user.user_id && user.account_role != AccountRole::Admin {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the session owner or Admin can delete this session" })),
        )
            .into_response();
    }

    purge_session_records(&state, &sid).await;

    emit_campaign_directory_event(
        &state,
        CampaignDirAudit {
            event_type: CampaignDirectoryEventType::SessionDeleted,
            actor: actor_from_user(&user, SessionRole::Gm),
            owner_user_id: Some(owner_user_id),
            campaign_id,
            campaign_title: title,
            campaign_slug: None,
            search_query_sha256: None,
            result_count: 1,
            discoverable: false,
            scope_session_id: &sid,
        },
    )
    .await;

    (
        StatusCode::OK,
        Json(json!({ "status": "deleted", "session_id": sid })),
    )
        .into_response()
}

/// Query parameters for `GET /api/directory/search`.
#[derive(Debug, Default, Deserialize)]
pub struct DirectorySearchQuery {
    /// GM's exact email address OR `@username` handle (exact or prefix).
    #[serde(default)]
    pub q: String,
}

/// Privacy-preserving user/campaign directory search at `GET /api/directory/search?q=...`.
///
/// Matches any user account by exact email (`COLLATE NOCASE`) or `@username` handle,
/// returning only discoverable campaigns and sessions **without ever leaking the owner's email**.
pub async fn search_gm_directory_handler(
    State(state): State<Arc<SignalingAppState>>,
    Query(params): Query<DirectorySearchQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    let search_limit = state
        .storage
        .config()
        .quotas
        .max_directory_searches_per_user_per_min;
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record(
            "directory_search",
            &user.user_id,
            search_limit,
            Duration::from_secs(60),
        )
        .await
    {
        emit_session_access_event(
            &state,
            SessionAccessAudit {
                event_type: SessionAccessEventType::RateLimitRejected,
                actor: actor_from_user(&user, SessionRole::Player),
                target_participant: None,
                session_id: "directory",
                session_mode: SessionMode::Ttrpg,
                request_id: None,
                campaign_id: None,
                requested_role: None,
                delivery_channel: None,
                reason: Some(format!("Directory search rate limit (retry_after={retry_secs}s)")),
            },
        )
        .await;
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": format!("Directory search rate limit exceeded; retry in {retry_secs}s"),
                "retry_after_secs": retry_secs
            })),
        )
            .into_response();
    }

    let now_us = Utc::now().timestamp_micros();
    expire_stale_join_requests(&state, now_us).await;

    let raw_query = params.q.trim().to_string();
    if raw_query.is_empty() {
        return (
            StatusCode::OK,
            Json(json!(DirectorySearchResponse {
                query: raw_query,
                gms: Vec::new(),
            })),
        )
            .into_response();
    }

    let is_email_query = raw_query.contains('@') && !raw_query.starts_with('@');
    let gm_rows = if is_email_query {
        sqlx::query(
            r#"
            SELECT user_id, username, display_name
            FROM users
            WHERE email = ?1 COLLATE NOCASE
            LIMIT 5
            "#,
        )
        .bind(raw_query.to_lowercase())
        .fetch_all(state.auth.db_pool())
        .await
        .unwrap_or_default()
    } else {
        let handle = raw_query.strip_prefix('@').unwrap_or(&raw_query).trim().to_lowercase();
        if handle.is_empty() {
            Vec::new()
        } else {
            let prefix_pat = format!("{handle}%");
            sqlx::query(
                r#"
                SELECT user_id, username, display_name
                FROM users
                WHERE username = ?1 COLLATE NOCASE OR username LIKE ?2 COLLATE NOCASE
                ORDER BY CASE WHEN username = ?1 COLLATE NOCASE THEN 0 ELSE 1 END, username ASC
                LIMIT 10
                "#,
            )
            .bind(&handle)
            .bind(&prefix_pat)
            .fetch_all(state.auth.db_pool())
            .await
            .unwrap_or_default()
        }
    };

    // Load caller's existing memberships & latest join requests per session
    let member_sids: Vec<String> = sqlx::query_scalar(
        "SELECT session_id FROM session_participants WHERE user_id = ?1",
    )
    .bind(&user.user_id)
    .fetch_all(state.auth.db_pool())
    .await
    .unwrap_or_default();

    let caller_reqs = sqlx::query(
        r#"
        SELECT session_id, request_id, status, approval_token
        FROM session_join_requests
        WHERE requester_user_id = ?1
        ORDER BY updated_at_us DESC
        "#,
    )
    .bind(&user.user_id)
    .fetch_all(state.auth.db_pool())
    .await
    .unwrap_or_default();

    let mut req_by_sid: HashMap<String, (String, JoinRequestStatus, Option<String>)> =
        HashMap::new();
    for r in caller_reqs {
        let Ok(sid) = r.try_get::<String, _>("session_id") else {
            continue;
        };
        if req_by_sid.contains_key(&sid) {
            continue;
        }
        let req_id: String = r.try_get("request_id").unwrap_or_default();
        let status_str: String = r.try_get("status").unwrap_or_default();
        let status = join_request_status_from_db(&status_str);
        let token = if status == JoinRequestStatus::Approved {
            r.try_get::<Option<String>, _>("approval_token")
                .ok()
                .flatten()
        } else {
            None
        };
        req_by_sid.insert(sid, (req_id, status, token));
    }

    let active_guard = state.auth.sessions.read().await;
    let mut gms = Vec::with_capacity(gm_rows.len());
    let mut total_sessions = 0usize;

    for gm_row in gm_rows {
        let Ok(gm_uid) = gm_row.try_get::<String, _>("user_id") else {
            continue;
        };
        let gm_username: String = gm_row.try_get("username").unwrap_or_default();
        let gm_display_name: String = gm_row.try_get("display_name").unwrap_or_default();

        let camp_rows = sqlx::query(
            r#"
            SELECT campaign_id, slug, title, description, setting_pack
            FROM campaigns
            WHERE owner_user_id = ?1 AND is_discoverable = 1 AND is_archived = 0
            ORDER BY updated_at_us DESC
            "#,
        )
        .bind(&gm_uid)
        .fetch_all(state.auth.db_pool())
        .await
        .unwrap_or_default();

        let mut camp_map: HashMap<String, DirectoryCampaignEntry> = HashMap::new();
        let mut camp_order: Vec<String> = Vec::with_capacity(camp_rows.len());
        for crow in camp_rows {
            let Ok(cid) = crow.try_get::<String, _>("campaign_id") else {
                continue;
            };
            camp_order.push(cid.clone());
            camp_map.insert(
                cid.clone(),
                DirectoryCampaignEntry {
                    campaign_id: Some(cid),
                    slug: crow.try_get("slug").unwrap_or_default(),
                    title: crow.try_get("title").unwrap_or_default(),
                    description: crow.try_get("description").unwrap_or_default(),
                    setting_pack: crow
                        .try_get("setting_pack")
                        .unwrap_or_else(|_| "awakened".to_string()),
                    sessions: Vec::new(),
                },
            );
        }

        let sess_rows = sqlx::query(
            r#"
            SELECT session_id, session_mode, campaign_id, title, enable_ai_gm,
                   allow_join_requests, updated_at_us
            FROM saved_sessions
            WHERE COALESCE(NULLIF(owner_user_id, ''), host_user_id) = ?1
              AND is_discoverable = 1
            ORDER BY updated_at_us DESC
            "#,
        )
        .bind(&gm_uid)
        .fetch_all(state.auth.db_pool())
        .await
        .unwrap_or_default();

        let mut uncategorized_sessions = Vec::new();
        for srow in sess_rows {
            let Ok(sid) = srow.try_get::<String, _>("session_id") else {
                continue;
            };
            let mode_str: String = srow.try_get("session_mode").unwrap_or_default();
            let cid_opt: Option<String> = srow
                .try_get::<Option<String>, _>("campaign_id")
                .ok()
                .flatten()
                .filter(|s| !s.trim().is_empty());
            let title_opt: Option<String> = srow
                .try_get::<Option<String>, _>("title")
                .ok()
                .flatten()
                .filter(|s| !s.trim().is_empty());
            let has_ai_gm = srow.try_get::<i64, _>("enable_ai_gm").unwrap_or(0) != 0;
            let allow_join_requests =
                srow.try_get::<i64, _>("allow_join_requests").unwrap_or(1) != 0;
            let updated_at_us: i64 = srow.try_get("updated_at_us").unwrap_or(0);

            let active_human_count = active_guard
                .get(&sid)
                .map(|s| {
                    s.roster
                        .values()
                        .filter(|p| p.participant_kind == satya_schema::ParticipantKind::Human)
                        .count()
                })
                .unwrap_or(0);

            let (membership_status, caller_request_id, approval_token) = if gm_uid == user.user_id {
                (DirectoryMembershipStatus::Owner, None, None)
            } else if member_sids.iter().any(|m| m == &sid) {
                (DirectoryMembershipStatus::Member, None, None)
            } else if let Some((req_id, status, tok)) = req_by_sid.get(&sid) {
                match status {
                    JoinRequestStatus::Pending => (
                        DirectoryMembershipStatus::PendingRequest,
                        Some(req_id.clone()),
                        None,
                    ),
                    JoinRequestStatus::Approved => (
                        DirectoryMembershipStatus::ApprovedReady,
                        Some(req_id.clone()),
                        tok.clone(),
                    ),
                    JoinRequestStatus::Denied => (
                        DirectoryMembershipStatus::Denied,
                        Some(req_id.clone()),
                        None,
                    ),
                    _ => (DirectoryMembershipStatus::None, None, None),
                }
            } else {
                (DirectoryMembershipStatus::None, None, None)
            };

            let entry = DirectorySessionEntry {
                session_id: sid,
                title: title_opt,
                session_mode: session_mode_from_db(&mode_str),
                has_ai_gm,
                is_live: active_human_count > 0,
                active_human_count,
                allow_join_requests,
                caller_membership_status: membership_status,
                caller_request_id,
                approval_token,
                updated_at_us,
            };

            if let Some(cid) = cid_opt.as_ref().and_then(|c| camp_map.get_mut(c)) {
                cid.sessions.push(entry);
                total_sessions += 1;
            } else if cid_opt.is_none() {
                uncategorized_sessions.push(entry);
                total_sessions += 1;
            }
        }

        let mut campaigns: Vec<DirectoryCampaignEntry> = camp_order
            .into_iter()
            .filter_map(|cid| camp_map.remove(&cid))
            .collect();
        if !uncategorized_sessions.is_empty() {
            campaigns.push(DirectoryCampaignEntry {
                campaign_id: None,
                slug: "uncategorized".to_string(),
                title: "Standalone Sessions".to_string(),
                description: "Sessions not assigned to a campaign".to_string(),
                setting_pack: state.campaign_pack.clone(),
                sessions: uncategorized_sessions,
            });
        }

        let is_online = state.is_user_online(&gm_uid).await;
        gms.push(DirectoryGmEntry {
            user_id: gm_uid,
            username: gm_username,
            display_name: gm_display_name,
            is_online,
            campaigns,
        });
    }
    drop(active_guard);

    emit_campaign_directory_event(
        &state,
        CampaignDirAudit {
            event_type: CampaignDirectoryEventType::DirectorySearched,
            actor: actor_from_user(&user, SessionRole::Player),
            owner_user_id: None,
            campaign_id: None,
            campaign_title: None,
            campaign_slug: None,
            search_query_sha256: Some(hash_query_sha256(&raw_query)),
            result_count: total_sessions as i32,
            discoverable: true,
            scope_session_id: "directory",
        },
    )
    .await;

    let safe_query = if is_email_query {
        "[email lookup]".to_string()
    } else {
        raw_query
    };

    (
        StatusCode::OK,
        Json(json!(DirectorySearchResponse {
            query: safe_query,
            gms,
        })),
    )
        .into_response()
}

struct AccessRequestDraft {
    request_kind: JoinRequestKind,
    session_id: String,
    session_title: Option<String>,
    proposed_session_mode: Option<SessionMode>,
    proposed_enable_ai_gm: bool,
    session_mode: SessionMode,
    campaign_id: Option<String>,
    campaign_title: Option<String>,
    owner_user_id: String,
    owner_username: String,
    owner_display_name: String,
    owner_email: Option<String>,
    requested_role: SessionRole,
    clean_message: Option<String>,
}

fn validate_requested_member_role(
    role: SessionRole,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if matches!(
        role,
        SessionRole::Gm | SessionRole::Host | SessionRole::AiGm | SessionRole::SoloGm
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Requests may only request PLAYER, PARTICIPANT, or OBSERVER roles"
            })),
        ));
    }
    Ok(())
}

async fn check_join_request_rate_and_quota(
    state: &SignalingAppState,
    user: &UserAccount,
    actor: &Participant,
    sid: &str,
    session_mode: SessionMode,
    campaign_id: Option<String>,
    requested_role: SessionRole,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let jr_rate = state
        .storage
        .config()
        .quotas
        .max_join_requests_per_user_per_hour;
    if let Err(retry_secs) = state
        .rate_limiter
        .check_and_record(
            "join_request",
            &user.user_id,
            jr_rate,
            Duration::from_secs(3600),
        )
        .await
    {
        emit_session_access_event(
            state,
            SessionAccessAudit {
                event_type: SessionAccessEventType::RateLimitRejected,
                actor: actor.clone(),
                target_participant: None,
                session_id: sid,
                session_mode,
                request_id: None,
                campaign_id: campaign_id.clone(),
                requested_role: Some(requested_role),
                delivery_channel: None,
                reason: Some(format!("Join request rate limit (retry_after={retry_secs}s)")),
            },
        )
        .await;
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": format!("Join request rate limit exceeded; retry in {retry_secs}s"),
                "retry_after_secs": retry_secs
            })),
        ));
    }

    let quota = state.auth.compute_account_quota_usage(user).await;
    if quota.pending_requests_used >= quota.pending_requests_limit {
        let msg = format!(
            "Quota exceeded: maximum {} pending join requests allowed",
            quota.pending_requests_limit
        );
        emit_session_access_event(
            state,
            SessionAccessAudit {
                event_type: SessionAccessEventType::QuotaRejected,
                actor: actor.clone(),
                target_participant: None,
                session_id: sid,
                session_mode,
                request_id: None,
                campaign_id,
                requested_role: Some(requested_role),
                delivery_channel: None,
                reason: Some(msg.clone()),
            },
        )
        .await;
        return Err((StatusCode::TOO_MANY_REQUESTS, Json(json!({ "error": msg }))));
    }
    Ok(())
}

async fn persist_and_notify_access_request(
    state: &SignalingAppState,
    user: &UserAccount,
    actor: Participant,
    draft: AccessRequestDraft,
) -> axum::response::Response {
    let now_us = Utc::now().timestamp_micros();
    let request_id = format!("jreq_{}", Uuid::now_v7());
    let ttl_us = (state.storage.config().quotas.join_request_ttl_hours as i64) * 3_600_000_000;
    let expires_at_us = now_us + ttl_us;

    let gm_online = state.is_user_online(&draft.owner_user_id).await;
    let mut emailed_offline = false;
    let proposed_mode_db = draft.proposed_session_mode.map(session_mode_to_db);

    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO session_join_requests (
            request_id, request_kind, session_id, proposed_session_title,
            proposed_session_mode, proposed_enable_ai_gm, campaign_id, owner_user_id,
            requester_user_id, requester_username, requester_display_name,
            requested_role, message, status, approval_token, decision_note,
            emailed_offline, created_at_us, updated_at_us, expires_at_us
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'PENDING', NULL, NULL, 0, ?14, ?15, ?16)
        "#,
    )
    .bind(&request_id)
    .bind(draft.request_kind.as_str())
    .bind(&draft.session_id)
    .bind(&draft.session_title)
    .bind(proposed_mode_db)
    .bind(if draft.proposed_enable_ai_gm { 1_i64 } else { 0_i64 })
    .bind(&draft.campaign_id)
    .bind(&draft.owner_user_id)
    .bind(&user.user_id)
    .bind(&user.username)
    .bind(&user.display_name)
    .bind(session_role_to_db(draft.requested_role))
    .bind(&draft.clean_message)
    .bind(now_us)
    .bind(now_us)
    .bind(expires_at_us)
    .execute(state.auth.db_pool())
    .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to persist request: {e}") })),
        )
            .into_response();
    }

    let target_gm = Participant::human(
        &draft.owner_user_id,
        &draft.owner_username,
        &draft.owner_display_name,
        SessionRole::Gm,
    );
    let delivery_channel = if gm_online {
        "LIVE_WS".to_string()
    } else if draft.owner_email.is_some() {
        "EMAIL".to_string()
    } else {
        "INBOX_ONLY".to_string()
    };

    emit_session_access_event(
        state,
        SessionAccessAudit {
            event_type: SessionAccessEventType::JoinRequestCreated,
            actor,
            target_participant: Some(target_gm.clone()),
            session_id: &draft.session_id,
            session_mode: draft.session_mode,
            request_id: Some(request_id.clone()),
            campaign_id: draft.campaign_id.clone(),
            requested_role: Some(draft.requested_role),
            delivery_channel: Some(delivery_channel),
            reason: draft.clean_message.clone(),
        },
    )
    .await;

    let record_for_gm = JoinRequestRecord {
        request_id: request_id.clone(),
        request_kind: draft.request_kind,
        session_id: draft.session_id.clone(),
        session_title: draft.session_title.clone(),
        proposed_session_mode: draft.proposed_session_mode,
        proposed_enable_ai_gm: draft.proposed_enable_ai_gm,
        campaign_id: draft.campaign_id.clone(),
        campaign_title: draft.campaign_title.clone(),
        owner_user_id: draft.owner_user_id.clone(),
        owner_username: draft.owner_username.clone(),
        owner_display_name: draft.owner_display_name.clone(),
        requester_user_id: user.user_id.clone(),
        requester_username: user.username.clone(),
        requester_display_name: user.display_name.clone(),
        requested_role: draft.requested_role,
        message: draft.clean_message.clone(),
        status: JoinRequestStatus::Pending,
        approval_token: None,
        decision_note: None,
        emailed_offline: false,
        created_at_us: now_us,
        updated_at_us: now_us,
        expires_at_us,
    };

    if gm_online {
        state
            .send_account_notification(
                &draft.owner_user_id,
                &AccountWsPacket::JoinRequestReceived {
                    request: Box::new(record_for_gm.clone()),
                },
            )
            .await;
        state
            .send_user_live_session_packet(
                &draft.owner_user_id,
                &ServerWsPacket::JoinRequestNotification {
                    request: Box::new(record_for_gm.clone()),
                },
            )
            .await;
        let pending_count = count_pending_incoming(state, &draft.owner_user_id).await;
        state
            .send_account_notification(
                &draft.owner_user_id,
                &AccountWsPacket::PendingCountSync {
                    pending_incoming_count: pending_count,
                },
            )
            .await;
    } else if let Some(to_email) = draft.owner_email {
        let app_url = state.storage.config().email.public_app_url.trim_end_matches('/');
        let camp_label = draft
            .campaign_title
            .as_deref()
            .unwrap_or("Standalone Session");
        let note_line = draft
            .clean_message
            .as_deref()
            .map(|m| format!("\nMessage: {m}"))
            .unwrap_or_default();
        let (subject, action_desc) = match draft.request_kind {
            JoinRequestKind::CreateSession => {
                let title_disp = draft.session_title.as_deref().unwrap_or(&draft.session_id);
                (
                    format!(
                        "[Satya] New session request '{}' from {} (@{})",
                        title_disp, user.display_name, user.username
                    ),
                    format!(
                        "requested a new session '{}' ({}) in your campaign '{}'",
                        title_disp, draft.session_id, camp_label
                    ),
                )
            }
            JoinRequestKind::JoinSession => (
                format!(
                    "[Satya] Join request from {} (@{}) for {}",
                    user.display_name, user.username, draft.session_id
                ),
                format!(
                    "requested to join your session '{}' (Campaign: {})",
                    draft.session_id, camp_label
                ),
            ),
        };
        let email_msg = OutboundEmail {
            to_email,
            subject,
            body_text: format!(
                "Hello {},\n\n{} (@{}) {action_desc} as {:?}.{}\n\nLog in to Satya to approve or deny: {app_url}\n",
                draft.owner_display_name,
                user.display_name,
                user.username,
                draft.requested_role,
                note_line
            ),
            request_id: request_id.clone(),
        };
        if state.email_notifier.send_email(&email_msg).await.is_ok() {
            emailed_offline = true;
            let _ = sqlx::query(
                "UPDATE session_join_requests SET emailed_offline = 1 WHERE request_id = ?1",
            )
            .bind(&request_id)
            .execute(state.auth.db_pool())
            .await;
            emit_session_access_event(
                state,
                SessionAccessAudit {
                    event_type: SessionAccessEventType::JoinRequestEmailed,
                    actor: actor_from_user(user, draft.requested_role),
                    target_participant: Some(target_gm),
                    session_id: &draft.session_id,
                    session_mode: draft.session_mode,
                    request_id: Some(request_id),
                    campaign_id: draft.campaign_id,
                    requested_role: Some(draft.requested_role),
                    delivery_channel: Some("EMAIL".to_string()),
                    reason: Some("Dispatched offline owner notification".to_string()),
                },
            )
            .await;
        }
    }

    let mut resp_record = record_for_gm;
    resp_record.emailed_offline = emailed_offline;
    (StatusCode::OK, Json(json!(resp_record))).into_response()
}

/// Submits a Request-to-Join for `session_id` at `POST /api/sessions/:session_id/join-requests`.
///
/// - If the owning user is online (`/ws/notifications` or `/ws/session`), pushes a live alert.
/// - If offline and email is configured, dispatches an email via [`EmailNotifier`].
pub async fn create_join_request_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<CreateJoinRequestBody>,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = validate_requested_member_role(req.requested_role) {
        return resp.into_response();
    }
    let sid = session_id.trim().to_string();
    let actor = actor_from_user(&user, req.requested_role);

    let now_us = Utc::now().timestamp_micros();
    expire_stale_join_requests(&state, now_us).await;

    let Some(sess_row) = sqlx::query(
        r#"
        SELECT s.session_mode,
               COALESCE(NULLIF(s.owner_user_id, ''), s.host_user_id) AS owner_user_id,
               s.campaign_id,
               s.title AS session_title,
               s.allow_join_requests,
               c.title AS campaign_title,
               u.username AS owner_username,
               u.display_name AS owner_display_name,
               u.email AS owner_email
        FROM saved_sessions s
        LEFT JOIN campaigns c ON c.campaign_id = s.campaign_id
        JOIN users u ON u.user_id = COALESCE(NULLIF(s.owner_user_id, ''), s.host_user_id)
        WHERE s.session_id = ?1
        "#,
    )
    .bind(&sid)
    .fetch_optional(state.auth.db_pool())
    .await
    .ok()
    .flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{sid}' does not exist") })),
        )
            .into_response();
    };

    let session_mode =
        session_mode_from_db(&sess_row.try_get::<String, _>("session_mode").unwrap_or_default());
    let owner_user_id: String = sess_row.try_get("owner_user_id").unwrap_or_default();
    let owner_username: String = sess_row.try_get("owner_username").unwrap_or_default();
    let owner_display_name: String = sess_row.try_get("owner_display_name").unwrap_or_default();
    let owner_email: Option<String> = sess_row
        .try_get::<Option<String>, _>("owner_email")
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty());
    let campaign_id: Option<String> = sess_row.try_get("campaign_id").ok().flatten();
    let campaign_title: Option<String> = sess_row.try_get("campaign_title").ok().flatten();
    let session_title: Option<String> = sess_row.try_get("session_title").ok().flatten();
    let allow_join_requests = sess_row.try_get::<i64, _>("allow_join_requests").unwrap_or(1) != 0;

    if !allow_join_requests {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "This session is invite-password only and does not accept join requests" })),
        )
            .into_response();
    }
    if owner_user_id == user.user_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "You own this session; resume it directly" })),
        )
            .into_response();
    }

    let already_pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session_join_requests WHERE session_id = ?1 AND requester_user_id = ?2 AND status = 'PENDING'",
    )
    .bind(&sid)
    .bind(&user.user_id)
    .fetch_one(state.auth.db_pool())
    .await
    .unwrap_or(0);
    if already_pending > 0 {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "You already have a pending join request for this session" })),
        )
            .into_response();
    }

    if let Err(resp) = check_join_request_rate_and_quota(
        &state,
        &user,
        &actor,
        &sid,
        session_mode,
        campaign_id.clone(),
        req.requested_role,
    )
    .await
    {
        return resp.into_response();
    }

    let clean_msg = req
        .message
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(280).collect::<String>());

    persist_and_notify_access_request(
        &state,
        &user,
        actor,
        AccessRequestDraft {
            request_kind: JoinRequestKind::JoinSession,
            session_id: sid,
            session_title,
            proposed_session_mode: None,
            proposed_enable_ai_gm: false,
            session_mode,
            campaign_id,
            campaign_title,
            owner_user_id,
            owner_username,
            owner_display_name,
            owner_email,
            requested_role: req.requested_role,
            clean_message: clean_msg,
        },
    )
    .await
}

/// Submits a request for a campaign owner to create a new session in their campaign
/// at `POST /api/campaigns/:campaign_id/session-requests`.
pub async fn create_campaign_session_request_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(campaign_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<CreateCampaignSessionRequestBody>,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = validate_requested_member_role(req.requested_role) {
        return resp.into_response();
    }
    let session_title = req.session_title.trim().to_string();
    if session_title.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Proposed session title cannot be empty" })),
        )
            .into_response();
    }
    let cid = campaign_id.trim().to_string();
    let actor = actor_from_user(&user, req.requested_role);

    let now_us = Utc::now().timestamp_micros();
    expire_stale_join_requests(&state, now_us).await;

    let Some(camp_row) = sqlx::query(
        r#"
        SELECT c.campaign_id, c.slug, c.title AS campaign_title,
               c.is_discoverable, c.is_archived, c.owner_user_id,
               u.username AS owner_username,
               u.display_name AS owner_display_name,
               u.email AS owner_email
        FROM campaigns c
        JOIN users u ON u.user_id = c.owner_user_id
        WHERE c.campaign_id = ?1
        "#,
    )
    .bind(&cid)
    .fetch_optional(state.auth.db_pool())
    .await
    .ok()
    .flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Campaign '{cid}' not found") })),
        )
            .into_response();
    };

    let owner_user_id: String = camp_row.try_get("owner_user_id").unwrap_or_default();
    if owner_user_id == user.user_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "You own this campaign; create the session directly" })),
        )
            .into_response();
    }
    let is_archived = camp_row.try_get::<i64, _>("is_archived").unwrap_or(0) != 0;
    if is_archived {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Cannot request a new session in an archived campaign" })),
        )
            .into_response();
    }
    let is_discoverable = camp_row.try_get::<i64, _>("is_discoverable").unwrap_or(1) != 0;
    if !is_discoverable {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "This campaign is private and does not accept session requests" })),
        )
            .into_response();
    }

    let slug: String = camp_row.try_get("slug").unwrap_or_default();
    let campaign_title: String = camp_row.try_get("campaign_title").unwrap_or_default();
    let owner_username: String = camp_row.try_get("owner_username").unwrap_or_default();
    let owner_display_name: String = camp_row.try_get("owner_display_name").unwrap_or_default();
    let owner_email: Option<String> = camp_row
        .try_get::<Option<String>, _>("owner_email")
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty());

    let base_sid = req
        .session_id
        .as_deref()
        .map(slugify_campaign_title)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{slug}-{}", slugify_campaign_title(&session_title)));
    let mut proposed_sid = base_sid.clone();
    for suffix in 2..=50 {
        let sess_exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM saved_sessions WHERE session_id = ?1")
                .bind(&proposed_sid)
                .fetch_one(state.auth.db_pool())
                .await
                .unwrap_or(0);
        let req_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM session_join_requests WHERE session_id = ?1 AND status IN ('PENDING', 'APPROVED')",
        )
        .bind(&proposed_sid)
        .fetch_one(state.auth.db_pool())
        .await
        .unwrap_or(0);
        if sess_exists == 0 && req_exists == 0 {
            break;
        }
        proposed_sid = format!("{base_sid}-{suffix}");
    }

    if let Err(resp) = check_join_request_rate_and_quota(
        &state,
        &user,
        &actor,
        &proposed_sid,
        req.session_mode,
        Some(cid.clone()),
        req.requested_role,
    )
    .await
    {
        return resp.into_response();
    }

    let clean_msg = req
        .message
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(280).collect::<String>());

    persist_and_notify_access_request(
        &state,
        &user,
        actor,
        AccessRequestDraft {
            request_kind: JoinRequestKind::CreateSession,
            session_id: proposed_sid,
            session_title: Some(session_title),
            proposed_session_mode: Some(req.session_mode),
            proposed_enable_ai_gm: req.enable_ai_gm,
            session_mode: req.session_mode,
            campaign_id: Some(cid),
            campaign_title: Some(campaign_title),
            owner_user_id,
            owner_username,
            owner_display_name,
            owner_email,
            requested_role: req.requested_role,
            clean_message: clean_msg,
        },
    )
    .await
}

fn row_to_join_request_record(
    row: &sqlx::sqlite::SqliteRow,
    include_token: bool,
) -> Option<JoinRequestRecord> {
    let status_str: String = row.try_get("status").ok()?;
    let status = join_request_status_from_db(&status_str);
    let kind_str: String = row
        .try_get("request_kind")
        .unwrap_or_else(|_| "JOIN_SESSION".to_string());
    let request_kind =
        JoinRequestKind::from_code_str(&kind_str).unwrap_or(JoinRequestKind::JoinSession);
    let proposed_session_mode = row
        .try_get::<Option<String>, _>("proposed_session_mode")
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .map(|s| session_mode_from_db(&s));
    let proposed_enable_ai_gm = row.try_get::<i64, _>("proposed_enable_ai_gm").unwrap_or(0) != 0;
    let approval_token = if include_token && status == JoinRequestStatus::Approved {
        row.try_get::<Option<String>, _>("approval_token")
            .ok()
            .flatten()
    } else {
        None
    };
    Some(JoinRequestRecord {
        request_id: row.try_get("request_id").ok()?,
        request_kind,
        session_id: row.try_get("session_id").ok()?,
        session_title: row.try_get("session_title").ok().flatten(),
        proposed_session_mode,
        proposed_enable_ai_gm,
        campaign_id: row.try_get("campaign_id").ok().flatten(),
        campaign_title: row.try_get("campaign_title").ok().flatten(),
        owner_user_id: row.try_get("owner_user_id").ok()?,
        owner_username: row.try_get("owner_username").unwrap_or_default(),
        owner_display_name: row.try_get("owner_display_name").unwrap_or_default(),
        requester_user_id: row.try_get("requester_user_id").ok()?,
        requester_username: row.try_get("requester_username").unwrap_or_default(),
        requester_display_name: row.try_get("requester_display_name").unwrap_or_default(),
        requested_role: session_role_from_db(
            &row.try_get::<String, _>("requested_role").unwrap_or_default(),
        ),
        message: row.try_get("message").ok().flatten(),
        status,
        approval_token,
        decision_note: row.try_get("decision_note").ok().flatten(),
        emailed_offline: row.try_get::<i64, _>("emailed_offline").unwrap_or(0) != 0,
        created_at_us: row.try_get("created_at_us").unwrap_or(0),
        updated_at_us: row.try_get("updated_at_us").unwrap_or(0),
        expires_at_us: row.try_get("expires_at_us").unwrap_or(0),
    })
}

/// Lists join requests submitted by the authenticated caller at `GET /api/join-requests/mine`.
pub async fn list_my_join_requests_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let now_us = Utc::now().timestamp_micros();
    expire_stale_join_requests(&state, now_us).await;

    let rows = sqlx::query(
        r#"
        SELECT r.*,
               COALESCE(s.title, r.proposed_session_title) AS session_title,
               c.title AS campaign_title,
               u.username AS owner_username,
               u.display_name AS owner_display_name
        FROM session_join_requests r
        LEFT JOIN saved_sessions s ON s.session_id = r.session_id
        LEFT JOIN campaigns c ON c.campaign_id = r.campaign_id
        LEFT JOIN users u ON u.user_id = r.owner_user_id
        WHERE r.requester_user_id = ?1
        ORDER BY r.updated_at_us DESC
        LIMIT 50
        "#,
    )
    .bind(&user.user_id)
    .fetch_all(state.auth.db_pool())
    .await
    .unwrap_or_default();

    let requests: Vec<JoinRequestRecord> = rows
        .iter()
        .filter_map(|r| row_to_join_request_record(r, true))
        .collect();
    (
        StatusCode::OK,
        Json(json!(JoinRequestsListResponse { requests })),
    )
        .into_response()
}

/// Lists incoming join requests for sessions owned by the caller at `GET /api/join-requests/incoming`.
pub async fn list_incoming_join_requests_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let now_us = Utc::now().timestamp_micros();
    expire_stale_join_requests(&state, now_us).await;

    let rows = sqlx::query(
        r#"
        SELECT r.*,
               COALESCE(s.title, r.proposed_session_title) AS session_title,
               c.title AS campaign_title,
               u.username AS owner_username,
               u.display_name AS owner_display_name
        FROM session_join_requests r
        LEFT JOIN saved_sessions s ON s.session_id = r.session_id
        LEFT JOIN campaigns c ON c.campaign_id = r.campaign_id
        LEFT JOIN users u ON u.user_id = r.owner_user_id
        WHERE r.owner_user_id = ?1
        ORDER BY CASE WHEN r.status = 'PENDING' THEN 0 ELSE 1 END, r.updated_at_us DESC
        LIMIT 100
        "#,
    )
    .bind(&user.user_id)
    .fetch_all(state.auth.db_pool())
    .await
    .unwrap_or_default();

    let requests: Vec<JoinRequestRecord> = rows
        .iter()
        .filter_map(|r| row_to_join_request_record(r, false))
        .collect();
    (
        StatusCode::OK,
        Json(json!(JoinRequestsListResponse { requests })),
    )
        .into_response()
}

/// Approves or denies a `PENDING` join or new-session request at `POST /api/join-requests/:request_id/decide`.
pub async fn decide_join_request_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(request_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<DecideJoinRequestBody>,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let rid = request_id.trim().to_string();
    let now_us = Utc::now().timestamp_micros();
    expire_stale_join_requests(&state, now_us).await;

    let Some(row) = sqlx::query(
        r#"
        SELECT r.*,
               COALESCE(s.title, r.proposed_session_title) AS session_title,
               s.session_mode,
               c.title AS campaign_title,
               u.username AS owner_username,
               u.display_name AS owner_display_name
        FROM session_join_requests r
        LEFT JOIN saved_sessions s ON s.session_id = r.session_id
        LEFT JOIN campaigns c ON c.campaign_id = r.campaign_id
        LEFT JOIN users u ON u.user_id = r.owner_user_id
        WHERE r.request_id = ?1
        "#,
    )
    .bind(&rid)
    .fetch_optional(state.auth.db_pool())
    .await
    .ok()
    .flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Join request '{rid}' not found") })),
        )
            .into_response();
    };

    let owner_user_id: String = row.try_get("owner_user_id").unwrap_or_default();
    if owner_user_id != user.user_id && user.account_role != AccountRole::Admin {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the owning user or Admin can decide this request" })),
        )
            .into_response();
    }

    let current_status =
        join_request_status_from_db(&row.try_get::<String, _>("status").unwrap_or_default());
    if current_status != JoinRequestStatus::Pending {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "error": format!("Request is already {}", current_status.as_str())
            })),
        )
            .into_response();
    }

    let mut updated_record = row_to_join_request_record(&row, false).unwrap();
    let session_id = updated_record.session_id.clone();
    let campaign_id = updated_record.campaign_id.clone();
    let session_mode = row
        .try_get::<Option<String>, _>("session_mode")
        .ok()
        .flatten()
        .or_else(|| {
            updated_record
                .proposed_session_mode
                .map(|m| session_mode_to_db(m).to_string())
        })
        .map(|s| session_mode_from_db(&s))
        .unwrap_or(SessionMode::Ttrpg);

    if req.approve && updated_record.request_kind == JoinRequestKind::CreateSession {
        let Some(ref cid) = campaign_id else {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Session creation request is missing campaign_id" })),
            )
                .into_response();
        };
        let Some(camp_row) = sqlx::query(
            "SELECT slug, is_archived FROM campaigns WHERE campaign_id = ?1",
        )
        .bind(cid)
        .fetch_optional(state.auth.db_pool())
        .await
        .ok()
        .flatten() else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("Campaign '{cid}' no longer exists") })),
            )
                .into_response();
        };
        if camp_row.try_get::<i64, _>("is_archived").unwrap_or(0) != 0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Cannot create a session in an archived campaign" })),
            )
                .into_response();
        }
        let campaign_slug: String = camp_row.try_get("slug").unwrap_or_default();
        let Some(owner_acct) = state.auth.get_user_by_id(&owner_user_id).await else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Campaign owner account not found" })),
            )
                .into_response();
        };
        let title_str = updated_record
            .session_title
            .as_deref()
            .unwrap_or(&session_id);
        if let Err(err) = state
            .auth
            .provision_approved_campaign_session(
                &owner_acct,
                ProvisionCampaignSessionParams {
                    campaign_id: cid,
                    campaign_slug: &campaign_slug,
                    session_id: &session_id,
                    title: title_str,
                    session_mode,
                    enable_ai_gm: updated_record.proposed_enable_ai_gm,
                },
            )
            .await
        {
            let msg = err.to_string();
            let status = if msg.starts_with("Quota exceeded:") {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::BAD_REQUEST
            };
            return (status, Json(json!({ "error": msg }))).into_response();
        }
        emit_campaign_directory_event(
            &state,
            CampaignDirAudit {
                event_type: CampaignDirectoryEventType::SessionAssignedToCampaign,
                actor: actor_from_user(&user, SessionRole::Gm),
                owner_user_id: Some(owner_user_id.clone()),
                campaign_id: Some(cid.clone()),
                campaign_title: updated_record.session_title.clone(),
                campaign_slug: Some(campaign_slug),
                search_query_sha256: None,
                result_count: 1,
                discoverable: true,
                scope_session_id: &session_id,
            },
        )
        .await;
    }

    let next_status = if req.approve {
        JoinRequestStatus::Approved
    } else {
        JoinRequestStatus::Denied
    };
    let approval_token = if req.approve {
        Some(format!("aprv_{}", Uuid::new_v4()))
    } else {
        None
    };
    let grant_ttl_us = (state.storage.config().quotas.approval_grant_ttl_hours as i64) * 3_600_000_000;
    let expires_at_us = if req.approve {
        now_us + grant_ttl_us
    } else {
        row.try_get("expires_at_us").unwrap_or(now_us)
    };
    let decision_note = req
        .decision_note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(280).collect::<String>());

    let _ = sqlx::query(
        r#"
        UPDATE session_join_requests
        SET status = ?1, approval_token = ?2, decision_note = ?3, updated_at_us = ?4, expires_at_us = ?5
        WHERE request_id = ?6
        "#,
    )
    .bind(join_request_status_to_db(next_status))
    .bind(&approval_token)
    .bind(&decision_note)
    .bind(now_us)
    .bind(expires_at_us)
    .bind(&rid)
    .execute(state.auth.db_pool())
    .await;

    let requester_user_id = updated_record.requester_user_id.clone();
    let requester_username = updated_record.requester_username.clone();
    let requester_display_name = updated_record.requester_display_name.clone();
    let requested_role = updated_record.requested_role;

    updated_record.status = next_status;
    updated_record.decision_note = decision_note.clone();
    updated_record.updated_at_us = now_us;
    updated_record.expires_at_us = expires_at_us;
    updated_record.approval_token = approval_token.clone();

    let access_ev_type = if req.approve {
        SessionAccessEventType::JoinRequestApproved
    } else {
        SessionAccessEventType::JoinRequestDenied
    };
    emit_session_access_event(
        &state,
        SessionAccessAudit {
            event_type: access_ev_type,
            actor: actor_from_user(&user, SessionRole::Gm),
            target_participant: Some(Participant::human(
                &requester_user_id,
                &requester_username,
                &requester_display_name,
                requested_role,
            )),
            session_id: &session_id,
            session_mode,
            request_id: Some(rid),
            campaign_id,
            requested_role: Some(requested_role),
            delivery_channel: Some("LIVE_WS".to_string()),
            reason: decision_note,
        },
    )
    .await;

    state
        .send_account_notification(
            &requester_user_id,
            &AccountWsPacket::JoinRequestResolved {
                request: Box::new(updated_record.clone()),
            },
        )
        .await;
    let pending_count = count_pending_incoming(&state, &owner_user_id).await;
    state
        .send_account_notification(
            &owner_user_id,
            &AccountWsPacket::PendingCountSync {
                pending_incoming_count: pending_count,
            },
        )
        .await;

    // Never return another user's approval token in the owner's HTTP response
    let mut gm_resp = updated_record;
    gm_resp.approval_token = None;
    (StatusCode::OK, Json(json!(gm_resp))).into_response()
}

/// Cancels the caller's own `PENDING` join request at `DELETE /api/join-requests/:request_id`.
pub async fn cancel_join_request_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(request_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match verify_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let rid = request_id.trim().to_string();
    let now_us = Utc::now().timestamp_micros();

    let Some(row) = sqlx::query(
        "SELECT session_id, campaign_id, owner_user_id, requester_user_id, requested_role, status FROM session_join_requests WHERE request_id = ?1",
    )
    .bind(&rid)
    .fetch_optional(state.auth.db_pool())
    .await
    .ok()
    .flatten() else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Join request '{rid}' not found") })),
        )
            .into_response();
    };

    let requester_uid: String = row.try_get("requester_user_id").unwrap_or_default();
    if requester_uid != user.user_id && user.account_role != AccountRole::Admin {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the requester can cancel this join request" })),
        )
            .into_response();
    }
    let status = join_request_status_from_db(&row.try_get::<String, _>("status").unwrap_or_default());
    if status != JoinRequestStatus::Pending {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "Only PENDING join requests can be cancelled" })),
        )
            .into_response();
    }

    let _ = sqlx::query(
        "UPDATE session_join_requests SET status = 'CANCELLED', updated_at_us = ?1 WHERE request_id = ?2",
    )
    .bind(now_us)
    .bind(&rid)
    .execute(state.auth.db_pool())
    .await;

    let session_id: String = row.try_get("session_id").unwrap_or_default();
    let owner_user_id: String = row.try_get("owner_user_id").unwrap_or_default();
    let campaign_id: Option<String> = row.try_get("campaign_id").ok().flatten();
    let requested_role =
        session_role_from_db(&row.try_get::<String, _>("requested_role").unwrap_or_default());

    emit_session_access_event(
        &state,
        SessionAccessAudit {
            event_type: SessionAccessEventType::JoinRequestCancelled,
            actor: actor_from_user(&user, requested_role),
            target_participant: None,
            session_id: &session_id,
            session_mode: SessionMode::Ttrpg,
            request_id: Some(rid.clone()),
            campaign_id,
            requested_role: Some(requested_role),
            delivery_channel: None,
            reason: Some("Requester cancelled join request".to_string()),
        },
    )
    .await;

    let pending_count = count_pending_incoming(&state, &owner_user_id).await;
    state
        .send_account_notification(
            &owner_user_id,
            &AccountWsPacket::PendingCountSync {
                pending_incoming_count: pending_count,
            },
        )
        .await;

    (
        StatusCode::OK,
        Json(json!({ "status": "cancelled", "request_id": rid })),
    )
        .into_response()
}

/// Query string parameters for `GET /ws/notifications?auth_token=...`.
#[derive(Debug, Deserialize)]
pub struct WsNotificationsQuery {
    /// Bearer account token (`satya_tok_*`) authenticating the notification stream.
    pub auth_token: String,
}

/// Upgrades an authenticated lobby/user connection to `/ws/notifications` for real-time
/// join request alerts and approval pushes.
pub async fn ws_notifications_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsNotificationsQuery>,
    State(state): State<Arc<SignalingAppState>>,
) -> impl IntoResponse {
    let Some(user) = state.auth.verify_auth_token(&query.auth_token).await else {
        return (StatusCode::UNAUTHORIZED, "Invalid auth_token").into_response();
    };
    ws.on_upgrade(move |socket| handle_notifications_socket(socket, user, state))
}

async fn handle_notifications_socket(
    socket: WebSocket,
    user: UserAccount,
    state: Arc<SignalingAppState>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let conn_id = state
        .next_conn_id
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    {
        let mut guard = state.account_sockets.write().await;
        guard
            .entry(user.user_id.clone())
            .or_default()
            .insert(conn_id, tx.clone());
    }

    let pending_incoming_count = count_pending_incoming(&state, &user.user_id).await;
    if let Ok(hello) = serde_json::to_string(&AccountWsPacket::Connected {
        user_id: user.user_id.clone(),
        pending_incoming_count,
    }) {
        let _ = tx.send(hello);
    }

    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(WsMessage::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_rx.next().await {
        if matches!(msg, WsMessage::Close(_)) {
            break;
        }
    }

    forward_task.abort();
    let mut guard = state.account_sockets.write().await;
    if let Some(map) = guard.get_mut(&user.user_id) {
        map.remove(&conn_id);
        if map.is_empty() {
            guard.remove(&user.user_id);
        }
    }
}
