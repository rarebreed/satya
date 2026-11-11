//! Companion Leptos component (`NotebookGmAndNpcPanes`) for the Tabletop Notebook's
//! Role-Filtered NPC Roster & Quick NPC Roll-Up Generator (`#notebook-pane-npcs`) and
//! `GM_ONLY` Document Vault, Scenario Director Dials, GM Archivist Omnisearch, and
//! Persistent GM Memory Notes (`#notebook-pane-docs`).

use crate::wasm_client;
use leptos::prelude::*;

/// Canonical NPC roll-up archetype `<option>` pairs (`(code, label)`).
const NPC_ARCHETYPE_OPTIONS: &[(&str, &str)] = &[
    ("AUTO", "Auto-Infer from Description"),
    ("MEDICAL_DOCTOR", "General MD / Physician / Surgeon"),
    ("COMBAT_SOLDIER", "Combat Soldier / Tactical Operator"),
    (
        "DETECTIVE_INVESTIGATOR",
        "Detective / Law Enforcement Investigator",
    ),
    ("SYNDICATE_ENFORCER", "Syndicate Enforcer / Underworld Fixer"),
    ("STEM_ENGINEER", "STEM Scientist / Cyber & Robotics Engineer"),
    (
        "INTELLIGENCE_OPERATIVE",
        "Clandestine Intelligence Operative / Handler",
    ),
    ("BLUE_COLLAR_TECH", "Blue-Collar Technician / Mechanic / Pilot"),
    (
        "FRONTIER_RANGER",
        "Frontier Ranger / Wilderness Scout & Herbalist",
    ),
];

/// Quick Scenario Director dial preset chips (`(preset_id, label)`).
const SCENARIO_PRESET_SPECS: &[(&str, &str)] = &[
    ("balanced", "Balanced Mix"),
    ("combat_heavy", "Combat & Tactical Heavy"),
    ("intrigue_drama", "Intrigue & High Drama"),
    ("investigation_heist", "Investigation & Heist"),
];

/// GM Archivist omnisearch scope `<option>` pairs (`(code, label)`).
const GM_OMNISEARCH_SCOPE_OPTIONS: &[(&str, &str)] = &[
    (
        "ALL",
        "All Sources (Rules + Docs + Memories + Sheets + Combat + Map)",
    ),
    ("RULES", "Rules Book & Mechanics Only"),
    ("GM_DOCS", "GM_ONLY Documents (Session & Persistent)"),
    ("GM_MEMORY", "GM Memory Notes & Pinned AI Facts"),
    ("CHARACTERS", "Bound Session Character Sheets"),
    ("COMBAT", "Combat Tracker & Tick Timeline"),
    ("MAP", "Tactical Map & Telemetry"),
];

/// Persistent GM Memory category `<option>` pairs (`(code, label)`).
const GM_MEMORY_CATEGORY_OPTIONS: &[(&str, &str)] = &[
    ("SCENE_STATE", "Scene State (`SCENE_STATE`)"),
    ("NPC_SECRET", "NPC Secret / Agenda (`NPC_SECRET`)"),
    ("QUEST_HOOK", "Quest Hook / Objective (`QUEST_HOOK`)"),
    ("RULING", "Table Ruling (`RULING`)"),
    ("ROLLING_SUMMARY", "Rolling Session Summary (`ROLLING_SUMMARY`)"),
];

/// Renders the NPC Roster/Generator (`#notebook-pane-npcs`) and `GM_ONLY` Document Vault,
/// Scenario Director, Omnisearch, and Persistent GM Memories (`#notebook-pane-docs`) panes.
#[component]
pub fn NotebookGmAndNpcPanes() -> impl IntoView {
    view! {
        // --- TAB: NPC LIST & QUICK NPC GENERATOR (ROLE-FILTERED PLAYER vs GM/SOLO VIEW) ---
        <div
            id="notebook-pane-npcs"
            role="tabpanel"
            aria-labelledby="tab-btn-npcs"
            data-notebook-pane="npcs"
            class="notebook-pane"
            style="display: none;"
        >
            <div class="pane-toolbar">
                <div>
                    <span class="subtitle">
                        "Session NPC Roster & Quick NPC Roll-Up (Public Summary for Players · Full Psyche, Skills & Secrets for GM / Solo Mode)"
                    </span>
                </div>
                <div class="btn-row-compact">
                    <span id="npc-roster-count-pill" class="pill pill-iggy">"1 NPCs"</span>
                    <span id="npc-list-visibility-badge" class="pill pill-accent">
                        "GM / Solo View (Full Stats, Psyche & Secrets Visible)"
                    </span>
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaRefreshNpcsTab()"
                        on:click=move |_| wasm_client::satya_refresh_npcs_tab()
                    >
                        "Refresh NPCs"
                    </button>
                </div>
            </div>

            // --- 1. NPC SELECTOR & SHORTENED DISPLAY CARD ---
            <div class="roster-bar">
                <span class="roster-title">"1. Select Session NPC to Inspect:"</span>
                <div class="form-grid">
                    <label>
                        <span>"Active Session & Campaign NPCs"</span>
                        <select
                            id="npc-roster-select"
                            onchange="window.satyaSelectNpcFromList()"
                            on:change=move |_| wasm_client::satya_select_npc_from_list()
                        >
                            <option value="npc_demo_terry">
                                "Terry — General MD & Clinical Physician (Age 26)"
                            </option>
                        </select>
                    </label>
                </div>
                <div id="npc-selected-card-container" class="gm-docs-list">
                    <div class="gm-doc-card">
                        <div class="gm-doc-header">
                            <div>
                                <strong>"Terry"</strong>
                                <span class="muted">" · General MD & Clinical Physician (Age 26)"</span>
                            </div>
                            <div class="btn-row-compact">
                                <span class="pill pill-active">"Healthy & Alert"</span>
                                <span class="pill pill-accent">"Persistent Campaign NPC"</span>
                            </div>
                        </div>
                        <div class="gm-doc-meta">
                            <strong>"Public Appearance & Demeanor (Visible to All Players):"</strong>
                            <br />
                            "Terry — General MD & Clinical Physician (Age 26). Apparent demeanor: Terry is a kind general MD, but needs to feel like she belongs."
                        </div>
                        <div class="gm-doc-meta">
                            <strong>"GM / Solo Only — 7 Bipolar Psyche Highlights:"</strong>
                            " SLF 5 (Altruistic / Self-Sacrificing) · AGG 5 (Passive / Conflict-Averse / Gentle) · EGO 7 (Humble / Self-Effacing) · BLG 16 (Tribal / Group-Dependent) · OPN 14 (Curious / Open-Minded)"
                        </div>
                    </div>
                </div>
            </div>

            // --- 2. GM / SOLO QUICK NPC ROLL-UP GENERATOR ---
            <div id="npc-generator-panel" class="roster-bar">
                <span class="roster-title">
                    "2. Quick NPC Roll-Up Generator (GM & Solo Mode Only — Hidden from Non-GM Players):"
                </span>
                <div class="form-grid">
                    <label>
                        <span>"Natural-Language NPC Description / Personality & Role Prompt"</span>
                        <input
                            id="npc-gen-prompt"
                            type="text"
                            value="Terry is a kind general MD, but needs to feel like she belongs"
                            placeholder="e.g. Terry is a kind general MD, but needs to feel like she belongs"
                        />
                    </label>
                    <label>
                        <span>"Archetype Template (or Auto-Infer from Prompt)"</span>
                        <select id="npc-gen-template">
                            {NPC_ARCHETYPE_OPTIONS
                                .iter()
                                .copied()
                                .map(|(code, label)| view! { <option value=code>{label}</option> })
                                .collect_view()}
                        </select>
                    </label>
                    <label>
                        <span>"Optional Name Override"</span>
                        <input
                            id="npc-gen-name"
                            type="text"
                            placeholder="Leave blank to extract from prompt"
                        />
                    </label>
                    <label>
                        <span>"Adult 4-Year Career Terms"</span>
                        <select id="npc-gen-terms">
                            <option value="1">"1 Term (Age 22)"</option>
                            <option value="2" selected>"2 Terms (Age 26)"</option>
                            <option value="3">"3 Terms (Age 30)"</option>
                            <option value="4">"4 Terms (Age 34)"</option>
                        </select>
                    </label>
                </div>
                <div class="form-grid">
                    <label>
                        <span>"Optional GM-Only Secret Agenda / Notes (Never Shown to Players)"</span>
                        <input
                            id="npc-gen-secret"
                            type="text"
                            placeholder="e.g. Quietly treating wounded couriers off the books to earn their trust..."
                        />
                    </label>
                    <label class="inline-label">
                        <input id="npc-gen-persist" type="checkbox" checked />
                        <span>"Persist NPC & GM Memory Across Campaign Sessions"</span>
                    </label>
                </div>
                <div class="btn-row">
                    <button
                        type="button"
                        id="btn-generate-npc"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaGenerateNpcFromPrompt()"
                        on:click=move |_| wasm_client::satya_generate_npc_from_prompt()
                    >
                        "Roll Up NPC & Assign Skills + Psyche"
                    </button>
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaLoadSelectedNpcToSheet()"
                        on:click=move |_| wasm_client::satya_load_selected_npc_to_sheet()
                    >
                        "Open Selected NPC in Character Sheet Tab"
                    </button>
                </div>
            </div>
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
                        "GM_ONLY Document Vault, Persistent GM Memory & Omnisearch (`Player -> GM & AI GM Only` · Campaign/Session Persistence)"
                    </span>
                </div>
                <div class="btn-row-compact">
                    <span id="gm-docs-count-pill" class="pill pill-iggy">"0 GM Docs"</span>
                    <span id="gm-memories-count-pill" class="pill pill-accent">"0 GM Memories"</span>
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaRefreshGmDocuments(); window.satyaRefreshGmMemories();"
                        on:click=move |_| {
                            wasm_client::satya_refresh_gm_documents();
                            wasm_client::satya_refresh_gm_memories();
                        }
                    >
                        "Refresh"
                    </button>
                </div>
            </div>

            // --- SCENARIO & CAMPAIGN DIRECTOR DESIGNER (PLOT GUIDE + EVENT PROBABILITY DIALS) ---
            <div id="scenario-director-designer-panel" class="roster-bar">
                <div class="gm-doc-header">
                    <span class="roster-title">
                        "Scenario & Campaign Director (LangGraph Flow Mode, Plot Guide & Event Probability Dials):"
                    </span>
                    <span id="scenario-dials-summary-pill" class="pill pill-active">
                        "Beat Odds: 35%/turn (Cap 3t) · Top: Combat & Ambush: 15% · Mysterious Stranger: 11% · New NPC Arrives: 11%"
                    </span>
                </div>
                <div class="preset-users">
                    <span class="subtitle">"Quick Dial Presets:"</span>
                    {SCENARIO_PRESET_SPECS
                        .iter()
                        .copied()
                        .map(|(preset_id, label)| {
                            let onclick_js =
                                format!("window.satyaApplyScenarioPreset('{preset_id}')");
                            view! {
                                <button
                                    type="button"
                                    class="btn-chip"
                                    onclick=onclick_js
                                    on:click=move |_| {
                                        wasm_client::satya_apply_scenario_preset(
                                            preset_id.to_string(),
                                        )
                                    }
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="form-grid">
                    <label>
                        <span>"Director Flow Mode (`flow_mode`)"</span>
                        <select id="scenario-flow-mode">
                            <option value="hybrid" selected>
                                "Hybrid (Overarching Plot Milestones + Weighted Emergent Dials)"
                            </option>
                            <option value="scenario_driven">
                                "Scenario-Driven (Non-Railroaded Overarching Plot Adaptation)"
                            </option>
                            <option value="free_form">
                                "Free-Form Sandbox (Sample Purely from Event Probability Dials)"
                            </option>
                        </select>
                    </label>
                    <label>
                        <span>"Campaign Premise / Overarching Scenario Guide"</span>
                        <input
                            id="scenario-premise"
                            type="text"
                            value="Orbital quarantine aboard Aethelgard Sector 4; rival syndicates vying over synthetic telemetry."
                            placeholder="e.g. Orbital quarantine aboard Aethelgard Sector 4..."
                        />
                    </label>
                </div>
                <div class="form-grid">
                    <label>
                        <span>
                            "Per-Turn Emergent Beat Probability (`beat_probability_per_turn`): "
                            <strong id="scenario-beat-prob-val" class="mono">"35% / turn"</strong>
                        </span>
                        <input
                            id="scenario-beat-prob"
                            type="range"
                            min="0"
                            max="100"
                            step="5"
                            value="35"
                            oninput="window.satyaUpdateScenarioDialReadouts()"
                            on:input=move |_| wasm_client::satya_update_scenario_dial_readouts()
                        />
                    </label>
                    <label>
                        <span>"Max Rabbit-Hole Turn Ceiling (`beat_interval_turns`, 1-20)"</span>
                        <input
                            id="scenario-beat-interval"
                            type="number"
                            min="1"
                            max="20"
                            value="3"
                            onchange="window.satyaUpdateScenarioDialReadouts()"
                            on:change=move |_| wasm_client::satya_update_scenario_dial_readouts()
                        />
                    </label>
                </div>
                <label>
                    <span>
                        "Planned Scenario Milestones / Plot Beats (One per line — AI GM adapts these to player actions without railroading)"
                    </span>
                    <textarea
                        id="scenario-planned-beats"
                        class="gm-doc-textarea"
                        rows="3"
                        placeholder="e.g.\nEncrypted distress beacon from Bio-Lab 3 reveals a survivor.\nSyndicate enforcers lock down the tramway where the crew is operating."
                    ></textarea>
                </label>
                <div>
                    <span class="subtitle">
                        "Event Probability Dials (0 = Disable Event Completely · Higher = Higher Relative Probability):"
                    </span>
                    <div class="scenario-dial-grid">
                        {wasm_client::SCENARIO_DIRECTOR_DIAL_SPECS
                            .iter()
                            .copied()
                            .map(|(slug, label, pillar_hint, default_w)| {
                                let input_id = format!("scenario-dial-{slug}");
                                let val_id = format!("scenario-dial-val-{slug}");
                                let default_str = default_w.to_string();
                                let readout_init = format!("{default_w}");
                                view! {
                                    <div class="scenario-dial-card">
                                        <div class="scenario-dial-header">
                                            <strong>{label}</strong>
                                            <span id=val_id class="mono">{readout_init}</span>
                                        </div>
                                        <span class="subtitle">{pillar_hint}</span>
                                        <input
                                            id=input_id
                                            type="range"
                                            min="0"
                                            max="100"
                                            step="5"
                                            value=default_str
                                            oninput="window.satyaUpdateScenarioDialReadouts()"
                                            on:input=move |_| wasm_client::satya_update_scenario_dial_readouts()
                                        />
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
                <div class="btn-row">
                    <label class="checkbox-label">
                        <input id="scenario-persistent" type="checkbox" checked=true />
                        <span>"Persist Scenario Director Config in Campaign/Session SQLite Vault (`mem-scenario-director`)"</span>
                    </label>
                    <button
                        type="button"
                        id="btn-save-scenario-director"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaSaveScenarioDirectorConfig()"
                        on:click=move |_| wasm_client::satya_save_scenario_director_config()
                    >
                        "Save & Pin Scenario Director Config"
                    </button>
                </div>
            </div>

            // --- GM ARCHIVIST PANEL (RULES + PERSISTENT GM DOCS + GM MEMORIES + SHEETS + COMBAT + MAP) ---
            <div class="roster-bar">
                <span class="roster-title">"0. GM Archivist (information lookup)"</span>
                <div class="form-grid">
                    <label>
                        <span>"Search Query"</span>
                        <input
                            id="gm-omnisearch-input"
                            type="text"
                            placeholder="e.g. Sara Valkyrie, bleeding trauma, tick timeline, or secret objective..."
                            onchange="window.satyaSearchGmOmnisearch()"
                            on:change=move |_| wasm_client::satya_search_gm_omnisearch()
                        />
                    </label>
                    <label>
                        <span>"Lookup Scope"</span>
                        <select
                            id="gm-omnisearch-scope"
                            onchange="window.satyaSearchGmOmnisearch()"
                            on:change=move |_| wasm_client::satya_search_gm_omnisearch()
                        >
                            {GM_OMNISEARCH_SCOPE_OPTIONS
                                .iter()
                                .copied()
                                .map(|(code, label)| view! { <option value=code>{label}</option> })
                                .collect_view()}
                        </select>
                    </label>
                </div>
                <div class="btn-row">
                    <button
                        type="button"
                        id="btn-gm-omnisearch"
                        class="btn btn-sm btn-primary"
                        onclick="window.satyaSearchGmOmnisearch()"
                        on:click=move |_| wasm_client::satya_search_gm_omnisearch()
                    >
                        "Search GM Vault & Rules"
                    </button>
                </div>
                <div id="gm-omnisearch-results" class="gm-docs-list">
                    <span class="pill pill-muted">"Enter a query above to search across Rules, Persistent GM Docs, GM Memory Notes, Character Sheets, Combat Tracker, and Map state."</span>
                </div>
            </div>

            <div class="roster-bar">
                <span class="roster-title">"1. Specify File: Upload Local File, from Google Drive, or Load Demo Preset:"</span>
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
                            value=""
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
                <div class="form-grid">
                    <label class="checkbox-label">
                        <input id="gm-doc-persistent" type="checkbox" checked=true />
                        <span>"Persist across sessions (save to Campaign/Session SQLite GM Vault)"</span>
                    </label>
                    <label>
                        <span>"Campaign Tag / Scope (defaults to current Session ID)"</span>
                        <input
                            id="gm-doc-campaign-tag"
                            type="text"
                            placeholder="Optional campaign tag (e.g. campaign-alpha)"
                        />
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
                <span class="roster-title">"Session & Persistent GM_ONLY Documents:"</span>
                <div id="gm-docs-list" class="gm-docs-list">
                    <span class="pill pill-muted">"No GM_ONLY documents submitted in this session yet."</span>
                </div>
            </div>

            // --- PERSISTENT GM MEMORY NOTES & PINNED AI GM FACTS ---
            <div class="roster-bar">
                <span class="roster-title">"2. Persistent GM Memory Notes & Pinned AI GM Facts (Shared Human + AI GM Campaign Memory):"</span>
                <div class="form-grid">
                    <label>
                        <span>"Memory Note Title / Fact Key"</span>
                        <input
                            id="gm-memory-title"
                            type="text"
                            placeholder="e.g. Dock 7 Ambush Trigger or Ruling: Zero-G Recoil"
                        />
                    </label>
                    <label>
                        <span>"Memory Category"</span>
                        <select id="gm-memory-category">
                            {GM_MEMORY_CATEGORY_OPTIONS
                                .iter()
                                .copied()
                                .map(|(code, label)| view! { <option value=code>{label}</option> })
                                .collect_view()}
                        </select>
                    </label>
                </div>
                <div class="form-grid">
                    <label class="checkbox-label">
                        <input id="gm-memory-pinned" type="checkbox" checked=true />
                        <span>"Pin to AI GM Core Working Memory (always visible in AI GM system header)"</span>
                    </label>
                    <label class="checkbox-label">
                        <input id="gm-memory-persistent" type="checkbox" checked=true />
                        <span>"Persist across sessions in Campaign/Session SQLite Vault"</span>
                    </label>
                </div>
                <label>
                    <span>"GM Memory Fact / Ruling / Scene Note (`<= 16 KB`)"</span>
                    <textarea
                        id="gm-memory-content"
                        class="gm-doc-textarea"
                        rows="3"
                        placeholder="Record a persistent GM note, NPC secret, or table ruling that both Human and AI GMs will remember across sessions..."
                    ></textarea>
                </label>
                <div class="btn-row">
                    <button
                        type="button"
                        id="btn-submit-gm-memory"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaSubmitGmMemory()"
                        on:click=move |_| wasm_client::satya_submit_gm_memory()
                    >
                        "Save GM Memory Note"
                    </button>
                </div>
                <div id="gm-memories-list" class="gm-docs-list">
                    <span class="pill pill-muted">"No persistent GM memory notes recorded in this session yet."</span>
                </div>
            </div>
        </div>
    }
}
