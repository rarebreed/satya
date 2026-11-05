//! Left Tabbed Notebook UI (`Maps`, `3D Dice`, `3D Model`, `Character Sheet/Status`,
//! `Combat Tracker`, `GM Docs`, and `Apache Iggy, Topics & PyLance`) handlers and HTML formatters.

use crate::wasm_client::chat_and_ttrpg::{append_system_notice, satya_update_cascade_preview};
use crate::wasm_client::dom::{
    document, escape_html, fetch_json, get_element, input_value, select_value, set_class,
    set_display, set_feedback, set_input_value, set_select_value, set_text, window,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use crate::wasm_client::telemetry::satya_refresh_metrics;
use crate::wasm_client::webrtc::{send_ws_packet, sync_workspace_drawers_dom};
use js_sys::{Function, Promise, Reflect};
use satya_schema::{
    ChannelContext, ClientWsPacket, GmDocumentRecord, GmDocumentSource, GmMapUpdateRequest,
    GoogleAuthLinkRequest, GoogleAuthStatusResponse, MessageVisibility, Participant,
    ParticipantKind, SessionGmDocsResponse, SessionRole, SubmitGmDocumentRequest, WgpuDiceSpec,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Headers, HtmlElement, Request, RequestInit, Response};

/// Canonical list of the 8 Tabbed Notebook tab slugs and their `(button_id, pane_id)` pairs.
pub const NOTEBOOK_TABS: &[(&str, &str, &str)] = &[
    ("maps", "tab-btn-maps", "notebook-pane-maps"),
    ("dice", "tab-btn-dice", "notebook-pane-dice"),
    ("model", "tab-btn-model", "notebook-pane-model"),
    ("character", "tab-btn-character", "notebook-pane-character"),
    ("combat", "tab-btn-combat", "notebook-pane-combat"),
    ("rules", "tab-btn-rules", "notebook-pane-rules"),
    ("docs", "tab-btn-docs", "notebook-pane-docs"),
    ("iggy", "tab-btn-iggy", "notebook-pane-iggy"),
];

/// Switches the active view inside the Left Tabbed Notebook (`#tabletop-canvas-card`) between
/// `"maps"`, `"dice"`, `"model"`, `"character"`, `"combat"`, `"rules"`, `"docs"`, and `"iggy"`.
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
    } else if target == "docs" {
        satya_refresh_gm_documents();
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

/// Advances (or resets when `delta == 0`) the Satya Count-Up Tick Timeline (`1 Tick = 0.5s`) in the `Combat Tracker` notebook tab.
#[wasm_bindgen]
pub fn satya_advance_combat_tick(delta: i32) {
    let (round, tick) = with_state_mut(|st| {
        if delta <= 0 {
            st.combat_round = 1;
            st.combat_tick = 1;
        } else {
            let next = st.combat_tick + (delta as u32);
            st.combat_tick = next;
            st.combat_round = 1 + (next / 20);
        }
        (st.combat_round, st.combat_tick)
    });
    let elapsed_sec = f64::from(tick) * 0.5;
    set_text(
        "combat-round-pill",
        &format!("Window {round} · Tick {tick} ({elapsed_sec:.1}s)"),
    );
    render_combat_tracker();
}

/// Formats the HTML chip list of active session combatants and their continuous Count-Up Tick readiness (`Tick_start = max(1, 15 - OODA - Awareness)`) for `#combat-tracker-list`.
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
            let badge_cls = if p.participant_kind == ParticipantKind::AiAgent {
                "participant-chip ai"
            } else {
                "participant-chip"
            };
            format!(
                r#"<div class="{badge_cls}"><strong>{}</strong> [{:?}] · OODA {ooda} · Next Action @ Tick {actor_tick} (W{round})</div>"#,
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

/// Parses a `GM_ONLY` document source select string into [`GmDocumentSource`].
pub fn parse_gm_doc_source(raw: &str) -> GmDocumentSource {
    match raw.trim().to_uppercase().as_str() {
        "GOOGLE_DOC" => GmDocumentSource::GoogleDoc,
        "GOOGLE_SHEET" => GmDocumentSource::GoogleSheet,
        _ => GmDocumentSource::DirectTextUpload,
    }
}

/// Formats the HTML list of sanitized `GM_ONLY` documents for `#gm-docs-list`.
pub fn format_gm_documents_list_html(docs: &[GmDocumentRecord]) -> String {
    if docs.is_empty() {
        return r#"<span class="pill pill-muted">No GM_ONLY documents submitted in this session yet.</span>"#.to_string();
    }
    docs.iter()
        .rev()
        .map(|d| {
            let src_label = match d.source_kind {
                GmDocumentSource::GoogleDoc => "Google Doc (drive.file)",
                GmDocumentSource::GoogleSheet => "Google Sheet (drive.file)",
                GmDocumentSource::DirectTextUpload => "Text/Markdown",
            };
            let flag_badge = if d.safety_flagged {
                r#" <span class="pill pill-muted" title="HTML/script or prompt-injection tokens were scrubbed">Sanitized</span>"#
            } else {
                ""
            };
            format!(
                r#"<div class="gm-doc-card">
                    <div class="gm-doc-header">
                        <strong>{}</strong>
                        <span class="pill pill-iggy">{src_label} · {} chars · GM_ONLY</span>{flag_badge}
                    </div>
                    <div class="subtitle">Submitted by <strong>{}</strong></div>
                    <pre class="gm-doc-body">{}</pre>
                </div>"#,
                escape_html(&d.title),
                d.char_count,
                escape_html(&d.uploader_display_name),
                escape_html(&d.sanitized_content)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Appends or updates an incoming [`GmDocumentRecord`] in client state and re-renders `#gm-docs-list`.
pub fn render_gm_document_added(doc: &GmDocumentRecord) {
    let docs = with_state_mut(|st| {
        if let Some(existing) = st.gm_documents.iter_mut().find(|d| d.doc_id == doc.doc_id) {
            *existing = doc.clone();
        } else {
            st.gm_documents.push(doc.clone());
        }
        st.gm_documents.clone()
    });
    if let Some(list_el) = get_element("gm-docs-list") {
        list_el.set_inner_html(&format_gm_documents_list_html(&docs));
    }
    set_text("gm-docs-count-pill", &format!("{} GM Docs", docs.len()));
}

/// Extracts a clean Google Drive File ID (and optional inferred `GOOGLE_DOC` / `GOOGLE_SHEET` kind)
/// from either a raw File ID or a full `https://docs.google.com/.../d/<FILE_ID>/...` URL.
pub fn extract_google_drive_file_id_and_kind(raw_input: &str) -> (String, Option<&'static str>) {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    let inferred_kind = if trimmed.contains("docs.google.com/spreadsheets") {
        Some("GOOGLE_SHEET")
    } else if trimmed.contains("docs.google.com/document") {
        Some("GOOGLE_DOC")
    } else {
        None
    };

    if let Some(after_d) = trimmed.split("/d/").nth(1)
        && let Some(id_seg) = after_d.split(['/', '?', '#', '&']).next()
        && !id_seg.trim().is_empty()
    {
        return (id_seg.trim().to_string(), inferred_kind);
    }
    if let Some(after_id) = trimmed.split("id=").nth(1)
        && let Some(id_seg) = after_id.split(['&', '#', '/']).next()
        && !id_seg.trim().is_empty()
    {
        return (id_seg.trim().to_string(), inferred_kind);
    }
    (trimmed.to_string(), inferred_kind)
}

/// Synchronizes server-configured Google OAuth (`drive.file`) settings (`google_oauth_client_id`
/// and `google_picker_api_key`) from `GET /api/auth/google/status` after user authentication.
pub fn sync_google_auth_status_from_server() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        if let Ok(status) = fetch_json::<(), GoogleAuthStatusResponse>(
            "GET",
            "/api/auth/google/status",
            Some(&tok),
            None,
        )
        .await
        {
            with_state_mut(|st| {
                st.google_linked = status.google_linked;
                if status.google_oauth_client_id.is_some() {
                    st.google_oauth_client_id = status.google_oauth_client_id.clone();
                }
                if status.google_picker_api_key.is_some() {
                    st.google_picker_api_key = status.google_picker_api_key.clone();
                }
            });
            if let Some(cid) = status.google_oauth_client_id.as_deref()
                && input_value("gm-google-client-id").trim().is_empty()
            {
                set_input_value("gm-google-client-id", cid);
            }
            if let Some(akey) = status.google_picker_api_key.as_deref()
                && input_value("gm-google-api-key").trim().is_empty()
            {
                set_input_value("gm-google-api-key", akey);
            }
            if status.google_linked {
                set_text(
                    "google-auth-status-pill",
                    "Google Linked (drive.file Scope)",
                );
                set_class("google-auth-status-pill", "pill pill-active");
            }
        }
    });
}

/// Links the authenticated user's Satya account to Google OAuth (`drive.file` single-file scope)
/// via `POST /api/auth/google`. If a Google Cloud OAuth Client ID is configured, launches Google Identity
/// Services (`google.accounts.oauth2.initTokenClient`) first; otherwise links in local development mode.
#[wasm_bindgen]
pub fn satya_link_google_account() {
    let client_id = resolved_google_client_id();
    let api_key = resolved_google_api_key();
    if !client_id.is_empty() {
        launch_browser_google_picker(&client_id, &api_key, false);
        return;
    }

    spawn_local(async move {
        let (auth_token, user) = with_state(|st| (st.auth_token.clone(), st.user.clone()));
        let (Some(tok), Some(u)) = (auth_token, user) else {
            set_feedback(
                "auth-feedback",
                "Please log in to your Satya account first before linking Google (`drive.file`).",
                true,
            );
            return;
        };

        let req = GoogleAuthLinkRequest {
            google_sub: format!("google-oidc-{}", u.username),
            google_display_hint: Some(u.display_name.clone()),
            access_token: Some(format!("ya29.drive-file-ephemeral-{}", u.user_id)),
        };

        match fetch_json::<_, GoogleAuthStatusResponse>(
            "POST",
            "/api/auth/google",
            Some(&tok),
            Some(&req),
        )
        .await
        {
            Ok(status) => {
                with_state_mut(|st| {
                    st.google_linked = status.google_linked;
                });
                set_text(
                    "google-auth-status-pill",
                    "Google Linked (drive.file Scope)",
                );
                set_class("google-auth-status-pill", "pill pill-active");
                set_feedback(
                    "auth-feedback",
                    "Linked Google identity (`drive.file` single-file scope). Personal email hidden from other players.",
                    false,
                );
            }
            Err(err) => set_feedback("auth-feedback", &err, true),
        }
    });
}

fn resolved_google_client_id() -> String {
    let ui_val = input_value("gm-google-client-id").trim().to_string();
    if !ui_val.is_empty() {
        return ui_val;
    }
    with_state(|st| st.google_oauth_client_id.clone().unwrap_or_default())
}

fn resolved_google_api_key() -> String {
    let ui_val = input_value("gm-google-api-key").trim().to_string();
    if !ui_val.is_empty() {
        return ui_val;
    }
    with_state(|st| st.google_picker_api_key.clone().unwrap_or_default())
}

/// Receives a live Google OAuth 2.0 `drive.file` access token from Google Identity Services in the browser
/// and links it to the caller's Satya account via `POST /api/auth/google`.
#[wasm_bindgen]
pub fn satya_receive_google_token(access_token: String) {
    let clean_tok = access_token.trim().to_string();
    if clean_tok.is_empty() {
        return;
    }
    with_state_mut(|st| {
        st.google_access_token = Some(clean_tok.clone());
        st.google_linked = true;
    });
    set_text(
        "google-auth-status-pill",
        "Google Linked (drive.file Scope)",
    );
    set_class("google-auth-status-pill", "pill pill-active");

    spawn_local(async move {
        let (auth_token, user) = with_state(|st| (st.auth_token.clone(), st.user.clone()));
        let (Some(tok), Some(u)) = (auth_token, user) else {
            return;
        };
        let req = GoogleAuthLinkRequest {
            google_sub: format!("google-oidc-{}", u.username),
            google_display_hint: Some(u.display_name),
            access_token: Some(clean_tok),
        };
        let _ = fetch_json::<_, GoogleAuthStatusResponse>(
            "POST",
            "/api/auth/google",
            Some(&tok),
            Some(&req),
        )
        .await;
    });
}

/// Receives a Google Doc or Google Sheet selected via the browser's `google.picker.PickerBuilder`
/// (`drive.file` single-file scope) along with its exported plain-text or CSV snapshot.
#[wasm_bindgen]
pub fn satya_receive_picked_google_doc(
    file_id: String,
    title: String,
    source_kind: String,
    content: String,
) {
    if !title.trim().is_empty() {
        set_input_value("gm-doc-title", title.trim());
    }
    let normalized_src = if source_kind == "GOOGLE_SHEET" {
        "GOOGLE_SHEET"
    } else {
        "GOOGLE_DOC"
    };
    set_select_value("gm-doc-source", normalized_src);
    set_input_value("gm-doc-google-id", file_id.trim());
    set_input_value("gm-doc-content", &content);
    satya_gm_doc_source_changed();
    set_feedback(
        "gm-doc-feedback",
        &format!(
            "Selected '{}' ({} chars) via Google Drive Picker (`drive.file`). Click 'Submit to GM Only' to share with the GM.",
            title.trim(),
            content.chars().count()
        ),
        false,
    );
}

/// Reads a local `.txt`, `.md`, or `.csv` file selected via `#gm-doc-file-input` (`File.text()`)
/// and populates `#gm-doc-title`, `#gm-doc-source`, and `#gm-doc-content`.
#[wasm_bindgen]
pub fn satya_gm_doc_file_selected() {
    spawn_local(async move {
        let Some(el) = get_element("gm-doc-file-input") else {
            return;
        };
        let Ok(files) = Reflect::get(&el, &JsValue::from_str("files")) else {
            return;
        };
        let Ok(file) = Reflect::get(&files, &JsValue::from_f64(0.0)) else {
            return;
        };
        if file.is_undefined() || file.is_null() {
            return;
        }
        let file_name = Reflect::get(&file, &JsValue::from_str("name"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "Uploaded Document".to_string());

        let Ok(text_fn_val) = Reflect::get(&file, &JsValue::from_str("text")) else {
            return;
        };
        let Ok(text_fn) = text_fn_val.dyn_into::<Function>() else {
            return;
        };
        let Ok(promise_val) = text_fn.call0(&file) else {
            return;
        };
        let Ok(promise) = promise_val.dyn_into::<Promise>() else {
            return;
        };
        let Ok(text_val) = JsFuture::from(promise).await else {
            set_feedback(
                "gm-doc-feedback",
                "Failed to read local file contents.",
                true,
            );
            return;
        };
        let mut content = text_val.as_string().unwrap_or_default();
        let mut truncated_note = "";
        if content.len() > 32_768 {
            content.truncate(32_768);
            truncated_note = " (truncated to 32 KB limit)";
        }

        let clean_title = file_name
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(&file_name)
            .replace(['_', '-'], " ");
        if !clean_title.trim().is_empty() {
            set_input_value("gm-doc-title", clean_title.trim());
        }
        set_select_value("gm-doc-source", "DIRECT_TEXT_UPLOAD");
        satya_gm_doc_source_changed();
        set_input_value("gm-doc-content", &content);
        set_feedback(
            "gm-doc-feedback",
            &format!(
                "Loaded '{}' ({} chars){truncated_note}. Click 'Submit to GM Only' to share with the GM.",
                file_name,
                content.chars().count()
            ),
            false,
        );
    });
}

/// Automatically extracts the Google Drive File ID (and switches `#gm-doc-source` between
/// `GOOGLE_DOC` and `GOOGLE_SHEET`) when the user pastes a full `https://docs.google.com/...` URL
/// into `#gm-doc-google-id`.
#[wasm_bindgen]
pub fn satya_gm_doc_url_changed() {
    let raw = input_value("gm-doc-google-id");
    let (clean_id, inferred_kind) = extract_google_drive_file_id_and_kind(&raw);
    if !clean_id.is_empty() && clean_id != raw {
        set_input_value("gm-doc-google-id", &clean_id);
        if let Some(kind) = inferred_kind {
            set_select_value("gm-doc-source", kind);
            satya_gm_doc_source_changed();
        }
        set_feedback(
            "gm-doc-feedback",
            &format!("Extracted Google Drive File ID: {clean_id}"),
            false,
        );
    }
}

/// Toggles visibility of `#gm-google-oauth-config-row` so the user can inspect or enter a
/// Google Cloud OAuth 2.0 Web Client ID and Google Picker API Key.
#[wasm_bindgen]
pub fn satya_toggle_google_oauth_config() {
    let Some(el) =
        get_element("gm-google-oauth-config-row").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let cur = el.style().get_property_value("display").unwrap_or_default();
    let next = if cur == "none" || cur.is_empty() {
        "grid"
    } else {
        "none"
    };
    let _ = el.style().set_property("display", next);
}

/// Opens Google's official `google.picker.PickerBuilder` modal (`drive.file` single-file scope)
/// so the user can click a Google Doc or Google Sheet in their Google Drive, grant single-file
/// read permission, and export its plain-text/CSV content into `#gm-doc-content`.
#[wasm_bindgen]
pub fn satya_open_google_drive_picker() {
    let client_id = resolved_google_client_id();
    let api_key = resolved_google_api_key();
    if client_id.is_empty() {
        set_display("gm-google-oauth-config-row", "grid");
        set_feedback(
            "gm-doc-feedback",
            "To open the live Google Drive Picker (`drive.file` single-file scope), enter your Google Cloud Web OAuth Client ID & Picker API Key above (or in `config/storage.toml`)—or download your Google Doc as .txt/.md (or Sheet as .csv) and use 'Choose Local File' above.",
            false,
        );
        return;
    }
    launch_browser_google_picker(&client_id, &api_key, true);
}

fn launch_browser_google_picker(client_id: &str, api_key: &str, open_picker: bool) {
    let js_code = r#"
        const loadScript = (src) => new Promise((resolve, reject) => {
            if (document.querySelector(`script[src="${src}"]`)) return resolve();
            const s = document.createElement('script');
            s.src = src;
            s.async = true;
            s.onload = () => resolve();
            s.onerror = () => reject(new Error('Failed to load ' + src));
            document.head.appendChild(s);
        });
        (async () => {
            try {
                await loadScript('https://accounts.google.com/gsi/client');
                if (openPicker) {
                    await loadScript('https://apis.google.com/js/api.js');
                }
                const runPicker = (token) => {
                    if (window.satyaReceiveGoogleToken) {
                        window.satyaReceiveGoogleToken(token);
                    }
                    if (!openPicker) return;
                    if (!window.gapi) return;
                    window.gapi.load('picker', () => {
                        const docsView = new window.google.picker.DocsView(window.google.picker.ViewId.DOCUMENTS);
                        const sheetsView = new window.google.picker.DocsView(window.google.picker.ViewId.SPREADSHEETS);
                        const builder = new window.google.picker.PickerBuilder()
                            .addView(docsView)
                            .addView(sheetsView)
                            .setOAuthToken(token)
                            .setCallback(async (data) => {
                                if (data.action === window.google.picker.Action.PICKED && data.docs && data.docs.length > 0) {
                                    const doc = data.docs[0];
                                    const isSheet = (doc.mimeType || '').includes('spreadsheet');
                                    const exportMime = isSheet ? 'text/csv' : 'text/plain';
                                    const srcKind = isSheet ? 'GOOGLE_SHEET' : 'GOOGLE_DOC';
                                    let exported = '';
                                    try {
                                        const resp = await fetch(
                                            `https://www.googleapis.com/drive/v3/files/${encodeURIComponent(doc.id)}/export?mimeType=${encodeURIComponent(exportMime)}`,
                                            { headers: { Authorization: `Bearer ${token}` } }
                                        );
                                        if (resp.ok) {
                                            exported = await resp.text();
                                        }
                                    } catch (_) {}
                                    if (window.satyaReceivePickedGoogleDoc) {
                                        window.satyaReceivePickedGoogleDoc(doc.id || '', doc.name || 'Google Drive Document', srcKind, exported);
                                    }
                                }
                            });
                        if (apiKey) {
                            builder.setDeveloperKey(apiKey);
                        }
                        builder.build().setVisible(true);
                    });
                };
                if (window._satyaGoogleAccessToken && openPicker) {
                    runPicker(window._satyaGoogleAccessToken);
                    return;
                }
                const tokenClient = window.google.accounts.oauth2.initTokenClient({
                    client_id: clientId,
                    scope: 'https://www.googleapis.com/auth/drive.file',
                    callback: (resp) => {
                        if (resp && resp.access_token) {
                            window._satyaGoogleAccessToken = resp.access_token;
                            runPicker(resp.access_token);
                        }
                    }
                });
                tokenClient.requestAccessToken({ prompt: '' });
            } catch (err) {
                console.error('Google Picker launch error:', err);
            }
        })();
    "#;
    let func = Function::new_with_args("clientId, apiKey, openPicker", js_code);
    let _ = func.call3(
        &JsValue::NULL,
        &JsValue::from_str(client_id),
        &JsValue::from_str(api_key),
        &JsValue::from_bool(open_picker),
    );
}

async fn try_export_google_drive_file_in_browser(
    source_kind: GmDocumentSource,
    file_id: &str,
    access_token: &str,
) -> Option<String> {
    let win = window()?;
    let export_mime = match source_kind {
        GmDocumentSource::GoogleSheet => "text/csv",
        _ => "text/plain",
    };
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}/export?mimeType={}",
        file_id.trim(),
        export_mime
    );
    let opts = RequestInit::new();
    opts.set_method("GET");
    let headers = Headers::new().ok()?;
    let _ = headers.set("Authorization", &format!("Bearer {access_token}"));
    opts.set_headers(&headers);
    let req = Request::new_with_str_and_init(&url, &opts).ok()?;
    let resp_val = JsFuture::from(win.fetch_with_request(&req)).await.ok()?;
    let resp: Response = resp_val.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let text_promise = resp.text().ok()?;
    let text_val = JsFuture::from(text_promise).await.ok()?;
    text_val.as_string().filter(|s| !s.trim().is_empty())
}

/// Toggles the Google File ID picker input when `#gm-doc-source` changes between
/// `DIRECT_TEXT_UPLOAD`, `GOOGLE_DOC`, and `GOOGLE_SHEET`.
#[wasm_bindgen]
pub fn satya_gm_doc_source_changed() {
    let src = parse_gm_doc_source(&select_value("gm-doc-source"));
    let is_google = matches!(
        src,
        GmDocumentSource::GoogleDoc | GmDocumentSource::GoogleSheet
    );
    set_display("gm-doc-google-row", if is_google { "grid" } else { "none" });
    if !is_google {
        set_display("gm-google-oauth-config-row", "none");
    }
}

/// Loads a built-in Google Doc (`gdoc-demo-sara`) or Google Sheet (`gsheet-demo-gear`) Picker preset
/// into the `GM Docs` form for single-click testing.
#[wasm_bindgen]
pub fn satya_load_gm_doc_preset(preset_kind: String) {
    if preset_kind == "sheet" {
        set_select_value("gm-doc-source", "GOOGLE_SHEET");
        set_input_value("gm-doc-title", "Valkyrie Squad Loadout Sheet");
        set_input_value("gm-doc-google-id", "gsheet-demo-gear");
        set_input_value("gm-doc-content", "");
    } else {
        set_select_value("gm-doc-source", "GOOGLE_DOC");
        set_input_value("gm-doc-title", "Sara Valkyrie Medical & Sector 4 Dossier");
        set_input_value("gm-doc-google-id", "gdoc-demo-sara");
        set_input_value("gm-doc-content", "");
    }
    satya_gm_doc_source_changed();
}

/// Submits a `GM_ONLY` document (direct text/Markdown/CSV or Google Doc/Sheet snapshot)
/// to `POST /api/sessions/:session_id/docs`.
#[wasm_bindgen]
pub fn satya_submit_gm_document() {
    spawn_local(async move {
        let (session_id, auth_token, google_tok) = with_state(|st| {
            (
                st.session_id.clone(),
                st.auth_token.clone(),
                st.google_access_token.clone(),
            )
        });
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            set_feedback(
                "gm-doc-feedback",
                "Join or create an active session first before submitting a GM_ONLY document.",
                true,
            );
            return;
        };

        let title = input_value("gm-doc-title").trim().to_string();
        let source_kind = parse_gm_doc_source(&select_value("gm-doc-source"));
        let mut raw_text = input_value("gm-doc-content");
        let (google_id, _) =
            extract_google_drive_file_id_and_kind(&input_value("gm-doc-google-id"));
        if !google_id.is_empty() {
            set_input_value("gm-doc-google-id", &google_id);
        }

        // If importing a non-demo Google Doc/Sheet with an active browser `drive.file` token and empty textarea,
        // export the plain-text/CSV content directly from the Google Drive API in the browser first.
        if matches!(
            source_kind,
            GmDocumentSource::GoogleDoc | GmDocumentSource::GoogleSheet
        ) && raw_text.trim().is_empty()
            && !google_id.is_empty()
            && !google_id.starts_with("gdoc-demo")
            && !google_id.starts_with("gsheet-demo")
            && let Some(ref gtok) = google_tok
            && !gtok.starts_with("ya29.drive-file-ephemeral-")
            && let Some(exported) =
                try_export_google_drive_file_in_browser(source_kind, &google_id, gtok).await
        {
            raw_text = exported;
        }

        let req = SubmitGmDocumentRequest {
            title,
            source_kind,
            raw_content: if raw_text.trim().is_empty() {
                None
            } else {
                Some(raw_text)
            },
            google_file_id: if google_id.is_empty() {
                None
            } else {
                Some(google_id)
            },
            google_access_token: google_tok,
        };

        let url = format!("/api/sessions/{sid}/docs");
        match fetch_json::<_, GmDocumentRecord>("POST", &url, Some(&tok), Some(&req)).await {
            Ok(doc) => {
                render_gm_document_added(&doc);
                set_input_value("gm-doc-content", "");
                set_feedback(
                    "gm-doc-feedback",
                    &format!(
                        "Submitted '{}' ({} chars) strictly to GM & AI GM (`GM_ONLY`).",
                        doc.title, doc.char_count
                    ),
                    false,
                );
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Refreshes the list of `GM_ONLY` documents visible to the current user from `GET /api/sessions/:session_id/docs`.
#[wasm_bindgen]
pub fn satya_refresh_gm_documents() {
    spawn_local(async move {
        let (session_id, auth_token) =
            with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            return;
        };
        let url = format!("/api/sessions/{sid}/docs");
        if let Ok(resp) =
            fetch_json::<(), SessionGmDocsResponse>("GET", &url, Some(&tok), None).await
        {
            with_state_mut(|st| {
                st.gm_documents = resp.documents.clone();
            });
            if let Some(list_el) = get_element("gm-docs-list") {
                list_el.set_inner_html(&format_gm_documents_list_html(&resp.documents));
            }
            set_text(
                "gm-docs-count-pill",
                &format!("{} GM Docs", resp.documents.len()),
            );
        }
    });
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
    }
}

/// Searches across all 9 canonical Satya TTRPG Rules Book chapters and renders clickable hits
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
        set_display("rules-search-results", "block");
        return;
    }
    let items: String = hits
        .iter()
        .map(|hit| {
            format!(
                r#"<div class="gm-doc-card rules-search-hit"><div class="gm-doc-header"><button type="button" class="btn-chip" onclick="window.satyaSelectRulesChapter('{}')">{}</button><span class="mono">{}</span></div><div class="gm-doc-preview">{}</div></div>"#,
                escape_html(&hit.chapter_id),
                escape_html(&hit.chapter_title),
                escape_html(&hit.section_heading),
                escape_html(&hit.snippet),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    results_el.set_inner_html(&items);
    set_display("rules-search-results", "block");
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
