use crate::build_router;
use crate::state::SignalingAppState;
use axum::{
    body::{Body, to_bytes},
    http::{HeaderMap, HeaderValue, Request, StatusCode, header},
};
use satya_schema::{
    AiGmProviderConfig, CharacterSheetRecord, GmMapUpdateRequest, GmMemoryCategory,
    GmMemoryNoteRecord, ImportSessionSnapshotRequest, ResumeSessionRequest, SaveCharacterRequest,
    SaveMapRequest, SessionHistorySnapshot, SessionMode, SessionRole, SessionSnapshotBundle,
    UpsertGmMemoryRequest,
};
use satya_storage::StorageConfig;
use serde_json::json;
use sqlx::Row;
use tower::ServiceExt;
use uuid::Uuid;

fn new_temp_dir(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{prefix}_{}", Uuid::now_v7()))
}

#[tokio::test]
async fn test_wave2_demo_seeding_disabled_uniform_login_error_and_password_policy() {
    let temp_dir = new_temp_dir("satya_sec_auth");
    let mut cfg = StorageConfig::default();
    cfg.auth.seed_demo_accounts = Some(false);
    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    // Demo accounts are NOT seeded when seed_demo_accounts = Some(false).
    assert!(
        state
            .auth
            .authenticate_user("gm_sean", "satya-truth-2026")
            .await
            .is_err()
    );

    // Registration rejects short passwords (< 8), oversized passwords (> 128), and invalid usernames.
    for (uname, pw, expected_status) in [
        ("ab", "strong-pass-1", StatusCode::BAD_REQUEST),
        ("bad user!", "strong-pass-1", StatusCode::BAD_REQUEST),
        ("valid_user", "short7", StatusCode::BAD_REQUEST),
        ("valid_user", &"a".repeat(129), StatusCode::BAD_REQUEST),
    ] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "username": uname,
                            "password": pw,
                            "display_name": "Sec Tester"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), expected_status, "uname={uname}");
    }

    // Valid user registration works, and unknown user vs wrong password produce the exact same error.
    state
        .auth
        .register_user("alice_sec", "strong-pass-1", "Alice Sec")
        .await
        .unwrap();
    let err_unknown = state
        .auth
        .authenticate_user("no_such_user", "strong-pass-1")
        .await
        .unwrap_err()
        .to_string();
    let err_wrong_pw = state
        .auth
        .authenticate_user("alice_sec", "wrong-pass-1")
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(err_unknown, "Invalid username or password");
    assert_eq!(err_wrong_pw, "Invalid username or password");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_wave2_login_rate_limit_retry_after_and_xff_spoof_ignored() {
    let temp_dir = new_temp_dir("satya_sec_ratelimit");
    let mut cfg = StorageConfig::default();
    cfg.auth.seed_demo_accounts = Some(false);
    cfg.quotas.max_logins_per_username_per_min = 3;
    cfg.quotas.max_logins_per_ip_per_min = 100;
    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    state
        .auth
        .register_user("ratelimited_user", "strong-pass-1", "Rate Limited")
        .await
        .unwrap();

    // Even if client sends different X-Forwarded-For headers (trusted_proxy_hops == 0),
    // the per-username limiter kicks in and returns 429 + Retry-After.
    for i in 0..3 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("Content-Type", "application/json")
                    .header("X-Forwarded-For", format!("203.0.113.{}", i + 1))
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "username": "ratelimited_user",
                            "password": "wrong-password"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    let resp4 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .header("X-Forwarded-For", "203.0.113.99")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "username": "ratelimited_user",
                        "password": "wrong-password"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp4.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(resp4.headers().get("retry-after").is_some());

    // Confirm extract_client_ip ignores X-Forwarded-For by default.
    let mut hdrs = HeaderMap::new();
    hdrs.insert("x-forwarded-for", HeaderValue::from_static("1.2.3.4"));
    let extracted = crate::rate_limit::extract_client_ip(
        &hdrs,
        Some("198.51.100.7".parse().unwrap()),
    );
    assert_eq!(extracted, "198.51.100.7");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_wave2_token_expiry_logout_all_and_ws_ticket_origin_guards() {
    let temp_dir = new_temp_dir("satya_sec_tokens_ws");
    let mut cfg = StorageConfig::default();
    cfg.auth.seed_demo_accounts = Some(false);
    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    let reg1 = state
        .auth
        .register_user("tok_user", "strong-pass-1", "Token User")
        .await
        .unwrap();
    let login2 = state
        .auth
        .authenticate_user("tok_user", "strong-pass-1")
        .await
        .unwrap();
    let user = state.auth.verify_auth_token(&reg1.auth_token).await.unwrap();
    let (join_tok, claims_participant, _) = state
        .auth
        .create_session(
            &user,
            "sec-ws-room",
            "room-pass",
            SessionMode::Ttrpg,
            SessionRole::Gm,
            None,
            false,
        )
        .await
        .unwrap();
    assert_eq!(claims_participant.user_id, user.user_id);

    // 1. Issue single-use WS ticket and verify missing/wrong/evil-origin rejection.
    let ticket_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ws/ticket")
                .header("Authorization", format!("Bearer {}", reg1.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "kind": "session",
                        "join_token": join_tok,
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ticket_resp.status(), StatusCode::OK);
    let ticket_body: serde_json::Value = serde_json::from_slice(
        &to_bytes(ticket_resp.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    let ticket = ticket_body["ticket"].as_str().unwrap().to_string();

    // Cross-origin upgrade attempt is rejected with 403 before consuming the ticket (or even with valid host).
    let evil_ws_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/ws/session?ticket={ticket}"))
                .header("Host", "127.0.0.1:3000")
                .header("Origin", "https://evil.example")
                .header("Connection", "upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(evil_ws_resp.status(), StatusCode::FORBIDDEN);

    // Same-origin header verification passes `verify_ws_origin`.
    let mut same_origin_hdrs = HeaderMap::new();
    same_origin_hdrs.insert("host", HeaderValue::from_static("127.0.0.1:3000"));
    same_origin_hdrs.insert("origin", HeaderValue::from_static("http://127.0.0.1:3000"));
    assert!(crate::security::verify_ws_origin(&same_origin_hdrs).is_ok());

    // Direct ticket consumption succeeds once; replay returns 401.
    let (consumed_claims, _) =
        crate::ws::consume_ws_session_ticket(&state, Some(&ticket))
            .await
            .expect("first WS ticket use should succeed");
    assert_eq!(consumed_claims.session_id, "sec-ws-room");
    let replay_err = crate::ws::consume_ws_session_ticket(&state, Some(&ticket))
        .await
        .unwrap_err();
    assert_eq!(replay_err.0, StatusCode::UNAUTHORIZED);

    // 2. Expired join_token is rejected.
    {
        let mut jt_guard = state.auth.join_tokens.write().await;
        jt_guard.get_mut(&join_tok).unwrap().expires_at_us = 1;
    }
    assert!(state.auth.verify_join_token(&join_tok).await.is_none());

    // 3. POST /api/auth/logout-all revokes all active bearer tokens for that account.
    let logout_all_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout-all")
                .header("Authorization", format!("Bearer {}", reg1.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout_all_resp.status(), StatusCode::OK);
    assert!(state.auth.verify_auth_token(&reg1.auth_token).await.is_none());
    assert!(state.auth.verify_auth_token(&login2.auth_token).await.is_none());

    // 4. Expired auth_token is rejected on verify.
    let login3 = state
        .auth
        .authenticate_user("tok_user", "strong-pass-1")
        .await
        .unwrap();
    {
        let mut at_guard = state.auth.auth_tokens.write().await;
        at_guard.get_mut(&login3.auth_token).unwrap().expires_at_us = 1;
    }
    assert!(state.auth.verify_auth_token(&login3.auth_token).await.is_none());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_wave2_security_headers_and_authenticated_maps_route() {
    let temp_dir = new_temp_dir("satya_sec_headers");
    let mut cfg = StorageConfig::default();
    cfg.auth.seed_demo_accounts = Some(false);
    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    let index_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(index_resp.status(), StatusCode::OK);
    let hdrs = index_resp.headers();
    assert_eq!(
        hdrs.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff"
    );
    assert_eq!(hdrs.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
    assert_eq!(hdrs.get(header::REFERRER_POLICY).unwrap(), "no-referrer");
    assert!(hdrs.get("permissions-policy").is_some());
    assert!(hdrs.get("cross-origin-opener-policy").is_some());
    assert!(hdrs.get("content-security-policy").is_some());

    // Unauthenticated `/maps/*` is rejected with 401.
    let unauth_map = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/maps/sample.png")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth_map.status(), StatusCode::UNAUTHORIZED);

    // Authenticated traversal attempt returns 400.
    let reg = state
        .auth
        .register_user("map_reader", "strong-pass-1", "Map Reader")
        .await
        .unwrap();
    let trav_map = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/maps/..%2F..%2Fetc%2Fpasswd")
                .header("Authorization", format!("Bearer {}", reg.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(trav_map.status(), StatusCode::BAD_REQUEST);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_wave3_directory_privacy_join_request_discoverability_and_purge() {
    let temp_dir = new_temp_dir("satya_sec_privacy");
    let mut cfg = StorageConfig::default();
    cfg.auth.seed_demo_accounts = Some(false);
    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    let gm = state
        .auth
        .register_user("hidden_gm_owner", "strong-pass-1", "Hidden GM")
        .await
        .unwrap();
    let searcher = state
        .auth
        .register_user("searcher_user", "strong-pass-1", "Searcher")
        .await
        .unwrap();
    let hidden_user = state
        .auth
        .register_user("hidden_account_only", "strong-pass-1", "Hidden Account")
        .await
        .unwrap();

    // GM creates a private non-discoverable session.
    let gm_account = state.auth.verify_auth_token(&gm.auth_token).await.unwrap();
    state
        .auth
        .create_session_with_params(
            &gm_account,
            crate::auth::CreateSessionParams {
                session_id: "hidden-room",
                session_password: "secret-room-pw",
                session_mode: SessionMode::Ttrpg,
                role: SessionRole::Gm,
                display_name_override: None,
                enable_ai_gm: false,
                campaign_id: None,
                title: Some("Private Game".to_string()),
                is_discoverable: false,
                allow_join_requests: true,
            },
        )
        .await
        .unwrap();

    // Non-participant cannot submit a join request to a non-discoverable session (403).
    let jr_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/hidden-room/join-requests")
                .header("Authorization", format!("Bearer {}", searcher.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "requested_role": "PLAYER" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(jr_resp.status(), StatusCode::FORBIDDEN);

    // Directory search returns empty results for queries < 3 chars and wildcard-only strings.
    for query in ["ab", "%%%", "___"] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/directory/search?q={}", query.replace('%', "%25")))
                    .header("Authorization", format!("Bearer {}", searcher.auth_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let q_body: serde_json::Value = serde_json::from_slice(
            &to_bytes(res.into_body(), usize::MAX).await.unwrap(),
        )
        .unwrap();
        assert_eq!(q_body["gms"].as_array().unwrap().len(), 0, "query={query}");
    }

    // Search for `hidden_account` (zero discoverable campaigns/sessions) is suppressed for others.
    let search_other = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/directory/search?q=hidden_account")
                .header("Authorization", format!("Bearer {}", searcher.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_other.status(), StatusCode::OK);
    let body_other: serde_json::Value = serde_json::from_slice(
        &to_bytes(search_other.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body_other["gms"].as_array().unwrap().len(), 0);

    // Search for own handle returns self.
    let search_self = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/directory/search?q=hidden_account")
                .header("Authorization", format!("Bearer {}", hidden_user.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_self.status(), StatusCode::OK);
    let body_self: serde_json::Value = serde_json::from_slice(
        &to_bytes(search_self.into_body(), usize::MAX).await.unwrap(),
    )
    .unwrap();
    assert_eq!(body_self["gms"].as_array().unwrap().len(), 1);

    // Non-persistent gm_memories + session_npcs purged when session is deleted.
    state
        .auth
        .upsert_session_gm_memory(
            "hidden-room",
            &gm.user_id,
            "Hidden GM",
            false,
            UpsertGmMemoryRequest {
                memory_id: Some("mem-ephemeral-1".to_string()),
                campaign_tag: None,
                category: GmMemoryCategory::SceneState,
                title: "Temp Note".to_string(),
                content: "Secret".to_string(),
                is_pinned: false,
                is_persistent: false,
            },
        )
        .await
        .unwrap();

    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/sessions/hidden-room")
                .header("Authorization", format!("Bearer {}", gm.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    let mem_cnt: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM gm_memories WHERE memory_id = 'mem-ephemeral-1'")
            .fetch_one(state.auth.db_pool())
            .await
            .unwrap();
    assert_eq!(mem_cnt, 0);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_wave3_fail_closed_service_token_ssrf_and_crlf_api_key_guards() {
    let temp_dir = new_temp_dir("satya_sec_ssrf");
    let mut cfg = StorageConfig::default();
    cfg.auth.seed_demo_accounts = Some(false);
    cfg.ai_gm.service_token = String::new();
    cfg.ai_gm.allow_dev_service_token_fallback = Some(false);
    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    // Fail-closed: empty service_token + allow_dev_service_token_fallback = Some(false) rejects dev token.
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/gm/sessions/no-room/context")
                .header("Authorization", "Bearer satya_gm_secret_dev_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // BYOK API key CRLF injection guard.
    assert!(crate::security::validate_api_key_header_safe("sk-test\r\nEvil: 1").is_err());
    assert!(crate::security::validate_api_key_header_safe("sk-valid-key-123").is_ok());

    // BYOK SSRF guards.
    assert!(
        crate::security::validate_provider_base_url(
            "openai",
            "http://169.254.169.254/latest/meta-data",
            "http://127.0.0.1:11434",
            &[],
        )
        .is_err()
    );
    assert!(
        crate::security::validate_provider_base_url(
            "openai",
            "http://127.0.0.1:9000",
            "http://127.0.0.1:11434",
            &[],
        )
        .is_err()
    );
    assert!(
        crate::security::validate_provider_base_url(
            "openai",
            "https://api.openai.com/v1",
            "http://127.0.0.1:11434",
            &[],
        )
        .is_ok()
    );
    assert!(
        crate::security::validate_provider_base_url(
            "ollama",
            "http://127.0.0.1:11434",
            "http://127.0.0.1:11434",
            &[],
        )
        .is_ok()
    );

    // Self-port loopback SSRF guard for AI Player URLs.
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    assert!(
        crate::player_ai_api::validate_ai_player_service_url(
            &format!("http://127.0.0.1:{port}"),
            &[],
        )
        .is_err()
    );
    assert!(
        crate::player_ai_api::validate_ai_player_service_url("http://127.0.0.1:8091", &[]).is_ok()
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_wave4_path_traversal_unicode_panic_idor_observer_and_body_limit() {
    let temp_dir = new_temp_dir("satya_sec_wave4");
    let mut cfg = StorageConfig::default();
    cfg.auth.seed_demo_accounts = Some(false);
    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    let gm_a = state
        .auth
        .register_user("gm_wave4_a", "strong-pass-1", "GM A")
        .await
        .unwrap();
    let gm_b = state
        .auth
        .register_user("gm_wave4_b", "strong-pass-1", "GM B")
        .await
        .unwrap();
    let player_u = state
        .auth
        .register_user("player_wave4", "strong-pass-1", "Player W4")
        .await
        .unwrap();
    let gm_a_acc = state.auth.verify_auth_token(&gm_a.auth_token).await.unwrap();
    let gm_b_acc = state.auth.verify_auth_token(&gm_b.auth_token).await.unwrap();
    let player_acc = state
        .auth
        .verify_auth_token(&player_u.auth_token)
        .await
        .unwrap();

    // 1. Unicode 'İ' in HTML/media stripper must not panic.
    let (stripped, modified) =
        crate::gm_docs::strip_html_and_media_tags("İstanbul <script>alert(1)</script> ok");
    assert!(modified);
    assert!(stripped.contains("İstanbul"));
    assert!(!stripped.contains("alert"));

    // 2. Map Vault and Character Vault path traversal IDs are rejected with 400.
    let bad_map = GmMapUpdateRequest {
        map_id: "../../pwn".to_string(),
        ..Default::default()
    };
    let map_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/maps")
                .header("Authorization", format!("Bearer {}", gm_a.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&SaveMapRequest {
                        map: bad_map,
                        save_as_new: false,
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(map_res.status(), StatusCode::BAD_REQUEST);

    let bad_char = CharacterSheetRecord {
        character_id: "../../pwn".to_string(),
        character_name: "Trav".to_string(),
        ..Default::default()
    };
    let char_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/characters")
                .header("Authorization", format!("Bearer {}", gm_a.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&SaveCharacterRequest {
                        character: bad_char,
                        session_id: None,
                        bind_to_session: false,
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(char_res.status(), StatusCode::BAD_REQUEST);

    // 3. Cross-session GM memory IDOR: GM B cannot overwrite or delete GM A's memory_id.
    state
        .auth
        .create_session(&gm_a_acc, "room-a", "pw", SessionMode::Ttrpg, SessionRole::Gm, None, false)
        .await
        .unwrap();
    state
        .auth
        .create_session(&gm_b_acc, "room-b", "pw", SessionMode::Ttrpg, SessionRole::Gm, None, false)
        .await
        .unwrap();

    state
        .auth
        .upsert_session_gm_memory(
            "room-a",
            &gm_a.user_id,
            "GM A",
            false,
            UpsertGmMemoryRequest {
                memory_id: Some("mem-shared-probe".to_string()),
                campaign_tag: None,
                category: GmMemoryCategory::NpcSecret,
                title: "Owner A".to_string(),
                content: "Secret A".to_string(),
                is_pinned: false,
                is_persistent: true,
            },
        )
        .await
        .unwrap();

    assert!(
        state
            .auth
            .upsert_session_gm_memory(
                "room-b",
                &gm_b.user_id,
                "GM B",
                false,
                UpsertGmMemoryRequest {
                    memory_id: Some("mem-shared-probe".to_string()),
                    campaign_tag: None,
                    category: GmMemoryCategory::NpcSecret,
                    title: "Owner B Hijack".to_string(),
                    content: "Hijacked".to_string(),
                    is_pinned: false,
                    is_persistent: true,
                },
            )
            .await
            .is_err()
    );
    assert!(
        !state
            .auth
            .delete_session_gm_memory("room-b", Some(&gm_b.user_id), false, "mem-shared-probe")
            .await
            .unwrap()
    );
    let row = sqlx::query("SELECT title FROM gm_memories WHERE memory_id = 'mem-shared-probe'")
        .fetch_one(state.auth.db_pool())
        .await
        .unwrap();
    assert_eq!(row.try_get::<String, _>("title").unwrap(), "Owner A");

    // 4. Observer -> Player/GM escalation blocked on resume_session; AI player removal rejects human.
    state
        .auth
        .join_session(&player_acc, "room-a", "pw", SessionRole::Observer, None)
        .await
        .unwrap();
    assert!(
        state
            .auth
            .resume_session(
                &player_acc,
                &ResumeSessionRequest {
                    session_id: "room-a".to_string(),
                    session_password: Some("pw".to_string()),
                    join_token: None,
                    role: Some(SessionRole::Gm),
                    display_name: None,
                },
            )
            .await
            .is_err()
    );

    let rm_human_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/sessions/room-a/ai-players/{}", player_u.user_id))
                .header("Authorization", format!("Bearer {}", gm_a.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rm_human_resp.status(), StatusCode::BAD_REQUEST);

    // 5. Snapshot import strips raw HTML from gm_memories and rejects oversized 8MiB+ HTTP bodies.
    let imported = state
        .auth
        .import_session_snapshot(
            &gm_a_acc,
            ImportSessionSnapshotRequest {
                bundle: SessionSnapshotBundle {
                    format_version: 1,
                    exported_at_us: 1,
                    session_id: "room-imported".to_string(),
                    session_mode: SessionMode::Ttrpg,
                    campaign_tag: "awakened".to_string(),
                    roster: Vec::new(),
                    active_map: None,
                    combat_tracker: None,
                    gm_documents: Vec::new(),
                    gm_memories: vec![GmMemoryNoteRecord {
                        memory_id: "mem-imp-1".to_string(),
                        session_id: "room-imported".to_string(),
                        campaign_tag: "room-imported".to_string(),
                        author_user_id: gm_a.user_id.clone(),
                        author_display_name: "GM A".to_string(),
                        category: GmMemoryCategory::SceneState,
                        title: "Scene".to_string(),
                        content: "Hello <img src=x onerror=alert(1)> world".to_string(),
                        is_pinned: false,
                        is_persistent: true,
                        updated_at_us: 1,
                    }],
                    session_characters: Vec::new(),
                    chat_events: Vec::new(),
                    mechanics_events: Vec::new(),
                    ai_players: Vec::new(),
                },
                session_id_override: Some("room-imported-sec".to_string()),
                session_password: "import-pw".to_string(),
                role: Some(SessionRole::Gm),
            },
        )
        .await
        .unwrap();
    assert!(imported.0.starts_with("join_"));
    let mem_content: String =
        sqlx::query_scalar("SELECT content FROM gm_memories WHERE campaign_tag = 'room-imported-sec' LIMIT 1")
            .fetch_one(state.auth.db_pool())
            .await
            .unwrap();
    assert!(!mem_content.contains("<img"));

    // Oversized body (> 8 MiB) rejected with 413 Payload Too Large.
    let big_body = vec![b'a'; 8 * 1024 * 1024 + 1024];
    let big_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(big_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(big_resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_account_recovery_email_username_reset_and_anti_enumeration() {
    let temp_dir = new_temp_dir("satya_sec_recovery");
    let mut cfg = StorageConfig::default();
    cfg.auth.seed_demo_accounts = Some(false);
    cfg.quotas.max_recovery_requests_per_ip_per_hour = 20;
    cfg.quotas.max_recovery_requests_per_identifier_per_hour = 3;
    cfg.quotas.max_recovery_requests_per_email_per_window = 2;
    cfg.quotas.recovery_email_window_mins = 60;
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    let alice = state
        .auth
        .register_user_with_email(
            "alice_rec",
            "initial-pass-123",
            "Alice Rec",
            Some("alice@example.com"),
        )
        .await
        .unwrap();
    state
        .auth
        .register_user("bob_no_email", "initial-pass-123", "Bob NoEmail")
        .await
        .unwrap();

    // 1. Anti-enumeration: unknown email, unknown username, and user without email all return identical 200 OK and emit zero outbox rows.
    let mut expected_msg: Option<String> = None;
    for ident in [
        "missing@example.com",
        "no_such_user",
        "bob_no_email",
        "bad-email@",
    ] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/recovery/request")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "identifier": ident })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "ident={ident}");
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp.into_body(), 64 * 1024).await.unwrap()).unwrap();
        assert_eq!(body["status"], "accepted");
        let msg = body["message"].as_str().unwrap().to_string();
        if let Some(ref prev) = expected_msg {
            assert_eq!(&msg, prev);
        } else {
            expected_msg = Some(msg);
        }
    }
    let outbox_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM email_outbox")
        .fetch_one(state.auth.db_pool())
        .await
        .unwrap();
    assert_eq!(outbox_before, 0);

    // 2. Request recovery by email (1st email for alice@example.com) -> emits outbox email with @alice_rec & reset token; DB stores only SHA-256 hash.
    let resp_email = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/recovery/request")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "identifier": "ALICE@EXAMPLE.COM" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_email.status(), StatusCode::OK);
    let body_email: serde_json::Value =
        serde_json::from_slice(&to_bytes(resp_email.into_body(), 64 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(
        body_email["message"].as_str().unwrap(),
        expected_msg.as_deref().unwrap()
    );

    let outbox_row = sqlx::query(
        "SELECT to_email, subject, body_text FROM email_outbox ORDER BY rowid DESC LIMIT 1",
    )
    .fetch_one(state.auth.db_pool())
    .await
    .unwrap();
    let to_email: String = outbox_row.try_get("to_email").unwrap();
    let subject: String = outbox_row.try_get("subject").unwrap();
    let body_text: String = outbox_row.try_get("body_text").unwrap();
    assert_eq!(to_email, "alice@example.com");
    assert!(subject.contains("Account Recovery"));
    assert!(body_text.contains("@alice_rec"));
    let tok_start = body_text.find("satya_rst_").expect("missing reset token");
    let raw_token_1: String = body_text[tok_start..]
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();
    assert!(body_text.contains(&format!("?reset_token={raw_token_1}")));

    let stored_hash: String = sqlx::query_scalar(
        "SELECT token_hash FROM password_reset_tokens WHERE user_id = ?1 AND used_at_us IS NULL",
    )
    .bind(&alice.user_id)
    .fetch_one(state.auth.db_pool())
    .await
    .unwrap();
    assert_ne!(stored_hash, raw_token_1);
    assert_eq!(
        stored_hash,
        hex::encode(crate::characters::sha256_digest(raw_token_1.as_bytes()))
    );

    // 3. Request recovery by @username (2nd email for alice@example.com, hits per-email cap=2) -> invalidates token_1 and issues raw_token_2.
    let resp_uname = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/recovery/request")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "identifier": "@alice_rec" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_uname.status(), StatusCode::OK);
    let body_text_2: String =
        sqlx::query_scalar("SELECT body_text FROM email_outbox ORDER BY rowid DESC LIMIT 1")
            .fetch_one(state.auth.db_pool())
            .await
            .unwrap();
    let tok_start_2 = body_text_2.find("satya_rst_").unwrap();
    let raw_token_2: String = body_text_2[tok_start_2..]
        .split_whitespace()
        .next()
        .unwrap()
        .to_string();
    assert_ne!(raw_token_1, raw_token_2);

    // 3b. Persistent per-email window cap (max_recovery_requests_per_email_per_window = 2):
    // A 3rd request via a different alias ("alice_rec") still returns uniform 200 OK,
    // does NOT queue a 3rd email, and does NOT invalidate raw_token_2 (DoS protection).
    let resp_email_cap = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/recovery/request")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "identifier": "alice_rec" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_email_cap.status(), StatusCode::OK);
    let outbox_count_after_cap: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM email_outbox")
        .fetch_one(state.auth.db_pool())
        .await
        .unwrap();
    assert_eq!(outbox_count_after_cap, 2);

    // Superseded token_1 must fail.
    let fail_superseded = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/recovery/confirm")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "token": raw_token_1,
                        "new_password": "new-strong-pass-1"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fail_superseded.status(), StatusCode::BAD_REQUEST);

    // 4. Weak password fails 400 WITHOUT consuming raw_token_2.
    let weak_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/recovery/confirm")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "token": raw_token_2,
                        "new_password": "short"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(weak_resp.status(), StatusCode::BAD_REQUEST);

    // 5. Valid password reset with raw_token_2 (still intact after email cap block!) succeeds,
    // returns username, revokes prior bearer token, invalidates old password, and enables new login.
    let ok_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/recovery/confirm")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "token": raw_token_2,
                        "new_password": "new-strong-pass-99"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok_resp.status(), StatusCode::OK);
    let ok_body: serde_json::Value =
        serde_json::from_slice(&to_bytes(ok_resp.into_body(), 64 * 1024).await.unwrap()).unwrap();
    assert_eq!(ok_body["status"], "reset");
    assert_eq!(ok_body["username"], "alice_rec");

    // Prior bearer token must now be revoked.
    let me_after = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
                .header("Authorization", format!("Bearer {}", alice.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me_after.status(), StatusCode::UNAUTHORIZED);

    // Old password fails; new password succeeds.
    assert!(
        state
            .auth
            .authenticate_user("alice_rec", "initial-pass-123")
            .await
            .is_err()
    );
    assert!(
        state
            .auth
            .authenticate_user("alice_rec", "new-strong-pass-99")
            .await
            .is_ok()
    );

    // Replaying consumed raw_token_2 fails.
    let replay_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/recovery/confirm")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "token": raw_token_2,
                        "new_password": "another-pass-99"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(replay_resp.status(), StatusCode::BAD_REQUEST);

    // 6. Rate limiting on identifier spray (2 already sent for alice_rec; 3rd succeeds, 4th returns 429).
    let req3 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/recovery/request")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "identifier": "alice_rec" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(req3.status(), StatusCode::OK);
    let req4 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/recovery/request")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "identifier": "alice_rec" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(req4.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(req4.headers().get("retry-after").is_some());

    // 7. Verify dedicated `account-security` Iggy topic recorded and flushed events with zero plaintext secrets/emails/tokens.
    let metrics = state.storage.metrics_snapshot().await;
    let sec_metric = metrics
        .iter()
        .find(|m| m.topic == satya_schema::topics::ACCOUNT_SECURITY)
        .expect("account-security topic missing from metrics");
    assert!(
        sec_metric.total_messages_ingested >= 10,
        "expected >= 10 account-security events, got {}",
        sec_metric.total_messages_ingested
    );
    assert!(sec_metric.flushed_batches_count >= 1);

    let sec_topic_dir = temp_dir.join("data").join("topics").join("account-security");
    assert!(sec_topic_dir.exists(), "account-security topic directory missing");
    let mut stack = vec![sec_topic_dir];
    let mut inspected_files = 0usize;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let bytes = std::fs::read(&path).unwrap();
                let lossy = String::from_utf8_lossy(&bytes);
                assert!(
                    !lossy.contains("satya_rst_"),
                    "plaintext reset token leaked into {}",
                    path.display()
                );
                assert!(
                    !lossy.contains("alice@example.com"),
                    "plaintext email leaked into {}",
                    path.display()
                );
                assert!(
                    !lossy.contains("new-strong-pass-99"),
                    "plaintext password leaked into {}",
                    path.display()
                );
                inspected_files += 1;
            }
        }
    }
    assert!(inspected_files >= 1, "expected flushed .avro files under account-security");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[allow(dead_code)]
fn _keep_schema_import(_s: Option<SessionHistorySnapshot>, _c: Option<AiGmProviderConfig>) {}
