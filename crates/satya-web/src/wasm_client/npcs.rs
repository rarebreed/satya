//! `NPC List` Tab (`#notebook-pane-npcs`) & Prompt-/Template-Driven Quick NPC Generator
//! for Human and AI Game Masters, with Role-Aware Player vs. GM/Solo Visibility.

use crate::wasm_client::character_sheet::render_character_sheet;
use crate::wasm_client::chat_and_ttrpg::append_system_notice;
use crate::wasm_client::dom::{
    escape_html, fetch_json, get_element, input_checked, input_value, select_value, set_display,
    set_text,
};
use crate::wasm_client::notebook::satya_select_notebook_tab;
use crate::wasm_client::state::{with_state, with_state_mut};
use satya_engine::{NpcGenerationParams, NpcTemplateKind, generate_npc_from_prompt};
use satya_schema::{
    GenerateNpcRequest, GenerateNpcResponse, ParticipantKind, SessionNpcEntry, SessionNpcsResponse,
    SessionRole,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// Determines whether the local viewer is allowed to see GM-only NPC secrets, Psyche traits, and full skill ranks.
///
/// Returns `true` when:
/// - The user is in local/solo mode (not connected to a multi-player session), OR
/// - The user's session role is `SessionRole::Gm`, OR
/// - The session has no human `SessionRole::Gm` (e.g. Solo / Autonomous AI-GM mode).
#[must_use]
pub fn viewer_can_see_npc_gm_secrets() -> bool {
    with_state(|st| {
        if st.session_id.is_none() {
            return true;
        }
        if let Some(me) = &st.self_participant
            && me.role == SessionRole::Gm
        {
            return true;
        }
        if !st.roster.is_empty() {
            let has_human_gm = st
                .roster
                .iter()
                .any(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm);
            return !has_human_gm;
        }
        st.viewer_can_see_gm_secrets
    })
}

fn ensure_default_demo_npc_if_empty() {
    let is_empty = with_state(|st| st.session_npcs.is_empty());
    if !is_empty {
        return;
    }
    let gen_res = generate_npc_from_prompt(&NpcGenerationParams {
        prompt: "Terry is a kind general MD, but needs to feel like she belongs".to_string(),
        name_override: None,
        template: NpcTemplateKind::Auto,
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
/// When `can_see_gm_secrets == true` (GM or Solo mode), the 7 bipolar Psyche traits, key characteristics,
/// top trained skills, and GM-only secret dossier are also displayed.
#[must_use]
pub fn format_npc_card_html(npc: &SessionNpcEntry, can_see_gm_secrets: bool) -> String {
    let persist_badge = if npc.is_persistent {
        r#"<span class="pill pill-accent">Persistent Campaign NPC</span>"#
    } else {
        r#"<span class="pill pill-muted">Session NPC</span>"#
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
        let psyche_chips = if npc.psyche_summary.is_empty() {
            r#"<span class="pill pill-muted">Balanced Psyche</span>"#.to_string()
        } else {
            npc.psyche_summary
                .iter()
                .map(|p| {
                    let is_polar = !p.contains("Grounded")
                        && !p.contains("Pragmatic")
                        && !p.contains("Cooperative")
                        && !p.contains("Ambiverted")
                        && !p.contains("Adaptive")
                        && !p.contains("Measured");
                    let cls = if is_polar {
                        "pill pill-accent"
                    } else {
                        "pill pill-muted"
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

        let stats_line = if let Some(ch) = &npc.gm_character {
            let c = &ch.characteristics;
            format!(
                "SPD {} · FOR {} · DEX {} · HLT {} · FIT {} | MEM {} · ANA {} · INS {} · DIS {} · FOC {} | COM {} · PRE {} · ELO {}",
                c.spd,
                c.for_stat,
                c.dex,
                c.hlt,
                c.fit,
                c.mem,
                c.ana,
                c.ins,
                c.dis,
                c.foc,
                c.com,
                c.pre,
                c.elo
            )
        } else {
            "Stats loaded in GM Vault".to_string()
        };

        let dossier_text = npc
            .gm_secret_dossier
            .as_deref()
            .unwrap_or("No additional GM secret notes.");

        html.push_str(&format!(
            r#"<div class="gm-doc-meta" style="margin-top:8px; border-top:1px solid rgba(148,163,184,0.2); padding-top:8px;">
                <div><strong>GM / Solo Only — 7 Bipolar Psyche Traits (1–20):</strong></div>
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

    if let Some(sel_el) = get_element("npc-roster-select") {
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
        sel_el.set_inner_html(&opts_html);
    }

    if let Some(card_el) = get_element("npc-selected-card-container") {
        if let Some(active_npc) = npcs
            .iter()
            .find(|n| Some(&n.character_id) == selected_id.as_ref())
            .or_else(|| npcs.first())
        {
            card_el.set_inner_html(&format_npc_card_html(active_npc, can_see_gm));
        } else {
            card_el.set_inner_html(
                r#"<span class="pill pill-muted">No NPCs in this session yet.</span>"#,
            );
        }
    }
}

/// Refreshes the `NPC List` tab from the server (`GET /api/sessions/:session_id/npcs`) if connected,
/// and updates the DOM.
#[wasm_bindgen]
pub fn satya_refresh_npcs_tab() {
    render_npcs_tab_dom();

    let (sid_opt, tok_opt) = with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
    let (Some(sid), Some(tok)) = (sid_opt, tok_opt) else {
        return;
    };

    spawn_local(async move {
        let url = format!("/api/sessions/{sid}/npcs");
        if let Ok(resp) = fetch_json::<(), SessionNpcsResponse>("GET", &url, Some(&tok), None).await
        {
            with_state_mut(|st| {
                st.viewer_can_see_gm_secrets = resp.viewer_can_see_gm_secrets;
                if !resp.npcs.is_empty() {
                    st.session_npcs = resp.npcs;
                }
            });
            render_npcs_tab_dom();
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

/// Rolls up a rules-legal Satya NPC from `#npc-gen-prompt` and `#npc-gen-template`, adds it to the
/// `NPC List` selector immediately, and persists it to the session/campaign vault when connected.
#[wasm_bindgen]
pub fn satya_generate_npc_from_prompt() {
    if !viewer_can_see_npc_gm_secrets() {
        append_system_notice("Only the Game Master (or Solo Mode host) can roll up new NPCs.");
        return;
    }

    let raw_prompt = input_value("npc-gen-prompt");
    let prompt = if raw_prompt.trim().is_empty() {
        "Terry is a kind general MD, but needs to feel like she belongs".to_string()
    } else {
        raw_prompt.trim().to_string()
    };
    let name_ovr_raw = input_value("npc-gen-name");
    let name_override = if name_ovr_raw.trim().is_empty() {
        None
    } else {
        Some(name_ovr_raw.trim().to_string())
    };
    let tmpl_str = select_value("npc-gen-template");
    let template = NpcTemplateKind::from_code_str(&tmpl_str).unwrap_or(NpcTemplateKind::Auto);
    let career_terms = select_value("npc-gen-terms")
        .trim()
        .parse::<usize>()
        .unwrap_or(2)
        .clamp(1, 4);
    let secret_raw = input_value("npc-gen-secret");
    let gm_secret_notes = if secret_raw.trim().is_empty() {
        None
    } else {
        Some(secret_raw.trim().to_string())
    };
    let is_persistent = input_checked("npc-gen-persist");

    // 1. Generate immediately in Wasm via satya-engine so offline/solo and online GMs get instant results
    let gen_res = generate_npc_from_prompt(&NpcGenerationParams {
        prompt: prompt.clone(),
        name_override: name_override.clone(),
        template,
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
    };

    with_state_mut(|st| {
        st.session_npcs
            .retain(|n| n.character_id != local_entry.character_id);
        st.session_npcs.insert(0, local_entry.clone());
        st.selected_npc_id = Some(local_entry.character_id.clone());
    });
    render_npcs_tab_dom();
    append_system_notice(&format!(
        "Rolled up NPC '{}' ({}) with rules-legal 5-Stage Lifepath, Skills, and 7 Psyche traits.",
        local_entry.character_name, local_entry.callsign_or_role
    ));

    // 2. If connected to a signaling session, persist to SQLite (`session_npcs`, `characters`, `gm_memories`)
    let (sid_opt, tok_opt) = with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
    let (Some(sid), Some(tok)) = (sid_opt, tok_opt) else {
        return;
    };

    let req_body = GenerateNpcRequest {
        prompt,
        name_override,
        template: Some(template.as_str().to_string()),
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
            with_state_mut(|st| {
                st.session_npcs
                    .retain(|n| n.character_id != resp.npc.character_id);
                st.session_npcs.insert(0, resp.npc.clone());
                st.selected_npc_id = Some(resp.npc.character_id);
                if let Some(mem) = resp.gm_memory {
                    st.gm_memories.retain(|m| m.memory_id != mem.memory_id);
                    st.gm_memories.insert(0, mem);
                }
            });
            render_npcs_tab_dom();
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
