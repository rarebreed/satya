//! Integration tests for `satya-signaling` AI GM fail-fast notice, AI Player context
//! isolation & rally budget, and continuous Tactical Map authoring / movement / visibility filtering.

use super::*;
use satya_schema::{
    ChannelContext, GmMapUpdateRequest, MessageVisibility, Participant, SessionMode, SessionRole,
};
use satya_storage::StorageConfig;
use uuid::Uuid;

#[tokio::test]
async fn test_ai_gm_fail_fast_when_offline_emits_system_log_and_notice() {
    let mut cfg = StorageConfig::default();
    let temp_dir = std::env::temp_dir().join(format!("satya_sig_failfast_{}", Uuid::now_v7()));
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir
        .join("users_failfast.db")
        .to_string_lossy()
        .to_string();
    // Enable fail-fast behavior and point to an unreachable local port
    cfg.ai_gm.fail_fast_when_offline = true;
    cfg.ai_gm.service_url = "http://127.0.0.1:59999".to_string();

    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let gm = Participant::human("u-gm-ff", "gm_sean", "Sean (GM)", SessionRole::Gm);
    let ai_gm = Participant::ai_gm("ep-failfast", "ollama/qwen2.5:14b", "harsh-reality-arbiter");
    let roster = vec![gm.clone(), ai_gm.clone()];

    trigger_ai_gm_chat_reply(
        &state,
        "ep-failfast",
        SessionMode::Ttrpg,
        &ai_gm,
        &gm,
        "ev-trigger-1",
        "CHAT",
        MessageVisibility::GmOnly,
        Some(ChannelContext::InCharacter),
        "Check Captain Vance's leadership TN",
        &roster,
    )
    .await;

    let metrics = state.storage.metrics_snapshot().await;
    let sys_logs = metrics.iter().find(|m| m.topic == "system-logs").unwrap();
    assert_eq!(sys_logs.total_messages_ingested, 1);

    let chat_logs = metrics.iter().find(|m| m.topic == "chat-messages").unwrap();
    assert_eq!(chat_logs.total_messages_ingested, 1);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_ai_player_spawn_step_context_isolation_and_rally_budget() {
    let mut cfg = StorageConfig::default();
    let temp_dir = std::env::temp_dir().join(format!("satya_sig_ai_player_{}", Uuid::now_v7()));
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir
        .join("users_ai_player.db")
        .to_string_lossy()
        .to_string();

    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    let p1 = state
        .auth
        .authenticate_user("sara_valkyrie", "satya-truth-2026")
        .await
        .unwrap();
    let p1_user = state.auth.verify_auth_token(&p1.auth_token).await.unwrap();
    let _enter = state
        .auth
        .create_session(
            &p1_user,
            "ep-ai-player-1",
            "room-pass",
            SessionMode::Ttrpg,
            SessionRole::Player,
            Some("Sara (Valkyrie)".to_string()),
            true,
        )
        .await
        .unwrap();

    // Submit a GM_ONLY document that must NEVER leak into the AI Player's context
    let _ = state
        .auth
        .add_session_gm_document(
            "ep-ai-player-1",
            &p1.user_id,
            satya_schema::GmDocumentRecord {
                doc_id: "doc-secret-1".to_string(),
                session_id: "ep-ai-player-1".to_string(),
                uploader_user_id: p1.user_id.clone(),
                uploader_display_name: "Sara".to_string(),
                title: "Secret Plot".to_string(),
                source_kind: satya_schema::GmDocumentSource::DirectTextUpload,
                google_file_id: None,
                visibility: MessageVisibility::GmOnly,
                sanitized_content: "TOP_SECRET_VANCE_CODE_991".to_string(),
                char_count: 25,
                safety_flagged: false,
                is_persistent: false,
                campaign_tag: Some("ep-ai-player-1".to_string()),
                created_at_us: 1000,
            },
        )
        .await
        .unwrap();

    // 1. Spawn an AI Player via POST /api/sessions/:id/ai-players
    let spawn_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-ai-player-1/ai-players")
        .header("Authorization", format!("Bearer {}", p1.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::AiPlayerConfig {
                user_id: Some("ai-player-kaelen".to_string()),
                display_name: Some("Kaelen Voss (AI Player)".to_string()),
                character_id: None,
                persona_prompt: Some("Recon scout".to_string()),
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
    assert_eq!(spawn_resp.status(), axum::http::StatusCode::OK);

    // Verify the AI Player is in the session roster with SessionRole::Player and ParticipantKind::AiAgent
    let roster = state.auth.get_session_roster("ep-ai-player-1").await;
    assert!(roster.iter().any(|p| {
        p.user_id == "ai-player-kaelen"
            && p.role == SessionRole::Player
            && p.participant_kind == satya_schema::ParticipantKind::AiAgent
    }));

    // 2. Query GET /api/player-ai/sessions/:id/context and verify strict GM_ONLY isolation
    let ctx_req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/player-ai/sessions/ep-ai-player-1/context?user_id=ai-player-kaelen")
        .header(
            "Authorization",
            format!("Bearer {}", state.storage.config().ai_gm.service_token),
        )
        .body(axum::body::Body::empty())
        .unwrap();
    let ctx_resp = tower::ServiceExt::oneshot(app.clone(), ctx_req)
        .await
        .unwrap();
    assert_eq!(ctx_resp.status(), axum::http::StatusCode::OK);
    let ctx_bytes = axum::body::to_bytes(ctx_resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    let ctx_str = String::from_utf8_lossy(&ctx_bytes);
    assert!(!ctx_str.contains("TOP_SECRET_VANCE_CODE_991"));
    assert!(!ctx_str.contains("gm_documents"));

    // 3. Step the AI Player turn and verify rally counter & chat ingestion
    let step_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-ai-player-1/ai-players/step")
        .header("Authorization", format!("Bearer {}", p1.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::StepAiPlayerRequest {
                user_id: Some("ai-player-kaelen".to_string()),
                prompt: Some("Advance to the airlock".to_string()),
                auto_trigger_gm: true,
            })
            .unwrap(),
        ))
        .unwrap();
    let step_resp = tower::ServiceExt::oneshot(app.clone(), step_req)
        .await
        .unwrap();
    assert_eq!(step_resp.status(), axum::http::StatusCode::OK);

    // Verify rally budget cap enforcement (`max_rally_turns = 2`: 1st already consumed by step)
    assert!(state.try_consume_ai_rally_turn("ep-ai-player-1", 2).await);
    assert!(!state.try_consume_ai_rally_turn("ep-ai-player-1", 2).await);
    state.reset_ai_rally_counter("ep-ai-player-1").await;
    assert!(state.try_consume_ai_rally_turn("ep-ai-player-1", 2).await);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_tactical_map_generation_token_move_wall_collision_and_gm_only_filtering() {
    let mut cfg = StorageConfig::default();
    let temp_dir = std::env::temp_dir().join(format!("satya_sig_map3d_{}", Uuid::now_v7()));
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir
        .join("users_map3d.db")
        .to_string_lossy()
        .to_string();

    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    let gm_auth = state
        .auth
        .authenticate_user("gm_sean", "satya-truth-2026")
        .await
        .unwrap();
    let gm_user = state
        .auth
        .verify_auth_token(&gm_auth.auth_token)
        .await
        .unwrap();
    let _ = state
        .auth
        .create_session(
            &gm_user,
            "ep-map-3d",
            "room-pass",
            SessionMode::Ttrpg,
            SessionRole::Gm,
            Some("Sean (GM)".to_string()),
            false,
        )
        .await
        .unwrap();

    let p1_auth = state
        .auth
        .authenticate_user("sara_valkyrie", "satya-truth-2026")
        .await
        .unwrap();
    let p1_user = state
        .auth
        .verify_auth_token(&p1_auth.auth_token)
        .await
        .unwrap();
    let _ = state
        .auth
        .join_session(
            &p1_user,
            "ep-map-3d",
            "room-pass",
            SessionRole::Player,
            Some("Sara (Valkyrie)".to_string()),
        )
        .await
        .unwrap();

    // 1. GM generates a 20x14 1.0m procedural Docking Bay sector map
    let gen_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                preset: "docking_bay".to_string(),
                title: Some("Aethelgard Bay 7".to_string()),
                grid_width: 20,
                grid_height: 14,
                cell_size_cm: 100,
                seed: Some(77),
                populate_roster_tokens: true,
                edit_cell_x: None,
                edit_cell_y: None,
                set_tile_kind: None,
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let gen_resp = tower::ServiceExt::oneshot(app.clone(), gen_req)
        .await
        .unwrap();
    assert_eq!(gen_resp.status(), axum::http::StatusCode::OK);
    let gen_bytes = axum::body::to_bytes(gen_resp.into_body(), 128 * 1024)
        .await
        .unwrap();
    let generated_map: GmMapUpdateRequest = serde_json::from_slice(&gen_bytes).unwrap();
    assert_eq!(generated_map.grid_width, 20);
    assert_eq!(generated_map.grid_height, 14);
    assert_eq!(generated_map.cell_size_cm, 100);
    assert!(generated_map.tiles.is_empty(), "Procedural vector maps emit empty legacy tiles");
    assert!(!generated_map.wall_edges.is_empty());
    assert!(!generated_map.props.is_empty());
    assert!(
        generated_map
            .tokens
            .iter()
            .any(|t| t.entity_id == p1_auth.user_id)
    );

    // 2. Player attempting to move out-of-bounds OR deep into a solid obstacle gets 400 BAD_REQUEST
    let obstacle_cm = generated_map
        .props
        .iter()
        .find(|p| {
            satya_engine::map_dressing::prop_spec(p.kind).obstruction
                == satya_engine::map_dressing::PropObstruction::FullObstacle
        })
        .map(|p| p.center_cm())
        .unwrap_or((-50, -50));
    let wall_move_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: 0,
                to_y: 0,
                to_x_cm: Some(obstacle_cm.0),
                to_y_cm: Some(obstacle_cm.1),
                facing_deg: None,
                model_preset: None,
                color_hex: None,
                stance_pose: None,
                portrait_url: None,
                visibility: None,
                remove: false,
            })
            .unwrap(),
        ))
        .unwrap();
    let wall_resp = tower::ServiceExt::oneshot(app.clone(), wall_move_req)
        .await
        .unwrap();
    assert_eq!(wall_resp.status(), axum::http::StatusCode::BAD_REQUEST);

    // 3. Player attempting to move the SecOps NPC while a Human GM leads gets 403 FORBIDDEN
    let npc_move_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: "npc-secops-1".to_string(),
                label: None,
                to_x: 4,
                to_y: 7,
                to_x_cm: None,
                to_y_cm: None,
                facing_deg: None,
                model_preset: None,
                color_hex: None,
                stance_pose: None,
                portrait_url: None,
                visibility: None,
                remove: false,
            })
            .unwrap(),
        ))
        .unwrap();
    let npc_resp = tower::ServiceExt::oneshot(app.clone(), npc_move_req)
        .await
        .unwrap();
    assert_eq!(npc_resp.status(), axum::http::StatusCode::FORBIDDEN);

    // 4. Player moves their own miniature to a continuous unblocked sub-meter spot -> 200 OK
    let (board_w_m, board_h_m) = generated_map.board_size_m();
    let (free_mv_x_cm, free_mv_y_cm) = (1..generated_map.grid_height - 1)
        .flat_map(|yy| (1..generated_map.grid_width - 1).map(move |xx| (xx * 100 + 42, yy * 100 + 28)))
        .find(|&(x_cm, y_cm)| {
            !satya_engine::tactical_map_geom::circle_collides_obstacles(
                board_w_m,
                board_h_m,
                &generated_map.wall_edges,
                &generated_map.props,
                x_cm as f32 / 100.0,
                y_cm as f32 / 100.0,
                satya_engine::tactical_map_geom::TOKEN_RADIUS_M,
            )
        })
        .expect("unblocked sub-meter spot");
    let ok_move_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: free_mv_x_cm / 100,
                to_y: free_mv_y_cm / 100,
                to_x_cm: Some(free_mv_x_cm),
                to_y_cm: Some(free_mv_y_cm),
                facing_deg: None,
                model_preset: Some("valkyrie_rig".to_string()),
                color_hex: Some("#38bdf8".to_string()),
                stance_pose: None,
                portrait_url: None,
                visibility: None,
                remove: false,
            })
            .unwrap(),
        ))
        .unwrap();
    let ok_resp = tower::ServiceExt::oneshot(app.clone(), ok_move_req)
        .await
        .unwrap();
    assert_eq!(ok_resp.status(), axum::http::StatusCode::OK);
    let ok_map: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(ok_resp.into_body(), 128 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        ok_map
            .visual_for(&p1_auth.user_id)
            .and_then(|v| v.pos_x_cm),
        Some(free_mv_x_cm)
    );
    assert_eq!(
        ok_map
            .visual_for(&p1_auth.user_id)
            .and_then(|v| v.pos_y_cm),
        Some(free_mv_y_cm)
    );

    // 5. GM spawns a hidden `GM_ONLY` ambush sentry; GM sees it on GET, Player does NOT see it!
    let ambush_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map/move")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: "npc-ambush-stalker".to_string(),
                label: Some("Cloaked Stalker".to_string()),
                to_x: 15,
                to_y: 7,
                to_x_cm: None,
                to_y_cm: None,
                facing_deg: Some(270),
                model_preset: Some("secops_sentry".to_string()),
                color_hex: Some("#f43f5e".to_string()),
                stance_pose: None,
                portrait_url: None,
                visibility: Some(MessageVisibility::GmOnly),
                remove: false,
            })
            .unwrap(),
        ))
        .unwrap();
    let ambush_resp = tower::ServiceExt::oneshot(app.clone(), ambush_req)
        .await
        .unwrap();
    assert_eq!(ambush_resp.status(), axum::http::StatusCode::OK);

    let gm_get_req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/sessions/ep-map-3d/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .body(axum::body::Body::empty())
        .unwrap();
    let gm_get_resp = tower::ServiceExt::oneshot(app.clone(), gm_get_req)
        .await
        .unwrap();
    let gm_map_bytes = axum::body::to_bytes(gm_get_resp.into_body(), 128 * 1024)
        .await
        .unwrap();
    let gm_map: GmMapUpdateRequest = serde_json::from_slice(&gm_map_bytes).unwrap();
    assert!(
        gm_map
            .tokens
            .iter()
            .any(|t| t.entity_id == "npc-ambush-stalker")
    );

    let p1_get_req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/sessions/ep-map-3d/map")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .body(axum::body::Body::empty())
        .unwrap();
    let p1_get_resp = tower::ServiceExt::oneshot(app.clone(), p1_get_req)
        .await
        .unwrap();
    let p1_map_bytes = axum::body::to_bytes(p1_get_resp.into_body(), 128 * 1024)
        .await
        .unwrap();
    let p1_map: GmMapUpdateRequest = serde_json::from_slice(&p1_map_bytes).unwrap();
    assert!(
        !p1_map
            .tokens
            .iter()
            .any(|t| t.entity_id == "npc-ambush-stalker"),
        "GM_ONLY ambush token must be stripped from non-GM Player map view"
    );

    // 6. Phase 4: GM 360-deg continuous prop placement & removal + portrait URL sanitization
    let occupied_tokens_m: Vec<(f32, f32)> =
        p1_map.tokens.iter().map(|t| p1_map.token_pos_m(t)).collect();
    let sofa = (2..p1_map.grid_height - 2)
        .flat_map(|yy| (2..p1_map.grid_width - 2).map(move |xx| (xx * 100 + 50, yy * 100 + 50)))
        .find_map(|(cx_cm, cy_cm)| {
            let candidate = satya_engine::map_dressing::make_prop_at_cm(
                satya_schema::PropKind::Sofa,
                cx_cm,
                cy_cm,
                33,
                None,
                12,
            );
            let mut probe = p1_map.props.clone();
            satya_engine::tactical_map_geom::place_prop_on_map(
                &mut probe,
                &p1_map.wall_edges,
                &occupied_tokens_m,
                (board_w_m, board_h_m),
                candidate,
            )
            .ok()
        })
        .expect("interior clear spot for 33-deg Sofa");
    let sofa_cm = sofa.center_cm();
    let p_prop_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                edit_prop: Some(sofa),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let p_prop_resp = tower::ServiceExt::oneshot(app.clone(), p_prop_req)
        .await
        .unwrap();
    assert_eq!(p_prop_resp.status(), axum::http::StatusCode::FORBIDDEN);

    let gm_prop_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                edit_prop: Some(sofa),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let gm_prop_resp = tower::ServiceExt::oneshot(app.clone(), gm_prop_req)
        .await
        .unwrap();
    assert_eq!(gm_prop_resp.status(), axum::http::StatusCode::OK);
    let prop_bytes = axum::body::to_bytes(gm_prop_resp.into_body(), 128 * 1024)
        .await
        .unwrap();
    let prop_map: GmMapUpdateRequest = serde_json::from_slice(&prop_bytes).unwrap();
    assert!(
        prop_map
            .props
            .iter()
            .any(|p| p.kind == satya_schema::PropKind::Sofa && p.rotation_deg == 33 && p.center_cm() == sofa_cm)
    );

    let gm_clear_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                clear_prop_at_cm: Some(sofa_cm),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let gm_clear_resp = tower::ServiceExt::oneshot(app.clone(), gm_clear_req)
        .await
        .unwrap();
    assert_eq!(gm_clear_resp.status(), axum::http::StatusCode::OK);
    let cleared_bytes = axum::body::to_bytes(gm_clear_resp.into_body(), 128 * 1024)
        .await
        .unwrap();
    let cleared_map: GmMapUpdateRequest = serde_json::from_slice(&cleared_bytes).unwrap();
    assert!(
        !cleared_map
            .props
            .iter()
            .any(|p| p.kind == satya_schema::PropKind::Sofa && p.center_cm() == sofa_cm)
    );

    // Portrait URL sanitization on token move
    let bad_url_move = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: free_mv_x_cm / 100,
                to_y: free_mv_y_cm / 100,
                to_x_cm: Some(free_mv_x_cm),
                to_y_cm: Some(free_mv_y_cm),
                facing_deg: None,
                model_preset: None,
                color_hex: None,
                stance_pose: None,
                portrait_url: Some("javascript:alert(1)".to_string()),
                visibility: None,
                remove: false,
            })
            .unwrap(),
        ))
        .unwrap();
    let bad_url_resp = tower::ServiceExt::oneshot(app.clone(), bad_url_move)
        .await
        .unwrap();
    assert_eq!(bad_url_resp.status(), axum::http::StatusCode::OK);
    let bad_url_map: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(bad_url_resp.into_body(), 128 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bad_url_map
            .visual_for(&p1_auth.user_id)
            .and_then(|v| v.portrait_url.as_deref()),
        None
    );

    let ok_url_move = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: free_mv_x_cm / 100,
                to_y: free_mv_y_cm / 100,
                to_x_cm: Some(free_mv_x_cm),
                to_y_cm: Some(free_mv_y_cm),
                facing_deg: None,
                model_preset: None,
                color_hex: None,
                stance_pose: None,
                portrait_url: Some("/api/characters/char-1/assets/asset-1".to_string()),
                visibility: None,
                remove: false,
            })
            .unwrap(),
        ))
        .unwrap();
    let ok_url_resp = tower::ServiceExt::oneshot(app.clone(), ok_url_move)
        .await
        .unwrap();
    assert_eq!(ok_url_resp.status(), axum::http::StatusCode::OK);
    let ok_url_map: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(ok_url_resp.into_body(), 128 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        ok_url_map
            .visual_for(&p1_auth.user_id)
            .and_then(|v| v.portrait_url.as_deref()),
        Some("/api/characters/char-1/assets/asset-1")
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
