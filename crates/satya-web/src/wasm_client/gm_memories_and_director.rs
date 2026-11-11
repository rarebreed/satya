//! Persistent GM Memories CRUD, 12-dial Scenario & Campaign Director, and 6-source GM Archivist
//! Omnisearch (`#notebook-pane-docs`).

use crate::wasm_client::dom::{
    active_session_and_token_or_feedback, escape_html, escape_js_single_quoted_attr, fetch_json,
    input_checked, input_value, local_storage_get, local_storage_set, select_value, set_display,
    set_feedback, set_html, set_input_value, set_select_value, set_text,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use satya_schema::{
    GmLookupHit, GmLookupSourceKind, GmMemoryCategory, GmMemoryNoteRecord, GmOmnisearchResponse,
    SessionGmMemoriesResponse, UpsertGmMemoryRequest,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

const STORAGE_KEY_SCENARIO_DIRECTOR: &str = "satya_scenario_director_config";

/// Formats the HTML list of persistent and session GM memory notes for `#gm-memories-list`.
pub fn format_gm_memories_list_html(memories: &[GmMemoryNoteRecord]) -> String {
    if memories.is_empty() {
        return r#"<span class="pill pill-muted">No persistent GM memory notes saved for this campaign/session yet.</span>"#.to_string();
    }
    memories
        .iter()
        .map(|m| {
            let pin_badge = if m.is_pinned {
                r#" <span class="pill pill-active" title="Pinned into AI GM active context header">Pinned to AI GM</span>"#
            } else {
                ""
            };
            let persist_badge = if m.is_persistent {
                format!(
                    r#" <span class="pill pill-iggy">Persistent ({})</span>"#,
                    escape_html(&m.campaign_tag)
                )
            } else {
                r#" <span class="pill pill-muted">Session-Only</span>"#.to_string()
            };
            format!(
                r#"<div class="gm-doc-card">
                    <div class="gm-doc-header">
                        <strong>[{}] {}</strong>
                        <div class="btn-row-compact">
                            {pin_badge}{persist_badge}
                            <button type="button" class="btn-chip" onclick="window.satyaDeleteGmMemory('{}')">Delete</button>
                        </div>
                    </div>
                    <div class="subtitle">Recorded by <strong>{}</strong></div>
                    <pre class="gm-doc-body">{}</pre>
                </div>"#,
                escape_html(m.category.as_str()),
                escape_html(&m.title),
                escape_js_single_quoted_attr(&m.memory_id),
                escape_html(&m.author_display_name),
                escape_html(&m.content)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Creates or updates a persistent GM memory note via `POST /api/sessions/:session_id/memories`.
#[wasm_bindgen]
pub fn satya_submit_gm_memory() {
    spawn_local(async move {
        let Some((sid, tok)) = active_session_and_token_or_feedback(
            "gm-doc-feedback",
            "Join or create an active session first before saving a GM memory note.",
        ) else {
            return;
        };
        let title = input_value("gm-memory-title").trim().to_string();
        let content = input_value("gm-memory-content").trim().to_string();
        let category = GmMemoryCategory::from_code_str(&select_value("gm-memory-category"));
        let is_pinned = input_checked("gm-memory-pinned");
        let is_persistent = input_checked("gm-memory-persistent");
        let ctag_raw = input_value("gm-doc-campaign-tag").trim().to_string();

        let req = UpsertGmMemoryRequest {
            memory_id: None,
            campaign_tag: if ctag_raw.is_empty() {
                None
            } else {
                Some(ctag_raw)
            },
            category,
            title,
            content,
            is_pinned,
            is_persistent,
        };
        let url = format!("/api/sessions/{sid}/memories");
        match fetch_json::<_, GmMemoryNoteRecord>("POST", &url, Some(&tok), Some(&req)).await {
            Ok(mem) => {
                set_input_value("gm-memory-content", "");
                set_feedback(
                    "gm-doc-feedback",
                    &format!(
                        "Saved GM memory note '{}' [{}].",
                        mem.title,
                        mem.category.as_str()
                    ),
                    false,
                );
                satya_refresh_gm_memories();
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Deletes a GM memory note via `DELETE /api/sessions/:session_id/memories/:memory_id`.
#[wasm_bindgen]
pub fn satya_delete_gm_memory(memory_id: String) {
    spawn_local(async move {
        let Some((sid, tok)) = active_session_and_token_or_feedback("", "") else {
            return;
        };
        let url = format!("/api/sessions/{sid}/memories/{memory_id}");
        if fetch_json::<(), serde_json::Value>("DELETE", &url, Some(&tok), None)
            .await
            .is_ok()
        {
            satya_refresh_gm_memories();
        }
    });
}

/// Canonical 12 TTRPG Scenario Director event sub-routine dials (`(slug, label, pillar_hint, default_weight)`).
pub const SCENARIO_DIRECTOR_DIAL_SPECS: &[(&str, &str, &str, u32)] = &[
    (
        "combat_encounter",
        "Combat & Ambush",
        "Tactical / Action",
        20,
    ),
    (
        "mysterious_stranger",
        "Mysterious Stranger",
        "Intrigue / Espionage",
        15,
    ),
    (
        "new_npc_arrives",
        "New NPC Arrives",
        "Social / Dynamic Scene",
        15,
    ),
    (
        "dramatic_revelation",
        "Dramatic Revelation",
        "Drama / Plot Twist",
        15,
    ),
    (
        "thought_provoking_discussion",
        "Thought-Provoking Discussion",
        "Philosophy / 7 Psyche Traits",
        10,
    ),
    (
        "discovery_or_clue",
        "Discovery or Clue",
        "Investigation / Lore",
        10,
    ),
    (
        "environmental_hazard",
        "Environmental Hazard",
        "Survival / Exploration",
        10,
    ),
    (
        "ticking_clock_escalation",
        "Ticking Clock Escalation",
        "Pacing / Tension",
        10,
    ),
    (
        "heist_or_infiltration_complication",
        "Heist / Stealth Complication",
        "Infiltration / Improv",
        10,
    ),
    (
        "moral_or_faction_dilemma",
        "Moral / Faction Dilemma",
        "Politics / Factions",
        10,
    ),
    (
        "resource_or_gear_windfall",
        "Resource / Gear Windfall",
        "Loot / Opportunity",
        5,
    ),
    (
        "downtime_campfire_moment",
        "Downtime / Campfire Beat",
        "Character Bonding",
        5,
    ),
];

/// Updates the percentage readouts (`#scenario-dial-val-{slug}`) and normalized share summary pill
/// (`#scenario-dials-summary-pill`) whenever a Scenario Director probability slider or preset changes.
#[wasm_bindgen]
pub fn satya_update_scenario_dial_readouts() {
    let mut weights: Vec<(&str, &str, u32)> =
        Vec::with_capacity(SCENARIO_DIRECTOR_DIAL_SPECS.len());
    let mut total: u32 = 0;
    for &(slug, label, _, default_w) in SCENARIO_DIRECTOR_DIAL_SPECS {
        let id = format!("scenario-dial-{slug}");
        let w = input_value(&id)
            .parse::<u32>()
            .unwrap_or(default_w)
            .min(100);
        total += w;
        weights.push((slug, label, w));
    }
    let mut top_active: Vec<String> = Vec::new();
    for (slug, label, w) in &weights {
        let share_pct = if total > 0 {
            (*w as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let readout_id = format!("scenario-dial-val-{slug}");
        if *w == 0 {
            set_text(&readout_id, "0 (Off)");
        } else {
            set_text(&readout_id, &format!("{w} ({share_pct:.0}%)"));
            top_active.push(format!("{label}: {share_pct:.0}%"));
        }
    }
    let prob = input_value("scenario-beat-prob")
        .parse::<u32>()
        .unwrap_or(35)
        .min(100);
    let interval = input_value("scenario-beat-interval")
        .parse::<u32>()
        .unwrap_or(3)
        .max(1);
    set_text("scenario-beat-prob-val", &format!("{prob}% / turn"));
    let summary = if top_active.is_empty() {
        format!("All Emergent Dials Off (0%) · Max Interval: {interval} turns")
    } else {
        let preview = top_active
            .into_iter()
            .take(3)
            .collect::<Vec<_>>()
            .join(" · ");
        format!("Beat Odds: {prob}%/turn (Cap {interval}t) · Top: {preview}")
    };
    set_text("scenario-dials-summary-pill", &summary);
}

/// Applies a quick Scenario Director probability dial preset (`"balanced"`, `"combat_heavy"`,
/// `"intrigue_drama"`, or `"investigation_heist"`).
#[wasm_bindgen]
pub fn satya_apply_scenario_preset(preset: String) {
    let p = preset.trim().to_lowercase();
    for &(slug, _, _, default_w) in SCENARIO_DIRECTOR_DIAL_SPECS {
        let w = match p.as_str() {
            "combat_heavy" => match slug {
                "combat_encounter" => 50,
                "ticking_clock_escalation" => 20,
                "environmental_hazard" => 15,
                "heist_or_infiltration_complication" => 15,
                "resource_or_gear_windfall" => 10,
                "thought_provoking_discussion" | "downtime_campfire_moment" => 0,
                _ => 5,
            },
            "intrigue_drama" => match slug {
                "dramatic_revelation" => 35,
                "mysterious_stranger" => 30,
                "moral_or_faction_dilemma" => 25,
                "new_npc_arrives" => 20,
                "thought_provoking_discussion" => 20,
                "combat_encounter" => 5,
                _ => 10,
            },
            "investigation_heist" => match slug {
                "discovery_or_clue" => 35,
                "heist_or_infiltration_complication" => 35,
                "mysterious_stranger" => 25,
                "ticking_clock_escalation" => 20,
                "thought_provoking_discussion" => 0,
                _ => 10,
            },
            _ => default_w,
        };
        set_input_value(&format!("scenario-dial-{slug}"), &w.to_string());
    }
    satya_update_scenario_dial_readouts();
}

/// Applies a serialized `mem-scenario-director` JSON string onto the Scenario Designer DOM controls.
pub fn apply_scenario_director_json_to_dom(raw_json: &str) {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(raw_json) else {
        return;
    };
    if let Some(fm) = val.get("flow_mode").and_then(|v| v.as_str()) {
        set_select_value("scenario-flow-mode", fm);
    }
    if let Some(premise) = val.get("campaign_premise").and_then(|v| v.as_str()) {
        set_input_value("scenario-premise", premise);
    }
    if let Some(beats) = val.get("planned_beats").and_then(|v| v.as_array()) {
        let joined = beats
            .iter()
            .filter_map(|b| b.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        set_input_value("scenario-planned-beats", &joined);
    }
    if let Some(prob) = val
        .get("beat_probability_per_turn")
        .and_then(|v| v.as_f64())
    {
        let pct = (prob * 100.0).round().clamp(0.0, 100.0) as u32;
        set_input_value("scenario-beat-prob", &pct.to_string());
    }
    if let Some(interval) = val.get("beat_interval_turns").and_then(|v| v.as_u64()) {
        set_input_value("scenario-beat-interval", &interval.to_string());
    }
    if let Some(weights) = val.get("event_weights").and_then(|v| v.as_object()) {
        for &(slug, _, _, _) in SCENARIO_DIRECTOR_DIAL_SPECS {
            if let Some(w) = weights.get(slug).and_then(|v| v.as_f64()) {
                let clamped = w.round().clamp(0.0, 100.0) as u32;
                set_input_value(&format!("scenario-dial-{slug}"), &clamped.to_string());
            }
        }
    }
    satya_update_scenario_dial_readouts();
}

/// Serializes the Scenario & Campaign Director UI controls into a JSON configuration string.
pub fn build_scenario_director_config_json(
    flow_mode: &str,
    campaign_premise: &str,
    planned_beats_raw: &str,
    beat_prob_pct: u32,
    beat_interval_turns: u32,
    event_weights: &[(String, f64)],
) -> String {
    let planned_beats: Vec<String> = planned_beats_raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(ToString::to_string)
        .collect();
    let mut weights_map = serde_json::Map::new();
    let mut player_desires = Vec::new();
    for (slug, w) in event_weights {
        weights_map.insert(slug.clone(), serde_json::json!(*w));
        if *w > 0.0 {
            player_desires.push(slug.clone());
        }
    }
    serde_json::json!({
        "flow_mode": if flow_mode.trim().is_empty() { "hybrid" } else { flow_mode.trim() },
        "campaign_premise": campaign_premise.trim(),
        "planned_beats": planned_beats,
        "player_desires": player_desires,
        "beat_probability_per_turn": (beat_prob_pct.min(100) as f64) / 100.0,
        "beat_interval_turns": beat_interval_turns.clamp(1, 20),
        "event_weights": weights_map,
    })
    .to_string()
}

/// Saves the Scenario & Campaign Director configuration (`flow_mode`, `campaign_premise`,
/// `planned_beats`, `beat_probability_per_turn`, `beat_interval_turns`, and all 12 `event_weights` dials)
/// to browser `localStorage` and (when in an active session) persists it as pinned GM Memory
/// `mem-scenario-director` in the SQLite GM Vault (`POST /api/sessions/:session_id/memories`).
#[wasm_bindgen]
pub fn satya_save_scenario_director_config() {
    let flow_mode = select_value("scenario-flow-mode");
    let premise = input_value("scenario-premise");
    let planned_raw = input_value("scenario-planned-beats");
    let prob_pct = input_value("scenario-beat-prob")
        .parse::<u32>()
        .unwrap_or(35)
        .min(100);
    let interval = input_value("scenario-beat-interval")
        .parse::<u32>()
        .unwrap_or(3)
        .clamp(1, 20);
    let is_persistent = input_checked("scenario-persistent");
    let ctag_raw = input_value("gm-doc-campaign-tag").trim().to_string();

    let mut weights = Vec::with_capacity(SCENARIO_DIRECTOR_DIAL_SPECS.len());
    for &(slug, _, _, default_w) in SCENARIO_DIRECTOR_DIAL_SPECS {
        let w = input_value(&format!("scenario-dial-{slug}"))
            .parse::<f64>()
            .unwrap_or(default_w as f64)
            .clamp(0.0, 100.0);
        weights.push((slug.to_string(), w));
    }
    satya_update_scenario_dial_readouts();
    let content_json = build_scenario_director_config_json(
        &flow_mode,
        &premise,
        &planned_raw,
        prob_pct,
        interval,
        &weights,
    );

    local_storage_set(STORAGE_KEY_SCENARIO_DIRECTOR, &content_json);

    spawn_local(async move {
        let Some((sid, tok)) = active_session_and_token_or_feedback("", "") else {
            set_feedback(
                "gm-doc-feedback",
                "Saved Scenario Director configuration & event probability dials locally (`localStorage`). Join a session to sync to the SQLite GM Vault.",
                false,
            );
            return;
        };

        let req = UpsertGmMemoryRequest {
            memory_id: Some("mem-scenario-director".to_string()),
            campaign_tag: if ctag_raw.is_empty() {
                None
            } else {
                Some(ctag_raw)
            },
            category: GmMemoryCategory::QuestHook,
            title: "Scenario Director & Campaign Pacing Guide".to_string(),
            content: content_json,
            is_pinned: true,
            is_persistent,
        };
        let url = format!("/api/sessions/{sid}/memories");
        match fetch_json::<_, GmMemoryNoteRecord>("POST", &url, Some(&tok), Some(&req)).await {
            Ok(_) => {
                set_feedback(
                    "gm-doc-feedback",
                    "Saved & pinned Scenario Director configuration (`mem-scenario-director`) to the Campaign/Session SQLite GM Vault.",
                    false,
                );
                satya_refresh_gm_memories();
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Refreshes the list of persistent & session GM memory notes from `GET /api/sessions/:session_id/memories`.
#[wasm_bindgen]
pub fn satya_refresh_gm_memories() {
    spawn_local(async move {
        let Some((sid, tok)) = active_session_and_token_or_feedback("", "") else {
            if let Some(saved) = local_storage_get(STORAGE_KEY_SCENARIO_DIRECTOR) {
                apply_scenario_director_json_to_dom(&saved);
            }
            return;
        };
        let url = format!("/api/sessions/{sid}/memories");
        if let Ok(resp) =
            fetch_json::<(), SessionGmMemoriesResponse>("GET", &url, Some(&tok), None).await
        {
            if let Some(dir_mem) = resp.memories.iter().find(|m| {
                m.memory_id == "mem-scenario-director"
                    || m.title == "Scenario Director & Campaign Pacing Guide"
            }) {
                apply_scenario_director_json_to_dom(&dir_mem.content);
            }
            with_state_mut(|st| {
                st.gm_memories = resp.memories.clone();
            });
            set_html(
                "gm-memories-list",
                &format_gm_memories_list_html(&resp.memories),
            );
            set_text(
                "gm-memories-count-pill",
                &format!("{} GM Memories", resp.memories.len()),
            );
        }
    });
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
            };
            let persist_pill = if h.is_persistent {
                r#" <span class="pill pill-active">Persistent</span>"#
            } else {
                ""
            };
            format!(
                r#"<div class="gm-doc-card">
                    <div class="gm-doc-header">
                        <strong>{}</strong>
                        <span class="pill pill-iggy">{kind_badge} · {}</span>{persist_pill}
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
/// Combat Tracker, and Tactical Map via `GET /api/sessions/:session_id/lookup` (with local fallback when offline).
#[wasm_bindgen]
pub fn satya_search_gm_omnisearch() {
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
                set_html("gm-omnisearch-results", &format_gm_omnisearch_hits_html(&resp));
            }
        });
        return;
    }

    let (local_docs, local_memories) =
        with_state(|st| (st.gm_documents.clone(), st.gm_memories.clone()));

    // Immediate local fallback when searching before joining a session
    let mut hits = Vec::new();
    let q_for_rules = if query.is_empty() {
        "cascade"
    } else {
        query.as_str()
    };
    if matches!(norm_scope.as_str(), "ALL" | "RULES" | "RULES_BOOK") {
        for r in satya_engine::search_rules_book(q_for_rules, 5) {
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
    for d in &local_docs {
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
    for m in &local_memories {
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
    let resp = GmOmnisearchResponse {
        session_id: "local".to_string(),
        campaign_tag: "local".to_string(),
        query,
        scope: norm_scope,
        hits,
    };
    set_display("gm-omnisearch-results", "flex");
    set_html("gm-omnisearch-results", &format_gm_omnisearch_hits_html(&resp));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_gm_memories_list_html_empty_and_pinned_notes() {
        assert!(format_gm_memories_list_html(&[]).contains("No persistent GM memory notes"));

        let notes = vec![GmMemoryNoteRecord {
            memory_id: "mem_vance's_1".to_string(),
            session_id: "sector-4".to_string(),
            campaign_tag: "orbital-arc".to_string(),
            author_user_id: "u-sean".to_string(),
            author_display_name: "Sean (GM)".to_string(),
            category: GmMemoryCategory::NpcSecret,
            title: "Vance's Override Key".to_string(),
            content: "Hidden inside EVA helmet lining.".to_string(),
            is_pinned: true,
            is_persistent: true,
            updated_at_us: 1_793_644_935_000_000,
        }];
        let html = format_gm_memories_list_html(&notes);
        assert!(html.contains("[NPC_SECRET] Vance&#39;s Override Key"));
        assert!(html.contains("Pinned to AI GM"));
        assert!(html.contains("Persistent (orbital-arc)"));
        assert!(html.contains("window.satyaDeleteGmMemory('mem_vance\\'s_1')"));
        assert!(html.contains("Hidden inside EVA helmet lining."));
    }

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
}
