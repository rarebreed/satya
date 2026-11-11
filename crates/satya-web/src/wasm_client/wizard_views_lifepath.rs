//! Step 4 (Chronological 5-Stage Lifepath) HTML formatter and shared `[Access]` gate pill helpers
//! for the Character Creation Wizard.

use crate::wasm_client::dom::escape_html;
use crate::wasm_client::wizard::CharacterWizardState;
use satya_engine::{
    CANONICAL_SKILL_TREES, CareerTermConfig, CareerTermTrack, EducationalAdvancementResult,
    HighSchoolTrack, LifepathBaselineResult, LifepathBuildConfig, RuralYouthArchetype, SkillType,
    SocioeconomicClass, Stage4Path, StreetGangArchetype, lookup_skill_node,
    resolve_socioeconomic_origin,
};

fn roll_2d20_pairs_with<F: FnMut(i32) -> i32>(rng: &mut F) -> [(i32, i32); 2] {
    [(rng(20), rng(20)), (rng(20), rng(20))]
}

/// Rolls one Lifepath stage (`1..=5`) in `lp` using `rng` and clears any manual override on that stage.
pub(crate) fn roll_lifepath_stage_in_config<F: FnMut(i32) -> i32>(
    lp: &mut LifepathBuildConfig,
    stage_num: u8,
    rng: &mut F,
) {
    match stage_num {
        1 => {
            lp.origin_roll_2d20 = rng(20) + rng(20);
            let orig = resolve_socioeconomic_origin(lp.origin_roll_2d20);
            if let Some(&first_tal) = orig.early_environment_bonus_talents.first() {
                lp.stage1_early_talent = first_tal.to_string();
            }
        }
        2 => {
            lp.stage2_lit_pairs = roll_2d20_pairs_with(rng);
            lp.stage2_lit_override = None;
            lp.stage2_math_pairs = roll_2d20_pairs_with(rng);
            lp.stage2_math_override = None;
        }
        3 => {
            lp.stage3_math_pairs = roll_2d20_pairs_with(rng);
            lp.stage3_math_override = None;
            lp.stage3_sci_pairs = roll_2d20_pairs_with(rng);
            lp.stage3_sci_override = None;
            lp.stage3_hum_pairs = roll_2d20_pairs_with(rng);
            lp.stage3_hum_override = None;
            lp.age15_crisis_roll_2d20 = rng(20) + rng(20);
            lp.age15_dropout_override = None;
            lp.stage4_path_override = None;
        }
        4 => {
            lp.stage4a_hs_pairs = roll_2d20_pairs_with(rng);
            lp.stage4a_hs_override = None;
        }
        5 => {
            for term in &mut lp.career_terms {
                term.term_roll_pairs = roll_2d20_pairs_with(rng);
                term.override_successes = None;
            }
        }
        _ => {}
    }
}

/// Applies a field or override edit onto `lp` for Step 4 Lifepath controls.
pub(crate) fn apply_lifepath_field_in_config<F: FnMut(i32) -> i32>(
    lp: &mut LifepathBuildConfig,
    field: &str,
    val: &str,
    rng: &mut F,
) {
    let parsed_ov = val.parse::<usize>().ok().map(|n| n.min(2));
    match field {
        "origin_class" => {
            if let Some(cls) = SocioeconomicClass::from_code_str(val) {
                lp.origin_roll_2d20 = cls.representative_roll_2d20();
                let orig = resolve_socioeconomic_origin(lp.origin_roll_2d20);
                if let Some(&first_tal) = orig.early_environment_bonus_talents.first() {
                    lp.stage1_early_talent = first_tal.to_string();
                }
            }
        }
        "stage1_talent" if lookup_skill_node(val).is_some() => {
            lp.stage1_early_talent = val.to_string();
        }
        "stage2_lit_override" => lp.stage2_lit_override = parsed_ov,
        "stage2_math_override" => lp.stage2_math_override = parsed_ov,
        "stage2_talent" if lookup_skill_node(val).is_some() => {
            lp.stage2_playground_talent = val.to_string();
        }
        "stage3_math_override" => lp.stage3_math_override = parsed_ov,
        "stage3_sci_override" => lp.stage3_sci_override = parsed_ov,
        "stage3_hum_override" => lp.stage3_hum_override = parsed_ov,
        "age15_path" => {
            let code = val.to_ascii_uppercase();
            lp.stage4_path_override = Stage4Path::from_code_str(&code);
            lp.age15_dropout_override = match code.as_str() {
                "STAY_4A" => Some(false),
                "DROPOUT_4B" | "RURAL_4C" => Some(true),
                _ => None,
            };
        }
        "stage4a_track" => {
            if let Some(trk) = HighSchoolTrack::from_code_str(val) {
                lp.stage4a_hs_track = trk;
            }
        }
        "stage4a_hs_override" => lp.stage4a_hs_override = parsed_ov,
        "stage4b_archetype" => {
            if let Some(arch) = StreetGangArchetype::from_code_str(val) {
                lp.stage4b_gang_archetype = arch;
            }
        }
        "stage4c_archetype" => {
            if let Some(arch) = RuralYouthArchetype::from_code_str(val) {
                lp.stage4c_rural_archetype = arch;
            }
        }
        "num_career_terms" => {
            let target_len = val.parse::<usize>().unwrap_or(1).min(4);
            while lp.career_terms.len() < target_len {
                lp.career_terms.push(CareerTermConfig {
                    track: CareerTermTrack::MilitaryCombatArmsSpecOps,
                    term_roll_pairs: roll_2d20_pairs_with(rng),
                    override_successes: None,
                    waiver_override: true,
                });
            }
            lp.career_terms.truncate(target_len);
        }
        "adult_ged_waiver" => {
            lp.adult_ged_waiver = val.eq_ignore_ascii_case("true") || val == "1";
        }
        _ => {}
    }
}

/// Applies a Stage 5 Career Term (`0..4`) track, override, or re-roll mutation onto `lp`.
pub(crate) fn apply_career_term_in_config<F: FnMut(i32) -> i32>(
    lp: &mut LifepathBuildConfig,
    term_idx: usize,
    field: &str,
    val: &str,
    rng: &mut F,
) {
    if let Some(term) = lp.career_terms.get_mut(term_idx) {
        match field {
            "track" => {
                if let Some(trk) = CareerTermTrack::from_code_str(val) {
                    term.track = trk;
                }
            }
            "override_successes" => {
                term.override_successes = val.parse::<usize>().ok().map(|n| n.min(2));
            }
            "roll" => {
                term.term_roll_pairs = roll_2d20_pairs_with(rng);
                term.override_successes = None;
            }
            _ => {}
        }
    }
}

/// Formats the unlocked `[Access: ...]` gate pills and optional `[Academic Tree FROZEN]` pill for `baseline`.
pub fn format_access_gates_pills_html(baseline: &LifepathBaselineResult) -> String {
    let access_badges: String = baseline
        .granted_access_tags
        .iter()
        .map(|t| {
            format!(
                r#"<span class="pill pill-active">[Access: {}]</span>"#,
                t.label()
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let freeze_badge = if baseline.academic_tree_frozen {
        r#" <span class="pill pill-danger">[Academic Tree FROZEN]</span>"#
    } else {
        ""
    };
    format!("{access_badges}{freeze_badge}")
}

fn format_override_select_options(cur: Option<usize>) -> String {
    let opts = [
        (None, "Use 2d20 Roll Result"),
        (Some(0), "Override: 0 Successes (Remedial / +0)"),
        (Some(1), "Override: 1 Success (Standard / +1)"),
        (Some(2), "Override: 2 Successes (Honors / Breakthrough)"),
    ];
    opts.iter()
        .map(|&(val, label)| {
            let sel = if cur == val { " selected" } else { "" };
            let v_str = match val {
                Some(n) => n.to_string(),
                None => String::new(),
            };
            format!(r#"<option value="{v_str}"{sel}>{label}</option>"#)
        })
        .collect::<Vec<_>>()
        .join("")
}

fn format_talent_options_html(selected_id: &str) -> String {
    CANONICAL_SKILL_TREES
        .iter()
        .filter(|node| node.skill_type == SkillType::Tal && node.id != "native_language_speech")
        .map(|node| {
            let sel = if node.id == selected_id {
                " selected"
            } else {
                ""
            };
            format!(
                r#"<option value="{id}"{sel}>{name} ({prim})</option>"#,
                id = node.id,
                name = escape_html(node.name),
                prim = node.primary_char.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn format_k12_subject_card_html(
    title: &str,
    select_id: &str,
    field_name: &str,
    eval: &EducationalAdvancementResult,
    cur_override: Option<usize>,
    show_c_study: bool,
) -> String {
    let opts = format_override_select_options(cur_override);
    let pair_caption = if show_c_study {
        format!(
            "Pairs [{}, {}] · C_study = {}",
            eval.pair_sums[0], eval.pair_sums[1], eval.c_study
        )
    } else {
        format!("Pairs [{}, {}]", eval.pair_sums[0], eval.pair_sums[1])
    };
    format!(
        r#"<div class="wiz-char-assign-card">
            <div class="wiz-char-header">
                <strong>{title}</strong>
                <span class="pill pill-active">{succ} Succ (TN {tn})</span>
            </div>
            <label for="{select_id}">
                <span>{pair_caption}</span>
                <select
                    id="{select_id}"
                    onchange="window.satyaWizardUpdateLifepathField('{field_name}', this.value)"
                >
                    {opts}
                </select>
            </label>
        </div>"#,
        succ = eval.successes,
        tn = eval.tn_edu,
    )
}

fn format_archetype_radio_card_html(
    ids: (&str, &str, &str),
    copy: (&str, &str, &str),
    is_checked: bool,
) -> String {
    let (radio_prefix, radio_name, field_name) = ids;
    let (code, label, subtitle) = copy;
    let checked = if is_checked { " checked" } else { "" };
    let radio_id = format!("{radio_prefix}-{}", code.to_ascii_lowercase());
    format!(
        r#"<label class="wiz-radio-card" for="{radio_id}">
            <input
                id="{radio_id}"
                type="radio"
                name="{radio_name}"
                value="{code}"
                {checked}
                onchange="window.satyaWizardUpdateLifepathField('{field_name}', '{code}')"
            />
            <div>
                <strong>{label}</strong>
                <div class="subtitle">{sub}</div>
            </div>
        </label>"#,
        label = escape_html(label),
        sub = escape_html(subtitle),
    )
}

/// Formats Step 4: Chronological 5-Stage Lifepath (`Stages 1–5`).
pub fn format_lifepath_step_html(st: &CharacterWizardState) -> String {
    let baseline = st.compute_lifepath_baseline();
    let cfg = &st.lifepath;
    let origin = &baseline.origin;

    let origin_class_opts: String = SocioeconomicClass::ALL
        .iter()
        .map(|&cls| {
            let spec = resolve_socioeconomic_origin(cls.representative_roll_2d20());
            let sel = if spec.class == origin.class {
                " selected"
            } else {
                ""
            };
            format!(
                r#"<option value="{code}"{sel}>{label} (SQM {sqm:+}, Dropout <= {drop})</option>"#,
                code = cls.as_str(),
                label = escape_html(spec.label),
                sqm = spec.school_quality_modifier_sqm,
                drop = spec.adolescent_dropout_risk_max_2d20,
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let stage1_talent_opts = format_talent_options_html(&cfg.stage1_early_talent);
    let stage2_talent_opts = format_talent_options_html(&cfg.stage2_playground_talent);

    let s2_lit_card = format_k12_subject_card_html(
        "Elementary Literacy (DIS/MEM + APT-LNG)",
        "wiz-s2-lit-ov",
        "stage2_lit_override",
        &baseline.stage2_lit_eval,
        cfg.stage2_lit_override,
        true,
    );
    let s2_math_card = format_k12_subject_card_html(
        "Elementary Mathematics (DIS/ANA + APT-QNT)",
        "wiz-s2-math-ov",
        "stage2_math_override",
        &baseline.stage2_math_eval,
        cfg.stage2_math_override,
        true,
    );
    let s3_math_card = format_k12_subject_card_html(
        "Junior High Math (Algebra / Trig Gate)",
        "wiz-s3-math-ov",
        "stage3_math_override",
        &baseline.stage3_math_eval,
        cfg.stage3_math_override,
        false,
    );
    let s3_sci_card = format_k12_subject_card_html(
        "Junior High Science (Biology &amp; Tech)",
        "wiz-s3-sci-ov",
        "stage3_sci_override",
        &baseline.stage3_sci_eval,
        cfg.stage3_sci_override,
        false,
    );
    let s3_hum_card = format_k12_subject_card_html(
        "Junior High Humanities (History &amp; Writing)",
        "wiz-s3-hum-ov",
        "stage3_hum_override",
        &baseline.stage3_hum_eval,
        cfg.stage3_hum_override,
        false,
    );

    let age15_auto_sel =
        if cfg.stage4_path_override.is_none() && cfg.age15_dropout_override.is_none() {
            " selected"
        } else {
            ""
        };
    let age15_stay_sel = if cfg.stage4_path_override == Some(Stage4Path::Path4AHighSchool)
        || (cfg.stage4_path_override.is_none() && cfg.age15_dropout_override == Some(false))
    {
        " selected"
    } else {
        ""
    };
    let age15_drop_sel = if cfg.stage4_path_override == Some(Stage4Path::Path4BStreetGang)
        || (cfg.stage4_path_override.is_none() && cfg.age15_dropout_override == Some(true))
    {
        " selected"
    } else {
        ""
    };
    let age15_rural_sel = if cfg.stage4_path_override == Some(Stage4Path::Path4CRuralAgrarian) {
        " selected"
    } else {
        ""
    };

    let stage4_html = match baseline.stage4_path {
        Stage4Path::Path4BStreetGang => {
            let gang_cards: String = StreetGangArchetype::ALL
                .iter()
                .map(|&arch| {
                    format_archetype_radio_card_html(
                        ("wiz-gang-arch", "wiz_stage4b_archetype", "stage4b_archetype"),
                        (arch.as_str(), arch.label(), arch.subtitle()),
                        cfg.stage4b_gang_archetype == arch,
                    )
                })
                .collect::<Vec<_>>()
                .join("");

            format!(
                r#"<fieldset class="wiz-fieldset">
                    <legend class="wiz-legend">
                        <span>Stage 4B (Ages 16–18): School Dropout &amp; Street Gang Archetype</span>
                        <span class="pill pill-danger">Academic Tree FROZEN · [Access: Illicit] Unlocked</span>
                    </legend>
                    <div class="subtitle">Dropping out at Age 15 freezes `[Access: Academic]` (blocking University Calculus/Engineering unless a GED waiver is taken in Stage 5) and unlocks `[Access: Illicit]` street gang training:</div>
                    <div class="wiz-method-radios">
                        {gang_cards}
                    </div>
                </fieldset>"#
            )
        }
        Stage4Path::Path4CRuralAgrarian => {
            let rural_cards: String = RuralYouthArchetype::ALL
                .iter()
                .map(|&arch| {
                    format_archetype_radio_card_html(
                        ("wiz-rural-arch", "wiz_stage4c_archetype", "stage4c_archetype"),
                        (arch.as_str(), arch.label(), arch.subtitle()),
                        cfg.stage4c_rural_archetype == arch,
                    )
                })
                .collect::<Vec<_>>()
                .join("");

            format!(
                r#"<fieldset class="wiz-fieldset">
                    <legend class="wiz-legend">
                        <span>Stage 4C (Ages 15–18): Rural, Agrarian &amp; Developing-Region Youth</span>
                        <span class="pill pill-active">Academic Tree FROZEN · [Access: Vocational/Licensed] Unlocked</span>
                    </legend>
                    <div class="subtitle">Leaving formal classroom schooling after Junior High to work family homesteads, herd livestock, hunt/trap in the bush, or serve in frontier militias freezes `[Access: Academic]` (unless an Adult GED waiver is taken in Stage 5) and grants `[Access: Vocational/Licensed]` &amp; rural fieldcraft skills:</div>
                    <div class="wiz-method-radios">
                        {rural_cards}
                    </div>
                </fieldset>"#
            )
        }
        Stage4Path::Path4AHighSchool => {
            let alg_n = baseline
                .baseline_skills
                .get("elementary_algebra_geometry")
                .copied()
                .unwrap_or(0);
            let stem_qual =
                alg_n >= 2 && st.characteristics.ana >= 11 && st.characteristics.dis >= 11;
            let stem_badge = if stem_qual {
                "Qualifies Natively"
            } else {
                "Waiver / Player Override"
            };
            let hs_cards: String = HighSchoolTrack::ALL
                .iter()
                .map(|&trk| {
                    let checked = if cfg.stage4a_hs_track == trk {
                        " checked"
                    } else {
                        ""
                    };
                    let code = trk.as_str();
                    let radio_id = format!("wiz-hs-track-{}", code.to_ascii_lowercase());
                    let qual_note = if trk == HighSchoolTrack::ApHonorsStemMagnet {
                        format!(" ({stem_badge})")
                    } else {
                        String::new()
                    };
                    format!(
                        r#"<label class="wiz-radio-card" for="{radio_id}">
                            <input
                                id="{radio_id}"
                                type="radio"
                                name="wiz_stage4a_track"
                                value="{code}"
                                {checked}
                                onchange="window.satyaWizardUpdateLifepathField('stage4a_track', '{code}')"
                            />
                            <div>
                                <strong>{label}{qual_note}</strong>
                                <div class="subtitle">{sub}</div>
                            </div>
                        </label>"#,
                        label = escape_html(trk.label()),
                        sub = escape_html(trk.subtitle()),
                    )
                })
                .collect::<Vec<_>>()
                .join("");

            let hs_eval = baseline.stage4a_hs_eval.as_ref();
            let hs_roll_str = hs_eval.map_or_else(
                || "N/A".to_string(),
                |e| {
                    format!(
                        "Pairs [{}, {}] vs TN {} → {} Success(es)",
                        e.pair_sums[0], e.pair_sums[1], e.tn_edu, e.successes
                    )
                },
            );
            let hs_override_opts = format_override_select_options(cfg.stage4a_hs_override);

            format!(
                r#"<fieldset class="wiz-fieldset">
                    <legend class="wiz-legend">
                        <span>Stage 4A (Ages 16–18): Formal High School Track</span>
                        <span class="pill pill-active">[Access: Academic] Active</span>
                    </legend>
                    <div class="pane-toolbar">
                        <span class="subtitle">Select High School curriculum track and roll 2d20 for High School honors advancement:</span>
                        <button
                            type="button"
                            class="btn btn-sm btn-secondary"
                            onclick="window.satyaWizardRollLifepathStage(4)"
                        >
                            "Roll Stage 4A High School (2d20)"
                        </button>
                    </div>
                    <div class="wiz-method-radios">
                        {hs_cards}
                    </div>
                    <div class="wiz-char-grid">
                        <div class="wiz-char-assign-card">
                            <div class="wiz-char-header">
                                <strong>High School Advancement Roll</strong>
                                <span class="pill pill-iggy">{hs_roll_str}</span>
                            </div>
                            <label for="wiz-s4a-hs-override">
                                <span>Roll Result or Player Override</span>
                                <select
                                    id="wiz-s4a-hs-override"
                                    onchange="window.satyaWizardUpdateLifepathField('stage4a_hs_override', this.value)"
                                >
                                    {hs_override_opts}
                                </select>
                            </label>
                        </div>
                    </div>
                </fieldset>"#
            )
        }
    };

    let num_terms = cfg.career_terms.len().min(4);
    let num_terms_opts: String = (0..=4)
        .map(|n| {
            let age = 18 + n * 4;
            let sel = if n == num_terms { " selected" } else { "" };
            format!(
                r#"<option value="{n}"{sel}>{n} Term(s) — Starting Age {age} ({min_sp}–{max_sp} Formal SP + {exp_sp} Experience SP)</option>"#,
                min_sp = n * 120,
                max_sp = n * 160,
                exp_sp = 40 + n * 40
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let ged_checked = if cfg.adult_ged_waiver { " checked" } else { "" };

    let career_term_cards: String = cfg
        .career_terms
        .iter()
        .take(4)
        .enumerate()
        .map(|(t_idx, term)| {
            let age_start = 18 + t_idx * 4;
            let age_end = age_start + 4;
            let (native_ok, qual_detail) = term.track.check_entry_qualification(
                &st.characteristics,
                &baseline.baseline_skills,
                baseline.academic_tree_frozen && !cfg.adult_ged_waiver,
            );
            let qual_pill = if native_ok {
                r#"<span class="pill pill-active">Meets Prerequisites</span>"#
            } else if term.waiver_override {
                r#"<span class="pill pill-iggy">Entry Waiver Active</span>"#
            } else {
                r#"<span class="pill pill-danger">Prerequisites Shortfall</span>"#
            };

            let track_opts: String = CareerTermTrack::ALL
                .iter()
                .map(|&trk| {
                    let sel = if trk == term.track { " selected" } else { "" };
                    format!(
                        r#"<option value="{code}"{sel}>{label}</option>"#,
                        code = trk.as_str(),
                        label = escape_html(trk.label())
                    )
                })
                .collect::<Vec<_>>()
                .join("");

            let t_eval = baseline.term_evals.get(t_idx);
            let t_roll_str = t_eval.map_or_else(
                || "N/A".to_string(),
                |e| {
                    format!(
                        "Pairs [{}, {}] vs TN {} → {} Succ (+{} Formal SP + 40 Exp SP)",
                        e.pair_sums[0],
                        e.pair_sums[1],
                        e.tn_edu,
                        e.successes,
                        120 + 20 * e.successes.min(2)
                    )
                },
            );
            let term_ov_opts = format_override_select_options(term.override_successes);
            let access_str: String = term
                .track
                .granted_access_tags()
                .iter()
                .map(|t| format!("[Access: {}]", t.label()))
                .collect::<Vec<_>>()
                .join(" · ");

            format!(
                r#"<div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <div>
                            <span class="wiz-char-code">TERM #{} (AGES {age_start}–{age_end})</span>
                            <strong class="wiz-char-name">{track_label}</strong>
                        </div>
                        {qual_pill}
                    </div>
                    <label for="wiz-career-track-{t_idx}">
                        <span>Select 4-Year Career / Institution Track</span>
                        <select
                            id="wiz-career-track-{t_idx}"
                            onchange="window.satyaWizardUpdateCareerTerm({t_idx}, 'track', this.value)"
                        >
                            {track_opts}
                        </select>
                    </label>
                    <label for="wiz-career-ov-{t_idx}">
                        <span>Term Advancement Roll ({t_roll_str})</span>
                        <select
                            id="wiz-career-ov-{t_idx}"
                            onchange="window.satyaWizardUpdateCareerTerm({t_idx}, 'override_successes', this.value)"
                        >
                            {term_ov_opts}
                        </select>
                    </label>
                    <div class="btn-row-compact">
                        <button
                            type="button"
                            class="btn btn-sm btn-secondary"
                            onclick="window.satyaWizardUpdateCareerTerm({t_idx}, 'roll', '1')"
                        >
                            "Re-Roll Term #{term_num} (2d20)"
                        </button>
                    </div>
                    <div class="stat-note"><strong>Unlocks:</strong> {access_str} · <strong>Prereqs:</strong> {qual_detail}</div>
                </div>"#,
                t_idx + 1,
                term_num = t_idx + 1,
                track_label = escape_html(term.track.label()),
                qual_detail = escape_html(&qual_detail),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let access_gates_html = format_access_gates_pills_html(&baseline);
    let history_items: String = baseline
        .stage_history
        .iter()
        .map(|entry| format!("<li>{}</li>", escape_html(entry)))
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Step 4: Chronological 5-Stage Lifepath (Ages 0–{age})</span>
                <span class="pill pill-active">Age {age} · Formal SP: {c_bud} (FOC+INS) · Experience SP: {e_bud} (INS)</span>
            </legend>
            <div class="pane-toolbar">
                <span class="subtitle">Roll or choose Socioeconomic Origin, Elementary &amp; Junior High schooling, Age-15 Dropout vs. High School, and 1–4 Adult Career Terms:</span>
                <div class="btn-row-compact">
                    <button
                        type="button"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaWizardRollCompleteLifepath()"
                    >
                        "Roll Complete Lifepath (Stages 1–5) &amp; Auto-Allocate Skills"
                    </button>
                </div>
            </div>
            <div class="wiz-pool-readout">
                <div class="wiz-kept-slots-row">
                    <span class="roster-title">Unlocked [Access] Gates:</span> {access_gates_html}
                </div>
                <ul class="wiz-lifepath-log">{history_items}</ul>
            </div>
        </fieldset>

        <fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Stage 1 (Ages 0–5): Socioeconomic Origin &amp; Childhood Baseline</span>
                <span class="pill pill-iggy">2d20 Roll = {orig_roll} · SQM {sqm:+} · Dropout Risk &lt;= {drop_max}</span>
            </legend>
            <div class="pane-toolbar">
                <span class="subtitle">Grants all 20 Universal Childhood Talents (`Native Language Speech` N=2, others N=1) plus +1 in an Early Environment Talent:</span>
                <button
                    type="button"
                    class="btn btn-sm btn-secondary"
                    onclick="window.satyaWizardRollLifepathStage(1)"
                >
                    "Roll 2d20 Origin"
                </button>
            </div>
            <div class="wiz-char-grid">
                <div class="wiz-char-assign-card">
                    <label for="wiz-s1-origin-class">
                        <span>Socioeconomic Class (Roll or Override)</span>
                        <select
                            id="wiz-s1-origin-class"
                            onchange="window.satyaWizardUpdateLifepathField('origin_class', this.value)"
                        >
                            {origin_class_opts}
                        </select>
                    </label>
                    <div class="stat-note">School Quality Modifier (`SQM {sqm:+}`) shifts all K–12 `TN_edu = 31 - C_study + SQM`.</div>
                </div>
                <div class="wiz-char-assign-card">
                    <label for="wiz-s1-early-talent">
                        <span>Early Environment Bonus Talent (Advances to N=2)</span>
                        <select
                            id="wiz-s1-early-talent"
                            onchange="window.satyaWizardUpdateLifepathField('stage1_talent', this.value)"
                        >
                            {stage1_talent_opts}
                        </select>
                    </label>
                    <div class="stat-note">Reflects early childhood neighborhood exposure (Ages 0–5).</div>
                </div>
            </div>
        </fieldset>

        <fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Stage 2 (Ages 6–11): Elementary School Curriculum &amp; Advancement</span>
                <span class="pill pill-iggy">Grants Reading, Writing, Arithmetic &amp; Digital Literacy N=1</span>
            </legend>
            <div class="pane-toolbar">
                <span class="subtitle">Roll 2 pairs of 2d20 against `TN_edu = 31 - C_study + SQM` (or override successes) plus choose a Recess Talent:</span>
                <button
                    type="button"
                    class="btn btn-sm btn-secondary"
                    onclick="window.satyaWizardRollLifepathStage(2)"
                >
                    "Roll Stage 2 Elementary (2d20)"
                </button>
            </div>
            <div class="wiz-char-grid">
                {s2_lit_card}
                {s2_math_card}
                <div class="wiz-char-assign-card">
                    <label for="wiz-s2-recess-talent">
                        <span>Playground / Recess Talent (+1 Level)</span>
                        <select
                            id="wiz-s2-recess-talent"
                            onchange="window.satyaWizardUpdateLifepathField('stage2_talent', this.value)"
                        >
                            {stage2_talent_opts}
                        </select>
                    </label>
                </div>
            </div>
        </fieldset>

        <fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Stage 3 (Ages 12–15): Junior High &amp; Age-15 Dropout Crisis Check</span>
                <span class="pill pill-iggy">Crisis Roll 2d20 = {crisis_roll} (Threshold &lt;= {drop_max})</span>
            </legend>
            <div class="pane-toolbar">
                <span class="subtitle">Advances Algebra/Trigonometry, General Science &amp; Humanities, then checks for Adolescent Dropout Crisis at Age 15:</span>
                <button
                    type="button"
                    class="btn btn-sm btn-secondary"
                    onclick="window.satyaWizardRollLifepathStage(3)"
                >
                    "Roll Stage 3 Junior High &amp; Crisis Check"
                </button>
            </div>
            <div class="wiz-char-grid">
                {s3_math_card}
                {s3_sci_card}
                {s3_hum_card}
                <div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <strong>Age-15 Path (High School, Street Gang, or Rural/Agrarian)</strong>
                        <span class="pill pill-iggy">2d20 = {crisis_roll}</span>
                    </div>
                    <label for="wiz-s3-age15-path">
                        <span>Roll Outcome or Player Override</span>
                        <select
                            id="wiz-s3-age15-path"
                            onchange="window.satyaWizardUpdateLifepathField('age15_path', this.value)"
                        >
                            <option value="AUTO"{age15_auto_sel}>Follow 2d20 Roll ({crisis_roll} vs &lt;={drop_max})</option>
                            <option value="STAY_4A"{age15_stay_sel}>Override: Stay in High School (Path 4A)</option>
                            <option value="DROPOUT_4B"{age15_drop_sel}>Override: Drop Out at Age 15 — Street Gang (Path 4B)</option>
                            <option value="RURAL_4C"{age15_rural_sel}>Override: Rural / Agrarian / Frontier Youth (Path 4C)</option>
                        </select>
                    </label>
                </div>
            </div>
        </fieldset>

        {stage4_html}

        <fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Stage 5 (Ages 18–34): Adult 4-Year Career Terms (1 to 4 Terms)</span>
                <span class="pill pill-active">{num_terms} Term(s) · +{c_bud} Formal SP (FOC+INS) · +{e_bud} Experience SP (INS)</span>
            </legend>
            <div class="pane-toolbar">
                <div class="model-controls-row">
                    <label class="inline-label" for="wiz-s5-num-terms">
                        <span>Career Terms</span>
                        <select
                            id="wiz-s5-num-terms"
                            onchange="window.satyaWizardUpdateLifepathField('num_career_terms', this.value)"
                        >
                            {num_terms_opts}
                        </select>
                    </label>
                    <label class="inline-label" for="wiz-s5-ged-waiver">
                        <input
                            id="wiz-s5-ged-waiver"
                            type="checkbox"
                            {ged_checked}
                            onchange="window.satyaWizardUpdateLifepathField('adult_ged_waiver', this.checked ? 'true' : 'false')"
                        />
                        <span>Adult GED / Unfreeze [Access: Academic]</span>
                    </label>
                </div>
                <div class="btn-row-compact">
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaWizardRollLifepathStage(5)"
                    >
                        "Roll All Career Term Checks (2d20)"
                    </button>
                    <button
                        type="button"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaWizardAutoAllocateSkills()"
                    >
                        "Auto-Fill Career &amp; Elective Skills"
                    </button>
                </div>
            </div>
            <div class="wiz-char-grid">
                {career_term_cards}
            </div>
        </fieldset>"#,
        age = baseline.age_years,
        c_bud = baseline.career_skill_budget,
        e_bud = baseline.elective_skill_budget,
        orig_roll = cfg.origin_roll_2d20.clamp(2, 40),
        sqm = origin.school_quality_modifier_sqm,
        drop_max = origin.adolescent_dropout_risk_max_2d20,
        crisis_roll = cfg.age15_crisis_roll_2d20.clamp(2, 40),
    )
}
