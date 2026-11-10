//! Session Transcript Export (`.md` / `.json`) and Portable `.satyasession` Snapshot Export/Import
//! (`GET /api/sessions/:id/transcript`, `GET /api/sessions/:id/export`, `POST /api/sessions/import`).

use crate::auth::{AuthAndSessionManager, JoinTokenClaims, ProtectedSession, UserAccount};
use crate::session_persistence::session_role_from_db;
use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use chrono::{TimeZone, Utc};
use satya_schema::{
    ImportSessionSnapshotRequest, MessageVisibility, Participant, SessionEnterResponse,
    SessionHistorySnapshot, SessionMode, SessionRole, SessionSnapshotBundle, SessionTranscriptItem,
    SessionTranscriptItemKind, SessionTranscriptResponse,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Query parameters for `GET /api/sessions/:session_id/transcript`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TranscriptExportQuery {
    /// Output format: `"json"` (default) or `"md"` / `"markdown"`.
    #[serde(default)]
    pub format: Option<String>,
}

impl AuthAndSessionManager {
    /// Resolves the caller's role in `session_id` (from active roster or `session_participants`),
    /// returning an error if `user` has never joined or hosted `session_id`.
    pub(crate) async fn resolve_authorized_member_role(
        &self,
        session_id: &str,
        user: &UserAccount,
    ) -> anyhow::Result<(SessionMode, SessionRole, String)> {
        let sid = session_id.trim();
        if !self.sessions.read().await.contains_key(sid) {
            let _ = self.rehydrate_session_from_db(sid).await;
        }

        let (mode, host_uid, active_role, campaign_tag) = {
            let guard = self.sessions.read().await;
            let session = guard
                .get(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' not found"))?;
            let active_role = session.roster.get(&user.user_id).map(|p| p.role);
            (
                session.session_mode,
                session.host_user_id.clone(),
                active_role,
                session.campaign_tag.clone(),
            )
        };

        if let Some(r) = active_role {
            return Ok((mode, r, campaign_tag));
        }

        let saved_role: Option<String> = sqlx::query_scalar(
            "SELECT role FROM session_participants WHERE session_id = ?1 AND user_id = ?2",
        )
        .bind(sid)
        .bind(&user.user_id)
        .fetch_optional(&self.db_pool)
        .await
        .ok()
        .flatten();

        if let Some(role_str) = saved_role {
            return Ok((mode, session_role_from_db(&role_str), campaign_tag));
        }

        if host_uid == user.user_id {
            let role = match mode {
                SessionMode::Ttrpg => SessionRole::Gm,
                SessionMode::Meeting => SessionRole::Host,
            };
            return Ok((mode, role, campaign_tag));
        }

        anyhow::bail!(
            "User '{}' is not a participant in session '{sid}'",
            user.username
        )
    }

    /// Builds a chronological, visibility-filtered [`SessionTranscriptResponse`] for `user` in `session_id`.
    pub async fn build_session_transcript(
        &self,
        session_id: &str,
        user: &UserAccount,
    ) -> anyhow::Result<SessionTranscriptResponse> {
        let sid = session_id.trim();
        let (session_mode, role, _campaign_tag) =
            self.resolve_authorized_member_role(sid, user).await?;
        let has_human_gm = self.session_has_human_gm(sid).await;
        let (chat_events, mechanics_events) = self
            .load_visible_events_from_db(sid, &user.user_id, role, has_human_gm)
            .await;

        let mut items: Vec<SessionTranscriptItem> =
            Vec::with_capacity(chat_events.len() + mechanics_events.len());
        for ev in chat_events {
            let ts_us = if ev.metadata.ingest_timestamp_us > 0 {
                ev.metadata.ingest_timestamp_us
            } else {
                ev.metadata.client_timestamp_us
            };
            items.push(SessionTranscriptItem {
                sequence_num: ev.metadata.sequence_num,
                timestamp_us: ts_us,
                kind: SessionTranscriptItemKind::Chat,
                actor_display_name: ev.sender.display_name.clone(),
                actor_role: ev.sender.role,
                visibility: ev.visibility,
                summary_markdown: ev.body.clone(),
                chat_event: Some(ev),
                mechanics_event: None,
            });
        }
        for ev in mechanics_events {
            let ts_us = if ev.metadata.ingest_timestamp_us > 0 {
                ev.metadata.ingest_timestamp_us
            } else {
                ev.metadata.client_timestamp_us
            };
            let n_pairs = (ev.dice_rolled_d20.len() / 2).max(1);
            let summary = format!(
                "**{}** rolled `{}` (`N={}x2d20`, `Control C={}`, `TN_eff={}`) -> **{:?}** (`{}` successes, cost `{}` ticks){}",
                ev.actor.display_name,
                ev.skill_name.as_deref().unwrap_or("Action"),
                n_pairs,
                ev.attribute_tn.unwrap_or(10),
                ev.difficulty_rating.unwrap_or(20),
                ev.outcome_tier,
                ev.successes_counted,
                ev.action_count_cost.unwrap_or(0),
                ev.notes
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .map(|n| format!(" — _{n}_"))
                    .unwrap_or_default()
            );
            items.push(SessionTranscriptItem {
                sequence_num: ev.metadata.sequence_num,
                timestamp_us: ts_us,
                kind: SessionTranscriptItemKind::MechanicsRoll,
                actor_display_name: ev.actor.display_name.clone(),
                actor_role: ev.actor.role,
                visibility: MessageVisibility::Public,
                summary_markdown: summary,
                chat_event: None,
                mechanics_event: Some(ev),
            });
        }

        items.sort_by(|a, b| {
            a.sequence_num
                .cmp(&b.sequence_num)
                .then_with(|| a.timestamp_us.cmp(&b.timestamp_us))
        });

        Ok(SessionTranscriptResponse {
            session_id: sid.to_string(),
            session_mode,
            exported_at_us: Utc::now().timestamp_micros(),
            exported_by_user_id: user.user_id.clone(),
            exported_by_role: role,
            total_items: items.len(),
            items,
        })
    }

    /// Formats a chronological slice of [`SessionTranscriptItem`]s into a human-readable Markdown document.
    pub fn format_session_transcript_markdown(
        session_id: &str,
        session_mode: SessionMode,
        exported_at_us: i64,
        items: &[SessionTranscriptItem],
    ) -> String {
        let exported_iso = format_timestamp_us_iso(exported_at_us);
        let mut out = String::with_capacity(2048 + items.len() * 160);
        out.push_str(&format!("# Satya Session Transcript: `{session_id}`\n\n"));
        out.push_str(&format!(
            "- **Session Mode**: `{:?}`\n- **Exported At (UTC)**: `{}`\n- **Total Events**: `{}`\n\n---\n\n",
            session_mode,
            exported_iso,
            items.len()
        ));

        if items.is_empty() {
            out.push_str("_No chat messages or mechanics events recorded in this session._\n");
            return out;
        }

        for item in items {
            let ts_str = format_timestamp_us_hms(item.timestamp_us);
            match item.kind {
                SessionTranscriptItemKind::Chat => {
                    let vis_badge = match item.visibility {
                        MessageVisibility::Public => "PUBLIC",
                        MessageVisibility::Private => "WHISPER",
                        MessageVisibility::GmOnly => "GM_ONLY",
                        MessageVisibility::HostOnly => "HOST_ONLY",
                    };
                    out.push_str(&format!(
                        "### [{}] {} (`{:?}` · `{}`)\n\n{}\n\n",
                        ts_str,
                        item.actor_display_name,
                        item.actor_role,
                        vis_badge,
                        item.summary_markdown.trim()
                    ));
                }
                SessionTranscriptItemKind::MechanicsRoll => {
                    out.push_str(&format!(
                        "### [{}] 🎲 Task Check — {}\n\n> {}\n\n",
                        ts_str,
                        item.actor_display_name,
                        item.summary_markdown.trim()
                    ));
                }
            }
        }
        out
    }

    /// Exports a portable `.satyasession` [`SessionSnapshotBundle`] for `session_id` scoped to `user`'s visibility.
    pub async fn export_session_snapshot(
        &self,
        session_id: &str,
        user: &UserAccount,
    ) -> anyhow::Result<SessionSnapshotBundle> {
        let sid = session_id.trim();
        let (session_mode, role, campaign_tag) =
            self.resolve_authorized_member_role(sid, user).await?;
        let has_human_gm = self.session_has_human_gm(sid).await;
        let (chat_events, mechanics_events) = self
            .load_visible_events_from_db(sid, &user.user_id, role, has_human_gm)
            .await;

        let (roster, active_map, combat_tracker, gm_memories) = {
            let guard = self.sessions.read().await;
            let session = guard
                .get(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' not found"))?;
            let roster_vec: Vec<Participant> = session.roster.values().cloned().collect();
            let memories = if matches!(role, SessionRole::Gm | SessionRole::Host) {
                session.gm_memories.clone()
            } else {
                Vec::new()
            };
            (
                roster_vec,
                session.active_map.clone(),
                Some(session.combat_tracker.clone()),
                memories,
            )
        };

        let gm_documents = self
            .list_session_gm_documents_for_user(sid, &user.user_id)
            .await
            .unwrap_or_default();

        let session_characters = self.load_session_characters_from_db(sid, &roster).await;

        Ok(SessionSnapshotBundle {
            format_version: 1,
            exported_at_us: Utc::now().timestamp_micros(),
            session_id: sid.to_string(),
            session_mode,
            campaign_tag,
            roster,
            active_map,
            combat_tracker,
            gm_documents,
            gm_memories,
            session_characters,
            chat_events,
            mechanics_events,
        })
    }

    /// Imports a portable `.satyasession` [`ImportSessionSnapshotRequest`], persisting its state
    /// and history into SQLite and joining `user` into the restored session.
    pub async fn import_session_snapshot(
        &self,
        user: &UserAccount,
        req: ImportSessionSnapshotRequest,
    ) -> anyhow::Result<(
        String,
        SessionMode,
        Participant,
        Vec<Participant>,
        SessionHistorySnapshot,
    )> {
        let target_sid = req
            .session_id_override
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| req.bundle.session_id.trim())
            .to_string();

        if target_sid.is_empty() {
            anyhow::bail!("Imported session_id cannot be empty");
        }
        if req.session_password.trim().is_empty() {
            anyhow::bail!("session_password cannot be empty when importing a session");
        }

        let password_hash = Self::hash_secret(&req.session_password)?;
        let session_mode = req.bundle.session_mode;
        let role = req.role.unwrap_or(match session_mode {
            SessionMode::Ttrpg => SessionRole::Gm,
            SessionMode::Meeting => SessionRole::Host,
        });

        let mut self_participant = Participant::human(
            &user.user_id,
            &user.username,
            user.display_name.clone(),
            role,
        );

        for sheet_rec in &req.bundle.session_characters {
            if sheet_rec.owner_user_id == user.user_id {
                self_participant.character_id = Some(sheet_rec.character_id.clone());
            }
        }

        let mut roster = HashMap::new();
        roster.insert(self_participant.user_id.clone(), self_participant.clone());
        for p in &req.bundle.roster {
            if p.role == SessionRole::AiGm {
                let mut ai_p = p.clone();
                ai_p.user_id = format!("ai-gm-{target_sid}");
                roster.insert(ai_p.user_id.clone(), ai_p);
            }
        }
        let roster_vec: Vec<Participant> = roster.values().cloned().collect();

        let mut combat_tracker = match req.bundle.combat_tracker {
            Some(ct) => ct,
            None => {
                self.load_or_init_combat_tracker(&target_sid, &roster_vec)
                    .await
            }
        };
        combat_tracker.session_id = target_sid.clone();
        Self::sync_combatant_into_tracker(&mut combat_tracker, &self_participant);
        if let Ok(ct_json) = serde_json::to_string(&combat_tracker) {
            let _ = sqlx::query(
                r#"
                INSERT INTO session_combat_state (session_id, state_json, updated_at_us)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(session_id) DO UPDATE SET
                    state_json = excluded.state_json,
                    updated_at_us = excluded.updated_at_us
                "#,
            )
            .bind(&target_sid)
            .bind(ct_json)
            .bind(combat_tracker.updated_at_us)
            .execute(&self.db_pool)
            .await;
        }

        let mut gm_documents = req.bundle.gm_documents;
        for doc in &mut gm_documents {
            doc.session_id = target_sid.clone();
            if doc.is_persistent {
                let _ = self.persist_gm_doc_to_db(doc).await;
            }
        }

        let mut gm_memories = req.bundle.gm_memories;
        for mem in &mut gm_memories {
            mem.session_id = target_sid.clone();
            if mem.is_persistent {
                let _ = sqlx::query(
                    r#"
                    INSERT INTO gm_memories (
                        memory_id, session_id, campaign_tag, author_user_id, author_display_name,
                        category, title, content, is_pinned, is_persistent, updated_at_us
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                    ON CONFLICT(memory_id) DO UPDATE SET
                        campaign_tag = excluded.campaign_tag,
                        category = excluded.category,
                        title = excluded.title,
                        content = excluded.content,
                        is_pinned = excluded.is_pinned,
                        is_persistent = excluded.is_persistent,
                        updated_at_us = excluded.updated_at_us
                    "#,
                )
                .bind(&mem.memory_id)
                .bind(&mem.session_id)
                .bind(&mem.campaign_tag)
                .bind(&mem.author_user_id)
                .bind(&mem.author_display_name)
                .bind(mem.category.as_str())
                .bind(&mem.title)
                .bind(&mem.content)
                .bind(i64::from(mem.is_pinned))
                .bind(i64::from(mem.is_persistent))
                .bind(mem.updated_at_us)
                .execute(&self.db_pool)
                .await;
            }
        }

        let has_ai_gm = roster.values().any(|p| p.role == SessionRole::AiGm);
        let session_obj = ProtectedSession {
            session_id: target_sid.clone(),
            password_hash,
            session_mode,
            host_user_id: user.user_id.clone(),
            roster,
            ai_gm_vault: None,
            active_map: req.bundle.active_map,
            gm_documents,
            campaign_tag: if req.bundle.campaign_tag.trim().is_empty() {
                target_sid.clone()
            } else {
                req.bundle.campaign_tag
            },
            gm_memories,
            combat_tracker,
            created_at_us: Utc::now().timestamp_micros(),
        };

        self.persist_session_metadata_to_db(&session_obj, has_ai_gm)
            .await;
        self.persist_session_participant_to_db(&target_sid, &self_participant)
            .await;

        for mut chat_ev in req.bundle.chat_events {
            chat_ev.metadata.session_id = target_sid.clone();
            self.record_chat_event_to_db(&target_sid, &chat_ev).await;
        }
        for mut mech_ev in req.bundle.mechanics_events {
            mech_ev.metadata.session_id = target_sid.clone();
            self.record_mechanics_event_to_db(&target_sid, &mech_ev)
                .await;
        }

        {
            let mut guard = self.sessions.write().await;
            guard.insert(target_sid.clone(), session_obj);
        }

        let join_token = format!("join_{}", Uuid::new_v4());
        self.join_tokens.write().await.insert(
            join_token.clone(),
            JoinTokenClaims {
                session_id: target_sid.clone(),
                session_mode,
                participant: self_participant.clone(),
            },
        );

        let history = self
            .build_session_history_snapshot(&target_sid, &self_participant)
            .await;

        Ok((
            join_token,
            session_mode,
            self_participant,
            roster_vec,
            history,
        ))
    }
}

/// Exports the visibility-filtered chat & mechanics transcript at `GET /api/sessions/:session_id/transcript`.
pub async fn export_session_transcript_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    Query(query): Query<TranscriptExportQuery>,
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

    match state
        .auth
        .build_session_transcript(&session_id, &user)
        .await
    {
        Ok(transcript) => {
            let fmt = query
                .format
                .as_deref()
                .unwrap_or("json")
                .trim()
                .to_lowercase();
            if fmt == "md" || fmt == "markdown" {
                let md = AuthAndSessionManager::format_session_transcript_markdown(
                    &transcript.session_id,
                    transcript.session_mode,
                    transcript.exported_at_us,
                    &transcript.items,
                );
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
                    md,
                )
                    .into_response()
            } else {
                (StatusCode::OK, Json(json!(transcript))).into_response()
            }
        }
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Exports a portable `.satyasession` JSON snapshot at `GET /api/sessions/:session_id/export`.
pub async fn export_session_snapshot_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
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

    match state.auth.export_session_snapshot(&session_id, &user).await {
        Ok(bundle) => (StatusCode::OK, Json(json!(bundle))).into_response(),
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Imports a portable `.satyasession` JSON snapshot at `POST /api/sessions/import`.
pub async fn import_session_snapshot_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<ImportSessionSnapshotRequest>,
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

    match state.auth.import_session_snapshot(&user, req).await {
        Ok((join_token, session_mode, self_participant, roster, history)) => {
            let sid = history
                .combat_tracker
                .as_ref()
                .map(|c| c.session_id.clone())
                .unwrap_or_default();
            (
                StatusCode::OK,
                Json(json!(SessionEnterResponse {
                    session_id: sid,
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
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

fn format_timestamp_us_iso(ts_us: i64) -> String {
    Utc.timestamp_micros(ts_us)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| ts_us.to_string())
}

fn format_timestamp_us_hms(ts_us: i64) -> String {
    Utc.timestamp_micros(ts_us)
        .single()
        .map(|dt| dt.format("%H:%M:%S UTC").to_string())
        .unwrap_or_else(|| ts_us.to_string())
}
