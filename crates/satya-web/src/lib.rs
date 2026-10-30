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
            <div class="col-left">
                <div class="card webrtc-card">
                    <div class="card-header-inline">
                        <div>
                            <h2>"WebRTC Audio / Video & Prosody Mesh"</h2>
                            <span class="subtitle">"P2P Media + DataChannel + VAD InteractionDynamicsEvent"</span>
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
                    </div>
                    <div id="video-grid" class="video-grid">
                        <div class="video-tile local-tile">
                            <video id="local-video" autoplay muted playsinline></video>
                            <div class="tile-overlay">
                                <span id="local-participant-label" class="role-tag">"Local Preview"</span>
                                <span id="vad-energy-readout" class="telemetry-mini">"RMS: 0.00"</span>
                            </div>
                        </div>
                    </div>
                    <div class="roster-bar">
                        <span class="roster-title">"Session Roster (Nested Avro Participant Records):"</span>
                        <div id="roster-list" class="roster-chips"></div>
                    </div>
                </div>
            </div>

            <div class="col-right">
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
                            <span class="subtitle">"Logs TtrpgMechanicsEvent to Apache Iggy (`ttrpg-mechanics`)"</span>
                        </div>
                    </div>
                    <div class="ttrpg-controls">
                        <label>
                            <span>"Skill Name"</span>
                            <input id="ttrpg-skill" type="text" value="Trauma Surgery"/>
                        </label>
                        <label>
                            <span>"Attribute TN (1-20)"</span>
                            <input id="ttrpg-tn" type="number" value="13" min="1" max="20"/>
                        </label>
                        <label>
                            <span>"Dice Pool (d20s)"</span>
                            <input id="ttrpg-pool" type="number" value="3" min="1" max="10"/>
                        </label>
                        <label>
                            <span>"Difficulty (Req. Successes)"</span>
                            <input id="ttrpg-diff" type="number" value="2" min="1" max="8"/>
                        </label>
                        <label>
                            <span>"Combat Stance"</span>
                            <select id="ttrpg-stance">
                                <option value="">"Non-Combat Task"</option>
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

                <div id="admin-inspector-card" class="card inspector-card" style="display: none;">
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
            wasm_client::parse_session_mode("TTRPG_AI_GM"),
            SessionMode::Ttrpg
        );
        assert!(!wasm_client::is_ai_gm_mode("TTRPG_GM"));
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
        assert!(html.contains("TTRPG Mode (AI GM + Players)"));
        assert!(html.contains("btn-toggle-auth-password"));
        assert!(html.contains("btn-toggle-session-password"));
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
        assert!(html.contains("btn-logout"));
        assert!(html.contains("btn-leave-session"));
        assert!(html.contains("btn-workspace-disconnect"));
        assert!(html.contains(r#"id="admin-inspector-card""#));
        assert!(html.contains("display: none"));
    }
}
