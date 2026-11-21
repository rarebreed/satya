//! Session World Clocks, Operational `Heat` (`0-10`), and Jurisdiction [`EconomicTier`]
//! Ledger Persistence & Role-Filtered Endpoints:
//! - `GET  /api/sessions/:session_id/world-ledger`
//! - `POST /api/sessions/:session_id/world-ledger`
//! - `GET  /api/gm/sessions/:session_id/world-ledger`
//! - `POST /api/gm/sessions/:session_id/world-ledger`
//!
//! Couples campaign-pack faction progress clocks with [`satya_maps::default_zoning_and_tier`]
//! and [`satya_campaign::apply_world_ledger_update`], automatically stripping `gm_secret_agenda`
//! for non-GM player viewers in human-GM sessions.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use satya_campaign::{
    apply_world_ledger_update, filter_world_ledger_for_viewer, seed_default_world_ledger,
};
use satya_maps::SectorPreset;
use satya_maps::map_gen_zoning::default_zoning_and_tier;
use satya_schema::{SessionWorldLedger, UpdateWorldLedgerRequest, WorldLedgerUpdateResponse};
use serde_json::json;
use sqlx::{Row, SqlitePool};

use crate::auth::UserAccount;
use crate::campaign_lore::{resolve_active_pack_name, resolve_pack_defaults};
use crate::gm_api::verify_gm_service_header;
use crate::npcs::can_caller_view_gm_secrets;
use crate::state::SignalingAppState;

/// Ensures the `session_world_ledger` SQLite table exists idempotently.
pub(crate) async fn ensure_session_world_ledger_table(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS session_world_ledger (
            session_id    TEXT PRIMARY KEY,
            ledger_json   TEXT NOT NULL,
            updated_at_us INTEGER NOT NULL
        );
        "#,
    )
    .execute(pool)
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

/// Loads the persistent [`SessionWorldLedger`] for `session_id`, seeding canonical
/// campaign-pack faction clocks and map [`EconomicTier`] jurisdiction defaults if absent.
pub async fn get_or_init_world_ledger(
    state: &SignalingAppState,
    session_id: &str,
) -> SessionWorldLedger {
    let sid = session_id.trim();
    let pool = state.auth.db_pool();
    let _ = ensure_session_world_ledger_table(pool).await;

    if let Ok(Some(row)) =
        sqlx::query("SELECT ledger_json FROM session_world_ledger WHERE session_id = ?1")
            .bind(sid)
            .fetch_optional(pool)
            .await
    {
        let raw_json: String = row.try_get("ledger_json").unwrap_or_default();
        if let Ok(ledger) = serde_json::from_str::<SessionWorldLedger>(&raw_json) {
            return ledger;
        }
    }

    let gm_ctx_opt = state.auth.get_session_gm_context(sid).await;
    let pack_defaults = resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let pack_id = resolve_active_pack_name(Some(state.campaign_pack.as_str()));
    let (campaign_tag, sector_preset) = match gm_ctx_opt {
        Some((ctx, _)) => {
            let preset = ctx
                .active_map
                .as_ref()
                .and_then(|m| m.theme_preset.as_deref())
                .and_then(SectorPreset::from_code_str)
                .unwrap_or(pack_defaults.default_sector_preset);
            let tag = ctx
                .active_map
                .as_ref()
                .map(|m| m.map_id.clone())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| sid.to_string());
            (tag, preset)
        }
        None => (sid.to_string(), pack_defaults.default_sector_preset),
    };

    let (zoning, tier) = default_zoning_and_tier(sector_preset);
    let mut ledger = seed_default_world_ledger(sid, &campaign_tag, &pack_id, tier, zoning);
    ledger.updated_at_us = Utc::now().timestamp_micros();

    if let Ok(encoded) = serde_json::to_string(&ledger) {
        let _ = sqlx::query(
            r#"
            INSERT INTO session_world_ledger (session_id, ledger_json, updated_at_us)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(session_id) DO NOTHING
            "#,
        )
        .bind(sid)
        .bind(encoded)
        .bind(ledger.updated_at_us)
        .execute(pool)
        .await;
    }

    ledger
}

/// Applies `req` to `session_id`'s [`SessionWorldLedger`], persists the updated snapshot,
/// and returns [`WorldLedgerUpdateResponse`] with any triggered world alerts.
pub async fn update_world_ledger(
    state: &SignalingAppState,
    session_id: &str,
    req: &UpdateWorldLedgerRequest,
) -> WorldLedgerUpdateResponse {
    let sid = session_id.trim();
    let mut ledger = get_or_init_world_ledger(state, sid).await;
    let triggered_events = apply_world_ledger_update(&mut ledger, req);
    ledger.updated_at_us = Utc::now().timestamp_micros();

    let pool = state.auth.db_pool();
    let _ = ensure_session_world_ledger_table(pool).await;
    if let Ok(encoded) = serde_json::to_string(&ledger) {
        let _ = sqlx::query(
            r#"
            INSERT INTO session_world_ledger (session_id, ledger_json, updated_at_us)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(session_id) DO UPDATE SET
                ledger_json   = excluded.ledger_json,
                updated_at_us = excluded.updated_at_us
            "#,
        )
        .bind(sid)
        .bind(encoded)
        .bind(ledger.updated_at_us)
        .execute(pool)
        .await;
    }

    WorldLedgerUpdateResponse {
        ledger,
        triggered_events,
    }
}

/// Fetches the role-filtered [`SessionWorldLedger`] for `session_id`
/// (`GET /api/sessions/:session_id/world-ledger` or `/api/gm/sessions/:session_id/world-ledger`).
pub async fn get_session_world_ledger_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user_opt, is_gm_service) = match resolve_caller(&state, &headers).await {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };
    let sid = session_id.trim();
    if let Some(ref user) = user_opt {
        let roster = state.auth.get_session_roster(sid).await;
        if !roster.is_empty() && !roster.iter().any(|p| p.user_id == user.user_id) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Caller is not an active participant in this session" })),
            )
                .into_response();
        }
    }

    let can_see_gm =
        can_caller_view_gm_secrets(&state, sid, user_opt.as_ref(), is_gm_service).await;
    let ledger = get_or_init_world_ledger(&state, sid).await;
    (
        StatusCode::OK,
        Json(filter_world_ledger_for_viewer(&ledger, can_see_gm)),
    )
        .into_response()
}

/// Mutates `session_id`'s Operational Heat, jurisdiction tier, or faction progress clocks
/// (`POST /api/sessions/:session_id/world-ledger` or `/api/gm/sessions/:session_id/world-ledger`).
pub async fn update_session_world_ledger_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<UpdateWorldLedgerRequest>,
) -> impl IntoResponse {
    let (user_opt, is_gm_service) = match resolve_caller(&state, &headers).await {
        Ok(v) => v,
        Err(err) => return err.into_response(),
    };
    let sid = session_id.trim();
    let can_see_gm =
        can_caller_view_gm_secrets(&state, sid, user_opt.as_ref(), is_gm_service).await;
    if !can_see_gm {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the Game Master or Solo Host can mutate world clocks and Operational Heat" })),
        )
            .into_response();
    }

    let updated = update_world_ledger(&state, sid, &req).await;
    (StatusCode::OK, Json(updated)).into_response()
}
