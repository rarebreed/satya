//! Integration tests for GM-owned Campaigns, Privacy-Preserving Directory Search,
//! Request-to-Join (live WebSocket + offline email + user-bound approval grants),
//! GM/Admin RBAC, and anti-bot quotas & sliding-window rate limits.

use crate::build_router;
use crate::state::SignalingAppState;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use satya_schema::{
    AccountQuotaUsage, AccountWsPacket, CampaignListResponse, CampaignSummary,
    DirectoryMembershipStatus, DirectorySearchResponse, JoinRequestKind, JoinRequestRecord,
    JoinRequestStatus, JoinRequestsListResponse, SavedSessionsListResponse, SessionEnterResponse,
    SessionMode, SessionRole, topics,
};
use satya_storage::StorageConfig;
use serde_json::json;
use sqlx::Row;
use tokio::sync::mpsc;
use tower::ServiceExt;
use uuid::Uuid;

fn test_storage_config(temp_dir: &std::path::Path) -> StorageConfig {
    let mut cfg = StorageConfig::default();
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir
        .join("users_campaigns_test.db")
        .to_string_lossy()
        .to_string();
    cfg
}

#[tokio::test]
async fn test_gm_rbac_quotas_and_reclamation() {
    let temp_dir = std::env::temp_dir().join(format!("satya_rbac_quota_{}", Uuid::new_v4()));
    let mut cfg = test_storage_config(&temp_dir);
    cfg.quotas.max_campaigns_per_gm = 2;
    cfg.quotas.max_total_sessions_per_gm = 2;
    cfg.quotas.max_sessions_per_campaign = 2;

    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    // 1. Self-registered user defaults to USER, can own campaigns & sessions up to quota,
    //    and remains blocked (403) from admin RBAC routes
    let reg_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "username": "new_bot_user",
                        "password": "password123",
                        "display_name": "Bot Suspect",
                        "email": "bot@example.com"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reg_res.status(), StatusCode::OK);
    let reg_body: serde_json::Value =
        serde_json::from_slice(&to_bytes(reg_res.into_body(), 64 * 1024).await.unwrap()).unwrap();
    let user_tok = reg_body["auth_token"].as_str().unwrap().to_string();
    assert_eq!(reg_body["account_role"], "USER");

    let non_admin_role_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/users/role")
                .header("Authorization", format!("Bearer {user_tok}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "username": "new_bot_user", "account_role": "ADMIN" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(non_admin_role_res.status(), StatusCode::FORBIDDEN);

    // 2. Admin can still manage account roles via POST /api/admin/users/role
    let admin_auth = state
        .auth
        .authenticate_user("gm_sean", "satya-truth-2026")
        .await
        .unwrap();
    let admin_camp_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/campaigns")
                .header("Authorization", format!("Bearer {}", admin_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "title": "Admin Camp", "description": "owner isolation" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(admin_camp_res.status(), StatusCode::OK);
    let admin_camp: CampaignSummary =
        serde_json::from_slice(&to_bytes(admin_camp_res.into_body(), 64 * 1024).await.unwrap())
            .unwrap();

    // Non-owner USER cannot create a session directly inside another user's campaign or delete it
    let hijack_sess_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/create")
                .header("Authorization", format!("Bearer {user_tok}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "session_id": "hijack-room",
                        "session_password": "pw",
                        "session_mode": "TTRPG",
                        "role": "GM",
                        "enable_ai_gm": false,
                        "campaign_id": admin_camp.campaign_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hijack_sess_res.status(), StatusCode::BAD_REQUEST);

    let hijack_del_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/campaigns/{}", admin_camp.campaign_id))
                .header("Authorization", format!("Bearer {user_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hijack_del_res.status(), StatusCode::FORBIDDEN);

    // 3. Regular USER can create up to quota (2 campaigns), 3rd is rejected (429)
    let mut created_camp_ids = Vec::new();
    for title in ["Satyang", "DAISE"] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/campaigns")
                    .header("Authorization", format!("Bearer {user_tok}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({ "title": title, "description": format!("{title} ops") }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let camp: CampaignSummary =
            serde_json::from_slice(&to_bytes(res.into_body(), 64 * 1024).await.unwrap()).unwrap();
        created_camp_ids.push(camp.campaign_id);
    }

    let over_camp_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/campaigns")
                .header("Authorization", format!("Bearer {user_tok}"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "title": "Third Camp" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(over_camp_res.status(), StatusCode::TOO_MANY_REQUESTS);

    // 4. Create 2 sessions under the first campaign; 3rd session hits quota (429)
    for sid in ["satyang-ep1", "satyang-ep2"] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions/create")
                    .header("Authorization", format!("Bearer {user_tok}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "session_id": sid,
                            "session_password": "secret",
                            "session_mode": "TTRPG",
                            "role": "GM",
                            "enable_ai_gm": false,
                            "campaign_id": created_camp_ids[0],
                            "title": format!("Episode {sid}")
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
    let over_sess_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/create")
                .header("Authorization", format!("Bearer {user_tok}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "session_id": "satyang-ep3",
                        "session_password": "secret",
                        "session_mode": "TTRPG",
                        "role": "GM",
                        "enable_ai_gm": false,
                        "campaign_id": created_camp_ids[1]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(over_sess_res.status(), StatusCode::TOO_MANY_REQUESTS);

    // 5. Delete one session and verify quota is reclaimed
    let del_sess = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/sessions/satyang-ep2")
                .header("Authorization", format!("Bearer {user_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_sess.status(), StatusCode::OK);

    let quota_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/quotas")
                .header("Authorization", format!("Bearer {user_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let quota: AccountQuotaUsage =
        serde_json::from_slice(&to_bytes(quota_res.into_body(), 64 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(quota.campaigns_used, 2);
    assert_eq!(quota.sessions_used, 1);

    // Verify QUOTA_REJECTED events logged to session-access topic
    let metrics = state.storage.metrics_snapshot().await;
    let access_m = metrics
        .iter()
        .find(|m| m.topic == topics::SESSION_ACCESS)
        .unwrap();
    assert!(access_m.total_messages_ingested >= 2);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_directory_search_join_requests_offline_email_live_ws_and_passwordless_entry() {
    let temp_dir = std::env::temp_dir().join(format!("satya_dir_join_{}", Uuid::new_v4()));
    let state = SignalingAppState::new(test_storage_config(&temp_dir), &temp_dir)
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
    let marcus_auth = state
        .auth
        .authenticate_user("marcus_recon", "satya-truth-2026")
        .await
        .unwrap();

    // 1. GM creates two campaigns ("Satyang" & "DAISE") and one session in each
    let mut camps = Vec::new();
    for title in ["Satyang", "DAISE"] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/campaigns")
                    .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "title": title,
                            "description": format!("{title} campaign"),
                            "is_discoverable": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let c: CampaignSummary =
            serde_json::from_slice(&to_bytes(res.into_body(), 64 * 1024).await.unwrap()).unwrap();
        camps.push(c);
    }

    for (sid, cid) in [
        ("satyang-airlock", &camps[0].campaign_id),
        ("daise-blackout", &camps[1].campaign_id),
    ] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions/create")
                    .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "session_id": sid,
                            "session_password": "top-secret-password",
                            "session_mode": "TTRPG",
                            "role": "GM",
                            "enable_ai_gm": false,
                            "campaign_id": cid,
                            "title": format!("Session {sid}"),
                            "is_discoverable": true,
                            "allow_join_requests": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    // Verify campaign_tag was unified to the campaign slug on saved_sessions
    let stored_tag: String =
        sqlx::query_scalar("SELECT campaign_tag FROM saved_sessions WHERE session_id = 'satyang-airlock'")
            .fetch_one(state.auth.db_pool())
            .await
            .unwrap();
    assert_eq!(stored_tag, "satyang");

    // 2. Sara searches by GM's exact email AND by @handle; neither response leaks GM's email
    for query in ["gm_sean@satya.local", "@gm_sean"] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/directory/search?q={query}"))
                    .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = to_bytes(res.into_body(), 128 * 1024).await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("gm_sean@satya.local"),
            "GM email must never be leaked in directory search JSON"
        );
        let parsed: DirectorySearchResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.gms.len(), 1);
        assert_eq!(parsed.gms[0].username, "gm_sean");
        assert_eq!(parsed.gms[0].campaigns.len(), 2);
    }

    // 3. While GM is OFFLINE (no /ws/notifications or /ws/session socket), Sara requests to join -> triggers email_outbox
    let jr_offline_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/satyang-airlock/join-requests")
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "requested_role": "PLAYER", "message": "Ready to breach!" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(jr_offline_res.status(), StatusCode::OK);
    let jr_sara: JoinRequestRecord = serde_json::from_slice(
        &to_bytes(jr_offline_res.into_body(), 64 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(jr_sara.status, JoinRequestStatus::Pending);
    assert!(jr_sara.emailed_offline);
    assert!(jr_sara.approval_token.is_none());

    let outbox_row = sqlx::query(
        "SELECT to_email, subject, status FROM email_outbox WHERE request_id = ?1",
    )
    .bind(&jr_sara.request_id)
    .fetch_one(state.auth.db_pool())
    .await
    .unwrap();
    assert_eq!(
        outbox_row.try_get::<String, _>("to_email").unwrap(),
        "gm_sean@satya.local"
    );
    assert_eq!(
        outbox_row.try_get::<String, _>("status").unwrap(),
        "QUEUED_LOCAL"
    );

    // 4. Register an online /ws/notifications channel for GM, then Marcus submits a request -> live WS push, NO email!
    let (gm_tx, mut gm_rx) = mpsc::unbounded_channel::<String>();
    state
        .account_sockets
        .write()
        .await
        .entry(gm_auth.user_id.clone())
        .or_default()
        .insert(9001, gm_tx);

    let jr_online_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/daise-blackout/join-requests")
                .header("Authorization", format!("Bearer {}", marcus_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "requested_role": "PLAYER", "message": "DAISE recon ready" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(jr_online_res.status(), StatusCode::OK);
    let jr_marcus: JoinRequestRecord = serde_json::from_slice(
        &to_bytes(jr_online_res.into_body(), 64 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(!jr_marcus.emailed_offline);

    let pushed_pkt_raw = gm_rx.try_recv().expect("GM should receive live WS notification");
    let pushed_pkt: AccountWsPacket = serde_json::from_str(&pushed_pkt_raw).unwrap();
    match pushed_pkt {
        AccountWsPacket::JoinRequestReceived { request } => {
            assert_eq!(request.request_id, jr_marcus.request_id);
        }
        other => panic!("Expected JoinRequestReceived, got {other:?}"),
    }

    // 5. GM lists incoming requests and approves Sara's request
    let incoming_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/join-requests/incoming")
                .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let incoming: JoinRequestsListResponse = serde_json::from_slice(
        &to_bytes(incoming_res.into_body(), 64 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(incoming.requests.len(), 2);

    let decide_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/join-requests/{}/decide", jr_sara.request_id))
                .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "approve": true, "decision_note": "Welcome aboard" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(decide_res.status(), StatusCode::OK);

    // 6. Sara fetches /api/join-requests/mine (or directory search) and sees her user-bound approval_token
    let mine_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/join-requests/mine")
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let mine: JoinRequestsListResponse =
        serde_json::from_slice(&to_bytes(mine_res.into_body(), 64 * 1024).await.unwrap()).unwrap();
    assert_eq!(mine.requests[0].status, JoinRequestStatus::Approved);
    let approval_token = mine.requests[0].approval_token.clone().unwrap();

    // Directory search also exposes ApprovedReady for Sara specifically
    let dir_after = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/directory/search?q=@gm_sean")
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let dir_parsed: DirectorySearchResponse = serde_json::from_slice(
        &to_bytes(dir_after.into_body(), 128 * 1024).await.unwrap(),
    )
    .unwrap();
    let satyang_sess = dir_parsed.gms[0]
        .campaigns
        .iter()
        .find(|c| c.title == "Satyang")
        .unwrap()
        .sessions[0]
        .clone();
    assert_eq!(
        satyang_sess.caller_membership_status,
        DirectoryMembershipStatus::ApprovedReady
    );
    assert_eq!(satyang_sess.approval_token.as_deref(), Some(approval_token.as_str()));

    // 7. Wrong user (Marcus) CANNOT steal Sara's approval_token!
    let stolen_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/join")
                .header("Authorization", format!("Bearer {}", marcus_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "session_id": "satyang-airlock",
                        "approval_token": approval_token,
                        "role": "PLAYER"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stolen_res.status(), StatusCode::FORBIDDEN);

    // 8. Sara joins WITHOUT session_password using her approval_token!
    let join_ok_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/join")
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "session_id": "satyang-airlock",
                        "approval_token": approval_token,
                        "role": "PLAYER"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(join_ok_res.status(), StatusCode::OK);
    let entered: SessionEnterResponse = serde_json::from_slice(
        &to_bytes(join_ok_res.into_body(), 128 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(entered.session_id, "satyang-airlock");
    assert_eq!(entered.session_mode, SessionMode::Ttrpg);
    assert_eq!(entered.self_participant.role, SessionRole::Player);

    // 9. Reusing the consumed approval_token is rejected (single-use)
    let replay_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/join")
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "session_id": "satyang-airlock",
                        "approval_token": approval_token,
                        "role": "PLAYER"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(replay_res.status(), StatusCode::FORBIDDEN);

    // 10. Verify both Apache Iggy topics recorded all lifecycle events
    let metrics = state.storage.metrics_snapshot().await;
    let dir_m = metrics
        .iter()
        .find(|m| m.topic == topics::CAMPAIGN_DIRECTORY)
        .unwrap();
    let acc_m = metrics
        .iter()
        .find(|m| m.topic == topics::SESSION_ACCESS)
        .unwrap();
    assert!(dir_m.total_messages_ingested >= 5);
    assert!(acc_m.total_messages_ingested >= 5);

    // Clean up campaigns with ?force=true
    let my_camps_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/campaigns")
                .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let my_camps: CampaignListResponse = serde_json::from_slice(
        &to_bytes(my_camps_res.into_body(), 64 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(my_camps.campaigns.len(), 2);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_request_new_session_in_other_users_campaign_and_approve() {
    let temp_dir = std::env::temp_dir().join(format!("satya_camp_sess_req_{}", Uuid::new_v4()));
    let state = SignalingAppState::new(test_storage_config(&temp_dir), &temp_dir)
        .await
        .unwrap();
    let app = build_router(state.clone());

    let sara_auth = state
        .auth
        .authenticate_user("sara_valkyrie", "satya-truth-2026")
        .await
        .unwrap();
    let marcus_auth = state
        .auth
        .authenticate_user("marcus_recon", "satya-truth-2026")
        .await
        .unwrap();

    // 1. Standard USER account (Sara) creates a discoverable campaign
    let camp_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/campaigns")
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "title": "Valkyrie Ops",
                        "description": "Orbital insertion",
                        "is_discoverable": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(camp_res.status(), StatusCode::OK);
    let camp: CampaignSummary =
        serde_json::from_slice(&to_bytes(camp_res.into_body(), 64 * 1024).await.unwrap()).unwrap();

    // 2. Owner cannot request a new session in their own campaign (400)
    let self_req = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/campaigns/{}/session-requests", camp.campaign_id))
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "session_title": "Self Request",
                        "session_mode": "TTRPG",
                        "requested_role": "PLAYER"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(self_req.status(), StatusCode::BAD_REQUEST);

    // 3. Marcus searches for Sara via directory and requests a new session in her campaign (Row 4)
    let search_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/directory/search?q=@sara_valkyrie")
                .header("Authorization", format!("Bearer {}", marcus_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_res.status(), StatusCode::OK);
    let dir: DirectorySearchResponse =
        serde_json::from_slice(&to_bytes(search_res.into_body(), 64 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(dir.gms.len(), 1);
    assert_eq!(dir.gms[0].campaigns[0].campaign_id.as_deref(), Some(camp.campaign_id.as_str()));

    let req_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/campaigns/{}/session-requests", camp.campaign_id))
                .header("Authorization", format!("Bearer {}", marcus_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "session_title": "Night Watch",
                        "session_mode": "TTRPG",
                        "enable_ai_gm": true,
                        "requested_role": "PLAYER",
                        "message": "Can we run a recon session?"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(req_res.status(), StatusCode::OK);
    let created_req: JoinRequestRecord =
        serde_json::from_slice(&to_bytes(req_res.into_body(), 64 * 1024).await.unwrap()).unwrap();
    assert_eq!(created_req.request_kind, JoinRequestKind::CreateSession);
    assert_eq!(created_req.session_title.as_deref(), Some("Night Watch"));
    assert_eq!(created_req.proposed_session_mode, Some(SessionMode::Ttrpg));
    assert!(created_req.proposed_enable_ai_gm);
    assert_eq!(created_req.status, JoinRequestStatus::Pending);

    // 4. Sara sees incoming CREATE_SESSION request and approves it
    let incoming_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/join-requests/incoming")
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let incoming: JoinRequestsListResponse =
        serde_json::from_slice(&to_bytes(incoming_res.into_body(), 64 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(incoming.requests.len(), 1);
    assert_eq!(incoming.requests[0].request_kind, JoinRequestKind::CreateSession);
    assert_eq!(incoming.requests[0].session_title.as_deref(), Some("Night Watch"));

    let approve_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/join-requests/{}/decide", created_req.request_id))
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({ "approve": true }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(approve_res.status(), StatusCode::OK);

    // 5. The new session now exists in Sara's saved sessions under Valkyrie Ops
    let saved_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/sessions/saved")
                .header("Authorization", format!("Bearer {}", sara_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let saved: SavedSessionsListResponse =
        serde_json::from_slice(&to_bytes(saved_res.into_body(), 64 * 1024).await.unwrap())
            .unwrap();
    let created_sess = saved
        .sessions
        .iter()
        .find(|s| s.session_id == created_req.session_id)
        .expect("approved session should be provisioned in saved_sessions");
    assert_eq!(created_sess.campaign_id.as_deref(), Some(camp.campaign_id.as_str()));
    assert_eq!(created_sess.title.as_deref(), Some("Night Watch"));
    assert!(created_sess.has_ai_gm);

    // 6. Marcus fetches /api/join-requests/mine, gets approval_token, and enters passwordlessly
    let mine_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/join-requests/mine")
                .header("Authorization", format!("Bearer {}", marcus_auth.auth_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let mine: JoinRequestsListResponse =
        serde_json::from_slice(&to_bytes(mine_res.into_body(), 64 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(mine.requests[0].status, JoinRequestStatus::Approved);
    let tok = mine.requests[0].approval_token.clone().unwrap();

    let join_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions/join")
                .header("Authorization", format!("Bearer {}", marcus_auth.auth_token))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "session_id": created_req.session_id,
                        "approval_token": tok,
                        "role": "PLAYER"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(join_res.status(), StatusCode::OK);
    let entered: SessionEnterResponse =
        serde_json::from_slice(&to_bytes(join_res.into_body(), 128 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(entered.session_id, created_req.session_id);
    assert_eq!(entered.self_participant.role, SessionRole::Player);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
