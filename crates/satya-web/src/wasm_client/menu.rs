//! Top-bar Hamburger Menu, pane visibility toggles, password visibility toggles,
//! and ephemeral BYOK AI GM session configuration handlers.

use crate::wasm_client::dom::{
    EYE_OFF_SVG, EYE_OPEN_SVG, fetch_json, get_element, input_checked, input_value,
    local_storage_get, local_storage_set, select_value, set_display, set_feedback, set_input_value,
    set_root_data_attribute, set_select_value, set_text, toggle_element_display,
};
use crate::wasm_client::options::{
    APPEARANCE_THEME_OPTIONS, default_model_for_provider, normalize_corners_id,
    normalize_density_id, normalize_font_id, normalize_theme_id,
};
use crate::wasm_client::state::with_state;
use satya_schema::{AiGmProviderConfig, AiPlayerConfig, StepAiPlayerRequest};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;

fn close_hamburger_dropdown() {
    set_display("hamburger-menu-dropdown", "none");
    if let Some(btn) = get_element("btn-hamburger-menu") {
        let _ = btn.set_attribute("aria-expanded", "false");
    }
}

/// Toggles the top-bar hamburger menu dropdown (`#hamburger-menu-dropdown`) open or closed.
#[wasm_bindgen]
pub fn satya_toggle_hamburger_menu() {
    let Some(is_opening) = toggle_element_display("hamburger-menu-dropdown", "flex") else {
        return;
    };
    if let Some(btn) = get_element("btn-hamburger-menu") {
        let _ = btn.set_attribute("aria-expanded", if is_opening { "true" } else { "false" });
    }
}

/// Toggles the `1. User Authentication` pane (`#auth-card`) visible or hidden.
#[wasm_bindgen]
pub fn satya_toggle_auth_pane() {
    let Some(is_opening) = toggle_element_display("auth-card", "block") else {
        return;
    };
    set_text(
        "btn-menu-toggle-auth-pane",
        if is_opening {
            "Hide Auth Pane"
        } else {
            "Show Auth Pane"
        },
    );
    close_hamburger_dropdown();
}

/// Toggles the `2. Create or Join Password-Protected Session` pane (`#session-card`) visible or hidden.
#[wasm_bindgen]
pub fn satya_toggle_session_pane() {
    let Some(is_opening) = toggle_element_display("session-card", "block") else {
        return;
    };
    set_text(
        "btn-menu-toggle-session-pane",
        if is_opening {
            "Hide Session Pane"
        } else {
            "Show Session Pane"
        },
    );
    close_hamburger_dropdown();
}

/// Toggles the `Settings` sub-panel (`#menu-settings-panel`) inside the top-bar Hamburger Menu.
#[wasm_bindgen]
pub fn satya_toggle_settings_menu() {
    let _ = toggle_element_display("menu-settings-panel", "flex");
}

/// Toggles the `Settings -> Chat` sub-panel (`#menu-chat-settings-panel`) inside the top-bar Hamburger Menu.
#[wasm_bindgen]
pub fn satya_toggle_chat_settings() {
    let _ = toggle_element_display("menu-chat-settings-panel", "flex");
}

/// Toggles the `Settings -> AI GM Provider` sub-panel (`#menu-ai-gm-settings-panel`) inside the top-bar Hamburger Menu.
#[wasm_bindgen]
pub fn satya_toggle_ai_gm_settings() {
    let _ = toggle_element_display("menu-ai-gm-settings-panel", "flex");
}

/// Toggles the `Settings -> AI Player (Test Bot)` sub-panel (`#menu-ai-player-settings-panel`) inside the top-bar Hamburger Menu.
#[wasm_bindgen]
pub fn satya_toggle_ai_player_settings() {
    let _ = toggle_element_display("menu-ai-player-settings-panel", "flex");
}

/// Toggles the `Settings -> Appearance & Theme` sub-panel (`#menu-appearance-settings-panel`) inside the top-bar Hamburger Menu.
#[wasm_bindgen]
pub fn satya_toggle_appearance_settings() {
    let _ = toggle_element_display("menu-appearance-settings-panel", "flex");
}

const STORAGE_KEY_THEME: &str = "satya.appearance.theme";
const STORAGE_KEY_DENSITY: &str = "satya.appearance.density";
const STORAGE_KEY_CORNERS: &str = "satya.appearance.corners";
const STORAGE_KEY_FONT: &str = "satya.appearance.font";

fn sync_theme_swatch_buttons(active_theme: &str) {
    for (theme_id, _) in APPEARANCE_THEME_OPTIONS {
        let btn_id = format!("btn-theme-swatch-{theme_id}");
        if let Some(btn) = get_element(&btn_id) {
            let is_active = *theme_id == active_theme;
            btn.set_class_name(if is_active {
                "theme-swatch-btn active"
            } else {
                "theme-swatch-btn"
            });
            let _ = btn.set_attribute("aria-pressed", if is_active { "true" } else { "false" });
        }
    }
}

fn apply_appearance_to_root(theme: &str, density: &str, corners: &str, font: &str) {
    set_root_data_attribute("data-theme", theme);
    set_root_data_attribute("data-density", density);
    set_root_data_attribute("data-corners", corners);
    set_root_data_attribute("data-font", font);
    sync_theme_swatch_buttons(theme);
}

/// Restores saved appearance preferences (`theme`, `density`, `corners`, `font`) from browser
/// `localStorage` onto `document.documentElement` and synchronizes the Hamburger Menu controls.
pub fn restore_saved_appearance_settings() {
    let theme = normalize_theme_id(&local_storage_get(STORAGE_KEY_THEME).unwrap_or_default());
    let density = normalize_density_id(&local_storage_get(STORAGE_KEY_DENSITY).unwrap_or_default());
    let corners = normalize_corners_id(&local_storage_get(STORAGE_KEY_CORNERS).unwrap_or_default());
    let font = normalize_font_id(&local_storage_get(STORAGE_KEY_FONT).unwrap_or_default());

    set_select_value("menu-theme-select", theme);
    set_select_value("menu-density-select", density);
    set_select_value("menu-corners-select", corners);
    set_select_value("menu-font-select", font);
    apply_appearance_to_root(theme, density, corners, font);
}

/// Reads `#menu-theme-select`, `#menu-density-select`, `#menu-corners-select`, and `#menu-font-select`,
/// applies the normalized `data-*` attributes to `<html>`, and persists them to `localStorage`.
#[wasm_bindgen]
pub fn satya_apply_appearance_settings() {
    let theme = normalize_theme_id(&select_value("menu-theme-select"));
    let density = normalize_density_id(&select_value("menu-density-select"));
    let corners = normalize_corners_id(&select_value("menu-corners-select"));
    let font = normalize_font_id(&select_value("menu-font-select"));

    local_storage_set(STORAGE_KEY_THEME, theme);
    local_storage_set(STORAGE_KEY_DENSITY, density);
    local_storage_set(STORAGE_KEY_CORNERS, corners);
    local_storage_set(STORAGE_KEY_FONT, font);
    apply_appearance_to_root(theme, density, corners, font);
}

/// Selects a color theme preset by ID (from a visual swatch chip), updates `#menu-theme-select`,
/// and immediately applies and persists the updated appearance settings.
#[wasm_bindgen]
pub fn satya_select_theme_preset(theme_id: String) {
    let theme = normalize_theme_id(&theme_id);
    set_select_value("menu-theme-select", theme);
    satya_apply_appearance_settings();
}

/// Resets color theme, UI density, corner radius, and font family to their canonical defaults
/// (`obsidian`, `default`, `default`, `sans`) and persists the reset state.
#[wasm_bindgen]
pub fn satya_reset_appearance_settings() {
    set_select_value("menu-theme-select", "obsidian");
    set_select_value("menu-density-select", "default");
    set_select_value("menu-corners-select", "default");
    set_select_value("menu-font-select", "sans");
    satya_apply_appearance_settings();
}

/// Updates `#ai-gm-model-input` with the recommended default model when `#ai-gm-provider-select` changes.
#[wasm_bindgen]
pub fn satya_ai_gm_provider_changed() {
    let provider = select_value("ai-gm-provider-select");
    set_input_value("ai-gm-model-input", default_model_for_provider(&provider));
}

/// Updates `#menu-ai-gm-model` with the recommended default model when `#menu-ai-gm-provider` changes.
#[wasm_bindgen]
pub fn satya_menu_ai_gm_provider_changed() {
    let provider = select_value("menu-ai-gm-provider");
    set_input_value("menu-ai-gm-model", default_model_for_provider(&provider));
}

/// Applies an updated AI GM provider, model, and optional ephemeral BYOK API key to the active session
/// via `POST /api/sessions/:id/gm-config`, scrubbing the password input immediately after reading it.
#[wasm_bindgen]
pub fn satya_update_session_gm_config() {
    spawn_local(async move {
        let (session_id, auth_token) =
            with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            set_feedback(
                "menu-ai-gm-feedback",
                "Join or create an active session as GM first.",
                true,
            );
            return;
        };
        let provider = select_value("menu-ai-gm-provider");
        let model_raw = input_value("menu-ai-gm-model").trim().to_string();
        let model_id = if model_raw.is_empty() {
            default_model_for_provider(&provider).to_string()
        } else {
            model_raw
        };
        let key_raw = input_value("menu-ai-gm-key").trim().to_string();
        // Scrub ephemeral BYOK password field from DOM immediately after reading
        set_input_value("menu-ai-gm-key", "");
        let api_key = if key_raw.is_empty() {
            None
        } else {
            Some(key_raw)
        };

        let cfg = AiGmProviderConfig {
            provider: provider.clone(),
            model_id: model_id.clone(),
            vision_model_id: None,
            temperature: 0.35,
            persona_id: "harsh-reality-arbiter".to_string(),
            api_key,
            base_url: None,
        };
        let path = format!("/api/sessions/{sid}/gm-config");
        match fetch_json::<_, serde_json::Value>("POST", &path, Some(&tok), Some(&cfg)).await {
            Ok(_) => {
                set_feedback(
                    "menu-ai-gm-feedback",
                    &format!("Updated AI GM: {provider} / {model_id}"),
                    false,
                );
            }
            Err(err) => {
                set_feedback("menu-ai-gm-feedback", &err, true);
            }
        }
    });
}

/// Spawns or updates an AI Player test agent in the active session via `POST /api/sessions/:id/ai-players`.
#[wasm_bindgen]
pub fn satya_spawn_session_ai_player() {
    spawn_local(async move {
        let (session_id, auth_token) =
            with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            set_feedback(
                "menu-ai-player-feedback",
                "Join or create an active session first.",
                true,
            );
            return;
        };
        let display_name = input_value("menu-ai-player-name").trim().to_string();
        let persona_prompt = input_value("menu-ai-player-persona").trim().to_string();
        let auto_respond = input_checked("menu-ai-player-auto");

        let cfg = AiPlayerConfig {
            user_id: Some("ai-player-1".to_string()),
            display_name: if display_name.is_empty() {
                None
            } else {
                Some(display_name)
            },
            character_id: None,
            persona_prompt: if persona_prompt.is_empty() {
                None
            } else {
                Some(persona_prompt)
            },
            provider: None,
            model_id: None,
            service_url: None,
            auto_respond,
            max_rally_turns: 2,
        };
        let path = format!("/api/sessions/{sid}/ai-players");
        match fetch_json::<_, serde_json::Value>("POST", &path, Some(&tok), Some(&cfg)).await {
            Ok(_) => {
                set_feedback(
                    "menu-ai-player-feedback",
                    "AI Player active in session roster.",
                    false,
                );
            }
            Err(err) => {
                set_feedback("menu-ai-player-feedback", &err, true);
            }
        }
    });
}

/// Triggers the active AI Player to take a turn via `POST /api/sessions/:id/ai-players/step`.
#[wasm_bindgen]
pub fn satya_step_session_ai_player() {
    spawn_local(async move {
        let (session_id, auth_token) =
            with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            set_feedback(
                "menu-ai-player-feedback",
                "Join or create an active session first.",
                true,
            );
            return;
        };
        let req = StepAiPlayerRequest {
            user_id: Some("ai-player-1".to_string()),
            prompt: None,
            auto_trigger_gm: true,
        };
        let path = format!("/api/sessions/{sid}/ai-players/step");
        match fetch_json::<_, serde_json::Value>("POST", &path, Some(&tok), Some(&req)).await {
            Ok(_) => {
                set_feedback("menu-ai-player-feedback", "Stepped AI Player turn!", false);
            }
            Err(err) => {
                set_feedback("menu-ai-player-feedback", &err, true);
            }
        }
    });
}

/// Toggles a password `<input>` element between `"password"` (masked) and `"text"` (visible characters),
/// updating the associated eye toggle button's icon and accessibility attributes.
#[wasm_bindgen]
pub fn satya_toggle_password(input_id: String, button_id: String) {
    let Some(inp) = get_element(&input_id).and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
    else {
        return;
    };
    let showing = inp.type_() == "password";
    inp.set_type(if showing { "text" } else { "password" });

    if let Some(btn) = get_element(&button_id) {
        let label = if showing {
            "Hide password"
        } else {
            "Show password"
        };
        let _ = btn.set_attribute("title", label);
        let _ = btn.set_attribute("aria-label", label);
        let _ = btn.set_attribute("aria-pressed", if showing { "true" } else { "false" });
        btn.set_inner_html(if showing { EYE_OFF_SVG } else { EYE_OPEN_SVG });
    }
}
