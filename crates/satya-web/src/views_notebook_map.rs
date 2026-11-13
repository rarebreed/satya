//! Tactical Map Pane component ([`NotebookMapPane`]) extracted from [`crate::views_notebook`]
//! so each SSR view file stays concise while rendering `#notebook-pane-maps`.

use crate::views_notebook::MODEL_RIG_OPTIONS;
use crate::wasm_client;
use leptos::prelude::*;

/// Interactive Tactical Map pane (`#notebook-pane-maps`) hosting WebGPU 3D/2D diorama,
/// illustrated 2D SVG battlemap, zoom/pan controls, boundary continuation buttons,
/// and the docked top-right Inspector pop-up pane (`#map-hover-tooltip`).
#[component]
pub fn NotebookMapPane() -> impl IntoView {
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
                        title="Pin the docked Object & Ground Inspector pane (or hold Alt / press I while hovering)"
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
                        id="btn-map-zoom-out"
                        class="btn btn-sm btn-secondary"
                        title="Zoom out tactical map"
                        onclick="window.satyaZoomMapOut()"
                        on:click=move |_| wasm_client::satya_zoom_map_out()
                    >
                        "−"
                    </button>
                    <span
                        id="map-zoom-readout"
                        class="pill pill-muted mono map-zoom-pill"
                        title="Current tactical map zoom scale"
                    >
                        "100%"
                    </span>
                    <button
                        type="button"
                        id="btn-map-zoom-in"
                        class="btn btn-sm btn-secondary"
                        title="Zoom in tactical map"
                        onclick="window.satyaZoomMapIn()"
                        on:click=move |_| wasm_client::satya_zoom_map_in()
                    >
                        "+"
                    </button>
                    <span
                        id="map-hover-coords-pill"
                        class="pill pill-muted mono map-coords-pill"
                        title="Cursor metric coordinates"
                    >
                        "Hover: --"
                    </span>
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
                    style="display: none;"
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
                <button
                    type="button"
                    id="btn-map-continue-north"
                    class="btn btn-sm btn-accent map-edge-continue-btn map-edge-north"
                    title="Generate a seamless procedural sector north of this map edge"
                    style="display: none;"
                    onclick="window.satyaContinueMapDir('NORTH')"
                    on:click=move |_| wasm_client::satya_continue_map_dir("NORTH".to_string())
                >
                    "▲ Continue North"
                </button>
                <button
                    type="button"
                    id="btn-map-continue-south"
                    class="btn btn-sm btn-accent map-edge-continue-btn map-edge-south"
                    title="Generate a seamless procedural sector south of this map edge"
                    style="display: none;"
                    onclick="window.satyaContinueMapDir('SOUTH')"
                    on:click=move |_| wasm_client::satya_continue_map_dir("SOUTH".to_string())
                >
                    "▼ Continue South"
                </button>
                <button
                    type="button"
                    id="btn-map-continue-west"
                    class="btn btn-sm btn-accent map-edge-continue-btn map-edge-west"
                    title="Generate a seamless procedural sector west of this map edge"
                    style="display: none;"
                    onclick="window.satyaContinueMapDir('WEST')"
                    on:click=move |_| wasm_client::satya_continue_map_dir("WEST".to_string())
                >
                    "◀ Continue West"
                </button>
                <button
                    type="button"
                    id="btn-map-continue-east"
                    class="btn btn-sm btn-accent map-edge-continue-btn map-edge-east"
                    title="Generate a seamless procedural sector east of this map edge"
                    style="display: none;"
                    onclick="window.satyaContinueMapDir('EAST')"
                    on:click=move |_| wasm_client::satya_continue_map_dir("EAST".to_string())
                >
                    "Continue East ▶"
                </button>
                <div
                    id="map-hover-tooltip"
                    class="map-hover-tooltip"
                    role="status"
                    style="display: none;"
                ></div>
            </div>
            <div
                id="tabletop-map-los-hud"
                class="map-los-hud"
                aria-live="polite"
                inner_html=wasm_client::default_tactical_map_hud_html()
            ></div>
            <div class="roster-bar">
                <span class="roster-title">"Miniatures (1.0m Grid · Wheel / +/−: Zoom · Drag: Pan [2D] / Orbit [3D] · Click: Select/Move):"</span>
                <div
                    id="tabletop-token-list"
                    class="roster-chips"
                    inner_html=wasm_client::default_tactical_map_tokens_html()
                ></div>
            </div>
        </div>
    }
}
