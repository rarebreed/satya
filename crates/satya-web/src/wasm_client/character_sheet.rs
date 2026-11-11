//! Interactive Character Sheet & Anatomical/Physiological/Psyche Status Tracker state and
//! `#[wasm_bindgen]` handlers for the `#notebook-pane-character` tab.
//!
//! Companion modules re-exported flatly here:
//! - [`crate::wasm_client::character_sheet_views`]: Top summary, all 30 Characteristics, 9 Domain Aptitudes,
//!   10 Derived Metrics, Tactical Pool Envelope, Innate Talents Table, and Trained Skills Table.
//! - [`crate::wasm_client::character_sheet_equip_views`]: Ranged Firearms (with live Ballistics),
//!   Melee Weapons, Armor Profiles, and Field Gear / Ergonomic Encumbrance tables.
//! - [`crate::wasm_client::character_sheet_status_views`]: Human-shaped Anatomical Hit Location SVG diagram,
//!   13-zone physical status editor, Damage/Fatigue/Psyche trackers, and writable Operator Field Notes.
//! - [`crate::wasm_client::character_sheet_record`]: `CharacterSheetRecord` conversion & offline `.satyachar` framing.
//! - [`crate::wasm_client::character_sheet_vault`]: Vault API sync, `.satyachar` file picker, and multimodal assets.

use std::collections::BTreeMap;

pub use crate::wasm_client::character_sheet_equip_views::*;
pub use crate::wasm_client::character_sheet_record::*;
pub use crate::wasm_client::character_sheet_status_views::*;
pub use crate::wasm_client::character_sheet_vault::*;
pub use crate::wasm_client::character_sheet_views::*;
use crate::wasm_client::chat_and_ttrpg::{append_system_notice, satya_update_cascade_preview};
use crate::wasm_client::dom::{
    get_element, input_value, roll_browser_die_i32, select_value, set_input_value, set_text,
    slugify_identifier,
};
use crate::wasm_client::state::{with_state, with_state_mut};
pub use satya_engine::{
    BoneFractureStatus, CANONICAL_FIELD_GEAR_PRESETS, CANONICAL_FIREARM_PRESETS, FieldGearItem,
    lookup_field_gear_preset, lookup_firearm_preset,
};
use satya_engine::{
    AcuteMentalTraumaTier, AptitudeCode, ArmorProfileId, CarriedLoadItem, CarryErgonomicMethod,
    CharacterAptitudes, CharacterCharacteristics, CharacteristicCode, ConditionImpact,
    ConditionSource, HemorrhageGrade, HitLocationId, MeleeWeaponId, MoraleState,
    PhysiologicalStatusCondition, SkillAccessTier, SocialWoundKind, StructuralWoundTier,
    TacticalPoolEnvelope, TacticalPoolParams, Triad, accumulate_exertion_marks,
    build_amara_okafor_lifepath, build_mateo_vance_lifepath, build_priya_nair_lifepath,
    compute_lifepath_baseline_from_config, compute_tactical_pool, evaluate_acute_mental_trauma,
    evaluate_blood_loss_progression, evaluate_encumbrance_load, evaluate_fatigue_level,
    evaluate_physiological_status_conditions, initialize_universal_talents,
    resolve_hit_location_d20,
};
use wasm_bindgen::prelude::*;

/// Per-zone physical trauma state for one of the 13 canonical [`HitLocationId`] regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnatomicalZoneState {
    pub location: HitLocationId,
    /// Structural wound severity (`0` = Uninjured, `1..=5` = `Tier 1` .. `Tier 5`).
    pub wound_tier: u8,
    pub bone_status: BoneFractureStatus,
    pub bleed_grade: HemorrhageGrade,
}

impl AnatomicalZoneState {
    #[must_use]
    pub const fn new_healthy(location: HitLocationId) -> Self {
        Self {
            location,
            wound_tier: 0,
            bone_status: BoneFractureStatus::Intact,
            bleed_grade: HemorrhageGrade::Grade0None,
        }
    }

    #[must_use]
    pub const fn is_injured(&self) -> bool {
        self.wound_tier > 0
            || !matches!(self.bone_status, BoneFractureStatus::Intact)
            || !matches!(self.bleed_grade, HemorrhageGrade::Grade0None)
    }

    #[must_use]
    pub const fn effective_cap_penalty(&self) -> i32 {
        let wound_pen = if self.wound_tier == 0 {
            0
        } else {
            StructuralWoundTier::from_tier_number(self.wound_tier).aspect_ceiling_penalty()
        };
        let bone_pen = self.bone_status.extra_cap_penalty();
        if wound_pen > bone_pen {
            wound_pen
        } else {
            bone_pen
        }
    }
}

/// Complete state for the interactive Character Sheet / Status notebook pane (`#notebook-pane-character`).
#[derive(Debug, Clone)]
pub struct CharacterSheetState {
    pub character_id: String,
    pub preset_id: String,
    pub character_name: String,
    pub callsign_or_role: String,
    pub age_years: u32,
    pub origin_label: String,
    pub campaign_sqm: u8,
    pub operational_heat: u8,
    pub granted_access_tags: Vec<SkillAccessTier>,
    pub stage_history: Vec<String>,
    pub lifepath_config_json: Option<String>,
    pub section_filter: String,
    pub skill_tree_filter: String,
    pub skills_trained_only: bool,
    pub characteristics: CharacterCharacteristics,
    pub aptitudes: CharacterAptitudes,
    pub skills: BTreeMap<String, usize>,
    pub equipped_ranged_ids: Vec<String>,
    pub ballistic_range_m: f64,
    pub equipped_melee: Vec<MeleeWeaponId>,
    pub equipped_armor: Vec<ArmorProfileId>,
    pub field_gear: Vec<FieldGearItem>,
    pub hit_zones: [AnatomicalZoneState; 13],
    pub selected_hit_zone: usize,
    pub last_hit_location_roll_d20: Option<i32>,
    pub fatigue_level: i32,
    pub exertion_marks: i32,
    pub wound_shock_index: i32,
    pub blood_units_lost: f64,
    pub active_status_conditions: Vec<PhysiologicalStatusCondition>,
    pub morale_state: MoraleState,
    pub mental_trauma: Option<AcuteMentalTraumaTier>,
    pub social_composure_lost: i32,
    pub social_wound: Option<SocialWoundKind>,
    pub psyche_drift_marks: i32,
    pub conviction_surge_active: bool,
    pub notes: String,
    pub verification_status: satya_schema::CharacterVerificationStatus,
    pub audit_violations: Vec<String>,
    pub local_satya_file_path: Option<String>,
    pub assets: Vec<satya_schema::CharacterAssetRef>,
    pub saved_vault_list: Vec<satya_schema::CharacterSheetRecord>,
    pub vault_feedback: String,
}

impl Default for CharacterSheetState {
    fn default() -> Self {
        Self::from_preset("sara_valkyrie")
    }
}

impl CharacterSheetState {
    /// Builds a canonical operator preset (`"sara_valkyrie"`, `"mateo_vance"`, `"priya_nair"`, or `"amara_okafor"`).
    #[must_use]
    pub fn from_preset(preset_id: &str) -> Self {
        match preset_id {
            "mateo_vance" => Self::from_lifepath_profile(
                "mateo_vance",
                "Syndicate Enforcer & Urban Pointman",
                build_mateo_vance_lifepath(),
                vec!["sig_p320_9mm".to_string(), "akm_762x39".to_string()],
                vec![
                    MeleeWeaponId::CombatKnifeKarambitShiv,
                    MeleeWeaponId::UnarmedJabLowKickClinchKnee,
                ],
                vec![ArmorProfileId::ConcealableSoftKevlarVestNij3a],
                &["ifak_cat_tourniquet", "comms_tac_mesh"],
                "Syndicate safehouse keycard (Sector 9). Watching Dock 4 shipments.\nHigh APT-QNT (+3) ballistic intuition; keep Heat below 4.",
            ),
            "priya_nair" => Self::from_lifepath_profile(
                "priya_nair",
                "Corporate AI & Robotics Lead Scientist",
                build_priya_nair_lifepath(),
                vec!["mp7_pdw_57".to_string()],
                vec![MeleeWeaponId::ExpandableBatonTonfaClub],
                vec![ArmorProfileId::HeavyLeatherSynthWeaveJacket],
                &[
                    "cyber_deck_analyzer",
                    "comms_tac_mesh",
                    "nvg_thermal_binocs",
                ],
                "Holding Level-4 autonomous drone override keys for Sector 4 Airlock.\nPrimary mission: audit telemetry anomalies and secure robotics core.",
            ),
            "amara_okafor" => Self::from_lifepath_profile(
                "amara_okafor",
                "Frontier Recon Tracker & Anti-Poaching Ranger",
                build_amara_okafor_lifepath(),
                vec!["ai_atx_65cm".to_string(), "sig_p320_9mm".to_string()],
                vec![
                    MeleeWeaponId::MacheteWakizashiTacticalBlade,
                    MeleeWeaponId::CombatKnifeKarambitShiv,
                ],
                vec![ArmorProfileId::LightweightUhmwpePatrolCarrierNij3],
                &[
                    "trauma_kit_pro",
                    "ifak_cat_tourniquet",
                    "nvg_thermal_binocs",
                ],
                "Perimeter overwatch established at 150m elevation.\nHerbalism & field triage stocks full; tracking hostile patrol signatures.",
            ),
            _ => {
                let chars = CharacterCharacteristics {
                    spd: 14,
                    for_stat: 13,
                    dex: 14,
                    kin: 14,
                    hlt: 13,
                    fit: 12,
                    hgt_m: 1.78,
                    mas_kg: 72.0,
                    mem: 13,
                    ana: 15,
                    ins: 13,
                    cre: 11,
                    dis: 14,
                    foc: 14,
                    com: 11,
                    pre: 13,
                    elo: 11,
                    sig: 14,
                    hea: 12,
                    tac: 12,
                    olf: 10,
                    tas: 10,
                    itu: 13,
                    ego: 11,
                    slf: 7,
                    opn: 13,
                    blg: 14,
                    ext: 10,
                    cnf: 12,
                    agg: 11,
                };
                let mut aptitudes = CharacterAptitudes::default();
                aptitudes[AptitudeCode::AptBio] = 2;
                aptitudes[AptitudeCode::AptKin] = 2;
                aptitudes[AptitudeCode::AptFin] = 1;
                aptitudes[AptitudeCode::AptMec] = 1;
                let mut skills = initialize_universal_talents();
                for (id, n) in [
                    ("alertness_situational_awareness", 3),
                    ("running_sprinting", 2),
                    ("brawling_unarmed_striking", 2),
                    ("manual_dexterity_tinkering", 2),
                    ("empathy_social_perception", 2),
                    ("reading_comprehension", 2),
                    ("arithmetic", 2),
                    ("first_aid_bls_cpr", 3),
                    ("paramedicine_combat_trauma_tccc", 3),
                    ("trauma_orthopedic_general_surgery", 3),
                    ("zero_g_eva_operations", 4),
                    ("pistols_handguns", 3),
                    ("long_guns_rifles_carbines_shotguns", 3),
                    ("fully_automatic_burst_weapons", 3),
                    ("small_unit_tactics_cqb_doctrine", 4),
                    ("surveillance_reconnaissance_countersurveillance", 2),
                    ("camouflage_concealment", 2),
                ] {
                    skills.insert(id.to_string(), n);
                }
                Self {
                    character_id: "char-sara_valkyrie".to_string(),
                    preset_id: "sara_valkyrie".to_string(),
                    character_name: "Sara \"Valkyrie\" Lin".to_string(),
                    callsign_or_role: "Orbital Med-Assault Operator (Valkyrie Exo-Rig)".to_string(),
                    age_years: 26,
                    origin_label: "Professional / Upper-Middle Class".to_string(),
                    campaign_sqm: 6,
                    operational_heat: 1,
                    granted_access_tags: vec![
                        SkillAccessTier::Universal,
                        SkillAccessTier::Academic,
                        SkillAccessTier::VocationalLicensed,
                        SkillAccessTier::Restricted,
                    ],
                    stage_history: vec![
                        "Stage 4A: STEM Magnet High School -> B.S. Pre-Med & Combat Trauma Residency.".to_string(),
                        "Stage 5: 2 Terms Orbital SpecOps Pararescue & Med-Assault (Zero-G EVA N=4, Trauma Surgery N=3, Small-Unit Tactics N=4).".to_string(),
                    ],
                    lifepath_config_json: None,
                    section_filter: "ALL".to_string(),
                    skill_tree_filter: "ALL".to_string(),
                    skills_trained_only: true,
                    characteristics: chars,
                    aptitudes,
                    skills,
                    equipped_ranged_ids: vec!["hk416_carbine_556".to_string(), "sig_p320_9mm".to_string()],
                    ballistic_range_m: 25.0,
                    equipped_melee: vec![
                        MeleeWeaponId::CombatKnifeKarambitShiv,
                        MeleeWeaponId::UnarmedJabLowKickClinchKnee,
                    ],
                    equipped_armor: vec![
                        ArmorProfileId::ValkyrieReconExoCarapace2035,
                        ArmorProfileId::BallisticCombatHelmetVisor,
                    ],
                    field_gear: Self::gear_from_ids(&[
                        "trauma_kit_pro",
                        "ifak_cat_tourniquet",
                        "comms_tac_mesh",
                        "eva_mag_thruster_pack",
                    ]),
                    hit_zones: HitLocationId::ALL.map(AnatomicalZoneState::new_healthy),
                    selected_hit_zone: HitLocationId::ChestRibcageLungs as usize,
                    last_hit_location_roll_d20: None,
                    fatigue_level: 1,
                    exertion_marks: 3,
                    wound_shock_index: 0,
                    blood_units_lost: 0.0,
                    active_status_conditions: Vec::new(),
                    morale_state: MoraleState::Steady,
                    mental_trauma: None,
                    social_composure_lost: 0,
                    social_wound: None,
                    psyche_drift_marks: 1,
                    conviction_surge_active: false,
                    notes: "Squad Med-Assault Lead — Sector 4 Orbital Airlock breach team.\n• Valkyrie Exo-Rig battery at 94% (+20.0 kg load offset).\n• Trauma Surgery Kit & 2x CAT tourniquets staged on chest rig.\n• Primary objective: secure decompression valve and extract VIP.".to_string(),
                    verification_status: satya_schema::CharacterVerificationStatus::VerifiedOfficial,
                    audit_violations: Vec::new(),
                    local_satya_file_path: None,
                    assets: Vec::new(),
                    saved_vault_list: Vec::new(),
                    vault_feedback: String::new(),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn from_lifepath_profile(
        preset_id: &str,
        role: &str,
        lp: satya_engine::LifepathCharacterProfile,
        ranged_ids: Vec<String>,
        melee: Vec<MeleeWeaponId>,
        armor: Vec<ArmorProfileId>,
        gear_ids: &[&str],
        notes: &str,
    ) -> Self {
        Self {
            character_id: format!("char-{preset_id}"),
            preset_id: preset_id.to_string(),
            character_name: lp.name,
            callsign_or_role: role.to_string(),
            age_years: lp.age_years,
            origin_label: lp.origin.label.to_string(),
            campaign_sqm: lp.campaign_sqm,
            operational_heat: lp.operational_heat,
            granted_access_tags: lp.granted_access_tags,
            stage_history: lp.stage_history,
            lifepath_config_json: None,
            section_filter: "ALL".to_string(),
            skill_tree_filter: "ALL".to_string(),
            skills_trained_only: true,
            characteristics: lp.characteristics,
            aptitudes: lp.aptitudes,
            skills: lp.skills,
            equipped_ranged_ids: ranged_ids,
            ballistic_range_m: 25.0,
            equipped_melee: melee,
            equipped_armor: armor,
            field_gear: Self::gear_from_ids(gear_ids),
            hit_zones: HitLocationId::ALL.map(AnatomicalZoneState::new_healthy),
            selected_hit_zone: HitLocationId::ChestRibcageLungs as usize,
            last_hit_location_roll_d20: None,
            fatigue_level: 0,
            exertion_marks: 0,
            wound_shock_index: 0,
            blood_units_lost: 0.0,
            active_status_conditions: Vec::new(),
            morale_state: MoraleState::Steady,
            mental_trauma: None,
            social_composure_lost: 0,
            social_wound: None,
            psyche_drift_marks: 0,
            conviction_surge_active: false,
            notes: notes.to_string(),
            verification_status: satya_schema::CharacterVerificationStatus::VerifiedOfficial,
            audit_violations: Vec::new(),
            local_satya_file_path: None,
            assets: Vec::new(),
            saved_vault_list: Vec::new(),
            vault_feedback: String::new(),
        }
    }

    pub(crate) fn gear_from_ids(ids: &[&str]) -> Vec<FieldGearItem> {
        ids.iter()
            .filter_map(|&id| lookup_field_gear_preset(id))
            .collect()
    }

    /// Computes localized `(Tempo, Control, Force)` ceiling penalties from all 13 anatomical hit zones.
    #[must_use]
    pub fn compute_anatomical_wound_cap_penalties(&self) -> Triad<i32> {
        let mut pen = Triad::ZERO;
        for z in &self.hit_zones {
            let cap_pen = z.effective_cap_penalty();
            if cap_pen > 0 {
                let spec = z.location.spec();
                if spec.slashes_tempo_cap {
                    pen.tempo = (pen.tempo + cap_pen).min(20);
                }
                if spec.slashes_control_cap {
                    pen.control = (pen.control + cap_pen).min(20);
                }
                if spec.slashes_force_cap {
                    pen.force = (pen.force + cap_pen).min(20);
                }
            }
        }
        pen
    }

    /// Collects all active non-zero [`HemorrhageGrade`] bleeds across the 13 anatomical hit zones.
    #[must_use]
    pub fn active_bleeds(&self) -> Vec<HemorrhageGrade> {
        self.hit_zones
            .iter()
            .filter_map(|z| (z.bleed_grade != HemorrhageGrade::Grade0None).then_some(z.bleed_grade))
            .collect()
    }

    /// Computes systemic wound shock contributed by anatomical wounds and blood loss, combined with `self.wound_shock_index`.
    #[must_use]
    pub fn effective_wound_shock_index(&self) -> i32 {
        let wsr = self
            .characteristics
            .compute_derived(0)
            .wsr_wound_shock_resistance;
        let zone_shock: i32 = self
            .hit_zones
            .iter()
            .filter(|z| z.wound_tier > 0)
            .map(|z| StructuralWoundTier::from_tier_number(z.wound_tier).wound_shock_added(wsr))
            .sum();
        let blood_eval =
            evaluate_blood_loss_progression(self.blood_units_lost, &self.active_bleeds(), 0);
        (self.wound_shock_index.max(zone_shock) + blood_eval.added_wound_shock_index).clamp(0, 5)
    }

    /// Computes effective Fatigue Level (`0..=5`) combining `self.fatigue_level` and hypovolemic blood loss fatigue.
    #[must_use]
    pub fn effective_fatigue_level(&self) -> i32 {
        let blood_eval =
            evaluate_blood_loss_progression(self.blood_units_lost, &self.active_bleeds(), 0);
        (self.fatigue_level + blood_eval.added_fatigue_level).clamp(0, 5)
    }

    /// Computes total carried items and powered exoskeleton offset for `evaluate_encumbrance_load`.
    #[must_use]
    pub fn compute_encumbrance(&self) -> satya_engine::EncumbranceEvaluation {
        let mut items = Vec::new();
        for r_id in &self.equipped_ranged_ids {
            if let Some(spec) = lookup_firearm_preset(r_id) {
                items.push(CarriedLoadItem {
                    mass_kg: spec.loaded_weight_kg,
                    carry_method: CarryErgonomicMethod::BalancedWebbingOrRucksack,
                });
            }
        }
        for &m_id in &self.equipped_melee {
            let mass_kg = m_id.mass_kg();
            if mass_kg > 0.0 {
                items.push(CarriedLoadItem {
                    mass_kg,
                    carry_method: CarryErgonomicMethod::TailoredArmorOrDutyBelt,
                });
            }
        }
        let mut exo_offset_kg = 0.0;
        for &a_id in &self.equipped_armor {
            let spec = a_id.spec();
            items.push(CarriedLoadItem {
                mass_kg: spec.weight_kg,
                carry_method: CarryErgonomicMethod::TailoredArmorOrDutyBelt,
            });
            exo_offset_kg += spec.exo_load_offset_kg;
        }
        for g in &self.field_gear {
            items.push(CarriedLoadItem {
                mass_kg: g.mass_kg,
                carry_method: g.carry_method,
            });
        }
        evaluate_encumbrance_load(
            self.characteristics.mas_kg,
            self.characteristics.for_stat,
            &items,
            exo_offset_kg,
        )
    }

    /// Computes aggregated [`ConditionImpact`] from active physiological status conditions, mental trauma, and encumbrance.
    #[must_use]
    pub fn compute_combined_condition_impact(&self) -> ConditionImpact {
        let phys = evaluate_physiological_status_conditions(&self.active_status_conditions)
            .to_condition_impact();
        let mut combined = phys.combine(self.compute_encumbrance().to_condition_impact());
        if let Some(mt) = self.mental_trauma {
            combined = combined.combine(
                evaluate_acute_mental_trauma(mt, false, self.characteristics.agg)
                    .to_condition_impact(),
            );
        }
        if let Some(sw) = self.social_wound {
            combined.delta_int += sw.delta_int();
        }
        if self.conviction_surge_active {
            combined.delta_int = (combined.delta_int - 6).max(-2);
            combined.tactical_pool_mod += 2;
        }
        combined
    }

    /// Computes total internal difficulty modifier $\Delta_{\text{int}}$ affecting `2d20` tasks.
    #[must_use]
    pub fn total_task_delta_int(&self) -> i32 {
        let fat_spec = evaluate_fatigue_level(self.effective_fatigue_level());
        let raw = fat_spec.delta_int
            + (2 * self.effective_wound_shock_index())
            + self.compute_combined_condition_impact().delta_int;
        if self.conviction_surge_active {
            raw.max(-2)
        } else {
            raw
        }
    }

    /// Computes the live [`TacticalPoolEnvelope`] (`P_eff / P_base`, `Tempo / Control / Force` caps) for this operator.
    #[must_use]
    pub fn compute_live_tactical_envelope(&self) -> TacticalPoolEnvelope {
        let c_dex_tac = self
            .characteristics
            .compute_controller(CharacteristicCode::DEX, Some(CharacteristicCode::TAC));
        compute_tactical_pool(&TacticalPoolParams {
            chars: self.characteristics.clone(),
            skill_c: c_dex_tac,
            skill_n: 3,
            is_ranged: true,
            fatigue_level: self.effective_fatigue_level(),
            wound_shock_index: self.effective_wound_shock_index(),
            wound_cap_penalties: self.compute_anatomical_wound_cap_penalties(),
            condition: self.compute_combined_condition_impact(),
            morale_state: self.morale_state,
        })
    }

    /// Formats the status pill text for `#char-sheet-status-pill`.
    #[must_use]
    pub fn format_status_pill_text(&self) -> String {
        let derived = self.characteristics.compute_derived(0);
        let env = self.compute_live_tactical_envelope();
        let (fat, shock) = (
            self.effective_fatigue_level(),
            self.effective_wound_shock_index(),
        );
        let state_tag = if env.caps == Triad::ZERO {
            "INCAPACITATED"
        } else if shock >= 2 || fat >= 3 {
            "IMPAIRED"
        } else {
            "Combat Ready"
        };
        format!(
            "{} (Age {}) · {} · OODA {} · D_cog {} · Pool {}/{} · Fatigue {} · Shock {} · Δ_int {:+}",
            self.character_name,
            self.age_years,
            state_tag,
            derived.ooda_initiative,
            derived.d_cog_quick_thinking,
            env.p_eff,
            env.p_base,
            fat,
            shock,
            self.total_task_delta_int()
        )
    }

    /// Applies a pure anatomical hit-zone mutation (`"wound_tier"`, `"bone_status"`, `"bleed_grade"`,
    /// `"stabilize_bleeds"`, or `"clear_all"`).
    pub fn apply_zone_mutation(&mut self, zone_idx: usize, field: &str, value: &str) {
        let f = field.trim().to_ascii_lowercase();
        let v = value.trim();
        if f == "clear_all" {
            self.hit_zones = HitLocationId::ALL.map(AnatomicalZoneState::new_healthy);
            self.wound_shock_index = 0;
            self.blood_units_lost = 0.0;
            return;
        }
        if f == "stabilize_bleeds" {
            for z in &mut self.hit_zones {
                z.bleed_grade = HemorrhageGrade::Grade0None;
            }
            return;
        }
        let Some(zone) = self.hit_zones.get_mut(zone_idx) else {
            return;
        };
        self.selected_hit_zone = zone_idx;
        match f.as_str() {
            "wound_tier" => zone.wound_tier = v.parse::<u8>().unwrap_or(0).min(5),
            "bone_status" => zone.bone_status = BoneFractureStatus::from_code(v),
            "bleed_grade" => {
                zone.bleed_grade = HemorrhageGrade::from_grade_number(v.parse::<i32>().unwrap_or(0));
            }
            _ => {}
        }
    }

    /// Applies a pure physiological, fatigue, blood-loss, morale, or psyche mutation onto `self`.
    pub fn apply_status_mutation(&mut self, field: &str, value: &str) {
        let f = field.trim().to_ascii_lowercase();
        let v = value.trim();
        let derived = self.characteristics.compute_derived(0);
        match f.as_str() {
            "fatigue_delta" => {
                self.fatigue_level =
                    (self.fatigue_level + v.parse::<i32>().unwrap_or(0)).clamp(0, 5);
            }
            "exertion_delta" => {
                let d = v.parse::<i32>().unwrap_or(0);
                if d <= -99 {
                    self.exertion_marks = 0;
                } else if d > 0 {
                    let prog = accumulate_exertion_marks(
                        self.exertion_marks,
                        d,
                        self.fatigue_level,
                        derived.ft_fatigue_threshold,
                    );
                    self.exertion_marks = prog.remaining_exertion_marks;
                    self.fatigue_level = prog.new_fatigue_level.clamp(0, 5);
                } else {
                    self.exertion_marks = (self.exertion_marks + d).max(0);
                }
            }
            "wound_shock_delta" => {
                self.wound_shock_index =
                    (self.wound_shock_index + v.parse::<i32>().unwrap_or(0)).clamp(0, 5);
            }
            "blood_loss_delta" => {
                self.blood_units_lost =
                    (self.blood_units_lost + v.parse::<f64>().unwrap_or(0.0)).clamp(0.0, 10.0);
            }
            "advance_bleed_ticks" => {
                let ticks = v.parse::<i32>().unwrap_or(20).max(1);
                let out = evaluate_blood_loss_progression(
                    self.blood_units_lost,
                    &self.active_bleeds(),
                    ticks,
                );
                self.blood_units_lost = out.total_units_lost;
            }
            "morale_state" => {
                if let Some(m) = MoraleState::from_code_str(v) {
                    self.morale_state = m;
                }
            }
            "mental_trauma" => {
                self.mental_trauma = AcuteMentalTraumaTier::from_code_str(v);
            }
            "social_wound" => {
                self.social_wound = SocialWoundKind::from_code_str(v);
            }
            "composure_delta" => {
                self.social_composure_lost = (self.social_composure_lost
                    + v.parse::<i32>().unwrap_or(0))
                .clamp(0, derived.cmp_social_composure_threshold);
            }
            "drift_delta" => {
                self.psyche_drift_marks =
                    (self.psyche_drift_marks + v.parse::<i32>().unwrap_or(0)).clamp(0, 10);
            }
            "toggle_surge" => self.conviction_surge_active = !self.conviction_surge_active,
            "toggle_condition" => {
                if let Some(cond) = PhysiologicalStatusCondition::from_code_str(v) {
                    if let Some(pos) = self
                        .active_status_conditions
                        .iter()
                        .position(|&c| c == cond)
                    {
                        self.active_status_conditions.remove(pos);
                    } else {
                        self.active_status_conditions.push(cond);
                    }
                }
            }
            _ => {}
        }
    }

    /// Applies a pure equipment loadout mutation (`"set_range"`, `"add_ranged"`, `"remove_ranged"`,
    /// `"add_melee"`, `"remove_melee"`, `"add_armor"`, `"remove_armor"`, `"add_gear"`, `"remove_gear"`).
    pub fn apply_equipment_mutation(&mut self, action: &str, item_id: &str) {
        let act = action.trim().to_ascii_lowercase();
        let id = item_id.trim();
        match act.as_str() {
            "set_range" => {
                self.ballistic_range_m = id.parse::<f64>().unwrap_or(25.0).clamp(5.0, 1000.0);
            }
            "add_ranged" => {
                if lookup_firearm_preset(id).is_some()
                    && !self.equipped_ranged_ids.iter().any(|x| x == id)
                {
                    self.equipped_ranged_ids.push(id.to_string());
                }
            }
            "remove_ranged" => self.equipped_ranged_ids.retain(|x| x != id),
            "add_melee" => {
                if let Ok(idx) = id.parse::<usize>()
                    && let Some(&mw) = MeleeWeaponId::ALL.get(idx)
                    && !self.equipped_melee.contains(&mw)
                {
                    self.equipped_melee.push(mw);
                }
            }
            "remove_melee" => {
                if let Ok(idx) = id.parse::<usize>()
                    && let Some(&mw) = MeleeWeaponId::ALL.get(idx)
                {
                    self.equipped_melee.retain(|&x| x != mw);
                }
            }
            "add_armor" => {
                if let Ok(idx) = id.parse::<usize>()
                    && let Some(&ar) = ArmorProfileId::ALL.get(idx)
                    && !self.equipped_armor.contains(&ar)
                {
                    self.equipped_armor.push(ar);
                }
            }
            "remove_armor" => {
                if let Ok(idx) = id.parse::<usize>()
                    && let Some(&ar) = ArmorProfileId::ALL.get(idx)
                {
                    self.equipped_armor.retain(|&x| x != ar);
                }
            }
            "add_gear" => {
                if !self.field_gear.iter().any(|g| g.id == id)
                    && let Some(item) = lookup_field_gear_preset(id)
                {
                    self.field_gear.push(item);
                }
            }
            "remove_gear" => self.field_gear.retain(|g| g.id != id),
            _ => {}
        }
    }
}

/// Re-renders `#char-sheet-stage-container` and `#char-sheet-status-pill` from thread-local state.
pub fn render_character_sheet() {
    let notes_dom = input_value("char-sheet-notes-textarea");
    with_state_mut(|st| {
        if get_element("char-sheet-notes-textarea").is_some() {
            st.character_sheet.notes = notes_dom;
        }
    });
    let (html, pill_text, preset_id) = with_state(|st| {
        (
            format_character_sheet_html(&st.character_sheet),
            st.character_sheet.format_status_pill_text(),
            st.character_sheet.preset_id.clone(),
        )
    });
    set_text("char-sheet-status-pill", &pill_text);
    crate::wasm_client::dom::set_select_value("char-sheet-preset-select", &preset_id);
    if let Some(el) = get_element("char-sheet-stage-container") {
        el.set_inner_html(&html);
    }
}

/// Synchronizes the Character Creation Wizard (`st.wizard`) into `st.character_sheet`.
pub fn apply_wizard_to_character_sheet(st: &mut crate::wasm_client::state::SatyaWasmState) {
    st.wizard.sync_lifepath_skills();
    let baseline = compute_lifepath_baseline_from_config(
        &st.wizard.lifepath,
        &st.wizard.characteristics,
        &st.wizard.aptitudes,
    );
    let sheet = &mut st.character_sheet;
    let slug = slugify_identifier(&st.wizard.character_name, "custom-wizard");
    sheet.character_id = format!("char-{slug}");
    sheet.preset_id = "custom_wizard".to_string();
    sheet.character_name = st.wizard.character_name.clone();
    sheet.callsign_or_role = format!("Custom Lifepath Operator ({})", baseline.origin.label);
    sheet.age_years = baseline.age_years;
    sheet.origin_label = baseline.origin.label.to_string();
    sheet.campaign_sqm = baseline.campaign_sqm;
    sheet.operational_heat = baseline.operational_heat;
    sheet.granted_access_tags = baseline.granted_access_tags.clone();
    sheet.stage_history = baseline.stage_history.clone();
    sheet.lifepath_config_json = serde_json::to_string(&st.wizard.lifepath).ok();
    sheet.characteristics = st.wizard.characteristics.clone();
    sheet.aptitudes = st.wizard.aptitudes.clone();
    sheet.skills = st.wizard.skills.clone();
    let rec = sheet.to_character_sheet_record("local-operator");
    sheet.verification_status = rec.verification_status;
    sheet.audit_violations = rec.audit_violations;
}

/// Loads a canonical operator preset (`"sara_valkyrie"`, `"mateo_vance"`, `"priya_nair"`, `"amara_okafor"`) into the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_load_preset(preset_id: String) {
    let target = if preset_id.trim().is_empty() {
        select_value("char-sheet-preset-select")
    } else {
        preset_id.trim().to_string()
    };
    if target == "custom_wizard" {
        with_state_mut(apply_wizard_to_character_sheet);
    } else {
        with_state_mut(|st| {
            let prev_filter = st.character_sheet.section_filter.clone();
            let prev_vault = st.character_sheet.saved_vault_list.clone();
            st.character_sheet = CharacterSheetState::from_preset(&target);
            st.character_sheet.section_filter = prev_filter;
            st.character_sheet.saved_vault_list = prev_vault;
        });
    }
    render_character_sheet();
}

/// Switches the visible section filter (`"ALL"`, `"STATS"`, `"SKILLS"`, `"EQUIPMENT"`, `"STATUS"`, `"NOTES"`) on the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_set_section(section: String) {
    with_state_mut(|st| {
        st.character_sheet.section_filter = section.trim().to_ascii_uppercase();
    });
    render_character_sheet();
}

/// Updates the Skill Table tree filter (`"ALL"` or `"TREE_1"`..`"TREE_8"`) and trained-only toggle on the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_set_skill_filter(tree_code: String, trained_only: bool) {
    with_state_mut(|st| {
        st.character_sheet.skill_tree_filter = tree_code.trim().to_ascii_uppercase();
        st.character_sheet.skills_trained_only = trained_only;
    });
    render_character_sheet();
}

/// Selects one of the 13 anatomical hit zones (`0..13`) on the Human-Shaped Hit Location SVG or table.
#[wasm_bindgen]
pub fn satya_char_sheet_select_zone(zone_idx: usize) {
    with_state_mut(|st| {
        if zone_idx < 13 {
            st.character_sheet.selected_hit_zone = zone_idx;
        }
    });
    render_character_sheet();
}

/// Rolls `1d20` on the Master Anatomical Hit Location Table (`HIT_LOCATION_TABLE`) and selects that zone on the human diagram.
#[wasm_bindgen]
pub fn satya_char_sheet_roll_hit_location() {
    let roll = roll_browser_die_i32(20);
    let spec = resolve_hit_location_d20(roll);
    with_state_mut(|st| {
        st.character_sheet.last_hit_location_roll_d20 = Some(roll);
        st.character_sheet.selected_hit_zone = spec.id as usize;
    });
    render_character_sheet();
    append_system_notice(&format!(
        "Rolled 1d20 Hit Location = {} -> {} (Called Shot +{} Δ_ext): {}",
        roll, spec.name, spec.called_shot_delta_ext, spec.mechanical_notes
    ));
}

/// Updates the physical trauma state (`"wound_tier"`, `"bone_status"`, `"bleed_grade"`, `"stabilize_bleeds"`, `"clear_all"`) of an anatomical hit zone.
#[wasm_bindgen]
pub fn satya_char_sheet_update_zone(zone_idx: usize, field: String, value: String) {
    with_state_mut(|st| {
        st.character_sheet.apply_zone_mutation(zone_idx, &field, &value);
    });
    render_character_sheet();
}

/// Adjusts or sets Damage, Fatigue, Exertion, Blood Loss, Morale, or Psyche status fields on the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_update_status(field: String, value: String) {
    let is_sync = field.trim().eq_ignore_ascii_case("sync_to_roller");
    let roller_vals = with_state_mut(|st| {
        st.character_sheet.apply_status_mutation(&field, &value);
        is_sync.then(|| {
            (
                st.character_sheet.total_task_delta_int(),
                st.character_sheet.effective_wound_shock_index(),
            )
        })
    });
    if let Some((delta_int, shock)) = roller_vals {
        set_input_value("ttrpg-internal-mod", &delta_int.to_string());
        set_input_value("ttrpg-wound-shock", &shock.to_string());
        satya_update_cascade_preview();
    }
    render_character_sheet();
}

/// Modifies equipped Ranged Firearms, Melee Weapons, Armor Profiles, Field Gear, or Ballistic Range on the Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_modify_equipment(action: String, item_id: String) {
    let act = action.trim().to_ascii_lowercase();
    let resolved_id = if item_id.trim().is_empty() {
        match act.as_str() {
            "add_ranged" => select_value("char-add-ranged-select"),
            "add_melee" => select_value("char-add-melee-select"),
            "add_armor" => select_value("char-add-armor-select"),
            "add_gear" => select_value("char-add-gear-select"),
            _ => String::new(),
        }
    } else {
        item_id.trim().to_string()
    };
    with_state_mut(|st| {
        st.character_sheet
            .apply_equipment_mutation(&act, &resolved_id);
    });
    render_character_sheet();
}

/// Saves or updates the writable Operator Field Notes (`#char-sheet-notes-textarea`) in [`CharacterSheetState`].
#[wasm_bindgen]
pub fn satya_char_sheet_save_notes(mode: String) {
    let current = input_value("char-sheet-notes-textarea");
    with_state_mut(|st| {
        if mode == "clear" {
            st.character_sheet.notes.clear();
        } else if mode == "stamp_status" {
            let pill = st.character_sheet.format_status_pill_text();
            st.character_sheet.notes = if current.trim().is_empty() {
                format!("[Status Log] {pill}")
            } else {
                format!("{}\n[Status Log] {pill}", current.trim_end())
            };
        } else {
            st.character_sheet.notes = current;
        }
    });
    if mode == "clear" || mode == "stamp_status" {
        render_character_sheet();
    }
}
