//! `NPC List` Tab (`#notebook-pane-npcs`) & Prompt-/Template-Driven Quick NPC Generator
//! for Human and AI Game Masters, with Role-Aware Player vs. GM/Solo Visibility.

use crate::wasm_client::character_sheet::render_character_sheet;
use crate::wasm_client::chat_and_ttrpg::append_system_notice;
use crate::wasm_client::dom::{
    active_session_and_token_or_feedback, escape_html, escape_js_single_quoted_attr, fetch_json,
    get_element, input_checked, input_value, select_value, set_display, set_feedback, set_html,
    set_text,
};
use crate::wasm_client::gm_memories_and_director::{
    satya_refresh_gm_memories, sync_local_gm_memories_dom,
};
use crate::wasm_client::gm_omnisearch::refresh_gm_omnisearch_if_active;
use crate::wasm_client::notebook::satya_select_notebook_tab;
use crate::wasm_client::state::{with_state, with_state_mut};
use satya_campaign::{
    FactionArchetypeId, NpcGenerationParams, NpcTemplateKind, generate_npc_from_prompt,
};
use satya_schema::{
    GenerateNpcRequest, GenerateNpcResponse, Participant, ParticipantKind, SessionNpcEntry,
    SessionNpcsResponse, SessionRole, UpdateSessionNpcRequest,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Pure role-and-roster policy determining whether a viewer can inspect GM-only NPC secrets.
/// Returns `true` when offline (`session_id.is_none()`), or when the viewer is `GM`/`Host`/`AiGm`;
/// always returns `false` for `Observer` or for a non-GM `Player` when a Human GM is in the roster;
/// otherwise defers to server-provided `fallback` (`viewer_can_see_gm_secrets`).
#[must_use]
pub fn can_viewer_see_npc_secrets(
    session_id: Option<&str>,
    self_part: Option<&Participant>,
    roster: &[Participant],
    fallback: bool,
) -> bool {
    if session_id.is_none() {
        return true;
    }
    if let Some(me) = self_part {
        if me.role == SessionRole::Observer {
            return false;
        }
        if matches!(
            me.role,
            SessionRole::Gm | SessionRole::Host | SessionRole::AiGm
        ) {
            return true;
        }
    }
    if roster
        .iter()
        .any(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm)
    {
        return false;
    }
    fallback
}

/// Determines whether the local viewer is allowed to see GM-only NPC secrets, Psyche traits, and full skill ranks.
#[must_use]
pub fn viewer_can_see_npc_gm_secrets() -> bool {
    with_state(|st| {
        can_viewer_see_npc_secrets(
            st.session_id.as_deref(),
            st.self_participant.as_ref(),
            &st.roster,
            st.viewer_can_see_gm_secrets,
        )
    })
}

fn ensure_default_demo_npc_if_empty() {
    let should_seed = with_state(|st| st.session_id.is_none() && st.session_npcs.is_empty());
    if !should_seed {
        return;
    }
    let gen_res = generate_npc_from_prompt(&NpcGenerationParams {
        prompt: "Terry is a kind general MD, but needs to feel like she belongs".to_string(),
        name_override: None,
        template: NpcTemplateKind::Auto,
        faction_archetype: None,
        career_terms: 2,
        seed: 42,
        owner_user_id: "gm_npc".to_string(),
    });
    let psyche_summary = gen_res
        .psyche_traits
        .iter()
        .map(|p| format!("{} {} ({})", p.code, p.score, p.descriptor))
        .collect();
    let top_skills = gen_res
        .top_skills
        .iter()
        .map(|(_, name, rank)| format!("{name} (N={rank})"))
        .collect();
    let demo_entry = SessionNpcEntry {
        character_id: gen_res.character.character_id.clone(),
        character_name: gen_res.character.character_name.clone(),
        callsign_or_role: gen_res.character.callsign_or_role.clone(),
        age_years: gen_res.character.age_years,
        public_summary: gen_res.public_summary,
        apparent_condition: "Healthy & Alert".to_string(),
        is_persistent: true,
        gm_character: Some(gen_res.character),
        psyche_summary,
        top_skills,
        gm_secret_dossier: Some(gen_res.gm_secret_dossier),
        ..Default::default()
    };
    with_state_mut(|st| {
        st.selected_npc_id = Some(demo_entry.character_id.clone());
        st.session_npcs.push(demo_entry);
    });
}

/// Pure-Rust HTML formatter for the selected NPC card in the `NPC List` tab (`#npc-selected-card-container`).
///
/// When `can_see_gm_secrets == false` (Player view in a human-GM session), only the public fields
/// (`character_name`, `callsign_or_role`, `age_years`, `apparent_condition`, `public_summary`) are rendered.
/// When `can_see_gm_secrets == true` (GM or Solo mode), the 9 bipolar Psyche traits, key characteristics,
/// top trained skills, and GM-only secret dossier are also displayed.
#[must_use]
pub fn format_npc_card_html(npc: &SessionNpcEntry, can_see_gm_secrets: bool) -> String {
    let persist_badge = if npc.is_persistent {
        r#"<span class="pill pill-accent">Persistent Campaign NPC</span>"#
    } else {
        r#"<span class="pill pill-muted">Session NPC</span>"#
    };
    let gm_controls_html = if can_see_gm_secrets {
        let js_id = escape_js_single_quoted_attr(&npc.character_id);
        let next_persist = !npc.is_persistent;
        let toggle_label = if npc.is_persistent {
            "Unmark Persistent"
        } else {
            "Mark Persistent"
        };
        format!(
            r#"<button type="button" class="btn-chip" onclick="window.satyaToggleNpcPersistence('{js_id}', {next_persist})">{toggle_label}</button><button type="button" class="btn-chip" onclick="window.satyaDeleteSessionNpc('{js_id}')">Delete NPC</button>"#
        )
    } else {
        String::new()
    };
    let cond_cls = if npc.apparent_condition.contains("Critically") {
        "pill pill-danger"
    } else if npc.apparent_condition.contains("Fatigued") {
        "pill pill-iggy"
    } else {
        "pill pill-active"
    };

    let mut html = format!(
        r#"<div class="gm-doc-card">
            <div class="gm-doc-header">
                <div>
                    <strong>{}</strong> <span class="muted">· {} (Age {})</span>
                </div>
                <div class="btn-row-compact">
                    <span class="{cond_cls}">{}</span>
                    {persist_badge}
                    {gm_controls_html}
                </div>
            </div>
            <div class="gm-doc-meta">
                <strong>Public Appearance &amp; Demeanor (Visible to All Players):</strong><br/>
                {}
            </div>"#,
        escape_html(&npc.character_name),
        escape_html(&npc.callsign_or_role),
        npc.age_years,
        escape_html(&npc.apparent_condition),
        escape_html(&npc.public_summary),
    );

    if can_see_gm_secrets {
        const BALANCED: &str = "Grounded Pragmatic Cooperative Ambiverted Adaptive Adaptable Measured Discerning Humane";
        let psyche_chips = if npc.psyche_summary.is_empty() {
            r#"<span class="pill pill-muted">Balanced Psyche</span>"#.to_string()
        } else {
            npc.psyche_summary
                .iter()
                .map(|p| {
                    let cls = if BALANCED.split(' ').any(|w| p.contains(w)) {
                        "pill pill-muted"
                    } else {
                        "pill pill-accent"
                    };
                    format!(r#"<span class="{cls}">{}</span>"#, escape_html(p))
                })
                .collect::<Vec<_>>()
                .join(" ")
        };

        let skills_chips = if npc.top_skills.is_empty() {
            r#"<span class="pill pill-muted">Standard Lifepath Baseline</span>"#.to_string()
        } else {
            npc.top_skills
                .iter()
                .map(|s| format!(r#"<span class="pill pill-iggy">{}</span>"#, escape_html(s)))
                .collect::<Vec<_>>()
                .join(" ")
        };

        let stats_line = npc.gm_character.as_ref().map_or_else(
            || "Stats loaded in GM Vault".to_string(),
            |ch| {
                let c = &ch.characteristics;
                [
                    format!(
                        "SPD {} · FOR {} · DEX {} · HLT {} · FIT {}",
                        c.spd, c.for_stat, c.dex, c.hlt, c.fit
                    ),
                    format!(
                        "MEM {} · ANA {} · INS {} · DIS {} · FOC {}",
                        c.mem, c.ana, c.ins, c.dis, c.foc
                    ),
                    format!("COM {} · PRE {} · ELO {}", c.com, c.pre, c.elo),
                ]
                .join(" | ")
            },
        );

        let dossier_text = npc
            .gm_secret_dossier
            .as_deref()
            .unwrap_or("No additional GM secret notes.");

        html.push_str(&format!(
            r#"<div class="gm-doc-meta" style="margin-top:8px; border-top:1px solid rgba(148,163,184,0.2); padding-top:8px;">
                <div><strong>GM / Solo Only — 9 Bipolar Psyche Traits (1–20):</strong></div>
                <div class="roster-chips" style="margin:4px 0 8px 0;">{psyche_chips}</div>
                <div><strong>GM / Solo Only — Key Characteristics (1–20):</strong> <span class="mono">{stats_line}</span></div>
                <div style="margin-top:6px;"><strong>GM / Solo Only — Top Trained Skills:</strong></div>
                <div class="roster-chips" style="margin:4px 0 8px 0;">{skills_chips}</div>
                <div><strong>GM / Solo Only — Secret Dossier &amp; Notes:</strong></div>
                <pre class="gm-doc-body">{}</pre>
                <div class="btn-row" style="margin-top:6px;">
                    <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaLoadSelectedNpcToSheet()">
                        Open Full NPC Sheet in Character Tab
                    </button>
                </div>
            </div>"#,
            escape_html(dossier_text)
        ));
    } else {
        html.push_str(
            r#"<div class="gm-doc-meta" style="margin-top:6px;">
                <span class="pill pill-muted">Player View: GM-only Characteristics, Psyche scores, Skill ranks, and Secret Dossier are hidden.</span>
            </div>"#,
        );
    }

    html.push_str("</div>");
    html
}

/// Renders the NPC selector dropdown (`#npc-roster-select`), visibility badge (`#npc-list-visibility-badge`),
/// generator panel (`#npc-generator-panel`), and selected NPC card (`#npc-selected-card-container`).
pub fn render_npcs_tab_dom() {
    ensure_default_demo_npc_if_empty();
    let can_see_gm = viewer_can_see_npc_gm_secrets();

    set_display(
        "npc-generator-panel",
        if can_see_gm { "block" } else { "none" },
    );
    set_text(
        "npc-list-visibility-badge",
        if can_see_gm {
            "GM / Solo View (Full Stats, Psyche & Secrets Visible)"
        } else {
            "Player View (GM Secrets Hidden)"
        },
    );

    let (npcs, selected_id) = with_state_mut(|st| {
        if st.selected_npc_id.is_none()
            || !st
                .session_npcs
                .iter()
                .any(|n| Some(&n.character_id) == st.selected_npc_id.as_ref())
        {
            st.selected_npc_id = st.session_npcs.first().map(|n| n.character_id.clone());
        }
        (st.session_npcs.clone(), st.selected_npc_id.clone())
    });

    set_text("npc-roster-count-pill", &format!("{} NPCs", npcs.len()));

    let mut opts_html = String::new();
    for n in &npcs {
        let is_sel = Some(&n.character_id) == selected_id.as_ref();
        opts_html.push_str(&format!(
            r#"<option value="{}"{}>{} — {} (Age {})</option>"#,
            escape_html(&n.character_id),
            if is_sel { " selected" } else { "" },
            escape_html(&n.character_name),
            escape_html(&n.callsign_or_role),
            n.age_years
        ));
    }
    set_html("npc-roster-select", &opts_html);

    if get_element("npc-selected-card-container").is_some() {
        let card_html = if let Some(active_npc) = npcs
            .iter()
            .find(|n| Some(&n.character_id) == selected_id.as_ref())
            .or_else(|| npcs.first())
        {
            format_npc_card_html(active_npc, can_see_gm)
        } else {
            r#"<span class="pill pill-muted">No NPCs in this session yet.</span>"#.to_string()
        };
        set_html("npc-selected-card-container", &card_html);
    }
}

/// Refreshes the `NPC List` tab from the server (`GET /api/sessions/:session_id/npcs`) if connected,
/// and updates the DOM.
#[wasm_bindgen]
pub fn satya_refresh_npcs_tab() {
    render_npcs_tab_dom();
    crate::wasm_client::notebook::render_combat_tracker();

    let Some((sid, tok)) = active_session_and_token_or_feedback("", "") else {
        return;
    };

    spawn_local(async move {
        let url = format!("/api/sessions/{sid}/npcs");
        if let Ok(resp) = fetch_json::<(), SessionNpcsResponse>("GET", &url, Some(&tok), None).await
        {
            with_state_mut(|st| {
                st.viewer_can_see_gm_secrets = resp.viewer_can_see_gm_secrets;
                st.session_npcs = resp.npcs;
            });
            render_npcs_tab_dom();
            crate::wasm_client::notebook::render_combat_tracker();
        }
    });
}

/// Switches the active NPC displayed in `#npc-selected-card-container` when `#npc-roster-select` changes.
#[wasm_bindgen]
pub fn satya_select_npc_from_list() {
    let chosen = select_value("npc-roster-select");
    if !chosen.trim().is_empty() {
        with_state_mut(|st| {
            st.selected_npc_id = Some(chosen);
        });
    }
    render_npcs_tab_dom();
}

fn npc_persist_status_msg(name: &str, is_persistent: bool) -> String {
    let label = if is_persistent {
        "Marked Persistent in Campaign Roster"
    } else {
        "Set to Session-Only"
    };
    format!("Updated NPC '{name}': {label}")
}

fn is_matching_npc_memory(
    m: &satya_schema::GmMemoryNoteRecord,
    cid: &str,
    name_low: Option<&str>,
) -> bool {
    m.memory_id == format!("mem_{cid}")
        || (m.memory_id.starts_with("mem_npc_")
            && name_low.is_some_and(|n| {
                !n.is_empty()
                    && m.title
                        .to_ascii_lowercase()
                        .starts_with(&format!("npc: {n}"))
            }))
}

fn retain_non_duplicate_npcs(
    npcs: &mut Vec<SessionNpcEntry>,
    cid: &str,
    name_low: &str,
    drop_demo: bool,
) {
    npcs.retain(|n| {
        n.character_id != cid
            && (!drop_demo || n.character_id != "npc_000000000000002a")
            && (name_low.is_empty() || n.character_name.trim().to_ascii_lowercase() != name_low)
    });
}

fn notify_npc_vault_synced(msg: &str, append_notice: bool) {
    render_npcs_tab_dom();
    crate::wasm_client::notebook::render_combat_tracker();
    satya_refresh_gm_memories();
    refresh_gm_omnisearch_if_active();
    set_feedback("gm-doc-feedback", msg, false);
    if append_notice {
        append_system_notice(msg);
    }
}

/// Toggles `is_persistent` on a Session NPC (and its companion `mem_<character_id>` GM memory)
/// via `PATCH /api/sessions/:session_id/npcs/:character_id`.
#[wasm_bindgen]
pub fn satya_toggle_npc_persistence(character_id: String, is_persistent: bool) {
    if !viewer_can_see_npc_gm_secrets() {
        return;
    }
    let Some((sid, tok)) = active_session_and_token_or_feedback("", "") else {
        let (updated_name, mems_opt) = with_state_mut(|st| {
            let name = st
                .session_npcs
                .iter_mut()
                .find(|n| n.character_id == character_id)
                .map(|npc| {
                    npc.is_persistent = is_persistent;
                    npc.character_name.clone()
                });
            let mem_id = format!("mem_{character_id}");
            let mut changed = false;
            for mem in st.gm_memories.iter_mut().filter(|m| m.memory_id == mem_id) {
                mem.is_persistent = is_persistent;
                changed = true;
            }
            (name, changed.then(|| st.gm_memories.clone()))
        });
        render_npcs_tab_dom();
        if let Some(mems) = mems_opt {
            sync_local_gm_memories_dom(&mems);
        }
        refresh_gm_omnisearch_if_active();
        if let Some(name) = updated_name {
            let msg = npc_persist_status_msg(&name, is_persistent);
            set_feedback("gm-doc-feedback", &msg, false);
        }
        return;
    };

    spawn_local(async move {
        let req = UpdateSessionNpcRequest {
            is_persistent: Some(is_persistent),
            ..Default::default()
        };
        let url = format!("/api/sessions/{sid}/npcs/{character_id}");
        match fetch_json::<_, SessionNpcEntry>("PATCH", &url, Some(&tok), Some(&req)).await {
            Ok(updated) => {
                let msg = npc_persist_status_msg(&updated.character_name, updated.is_persistent);
                with_state_mut(|st| {
                    if let Some(slot) = st
                        .session_npcs
                        .iter_mut()
                        .find(|n| n.character_id == updated.character_id)
                    {
                        *slot = updated;
                    } else {
                        st.session_npcs.insert(0, updated);
                    }
                });
                notify_npc_vault_synced(&msg, true);
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Deletes a Session NPC (plus companion `mem_<character_id>` GM memory and same-name duplicates)
/// via `DELETE /api/sessions/:session_id/npcs/:character_id`.
#[wasm_bindgen]
pub fn satya_delete_session_npc(character_id: String) {
    if !viewer_can_see_npc_gm_secrets() {
        return;
    }
    let remove_local = |cid: &str| {
        with_state_mut(|st| {
            let name_low = st
                .session_npcs
                .iter()
                .find(|n| n.character_id == cid)
                .map(|n| n.character_name.trim().to_ascii_lowercase());
            retain_non_duplicate_npcs(
                &mut st.session_npcs,
                cid,
                name_low.as_deref().unwrap_or(""),
                false,
            );
            if st.selected_npc_id.as_deref() == Some(cid) {
                st.selected_npc_id = st.session_npcs.first().map(|n| n.character_id.clone());
            }
            st.gm_memories
                .retain(|m| !is_matching_npc_memory(m, cid, name_low.as_deref()));
            st.gm_memories.clone()
        })
    };

    let Some((sid, tok)) = active_session_and_token_or_feedback("", "") else {
        let mems = remove_local(&character_id);
        sync_local_gm_memories_dom(&mems);
        render_npcs_tab_dom();
        refresh_gm_omnisearch_if_active();
        set_feedback("gm-doc-feedback", "Deleted Session NPC.", false);
        return;
    };

    spawn_local(async move {
        let url = format!("/api/sessions/{sid}/npcs/{character_id}");
        match fetch_json::<(), serde_json::Value>("DELETE", &url, Some(&tok), None).await {
            Ok(_) => {
                let _ = remove_local(&character_id);
                notify_npc_vault_synced("Deleted Session NPC.", true);
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Rolls up a rules-legal Satya NPC from `#npc-gen-prompt` and `#npc-gen-template`, adds it to the
/// `NPC List` selector immediately, and persists it to the session/campaign vault when connected.
#[wasm_bindgen]
pub fn satya_generate_npc_from_prompt() {
    if !viewer_can_see_npc_gm_secrets() {
        append_system_notice("Only the Game Master (or Solo Mode host) can roll up new NPCs.");
        return;
    }

    let trim_opt = |id: &str| {
        let raw = input_value(id);
        (!raw.trim().is_empty()).then(|| raw.trim().to_string())
    };
    let prompt = trim_opt("npc-gen-prompt").unwrap_or_else(|| {
        "Terry is a kind general MD, but needs to feel like she belongs".to_string()
    });
    let name_override = trim_opt("npc-gen-name");
    let tmpl_str = select_value("npc-gen-template");
    let faction_archetype = FactionArchetypeId::from_code_str(&tmpl_str);
    let template = if faction_archetype.is_some() {
        NpcTemplateKind::Auto
    } else {
        NpcTemplateKind::from_code_str(&tmpl_str).unwrap_or(NpcTemplateKind::Auto)
    };
    let career_terms = select_value("npc-gen-terms")
        .trim()
        .parse::<usize>()
        .unwrap_or(2)
        .clamp(1, 4);
    let gm_secret_notes = trim_opt("npc-gen-secret");
    let is_persistent = input_checked("npc-gen-persist");

    // 1. Generate immediately in Wasm via satya-engine so offline/solo and online GMs get instant results
    let gen_res = generate_npc_from_prompt(&NpcGenerationParams {
        prompt: prompt.clone(),
        name_override: name_override.clone(),
        template,
        faction_archetype,
        career_terms,
        seed: 0,
        owner_user_id: "gm_npc".to_string(),
    });

    let mut full_dossier = gen_res.gm_secret_dossier.clone();
    if let Some(extra) = &gm_secret_notes {
        full_dossier.push_str("\nGM Secret Notes: ");
        full_dossier.push_str(extra);
    }

    let psyche_summary: Vec<String> = gen_res
        .psyche_traits
        .iter()
        .map(|p| format!("{} {} ({})", p.code, p.score, p.descriptor))
        .collect();
    let top_skills: Vec<String> = gen_res
        .top_skills
        .iter()
        .map(|(_, name, rank)| format!("{name} (N={rank})"))
        .collect();

    let local_entry = SessionNpcEntry {
        character_id: gen_res.character.character_id.clone(),
        character_name: gen_res.character.character_name.clone(),
        callsign_or_role: gen_res.character.callsign_or_role.clone(),
        age_years: gen_res.character.age_years,
        public_summary: gen_res.public_summary,
        apparent_condition: "Healthy & Alert".to_string(),
        is_persistent,
        gm_character: Some(gen_res.character),
        psyche_summary,
        top_skills,
        gm_secret_dossier: Some(full_dossier),
        ..Default::default()
    };

    let local_name_low = local_entry.character_name.trim().to_ascii_lowercase();
    with_state_mut(|st| {
        retain_non_duplicate_npcs(
            &mut st.session_npcs,
            &local_entry.character_id,
            &local_name_low,
            false,
        );
        st.session_npcs.insert(0, local_entry.clone());
        st.selected_npc_id = Some(local_entry.character_id.clone());
    });
    render_npcs_tab_dom();
    crate::wasm_client::notebook::render_combat_tracker();
    refresh_gm_omnisearch_if_active();
    append_system_notice(&format!(
        "Rolled up NPC '{}' ({}) with rules-legal 5-Stage Lifepath, Skills, and 9 Psyche traits.",
        local_entry.character_name, local_entry.callsign_or_role
    ));

    // 2. If connected to a signaling session, persist to SQLite (`session_npcs`, `characters`, `gm_memories`)
    let Some((sid, tok)) = active_session_and_token_or_feedback("", "") else {
        return;
    };

    let req_body = GenerateNpcRequest {
        prompt,
        name_override,
        template: Some(
            faction_archetype
                .map(|a| a.as_str().to_string())
                .unwrap_or_else(|| template.as_str().to_string()),
        ),
        career_terms,
        seed: 0,
        gm_secret_notes,
        is_persistent,
        campaign_tag: Some(sid.clone()),
    };

    spawn_local(async move {
        let url = format!("/api/sessions/{sid}/npcs/generate");
        if let Ok(resp) = fetch_json::<GenerateNpcRequest, GenerateNpcResponse>(
            "POST",
            &url,
            Some(&tok),
            Some(&req_body),
        )
        .await
        {
            let resp_name_low = resp.npc.character_name.trim().to_ascii_lowercase();
            with_state_mut(|st| {
                retain_non_duplicate_npcs(
                    &mut st.session_npcs,
                    &resp.npc.character_id,
                    &resp_name_low,
                    true,
                );
                if let Some(mem) = resp.gm_memory {
                    st.gm_memories.retain(|m| {
                        m.memory_id != mem.memory_id
                            && !is_matching_npc_memory(
                                m,
                                &resp.npc.character_id,
                                Some(&resp_name_low),
                            )
                    });
                    st.gm_memories.insert(0, mem);
                }
                st.selected_npc_id = Some(resp.npc.character_id.clone());
                st.session_npcs.insert(0, resp.npc);
            });
            render_npcs_tab_dom();
            crate::wasm_client::notebook::render_combat_tracker();
            refresh_gm_omnisearch_if_active();
        }
    });
}

/// Loads the currently selected NPC's full [`CharacterSheetRecord`] into the `Character Sheet/Status` tab.
#[wasm_bindgen]
pub fn satya_load_selected_npc_to_sheet() {
    let sheet_opt = with_state(|st| {
        let sel = st.selected_npc_id.as_ref()?;
        st.session_npcs
            .iter()
            .find(|n| &n.character_id == sel)
            .and_then(|n| n.gm_character.clone())
    });
    let Some(sheet) = sheet_opt else {
        append_system_notice(
            "Full NPC character sheet is only accessible to the Game Master (or in Solo Mode).",
        );
        return;
    };
    with_state_mut(|st| {
        st.character_sheet.apply_character_sheet_record(&sheet);
    });
    render_character_sheet();
    satya_select_notebook_tab("character".to_string());
    append_system_notice(&format!(
        "Loaded NPC '{}' into Character Sheet/Status tab.",
        sheet.character_name
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_viewer_see_npc_secrets_solo_gm_and_player_modes() {
        let human_gm = Participant::human("u-gm", "sean", "Sean (GM)", SessionRole::Gm);
        let human_player = Participant::human("u-p1", "sara", "Sara", SessionRole::Player);
        let ai_gm = Participant::ai_gm("sess-1", "gemma4:12b", "harsh-reality-arbiter");
        let gm_roster = [human_gm.clone(), human_player.clone()];
        let ai_roster = [ai_gm, human_player.clone()];
        let hp = Some(&human_player);

        // 1. Offline / Solo mode (no session_id) -> always true
        assert!(can_viewer_see_npc_secrets(None, hp, &gm_roster, false));
        // 2. Active session where viewer is Human GM -> true
        assert!(can_viewer_see_npc_secrets(
            Some("sess-1"),
            Some(&human_gm),
            &gm_roster,
            false
        ));
        // 3. Active session where viewer is Player and a Human GM is present -> false
        assert!(!can_viewer_see_npc_secrets(
            Some("sess-1"),
            hp,
            &gm_roster,
            true
        ));
        // 4. Active session where viewer is Player with ONLY an AI GM -> respects server fallback (redacted when false)
        assert!(!can_viewer_see_npc_secrets(
            Some("sess-1"),
            hp,
            &ai_roster,
            false
        ));
        assert!(can_viewer_see_npc_secrets(
            Some("sess-1"),
            hp,
            &ai_roster,
            true
        ));
        // 4b. Observer NEVER sees GM secrets even if fallback is true
        let observer = Participant::human("u-obs", "obs", "Observer", SessionRole::Observer);
        assert!(!can_viewer_see_npc_secrets(
            Some("sess-1"),
            Some(&observer),
            &ai_roster,
            true
        ));
        // 5. Empty roster before initial ROSTER_SYNC -> defers to server fallback flag
        assert!(!can_viewer_see_npc_secrets(Some("sess-1"), hp, &[], false));
        assert!(can_viewer_see_npc_secrets(Some("sess-1"), hp, &[], true));
    }

    #[test]
    fn test_format_npc_card_html_gm_visibility_vs_player_redaction() {
        let gen_res = generate_npc_from_prompt(&NpcGenerationParams {
            prompt: "Dr. Lin <Trauma> is a methodical field surgeon hiding a syndicate debt"
                .to_string(),
            name_override: Some("Dr. Lin <Trauma>".to_string()),
            template: NpcTemplateKind::Auto,
            faction_archetype: None,
            career_terms: 2,
            seed: 7,
            owner_user_id: "gm_npc".to_string(),
        });
        let entry = SessionNpcEntry {
            character_id: gen_res.character.character_id.clone(),
            character_name: gen_res.character.character_name.clone(),
            callsign_or_role: gen_res.character.callsign_or_role.clone(),
            age_years: gen_res.character.age_years,
            public_summary: "Calm bedside manner under fire.".to_string(),
            apparent_condition: "Fatigued".to_string(),
            is_persistent: true,
            gm_character: Some(gen_res.character),
            psyche_summary: vec!["EMP 17 (Compassionate)".to_string()],
            top_skills: vec!["Trauma Surgery (N=4)".to_string()],
            gm_secret_dossier: Some("Owes 40,000 cr to Black Sun brokers.".to_string()),
            ..Default::default()
        };

        let gm_html = format_npc_card_html(&entry, true);
        let toggle_js = format!(
            "window.satyaToggleNpcPersistence('{}', false)",
            entry.character_id
        );
        let del_js = format!("window.satyaDeleteSessionNpc('{}')", entry.character_id);
        assert!(
            gm_html.contains("Dr. Lin &lt;Trauma&gt;")
                && gm_html.contains("EMP 17 (Compassionate)")
        );
        assert!(
            gm_html.contains("Calm bedside manner under fire.")
                && gm_html.contains("Trauma Surgery (N=4)")
        );
        assert!(
            gm_html.contains("Owes 40,000 cr to Black Sun brokers.")
                && gm_html.contains("satyaLoadSelectedNpcToSheet()")
        );
        assert!(gm_html.contains(&toggle_js) && gm_html.contains(&del_js));

        let player_html = format_npc_card_html(&entry, false);
        assert!(
            player_html.contains("Dr. Lin &lt;Trauma&gt;")
                && player_html.contains("Calm bedside manner under fire.")
        );
        assert!(player_html.contains("Player View: GM-only Characteristics"));
        assert!(
            !player_html.contains("EMP 17 (Compassionate)")
                && !player_html.contains("Trauma Surgery (N=4)")
        );
        assert!(
            !player_html.contains("Owes 40,000 cr to Black Sun brokers.")
                && !player_html.contains("satyaLoadSelectedNpcToSheet()")
        );
        assert!(
            !player_html.contains("satyaToggleNpcPersistence")
                && !player_html.contains("satyaDeleteSessionNpc")
        );
    }
}
