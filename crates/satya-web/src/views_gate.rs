//! Top-bar navigation header (`HeaderBar`), hamburger menu settings, and the pre-session
//! authentication and password-protected session gate (`AuthAndSessionGate`).

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

/// Static table of quick-fill demo accounts for testing TTRPG and Meeting sessions.
const DEMO_ACCOUNT_SPECS: &[(&str, &str, &str, &str)] = &[
    ("gm_sean", "satya-truth-2026", "Sean (GM)", "GM: gm_sean"),
    (
        "sara_valkyrie",
        "satya-truth-2026",
        "Sara (Valkyrie)",
        "Player: sara_valkyrie",
    ),
    (
        "marcus_recon",
        "satya-truth-2026",
        "Marcus (Recon)",
        "Player: marcus_recon",
    ),
    (
        "host_elena",
        "satya-truth-2026",
        "Elena (Meeting Host)",
        "Host: host_elena",
    ),
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
                                            <option value="ollama">"Ollama (Local)"</option>
                                            <option value="google_genai">"Google Gemini (BYOK)"</option>
                                            <option value="openai">"OpenAI (BYOK)"</option>
                                            <option value="anthropic">"Anthropic Claude (BYOK)"</option>
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

/// Pre-session user authentication card (`#auth-card`) and session creation/join card (`#session-card`).
#[component]
pub fn AuthAndSessionGate() -> impl IntoView {
    view! {
        <section id="gate-panel" class="gate-container">
            <div id="auth-card" class="card auth-card">
                <div class="card-header">
                    <h2>"1. User Authentication"</h2>
                    <span class="subtitle">"Persistent user_id across sessions for ML & analytics"</span>
                </div>
                <div class="preset-users">
                    <span class="preset-label">"Quick Fill Demo Accounts:"</span>
                    {DEMO_ACCOUNT_SPECS
                        .iter()
                        .copied()
                        .map(|(user, pass, display, chip_label)| {
                            let onclick_js =
                                format!("window.satyaFillDemo('{user}', '{pass}', '{display}')");
                            view! {
                                <button
                                    type="button"
                                    class="btn-chip"
                                    onclick=onclick_js
                                    on:click=move |_| {
                                        wasm_client::satya_fill_demo(
                                            user.to_string(),
                                            pass.to_string(),
                                            display.to_string(),
                                        )
                                    }
                                >
                                    {chip_label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="form-grid">
                    <label>
                        <span>"Username"</span>
                        <input id="auth-username" type="text" placeholder="e.g. gm_sean" value="gm_sean"/>
                    </label>
                    <label>
                        <span>"Password"</span>
                        <div class="password-field">
                            <input id="auth-password" type="password" placeholder="Password" value="satya-truth-2026"/>
                            <PasswordToggleButton
                                input_id="auth-password"
                                btn_id="btn-toggle-auth-password"
                            />
                        </div>
                    </label>
                    <label>
                        <span>"Display / Character Name"</span>
                        <input id="auth-display-name" type="text" placeholder="e.g. Sean (GM)" value="Sean (GM)"/>
                    </label>
                </div>
                <div class="btn-row">
                    <button
                        type="button"
                        class="btn btn-primary"
                        onclick="window.satyaLogin()"
                        on:click=move |_| wasm_client::satya_login()
                    >
                        "Login"
                    </button>
                    <button
                        type="button"
                        class="btn btn-secondary"
                        onclick="window.satyaRegister()"
                        on:click=move |_| wasm_client::satya_register()
                    >
                        "Register New Account"
                    </button>
                    <button
                        type="button"
                        id="btn-google-auth"
                        class="btn btn-secondary"
                        title="Link Google account with non-sensitive `drive.file` single-file Picker scope for GM_ONLY Google Doc/Sheet import"
                        onclick="window.satyaLinkGoogleAccount()"
                        on:click=move |_| wasm_client::satya_link_google_account()
                    >
                        "Link Google (drive.file)"
                    </button>
                    <span id="google-auth-status-pill" class="pill pill-muted">"Google Not Linked"</span>
                    <button
                        type="button"
                        id="btn-logout"
                        class="btn btn-danger"
                        onclick="window.satyaLogout()"
                        on:click=move |_| wasm_client::satya_logout()
                        disabled
                    >
                        "Logout"
                    </button>
                </div>
                <div id="auth-feedback" class="feedback-box"></div>
            </div>

            <div id="session-card" class="card session-card" style="display: none;">
                <div class="card-header">
                    <h2>"2. Create or Join Password-Protected Session"</h2>
                    <span class="subtitle">"TTRPG (GM / AI-GM / Players) or Meeting (Host / Participants)"</span>
                </div>
                <div class="form-grid">
                    <label>
                        <span>"Session ID"</span>
                        <input id="session-id-input" type="text" placeholder="e.g. harsh-reality-ep1" value="harsh-reality-ep1"/>
                    </label>
                    <label>
                        <span>"Session Password"</span>
                        <div class="password-field">
                            <input id="session-password-input" type="password" placeholder="Session password" value="orbital-drop-7"/>
                            <PasswordToggleButton
                                input_id="session-password-input"
                                btn_id="btn-toggle-session-password"
                            />
                        </div>
                    </label>
                    <label>
                        <span>"Business Mode"</span>
                        <select
                            id="session-mode-select"
                            onchange="window.satyaModeChanged()"
                            on:change=move |_| wasm_client::satya_mode_changed()
                        >
                            <option value="TTRPG">"TTRPG Mode (GM + Players)"</option>
                            <option value="TTRPG_GM_AI">"TTRPG Mode (GM + AI + Players)"</option>
                            <option value="TTRPG_AI_GM">"TTRPG Mode (AI GM + Players)"</option>
                            <option value="MEETING">"Meeting Mode (Host + 2+ Attendees)"</option>
                        </select>
                    </label>
                    <label>
                        <span>"Your Session Role"</span>
                        <select id="session-role-select">
                            <option value="GM">"Game Master (GM)"</option>
                            <option value="PLAYER">"Player"</option>
                            <option value="OBSERVER">"Observer"</option>
                        </select>
                    </label>
                    <input id="enable-ai-gm" type="checkbox" style="display: none"/>
                </div>
                <div id="ai-gm-config-fields" class="form-grid ai-gm-config-grid" style="display: none;">
                    <label for="ai-gm-provider-select">
                        <span>"AI GM Provider"</span>
                        <select
                            id="ai-gm-provider-select"
                            name="ai_gm_provider"
                            onchange="window.satyaAiGmProviderChanged()"
                            on:change=move |_| wasm_client::satya_ai_gm_provider_changed()
                        >
                            <option value="ollama">"Ollama (Local - Default)"</option>
                            <option value="google_genai">"Google Gemini (BYOK / Env)"</option>
                            <option value="openai">"OpenAI (BYOK / Env)"</option>
                            <option value="anthropic">"Anthropic Claude (BYOK / Env)"</option>
                        </select>
                    </label>
                    <label for="ai-gm-model-input">
                        <span>"AI GM Model ID"</span>
                        <input
                            id="ai-gm-model-input"
                            name="ai_gm_model"
                            type="text"
                            autocorrect="off"
                            autocapitalize="off"
                            spellcheck="false"
                            placeholder="e.g. gemma4:12b or gemini-2.5-flash"
                            value="gemma4:12b"
                        />
                    </label>
                    <label for="ai-gm-api-key-input">
                        <span>"Ephemeral BYOK API Key (RAM Only)"</span>
                        <div class="password-field">
                            <input
                                id="ai-gm-api-key-input"
                                name="ai_gm_api_key"
                                type="password"
                                autocomplete="off"
                                autocorrect="off"
                                autocapitalize="off"
                                spellcheck="false"
                                placeholder="Optional cloud API key (never logged)"
                            />
                            <PasswordToggleButton
                                input_id="ai-gm-api-key-input"
                                btn_id="btn-toggle-ai-gm-key"
                            />
                        </div>
                    </label>
                </div>
                <div class="btn-row">
                    <button
                        type="button"
                        class="btn btn-accent"
                        onclick="window.satyaCreateSession()"
                        on:click=move |_| wasm_client::satya_create_session()
                    >
                        "Host: Create Session"
                    </button>
                    <button
                        type="button"
                        class="btn btn-primary"
                        onclick="window.satyaJoinSession()"
                        on:click=move |_| wasm_client::satya_join_session()
                    >
                        "Join Existing Session"
                    </button>
                    <button
                        type="button"
                        id="btn-leave-session"
                        class="btn btn-danger"
                        onclick="window.satyaLeaveSession()"
                        on:click=move |_| wasm_client::satya_leave_session()
                        disabled
                    >
                        "Disconnect Session"
                    </button>
                </div>
                <div id="session-feedback" class="feedback-box"></div>
            </div>
        </section>
    }
}
