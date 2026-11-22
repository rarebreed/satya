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
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use satya_campaign::{
    FactionArchetypeId, NpcGenerationParams, NpcTemplateKind, generate_npc_from_prompt_with_pack,
};
use satya_schema::{
    CharacterSheetRecord, GenerateNpcRequest, GenerateNpcResponse, GmMemoryCategory,
    GmMemoryNoteRecord, LoadStarterNpcRequest, ParticipantKind, ServerWsPacket, SessionNpcEntry,
    SessionNpcsResponse, SessionRole, UpdateSessionNpcRequest, UpsertGmMemoryRequest,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::UserAccount;
use crate::characters::audit_and_stamp_character_record;
use crate::gm_api::verify_gm_service_header;
pub(crate) use crate::npcs_store::*;
use crate::state::SignalingAppState;

/// Optional query toggle allowing a `SessionRole::SoloGm` Co-GM to view redacted player-safe data.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SpoilerShieldQuery {
    #[serde(default)]
    pub spoiler_shield: bool,
}

fn json_err(status: StatusCode, msg: impl std::fmt::Display) -> Response {
    (status, Json(json!({ "error": msg.to_string() }))).into_response()
}

pub(crate) async fn broadcast_npc_roster_updated(state: &SignalingAppState, sid: &str) {
    if let Ok(pkt) = serde_json::to_string(&ServerWsPacket::NpcRosterUpdated {
        session_id: sid.to_string(),
    }) {
        state.broadcast_to_session(sid, &pkt).await;
    }
}

async fn prepare_npc_request(
    state: &SignalingAppState,
    headers: &HeaderMap,
    sid: &str,
    require_gm: bool,
    spoiler_shield: bool,
) -> Result<(Option<UserAccount>, bool, bool), Box<Response>> {
    let (user_opt, is_gm_service) = if verify_gm_service_header(state, headers).is_ok() {
        (None, true)
    } else {
        let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
            return Err(Box::new(json_err(
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header",
            )));
        };
        let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
            return Err(Box::new(json_err(
                StatusCode::UNAUTHORIZED,
                "Invalid or expired auth token",
            )));
        };
        (Some(user), false)
    };
    if sid.is_empty() {
        return Err(Box::new(json_err(
            StatusCode::BAD_REQUEST,
            "session_id cannot be empty",
        )));
    }
    if let Some(ref user) = user_opt {
        if !state.auth.sessions.read().await.contains_key(sid) {
            let _ = state.auth.rehydrate_session_from_db(sid).await;
        }
        if state
            .auth
            .resolve_authorized_member_role(sid, user)
            .await
            .is_err()
        {
            return Err(Box::new(json_err(
                StatusCode::FORBIDDEN,
                "Caller is not a participant in this session",
            )));
        }
    }
    let can_manage = can_caller_manage_gm_table(state, sid, user_opt.as_ref(), is_gm_service).await;
    if require_gm && !can_manage {
        return Err(Box::new(json_err(
            StatusCode::FORBIDDEN,
            "Only the Game Master or Solo Co-GM can modify NPC state",
        )));
    }
    let can_see_gm = can_caller_view_gm_secrets_with_shield(
        state,
        sid,
        user_opt.as_ref(),
        is_gm_service,
        spoiler_shield,
    )
    .await;
    ensure_session_npcs_table_with_pool(state.auth.db_pool())
        .await
        .map_err(|e| Box::new(json_err(StatusCode::INTERNAL_SERVER_ERROR, e)))?;
    Ok((user_opt, is_gm_service, can_see_gm))
}

/// Returns `true` when the caller may manage GM table controls (NPCs, maps, docs, clocks).
pub(crate) async fn can_caller_manage_gm_table(
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
    roster
        .iter()
        .any(|p| p.user_id == user.user_id && p.role.is_gm_or_co_gm())
}

/// Determines whether the caller is allowed to see GM-only stats, Psyche traits, and secrets,
/// honoring an optional `spoiler_shield` toggle for `SessionRole::SoloGm`.
pub(crate) async fn can_caller_view_gm_secrets_with_shield(
    state: &SignalingAppState,
    session_id: &str,
    user_opt: Option<&UserAccount>,
    is_gm_service: bool,
    spoiler_shield: bool,
) -> bool {
    if is_gm_service {
        return true;
    }
    let Some(user) = user_opt else {
        return false;
    };
    let roster = state.auth.get_session_roster(session_id).await;
    roster.iter().any(|p| {
        p.user_id == user.user_id
            && p.role.is_gm_or_co_gm()
            && !(spoiler_shield && p.role == SessionRole::SoloGm)
    })
}

/// Determines whether the caller is allowed to see GM-only NPC stats, Psyche traits, and secrets.
pub(crate) async fn can_caller_view_gm_secrets(
    state: &SignalingAppState,
    session_id: &str,
    user_opt: Option<&UserAccount>,
    is_gm_service: bool,
) -> bool {
    can_caller_view_gm_secrets_with_shield(state, session_id, user_opt, is_gm_service, false).await
}

/// Determines whether the caller may orchestrate AI GM / AI Player configuration and stepping
/// without exposing GM secrets (allows active `Gm`/`Host`/`SoloGm`, or active `host_user_id` as `Player`/`Observer`
/// when no Human GM is present).
pub(crate) async fn can_caller_orchestrate_ai_session(
    state: &SignalingAppState,
    session_id: &str,
    user: &UserAccount,
) -> bool {
    let guard = state.auth.sessions.read().await;
    let Some(session) = guard.get(session_id.trim()) else {
        return false;
    };
    let Some(caller) = session.roster.get(&user.user_id) else {
        return false;
    };
    if caller.role.is_human_gm_or_co_gm() {
        return true;
    }
    let has_human_gm = session
        .roster
        .values()
        .any(|p| p.participant_kind == ParticipantKind::Human && p.role.is_human_gm_or_co_gm());
    !has_human_gm
        && session.host_user_id == user.user_id
        && matches!(caller.role, SessionRole::Player | SessionRole::Observer)
}

pub(crate) fn compute_apparent_condition(sheet: &CharacterSheetRecord) -> String {
    if sheet.wound_shock_index >= 3 || sheet.fatigue_level >= 4 {
        "Critically Wounded / Exhausted".to_string()
    } else if sheet.wound_shock_index >= 1 || sheet.fatigue_level >= 2 {
        "Visibly Fatigued / Battered".to_string()
    } else {
        "Healthy & Alert".to_string()
    }
}

struct NpcVaultWriteContext<'a> {
    sid: &'a str,
    campaign_tag: &'a str,
    owner_uid: &'a str,
    author_name: &'a str,
    is_gm_service: bool,
    caller_uid: Option<&'a str>,
    reuse_existing_id: bool,
}

async fn persist_npc_entry_to_vault(
    state: &SignalingAppState,
    ctx: &NpcVaultWriteContext<'_>,
    mut full_entry: SessionNpcEntry,
    full_dossier: &str,
    now_us: i64,
) -> Result<(SessionNpcEntry, Option<GmMemoryNoteRecord>), Box<Response>> {
    let existing_ids: Vec<String> = sqlx::query_scalar::<_, String>(
        r#"
        SELECT character_id FROM session_npcs
        WHERE (session_id = ?1 OR (campaign_tag = ?2 COLLATE NOCASE AND is_persistent = 1))
          AND LOWER(TRIM(character_name)) = LOWER(TRIM(?3))
        ORDER BY updated_at_us DESC
        "#,
    )
    .bind(ctx.sid)
    .bind(ctx.campaign_tag)
    .bind(&full_entry.character_name)
    .fetch_all(state.auth.db_pool())
    .await
    .unwrap_or_default();

    if ctx.reuse_existing_id
        && let Some(primary_id) = existing_ids.first()
    {
        full_entry.character_id = primary_id.clone();
        if let Some(ref mut ch) = full_entry.gm_character {
            ch.character_id = primary_id.clone();
        }
    }
    for dup_id in existing_ids {
        if dup_id != full_entry.character_id {
            delete_npc_records_by_id(state, ctx.sid, ctx.caller_uid, ctx.is_gm_service, &dup_id)
                .await;
        }
    }

    if let Some(ref mut ch) = full_entry.gm_character {
        audit_and_stamp_character_record(state, ch, true);
        let sheet_json = serde_json::to_string(ch).unwrap_or_default();
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
        .bind(&ch.character_id)
        .bind(&ch.owner_user_id)
        .bind(&ch.character_name)
        .bind(ch.version)
        .bind(ch.verification_status.as_str())
        .bind(&ch.content_sha256)
        .bind(&ch.server_signature_hex)
        .bind(&sheet_json)
        .bind(now_us)
        .bind(now_us)
        .execute(state.auth.db_pool())
        .await;

        let npc_slot_user_id = format!("npc:{}", ch.character_id);
        let _ = sqlx::query(
            r#"
            INSERT INTO session_characters (session_id, user_id, character_id, updated_at_us)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(session_id, user_id) DO UPDATE SET
                character_id  = excluded.character_id,
                updated_at_us = excluded.updated_at_us
            "#,
        )
        .bind(ctx.sid)
        .bind(&npc_slot_user_id)
        .bind(&ch.character_id)
        .bind(now_us)
        .execute(state.auth.db_pool())
        .await;
    }

    if let Err(err) = upsert_session_npc_row(
        state.auth.db_pool(),
        ctx.sid,
        ctx.campaign_tag,
        ctx.owner_uid,
        &full_entry,
        now_us,
    )
    .await
    {
        return Err(Box::new(json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to persist NPC to session_npcs: {err}"),
        )));
    }

    let mem_id = format!("mem_{}", full_entry.character_id);
    let gm_memory = if full_entry.is_persistent && !full_dossier.trim().is_empty() {
        state
            .auth
            .upsert_session_gm_memory(
                ctx.sid,
                ctx.owner_uid,
                ctx.author_name,
                ctx.is_gm_service,
                UpsertGmMemoryRequest {
                    memory_id: Some(mem_id),
                    campaign_tag: Some(ctx.campaign_tag.to_string()),
                    category: GmMemoryCategory::NpcSecret,
                    title: format!(
                        "NPC: {} ({})",
                        full_entry.character_name, full_entry.callsign_or_role
                    ),
                    content: full_dossier.to_string(),
                    is_pinned: true,
                    is_persistent: true,
                },
            )
            .await
            .ok()
    } else {
        let _ = state
            .auth
            .delete_session_gm_memory(ctx.sid, ctx.caller_uid, ctx.is_gm_service, &mem_id)
            .await;
        None
    };

    broadcast_npc_roster_updated(state, ctx.sid).await;
    Ok((full_entry, gm_memory))
}

/// Generates a rules-legal Satya NPC from a natural-language prompt and/or template, persists it to the session/campaign,
/// and optionally creates a persistent `GmMemoryNoteRecord` (`POST /api/sessions/:session_id/npcs/generate`).
pub async fn generate_session_npc_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    Query(shield): Query<SpoilerShieldQuery>,
    headers: HeaderMap,
    Json(req): Json<GenerateNpcRequest>,
) -> impl IntoResponse {
    let sid = session_id.trim();
    let (user_opt, is_gm_service, can_see_gm) =
        match prepare_npc_request(&state, &headers, sid, true, shield.spoiler_shield).await {
            Ok(v) => v,
            Err(resp) => return *resp,
        };

    let owner_uid = user_opt
        .as_ref()
        .map(|u| u.user_id.clone())
        .unwrap_or_else(|| "ai_gm_service".to_string());
    let author_name = user_opt
        .as_ref()
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| "Satya AI GM".to_string());

    let faction_archetype = req
        .template
        .as_deref()
        .and_then(FactionArchetypeId::from_code_str);
    let template = if faction_archetype.is_some() {
        NpcTemplateKind::Auto
    } else {
        req.template
            .as_deref()
            .and_then(NpcTemplateKind::from_code_str)
            .unwrap_or(NpcTemplateKind::Auto)
    };

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

    let pack = crate::campaign_lore::resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let gen_res = generate_npc_from_prompt_with_pack(
        &NpcGenerationParams {
            prompt: req.prompt.clone(),
            name_override: req.name_override.clone(),
            template,
            faction_archetype,
            career_terms: req.career_terms.clamp(1, 4),
            seed,
            owner_user_id: owner_uid.clone(),
        },
        &pack.pack_name,
    );

    let mut character = gen_res.character;
    let now_us = Utc::now().timestamp_micros();
    character.updated_at_us = now_us;

    let disposition = if character.characteristics.agg >= 14 {
        "guarded".to_string()
    } else if character.characteristics.trs <= 4 && character.characteristics.cps <= 6 {
        "suspicious".to_string()
    } else if character.characteristics.cps >= 14 || character.characteristics.slf <= 7 {
        "compassionate".to_string()
    } else {
        "wary".to_string()
    };
    let immediate_agenda = Some(gen_res.motivation.immediate_agenda.clone());
    let core_drive = Some(gen_res.motivation.core_drive.clone());
    let dual_psyche_wedge_hint = Some(gen_res.motivation.dual_psyche_wedge.clone());

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

    let campaign_tag = req
        .campaign_tag
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(sid)
        .to_string();
    let caller_uid = user_opt.as_ref().map(|u| u.user_id.as_str());

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
    let faction_id = character.faction_id.clone();

    let full_entry = SessionNpcEntry {
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
        gm_secret_dossier: Some(full_dossier.clone()),
        disposition: Some(disposition),
        faction_id,
        immediate_agenda,
        core_drive,
        dual_psyche_wedge_hint,
    };

    let ctx = NpcVaultWriteContext {
        sid,
        campaign_tag: &campaign_tag,
        owner_uid: &owner_uid,
        author_name: &author_name,
        is_gm_service,
        caller_uid,
        reuse_existing_id: true,
    };
    let (saved_entry, gm_memory) =
        match persist_npc_entry_to_vault(&state, &ctx, full_entry, &full_dossier, now_us).await {
            Ok(res) => res,
            Err(resp) => return *resp,
        };

    let entry = redact_npc_entry_for_viewer(saved_entry, can_see_gm);
    (
        StatusCode::OK,
        Json(GenerateNpcResponse {
            npc: entry,
            gm_memory: gm_memory.filter(|_| can_see_gm),
        }),
    )
        .into_response()
}

/// Materializes a canonical universal campaign starter NPC (`data/npcs/*.toml`, e.g., `dr_bandajara`, `alex_wen`)
/// with a full rules-legal `CharacterSheetRecord` into the session (`POST /api/(gm/)sessions/:session_id/npcs/load-starter`).
pub async fn load_starter_npc_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    Query(shield): Query<SpoilerShieldQuery>,
    headers: HeaderMap,
    Json(req): Json<LoadStarterNpcRequest>,
) -> impl IntoResponse {
    let sid = session_id.trim();
    let needle = req.npc_id.trim();
    if needle.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "npc_id is required");
    }
    let (user_opt, is_gm_service, can_see_gm) =
        match prepare_npc_request(&state, &headers, sid, true, shield.spoiler_shield).await {
            Ok(v) => v,
            Err(resp) => return *resp,
        };

    let owner_uid = user_opt
        .as_ref()
        .map(|u| u.user_id.clone())
        .unwrap_or_else(|| "ai_gm_service".to_string());
    let author_name = user_opt
        .as_ref()
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| "Satya AI GM".to_string());

    let Some(table) =
        crate::campaign_lore::find_pack_starter_npc_table(Some(state.campaign_pack.as_str()), needle)
    else {
        return json_err(
            StatusCode::NOT_FOUND,
            format!("Campaign starter NPC '{needle}' not found in active pack"),
        );
    };
    let Some(full_entry) = crate::campaign_lore::materialize_starter_npc_entry(
        &state,
        &table,
        &owner_uid,
        req.is_persistent,
    ) else {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to materialize starter NPC '{needle}'"),
        );
    };

    let campaign_tag = req
        .campaign_tag
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(sid)
        .to_string();
    let full_dossier = full_entry
        .gm_secret_dossier
        .clone()
        .unwrap_or_else(|| full_entry.public_summary.clone());
    let now_us = Utc::now().timestamp_micros();
    let caller_uid = user_opt.as_ref().map(|u| u.user_id.as_str());
    let ctx = NpcVaultWriteContext {
        sid,
        campaign_tag: &campaign_tag,
        owner_uid: &owner_uid,
        author_name: &author_name,
        is_gm_service,
        caller_uid,
        reuse_existing_id: false,
    };
    let (saved_entry, gm_memory) =
        match persist_npc_entry_to_vault(&state, &ctx, full_entry, &full_dossier, now_us).await {
            Ok(res) => res,
            Err(resp) => return *resp,
        };

    let entry = redact_npc_entry_for_viewer(saved_entry, can_see_gm);
    (
        StatusCode::OK,
        Json(GenerateNpcResponse {
            npc: entry,
            gm_memory: gm_memory.filter(|_| can_see_gm),
        }),
    )
        .into_response()
}

/// Lists all NPCs for `session_id` (and matching persistent `campaign_tag`) alongside the active pack's
/// universal Campaign Starter NPC catalog, automatically redacting GM-only stats/secrets when hidden (`GET /api/sessions/:session_id/npcs`).
pub async fn list_session_npcs_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    Query(shield): Query<SpoilerShieldQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let sid = session_id.trim();
    let (_, _, can_see_gm) =
        match prepare_npc_request(&state, &headers, sid, false, shield.spoiler_shield).await {
            Ok(v) => v,
            Err(resp) => return *resp,
        };

    let npcs = match query_session_npc_rows(state.auth.db_pool(), sid, can_see_gm).await {
        Ok(list) => list,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let pack = crate::campaign_lore::resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let campaign_npcs =
        crate::campaign_lore::load_pack_starter_npcs(Some(state.campaign_pack.as_str()), can_see_gm);

    (
        StatusCode::OK,
        Json(SessionNpcsResponse {
            session_id: sid.to_string(),
            campaign_tag: sid.to_string(),
            viewer_can_see_gm_secrets: can_see_gm,
            npcs,
            campaign_pack: pack.pack_name,
            campaign_npcs,
        }),
    )
        .into_response()
}

/// Patches an existing session NPC (or materializes a pack starter NPC into `session_npcs`)
/// via `PATCH /api/(gm/)sessions/:session_id/npcs/:character_id`.
pub async fn update_session_npc_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path((session_id, character_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<UpdateSessionNpcRequest>,
) -> impl IntoResponse {
    let sid = session_id.trim();
    let cid = character_id.trim();
    if cid.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "character_id is required");
    }
    let (user_opt, is_gm_service, _) =
        match prepare_npc_request(&state, &headers, sid, true, false).await {
            Ok(v) => v,
            Err(resp) => return *resp,
        };
    let owner_uid = user_opt
        .as_ref()
        .map(|u| u.user_id.clone())
        .unwrap_or_else(|| "ai_gm_service".to_string());

    let find_sql = format!(
        "SELECT session_id, campaign_tag, {NPC_SELECT_COLS} FROM session_npcs WHERE character_id = ?1 AND (session_id = ?2 OR campaign_tag = ?2 COLLATE NOCASE) LIMIT 1"
    );
    let existing_row = sqlx::query(&find_sql)
        .bind(cid)
        .bind(sid)
        .fetch_optional(state.auth.db_pool())
        .await
        .ok()
        .flatten();

    let (row_sid, row_ctag, mut entry) = if let Some(row) = existing_row {
        let r_sid: String = row
            .try_get("session_id")
            .unwrap_or_else(|_| sid.to_string());
        let r_ctag: String = row
            .try_get("campaign_tag")
            .unwrap_or_else(|_| sid.to_string());
        (r_sid, r_ctag, decode_npc_row(&row, true))
    } else if let Some(table) =
        crate::campaign_lore::find_pack_starter_npc_table(Some(state.campaign_pack.as_str()), cid)
        && let Some(starter) =
            crate::campaign_lore::materialize_starter_npc_entry(&state, &table, &owner_uid, true)
    {
        (sid.to_string(), sid.to_string(), starter)
    } else {
        return json_err(
            StatusCode::NOT_FOUND,
            format!("NPC '{cid}' not found in session '{sid}'"),
        );
    };

    if let Some(d) = req.disposition {
        entry.disposition = Some(d);
    }
    if let Some(cond) = req.apparent_condition {
        entry.apparent_condition = cond;
    }
    if let Some(pub_sum) = req.public_summary {
        entry.public_summary = pub_sum;
    }
    if let Some(agenda) = req.immediate_agenda {
        entry.immediate_agenda = Some(agenda);
    }
    if let Some(append_txt) = req
        .gm_secret_dossier_append
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let mut dossier = entry.gm_secret_dossier.take().unwrap_or_default();
        if !dossier.is_empty() {
            dossier.push('\n');
        }
        dossier.push_str(append_txt);
        entry.gm_secret_dossier = Some(dossier);
    }
    if let Some(persist) = req.is_persistent {
        entry.is_persistent = persist;
    }

    let now_us = Utc::now().timestamp_micros();
    if let Err(err) = upsert_session_npc_row(
        state.auth.db_pool(),
        &row_sid,
        &row_ctag,
        &owner_uid,
        &entry,
        now_us,
    )
    .await
    {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to persist updated NPC to session_npcs: {err}"),
        );
    }

    if let Some(persist) = req.is_persistent {
        let _ = sqlx::query(
            "UPDATE session_npcs SET is_persistent = ?1 WHERE LOWER(TRIM(character_name)) = LOWER(TRIM(?2)) AND (session_id = ?3 OR campaign_tag = ?3 COLLATE NOCASE)",
        )
        .bind(i32::from(persist))
        .bind(&entry.character_name)
        .bind(sid)
        .execute(state.auth.db_pool())
        .await;

        let caller_uid = user_opt.as_ref().map(|u| u.user_id.as_str());
        let _ = state
            .auth
            .toggle_session_gm_memory_persistence(
                sid,
                caller_uid,
                is_gm_service,
                &format!("mem_{}", entry.character_id),
                persist,
                None,
            )
            .await;
    }

    broadcast_npc_roster_updated(&state, sid).await;
    (StatusCode::OK, Json(entry)).into_response()
}

/// Deletes a session NPC (plus any same-name duplicate rows, `characters`/`session_characters` links,
/// and companion `mem_<character_id>` GM Memory note) via `DELETE /api/(gm/)sessions/:session_id/npcs/:character_id`.
pub async fn delete_session_npc_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path((session_id, character_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let sid = session_id.trim();
    let cid = character_id.trim();
    if cid.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "character_id is required");
    }
    let (user_opt, is_gm_service, _) =
        match prepare_npc_request(&state, &headers, sid, true, false).await {
            Ok(v) => v,
            Err(resp) => return *resp,
        };

    let pool = state.auth.db_pool();
    let mut target_ids = vec![cid.to_string()];
    if let Ok(Some(npc_name)) =
        sqlx::query_scalar::<_, String>("SELECT character_name FROM session_npcs WHERE character_id = ?1")
            .bind(cid)
            .fetch_optional(pool)
            .await
        && let Ok(dup_ids) = sqlx::query_scalar::<_, String>(
            "SELECT character_id FROM session_npcs WHERE LOWER(TRIM(character_name)) = LOWER(TRIM(?1)) AND (session_id = ?2 OR campaign_tag = ?2 COLLATE NOCASE)",
        )
        .bind(&npc_name)
        .bind(sid)
        .fetch_all(pool)
        .await
    {
        for id in dup_ids {
            if !target_ids.contains(&id) {
                target_ids.push(id);
            }
        }
    }

    let caller_uid = user_opt.as_ref().map(|u| u.user_id.as_str());
    let mut deleted = false;
    for id in target_ids {
        deleted |= delete_npc_records_by_id(&state, sid, caller_uid, is_gm_service, &id).await;
    }

    if deleted {
        broadcast_npc_roster_updated(&state, sid).await;
    }

    (
        StatusCode::OK,
        Json(json!({ "status": "ok", "character_id": cid, "deleted": deleted })),
    )
        .into_response()
}
