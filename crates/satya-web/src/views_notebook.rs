//! Left-column Tabbed Tabletop Notebook (`NotebookColumn`) rendering the tab header and
//! the primary interactive stages (`Maps`, `3D Dice`, `3D Model`, `Character Sheet/Status`,
//! and `6-Step Character Wizard`), plus [`NotebookReferencePanes`] for `Combat Tracker`,
//! `Rules Book`, `GM_ONLY Document Vault`, and `Apache Iggy`.

pub use crate::views_notebook_docs::*;

use crate::wasm_client;
use leptos::prelude::*;

/// Quick-roll 3D polyhedral dice button specifications (`(notation, button_label)`).
const QUICK_3D_DICE_SPECS: &[(&str, &str)] = &[
    ("1d4", "1d4"),
    ("2d6", "2d6"),
    ("1d8", "1d8"),
    ("1d10", "1d10"),
    ("1d12", "1d12"),
    ("2d20", "Roll 2d20"),
    ("4d20", "Roll 4d20"),
    ("6d20", "Roll 6d20"),
];

/// Default Character Sheet stat summary boxes (`(label, optional_id, default_val, note)`).
const CHAR_SHEET_STAT_BOXES: &[(&str, Option<&str>, &str, &str)] = &[
    (
        "Control (C)",
        Some("char-stat-control"),
        "12",
        "DEX / TAC Composite",
    ),
    (
        "Speed (SPD)",
        Some("char-stat-spd"),
        "14",
        "OODA / Tempo Cap",
    ),
    (
        "Kinesthesia (KIN)",
        Some("char-stat-kin"),
        "14",
        "Balance / Melee Control",
    ),
    (
        "Analysis (ANA)",
        Some("char-stat-ana"),
        "15",
        "Quick Thinking (D_cog 14)",
    ),
    (
        "Discipline (DIS)",
        Some("char-stat-dis"),
        "14",
        "FT 19 / WSR 20 / CMP 6",
    ),
    (
        "Armor Mitigation",
        None,
        "DR 4",
        "EVA Ballistic · Tier 2 Load",
    ),
];

/// Default Character Sheet trained skill presets (`(skill_name, skill_level, control_c, chip_label)`).
const DEFAULT_SKILL_PRESETS: &[(&str, i32, i32, &str)] = &[
    (
        "Trauma, Orthopedic & General Surgery",
        3,
        12,
        "Trauma Surgery [Tree 3] (N=3, DEX/FOC=12)",
    ),
    (
        "Zero-G / EVA Operations",
        4,
        14,
        "Zero-G / EVA Ops [Tree 5] (N=4, KIN/FOC=14)",
    ),
    (
        "Fully Automatic & Burst Weapons",
        3,
        13,
        "Full-Auto & Burst [Tree 6] (N=3, FOR/DEX=13)",
    ),
    (
        "Surveillance, Reconnaissance & Countersurveillance",
        2,
        15,
        "Surveillance & Recon [Tree 7] (N=2, SIG/FOC=15)",
    ),
    (
        "Small-Unit Tactics & CQB Doctrine",
        4,
        11,
        "Small-Unit Tactics & CQB [Tree 6] (N=4, INS/ANA=11)",
    ),
];

/// Left-column Tabbed Tabletop Notebook (`#notebook-column`).
#[component]
pub fn NotebookColumn() -> impl IntoView {
    view! {
        <div id="notebook-column" class="col-notebook">
            <div id="tabletop-canvas-card" class="card tabletop-canvas-card notebook-card">
                <div class="notebook-header">
                    <div class="notebook-title-row">
                        <div>
                            <h2>"Tabletop Notebook"</h2>
                        </div>
                        <span id="canvas-map-pill" class="pill pill-muted">"Standby Grid"</span>
                    </div>
                    <div class="notebook-tabs" role="tablist" aria-label="Tabletop Notebook Views">
                        {wasm_client::NOTEBOOK_TAB_SPECS
                            .iter()
                            .enumerate()
                            .map(|(idx, (key, btn_id, pane_id, label))| {
                                let is_first = idx == 0;
                                let aria_sel = if is_first { "true" } else { "false" };
                                let cls = if is_first {
                                    "notebook-tab active"
                                } else {
                                    "notebook-tab"
                                };
                                let onclick_js = format!("window.satyaSelectNotebookTab('{key}')");
                                let key_owned = (*key).to_string();
                                view! {
                                    <button
                                        type="button"
                                        id=*btn_id
                                        role="tab"
                                        aria-selected=aria_sel
                                        aria-controls=*pane_id
                                        data-notebook-tab=*key
                                        class=cls
                                        onclick=onclick_js
                                        on:click=move |_| {
                                            wasm_client::satya_select_notebook_tab(key_owned.clone())
                                        }
                                    >
                                        {*label}
                                    </button>
                                }
                            })
                            .collect_view()}
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
                                {QUICK_3D_DICE_SPECS
                                    .iter()
                                    .copied()
                                    .map(|(notation, label)| {
                                        let onclick_js =
                                            format!("window.satyaQuick3dDiceRoll('{notation}')");
                                        view! {
                                            <button
                                                type="button"
                                                class="btn btn-sm btn-secondary"
                                                onclick=onclick_js
                                                on:click=move |_| {
                                                    wasm_client::satya_quick_3d_dice_roll(
                                                        notation.to_string(),
                                                    )
                                                }
                                            >
                                                {label}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
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
                        <div class="dice-material-toolbar" role="group" aria-label="3D Dice Material & Translucency Controls">
                            <label class="inline-label" for="dice-material-preset">
                                <span>"Material Preset"</span>
                                <select
                                    id="dice-material-preset"
                                    onchange="window.satyaSelectDiceMaterialPreset()"
                                    on:change=move |_| wasm_client::satya_select_dice_material_preset()
                                >
                                    <option value="translucent_sapphire">"Translucent Sapphire"</option>
                                    <option value="amber_resin">"Amber Resin"</option>
                                    <option value="emerald_glass">"Emerald Glass"</option>
                                    <option value="crimson_ruby">"Crimson Ruby"</option>
                                    <option value="amethyst_void">"Amethyst Void"</option>
                                    <option value="obsidian_opaque">"Obsidian Opaque"</option>
                                </select>
                            </label>
                            <label class="inline-label" for="dice-base-color">
                                <span>"Die Color"</span>
                                <input
                                    type="color"
                                    id="dice-base-color"
                                    class="dice-color-input"
                                    value="#0ea5e9"
                                    oninput="window.satyaUpdateDiceMaterial()"
                                    on:input=move |_| wasm_client::satya_update_dice_material()
                                />
                            </label>
                            <label class="inline-label" for="dice-ink-color">
                                <span>"Numeral Ink"</span>
                                <input
                                    type="color"
                                    id="dice-ink-color"
                                    class="dice-color-input"
                                    value="#f8fafc"
                                    oninput="window.satyaUpdateDiceMaterial()"
                                    on:input=move |_| wasm_client::satya_update_dice_material()
                                />
                            </label>
                            <label class="inline-label dice-slider-label" for="dice-opacity-slider">
                                <span>"Opacity / Translucency"</span>
                                <input
                                    type="range"
                                    id="dice-opacity-slider"
                                    class="dice-opacity-slider"
                                    min="8"
                                    max="100"
                                    step="1"
                                    value="38"
                                    aria-valuetext="38% opacity"
                                    oninput="window.satyaUpdateDiceMaterial()"
                                    on:input=move |_| wasm_client::satya_update_dice_material()
                                />
                                <span id="dice-opacity-readout" class="mono">"38%"</span>
                            </label>
                            <span id="wgpu-status-pill" class="pill pill-iggy">"WebGPU Ready · 2-Pass Translucent"</span>
                        </div>
                        <div class="dice-camera-toolbar" role="group" aria-label="3D Dice Camera Angle, Pan & Zoom Controls">
                            <label class="inline-label dice-camera-slider-label" for="dice-camera-yaw">
                                <span>"Orbit Yaw"</span>
                                <input
                                    type="range"
                                    id="dice-camera-yaw"
                                    class="dice-opacity-slider"
                                    min="-180"
                                    max="180"
                                    step="1"
                                    value="0"
                                    aria-valuetext="0 degrees yaw"
                                    oninput="window.satyaUpdateDiceCamera()"
                                    on:input=move |_| wasm_client::satya_update_dice_camera()
                                />
                                <span id="dice-camera-yaw-readout" class="mono">"0°"</span>
                            </label>
                            <label class="inline-label dice-camera-slider-label" for="dice-camera-pitch">
                                <span>"Viewing Angle"</span>
                                <input
                                    type="range"
                                    id="dice-camera-pitch"
                                    class="dice-opacity-slider"
                                    min="12"
                                    max="88"
                                    step="1"
                                    value="55"
                                    aria-valuetext="55 degrees pitch"
                                    oninput="window.satyaUpdateDiceCamera()"
                                    on:input=move |_| wasm_client::satya_update_dice_camera()
                                />
                                <span id="dice-camera-pitch-readout" class="mono">"55°"</span>
                            </label>
                            <label class="inline-label dice-camera-slider-label" for="dice-camera-zoom">
                                <span>"Zoom"</span>
                                <input
                                    type="range"
                                    id="dice-camera-zoom"
                                    class="dice-opacity-slider"
                                    min="40"
                                    max="250"
                                    step="5"
                                    value="100"
                                    aria-valuetext="100% zoom"
                                    oninput="window.satyaUpdateDiceCamera()"
                                    on:input=move |_| wasm_client::satya_update_dice_camera()
                                />
                                <span id="dice-camera-zoom-readout" class="mono">"100%"</span>
                            </label>
                            <div class="dice-pan-controls">
                                <span class="subtitle">"Pan:"</span>
                                <button
                                    type="button"
                                    id="btn-dice-pan-left"
                                    class="btn btn-sm btn-secondary"
                                    title="Pan camera left"
                                    aria-label="Pan camera left"
                                    onclick="window.satyaPanDiceCamera(-0.6, 0.0)"
                                    on:click=move |_| wasm_client::satya_pan_dice_camera(-0.6, 0.0)
                                >
                                    "◀"
                                </button>
                                <button
                                    type="button"
                                    id="btn-dice-pan-up"
                                    class="btn btn-sm btn-secondary"
                                    title="Pan camera forward"
                                    aria-label="Pan camera forward"
                                    onclick="window.satyaPanDiceCamera(0.0, 0.6)"
                                    on:click=move |_| wasm_client::satya_pan_dice_camera(0.0, 0.6)
                                >
                                    "▲"
                                </button>
                                <button
                                    type="button"
                                    id="btn-dice-pan-down"
                                    class="btn btn-sm btn-secondary"
                                    title="Pan camera backward"
                                    aria-label="Pan camera backward"
                                    onclick="window.satyaPanDiceCamera(0.0, -0.6)"
                                    on:click=move |_| wasm_client::satya_pan_dice_camera(0.0, -0.6)
                                >
                                    "▼"
                                </button>
                                <button
                                    type="button"
                                    id="btn-dice-pan-right"
                                    class="btn btn-sm btn-secondary"
                                    title="Pan camera right"
                                    aria-label="Pan camera right"
                                    onclick="window.satyaPanDiceCamera(0.6, 0.0)"
                                    on:click=move |_| wasm_client::satya_pan_dice_camera(0.6, 0.0)
                                >
                                    "▶"
                                </button>
                                <span id="dice-camera-pan-readout" class="mono">"(+0.0, +0.0)"</span>
                                <button
                                    type="button"
                                    id="btn-reset-dice-camera"
                                    class="btn btn-sm btn-secondary"
                                    title="Reset camera angle, pan, and zoom (or double-click canvas)"
                                    onclick="window.satyaResetDiceCamera()"
                                    on:click=move |_| wasm_client::satya_reset_dice_camera()
                                >
                                    "Reset View"
                                </button>
                            </div>
                        </div>
                        <div id="wgpu-dice-tray" class="wgpu-dice-tray notebook-dice-stage">
                            <span class="roster-title">"3D Physics Dice Tray (`wgpu` Seed Sync · Drag: Orbit Angle · Shift+Drag / Right-Drag: Pan · Wheel: Zoom):"</span>
                            <div class="wgpu-dice-canvas-wrap">
                                <canvas
                                    id="wgpu-dice-canvas"
                                    class="wgpu-dice-canvas"
                                    width="640"
                                    height="320"
                                    aria-label="3D WebGPU Polyhedral Dice Stage"
                                ></canvas>
                            </div>
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
                            inner_html=wasm_client::default_3d_model_stage_html()
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
                            <span class="subtitle">"Satya 25-Characteristic Operator Profile, Derived OODA/D_cog & 8-Tree Skill Presets"</span>
                            <span id="char-sheet-status-pill" class="pill pill-active">"Combat Ready · OODA 13 · D_cog 14 · Fatigue 1 · Shock 0.0"</span>
                        </div>
                        <div class="char-sheet-grid">
                            {CHAR_SHEET_STAT_BOXES
                                .iter()
                                .copied()
                                .map(|(label, maybe_id, val, note)| {
                                    view! {
                                        <div class="char-stat-box">
                                            <span class="stat-label">{label}</span>
                                            <span id=maybe_id class="stat-value">{val}</span>
                                            <span class="stat-note">{note}</span>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                        <div class="roster-bar">
                            <span class="roster-title">"Trained 8-Tree Skills (Click to Load into Die Roller):"</span>
                            <div id="char-sheet-skills-chips" class="preset-users">
                                {DEFAULT_SKILL_PRESETS
                                    .iter()
                                    .copied()
                                    .map(|(skill, n, c, label)| {
                                        let onclick_js =
                                            format!("window.satyaLoadSkillPreset('{skill}', {n}, {c})");
                                        view! {
                                            <button
                                                type="button"
                                                class="btn-chip"
                                                onclick=onclick_js
                                                on:click=move |_| {
                                                    wasm_client::satya_load_skill_preset(
                                                        skill.to_string(),
                                                        n,
                                                        c,
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
                        <div id="char-sheet-lifepath-summary" class="roster-bar">
                            <span class="roster-title">"Lifepath, [Access] Gates & Economy:"</span>
                            <div class="roster-chips">
                                <span class="pill pill-iggy">"Use Character Wizard Tab to Generate Lifepath & Skills"</span>
                            </div>
                        </div>
                    </div>

                    // --- TAB 4B: CHARACTER CREATION WIZARD ---
                    <div
                        id="notebook-pane-wizard"
                        role="tabpanel"
                        aria-labelledby="tab-btn-wizard"
                        data-notebook-pane="wizard"
                        class="notebook-pane"
                        style="display: none;"
                    >
                        <div class="pane-toolbar">
                            <div class="model-controls-row">
                                <label class="inline-label" for="wiz-char-name-input">
                                    <span>"Character Name"</span>
                                    <input
                                        id="wiz-char-name-input"
                                        name="wiz_char_name"
                                        type="text"
                                        autocorrect="off"
                                        spellcheck="false"
                                        value="New Operator"
                                        onchange="window.satyaWizardNameChanged()"
                                    />
                                </label>
                            </div>
                            <span id="wizard-status-pill" class="pill pill-active">
                                "Step 1/4 · OODA 10 · D_cog 10 · POW 10.0"
                            </span>
                        </div>
                        <div id="wizard-feedback" class="feedback-box" aria-live="polite"></div>
                        <div
                            id="wizard-stage-container"
                            class="wizard-stage-container"
                            inner_html=wasm_client::default_character_wizard_html()
                        ></div>
                    </div>

                    <NotebookReferencePanes/>
                </div>
            </div>
        </div>
    }
}
