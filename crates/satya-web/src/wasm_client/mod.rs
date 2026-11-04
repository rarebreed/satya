//! Pure Rust + WebAssembly (`wasm-bindgen` / `web-sys`) client runtime for the Satya
//! WebRTC Audio/Video, Chat, TTRPG Mechanics, Tabbed Notebook, and Apache Iggy Event Streaming platform.
//!
//! ## Submodules
//! - [`state`]: Thread-local [`SatyaWasmState`] and WebRTC [`PeerEntry`] state management.
//! - [`dom`]: Typed `web-sys` DOM query/mutation helpers, JSON `fetch` wrapper, and inline SVG constants.
//! - [`options`]: Pure Rust enum parsers, dropdown `<select>` option tables, and AI GM model defaults.
//! - [`menu`]: Top-bar Hamburger Menu, pane toggles, password visibility toggles, and BYOK AI GM configuration.
//! - [`auth_session`]: User registration, login, logout, demo presets, session create/join/leave, and roster rendering.
//! - [`notebook`]: Left Tabbed Notebook UI (`Maps`, `3D Dice`, `3D Model`, `Character Sheet/Status`, `Combat Tracker`, `Apache Iggy`).
//! - [`webrtc`]: WebSocket signaling, P2P WebRTC Audio/Video + DataChannel mesh, prosody telemetry, and floating/column Webcam controls.
//! - [`chat_and_ttrpg`]: Targeted Session Chat, slash-command/roll execution, message filtering, and Satya Two-List Cascading (`2d20`) task resolution.
//! - [`telemetry`]: Apache Iggy Topic & Avro Schema Inspector (`#admin-inspector-card`).

pub mod auth_session;
pub mod chat_and_ttrpg;
pub mod dom;
pub mod menu;
pub mod notebook;
pub mod options;
pub mod state;
pub mod telemetry;
pub mod webrtc;

pub use auth_session::*;
pub use chat_and_ttrpg::*;
pub use dom::*;
pub use menu::*;
pub use notebook::*;
pub use options::*;
pub use state::*;
pub use telemetry::*;
pub use webrtc::*;

use crate::wasm_client::dom::{select_value, set_display, window};
use crate::wasm_client::options::{sync_chat_options_for_mode, sync_role_options_for_mode};
use crate::wasm_client::webrtc::handle_webcam_drag_move;
use js_sys::Reflect;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

/// WebAssembly module entrypoint automatically invoked when `/pkg/satya_web.js` initializes.
#[wasm_bindgen(start)]
pub fn start_satya_wasm_client() {
    register_window_bindings();
    let mode_raw = select_value("session-mode-select");
    let mode = parse_session_mode(&mode_raw);
    sync_role_options_for_mode(&mode_raw, None);
    sync_chat_options_for_mode(mode);
    satya_visibility_changed();
    set_display("admin-inspector-card", "none");
}

/// Binds the Rust Wasm handlers onto `window` so both SSR HTML attributes and CSR/hydrated
/// Leptos views invoke the compiled Rust WebAssembly functions directly.
pub fn register_window_bindings() {
    let Some(win) = window() else {
        return;
    };
    let bind = |name: &str, func: &JsValue| {
        let _ = Reflect::set(&win, &JsValue::from_str(name), func);
    };

    let c_fill = Closure::<dyn Fn(String, String, String)>::new(satya_fill_demo);
    bind("satyaFillDemo", c_fill.as_ref());
    c_fill.forget();

    let c_mode = Closure::<dyn Fn()>::new(satya_mode_changed);
    bind("satyaModeChanged", c_mode.as_ref());
    c_mode.forget();

    let c_reg = Closure::<dyn Fn()>::new(satya_register);
    bind("satyaRegister", c_reg.as_ref());
    c_reg.forget();

    let c_login = Closure::<dyn Fn()>::new(satya_login);
    bind("satyaLogin", c_login.as_ref());
    c_login.forget();

    let c_logout = Closure::<dyn Fn()>::new(satya_logout);
    bind("satyaLogout", c_logout.as_ref());
    c_logout.forget();

    let c_create = Closure::<dyn Fn()>::new(satya_create_session);
    bind("satyaCreateSession", c_create.as_ref());
    c_create.forget();

    let c_join = Closure::<dyn Fn()>::new(satya_join_session);
    bind("satyaJoinSession", c_join.as_ref());
    c_join.forget();

    let c_leave = Closure::<dyn Fn()>::new(satya_leave_session);
    bind("satyaLeaveSession", c_leave.as_ref());
    c_leave.forget();

    let c_cam = Closure::<dyn Fn()>::new(satya_start_camera);
    bind("satyaStartCamera", c_cam.as_ref());
    c_cam.forget();

    let c_aud = Closure::<dyn Fn()>::new(satya_toggle_audio);
    bind("satyaToggleAudio", c_aud.as_ref());
    c_aud.forget();

    let c_vid = Closure::<dyn Fn()>::new(satya_toggle_video);
    bind("satyaToggleVideo", c_vid.as_ref());
    c_vid.forget();

    let c_vad = Closure::<dyn Fn()>::new(satya_emit_prosody_sample);
    bind("satyaEmitProsodySample", c_vad.as_ref());
    c_vad.forget();

    let c_vis = Closure::<dyn Fn()>::new(satya_visibility_changed);
    bind("satyaVisibilityChanged", c_vis.as_ref());
    c_vis.forget();

    let c_key = Closure::<dyn Fn(KeyboardEvent)>::new(satya_chat_keydown);
    bind("satyaChatKeydown", c_key.as_ref());
    c_key.forget();

    let c_chat = Closure::<dyn Fn()>::new(satya_send_chat);
    bind("satyaSendChat", c_chat.as_ref());
    c_chat.forget();

    let c_roll = Closure::<dyn Fn()>::new(satya_roll_task);
    bind("satyaRollTask", c_roll.as_ref());
    c_roll.forget();

    let c_metrics = Closure::<dyn Fn()>::new(satya_refresh_metrics);
    bind("satyaRefreshMetrics", c_metrics.as_ref());
    c_metrics.forget();

    let c_flush = Closure::<dyn Fn()>::new(satya_flush_batches);
    bind("satyaFlushBatches", c_flush.as_ref());
    c_flush.forget();

    let c_pw = Closure::<dyn Fn(String, String)>::new(satya_toggle_password);
    bind("satyaTogglePassword", c_pw.as_ref());
    c_pw.forget();

    let c_filter = Closure::<dyn Fn()>::new(satya_filter_chat);
    bind("satyaFilterChat", c_filter.as_ref());
    c_filter.forget();

    let c_menu = Closure::<dyn Fn()>::new(satya_toggle_hamburger_menu);
    bind("satyaToggleHamburgerMenu", c_menu.as_ref());
    c_menu.forget();

    let c_auth_pane = Closure::<dyn Fn()>::new(satya_toggle_auth_pane);
    bind("satyaToggleAuthPane", c_auth_pane.as_ref());
    c_auth_pane.forget();

    let c_session_pane = Closure::<dyn Fn()>::new(satya_toggle_session_pane);
    bind("satyaToggleSessionPane", c_session_pane.as_ref());
    c_session_pane.forget();

    let c_settings = Closure::<dyn Fn()>::new(satya_toggle_settings_menu);
    bind("satyaToggleSettingsMenu", c_settings.as_ref());
    c_settings.forget();

    let c_chat_settings = Closure::<dyn Fn()>::new(satya_toggle_chat_settings);
    bind("satyaToggleChatSettings", c_chat_settings.as_ref());
    c_chat_settings.forget();

    let c_ai_gm_settings = Closure::<dyn Fn()>::new(satya_toggle_ai_gm_settings);
    bind("satyaToggleAiGmSettings", c_ai_gm_settings.as_ref());
    c_ai_gm_settings.forget();

    let c_ai_gm_prov = Closure::<dyn Fn()>::new(satya_ai_gm_provider_changed);
    bind("satyaAiGmProviderChanged", c_ai_gm_prov.as_ref());
    c_ai_gm_prov.forget();

    let c_menu_ai_gm_prov = Closure::<dyn Fn()>::new(satya_menu_ai_gm_provider_changed);
    bind("satyaMenuAiGmProviderChanged", c_menu_ai_gm_prov.as_ref());
    c_menu_ai_gm_prov.forget();

    let c_update_gm_cfg = Closure::<dyn Fn()>::new(satya_update_session_gm_config);
    bind("satyaUpdateSessionGmConfig", c_update_gm_cfg.as_ref());
    c_update_gm_cfg.forget();

    let c_bcast = Closure::<dyn Fn()>::new(satya_select_broadcast);
    bind("satyaSelectBroadcast", c_bcast.as_ref());
    c_bcast.forget();

    let c_recip = Closure::<dyn Fn(String)>::new(satya_toggle_recipient);
    bind("satyaToggleRecipient", c_recip.as_ref());
    c_recip.forget();

    let c_ctx = Closure::<dyn Fn(String)>::new(satya_select_chat_context);
    bind("satyaSelectChatContext", c_ctx.as_ref());
    c_ctx.forget();

    let c_cascade_prev = Closure::<dyn Fn()>::new(satya_update_cascade_preview);
    bind("satyaUpdateCascadePreview", c_cascade_prev.as_ref());
    c_cascade_prev.forget();

    let c_cascade_odds = Closure::<dyn Fn()>::new(satya_update_cascade_odds);
    bind("satyaUpdateCascadeOdds", c_cascade_odds.as_ref());
    c_cascade_odds.forget();

    let c_nb_tab = Closure::<dyn Fn(String)>::new(satya_select_notebook_tab);
    bind("satyaSelectNotebookTab", c_nb_tab.as_ref());
    c_nb_tab.forget();

    let c_wc_float = Closure::<dyn Fn()>::new(satya_toggle_webcam_float);
    bind("satyaToggleWebcamFloat", c_wc_float.as_ref());
    c_wc_float.forget();

    let c_wc_drawer = Closure::<dyn Fn()>::new(satya_toggle_webcam_drawer);
    bind("satyaToggleWebcamDrawer", c_wc_drawer.as_ref());
    c_wc_drawer.forget();

    let c_wc_cols = Closure::<dyn Fn()>::new(satya_toggle_webcam_columns);
    bind("satyaToggleWebcamColumns", c_wc_cols.as_ref());
    c_wc_cols.forget();

    let c_wc_drag = Closure::<dyn Fn(web_sys::Event)>::new(satya_start_webcam_drag);
    bind("satyaStartWebcamDrag", c_wc_drag.as_ref());
    c_wc_drag.forget();

    let c_q_dice = Closure::<dyn Fn(String)>::new(satya_quick_3d_dice_roll);
    bind("satyaQuick3dDiceRoll", c_q_dice.as_ref());
    c_q_dice.forget();

    let c_model_view = Closure::<dyn Fn()>::new(satya_update_3d_model_view);
    bind("satyaUpdate3dModelView", c_model_view.as_ref());
    c_model_view.forget();

    let c_skill_preset = Closure::<dyn Fn(String, i32, i32)>::new(satya_load_skill_preset);
    bind("satyaLoadSkillPreset", c_skill_preset.as_ref());
    c_skill_preset.forget();

    let c_combat_tick = Closure::<dyn Fn(i32)>::new(satya_advance_combat_tick);
    bind("satyaAdvanceCombatTick", c_combat_tick.as_ref());
    c_combat_tick.forget();

    let c_google_auth = Closure::<dyn Fn()>::new(satya_link_google_account);
    bind("satyaLinkGoogleAccount", c_google_auth.as_ref());
    c_google_auth.forget();

    let c_google_tok = Closure::<dyn Fn(String)>::new(satya_receive_google_token);
    bind("satyaReceiveGoogleToken", c_google_tok.as_ref());
    c_google_tok.forget();

    let c_google_picked =
        Closure::<dyn Fn(String, String, String, String)>::new(satya_receive_picked_google_doc);
    bind("satyaReceivePickedGoogleDoc", c_google_picked.as_ref());
    c_google_picked.forget();

    let c_doc_file = Closure::<dyn Fn()>::new(satya_gm_doc_file_selected);
    bind("satyaGmDocFileSelected", c_doc_file.as_ref());
    c_doc_file.forget();

    let c_doc_url = Closure::<dyn Fn()>::new(satya_gm_doc_url_changed);
    bind("satyaGmDocUrlChanged", c_doc_url.as_ref());
    c_doc_url.forget();

    let c_doc_oauth_cfg = Closure::<dyn Fn()>::new(satya_toggle_google_oauth_config);
    bind("satyaToggleGoogleOauthConfig", c_doc_oauth_cfg.as_ref());
    c_doc_oauth_cfg.forget();

    let c_doc_picker = Closure::<dyn Fn()>::new(satya_open_google_drive_picker);
    bind("satyaOpenGoogleDrivePicker", c_doc_picker.as_ref());
    c_doc_picker.forget();

    let c_doc_src = Closure::<dyn Fn()>::new(satya_gm_doc_source_changed);
    bind("satyaGmDocSourceChanged", c_doc_src.as_ref());
    c_doc_src.forget();

    let c_doc_preset = Closure::<dyn Fn(String)>::new(satya_load_gm_doc_preset);
    bind("satyaLoadGmDocPreset", c_doc_preset.as_ref());
    c_doc_preset.forget();

    let c_doc_submit = Closure::<dyn Fn()>::new(satya_submit_gm_document);
    bind("satyaSubmitGmDocument", c_doc_submit.as_ref());
    c_doc_submit.forget();

    let c_doc_refresh = Closure::<dyn Fn()>::new(satya_refresh_gm_documents);
    bind("satyaRefreshGmDocuments", c_doc_refresh.as_ref());
    c_doc_refresh.forget();

    let c_rules_chap = Closure::<dyn Fn(String)>::new(satya_select_rules_chapter);
    bind("satyaSelectRulesChapter", c_rules_chap.as_ref());
    c_rules_chap.forget();

    let c_rules_search = Closure::<dyn Fn()>::new(satya_search_rules_book);
    bind("satyaSearchRulesBook", c_rules_search.as_ref());
    c_rules_search.forget();

    let c_rules_ex = Closure::<dyn Fn(String)>::new(satya_load_rules_example);
    bind("satyaLoadRulesExample", c_rules_ex.as_ref());
    c_rules_ex.forget();

    let c_rules_ch = Closure::<dyn Fn(String)>::new(satya_select_rules_chapter);
    bind("satyaSelectRulesChapter", c_rules_ch.as_ref());
    c_rules_ch.forget();

    let c_rules_search = Closure::<dyn Fn()>::new(satya_search_rules_book);
    bind("satyaSearchRulesBook", c_rules_search.as_ref());
    c_rules_search.forget();

    let c_rules_ex = Closure::<dyn Fn(String)>::new(satya_load_rules_example);
    bind("satyaLoadRulesExample", c_rules_ex.as_ref());
    c_rules_ex.forget();

    let on_move = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
        handle_webcam_drag_move(&ev);
    });
    let _ = win.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
    on_move.forget();

    let on_up = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
        with_state_mut(|st| {
            st.webcam_drag_offset = None;
        });
    });
    let _ = win.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
    on_up.forget();
}
