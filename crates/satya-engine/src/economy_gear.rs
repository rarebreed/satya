//! Socioeconomic Origin, Campaign Upkeep (`SQM 1–10`), Black-Market `[Access]` Procurement,
//! Operational `Heat` (`0–10`), and Post-School Skill Point (`SP`) Ledger & Study Progression
//! (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1, §1.6–§1.7, §3.1–§3.2;
//! `docs/ttrpg/03-character/characteristics_and_aptitudes.md` §9.1.3).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::characteristics::{CharacterAptitudes, CharacterCharacteristics};
use crate::define_coded_enum;
use crate::lifepath_economy::{LifepathBaselineResult, LifepathBuildConfig};
use crate::skills_trees::{
    SkillStepCostBreakdown, SkillTrainingMode, compute_skill_step_sp_cost,
    validate_skill_advancement,
};

define_coded_enum! {
    /// Socioeconomic Class from Stage 1 (`Ages 0–5`) (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SocioeconomicClass [count = 6] {
        /// `2–7`: Destitute / Urban Underclass / War Zone (`SQM +4`, Dropout Risk `2–16`).
        DestituteUnderclassWarZone => {
            code: "DESTITUTE",
            label: "Destitute / Underclass / War Zone",
            aliases: ["DESTITUTE_UNDERCLASS_WAR_ZONE", "DESTITUTE_URBAN_UNDERCLASS"],
        },
        /// `8–14`: Working Poor / Inner-City Lower Class (`SQM +2`, Dropout Risk `2–13`).
        WorkingPoorLowerClass => {
            code: "WORKING_POOR",
            label: "Working Poor / Inner-City Lower Class",
            aliases: ["WORKING_POOR_LOWER_CLASS"],
        },
        /// `15–26`: Working / Lower-Middle Class (`SQM 0`, Dropout Risk `2–9`).
        WorkingMiddleClass => {
            code: "MIDDLE_CLASS",
            label: "Working / Middle Class",
            aliases: ["WORKING_MIDDLE_CLASS", "WORKING_LOWER_MIDDLE_CLASS"],
        },
        /// `27–33`: Professional / Middle Class (`SQM -2`, Dropout Risk `2–6`).
        ProfessionalUpperMiddleClass => {
            code: "UPPER_MIDDLE",
            label: "Professional / Upper-Middle Class",
            aliases: ["PROFESSIONAL_UPPER_MIDDLE_CLASS", "PROFESSIONAL_MIDDLE_CLASS"],
        },
        /// `34–38`: Upper-Middle / Technocratic Class (`SQM -4`, Dropout Risk `2–4`).
        AffluentElitePreparatory => {
            code: "AFFLUENT_PREP",
            label: "Affluent / Elite Preparatory",
            aliases: ["AFFLUENT_ELITE_PREPARATORY", "UPPER_MIDDLE_TECHNOCRATIC_CLASS"],
        },
        /// `39–40`: Wealthy / Corporate or Aristocratic Elite (`SQM -6`, Dropout Risk `2`).
        WealthyCorporateAristocraticElite => {
            code: "WEALTHY_ELITE",
            label: "Wealthy / Corporate or Aristocratic Elite",
            aliases: ["WEALTHY_CORPORATE_ARISTOCRATIC_ELITE"],
        },
    }
}

impl SocioeconomicClass {
    /// Representative `2d20` roll (`2..=40`) that maps to this Socioeconomic Class in [`resolve_socioeconomic_origin`]
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1).
    #[must_use]
    pub const fn representative_roll_2d20(self) -> i32 {
        match self {
            Self::DestituteUnderclassWarZone => 4,
            Self::WorkingPoorLowerClass => 8,
            Self::WorkingMiddleClass => 18,
            Self::ProfessionalUpperMiddleClass => 30,
            Self::AffluentElitePreparatory => 36,
            Self::WealthyCorporateAristocraticElite => 40,
        }
    }

    /// Default campaign `SQM` tier (`1–10`) associated with this socioeconomic origin
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1, §3.1).
    #[must_use]
    pub const fn default_campaign_sqm(self) -> u8 {
        match self {
            Self::DestituteUnderclassWarZone => 2,
            Self::WorkingPoorLowerClass => 4,
            Self::WorkingMiddleClass => 5,
            Self::ProfessionalUpperMiddleClass => 7,
            Self::AffluentElitePreparatory => 8,
            Self::WealthyCorporateAristocraticElite => 9,
        }
    }
}

/// Complete Stage 1 Socioeconomic Origin specification (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SocioeconomicOriginSpec {
    pub class: SocioeconomicClass,
    pub label: &'static str,
    pub school_quality_modifier_sqm: i32,
    /// Eligible early-environment universal talent bonus choices (`+1` rank at Stage 1).
    pub early_environment_bonus_talents: &'static [&'static str],
    pub adolescent_dropout_risk_max_2d20: i32,
}

/// Resolve Stage 1 Socioeconomic Origin from a `2d20` roll (`2..=40`)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1).
#[must_use]
pub const fn resolve_socioeconomic_origin(roll_2d20: i32) -> SocioeconomicOriginSpec {
    match roll_2d20 {
        ..=7 => SocioeconomicOriginSpec {
            class: SocioeconomicClass::DestituteUnderclassWarZone,
            label: "Destitute / Underclass / War Zone",
            school_quality_modifier_sqm: 4,
            early_environment_bonus_talents: &[
                "alertness_situational_awareness",
                "stealth_sneaking_hiding",
                "brawling_unarmed_striking",
                "foraging_campcraft",
            ],
            adolescent_dropout_risk_max_2d20: 16,
        },
        8..=14 => SocioeconomicOriginSpec {
            class: SocioeconomicClass::WorkingPoorLowerClass,
            label: "Working Poor / Inner-City Lower Class",
            school_quality_modifier_sqm: 2,
            early_environment_bonus_talents: &[
                "alertness_situational_awareness",
                "intimidation_coercion",
                "deception_bluff",
                "manual_dexterity_tinkering",
            ],
            adolescent_dropout_risk_max_2d20: 13,
        },
        15..=26 => SocioeconomicOriginSpec {
            class: SocioeconomicClass::WorkingMiddleClass,
            label: "Working / Middle Class",
            school_quality_modifier_sqm: 0,
            early_environment_bonus_talents: &[
                "counting_basic_numeracy",
                "manual_dexterity_tinkering",
                "running_sprinting",
                "empathy_social_perception",
            ],
            adolescent_dropout_risk_max_2d20: 9,
        },
        27..=33 => SocioeconomicOriginSpec {
            class: SocioeconomicClass::ProfessionalUpperMiddleClass,
            label: "Professional / Upper-Middle Class",
            school_quality_modifier_sqm: -2,
            early_environment_bonus_talents: &[
                "counting_basic_numeracy",
                "persuasion_fast_talk",
                "sketching_doodling",
                "singing_vocal_rhythm",
            ],
            adolescent_dropout_risk_max_2d20: 6,
        },
        34..=38 => SocioeconomicOriginSpec {
            class: SocioeconomicClass::AffluentElitePreparatory,
            label: "Affluent / Elite Preparatory",
            school_quality_modifier_sqm: -4,
            early_environment_bonus_talents: &[
                "counting_basic_numeracy",
                "persuasion_fast_talk",
                "empathy_social_perception",
            ],
            adolescent_dropout_risk_max_2d20: 4,
        },
        39.. => SocioeconomicOriginSpec {
            class: SocioeconomicClass::WealthyCorporateAristocraticElite,
            label: "Wealthy / Corporate or Aristocratic Elite",
            school_quality_modifier_sqm: -6,
            early_environment_bonus_talents: &[
                "counting_basic_numeracy",
                "persuasion_fast_talk",
                "foreign_language_specific",
            ],
            adolescent_dropout_risk_max_2d20: 2,
        },
    }
}

/// Campaign Socioeconomic Status & Upkeep Tier (`SQM 1–10`)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CampaignSqmSpec {
    /// Clamped campaign socioeconomic tier (`1..=10`).
    pub sqm_tier: u8,
    pub tier_name: &'static str,
    pub monthly_liquid_baseline_usd: u32,
    /// Daily internal fatigue penalty (`\Delta_int`) from unhoused exposure.
    pub unhoused_fatigue_delta_int: i32,
    pub school_quality_modifier: i32,
}

/// Look up the canonical Campaign `SQM 1–10` Upkeep & Lifestyle specification
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.1).
#[must_use]
pub const fn lookup_campaign_sqm(sqm_tier: u8) -> CampaignSqmSpec {
    match sqm_tier {
        ..=2 => CampaignSqmSpec {
            sqm_tier: if sqm_tier == 0 { 1 } else { sqm_tier },
            tier_name: "Destitute / Unhoused / Off-Grid Squat",
            monthly_liquid_baseline_usd: 400,
            unhoused_fatigue_delta_int: 1,
            school_quality_modifier: 4,
        },
        3..=4 => CampaignSqmSpec {
            sqm_tier,
            tier_name: "Working Class / Hustler",
            monthly_liquid_baseline_usd: 3_000,
            unhoused_fatigue_delta_int: 0,
            school_quality_modifier: 2,
        },
        5..=6 => CampaignSqmSpec {
            sqm_tier,
            tier_name: "Middle Class / Salaried Professional",
            monthly_liquid_baseline_usd: 9_000,
            unhoused_fatigue_delta_int: 0,
            school_quality_modifier: 0,
        },
        7..=8 => CampaignSqmSpec {
            sqm_tier,
            tier_name: "Affluent / Corporate Executive / Syndicate Boss",
            monthly_liquid_baseline_usd: 50_000,
            unhoused_fatigue_delta_int: 0,
            school_quality_modifier: -2,
        },
        9.. => CampaignSqmSpec {
            sqm_tier: if sqm_tier > 10 { 10 } else { sqm_tier },
            tier_name: "Ultra-High Net Worth / Sovereign Oligarch",
            monthly_liquid_baseline_usd: 500_000,
            unhoused_fatigue_delta_int: 0,
            school_quality_modifier: -4,
        },
    }
}

define_coded_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum BlackMarketRestrictionClass [count = 4] {
        /// Class I: Licensed Civilian (`+0` $\Delta_{\text{ext}}$, `1.2x–1.5x` cost).
        Class1LicensedCivilian => {
            code: "CLASS_I_CIVILIAN",
            label: "Class I: Licensed Civilian",
            aliases: ["CLASS_1_LICENSED_CIVILIAN", "CLASS_I"],
        },
        /// Class II: Law Enforcement / Paramilitary (`+2` $\Delta_{\text{ext}}$, `2x–3x` cost).
        Class2LawEnforcementParamilitary => {
            code: "CLASS_II_LAW_ENFORCEMENT",
            label: "Class II: Law Enforcement / Paramilitary",
            aliases: ["CLASS_2_LAW_ENFORCEMENT_PARAMILITARY", "CLASS_II"],
        },
        /// Class III: Military Ordnance / Full-Auto / Explosives (`+4` $\Delta_{\text{ext}}$, `3x–5x` cost).
        Class3MilitaryOrdnanceFullAuto => {
            code: "CLASS_III_MILITARY",
            label: "Class III: Military Ordnance / Full-Auto / Explosives",
            aliases: ["CLASS_3_MILITARY_ORDNANCE_FULL_AUTO", "CLASS_III"],
        },
        /// Class IV: Strategic / State Secret / Bioweapons (`+8` $\Delta_{\text{ext}}$, `10x+` cost).
        Class4StrategicStateSecret => {
            code: "CLASS_IV_STRATEGIC",
            label: "Class IV: Strategic / State Secret / Bioweapons",
            aliases: ["CLASS_4_STRATEGIC_STATE_SECRET", "CLASS_IV"],
        },
    }
}

impl BlackMarketRestrictionClass {
    /// External difficulty modifier ($\Delta_{\text{ext}}$) on the Black-Market procurement roll
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.2).
    #[must_use]
    pub const fn procurement_delta_ext(self) -> i32 {
        match self {
            Self::Class1LicensedCivilian => 0,
            Self::Class2LawEnforcementParamilitary => 2,
            Self::Class3MilitaryOrdnanceFullAuto => 4,
            Self::Class4StrategicStateSecret => 8,
        }
    }

    /// Typical black-market price multiplier midpoint (`1.35x`, `2.5x`, `4.0x`, `10.0x`)
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.2).
    #[must_use]
    pub const fn typical_price_multiplier(self) -> f64 {
        match self {
            Self::Class1LicensedCivilian => 1.35,
            Self::Class2LawEnforcementParamilitary => 2.5,
            Self::Class3MilitaryOrdnanceFullAuto => 4.0,
            Self::Class4StrategicStateSecret => 10.0,
        }
    }
}

define_coded_enum! {
    /// Operational `Heat` & Law Enforcement Response Tier (`0–10`)
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum OperationalHeatTier [count = 5] {
        /// `Heat 0`: Clean — no active warrants or heightened scrutiny.
        Heat0Clean => {
            code: "HEAT_0_CLEAN",
            label: "Heat 0: Clean",
            aliases: ["CLEAN"],
        },
        /// `Heat 1–2`: Local Beat Inquiry — routine patrol questions, minor surveillance.
        Heat1To2LocalBeatInquiry => {
            code: "HEAT_1_2_INQUIRY",
            label: "Heat 1–2: Local Beat Inquiry",
            aliases: ["HEAT_1_TO_2_LOCAL_BEAT_INQUIRY"],
        },
        /// `Heat 3–5`: Active Warrant / APB — `+4` $\Delta_{\text{ext}}$ to all legal/public social & travel checks.
        Heat3To5ActiveWarrantApb => {
            code: "HEAT_3_5_WARRANT",
            label: "Heat 3–5: Active Warrant / APB",
            aliases: ["HEAT_3_TO_5_ACTIVE_WARRANT_APB"],
        },
        /// `Heat 6–8`: Tactical Task Force / Strike Team — SWAT/military raids; legal `SQM` bank accounts frozen!
        Heat6To8TacticalTaskForce => {
            code: "HEAT_6_8_TASK_FORCE",
            label: "Heat 6–8: Tactical Task Force / Strike Team",
            aliases: ["HEAT_6_TO_8_TACTICAL_TASK_FORCE"],
        },
        /// `Heat 9–10`: Burned / National Manhunt — shoot-on-sight directives; total underground existence required.
        Heat9To10BurnedNationalManhunt => {
            code: "HEAT_9_10_BURNED",
            label: "Heat 9–10: Burned / National Manhunt",
            aliases: ["HEAT_9_TO_10_BURNED_NATIONAL_MANHUNT"],
        },
    }
}

/// Mechanical effects of a character's current Operational `Heat` (`0–10`)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationalHeatStatus {
    /// Clamped operational heat score (`0..=10`).
    pub heat_score: u8,
    pub tier: OperationalHeatTier,
    /// External difficulty modifier (`\Delta_ext`) applied to legal/public social and travel checks.
    pub public_check_delta_ext: i32,
    pub legal_bank_accounts_frozen: bool,
    pub description: &'static str,
}

/// Evaluate the mechanical effects of Operational `Heat` (`0..=10`)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.2).
#[must_use]
pub const fn evaluate_operational_heat(heat_score: u8) -> OperationalHeatStatus {
    let clamped = if heat_score > 10 { 10 } else { heat_score };
    match clamped {
        0 => OperationalHeatStatus {
            heat_score: 0,
            tier: OperationalHeatTier::Heat0Clean,
            public_check_delta_ext: 0,
            legal_bank_accounts_frozen: false,
            description: "Clean: No active warrants or heightened scrutiny.",
        },
        1..=2 => OperationalHeatStatus {
            heat_score: clamped,
            tier: OperationalHeatTier::Heat1To2LocalBeatInquiry,
            public_check_delta_ext: 1,
            legal_bank_accounts_frozen: false,
            description: "Local Beat Inquiry: Routine patrol questions and minor surveillance.",
        },
        3..=5 => OperationalHeatStatus {
            heat_score: clamped,
            tier: OperationalHeatTier::Heat3To5ActiveWarrantApb,
            public_check_delta_ext: 4,
            legal_bank_accounts_frozen: false,
            description: "Active Warrant / APB: Facial recognition alerts; +4 Delta_ext to legal/public social and travel checks.",
        },
        6..=8 => OperationalHeatStatus {
            heat_score: clamped,
            tier: OperationalHeatTier::Heat6To8TacticalTaskForce,
            public_check_delta_ext: 6,
            legal_bank_accounts_frozen: true,
            description: "Tactical Task Force / Strike Team: SWAT raids on known safehouses; legal SQM bank accounts frozen.",
        },
        9.. => OperationalHeatStatus {
            heat_score: clamped,
            tier: OperationalHeatTier::Heat9To10BurnedNationalManhunt,
            public_check_delta_ext: 10,
            legal_bank_accounts_frozen: true,
            description: "Burned / National Manhunt: Counter-terror / intelligence shoot-on-sight directives; total underground existence required.",
        },
    }
}

/// Returns true if `skill_id` is part of the formal curriculum (`recommended_skills()`) of any active Stage 5 Career Term
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6–§1.7).
#[must_use]
pub fn is_career_formal_skill(skill_id: &str, cfg: &LifepathBuildConfig) -> bool {
    cfg.career_terms
        .iter()
        .take(4)
        .any(|t| t.track.recommended_skills().contains(&skill_id))
}

/// Ledger tracking Formal Training SP (`FOC + INS`) and Experience SP (`INS`) spent across post-school skill advancements
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillAllocationSpLedger {
    pub formal_sp_budget: usize,
    pub experience_sp_budget: usize,
    pub formal_sp_spent: usize,
    pub experience_sp_spent: usize,
    pub formal_sp_remaining: usize,
    pub experience_sp_remaining: usize,
    pub total_sp_budget: usize,
    pub total_sp_spent: usize,
    pub total_sp_remaining: usize,
    /// Total skill ranks gained above the Stage 1–4 baseline floor.
    pub total_levels_gained: usize,
    /// Total SP invested in each skill above its Stage 1–4 baseline floor.
    pub per_skill_sp_spent: BTreeMap<String, usize>,
}

/// Computes the exact Formal Training SP (`FOC + INS`) and Experience SP (`INS`) spent and remaining
/// for a character's current `skills` map relative to their `baseline.baseline_skills` floor
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7).
///
/// - Skills in any active Career Term's `recommended_skills()` draw from `Formal SP` first (using
///   [`SkillTrainingMode::FormalTraining`]); if a step exceeds `formal_sp_remaining`, it spills over into
///   `Experience SP` (using [`SkillTrainingMode::Experience`]).
/// - Non-career elective/hobby skills draw directly from `Experience SP` (using [`SkillTrainingMode::Experience`]),
///   or from `Formal SP` if `Experience SP` is exhausted.
#[must_use]
pub fn compute_skill_allocation_sp_ledger(
    baseline: &LifepathBaselineResult,
    cfg: &LifepathBuildConfig,
    chars: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
    skills: &BTreeMap<String, usize>,
) -> SkillAllocationSpLedger {
    let formal_sp_budget = baseline.career_skill_budget;
    let experience_sp_budget = baseline.elective_skill_budget;
    let mut formal_sp_spent = 0usize;
    let mut experience_sp_spent = 0usize;
    let mut total_levels_gained = 0usize;
    let mut per_skill_sp_spent = BTreeMap::new();

    for pass_career in [true, false] {
        for (sk_id, &cur_n) in skills {
            let floor_n = baseline.baseline_skills.get(sk_id).copied().unwrap_or(0);
            if cur_n <= floor_n || is_career_formal_skill(sk_id, cfg) != pass_career {
                continue;
            }
            let mut sk_spent = 0usize;
            for step_n in (floor_n + 1)..=cur_n {
                total_levels_gained += 1;
                if pass_career {
                    let formal_cost = compute_skill_step_sp_cost(
                        sk_id,
                        step_n,
                        SkillTrainingMode::FormalTraining,
                        chars,
                        aptitudes,
                    )
                    .final_sp_cost;
                    if formal_sp_spent + formal_cost <= formal_sp_budget {
                        formal_sp_spent += formal_cost;
                        sk_spent += formal_cost;
                    } else {
                        let exp_cost = compute_skill_step_sp_cost(
                            sk_id,
                            step_n,
                            SkillTrainingMode::Experience,
                            chars,
                            aptitudes,
                        )
                        .final_sp_cost;
                        experience_sp_spent += exp_cost;
                        sk_spent += exp_cost;
                    }
                } else {
                    let exp_cost = compute_skill_step_sp_cost(
                        sk_id,
                        step_n,
                        SkillTrainingMode::Experience,
                        chars,
                        aptitudes,
                    )
                    .final_sp_cost;
                    if experience_sp_spent + exp_cost <= experience_sp_budget {
                        experience_sp_spent += exp_cost;
                    } else if formal_sp_spent + exp_cost <= formal_sp_budget {
                        formal_sp_spent += exp_cost;
                    } else {
                        experience_sp_spent += exp_cost;
                    }
                    sk_spent += exp_cost;
                }
            }
            per_skill_sp_spent.insert(sk_id.clone(), sk_spent);
        }
    }

    let formal_sp_remaining = formal_sp_budget.saturating_sub(formal_sp_spent);
    let experience_sp_remaining = experience_sp_budget.saturating_sub(experience_sp_spent);
    let total_sp_budget = formal_sp_budget + experience_sp_budget;
    let total_sp_spent = formal_sp_spent + experience_sp_spent;
    let total_sp_remaining = formal_sp_remaining + experience_sp_remaining;

    SkillAllocationSpLedger {
        formal_sp_budget,
        experience_sp_budget,
        formal_sp_spent,
        experience_sp_spent,
        formal_sp_remaining,
        experience_sp_remaining,
        total_sp_budget,
        total_sp_spent,
        total_sp_remaining,
        total_levels_gained,
        per_skill_sp_spent,
    }
}

/// Preview the [`SkillStepCostBreakdown`] and whether the character has enough SP to advance `skill_id` to `target_level`
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7).
#[must_use]
pub fn preview_next_skill_step_sp(
    skill_id: &str,
    target_level: usize,
    cfg: &LifepathBuildConfig,
    ledger: &SkillAllocationSpLedger,
    chars: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
) -> (SkillStepCostBreakdown, bool) {
    if is_career_formal_skill(skill_id, cfg) {
        let formal_bd = compute_skill_step_sp_cost(
            skill_id,
            target_level,
            SkillTrainingMode::FormalTraining,
            chars,
            aptitudes,
        );
        if formal_bd.final_sp_cost <= ledger.formal_sp_remaining {
            return (formal_bd, true);
        }
        let exp_bd = compute_skill_step_sp_cost(
            skill_id,
            target_level,
            SkillTrainingMode::Experience,
            chars,
            aptitudes,
        );
        if exp_bd.final_sp_cost <= ledger.experience_sp_remaining {
            return (exp_bd, true);
        }
        (formal_bd, false)
    } else {
        let exp_bd = compute_skill_step_sp_cost(
            skill_id,
            target_level,
            SkillTrainingMode::Experience,
            chars,
            aptitudes,
        );
        let affordable = exp_bd.final_sp_cost <= ledger.experience_sp_remaining
            || exp_bd.final_sp_cost <= ledger.formal_sp_remaining;
        (exp_bd, affordable)
    }
}

/// Automatically allocates available `career_skill_budget` (`Formal SP`) and `elective_skill_budget` (`Experience SP`)
/// across the character's Career Track `recommended_skills()` and universal electives, strictly respecting all 4 canonical
/// DAG gates (`validate_skill_advancement`), the 5-factor SP step cost, and `max_post_school_gain_per_skill`
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6–§1.7).
pub fn auto_allocate_career_and_elective_skills(
    baseline: &LifepathBaselineResult,
    cfg: &LifepathBuildConfig,
    chars: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
    skills: &mut BTreeMap<String, usize>,
) {
    for (k, &v) in &baseline.baseline_skills {
        let cur = skills.entry(k.clone()).or_insert(0);
        if *cur < v {
            *cur = v;
        }
    }

    let mut all_candidates: Vec<&'static str> = Vec::new();
    for term in cfg.career_terms.iter().take(4) {
        for &sk_id in term.track.recommended_skills() {
            if !all_candidates.contains(&sk_id) {
                all_candidates.push(sk_id);
            }
        }
    }
    const FALLBACK_ELECTIVES: [&str; 11] = [
        "swimming_diving",
        "alertness_situational_awareness",
        "first_aid_bls_cpr",
        "driving_wheeled_tracked",
        "digital_ai_literacy",
        "stealth_sneaking_hiding",
        "persuasion_fast_talk",
        "empathy_social_perception",
        "running_sprinting",
        "brawling_unarmed_striking",
        "manual_dexterity_tinkering",
    ];
    for &sk_id in &FALLBACK_ELECTIVES {
        if !all_candidates.contains(&sk_id) {
            all_candidates.push(sk_id);
        }
    }

    for _pass in 0..6 {
        let mut made_progress = false;
        for &sk_id in &all_candidates {
            let cur_n = skills.get(sk_id).copied().unwrap_or(0);
            let floor_n = baseline.baseline_skills.get(sk_id).copied().unwrap_or(0);
            if cur_n.saturating_sub(floor_n) >= baseline.max_post_school_gain_per_skill
                || cur_n >= 5
            {
                continue;
            }
            let target_n = cur_n + 1;
            let validation = validate_skill_advancement(
                sk_id,
                target_n,
                skills,
                chars,
                aptitudes,
                &baseline.granted_access_tags,
                baseline.academic_tree_frozen,
            );
            if !validation.is_valid {
                continue;
            }
            let ledger =
                compute_skill_allocation_sp_ledger(baseline, cfg, chars, aptitudes, skills);
            let (_, affordable) =
                preview_next_skill_step_sp(sk_id, target_n, cfg, &ledger, chars, aptitudes);
            if affordable {
                skills.insert(sk_id.to_string(), target_n);
                let post_ledger =
                    compute_skill_allocation_sp_ledger(baseline, cfg, chars, aptitudes, skills);
                if post_ledger.formal_sp_spent <= post_ledger.formal_sp_budget
                    && post_ledger.experience_sp_spent <= post_ledger.experience_sp_budget
                {
                    made_progress = true;
                } else if cur_n == 0 {
                    skills.remove(sk_id);
                } else {
                    skills.insert(sk_id.to_string(), cur_n);
                }
            }
        }
        if !made_progress {
            break;
        }
    }
}

/// Summary of Lifepath exponential skill progression (`N in 0..=10`) from study hours
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7; `docs/ttrpg/03-character/characteristics_and_aptitudes.md` §9.1.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifepathSkillProgression {
    pub raw_hours: f64,
    pub effective_hours: f64,
    pub skill_level_n: i32,
    pub next_level_n: i32,
    pub hours_to_next_level: f64,
    /// Cumulative effective hour thresholds for each rank `1..=10`.
    pub cumulative_thresholds: BTreeMap<i32, f64>,
}

fn round_1_dec(val: f64) -> f64 {
    (val * 10.0).round() / 10.0
}

/// Computes non-linear post-creation skill level $N \in [0, 10]$ from study hours using the
/// canonical Aptitude training time multiplier (`-15%` per `+1` APT, `+20%` per `-1` APT;
/// `docs/ttrpg/03-character/characteristics_and_aptitudes.md` §9.1.3).
///
/// Each rank $N \ge 1$ requires $100 \times 2^{N - 1} \times \text{difficulty\_multiplier} \times \text{aptitude\_time\_multiplier}$ hours.
#[must_use]
pub fn compute_lifepath_skill_progression(
    hours_studied: f64,
    difficulty_multiplier: f64,
    instruction_quality: f64,
    aptitude_bonus: i32,
) -> LifepathSkillProgression {
    let apt_time_mult =
        crate::characteristics::compute_aptitude_training_time_multiplier(aptitude_bonus);
    let eff_hours =
        (hours_studied.max(0.0) * instruction_quality.max(0.25)) / apt_time_mult.max(0.25);
    let diff = difficulty_multiplier.max(0.5);
    let mut cumulative_thresholds = BTreeMap::new();
    let mut cumulative = 0.0;
    let mut achieved_level = 0;

    for lvl in 1..=10 {
        let step_req = 100.0 * (2.0_f64.powi(lvl - 1)) * diff;
        cumulative += step_req;
        let rounded_cum = round_1_dec(cumulative);
        cumulative_thresholds.insert(lvl, rounded_cum);
        if eff_hours >= cumulative {
            achieved_level = lvl;
        }
    }

    let next_level = (achieved_level + 1).min(10);
    let next_threshold = cumulative_thresholds
        .get(&next_level)
        .copied()
        .unwrap_or(0.0);
    let hours_to_next = round_1_dec((next_threshold - eff_hours).max(0.0));

    LifepathSkillProgression {
        raw_hours: round_1_dec(hours_studied),
        effective_hours: round_1_dec(eff_hours),
        skill_level_n: achieved_level,
        next_level_n: next_level,
        hours_to_next_level: hours_to_next,
        cumulative_thresholds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AptitudeCode;
    use crate::lifepath_economy::*;
    use crate::skills_trees::SkillAccessTier;

    #[test]
    fn test_interactive_lifepath_builder_and_auto_skill_allocation() {
        let chars = CharacterCharacteristics {
            ana: 14,
            dis: 13,
            mem: 12,
            foc: 13,
            ins: 13,
            for_stat: 11,
            dex: 13,
            sig: 12,
            fit: 11,
            hlt: 11,
            ..Default::default()
        };

        let mut aptitudes = CharacterAptitudes::default();
        aptitudes.set(AptitudeCode::AptQnt, 2);
        aptitudes.set(AptitudeCode::AptFin, 1);

        let cfg = LifepathBuildConfig {
            origin_roll_2d20: 28,
            stage4a_hs_track: HighSchoolTrack::ApHonorsStemMagnet,
            career_terms: vec![
                CareerTermConfig {
                    track: CareerTermTrack::UniversityUndergraduateStem,
                    term_roll_pairs: [(14, 12), (13, 11)],
                    override_successes: Some(2),
                    waiver_override: false,
                },
                CareerTermConfig {
                    track: CareerTermTrack::MilitaryCombatArmsSpecOps,
                    term_roll_pairs: [(13, 10), (12, 9)],
                    override_successes: Some(1),
                    waiver_override: false,
                },
            ],
            ..Default::default()
        };

        let baseline = compute_lifepath_baseline_from_config(&cfg, &chars, &aptitudes);
        assert_eq!(baseline.age_years, 26);
        assert!(!baseline.academic_tree_frozen);
        assert!(
            baseline
                .granted_access_tags
                .contains(&SkillAccessTier::Academic)
        );
        assert!(
            baseline
                .granted_access_tags
                .contains(&SkillAccessTier::Restricted)
        );
        assert_eq!(baseline.career_skill_budget, 160 + 140);
        assert_eq!(baseline.elective_skill_budget, 40 + 80);

        let mut skills = baseline.baseline_skills.clone();
        auto_allocate_career_and_elective_skills(&baseline, &cfg, &chars, &aptitudes, &mut skills);

        let ledger =
            compute_skill_allocation_sp_ledger(&baseline, &cfg, &chars, &aptitudes, &skills);
        assert!(ledger.total_levels_gained >= 12);
        assert!(ledger.formal_sp_spent <= ledger.formal_sp_budget);
        assert!(ledger.experience_sp_spent <= ledger.experience_sp_budget);
        assert!(ledger.total_sp_spent >= 350);
    }

    #[test]
    fn test_mateo_vance_walkthrough_and_academic_freeze() {
        let mateo = build_mateo_vance_lifepath();
        assert_eq!(mateo.origin.school_quality_modifier_sqm, 2);

        let edu = evaluate_educational_advancement_roll(11, 13, 3, 2, [(10, 9), (11, 8)]);
        assert_eq!(edu.c_study, 15);
        assert_eq!(edu.tn_edu, 18);
        assert_eq!(edu.successes, 2);
        assert_eq!(
            edu.outcome_tier,
            EducationalAdvancementTier::HonorsAcceleratedBreakthrough
        );

        assert_eq!(mateo.skills.get("trigonometry"), Some(&2));
        assert_eq!(mateo.skills.get("fully_automatic_burst_weapons"), Some(&2));
        assert!(mateo.academic_tree_frozen);

        let calc_audit = mateo.audit_skill_unlock("calculus_linear_algebra", 1);
        assert!(!calc_audit.is_valid);
        assert!(
            calc_audit
                .failures
                .iter()
                .any(|f| f.code == "MISSING_ACCESS_GATE")
        );
    }

    #[test]
    fn test_priya_nair_walkthrough_and_full_auto_three_gate_lockout() {
        let priya = build_priya_nair_lifepath();
        assert_eq!(
            priya.skills.get("machine_learning_ai_engineering"),
            Some(&5)
        );
        assert_eq!(
            priya.skills.get("aerospace_naval_robotics_engineering"),
            Some(&4)
        );

        let full_auto_audit = priya.audit_skill_unlock("fully_automatic_burst_weapons", 1);
        assert!(!full_auto_audit.is_valid);
        assert!(
            full_auto_audit
                .failures
                .iter()
                .any(|f| f.code == "MISSING_ACCESS_GATE")
        );
        assert!(
            full_auto_audit
                .failures
                .iter()
                .any(|f| f.code == "MIN_CHARACTERISTIC_SHORTFALL")
        );
        assert!(
            full_auto_audit
                .failures
                .iter()
                .any(|f| f.code == "PARENT_PREREQ_SHORTFALL")
        );
    }

    #[test]
    fn test_campaign_sqm_and_operational_heat() {
        let origin_7 = resolve_socioeconomic_origin(7);
        assert_eq!(
            origin_7.class,
            SocioeconomicClass::DestituteUnderclassWarZone
        );
        assert_eq!(origin_7.adolescent_dropout_risk_max_2d20, 16);

        let origin_14 = resolve_socioeconomic_origin(14);
        assert_eq!(origin_14.class, SocioeconomicClass::WorkingPoorLowerClass);
        assert_eq!(origin_14.adolescent_dropout_risk_max_2d20, 13);

        let origin_26 = resolve_socioeconomic_origin(26);
        assert_eq!(origin_26.class, SocioeconomicClass::WorkingMiddleClass);
        assert_eq!(origin_26.adolescent_dropout_risk_max_2d20, 9);

        let origin_33 = resolve_socioeconomic_origin(33);
        assert_eq!(
            origin_33.class,
            SocioeconomicClass::ProfessionalUpperMiddleClass
        );
        assert_eq!(origin_33.adolescent_dropout_risk_max_2d20, 6);

        let origin_38 = resolve_socioeconomic_origin(38);
        assert_eq!(
            origin_38.class,
            SocioeconomicClass::AffluentElitePreparatory
        );
        assert_eq!(origin_38.adolescent_dropout_risk_max_2d20, 4);

        let origin_39 = resolve_socioeconomic_origin(39);
        assert_eq!(
            origin_39.class,
            SocioeconomicClass::WealthyCorporateAristocraticElite
        );
        assert_eq!(origin_39.adolescent_dropout_risk_max_2d20, 2);

        let sqm1 = lookup_campaign_sqm(1);
        assert_eq!(sqm1.tier_name, "Destitute / Unhoused / Off-Grid Squat");
        assert_eq!(sqm1.monthly_liquid_baseline_usd, 400);
        assert_eq!(sqm1.unhoused_fatigue_delta_int, 1);

        let sqm4 = lookup_campaign_sqm(4);
        assert_eq!(sqm4.tier_name, "Working Class / Hustler");
        assert_eq!(sqm4.monthly_liquid_baseline_usd, 3_000);

        let sqm6 = lookup_campaign_sqm(6);
        assert_eq!(sqm6.tier_name, "Middle Class / Salaried Professional");
        assert_eq!(sqm6.monthly_liquid_baseline_usd, 9_000);

        let sqm8 = lookup_campaign_sqm(8);
        assert_eq!(
            sqm8.tier_name,
            "Affluent / Corporate Executive / Syndicate Boss"
        );
        assert_eq!(sqm8.monthly_liquid_baseline_usd, 50_000);

        let sqm10 = lookup_campaign_sqm(10);
        assert_eq!(sqm10.tier_name, "Ultra-High Net Worth / Sovereign Oligarch");
        assert_eq!(sqm10.monthly_liquid_baseline_usd, 500_000);

        let heat4 = evaluate_operational_heat(4);
        assert_eq!(heat4.tier, OperationalHeatTier::Heat3To5ActiveWarrantApb);
        assert_eq!(heat4.public_check_delta_ext, 4);
        assert!(!heat4.legal_bank_accounts_frozen);

        let heat7 = evaluate_operational_heat(7);
        assert!(heat7.legal_bank_accounts_frozen);
    }

    #[test]
    fn test_amara_okafor_rural_lifepath_and_skills() {
        let amara = build_amara_okafor_lifepath();
        assert_eq!(amara.origin.school_quality_modifier_sqm, 4);
        assert!(amara.academic_tree_frozen);
        assert!(
            amara
                .granted_access_tags
                .contains(&SkillAccessTier::VocationalLicensed)
        );
        assert_eq!(amara.operational_heat, 0);

        assert_eq!(
            amara.skills.get("hunting_trapping_field_butchery"),
            Some(&4)
        );
        assert_eq!(
            amara.skills.get("animal_handling_husbandry_riding"),
            Some(&2)
        );
        assert_eq!(
            amara.skills.get("herbalism_ethnobotany_natural_remedies"),
            Some(&2)
        );
        assert_eq!(
            amara.skills.get("veterinary_medicine_livestock_care"),
            Some(&2)
        );

        let hunt_audit = amara.audit_skill_unlock("hunting_trapping_field_butchery", 5);
        assert!(hunt_audit.is_valid);
        let vet_audit = amara.audit_skill_unlock("veterinary_medicine_livestock_care", 3);
        assert!(vet_audit.is_valid);
        let org_chem_audit = amara.audit_skill_unlock("organic_analytical_chemistry", 1);
        assert!(!org_chem_audit.is_valid);
    }
}
