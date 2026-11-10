//! Pure HTML formatters for the Character Sheet (`#notebook-pane-character`):
//! - Top Operator Summary & Section Navigation
//! - All 30 Characteristics (Physical, Mental, Sensory, Social, Psyche), 9 Domain Aptitudes & 10 Derived Stats
//! - Human-Readable Innate Talents (`TAL`) Table & Trained 8-Tree Skills (`PHY`, `KNO`, `PHY/KNO`) Table
//! - Dedicated Equipment Sections: Ranged Firearms (with live Ballistics), Melee Weapons, Armor Profiles, and Field Gear / Encumbrance.

use crate::wasm_client::character_sheet::{
    CANONICAL_FIELD_GEAR_PRESETS, CANONICAL_FIREARM_PRESETS, CharacterSheetState,
    format_character_notes_and_lifepath_html, format_character_status_and_hit_locations_html,
    format_character_vault_and_assets_html, format_verification_badge_html,
};
use crate::wasm_client::dom::escape_html;
use crate::wasm_client::wizard_views_skills::format_trained_skill_chips_html;
use satya_engine::{
    ARMOR_CATALOG, AptitudeCode, CANONICAL_SKILL_TREES, CharacteristicCode, MELEE_WEAPON_CATALOG,
    PopulationTier, RANGED_FIRE_MODES, SkillTreeId, SkillType, compute_academic_study_controller,
    compute_aptitude_min_c_relief, compute_aptitude_training_time_multiplier,
    compute_stride_and_sprint_velocity, evaluate_granular_firearm_ballistics, psyche_deviation,
    psyche_pole_descriptor,
};

/// Returns the initial SSR HTML for `#char-sheet-stage-container`.
#[must_use]
pub fn default_character_sheet_html() -> String {
    format_character_sheet_html(&CharacterSheetState::default())
}

fn render_table_card(title: &str, headers: &[&str], rows_html: &str) -> String {
    let ths: String = headers.iter().map(|h| format!("<th>{h}</th>")).collect();
    format!(
        r#"<div class="char-table-card"><div class="char-table-title">{title}</div><div class="char-table-scroll"><table class="char-data-table"><thead><tr>{ths}</tr></thead><tbody>{rows_html}</tbody></table></div></div>"#
    )
}

fn format_char_table_rows(
    sheet: &CharacterSheetState,
    codes: &[CharacteristicCode],
    delta_int: i32,
) -> String {
    codes
        .iter()
        .map(|&code| {
            let val = sheet.characteristics.get_scale(code);
            let base_tn = PopulationTier::base_tn(val);
            let eff_tn = (base_tn - delta_int).clamp(2, 39);
            let tier_lbl = PopulationTier::from_controlling_scale(val).label();
            let escaped_lbl = code.label().replace('\'', "\\'");
            format!(
                r#"<tr><td><span class="wiz-char-code">{code_str}</span> <strong>{lbl}</strong></td><td class="mono"><strong>{val}</strong> / 20</td><td class="mono">TN {base_tn} <span class="stat-note">(Eff {eff_tn})</span></td><td><span class="stat-note">{tier_lbl}</span></td><td><button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaLoadSkillPreset('{escaped_lbl} Check', 1, {val})">Roll 1x2d20</button></td></tr>"#,
                code_str = code.as_str(),
                lbl = escape_html(code.label()),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Formats the complete 30-Characteristic, 9-Aptitude, and 10-Derived-Metric section.
#[must_use]
pub fn format_all_characteristics_and_aptitudes_html(sheet: &CharacterSheetState) -> String {
    let c = &sheet.characteristics;
    let a = &sheet.aptitudes;
    let derived = c.compute_derived(0);
    let env = sheet.compute_live_tactical_envelope();
    let delta_int = sheet.total_task_delta_int();
    let sprint = compute_stride_and_sprint_velocity(c.spd, c.hgt_m);

    let phys_rows = format_char_table_rows(sheet, &CharacteristicCode::PHYSICAL_SCALED, delta_int);
    let ment_rows = format_char_table_rows(sheet, &CharacteristicCode::MENTAL_ALL, delta_int);
    let sens_rows = format_char_table_rows(sheet, &CharacteristicCode::SENSORY_ALL, delta_int);
    let soc_rows = format_char_table_rows(sheet, &CharacteristicCode::SOCIAL_ALL, delta_int);

    let psyche_rows: String = CharacteristicCode::PSYCHE_ALL
        .iter()
        .map(|&code| {
            let val = c.get_scale(code);
            let pole_desc = psyche_pole_descriptor(code, val);
            let dev = psyche_deviation(val);
            let gm_tn = 21 - dev;
            let friction_note = if val <= 7 {
                format!("Low Pole (Δ_psyche={dev}, GM Friction TN {gm_tn})")
            } else if val >= 13 {
                format!("High Pole (Δ_psyche={dev}, GM Friction TN {gm_tn})")
            } else {
                format!("Balanced Equilibrium (Δ_psyche={dev})")
            };
            format!(
                r#"<tr><td><span class="wiz-char-code">{code_str}</span> <strong>{lbl}</strong></td><td class="mono"><strong>{val}</strong> / 20</td><td><strong>{pole_desc}</strong></td><td class="mono">{friction_note}</td></tr>"#,
                code_str = code.as_str(),
                lbl = escape_html(code.label()),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let aptitude_rows: String = AptitudeCode::ALL
        .iter()
        .map(|&apt| {
            let score = a.get(apt);
            let min_c_rel = compute_aptitude_min_c_relief(score);
            let train_mult = compute_aptitude_training_time_multiplier(score);
            let badge_cls = if score > 0 { "pill pill-active" } else if score < 0 { "pill pill-danger" } else { "pill pill-muted" };
            let relief = if min_c_rel > 0 { "-1 Min C Relief (APT >= +2)" } else { "Standard Min C" };
            format!(
                r#"<tr><td><span class="wiz-char-code">{code}</span> <strong>{lbl}</strong></td><td><span class="{badge_cls}">{score:+}</span></td><td class="mono">{train_mult:.2}x SP / Time</td><td class="mono">{relief}</td></tr>"#,
                code = apt.as_str(),
                lbl = escape_html(apt.label()),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let c_study_qnt = compute_academic_study_controller(c.dis, c.ana, a.apt_qnt);
    let c_study_lng = compute_academic_study_controller(c.dis, c.mem, a.apt_lng);
    let c_study_bio = compute_academic_study_controller(c.dis, c.mem, a.apt_bio);

    let derived_cards = [
        (
            "OODA Initiative",
            format!("{}", derived.ooda_initiative),
            format!(
                "Start Tick = max(1, 15 - {}) = Tick {}",
                derived.ooda_initiative,
                (15 - derived.ooda_initiative).max(1)
            ),
        ),
        (
            "Quick Thinking (D_cog)",
            format!("{}", derived.d_cog_quick_thinking),
            format!(
                "floor((max(ANA {}, CRE {}) + FOC {})/2)",
                c.ana, c.cre, c.foc
            ),
        ),
        (
            "Striking Power (POW)",
            format!("{:.1}", derived.pow_striking_power),
            format!("(FOR {} × SPD {}) / 10.0", c.for_stat, c.spd),
        ),
        (
            "Momentum Factor (MMF)",
            format!("{:.1}", derived.mmf_momentum_factor),
            format!("({:.1} kg / 70.0) × SPD {}", c.mas_kg, c.spd),
        ),
        (
            "Melee Reach Index",
            format!("{:+}", derived.melee_reach_index),
            format!(
                "HGT {:.2}m ({:+}) + KIN {} · Sprint {:.1} m/s",
                c.hgt_m, derived.height_reach_edge, c.kin, sprint.v_sprint_m_per_sec
            ),
        ),
        (
            "General Awareness (AWR)",
            format!("{}", derived.awr_general_awareness),
            format!(
                "floor((max(SIG {}, HEA {}) + FOC {} + ITU {})/3)",
                c.sig, c.hea, c.foc, c.itu
            ),
        ),
        (
            "Fatigue Threshold (FT)",
            format!("{}", derived.ft_fatigue_threshold),
            format!(
                "FIT {} + floor(DIS {}/2) · Exertion {}/{}",
                c.fit, c.dis, sheet.exertion_marks, derived.ft_fatigue_threshold
            ),
        ),
        (
            "Wound Shock Res (WSR)",
            format!("{}", derived.wsr_wound_shock_resistance),
            format!("HLT {} + floor(DIS {}/2)", c.hlt, c.dis),
        ),
        (
            "Social Composure (CMP)",
            format!("{}", derived.cmp_social_composure_threshold),
            format!(
                "floor((DIS {} + FOC {})/4) · Lost {}/{}",
                c.dis, c.foc, sheet.social_composure_lost, derived.cmp_social_composure_threshold
            ),
        ),
        (
            "Academic Study (C_study)",
            format!("QNT {c_study_qnt} · LNG {c_study_lng}"),
            format!("BIO {c_study_bio} · Base {}", derived.base_c_study),
        ),
        (
            "Tactical Pool (P_eff / P_base)",
            format!("{} / {}", env.p_eff, env.p_base),
            format!(
                "Neutral B={} · Time Factor {:.2}x",
                env.neutral_baseline_b, env.action_time_factor
            ),
        ),
        (
            "Triad Ceilings (T / C / F)",
            format!(
                "{} / {} / {}",
                env.tempo_cap, env.control_cap, env.force_cap
            ),
            format!("Morale: {} · Live Caps", env.morale_state.label()),
        ),
    ];

    let derived_html: String = derived_cards
        .iter()
        .map(|(lbl, val, note)| format!(r#"<div class="char-stat-box"><span class="stat-label">{lbl}</span><span class="stat-value">{val}</span><span class="stat-note">{note}</span></div>"#))
        .collect::<Vec<_>>()
        .join("");

    let hgt_scale = c.get_scale(CharacteristicCode::HGT);
    let mas_scale = c.get_scale(CharacteristicCode::MAS);
    let bmi = c.mas_kg / (c.hgt_m * c.hgt_m).max(1.0);
    let std_hdrs = &[
        "Characteristic",
        "Score",
        "Base / Eff TN",
        "Population Tier",
        "Roll",
    ];
    let cards_html = [
        render_table_card(
            &format!(
                "Physical Characteristics (6 Scaled + HGT {:.2}m &amp; MAS {:.1}kg)",
                c.hgt_m, c.mas_kg
            ),
            std_hdrs,
            &phys_rows,
        ),
        render_table_card(
            "Mental Characteristics (6 Cognitive &amp; Executive Scores)",
            std_hdrs,
            &ment_rows,
        ),
        render_table_card(
            "Sensory Characteristics (6 Perceptual Modalities)",
            std_hdrs,
            &sens_rows,
        ),
        render_table_card(
            "Social Characteristics (3 Interpersonal Scores)",
            std_hdrs,
            &soc_rows,
        ),
        render_table_card(
            "Psyche Profile (7 Bipolar Disposition Traits · Equilibrium = 10)",
            &[
                "Psyche Trait",
                "Score",
                "Active Pole Descriptor",
                "Deviation &amp; Friction TN",
            ],
            &psyche_rows,
        ),
        render_table_card(
            "9 Domain Aptitudes (APT-QNT .. APT-SOC · Range -2 to +3)",
            &[
                "Domain Aptitude",
                "Mod",
                "Training Multiplier",
                "Prerequisite Relief",
            ],
            &aptitude_rows,
        ),
    ]
    .join("");

    format!(
        r#"<section class="char-sheet-section" aria-label="All 30 Characteristics, 9 Domain Aptitudes &amp; 10 Derived Metrics">
            <div class="char-section-header">
                <h3>1. Complete 30-Characteristic Profile, 9 Domain Aptitudes &amp; 10 Derived Telemetry Metrics</h3>
                <span class="pill pill-iggy">HGT {hgt:.2} m (Scale {hgt_scale}) · MAS {mas:.1} kg (Scale {mas_scale} · BMI {bmi:.1})</span>
            </div>
            <div class="char-sheet-grid">{derived_html}</div>
            <div class="char-tables-two-col">{cards_html}</div>
        </section>"#,
        hgt = c.hgt_m,
        mas = c.mas_kg,
    )
}

/// Formats the human-readable **Innate Talents (`TAL`) Table** and **Trained Skills (`PHY`, `KNO`, `PHY/KNO`) Table**.
#[must_use]
pub fn format_talents_and_skills_tables_html(sheet: &CharacterSheetState) -> String {
    let c = &sheet.characteristics;
    let delta_int = sheet.total_task_delta_int();
    let selected_tree = SkillTreeId::from_code(&sheet.skill_tree_filter);

    let talent_rows: String = CANONICAL_SKILL_TREES
        .iter()
        .filter(|spec| spec.skill_type == SkillType::Tal)
        .filter(|spec| selected_tree.is_none_or(|t| spec.tree == t))
        .map(|spec| {
            let default_n = if spec.id == "native_language_speech" {
                2
            } else {
                1
            };
            let n = sheet.skills.get(spec.id).copied().unwrap_or(default_n).max(1);
            let ctrl = c.compute_controller(spec.primary_char, spec.secondary_char);
            let ctrl_formula = match spec.secondary_char {
                Some(sec) => format!("{}/{}", spec.primary_char.as_str(), sec.as_str()),
                None => spec.primary_char.as_str().to_string(),
            };
            let base_tn = PopulationTier::base_tn(ctrl);
            let eff_tn = (base_tn - delta_int).clamp(2, 39);
            let rank_badge = if n >= 2 {
                format!(r#"<span class="pill pill-active">N = {n} (Elevated)</span>"#)
            } else {
                r#"<span class="pill pill-muted">N = 1 (Innate)</span>"#.to_string()
            };
            let escaped_js = spec.name.replace('\'', "\\'");
            format!(
                r#"<tr>
                    <td><strong>{name}</strong></td>
                    <td><span class="stat-note">{tree_short}</span></td>
                    <td class="mono">{ctrl_formula} = <strong>{ctrl}</strong></td>
                    <td>{rank_badge}</td>
                    <td class="mono">TN {base_tn} <span class="stat-note">(Eff {eff_tn})</span></td>
                    <td>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaLoadSkillPreset('{escaped_js}', {n}, {ctrl})">Load 2d20</button>
                    </td>
                </tr>"#,
                name = escape_html(spec.name),
                tree_short = spec.tree.short_code(),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let mut trained_skill_count = 0usize;
    let skill_rows: String = CANONICAL_SKILL_TREES
        .iter()
        .filter(|spec| spec.skill_type != SkillType::Tal)
        .filter(|spec| selected_tree.is_none_or(|t| spec.tree == t))
        .filter_map(|spec| {
            let n = sheet.skills.get(spec.id).copied().unwrap_or(0);
            if n >= 1 {
                trained_skill_count += 1;
            }
            if sheet.skills_trained_only && n == 0 {
                return None;
            }
            let ctrl = c.compute_controller(spec.primary_char, spec.secondary_char);
            let ctrl_formula = match spec.secondary_char {
                Some(sec) => format!("{}/{}", spec.primary_char.as_str(), sec.as_str()),
                None => spec.primary_char.as_str().to_string(),
            };
            let base_tn = PopulationTier::base_tn(ctrl);
            let eff_tn = (base_tn - delta_int).clamp(2, 39);
            let access_str: String = spec
                .allowed_access
                .iter()
                .map(|a| a.label())
                .collect::<Vec<_>>()
                .join("/");
            let n_badge = if n >= 1 {
                format!(r#"<span class="pill pill-active">N = {n} ({n}d20 Pairs)</span>"#)
            } else {
                r#"<span class="pill pill-muted">N = 0 (Untrained)</span>"#.to_string()
            };
            let roll_n = n.max(1);
            let escaped_js = spec.name.replace('\'', "\\'");
            Some(format!(
                r#"<tr>
                    <td>
                        <strong>{name}</strong>
                        <div class="stat-note">[{access_str}] · {cats}</div>
                    </td>
                    <td>
                        <span class="wiz-char-code">{stype}</span>
                        <span class="stat-note">{tree_code} · T{tier} · D{diff}</span>
                    </td>
                    <td class="mono">{ctrl_formula} = <strong>{ctrl}</strong></td>
                    <td>{n_badge}</td>
                    <td class="mono">TN {base_tn} <span class="stat-note">(Eff {eff_tn})</span></td>
                    <td>
                        <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaLoadSkillPreset('{escaped_js}', {roll_n}, {ctrl})">Load 2d20</button>
                    </td>
                </tr>"#,
                name = escape_html(spec.name),
                stype = spec.skill_type.as_str(),
                tree_code = spec.tree.short_code(),
                tier = spec.tier,
                diff = spec.difficulty,
                cats = spec
                    .categories
                    .iter()
                    .map(|cat| cat.label())
                    .collect::<Vec<_>>()
                    .join(", "),
            ))
        })
        .collect::<Vec<_>>()
        .join("");

    let skill_rows_final = if skill_rows.is_empty() {
        r#"<tr><td colspan="6" class="stat-note">No trained skills match the selected tree filter. Switch to "Show All 73 Skills" or use the Character Wizard tab to allocate skills.</td></tr>"#.to_string()
    } else {
        skill_rows
    };

    let tree_opts: String = std::iter::once(("ALL", "All 8 Skill Trees"))
        .chain(SkillTreeId::ALL.iter().map(|t| (t.short_code(), t.title())))
        .map(|(code, title)| {
            let sel = if sheet.skill_tree_filter.eq_ignore_ascii_case(code) {
                " selected"
            } else {
                ""
            };
            format!(
                r#"<option value="{code}"{sel}>{}</option>"#,
                escape_html(title)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let trained_sel = if sheet.skills_trained_only {
        " selected"
    } else {
        ""
    };
    let all_sel = if !sheet.skills_trained_only {
        " selected"
    } else {
        ""
    };
    let chips_html = format_trained_skill_chips_html(c, &sheet.skills);

    format!(
        r#"<section class="char-sheet-section" aria-label="Innate Talents and Trained 8-Tree Skills Tables">
            <div class="char-section-header">
                <div>
                    <h3>2. Innate Talents (`TAL`) &amp; Trained 8-Tree Skills (`PHY`, `KNO`, `PHY/KNO`)</h3>
                    <span class="subtitle">Click any row's <strong>Load 2d20</strong> button to populate the Center-Bottom Two-List Cascading Die Roller.</span>
                </div>
                <div class="model-controls-row">
                    <label class="inline-label" for="char-sheet-tree-filter">
                        <span>Tree Filter</span>
                        <select
                            id="char-sheet-tree-filter"
                            onchange="window.satyaCharSheetSetSkillFilter(this.value, document.getElementById('char-sheet-skill-mode').value === 'TRAINED')"
                        >
                            {tree_opts}
                        </select>
                    </label>
                    <label class="inline-label" for="char-sheet-skill-mode">
                        <span>Display</span>
                        <select
                            id="char-sheet-skill-mode"
                            onchange="window.satyaCharSheetSetSkillFilter(document.getElementById('char-sheet-tree-filter').value, this.value === 'TRAINED')"
                        >
                            <option value="TRAINED"{trained_sel}>Trained Skills Only (N &gt;= 1)</option>
                            <option value="ALL"{all_sel}>All 73 Skills (Include N = 0)</option>
                        </select>
                    </label>
                </div>
            </div>

            <div class="char-table-card">
                <div class="char-table-title">
                    <span>Trained Skills Table ({trained_skill_count} Trained Nodes)</span>
                    <span class="pill pill-active">Δ_int {delta_int:+} Applied to Eff TN</span>
                </div>
                <div class="char-table-scroll">
                    <table id="char-sheet-skills-table" class="char-data-table">
                        <thead>
                            <tr>
                                <th>Skill Name &amp; [Access]</th>
                                <th>Type / Tree / Tier</th>
                                <th>Controller (C)</th>
                                <th>Level (N)</th>
                                <th>Target Number (TN)</th>
                                <th>Action</th>
                            </tr>
                        </thead>
                        <tbody>{skill_rows_final}</tbody>
                    </table>
                </div>
            </div>

            <div class="char-table-card">
                <div class="char-table-title">
                    <span>Universal Innate Talents (`TAL` · 16 Canonical Childhood Foundations)</span>
                    <span class="pill pill-iggy">Default N=1 (Native Speech N=2)</span>
                </div>
                <div class="char-table-scroll">
                    <table id="char-sheet-talents-table" class="char-data-table">
                        <thead>
                            <tr>
                                <th>Innate Talent</th>
                                <th>Tree</th>
                                <th>Controller (C)</th>
                                <th>Rank (N)</th>
                                <th>Target Number (TN)</th>
                                <th>Action</th>
                            </tr>
                        </thead>
                        <tbody>{talent_rows}</tbody>
                    </table>
                </div>
            </div>

            <div class="roster-bar">
                <span class="roster-title">Quick-Roll Trained Skill Chips:</span>
                <div id="char-sheet-skills-chips" class="preset-users">{chips_html}</div>
            </div>
        </section>"#
    )
}

/// Formats the Equipment Loadout section: Ranged Firearms, Melee Weapons, Armor Profiles, and Field Gear / Encumbrance.
#[must_use]
pub fn format_equipment_weapons_and_armor_html(sheet: &CharacterSheetState) -> String {
    let c = &sheet.characteristics;
    let derived = c.compute_derived(0);
    let enc = sheet.compute_encumbrance();
    let dist_m = sheet.ballistic_range_m;

    let ranged_rows: String = sheet
        .equipped_ranged_ids
        .iter()
        .filter_map(|r_id| {
            let &(pid, ref spec) = CANONICAL_FIREARM_PRESETS
                .iter()
                .find(|(id, _)| *id == r_id.as_str())?;
            let bal = evaluate_granular_firearm_ballistics(spec, dist_m);
            Some(format!(
                r#"<tr>
                    <td>
                        <strong>{name}</strong>
                        <div class="stat-note">{ammo} · {oal:.0}cm OAL ({bbl:.1}cm bbl) · ${cost}</div>
                    </td>
                    <td class="mono">{cap} rds</td>
                    <td class="mono"><strong>{e_imp:.0} J</strong> <span class="stat-note">(Muz {e_muz:.0}J)</span></td>
                    <td class="mono"><strong>{acc:+}</strong> <span class="stat-note">(Yaw {yaw})</span></td>
                    <td class="mono"><strong>R = {rec}</strong></td>
                    <td class="mono"><strong>Imp {imp}</strong> / <strong>Pen {pen}</strong></td>
                    <td class="mono">Rel {rel:+} <span class="stat-note">({jam}t/{strip}t)</span></td>
                    <td class="mono">{wt:.2} kg</td>
                    <td>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetModifyEquipment('remove_ranged', '{pid}')">Remove</button>
                    </td>
                </tr>"#,
                name = escape_html(spec.name),
                ammo = escape_html(spec.ammo.name),
                oal = spec.overall_length_cm,
                bbl = spec.barrel_length_cm,
                cost = spec.cost_usd,
                cap = spec.capacity_rounds,
                e_imp = bal.impact_energy_joules,
                e_muz = bal.muzzle_energy_joules,
                acc = bal.accuracy_acc,
                yaw = bal.wobble_yaw_factor,
                rec = bal.base_recoil_r,
                imp = bal.raw_ballistic_impact,
                pen = bal.base_armor_pen,
                rel = bal.reliability_rel,
                jam = bal.clear_jam_minor_ticks,
                strip = bal.strip_time_ticks,
                wt = spec.loaded_weight_kg,
            ))
        })
        .collect::<Vec<_>>()
        .join("");

    let ranged_options: String = CANONICAL_FIREARM_PRESETS
        .iter()
        .map(|(pid, spec)| {
            format!(
                r#"<option value="{pid}">{} ({})</option>"#,
                escape_html(spec.name),
                escape_html(spec.ammo.name)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let fire_modes_chips: String = RANGED_FIRE_MODES
        .iter()
        .map(|fm| {
            format!(
                r#"<span class="pill pill-muted" title="Rounds: {}-{}">{} (TC {}-{}, ΔR +{}-{})</span>"#,
                fm.min_rounds_expended,
                fm.max_rounds_expended,
                escape_html(fm.name),
                fm.min_base_tc,
                fm.max_base_tc,
                fm.min_mode_recoil_adder,
                fm.max_mode_recoil_adder,
            )
        })
        .collect::<Vec<_>>()
        .join(" ");

    let melee_rows: String = sheet
        .equipped_melee
        .iter()
        .map(|&mw_id| {
            let spec = mw_id.spec();
            let idx = mw_id as usize;
            let total_reach = derived.height_reach_edge
                + (c.kin - 10).div_euclid(4)
                + spec.max_reach;
            let base_impact =
                derived.pow_striking_power * spec.pow_multiplier + f64::from(spec.flat_trauma_or_bleed_bonus);
            let keep_out = if spec.has_keep_out_property {
                "Keep-Out (+4 Δ_ext)"
            } else {
                "Standard"
            };
            format!(
                r#"<tr>
                    <td><strong>{name}</strong></td>
                    <td class="mono">Reach {min_r}–{max_r} <span class="stat-note">(Total {total_reach:+})</span></td>
                    <td class="mono">{tc} Ticks</td>
                    <td class="mono">{guard:+} Guard</td>
                    <td class="mono"><strong>{base_impact:.1}</strong> <span class="stat-note">({mult:.2}x POW + {flat})</span></td>
                    <td><span class="stat-note">{keep_out} · +{blk} Sweep</span></td>
                    <td>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetModifyEquipment('remove_melee', '{idx}')">Remove</button>
                    </td>
                </tr>"#,
                name = escape_html(spec.name),
                min_r = spec.min_reach,
                max_r = spec.max_reach,
                tc = spec.base_tc,
                guard = spec.guard_bonus,
                mult = spec.pow_multiplier,
                flat = spec.flat_trauma_or_bleed_bonus,
                blk = spec.block_sweep_force_bonus,
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let melee_options: String = MELEE_WEAPON_CATALOG
        .iter()
        .enumerate()
        .map(|(idx, spec)| {
            format!(
                r#"<option value="{idx}">{} (Reach {}-{}, TC {})</option>"#,
                escape_html(spec.name),
                spec.min_reach,
                spec.max_reach,
                spec.base_tc
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let armor_rows: String = sheet
        .equipped_armor
        .iter()
        .map(|&ar_id| {
            let spec = ar_id.spec();
            let idx = ar_id as usize;
            let exo_note = if spec.exo_load_offset_kg > 0.0 {
                format!("Powered Exo (-{:.1} kg offset)", spec.exo_load_offset_kg)
            } else if spec.heat_retention_delta > 0 {
                format!("Heat Retention +{}", spec.heat_retention_delta)
            } else {
                "Passive Armor".to_string()
            };
            format!(
                r#"<tr>
                    <td>
                        <strong>{name}</strong>
                        <div class="stat-note">${cost} · {exo_note}</div>
                    </td>
                    <td>{zones}</td>
                    <td class="mono"><strong>PR {bal}</strong></td>
                    <td class="mono">Edged {edg} / Ice {ice} / Visor {vis}</td>
                    <td class="mono"><strong>BTD {btd}</strong></td>
                    <td class="mono">{wt:.1} kg</td>
                    <td>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetModifyEquipment('remove_armor', '{idx}')">Remove</button>
                    </td>
                </tr>"#,
                name = escape_html(spec.name),
                cost = spec.cost_usd,
                zones = escape_html(spec.encumbered_zones),
                bal = spec.pr_ballistic,
                edg = spec.pr_edged,
                ice = spec.pr_icepick_flechette,
                vis = spec.pr_visor_weak_spot,
                btd = spec.blunt_trauma_dampening_btd,
                wt = spec.weight_kg,
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let armor_options: String = ARMOR_CATALOG
        .iter()
        .enumerate()
        .map(|(idx, spec)| {
            format!(
                r#"<option value="{idx}">{} (Ballistic PR {}, BTD {}, {:.1}kg)</option>"#,
                escape_html(spec.name),
                spec.pr_ballistic,
                spec.blunt_trauma_dampening_btd,
                spec.weight_kg
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let gear_rows: String = sheet
        .field_gear
        .iter()
        .map(|g| {
            format!(
                r#"<tr>
                    <td><strong>{name}</strong></td>
                    <td><span class="wiz-char-code">{cat}</span></td>
                    <td class="mono">{mod_val:+} Δ_tool</td>
                    <td class="mono">{wt:.1} kg <span class="stat-note">({erg:.1}x)</span></td>
                    <td><span class="stat-note">{notes}</span></td>
                    <td>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetModifyEquipment('remove_gear', '{id}')">Remove</button>
                    </td>
                </tr>"#,
                name = escape_html(&g.name),
                cat = escape_html(&g.category),
                mod_val = g.tool_mod,
                wt = g.mass_kg,
                erg = g.carry_method.ergonomic_factor(),
                notes = escape_html(&g.notes),
                id = escape_html(&g.id),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let gear_options: String = CANONICAL_FIELD_GEAR_PRESETS
        .iter()
        .map(|&(pid, name, cat, mass_kg, _, tool_mod, _)| {
            format!(
                r#"<option value="{pid}">[{cat}] {} ({:.1}kg, Δ_tool {:+})</option>"#,
                escape_html(name),
                mass_kg,
                tool_mod
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let range_options: String = [
        (15.0, "15m (CQB / Indoor)"),
        (25.0, "25m (Standard Engagement)"),
        (50.0, "50m (Medium Urban)"),
        (150.0, "150m (Rifle Standoff)"),
        (400.0, "400m (Long-Range Overwatch)"),
    ]
    .iter()
    .map(|&(r, lbl)| {
        let sel = if (dist_m - r).abs() < 1.0 {
            " selected"
        } else {
            ""
        };
        format!(r#"<option value="{r}"{sel}>{lbl}</option>"#)
    })
    .collect::<Vec<_>>()
    .join("");

    format!(
        r#"<section class="char-sheet-section" aria-label="Equipment, Ranged Weapons, Melee Weapons, Armor &amp; Encumbrance">
            <div class="char-section-header">
                <div>
                    <h3>3. Operator Equipment Loadout: Weapons, Armor &amp; Field Gear</h3>
                    <span class="subtitle">Physics-derived firearm ballistics (`E_k`, `Acc`, `Recoil R`, `Pen`, `Rel`), Melee Reach/POW scaling, Armor `PR`/`BABT`, and Ergonomic Encumbrance.</span>
                </div>
                <span class="pill pill-active">
                    Load Tier {tier_num} ({tier_lbl}) · Raw {raw_kg:.1} kg / Eff {eff_kg:.1} kg (Comfort {base_kg:.1} kg · Max Drag {drag_kg:.0} kg)
                </span>
            </div>

            <div class="char-table-card">
                <div class="char-table-title">
                    <span>Ranged Weapons &amp; Granular Ballistics Table</span>
                    <div class="model-controls-row">
                        <label class="inline-label" for="char-ballistic-range-select">
                            <span>Target Range</span>
                            <select
                                id="char-ballistic-range-select"
                                onchange="window.satyaCharSheetModifyEquipment('set_range', this.value)"
                            >
                                {range_options}
                            </select>
                        </label>
                        <select id="char-add-ranged-select" aria-label="Add Ranged Firearm">{ranged_options}</select>
                        <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetModifyEquipment('add_ranged', '')">+ Equip Firearm</button>
                    </div>
                </div>
                <div class="char-table-scroll">
                    <table id="char-sheet-ranged-table" class="char-data-table">
                        <thead>
                            <tr>
                                <th>Firearm &amp; Cartridge</th>
                                <th>Mag</th>
                                <th>Energy @ {dist_m:.0}m</th>
                                <th>Acc / Yaw</th>
                                <th>Recoil (R)</th>
                                <th>Impact / Pen</th>
                                <th>Rel (Jam/Strip)</th>
                                <th>Mass</th>
                                <th>Action</th>
                            </tr>
                        </thead>
                        <tbody>{ranged_rows}</tbody>
                    </table>
                </div>
                <div class="roster-bar">
                    <span class="roster-title">Canonical Ranged Fire Modes (Braced when Force F &gt;= Recoil R):</span>
                    <div class="roster-chips">{fire_modes_chips}</div>
                </div>
            </div>

            <div class="char-tables-two-col">
                <div class="char-table-card">
                    <div class="char-table-title">
                        <span>Melee &amp; Unarmed Weapons</span>
                        <div class="btn-row-compact">
                            <select id="char-add-melee-select" aria-label="Add Melee Weapon">{melee_options}</select>
                            <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetModifyEquipment('add_melee', '')">+ Add Melee</button>
                        </div>
                    </div>
                    <div class="char-table-scroll">
                        <table id="char-sheet-melee-table" class="char-data-table">
                            <thead>
                                <tr>
                                    <th>Melee Weapon</th>
                                    <th>Reach</th>
                                    <th>Base TC</th>
                                    <th>Guard</th>
                                    <th>Strike Impact</th>
                                    <th>Properties</th>
                                    <th>Action</th>
                                </tr>
                            </thead>
                            <tbody>{melee_rows}</tbody>
                        </table>
                    </div>
                </div>

                <div class="char-table-card">
                    <div class="char-table-title">
                        <span>Equipped Armor &amp; Protection (PR vs. BABT)</span>
                        <div class="btn-row-compact">
                            <select id="char-add-armor-select" aria-label="Add Armor">{armor_options}</select>
                            <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetModifyEquipment('add_armor', '')">+ Equip Armor</button>
                        </div>
                    </div>
                    <div class="char-table-scroll">
                        <table id="char-sheet-armor-table" class="char-data-table">
                            <thead>
                                <tr>
                                    <th>Armor Profile</th>
                                    <th>Protected Zones</th>
                                    <th>Ballistic PR</th>
                                    <th>Edged / Ice / Visor</th>
                                    <th>Blunt BTD</th>
                                    <th>Mass</th>
                                    <th>Action</th>
                                </tr>
                            </thead>
                            <tbody>{armor_rows}</tbody>
                        </table>
                    </div>
                </div>
            </div>

            <div class="char-table-card">
                <div class="char-table-title">
                    <span>Field Equipment, Medical/Technical Kits &amp; Load Distribution</span>
                    <div class="btn-row-compact">
                        <select id="char-add-gear-select" aria-label="Add Field Gear">{gear_options}</select>
                        <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetModifyEquipment('add_gear', '')">+ Add Gear</button>
                    </div>
                </div>
                <div class="char-table-scroll">
                    <table id="char-sheet-gear-table" class="char-data-table">
                        <thead>
                            <tr>
                                <th>Equipment / Kit</th>
                                <th>Category</th>
                                <th>Tool Mod</th>
                                <th>Mass (Ergonomic)</th>
                                <th>Operational Notes</th>
                                <th>Action</th>
                            </tr>
                        </thead>
                        <tbody>{gear_rows}</tbody>
                    </table>
                </div>
            </div>
        </section>"#,
        tier_num = enc.tier.tier_number(),
        tier_lbl = match enc.tier.tier_number() {
            0 => "Unencumbered",
            1 => "Light / Patrol Load",
            2 => "Tactical / Fighting Load",
            3 => "Heavy / March Load",
            _ => "Overloaded / Staggering",
        },
        raw_kg = enc.raw_carried_mass_kg,
        eff_kg = enc.effective_load_kg,
        base_kg = enc.base_comfort_load_kg,
        drag_kg = enc.max_deadlift_drag_limit_kg,
    )
}

/// Renders the complete inner HTML of `#char-sheet-stage-container` based on [`CharacterSheetState`].
#[must_use]
pub fn format_character_sheet_html(sheet: &CharacterSheetState) -> String {
    let c = &sheet.characteristics;
    let derived = c.compute_derived(0);
    let control_dex_tac =
        c.compute_controller(CharacteristicCode::DEX, Some(CharacteristicCode::TAC));
    let max_ballistic_pr = sheet
        .equipped_armor
        .iter()
        .map(|a| a.spec().pr_ballistic)
        .max()
        .unwrap_or(0);
    let max_btd = sheet
        .equipped_armor
        .iter()
        .map(|a| a.spec().blunt_trauma_dampening_btd)
        .max()
        .unwrap_or(0);

    let sections = [
        ("ALL", "All Sections"),
        ("VAULT", "Vault & .satyachar"),
        ("STATUS", "Status & Hit Locations"),
        ("STATS", "30 Characteristics & Aptitudes"),
        ("SKILLS", "Skills & Talents Tables"),
        ("EQUIPMENT", "Weapons, Armor & Gear"),
        ("NOTES", "Notes & Lifepath"),
    ];
    let nav_pills: String = sections
        .iter()
        .map(|&(code, label)| {
            let cls = if sheet.section_filter.eq_ignore_ascii_case(code) {
                "wiz-step-btn active"
            } else {
                "wiz-step-btn"
            };
            format!(
                r#"<button type="button" class="{cls}" onclick="window.satyaCharSheetSetSection('{code}')">{label}</button>"#
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let show_all = sheet.section_filter == "ALL";
    let vault_html = if show_all || sheet.section_filter == "VAULT" {
        format_character_vault_and_assets_html(sheet)
    } else {
        String::new()
    };
    let status_html = if show_all || sheet.section_filter == "STATUS" {
        format_character_status_and_hit_locations_html(sheet)
    } else {
        String::new()
    };
    let stats_html = if show_all || sheet.section_filter == "STATS" {
        format_all_characteristics_and_aptitudes_html(sheet)
    } else {
        String::new()
    };
    let skills_html = if show_all || sheet.section_filter == "SKILLS" {
        format_talents_and_skills_tables_html(sheet)
    } else {
        String::new()
    };
    let equip_html = if show_all || sheet.section_filter == "EQUIPMENT" {
        format_equipment_weapons_and_armor_html(sheet)
    } else {
        String::new()
    };
    let notes_html = if show_all || sheet.section_filter == "NOTES" {
        format_character_notes_and_lifepath_html(sheet)
    } else {
        String::new()
    };
    let badge_html = format_verification_badge_html(sheet);

    format!(
        r#"<div class="char-sheet-shell">
            <div class="char-dossier-banner">
                <div>
                    <div class="char-dossier-title">{name} <span class="pill pill-iggy">Age {age} · {origin}</span> {badge_html}</div>
                    <div class="subtitle">{role}</div>
                </div>
                <div class="char-section-nav" role=" navigation" aria-label="Character Sheet Section Filter">
                    {nav_pills}
                </div>
            </div>

            <div class="char-sheet-grid">
                <div class="char-stat-box">
                    <span class="stat-label">Control (C)</span>
                    <span id="char-stat-control" class="stat-value">{control_dex_tac}</span>
                    <span class="stat-note">DEX {dex} / TAC {tac} Composite</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Speed (SPD)</span>
                    <span id="char-stat-spd" class="stat-value">{spd}</span>
                    <span class="stat-note">OODA {ooda} / Tempo Cap</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Kinesthesia (KIN)</span>
                    <span id="char-stat-kin" class="stat-value">{kin}</span>
                    <span class="stat-note">Balance / Melee Control</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Analysis (ANA)</span>
                    <span id="char-stat-ana" class="stat-value">{ana}</span>
                    <span class="stat-note">Quick Thinking (D_cog {dcog})</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Discipline (DIS)</span>
                    <span id="char-stat-dis" class="stat-value">{dis}</span>
                    <span class="stat-note">FT {ft} / WSR {wsr} / CMP {cmp}</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Armor Mitigation</span>
                    <span id="char-stat-armor" class="stat-value">PR {max_ballistic_pr} / BTD {max_btd}</span>
                    <span class="stat-note">Ballistic PR &amp; Blunt Dampening</span>
                </div>
            </div>

            {vault_html}
            {status_html}
            {stats_html}
            {skills_html}
            {equip_html}
            {notes_html}
        </div>"#,
        name = escape_html(&sheet.character_name),
        age = sheet.age_years,
        origin = escape_html(&sheet.origin_label),
        role = escape_html(&sheet.callsign_or_role),
        dex = c.dex,
        tac = c.tac,
        spd = c.spd,
        ooda = derived.ooda_initiative,
        kin = c.kin,
        ana = c.ana,
        dcog = derived.d_cog_quick_thinking,
        dis = c.dis,
        ft = derived.ft_fatigue_threshold,
        wsr = derived.wsr_wound_shock_resistance,
        cmp = derived.cmp_social_composure_threshold,
    )
}
