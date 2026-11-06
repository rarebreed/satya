//! Chronological 5-Stage Lifepath Creation, Socioeconomic Upkeep, Black-Market Procurement & Operational Heat
//! (`03-character/lifepaths_and_economy.md`).
//!
//! Implements:
//! - Stage 1: Socioeconomic Origin & School Quality Modifier (`SQM`) (`2d20` table) (§1.1)
//! - Educational Advancement Roll (`2d20` cascade with $N = 2$ pairs vs. $\text{TN}_{\text{edu}} = 31 - C_{\text{study}} + \text{SQM}$) (§1.2)
//! - Stages 2, 3, 4A/4B (High School Tracks vs. Dropout **Academic Tree Freeze** & Street Gang Archetypes),
//!   and Stage 5 Adult 4-Year Career Terms (§1.3–§1.6)
//! - Canonical Character Walkthrough Builders & Skill Lock Auditors (**Mateo Vance** §2.1 & **Dr. Priya Nair** §2.2)
//! - Campaign Socioeconomic Upkeep (`SQM 1–10`), Black-Market `[Access]` Procurement (`Class I–IV`),
//!   and Operational `Heat` (`0–10`) (§3.1–§3.2)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::characteristics::{
    AptitudeCode, CharacterAptitudes, CharacterCharacteristics, compute_academic_study_controller,
};
use crate::skills_trees::{
    SkillAccessTier, SkillAdvancementValidation, initialize_universal_talents,
    validate_skill_advancement,
};

/// Socioeconomic Class from Stage 1 (`Ages 0–5`) (§1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SocioeconomicClass {
    /// `2–5`: Destitute / Underclass / War Zone (`SQM +4`, Dropout Risk `2–16`)
    DestituteUnderclassWarZone,
    /// `6–11`: Working Poor / Inner-City Lower Class (`SQM +2`, Dropout Risk `2–12`)
    WorkingPoorLowerClass,
    /// `12–25`: Working / Middle Class (`SQM 0`, Dropout Risk `2–8`)
    WorkingMiddleClass,
    /// `26–33`: Professional / Upper-Middle Class (`SQM -2`, Dropout Risk `2–5`)
    ProfessionalUpperMiddleClass,
    /// `34–38`: Affluent / Elite Preparatory (`SQM -4`, Dropout Risk `2–3`)
    AffluentElitePreparatory,
    /// `39–40`: Wealthy / Corporate or Aristocratic Elite (`SQM -6`, Dropout Risk `2`)
    WealthyCorporateAristocraticElite,
}

/// Complete Stage 1 Socioeconomic Origin specification (§1.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SocioeconomicOriginSpec {
    pub class: SocioeconomicClass,
    pub label: &'static str,
    pub school_quality_modifier_sqm: i32,
    pub early_environment_bonus_talents: &'static [&'static str],
    pub adolescent_dropout_risk_max_2d20: i32,
}

/// Resolve Stage 1 Socioeconomic Origin from a `2d20` roll (`2..=40`) (§1.1).
#[must_use]
pub const fn resolve_socioeconomic_origin(roll_2d20: i32) -> SocioeconomicOriginSpec {
    match roll_2d20 {
        ..=5 => SocioeconomicOriginSpec {
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
        6..=11 => SocioeconomicOriginSpec {
            class: SocioeconomicClass::WorkingPoorLowerClass,
            label: "Working Poor / Inner-City Lower Class",
            school_quality_modifier_sqm: 2,
            early_environment_bonus_talents: &[
                "alertness_situational_awareness",
                "intimidation_coercion",
                "deception_bluff",
                "manual_dexterity_tinkering",
            ],
            adolescent_dropout_risk_max_2d20: 12,
        },
        12..=25 => SocioeconomicOriginSpec {
            class: SocioeconomicClass::WorkingMiddleClass,
            label: "Working / Middle Class",
            school_quality_modifier_sqm: 0,
            early_environment_bonus_talents: &[
                "counting_basic_numeracy",
                "manual_dexterity_tinkering",
                "running_sprinting",
                "empathy_social_perception",
            ],
            adolescent_dropout_risk_max_2d20: 8,
        },
        26..=33 => SocioeconomicOriginSpec {
            class: SocioeconomicClass::ProfessionalUpperMiddleClass,
            label: "Professional / Upper-Middle Class",
            school_quality_modifier_sqm: -2,
            early_environment_bonus_talents: &[
                "counting_basic_numeracy",
                "persuasion_fast_talk",
                "sketching_doodling",
                "singing_vocal_rhythm",
            ],
            adolescent_dropout_risk_max_2d20: 5,
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
            adolescent_dropout_risk_max_2d20: 3,
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

/// Outcome tier of an Educational Advancement Roll (`2d20` with $N = 2$ pairs) (§1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EducationalAdvancementTier {
    /// `0 Successes`: Stagnation / Remedial (`+0` levels in the subject)
    StagnationOrRemedial,
    /// `1 Success`: Standard Advancement (`+1` level in target node, or unlock next tier at $N = 1$)
    StandardAdvancement,
    /// `2+ Successes`: Honors / Accelerated Breakthrough (`+1` level in current node AND early unlock of next tier at $N = 1$, or `+2` levels split)
    HonorsAcceleratedBreakthrough,
}

/// Detailed result of evaluating a Lifepath Educational Advancement Roll (§1.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EducationalAdvancementResult {
    pub c_study: i32,
    pub school_quality_modifier_sqm: i32,
    pub tn_edu: i32,
    pub pair_sums: [i32; 2],
    pub successes: usize,
    pub outcome_tier: EducationalAdvancementTier,
    pub skill_levels_gained: usize,
    pub grants_early_tier_unlock: bool,
}

/// Evaluate a Lifepath Educational Advancement Roll (`N = 2` pairs of `2d20` against $\text{TN}_{\text{edu}} = 31 - C_{\text{study}} + \text{SQM}$) (§1.2).
///
/// Each pair succeeds if `d1 + d2 >= tn_edu` (and is not a double-1 botch; double-20 counts as 2 successes).
#[must_use]
pub fn evaluate_educational_advancement_roll(
    dis: i32,
    c_mental: i32,
    aptitude_domain: i32,
    school_quality_modifier_sqm: i32,
    pair_rolls: [(i32, i32); 2],
) -> EducationalAdvancementResult {
    let c_study = compute_academic_study_controller(dis, c_mental, aptitude_domain);
    let tn_edu = (31 - c_study + school_quality_modifier_sqm).clamp(2, 40);

    let mut successes = 0usize;
    let mut pair_sums = [0i32; 2];

    for (idx, (d1, d2)) in pair_rolls.iter().copied().enumerate() {
        let c1 = d1.clamp(1, 20);
        let c2 = d2.clamp(1, 20);
        let sum = c1 + c2;
        pair_sums[idx] = sum;
        if c1 == 20 && c2 == 20 {
            successes += 2;
        } else if !(c1 == 1 && c2 == 1) && sum >= tn_edu {
            successes += 1;
        }
    }

    let (outcome_tier, skill_levels_gained, grants_early_tier_unlock) = match successes {
        0 => (EducationalAdvancementTier::StagnationOrRemedial, 0, false),
        1 => (EducationalAdvancementTier::StandardAdvancement, 1, false),
        _ => (
            EducationalAdvancementTier::HonorsAcceleratedBreakthrough,
            2,
            true,
        ),
    };

    EducationalAdvancementResult {
        c_study,
        school_quality_modifier_sqm,
        tn_edu,
        pair_sums,
        successes,
        outcome_tier,
        skill_levels_gained,
        grants_early_tier_unlock,
    }
}

/// Stage 4A High School Track (`Ages 16–18`) (§1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HighSchoolTrack {
    /// Track 1: Standard / College-Prep High School (`[Access: Academic]`)
    StandardCollegePrep,
    /// Track 2: AP / Honors / STEM Magnet Track (requires `Elementary Algebra >= 2` and `C_study >= 13`, `SQM -1` bonus)
    ApHonorsStemMagnet,
    /// Track 3: Vocational / Technical High School (unlocks `[Access: Vocational/Licensed]`)
    VocationalTechnical,
}

/// Stage 4B Dropout / Street Gang / Juvenile Underworld Archetype (`Ages 16–18`) (§1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StreetGangArchetype {
    /// *Shooter / Enforcer*: `Short Blades` or `Pistols` to $N=2$ (and `Fully Automatic Weapons` to $N=1$ if `FOR >= 10`), `Intimidation` to $N=2$
    ShooterEnforcer,
    /// *Boost / Wheelman*: `Driving` to $N=2$, `Lockpicking & Physical Intrusion` to $N=2$, `Automotive Mechanics` to $N=1$
    BoostWheelman,
    /// *Runner / Lookout*: `Stealth` to $N=2$, `Alertness` to $N=2$, `Running` or `Acrobatics/Parkour` to $N=2$, `Sleight of Hand` to $N=1$
    RunnerLookout,
    /// *Back-Alley Cook / Modder*: `Pharmacology & Toxicology` (illicit synthesis) or `Armourer & Gunsmithing` to $N=2$
    BackAlleyCookModder,
}

/// Stage 5 Adult 4-Year Career Term Track (`Ages 18+`) (§1.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CareerTermTrack {
    UniversityUndergraduateStem,
    MedicalSchoolAndResidency,
    MilitaryCombatArmsSpecOps,
    LawEnforcementSwatDetective,
    IntelligenceServiceClandestine,
    OrganizedCrimeCartelSyndicate,
    BlueCollarTradesMerchantMarine,
}

impl CareerTermTrack {
    /// Institutional `[Access]` tags granted upon entering this Career Term (§1.6).
    #[must_use]
    pub const fn granted_access_tags(self) -> &'static [SkillAccessTier] {
        match self {
            Self::UniversityUndergraduateStem | Self::MedicalSchoolAndResidency => &[
                SkillAccessTier::Academic,
                SkillAccessTier::VocationalLicensed,
            ],
            Self::MilitaryCombatArmsSpecOps
            | Self::LawEnforcementSwatDetective
            | Self::IntelligenceServiceClandestine => &[
                SkillAccessTier::Restricted,
                SkillAccessTier::VocationalLicensed,
            ],
            Self::OrganizedCrimeCartelSyndicate => &[SkillAccessTier::Illicit],
            Self::BlueCollarTradesMerchantMarine => &[SkillAccessTier::VocationalLicensed],
        }
    }

    /// Maximum skill level advancement (`+1` to `+2`) per 4-year term in a single node (§1.6).
    #[must_use]
    pub const fn max_level_gain_per_term(self) -> usize {
        2
    }
}

/// Complete Lifepath Character Profile produced by the 5-Stage Chronological Lifepath Engine (§1–§2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LifepathCharacterProfile {
    pub name: String,
    pub age_years: u32,
    pub origin: SocioeconomicOriginSpec,
    pub characteristics: CharacterCharacteristics,
    pub aptitudes: CharacterAptitudes,
    pub skills: BTreeMap<String, usize>,
    pub granted_access_tags: Vec<SkillAccessTier>,
    /// True if the character dropped out in Stage 3 / 4B without returning to adult basic education (locks `[Access: Academic]`).
    pub academic_tree_frozen: bool,
    pub stage_history: Vec<String>,
    pub campaign_sqm: u8,
    pub operational_heat: u8,
}

impl LifepathCharacterProfile {
    /// Initialize a new character at Stage 1 (`Ages 0–5`) and Stage 2 (`Ages 6–11`) baseline (§1.1, §1.3).
    #[must_use]
    pub fn new_with_stage2_baseline(
        name: impl Into<String>,
        origin_roll_2d20: i32,
        early_talent_choice: &str,
        characteristics: CharacterCharacteristics,
        aptitudes: CharacterAptitudes,
    ) -> Self {
        let origin = resolve_socioeconomic_origin(origin_roll_2d20);
        let mut skills = initialize_universal_talents();

        // Stage 1 Early Environment Talent Bonus (+1 level, bringing chosen TAL from N=1 to N=2)
        if let Some(level) = skills.get_mut(early_talent_choice) {
            *level = (*level + 1).max(2);
        }

        // Stage 2 Automatic Elementary School Grants (N = 1)
        skills.insert("reading_comprehension".to_string(), 1);
        skills.insert("writing_composition".to_string(), 1);
        skills.insert("arithmetic".to_string(), 1);
        skills.insert("digital_ai_literacy".to_string(), 1);

        let granted_access_tags = vec![SkillAccessTier::Universal, SkillAccessTier::Academic];
        let stage_history = vec![
            format!(
                "Stage 1 (Ages 0–5): {} (SQM {:+}) — Early Talent '{}' -> N=2",
                origin.label, origin.school_quality_modifier_sqm, early_talent_choice
            ),
            "Stage 2 (Ages 6–11): Elementary School Grants (Reading, Writing, Arithmetic, Digital & AI Literacy at N=1)".to_string(),
        ];

        Self {
            name: name.into(),
            age_years: 11,
            origin,
            characteristics,
            aptitudes,
            skills,
            granted_access_tags,
            academic_tree_frozen: false,
            stage_history,
            campaign_sqm: 5,
            operational_heat: 0,
        }
    }

    /// Apply Stage 4B Dropout / Street Gang Transition, triggering the **Academic Tree Freeze** (§1.5).
    pub fn apply_stage4b_dropout_gang_package(&mut self, archetype: StreetGangArchetype) {
        self.age_years = 18;
        self.academic_tree_frozen = true;
        self.granted_access_tags
            .retain(|tag| *tag != SkillAccessTier::Academic);
        if !self.granted_access_tags.contains(&SkillAccessTier::Illicit) {
            self.granted_access_tags.push(SkillAccessTier::Illicit);
        }

        // Common Street Gang grants
        let sw = self
            .skills
            .entry("streetwise_underworld_etiquette".to_string())
            .or_insert(0);
        *sw = (*sw).max(2);

        match archetype {
            StreetGangArchetype::ShooterEnforcer => {
                self.skills.insert("pistols_handguns".to_string(), 2);
                self.skills
                    .insert("short_blades_knife_fighting".to_string(), 2);
                self.skills.insert("intimidation_coercion".to_string(), 2);
                if self.characteristics.for_stat >= 10 {
                    self.skills
                        .insert("fully_automatic_burst_weapons".to_string(), 1);
                }
            }
            StreetGangArchetype::BoostWheelman => {
                self.skills.insert("driving_wheeled_tracked".to_string(), 2);
                self.skills
                    .insert("lockpicking_physical_intrusion".to_string(), 2);
                self.skills
                    .insert("automotive_small_engine_mechanics".to_string(), 1);
            }
            StreetGangArchetype::RunnerLookout => {
                self.skills.insert("stealth_sneaking_hiding".to_string(), 2);
                self.skills
                    .insert("alertness_situational_awareness".to_string(), 2);
                self.skills
                    .insert("acrobatics_tumbling_parkour".to_string(), 2);
                self.skills
                    .insert("sleight_of_hand_pickpocketing_palming".to_string(), 1);
            }
            StreetGangArchetype::BackAlleyCookModder => {
                self.skills.insert("armourer_gunsmithing".to_string(), 2);
                self.skills.insert("pharmacology_toxicology".to_string(), 2);
            }
        }

        self.stage_history.push(format!(
            "Stage 4B (Ages 16–18): Dropped out into {:?} — Academic Tree Frozen; granted [Access: Illicit]",
            archetype
        ));
    }

    /// Audit whether this character can unlock or advance `skill_id` to `target_level` (§2.1, §2.2).
    #[must_use]
    pub fn audit_skill_unlock(
        &self,
        skill_id: &str,
        target_level: usize,
    ) -> SkillAdvancementValidation {
        validate_skill_advancement(
            skill_id,
            target_level,
            &self.skills,
            &self.characteristics,
            &self.aptitudes,
            &self.granted_access_tags,
            self.academic_tree_frozen,
        )
    }
}

/// Build the canonical walkthrough profile for **Mateo Vance — The Gifted Dropout & Gang Enforcer** (§2.1 of `lifepaths_and_economy.md`).
#[must_use]
pub fn build_mateo_vance_lifepath() -> LifepathCharacterProfile {
    let mut chars = CharacterCharacteristics::default();
    chars.ana = 13;
    chars.dis = 11;
    chars.spd = 14;
    chars.dex = 13;
    chars.for_stat = 11;
    chars.itu = 13;
    chars.pre = 12;

    let mut aptitudes = CharacterAptitudes::default();
    aptitudes.set(AptitudeCode::AptQnt, 3);
    aptitudes.set(AptitudeCode::AptFin, 1);

    // Stage 1: Working Poor (roll 8 -> SQM +2), early talent Alertness -> N=2
    let mut mateo = LifepathCharacterProfile::new_with_stage2_baseline(
        "Mateo Vance",
        8,
        "alertness_situational_awareness",
        chars,
        aptitudes,
    );

    // Stage 2 (Ages 6–11): Literacy 1 success -> Reading N=2; Math 2 successes -> Arithmetic N=2 & early unlock Elementary Algebra N=1; Physical -> Brawling N=2
    mateo.skills.insert("reading_comprehension".to_string(), 2);
    mateo.skills.insert("arithmetic".to_string(), 2);
    mateo
        .skills
        .insert("elementary_algebra_geometry".to_string(), 1);
    mateo
        .skills
        .insert("brawling_unarmed_striking".to_string(), 2);

    // Stage 3 (Ages 12–15): Math 2 successes -> Elementary Algebra N=2 & Trigonometry N=2; Science -> General Biology N=1;
    // Age 15 Adolescent Crisis: rolls 9 (<= 12) and fails DIS check -> forced into Stage 4B Dropout!
    mateo
        .skills
        .insert("elementary_algebra_geometry".to_string(), 2);
    mateo.skills.insert("trigonometry".to_string(), 2);
    mateo
        .skills
        .insert("general_biology_natural_history".to_string(), 1);
    mateo.stage_history.push(
        "Stage 3 (Ages 12–15): Reached Elementary Algebra N=2 & Trigonometry N=2 via APT-QNT +3 (C_study=15, TN_edu=18); forced out of school at Age 15.".to_string(),
    );

    // Stage 4B (Ages 16–18): Shooter / Enforcer Street Gang package + Driving N=1
    mateo.apply_stage4b_dropout_gang_package(StreetGangArchetype::ShooterEnforcer);
    mateo
        .skills
        .insert("driving_wheeled_tracked".to_string(), 1);

    // Stage 5 (Ages 18–22): 1 Term in Organized Crime / Syndicate
    mateo.age_years = 22;
    mateo.skills.insert("pistols_handguns".to_string(), 4);
    mateo
        .skills
        .insert("fully_automatic_burst_weapons".to_string(), 2);
    mateo.skills.insert("intimidation_coercion".to_string(), 3);
    mateo
        .skills
        .insert("streetwise_underworld_etiquette".to_string(), 3);
    mateo.campaign_sqm = 4;
    mateo.operational_heat = 2;
    mateo.stage_history.push(
        "Stage 5 (Ages 18–22): Organized Crime / Syndicate Enforcer (Pistols N=4, Fully Automatic Weapons N=2, Intimidation N=3, Streetwise N=3).".to_string(),
    );

    mateo
}

/// Build the canonical walkthrough profile for **Dr. Priya Nair — Corporate AI & Robotics Scientist** (§2.2 of `lifepaths_and_economy.md`).
#[must_use]
pub fn build_priya_nair_lifepath() -> LifepathCharacterProfile {
    let mut chars = CharacterCharacteristics::default();
    chars.ana = 16;
    chars.mem = 15;
    chars.dis = 14;
    chars.foc = 14;
    chars.dex = 11;
    chars.for_stat = 8;
    chars.spd = 9;

    let mut aptitudes = CharacterAptitudes::default();
    aptitudes.set(AptitudeCode::AptQnt, 2);
    aptitudes.set(AptitudeCode::AptCmp, 3);
    aptitudes.set(AptitudeCode::AptMec, 1);

    // Stage 1: Affluent / Elite Preparatory (roll 36 -> SQM -4)
    let mut priya = LifepathCharacterProfile::new_with_stage2_baseline(
        "Dr. Priya Nair",
        36,
        "counting_basic_numeracy",
        chars,
        aptitudes,
    );
    priya
        .granted_access_tags
        .push(SkillAccessTier::VocationalLicensed);

    // Stages 2–4A (Ages 6–18): AP / Honors STEM Magnet Track
    priya.skills.insert("reading_comprehension".to_string(), 3);
    priya.skills.insert("writing_composition".to_string(), 3);
    priya.skills.insert("arithmetic".to_string(), 3);
    priya.skills.insert("digital_ai_literacy".to_string(), 3);
    priya
        .skills
        .insert("elementary_algebra_geometry".to_string(), 3);
    priya.skills.insert("trigonometry".to_string(), 3);
    priya
        .skills
        .insert("calculus_linear_algebra".to_string(), 2);
    priya.skills.insert("programming_scripting".to_string(), 2);
    priya
        .skills
        .insert("classical_mechanics_physics".to_string(), 2);

    // Stage 5 (Ages 18–26): 2 Terms (B.S. Computer Science & Engineering + Ph.D. Robotics & AI)
    priya.age_years = 26;
    priya
        .skills
        .insert("calculus_linear_algebra".to_string(), 4);
    priya
        .skills
        .insert("differential_equations_vector_calculus".to_string(), 3);
    priya.skills.insert("statistics_probability".to_string(), 3);
    priya.skills.insert("programming_scripting".to_string(), 4);
    priya
        .skills
        .insert("algorithms_systems_architecture".to_string(), 4);
    priya
        .skills
        .insert("machine_learning_ai_engineering".to_string(), 5);
    priya
        .skills
        .insert("thermodynamics_fluid_dynamics".to_string(), 2);
    priya
        .skills
        .insert("mechanical_structural_engineering".to_string(), 3);
    priya
        .skills
        .insert("aerospace_naval_robotics_engineering".to_string(), 4);
    priya
        .skills
        .insert("electrical_power_systems".to_string(), 2);
    priya
        .skills
        .insert("micro_electronics_avionics_cyberware_tech".to_string(), 3);
    priya.campaign_sqm = 8;
    priya.operational_heat = 0;

    priya
}

/// Campaign Socioeconomic Status & Upkeep Tier (`SQM 1–10`) (§3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CampaignSqmSpec {
    pub sqm_tier: u8,
    pub tier_name: &'static str,
    pub monthly_liquid_baseline_usd: u32,
    pub unhoused_fatigue_delta_int: i32,
    pub school_quality_modifier: i32,
}

/// Look up the canonical Campaign `SQM 1–10` Upkeep & Lifestyle specification (§3.1).
#[must_use]
pub const fn lookup_campaign_sqm(sqm_tier: u8) -> CampaignSqmSpec {
    match sqm_tier {
        ..=2 => CampaignSqmSpec {
            sqm_tier: if sqm_tier == 0 { 1 } else { sqm_tier },
            tier_name: "Destitute / Unhoused / Squatting",
            monthly_liquid_baseline_usd: 200,
            unhoused_fatigue_delta_int: 1,
            school_quality_modifier: 4,
        },
        3..=4 => CampaignSqmSpec {
            sqm_tier,
            tier_name: "Working Poor / Tenement / Slum",
            monthly_liquid_baseline_usd: 1_500,
            unhoused_fatigue_delta_int: 0,
            school_quality_modifier: 2,
        },
        5..=6 => CampaignSqmSpec {
            sqm_tier,
            tier_name: "Middle Class / Standard Apartment",
            monthly_liquid_baseline_usd: 4_500,
            unhoused_fatigue_delta_int: 0,
            school_quality_modifier: 0,
        },
        7..=8 => CampaignSqmSpec {
            sqm_tier,
            tier_name: "Affluent Professional / Gated Condo",
            monthly_liquid_baseline_usd: 15_000,
            unhoused_fatigue_delta_int: 0,
            school_quality_modifier: -2,
        },
        9.. => CampaignSqmSpec {
            sqm_tier: if sqm_tier > 10 { 10 } else { sqm_tier },
            tier_name: "Corporate Executive / Penthouse / Compound",
            monthly_liquid_baseline_usd: 100_000,
            unhoused_fatigue_delta_int: 0,
            school_quality_modifier: -4,
        },
    }
}

/// Equipment restriction class for Legal vs. Black-Market Procurement (§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlackMarketRestrictionClass {
    /// Class I: Licensed Civilian (`+0` $\Delta_{\text{ext}}$, `1.2x–1.5x` cost)
    Class1LicensedCivilian,
    /// Class II: Law Enforcement / Paramilitary (`+2` $\Delta_{\text{ext}}$, `2x–3x` cost)
    Class2LawEnforcementParamilitary,
    /// Class III: Military Ordnance / Full-Auto / Explosives (`+4` $\Delta_{\text{ext}}$, `3x–5x` cost)
    Class3MilitaryOrdnanceFullAuto,
    /// Class IV: Strategic / State Secret / Bioweapons (`+8` $\Delta_{\text{ext}}$, `10x+` cost)
    Class4StrategicStateSecret,
}

impl BlackMarketRestrictionClass {
    /// External difficulty modifier ($\Delta_{\text{ext}}$) on the Black-Market procurement roll (§3.2).
    #[must_use]
    pub const fn procurement_delta_ext(self) -> i32 {
        match self {
            Self::Class1LicensedCivilian => 0,
            Self::Class2LawEnforcementParamilitary => 2,
            Self::Class3MilitaryOrdnanceFullAuto => 4,
            Self::Class4StrategicStateSecret => 8,
        }
    }

    /// Typical black-market price multiplier midpoint (`1.35x`, `2.5x`, `4.0x`, `10.0x`) (§3.2).
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

/// Operational `Heat` & Law Enforcement Response Tier (`0–10`) (§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationalHeatTier {
    /// `Heat 0`: Clean — no active warrants or heightened scrutiny
    Heat0Clean,
    /// `Heat 1–2`: Local Beat Inquiry — routine patrol questions, minor surveillance
    Heat1To2LocalBeatInquiry,
    /// `Heat 3–5`: Active Warrant / APB — `+4` $\Delta_{\text{ext}}$ to all legal/public social & travel checks
    Heat3To5ActiveWarrantApb,
    /// `Heat 6–8`: Tactical Task Force / Strike Team — SWAT/military raids; legal `SQM` bank accounts frozen!
    Heat6To8TacticalTaskForce,
    /// `Heat 9–10`: Burned / National Manhunt — shoot-on-sight directives; total underground existence required
    Heat9To10BurnedNationalManhunt,
}

/// Mechanical effects of a character's current Operational `Heat` (`0–10`) (§3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationalHeatStatus {
    pub heat_score: u8,
    pub tier: OperationalHeatTier,
    pub public_check_delta_ext: i32,
    pub legal_bank_accounts_frozen: bool,
    pub description: &'static str,
}

/// Evaluate the mechanical effects of Operational `Heat` (`0..=10`) (§3.2).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mateo_vance_walkthrough_and_academic_freeze() {
        let mateo = build_mateo_vance_lifepath();
        assert_eq!(mateo.origin.school_quality_modifier_sqm, 2);

        // Verify Mateo's Educational Advancement parameters from §2.1:
        // DIS 11, ANA 13, APT-QNT +3 -> C_study = 15, TN_edu = 31 - 15 + 2 = 18
        let edu = evaluate_educational_advancement_roll(11, 13, 3, 2, [(10, 9), (11, 8)]);
        assert_eq!(edu.c_study, 15);
        assert_eq!(edu.tn_edu, 18);
        assert_eq!(edu.successes, 2);
        assert_eq!(
            edu.outcome_tier,
            EducationalAdvancementTier::HonorsAcceleratedBreakthrough
        );

        // Mateo has Trigonometry N=2 and Fully Automatic Weapons N=2
        assert_eq!(mateo.skills.get("trigonometry"), Some(&2));
        assert_eq!(mateo.skills.get("fully_automatic_burst_weapons"), Some(&2));
        assert!(mateo.academic_tree_frozen);

        // Despite ANA 13 and APT-QNT +3, Mateo is LOCKED out of Calculus & Linear Algebra because of Academic Tree Freeze
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

        // Audit Priya attempting to unlock Fully Automatic & Burst Weapons at N=1 (§2.2):
        // Must fail ALL THREE canonical gates:
        // 1. MISSING_ACCESS_GATE (requires Restricted or Illicit)
        // 2. MIN_CHARACTERISTIC_SHORTFALL (requires FOR >= 10, Priya has FOR = 8)
        // 3. PARENT_PREREQ_SHORTFALL (requires Pistols >= 2 OR Long Guns >= 2, Priya has N = 0 in both)
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
        let sqm1 = lookup_campaign_sqm(1);
        assert_eq!(sqm1.unhoused_fatigue_delta_int, 1);
        let heat4 = evaluate_operational_heat(4);
        assert_eq!(heat4.tier, OperationalHeatTier::Heat3To5ActiveWarrantApb);
        assert_eq!(heat4.public_check_delta_ext, 4);
        assert!(!heat4.legal_bank_accounts_frozen);

        let heat7 = evaluate_operational_heat(7);
        assert!(heat7.legal_bank_accounts_frozen);
    }
}
