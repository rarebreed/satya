//! 6-Source GM Archivist Omnisearch (`#notebook-pane-docs`) & Vault Action Chips.
//!
//! Queries `GET /api/sessions/:session_id/lookup` (or searches local offline state across Rules Book,
//! GM Docs, GM Memories, and Session NPCs) and renders direct persistence toggle & delete controls
//! on mutable vault hits (`GM Doc`, `GM Memory`, `Session NPC`).

use crate::wasm_client::dom::{
    active_session_and_token_or_feedback, escape_html, escape_js_single_quoted_attr, fetch_json,
    input_value, select_value, set_display, set_html,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use satya_schema::{GmLookupHit, GmLookupSourceKind, GmOmnisearchResponse};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Re-runs the GM Archivist omnisearch query if `#gm-omnisearch-results` is currently showing results.
pub(crate) fn refresh_gm_omnisearch_if_active() {
    if with_state(|st| st.gm_omnisearch_active) {
        satya_search_gm_omnisearch();
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
            r#"<button type="button" class="btn-chip" onclick="window.{del_cb}('{js_id}')">Delete</button>"#
        ));
    }
    html
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
        .map(|h| {
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
            let persist_pill = if h.is_persistent {
                r#" <span class="pill pill-active">Persistent</span>"#
            } else if is_mutable_vault_hit {
                r#" <span class="pill pill-muted">Session-Only</span>"#
            } else {
                ""
            };
            let actions_html = if can_manage {
                match h.source_kind {
                    GmLookupSourceKind::GmDocument => format_vault_hit_actions_html(
                        "satyaToggleGmDocPersistence",
                        Some("satyaDeleteGmDocument"),
                        &h.item_id,
                        h.is_persistent,
                    ),
                    GmLookupSourceKind::GmMemory => format_vault_hit_actions_html(
                        "satyaToggleGmMemoryPersistence",
                        Some("satyaDeleteGmMemory"),
                        &h.item_id,
                        h.is_persistent,
                    ),
                    GmLookupSourceKind::SessionNpc => format_vault_hit_actions_html(
                        "satyaToggleNpcPersistence",
                        (!h.subtitle.starts_with("Starter NPC"))
                            .then_some("satyaDeleteSessionNpc"),
                        &h.item_id,
                        h.is_persistent,
                    ),
                    _ => String::new(),
                }
            } else {
                String::new()
            };
            format!(
                r#"<div class="gm-doc-card">
                    <div class="gm-doc-header">
                        <strong>{}</strong>
                        <div class="btn-row-compact">
                            <span class="pill pill-iggy">{kind_badge} · {}</span>{persist_pill}{actions_html}
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
        spawn_local(async move {
            let encoded_q: String = js_sys::encode_uri_component(&query).into();
            let encoded_s: String = js_sys::encode_uri_component(&norm_scope).into();
            let url = format!("/api/sessions/{sid}/lookup?q={encoded_q}&scope={encoded_s}");
            if let Ok(resp) =
                fetch_json::<(), GmOmnisearchResponse>("GET", &url, Some(&tok), None).await
            {
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
            });
        }
    }
    if matches!(norm_scope.as_str(), "ALL" | "DOCS" | "GM_DOC" | "GM_DOCS") {
        for d in &local_docs {
            if matches_q(&d.title, &d.sanitized_content) {
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::GmDocument,
                    item_id: d.doc_id.clone(),
                    title: d.title.clone(),
                    subtitle: format!("GM Doc · {} chars", d.char_count),
                    snippet: d.sanitized_content.chars().take(360).collect(),
                    is_persistent: d.is_persistent,
                    score: 20,
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
                hits.push(GmLookupHit {
                    source_kind: GmLookupSourceKind::GmMemory,
                    item_id: m.memory_id.clone(),
                    title: m.title.clone(),
                    subtitle: format!("GM Memory [{}]", m.category.as_str()),
                    snippet: m.content.chars().take(360).collect(),
                    is_persistent: m.is_persistent,
                    score: 25,
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
                });
            }
        }
    }
    let resp = GmOmnisearchResponse {
        session_id: "local".to_string(),
        campaign_tag: "local".to_string(),
        query,
        scope: norm_scope,
        hits,
    };
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
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::TacticalMap,
                    item_id: "map-airlock".to_string(),
                    title: "Sector 4 Cargo Airlock".to_string(),
                    subtitle: "20x15 Grid".to_string(),
                    snippet: "Bulkhead door sealed.".to_string(),
                    is_persistent: false,
                    score: 30,
                },
            ],
        };
        let html = format_gm_omnisearch_hits_html(&populated);
        assert!(html.contains("Rules Book · docs/ttrpg/tasks.md"));
        assert!(html.contains("Persistent"));
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
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::GmDocument,
                    item_id: "doc_s".to_string(),
                    title: "Scratch Note".to_string(),
                    subtitle: "TEXT · 18 chars".to_string(),
                    snippet: "Temp note".to_string(),
                    is_persistent: false,
                    score: 20,
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::GmMemory,
                    item_id: "mem_1".to_string(),
                    title: "Vault Passcode".to_string(),
                    subtitle: "GM Memory [WORLD_STATE]".to_string(),
                    snippet: "4419".to_string(),
                    is_persistent: true,
                    score: 25,
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::SessionNpc,
                    item_id: "npc_sammat".to_string(),
                    title: "Sammat".to_string(),
                    subtitle: "Session NPC · Broker · Healthy & Alert".to_string(),
                    snippet: "Information broker.".to_string(),
                    is_persistent: true,
                    score: 48,
                },
                GmLookupHit {
                    source_kind: GmLookupSourceKind::SessionNpc,
                    item_id: "starter_npc_0".to_string(),
                    title: "Kaelen".to_string(),
                    subtitle: "Starter NPC · Smuggler".to_string(),
                    snippet: "Runs cold cargo.".to_string(),
                    is_persistent: false,
                    score: 24,
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
        assert!(html.contains("window.satyaToggleNpcPersistence('starter_npc_0', true)"));
        assert!(!html.contains("satyaDeleteSessionNpc('starter_npc_0')"));
    }
}
