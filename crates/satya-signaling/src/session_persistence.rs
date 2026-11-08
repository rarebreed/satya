//! Durable SQLite session persistence, lazy session rehydration, visibility-filtered
//! chat/mechanics history replay, and session resume handlers (`GET /api/auth/me`,
//! `GET /api/sessions/saved`, `POST /api/sessions/resume`).

use crate::auth::{AuthAndSessionManager, JoinTokenClaims, ProtectedSession, SessionAiGmVault, UserAccount};
use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use satya_schema::{
    AuthSuccessResponse, CharacterSheetRecord, ChatMessageEvent, GmMapUpdateRequest,
    LifecycleEventType, MessageVisibility, Participant, ParticipantKind, ResumeSessionRequest,
    SavedSessionSummary, SavedSessionsListResponse, ServerWsPacket, SessionEnterResponse,
    SessionHistorySnapshot, SessionLifecycleEvent, SessionMode, SessionRole, TtrpgMechanicsEvent,
    subjects,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedAiGmMeta {
    provider: String,
    model_id: String,
    vision_model_id: Option<String>,
    temperature: f32,
    persona_id: String,
    base_url: Option<String>,
}

impl AuthAndSessionManager {
    /// Returns the maximum `sequence_num` stored in `session_event_history` (`0` if empty).
    pub(crate) async fn max_persisted_sequence_num(&self) -> i64 {
        sqlx::query_scalar::<_, Option<i64>>(
            "SELECT MAX(sequence_num) FROM session_event_history",
        )
        .fetch_one(&self.db_pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(0)
    }

    /// Loads `(password_hash, active_map, created_at_us)` for an existing saved session if present.
    pub(crate) async fn load_saved_session_meta_from_db(
        &self,
        session_id: &str,
    ) -> Option<(String, Option<GmMapUpdateRequest>, i64)> {
        let row = sqlx::query(
            "SELECT password_hash, active_map_json, created_at_us FROM saved_sessions WHERE session_id = ?1",
        )
        .bind(session_id)
        .fetch_optional(&self.db_pool)
        .await
        .ok()??;

        let password_hash: String = row.try_get("password_hash").ok()?;
        let active_map_json: Option<String> = row.try_get("active_map_json").ok().flatten();
        let active_map = active_map_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<GmMapUpdateRequest>(raw).ok());
        let created_at_us: i64 = row.try_get("created_at_us").ok()?;
        Some((password_hash, active_map, created_at_us))
    }

    /// Loads the previously bound `character_id` for `(session_id, user_id)` if saved.
    pub(crate) async fn load_saved_participant_character_id(
        &self,
        session_id: &str,
        user_id: &str,
    ) -> Option<String> {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT character_id FROM session_participants WHERE session_id = ?1 AND user_id = ?2",
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .ok()
        .flatten()
        .flatten()
        .filter(|s| !s.trim().is_empty())
    }

    /// Persists session metadata to the SQLite `saved_sessions` table (never storing BYOK API keys).
    pub(crate) async fn persist_session_metadata_to_db(
        &self,
        session: &ProtectedSession,
        enable_ai_gm: bool,
    ) {
        let now_us = Utc::now().timestamp_micros();
        let mode_str = session_mode_to_db(session.session_mode);
        let active_map_json = session
            .active_map
            .as_ref()
            .and_then(|m| serde_json::to_string(m).ok());
        let ai_meta_json = session.ai_gm_vault.as_ref().and_then(|v| {
            serde_json::to_string(&PersistedAiGmMeta {
                provider: v.provider.clone(),
                model_id: v.model_id.clone(),
                vision_model_id: v.vision_model_id.clone(),
                temperature: v.temperature,
                persona_id: v.persona_id.clone(),
                base_url: v.base_url.clone(),
            })
            .ok()
        });
        let has_ai_gm = enable_ai_gm
            || session
                .roster
                .values()
                .any(|p| p.role == SessionRole::AiGm);

        let _ = sqlx::query(
            r#"
            INSERT INTO saved_sessions (
                session_id, password_hash, session_mode, host_user_id, campaign_tag,
                enable_ai_gm, ai_provider_json, active_map_json, created_at_us, updated_at_us
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(session_id) DO UPDATE SET
                password_hash = excluded.password_hash,
                session_mode = excluded.session_mode,
                campaign_tag = excluded.campaign_tag,
                enable_ai_gm = excluded.enable_ai_gm,
                ai_provider_json = COALESCE(excluded.ai_provider_json, saved_sessions.ai_provider_json),
                active_map_json = COALESCE(excluded.active_map_json, saved_sessions.active_map_json),
                updated_at_us = excluded.updated_at_us
            "#,
        )
        .bind(&session.session_id)
        .bind(&session.password_hash)
        .bind(mode_str)
        .bind(&session.host_user_id)
        .bind(&session.campaign_tag)
        .bind(if has_ai_gm { 1_i64 } else { 0_i64 })
        .bind(ai_meta_json)
        .bind(active_map_json)
        .bind(session.created_at_us)
        .bind(now_us)
        .execute(&self.db_pool)
        .await;
    }

    /// Updates the persisted non-secret AI GM provider configuration on `saved_sessions`.
    pub(crate) async fn persist_session_ai_gm_meta_to_db(
        &self,
        session_id: &str,
        vault: &SessionAiGmVault,
    ) {
        let now_us = Utc::now().timestamp_micros();
        let ai_meta_json = serde_json::to_string(&PersistedAiGmMeta {
            provider: vault.provider.clone(),
            model_id: vault.model_id.clone(),
            vision_model_id: vault.vision_model_id.clone(),
            temperature: vault.temperature,
            persona_id: vault.persona_id.clone(),
            base_url: vault.base_url.clone(),
        })
        .ok();
        let _ = sqlx::query(
            "UPDATE saved_sessions SET enable_ai_gm = 1, ai_provider_json = ?1, updated_at_us = ?2 WHERE session_id = ?3",
        )
        .bind(ai_meta_json)
        .bind(now_us)
        .bind(session_id)
        .execute(&self.db_pool)
        .await;
    }

    /// Updates `active_map_json` on `saved_sessions`.
    pub(crate) async fn persist_session_active_map_to_db(
        &self,
        session_id: &str,
        map: &GmMapUpdateRequest,
    ) {
        let now_us = Utc::now().timestamp_micros();
        if let Ok(json_str) = serde_json::to_string(map) {
            let _ = sqlx::query(
                "UPDATE saved_sessions SET active_map_json = ?1, updated_at_us = ?2 WHERE session_id = ?3",
            )
            .bind(json_str)
            .bind(now_us)
            .bind(session_id)
            .execute(&self.db_pool)
            .await;
        }
    }

    /// Upserts a human participant's membership and character binding in `session_participants`.
    pub(crate) async fn persist_session_participant_to_db(
        &self,
        session_id: &str,
        participant: &Participant,
    ) {
        if participant.participant_kind != ParticipantKind::Human {
            return;
        }
        let now_us = Utc::now().timestamp_micros();
        let role_str = session_role_to_db(participant.role);
        let _ = sqlx::query(
            r#"
            INSERT INTO session_participants (
                session_id, user_id, username, display_name, role, character_id, updated_at_us
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(session_id, user_id) DO UPDATE SET
                username = excluded.username,
                display_name = excluded.display_name,
                role = excluded.role,
                character_id = COALESCE(excluded.character_id, session_participants.character_id),
                updated_at_us = excluded.updated_at_us
            "#,
        )
        .bind(session_id)
        .bind(&participant.user_id)
        .bind(&participant.username)
        .bind(&participant.display_name)
        .bind(role_str)
        .bind(&participant.character_id)
        .bind(now_us)
        .execute(&self.db_pool)
        .await;

        let _ = sqlx::query("UPDATE saved_sessions SET updated_at_us = ?1 WHERE session_id = ?2")
            .bind(now_us)
            .bind(session_id)
            .execute(&self.db_pool)
            .await;
    }

    /// Lazily rehydrates a [`ProtectedSession`] from SQLite into `self.sessions` if not currently in memory.
    pub async fn rehydrate_session_from_db(&self, session_id: &str) -> Option<ProtectedSession> {
        let sid = session_id.trim();
        if let Some(existing) = self.sessions.read().await.get(sid).cloned() {
            return Some(existing);
        }

        let row = sqlx::query(
            r#"
            SELECT session_id, password_hash, session_mode, host_user_id, campaign_tag,
                   enable_ai_gm, ai_provider_json, active_map_json, created_at_us
            FROM saved_sessions
            WHERE session_id = ?1
            "#,
        )
        .bind(sid)
        .fetch_optional(&self.db_pool)
        .await
        .ok()??;

        let password_hash: String = row.try_get("password_hash").ok()?;
        let mode_str: String = row.try_get("session_mode").ok()?;
        let session_mode = session_mode_from_db(&mode_str);
        let host_user_id: String = row.try_get("host_user_id").ok()?;
        let campaign_tag: String = row.try_get("campaign_tag").ok()?;
        let enable_ai_gm: i64 = row.try_get("enable_ai_gm").unwrap_or(0);
        let ai_provider_json: Option<String> = row.try_get("ai_provider_json").ok().flatten();
        let active_map_json: Option<String> = row.try_get("active_map_json").ok().flatten();
        let created_at_us: i64 = row
            .try_get("created_at_us")
            .unwrap_or_else(|_| Utc::now().timestamp_micros());

        let active_map = active_map_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<GmMapUpdateRequest>(raw).ok());

        let mut roster = HashMap::new();
        let mut ai_gm_vault = None;
        if session_mode == SessionMode::Ttrpg && enable_ai_gm != 0 {
            if let Some(meta) = ai_provider_json
                .as_deref()
                .and_then(|raw| serde_json::from_str::<PersistedAiGmMeta>(raw).ok())
            {
                let composite_model_id = if meta.model_id.contains('/') {
                    meta.model_id.clone()
                } else {
                    format!("{}/{}", meta.provider, meta.model_id)
                };
                let mut ai_p = Participant::ai_gm(sid, &composite_model_id, &meta.persona_id);
                if let Some(info) = ai_p.ai_agent_info.as_mut() {
                    info.temperature = meta.temperature;
                }
                roster.insert(ai_p.user_id.clone(), ai_p);
                ai_gm_vault = Some(SessionAiGmVault {
                    sponsor_user_id: host_user_id.clone(),
                    provider: meta.provider,
                    model_id: meta.model_id,
                    vision_model_id: meta.vision_model_id,
                    temperature: meta.temperature,
                    persona_id: meta.persona_id,
                    base_url: meta.base_url,
                    api_key: None,
                });
            } else {
                let ai_p = Participant::ai_gm(sid, "satya-gm-v1", "harsh-reality-arbiter");
                roster.insert(ai_p.user_id.clone(), ai_p);
            }
        }

        let roster_vec: Vec<Participant> = roster.values().cloned().collect();
        let gm_documents = self.load_persistent_gm_docs_from_db(sid).await;
        let gm_memories = self.load_persistent_gm_memories_from_db(sid).await;
        let combat_tracker = self.load_or_init_combat_tracker(sid, &roster_vec).await;

        let rehydrated = ProtectedSession {
            session_id: sid.to_string(),
            password_hash,
            session_mode,
            host_user_id,
            roster,
            ai_gm_vault,
            active_map,
            gm_documents,
            campaign_tag,
            gm_memories,
            combat_tracker,
            created_at_us,
        };

        let mut guard = self.sessions.write().await;
        let entry = guard
            .entry(sid.to_string())
            .or_insert_with(|| rehydrated.clone());
        Some(entry.clone())
    }

    /// Appends a [`ChatMessageEvent`] to the SQLite `session_event_history` table.
    pub async fn record_chat_event_to_db(&self, session_id: &str, ev: &ChatMessageEvent) {
        let Ok(payload_json) = serde_json::to_string(ev) else {
            return;
        };
        let ts_us = if ev.metadata.ingest_timestamp_us > 0 {
            ev.metadata.ingest_timestamp_us
        } else {
            ev.metadata.client_timestamp_us
        };
        let vis_str = visibility_to_db(ev.visibility);
        let _ = sqlx::query(
            r#"
            INSERT OR REPLACE INTO session_event_history (
                session_id, sequence_num, event_kind, timestamp_us, sender_user_id, visibility, payload_json
            )
            VALUES (?1, ?2, 'CHAT', ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(session_id.trim())
        .bind(ev.metadata.sequence_num)
        .bind(ts_us)
        .bind(&ev.sender.user_id)
        .bind(vis_str)
        .bind(payload_json)
        .execute(&self.db_pool)
        .await;

        let _ = sqlx::query("UPDATE saved_sessions SET updated_at_us = ?1 WHERE session_id = ?2")
            .bind(ts_us)
            .bind(session_id.trim())
            .execute(&self.db_pool)
            .await;
    }

    /// Appends a [`TtrpgMechanicsEvent`] to the SQLite `session_event_history` table.
    pub async fn record_mechanics_event_to_db(&self, session_id: &str, ev: &TtrpgMechanicsEvent) {
        let Ok(payload_json) = serde_json::to_string(ev) else {
            return;
        };
        let ts_us = if ev.metadata.ingest_timestamp_us > 0 {
            ev.metadata.ingest_timestamp_us
        } else {
            ev.metadata.client_timestamp_us
        };
        let _ = sqlx::query(
            r#"
            INSERT OR REPLACE INTO session_event_history (
                session_id, sequence_num, event_kind, timestamp_us, sender_user_id, visibility, payload_json
            )
            VALUES (?1, ?2, 'MECHANICS', ?3, ?4, 'PUBLIC', ?5)
            "#,
        )
        .bind(session_id.trim())
        .bind(ev.metadata.sequence_num)
        .bind(ts_us)
        .bind(&ev.actor.user_id)
        .bind(payload_json)
        .execute(&self.db_pool)
        .await;

        let _ = sqlx::query("UPDATE saved_sessions SET updated_at_us = ?1 WHERE session_id = ?2")
            .bind(ts_us)
            .bind(session_id.trim())
            .execute(&self.db_pool)
            .await;
    }

    /// Evaluates whether `ev` is authorized to be viewed by `(user_id, role)` under Satya's visibility rules.
    pub fn is_chat_event_visible_to(
        ev: &ChatMessageEvent,
        user_id: &str,
        role: SessionRole,
        has_human_gm: bool,
    ) -> bool {
        match ev.visibility {
            MessageVisibility::Public => true,
            MessageVisibility::Private => {
                ev.sender.user_id == user_id
                    || ev.recipients.iter().any(|r| r.user_id == user_id)
            }
            MessageVisibility::GmOnly => {
                ev.sender.user_id == user_id
                    || role == SessionRole::Gm
                    || (!has_human_gm && ev.recipients.iter().any(|r| r.user_id == user_id))
            }
            MessageVisibility::HostOnly => {
                ev.sender.user_id == user_id
                    || role == SessionRole::Host
                    || ev.recipients.iter().any(|r| r.user_id == user_id)
            }
        }
    }

    /// Queries SQLite for all historical chat and mechanics events in `session_id` visible to `participant`.
    pub async fn load_visible_events_from_db(
        &self,
        session_id: &str,
        user_id: &str,
        role: SessionRole,
        has_human_gm: bool,
    ) -> (Vec<ChatMessageEvent>, Vec<TtrpgMechanicsEvent>) {
        let rows = sqlx::query(
            r#"
            SELECT event_kind, payload_json
            FROM session_event_history
            WHERE session_id = ?1
            ORDER BY sequence_num ASC, timestamp_us ASC
            "#,
        )
        .bind(session_id.trim())
        .fetch_all(&self.db_pool)
        .await
        .unwrap_or_default();

        let mut chat_events = Vec::new();
        let mut mechanics_events = Vec::new();
        for row in rows {
            let Ok(kind) = row.try_get::<String, _>("event_kind") else {
                continue;
            };
            let Ok(payload_json) = row.try_get::<String, _>("payload_json") else {
                continue;
            };
            if kind == "CHAT" {
                if let Ok(ev) = serde_json::from_str::<ChatMessageEvent>(&payload_json)
                    && Self::is_chat_event_visible_to(&ev, user_id, role, has_human_gm)
                {
                    chat_events.push(ev);
                }
            } else if kind == "MECHANICS"
                && let Ok(ev) = serde_json::from_str::<TtrpgMechanicsEvent>(&payload_json)
            {
                mechanics_events.push(ev);
            }
        }

        (chat_events, mechanics_events)
    }

    /// Checks whether any human GM is registered in `session_id` (active roster or `session_participants`).
    pub(crate) async fn session_has_human_gm(&self, session_id: &str) -> bool {
        let sid = session_id.trim();
        if let Some(session) = self.sessions.read().await.get(sid)
            && session
                .roster
                .values()
                .any(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm)
        {
            return true;
        }
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM session_participants WHERE session_id = ?1 AND role = 'GM'",
        )
        .bind(sid)
        .fetch_one(&self.db_pool)
        .await
        .unwrap_or(0);
        count > 0
    }

    /// Builds a visibility-filtered [`SessionHistorySnapshot`] for `participant` entering or resuming `session_id`.
    pub async fn build_session_history_snapshot(
        &self,
        session_id: &str,
        participant: &Participant,
    ) -> SessionHistorySnapshot {
        let sid = session_id.trim();
        let has_human_gm = self.session_has_human_gm(sid).await;
        let (chat_events, mechanics_events) = self
            .load_visible_events_from_db(sid, &participant.user_id, participant.role, has_human_gm)
            .await;

        let (active_map, combat_tracker) = {
            let guard = self.sessions.read().await;
            if let Some(s) = guard.get(sid) {
                (s.active_map.clone(), Some(s.combat_tracker.clone()))
            } else {
                (None, None)
            }
        };

        let bound_character = if let Some(cid) = participant.character_id.as_deref() {
            sqlx::query_scalar::<_, String>(
                "SELECT sheet_json FROM characters WHERE character_id = ?1",
            )
            .bind(cid)
            .fetch_optional(&self.db_pool)
            .await
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<CharacterSheetRecord>(&raw).ok())
        } else {
            None
        };

        SessionHistorySnapshot {
            chat_events,
            mechanics_events,
            active_map,
            combat_tracker,
            bound_character,
        }
    }

    /// Lists all saved sessions in which `user_id` has participated or hosted, ordered by `updated_at_us DESC`.
    pub async fn list_saved_sessions_for_user(&self, user_id: &str) -> Vec<SavedSessionSummary> {
        let rows = sqlx::query(
            r#"
            SELECT s.session_id,
                   s.session_mode,
                   s.host_user_id,
                   s.campaign_tag,
                   s.enable_ai_gm,
                   s.created_at_us,
                   s.updated_at_us,
                   COALESCE(sp.role, CASE WHEN s.session_mode = 'MEETING' THEN 'HOST' ELSE 'GM' END) AS user_role,
                   COALESCE(sp.display_name, '') AS user_display_name,
                   sp.character_id AS user_character_id,
                   (SELECT COUNT(*) FROM session_event_history h WHERE h.session_id = s.session_id AND h.event_kind = 'CHAT') AS chat_count,
                   (SELECT COUNT(*) FROM session_event_history h WHERE h.session_id = s.session_id AND h.event_kind = 'MECHANICS') AS mech_count
            FROM saved_sessions s
            LEFT JOIN session_participants sp
                ON sp.session_id = s.session_id AND sp.user_id = ?1
            WHERE s.host_user_id = ?1 OR sp.user_id IS NOT NULL
            ORDER BY s.updated_at_us DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await
        .unwrap_or_default();

        let active_guard = self.sessions.read().await;
        let mut summaries = Vec::with_capacity(rows.len());
        for row in rows {
            let Ok(sid) = row.try_get::<String, _>("session_id") else {
                continue;
            };
            let mode_str: String = row.try_get("session_mode").unwrap_or_default();
            let role_str: String = row.try_get("user_role").unwrap_or_default();
            let active_human_count = active_guard
                .get(&sid)
                .map(|s| {
                    s.roster
                        .values()
                        .filter(|p| p.participant_kind == ParticipantKind::Human)
                        .count()
                })
                .unwrap_or(0);

            summaries.push(SavedSessionSummary {
                session_id: sid,
                session_mode: session_mode_from_db(&mode_str),
                role: session_role_from_db(&role_str),
                display_name: row.try_get("user_display_name").unwrap_or_default(),
                host_user_id: row.try_get("host_user_id").unwrap_or_default(),
                campaign_tag: row.try_get("campaign_tag").unwrap_or_default(),
                has_ai_gm: row.try_get::<i64, _>("enable_ai_gm").unwrap_or(0) != 0,
                character_id: row.try_get("user_character_id").ok().flatten(),
                chat_event_count: row.try_get::<i64, _>("chat_count").unwrap_or(0).max(0) as usize,
                mechanics_event_count: row.try_get::<i64, _>("mech_count").unwrap_or(0).max(0)
                    as usize,
                active_human_count,
                created_at_us: row.try_get("created_at_us").unwrap_or(0),
                updated_at_us: row.try_get("updated_at_us").unwrap_or(0),
            });
        }
        summaries
    }

    /// Resumes a saved or active session for `user`, rehydrating the session from SQLite if needed,
    /// verifying authorization (via previous participation/host identity, `join_token`, or `session_password`),
    /// and returning `(join_token, session_mode, self_participant, roster, history_snapshot)`.
    pub async fn resume_session(
        &self,
        user: &UserAccount,
        req: &ResumeSessionRequest,
    ) -> anyhow::Result<(
        String,
        SessionMode,
        Participant,
        Vec<Participant>,
        SessionHistorySnapshot,
    )> {
        let sid = req.session_id.trim();
        if sid.is_empty() {
            anyhow::bail!("session_id cannot be empty");
        }

        if !self.sessions.read().await.contains_key(sid) {
            self.rehydrate_session_from_db(sid)
                .await
                .ok_or_else(|| anyhow::anyhow!("Saved session '{sid}' does not exist"))?;
        }

        let saved_part_row = sqlx::query(
            "SELECT display_name, role, character_id FROM session_participants WHERE session_id = ?1 AND user_id = ?2",
        )
        .bind(sid)
        .bind(&user.user_id)
        .fetch_optional(&self.db_pool)
        .await
        .ok()
        .flatten();

        let saved_display_name: Option<String> = saved_part_row
            .as_ref()
            .and_then(|r| r.try_get::<String, _>("display_name").ok())
            .filter(|s| !s.trim().is_empty());
        let saved_role: Option<SessionRole> = saved_part_row
            .as_ref()
            .and_then(|r| r.try_get::<String, _>("role").ok())
            .map(|s| session_role_from_db(&s));
        let saved_cid: Option<String> = saved_part_row
            .as_ref()
            .and_then(|r| r.try_get::<Option<String>, _>("character_id").ok())
            .flatten()
            .filter(|s| !s.trim().is_empty());

        let has_valid_join_token = if let Some(jtok) = req
            .join_token
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            self.join_tokens
                .read()
                .await
                .get(jtok)
                .map(|c| c.session_id == sid && c.participant.user_id == user.user_id)
                .unwrap_or(false)
        } else {
            false
        };

        let (mode, self_participant, roster_vec) = {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;

            let is_known_member = session.host_user_id == user.user_id
                || saved_part_row.is_some()
                || session.roster.contains_key(&user.user_id);

            if let Some(pwd) = req
                .session_password
                .as_deref()
                .filter(|s| !s.is_empty())
            {
                if !Self::verify_secret(pwd, &session.password_hash) {
                    anyhow::bail!("Invalid session password for session '{sid}'");
                }
            } else if !has_valid_join_token && !is_known_member {
                anyhow::bail!(
                    "Session password is required to join session '{sid}' for the first time"
                );
            }

            let resolved_role = req.role.or(saved_role).unwrap_or_else(|| {
                if session.host_user_id == user.user_id {
                    match session.session_mode {
                        SessionMode::Ttrpg => SessionRole::Gm,
                        SessionMode::Meeting => SessionRole::Host,
                    }
                } else {
                    match session.session_mode {
                        SessionMode::Ttrpg => SessionRole::Player,
                        SessionMode::Meeting => SessionRole::Participant,
                    }
                }
            });

            let resolved_display_name = req
                .display_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .or(saved_display_name)
                .unwrap_or_else(|| user.display_name.clone());

            let mut self_participant = Participant::human(
                &user.user_id,
                &user.username,
                resolved_display_name,
                resolved_role,
            );
            if let Some(cid) = saved_cid {
                self_participant.character_id = Some(cid);
            }

            session
                .roster
                .insert(self_participant.user_id.clone(), self_participant.clone());
            Self::sync_combatant_into_tracker(&mut session.combat_tracker, &self_participant);
            let roster_vec: Vec<Participant> = session.roster.values().cloned().collect();
            (session.session_mode, self_participant, roster_vec)
        };

        self.persist_session_participant_to_db(sid, &self_participant)
            .await;

        let join_token = format!("join_{}", Uuid::new_v4());
        self.join_tokens.write().await.insert(
            join_token.clone(),
            JoinTokenClaims {
                session_id: sid.to_string(),
                session_mode: mode,
                participant: self_participant.clone(),
            },
        );

        let history = self
            .build_session_history_snapshot(sid, &self_participant)
            .await;

        Ok((join_token, mode, self_participant, roster_vec, history))
    }
}

/// Validates the caller's bearer token and returns their current [`AuthSuccessResponse`] at `GET /api/auth/me`.
pub async fn auth_me_handler(
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
    let clean = auth_hdr.strip_prefix("Bearer ").unwrap_or(auth_hdr).trim();
    let Some(user) = state.auth.verify_auth_token(clean).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };

    (
        StatusCode::OK,
        Json(json!(AuthSuccessResponse {
            auth_token: clean.to_string(),
            user_id: user.user_id,
            username: user.username,
            display_name: user.display_name,
            account_role: user.account_role,
        })),
    )
        .into_response()
}

/// Lists saved and resumable sessions for the authenticated user at `GET /api/sessions/saved`.
pub async fn list_saved_sessions_handler(
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

    let sessions = state.auth.list_saved_sessions_for_user(&user.user_id).await;
    (
        StatusCode::OK,
        Json(json!(SavedSessionsListResponse { sessions })),
    )
        .into_response()
}

/// Resumes a saved or active session at `POST /api/sessions/resume`.
pub async fn resume_session_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<ResumeSessionRequest>,
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

    match state.auth.resume_session(&user, &req).await {
        Ok((join_token, session_mode, self_participant, roster, history)) => {
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
                    reason: Some("Participant resumed saved session".to_string()),
                };
                let _ = state.storage.log_session_lifecycle(&ev).await;
            }

            if let Ok(sync_pkt) = serde_json::to_string(&ServerWsPacket::RosterSync {
                roster: roster.clone(),
            }) {
                state
                    .broadcast_to_session(&req.session_id, &sync_pkt)
                    .await;
            }

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

pub(crate) fn session_mode_to_db(mode: SessionMode) -> &'static str {
    match mode {
        SessionMode::Ttrpg => "TTRPG",
        SessionMode::Meeting => "MEETING",
    }
}

pub(crate) fn session_mode_from_db(raw: &str) -> SessionMode {
    match raw.trim().to_uppercase().as_str() {
        "MEETING" => SessionMode::Meeting,
        _ => SessionMode::Ttrpg,
    }
}

pub(crate) fn session_role_to_db(role: SessionRole) -> &'static str {
    match role {
        SessionRole::Gm => "GM",
        SessionRole::AiGm => "AI_GM",
        SessionRole::Player => "PLAYER",
        SessionRole::Host => "HOST",
        SessionRole::Participant => "PARTICIPANT",
        SessionRole::Observer => "OBSERVER",
    }
}

pub(crate) fn session_role_from_db(raw: &str) -> SessionRole {
    match raw.trim().to_uppercase().as_str() {
        "GM" => SessionRole::Gm,
        "AI_GM" => SessionRole::AiGm,
        "HOST" => SessionRole::Host,
        "PARTICIPANT" => SessionRole::Participant,
        "OBSERVER" => SessionRole::Observer,
        _ => SessionRole::Player,
    }
}

pub(crate) fn visibility_to_db(vis: MessageVisibility) -> &'static str {
    match vis {
        MessageVisibility::Public => "PUBLIC",
        MessageVisibility::Private => "PRIVATE",
        MessageVisibility::GmOnly => "GM_ONLY",
        MessageVisibility::HostOnly => "HOST_ONLY",
    }
}
