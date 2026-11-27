//! Step 4 Lifepath extensions: *Awakened* Faction Quick-Start Templates,
//! Youth & Off-Duty Personal Hobbies (`-15%` club SP discount), and `2d20` Formative
//! Life Events with moral/coping Choice Branches.

use crate::wasm_client::dom::{escape_html, set_feedback};
use crate::wasm_client::state::with_state_mut;
use crate::wasm_client::wizard::{CharacterWizardState, render_character_wizard};
use satya_campaign::{FactionArchetypeId, FactionArchetypeSpec};
use satya_engine::{
    CareerTermConfig, FormativeLifeEventId, LifeEventChoiceBranch, LifepathBuildConfig,
    PersonalHobbyId, resolve_formative_life_event,
};
use wasm_bindgen::prelude::*;

fn parse_branch_code(val: &str) -> LifeEventChoiceBranch {
    match val.trim().to_ascii_uppercase().as_str() {
        "BRANCH_A" | "A" => LifeEventChoiceBranch::BranchA,
        "BRANCH_B" | "B" => LifeEventChoiceBranch::BranchB,
        _ => LifeEventChoiceBranch::DefaultPath,
    }
}

fn parse_optional_hobby(val: &str) -> Option<PersonalHobbyId> {
    let trimmed = val.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("NONE") {
        None
    } else {
        PersonalHobbyId::from_code_str(trimmed)
    }
}

fn parse_life_event_roll_input<F: FnMut(i32) -> i32>(val: &str, rng: &mut F) -> Option<i32> {
    let trimmed = val.trim();
    if trimmed.eq_ignore_ascii_case("ROLL") {
        Some((rng(20) + rng(20)).clamp(2, 40))
    } else if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("NONE") {
        None
    } else {
        trimmed.parse::<i32>().ok().map(|n| n.clamp(2, 40))
    }
}

/// Applies one of the 16 *Awakened* [`FactionArchetypeId`] templates (or `"NONE"` to clear)
/// onto [`CharacterWizardState`], updating Lifepath, Career Terms, Hobbies, Life Event, and 9 Psyche traits.
pub fn apply_faction_template_to_wizard(
    wiz: &mut CharacterWizardState,
    arch_code: &str,
) -> Option<FactionArchetypeSpec> {
    let trimmed = arch_code.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("NONE") {
        wiz.lifepath.faction_archetype = None;
        wiz.sync_lifepath_skills();
        return None;
    }
    let arch = FactionArchetypeId::from_code_str(trimmed)?;
    let spec = arch.spec();

    wiz.lifepath.faction_archetype = Some(arch.as_str().to_string());
    wiz.lifepath.origin_roll_2d20 = spec.origin_roll_2d20;
    wiz.lifepath.stage4_path_override = Some(spec.stage4_path);
    if let Some(hs) = spec.stage4a_hs_track {
        wiz.lifepath.stage4a_hs_track = hs;
    }
    wiz.lifepath.youth_hobby = spec.preferred_hobbies.first().copied();
    wiz.lifepath.adolescent_life_event_roll_2d20 = Some(spec.recommended_life_event_roll_2d20);
    wiz.lifepath.adolescent_life_event_choice = LifeEventChoiceBranch::DefaultPath;

    let off_duty_hobby = spec
        .preferred_hobbies
        .get(1)
        .or_else(|| spec.preferred_hobbies.first())
        .copied();
    let terms: Vec<CareerTermConfig> = spec
        .career_tracks
        .iter()
        .take(4)
        .enumerate()
        .map(|(idx, &trk)| CareerTermConfig {
            track: trk,
            waiver_override: true,
            off_duty_hobby: if idx == 0 { off_duty_hobby } else { None },
            life_event_roll_2d20: if idx == 0 {
                Some(spec.recommended_life_event_roll_2d20)
            } else {
                None
            },
            ..CareerTermConfig::default()
        })
        .collect();
    if !terms.is_empty() {
        wiz.lifepath.career_terms = terms;
    }

    wiz.psyche_mode = "CHOOSE".to_string();
    wiz.psyche_pool = None;
    wiz.psyche_slots = [None; 9];
    wiz.characteristics.set_psyche_profile(spec.psyche);
    wiz.rebuild_and_auto_allocate_skills();
    Some(spec)
}

/// Applies `"youth_hobby"`, `"adolescent_event_roll"`, or `"adolescent_event_choice"` onto `lp`.
pub(crate) fn apply_lifepath_extra_field<F: FnMut(i32) -> i32>(
    lp: &mut LifepathBuildConfig,
    field: &str,
    val: &str,
    rng: &mut F,
) -> bool {
    match field {
        "youth_hobby" => {
            lp.youth_hobby = parse_optional_hobby(val);
            true
        }
        "adolescent_event_roll" => {
            lp.adolescent_life_event_roll_2d20 = parse_life_event_roll_input(val, rng);
            lp.adolescent_life_event_choice = LifeEventChoiceBranch::DefaultPath;
            true
        }
        "adolescent_event_choice" => {
            lp.adolescent_life_event_choice = parse_branch_code(val);
            true
        }
        _ => false,
    }
}

/// Applies `"off_duty_hobby"`, `"life_event_roll"`, or `"life_event_choice"` onto a Career Term.
pub(crate) fn apply_career_term_extra_field<F: FnMut(i32) -> i32>(
    term: &mut CareerTermConfig,
    field: &str,
    val: &str,
    rng: &mut F,
) -> bool {
    match field {
        "off_duty_hobby" => {
            term.off_duty_hobby = parse_optional_hobby(val);
            true
        }
        "life_event_roll" => {
            term.life_event_roll_2d20 = parse_life_event_roll_input(val, rng);
            term.life_event_choice = LifeEventChoiceBranch::DefaultPath;
            true
        }
        "life_event_choice" => {
            term.life_event_choice = parse_branch_code(val);
            true
        }
        _ => false,
    }
}

/// Applies an *Awakened* Faction Quick-Start Archetype (`"NONE"` or [`FactionArchetypeId`] code)
/// to Step 2 Psyche traits and Step 4 Lifepath/Careers/Hobbies in the Character Creation Wizard.
#[wasm_bindgen]
pub fn satya_wizard_apply_faction_template(arch_code: String) {
    let applied_label = with_state_mut(|st| {
        apply_faction_template_to_wizard(&mut st.wizard, &arch_code).map(|spec| {
            format!(
                "Applied Faction Archetype '{} ({})' — updated 9 Psyche traits, Lifepath, Hobbies & Career Tracks.",
                spec.id.label(),
                spec.faction.label(),
            )
        })
    });
    render_character_wizard();
    if let Some(msg) = applied_label {
        set_feedback("wizard-feedback", &msg, false);
    } else {
        set_feedback(
            "wizard-feedback",
            "Cleared Faction Archetype quick-start overlay (kept manual lifepath selections).",
            false,
        );
    }
}

fn format_hobby_options_html(selected: Option<PersonalHobbyId>, empty_label: &str) -> String {
    let none_sel = if selected.is_none() { " selected" } else { "" };
    let mut out = format!(r#"<option value="NONE"{none_sel}>{empty_label}</option>"#);
    for &hobby in &PersonalHobbyId::ALL {
        let spec = hobby.spec();
        let sel = if selected == Some(hobby) {
            " selected"
        } else {
            ""
        };
        let skills_hint = spec.signature_skill_ids.join(", ");
        out.push_str(&format!(
            r#"<option value="{code}"{sel} title="{skills_hint}">{label}</option>"#,
            code = hobby.as_str(),
            label = escape_html(hobby.label()),
            skills_hint = escape_html(&skills_hint),
        ));
    }
    out
}

fn format_life_event_select_and_branch_html(
    select_id: &str,
    choice_id: &str,
    roll_opt: Option<i32>,
    choice: LifeEventChoiceBranch,
    roll_onchange: &str,
    choice_onchange: &str,
    roll_btn_onclick: &str,
) -> String {
    let none_sel = if roll_opt.is_none() { " selected" } else { "" };
    let active_evt = roll_opt.map(FormativeLifeEventId::from_roll_2d20);
    let mut evt_opts =
        format!(r#"<option value="NONE"{none_sel}>-- None / Skip Event --</option>"#);
    for &ev in &FormativeLifeEventId::ALL {
        let rep_roll = ev.representative_roll_2d20();
        let sel = if active_evt == Some(ev) {
            " selected"
        } else {
            ""
        };
        evt_opts.push_str(&format!(
            r#"<option value="{rep_roll}"{sel}>{label}</option>"#,
            label = escape_html(ev.label()),
        ));
    }

    let detail_html = if let Some(roll) = roll_opt {
        let outcome = resolve_formative_life_event(roll, choice, false);
        let choice_html = if let Some((a_lbl, b_lbl)) = outcome.event_id.branch_labels() {
            let def_sel = if choice == LifeEventChoiceBranch::DefaultPath {
                " selected"
            } else {
                ""
            };
            let a_sel = if choice == LifeEventChoiceBranch::BranchA {
                " selected"
            } else {
                ""
            };
            let b_sel = if choice == LifeEventChoiceBranch::BranchB {
                " selected"
            } else {
                ""
            };
            format!(
                r#"<label for="{choice_id}">
                    <span>Moral / Coping Branch Choice</span>
                    <select id="{choice_id}" onchange="{choice_onchange}">
                        <option value="DEFAULT_PATH"{def_sel}>Default / Standard Resolution</option>
                        <option value="BRANCH_A"{a_sel}>{a_lbl}</option>
                        <option value="BRANCH_B"{b_sel}>{b_lbl}</option>
                    </select>
                </label>"#,
                a_lbl = escape_html(a_lbl),
                b_lbl = escape_html(b_lbl),
            )
        } else {
            String::new()
        };
        format!(
            r#"{choice_html}<div class="stat-note"><strong>{sum}</strong> — {hook}</div>"#,
            sum = escape_html(&outcome.summary_line),
            hook = escape_html(outcome.narrative_hook),
        )
    } else {
        String::new()
    };

    format!(
        r#"<div class="wiz-char-assign-card">
            <label for="{select_id}">
                <span>2d20 Formative Life Event</span>
                <select id="{select_id}" onchange="{roll_onchange}">
                    {evt_opts}
                </select>
            </label>
            <div class="btn-row-compact">
                <button type="button" class="btn btn-sm btn-secondary" onclick="{roll_btn_onclick}">
                    "Roll 2d20 Life Event"
                </button>
            </div>
            {detail_html}
        </div>"#
    )
}

/// Renders the optional *Awakened* 5-Faction Quick-Start Archetype selector at the top of Step 4.
#[must_use]
pub fn format_faction_quickstart_fieldset_html(st: &CharacterWizardState) -> String {
    let cur = st
        .lifepath
        .faction_archetype
        .as_deref()
        .and_then(FactionArchetypeId::from_code_str);
    let none_sel = if cur.is_none() { " selected" } else { "" };
    let mut opts =
        format!(r#"<option value="NONE"{none_sel}>-- Custom / Independent Lifepath --</option>"#);
    for &arch in &FactionArchetypeId::ALL {
        let sel = if cur == Some(arch) { " selected" } else { "" };
        opts.push_str(&format!(
            r#"<option value="{code}"{sel}>[{fac}] {label}</option>"#,
            code = arch.as_str(),
            fac = escape_html(arch.faction().as_str()),
            label = escape_html(arch.label()),
        ));
    }
    let summary_html = if let Some(arch) = cur {
        let spec = arch.spec();
        format!(
            r#"<div class="wiz-pool-readout">
                <strong>{role}</strong> · {fac} · <em>Wedge:</em> {wedge}
            </div>"#,
            role = escape_html(spec.default_callsign_or_role),
            fac = escape_html(spec.faction.label()),
            wedge = escape_html(spec.social_combat_wedge_hint),
        )
    } else {
        String::new()
    };
    format!(
        r#"<fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Awakened Faction Archetype Quick-Start (Optional)</span>
                <span class="pill pill-iggy">5 Factions · 14 Archetypes · 9-Trait Psyche Blueprints</span>
            </legend>
            <label for="wiz-s4-faction-archetype">
                <span>Select Faction Sub-Archetype (Pre-fills Origin, Hobbies, Career &amp; 9 Psyche Traits)</span>
                <select
                    id="wiz-s4-faction-archetype"
                    onchange="window.satyaWizardApplyFactionTemplate(this.value)"
                >
                    {opts}
                </select>
            </label>
            {summary_html}
        </fieldset>"#
    )
}

/// Renders Stage 3 Youth Club/Hobby (`-15%` Experience SP) and Adolescent Formative Life Event cards.
#[must_use]
pub fn format_youth_hobby_and_event_cards_html(cfg: &LifepathBuildConfig) -> String {
    let hobby_opts =
        format_hobby_options_html(cfg.youth_hobby, "-- None (No Youth Club Selected) --");
    let hobby_note = if let Some(h) = cfg.youth_hobby {
        let sp = h.spec();
        format!(
            r#"<div class="stat-note"><strong>-15% Experience SP (N&lt;=3):</strong> {} · <strong>Contact:</strong> {}</div>"#,
            escape_html(&sp.signature_skill_ids.join(", ")),
            escape_html(sp.grounded_contact_role),
        )
    } else {
        r#"<div class="stat-note">Clubs unlock a -15% Experience SP discount (N&lt;=3, min 3 SP) and a grounded campaign contact.</div>"#.to_string()
    };

    let event_card = format_life_event_select_and_branch_html(
        "wiz-s3-life-event",
        "wiz-s3-life-event-choice",
        cfg.adolescent_life_event_roll_2d20,
        cfg.adolescent_life_event_choice,
        "window.satyaWizardUpdateLifepathField('adolescent_event_roll', this.value)",
        "window.satyaWizardUpdateLifepathField('adolescent_event_choice', this.value)",
        "window.satyaWizardUpdateLifepathField('adolescent_event_roll', 'ROLL')",
    );

    format!(
        r#"<div class="wiz-char-assign-card">
            <label for="wiz-s3-youth-hobby">
                <span>Youth Personal Hobby / Club / Subculture</span>
                <select
                    id="wiz-s3-youth-hobby"
                    onchange="window.satyaWizardUpdateLifepathField('youth_hobby', this.value)"
                >
                    {hobby_opts}
                </select>
            </label>
            {hobby_note}
        </div>
        {event_card}"#
    )
}

/// Renders per-Term Off-Duty Hobby and `2d20` Formative Life Event controls for Stage 5 Career Term `t_idx`.
#[must_use]
pub fn format_term_hobby_and_event_controls_html(t_idx: usize, term: &CareerTermConfig) -> String {
    let hobby_opts =
        format_hobby_options_html(term.off_duty_hobby, "-- None (No Off-Duty Hobby) --");
    let roll_onchange =
        format!("window.satyaWizardUpdateCareerTerm({t_idx}, 'life_event_roll', this.value)");
    let choice_onchange =
        format!("window.satyaWizardUpdateCareerTerm({t_idx}, 'life_event_choice', this.value)");
    let roll_btn_onclick =
        format!("window.satyaWizardUpdateCareerTerm({t_idx}, 'life_event_roll', 'ROLL')");
    let event_html = format_life_event_select_and_branch_html(
        &format!("wiz-career-event-{t_idx}"),
        &format!("wiz-career-event-choice-{t_idx}"),
        term.life_event_roll_2d20,
        term.life_event_choice,
        &roll_onchange,
        &choice_onchange,
        &roll_btn_onclick,
    );

    format!(
        r#"<label for="wiz-career-hobby-{t_idx}">
            <span>Off-Duty Hobby / Club (-15% Experience SP, N&lt;=3)</span>
            <select
                id="wiz-career-hobby-{t_idx}"
                onchange="window.satyaWizardUpdateCareerTerm({t_idx}, 'off_duty_hobby', this.value)"
            >
                {hobby_opts}
            </select>
        </label>
        {event_html}"#
    )
}
