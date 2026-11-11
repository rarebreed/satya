//! Top-bar navigation header (`HeaderBar`), shared password visibility toggle
//! (`PasswordToggleButton`), and Hamburger Menu appearance/chat/BYOK AI settings.
//!
//! Pre-session authentication and session creation/resume live in [`crate::views_gate`].

use crate::wasm_client;
use leptos::prelude::*;

/// Static table of chat message visibility filter checkboxes in the Hamburger Menu.
const CHAT_FILTER_SPECS: &[(&str, &str, &str)] = &[
    ("filter-msg-public", "PUBLIC", "Public"),
    ("filter-msg-private", "PRIVATE", "Private / Whisper"),
    ("filter-msg-ai-gm", "AI_GM", "AI GM"),
    ("filter-msg-mechanics", "MECHANICS", "Mechanics"),
    ("filter-msg-system", "SYSTEM", "System (RUST/WASM)"),
];

/// Static table of visual theme swatch chips (`(theme_id, short_label, bg_hex, accent_hex)`).
const THEME_SWATCH_SPECS: &[(&str, &str, &str, &str)] = &[
    ("obsidian", "Obsidian", "#0b0e14", "#38bdf8"),
    ("amber-hud", "Amber HUD", "#090806", "#f59e0b"),
    ("parchment", "Parchment", "#f5efe2", "#b45309"),
    ("cyberpunk", "Cyberpunk", "#080412", "#f43f5e"),
    ("ranger", "Ranger", "#080f0b", "#34d399"),
    ("dusk-orange", "Dusk Orange", "#110a08", "#fb923c"),
    ("nebula", "Nebula", "#1a0f30", "#f97316"),
    ("navy-gold", "Navy & Gold", "#0b192e", "#facc15"),
    ("marine-corps", "USMC", "#15241a", "#9f1239"),
    ("lcars", "TNG LCARS", "#000000", "#ff9900"),
    ("mil-spec", "MIL-SPEC C4ISR", "#0e120c", "#a3e635"),
    ("pulp-1930s", "1930s Pulp", "#1e150f", "#d97706"),
    ("wehrmacht", "Feldgrau", "#181b19", "#dc2626"),
];

/// Static table of appearance `<select>` controls (`(select_id, select_name, label, options)`).
const APPEARANCE_SELECT_SPECS: &[(&str, &str, &str, wasm_client::SelectOptionSlice)] = &[
    (
        "menu-theme-select",
        "menu_theme_select",
        "Color Theme",
        wasm_client::APPEARANCE_THEME_OPTIONS,
    ),
    (
        "menu-style-select",
        "menu_style_select",
        "Panel & Frame Style",
        wasm_client::APPEARANCE_STYLE_OPTIONS,
    ),
    (
        "menu-density-select",
        "menu_density_select",
        "UI Density & Border Padding",
        wasm_client::APPEARANCE_DENSITY_OPTIONS,
    ),
    (
        "menu-corners-select",
        "menu_corners_select",
        "Corner Radius",
        wasm_client::APPEARANCE_CORNERS_OPTIONS,
    ),
    (
        "menu-font-select",
        "menu_font_select",
        "Font Family",
        wasm_client::APPEARANCE_FONT_OPTIONS,
    ),
    (
        "menu-font-size-select",
        "menu_font_size_select",
        "Font Size",
        wasm_client::APPEARANCE_FONT_SIZE_OPTIONS,
    ),
];

/// Shared AI GM LLM provider `(value, label)` options rendered in `#menu-ai-gm-provider`
/// and `#ai-gm-provider-select`.
pub const AI_GM_PROVIDER_OPTIONS: &[(&str, &str)] = &[
    ("ollama", "Ollama (Local - Default)"),
    ("google_genai", "Google Gemini (BYOK / Env)"),
    ("openai", "OpenAI (BYOK / Env)"),
    ("anthropic", "Anthropic Claude (BYOK / Env)"),
];

/// Shared password visibility toggle button (`EYE_OPEN_SVG` / `EYE_CLOSED_SVG`).
#[component]
pub fn PasswordToggleButton(
    /// Target `<input type="password">` element ID.
    input_id: &'static str,
    /// Toggle `<button>` element ID.
    btn_id: &'static str,
) -> impl IntoView {
    let onclick_js = format!("window.satyaTogglePassword('{input_id}', '{btn_id}')");
    view! {
        <button
            type="button"
            id=btn_id
            class="btn-password-toggle"
            title="Show password"
            aria-label="Show password"
            aria-pressed="false"
            onclick=onclick_js
            on:click=move |_| {
                wasm_client::satya_toggle_password(input_id.to_string(), btn_id.to_string())
            }
            inner_html=wasm_client::EYE_OPEN_SVG
        ></button>
    }
}

/// Top navigation bar with brand identity, auth/session status pills, and Hamburger Menu.
#[component]
pub fn HeaderBar() -> impl IntoView {
    view! {
        <header class="top-bar">
            <div class="brand">
                <span class="brand-badge">"Satya // Engine"</span>
                <h1>"Satsang"</h1>
            </div>
            <div class="top-bar-right">
                <div class="status-pills">
                    <span id="auth-status-pill" class="pill pill-muted">"Not Logged In"</span>
                    <span id="session-status-pill" class="pill pill-muted">"No Active Session"</span>
                </div>
                <div class="hamburger-menu-wrap">
                    <button
                        type="button"
                        id="btn-hamburger-menu"
                        class="btn-hamburger"
                        title="User & Pane Menu"
                        aria-label="User & Pane Menu"
                        aria-expanded="false"
                        onclick="window.satyaToggleHamburgerMenu()"
                        on:click=move |_| wasm_client::satya_toggle_hamburger_menu()
                        inner_html=wasm_client::HAMBURGER_SVG
                    ></button>
                    <div
                        id="hamburger-menu-dropdown"
                        class="hamburger-dropdown"
                        style="display: none;"
                    >
                        <div class="menu-section-title">"User Details"</div>
                        <div id="menu-user-details" class="menu-user-details">
                            "Not Logged In"
                        </div>
                        <div class="menu-actions">
                            <button
                                type="button"
                                id="btn-menu-toggle-auth-pane"
                                class="btn btn-sm btn-secondary"
                                onclick="window.satyaToggleAuthPane()"
                                on:click=move |_| wasm_client::satya_toggle_auth_pane()
                            >
                                "Hide Auth Pane"
                            </button>
                            <button
                                type="button"
                                id="btn-menu-toggle-session-pane"
                                class="btn btn-sm btn-secondary"
                                onclick="window.satyaToggleSessionPane()"
                                on:click=move |_| wasm_client::satya_toggle_session_pane()
                            >
                                "Show Session Pane"
                            </button>
                            <button
                                type="button"
                                id="btn-menu-toggle-chat-pane"
                                class="btn btn-sm btn-secondary"
                                aria-expanded="true"
                                aria-controls="chat-card"
                                onclick="window.satyaToggleChatDrawer()"
                                on:click=move |_| wasm_client::satya_toggle_chat_drawer()
                            >
                                "Hide Session Chat"
                            </button>
                            <button
                                type="button"
                                id="btn-menu-toggle-ttrpg-pane"
                                class="btn btn-sm btn-secondary"
                                aria-expanded="true"
                                aria-controls="ttrpg-drawer"
                                onclick="window.satyaToggleTtrpgDrawer()"
                                on:click=move |_| wasm_client::satya_toggle_ttrpg_drawer()
                            >
                                "Hide Task Roller"
                            </button>
                            <button
                                type="button"
                                id="btn-menu-toggle-webcam-pane"
                                class="btn btn-sm btn-secondary"
                                aria-expanded="true"
                                aria-controls="webrtc-card"
                                onclick="window.satyaToggleWebcamDrawer()"
                                on:click=move |_| wasm_client::satya_toggle_webcam_drawer()
                            >
                                "Hide Web Cams"
                            </button>
                            <button
                                type="button"
                                id="btn-menu-toggle-settings"
                                class="btn btn-sm btn-secondary"
                                onclick="window.satyaToggleSettingsMenu()"
                                on:click=move |_| wasm_client::satya_toggle_settings_menu()
                            >
                                "Settings"
                            </button>
                            <div
                                id="menu-settings-panel"
                                class="menu-settings-panel"
                                style="display: none;"
                            >
                                <div class="menu-section-title">"Settings"</div>
                                <button
                                    type="button"
                                    id="btn-menu-toggle-appearance-settings"
                                    class="btn btn-sm btn-secondary"
                                    onclick="window.satyaToggleAppearanceSettings()"
                                    on:click=move |_| wasm_client::satya_toggle_appearance_settings()
                                >
                                    "Appearance & Theme"
                                </button>
                                <div
                                    id="menu-appearance-settings-panel"
                                    class="menu-chat-settings-panel"
                                    style="display: none;"
                                >
                                    <span class="filter-label">"Theme Palette:"</span>
                                    <div id="appearance-swatch-grid" class="theme-swatch-grid">
                                        {THEME_SWATCH_SPECS
                                            .iter()
                                            .copied()
                                            .map(|(theme_id, short_label, bg_hex, accent_hex)| {
                                                let btn_id = format!("btn-theme-swatch-{theme_id}");
                                                let is_default = theme_id == "obsidian";
                                                let btn_class = if is_default {
                                                    "theme-swatch-btn active"
                                                } else {
                                                    "theme-swatch-btn"
                                                };
                                                let pressed = if is_default { "true" } else { "false" };
                                                let onclick_js =
                                                    format!("window.satyaSelectThemePreset('{theme_id}')");
                                                let dot_style = format!(
                                                    "background: linear-gradient(135deg, {bg_hex} 50%, {accent_hex} 50%);"
                                                );
                                                view! {
                                                    <button
                                                        type="button"
                                                        id=btn_id
                                                        class=btn_class
                                                        aria-pressed=pressed
                                                        onclick=onclick_js
                                                        on:click=move |_| {
                                                            wasm_client::satya_select_theme_preset(
                                                                theme_id.to_string(),
                                                            )
                                                        }
                                                    >
                                                        <span class="theme-swatch-dot" style=dot_style></span>
                                                        <span>{short_label}</span>
                                                    </button>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                    {APPEARANCE_SELECT_SPECS
                                        .iter()
                                        .copied()
                                        .map(|(sel_id, sel_name, label, options)| {
                                            view! {
                                                <label for=sel_id>
                                                    <span>{label}</span>
                                                    <select
                                                        id=sel_id
                                                        name=sel_name
                                                        onchange="window.satyaApplyAppearanceSettings()"
                                                        on:change=move |_| {
                                                            wasm_client::satya_apply_appearance_settings()
                                                        }
                                                    >
                                                        {options
                                                            .iter()
                                                            .copied()
                                                            .map(|(val, opt_label)| {
                                                                view! { <option value=val>{opt_label}</option> }
                                                            })
                                                            .collect_view()}
                                                    </select>
                                                </label>
                                            }
                                        })
                                        .collect_view()}
                                    <button
                                        type="button"
                                        id="btn-reset-appearance-defaults"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaResetAppearanceSettings()"
                                        on:click=move |_| wasm_client::satya_reset_appearance_settings()
                                    >
                                        "Reset Appearance Defaults"
                                    </button>
                                </div>
                                <button
                                    type="button"
                                    id="btn-menu-toggle-chat-settings"
                                    class="btn btn-sm btn-secondary"
                                    onclick="window.satyaToggleChatSettings()"
                                    on:click=move |_| wasm_client::satya_toggle_chat_settings()
                                >
                                    "Chat"
                                </button>
                                <div
                                    id="menu-chat-settings-panel"
                                    class="menu-chat-settings-panel"
                                    style="display: none;"
                                >
                                    <span class="filter-label">"Show Message Types:"</span>
                                    <div class="filter-checkboxes-vertical">
                                        {CHAT_FILTER_SPECS
                                            .iter()
                                            .copied()
                                            .map(|(input_id, val, label)| {
                                                view! {
                                                    <label class="recip-option">
                                                        <input
                                                            id=input_id
                                                            type="checkbox"
                                                            value=val
                                                            checked
                                                            onchange="window.satyaFilterChat()"
                                                            on:change=move |_| wasm_client::satya_filter_chat()
                                                        />
                                                        <span>{label}</span>
                                                    </label>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                                <button
                                    type="button"
                                    id="btn-menu-toggle-ai-gm-settings"
                                    class="btn btn-sm btn-secondary"
                                    onclick="window.satyaToggleAiGmSettings()"
                                    on:click=move |_| wasm_client::satya_toggle_ai_gm_settings()
                                >
                                    "AI GM (BYOK)"
                                </button>
                                <div
                                    id="menu-ai-gm-settings-panel"
                                    class="menu-chat-settings-panel"
                                    style="display: none;"
                                >
                                    <label for="menu-ai-gm-provider">
                                        <span>"LLM Provider"</span>
                                        <select
                                            id="menu-ai-gm-provider"
                                            name="menu_ai_gm_provider"
                                            onchange="window.satyaMenuAiGmProviderChanged()"
                                            on:change=move |_| wasm_client::satya_menu_ai_gm_provider_changed()
                                        >
                                            {AI_GM_PROVIDER_OPTIONS
                                                .iter()
                                                .copied()
                                                .map(|(val, label)| {
                                                    view! { <option value=val>{label}</option> }
                                                })
                                                .collect_view()}
                                        </select>
                                    </label>
                                    <label for="menu-ai-gm-model">
                                        <span>"Model ID"</span>
                                        <input
                                            id="menu-ai-gm-model"
                                            name="menu_ai_gm_model"
                                            type="text"
                                            autocorrect="off"
                                            autocapitalize="off"
                                            spellcheck="false"
                                            value="gemma4:12b"
                                        />
                                    </label>
                                    <label for="menu-ai-gm-key">
                                        <span>"Ephemeral BYOK API Key"</span>
                                        <input
                                            id="menu-ai-gm-key"
                                            name="menu_ai_gm_key"
                                            type="password"
                                            autocomplete="off"
                                            autocorrect="off"
                                            autocapitalize="off"
                                            spellcheck="false"
                                            placeholder="In-memory session key only"
                                        />
                                    </label>
                                    <button
                                        type="button"
                                        id="btn-apply-ai-gm-config"
                                        class="btn btn-sm btn-accent"
                                        onclick="window.satyaUpdateSessionGmConfig()"
                                        on:click=move |_| wasm_client::satya_update_session_gm_config()
                                    >
                                        "Apply Session AI GM Config"
                                    </button>
                                    <div id="menu-ai-gm-feedback" class="feedback-box" aria-live="polite"></div>
                                </div>
                                <button
                                    type="button"
                                    id="btn-menu-toggle-ai-player-settings"
                                    class="btn btn-sm btn-secondary"
                                    onclick="window.satyaToggleAiPlayerSettings()"
                                    on:click=move |_| wasm_client::satya_toggle_ai_player_settings()
                                >
                                    "AI Player (Test Bot)"
                                </button>
                                <div
                                    id="menu-ai-player-settings-panel"
                                    class="menu-chat-settings-panel"
                                    style="display: none;"
                                >
                                    <label for="menu-ai-player-name">
                                        <span>"AI Player Name"</span>
                                        <input
                                            id="menu-ai-player-name"
                                            name="menu_ai_player_name"
                                            type="text"
                                            value="Kaelen Voss (AI Player)"
                                        />
                                    </label>
                                    <label for="menu-ai-player-persona">
                                        <span>"Playstyle / Persona"</span>
                                        <input
                                            id="menu-ai-player-persona"
                                            name="menu_ai_player_persona"
                                            type="text"
                                            value="Tactical recon scout; favors disciplined Tempo/Control fire."
                                        />
                                    </label>
                                    <label class="recip-option" for="menu-ai-player-auto">
                                        <input
                                            id="menu-ai-player-auto"
                                            type="checkbox"
                                            checked
                                        />
                                        <span>"Auto-Respond to Party Turn Gate"</span>
                                    </label>
                                    <button
                                        type="button"
                                        id="btn-spawn-ai-player"
                                        class="btn btn-sm btn-accent"
                                        onclick="window.satyaSpawnSessionAiPlayer()"
                                        on:click=move |_| wasm_client::satya_spawn_session_ai_player()
                                    >
                                        "Spawn / Update AI Player"
                                    </button>
                                    <button
                                        type="button"
                                        id="btn-step-ai-player"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaStepSessionAiPlayer()"
                                        on:click=move |_| wasm_client::satya_step_session_ai_player()
                                    >
                                        "Step AI Player Turn"
                                    </button>
                                    <div id="menu-ai-player-feedback" class="feedback-box" aria-live="polite"></div>
                                </div>
                            </div>
                            <button
                                type="button"
                                id="btn-menu-logout"
                                class="btn btn-sm btn-danger"
                                onclick="window.satyaLogout()"
                                on:click=move |_| wasm_client::satya_logout()
                                disabled
                            >
                                "Logout"
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </header>
    }
}
