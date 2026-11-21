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
    let gm_user = state
        .auth
        .verify_auth_token(&gm_auth.auth_token)
        .await
        .unwrap();
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
    let p1_user = state
        .auth
        .verify_auth_token(&p1_auth.auth_token)
        .await
        .unwrap();
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
    let gm_user = state
        .auth
        .verify_auth_token(&gm_auth.auth_token)
        .await
        .unwrap();
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
    let p1_user = state
        .auth
        .verify_auth_token(&p1_auth.auth_token)
        .await
        .unwrap();
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
    assert!(
        mansion_map.floors.len() >= 2,
        "Luxury mansion must have >= 2 floors"
    );
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

    // 3. Position player at the 1F stairwell (via GM move), then climb to 2F (`to_floor: Some(1)`).
    let f1_stair_cm = mansion_map
        .floors
        .iter()
        .find(|f| f.floor_index == 0)
        .and_then(|f| f.props.iter().find(|p| p.kind == satya_schema::PropKind::Stairwell))
        .map(|p| p.center_cm())
        .unwrap_or_else(|| {
            f2_map
                .props
                .iter()
                .find(|p| p.kind == satya_schema::PropKind::Stairwell)
                .expect("Stairwell prop")
                .center_cm()
        });
    let stage_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-floors/map/move")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: f1_stair_cm.0 / 100,
                to_y: f1_stair_cm.1 / 100,
                to_x_cm: Some(f1_stair_cm.0),
                to_y_cm: Some(f1_stair_cm.1),
                to_floor: Some(0),
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
    let stage_resp = tower::ServiceExt::oneshot(app.clone(), stage_req)
        .await
        .unwrap();
    assert_eq!(stage_resp.status(), axum::http::StatusCode::OK);

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

#[tokio::test]
async fn test_cross_floor_move_requires_vertical_portal_proximity() {
    let mut cfg = StorageConfig::default();
    let temp_dir = std::env::temp_dir().join(format!("satya_sig_map_portal_{}", Uuid::now_v7()));
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir
        .join("users_map_portal.db")
        .to_string_lossy()
        .to_string();
    cfg.ai_gm.service_url = "http://127.0.0.1:59999".to_string();

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
            "ep-map-portal",
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
            "ep-map-portal",
            "room-pass",
            SessionRole::Player,
            Some("Sara".to_string()),
        )
        .await
        .unwrap();

    let gen_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-portal/map")
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

    let portals_cm: Vec<(i32, i32)> = mansion_map
        .floors
        .iter()
        .flat_map(|f| f.props.iter())
        .chain(mansion_map.props.iter())
        .filter(|p| p.kind.is_vertical_portal())
        .map(|p| p.center_cm())
        .collect();
    let &(stair_x_cm, stair_y_cm) = portals_cm.first().expect("mansion has vertical portal");

    // Pick a walkable cell on both 1F and 2F more than 3.5m away from any vertical portal
    let mut f2_view = mansion_map.clone();
    f2_view.switch_active_floor(1);
    let mut far_cm = None;
    'search: for gy in 1..(mansion_map.grid_height - 1) {
        for gx in 1..(mansion_map.grid_width - 1) {
            let cx = gx * 100 + 50;
            let cy = gy * 100 + 50;
            let far_from_portals = portals_cm
                .iter()
                .all(|&(px, py)| ((cx - px) as f32).hypot((cy - py) as f32) > 350.0);
            if far_from_portals
                && satya_maps::resolve_token_destination_cm(&mansion_map, &p1_auth.user_id, cx, cy)
                    .is_ok()
                && satya_maps::resolve_token_destination_cm(&f2_view, &p1_auth.user_id, cx, cy)
                    .is_ok()
            {
                far_cm = Some((cx, cy));
                break 'search;
            }
        }
    }
    let (far_x_cm, far_y_cm) = far_cm.expect("found walkable far cell away from stairwells");

    // First position player on Floor 0 at the far cell
    let prep_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-portal/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: far_x_cm / 100,
                to_y: far_y_cm / 100,
                to_x_cm: Some(far_x_cm),
                to_y_cm: Some(far_y_cm),
                to_floor: Some(0),
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
    let prep_resp = tower::ServiceExt::oneshot(app.clone(), prep_req)
        .await
        .unwrap();
    assert_eq!(prep_resp.status(), axum::http::StatusCode::OK);

    // Switching floor far from any stairwell/elevator is rejected (400 Bad Request)
    let far_jump_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-portal/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: far_x_cm / 100,
                to_y: far_y_cm / 100,
                to_x_cm: Some(far_x_cm),
                to_y_cm: Some(far_y_cm),
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
    let far_jump_resp = tower::ServiceExt::oneshot(app.clone(), far_jump_req)
        .await
        .unwrap();
    assert_eq!(far_jump_resp.status(), axum::http::StatusCode::BAD_REQUEST);

    // Jumping from far cell on 1F onto stairwell center on 2F must also be rejected (requires 1F portal origin too)
    let f1_portal_cm = mansion_map
        .floors
        .iter()
        .find(|f| f.floor_index == 0)
        .and_then(|f| f.props.iter().find(|p| p.kind.is_vertical_portal()))
        .map(|p| p.center_cm())
        .unwrap_or((stair_x_cm, stair_y_cm));
    let f2_portal_cm = mansion_map
        .floors
        .iter()
        .find(|f| f.floor_index == 1)
        .and_then(|f| f.props.iter().find(|p| p.kind.is_vertical_portal()))
        .map(|p| p.center_cm())
        .unwrap_or((stair_x_cm, stair_y_cm));

    let far_to_stair_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-portal/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: f2_portal_cm.0 / 100,
                to_y: f2_portal_cm.1 / 100,
                to_x_cm: Some(f2_portal_cm.0),
                to_y_cm: Some(f2_portal_cm.1),
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
    let far_to_stair_resp = tower::ServiceExt::oneshot(app.clone(), far_to_stair_req)
        .await
        .unwrap();
    assert_eq!(far_to_stair_resp.status(), axum::http::StatusCode::BAD_REQUEST);

    // Stage player onto the 1F stairwell (<= 2.5m), then climbing onto 2F stairwell succeeds
    let stage_portal_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-portal/map/move")
        .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: f1_portal_cm.0 / 100,
                to_y: f1_portal_cm.1 / 100,
                to_x_cm: Some(f1_portal_cm.0),
                to_y_cm: Some(f1_portal_cm.1),
                to_floor: Some(0),
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
    let stage_portal_resp = tower::ServiceExt::oneshot(app.clone(), stage_portal_req)
        .await
        .unwrap();
    assert_eq!(stage_portal_resp.status(), axum::http::StatusCode::OK);

    let near_jump_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/sessions/ep-map-portal/map/move")
        .header("Authorization", format!("Bearer {}", p1_auth.auth_token))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&satya_schema::MoveMapTokenRequest {
                entity_id: p1_auth.user_id.clone(),
                label: None,
                to_x: f2_portal_cm.0 / 100,
                to_y: f2_portal_cm.1 / 100,
                to_x_cm: Some(f2_portal_cm.0),
                to_y_cm: Some(f2_portal_cm.1),
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
    let near_jump_resp = tower::ServiceExt::oneshot(app.clone(), near_jump_req)
        .await
        .unwrap();
    assert_eq!(near_jump_resp.status(), axum::http::StatusCode::OK);
    let moved: GmMapUpdateRequest = serde_json::from_slice(
        &axum::body::to_bytes(near_jump_resp.into_body(), 256 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(moved.token_floor(&p1_auth.user_id), 1);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn wave2_portal_strictness_notes_stripping_and_player_ai_door() {
    use satya_schema::{
        AiPlayerConfig, EdgeSide, MapFloorLevel, MapWallEdge, MaterialClass, PlayerAiDoorRequest,
        TokenPlacement, TokenVisualSpec, WallEdgeKind,
    };

    let mut cfg = StorageConfig::default();
    let temp_dir = std::env::temp_dir().join(format!("satya_wave2_floors_{}", Uuid::now_v7()));
    for topic_cfg in cfg.topics.values_mut() {
        topic_cfg.local_path = temp_dir
            .join(&topic_cfg.local_path)
            .to_string_lossy()
            .to_string();
    }
    cfg.auth.database_url = temp_dir.join("floors.db").to_string_lossy().to_string();
    cfg.ai_gm.fail_fast_when_offline = false;

    let state = SignalingAppState::new(cfg.clone(), &temp_dir)
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
    let gm_user = state.auth.verify_auth_token(&gm_auth.auth_token).await.unwrap();
    let sara_user = state
        .auth
        .verify_auth_token(&sara_auth.auth_token)
        .await
        .unwrap();

    state
        .auth
        .create_session(
            &gm_user,
            "w2-floor-door",
            "pass",
            SessionMode::Ttrpg,
            SessionRole::Gm,
            None,
            true,
        )
        .await
        .unwrap();
    state
        .auth
        .join_session(&sara_user, "w2-floor-door", "pass", SessionRole::Player, None)
        .await
        .unwrap();

    // Configure an AI Player
    let up_resp = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/sessions/w2-floor-door/ai-players")
            .header("Authorization", format!("Bearer {}", gm_auth.auth_token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&AiPlayerConfig {
                    user_id: Some("ai-player-1".to_string()),
                    display_name: Some("Vanguard".to_string()),
                    character_id: None,
                    persona_prompt: None,
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
    assert_eq!(up_resp.status(), axum::http::StatusCode::OK);

    // Build a 2-floor map where floor 0 stairwell is at (2, 2), floor 1 stairwell is at (2, 2),
    // and an unlocked DoorClosed sits at (3, 3, EdgeSide::North) next to ai-player-1 at (3, 3)
    // plus a distant DoorClosed at (10, 10, EdgeSide::North).
    let mut map = satya_maps::tactical_map::build_procedural_map_request(
        "m-door",
        satya_maps::tactical_map::SectorPreset::OfficeFloor,
        None,
        14,
        14,
        7,
    );
    map.notes = Some("SECRET_GM_NOTES_SHOULD_BE_STRIPPED".to_string());
    let stair = satya_maps::map_dressing::make_prop(
        satya_schema::PropKind::Stairwell,
        2,
        2,
        0,
        None,
        0,
    );
    let f0_edges = vec![
        MapWallEdge {
            x: 3,
            y: 3,
            side: EdgeSide::North,
            kind: WallEdgeKind::DoorClosed,
            material: MaterialClass::SolidWood,
        },
        MapWallEdge {
            x: 10,
            y: 10,
            side: EdgeSide::North,
            kind: WallEdgeKind::DoorClosed,
            material: MaterialClass::SolidWood,
        },
    ];
    map.wall_edges = f0_edges.clone();
    map.props = vec![stair];
    map.floors = vec![
        MapFloorLevel {
            floor_index: 0,
            label: "1F".to_string(),
            elevation_cm: 0,
            ground_floor_override: None,
            ground_tint_rgb: None,
            wall_edges: f0_edges,
            props: vec![stair],
            surfaces: Vec::new(),
        },
        MapFloorLevel {
            floor_index: 1,
            label: "2F".to_string(),
            elevation_cm: 300,
            ground_floor_override: None,
            ground_tint_rgb: None,
            wall_edges: Vec::new(),
            props: vec![stair],
            surfaces: Vec::new(),
        },
    ];
    map.active_floor = 0;
    map.tokens = vec![TokenPlacement {
        entity_id: "ai-player-1".to_string(),
        label: "Vanguard".to_string(),
        x: 3,
        y: 3,
        visibility: satya_schema::MessageVisibility::Public,
    }];
    map.token_visuals = vec![TokenVisualSpec {
        entity_id: "ai-player-1".to_string(),
        model_preset: "recon_mk4".to_string(),
        color_hex: "#22d3ee".to_string(),
        facing_deg: 0,
        elevation_cm: 0,
        floor_index: 0,
        stance_pose: "idle".to_string(),
        portrait_url: None,
        pos_x_cm: Some(350),
        pos_y_cm: Some(350),
    }];

    // H8: Non-GM map view strips `notes`.
    let player_map = crate::map_handlers::filter_map_for_role(&map, SessionRole::Player);
    assert!(player_map.notes.is_none());

    // H13: Starting far away from origin portal (1100cm, 1100cm) to 2F portal (250cm, 250cm) must be rejected.
    assert!(
        !crate::map_handlers::is_near_vertical_portal(&map, 0, 1, Some((1100, 1100)), (250, 250)),
        "Must reject cross-floor jump when origin is far from source-floor stairwell"
    );
    assert!(
        crate::map_handlers::is_near_vertical_portal(&map, 0, 1, Some((250, 250)), (250, 250)),
        "Must allow cross-floor jump when both origin and dest are near floor portals"
    );

    state
        .auth
        .set_session_active_map("w2-floor-door", map)
        .await
        .unwrap();
    let gm_svc_tok = state.storage.config().ai_gm.service_token.clone();

    // Adjacent door toggles DoorClosed -> DoorOpen
    let near_door = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/player-ai/sessions/w2-floor-door/door")
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&PlayerAiDoorRequest {
                    user_id: "ai-player-1".to_string(),
                    cell_x: 3,
                    cell_y: 3,
                    side: EdgeSide::North,
                })
                .unwrap(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(near_door.status(), axum::http::StatusCode::OK);

    // Distant door (> 2.0m) is rejected
    let far_door = tower::ServiceExt::oneshot(
        app.clone(),
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/player-ai/sessions/w2-floor-door/door")
            .header("Authorization", format!("Bearer {gm_svc_tok}"))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&PlayerAiDoorRequest {
                    user_id: "ai-player-1".to_string(),
                    cell_x: 10,
                    cell_y: 10,
                    side: EdgeSide::North,
                })
                .unwrap(),
            ))
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(far_door.status(), axum::http::StatusCode::BAD_REQUEST);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
