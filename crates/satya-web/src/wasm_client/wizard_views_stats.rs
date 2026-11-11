//! Step 1 (Primary/Sensory Pools & Correlated Height/Mass), Step 2 (Psyche Profile), and
//! Step 3 (Domain Aptitudes) HTML formatters for the Character Creation Wizard.

use crate::wasm_client::dom::escape_html;
use crate::wasm_client::wizard::CharacterWizardState;
use satya_engine::{
    AptitudeCode, CharacterCharacteristics, CharacteristicCode, CharacteristicGroupPool,
    CharacteristicRollMethod, PopulationTier, PsychePole, SingleCharacteristicRoll,
    compute_aptitude_min_c_relief, compute_aptitude_training_time_multiplier,
    psyche_pole_descriptor,
};

/// Formats a single `2d4 + 3d5 (drop lowest d5)` or `2d4 + 2d5` roll into compact HTML showing individual dice.
pub fn format_single_char_roll_badge_html(idx: usize, roll: &SingleCharacteristicRoll) -> String {
    let d4_str = format!("{}d4:[{},{}]", 2, roll.d4_rolls[0], roll.d4_rolls[1]);
    let d5_parts: Vec<String> = roll
        .d5_rolls
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            if roll.dropped_d5_index == Some(i) {
                format!("<del title=\"Dropped lowest d5\">{v}</del>")
            } else {
                v.to_string()
            }
        })
        .collect();
    let d5_str = format!("{}d5:[{}]", roll.d5_rolls.len(), d5_parts.join(","));
    let cls = if roll.dropped_from_pool {
        "wiz-roll-chip dropped"
    } else {
        "wiz-roll-chip kept"
    };
    let drop_tag = if roll.dropped_from_pool {
        r#" <span class="wiz-drop-tag">DROPPED</span>"#
    } else {
        ""
    };
    format!(
        r#"<span class="{cls}"><strong>#{} = {}</strong> <span class="mono">({d4_str} + {d5_str})</span>{drop_tag}</span>"#,
        idx + 1,
        roll.total
    )
}

pub(crate) trait PoolCodeLabel: Copy {
    fn code_label(self) -> &'static str;
}

impl PoolCodeLabel for CharacteristicCode {
    fn code_label(self) -> &'static str {
        self.as_str()
    }
}

impl PoolCodeLabel for AptitudeCode {
    fn code_label(self) -> &'static str {
        self.as_str()
    }
}

/// Formats `<option>` tags for assigning or swapping a rolled pool slot across Characteristics, Psyche, or Aptitudes.
pub(crate) fn format_pool_slot_options_html<C: PoolCodeLabel>(
    slots: &[Option<usize>],
    cur_idx: usize,
    codes: &[C],
    none_label: &str,
    pool_len: usize,
    format_slot_prefix: impl Fn(usize) -> String,
) -> String {
    let mut opts = Vec::with_capacity(pool_len + 1);
    let cur_selected = slots.get(cur_idx).copied().flatten();
    let none_selected = if cur_selected.is_none() {
        " selected"
    } else {
        ""
    };
    opts.push(format!(r#"<option value=""{none_selected}>{none_label}</option>"#));
    let cur_code = codes.get(cur_idx).map_or("?", |c| c.code_label());
    for slot_idx in 0..pool_len {
        let sel_attr = if cur_selected == Some(slot_idx) {
            " selected"
        } else {
            ""
        };
        let holder = slots
            .iter()
            .position(|&s| s == Some(slot_idx))
            .and_then(|other_idx| {
                if other_idx == cur_idx {
                    Some(format!("Assigned to {cur_code}"))
                } else {
                    codes
                        .get(other_idx)
                        .map(|other_c| format!("Swap w/ {}", other_c.code_label()))
                }
            })
            .unwrap_or_else(|| "Available".to_string());
        let prefix = format_slot_prefix(slot_idx);
        opts.push(format!(
            r#"<option value="{slot_idx}"{sel_attr}>{prefix} ({holder})</option>"#
        ));
    }
    opts.join("")
}

/// Formats a domain group card (`Physical`, `Mental`, `Sensory`, or `Social`) in Step 1 of the Wizard.
#[allow(clippy::too_many_arguments)]
pub(crate) fn format_group_pool_section_html(
    group_slug: &str,
    title: &str,
    subtitle: &str,
    codes: &[CharacteristicCode],
    pool: Option<&CharacteristicGroupPool>,
    slots: &[Option<usize>],
    chars: &CharacterCharacteristics,
    method: CharacteristicRollMethod,
) -> String {
    let method_badge = match (group_slug, method) {
        ("social", CharacteristicRollMethod::Method1DropLowestD5) => {
            "Method 1: 3 × (2d4 + 3d5 drop lowest d5)"
        }
        ("social", CharacteristicRollMethod::Method2PoolKeepHighest) => {
            "Method 2: 4 × (2d4 + 2d5), keep 3 highest"
        }
        (_, CharacteristicRollMethod::Method1DropLowestD5) => {
            "Method 1: 6 × (2d4 + 3d5 drop lowest d5)"
        }
        (_, CharacteristicRollMethod::Method2PoolKeepHighest) => {
            "Method 2: 8 × (2d4 + 2d5), keep 6 highest"
        }
    };

    let pool_html = if let Some(p) = pool {
        let raw_badges: String = p
            .raw_rolls
            .iter()
            .enumerate()
            .map(|(i, r)| format_single_char_roll_badge_html(i, r))
            .collect::<Vec<_>>()
            .join(" ");
        let kept_chips: String = p
            .kept_values
            .iter()
            .enumerate()
            .map(|(slot_idx, &val)| {
                let assigned_to = slots
                    .iter()
                    .position(|&s| s == Some(slot_idx))
                    .and_then(|c_idx| codes.get(c_idx))
                    .map(|c| c.as_str());
                let assign_badge = match assigned_to {
                    Some(code_str) => format!(" → {code_str}"),
                    None => " (Available)".to_string(),
                };
                let pill_cls = if assigned_to.is_some() {
                    "pill pill-active"
                } else {
                    "pill pill-iggy"
                };
                format!(
                    r#"<span class="{pill_cls}">Slot #{}: <strong>{val}</strong>{assign_badge}</span>"#,
                    slot_idx + 1
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            r#"<div class="wiz-pool-readout">
                <div class="wiz-raw-rolls-row"><span class="roster-title">Rolled Dice:</span> {raw_badges}</div>
                <div class="wiz-kept-slots-row"><span class="roster-title">Kept Pool Slots:</span> {kept_chips}</div>
            </div>"#
        )
    } else {
        r#"<div class="wiz-pool-empty">Click <strong>Roll Group Pool</strong> (or <strong>Roll All Characteristic Groups</strong> above) to generate dice scores and assign them to each characteristic in any order.</div>"#.to_string()
    };

    let rows_html: String = codes
        .iter()
        .enumerate()
        .map(|(c_idx, &code)| {
            let cur_val = chars.get_scale(code);
            let tier = PopulationTier::from_controlling_scale(cur_val);
            let base_tn = PopulationTier::base_tn(cur_val);
            let select_id = format!("wiz-slot-{group_slug}-{}", code.as_str().to_ascii_lowercase());

            let options_html = if let Some(p) = pool {
                format_pool_slot_options_html(
                    slots,
                    c_idx,
                    codes,
                    "-- Choose Pool Slot --",
                    p.kept_values.len(),
                    |slot_idx| format!("Slot #{}: Score {}", slot_idx + 1, p.kept_values[slot_idx]),
                )
            } else {
                format!(r#"<option value="" selected>Default (C = {cur_val}) — Roll Pool First</option>"#)
            };

            format!(
                r#"<div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <div>
                            <span class="wiz-char-code">{code_str}</span>
                            <strong class="wiz-char-name">{full_name}</strong>
                        </div>
                        <span class="stat-value">{cur_val}</span>
                    </div>
                    <label for="{select_id}">
                        <span>Assign Rolled Pool Score</span>
                        <select
                            id="{select_id}"
                            name="{select_id}"
                            onchange="window.satyaWizardAssignSlot('{group_slug}', {c_idx})"
                        >
                            {options_html}
                        </select>
                    </label>
                    <div class="stat-note">Base TN {base_tn} · {tier_label}</div>
                </div>"#,
                code_str = code.as_str(),
                full_name = escape_html(code.full_name()),
                tier_label = escape_html(tier.label()),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>{title}</span>
                <span class="pill pill-iggy">{method_badge}</span>
            </legend>
            <div class="pane-toolbar">
                <span class="subtitle">{subtitle}</span>
                <div class="btn-row-compact">
                    <button
                        type="button"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaWizardRollGroup('{group_slug}')"
                    >
                        "Roll {title} Pool"
                    </button>
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaWizardAutoAssignGroup('{group_slug}')"
                    >
                        "Assign In Order"
                    </button>
                </div>
            </div>
            {pool_html}
            <div class="wiz-char-grid">
                {rows_html}
            </div>
        </fieldset>"#,
        title = escape_html(title),
        subtitle = escape_html(subtitle),
    )
}

/// Formats the special Height (`HGT`) and Mass (`MAS`) section in Step 1 of the Wizard.
pub(crate) fn format_height_mass_section_html(st: &CharacterWizardState) -> String {
    let hgt_m = st.characteristics.hgt_m;
    let hgt_cm = (hgt_m * 100.0).round() as i32;
    let mas_kg = st.characteristics.mas_kg;
    let mas_lbs = (mas_kg * 2.20462).round() as i32;
    let hgt_scale = st.characteristics.get_scale(CharacteristicCode::HGT);
    let mas_scale = st.characteristics.get_scale(CharacteristicCode::MAS);
    let derived = st.characteristics.compute_derived(0);

    let roll_readout = if let Some(ref hm) = st.height_mass_roll {
        let h_badge = format_single_char_roll_badge_html(0, &hm.height_roll);
        let b_badge = format_single_char_roll_badge_html(1, &hm.build_roll);
        let mode_note = if st.height_mass_manual_override {
            "Manual Slider/Input Override Active"
        } else {
            "Correlated Stature + Build/FOR BMI Roll Active"
        };
        format!(
            r#"<div class="wiz-pool-readout">
                <div class="wiz-raw-rolls-row">
                    <span class="roster-title">Height Roll (2d4+2d5):</span> {h_badge}
                    <span class="roster-title">Build Roll (2d4+2d5):</span> {b_badge}
                    <span class="pill pill-iggy">FOR = {for_stat} · BMI = {bmi:.1} kg/m² · {mode_note}</span>
                </div>
            </div>"#,
            for_stat = hm.for_stat,
            bmi = hm.bmi,
        )
    } else {
        r#"<div class="wiz-pool-empty">Height &amp; Mass use a correlated bell-curve generator: <code>2d4 + 2d5</code> sets stature around <code>1.72 m</code> (±3 cm/pt), and a second <code>2d4 + 2d5</code> Build roll combines with <strong>Force (FOR)</strong> to compute realistic body mass via BMI (or adjust the sliders directly).</div>"#.to_string()
    };

    format!(
        r#"<fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Physical Dimensions: Height (HGT) &amp; Mass (MAS)</span>
                <span class="pill pill-active">{hgt_m:.2} m ({hgt_cm} cm) · {mas_kg:.1} kg ({mas_lbs} lb)</span>
            </legend>
            <div class="pane-toolbar">
                <span class="subtitle">Correlated Stature &amp; Somatic Build/Force Generator (with manual override controls)</span>
                <div class="btn-row-compact">
                    <button
                        type="button"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaWizardRollHeightMass()"
                    >
                        "Roll Correlated Height &amp; Mass"
                    </button>
                </div>
            </div>
            {roll_readout}
            <div class="wiz-char-grid">
                <div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <div>
                            <span class="wiz-char-code">HGT</span>
                            <strong class="wiz-char-name">Height (Meters)</strong>
                        </div>
                        <span class="stat-value">{hgt_m:.2} m</span>
                    </div>
                    <label for="wiz-input-hgt-m">
                        <span>Height (1.40 m – 2.20 m · Scale C = {hgt_scale})</span>
                        <input
                            id="wiz-input-hgt-m"
                            name="wiz_input_hgt_m"
                            type="number"
                            step="0.01"
                            min="1.40"
                            max="2.20"
                            value="{hgt_m:.2}"
                            onchange="window.satyaWizardUpdateHeightMass()"
                        />
                    </label>
                    <div class="stat-note">Height Reach Edge: {reach_edge:+} (Melee Reach Index: {reach_idx:+})</div>
                </div>
                <div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <div>
                            <span class="wiz-char-code">MAS</span>
                            <strong class="wiz-char-name">Mass (Kilograms)</strong>
                        </div>
                        <span class="stat-value">{mas_kg:.1} kg</span>
                    </div>
                    <label for="wiz-input-mas-kg">
                        <span>Mass (40.0 kg – 160.0 kg · Scale C = {mas_scale})</span>
                        <input
                            id="wiz-input-mas-kg"
                            name="wiz_input_mas_kg"
                            type="number"
                            step="0.5"
                            min="40.0"
                            max="160.0"
                            value="{mas_kg:.1}"
                            onchange="window.satyaWizardUpdateHeightMass()"
                        />
                    </label>
                    <div class="stat-note">Mass-Momentum Factor (MMF): {mmf:.1} (at SPD {spd})</div>
                </div>
            </div>
        </fieldset>"#,
        reach_edge = derived.height_reach_edge,
        reach_idx = derived.melee_reach_index,
        mmf = derived.mmf_momentum_factor,
        spd = st.characteristics.spd,
    )
}

/// Formats Step 2: Psyche Characteristics (`7` bipolar traits: `EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`).
pub(crate) fn format_psyche_step_html(st: &CharacterWizardState) -> String {
    let pool_html = if let Some(ref pool) = st.psyche_pool {
        let badges: String = pool
            .iter()
            .enumerate()
            .map(|(i, r)| format_single_char_roll_badge_html(i, r))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            r#"<div class="wiz-pool-readout">
                <div class="wiz-raw-rolls-row"><span class="roster-title">Rolled Psyche Pool (7 × 2d4+2d5):</span> {badges}</div>
            </div>"#
        )
    } else {
        r#"<div class="wiz-pool-empty">Psyche traits quantify behavioral drives (`1–7` Low Pole, `8–12` Balanced, `13–20` High Pole). You can <strong>roll randomly (`2d4 + 2d5`)</strong> or <strong>choose any value (`1–20`)</strong> directly using the sliders and number inputs below.</div>"#.to_string()
    };

    let cards_html: String = CharacteristicCode::PSYCHE_ALL
        .iter()
        .enumerate()
        .map(|(p_idx, &code)| {
            let val = st.characteristics.get_scale(code);
            let pole = PsychePole::from_score(val);
            let desc = psyche_pole_descriptor(code, val);
            let (pole_badge_cls, pole_name) = match pole {
                PsychePole::LowPole => ("pill pill-iggy", "Low Pole (1–7)"),
                PsychePole::Balanced => ("pill pill-active", "Balanced (8–12)"),
                PsychePole::HighPole => ("pill pill-muted", "High Pole (13–20)"),
            };
            let code_lower = code.as_str().to_ascii_lowercase();
            let slider_id = format!("wiz-psyche-range-{code_lower}");
            let num_id = format!("wiz-psyche-num-{code_lower}");
            let slot_id = format!("wiz-slot-psyche-{code_lower}");

            let slot_select_html = if let Some(ref pool) = st.psyche_pool {
                let opts_html = format_pool_slot_options_html(
                    &st.psyche_slots,
                    p_idx,
                    &CharacteristicCode::PSYCHE_ALL,
                    &format!("-- Manual / Custom Score ({val}) --"),
                    pool.len(),
                    |slot_idx| format!("Roll #{}: {}", slot_idx + 1, pool[slot_idx].total),
                );
                format!(
                    r#"<label for="{slot_id}">
                        <span>Assign from Rolled Psyche Pool</span>
                        <select id="{slot_id}" name="{slot_id}" onchange="window.satyaWizardAssignSlot('psyche', {p_idx})">
                            {opts_html}
                        </select>
                    </label>"#
                )
            } else {
                String::new()
            };

            format!(
                r#"<div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <div>
                            <span class="wiz-char-code">{code_str}</span>
                            <strong class="wiz-char-name">{full_name}</strong>
                        </div>
                        <span class="stat-value">{val}</span>
                    </div>
                    {slot_select_html}
                    <div class="wiz-psyche-slider-row">
                        <label for="{slider_id}">
                            <span>Player-Chosen Score (1–20)</span>
                            <input
                                id="{slider_id}"
                                name="{slider_id}"
                                type="range"
                                min="1"
                                max="20"
                                value="{val}"
                                oninput="window.satyaWizardSetPsyche('{code_str}', this.value)"
                                onchange="window.satyaWizardSetPsyche('{code_str}', this.value)"
                            />
                        </label>
                        <label for="{num_id}">
                            <span>Exact</span>
                            <input
                                id="{num_id}"
                                name="{num_id}"
                                type="number"
                                min="1"
                                max="20"
                                value="{val}"
                                onchange="window.satyaWizardSetPsyche('{code_str}', this.value)"
                            />
                        </label>
                    </div>
                    <div class="wiz-psyche-pole-row">
                        <span class="{pole_badge_cls}">{pole_name}</span>
                        <span class="stat-note"><strong>{desc}</strong></span>
                    </div>
                </div>"#,
                code_str = code.as_str(),
                full_name = escape_html(code.full_name()),
                desc = escape_html(desc),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Step 2: Psyche Characteristics (The 7 Invisible Behavioral Traits)</span>
                <span class="pill pill-iggy">Roll 2d4 + 2d5 Randomly OR Choose Freely (1–20)</span>
            </legend>
            <div class="pane-toolbar">
                <span class="subtitle">Governs roleplay disposition, combat morale, Psyche Friction checks, and Social Combat resonance.</span>
                <div class="btn-row-compact">
                    <button
                        type="button"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaWizardRollPsychePool()"
                    >
                        "Roll 7 × (2d4 + 2d5) Pool"
                    </button>
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaWizardResetPsycheBalanced()"
                    >
                        "Set All Balanced (10)"
                    </button>
                </div>
            </div>
            {pool_html}
            <div class="wiz-char-grid">
                {cards_html}
            </div>
        </fieldset>"#
    )
}

/// Formats Step 3: Domain Aptitudes (`11 × 3d6` drop 2 lowest $\to$ 9 kept scores in `-2..=+3`).
pub(crate) fn format_aptitudes_step_html(st: &CharacterWizardState) -> String {
    let pool_html = if let Some(ref pool) = st.aptitude_pool {
        let raw_badges: String = pool
            .raw_rolls
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                let cls = if r.dropped_from_pool {
                    "wiz-roll-chip dropped"
                } else {
                    "wiz-roll-chip kept"
                };
                let drop_tag = if r.dropped_from_pool {
                    r#" <span class="wiz-drop-tag">DROPPED</span>"#
                } else {
                    ""
                };
                format!(
                    r#"<span class="{cls}"><strong>#{}: {:+}</strong> <span class="mono">(3d6:[{},{},{}]={})</span>{drop_tag}</span>"#,
                    idx + 1,
                    r.aptitude_value,
                    r.d6_rolls[0],
                    r.d6_rolls[1],
                    r.d6_rolls[2],
                    r.sum_3d6
                )
            })
            .collect::<Vec<_>>()
            .join(" ");

        let kept_chips: String = pool
            .kept_values
            .iter()
            .enumerate()
            .map(|(slot_idx, &val)| {
                let assigned_to = st
                    .aptitude_slots
                    .iter()
                    .position(|&s| s == Some(slot_idx))
                    .and_then(|a_idx| AptitudeCode::ALL.get(a_idx))
                    .map(|a| a.as_str());
                let assign_badge = match assigned_to {
                    Some(code_str) => format!(" → {code_str}"),
                    None => " (Available)".to_string(),
                };
                let pill_cls = if val >= 2 {
                    "pill pill-active"
                } else if val < 0 {
                    "pill pill-muted"
                } else {
                    "pill pill-iggy"
                };
                format!(
                    r#"<span class="{pill_cls}">Slot #{}: <strong>{val:+}</strong>{assign_badge}</span>"#,
                    slot_idx + 1
                )
            })
            .collect::<Vec<_>>()
            .join(" ");

        format!(
            r#"<div class="wiz-pool-readout">
                <div class="wiz-raw-rolls-row"><span class="roster-title">11 × 3d6 Rolls (2 Lowest Dropped):</span> {raw_badges}</div>
                <div class="wiz-kept-slots-row"><span class="roster-title">9 Kept Aptitude Slots:</span> {kept_chips}</div>
            </div>"#
        )
    } else {
        r#"<div class="wiz-pool-empty">Domain Aptitudes (<code>-2</code> to <code>+3</code>) are rolled <strong>11 times on a 3d6 bell-curve table, dropping the 2 lowest</strong>: <code>3–4 = -2</code> (1.9%), <code>5–7 = -1</code> (14.4%), <code>8–13 = 0</code> (62.5%), <code>14–15 = +1</code> (16.2%), <code>16–17 = +2</code> (4.6%), and <code>18 = +3</code> (0.46% per roll / ~5% per character). Click <strong>Roll 11 Aptitudes (Drop 2 Lowest)</strong> to generate your 9 values.</div>"#.to_string()
    };

    let cards_html: String = AptitudeCode::ALL
        .iter()
        .enumerate()
        .map(|(a_idx, &code)| {
            let val = st.aptitudes.get(code);
            let time_mult = (compute_aptitude_training_time_multiplier(val) * 100.0).round() as i32;
            let relief = compute_aptitude_min_c_relief(val);
            let relief_note = if relief > 0 {
                " · Min C -1 Relief & Early Tree Unlock!"
            } else {
                ""
            };
            let select_id = format!(
                "wiz-slot-aptitude-{}",
                code.as_str().to_ascii_lowercase().replace('-', "_")
            );

            let options_html = if let Some(ref pool) = st.aptitude_pool {
                format_pool_slot_options_html(
                    &st.aptitude_slots,
                    a_idx,
                    &AptitudeCode::ALL,
                    "-- Choose Aptitude Slot --",
                    pool.kept_values.len(),
                    |slot_idx| {
                        format!(
                            "Slot #{}: Aptitude {:+}",
                            slot_idx + 1,
                            pool.kept_values[slot_idx]
                        )
                    },
                )
            } else {
                format!(r#"<option value="" selected>Baseline ({val:+}) — Roll Pool First</option>"#)
            };

            format!(
                r#"<div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <div>
                            <span class="wiz-char-code">{code_str}</span>
                            <strong class="wiz-char-name">{full_name}</strong>
                        </div>
                        <span class="stat-value">{val:+}</span>
                    </div>
                    <label for="{select_id}">
                        <span>Assign Kept Aptitude Score</span>
                        <select
                            id="{select_id}"
                            name="{select_id}"
                            onchange="window.satyaWizardAssignSlot('aptitude', {a_idx})"
                        >
                            {options_html}
                        </select>
                    </label>
                    <div class="stat-note">Study Mod {val:+} · Training Time {time_mult}%{relief_note}</div>
                </div>"#,
                code_str = code.as_str(),
                full_name = escape_html(code.full_name()),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Step 3: Domain Aptitudes (11 × 3d6 Bell-Curve, Drop 2 Lowest → 9 Aptitudes)</span>
                <span class="pill pill-iggy">Centered at 0 (62.5%) · Rare +3 (0.46% / roll)</span>
            </legend>
            <div class="pane-toolbar">
                <span class="subtitle">Accelerates Lifepath school/career study ($C_{{\text{{study}}}}$) and grants prerequisite relief at +2/+3.</span>
                <div class="btn-row-compact">
                    <button
                        type="button"
                        class="btn btn-sm btn-accent"
                        onclick="window.satyaWizardRollAptitudes()"
                    >
                        "Roll 11 Aptitudes (Drop 2 Lowest)"
                    </button>
                    <button
                        type="button"
                        class="btn btn-sm btn-secondary"
                        onclick="window.satyaWizardAutoAssignGroup('aptitude')"
                    >
                        "Assign In Order"
                    </button>
                </div>
            </div>
            {pool_html}
            <div class="wiz-char-grid">
                {cards_html}
            </div>
        </fieldset>"#
    )
}
