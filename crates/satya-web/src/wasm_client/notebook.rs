//! Left 10-Pane Tabbed Notebook UI (`Maps`, `3D Dice`, `3D Model`, `Character Sheet/Status`,
//! `Character Wizard`, `NPC List`, `Combat Tracker`, `Rules Book`, `GM Docs`, and `Apache Iggy`)
//! handlers and HTML formatters.

use crate::wasm_client::chat_and_ttrpg::{append_system_notice, satya_update_cascade_preview};
use crate::wasm_client::dom::{
    document, escape_html, escape_js_single_quoted_attr, fetch_json, get_element, input_value,
    roll_browser_die_i64, select_value, set_class, set_display, set_input_value, set_select_value,
    set_text,
};
pub use crate::wasm_client::gm_docs::*;
use crate::wasm_client::state::{SECONDS_PER_TICK, with_state, with_state_mut};
use crate::wasm_client::telemetry::satya_refresh_metrics;
use crate::wasm_client::webrtc::{send_ws_packet, sync_workspace_drawers_dom};
use satya_schema::{
    ChannelContext, ClientWsPacket, GmMapUpdateRequest, MessageVisibility, Participant,
    ParticipantKind, SessionRole, WgpuDiceSpec,
};
use wasm_bindgen::prelude::*;

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
        "Rules Book",
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

/// Switches the active view inside the 10-pane Left Tabbed Notebook (`#tabletop-canvas-card`) between
/// `"maps"`, `"dice"`, `"model"`, `"character"`, `"wizard"`, `"npcs"`, `"combat"`, `"rules"`, `"docs"`, and `"iggy"`.
#[wasm_bindgen]
pub fn satya_select_notebook_tab(tab_id: String) {
    let target = if NOTEBOOK_TABS.iter().any(|(slug, _, _)| *slug == tab_id) {
        tab_id
    } else {
        "maps".to_string()
    };

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
        render_combat_tracker();
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
        _ => "#38bdf8",
    };
    let mode_badge = match render_mode {
        "pbr" => "PBR-SHADED",
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

/// Advances (or resets when `delta == 0`) the Satya Count-Up Tick Timeline (`1 Tick = 0.25s`, `20 Ticks = 5.0s`) in the `Combat Tracker` notebook tab and persists the updated state to SQLite when in a live session.
#[wasm_bindgen]
pub fn satya_advance_combat_tick(delta: i32) {
    let (round, tick, session_opt, token_opt) = with_state_mut(|st| {
        if delta <= 0 {
            st.combat_round = 1;
            st.combat_tick = 1;
        } else {
            let next = st.combat_tick + (delta as u32);
            st.combat_tick = next;
            st.combat_round = 1 + (next / 20);
        }
        (
            st.combat_round,
            st.combat_tick,
            st.session_id.clone(),
            st.auth_token.clone(),
        )
    });
    let elapsed_sec = f64::from(tick) * SECONDS_PER_TICK;
    set_text(
        "combat-round-pill",
        &format!("Window {round} · Tick {tick} ({elapsed_sec:.2}s)"),
    );
    render_combat_tracker();

    if let (Some(sid), Some(token)) = (session_opt, token_opt) {
        let req = satya_schema::UpdateCombatTrackerRequest {
            tick_delta: Some(delta),
            combat_active: Some(true),
            entity_id: None,
            active_stance: None,
            next_action_tick: None,
            fatigue_level: None,
            wound_shock: None,
            grid_x: None,
            grid_y: None,
            zone_label: None,
        };
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/sessions/{sid}/combat");
            let _: Result<satya_schema::CombatTrackerState, _> =
                fetch_json("POST", &url, Some(&token), Some(&req)).await;
        });
    }
}

/// Formats the HTML chip list of active session combatants and their continuous Count-Up Tick readiness (`1 Tick = 0.25s`, `Tick_start = max(1, 15 - OODA - Awareness)`) for `#combat-tracker-list`.
pub fn format_combat_tracker_html(roster: &[Participant], round: u32, tick: u32) -> String {
    if roster.is_empty() {
        return r#"<span class="pill pill-muted">Join or create a session to populate active combatants.</span>"#.to_string();
    }
    roster
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let ooda = (13 - (idx as i32)).clamp(8, 16);
            let start_tick = satya_engine::compute_starting_tick(ooda, 0, 0, ooda + 1, 12) as u32;
            let actor_tick = tick.max(start_tick) + (idx as u32) * 2;
            let actor_sec = f64::from(actor_tick) * SECONDS_PER_TICK;
            let badge_cls = if p.participant_kind == ParticipantKind::AiAgent {
                "participant-chip ai"
            } else {
                "participant-chip"
            };
            format!(
                r#"<div class="{badge_cls}"><strong>{}</strong> [{:?}] · OODA {ooda} · Next Action @ Tick {actor_tick} ({actor_sec:.2}s · W{round})</div>"#,
                escape_html(&p.display_name),
                p.role
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

pub(crate) fn render_combat_tracker() {
    let (roster, round, tick) =
        with_state(|st| (st.roster.clone(), st.combat_round, st.combat_tick));
    if let Some(el) = get_element("combat-tracker-list") {
        el.set_inner_html(&format_combat_tracker_html(&roster, round, tick));
    }
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
    let doc = satya_engine::get_rules_chapter(chapter_id)
        .or_else(|| satya_engine::get_rules_chapter("tasks"));
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

/// Switches the displayed chapter in the `Rules Book` notebook tab (`#rules-book-viewport`).
#[wasm_bindgen]
pub fn satya_select_rules_chapter(chapter_id: String) {
    let target = if chapter_id.trim().is_empty() {
        select_value("rules-chapter-select")
    } else {
        set_select_value("rules-chapter-select", &chapter_id);
        chapter_id
    };
    let (html, pill) = format_rules_chapter_html(&target);
    set_text("rules-chapter-pill", &pill);
    if let Some(vp) = get_element("rules-book-viewport") {
        vp.set_inner_html(&html);
        vp.set_scroll_top(0);
    }
}

/// Searches across all 17 canonical Satya TTRPG Rules Book chapters and renders clickable hits
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
    let hits = satya_engine::search_rules_book(&query, 8);
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
            format!(
                r#"<div class="gm-doc-card rules-search-hit"><div class="gm-doc-header"><button type="button" class="btn-chip" onclick="window.satyaSelectRulesChapter('{}')">{}</button><span class="mono">{}</span></div><div class="gm-doc-preview">{}</div></div>"#,
                escape_js_single_quoted_attr(&hit.chapter_id),
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

/// Loads a canonical worked example from the Satya Rules Book into the Center-Bottom `2d20`
/// Task Resolution drawer (`#ttrpg-drawer`) and recalculates the exact probability preview.
#[wasm_bindgen]
pub fn satya_load_rules_example(preset: String) {
    let p = satya_engine::get_canonical_task_preset(&preset);
    let was_collapsed = with_state_mut(|st| {
        let prev = st.ttrpg_collapsed;
        st.ttrpg_collapsed = false;
        prev
    });
    if was_collapsed {
        sync_workspace_drawers_dom();
    }
    set_input_value("ttrpg-skill", p.skill_name);
    set_input_value("ttrpg-skill-level", &p.player_skill_n.to_string());
    set_input_value("ttrpg-control-c", &p.player_control_c.to_string());
    set_input_value("ttrpg-internal-mod", &p.player_internal_mod.to_string());
    set_input_value("ttrpg-tool-mod", &p.player_known_tool_mod.to_string());
    set_input_value(
        "ttrpg-external-mod",
        &p.hidden_external_difficulty.to_string(),
    );
    set_select_value("ttrpg-carry-mode", "markov_step");
    set_select_value("ttrpg-botch-rule", "closest_miss");
    set_text("char-stat-control", &p.player_control_c.to_string());
    satya_update_cascade_preview();
}
