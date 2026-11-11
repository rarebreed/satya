//! Pre-session user authentication (`#auth-card`) and password-protected session
//! creation, join, and SQLite vault resume gate (`#session-card`).
//!
//! Re-exports [`HeaderBar`], [`PasswordToggleButton`], and [`AI_GM_PROVIDER_OPTIONS`]
//! from [`crate::views_header`].

pub use crate::views_header::*;

use crate::wasm_client;
use leptos::prelude::*;

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
                    <h2>"2. Create, Join, or Resume Password-Protected Session"</h2>
                    <span class="subtitle">"TTRPG (GM / AI-GM / Players) or Meeting (Host / Participants) · Auto-saves Chat, Rolls, Map & Combat State"</span>
                </div>
                <div class="form-grid" style="margin-bottom: 0.65rem;">
                    <label style="grid-column: 1 / -1;">
                        <span>"Saved & Resumable Sessions (SQLite Vault)"</span>
                        <div class="btn-row" style="margin-top: 0.25rem; flex-wrap: wrap;">
                            <select
                                id="saved-sessions-select"
                                style="flex: 1 1 260px;"
                                onchange="window.satyaSelectSavedSession()"
                                on:change=move |_| wasm_client::satya_select_saved_session()
                            >
                                <option value="">"No saved sessions yet — Create or Join a session below"</option>
                            </select>
                            <button
                                type="button"
                                id="btn-resume-saved-session"
                                class="btn btn-primary"
                                title="Resume the selected saved session and replay visibility-filtered chat & mechanics history"
                                onclick="window.satyaResumeSavedSession()"
                                on:click=move |_| wasm_client::satya_resume_saved_session()
                            >
                                "Resume Saved Session"
                            </button>
                            <button
                                type="button"
                                id="btn-refresh-saved-sessions"
                                class="btn btn-secondary"
                                title="Refresh saved sessions list from server"
                                onclick="window.satyaRefreshSavedSessions()"
                                on:click=move |_| wasm_client::satya_refresh_saved_sessions()
                            >
                                "Refresh List"
                            </button>
                            <button
                                type="button"
                                id="btn-export-session-snapshot"
                                class="btn btn-secondary"
                                title="Export a portable .satyasession snapshot file (without API keys)"
                                onclick="window.satyaExportSessionSnapshot()"
                                on:click=move |_| wasm_client::satya_export_session_snapshot()
                            >
                                "Export .satyasession"
                            </button>
                            <input
                                id="session-import-file-input"
                                type="file"
                                accept=".satyasession,.json"
                                style="max-width: 210px;"
                            />
                            <button
                                type="button"
                                id="btn-import-session-snapshot"
                                class="btn btn-secondary"
                                title="Import a portable .satyasession file and enter the restored session"
                                onclick="window.satyaImportSessionSnapshot()"
                                on:click=move |_| wasm_client::satya_import_session_snapshot()
                            >
                                "Import .satyasession"
                            </button>
                        </div>
                    </label>
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
                            {AI_GM_PROVIDER_OPTIONS
                                .iter()
                                .copied()
                                .map(|(val, label)| {
                                    view! { <option value=val>{label}</option> }
                                })
                                .collect_view()}
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
