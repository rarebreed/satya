//! Canonical Lifepath Stage, Track, Gang, Rural Youth & Adult Career Term Specifications
//! (docs/ttrpg/03-character/lifepaths_and_economy.md §1.2–§1.6).
//!
//! Separated from [crate::lifepath_economy] so the static lifepath stage/track tables
//! and the chronological character generator each fit within a single agent read window.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::characteristics::{
    CharacterAptitudes, CharacterCharacteristics, compute_academic_study_controller,
};
use crate::define_coded_enum;
use crate::skills_trees::SkillAccessTier;

/// Outcome tier of an Educational Advancement Roll (`2d20` with $N = 2$ pairs)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.2).
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

/// Detailed result of evaluating a Lifepath Educational Advancement Roll
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EducationalAdvancementResult {
    pub c_study: i32,
    pub school_quality_modifier_sqm: i32,
    /// Clamped educational target number (`TN_edu = 31 - C_study + SQM`).
    pub tn_edu: i32,
    pub pair_sums: [i32; 2],
    pub successes: usize,
    pub outcome_tier: EducationalAdvancementTier,
    pub skill_levels_gained: usize,
    pub grants_early_tier_unlock: bool,
}

/// Evaluate a Lifepath Educational Advancement Roll (`N = 2` pairs of `2d20` against $\text{TN}_{\text{edu}} = 31 - C_{\text{study}} + \text{SQM}$)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.2).
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

pub(crate) fn evaluate_educational_advancement_with_override(
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum Stage4Path [count = 3] {
        Path4AHighSchool => {
            code: "STAY_4A",
            label: "Path 4A: Formal High School (Ages 15–18)",
            aliases: ["PATH_4A", "HIGH_SCHOOL"],
        },
        Path4BStreetGang => {
            code: "DROPOUT_4B",
            label: "Path 4B: Urban Dropout / Street Gang (Ages 15–18)",
            aliases: ["PATH_4B", "STREET_GANG"],
        },
        Path4CRuralAgrarian => {
            code: "RURAL_4C",
            label: "Path 4C: Rural, Agrarian & Developing-Region Youth (Ages 15–18)",
            aliases: ["PATH_4C", "RURAL_AGRARIAN"],
        },
    }
}

define_coded_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum HighSchoolTrack [count = 3] {
        StandardCollegePrep => {
            code: "STANDARD_COLLEGE_PREP",
            label: "Standard / College-Prep High School",
            subtitle: "[Access: Academic] Advances Algebra & Geometry, Trigonometry, General Chemistry, Classical Physics, Foreign Language & Driving.",
            aliases: ["STANDARD"],
        },
        ApHonorsStemMagnet => {
            code: "AP_HONORS_STEM",
            label: "AP / Honors / STEM Magnet Academy",
            subtitle: "[Access: Academic] (Req: Algebra >= 2, ANA >= 11, DIS >= 11; SQM -1 bonus) Unlocks Calculus, Statistics, Programming, Classical Physics & Chemistry at N=2.",
            aliases: ["AP_HONORS_STEM_MAGNET", "STEM"],
        },
        VocationalTechnical => {
            code: "VOCATIONAL_TECHNICAL",
            label: "Vocational / Technical High School",
            subtitle: "[Access: Academic + Vocational/Licensed] Unlocks Automotive Mechanics, Metalworking, Electrical Systems, Tinkering N=2 & Driving.",
            aliases: ["VOCATIONAL"],
        },
    }
}

define_coded_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum StreetGangArchetype [count = 4] {
        ShooterEnforcer => {
            code: "SHOOTER_ENFORCER",
            label: "Shooter / Street Enforcer",
            subtitle: "Grants Streetwise N=2, Pistols N=2, Short Blades N=2, Intimidation N=2, plus Fully Automatic Weapons N=1 (if FOR >= 10).",
            aliases: ["SHOOTER"],
        },
        BoostWheelman => {
            code: "BOOST_WHEELMAN",
            label: "Boost / Wheelman & Tech Thief",
            subtitle: "Grants Streetwise N=2, Driving N=2, Lockpicking & Physical Intrusion N=2, Automotive Mechanics N=1.",
            aliases: ["WHEELMAN"],
        },
        RunnerLookout => {
            code: "RUNNER_LOOKOUT",
            label: "Runner / Lookout & Urban Burglar",
            subtitle: "Grants Streetwise N=2, Stealth N=2, Alertness N=2, Acrobatics/Parkour N=2, Sleight of Hand N=1.",
            aliases: ["RUNNER"],
        },
        BackAlleyCookModder => {
            code: "COOK_MODDER",
            label: "Back-Alley Cook / Street Gunsmith Modder",
            subtitle: "Grants Streetwise N=2, Armourer & Gunsmithing N=2, Pharmacology & Toxicology N=2.",
            aliases: ["BACK_ALLEY_COOK_MODDER"],
        },
    }
}

define_coded_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum RuralYouthArchetype [count = 4] {
        SubsistenceHunterTrapper => {
            code: "SUBSISTENCE_HUNTER_TRAPPER",
            label: "Subsistence Hunter, Trapper & Bush Guide",
            subtitle: "[Access: Vocational/Licensed] Grants Foraging N=2, Hunting & Trapping N=2, Wilderness Survival N=2, Long Guns N=2, Archery N=1, Stealth N=2.",
            aliases: ["HUNTER_TRAPPER", "HUNTER"],
        },
        AgrarianHomesteaderMechanist => {
            code: "AGRARIAN_HOMESTEADER_MECHANIST",
            label: "Agrarian Homesteader & Rural Mechanist",
            subtitle: "[Access: Vocational/Licensed] Grants Agriculture & Agronomy N=2, Animal Handling N=1, Automotive & Small-Engine Mechanics N=2, Carpentry N=1, Driving N=1.",
            aliases: ["AGRARIAN_HOMESTEADER", "AGRARIAN"],
        },
        PastoralHerderOutrider => {
            code: "PASTORAL_HERDER_OUTRIDER",
            label: "Pastoral Herder, Stockman & Outrider",
            subtitle: "[Access: Vocational/Licensed] Grants Animal Handling & Riding N=2, Veterinary Care N=1, Herbalism & Ethnobotany N=1, First Aid N=1, Land Navigation N=1.",
            aliases: ["PASTORAL_HERDER", "PASTORAL"],
        },
        FrontierScoutMilitiaSmuggler => {
            code: "FRONTIER_SCOUT_MILITIA",
            label: "Frontier Scout, Border Smuggler & Village Militia",
            subtitle: "[Access: Vocational/Licensed + Illicit] Grants Long Guns N=2, Camouflage N=2, Stealth N=2, Alertness N=2, Land Navigation N=1.",
            aliases: ["FRONTIER_SCOUT_MILITIA_SMUGGLER", "FRONTIER_SCOUT"],
        },
    }
}

define_coded_enum! {
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
    /// Institutional `[Access]` tags granted upon entering this Career Term
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6).
    #[must_use]
    pub const fn granted_access_tags(self) -> &'static [SkillAccessTier] {
        match self {
            Self::UniversityUndergraduateStem | Self::MedicalSchoolAndResidency => &[
                SkillAccessTier::Academic,
                SkillAccessTier::VocationalLicensed,
            ],
            Self::MilitaryCombatArmsSpecOps | Self::LawEnforcementSwatDetective => &[
                SkillAccessTier::Restricted,
                SkillAccessTier::VocationalLicensed,
            ],
            Self::IntelligenceServiceClandestine => &[
                SkillAccessTier::Restricted,
                SkillAccessTier::Illicit,
                SkillAccessTier::VocationalLicensed,
            ],
            Self::OrganizedCrimeCartelSyndicate => &[SkillAccessTier::Illicit],
            Self::BlueCollarTradesMerchantMarine | Self::RuralAgrarianFrontierRanger => {
                &[SkillAccessTier::VocationalLicensed]
            }
        }
    }

    /// Maximum skill level advancement (`+1` to `+2`) per 4-year term in a single node
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6).
    #[must_use]
    pub const fn max_level_gain_per_term(self) -> usize {
        2
    }

    /// Governing characteristic and aptitude values (`(c_mental_or_career, apt_career)`) for term advancement rolls
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6).
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

    /// Summary of entry prerequisites (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6).
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

    /// Check whether a character natively meets the entry prerequisites for this Career Track
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6).
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
                let ok =
                    foraging >= 2 || ag >= 1 || animal >= 1 || (chars.fit >= 9 && chars.hlt >= 9);
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

    /// Ordered list of canonical curriculum & career skills taught by this track (ordered from parent foundation to advanced specialization)
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6).
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
