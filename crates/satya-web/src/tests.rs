//! Unit and SSR HTML parity tests for `satya-web`.

use super::*;
use satya_schema::{CombatStance, MessageVisibility, SessionMode, SessionRole, TopicMetricsView};

#[test]
fn test_wasm_client_enum_parsers_and_metrics_table_render() {
    assert_eq!(
        wasm_client::parse_session_mode("MEETING"),
        SessionMode::Meeting
    );
    assert_eq!(wasm_client::parse_session_mode("TTRPG"), SessionMode::Ttrpg);
    assert_eq!(
        wasm_client::parse_session_mode("TTRPG_GM"),
        SessionMode::Ttrpg
    );
    assert_eq!(
        wasm_client::parse_session_mode("TTRPG_GM_AI"),
        SessionMode::Ttrpg
    );
    assert_eq!(
        wasm_client::parse_session_mode("TTRPG_AI_GM"),
        SessionMode::Ttrpg
    );
    assert!(!wasm_client::is_ai_gm_mode("TTRPG"));
    assert!(!wasm_client::is_ai_gm_mode("TTRPG_GM"));
    assert!(wasm_client::is_ai_gm_mode("TTRPG_GM_AI"));
    assert!(wasm_client::is_ai_gm_mode("TTRPG_AI_GM"));
    assert!(!wasm_client::is_ai_gm_mode("MEETING"));
    assert_eq!(wasm_client::parse_session_role("GM"), SessionRole::Gm);
    assert_eq!(wasm_client::parse_session_role("HOST"), SessionRole::Host);
    assert_eq!(
        wasm_client::parse_visibility("PRIVATE"),
        MessageVisibility::Private
    );
    assert_eq!(
        wasm_client::parse_combat_stance("MANEUVER"),
        Some(CombatStance::Maneuver)
    );
    assert_eq!(
        wasm_client::parse_combat_stance("punch"),
        Some(CombatStance::Power)
    );
    assert_eq!(
        wasm_client::parse_combat_stance("grab"),
        Some(CombatStance::Maneuver)
    );
    assert_eq!(
        wasm_client::parse_combat_stance("parry"),
        Some(CombatStance::Stability)
    );
    assert_eq!(
        wasm_client::parse_combat_stance("snap_shot"),
        Some(CombatStance::RangedSnap)
    );
    assert_eq!(
        wasm_client::parse_combat_stance("deliberate_aimed_shot"),
        Some(CombatStance::RangedAimed)
    );
    assert_eq!(wasm_client::parse_combat_stance(""), None);

    let (ttrpg_vis, ttrpg_ctx) = wasm_client::chat_options_for_mode(SessionMode::Ttrpg);
    assert!(ttrpg_vis.iter().any(|(v, _)| *v == "GM_ONLY"));
    assert!(!ttrpg_vis.iter().any(|(v, _)| *v == "HOST_ONLY"));
    assert!(ttrpg_ctx.iter().any(|(v, _)| *v == "IN_CHARACTER"));
    assert!(!ttrpg_ctx.iter().any(|(v, _)| *v == "MEETING_PUBLIC"));

    let (meet_vis, meet_ctx) = wasm_client::chat_options_for_mode(SessionMode::Meeting);
    assert!(meet_vis.iter().any(|(v, _)| *v == "HOST_ONLY"));
    assert!(!meet_vis.iter().any(|(v, _)| *v == "GM_ONLY"));
    assert!(meet_ctx.iter().any(|(v, _)| *v == "MEETING_PUBLIC"));
    assert!(!meet_ctx.iter().any(|(v, _)| *v == "IN_CHARACTER"));

    let ttrpg_roles = wasm_client::role_options_for_mode("TTRPG");
    assert!(ttrpg_roles.iter().any(|(v, _)| *v == "GM"));
    assert!(ttrpg_roles.iter().any(|(v, _)| *v == "PLAYER"));
    assert!(!ttrpg_roles.iter().any(|(v, _)| *v == "HOST"));
    assert!(!ttrpg_roles.iter().any(|(v, _)| *v == "PARTICIPANT"));

    let gm_ai_roles = wasm_client::role_options_for_mode("TTRPG_GM_AI");
    assert!(gm_ai_roles.iter().any(|(v, _)| *v == "GM"));
    assert!(gm_ai_roles.iter().any(|(v, _)| *v == "PLAYER"));
    assert!(gm_ai_roles.iter().any(|(v, _)| *v == "OBSERVER"));
    assert!(!gm_ai_roles.iter().any(|(v, _)| *v == "HOST"));

    let ai_gm_roles = wasm_client::role_options_for_mode("TTRPG_AI_GM");
    assert!(ai_gm_roles.iter().any(|(v, _)| *v == "PLAYER"));
    assert!(!ai_gm_roles.iter().any(|(v, _)| *v == "GM"));
    assert!(!ai_gm_roles.iter().any(|(v, _)| *v == "HOST"));

    let meet_roles = wasm_client::role_options_for_mode("MEETING");
    assert!(meet_roles.iter().any(|(v, _)| *v == "HOST"));
    assert!(meet_roles.iter().any(|(v, _)| *v == "PARTICIPANT"));
    assert!(!meet_roles.iter().any(|(v, _)| *v == "GM"));
    assert!(!meet_roles.iter().any(|(v, _)| *v == "PLAYER"));

    let table_html = wasm_client::format_metrics_table_html(&[TopicMetricsView {
        topic: "session-lifecycle".to_string(),
        topic_id: 2,
        storage_backend: "local".to_string(),
        destination_uri: "data/topics/session-lifecycle".to_string(),
        total_messages_ingested: 4,
        buffered_messages: 0,
        flushed_batches_count: 4,
        last_flushed_offset: Some(3),
        active_schema_version: 1,
        active_schema_fingerprint_hex: "0123456789abcdef".to_string(),
    }]);
    assert!(table_html.contains("session-lifecycle"));
    assert!(table_html.contains("last offset: 3"));

    // Initial mesh uses deterministic tie-breaking; camera/mic renegotiation always initiates
    assert!(wasm_client::should_initiate_peer_connection(
        "u-demo-gm-sean",
        "u-demo-sara",
        false
    ));
    assert!(!wasm_client::should_initiate_peer_connection(
        "u-demo-sara",
        "u-demo-gm-sean",
        false
    ));
    assert!(wasm_client::should_initiate_peer_connection(
        "u-demo-sara",
        "u-demo-gm-sean",
        true
    ));
    assert!(!wasm_client::should_initiate_peer_connection(
        "u-demo-sara",
        "u-demo-sara",
        true
    ));

    // Phase 3: AI GM provider default models & tabletop canvas HTML formatters
    assert_eq!(
        wasm_client::default_model_for_provider("google_genai"),
        "gemini-2.5-flash"
    );
    assert_eq!(wasm_client::default_model_for_provider("openai"), "gpt-4o");
    assert_eq!(
        wasm_client::default_model_for_provider("anthropic"),
        "claude-3-7-sonnet-latest"
    );
    assert_eq!(
        wasm_client::default_model_for_provider("ollama"),
        "gemma4:12b"
    );

    let map_req = satya_schema::GmMapUpdateRequest {
        map_id: "map-docking-bay".to_string(),
        title: "Sector 4 Docking Bay".to_string(),
        image_url: "/api/maps/ep-1/map-docking-bay.svg".to_string(),
        grid_width: 12,
        grid_height: 10,
        tokens: vec![satya_schema::TokenPlacement {
            entity_id: "tok-vance".to_string(),
            label: "Capt. Vance".to_string(),
            x: 5,
            y: 4,
            visibility: satya_schema::MessageVisibility::GmOnly,
        }],
        notes: Some("Cover bulkhead at (5, 4)".to_string()),
    };
    let tokens_html = wasm_client::format_tactical_tokens_html(&map_req);
    assert!(tokens_html.contains("Capt. Vance (5, 4)"));
    assert!(tokens_html.contains("GmOnly"));

    let dice_actor = satya_schema::Participant::human(
        "u-sara",
        "sara_p1",
        "Sara (Valkyrie)",
        satya_schema::SessionRole::Player,
    );
    let dice_html = wasm_client::format_wgpu_dice_roll_html(
        &dice_actor,
        &satya_schema::WgpuDiceSpec {
            dice_notation: "3d20".to_string(),
            physics_seed: 42,
            outcomes: vec![3, 11, 19],
            visibility: satya_schema::MessageVisibility::Public,
        },
    );
    assert!(dice_html.contains("3d20"));
    assert!(dice_html.contains(r#"<span class="dice-cube">3</span>"#));
    assert!(dice_html.contains(r#"<span class="dice-cube">19</span>"#));

    // Chat timestamp formatting & ChatMessageEvent HTML rendering
    // 2026-11-02T18:42:15.123000Z = 1_793_644_935_123_000 microseconds
    let (ts_short, ts_iso) = wasm_client::format_timestamp_us_utc(1_793_644_935_123_000);
    assert_eq!(ts_short, "18:42:15");
    assert_eq!(ts_iso, "2026-11-02T18:42:15.123Z");

    let chat_ev = satya_schema::ChatMessageEvent {
        metadata: satya_schema::EventMetadata {
            event_id: "ev-chat-ts-1".to_string(),
            sequence_num: 1,
            client_timestamp_us: 1_793_644_935_123_000,
            ingest_timestamp_us: 1_793_644_935_123_000,
            timezone_offset_min: -300,
            schema_subject: "io.satya.events.v1.ChatMessageEvent".to_string(),
            schema_version: 1,
            schema_fingerprint_crc64: vec![0; 8],
            session_id: "ep1-test".to_string(),
            session_mode: SessionMode::Ttrpg,
        },
        message_id: "msg-ts-1".to_string(),
        sender: dice_actor.clone(),
        visibility: MessageVisibility::Public,
        recipients: vec![],
        channel_context: satya_schema::ChannelContext::OutOfCharacter,
        reply_to_message_id: None,
        body: "Checking comms!".to_string(),
        char_count: 15,
        word_count: 2,
        composition_time_ms: 420,
        is_edited: false,
    };
    let chat_html = wasm_client::format_chat_event_html(&chat_ev);
    assert!(chat_html.contains(r#"class="chat-timestamp""#));
    assert!(chat_html.contains("18:42:15"));
    assert!(chat_html.contains("2026-11-02T18:42:15.123Z"));
    assert!(chat_html.contains("Checking comms!"));

    // satya-engine Four-Factor Cascade preview & mechanics HTML rendering
    let params = satya_engine::CascadeTaskParams::new_k20(3, 12, 0, 0);
    let preview_str = wasm_client::format_cascade_odds_preview(&params);
    assert!(preview_str.contains("TN_eff 19"));
    assert!(preview_str.contains("P(Succ)"));

    let (pill, readout) =
        wasm_client::format_cascade_odds_summary(3, 10, 0, 0, "markov_step", "closest_miss");
    assert_eq!(pill, "TN_eff: 21 · P(Succ≥1): 70.1%");
    assert!(readout.contains("P(Succ ≥ 1): 70.10%"));
    assert!(readout.contains("P(BotchFail): 12.96%"));
    assert!(readout.contains("P(Succ+Botch): 1.13%"));
    assert!(readout.contains("E[Margin]: +1.27"));

    let (c_pill, c_readout) = wasm_client::format_contested_cascade_odds_summary(
        3,
        12,
        0,
        4,
        2,
        "markov_step",
        "closest_miss",
    );
    assert!(c_pill.contains("TN_P: 19 vs TN_GM: 17"));
    assert!(c_readout.contains("Your Target (TN_P = 19)"));
    assert!(c_readout.contains("GM / Hazard Target (TN_GM = 17)"));
    assert!(c_readout.contains("ttrpg-odds-stacked-bar"));
    assert!(c_readout.contains("ttrpg-odds-table"));

    let (mech_ev, _) = satya_engine::evaluate_satya_cascade_task(
        chat_ev.metadata.clone(),
        dice_actor,
        vec![],
        "Trauma Surgery".to_string(),
        &params,
        &[(14, 9), (7, 11), (15, 12)],
        Some(CombatStance::Maneuver),
        1,
        0.0,
    );
    let mech_html = wasm_client::format_mechanics_event_html(&mech_ev);
    assert!(mech_html.contains("2d20 Pairs: [(14+9=23), (7+11=18), (15+12=27)]"));
    assert!(mech_html.contains("Control C: 12"));
    assert!(mech_html.contains("TN_eff: 19"));
    assert!(mech_html.contains("cascade(mode=markov_step"));
}

#[test]
fn test_leptos_app_renders_wasm_bootstrap_and_disconnect_controls() {
    let html = render_satya_app_html();
    assert!(html.contains("/pkg/satya_web.js"));
    assert!(html.contains("Satya // Engine"));
    assert!(html.contains("<h1>Satsang</h1>"));
    assert!(!html.contains("SATYA // ENGINE (RUST + WASM)"));
    assert!(!html.contains("Apache Iggy + Avro v1/v2 Ready"));
    assert!(!html.contains("WebRTC + Durable Avro Event Stream"));
    assert!(html.contains("TTRPG Mode (GM + Players)"));
    assert!(html.contains("TTRPG Mode (GM + AI + Players)"));
    assert!(html.contains("TTRPG Mode (AI GM + Players)"));
    assert!(html.contains("btn-toggle-auth-password"));
    assert!(html.contains("btn-toggle-session-password"));
    assert!(html.contains("btn-toggle-ai-gm-key"));
    assert!(html.contains("ai-gm-config-fields"));
    assert!(html.contains("ai-gm-provider-select"));
    assert!(html.contains("ai-gm-model-input"));
    assert!(html.contains("ai-gm-api-key-input"));
    assert!(html.contains("btn-menu-toggle-ai-gm-settings"));
    assert!(html.contains("menu-ai-gm-settings-panel"));
    assert!(html.contains("tabletop-canvas-card"));
    assert!(html.contains("tabletop-map-viewport"));
    assert!(html.contains("wgpu-dice-tray"));
    assert!(html.contains("wgpu-dice-canvas"));
    assert!(html.contains("dice-material-preset"));
    assert!(html.contains("dice-base-color"));
    assert!(html.contains("dice-ink-color"));
    assert!(html.contains("dice-opacity-slider"));
    assert!(html.contains("38% opacity"));
    assert!(html.contains("dice-camera-yaw"));
    assert!(html.contains("dice-camera-pitch"));
    assert!(html.contains("dice-camera-zoom"));
    assert!(html.contains("btn-dice-pan-left"));
    assert!(html.contains("btn-reset-dice-camera"));
    assert!(html.contains("wgpu-dice-readout"));
    assert!(html.contains("btn-hamburger-menu"));
    assert!(html.contains("hamburger-menu-dropdown"));
    assert!(html.contains("btn-menu-toggle-auth-pane"));
    assert!(html.contains("btn-menu-toggle-session-pane"));
    assert!(html.contains("btn-menu-toggle-settings"));
    assert!(html.contains("btn-menu-toggle-chat-settings"));
    assert!(html.contains("menu-chat-settings-panel"));
    assert!(html.contains("filter-checkboxes-vertical"));
    assert!(html.contains(r#"id="auth-card""#));
    assert!(html.contains(r#"id="session-card""#));
    assert!(html.contains(r#"id="workspace-panel""#));
    let session_card_idx = html.find(r#"id="session-card""#).unwrap();
    assert!(
        html[session_card_idx.saturating_sub(40)..session_card_idx + 80].contains("display: none")
    );
    let workspace_idx = html.find(r#"id="workspace-panel""#).unwrap();
    assert!(html[workspace_idx.saturating_sub(40)..workspace_idx + 80].contains("display: none"));
    assert!(html.contains("<h2>Session Chat</h2>"));
    assert!(!html.contains("Public &amp; Private Targeted Session Chat"));
    assert!(html.contains("<h2>Task Resolution &amp; Action Pool</h2>"));
    assert!(!html.contains("Satya TTRPG Task Resolution &amp; Combat Economy"));
    assert!(html.contains("Roll Satya Task"));
    assert!(html.contains(r#"id="ttrpg-skill""#));
    assert!(html.contains(r#"id="btn-toggle-ttrpg-skill-dropdown""#));
    assert!(html.contains(r#"id="ttrpg-skill-dropdown""#));
    assert!(html.contains("window.satyaFilterTtrpgSkillDropdown()"));
    assert!(html.contains("window.satyaSelectTtrpgSkill("));
    assert!(html.contains(r#"id="ttrpg-skill-level""#));
    assert!(html.contains(r#"id="ttrpg-control-c""#));
    assert!(html.contains(r#"id="ttrpg-internal-mod""#));
    assert!(html.contains(r#"id="ttrpg-external-mod""#));
    assert!(html.contains(r#"id="ttrpg-carry-mode""#));
    assert!(html.contains(r#"id="ttrpg-botch-rule""#));
    assert!(html.contains(r#"id="ttrpg-wound-shock""#));
    assert!(html.contains(r#"id="ttrpg-odds-pill""#));
    assert!(html.contains(r#"id="ttrpg-odds-readout""#));
    assert!(html.contains("ttrpg-odds-stacked-bar"));
    assert!(html.contains("ttrpg-odds-table"));
    let combat_pane_idx = html.find(r#"id="notebook-pane-combat""#).unwrap();
    let rules_pane_idx = html.find(r#"id="notebook-pane-rules""#).unwrap();
    let stance_idx = html.find(r#"id="ttrpg-stance""#).unwrap();
    assert!(
        stance_idx > combat_pane_idx && stance_idx < rules_pane_idx,
        "ttrpg-stance should live inside notebook-pane-combat rather than ttrpg-drawer"
    );
    for tech_code in [
        "grab",
        "disarm",
        "takedown",
        "throw",
        "trap",
        "lock",
        "push",
        "punch",
        "elbow",
        "knee",
        "kick",
        "head_butt",
        "thrust",
        "swing",
        "block",
        "parry",
        "feint",
        "dodge",
        "escape",
        "probe",
        "root",
        "snap_shot",
        "combat_sighted_shot",
        "deliberate_aimed_shot",
        "controlled_double_tap_burst",
        "suppressive_full_auto",
        "brace_weapon",
        "cover_peek_slice_pie",
        "bounding_shoot_on_move",
        "overwatch_hold_sector",
    ] {
        assert!(
            html.contains(&format!(r#"value="{tech_code}""#)),
            "missing canonical combat technique option '{tech_code}' in SSR HTML"
        );
    }
    assert!(html.contains("container: ttrpg-card / inline-size"));
    assert!(html.contains("@container ttrpg-card"));
    assert!(!html.contains(r#"id="ttrpg-tn""#));
    assert!(!html.contains(r#"id="ttrpg-pool""#));
    assert!(!html.contains(r#"id="ttrpg-diff""#));
    assert!(!html.contains("Roll Satya Task &amp; Emit Avro Event"));
    assert!(html.contains("filter-msg-system"));
    assert!(!html.contains(r#"id="chat-visibility""#));
    assert!(!html.contains(r#"id="chat-context""#));
    assert!(html.contains("recipient-cb-broadcast"));
    assert!(html.contains("BROADCAST (All Participants)"));
    assert!(html.contains("btn-ctx-ic"));
    assert!(html.contains("btn-ctx-ooc"));
    assert!(html.contains("btn-ctx-gm"));
    assert!(!html.contains("MEETING_PUBLIC"));
    assert!(!html.contains(r#"<option value="HOST">Meeting Host</option>"#));
    assert!(!html.contains(r#"<option value="PARTICIPANT">Meeting Participant</option>"#));
    let chat_idx = html.find(r#"class="card chat-card""#).unwrap();
    let ttrpg_idx = html.find(r#"id="ttrpg-drawer""#).unwrap();
    assert!(
        ttrpg_idx > chat_idx,
        "ttrpg-drawer should be rendered beneath chat-card"
    );
    // Verify 3-column layout order: Left Notebook -> Center Chat+Die Roller -> Right Web Cams
    let notebook_col_idx = html.find(r#"id="notebook-column""#).unwrap();
    let center_col_idx = html.find(r#"id="center-column""#).unwrap();
    let webrtc_col_idx = html.find(r#"id="webrtc-dock-column""#).unwrap();
    assert!(notebook_col_idx < center_col_idx);
    assert!(center_col_idx < webrtc_col_idx);

    // Verify all 9 Tabbed Notebook tabs & panes exist
    for (_, btn_id, pane_id) in wasm_client::NOTEBOOK_TABS {
        assert!(html.contains(&format!(r#"id="{btn_id}""#)));
        assert!(html.contains(&format!(r#"id="{pane_id}""#)));
    }

    // Verify Web Cams, Session Chat, and Task Roller collapsible drawer controls
    assert!(html.contains(r#"id="right-drawer-rails""#));
    assert!(html.contains(r#"id="btn-toggle-webcam-cols""#));
    assert!(html.contains(r#"id="btn-toggle-webcam-float""#));
    assert!(html.contains(r#"id="btn-toggle-webcam-drawer""#));
    assert!(html.contains(r#"id="btn-expand-webcam-drawer""#));
    assert!(html.contains(r#"id="webcam-drawer-rail""#));
    assert!(html.contains(r#"id="btn-menu-toggle-webcam-pane""#));
    assert!(html.contains(r#"id="chat-card""#));
    assert!(html.contains(r#"id="chat-drawer-rail""#));
    assert!(html.contains(r#"id="btn-toggle-chat-drawer""#));
    assert!(html.contains(r#"id="btn-expand-chat-drawer""#));
    assert!(html.contains(r#"id="btn-menu-toggle-chat-pane""#));
    assert!(html.contains(r#"id="ttrpg-drawer-rail""#));
    assert!(html.contains(r#"id="btn-toggle-ttrpg-drawer""#));
    assert!(html.contains(r#"id="btn-expand-ttrpg-drawer""#));
    assert!(html.contains(r#"id="btn-menu-toggle-ttrpg-pane""#));
    assert!(!html.contains("center-drawer-rail"));
    let right_rails_idx = html.find(r#"id="right-drawer-rails""#).unwrap();
    let chat_rail_idx = html.find(r#"id="chat-drawer-rail""#).unwrap();
    let ttrpg_rail_idx = html.find(r#"id="ttrpg-drawer-rail""#).unwrap();
    let webcam_rail_idx = html.find(r#"id="webcam-drawer-rail""#).unwrap();
    assert!(right_rails_idx > webrtc_col_idx);
    assert!(chat_rail_idx > right_rails_idx);
    assert!(ttrpg_rail_idx > chat_rail_idx);
    assert!(webcam_rail_idx > ttrpg_rail_idx);
    assert!(html.contains(r#"id="webrtc-drag-header""#));
    assert!(html.contains(r#"class="video-grid cols-1""#));

    // Verify Google OAuth (`drive.file`), GM_ONLY Document Vault, Persistent GM Memory & Omnisearch controls
    assert!(html.contains(r#"id="btn-google-auth""#));
    assert!(html.contains(r#"id="google-auth-status-pill""#));
    assert!(html.contains(r#"id="tab-btn-docs""#));
    assert!(html.contains(r#"id="notebook-pane-docs""#));
    assert!(html.contains(r#"id="gm-memories-count-pill""#));
    assert!(html.contains("0. GM Archivist (information lookup)"));
    assert!(html.contains(r#"id="gm-omnisearch-input""#));
    assert!(html.contains(r#"id="gm-omnisearch-scope""#));
    assert!(html.contains(r#"id="btn-gm-omnisearch""#));
    assert!(html.contains(r#"id="gm-omnisearch-results""#));
    assert!(html.contains(r#"id="gm-doc-file-input""#));
    assert!(html.contains(r#"id="gm-doc-title""#));
    assert!(html.contains(r#"id="gm-doc-source""#));
    assert!(html.contains(r#"id="gm-doc-persistent""#));
    assert!(html.contains(r#"id="gm-doc-campaign-tag""#));
    assert!(html.contains(r#"id="btn-google-picker""#));
    assert!(html.contains(r#"id="gm-google-oauth-config-row""#));
    assert!(html.contains(r#"id="gm-google-client-id""#));
    assert!(html.contains(r#"id="gm-google-api-key""#));
    assert!(html.contains(r#"id="gm-doc-content""#));
    assert!(html.contains(r#"id="btn-submit-gm-doc""#));
    assert!(html.contains(r#"id="gm-docs-list""#));
    assert!(html.contains(r#"id="gm-memory-title""#));
    assert!(html.contains(r#"id="gm-memory-category""#));
    assert!(html.contains(r#"id="gm-memory-pinned""#));
    assert!(html.contains(r#"id="gm-memory-persistent""#));
    assert!(html.contains(r#"id="gm-memory-content""#));
    assert!(html.contains(r#"id="btn-submit-gm-memory""#));
    assert!(html.contains(r#"id="gm-memories-list""#));

    assert_eq!(
        wasm_client::extract_google_drive_file_id_and_kind(
            "https://docs.google.com/document/d/1aBcDeFgHiJkLmNoPqRsTuVwXyZ/edit?usp=sharing"
        ),
        (
            "1aBcDeFgHiJkLmNoPqRsTuVwXyZ".to_string(),
            Some("GOOGLE_DOC")
        )
    );
    assert_eq!(
        wasm_client::extract_google_drive_file_id_and_kind(
            "https://docs.google.com/spreadsheets/d/1sheetId987654/edit#gid=0"
        ),
        ("1sheetId987654".to_string(), Some("GOOGLE_SHEET"))
    );

    let (model_html, model_pill) =
        wasm_client::format_3d_model_stage_html("recon_mk4", "hit_zones");
    assert!(model_html.contains("Recon Suit Mk-IV"));
    assert!(model_pill.contains("11.8k tris"));

    assert!(html.contains(r#"id="tab-btn-rules""#));
    assert!(html.contains(r#"id="notebook-pane-rules""#));
    assert!(html.contains(r#"id="rules-chapter-select""#));
    assert!(html.contains(r#"value="notation_glossary""#));
    assert!(html.contains(r#"value="task_requirements_catalog""#));
    assert!(html.contains(r#"value="skills_trees_tactical_and_social""#));
    assert!(html.contains(r#"value="firearms_and_ballistics""#));
    assert!(html.contains(r#"id="rules-search-input""#));
    assert!(html.contains(r#"id="rules-book-viewport""#));
    assert!(html.contains("Cascading 2d20 Engine &amp; Task Resolution"));
    assert!(html.contains(".rules-book-viewport"));
    assert!(html.contains("height: max(680px, calc(100dvh - 76px))"));
    assert!(html.contains("min-height: clamp(360px, 48dvh, 560px)"));
    assert!(html.contains(".chat-card > .chat-composer"));
    assert!(html.contains("max-height: 38%"));
    let rules_css_idx = html.find(".satya-rules-article").unwrap();
    let wiz_css_idx = html.find(".wiz-shell").unwrap();
    assert!(!html[rules_css_idx..wiz_css_idx].contains("overscroll-behavior: contain"));
    assert!(!html[rules_css_idx..wiz_css_idx].contains("overflow-x: auto"));
    assert!(html.contains("rules-math-inline"));
    assert!(html.contains("rules-math-block"));
    assert!(html.contains("rules-mermaid"));
    assert!(html.contains("mermaid-node"));
    assert!(!html.contains(r"\text{TN}_P"));
    assert!(!html.contains("&amp;amp;"));

    assert!(html.contains("btn-logout"));
    assert!(html.contains("btn-leave-session"));
    assert!(html.contains("btn-workspace-disconnect"));
    assert!(html.contains(r#"id="admin-inspector-card""#));
    assert!(html.contains("display: none"));

    // Verify Character Creation Wizard tab & all 6 steps
    assert!(html.contains(r#"id="tab-btn-wizard""#));
    assert!(html.contains(r#"id="notebook-pane-wizard""#));
    assert!(html.contains(r#"id="wiz-char-name-input""#));
    assert!(html.contains(r#"id="wizard-stage-container""#));
    assert!(html.contains("Method 1: Roll 2d4 + 3d5 (Drop Lowest d5)"));
    assert!(html.contains("Method 2: Roll 2d4 + 2d5 Pool (Keep Highest)"));
    assert!(html.contains("Physical Dimensions: Height (HGT) &amp; Mass (MAS)"));

    let mut wiz = wasm_client::CharacterWizardState {
        active_step: 2,
        ..Default::default()
    };
    let step2_html = wasm_client::format_character_wizard_html(&wiz);
    assert!(step2_html.contains("Step 2: Psyche Characteristics"));
    assert!(step2_html.contains("wiz-psyche-range-ego"));

    wiz.active_step = 3;
    let step3_html = wasm_client::format_character_wizard_html(&wiz);
    assert!(step3_html.contains("Step 3: Domain Aptitudes (11 × 3d6 Bell-Curve, Drop 2 Lowest"));
    assert!(step3_html.contains("APT-QNT"));
    assert!(step3_html.contains("APT-SOC"));

    wiz.active_step = 4;
    let step4_html = wasm_client::format_character_wizard_html(&wiz);
    assert!(step4_html.contains("Step 4: Chronological 5-Stage Lifepath"));
    assert!(step4_html.contains("Stage 1 (Ages 0–5): Socioeconomic Origin"));
    assert!(step4_html.contains("Stage 5 (Ages 18–34): Adult 4-Year Career Terms"));

    wiz.active_step = 5;
    let step5_html = wasm_client::format_character_wizard_html(&wiz);
    assert!(step5_html.contains("Step 5: 8-Tree Hierarchical Skill Builder"));
    assert!(step5_html.contains("5-Factor SP Cost Calculator"));
    assert!(step5_html.contains("Piloting (Rotary-Wing / Helicopter &amp; VTOL)"));
    assert!(step5_html.contains("Tree 1: Mathematics, Computing &amp; Physical Sciences"));

    wiz.active_step = 6;
    let step6_html = wasm_client::format_character_wizard_html(&wiz);
    assert!(step6_html.contains("Step 6: Derived Telemetry"));
    assert!(step6_html.contains("Striking Power (POW)"));
    assert!(step6_html.contains("OODA Initiative"));

    // Verify Complete Character Sheet / Status pane (All 30 Characteristics, 9 Aptitudes,
    // Talents & Skills tables, Weapons/Armor/Gear tables, Human-Shaped Hit Location SVG,
    // Damage/Fatigue/Psyche trackers, and Writable Notes)
    assert!(html.contains(r#"id="char-sheet-preset-select""#));
    assert!(html.contains(r#"id="char-sheet-stage-container""#));
    assert!(html.contains(r#"id="char-human-hit-svg""#));
    assert!(html.contains(r#"id="char-sheet-skills-table""#));
    assert!(html.contains(r#"id="char-sheet-talents-table""#));
    assert!(html.contains(r#"id="char-sheet-ranged-table""#));
    assert!(html.contains(r#"id="char-sheet-melee-table""#));
    assert!(html.contains(r#"id="char-sheet-armor-table""#));
    assert!(html.contains(r#"id="char-sheet-gear-table""#));
    assert!(html.contains(r#"id="char-sheet-notes-textarea""#));

    let mut sheet = wasm_client::CharacterSheetState::from_preset("mateo_vance");
    assert_eq!(sheet.character_name, "Mateo Vance");
    let zone_idx = satya_engine::HitLocationId::ChestRibcageLungs as usize;
    sheet.hit_zones[zone_idx].wound_tier = 3;
    sheet.hit_zones[zone_idx].bone_status = wasm_client::BoneFractureStatus::CompleteFracture;
    sheet.hit_zones[zone_idx].bleed_grade = satya_engine::HemorrhageGrade::Grade4ArterialSeverance;
    let mateo_html = wasm_client::format_character_sheet_html(&sheet);
    assert!(mateo_html.contains("Mateo Vance"));
    assert!(mateo_html.contains("Broken Bone"));
    assert!(mateo_html.contains("Bleed G4"));
    assert!(mateo_html.contains("Kalashnikov AKM Assault Rifle"));
    assert!(mateo_html.contains("Concealable Soft Kevlar Vest"));
    assert!(mateo_html.contains(r#"id="char-sheet-vault-bar""#));
    assert!(mateo_html.contains(r#"id="char-sheet-verification-badge""#));
    assert!(mateo_html.contains(r#"id="char-sheet-save-vault-btn""#));
    assert!(mateo_html.contains(r#"id="char-sheet-export-btn""#));
    assert!(mateo_html.contains(r#"id="char-sheet-import-file-input""#));
    assert!(mateo_html.contains(r#"id="char-sheet-assets-table""#));

    let record = sheet.to_character_sheet_record("usr-sara");
    let mut restored = wasm_client::CharacterSheetState::default();
    restored.apply_character_sheet_record(&record);
    assert_eq!(restored.character_name, "Mateo Vance");
    assert_eq!(restored.hit_zones[zone_idx].wound_tier, 3);
    assert_eq!(
        restored.hit_zones[zone_idx].bone_status,
        wasm_client::BoneFractureStatus::CompleteFracture
    );

    // Searchable Skill Name dropdown in Task Resolution & Action Pool
    let default_sheet = wasm_client::CharacterSheetState::default();
    let surg_hits = wasm_client::search_canonical_skills("Trauma Surgery", &default_sheet);
    assert_eq!(surg_hits.len(), 1);
    assert_eq!(surg_hits[0].id, "trauma_orthopedic_general_surgery");

    let dd_html = wasm_client::format_ttrpg_skill_dropdown_html("rotary", &default_sheet);
    assert!(dd_html.contains("Piloting (Rotary-Wing / Helicopter &amp; VTOL)"));
    assert!(dd_html.contains("window.satyaSelectTtrpgSkill('piloting_rotary_wing_helicopter')"));

    let empty_dd = wasm_client::format_ttrpg_skill_dropdown_html("zzzz_nonexistent_skill", &default_sheet);
    assert!(empty_dd.contains("No canonical skills matched"));

    // Verify NPC List tab & Role-Filtered GM/Solo vs Player NPC card formatting
    assert!(html.contains(r#"id="tab-btn-npcs""#));
    assert!(html.contains(r#"id="notebook-pane-npcs""#));
    assert!(html.contains(r#"id="npc-roster-select""#));
    assert!(html.contains(r#"id="npc-selected-card-container""#));
    assert!(html.contains(r#"id="npc-generator-panel""#));
    assert!(html.contains(r#"id="npc-gen-prompt""#));
    assert!(html.contains(r#"id="npc-gen-template""#));
    assert!(html.contains(r#"id="btn-generate-npc""#));

    let terry_entry = satya_schema::SessionNpcEntry {
        character_id: "npc_terry".to_string(),
        character_name: "Terry".to_string(),
        callsign_or_role: "General MD & Clinical Physician".to_string(),
        age_years: 26,
        public_summary: "Terry — General MD & Clinical Physician (Age 26). Apparent demeanor: kind general MD.".to_string(),
        apparent_condition: "Healthy & Alert".to_string(),
        is_persistent: true,
        gm_character: None,
        psyche_summary: vec![
            "SLF 5 (Altruistic / Self-Sacrificing)".to_string(),
            "BLG 16 (Tribal / Group-Dependent)".to_string(),
        ],
        top_skills: vec!["Clinical Diagnosis & Internal Medicine (N=3)".to_string()],
        gm_secret_dossier: Some("Secretly treating wounded couriers for free.".to_string()),
    };

    let gm_card_html = wasm_client::format_npc_card_html(&terry_entry, true);
    assert!(gm_card_html.contains("BLG 16 (Tribal / Group-Dependent)"));
    assert!(gm_card_html.contains("Clinical Diagnosis &amp; Internal Medicine (N=3)"));
    assert!(gm_card_html.contains("Secretly treating wounded couriers for free."));

    let player_card_html = wasm_client::format_npc_card_html(&terry_entry, false);
    assert!(player_card_html.contains("Terry"));
    assert!(player_card_html.contains("General MD &amp; Clinical Physician"));
    assert!(!player_card_html.contains("BLG 16"));
    assert!(!player_card_html.contains("Secretly treating wounded couriers for free."));
    assert!(player_card_html.contains("Player View: GM-only Characteristics"));

    // Verify Markdown Chat Composer (<textarea>) & Scenario Director Designer UI in SSR HTML
    assert!(html.contains(r#"id="chat-input""#));
    assert!(html.contains(r#"class="chat-markdown-input""#));
    assert!(html.contains(r#"id="scenario-director-designer-panel""#));
    assert!(html.contains(r#"id="scenario-flow-mode""#));
    assert!(html.contains(r#"id="scenario-premise""#));
    assert!(html.contains(r#"id="scenario-planned-beats""#));
    assert!(html.contains(r#"id="scenario-beat-prob""#));
    assert!(html.contains(r#"id="scenario-beat-interval""#));
    assert!(html.contains(r#"id="scenario-dial-combat_encounter""#));
    assert!(html.contains(r#"id="scenario-dial-thought_provoking_discussion""#));
    assert!(html.contains(r#"id="scenario-dial-dramatic_revelation""#));
    assert!(html.contains(r#"id="btn-save-scenario-director""#));

    // Verify Markdown rendering in Session Chat (bold, italics, lists, blockquotes, code, math, XSS safety)
    let md_html = wasm_client::format_chat_markdown_html(
        "**Vance** whispers *urgent* telemetry:\n\n> **[COMMS INTERCEPT]** Lockdown in `Sector 4`\n\n- Breach airlock\n- Negotiate with $\\text{TN}_P = 19$\n\n<script>alert('xss')</script>",
    );
    assert!(md_html.contains(r#"class="chat-markdown-body""#));
    assert!(md_html.contains("<strong>Vance</strong>"));
    assert!(md_html.contains("<em>urgent</em>"));
    assert!(md_html.contains("<blockquote"));
    assert!(md_html.contains(">Sector 4</code>"));
    assert!(md_html.contains("<li>Breach airlock</li>"));
    assert!(md_html.contains("rules-math-inline"));
    assert!(!md_html.contains("<script>"));
    assert!(md_html.contains("&lt;script&gt;"));

    // Verify Scenario Director JSON serialization from UI controls
    let dir_json = wasm_client::build_scenario_director_config_json(
        "hybrid",
        "Orbital quarantine",
        "Beat 1: Distress signal\nBeat 2: Enforcer lockdown",
        40,
        3,
        &[
            ("combat_encounter".to_string(), 60.0),
            ("thought_provoking_discussion".to_string(), 0.0),
        ],
    );
    let parsed_dir: serde_json::Value = serde_json::from_str(&dir_json).unwrap();
    assert_eq!(parsed_dir["flow_mode"], "hybrid");
    assert_eq!(parsed_dir["beat_interval_turns"], 3);
    assert_eq!(parsed_dir["planned_beats"].as_array().unwrap().len(), 2);
    assert_eq!(parsed_dir["event_weights"]["combat_encounter"], 60.0);
    assert_eq!(
        parsed_dir["event_weights"]["thought_provoking_discussion"],
        0.0
    );

    // Verify Appearance & Theme settings in Hamburger Menu + option normalizers
    assert_eq!(wasm_client::normalize_theme_id("dusk-orange"), "dusk-orange");
    assert_eq!(wasm_client::normalize_theme_id("nebula"), "nebula");
    assert_eq!(wasm_client::normalize_theme_id("parchment"), "parchment");
    assert_eq!(wasm_client::normalize_theme_id("unknown-theme"), "obsidian");
    assert_eq!(wasm_client::normalize_density_id("compact"), "compact");
    assert_eq!(wasm_client::normalize_density_id("spacious"), "spacious");
    assert_eq!(wasm_client::normalize_density_id("invalid"), "default");
    assert_eq!(wasm_client::normalize_corners_id("sharp"), "sharp");
    assert_eq!(wasm_client::normalize_corners_id("rounded"), "rounded");
    assert_eq!(wasm_client::normalize_corners_id("invalid"), "default");
    assert_eq!(wasm_client::normalize_font_id("mono"), "mono");
    assert_eq!(wasm_client::normalize_font_id("serif"), "serif");
    assert_eq!(wasm_client::normalize_font_id("invalid"), "sans");

    assert!(html.contains(r#"id="btn-menu-toggle-appearance-settings""#));
    assert!(html.contains(r#"id="menu-appearance-settings-panel""#));
    assert!(html.contains(r#"id="appearance-swatch-grid""#));
    for (theme_id, _) in wasm_client::APPEARANCE_THEME_OPTIONS {
        assert!(html.contains(&format!(r#"id="btn-theme-swatch-{theme_id}""#)));
        assert!(html.contains(&format!(r#"value="{theme_id}""#)));
    }
    assert!(html.contains(r#"id="menu-theme-select""#));
    assert!(html.contains(r#"id="menu-density-select""#));
    assert!(html.contains(r#"id="menu-corners-select""#));
    assert!(html.contains(r#"id="menu-font-select""#));
    assert!(html.contains(r#"id="btn-reset-appearance-defaults""#));
    assert!(html.contains(r#"html[data-theme="dusk-orange"]"#));
    assert!(html.contains(r#"html[data-theme="nebula"]"#));
    assert!(html.contains(r#"html[data-theme="parchment"]"#));
    assert!(html.contains(r#"html[data-density="compact"]"#));
    assert!(html.contains(r#"html[data-corners="sharp"]"#));
    assert!(html.contains(r#"html[data-font="mono"]"#));

    // Verify Saved Session Resume, .satyasession Snapshot Export/Import, and .md/.json Chat Transcript Export UI
    assert!(html.contains(r#"id="saved-sessions-select""#));
    assert!(html.contains(r#"id="btn-resume-saved-session""#));
    assert!(html.contains(r#"id="btn-refresh-saved-sessions""#));
    assert!(html.contains(r#"id="btn-export-session-snapshot""#));
    assert!(html.contains(r#"id="session-import-file-input""#));
    assert!(html.contains(r#"id="btn-import-session-snapshot""#));
    assert!(html.contains(r#"id="btn-export-chat-md""#));
    assert!(html.contains(r#"id="btn-export-chat-json""#));

    let opts_html = wasm_client::format_saved_sessions_options_html(
        &[satya_schema::SavedSessionSummary {
            session_id: "harsh-reality-ep1".to_string(),
            session_mode: satya_schema::SessionMode::Ttrpg,
            role: satya_schema::SessionRole::Gm,
            display_name: "Sean (GM)".to_string(),
            host_user_id: "u-demo-gm-sean".to_string(),
            campaign_tag: "harsh-reality-ep1".to_string(),
            has_ai_gm: true,
            character_id: None,
            chat_event_count: 12,
            mechanics_event_count: 4,
            active_human_count: 2,
            created_at_us: 1_700_000_000_000_000,
            updated_at_us: 1_700_000_100_000_000,
        }],
        Some("harsh-reality-ep1"),
    );
    assert!(opts_html.contains(r#"value="harsh-reality-ep1" selected"#));
    assert!(opts_html.contains("12 msgs, 4 rolls (2 live)"));
}
