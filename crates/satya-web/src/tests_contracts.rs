//! Automated SSR↔Wasm contract tests for `satya-web`.
//!
//! Verifies global DOM `id="..."` uniqueness, `window.satya*` callback registration parity
//! across SSR + dynamic HTML formatters, 13-theme registry parity across Rust/JS/CSS, and
//! `0.25s/tick` combat timing parity.

use super::*;
use std::collections::{BTreeSet, HashMap};

/// Extracts all `id="..."` attribute values from an HTML fragment, ignoring `<script>` and `<style>` blocks.
fn extract_dom_ids_outside_scripts_and_styles(html: &str) -> Vec<String> {
    let mut cleaned = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open_idx) = rest.find("<style").or_else(|| rest.find("<script")) {
        let style_idx = rest.find("<style").unwrap_or(usize::MAX);
        let script_idx = rest.find("<script").unwrap_or(usize::MAX);
        let (idx, close_tag) = if style_idx <= script_idx {
            (style_idx, "</style>")
        } else {
            (script_idx, "</script>")
        };
        let _ = open_idx;
        cleaned.push_str(&rest[..idx]);
        if let Some(end_rel) = rest[idx..].find(close_tag) {
            rest = &rest[idx + end_rel + close_tag.len()..];
        } else {
            rest = "";
            break;
        }
    }
    cleaned.push_str(rest);

    let mut ids = Vec::new();
    let bytes = cleaned.as_bytes();
    let mut i = 0;
    while i + 4 < bytes.len() {
        if &bytes[i..i + 4] == b"id=\"" {
            let prev_ok = i == 0 || matches!(bytes[i - 1], b' ' | b'\n' | b'\r' | b'\t');
            if prev_ok && let Some(end_rel) = cleaned[i + 4..].find('"') {
                ids.push(cleaned[i + 4..i + 4 + end_rel].to_string());
                i += 4 + end_rel + 1;
                continue;
            }
        }
        i += 1;
    }
    ids
}

/// Extracts all `window.satya[A-Za-z0-9_]+` identifiers from an HTML or JS snippet.
fn extract_window_satya_calls(src: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let needle = "window.satya";
    let mut rest = src;
    while let Some(idx) = rest.find(needle) {
        let after = &rest[idx + "window.".len()..];
        let ident_len = after
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        if ident_len > "satya".len() {
            let ident = &after[..ident_len];
            // Only match invoked/referenced callback functions (`satyaFoo`), not internal state fields (`_satya...`)
            if ident
                .as_bytes()
                .get(5)
                .is_some_and(|c| c.is_ascii_uppercase())
            {
                found.insert(ident.to_string());
            }
        }
        rest = &after[ident_len..];
    }
    found
}

#[test]
fn test_ssr_dom_ids_are_globally_unique() {
    let html = render_satya_app_html();
    let ids = extract_dom_ids_outside_scripts_and_styles(&html);
    assert!(
        ids.len() >= 120,
        "expected >= 120 DOM element IDs in SSR shell, found {}",
        ids.len()
    );

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for id in &ids {
        *counts.entry(id.as_str()).or_insert(0) += 1;
    }
    let duplicates: Vec<(&str, usize)> =
        counts.into_iter().filter(|(_, count)| *count > 1).collect();
    assert!(
        duplicates.is_empty(),
        "duplicate DOM element IDs found in SSR HTML: {duplicates:?}"
    );
}

#[test]
fn test_ssr_and_dynamic_window_callbacks_match_registered_bindings() {
    let registered: BTreeSet<&str> = wasm_client::REGISTERED_WINDOW_CALLBACKS
        .iter()
        .copied()
        .collect();
    assert_eq!(
        registered.len(),
        wasm_client::REGISTERED_WINDOW_CALLBACKS.len(),
        "REGISTERED_WINDOW_CALLBACKS must not contain duplicate names"
    );

    let mut combined_html = render_satya_app_html();
    // Append representative outputs from dynamic HTML formatters so inline onclick/onchange
    // callbacks generated at runtime are also verified against REGISTERED_WINDOW_CALLBACKS.
    for step in 1..=6 {
        let mut wiz = wasm_client::CharacterWizardState {
            active_step: step,
            ..Default::default()
        };
        if step == 4 {
            wasm_client::apply_faction_template_to_wizard(&mut wiz, "SATYANG_MANJUSRI");
        }
        combined_html.push('\n');
        combined_html.push_str(&wasm_client::format_character_wizard_html(&wiz));
    }
    let sheet = wasm_client::CharacterSheetState::from_preset("sara_valkyrie");
    for section in ["all", "overview", "skills", "equipment", "anatomy", "notes"] {
        let mut s = sheet.clone();
        s.section_filter = section.to_string();
        combined_html.push('\n');
        combined_html.push_str(&wasm_client::format_character_sheet_html(&s));
    }
    combined_html.push('\n');
    combined_html.push_str(&wasm_client::format_ttrpg_skill_dropdown_html(
        "surg", &sheet,
    ));
    combined_html.push('\n');
    let (rules_html, _) = wasm_client::format_rules_chapter_html("tasks");
    combined_html.push_str(&rules_html);
    let sample_roster = vec![
        satya_schema::Participant::human(
            "u-demo-gm-sean",
            "sean_gm",
            "Sean (GM)",
            satya_schema::SessionRole::Gm,
        ),
        satya_schema::Participant::human(
            "u-demo-sara",
            "sara_p1",
            "Sara (Valkyrie)",
            satya_schema::SessionRole::Player,
        ),
    ];
    let (roster_html, recip_html) = wasm_client::format_roster_and_recipients_html(
        &sample_roster,
        Some("u-demo-gm-sean"),
        &["u-demo-sara".to_string()],
    );
    combined_html.push('\n');
    combined_html.push_str(&roster_html);
    combined_html.push('\n');
    combined_html.push_str(&recip_html);

    let gm_docs_html = crate::wasm_client::gm_docs::format_gm_documents_list_html(&[
        satya_schema::GmDocumentRecord {
            doc_id: "doc_contract".to_string(),
            session_id: "sess-c".to_string(),
            uploader_user_id: "u-gm".to_string(),
            uploader_display_name: "GM".to_string(),
            title: "Brief".to_string(),
            source_kind: satya_schema::GmDocumentSource::DirectTextUpload,
            google_file_id: None,
            visibility: satya_schema::MessageVisibility::GmOnly,
            sanitized_content: "Classified".to_string(),
            char_count: 10,
            safety_flagged: false,
            is_persistent: true,
            campaign_tag: Some("awakened".to_string()),
            created_at_us: 1,
        },
    ]);
    let gm_mems_html =
        crate::wasm_client::gm_memories_and_director::format_gm_memories_list_html(&[
            satya_schema::GmMemoryNoteRecord {
                memory_id: "mem_contract".to_string(),
                session_id: "sess-c".to_string(),
                campaign_tag: "awakened".to_string(),
                author_user_id: "u-gm".to_string(),
                author_display_name: "GM".to_string(),
                category: satya_schema::GmMemoryCategory::SceneState,
                title: "Clock".to_string(),
                content: "Tick".to_string(),
                is_pinned: false,
                is_persistent: true,
                updated_at_us: 1,
            },
        ]);
    let omnisearch_html = crate::wasm_client::gm_omnisearch::format_gm_omnisearch_hits_html(
        &satya_schema::GmOmnisearchResponse {
            session_id: "sess-c".to_string(),
            campaign_tag: "awakened".to_string(),
            query: String::new(),
            scope: "ALL".to_string(),
            hits: vec![
                satya_schema::GmLookupHit {
                    source_kind: satya_schema::GmLookupSourceKind::GmDocument,
                    item_id: "doc_contract".to_string(),
                    title: "Brief".to_string(),
                    subtitle: "TEXT".to_string(),
                    snippet: "Classified".to_string(),
                    is_persistent: true,
                    score: 25,
                },
                satya_schema::GmLookupHit {
                    source_kind: satya_schema::GmLookupSourceKind::GmMemory,
                    item_id: "mem_contract".to_string(),
                    title: "Clock".to_string(),
                    subtitle: "GM Memory [SCENE_STATE]".to_string(),
                    snippet: "Tick".to_string(),
                    is_persistent: true,
                    score: 25,
                },
                satya_schema::GmLookupHit {
                    source_kind: satya_schema::GmLookupSourceKind::SessionNpc,
                    item_id: "npc_contract".to_string(),
                    title: "Sammat".to_string(),
                    subtitle: "Session NPC · Broker · Healthy & Alert".to_string(),
                    snippet: "Broker".to_string(),
                    is_persistent: true,
                    score: 48,
                },
            ],
        },
    );
    let npc_card_html = crate::wasm_client::npcs::format_npc_card_html(
        &satya_schema::SessionNpcEntry {
            character_id: "npc_contract".to_string(),
            character_name: "Sammat".to_string(),
            callsign_or_role: "Broker".to_string(),
            age_years: 34,
            public_summary: "Broker".to_string(),
            apparent_condition: "Healthy & Alert".to_string(),
            is_persistent: true,
            ..Default::default()
        },
        true,
    );
    let combat_sheet = sheet.clone();
    let combat_tracker_html = crate::wasm_client::format_combat_tracker_rich_html(
        &crate::wasm_client::CombatTrackerRenderContext {
            in_session: true,
            round: 1,
            tick: 5,
            combatants: &[
                satya_schema::CombatantTickEntry {
                    entity_id: "u-demo-sara".to_string(),
                    display_name: "Sara (Valkyrie)".to_string(),
                    role_or_faction: "Player".to_string(),
                    ooda_score: 13,
                    next_action_tick: 6,
                    active_stance: Some("PUNCH".to_string()),
                    fatigue_level: 1,
                    wound_shock: 0,
                    grid_x: Some(4),
                    grid_y: Some(6),
                    zone_label: Some("Core".to_string()),
                    visibility: satya_schema::MessageVisibility::Public,
                },
                satya_schema::CombatantTickEntry {
                    entity_id: "npc_contract".to_string(),
                    display_name: "Sammat".to_string(),
                    role_or_faction: "NPC".to_string(),
                    ooda_score: 11,
                    next_action_tick: 10,
                    active_stance: None,
                    fatigue_level: 0,
                    wound_shock: 0,
                    grid_x: None,
                    grid_y: None,
                    zone_label: None,
                    visibility: satya_schema::MessageVisibility::Public,
                },
            ],
            roster: &sample_roster,
            self_participant: sample_roster.get(1),
            character_sheet: &combat_sheet,
            session_npcs: &[satya_schema::SessionNpcEntry {
                character_id: "npc_contract".to_string(),
                character_name: "Sammat".to_string(),
                callsign_or_role: "Broker".to_string(),
                age_years: 34,
                public_summary: "Broker".to_string(),
                apparent_condition: "Healthy & Alert".to_string(),
                is_persistent: true,
                ..Default::default()
            }],
            can_see_gm_secrets: true,
            planned_action: &crate::wasm_client::PlannedCombatActionState::default(),
        },
    );
    let triad_warn_html = crate::wasm_client::evaluate_drawer_triad_preview(
        &crate::wasm_client::DrawerTriadInputs {
            technique_raw: "punch",
            tempo: 25,
            control: 25,
            force: 25,
            skill_c: 12,
            skill_n: 3,
            fatigue_level: 0,
            wound_shock_index: 0.0,
        },
    )
    .drawer_warning_html
    .unwrap_or_default();
    combined_html.push('\n');
    combined_html.push_str(&gm_docs_html);
    combined_html.push('\n');
    combined_html.push_str(&gm_mems_html);
    combined_html.push('\n');
    combined_html.push_str(&omnisearch_html);
    combined_html.push('\n');
    combined_html.push_str(&npc_card_html);
    combined_html.push('\n');
    combined_html.push_str(&combat_tracker_html);
    combined_html.push('\n');
    combined_html.push_str(&triad_warn_html);

    let referenced = extract_window_satya_calls(&combined_html);
    assert!(
        referenced.len() >= 90,
        "expected >= 90 distinct window.satya* callbacks across SSR + dynamic views, found {}",
        referenced.len()
    );
    let unregistered: Vec<&String> = referenced
        .iter()
        .filter(|name| !registered.contains(name.as_str()))
        .collect();
    assert!(
        unregistered.is_empty(),
        "HTML/JS references unregistered window.satya* callbacks: {unregistered:?}"
    );
}

#[test]
fn test_appearance_themes_bootstrap_and_css_parity() {
    let html = render_satya_app_html();
    assert!(
        WASM_BOOTSTRAP_LOADER
            .contains(r#"["theme", "data-theme", "menu-theme-select", "obsidian"]"#)
    );

    for &(theme_id, _label) in wasm_client::APPEARANCE_THEME_OPTIONS {
        assert_eq!(
            wasm_client::normalize_theme_id(theme_id),
            theme_id,
            "normalize_theme_id must accept canonical theme '{theme_id}'"
        );
        assert!(
            html.contains(&format!(r#"id="btn-theme-swatch-{theme_id}""#)),
            "missing swatch button for theme '{theme_id}'"
        );
        assert!(
            html.contains(&format!(r#"value="{theme_id}""#)),
            "missing <option> value for theme '{theme_id}'"
        );
        if theme_id != "obsidian" {
            assert!(
                html.contains(&format!(r#"html[data-theme="{theme_id}"]"#)),
                "missing CSS palette selector html[data-theme=\"{theme_id}\"]"
            );
        }
    }
}

#[test]
fn test_combat_tracker_quarter_second_tick_granularity_and_rules_chapters() {
    assert!((wasm_client::SECONDS_PER_TICK - 0.25).abs() < 1e-9);
    let roster = vec![satya_schema::Participant::human(
        "u-sara",
        "sara_p1",
        "Sara (Valkyrie)",
        satya_schema::SessionRole::Player,
    )];
    let tracker_html = wasm_client::format_combat_tracker_html(&roster, 2, 20);
    assert!(
        tracker_html.contains("Tick 20 (5.00s · W2)"),
        "20 ticks at 0.25s/tick must equal 5.00s in '{tracker_html}'"
    );

    let (chapter_html, chapter_pill) = wasm_client::format_rules_chapter_html("tasks");
    assert!(chapter_html.contains("satya-rules-article"));
    assert!(!chapter_pill.is_empty());
}

#[test]
fn test_workspace_float_controls_and_collapsed_grid_contracts() {
    let html = render_satya_app_html();

    for id in [
        "chat-drag-header",
        "btn-toggle-chat-float",
        "ttrpg-drag-header",
        "btn-toggle-ttrpg-float",
        "webrtc-drag-header",
        "btn-toggle-webcam-float",
    ] {
        assert!(
            html.contains(&format!(r#"id="{id}""#)),
            "missing workspace float DOM id '{id}'"
        );
    }

    for css_fragment in [
        ".workspace-columns.center-collapsed.webcam-collapsed",
        "minmax(0, 1fr) auto",
        ".workspace-columns.center-collapsed.webcam-undocked",
        ".col-center.is-undocked, .col-webcams.is-undocked",
        ".chat-card.is-floating",
        ".ttrpg-card.is-floating",
        ".is-floating > .pane-drag-header",
    ] {
        assert!(
            html.contains(css_fragment),
            "missing CSS layout rule fragment '{css_fragment}'"
        );
    }

    // Verify Action Mode & Triad readout moved to Combat Tracker while Task Roller keeps compact warning
    let combat_pane_idx = html.find(r#"id="notebook-pane-combat""#).unwrap();
    let rules_pane_idx = html.find(r#"id="notebook-pane-rules""#).unwrap();
    let combat_readout_idx = html.find(r#"id="combat-action-mode-readout""#).unwrap();
    assert!(combat_readout_idx > combat_pane_idx && combat_readout_idx < rules_pane_idx);

    let ttrpg_drawer_idx = html.find(r#"id="ttrpg-drawer""#).unwrap();
    let triad_warn_idx = html.find(r#"id="ttrpg-triad-warning""#).unwrap();
    assert!(triad_warn_idx > ttrpg_drawer_idx);
    assert!(!html.contains(r#"id="ttrpg-triad-readout""#));
    assert!(html.contains(r#"id="btn-toggle-ttrpg-odds-table""#));
    assert!(html.contains(r#"class="ttrpg-advanced-control""#));
    assert!(html.contains(r#"id="btn-menu-toggle-advanced-settings""#));
    assert!(html.contains(r#"id="menu-advanced-settings-panel""#));
    assert!(html.contains(r#"id="menu-advanced-roller-controls""#));
    assert!(html.contains(".ttrpg-advanced-control { display: none; }"));
    assert!(html.contains(r#"html[data-advanced-roller="true"] .ttrpg-advanced-control { display: flex; }"#));
}

#[test]
fn test_tactical_map_scale_selector_is_rendered() {
    let html = render_satya_app_html();
    assert!(
        html.contains(r#"id="map-scale-select""#),
        "missing #map-scale-select in tactical map toolbar SSR HTML"
    );
    assert!(
        html.contains(r#"id="map-floor-switcher""#) && html.contains(".map-floor-switcher"),
        "missing #map-floor-switcher container or CSS rule in SSR HTML"
    );
}

#[test]
fn test_phase2_psyche_9_linnea_cho_and_faction_templates() {
    let mut sheet = wasm_client::CharacterSheetState::from_preset("linnea_cho");
    assert_eq!(sheet.character_name, "Linnea Cho");
    assert_eq!(sheet.characteristics.trs, 12);
    assert_eq!(sheet.characteristics.cps, 17);
    assert_eq!(sheet.faction_id.as_deref(), Some("SATYANG"));
    assert_eq!(sheet.faction_archetype.as_deref(), Some("SATYANG_MANJUSRI"));
    let sheet_html = wasm_client::format_character_sheet_html(&sheet);
    assert!(sheet_html.contains("Satyang"));
    assert!(sheet_html.contains("Psyche Profile (9 Bipolar Disposition Traits"));
    assert!(sheet_html.contains("Trust"));
    assert!(sheet_html.contains("Compassion"));

    let record = sheet.to_character_sheet_record("usr-linnea");
    assert!(
        record.sp_budget_valid,
        "violations: {:?}",
        record.audit_violations
    );
    assert!(
        record.dag_prereqs_valid,
        "violations: {:?}",
        record.audit_violations
    );
    let bytes = wasm_client::encode_offline_satyachar_bytes(&record, Some("sess-linnea"))
        .expect("v3 .satyachar encode must succeed");
    let decoded = wasm_client::decode_offline_satyachar_bytes(&bytes)
        .expect("v3 .satyachar decode must succeed");
    assert_eq!(decoded.characteristics.trs, 12);
    assert_eq!(decoded.characteristics.cps, 17);
    assert_eq!(decoded.characteristics.sav, sheet.characteristics.sav);
    assert_eq!(decoded.characteristics.bea, sheet.characteristics.bea);
    assert_eq!(decoded.characteristics.wrm, sheet.characteristics.wrm);
    assert_eq!(decoded.institutional_status, sheet.institutional_status as i32);
    assert_eq!(decoded.faction_id.as_deref(), Some("SATYANG"));
    assert_eq!(
        decoded.faction_archetype.as_deref(),
        Some("SATYANG_MANJUSRI")
    );

    sheet.faction_id = None;
    sheet.faction_archetype = None;
    sheet.apply_character_sheet_record(&decoded);
    assert_eq!(sheet.characteristics.trs, 12);
    assert_eq!(sheet.characteristics.cps, 17);
    assert_eq!(sheet.faction_id.as_deref(), Some("SATYANG"));

    let mut wiz = wasm_client::CharacterWizardState::default();
    let spec = wasm_client::apply_faction_template_to_wizard(&mut wiz, "SATYANG_MANJUSRI")
        .expect("valid faction archetype code");
    assert_eq!(
        wiz.lifepath.faction_archetype.as_deref(),
        Some("SATYANG_MANJUSRI")
    );
    assert_eq!(wiz.characteristics.opn, spec.psyche.opn);
    assert_eq!(wiz.characteristics.cps, spec.psyche.cps);
    wiz.active_step = 2;
    let step2_html = wasm_client::format_character_wizard_html(&wiz);
    assert!(step2_html.contains("The 9 Invisible Behavioral Traits"));
    assert!(step2_html.contains("wiz-psyche-range-trs"));
    assert!(step2_html.contains("wiz-psyche-range-cps"));
    wiz.active_step = 4;
    let step4_html = wasm_client::format_character_wizard_html(&wiz);
    assert!(step4_html.contains(r#"id="wiz-s4-faction-archetype""#));
    assert!(step4_html.contains(r#"id="wiz-s3-youth-hobby""#));
    assert!(step4_html.contains(r#"id="wiz-s3-life-event-choice""#));

    let ssr = render_satya_app_html();
    assert!(ssr.contains(r#"value="linnea_cho""#));
    assert!(ssr.contains(r#"value="SATYANG_MANJUSRI""#));
}

#[test]
fn test_responsive_stacked_layout_cascade_order_and_flex_contracts() {
    let html = render_satya_app_html();

    let ttrpg_chat_collapsed_idx = html
        .find(".workspace-columns:is(.chat-collapsed, .chat-undocked) .ttrpg-card:not(.is-floating)")
        .expect("missing chat-collapsed/undocked ttrpg rule");
    let ws_1240_idx = html
        .find(".workspace-columns.webcam-collapsed {\n        --col-left-track: minmax(360px, 1.35fr);")
        .expect("missing 1240px tightened 2-column webcam-collapsed rule");
    let ws_900_idx = html
        .find("@media (max-width: 900px) {")
        .expect("missing 900px stacked workspace breakpoint");
    assert!(
        ws_1240_idx > ttrpg_chat_collapsed_idx && ws_900_idx > ws_1240_idx,
        "responsive workspace breakpoints must cascade after .ttrpg-card overrides"
    );
    assert!(
        html[ws_1240_idx..ws_900_idx].contains(".workspace-columns.webcam-undocked")
            && html[ws_1240_idx..ws_900_idx].contains("flex: 0 0 auto;"),
        "1240px breakpoint must keep 2-column webcam-undocked and set flex: 0 0 auto on stacked 3-column workspace"
    );

    let nb_card_base_idx = html
        .find("\n.notebook-card {")
        .expect("missing base .notebook-card rule");
    let nb_stages_900_idx = html[nb_card_base_idx..]
        .find("@media (max-width: 900px) {\n    .notebook-card,")
        .map(|off| nb_card_base_idx + off)
        .expect("stacked .notebook-card/@media (max-width: 900px) override must follow base .notebook-card rule");
    let nb_stages_900_block = &html[nb_stages_900_idx..];
    assert!(
        nb_stages_900_block.contains("flex: 0 0 auto;")
            && nb_stages_900_block.contains("height: auto;")
            && nb_stages_900_block.contains("#notebook-pane-maps")
            && nb_stages_900_block.contains("overflow: visible;"),
        "stacked styles_stages.css override must reset flex/height and restore overflow: visible on #notebook-pane-maps"
    );

    let rules_vp_base_idx = html
        .find(".rules-book-viewport {")
        .expect("missing base .rules-book-viewport rule");
    let wiz_stage_base_idx = html
        .find(".wizard-stage-container {")
        .expect("missing base .wizard-stage-container rule");
    let nb_900_idx = html[wiz_stage_base_idx..]
        .find("@media (max-width: 900px) {\n    .rules-book-viewport,")
        .map(|off| wiz_stage_base_idx + off)
        .expect("stacked inner viewport @media (max-width: 900px) override must follow base rules in styles_notebook.css");
    assert!(
        nb_900_idx > rules_vp_base_idx && nb_900_idx > wiz_stage_base_idx,
        "styles_notebook.css stacked override must appear after base viewport declarations"
    );
    assert!(
        html[nb_900_idx..].contains("flex: 0 0 auto;"),
        "stacked inner viewports must reset flex: 0 0 auto"
    );
}
