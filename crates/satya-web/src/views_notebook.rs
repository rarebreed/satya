//! Left-column Tabbed Tabletop Notebook (`NotebookColumn`) rendering the tab header and
//! the primary interactive stages (`Maps`, `3D Dice`, `3D Model`, `Character Sheet/Status`,
//! and `6-Step Character Wizard`), plus [`NotebookReferencePanes`] for `Combat Tracker`,
//! `Rules Book`, `NPCs`, `GM_ONLY Document Vault`, and `Apache Iggy`.

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

/// Shared 3D character miniature rig presets (`(preset_id, short_token_label, full_stage_label)`).
const MODEL_RIG_OPTIONS: &[(&str, &str, &str)] = &[
    ("valkyrie_rig", "Valkyrie Rig", "Valkyrie Exo-Rig (Sara)"),
    ("recon_mk4", "Recon Mk-IV", "Recon Suit Mk-IV (Marcus)"),
    ("secops_sentry", "SecOps Sentry", "SecOps Heavy Sentry"),
    ("operative_light", "Light Operative", "Light Tactical Operative"),
];

/// 3D Dice translucent material presets (`(value, label)`).
const DICE_MATERIAL_OPTIONS: &[(&str, &str)] = &[
    ("translucent_sapphire", "Translucent Sapphire"),
    ("amber_resin", "Amber Resin"),
    ("emerald_glass", "Emerald Glass"),
    ("crimson_ruby", "Crimson Ruby"),
    ("amethyst_void", "Amethyst Void"),
    ("obsidian_opaque", "Obsidian Opaque"),
];

/// 3D Dice camera range slider specifications:
/// `(input_id, readout_id, label, min, max, step, default_val, aria_valuetext, init_readout)`.
type DiceCameraSliderSpec = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

const DICE_CAMERA_SLIDERS: &[DiceCameraSliderSpec] = &[
    (
        "dice-camera-yaw",
        "dice-camera-yaw-readout",
        "Orbit Yaw",
        "-180",
        "180",
        "1",
        "0",
        "0 degrees yaw",
        "0°",
    ),
    (
        "dice-camera-pitch",
        "dice-camera-pitch-readout",
        "Viewing Angle",
        "12",
        "88",
        "1",
        "55",
        "55 degrees pitch",
        "55°",
    ),
    (
        "dice-camera-zoom",
        "dice-camera-zoom-readout",
        "Zoom",
        "40",
        "250",
        "5",
        "100",
        "100% zoom",
        "100%",
    ),
];

/// 3D Dice camera pan button specifications: `(btn_id, title, dx, dy, onclick_js, glyph)`.
const DICE_PAN_BUTTONS: &[(&str, &str, f32, f32, &str, &str)] = &[
    (
        "btn-dice-pan-left",
        "Pan camera left",
        -0.6,
        0.0,
        "window.satyaPanDiceCamera(-0.6, 0.0)",
        "◀",
    ),
    (
        "btn-dice-pan-up",
        "Pan camera forward",
        0.0,
        0.6,
        "window.satyaPanDiceCamera(0.0, 0.6)",
        "▲",
    ),
    (
        "btn-dice-pan-down",
        "Pan camera backward",
        0.0,
        -0.6,
        "window.satyaPanDiceCamera(0.0, -0.6)",
        "▼",
    ),
    (
        "btn-dice-pan-right",
        "Pan camera right",
        0.6,
        0.0,
        "window.satyaPanDiceCamera(0.6, 0.0)",
        "▶",
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
                    <NotebookMapPane/>
                    <NotebookDicePane/>
                    <NotebookModelAndCharacterPanes/>
                    <NotebookReferencePanes/>
                </div>
            </div>
        </div>
    }
}

#[component]
fn NotebookMapPane() -> impl IntoView {
    view! {
        <div
            id="notebook-pane-maps"
            role="tabpanel"
            aria-labelledby="tab-btn-maps"
            data-notebook-pane="maps"
            class="notebook-pane active"
        >
            <div class="pane-toolbar map-tactical-toolbar">
                <div class="model-controls-row">
                    <label class="inline-label" for="map-preset-select">
                        <span>"Sector"</span>
                        <select id="map-preset-select">
                            {satya_engine::SectorPreset::ALL
                                .iter()
                                .map(|preset| {
                                    view! { <option value=preset.as_str()>{preset.label()}</option> }
                                })
                                .collect_view()}
                        </select>
                    </label>
                    <button
                        type="button"
                        id="btn-map-generate"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaGenerateProceduralMap()"
                        on:click=move |_| wasm_client::satya_generate_procedural_map()
                    >
                        "Generate Sector"
                    </button>
                    <button
                        type="button"
                        id="btn-map-toggle-proj"
                        class="btn btn-sm btn-secondary"
                        title="Switch between 3D Perspective Diorama and 2D Top-Down Orthographic Grid"
                        onclick="window.satyaToggleMapProjection()"
                        on:click=move |_| wasm_client::satya_toggle_map_projection()
                    >
                        "View: 3D Diorama"
                    </button>
                    <button
                        type="button"
                        id="btn-map-view-mode"
                        class="btn btn-sm btn-accent"
                        title="Switch between the WebGPU 3D diorama and the illustrated 2D battlemap"
                        onclick="window.satyaToggleBattlemapView()"
                        on:click=move |_| wasm_client::satya_toggle_battlemap_view()
                    >
                        "Mode: 3D Diorama"
                    </button>
                    <button
                        type="button"
                        id="btn-map-grid-overlay"
                        class="btn btn-sm btn-secondary"
                        title="Show or hide the 1.0m / 5.0m visual reference grid on the 2D battlemap"
                        style="display: none;"
                        onclick="window.satyaToggleBattlemapGrid()"
                        on:click=move |_| wasm_client::satya_toggle_battlemap_grid()
                    >
                        "Grid: On"
                    </button>
                    <button
                        type="button"
                        id="btn-map-inspect-toggle"
                        class="btn btn-sm btn-secondary"
                        title="Pin the floating Object & Ground Inspector card (or hold Alt / press I while hovering)"
                        onclick="window.satyaToggleMapInspectMode()"
                        on:click=move |_| wasm_client::satya_toggle_map_inspect_mode()
                    >
                        "Inspect [Alt/I]"
                    </button>
                    <label
                        id="map-ink-style-wrap"
                        class="inline-label"
                        for="map-ink-style-select"
                        style="display: none;"
                    >
                        <span>"Style"</span>
                        <select
                            id="map-ink-style-select"
                            title="2D SVG battlemap paper and ink filter finish"
                            onchange="window.satyaSetBattlemapInkStyle()"
                            on:change=move |_| wasm_client::satya_set_battlemap_ink_style()
                        >
                            {wasm_client::BattlemapInkStyle::ALL
                                .iter()
                                .map(|style| {
                                    view! { <option value=style.as_str()>{style.label()}</option> }
                                })
                                .collect_view()}
                        </select>
                    </label>
                    <button
                        type="button"
                        id="btn-map-wall-cutaway"
                        class="btn btn-sm btn-secondary"
                        title="Toggle full 2.4m bulkheads vs. 0.6m cutaway walls"
                        onclick="window.satyaToggleMapWallCutaway()"
                        on:click=move |_| wasm_client::satya_toggle_map_wall_cutaway()
                    >
                        "Walls: Cutaway"
                    </button>
                    <button
                        type="button"
                        id="btn-map-reset-cam"
                        class="btn btn-sm btn-secondary"
                        title="Reset camera angle, pan, and zoom (or double-click canvas)"
                        onclick="window.satyaResetMapCamera()"
                        on:click=move |_| wasm_client::satya_reset_map_camera()
                    >
                        "Reset View"
                    </button>
                </div>
            </div>

            <div class="dice-material-toolbar map-tactical-subbar" role="group" aria-label="Tactical Movement, Cover & Miniature Controls">
                <label class="inline-label" for="map-weapon-select">
                    <span>"Weapon"</span>
                    <select
                        id="map-weapon-select"
                        title="Active firearm used to grade cover penetration vs. concealment"
                        onchange="window.satyaUpdateMapTacticalControls()"
                        on:change=move |_| wasm_client::satya_update_map_tactical_controls()
                    >
                        <option value="hk416_carbine_556">"5.56mm HK416 (Pen 6)"</option>
                        <option value="sig_p320_9mm">"9mm P320 JHP (Pen 2)"</option>
                        <option value="mp7_pdw_57">"5.7mm MP7 AP (Pen 6)"</option>
                        <option value="benelli_m4_12ga">"12-Ga Benelli (Pen 4)"</option>
                        <option value="akm_762x39">"7.62×39 AKM (Pen 5)"</option>
                        <option value="scar_h_762x51">"7.62 NATO SCAR-H (Pen 7)"</option>
                        <option value="ai_atx_65cm">"6.5 CM AT-X (Pen 8)"</option>
                    </select>
                </label>
                <label class="inline-label" for="map-gait-select">
                    <span>"Gait"</span>
                    <select
                        id="map-gait-select"
                        onchange="window.satyaUpdateMapTacticalControls()"
                        on:change=move |_| wasm_client::satya_update_map_tactical_controls()
                    >
                        <option value="combat_glide">"Combat Glide (Walk)"</option>
                        <option value="prone_crawl">"Prone Low Crawl"</option>
                        <option value="stealth_creep">"Stealth Creep"</option>
                        <option value="bounding_run">"Bounding Run"</option>
                        <option value="all_out_sprint">"All-Out Sprint"</option>
                    </select>
                </label>
                <label class="inline-label" for="map-ticks-input">
                    <span>"Ticks"</span>
                    <input
                        type="number"
                        id="map-ticks-input"
                        class="map-ticks-input"
                        min="1"
                        max="40"
                        value="8"
                        onchange="window.satyaUpdateMapTacticalControls()"
                        on:change=move |_| wasm_client::satya_update_map_tactical_controls()
                    />
                </label>
                <label class="inline-label" for="map-click-mode">
                    <span>"Click"</span>
                    <select
                        id="map-click-mode"
                        onchange="window.satyaUpdateMapTacticalControls()"
                        on:change=move |_| wasm_client::satya_update_map_tactical_controls()
                    >
                        <option value="move_or_select">"Select / Move / Door"</option>
                        <option value="measure_los">"Measure Range & Cover"</option>
                        <option value="edit_terrain">"Paint Cover / Wall / Prop"</option>
                    </select>
                </label>
                <label
                    id="map-prop-brush-wrap"
                    class="inline-label"
                    for="map-prop-brush-select"
                    style="display: none;"
                >
                    <span>"Brush"</span>
                    <select
                        id="map-prop-brush-select"
                        title="Choose Tile/Wall cycle, Erase Prop, or a multi-cell prop to place in Paint mode"
                        onchange="window.satyaUpdateMapPropBrush()"
                        on:change=move |_| wasm_client::satya_update_map_prop_brush()
                    >
                        <option value="">"Wall / Door / Window Edge"</option>
                        <option value="__erase__">"Erase Prop"</option>
                        {wasm_client::PROP_BRUSH_GROUPS
                            .iter()
                            .map(|&(group_label, kinds)| {
                                view! {
                                    <optgroup label=group_label>
                                        {kinds
                                            .iter()
                                            .map(|kind| {
                                                view! { <option value=kind.as_str()>{kind.label()}</option> }
                                            })
                                            .collect_view()}
                                    </optgroup>
                                }
                            })
                            .collect_view()}
                    </select>
                </label>
                <button
                    type="button"
                    id="btn-map-prop-rotate"
                    class="btn btn-sm btn-secondary"
                    title="Rotate prop brush clockwise by +15° (full 360° orientation; hotkey R)"
                    style="display: none;"
                    onclick="window.satyaRotateMapPropBrush()"
                    on:click=move |_| wasm_client::satya_rotate_map_prop_brush()
                >
                    "Rot +15°"
                </button>
                <label
                    id="map-prop-rot-wrap"
                    class="inline-label"
                    for="map-prop-rot-deg"
                    style="display: none;"
                >
                    <span>"Deg"</span>
                    <input
                        type="number"
                        id="map-prop-rot-deg"
                        class="map-ticks-input"
                        min="0"
                        max="359"
                        step="5"
                        value="0"
                        title="Exact 0..359° prop orientation angle"
                        onchange="window.satyaSetMapPropRotationDeg()"
                        on:change=move |_| wasm_client::satya_set_map_prop_rotation_deg()
                    />
                </label>
                <button
                    type="button"
                    id="btn-map-snap-toggle"
                    class="btn btn-sm btn-secondary"
                    title="Toggle free continuous placement vs. 0.50m increment snapping"
                    onclick="window.satyaToggleMapGridSnap()"
                    on:click=move |_| wasm_client::satya_toggle_map_grid_snap()
                >
                    "Snap: Free"
                </button>
                <label class="inline-label" for="map-token-model-select">
                    <span>"Mini"</span>
                    <select
                        id="map-token-model-select"
                        onchange="window.satyaUpdateSelectedTokenVisual()"
                        on:change=move |_| wasm_client::satya_update_selected_token_visual()
                    >
                        {MODEL_RIG_OPTIONS
                            .iter()
                            .copied()
                            .map(|(val, short_label, _)| {
                                view! { <option value=val>{short_label}</option> }
                            })
                            .collect_view()}
                        <option value="custom_glb">"Custom .glb"</option>
                    </select>
                </label>
                <label class="inline-label" for="map-token-color">
                    <span>"Tint"</span>
                    <input
                        type="color"
                        id="map-token-color"
                        class="dice-color-input"
                        value="#38bdf8"
                        oninput="window.satyaUpdateSelectedTokenVisual()"
                        on:input=move |_| wasm_client::satya_update_selected_token_visual()
                    />
                </label>
                <button
                    type="button"
                    id="btn-map-apply-cover"
                    class="btn btn-sm btn-secondary"
                    title="Send evaluated Cover Δ_ext into the Task Resolution 2d20 drawer"
                    onclick="window.satyaApplyMapCoverToRoller()"
                    on:click=move |_| wasm_client::satya_apply_map_cover_to_roller()
                >
                    "Send Cover Δ_ext → 2d20"
                </button>
            </div>

            <div id="tabletop-map-viewport" class="tabletop-map-viewport">
                <div id="wgpu-map-canvas-wrap" class="wgpu-map-canvas-wrap">
                    <canvas
                        id="wgpu-map-canvas"
                        class="wgpu-map-canvas"
                        width="720"
                        height="420"
                        aria-label="3D/2D WebGPU Tactical Diorama Battlemap (1.0m Grid)"
                    ></canvas>
                </div>
                <div
                    id="tabletop-map-svg-fallback"
                    class="tabletop-map-svg-fallback"
                    style="display: none;"
                    inner_html=wasm_client::default_tactical_map_svg_html()
                ></div>
                <div
                    id="tabletop-map-placeholder"
                    class="tabletop-map-placeholder"
                    style="display: none;"
                >
                    "Awaiting tactical map generation (`generate_tactical_map`) or GM map update..."
                </div>
                <img
                    id="tabletop-map-img"
                    class="tabletop-map-img"
                    alt="Tactical Battlemap"
                    style="display: none;"
                />
                <div
                    id="map-hover-tooltip"
                    class="map-hover-tooltip"
                    role="status"
                    style="display: none;"
                ></div>
            </div>
            <div
                id="map-hover-status-bar"
                class="map-hover-status-bar"
                aria-live="polite"
                inner_html=wasm_client::default_map_hover_status_bar_html()
            ></div>
            <div
                id="tabletop-map-los-hud"
                class="map-los-hud"
                aria-live="polite"
                inner_html=wasm_client::default_tactical_map_hud_html()
            ></div>
            <div class="roster-bar">
                <span class="roster-title">"Miniatures (`1.0m` Grid · Drag: Orbit · Shift/Right-Drag: Pan · Wheel: Zoom · Click: Move/Select):"</span>
                <div
                    id="tabletop-token-list"
                    class="roster-chips"
                    inner_html=wasm_client::default_tactical_map_tokens_html()
                ></div>
            </div>
        </div>
    }
}

#[component]
fn NotebookDicePane() -> impl IntoView {
    view! {
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
                            let onclick_js = format!("window.satyaQuick3dDiceRoll('{notation}')");
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-sm btn-secondary"
                                    onclick=onclick_js
                                    on:click=move |_| {
                                        wasm_client::satya_quick_3d_dice_roll(notation.to_string())
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
                        {DICE_MATERIAL_OPTIONS
                            .iter()
                            .copied()
                            .map(|(val, label)| view! { <option value=val>{label}</option> })
                            .collect_view()}
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
                {DICE_CAMERA_SLIDERS
                    .iter()
                    .copied()
                    .map(|(input_id, readout_id, label, min, max, step, def_val, valuetext, init_ro)| {
                        view! {
                            <label class="inline-label dice-camera-slider-label" for=input_id>
                                <span>{label}</span>
                                <input
                                    type="range"
                                    id=input_id
                                    class="dice-opacity-slider"
                                    min=min
                                    max=max
                                    step=step
                                    value=def_val
                                    aria-valuetext=valuetext
                                    oninput="window.satyaUpdateDiceCamera()"
                                    on:input=move |_| wasm_client::satya_update_dice_camera()
                                />
                                <span id=readout_id class="mono">{init_ro}</span>
                            </label>
                        }
                    })
                    .collect_view()}
                <div class="dice-pan-controls">
                    <span class="subtitle">"Pan:"</span>
                    {DICE_PAN_BUTTONS
                        .iter()
                        .copied()
                        .map(|(btn_id, title, dx, dy, onclick_js, glyph)| {
                            view! {
                                <button
                                    type="button"
                                    id=btn_id
                                    class="btn btn-sm btn-secondary"
                                    title=title
                                    aria-label=title
                                    onclick=onclick_js
                                    on:click=move |_| wasm_client::satya_pan_dice_camera(dx, dy)
                                >
                                    {glyph}
                                </button>
                            }
                        })
                        .collect_view()}
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
    }
}

#[component]
fn NotebookModelAndCharacterPanes() -> impl IntoView {
    view! {
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
                    <label class="inline-label" for="model-preset-select">
                        <span>"3D Asset"</span>
                        <select
                            id="model-preset-select"
                            onchange="window.satyaUpdate3dModelView()"
                            on:change=move |_| wasm_client::satya_update_3d_model_view()
                        >
                            {MODEL_RIG_OPTIONS
                                .iter()
                                .copied()
                                .map(|(val, _, full_label)| {
                                    view! { <option value=val>{full_label}</option> }
                                })
                                .collect_view()}
                            <option value="sector4_airlock">"Sector 4 Orbital Airlock"</option>
                            <option value="custom_glb">"Custom Imported (.glb)"</option>
                        </select>
                    </label>
                    <label class="inline-label" for="model-render-mode">
                        <span>"Shader Mode"</span>
                        <select
                            id="model-render-mode"
                            onchange="window.satyaUpdate3dModelView()"
                            on:change=move |_| wasm_client::satya_update_3d_model_view()
                        >
                            <option value="wireframe_pbr">"Wireframe + PBR"</option>
                            <option value="pbr">"Clean PBR Armor"</option>
                            <option value="hit_zones">"Anatomical Hit Zones"</option>
                            <option value="thermal">"Thermal / IR Sensor"</option>
                        </select>
                    </label>
                    <label class="inline-label" for="model-tint-color">
                        <span>"Armor Tint"</span>
                        <input
                            type="color"
                            id="model-tint-color"
                            class="dice-color-input"
                            value="#0ea5e9"
                            oninput="window.satyaUpdate3dModelView()"
                            on:input=move |_| wasm_client::satya_update_3d_model_view()
                        />
                    </label>
                    <button
                        type="button"
                        id="btn-reset-model-camera"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaResetModelCamera()"
                        on:click=move |_| wasm_client::satya_reset_model_camera()
                    >
                        "Reset View"
                    </button>
                    <label class="btn btn-sm btn-secondary model-glb-upload-label" for="model-glb-file-input">
                        "Load .glb"
                    </label>
                    <input
                        type="file"
                        id="model-glb-file-input"
                        accept=".glb,model/gltf-binary"
                        style="display: none;"
                        onchange="window.satyaImportCustomGlbModel()"
                        on:change=move |_| wasm_client::satya_import_custom_glb_model()
                    />
                    <button
                        type="button"
                        id="btn-export-glb-model"
                        class="btn btn-sm btn-accent"
                        title="Download spec-compliant glTF 2.0 binary (.glb) to edit in Blender"
                        onclick="window.satyaExportCurrentGlbModel()"
                        on:click=move |_| wasm_client::satya_export_current_glb_model()
                    >
                        "Download .glb"
                    </button>
                </div>
                <div class="btn-row-compact">
                    <span id="model-telemetry-pill" class="pill pill-iggy">"Mesh: Valkyrie Exo-Rig · 14.2k tris"</span>
                    <span id="model-wgpu-status-pill" class="pill pill-muted">"WebGPU glTF 2.0 (.glb) Turntable"</span>
                </div>
            </div>
            <div id="wgpu-model-canvas-wrap" class="wgpu-model-canvas-wrap">
                <canvas
                    id="wgpu-model-canvas"
                    class="wgpu-model-canvas"
                    width="640"
                    height="340"
                    aria-label="Rotatable 3D WebGPU glTF 2.0 (.glb) Character Model Stage"
                ></canvas>
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
                <div class="model-controls-row">
                    <label class="inline-label" for="char-sheet-preset-select">
                        <span>"Operator Dossier"</span>
                        <select
                            id="char-sheet-preset-select"
                            onchange="window.satyaCharSheetLoadPreset(this.value)"
                            on:change=move |ev| {
                                let val = ev
                                    .target()
                                    .and_then(|t| {
                                        wasm_bindgen::JsCast::dyn_into::<
                                            web_sys::HtmlSelectElement,
                                        >(t)
                                        .ok()
                                    })
                                    .map(|s| s.value())
                                    .unwrap_or_default();
                                wasm_client::satya_char_sheet_load_preset(val);
                            }
                        >
                            <option value="sara_valkyrie">"Sara \"Valkyrie\" Lin (Med-Assault Operator)"</option>
                            <option value="mateo_vance">"Mateo Vance (Syndicate Enforcer)"</option>
                            <option value="priya_nair">"Dr. Priya Nair (AI & Robotics Lead)"</option>
                            <option value="amara_okafor">"Amara Okafor (Frontier Recon Ranger)"</option>
                            <option value="custom_wizard">"Custom Character (From Character Wizard)"</option>
                        </select>
                    </label>
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaWizardApplyToSheet()"
                        on:click=move |_| wasm_client::satya_wizard_apply_to_sheet()
                    >
                        "Sync from Character Wizard"
                    </button>
                </div>
                <span id="char-sheet-status-pill" class="pill pill-active">
                    "Sara \"Valkyrie\" Lin (Age 26) · Combat Ready · OODA 14 · D_cog 14 · Pool 26/27 · Fatigue 1 · Shock 0 · Δ_int +1"
                </span>
            </div>
            <div
                id="char-sheet-stage-container"
                class="char-sheet-stage-container"
                inner_html=wasm_client::default_character_sheet_html()
            ></div>
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
                    "Step 1/6 · OODA 10 · D_cog 10 · POW 10.0"
                </span>
            </div>
            <div id="wizard-feedback" class="feedback-box" aria-live="polite"></div>
            <div
                id="wizard-stage-container"
                class="wizard-stage-container"
                inner_html=wasm_client::default_character_wizard_html()
            ></div>
        </div>
    }
}
