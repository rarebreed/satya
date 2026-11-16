//! Embodied Spatial Movement & Strictly Non-GM Public Omnisearch Endpoints for AI Players:
//! - `POST /api/player-ai/sessions/:session_id/map/move`
//! - `GET  /api/player-ai/sessions/:session_id/lookup`
//!
//! Enforces strict self-token ownership on map movement and guarantees zero leakage of
//! `GM_ONLY` tokens, GM documents/memories, NPC secret dossiers, or faction secret agendas.

use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use satya_engine::tactical_map::build_procedural_map_request;
use satya_engine::tactical_map_geom::resolve_token_destination_cm;
use satya_schema::{
    CanvasEventType, GmLookupHit, GmLookupSourceKind, GmOmnisearchResponse, MessageVisibility,
    MoveMapTokenRequest, SessionRole, TokenPlacement, TokenVisualSpec, UpdateCombatTrackerRequest,
    sanitize_portrait_url,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::campaign_lore::{append_omnisearch_lore_and_npc_hits, resolve_pack_defaults};
use crate::gm_api::verify_gm_service_header;
use crate::map_handlers::{
    broadcast_role_filtered_map, ensure_vector_map_ready, filter_map_for_role, log_map_canvas_event,
};
use crate::player_ai_api::{PlayerAiContextQuery, default_ai_player_uid};
use crate::state::SignalingAppState;

/// Query parameters for `GET /api/player-ai/sessions/:session_id/lookup`.
#[derive(Debug, Clone, Deserialize)]
pub struct PlayerAiLookupQuery {
    #[serde(default = "default_ai_player_uid")]
    pub user_id: String,
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub scope: String,
}

/// Moves an AI Player's own character miniature on the active tactical map with continuous wall
/// collision validation and `Player`-filtered broadcasts (`POST /api/player-ai/sessions/:id/map/move`).
///
/// Strictly rejects removing tokens (`remove == true`) or moving another entity's token (`403 Forbidden`).
pub async fn player_ai_move_map_token_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    Query(query): Query<PlayerAiContextQuery>,
    headers: HeaderMap,
    Json(req): Json<MoveMapTokenRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let sid = session_id.trim();
    let player_uid = query.user_id.trim();

    let Some((ctx, _)) = state.auth.get_session_gm_context(sid).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{sid}' not found") })),
        )
            .into_response();
    };

    let roster_part = ctx.roster.iter().find(|p| p.user_id == player_uid).cloned();
    let ai_entry_opt = {
        let guard = state.ai_players.read().await;
        guard.get(sid).and_then(|m| m.get(player_uid).cloned())
    };
    let actor_part = roster_part
        .clone()
        .or_else(|| ai_entry_opt.as_ref().map(|e| e.participant.clone()));

    let Some(actor) = actor_part else {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": format!("AI Player '{player_uid}' is not enrolled in session '{sid}'") })),
        )
            .into_response();
    };

    let owns_token = req.entity_id == player_uid
        || actor
            .character_id
            .as_deref()
            .is_some_and(|cid| cid == req.entity_id);
    if req.remove || !owns_token {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "AI Player may only move its own character miniature" })),
        )
            .into_response();
    }

    let mut map = ctx.active_map.unwrap_or_else(|| {
        build_procedural_map_request(
            &format!("map-{}", Uuid::now_v7()),
            resolve_pack_defaults(Some(state.campaign_pack.as_str())).default_sector_preset,
            None,
            20,
            14,
            42,
        )
    });
    ensure_vector_map_ready(&mut map);

    let (req_x_cm, req_y_cm) = req.target_cm(map.cell_size_cm);
    let (board_w_cm, board_h_cm) = map.board_size_cm();
    let (board_w_m, board_h_m) = map.board_size_m();
    if req_x_cm < 0 || req_y_cm < 0 || req_x_cm >= board_w_cm || req_y_cm >= board_h_cm {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!(
                    "Destination ({}cm, {}cm) is outside the {:.1}m x {:.1}m tactical map",
                    req_x_cm, req_y_cm, board_w_m, board_h_m
                )
            })),
        )
            .into_response();
    }
    if let Some(tf) = req.to_floor
        && !map.has_floor(tf)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("Floor {tf} does not exist on active map") })),
        )
            .into_response();
    }
    let target_floor = req
        .to_floor
        .unwrap_or_else(|| map.token_floor(&req.entity_id));
    let (final_x_cm, final_y_cm) = match if target_floor == map.active_floor {
        resolve_token_destination_cm(&map, &req.entity_id, req_x_cm, req_y_cm)
    } else {
        let mut floor_view = map.clone();
        floor_view.switch_active_floor(target_floor);
        resolve_token_destination_cm(&floor_view, &req.entity_id, req_x_cm, req_y_cm)
    } {
        Ok(coords) => coords,
        Err(msg) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response();
        }
    };

    let cell_cm = map.cell_size_cm.max(25);
    let final_gx = (final_x_cm / cell_cm).clamp(0, (map.grid_width - 1).max(0));
    let final_gy = (final_y_cm / cell_cm).clamp(0, (map.grid_height - 1).max(0));

    let mut auto_facing: Option<i32> = req.facing_deg;
    let existing_pos = map
        .tokens
        .iter()
        .find(|t| t.entity_id == req.entity_id)
        .map(|t| map.token_pos_cm(t));
    if let Some((prev_x_cm, prev_y_cm)) = existing_pos {
        if auto_facing.is_none() && (prev_x_cm != final_x_cm || prev_y_cm != final_y_cm) {
            let dx = (final_x_cm - prev_x_cm) as f32;
            let dy = (final_y_cm - prev_y_cm) as f32;
            auto_facing = Some(((dx.atan2(-dy).to_degrees().round() as i32) + 360) % 360);
        }
        if let Some(tok) = map.tokens.iter_mut().find(|t| t.entity_id == req.entity_id) {
            tok.x = final_gx;
            tok.y = final_gy;
            if let Some(lbl) = req
                .label
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                tok.label = lbl.to_string();
            }
            tok.visibility = MessageVisibility::Public;
        }
    } else {
        let lbl = req
            .label
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(actor.display_name.as_str())
            .to_string();
        map.tokens.push(TokenPlacement {
            entity_id: req.entity_id.clone(),
            label: lbl,
            x: final_gx,
            y: final_gy,
            visibility: MessageVisibility::Public,
        });
    }

    let target_elev_cm = map.floor_elevation_cm(target_floor);
    if let Some(vis_spec) = map
        .token_visuals
        .iter_mut()
        .find(|v| v.entity_id == req.entity_id)
    {
        vis_spec.pos_x_cm = Some(final_x_cm);
        vis_spec.pos_y_cm = Some(final_y_cm);
        if req.to_floor.is_some() {
            vis_spec.floor_index = target_floor;
            vis_spec.elevation_cm = target_elev_cm;
        }
        if let Some(f) = auto_facing {
            vis_spec.facing_deg = f.rem_euclid(360);
        }
        if let Some(mp) = req
            .model_preset
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            vis_spec.model_preset = mp.to_string();
        }
        if let Some(ch) = req
            .color_hex
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            vis_spec.color_hex = ch.to_string();
        }
        if let Some(sp) = req
            .stance_pose
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            vis_spec.stance_pose = sp.to_string();
        }
        if let Some(raw_url) = req.portrait_url.as_deref() {
            vis_spec.portrait_url = if raw_url.trim().is_empty() {
                None
            } else {
                sanitize_portrait_url(raw_url)
            };
        }
    } else {
        map.token_visuals.push(TokenVisualSpec {
            entity_id: req.entity_id.clone(),
            model_preset: req
                .model_preset
                .unwrap_or_else(|| "valkyrie_rig".to_string()),
            color_hex: req.color_hex.unwrap_or_else(|| "#38bdf8".to_string()),
            facing_deg: auto_facing.unwrap_or(90).rem_euclid(360),
            elevation_cm: target_elev_cm,
            floor_index: target_floor,
            stance_pose: req.stance_pose.unwrap_or_else(|| "idle".to_string()),
            portrait_url: req.portrait_url.as_deref().and_then(sanitize_portrait_url),
            pos_x_cm: Some(final_x_cm),
            pos_y_cm: Some(final_y_cm),
        });
    }

    if ctx
        .combat_tracker
        .as_ref()
        .is_some_and(|ct| ct.combatants.iter().any(|c| c.entity_id == req.entity_id))
    {
        let _ = state
            .auth
            .update_session_combat_tracker(
                sid,
                UpdateCombatTrackerRequest {
                    tick_delta: None,
                    combat_active: None,
                    entity_id: Some(req.entity_id.clone()),
                    active_stance: None,
                    next_action_tick: None,
                    fatigue_level: None,
                    wound_shock: None,
                    grid_x: Some(final_gx),
                    grid_y: Some(final_gy),
                    zone_label: None,
                },
            )
            .await;
    }

    let _ = state.auth.set_session_active_map(sid, map.clone()).await;
    log_map_canvas_event(
        &state,
        sid,
        ctx.session_mode,
        CanvasEventType::TokenMoved,
        &actor,
        &map,
    )
    .await;
    broadcast_role_filtered_map(&state, sid, &ctx.roster, &map).await;

    (
        StatusCode::OK,
        Json(filter_map_for_role(&map, SessionRole::Player)),
    )
        .into_response()
}

/// Strictly non-GM omnisearch endpoint for AI Players (`GET /api/player-ai/sessions/:id/lookup`).
///
/// Searches public Satya rules chapters, redacted session/starter NPC summaries, and public
/// faction rumors/Operational Heat. Never reads `GmDocument`, `GmMemory`, or `is_gm = true` lore.
pub async fn player_ai_public_lookup_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    Query(query): Query<PlayerAiLookupQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let sid = session_id.trim();
    if state.auth.get_session_gm_context(sid).await.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{sid}' not found") })),
        )
            .into_response();
    }

    let norm_scope = if query.scope.trim().is_empty() {
        "ALL".to_string()
    } else {
        query.scope.trim().to_ascii_uppercase()
    };
    let q_trim = query.q.trim();
    let mut hits: Vec<GmLookupHit> = Vec::new();
    let mut seen_ids = HashSet::new();

    if matches!(norm_scope.as_str(), "ALL" | "RULES" | "RULES_BOOK") {
        let rule_query = if q_trim.is_empty() { "cascade" } else { q_trim };
        for r in satya_engine::search_rules_book(rule_query, 5) {
            if seen_ids.insert(r.chapter_id.clone()) {
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::RulesBook,
                    item_id: r.chapter_id,
                    title: format!("{} — {}", r.chapter_title, r.section_heading),
                    subtitle: format!("Rules Book ({})", r.relative_path),
                    snippet: r.snippet.chars().take(360).collect(),
                    is_persistent: true,
                    score: r.score + 15,
                });
            }
        }
    }

    append_omnisearch_lore_and_npc_hits(&state, sid, false, q_trim, &norm_scope, &mut hits).await;

    (
        StatusCode::OK,
        Json(GmOmnisearchResponse {
            session_id: sid.to_string(),
            campaign_tag: sid.to_string(),
            query: q_trim.to_string(),
            scope: norm_scope,
            hits,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_router;
    use satya_schema::{
        AiPlayerConfig, AuthSuccessResponse, GmDocumentRecord, GmDocumentSource,
        GmMapUpdateRequest, SessionMode, SessionWorldLedger, TacticalTileKind,
        WorldLedgerUpdateResponse,
    };
    use satya_storage::StorageConfig;

    async fn build_test_app_and_users(
        suffix: &str,
    ) -> (
        std::path::PathBuf,
        Arc<SignalingAppState>,
        axum::Router,
        AuthSuccessResponse,
        AuthSuccessResponse,
    ) {
        let mut cfg = StorageConfig::default();
        let temp_dir = std::env::temp_dir().join(format!("satya_sig_{suffix}_{}", Uuid::now_v7()));
        for topic_cfg in cfg.topics.values_mut() {
            topic_cfg.local_path = temp_dir
                .join(&topic_cfg.local_path)
                .to_string_lossy()
                .to_string();
        }
        cfg.auth.database_url = temp_dir
            .join("users_spatial.db")
            .to_string_lossy()
            .to_string();

        let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
        let app = build_router(state.clone());
        let gm = state
            .auth
            .authenticate_user("gm_sean", "satya-truth-2026")
            .await
            .unwrap();
        let p1 = state
            .auth
            .authenticate_user("sara_valkyrie", "satya-truth-2026")
            .await
            .unwrap();
        let gm_user = state.auth.verify_auth_token(&gm.auth_token).await.unwrap();
        let p1_user = state.auth.verify_auth_token(&p1.auth_token).await.unwrap();

        let sid = format!("sess-{suffix}");
        state
            .auth
            .create_session(
                &gm_user,
                &sid,
                "room-pass",
                SessionMode::Ttrpg,
                SessionRole::Gm,
                Some("Sean (GM)".to_string()),
                true,
            )
            .await
            .unwrap();
        state
            .auth
            .join_session(
                &p1_user,
                &sid,
                "room-pass",
                SessionRole::Player,
                Some("Sara".to_string()),
            )
            .await
            .unwrap();

        (temp_dir, state, app, gm, p1)
    }

    #[tokio::test]
    async fn test_world_ledger_endpoints_and_gm_secret_redaction() {
        let (temp_dir, _state, app, gm, p1) = build_test_app_and_users("world_ledger").await;
        let sid = "sess-world_ledger";

        let p_get = axum::http::Request::builder()
            .method("GET")
            .uri(format!("/api/sessions/{sid}/world-ledger"))
            .header("Authorization", format!("Bearer {}", p1.auth_token))
            .body(axum::body::Body::empty())
            .unwrap();
        let p_resp = tower::ServiceExt::oneshot(app.clone(), p_get)
            .await
            .unwrap();
        assert_eq!(p_resp.status(), StatusCode::OK);
        let p_bytes = axum::body::to_bytes(p_resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let p_ledger: SessionWorldLedger = serde_json::from_slice(&p_bytes).unwrap();
        assert!(!p_ledger.faction_clocks.is_empty());
        assert!(
            p_ledger
                .faction_clocks
                .iter()
                .all(|c| c.gm_secret_agenda.is_none())
        );

        let p_mut = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/api/sessions/{sid}/world-ledger"))
            .header("Authorization", format!("Bearer {}", p1.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&satya_schema::UpdateWorldLedgerRequest {
                    heat_delta: Some(2),
                    ..Default::default()
                })
                .unwrap(),
            ))
            .unwrap();
        let p_mut_resp = tower::ServiceExt::oneshot(app.clone(), p_mut)
            .await
            .unwrap();
        assert_eq!(p_mut_resp.status(), StatusCode::FORBIDDEN);

        let gm_mut = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/api/sessions/{sid}/world-ledger"))
            .header("Authorization", format!("Bearer {}", gm.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&satya_schema::UpdateWorldLedgerRequest {
                    faction_id: Some("daise".to_string()),
                    clock_delta: Some(2),
                    incident_severity: Some("loud_firefight".to_string()),
                    reason: Some("Clinic perimeter shootout".to_string()),
                    ..Default::default()
                })
                .unwrap(),
            ))
            .unwrap();
        let gm_resp = tower::ServiceExt::oneshot(app.clone(), gm_mut)
            .await
            .unwrap();
        assert_eq!(gm_resp.status(), StatusCode::OK);
        let gm_bytes = axum::body::to_bytes(gm_resp.into_body(), 64 * 1024)
            .await
            .unwrap();
        let gm_updated: WorldLedgerUpdateResponse = serde_json::from_slice(&gm_bytes).unwrap();
        assert!(gm_updated.ledger.operational_heat >= 2);
        assert!(
            gm_updated
                .ledger
                .faction_clocks
                .iter()
                .any(|c| c.faction_id == "daise" && c.gm_secret_agenda.is_some())
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_player_ai_spatial_move_lookup_and_gm_secret_isolation() {
        let (temp_dir, state, app, gm, _p1) = build_test_app_and_users("ai_spatial").await;
        let sid = "sess-ai_spatial";
        let gm_svc_tok = state.storage.config().ai_gm.service_token.clone();

        // Add GM_ONLY document, secret NPC, and a GM_ONLY map token
        state
            .auth
            .add_session_gm_document(
                sid,
                &gm.user_id,
                GmDocumentRecord {
                    doc_id: "doc-hidden".to_string(),
                    session_id: sid.to_string(),
                    uploader_user_id: gm.user_id.clone(),
                    uploader_display_name: "Sean".to_string(),
                    title: "Hidden Vault Plot".to_string(),
                    source_kind: GmDocumentSource::DirectTextUpload,
                    google_file_id: None,
                    visibility: MessageVisibility::GmOnly,
                    sanitized_content: "TOP_SECRET_GM_DOC_VAULT_404".to_string(),
                    char_count: 27,
                    safety_flagged: false,
                    is_persistent: true,
                    campaign_tag: Some(sid.to_string()),
                    created_at_us: 1000,
                },
            )
            .await
            .unwrap();

        let npc_req = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/api/sessions/{sid}/npcs/generate"))
            .header("Authorization", format!("Bearer {}", gm.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&json!({
                    "prompt": "Dr. Terry is a compassionate clinic trauma physician.",
                    "name_override": "Dr. Terry",
                    "template": "MEDICAL_DOCTOR",
                    "gm_secret_notes": "SECRET_DOSSIER_SYNDICATE_KEY_999"
                }))
                .unwrap(),
            ))
            .unwrap();
        let npc_resp = tower::ServiceExt::oneshot(app.clone(), npc_req)
            .await
            .unwrap();
        assert_eq!(npc_resp.status(), StatusCode::OK);

        let spawn_req = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/api/sessions/{sid}/ai-players"))
            .header("Authorization", format!("Bearer {}", gm.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&AiPlayerConfig {
                    user_id: Some("ai-player-kaelen".to_string()),
                    display_name: Some("Kaelen Voss (AI Player)".to_string()),
                    character_id: None,
                    persona_prompt: Some("Field medic and investigator".to_string()),
                    provider: Some("ollama".to_string()),
                    model_id: Some("gemma4:12b".to_string()),
                    service_url: None,
                    auto_respond: true,
                    max_rally_turns: 2,
                })
                .unwrap(),
            ))
            .unwrap();
        let spawn_resp = tower::ServiceExt::oneshot(app.clone(), spawn_req)
            .await
            .unwrap();
        assert_eq!(spawn_resp.status(), StatusCode::OK);

        // Seed map with open floor + 1 public self token + 1 GM_ONLY sniper token
        let mut map = GmMapUpdateRequest {
            map_id: "map-spatial-test".to_string(),
            title: "Clinic Courtyard".to_string(),
            image_url: "data:image/svg+xml;utf8,<svg/>".to_string(),
            grid_width: 10,
            grid_height: 10,
            tiles: vec![TacticalTileKind::Floor; 100],
            tokens: vec![
                TokenPlacement {
                    entity_id: "ai-player-kaelen".to_string(),
                    label: "Kaelen Voss".to_string(),
                    x: 1,
                    y: 1,
                    visibility: MessageVisibility::Public,
                },
                TokenPlacement {
                    entity_id: "gm-ambush-1".to_string(),
                    label: "SECRET_SNIPER_TOKEN".to_string(),
                    x: 8,
                    y: 8,
                    visibility: MessageVisibility::GmOnly,
                },
            ],
            ..Default::default()
        };
        ensure_vector_map_ready(&mut map);
        state.auth.set_session_active_map(sid, map).await.unwrap();

        // Context endpoint hides GM_ONLY token, GM doc, NPC secret dossier, and faction gm_secret_agenda
        let ctx_req = axum::http::Request::builder()
            .method("GET")
            .uri(format!(
                "/api/player-ai/sessions/{sid}/context?user_id=ai-player-kaelen"
            ))
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .body(axum::body::Body::empty())
            .unwrap();
        let ctx_resp = tower::ServiceExt::oneshot(app.clone(), ctx_req)
            .await
            .unwrap();
        assert_eq!(ctx_resp.status(), StatusCode::OK);
        let ctx_bytes = axum::body::to_bytes(ctx_resp.into_body(), 128 * 1024)
            .await
            .unwrap();
        let ctx_str = String::from_utf8_lossy(&ctx_bytes);
        assert!(!ctx_str.contains("SECRET_SNIPER_TOKEN"));
        assert!(!ctx_str.contains("TOP_SECRET_GM_DOC_VAULT_404"));
        assert!(!ctx_str.contains("SECRET_DOSSIER_SYNDICATE_KEY_999"));
        assert!(!ctx_str.contains("gm_secret_agenda"));
        assert!(ctx_str.contains("Dr. Terry"));
        assert!(ctx_str.contains("public_npcs"));
        assert!(ctx_str.contains("world_ledger"));

        // Lookup endpoint returns Dr. Terry public summary & operational heat, zero GM secrets
        let lookup_req = axum::http::Request::builder()
            .method("GET")
            .uri(format!(
                "/api/player-ai/sessions/{sid}/lookup?user_id=ai-player-kaelen&q=Terry&scope=ALL"
            ))
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .body(axum::body::Body::empty())
            .unwrap();
        let lookup_resp = tower::ServiceExt::oneshot(app.clone(), lookup_req)
            .await
            .unwrap();
        assert_eq!(lookup_resp.status(), StatusCode::OK);
        let lookup_bytes = axum::body::to_bytes(lookup_resp.into_body(), 128 * 1024)
            .await
            .unwrap();
        let lookup_str = String::from_utf8_lossy(&lookup_bytes);
        assert!(lookup_str.contains("Dr. Terry"));
        assert!(!lookup_str.contains("SECRET_DOSSIER_SYNDICATE_KEY_999"));
        assert!(!lookup_str.contains("TOP_SECRET_GM_DOC_VAULT_404"));

        // Self-token move succeeds (200 OK); moving another entity returns 403 Forbidden
        let move_ok = axum::http::Request::builder()
            .method("POST")
            .uri(format!(
                "/api/player-ai/sessions/{sid}/map/move?user_id=ai-player-kaelen"
            ))
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&json!({
                    "entity_id": "ai-player-kaelen",
                    "to_x": 4,
                    "to_y": 3
                }))
                .unwrap(),
            ))
            .unwrap();
        let move_resp = tower::ServiceExt::oneshot(app.clone(), move_ok)
            .await
            .unwrap();
        assert_eq!(move_resp.status(), StatusCode::OK);
        let move_bytes = axum::body::to_bytes(move_resp.into_body(), 128 * 1024)
            .await
            .unwrap();
        let filtered_map: GmMapUpdateRequest = serde_json::from_slice(&move_bytes).unwrap();
        let moved_tok = filtered_map
            .tokens
            .iter()
            .find(|t| t.entity_id == "ai-player-kaelen")
            .unwrap();
        assert_eq!((moved_tok.x, moved_tok.y), (4, 3));
        assert!(
            !filtered_map
                .tokens
                .iter()
                .any(|t| t.entity_id == "gm-ambush-1")
        );

        let move_forbidden = axum::http::Request::builder()
            .method("POST")
            .uri(format!(
                "/api/player-ai/sessions/{sid}/map/move?user_id=ai-player-kaelen"
            ))
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&json!({
                    "entity_id": "gm-ambush-1",
                    "to_x": 2,
                    "to_y": 2
                }))
                .unwrap(),
            ))
            .unwrap();
        let forbidden_resp = tower::ServiceExt::oneshot(app.clone(), move_forbidden)
            .await
            .unwrap();
        assert_eq!(forbidden_resp.status(), StatusCode::FORBIDDEN);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
