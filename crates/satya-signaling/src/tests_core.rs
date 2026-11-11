//! Core unit and integration tests for `satya-signaling`: user authentication,
//! RBAC, WebRTC disconnect lifecycle, AI GM Co-Pilot vs. Autonomous modes, and fail-fast handling.

use super::*;
use axum::http::StatusCode;
use satya_schema::{
    AccountRole, AiGmOperatingMode, AiGmProviderConfig, AiGmTurnStatus, ChannelContext,
    GmChatRequest, GmContextResponse, GmMapUpdateRequest, GmRollRequest, MessageVisibility,
    Participant, SessionMode, SessionRole,
};
use satya_storage::StorageConfig;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_user_auth_session_password_and_private_message_isolation() {
    let temp_dir = std::env::temp_dir().join(format!("satya_signaling_test_{}", Uuid::new_v4()));
    let state = SignalingAppState::new(StorageConfig::default(), &temp_dir)
        .await
        .unwrap();

    // 1. Register and authenticate users
    let gm_auth = state
        .auth
        .register_user("gm_test", "strong-pass-1", "GM Arbiter")
        .await
        .unwrap();
    let p1_auth = state
        .auth
        .register_user("player_one", "strong-pass-2", "Player One")
        .await
        .unwrap();
    let p2_auth = state
        .auth
        .register_user("player_two", "strong-pass-3", "Player Two")
        .await
        .unwrap();

    // Re-login returns the exact same persistent user_id
    let gm_relogin = state
        .auth
        .authenticate_user("gm_test", "strong-pass-1")
        .await
        .unwrap();
    assert_eq!(gm_auth.user_id, gm_relogin.user_id);

    let gm_user = state
        .auth
        .verify_auth_token(&gm_auth.auth_token)
        .await
        .unwrap();
    let p1_user = state
        .auth
        .verify_auth_token(&p1_auth.auth_token)
        .await
        .unwrap();
    let p2_user = state
        .auth
        .verify_auth_token(&p2_auth.auth_token)
        .await
        .unwrap();

    // 2. Host/GM creates a password-protected session
    let (_, gm_part, _) = state
        .auth
        .create_session(
            &gm_user,
            "camp-alpha",
            "secret-room-pw",
            SessionMode::Ttrpg,
            SessionRole::Gm,
            None,
            true,
        )
        .await
        .unwrap();

    // 3. Joining with wrong session password fails
    let bad_join = state
        .auth
        .join_session(
            &p1_user,
            "camp-alpha",
            "wrong-password",
            SessionRole::Player,
            None,
        )
        .await;
    assert!(bad_join.is_err());

    // 4. Joining with valid session password succeeds
    let (_, _, p1_part, _) = state
        .auth
        .join_session(
            &p1_user,
            "camp-alpha",
            "secret-room-pw",
            SessionRole::Player,
            None,
        )
        .await
        .unwrap();
    let (_, _, p2_part, roster) = state
        .auth
        .join_session(
            &p2_user,
            "camp-alpha",
            "secret-room-pw",
            SessionRole::Player,
            None,
        )
        .await
        .unwrap();

    // Roster has GM + Player 1 + Player 2 + AI GM = 4 participants
    assert_eq!(roster.len(), 4);

    // 5. Private message from Player 1 to Player 2 ONLY targets Player 2 (and sender Player 1),
    // excluding GM and AI GM from delivery
    let (recip_records, delivery_ids) = SignalingAppState::resolve_message_targets(
        &p1_part,
        MessageVisibility::Private,
        std::slice::from_ref(&p2_part.user_id),
        &roster,
    );
    assert_eq!(recip_records.len(), 1);
    assert_eq!(recip_records[0].user_id, p2_part.user_id);
    assert!(delivery_ids.contains(&p1_part.user_id));
    assert!(delivery_ids.contains(&p2_part.user_id));
    assert!(!delivery_ids.contains(&gm_part.user_id));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_axum_http_endpoints_and_leptos_render() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let temp_dir = std::env::temp_dir().join(format!("satya_router_test_{}", Uuid::new_v4()));
    let state = SignalingAppState::new(StorageConfig::default(), &temp_dir)
        .await
        .unwrap();
    let app = build_router(state.clone());

    // 1. Verify Leptos HTML SSR shell renders at GET /
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Verify /api/storage/metrics requires ADMIN role (401 unauthenticated, 403 for USER, 200 for ADMIN)
    let res_unauth = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/storage/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_unauth.status(), StatusCode::UNAUTHORIZED);

    // Regular user (sara_valkyrie) is seeded with AccountRole::User -> 403 FORBIDDEN
    let sara_auth = state
        .auth
        .authenticate_user("sara_valkyrie", "satya-truth-2026")
        .await
        .unwrap();
    assert_eq!(sara_auth.account_role, AccountRole::User);
    let res_forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/storage/metrics")
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_forbidden.status(), StatusCode::FORBIDDEN);

    // Bootstrap admin (gm_sean) is seeded with AccountRole::Admin -> 200 OK
    let admin_auth = state
        .auth
        .authenticate_user("gm_sean", "satya-truth-2026")
        .await
        .unwrap();
    assert_eq!(admin_auth.account_role, AccountRole::Admin);
    let res_metrics = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/storage/metrics")
                .header("Authorization", format!("Bearer {}", admin_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_metrics.status(), StatusCode::OK);

    // 3. Verify GET /api/rules and GET /api/rules/:chapter_id serve the canonical Rules Book
    let res_rules = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/rules?q=Watershed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_rules.status(), StatusCode::OK);

    let res_tasks_ch = app
        .oneshot(
            Request::builder()
                .uri("/api/rules/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_tasks_ch.status(), StatusCode::OK);

    let metrics = state.storage.metrics_snapshot().await;
    assert_eq!(metrics.len(), 10);
    let schema_reg_topic = metrics
        .iter()
        .find(|m| m.topic == "schema-registry")
        .unwrap();
    assert_eq!(schema_reg_topic.total_messages_ingested, 11);
    assert_eq!(schema_reg_topic.flushed_batches_count, 1);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_gm_backend_api_byok_vault_and_copilot_vs_autonomous_modes() {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use satya_schema::{CombatStance, TokenPlacement};
    use tower::ServiceExt;

    let temp_dir = std::env::temp_dir().join(format!("satya_gm_api_test_{}", Uuid::new_v4()));
    let state = SignalingAppState::new(StorageConfig::default(), &temp_dir)
        .await
        .unwrap();
    let app = build_router(state.clone());

    let gm_auth = state
        .auth
        .authenticate_user("gm_sean", "satya-truth-2026")
        .await
        .unwrap();
    let sara_auth = state
        .auth
        .authenticate_user("sara_valkyrie", "satya-truth-2026")
        .await
        .unwrap();

    let gm_user = state
        .auth
        .verify_auth_token(&gm_auth.auth_token)
        .await
        .unwrap();
    let sara_user = state
        .auth
        .verify_auth_token(&sara_auth.auth_token)
        .await
        .unwrap();

    // 1. Create a Co-Pilot session (`TTRPG_GM_AI`: Human GM + AI GM + Player)
    state
        .auth
        .create_session(
            &gm_user,
            "ep1-copilot",
            "orbital-7",
            SessionMode::Ttrpg,
            SessionRole::Gm,
            None,
            true,
        )
        .await
        .unwrap();
    let (_, _, sara_part, copilot_roster) = state
        .auth
        .join_session(
            &sara_user,
            "ep1-copilot",
            "orbital-7",
            SessionRole::Player,
            None,
        )
        .await
        .unwrap();

    let ai_gm_part = copilot_roster
        .iter()
        .find(|p| p.role == SessionRole::AiGm)
        .unwrap()
        .clone();

    // Verify Co-Pilot vs Autonomous trigger filter rules:
    // In Co-Pilot mode (Human GM present), normal public IC chat does NOT trigger AI GM
    assert!(!should_trigger_ai_gm_for_chat(
        &copilot_roster,
        &ai_gm_part,
        MessageVisibility::Public,
        &[],
        ChannelContext::InCharacter,
        "I open the cargo bay door.",
    ));
    // Addressing @satya_ai_gm or sending GM_ONLY DOES trigger AI GM in Co-Pilot mode
    assert!(should_trigger_ai_gm_for_chat(
        &copilot_roster,
        &ai_gm_part,
        MessageVisibility::Public,
        &[],
        ChannelContext::InCharacter,
        "@satya_ai_gm look up Captain Vance's stats",
    ));
    assert!(should_trigger_ai_gm_for_chat(
        &copilot_roster,
        &ai_gm_part,
        MessageVisibility::GmOnly,
        &[],
        ChannelContext::InCharacter,
        "Roll 3d20 for the two sentries",
    ));
    assert!(!should_trigger_ai_gm_for_roll(&copilot_roster));

    // In Autonomous Solo AI GM mode (no human GM in roster), BOTH InCharacter and OutOfCharacter trigger the AI GM
    let autonomous_roster = vec![sara_part.clone(), ai_gm_part.clone()];
    assert!(should_trigger_ai_gm_for_chat(
        &autonomous_roster,
        &ai_gm_part,
        MessageVisibility::Public,
        &[],
        ChannelContext::OutOfCharacter,
        "How do we do combat? Are we at tick 0 now? how do I use my Tempo, Control and Force pool here?",
    ));
    let (triad_res, triad_env) = crate::gm_api::evaluate_roll_combat_action_and_triad(
        Some("snap_shot"),
        Some(5),
        Some(3),
        Some(3),
        Some(CombatStance::RangedSnap),
        12,
        3,
        0,
        0.0,
    )
    .expect("snap_shot with T/C/F should evaluate");
    assert_eq!(triad_res.technique.as_str(), "snap_shot");
    assert!(triad_res.effective_tc >= 1);
    assert!(triad_env.p_eff > 0);

    // 2. Configure Ephemeral BYOK Credentials via POST /api/sessions/:id/gm-config
    let cfg_req = Request::builder()
        .method("POST")
        .uri("/api/sessions/ep1-copilot/gm-config")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .body(Body::from(
            serde_json::to_string(&AiGmProviderConfig {
                provider: "google_genai".to_string(),
                model_id: "gemini-2.5-flash".to_string(),
                vision_model_id: Some("gemini-2.5-flash".to_string()),
                temperature: 0.3,
                persona_id: "harsh-reality-arbiter".to_string(),
                api_key: Some("ephemeral-secret-key-xyz999".to_string()),
                base_url: None,
            })
            .unwrap(),
        ))
        .unwrap();
    let cfg_res = app.clone().oneshot(cfg_req).await.unwrap();
    assert_eq!(cfg_res.status(), StatusCode::OK);

    // Verify GET /api/gm/sessions/:id/context requires service_token (401 without token)
    let unauth_ctx = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/gm/sessions/ep1-copilot/context")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth_ctx.status(), StatusCode::UNAUTHORIZED);

    // Authorized GET /api/gm/sessions/:id/context returns CO_PILOT mode and redacts api_key!
    let ctx_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/gm/sessions/ep1-copilot/context")
                .header("Authorization", "Bearer satya_gm_secret_dev_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ctx_res.status(), StatusCode::OK);
    let ctx_bytes = to_bytes(ctx_res.into_body(), usize::MAX).await.unwrap();
    let ctx_str = String::from_utf8(ctx_bytes.to_vec()).unwrap();
    assert!(!ctx_str.contains("ephemeral-secret-key-xyz999"));
    let gm_ctx: GmContextResponse = serde_json::from_str(&ctx_str).unwrap();
    assert_eq!(gm_ctx.operating_mode, AiGmOperatingMode::CoPilot);
    assert_eq!(
        gm_ctx.human_gm_user_id.as_deref(),
        Some(gm_user.user_id.as_str())
    );
    assert_eq!(
        gm_ctx
            .ai_gm_participant
            .unwrap()
            .ai_agent_info
            .unwrap()
            .model_id,
        "google_genai/gemini-2.5-flash"
    );

    // 3. Test POST /api/gm/sessions/:id/roll, /chat, /map, and /trajectory
    let roll_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/gm/sessions/ep1-copilot/roll")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer satya_gm_secret_dev_token")
                .body(Body::from(
                    serde_json::to_string(&GmRollRequest {
                        actor_display_name: "Station SecOps Sentry".to_string(),
                        target_user_ids: vec![sara_part.user_id.clone()],
                        skill_name: "Suppressive Fire".to_string(),
                        attribute_tn: 13,
                        pool_size: 3,
                        known_tool_mod: 0,
                        difficulty_rating: 2,
                        internal_mod: Some(1),
                        delta_over: None,
                        gm_resistance_mode: None,
                        tie_rule: None,
                        carry_mode: Some("markov_step".to_string()),
                        botch_rule: None,
                        combat_stance: Some(CombatStance::RangedSnap),
                        combat_technique: Some("snap_shot".to_string()),
                        tempo_pool: Some(4),
                        control_pool: Some(3),
                        force_pool: Some(3),
                        fatigue_level: 1,
                        wound_shock_index: 0.0,
                        notes: Some("Sentry reacts to corridor movement".to_string()),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(roll_res.status(), StatusCode::OK);

    let map_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/gm/sessions/ep1-copilot/map")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer satya_gm_secret_dev_token")
                .body(Body::from(
                    serde_json::to_string(&GmMapUpdateRequest {
                        map_id: "sector-4-airlock".to_string(),
                        title: "Sector 4 Cargo Airlock".to_string(),
                        image_url: "/maps/ep1-copilot/sector-4-airlock.svg".to_string(),
                        grid_width: 20,
                        grid_height: 15,
                        tokens: vec![TokenPlacement {
                            entity_id: sara_part.user_id.clone(),
                            label: "Sara".to_string(),
                            x: 5,
                            y: 7,
                            visibility: MessageVisibility::Public,
                        }],
                        notes: Some("Low-gravity cargo bay".to_string()),
                        ..Default::default()
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(map_res.status(), StatusCode::OK);

    let chat_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/gm/sessions/ep1-copilot/chat")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer satya_gm_secret_dev_token")
                .body(Body::from(
                    serde_json::to_string(&GmChatRequest {
                        visibility: MessageVisibility::Private,
                        recipient_user_ids: vec![gm_user.user_id.clone()],
                        channel_context: ChannelContext::GmNarration,
                        body: "[Co-Pilot] Captain Vance TN 14 Leadership, wary disposition."
                            .to_string(),
                        model_id: None,
                        speaker_override: None,
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chat_res.status(), StatusCode::OK);

    let traj_res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/gm/sessions/ep1-copilot/trajectory")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer satya_gm_secret_dev_token")
                .body(Body::from(
                    serde_json::to_string(&GmTrajectoryReportRequest {
                        turn_id: "turn-copilot-1".to_string(),
                        trace_id: "0123456789abcdef0123456789abcdef".to_string(),
                        operating_mode: AiGmOperatingMode::CoPilot,
                        trigger_actor: sara_part.clone(),
                        trigger_event_id: "msg-1".to_string(),
                        trigger_summary: "Look up Captain Vance".to_string(),
                        llm_provider: "google_genai".to_string(),
                        reasoning_model_id: "gemini-2.5-flash".to_string(),
                        vision_model_id: None,
                        ruleset_engine_id: "satya-d20".to_string(),
                        tool_invocations: vec![],
                        rag_sources_used: vec!["data/npcs/sector_4_npcs.toml".to_string()],
                        emitted_event_ids: vec![],
                        prompt_tokens: 410,
                        completion_tokens: 65,
                        cached_tokens: 0,
                        total_turn_latency_ms: 310,
                        turn_status: AiGmTurnStatus::Completed,
                        turn_logs: vec![GmSessionLogEntry {
                            level: satya_schema::LogLevel::Info,
                            span_id: Some("a1b2c3d4e5f60001".to_string()),
                            parent_span_id: Some("00f067aa0ba902b7".to_string()),
                            lineage_id: Some("L1".to_string()),
                            parent_lineage_ids: vec!["msg-1".to_string()],
                            log_line: "[2026-11-02T22:10:28.112Z] [INFO] [session=ep1-copilot] [trace=0123456789abcdef0123456789abcdef span=a1b2c3d4e5f60001<-00f067aa0ba902b7] [lineage=L1<-msg-1] satya_ml.ai_gm.graph: Completed Co-Pilot turn".to_string(),
                        }],
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(traj_res.status(), StatusCode::OK);
    let gm_metrics = state.storage.metrics_snapshot().await;
    assert_eq!(
        gm_metrics
            .iter()
            .find(|m| m.topic == "ai-gm-trajectories")
            .unwrap()
            .total_messages_ingested,
        1
    );
    assert_eq!(
        gm_metrics
            .iter()
            .find(|m| m.topic == "system-logs")
            .unwrap()
            .total_messages_ingested,
        1
    );

    // 4. Verify ephemeral BYOK key is scrubbed when sponsoring user (GM) leaves the session
    let (_, vault_before) = state
        .auth
        .get_session_gm_context("ep1-copilot")
        .await
        .unwrap();
    assert_eq!(
        vault_before.unwrap().api_key.as_deref(),
        Some("ephemeral-secret-key-xyz999")
    );
    state
        .auth
        .leave_session("ep1-copilot", &gm_user.user_id)
        .await;
    let (ctx_after_gm_left, vault_after) = state
        .auth
        .get_session_gm_context("ep1-copilot")
        .await
        .unwrap();
    assert!(vault_after.unwrap().api_key.is_none());
    // With no human GM remaining, session automatically transitions to AUTONOMOUS mode!
    assert_eq!(
        ctx_after_gm_left.operating_mode,
        AiGmOperatingMode::Autonomous
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_session_leave_and_user_logout_emit_disconnect_events() {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let temp_dir = std::env::temp_dir().join(format!("satya_disconnect_test_{}", Uuid::new_v4()));
    let state = SignalingAppState::new(StorageConfig::default(), &temp_dir)
        .await
        .unwrap();
    let app = build_router(state.clone());

    let gm_auth = state
        .auth
        .register_user("gm_disc", "strong-pass-1", "GM Sean")
        .await
        .unwrap();
    let p1_auth = state
        .auth
        .register_user("p1_disc", "strong-pass-2", "Player Sara")
        .await
        .unwrap();

    let gm_user = state
        .auth
        .verify_auth_token(&gm_auth.auth_token)
        .await
        .unwrap();
    let p1_user = state
        .auth
        .verify_auth_token(&p1_auth.auth_token)
        .await
        .unwrap();

    let (gm_join_tok, _, _) = state
        .auth
        .create_session(
            &gm_user,
            "ep-disconnect",
            "room-secret",
            SessionMode::Ttrpg,
            SessionRole::Gm,
            None,
            true,
        )
        .await
        .unwrap();
    let (p1_join_tok, _, _, roster_before) = state
        .auth
        .join_session(
            &p1_user,
            "ep-disconnect",
            "room-secret",
            SessionRole::Player,
            None,
        )
        .await
        .unwrap();
    assert_eq!(roster_before.len(), 3); // GM + Player 1 + AI GM

    // 1. Player 1 disconnects from the session via POST /api/sessions/leave
    let leave_req = Request::builder()
        .method("POST")
        .uri("/api/sessions/leave")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "session_id": "ep-disconnect",
                "reason": "Player Sara clicked Disconnect"
            }))
            .unwrap(),
        ))
        .unwrap();
    let leave_res = app.clone().oneshot(leave_req).await.unwrap();
    assert_eq!(leave_res.status(), StatusCode::OK);

    // Player 1's join token is invalidated and roster now has 2 participants (GM + AI GM)
    assert!(state.auth.verify_join_token(&p1_join_tok).await.is_none());
    assert!(state.auth.verify_join_token(&gm_join_tok).await.is_some());
    let roster_after_leave = state.auth.get_session_roster("ep-disconnect").await;
    assert_eq!(roster_after_leave.len(), 2);

    // 2. GM logs out completely via POST /api/auth/logout while still in the session
    let logout_req = Request::builder()
        .method("POST")
        .uri("/api/auth/logout")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .body(Body::from(
            serde_json::to_string(&json!({
                "session_id": "ep-disconnect",
                "reason": "GM logged out"
            }))
            .unwrap(),
        ))
        .unwrap();
    let logout_res = app.oneshot(logout_req).await.unwrap();
    assert_eq!(logout_res.status(), StatusCode::OK);

    // GM's auth token and join token are both revoked
    assert!(
        state
            .auth
            .verify_auth_token(&gm_auth.auth_token)
            .await
            .is_none()
    );
    assert!(state.auth.verify_join_token(&gm_join_tok).await.is_none());

    // Verify disconnect & logout events were logged to `session-lifecycle` and `webrtc-telemetry`
    let metrics = state.storage.metrics_snapshot().await;
    let lifecycle_metrics = metrics
        .iter()
        .find(|m| m.topic == "session-lifecycle")
        .unwrap();
    // Events: P1 ParticipantLeft (1) + GM ParticipantLeft (2) + SessionEnded (3, since 0 humans left) + GM UserLoggedOut (4)
    assert_eq!(lifecycle_metrics.total_messages_ingested, 4);

    let webrtc_metrics = metrics
        .iter()
        .find(|m| m.topic == "webrtc-telemetry")
        .unwrap();
    // Events: P1 PeerDisconnected (1) + GM PeerDisconnected (2)
    assert_eq!(webrtc_metrics.total_messages_ingested, 2);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

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
    assert_eq!(generated_map.tiles.len(), 280);
    assert!(
        generated_map
            .tokens
            .iter()
            .any(|t| t.entity_id == p1_auth.user_id)
    );

    // 2. Player attempting to step into outer perimeter FullWall (0, 0) gets 400 BAD_REQUEST
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
                facing_deg: None,
                model_preset: None,
                color_hex: None,
                stance_pose: None,
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
                facing_deg: None,
                model_preset: None,
                color_hex: None,
                stance_pose: None,
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

    // 4. Player moves their own miniature to unblocked floor cell (3, 7) -> 200 OK
    let ok_move_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-3d/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: 3,
                to_y: 7,
                facing_deg: None,
                model_preset: Some("valkyrie_rig".to_string()),
                color_hex: Some("#38bdf8".to_string()),
                stance_pose: None,
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
                facing_deg: Some(270),
                model_preset: Some("secops_sentry".to_string()),
                color_hex: Some("#f43f5e".to_string()),
                stance_pose: None,
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

    let _ = std::fs::remove_dir_all(&temp_dir);
}
