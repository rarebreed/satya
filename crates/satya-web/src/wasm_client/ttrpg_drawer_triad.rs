//! Tactical Triad (`Tempo` / `Control` / `Force`) & Combat Technique helpers for `#ttrpg-drawer`
//! (`Task Resolution & Action Pool`).

use crate::wasm_client::chat_and_ttrpg::satya_update_cascade_preview;
use crate::wasm_client::dom::{escape_html, input_value, select_value, set_html, set_input_value};
use crate::wasm_client::state::with_state;
use satya_engine::{
    ActionTimeFactorParams, CombatDomain, CombatTechniqueId, TacticalPoolEnvelope,
    TacticalPoolParams, TechniqueTriadEvalParams, Triad, compute_tactical_pool,
    evaluate_technique_with_triad,
};
use wasm_bindgen::prelude::*;

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

/// Returns the default SSR HTML for `#ttrpg-triad-readout`.
pub fn default_ttrpg_triad_readout_html() -> String {
    format_drawer_triad_readout("non_combat", 7, 7, 7, 12, 3, 1, 0.0)
}

/// Formats the live HTML readout for `#ttrpg-triad-readout` in `#ttrpg-drawer`.
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
    let raw_trimmed = technique_raw.trim();
    let tech_opt = if raw_trimmed.is_empty()
        || raw_trimmed.eq_ignore_ascii_case("non_combat")
        || raw_trimmed.eq_ignore_ascii_case("none")
    {
        None
    } else {
        CombatTechniqueId::from_code_str(raw_trimmed)
    };

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
        skill_c,
        skill_n,
        domain == CombatDomain::Ranged,
        fatigue_level,
        wound_shock_index,
    );
    let sum = tempo + control + force;

    let Some(tech_id) = tech_opt else {
        return format!(
            r#"<div class="cascade-summary-header">
                <span><strong>Action Mode:</strong> Non-Combat Task (Standard 2d20 Skill Check)</span>
                <span class="mono">Tactical Pool P_eff = <strong>{}/{}</strong> · Neutral B = <strong>{}</strong> · Caps: T&le;{} · C&le;{} · F&le;{}</span>
            </div>
            <div class="mono" style="font-size:0.78rem;opacity:0.88;">
                Select a <strong>Combat Action / Technique</strong> above (e.g. <em>Strike: Punch/Thrust</em>, <em>Control: Grab/Takedown</em>, or <em>Ranged: Snap Shot / Sighted Shot</em>) to activate Count-Up Tick &amp; Tempo/Control/Force (T/C/F) pool resolution.
            </div>"#,
            env.p_eff,
            env.p_base,
            env.neutral_baseline_b,
            env.tempo_cap,
            env.control_cap,
            env.force_cap
        );
    };

    let eval_params = TechniqueTriadEvalParams {
        technique: tech_id,
        domain,
        envelope: env.clone(),
        alloc: Triad::new(tempo, control, force),
        skill_level_n: skill_n.clamp(1, 10),
        time_factor_params: ActionTimeFactorParams {
            fatigue_level: fatigue_level.max(0),
            wound_shock_index: f64::from(wound_shock_index.max(0.0)),
            ..Default::default()
        },
        weapon_recoil_r: if domain == CombatDomain::Ranged { 4 } else { 0 },
        ..Default::default()
    };
    let res = evaluate_technique_with_triad(&eval_params);
    let spec = tech_id.spec();
    let eff_secs = f64::from(res.effective_tc) * 0.25;

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

    format!(
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
    )
}

/// Invoked when `#ttrpg-action-technique` changes in `#ttrpg-drawer`.
#[wasm_bindgen]
pub fn satya_on_drawer_technique_change() {
    let tech_raw = select_value("ttrpg-action-technique");
    if let Some(tid) = CombatTechniqueId::from_code_str(&tech_raw) {
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
    let is_ranged = CombatTechniqueId::from_code_str(&tech_raw)
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

/// Recalculates and updates `#ttrpg-triad-readout` when `T`, `C`, `F`, or any drawer stat changes.
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

    let html = format_drawer_triad_readout(
        &tech_raw,
        tempo,
        control,
        force,
        control_c,
        skill_n,
        fatigue,
        wound_shock,
    );
    set_html("ttrpg-triad-readout", &html);
}
