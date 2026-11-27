//! 6-Source GM Archivist Omnisearch (`#notebook-pane-docs`) & Vault Action Chips.
//!
//! Queries `GET /api/sessions/:session_id/lookup` (or searches local offline state across Rules Book,
//! GM Docs, GM Memories, and Session NPCs) and renders direct persistence toggle & delete controls
//! on mutable vault hits (`GM Doc`, `GM Memory`, `Session NPC`).

use crate::wasm_client::dom::{
    active_session_and_token_or_feedback, escape_html, escape_js_single_quoted_attr, fetch_json,
    input_value, select_value, set_display, set_html, set_text,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use satya_schema::{GmLookupHit, GmLookupSourceKind, GmOmnisearchResponse, SessionNpcEntry};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Re-runs the GM Archivist omnisearch query if `#gm-omnisearch-results` is currently showing results.
pub(crate) fn refresh_gm_omnisearch_if_active() {
    if with_state(|st| st.gm_omnisearch_active) {
        satya_search_gm_omnisearch();
    }
}

/// Closes `#gm-archivist-modal` if it is currently open for `item_id`.
pub(crate) fn close_archivist_modal_if_matching(item_id: &str) {
    let should_close = with_state(|st| st.archivist_modal_item_id.as_deref() == Some(item_id));
    if should_close {
        satya_close_archivist_modal();
    }
}

fn format_vault_hit_actions_html(
    toggle_cb: &str,
    delete_cb: Option<&str>,
    item_id: &str,
    is_persistent: bool,
) -> String {
    let js_id = escape_js_single_quoted_attr(item_id);
    let next_persist = !is_persistent;
    let toggle_label = if is_persistent {
        "Unmark Persistent"
    } else {
        "Mark Persistent"
    };
    let mut html = format!(
        r#"<button type="button" class="btn-chip" onclick="window.{toggle_cb}('{js_id}', {next_persist})">{toggle_label}</button>"#
    );
    if let Some(del_cb) = delete_cb {
        html.push_str(&format!(
            r#"<button type="button" class="btn-chip btn-chip-danger" onclick="window.{del_cb}('{js_id}')">Delete</button>"#
        ));
    }
    html
}

fn is_markdown_lore_hit(hit: &GmLookupHit) -> bool {
    hit.source_kind == GmLookupSourceKind::CampaignLore
        && !hit.item_id.starts_with("faction-clock:")
        && hit.item_id != "world-operational-heat"
}

/// Formats ranked [`GmOmnisearchResponse`] hits for `#gm-omnisearch-results`.
pub fn format_gm_omnisearch_hits_html(resp: &GmOmnisearchResponse) -> String {
    if resp.hits.is_empty() {
        return format!(
            r#"<span class="pill pill-muted">No matches found for '{}' in scope {}.</span>"#,
            escape_html(&resp.query),
            escape_html(&resp.scope)
        );
    }
    let can_manage = crate::wasm_client::gm_docs::viewer_can_manage_gm_vault();
    resp.hits
        .iter()
        .enumerate()
        .map(|(idx, h)| {
            let kind_badge = match h.source_kind {
                GmLookupSourceKind::RulesBook => "Rules Book",
                GmLookupSourceKind::GmDocument => "GM Doc",
                GmLookupSourceKind::GmMemory => "GM Memory",
                GmLookupSourceKind::CharacterSheet => "Character Sheet",
                GmLookupSourceKind::CombatTracker => "Combat Tracker",
                GmLookupSourceKind::TacticalMap => "Tactical Map",
                GmLookupSourceKind::CampaignLore => "Campaign Lore",
                GmLookupSourceKind::SessionNpc => "Session NPC",
            };
            let is_mutable_vault_hit = matches!(
                h.source_kind,
                GmLookupSourceKind::GmDocument
                    | GmLookupSourceKind::GmMemory
                    | GmLookupSourceKind::SessionNpc
            );
            let persist_pill = if h.is_campaign_default {
                r#" <span class="pill pill-muted" title="Built-in Satya campaign setting resource">Campaign Default</span>"#
            } else if h.is_persistent {
                r#" <span class="pill pill-active">Persistent</span>"#
            } else if is_mutable_vault_hit {
                r#" <span class="pill pill-muted">Session-Only</span>"#
            } else {
                ""
            };
            let js_id = escape_js_single_quoted_attr(&h.item_id);
            let nav_btn = if h.source_kind == GmLookupSourceKind::RulesBook {
                let anchor = escape_js_single_quoted_attr(h.section_anchor.as_deref().unwrap_or(""));
                format!(
                    r#"<button type="button" class="btn-chip" onclick="window.satyaOpenRulesSection('{js_id}', '{anchor}')">Open in Rules &amp; Lore</button>"#
                )
            } else if is_markdown_lore_hit(h) {
                let anchor = escape_js_single_quoted_attr(h.section_anchor.as_deref().unwrap_or(""));
                format!(
                    r#"<button type="button" class="btn-chip" onclick="window.satyaOpenCampaignLoreSection('{js_id}', '{anchor}')">Open in Rules &amp; Lore</button>"#
                )
            } else {
                format!(
                    r#"<button type="button" class="btn-chip" onclick="window.satyaOpenArchivistHit('{idx}')">Inspect Note</button>"#
                )
            };
            let actions_html = match h.source_kind {
                GmLookupSourceKind::GmDocument => {
                    if can_manage || h.can_delete {
                        if can_manage {
                            format_vault_hit_actions_html(
                                "satyaToggleGmDocPersistence",
                                (h.can_delete && !h.is_campaign_default)
                                    .then_some("satyaDeleteGmDocument"),
                                &h.item_id,
                                h.is_persistent,
                            )
                        } else if h.can_delete && !h.is_campaign_default {
                            format!(
                                r#"<button type="button" class="btn-chip btn-chip-danger" onclick="window.satyaDeleteGmDocument('{js_id}')">Delete</button>"#
                            )
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                }
                GmLookupSourceKind::GmMemory if can_manage => format_vault_hit_actions_html(
                    "satyaToggleGmMemoryPersistence",
                    (h.can_delete && !h.is_campaign_default).then_some("satyaDeleteGmMemory"),
                    &h.item_id,
                    h.is_persistent,
                ),
                GmLookupSourceKind::SessionNpc if can_manage => {
                    if h.is_campaign_default || h.subtitle.contains("Not Loaded") {
                        format!(
                            r#"<button type="button" class="btn-chip btn-chip-accent" onclick="window.satyaLoadStarterNpc('{js_id}')">Load into Session</button>"#
                        )
                    } else {
                        format_vault_hit_actions_html(
                            "satyaToggleNpcPersistence",
                            (h.can_delete && !h.is_campaign_default)
                                .then_some("satyaDeleteSessionNpc"),
                            &h.item_id,
                            h.is_persistent,
                        )
                    }
                }
                _ => String::new(),
            };
            let extra_persistent_for_defaults = if h.is_campaign_default && h.is_persistent {
                r#" <span class="pill pill-active">Persistent</span>"#
            } else {
                ""
            };
            format!(
                r#"<div class="gm-doc-card">
                    <div class="gm-doc-header">
                        <button type="button" class="archivist-hit-link" onclick="window.satyaOpenArchivistHit('{idx}')">{}</button>
                        <div class="btn-row-compact">
                            <span class="pill pill-iggy">{kind_badge} · {}</span>{extra_persistent_for_defaults}{persist_pill}{nav_btn}{actions_html}
                        </div>
                    </div>
                    <pre class="gm-doc-body">{}</pre>
                </div>"#,
                escape_html(&h.title),
                escape_html(&h.subtitle),
                escape_html(&h.snippet)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Formats `(title, meta, rendered_body_html, actions_html)` for `#gm-archivist-modal` when a user
/// clicks an Archivist hit (GM Docs, GM Memories, Clocks, NPCs, Character Sheets, Combat, Maps).
#[must_use]
pub fn format_archivist_modal_html(
    hit: &GmLookupHit,
    full_markdown: &str,
    can_manage: bool,
    matched_npc: Option<&SessionNpcEntry>,
) -> (String, String, String, String) {
    let title = hit.title.clone();
    let mut meta = hit.subtitle.clone();
    if hit.is_campaign_default {
        meta.push_str(" · Campaign Default (Read-Only)");
    }
    let body_source = if full_markdown.trim().is_empty() {
        hit.snippet.as_str()
    } else {
        full_markdown
    };
    let rendered = satya_rules_book::render_rules_markdown_to_html(body_source);
    let body_html = if rendered.trim().is_empty() {
        format!(r#"<pre class="gm-doc-body">{}</pre>"#, escape_html(body_source))
    } else {
        rendered
    };

    let js_id = escape_js_single_quoted_attr(&hit.item_id);
    let mut actions = String::new();
    match hit.source_kind {
        GmLookupSourceKind::RulesBook => {
            let anchor = escape_js_single_quoted_attr(hit.section_anchor.as_deref().unwrap_or(""));
            actions.push_str(&format!(
                r#"<button type="button" class="btn btn-sm btn-primary" onclick="window.satyaOpenRulesSection('{js_id}', '{anchor}'); window.satyaCloseArchivistModal()">Jump to Rules &amp; Lore</button>"#
            ));
        }
        GmLookupSourceKind::CampaignLore if is_markdown_lore_hit(hit) => {
            let anchor = escape_js_single_quoted_attr(hit.section_anchor.as_deref().unwrap_or(""));
            actions.push_str(&format!(
                r#"<button type="button" class="btn btn-sm btn-primary" onclick="window.satyaOpenCampaignLoreSection('{js_id}', '{anchor}'); window.satyaCloseArchivistModal()">Jump to Rules &amp; Lore</button>"#
            ));
        }
        GmLookupSourceKind::GmDocument => {
            if can_manage {
                let next_p = !hit.is_persistent;
                let lbl = if hit.is_persistent {
                    "Unmark Persistent"
                } else {
                    "Mark Persistent"
                };
                actions.push_str(&format!(
                    r#"<button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaToggleGmDocPersistence('{js_id}', {next_p})">{lbl}</button>"#
                ));
            }
            if hit.can_delete && !hit.is_campaign_default {
                actions.push_str(&format!(
                    r#"<button type="button" class="btn btn-sm btn-danger" onclick="window.satyaDeleteGmDocument('{js_id}')">Delete Document</button>"#
                ));
            }
        }
        GmLookupSourceKind::GmMemory => {
            if can_manage {
                let next_p = !hit.is_persistent;
                let lbl = if hit.is_persistent {
                    "Unmark Persistent"
                } else {
                    "Mark Persistent"
                };
                actions.push_str(&format!(
                    r#"<button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaToggleGmMemoryPersistence('{js_id}', {next_p})">{lbl}</button>"#
                ));
                if hit.can_delete && !hit.is_campaign_default {
                    actions.push_str(&format!(
                        r#"<button type="button" class="btn btn-sm btn-danger" onclick="window.satyaDeleteGmMemory('{js_id}')">Delete Memory Note</button>"#
                    ));
                }
            }
        }
        GmLookupSourceKind::SessionNpc => {
            if hit.is_campaign_default || hit.subtitle.contains("Not Loaded") {
                if can_manage {
                    actions.push_str(&format!(
                        r#"<button type="button" class="btn btn-sm btn-accent" onclick="window.satyaLoadStarterNpc('{js_id}'); window.satyaCloseArchivistModal()">Load into Session</button>"#
                    ));
                }
            } else {
                if matched_npc.is_some() {
                    actions.push_str(&format!(
                        r#"<button type="button" class="btn btn-sm btn-primary" onclick="window.satyaInspectNpcFromArchivist('{js_id}')">Open in NPC List</button>"#
                    ));
                }
                if can_manage {
                    let next_p = !hit.is_persistent;
                    let lbl = if hit.is_persistent {
                        "Unmark Persistent"
                    } else {
                        "Mark Persistent"
                    };
                    actions.push_str(&format!(
                        r#"<button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaToggleNpcPersistence('{js_id}', {next_p})">{lbl}</button>"#
                    ));
                    if hit.can_delete && !hit.is_campaign_default {
                        actions.push_str(&format!(
                            r#"<button type="button" class="btn btn-sm btn-danger" onclick="window.satyaDeleteSessionNpc('{js_id}')">Remove NPC</button>"#
                        ));
                    }
                }
            }
        }
        _ => {}
    }

    (title, meta, body_html, actions)
}

/// Closes the `#gm-archivist-modal` pop-up overlay.
#[wasm_bindgen]
pub fn satya_close_archivist_modal() {
    with_state_mut(|st| {
        st.archivist_modal_item_id = None;
    });
    set_display("gm-archivist-modal", "none");
}

/// Switches to the `NPC List` tab and selects `npc_id`, closing `#gm-archivist-modal`.
#[wasm_bindgen]
pub fn satya_inspect_npc_from_archivist(npc_id: String) {
    satya_close_archivist_modal();
    with_state_mut(|st| {
        st.selected_npc_id = Some(npc_id);
    });
    crate::wasm_client::notebook::satya_select_notebook_tab("npcs".to_string());
    crate::wasm_client::npcs::render_npcs_tab_dom();
}

/// Opens a GM Archivist hit either directly inside the `Rules & Lore` reader (with anchor scroll &
/// highlight) or in the `#gm-archivist-modal` pop-up for GM Docs, Memory Notes, NPCs, and Clocks.
#[wasm_bindgen]
pub fn satya_open_archivist_hit(idx_str: String) {
    let Ok(idx) = idx_str.trim().parse::<usize>() else {
        return;
    };
    let hit_opt = with_state(|st| st.last_omnisearch_hits.get(idx).cloned());
    let Some(hit) = hit_opt else {
        return;
    };

    if hit.source_kind == GmLookupSourceKind::RulesBook {
        crate::wasm_client::notebook::satya_open_rules_section(
            hit.item_id,
            hit.section_anchor.unwrap_or_default(),
        );
        return;
    }
    if is_markdown_lore_hit(&hit) {
        crate::wasm_client::notebook::satya_open_campaign_lore_section(
            hit.item_id,
            hit.section_anchor.unwrap_or_default(),
        );
        return;
    }

    let can_manage = crate::wasm_client::gm_docs::viewer_can_manage_gm_vault();
    let (full_markdown, matched_npc) = with_state_mut(|st| {
        st.archivist_modal_item_id = Some(hit.item_id.clone());
        let npc_opt = st
            .session_npcs
            .iter()
            .find(|n| n.character_id == hit.item_id)
            .cloned();
        let full_text = match hit.source_kind {
            GmLookupSourceKind::GmDocument => st
                .gm_documents
                .iter()
                .find(|d| d.doc_id == hit.item_id)
                .map(|d| d.sanitized_content.clone())
                .unwrap_or_else(|| hit.snippet.clone()),
            GmLookupSourceKind::GmMemory => st
                .gm_memories
                .iter()
                .find(|m| m.memory_id == hit.item_id)
                .map(|m| m.content.clone())
                .unwrap_or_else(|| hit.snippet.clone()),
            GmLookupSourceKind::SessionNpc => {
                if let Some(ref npc) = npc_opt {
                    let mut md = format!("**{}** — {}\n\n{}", npc.character_name, npc.callsign_or_role, npc.public_summary);
                    if st.viewer_can_see_gm_secrets {
                        if let Some(ref ag) = npc.immediate_agenda {
                            md.push_str(&format!("\n\n**Immediate Agenda:** {ag}"));
                        }
                        if let Some(ref dos) = npc.gm_secret_dossier {
                            md.push_str(&format!("\n\n{dos}"));
                        }
                    }
                    md
                } else {
                    hit.snippet.clone()
                }
            }
            _ => hit.snippet.clone(),
        };
        (full_text, npc_opt)
    });

    let (title, meta, body_html, actions_html) =
        format_archivist_modal_html(&hit, &full_markdown, can_manage, matched_npc.as_ref());
    set_text("gm-archivist-modal-title", &title);
    set_text("gm-archivist-modal-meta", &meta);
    set_html("gm-archivist-modal-body", &body_html);
    set_html("gm-archivist-modal-actions", &actions_html);
    set_display("gm-archivist-modal", "flex");
}

/// Searches across Rules Book chapters, Persistent GM Docs, GM Memories, Character Sheets,
/// Combat Tracker, Tactical Map, Campaign Lore, and Session NPCs via
/// `GET /api/sessions/:session_id/lookup` (with local fallback when offline).
#[wasm_bindgen]
pub fn satya_search_gm_omnisearch() {
    with_state_mut(|st| {
        st.gm_omnisearch_active = true;
    });
    let query = input_value("gm-omnisearch-input").trim().to_string();
    let scope = select_value("gm-omnisearch-scope");
    let norm_scope = if scope.trim().is_empty() {
        "ALL".to_string()
    } else {
        scope.trim().to_uppercase()
    };

    if let Some((sid, tok)) = active_session_and_token_or_feedback("", "") {
        let shielded = with_state(|st| st.solo_spoiler_shield);
        spawn_local(async move {
            let encoded_q: String = js_sys::encode_uri_component(&query).into();
            let encoded_s: String = js_sys::encode_uri_component(&norm_scope).into();
            let shield_q = if shielded { "&spoiler_shield=true" } else { "" };
            let url = format!(
                "/api/sessions/{sid}/lookup?q={encoded_q}&scope={encoded_s}{shield_q}"
            );
            if let Ok(resp) =
                fetch_json::<(), GmOmnisearchResponse>("GET", &url, Some(&tok), None).await
            {
                with_state_mut(|st| {
                    st.last_omnisearch_hits = resp.hits.clone();
                });
                set_display("gm-omnisearch-results", "flex");
                set_html(
                    "gm-omnisearch-results",
                    &format_gm_omnisearch_hits_html(&resp),
                );
            }
        });
        return;
    }

    let (local_docs, local_memories, local_npcs) = with_state(|st| {
        (
            st.gm_documents.clone(),
            st.gm_memories.clone(),
            st.session_npcs.clone(),
        )
    });

    let q_low = query.to_ascii_lowercase();
    let matches_q = |title: &str, body: &str| -> bool {
        q_low.is_empty()
            || title.to_ascii_lowercase().contains(&q_low)
            || body.to_ascii_lowercase().contains(&q_low)
    };
    let mut hits = Vec::new();
    let q_for_rules = if query.is_empty() {
        "cascade"
    } else {
        query.as_str()
    };
    if matches!(norm_scope.as_str(), "ALL" | "RULES" | "RULES_BOOK") {
        for r in satya_rules_book::search_rules_book(q_for_rules, 5) {
            hits.push(GmLookupHit {
                source_kind: GmLookupSourceKind::RulesBook,
                item_id: r.chapter_id,
                title: format!("{} — {}", r.chapter_title, r.section_heading),
                subtitle: format!("Rules Book ({})", r.relative_path),
                snippet: r.snippet.chars().take(360).collect(),
                is_persistent: true,
                score: r.score,
                section_anchor: Some(satya_rules_book::slugify_heading(&r.section_heading)),
                can_delete: false,
                is_campaign_default: true,
            });
        }
    }
    if matches!(norm_scope.as_str(), "ALL" | "DOCS" | "GM_DOC" | "GM_DOCS") {
        for d in &local_docs {
            if matches_q(&d.title, &d.sanitized_content) {
                let is_default_doc = d.doc_id.starts_with("default_")
                    || d.doc_id.starts_with("campaign_default_");
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::GmDocument,
                    item_id: d.doc_id.clone(),
                    title: d.title.clone(),
                    subtitle: format!("GM Doc · {} chars", d.char_count),
                    snippet: d.sanitized_content.chars().take(360).collect(),
                    is_persistent: d.is_persistent,
                    score: 20,
                    section_anchor: None,
                    can_delete: !is_default_doc,
                    is_campaign_default: is_default_doc,
                });
            }
        }
    }
    let include_npcs = matches!(
        norm_scope.as_str(),
        "ALL"
            | "NPC"
            | "NPCS"
            | "SESSION_NPC"
            | "SESSION_NPCS"
            | "CHARACTER"
            | "CHARACTERS"
            | "LORE"
    );
    if matches!(
        norm_scope.as_str(),
        "ALL" | "MEMORY" | "MEMORIES" | "GM_MEMORY"
    ) {
        for m in &local_memories {
            let dup_npc = include_npcs
                && local_npcs.iter().any(|n| {
                    m.memory_id.strip_prefix("mem_") == Some(n.character_id.as_str())
                        || (m.memory_id.starts_with("mem_npc_")
                            && m.title.to_ascii_lowercase().starts_with(&format!(
                                "npc: {}",
                                n.character_name.trim().to_ascii_lowercase()
                            )))
                });
            if !dup_npc && matches_q(&m.title, &m.content) {
                let is_default_mem = m.memory_id.starts_with("default_")
                    || m.memory_id.starts_with("campaign_default_");
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::GmMemory,
                    item_id: m.memory_id.clone(),
                    title: m.title.clone(),
                    subtitle: format!("GM Memory [{}]", m.category.as_str()),
                    snippet: m.content.chars().take(360).collect(),
                    is_persistent: m.is_persistent,
                    score: 25,
                    section_anchor: None,
                    can_delete: !is_default_mem,
                    is_campaign_default: is_default_mem,
                });
            }
        }
    }
    if include_npcs {
        for npc in &local_npcs {
            if matches_q(&npc.character_name, &npc.public_summary) {
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::SessionNpc,
                    item_id: npc.character_id.clone(),
                    title: npc.character_name.clone(),
                    subtitle: format!(
                        "Session NPC · {} · {}",
                        npc.callsign_or_role, npc.apparent_condition
                    ),
                    snippet: npc.public_summary.chars().take(360).collect(),
                    is_persistent: npc.is_persistent,
                    score: 24,
                    section_anchor: None,
                    can_delete: true,
                    is_campaign_default: false,
                });
            }
        }
    }
    let resp = GmOmnisearchResponse {
        session_id: "local".to_string(),
        campaign_tag: "local".to_string(),
        query,
        scope: norm_scope,
        hits: hits.clone(),
    };
    with_state_mut(|st| {
        st.last_omnisearch_hits = hits;
    });
    set_display("gm-omnisearch-results", "flex");
    set_html(
        "gm-omnisearch-results",
        &format_gm_omnisearch_hits_html(&resp),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_gm_omnisearch_hits_html_empty_and_multi_source() {
        let empty = GmOmnisearchResponse {
            session_id: "sector-4".to_string(),
            campaign_tag: "orbital-arc".to_string(),
            query: "quantum".to_string(),
            scope: "ALL".to_string(),
            hits: vec![],
        };
        assert!(
            format_gm_omnisearch_hits_html(&empty)
                .contains("No matches found for 'quantum' in scope ALL.")
        );

        let populated = GmOmnisearchResponse {
            session_id: "sector-4".to_string(),
            campaign_tag: "orbital-arc".to_string(),
            query: "airlock".to_string(),
            scope: "ALL".to_string(),
            hits: vec![
                GmLookupHit {
                    source_kind: GmLookupSourceKind::RulesBook,
                    item_id: "tasks".to_string(),
                    title: "Tasks — Contested Checks".to_string(),
                    subtitle: "docs/ttrpg/tasks.md".to_string(),
                    snippet: "Roll 2d20 against TN_P and TN_GM.".to_string(),
                    is_persistent: true,
                    score: 45,
                    section_anchor: Some("contested-checks".to_string()),
                    is_campaign_default: true,
                    ..Default::default()
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::TacticalMap,
                    item_id: "map-airlock".to_string(),
                    title: "Sector 4 Cargo Airlock".to_string(),
                    subtitle: "20x15 Grid".to_string(),
                    snippet: "Bulkhead door sealed.".to_string(),
                    is_persistent: false,
                    score: 30,
                    ..Default::default()
                },
            ],
        };
        let html = format_gm_omnisearch_hits_html(&populated);
        assert!(html.contains("Rules Book · docs/ttrpg/tasks.md"));
        assert!(html.contains("Persistent"));
        assert!(html.contains("window.satyaOpenRulesSection('tasks', 'contested-checks')"));
        assert!(html.contains("window.satyaOpenArchivistHit('0')"));
        assert!(html.contains("Tactical Map · 20x15 Grid"));
        assert!(html.contains("Bulkhead door sealed."));
    }

    #[test]
    fn test_format_gm_omnisearch_hits_html_vault_persistence_and_delete_controls() {
        let resp = GmOmnisearchResponse {
            session_id: "awakened".to_string(),
            campaign_tag: "awakened".to_string(),
            query: String::new(),
            scope: "ALL".to_string(),
            hits: vec![
                GmLookupHit {
                    source_kind: GmLookupSourceKind::GmDocument,
                    item_id: "doc_p".to_string(),
                    title: "Syndicate Brief".to_string(),
                    subtitle: "TEXT · 42 chars".to_string(),
                    snippet: "Docking bay 4".to_string(),
                    is_persistent: true,
                    score: 25,
                    can_delete: true,
                    ..Default::default()
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::GmDocument,
                    item_id: "doc_s".to_string(),
                    title: "Scratch Note".to_string(),
                    subtitle: "TEXT · 18 chars".to_string(),
                    snippet: "Temp note".to_string(),
                    is_persistent: false,
                    score: 20,
                    can_delete: true,
                    ..Default::default()
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::GmMemory,
                    item_id: "mem_1".to_string(),
                    title: "Vault Passcode".to_string(),
                    subtitle: "GM Memory [WORLD_STATE]".to_string(),
                    snippet: "4419".to_string(),
                    is_persistent: true,
                    score: 25,
                    can_delete: true,
                    ..Default::default()
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::SessionNpc,
                    item_id: "npc_sammat".to_string(),
                    title: "Sammat".to_string(),
                    subtitle: "Session NPC · Broker · Healthy & Alert".to_string(),
                    snippet: "Information broker.".to_string(),
                    is_persistent: true,
                    score: 48,
                    can_delete: true,
                    ..Default::default()
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::SessionNpc,
                    item_id: "starter_npc_0".to_string(),
                    title: "Kaelen".to_string(),
                    subtitle: "Campaign NPC (awakened · Not Loaded) · Smuggler".to_string(),
                    snippet: "Runs cold cargo.".to_string(),
                    is_persistent: false,
                    score: 24,
                    is_campaign_default: true,
                    ..Default::default()
                },
            ],
        };
        let html = format_gm_omnisearch_hits_html(&resp);
        assert!(html.contains("window.satyaToggleGmDocPersistence('doc_p', false)"));
        assert!(html.contains("window.satyaDeleteGmDocument('doc_p')"));
        assert!(html.contains("Session-Only"));
        assert!(html.contains("window.satyaToggleGmDocPersistence('doc_s', true)"));
        assert!(html.contains("window.satyaDeleteGmDocument('doc_s')"));
        assert!(html.contains("window.satyaToggleGmMemoryPersistence('mem_1', false)"));
        assert!(html.contains("window.satyaDeleteGmMemory('mem_1')"));
        assert!(html.contains("window.satyaToggleNpcPersistence('npc_sammat', false)"));
        assert!(html.contains("window.satyaDeleteSessionNpc('npc_sammat')"));
        assert!(html.contains("window.satyaLoadStarterNpc('starter_npc_0')"));
        assert!(html.contains("Campaign Default"));
        assert!(!html.contains("satyaDeleteSessionNpc('starter_npc_0')"));
    }

    #[test]
    fn test_format_archivist_modal_html_renders_note_actions_and_campaign_defaults() {
        let mem_hit = GmLookupHit {
            source_kind: GmLookupSourceKind::GmMemory,
            item_id: "mem_custom".to_string(),
            title: "Clinic Trapdoor".to_string(),
            subtitle: "GM Memory [SCENE_STATE]".to_string(),
            snippet: "Hidden stairs behind **autoclave**.".to_string(),
            is_persistent: true,
            score: 40,
            can_delete: true,
            is_campaign_default: false,
            ..Default::default()
        };
        let (t, meta, body, actions) = format_archivist_modal_html(&mem_hit, "", true, None);
        assert_eq!(t, "Clinic Trapdoor");
        assert!(meta.contains("GM Memory"));
        assert!(body.contains("<strong>autoclave</strong>"));
        assert!(actions.contains("satyaDeleteGmMemory('mem_custom')"));

        let default_lore = GmLookupHit {
            source_kind: GmLookupSourceKind::CampaignLore,
            item_id: "docs/campaign/03-minds-and-factions/major_and_minor_characters.md#2".to_string(),
            title: "Dr. Budi Bandajara".to_string(),
            subtitle: "Campaign Lore (awakened)".to_string(),
            snippet: "Malaysian mathematician.".to_string(),
            is_persistent: true,
            score: 50,
            section_anchor: Some("dr-budi-bandajara".to_string()),
            can_delete: false,
            is_campaign_default: true,
        };
        let (_, meta2, _, actions2) = format_archivist_modal_html(&default_lore, "", true, None);
        assert!(meta2.contains("Campaign Default (Read-Only)"));
        assert!(actions2.contains("satyaOpenCampaignLoreSection("));
        assert!(!actions2.contains("Delete"));
    }
}
