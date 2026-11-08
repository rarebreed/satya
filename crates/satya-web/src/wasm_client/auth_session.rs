//! User registration, login, logout, demo presets, password-protected session create/join/leave
//! lifecycle, and session participant roster rendering.

use crate::wasm_client::chat_and_ttrpg::{
    append_system_notice, satya_select_broadcast, satya_toggle_recipient,
    sync_gm_context_button_visibility,
};
use crate::wasm_client::dom::{
    document, escape_html, fetch_json, get_element, input_checked, input_value,
    local_storage_remove, local_storage_set, select_value, set_button_disabled, set_class,
    set_display, set_feedback, set_input_checked, set_input_value, set_select_value, set_text,
};
use crate::wasm_client::notebook::{render_combat_tracker, sync_google_auth_status_from_server};
use crate::wasm_client::options::{
    default_model_for_provider, is_ai_gm_mode, parse_session_mode, parse_session_role,
    sync_chat_options_for_mode, sync_role_options_for_mode,
};
use crate::wasm_client::session_persistence::{
    AUTH_TOKEN_STORAGE_KEY, ACTIVE_SESSION_STORAGE_KEY, PersistedActiveSessionRef,
    replay_session_history_snapshot, satya_refresh_saved_sessions,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use crate::wasm_client::telemetry::satya_refresh_metrics;
use crate::wasm_client::webrtc::{connect_signaling_websocket, send_ws_packet};
use satya_schema::{
    AccountRole, AiGmProviderConfig, AuthSuccessResponse, ClientWsPacket, CreateSessionRequest,
    JoinSessionRequest, LeaveSessionRequest, LoginRequest, LogoutRequest, ParticipantKind,
    RegisterRequest, SessionEnterResponse, SessionMode, SessionRole, WebRtcEventKind,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlVideoElement};

/// Populates the login form inputs and default session mode/role for a preset demo account.
#[wasm_bindgen]
pub fn satya_fill_demo(username: String, password: String, display_name: String) {
    set_input_value("auth-username", &username);
    set_input_value("auth-password", &password);
    set_input_value("auth-display-name", &display_name);
    if username.starts_with("gm_") {
        let cur_mode = select_value("session-mode-select");
        let target_mode = if cur_mode == "TTRPG_GM_AI" {
            "TTRPG_GM_AI"
        } else {
            set_select_value("session-mode-select", "TTRPG");
            "TTRPG"
        };
        let ai_enabled = is_ai_gm_mode(target_mode);
        sync_role_options_for_mode(target_mode, Some("GM"));
        set_input_checked("enable-ai-gm", ai_enabled);
        set_display(
            "ai-gm-config-fields",
            if ai_enabled { "grid" } else { "none" },
        );
        set_display("ttrpg-drawer", "flex");
        set_display("tabletop-canvas-card", "flex");
        sync_chat_options_for_mode(SessionMode::Ttrpg);
    } else if username.starts_with("host_") {
        set_select_value("session-mode-select", "MEETING");
        sync_role_options_for_mode("MEETING", Some("HOST"));
        set_input_checked("enable-ai-gm", false);
        set_display("ai-gm-config-fields", "none");
        set_display("ttrpg-drawer", "none");
        set_display("tabletop-canvas-card", "none");
        sync_chat_options_for_mode(SessionMode::Meeting);
    } else {
        let cur_mode = select_value("session-mode-select");
        let target_mode = if cur_mode == "MEETING" {
            set_select_value("session-mode-select", "TTRPG");
            set_input_checked("enable-ai-gm", false);
            set_display("ai-gm-config-fields", "none");
            set_display("ttrpg-drawer", "flex");
            set_display("tabletop-canvas-card", "flex");
            sync_chat_options_for_mode(SessionMode::Ttrpg);
            "TTRPG".to_string()
        } else {
            cur_mode
        };
        sync_role_options_for_mode(&target_mode, Some("PLAYER"));
    }
}

/// Updates the default session role, chat dropdown options, and TTRPG task resolution drawer when business mode changes.
#[wasm_bindgen]
pub fn satya_mode_changed() {
    let mode = select_value("session-mode-select");
    if mode == "MEETING" {
        sync_role_options_for_mode("MEETING", Some("HOST"));
        set_input_checked("enable-ai-gm", false);
        set_display("ai-gm-config-fields", "none");
        set_display("ttrpg-drawer", "none");
        set_display("tabletop-canvas-card", "none");
        sync_chat_options_for_mode(SessionMode::Meeting);
    } else if mode == "TTRPG_AI_GM" {
        sync_role_options_for_mode("TTRPG_AI_GM", Some("PLAYER"));
        set_input_checked("enable-ai-gm", true);
        set_display("ai-gm-config-fields", "grid");
        set_display("ttrpg-drawer", "flex");
        set_display("tabletop-canvas-card", "flex");
        sync_chat_options_for_mode(SessionMode::Ttrpg);
    } else if mode == "TTRPG_GM_AI" {
        sync_role_options_for_mode("TTRPG_GM_AI", Some("GM"));
        set_input_checked("enable-ai-gm", true);
        set_display("ai-gm-config-fields", "grid");
        set_display("ttrpg-drawer", "flex");
        set_display("tabletop-canvas-card", "flex");
        sync_chat_options_for_mode(SessionMode::Ttrpg);
    } else {
        sync_role_options_for_mode("TTRPG", Some("GM"));
        set_input_checked("enable-ai-gm", false);
        set_display("ai-gm-config-fields", "none");
        set_display("ttrpg-drawer", "flex");
        set_display("tabletop-canvas-card", "flex");
        sync_chat_options_for_mode(SessionMode::Ttrpg);
    }
}

/// Registers a new persistent user account via `POST /api/auth/register` and logs in automatically.
#[wasm_bindgen]
pub fn satya_register() {
    spawn_local(async move {
        let username = input_value("auth-username").trim().to_string();
        let password = input_value("auth-password");
        let raw_display = input_value("auth-display-name").trim().to_string();
        let display_name = if raw_display.is_empty() {
            username.clone()
        } else {
            raw_display
        };

        let req = RegisterRequest {
            username,
            password,
            display_name,
        };
        match fetch_json::<_, AuthSuccessResponse>("POST", "/api/auth/register", None, Some(&req))
            .await
        {
            Ok(data) => {
                set_feedback(
                    "auth-feedback",
                    &format!("Registered & authenticated user_id: {}", data.user_id),
                    false,
                );
                apply_auth_success(data);
            }
            Err(err) => set_feedback("auth-feedback", &err, true),
        }
    });
}

/// Authenticates the user with the credentials in `#auth-username` and `#auth-password` via `POST /api/auth/login`.
#[wasm_bindgen]
pub fn satya_login() {
    spawn_local(async move {
        let _ = perform_login().await;
    });
}

pub(crate) async fn perform_login() -> bool {
    let username = input_value("auth-username").trim().to_string();
    let password = input_value("auth-password");
    let req = LoginRequest { username, password };
    match fetch_json::<_, AuthSuccessResponse>("POST", "/api/auth/login", None, Some(&req)).await {
        Ok(data) => {
            set_feedback(
                "auth-feedback",
                &format!(
                    "Authenticated as {} (user_id: {})",
                    data.username, data.user_id
                ),
                false,
            );
            apply_auth_success(data);
            true
        }
        Err(err) => {
            set_feedback("auth-feedback", &err, true);
            false
        }
    }
}

pub(crate) fn apply_auth_success(data: AuthSuccessResponse) {
    let short_id: String = data.user_id.chars().take(8).collect();
    let is_admin = data.account_role == AccountRole::Admin;
    let role_badge = data.account_role.as_db_str();
    set_text(
        "auth-status-pill",
        &format!("User: {} [{role_badge}] ({short_id}...)", data.username),
    );
    set_class("auth-status-pill", "pill pill-active");
    set_button_disabled("btn-logout", false);
    set_button_disabled("btn-menu-logout", false);

    // Hide the Authentication pane once logged in and reveal the Session & Workspace panes
    set_display("auth-card", "none");
    set_display("session-card", "block");
    set_display("workspace-panel", "grid");
    set_display("hamburger-menu-dropdown", "none");
    set_text("btn-menu-toggle-auth-pane", "Show Auth Pane");
    set_text("btn-menu-toggle-session-pane", "Hide Session Pane");

    if let Some(details) = get_element("menu-user-details") {
        details.set_inner_html(&format!(
            r#"<div><strong>{}</strong> (@{})</div>
            <div>Role: <span class="mono">{role_badge}</span></div>
            <div>ID: <span class="mono">{}</span></div>"#,
            escape_html(&data.display_name),
            escape_html(&data.username),
            escape_html(&data.user_id)
        ));
    }

    local_storage_set(AUTH_TOKEN_STORAGE_KEY, &data.auth_token);
    with_state_mut(|st| {
        st.auth_token = Some(data.auth_token.clone());
        st.user = Some(data);
    });
    sync_google_auth_status_from_server();
    satya_refresh_saved_sessions();

    if is_admin {
        set_display("iggy-non-admin-notice", "none");
        set_display("admin-inspector-card", "flex");
        satya_refresh_metrics();
    } else {
        set_display("iggy-non-admin-notice", "block");
        set_display("admin-inspector-card", "none");
        if let Some(wrap) = get_element("topic-metrics-table") {
            wrap.set_inner_html("");
        }
    }
}

pub(crate) fn set_session_buttons_enabled(in_session: bool) {
    set_button_disabled("btn-leave-session", !in_session);
    set_button_disabled("btn-workspace-disconnect", !in_session);
}

/// Creates a new password-protected session via `POST /api/sessions/create` and opens `/ws/session`.
#[wasm_bindgen]
pub fn satya_create_session() {
    spawn_local(async move {
        let has_token = with_state(|st| st.auth_token.is_some());
        if !has_token && !perform_login().await {
            return;
        }
        let Some(token) = with_state(|st| st.auth_token.clone()) else {
            return;
        };

        let session_id = input_value("session-id-input").trim().to_string();
        let session_password = input_value("session-password-input");
        let mode_raw = select_value("session-mode-select");
        let session_mode = parse_session_mode(&mode_raw);
        let role = parse_session_role(&select_value("session-role-select"));
        let enable_ai_gm = is_ai_gm_mode(&mode_raw) || input_checked("enable-ai-gm");
        let display_raw = input_value("auth-display-name").trim().to_string();
        let display_name = if display_raw.is_empty() {
            None
        } else {
            Some(display_raw)
        };

        let ai_gm_config = if enable_ai_gm {
            let provider = select_value("ai-gm-provider-select");
            let provider = if provider.is_empty() {
                "ollama".to_string()
            } else {
                provider
            };
            let model_raw = input_value("ai-gm-model-input").trim().to_string();
            let model_id = if model_raw.is_empty() {
                default_model_for_provider(&provider).to_string()
            } else {
                model_raw
            };
            let key_raw = input_value("ai-gm-api-key-input").trim().to_string();
            // Scrub ephemeral BYOK password field from DOM immediately after reading
            set_input_value("ai-gm-api-key-input", "");
            let api_key = if key_raw.is_empty() {
                None
            } else {
                Some(key_raw)
            };
            Some(AiGmProviderConfig {
                provider,
                model_id,
                vision_model_id: None,
                temperature: 0.35,
                persona_id: "harsh-reality-arbiter".to_string(),
                api_key,
                base_url: None,
            })
        } else {
            None
        };

        let req = CreateSessionRequest {
            session_id,
            session_password,
            session_mode,
            role,
            display_name,
            enable_ai_gm,
            ai_gm_config,
        };

        match fetch_json::<_, SessionEnterResponse>(
            "POST",
            "/api/sessions/create",
            Some(&token),
            Some(&req),
        )
        .await
        {
            Ok(data) => {
                set_feedback(
                    "session-feedback",
                    &format!(
                        "Created session '{}' ({:?})",
                        data.session_id, data.session_mode
                    ),
                    false,
                );
                enter_session(data);
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Joins an existing password-protected session via `POST /api/sessions/join` and opens `/ws/session`.
#[wasm_bindgen]
pub fn satya_join_session() {
    spawn_local(async move {
        let has_token = with_state(|st| st.auth_token.is_some());
        if !has_token && !perform_login().await {
            return;
        }
        let Some(token) = with_state(|st| st.auth_token.clone()) else {
            return;
        };

        let session_id = input_value("session-id-input").trim().to_string();
        let session_password = input_value("session-password-input");
        let role = parse_session_role(&select_value("session-role-select"));
        let display_raw = input_value("auth-display-name").trim().to_string();
        let display_name = if display_raw.is_empty() {
            None
        } else {
            Some(display_raw)
        };

        let req = JoinSessionRequest {
            session_id,
            session_password,
            role,
            display_name,
        };

        match fetch_json::<_, SessionEnterResponse>(
            "POST",
            "/api/sessions/join",
            Some(&token),
            Some(&req),
        )
        .await
        {
            Ok(data) => {
                set_feedback(
                    "session-feedback",
                    &format!(
                        "Joined session '{}' as {:?}",
                        data.session_id, data.self_participant.role
                    ),
                    false,
                );
                enter_session(data);
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

pub(crate) fn enter_session(data: SessionEnterResponse) {
    set_text(
        "session-status-pill",
        &format!(
            "Session: {} [{:?} / {:?}]",
            data.session_id, data.session_mode, data.self_participant.role
        ),
    );
    set_class("session-status-pill", "pill pill-active");
    set_text(
        "local-participant-label",
        &format!(
            "{} ({:?})",
            data.self_participant.display_name, data.self_participant.role
        ),
    );

    if let Ok(active_json) = serde_json::to_string(&PersistedActiveSessionRef {
        session_id: data.session_id.clone(),
        join_token: Some(data.join_token.clone()),
        role: Some(data.self_participant.role),
    }) {
        local_storage_set(ACTIVE_SESSION_STORAGE_KEY, &active_json);
    }

    let session_mode = data.session_mode;
    let is_gm = data.self_participant.role == SessionRole::Gm;
    let history = data.history;
    with_state_mut(|st| {
        st.session_id = Some(data.session_id);
        st.session_mode = session_mode;
        st.join_token = Some(data.join_token);
        st.self_participant = Some(data.self_participant);
        st.roster = data.roster;
        st.selected_recipients.clear();
    });

    sync_gm_context_button_visibility(is_gm);
    sync_chat_options_for_mode(session_mode);
    set_display("session-card", "none");
    set_display("hamburger-menu-dropdown", "none");
    set_text("btn-menu-toggle-session-pane", "Show Session Pane");
    set_display(
        "ttrpg-drawer",
        if session_mode == SessionMode::Meeting {
            "none"
        } else {
            "flex"
        },
    );
    set_display(
        "tabletop-canvas-card",
        if session_mode == SessionMode::Meeting {
            "none"
        } else {
            "flex"
        },
    );
    render_roster();
    set_session_buttons_enabled(true);
    replay_session_history_snapshot(history);
    connect_signaling_websocket();
    satya_refresh_saved_sessions();
    satya_refresh_metrics();
}

/// Closes all active WebRTC peer connections, DataChannels, local media tracks, and signaling WebSockets,
/// resetting the client's active session state and UI controls.
pub fn cleanup_client_session_state() {
    let remote_ids: Vec<String> = with_state_mut(|st| {
        let ids: Vec<String> = st.peers.keys().cloned().collect();
        for (_, peer) in st.peers.drain() {
            if let Some(dc) = peer.dc {
                dc.close();
            }
            peer.pc.close();
        }
        if let Some(stream) = st.local_stream.take() {
            let tracks = stream.get_tracks();
            for i in 0..tracks.length() {
                if let Ok(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
                    track.stop();
                }
            }
        }
        if let Some(ws) = st.ws.take() {
            let _ = ws.close();
        }
        st.session_id = None;
        st.join_token = None;
        st.self_participant = None;
        st.roster.clear();
        st.selected_recipients.clear();
        ids
    });

    for uid in remote_ids {
        if let Some(tile) = get_element(&format!("video-tile-{uid}")) {
            tile.remove();
        }
    }

    if let Some(video) =
        get_element("local-video").and_then(|e| e.dyn_into::<HtmlVideoElement>().ok())
    {
        video.set_src_object(None);
    }
    set_text("local-participant-label", "Local Preview");
    sync_gm_context_button_visibility(false);
    render_roster();
    set_session_buttons_enabled(false);
    set_text("session-status-pill", "No Active Session");
    set_class("session-status-pill", "pill pill-muted");
}

/// Disconnects the current user from the active session, emitting `LEAVE_SESSION` and calling `POST /api/sessions/leave`.
#[wasm_bindgen]
pub fn satya_leave_session() {
    spawn_local(async move {
        let (session_id, auth_token, peer_ids, audio_muted, video_muted) = with_state(|st| {
            (
                st.session_id.clone(),
                st.auth_token.clone(),
                st.peers.keys().cloned().collect::<Vec<_>>(),
                st.audio_muted,
                st.video_muted,
            )
        });

        let Some(departing_sid) = session_id else {
            set_feedback(
                "session-feedback",
                "Not currently connected to an active session",
                true,
            );
            return;
        };

        for remote_uid in peer_ids {
            send_ws_packet(&ClientWsPacket::WebrtcTelemetry {
                target_user_id: Some(remote_uid),
                event_kind: WebRtcEventKind::DataChannelClosed,
                audio_muted,
                video_muted,
            });
        }

        send_ws_packet(&ClientWsPacket::LeaveSession {
            reason: Some("User disconnected from session via Rust/Wasm UI".to_string()),
        });

        if let Some(tok) = auth_token {
            let req = LeaveSessionRequest {
                session_id: departing_sid.clone(),
                reason: Some("User disconnected from session via Rust/Wasm UI".to_string()),
            };
            let _ = fetch_json::<_, serde_json::Value>(
                "POST",
                "/api/sessions/leave",
                Some(&tok),
                Some(&req),
            )
            .await;
        }

        local_storage_remove(ACTIVE_SESSION_STORAGE_KEY);
        cleanup_client_session_state();
        satya_refresh_saved_sessions();
        set_display("session-card", "block");
        set_text("btn-menu-toggle-session-pane", "Hide Session Pane");
        set_feedback(
            "session-feedback",
            &format!("Disconnected from session '{departing_sid}'"),
            false,
        );
        append_system_notice(&format!(
            "Disconnected from session '{departing_sid}'. Logged PARTICIPANT_LEFT & PEER_DISCONNECTED events."
        ));
        satya_refresh_metrics();
    });
}

/// Logs out the currently authenticated user via `POST /api/auth/logout`, revoking their bearer token
/// and disconnecting them from any active session.
#[wasm_bindgen]
pub fn satya_logout() {
    spawn_local(async move {
        let (auth_token, username, active_sid, peer_ids, audio_muted, video_muted) =
            with_state(|st| {
                (
                    st.auth_token.clone(),
                    st.user
                        .as_ref()
                        .map(|u| u.username.clone())
                        .unwrap_or_else(|| "user".to_string()),
                    st.session_id.clone(),
                    st.peers.keys().cloned().collect::<Vec<_>>(),
                    st.audio_muted,
                    st.video_muted,
                )
            });

        let Some(tok) = auth_token else {
            set_feedback("auth-feedback", "No user is currently logged in", true);
            return;
        };

        for remote_uid in peer_ids {
            send_ws_packet(&ClientWsPacket::WebrtcTelemetry {
                target_user_id: Some(remote_uid),
                event_kind: WebRtcEventKind::DataChannelClosed,
                audio_muted,
                video_muted,
            });
        }

        let req = LogoutRequest {
            session_id: active_sid,
            reason: Some("User logged out of session via Rust/Wasm UI".to_string()),
        };
        let _ =
            fetch_json::<_, serde_json::Value>("POST", "/api/auth/logout", Some(&tok), Some(&req))
                .await;

        local_storage_remove(AUTH_TOKEN_STORAGE_KEY);
        local_storage_remove(ACTIVE_SESSION_STORAGE_KEY);
        cleanup_client_session_state();
        with_state_mut(|st| {
            st.auth_token = None;
            st.user = None;
        });

        set_text("auth-status-pill", "Not Logged In");
        set_class("auth-status-pill", "pill pill-muted");
        set_button_disabled("btn-logout", true);
        set_button_disabled("btn-menu-logout", true);
        set_display("auth-card", "block");
        set_display("session-card", "none");
        set_display("workspace-panel", "none");
        set_display("hamburger-menu-dropdown", "none");
        set_text("btn-menu-toggle-auth-pane", "Hide Auth Pane");
        set_text("btn-menu-toggle-session-pane", "Show Session Pane");
        set_text("menu-user-details", "Not Logged In");
        set_display("iggy-non-admin-notice", "block");
        set_display("admin-inspector-card", "none");
        if let Some(wrap) = get_element("topic-metrics-table") {
            wrap.set_inner_html("");
        }

        set_feedback(
            "auth-feedback",
            &format!("Logged out user '{username}'"),
            false,
        );
        set_feedback("session-feedback", "", false);
        append_system_notice(&format!(
            "User '{username}' logged out. Logged USER_LOGGED_OUT lifecycle event."
        ));
    });
}

pub(crate) fn render_roster() {
    let Some(doc) = document() else {
        return;
    };
    let (Some(list_el), Some(recip_el)) = (
        get_element("roster-list"),
        get_element("recipient-checkboxes"),
    ) else {
        return;
    };

    list_el.set_inner_html("");
    recip_el.set_inner_html("");

    let (roster, self_uid, selected_recips) = with_state_mut(|st| {
        let self_id = st.self_participant.as_ref().map(|p| p.user_id.clone());
        st.selected_recipients.retain(|uid| {
            self_id.as_deref() != Some(uid.as_str()) && st.roster.iter().any(|p| &p.user_id == uid)
        });
        (st.roster.clone(), self_id, st.selected_recipients.clone())
    });

    let broadcast_checked = selected_recips.is_empty();
    if let Ok(b_label) = doc.create_element("label") {
        b_label.set_id("recipient-row-broadcast");
        b_label.set_class_name(if broadcast_checked {
            "recip-row broadcast-row selected"
        } else {
            "recip-row broadcast-row"
        });
        if let Ok(cb_el) = doc.create_element("input")
            && let Ok(cb) = cb_el.dyn_into::<HtmlInputElement>()
        {
            cb.set_id("recipient-cb-broadcast");
            cb.set_type("checkbox");
            cb.set_value("BROADCAST");
            cb.set_checked(broadcast_checked);
            let _ = cb.set_attribute("onchange", "window.satyaSelectBroadcast()");
            let on_chg = Closure::<dyn FnMut()>::new(move || {
                satya_select_broadcast();
            });
            cb.set_onchange(Some(on_chg.as_ref().unchecked_ref()));
            on_chg.forget();
            let _ = b_label.append_child(&cb);
        }
        if let Ok(span) = doc.create_element("span") {
            span.set_class_name("recip-name broadcast-tag");
            span.set_text_content(Some("BROADCAST (All Participants)"));
            let _ = b_label.append_child(&span);
        }
        let _ = recip_el.append_child(&b_label);
    }

    let roster_count = roster.len().max(1);
    set_text("webcam-drawer-count-badge", &roster_count.to_string());

    for p in roster {
        if let Ok(chip) = doc.create_element("span") {
            let class_name = if p.participant_kind == ParticipantKind::AiAgent {
                "participant-chip ai"
            } else {
                "participant-chip"
            };
            chip.set_class_name(class_name);
            let short_id: String = p.user_id.chars().take(8).collect();
            chip.set_text_content(Some(&format!(
                "{} [{:?}] ({short_id})",
                p.display_name, p.role
            )));
            let _ = list_el.append_child(&chip);
        }

        if self_uid.as_deref() != Some(p.user_id.as_str())
            && let Ok(label) = doc.create_element("label")
        {
            let is_checked = selected_recips.contains(&p.user_id);
            label.set_class_name(if is_checked {
                "recip-row selected"
            } else {
                "recip-row"
            });
            let _ = label.set_attribute("data-recipient-row-uid", &p.user_id);
            if let Ok(cb_el) = doc.create_element("input")
                && let Ok(cb) = cb_el.dyn_into::<HtmlInputElement>()
            {
                let uid_for_attr = p.user_id.clone();
                let uid_for_cb = p.user_id.clone();
                cb.set_type("checkbox");
                cb.set_class_name("recipient-user-cb");
                cb.set_value(&p.user_id);
                cb.set_checked(is_checked);
                let _ = cb.set_attribute("data-recipient-uid", &p.user_id);
                let _ = cb.set_attribute(
                    "onchange",
                    &format!("window.satyaToggleRecipient('{uid_for_attr}')"),
                );
                let on_chg = Closure::<dyn FnMut()>::new(move || {
                    satya_toggle_recipient(uid_for_cb.clone());
                });
                cb.set_onchange(Some(on_chg.as_ref().unchecked_ref()));
                on_chg.forget();
                let _ = label.append_child(&cb);
            }
            if let Ok(span) = doc.create_element("span") {
                span.set_class_name("recip-name");
                span.set_text_content(Some(&format!("{} ({:?})", p.display_name, p.role)));
                let _ = label.append_child(&span);
            }
            let _ = recip_el.append_child(&label);
        }
    }

    render_combat_tracker();
}
