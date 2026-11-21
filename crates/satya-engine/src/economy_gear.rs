//! Socioeconomic Origin, Campaign Upkeep (`SQM 1–10`), Black-Market `[Access]` Procurement,
//! Operational `Heat` (`0–10`), Contacts & Favors, Cover Identities, Safehouses, and Post-School
//! Skill Point (`SP`) Ledger & Study Progression
//! (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1, §1.6–§1.7, §3.1–§3.5;
//! `docs/ttrpg/03-character/characteristics_and_aptitudes.md` §9.1.3).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::characteristics::{CharacterAptitudes, CharacterCharacteristics};
use crate::dice_probabilities::{ContestedOutcomeKind, ContestedTieRule};
use crate::lifepath_economy::{LifepathBaselineResult, LifepathBuildConfig};
use crate::lifepath_hobbies_events::{
    HOBBY_CLUB_DISCOUNT_MAX_LEVEL, HOBBY_CLUB_EXPERIENCE_DISCOUNT_MULTIPLIER, PersonalHobbyId,
    is_hobby_club_discounted_skill,
};
use crate::primitives::{ContestedCheck, TieredSaveEval};
use crate::skills_trees::{
    SkillStepCostBreakdown, SkillTrainingMode, compute_skill_step_sp_cost,
    validate_skill_advancement,
};
use crate::{define_coded_enum, impl_static_catalog};

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
fn apply_hobby_club_sp_discount(
    skill_id: &str,
    target_level: usize,
    training_mode: SkillTrainingMode,
    hobbies: &[PersonalHobbyId],
    mut bd: SkillStepCostBreakdown,
) -> SkillStepCostBreakdown {
    if training_mode == SkillTrainingMode::Experience
        && target_level <= HOBBY_CLUB_DISCOUNT_MAX_LEVEL
        && is_hobby_club_discounted_skill(skill_id, hobbies)
    {
        bd.training_mode_multiplier *= HOBBY_CLUB_EXPERIENCE_DISCOUNT_MULTIPLIER;
        let raw_sp = (bd.base_level_sp as f64)
            * bd.difficulty_multiplier
            * bd.controller_multiplier
            * bd.aptitude_multiplier
            * bd.training_mode_multiplier;
        bd.final_sp_cost = (raw_sp.round().max(3.0)) as usize;
    }
    bd
}

/// Computes the deterministic Skill Point (`SP`) expenditure ledger for a post-lifepath skill map
/// relative to `baseline` using any hobbies recorded in `cfg` (`FormalTraining` vs `Experience`, `§1.7`).
#[must_use]
pub fn compute_skill_allocation_sp_ledger(
    baseline: &LifepathBaselineResult,
    cfg: &LifepathBuildConfig,
    chars: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
    skills: &BTreeMap<String, usize>,
) -> SkillAllocationSpLedger {
    let hobbies = cfg.collected_hobbies();
    compute_skill_allocation_sp_ledger_with_hobbies(
        baseline, cfg, &hobbies, chars, aptitudes, skills,
    )
}

/// Computes the deterministic Skill Point (`SP`) expenditure ledger with an explicit slice of
/// active [`PersonalHobbyId`] clubs (`-15%` Experience SP discount on signature skills `N <= 3`).
#[must_use]
pub fn compute_skill_allocation_sp_ledger_with_hobbies(
    baseline: &LifepathBaselineResult,
    cfg: &LifepathBuildConfig,
    hobbies: &[PersonalHobbyId],
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
                        let exp_cost = apply_hobby_club_sp_discount(
                            sk_id,
                            step_n,
                            SkillTrainingMode::Experience,
                            hobbies,
                            compute_skill_step_sp_cost(
                                sk_id,
                                step_n,
                                SkillTrainingMode::Experience,
                                chars,
                                aptitudes,
                            ),
                        )
                        .final_sp_cost;
                        experience_sp_spent += exp_cost;
                        sk_spent += exp_cost;
                    }
                } else {
                    let exp_cost = apply_hobby_club_sp_discount(
                        sk_id,
                        step_n,
                        SkillTrainingMode::Experience,
                        hobbies,
                        compute_skill_step_sp_cost(
                            sk_id,
                            step_n,
                            SkillTrainingMode::Experience,
                            chars,
                            aptitudes,
                        ),
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
    let hobbies = cfg.collected_hobbies();
    preview_next_skill_step_sp_with_hobbies(
        skill_id,
        target_level,
        cfg,
        &hobbies,
        ledger,
        chars,
        aptitudes,
    )
}

/// Preview the [`SkillStepCostBreakdown`] with an explicit slice of active [`PersonalHobbyId`] clubs.
#[must_use]
pub fn preview_next_skill_step_sp_with_hobbies(
    skill_id: &str,
    target_level: usize,
    cfg: &LifepathBuildConfig,
    hobbies: &[PersonalHobbyId],
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
        let exp_bd = apply_hobby_club_sp_discount(
            skill_id,
            target_level,
            SkillTrainingMode::Experience,
            hobbies,
            compute_skill_step_sp_cost(
                skill_id,
                target_level,
                SkillTrainingMode::Experience,
                chars,
                aptitudes,
            ),
        );
        if exp_bd.final_sp_cost <= ledger.experience_sp_remaining {
            return (exp_bd, true);
        }
        (formal_bd, false)
    } else {
        let exp_bd = apply_hobby_club_sp_discount(
            skill_id,
            target_level,
            SkillTrainingMode::Experience,
            hobbies,
            compute_skill_step_sp_cost(
                skill_id,
                target_level,
                SkillTrainingMode::Experience,
                chars,
                aptitudes,
            ),
        );
        let affordable = exp_bd.final_sp_cost <= ledger.experience_sp_remaining
            || exp_bd.final_sp_cost <= ledger.formal_sp_remaining;
        (exp_bd, affordable)
    }
}

/// Automatically allocates available `career_skill_budget` (`Formal SP`) and `elective_skill_budget` (`Experience SP`)
/// across the character's Career Track `recommended_skills()`, hobby signature skills, and universal electives,
/// strictly respecting all 4 canonical DAG gates (`validate_skill_advancement`), the 5-factor SP step cost,
/// and `max_post_school_gain_per_skill` (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6–§1.7).
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

    let hobbies = cfg.collected_hobbies();
    let mut all_candidates: Vec<&'static str> = Vec::new();
    for term in cfg.career_terms.iter().take(4) {
        for &sk_id in term.track.recommended_skills() {
            if !all_candidates.contains(&sk_id) {
                all_candidates.push(sk_id);
            }
        }
    }
    for &hobby in &hobbies {
        for &sk_id in hobby.spec().signature_skill_ids {
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

// ============================================================================
// Phase 2 Expansion (§3.3–§3.5): Contacts & Favors, Cover Identities & Safehouses
// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.3–§3.5)
// ============================================================================

/// Campaign Contact profile (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContactProfile {
    /// Institutional reach / `[Access]` breadth (`1` Street/Local .. `4` National/Boardroom/Agency).
    pub reach_access_tier: u8,
    /// Loyalty & reliability (`1` Transactional .. `5` Ride-or-Die).
    pub reliability_trust: u8,
    /// Current surveillance / compromise exposure (`0` Clean .. `5` Under Active Audit).
    pub exposure_burn_risk: u8,
}

/// Scope and political/logistical risk of a requested favor (`lifepaths_and_economy.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FavorSeverityTier {
    /// Minor: tip-off, local introduction, or routine `[Access I]` lookup (`+0` $\Delta_{\text{ext}}$).
    Minor,
    /// Moderate: forged transit pass, `[Access II–III]` fencing, temporary safe harbor (`+4` $\Delta_{\text{ext}}$).
    Moderate,
    /// Major: scrubbing active arrest warrants, breaking into Class IV networks, high-risk exfil (`+8` $\Delta_{\text{ext}}$).
    Major,
}

impl FavorSeverityTier {
    /// Base hidden external difficulty ($\Delta_{\text{ext}}$) of the favor.
    #[must_use]
    pub const fn base_delta_ext(self) -> i32 {
        match self {
            Self::Minor => 0,
            Self::Moderate => 4,
            Self::Major => 8,
        }
    }
}

/// Resolution outcome of requesting a favor from a campaign Contact (`lifepaths_and_economy.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContactFavorOutcomeKind {
    GrantedCleanlyWithBonus,
    GrantedCleanly,
    GrantedWithReciprocalDebt,
    DeclinedContactIntact,
    BurnedOrCompromised,
}

/// Detailed mechanical output of [`evaluate_contact_favor_request`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContactFavorEffect {
    pub outcome_kind: ContactFavorOutcomeKind,
    pub total_hidden_delta_ext: i32,
    pub favor_granted: bool,
    pub cost_multiplier: f64,
    pub reciprocal_favor_owed: bool,
    pub exposure_increase: u8,
    pub contact_burned: bool,
    pub caller_heat_added: u8,
    pub summary: &'static str,
}

/// Resolve a Contact Favor request using `1-Pair Tiered GM Resistance`
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.3).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_contact_favor_request(
    contact: ContactProfile,
    favor: FavorSeverityTier,
    required_reach_tier: u8,
    jurisdiction_heat: u8,
    requester_skill_n: usize,
    requester_control_c: i32,
    internal_mod: i32,
    player_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<ContactFavorEffect> {
    let trust_clamped = contact.reliability_trust.clamp(1, 5) as i32;
    let reliability_mod = -(trust_clamped - 2) * 2;
    let exposure_mod = contact.exposure_burn_risk.min(5) as i32;
    let heat_mod = if jurisdiction_heat >= 3 {
        (jurisdiction_heat.min(10) - 2) as i32
    } else {
        0
    };
    let reach_shortfall_mod = (required_reach_tier
        .clamp(1, 4)
        .saturating_sub(contact.reach_access_tier.clamp(1, 4))
        as i32)
        * 4;
    let total_hidden_delta_ext =
        favor.base_delta_ext() + reliability_mod + exposure_mod + heat_mod + reach_shortfall_mod;

    let eval = TieredSaveEval::evaluate(
        requester_skill_n,
        requester_control_c,
        internal_mod,
        0,
        total_hidden_delta_ext,
        tie_rule,
        player_pairs,
        gm_pair,
    );

    let effect = eval.classify_4way(
        |_| ContactFavorEffect {
            outcome_kind: ContactFavorOutcomeKind::GrantedCleanlyWithBonus,
            total_hidden_delta_ext,
            favor_granted: true,
            cost_multiplier: 0.75,
            reciprocal_favor_owed: false,
            exposure_increase: 0,
            contact_burned: false,
            caller_heat_added: 0,
            summary: "Trusted Priority Favor: delivered ahead of schedule at 0.75x fee with zero blowback.",
        },
        |roll| match roll.contested_kind {
            ContestedOutcomeKind::CleanSuccess => ContactFavorEffect {
                outcome_kind: ContactFavorOutcomeKind::GrantedCleanly,
                total_hidden_delta_ext,
                favor_granted: true,
                cost_multiplier: 1.0,
                reciprocal_favor_owed: false,
                exposure_increase: if favor == FavorSeverityTier::Major {
                    1
                } else {
                    0
                },
                contact_burned: false,
                caller_heat_added: 0,
                summary: "Favor Granted Cleanly: contact fulfills request at standard rate.",
            },
            _ => ContactFavorEffect {
                outcome_kind: ContactFavorOutcomeKind::GrantedWithReciprocalDebt,
                total_hidden_delta_ext,
                favor_granted: true,
                cost_multiplier: 1.5,
                reciprocal_favor_owed: true,
                exposure_increase: 1,
                contact_burned: false,
                caller_heat_added: 0,
                summary: "Favor Granted at Reciprocal Cost: requires 1.5x fee OR an open return favor owed.",
            },
        },
        |_| ContactFavorEffect {
            outcome_kind: ContactFavorOutcomeKind::DeclinedContactIntact,
            total_hidden_delta_ext,
            favor_granted: false,
            cost_multiplier: 1.0,
            reciprocal_favor_owed: false,
            exposure_increase: 1,
            contact_burned: false,
            caller_heat_added: 0,
            summary: "Politely Refused: heat or reach limits prevented delivery, but relationship stays intact (+1 exposure).",
        },
        |_| ContactFavorEffect {
            outcome_kind: ContactFavorOutcomeKind::BurnedOrCompromised,
            total_hidden_delta_ext,
            favor_granted: false,
            cost_multiplier: 1.5,
            reciprocal_favor_owed: false,
            exposure_increase: 3,
            contact_burned: true,
            caller_heat_added: 2,
            summary: "Contact Burned: surveillance flagged the outreach; contact severed or turned (+2 Heat)!",
        },
    );

    ContestedCheck::new(eval, effect)
}

define_coded_enum! {
    /// Four tiers of Layered Cover Identities & Paper Trails (`lifepaths_and_economy.md` §3.4).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[repr(usize)]
    pub enum CoverIdentityTier [count = 4] {
        /// Tier 1: Paper-Only Burnable ID (`$600`, `PR_id +2`, `Forgery N >= 1`).
        Tier1PaperOnlyBurnableId => {
            code: "COVER_TIER_1_PAPER",
            label: "Tier 1: Paper-Only / Burnable Local ID",
            aliases: ["TIER_1_PAPER_ONLY", "PAPER_ID"],
        },
        /// Tier 2: Database-Seeded Alias (`$3,500`, `PR_id +4`, `Forgery N >= 2`).
        Tier2DatabaseSeededAlias => {
            code: "COVER_TIER_2_DB_ALIAS",
            label: "Tier 2: Database-Seeded Commercial & Transit Alias",
            aliases: ["TIER_2_DATABASE_ALIAS", "DB_ALIAS"],
        },
        /// Tier 3: Deep Biometric / Credit Legend (`$18,000`, `PR_id +6`, `Forgery N >= 3`).
        Tier3DeepBiometricCreditLegend => {
            code: "COVER_TIER_3_BIOMETRIC",
            label: "Tier 3: Deep Biometric, Tax & Credit Legend",
            aliases: ["TIER_3_BIOMETRIC_LEGEND", "BIOMETRIC_COVER"],
        },
        /// Tier 4: State / Syndicate Ghost Cover (`$85,000`, `PR_id +8`, `Forgery N >= 4`).
        Tier4StateSyndicateGhostCover => {
            code: "COVER_TIER_4_GHOST",
            label: "Tier 4: Sovereign / Syndicate Black-Ops Ghost Cover",
            aliases: ["TIER_4_GHOST_COVER", "GHOST_COVER"],
        },
    }
}

/// Canonical specification for a Cover Identity tier (`lifepaths_and_economy.md` §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CoverIdentitySpec {
    pub tier: CoverIdentityTier,
    pub cost_usd: u32,
    pub access_class: BlackMarketRestrictionClass,
    pub forgery_n_min: usize,
    pub scrutiny_resistance_pr_id: i32,
    pub biometric_linked: bool,
    pub flag_heat_threshold: u8,
    pub burn_heat_threshold: u8,
    pub clean_burn_heat_shed: u8,
}

const COVER_IDENTITY_CATALOG_ARRAY: [CoverIdentitySpec; 4] = [
    CoverIdentitySpec {
        tier: CoverIdentityTier::Tier1PaperOnlyBurnableId,
        cost_usd: 600,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
        forgery_n_min: 1,
        scrutiny_resistance_pr_id: 2,
        biometric_linked: false,
        flag_heat_threshold: 4,
        burn_heat_threshold: 6,
        clean_burn_heat_shed: 2,
    },
    CoverIdentitySpec {
        tier: CoverIdentityTier::Tier2DatabaseSeededAlias,
        cost_usd: 3_500,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
        forgery_n_min: 2,
        scrutiny_resistance_pr_id: 4,
        biometric_linked: false,
        flag_heat_threshold: 6,
        burn_heat_threshold: 9,
        clean_burn_heat_shed: 4,
    },
    CoverIdentitySpec {
        tier: CoverIdentityTier::Tier3DeepBiometricCreditLegend,
        cost_usd: 18_000,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
        forgery_n_min: 3,
        scrutiny_resistance_pr_id: 6,
        biometric_linked: true,
        flag_heat_threshold: 6,
        burn_heat_threshold: 9,
        clean_burn_heat_shed: 4,
    },
    CoverIdentitySpec {
        tier: CoverIdentityTier::Tier4StateSyndicateGhostCover,
        cost_usd: 85_000,
        access_class: BlackMarketRestrictionClass::Class4StrategicStateSecret,
        forgery_n_min: 4,
        scrutiny_resistance_pr_id: 8,
        biometric_linked: true,
        flag_heat_threshold: 7,
        burn_heat_threshold: 10,
        clean_burn_heat_shed: 6,
    },
];

impl_static_catalog!(CoverIdentityTier => CoverIdentitySpec : COVER_IDENTITY_CATALOG_ARRAY);

/// Canonical 4-tier Cover Identity reference catalog (`lifepaths_and_economy.md` §3.4).
pub const COVER_IDENTITY_CATALOG: &[CoverIdentitySpec] = &COVER_IDENTITY_CATALOG_ARRAY;

/// Return the full canonical 4-tier Cover Identity catalog.
#[must_use]
pub const fn canonical_cover_identity_catalog() -> &'static [CoverIdentitySpec] {
    COVER_IDENTITY_CATALOG
}

/// Checkpoint / authority inspection depth against a Cover Identity (`lifepaths_and_economy.md` §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoverScrutinyLevel {
    /// Bouncer, hotel clerk, or glance at physical card (`+0` base).
    CasualVisualCheck,
    /// Police traffic stop, bank counter, or transit gate (`+4` base).
    DatabaseOrBorderTerminal,
    /// Federal/corporate iris-gait biometric cross-check (`+8` base; `+6` extra if cover is non-biometric).
    BiometricAuditOrFederalCrossCheck,
}

impl CoverScrutinyLevel {
    /// Base inspection difficulty ($\Delta_{\text{ext}}$).
    #[must_use]
    pub const fn delta_ext(self) -> i32 {
        match self {
            Self::CasualVisualCheck => 0,
            Self::DatabaseOrBorderTerminal => 4,
            Self::BiometricAuditOrFederalCrossCheck => 8,
        }
    }
}

/// Resolution output for a Cover Identity inspection or checkpoint test (`lifepaths_and_economy.md` §3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverIdentityScrutinyEffect {
    pub tier: CoverIdentityTier,
    pub passed: bool,
    pub total_hidden_delta_ext: i32,
    pub cover_flagged: bool,
    pub cover_burned: bool,
    pub cover_heat_added: u8,
    pub personal_heat_bleed: u8,
    pub summary: &'static str,
}

/// Evaluate passing a checkpoint, background check, or biometric terminal using a Cover Identity
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.4).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_cover_identity_scrutiny(
    tier: CoverIdentityTier,
    scrutiny: CoverScrutinyLevel,
    cover_heat: u8,
    holder_skill_n: usize,
    holder_control_c: i32,
    internal_mod: i32,
    player_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<CoverIdentityScrutinyEffect> {
    let spec = tier.spec();
    let flagged_penalty = if cover_heat >= spec.flag_heat_threshold {
        4
    } else {
        0
    };
    let already_burned = cover_heat >= spec.burn_heat_threshold;
    let biometric_mismatch_penalty = if scrutiny
        == CoverScrutinyLevel::BiometricAuditOrFederalCrossCheck
        && !spec.biometric_linked
    {
        6
    } else {
        0
    };
    let total_hidden_delta_ext = scrutiny.delta_ext()
        + flagged_penalty
        + biometric_mismatch_penalty
        + if already_burned { 8 } else { 0 }
        - spec.scrutiny_resistance_pr_id;

    let eval = TieredSaveEval::evaluate(
        holder_skill_n,
        holder_control_c,
        internal_mod,
        0,
        total_hidden_delta_ext,
        tie_rule,
        player_pairs,
        gm_pair,
    );

    let is_extraordinary =
        eval.contested_roll.contested_kind == ContestedOutcomeKind::ExtraordinarySuccess;
    let effect = if already_burned && !is_extraordinary {
        CoverIdentityScrutinyEffect {
            tier,
            passed: false,
            total_hidden_delta_ext,
            cover_flagged: true,
            cover_burned: true,
            cover_heat_added: 4,
            personal_heat_bleed: 2,
            summary: "Burned Cover Detected: terminal immediately flagged dead alias; detain alert triggered!",
        }
    } else {
        eval.classify_4way(
            |_| CoverIdentityScrutinyEffect {
                tier,
                passed: true,
                total_hidden_delta_ext,
                cover_flagged: false,
                cover_burned: false,
                cover_heat_added: 0,
                personal_heat_bleed: 0,
                summary: "Verified Clean: cover legend cleared inspection with zero audit residue.",
            },
            |roll| match roll.contested_kind {
                ContestedOutcomeKind::CleanSuccess => CoverIdentityScrutinyEffect {
                    tier,
                    passed: true,
                    total_hidden_delta_ext,
                    cover_flagged: false,
                    cover_burned: false,
                    cover_heat_added: 0,
                    personal_heat_bleed: 0,
                    summary: "Passed Inspection: credentials accepted as authentic.",
                },
                _ => CoverIdentityScrutinyEffect {
                    tier,
                    passed: true,
                    total_hidden_delta_ext,
                    cover_flagged: true,
                    cover_burned: false,
                    cover_heat_added: 2,
                    personal_heat_bleed: 0,
                    summary: "Waved Through With Flag: checkpoint logged a secondary review (+2 Cover Heat).",
                },
            },
            |_| CoverIdentityScrutinyEffect {
                tier,
                passed: false,
                total_hidden_delta_ext,
                cover_flagged: true,
                cover_burned: false,
                cover_heat_added: 3,
                personal_heat_bleed: 1,
                summary: "Inspection Rejected: manual supervisor hold triggered (+3 Cover Heat, +1 Personal Heat).",
            },
            |_| CoverIdentityScrutinyEffect {
                tier,
                passed: false,
                total_hidden_delta_ext,
                cover_flagged: true,
                cover_burned: true,
                cover_heat_added: 5,
                personal_heat_bleed: 3,
                summary: "Biometric/Paper Trail Mismatch: cover burned and cross-linked (+3 Personal Heat)!",
            },
        )
    };

    ContestedCheck::new(eval, effect)
}

/// Outcome of proactively burning and discarding a Cover Identity (`lifepaths_and_economy.md` §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverBurnResult {
    pub tier: CoverIdentityTier,
    pub previous_personal_heat: u8,
    pub heat_shed: u8,
    pub new_personal_heat: u8,
    pub biometric_crosslinked: bool,
}

/// Proactively burn a Cover Identity before federal cross-linking to deflect pursuit Heat
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.4).
#[must_use]
pub const fn evaluate_cover_identity_burn(
    tier: CoverIdentityTier,
    current_personal_heat: u8,
    biometric_crosslinked: bool,
) -> CoverBurnResult {
    let spec = tier.spec();
    let heat_shed = if biometric_crosslinked {
        spec.clean_burn_heat_shed / 2
    } else {
        spec.clean_burn_heat_shed
    };
    let clamped_heat = if current_personal_heat > 10 {
        10
    } else {
        current_personal_heat
    };
    CoverBurnResult {
        tier,
        previous_personal_heat: clamped_heat,
        heat_shed,
        new_personal_heat: clamped_heat.saturating_sub(heat_shed),
        biometric_crosslinked,
    }
}

define_coded_enum! {
    /// Four tiers of Operational Safehouses & Bolt-Holes (`lifepaths_and_economy.md` §3.5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[repr(usize)]
    pub enum SafehouseTier [count = 4] {
        /// Cash-only motel / off-grid squat (`$450/mo`, no Faraday, `+0` breach protection).
        OffGridSquatMotel => {
            code: "SAFEHOUSE_SQUAT_MOTEL",
            label: "Off-Grid Squat / Cash Motel",
            aliases: ["SQUAT_MOTEL", "TIER_1_SAFEHOUSE"],
        },
        /// Subleased shell apartment (`$2,400/mo`, `+4` lock breach, basic trauma kit `-2`).
        SubleasedShellApartment => {
            code: "SAFEHOUSE_SHELL_APT",
            label: "Subleased Shell Apartment",
            aliases: ["SHELL_APARTMENT", "TIER_2_SAFEHOUSE"],
        },
        /// Hardened Faraday workshop (`$8,500/mo`, Faraday RF shield, `+6` breach, `-2` workshop & medical).
        HardenedFaradayWorkshop => {
            code: "SAFEHOUSE_FARADAY_SHOP",
            label: "Hardened Faraday Workshop & Bolt-Hole",
            aliases: ["FARADAY_WORKSHOP", "TIER_3_SAFEHOUSE"],
        },
        /// Sovereign/corporate redoubt (`$35,000/mo`, Faraday RF shield, `+8` breach, `-4` workshop & surgical suite).
        SovereignCorporateRedoubt => {
            code: "SAFEHOUSE_REDOUBT",
            label: "Sovereign / Corporate Redoubt",
            aliases: ["CORPORATE_REDOUBT", "TIER_4_SAFEHOUSE"],
        },
    }
}

/// Canonical specification for a Safehouse tier (`lifepaths_and_economy.md` §3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SafehouseSpec {
    pub tier: SafehouseTier,
    pub min_sqm_upkeep: u8,
    pub monthly_cost_usd: u32,
    pub physical_breach_delta_ext: i32,
    pub faraday_rf_shielded: bool,
    pub workshop_tool_delta_mod: i32,
    pub medical_stabilization_delta_mod: i32,
    pub compromise_resistance_bonus: i32,
}

const SAFEHOUSE_CATALOG_ARRAY: [SafehouseSpec; 4] = [
    SafehouseSpec {
        tier: SafehouseTier::OffGridSquatMotel,
        min_sqm_upkeep: 1,
        monthly_cost_usd: 450,
        physical_breach_delta_ext: 0,
        faraday_rf_shielded: false,
        workshop_tool_delta_mod: 0,
        medical_stabilization_delta_mod: 0,
        compromise_resistance_bonus: 0,
    },
    SafehouseSpec {
        tier: SafehouseTier::SubleasedShellApartment,
        min_sqm_upkeep: 4,
        monthly_cost_usd: 2_400,
        physical_breach_delta_ext: 4,
        faraday_rf_shielded: false,
        workshop_tool_delta_mod: 0,
        medical_stabilization_delta_mod: -2,
        compromise_resistance_bonus: 2,
    },
    SafehouseSpec {
        tier: SafehouseTier::HardenedFaradayWorkshop,
        min_sqm_upkeep: 6,
        monthly_cost_usd: 8_500,
        physical_breach_delta_ext: 6,
        faraday_rf_shielded: true,
        workshop_tool_delta_mod: -2,
        medical_stabilization_delta_mod: -2,
        compromise_resistance_bonus: 4,
    },
    SafehouseSpec {
        tier: SafehouseTier::SovereignCorporateRedoubt,
        min_sqm_upkeep: 8,
        monthly_cost_usd: 35_000,
        physical_breach_delta_ext: 8,
        faraday_rf_shielded: true,
        workshop_tool_delta_mod: -4,
        medical_stabilization_delta_mod: -4,
        compromise_resistance_bonus: 6,
    },
];

impl_static_catalog!(SafehouseTier => SafehouseSpec : SAFEHOUSE_CATALOG_ARRAY);

/// Canonical 4-tier Safehouse & Bolt-Hole reference catalog (`lifepaths_and_economy.md` §3.5).
pub const SAFEHOUSE_CATALOG: &[SafehouseSpec] = &SAFEHOUSE_CATALOG_ARRAY;

/// Return the full canonical 4-tier Safehouse catalog.
#[must_use]
pub const fn canonical_safehouse_catalog() -> &'static [SafehouseSpec] {
    SAFEHOUSE_CATALOG
}

/// Compromise posture of a Safehouse after operatives return hot (`lifepaths_and_economy.md` §3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SafehouseCompromiseStatus {
    SecureUndetected,
    WatchedSurveillancePlanted,
    CompromisedRaidImminent,
}

/// Detailed result of [`evaluate_safehouse_compromise_check`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafehouseCompromiseEffect {
    pub tier: SafehouseTier,
    pub status: SafehouseCompromiseStatus,
    pub total_hidden_delta_ext: i32,
    pub faraday_blocked_rf_leak: bool,
    pub warning_lead_min: i32,
    pub must_abandon: bool,
    pub summary: &'static str,
}

/// Evaluate whether returning hot (`Heat >= 4`), trailed, or with active RF devices compromises
/// a Safehouse (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.5).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_safehouse_compromise_check(
    tier: SafehouseTier,
    returning_heat: u8,
    actively_tailed: bool,
    rf_hygiene_breached: bool,
    operator_skill_n: usize,
    operator_control_c: i32,
    internal_mod: i32,
    operator_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<SafehouseCompromiseEffect> {
    let spec = tier.spec();
    let heat_penalty = if returning_heat >= 4 {
        (returning_heat.min(10) - 3) as i32 * 2
    } else {
        0
    };
    let tail_penalty = if actively_tailed { 4 } else { 0 };
    let faraday_blocked_rf_leak = rf_hygiene_breached && spec.faraday_rf_shielded;
    let rf_penalty = if rf_hygiene_breached && !spec.faraday_rf_shielded {
        4
    } else {
        0
    };
    let total_hidden_delta_ext =
        heat_penalty + tail_penalty + rf_penalty - spec.compromise_resistance_bonus;

    let eval = TieredSaveEval::evaluate(
        operator_skill_n,
        operator_control_c,
        internal_mod,
        0,
        total_hidden_delta_ext,
        tie_rule,
        operator_pairs,
        gm_pair,
    );

    let effect = eval.classify_4way(
        |_| SafehouseCompromiseEffect {
            tier,
            status: SafehouseCompromiseStatus::SecureUndetected,
            total_hidden_delta_ext,
            faraday_blocked_rf_leak,
            warning_lead_min: 0,
            must_abandon: false,
            summary: "Safehouse Cold & Secure: countersurveillance protocols scrubbed any pursuit vector.",
        },
        |roll| match roll.contested_kind {
            ContestedOutcomeKind::CleanSuccess => SafehouseCompromiseEffect {
                tier,
                status: SafehouseCompromiseStatus::SecureUndetected,
                total_hidden_delta_ext,
                faraday_blocked_rf_leak,
                warning_lead_min: 0,
                must_abandon: false,
                summary: "Safehouse Remains Uncompromised: no hostile spotters logged location.",
            },
            _ => SafehouseCompromiseEffect {
                tier,
                status: SafehouseCompromiseStatus::WatchedSurveillancePlanted,
                total_hidden_delta_ext,
                faraday_blocked_rf_leak,
                warning_lead_min: 15,
                must_abandon: false,
                summary: "Street Block Flagged: hostile scouts narrowed neighborhood; vacate before next daylight shift (15 min lead).",
            },
        },
        |_| SafehouseCompromiseEffect {
            tier,
            status: SafehouseCompromiseStatus::WatchedSurveillancePlanted,
            total_hidden_delta_ext,
            faraday_blocked_rf_leak,
            warning_lead_min: 5,
            must_abandon: true,
            summary: "Safehouse Surveilled: drones/van spotted ingress; 5 minutes to exfil via bolt-hole.",
        },
        |_| SafehouseCompromiseEffect {
            tier,
            status: SafehouseCompromiseStatus::CompromisedRaidImminent,
            total_hidden_delta_ext,
            faraday_blocked_rf_leak,
            warning_lead_min: 2,
            must_abandon: true,
            summary: "Compromised — Strike Team En Route: breach imminent in 2 minutes!",
        },
    );

    ContestedCheck::new(eval, effect)
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
                    ..Default::default()
                },
                CareerTermConfig {
                    track: CareerTermTrack::MilitaryCombatArmsSpecOps,
                    term_roll_pairs: [(13, 10), (12, 9)],
                    override_successes: Some(1),
                    waiver_override: false,
                    ..Default::default()
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
