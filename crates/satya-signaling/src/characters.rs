//! Character Persistence, `.satyachar` Portable Bundle Export/Import, Anti-Cheat Signing,
//! Multimodal Assets (`PORTRAIT_IMAGE`, `TACTICAL_TOKEN`, `DOSSIER_PDF`, `MODEL_3D_GLB`),
//! and Apache Iggy `character-sheets` Topic Event Publishing (`/api/characters/*`).

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use chrono::Utc;
use satya_engine::verify_character_sheet_legality;
use satya_schema::{
    BindCharacterSessionRequest, CharacterAssetRef, CharacterEventType, CharacterListResponse,
    CharacterSheetEvent, CharacterSheetRecord, CharacterVerificationStatus, ImportCharacterRequest,
    Participant, SaveCharacterRequest, SaveCharacterResponse, ServerWsPacket, SessionMode,
    SessionRole, UploadCharacterAssetRequest, subjects,
};
use satya_storage::default_local_satya_characters_dir;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::UserAccount;
use crate::state::SignalingAppState;

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Computes a standard FIPS 180-4 SHA-256 32-byte digest.
#[must_use]
pub fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().enumerate().take(16) {
            let j = i * 4;
            *word = u32::from_be_bytes([chunk[j], chunk[j + 1], chunk[j + 2], chunk[j + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Computes a standard RFC 2104 HMAC-SHA256 hex signature.
#[must_use]
pub fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let hashed = sha256_digest(key);
        key_block[..32].copy_from_slice(&hashed);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut o_key_pad = Vec::with_capacity(64 + 32);
    let mut i_key_pad = Vec::with_capacity(64 + message.len());
    for &b in &key_block {
        o_key_pad.push(b ^ 0x5c);
        i_key_pad.push(b ^ 0x36);
    }
    i_key_pad.extend_from_slice(message);
    let inner_hash = sha256_digest(&i_key_pad);
    o_key_pad.extend_from_slice(&inner_hash);
    hex::encode(sha256_digest(&o_key_pad))
}

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard RFC 4648 Base64 encoder.
#[must_use]
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_CHARS[((triple >> 18) & 0x3f) as usize] as char);
        out.push(B64_CHARS[((triple >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64_CHARS[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64_CHARS[(triple & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Standard RFC 4648 Base64 decoder (also strips optional `data:*;base64,` prefixes and whitespace).
pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let payload = input
        .split_once(";base64,")
        .map_or(input, |(_, b64)| b64)
        .trim();
    let mut vals = Vec::with_capacity(payload.len());
    for ch in payload.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\r' | b'\n' | b' ' | b'\t' => continue,
            _ => return Err(format!("Invalid base64 byte: 0x{ch:02x}")),
        };
        vals.push(v);
    }
    let mut out = Vec::with_capacity(vals.len() * 3 / 4);
    for chunk in vals.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let b0 = (chunk[0] << 2) | (chunk[1] >> 4);
        out.push(b0);
        if chunk.len() > 2 {
            let b1 = (chunk[1] << 4) | (chunk[2] >> 2);
            out.push(b1);
        }
        if chunk.len() > 3 {
            let b2 = (chunk[2] << 6) | chunk[3];
            out.push(b2);
        }
    }
    Ok(out)
}

/// Computes the canonical SHA-256 content hash over a [`CharacterSheetRecord`] (zeroing transient signature/timestamp fields).
#[must_use]
pub fn compute_character_content_sha256(record: &CharacterSheetRecord) -> String {
    let mut canonical = record.clone();
    canonical.verification_status = CharacterVerificationStatus::VerifiedOfficial;
    canonical.sp_budget_valid = true;
    canonical.dag_prereqs_valid = true;
    canonical.audit_violations.clear();
    canonical.content_sha256.clear();
    canonical.server_signature_hex = None;
    canonical.updated_at_us = 0;
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    hex::encode(sha256_digest(&bytes))
}

fn server_signing_secret(state: &SignalingAppState) -> Vec<u8> {
    if let Ok(env_secret) = std::env::var("SATYA_CHARACTER_SIGNING_KEY")
        && !env_secret.trim().is_empty()
    {
        return env_secret.into_bytes();
    }
    format!(
        "satya-char-hmac-v1:{}",
        state.storage.config().ai_gm.service_token
    )
    .into_bytes()
}

/// Audits a [`CharacterSheetRecord`] using `satya-engine` (`verify_character_sheet_legality`) and either:
/// - **Signs** it (`sign_as_official = true`) when saved/created via the server API, or
/// - **Verifies** its existing signature (`sign_as_official = false`) when imported from a `.satyachar` file.
pub fn audit_and_stamp_character_record(
    state: &SignalingAppState,
    record: &mut CharacterSheetRecord,
    sign_as_official: bool,
) {
    let report = verify_character_sheet_legality(record);
    record.sp_budget_valid = report.sp_budget_valid;
    record.dag_prereqs_valid = report.dag_prereqs_valid;
    record.audit_violations = report.violations.clone();
    record.content_sha256 = compute_character_content_sha256(record);

    let secret = server_signing_secret(state);
    let expected_sig = hmac_sha256_hex(&secret, record.content_sha256.as_bytes());

    if !report.is_legal {
        record.verification_status = CharacterVerificationStatus::TamperedOrOverBudget;
        record.server_signature_hex = None;
    } else if sign_as_official {
        record.verification_status = CharacterVerificationStatus::VerifiedOfficial;
        record.server_signature_hex = Some(expected_sig);
    } else if record.server_signature_hex.as_deref() == Some(expected_sig.as_str()) {
        record.verification_status = CharacterVerificationStatus::VerifiedOfficial;
    } else {
        record.verification_status = CharacterVerificationStatus::LegalCustom;
    }
}

fn sanitize_slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        "operator".to_string()
    } else {
        trimmed.chars().take(36).collect()
    }
}

fn write_local_satyachar_bundle(
    record: &CharacterSheetRecord,
    framed_bytes: &[u8],
) -> std::io::Result<PathBuf> {
    let dir = default_local_satya_characters_dir();
    std::fs::create_dir_all(&dir)?;
    let slug = sanitize_slug(&record.character_name);
    let short_id: String = record
        .character_id
        .trim_start_matches("char_")
        .chars()
        .take(8)
        .collect();
    let filename = if short_id.is_empty() {
        format!("{slug}.satyachar")
    } else {
        format!("{slug}_{short_id}.satyachar")
    };
    let path = dir.join(filename);
    std::fs::write(&path, framed_bytes)?;
    Ok(path)
}

async fn require_authenticated_user(
    state: &SignalingAppState,
    headers: &HeaderMap,
) -> Result<UserAccount, (StatusCode, Json<serde_json::Value>)> {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        ));
    };
    state.auth.verify_auth_token(auth_hdr).await.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
    })
}

async fn emit_character_sheet_event(
    state: &SignalingAppState,
    user: &UserAccount,
    event_type: CharacterEventType,
    character: &CharacterSheetRecord,
    session_id_opt: Option<&str>,
) -> anyhow::Result<Vec<u8>> {
    let sid = session_id_opt
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("character-vault");
    let meta = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::CHARACTER_SHEET,
        1,
        sid,
        SessionMode::Ttrpg,
    )?;
    let mut actor = Participant::human(
        &user.user_id,
        &user.username,
        &user.display_name,
        SessionRole::Player,
    );
    actor.character_id = Some(character.character_id.clone());

    let ev = CharacterSheetEvent {
        metadata: meta,
        character_event_type: event_type,
        actor,
        character: character.clone(),
    };
    let framed = state
        .storage
        .registry()
        .encode_framed(subjects::CHARACTER_SHEET, 1, &ev)?;
    state.storage.log_character_sheet(&ev).await?;
    Ok(framed)
}

async fn upsert_character_in_sqlite(
    state: &SignalingAppState,
    record: &CharacterSheetRecord,
) -> anyhow::Result<()> {
    let sheet_json = serde_json::to_string(record)?;
    sqlx::query(
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
    .bind(&record.character_id)
    .bind(&record.owner_user_id)
    .bind(&record.character_name)
    .bind(record.version)
    .bind(record.verification_status.as_str())
    .bind(&record.content_sha256)
    .bind(&record.server_signature_hex)
    .bind(&sheet_json)
    .bind(record.updated_at_us)
    .bind(record.updated_at_us)
    .execute(state.auth.db_pool())
    .await?;
    Ok(())
}

async fn load_character_from_sqlite(
    state: &SignalingAppState,
    character_id: &str,
) -> anyhow::Result<Option<CharacterSheetRecord>> {
    let row = sqlx::query("SELECT sheet_json FROM characters WHERE character_id = ?1")
        .bind(character_id)
        .fetch_optional(state.auth.db_pool())
        .await?;
    let Some(r) = row else {
        return Ok(None);
    };
    let raw_json: String = r.try_get("sheet_json")?;
    let rec: CharacterSheetRecord = serde_json::from_str(&raw_json)?;
    Ok(Some(rec))
}

async fn maybe_bind_and_broadcast_session(
    state: &SignalingAppState,
    user: &UserAccount,
    record: &CharacterSheetRecord,
    session_id_opt: Option<&str>,
    emit_bind_event: bool,
) {
    let Some(sid) = session_id_opt.map(str::trim).filter(|s| !s.is_empty()) else {
        return;
    };
    if let Ok((_, updated_roster)) = state
        .auth
        .bind_character_to_session(
            sid,
            &user.user_id,
            &record.character_id,
            &record.character_name,
        )
        .await
    {
        if emit_bind_event {
            let _ = emit_character_sheet_event(
                state,
                user,
                CharacterEventType::CharacterBoundToSession,
                record,
                Some(sid),
            )
            .await;
        }
        if let Ok(sync_msg) = serde_json::to_string(&ServerWsPacket::RosterSync {
            roster: updated_roster,
        }) {
            state.broadcast_to_session(sid, &sync_msg).await;
        }
    }
}

/// Lists all character sheets owned by the authenticated player (`GET /api/characters`).
pub async fn list_characters_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    let rows = match sqlx::query(
        "SELECT sheet_json FROM characters WHERE owner_user_id = ?1 ORDER BY updated_at_us DESC",
    )
    .bind(&user.user_id)
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

    let mut characters = Vec::with_capacity(rows.len());
    for row in rows {
        if let Ok(raw_json) = row.try_get::<String, _>("sheet_json")
            && let Ok(rec) = serde_json::from_str::<CharacterSheetRecord>(&raw_json)
        {
            characters.push(rec);
        }
    }

    let local_directory = default_local_satya_characters_dir().display().to_string();
    (
        StatusCode::OK,
        Json(CharacterListResponse {
            local_directory,
            characters,
        }),
    )
        .into_response()
}

/// Saves or updates a character sheet (`POST /api/characters`):
/// 1. Runs `verify_character_sheet_legality` (`satya-engine`) and signs the record.
/// 2. Persists to the SQLite `characters` table (strictly scoped to `user.user_id`).
/// 3. Emits a `CharacterSheetEvent` (`CHARACTER_CREATED` or `CHARACTER_UPDATED`) to Apache Iggy topic `character-sheets`.
/// 4. Writes the signed `.satyachar` binary bundle to `~/Satya/characters/<slug>.satyachar`.
/// 5. Optionally binds the character to `req.session_id` (emitting `CHARACTER_BOUND_TO_SESSION`).
pub async fn save_character_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(mut req): Json<SaveCharacterRequest>,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    let existing = if req.character.character_id.trim().is_empty() {
        None
    } else {
        load_character_from_sqlite(&state, &req.character.character_id)
            .await
            .ok()
            .flatten()
    };

    if let Some(prev) = &existing
        && prev.owner_user_id != user.user_id
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Cannot overwrite a character owned by another player" })),
        )
            .into_response();
    }

    let event_type = if let Some(prev) = existing {
        req.character.version = prev.version + 1;
        if req.character.assets.is_empty() && !prev.assets.is_empty() {
            req.character.assets = prev.assets;
        }
        CharacterEventType::CharacterUpdated
    } else {
        if req.character.character_id.trim().is_empty() {
            req.character.character_id = format!("char_{}", Uuid::now_v7());
        }
        req.character.version = 1;
        CharacterEventType::CharacterCreated
    };

    req.character.owner_user_id = user.user_id.clone();
    req.character.updated_at_us = Utc::now().timestamp_micros();
    audit_and_stamp_character_record(&state, &mut req.character, true);

    if let Err(e) = upsert_character_in_sqlite(&state, &req.character).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    let framed_bytes = match emit_character_sheet_event(
        &state,
        &user,
        event_type,
        &req.character,
        req.session_id.as_deref(),
    )
    .await
    {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let local_file_path = write_local_satyachar_bundle(&req.character, &framed_bytes)
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    if req.bind_to_session {
        maybe_bind_and_broadcast_session(
            &state,
            &user,
            &req.character,
            req.session_id.as_deref(),
            true,
        )
        .await;
    }

    (
        StatusCode::OK,
        Json(SaveCharacterResponse {
            character: req.character,
            local_file_path,
            satyachar_base64: base64_encode(&framed_bytes),
        }),
    )
        .into_response()
}

/// Optional query parameters for `GET /api/characters/:character_id/export`.
#[derive(Debug, Default, Deserialize)]
pub struct ExportCharacterQuery {
    pub session_id: Option<String>,
}

/// Exports a character sheet as a signed `.satyachar` binary bundle (`GET /api/characters/:character_id/export`)
/// and emits a `CHARACTER_EXPORTED` event to the Apache Iggy `character-sheets` topic.
pub async fn export_character_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Path(character_id): Path<String>,
    Query(query): Query<ExportCharacterQuery>,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    let Some(record) = load_character_from_sqlite(&state, &character_id)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Character '{character_id}' not found") })),
        )
            .into_response();
    };

    if record.owner_user_id != user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the character owner can export this character sheet" })),
        )
            .into_response();
    }

    let framed_bytes = match emit_character_sheet_event(
        &state,
        &user,
        CharacterEventType::CharacterExported,
        &record,
        query.session_id.as_deref(),
    )
    .await
    {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let _ = write_local_satyachar_bundle(&record, &framed_bytes);
    let slug = sanitize_slug(&record.character_name);
    let disposition = format!("attachment; filename=\"{slug}.satyachar\"");

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        Body::from(framed_bytes),
    )
        .into_response()
}

/// Imports a `.satyachar` binary bundle (`POST /api/characters/import`), verifies its two-layer
/// anti-cheat signature & `satya-engine` legality, persists it to SQLite and `~/Satya/characters/`,
/// and emits a `CHARACTER_IMPORTED` event to the Apache Iggy `character-sheets` topic.
pub async fn import_character_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<ImportCharacterRequest>,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    let raw_bytes = match base64_decode(&req.satyachar_base64) {
        Ok(b) if !b.is_empty() => b,
        _ => req.satyachar_base64.as_bytes().to_vec(),
    };

    let mut imported_record: CharacterSheetRecord = match state
        .storage
        .registry()
        .decode_framed::<CharacterSheetEvent>(&raw_bytes, None)
    {
        Ok((_, ev)) => ev.character,
        Err(_) => match serde_json::from_slice::<CharacterSheetRecord>(&raw_bytes) {
            Ok(rec) => rec,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Invalid .satyachar bundle: {e}") })),
                )
                    .into_response();
            }
        },
    };

    if imported_record.character_id.trim().is_empty()
        || load_character_from_sqlite(&state, &imported_record.character_id)
            .await
            .ok()
            .flatten()
            .is_some_and(|existing| existing.owner_user_id != user.user_id)
    {
        imported_record.character_id = format!("char_{}", Uuid::now_v7());
    }
    imported_record.owner_user_id = user.user_id.clone();
    imported_record.updated_at_us = Utc::now().timestamp_micros();

    // Verify signature + satya-engine legality without blindly overwriting a forged signature
    audit_and_stamp_character_record(&state, &mut imported_record, false);

    if let Err(e) = upsert_character_in_sqlite(&state, &imported_record).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    let framed_bytes = match emit_character_sheet_event(
        &state,
        &user,
        CharacterEventType::CharacterImported,
        &imported_record,
        req.session_id.as_deref(),
    )
    .await
    {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let local_file_path = write_local_satyachar_bundle(&imported_record, &framed_bytes)
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    if req.bind_to_session {
        maybe_bind_and_broadcast_session(
            &state,
            &user,
            &imported_record,
            req.session_id.as_deref(),
            true,
        )
        .await;
    }

    (
        StatusCode::OK,
        Json(SaveCharacterResponse {
            character: imported_record,
            local_file_path,
            satyachar_base64: base64_encode(&framed_bytes),
        }),
    )
        .into_response()
}

/// Binds an existing saved character sheet to an active session (`POST /api/characters/:character_id/bind-session`),
/// updating the session roster and emitting `CHARACTER_BOUND_TO_SESSION` to Apache Iggy.
pub async fn bind_character_session_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Path(character_id): Path<String>,
    Json(req): Json<BindCharacterSessionRequest>,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    let Some(record) = load_character_from_sqlite(&state, &character_id)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Character '{character_id}' not found") })),
        )
            .into_response();
    };

    if record.owner_user_id != user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Cannot bind a character owned by another player" })),
        )
            .into_response();
    }

    maybe_bind_and_broadcast_session(
        &state,
        &user,
        &record,
        Some(&req.session_id),
        true,
    )
    .await;

    (
        StatusCode::OK,
        Json(SaveCharacterResponse {
            character: record,
            local_file_path: String::new(),
            satyachar_base64: String::new(),
        }),
    )
        .into_response()
}

/// Uploads a multimodal asset (`PORTRAIT_IMAGE`, `TACTICAL_TOKEN`, `DOSSIER_PDF`, `MODEL_3D_GLB`)
/// to a character sheet (`POST /api/characters/:character_id/assets`), stores the binary in SQLite
/// `character_assets` + `data/characters/assets/` (for PyLance compaction), and emits `CHARACTER_ASSET_UPLOADED`
/// to the Apache Iggy `character-sheets` topic.
pub async fn upload_character_asset_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Path(character_id): Path<String>,
    Json(req): Json<UploadCharacterAssetRequest>,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    let Some(mut record) = load_character_from_sqlite(&state, &character_id)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Character '{character_id}' not found") })),
        )
            .into_response();
    };

    if record.owner_user_id != user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the character owner can upload assets to this character" })),
        )
            .into_response();
    }

    let blob_data = match base64_decode(&req.data_base64) {
        Ok(b) if !b.is_empty() => b,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid or empty base64 asset data" })),
            )
                .into_response();
        }
    };

    if blob_data.len() > 25 * 1024 * 1024 {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({ "error": "Character asset exceeds 25 MB limit" })),
        )
            .into_response();
    }

    let now_us = Utc::now().timestamp_micros();
    let asset_id = format!("asset_{}", Uuid::now_v7());
    let sha256_hex = hex::encode(sha256_digest(&blob_data));
    let clean_filename: String = req
        .filename
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .take(80)
        .collect();
    let filename = if clean_filename.is_empty() {
        "asset.bin".to_string()
    } else {
        clean_filename
    };

    let asset_ref = CharacterAssetRef {
        asset_id: asset_id.clone(),
        asset_kind: req.asset_kind,
        filename: filename.clone(),
        mime_type: req.mime_type.clone(),
        sha256_hex: sha256_hex.clone(),
        size_bytes: blob_data.len() as i64,
        created_at_us: now_us,
    };

    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO character_assets (
            asset_id, character_id, owner_user_id, asset_kind,
            filename, mime_type, sha256_hex, size_bytes, blob_data, created_at_us
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
    )
    .bind(&asset_ref.asset_id)
    .bind(&record.character_id)
    .bind(&user.user_id)
    .bind(asset_ref.asset_kind.as_str())
    .bind(&asset_ref.filename)
    .bind(&asset_ref.mime_type)
    .bind(&asset_ref.sha256_hex)
    .bind(asset_ref.size_bytes)
    .bind(&blob_data)
    .bind(asset_ref.created_at_us)
    .execute(state.auth.db_pool())
    .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    // Also mirror binary asset to data/characters/assets/ for PyLance multimodal dataset compaction
    let assets_dir = PathBuf::from("data").join("characters").join("assets");
    if std::fs::create_dir_all(&assets_dir).is_ok() {
        let _ = std::fs::write(assets_dir.join(format!("{asset_id}_{filename}")), &blob_data);
    }

    record.assets.push(asset_ref.clone());
    record.version += 1;
    record.updated_at_us = now_us;
    audit_and_stamp_character_record(&state, &mut record, true);
    let _ = upsert_character_in_sqlite(&state, &record).await;

    let mut local_file_path = String::new();
    let mut satyachar_base64 = String::new();
    if let Ok(framed) = emit_character_sheet_event(
        &state,
        &user,
        CharacterEventType::CharacterAssetUploaded,
        &record,
        req.session_id.as_deref(),
    )
    .await
    {
        local_file_path = write_local_satyachar_bundle(&record, &framed)
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        satyachar_base64 = base64_encode(&framed);
    }

    (
        StatusCode::OK,
        Json(SaveCharacterResponse {
            character: record,
            local_file_path,
            satyachar_base64,
        }),
    )
        .into_response()
}

/// Serves a multimodal character asset (`GET /api/characters/:character_id/assets/:asset_id`).
pub async fn get_character_asset_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Path((character_id, asset_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let row = match sqlx::query(
        "SELECT owner_user_id, mime_type, filename, blob_data FROM character_assets WHERE character_id = ?1 AND asset_id = ?2",
    )
    .bind(&character_id)
    .bind(&asset_id)
    .fetch_optional(state.auth.db_pool())
    .await
    {
        Ok(Some(r)) => r,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Character asset not found" })),
            )
                .into_response();
        }
    };

    if headers.contains_key("Authorization") {
        let Ok(user) = require_authenticated_user(&state, &headers).await else {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid auth token" })),
            )
                .into_response();
        };
        let owner_id: String = row.try_get("owner_user_id").unwrap_or_default();
        if owner_id != user.user_id {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Character asset not found" })),
            )
                .into_response();
        }
    }

    let mime_type: String = row
        .try_get("mime_type")
        .unwrap_or_else(|_| "application/octet-stream".to_string());
    let filename: String = row
        .try_get("filename")
        .unwrap_or_else(|_| "asset.bin".to_string());
    let blob_data: Vec<u8> = row.try_get("blob_data").unwrap_or_default();

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime_type),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{filename}\""),
            ),
        ],
        Body::from(blob_data),
    )
        .into_response()
}
