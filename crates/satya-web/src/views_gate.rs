//! Pre-session user authentication (`#auth-card`) and password-protected session
//! creation, join, and SQLite vault resume gate (`#session-card`).
//!
//! Re-exports [`HeaderBar`], [`PasswordToggleButton`], and [`AI_GM_PROVIDER_OPTIONS`]
//! from [`crate::views_header`].

pub use crate::views_header::*;

use crate::wasm_client;
use leptos::prelude::*;

/// Static table of quick-fill demo accounts for testing TTRPG and Meeting sessions (opt-in `demo-accounts` feature only).
#[cfg(feature = "demo-accounts")]
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
    #[cfg(feature = "demo-accounts")]
    let demo_chips_view = view! {
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
    }
    .into_any();
    #[cfg(not(feature = "demo-accounts"))]
    let demo_chips_view = ().into_any();

    view! {
        <section id="gate-panel" class="gate-container">
            <div id="auth-card" class="card auth-card">
                <div class="card-header">
                    <h2>"1. User Authentication"</h2>
                    <span class="subtitle">"Persistent user_id across sessions for ML & analytics"</span>
                </div>
                {demo_chips_view}
                <div class="form-grid">
                    <label>
                        <span>"Username"</span>
                        <input id="auth-username" type="text" placeholder="e.g. gm_sean"/>
                    </label>
                    <label>
                        <span>"Password"</span>
                        <div class="password-field">
                            <input id="auth-password" type="password" placeholder="Password"/>
                            <PasswordToggleButton
                                input_id="auth-password"
                                btn_id="btn-toggle-auth-password"
                            />
                        </div>
                    </label>
                    <label>
                        <span>"Display / Character Name"</span>
                        <input id="auth-display-name" type="text" placeholder="e.g. Sean (GM)"/>
                    </label>
                    <label>
                        <span>"Email (Optional, Private — for Recovery, GM Search & Alerts)"</span>
                        <input id="auth-email" type="email" placeholder="e.g. gm@example.com"/>
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
                        id="btn-forgot-credentials"
                        class="btn btn-secondary"
                        onclick="window.satyaToggleRecoveryPanel()"
                        on:click=move |_| wasm_client::satya_toggle_recovery_panel()
                    >
                        "Forgot Username / Password?"
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
                <div id="auth-recovery-panel" class="auth-recovery-panel" style="display: none;">
                    <div class="auth-recovery-header">
                        <strong>"Account Recovery (Forgotten Username or Password)"</strong>
                        <button
                            type="button"
                            id="btn-close-recovery-panel"
                            class="btn btn-sm btn-secondary"
                            aria-label="Close Account Recovery"
                            onclick="window.satyaCloseRecoveryPanel()"
                            on:click=move |_| wasm_client::satya_close_recovery_panel()
                        >
                            "×"
                        </button>
                    </div>
                    <p class="auth-recovery-hint">
                        "Enter your registered email address or username. If an email is on file, we will send your username reminder and a single-use password reset token."
                    </p>
                    <div class="auth-recovery-section">
                        <div class="form-grid">
                            <label>
                                <span>"Email or Username"</span>
                                <input
                                    id="recovery-identifier"
                                    type="text"
                                    placeholder="Registered email or username"
                                />
                            </label>
                        </div>
                        <div class="btn-row">
                            <button
                                type="button"
                                id="btn-request-recovery"
                                class="btn btn-primary btn-sm"
                                onclick="window.satyaRequestAccountRecovery()"
                                on:click=move |_| wasm_client::satya_request_account_recovery()
                            >
                                "Send Recovery Email"
                            </button>
                        </div>
                    </div>
                    <div class="auth-recovery-section">
                        <div class="form-grid">
                            <label>
                                <span>"Reset Token"</span>
                                <input
                                    id="recovery-token"
                                    type="text"
                                    placeholder="Paste reset token (satya_rst_...)"
                                />
                            </label>
                            <label>
                                <span>"New Password"</span>
                                <div class="password-field">
                                    <input
                                        id="recovery-new-password"
                                        type="password"
                                        placeholder="New password (8+ chars)"
                                    />
                                    <PasswordToggleButton
                                        input_id="recovery-new-password"
                                        btn_id="btn-toggle-recovery-new-password"
                                    />
                                </div>
                            </label>
                            <label>
                                <span>"Confirm New Password"</span>
                                <div class="password-field">
                                    <input
                                        id="recovery-confirm-password"
                                        type="password"
                                        placeholder="Confirm new password"
                                    />
                                    <PasswordToggleButton
                                        input_id="recovery-confirm-password"
                                        btn_id="btn-toggle-recovery-confirm-password"
                                    />
                                </div>
                            </label>
                        </div>
                        <div class="btn-row">
                            <button
                                type="button"
                                id="btn-confirm-password-reset"
                                class="btn btn-accent btn-sm"
                                onclick="window.satyaConfirmPasswordReset()"
                                on:click=move |_| wasm_client::satya_confirm_password_reset()
                            >
                                "Reset Password"
                            </button>
                        </div>
                    </div>
                    <div id="auth-recovery-feedback" class="feedback-box"></div>
                </div>
            </div>
        </section>

        <div id="session-modal-backdrop" class="session-modal-backdrop" style="display: none;" onclick="window.satyaCloseSessionPane()" on:click=move |_| wasm_client::satya_close_session_pane()></div>
            <div id="session-card" class="card session-card session-modal" style="display: none;">
                <div class="lobby-modal-header">
                    <div class="lobby-modal-nav">
                        <button
                            type="button"
                            id="btn-lobby-back"
                            class="btn btn-sm btn-secondary"
                            style="display: none;"
                            onclick="window.satyaLobbyGoBack()"
                            on:click=move |_| wasm_client::satya_lobby_go_back()
                        >
                            "← Back"
                        </button>
                        <div id="lobby-step-breadcrumb" class="lobby-breadcrumb"></div>
                    </div>
                    <div class="lobby-header-actions">
                        <button
                            type="button"
                            id="btn-toggle-requests-tray"
                            class="btn btn-sm btn-secondary"
                            style="display: none;"
                            onclick="window.satyaToggleRequestsTray()"
                            on:click=move |_| wasm_client::satya_toggle_requests_tray()
                        >
                            <span>"Requests "</span>
                            <span id="requests-tray-badge" class="pill pill-muted">"0"</span>
                        </button>
                        <button
                            type="button"
                            id="btn-leave-session"
                            class="btn btn-sm btn-danger"
                            style="display: none;"
                            onclick="window.satyaLeaveSession()"
                            on:click=move |_| wasm_client::satya_leave_session()
                            disabled
                        >
                            "Disconnect"
                        </button>
                        <button
                            type="button"
                            id="btn-close-session-pane"
                            class="btn btn-sm btn-secondary"
                            title="Close Campaigns & Sessions"
                            aria-label="Close Campaigns & Sessions"
                            onclick="window.satyaCloseSessionPane()"
                            on:click=move |_| wasm_client::satya_close_session_pane()
                        >
                            "×"
                        </button>
                    </div>
                </div>

                <div id="session-requests-tray" class="lobby-requests-tray" style="display: none;">
                    <div class="lobby-requests-head">
                        <span class="menu-section-title">"Join & Session Requests"</span>
                        <button
                            type="button"
                            id="btn-refresh-join-requests"
                            class="btn btn-sm btn-secondary"
                            onclick="window.satyaRefreshJoinRequests()"
                            on:click=move |_| wasm_client::satya_refresh_join_requests()
                        >
                            "Refresh"
                        </button>
                    </div>
                    <div id="gm-join-requests-list" class="join-requests-list"></div>
                    <div id="my-join-requests-list" class="join-requests-list"></div>
                </div>

                <div id="lobby-step-1" class="lobby-step-view">
                    <div class="lobby-step-intro">
                        <h2>"Choose a Campaign"</h2>
                        <div class="lobby-scope-toggle" role="tablist" aria-label="Campaign Scope">
                            <button
                                type="button"
                                id="btn-scope-my-campaigns"
                                class="lobby-scope-btn active"
                                onclick="window.satyaSetLobbyScope('mine')"
                                on:click=move |_| wasm_client::satya_set_lobby_scope("mine".to_string())
                            >
                                "My Campaigns"
                            </button>
                            <button
                                type="button"
                                id="btn-scope-search-campaigns"
                                class="lobby-scope-btn"
                                onclick="window.satyaSetLobbyScope('search')"
                                on:click=move |_| wasm_client::satya_set_lobby_scope("search".to_string())
                            >
                                "Find Another User"
                            </button>
                        </div>
                    </div>

                    <div id="campaign-search-bar" class="lobby-search-bar" style="display: none;">
                        <div class="btn-row" style="align-items: center;">
                            <input
                                id="directory-gm-query"
                                type="text"
                                style="flex: 1 1 200px;"
                                placeholder="Search by email or @handle (e.g. @gm_sean)"
                                onkeydown="window.satyaDirectorySearchKeydown(event)"
                                on:keydown=wasm_client::satya_directory_search_keydown
                            />
                            <button
                                type="button"
                                id="btn-search-gm-directory"
                                class="btn btn-sm btn-primary"
                                onclick="window.satyaSearchGmDirectory()"
                                on:click=move |_| wasm_client::satya_search_gm_directory()
                            >
                                "Search"
                            </button>
                        </div>
                        <div id="directory-user-chips" class="lobby-user-chips"></div>
                    </div>

                    <div id="campaign-list-pane" class="lobby-step-list"></div>
                </div>

                <div id="lobby-step-2" class="lobby-step-view" style="display: none;">
                    <div id="lobby-step2-context" class="lobby-step-context"></div>
                    <div id="session-list-pane" class="lobby-step-list"></div>
                    <div id="lobby-new-campaign-fields" class="lobby-stack-form" style="display: none;">
                        <label>
                            <span>"Campaign Title"</span>
                            <input id="campaign-title-input" type="text" placeholder="e.g. Satyang or DAISE"/>
                        </label>
                        <label>
                            <span>"Description (Optional)"</span>
                            <input id="campaign-desc-input" type="text" placeholder="Short pitch for players"/>
                        </label>
                        <label class="recip-option" for="campaign-discoverable-checkbox">
                            <input id="campaign-discoverable-checkbox" type="checkbox" checked/>
                            <span>"Discoverable when other users search for you"</span>
                        </label>
                        <div class="btn-row" style="margin-top: 4px;">
                            <button
                                type="button"
                                id="btn-create-campaign-and-session"
                                class="btn btn-accent"
                                onclick="window.satyaCreateCampaignStep(true)"
                                on:click=move |_| wasm_client::satya_create_campaign_step(true)
                            >
                                "Create Campaign & Add Session →"
                            </button>
                            <button
                                type="button"
                                id="btn-create-empty-campaign"
                                class="btn btn-secondary"
                                onclick="window.satyaCreateCampaignStep(false)"
                                on:click=move |_| wasm_client::satya_create_campaign_step(false)
                            >
                                "Create Empty Campaign"
                            </button>
                        </div>
                    </div>
                </div>

                <div id="lobby-step-3" class="lobby-step-view" style="display: none;">
                    <div id="lobby-step3-context" class="lobby-step-context"></div>
                    <div id="lobby-dynamic-action-body"></div>
                    <div id="lobby-new-session-panel" class="lobby-stack-form" style="display: none;">
                        <div class="lobby-form-row">
                            <label>
                                <span>"Session Title"</span>
                                <input id="session-title-input" type="text" placeholder="e.g. Ep. 1: Orbital Breach"/>
                            </label>
                            <label>
                                <span>"Session Password"</span>
                                <div class="password-field">
                                    <input id="session-password-input" type="password" placeholder="Session password"/>
                                    <PasswordToggleButton
                                        input_id="session-password-input"
                                        btn_id="btn-toggle-session-password"
                                    />
                                </div>
                            </label>
                            <label>
                                <span>"Mode"</span>
                                <select
                                    id="session-mode-select"
                                    onchange="window.satyaModeChanged()"
                                    on:change=move |_| wasm_client::satya_mode_changed()
                                >
                                    <option value="TTRPG">"TTRPG Mode (GM + Players)"</option>
                                    <option value="TTRPG_GM_AI">"TTRPG Mode (GM + AI + Players)"</option>
                                    <option value="TTRPG_AI_GM">"TTRPG Mode (AI GM + Players)"</option>
                                    <option value="TTRPG_SOLO_AI_GM">"TTRPG Mode (Solo Player / Co-GM + AI GM)"</option>
                                    <option value="MEETING">"Meeting Mode (Host + 2+ Attendees)"</option>
                                </select>
                            </label>
                            <label>
                                <span>"Your Role"</span>
                                <select id="session-role-select">
                                    <option value="GM">"Game Master (GM)"</option>
                                    <option value="SOLO_GM">"Solo Player (Co-GM)"</option>
                                    <option value="PLAYER">"Player"</option>
                                    <option value="OBSERVER">"Observer"</option>
                                </select>
                            </label>
                            <input id="enable-ai-gm" type="checkbox" style="display: none"/>
                        </div>

                        <details id="lobby-advanced-details" class="lobby-disclosure">
                            <summary>"Advanced (Session ID, Visibility & AI GM)"</summary>
                            <div class="lobby-form-row" style="margin-top: 6px;">
                                <label>
                                    <span>"Session ID (Optional)"</span>
                                    <input id="session-id-input" type="text" placeholder="Auto-generated from title if blank" value=""/>
                                </label>
                                <div class="lobby-inline-checks">
                                    <label class="recip-option" for="session-discoverable-checkbox">
                                        <input id="session-discoverable-checkbox" type="checkbox" checked/>
                                        <span>"Discoverable"</span>
                                    </label>
                                    <label class="recip-option" for="session-allow-requests-checkbox">
                                        <input id="session-allow-requests-checkbox" type="checkbox" checked/>
                                        <span>"Allow Requests"</span>
                                    </label>
                                </div>
                            </div>
                            <div id="ai-gm-config-fields" class="form-grid ai-gm-config-grid" style="display: none; margin-top: 6px;">
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
                        </details>

                        <div class="btn-row" style="margin-top: 4px;">
                            <button
                                type="button"
                                id="btn-lobby-create-submit"
                                class="btn btn-accent"
                                onclick="window.satyaSubmitLobbyCreate()"
                                on:click=move |_| wasm_client::satya_submit_lobby_create()
                            >
                                "Create & Enter Session"
                            </button>
                        </div>
                    </div>
                </div>

                <div class="lobby-modal-footer">
                    <details id="session-backup-details" class="lobby-disclosure">
                        <summary>"Import / Export & Quota"</summary>
                        <div class="btn-row" style="margin-top: 6px; align-items: center;">
                            <span id="account-quota-pill" class="pill pill-muted">"Quota: —"</span>
                            <button
                                type="button"
                                id="btn-refresh-saved-sessions"
                                class="btn btn-sm btn-secondary"
                                title="Refresh campaigns and saved sessions from server"
                                onclick="window.satyaRefreshSavedSessions()"
                                on:click=move |_| wasm_client::satya_refresh_saved_sessions()
                            >
                                "Refresh"
                            </button>
                            <button
                                type="button"
                                id="btn-export-session-snapshot"
                                class="btn btn-sm btn-secondary"
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
                                class="btn btn-sm btn-secondary"
                                title="Import a portable .satyasession file and enter the restored session"
                                onclick="window.satyaImportSessionSnapshot()"
                                on:click=move |_| wasm_client::satya_import_session_snapshot()
                            >
                                "Import .satyasession"
                            </button>
                        </div>
                    </details>
                    <div id="session-feedback" class="feedback-box"></div>
                </div>
            </div>
    }
}
