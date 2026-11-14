//! Integration tests for N/S/E/W map continuation stitching and multi-floor tactical map
//! switching & cross-floor token movement.

use super::*;
use satya_schema::{GmMapUpdateRequest, SessionMode, SessionRole};
use satya_storage::StorageConfig;
use uuid::Uuid;

#[tokio::test]
async fn test_map_continuation_permissions_and_stitched_coordinates() {
    let mut cfg = StorageConfig::default();
    let temp_dir = std::env::temp_dir().join(format!("satya_sig_map_cont_{}", Uuid::now_v7()));
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir
        .join("users_map_cont.db")
        .to_string_lossy()
        .to_string();

    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    let gm_auth = state
        .auth
        .authenticate_user("gm_sean", "satya-truth-2026")
        .await
        .unwrap();
    let gm_user = state.auth.verify_auth_token(&gm_auth.auth_token).await.unwrap();
    let _ = state
        .auth
        .create_session(
            &gm_user,
            "ep-map-cont-gm",
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
    let p1_user = state.auth.verify_auth_token(&p1_auth.auth_token).await.unwrap();
    let _ = state
        .auth
        .join_session(
            &p1_user,
            "ep-map-cont-gm",
            "room-pass",
            SessionRole::Player,
            Some("Sara (Valkyrie)".to_string()),
        )
        .await
        .unwrap();

    // 1. GM generates a 20x14 Docking Bay base sector
    let gen_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-cont-gm/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                preset: "docking_bay".to_string(),
                title: Some("Aethelgard Bay".to_string()),
                grid_width: 20,
                grid_height: 14,
                cell_size_cm: 100,
                seed: Some(91),
                populate_roster_tokens: true,
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let gen_resp = tower::ServiceExt::oneshot(app.clone(), gen_req)
        .await
        .unwrap();
    assert_eq!(gen_resp.status(), axum::http::StatusCode::OK);
    let base_map: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(gen_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    let orig_token_cm = base_map
        .tokens
        .iter()
        .find(|t| t.entity_id == p1_auth.user_id)
        .map(|t| base_map.token_pos_cm(t))
        .expect("Sara token present");

    // 2. Non-GM player cannot trigger procedural map continuation in a Human-GM session (403)
    let p_cont_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-cont-gm/map")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                continue_direction: Some(satya_schema::MapContinuationDir::East),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let p_cont_resp = tower::ServiceExt::oneshot(app.clone(), p_cont_req)
        .await
        .unwrap();
    assert_eq!(p_cont_resp.status(), axum::http::StatusCode::FORBIDDEN);

    // 3. Human GM continues East -> 40x14 stitched arena keeping existing token & adding Eastern sector
    let gm_cont_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-cont-gm/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                continue_direction: Some(satya_schema::MapContinuationDir::East),
                seed: Some(202),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let gm_cont_resp = tower::ServiceExt::oneshot(app.clone(), gm_cont_req)
        .await
        .unwrap();
    assert_eq!(gm_cont_resp.status(), axum::http::StatusCode::OK);
    let east_map: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(gm_cont_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(east_map.grid_width, 40);
    assert_eq!(east_map.grid_height, 14);
    assert_eq!(
        east_map
            .tokens
            .iter()
            .find(|t| t.entity_id == p1_auth.user_id)
            .map(|t| east_map.token_pos_cm(t)),
        Some(orig_token_cm)
    );
    assert!(east_map.wall_edges.iter().any(|e| e.x >= 20));

    // 4. Solo player in an AI-GM session (`!has_human_gm`) CAN continue North -> 20x28 with +1400cm Y shift
    let _ = state
        .auth
        .create_session(
            &p1_user,
            "ep-map-cont-solo",
            "room-pass",
            SessionMode::Ttrpg,
            SessionRole::Player,
            Some("Sara (Solo)".to_string()),
            true,
        )
        .await
        .unwrap();
    let solo_gen_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-cont-solo/map")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                preset: "alley_corridor".to_string(),
                grid_width: 20,
                grid_height: 14,
                cell_size_cm: 100,
                seed: Some(55),
                populate_roster_tokens: true,
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let solo_gen_resp = tower::ServiceExt::oneshot(app.clone(), solo_gen_req)
        .await
        .unwrap();
    assert_eq!(solo_gen_resp.status(), axum::http::StatusCode::OK);
    let solo_base: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(solo_gen_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    let solo_orig_cm = solo_base
        .tokens
        .iter()
        .find(|t| t.entity_id == p1_auth.user_id)
        .map(|t| solo_base.token_pos_cm(t))
        .expect("Solo Sara token present");

    let solo_north_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-cont-solo/map")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                continue_direction: Some(satya_schema::MapContinuationDir::North),
                seed: Some(56),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let solo_north_resp = tower::ServiceExt::oneshot(app.clone(), solo_north_req)
        .await
        .unwrap();
    assert_eq!(solo_north_resp.status(), axum::http::StatusCode::OK);
    let solo_north: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(solo_north_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(solo_north.grid_width, 20);
    assert_eq!(solo_north.grid_height, 28);
    assert_eq!(
        solo_north
            .tokens
            .iter()
            .find(|t| t.entity_id == p1_auth.user_id)
            .map(|t| solo_north.token_pos_cm(t)),
        Some((solo_orig_cm.0, solo_orig_cm.1 + 1400))
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn multi_floor_mansion_generation_floor_switch_and_cross_floor_token_move() {
    let mut cfg = StorageConfig::default();
    let temp_dir = std::env::temp_dir().join(format!("satya_sig_map_floor_{}", Uuid::now_v7()));
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir
        .join("users_map_floor.db")
        .to_string_lossy()
        .to_string();

    let state = SignalingAppState::new(cfg, &temp_dir).await.unwrap();
    let app = build_router(state.clone());

    let gm_auth = state
        .auth
        .authenticate_user("gm_sean", "satya-truth-2026")
        .await
        .unwrap();
    let gm_user = state.auth.verify_auth_token(&gm_auth.auth_token).await.unwrap();
    let _ = state
        .auth
        .create_session(
            &gm_user,
            "ep-map-floors",
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
    let p1_user = state.auth.verify_auth_token(&p1_auth.auth_token).await.unwrap();
    let _ = state
        .auth
        .join_session(
            &p1_user,
            "ep-map-floors",
            "room-pass",
            SessionRole::Player,
            Some("Sara".to_string()),
        )
        .await
        .unwrap();

    // 1. Generate a multi-floor Luxury Mansion on 1F (`active_floor == 0`).
    let gen_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-floors/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                preset: "luxury_mansion".to_string(),
                grid_width: 24,
                grid_height: 18,
                cell_size_cm: 100,
                seed: Some(42),
                populate_roster_tokens: true,
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let gen_resp = tower::ServiceExt::oneshot(app.clone(), gen_req)
        .await
        .unwrap();
    assert_eq!(gen_resp.status(), axum::http::StatusCode::OK);
    let mansion_map: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(gen_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(mansion_map.floors.len() >= 2, "Luxury mansion must have >= 2 floors");
    assert_eq!(mansion_map.active_floor, 0);

    // 2. Switch active floor to 2F (`floor_index == 1`).
    let switch_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-floors/map")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::GenerateSessionMapRequest {
                set_active_floor: Some(1),
                ..Default::default()
            })
            .unwrap(),
        ))
        .unwrap();
    let switch_resp = tower::ServiceExt::oneshot(app.clone(), switch_req)
        .await
        .unwrap();
    assert_eq!(switch_resp.status(), axum::http::StatusCode::OK);
    let f2_map: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(switch_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(f2_map.active_floor, 1);
    let f2_entry = f2_map
        .floors
        .iter()
        .find(|f| f.floor_index == 1)
        .expect("2F floor entry present");
    assert_eq!(f2_map.wall_edges, f2_entry.wall_edges);

    // 3. Move player token across floors to 2F (`to_floor: Some(1)`) at the stairwell center.
    let stair = f2_map
        .props
        .iter()
        .find(|p| p.kind == satya_schema::PropKind::Stairwell)
        .expect("2F must contain a Stairwell prop");
    let (sx_cm, sy_cm) = stair.center_cm();
    let move_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-floors/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: sx_cm / 100,
                to_y: sy_cm / 100,
                to_x_cm: Some(sx_cm),
                to_y_cm: Some(sy_cm),
                to_floor: Some(1),
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
    let move_resp = tower::ServiceExt::oneshot(app.clone(), move_req)
        .await
        .unwrap();
    assert_eq!(move_resp.status(), axum::http::StatusCode::OK);
    let moved_map: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(move_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(moved_map.token_floor(&p1_auth.user_id), 1);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
