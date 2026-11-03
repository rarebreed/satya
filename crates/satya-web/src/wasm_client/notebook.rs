//! Left Tabbed Notebook UI (`Maps`, `3D Dice`, `3D Model`, `Character Sheet/Status`,
//! `Combat Tracker`, and `Apache Iggy, Topics & PyLance`) handlers and HTML formatters.

use crate::wasm_client::chat_and_ttrpg::{append_system_notice, satya_update_cascade_preview};
use crate::wasm_client::dom::{
    document, escape_html, get_element, select_value, set_class, set_display, set_input_value,
    set_text,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use crate::wasm_client::telemetry::satya_refresh_metrics;
use crate::wasm_client::webrtc::send_ws_packet;
use satya_schema::{
    ChannelContext, ClientWsPacket, GmMapUpdateRequest, MessageVisibility, Participant,
    ParticipantKind, SessionRole, WgpuDiceSpec,
};
use wasm_bindgen::prelude::*;

/// Canonical list of the 6 Tabbed Notebook tab slugs and their `(button_id, pane_id)` pairs.
pub const NOTEBOOK_TABS: &[(&str, &str, &str)] = &[
    ("maps", "tab-btn-maps", "notebook-pane-maps"),
    ("dice", "tab-btn-dice", "notebook-pane-dice"),
    ("model", "tab-btn-model", "notebook-pane-model"),
    ("character", "tab-btn-character", "notebook-pane-character"),
    ("combat", "tab-btn-combat", "notebook-pane-combat"),
    ("iggy", "tab-btn-iggy", "notebook-pane-iggy"),
];

/// Switches the active view inside the Left Tabbed Notebook (`#tabletop-canvas-card`) between
/// `"maps"`, `"dice"`, `"model"`, `"character"`, `"combat"`, and `"iggy"`.
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

    if target == "iggy" {
        satya_refresh_metrics();
    } else if target == "combat" {
        render_combat_tracker();
    }
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
    let Ok(outcome) = pipeline.evaluate_with_roller(&expr_raw, |sides| {
        ((js_sys::Math::random() * (sides as f64)).floor() as i64 + 1).clamp(1, sides)
    }) else {
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
    let (title, tris, scale, hit_zones) = match preset {
        "recon_mk4" => (
            "Recon Suit Mk-IV (Marcus)",
            "11.8k tris",
            "1.85m · Light EVA",
            "Optics Visor (DR 3) · Chest Carapace (DR 4) · Thruster Pack (DR 2)",
        ),
        "secops_sentry" => (
            "SecOps Heavy Sentry",
            "18.4k tris",
            "2.10m · Armored Bipedal",
            "Sensor Dome (DR 5) · Reactive Plating (DR 7) · Servo Joints (DR 3)",
        ),
        "sector4_airlock" => (
            "Sector 4 Orbital Airlock",
            "26.0k tris",
            "20m x 15m · Zero-G Chamber",
            "Inner Bulkhead · Decompression Valve · Mag-Rail Crane",
        ),
        _ => (
            "Valkyrie Exo-Rig (Sara)",
            "14.2k tris",
            "1.92m · Med-Assault Exo",
            "Ballistic Helm (DR 4) · Trauma Auto-Injector · Mag-Boots (DR 4)",
        ),
    };

    let stroke_color = match render_mode {
        "hit_zones" => "#f43f5e",
        "thermal" => "#f59e0b",
        _ => "#38bdf8",
    };
    let mode_badge = match render_mode {
        "hit_zones" => "HIT-ZONES",
        "thermal" => "THERMAL-IR",
        _ => "WIREFRAME+PBR",
    };

    let pill = format!("Mesh: {title} · {tris}");
    let html = format!(
        r##"<div class="model-stage-inner">
            <svg viewBox="0 0 360 210" class="model-wireframe-svg" role="img" aria-label="{title}">
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
                <text x="14" y="196" fill="#e2e8f0" font-family="monospace" font-size="11">{title} ({scale})</text>
            </svg>
            <div class="model-hit-zones">
                <span class="roster-title">Hit Locations &amp; Subsystem Mesh:</span>
                <span class="mono">{hit_zones}</span>
            </div>
        </div>"##,
    );
    (html, pill)
}

/// Updates the `3D Model` notebook tab stage when `#model-preset-select` or `#model-render-mode` changes.
#[wasm_bindgen]
pub fn satya_update_3d_model_view() {
    let preset = select_value("model-preset-select");
    let mode = select_value("model-render-mode");
    let (html, pill) = format_3d_model_stage_html(&preset, &mode);
    set_text("model-telemetry-pill", &pill);
    if let Some(stage) = get_element("model-preview-stage") {
        stage.set_inner_html(&html);
    }
}

/// Populates the Center-Bottom Die Roller (`#ttrpg-drawer`) with a skill preset clicked in the
/// `Character Sheet/Status` notebook tab and recalculates the exact Four-Factor Cascade preview.
#[wasm_bindgen]
pub fn satya_load_skill_preset(skill_name: String, skill_level: i32, control_c: i32) {
    set_input_value("ttrpg-skill", &skill_name);
    set_input_value("ttrpg-skill-level", &skill_level.clamp(1, 10).to_string());
    set_input_value("ttrpg-control-c", &control_c.clamp(1, 20).to_string());
    set_text("char-stat-control", &control_c.clamp(1, 20).to_string());
    satya_update_cascade_preview();
}

/// Advances (or resets when `delta == 0`) the Satya Action-Count (Tick) clock in the `Combat Tracker` notebook tab.
#[wasm_bindgen]
pub fn satya_advance_combat_tick(delta: i32) {
    let (round, tick) = with_state_mut(|st| {
        if delta <= 0 {
            st.combat_round = 1;
            st.combat_tick = 0;
        } else {
            let next = st.combat_tick + (delta as u32);
            st.combat_round += next / 20;
            st.combat_tick = next % 20;
        }
        (st.combat_round, st.combat_tick)
    });
    set_text("combat-round-pill", &format!("Round {round} · Tick {tick}"));
    render_combat_tracker();
}

/// Formats the HTML chip list of active session combatants and their tick readiness for `#combat-tracker-list`.
pub fn format_combat_tracker_html(roster: &[Participant], round: u32, tick: u32) -> String {
    if roster.is_empty() {
        return r#"<span class="pill pill-muted">Join or create a session to populate active combatants.</span>"#.to_string();
    }
    roster
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let actor_tick = (tick + (idx as u32) * 3) % 20;
            let badge_cls = if p.participant_kind == ParticipantKind::AiAgent {
                "participant-chip ai"
            } else {
                "participant-chip"
            };
            format!(
                r#"<div class="{badge_cls}"><strong>{}</strong> [{:?}] · R{round} Tick {actor_tick}</div>"#,
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

/// Renders an incoming [`GmMapUpdateRequest`] into the `#tabletop-canvas-card` SVG battlemap viewport and token bar.
pub fn render_map_updated(map: &GmMapUpdateRequest) {
    set_text(
        "canvas-map-pill",
        &format!("{} ({}x{})", map.title, map.grid_width, map.grid_height),
    );
    set_class("canvas-map-pill", "pill pill-active");
    set_display("tabletop-map-placeholder", "none");
    if let Some(img) = get_element("tabletop-map-img") {
        let _ = img.set_attribute("src", &map.image_url);
        let _ = img.set_attribute(
            "alt",
            &format!(
                "Tactical map {} ({}x{})",
                map.title, map.grid_width, map.grid_height
            ),
        );
    }
    set_display("tabletop-map-img", "block");
    if let Some(list) = get_element("tabletop-token-list") {
        list.set_inner_html(&format_tactical_tokens_html(map));
    }
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

/// Renders an incoming [`WgpuDiceSpec`] into `#wgpu-dice-readout` inside the tabletop canvas card.
pub fn render_wgpu_dice_roll(actor: &Participant, spec: &WgpuDiceSpec) {
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
