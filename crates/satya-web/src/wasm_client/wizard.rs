//! Step-by-Step Character Creation Wizard (`#notebook-pane-wizard`) for the Left Tabbed Notebook.
//!
//! Walks a player through generating a new Satya character step by step:
//! - **Step 1: Primary & Sensory Characteristics**:
//!   - Method 1 (`2d4 + 3d5`, drop lowest `d5`, range `4–18`): Roll `6` times for Physical, `6` for Mental,
//!     `6` for Sensory, and `3` for Social; assign kept scores in any order.
//!   - Method 2 (`2d4 + 2d5`, keep highest, range `4–18`): Roll `8` times and keep the `6` highest for
//!     Physical, Mental, and Sensory; roll `4` times and keep the `3` highest for Social; assign in any order.
//!   - Correlated Height (`m`) & Build/Force-adjusted Mass (`kg`) generator (`2d4 + 2d5` stature around `1.72 m`
//!     + `2d4 + 2d5` build & `FOR` BMI scaling), with manual slider/number override.
//! - **Step 2: Psyche Profile (`7` Bipolar Behavioral Traits)**:
//!   - Either roll randomly (`2d4 + 2d5` direct or 7-value assignable pool) or choose values freely (`1–20`)
//!     with live `Low Pole` / `Balanced` / `High Pole` descriptors.
//! - **Step 3: Domain Aptitudes (`9` Learning Affinities, `-2` to `+3`)**:
//!   - Roll `3d6` `11` times using the canonical bell-curve table (highly centered at `0` at `62.5%`,
//!     rarely producing `+3` at `0.46%` per roll), drop the `2` lowest rolls, and assign the `9` kept scores.
//! - **Step 4: Derived Telemetry Review & Apply to Character Sheet**:
//!   - Displays all 10 Derived Characteristics (`POW`, `MMF`, `Melee Reach`, `D_cog`, `OODA`, `AWR`, `FT`,
//!     `WSR`, `CMP`, `C_study`) and applies the character to the active Character Sheet & Task Roller.

use std::collections::BTreeMap;

use crate::wasm_client::chat_and_ttrpg::{append_system_notice, satya_update_cascade_preview};
use crate::wasm_client::dom::{
    escape_html, get_element, input_value, select_value, set_feedback, set_input_value, set_text,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use satya_engine::{
    AptitudeCode, AptitudePoolRoll, CANONICAL_SKILL_TREES, CareerTermConfig, CareerTermTrack,
    CharacterAptitudes, CharacterCharacteristics, CharacteristicCategory, CharacteristicCode,
    CharacteristicGroupPool, CharacteristicRollMethod, CorrelatedHeightMassRoll, HighSchoolTrack,
    LifepathBaselineResult, LifepathBuildConfig, PopulationTier, PrerequisiteMode, PsychePole,
    SingleCharacteristicRoll, SkillTreeId, SkillType, SocioeconomicClass, StreetGangArchetype,
    auto_allocate_career_and_elective_skills, compute_academic_study_controller,
    compute_aptitude_min_c_relief, compute_aptitude_training_time_multiplier,
    compute_correlated_height_mass, compute_lifepath_baseline_from_config,
    evaluate_operational_heat, lookup_campaign_sqm, lookup_skill_node, psyche_pole_descriptor,
    resolve_skill_tree_default, resolve_socioeconomic_origin, roll_aptitude_pool_11_drop_2,
    roll_characteristic_2d4_2d5, roll_characteristic_group_pool, roll_correlated_height_mass,
    skills_in_tree, validate_skill_advancement,
};
use wasm_bindgen::prelude::*;

/// Client-side state for the step-by-step Character Creation Wizard tab (`#notebook-pane-wizard`).
#[derive(Debug, Clone)]
pub struct CharacterWizardState {
    /// Active wizard step (`1` = Primary/Sensory, `2` = Psyche, `3` = Aptitudes, `4` = Lifepath, `5` = Skills, `6` = Review & Apply).
    pub active_step: u8,
    /// Character name or callsign being created.
    pub character_name: String,
    /// Selected dice generation method for Physical, Mental, Sensory, and Social pools.
    pub roll_method: CharacteristicRollMethod,
    /// Rolled pool for the 6 Physical characteristics (`SPD`, `FOR`, `DEX`, `KIN`, `HLT`, `FIT`).
    pub physical_pool: Option<CharacteristicGroupPool>,
    /// Assigned pool slot index (`0..6`) for each of `CharacteristicCode::PHYSICAL_SCALED`.
    pub physical_slots: [Option<usize>; 6],
    /// Correlated Height (`m`) and Mass (`kg`) roll state.
    pub height_mass_roll: Option<CorrelatedHeightMassRoll>,
    /// True if the player manually edited Height or Mass via input controls.
    pub height_mass_manual_override: bool,
    /// Rolled pool for the 6 Mental characteristics (`MEM`, `ANA`, `INS`, `CRE`, `DIS`, `FOC`).
    pub mental_pool: Option<CharacteristicGroupPool>,
    /// Assigned pool slot index (`0..6`) for each of `CharacteristicCode::MENTAL_ALL`.
    pub mental_slots: [Option<usize>; 6],
    /// Rolled pool for the 6 Sensory characteristics (`SIG`, `HEA`, `TAC`, `OLF`, `TAS`, `ITU`).
    pub sensory_pool: Option<CharacteristicGroupPool>,
    /// Assigned pool slot index (`0..6`) for each of `CharacteristicCode::SENSORY_ALL`.
    pub sensory_slots: [Option<usize>; 6],
    /// Rolled pool for the 3 Social characteristics (`COM`, `PRE`, `ELO`).
    pub social_pool: Option<CharacteristicGroupPool>,
    /// Assigned pool slot index (`0..3`) for each of `CharacteristicCode::SOCIAL_ALL`.
    pub social_slots: [Option<usize>; 3],
    /// Mode for Psyche traits (`"CHOOSE"` = manual sliders, `"ROLL_DIRECT"` = `2d4+2d5` in order, `"ROLL_POOL"` = `7 x 2d4+2d5` pool).
    pub psyche_mode: String,
    /// Optional 7-roll pool for Psyche traits (`2d4 + 2d5`).
    pub psyche_pool: Option<Vec<SingleCharacteristicRoll>>,
    /// Assigned pool slot index (`0..7`) for each of `CharacteristicCode::PSYCHE_ALL` when using `ROLL_POOL` or `ROLL_DIRECT`.
    pub psyche_slots: [Option<usize>; 7],
    /// Rolled `11 x 3d6 (drop 2 lowest -> 9 kept)` pool for Domain Aptitudes.
    pub aptitude_pool: Option<AptitudePoolRoll>,
    /// Assigned pool slot index (`0..9`) for each of `AptitudeCode::ALL`.
    pub aptitude_slots: [Option<usize>; 9],
    /// Current working `CharacterCharacteristics` assembled by the wizard.
    pub characteristics: CharacterCharacteristics,
    /// Current working `CharacterAptitudes` assembled by the wizard.
    pub aptitudes: CharacterAptitudes,
    /// Step 4: Chronological 5-Stage Lifepath configuration & rolls.
    pub lifepath: LifepathBuildConfig,
    /// Step 5: Current skill levels (`N`) across all 8 Hierarchical Skill Trees.
    pub skills: BTreeMap<String, usize>,
    /// Step 5: Tree filter (`"ALL"` or `"TREE_1"` .. `"TREE_8"`).
    pub skill_tree_filter: String,
    /// Step 5: View filter (`"ALL"`, `"TRAINED"`, or `"UNLOCKABLE"`).
    pub skill_view_mode: String,
}

impl Default for CharacterWizardState {
    fn default() -> Self {
        let chars = CharacterCharacteristics::default();
        let apts = CharacterAptitudes::default();
        let lifepath = LifepathBuildConfig::default();
        let baseline = compute_lifepath_baseline_from_config(&lifepath, &chars, &apts);
        let mut skills = baseline.baseline_skills.clone();
        auto_allocate_career_and_elective_skills(&baseline, &lifepath, &chars, &apts, &mut skills);

        Self {
            active_step: 1,
            character_name: "New Operator".to_string(),
            roll_method: CharacteristicRollMethod::Method1DropLowestD5,
            physical_pool: None,
            physical_slots: [None; 6],
            height_mass_roll: None,
            height_mass_manual_override: false,
            mental_pool: None,
            mental_slots: [None; 6],
            sensory_pool: None,
            sensory_slots: [None; 6],
            social_pool: None,
            social_slots: [None; 3],
            psyche_mode: "CHOOSE".to_string(),
            psyche_pool: None,
            psyche_slots: [None; 7],
            aptitude_pool: None,
            aptitude_slots: [None; 9],
            characteristics: chars,
            aptitudes: apts,
            lifepath,
            skills,
            skill_tree_filter: "ALL".to_string(),
            skill_view_mode: "ALL".to_string(),
        }
    }
}

fn browser_die_roller(sides: i32) -> i32 {
    let s = sides.max(1);
    ((js_sys::Math::random() * (s as f64)).floor() as i32 + 1).clamp(1, s)
}

fn roll_2d20_pairs() -> [(i32, i32); 2] {
    [
        (browser_die_roller(20), browser_die_roller(20)),
        (browser_die_roller(20), browser_die_roller(20)),
    ]
}

impl CharacterWizardState {
    /// Computes the current [`LifepathBaselineResult`] from `self.lifepath`, `self.characteristics`, and `self.aptitudes`.
    #[must_use]
    pub fn compute_lifepath_baseline(&self) -> LifepathBaselineResult {
        compute_lifepath_baseline_from_config(
            &self.lifepath,
            &self.characteristics,
            &self.aptitudes,
        )
    }

    /// Ensures `self.skills` respects the Stage 1–4 `baseline_skills` floor and clamps any skills that lost prerequisites.
    pub fn sync_lifepath_skills(&mut self) {
        let baseline = self.compute_lifepath_baseline();
        for (k, &v) in &baseline.baseline_skills {
            let entry = self.skills.entry(k.clone()).or_insert(0);
            if *entry < v {
                *entry = v;
            }
        }
    }

    /// Resets `self.skills` to the Stage 1–4 baseline and auto-allocates Career & Elective skill budgets.
    pub fn rebuild_and_auto_allocate_skills(&mut self) {
        let baseline = self.compute_lifepath_baseline();
        self.skills = baseline.baseline_skills.clone();
        auto_allocate_career_and_elective_skills(
            &baseline,
            &self.lifepath,
            &self.characteristics,
            &self.aptitudes,
            &mut self.skills,
        );
    }

    /// Synchronizes `self.characteristics` and `self.aptitudes` from currently assigned pool slots.
    pub fn sync_from_slots(&mut self) {
        if let Some(ref pool) = self.physical_pool {
            for (idx, &code) in CharacteristicCode::PHYSICAL_SCALED.iter().enumerate() {
                if let Some(slot) = self.physical_slots[idx]
                    && let Some(&val) = pool.kept_values.get(slot)
                {
                    self.characteristics.set_scale(code, val);
                }
            }
        }
        if !self.height_mass_manual_override
            && let Some(ref mut hm) = self.height_mass_roll
        {
            hm.for_stat = self.characteristics.for_stat;
            let (hgt_m, bmi, mas_kg) = compute_correlated_height_mass(
                hm.height_roll.total,
                hm.build_roll.total,
                hm.for_stat,
            );
            hm.height_m = hgt_m;
            hm.bmi = bmi;
            hm.mass_kg = mas_kg;
            self.characteristics.hgt_m = hgt_m;
            self.characteristics.mas_kg = mas_kg;
        }
        if let Some(ref pool) = self.mental_pool {
            for (idx, &code) in CharacteristicCode::MENTAL_ALL.iter().enumerate() {
                if let Some(slot) = self.mental_slots[idx]
                    && let Some(&val) = pool.kept_values.get(slot)
                {
                    self.characteristics.set_scale(code, val);
                }
            }
        }
        if let Some(ref pool) = self.sensory_pool {
            for (idx, &code) in CharacteristicCode::SENSORY_ALL.iter().enumerate() {
                if let Some(slot) = self.sensory_slots[idx]
                    && let Some(&val) = pool.kept_values.get(slot)
                {
                    self.characteristics.set_scale(code, val);
                }
            }
        }
        if let Some(ref pool) = self.social_pool {
            for (idx, &code) in CharacteristicCode::SOCIAL_ALL.iter().enumerate() {
                if let Some(slot) = self.social_slots[idx]
                    && let Some(&val) = pool.kept_values.get(slot)
                {
                    self.characteristics.set_scale(code, val);
                }
            }
        }
        if let Some(ref pool) = self.psyche_pool {
            for (idx, &code) in CharacteristicCode::PSYCHE_ALL.iter().enumerate() {
                if let Some(slot) = self.psyche_slots[idx]
                    && let Some(r) = pool.get(slot)
                {
                    self.characteristics.set_scale(code, r.total);
                }
            }
        }
        if let Some(ref pool) = self.aptitude_pool {
            for (idx, &code) in AptitudeCode::ALL.iter().enumerate() {
                if let Some(slot) = self.aptitude_slots[idx]
                    && let Some(&val) = pool.kept_values.get(slot)
                {
                    self.aptitudes.set(code, val);
                }
            }
        }
        self.sync_lifepath_skills();
    }
}

/// Assigns `new_slot` to `target_idx` within `slots`, swapping with any other item that already held `new_slot`.
fn assign_or_swap_slot(slots: &mut [Option<usize>], target_idx: usize, new_slot: Option<usize>) {
    if target_idx >= slots.len() {
        return;
    }
    let prev_slot = slots[target_idx];
    if let Some(ns) = new_slot {
        for (idx, slot_ref) in slots.iter_mut().enumerate() {
            if idx != target_idx && *slot_ref == Some(ns) {
                *slot_ref = prev_slot;
                break;
            }
        }
    }
    slots[target_idx] = new_slot;
}

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

/// Formats a domain group card (`Physical`, `Mental`, `Sensory`, or `Social`) in Step 1 of the Wizard.
#[allow(clippy::too_many_arguments)]
fn format_group_pool_section_html(
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
                let mut opts = Vec::with_capacity(p.kept_values.len() + 1);
                let none_selected = if slots.get(c_idx).copied().flatten().is_none() {
                    " selected"
                } else {
                    ""
                };
                opts.push(format!(
                    r#"<option value=""{none_selected}>-- Choose Pool Slot --</option>"#
                ));
                for (slot_idx, &val) in p.kept_values.iter().enumerate() {
                    let is_sel = slots.get(c_idx).copied().flatten() == Some(slot_idx);
                    let sel_attr = if is_sel { " selected" } else { "" };
                    let holder = slots
                        .iter()
                        .position(|&s| s == Some(slot_idx))
                        .and_then(|other_idx| {
                            if other_idx == c_idx {
                                Some(format!("Assigned to {}", code.as_str()))
                            } else {
                                codes
                                    .get(other_idx)
                                    .map(|other_c| format!("Swap w/ {}", other_c.as_str()))
                            }
                        })
                        .unwrap_or_else(|| "Available".to_string());
                    opts.push(format!(
                        r#"<option value="{slot_idx}"{sel_attr}>Slot #{}: Score {val} ({holder})</option>"#,
                        slot_idx + 1
                    ));
                }
                opts.join("")
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
fn format_height_mass_section_html(st: &CharacterWizardState) -> String {
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
fn format_psyche_step_html(st: &CharacterWizardState) -> String {
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
                let mut opts = Vec::with_capacity(pool.len() + 1);
                let none_sel = if st.psyche_slots[p_idx].is_none() {
                    " selected"
                } else {
                    ""
                };
                opts.push(format!(
                    r#"<option value=""{none_sel}>-- Manual / Custom Score ({val}) --</option>"#
                ));
                for (slot_idx, r) in pool.iter().enumerate() {
                    let is_sel = st.psyche_slots[p_idx] == Some(slot_idx);
                    let sel_attr = if is_sel { " selected" } else { "" };
                    let holder = st
                        .psyche_slots
                        .iter()
                        .position(|&s| s == Some(slot_idx))
                        .and_then(|other_idx| {
                            if other_idx == p_idx {
                                Some(format!("Assigned to {}", code.as_str()))
                            } else {
                                CharacteristicCode::PSYCHE_ALL
                                    .get(other_idx)
                                    .map(|c| format!("Swap w/ {}", c.as_str()))
                            }
                        })
                        .unwrap_or_else(|| "Available".to_string());
                    opts.push(format!(
                        r#"<option value="{slot_idx}"{sel_attr}>Roll #{}: {} ({holder})</option>"#,
                        slot_idx + 1,
                        r.total
                    ));
                }
                format!(
                    r#"<label for="{slot_id}">
                        <span>Assign from Rolled Psyche Pool</span>
                        <select id="{slot_id}" name="{slot_id}" onchange="window.satyaWizardAssignSlot('psyche', {p_idx})">
                            {}
                        </select>
                    </label>"#,
                    opts.join("")
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
fn format_aptitudes_step_html(st: &CharacterWizardState) -> String {
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
                let mut opts = Vec::with_capacity(pool.kept_values.len() + 1);
                let none_sel = if st.aptitude_slots[a_idx].is_none() {
                    " selected"
                } else {
                    ""
                };
                opts.push(format!(
                    r#"<option value=""{none_sel}>-- Choose Aptitude Slot --</option>"#
                ));
                for (slot_idx, &v) in pool.kept_values.iter().enumerate() {
                    let is_sel = st.aptitude_slots[a_idx] == Some(slot_idx);
                    let sel_attr = if is_sel { " selected" } else { "" };
                    let holder = st
                        .aptitude_slots
                        .iter()
                        .position(|&s| s == Some(slot_idx))
                        .and_then(|other_idx| {
                            if other_idx == a_idx {
                                Some(format!("Assigned to {}", code.as_str()))
                            } else {
                                AptitudeCode::ALL
                                    .get(other_idx)
                                    .map(|c| format!("Swap w/ {}", c.as_str()))
                            }
                        })
                        .unwrap_or_else(|| "Available".to_string());
                    opts.push(format!(
                        r#"<option value="{slot_idx}"{sel_attr}>Slot #{}: Aptitude {v:+} ({holder})</option>"#,
                        slot_idx + 1
                    ));
                }
                opts.join("")
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

/// Formats Step 4: Chronological 5-Stage Lifepath (`Stages 1–5`).
fn format_lifepath_step_html(st: &CharacterWizardState) -> String {
    let baseline = st.compute_lifepath_baseline();
    let cfg = &st.lifepath;
    let origin = &baseline.origin;

    // Stage 1: Socioeconomic Class options
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

    let s2_lit_opts = format_override_select_options(cfg.stage2_lit_override);
    let s2_math_opts = format_override_select_options(cfg.stage2_math_override);
    let s3_math_opts = format_override_select_options(cfg.stage3_math_override);
    let s3_sci_opts = format_override_select_options(cfg.stage3_sci_override);
    let s3_hum_opts = format_override_select_options(cfg.stage3_hum_override);

    let age15_auto_sel = if cfg.age15_dropout_override.is_none() {
        " selected"
    } else {
        ""
    };
    let age15_stay_sel = if cfg.age15_dropout_override == Some(false) {
        " selected"
    } else {
        ""
    };
    let age15_drop_sel = if cfg.age15_dropout_override == Some(true) {
        " selected"
    } else {
        ""
    };

    // Stage 4: Either Path 4A (High School) or Path 4B (Dropout / Street Gang)
    let stage4_html = if baseline.dropped_out_at_15 {
        let gang_cards: String = StreetGangArchetype::ALL
            .iter()
            .map(|&arch| {
                let checked = if cfg.stage4b_gang_archetype == arch {
                    " checked"
                } else {
                    ""
                };
                let code = arch.as_str();
                let radio_id = format!("wiz-gang-arch-{}", code.to_ascii_lowercase());
                format!(
                    r#"<label class="wiz-radio-card" for="{radio_id}">
                        <input
                            id="{radio_id}"
                            type="radio"
                            name="wiz_stage4b_archetype"
                            value="{code}"
                            {checked}
                            onchange="window.satyaWizardUpdateLifepathField('stage4b_archetype', '{code}')"
                        />
                        <div>
                            <strong>{label}</strong>
                            <div class="subtitle">{sub}</div>
                        </div>
                    </label>"#,
                    label = escape_html(arch.label()),
                    sub = escape_html(arch.subtitle()),
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
    } else {
        let alg_n = baseline
            .baseline_skills
            .get("elementary_algebra_geometry")
            .copied()
            .unwrap_or(0);
        let stem_qual = alg_n >= 2 && st.characteristics.ana >= 11 && st.characteristics.dis >= 11;
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
    };

    // Stage 5: Career Terms (0..=4 terms)
    let num_terms = cfg.career_terms.len().min(4);
    let num_terms_opts: String = (0..=4)
        .map(|n| {
            let age = 18 + n * 4;
            let sel = if n == num_terms { " selected" } else { "" };
            format!(
                r#"<option value="{n}"{sel}>{n} Term(s) — Starting Age {age} ({pts} Career + {elec} Elective Skill Levels)</option>"#,
                pts = n * 6,
                elec = 2 + n * 2
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
                        "Pairs [{}, {}] vs TN {} → {} Succ (+{} Skill Levels)",
                        e.pair_sums[0],
                        e.pair_sums[1],
                        e.tn_edu,
                        e.successes,
                        6 + usize::from(e.successes >= 2)
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
                <span class="pill pill-active">Age {age} · Career Budget: {c_bud} Levels · Elective: {e_bud} Levels</span>
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
                    <span class="roster-title">Unlocked [Access] Gates:</span> {access_badges}{freeze_badge}
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
                <div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <strong>Elementary Literacy (DIS/MEM + APT-LNG)</strong>
                        <span class="pill pill-active">{s2_lit_succ} Succ (TN {s2_lit_tn})</span>
                    </div>
                    <label for="wiz-s2-lit-ov">
                        <span>Pairs [{s2_l0}, {s2_l1}] · C_study = {s2_lit_c}</span>
                        <select
                            id="wiz-s2-lit-ov"
                            onchange="window.satyaWizardUpdateLifepathField('stage2_lit_override', this.value)"
                        >
                            {s2_lit_opts}
                        </select>
                    </label>
                </div>
                <div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <strong>Elementary Mathematics (DIS/ANA + APT-QNT)</strong>
                        <span class="pill pill-active">{s2_m_succ} Succ (TN {s2_m_tn})</span>
                    </div>
                    <label for="wiz-s2-math-ov">
                        <span>Pairs [{s2_m0}, {s2_m1}] · C_study = {s2_m_c}</span>
                        <select
                            id="wiz-s2-math-ov"
                            onchange="window.satyaWizardUpdateLifepathField('stage2_math_override', this.value)"
                        >
                            {s2_math_opts}
                        </select>
                    </label>
                </div>
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
                <div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <strong>Junior High Math (Algebra / Trig Gate)</strong>
                        <span class="pill pill-active">{s3_m_succ} Succ (TN {s3_m_tn})</span>
                    </div>
                    <label for="wiz-s3-math-ov">
                        <span>Pairs [{s3_m0}, {s3_m1}]</span>
                        <select
                            id="wiz-s3-math-ov"
                            onchange="window.satyaWizardUpdateLifepathField('stage3_math_override', this.value)"
                        >
                            {s3_math_opts}
                        </select>
                    </label>
                </div>
                <div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <strong>Junior High Science (Biology &amp; Tech)</strong>
                        <span class="pill pill-active">{s3_s_succ} Succ (TN {s3_s_tn})</span>
                    </div>
                    <label for="wiz-s3-sci-ov">
                        <span>Pairs [{s3_s0}, {s3_s1}]</span>
                        <select
                            id="wiz-s3-sci-ov"
                            onchange="window.satyaWizardUpdateLifepathField('stage3_sci_override', this.value)"
                        >
                            {s3_sci_opts}
                        </select>
                    </label>
                </div>
                <div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <strong>Junior High Humanities (History &amp; Writing)</strong>
                        <span class="pill pill-active">{s3_h_succ} Succ (TN {s3_h_tn})</span>
                    </div>
                    <label for="wiz-s3-hum-ov">
                        <span>Pairs [{s3_h0}, {s3_h1}]</span>
                        <select
                            id="wiz-s3-hum-ov"
                            onchange="window.satyaWizardUpdateLifepathField('stage3_hum_override', this.value)"
                        >
                            {s3_hum_opts}
                        </select>
                    </label>
                </div>
                <div class="wiz-char-assign-card">
                    <div class="wiz-char-header">
                        <strong>Age-15 Path (High School vs. Dropout)</strong>
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
                        </select>
                    </label>
                </div>
            </div>
        </fieldset>

        {stage4_html}

        <fieldset class="wiz-fieldset">
            <legend class="wiz-legend">
                <span>Stage 5 (Ages 18–34): Adult 4-Year Career Terms (1 to 4 Terms)</span>
                <span class="pill pill-active">{num_terms} Term(s) · +{c_bud} Career Levels · +{e_bud} Elective Levels</span>
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
        s2_lit_succ = baseline.stage2_lit_eval.successes,
        s2_lit_tn = baseline.stage2_lit_eval.tn_edu,
        s2_l0 = baseline.stage2_lit_eval.pair_sums[0],
        s2_l1 = baseline.stage2_lit_eval.pair_sums[1],
        s2_lit_c = baseline.stage2_lit_eval.c_study,
        s2_m_succ = baseline.stage2_math_eval.successes,
        s2_m_tn = baseline.stage2_math_eval.tn_edu,
        s2_m0 = baseline.stage2_math_eval.pair_sums[0],
        s2_m1 = baseline.stage2_math_eval.pair_sums[1],
        s2_m_c = baseline.stage2_math_eval.c_study,
        crisis_roll = cfg.age15_crisis_roll_2d20.clamp(2, 40),
        s3_m_succ = baseline.stage3_math_eval.successes,
        s3_m_tn = baseline.stage3_math_eval.tn_edu,
        s3_m0 = baseline.stage3_math_eval.pair_sums[0],
        s3_m1 = baseline.stage3_math_eval.pair_sums[1],
        s3_s_succ = baseline.stage3_sci_eval.successes,
        s3_s_tn = baseline.stage3_sci_eval.tn_edu,
        s3_s0 = baseline.stage3_sci_eval.pair_sums[0],
        s3_s1 = baseline.stage3_sci_eval.pair_sums[1],
        s3_h_succ = baseline.stage3_hum_eval.successes,
        s3_h_tn = baseline.stage3_hum_eval.tn_edu,
        s3_h0 = baseline.stage3_hum_eval.pair_sums[0],
        s3_h1 = baseline.stage3_hum_eval.pair_sums[1],
    )
}

/// Formats Step 5: 8-Tree Hierarchical Skill Builder & DAG Prerequisite Auditor.
fn format_skills_step_html(st: &CharacterWizardState) -> String {
    let baseline = st.compute_lifepath_baseline();
    let total_budget = baseline.career_skill_budget + baseline.elective_skill_budget;

    let mut career_spent = 0usize;
    let mut elective_spent = 0usize;
    let career_recommended: Vec<&'static str> = st
        .lifepath
        .career_terms
        .iter()
        .take(4)
        .flat_map(|t| t.track.recommended_skills().iter().copied())
        .collect();

    for (sk_id, &cur_n) in &st.skills {
        let floor_n = baseline.baseline_skills.get(sk_id).copied().unwrap_or(0);
        let added = cur_n.saturating_sub(floor_n);
        if added > 0 {
            if career_recommended.contains(&sk_id.as_str())
                && career_spent + added <= baseline.career_skill_budget
            {
                career_spent += added;
            } else if career_recommended.contains(&sk_id.as_str())
                && career_spent < baseline.career_skill_budget
            {
                let to_career = baseline.career_skill_budget - career_spent;
                career_spent += to_career;
                elective_spent += added - to_career;
            } else {
                elective_spent += added;
            }
        }
    }
    let total_spent = career_spent + elective_spent;
    let remaining_budget = total_budget.saturating_sub(total_spent);

    let tree_opts: String = std::iter::once((
        "ALL",
        "All 8 Hierarchical Skill Trees (88 Canonical Skills)",
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
                    let next_validation = validate_skill_advancement(
                        spec.id,
                        cur_n + 1,
                        &st.skills,
                        &st.characteristics,
                        &st.aptitudes,
                        &baseline.granted_access_tags,
                        baseline.academic_tree_frozen,
                    );
                    let within_term_cap =
                        gained_post_school < baseline.max_post_school_gain_per_skill && cur_n < 6;
                    let can_advance =
                        next_validation.is_valid && within_term_cap && remaining_budget > 0;
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
                    let is_career_rec = career_recommended.contains(&spec.id);

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

                    let status_note = if cur_n >= 1 {
                        let career_tag = if is_career_rec {
                            " · [Career Track]"
                        } else {
                            ""
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
                        } else {
                            r#"<span class="pill pill-active">Eligible (+1 Valid)</span>"#
                                .to_string()
                        }
                    } else {
                        let first_reason = next_validation
                            .failures
                            .first()
                            .map_or("Locked", |f| f.detail.as_str());
                        format!(
                            r#"<span class="pill pill-danger" title="{tip}">Locked: {code}</span>"#,
                            tip = escape_html(first_reason),
                            code = next_validation
                                .failures
                                .first()
                                .map_or("LOCKED", |f| f.code)
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
                                    <span class="wiz-char-code">T{tier} · {stype}</span>
                                    <strong class="wiz-char-name">{name}</strong>
                                </div>
                                <span class="stat-value">N = {cur_n}</span>
                            </div>
                            <div class="stat-note">
                                <strong>{ctrl_str} = {c_val}</strong> (Base TN {base_tn}) · <strong>{apt_str} {apt_val:+}</strong> · [{access_str}]
                            </div>
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
                                        "+1"
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
                        name = escape_html(spec.name),
                        apt_str = apt_code.as_str(),
                        sk_id = spec.id,
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
                <span>Step 5: 8-Tree Hierarchical Skill Builder &amp; Prerequisite Auditor</span>
                <span class="pill pill-active">Remaining Budget: {remaining_budget} Level(s) · Career {career_spent}/{c_bud} · Elective {elective_spent}/{e_bud}</span>
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
                        "Auto-Allocate Remaining ({remaining_budget}) Levels"
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
                Enforces all 4 canonical Satya DAG gates: <strong>1. [Access] Tag &amp; Academic Freeze</strong>, <strong>2. Min Characteristic</strong> (-1 relief if APT &gt;= +2), <strong>3. Parent Skill Level</strong>, and <strong>4. Foundation Ceiling (N_child &lt;= N_parent + 2)</strong>. Max post-school gain per skill: <strong>+{max_gain}</strong>.
            </div>
        </fieldset>
        {trees_html}"#,
        c_bud = baseline.career_skill_budget,
        e_bud = baseline.elective_skill_budget,
        max_gain = baseline.max_post_school_gain_per_skill,
    )
}

/// Formats Step 6: Complete Derived Characteristics, Lifepath Economy & Trained Skills Review.
fn format_summary_step_html(st: &CharacterWizardState) -> String {
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

    let trained_chips: String = CANONICAL_SKILL_TREES
        .iter()
        .filter_map(|spec| {
            let n = st.skills.get(spec.id).copied().unwrap_or(0);
            if n == 0 || (spec.skill_type == SkillType::Tal && n == 1) {
                return None;
            }
            let ctrl = c.compute_controller(spec.primary_char, spec.secondary_char);
            let escaped_name_js = spec.name.replace('\'', "\\'");
            Some(format!(
                r#"<button type="button" class="btn-chip" onclick="window.satyaLoadSkillPreset('{escaped_name_js}', {n}, {ctrl})">{name} (N={n}, C={ctrl})</button>"#,
                name = escape_html(spec.name),
            ))
        })
        .collect::<Vec<_>>()
        .join(" ");

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
                <div class="char-stat-box">
                    <span class="stat-label">Striking Power (POW)</span>
                    <span class="stat-value">{pow:.1}</span>
                    <span class="stat-note">(FOR {for_v} × SPD {spd}) / 10</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Momentum Factor (MMF)</span>
                    <span class="stat-value">{mmf:.1}</span>
                    <span class="stat-note">({mas:.1} kg / 70) × SPD {spd}</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Melee Reach Index</span>
                    <span class="stat-value">{reach:+}</span>
                    <span class="stat-note">HGT {hgt:.2}m ({h_edge:+}) + KIN {kin}</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Quick Thinking (D_cog)</span>
                    <span class="stat-value">{dcog}</span>
                    <span class="stat-note">floor((max(ANA {ana}, CRE {cre}) + FOC {foc})/2)</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">OODA Initiative</span>
                    <span class="stat-value">{ooda}</span>
                    <span class="stat-note">Start Tick = max(1, 15 - OODA)</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">General Awareness (AWR)</span>
                    <span class="stat-value">{awr}</span>
                    <span class="stat-note">floor((max(SIG {sig}, HEA {hea}) + FOC + ITU)/3)</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Fatigue Threshold (FT)</span>
                    <span class="stat-value">{ft}</span>
                    <span class="stat-note">FIT {fit} + floor(DIS {dis} / 2)</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Wound Shock Res (WSR)</span>
                    <span class="stat-value">{wsr}</span>
                    <span class="stat-note">HLT {hlt} + floor(DIS {dis} / 2)</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Social Composure (CMP)</span>
                    <span class="stat-value">{cmp}</span>
                    <span class="stat-note">floor((DIS {dis} + FOC {foc}) / 4)</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Lifepath Study (C_study)</span>
                    <span class="stat-value">Math {c_study_math} / Lit {c_study_lit}</span>
                    <span class="stat-note">CS {c_study_cmp} · Bio {c_study_bio} (Base {base_study})</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Campaign Upkeep (SQM {sqm_tier})</span>
                    <span class="stat-value">${sqm_usd}/mo</span>
                    <span class="stat-note">{sqm_name}</span>
                </div>
                <div class="char-stat-box">
                    <span class="stat-label">Operational Heat ({heat_score}/10)</span>
                    <span class="stat-value">+{heat_delta} Δ_ext</span>
                    <span class="stat-note">{heat_desc}</span>
                </div>
            </div>
            <div class="roster-bar">
                <span class="roster-title">Trained Skills &amp; Elevated Talents (Click to Load into 2d20 Task Roller):</span>
                <div class="preset-users">{trained_chips}</div>
            </div>
        </fieldset>"#,
        name = escape_html(&st.character_name),
        age = baseline.age_years,
        pow = derived.pow_striking_power,
        for_v = c.for_stat,
        spd = c.spd,
        mmf = derived.mmf_momentum_factor,
        mas = c.mas_kg,
        reach = derived.melee_reach_index,
        hgt = c.hgt_m,
        h_edge = derived.height_reach_edge,
        kin = c.kin,
        dcog = derived.d_cog_quick_thinking,
        ana = c.ana,
        cre = c.cre,
        foc = c.foc,
        ooda = derived.ooda_initiative,
        awr = derived.awr_general_awareness,
        sig = c.sig,
        hea = c.hea,
        ft = derived.ft_fatigue_threshold,
        fit = c.fit,
        dis = c.dis,
        wsr = derived.wsr_wound_shock_resistance,
        hlt = c.hlt,
        cmp = derived.cmp_social_composure_threshold,
        base_study = derived.base_c_study,
        sqm_tier = sqm_spec.sqm_tier,
        sqm_usd = sqm_spec.monthly_liquid_baseline_usd,
        sqm_name = escape_html(sqm_spec.tier_name),
        heat_score = heat_spec.heat_score,
        heat_delta = heat_spec.public_check_delta_ext,
        heat_desc = escape_html(heat_spec.description),
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

/// Re-renders `#wizard-stage-container` and `#wizard-status-pill` from thread-local state.
pub fn render_character_wizard() {
    let (html, pill_text) = with_state(|st| {
        let derived = st.wizard.characteristics.compute_derived(0);
        let baseline = st.wizard.compute_lifepath_baseline();
        let pill = format!(
            "Step {}/6 · Age {} · OODA {} · D_cog {} · POW {:.1}",
            st.wizard.active_step,
            baseline.age_years,
            derived.ooda_initiative,
            derived.d_cog_quick_thinking,
            derived.pow_striking_power
        );
        (format_character_wizard_html(&st.wizard), pill)
    });
    set_text("wizard-status-pill", &pill_text);
    if let Some(el) = get_element("wizard-stage-container") {
        el.set_inner_html(&html);
    }
}

/// Switches the active step (`1..=6`) in the Character Creation Wizard.
#[wasm_bindgen]
pub fn satya_wizard_set_step(step: u8) {
    let name_val = input_value("wiz-char-name-input");
    with_state_mut(|st| {
        if !name_val.trim().is_empty() {
            st.wizard.character_name = name_val.trim().to_string();
        }
        st.wizard.active_step = step.clamp(1, 6);
        st.wizard.sync_lifepath_skills();
    });
    render_character_wizard();
}

/// Updates the character name in [`CharacterWizardState`] when `#wiz-char-name-input` changes.
#[wasm_bindgen]
pub fn satya_wizard_name_changed() {
    let name_val = input_value("wiz-char-name-input");
    with_state_mut(|st| {
        if !name_val.trim().is_empty() {
            st.wizard.character_name = name_val.trim().to_string();
        }
    });
}

/// Switches between `METHOD_1` (`2d4 + 3d5` drop lowest `d5`) and `METHOD_2` (`2d4 + 2d5` keep highest).
#[wasm_bindgen]
pub fn satya_wizard_set_method(method_str: String) {
    let method = if method_str.trim().eq_ignore_ascii_case("METHOD_2") {
        CharacteristicRollMethod::Method2PoolKeepHighest
    } else {
        CharacteristicRollMethod::Method1DropLowestD5
    };
    with_state_mut(|st| {
        st.wizard.roll_method = method;
    });
    render_character_wizard();
}

fn roll_single_group_in_state(st: &mut CharacterWizardState, group_slug: &str) {
    let method = st.roll_method;
    match group_slug {
        "physical" => {
            let mut pool = roll_characteristic_group_pool(
                CharacteristicCategory::Physical,
                method,
                &mut browser_die_roller,
            );
            pool.kept_values.sort_by(|a, b| b.cmp(a));
            st.physical_pool = Some(pool);
            st.physical_slots = [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)];
        }
        "mental" => {
            let mut pool = roll_characteristic_group_pool(
                CharacteristicCategory::Mental,
                method,
                &mut browser_die_roller,
            );
            pool.kept_values.sort_by(|a, b| b.cmp(a));
            st.mental_pool = Some(pool);
            st.mental_slots = [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)];
        }
        "sensory" => {
            let mut pool = roll_characteristic_group_pool(
                CharacteristicCategory::Sensory,
                method,
                &mut browser_die_roller,
            );
            pool.kept_values.sort_by(|a, b| b.cmp(a));
            st.sensory_pool = Some(pool);
            st.sensory_slots = [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)];
        }
        "social" => {
            let mut pool = roll_characteristic_group_pool(
                CharacteristicCategory::Social,
                method,
                &mut browser_die_roller,
            );
            pool.kept_values.sort_by(|a, b| b.cmp(a));
            st.social_pool = Some(pool);
            st.social_slots = [Some(0), Some(1), Some(2)];
        }
        _ => {}
    }
    st.sync_from_slots();
}

/// Rolls a single characteristic group pool (`"physical"`, `"mental"`, `"sensory"`, or `"social"`).
#[wasm_bindgen]
pub fn satya_wizard_roll_group(group_slug: String) {
    let slug = group_slug.trim().to_ascii_lowercase();
    with_state_mut(|st| {
        roll_single_group_in_state(&mut st.wizard, &slug);
    });
    render_character_wizard();
    set_feedback(
        "wizard-feedback",
        &format!(
            "Rolled {slug} characteristic pool. Use the dropdowns to swap or assign scores to any characteristic."
        ),
        false,
    );
}

/// Rolls Physical, Correlated Height/Mass, Mental, Sensory, and Social pools all at once.
#[wasm_bindgen]
pub fn satya_wizard_roll_all_groups() {
    with_state_mut(|st| {
        roll_single_group_in_state(&mut st.wizard, "physical");
        roll_single_group_in_state(&mut st.wizard, "mental");
        roll_single_group_in_state(&mut st.wizard, "sensory");
        roll_single_group_in_state(&mut st.wizard, "social");
        st.wizard.height_mass_manual_override = false;
        let hm = roll_correlated_height_mass(
            st.wizard.characteristics.for_stat,
            &mut browser_die_roller,
        );
        st.wizard.characteristics.hgt_m = hm.height_m;
        st.wizard.characteristics.mas_kg = hm.mass_kg;
        st.wizard.height_mass_roll = Some(hm);
    });
    render_character_wizard();
    set_feedback(
        "wizard-feedback",
        "Rolled all Primary & Sensory characteristic pools plus correlated Height & Mass. Swap any slot assignments as desired.",
        false,
    );
}

/// Assigns kept slots `0..N` in order to all characteristics in `group_slug`.
#[wasm_bindgen]
pub fn satya_wizard_auto_assign_group(group_slug: String) {
    let slug = group_slug.trim().to_ascii_lowercase();
    with_state_mut(|st| {
        match slug.as_str() {
            "physical" => {
                st.wizard.physical_slots = [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)]
            }
            "mental" => {
                st.wizard.mental_slots = [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)]
            }
            "sensory" => {
                st.wizard.sensory_slots = [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)]
            }
            "social" => st.wizard.social_slots = [Some(0), Some(1), Some(2)],
            "aptitude" => {
                st.wizard.aptitude_slots = [
                    Some(0),
                    Some(1),
                    Some(2),
                    Some(3),
                    Some(4),
                    Some(5),
                    Some(6),
                    Some(7),
                    Some(8),
                ];
            }
            _ => {}
        }
        st.wizard.sync_from_slots();
    });
    render_character_wizard();
}

/// Assigns or swaps a rolled pool slot when the player changes a characteristic/aptitude `<select>`.
#[wasm_bindgen]
pub fn satya_wizard_assign_slot(group_slug: String, char_idx: usize) {
    let slug = group_slug.trim().to_ascii_lowercase();
    let select_id = match slug.as_str() {
        "physical" => CharacteristicCode::PHYSICAL_SCALED
            .get(char_idx)
            .map(|c| format!("wiz-slot-physical-{}", c.as_str().to_ascii_lowercase())),
        "mental" => CharacteristicCode::MENTAL_ALL
            .get(char_idx)
            .map(|c| format!("wiz-slot-mental-{}", c.as_str().to_ascii_lowercase())),
        "sensory" => CharacteristicCode::SENSORY_ALL
            .get(char_idx)
            .map(|c| format!("wiz-slot-sensory-{}", c.as_str().to_ascii_lowercase())),
        "social" => CharacteristicCode::SOCIAL_ALL
            .get(char_idx)
            .map(|c| format!("wiz-slot-social-{}", c.as_str().to_ascii_lowercase())),
        "psyche" => CharacteristicCode::PSYCHE_ALL
            .get(char_idx)
            .map(|c| format!("wiz-slot-psyche-{}", c.as_str().to_ascii_lowercase())),
        "aptitude" => AptitudeCode::ALL.get(char_idx).map(|c| {
            format!(
                "wiz-slot-aptitude-{}",
                c.as_str().to_ascii_lowercase().replace('-', "_")
            )
        }),
        _ => None,
    };
    let Some(sel_id) = select_id else {
        return;
    };
    let raw_val = select_value(&sel_id);
    let parsed_slot = raw_val.trim().parse::<usize>().ok();

    with_state_mut(|st| {
        match slug.as_str() {
            "physical" => assign_or_swap_slot(&mut st.wizard.physical_slots, char_idx, parsed_slot),
            "mental" => assign_or_swap_slot(&mut st.wizard.mental_slots, char_idx, parsed_slot),
            "sensory" => assign_or_swap_slot(&mut st.wizard.sensory_slots, char_idx, parsed_slot),
            "social" => assign_or_swap_slot(&mut st.wizard.social_slots, char_idx, parsed_slot),
            "psyche" => assign_or_swap_slot(&mut st.wizard.psyche_slots, char_idx, parsed_slot),
            "aptitude" => assign_or_swap_slot(&mut st.wizard.aptitude_slots, char_idx, parsed_slot),
            _ => {}
        }
        st.wizard.sync_from_slots();
    });
    render_character_wizard();
}

/// Rolls correlated Height (`m`) and Mass (`kg`) around the mean (`1.72 m`, BMI `23.0` adjusted by Build & `FOR`).
#[wasm_bindgen]
pub fn satya_wizard_roll_height_mass() {
    with_state_mut(|st| {
        st.wizard.height_mass_manual_override = false;
        let hm = roll_correlated_height_mass(
            st.wizard.characteristics.for_stat,
            &mut browser_die_roller,
        );
        st.wizard.characteristics.hgt_m = hm.height_m;
        st.wizard.characteristics.mas_kg = hm.mass_kg;
        st.wizard.height_mass_roll = Some(hm);
    });
    render_character_wizard();
    set_feedback(
        "wizard-feedback",
        "Rolled correlated Height (m) and Build/Force-adjusted Mass (kg).",
        false,
    );
}

/// Applies manual Height (`m`) and Mass (`kg`) edits from `#wiz-input-hgt-m` and `#wiz-input-mas-kg`.
#[wasm_bindgen]
pub fn satya_wizard_update_height_mass() {
    let hgt_val = input_value("wiz-input-hgt-m")
        .trim()
        .parse::<f64>()
        .unwrap_or(1.75)
        .clamp(1.40, 2.20);
    let mas_val = input_value("wiz-input-mas-kg")
        .trim()
        .parse::<f64>()
        .unwrap_or(70.0)
        .clamp(40.0, 160.0);
    with_state_mut(|st| {
        st.wizard.height_mass_manual_override = true;
        st.wizard.characteristics.hgt_m = hgt_val;
        st.wizard.characteristics.mas_kg = mas_val;
    });
    render_character_wizard();
}

/// Rolls a 7-value `2d4 + 2d5` pool for the 7 Psyche characteristics and assigns them in order (swap-enabled).
#[wasm_bindgen]
pub fn satya_wizard_roll_psyche_pool() {
    with_state_mut(|st| {
        let mut rolls: Vec<SingleCharacteristicRoll> = (0..7)
            .map(|_| roll_characteristic_2d4_2d5(&mut browser_die_roller))
            .collect();
        rolls.sort_by_key(|a| std::cmp::Reverse(a.total));
        st.wizard.psyche_pool = Some(rolls);
        st.wizard.psyche_slots = [
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
        ];
        st.wizard.psyche_mode = "ROLL_POOL".to_string();
        st.wizard.sync_from_slots();
    });
    render_character_wizard();
    set_feedback(
        "wizard-feedback",
        "Rolled 7 × (2d4 + 2d5) Psyche scores. Swap pool slots or drag any slider (1–20) to customize.",
        false,
    );
}

/// Resets all 7 Psyche characteristics to Balanced (`10`).
#[wasm_bindgen]
pub fn satya_wizard_reset_psyche_balanced() {
    with_state_mut(|st| {
        st.wizard.psyche_mode = "CHOOSE".to_string();
        st.wizard.psyche_slots = [None; 7];
        for code in CharacteristicCode::PSYCHE_ALL {
            st.wizard.characteristics.set_scale(code, 10);
        }
    });
    render_character_wizard();
}

/// Sets a specific Psyche characteristic (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`) to a player-chosen value (`1–20`).
#[wasm_bindgen]
pub fn satya_wizard_set_psyche(code_str: String, raw_value: String) {
    let Some(code) = CharacteristicCode::from_code_str(&code_str) else {
        return;
    };
    let val = raw_value.trim().parse::<i32>().unwrap_or(10).clamp(1, 20);
    with_state_mut(|st| {
        if let Some(p_idx) = CharacteristicCode::PSYCHE_ALL
            .iter()
            .position(|&c| c == code)
        {
            st.wizard.psyche_slots[p_idx] = None;
        }
        st.wizard.characteristics.set_scale(code, val);
    });
    render_character_wizard();
}

/// Rolls the `11 × 3d6 (drop 2 lowest -> 9 kept)` Domain Aptitude pool and assigns slots `0..9`.
#[wasm_bindgen]
pub fn satya_wizard_roll_aptitudes() {
    with_state_mut(|st| {
        let mut pool = roll_aptitude_pool_11_drop_2(&mut browser_die_roller);
        pool.kept_values.sort_by(|a, b| b.cmp(a));
        st.wizard.aptitude_pool = Some(pool);
        st.wizard.aptitude_slots = [
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            Some(8),
        ];
        st.wizard.sync_from_slots();
    });
    render_character_wizard();
    set_feedback(
        "wizard-feedback",
        "Rolled 11 × 3d6 Domain Aptitudes and dropped the 2 lowest. Assign the 9 kept scores across the 9 domains.",
        false,
    );
}

/// Rolls all 5 Lifepath stages (`Stages 1–5`) and auto-allocates Career & Elective skills across the 8 trees.
#[wasm_bindgen]
pub fn satya_wizard_roll_complete_lifepath() {
    with_state_mut(|st| {
        let lp = &mut st.wizard.lifepath;
        lp.origin_roll_2d20 = browser_die_roller(20) + browser_die_roller(20);
        let orig = resolve_socioeconomic_origin(lp.origin_roll_2d20);
        if let Some(&first_tal) = orig.early_environment_bonus_talents.first() {
            lp.stage1_early_talent = first_tal.to_string();
        }
        lp.stage2_lit_pairs = roll_2d20_pairs();
        lp.stage2_lit_override = None;
        lp.stage2_math_pairs = roll_2d20_pairs();
        lp.stage2_math_override = None;
        lp.stage3_math_pairs = roll_2d20_pairs();
        lp.stage3_math_override = None;
        lp.stage3_sci_pairs = roll_2d20_pairs();
        lp.stage3_sci_override = None;
        lp.stage3_hum_pairs = roll_2d20_pairs();
        lp.stage3_hum_override = None;
        lp.age15_crisis_roll_2d20 = browser_die_roller(20) + browser_die_roller(20);
        lp.age15_dropout_override = None;
        lp.stage4a_hs_pairs = roll_2d20_pairs();
        lp.stage4a_hs_override = None;
        for term in &mut lp.career_terms {
            term.term_roll_pairs = roll_2d20_pairs();
            term.override_successes = None;
        }
        st.wizard.rebuild_and_auto_allocate_skills();
    });
    render_character_wizard();
    set_feedback(
        "wizard-feedback",
        "Rolled complete 5-Stage Lifepath (Origin, Elementary, Junior High, Age-15 Crisis, High School/Dropout & Career Terms) and auto-allocated Career/Elective skills.",
        false,
    );
}

/// Rolls a single Lifepath stage (`1` = Origin, `2` = Elementary, `3` = Junior High & Age-15 Crisis, `4` = High School, `5` = Career Terms).
#[wasm_bindgen]
pub fn satya_wizard_roll_lifepath_stage(stage_num: u8) {
    with_state_mut(|st| {
        let lp = &mut st.wizard.lifepath;
        match stage_num {
            1 => {
                lp.origin_roll_2d20 = browser_die_roller(20) + browser_die_roller(20);
                let orig = resolve_socioeconomic_origin(lp.origin_roll_2d20);
                if let Some(&first_tal) = orig.early_environment_bonus_talents.first() {
                    lp.stage1_early_talent = first_tal.to_string();
                }
            }
            2 => {
                lp.stage2_lit_pairs = roll_2d20_pairs();
                lp.stage2_lit_override = None;
                lp.stage2_math_pairs = roll_2d20_pairs();
                lp.stage2_math_override = None;
            }
            3 => {
                lp.stage3_math_pairs = roll_2d20_pairs();
                lp.stage3_math_override = None;
                lp.stage3_sci_pairs = roll_2d20_pairs();
                lp.stage3_sci_override = None;
                lp.stage3_hum_pairs = roll_2d20_pairs();
                lp.stage3_hum_override = None;
                lp.age15_crisis_roll_2d20 = browser_die_roller(20) + browser_die_roller(20);
                lp.age15_dropout_override = None;
            }
            4 => {
                lp.stage4a_hs_pairs = roll_2d20_pairs();
                lp.stage4a_hs_override = None;
            }
            5 => {
                for term in &mut lp.career_terms {
                    term.term_roll_pairs = roll_2d20_pairs();
                    term.override_successes = None;
                }
            }
            _ => {}
        }
        st.wizard.rebuild_and_auto_allocate_skills();
    });
    render_character_wizard();
    set_feedback(
        "wizard-feedback",
        &format!("Rolled Lifepath Stage {stage_num} and updated baseline skills."),
        false,
    );
}

/// Updates a field or override in [`LifepathBuildConfig`] and syncs baseline skills.
#[wasm_bindgen]
pub fn satya_wizard_update_lifepath_field(field_name: String, raw_value: String) {
    let field = field_name.trim().to_ascii_lowercase();
    let val = raw_value.trim();
    let parsed_ov = val.parse::<usize>().ok().map(|n| n.min(2));

    with_state_mut(|st| {
        let lp = &mut st.wizard.lifepath;
        match field.as_str() {
            "origin_class" => {
                if let Some(cls) = SocioeconomicClass::from_code_str(val) {
                    lp.origin_roll_2d20 = cls.representative_roll_2d20();
                    let orig = resolve_socioeconomic_origin(lp.origin_roll_2d20);
                    if let Some(&first_tal) = orig.early_environment_bonus_talents.first() {
                        lp.stage1_early_talent = first_tal.to_string();
                    }
                }
            }
            "stage1_talent" => {
                if lookup_skill_node(val).is_some() {
                    lp.stage1_early_talent = val.to_string();
                }
            }
            "stage2_lit_override" => lp.stage2_lit_override = parsed_ov,
            "stage2_math_override" => lp.stage2_math_override = parsed_ov,
            "stage2_talent" => {
                if lookup_skill_node(val).is_some() {
                    lp.stage2_playground_talent = val.to_string();
                }
            }
            "stage3_math_override" => lp.stage3_math_override = parsed_ov,
            "stage3_sci_override" => lp.stage3_sci_override = parsed_ov,
            "stage3_hum_override" => lp.stage3_hum_override = parsed_ov,
            "age15_path" => {
                lp.age15_dropout_override = match val.to_ascii_uppercase().as_str() {
                    "STAY_4A" => Some(false),
                    "DROPOUT_4B" => Some(true),
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
            "num_career_terms" => {
                let target_len = val.parse::<usize>().unwrap_or(1).min(4);
                while lp.career_terms.len() < target_len {
                    lp.career_terms.push(CareerTermConfig {
                        track: CareerTermTrack::MilitaryCombatArmsSpecOps,
                        term_roll_pairs: roll_2d20_pairs(),
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
        st.wizard.rebuild_and_auto_allocate_skills();
    });
    render_character_wizard();
}

/// Updates a specific Stage 5 Career Term (`0..4`) track, roll, or override.
#[wasm_bindgen]
pub fn satya_wizard_update_career_term(term_idx: usize, field: String, raw_value: String) {
    let f = field.trim().to_ascii_lowercase();
    let v = raw_value.trim();
    with_state_mut(|st| {
        if let Some(term) = st.wizard.lifepath.career_terms.get_mut(term_idx) {
            match f.as_str() {
                "track" => {
                    if let Some(trk) = CareerTermTrack::from_code_str(v) {
                        term.track = trk;
                    }
                }
                "override_successes" => {
                    term.override_successes = v.parse::<usize>().ok().map(|n| n.min(2));
                }
                "roll" => {
                    term.term_roll_pairs = roll_2d20_pairs();
                    term.override_successes = None;
                }
                _ => {}
            }
        }
        st.wizard.rebuild_and_auto_allocate_skills();
    });
    render_character_wizard();
}

/// Updates the Step 5 Skill Tree filter (`"ALL"` or `"TREE_1"` .. `"TREE_8"`) and view mode (`"ALL"`, `"TRAINED"`, `"UNLOCKABLE"`).
#[wasm_bindgen]
pub fn satya_wizard_set_skill_filter(tree_code: String, view_mode: String) {
    with_state_mut(|st| {
        st.wizard.skill_tree_filter = tree_code.trim().to_ascii_uppercase();
        st.wizard.skill_view_mode = view_mode.trim().to_ascii_uppercase();
    });
    render_character_wizard();
}

/// Adjusts a single skill level by `delta` (`+1` or `-1`) in Step 5, enforcing all 4 DAG gates and the Stage 1–4 school floor.
#[wasm_bindgen]
pub fn satya_wizard_adjust_skill(skill_id: String, delta: i32) {
    let sk = skill_id.trim();
    let mut feedback_msg: Option<(String, bool)> = None;

    with_state_mut(|st| {
        let baseline = st.wizard.compute_lifepath_baseline();
        let cur_n = st.wizard.skills.get(sk).copied().unwrap_or(0);
        let floor_n = baseline.baseline_skills.get(sk).copied().unwrap_or(0);

        if delta < 0 {
            if cur_n > floor_n {
                let new_n = cur_n - 1;
                if new_n == 0 {
                    st.wizard.skills.remove(sk);
                } else {
                    st.wizard.skills.insert(sk.to_string(), new_n);
                }
                feedback_msg = Some((format!("Reduced '{sk}' to N={new_n}."), false));
            } else {
                feedback_msg = Some((
                    format!("Cannot reduce '{sk}' below Stage 1–4 Lifepath floor (N={floor_n})."),
                    true,
                ));
            }
        } else if delta > 0 {
            let total_budget = baseline.career_skill_budget + baseline.elective_skill_budget;
            let total_spent: usize = st
                .wizard
                .skills
                .iter()
                .map(|(k, &v)| {
                    let fl = baseline.baseline_skills.get(k).copied().unwrap_or(0);
                    v.saturating_sub(fl)
                })
                .sum();
            let gained_post_school = cur_n.saturating_sub(floor_n);
            if total_spent >= total_budget {
                feedback_msg = Some((
                    "No remaining Career or Elective skill level budget. Reduce another skill or add a Career Term.".to_string(),
                    true,
                ));
            } else if gained_post_school >= baseline.max_post_school_gain_per_skill {
                feedback_msg = Some((
                    format!(
                        "Skill '{sk}' has reached the maximum post-school gain cap (+{} levels).",
                        baseline.max_post_school_gain_per_skill
                    ),
                    true,
                ));
            } else {
                let target_n = cur_n + 1;
                let val = validate_skill_advancement(
                    sk,
                    target_n,
                    &st.wizard.skills,
                    &st.wizard.characteristics,
                    &st.wizard.aptitudes,
                    &baseline.granted_access_tags,
                    baseline.academic_tree_frozen,
                );
                if val.is_valid {
                    st.wizard.skills.insert(sk.to_string(), target_n);
                    feedback_msg = Some((
                        format!("Advanced '{}' to N={target_n}.", val.skill_name),
                        false,
                    ));
                } else {
                    let err_str = val
                        .failures
                        .iter()
                        .map(|f| format!("{}: {}", f.code, f.detail))
                        .collect::<Vec<_>>()
                        .join(" | ");
                    feedback_msg = Some((err_str, true));
                }
            }
        }
    });

    render_character_wizard();
    if let Some((msg, is_err)) = feedback_msg {
        set_feedback("wizard-feedback", &msg, is_err);
    }
}

/// Auto-allocates all remaining Career & Elective skill budget across eligible career and staple skills.
#[wasm_bindgen]
pub fn satya_wizard_auto_allocate_skills() {
    with_state_mut(|st| {
        let baseline = st.wizard.compute_lifepath_baseline();
        auto_allocate_career_and_elective_skills(
            &baseline,
            &st.wizard.lifepath,
            &st.wizard.characteristics,
            &st.wizard.aptitudes,
            &mut st.wizard.skills,
        );
    });
    render_character_wizard();
    set_feedback(
        "wizard-feedback",
        "Auto-allocated remaining Career and Elective skill levels across unlocked 8-Tree nodes.",
        false,
    );
}

/// Resets post-school skill allocations back to the Stage 1–4 Lifepath baseline so the player can spend Career/Elective levels manually.
#[wasm_bindgen]
pub fn satya_wizard_reset_post_school_skills() {
    with_state_mut(|st| {
        let baseline = st.wizard.compute_lifepath_baseline();
        st.wizard.skills = baseline.baseline_skills;
    });
    render_character_wizard();
    set_feedback(
        "wizard-feedback",
        "Reset skills to Stage 1–4 school/dropout baseline. You can now allocate all Career and Elective levels manually.",
        false,
    );
}

/// Applies the wizard's generated `CharacterCharacteristics`, `CharacterAptitudes`, Lifepath, and Trained 8-Tree Skills
/// to the active `Character Sheet/Status` notebook tab and the Center-Bottom `2d20` Task Roller.
#[wasm_bindgen]
pub fn satya_wizard_apply_to_sheet() {
    let name_val = input_value("wiz-char-name-input");
    let (name, chars, derived, baseline, skills) = with_state_mut(|st| {
        if !name_val.trim().is_empty() {
            st.wizard.character_name = name_val.trim().to_string();
        }
        st.wizard.sync_lifepath_skills();
        let c = st.wizard.characteristics.clone();
        let d = c.compute_derived(0);
        let b = st.wizard.compute_lifepath_baseline();
        let s = st.wizard.skills.clone();
        (st.wizard.character_name.clone(), c, d, b, s)
    });

    let control_dex_tac =
        chars.compute_controller(CharacteristicCode::DEX, Some(CharacteristicCode::TAC));
    set_text("char-stat-control", &control_dex_tac.to_string());
    set_text("char-stat-spd", &chars.spd.to_string());
    set_text("char-stat-kin", &chars.kin.to_string());
    set_text("char-stat-ana", &chars.ana.to_string());
    set_text("char-stat-dis", &chars.dis.to_string());
    set_text(
        "char-sheet-status-pill",
        &format!(
            "{} (Age {}) · OODA {} · D_cog {} · FT {} · WSR {} · POW {:.1} · SQM {}",
            name,
            baseline.age_years,
            derived.ooda_initiative,
            derived.d_cog_quick_thinking,
            derived.ft_fatigue_threshold,
            derived.wsr_wound_shock_resistance,
            derived.pow_striking_power,
            baseline.campaign_sqm
        ),
    );

    // Sync trained skills into #char-sheet-skills-chips on the Character Sheet tab
    if let Some(chips_el) = get_element("char-sheet-skills-chips") {
        let chips_html: String = CANONICAL_SKILL_TREES
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
            .join(" ");
        if !chips_html.is_empty() {
            chips_el.set_inner_html(&chips_html);
        }
    }

    // Sync Lifepath, [Access] Gates & SQM/Heat into #char-sheet-lifepath-summary
    if let Some(lp_el) = get_element("char-sheet-lifepath-summary") {
        let sqm_spec = lookup_campaign_sqm(baseline.campaign_sqm);
        let heat_spec = evaluate_operational_heat(baseline.operational_heat);
        let access_pills: String = baseline
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
        let freeze_pill = if baseline.academic_tree_frozen {
            r#" <span class="pill pill-danger">[Academic Tree FROZEN]</span>"#
        } else {
            ""
        };
        lp_el.set_inner_html(&format!(
            r#"<span class="roster-title">Lifepath, [Access] Gates &amp; Economy:</span>
            <div class="roster-chips">
                <span class="pill pill-iggy">Origin: {orig}</span>
                <span class="pill pill-iggy">SQM {sqm}: ${usd}/mo ({tier_name})</span>
                <span class="pill pill-iggy">Heat {heat}/10 (+{h_delta} Δ_ext)</span>
                {access_pills}{freeze_pill}
            </div>"#,
            orig = escape_html(baseline.origin.label),
            sqm = sqm_spec.sqm_tier,
            usd = sqm_spec.monthly_liquid_baseline_usd,
            tier_name = escape_html(sqm_spec.tier_name),
            heat = heat_spec.heat_score,
            h_delta = heat_spec.public_check_delta_ext,
        ));
    }

    set_input_value("ttrpg-control-c", &control_dex_tac.to_string());
    satya_update_cascade_preview();

    let trained_count = skills.values().filter(|&&n| n >= 2).count();
    let notice = format!(
        "Applied character '{}' (Age {}, {}, SQM {}, {} skills at N>=2) to Character Sheet (SPD {}, FOR {}, DEX {}, KIN {}, HLT {}, FIT {}, {:.2}m / {:.1}kg, ANA {}, DIS {}, OODA {}, D_cog {}).",
        name,
        baseline.age_years,
        baseline.origin.label,
        baseline.campaign_sqm,
        trained_count,
        chars.spd,
        chars.for_stat,
        chars.dex,
        chars.kin,
        chars.hlt,
        chars.fit,
        chars.hgt_m,
        chars.mas_kg,
        chars.ana,
        chars.dis,
        derived.ooda_initiative,
        derived.d_cog_quick_thinking
    );
    set_feedback("wizard-feedback", &notice, false);
    append_system_notice(&notice);
}
