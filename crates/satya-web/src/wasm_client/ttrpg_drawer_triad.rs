//! Tactical Triad (`Tempo` / `Control` / `Force`) & Combat Technique helpers for `#ttrpg-drawer`
//! (`Task Resolution & Action Pool`).

use crate::wasm_client::chat_and_ttrpg::satya_update_cascade_preview;
use crate::wasm_client::dom::{
    escape_html, input_value, select_value, set_display, set_html, set_input_value,
    set_select_value,
};
use crate::wasm_client::state::{
    PlannedCombatActionState, SECONDS_PER_TICK, with_state, with_state_mut,
};
use satya_engine::{
    ActionTimeFactorParams, CombatDomain, CombatTechniqueId, TacticalPoolEnvelope,
    TacticalPoolParams, TechniqueTriadEvalParams, Triad, compute_tactical_pool,
    evaluate_technique_with_triad,
};
use wasm_bindgen::prelude::*;

/// Inputs for evaluating the Action Mode & Tactical Triad (`T / C / F`) preview shared between
/// `#ttrpg-drawer` and `#notebook-pane-combat`.
#[derive(Debug, Clone, Copy)]
pub struct DrawerTriadInputs<'a> {
    pub technique_raw: &'a str,
    pub tempo: i32,
    pub control: i32,
    pub force: i32,
    pub skill_c: i32,
    pub skill_n: usize,
    pub fatigue_level: i32,
    pub wound_shock_index: f32,
}

/// Pure output of [`evaluate_drawer_triad_preview`] for Combat Tracker display,
/// Task Roller pool-violation warnings, and [`PlannedCombatActionState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawerTriadPreviewOutput {
    pub combat_tracker_html: String,
    pub drawer_warning_html: Option<String>,
    pub planned_action: PlannedCombatActionState,
}

/// Resolves a `<select>` option value (either a legacy [`satya_schema::CombatStance`] code or a
/// canonical [`CombatTechniqueId`] code) into an optional [`CombatTechniqueId`].
#[must_use]
pub fn resolve_combat_technique_from_select(raw: &str) -> Option<CombatTechniqueId> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("non_combat")
        || trimmed.eq_ignore_ascii_case("none")
    {
        return None;
    }
    match trimmed {
        "POWER" => Some(CombatTechniqueId::Swing),
        "MANEUVER" => Some(CombatTechniqueId::Probe),
        "STABILITY" => Some(CombatTechniqueId::Parry),
        "RANGED_AIMED" => Some(CombatTechniqueId::DeliberateAimedShot),
        "RANGED_SNAP" => Some(CombatTechniqueId::SnapShot),
        _ => CombatTechniqueId::from_code_str(trimmed),
    }
}

/// Computes the active character's [`TacticalPoolEnvelope`] for the current `#ttrpg-drawer` settings.
pub fn compute_active_drawer_envelope(
    skill_c: i32,
    skill_n: usize,
    is_ranged: bool,
    fatigue_level: i32,
    wound_shock_index: f32,
) -> TacticalPoolEnvelope {
    let mut chars = with_state(|st| st.character_sheet.characteristics.clone());
    if chars.spd <= 0 {
        chars.spd = skill_c.clamp(8, 18);
    }
    if chars.for_stat <= 0 {
        chars.for_stat = skill_c.clamp(8, 18);
    }
    if chars.dex <= 0 {
        chars.dex = skill_c.clamp(8, 18);
    }
    let params = TacticalPoolParams {
        chars,
        skill_c: skill_c.clamp(1, 20),
        skill_n: skill_n.clamp(1, 10),
        is_ranged,
        fatigue_level: fatigue_level.max(0),
        wound_shock_index: wound_shock_index.round() as i32,
        ..Default::default()
    };
    compute_tactical_pool(&params)
}

/// Computes a valid `(Tempo, Control, Force)` integer split for a preset mode (`"balanced"`,
/// `"blitz_tempo"`, `"max_control"`, or `"heavy_force"`) bounded by `envelope.p_eff` and aspect caps.
pub fn compute_preset_triad_allocation(
    envelope: &TacticalPoolEnvelope,
    preset: &str,
) -> (i32, i32, i32) {
    let b = envelope.neutral_baseline_b;
    let p_eff = envelope.p_eff.max(0);
    let t_cap = envelope.tempo_cap.max(0);
    let c_cap = envelope.control_cap.max(0);
    let f_cap = envelope.force_cap.max(0);

    let (mut t, mut c, mut f) = match preset {
        "blitz_tempo" => {
            let t0 = t_cap.min(p_eff);
            let rem = (p_eff - t0).max(0);
            let c0 = (rem / 2).min(c_cap);
            let f0 = (rem - c0).min(f_cap);
            (t0, c0, f0)
        }
        "max_control" => {
            let c0 = c_cap.min(p_eff);
            let rem = (p_eff - c0).max(0);
            let t0 = (rem / 2).min(t_cap);
            let f0 = (rem - t0).min(f_cap);
            (t0, c0, f0)
        }
        "heavy_force" => {
            let f0 = f_cap.min(p_eff);
            let rem = (p_eff - f0).max(0);
            let c0 = (rem / 2).min(c_cap);
            let t0 = (rem - c0).min(t_cap);
            (t0, c0, f0)
        }
        _ => (b.min(t_cap), b.min(c_cap), b.min(f_cap)),
    };

    while t + c + f > p_eff && (t > 0 || c > 0 || f > 0) {
        if f >= c && f >= t && f > 0 {
            f -= 1;
        } else if c >= t && c > 0 {
            c -= 1;
        } else if t > 0 {
            t -= 1;
        }
    }
    (t, c, f)
}

/// Returns the default SSR HTML for `#combat-action-mode-readout`.
pub fn default_ttrpg_triad_readout_html() -> String {
    format_drawer_triad_readout("non_combat", 7, 7, 7, 12, 3, 1, 0.0)
}

/// Evaluates the Action Mode & Tactical Triad (`T / C / F`) preview HTML for the Combat Tracker,
/// an optional compact Task Roller warning pill (only when a combat technique has a T/C/F violation),
/// and the [`PlannedCombatActionState`] snapshot.
#[must_use]
pub fn evaluate_drawer_triad_preview(inputs: &DrawerTriadInputs<'_>) -> DrawerTriadPreviewOutput {
    let tech_opt = resolve_combat_technique_from_select(inputs.technique_raw);
    let domain = match tech_opt {
        Some(tid) => {
            let spec = tid.spec();
            if spec.allowed_ranged && !spec.allowed_unarmed {
                CombatDomain::Ranged
            } else if spec.allowed_armed_melee && !spec.allowed_unarmed {
                CombatDomain::ArmedMelee
            } else {
                CombatDomain::UnarmedMartialArts
            }
        }
        None => CombatDomain::UnarmedMartialArts,
    };

    let env = compute_active_drawer_envelope(
        inputs.skill_c,
        inputs.skill_n,
        domain == CombatDomain::Ranged,
        inputs.fatigue_level,
        inputs.wound_shock_index,
    );
    let tempo = inputs.tempo;
    let control = inputs.control;
    let force = inputs.force;
    let sum = tempo + control + force;

    let Some(tech_id) = tech_opt else {
        let combat_tracker_html = format!(
            r#"<div class="cascade-summary-header">
                <span><strong>Action Mode:</strong> Non-Combat Task (Standard 2d20 Skill Check)</span>
                <span class="mono">Tactical Pool P_eff = <strong>{}/{}</strong> · Neutral B = <strong>{}</strong> · Caps: T&le;{} · C&le;{} · F&le;{}</span>
            </div>
            <div class="mono" style="font-size:0.78rem;opacity:0.88;">
                Planned Triad: T=<strong>{tempo}</strong> · C=<strong>{control}</strong> · F=<strong>{force}</strong> (Sum {sum}/{}). Select a <strong>Combat Action / Technique</strong> above or in Task Resolution to activate Count-Up Tick &amp; T/C/F resolution.
            </div>"#,
            env.p_eff,
            env.p_base,
            env.neutral_baseline_b,
            env.tempo_cap,
            env.control_cap,
            env.force_cap,
            env.p_eff,
        );
        return DrawerTriadPreviewOutput {
            combat_tracker_html,
            drawer_warning_html: None,
            planned_action: PlannedCombatActionState {
                technique_code: None,
                technique_name: None,
                tempo,
                control,
                force,
                p_eff: env.p_eff,
                effective_tc: None,
                valid: true,
                archetype_label: None,
            },
        };
    };

    let eval_params = TechniqueTriadEvalParams {
        technique: tech_id,
        domain,
        envelope: env.clone(),
        alloc: Triad::new(tempo, control, force),
        skill_level_n: inputs.skill_n.clamp(1, 10),
        time_factor_params: ActionTimeFactorParams {
            fatigue_level: inputs.fatigue_level.max(0),
            wound_shock_index: f64::from(inputs.wound_shock_index.max(0.0)),
            ..Default::default()
        },
        weapon_recoil_r: if domain == CombatDomain::Ranged { 4 } else { 0 },
        ..Default::default()
    };
    let res = evaluate_technique_with_triad(&eval_params);
    let spec = tech_id.spec();
    let eff_secs = f64::from(res.effective_tc) * SECONDS_PER_TICK;

    let status_html = if res.valid {
        format!(
            r#"<span class="pill pill-active">Valid T/C/F ({sum}/{}) · {}</span>"#,
            env.p_eff,
            escape_html(&res.archetype_label)
        )
    } else {
        format!(
            r#"<span class="pill pill-danger">Pool Violation ({sum}/{}): {}</span>"#,
            env.p_eff,
            escape_html(&res.violations.join(" "))
        )
    };

    let combat_tracker_html = format!(
        r#"<div class="cascade-summary-header">
            <span><strong>Action:</strong> {} <span class="mono">[{}]</span></span>
            {status_html}
        </div>
        <div class="mono" style="font-size:0.79rem;line-height:1.45;">
            <strong>Clock &amp; Tick Cost:</strong> Base <strong>{} TC</strong> (Decide {} + Act {}) &rarr; <strong>Effective {} Ticks ({eff_secs:.2}s)</strong> (Wind-Up {}t / Recovery {}t) ·
            <strong>Triad:</strong> T=<strong>{tempo}</strong> (&le;{}) · C=<strong>{control}</strong> (&le;{}) · F=<strong>{force}</strong> (&le;{}) [Neutral B={}]<br/>
            <strong>Combat Output:</strong> Tech &Delta;_ext=<strong>{:+}</strong> · Guard=<strong>{}</strong> · Root=<strong>{:.1}</strong> · Strike Impact=<strong>{:.1}</strong> · Recoil R_eff=<strong>{}</strong> (Unbraced &Delta;_ext={:+}) · Exertion=<strong>+{}</strong>
        </div>"#,
        escape_html(spec.name),
        escape_html(domain.label()),
        res.action_cost.base_total_tc,
        res.action_cost.effective_tc_decide,
        res.action_cost.effective_tc_act,
        res.effective_tc,
        res.action_cost.wind_up_ticks,
        res.action_cost.recovery_ticks,
        env.tempo_cap,
        env.control_cap,
        env.force_cap,
        env.neutral_baseline_b,
        res.technique_delta_ext,
        res.effective_guard,
        res.effective_root,
        res.strike_impact,
        res.ranged_effective_recoil_r,
        res.ranged_unbraced_dispersion_delta_ext,
        res.exertion_marks_gained
    );

    let drawer_warning_html = (!res.valid).then(|| {
        format!(
            r#"<span class="pill pill-danger">T/C/F Violation ({sum}/{}): {}</span><button type="button" class="btn-chip" onclick="window.satyaSelectNotebookTab('combat')">Combat Tracker ↗</button>"#,
            env.p_eff,
            escape_html(&res.violations.join(" "))
        )
    });

    let planned_action = PlannedCombatActionState {
        technique_code: Some(tech_id.as_str().to_string()),
        technique_name: Some(spec.name.to_string()),
        tempo,
        control,
        force,
        p_eff: env.p_eff,
        effective_tc: Some(res.effective_tc),
        valid: res.valid,
        archetype_label: Some(res.archetype_label),
    };

    DrawerTriadPreviewOutput {
        combat_tracker_html,
        drawer_warning_html,
        planned_action,
    }
}

/// Formats the live Action Mode & Tactical Triad HTML readout for `#combat-action-mode-readout`.
#[allow(clippy::too_many_arguments)]
pub fn format_drawer_triad_readout(
    technique_raw: &str,
    tempo: i32,
    control: i32,
    force: i32,
    skill_c: i32,
    skill_n: usize,
    fatigue_level: i32,
    wound_shock_index: f32,
) -> String {
    evaluate_drawer_triad_preview(&DrawerTriadInputs {
        technique_raw,
        tempo,
        control,
        force,
        skill_c,
        skill_n,
        fatigue_level,
        wound_shock_index,
    })
    .combat_tracker_html
}

fn sync_skill_suggestion_for_technique(tech_opt: Option<CombatTechniqueId>) {
    if let Some(tid) = tech_opt {
        let spec = tid.spec();
        let current_skill = input_value("ttrpg-skill");
        if current_skill.trim().is_empty() || current_skill == "Trauma Surgery" {
            let suggested_skill = if spec.allowed_ranged && !spec.allowed_unarmed {
                "Pistols & Carbines"
            } else if spec.allowed_armed_melee && !spec.allowed_unarmed {
                "Melee Weapons"
            } else {
                "Unarmed Combat"
            };
            set_input_value("ttrpg-skill", suggested_skill);
        }
    }
}

/// Invoked when `#ttrpg-action-technique` changes in `#ttrpg-drawer`.
/// Two-way syncs `#ttrpg-stance` in the Combat Tracker, applies the Triad preset, and refreshes previews.
#[wasm_bindgen]
pub fn satya_on_drawer_technique_change() {
    let tech_raw = select_value("ttrpg-action-technique");
    let resolved = resolve_combat_technique_from_select(&tech_raw);
    set_select_value(
        "ttrpg-stance",
        resolved.map(CombatTechniqueId::as_str).unwrap_or(""),
    );
    sync_skill_suggestion_for_technique(resolved);
    satya_apply_triad_preset();
    satya_update_cascade_preview();
}

/// Invoked when `#ttrpg-stance` changes in `#notebook-pane-combat` (`Combat Tracker`).
/// Two-way syncs `#ttrpg-action-technique` in `#ttrpg-drawer`, applies the Triad preset, and refreshes previews.
#[wasm_bindgen]
pub fn satya_on_combat_tracker_technique_change() {
    let stance_raw = select_value("ttrpg-stance");
    let resolved = resolve_combat_technique_from_select(&stance_raw);
    set_select_value(
        "ttrpg-action-technique",
        resolved
            .map(CombatTechniqueId::as_str)
            .unwrap_or("non_combat"),
    );
    sync_skill_suggestion_for_technique(resolved);
    satya_apply_triad_preset();
    satya_update_cascade_preview();
}

/// Applies the selected `#ttrpg-triad-preset` (`"balanced"`, `"blitz_tempo"`, `"max_control"`, `"heavy_force"`)
/// to `#ttrpg-pool-tempo`, `#ttrpg-pool-control`, and `#ttrpg-pool-force`.
#[wasm_bindgen]
pub fn satya_apply_triad_preset() {
    let preset = select_value("ttrpg-triad-preset");
    if preset == "custom" {
        satya_update_drawer_triad_preview();
        return;
    }
    let tech_raw = select_value("ttrpg-action-technique");
    let is_ranged = resolve_combat_technique_from_select(&tech_raw)
        .map(|tid| tid.spec().allowed_ranged && !tid.spec().allowed_unarmed)
        .unwrap_or(false);
    let skill_n = input_value("ttrpg-skill-level")
        .parse::<usize>()
        .unwrap_or(3)
        .clamp(1, 10);
    let control_c = input_value("ttrpg-control-c")
        .parse::<i32>()
        .unwrap_or(12)
        .clamp(1, 20);
    let fatigue = input_value("ttrpg-fatigue").parse::<i32>().unwrap_or(0);
    let wound_shock = input_value("ttrpg-wound-shock")
        .parse::<f32>()
        .unwrap_or(0.0);

    let env = compute_active_drawer_envelope(control_c, skill_n, is_ranged, fatigue, wound_shock);
    let (t, c, f) = compute_preset_triad_allocation(
        &env,
        if preset.is_empty() {
            "balanced"
        } else {
            &preset
        },
    );
    set_input_value("ttrpg-pool-tempo", &t.to_string());
    set_input_value("ttrpg-pool-control", &c.to_string());
    set_input_value("ttrpg-pool-force", &f.to_string());
    satya_update_drawer_triad_preview();
}

/// Recalculates and updates `#combat-action-mode-readout`, `#ttrpg-triad-warning`,
/// `SatyaWasmState::planned_combat_action`, and `#combat-tracker-list` when `T`, `C`, `F`,
/// or any drawer stat changes.
#[wasm_bindgen]
pub fn satya_update_drawer_triad_preview() {
    let tech_raw = select_value("ttrpg-action-technique");
    let tempo = input_value("ttrpg-pool-tempo")
        .parse::<i32>()
        .unwrap_or(7)
        .max(0);
    let control = input_value("ttrpg-pool-control")
        .parse::<i32>()
        .unwrap_or(7)
        .max(0);
    let force = input_value("ttrpg-pool-force")
        .parse::<i32>()
        .unwrap_or(7)
        .max(0);
    let skill_n = input_value("ttrpg-skill-level")
        .parse::<usize>()
        .unwrap_or(3)
        .clamp(1, 10);
    let control_c = input_value("ttrpg-control-c")
        .parse::<i32>()
        .unwrap_or(12)
        .clamp(1, 20);
    let fatigue = input_value("ttrpg-fatigue").parse::<i32>().unwrap_or(0);
    let wound_shock = input_value("ttrpg-wound-shock")
        .parse::<f32>()
        .unwrap_or(0.0);

    let out = evaluate_drawer_triad_preview(&DrawerTriadInputs {
        technique_raw: &tech_raw,
        tempo,
        control,
        force,
        skill_c: control_c,
        skill_n,
        fatigue_level: fatigue,
        wound_shock_index: wound_shock,
    });
    set_html("combat-action-mode-readout", &out.combat_tracker_html);
    if let Some(warn) = out.drawer_warning_html {
        set_html("ttrpg-triad-warning", &warn);
        set_display("ttrpg-triad-warning", "flex");
    } else {
        set_html("ttrpg-triad-warning", "");
        set_display("ttrpg-triad-warning", "none");
    }
    with_state_mut(|st| {
        st.planned_combat_action = out.planned_action;
    });
    crate::wasm_client::notebook::render_combat_tracker();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_preset_triad_allocation_respects_caps_and_p_eff() {
        let mut env = compute_active_drawer_envelope(12, 3, false, 0, 0.0);
        for preset in ["balanced", "blitz_tempo", "max_control", "heavy_force"] {
            let (t, c, f) = compute_preset_triad_allocation(&env, preset);
            assert!(
                t >= 0 && t <= env.tempo_cap,
                "tempo cap violated for {preset}"
            );
            assert!(
                c >= 0 && c <= env.control_cap,
                "control cap violated for {preset}"
            );
            assert!(
                f >= 0 && f <= env.force_cap,
                "force cap violated for {preset}"
            );
            assert!(
                t + c + f <= env.p_eff,
                "sum {} exceeded p_eff {} for {preset}",
                t + c + f,
                env.p_eff
            );
        }

        // Verify tight fatigue/wound-shock clamping when 3 * neutral_baseline_b > p_eff
        env.p_eff = 8;
        env.tempo_cap = 5;
        env.control_cap = 6;
        env.force_cap = 4;
        env.neutral_baseline_b = 5;
        let (tb, cb, fb) = compute_preset_triad_allocation(&env, "balanced");
        assert!(tb <= 5 && cb <= 6 && fb <= 4);
        assert_eq!(tb + cb + fb, 8);
    }

    #[test]
    fn test_format_drawer_triad_readout_non_combat_and_technique_modes() {
        let non_combat = format_drawer_triad_readout("non_combat", 7, 7, 7, 12, 3, 0, 0.0);
        assert!(non_combat.contains("Non-Combat Task"));
        assert!(non_combat.contains("Tactical Pool P_eff"));

        let env = compute_active_drawer_envelope(12, 3, false, 0, 0.0);
        let (t, c, f) = compute_preset_triad_allocation(&env, "balanced");
        let punch_ok = format_drawer_triad_readout("punch", t, c, f, 12, 3, 0, 0.0);
        assert!(punch_ok.contains("Valid T/C/F"));
        assert!(punch_ok.contains("Effective"));
        assert!(punch_ok.contains("Ticks ("));

        let punch_over = format_drawer_triad_readout("punch", 30, 30, 30, 12, 3, 0, 0.0);
        assert!(punch_over.contains("Pool Violation"));
    }

    #[test]
    fn test_resolve_combat_technique_and_drawer_warning_behavior() {
        assert_eq!(resolve_combat_technique_from_select(""), None);
        assert_eq!(resolve_combat_technique_from_select("non_combat"), None);
        assert_eq!(
            resolve_combat_technique_from_select("POWER"),
            Some(CombatTechniqueId::Swing)
        );
        assert_eq!(
            resolve_combat_technique_from_select("RANGED_SNAP"),
            Some(CombatTechniqueId::SnapShot)
        );
        assert_eq!(
            resolve_combat_technique_from_select("punch"),
            Some(CombatTechniqueId::Punch)
        );

        let non_combat_out = evaluate_drawer_triad_preview(&DrawerTriadInputs {
            technique_raw: "non_combat",
            tempo: 7,
            control: 7,
            force: 7,
            skill_c: 12,
            skill_n: 3,
            fatigue_level: 0,
            wound_shock_index: 0.0,
        });
        assert!(non_combat_out.drawer_warning_html.is_none());
        assert_eq!(non_combat_out.planned_action.technique_code, None);
        assert!(non_combat_out.planned_action.valid);

        let env = compute_active_drawer_envelope(12, 3, false, 0, 0.0);
        let (t, c, f) = compute_preset_triad_allocation(&env, "balanced");
        let valid_out = evaluate_drawer_triad_preview(&DrawerTriadInputs {
            technique_raw: "punch",
            tempo: t,
            control: c,
            force: f,
            skill_c: 12,
            skill_n: 3,
            fatigue_level: 0,
            wound_shock_index: 0.0,
        });
        assert!(valid_out.drawer_warning_html.is_none());
        assert_eq!(
            valid_out.planned_action.technique_code.as_deref(),
            Some("punch")
        );
        assert!(valid_out.planned_action.effective_tc.is_some());
        assert!(valid_out.planned_action.valid);

        let bad_out = evaluate_drawer_triad_preview(&DrawerTriadInputs {
            technique_raw: "punch",
            tempo: 25,
            control: 25,
            force: 25,
            skill_c: 12,
            skill_n: 3,
            fatigue_level: 0,
            wound_shock_index: 0.0,
        });
        let warn = bad_out
            .drawer_warning_html
            .expect("violation warning must emit on illegal T/C/F");
        assert!(warn.contains("T/C/F Violation"));
        assert!(warn.contains("window.satyaSelectNotebookTab('combat')"));
        assert!(!bad_out.planned_action.valid);
    }
}
