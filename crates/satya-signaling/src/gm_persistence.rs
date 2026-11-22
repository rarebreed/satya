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
    GmDocumentSource, GmLookupHit, GmLookupSourceKind, GmMapUpdateRequest, GmMemoryCategory,
    GmMemoryNoteRecord, GmOmnisearchResponse, MessageVisibility, Participant, ParticipantKind,
    ServerWsPacket, SessionGmMemoriesResponse, SessionRole, ToggleGmDocumentPersistenceRequest,
    TokenPlacement, UpdateCombatTrackerRequest, UpsertGmMemoryRequest,
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

/// Extracts a normalized NPC name from an auto-generated `NPC: <Name> (<Role>)` memory title.
pub(crate) fn extract_npc_name_from_memory_title(title: &str) -> Option<String> {
    let rest = title.trim().strip_prefix("NPC:")?;
    let name_part = rest.split('(').next().unwrap_or(rest).trim();
    if name_part.is_empty() {
        None
    } else {
        Some(name_part.to_ascii_lowercase())
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

    /// Loads persistent [`GmDocumentRecord`]s matching `session_or_campaign_tag` from SQLite,
    /// enforcing owner/host scoping when `owner_user_id` is supplied so unrelated GMs cannot inherit each other's docs.
    pub async fn load_persistent_gm_docs_from_db(
        &self,
        session_or_campaign_tag: &str,
        owner_user_id: Option<&str>,
    ) -> Vec<GmDocumentRecord> {
        let key = session_or_campaign_tag.trim();
        let query_res = if let Some(uid) = owner_user_id {
            sqlx::query(
                r#"
                SELECT doc_id, session_id, campaign_tag, uploader_user_id, uploader_display_name,
                       title, source_kind, google_file_id, sanitized_content, char_count,
                       safety_flagged, created_at_us
                FROM gm_documents
                WHERE (session_id = ?1 OR campaign_tag = ?1 COLLATE NOCASE)
                  AND (uploader_user_id = ?2 OR EXISTS(
                      SELECT 1 FROM saved_sessions s
                      WHERE s.session_id = gm_documents.session_id AND s.host_user_id = ?2
                  ))
                ORDER BY created_at_us ASC
                "#,
            )
            .bind(key)
            .bind(uid)
            .fetch_all(self.db_pool())
            .await
        } else {
            sqlx::query(
                r#"
                SELECT doc_id, session_id, campaign_tag, uploader_user_id, uploader_display_name,
                       title, source_kind, google_file_id, sanitized_content, char_count,
                       safety_flagged, created_at_us
                FROM gm_documents
                WHERE session_id = ?1 OR campaign_tag = ?1 COLLATE NOCASE
                ORDER BY created_at_us ASC
                "#,
            )
            .bind(key)
            .fetch_all(self.db_pool())
            .await
        };
        let Ok(rows) = query_res else {
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
            let caller =
                session.roster.get(caller_user_id).cloned().ok_or_else(|| {
                    anyhow::anyhow!("Caller is not an active participant in '{sid}'")
                })?;
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
            if let Some(tag) = campaign_tag
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
            {
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

    /// Deletes a `GM_ONLY` document from in-memory session state and SQLite `gm_documents`.
    /// Authorized for the session's Human GM/Host, Solo host, or original uploader.
    pub async fn delete_session_gm_document(
        &self,
        session_id: &str,
        caller_user_id: &str,
        doc_id: &str,
    ) -> anyhow::Result<bool> {
        let sid = session_id.trim();
        let did = doc_id.trim();
        let (removed_in_mem, is_gm, session_ctag, host_uid) = {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;
            let caller = session
                .roster
                .get(caller_user_id)
                .ok_or_else(|| anyhow::anyhow!("Caller is not an active participant in '{sid}'"))?;
            let is_gm = matches!(caller.role, SessionRole::Gm | SessionRole::Host);
            let ctag = session.campaign_tag.clone();
            let huid = session.host_user_id.clone();

            if let Some(idx) = session.gm_documents.iter().position(|d| d.doc_id == did) {
                if !is_gm && session.gm_documents[idx].uploader_user_id != caller_user_id {
                    anyhow::bail!(
                        "Only the session GM or document uploader can delete this document"
                    );
                }
                session.gm_documents.remove(idx);
                (true, is_gm, ctag, huid)
            } else {
                (false, is_gm, ctag, huid)
            }
        };

        if !removed_in_mem {
            let row = sqlx::query(
                "SELECT uploader_user_id, session_id, campaign_tag FROM gm_documents WHERE doc_id = ?1",
            )
            .bind(did)
            .fetch_optional(self.db_pool())
            .await?;
            let Some(row) = row else {
                return Ok(false);
            };
            let row_sid: String = row.try_get("session_id").unwrap_or_default();
            let row_ctag: String = row.try_get("campaign_tag").unwrap_or_default();
            let uploader_uid: String = row.try_get("uploader_user_id").unwrap_or_default();
            let owned_campaign = !session_ctag.is_empty()
                && row_ctag.eq_ignore_ascii_case(&session_ctag)
                && uploader_uid == host_uid;
            if row_sid != sid && !owned_campaign {
                anyhow::bail!("Document '{did}' does not belong to session '{sid}'");
            }
            if !is_gm && uploader_uid != caller_user_id {
                anyhow::bail!("Only the session GM or document uploader can delete this document");
            }
        }

        let res = sqlx::query(
            r#"
            DELETE FROM gm_documents
            WHERE doc_id = ?1
              AND (session_id = ?2 OR (campaign_tag = ?3 COLLATE NOCASE AND uploader_user_id = ?4))
            "#,
        )
        .bind(did)
        .bind(sid)
        .bind(&session_ctag)
        .bind(&host_uid)
        .execute(self.db_pool())
        .await?;
        Ok(removed_in_mem || res.rows_affected() > 0)
    }

    fn decode_gm_memory_row(r: &sqlx::sqlite::SqliteRow) -> Option<GmMemoryNoteRecord> {
        let cat_raw: String = r.try_get("category").ok()?;
        let pinned: i64 = r.try_get("is_pinned").unwrap_or(0);
        let persistent: i64 = r.try_get("is_persistent").unwrap_or(1);
        Some(GmMemoryNoteRecord {
            memory_id: r.try_get("memory_id").ok()?,
            session_id: r.try_get("session_id").ok()?,
            campaign_tag: r.try_get("campaign_tag").ok()?,
            author_user_id: r.try_get("author_user_id").ok()?,
            author_display_name: r.try_get("author_display_name").ok()?,
            category: GmMemoryCategory::parse_loose(&cat_raw),
            title: r.try_get("title").ok()?,
            content: r.try_get("content").ok()?,
            is_pinned: pinned != 0,
            is_persistent: persistent != 0,
            updated_at_us: r.try_get("updated_at_us").unwrap_or(0),
        })
    }

    /// Loads persistent [`GmMemoryNoteRecord`]s matching `session_or_campaign_tag` from SQLite,
    /// enforcing owner/host scoping when `owner_user_id` is supplied so unrelated GMs cannot inherit each other's notes.
    pub async fn load_persistent_gm_memories_from_db(
        &self,
        session_or_campaign_tag: &str,
        owner_user_id: Option<&str>,
    ) -> Vec<GmMemoryNoteRecord> {
        let key = session_or_campaign_tag.trim();
        let query_res = if let Some(uid) = owner_user_id {
            sqlx::query(
                r#"
                SELECT memory_id, session_id, campaign_tag, author_user_id, author_display_name,
                       category, title, content, is_pinned, is_persistent, updated_at_us
                FROM gm_memories
                WHERE (session_id = ?1 OR campaign_tag = ?1 COLLATE NOCASE)
                  AND (author_user_id = ?2 OR EXISTS(
                      SELECT 1 FROM saved_sessions s
                      WHERE s.session_id = gm_memories.session_id AND s.host_user_id = ?2
                  ))
                ORDER BY is_pinned DESC, updated_at_us DESC
                "#,
            )
            .bind(key)
            .bind(uid)
            .fetch_all(self.db_pool())
            .await
        } else {
            sqlx::query(
                r#"
                SELECT memory_id, session_id, campaign_tag, author_user_id, author_display_name,
                       category, title, content, is_pinned, is_persistent, updated_at_us
                FROM gm_memories
                WHERE session_id = ?1 OR campaign_tag = ?1 COLLATE NOCASE
                ORDER BY is_pinned DESC, updated_at_us DESC
                "#,
            )
            .bind(key)
            .fetch_all(self.db_pool())
            .await
        };
        let Ok(rows) = query_res else {
            return Vec::new();
        };

        let mut seen_npc_names: HashSet<String> = HashSet::new();
        rows.into_iter()
            .filter_map(|r| {
                let rec = Self::decode_gm_memory_row(&r)?;
                if rec.memory_id.starts_with("mem_npc_")
                    && let Some(npc_name) = extract_npc_name_from_memory_title(&rec.title)
                    && !seen_npc_names.insert(npc_name)
                {
                    return None;
                }
                Some(rec)
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
            .map(|id| {
                if id == "mem-scenario-director" {
                    format!("mem-scenario-director-{sid}")
                } else {
                    id
                }
            })
            .unwrap_or_else(|| format!("mem_{}", Uuid::now_v7()));

        let record = {
            let mut guard = self.sessions.write().await;
            if let Some(session) = guard.get_mut(sid) {
                if !is_gm_service {
                    let caller = session.roster.get(author_user_id).ok_or_else(|| {
                        anyhow::anyhow!("Caller is not an active participant in '{sid}'")
                    })?;
                    if !matches!(caller.role, SessionRole::Gm | SessionRole::Host) {
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
                if let Some(existing) = session.gm_memories.iter_mut().find(|m| {
                    m.memory_id == rec.memory_id || m.title.eq_ignore_ascii_case(&rec.title)
                }) {
                    *existing = rec.clone();
                } else {
                    session.gm_memories.push(rec.clone());
                }
                if rec.memory_id.starts_with("mem_npc_")
                    && let Some(npc_name) = extract_npc_name_from_memory_title(&rec.title)
                {
                    session.gm_memories.retain(|m| {
                        m.memory_id == rec.memory_id
                            || !m.memory_id.starts_with("mem_npc_")
                            || extract_npc_name_from_memory_title(&m.title).as_deref()
                                != Some(npc_name.as_str())
                    });
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
            self.persist_gm_memory_to_db(&record).await?;
        } else {
            let _ = sqlx::query("DELETE FROM gm_memories WHERE memory_id = ?1")
                .bind(&record.memory_id)
                .execute(self.db_pool())
                .await;
        }

        Ok(record)
    }

    async fn persist_gm_memory_to_db(&self, record: &GmMemoryNoteRecord) -> anyhow::Result<()> {
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
        Ok(())
    }

    async fn delete_duplicate_npc_memories_from_db(
        &self,
        session_id: &str,
        campaign_tag: &str,
        npc_name: &str,
        keep_memory_id: Option<&str>,
    ) {
        let Ok(rows) = sqlx::query(
            r#"
            SELECT memory_id, title FROM gm_memories
            WHERE memory_id LIKE 'mem_npc_%'
              AND (session_id = ?1 OR campaign_tag = ?1 COLLATE NOCASE OR campaign_tag = ?2 COLLATE NOCASE)
            "#,
        )
        .bind(session_id)
        .bind(campaign_tag)
        .fetch_all(self.db_pool())
        .await
        else {
            return;
        };

        for row in rows {
            let Ok(mid) = row.try_get::<String, _>("memory_id") else {
                continue;
            };
            if keep_memory_id == Some(mid.as_str()) {
                continue;
            }
            let Ok(title) = row.try_get::<String, _>("title") else {
                continue;
            };
            if extract_npc_name_from_memory_title(&title).as_deref() == Some(npc_name) {
                let _ = sqlx::query("DELETE FROM gm_memories WHERE memory_id = ?1")
                    .bind(&mid)
                    .execute(self.db_pool())
                    .await;
            }
        }
    }

    async fn sync_npc_persistence_from_memory(
        &self,
        memory_id: &str,
        title: &str,
        session_id: &str,
        campaign_tag: &str,
        is_persistent: bool,
    ) {
        if let Some(npc_id) = memory_id.strip_prefix("mem_") {
            let _ =
                sqlx::query("UPDATE session_npcs SET is_persistent = ?1 WHERE character_id = ?2")
                    .bind(i32::from(is_persistent))
                    .bind(npc_id)
                    .execute(self.db_pool())
                    .await;
            if memory_id.starts_with("mem_npc_")
                && let Some(npc_name) = extract_npc_name_from_memory_title(title)
            {
                let _ = sqlx::query(
                    "UPDATE session_npcs SET is_persistent = ?1 WHERE LOWER(TRIM(character_name)) = ?2 AND (session_id = ?3 OR campaign_tag = ?3 COLLATE NOCASE OR campaign_tag = ?4 COLLATE NOCASE)",
                )
                .bind(i32::from(is_persistent))
                .bind(&npc_name)
                .bind(session_id)
                .bind(campaign_tag)
                .execute(self.db_pool())
                .await;
            }
        }
    }

    /// Toggles `is_persistent` on a GM memory note in `session_id`, syncing SQLite `gm_memories`
    /// and any companion `session_npcs` entry when `memory_id` is `mem_<character_id>`.
    pub async fn toggle_session_gm_memory_persistence(
        &self,
        session_id: &str,
        caller_user_id: Option<&str>,
        is_gm_service: bool,
        memory_id: &str,
        is_persistent: bool,
        campaign_tag: Option<String>,
    ) -> anyhow::Result<GmMemoryNoteRecord> {
        let sid = session_id.trim();
        let mut mid = memory_id.trim().to_string();
        if mid == "mem-scenario-director" {
            mid = format!("mem-scenario-director-{sid}");
        }
        let now_us = Utc::now().timestamp_micros();
        if !self.sessions.read().await.contains_key(sid) {
            let _ = self.rehydrate_session_from_db(sid).await;
        }

        let (session_ctag, host_uid) = {
            let guard = self.sessions.read().await;
            if let Some(session) = guard.get(sid) {
                (session.campaign_tag.clone(), session.host_user_id.clone())
            } else if is_gm_service {
                (sid.to_string(), String::new())
            } else {
                anyhow::bail!("Session '{sid}' does not exist");
            }
        };

        let db_fallback = sqlx::query(
            r#"
            SELECT memory_id, session_id, campaign_tag, author_user_id, author_display_name,
                   category, title, content, is_pinned, is_persistent, updated_at_us
            FROM gm_memories
            WHERE memory_id = ?1
              AND (session_id = ?2
                   OR (campaign_tag = ?3 COLLATE NOCASE AND (?4 = '' OR author_user_id = ?4)))
            "#,
        )
        .bind(&mid)
        .bind(sid)
        .bind(&session_ctag)
        .bind(&host_uid)
        .fetch_optional(self.db_pool())
        .await
        .ok()
        .flatten()
        .and_then(|r| Self::decode_gm_memory_row(&r));

        let record = {
            let mut guard = self.sessions.write().await;
            if let Some(session) = guard.get_mut(sid) {
                if !is_gm_service {
                    let uid = caller_user_id.unwrap_or("");
                    let caller = session
                        .roster
                        .get(uid)
                        .ok_or_else(|| anyhow::anyhow!("Caller is not in session '{sid}'"))?;
                    if !matches!(caller.role, SessionRole::Gm | SessionRole::Host) {
                        anyhow::bail!(
                            "Only the session Game Master can modify GM memory persistence"
                        );
                    }
                }
                let tag_override = campaign_tag
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string);

                let mem_idx = session.gm_memories.iter().position(|m| m.memory_id == mid);
                let updated = if let Some(idx) = mem_idx {
                    let mem = &mut session.gm_memories[idx];
                    mem.is_persistent = is_persistent;
                    mem.updated_at_us = now_us;
                    if let Some(tag) = tag_override {
                        mem.campaign_tag = tag;
                    }
                    mem.clone()
                } else if let Some(mut mem) = db_fallback {
                    mem.is_persistent = is_persistent;
                    mem.updated_at_us = now_us;
                    if let Some(tag) = tag_override {
                        mem.campaign_tag = tag;
                    }
                    session.gm_memories.push(mem.clone());
                    mem
                } else {
                    anyhow::bail!("GM memory note '{mid}' not found in '{sid}'");
                };

                if updated.memory_id.starts_with("mem_npc_")
                    && let Some(npc_name) = extract_npc_name_from_memory_title(&updated.title)
                {
                    session.gm_memories.retain(|m| {
                        m.memory_id == updated.memory_id
                            || !m.memory_id.starts_with("mem_npc_")
                            || extract_npc_name_from_memory_title(&m.title).as_deref()
                                != Some(npc_name.as_str())
                    });
                }
                updated
            } else if is_gm_service {
                let mut mem = db_fallback.ok_or_else(|| {
                    anyhow::anyhow!("GM memory note '{mid}' not found in '{sid}'")
                })?;
                mem.is_persistent = is_persistent;
                mem.updated_at_us = now_us;
                if let Some(tag) = campaign_tag
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    mem.campaign_tag = tag.to_string();
                }
                mem
            } else {
                anyhow::bail!("Session '{sid}' does not exist");
            }
        };

        if record.is_persistent {
            self.persist_gm_memory_to_db(&record).await?;
        } else {
            let _ = sqlx::query(
                r#"
                DELETE FROM gm_memories
                WHERE memory_id = ?1
                  AND (session_id = ?2 OR (campaign_tag = ?3 COLLATE NOCASE AND author_user_id = ?4))
                "#,
            )
            .bind(&record.memory_id)
            .bind(sid)
            .bind(&session_ctag)
            .bind(&record.author_user_id)
            .execute(self.db_pool())
            .await;
            if record.memory_id.starts_with("mem_npc_")
                && let Some(npc_name) = extract_npc_name_from_memory_title(&record.title)
            {
                self.delete_duplicate_npc_memories_from_db(
                    sid,
                    &record.campaign_tag,
                    &npc_name,
                    None,
                )
                .await;
            }
        }

        self.sync_npc_persistence_from_memory(
            &record.memory_id,
            &record.title,
            sid,
            &record.campaign_tag,
            is_persistent,
        )
        .await;

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
                if !matches!(caller.role, SessionRole::Gm | SessionRole::Host) {
                    return Ok((session.campaign_tag.clone(), Vec::new()));
                }
            }
            return Ok((session.campaign_tag.clone(), session.gm_memories.clone()));
        }
        drop(guard);

        if is_gm_service {
            let db_mems = self.load_persistent_gm_memories_from_db(sid, None).await;
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
        let mut mid = memory_id.trim().to_string();
        if mid == "mem-scenario-director" {
            mid = format!("mem-scenario-director-{sid}");
        }
        if !self.sessions.read().await.contains_key(sid) {
            let _ = self.rehydrate_session_from_db(sid).await;
        }
        let (removed_in_mem, mem_meta, session_ctag, host_uid) = {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;
            if !is_gm_service {
                let uid = caller_user_id.unwrap_or("");
                let caller = session
                    .roster
                    .get(uid)
                    .ok_or_else(|| anyhow::anyhow!("Caller is not in session '{sid}'"))?;
                if !matches!(caller.role, SessionRole::Gm | SessionRole::Host) {
                    anyhow::bail!("Only the session Game Master can delete GM memory notes");
                }
            }
            let ctag = session.campaign_tag.clone();
            let huid = session.host_user_id.clone();
            let meta = session
                .gm_memories
                .iter()
                .find(|m| m.memory_id == mid)
                .map(|m| (m.title.clone(), m.campaign_tag.clone()));
            let prev_len = session.gm_memories.len();
            session.gm_memories.retain(|m| m.memory_id != mid);
            if mid.starts_with("mem_npc_")
                && let Some((ref title, _)) = meta
                && let Some(npc_name) = extract_npc_name_from_memory_title(title)
            {
                session.gm_memories.retain(|m| {
                    !m.memory_id.starts_with("mem_npc_")
                        || extract_npc_name_from_memory_title(&m.title).as_deref()
                            != Some(npc_name.as_str())
                });
            }
            (session.gm_memories.len() != prev_len, meta, ctag, huid)
        };

        let db_meta = if mem_meta.is_some() {
            mem_meta
        } else {
            sqlx::query(
                r#"
                SELECT title, campaign_tag FROM gm_memories
                WHERE memory_id = ?1
                  AND (session_id = ?2 OR (campaign_tag = ?3 COLLATE NOCASE AND author_user_id = ?4))
                "#,
            )
            .bind(&mid)
            .bind(sid)
            .bind(&session_ctag)
            .bind(&host_uid)
            .fetch_optional(self.db_pool())
            .await
            .ok()
            .flatten()
            .and_then(|r| {
                Some((
                    r.try_get::<String, _>("title").ok()?,
                    r.try_get::<String, _>("campaign_tag").ok()?,
                ))
            })
        };

        let res = sqlx::query(
            r#"
            DELETE FROM gm_memories
            WHERE memory_id = ?1
              AND (session_id = ?2 OR (campaign_tag = ?3 COLLATE NOCASE AND author_user_id = ?4))
            "#,
        )
        .bind(&mid)
        .bind(sid)
        .bind(&session_ctag)
        .bind(&host_uid)
        .execute(self.db_pool())
        .await?;

        if mid.starts_with("mem_npc_")
            && let Some((title, ctag)) = db_meta
            && let Some(npc_name) = extract_npc_name_from_memory_title(&title)
        {
            self.delete_duplicate_npc_memories_from_db(sid, &ctag, &npc_name, None)
                .await;
        }

        Ok(removed_in_mem || res.rows_affected() > 0)
    }

    /// Synchronizes a [`Participant`] into a [`CombatTrackerState`] if not already listed.
    pub fn sync_combatant_into_tracker(tracker: &mut CombatTrackerState, part: &Participant) {
        if part.role != SessionRole::Player {
            return;
        }
        if tracker
            .combatants
            .iter()
            .any(|c| c.entity_id == part.user_id)
        {
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
            grid_x: Some(4 + (idx % 4) * 2),
            grid_y: Some(6 + (idx / 4) * 2),
            zone_label: Some("Core Sector".to_string()),
            visibility: MessageVisibility::Public,
        });
    }

    /// Returns a role-filtered copy of [`CombatTrackerState`], stripping combatants marked `GM_ONLY`
    /// or bound to `GM_ONLY` map tokens for non-GM viewers (even when `active_map` is `None`).
    pub fn filter_combat_tracker_for_viewer(
        tracker: &CombatTrackerState,
        active_map: Option<&GmMapUpdateRequest>,
        can_see_gm: bool,
    ) -> CombatTrackerState {
        if can_see_gm {
            return tracker.clone();
        }
        let hidden_ids: std::collections::BTreeSet<&str> = active_map
            .map(|map| {
                map.tokens
                    .iter()
                    .filter(|t| t.visibility == MessageVisibility::GmOnly)
                    .map(|t| t.entity_id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut filtered = tracker.clone();
        filtered.combatants.retain(|c| {
            c.visibility != MessageVisibility::GmOnly
                && !hidden_ids.contains(c.entity_id.as_str())
        });
        filtered
    }

    /// Stubbed hook for future Procedural Environment & Tactical Map Generation.
    ///
    /// When Action Mode (`combat_active == true`) is active, initializes a stubbed `16x12`
    /// tactical grid if `active_map` is `None` and synchronizes [`TokenPlacement`] coordinates
    /// for all combatants on the [`CombatTrackerState`].
    pub fn sync_or_generate_tactical_map_stub(
        session_id: &str,
        tracker: &CombatTrackerState,
        active_map: &mut Option<GmMapUpdateRequest>,
    ) -> Option<GmMapUpdateRequest> {
        if !tracker.combat_active && active_map.is_none() {
            return None;
        }
        let map = active_map.get_or_insert_with(|| {
            let mut built = satya_maps::tactical_map::build_procedural_map_request(
                &format!("map-action-stub-{session_id}"),
                satya_maps::tactical_map::SectorPreset::default(),
                Some(&format!("Action Mode Tactical Grid ({session_id})")),
                16,
                12,
                42,
            );
            built.tokens.clear();
            built.token_visuals.clear();
            built
        });

        let cell_cm = map.cell_size_cm.max(25);
        for (idx, c) in tracker.combatants.iter().enumerate() {
            let x = c
                .grid_x
                .unwrap_or(4 + ((idx as i32) % 4) * 2)
                .clamp(0, map.grid_width - 1);
            let y = c
                .grid_y
                .unwrap_or(5 + ((idx as i32) / 4) * 2)
                .clamp(0, map.grid_height - 1);
            let label = match c.zone_label.as_deref() {
                Some(z) if !z.is_empty() => format!("{} [{}]", c.display_name, z),
                _ => c.display_name.clone(),
            };
            // Match strictly by exact entity_id first, then exact display_name/label (never starts_with).
            let existing_id = map
                .tokens
                .iter()
                .find(|t| t.entity_id == c.entity_id)
                .or_else(|| {
                    map.tokens
                        .iter()
                        .find(|t| t.label == c.display_name || t.label == label)
                })
                .map(|t| t.entity_id.clone());
            if let Some(eid) = existing_id {
                if let Some(tok) = map.tokens.iter_mut().find(|t| t.entity_id == eid) {
                    // Never overwrite live tactical map coordinates of an already-placed token from stale combat_tracker state.
                    tok.label = label;
                    if c.visibility == MessageVisibility::GmOnly {
                        tok.visibility = MessageVisibility::GmOnly;
                    }
                }
            } else {
                map.tokens.push(TokenPlacement {
                    entity_id: c.entity_id.clone(),
                    label,
                    x,
                    y,
                    visibility: c.visibility,
                });
                map.set_token_pos_cm(
                    &c.entity_id,
                    x * cell_cm + cell_cm / 2,
                    y * cell_cm + cell_cm / 2,
                );
            }
        }
        Some(map.clone())
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

    /// Persists `tracker` JSON into the `session_combat_state` SQLite table.
    pub async fn persist_combat_tracker_to_db(
        &self,
        session_id: &str,
        tracker: &CombatTrackerState,
    ) {
        let sid = session_id.trim();
        let updated_at_us = if tracker.updated_at_us > 0 {
            tracker.updated_at_us
        } else {
            Utc::now().timestamp_micros()
        };
        if let Ok(json_str) = serde_json::to_string(tracker) {
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
            .bind(updated_at_us)
            .execute(self.db_pool())
            .await;
        }
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
            let is_condition_only_update = req.tick_delta.is_none()
                && req.combat_active.is_none()
                && req.active_stance.is_none()
                && req.next_action_tick.is_none()
                && req.grid_x.is_none()
                && req.grid_y.is_none()
                && req.zone_label.is_none();
            if let Some(ref eid) = req.entity_id
                && !session
                    .combat_tracker
                    .combatants
                    .iter()
                    .any(|c| c.entity_id == *eid || c.display_name.eq_ignore_ascii_case(eid))
                && let Some(player_part) = session
                    .roster
                    .get(eid)
                    .filter(|p| p.role == SessionRole::Player)
                    .cloned()
            {
                Self::sync_combatant_into_tracker(&mut session.combat_tracker, &player_part);
            }
            let tracker = &mut session.combat_tracker;
            if let Some(active) = req.combat_active {
                tracker.combat_active = active;
            }
            if let Some(delta) = req.tick_delta {
                if delta <= 0 {
                    tracker.round = 1;
                    tracker.tick = 1;
                    if req.combat_active.is_none() {
                        tracker.combat_active = true;
                    }
                } else {
                    tracker.tick += delta as u32;
                    tracker.round = 1 + (tracker.tick / 20);
                    tracker.combat_active = true;
                }
            }
            if let Some(ref eid) = req.entity_id {
                if let Some(entry) = tracker
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
                    if let Some(gx) = req.grid_x {
                        entry.grid_x = Some(gx);
                    }
                    if let Some(gy) = req.grid_y {
                        entry.grid_y = Some(gy);
                    }
                    if let Some(ref zl) = req.zone_label {
                        entry.zone_label = Some(zl.clone());
                    }
                } else {
                    let idx = tracker.combatants.len() as i32;
                    let ooda = (12 - idx).clamp(8, 16);
                    let start_tick =
                        satya_engine::compute_starting_tick(ooda, 0, 0, ooda + 1, 12) as u32;
                    tracker.combatants.push(CombatantTickEntry {
                        entity_id: eid.clone(),
                        display_name: eid.clone(),
                        role_or_faction: "NPC".to_string(),
                        ooda_score: ooda,
                        next_action_tick: req
                            .next_action_tick
                            .unwrap_or_else(|| tracker.tick.max(start_tick)),
                        active_stance: req
                            .active_stance
                            .clone()
                            .or_else(|| Some("NEUTRAL".to_string())),
                        fatigue_level: req.fatigue_level.unwrap_or(0).max(0),
                        wound_shock: req.wound_shock.unwrap_or(0).max(0),
                        grid_x: req.grid_x.or(Some(12 - (idx % 3) * 2)),
                        grid_y: req.grid_y.or(Some(5 + (idx / 3) * 2)),
                        zone_label: req
                            .zone_label
                            .clone()
                            .or_else(|| Some("Entry Bulkhead".to_string())),
                        visibility: MessageVisibility::Public,
                    });
                }
            }
            tracker.updated_at_us = now_us;
            let snap = tracker.clone();
            if !is_condition_only_update {
                let _ =
                    Self::sync_or_generate_tactical_map_stub(sid, &snap, &mut session.active_map);
            }
            snap
        };

        self.persist_combat_tracker_to_db(sid, &updated).await;

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

        let npc_ids: HashSet<String> =
            sqlx::query_scalar::<_, String>("SELECT character_id FROM session_npcs")
                .fetch_all(self.db_pool())
                .await
                .unwrap_or_default()
                .into_iter()
                .collect();

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
            if sheet.character_id.starts_with("npc_") || npc_ids.contains(&sheet.character_id) {
                continue;
            }
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
                let is_gm = if is_gm_service {
                    true
                } else if let Some(uid) = caller_user_id {
                    let caller = session
                        .roster
                        .get(uid)
                        .ok_or_else(|| anyhow::anyhow!("Caller is not in session '{sid}'"))?;
                    matches!(
                        caller.role,
                        SessionRole::Gm | SessionRole::Host | SessionRole::AiGm
                    )
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
                let docs = self.load_persistent_gm_docs_from_db(sid, None).await;
                let mems = self.load_persistent_gm_memories_from_db(sid, None).await;
                let tracker = self.load_or_init_combat_tracker(sid, &[]).await;
                (sid.to_string(), Vec::new(), docs, mems, tracker, None, true)
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
            for r in satya_rules_book::search_rules_book(rule_query, 5) {
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
                            if doc.is_persistent {
                                " · Persistent"
                            } else {
                                ""
                            }
                        ),
                        snippet: excerpt,
                        is_persistent: doc.is_persistent,
                        score: sc + 20,
                    });
                }
            }
        }

        // 3. Persistent GM Memories
        if caller_is_gm
            && matches!(
                norm_scope.as_str(),
                "ALL" | "MEMORY" | "MEMORIES" | "GM_MEMORY"
            )
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
            let visible_tracker = Self::filter_combat_tracker_for_viewer(
                &combat_tracker,
                active_map.as_ref(),
                caller_is_gm,
            );
            let combat_summary = format!(
                "Window {} · Tick {} ({:.2}s) | Active: {} | Combatants: {}",
                visible_tracker.round,
                visible_tracker.tick,
                f64::from(visible_tracker.tick) * 0.25,
                visible_tracker.combat_active,
                visible_tracker
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
                        visible_tracker.round, visible_tracker.tick
                    ),
                    subtitle: format!(
                        "{} Combatants on Timeline",
                        visible_tracker.combatants.len()
                    ),
                    snippet: combat_summary,
                    is_persistent: true,
                    score: sc.max(15),
                });
            }

            if let Some(map) = active_map {
                let tok_str = map
                    .tokens
                    .iter()
                    .filter(|t| caller_is_gm || t.visibility != MessageVisibility::GmOnly)
                    .map(|t| format!("{} at ({},{})", t.label, t.x, t.y))
                    .collect::<Vec<_>>()
                    .join(", ");
                let map_summary = format!(
                    "Map: {} ({}x{}) | Tokens: {} | Notes: {}",
                    map.title,
                    map.grid_width,
                    map.grid_height,
                    if tok_str.is_empty() { "none" } else { &tok_str },
                    if caller_is_gm {
                        map.notes.as_deref().unwrap_or("none")
                    } else {
                        "none"
                    }
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
        hits.truncate(20);

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
        None => (format!("ai-gm-{session_id}"), "Satya AI GM".to_string()),
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

/// Toggles `is_persistent` on a GM memory note at `POST /api/sessions/:session_id/memories/:memory_id/persist`.
pub async fn toggle_session_gm_memory_persistence_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path((session_id, memory_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<ToggleGmDocumentPersistenceRequest>,
) -> impl IntoResponse {
    let (user_opt, is_gm_service) = match resolve_user_or_gm_service(&state, &headers).await {
        Ok(v) => v,
        Err(err_resp) => return err_resp.into_response(),
    };
    let uid = user_opt.as_ref().map(|u| u.user_id.as_str());
    match state
        .auth
        .toggle_session_gm_memory_persistence(
            &session_id,
            uid,
            is_gm_service,
            &memory_id,
            req.is_persistent,
            req.campaign_tag,
        )
        .await
    {
        Ok(record) => (StatusCode::OK, Json(record)).into_response(),
        Err(e) => (
            StatusCode::FORBIDDEN,
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
    let (user_opt, is_gm_service) = match resolve_user_or_gm_service(&state, &headers).await {
        Ok(v) => v,
        Err(err_resp) => return err_resp.into_response(),
    };
    if !is_gm_service && let Some(ref user) = user_opt {
        let roster = state.auth.get_session_roster(&session_id).await;
        if !roster.iter().any(|p| p.user_id == user.user_id) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Caller is not an active participant in this session" })),
            )
                .into_response();
        }
    }
    let can_see_gm = crate::npcs::can_caller_view_gm_secrets(
        &state,
        &session_id,
        user_opt.as_ref(),
        is_gm_service,
    )
    .await;
    match state.auth.get_session_combat_tracker(&session_id).await {
        Some(tracker) => {
            let active_map = state
                .auth
                .get_session_gm_context(&session_id)
                .await
                .and_then(|(ctx, _)| ctx.active_map);
            let filtered = AuthAndSessionManager::filter_combat_tracker_for_viewer(
                &tracker,
                active_map.as_ref(),
                can_see_gm,
            );
            (StatusCode::OK, Json(filtered)).into_response()
        }
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
    let (user_opt, is_gm_service) = match resolve_user_or_gm_service(&state, &headers).await {
        Ok(v) => v,
        Err(err_resp) => return err_resp.into_response(),
    };
    let can_see_gm = crate::npcs::can_caller_view_gm_secrets(
        &state,
        &session_id,
        user_opt.as_ref(),
        is_gm_service,
    )
    .await;
    let is_self_condition_sync = !is_gm_service
        && user_opt.as_ref().is_some_and(|user| {
            req.entity_id.as_deref() == Some(user.user_id.as_str())
                && (req.fatigue_level.is_some() || req.wound_shock.is_some())
                && req.tick_delta.is_none()
                && req.combat_active.is_none()
                && req.active_stance.is_none()
                && req.next_action_tick.is_none()
                && req.grid_x.is_none()
                && req.grid_y.is_none()
                && req.zone_label.is_none()
        });
    let caller_is_player = if is_self_condition_sync
        && let Some(ref user) = user_opt
    {
        state
            .auth
            .get_session_roster(&session_id)
            .await
            .iter()
            .any(|p| p.user_id == user.user_id && p.role == SessionRole::Player)
    } else {
        false
    };
    if !can_see_gm && !(is_self_condition_sync && caller_is_player) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the Game Master can update the Combat Tracker" })),
        )
            .into_response();
    }
    match state
        .auth
        .update_session_combat_tracker(&session_id, req)
        .await
    {
        Ok(tracker) => {
            let ctx_opt = state.auth.get_session_gm_context(&session_id).await;
            let active_map = ctx_opt
                .as_ref()
                .and_then(|(ctx, _)| ctx.active_map.as_ref());
            if can_see_gm
                && let Some((ctx, _)) = ctx_opt.as_ref()
                && let Some(map) = ctx.active_map.as_ref()
            {
                crate::map_handlers::broadcast_role_filtered_map(
                    &state,
                    &session_id,
                    &ctx.roster,
                    map,
                )
                .await;
            }
            let filtered = AuthAndSessionManager::filter_combat_tracker_for_viewer(
                &tracker,
                active_map,
                can_see_gm,
            );
            (StatusCode::OK, Json(filtered)).into_response()
        }
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
    let is_gm_caller = crate::npcs::can_caller_view_gm_secrets(
        &state,
        &session_id,
        user_opt.as_ref(),
        is_gm_service,
    )
    .await;
    match state
        .auth
        .search_session_gm_omnisearch(&session_id, uid, is_gm_service, &params.q, &params.scope)
        .await
    {
        Ok(mut resp) => {
            crate::campaign_lore::append_omnisearch_lore_and_npc_hits(
                &state,
                &session_id,
                is_gm_caller,
                &params.q,
                &params.scope,
                &mut resp.hits,
            )
            .await;
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
