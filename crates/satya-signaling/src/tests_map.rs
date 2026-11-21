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
    cfg.ai_gm.service_url = "http://127.0.0.1:59999".to_string();

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
    assert!(
        generated_map.tiles.is_empty(),
        "Procedural vector maps emit empty legacy tiles"
    );
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
            satya_maps::map_dressing::prop_spec(p.kind).obstruction
                == satya_maps::map_dressing::PropObstruction::FullObstacle
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
                to_floor: None,
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
                to_floor: None,
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
        .flat_map(|yy| {
            (1..generated_map.grid_width - 1).map(move |xx| (xx * 100 + 42, yy * 100 + 28))
        })
        .find(|&(x_cm, y_cm)| {
            !satya_maps::tactical_map_geom::circle_collides_obstacles(
                board_w_m,
                board_h_m,
                &generated_map.wall_edges,
                &generated_map.props,
                x_cm as f32 / 100.0,
                y_cm as f32 / 100.0,
                satya_maps::tactical_map_geom::TOKEN_RADIUS_M,
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
                to_floor: None,
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
        ok_map.visual_for(&p1_auth.user_id).and_then(|v| v.pos_x_cm),
        Some(free_mv_x_cm)
    );
    assert_eq!(
        ok_map.visual_for(&p1_auth.user_id).and_then(|v| v.pos_y_cm),
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
                to_floor: None,
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
    let occupied_tokens_m: Vec<(f32, f32)> = p1_map
        .tokens
        .iter()
        .map(|t| p1_map.token_pos_m(t))
        .collect();
    let sofa = (2..p1_map.grid_height - 2)
        .flat_map(|yy| (2..p1_map.grid_width - 2).map(move |xx| (xx * 100 + 50, yy * 100 + 50)))
        .find_map(|(cx_cm, cy_cm)| {
            let candidate = satya_maps::map_dressing::make_prop_at_cm(
                satya_schema::PropKind::Sofa,
                cx_cm,
                cy_cm,
                33,
                None,
                12,
            );
            let mut probe = p1_map.props.clone();
            satya_maps::tactical_map_geom::place_prop_on_map(
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
            .any(|p| p.kind == satya_schema::PropKind::Sofa
                && p.rotation_deg == 33
                && p.center_cm() == sofa_cm)
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
                to_floor: None,
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
                to_floor: None,
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

    // 7. Editor authoring operations: prop move/rotate/tint/decorative, batch edges,
    // surface add/remove, ground style overrides, replace_floor_geometry, and player 403 gate.
    let add_bed_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                edit_prop: Some(satya_maps::map_dressing::make_prop_at_cm(
                    satya_schema::PropKind::HospitalBed,
                    850,
                    550,
                    12,
                    None,
                    9,
                )),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let add_bed_resp = tower::ServiceExt::oneshot(app.clone(), add_bed_req)
        .await
        .unwrap();
    assert_eq!(add_bed_resp.status(), axum::http::StatusCode::OK);
    let bed_map: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(add_bed_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    let bed_idx = bed_map
        .props
        .iter()
        .rposition(|p| p.kind == satya_schema::PropKind::HospitalBed && p.center_cm() == (850, 550))
        .expect("HospitalBed added");

    let mut updated_bed = bed_map.props[bed_idx];
    updated_bed.rotation_deg = 45;
    updated_bed.tint_rgb = Some([0x38, 0xbd, 0xf8]);
    updated_bed.decorative = true;
    let patch_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                update_prop_at_index: Some((bed_idx, updated_bed)),
                set_ground_floor_override: Some("clinical_vinyl".to_string()),
                set_ground_tint_hex: Some("#d8e6ec".to_string()),
                add_surface: Some(satya_schema::MapSurfaceFeature {
                    kind: satya_schema::SurfaceFeatureKind::TerrazzoFloor,
                    shape: satya_schema::SurfaceShapeKind::Polygon,
                    points_cm: vec![
                        satya_schema::MapPointCm::new(400, 400),
                        satya_schema::MapPointCm::new(700, 400),
                        satya_schema::MapPointCm::new(700, 700),
                        satya_schema::MapPointCm::new(400, 700),
                    ],
                    width_cm: 0,
                    border_style_cm: 0,
                    variant_seed: 3,
                    tint_rgb: Some([0xe0, 0xf2, 0xfe]),
                }),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let patch_resp = tower::ServiceExt::oneshot(app.clone(), patch_req)
        .await
        .unwrap();
    assert_eq!(patch_resp.status(), axum::http::StatusCode::OK);
    let patched: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(patch_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(patched.props[bed_idx].rotation_deg, 45);
    assert_eq!(patched.props[bed_idx].tint_rgb, Some([0x38, 0xbd, 0xf8]));
    assert!(patched.props[bed_idx].decorative);
    assert_eq!(
        patched.ground_floor_override.as_deref(),
        Some("clinical_vinyl")
    );
    assert_eq!(patched.ground_tint_rgb, Some([0xd8, 0xe6, 0xec]));
    assert!(
        patched
            .surfaces
            .iter()
            .any(|s| s.kind == satya_schema::SurfaceFeatureKind::TerrazzoFloor)
    );

    // Non-GM player cannot overwrite floor geometry or tint ground.
    let p1_editor_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                set_ground_floor_override: Some("terrazzo_stone".to_string()),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let p1_editor_resp = tower::ServiceExt::oneshot(app.clone(), p1_editor_req)
        .await
        .unwrap();
    assert_eq!(p1_editor_resp.status(), axum::http::StatusCode::FORBIDDEN);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_observer_ai_gm_session_isolation_escalation_blocks_and_autonomous_rally() {
    let mut cfg = StorageConfig::default();
    let temp_dir = std::env::temp_dir().join(format!("satya_sig_obs_wave1_{}", Uuid::now_v7()));
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir
        .join("users_obs_wave1.db")
        .to_string_lossy()
        .to_string();
    cfg.ai_gm.service_url = "http://127.0.0.1:59999".to_string();

    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());
    let gm_svc_tok = state.storage.config().ai_gm.service_token.clone();

    let obs_auth = state
        .auth
        .register_user("obs_wave1", "satya-truth-2026", "Observer Wave1")
        .await
        .unwrap();
    let obs_user = state
        .auth
        .verify_auth_token(&obs_auth.auth_token)
        .await
        .unwrap();
    let p2_auth = state
        .auth
        .register_user("p2_wave1", "satya-truth-2026", "Player2 Wave1")
        .await
        .unwrap();
    let p2_user = state
        .auth
        .verify_auth_token(&p2_auth.auth_token)
        .await
        .unwrap();

    // 1. Observer creation requires enable_ai_gm == true
    assert!(
        state
            .auth
            .create_session(
                &obs_user,
                "ep-obs-wave1",
                "room-pass",
                SessionMode::Ttrpg,
                SessionRole::Observer,
                Some("Observer One".to_string()),
                false,
            )
            .await
            .is_err()
    );
    state
        .auth
        .create_session(
            &obs_user,
            "ep-obs-wave1",
            "room-pass",
            SessionMode::Ttrpg,
            SessionRole::Observer,
            Some("Observer One".to_string()),
            true,
        )
        .await
        .unwrap();

    // 2. join_session blocks SessionRole::AiGm; resume_session blocks escalation to Gm/Host/AiGm
    assert!(
        state
            .auth
            .join_session(
                &p2_user,
                "ep-obs-wave1",
                "room-pass",
                SessionRole::AiGm,
                None,
            )
            .await
            .is_err()
    );
    state
        .auth
        .join_session(
            &p2_user,
            "ep-obs-wave1",
            "room-pass",
            SessionRole::Player,
            Some("P2".to_string()),
        )
        .await
        .unwrap();
    assert!(
        state
            .auth
            .leave_session("ep-obs-wave1", &p2_auth.user_id)
            .await
            .is_some()
    );
    for bad_role in [SessionRole::Gm, SessionRole::Host, SessionRole::AiGm] {
        assert!(
            state
                .auth
                .resume_session(
                    &p2_user,
                    &satya_schema::ResumeSessionRequest {
                        session_id: "ep-obs-wave1".to_string(),
                        session_password: Some("room-pass".to_string()),
                        join_token: None,
                        role: Some(bad_role),
                        display_name: None,
                    },
                )
                .await
                .is_err(),
            "Non-host Player must not escalate to {bad_role:?} on resume"
        );
    }

    // 3. Observer can sponsor/orchestrate AI GM BYOK vault while remaining barred from GM secrets
    let cfg_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/sessions/ep-obs-wave1/gm-config")
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::json!({
                    "provider": "ollama",
                    "model_id": "gemma4:12b",
                    "api_key": "sk-obs-sponsor-key"
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(cfg_resp.status(), axum::http::StatusCode::OK);

    // Seed GM_ONLY document, GM memory, map tokens, and GM_ONLY combatant via GM service
    state
        .auth
        .add_session_gm_document(
            "ep-obs-wave1",
            "ai-gm-ep-obs-wave1",
            satya_schema::GmDocumentRecord {
                doc_id: "doc-obs-secret".to_string(),
                session_id: "ep-obs-wave1".to_string(),
                uploader_user_id: "ai-gm-ep-obs-wave1".to_string(),
                uploader_display_name: "AI GM".to_string(),
                title: "Observer Secret Doc".to_string(),
                source_kind: satya_schema::GmDocumentSource::DirectTextUpload,
                google_file_id: None,
                visibility: MessageVisibility::GmOnly,
                sanitized_content: "OBS_SECRET_DOC_TOKEN_99".to_string(),
                char_count: 23,
                safety_flagged: false,
                is_persistent: false,
                campaign_tag: Some("ep-obs-wave1".to_string()),
                created_at_us: 1000,
            },
        )
        .await
        .unwrap();

    let mem_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/sessions/ep-obs-wave1/memories")
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&satya_schema::UpsertGmMemoryRequest {
                    memory_id: Some("mem-obs-secret".to_string()),
                    campaign_tag: Some("ep-obs-wave1".to_string()),
                    category: satya_schema::GmMemoryCategory::NpcSecret,
                    title: "Secret Ambush".to_string(),
                    content: "OBS_SECRET_MEMORY_99".to_string(),
                    is_pinned: false,
                    is_persistent: false,
                })
                .unwrap(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(mem_resp.status(), axum::http::StatusCode::OK);

    let gm_map_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/gm/sessions/ep-obs-wave1/map")
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&GmMapUpdateRequest {
                    map_id: "map-obs".to_string(),
                    title: "Clinic Lobby".to_string(),
                    image_url: String::new(),
                    grid_width: 12,
                    grid_height: 10,
                    cell_size_cm: 100,
                    tokens: vec![
                        satya_schema::TokenPlacement {
                            entity_id: "tok-pub".to_string(),
                            label: "Public Medic".to_string(),
                            x: 3,
                            y: 3,
                            visibility: MessageVisibility::Public,
                        },
                        satya_schema::TokenPlacement {
                            entity_id: "tok-hidden".to_string(),
                            label: "Hidden Assassin".to_string(),
                            x: 9,
                            y: 8,
                            visibility: MessageVisibility::GmOnly,
                        },
                    ],
                    ..GmMapUpdateRequest::default()
                })
                .unwrap(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(gm_map_resp.status(), axum::http::StatusCode::OK);

    let gm_combat_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/gm/sessions/ep-obs-wave1/combat")
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&satya_schema::UpdateCombatTrackerRequest {
                    combat_active: Some(true),
                    entity_id: Some("tok-hidden".to_string()),
                    ..Default::default()
                })
                .unwrap(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(gm_combat_resp.status(), axum::http::StatusCode::OK);

    // Observer cannot view GM memories or unowned GM docs (redacted empty) and is blocked from GM mutations
    for secret_uri in [
        "/api/sessions/ep-obs-wave1/memories",
        "/api/sessions/ep-obs-wave1/docs",
    ] {
        let resp = tower::ServiceExt::oneshot(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri(secret_uri)
                .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = String::from_utf8_lossy(
            &axum::body::to_bytes(resp.into_body(), 64 * 1024)
                .await
                .unwrap(),
        )
        .to_string();
        assert!(!body.contains("OBS_SECRET_MEMORY_99"));
        assert!(!body.contains("OBS_SECRET_DOC_TOKEN_99"));
    }

    let obs_mem_post = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/sessions/ep-obs-wave1/memories")
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::json!({"category": "SCENE_STATE", "title": "t", "content": "c"})
                    .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert!(obs_mem_post.status().is_client_error());

    for (method, uri, body_opt) in [
        (
            "POST",
            "/api/sessions/ep-obs-wave1/combat",
            Some(serde_json::json!({"tick_delta": 1}).to_string()),
        ),
        (
            "POST",
            "/api/sessions/ep-obs-wave1/npcs/generate",
            Some(serde_json::json!({"prompt": "guard"}).to_string()),
        ),
        (
            "POST",
            "/api/sessions/ep-obs-wave1/map",
            Some(
                serde_json::to_string(&satya_schema::GenerateSessionMapRequest::default()).unwrap(),
            ),
        ),
        (
            "POST",
            "/api/sessions/ep-obs-wave1/map/move",
            Some(serde_json::json!({"entity_id": "tok-pub", "to_x": 4, "to_y": 4}).to_string()),
        ),
    ] {
        let mut builder = axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token));
        let req = if let Some(b) = body_opt {
            builder = builder.header("Content-Type", "application/json");
            builder.body(axum::body::Body::from(b)).unwrap()
        } else {
            builder.body(axum::body::Body::empty()).unwrap()
        };
        let resp = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
        assert_eq!(
            resp.status(),
            axum::http::StatusCode::FORBIDDEN,
            "Observer must be rejected with 403 on {method} {uri}"
        );
    }

    // Observer receives redacted combat tracker, map, export, and strictly PUBLIC omnisearch lore
    let obs_combat_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("GET")
            .uri("/api/sessions/ep-obs-wave1/combat")
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(obs_combat_resp.status(), axum::http::StatusCode::OK);
    let obs_combat_str = String::from_utf8_lossy(
        &axum::body::to_bytes(obs_combat_resp.into_body(), 64 * 1024)
            .await
            .unwrap(),
    )
    .to_string();
    assert!(!obs_combat_str.contains("tok-hidden"));

    let obs_map_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("GET")
            .uri("/api/sessions/ep-obs-wave1/map")
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(obs_map_resp.status(), axum::http::StatusCode::OK);
    let obs_map_str = String::from_utf8_lossy(
        &axum::body::to_bytes(obs_map_resp.into_body(), 64 * 1024)
            .await
            .unwrap(),
    )
    .to_string();
    assert!(obs_map_str.contains("tok-pub"));
    assert!(!obs_map_str.contains("tok-hidden"));

    let export_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("GET")
            .uri("/api/sessions/ep-obs-wave1/export")
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(export_resp.status(), axum::http::StatusCode::OK);
    let export_bytes = axum::body::to_bytes(export_resp.into_body(), 256 * 1024)
        .await
        .unwrap();
    let export_str = String::from_utf8_lossy(&export_bytes);
    assert!(!export_str.contains("OBS_SECRET_DOC_TOKEN_99"));
    assert!(!export_str.contains("OBS_SECRET_MEMORY_99"));
    assert!(!export_str.contains("tok-hidden"));
    let bundle: satya_schema::SessionSnapshotBundle =
        serde_json::from_slice(&export_bytes).unwrap();
    assert!(bundle.gm_documents.is_empty());
    assert!(bundle.gm_memories.is_empty());

    let lookup_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("GET")
            .uri("/api/sessions/ep-obs-wave1/lookup?q=polycrisis&scope=CAMPAIGN")
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(lookup_resp.status(), axum::http::StatusCode::OK);
    let lookup_json: satya_schema::GmOmnisearchResponse = serde_json::from_slice(
        &axum::body::to_bytes(lookup_resp.into_body(), 128 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(
        lookup_json
            .hits
            .iter()
            .any(|h| h.source_kind == satya_schema::GmLookupSourceKind::CampaignLore)
    );
    assert!(
        !lookup_json
            .hits
            .iter()
            .any(|h| h.item_id.contains("04-chronicle-and-gm"))
    );

    // 4. Observer orchestrates AI Player + AI GM rally without any human Player in the room
    let spawn_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/sessions/ep-obs-wave1/ai-players")
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&satya_schema::AiPlayerConfig {
                    user_id: Some("ai-player-mateo".to_string()),
                    display_name: None,
                    character_id: None,
                    persona_prompt: Some("Field medic".to_string()),
                    provider: Some("ollama".to_string()),
                    model_id: Some("gemma4:12b".to_string()),
                    service_url: None,
                    auto_respond: true,
                    max_rally_turns: 2,
                })
                .unwrap(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(spawn_resp.status(), axum::http::StatusCode::OK);

    let step_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/sessions/ep-obs-wave1/ai-players/step")
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&satya_schema::StepAiPlayerRequest {
                    user_id: Some("ai-player-mateo".to_string()),
                    prompt: Some("Check the clinic triage desk".to_string()),
                    auto_trigger_gm: true,
                })
                .unwrap(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(step_resp.status(), axum::http::StatusCode::OK);

    let gm_chat_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/gm/sessions/ep-obs-wave1/chat")
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&satya_schema::GmChatRequest {
                    visibility: MessageVisibility::Public,
                    recipient_user_ids: Vec::new(),
                    channel_context: satya_schema::ChannelContext::GmNarration,
                    body: "The clinic lights flicker across triage.".to_string(),
                    model_id: None,
                    speaker_override: None,
                })
                .unwrap(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(gm_chat_resp.status(), axum::http::StatusCode::OK);

    // 5. leave_session scrubs sponsor BYOK key while keeping session alive if AI Player remains,
    // and clears vault once no humans or AI Players remain.
    assert!(
        state
            .auth
            .leave_session("ep-obs-wave1", &obs_auth.user_id)
            .await
            .is_some()
    );
    let (_, vault_mid) = state
        .auth
        .get_session_gm_context("ep-obs-wave1")
        .await
        .expect("session preserved while active AI Player remains");
    assert!(
        vault_mid
            .as_ref()
            .and_then(|v| v.api_key.as_ref())
            .is_none(),
        "Departing sponsor API key must be scrubbed immediately"
    );

    let _ = state
        .auth
        .resume_session(
            &obs_user,
            &satya_schema::ResumeSessionRequest {
                session_id: "ep-obs-wave1".to_string(),
                session_password: Some("room-pass".to_string()),
                join_token: None,
                role: Some(SessionRole::Observer),
                display_name: None,
            },
        )
        .await
        .unwrap();
    let del_ai_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("DELETE")
            .uri("/api/sessions/ep-obs-wave1/ai-players/ai-player-mateo")
            .header("Authorization", format!("Bearer {}", obs_auth.auth_token))
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(del_ai_resp.status(), axum::http::StatusCode::OK);

    assert!(
        state
            .auth
            .leave_session("ep-obs-wave1", &obs_auth.user_id)
            .await
            .is_some()
    );
    let (_, vault_end) = state
        .auth
        .get_session_gm_context("ep-obs-wave1")
        .await
        .unwrap();
    assert!(vault_end.is_none());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_wave5_gm_only_surveillance_filter_and_passive_movement_whisper() {
    let mut cfg = StorageConfig::default();
    let temp_dir = std::env::temp_dir().join(format!("satya_sig_w5_surv_{}", Uuid::now_v7()));
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir
        .join("users_w5_surv.db")
        .to_string_lossy()
        .to_string();
    cfg.ai_gm.service_url = "http://127.0.0.1:59999".to_string();

    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());
    let gm_svc_tok = state.storage.config().ai_gm.service_token.clone();

    let gm_auth = state
        .auth
        .register_user("gm_w5", "satya-truth-2026", "GM W5")
        .await
        .unwrap();
    let gm_user = state
        .auth
        .verify_auth_token(&gm_auth.auth_token)
        .await
        .unwrap();
    let p1_auth = state
        .auth
        .register_user("p1_w5", "satya-truth-2026", "Mateo W5")
        .await
        .unwrap();
    let p1_user = state
        .auth
        .verify_auth_token(&p1_auth.auth_token)
        .await
        .unwrap();
    let obs_auth = state
        .auth
        .register_user("obs_w5", "satya-truth-2026", "Observer W5")
        .await
        .unwrap();
    let obs_user = state
        .auth
        .verify_auth_token(&obs_auth.auth_token)
        .await
        .unwrap();

    state
        .auth
        .create_session(
            &gm_user,
            "ep-wave5-surv",
            "room-pass",
            SessionMode::Ttrpg,
            SessionRole::Gm,
            Some("GM".to_string()),
            true,
        )
        .await
        .unwrap();
    state
        .auth
        .join_session(
            &p1_user,
            "ep-wave5-surv",
            "room-pass",
            SessionRole::Player,
            Some("Mateo".to_string()),
        )
        .await
        .unwrap();
    state
        .auth
        .join_session(
            &obs_user,
            "ep-wave5-surv",
            "room-pass",
            SessionRole::Observer,
            Some("Watcher".to_string()),
        )
        .await
        .unwrap();

    let seeded_map = GmMapUpdateRequest {
        map_id: "map-w5-surv".to_string(),
        title: "St. Jude Clinic Annex".to_string(),
        image_url: String::new(),
        grid_width: 12,
        grid_height: 10,
        cell_size_cm: 100,
        props: vec![satya_maps::map_dressing::make_prop(
            satya_schema::PropKind::HazardSpill,
            5,
            4,
            0,
            Some(1),
            0,
        )],
        tokens: vec![satya_schema::TokenPlacement {
            entity_id: p1_auth.user_id.clone(),
            label: "Mateo".to_string(),
            x: 2,
            y: 4,
            visibility: MessageVisibility::Public,
        }],
        surveillance_nodes: vec![
            satya_schema::MapSurveillanceNode {
                node_id: "cam-public".to_string(),
                label: "Clinic Vestibule Camera".to_string(),
                node_kind_code: "DOME_CAMERA".to_string(),
                floor_index: 0,
                x_cm: 150,
                y_cm: 150,
                facing_deg: 0,
                fov_deg: 90,
                range_cm: 600,
                powered: true,
                visibility: MessageVisibility::Public,
            },
            satya_schema::MapSurveillanceNode {
                node_id: "cam-covert-gm".to_string(),
                label: "COVERT_GM_PINHOLE_77".to_string(),
                node_kind_code: "COVERT_MIC".to_string(),
                floor_index: 0,
                x_cm: 550,
                y_cm: 450,
                facing_deg: 180,
                fov_deg: 120,
                range_cm: 600,
                powered: true,
                visibility: MessageVisibility::GmOnly,
            },
        ],
        ..GmMapUpdateRequest::default()
    };

    // 1. Direct unit check of filter_map_for_role
    let player_filtered = filter_map_for_role(&seeded_map, SessionRole::Player);
    assert_eq!(player_filtered.surveillance_nodes.len(), 1);
    assert_eq!(player_filtered.surveillance_nodes[0].node_id, "cam-public");

    let observer_filtered = filter_map_for_role(&seeded_map, SessionRole::Observer);
    assert_eq!(observer_filtered.surveillance_nodes.len(), 1);
    assert_eq!(observer_filtered.surveillance_nodes[0].node_id, "cam-public");

    let gm_filtered = filter_map_for_role(&seeded_map, SessionRole::Gm);
    assert_eq!(gm_filtered.surveillance_nodes.len(), 2);

    // 2. Seed map via GM service API and check HTTP map redaction for Observer & Player
    let gm_map_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/gm/sessions/ep-wave5-surv/map")
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&seeded_map).unwrap(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(gm_map_resp.status(), axum::http::StatusCode::OK);

    for auth_tok in [&p1_auth.auth_token, &obs_auth.auth_token] {
        let resp = tower::ServiceExt::oneshot(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/sessions/ep-wave5-surv/map")
                .header("Authorization", format!("Bearer {auth_tok}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = String::from_utf8_lossy(
            &axum::body::to_bytes(resp.into_body(), 64 * 1024)
                .await
                .unwrap(),
        )
        .to_string();
        assert!(body.contains("cam-public"));
        assert!(!body.contains("cam-covert-gm"));
        assert!(!body.contains("COVERT_GM_PINHOLE_77"));
    }

    // 3. Moving Mateo near the non-decorative HazardSpill triggers private passive whisper
    //    delivered to Player + GM while staying invisible to Observer.
    let move_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/sessions/ep-wave5-surv/map/move")
            .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::json!({
                    "entity_id": p1_auth.user_id,
                    "to_x": 4,
                    "to_y": 4
                })
                .to_string(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(move_resp.status(), axum::http::StatusCode::OK);

    let (p1_chats, _) = state
        .auth
        .load_visible_events_from_db("ep-wave5-surv", &p1_auth.user_id, SessionRole::Player, true)
        .await;
    assert!(
        p1_chats
            .iter()
            .any(|c| c.body.contains("[Passive OLF]") && c.visibility == MessageVisibility::Private),
        "Moving player should receive private passive perception whisper: {p1_chats:?}"
    );

    let (gm_chats, _) = state
        .auth
        .load_visible_events_from_db("ep-wave5-surv", &gm_auth.user_id, SessionRole::Gm, true)
        .await;
    assert!(
        gm_chats
            .iter()
            .any(|c| c.body.contains("[Passive OLF]") && c.visibility == MessageVisibility::Private),
        "GM should receive copy of private passive perception whisper: {gm_chats:?}"
    );

    let (obs_chats, _) = state
        .auth
        .load_visible_events_from_db(
            "ep-wave5-surv",
            &obs_auth.user_id,
            SessionRole::Observer,
            true,
        )
        .await;
    assert!(
        obs_chats.iter().all(|c| !c.body.contains("[Passive")),
        "Observer must never receive player-private passive perception whispers: {obs_chats:?}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
