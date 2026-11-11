//! Companion Leptos component (`NotebookReferencePanes`) for the Tabletop Notebook's
//! Combat Tracker (`#notebook-pane-combat`), Satya TTRPG Rules Book (`#notebook-pane-rules`),
//! GM/NPC panes ([`NotebookGmAndNpcPanes`]), and Apache Iggy Inspector (`#notebook-pane-iggy`).

pub use crate::views_notebook_gm::*;

use crate::wasm_client;
use leptos::prelude::*;

/// Static table of Combat Stance reference rows (`(code, name, tc_label, windup_label, effect)`).
const COMBAT_STANCE_TABLE_ROWS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "MANEUVER",
        "Melee: Maneuver / Probe",
        "11 TC (4 decide + 7 act)",
        "8 wind-up · 2 floor",
        "INS/CRE/KIN; +1 defensive parry margin; D_cog & N>=4 compress TC_decide",
    ),
    (
        "POWER",
        "Melee: Heavy Power Strike",
        "14 TC (3 decide + 11 act)",
        "9 wind-up · 2 floor",
        "FOR/SPD/MAS; +2 Wound Shock on hit; vulnerable to fast interrupts",
    ),
    (
        "STABILITY",
        "Melee: Defensive Guard / Stability",
        "9 TC (3 decide + 6 act)",
        "6 wind-up · 2 floor",
        "KIN/INS/SPD; braced stance resists knockdown & boosts Guard Pool",
    ),
    (
        "RANGED_AIMED",
        "Ranged: Aimed Single Shot",
        "12 TC (5 decide + 7 act)",
        "9 wind-up · 2 floor",
        "SIG/FOC/DEX; deliberate sight acquisition (-2 to -4 to TN)",
    ),
    (
        "RANGED_SNAP",
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

/// Renders the Combat Tracker, Rules Book, [`NotebookGmAndNpcPanes`], and Apache Iggy panes
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
                <div class="model-controls-row">
                    <span class="subtitle">"Satya Count-Up Tick Timeline (1 Tick = 0.5s) · 30 Combat Techniques × TCF Triad"</span>
                    <label class="inline-label" for="ttrpg-stance">
                        <span>"Combat Action / Technique"</span>
                        <select id="ttrpg-stance">
                            <option value="">"Non-Combat Task (TC 10)"</option>
                            {COMBAT_STANCE_TABLE_ROWS
                                .iter()
                                .copied()
                                .map(|(code, name, tc, _, _)| view! { <option value={code}>{format!("{name} ({tc})")}</option> })
                                .collect_view()}
                            {satya_engine::canonical_combat_technique_catalog()
                                .iter()
                                .map(|spec| {
                                    let code = spec.id.as_str();
                                    let label = format!(
                                        "[{}] {} ({} TC = {}d + {}a)",
                                        spec.category.label(),
                                        spec.name,
                                        spec.base_total_tc,
                                        spec.base_tc_decide,
                                        spec.base_tc_act,
                                    );
                                    view! { <option value={code}>{label}</option> }
                                })
                                .collect_view()}
                        </select>
                    </label>
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
                            <th>"Category / Technique (What You Do)"</th>
                            <th>"Base TC (TC_decide + TC_act)"</th>
                            <th>"Floor / Impact"</th>
                            <th>"TCF Triad Execution (Tempo / Control / Force)"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {COMBAT_STANCE_TABLE_ROWS
                            .iter()
                            .copied()
                            .map(|(_, stance_name, tc_label, windup, effect)| {
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
                        {satya_engine::canonical_combat_technique_catalog()
                            .iter()
                            .map(|spec| {
                                let name_col = format!("{}: {}", spec.category.label(), spec.name);
                                let tc_col = format!(
                                    "{} TC ({} decide + {} act)",
                                    spec.base_total_tc,
                                    spec.base_tc_decide,
                                    spec.base_tc_act,
                                );
                                let floor_col = format!(
                                    "{} floor · {:.2}× POW (Guard {:+}, Root {:+})",
                                    spec.min_floor_ticks,
                                    spec.pow_multiplier,
                                    spec.guard_mod,
                                    spec.root_mod,
                                );
                                view! {
                                    <tr>
                                        <td><strong>{name_col}</strong></td>
                                        <td><span class="mono">{tc_col}</span></td>
                                        <td>{floor_col}</td>
                                        <td>{spec.tcf_interplay_notes}</td>
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

        <NotebookGmAndNpcPanes/>

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
