//! Chronological 5-Stage Lifepath Creation, Socioeconomic Upkeep, Black-Market Procurement & Operational Heat
//! (`03-character/lifepaths_and_economy.md`).
//!
//! Implements:
//! - Stage 1: Socioeconomic Origin & School Quality Modifier (`SQM`) (`2d20` table) (§1.1, in [`crate::economy_gear`])
//! - Educational Advancement Roll (`2d20` cascade with $N = 2$ pairs vs. $\text{TN}_{\text{edu}} = 31 - C_{\text{study}} + \text{SQM}$) (§1.2)
//! - Stages 2, 3, 4A/4B/4C (High School Tracks vs. Dropout **Academic Tree Freeze**, Street Gang Archetypes, & Rural Youth),
//!   and Stage 5 Adult 4-Year Career Terms (§1.3–§1.6)
//! - Canonical Character Walkthrough Builders & Skill Lock Auditors (**Mateo Vance** §2.1, **Dr. Priya Nair** §2.2, **Amara Okafor** §2.3)
//! - Campaign Socioeconomic Upkeep (`SQM 1–10`), Black-Market `[Access]` Procurement (`Class I–IV`),
//!   and Operational `Heat` (`0–10`) (§3.1–§3.2, re-exported from [`crate::economy_gear`])

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::characteristics::{
    CharacterAptitudes, CharacterCharacteristics, compute_academic_study_controller,
};
use crate::define_coded_enum;
pub use crate::economy_gear::*;
use crate::skills_trees::{
    SkillAccessTier, SkillAdvancementValidation, initialize_universal_talents,
    validate_skill_advancement,
};
use crate::walkthroughs::insert_skill_ranks;
pub use crate::walkthroughs::{
    build_amara_okafor_lifepath, build_mateo_vance_lifepath, build_priya_nair_lifepath,
};

/// Outcome tier of an Educational Advancement Roll (`2d20` with $N = 2$ pairs) (§1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EducationalAdvancementTier {
    /// `0 Successes`: Stagnation / Remedial (`+0` levels in the subject).
    StagnationOrRemedial,
    /// `1 Success`: Standard Advancement (`+1` level in target node, or unlock next tier at $N = 1$).
    StandardAdvancement,
    /// `2+ Successes`: Honors / Accelerated Breakthrough (`+1` level in current node AND early unlock of next tier at $N = 1$, or `+2` levels split).
    HonorsAcceleratedBreakthrough,
}

/// Detailed result of evaluating a Lifepath Educational Advancement Roll (§1.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EducationalAdvancementResult {
    /// Computed academic study controller (`C_study`).
    pub c_study: i32,
    /// School Quality Modifier (`SQM`) applied to `TN_edu`.
    pub school_quality_modifier_sqm: i32,
    /// Clamped educational target number (`TN_edu = 31 - C_study + SQM`).
    pub tn_edu: i32,
    /// Clamped `2d20` pair sums for the two rolls.
    pub pair_sums: [i32; 2],
    /// Total successes scored across the two `2d20` pairs.
    pub successes: usize,
    /// Achieved advancement outcome tier.
    pub outcome_tier: EducationalAdvancementTier,
    /// Number of skill ranks gained from this roll (`0`, `1`, or `2`).
    pub skill_levels_gained: usize,
    /// Whether this roll grants an early child-tier unlock (`true` on `2+` successes).
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

fn evaluate_educational_advancement_with_override(
    dis: i32,
    c_mental: i32,
    aptitude_domain: i32,
    school_quality_modifier_sqm: i32,
    pair_rolls: [(i32, i32); 2],
    override_successes: Option<usize>,
) -> EducationalAdvancementResult {
    let mut eval = evaluate_educational_advancement_roll(
        dis,
        c_mental,
        aptitude_domain,
        school_quality_modifier_sqm,
        pair_rolls,
    );
    if let Some(ov) = override_successes {
        eval.successes = ov.min(2);
    }
    eval
}

define_coded_enum! {
    /// Stage 4 Adolescent Path (`Ages 15–18`) (§1.5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum Stage4Path [count = 3] {
        /// Path 4A: Formal High School Track (`[Access: Academic]`, or `+ [Access: Vocational/Licensed]` on Vocational Track).
        Path4AHighSchool => {
            code: "STAY_4A",
            label: "Path 4A: Formal High School (Ages 15–18)",
            aliases: ["PATH_4A", "HIGH_SCHOOL"],
        },
        /// Path 4B: Urban Dropout / Street Gang / Juvenile Underworld (`Academic Tree Freeze`, unlocks `[Access: Illicit]`).
        Path4BStreetGang => {
            code: "DROPOUT_4B",
            label: "Path 4B: Urban Dropout / Street Gang (Ages 15–18)",
            aliases: ["PATH_4B", "STREET_GANG"],
        },
        /// Path 4C: Rural, Agrarian & Developing-Region Youth (`Academic Tree Freeze` if leaving school at 15, unlocks `[Access: Vocational/Licensed]` & rural/fieldcraft skills).
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
        /// Track 1: Standard / College-Prep High School (`[Access: Academic]`).
        StandardCollegePrep => {
            code: "STANDARD_COLLEGE_PREP",
            label: "Standard / College-Prep High School",
            subtitle: "[Access: Academic] Advances Algebra & Geometry, Trigonometry, General Chemistry, Classical Physics, Foreign Language & Driving.",
            aliases: ["STANDARD"],
        },
        /// Track 2: AP / Honors / STEM Magnet Track (requires `Elementary Algebra >= 2` and `C_study >= 13`, `SQM -1` bonus).
        ApHonorsStemMagnet => {
            code: "AP_HONORS_STEM",
            label: "AP / Honors / STEM Magnet Academy",
            subtitle: "[Access: Academic] (Req: Algebra >= 2, ANA >= 11, DIS >= 11; SQM -1 bonus) Unlocks Calculus, Statistics, Programming, Classical Physics & Chemistry at N=2.",
            aliases: ["AP_HONORS_STEM_MAGNET", "STEM"],
        },
        /// Track 3: Vocational / Technical High School (unlocks `[Access: Vocational/Licensed]`).
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
        /// *Shooter / Enforcer*: `Short Blades` or `Pistols` to $N=2$ (and `Fully Automatic Weapons` to $N=1$ if `FOR >= 10`), `Intimidation` to $N=2$.
        ShooterEnforcer => {
            code: "SHOOTER_ENFORCER",
            label: "Shooter / Street Enforcer",
            subtitle: "Grants Streetwise N=2, Pistols N=2, Short Blades N=2, Intimidation N=2, plus Fully Automatic Weapons N=1 (if FOR >= 10).",
            aliases: ["SHOOTER"],
        },
        /// *Boost / Wheelman*: `Driving` to $N=2$, `Lockpicking & Physical Intrusion` to $N=2$, `Automotive Mechanics` to $N=1$.
        BoostWheelman => {
            code: "BOOST_WHEELMAN",
            label: "Boost / Wheelman & Tech Thief",
            subtitle: "Grants Streetwise N=2, Driving N=2, Lockpicking & Physical Intrusion N=2, Automotive Mechanics N=1.",
            aliases: ["WHEELMAN"],
        },
        /// *Runner / Lookout*: `Stealth` to $N=2$, `Alertness` to $N=2$, `Running` or `Acrobatics/Parkour` to $N=2$, `Sleight of Hand` to $N=1$.
        RunnerLookout => {
            code: "RUNNER_LOOKOUT",
            label: "Runner / Lookout & Urban Burglar",
            subtitle: "Grants Streetwise N=2, Stealth N=2, Alertness N=2, Acrobatics/Parkour N=2, Sleight of Hand N=1.",
            aliases: ["RUNNER"],
        },
        /// *Back-Alley Cook / Modder*: `Pharmacology & Toxicology` (illicit synthesis) or `Armourer & Gunsmithing` to $N=2$.
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
        /// *Subsistence Hunter, Trapper & Bush Guide*: `Hunting, Trapping & Field Butchery` $N=2$, `Wilderness Survival` $N=2$, `Long Guns` or `Archery & Bows` $N=2$, `Stealth` $N=2$.
        SubsistenceHunterTrapper => {
            code: "SUBSISTENCE_HUNTER_TRAPPER",
            label: "Subsistence Hunter, Trapper & Bush Guide",
            subtitle: "[Access: Vocational/Licensed] Grants Foraging N=2, Hunting & Trapping N=2, Wilderness Survival N=2, Long Guns N=2, Archery N=1, Stealth N=2.",
            aliases: ["HUNTER_TRAPPER", "HUNTER"],
        },
        /// *Agrarian Homesteader & Rural Mechanist*: `Agriculture, Agronomy & Crop Husbandry` $N=2$, `Animal Handling` $N=1$, `Automotive & Small-Engine Mechanics` $N=2$, `Carpentry & Woodworking` $N=1$, `Driving` $N=1$.
        AgrarianHomesteaderMechanist => {
            code: "AGRARIAN_HOMESTEADER_MECHANIST",
            label: "Agrarian Homesteader & Rural Mechanist",
            subtitle: "[Access: Vocational/Licensed] Grants Agriculture & Agronomy N=2, Animal Handling N=1, Automotive & Small-Engine Mechanics N=2, Carpentry N=1, Driving N=1.",
            aliases: ["AGRARIAN_HOMESTEADER", "AGRARIAN"],
        },
        /// *Pastoral Herder, Stockman & Outrider*: `Animal Handling, Husbandry & Riding` $N=2$, `Veterinary Medicine & Livestock Care` $N=1$, `Herbalism, Ethnobotany & Field Remedies` $N=1$, `Land Navigation` $N=1$.
        PastoralHerderOutrider => {
            code: "PASTORAL_HERDER_OUTRIDER",
            label: "Pastoral Herder, Stockman & Outrider",
            subtitle: "[Access: Vocational/Licensed] Grants Animal Handling & Riding N=2, Veterinary Care N=1, Herbalism & Ethnobotany N=1, First Aid N=1, Land Navigation N=1.",
            aliases: ["PASTORAL_HERDER", "PASTORAL"],
        },
        /// *Frontier Scout, Border Smuggler & Village Militia*: `Long Guns` $N=2$, `Camouflage & Concealment` $N=2$, `Land Navigation` $N=1$, `Alertness` $N=2$ (also unlocks `[Access: Illicit]`).
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
        /// University — STEM / Engineering (`B.S. / M.S. / Ph.D.`).
        UniversityUndergraduateStem => {
            code: "UNIVERSITY_STEM",
            label: "University — STEM / Engineering (B.S. / M.S. / Ph.D.)",
            aliases: ["UNIVERSITY_UNDERGRADUATE_STEM"],
        },
        /// Medical School & Surgical Residency.
        MedicalSchoolAndResidency => {
            code: "MEDICAL_SCHOOL",
            label: "Medical School & Surgical Residency",
            aliases: ["MEDICAL_SCHOOL_AND_RESIDENCY"],
        },
        /// Military Service — Combat Arms / Special Operations.
        MilitaryCombatArmsSpecOps => {
            code: "MILITARY_COMBAT_ARMS",
            label: "Military Service — Combat Arms / Special Operations",
            aliases: ["MILITARY_COMBAT_ARMS_SPEC_OPS"],
        },
        /// Law Enforcement / SWAT / Detective Bureau.
        LawEnforcementSwatDetective => {
            code: "LAW_ENFORCEMENT_SWAT",
            label: "Law Enforcement / SWAT / Detective Bureau",
            aliases: ["LAW_ENFORCEMENT_SWAT_DETECTIVE"],
        },
        /// Intelligence Service / Clandestine Operations.
        IntelligenceServiceClandestine => {
            code: "INTELLIGENCE_CLANDESTINE",
            label: "Intelligence Service / Clandestine Operations",
            aliases: ["INTELLIGENCE_SERVICE_CLANDESTINE"],
        },
        /// Organized Crime / Cartel / Street Syndicate.
        OrganizedCrimeCartelSyndicate => {
            code: "ORGANIZED_CRIME_SYNDICATE",
            label: "Organized Crime / Cartel / Street Syndicate",
            aliases: ["ORGANIZED_CRIME_CARTEL_SYNDICATE"],
        },
        /// Blue-Collar Trades / Industrial Tech / Merchant Marine.
        BlueCollarTradesMerchantMarine => {
            code: "BLUE_COLLAR_TRADES",
            label: "Blue-Collar Trades / Industrial Tech / Merchant Marine",
            aliases: ["BLUE_COLLAR_TRADES_MERCHANT_MARINE"],
        },
        /// Rural, Agrarian, Wilderness Guide & Frontier Ranger.
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

    /// Governing characteristic and aptitude values (`(c_mental_or_career, apt_career)`) for term advancement rolls.
    #[must_use]
    pub fn eval_controllers(
        self,
        chars: &CharacterCharacteristics,
        aptitudes: &CharacterAptitudes,
    ) -> (i32, i32) {
        match self {
            Self::UniversityUndergraduateStem => {
                (chars.ana, aptitudes.apt_qnt.max(aptitudes.apt_cmp))
            }
            Self::MedicalSchoolAndResidency => (chars.mem.max(chars.ana), aptitudes.apt_bio),
            Self::MilitaryCombatArmsSpecOps => (
                chars.for_stat.max(chars.dex),
                aptitudes.apt_fin.max(aptitudes.apt_kin),
            ),
            Self::LawEnforcementSwatDetective => (
                chars.ins.max(chars.sig),
                aptitudes.apt_soc.max(aptitudes.apt_fin),
            ),
            Self::IntelligenceServiceClandestine => (
                chars.ins.max(chars.elo),
                aptitudes.apt_soc.max(aptitudes.apt_cmp),
            ),
            Self::OrganizedCrimeCartelSyndicate => (
                chars.pre.max(chars.ins),
                aptitudes.apt_soc.max(aptitudes.apt_fin),
            ),
            Self::BlueCollarTradesMerchantMarine => (chars.dex.max(chars.ana), aptitudes.apt_mec),
            Self::RuralAgrarianFrontierRanger => (
                chars.ins.max(chars.sig.max(chars.fit)),
                aptitudes.apt_bio.max(aptitudes.apt_mec),
            ),
        }
    }

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
        let rank = |id: &str| skills.get(id).copied().unwrap_or(0);
        match self {
            Self::UniversityUndergraduateStem => {
                let trig = rank("trigonometry");
                let ok = !academic_tree_frozen && trig >= 2 && chars.ana >= 11 && chars.dis >= 10;
                (
                    ok,
                    format!(
                        "Academic Unfrozen: {}, Trig N={trig} (req >=2), ANA {} (req >=11), DIS {} (req >=10)",
                        !academic_tree_frozen, chars.ana, chars.dis
                    ),
                )
            }
            Self::MedicalSchoolAndResidency => {
                let bio = rank("general_biology_natural_history");
                let chem = rank("general_chemistry");
                let ok = !academic_tree_frozen
                    && bio >= 1
                    && chem >= 1
                    && chars.mem >= 12
                    && chars.ana >= 11
                    && chars.dis >= 12;
                (
                    ok,
                    format!(
                        "Academic Unfrozen: {}, Bio N={bio}, Chem N={chem}, MEM {} (req >=12), ANA {} (req >=11), DIS {} (req >=12)",
                        !academic_tree_frozen, chars.mem, chars.ana, chars.dis
                    ),
                )
            }
            Self::MilitaryCombatArmsSpecOps => {
                let ok = chars.for_stat >= 9 && chars.fit >= 10 && chars.hlt >= 10;
                (
                    ok,
                    format!(
                        "FOR {} (req >=9), FIT {} (req >=10), HLT {} (req >=10)",
                        chars.for_stat, chars.fit, chars.hlt
                    ),
                )
            }
            Self::LawEnforcementSwatDetective => {
                let ok = chars.dis >= 10 && chars.sig >= 10 && chars.ins >= 10;
                (
                    ok,
                    format!(
                        "DIS {} (req >=10), SIG {} (req >=10), INS {} (req >=10)",
                        chars.dis, chars.sig, chars.ins
                    ),
                )
            }
            Self::IntelligenceServiceClandestine => {
                let ok = chars.ins >= 11 && chars.dis >= 11 && chars.elo >= 10;
                (
                    ok,
                    format!(
                        "INS {} (req >=11), DIS {} (req >=11), ELO {} (req >=10)",
                        chars.ins, chars.dis, chars.elo
                    ),
                )
            }
            Self::OrganizedCrimeCartelSyndicate => {
                let ok = chars.pre >= 9 || chars.ins >= 9 || academic_tree_frozen;
                (
                    ok,
                    format!(
                        "PRE {} / INS {} (or Dropout/Street background)",
                        chars.pre, chars.ins
                    ),
                )
            }
            Self::BlueCollarTradesMerchantMarine => {
                let arith = rank("arithmetic");
                (arith >= 1, format!("Arithmetic N={arith} (req >=1)"))
            }
            Self::RuralAgrarianFrontierRanger => {
                let foraging = rank("foraging_campcraft");
                let ag = rank("agriculture_agronomy_pastoralism");
                let animal = rank("animal_handling_husbandry_riding");
                let ok = foraging >= 2
                    || ag >= 1
                    || animal >= 1
                    || (chars.fit >= 9 && chars.hlt >= 9);
                (
                    ok,
                    format!(
                        "Foraging N={foraging}, Agriculture N={ag}, Animal Handling N={animal}, FIT {}/HLT {}",
                        chars.fit, chars.hlt
                    ),
                )
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

/// Complete Lifepath Character Profile produced by the 5-Stage Chronological Lifepath Engine (§1–§2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LifepathCharacterProfile {
    /// Character name.
    pub name: String,
    /// Character age in years at the end of lifepath generation.
    pub age_years: u32,
    /// Stage 1 Socioeconomic Origin specification.
    pub origin: SocioeconomicOriginSpec,
    /// 20 Core Characteristics.
    pub characteristics: CharacterCharacteristics,
    /// 11 Domain Aptitudes.
    pub aptitudes: CharacterAptitudes,
    /// Unlocked skill ranks (`skill_id -> rank N`).
    pub skills: BTreeMap<String, usize>,
    /// Institutional `[Access]` tags unlocked across Stages 1–5.
    pub granted_access_tags: Vec<SkillAccessTier>,
    /// True if the character dropped out in Stage 3 / 4B / 4C without returning to adult basic education (locks `[Access: Academic]`).
    pub academic_tree_frozen: bool,
    /// Chronological narrative log of Stages 1–5.
    pub stage_history: Vec<String>,
    /// Campaign socioeconomic tier (`SQM 1–10`).
    pub campaign_sqm: u8,
    /// Operational heat score (`0–10`).
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

        if let Some(level) = skills.get_mut(early_talent_choice) {
            *level = (*level + 1).max(2);
        }

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

fn bump_skill_floor(skills: &mut BTreeMap<String, usize>, skill_id: &str, min_level: usize) {
    let entry = skills.entry(skill_id.to_string()).or_insert(0);
    if *entry < min_level {
        *entry = min_level;
    }
}

fn bump_skill_floors(skills: &mut BTreeMap<String, usize>, floors: &[(&str, usize)]) {
    for &(id, min_level) in floors {
        bump_skill_floor(skills, id, min_level);
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

/// Configuration for a single 4-Year Adult Career Term in Stage 5 (`Ages 18+`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareerTermConfig {
    /// Selected career term track.
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
    /// Stage 2: Optional manual override for Elementary Literacy successes (`0..=2`).
    pub stage2_lit_override: Option<usize>,
    /// Stage 2: Elementary Mathematics `2d20` pairs (`DIS/ANA + APT-QNT`).
    pub stage2_math_pairs: [(i32, i32); 2],
    /// Stage 2: Optional manual override for Elementary Mathematics successes (`0..=2`).
    pub stage2_math_override: Option<usize>,
    /// Stage 2: Playground / Recess Talent (`TAL` boosted by `+1`).
    pub stage2_playground_talent: String,
    /// Stage 3: Junior High Mathematics `2d20` pairs (`DIS/ANA + APT-QNT`).
    pub stage3_math_pairs: [(i32, i32); 2],
    /// Stage 3: Optional manual override for Junior High Mathematics successes (`0..=2`).
    pub stage3_math_override: Option<usize>,
    /// Stage 3: Junior High General Science `2d20` pairs (`DIS/ANA + APT-QNT`).
    pub stage3_sci_pairs: [(i32, i32); 2],
    /// Stage 3: Optional manual override for Junior High General Science successes (`0..=2`).
    pub stage3_sci_override: Option<usize>,
    /// Stage 3: Junior High Literacy & Humanities `2d20` pairs (`DIS/MEM + APT-LNG`).
    pub stage3_hum_pairs: [(i32, i32); 2],
    /// Stage 3: Optional manual override for Junior High Literacy & Humanities successes (`0..=2`).
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
    /// Stage 4A: Optional manual override for High School Advancement successes (`0..=2`).
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
    /// Character age in years at the end of Stage 5.
    pub age_years: u32,
    /// Stage 1 Socioeconomic Origin specification.
    pub origin: SocioeconomicOriginSpec,
    /// Stage 2 Elementary Literacy advancement roll evaluation.
    pub stage2_lit_eval: EducationalAdvancementResult,
    /// Stage 2 Elementary Mathematics advancement roll evaluation.
    pub stage2_math_eval: EducationalAdvancementResult,
    /// Stage 3 Junior High Mathematics advancement roll evaluation.
    pub stage3_math_eval: EducationalAdvancementResult,
    /// Stage 3 Junior High Science advancement roll evaluation.
    pub stage3_sci_eval: EducationalAdvancementResult,
    /// Stage 3 Junior High Humanities advancement roll evaluation.
    pub stage3_hum_eval: EducationalAdvancementResult,
    /// True if the character left formal academic high school at Age 15 (`Path4B` or `Path4C`).
    pub dropped_out_at_15: bool,
    /// Active Stage 4 adolescent path (`Path4A`, `Path4B`, or `Path4C`).
    pub stage4_path: Stage4Path,
    /// Stage 4A High School advancement roll evaluation (when `stage4_path == Path4AHighSchool`).
    pub stage4a_hs_eval: Option<EducationalAdvancementResult>,
    /// Stage 5 Adult Career Term advancement roll evaluations.
    pub term_evals: Vec<EducationalAdvancementResult>,
    /// Skills granted automatically by Stages 1–4 (`Universal Talents` + `Origin` + `Elementary` + `Junior High` + `High School / Gang / Rural`).
    /// Serves as the minimum floor in the Step 5 Skill Tree Builder.
    pub baseline_skills: BTreeMap<String, usize>,
    /// All `[Access]` tags unlocked across Stages 1–5.
    pub granted_access_tags: Vec<SkillAccessTier>,
    /// True if `[Access: Academic]` is frozen due to Stage 4B/4C Dropout without a GED/University waiver.
    pub academic_tree_frozen: bool,
    /// Total Formal Training SP budget granted by Stage 5 Career Terms (`120 + 20 * successes` per term).
    pub career_skill_budget: usize,
    /// Total Experience / Elective SP budget (`40` youth + `40` per Career Term).
    pub elective_skill_budget: usize,
    /// Maximum additional levels any single skill can gain during Stage 5 (`2 * max(1, career_terms.len())`).
    pub max_post_school_gain_per_skill: usize,
    /// Effective Campaign `SQM` Tier (`1–10`).
    pub campaign_sqm: u8,
    /// Effective Campaign Operational `Heat` (`0–10`).
    pub operational_heat: u8,
    /// Chronological narrative log of Stages 1–5.
    pub stage_history: Vec<String>,
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
    bump_skill_floors(
        &mut skills,
        &[
            ("reading_comprehension", 1),
            ("writing_composition", 1),
            ("arithmetic", 1),
            ("digital_ai_literacy", 1),
        ],
    );

    let stage2_lit_eval = evaluate_educational_advancement_with_override(
        chars.dis,
        chars.mem,
        aptitudes.apt_lng,
        sqm,
        cfg.stage2_lit_pairs,
        cfg.stage2_lit_override,
    );
    match stage2_lit_eval.successes {
        0 => {}
        1 => bump_skill_floors(
            &mut skills,
            &[("reading_comprehension", 2), ("writing_composition", 2)],
        ),
        _ => bump_skill_floors(
            &mut skills,
            &[
                ("reading_comprehension", 3),
                ("writing_composition", 2),
                ("history_political_science", 1),
            ],
        ),
    }

    let stage2_math_eval = evaluate_educational_advancement_with_override(
        chars.dis,
        chars.ana,
        aptitudes.apt_qnt,
        sqm,
        cfg.stage2_math_pairs,
        cfg.stage2_math_override,
    );
    match stage2_math_eval.successes {
        0 => {}
        1 => bump_skill_floor(&mut skills, "arithmetic", 2),
        _ => bump_skill_floors(
            &mut skills,
            &[("arithmetic", 3), ("elementary_algebra_geometry", 1)],
        ),
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
    let stage3_math_eval = evaluate_educational_advancement_with_override(
        chars.dis,
        chars.ana,
        aptitudes.apt_qnt,
        sqm,
        cfg.stage3_math_pairs,
        cfg.stage3_math_override,
    );
    let arith_before_s3 = skills.get("arithmetic").copied().unwrap_or(1);
    if arith_before_s3 >= 2 {
        bump_skill_floor(&mut skills, "elementary_algebra_geometry", 1);
        if stage3_math_eval.successes >= 1 {
            bump_skill_floor(&mut skills, "elementary_algebra_geometry", 2);
        }
        if stage3_math_eval.successes >= 2 {
            bump_skill_floor(&mut skills, "trigonometry", 1);
        }
    } else {
        if stage3_math_eval.successes >= 1 {
            bump_skill_floor(&mut skills, "arithmetic", 2);
        }
        if stage3_math_eval.successes >= 2 {
            bump_skill_floor(&mut skills, "elementary_algebra_geometry", 1);
        }
    }

    let stage3_sci_eval = evaluate_educational_advancement_with_override(
        chars.dis,
        chars.ana.max(chars.mem),
        aptitudes.apt_qnt.max(aptitudes.apt_bio),
        sqm,
        cfg.stage3_sci_pairs,
        cfg.stage3_sci_override,
    );
    bump_skill_floor(
        &mut skills,
        "general_biology_natural_history",
        1 + usize::from(stage3_sci_eval.successes >= 2),
    );
    if stage3_sci_eval.successes >= 1 {
        bump_skill_floor(&mut skills, "digital_ai_literacy", 2);
    }

    let stage3_hum_eval = evaluate_educational_advancement_with_override(
        chars.dis,
        chars.mem,
        aptitudes.apt_lng,
        sqm,
        cfg.stage3_hum_pairs,
        cfg.stage3_hum_override,
    );
    if stage3_hum_eval.successes >= 1 {
        add_skill_levels_capped(&mut skills, "reading_comprehension", 1, 3);
        add_skill_levels_capped(&mut skills, "writing_composition", 1, 3);
        bump_skill_floor(
            &mut skills,
            "history_political_science",
            1 + usize::from(stage3_hum_eval.successes >= 2),
        );
    }

    let natural_dropout =
        cfg.age15_crisis_roll_2d20.clamp(2, 40) <= origin.adolescent_dropout_risk_max_2d20;
    let stage4_path = match cfg.stage4_path_override {
        Some(path) => path,
        None => match cfg.age15_dropout_override {
            Some(true) => Stage4Path::Path4BStreetGang,
            Some(false) => Stage4Path::Path4AHighSchool,
            None if natural_dropout => Stage4Path::Path4BStreetGang,
            None => Stage4Path::Path4AHighSchool,
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

    // Stage 4: Ages 15–18
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
            let hs_sqm = sqm - i32::from(cfg.stage4a_hs_track == HighSchoolTrack::ApHonorsStemMagnet);
            let hs_eval = evaluate_educational_advancement_with_override(
                chars.dis,
                chars.ana.max(chars.mem),
                aptitudes.apt_qnt.max(aptitudes.apt_cmp),
                hs_sqm,
                cfg.stage4a_hs_pairs,
                cfg.stage4a_hs_override,
            );
            let s1 = usize::from(hs_eval.successes >= 1);
            let s2 = usize::from(hs_eval.successes >= 2);

            match cfg.stage4a_hs_track {
                HighSchoolTrack::StandardCollegePrep => bump_skill_floors(
                    &mut skills,
                    &[
                        ("arithmetic", 2),
                        ("elementary_algebra_geometry", 2),
                        ("trigonometry", 1 + s1),
                        ("general_chemistry", 1 + s2),
                        ("classical_mechanics_physics", 1),
                        ("foreign_language_specific", 1),
                        ("driving_wheeled_tracked", 1),
                    ],
                ),
                HighSchoolTrack::ApHonorsStemMagnet => bump_skill_floors(
                    &mut skills,
                    &[
                        ("arithmetic", 3),
                        ("digital_ai_literacy", 2),
                        ("elementary_algebra_geometry", 3),
                        ("trigonometry", 2 + s1),
                        ("calculus_linear_algebra", 1 + s1),
                        ("statistics_probability", 1),
                        ("programming_scripting", 1 + s2),
                        ("classical_mechanics_physics", 2),
                        ("general_chemistry", 2),
                    ],
                ),
                HighSchoolTrack::VocationalTechnical => {
                    if !granted_access_tags.contains(&SkillAccessTier::VocationalLicensed) {
                        granted_access_tags.push(SkillAccessTier::VocationalLicensed);
                    }
                    bump_skill_floors(
                        &mut skills,
                        &[
                            ("arithmetic", 2),
                            ("elementary_algebra_geometry", 2),
                            ("manual_dexterity_tinkering", 2),
                            ("driving_wheeled_tracked", 1),
                            ("automotive_small_engine_mechanics", 1 + s1),
                            ("electrical_power_systems", 1 + s2),
                            ("metalworking_machining_blacksmithing", 1),
                        ],
                    );
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
    let mut elective_skill_budget = 40usize;
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

        let (c_mental_or_career, apt_career) = term.track.eval_controllers(chars, aptitudes);
        let t_eval = evaluate_educational_advancement_with_override(
            chars.dis,
            c_mental_or_career,
            apt_career,
            0,
            term.term_roll_pairs,
            term.override_successes,
        );
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

    let default_sqm = origin.class.default_campaign_sqm();
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
