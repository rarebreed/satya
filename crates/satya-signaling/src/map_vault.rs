//! Saved Battlemap Vault (`SQLite` `saved_maps` + `~/Satya/maps/*.satyamap` local mirror),
//! `.satyamap` JSON bundle export, and map validation/sanitization (`/api/maps/*`).

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use chrono::Utc;
use satya_maps::map_continuation::{MAX_STITCHED_MAP_H, MAX_STITCHED_MAP_W};
use satya_maps::tactical_map::MIN_SECTOR_DIM;
use satya_maps::tactical_map_geom::migrate_legacy_tiles_to_vector;
use satya_schema::{
    EdgeSide, GmMapUpdateRequest, MapSurfaceFeature, MapWallEdge, Participant,
    SAVED_MAP_FORMAT_VERSION, SaveMapRequest, SaveMapResponse, SavedMapBundle, SavedMapEvent,
    SavedMapEventType, SavedMapListResponse, SavedMapRecordResponse, SavedMapSummary,
    SessionMode, SessionRole, SurfaceShapeKind, sanitize_portrait_url, subjects,
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::UserAccount;
use crate::characters::sha256_digest;
use crate::state::SignalingAppState;

const MAX_MAP_TITLE_CHARS: usize = 120;
const MAX_MAP_NOTES_CHARS: usize = 16_384;

fn resolve_local_maps_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("SATYA_LOCAL_MAPS_DIR")
        && !custom.trim().is_empty()
    {
        return PathBuf::from(custom.trim());
    }
    #[cfg(test)]
    {
        std::env::temp_dir().join("satya_test_maps")
    }
    #[cfg(not(test))]
    {
        satya_storage::default_local_satya_maps_dir()
    }
}

fn sanitize_slug(raw: &str) -> String {
    let s: String = raw
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
        "battlemap".to_string()
    } else {
        trimmed.chars().take(40).collect()
    }
}

fn edge_in_bounds(w: i32, h: i32, e: &MapWallEdge) -> bool {
    match e.side {
        EdgeSide::North => (0..w).contains(&e.x) && (0..=h).contains(&e.y),
        EdgeSide::West => (0..=w).contains(&e.x) && (0..h).contains(&e.y),
    }
}

fn validate_surface_pts(sf: &MapSurfaceFeature) -> Result<(), String> {
    let min_pts = match sf.shape {
        SurfaceShapeKind::Polygon | SurfaceShapeKind::OrganicClosed => 3,
        SurfaceShapeKind::RibbonSpline => 2,
    };
    if sf.points_cm.len() < min_pts {
        return Err(format!(
            "Surface feature '{}' requires at least {min_pts} control points",
            sf.kind.label()
        ));
    }
    Ok(())
}

/// Validates and sanitizes a full [`GmMapUpdateRequest`] before persisting to `saved_maps`
/// or loading into an active session.
///
/// Intentionally preserves empty `wall_edges`/`props`/`surfaces` when `tiles` is empty so
/// user-authored blank or cleared maps are never overwritten by procedural generation.
pub(crate) fn sanitize_and_validate_map(map: &mut GmMapUpdateRequest) -> Result<(), String> {
    if !(MIN_SECTOR_DIM..=MAX_STITCHED_MAP_W).contains(&map.grid_width)
        || !(MIN_SECTOR_DIM..=MAX_STITCHED_MAP_H).contains(&map.grid_height)
    {
        return Err(format!(
            "Map dimensions {}x{} out of valid bounds ({MIN_SECTOR_DIM}..={MAX_STITCHED_MAP_W} x {MIN_SECTOR_DIM}..={MAX_STITCHED_MAP_H})",
            map.grid_width, map.grid_height
        ));
    }
    map.cell_size_cm = if map.cell_size_cm <= 0 {
        100
    } else {
        map.cell_size_cm.clamp(25, 250)
    };

    let trimmed_title = map.title.trim();
    map.title = if trimmed_title.is_empty() {
        "Untitled Battlemap".to_string()
    } else {
        trimmed_title.chars().take(MAX_MAP_TITLE_CHARS).collect()
    };

    if let Some(notes) = map.notes.as_mut()
        && notes.chars().count() > MAX_MAP_NOTES_CHARS
    {
        *notes = notes.chars().take(MAX_MAP_NOTES_CHARS).collect();
    }

    if !map.image_url.trim().is_empty() {
        map.image_url = sanitize_portrait_url(&map.image_url).unwrap_or_default();
    } else {
        map.image_url.clear();
    }
    for tv in &mut map.token_visuals {
        tv.portrait_url = tv.portrait_url.as_deref().and_then(sanitize_portrait_url);
    }

    if !map.tiles.is_empty() {
        migrate_legacy_tiles_to_vector(
            &map.tiles,
            map.grid_width,
            map.grid_height,
            &mut map.wall_edges,
            &mut map.props,
        );
        map.tiles.clear();
    }

    for edge in &map.wall_edges {
        if !edge_in_bounds(map.grid_width, map.grid_height, edge) {
            return Err("Wall edge out of grid bounds".to_string());
        }
    }
    for sf in &map.surfaces {
        validate_surface_pts(sf)?;
    }
    for fl in &map.floors {
        for edge in &fl.wall_edges {
            if !edge_in_bounds(map.grid_width, map.grid_height, edge) {
                return Err(format!("Wall edge on floor '{}' out of grid bounds", fl.label));
            }
        }
        for sf in &fl.surfaces {
            validate_surface_pts(sf)?;
        }
    }

    map.sync_active_floor_snapshot();
    Ok(())
}

fn write_local_satyamap_bundle(
    map: &GmMapUpdateRequest,
    saved_at_us: i64,
) -> std::io::Result<PathBuf> {
    let dir = resolve_local_maps_dir();
    std::fs::create_dir_all(&dir)?;
    let slug = sanitize_slug(&map.title);
    let raw_short: String = map
        .map_id
        .trim_start_matches("map_")
        .trim_start_matches("map-")
        .chars()
        .take(8)
        .collect();
    let short_id: String = raw_short
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        .collect();
    let filename = if short_id.is_empty() {
        format!("{slug}.satyamap")
    } else {
        format!("{slug}_{short_id}.satyamap")
    };
    let path = dir.join(filename);
    let bundle = SavedMapBundle {
        format_version: SAVED_MAP_FORMAT_VERSION,
        saved_at_us,
        map: map.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&bundle)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(&path, bytes)?;
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

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha256_digest(bytes);
    let mut hex = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

fn vault_actor_for_user(user: &UserAccount) -> Participant {
    Participant::human(
        &user.user_id,
        &user.username,
        &user.display_name,
        SessionRole::Gm,
    )
}

/// Emits a [`SavedMapEvent`] (`v1`, `schema_id = 14`) to Apache Iggy topic `saved-maps` (Topic 11).
pub(crate) async fn emit_saved_map_event(
    state: &SignalingAppState,
    actor: &Participant,
    event_type: SavedMapEventType,
    summary: &SavedMapSummary,
    canonical_map_json: &str,
    session_id_opt: Option<&str>,
) {
    let sid = session_id_opt
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("map-vault");
    let Ok(metadata) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::SAVED_MAP,
        1,
        sid,
        SessionMode::Ttrpg,
    ) else {
        return;
    };
    let include_snapshot = !matches!(event_type, SavedMapEventType::MapDeleted);
    let ev = SavedMapEvent {
        metadata,
        map_event_type: event_type,
        actor: actor.clone(),
        map_id: summary.map_id.clone(),
        title: summary.title.clone(),
        grid_width: summary.grid_width,
        grid_height: summary.grid_height,
        cell_size_cm: summary.cell_size_cm,
        theme_preset: summary.theme_preset.clone(),
        floor_count: summary.floor_count as i32,
        prop_count: summary.prop_count as i32,
        wall_count: summary.wall_count as i32,
        surface_count: summary.surface_count as i32,
        content_sha256: sha256_hex(canonical_map_json.as_bytes()),
        map_json: include_snapshot.then(|| canonical_map_json.to_string()),
    };
    let _ = state.storage.log_saved_map(&ev).await;
}

/// Lists saved battlemaps owned by the authenticated user (`GET /api/maps`).
pub async fn list_saved_maps_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    let rows = match sqlx::query(
        r#"
        SELECT map_id, title, theme_preset, grid_width, grid_height, cell_size_cm,
               floor_count, prop_count, wall_count, surface_count, created_at_us, updated_at_us
        FROM saved_maps
        WHERE owner_user_id = ?1
        ORDER BY updated_at_us DESC
        "#,
    )
    .bind(&user.user_id)
    .fetch_all(&state.auth.db_pool)
    .await
    {
        Ok(r) => r,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Database query failed: {err}") })),
            )
                .into_response();
        }
    };

    let maps = rows
        .into_iter()
        .map(|r| SavedMapSummary {
            map_id: r.get("map_id"),
            title: r.get("title"),
            theme_preset: r.get("theme_preset"),
            grid_width: r.get("grid_width"),
            grid_height: r.get("grid_height"),
            cell_size_cm: r.get("cell_size_cm"),
            floor_count: r.get::<i64, _>("floor_count").max(1) as usize,
            prop_count: r.get::<i64, _>("prop_count").max(0) as usize,
            wall_count: r.get::<i64, _>("wall_count").max(0) as usize,
            surface_count: r.get::<i64, _>("surface_count").max(0) as usize,
            created_at_us: r.get("created_at_us"),
            updated_at_us: r.get("updated_at_us"),
        })
        .collect();

    (
        StatusCode::OK,
        Json(SavedMapListResponse {
            maps,
            local_directory: Some(resolve_local_maps_dir().display().to_string()),
        }),
    )
        .into_response()
}

/// Saves or updates a battlemap in SQLite `saved_maps` and mirrors a `.satyamap` bundle
/// to `~/Satya/maps/*.satyamap` (`POST /api/maps`).
pub async fn save_map_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(mut req): Json<SaveMapRequest>,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };

    if let Err(msg) = sanitize_and_validate_map(&mut req.map) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response();
    }

    let now_us = Utc::now().timestamp_micros();
    let incoming_id = req.map.map_id.trim().to_string();
    if !incoming_id.is_empty()
        && let Err(msg) = crate::security::validate_safe_resource_id("map_id", &incoming_id)
    {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response();
    }
    let is_ephemeral_id =
        incoming_id.is_empty() || incoming_id == "map-default" || incoming_id == "map-starter";

    let mut created_at_us = now_us;
    let (resolved_id, is_update) = if req.save_as_new || is_ephemeral_id {
        (format!("map_{}", Uuid::now_v7()), false)
    } else {
        match sqlx::query("SELECT owner_user_id, created_at_us FROM saved_maps WHERE map_id = ?1")
            .bind(&incoming_id)
            .fetch_optional(&state.auth.db_pool)
            .await
        {
            Ok(Some(row)) => {
                let owner: String = row.get("owner_user_id");
                if owner == user.user_id {
                    created_at_us = row.get("created_at_us");
                    (incoming_id, true)
                } else {
                    (format!("map_{}", Uuid::now_v7()), false)
                }
            }
            Ok(None) => (incoming_id, false),
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Database lookup failed: {err}") })),
                )
                    .into_response();
            }
        }
    };

    req.map.map_id = resolved_id;
    let summary = SavedMapSummary::from_map(&req.map, created_at_us, now_us);
    let map_json = match serde_json::to_string(&req.map) {
        Ok(s) => s,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to serialize battlemap: {err}") })),
            )
                .into_response();
        }
    };

    if let Err(err) = sqlx::query(
        r#"
        INSERT INTO saved_maps (
            map_id, owner_user_id, title, theme_preset, grid_width, grid_height, cell_size_cm,
            floor_count, prop_count, wall_count, surface_count, map_json, created_at_us, updated_at_us
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(map_id) DO UPDATE SET
            title         = excluded.title,
            theme_preset  = excluded.theme_preset,
            grid_width    = excluded.grid_width,
            grid_height   = excluded.grid_height,
            cell_size_cm  = excluded.cell_size_cm,
            floor_count   = excluded.floor_count,
            prop_count    = excluded.prop_count,
            wall_count    = excluded.wall_count,
            surface_count = excluded.surface_count,
            map_json      = excluded.map_json,
            updated_at_us = excluded.updated_at_us
        "#,
    )
    .bind(&summary.map_id)
    .bind(&user.user_id)
    .bind(&summary.title)
    .bind(&summary.theme_preset)
    .bind(summary.grid_width)
    .bind(summary.grid_height)
    .bind(summary.cell_size_cm)
    .bind(summary.floor_count as i64)
    .bind(summary.prop_count as i64)
    .bind(summary.wall_count as i64)
    .bind(summary.surface_count as i64)
    .bind(&map_json)
    .bind(created_at_us)
    .bind(now_us)
    .execute(&state.auth.db_pool)
    .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to persist saved map: {err}") })),
        )
            .into_response();
    }

    let local_file_path = write_local_satyamap_bundle(&req.map, now_us)
        .ok()
        .map(|p| p.display().to_string());

    let actor = vault_actor_for_user(&user);
    let event_type = if is_update {
        SavedMapEventType::MapUpdated
    } else {
        SavedMapEventType::MapSaved
    };
    emit_saved_map_event(&state, &actor, event_type, &summary, &map_json, None).await;

    (
        StatusCode::OK,
        Json(SaveMapResponse {
            summary,
            map: req.map,
            local_file_path,
        }),
    )
        .into_response()
}

async fn load_owned_saved_map(
    state: &SignalingAppState,
    user_id: &str,
    map_id: &str,
) -> Result<(SavedMapRecordResponse, String), (StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query(
        r#"
        SELECT map_json, created_at_us, updated_at_us
        FROM saved_maps
        WHERE map_id = ?1 AND owner_user_id = ?2
        "#,
    )
    .bind(map_id)
    .bind(user_id)
    .fetch_optional(&state.auth.db_pool)
    .await
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Database lookup failed: {err}") })),
        )
    })?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Saved map '{map_id}' not found") })),
        )
    })?;

    let map_json: String = row.get("map_json");
    let created_at_us: i64 = row.get("created_at_us");
    let updated_at_us: i64 = row.get("updated_at_us");
    let map: GmMapUpdateRequest = serde_json::from_str(&map_json).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Stored map JSON is corrupt: {err}") })),
        )
    })?;
    let summary = SavedMapSummary::from_map(&map, created_at_us, updated_at_us);
    Ok((SavedMapRecordResponse { summary, map }, map_json))
}

/// Fetches a single saved battlemap owned by the authenticated user (`GET /api/maps/:map_id`).
pub async fn get_saved_map_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(map_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    match load_owned_saved_map(&state, &user.user_id, &map_id).await {
        Ok((rec, map_json)) => {
            let actor = vault_actor_for_user(&user);
            emit_saved_map_event(
                &state,
                &actor,
                SavedMapEventType::MapLoaded,
                &rec.summary,
                &map_json,
                None,
            )
            .await;
            (StatusCode::OK, Json(rec)).into_response()
        }
        Err(resp) => resp.into_response(),
    }
}

/// Deletes a saved battlemap owned by the authenticated user (`DELETE /api/maps/:map_id`).
pub async fn delete_saved_map_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(map_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let (rec, map_json) = match load_owned_saved_map(&state, &user.user_id, &map_id).await {
        Ok(pair) => pair,
        Err(resp) => return resp.into_response(),
    };
    let res = match sqlx::query("DELETE FROM saved_maps WHERE map_id = ?1 AND owner_user_id = ?2")
        .bind(&map_id)
        .bind(&user.user_id)
        .execute(&state.auth.db_pool)
        .await
    {
        Ok(r) => r,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to delete saved map: {err}") })),
            )
                .into_response();
        }
    };
    if res.rows_affected() == 0 {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Saved map '{map_id}' not found") })),
        )
            .into_response();
    }
    let actor = vault_actor_for_user(&user);
    emit_saved_map_event(
        &state,
        &actor,
        SavedMapEventType::MapDeleted,
        &rec.summary,
        &map_json,
        None,
    )
    .await;
    (
        StatusCode::OK,
        Json(json!({ "deleted": true, "map_id": map_id })),
    )
        .into_response()
}

/// Exports a saved battlemap as a portable `.satyamap` JSON bundle (`GET /api/maps/:map_id/export`).
pub async fn export_saved_map_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(map_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match require_authenticated_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp.into_response(),
    };
    let (rec, map_json) = match load_owned_saved_map(&state, &user.user_id, &map_id).await {
        Ok(r) => r,
        Err(resp) => return resp.into_response(),
    };
    let bundle = SavedMapBundle {
        format_version: SAVED_MAP_FORMAT_VERSION,
        saved_at_us: rec.summary.updated_at_us,
        map: rec.map,
    };
    let bytes = match serde_json::to_vec_pretty(&bundle) {
        Ok(b) => b,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to serialize .satyamap bundle: {err}") })),
            )
                .into_response();
        }
    };
    let actor = vault_actor_for_user(&user);
    emit_saved_map_event(
        &state,
        &actor,
        SavedMapEventType::MapExported,
        &rec.summary,
        &map_json,
        None,
    )
    .await;
    let slug = sanitize_slug(&rec.summary.title);
    let disposition = format!("attachment; filename=\"{slug}.satyamap\"");
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::CONTENT_DISPOSITION, disposition.as_str()),
        ],
        Body::from(bytes),
    )
        .into_response()
}
