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
    CharacterAptitudes, CharacterCharacteristics, compute_academic_study_controller,
};
use crate::define_coded_enum;
use crate::skills_trees::{
    SkillAccessTier, SkillAdvancementValidation, SkillStepCostBreakdown, SkillTrainingMode,
    compute_skill_step_sp_cost, initialize_universal_talents, validate_skill_advancement,
};

define_coded_enum! {
    /// Socioeconomic Class from Stage 1 (`Ages 0–5`) (§1.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SocioeconomicClass [count = 6] {
        /// `2–5`: Destitute / Underclass / War Zone (`SQM +4`, Dropout Risk `2–16`)
        DestituteUnderclassWarZone => {
            code: "DESTITUTE",
            label: "Destitute / Underclass / War Zone",
            aliases: ["DESTITUTE_UNDERCLASS_WAR_ZONE"],
        },
        /// `6–11`: Working Poor / Inner-City Lower Class (`SQM +2`, Dropout Risk `2–12`)
        WorkingPoorLowerClass => {
            code: "WORKING_POOR",
            label: "Working Poor / Inner-City Lower Class",
            aliases: ["WORKING_POOR_LOWER_CLASS"],
        },
        /// `12–25`: Working / Middle Class (`SQM 0`, Dropout Risk `2–8`)
        WorkingMiddleClass => {
            code: "MIDDLE_CLASS",
            label: "Working / Middle Class",
            aliases: ["WORKING_MIDDLE_CLASS"],
        },
        /// `26–33`: Professional / Upper-Middle Class (`SQM -2`, Dropout Risk `2–5`)
        ProfessionalUpperMiddleClass => {
            code: "UPPER_MIDDLE",
            label: "Professional / Upper-Middle Class",
            aliases: ["PROFESSIONAL_UPPER_MIDDLE_CLASS"],
        },
        /// `34–38`: Affluent / Elite Preparatory (`SQM -4`, Dropout Risk `2–3`)
        AffluentElitePreparatory => {
            code: "AFFLUENT_PREP",
            label: "Affluent / Elite Preparatory",
            aliases: ["AFFLUENT_ELITE_PREPARATORY"],
        },
        /// `39–40`: Wealthy / Corporate or Aristocratic Elite (`SQM -6`, Dropout Risk `2`)
        WealthyCorporateAristocraticElite => {
            code: "WEALTHY_ELITE",
            label: "Wealthy / Corporate or Aristocratic Elite",
            aliases: ["WEALTHY_CORPORATE_ARISTOCRATIC_ELITE"],
        },
    }
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

define_coded_enum! {
    /// Stage 4 Adolescent Path (`Ages 15–18`) (§1.5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum Stage4Path [count = 3] {
        /// Path 4A: Formal High School Track (`[Access: Academic]`, or `+ [Access: Vocational/Licensed]` on Vocational Track)
        Path4AHighSchool => {
            code: "STAY_4A",
            label: "Path 4A: Formal High School (Ages 15–18)",
            aliases: ["PATH_4A", "HIGH_SCHOOL"],
        },
        /// Path 4B: Urban Dropout / Street Gang / Juvenile Underworld (`Academic Tree Freeze`, unlocks `[Access: Illicit]`)
        Path4BStreetGang => {
            code: "DROPOUT_4B",
            label: "Path 4B: Urban Dropout / Street Gang (Ages 15–18)",
            aliases: ["PATH_4B", "STREET_GANG"],
        },
        /// Path 4C: Rural, Agrarian & Developing-Region Youth (`Academic Tree Freeze` if leaving school at 15, unlocks `[Access: Vocational/Licensed]` & rural/fieldcraft skills)
        Path4CRuralAgrarian => {
            code: "RURAL_4C",
            label: "Path 4C: Rural, Agrarian & Developing-Region Youth (Ages 15–18)",
            aliases: ["PATH_4C", "RURAL_AGRARIAN"],
        },
    }
}

define_coded_enum! {
    /// Stage 4A High School Track (`Ages 16–18`) (§1.5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum HighSchoolTrack [count = 3] {
        /// Track 1: Standard / College-Prep High School (`[Access: Academic]`)
        StandardCollegePrep => {
            code: "STANDARD_COLLEGE_PREP",
            label: "Standard / College-Prep High School",
            subtitle: "[Access: Academic] Advances Algebra & Geometry, Trigonometry, General Chemistry, Classical Physics, Foreign Language & Driving.",
            aliases: ["STANDARD"],
        },
        /// Track 2: AP / Honors / STEM Magnet Track (requires `Elementary Algebra >= 2` and `C_study >= 13`, `SQM -1` bonus)
        ApHonorsStemMagnet => {
            code: "AP_HONORS_STEM",
            label: "AP / Honors / STEM Magnet Academy",
            subtitle: "[Access: Academic] (Req: Algebra >= 2, ANA >= 11, DIS >= 11; SQM -1 bonus) Unlocks Calculus, Statistics, Programming, Classical Physics & Chemistry at N=2.",
            aliases: ["AP_HONORS_STEM_MAGNET", "STEM"],
        },
        /// Track 3: Vocational / Technical High School (unlocks `[Access: Vocational/Licensed]`)
        VocationalTechnical => {
            code: "VOCATIONAL_TECHNICAL",
            label: "Vocational / Technical High School",
            subtitle: "[Access: Academic + Vocational/Licensed] Unlocks Automotive Mechanics, Metalworking, Electrical Systems, Tinkering N=2 & Driving.",
            aliases: ["VOCATIONAL"],
        },
    }
}

define_coded_enum! {
    /// Stage 4B Dropout / Street Gang / Juvenile Underworld Archetype (`Ages 16–18`) (§1.5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum StreetGangArchetype [count = 4] {
        /// *Shooter / Enforcer*: `Short Blades` or `Pistols` to $N=2$ (and `Fully Automatic Weapons` to $N=1$ if `FOR >= 10`), `Intimidation` to $N=2$
        ShooterEnforcer => {
            code: "SHOOTER_ENFORCER",
            label: "Shooter / Street Enforcer",
            subtitle: "Grants Streetwise N=2, Pistols N=2, Short Blades N=2, Intimidation N=2, plus Fully Automatic Weapons N=1 (if FOR >= 10).",
            aliases: ["SHOOTER"],
        },
        /// *Boost / Wheelman*: `Driving` to $N=2$, `Lockpicking & Physical Intrusion` to $N=2$, `Automotive Mechanics` to $N=1$
        BoostWheelman => {
            code: "BOOST_WHEELMAN",
            label: "Boost / Wheelman & Tech Thief",
            subtitle: "Grants Streetwise N=2, Driving N=2, Lockpicking & Physical Intrusion N=2, Automotive Mechanics N=1.",
            aliases: ["WHEELMAN"],
        },
        /// *Runner / Lookout*: `Stealth` to $N=2$, `Alertness` to $N=2$, `Running` or `Acrobatics/Parkour` to $N=2$, `Sleight of Hand` to $N=1$
        RunnerLookout => {
            code: "RUNNER_LOOKOUT",
            label: "Runner / Lookout & Urban Burglar",
            subtitle: "Grants Streetwise N=2, Stealth N=2, Alertness N=2, Acrobatics/Parkour N=2, Sleight of Hand N=1.",
            aliases: ["RUNNER"],
        },
        /// *Back-Alley Cook / Modder*: `Pharmacology & Toxicology` (illicit synthesis) or `Armourer & Gunsmithing` to $N=2$
        BackAlleyCookModder => {
            code: "COOK_MODDER",
            label: "Back-Alley Cook / Street Gunsmith Modder",
            subtitle: "Grants Streetwise N=2, Armourer & Gunsmithing N=2, Pharmacology & Toxicology N=2.",
            aliases: ["BACK_ALLEY_COOK_MODDER"],
        },
    }
}

define_coded_enum! {
    /// Stage 4C Rural, Agrarian & Developing-Region Youth Archetype (`Ages 15–18`) (§1.5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum RuralYouthArchetype [count = 4] {
        /// *Subsistence Hunter, Trapper & Bush Guide*: `Hunting, Trapping & Field Butchery` $N=2$, `Wilderness Survival` $N=2$, `Long Guns` or `Archery & Bows` $N=2$, `Stealth` $N=2$
        SubsistenceHunterTrapper => {
            code: "SUBSISTENCE_HUNTER_TRAPPER",
            label: "Subsistence Hunter, Trapper & Bush Guide",
            subtitle: "[Access: Vocational/Licensed] Grants Foraging N=2, Hunting & Trapping N=2, Wilderness Survival N=2, Long Guns N=2, Archery N=1, Stealth N=2.",
            aliases: ["HUNTER_TRAPPER", "HUNTER"],
        },
        /// *Agrarian Homesteader & Rural Mechanist*: `Agriculture, Agronomy & Crop Husbandry` $N=2$, `Animal Handling` $N=1$, `Automotive & Small-Engine Mechanics` $N=2$, `Carpentry & Woodworking` $N=1$, `Driving` $N=1$
        AgrarianHomesteaderMechanist => {
            code: "AGRARIAN_HOMESTEADER_MECHANIST",
            label: "Agrarian Homesteader & Rural Mechanist",
            subtitle: "[Access: Vocational/Licensed] Grants Agriculture & Agronomy N=2, Animal Handling N=1, Automotive & Small-Engine Mechanics N=2, Carpentry N=1, Driving N=1.",
            aliases: ["AGRARIAN_HOMESTEADER", "AGRARIAN"],
        },
        /// *Pastoral Herder, Stockman & Outrider*: `Animal Handling, Husbandry & Riding` $N=2$, `Veterinary Medicine & Livestock Care` $N=1$, `Herbalism, Ethnobotany & Field Remedies` $N=1$, `Land Navigation` $N=1$
        PastoralHerderOutrider => {
            code: "PASTORAL_HERDER_OUTRIDER",
            label: "Pastoral Herder, Stockman & Outrider",
            subtitle: "[Access: Vocational/Licensed] Grants Animal Handling & Riding N=2, Veterinary Care N=1, Herbalism & Ethnobotany N=1, First Aid N=1, Land Navigation N=1.",
            aliases: ["PASTORAL_HERDER", "PASTORAL"],
        },
        /// *Frontier Scout, Border Smuggler & Village Militia*: `Long Guns` $N=2$, `Camouflage & Concealment` $N=2$, `Land Navigation` $N=1$, `Alertness` $N=2$ (also unlocks `[Access: Illicit]`)
        FrontierScoutMilitiaSmuggler => {
            code: "FRONTIER_SCOUT_MILITIA",
            label: "Frontier Scout, Border Smuggler & Village Militia",
            subtitle: "[Access: Vocational/Licensed + Illicit] Grants Long Guns N=2, Camouflage N=2, Stealth N=2, Alertness N=2, Land Navigation N=1.",
            aliases: ["FRONTIER_SCOUT_MILITIA_SMUGGLER", "FRONTIER_SCOUT"],
        },
    }
}

define_coded_enum! {
    /// Stage 5 Adult 4-Year Career Term Track (`Ages 18+`) (§1.6).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum CareerTermTrack [count = 8] {
        UniversityUndergraduateStem => {
            code: "UNIVERSITY_STEM",
            label: "University — STEM / Engineering (B.S. / M.S. / Ph.D.)",
            aliases: ["UNIVERSITY_UNDERGRADUATE_STEM"],
        },
        MedicalSchoolAndResidency => {
            code: "MEDICAL_SCHOOL",
            label: "Medical School & Surgical Residency",
            aliases: ["MEDICAL_SCHOOL_AND_RESIDENCY"],
        },
        MilitaryCombatArmsSpecOps => {
            code: "MILITARY_COMBAT_ARMS",
            label: "Military Service — Combat Arms / Special Operations",
            aliases: ["MILITARY_COMBAT_ARMS_SPEC_OPS"],
        },
        LawEnforcementSwatDetective => {
            code: "LAW_ENFORCEMENT_SWAT",
            label: "Law Enforcement / SWAT / Detective Bureau",
            aliases: ["LAW_ENFORCEMENT_SWAT_DETECTIVE"],
        },
        IntelligenceServiceClandestine => {
            code: "INTELLIGENCE_CLANDESTINE",
            label: "Intelligence Service / Clandestine Operations",
            aliases: ["INTELLIGENCE_SERVICE_CLANDESTINE"],
        },
        OrganizedCrimeCartelSyndicate => {
            code: "ORGANIZED_CRIME_SYNDICATE",
            label: "Organized Crime / Cartel / Street Syndicate",
            aliases: ["ORGANIZED_CRIME_CARTEL_SYNDICATE"],
        },
        BlueCollarTradesMerchantMarine => {
            code: "BLUE_COLLAR_TRADES",
            label: "Blue-Collar Trades / Industrial Tech / Merchant Marine",
            aliases: ["BLUE_COLLAR_TRADES_MERCHANT_MARINE"],
        },
        RuralAgrarianFrontierRanger => {
            code: "RURAL_AGRARIAN_RANGER",
            label: "Rural, Agrarian, Wilderness Guide & Frontier Ranger",
            aliases: ["RURAL_AGRARIAN_FRONTIER_RANGER", "RURAL_RANGER", "AGRARIAN_RANGER"],
        },
    }
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
            Self::BlueCollarTradesMerchantMarine | Self::RuralAgrarianFrontierRanger => {
                &[SkillAccessTier::VocationalLicensed]
            }
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
        insert_skill_ranks(
            &mut skills,
            &[
                ("reading_comprehension", 1),
                ("writing_composition", 1),
                ("arithmetic", 1),
                ("digital_ai_literacy", 1),
            ],
        );

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
        archetype.apply_grants(
            &mut self.skills,
            &mut self.granted_access_tags,
            self.characteristics.for_stat,
            false,
        );
        self.stage_history.push(format!(
            "Stage 4B (Ages 16–18): Dropped out into {:?} — Academic Tree Frozen; granted [Access: Illicit]",
            archetype
        ));
    }

    /// Apply Stage 4C Rural, Agrarian & Developing-Region Youth Transition (`Ages 15–18`) (§1.5).
    ///
    /// Leaving formal classroom schooling at age 15 in a rural/agrarian or underdeveloped region
    /// triggers the **Academic Tree Freeze** (unless later cleared via GED/Adult Education),
    /// unlocks `[Access: Vocational/Licensed]` (plus `[Access: Illicit]` for `FrontierScoutMilitiaSmuggler`),
    /// and grants authentic rural/fieldcraft/agrarian skill foundations.
    pub fn apply_stage4c_rural_agrarian_package(&mut self, archetype: RuralYouthArchetype) {
        self.age_years = 18;
        self.academic_tree_frozen = true;
        archetype.apply_grants(&mut self.skills, &mut self.granted_access_tags);
        self.stage_history.push(format!(
            "Stage 4C (Ages 15–18): Rural / Developing-Region Youth ({:?}) — Academic Tree Frozen; granted [Access: Vocational/Licensed]",
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

pub use crate::walkthroughs::{
    build_amara_okafor_lifepath, build_mateo_vance_lifepath, build_priya_nair_lifepath,
};
use crate::walkthroughs::insert_skill_ranks;

fn bump_skill_floors(skills: &mut BTreeMap<String, usize>, floors: &[(&str, usize)]) {
    for &(id, min_level) in floors {
        bump_skill_floor(skills, id, min_level);
    }
}

impl StreetGangArchetype {
    fn apply_grants(
        self,
        skills: &mut BTreeMap<String, usize>,
        granted_access_tags: &mut Vec<SkillAccessTier>,
        for_stat: i32,
        include_prereq_talents: bool,
    ) {
        granted_access_tags.retain(|t| *t != SkillAccessTier::Academic);
        if !granted_access_tags.contains(&SkillAccessTier::Illicit) {
            granted_access_tags.push(SkillAccessTier::Illicit);
        }
        bump_skill_floor(skills, "streetwise_underworld_etiquette", 2);
        if include_prereq_talents {
            bump_skill_floor(skills, "brawling_unarmed_striking", 2);
        }
        let grants: &[(&str, usize)] = match self {
            Self::ShooterEnforcer => &[
                ("pistols_handguns", 2),
                ("short_blades_knife_fighting", 2),
                ("intimidation_coercion", 2),
            ],
            Self::BoostWheelman => {
                if include_prereq_talents {
                    bump_skill_floor(skills, "manual_dexterity_tinkering", 2);
                }
                &[
                    ("driving_wheeled_tracked", 2),
                    ("lockpicking_physical_intrusion", 2),
                    ("automotive_small_engine_mechanics", 1),
                ]
            }
            Self::RunnerLookout => {
                if include_prereq_talents {
                    bump_skill_floors(
                        skills,
                        &[
                            ("balance_footwork", 2),
                            ("jumping_leaping", 2),
                            ("manual_dexterity_tinkering", 2),
                        ],
                    );
                }
                &[
                    ("stealth_sneaking_hiding", 2),
                    ("alertness_situational_awareness", 2),
                    ("acrobatics_tumbling_parkour", 2),
                    ("sleight_of_hand_pickpocketing_palming", 1),
                ]
            }
            Self::BackAlleyCookModder => {
                if include_prereq_talents {
                    bump_skill_floors(
                        skills,
                        &[
                            ("manual_dexterity_tinkering", 2),
                            ("metalworking_machining_blacksmithing", 2),
                        ],
                    );
                }
                &[("armourer_gunsmithing", 2), ("pharmacology_toxicology", 2)]
            }
        };
        bump_skill_floors(skills, grants);
        if self == Self::ShooterEnforcer && for_stat >= 10 {
            bump_skill_floor(skills, "fully_automatic_burst_weapons", 1);
        }
    }
}

impl RuralYouthArchetype {
    fn apply_grants(
        self,
        skills: &mut BTreeMap<String, usize>,
        granted_access_tags: &mut Vec<SkillAccessTier>,
    ) {
        granted_access_tags.retain(|t| *t != SkillAccessTier::Academic);
        if !granted_access_tags.contains(&SkillAccessTier::VocationalLicensed) {
            granted_access_tags.push(SkillAccessTier::VocationalLicensed);
        }
        bump_skill_floors(
            skills,
            &[
                ("foraging_campcraft", 2),
                ("wilderness_survival_biome_specific", 1),
                ("manual_dexterity_tinkering", 2),
            ],
        );
        let grants: &[(&str, usize)] = match self {
            Self::SubsistenceHunterTrapper => &[
                ("stealth_sneaking_hiding", 2),
                ("hunting_trapping_field_butchery", 2),
                ("wilderness_survival_biome_specific", 2),
                ("long_guns_rifles_carbines_shotguns", 2),
                ("archery_bows_crossbows", 1),
            ],
            Self::AgrarianHomesteaderMechanist => &[
                ("agriculture_agronomy_pastoralism", 2),
                ("animal_handling_husbandry_riding", 1),
                ("automotive_small_engine_mechanics", 2),
                ("carpentry_woodworking", 1),
                ("driving_wheeled_tracked", 1),
            ],
            Self::PastoralHerderOutrider => &[
                ("animal_handling_husbandry_riding", 2),
                ("first_aid_bls_cpr", 1),
                ("veterinary_medicine_livestock_care", 1),
                ("herbalism_ethnobotany_natural_remedies", 1),
                ("land_navigation_cartography", 1),
            ],
            Self::FrontierScoutMilitiaSmuggler => {
                if !granted_access_tags.contains(&SkillAccessTier::Illicit) {
                    granted_access_tags.push(SkillAccessTier::Illicit);
                }
                &[
                    ("alertness_situational_awareness", 2),
                    ("stealth_sneaking_hiding", 2),
                    ("camouflage_concealment", 2),
                    ("long_guns_rifles_carbines_shotguns", 2),
                    ("land_navigation_cartography", 1),
                ]
            }
        };
        bump_skill_floors(skills, grants);
    }
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

impl SocioeconomicClass {
    /// Representative `2d20` roll (`2..=40`) that maps to this Socioeconomic Class in [`resolve_socioeconomic_origin`].
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
}

impl CareerTermTrack {

    /// Summary of entry prerequisites (§1.6).
    #[must_use]
    pub const fn entry_prereq_summary(self) -> &'static str {
        match self {
            Self::UniversityUndergraduateStem => {
                "High School Diploma (or GED), Trigonometry >= 2, ANA >= 11, DIS >= 10"
            }
            Self::MedicalSchoolAndResidency => {
                "General Biology >= 2, General Chemistry >= 2, MEM >= 12, ANA >= 11, DIS >= 12"
            }
            Self::MilitaryCombatArmsSpecOps => "FOR >= 9, FIT >= 10, HLT >= 10",
            Self::LawEnforcementSwatDetective => "DIS >= 10, SIG >= 10, INS >= 10",
            Self::IntelligenceServiceClandestine => "INS >= 11, DIS >= 11, ELO >= 10",
            Self::OrganizedCrimeCartelSyndicate => {
                "Underworld recruitment or Juvenile Gang background (PRE >= 9 or INS >= 9)"
            }
            Self::BlueCollarTradesMerchantMarine => "Arithmetic >= 2, Manual Dexterity >= 1",
            Self::RuralAgrarianFrontierRanger => {
                "Foraging & Campcraft >= 2, Agriculture >= 1, or Animal Handling >= 1 (or FIT >= 9 & HLT >= 9)"
            }
        }
    }

    /// Check whether a character natively meets the entry prerequisites for this Career Track (§1.6).
    #[must_use]
    pub fn check_entry_qualification(
        self,
        chars: &CharacterCharacteristics,
        skills: &BTreeMap<String, usize>,
        academic_tree_frozen: bool,
    ) -> (bool, String) {
        let trig = skills.get("trigonometry").copied().unwrap_or(0);
        let bio = skills
            .get("general_biology_natural_history")
            .copied()
            .unwrap_or(0);
        let chem = skills.get("general_chemistry").copied().unwrap_or(0);
        let arith = skills.get("arithmetic").copied().unwrap_or(0);
        let foraging = skills.get("foraging_campcraft").copied().unwrap_or(0);
        let ag = skills
            .get("agriculture_agronomy_pastoralism")
            .copied()
            .unwrap_or(0);
        let animal = skills
            .get("animal_handling_husbandry_riding")
            .copied()
            .unwrap_or(0);

        match self {
            Self::UniversityUndergraduateStem => {
                let ok = !academic_tree_frozen && trig >= 2 && chars.ana >= 11 && chars.dis >= 10;
                let msg = format!(
                    "Academic Unfrozen: {}, Trig N={trig} (req >=2), ANA {} (req >=11), DIS {} (req >=10)",
                    !academic_tree_frozen, chars.ana, chars.dis
                );
                (ok, msg)
            }
            Self::MedicalSchoolAndResidency => {
                let ok = !academic_tree_frozen
                    && bio >= 1
                    && chem >= 1
                    && chars.mem >= 12
                    && chars.ana >= 11
                    && chars.dis >= 12;
                let msg = format!(
                    "Academic Unfrozen: {}, Bio N={bio}, Chem N={chem}, MEM {} (req >=12), ANA {} (req >=11), DIS {} (req >=12)",
                    !academic_tree_frozen, chars.mem, chars.ana, chars.dis
                );
                (ok, msg)
            }
            Self::MilitaryCombatArmsSpecOps => {
                let ok = chars.for_stat >= 9 && chars.fit >= 10 && chars.hlt >= 10;
                let msg = format!(
                    "FOR {} (req >=9), FIT {} (req >=10), HLT {} (req >=10)",
                    chars.for_stat, chars.fit, chars.hlt
                );
                (ok, msg)
            }
            Self::LawEnforcementSwatDetective => {
                let ok = chars.dis >= 10 && chars.sig >= 10 && chars.ins >= 10;
                let msg = format!(
                    "DIS {} (req >=10), SIG {} (req >=10), INS {} (req >=10)",
                    chars.dis, chars.sig, chars.ins
                );
                (ok, msg)
            }
            Self::IntelligenceServiceClandestine => {
                let ok = chars.ins >= 11 && chars.dis >= 11 && chars.elo >= 10;
                let msg = format!(
                    "INS {} (req >=11), DIS {} (req >=11), ELO {} (req >=10)",
                    chars.ins, chars.dis, chars.elo
                );
                (ok, msg)
            }
            Self::OrganizedCrimeCartelSyndicate => {
                let ok = chars.pre >= 9 || chars.ins >= 9 || academic_tree_frozen;
                let msg = format!(
                    "PRE {} / INS {} (or Dropout/Street background)",
                    chars.pre, chars.ins
                );
                (ok, msg)
            }
            Self::BlueCollarTradesMerchantMarine => {
                let ok = arith >= 1;
                let msg = format!("Arithmetic N={arith} (req >=1)");
                (ok, msg)
            }
            Self::RuralAgrarianFrontierRanger => {
                let ok = foraging >= 2
                    || ag >= 1
                    || animal >= 1
                    || (chars.fit >= 9 && chars.hlt >= 9);
                let msg = format!(
                    "Foraging N={foraging}, Agriculture N={ag}, Animal Handling N={animal}, FIT {}/HLT {}",
                    chars.fit, chars.hlt
                );
                (ok, msg)
            }
        }
    }

    /// Ordered list of canonical curriculum & career skills taught by this track (ordered from parent foundation to advanced specialization).
    #[must_use]
    pub const fn recommended_skills(self) -> &'static [&'static str] {
        match self {
            Self::UniversityUndergraduateStem => &[
                "elementary_algebra_geometry",
                "trigonometry",
                "digital_ai_literacy",
                "calculus_linear_algebra",
                "statistics_probability",
                "programming_scripting",
                "classical_mechanics_physics",
                "algorithms_systems_architecture",
                "differential_equations_vector_calculus",
                "mechanical_structural_engineering",
                "electrical_power_systems",
                "thermodynamics_fluid_dynamics",
                "machine_learning_ai_engineering",
                "aerospace_naval_robotics_engineering",
                "micro_electronics_avionics_cyberware_tech",
            ],
            Self::MedicalSchoolAndResidency => &[
                "hygiene_basic_self_care",
                "first_aid_bls_cpr",
                "general_biology_natural_history",
                "elementary_algebra_geometry",
                "general_chemistry",
                "anatomy_physiology",
                "organic_analytical_chemistry",
                "paramedic_combat_trauma_care",
                "nursing_patient_care",
                "pharmacology_toxicology",
                "clinical_diagnosis_internal_medicine",
                "trauma_orthopedic_general_surgery",
                "psychology_behavioral_analysis_counseling",
                "genetics_biotech_cyber_implantation",
            ],
            Self::MilitaryCombatArmsSpecOps => &[
                "manual_dexterity_tinkering",
                "brawling_unarmed_striking",
                "balance_footwork",
                "alertness_situational_awareness",
                "stealth_sneaking_hiding",
                "persuasion_fast_talk",
                "first_aid_bls_cpr",
                "long_guns_rifles_carbines_shotguns",
                "pistols_handguns",
                "fully_automatic_burst_weapons",
                "leadership_command",
                "small_unit_tactics_cqb_doctrine",
                "heavy_weapons_mounted_gunnery_atgms",
                "martial_arts_striking",
                "grappling_wrestling_submissions",
                "short_blades_knife_fighting",
                "camouflage_concealment",
                "land_navigation_cartography",
                "paramedic_combat_trauma_care",
                "precision_marksmanship_sniping",
                "demolitions_explosive_ordnance_eod",
                "piloting_fixed_wing_rotary",
                "piloting_rotary_wing_helicopter",
            ],
            Self::LawEnforcementSwatDetective => &[
                "manual_dexterity_tinkering",
                "alertness_situational_awareness",
                "empathy_social_perception",
                "intimidation_coercion",
                "deception_bluff",
                "persuasion_fast_talk",
                "brawling_unarmed_striking",
                "balance_footwork",
                "pistols_handguns",
                "long_guns_rifles_carbines_shotguns",
                "driving_wheeled_tracked",
                "grappling_wrestling_submissions",
                "interrogation_counter_interrogation",
                "surveillance_reconnaissance_countersurveillance",
                "leadership_command",
                "small_unit_tactics_cqb_doctrine",
                "fully_automatic_burst_weapons",
                "forensics_crime_scene_investigation",
                "law_jurisprudence",
            ],
            Self::IntelligenceServiceClandestine => &[
                "alertness_situational_awareness",
                "stealth_sneaking_hiding",
                "deception_bluff",
                "empathy_social_perception",
                "persuasion_fast_talk",
                "intimidation_coercion",
                "manual_dexterity_tinkering",
                "pistols_handguns",
                "foreign_language_specific",
                "surveillance_reconnaissance_countersurveillance",
                "interrogation_counter_interrogation",
                "camouflage_concealment",
                "disguise_impersonation",
                "lockpicking_physical_intrusion",
                "streetwise_underworld_etiquette",
                "programming_scripting",
                "cybersecurity_network_intrusion",
                "forgery_counterfeiting_document_alteration",
            ],
            Self::OrganizedCrimeCartelSyndicate => &[
                "manual_dexterity_tinkering",
                "brawling_unarmed_striking",
                "balance_footwork",
                "intimidation_coercion",
                "deception_bluff",
                "empathy_social_perception",
                "stealth_sneaking_hiding",
                "alertness_situational_awareness",
                "streetwise_underworld_etiquette",
                "pistols_handguns",
                "short_blades_knife_fighting",
                "fully_automatic_burst_weapons",
                "lockpicking_physical_intrusion",
                "driving_wheeled_tracked",
                "sleight_of_hand_pickpocketing_palming",
                "interrogation_counter_interrogation",
                "armourer_gunsmithing",
            ],
            Self::BlueCollarTradesMerchantMarine => &[
                "manual_dexterity_tinkering",
                "arithmetic",
                "elementary_algebra_geometry",
                "driving_wheeled_tracked",
                "automotive_small_engine_mechanics",
                "metalworking_machining_blacksmithing",
                "electrical_power_systems",
                "carpentry_woodworking",
                "masonry_construction",
                "small_boat_seamanship",
                "armourer_gunsmithing",
                "micro_electronics_avionics_cyberware_tech",
                "piloting_fixed_wing_rotary",
            ],
            Self::RuralAgrarianFrontierRanger => &[
                "foraging_campcraft",
                "stealth_sneaking_hiding",
                "alertness_situational_awareness",
                "manual_dexterity_tinkering",
                "first_aid_bls_cpr",
                "general_biology_natural_history",
                "agriculture_agronomy_pastoralism",
                "animal_handling_husbandry_riding",
                "hunting_trapping_field_butchery",
                "wilderness_survival_biome_specific",
                "herbalism_ethnobotany_natural_remedies",
                "veterinary_medicine_livestock_care",
                "long_guns_rifles_carbines_shotguns",
                "archery_bows_crossbows",
                "land_navigation_cartography",
                "camouflage_concealment",
                "automotive_small_engine_mechanics",
                "carpentry_woodworking",
                "driving_wheeled_tracked",
                "small_boat_seamanship",
            ],
        }
    }
}

/// Configuration for a single 4-Year Adult Career Term in Stage 5 (`Ages 18+`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareerTermConfig {
    pub track: CareerTermTrack,
    /// Two `2d20` pairs rolled for career term performance (`0`, `1`, or `2` successes).
    pub term_roll_pairs: [(i32, i32); 2],
    /// Optional manual override for term advancement successes (`0..=2`).
    pub override_successes: Option<usize>,
    /// True if the player granted a moral/academic entry waiver for this term.
    pub waiver_override: bool,
}

impl Default for CareerTermConfig {
    fn default() -> Self {
        Self {
            track: CareerTermTrack::MilitaryCombatArmsSpecOps,
            term_roll_pairs: [(12, 11), (13, 10)],
            override_successes: None,
            waiver_override: true,
        }
    }
}

/// Complete player-configurable 5-Stage Lifepath state for the Character Creation Wizard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifepathBuildConfig {
    /// Stage 1: `2d20` roll (`2..=40`) for Socioeconomic Origin (or set via class override).
    pub origin_roll_2d20: i32,
    /// Stage 1: Chosen Early Environment Bonus Talent (`TAL` boosted by `+1` to `N = 2`).
    pub stage1_early_talent: String,
    /// Stage 2: Elementary Literacy `2d20` pairs (`DIS/MEM + APT-LNG`).
    pub stage2_lit_pairs: [(i32, i32); 2],
    pub stage2_lit_override: Option<usize>,
    /// Stage 2: Elementary Mathematics `2d20` pairs (`DIS/ANA + APT-QNT`).
    pub stage2_math_pairs: [(i32, i32); 2],
    pub stage2_math_override: Option<usize>,
    /// Stage 2: Playground / Recess Talent (`TAL` boosted by `+1`).
    pub stage2_playground_talent: String,
    /// Stage 3: Junior High Mathematics `2d20` pairs (`DIS/ANA + APT-QNT`).
    pub stage3_math_pairs: [(i32, i32); 2],
    pub stage3_math_override: Option<usize>,
    /// Stage 3: Junior High General Science `2d20` pairs (`DIS/ANA + APT-QNT`).
    pub stage3_sci_pairs: [(i32, i32); 2],
    pub stage3_sci_override: Option<usize>,
    /// Stage 3: Junior High Literacy & Humanities `2d20` pairs (`DIS/MEM + APT-LNG`).
    pub stage3_hum_pairs: [(i32, i32); 2],
    pub stage3_hum_override: Option<usize>,
    /// Stage 3 -> 4 Transition: `2d20` roll for Adolescent Crisis / Dropout Check at Age 15.
    pub age15_crisis_roll_2d20: i32,
    /// Optional player override for Age 15 path (`Some(true)` = Force Dropout Path 4B, `Some(false)` = Stay in High School Path 4A).
    pub age15_dropout_override: Option<bool>,
    /// Optional explicit Stage 4 path override (`Path4AHighSchool`, `Path4BStreetGang`, or `Path4CRuralAgrarian`).
    /// Takes precedence over `age15_dropout_override` when set.
    pub stage4_path_override: Option<Stage4Path>,
    /// Stage 4A: Selected High School Track (when staying in school).
    pub stage4a_hs_track: HighSchoolTrack,
    /// Stage 4A: High School Advancement `2d20` pairs.
    pub stage4a_hs_pairs: [(i32, i32); 2],
    pub stage4a_hs_override: Option<usize>,
    /// Stage 4B: Selected Street Gang / Underworld Archetype (when dropping out into urban underworld at Age 15).
    pub stage4b_gang_archetype: StreetGangArchetype,
    /// Stage 4C: Selected Rural, Agrarian & Developing-Region Youth Archetype (when taking Path 4C at Age 15).
    pub stage4c_rural_archetype: RuralYouthArchetype,
    /// Stage 5: Whether a dropout later obtained a GED / Adult Academic Waiver to unfreeze `[Access: Academic]`.
    pub adult_ged_waiver: bool,
    /// Stage 5: `0` to `4` Adult 4-Year Career Terms (`Ages 18–34`).
    pub career_terms: Vec<CareerTermConfig>,
    /// Campaign `SQM` Tier (`1–10`) override (`None` = derived from Origin & Career Terms).
    pub campaign_sqm_override: Option<u8>,
    /// Campaign Operational `Heat` (`0–10`) override (`None` = derived from Dropout/Syndicate history).
    pub operational_heat_override: Option<u8>,
}

impl Default for LifepathBuildConfig {
    fn default() -> Self {
        Self {
            origin_roll_2d20: 18,
            stage1_early_talent: "alertness_situational_awareness".to_string(),
            stage2_lit_pairs: [(12, 11), (10, 11)],
            stage2_lit_override: None,
            stage2_math_pairs: [(13, 11), (11, 10)],
            stage2_math_override: None,
            stage2_playground_talent: "running_sprinting".to_string(),
            stage3_math_pairs: [(12, 12), (11, 10)],
            stage3_math_override: None,
            stage3_sci_pairs: [(12, 10), (11, 9)],
            stage3_sci_override: None,
            stage3_hum_pairs: [(11, 11), (10, 9)],
            stage3_hum_override: None,
            age15_crisis_roll_2d20: 20,
            age15_dropout_override: None,
            stage4_path_override: None,
            stage4a_hs_track: HighSchoolTrack::StandardCollegePrep,
            stage4a_hs_pairs: [(13, 11), (12, 10)],
            stage4a_hs_override: None,
            stage4b_gang_archetype: StreetGangArchetype::ShooterEnforcer,
            stage4c_rural_archetype: RuralYouthArchetype::SubsistenceHunterTrapper,
            adult_ged_waiver: false,
            career_terms: vec![CareerTermConfig::default()],
            campaign_sqm_override: None,
            operational_heat_override: None,
        }
    }
}

/// Detailed result of evaluating Stages 1–5 of a [`LifepathBuildConfig`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LifepathBaselineResult {
    pub age_years: u32,
    pub origin: SocioeconomicOriginSpec,
    pub stage2_lit_eval: EducationalAdvancementResult,
    pub stage2_math_eval: EducationalAdvancementResult,
    pub stage3_math_eval: EducationalAdvancementResult,
    pub stage3_sci_eval: EducationalAdvancementResult,
    pub stage3_hum_eval: EducationalAdvancementResult,
    pub dropped_out_at_15: bool,
    pub stage4_path: Stage4Path,
    pub stage4a_hs_eval: Option<EducationalAdvancementResult>,
    pub term_evals: Vec<EducationalAdvancementResult>,
    /// Skills granted automatically by Stages 1–4 (`Universal Talents` + `Origin` + `Elementary` + `Junior High` + `High School / Gang / Rural`).
    /// Serves as the minimum floor in the Step 5 Skill Tree Builder.
    pub baseline_skills: BTreeMap<String, usize>,
    /// All `[Access]` tags unlocked across Stages 1–5.
    pub granted_access_tags: Vec<SkillAccessTier>,
    /// True if `[Access: Academic]` is frozen due to Stage 4B Dropout without a GED/University waiver.
    pub academic_tree_frozen: bool,
    /// Total Career Skill Level budget granted by Stage 5 Career Terms (`6` per term + `+1` per Honors term roll).
    pub career_skill_budget: usize,
    /// Total Elective / Hobby Skill Level budget (`2` youth elective + `2` per Career Term).
    pub elective_skill_budget: usize,
    /// Maximum additional levels any single skill can gain during Stage 5 (`2 * max(1, career_terms.len())`).
    pub max_post_school_gain_per_skill: usize,
    pub campaign_sqm: u8,
    pub operational_heat: u8,
    pub stage_history: Vec<String>,
}

fn bump_skill_floor(skills: &mut BTreeMap<String, usize>, skill_id: &str, min_level: usize) {
    let entry = skills.entry(skill_id.to_string()).or_insert(0);
    if *entry < min_level {
        *entry = min_level;
    }
}

fn add_skill_levels_capped(
    skills: &mut BTreeMap<String, usize>,
    skill_id: &str,
    add: usize,
    cap: usize,
) {
    let entry = skills.entry(skill_id.to_string()).or_insert(0);
    *entry = (*entry + add).min(cap);
}

/// Deterministically evaluates Stages 1–5 from [`LifepathBuildConfig`], returning the Stage 1–4 skill floor,
/// Stage 5 `[Access]` gates, Career/Elective skill budgets, and chronological history log.
#[must_use]
pub fn compute_lifepath_baseline_from_config(
    cfg: &LifepathBuildConfig,
    chars: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
) -> LifepathBaselineResult {
    let origin = resolve_socioeconomic_origin(cfg.origin_roll_2d20.clamp(2, 40));
    let sqm = origin.school_quality_modifier_sqm;
    let mut skills = initialize_universal_talents();
    let mut granted_access_tags = vec![SkillAccessTier::Universal, SkillAccessTier::Academic];
    let mut stage_history = Vec::new();

    // Stage 1: Ages 0–5 (Universal Talents + Early Environment Bonus Talent)
    let early_tal = if skills.contains_key(&cfg.stage1_early_talent) {
        cfg.stage1_early_talent.as_str()
    } else {
        origin
            .early_environment_bonus_talents
            .first()
            .copied()
            .unwrap_or("alertness_situational_awareness")
    };
    add_skill_levels_capped(&mut skills, early_tal, 1, 3);
    stage_history.push(format!(
        "Stage 1 (Ages 0–5): {} (2d20={}, SQM {:+}) — Early Talent '{}' -> N={}",
        origin.label,
        cfg.origin_roll_2d20.clamp(2, 40),
        sqm,
        early_tal,
        skills.get(early_tal).copied().unwrap_or(2)
    ));

    // Stage 2: Ages 6–11 (Elementary School Automatic Grants + Literacy & Math Rolls + Playground Talent)
    bump_skill_floor(&mut skills, "reading_comprehension", 1);
    bump_skill_floor(&mut skills, "writing_composition", 1);
    bump_skill_floor(&mut skills, "arithmetic", 1);
    bump_skill_floor(&mut skills, "digital_ai_literacy", 1);

    let mut stage2_lit_eval = evaluate_educational_advancement_roll(
        chars.dis,
        chars.mem,
        aptitudes.apt_lng,
        sqm,
        cfg.stage2_lit_pairs,
    );
    if let Some(ov) = cfg.stage2_lit_override {
        stage2_lit_eval.successes = ov.min(2);
    }
    match stage2_lit_eval.successes {
        0 => {}
        1 => {
            bump_skill_floor(&mut skills, "reading_comprehension", 2);
            bump_skill_floor(&mut skills, "writing_composition", 2);
        }
        _ => {
            bump_skill_floor(&mut skills, "reading_comprehension", 3);
            bump_skill_floor(&mut skills, "writing_composition", 2);
            bump_skill_floor(&mut skills, "history_political_science", 1);
        }
    }

    let mut stage2_math_eval = evaluate_educational_advancement_roll(
        chars.dis,
        chars.ana,
        aptitudes.apt_qnt,
        sqm,
        cfg.stage2_math_pairs,
    );
    if let Some(ov) = cfg.stage2_math_override {
        stage2_math_eval.successes = ov.min(2);
    }
    match stage2_math_eval.successes {
        0 => {}
        1 => {
            bump_skill_floor(&mut skills, "arithmetic", 2);
        }
        _ => {
            bump_skill_floor(&mut skills, "arithmetic", 3);
            bump_skill_floor(&mut skills, "elementary_algebra_geometry", 1);
        }
    }

    if skills.contains_key(&cfg.stage2_playground_talent) {
        add_skill_levels_capped(&mut skills, &cfg.stage2_playground_talent, 1, 3);
    }
    stage_history.push(format!(
        "Stage 2 (Ages 6–11): Elementary School — Literacy {} succ (TN {}), Math {} succ (TN {}), Recess Talent '{}' -> N={}",
        stage2_lit_eval.successes,
        stage2_lit_eval.tn_edu,
        stage2_math_eval.successes,
        stage2_math_eval.tn_edu,
        cfg.stage2_playground_talent,
        skills.get(&cfg.stage2_playground_talent).copied().unwrap_or(2)
    ));

    // Stage 3: Ages 12–15 (Junior High / Middle School + Age-15 Dropout Crisis Check)
    let mut stage3_math_eval = evaluate_educational_advancement_roll(
        chars.dis,
        chars.ana,
        aptitudes.apt_qnt,
        sqm,
        cfg.stage3_math_pairs,
    );
    if let Some(ov) = cfg.stage3_math_override {
        stage3_math_eval.successes = ov.min(2);
    }
    let arith_before_s3 = skills.get("arithmetic").copied().unwrap_or(1);
    if arith_before_s3 >= 2 {
        bump_skill_floor(&mut skills, "elementary_algebra_geometry", 1);
        match stage3_math_eval.successes {
            0 => {}
            1 => {
                bump_skill_floor(&mut skills, "elementary_algebra_geometry", 2);
            }
            _ => {
                bump_skill_floor(&mut skills, "elementary_algebra_geometry", 2);
                bump_skill_floor(&mut skills, "trigonometry", 1);
            }
        }
    } else {
        match stage3_math_eval.successes {
            0 => {}
            1 => {
                bump_skill_floor(&mut skills, "arithmetic", 2);
            }
            _ => {
                bump_skill_floor(&mut skills, "arithmetic", 2);
                bump_skill_floor(&mut skills, "elementary_algebra_geometry", 1);
            }
        }
    }

    let mut stage3_sci_eval = evaluate_educational_advancement_roll(
        chars.dis,
        chars.ana.max(chars.mem),
        aptitudes.apt_qnt.max(aptitudes.apt_bio),
        sqm,
        cfg.stage3_sci_pairs,
    );
    if let Some(ov) = cfg.stage3_sci_override {
        stage3_sci_eval.successes = ov.min(2);
    }
    match stage3_sci_eval.successes {
        0 => {
            bump_skill_floor(&mut skills, "general_biology_natural_history", 1);
        }
        1 => {
            bump_skill_floor(&mut skills, "general_biology_natural_history", 1);
            bump_skill_floor(&mut skills, "digital_ai_literacy", 2);
        }
        _ => {
            bump_skill_floor(&mut skills, "general_biology_natural_history", 2);
            bump_skill_floor(&mut skills, "digital_ai_literacy", 2);
        }
    }

    let mut stage3_hum_eval = evaluate_educational_advancement_roll(
        chars.dis,
        chars.mem,
        aptitudes.apt_lng,
        sqm,
        cfg.stage3_hum_pairs,
    );
    if let Some(ov) = cfg.stage3_hum_override {
        stage3_hum_eval.successes = ov.min(2);
    }
    if stage3_hum_eval.successes >= 1 {
        add_skill_levels_capped(&mut skills, "reading_comprehension", 1, 3);
        add_skill_levels_capped(&mut skills, "writing_composition", 1, 3);
        bump_skill_floor(&mut skills, "history_political_science", 1);
    }
    if stage3_hum_eval.successes >= 2 {
        bump_skill_floor(&mut skills, "history_political_science", 2);
    }

    let natural_dropout =
        cfg.age15_crisis_roll_2d20.clamp(2, 40) <= origin.adolescent_dropout_risk_max_2d20;
    let stage4_path = match cfg.stage4_path_override {
        Some(path) => path,
        None => match cfg.age15_dropout_override {
            Some(true) => Stage4Path::Path4BStreetGang,
            Some(false) => Stage4Path::Path4AHighSchool,
            None => {
                if natural_dropout {
                    Stage4Path::Path4BStreetGang
                } else {
                    Stage4Path::Path4AHighSchool
                }
            }
        },
    };
    let dropped_out_at_15 = stage4_path != Stage4Path::Path4AHighSchool;

    stage_history.push(format!(
        "Stage 3 (Ages 12–15): Junior High — Math {} succ, Science {} succ, Humanities {} succ; Age-15 Crisis Roll={} (Threshold <={}) -> {}",
        stage3_math_eval.successes,
        stage3_sci_eval.successes,
        stage3_hum_eval.successes,
        cfg.age15_crisis_roll_2d20.clamp(2, 40),
        origin.adolescent_dropout_risk_max_2d20,
        match stage4_path {
            Stage4Path::Path4AHighSchool => "Stayed in High School (Path 4A)",
            Stage4Path::Path4BStreetGang => "DROPPED OUT — Urban Underworld (Path 4B)",
            Stage4Path::Path4CRuralAgrarian => "RURAL / AGRARIAN / FRONTIER YOUTH (Path 4C)",
        }
    ));

    // Stage 4: Ages 15–18 (Path 4A High School vs. Path 4B Urban Dropout & Street Gang vs. Path 4C Rural & Developing-Region Youth)
    let mut academic_tree_frozen = false;
    let mut stage4a_hs_eval = None;

    match stage4_path {
        Stage4Path::Path4BStreetGang => {
            academic_tree_frozen = true;
            cfg.stage4b_gang_archetype.apply_grants(
                &mut skills,
                &mut granted_access_tags,
                chars.for_stat,
                true,
            );
            stage_history.push(format!(
                "Stage 4B (Ages 16–18): Dropout / {} — Academic Tree FROZEN; unlocked [Access: Illicit]",
                cfg.stage4b_gang_archetype.label()
            ));
        }
        Stage4Path::Path4CRuralAgrarian => {
            academic_tree_frozen = true;
            cfg.stage4c_rural_archetype
                .apply_grants(&mut skills, &mut granted_access_tags);
            stage_history.push(format!(
                "Stage 4C (Ages 15–18): Rural / Developing-Region Youth ({}) — Academic Tree FROZEN; unlocked [Access: Vocational/Licensed]",
                cfg.stage4c_rural_archetype.label()
            ));
        }
        Stage4Path::Path4AHighSchool => {
            let hs_sqm = if cfg.stage4a_hs_track == HighSchoolTrack::ApHonorsStemMagnet {
                sqm - 1
            } else {
                sqm
            };
            let mut hs_eval = evaluate_educational_advancement_roll(
                chars.dis,
                chars.ana.max(chars.mem),
                aptitudes.apt_qnt.max(aptitudes.apt_cmp),
                hs_sqm,
                cfg.stage4a_hs_pairs,
            );
            if let Some(ov) = cfg.stage4a_hs_override {
                hs_eval.successes = ov.min(2);
            }

            match cfg.stage4a_hs_track {
                HighSchoolTrack::StandardCollegePrep => {
                    bump_skill_floor(&mut skills, "arithmetic", 2);
                    bump_skill_floor(&mut skills, "elementary_algebra_geometry", 2);
                    bump_skill_floor(
                        &mut skills,
                        "trigonometry",
                        1 + usize::from(hs_eval.successes >= 1),
                    );
                    bump_skill_floor(
                        &mut skills,
                        "general_chemistry",
                        1 + usize::from(hs_eval.successes >= 2),
                    );
                    bump_skill_floor(&mut skills, "classical_mechanics_physics", 1);
                    bump_skill_floor(&mut skills, "foreign_language_specific", 1);
                    bump_skill_floor(&mut skills, "driving_wheeled_tracked", 1);
                }
                HighSchoolTrack::ApHonorsStemMagnet => {
                    bump_skill_floor(&mut skills, "arithmetic", 3);
                    bump_skill_floor(&mut skills, "digital_ai_literacy", 2);
                    bump_skill_floor(&mut skills, "elementary_algebra_geometry", 3);
                    bump_skill_floor(
                        &mut skills,
                        "trigonometry",
                        2 + usize::from(hs_eval.successes >= 1),
                    );
                    bump_skill_floor(
                        &mut skills,
                        "calculus_linear_algebra",
                        1 + usize::from(hs_eval.successes >= 1),
                    );
                    bump_skill_floor(&mut skills, "statistics_probability", 1);
                    bump_skill_floor(
                        &mut skills,
                        "programming_scripting",
                        1 + usize::from(hs_eval.successes >= 2),
                    );
                    bump_skill_floor(&mut skills, "classical_mechanics_physics", 2);
                    bump_skill_floor(&mut skills, "general_chemistry", 2);
                }
                HighSchoolTrack::VocationalTechnical => {
                    if !granted_access_tags.contains(&SkillAccessTier::VocationalLicensed) {
                        granted_access_tags.push(SkillAccessTier::VocationalLicensed);
                    }
                    bump_skill_floor(&mut skills, "arithmetic", 2);
                    bump_skill_floor(&mut skills, "elementary_algebra_geometry", 2);
                    bump_skill_floor(&mut skills, "manual_dexterity_tinkering", 2);
                    bump_skill_floor(&mut skills, "driving_wheeled_tracked", 1);
                    bump_skill_floor(
                        &mut skills,
                        "automotive_small_engine_mechanics",
                        1 + usize::from(hs_eval.successes >= 1),
                    );
                    bump_skill_floor(
                        &mut skills,
                        "electrical_power_systems",
                        1 + usize::from(hs_eval.successes >= 2),
                    );
                    bump_skill_floor(&mut skills, "metalworking_machining_blacksmithing", 1);
                }
            }
            stage_history.push(format!(
                "Stage 4A (Ages 16–18): {} — Advancement {} succ (TN {})",
                cfg.stage4a_hs_track.label(),
                hs_eval.successes,
                hs_eval.tn_edu
            ));
            stage4a_hs_eval = Some(hs_eval);
        }
    }

    // Stage 5: Adult 4-Year Career Terms (Ages 18+, 0..=4 terms)
    if cfg.adult_ged_waiver && academic_tree_frozen {
        academic_tree_frozen = false;
        if !granted_access_tags.contains(&SkillAccessTier::Academic) {
            granted_access_tags.push(SkillAccessTier::Academic);
        }
        stage_history.push(
            "Stage 5 Adult Education: Completed GED / Equivalency Waiver — [Access: Academic] unfrozen.".to_string(),
        );
    }

    let mut career_skill_budget = 0usize;
    let mut elective_skill_budget = 40usize; // 40 base youth/young-adult Experience SP
    let mut term_evals = Vec::with_capacity(cfg.career_terms.len());

    for (t_idx, term) in cfg.career_terms.iter().take(4).enumerate() {
        for &tag in term.track.granted_access_tags() {
            if tag == SkillAccessTier::Academic && academic_tree_frozen && !term.waiver_override {
                continue;
            }
            if tag == SkillAccessTier::Academic {
                academic_tree_frozen = false;
            }
            if !granted_access_tags.contains(&tag) {
                granted_access_tags.push(tag);
            }
        }
        if term.track == CareerTermTrack::IntelligenceServiceClandestine
            && !granted_access_tags.contains(&SkillAccessTier::Illicit)
        {
            granted_access_tags.push(SkillAccessTier::Illicit);
        }

        let c_mental_or_career = match term.track {
            CareerTermTrack::UniversityUndergraduateStem => chars.ana,
            CareerTermTrack::MedicalSchoolAndResidency => chars.mem.max(chars.ana),
            CareerTermTrack::MilitaryCombatArmsSpecOps => chars.for_stat.max(chars.dex),
            CareerTermTrack::LawEnforcementSwatDetective => chars.ins.max(chars.sig),
            CareerTermTrack::IntelligenceServiceClandestine => chars.ins.max(chars.elo),
            CareerTermTrack::OrganizedCrimeCartelSyndicate => chars.pre.max(chars.ins),
            CareerTermTrack::BlueCollarTradesMerchantMarine => chars.dex.max(chars.ana),
            CareerTermTrack::RuralAgrarianFrontierRanger => chars.ins.max(chars.sig.max(chars.fit)),
        };
        let apt_career = match term.track {
            CareerTermTrack::UniversityUndergraduateStem => {
                aptitudes.apt_qnt.max(aptitudes.apt_cmp)
            }
            CareerTermTrack::MedicalSchoolAndResidency => aptitudes.apt_bio,
            CareerTermTrack::MilitaryCombatArmsSpecOps => aptitudes.apt_fin.max(aptitudes.apt_kin),
            CareerTermTrack::LawEnforcementSwatDetective => {
                aptitudes.apt_soc.max(aptitudes.apt_fin)
            }
            CareerTermTrack::IntelligenceServiceClandestine => {
                aptitudes.apt_soc.max(aptitudes.apt_cmp)
            }
            CareerTermTrack::OrganizedCrimeCartelSyndicate => {
                aptitudes.apt_soc.max(aptitudes.apt_fin)
            }
            CareerTermTrack::BlueCollarTradesMerchantMarine => aptitudes.apt_mec,
            CareerTermTrack::RuralAgrarianFrontierRanger => {
                aptitudes.apt_bio.max(aptitudes.apt_mec)
            }
        };
        let mut t_eval = evaluate_educational_advancement_roll(
            chars.dis,
            c_mental_or_career,
            apt_career,
            0,
            term.term_roll_pairs,
        );
        if let Some(ov) = term.override_successes {
            t_eval.successes = ov.min(2);
        }
        let term_formal_sp = 120 + 20 * t_eval.successes.min(2);
        career_skill_budget += term_formal_sp;
        elective_skill_budget += 40;

        let age_start = 18 + (t_idx as u32) * 4;
        let age_end = age_start + 4;
        stage_history.push(format!(
            "Stage 5 Term {} (Ages {age_start}–{age_end}): {} — Roll {} succ (TN {}) -> +{} Formal SP (+40 Experience SP)",
            t_idx + 1,
            term.track.label(),
            t_eval.successes,
            t_eval.tn_edu,
            term_formal_sp
        ));
        term_evals.push(t_eval);
    }

    let num_terms = cfg.career_terms.len().min(4);
    let age_years = 18 + (num_terms as u32) * 4;
    let max_post_school_gain_per_skill = (2 * num_terms.max(1)).max(2);

    let default_sqm: u8 = match origin.class {
        SocioeconomicClass::DestituteUnderclassWarZone => 2,
        SocioeconomicClass::WorkingPoorLowerClass => 4,
        SocioeconomicClass::WorkingMiddleClass => 5,
        SocioeconomicClass::ProfessionalUpperMiddleClass => 7,
        SocioeconomicClass::AffluentElitePreparatory => 8,
        SocioeconomicClass::WealthyCorporateAristocraticElite => 9,
    };
    let default_heat: u8 = if cfg
        .career_terms
        .iter()
        .any(|t| t.track == CareerTermTrack::OrganizedCrimeCartelSyndicate)
    {
        2
    } else if stage4_path == Stage4Path::Path4BStreetGang
        || (stage4_path == Stage4Path::Path4CRuralAgrarian
            && cfg.stage4c_rural_archetype == RuralYouthArchetype::FrontierScoutMilitiaSmuggler)
    {
        1
    } else {
        0
    };

    LifepathBaselineResult {
        age_years,
        origin,
        stage2_lit_eval,
        stage2_math_eval,
        stage3_math_eval,
        stage3_sci_eval,
        stage3_hum_eval,
        dropped_out_at_15,
        stage4_path,
        stage4a_hs_eval,
        term_evals,
        baseline_skills: skills,
        granted_access_tags,
        academic_tree_frozen,
        career_skill_budget,
        elective_skill_budget,
        max_post_school_gain_per_skill,
        campaign_sqm: cfg
            .campaign_sqm_override
            .unwrap_or(default_sqm)
            .clamp(1, 10),
        operational_heat: cfg
            .operational_heat_override
            .unwrap_or(default_heat)
            .clamp(0, 10),
        stage_history,
    }
}

/// Returns true if `skill_id` is part of the formal curriculum (`recommended_skills()`) of any active Stage 5 Career Term.
#[must_use]
pub fn is_career_formal_skill(skill_id: &str, cfg: &LifepathBuildConfig) -> bool {
    cfg.career_terms
        .iter()
        .take(4)
        .any(|t| t.track.recommended_skills().contains(&skill_id))
}

/// Ledger tracking Formal Training SP (`FOC + INS`) and Experience SP (`INS`) spent across post-school skill advancements.
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
    pub total_levels_gained: usize,
    /// Total SP invested in each skill above its Stage 1–4 baseline floor.
    pub per_skill_sp_spent: BTreeMap<String, usize>,
}

/// Computes the exact Formal Training SP (`FOC + INS`) and Experience SP (`INS`) spent and remaining
/// for a character's current `skills` map relative to their `baseline.baseline_skills` floor.
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

    // First pass: Career-recommended skills (use Formal SP first)
    for (sk_id, &cur_n) in skills {
        let floor_n = baseline.baseline_skills.get(sk_id).copied().unwrap_or(0);
        if cur_n <= floor_n || !is_career_formal_skill(sk_id, cfg) {
            continue;
        }
        let mut sk_spent = 0usize;
        for step_n in (floor_n + 1)..=cur_n {
            total_levels_gained += 1;
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
        }
        per_skill_sp_spent.insert(sk_id.clone(), sk_spent);
    }

    // Second pass: Non-career elective skills (use Experience SP first, or Formal SP if Experience SP is full)
    for (sk_id, &cur_n) in skills {
        let floor_n = baseline.baseline_skills.get(sk_id).copied().unwrap_or(0);
        if cur_n <= floor_n || is_career_formal_skill(sk_id, cfg) {
            continue;
        }
        let mut sk_spent = 0usize;
        for step_n in (floor_n + 1)..=cur_n {
            total_levels_gained += 1;
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
                sk_spent += exp_cost;
            } else if formal_sp_spent + exp_cost <= formal_sp_budget {
                formal_sp_spent += exp_cost;
                sk_spent += exp_cost;
            } else {
                experience_sp_spent += exp_cost;
                sk_spent += exp_cost;
            }
        }
        per_skill_sp_spent.insert(sk_id.clone(), sk_spent);
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

/// Preview the [`SkillStepCostBreakdown`] and whether the character has enough SP to advance `skill_id` to `target_level`.
#[must_use]
pub fn preview_next_skill_step_sp(
    skill_id: &str,
    target_level: usize,
    cfg: &LifepathBuildConfig,
    ledger: &SkillAllocationSpLedger,
    chars: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
) -> (SkillStepCostBreakdown, bool) {
    let is_career = is_career_formal_skill(skill_id, cfg);
    if is_career {
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
/// DAG gates (`validate_skill_advancement`), the 5-factor SP step cost, and `max_post_school_gain_per_skill`.
pub fn auto_allocate_career_and_elective_skills(
    baseline: &LifepathBaselineResult,
    cfg: &LifepathBuildConfig,
    chars: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
    skills: &mut BTreeMap<String, usize>,
) {
    // Ensure all baseline skills are at least at their Stage 1-4 floor
    for (k, &v) in &baseline.baseline_skills {
        let cur = skills.entry(k.clone()).or_insert(0);
        if *cur < v {
            *cur = v;
        }
    }

    // Build priority candidate list from all Career Terms' recommended skills, followed by general staples
    let mut career_candidates: Vec<&'static str> = Vec::new();
    for term in cfg.career_terms.iter().take(4) {
        for &sk_id in term.track.recommended_skills() {
            if !career_candidates.contains(&sk_id) {
                career_candidates.push(sk_id);
            }
        }
    }
    let fallback_electives = [
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
    let mut all_candidates = career_candidates.clone();
    for &sk_id in &fallback_electives {
        if !all_candidates.contains(&sk_id) {
            all_candidates.push(sk_id);
        }
    }

    // Multi-pass allocation so parent prerequisites advance first and unlock child specializations
    for _pass in 0..6 {
        let mut made_progress = false;
        for &sk_id in &all_candidates {
            let cur_n = skills.get(sk_id).copied().unwrap_or(0);
            let floor_n = baseline.baseline_skills.get(sk_id).copied().unwrap_or(0);
            let gained_post_school = cur_n.saturating_sub(floor_n);
            if gained_post_school >= baseline.max_post_school_gain_per_skill || cur_n >= 5 {
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
                made_progress = true;
            }
        }
        if !made_progress {
            break;
        }
    }
}

/// Summary of Lifepath exponential skill progression (`N in 0..=10`) from study hours.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifepathSkillProgression {
    /// Unmodified study hours invested.
    pub raw_hours: f64,
    /// Effective hours after instruction quality and aptitude multipliers.
    pub effective_hours: f64,
    /// Achieved skill rank $N \in [0, 10]$.
    pub skill_level_n: i32,
    /// Next skill rank target $\min(10, N + 1)$.
    pub next_level_n: i32,
    /// Remaining effective hours needed to reach `next_level_n`.
    pub hours_to_next_level: f64,
    /// Cumulative effective hour thresholds for each rank `1..=10`.
    pub cumulative_thresholds: BTreeMap<i32, f64>,
}

fn round_1_dec(val: f64) -> f64 {
    (val * 10.0).round() / 10.0
}

/// Computes non-linear post-creation skill level $N \in [0, 10]$ from study hours using the
/// canonical Aptitude training time multiplier (`-15%` per `+1` APT, `+20%` per `-1` APT;
/// `characteristics_and_aptitudes.md` §9.1.3).
///
/// Each rank $N \ge 1$ requires $100 \times 2^{N - 1} \times \text{difficulty\_multiplier} \times \text{aptitude\_time\_multiplier}$ hours.
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
    let next_threshold = if achieved_level < 10 {
        cumulative_thresholds
            .get(&next_level)
            .copied()
            .unwrap_or(0.0)
    } else {
        cumulative_thresholds.get(&10).copied().unwrap_or(0.0)
    };
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
            origin_roll_2d20: 28, // Professional / Upper-Middle (SQM -2)
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
        assert_eq!(baseline.career_skill_budget, 160 + 140); // Term 1 Honors (160 Formal SP) + Term 2 Standard (140 Formal SP)
        assert_eq!(baseline.elective_skill_budget, 40 + 80); // 40 Youth Experience SP + 2*40 Term Experience SP

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

        // Verify Amara's rural/fieldcraft skills
        assert_eq!(
            amara.skills.get("hunting_trapping_field_butchery"),
            Some(&4)
        );
        assert_eq!(
            amara.skills.get("animal_handling_husbandry_riding"),
            Some(&2)
        );
        assert_eq!(
            amara
                .skills
                .get("herbalism_ethnobotany_natural_remedies"),
            Some(&2)
        );
        assert_eq!(
            amara.skills.get("veterinary_medicine_livestock_care"),
            Some(&2)
        );

        // Amara can advance Hunting & Trapping to N=5 and Veterinary Medicine to N=3,
        // but is locked out of Academic-only Organic Chemistry due to Stage 4C Academic Tree Freeze
        let hunt_audit = amara.audit_skill_unlock("hunting_trapping_field_butchery", 5);
        assert!(hunt_audit.is_valid);
        let vet_audit = amara.audit_skill_unlock("veterinary_medicine_livestock_care", 3);
        assert!(vet_audit.is_valid);
        let org_chem_audit = amara.audit_skill_unlock("organic_analytical_chemistry", 1);
        assert!(!org_chem_audit.is_valid);
    }
}
