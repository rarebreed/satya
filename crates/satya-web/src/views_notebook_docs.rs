//! Companion Leptos component (`NotebookReferencePanes`) for the Tabletop Notebook's
//! Combat Tracker (`#notebook-pane-combat`), Satya TTRPG Rules Book (`#notebook-pane-rules`),
//! `GM_ONLY` Document Vault (`#notebook-pane-docs`), and Apache Iggy Inspector (`#notebook-pane-iggy`).

use crate::wasm_client;
use leptos::prelude::*;

/// Static table of Combat Stance reference rows in the Combat Tracker (`(name, tc_label, windup_label, effect)`).
const COMBAT_STANCE_TABLE_ROWS: &[(&str, &str, &str, &str)] = &[
    (
        "Melee: Maneuver / Probe",
        "11 TC (4 decide + 7 act)",
        "8 wind-up · 2 floor",
        "INS/CRE/KIN; +1 defensive parry margin; D_cog & N>=4 compress TC_decide",
    ),
    (
        "Melee: Heavy Power Strike",
        "14 TC (3 decide + 11 act)",
        "9 wind-up · 2 floor",
        "FOR/SPD/MAS; +2 Wound Shock on hit; vulnerable to fast interrupts",
    ),
    (
        "Melee: Defensive Guard / Stability",
        "9 TC (3 decide + 6 act)",
        "6 wind-up · 2 floor",
        "KIN/INS/SPD; braced stance resists knockdown & boosts Guard Pool",
    ),
    (
        "Ranged: Aimed Single Shot",
        "12 TC (5 decide + 7 act)",
        "9 wind-up · 2 floor",
        "SIG/FOC/DEX; deliberate sight acquisition (-2 to -4 to TN)",
    ),
    (
        "Ranged: Snap Shot",
        "7 TC (2 decide + 5 act)",
        "5 wind-up · 2 floor",
        "SPD/DEX/ITU; rapid reaction shot (-2 precision penalty)",
    ),
];

/// Worked Rules Book example presets (`(preset_id, button_label)`).
const WORKED_BOOK_EXAMPLES: &[(&str, &str)] = &[
    (
        "sarah_lockpick",
        "Sarah's Lockpick (N=3, C=12, Δ_tool=0, Δ_int=+2, Δ_ext=+4)",
    ),
    (
        "marcus_surgery",
        "Marcus's Trauma Surgery (N=4, C=14, Δ_tool=-2, Δ_int=-1, Δ_ext=+4)",
    ),
    (
        "elena_hack",
        "Elena's Intrusion Hack (N=3, C=15, Δ_tool=-2, Δ_ext=+2)",
    ),
    ("elena_probe", "Elena's Sword Probe (N=5, C=16, Δ_ext=+2)"),
    (
        "viktor_cleave",
        "Viktor's Greatsword Cleave (N=4, C=13, Δ_ext=+3)",
    ),
    (
        "social_persuasion",
        "Social Rhetoric (N=3, C=15, Δ_int=+2, Δ_ext=+3)",
    ),
];

/// Renders the Combat Tracker, Rules Book, `GM_ONLY` Document Vault, and Apache Iggy panes
/// inside `.notebook-body`.
#[component]
pub fn NotebookReferencePanes() -> impl IntoView {
    view! {
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
                    <span class="subtitle">"Satya Count-Up Tick Timeline (1 Tick = 0.5s) & Two-Component Action Economy"</span>
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
                            <th>"Combat Stance / Action"</th>
                            <th>"Base TC (TC_decide + TC_act)"</th>
                            <th>"Wind-Up / Floor"</th>
                            <th>"Tactical & OODA Effect"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {COMBAT_STANCE_TABLE_ROWS
                            .iter()
                            .copied()
                            .map(|(stance_name, tc_label, windup, effect)| {
                                view! {
                                    <tr>
                                        <td><strong>{stance_name}</strong></td>
                                        <td><span class="mono">{tc_label}</span></td>
                                        <td>{windup}</td>
                                        <td>{effect}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
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
                            {satya_engine::list_rules_chapters()
                                .into_iter()
                                .enumerate()
                                .map(|(idx, ch)| {
                                    let option_label = format!("{}. {}", idx + 1, ch.title);
                                    let is_default = ch.chapter_id == "tasks";
                                    view! {
                                        <option value=ch.chapter_id selected=is_default>
                                            {option_label}
                                        </option>
                                    }
                                })
                                .collect_view()}
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
                    {WORKED_BOOK_EXAMPLES
                        .iter()
                        .copied()
                        .map(|(preset_id, label)| {
                            let onclick_js =
                                format!("window.satyaLoadRulesExample('{preset_id}')");
                            view! {
                                <button
                                    type="button"
                                    class="btn-chip"
                                    onclick=onclick_js
                                    on:click=move |_| {
                                        wasm_client::satya_load_rules_example(
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

        // --- TAB 8: APACHE IGGY, TOPICS AND PYLANCE STATS ---
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
    }
}
