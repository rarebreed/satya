//! Full-stack Rust + Leptos + WebAssembly frontend for the Satya WebRTC, TTRPG,
//! and Apache Iggy Event Streaming Platform.
#![deny(missing_docs)]

pub mod chat_cmd;
pub mod wasm_client;

use leptos::prelude::*;

/// Minimal 4-line ES module loader that boots the compiled `satya-web` Rust/Wasm module
/// (`/pkg/satya_web.js` + `/pkg/satya_web_bg.wasm`), invoking `#[wasm_bindgen(start)]`.
const WASM_BOOTSTRAP_LOADER: &str = r#"
import init from "/pkg/satya_web.js";
init().catch((err) => console.warn("Satya Wasm module init:", err));
"#;

/// Renders the full-stack Leptos application shell for Satya WebRTC, Chat & Avro Telemetry.
pub fn render_satya_app_html() -> String {
    SatyaApp().to_html()
}

/// Root Leptos component rendering the Satya WebRTC, Chat, TTRPG Mechanics, and Avro Inspector UI.
#[component]
pub fn SatyaApp() -> impl IntoView {
    let app_styles = include_str!("styles.css");

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>"SATYA // WebRTC, TTRPG & Meeting Event Streaming Platform"</title>
                <style inner_html=app_styles></style>
            </head>
            <body>
                <div class="app-shell">
                    <HeaderBar/>
                    <main class="workspace-grid">
                        <AuthAndSessionGate/>
                        <ActiveSessionWorkspace/>
                    </main>
                </div>
                <script type="module" inner_html=WASM_BOOTSTRAP_LOADER></script>
            </body>
        </html>
    }
}

#[component]
fn HeaderBar() -> impl IntoView {
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
                                        <label class="recip-option">
                                            <input
                                                id="filter-msg-public"
                                                type="checkbox"
                                                value="PUBLIC"
                                                checked
                                                onchange="window.satyaFilterChat()"
                                                on:change=move |_| wasm_client::satya_filter_chat()
                                            />
                                            <span>"Public"</span>
                                        </label>
                                        <label class="recip-option">
                                            <input
                                                id="filter-msg-private"
                                                type="checkbox"
                                                value="PRIVATE"
                                                checked
                                                onchange="window.satyaFilterChat()"
                                                on:change=move |_| wasm_client::satya_filter_chat()
                                            />
                                            <span>"Private / Whisper"</span>
                                        </label>
                                        <label class="recip-option">
                                            <input
                                                id="filter-msg-ai-gm"
                                                type="checkbox"
                                                value="AI_GM"
                                                checked
                                                onchange="window.satyaFilterChat()"
                                                on:change=move |_| wasm_client::satya_filter_chat()
                                            />
                                            <span>"AI GM"</span>
                                        </label>
                                        <label class="recip-option">
                                            <input
                                                id="filter-msg-mechanics"
                                                type="checkbox"
                                                value="MECHANICS"
                                                checked
                                                onchange="window.satyaFilterChat()"
                                                on:change=move |_| wasm_client::satya_filter_chat()
                                            />
                                            <span>"Mechanics"</span>
                                        </label>
                                        <label class="recip-option">
                                            <input
                                                id="filter-msg-system"
                                                type="checkbox"
                                                value="SYSTEM"
                                                checked
                                                onchange="window.satyaFilterChat()"
                                                on:change=move |_| wasm_client::satya_filter_chat()
                                            />
                                            <span>"System (RUST/WASM)"</span>
                                        </label>
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

#[component]
fn AuthAndSessionGate() -> impl IntoView {
    view! {
        <section id="gate-panel" class="gate-container">
            <div id="auth-card" class="card auth-card">
                <div class="card-header">
                    <h2>"1. User Authentication"</h2>
                    <span class="subtitle">"Persistent user_id across sessions for ML & analytics"</span>
                </div>
                <div class="preset-users">
                    <span class="preset-label">"Quick Fill Demo Accounts:"</span>
                    <button
                        type="button"
                        class="btn-chip"
                        onclick="window.satyaFillDemo('gm_sean', 'satya-truth-2026', 'Sean (GM)')"
                        on:click=move |_| wasm_client::satya_fill_demo(
                            "gm_sean".to_string(),
                            "satya-truth-2026".to_string(),
                            "Sean (GM)".to_string(),
                        )
                    >
                        "GM: gm_sean"
                    </button>
                    <button
                        type="button"
                        class="btn-chip"
                        onclick="window.satyaFillDemo('sara_valkyrie', 'satya-truth-2026', 'Sara (Valkyrie)')"
                        on:click=move |_| wasm_client::satya_fill_demo(
                            "sara_valkyrie".to_string(),
                            "satya-truth-2026".to_string(),
                            "Sara (Valkyrie)".to_string(),
                        )
                    >
                        "Player: sara_valkyrie"
                    </button>
                    <button
                        type="button"
                        class="btn-chip"
                        onclick="window.satyaFillDemo('marcus_recon', 'satya-truth-2026', 'Marcus (Recon)')"
                        on:click=move |_| wasm_client::satya_fill_demo(
                            "marcus_recon".to_string(),
                            "satya-truth-2026".to_string(),
                            "Marcus (Recon)".to_string(),
                        )
                    >
                        "Player: marcus_recon"
                    </button>
                    <button
                        type="button"
                        class="btn-chip"
                        onclick="window.satyaFillDemo('host_elena', 'satya-truth-2026', 'Elena (Meeting Host)')"
                        on:click=move |_| wasm_client::satya_fill_demo(
                            "host_elena".to_string(),
                            "satya-truth-2026".to_string(),
                            "Elena (Meeting Host)".to_string(),
                        )
                    >
                        "Host: host_elena"
                    </button>
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
                            <button
                                type="button"
                                id="btn-toggle-auth-password"
                                class="btn-password-toggle"
                                title="Show password"
                                aria-label="Show password"
                                aria-pressed="false"
                                onclick="window.satyaTogglePassword('auth-password', 'btn-toggle-auth-password')"
                                on:click=move |_| wasm_client::satya_toggle_password(
                                    "auth-password".to_string(),
                                    "btn-toggle-auth-password".to_string(),
                                )
                                inner_html=wasm_client::EYE_OPEN_SVG
                            ></button>
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
                            <button
                                type="button"
                                id="btn-toggle-session-password"
                                class="btn-password-toggle"
                                title="Show password"
                                aria-label="Show password"
                                aria-pressed="false"
                                onclick="window.satyaTogglePassword('session-password-input', 'btn-toggle-session-password')"
                                on:click=move |_| wasm_client::satya_toggle_password(
                                    "session-password-input".to_string(),
                                    "btn-toggle-session-password".to_string(),
                                )
                                inner_html=wasm_client::EYE_OPEN_SVG
                            ></button>
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
                            <button
                                type="button"
                                id="btn-toggle-ai-gm-key"
                                class="btn-password-toggle"
                                title="Show password"
                                aria-label="Show password"
                                aria-pressed="false"
                                onclick="window.satyaTogglePassword('ai-gm-api-key-input', 'btn-toggle-ai-gm-key')"
                                on:click=move |_| wasm_client::satya_toggle_password(
                                    "ai-gm-api-key-input".to_string(),
                                    "btn-toggle-ai-gm-key".to_string(),
                                )
                                inner_html=wasm_client::EYE_OPEN_SVG
                            ></button>
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

#[component]
fn ActiveSessionWorkspace() -> impl IntoView {
    view! {
        <section id="workspace-panel" class="workspace-columns" style="display: none;">
            // ================================================================
            // LEFT COLUMN: Tabbed "Notebook" UI
            // (Maps, 3D Dice, 3D Model, Character Sheet/Status, Combat Tracker,
            //  Apache Iggy / Topics / PyLance stats)
            // ================================================================
            <div id="notebook-column" class="col-notebook">
                <div id="tabletop-canvas-card" class="card tabletop-canvas-card notebook-card">
                    <div class="notebook-header">
                        <div class="notebook-title-row">
                            <div>
                                <h2>"Tabletop Notebook"</h2>
                                <span class="subtitle">"Maps · 3D Dice · 3D Model · Character Sheet · Combat Tracker · Apache Iggy & PyLance"</span>
                            </div>
                            <span id="canvas-map-pill" class="pill pill-muted">"Standby Grid"</span>
                        </div>
                        <div class="notebook-tabs" role="tablist" aria-label="Tabletop Notebook Views">
                            <button
                                type="button"
                                id="tab-btn-maps"
                                role="tab"
                                aria-selected="true"
                                aria-controls="notebook-pane-maps"
                                data-notebook-tab="maps"
                                class="notebook-tab active"
                                onclick="window.satyaSelectNotebookTab('maps')"
                                on:click=move |_| wasm_client::satya_select_notebook_tab("maps".to_string())
                            >
                                "Maps"
                            </button>
                            <button
                                type="button"
                                id="tab-btn-dice"
                                role="tab"
                                aria-selected="false"
                                aria-controls="notebook-pane-dice"
                                data-notebook-tab="dice"
                                class="notebook-tab"
                                onclick="window.satyaSelectNotebookTab('dice')"
                                on:click=move |_| wasm_client::satya_select_notebook_tab("dice".to_string())
                            >
                                "3D Dice"
                            </button>
                            <button
                                type="button"
                                id="tab-btn-model"
                                role="tab"
                                aria-selected="false"
                                aria-controls="notebook-pane-model"
                                data-notebook-tab="model"
                                class="notebook-tab"
                                onclick="window.satyaSelectNotebookTab('model')"
                                on:click=move |_| wasm_client::satya_select_notebook_tab("model".to_string())
                            >
                                "3D Model"
                            </button>
                            <button
                                type="button"
                                id="tab-btn-character"
                                role="tab"
                                aria-selected="false"
                                aria-controls="notebook-pane-character"
                                data-notebook-tab="character"
                                class="notebook-tab"
                                onclick="window.satyaSelectNotebookTab('character')"
                                on:click=move |_| wasm_client::satya_select_notebook_tab("character".to_string())
                            >
                                "Character Sheet/Status"
                            </button>
                            <button
                                type="button"
                                id="tab-btn-combat"
                                role="tab"
                                aria-selected="false"
                                aria-controls="notebook-pane-combat"
                                data-notebook-tab="combat"
                                class="notebook-tab"
                                onclick="window.satyaSelectNotebookTab('combat')"
                                on:click=move |_| wasm_client::satya_select_notebook_tab("combat".to_string())
                            >
                                "Combat Tracker"
                            </button>
                            <button
                                type="button"
                                id="tab-btn-rules"
                                role="tab"
                                aria-selected="false"
                                aria-controls="notebook-pane-rules"
                                data-notebook-tab="rules"
                                class="notebook-tab"
                                onclick="window.satyaSelectNotebookTab('rules')"
                                on:click=move |_| wasm_client::satya_select_notebook_tab("rules".to_string())
                            >
                                "Rules Book"
                            </button>
                            <button
                                type="button"
                                id="tab-btn-docs"
                                role="tab"
                                aria-selected="false"
                                aria-controls="notebook-pane-docs"
                                data-notebook-tab="docs"
                                class="notebook-tab"
                                onclick="window.satyaSelectNotebookTab('docs')"
                                on:click=move |_| wasm_client::satya_select_notebook_tab("docs".to_string())
                            >
                                "GM Docs"
                            </button>
                            <button
                                type="button"
                                id="tab-btn-iggy"
                                role="tab"
                                aria-selected="false"
                                aria-controls="notebook-pane-iggy"
                                data-notebook-tab="iggy"
                                class="notebook-tab"
                                onclick="window.satyaSelectNotebookTab('iggy')"
                                on:click=move |_| wasm_client::satya_select_notebook_tab("iggy".to_string())
                            >
                                "Apache Iggy, Topics & PyLance"
                            </button>
                        </div>
                    </div>

                    <div class="notebook-body">
                        // --- TAB 1: MAPS ---
                        <div
                            id="notebook-pane-maps"
                            role="tabpanel"
                            aria-labelledby="tab-btn-maps"
                            data-notebook-pane="maps"
                            class="notebook-pane active"
                        >
                            <div class="pane-toolbar">
                                <span class="subtitle">"Live SVG Battlemap (`MAP_UPDATED`) & Tactical Grid Coordinates"</span>
                            </div>
                            <div id="tabletop-map-viewport" class="tabletop-map-viewport">
                                <div id="tabletop-map-placeholder" class="tabletop-map-placeholder">
                                    "Awaiting tactical map generation (`generate_tactical_map`) or GM map update..."
                                </div>
                                <img
                                    id="tabletop-map-img"
                                    class="tabletop-map-img"
                                    alt="Tactical Battlemap"
                                    style="display: none;"
                                />
                            </div>
                            <div class="roster-bar">
                                <span class="roster-title">"Grid Tokens & Coordinates:"</span>
                                <div id="tabletop-token-list" class="roster-chips">
                                    <span class="pill pill-muted">"No active tokens"</span>
                                </div>
                            </div>
                        </div>

                        // --- TAB 2: 3D DICE ---
                        <div
                            id="notebook-pane-dice"
                            role="tabpanel"
                            aria-labelledby="tab-btn-dice"
                            data-notebook-pane="dice"
                            class="notebook-pane"
                            style="display: none;"
                        >
                            <div class="pane-toolbar">
                                <div>
                                    <span class="subtitle">"Synchronized 3D Physics Dice (`WGPU_DICE_ROLL` · Deterministic Seed Sync)"</span>
                                </div>
                                <div class="btn-row-compact">
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaQuick3dDiceRoll('2d20')"
                                        on:click=move |_| wasm_client::satya_quick_3d_dice_roll("2d20".to_string())
                                    >
                                        "Roll 2d20"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaQuick3dDiceRoll('4d20')"
                                        on:click=move |_| wasm_client::satya_quick_3d_dice_roll("4d20".to_string())
                                    >
                                        "Roll 4d20"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaQuick3dDiceRoll('6d20')"
                                        on:click=move |_| wasm_client::satya_quick_3d_dice_roll("6d20".to_string())
                                    >
                                        "Roll 6d20"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-accent"
                                        onclick="window.satyaRollTask()"
                                        on:click=move |_| wasm_client::satya_roll_task()
                                    >
                                        "Roll Active Task"
                                    </button>
                                </div>
                            </div>
                            <div id="wgpu-dice-tray" class="wgpu-dice-tray notebook-dice-stage">
                                <span class="roster-title">"3D Physics Dice Tray (`wgpu` Seed Sync):"</span>
                                <div id="wgpu-dice-readout" class="wgpu-dice-readout" aria-live="polite">
                                    "No 3D dice rolls spawned yet."
                                </div>
                                <div id="wgpu-dice-history" class="wgpu-dice-history">
                                    <div class="roster-title">"Recent 3D Roll Log:"</div>
                                    <div id="wgpu-dice-history-list" class="dice-history-list">
                                        <span class="subtitle">"Roll via Die Roller, `/roll`, or quick buttons above to populate 3D trajectory log."</span>
                                    </div>
                                </div>
                            </div>
                        </div>

                        // --- TAB 3: 3D MODEL ---
                        <div
                            id="notebook-pane-model"
                            role="tabpanel"
                            aria-labelledby="tab-btn-model"
                            data-notebook-pane="model"
                            class="notebook-pane"
                            style="display: none;"
                        >
                            <div class="pane-toolbar">
                                <div class="model-controls-row">
                                    <label class="inline-label">
                                        <span>"3D Asset"</span>
                                        <select
                                            id="model-preset-select"
                                            onchange="window.satyaUpdate3dModelView()"
                                            on:change=move |_| wasm_client::satya_update_3d_model_view()
                                        >
                                            <option value="valkyrie_rig">"Valkyrie Exo-Rig (Sara)"</option>
                                            <option value="recon_mk4">"Recon Suit Mk-IV (Marcus)"</option>
                                            <option value="secops_sentry">"SecOps Heavy Sentry"</option>
                                            <option value="sector4_airlock">"Sector 4 Orbital Airlock"</option>
                                        </select>
                                    </label>
                                    <label class="inline-label">
                                        <span>"Shader Mode"</span>
                                        <select
                                            id="model-render-mode"
                                            onchange="window.satyaUpdate3dModelView()"
                                            on:change=move |_| wasm_client::satya_update_3d_model_view()
                                        >
                                            <option value="wireframe_pbr">"Wireframe + PBR"</option>
                                            <option value="hit_zones">"Anatomical Hit Zones"</option>
                                            <option value="thermal">"Thermal / IR Sensor"</option>
                                        </select>
                                    </label>
                                </div>
                                <span id="model-telemetry-pill" class="pill pill-iggy">"Mesh: Valkyrie Exo-Rig · 14.2k tris"</span>
                            </div>
                            <div
                                id="model-preview-stage"
                                class="model-preview-stage"
                                inner_html=wasm_client:: default_3d_model_stage_html()
                            ></div>
                        </div>

                        // --- TAB 4: CHARACTER SHEET / STATUS ---
                        <div
                            id="notebook-pane-character"
                            role="tabpanel"
                            aria-labelledby="tab-btn-character"
                            data-notebook-pane="character"
                            class="notebook-pane"
                            style="display: none;"
                        >
                            <div class="pane-toolbar">
                                <span class="subtitle">"Satya d20 Operator Profile, Attributes & Quick-Load Skill Presets"</span>
                                <span id="char-sheet-status-pill" class="pill pill-active">"Combat Ready · Fatigue 1 · Shock 0.0"</span>
                            </div>
                            <div class="char-sheet-grid">
                                <div class="char-stat-box">
                                    <span class="stat-label">"Control (C)"</span>
                                    <span id="char-stat-control" class="stat-value">"12"</span>
                                    <span class="stat-note">"Base Precision"</span>
                                </div>
                                <div class="char-stat-box">
                                    <span class="stat-label">"Reflexes"</span>
                                    <span class="stat-value">"14"</span>
                                    <span class="stat-note">"Initiative / Dodge"</span>
                                </div>
                                <div class="char-stat-box">
                                    <span class="stat-label">"Stamina"</span>
                                    <span class="stat-value">"13"</span>
                                    <span class="stat-note">"Endurance / Soak"</span>
                                </div>
                                <div class="char-stat-box">
                                    <span class="stat-label">"Intellect"</span>
                                    <span class="stat-value">"15"</span>
                                    <span class="stat-note">"Medical / Tech"</span>
                                </div>
                                <div class="char-stat-box">
                                    <span class="stat-label">"Resolve"</span>
                                    <span class="stat-value">"12"</span>
                                    <span class="stat-note">"Composure / Stress"</span>
                                </div>
                                <div class="char-stat-box">
                                    <span class="stat-label">"Armor Mitigation"</span>
                                    <span class="stat-value">"DR 4"</span>
                                    <span class="stat-note">"EVA Ballistic"</span>
                                </div>
                            </div>
                            <div class="roster-bar">
                                <span class="roster-title">"Trained Skills (Click to Load into Die Roller):"</span>
                                <div class="preset-users">
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadSkillPreset('Trauma Surgery', 3, 12)"
                                        on:click=move |_| wasm_client::satya_load_skill_preset("Trauma Surgery".to_string(), 3, 12)
                                    >
                                        "Trauma Surgery (N=3, C=12)"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadSkillPreset('Zero-G Maneuver', 4, 14)"
                                        on:click=move |_| wasm_client::satya_load_skill_preset("Zero-G Maneuver".to_string(), 4, 14)
                                    >
                                        "Zero-G Maneuver (N=4, C=14)"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadSkillPreset('Suppressive Fire', 3, 13)"
                                        on:click=move |_| wasm_client::satya_load_skill_preset("Suppressive Fire".to_string(), 3, 13)
                                    >
                                        "Suppressive Fire (N=3, C=13)"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadSkillPreset('Sensor Recon', 2, 15)"
                                        on:click=move |_| wasm_client::satya_load_skill_preset("Sensor Recon".to_string(), 2, 15)
                                    >
                                        "Sensor Recon (N=2, C=15)"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadSkillPreset('Close-Quarters Breach', 4, 11)"
                                        on:click=move |_| wasm_client::satya_load_skill_preset("Close-Quarters Breach".to_string(), 4, 11)
                                    >
                                        "Close-Quarters Breach (N=4, C=11)"
                                    </button>
                                </div>
                            </div>
                        </div>

                        // --- TAB 5: COMBAT TRACKER ---
                        <div
                            id="notebook-pane-combat"
                            role="tabpanel"
                            aria-labelledby="tab-btn-combat"
                            data-notebook-pane="combat"
                            class="notebook-pane"
                            style="display: none;"
                        >
                            <div class="pane-toolbar">
                                <div>
                                    <span class="subtitle">"Satya Action-Count (Tick) Initiative Timeline & Stance Economy"</span>
                                </div>
                                <div class="btn-row-compact">
                                    <span id="combat-round-pill" class="pill pill-active">"Round 1 · Tick 0"</span>
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaAdvanceCombatTick(1)"
                                        on:click=move |_| wasm_client::satya_advance_combat_tick(1)
                                    >
                                        "+1 Tick"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaAdvanceCombatTick(5)"
                                        on:click=move |_| wasm_client::satya_advance_combat_tick(5)
                                    >
                                        "+5 Ticks"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-accent"
                                        onclick="window.satyaAdvanceCombatTick(0)"
                                        on:click=move |_| wasm_client::satya_advance_combat_tick(0)
                                    >
                                        "Reset Clock"
                                    </button>
                                </div>
                            </div>
                            <div class="roster-bar">
                                <span class="roster-title">"Active Combatants & Session Order:"</span>
                                <div id="combat-tracker-list" class="combat-tracker-list">
                                    <span class="pill pill-muted">"Join or create a session to populate active combatants."</span>
                                </div>
                            </div>
                            <div class="metrics-table-wrap">
                                <table class="metrics-table">
                                    <thead>
                                        <tr>
                                            <th>"Combat Stance"</th>
                                            <th>"Target Control (TC)"</th>
                                            <th>"Action Cost"</th>
                                            <th>"Tactical Effect"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <tr>
                                            <td><strong>"Melee: Maneuver"</strong></td>
                                            <td><span class="mono">"TC 11"</span></td>
                                            <td>"8 ticks"</td>
                                            <td>"Balanced footwork; +1 defensive parry margin"</td>
                                        </tr>
                                        <tr>
                                            <td><strong>"Melee: Power"</strong></td>
                                            <td><span class="mono">"TC 14"</span></td>
                                            <td>"12 ticks"</td>
                                            <td>"Heavy kinetic strike; +2 Wound Shock on hit"</td>
                                        </tr>
                                        <tr>
                                            <td><strong>"Melee: Stability"</strong></td>
                                            <td><span class="mono">"TC 9"</span></td>
                                            <td>"10 ticks"</td>
                                            <td>"Braced zero-G/recoil stance; resists knockdown"</td>
                                        </tr>
                                        <tr>
                                            <td><strong>"Ranged: Aimed"</strong></td>
                                            <td><span class="mono">"TC 12"</span></td>
                                            <td>"11 ticks"</td>
                                            <td>"Deliberate sight acquisition; precision bonus"</td>
                                        </tr>
                                        <tr>
                                            <td><strong>"Ranged: Snap"</strong></td>
                                            <td><span class="mono">"TC 7"</span></td>
                                            <td>"6 ticks"</td>
                                            <td>"Rapid reaction shot; low tick commitment"</td>
                                        </tr>
                                    </tbody>
                                </table>
                            </div>
                        </div>

                        // --- TAB 6: SATYA TTRPG RULES BOOK ---
                        <div
                            id="notebook-pane-rules"
                            role="tabpanel"
                            aria-labelledby="tab-btn-rules"
                            data-notebook-pane="rules"
                            class="notebook-pane"
                            style="display: none;"
                        >
                            <div class="pane-toolbar">
                                <div class="model-controls-row">
                                    <label class="inline-label" for="rules-chapter-select">
                                        <span>"Chapter"</span>
                                        <select
                                            id="rules-chapter-select"
                                            onchange="window.satyaSelectRulesChapter('')"
                                            on:change=move |_| wasm_client::satya_select_rules_chapter(String::new())
                                        >
                                            <option value="background">"1. Design Philosophy & Goals of Satya"</option>
                                            <option value="satya_d20_rules">"2. Core 2d20 Cascading System Rules"</option>
                                            <option value="tasks" selected>"3. Task Resolution & Skill Check Mechanics"</option>
                                            <option value="probability">"4. Mathematical Foundations & Exact Probability Engine"</option>
                                            <option value="characteristics">"5. Core Characteristics & Derived Scales"</option>
                                            <option value="skill-design">"6. Skill Ontology, Lifepath & Non-Linear Progression"</option>
                                            <option value="combat">"7. Combat Overview, Morale & Ranged Firefights"</option>
                                            <option value="melee_combat">"8. Close-Quarters & Melee Combat (Tactical Triad)"</option>
                                            <option value="social_interactions">"9. Social Mechanics & Cognitive Dissonance"</option>
                                        </select>
                                    </label>
                                    <label class="inline-label" for="rules-search-input">
                                        <span>"Search Rules"</span>
                                        <input
                                            id="rules-search-input"
                                            type="search"
                                            placeholder="e.g. TN_eff, botch, Tempo, Dissonance..."
                                            oninput="window.satyaSearchRulesBook()"
                                            onchange="window.satyaSearchRulesBook()"
                                        />
                                    </label>
                                </div>
                                <div class="btn-row-compact">
                                    <span id="rules-chapter-pill" class="pill pill-active">
                                        {wasm_client::format_rules_chapter_html("tasks").1}
                                    </span>
                                    <a
                                        href="/book/index.html"
                                        target="_blank"
                                        rel="noopener"
                                        class="btn btn-sm btn-secondary"
                                    >
                                        "Static Marimo Book"
                                    </a>
                                </div>
                            </div>
                            <div class="roster-bar">
                                <span class="roster-title">"Worked Book Examples (Click to Load into 2d20 Task Roller):"</span>
                                <div class="preset-users">
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadRulesExample('sarah_lockpick')"
                                        on:click=move |_| wasm_client::satya_load_rules_example("sarah_lockpick".to_string())
                                    >
                                        "Sarah's Lockpick (N=4, C=14, Δ_int=+1, Δ_ext=+4)"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadRulesExample('elena_probe')"
                                        on:click=move |_| wasm_client::satya_load_rules_example("elena_probe".to_string())
                                    >
                                        "Elena's Sword Probe (N=5, C=16, Δ_ext=+2)"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadRulesExample('viktor_cleave')"
                                        on:click=move |_| wasm_client::satya_load_rules_example("viktor_cleave".to_string())
                                    >
                                        "Viktor's Greatsword Cleave (N=4, C=13, Δ_ext=+3)"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadRulesExample('social_persuasion')"
                                        on:click=move |_| wasm_client::satya_load_rules_example("social_persuasion".to_string())
                                    >
                                        "Social Rhetoric (N=3, C=15, Δ_int=+2, Δ_ext=+3)"
                                    </button>
                                </div>
                            </div>
                            <div
                                id="rules-search-results"
                                class="gm-docs-list"
                                style="display: none;"
                            ></div>
                            <div
                                id="rules-book-viewport"
                                class="rules-book-viewport"
                                inner_html=wasm_client::default_rules_book_chapter_html()
                            ></div>
                        </div>

                        // --- TAB 7: GM_ONLY DOCUMENTS (TEXT/MARKDOWN & GOOGLE DOCS/SHEETS) ---
                        <div
                            id="notebook-pane-docs"
                            role="tabpanel"
                            aria-labelledby="tab-btn-docs"
                            data-notebook-pane="docs"
                            class="notebook-pane"
                            style="display: none;"
                        >
                            <div class="pane-toolbar">
                                <div>
                                    <span class="subtitle">
                                        "GM_ONLY Document Vault (`Player -> GM & AI GM Only` · Text/Table Extraction · Images & Scripts Stripped)"
                                    </span>
                                </div>
                                <div class="btn-row-compact">
                                    <span id="gm-docs-count-pill" class="pill pill-iggy">"0 GM Docs"</span>
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaRefreshGmDocuments()"
                                        on:click=move |_| wasm_client::satya_refresh_gm_documents()
                                    >
                                        "Refresh"
                                    </button>
                                </div>
                            </div>

                            <div class="roster-bar">
                                <span class="roster-title">"1. Specify File: Upload Local File, Pick from Google Drive (`drive.file`), or Load Demo Preset:"</span>
                                <div class="preset-users">
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadGmDocPreset('doc')"
                                        on:click=move |_| wasm_client::satya_load_gm_doc_preset("doc".to_string())
                                    >
                                        "Demo Preset: Sara Dossier (gdoc-demo-sara)"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn-chip"
                                        onclick="window.satyaLoadGmDocPreset('sheet')"
                                        on:click=move |_| wasm_client::satya_load_gm_doc_preset("sheet".to_string())
                                    >
                                        "Demo Preset: Squad Loadout (gsheet-demo-gear)"
                                    </button>
                                </div>
                                <div id="gm-doc-file-row" class="form-grid">
                                    <label for="gm-doc-file-input">
                                        <span>"Choose Local File or Exported Google Doc/Sheet (.md, .txt, .csv, <= 32 KB)"</span>
                                        <input
                                            id="gm-doc-file-input"
                                            class="gm-doc-file-input"
                                            type="file"
                                            accept=".txt,.md,.markdown,.csv,.tsv"
                                            onchange="window.satyaGmDocFileSelected()"
                                            on:change=move |_| wasm_client::satya_gm_doc_file_selected()
                                        />
                                    </label>
                                </div>
                                <div class="form-grid">
                                    <label>
                                        <span>"Document Title"</span>
                                        <input
                                            id="gm-doc-title"
                                            type="text"
                                            placeholder="e.g. Character Backstory or Gear Sheet"
                                            value="Sara Valkyrie Medical Dossier"
                                        />
                                    </label>
                                    <label>
                                        <span>"Document Source (`GM_ONLY`)"</span>
                                        <select
                                            id="gm-doc-source"
                                            onchange="window.satyaGmDocSourceChanged()"
                                            on:change=move |_| wasm_client::satya_gm_doc_source_changed()
                                        >
                                            <option value="DIRECT_TEXT_UPLOAD">"Direct Text / Markdown / CSV (<= 32 KB)"</option>
                                            <option value="GOOGLE_DOC">"Google Doc (drive.file Single-File Scope)"</option>
                                            <option value="GOOGLE_SHEET">"Google Sheet (drive.file Single-File Scope)"</option>
                                        </select>
                                    </label>
                                </div>
                                <div id="gm-doc-google-row" class="form-grid" style="display: none;">
                                    <label>
                                        <span>"Google Drive File ID or Full Google Docs/Sheets URL (`drive.file`)"</span>
                                        <input
                                            id="gm-doc-google-id"
                                            type="text"
                                            placeholder="Paste https://docs.google.com/document/d/<ID>/edit or File ID"
                                            value="gdoc-demo-sara"
                                            onchange="window.satyaGmDocUrlChanged()"
                                            on:change=move |_| wasm_client::satya_gm_doc_url_changed()
                                        />
                                    </label>
                                    <div class="btn-row">
                                        <button
                                            type="button"
                                            id="btn-google-picker"
                                            class="btn btn-sm btn-primary"
                                            onclick="window.satyaOpenGoogleDrivePicker()"
                                            on:click=move |_| wasm_client::satya_open_google_drive_picker()
                                        >
                                            "Pick from Google Drive (drive.file)..."
                                        </button>
                                        <button
                                            type="button"
                                            id="btn-toggle-google-oauth-cfg"
                                            class="btn btn-sm btn-secondary"
                                            onclick="window.satyaToggleGoogleOauthConfig()"
                                            on:click=move |_| wasm_client::satya_toggle_google_oauth_config()
                                        >
                                            "Google OAuth Credentials"
                                        </button>
                                    </div>
                                </div>
                                <div id="gm-google-oauth-config-row" class="form-grid" style="display: none;">
                                    <label>
                                        <span>"Google Cloud OAuth 2.0 Web Client ID (or set in `config/storage.toml`)"</span>
                                        <input
                                            id="gm-google-client-id"
                                            type="text"
                                            autocorrect="off"
                                            autocapitalize="off"
                                            spellcheck="false"
                                            placeholder="e.g. 123456789-abc.apps.googleusercontent.com"
                                        />
                                    </label>
                                    <label>
                                        <span>"Google Cloud Picker API Key (`google.picker.PickerBuilder`)"</span>
                                        <input
                                            id="gm-google-api-key"
                                            type="password"
                                            autocomplete="off"
                                            autocorrect="off"
                                            autocapitalize="off"
                                            spellcheck="false"
                                            placeholder="Optional Picker API Key (AIza...)"
                                        />
                                    </label>
                                </div>
                                <label>
                                    <span>"Plain-Text / Markdown / CSV Snapshot (Images, HTML & Prompt-Injection Stripped on Ingest)"</span>
                                    <textarea
                                        id="gm-doc-content"
                                        class="gm-doc-textarea"
                                        rows="4"
                                        placeholder="Paste Markdown, character notes, or CSV table here, choose a local file above, or pick from Google Drive..."
                                    ></textarea>
                                </label>
                                <div class="btn-row">
                                    <button
                                        type="button"
                                        id="btn-submit-gm-doc"
                                        class="btn btn-sm btn-accent"
                                        onclick="window.satyaSubmitGmDocument()"
                                        on:click=move |_| wasm_client::satya_submit_gm_document()
                                    >
                                        "Submit to GM Only (Human & AI GM)"
                                    </button>
                                </div>
                                <div id="gm-doc-feedback" class="feedback-box" aria-live="polite"></div>
                            </div>

                            <div class="roster-bar">
                                <span class="roster-title">"Session GM_ONLY Documents (Visible to GM/AI-GM & Uploader Only):"</span>
                                <div id="gm-docs-list" class="gm-docs-list">
                                    <span class="pill pill-muted">"No GM_ONLY documents submitted in this session yet."</span>
                                </div>
                            </div>
                        </div>

                        // --- TAB 7: APACHE IGGY, TOPICS AND PYLANCE STATS ---
                        <div
                            id="notebook-pane-iggy"
                            role="tabpanel"
                            aria-labelledby="tab-btn-iggy"
                            data-notebook-pane="iggy"
                            class="notebook-pane"
                            style="display: none;"
                        >
                            <div id="iggy-non-admin-notice" class="roster-bar">
                                <span class="roster-title">"Apache Iggy Event Stream & PyLance Sink Telemetry"</span>
                                <span class="subtitle">
                                    "Log in with an ADMIN account (e.g. `gm_sean`) to inspect live CRC-64-AVRO fingerprints, topic offsets, and trigger PyLance `.avro` batch flushes."
                                </span>
                            </div>
                            <div id="admin-inspector-card" class="inspector-card-inner" style="display: none;">
                                <div class="card-header-inline">
                                    <div>
                                        <h2>"Apache Iggy Topics, Avro Schemas & PyLance Batch Sink"</h2>
                                        <span class="subtitle">"Admin Only · CRC-64-AVRO Fingerprints + Offset-Tracked .avro Container Batches"</span>
                                    </div>
                                    <div class="btn-row-compact">
                                        <button
                                            type="button"
                                            class="btn btn-sm btn-secondary"
                                            onclick="window.satyaRefreshMetrics()"
                                            on:click=move |_| wasm_client::satya_refresh_metrics()
                                        >
                                            "Refresh"
                                        </button>
                                        <button
                                            type="button"
                                            class="btn btn-sm btn-accent"
                                            onclick="window.satyaFlushBatches()"
                                            on:click=move |_| wasm_client::satya_flush_batches()
                                        >
                                            "Flush Avro Batches Now"
                                        </button>
                                    </div>
                                </div>
                                <div id="topic-metrics-table" class="metrics-table-wrap"></div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            // ================================================================
            // CENTER COLUMN: Chat Session Interface (Top) + Die Roller (Bottom)
            // ================================================================
            <div id="center-column" class="col-center">
                <div class="card chat-card">
                    <div class="card-header-inline">
                        <div>
                            <h2>"Session Chat"</h2>
                            <span class="subtitle">"Broadcast publicly or whisper privately to selected Participant(s)"</span>
                        </div>
                    </div>

                    <div class="recipient-selector-bar">
                        <span class="picker-label">"Send To (Click BROADCAST or Select Participant(s)):"</span>
                        <div id="recipient-checkboxes" class="chat-recipient-list">
                            <label id="recipient-row-broadcast" class="recip-row broadcast-row selected">
                                <input
                                    id="recipient-cb-broadcast"
                                    type="checkbox"
                                    value="BROADCAST"
                                    checked
                                    onchange="window.satyaSelectBroadcast()"
                                    on:change=move |_| wasm_client::satya_select_broadcast()
                                />
                                <span class="recip-name broadcast-tag">"BROADCAST (All Participants)"</span>
                            </label>
                        </div>
                    </div>

                    <div id="chat-log" class="chat-log"></div>

                    <div class="chat-composer">
                        <div class="chat-input-wrap">
                            <input
                                id="chat-input"
                                type="text"
                                placeholder="Type message, /msg @user ..., or /roll 4d10|>hi(3)..."
                                onkeydown="window.satyaChatKeydown(event)"
                                on:keydown=wasm_client::satya_chat_keydown
                            />
                            <div class="chat-ctx-toggles" role="group" aria-label="Message Channel Context">
                                <button
                                    type="button"
                                    id="btn-ctx-ic"
                                    class="btn-ctx-toggle active"
                                    title="IN_CHARACTER (In-Character Roleplay)"
                                    aria-label="IN_CHARACTER"
                                    aria-pressed="true"
                                    onclick="window.satyaSelectChatContext('IN_CHARACTER')"
                                    on:click=move |_| wasm_client::satya_select_chat_context("IN_CHARACTER".to_string())
                                    inner_html=wasm_client::CTX_IC_SVG
                                ></button>
                                <button
                                    type="button"
                                    id="btn-ctx-ooc"
                                    class="btn-ctx-toggle"
                                    title="OUT_OF_CHARACTER (Out-of-Character / Table Talk)"
                                    aria-label="OUT_OF_CHARACTER"
                                    aria-pressed="false"
                                    onclick="window.satyaSelectChatContext('OUT_OF_CHARACTER')"
                                    on:click=move |_| wasm_client::satya_select_chat_context("OUT_OF_CHARACTER".to_string())
                                    inner_html=wasm_client::CTX_OOC_SVG
                                ></button>
                                <button
                                    type="button"
                                    id="btn-ctx-gm"
                                    class="btn-ctx-toggle"
                                    style="display: none;"
                                    title="GM_NARRATION (Game Master Narration)"
                                    aria-label="GM_NARRATION"
                                    aria-pressed="false"
                                    onclick="window.satyaSelectChatContext('GM_NARRATION')"
                                    on:click=move |_| wasm_client::satya_select_chat_context("GM_NARRATION".to_string())
                                    inner_html=wasm_client::CTX_GM_SVG
                                ></button>
                            </div>
                        </div>
                        <button
                            type="button"
                            class="btn btn-primary"
                            onclick="window.satyaSendChat()"
                            on:click=move |_| wasm_client::satya_send_chat()
                        >
                            "Send"
                        </button>
                    </div>
                </div>

                <div id="ttrpg-drawer" class="card ttrpg-card">
                    <div class="card-header-inline">
                        <div>
                            <h2>"Satya TTRPG Task Resolution & Combat Economy"</h2>
                            <span class="subtitle">"Two-List Cascading (`2d20`) Roll-Over Engine · Logs TtrpgMechanicsEvent to Apache Iggy (`ttrpg-mechanics`)"</span>
                        </div>
                        <span id="ttrpg-odds-pill" class="pill pill-active">
                            {wasm_client::format_cascade_odds_preview(
                                &satya_engine::CascadeTaskParams::new_k20(3, 12, 0, 0),
                            )}
                        </span>
                    </div>
                    <div class="ttrpg-controls">
                        <label>
                            <span>"Skill Name"</span>
                            <input id="ttrpg-skill" type="text" value="Trauma Surgery"/>
                        </label>
                        <label>
                            <span>"Skill Level N (2d20 Pairs, 1-10)"</span>
                            <input
                                id="ttrpg-skill-level"
                                type="number"
                                value="3"
                                min="1"
                                max="10"
                                onchange="window.satyaUpdateCascadePreview()"
                                oninput="window.satyaUpdateCascadePreview()"
                            />
                        </label>
                        <label>
                            <span>"Control Characteristic C (1-20)"</span>
                            <input
                                id="ttrpg-control-c"
                                type="number"
                                value="12"
                                min="1"
                                max="20"
                                onchange="window.satyaUpdateCascadePreview()"
                                oninput="window.satyaUpdateCascadePreview()"
                            />
                        </label>
                        <label>
                            <span>"Internal Mod Δ_int (-2 to +10)"</span>
                            <input
                                id="ttrpg-internal-mod"
                                type="number"
                                value="0"
                                min="-2"
                                max="10"
                                onchange="window.satyaUpdateCascadePreview()"
                                oninput="window.satyaUpdateCascadePreview()"
                            />
                        </label>
                        <label>
                            <span>"External Mod Δ_ext (-10 to +16)"</span>
                            <input
                                id="ttrpg-external-mod"
                                type="number"
                                value="0"
                                min="-10"
                                max="16"
                                onchange="window.satyaUpdateCascadePreview()"
                                oninput="window.satyaUpdateCascadePreview()"
                            />
                        </label>
                        <label>
                            <span>"Cascade Carry Mode"</span>
                            <select
                                id="ttrpg-carry-mode"
                                onchange="window.satyaUpdateCascadePreview()"
                            >
                                <option value="markov_step">"Markov Step (Standard)"</option>
                                <option value="cumulative_snowball">"Cumulative Snowball"</option>
                                <option value="linear_margin">"Linear Margin"</option>
                            </select>
                        </label>
                        <label>
                            <span>"Botch Severity Rule"</span>
                            <select
                                id="ttrpg-botch-rule"
                                onchange="window.satyaUpdateCascadePreview()"
                            >
                                <option value="closest_miss" selected>"Closest Miss (Paradox-Free)"</option>
                                <option value="ceiling_breach">"Floor Breach (<2 Complication)"</option>
                            </select>
                        </label>
                        <label>
                            <span>"Combat Stance (Count-Up TC)"</span>
                            <select id="ttrpg-stance">
                                <option value="">"Non-Combat Task (TC 10)"</option>
                                <option value="MANEUVER">"Melee: Maneuver (TC 11)"</option>
                                <option value="POWER">"Melee: Power (TC 14)"</option>
                                <option value="STABILITY">"Melee: Stability (TC 9)"</option>
                                <option value="RANGED_AIMED">"Ranged: Aimed (TC 12)"</option>
                                <option value="RANGED_SNAP">"Ranged: Snap (TC 7)"</option>
                            </select>
                        </label>
                        <label>
                            <span>"Fatigue Level"</span>
                            <input id="ttrpg-fatigue" type="number" value="1" min="0" max="10"/>
                        </label>
                        <label>
                            <span>"Wound Shock Index"</span>
                            <input
                                id="ttrpg-wound-shock"
                                type="number"
                                step="0.5"
                                value="0"
                                min="0"
                                max="10"
                            />
                        </label>
                    </div>
                    <div id="ttrpg-odds-readout" class="ttrpg-odds-readout" aria-live="polite">
                        {
                            wasm_client::format_cascade_odds_summary(
                                3,
                                12,
                                0,
                                0,
                                "markov_step",
                                "closest_miss",
                            )
                            .1
                        }
                    </div>
                    <div class="btn-row">
                        <button
                            type="button"
                            class="btn btn-accent"
                            onclick="window.satyaRollTask()"
                            on:click=move |_| wasm_client::satya_roll_task()
                        >
                            "Roll Satya Task"
                        </button>
                    </div>
                </div>
            </div>

            // ================================================================
            // RIGHT COLUMN / COLLAPSIBLE SLIDE-OUT DRAWER & FLOATING FRAME:
            // Web Cams of Joined Users (1 or at most 2 columns; collapsible to right-edge rail)
            // ================================================================
            <div id="webrtc-dock-column" class="col-webcams">
                <div
                    id="webcam-drawer-rail"
                    class="webcam-drawer-rail"
                    style="display: none;"
                >
                    <button
                        type="button"
                        id="btn-expand-webcam-drawer"
                        class="btn-webcam-drawer-tab"
                        title="Slide out Web Cams Drawer"
                        aria-label="Expand Web Cams Drawer"
                        aria-expanded="false"
                        aria-controls="webrtc-card"
                        onclick="window.satyaToggleWebcamDrawer()"
                        on:click=move |_| wasm_client::satya_toggle_webcam_drawer()
                    >
                        <span class="drawer-tab-chevron" aria-hidden="true">"◂"</span>
                        <span class="drawer-tab-label">"Web Cams"</span>
                        <span id="webcam-drawer-count-badge" class="pill pill-active drawer-count-badge">"1"</span>
                    </button>
                </div>
                <div id="webrtc-card" class="card webrtc-card">
                    <div
                        id="webrtc-drag-header"
                        class="webrtc-header"
                        onmousedown="window.satyaStartWebcamDrag(event)"
                    >
                        <div class="webrtc-title-wrap">
                            <h2>"Web Cams"</h2>
                            <span class="subtitle">"P2P Media & Prosody Mesh"</span>
                        </div>
                        <div class="webcam-frame-actions">
                            <button
                                type="button"
                                id="btn-toggle-webcam-cols"
                                class="btn btn-sm btn-secondary"
                                title="Switch between 1-column and 2-column vertical webcam layout (max 2 columns)"
                                onclick="window.satyaToggleWebcamColumns()"
                                on:click=move |_| wasm_client::satya_toggle_webcam_columns()
                            >
                                "2 Cols"
                            </button>
                            <button
                                type="button"
                                id="btn-toggle-webcam-float"
                                class="btn btn-sm btn-secondary"
                                title="Detach Web Cams into a floating draggable frame or dock back into column"
                                onclick="window.satyaToggleWebcamFloat()"
                                on:click=move |_| wasm_client::satya_toggle_webcam_float()
                            >
                                "Float"
                            </button>
                            <button
                                type="button"
                                id="btn-toggle-webcam-drawer"
                                class="btn btn-sm btn-secondary"
                                title="Collapse Web Cams drawer into the right edge to save workspace space"
                                aria-label="Collapse Web Cams Drawer"
                                aria-expanded="true"
                                aria-controls="webrtc-card"
                                onclick="window.satyaToggleWebcamDrawer()"
                                on:click=move |_| wasm_client::satya_toggle_webcam_drawer()
                            >
                                "Collapse ▸"
                            </button>
                        </div>
                    </div>
                    <div class="media-controls">
                        <button
                            type="button"
                            class="btn btn-sm btn-secondary"
                            onclick="window.satyaStartCamera()"
                            on:click=move |_| wasm_client::satya_start_camera()
                        >
                            "Enable Cam/Mic"
                        </button>
                        <button
                            type="button"
                            id="btn-mute-audio"
                            class="btn btn-sm btn-secondary"
                            onclick="window.satyaToggleAudio()"
                            on:click=move |_| wasm_client::satya_toggle_audio()
                        >
                            "Mute Mic"
                        </button>
                        <button
                            type="button"
                            id="btn-mute-video"
                            class="btn btn-sm btn-secondary"
                            onclick="window.satyaToggleVideo()"
                            on:click=move |_| wasm_client::satya_toggle_video()
                        >
                            "Mute Video"
                        </button>
                        <button
                            type="button"
                            class="btn btn-sm btn-accent"
                            onclick="window.satyaEmitProsodySample()"
                            on:click=move |_| wasm_client::satya_emit_prosody_sample()
                        >
                            "Log VAD Turn"
                        </button>
                        <button
                            type="button"
                            id="btn-workspace-disconnect"
                            class="btn btn-sm btn-danger"
                            onclick="window.satyaLeaveSession()"
                            on:click=move |_| wasm_client::satya_leave_session()
                            disabled
                        >
                            "Disconnect"
                        </button>
                    </div>
                    <div id="video-grid" class="video-grid cols-1">
                        <div class="video-tile local-tile">
                            <video id="local-video" autoplay muted playsinline></video>
                            <div class="tile-overlay">
                                <span id="local-participant-label" class="role-tag">"Local Preview"</span>
                                <span id="vad-energy-readout" class="telemetry-mini">"RMS: 0.00"</span>
                            </div>
                        </div>
                    </div>
                    <div class="roster-bar">
                        <span class="roster-title">"Session Roster:"</span>
                        <div id="roster-list" class="roster-chips"></div>
                    </div>
                </div>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::{
        CombatStance, MessageVisibility, SessionMode, SessionRole, TopicMetricsView,
    };

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
            html[session_card_idx.saturating_sub(40)..session_card_idx + 80]
                .contains("display: none")
        );
        let workspace_idx = html.find(r#"id="workspace-panel""#).unwrap();
        assert!(
            html[workspace_idx.saturating_sub(40)..workspace_idx + 80].contains("display: none")
        );
        assert!(html.contains("<h2>Session Chat</h2>"));
        assert!(!html.contains("Public &amp; Private Targeted Session Chat"));
        assert!(html.contains("Roll Satya Task"));
        assert!(html.contains(r#"id="ttrpg-skill-level""#));
        assert!(html.contains(r#"id="ttrpg-control-c""#));
        assert!(html.contains(r#"id="ttrpg-internal-mod""#));
        assert!(html.contains(r#"id="ttrpg-external-mod""#));
        assert!(html.contains(r#"id="ttrpg-carry-mode""#));
        assert!(html.contains(r#"id="ttrpg-botch-rule""#));
        assert!(html.contains(r#"id="ttrpg-wound-shock""#));
        assert!(html.contains(r#"id="ttrpg-odds-pill""#));
        assert!(html.contains(r#"id="ttrpg-odds-readout""#));
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

        // Verify all 6 Tabbed Notebook tabs & panes exist
        for (_, btn_id, pane_id) in wasm_client::NOTEBOOK_TABS {
            assert!(html.contains(&format!(r#"id="{btn_id}""#)));
            assert!(html.contains(&format!(r#"id="{pane_id}""#)));
        }

        // Verify Web Cams vertical column controls (1 or at most 2 cols), collapsible drawer & floating frame toggle
        assert!(html.contains(r#"id="btn-toggle-webcam-cols""#));
        assert!(html.contains(r#"id="btn-toggle-webcam-float""#));
        assert!(html.contains(r#"id="btn-toggle-webcam-drawer""#));
        assert!(html.contains(r#"id="btn-expand-webcam-drawer""#));
        assert!(html.contains(r#"id="webcam-drawer-rail""#));
        assert!(html.contains(r#"id="btn-menu-toggle-webcam-pane""#));
        assert!(html.contains(r#"id="webrtc-drag-header""#));
        assert!(html.contains(r#"class="video-grid cols-1""#));

        // Verify Google OAuth (`drive.file`) & GM_ONLY Document Vault controls
        assert!(html.contains(r#"id="btn-google-auth""#));
        assert!(html.contains(r#"id="google-auth-status-pill""#));
        assert!(html.contains(r#"id="tab-btn-docs""#));
        assert!(html.contains(r#"id="notebook-pane-docs""#));
        assert!(html.contains(r#"id="gm-doc-file-input""#));
        assert!(html.contains(r#"id="gm-doc-title""#));
        assert!(html.contains(r#"id="gm-doc-source""#));
        assert!(html.contains(r#"id="btn-google-picker""#));
        assert!(html.contains(r#"id="gm-google-oauth-config-row""#));
        assert!(html.contains(r#"id="gm-google-client-id""#));
        assert!(html.contains(r#"id="gm-google-api-key""#));
        assert!(html.contains(r#"id="gm-doc-content""#));
        assert!(html.contains(r#"id="btn-submit-gm-doc""#));
        assert!(html.contains(r#"id="gm-docs-list""#));

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
        assert!(html.contains(r#"id="rules-search-input""#));
        assert!(html.contains(r#"id="rules-book-viewport""#));
        assert!(html.contains("Task Resolution &amp; Skill Check Mechanics"));

        assert!(html.contains("btn-logout"));
        assert!(html.contains("btn-leave-session"));
        assert!(html.contains("btn-workspace-disconnect"));
        assert!(html.contains(r#"id="admin-inspector-card""#));
        assert!(html.contains("display: none"));
    }
}
