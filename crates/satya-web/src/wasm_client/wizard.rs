//! Step-by-Step Character Creation Wizard (`#notebook-pane-wizard`) state and `#[wasm_bindgen]` handlers.
//!
//! HTML rendering for the 6 wizard steps is modularized into:
//! - [`crate::wasm_client::wizard_views_stats`] (Steps 1–3: Primary/Sensory Pools, Height/Mass, Psyche & Aptitudes)
//! - [`crate::wasm_client::wizard_views_lifepath`] (Step 4: Chronological 5-Stage Lifepath)
//! - [`crate::wasm_client::wizard_views_skills`] (Steps 5–6: 8-Tree Skill Builder, 5-Factor SP Cost & Derived Summary)

use std::collections::BTreeMap;

use crate::wasm_client::chat_and_ttrpg::{append_system_notice, satya_update_cascade_preview};
use crate::wasm_client::dom::{
    get_element, input_value, roll_browser_die_i32, select_value, set_feedback, set_input_value,
    set_text,
};
use crate::wasm_client::state::{with_state, with_state_mut};
pub use crate::wasm_client::wizard_views_lifepath::*;
pub use crate::wasm_client::wizard_views_skills::*;
pub use crate::wasm_client::wizard_views_stats::*;
use satya_engine::{
    AptitudeCode, AptitudePoolRoll, CharacterAptitudes, CharacterCharacteristics,
    CharacteristicCategory, CharacteristicCode, CharacteristicGroupPool, CharacteristicRollMethod,
    CorrelatedHeightMassRoll, LifepathBaselineResult, LifepathBuildConfig,
    SingleCharacteristicRoll, auto_allocate_career_and_elective_skills,
    compute_correlated_height_mass, compute_lifepath_baseline_from_config,
    compute_skill_allocation_sp_ledger, lookup_skill_node, preview_next_skill_step_sp,
    roll_aptitude_pool_11_drop_2, roll_characteristic_2d4_2d5, roll_characteristic_group_pool,
    roll_correlated_height_mass, validate_skill_advancement,
};
use wasm_bindgen::prelude::*;

/// Client-side state for the step-by-step Character Creation Wizard tab (`#notebook-pane-wizard`).
#[derive(Debug, Clone)]
pub struct CharacterWizardState {
    /// Active wizard step (`1` = Primary/Sensory .. `6` = Review & Apply).
    pub active_step: u8,
    pub character_name: String,
    pub roll_method: CharacteristicRollMethod,
    pub physical_pool: Option<CharacteristicGroupPool>,
    pub physical_slots: [Option<usize>; 6],
    pub height_mass_roll: Option<CorrelatedHeightMassRoll>,
    pub height_mass_manual_override: bool,
    pub mental_pool: Option<CharacteristicGroupPool>,
    pub mental_slots: [Option<usize>; 6],
    pub sensory_pool: Option<CharacteristicGroupPool>,
    pub sensory_slots: [Option<usize>; 6],
    pub social_pool: Option<CharacteristicGroupPool>,
    pub social_slots: [Option<usize>; 3],
    /// Psyche generation mode (`"CHOOSE"`, `"ROLL_DIRECT"`, or `"ROLL_POOL"`).
    pub psyche_mode: String,
    pub psyche_pool: Option<Vec<SingleCharacteristicRoll>>,
    pub psyche_slots: [Option<usize>; 7],
    pub aptitude_pool: Option<AptitudePoolRoll>,
    pub aptitude_slots: [Option<usize>; 9],
    pub characteristics: CharacterCharacteristics,
    pub aptitudes: CharacterAptitudes,
    pub lifepath: LifepathBuildConfig,
    pub skills: BTreeMap<String, usize>,
    pub skill_tree_filter: String,
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

    /// Returns a mutable slice of slot assignments for `group_slug` (`"physical"`, `"mental"`, `"sensory"`, `"social"`, `"psyche"`, `"aptitude"`).
    pub(crate) fn slots_mut(&mut self, group_slug: &str) -> Option<&mut [Option<usize>]> {
        match group_slug {
            "physical" => Some(&mut self.physical_slots),
            "mental" => Some(&mut self.mental_slots),
            "sensory" => Some(&mut self.sensory_slots),
            "social" => Some(&mut self.social_slots),
            "psyche" => Some(&mut self.psyche_slots),
            "aptitude" => Some(&mut self.aptitude_slots),
            _ => None,
        }
    }

    /// Assigns sequential slots `0..len` to `group_slug`.
    pub(crate) fn assign_slots_in_order(&mut self, group_slug: &str) {
        if let Some(slots) = self.slots_mut(group_slug) {
            for (i, slot) in slots.iter_mut().enumerate() {
                *slot = Some(i);
            }
        }
    }

    /// Synchronizes `self.characteristics` and `self.aptitudes` from currently assigned pool slots.
    pub fn sync_from_slots(&mut self) {
        for (pool_opt, slots, codes) in [
            (
                self.physical_pool.as_ref(),
                &self.physical_slots[..],
                &CharacteristicCode::PHYSICAL_SCALED[..],
            ),
            (
                self.mental_pool.as_ref(),
                &self.mental_slots[..],
                &CharacteristicCode::MENTAL_ALL[..],
            ),
            (
                self.sensory_pool.as_ref(),
                &self.sensory_slots[..],
                &CharacteristicCode::SENSORY_ALL[..],
            ),
            (
                self.social_pool.as_ref(),
                &self.social_slots[..],
                &CharacteristicCode::SOCIAL_ALL[..],
            ),
        ] {
            if let Some(pool) = pool_opt {
                for (idx, &code) in codes.iter().enumerate() {
                    if let Some(slot) = slots[idx]
                        && let Some(&val) = pool.kept_values.get(slot)
                    {
                        self.characteristics.set_scale(code, val);
                    }
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

    /// Assigns or swaps `new_slot` for `char_idx` within `group_slug` and synchronizes characteristics/aptitudes.
    pub fn assign_slot(&mut self, group_slug: &str, char_idx: usize, new_slot: Option<usize>) {
        if let Some(slots) = self.slots_mut(group_slug) {
            assign_or_swap_slot(slots, char_idx, new_slot);
        }
        self.sync_from_slots();
    }

    /// Applies a manual Height (`1.40..=2.20 m`) and Mass (`40.0..=160.0 kg`) override.
    pub fn set_manual_height_mass(&mut self, hgt_m: f64, mas_kg: f64) {
        self.height_mass_manual_override = true;
        self.characteristics.hgt_m = hgt_m.clamp(1.40, 2.20);
        self.characteristics.mas_kg = mas_kg.clamp(40.0, 160.0);
    }

    /// Rolls a single characteristic group pool (`"physical"`, `"mental"`, `"sensory"`, or `"social"`) using `rng`,
    /// sorts kept scores descending, assigns sequential slots, and synchronizes characteristics.
    pub fn roll_group_with<F: FnMut(i32) -> i32>(&mut self, group_slug: &str, rng: &mut F) {
        let cat = match group_slug {
            "physical" => CharacteristicCategory::Physical,
            "mental" => CharacteristicCategory::Mental,
            "sensory" => CharacteristicCategory::Sensory,
            "social" => CharacteristicCategory::Social,
            _ => return,
        };
        let mut pool = roll_characteristic_group_pool(cat, self.roll_method, rng);
        pool.kept_values.sort_by(|a, b| b.cmp(a));
        match group_slug {
            "physical" => self.physical_pool = Some(pool),
            "mental" => self.mental_pool = Some(pool),
            "sensory" => self.sensory_pool = Some(pool),
            "social" => self.social_pool = Some(pool),
            _ => {}
        }
        self.assign_slots_in_order(group_slug);
        self.sync_from_slots();
    }

    /// Rolls correlated Height (`m`) and Mass (`kg`) using `rng`, clearing any manual override.
    pub fn roll_height_mass_with<F: FnMut(i32) -> i32>(&mut self, rng: &mut F) {
        self.height_mass_manual_override = false;
        let hm = roll_correlated_height_mass(self.characteristics.for_stat, rng);
        self.characteristics.hgt_m = hm.height_m;
        self.characteristics.mas_kg = hm.mass_kg;
        self.height_mass_roll = Some(hm);
    }

    /// Pure, host-testable Step 5 skill adjustment (`delta < 0` reduces by 1 down to Stage 1–4 school floor;
    /// `delta > 0` advances by 1 after validating `N <= 6`, all 4 DAG prerequisite gates, post-school term cap,
    /// and 5-factor Skill Point affordability).
    pub fn try_adjust_skill(&mut self, skill_id: &str, delta: i32) -> Result<usize, String> {
        let sk = skill_id.trim();
        if lookup_skill_node(sk).is_none() {
            return Err(format!("Unknown skill '{sk}'."));
        }
        let baseline = self.compute_lifepath_baseline();
        let cur_n = self.skills.get(sk).copied().unwrap_or(0);
        let floor_n = baseline.baseline_skills.get(sk).copied().unwrap_or(0);

        if delta == 0 {
            return Ok(cur_n);
        }
        if delta < 0 {
            if cur_n <= floor_n {
                return Err(format!(
                    "Cannot reduce '{sk}' below Stage 1–4 Lifepath floor (N={floor_n})."
                ));
            }
            let new_n = cur_n - 1;
            if new_n == 0 {
                self.skills.remove(sk);
            } else {
                self.skills.insert(sk.to_string(), new_n);
            }
            return Ok(new_n);
        }

        let target_n = cur_n + 1;
        if target_n > 6 {
            return Err(format!("Skill '{sk}' cannot exceed maximum level N=6."));
        }
        let val = validate_skill_advancement(
            sk,
            target_n,
            &self.skills,
            &self.characteristics,
            &self.aptitudes,
            &baseline.granted_access_tags,
            baseline.academic_tree_frozen,
        );
        if !val.is_valid {
            let err_str = val
                .failures
                .iter()
                .map(|f| format!("{}: {}", f.code, f.detail))
                .collect::<Vec<_>>()
                .join(" | ");
            return Err(err_str);
        }
        let gained_post_school = cur_n.saturating_sub(floor_n);
        if gained_post_school >= baseline.max_post_school_gain_per_skill {
            return Err(format!(
                "Skill '{sk}' has reached the maximum post-school gain cap (+{} levels).",
                baseline.max_post_school_gain_per_skill
            ));
        }
        let ledger = compute_skill_allocation_sp_ledger(
            &baseline,
            &self.lifepath,
            &self.characteristics,
            &self.aptitudes,
            &self.skills,
        );
        let (step_bd, sp_affordable) = preview_next_skill_step_sp(
            sk,
            target_n,
            &self.lifepath,
            &ledger,
            &self.characteristics,
            &self.aptitudes,
        );
        if !sp_affordable {
            return Err(format!(
                "Advancing '{sk}' to N={target_n} costs {} SP ({}), but only {} SP remains (Formal {} SP, Experience {} SP).",
                step_bd.final_sp_cost,
                step_bd.training_mode.as_label(),
                ledger.total_sp_remaining,
                ledger.formal_sp_remaining,
                ledger.experience_sp_remaining
            ));
        }
        self.skills.insert(sk.to_string(), target_n);
        Ok(target_n)
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

/// Rolls a single characteristic group pool (`"physical"`, `"mental"`, `"sensory"`, or `"social"`).
#[wasm_bindgen]
pub fn satya_wizard_roll_group(group_slug: String) {
    let slug = group_slug.trim().to_ascii_lowercase();
    with_state_mut(|st| {
        st.wizard.roll_group_with(&slug, &mut roll_browser_die_i32);
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
        for slug in ["physical", "mental", "sensory", "social"] {
            st.wizard.roll_group_with(slug, &mut roll_browser_die_i32);
        }
        st.wizard.roll_height_mass_with(&mut roll_browser_die_i32);
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
        st.wizard.assign_slots_in_order(&slug);
        st.wizard.sync_from_slots();
    });
    render_character_wizard();
}

/// Assigns or swaps a rolled pool slot when the player changes a characteristic/aptitude `<select>`.
#[wasm_bindgen]
pub fn satya_wizard_assign_slot(group_slug: String, char_idx: usize) {
    let slug = group_slug.trim().to_ascii_lowercase();
    let code_opt = match slug.as_str() {
        "physical" => CharacteristicCode::PHYSICAL_SCALED
            .get(char_idx)
            .map(|c| c.as_str()),
        "mental" => CharacteristicCode::MENTAL_ALL
            .get(char_idx)
            .map(|c| c.as_str()),
        "sensory" => CharacteristicCode::SENSORY_ALL
            .get(char_idx)
            .map(|c| c.as_str()),
        "social" => CharacteristicCode::SOCIAL_ALL
            .get(char_idx)
            .map(|c| c.as_str()),
        "psyche" => CharacteristicCode::PSYCHE_ALL
            .get(char_idx)
            .map(|c| c.as_str()),
        "aptitude" => AptitudeCode::ALL.get(char_idx).map(|c| c.as_str()),
        _ => None,
    };
    let Some(code_str) = code_opt else {
        return;
    };
    let sel_id = format!(
        "wiz-slot-{slug}-{}",
        code_str.to_ascii_lowercase().replace('-', "_")
    );
    let parsed_slot = select_value(&sel_id).trim().parse::<usize>().ok();

    with_state_mut(|st| {
        st.wizard.assign_slot(&slug, char_idx, parsed_slot);
    });
    render_character_wizard();
}

/// Rolls correlated Height (`m`) and Mass (`kg`) around the mean (`1.72 m`, BMI `23.0` adjusted by Build & `FOR`).
#[wasm_bindgen]
pub fn satya_wizard_roll_height_mass() {
    with_state_mut(|st| {
        st.wizard.roll_height_mass_with(&mut roll_browser_die_i32);
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
        .unwrap_or(1.75);
    let mas_val = input_value("wiz-input-mas-kg")
        .trim()
        .parse::<f64>()
        .unwrap_or(70.0);
    with_state_mut(|st| {
        st.wizard.set_manual_height_mass(hgt_val, mas_val);
    });
    render_character_wizard();
}

/// Rolls a 7-value `2d4 + 2d5` pool for the 7 Psyche characteristics and assigns them in order (swap-enabled).
#[wasm_bindgen]
pub fn satya_wizard_roll_psyche_pool() {
    with_state_mut(|st| {
        let mut rolls: Vec<SingleCharacteristicRoll> = (0..7)
            .map(|_| roll_characteristic_2d4_2d5(&mut roll_browser_die_i32))
            .collect();
        rolls.sort_by_key(|a| std::cmp::Reverse(a.total));
        st.wizard.psyche_pool = Some(rolls);
        st.wizard.assign_slots_in_order("psyche");
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
        let mut pool = roll_aptitude_pool_11_drop_2(&mut roll_browser_die_i32);
        pool.kept_values.sort_by(|a, b| b.cmp(a));
        st.wizard.aptitude_pool = Some(pool);
        st.wizard.assign_slots_in_order("aptitude");
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
        for stage in 1..=5 {
            roll_lifepath_stage_in_config(&mut st.wizard.lifepath, stage, &mut roll_browser_die_i32);
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
        roll_lifepath_stage_in_config(&mut st.wizard.lifepath, stage_num, &mut roll_browser_die_i32);
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
    with_state_mut(|st| {
        apply_lifepath_field_in_config(&mut st.wizard.lifepath, &field, val, &mut roll_browser_die_i32);
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
        apply_career_term_in_config(&mut st.wizard.lifepath, term_idx, &f, v, &mut roll_browser_die_i32);
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

/// Adjusts a single skill level by `delta` (`+1` or `-1`) in Step 5, enforcing all 4 DAG gates,
/// the 5-factor Skill Point (`SP`) step cost, and the Stage 1–4 school floor.
#[wasm_bindgen]
pub fn satya_wizard_adjust_skill(skill_id: String, delta: i32) {
    let sk = skill_id.trim();
    if delta == 0 {
        return;
    }
    let (msg, is_err) = with_state_mut(|st| match st.wizard.try_adjust_skill(sk, delta) {
        Ok(new_n) => {
            let baseline = st.wizard.compute_lifepath_baseline();
            let ledger = compute_skill_allocation_sp_ledger(
                &baseline,
                &st.wizard.lifepath,
                &st.wizard.characteristics,
                &st.wizard.aptitudes,
                &st.wizard.skills,
            );
            let name = lookup_skill_node(sk).map_or(sk, |spec| spec.name);
            let verb = if delta < 0 { "Reduced" } else { "Advanced" };
            (
                format!(
                    "{verb} '{name}' to N={new_n} (Remaining: {} SP — Formal {}/{} SP, Experience {}/{} SP).",
                    ledger.total_sp_remaining,
                    ledger.formal_sp_spent,
                    ledger.formal_sp_budget,
                    ledger.experience_sp_spent,
                    ledger.experience_sp_budget
                ),
                false,
            )
        }
        Err(err) => (err, true),
    });

    render_character_wizard();
    set_feedback("wizard-feedback", &msg, is_err);
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
        crate::wasm_client::character_sheet::apply_wizard_to_character_sheet(st);
        let c = st.wizard.characteristics.clone();
        let d = c.compute_derived(0);
        let b = st.wizard.compute_lifepath_baseline();
        let s = st.wizard.skills.clone();
        (st.wizard.character_name.clone(), c, d, b, s)
    });

    crate::wasm_client::character_sheet::render_character_sheet();

    let control_dex_tac =
        chars.compute_controller(CharacteristicCode::DEX, Some(CharacteristicCode::TAC));
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

#[cfg(test)]
mod tests {
    use super::*;
    use satya_engine::{CANONICAL_SKILL_TREES, SkillType};

    fn make_cycling_die() -> impl FnMut(i32) -> i32 {
        let mut idx = 0usize;
        let seq = [1, 4, 2, 5, 3, 1, 2, 4, 5, 3, 4, 4, 5, 5, 5];
        move |sides: i32| {
            let v = seq[idx % seq.len()].clamp(1, sides.max(1));
            idx += 1;
            v
        }
    }

    #[test]
    fn wizard_slot_assignment_and_swapping_updates_characteristics() {
        let mut wiz = CharacterWizardState::default();
        let mut rng = make_cycling_die();
        wiz.roll_group_with("physical", &mut rng);

        assert_eq!(
            wiz.physical_slots,
            [Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)]
        );
        let kept = wiz.physical_pool.as_ref().unwrap().kept_values.clone();
        assert!(kept[0] > kept[5]);
        assert_eq!(wiz.characteristics.spd, kept[0]);
        assert_eq!(wiz.characteristics.fit, kept[5]);

        // Swap SPD (index 0) with slot 5 (currently held by FIT at index 5)
        wiz.assign_slot("physical", 0, Some(5));
        assert_eq!(wiz.physical_slots[0], Some(5));
        assert_eq!(wiz.physical_slots[5], Some(0));
        assert_eq!(wiz.characteristics.spd, kept[5]);
        assert_eq!(wiz.characteristics.fit, kept[0]);
    }

    #[test]
    fn wizard_height_mass_auto_recalc_vs_manual_override() {
        let mut wiz = CharacterWizardState::default();
        let mut rng = make_cycling_die();
        wiz.roll_group_with("physical", &mut rng);
        let kept = wiz.physical_pool.as_ref().unwrap().kept_values.clone();
        assert!(kept[0] > kept[5]);

        // Assign FOR (index 1) to lowest slot (5), then roll correlated Height/Mass
        wiz.assign_slot("physical", 1, Some(5));
        wiz.roll_height_mass_with(&mut |sides| (sides / 2).max(1));
        assert!(!wiz.height_mass_manual_override);
        let base_hgt = wiz.characteristics.hgt_m;
        let low_for_mass = wiz.characteristics.mas_kg;

        // Swapping FOR to highest slot (0) auto-recalculates Mass upward while preserving stature
        wiz.assign_slot("physical", 1, Some(0));
        assert!((wiz.characteristics.hgt_m - base_hgt).abs() < 1e-9);
        assert!(wiz.characteristics.mas_kg > low_for_mass);

        // Manual override locks Height/Mass against subsequent FOR slot changes and clamps bounds
        wiz.set_manual_height_mass(1.91, 94.5);
        assert!(wiz.height_mass_manual_override);
        assert!((wiz.characteristics.hgt_m - 1.91).abs() < 1e-9);
        assert!((wiz.characteristics.mas_kg - 94.5).abs() < 1e-9);

        wiz.assign_slot("physical", 1, Some(5));
        assert!((wiz.characteristics.hgt_m - 1.91).abs() < 1e-9);
        assert!((wiz.characteristics.mas_kg - 94.5).abs() < 1e-9);

        wiz.set_manual_height_mass(2.90, 20.0);
        assert!((wiz.characteristics.hgt_m - 2.20).abs() < 1e-9);
        assert!((wiz.characteristics.mas_kg - 40.0).abs() < 1e-9);
    }

    #[test]
    fn wizard_try_adjust_skill_enforces_floor_dag_term_cap_and_sp_budget() {
        let mut wiz = CharacterWizardState::default();
        wiz.lifepath.career_terms.truncate(1);
        let baseline = wiz.compute_lifepath_baseline();
        wiz.skills = baseline.baseline_skills.clone();

        // 1. Stage 1–4 school floor cannot be reduced
        let floor_err = wiz
            .try_adjust_skill("native_language_speech", -1)
            .unwrap_err();
        assert!(floor_err.contains("Stage 1–4 Lifepath floor"));

        // 2. High-tier gated skills fail 4-gate DAG validation when prerequisites are missing
        assert!(
            wiz.try_adjust_skill("trauma_orthopedic_general_surgery", 1)
                .is_err()
        );
        assert!(
            wiz.try_adjust_skill("piloting_rotary_wing_helicopter", 1)
                .is_err()
        );

        // 3. Valid +1 advancement up to post-school term cap, cap rejection, and -1 step-back
        let root_talent_id = CANONICAL_SKILL_TREES
            .iter()
            .find(|s| {
                s.skill_type == SkillType::Tal
                    && baseline.baseline_skills.get(s.id).copied() == Some(1)
            })
            .expect("at least one N=1 root talent")
            .id;
        let max_gain = baseline.max_post_school_gain_per_skill;
        for step_idx in 1..=max_gain {
            assert_eq!(wiz.try_adjust_skill(root_talent_id, 1), Ok(1 + step_idx));
        }
        let cap_err = wiz.try_adjust_skill(root_talent_id, 1).unwrap_err();
        assert!(cap_err.contains("post-school gain cap"));
        for step_idx in (1..=max_gain).rev() {
            assert_eq!(wiz.try_adjust_skill(root_talent_id, -1), Ok(step_idx));
        }
        assert!(wiz.try_adjust_skill(root_talent_id, -1).is_err());

        // 4. SP budget exhaustion rejects further advancement once remaining SP is insufficient
        let mut sp_exhausted_err = None;
        for spec in CANONICAL_SKILL_TREES.iter().filter(|s| {
            s.skill_type == SkillType::Tal && baseline.baseline_skills.get(s.id).copied() == Some(1)
        }) {
            if let Err(err) = wiz.try_adjust_skill(spec.id, 1) {
                sp_exhausted_err = Some(err);
                break;
            }
        }
        let err_msg = sp_exhausted_err.expect("Experience SP budget must exhaust across talents");
        assert!(err_msg.contains("SP remains"));
    }
}
