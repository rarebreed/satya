//! Step-by-Step Character Creation Wizard (`#notebook-pane-wizard`) state and `#[wasm_bindgen]` handlers.
//!
//! HTML rendering for the 6 wizard steps is modularized into:
//! - [`crate::wasm_client::wizard_views_stats`] (Steps 1–3: Primary/Sensory Pools, Height/Mass, Psyche & Aptitudes)
//! - [`crate::wasm_client::wizard_views_lifepath`] (Step 4: Chronological 5-Stage Lifepath)
//! - [`crate::wasm_client::wizard_views_skills`] (Steps 5–6: 8-Tree Skill Builder, 5-Factor SP Cost & Derived Summary)

use std::collections::BTreeMap;

use crate::wasm_client::chat_and_ttrpg::{append_system_notice, satya_update_cascade_preview};
use crate::wasm_client::dom::{
    escape_html, get_element, input_value, roll_browser_die_i32, select_value, set_feedback,
    set_input_value, set_text,
};
use crate::wasm_client::state::{with_state, with_state_mut};
pub use crate::wasm_client::wizard_views_lifepath::*;
pub use crate::wasm_client::wizard_views_skills::*;
pub use crate::wasm_client::wizard_views_stats::*;
use satya_engine::{
    AptitudeCode, AptitudePoolRoll, CareerTermConfig, CareerTermTrack, CharacterAptitudes,
    CharacterCharacteristics, CharacteristicCategory, CharacteristicCode, CharacteristicGroupPool,
    CharacteristicRollMethod, CorrelatedHeightMassRoll, HighSchoolTrack, LifepathBaselineResult,
    LifepathBuildConfig, RuralYouthArchetype, SingleCharacteristicRoll, SocioeconomicClass,
    Stage4Path, StreetGangArchetype, auto_allocate_career_and_elective_skills,
    compute_correlated_height_mass, compute_lifepath_baseline_from_config,
    compute_skill_allocation_sp_ledger, evaluate_operational_heat, lookup_campaign_sqm,
    lookup_skill_node, preview_next_skill_step_sp, resolve_socioeconomic_origin,
    roll_aptitude_pool_11_drop_2, roll_characteristic_2d4_2d5, roll_characteristic_group_pool,
    roll_correlated_height_mass, validate_skill_advancement,
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

fn roll_2d20_pairs() -> [(i32, i32); 2] {
    [
        (roll_browser_die_i32(20), roll_browser_die_i32(20)),
        (roll_browser_die_i32(20), roll_browser_die_i32(20)),
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

fn roll_single_group_in_state(st: &mut CharacterWizardState, group_slug: &str) {
    let cat = match group_slug {
        "physical" => CharacteristicCategory::Physical,
        "mental" => CharacteristicCategory::Mental,
        "sensory" => CharacteristicCategory::Sensory,
        "social" => CharacteristicCategory::Social,
        _ => return,
    };
    let mut pool = roll_characteristic_group_pool(cat, st.roll_method, &mut roll_browser_die_i32);
    pool.kept_values.sort_by(|a, b| b.cmp(a));
    match group_slug {
        "physical" => st.physical_pool = Some(pool),
        "mental" => st.mental_pool = Some(pool),
        "sensory" => st.sensory_pool = Some(pool),
        "social" => st.social_pool = Some(pool),
        _ => {}
    }
    st.assign_slots_in_order(group_slug);
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
        for slug in ["physical", "mental", "sensory", "social"] {
            roll_single_group_in_state(&mut st.wizard, slug);
        }
        st.wizard.height_mass_manual_override = false;
        let hm = roll_correlated_height_mass(
            st.wizard.characteristics.for_stat,
            &mut roll_browser_die_i32,
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
        st.wizard.assign_slots_in_order(&slug);
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
    let parsed_slot = select_value(&sel_id).trim().parse::<usize>().ok();

    with_state_mut(|st| {
        if let Some(slots) = st.wizard.slots_mut(&slug) {
            assign_or_swap_slot(slots, char_idx, parsed_slot);
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
            &mut roll_browser_die_i32,
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
        let lp = &mut st.wizard.lifepath;
        lp.origin_roll_2d20 = roll_browser_die_i32(20) + roll_browser_die_i32(20);
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
        lp.age15_crisis_roll_2d20 = roll_browser_die_i32(20) + roll_browser_die_i32(20);
        lp.age15_dropout_override = None;
        lp.stage4_path_override = None;
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
                lp.origin_roll_2d20 = roll_browser_die_i32(20) + roll_browser_die_i32(20);
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
                lp.age15_crisis_roll_2d20 = roll_browser_die_i32(20) + roll_browser_die_i32(20);
                lp.age15_dropout_override = None;
                lp.stage4_path_override = None;
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

/// Adjusts a single skill level by `delta` (`+1` or `-1`) in Step 5, enforcing all 4 DAG gates,
/// the 5-factor Skill Point (`SP`) step cost, and the Stage 1–4 school floor.
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
                let new_ledger = compute_skill_allocation_sp_ledger(
                    &baseline,
                    &st.wizard.lifepath,
                    &st.wizard.characteristics,
                    &st.wizard.aptitudes,
                    &st.wizard.skills,
                );
                feedback_msg = Some((
                    format!(
                        "Reduced '{sk}' to N={new_n} (Remaining: {} SP — Formal {}/{} SP, Experience {}/{} SP).",
                        new_ledger.total_sp_remaining,
                        new_ledger.formal_sp_spent,
                        new_ledger.formal_sp_budget,
                        new_ledger.experience_sp_spent,
                        new_ledger.experience_sp_budget
                    ),
                    false,
                ));
            } else {
                feedback_msg = Some((
                    format!("Cannot reduce '{sk}' below Stage 1–4 Lifepath floor (N={floor_n})."),
                    true,
                ));
            }
        } else if delta > 0 {
            let target_n = cur_n + 1;
            let ledger = compute_skill_allocation_sp_ledger(
                &baseline,
                &st.wizard.lifepath,
                &st.wizard.characteristics,
                &st.wizard.aptitudes,
                &st.wizard.skills,
            );
            let (step_bd, sp_affordable) = preview_next_skill_step_sp(
                sk,
                target_n,
                &st.wizard.lifepath,
                &ledger,
                &st.wizard.characteristics,
                &st.wizard.aptitudes,
            );
            let gained_post_school = cur_n.saturating_sub(floor_n);
            if !sp_affordable {
                feedback_msg = Some((
                    format!(
                        "Advancing '{sk}' to N={target_n} costs {} SP ({}), but only {} SP remains (Formal {} SP, Experience {} SP).",
                        step_bd.final_sp_cost,
                        step_bd.training_mode.as_label(),
                        ledger.total_sp_remaining,
                        ledger.formal_sp_remaining,
                        ledger.experience_sp_remaining
                    ),
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
                    let new_ledger = compute_skill_allocation_sp_ledger(
                        &baseline,
                        &st.wizard.lifepath,
                        &st.wizard.characteristics,
                        &st.wizard.aptitudes,
                        &st.wizard.skills,
                    );
                    feedback_msg = Some((
                        format!(
                            "Advanced '{}' to N={target_n} for {} SP ({} · Diff {}/5). Remaining: {} SP.",
                            val.skill_name,
                            step_bd.final_sp_cost,
                            step_bd.training_mode.as_label(),
                            step_bd.difficulty_rating,
                            new_ledger.total_sp_remaining
                        ),
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

    if let Some(chips_el) = get_element("char-sheet-skills-chips") {
        let chips_html = format_trained_skill_chips_html(&chars, &skills);
        if !chips_html.is_empty() {
            chips_el.set_inner_html(&chips_html);
        }
    }

    if let Some(lp_el) = get_element("char-sheet-lifepath-summary") {
        let sqm_spec = lookup_campaign_sqm(baseline.campaign_sqm);
        let heat_spec = evaluate_operational_heat(baseline.operational_heat);
        let access_gates_html = format_access_gates_pills_html(&baseline);
        lp_el.set_inner_html(&format!(
            r#"<span class="roster-title">Lifepath, [Access] Gates &amp; Economy:</span>
            <div class="roster-chips">
                <span class="pill pill-iggy">Origin: {orig}</span>
                <span class="pill pill-iggy">SQM {sqm}: ${usd}/mo ({tier_name})</span>
                <span class="pill pill-iggy">Heat {heat}/10 (+{h_delta} Δ_ext)</span>
                {access_gates_html}
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
