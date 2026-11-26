//! Left 10-Pane Tabbed Notebook UI (`Maps`, `3D Dice`, `3D Model`, `Character Sheet/Status`,
//! `Character Wizard`, `NPC List`, `Combat Tracker`, `Rules Book`, `GM Docs`, and `Apache Iggy`)
//! handlers and HTML formatters.

use crate::wasm_client::chat_and_ttrpg::{append_system_notice, satya_update_cascade_preview};
pub use crate::wasm_client::combat_tracker::*;
use crate::wasm_client::dom::{
    document, escape_html, escape_js_single_quoted_attr, fetch_json, get_element, input_value,
    roll_browser_die_i64, select_value, set_class, set_display, set_html, set_input_value,
    set_select_value, set_text,
};
pub use crate::wasm_client::gm_docs::*;
use crate::wasm_client::state::{with_state, with_state_mut};
use crate::wasm_client::telemetry::satya_refresh_metrics;
use crate::wasm_client::webrtc::{send_ws_packet, sync_workspace_drawers_dom};
use satya_schema::{
    CampaignLoreCatalogResponse, CampaignLoreChapterDocument, ChannelContext, ClientWsPacket,
    GmMapUpdateRequest, MessageVisibility, Participant, SessionRole, WgpuDiceSpec,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Canonical list of the 10 Tabbed Notebook tab slugs and their `(button_id, pane_id)` pairs.
pub const NOTEBOOK_TABS: &[(&str, &str, &str)] = &[
    ("maps", "tab-btn-maps", "notebook-pane-maps"),
    ("dice", "tab-btn-dice", "notebook-pane-dice"),
    ("model", "tab-btn-model", "notebook-pane-model"),
    ("character", "tab-btn-character", "notebook-pane-character"),
    ("wizard", "tab-btn-wizard", "notebook-pane-wizard"),
    ("npcs", "tab-btn-npcs", "notebook-pane-npcs"),
    ("combat", "tab-btn-combat", "notebook-pane-combat"),
    ("rules", "tab-btn-rules", "notebook-pane-rules"),
    ("docs", "tab-btn-docs", "notebook-pane-docs"),
    ("iggy", "tab-btn-iggy", "notebook-pane-iggy"),
];

/// Canonical list of the 10 Tabbed Notebook tabs with `(slug, button_id, pane_id, label)` for SSR and Wasm.
pub const NOTEBOOK_TAB_SPECS: &[(&str, &str, &str, &str)] = &[
    ("maps", "tab-btn-maps", "notebook-pane-maps", "Maps"),
    ("dice", "tab-btn-dice", "notebook-pane-dice", "3D Dice"),
    ("model", "tab-btn-model", "notebook-pane-model", "3D Model"),
    (
        "character",
        "tab-btn-character",
        "notebook-pane-character",
        "Character Sheet/Status",
    ),
    (
        "wizard",
        "tab-btn-wizard",
        "notebook-pane-wizard",
        "Character Wizard",
    ),
    ("npcs", "tab-btn-npcs", "notebook-pane-npcs", "NPC List"),
    (
        "combat",
        "tab-btn-combat",
        "notebook-pane-combat",
        "Combat Tracker",
    ),
    (
        "rules",
        "tab-btn-rules",
        "notebook-pane-rules",
        "Rules & Lore",
    ),
    (
        "docs",
        "tab-btn-docs",
        "notebook-pane-docs",
        "GM Docs (GM_ONLY)",
    ),
    (
        "iggy",
        "tab-btn-iggy",
        "notebook-pane-iggy",
        "Apache Iggy, Topics & PyLance",
    ),
];

/// Returns a crisp `14×14` `currentColor` inline SVG icon for each Notebook tab slug.
#[must_use]
pub fn notebook_tab_icon_svg(slug: &str) -> &'static str {
    match slug {
        "maps" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="1 6 1 22 8 18 16 22 23 18 23 2 16 6 8 2 1 6"/><line x1="8" y1="2" x2="8" y2="18"/><line x1="16" y1="6" x2="16" y2="22"/></svg>"#
        }
        "dice" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 21 7 21 17 12 22 3 17 3 7"/><polyline points="12 2 12 8 21 7"/><polyline points="12 8 3 7"/><polyline points="12 8 7 16 17 16 12 8"/><line x1="7" y1="16" x2="3" y2="17"/><line x1="17" y1="16" x2="21" y2="17"/><line x1="7" y1="16" x2="12" y2="22"/><line x1="17" y1="16" x2="12" y2="22"/></svg>"#
        }
        "model" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>"#
        }
        "character" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="2.2"/><path d="M5.5 16.5a3.5 3.5 0 0 1 7 0"/><line x1="14.5" y1="9" x2="18.5" y2="9"/><line x1="14.5" y1="13" x2="18.5" y2="13"/></svg>"#
        }
        "wizard" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 4-1 2 2 1-2 1 1 2-2-1-1 2-1-2-2 1 1-2-2-1 2-1-1-2 2 1 1-2 1 2z"/><path d="m3 21 9-9"/></svg>"#
        }
        "npcs" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="3.5"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a3.5 3.5 0 0 1 0 6.75"/></svg>"#
        }
        "combat" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="14.5 17.5 3 6 3 3 6 3 17.5 14.5"/><line x1="13" y1="19" x2="19" y2="13"/><line x1="16" y1="16" x2="20" y2="20"/><line x1="19" y1="21" x2="21" y2="19"/><polyline points="14.5 6.5 18 3 21 3 21 6 17.5 9.5"/><line x1="5" y1="14" x2="9" y2="18"/><line x1="7" y1="17" x2="4" y2="20"/><line x1="3" y1="19" x2="5" y2="21"/></svg>"#
        }
        "rules" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>"#
        }
        "docs" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>"#
        }
        "iggy" => {
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/><path d="M3 12c0 1.66 4 3 9 3s9-1.34 9-3"/></svg>"#
        }
        _ => "",
    }
}

/// Returns `true` when the active session role holds human GM / Solo Co-GM / Host authority.
#[must_use]
pub const fn is_session_gm_role(role: Option<SessionRole>) -> bool {
    matches!(role, Some(r) if r.is_human_gm_or_co_gm())
}

/// Shows or hides the `Apache Iggy, Topics & PyLance` tab (`#tab-btn-iggy`) based on session GM authority.
pub(crate) fn sync_iggy_tab_visibility(is_gm: bool) {
    set_display("tab-btn-iggy", if is_gm { "inline-flex" } else { "none" });
    if !is_gm && with_state(|st| st.active_notebook_tab == "iggy") {
        satya_select_notebook_tab("maps".to_string());
    }
}

/// Switches the active view inside the 10-pane Left Tabbed Notebook (`#tabletop-canvas-card`) between
/// `"maps"`, `"dice"`, `"model"`, `"character"`, `"wizard"`, `"npcs"`, `"combat"`, `"rules"`, `"docs"`, and `"iggy"`.
#[wasm_bindgen]
pub fn satya_select_notebook_tab(tab_id: String) {
    let mut target = if NOTEBOOK_TABS.iter().any(|(slug, _, _)| *slug == tab_id) {
        tab_id
    } else {
        "maps".to_string()
    };
    if target == "iggy"
        && !with_state(|st| {
            st.session_id.is_some()
                && is_session_gm_role(st.self_participant.as_ref().map(|p| p.role))
        })
    {
        target = "maps".to_string();
    }

    with_state_mut(|st| {
        st.active_notebook_tab = target.clone();
    });

    for (slug, btn_id, pane_id) in NOTEBOOK_TABS {
        let is_active = *slug == target;
        set_class(
            btn_id,
            if is_active {
                "notebook-tab active"
            } else {
                "notebook-tab"
            },
        );
        if let Some(btn) = get_element(btn_id) {
            let _ = btn.set_attribute("aria-selected", if is_active { "true" } else { "false" });
        }
        set_display(pane_id, if is_active { "flex" } else { "none" });
        set_class(
            pane_id,
            if is_active {
                "notebook-pane active"
            } else {
                "notebook-pane"
            },
        );
    }

    if target == "maps" {
        crate::wasm_client::map_3d::refresh_map_dom_and_wgpu();
    } else if target == "model" {
        crate::wasm_client::model_3d_wgpu::sync_3d_model_viewer_from_dom();
    } else if target == "iggy" {
        satya_refresh_metrics();
    } else if target == "combat" {
        refresh_combat_tracker();
        crate::wasm_client::ttrpg_drawer_triad::satya_update_drawer_triad_preview();
    } else if target == "docs" {
        satya_refresh_gm_documents();
    } else if target == "npcs" {
        crate::wasm_client::npcs::satya_refresh_npcs_tab();
    } else if target == "character" {
        crate::wasm_client::character_sheet::render_character_sheet();
    } else if target == "wizard" {
        crate::wasm_client::wizard::render_character_wizard();
    } else if target == "dice" {
        crate::wasm_client::dice_3d::sync_dice_material_from_dom_and_render();
        crate::wasm_client::dice_3d::sync_dice_camera_from_dom_and_render();
    }
}

/// Updates the 3D WebGPU dice material (base color, numeral ink color, and opacity/translucency)
/// when the color pickers or opacity range slider change.
#[wasm_bindgen]
pub fn satya_update_dice_material() {
    crate::wasm_client::dice_3d::sync_dice_material_from_dom_and_render();
}

/// Applies a preset 3D dice material (`"translucent_sapphire"`, `"amber_resin"`, `"emerald_glass"`,
/// `"crimson_ruby"`, `"amethyst_void"`, or `"obsidian_opaque"`) and updates the WebGPU canvas.
#[wasm_bindgen]
pub fn satya_select_dice_material_preset() {
    crate::wasm_client::dice_3d::apply_dice_material_preset_from_dom();
}

/// Updates the 3D WebGPU dice camera viewing angle (`#dice-camera-yaw`, `#dice-camera-pitch`) and
/// zoom (`#dice-camera-zoom`) from the toolbar controls and immediately re-renders the canvas.
#[wasm_bindgen]
pub fn satya_update_dice_camera() {
    crate::wasm_client::dice_3d::sync_dice_camera_from_dom_and_render();
}

/// Pans the 3D WebGPU dice tray camera across the floor plane by `(dx, dz)` and re-renders.
#[wasm_bindgen]
pub fn satya_pan_dice_camera(dx: f32, dz: f32) {
    crate::wasm_client::dice_3d::pan_dice_camera_and_render(dx, dz);
}

/// Resets the 3D WebGPU dice tray camera orbit angle, pan offset, and zoom factor to defaults.
#[wasm_bindgen]
pub fn satya_reset_dice_camera() {
    crate::wasm_client::dice_3d::reset_dice_camera_and_render();
}

/// Triggers a quick 3D dice roll from the `3D Dice` notebook tab and updates the 3D dice tray readout.
#[wasm_bindgen]
pub fn satya_quick_3d_dice_roll(notation: String) {
    if with_state(|st| {
        st.session_id.is_some()
            && st
                .self_participant
                .as_ref()
                .is_some_and(|p| p.role == SessionRole::Observer)
    }) {
        append_system_notice(
            "Observers are in read-only spectator mode and cannot broadcast dice rolls.",
        );
        return;
    }
    let cmd_str = format!("/roll {notation}");
    let Ok(Some(crate::chat_cmd::ChatCommand::Roll { expr_raw, pipeline })) =
        crate::chat_cmd::parse_chat_command(&cmd_str)
    else {
        return;
    };
    let Ok(outcome) = pipeline.evaluate_with_roller(&expr_raw, roll_browser_die_i64) else {
        return;
    };

    let actor = with_state(|st| st.self_participant.clone()).unwrap_or_else(|| {
        Participant::human(
            "u-local",
            "local_operator",
            "Local Operator",
            SessionRole::Player,
        )
    });
    let outcomes: Vec<i32> = outcome
        .initial_dice
        .iter()
        .map(|d| d.value as i32)
        .collect();
    let seed = (js_sys::Math::random() * 1_000_000.0).floor() as i64;
    let spec = WgpuDiceSpec {
        dice_notation: notation.clone(),
        physics_seed: seed,
        outcomes,
        visibility: MessageVisibility::Public,
    };
    render_wgpu_dice_roll(&actor, &spec);

    let has_ws = with_state(|st| st.ws.is_some());
    if has_ws {
        send_ws_packet(&ClientWsPacket::SendChat {
            visibility: MessageVisibility::Public,
            recipient_user_ids: Vec::new(),
            channel_context: ChannelContext::OutOfCharacter,
            body: outcome.summary,
            composition_time_ms: 120,
        });
    } else {
        append_system_notice(&outcome.summary);
    }
}

/// Default HTML markup for the `3D Model` notebook tab stage (`Valkyrie Exo-Rig` in `Wireframe + PBR` mode).
pub fn default_3d_model_stage_html() -> String {
    format_3d_model_stage_html("valkyrie_rig", "wireframe_pbr").0
}

/// Formats `(stage_html, telemetry_pill_text)` for a selected 3D asset preset and shader mode in the `3D Model` notebook tab.
pub fn format_3d_model_stage_html(preset: &str, render_mode: &str) -> (String, String) {
    let trimmed = preset.trim();
    let (title, scale, hit_zones) = if trimmed == "custom_glb" {
        (
            "Custom glTF 2.0 (.glb) Asset",
            "1.0m Metric Scale",
            "Imported glTF 2.0 Submeshes & Anatomical Height-Band Hit Zones",
        )
    } else {
        crate::wasm_client::glb_model::preset_model_metadata(trimmed)
    };
    let tris = match trimmed {
        "recon_mk4" => "11.8k tris",
        "secops_sentry" => "18.4k tris",
        "operative_light" => "9.6k tris",
        "sector4_airlock" => "26.0k tris",
        "custom_glb" => "User Mesh",
        _ => "14.2k tris",
    };

    let stroke_color = match render_mode {
        "hit_zones" => "#f43f5e",
        "thermal" => "#f59e0b",
        "cel" => "#34d399",
        "cel_graphic_novel" => "#fbbf24",
        _ => "#38bdf8",
    };
    let mode_badge = match render_mode {
        "pbr" => "PBR-SHADED",
        "cel" => "CEL-SHADED",
        "cel_graphic_novel" => "GRAPHIC-NOVEL",
        "hit_zones" => "HIT-ZONES",
        "thermal" => "THERMAL-IR",
        _ => "WIREFRAME+PBR",
    };

    let pill = format!("Mesh: {title} · {tris}");
    let safe_title = escape_html(title);
    let safe_scale = escape_html(scale);
    let safe_zones = escape_html(hit_zones);
    let html = format!(
        r##"<div class="model-stage-inner">
            <svg viewBox="0 0 360 210" class="model-wireframe-svg" role="img" aria-label="{safe_title}">
                <defs>
                    <radialGradient id="modelGlow" cx="50%" cy="50%" r="50%">
                        <stop offset="0%" stop-color="{stroke_color}" stop-opacity="0.22"/>
                        <stop offset="100%" stop-color="#090d16" stop-opacity="0.0"/>
                    </radialGradient>
                </defs>
                <rect width="360" height="210" fill="#090d16"/>
                <circle cx="180" cy="105" r="85" fill="url(#modelGlow)"/>
                <ellipse cx="180" cy="175" rx="95" ry="18" fill="none" stroke="#2a354b" stroke-width="1.5" stroke-dasharray="4 4"/>
                <polygon points="180,24 222,62 208,136 180,166 152,136 138,62" fill="none" stroke="{stroke_color}" stroke-width="1.8"/>
                <line x1="180" y1="24" x2="180" y2="166" stroke="{stroke_color}" stroke-width="1.2" stroke-dasharray="3 3"/>
                <line x1="138" y1="62" x2="222" y2="62" stroke="{stroke_color}" stroke-width="1.2"/>
                <line x1="152" y1="136" x2="208" y2="136" stroke="{stroke_color}" stroke-width="1.2"/>
                <circle cx="180" cy="46" r="10" fill="none" stroke="#34d399" stroke-width="1.5"/>
                <text x="14" y="22" fill="#94a3b8" font-family="monospace" font-size="11">{mode_badge} // {tris}</text>
                <text x="14" y="196" fill="#e2e8f0" font-family="monospace" font-size="11">{safe_title} ({safe_scale})</text>
            </svg>
            <div class="model-hit-zones">
                <span class="roster-title">Hit Locations &amp; Subsystem Mesh:</span>
                <span class="mono">{safe_zones}</span>
            </div>
        </div>"##,
    );
    (html, pill)
}

/// Updates the `3D Model` notebook tab stage when `#model-preset-select`, `#model-render-mode`, or `#model-tint-color` changes.
#[wasm_bindgen]
pub fn satya_update_3d_model_view() {
    let preset = select_value("model-preset-select");
    let mode = select_value("model-render-mode");
    let (html, pill) = format_3d_model_stage_html(&preset, &mode);
    set_text("model-telemetry-pill", &pill);
    if let Some(stage) = get_element("model-preview-stage") {
        stage.set_inner_html(&html);
    }
    crate::wasm_client::model_3d_wgpu::sync_3d_model_viewer_from_dom();
}

/// Populates the Center-Bottom Die Roller (`#ttrpg-drawer`) with a skill preset clicked in the
/// `Character Sheet/Status` notebook tab and recalculates the exact Four-Factor Cascade preview.
#[wasm_bindgen]
pub fn satya_load_skill_preset(skill_name: String, skill_level: i32, control_c: i32) {
    let was_collapsed = with_state_mut(|st| {
        let prev = st.ttrpg_collapsed;
        st.ttrpg_collapsed = false;
        prev
    });
    if was_collapsed {
        sync_workspace_drawers_dom();
    }
    set_input_value("ttrpg-skill", &skill_name);
    set_input_value("ttrpg-skill-level", &skill_level.clamp(1, 10).to_string());
    set_input_value("ttrpg-control-c", &control_c.clamp(1, 20).to_string());
    set_text("char-stat-control", &control_c.clamp(1, 20).to_string());
    satya_update_cascade_preview();
}

/// Formats an HTML snippet of tactical token chips (`label (x, y)`) for `#tabletop-token-list`.
pub fn format_tactical_tokens_html(map: &GmMapUpdateRequest) -> String {
    if map.tokens.is_empty() {
        return r#"<span class="pill pill-muted">No tokens on map</span>"#.to_string();
    }
    map.tokens
        .iter()
        .map(|tok| {
            let vis_suffix = if tok.visibility != MessageVisibility::Public {
                format!(" · {:?}", tok.visibility)
            } else {
                String::new()
            };
            format!(
                r#"<span class="pill pill-active">{} ({}, {}){}</span>"#,
                escape_html(&tok.label),
                tok.x,
                tok.y,
                vis_suffix
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Renders an incoming [`GmMapUpdateRequest`] into the `#tabletop-canvas-card` 3D/2D WebGPU diorama, SVG fallback, and token bar.
pub fn render_map_updated(map: &GmMapUpdateRequest) {
    crate::wasm_client::map_3d::apply_incoming_map_update(map);
}

/// Formats an HTML snippet of 3D physics dice cubes and roll notation for `#wgpu-dice-readout`.
pub fn format_wgpu_dice_roll_html(actor: &Participant, spec: &WgpuDiceSpec) -> String {
    let cubes: String = spec
        .outcomes
        .iter()
        .map(|val| format!(r#"<span class="dice-cube">{val}</span>"#))
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<span><strong>{}</strong> rolled <span class="mono">{}</span>:</span> {cubes}"#,
        escape_html(&actor.display_name),
        escape_html(&spec.dice_notation),
    )
}

/// Renders an incoming [`WgpuDiceSpec`] into `#wgpu-dice-readout` and spawns the 3D WebGPU trajectory on `#wgpu-dice-canvas`.
pub fn render_wgpu_dice_roll(actor: &Participant, spec: &WgpuDiceSpec) {
    crate::wasm_client::dice_3d::spawn_wgpu_dice_scene(spec);
    let html = format_wgpu_dice_roll_html(actor, spec);
    if let Some(readout) = get_element("wgpu-dice-readout") {
        readout.set_inner_html(&html);
    }
    if let (Some(doc), Some(hist)) = (document(), get_element("wgpu-dice-history-list"))
        && let Ok(row) = doc.create_element("div")
    {
        row.set_class_name("dice-history-item");
        row.set_inner_html(&html);
        let _ = hist.prepend_with_node_1(&row);
    }
}

/// Returns the default rendered HTML for the `Rules Book` notebook tab (`tasks` chapter).
pub fn default_rules_book_chapter_html() -> String {
    format_rules_chapter_html("tasks").0
}

/// Formats `(rendered_html, pill_text)` for a canonical Satya TTRPG Rules Book chapter.
pub fn format_rules_chapter_html(chapter_id: &str) -> (String, String) {
    let doc = satya_rules_book::get_rules_chapter(chapter_id)
        .or_else(|| satya_rules_book::get_rules_chapter("tasks"));
    let Some(doc) = doc else {
        return (
            r#"<div class="rules-book-empty">Rules chapter not found.</div>"#.to_string(),
            "Chapter Not Found".to_string(),
        );
    };
    let pill = format!(
        "{} · {} ({} sections)",
        doc.summary.part,
        doc.summary.relative_path,
        doc.summary.headings.len()
    );
    (doc.rendered_html, pill)
}

fn scroll_and_highlight_anchor(anchor: &str) {
    let trimmed = anchor.trim();
    if trimmed.is_empty() {
        return;
    }
    if let Some(el) = get_element(trimmed) {
        el.scroll_into_view();
        let prev = el.class_name();
        if !prev.split_whitespace().any(|c| c == "archivist-jump-highlight") {
            let next = if prev.trim().is_empty() {
                "archivist-jump-highlight".to_string()
            } else {
                format!("{prev} archivist-jump-highlight")
            };
            el.set_class_name(&next);
        }
    }
}

fn render_rules_chapters_dropdown(selected: &str) {
    let html: String = satya_rules_book::list_rules_chapters()
        .into_iter()
        .enumerate()
        .map(|(idx, ch)| {
            let sel = if ch.chapter_id == selected {
                " selected"
            } else {
                ""
            };
            format!(
                r#"<option value="{}"{sel}>{}. {}</option>"#,
                escape_html(&ch.chapter_id),
                idx + 1,
                escape_html(&ch.title),
            )
        })
        .collect();
    set_html("rules-chapter-select", &html);
}

fn render_campaign_lore_chapters_dropdown(selected: &str) {
    let chapters = with_state(|st| st.campaign_lore_catalog.clone());
    if chapters.is_empty() {
        set_html(
            "rules-chapter-select",
            r#"<option value="">No Campaign Lore chapters loaded for current clearance</option>"#,
        );
        return;
    }
    let eff_selected = if chapters.iter().any(|c| c.relative_path == selected) {
        selected.to_string()
    } else {
        chapters[0].relative_path.clone()
    };
    let html: String = chapters
        .iter()
        .enumerate()
        .map(|(idx, ch)| {
            let sel = if ch.relative_path == eff_selected {
                " selected"
            } else {
                ""
            };
            let vis_tag = if ch.visibility == "PUBLIC" {
                ""
            } else {
                " [GM]"
            };
            format!(
                r#"<option value="{}"{sel}>{}. {}{}</option>"#,
                escape_html(&ch.relative_path),
                idx + 1,
                escape_html(&ch.title),
                vis_tag,
            )
        })
        .collect();
    set_html("rules-chapter-select", &html);
    set_select_value("rules-chapter-select", &eff_selected);
}

/// Fetches the role- and Spoiler-Shield-filtered Campaign Lore catalog and updates `#rules-chapter-select`
/// if Campaign Lore mode is active.
pub(crate) fn refresh_campaign_lore_catalog() {
    let ctx = with_state(|st| {
        (
            st.session_id.clone(),
            st.auth_token.clone(),
            st.solo_spoiler_shield,
        )
    });
    spawn_local(async move {
        let (url, tok_opt) = match ctx {
            (Some(sid), Some(tok), shielded) => {
                let q = if shielded { "?spoiler_shield=true" } else { "" };
                (format!("/api/sessions/{sid}/lore{q}"), Some(tok))
            }
            _ => ("/api/campaign/lore".to_string(), None),
        };
        if let Ok(resp) =
            fetch_json::<(), CampaignLoreCatalogResponse>("GET", &url, tok_opt.as_deref(), None)
                .await
        {
            let active_mode = with_state_mut(|st| {
                st.campaign_pack_name = resp.pack_name.clone();
                st.campaign_lore_catalog = resp.chapters;
                st.reader_source_mode.clone()
            });
            if active_mode == "CAMPAIGN_LORE" {
                render_campaign_lore_chapters_dropdown("");
            }
        }
    });
}

fn load_campaign_lore_chapter_and_scroll(rel_path: String, anchor: String) {
    let clean_path = rel_path.split('#').next().unwrap_or(&rel_path).trim().to_string();
    if clean_path.is_empty() {
        return;
    }
    let ctx = with_state(|st| {
        (
            st.session_id.clone(),
            st.auth_token.clone(),
            st.solo_spoiler_shield,
        )
    });
    spawn_local(async move {
        let encoded: String = js_sys::encode_uri_component(&clean_path).into();
        let (url, tok_opt) = match ctx {
            (Some(sid), Some(tok), shielded) => {
                let shield_q = if shielded { "&spoiler_shield=true" } else { "" };
                (
                    format!("/api/sessions/{sid}/lore/chapter?path={encoded}{shield_q}"),
                    Some(tok),
                )
            }
            _ => (format!("/api/campaign/lore/chapter?path={encoded}"), None),
        };
        match fetch_json::<(), CampaignLoreChapterDocument>("GET", &url, tok_opt.as_deref(), None)
            .await
        {
            Ok(doc) => {
                set_select_value("rules-chapter-select", &doc.summary.relative_path);
                let pill = format!(
                    "{} · {} ({} sections · {})",
                    doc.summary.part,
                    doc.summary.relative_path,
                    doc.summary.headings.len(),
                    doc.summary.visibility
                );
                set_text("rules-chapter-pill", &pill);
                if let Some(vp) = get_element("rules-book-viewport") {
                    vp.set_inner_html(&doc.rendered_html);
                    if anchor.trim().is_empty() {
                        vp.set_scroll_top(0);
                    }
                }
                scroll_and_highlight_anchor(&anchor);
            }
            Err(err) => {
                set_text("rules-chapter-pill", &format!("Lore restricted: {err}"));
            }
        }
    });
}

/// Switches the active source inside `#notebook-pane-rules` between `"RULES"` (24-Ch. TTRPG Rules Book)
/// and `"CAMPAIGN_LORE"` (Awakened Campaign Lore chapters).
#[wasm_bindgen]
pub fn satya_select_reader_source(source: String) {
    let raw = if source.trim().is_empty() {
        select_value("rules-lore-source-select")
    } else {
        set_select_value("rules-lore-source-select", &source);
        source
    };
    let mode = if raw.trim().eq_ignore_ascii_case("CAMPAIGN_LORE") {
        "CAMPAIGN_LORE".to_string()
    } else {
        "RULES".to_string()
    };
    with_state_mut(|st| {
        st.reader_source_mode = mode.clone();
    });
    if mode == "CAMPAIGN_LORE" {
        let first_path = with_state(|st| {
            st.campaign_lore_catalog
                .first()
                .map(|c| c.relative_path.clone())
        });
        render_campaign_lore_chapters_dropdown(first_path.as_deref().unwrap_or(""));
        if let Some(path) = first_path {
            load_campaign_lore_chapter_and_scroll(path, String::new());
        } else {
            refresh_campaign_lore_catalog();
            load_campaign_lore_chapter_and_scroll(
                "docs/campaign/03-minds-and-factions/major_characters.md".to_string(),
                String::new(),
            );
        }
    } else {
        render_rules_chapters_dropdown("tasks");
        satya_select_rules_chapter("tasks".to_string());
    }
}

/// Opens a specific canonical Rules Book chapter in `#notebook-pane-rules` and scrolls/highlights `anchor`.
#[wasm_bindgen]
pub fn satya_open_rules_section(chapter_id: String, anchor: String) {
    with_state_mut(|st| {
        st.reader_source_mode = "RULES".to_string();
    });
    set_select_value("rules-lore-source-select", "RULES");
    render_rules_chapters_dropdown(&chapter_id);
    satya_select_notebook_tab("rules".to_string());
    satya_select_rules_chapter(chapter_id);
    scroll_and_highlight_anchor(&anchor);
}

/// Opens a Campaign Lore chapter in `#notebook-pane-rules` and scrolls/highlights `anchor`.
#[wasm_bindgen]
pub fn satya_open_campaign_lore_section(rel_path: String, anchor: String) {
    with_state_mut(|st| {
        st.reader_source_mode = "CAMPAIGN_LORE".to_string();
    });
    set_select_value("rules-lore-source-select", "CAMPAIGN_LORE");
    let clean_path = rel_path.split('#').next().unwrap_or(&rel_path).trim().to_string();
    render_campaign_lore_chapters_dropdown(&clean_path);
    refresh_campaign_lore_catalog();
    satya_select_notebook_tab("rules".to_string());
    load_campaign_lore_chapter_and_scroll(clean_path, anchor);
}

/// Switches the displayed chapter in the `Rules & Lore` notebook tab (`#rules-book-viewport`).
#[wasm_bindgen]
pub fn satya_select_rules_chapter(chapter_id: String) {
    let active_mode = with_state(|st| st.reader_source_mode.clone());
    let chosen = if chapter_id.trim().is_empty() {
        select_value("rules-chapter-select")
    } else {
        set_select_value("rules-chapter-select", &chapter_id);
        chapter_id
    };
    if active_mode == "CAMPAIGN_LORE" {
        load_campaign_lore_chapter_and_scroll(chosen, String::new());
        return;
    }
    let (html, pill) = format_rules_chapter_html(&chosen);
    set_text("rules-chapter-pill", &pill);
    if let Some(vp) = get_element("rules-book-viewport") {
        vp.set_inner_html(&html);
        vp.set_scroll_top(0);
    }
}

/// Searches across all 24 canonical Satya TTRPG Rules Book chapters and renders clickable hits
/// inside `#rules-search-results`.
#[wasm_bindgen]
pub fn satya_search_rules_book() {
    let query = input_value("rules-search-input");
    let Some(results_el) = get_element("rules-search-results") else {
        return;
    };
    if query.trim().is_empty() {
        results_el.set_inner_html("");
        set_display("rules-search-results", "none");
        return;
    }
    let hits = satya_rules_book::search_rules_book(&query, 8);
    if hits.is_empty() {
        results_el.set_inner_html(&format!(
            r#"<div class="pill pill-muted">No rules sections matched "{}"</div>"#,
            escape_html(query.trim())
        ));
        set_display("rules-search-results", "flex");
        return;
    }
    let items: String = hits
        .iter()
        .map(|hit| {
            let anchor = satya_rules_book::slugify_heading(&hit.section_heading);
            format!(
                r#"<div class="gm-doc-card rules-search-hit"><div class="gm-doc-header"><button type="button" class="btn-chip" onclick="window.satyaOpenRulesSection('{}', '{}')">{}</button><span class="mono">{}</span></div><div class="gm-doc-preview">{}</div></div>"#,
                escape_js_single_quoted_attr(&hit.chapter_id),
                escape_js_single_quoted_attr(&anchor),
                escape_html(&hit.chapter_title),
                escape_html(&hit.section_heading),
                escape_html(&hit.snippet),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    results_el.set_inner_html(&items);
    set_display("rules-search-results", "flex");
}
