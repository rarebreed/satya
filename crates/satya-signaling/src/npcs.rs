//! Prompt- & Template-Driven NPC Generation & Role-Filtered Session NPC Roster Endpoints
//! (`POST /api/sessions/:session_id/npcs/generate`, `GET /api/sessions/:session_id/npcs`,
//! `POST /api/gm/sessions/:session_id/npcs/generate`, `GET /api/gm/sessions/:session_id/npcs`).
//!
//! Ensures:
//! - Both Human GMs and the AI GM can roll up rules-legal Satya NPCs from a short prompt or template.
//! - Generated NPCs are audited/signed (`VerifiedOfficial`), saved to the `characters` and `session_npcs`
//!   tables, bound to the session character roster (`session_characters`) for Omnisearch, and optionally
//!   stored as a persistent `GmMemoryNoteRecord` (`NPC_SECRET`).
//! - Non-GM players in human-GM sessions see only the public NPC summary (`character_name`, `callsign_or_role`,
//!   `age_years`, `public_summary`, `apparent_condition`), whereas GMs and Solo-mode players see the full
//!   `CharacterSheetRecord`, `psyche_summary`, `top_skills`, and `gm_secret_dossier`.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use satya_engine::{NpcGenerationParams, NpcTemplateKind, generate_npc_from_prompt};
use satya_schema::{
    CharacterSheetRecord, GenerateNpcRequest, GenerateNpcResponse, GmMemoryCategory,
    ParticipantKind, SessionNpcEntry, SessionNpcsResponse, SessionRole, UpsertGmMemoryRequest,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::UserAccount;
use crate::characters::audit_and_stamp_character_record;
use crate::gm_api::verify_gm_service_header;
use crate::state::SignalingAppState;

async fn ensure_session_npcs_table(state: &SignalingAppState) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS session_npcs (
            character_id        TEXT PRIMARY KEY,
            session_id          TEXT NOT NULL,
            campaign_tag        TEXT NOT NULL,
            owner_user_id       TEXT NOT NULL,
            character_name      TEXT NOT NULL,
            callsign_or_role    TEXT NOT NULL,
            age_years           INTEGER NOT NULL,
            public_summary      TEXT NOT NULL,
            apparent_condition  TEXT NOT NULL,
            gm_secret_dossier   TEXT NOT NULL,
            psyche_json         TEXT NOT NULL,
            top_skills_json     TEXT NOT NULL,
            sheet_json          TEXT NOT NULL,
            is_persistent       INTEGER NOT NULL DEFAULT 1,
            updated_at_us       INTEGER NOT NULL
        );
        "#,
    )
    .execute(state.auth.db_pool())
    .await?;
    Ok(())
}

async fn resolve_caller(
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

/// Determines whether the caller is allowed to see GM-only NPC stats, Psyche traits, and secrets.
///
/// Returns `true` when:
/// - The caller is the AI GM service (`is_gm_service == true`), OR
/// - The caller has `SessionRole::Gm` in the session roster, OR
/// - The session has no human `SessionRole::Gm` (e.g. Solo / Autonomous AI-GM mode where the player IS the table host).
async fn can_caller_view_gm_secrets(
    state: &SignalingAppState,
    session_id: &str,
    user_opt: Option<&UserAccount>,
    is_gm_service: bool,
) -> bool {
    if is_gm_service {
        return true;
    }
    let Some(user) = user_opt else {
        return false;
    };
    let roster = state.auth.get_session_roster(session_id).await;
    if roster.is_empty() {
        return true;
    }
    let caller_is_gm = roster
        .iter()
        .any(|p| p.user_id == user.user_id && p.role == SessionRole::Gm);
    if caller_is_gm {
        return true;
    }
    let has_human_gm = roster
        .iter()
        .any(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm);
    !has_human_gm
}

fn redact_npc_entry_for_viewer(mut entry: SessionNpcEntry, can_see_gm: bool) -> SessionNpcEntry {
    if !can_see_gm {
        entry.gm_character = None;
        entry.psyche_summary.clear();
        entry.top_skills.clear();
        entry.gm_secret_dossier = None;
    }
    entry
}

fn compute_apparent_condition(sheet: &CharacterSheetRecord) -> String {
    if sheet.wound_shock_index >= 3 || sheet.fatigue_level >= 4 {
        "Critically Wounded / Exhausted".to_string()
    } else if sheet.wound_shock_index >= 1 || sheet.fatigue_level >= 2 {
        "Visibly Fatigued / Battered".to_string()
    } else {
        "Healthy & Alert".to_string()
    }
}

/// Generates a rules-legal Satya NPC from a natural-language prompt and/or template, persists it to the session/campaign,
/// and optionally creates a persistent `GmMemoryNoteRecord` (`POST /api/sessions/:session_id/npcs/generate`).
pub async fn generate_session_npc_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<GenerateNpcRequest>,
) -> impl IntoResponse {
    let (user_opt, is_gm_service) = match resolve_caller(&state, &headers).await {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };

    let sid = session_id.trim();
    if sid.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "session_id cannot be empty" })),
        )
            .into_response();
    }

    if let Err(e) = ensure_session_npcs_table(&state).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    let owner_uid = user_opt
        .as_ref()
        .map(|u| u.user_id.clone())
        .unwrap_or_else(|| "ai_gm_service".to_string());
    let author_name = user_opt
        .as_ref()
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| "Satya AI GM".to_string());

    let template = req
        .template
        .as_deref()
        .and_then(NpcTemplateKind::from_code_str)
        .unwrap_or(NpcTemplateKind::Auto);

    let seed = if req.seed != 0 {
        req.seed
    } else {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in format!("{}|{}|{}", sid, req.prompt.trim(), Uuid::new_v4()).bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h
    };

    let gen_res = generate_npc_from_prompt(&NpcGenerationParams {
        prompt: req.prompt.clone(),
        name_override: req.name_override.clone(),
        template,
        career_terms: req.career_terms.clamp(1, 4),
        seed,
        owner_user_id: owner_uid.clone(),
    });

    let mut character = gen_res.character;
    let now_us = Utc::now().timestamp_micros();
    character.updated_at_us = now_us;

    let mut full_dossier = gen_res.gm_secret_dossier.clone();
    if let Some(extra_secret) = req
        .gm_secret_notes
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        full_dossier.push_str("\nGM Secret Notes: ");
        full_dossier.push_str(extra_secret);
        character.notes = Some(full_dossier.clone());
    }

    audit_and_stamp_character_record(&state, &mut character, true);

    let campaign_tag = req
        .campaign_tag
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(sid)
        .to_string();

    let psyche_summary: Vec<String> = gen_res
        .psyche_traits
        .iter()
        .map(|p| format!("{} {} ({})", p.code, p.score, p.descriptor))
        .collect();
    let top_skills: Vec<String> = gen_res
        .top_skills
        .iter()
        .map(|(_, name, rank)| format!("{name} (N={rank})"))
        .collect();
    let apparent_condition = compute_apparent_condition(&character);

    let sheet_json = serde_json::to_string(&character).unwrap_or_default();
    let psyche_json = serde_json::to_string(&psyche_summary).unwrap_or_default();
    let top_skills_json = serde_json::to_string(&top_skills).unwrap_or_default();

    // Save into `characters` and `session_characters` so Omnisearch & character sheet lookup find the NPC
    let _ = sqlx::query(
        r#"
        INSERT INTO characters (
            character_id, owner_user_id, character_name, version,
            verification_status, content_sha256, signature_hex, sheet_json,
            created_at_us, updated_at_us
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(character_id) DO UPDATE SET
            character_name      = excluded.character_name,
            version             = excluded.version,
            verification_status = excluded.verification_status,
            content_sha256      = excluded.content_sha256,
            signature_hex       = excluded.signature_hex,
            sheet_json          = excluded.sheet_json,
            updated_at_us       = excluded.updated_at_us
        "#,
    )
    .bind(&character.character_id)
    .bind(&character.owner_user_id)
    .bind(&character.character_name)
    .bind(character.version)
    .bind(character.verification_status.as_str())
    .bind(&character.content_sha256)
    .bind(&character.server_signature_hex)
    .bind(&sheet_json)
    .bind(now_us)
    .bind(now_us)
    .execute(state.auth.db_pool())
    .await;

    let npc_slot_user_id = format!("npc:{}", character.character_id);
    let _ = sqlx::query(
        r#"
        INSERT INTO session_characters (session_id, user_id, character_id, updated_at_us)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(session_id, user_id) DO UPDATE SET
            character_id  = excluded.character_id,
            updated_at_us = excluded.updated_at_us
        "#,
    )
    .bind(sid)
    .bind(&npc_slot_user_id)
    .bind(&character.character_id)
    .bind(now_us)
    .execute(state.auth.db_pool())
    .await;

    let _ = sqlx::query(
        r#"
        INSERT INTO session_npcs (
            character_id, session_id, campaign_tag, owner_user_id,
            character_name, callsign_or_role, age_years, public_summary,
            apparent_condition, gm_secret_dossier, psyche_json, top_skills_json,
            sheet_json, is_persistent, updated_at_us
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        ON CONFLICT(character_id) DO UPDATE SET
            session_id         = excluded.session_id,
            campaign_tag       = excluded.campaign_tag,
            character_name     = excluded.character_name,
            callsign_or_role   = excluded.callsign_or_role,
            age_years          = excluded.age_years,
            public_summary     = excluded.public_summary,
            apparent_condition = excluded.apparent_condition,
            gm_secret_dossier  = excluded.gm_secret_dossier,
            psyche_json        = excluded.psyche_json,
            top_skills_json    = excluded.top_skills_json,
            sheet_json         = excluded.sheet_json,
            is_persistent      = excluded.is_persistent,
            updated_at_us      = excluded.updated_at_us
        "#,
    )
    .bind(&character.character_id)
    .bind(sid)
    .bind(&campaign_tag)
    .bind(&owner_uid)
    .bind(&character.character_name)
    .bind(&character.callsign_or_role)
    .bind(character.age_years)
    .bind(&gen_res.public_summary)
    .bind(&apparent_condition)
    .bind(&full_dossier)
    .bind(&psyche_json)
    .bind(&top_skills_json)
    .bind(&sheet_json)
    .bind(i32::from(req.is_persistent))
    .bind(now_us)
    .execute(state.auth.db_pool())
    .await;

    let gm_memory = if req.is_persistent {
        state
            .auth
            .upsert_session_gm_memory(
                sid,
                &owner_uid,
                &author_name,
                is_gm_service,
                UpsertGmMemoryRequest {
                    memory_id: Some(format!("mem_{}", character.character_id)),
                    campaign_tag: Some(campaign_tag.clone()),
                    category: GmMemoryCategory::NpcSecret,
                    title: format!("NPC: {} ({})", character.character_name, character.callsign_or_role),
                    content: full_dossier.clone(),
                    is_pinned: true,
                    is_persistent: true,
                },
            )
            .await
            .ok()
    } else {
        None
    };

    let can_see_gm = can_caller_view_gm_secrets(&state, sid, user_opt.as_ref(), is_gm_service).await;
    let entry = redact_npc_entry_for_viewer(
        SessionNpcEntry {
            character_id: character.character_id.clone(),
            character_name: character.character_name.clone(),
            callsign_or_role: character.callsign_or_role.clone(),
            age_years: character.age_years,
            public_summary: gen_res.public_summary,
            apparent_condition,
            is_persistent: req.is_persistent,
            gm_character: Some(character),
            psyche_summary,
            top_skills,
            gm_secret_dossier: Some(full_dossier),
        },
        can_see_gm,
    );

    (
        StatusCode::OK,
        Json(GenerateNpcResponse {
            npc: entry,
            gm_memory,
        }),
    )
        .into_response()
}

/// Lists all NPCs for `session_id` (and matching persistent `campaign_tag`), automatically redacting
/// GM-only stats/secrets when the caller is a non-GM player in a human-GM session (`GET /api/sessions/:session_id/npcs`).
pub async fn list_session_npcs_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user_opt, is_gm_service) = match resolve_caller(&state, &headers).await {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };

    let sid = session_id.trim();
    if let Err(e) = ensure_session_npcs_table(&state).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    let can_see_gm = can_caller_view_gm_secrets(&state, sid, user_opt.as_ref(), is_gm_service).await;

    let rows = match sqlx::query(
        r#"
        SELECT character_id, character_name, callsign_or_role, age_years,
               public_summary, apparent_condition, gm_secret_dossier,
               psyche_json, top_skills_json, sheet_json, is_persistent
        FROM session_npcs
        WHERE session_id = ?1 OR (campaign_tag = ?1 AND is_persistent = 1)
        ORDER BY updated_at_us DESC
        "#,
    )
    .bind(sid)
    .fetch_all(state.auth.db_pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let mut npcs = Vec::with_capacity(rows.len());
    for row in rows {
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

        let psyche_summary: Vec<String> = serde_json::from_str(&psyche_json).unwrap_or_default();
        let top_skills: Vec<String> = serde_json::from_str(&top_skills_json).unwrap_or_default();
        let gm_character: Option<CharacterSheetRecord> = serde_json::from_str(&sheet_json).ok();

        npcs.push(redact_npc_entry_for_viewer(
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
                gm_secret_dossier: Some(gm_secret_dossier),
            },
            can_see_gm,
        ));
    }

    (
        StatusCode::OK,
        Json(SessionNpcsResponse {
            session_id: sid.to_string(),
            campaign_tag: sid.to_string(),
            viewer_can_see_gm_secrets: can_see_gm,
            npcs,
        }),
    )
        .into_response()
}
