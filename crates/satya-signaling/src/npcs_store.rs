//! SQLite Storage & Redaction Helpers for Session NPCs (`session_npcs` Table).
//!
//! Companion module to [`crate::npcs`] isolating SQLite table creation, NPC row upserts,
//! cross-table cascades (`session_npcs`, `session_characters`, `characters`, `mem_*`),
//! row decoding, case-insensitive name deduplication, and GM-secret redaction.

use satya_schema::{CharacterSheetRecord, SessionNpcEntry};
use sqlx::{Row, SqlitePool};

use crate::auth::AuthAndSessionManager;
use crate::state::SignalingAppState;

pub(crate) const NPC_SELECT_COLS: &str = "character_id, character_name, callsign_or_role, age_years, public_summary, apparent_condition, gm_secret_dossier, psyche_json, top_skills_json, sheet_json, is_persistent, disposition, faction_id, immediate_agenda, core_drive, dual_psyche_wedge_hint";

pub(crate) async fn ensure_session_npcs_table_with_pool(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS session_npcs (
            character_id TEXT PRIMARY KEY, session_id TEXT NOT NULL, campaign_tag TEXT NOT NULL,
            owner_user_id TEXT NOT NULL, character_name TEXT NOT NULL, callsign_or_role TEXT NOT NULL,
            age_years INTEGER NOT NULL, public_summary TEXT NOT NULL, apparent_condition TEXT NOT NULL,
            gm_secret_dossier TEXT NOT NULL, psyche_json TEXT NOT NULL, top_skills_json TEXT NOT NULL,
            sheet_json TEXT NOT NULL, is_persistent INTEGER NOT NULL DEFAULT 1, updated_at_us INTEGER NOT NULL,
            disposition TEXT, faction_id TEXT, immediate_agenda TEXT, core_drive TEXT, dual_psyche_wedge_hint TEXT
        )",
    )
    .execute(pool)
    .await?;
    for col in [
        "disposition TEXT",
        "faction_id TEXT",
        "immediate_agenda TEXT",
        "core_drive TEXT",
        "dual_psyche_wedge_hint TEXT",
    ] {
        let _ = sqlx::query(&format!("ALTER TABLE session_npcs ADD COLUMN {col}"))
            .execute(pool)
            .await;
    }
    Ok(())
}

pub(crate) async fn upsert_session_npc_row(
    pool: &SqlitePool,
    sid: &str,
    campaign_tag: &str,
    owner_uid: &str,
    entry: &SessionNpcEntry,
    now_us: i64,
) -> anyhow::Result<()> {
    let psyche_json = serde_json::to_string(&entry.psyche_summary).unwrap_or_default();
    let top_skills_json = serde_json::to_string(&entry.top_skills).unwrap_or_default();
    let sheet_json = entry
        .gm_character
        .as_ref()
        .and_then(|c| serde_json::to_string(c).ok())
        .unwrap_or_default();
    sqlx::query(
        "INSERT INTO session_npcs (
            character_id, session_id, campaign_tag, owner_user_id,
            character_name, callsign_or_role, age_years, public_summary,
            apparent_condition, gm_secret_dossier, psyche_json, top_skills_json,
            sheet_json, is_persistent, updated_at_us,
            disposition, faction_id, immediate_agenda, core_drive, dual_psyche_wedge_hint
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
        ON CONFLICT(character_id) DO UPDATE SET
            session_id = excluded.session_id,
            campaign_tag = CASE WHEN excluded.campaign_tag != '' THEN excluded.campaign_tag ELSE session_npcs.campaign_tag END,
            character_name = excluded.character_name, callsign_or_role = excluded.callsign_or_role,
            age_years = excluded.age_years, public_summary = excluded.public_summary,
            apparent_condition = excluded.apparent_condition, gm_secret_dossier = excluded.gm_secret_dossier,
            psyche_json = excluded.psyche_json, top_skills_json = excluded.top_skills_json,
            sheet_json = CASE WHEN excluded.sheet_json != '' THEN excluded.sheet_json ELSE session_npcs.sheet_json END,
            is_persistent = excluded.is_persistent, updated_at_us = excluded.updated_at_us,
            disposition = excluded.disposition,
            faction_id = COALESCE(excluded.faction_id, session_npcs.faction_id),
            immediate_agenda = excluded.immediate_agenda,
            core_drive = COALESCE(excluded.core_drive, session_npcs.core_drive),
            dual_psyche_wedge_hint = COALESCE(excluded.dual_psyche_wedge_hint, session_npcs.dual_psyche_wedge_hint)",
    )
    .bind(&entry.character_id)
    .bind(sid)
    .bind(campaign_tag)
    .bind(owner_uid)
    .bind(&entry.character_name)
    .bind(&entry.callsign_or_role)
    .bind(entry.age_years)
    .bind(&entry.public_summary)
    .bind(&entry.apparent_condition)
    .bind(entry.gm_secret_dossier.as_deref().unwrap_or(""))
    .bind(&psyche_json)
    .bind(&top_skills_json)
    .bind(&sheet_json)
    .bind(i32::from(entry.is_persistent))
    .bind(now_us)
    .bind(&entry.disposition)
    .bind(&entry.faction_id)
    .bind(&entry.immediate_agenda)
    .bind(&entry.core_drive)
    .bind(&entry.dual_psyche_wedge_hint)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn delete_npc_records_by_id(
    state: &SignalingAppState,
    sid: &str,
    caller_uid: Option<&str>,
    is_gm_service: bool,
    character_id: &str,
) -> bool {
    let pool = state.auth.db_pool();
    let deleted = sqlx::query(
        "DELETE FROM session_npcs WHERE character_id = ?1 AND (session_id = ?2 OR campaign_tag = ?2 COLLATE NOCASE)",
    )
    .bind(character_id)
    .bind(sid)
    .execute(pool)
    .await
    .map(|r| r.rows_affected() > 0)
    .unwrap_or(false);
    let _ = sqlx::query(
        "DELETE FROM session_characters WHERE session_id = ?1 AND (user_id = ?2 OR character_id = ?3)",
    )
    .bind(sid)
    .bind(format!("npc:{character_id}"))
    .bind(character_id)
    .execute(pool)
    .await;
    if deleted {
        let _ = sqlx::query(
            "DELETE FROM characters WHERE character_id = ?1 AND character_id LIKE 'npc_%' AND NOT EXISTS(SELECT 1 FROM session_npcs n WHERE n.character_id = ?1)",
        )
        .bind(character_id)
        .execute(pool)
        .await;
    }
    let _ = state
        .auth
        .delete_session_gm_memory(
            sid,
            caller_uid,
            is_gm_service,
            &format!("mem_{character_id}"),
        )
        .await;
    deleted
}

pub(crate) fn redact_npc_entry_for_viewer(
    mut entry: SessionNpcEntry,
    can_see_gm: bool,
) -> SessionNpcEntry {
    if !can_see_gm {
        entry.gm_character = None;
        entry.psyche_summary.clear();
        entry.top_skills.clear();
        entry.gm_secret_dossier = None;
        entry.immediate_agenda = None;
        entry.core_drive = None;
        entry.dual_psyche_wedge_hint = None;
    }
    entry
}

pub(crate) fn decode_npc_row(row: &sqlx::sqlite::SqliteRow, can_see_gm: bool) -> SessionNpcEntry {
    let character_id: String = row.try_get("character_id").unwrap_or_default();
    let character_name: String = row.try_get("character_name").unwrap_or_default();
    let callsign_or_role: String = row.try_get("callsign_or_role").unwrap_or_default();
    let age_years: i32 = row.try_get("age_years").unwrap_or(26);
    let public_summary: String = row.try_get("public_summary").unwrap_or_default();
    let apparent_condition: String = row.try_get("apparent_condition").unwrap_or_default();
    let gm_secret_dossier: String = row.try_get("gm_secret_dossier").unwrap_or_default();
    let psyche_json: String = row.try_get("psyche_json").unwrap_or_default();
    let top_skills_json: String = row.try_get("top_skills_json").unwrap_or_default();
    let sheet_json: String = row.try_get("sheet_json").unwrap_or_default();
    let is_persistent: i32 = row.try_get("is_persistent").unwrap_or(1);
    let disposition: Option<String> = row.try_get("disposition").ok().flatten();
    let faction_id: Option<String> = row.try_get("faction_id").ok().flatten();
    let immediate_agenda: Option<String> = row.try_get("immediate_agenda").ok().flatten();
    let core_drive: Option<String> = row.try_get("core_drive").ok().flatten();
    let dual_psyche_wedge_hint: Option<String> =
        row.try_get("dual_psyche_wedge_hint").ok().flatten();

    let psyche_summary: Vec<String> = serde_json::from_str(&psyche_json).unwrap_or_default();
    let top_skills: Vec<String> = serde_json::from_str(&top_skills_json).unwrap_or_default();
    let gm_character: Option<CharacterSheetRecord> = serde_json::from_str(&sheet_json).ok();

    redact_npc_entry_for_viewer(
        SessionNpcEntry {
            character_id,
            character_name,
            callsign_or_role,
            age_years,
            public_summary,
            apparent_condition,
            is_persistent: is_persistent != 0,
            gm_character,
            psyche_summary,
            top_skills,
            gm_secret_dossier: if gm_secret_dossier.is_empty() {
                None
            } else {
                Some(gm_secret_dossier)
            },
            disposition,
            faction_id,
            immediate_agenda,
            core_drive,
            dual_psyche_wedge_hint,
        },
        can_see_gm,
    )
}

pub(crate) async fn query_session_npc_rows(
    pool: &SqlitePool,
    sid: &str,
    can_see_gm: bool,
) -> anyhow::Result<Vec<SessionNpcEntry>> {
    let sql = format!(
        "SELECT {NPC_SELECT_COLS} FROM session_npcs WHERE session_id = ?1 OR (campaign_tag = ?1 COLLATE NOCASE AND is_persistent = 1) ORDER BY updated_at_us DESC"
    );
    let rows = sqlx::query(&sql).bind(sid).fetch_all(pool).await?;

    let mut seen_names = std::collections::HashSet::new();
    let mut npcs = Vec::with_capacity(rows.len());
    for row in &rows {
        let entry = decode_npc_row(row, can_see_gm);
        let key = entry.character_name.trim().to_ascii_lowercase();
        if key.is_empty() || seen_names.insert(key) {
            npcs.push(entry);
        }
    }
    Ok(npcs)
}

/// Queries SQLite `session_npcs` for Omnisearch indexing, applying `can_see_gm` redaction.
pub(crate) async fn load_session_npcs_for_omnisearch(
    auth: &AuthAndSessionManager,
    sid: &str,
    can_see_gm: bool,
) -> Vec<SessionNpcEntry> {
    if ensure_session_npcs_table_with_pool(auth.db_pool())
        .await
        .is_err()
    {
        return Vec::new();
    }
    query_session_npc_rows(auth.db_pool(), sid, can_see_gm)
        .await
        .unwrap_or_default()
}
