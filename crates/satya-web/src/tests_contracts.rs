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
            if ident.as_bytes().get(5).is_some_and(|c| c.is_ascii_uppercase()) {
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
    let duplicates: Vec<(&str, usize)> = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .collect();
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
        let wiz = wasm_client::CharacterWizardState {
            active_step: step,
            ..Default::default()
        };
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
    combined_html.push_str(&wasm_client::format_ttrpg_skill_dropdown_html("surg", &sheet));
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
    assert!(WASM_BOOTSTRAP_LOADER.contains(r#"["theme", "data-theme", "menu-theme-select", "obsidian"]"#));

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
