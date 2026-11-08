//! Persistent SQLite GM Memory Vault (`gm_documents`, `gm_memories`, `session_combat_state`)
//! and Unified GM Omnisearch (`GET /api/sessions/:session_id/lookup`) for both Human GM and AI GM.

use crate::auth::{AuthAndSessionManager, UserAccount};
use crate::gm_api::verify_gm_service_header;
use crate::gm_docs::sanitize_and_guard_gm_document;
use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use satya_schema::{
    CharacterSheetRecord, CombatTrackerState, CombatantTickEntry, GmDocumentRecord,
    GmDocumentSource, GmLookupHit, GmLookupSourceKind, GmMemoryCategory, GmMemoryNoteRecord,
    GmOmnisearchResponse, MessageVisibility, Participant, ParticipantKind, ServerWsPacket,
    SessionGmMemoriesResponse, SessionRole, ToggleGmDocumentPersistenceRequest,
    UpdateCombatTrackerRequest, UpsertGmMemoryRequest,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

fn source_kind_to_str(src: GmDocumentSource) -> &'static str {
    match src {
        GmDocumentSource::DirectTextUpload => "DIRECT_TEXT_UPLOAD",
        GmDocumentSource::GoogleDoc => "GOOGLE_DOC",
        GmDocumentSource::GoogleSheet => "GOOGLE_SHEET",
    }
}

fn source_kind_from_str(raw: &str) -> GmDocumentSource {
    match raw.trim().to_uppercase().as_str() {
        "GOOGLE_DOC" => GmDocumentSource::GoogleDoc,
        "GOOGLE_SHEET" => GmDocumentSource::GoogleSheet,
        _ => GmDocumentSource::DirectTextUpload,
    }
}

impl AuthAndSessionManager {
    /// Persists or updates a [`GmDocumentRecord`] in the SQLite `gm_documents` table.
    pub async fn persist_gm_doc_to_db(&self, doc: &GmDocumentRecord) -> anyhow::Result<()> {
        let tag = doc
            .campaign_tag
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&doc.session_id);
        sqlx::query(
            r#"
            INSERT INTO gm_documents (
                doc_id, session_id, campaign_tag, uploader_user_id, uploader_display_name,
                title, source_kind, google_file_id, sanitized_content, char_count,
                safety_flagged, created_at_us
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(doc_id) DO UPDATE SET
                campaign_tag = excluded.campaign_tag,
                title = excluded.title,
                sanitized_content = excluded.sanitized_content,
                char_count = excluded.char_count
            "#,
        )
        .bind(&doc.doc_id)
        .bind(&doc.session_id)
        .bind(tag)
        .bind(&doc.uploader_user_id)
        .bind(&doc.uploader_display_name)
        .bind(&doc.title)
        .bind(source_kind_to_str(doc.source_kind))
        .bind(&doc.google_file_id)
        .bind(&doc.sanitized_content)
        .bind(doc.char_count as i64)
        .bind(i64::from(doc.safety_flagged))
        .bind(doc.created_at_us)
        .execute(self.db_pool())
        .await?;
        Ok(())
    }

    /// Removes a [`GmDocumentRecord`] from the SQLite `gm_documents` table when unmarked as persistent.
    pub async fn delete_gm_doc_from_db(&self, doc_id: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM gm_documents WHERE doc_id = ?1")
            .bind(doc_id)
            .execute(self.db_pool())
            .await?;
        Ok(())
    }

    /// Loads all persistent [`GmDocumentRecord`]s matching `session_or_campaign_tag` from SQLite.
    pub async fn load_persistent_gm_docs_from_db(
        &self,
        session_or_campaign_tag: &str,
    ) -> Vec<GmDocumentRecord> {
        let key = session_or_campaign_tag.trim();
        let Ok(rows) = sqlx::query(
            r#"
            SELECT doc_id, session_id, campaign_tag, uploader_user_id, uploader_display_name,
                   title, source_kind, google_file_id, sanitized_content, char_count,
                   safety_flagged, created_at_us
            FROM gm_documents
            WHERE session_id = ?1 OR campaign_tag = ?1
            ORDER BY created_at_us ASC
            "#,
        )
        .bind(key)
        .fetch_all(self.db_pool())
        .await
        else {
            return Vec::new();
        };

        rows.into_iter()
            .filter_map(|r| {
                let src_raw: String = r.try_get("source_kind").ok()?;
                let flagged: i64 = r.try_get("safety_flagged").unwrap_or(0);
                let char_cnt: i64 = r.try_get("char_count").unwrap_or(0);
                let ctag: String = r.try_get("campaign_tag").ok()?;
                Some(GmDocumentRecord {
                    doc_id: r.try_get("doc_id").ok()?,
                    session_id: r.try_get("session_id").ok()?,
                    uploader_user_id: r.try_get("uploader_user_id").ok()?,
                    uploader_display_name: r.try_get("uploader_display_name").ok()?,
                    title: r.try_get("title").ok()?,
                    source_kind: source_kind_from_str(&src_raw),
                    google_file_id: r.try_get("google_file_id").ok().flatten(),
                    visibility: MessageVisibility::GmOnly,
                    sanitized_content: r.try_get("sanitized_content").ok()?,
                    char_count: char_cnt.max(0) as usize,
                    safety_flagged: flagged != 0,
                    is_persistent: true,
                    campaign_tag: Some(ctag),
                    created_at_us: r.try_get("created_at_us").unwrap_or(0),
                })
            })
            .collect()
    }

    /// Toggles `is_persistent` (and optional `campaign_tag`) on an existing `GM_ONLY` document in `session_id`.
    /// Authorized for the session's Human GM/Host or the document's original uploader.
    pub async fn toggle_session_gm_document_persistence(
        &self,
        session_id: &str,
        caller_user_id: &str,
        doc_id: &str,
        is_persistent: bool,
        campaign_tag: Option<String>,
    ) -> anyhow::Result<(GmDocumentRecord, Vec<String>)> {
        let sid = session_id.trim();
        let (updated_doc, delivery_ids) = {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;
            let caller = session
                .roster
                .get(caller_user_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Caller is not an active participant in '{sid}'"))?;
            let is_gm = matches!(caller.role, SessionRole::Gm | SessionRole::Host);

            if let Some(tag_override) = campaign_tag
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                && is_gm
            {
                session.campaign_tag = tag_override.to_string();
            }
            let session_ctag = session.campaign_tag.clone();

            let doc = session
                .gm_documents
                .iter_mut()
                .find(|d| d.doc_id == doc_id.trim())
                .ok_or_else(|| anyhow::anyhow!("Document '{doc_id}' not found in '{sid}'"))?;

            if !is_gm && doc.uploader_user_id != caller_user_id {
                anyhow::bail!("Only the session GM or document uploader can change persistence");
            }

            doc.is_persistent = is_persistent;
            if let Some(tag) = campaign_tag.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                doc.campaign_tag = Some(tag);
            } else if doc.campaign_tag.is_none() {
                doc.campaign_tag = Some(session_ctag);
            }
            let updated = doc.clone();

            let mut recipients: Vec<String> = session
                .roster
                .values()
                .filter(|p| {
                    p.participant_kind == ParticipantKind::Human
                        && (matches!(p.role, SessionRole::Gm | SessionRole::Host)
                            || p.user_id == updated.uploader_user_id)
                })
                .map(|p| p.user_id.clone())
                .collect();
            recipients.sort();
            recipients.dedup();
            (updated, recipients)
        };

        if updated_doc.is_persistent {
            self.persist_gm_doc_to_db(&updated_doc).await?;
        } else {
            self.delete_gm_doc_from_db(&updated_doc.doc_id).await?;
        }

        Ok((updated_doc, delivery_ids))
    }

    /// Loads all persistent [`GmMemoryNoteRecord`]s matching `session_or_campaign_tag` from SQLite.
    pub async fn load_persistent_gm_memories_from_db(
        &self,
        session_or_campaign_tag: &str,
    ) -> Vec<GmMemoryNoteRecord> {
        let key = session_or_campaign_tag.trim();
        let Ok(rows) = sqlx::query(
            r#"
            SELECT memory_id, session_id, campaign_tag, author_user_id, author_display_name,
                   category, title, content, is_pinned, is_persistent, updated_at_us
            FROM gm_memories
            WHERE session_id = ?1 OR campaign_tag = ?1
            ORDER BY is_pinned DESC, updated_at_us DESC
            "#,
        )
        .bind(key)
        .fetch_all(self.db_pool())
        .await
        else {
            return Vec::new();
        };

        rows.into_iter()
            .filter_map(|r| {
                let cat_raw: String = r.try_get("category").ok()?;
                let pinned: i64 = r.try_get("is_pinned").unwrap_or(0);
                let persistent: i64 = r.try_get("is_persistent").unwrap_or(1);
                Some(GmMemoryNoteRecord {
                    memory_id: r.try_get("memory_id").ok()?,
                    session_id: r.try_get("session_id").ok()?,
                    campaign_tag: r.try_get("campaign_tag").ok()?,
                    author_user_id: r.try_get("author_user_id").ok()?,
                    author_display_name: r.try_get("author_display_name").ok()?,
                    category: GmMemoryCategory::from_code_str(&cat_raw),
                    title: r.try_get("title").ok()?,
                    content: r.try_get("content").ok()?,
                    is_pinned: pinned != 0,
                    is_persistent: persistent != 0,
                    updated_at_us: r.try_get("updated_at_us").unwrap_or(0),
                })
            })
            .collect()
    }

    /// Creates or updates a [`GmMemoryNoteRecord`] in `session_id` (and persists to SQLite when `is_persistent == true`).
    pub async fn upsert_session_gm_memory(
        &self,
        session_id: &str,
        author_user_id: &str,
        author_display_name: &str,
        is_gm_service: bool,
        req: UpsertGmMemoryRequest,
    ) -> anyhow::Result<GmMemoryNoteRecord> {
        let (clean_title, clean_content, _) =
            sanitize_and_guard_gm_document(&req.title, &req.content)
                .map_err(|e| anyhow::anyhow!(e))?;

        let sid = session_id.trim();
        let now_us = Utc::now().timestamp_micros();
        let mem_id = req
            .memory_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("mem_{}", Uuid::now_v7()));

        let record = {
            let mut guard = self.sessions.write().await;
            if let Some(session) = guard.get_mut(sid) {
                if !is_gm_service {
                    let caller = session.roster.get(author_user_id).ok_or_else(|| {
                        anyhow::anyhow!("Caller is not an active participant in '{sid}'")
                    })?;
                    let has_human_gm = session.roster.values().any(|p| {
                        p.participant_kind == ParticipantKind::Human
                            && matches!(p.role, SessionRole::Gm | SessionRole::Host)
                    });
                    if has_human_gm && !matches!(caller.role, SessionRole::Gm | SessionRole::Host) {
                        anyhow::bail!("Only the session Game Master can edit GM memory notes");
                    }
                }
                if let Some(ctag) = req
                    .campaign_tag
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    session.campaign_tag = ctag.to_string();
                }
                let rec = GmMemoryNoteRecord {
                    memory_id: mem_id.clone(),
                    session_id: sid.to_string(),
                    campaign_tag: session.campaign_tag.clone(),
                    author_user_id: author_user_id.to_string(),
                    author_display_name: author_display_name.to_string(),
                    category: req.category,
                    title: clean_title,
                    content: clean_content,
                    is_pinned: req.is_pinned,
                    is_persistent: req.is_persistent,
                    updated_at_us: now_us,
                };
                if let Some(existing) = session
                    .gm_memories
                    .iter_mut()
                    .find(|m| m.memory_id == rec.memory_id || m.title.eq_ignore_ascii_case(&rec.title))
                {
                    *existing = rec.clone();
                } else {
                    session.gm_memories.push(rec.clone());
                }
                rec
            } else if is_gm_service {
                let ctag = req
                    .campaign_tag
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| sid.to_string());
                GmMemoryNoteRecord {
                    memory_id: mem_id,
                    session_id: sid.to_string(),
                    campaign_tag: ctag,
                    author_user_id: author_user_id.to_string(),
                    author_display_name: author_display_name.to_string(),
                    category: req.category,
                    title: clean_title,
                    content: clean_content,
                    is_pinned: req.is_pinned,
                    is_persistent: req.is_persistent,
                    updated_at_us: now_us,
                }
            } else {
                anyhow::bail!("Session '{sid}' does not exist");
            }
        };

        if record.is_persistent {
            sqlx::query(
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
            .bind(&record.memory_id)
            .bind(&record.session_id)
            .bind(&record.campaign_tag)
            .bind(&record.author_user_id)
            .bind(&record.author_display_name)
            .bind(record.category.as_str())
            .bind(&record.title)
            .bind(&record.content)
            .bind(i64::from(record.is_pinned))
            .bind(i64::from(record.is_persistent))
            .bind(record.updated_at_us)
            .execute(self.db_pool())
            .await?;
        } else {
            let _ = sqlx::query("DELETE FROM gm_memories WHERE memory_id = ?1")
                .bind(&record.memory_id)
                .execute(self.db_pool())
                .await;
        }

        Ok(record)
    }

    /// Lists GM memory notes for `session_id` (enforcing GM/AI-GM visibility when a human GM is present).
    pub async fn list_session_gm_memories(
        &self,
        session_id: &str,
        caller_user_id: Option<&str>,
        is_gm_service: bool,
    ) -> anyhow::Result<(String, Vec<GmMemoryNoteRecord>)> {
        let sid = session_id.trim();
        let guard = self.sessions.read().await;
        if let Some(session) = guard.get(sid) {
            if !is_gm_service {
                let uid = caller_user_id.unwrap_or("");
                let caller = session
                    .roster
                    .get(uid)
                    .ok_or_else(|| anyhow::anyhow!("Caller is not in session '{sid}'"))?;
                let has_human_gm = session.roster.values().any(|p| {
                    p.participant_kind == ParticipantKind::Human
                        && matches!(p.role, SessionRole::Gm | SessionRole::Host)
                });
                if has_human_gm && !matches!(caller.role, SessionRole::Gm | SessionRole::Host) {
                    return Ok((session.campaign_tag.clone(), Vec::new()));
                }
            }
            return Ok((session.campaign_tag.clone(), session.gm_memories.clone()));
        }
        drop(guard);

        if is_gm_service {
            let db_mems = self.load_persistent_gm_memories_from_db(sid).await;
            return Ok((sid.to_string(), db_mems));
        }
        anyhow::bail!("Session '{sid}' does not exist")
    }

    /// Deletes a GM memory note by `memory_id` from both in-memory session state and SQLite.
    pub async fn delete_session_gm_memory(
        &self,
        session_id: &str,
        caller_user_id: Option<&str>,
        is_gm_service: bool,
        memory_id: &str,
    ) -> anyhow::Result<bool> {
        let sid = session_id.trim();
        let mid = memory_id.trim();
        {
            let mut guard = self.sessions.write().await;
            if let Some(session) = guard.get_mut(sid) {
                if !is_gm_service {
                    let uid = caller_user_id.unwrap_or("");
                    let caller = session
                        .roster
                        .get(uid)
                        .ok_or_else(|| anyhow::anyhow!("Caller is not in session '{sid}'"))?;
                    let has_human_gm = session.roster.values().any(|p| {
                        p.participant_kind == ParticipantKind::Human
                            && matches!(p.role, SessionRole::Gm | SessionRole::Host)
                    });
                    if has_human_gm && !matches!(caller.role, SessionRole::Gm | SessionRole::Host) {
                        anyhow::bail!("Only the session Game Master can delete GM memory notes");
                    }
                }
                session.gm_memories.retain(|m| m.memory_id != mid);
            }
        }
        let res = sqlx::query("DELETE FROM gm_memories WHERE memory_id = ?1")
            .bind(mid)
            .execute(self.db_pool())
            .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Synchronizes a [`Participant`] into a [`CombatTrackerState`] if not already listed.
    pub fn sync_combatant_into_tracker(tracker: &mut CombatTrackerState, part: &Participant) {
        if tracker.combatants.iter().any(|c| c.entity_id == part.user_id) {
            return;
        }
        let idx = tracker.combatants.len() as i32;
        let ooda = (13 - idx).clamp(8, 16);
        let start_tick = satya_engine::compute_starting_tick(ooda, 0, 0, ooda + 1, 12) as u32;
        tracker.combatants.push(CombatantTickEntry {
            entity_id: part.user_id.clone(),
            display_name: part.display_name.clone(),
            role_or_faction: format!("{:?}", part.role),
            ooda_score: ooda,
            next_action_tick: tracker.tick.max(start_tick) + (idx as u32) * 2,
            active_stance: Some("NEUTRAL".to_string()),
            fatigue_level: 0,
            wound_shock: 0,
        });
    }

    /// Loads the persisted [`CombatTrackerState`] for `session_id` from SQLite, or initializes a new one.
    pub async fn load_or_init_combat_tracker(
        &self,
        session_id: &str,
        roster: &[Participant],
    ) -> CombatTrackerState {
        let sid = session_id.trim();
        if let Ok(Some(row)) =
            sqlx::query("SELECT state_json FROM session_combat_state WHERE session_id = ?1")
                .bind(sid)
                .fetch_optional(self.db_pool())
                .await
            && let Ok(json_str) = row.try_get::<String, _>("state_json")
            && let Ok(mut tracker) = serde_json::from_str::<CombatTrackerState>(&json_str)
        {
            for p in roster {
                Self::sync_combatant_into_tracker(&mut tracker, p);
            }
            return tracker;
        }

        let mut tracker = CombatTrackerState {
            session_id: sid.to_string(),
            combat_active: false,
            round: 1,
            tick: 0,
            combatants: Vec::new(),
            updated_at_us: Utc::now().timestamp_micros(),
        };
        for p in roster {
            Self::sync_combatant_into_tracker(&mut tracker, p);
        }
        tracker
    }

    /// Returns the authoritative [`CombatTrackerState`] for `session_id`.
    pub async fn get_session_combat_tracker(&self, session_id: &str) -> Option<CombatTrackerState> {
        let sid = session_id.trim();
        let guard = self.sessions.read().await;
        if let Some(session) = guard.get(sid) {
            return Some(session.combat_tracker.clone());
        }
        drop(guard);
        sqlx::query("SELECT state_json FROM session_combat_state WHERE session_id = ?1")
            .bind(sid)
            .fetch_optional(self.db_pool())
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get::<String, _>("state_json").ok())
            .and_then(|s| serde_json::from_str::<CombatTrackerState>(&s).ok())
    }

    /// Updates the authoritative [`CombatTrackerState`] for `session_id` and persists it to SQLite.
    pub async fn update_session_combat_tracker(
        &self,
        session_id: &str,
        req: UpdateCombatTrackerRequest,
    ) -> anyhow::Result<CombatTrackerState> {
        let sid = session_id.trim();
        let now_us = Utc::now().timestamp_micros();
        let updated = {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;
            let tracker = &mut session.combat_tracker;
            if let Some(active) = req.combat_active {
                tracker.combat_active = active;
            }
            if let Some(delta) = req.tick_delta {
                if delta <= 0 {
                    tracker.round = 1;
                    tracker.tick = 1;
                } else {
                    tracker.tick += delta as u32;
                    tracker.round = 1 + (tracker.tick / 20);
                    tracker.combat_active = true;
                }
            }
            if let Some(ref eid) = req.entity_id
                && let Some(entry) = tracker
                    .combatants
                    .iter_mut()
                    .find(|c| c.entity_id == *eid || c.display_name.eq_ignore_ascii_case(eid))
            {
                if let Some(ref st) = req.active_stance {
                    entry.active_stance = Some(st.clone());
                }
                if let Some(nt) = req.next_action_tick {
                    entry.next_action_tick = nt;
                }
                if let Some(fat) = req.fatigue_level {
                    entry.fatigue_level = fat.max(0);
                }
                if let Some(ws) = req.wound_shock {
                    entry.wound_shock = ws.max(0);
                }
            }
            tracker.updated_at_us = now_us;
            tracker.clone()
        };

        if let Ok(json_str) = serde_json::to_string(&updated) {
            let _ = sqlx::query(
                r#"
                INSERT INTO session_combat_state (session_id, state_json, updated_at_us)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(session_id) DO UPDATE SET
                    state_json = excluded.state_json,
                    updated_at_us = excluded.updated_at_us
                "#,
            )
            .bind(sid)
            .bind(json_str)
            .bind(now_us)
            .execute(self.db_pool())
            .await;
        }

        Ok(updated)
    }

    /// Loads [`CharacterSheetRecord`]s from SQLite bound to `session_id` or owned by participants in `roster`.
    pub async fn load_session_characters_from_db(
        &self,
        _session_id: &str,
        roster: &[Participant],
    ) -> Vec<CharacterSheetRecord> {
        let Ok(rows) =
            sqlx::query("SELECT sheet_json FROM characters ORDER BY updated_at_us DESC LIMIT 50")
                .fetch_all(self.db_pool())
                .await
        else {
            return Vec::new();
        };

        let roster_user_ids: HashSet<&str> = roster.iter().map(|p| p.user_id.as_str()).collect();
        let roster_char_ids: HashSet<&str> = roster
            .iter()
            .filter_map(|p| p.character_id.as_deref())
            .collect();

        let mut sheets = Vec::new();
        for row in rows {
            let Ok(json_str) = row.try_get::<String, _>("sheet_json") else {
                continue;
            };
            let Ok(sheet) = serde_json::from_str::<CharacterSheetRecord>(&json_str) else {
                continue;
            };
            let matches_session = roster_char_ids.contains(sheet.character_id.as_str())
                || roster_user_ids.contains(sheet.owner_user_id.as_str());
            if matches_session {
                sheets.push(sheet);
            }
        }
        sheets
    }

    /// Executes a unified omnisearch across Rules Book chapters, GM Documents, GM Memories,
    /// bound Character Sheets, Combat Tracker, and Tactical Map for `session_id`.
    pub async fn search_session_gm_omnisearch(
        &self,
        session_id: &str,
        caller_user_id: Option<&str>,
        is_gm_service: bool,
        query: &str,
        scope: &str,
    ) -> anyhow::Result<GmOmnisearchResponse> {
        let sid = session_id.trim();
        let norm_scope = if scope.trim().is_empty() {
            "ALL".to_string()
        } else {
            scope.trim().to_uppercase()
        };
        let q_trim = query.trim();
        let keywords: Vec<String> = q_trim
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() >= 2)
            .map(str::to_lowercase)
            .collect();

        let (campaign_tag, roster, gm_docs, gm_memories, combat_tracker, active_map, caller_is_gm) = {
            let guard = self.sessions.read().await;
            if let Some(session) = guard.get(sid) {
                let roster_vec: Vec<Participant> = session.roster.values().cloned().collect();
                let has_human_gm = roster_vec.iter().any(|p| {
                    p.participant_kind == ParticipantKind::Human
                        && matches!(p.role, SessionRole::Gm | SessionRole::Host)
                });
                let is_gm = if is_gm_service {
                    true
                } else if let Some(uid) = caller_user_id {
                    let caller = session
                        .roster
                        .get(uid)
                        .ok_or_else(|| anyhow::anyhow!("Caller is not in session '{sid}'"))?;
                    matches!(caller.role, SessionRole::Gm | SessionRole::Host) || !has_human_gm
                } else {
                    false
                };
                (
                    session.campaign_tag.clone(),
                    roster_vec,
                    session.gm_documents.clone(),
                    session.gm_memories.clone(),
                    session.combat_tracker.clone(),
                    session.active_map.clone(),
                    is_gm,
                )
            } else if is_gm_service {
                let docs = self.load_persistent_gm_docs_from_db(sid).await;
                let mems = self.load_persistent_gm_memories_from_db(sid).await;
                let tracker = self.load_or_init_combat_tracker(sid, &[]).await;
                (
                    sid.to_string(),
                    Vec::new(),
                    docs,
                    mems,
                    tracker,
                    None,
                    true,
                )
            } else {
                anyhow::bail!("Session '{sid}' does not exist");
            }
        };

        let mut hits: Vec<GmLookupHit> = Vec::new();
        let score_text = |text: &str, title: &str| -> usize {
            if keywords.is_empty() {
                return 10;
            }
            let hay = format!("{} {}", title.to_lowercase(), text.to_lowercase());
            let title_low = title.to_lowercase();
            let mut s = 0usize;
            for kw in &keywords {
                if title_low.contains(kw) {
                    s += 25;
                }
                if hay.contains(kw) {
                    s += 12;
                }
            }
            s
        };

        // 1. Rules Book Search
        if matches!(norm_scope.as_str(), "ALL" | "RULES" | "RULES_BOOK") {
            let rule_query = if q_trim.is_empty() { "cascade" } else { q_trim };
            for r in satya_engine::search_rules_book(rule_query, 5) {
                let excerpt: String = r.snippet.chars().take(360).collect();
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::RulesBook,
                    item_id: r.chapter_id,
                    title: format!("{} — {}", r.chapter_title, r.section_heading),
                    subtitle: format!("Rules Book ({})", r.relative_path),
                    snippet: excerpt,
                    is_persistent: true,
                    score: r.score + 15,
                });
            }
        }

        // 2. GM Documents (Session + Persistent)
        if matches!(norm_scope.as_str(), "ALL" | "DOCS" | "GM_DOC" | "GM_DOCS") {
            for doc in &gm_docs {
                if !caller_is_gm && Some(doc.uploader_user_id.as_str()) != caller_user_id {
                    continue;
                }
                let sc = score_text(&doc.sanitized_content, &doc.title);
                if sc > 0 {
                    let excerpt: String = doc.sanitized_content.chars().take(420).collect();
                    hits.push(GmLookupHit {
                        source_kind: GmLookupSourceKind::GmDocument,
                        item_id: doc.doc_id.clone(),
                        title: doc.title.clone(),
                        subtitle: format!(
                            "GM Doc by {} · {} chars{}",
                            doc.uploader_display_name,
                            doc.char_count,
                            if doc.is_persistent { " · Persistent" } else { "" }
                        ),
                        snippet: excerpt,
                        is_persistent: doc.is_persistent,
                        score: sc + 20,
                    });
                }
            }
        }

        // 3. Persistent GM Memories
        if caller_is_gm && matches!(norm_scope.as_str(), "ALL" | "MEMORY" | "MEMORIES" | "GM_MEMORY")
        {
            for mem in &gm_memories {
                let sc = score_text(&mem.content, &mem.title);
                if sc > 0 {
                    hits.push(GmLookupHit {
                        source_kind: GmLookupSourceKind::GmMemory,
                        item_id: mem.memory_id.clone(),
                        title: mem.title.clone(),
                        subtitle: format!(
                            "GM Memory [{}] by {}{}",
                            mem.category.as_str(),
                            mem.author_display_name,
                            if mem.is_pinned { " · Pinned" } else { "" }
                        ),
                        snippet: mem.content.chars().take(420).collect(),
                        is_persistent: mem.is_persistent,
                        score: sc + if mem.is_pinned { 35 } else { 25 },
                    });
                }
            }
        }

        // 4. Bound Player Character Sheets
        if matches!(
            norm_scope.as_str(),
            "ALL" | "CHARACTER" | "CHARACTERS" | "CHARACTER_SHEET" | "SHEETS"
        ) {
            let sheets = self.load_session_characters_from_db(sid, &roster).await;
            for ch in sheets {
                if !caller_is_gm && Some(ch.owner_user_id.as_str()) != caller_user_id {
                    continue;
                }
                let top_skills: Vec<String> = ch
                    .skills
                    .iter()
                    .map(|(k, v)| format!("{k} N={v}"))
                    .take(8)
                    .collect();
                let summary = format!(
                    "Role: {} | Origin: {} | SPD={} FOR={} DEX={} INS={} FOC={} | Fatigue={} WoundShock={} | Skills: {} | Notes: {}",
                    ch.callsign_or_role,
                    ch.origin_label,
                    ch.characteristics.spd,
                    ch.characteristics.for_stat,
                    ch.characteristics.dex,
                    ch.characteristics.ins,
                    ch.characteristics.foc,
                    ch.fatigue_level,
                    ch.wound_shock_index,
                    top_skills.join(", "),
                    ch.notes.as_deref().unwrap_or("none")
                );
                let sc = score_text(&summary, &ch.character_name);
                if sc > 0 {
                    hits.push(GmLookupHit {
                        source_kind: GmLookupSourceKind::CharacterSheet,
                        item_id: ch.character_id.clone(),
                        title: format!("{} ({})", ch.character_name, ch.callsign_or_role),
                        subtitle: format!("Character Sheet · {:?}", ch.verification_status),
                        snippet: summary,
                        is_persistent: true,
                        score: sc + 22,
                    });
                }
            }
        }

        // 5. Combat Tracker & Tactical Map
        if matches!(
            norm_scope.as_str(),
            "ALL" | "COMBAT" | "COMBAT_TRACKER" | "MAP" | "TACTICAL_MAP"
        ) {
            let combat_summary = format!(
                "Window {} · Tick {} ({:.1}s) | Active: {} | Combatants: {}",
                combat_tracker.round,
                combat_tracker.tick,
                f64::from(combat_tracker.tick) * 0.5,
                combat_tracker.combat_active,
                combat_tracker
                    .combatants
                    .iter()
                    .map(|c| format!(
                        "{} (OODA {}, Next @ Tick {}, Stance: {})",
                        c.display_name,
                        c.ooda_score,
                        c.next_action_tick,
                        c.active_stance.as_deref().unwrap_or("NEUTRAL")
                    ))
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            let sc = score_text(&combat_summary, "Combat Tracker Initiative Tick Timeline");
            if sc > 0 || norm_scope.contains("COMBAT") {
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::CombatTracker,
                    item_id: format!("combat_{sid}"),
                    title: format!(
                        "Combat Tracker — Window {} · Tick {}",
                        combat_tracker.round, combat_tracker.tick
                    ),
                    subtitle: format!("{} Combatants on Timeline", combat_tracker.combatants.len()),
                    snippet: combat_summary,
                    is_persistent: true,
                    score: sc.max(15),
                });
            }

            if let Some(map) = active_map {
                let tok_str = map
                    .tokens
                    .iter()
                    .map(|t| format!("{} at ({},{})", t.label, t.x, t.y))
                    .collect::<Vec<_>>()
                    .join(", ");
                let map_summary = format!(
                    "Map: {} ({}x{}) | Tokens: {} | Notes: {}",
                    map.title,
                    map.grid_width,
                    map.grid_height,
                    if tok_str.is_empty() { "none" } else { &tok_str },
                    map.notes.as_deref().unwrap_or("none")
                );
                let map_sc = score_text(&map_summary, &map.title);
                if map_sc > 0 || norm_scope.contains("MAP") {
                    hits.push(GmLookupHit {
                        source_kind: GmLookupSourceKind::TacticalMap,
                        item_id: map.map_id,
                        title: format!("Tactical Map — {}", map.title),
                        subtitle: format!("{}x{} Grid", map.grid_width, map.grid_height),
                        snippet: map_summary,
                        is_persistent: false,
                        score: map_sc.max(15),
                    });
                }
            }
        }

        hits.sort_by_key(|a| std::cmp::Reverse(a.score));
        hits.truncate(15);

        Ok(GmOmnisearchResponse {
            session_id: sid.to_string(),
            campaign_tag,
            query: q_trim.to_string(),
            scope: norm_scope,
            hits,
        })
    }
}

async fn resolve_user_or_gm_service(
    state: &SignalingAppState,
    headers: &HeaderMap,
) -> Result<(Option<UserAccount>, bool), (StatusCode, Json<serde_json::Value>)> {
    if verify_gm_service_header(state, headers).is_ok() {
        return Ok((None, true));
    }
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
    Ok((Some(user), false))
}

/// Toggles `is_persistent` on a `GM_ONLY` document at `POST /api/sessions/:session_id/docs/:doc_id/persist`.
pub async fn toggle_session_gm_doc_persistence_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path((session_id, doc_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<ToggleGmDocumentPersistenceRequest>,
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
        .toggle_session_gm_document_persistence(
            &session_id,
            &user.user_id,
            &doc_id,
            req.is_persistent,
            req.campaign_tag,
        )
        .await
    {
        Ok((updated_doc, delivery_ids)) => {
            if let Ok(ws_pkt) = serde_json::to_string(&ServerWsPacket::GmDocumentAdded {
                document: Box::new(updated_doc.clone()),
            }) {
                state
                    .send_to_users(&session_id, &delivery_ids, &ws_pkt)
                    .await;
            }
            (StatusCode::OK, Json(updated_doc)).into_response()
        }
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Lists persistent & session GM memory notes at `GET /api/sessions/:session_id/memories` (and `/api/gm/sessions/:session_id/memories`).
pub async fn list_session_gm_memories_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user_opt, is_gm_service) = match resolve_user_or_gm_service(&state, &headers).await {
        Ok(v) => v,
        Err(err_resp) => return err_resp.into_response(),
    };
    let uid = user_opt.as_ref().map(|u| u.user_id.as_str());
    match state
        .auth
        .list_session_gm_memories(&session_id, uid, is_gm_service)
        .await
    {
        Ok((campaign_tag, memories)) => (
            StatusCode::OK,
            Json(SessionGmMemoriesResponse {
                session_id,
                campaign_tag,
                memories,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Upserts a persistent or session GM memory note at `POST /api/sessions/:session_id/memories` (and `/api/gm/sessions/:session_id/memories`).
pub async fn upsert_session_gm_memory_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<UpsertGmMemoryRequest>,
) -> impl IntoResponse {
    let (user_opt, is_gm_service) = match resolve_user_or_gm_service(&state, &headers).await {
        Ok(v) => v,
        Err(err_resp) => return err_resp.into_response(),
    };
    let (author_id, author_name) = match user_opt {
        Some(u) => (u.user_id, u.display_name),
        None => (
            format!("ai-gm-{session_id}"),
            "Satya AI GM".to_string(),
        ),
    };
    match state
        .auth
        .upsert_session_gm_memory(&session_id, &author_id, &author_name, is_gm_service, req)
        .await
    {
        Ok(record) => (StatusCode::OK, Json(record)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Deletes a GM memory note at `DELETE /api/sessions/:session_id/memories/:memory_id`.
pub async fn delete_session_gm_memory_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path((session_id, memory_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user_opt, is_gm_service) = match resolve_user_or_gm_service(&state, &headers).await {
        Ok(v) => v,
        Err(err_resp) => return err_resp.into_response(),
    };
    let uid = user_opt.as_ref().map(|u| u.user_id.as_str());
    match state
        .auth
        .delete_session_gm_memory(&session_id, uid, is_gm_service, &memory_id)
        .await
    {
        Ok(deleted) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "memory_id": memory_id, "deleted": deleted })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Returns the authoritative Satya Count-Up Tick Combat Tracker state at `GET /api/sessions/:session_id/combat` (and `/api/gm/sessions/:session_id/combat`).
pub async fn get_session_combat_tracker_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err_resp) = resolve_user_or_gm_service(&state, &headers).await {
        return err_resp.into_response();
    }
    match state.auth.get_session_combat_tracker(&session_id).await {
        Some(tracker) => (StatusCode::OK, Json(tracker)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response(),
    }
}

/// Advances or updates the authoritative Satya Count-Up Tick Combat Tracker at `POST /api/sessions/:session_id/combat` (and `/api/gm/sessions/:session_id/combat`).
pub async fn update_session_combat_tracker_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<UpdateCombatTrackerRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = resolve_user_or_gm_service(&state, &headers).await {
        return err_resp.into_response();
    }
    match state
        .auth
        .update_session_combat_tracker(&session_id, req)
        .await
    {
        Ok(tracker) => (StatusCode::OK, Json(tracker)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Query parameters for `GET /api/sessions/:session_id/lookup` (and `/api/gm/sessions/:session_id/lookup`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GmLookupQueryParams {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub scope: String,
}

/// Unified GM Omnisearch handler (`GET /api/sessions/:session_id/lookup?q=...&scope=...`) for both Human GM UI lookup and AI GM RAG/MCP tools.
pub async fn gm_omnisearch_lookup_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    Query(params): Query<GmLookupQueryParams>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user_opt, is_gm_service) = match resolve_user_or_gm_service(&state, &headers).await {
        Ok(v) => v,
        Err(err_resp) => return err_resp.into_response(),
    };
    let uid = user_opt.as_ref().map(|u| u.user_id.as_str());
    match state
        .auth
        .search_session_gm_omnisearch(&session_id, uid, is_gm_service, &params.q, &params.scope)
        .await
    {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
