//! User registration, login, logout, demo presets, password-protected session create/join/leave
//! lifecycle, and session participant roster rendering.

use crate::wasm_client::chat_and_ttrpg::{append_system_notice, sync_gm_context_button_visibility};
use crate::wasm_client::dom::{
    escape_html, escape_js_single_quoted_attr, fetch_json, get_element, input_checked, input_value,
    local_storage_remove, local_storage_set, select_value, set_button_disabled, set_class,
    set_display, set_feedback, set_html, set_input_checked, set_input_value, set_select_value,
    set_text, window,
};
use crate::wasm_client::notebook::{render_combat_tracker, sync_google_auth_status_from_server};
use crate::wasm_client::options::{
    default_model_for_provider, is_ai_gm_mode, parse_session_mode, parse_session_role,
    sync_chat_options_for_mode, sync_role_options_for_mode,
};
use crate::wasm_client::session_persistence::{
    ACTIVE_SESSION_STORAGE_KEY, AUTH_TOKEN_STORAGE_KEY, PersistedActiveSessionRef,
    replay_session_history_snapshot, satya_refresh_saved_sessions,
};
use crate::wasm_client::state::{
    CampaignSelectionKey, LobbyScope, SessionSelectionKey, with_state, with_state_mut,
};
use crate::wasm_client::telemetry::satya_refresh_metrics;
use crate::wasm_client::webrtc::{
    connect_signaling_websocket, send_ws_packet, teardown_prosody_monitor,
};
use satya_schema::{
    AccountRecoveryRequest, AccountRecoveryResponse, AccountRole, AiGmProviderConfig,
    AuthSuccessResponse, ClientWsPacket, CreateSessionRequest, LeaveSessionRequest, LoginRequest,
    LogoutRequest, Participant, ParticipantKind, PasswordResetConfirmRequest,
    PasswordResetConfirmResponse, RegisterRequest, SessionEnterResponse, SessionMode, SessionRole,
    WebRtcEventKind,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlVideoElement;

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
    } else if mode == "TTRPG_SOLO_AI_GM" {
        sync_role_options_for_mode("TTRPG_SOLO_AI_GM", Some("SOLO_GM"));
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

        let email_raw = input_value("auth-email").trim().to_string();
        let email = if email_raw.is_empty() {
            None
        } else {
            Some(email_raw)
        };

        let req = RegisterRequest {
            username,
            password,
            display_name,
            email,
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

/// Extracts a non-empty `reset_token` parameter from a URL query string (`?reset_token=...`).
#[must_use]
pub fn extract_reset_token_from_query(search: &str) -> Option<String> {
    let query = search.trim().strip_prefix('?').unwrap_or(search.trim());
    for pair in query.split('&') {
        if let Some((key, raw_val)) = pair.split_once('=')
            && key == "reset_token"
        {
            let trimmed = raw_val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn reset_recovery_secret_inputs() {
    set_input_value("recovery-token", "");
    set_input_value("recovery-new-password", "");
    set_input_value("recovery-confirm-password", "");
}

/// Inspects `window.location.search` on startup; if `?reset_token=...` is present, opens the
/// recovery panel pre-filled with the token and scrubs the token from the browser URL bar.
pub(crate) fn apply_reset_token_from_url_if_present() -> bool {
    let Some(win) = window() else {
        return false;
    };
    let Ok(search) = win.location().search() else {
        return false;
    };
    let Some(token) = extract_reset_token_from_query(&search) else {
        return false;
    };

    set_display("auth-card", "block");
    set_display("auth-recovery-panel", "flex");
    set_input_value("recovery-token", &token);
    set_feedback(
        "auth-recovery-feedback",
        "Reset link loaded. Enter and confirm your new password below.",
        false,
    );

    if let Ok(history) = js_sys::Reflect::get(&win, &JsValue::from_str("history"))
        && let Ok(replace_fn) = js_sys::Reflect::get(&history, &JsValue::from_str("replaceState"))
            .and_then(|v| v.dyn_into::<js_sys::Function>())
        && let Ok(pathname) = win.location().pathname()
    {
        let _ = replace_fn.call3(
            &history,
            &JsValue::NULL,
            &JsValue::from_str(""),
            &JsValue::from_str(&pathname),
        );
    }
    true
}

/// Toggles the inline Account Recovery panel (`#auth-recovery-panel`) inside `#auth-card`.
#[wasm_bindgen]
pub fn satya_toggle_recovery_panel() {
    let Some(panel) = get_element("auth-recovery-panel")
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
    else {
        return;
    };
    let currently_hidden = panel
        .style()
        .get_property_value("display")
        .map(|v| v == "none" || v.is_empty())
        .unwrap_or(true);
    if currently_hidden {
        set_display("auth-recovery-panel", "flex");
        if input_value("recovery-identifier").trim().is_empty() {
            let email = input_value("auth-email").trim().to_string();
            let username = input_value("auth-username").trim().to_string();
            let prefill = if !email.is_empty() { email } else { username };
            if !prefill.is_empty() {
                set_input_value("recovery-identifier", &prefill);
            }
        }
        set_feedback("auth-recovery-feedback", "", false);
    } else {
        satya_close_recovery_panel();
    }
}

/// Closes `#auth-recovery-panel` and scrubs any recovery token/password inputs from the DOM.
#[wasm_bindgen]
pub fn satya_close_recovery_panel() {
    set_display("auth-recovery-panel", "none");
    reset_recovery_secret_inputs();
    set_feedback("auth-recovery-feedback", "", false);
}

/// Requests a username reminder and password reset token via `POST /api/auth/recovery/request`.
#[wasm_bindgen]
pub fn satya_request_account_recovery() {
    spawn_local(async move {
        let identifier = input_value("recovery-identifier").trim().to_string();
        if identifier.is_empty() {
            set_feedback(
                "auth-recovery-feedback",
                "Please enter your registered email address or username.",
                true,
            );
            return;
        }
        let req = AccountRecoveryRequest { identifier };
        match fetch_json::<_, AccountRecoveryResponse>(
            "POST",
            "/api/auth/recovery/request",
            None,
            Some(&req),
        )
        .await
        {
            Ok(resp) => set_feedback("auth-recovery-feedback", &resp.message, false),
            Err(err) => set_feedback("auth-recovery-feedback", &err, true),
        }
    });
}

/// Confirms a single-use password reset token via `POST /api/auth/recovery/confirm`.
#[wasm_bindgen]
pub fn satya_confirm_password_reset() {
    spawn_local(async move {
        let token = input_value("recovery-token").trim().to_string();
        let new_password = input_value("recovery-new-password");
        let confirm_password = input_value("recovery-confirm-password");

        if token.is_empty() {
            set_feedback(
                "auth-recovery-feedback",
                "Please paste the password reset token from your recovery email.",
                true,
            );
            return;
        }
        if new_password.chars().count() < 8 {
            set_feedback(
                "auth-recovery-feedback",
                "New password must be at least 8 characters.",
                true,
            );
            return;
        }
        if new_password != confirm_password {
            set_feedback(
                "auth-recovery-feedback",
                "New password and confirmation do not match.",
                true,
            );
            return;
        }

        let req = PasswordResetConfirmRequest {
            token,
            new_password,
        };
        match fetch_json::<_, PasswordResetConfirmResponse>(
            "POST",
            "/api/auth/recovery/confirm",
            None,
            Some(&req),
        )
        .await
        {
            Ok(resp) => {
                reset_recovery_secret_inputs();
                set_display("auth-recovery-panel", "none");
                set_feedback("auth-recovery-feedback", "", false);
                set_input_value("auth-username", &resp.username);
                set_input_value("auth-password", "");
                set_feedback("auth-feedback", &resp.message, false);
            }
            Err(err) => set_feedback("auth-recovery-feedback", &err, true),
        }
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
    set_display("btn-open-campaigns-modal", "inline-flex");
    set_display("notifications-bell-wrap", "inline-flex");

    // Hide the Authentication pane once logged in and open the guided Campaign & Session modal
    set_display("auth-card", "none");
    satya_close_recovery_panel();
    set_display("workspace-panel", "grid");
    set_display("hamburger-menu-dropdown", "none");
    set_text("btn-menu-toggle-auth-pane", "Show Auth Pane");

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
        st.lobby_scope = LobbyScope::Mine;
        st.lobby_step = crate::wasm_client::state::LobbyStep::ChooseCampaign;
        st.selected_directory_user_id = None;
        st.selected_campaign_key = CampaignSelectionKey::None;
        st.selected_session_key = SessionSelectionKey::None;
        st.requests_tray_open = false;
        st.show_join_password_box = false;
    });
    crate::wasm_client::menu::open_session_modal();
    sync_google_auth_status_from_server();
    satya_refresh_saved_sessions();
    crate::wasm_client::campaigns_directory::satya_refresh_my_campaigns();
    crate::wasm_client::campaigns_directory::satya_refresh_join_requests();
    crate::wasm_client::campaigns_directory::refresh_account_quotas();
    crate::wasm_client::campaigns_directory::connect_notifications_websocket();
    crate::wasm_client::map_vault::satya_refresh_saved_maps();

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
    set_display(
        "btn-leave-session",
        if in_session { "inline-flex" } else { "none" },
    );
    set_button_disabled("btn-workspace-disconnect", !in_session);
}

/// Creates a new password-protected session inside `campaign_id` from the compact lobby inputs
/// via `POST /api/sessions/create` and enters `/ws/session`.
pub(crate) async fn create_session_in_campaign(
    campaign_id: Option<String>,
    campaign_slug: Option<String>,
) -> bool {
    let has_token = with_state(|st| st.auth_token.is_some());
    if !has_token && !perform_login().await {
        return false;
    }
    let Some(token) = with_state(|st| st.auth_token.clone()) else {
        return false;
    };

    let title_raw = input_value("session-title-input").trim().to_string();
    let title = if title_raw.is_empty() {
        None
    } else {
        Some(title_raw.clone())
    };
    let explicit_sid = input_value("session-id-input").trim().to_string();
    let session_id = if explicit_sid.is_empty() {
        let base_title = if title_raw.is_empty() {
            "session"
        } else {
            &title_raw
        };
        crate::wasm_client::campaigns_directory::derive_session_id_slug(
            campaign_slug.as_deref(),
            base_title,
        )
    } else {
        explicit_sid
    };
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
            clear_api_key: false,
        })
    } else {
        None
    };

    let is_discoverable = input_checked("session-discoverable-checkbox");
    let allow_join_requests = input_checked("session-allow-requests-checkbox");

    let req = CreateSessionRequest {
        session_id,
        session_password,
        session_mode,
        role,
        display_name,
        enable_ai_gm,
        ai_gm_config,
        campaign_id,
        title,
        is_discoverable,
        allow_join_requests,
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
            set_input_value("session-title-input", "");
            set_input_value("session-id-input", "");
            set_input_value("session-password-input", "");
            set_feedback(
                "session-feedback",
                &format!(
                    "Created session '{}' ({:?})",
                    data.session_id, data.session_mode
                ),
                false,
            );
            enter_session(data);
            true
        }
        Err(err) => {
            set_feedback("session-feedback", &err, true);
            false
        }
    }
}

pub(crate) fn enter_session(data: SessionEnterResponse) {
    let switching_room = with_state(|st| {
        st.session_id
            .as_deref()
            .is_some_and(|prev_sid| prev_sid != data.session_id)
    });
    if switching_room {
        cleanup_client_session_state();
    }

    set_text(
        "session-status-pill",
        &format!(
            "Session: {} [{:?} / {:?}]",
            data.session_id, data.session_mode, data.self_participant.role
        ),
    );
    set_class("session-status-pill", "pill pill-active pill-clickable");
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
    let role = data.self_participant.role;
    let is_gm = role.is_human_gm_or_co_gm();
    let is_solo_gm = role == SessionRole::SoloGm;
    let initial_can_see_secrets = role.is_gm_or_co_gm();
    let history = data.history;
    with_state_mut(|st| {
        st.session_id = Some(data.session_id);
        st.session_mode = session_mode;
        st.join_token = Some(data.join_token);
        st.self_participant = Some(data.self_participant);
        st.roster = data.roster;
        st.selected_recipients.clear();
        st.ws_reconnect_suppressed = false;
        st.ws_reconnect_attempts = 0;
        st.ws_reconnect_scheduled = false;
        st.combat_combatants.clear();
        st.solo_spoiler_shield = false;
        st.viewer_can_see_gm_secrets = initial_can_see_secrets;
        st.ai_rally_paused = false;
    });

    set_display(
        "btn-solo-spoiler-shield",
        if is_solo_gm { "inline-flex" } else { "none" },
    );
    set_text("btn-solo-spoiler-shield", "Spoiler Shield: OFF");
    set_class("btn-solo-spoiler-shield", "btn btn-sm btn-secondary");
    sync_gm_context_button_visibility(is_gm);
    crate::wasm_client::notebook::sync_iggy_tab_visibility(is_gm);
    sync_chat_options_for_mode(session_mode);
    crate::wasm_client::menu::satya_close_session_pane();
    set_display("hamburger-menu-dropdown", "none");
    crate::wasm_client::workspace_drawers::sync_workspace_drawers_dom();
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
    crate::wasm_client::notebook::refresh_campaign_lore_catalog();
    connect_signaling_websocket();
    satya_refresh_saved_sessions();
    crate::wasm_client::campaigns_directory::satya_refresh_my_campaigns();
    crate::wasm_client::campaigns_directory::refresh_account_quotas();
    satya_refresh_metrics();
}

/// Toggles the optional Solo Co-GM Spoiler Shield (`SOLO_GM`) between full Co-GM visibility
/// (`OFF`) and player-safe redaction (`ON`) while keeping Co-GM table controls active.
#[wasm_bindgen]
pub fn satya_toggle_solo_spoiler_shield() {
    let update_opt = with_state_mut(|st| {
        if st.self_participant.as_ref().map(|p| p.role) != Some(SessionRole::SoloGm) {
            return None;
        }
        st.solo_spoiler_shield = !st.solo_spoiler_shield;
        st.viewer_can_see_gm_secrets = !st.solo_spoiler_shield;
        Some((
            st.solo_spoiler_shield,
            st.session_id.clone(),
            st.auth_token.clone(),
            st.gm_omnisearch_active,
        ))
    });
    let Some((shielded, sid_opt, tok_opt, omnisearch_active)) = update_opt else {
        return;
    };
    set_text(
        "btn-solo-spoiler-shield",
        if shielded {
            "Spoiler Shield: ON"
        } else {
            "Spoiler Shield: OFF"
        },
    );
    set_class(
        "btn-solo-spoiler-shield",
        if shielded {
            "btn btn-sm btn-warning"
        } else {
            "btn btn-sm btn-secondary"
        },
    );
    set_feedback(
        "session-feedback",
        if shielded {
            "Spoiler Shield enabled: hiding GM-only NPC secrets, GM notes, hidden map tokens, and spoiler campaign lore."
        } else {
            "Spoiler Shield disabled: full Solo Co-GM dossier and world state visible."
        },
        false,
    );
    crate::wasm_client::npcs::satya_refresh_npcs_tab();
    crate::wasm_client::gm_memories_and_director::satya_refresh_gm_memories();
    crate::wasm_client::notebook::refresh_campaign_lore_catalog();
    if omnisearch_active {
        crate::wasm_client::gm_omnisearch::satya_search_gm_omnisearch();
    }
    if let (Some(sid), Some(tok)) = (sid_opt, tok_opt) {
        spawn_local(async move {
            let suffix = if shielded { "?spoiler_shield=true" } else { "" };
            if let Ok(map) = fetch_json::<(), satya_schema::GmMapUpdateRequest>(
                "GET",
                &format!("/api/sessions/{sid}/map{suffix}"),
                Some(&tok),
                None,
            )
            .await
            {
                crate::wasm_client::notebook::render_map_updated(&map);
            }
            if let Ok(ct) = fetch_json::<(), satya_schema::CombatTrackerState>(
                "GET",
                &format!("/api/sessions/{sid}/combat{suffix}"),
                Some(&tok),
                None,
            )
            .await
            {
                let round = ct.round.max(1);
                let tick = ct.tick;
                with_state_mut(|st| {
                    st.combat_round = round;
                    st.combat_tick = tick;
                    st.combat_combatants = ct.combatants;
                });
                render_combat_tracker();
            }
        });
    }
}

/// Closes all active WebRTC peer connections, DataChannels, local media tracks, prosody timers,
/// and signaling WebSockets, resetting the client's active session state and UI controls.
pub fn cleanup_client_session_state() {
    teardown_prosody_monitor();
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
            ws.set_onclose(None);
            ws.set_onerror(None);
            let _ = ws.close();
        }
        st.ws_reconnect_suppressed = true;
        st.ws_reconnect_attempts = 0;
        st.ws_reconnect_scheduled = false;
        st.combat_combatants.clear();
        st.solo_spoiler_shield = false;
        st.viewer_can_see_gm_secrets = false;
        st.ai_rally_paused = false;
        st.gm_documents.clear();
        st.gm_memories.clear();
        st.gm_omnisearch_active = false;
        st.session_npcs.clear();
        st.campaign_pack_name = "awakened".to_string();
        st.campaign_npcs.clear();
        st.campaign_lore_catalog.clear();
        st.reader_source_mode = "RULES".to_string();
        st.last_omnisearch_hits.clear();
        st.archivist_modal_item_id = None;
        st.selected_npc_id = None;
        st.session_id = None;
        st.join_token = None;
        st.self_participant = None;
        st.roster.clear();
        st.selected_recipients.clear();
        ids
    });

    crate::wasm_client::map_vault::reset_map_vault_client_cache();

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
    set_display("btn-solo-spoiler-shield", "none");
    set_display("gm-archivist-modal", "none");
    sync_gm_context_button_visibility(false);
    crate::wasm_client::notebook::sync_iggy_tab_visibility(false);
    render_roster();
    set_session_buttons_enabled(false);
    set_text("session-status-pill", "No Active Session");
    set_class("session-status-pill", "pill pill-muted pill-clickable");
}

/// Disconnects the current user from the active session, emitting `LEAVE_SESSION` and calling `POST /api/sessions/leave`.
#[wasm_bindgen]
pub fn satya_leave_session() {
    spawn_local(async move {
        let (session_id, auth_token, peer_ids, audio_muted, video_muted) = with_state_mut(|st| {
            st.ws_reconnect_suppressed = true;
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
        crate::wasm_client::campaigns_directory::satya_refresh_my_campaigns();
        crate::wasm_client::campaigns_directory::refresh_account_quotas();
        with_state_mut(|st| {
            st.lobby_step = crate::wasm_client::state::LobbyStep::ChooseCampaign;
            st.show_join_password_box = false;
        });
        crate::wasm_client::menu::open_session_modal();
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
            with_state_mut(|st| {
                st.ws_reconnect_suppressed = true;
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
        crate::wasm_client::campaigns_directory::disconnect_notifications_websocket();
        with_state_mut(|st| {
            st.auth_token = None;
            st.user = None;
            st.google_access_token = None;
            st.google_linked = false;
            st.my_campaigns.clear();
            st.saved_sessions.clear();
            st.lobby_scope = LobbyScope::Mine;
            st.lobby_step = crate::wasm_client::state::LobbyStep::ChooseCampaign;
            st.selected_directory_user_id = None;
            st.selected_campaign_key = CampaignSelectionKey::None;
            st.selected_session_key = SessionSelectionKey::None;
            st.requests_tray_open = false;
            st.show_join_password_box = false;
            st.directory_results = None;
            st.incoming_join_requests.clear();
            st.my_join_requests.clear();
            st.quota_usage = None;
            st.notifications_popover_open = false;
        });
        if let Some(win) = window() {
            let _ = js_sys::Reflect::set(
                &win,
                &JsValue::from_str("_satyaGoogleAccessToken"),
                &JsValue::NULL,
            );
            let _ = js_sys::Reflect::delete_property(
                &win,
                &JsValue::from_str("_satyaGoogleAccessToken"),
            );
        }

        set_text("auth-status-pill", "Not Logged In");
        set_class("auth-status-pill", "pill pill-muted");
        set_text("google-auth-status-pill", "Google Unlinked (drive.file)");
        set_class("google-auth-status-pill", "pill pill-muted");
        set_button_disabled("btn-logout", true);
        set_button_disabled("btn-menu-logout", true);
        set_display("auth-card", "block");
        crate::wasm_client::menu::satya_close_session_pane();
        set_display("workspace-panel", "none");
        set_display("hamburger-menu-dropdown", "none");
        set_display("btn-open-campaigns-modal", "none");
        set_display("notifications-bell-wrap", "none");
        set_display("notifications-popover", "none");
        set_display("live-notification-toast", "none");
        set_display("session-requests-tray", "none");
        set_html("directory-user-chips", "");
        set_html("campaign-list-pane", "");
        set_html("session-list-pane", "");
        set_html("lobby-dynamic-action-body", "");
        set_html("gm-join-requests-list", "");
        set_html("my-join-requests-list", "");
        set_input_value("auth-password", "");
        set_input_value("session-password-input", "");
        set_input_value("dir-join-password-input", "");
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

/// Formats `(roster_chips_html, recipient_checkboxes_html)` for `#roster-list` and `#recipient-checkboxes`
/// without allocating per-sync JS closures.
#[must_use]
pub fn format_roster_and_recipients_html(
    roster: &[Participant],
    self_user_id: Option<&str>,
    selected_recipients: &[String],
) -> (String, String) {
    let mut roster_html = String::with_capacity(roster.len() * 112);
    let broadcast_checked = selected_recipients.is_empty();
    let b_row_cls = if broadcast_checked {
        "recip-row broadcast-row selected"
    } else {
        "recip-row broadcast-row"
    };
    let b_chk_attr = if broadcast_checked { " checked" } else { "" };

    let mut recip_html = format!(
        r#"<label id="recipient-row-broadcast" class="{b_row_cls}"><input id="recipient-cb-broadcast" type="checkbox" value="BROADCAST"{b_chk_attr} onchange="window.satyaSelectBroadcast()" /><span class="recip-name broadcast-tag">BROADCAST (All Participants)</span></label>"#
    );

    for p in roster {
        let chip_cls = if p.participant_kind == ParticipantKind::AiAgent {
            "participant-chip ai"
        } else {
            "participant-chip"
        };
        let short_id: String = p.user_id.chars().take(8).collect();
        roster_html.push_str(&format!(
            r#"<span class="{chip_cls}">{} [{:?}] ({})</span>"#,
            escape_html(&p.display_name),
            p.role,
            escape_html(&short_id),
        ));

        if self_user_id != Some(p.user_id.as_str()) {
            let is_checked = selected_recipients.iter().any(|id| id == &p.user_id);
            let row_cls = if is_checked {
                "recip-row selected"
            } else {
                "recip-row"
            };
            let chk_attr = if is_checked { " checked" } else { "" };
            let uid_attr = escape_html(&p.user_id);
            let uid_js = escape_js_single_quoted_attr(&p.user_id);
            recip_html.push_str(&format!(
                r#"<label class="{row_cls}" data-recipient-row-uid="{uid_attr}"><input type="checkbox" class="recipient-user-cb" value="{uid_attr}" data-recipient-uid="{uid_attr}"{chk_attr} onchange="window.satyaToggleRecipient('{uid_js}')" /><span class="recip-name">{} ({:?})</span></label>"#,
                escape_html(&p.display_name),
                p.role,
            ));
        }
    }

    (roster_html, recip_html)
}

pub(crate) fn render_roster() {
    let (roster, self_uid, selected_recips) = with_state_mut(|st| {
        let self_id = st.self_participant.as_ref().map(|p| p.user_id.clone());
        st.selected_recipients.retain(|uid| {
            self_id.as_deref() != Some(uid.as_str()) && st.roster.iter().any(|p| &p.user_id == uid)
        });
        (st.roster.clone(), self_id, st.selected_recipients.clone())
    });

    let (roster_html, recip_html) =
        format_roster_and_recipients_html(&roster, self_uid.as_deref(), &selected_recips);
    set_html("roster-list", &roster_html);
    set_html("recipient-checkboxes", &recip_html);

    let roster_count = roster.len().max(1);
    set_text("webcam-drawer-count-badge", &roster_count.to_string());
    render_combat_tracker();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_roster_and_recipients_html_broadcast_vs_targeted_and_escaping() {
        let roster = vec![
            Participant::human("u-sean-01", "gm_sean", "Sean (GM)", SessionRole::Gm),
            Participant::human("u-sara'02", "sara", "Sara <Valkyrie>", SessionRole::Player),
            Participant::ai_gm("sector-4", "gemma4:12b", "harsh-reality-arbiter"),
        ];

        // 1. Empty selected_recipients => Broadcast row selected, self excluded from recipient list
        let (roster_html, recip_html) =
            format_roster_and_recipients_html(&roster, Some("u-sean-01"), &[]);
        assert!(roster_html.contains("Sean (GM) [Gm] (u-sean-0)"));
        assert!(roster_html.contains("Sara &lt;Valkyrie&gt; [Player]"));
        assert!(roster_html.contains(r#"class="participant-chip ai""#));

        assert!(
            recip_html.contains(
                r#"id="recipient-row-broadcast" class="recip-row broadcast-row selected""#
            )
        );
        assert!(recip_html.contains(r#"onchange="window.satyaSelectBroadcast()""#));
        assert!(!recip_html.contains(r#"data-recipient-uid="u-sean-01""#));
        assert!(recip_html.contains(r#"window.satyaToggleRecipient('u-sara\'02')"#));

        // 2. Selecting Sara unchecks broadcast and marks Sara's row selected
        let (_, recip_sel) = format_roster_and_recipients_html(
            &roster,
            Some("u-sean-01"),
            &["u-sara'02".to_string()],
        );
        assert!(
            recip_sel.contains(r#"id="recipient-row-broadcast" class="recip-row broadcast-row""#)
        );
        assert!(
            !recip_sel.contains(
                r#"id="recipient-cb-broadcast" type="checkbox" value="BROADCAST" checked"#
            )
        );
        assert!(
            recip_sel
                .contains(r#"class="recip-row selected" data-recipient-row-uid="u-sara&#39;02""#)
        );
        assert!(recip_sel.contains(r#"data-recipient-uid="u-sara&#39;02" checked"#));
    }

    #[test]
    fn test_extract_reset_token_from_query() {
        assert_eq!(extract_reset_token_from_query(""), None);
        assert_eq!(extract_reset_token_from_query("?foo=bar"), None);
        assert_eq!(extract_reset_token_from_query("?reset_token=   "), None);
        assert_eq!(
            extract_reset_token_from_query("?reset_token=satya_rst_abc123"),
            Some("satya_rst_abc123".to_string())
        );
        assert_eq!(
            extract_reset_token_from_query("?utm=1&reset_token=satya_rst_xyz789&other=2"),
            Some("satya_rst_xyz789".to_string())
        );
    }
}
