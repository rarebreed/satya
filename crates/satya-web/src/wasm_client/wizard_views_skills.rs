//! Step 5 (8-Tree Hierarchical Skill Builder & 5-Factor SP Cost Calculator),
//! Step 6 (Derived Telemetry & Lifepath Summary), and top-level Wizard Shell HTML formatters.

use std::collections::BTreeMap;

use crate::wasm_client::dom::escape_html;
use crate::wasm_client::wizard::CharacterWizardState;
use crate::wasm_client::wizard_views_lifepath::format_lifepath_step_html;
use crate::wasm_client::wizard_views_stats::{
    format_aptitudes_step_html, format_group_pool_section_html, format_height_mass_section_html,
    format_psyche_step_html,
};
use satya_engine::{
    CANONICAL_SKILL_TREES, CharacterCharacteristics, CharacteristicCode, CharacteristicRollMethod,
    PopulationTier, PrerequisiteMode, SkillTrainingMode, SkillTreeId, SkillType,
    compute_academic_study_controller, compute_skill_allocation_sp_ledger,
    evaluate_operational_heat, is_career_formal_skill, lookup_campaign_sqm,
    preview_next_skill_step_sp, resolve_skill_tree_default, skills_in_tree,
    validate_skill_advancement,
};

/// Formats clickable `btn-chip` elements for all trained skills and elevated talents (`N >= 2` for `Tal`, `N >= 1` for others).
pub(crate) fn format_trained_skill_chips_html(
    chars: &CharacterCharacteristics,
    skills: &BTreeMap<String, usize>,
) -> String {
    CANONICAL_SKILL_TREES
        .iter()
        .filter_map(|spec| {
            let n = skills.get(spec.id).copied().unwrap_or(0);
            if n == 0 || (spec.skill_type == SkillType::Tal && n == 1) {
                return None;
            }
            let ctrl = chars.compute_controller(spec.primary_char, spec.secondary_char);
            let escaped_name_js = spec.name.replace('\'', "\\'");
            Some(format!(
                r#"<button type="button" class="btn-chip" onclick="window.satyaLoadSkillPreset('{escaped_name_js}', {n}, {ctrl})">{name} (N={n}, C={ctrl})</button>"#,
                name = escape_html(spec.name),
            ))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Formats Step 5: 8-Tree Hierarchical Skill Builder, 5-Factor SP Cost Calculator & DAG Prerequisite Auditor.
pub(crate) fn format_skills_step_html(st: &CharacterWizardState) -> String {
    let baseline = st.compute_lifepath_baseline();
    let ledger = compute_skill_allocation_sp_ledger(
        &baseline,
        &st.lifepath,
        &st.characteristics,
        &st.aptitudes,
        &st.skills,
    );

    let tree_opts: String = std::iter::once((
        "ALL",
        "All 8 Hierarchical Skill Trees (89 Canonical Skills)",
    ))
    .chain(SkillTreeId::ALL.iter().map(|t| (t.short_code(), t.title())))
    .map(|(code, title)| {
        let sel = if st.skill_tree_filter.eq_ignore_ascii_case(code) {
            " selected"
        } else {
            ""
        };
        format!(
            r#"<option value="{code}"{sel}>{title}</option>"#,
            title = escape_html(title)
        )
    })
    .collect::<Vec<_>>()
    .join("");

    let view_opts: String = [
        ("ALL", "Show All Skills (Trained, Unlockable & Locked)"),
        ("TRAINED", "Show Trained Skills Only (N >= 1)"),
        ("UNLOCKABLE", "Show Skills Ready to Advance (+1 Valid Now)"),
    ]
    .iter()
    .map(|&(code, label)| {
        let sel = if st.skill_view_mode.eq_ignore_ascii_case(code) {
            " selected"
        } else {
            ""
        };
        format!(r#"<option value="{code}"{sel}>{label}</option>"#)
    })
    .collect::<Vec<_>>()
    .join("");

    let selected_tree_filter = SkillTreeId::from_code(&st.skill_tree_filter);

    let trees_html: String = SkillTreeId::ALL
        .iter()
        .filter(|&&t| selected_tree_filter.is_none_or(|sel| sel == t))
        .map(|&tree| {
            let nodes = skills_in_tree(tree);
            let cards: Vec<String> = nodes
                .into_iter()
                .filter_map(|spec| {
                    let cur_n = st.skills.get(spec.id).copied().unwrap_or(0);
                    let floor_n = baseline.baseline_skills.get(spec.id).copied().unwrap_or(0);
                    let gained_post_school = cur_n.saturating_sub(floor_n);
                    let next_n = cur_n + 1;
                    let next_validation = validate_skill_advancement(
                        spec.id,
                        next_n,
                        &st.skills,
                        &st.characteristics,
                        &st.aptitudes,
                        &baseline.granted_access_tags,
                        baseline.academic_tree_frozen,
                    );
                    let (step_cost_bd, sp_affordable) = preview_next_skill_step_sp(
                        spec.id,
                        next_n,
                        &st.lifepath,
                        &ledger,
                        &st.characteristics,
                        &st.aptitudes,
                    );
                    let within_term_cap =
                        gained_post_school < baseline.max_post_school_gain_per_skill && cur_n < 6;
                    let can_advance =
                        next_validation.is_valid && within_term_cap && sp_affordable;
                    let can_reduce = cur_n > floor_n;

                    if st.skill_view_mode.eq_ignore_ascii_case("TRAINED") && cur_n == 0 {
                        return None;
                    }
                    if st.skill_view_mode.eq_ignore_ascii_case("UNLOCKABLE")
                        && !(next_validation.is_valid && within_term_cap)
                    {
                        return None;
                    }

                    let ctrl_str = match spec.secondary_char {
                        Some(sec) => format!("{}/{}", spec.primary_char.as_str(), sec.as_str()),
                        None => spec.primary_char.as_str().to_string(),
                    };
                    let c_val = next_validation.effective_controller_c;
                    let base_tn = PopulationTier::base_tn(c_val);
                    let apt_code = spec.governing_aptitude();
                    let apt_val = st.aptitudes.get(apt_code);
                    let is_career_rec = is_career_formal_skill(spec.id, &st.lifepath);
                    let sk_sp_invested = ledger.per_skill_sp_spent.get(spec.id).copied().unwrap_or(0);

                    let access_str: String = spec
                        .allowed_access
                        .iter()
                        .map(|a| a.label())
                        .collect::<Vec<_>>()
                        .join(" / ");

                    let prereq_str = if spec.parent_prereqs.is_empty() {
                        "Root / Innate Foundation".to_string()
                    } else {
                        let joiner = match spec.parent_mode {
                            PrerequisiteMode::AllOf => " + ",
                            PrerequisiteMode::AnyOf => " OR ",
                        };
                        spec.parent_prereqs
                            .iter()
                            .map(|&(p_id, min_n)| {
                                let p_cur = st.skills.get(p_id).copied().unwrap_or(0);
                                format!("{p_id} >={min_n} (N={p_cur})")
                            })
                            .collect::<Vec<_>>()
                            .join(joiner)
                    };

                    let mode_short = match step_cost_bd.training_mode {
                        SkillTrainingMode::FormalTraining => "Formal [FOC+INS]",
                        SkillTrainingMode::Experience => "Experience [INS]",
                    };
                    let cost_breakdown_line = format!(
                        "Next N={next_n}: <strong>{final_sp} SP</strong> ({mode_short}) — Base {base_sp} × Diff {diff_m:.2}x (D{diff_r}) × Ctrl {ctrl_m:.2}x × Apt {apt_m:.2}x × Train {tr_m:.2}x",
                        final_sp = step_cost_bd.final_sp_cost,
                        base_sp = step_cost_bd.base_level_sp,
                        diff_m = step_cost_bd.difficulty_multiplier,
                        diff_r = step_cost_bd.difficulty_rating,
                        ctrl_m = step_cost_bd.controller_multiplier,
                        apt_m = step_cost_bd.aptitude_multiplier,
                        tr_m = step_cost_bd.training_mode_multiplier,
                    );

                    let status_note = if cur_n >= 1 {
                        let career_tag = if is_career_rec {
                            format!(" · [Career Curriculum · {sk_sp_invested} SP Invested]")
                        } else if sk_sp_invested > 0 {
                            format!(" · [Elective · {sk_sp_invested} SP Invested]")
                        } else {
                            String::new()
                        };
                        format!(
                            "Trained N={cur_n} (Floor N={floor_n}, Post-School +{gained_post_school}) · Roll {cur_n} Pair(s) vs Base TN {base_tn}{career_tag}"
                        )
                    } else {
                        let def_res = resolve_skill_tree_default(spec.id, &st.skills);
                        if def_res.can_attempt {
                            escape_html(&def_res.reason)
                        } else {
                            format!(
                                "[No Default / Gated at N=0] {}",
                                escape_html(&def_res.reason)
                            )
                        }
                    };

                    let gate_badge = if next_validation.is_valid {
                        if !within_term_cap {
                            format!(
                                r#"<span class="pill pill-iggy">Term Cap (+{} Max Reached)</span>"#,
                                baseline.max_post_school_gain_per_skill
                            )
                        } else if !sp_affordable {
                            format!(
                                r#"<span class="pill pill-iggy">Needs {} SP (Rem: {} SP)</span>"#,
                                step_cost_bd.final_sp_cost, ledger.total_sp_remaining
                            )
                        } else {
                            format!(
                                r#"<span class="pill pill-active">+1 → {} SP ({mode_short})</span>"#,
                                step_cost_bd.final_sp_cost
                            )
                        }
                    } else {
                        let first_reason = next_validation
                            .failures
                            .first()
                            .map_or("Locked", |f| f.detail.as_str());
                        format!(
                            r#"<span class="pill pill-danger" title="{tip}">Locked: {code} ({cost} SP)</span>"#,
                            tip = escape_html(first_reason),
                            code = next_validation
                                .failures
                                .first()
                                .map_or("LOCKED", |f| f.code),
                            cost = step_cost_bd.final_sp_cost,
                        )
                    };

                    let minus_disabled = if can_reduce { "" } else { " disabled" };
                    let plus_disabled = if can_advance { "" } else { " disabled" };
                    let roll_n = cur_n.max(1);
                    let escaped_name_js = spec.name.replace('\'', "\\'");

                    Some(format!(
                        r#"<div class="wiz-char-assign-card">
                            <div class="wiz-char-header">
                                <div>
                                    <span class="wiz-char-code">T{tier} · {stype} · Diff {diff}/5 ({diff_lbl})</span>
                                    <strong class="wiz-char-name">{name}</strong>
                                </div>
                                <span class="stat-value">N = {cur_n}</span>
                            </div>
                            <div class="stat-note">
                                <strong>{ctrl_str} = {c_val}</strong> (Base TN {base_tn}) · <strong>{apt_str} {apt_val:+}</strong> · [{access_str}]
                            </div>
                            <div class="stat-note">{cost_breakdown_line}</div>
                            <div class="stat-note"><strong>Prereqs:</strong> {prereq_str}</div>
                            <div class="stat-note">{status_note}</div>
                            <div class="wiz-skill-actions-row">
                                {gate_badge}
                                <div class="btn-row-compact">
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaWizardAdjustSkill('{sk_id}', -1)"
                                        {minus_disabled}
                                    >
                                        "−1"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-accent"
                                        onclick="window.satyaWizardAdjustSkill('{sk_id}', 1)"
                                        {plus_disabled}
                                    >
                                        "+1 ({next_sp} SP)"
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-sm btn-secondary"
                                        onclick="window.satyaLoadSkillPreset('{escaped_name_js}', {roll_n}, {c_val})"
                                    >
                                        "Load 2d20"
                                    </button>
                                </div>
                            </div>
                        </div>"#,
                        tier = spec.tier,
                        stype = spec.skill_type.as_str(),
                        diff = spec.difficulty,
                        diff_lbl = spec.difficulty_label(),
                        name = escape_html(spec.name),
                        apt_str = apt_code.as_str(),
                        sk_id = spec.id,
                        next_sp = step_cost_bd.final_sp_cost,
                    ))
                })
                .collect();

            if cards.is_empty() {
                String::new()
            } else {
                let apt = tree.primary_aptitude();
                let apt_v = st.aptitudes.get(apt);
                format!(
                    r#"<fieldset class="wiz-fieldset">
                        <legend class="wiz-legend">
                            <span>{title}</span>
                            <span class="pill pill-iggy">Primary {apt_code} {apt_v:+}</span>
                        </legend>
                        <div class="wiz-char-grid">
                            {cards_html}
                        </div>
                    </fieldset>"#,
                    title = escape_html(tree.title()),
                    apt_code = apt.as_str(),
                    cards_html = cards.join("")
                )
            }
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Step 5: 8-Tree Hierarchical Skill Builder &amp; 5-Factor SP Cost Calculator (FOC={foc}, INS={ins})</span>
                <span class="pill pill-active">Remaining: {rem_sp} SP · Formal {f_spent}/{f_bud} SP · Experience {e_spent}/{e_bud} SP</span>
            </legend>
            <div class="pane-toolbar">
                <div class="model-controls-row">
                    <label class="inline-label" for="wiz-skill-tree-filter">
                        <span>Skill Tree</span>
                        <select
                            id="wiz-skill-tree-filter"
                            onchange="window.satyaWizardSetSkillFilter(this.value, document.getElementById('wiz-skill-view-mode').value)"
                        >
                            {tree_opts}
                        </select>
                    </label>
                    <label class="inline-label" for="wiz-skill-view-mode">
                        <span>Filter</span>
                        <select
                            id="wiz-skill-view-mode"
                            onchange="window.satyaWizardSetSkillFilter(document.getElementById('wiz-skill-tree-filter').value, this.value)"
                        >
                            {view_opts}
                        </select>
                    </label>
                </div>
                <div class="btn-row-compact">
                    <button
                        type="button"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaWizardAutoAllocateSkills()"
                    >
                        "Auto-Allocate Remaining ({rem_sp} SP)"
                    </button>
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaWizardResetPostSchoolSkills()"
                    >
                        "Reset to Stage 1–4 School Baseline"
                    </button>
                </div>
            </div>
            <div class="subtitle">
                <strong>5-Factor SP Step Cost:</strong> <code>Base(N) = 10 × N</code> × <code>Difficulty (1–5: 0.80x–2.50x)</code> × <code>Controller C (±4%/pt)</code> × <code>Aptitude (-15%/+20%)</code> × <code>Training Mode (Formal [FOC+INS] 0.85x vs Experience [INS] 1.00x/1.25x)</code>. Compare <em>Swimming &amp; Diving</em> (Diff 1) vs <em>Piloting (Rotary-Wing / Helicopter &amp; VTOL)</em> (Diff 5). Max post-school gain per skill: <strong>+{max_gain}</strong>.
            </div>
        </fieldset>
        {trees_html}"#,
        foc = st.characteristics.foc,
        ins = st.characteristics.ins,
        rem_sp = ledger.total_sp_remaining,
        f_spent = ledger.formal_sp_spent,
        f_bud = ledger.formal_sp_budget,
        e_spent = ledger.experience_sp_spent,
        e_bud = ledger.experience_sp_budget,
        max_gain = baseline.max_post_school_gain_per_skill,
    )
}

/// Formats Step 6: Complete Derived Characteristics, Lifepath Economy & Trained Skills Review.
pub(crate) fn format_summary_step_html(st: &CharacterWizardState) -> String {
    let c = &st.characteristics;
    let a = &st.aptitudes;
    let derived = c.compute_derived(0);
    let baseline = st.compute_lifepath_baseline();
    let sqm_spec = lookup_campaign_sqm(baseline.campaign_sqm);
    let heat_spec = evaluate_operational_heat(baseline.operational_heat);

    let c_study_math = compute_academic_study_controller(c.dis, c.ana, a.apt_qnt);
    let c_study_lit = compute_academic_study_controller(c.dis, c.mem, a.apt_lng);
    let c_study_cmp = compute_academic_study_controller(c.dis, c.ana, a.apt_cmp);
    let c_study_bio = compute_academic_study_controller(c.dis, c.mem, a.apt_bio);

    let trained_chips = format_trained_skill_chips_html(c, &st.skills);

    let stat_boxes = [
        (
            "Striking Power (POW)".to_string(),
            format!("{:.1}", derived.pow_striking_power),
            format!("(FOR {} × SPD {}) / 10", c.for_stat, c.spd),
        ),
        (
            "Momentum Factor (MMF)".to_string(),
            format!("{:.1}", derived.mmf_momentum_factor),
            format!("({:.1} kg / 70) × SPD {}", c.mas_kg, c.spd),
        ),
        (
            "Melee Reach Index".to_string(),
            format!("{:+}", derived.melee_reach_index),
            format!(
                "HGT {:.2}m ({:+}) + KIN {}",
                c.hgt_m, derived.height_reach_edge, c.kin
            ),
        ),
        (
            "Quick Thinking (D_cog)".to_string(),
            derived.d_cog_quick_thinking.to_string(),
            format!(
                "floor((max(ANA {}, CRE {}) + FOC {})/2)",
                c.ana, c.cre, c.foc
            ),
        ),
        (
            "OODA Initiative".to_string(),
            derived.ooda_initiative.to_string(),
            "Start Tick = max(1, 15 - OODA)".to_string(),
        ),
        (
            "General Awareness (AWR)".to_string(),
            derived.awr_general_awareness.to_string(),
            format!(
                "floor((max(SIG {}, HEA {}) + FOC + ITU)/3)",
                c.sig, c.hea
            ),
        ),
        (
            "Fatigue Threshold (FT)".to_string(),
            derived.ft_fatigue_threshold.to_string(),
            format!("FIT {} + floor(DIS {} / 2)", c.fit, c.dis),
        ),
        (
            "Wound Shock Res (WSR)".to_string(),
            derived.wsr_wound_shock_resistance.to_string(),
            format!("HLT {} + floor(DIS {} / 2)", c.hlt, c.dis),
        ),
        (
            "Social Composure (CMP)".to_string(),
            derived.cmp_social_composure_threshold.to_string(),
            format!("floor((DIS {} + FOC {}) / 4)", c.dis, c.foc),
        ),
        (
            "Lifepath Study (C_study)".to_string(),
            format!("Math {c_study_math} / Lit {c_study_lit}"),
            format!(
                "CS {c_study_cmp} · Bio {c_study_bio} (Base {})",
                derived.base_c_study
            ),
        ),
        (
            format!("Campaign Upkeep (SQM {})", sqm_spec.sqm_tier),
            format!("${}/mo", sqm_spec.monthly_liquid_baseline_usd),
            escape_html(sqm_spec.tier_name),
        ),
        (
            format!("Operational Heat ({}/10)", heat_spec.heat_score),
            format!("+{} Δ_ext", heat_spec.public_check_delta_ext),
            escape_html(heat_spec.description),
        ),
    ];

    let stat_boxes_html: String = stat_boxes
        .iter()
        .map(|(lbl, val, note)| {
            format!(
                r#"<div class="char-stat-box">
                    <span class="stat-label">{lbl}</span>
                    <span class="stat-value">{val}</span>
                    <span class="stat-note">{note}</span>
                </div>"#
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Step 6: Derived Telemetry, Lifepath Dossier &amp; Apply for "{name}"</span>
                <span class="pill pill-active">Age {age} · OODA {ooda} · D_cog {dcog} · POW {pow:.1} · SQM {sqm_tier}</span>
            </legend>
            <div class="pane-toolbar">
                <span class="subtitle">Review all 10 Derived Characteristics, Campaign Socioeconomic Upkeep (`SQM`), and Trained 8-Tree Skills before applying to the active Character Sheet:</span>
                <div class="btn-row-compact">
                    <button
                        type="button"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaWizardApplyToSheet()"
                    >
                        "Apply Character to Active Sheet &amp; Task Roller"
                    </button>
                </div>
            </div>
            <div class="char-sheet-grid">
                {stat_boxes_html}
            </div>
            <div class="roster-bar">
                <span class="roster-title">Trained Skills &amp; Elevated Talents (Click to Load into 2d20 Task Roller):</span>
                <div class="preset-users">{trained_chips}</div>
            </div>
        </fieldset>"#,
        name = escape_html(&st.character_name),
        age = baseline.age_years,
        pow = derived.pow_striking_power,
        dcog = derived.d_cog_quick_thinking,
        ooda = derived.ooda_initiative,
        sqm_tier = sqm_spec.sqm_tier,
    )
}

/// Renders the complete inner HTML of `#wizard-stage-container` based on [`CharacterWizardState`].
pub fn format_character_wizard_html(st: &CharacterWizardState) -> String {
    let step = st.active_step.clamp(1, 6);
    let steps_meta = [
        (1u8, "1. Primary & Sensory"),
        (2u8, "2. Psyche"),
        (3u8, "3. Domain Aptitudes"),
        (4u8, "4. Lifepath (Stages 1–5)"),
        (5u8, "5. Skill Trees (8 Trees)"),
        (6u8, "6. Review & Apply"),
    ];
    let step_nav_items: String = steps_meta
        .iter()
        .map(|&(num, label)| {
            let is_cur = num == step;
            let cls = if is_cur {
                "wiz-step-btn active"
            } else if num < step {
                "wiz-step-btn completed"
            } else {
                "wiz-step-btn"
            };
            let aria_cur = if is_cur {
                r#" aria-current="step""#
            } else {
                ""
            };
            format!(
                r#"<li><button type="button" class="{cls}"{aria_cur} onclick="window.satyaWizardSetStep({num})">{label}</button></li>"#
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let m1_checked = if st.roll_method == CharacteristicRollMethod::Method1DropLowestD5 {
        " checked"
    } else {
        ""
    };
    let m2_checked = if st.roll_method == CharacteristicRollMethod::Method2PoolKeepHighest {
        " checked"
    } else {
        ""
    };

    let step_body_html = match step {
        1 => {
            let phys_html = format_group_pool_section_html(
                "physical",
                "Physical Characteristics (6 Scores)",
                "Speed (SPD), Force (FOR), Dexterity (DEX), Kinesthesia (KIN), Health (HLT), Fitness (FIT)",
                &CharacteristicCode::PHYSICAL_SCALED,
                st.physical_pool.as_ref(),
                &st.physical_slots,
                &st.characteristics,
                st.roll_method,
            );
            let hm_html = format_height_mass_section_html(st);
            let ment_html = format_group_pool_section_html(
                "mental",
                "Mental Characteristics (6 Scores)",
                "Memory (MEM), Analysis (ANA), Insight (INS), Creativity (CRE), Discipline (DIS), Focus (FOC)",
                &CharacteristicCode::MENTAL_ALL,
                st.mental_pool.as_ref(),
                &st.mental_slots,
                &st.characteristics,
                st.roll_method,
            );
            let sens_html = format_group_pool_section_html(
                "sensory",
                "Sensory Characteristics (6 Scores)",
                "Sight (SIG), Hearing (HEA), Tactile (TAC), Olfaction (OLF), Taste (TAS), Intuition (ITU)",
                &CharacteristicCode::SENSORY_ALL,
                st.sensory_pool.as_ref(),
                &st.sensory_slots,
                &st.characteristics,
                st.roll_method,
            );
            let soc_html = format_group_pool_section_html(
                "social",
                "Social Characteristics (3 Scores)",
                "Comeliness (COM), Presence (PRE), Eloquence (ELO)",
                &CharacteristicCode::SOCIAL_ALL,
                st.social_pool.as_ref(),
                &st.social_slots,
                &st.characteristics,
                st.roll_method,
            );

            format!(
                r#"<fieldset class="wiz-fieldset">
                    <legend class="wiz-legend">
                        <span>Characteristic Dice Generation Method (Range 4–18)</span>
                    </legend>
                    <div class="wiz-method-radios">
                        <label class="wiz-radio-card" for="wiz-method-1">
                            <input
                                id="wiz-method-1"
                                type="radio"
                                name="wiz_roll_method"
                                value="METHOD_1"
                                {m1_checked}
                                onchange="window.satyaWizardSetMethod('METHOD_1')"
                            />
                            <div>
                                <strong>Method 1: Roll 2d4 + 3d5 (Drop Lowest d5)</strong>
                                <div class="subtitle">Range 4–18 (skewed mean ~11.96). Roll 6 times for Physical, 6 for Mental, 6 for Sensory, and 3 for Social; assign in any order.</div>
                            </div>
                        </label>
                        <label class="wiz-radio-card" for="wiz-method-2">
                            <input
                                id="wiz-method-2"
                                type="radio"
                                name="wiz_roll_method"
                                value="METHOD_2"
                                {m2_checked}
                                onchange="window.satyaWizardSetMethod('METHOD_2')"
                            />
                            <div>
                                <strong>Method 2: Roll 2d4 + 2d5 Pool (Keep Highest)</strong>
                                <div class="subtitle">Roll 2d4 + 2d5 8 times (keep 6 highest) for Physical, Mental, and Sensory; roll 4 times (drop lowest, keep 3) for Social; assign in any order.</div>
                            </div>
                        </label>
                    </div>
                    <div class="btn-row">
                        <button
                            type="button"
                            class="btn btn-sm btn-accent"
                            onclick="window.satyaWizardRollAllGroups()"
                        >
                            "Roll All Characteristic Groups (Physical, Height/Mass, Mental, Sensory &amp; Social)"
                        </button>
                    </div>
                </fieldset>
                {phys_html}
                {hm_html}
                {ment_html}
                {sens_html}
                {soc_html}"#
            )
        }
        2 => format_psyche_step_html(st),
        3 => format_aptitudes_step_html(st),
        4 => format_lifepath_step_html(st),
        5 => format_skills_step_html(st),
        _ => format_summary_step_html(st),
    };

    let prev_btn = if step > 1 {
        format!(
            r#"<button type="button" class="btn btn-sm btn-secondary" enterkeyhint="previous" onclick="window.satyaWizardSetStep({})">◂ Previous Step</button>"#,
            step - 1
        )
    } else {
        String::new()
    };
    let next_btn = if step < 6 {
        format!(
            r#"<button type="button" class="btn btn-sm btn-accent" enterkeyhint="next" onclick="window.satyaWizardSetStep({})">Next Step ▸</button>"#,
            step + 1
        )
    } else {
        r#"<button type="button" class="btn btn-sm btn-accent" onclick="window.satyaWizardApplyToSheet()">Apply Character to Active Sheet</button>"#.to_string()
    };

    format!(
        r#"<div class="wiz-shell">
            <nav aria-label="Character Generation Wizard Progress" class="wiz-progress-nav">
                <ol class="wiz-step-list" role="list">
                    {step_nav_items}
                </ol>
            </nav>
            <div class="wiz-body">
                {step_body_html}
            </div>
            <div class="wiz-footer-nav">
                <div>{prev_btn}</div>
                <div>{next_btn}</div>
            </div>
        </div>"#
    )
}

/// Returns the initial SSR HTML for `#wizard-stage-container`.
pub fn default_character_wizard_html() -> String {
    format_character_wizard_html(&CharacterWizardState::default())
}
