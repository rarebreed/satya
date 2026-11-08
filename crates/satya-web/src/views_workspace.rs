//! Active 3-column session workspace (`ActiveSessionWorkspace`):
//! - Left column: [`NotebookColumn`](crate::views_notebook::NotebookColumn)
//! - Center column (`#center-column`): Targeted Session Chat (`#chat-card`) + Satya 2d20 Task Resolution (`#ttrpg-drawer`)
//! - Right column (`#webrtc-dock-column`): WebRTC Audio/Video + Prosody Mesh (`#webrtc-card`) & collapsible drawer rails (`#right-drawer-rails`)

use crate::views_notebook::NotebookColumn;
use crate::wasm_client;
use leptos::prelude::*;

/// Active 3-column session workspace (`#workspace-panel`).
#[component]
pub fn ActiveSessionWorkspace() -> impl IntoView {
    view! {
        <section id="workspace-panel" class="workspace-columns" style="display: none;">
            <NotebookColumn/>

            // ================================================================
            // CENTER COLUMN: Chat Session Interface (Top) + Die Roller (Bottom)
            // ================================================================
            <div id="center-column" class="col-center">
                <div id="chat-card" class="card chat-card">
                    <div class="card-header-inline">
                        <div>
                            <h2>"Session Chat"</h2>
                            <span class="subtitle">"Broadcast publicly or whisper privately to selected Participant(s)"</span>
                        </div>
                        <div class="webcam-frame-actions">
                            <button
                                type="button"
                                id="btn-toggle-chat-drawer"
                                class="btn btn-sm btn-secondary"
                                title="Collapse Session Chat drawer to save workspace space"
                                aria-label="Collapse Session Chat Drawer"
                                aria-expanded="true"
                                aria-controls="chat-card"
                                onclick="window.satyaToggleChatDrawer()"
                                on:click=move |_| wasm_client::satya_toggle_chat_drawer()
                            >
                                "Collapse ▸"
                            </button>
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
                            <h2>"Task Resolution & Action Pool"</h2>
                            <span class="subtitle">"Two-List Cascading (`2d20`) Roll-Over Engine · Logs TtrpgMechanicsEvent to Apache Iggy (`ttrpg-mechanics`)"</span>
                        </div>
                        <div class="webcam-frame-actions">
                            <span id="ttrpg-odds-pill" class="pill pill-active">
                                {wasm_client::format_cascade_odds_preview(
                                    &satya_engine::CascadeTaskParams::new_k20(3, 12, 0, 0),
                                )}
                            </span>
                            <button
                                type="button"
                                id="btn-toggle-ttrpg-drawer"
                                class="btn btn-sm btn-secondary"
                                title="Collapse Task Resolution & Action Pool drawer to save workspace space"
                                aria-label="Collapse Task Resolution & Action Pool Drawer"
                                aria-expanded="true"
                                aria-controls="ttrpg-drawer"
                                onclick="window.satyaToggleTtrpgDrawer()"
                                on:click=move |_| wasm_client::satya_toggle_ttrpg_drawer()
                            >
                                "Collapse ▸"
                            </button>
                        </div>
                    </div>
                    <div class="ttrpg-controls">
                        <div class="ttrpg-skill-combobox">
                            <label for="ttrpg-skill">
                                <span>"Skill Name"</span>
                            </label>
                            <div class="ttrpg-skill-input-wrap">
                                <input
                                    id="ttrpg-skill"
                                    type="text"
                                    value="Trauma Surgery"
                                    placeholder="Search or select a skill..."
                                    autocomplete="off"
                                    role="combobox"
                                    aria-autocomplete="list"
                                    aria-expanded="false"
                                    aria-controls="ttrpg-skill-dropdown"
                                    oninput="window.satyaFilterTtrpgSkillDropdown()"
                                    on:input=move |_| wasm_client::satya_filter_ttrpg_skill_dropdown()
                                    onfocus="window.satyaOpenTtrpgSkillDropdown()"
                                    on:focus=move |_| wasm_client::satya_open_ttrpg_skill_dropdown()
                                    onkeydown="window.satyaTtrpgSkillKeydown(event)"
                                    on:keydown=wasm_client::satya_ttrpg_skill_keydown
                                />
                                <button
                                    type="button"
                                    id="btn-toggle-ttrpg-skill-dropdown"
                                    class="btn-skill-dropdown-toggle"
                                    title="Browse & search canonical skills"
                                    aria-label="Toggle Skill Dropdown"
                                    onclick="window.satyaToggleTtrpgSkillDropdown()"
                                    on:click=move |_| wasm_client::satya_toggle_ttrpg_skill_dropdown()
                                >
                                    "▾"
                                </button>
                            </div>
                            <div
                                id="ttrpg-skill-dropdown"
                                class="ttrpg-skill-dropdown"
                                role="listbox"
                                aria-label="Canonical Satya Skills"
                                style="display: none;"
                                inner_html=wasm_client::default_ttrpg_skill_dropdown_html()
                            ></div>
                        </div>
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
                            <span>"Tool Mod Δ_tool (-4 to +12)"</span>
                            <input
                                id="ttrpg-tool-mod"
                                type="number"
                                value="0"
                                min="-4"
                                max="12"
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

                <div
                    id="right-drawer-rails"
                    class="right-drawer-rails"
                    style="display: none;"
                >
                    <div
                        id="chat-drawer-rail"
                        class="webcam-drawer-rail"
                        style="display: none;"
                    >
                        <button
                            type="button"
                            id="btn-expand-chat-drawer"
                            class="btn-webcam-drawer-tab"
                            title="Slide out Session Chat Drawer"
                            aria-label="Expand Session Chat Drawer"
                            aria-expanded="false"
                            aria-controls="chat-card"
                            onclick="window.satyaToggleChatDrawer()"
                            on:click=move |_| wasm_client::satya_toggle_chat_drawer()
                        >
                            <span class="drawer-tab-chevron" aria-hidden="true">"◂"</span>
                            <span class="drawer-tab-label">"Session Chat"</span>
                        </button>
                    </div>
                    <div
                        id="ttrpg-drawer-rail"
                        class="webcam-drawer-rail"
                        style="display: none;"
                    >
                        <button
                            type="button"
                            id="btn-expand-ttrpg-drawer"
                            class="btn-webcam-drawer-tab"
                            title="Slide out Task Resolution & Action Pool Drawer"
                            aria-label="Expand Task Resolution & Action Pool Drawer"
                            aria-expanded="false"
                            aria-controls="ttrpg-drawer"
                            onclick="window.satyaToggleTtrpgDrawer()"
                            on:click=move |_| wasm_client::satya_toggle_ttrpg_drawer()
                        >
                            <span class="drawer-tab-chevron" aria-hidden="true">"◂"</span>
                            <span class="drawer-tab-label">"Task Roller"</span>
                        </button>
                    </div>
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
                </div>
            </div>
        </section>
    }
}
