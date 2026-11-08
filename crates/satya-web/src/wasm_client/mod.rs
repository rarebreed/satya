//! Pure Rust + WebAssembly (`wasm-bindgen` / `web-sys`) client runtime for the Satya
//! WebRTC Audio/Video, Chat, TTRPG Mechanics, Tabbed Notebook, and Apache Iggy Event Streaming platform.
//!
//! ## Submodules
//! - [`state`]: Thread-local [`SatyaWasmState`] and WebRTC [`PeerEntry`] state management.
//! - [`dom`]: Typed `web-sys` DOM query/mutation helpers, JSON `fetch` wrapper, and inline SVG constants.
//! - [`options`]: Pure Rust enum parsers, dropdown `<select>` option tables, and AI GM model defaults.
//! - [`menu`]: Top-bar Hamburger Menu, pane toggles, password visibility toggles, and BYOK AI GM configuration.
//! - [`auth_session`]: User registration, login, logout, demo presets, session create/join/leave, and roster rendering.
//! - [`notebook`]: Left Tabbed Notebook UI (`Maps`, `3D Dice`, `3D Model`, `Character Sheet/Status`, `Combat Tracker`, `Rules Book`, `Apache Iggy`).
//! - [`gm_docs`]: `GM_ONLY` Document Vault & Google Drive (`drive.file`) OAuth/Picker integration.
//! - [`dice_3d`] & [`dice_3d_wgpu`]: Deterministic 3D polyhedral dice meshes, SDF atlas, physics, and WebGPU (`wgpu`) / WGSL renderer.
//! - [`wizard`], [`wizard_views_stats`], [`wizard_views_lifepath`], [`wizard_views_skills`]: Interactive 6-step Character Creation Wizard.
//! - [`webrtc`]: WebSocket signaling, P2P WebRTC Audio/Video + DataChannel mesh, prosody telemetry, and floating/column Webcam controls.
//! - [`chat_and_ttrpg`]: Targeted Session Chat, slash-command/roll execution, message filtering, and Satya Two-List Cascading (`2d20`) task resolution.
//! - [`telemetry`]: Apache Iggy Topic & Avro Schema Inspector (`#admin-inspector-card`).

pub mod auth_session;
pub mod character_sheet;
pub mod character_sheet_status_views;
pub mod character_sheet_vault;
pub mod character_sheet_views;
pub mod chat_and_ttrpg;
pub mod dice_3d;
pub mod dice_3d_wgpu;
pub mod dom;
pub mod gm_docs;
pub mod menu;
pub mod notebook;
pub mod options;
pub mod state;
pub mod telemetry;
pub mod webrtc;
pub mod wizard;
pub mod wizard_views_lifepath;
pub mod wizard_views_skills;
pub mod wizard_views_stats;

pub use auth_session::*;
pub use character_sheet::*;
pub use chat_and_ttrpg::*;
pub use dice_3d::*;
pub use dom::*;
pub use menu::*;
pub use notebook::*;
pub use options::*;
pub use state::*;
pub use telemetry::*;
pub use webrtc::*;
pub use wizard::*;

use crate::wasm_client::dom::{select_value, set_display, window};
use crate::wasm_client::options::{sync_chat_options_for_mode, sync_role_options_for_mode};
use crate::wasm_client::webrtc::handle_webcam_drag_move;
use js_sys::Reflect;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

/// Declarative macro that binds Rust handler functions onto `window.<jsName>` via
/// `Closure::new` + `Reflect::set` + `forget()`.
macro_rules! bind_window_callbacks {
    ($bind:ident; $( $js_name:literal => $fn_ident:ident ( $($arg_ty:ty),* ) ),* $(,)?) => {
        $(
            {
                let closure = Closure::<dyn Fn($($arg_ty),*)>::new($fn_ident);
                $bind($js_name, closure.as_ref());
                closure.forget();
            }
        )*
    };
}

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

    bind_window_callbacks!(bind;
        // Auth, Session Gate & Hamburger Menu
        "satyaFillDemo" => satya_fill_demo(String, String, String),
        "satyaModeChanged" => satya_mode_changed(),
        "satyaRegister" => satya_register(),
        "satyaLogin" => satya_login(),
        "satyaLogout" => satya_logout(),
        "satyaCreateSession" => satya_create_session(),
        "satyaJoinSession" => satya_join_session(),
        "satyaLeaveSession" => satya_leave_session(),
        "satyaTogglePassword" => satya_toggle_password(String, String),
        "satyaToggleHamburgerMenu" => satya_toggle_hamburger_menu(),
        "satyaToggleAuthPane" => satya_toggle_auth_pane(),
        "satyaToggleSessionPane" => satya_toggle_session_pane(),
        "satyaToggleSettingsMenu" => satya_toggle_settings_menu(),
        "satyaToggleChatSettings" => satya_toggle_chat_settings(),
        "satyaToggleAiGmSettings" => satya_toggle_ai_gm_settings(),
        "satyaAiGmProviderChanged" => satya_ai_gm_provider_changed(),
        "satyaMenuAiGmProviderChanged" => satya_menu_ai_gm_provider_changed(),
        "satyaUpdateSessionGmConfig" => satya_update_session_gm_config(),

        // WebRTC Media, Drawers & Telemetry
        "satyaStartCamera" => satya_start_camera(),
        "satyaToggleAudio" => satya_toggle_audio(),
        "satyaToggleVideo" => satya_toggle_video(),
        "satyaEmitProsodySample" => satya_emit_prosody_sample(),
        "satyaToggleWebcamFloat" => satya_toggle_webcam_float(),
        "satyaToggleWebcamDrawer" => satya_toggle_webcam_drawer(),
        "satyaToggleChatDrawer" => satya_toggle_chat_drawer(),
        "satyaToggleTtrpgDrawer" => satya_toggle_ttrpg_drawer(),
        "satyaToggleWebcamColumns" => satya_toggle_webcam_columns(),
        "satyaStartWebcamDrag" => satya_start_webcam_drag(web_sys::Event),
        "satyaRefreshMetrics" => satya_refresh_metrics(),
        "satyaFlushBatches" => satya_flush_batches(),

        // Targeted Session Chat & TTRPG Task Resolution
        "satyaVisibilityChanged" => satya_visibility_changed(),
        "satyaChatKeydown" => satya_chat_keydown(KeyboardEvent),
        "satyaSendChat" => satya_send_chat(),
        "satyaRollTask" => satya_roll_task(),
        "satyaFilterChat" => satya_filter_chat(),
        "satyaSelectBroadcast" => satya_select_broadcast(),
        "satyaToggleRecipient" => satya_toggle_recipient(String),
        "satyaSelectChatContext" => satya_select_chat_context(String),
        "satyaUpdateCascadePreview" => satya_update_cascade_preview(),
        "satyaUpdateCascadeOdds" => satya_update_cascade_odds(),
        "satyaFilterTtrpgSkillDropdown" => satya_filter_ttrpg_skill_dropdown(),
        "satyaOpenTtrpgSkillDropdown" => satya_open_ttrpg_skill_dropdown(),
        "satyaToggleTtrpgSkillDropdown" => satya_toggle_ttrpg_skill_dropdown(),
        "satyaSelectTtrpgSkill" => satya_select_ttrpg_skill(String),
        "satyaTtrpgSkillKeydown" => satya_ttrpg_skill_keydown(KeyboardEvent),

        // Tabbed Notebook, 3D Dice, 3D Model, Combat Tracker & Rules Book
        "satyaSelectNotebookTab" => satya_select_notebook_tab(String),
        "satyaQuick3dDiceRoll" => satya_quick_3d_dice_roll(String),
        "satyaUpdateDiceMaterial" => satya_update_dice_material(),
        "satyaSelectDiceMaterialPreset" => satya_select_dice_material_preset(),
        "satyaUpdateDiceCamera" => satya_update_dice_camera(),
        "satyaPanDiceCamera" => satya_pan_dice_camera(f32, f32),
        "satyaResetDiceCamera" => satya_reset_dice_camera(),
        "satyaUpdate3dModelView" => satya_update_3d_model_view(),
        "satyaLoadSkillPreset" => satya_load_skill_preset(String, i32, i32),
        "satyaAdvanceCombatTick" => satya_advance_combat_tick(i32),
        "satyaSelectRulesChapter" => satya_select_rules_chapter(String),
        "satyaSearchRulesBook" => satya_search_rules_book(),
        "satyaLoadRulesExample" => satya_load_rules_example(String),

        // GM Document Vault & Google Drive Picker
        "satyaLinkGoogleAccount" => satya_link_google_account(),
        "satyaReceiveGoogleToken" => satya_receive_google_token(String),
        "satyaReceivePickedGoogleDoc" => satya_receive_picked_google_doc(String, String, String, String),
        "satyaGmDocFileSelected" => satya_gm_doc_file_selected(),
        "satyaGmDocUrlChanged" => satya_gm_doc_url_changed(),
        "satyaToggleGoogleOauthConfig" => satya_toggle_google_oauth_config(),
        "satyaOpenGoogleDrivePicker" => satya_open_google_drive_picker(),
        "satyaGmDocSourceChanged" => satya_gm_doc_source_changed(),
        "satyaLoadGmDocPreset" => satya_load_gm_doc_preset(String),
        "satyaSubmitGmDocument" => satya_submit_gm_document(),
        "satyaRefreshGmDocuments" => satya_refresh_gm_documents(),
        "satyaToggleGmDocPersistence" => satya_toggle_gm_doc_persistence(String, bool),
        "satyaSubmitGmMemory" => satya_submit_gm_memory(),
        "satyaDeleteGmMemory" => satya_delete_gm_memory(String),
        "satyaRefreshGmMemories" => satya_refresh_gm_memories(),
        "satyaSearchGmOmnisearch" => satya_search_gm_omnisearch(),

        // 6-Step Character Creation Wizard
        "satyaWizardSetStep" => satya_wizard_set_step(u8),
        "satyaWizardNameChanged" => satya_wizard_name_changed(),
        "satyaWizardSetMethod" => satya_wizard_set_method(String),
        "satyaWizardRollGroup" => satya_wizard_roll_group(String),
        "satyaWizardRollAllGroups" => satya_wizard_roll_all_groups(),
        "satyaWizardAutoAssignGroup" => satya_wizard_auto_assign_group(String),
        "satyaWizardAssignSlot" => satya_wizard_assign_slot(String, usize),
        "satyaWizardRollHeightMass" => satya_wizard_roll_height_mass(),
        "satyaWizardUpdateHeightMass" => satya_wizard_update_height_mass(),
        "satyaWizardRollPsychePool" => satya_wizard_roll_psyche_pool(),
        "satyaWizardResetPsycheBalanced" => satya_wizard_reset_psyche_balanced(),
        "satyaWizardSetPsyche" => satya_wizard_set_psyche(String, String),
        "satyaWizardRollAptitudes" => satya_wizard_roll_aptitudes(),
        "satyaWizardRollCompleteLifepath" => satya_wizard_roll_complete_lifepath(),
        "satyaWizardRollLifepathStage" => satya_wizard_roll_lifepath_stage(u8),
        "satyaWizardUpdateLifepathField" => satya_wizard_update_lifepath_field(String, String),
        "satyaWizardUpdateCareerTerm" => satya_wizard_update_career_term(usize, String, String),
        "satyaWizardSetSkillFilter" => satya_wizard_set_skill_filter(String, String),
        "satyaWizardAdjustSkill" => satya_wizard_adjust_skill(String, i32),
        "satyaWizardAutoAllocateSkills" => satya_wizard_auto_allocate_skills(),
        "satyaWizardResetPostSchoolSkills" => satya_wizard_reset_post_school_skills(),
        "satyaWizardApplyToSheet" => satya_wizard_apply_to_sheet(),

        // Interactive Character Sheet & Anatomical/Physiological/Psyche Status Tracker
        "satyaCharSheetLoadPreset" => satya_char_sheet_load_preset(String),
        "satyaCharSheetSetSection" => satya_char_sheet_set_section(String),
        "satyaCharSheetSetSkillFilter" => satya_char_sheet_set_skill_filter(String, bool),
        "satyaCharSheetSelectZone" => satya_char_sheet_select_zone(usize),
        "satyaCharSheetRollHitLocation" => satya_char_sheet_roll_hit_location(),
        "satyaCharSheetUpdateZone" => satya_char_sheet_update_zone(usize, String, String),
        "satyaCharSheetUpdateStatus" => satya_char_sheet_update_status(String, String),
        "satyaCharSheetModifyEquipment" => satya_char_sheet_modify_equipment(String, String),
        "satyaCharSheetSaveNotes" => satya_char_sheet_save_notes(String),
        "satyaCharSheetSaveToVault" => satya_char_sheet_save_to_vault(),
        "satyaCharSheetExportFile" => satya_char_sheet_export_file(),
        "satyaCharSheetImportFile" => satya_char_sheet_import_file(),
        "satyaCharSheetRefreshVault" => satya_char_sheet_refresh_vault(),
        "satyaCharSheetLoadSaved" => satya_char_sheet_load_saved(String),
        "satyaCharSheetBindSession" => satya_char_sheet_bind_session(),
        "satyaCharSheetUploadAsset" => satya_char_sheet_upload_asset(),
    );

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
