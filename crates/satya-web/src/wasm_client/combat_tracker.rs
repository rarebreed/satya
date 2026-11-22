//! Combat Tracker (`#notebook-pane-combat`) Count-Up Tick timeline, role-aware
//! combatant Health & Fatigue projection, planned `T/C/F` reflection, and
//! Character Sheet <-> Task Resolution condition auto-synchronization.

use crate::wasm_client::character_sheet::{BoneFractureStatus, CharacterSheetState};
use crate::wasm_client::dom::{
    escape_html, escape_js_single_quoted_attr, fetch_json, get_element, select_value,
    set_input_value, set_text,
};
use crate::wasm_client::notebook::satya_select_notebook_tab;
use crate::wasm_client::npcs::can_viewer_see_npc_secrets;
use crate::wasm_client::state::{
    PlannedCombatActionState, SECONDS_PER_TICK, with_state, with_state_mut,
};
use satya_engine::HemorrhageGrade;
use satya_schema::{
    CharacterSheetRecord, CombatantTickEntry, Participant, ParticipantKind, SessionNpcEntry,
    SessionRole,
};
use wasm_bindgen::prelude::*;

/// Inputs needed to render the Combat Tracker combatant list (`#combat-tracker-list`).
pub struct CombatTrackerRenderContext<'a> {
    pub in_session: bool,
    pub round: u32,
    pub tick: u32,
    pub combatants: &'a [CombatantTickEntry],
    pub roster: &'a [Participant],
    pub self_participant: Option<&'a Participant>,
    pub character_sheet: &'a CharacterSheetState,
    pub session_npcs: &'a [SessionNpcEntry],
    pub can_see_gm_secrets: bool,
    pub planned_action: &'a PlannedCombatActionState,
}

fn condition_severity_pill_class(level: i32) -> &'static str {
    if level <= 0 {
        "pill-muted"
    } else if level <= 2 {
        "pill-warn"
    } else {
        "pill-danger"
    }
}

fn summarize_trauma_counts(
    injured_zones: usize,
    max_wound_tier: u8,
    max_bleed_grade: i32,
    fractures: usize,
    blood_units_lost: f64,
) -> Vec<(&'static str, String)> {
    if injured_zones == 0 && blood_units_lost < 1.0 && fractures == 0 && max_bleed_grade <= 0 {
        return vec![("pill-muted", "Intact".to_string())];
    }

    let mut pills = Vec::new();
    if injured_zones > 0 {
        let cls = if max_wound_tier >= 3 || fractures > 0 {
            "pill-danger"
        } else {
            "pill-warn"
        };
        let suffix = if injured_zones == 1 { "" } else { "s" };
        let mut lbl = format!("{injured_zones} Wound{suffix} (T{max_wound_tier}");
        if fractures > 0 {
            lbl.push_str(&format!(", {fractures} Fx"));
        }
        lbl.push(')');
        pills.push((cls, lbl));
    } else if fractures > 0 {
        pills.push(("pill-danger", format!("{fractures} Fx")));
    }

    if max_bleed_grade > 0 {
        pills.push(("pill-danger", format!("Bleed G{max_bleed_grade}")));
    }
    if blood_units_lost >= 1.0 {
        pills.push(("pill-danger", format!("Blood -{blood_units_lost:.1}U")));
    }
    pills
}

fn summarize_sheet_trauma_pills(sheet: &CharacterSheetState) -> Vec<(&'static str, String)> {
    let mut injured_zones = 0usize;
    let mut max_tier = 0u8;
    let mut max_bleed = 0i32;
    let mut fractures = 0usize;
    for z in &sheet.hit_zones {
        if z.is_injured() {
            injured_zones += 1;
        }
        max_tier = max_tier.max(z.wound_tier);
        max_bleed = max_bleed.max(z.bleed_grade.grade_number());
        if !matches!(z.bone_status, BoneFractureStatus::Intact) {
            fractures += 1;
        }
    }
    summarize_trauma_counts(
        injured_zones,
        max_tier,
        max_bleed,
        fractures,
        sheet.blood_units_lost,
    )
}

fn summarize_record_trauma_pills(rec: &CharacterSheetRecord) -> Vec<(&'static str, String)> {
    let mut injured_zones = 0usize;
    let mut max_tier = 0u8;
    let mut max_bleed = 0i32;
    let mut fractures = 0usize;
    for z in &rec.hit_zones {
        let bone = BoneFractureStatus::from_code(&z.bone_status);
        let bleed = HemorrhageGrade::from_grade_number(z.bleed_grade);
        let has_fx = !matches!(bone, BoneFractureStatus::Intact);
        if z.wound_tier > 0 || has_fx || bleed != HemorrhageGrade::Grade0None {
            injured_zones += 1;
        }
        max_tier = max_tier.max(z.wound_tier.clamp(0, 5) as u8);
        max_bleed = max_bleed.max(bleed.grade_number());
        if has_fx {
            fractures += 1;
        }
    }
    summarize_trauma_counts(
        injured_zones,
        max_tier,
        max_bleed,
        fractures,
        rec.blood_units_lost,
    )
}

fn apparent_condition_pill(
    apparent: Option<&str>,
    fatigue: i32,
    shock: i32,
) -> (&'static str, String) {
    let label = apparent
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if shock >= 3 || fatigue >= 4 {
                "Severely Injured / Critical".to_string()
            } else if shock >= 1 || fatigue >= 2 {
                "Winded / Wounded".to_string()
            } else {
                "Healthy & Alert".to_string()
            }
        });
    let low = label.to_ascii_lowercase();
    let cls = if shock >= 3
        || fatigue >= 4
        || low.contains("severe")
        || low.contains("critical")
        || low.contains("bleed")
    {
        "pill-danger"
    } else if shock >= 1
        || fatigue >= 2
        || low.contains("winded")
        || low.contains("wounded")
        || low.contains("injured")
    {
        "pill-warn"
    } else {
        "pill-muted"
    };
    (cls, label)
}

fn format_planned_action_pill(planned: &PlannedCombatActionState) -> String {
    if let Some(ref tech_name) = planned.technique_name {
        let tc_part = planned
            .effective_tc
            .map(|tc| format!(" · {tc}t"))
            .unwrap_or_default();
        let (cls, violation_tag) = if planned.valid {
            ("pill-active", "")
        } else {
            ("pill-danger", " · Pool Violation")
        };
        format!(
            r#"<span class="pill {cls}">Planned: {} (T{}/C{}/F{}{tc_part}{violation_tag})</span>"#,
            escape_html(tech_name),
            planned.tempo,
            planned.control,
            planned.force,
        )
    } else if planned.p_eff > 0 {
        format!(
            r#"<span class="pill pill-muted">T{}/C{}/F{}</span>"#,
            planned.tempo, planned.control, planned.force,
        )
    } else {
        String::new()
    }
}

fn format_pill(cls: &str, text: &str) -> String {
    format!(r#"<span class="pill {cls}">{}</span>"#, escape_html(text))
}

fn render_combatant_row_html(row_cls: &str, main_html: &str, pills_html: &[String]) -> String {
    let joined_pills = pills_html.join("");
    format!(
        r#"<div class="{row_cls}"><div class="combatant-main">{main_html}</div><div class="combatant-status-pills">{joined_pills}</div></div>"#
    )
}

fn append_self_sheet_pills(
    pills: &mut Vec<String>,
    sheet: &CharacterSheetState,
    planned: &PlannedCombatActionState,
) {
    let fat = sheet.effective_fatigue_level();
    let shock = sheet.effective_wound_shock_index();
    pills.push(format_pill(
        condition_severity_pill_class(fat),
        &format!("Fatigue L{fat}/5"),
    ));
    pills.push(format_pill(
        condition_severity_pill_class(shock),
        &format!("Shock {shock}/5"),
    ));
    for (cls, lbl) in summarize_sheet_trauma_pills(sheet) {
        pills.push(format_pill(cls, &lbl));
    }
    let planned_pill = format_planned_action_pill(planned);
    if !planned_pill.is_empty() {
        pills.push(planned_pill);
    }
    pills.push(
        r#"<button type="button" class="btn-chip combatant-inspect-btn" onclick="window.satyaOpenCombatantDetails('self', '')">Sheet ↗</button>"#
            .to_string(),
    );
}

/// Formats the rich, role-aware Combat Tracker combatant list (`#combat-tracker-list`),
/// combining Count-Up Tick readiness, live Health & Fatigue summary pills, planned `T/C/F`,
/// and one-click Character Sheet / NPC List navigation.
#[must_use]
pub fn format_combat_tracker_rich_html(ctx: &CombatTrackerRenderContext<'_>) -> String {
    if ctx.in_session || !ctx.combatants.is_empty() {
        if ctx.combatants.is_empty() {
            return r#"<span class="pill pill-muted">No active combatants recorded in session tracker.</span>"#
                .to_string();
        }
        let mut ordered: Vec<&CombatantTickEntry> = ctx.combatants.iter().collect();
        ordered.sort_by_key(|c| (c.next_action_tick, c.display_name.as_str()));
        return ordered
            .into_iter()
            .map(|c| {
                let actor_sec = f64::from(c.next_action_tick) * SECONDS_PER_TICK;
                let role_low = c.role_or_faction.to_ascii_lowercase();
                let is_self = ctx.self_participant.is_some_and(|me| {
                    me.role != SessionRole::Observer
                        && (c.entity_id == me.user_id
                            || (me.role == SessionRole::Player
                                && c.display_name.eq_ignore_ascii_case(&me.display_name)))
                });
                let matched_npc = if is_self {
                    None
                } else {
                    ctx.session_npcs.iter().find(|n| {
                        n.character_id == c.entity_id
                            || n.character_name.eq_ignore_ascii_case(&c.display_name)
                            || n.character_name.eq_ignore_ascii_case(&c.entity_id)
                    })
                };
                let is_npc_or_ai = matched_npc.is_some()
                    || c.entity_id.starts_with("ai-")
                    || c.entity_id.starts_with("npc")
                    || role_low.contains("ai")
                    || role_low.contains("npc")
                    || role_low.contains("enemy")
                    || role_low.contains("hostile");

                let mut row_cls = String::from("combatant-row");
                if is_npc_or_ai {
                    row_cls.push_str(" ai");
                }
                if is_self {
                    row_cls.push_str(" is-self");
                }

                let stance = c
                    .active_stance
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| format!(" · {}", escape_html(s)))
                    .unwrap_or_default();
                let zone = c
                    .zone_label
                    .as_deref()
                    .map(str::trim)
                    .filter(|z| !z.is_empty())
                    .map(|z| format!(" · {}", escape_html(z)))
                    .unwrap_or_default();

                let main_html = format!(
                    r#"<strong>{}</strong> [{}] · OODA {} · Next Action @ Tick {} ({actor_sec:.2}s · W{}){stance}{zone}"#,
                    escape_html(&c.display_name),
                    escape_html(&c.role_or_faction),
                    c.ooda_score,
                    c.next_action_tick,
                    ctx.round,
                );

                let mut pills = Vec::new();
                if is_self {
                    append_self_sheet_pills(&mut pills, ctx.character_sheet, ctx.planned_action);
                } else if is_npc_or_ai {
                    if ctx.can_see_gm_secrets {
                        let (fat, shock, gm_char_opt) = match matched_npc
                            .and_then(|n| n.gm_character.as_ref())
                        {
                            Some(rec) => (
                                rec.fatigue_level.max(c.fatigue_level),
                                rec.wound_shock_index.max(c.wound_shock),
                                Some(rec),
                            ),
                            None => (c.fatigue_level, c.wound_shock, None),
                        };
                        pills.push(format_pill(
                            condition_severity_pill_class(fat),
                            &format!("Fatigue L{fat}/5"),
                        ));
                        pills.push(format_pill(
                            condition_severity_pill_class(shock),
                            &format!("Shock {shock}/5"),
                        ));
                        if let Some(rec) = gm_char_opt {
                            for (cls, lbl) in summarize_record_trauma_pills(rec) {
                                pills.push(format_pill(cls, &lbl));
                            }
                        }
                    } else {
                        let (cls, lbl) = apparent_condition_pill(
                            matched_npc.map(|n| n.apparent_condition.as_str()),
                            c.fatigue_level,
                            c.wound_shock,
                        );
                        pills.push(format_pill(cls, &lbl));
                    }
                    if let Some(npc) = matched_npc {
                        let js_id = escape_js_single_quoted_attr(&npc.character_id);
                        pills.push(format!(
                            r#"<button type="button" class="btn-chip combatant-inspect-btn" onclick="window.satyaOpenCombatantDetails('npc', '{js_id}')">NPC ↗</button>"#
                        ));
                    }
                } else {
                    pills.push(format_pill(
                        condition_severity_pill_class(c.fatigue_level),
                        &format!("Fatigue L{}/5", c.fatigue_level),
                    ));
                    pills.push(format_pill(
                        condition_severity_pill_class(c.wound_shock),
                        &format!("Shock {}/5", c.wound_shock),
                    ));
                }

                render_combatant_row_html(&row_cls, &main_html, &pills)
            })
            .collect::<Vec<_>>()
            .join("");
    }

    let combatants: Vec<&Participant> = ctx
        .roster
        .iter()
        .filter(|p| p.role == SessionRole::Player)
        .collect();
    if !combatants.is_empty() {
        return combatants
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                let ooda = (13 - (idx as i32)).clamp(8, 16);
                let start_tick = satya_engine::compute_starting_tick(ooda, 0, 0, ooda + 1, 12) as u32;
                let actor_tick = ctx.tick.max(start_tick) + (idx as u32) * 2;
                let actor_sec = f64::from(actor_tick) * SECONDS_PER_TICK;
                let is_self = ctx
                    .self_participant
                    .map(|me| p.user_id == me.user_id)
                    .unwrap_or(idx == 0);
                let mut row_cls = String::from("combatant-row");
                if p.participant_kind == ParticipantKind::AiAgent {
                    row_cls.push_str(" ai");
                }
                if is_self {
                    row_cls.push_str(" is-self");
                }
                let main_html = format!(
                    r#"<strong>{}</strong> [{:?}] · OODA {ooda} · Next Action @ Tick {actor_tick} ({actor_sec:.2}s · W{})"#,
                    escape_html(&p.display_name),
                    p.role,
                    ctx.round,
                );
                let mut pills = Vec::new();
                if is_self {
                    append_self_sheet_pills(&mut pills, ctx.character_sheet, ctx.planned_action);
                } else {
                    pills.push(format_pill("pill-muted", "Fatigue L0/5"));
                    pills.push(format_pill("pill-muted", "Shock 0/5"));
                }
                render_combatant_row_html(&row_cls, &main_html, &pills)
            })
            .collect::<Vec<_>>()
            .join("");
    }

    let derived = ctx.character_sheet.characteristics.compute_derived(0);
    let ooda = derived.ooda_initiative;
    let d_cog = derived.d_cog_quick_thinking;
    let start_tick = satya_engine::compute_starting_tick(
        ooda,
        ctx.character_sheet.effective_fatigue_level(),
        ctx.character_sheet.effective_wound_shock_index(),
        d_cog,
        12,
    ) as u32;
    let actor_tick = ctx.tick.max(start_tick);
    let actor_sec = f64::from(actor_tick) * SECONDS_PER_TICK;
    let main_html = format!(
        r#"<strong>{}</strong> [Local PC] · OODA {ooda} · Next Action @ Tick {actor_tick} ({actor_sec:.2}s · W{})"#,
        escape_html(&ctx.character_sheet.character_name),
        ctx.round,
    );
    let mut pills = Vec::new();
    append_self_sheet_pills(&mut pills, ctx.character_sheet, ctx.planned_action);
    render_combatant_row_html("combatant-row is-self", &main_html, &pills)
}

/// Formats authoritative server [`CombatantTickEntry`] rows (including NPCs, stance, fatigue, and wound shock) for `#combat-tracker-list`.
pub fn format_combat_tracker_entries_html(
    combatants: &[CombatantTickEntry],
    round: u32,
) -> String {
    if combatants.is_empty() {
        return r#"<span class="pill pill-muted">No active combatants recorded in session tracker.</span>"#
            .to_string();
    }
    let mut ordered: Vec<&CombatantTickEntry> = combatants.iter().collect();
    ordered.sort_by_key(|c| (c.next_action_tick, c.display_name.as_str()));
    ordered
        .into_iter()
        .map(|c| {
            let actor_sec = f64::from(c.next_action_tick) * SECONDS_PER_TICK;
            let role_low = c.role_or_faction.to_ascii_lowercase();
            let badge_cls = if c.entity_id.starts_with("ai-")
                || role_low.contains("ai")
                || role_low.contains("npc")
                || role_low.contains("enemy")
            {
                "participant-chip ai"
            } else {
                "participant-chip"
            };
            let stance = c
                .active_stance
                .as_ref()
                .map(|s| format!(" · {s:?}"))
                .unwrap_or_default();
            let fatigue = if c.fatigue_level != 0 {
                format!(" · Fatigue {}", c.fatigue_level)
            } else {
                String::new()
            };
            let shock = if c.wound_shock != 0 {
                format!(" · Shock {}", c.wound_shock)
            } else {
                String::new()
            };
            let zone = c
                .zone_label
                .as_deref()
                .map(str::trim)
                .filter(|z| !z.is_empty())
                .map(|z| format!(" · {}", escape_html(z)))
                .unwrap_or_default();
            format!(
                r#"<div class="{badge_cls}"><strong>{}</strong> [{}] · OODA {} · Next Action @ Tick {} ({actor_sec:.2}s · W{round}){stance}{fatigue}{shock}{zone}</div>"#,
                escape_html(&c.display_name),
                escape_html(&c.role_or_faction),
                c.ooda_score,
                c.next_action_tick,
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Formats the HTML chip list of active session combatants and their continuous Count-Up Tick readiness (`1 Tick = 0.25s`, `Tick_start = max(1, 15 - OODA - Awareness)`) for `#combat-tracker-list`.
pub fn format_combat_tracker_html(roster: &[Participant], round: u32, tick: u32) -> String {
    let combatants: Vec<&Participant> = roster
        .iter()
        .filter(|p| p.role == SessionRole::Player)
        .collect();
    if combatants.is_empty() {
        return r#"<span class="pill pill-muted">No active player combatants in roster.</span>"#
            .to_string();
    }
    combatants
        .into_iter()
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

/// Re-renders `#combat-tracker-list` from current [`SatyaWasmState`].
pub(crate) fn render_combat_tracker() {
    let html = with_state(|st| {
        let can_see_gm = can_viewer_see_npc_secrets(
            st.session_id.as_deref(),
            st.self_participant.as_ref(),
            &st.roster,
            st.viewer_can_see_gm_secrets,
        );
        let ctx = CombatTrackerRenderContext {
            in_session: st.session_id.is_some(),
            round: st.combat_round,
            tick: st.combat_tick,
            combatants: &st.combat_combatants,
            roster: &st.roster,
            self_participant: st.self_participant.as_ref(),
            character_sheet: &st.character_sheet,
            session_npcs: &st.session_npcs,
            can_see_gm_secrets: can_see_gm,
            planned_action: &st.planned_combat_action,
        };
        format_combat_tracker_rich_html(&ctx)
    });
    if let Some(el) = get_element("combat-tracker-list") {
        el.set_inner_html(&html);
    }
}

/// Fetches the latest authoritative [`satya_schema::CombatTrackerState`] when in a live session and re-renders `#combat-tracker-list`.
pub(crate) fn refresh_combat_tracker() {
    render_combat_tracker();
    let (session_opt, token_opt) = with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
    if let (Some(sid), Some(tok)) = (session_opt, token_opt) {
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/sessions/{sid}/combat");
            if let Ok(ct) =
                fetch_json::<(), satya_schema::CombatTrackerState>("GET", &url, Some(&tok), None)
                    .await
            {
                let tick = ct.tick;
                let round = ct.round.max(1);
                with_state_mut(|st| {
                    st.combat_tick = tick;
                    st.combat_round = round;
                    st.combat_combatants = ct.combatants;
                });
                let elapsed_sec = f64::from(tick) * SECONDS_PER_TICK;
                set_text(
                    "combat-round-pill",
                    &format!("Window {round} · Tick {tick} ({elapsed_sec:.2}s)"),
                );
                render_combat_tracker();
            }
        });
    }
}

/// Advances (or resets when `delta == 0`) the Satya Count-Up Tick Timeline (`1 Tick = 0.25s`, `20 Ticks = 5.0s`) in the `Combat Tracker` notebook tab and persists the updated state to SQLite when in a live session.
#[wasm_bindgen]
pub fn satya_advance_combat_tick(delta: i32) {
    if with_state(|st| {
        st.session_id.is_some()
            && st
                .self_participant
                .as_ref()
                .is_some_and(|p| p.role == SessionRole::Observer)
    }) {
        return;
    }
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
            ..Default::default()
        };
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("/api/sessions/{sid}/combat");
            if let Ok(ct) = fetch_json::<_, satya_schema::CombatTrackerState>(
                "POST",
                &url,
                Some(&token),
                Some(&req),
            )
            .await
            {
                let tick = ct.tick;
                let round = ct.round.max(1);
                with_state_mut(|st| {
                    st.combat_tick = tick;
                    st.combat_round = round;
                    st.combat_combatants = ct.combatants;
                });
                let elapsed_sec = f64::from(tick) * SECONDS_PER_TICK;
                set_text(
                    "combat-round-pill",
                    &format!("Window {round} · Tick {tick} ({elapsed_sec:.2}s)"),
                );
                render_combat_tracker();
            }
        });
    }
}

/// Opens the inspected combatant's canonical sheet (`"self"` -> `Character Sheet/Status`)
/// or NPC dossier (`"npc"` -> `NPC List` with `selected_npc_id` preselected).
#[wasm_bindgen]
pub fn satya_open_combatant_details(kind: String, target_id: String) {
    if kind.eq_ignore_ascii_case("npc") && !target_id.trim().is_empty() {
        let tid = target_id.trim().to_string();
        with_state_mut(|st| {
            st.selected_npc_id = Some(tid);
        });
        satya_select_notebook_tab("npcs".to_string());
        crate::wasm_client::npcs::render_npcs_tab_dom();
        return;
    }
    with_state_mut(|st| {
        if !st.character_sheet.section_filter.eq_ignore_ascii_case("ALL") {
            st.character_sheet.section_filter = "STATUS".to_string();
        }
    });
    satya_select_notebook_tab("character".to_string());
}

fn sync_conditions_inner(push_to_server: bool) {
    let (delta_int, fatigue, shock, sync_target) = with_state_mut(|st| {
        let delta_int = st.character_sheet.total_task_delta_int();
        let fatigue = st.character_sheet.effective_fatigue_level();
        let shock = st.character_sheet.effective_wound_shock_index();
        let target = if let (Some(sid), Some(tok), Some(me)) = (
            st.session_id.as_ref(),
            st.auth_token.as_ref(),
            st.self_participant.as_ref(),
        ) {
            if let Some(entry) = st
                .combat_combatants
                .iter_mut()
                .find(|c| c.entity_id == me.user_id)
            {
                entry.fatigue_level = fatigue;
                entry.wound_shock = shock;
            }
            let can_push = push_to_server
                && me.role != SessionRole::Observer
                && (me.role == SessionRole::Player
                    || st
                        .combat_combatants
                        .iter()
                        .any(|c| c.entity_id == me.user_id));
            can_push.then(|| (sid.clone(), tok.clone(), me.user_id.clone()))
        } else {
            None
        };
        (delta_int, fatigue, shock, target)
    });

    set_input_value("ttrpg-internal-mod", &delta_int.to_string());
    set_input_value("ttrpg-fatigue", &fatigue.to_string());
    set_input_value("ttrpg-wound-shock", &shock.to_string());
    if select_value("ttrpg-triad-preset") != "custom" {
        crate::wasm_client::ttrpg_drawer_triad::satya_apply_triad_preset();
    }
    crate::wasm_client::ttrpg_drawer_cascade::satya_update_cascade_preview();

    if let Some((sid, tok, uid)) = sync_target {
        wasm_bindgen_futures::spawn_local(async move {
            let req = satya_schema::UpdateCombatTrackerRequest {
                entity_id: Some(uid),
                fatigue_level: Some(fatigue),
                wound_shock: Some(shock),
                ..Default::default()
            };
            let url = format!("/api/sessions/{sid}/combat");
            if let Ok(ct) = fetch_json::<_, satya_schema::CombatTrackerState>(
                "POST",
                &url,
                Some(&tok),
                Some(&req),
            )
            .await
            {
                let tick = ct.tick;
                let round = ct.round.max(1);
                with_state_mut(|st| {
                    st.combat_tick = tick;
                    st.combat_round = round;
                    st.combat_combatants = ct.combatants;
                });
                render_combat_tracker();
            }
        });
    }
}

/// Auto-syncs Character Sheet Fatigue, Wound Shock, and `Δ_int` into `#ttrpg-drawer`,
/// updates local Combat Tracker state, and persists self condition updates to the server in live sessions.
pub(crate) fn sync_character_sheet_conditions_to_roller_and_tracker() {
    sync_conditions_inner(true);
}

/// Syncs Character Sheet Fatigue, Wound Shock, and `Δ_int` into `#ttrpg-drawer` and local
/// Combat Tracker projections without issuing an extra server POST (used after history replay).
pub(crate) fn sync_character_sheet_conditions_locally() {
    sync_conditions_inner(false);
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::{AnatomicalZoneRecord, MessageVisibility};

    #[test]
    fn test_format_combat_tracker_rich_html_pc_npc_redaction_and_offline() {
        let mut sheet = CharacterSheetState {
            character_name: "Sara (Valkyrie)".to_string(),
            fatigue_level: 2,
            ..Default::default()
        };
        sheet.hit_zones[6].wound_tier = 2;
        sheet.hit_zones[6].bleed_grade = HemorrhageGrade::Grade2Venous;

        let self_part = Participant::human("u-sara", "sara", "Sara (Valkyrie)", SessionRole::Player);
        let mut npc_sheet = sheet.to_character_sheet_record("gm");
        npc_sheet.character_id = "npc-guard".to_string();
        npc_sheet.character_name = "Gate Sentry".to_string();
        npc_sheet.fatigue_level = 3;
        npc_sheet.wound_shock_index = 2;
        npc_sheet.hit_zones = vec![AnatomicalZoneRecord {
            location_code: "ZONE_6".to_string(),
            wound_tier: 3,
            bone_status: "FRACTURED".to_string(),
            bleed_grade: 1,
        }];

        let npc_entry = SessionNpcEntry {
            character_id: "npc-guard".to_string(),
            character_name: "Gate Sentry".to_string(),
            callsign_or_role: "SecOps".to_string(),
            age_years: 30,
            public_summary: "Armored sentry".to_string(),
            apparent_condition: "Winded / Wounded".to_string(),
            is_persistent: false,
            gm_character: Some(npc_sheet),
            ..Default::default()
        };

        let combatants = vec![
            CombatantTickEntry {
                entity_id: "u-sara".to_string(),
                display_name: "Sara (Valkyrie)".to_string(),
                role_or_faction: "Player".to_string(),
                ooda_score: 13,
                next_action_tick: 8,
                active_stance: Some("PUNCH (T7/C7/F7)".to_string()),
                fatigue_level: 0,
                wound_shock: 0,
                grid_x: Some(4),
                grid_y: Some(6),
                zone_label: Some("Core Sector".to_string()),
                visibility: MessageVisibility::Public,
            },
            CombatantTickEntry {
                entity_id: "npc-guard".to_string(),
                display_name: "Gate Sentry".to_string(),
                role_or_faction: "Enemy".to_string(),
                ooda_score: 11,
                next_action_tick: 12,
                active_stance: Some("STABILITY".to_string()),
                fatigue_level: 3,
                wound_shock: 2,
                grid_x: Some(10),
                grid_y: Some(6),
                zone_label: Some("Gate".to_string()),
                visibility: MessageVisibility::Public,
            },
        ];

        let planned = PlannedCombatActionState {
            technique_code: Some("punch".to_string()),
            technique_name: Some("Strike: Punch / Palm / Elbow".to_string()),
            tempo: 8,
            control: 6,
            force: 5,
            p_eff: 19,
            effective_tc: Some(7),
            valid: true,
            archetype_label: Some("Balanced".to_string()),
        };

        // 1. GM / Solo view exposes exact NPC Fatigue/Shock/Wounds + both inspect buttons
        let gm_html = format_combat_tracker_rich_html(&CombatTrackerRenderContext {
            in_session: true,
            round: 1,
            tick: 4,
            combatants: &combatants,
            roster: std::slice::from_ref(&self_part),
            self_participant: Some(&self_part),
            character_sheet: &sheet,
            session_npcs: std::slice::from_ref(&npc_entry),
            can_see_gm_secrets: true,
            planned_action: &planned,
        });
        assert!(gm_html.contains("Fatigue L2/5"));
        assert!(gm_html.contains("1 Wound (T2)"));
        assert!(gm_html.contains("Bleed G2"));
        assert!(gm_html.contains("Planned: Strike: Punch / Palm / Elbow (T8/C6/F5 · 7t)"));
        assert!(gm_html.contains("window.satyaOpenCombatantDetails('self', '')"));
        assert!(gm_html.contains("Fatigue L3/5"));
        assert!(gm_html.contains("Shock 2/5"));
        assert!(gm_html.contains("1 Wound (T3, 1 Fx)"));
        assert!(gm_html.contains("window.satyaOpenCombatantDetails('npc', 'npc-guard')"));

        // 2. Non-GM player view shows qualitative apparent condition ONLY for NPCs (never numeric Fatigue/Shock)
        let player_html = format_combat_tracker_rich_html(&CombatTrackerRenderContext {
            in_session: true,
            round: 1,
            tick: 4,
            combatants: &combatants,
            roster: std::slice::from_ref(&self_part),
            self_participant: Some(&self_part),
            character_sheet: &sheet,
            session_npcs: std::slice::from_ref(&npc_entry),
            can_see_gm_secrets: false,
            planned_action: &planned,
        });
        assert!(player_html.contains("Fatigue L2/5")); // Own PC stats visible
        assert!(player_html.contains("Winded / Wounded"));
        assert!(!player_html.contains("Fatigue L3/5"));
        assert!(!player_html.contains("Shock 2/5"));
        assert!(!player_html.contains("1 Wound (T3, 1 Fx)"));

        // 3. Offline fallback with empty roster & empty combatants renders active CharacterSheetState
        let healthy_sheet = CharacterSheetState::default();
        let offline_html = format_combat_tracker_rich_html(&CombatTrackerRenderContext {
            in_session: false,
            round: 1,
            tick: 1,
            combatants: &[],
            roster: &[],
            self_participant: None,
            character_sheet: &healthy_sheet,
            session_npcs: &[],
            can_see_gm_secrets: true,
            planned_action: &PlannedCombatActionState::default(),
        });
        assert!(offline_html.contains(&escape_html(&healthy_sheet.character_name)));
        assert!(offline_html.contains("Local PC"));
        assert!(offline_html.contains("Fatigue L"));
        assert!(offline_html.contains("Intact"));
        assert!(offline_html.contains("Sheet ↗"));
    }
}
