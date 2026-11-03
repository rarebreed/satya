//! Top-bar Hamburger Menu, pane visibility toggles, password visibility toggles,
//! and ephemeral BYOK AI GM session configuration handlers.

use crate::wasm_client::dom::{
    EYE_OFF_SVG, EYE_OPEN_SVG, fetch_json, get_element, input_value, select_value, set_display,
    set_feedback, set_input_value, set_text,
};
use crate::wasm_client::options::default_model_for_provider;
use crate::wasm_client::state::with_state;
use satya_schema::AiGmProviderConfig;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlElement, HtmlInputElement};

/// Toggles the top-bar hamburger menu dropdown (`#hamburger-menu-dropdown`) open or closed.
#[wasm_bindgen]
pub fn satya_toggle_hamburger_menu() {
    let Some(menu) =
        get_element("hamburger-menu-dropdown").and_then(|el| el.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let is_hidden = menu
        .style()
        .get_property_value("display")
        .unwrap_or_default()
        == "none";
    let _ = menu
        .style()
        .set_property("display", if is_hidden { "flex" } else { "none" });
    if let Some(btn) = get_element("btn-hamburger-menu") {
        let _ = btn.set_attribute("aria-expanded", if is_hidden { "true" } else { "false" });
    }
}

/// Toggles the `1. User Authentication` pane (`#auth-card`) visible or hidden.
#[wasm_bindgen]
pub fn satya_toggle_auth_pane() {
    let Some(card) = get_element("auth-card").and_then(|el| el.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let is_hidden = card
        .style()
        .get_property_value("display")
        .unwrap_or_default()
        == "none";
    let _ = card
        .style()
        .set_property("display", if is_hidden { "block" } else { "none" });
    set_text(
        "btn-menu-toggle-auth-pane",
        if is_hidden {
            "Hide Auth Pane"
        } else {
            "Show Auth Pane"
        },
    );
    set_display("hamburger-menu-dropdown", "none");
    if let Some(btn) = get_element("btn-hamburger-menu") {
        let _ = btn.set_attribute("aria-expanded", "false");
    }
}

/// Toggles the `2. Create or Join Password-Protected Session` pane (`#session-card`) visible or hidden.
#[wasm_bindgen]
pub fn satya_toggle_session_pane() {
    let Some(card) = get_element("session-card").and_then(|el| el.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let is_hidden = card
        .style()
        .get_property_value("display")
        .unwrap_or_default()
        == "none";
    let _ = card
        .style()
        .set_property("display", if is_hidden { "block" } else { "none" });
    set_text(
        "btn-menu-toggle-session-pane",
        if is_hidden {
            "Hide Session Pane"
        } else {
            "Show Session Pane"
        },
    );
    set_display("hamburger-menu-dropdown", "none");
    if let Some(btn) = get_element("btn-hamburger-menu") {
        let _ = btn.set_attribute("aria-expanded", "false");
    }
}

/// Toggles the `Settings` sub-panel (`#menu-settings-panel`) inside the top-bar Hamburger Menu.
#[wasm_bindgen]
pub fn satya_toggle_settings_menu() {
    let Some(panel) =
        get_element("menu-settings-panel").and_then(|el| el.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let is_hidden = panel
        .style()
        .get_property_value("display")
        .unwrap_or_default()
        == "none";
    let _ = panel
        .style()
        .set_property("display", if is_hidden { "flex" } else { "none" });
}

/// Toggles the `Settings -> Chat` sub-panel (`#menu-chat-settings-panel`) inside the top-bar Hamburger Menu.
#[wasm_bindgen]
pub fn satya_toggle_chat_settings() {
    let Some(panel) =
        get_element("menu-chat-settings-panel").and_then(|el| el.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let is_hidden = panel
        .style()
        .get_property_value("display")
        .unwrap_or_default()
        == "none";
    let _ = panel
        .style()
        .set_property("display", if is_hidden { "flex" } else { "none" });
}

/// Toggles the `Settings -> AI GM Provider` sub-panel (`#menu-ai-gm-settings-panel`) inside the top-bar Hamburger Menu.
#[wasm_bindgen]
pub fn satya_toggle_ai_gm_settings() {
    let Some(panel) =
        get_element("menu-ai-gm-settings-panel").and_then(|el| el.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let is_hidden = panel
        .style()
        .get_property_value("display")
        .unwrap_or_default()
        == "none";
    let _ = panel
        .style()
        .set_property("display", if is_hidden { "flex" } else { "none" });
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
