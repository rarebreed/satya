//! Pure Rust + WebAssembly (`wasm-bindgen` / `web-sys`) client runtime for the Satya
//! WebRTC Audio/Video, Chat, TTRPG Mechanics, Tabbed Notebook, and Apache Iggy Event Streaming platform.
//!
//! ## Submodules
//! - [`state`]: Thread-local [`SatyaWasmState`], [`PeerEntry`], and [`SECONDS_PER_TICK`] (`0.25s`) state management.
//! - [`dom`]: Typed `web-sys` DOM query/mutation helpers, HTML/JS escaping, base64/file I/O, and JSON `fetch` wrappers.
//! - [`options`]: Pure Rust enum parsers, dropdown `<select>` option tables, appearance normalizers, and AI GM defaults.
//! - [`menu`]: Top-bar Hamburger Menu, pane toggles, password visibility toggles, and BYOK AI GM configuration.
//! - [`auth_session`] & [`session_persistence`]: Auth, demo presets, session create/join/leave/resume, `.satyasession` snapshots, and roster rendering.
//! - [`notebook`]: Left Tabbed Notebook hub (`10` panes), Count-Up Combat Tick Tracker (`0.25s/tick`), and offline Rules Book reader.
//! - [`gm_docs`], [`gm_memories_and_director`], & [`npcs`]: `GM_ONLY` Document Vault, Google Drive Picker, GM Memories, 12-dial Scenario Director, and NPC roster.
//! - [`wgpu_shared`]: Shared `Vec3`/`Mat4` math, [`OrbitOrthoCamera`], WebGPU surface/depth init, and [`SHARED_GLB_DIORAMA_WGSL`].
//! - [`map_3d`] & [`map_3d_wgpu`]: Procedural 3D Tactical Diorama, Ray-vs-OBB cover/LoS solver, and instanced WebGPU renderer.
//! - [`dice_3d`] & [`dice_3d_wgpu`]: Deterministic 3D polyhedral dice meshes, SDF numeral atlas, physics, and 2-pass translucent WebGPU renderer.
//! - [`glb_model`] & [`model_3d_wgpu`]: Binary glTF 2.0 (`.glb`) parser/exporter, procedural humanoid rigs, and 3D Model Stage renderer.
//! - [`wizard`], [`wizard_views_stats`], [`wizard_views_lifepath`], & [`wizard_views_skills`]: Interactive 6-step Character Creation Wizard.
//! - [`character_sheet`], [`character_sheet_views`], [`character_sheet_equip_views`], [`character_sheet_status_views`], [`character_sheet_record`], & [`character_sheet_vault`]: Operator Character Sheet, 13-zone SVG trauma tracker, `.satyachar` framing, and Vault sync.
//! - [`webrtc`]: WebSocket signaling, P2P WebRTC Audio/Video + DataChannel mesh, and prosody telemetry.
//! - [`workspace_drawers`]: Collapsible drawers, floating Chat/Task/Webcam frames, and column splitter resize.
//! - [`chat_and_ttrpg`], [`ttrpg_drawer_cascade`], & [`ttrpg_drawer_triad`]: Targeted Session Chat, slash-command/roll execution, `2d20` Cascade odds, and Triad Combat Pool.
//! - [`telemetry`]: Apache Iggy Topic & Avro Schema Inspector (`#admin-inspector-card`).

pub mod auth_session;
pub mod character_sheet;
pub mod character_sheet_equip_views;
pub mod character_sheet_record;
pub mod character_sheet_status_views;
pub mod character_sheet_vault;
pub mod character_sheet_views;
pub mod chat_and_ttrpg;
pub mod dice_3d;
pub mod dice_3d_wgpu;
pub mod dom;
pub mod dom_tooltip;
pub mod glb_model;
pub mod gm_docs;
pub mod gm_memories_and_director;
pub mod map_2d_art;
pub mod map_2d_art_furniture;
pub mod map_2d_art_overlay;
pub mod map_2d_art_props;
pub mod map_2d_art_surfaces;
pub mod map_2d_art_walls;
pub mod map_2d_tokens;
pub mod map_3d;
pub mod map_3d_dressing_geom;
pub mod map_3d_input;
pub mod map_3d_wgpu;
pub mod map_edge_edit;
pub mod map_hover_inspect;
pub mod menu;
pub mod model_3d_wgpu;
pub mod notebook;
pub mod npcs;
pub mod options;
pub mod session_persistence;
pub mod state;
pub mod telemetry;
pub mod ttrpg_drawer_cascade;
pub mod ttrpg_drawer_triad;
pub mod webrtc;
pub mod wgpu_shared;
pub mod wizard;
pub mod wizard_views_lifepath;
pub mod wizard_views_skills;
pub mod wizard_views_stats;
pub mod workspace_drawers;

pub use auth_session::*;
pub use character_sheet::*;
pub use chat_and_ttrpg::*;
pub use dice_3d::*;
pub use dom::*;
pub use dom_tooltip::*;
pub use glb_model::*;
pub use map_3d::*;
pub use map_hover_inspect::*;
pub use menu::*;
pub use model_3d_wgpu::*;
pub use notebook::*;
pub use npcs::*;
pub use options::*;
pub use session_persistence::*;
pub use state::*;
pub use telemetry::*;
pub use ttrpg_drawer_triad::*;
pub use webrtc::*;
pub use wizard::*;

use crate::wasm_client::dom::{select_value, set_display, window};
use crate::wasm_client::options::{sync_chat_options_for_mode, sync_role_options_for_mode};
use crate::wasm_client::workspace_drawers::{
    end_workspace_pointer_interactions, handle_column_resize_move, handle_floating_pane_drag_move,
};
use js_sys::Reflect;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

/// Single-source-of-truth table for all `window.satya*` callbacks exposed to SSR/DOM handlers.
/// Feeds both [`REGISTERED_WINDOW_CALLBACKS`] (for host contract tests) and [`register_window_bindings`].
macro_rules! for_all_window_callbacks {
    ($mac:ident $(, $ctx:ident)?) => {
        $mac!($($ctx;)?
            // Auth, Session Gate & Hamburger Menu
            "satyaFillDemo" => satya_fill_demo(String, String, String),
            "satyaModeChanged" => satya_mode_changed(),
            "satyaRegister" => satya_register(),
            "satyaLogin" => satya_login(),
            "satyaLogout" => satya_logout(),
            "satyaCreateSession" => satya_create_session(),
            "satyaJoinSession" => satya_join_session(),
            "satyaLeaveSession" => satya_leave_session(),
            "satyaRefreshSavedSessions" => satya_refresh_saved_sessions(),
            "satyaSelectSavedSession" => satya_select_saved_session(),
            "satyaResumeSavedSession" => satya_resume_saved_session(),
            "satyaExportSessionSnapshot" => satya_export_session_snapshot(),
            "satyaImportSessionSnapshot" => satya_import_session_snapshot(),
            "satyaExportChatTranscript" => satya_export_chat_transcript(String),
            "satyaTogglePassword" => satya_toggle_password(String, String),
            "satyaToggleHamburgerMenu" => satya_toggle_hamburger_menu(),
            "satyaToggleAuthPane" => satya_toggle_auth_pane(),
            "satyaToggleSessionPane" => satya_toggle_session_pane(),
            "satyaToggleSettingsMenu" => satya_toggle_settings_menu(),
            "satyaToggleChatSettings" => satya_toggle_chat_settings(),
            "satyaToggleAiGmSettings" => satya_toggle_ai_gm_settings(),
            "satyaToggleAppearanceSettings" => satya_toggle_appearance_settings(),
            "satyaApplyAppearanceSettings" => satya_apply_appearance_settings(),
            "satyaSelectThemePreset" => satya_select_theme_preset(String),
            "satyaResetAppearanceSettings" => satya_reset_appearance_settings(),
            "satyaAiGmProviderChanged" => satya_ai_gm_provider_changed(),
            "satyaMenuAiGmProviderChanged" => satya_menu_ai_gm_provider_changed(),
            "satyaUpdateSessionGmConfig" => satya_update_session_gm_config(),
            "satyaToggleAiPlayerSettings" => satya_toggle_ai_player_settings(),
            "satyaSpawnSessionAiPlayer" => satya_spawn_session_ai_player(),
            "satyaStepSessionAiPlayer" => satya_step_session_ai_player(),

            // WebRTC Media, Drawers & Telemetry
            "satyaStartCamera" => satya_start_camera(),
            "satyaToggleAudio" => satya_toggle_audio(),
            "satyaToggleVideo" => satya_toggle_video(),
            "satyaEmitProsodySample" => satya_emit_prosody_sample(),
            "satyaToggleWebcamFloat" => satya_toggle_webcam_float(),
            "satyaToggleChatFloat" => satya_toggle_chat_float(),
            "satyaToggleTtrpgFloat" => satya_toggle_ttrpg_float(),
            "satyaToggleWebcamDrawer" => satya_toggle_webcam_drawer(),
            "satyaToggleChatDrawer" => satya_toggle_chat_drawer(),
            "satyaToggleTtrpgDrawer" => satya_toggle_ttrpg_drawer(),
            "satyaToggleWebcamColumns" => satya_toggle_webcam_columns(),
            "satyaStartWebcamDrag" => satya_start_webcam_drag(web_sys::Event),
            "satyaStartChatDrag" => satya_start_chat_drag(web_sys::Event),
            "satyaStartTtrpgDrag" => satya_start_ttrpg_drag(web_sys::Event),
            "satyaFloatingHeaderKeydown" => satya_floating_header_keydown(String, KeyboardEvent),
            "satyaStartColumnResize" => satya_start_column_resize(web_sys::Event),
            "satyaResetColumnResize" => satya_reset_column_resize(),
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
            "satyaOnDrawerTechniqueChange" => satya_on_drawer_technique_change(),
            "satyaApplyTriadPreset" => satya_apply_triad_preset(),
            "satyaUpdateDrawerTriadPreview" => satya_update_drawer_triad_preview(),

            // Tabbed Notebook, 3D Dice, 3D Model, Combat Tracker & Rules Book
            "satyaSelectNotebookTab" => satya_select_notebook_tab(String),
            "satyaQuick3dDiceRoll" => satya_quick_3d_dice_roll(String),
            "satyaUpdateDiceMaterial" => satya_update_dice_material(),
            "satyaSelectDiceMaterialPreset" => satya_select_dice_material_preset(),
            "satyaUpdateDiceCamera" => satya_update_dice_camera(),
            "satyaPanDiceCamera" => satya_pan_dice_camera(f32, f32),
            "satyaResetDiceCamera" => satya_reset_dice_camera(),
            "satyaUpdate3dModelView" => satya_update_3d_model_view(),
            "satyaResetModelCamera" => satya_reset_model_camera(),
            "satyaImportCustomGlbModel" => satya_import_custom_glb_model(),
            "satyaExportCurrentGlbModel" => satya_export_current_glb_model(),
            "satyaSelectMapToken" => satya_select_map_token(String),
            "satyaClickMapCell" => satya_click_map_cell(i32, i32),
            "satyaClickMapPoint" => satya_click_map_point(f32, f32),
            "satyaHoverMapCell" => satya_hover_map_cell(i32, i32),
            "satyaHoverMapPoint" => satya_hover_map_point(f32, f32, f32, f32),
            "satyaLeaveMapHover" => satya_leave_map_hover(),
            "satyaToggleMapInspectMode" => satya_toggle_map_inspect_mode(),
            "satyaToggleBattlemapView" => satya_toggle_battlemap_view(),
            "satyaToggleBattlemapGrid" => satya_toggle_battlemap_grid(),
            "satyaSetBattlemapInkStyle" => satya_set_battlemap_ink_style(),
            "satyaUpdateMapPropBrush" => satya_update_map_prop_brush(),
            "satyaRotateMapPropBrush" => satya_rotate_map_prop_brush(),
            "satyaSetMapPropRotationDeg" => satya_set_map_prop_rotation_deg(),
            "satyaToggleMapGridSnap" => satya_toggle_map_grid_snap(),
            "satyaGenerateProceduralMap" => satya_generate_procedural_map(),
            "satyaUpdateMapTacticalControls" => satya_update_map_tactical_controls(),
            "satyaUpdateSelectedTokenVisual" => satya_update_selected_token_visual(),
            "satyaApplyMapCoverToRoller" => satya_apply_map_cover_to_roller(),
            "satyaToggleMapProjection" => satya_toggle_map_projection(),
            "satyaToggleMapWallCutaway" => satya_toggle_map_wall_cutaway(),
            "satyaResetMapCamera" => satya_reset_map_camera(),
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
            "satyaUpdateScenarioDialReadouts" => satya_update_scenario_dial_readouts(),
            "satyaApplyScenarioPreset" => satya_apply_scenario_preset(String),
            "satyaSaveScenarioDirectorConfig" => satya_save_scenario_director_config(),
            "satyaSearchGmOmnisearch" => satya_search_gm_omnisearch(),
            "satyaRefreshNpcsTab" => satya_refresh_npcs_tab(),
            "satyaSelectNpcFromList" => satya_select_npc_from_list(),
            "satyaGenerateNpcFromPrompt" => satya_generate_npc_from_prompt(),
            "satyaLoadSelectedNpcToSheet" => satya_load_selected_npc_to_sheet(),

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
        )
    };
}

macro_rules! emit_window_callback_names {
    ($( $js_name:literal => $fn_ident:ident ( $($arg_ty:ty),* ) ),* $(,)?) => {
        &[ $( $js_name ),* ]
    };
}

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

/// Canonical slice of every `window.satya*` callback bound by [`register_window_bindings`].
/// Used by host SSR↔Wasm contract tests to prevent typos or unregistered inline handlers.
pub const REGISTERED_WINDOW_CALLBACKS: &[&str] =
    for_all_window_callbacks!(emit_window_callback_names);

/// WebAssembly module entrypoint automatically invoked when `/pkg/satya_web.js` initializes.
#[wasm_bindgen(start)]
pub fn start_satya_wasm_client() {
    register_window_bindings();
    restore_saved_appearance_settings();
    restore_saved_column_split();
    let mode_raw = select_value("session-mode-select");
    let mode = parse_session_mode(&mode_raw);
    sync_role_options_for_mode(&mode_raw, None);
    sync_chat_options_for_mode(mode);
    satya_visibility_changed();
    set_display("admin-inspector-card", "none");
    refresh_map_dom_and_wgpu();
    restore_saved_auth_and_session();
}

/// Binds the Rust Wasm handlers onto `window` so both SSR HTML attributes and dynamic
/// `format_*_html()` fragments invoke compiled Rust WebAssembly functions directly.
pub fn register_window_bindings() {
    let Some(win) = window() else {
        return;
    };
    let bind = |name: &str, func: &JsValue| {
        let _ = Reflect::set(&win, &JsValue::from_str(name), func);
    };

    for_all_window_callbacks!(bind_window_callbacks, bind);

    let on_move = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
        handle_floating_pane_drag_move(&ev);
        handle_column_resize_move(&ev);
    });
    let _ = win.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
    on_move.forget();

    let on_up = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
        end_workspace_pointer_interactions();
    });
    let _ = win.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
    on_up.forget();

    let on_keydown = Closure::<dyn FnMut(KeyboardEvent)>::new(move |ev: KeyboardEvent| {
        handle_map_global_keydown(&ev);
    });
    let _ = win.add_event_listener_with_callback("keydown", on_keydown.as_ref().unchecked_ref());
    on_keydown.forget();

    let on_keyup = Closure::<dyn FnMut(KeyboardEvent)>::new(move |ev: KeyboardEvent| {
        handle_map_global_keyup(&ev);
    });
    let _ = win.add_event_listener_with_callback("keyup", on_keyup.as_ref().unchecked_ref());
    on_keyup.forget();
}
