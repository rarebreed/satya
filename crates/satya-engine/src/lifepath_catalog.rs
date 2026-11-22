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

define_coded_enum! {
    /// Starting or attained Role Echelon within a Stage 5 Career Term (`EntryRank`, `MidSpecialist`, `SeniorLeadership`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum CareerEchelonTier [count = 3] {
        EntryRank => {
            code: "ENTRY_RANK",
            label: "Entry Rank / Apprentice / Junior",
            aliases: ["ENTRY", "ENLISTED"],
        },
        MidSpecialist => {
            code: "MID_SPECIALIST",
            label: "Mid-Tier Specialist / Junior Officer / Case Officer",
            aliases: ["MID", "SPECIALIST", "OFFICER"],
        },
        SeniorLeadership => {
            code: "SENIOR_LEADERSHIP",
            label: "Senior Leadership / Executive / Command",
            aliases: ["SENIOR", "LEADERSHIP", "EXECUTIVE"],
        },
    }
}

impl CareerEchelonTier {
    /// Additional Formal SP granted per 4-year term for higher role echelons (`0`, `+10`, `+20`).
    #[must_use]
    pub const fn formal_sp_bonus(self) -> usize {
        match self {
            Self::EntryRank => 0,
            Self::MidSpecialist => 10,
            Self::SeniorLeadership => 20,
        }
    }

    /// Bonus Contact Reach tier granted for senior leadership roles (`0`, `0`, `+1`).
    #[must_use]
    pub const fn contact_reach_bonus(self) -> u8 {
        match self {
            Self::EntryRank | Self::MidSpecialist => 0,
            Self::SeniorLeadership => 1,
        }
    }
}

define_coded_enum! {
    /// Tuition or patronage funding path used to enter a Stage 5 Career / Higher Education Term.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum CareerFundingMode [count = 5] {
        NotApplicable => {
            code: "NOT_APPLICABLE",
            label: "Direct Career / Salaried / No Tuition Gate",
        },
        SelfOrFamilyFunded => {
            code: "SELF_FAMILY_FUNDED",
            label: "Self / Family Wealth Funded (No New Debt)",
        },
        GiBillServiceFunded => {
            code: "GI_BILL_FUNDED",
            label: "GI Bill / State Service Tuition Waiver",
        },
        MeritScholarship => {
            code: "MERIT_SCHOLARSHIP",
            label: "Merit Scholarship (High C_study / Honors)",
        },
        DebtFunded => {
            code: "DEBT_FUNDED",
            label: "Student / Institutional Loan (+1 Debt Tier)",
        },
    }
}

/// Static Socio-Economic entry/exit profile for a Stage 5 [`CareerTermTrack`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CareerEconomicProfile {
    /// Minimum Wealth Tier (`1..=10`) to clear tuition without GI Bill, Merit Scholarship, or Debt.
    pub requires_tuition_wealth_min: Option<u8>,
    /// True if track favors Institutional Status (`Corporate`, `Law`, `Med`, `Intel`, `LawEnforcement`).
    pub high_status_institutional: bool,
    /// True if track imposes clearance/background friction when `operational_heat >= 3`.
    pub requires_clean_clearance: bool,
    /// True if track builds Street/Subculture Cred rather than corporate/civic standing.
    pub underworld_or_grassroots: bool,
    /// Target Wealth Tier (`1..=10`) this career pulls the character toward per term.
    pub target_wealth_tier: u8,
    /// Target Institutional Status (`1..=10`) this career pulls the character toward per term.
    pub target_institutional_status: u8,
    /// Street/Subculture Cred gain per successful term (`0..=2`).
    pub street_cred_gain: i8,
    /// Baseline Operational Heat change per term (`0` or `+1`).
    pub heat_gain: i8,
    /// Grounded Contact role generated on completing a term in this track.
    pub contact_role: &'static str,
    /// Base Reach tier (`1..=4`) of that contact.
    pub base_contact_reach: u8,
}

/// Inputs from prior Lifepath stages used to evaluate Stage 5 Career Term entry & starting echelon.
#[derive(Debug, Clone, Copy)]
pub struct CareerEntryContext<'a> {
    pub wealth_tier: u8,
    pub institutional_status: u8,
    pub street_cred: u8,
    pub current_heat: u8,
    pub academic_tree_frozen: bool,
    pub prior_tracks: &'a [CareerTermTrack],
    pub prior_honors_count: usize,
    pub chars: &'a CharacterCharacteristics,
    pub aptitudes: &'a CharacterAptitudes,
    pub echelon_override: Option<CareerEchelonTier>,
}

/// Socio-economic evaluation of entering a specific Stage 5 Career Term (`Socio-Economic -> Lifepath`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CareerTermEntryEvaluation {
    pub funding_mode: CareerFundingMode,
    pub debt_added: u8,
    /// Net TN_edu modifier (`< 0` patronage/cred edge, `> 0` glass-ceiling or clearance friction).
    pub entry_tn_mod: i32,
    pub resolved_echelon: CareerEchelonTier,
    pub role_title: &'static str,
    pub entry_notes: String,
}

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
    pub enum CareerTermTrack [count = 14] {
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
        UniversityHumanitiesLawEthics => {
            code: "UNIV_HUMANITIES",
            label: "University — Humanities, Law, Philosophy & Ethics (B.A. / J.D. / Ph.D.)",
            aliases: ["UNIVERSITY_HUMANITIES_LAW_ETHICS"],
        },
        CorporateManagementFinancePolicy => {
            code: "CORPORATE_POLICY",
            label: "Corporate Management, Quantitative Finance & Policy",
            aliases: ["CORPORATE_MANAGEMENT_FINANCE_POLICY"],
        },
        JournalismMediaPerformingArts => {
            code: "JOURNALISM_MEDIA",
            label: "Journalism, Media, PR & Performing Arts",
            aliases: ["JOURNALISM_MEDIA_PERFORMING_ARTS"],
        },
        GrassrootsActivismHumanitarianNgo => {
            code: "GRASSROOTS_NGO",
            label: "Grassroots Activism, Humanitarian NGO & Contemplative Ministry",
            aliases: ["GRASSROOTS_ACTIVISM_HUMANITARIAN_NGO"],
        },
        CybersecurityAiRedTeaming => {
            code: "CYBER_RED_TEAM",
            label: "Cybersecurity, AI Red-Teaming & Underground Collective",
            aliases: ["CYBERSECURITY_AI_RED_TEAMING"],
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
        CorporateSecOpsExecutiveProtection => {
            code: "CORPORATE_SECOPS",
            label: "Corporate SecOps, Private Military & Executive Protection",
            aliases: ["CORPORATE_SEC_OPS_EXECUTIVE_PROTECTION"],
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
            Self::UniversityUndergraduateStem
            | Self::MedicalSchoolAndResidency
            | Self::UniversityHumanitiesLawEthics
            | Self::CorporateManagementFinancePolicy => &[
                SkillAccessTier::Academic,
                SkillAccessTier::VocationalLicensed,
            ],
            Self::MilitaryCombatArmsSpecOps
            | Self::LawEnforcementSwatDetective
            | Self::CorporateSecOpsExecutiveProtection => &[
                SkillAccessTier::Restricted,
                SkillAccessTier::VocationalLicensed,
            ],
            Self::IntelligenceServiceClandestine => &[
                SkillAccessTier::Restricted,
                SkillAccessTier::Illicit,
                SkillAccessTier::VocationalLicensed,
            ],
            Self::CybersecurityAiRedTeaming => &[
                SkillAccessTier::VocationalLicensed,
                SkillAccessTier::Illicit,
            ],
            Self::OrganizedCrimeCartelSyndicate => &[SkillAccessTier::Illicit],
            Self::JournalismMediaPerformingArts
            | Self::GrassrootsActivismHumanitarianNgo
            | Self::BlueCollarTradesMerchantMarine
            | Self::RuralAgrarianFrontierRanger => &[SkillAccessTier::VocationalLicensed],
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
            Self::UniversityHumanitiesLawEthics => (
                chars.mem.max(chars.ins),
                aptitudes.apt_lng.max(aptitudes.apt_soc),
            ),
            Self::CorporateManagementFinancePolicy => (
                chars.ana.max(chars.elo).max(chars.bea),
                aptitudes.apt_qnt.max(aptitudes.apt_soc),
            ),
            Self::JournalismMediaPerformingArts => (
                chars.cre.max(chars.elo).max(chars.sav),
                aptitudes
                    .apt_lng
                    .max(aptitudes.apt_soc)
                    .max(aptitudes.apt_aud),
            ),
            Self::GrassrootsActivismHumanitarianNgo => (
                chars.ins.max(chars.elo).max(chars.wrm),
                aptitudes.apt_soc.max(aptitudes.apt_bio),
            ),
            Self::CybersecurityAiRedTeaming => (
                chars.ana.max(chars.cre),
                aptitudes.apt_cmp.max(aptitudes.apt_qnt),
            ),
            Self::MilitaryCombatArmsSpecOps => (
                chars.for_stat.max(chars.dex),
                aptitudes.apt_fin.max(aptitudes.apt_kin),
            ),
            Self::LawEnforcementSwatDetective => (
                chars.ins.max(chars.sig).max(chars.sav),
                aptitudes.apt_soc.max(aptitudes.apt_fin),
            ),
            Self::CorporateSecOpsExecutiveProtection => (
                chars.dis.max(chars.sig).max(chars.bea),
                aptitudes.apt_fin.max(aptitudes.apt_soc),
            ),
            Self::IntelligenceServiceClandestine => (
                chars.ins.max(chars.elo).max(chars.sav).max(chars.bea),
                aptitudes.apt_soc.max(aptitudes.apt_cmp),
            ),
            Self::OrganizedCrimeCartelSyndicate => (
                chars.pre.max(chars.ins).max(chars.sav),
                aptitudes.apt_soc.max(aptitudes.apt_fin),
            ),
            Self::BlueCollarTradesMerchantMarine => (chars.dex.max(chars.ana), aptitudes.apt_mec),
            Self::RuralAgrarianFrontierRanger => (
                chars.ins.max(chars.sig.max(chars.fit)),
                aptitudes.apt_bio.max(aptitudes.apt_mec),
            ),
        }
    }

    /// Canonical Socio-Economic Entry & Exit profile for this Stage 5 Career Track.
    #[must_use]
    pub const fn economic_profile(self) -> CareerEconomicProfile {
        match self {
            Self::UniversityUndergraduateStem => CareerEconomicProfile {
                requires_tuition_wealth_min: Some(6),
                high_status_institutional: false,
                requires_clean_clearance: false,
                underworld_or_grassroots: false,
                target_wealth_tier: 6,
                target_institutional_status: 6,
                street_cred_gain: 0,
                heat_gain: 0,
                contact_role: "Faculty Lab Advisor / Tech Incubator Alum",
                base_contact_reach: 2,
            },
            Self::MedicalSchoolAndResidency => CareerEconomicProfile {
                requires_tuition_wealth_min: Some(7),
                high_status_institutional: true,
                requires_clean_clearance: true,
                underworld_or_grassroots: false,
                target_wealth_tier: 7,
                target_institutional_status: 7,
                street_cred_gain: 0,
                heat_gain: 0,
                contact_role: "Attending Trauma Surgeon / Hospital Chief",
                base_contact_reach: 3,
            },
            Self::UniversityHumanitiesLawEthics => CareerEconomicProfile {
                requires_tuition_wealth_min: Some(6),
                high_status_institutional: true,
                requires_clean_clearance: false,
                underworld_or_grassroots: false,
                target_wealth_tier: 6,
                target_institutional_status: 7,
                street_cred_gain: 0,
                heat_gain: 0,
                contact_role: "Law Clerk / Ethics Review Fellow",
                base_contact_reach: 2,
            },
            Self::CorporateManagementFinancePolicy => CareerEconomicProfile {
                requires_tuition_wealth_min: Some(6),
                high_status_institutional: true,
                requires_clean_clearance: false,
                underworld_or_grassroots: false,
                target_wealth_tier: 8,
                target_institutional_status: 8,
                street_cred_gain: 0,
                heat_gain: 0,
                contact_role: "Corporate VP / Regulatory Lobbyist",
                base_contact_reach: 3,
            },
            Self::JournalismMediaPerformingArts => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: false,
                requires_clean_clearance: false,
                underworld_or_grassroots: true,
                target_wealth_tier: 5,
                target_institutional_status: 5,
                street_cred_gain: 1,
                heat_gain: 0,
                contact_role: "Investigative Editor / Producer",
                base_contact_reach: 2,
            },
            Self::GrassrootsActivismHumanitarianNgo => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: false,
                requires_clean_clearance: false,
                underworld_or_grassroots: true,
                target_wealth_tier: 4,
                target_institutional_status: 4,
                street_cred_gain: 2,
                heat_gain: 0,
                contact_role: "Mutual-Aid Director / Union Organizer",
                base_contact_reach: 2,
            },
            Self::CybersecurityAiRedTeaming => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: false,
                requires_clean_clearance: false,
                underworld_or_grassroots: true,
                target_wealth_tier: 6,
                target_institutional_status: 5,
                street_cred_gain: 1,
                heat_gain: 0,
                contact_role: "Zero-Day Broker / Red-Team Lead",
                base_contact_reach: 2,
            },
            Self::MilitaryCombatArmsSpecOps => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: false,
                requires_clean_clearance: true,
                underworld_or_grassroots: false,
                target_wealth_tier: 5,
                target_institutional_status: 6,
                street_cred_gain: 1,
                heat_gain: 0,
                contact_role: "Platoon Sergeant / Arms Quartermaster",
                base_contact_reach: 2,
            },
            Self::LawEnforcementSwatDetective => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: true,
                requires_clean_clearance: true,
                underworld_or_grassroots: false,
                target_wealth_tier: 5,
                target_institutional_status: 7,
                street_cred_gain: 0,
                heat_gain: 0,
                contact_role: "Precinct Detective / Forensic Lab Supervisor",
                base_contact_reach: 2,
            },
            Self::CorporateSecOpsExecutiveProtection => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: true,
                requires_clean_clearance: true,
                underworld_or_grassroots: false,
                target_wealth_tier: 7,
                target_institutional_status: 7,
                street_cred_gain: 0,
                heat_gain: 0,
                contact_role: "SecOps Watch Commander / Private Contractor",
                base_contact_reach: 3,
            },
            Self::IntelligenceServiceClandestine => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: true,
                requires_clean_clearance: true,
                underworld_or_grassroots: false,
                target_wealth_tier: 6,
                target_institutional_status: 8,
                street_cred_gain: 1,
                heat_gain: 0,
                contact_role: "Desk Handler / Signals Station Liaison",
                base_contact_reach: 3,
            },
            Self::OrganizedCrimeCartelSyndicate => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: false,
                requires_clean_clearance: false,
                underworld_or_grassroots: true,
                target_wealth_tier: 6,
                target_institutional_status: 2,
                street_cred_gain: 2,
                heat_gain: 1,
                contact_role: "Syndicate Fixer / Quartermaster",
                base_contact_reach: 2,
            },
            Self::BlueCollarTradesMerchantMarine => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: false,
                requires_clean_clearance: false,
                underworld_or_grassroots: false,
                target_wealth_tier: 5,
                target_institutional_status: 4,
                street_cred_gain: 1,
                heat_gain: 0,
                contact_role: "Port Master / Union Master Machinist",
                base_contact_reach: 2,
            },
            Self::RuralAgrarianFrontierRanger => CareerEconomicProfile {
                requires_tuition_wealth_min: None,
                high_status_institutional: false,
                requires_clean_clearance: false,
                underworld_or_grassroots: true,
                target_wealth_tier: 4,
                target_institutional_status: 4,
                street_cred_gain: 1,
                heat_gain: 0,
                contact_role: "County Warden / Frontier Bush Pilot",
                base_contact_reach: 2,
            },
        }
    }

    /// Track-specific role title for a given [`CareerEchelonTier`].
    #[must_use]
    pub const fn echelon_role_title(self, echelon: CareerEchelonTier) -> &'static str {
        match (self, echelon) {
            (Self::UniversityUndergraduateStem, CareerEchelonTier::EntryRank) => "B.S. STEM Undergraduate",
            (Self::UniversityUndergraduateStem, CareerEchelonTier::MidSpecialist) => "M.S. / Ph.D. Research Fellow",
            (Self::UniversityUndergraduateStem, CareerEchelonTier::SeniorLeadership) => "Principal Investigator / Lab Lead",
            (Self::MedicalSchoolAndResidency, CareerEchelonTier::EntryRank) => "Medical Student / Clinical Intern",
            (Self::MedicalSchoolAndResidency, CareerEchelonTier::MidSpecialist) => "Surgical / Trauma Resident",
            (Self::MedicalSchoolAndResidency, CareerEchelonTier::SeniorLeadership) => "Attending Physician / Department Chief",
            (Self::UniversityHumanitiesLawEthics, CareerEchelonTier::EntryRank) => "B.A. Humanities / Pre-Law Scholar",
            (Self::UniversityHumanitiesLawEthics, CareerEchelonTier::MidSpecialist) => "J.D. Fellow / Policy Ethicist",
            (Self::UniversityHumanitiesLawEthics, CareerEchelonTier::SeniorLeadership) => "Senior Counsel / Constitutional Scholar",
            (Self::CorporateManagementFinancePolicy, CareerEchelonTier::EntryRank) => "Liability-Sink Junior Analyst",
            (Self::CorporateManagementFinancePolicy, CareerEchelonTier::MidSpecialist) => "Director / Quantitative Strategist",
            (Self::CorporateManagementFinancePolicy, CareerEchelonTier::SeniorLeadership) => "VP / Executive Fast-Track",
            (Self::JournalismMediaPerformingArts, CareerEchelonTier::EntryRank) => "Stringer / Indie Performer",
            (Self::JournalismMediaPerformingArts, CareerEchelonTier::MidSpecialist) => "Investigative Beat Correspondent",
            (Self::JournalismMediaPerformingArts, CareerEchelonTier::SeniorLeadership) => "Senior Editor / Bureau Producer",
            (Self::GrassrootsActivismHumanitarianNgo, CareerEchelonTier::EntryRank) => "Clinic Volunteer / Field Organizer",
            (Self::GrassrootsActivismHumanitarianNgo, CareerEchelonTier::MidSpecialist) => "Relief Coordinator / Lead Mediator",
            (Self::GrassrootsActivismHumanitarianNgo, CareerEchelonTier::SeniorLeadership) => "Coalition Director / Envoy",
            (Self::CybersecurityAiRedTeaming, CareerEchelonTier::EntryRank) => "SOC Analyst / Bug-Bounty Hunter",
            (Self::CybersecurityAiRedTeaming, CareerEchelonTier::MidSpecialist) => "Red-Team Exploit Engineer",
            (Self::CybersecurityAiRedTeaming, CareerEchelonTier::SeniorLeadership) => "Zero-Day Cell Architect",
            (Self::MilitaryCombatArmsSpecOps, CareerEchelonTier::EntryRank) => "Enlisted Specialist / Infantry",
            (Self::MilitaryCombatArmsSpecOps, CareerEchelonTier::MidSpecialist) => "Junior Officer (O-1/O-2) or Senior NCO",
            (Self::MilitaryCombatArmsSpecOps, CareerEchelonTier::SeniorLeadership) => "Special Operations Detachment Commander",
            (Self::LawEnforcementSwatDetective, CareerEchelonTier::EntryRank) => "Patrol Officer / Junior Agent",
            (Self::LawEnforcementSwatDetective, CareerEchelonTier::MidSpecialist) => "Detective / SWAT Tactical Operator",
            (Self::LawEnforcementSwatDetective, CareerEchelonTier::SeniorLeadership) => "Task Force Commander / Bureau Chief",
            (Self::CorporateSecOpsExecutiveProtection, CareerEchelonTier::EntryRank) => "Site Guard / Perimeter Contractor",
            (Self::CorporateSecOpsExecutiveProtection, CareerEchelonTier::MidSpecialist) => "Executive Protection Detail Lead",
            (Self::CorporateSecOpsExecutiveProtection, CareerEchelonTier::SeniorLeadership) => "SecOps Operations Director",
            (Self::IntelligenceServiceClandestine, CareerEchelonTier::EntryRank) => "Junior Desk Analyst / Watch Officer",
            (Self::IntelligenceServiceClandestine, CareerEchelonTier::MidSpecialist) => "Clandestine Field Case Officer",
            (Self::IntelligenceServiceClandestine, CareerEchelonTier::SeniorLeadership) => "Station Chief / Directorate Handler",
            (Self::OrganizedCrimeCartelSyndicate, CareerEchelonTier::EntryRank) => "Street Soldier / Wheelman / Runner",
            (Self::OrganizedCrimeCartelSyndicate, CareerEchelonTier::MidSpecialist) => "Crew Enforcer / White-Collar Launderer",
            (Self::OrganizedCrimeCartelSyndicate, CareerEchelonTier::SeniorLeadership) => "Syndicate Lieutenant / Capo",
            (Self::BlueCollarTradesMerchantMarine, CareerEchelonTier::EntryRank) => "Apprentice Tradesperson / Deckhand",
            (Self::BlueCollarTradesMerchantMarine, CareerEchelonTier::MidSpecialist) => "Journeyman Machinist / Chief Mate",
            (Self::BlueCollarTradesMerchantMarine, CareerEchelonTier::SeniorLeadership) => "Master Foreman / Port Chief Engineer",
            (Self::RuralAgrarianFrontierRanger, CareerEchelonTier::EntryRank) => "Field Hand / Trainee Tracker",
            (Self::RuralAgrarianFrontierRanger, CareerEchelonTier::MidSpecialist) => "Licensed Frontier Ranger / Homesteader",
            (Self::RuralAgrarianFrontierRanger, CareerEchelonTier::SeniorLeadership) => "District Game Warden / Caravan Master",
        }
    }

    /// Evaluates how entering Socio-Economic state (`wealth`, `institutional_status`, `street_cred`, `heat`)
    /// modifies tuition/debt, advancement TN, and starting Role Echelon for this Career Track.
    #[must_use]
    pub fn evaluate_socioeconomic_entry(self, ctx: &CareerEntryContext<'_>) -> CareerTermEntryEvaluation {
        let ep = self.economic_profile();
        let (c_career, apt_career) = self.eval_controllers(ctx.chars, ctx.aptitudes);
        let c_study = compute_academic_study_controller(ctx.chars.dis, c_career, apt_career);

        let has_military = ctx
            .prior_tracks
            .contains(&Self::MilitaryCombatArmsSpecOps);
        let has_degree = ctx.prior_tracks.iter().any(|t| {
            matches!(
                t,
                Self::UniversityUndergraduateStem
                    | Self::MedicalSchoolAndResidency
                    | Self::UniversityHumanitiesLawEthics
            )
        });
        let same_track_count = ctx.prior_tracks.iter().filter(|&&t| t == self).count();

        let (funding_mode, debt_added) = match ep.requires_tuition_wealth_min {
            Some(min_w) => {
                if ctx.wealth_tier >= min_w {
                    (CareerFundingMode::SelfOrFamilyFunded, 0)
                } else if has_military {
                    (CareerFundingMode::GiBillServiceFunded, 0)
                } else if c_study >= 13 || ctx.prior_honors_count >= 1 {
                    (CareerFundingMode::MeritScholarship, 0)
                } else {
                    (CareerFundingMode::DebtFunded, 1)
                }
            }
            None => (CareerFundingMode::NotApplicable, 0),
        };

        let mut entry_tn_mod = 0i32;
        let mut notes = Vec::new();
        if funding_mode != CareerFundingMode::NotApplicable {
            notes.push(funding_mode.label().to_string());
        }

        if ep.high_status_institutional {
            if ctx.institutional_status >= 7 {
                entry_tn_mod -= 2;
                notes.push("Elite patronage (-2 TN)".to_string());
            } else if ctx.institutional_status <= 3 {
                entry_tn_mod += 2;
                notes.push("Glass-ceiling friction (+2 TN)".to_string());
            }
        }
        if ep.underworld_or_grassroots {
            if ctx.street_cred >= 2 {
                entry_tn_mod -= 2;
                notes.push("Street/community cred (-2 TN)".to_string());
            } else if ctx.institutional_status >= 8 && ctx.street_cred == 0 {
                entry_tn_mod += 2;
                notes.push("Outsider privilege distrust (+2 TN)".to_string());
            }
        }
        if ep.requires_clean_clearance && ctx.current_heat >= 3 {
            entry_tn_mod += 3;
            notes.push(format!("Clearance scrutiny at Heat {} (+3 TN)", ctx.current_heat));
        }

        let natural_echelon = match self {
            Self::UniversityUndergraduateStem
            | Self::MedicalSchoolAndResidency
            | Self::UniversityHumanitiesLawEthics => {
                if same_track_count >= 1 && (ctx.prior_tracks.len() >= 2 || ctx.institutional_status >= 7) {
                    CareerEchelonTier::SeniorLeadership
                } else if has_degree || same_track_count >= 1 {
                    CareerEchelonTier::MidSpecialist
                } else {
                    CareerEchelonTier::EntryRank
                }
            }
            Self::CorporateManagementFinancePolicy => {
                if (has_degree || same_track_count >= 1)
                    && (ctx.institutional_status >= 7
                        || (ctx.chars.elo >= 12 && ctx.chars.bea >= 11))
                {
                    CareerEchelonTier::SeniorLeadership
                } else if has_degree || same_track_count >= 1 || ctx.institutional_status >= 6 {
                    CareerEchelonTier::MidSpecialist
                } else {
                    CareerEchelonTier::EntryRank
                }
            }
            Self::MilitaryCombatArmsSpecOps => {
                if same_track_count >= 1
                    && (has_degree || ctx.institutional_status >= 7 || ctx.chars.pre >= 12)
                {
                    CareerEchelonTier::SeniorLeadership
                } else if has_degree || same_track_count >= 1 {
                    CareerEchelonTier::MidSpecialist
                } else {
                    CareerEchelonTier::EntryRank
                }
            }
            Self::LawEnforcementSwatDetective
            | Self::CorporateSecOpsExecutiveProtection
            | Self::IntelligenceServiceClandestine => {
                if same_track_count >= 1
                    && (ctx.institutional_status >= 7 || ctx.chars.sav >= 12 || ctx.chars.bea >= 12)
                {
                    CareerEchelonTier::SeniorLeadership
                } else if has_degree
                    || same_track_count >= 1
                    || has_military
                    || ctx.institutional_status >= 6
                {
                    CareerEchelonTier::MidSpecialist
                } else {
                    CareerEchelonTier::EntryRank
                }
            }
            Self::OrganizedCrimeCartelSyndicate => {
                if (same_track_count >= 1 && ctx.street_cred >= 3)
                    || (ctx.wealth_tier >= 7 && ctx.chars.sav >= 11)
                {
                    CareerEchelonTier::SeniorLeadership
                } else if same_track_count >= 1
                    || ctx.street_cred >= 2
                    || (ctx.wealth_tier >= 6 && !ctx.academic_tree_frozen)
                {
                    CareerEchelonTier::MidSpecialist
                } else {
                    CareerEchelonTier::EntryRank
                }
            }
            Self::JournalismMediaPerformingArts
            | Self::GrassrootsActivismHumanitarianNgo
            | Self::CybersecurityAiRedTeaming
            | Self::BlueCollarTradesMerchantMarine
            | Self::RuralAgrarianFrontierRanger => {
                if same_track_count >= 1
                    && (ctx.street_cred >= 3
                        || ctx.institutional_status >= 7
                        || ctx.chars.elo.max(ctx.chars.wrm) >= 13)
                {
                    CareerEchelonTier::SeniorLeadership
                } else if same_track_count >= 1
                    || has_degree
                    || ctx.street_cred >= 2
                    || ctx.institutional_status >= 6
                {
                    CareerEchelonTier::MidSpecialist
                } else {
                    CareerEchelonTier::EntryRank
                }
            }
        };
        let resolved_echelon = ctx.echelon_override.unwrap_or(natural_echelon);
        let role_title = self.echelon_role_title(resolved_echelon);
        if notes.is_empty() {
            notes.push("Standard entry".to_string());
        }

        CareerTermEntryEvaluation {
            funding_mode,
            debt_added,
            entry_tn_mod,
            resolved_echelon,
            role_title,
            entry_notes: notes.join(" · "),
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
            Self::UniversityHumanitiesLawEthics => {
                "High School Diploma (or GED), Reading >= 2, Writing >= 2, MEM >= 10, INS >= 10"
            }
            Self::CorporateManagementFinancePolicy => {
                "High School Diploma (or University), Algebra >= 2, ANA >= 11, ELO >= 10"
            }
            Self::JournalismMediaPerformingArts => {
                "Writing >= 2 (or Artistic Portfolio), CRE >= 10 or ELO >= 10"
            }
            Self::GrassrootsActivismHumanitarianNgo => {
                "Any Origin (Empathy >= 1 or Persuasion >= 1, INS >= 10 or CPS >= 12)"
            }
            Self::CybersecurityAiRedTeaming => {
                "Programming >= 1 or Digital Literacy >= 1, ANA >= 11, CRE >= 10"
            }
            Self::MilitaryCombatArmsSpecOps => "FOR >= 9, FIT >= 10, HLT >= 10",
            Self::LawEnforcementSwatDetective => "DIS >= 10, SIG >= 10, INS >= 10",
            Self::CorporateSecOpsExecutiveProtection => {
                "Military/Police Vet or Academy (FOR >= 10, DIS >= 11, SIG >= 11)"
            }
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
            Self::UniversityHumanitiesLawEthics => {
                let reading = rank("reading_comprehension");
                let writing = rank("writing_composition");
                let ok = !academic_tree_frozen
                    && reading >= 2
                    && writing >= 2
                    && chars.mem >= 10
                    && chars.ins >= 10;
                (
                    ok,
                    format!(
                        "Academic Unfrozen: {}, Reading N={reading} (req >=2), Writing N={writing} (req >=2), MEM {} (req >=10), INS {} (req >=10)",
                        !academic_tree_frozen, chars.mem, chars.ins
                    ),
                )
            }
            Self::CorporateManagementFinancePolicy => {
                let alg = rank("elementary_algebra_geometry");
                let ok = !academic_tree_frozen && alg >= 2 && chars.ana >= 11 && chars.elo >= 10;
                (
                    ok,
                    format!(
                        "Academic Unfrozen: {}, Algebra N={alg} (req >=2), ANA {} (req >=11), ELO {} (req >=10)",
                        !academic_tree_frozen, chars.ana, chars.elo
                    ),
                )
            }
            Self::JournalismMediaPerformingArts => {
                let writing = rank("writing_composition");
                let ok = writing >= 2 && (chars.cre >= 10 || chars.elo >= 10);
                (
                    ok,
                    format!(
                        "Writing N={writing} (req >=2), CRE {} / ELO {} (req >=10)",
                        chars.cre, chars.elo
                    ),
                )
            }
            Self::GrassrootsActivismHumanitarianNgo => {
                let emp = rank("empathy_social_perception");
                let pers = rank("persuasion_fast_talk");
                let ok = (emp >= 1 || pers >= 1) && (chars.ins >= 10 || chars.cps >= 12);
                (
                    ok,
                    format!(
                        "Empathy N={emp} / Persuasion N={pers} (req >=1), INS {} (req >=10) or CPS {} (req >=12)",
                        chars.ins, chars.cps
                    ),
                )
            }
            Self::CybersecurityAiRedTeaming => {
                let prog = rank("programming_scripting");
                let lit = rank("digital_ai_literacy");
                let ok = (prog >= 1 || lit >= 1) && chars.ana >= 11 && chars.cre >= 10;
                (
                    ok,
                    format!(
                        "Programming N={prog} / Digital Lit N={lit} (req >=1), ANA {} (req >=11), CRE {} (req >=10)",
                        chars.ana, chars.cre
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
            Self::CorporateSecOpsExecutiveProtection => {
                let ok = chars.for_stat >= 10 && chars.dis >= 11 && chars.sig >= 11;
                (
                    ok,
                    format!(
                        "FOR {} (req >=10), DIS {} (req >=11), SIG {} (req >=11)",
                        chars.for_stat, chars.dis, chars.sig
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
            Self::UniversityHumanitiesLawEthics => &[
                "reading_comprehension",
                "writing_composition",
                "persuasion_fast_talk",
                "history_political_science",
                "rhetoric_oratory_debate",
                "philosophy_formal_logic_ethics",
                "research_archival_science",
                "foreign_language_specific",
                "law_jurisprudence",
                "statistics_probability",
            ],
            Self::CorporateManagementFinancePolicy => &[
                "elementary_algebra_geometry",
                "digital_ai_literacy",
                "statistics_probability",
                "economics_finance_accounting",
                "persuasion_fast_talk",
                "negotiation_diplomacy",
                "leadership_command",
                "rhetoric_oratory_debate",
                "law_jurisprudence",
            ],
            Self::JournalismMediaPerformingArts => &[
                "writing_composition",
                "persuasion_fast_talk",
                "charm_rapport",
                "deception_bluff",
                "empathy_social_perception",
                "sketching_doodling",
                "digital_ai_literacy",
                "photography_cinematography_digital_media",
                "singing_vocal_rhythm",
                "musical_instrument_specific",
                "acting_stagecraft_mimicry",
                "research_archival_science",
                "rhetoric_oratory_debate",
            ],
            Self::GrassrootsActivismHumanitarianNgo => &[
                "empathy_social_perception",
                "persuasion_fast_talk",
                "first_aid_bls_cpr",
                "deception_bluff",
                "streetwise_underworld_etiquette",
                "negotiation_diplomacy",
                "foreign_language_specific",
                "psychology_behavioral_analysis_counseling",
                "rhetoric_oratory_debate",
                "philosophy_formal_logic_ethics",
            ],
            Self::CybersecurityAiRedTeaming => &[
                "digital_ai_literacy",
                "elementary_algebra_geometry",
                "statistics_probability",
                "programming_scripting",
                "algorithms_systems_architecture",
                "cybersecurity_network_intrusion",
                "cryptography_quantum_computing",
                "machine_learning_ai_engineering",
                "alertness_situational_awareness",
                "stealth_sneaking_hiding",
                "surveillance_reconnaissance_countersurveillance",
                "manual_dexterity_tinkering",
                "lockpicking_physical_intrusion",
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
            Self::CorporateSecOpsExecutiveProtection => &[
                "manual_dexterity_tinkering",
                "alertness_situational_awareness",
                "stealth_sneaking_hiding",
                "brawling_unarmed_striking",
                "balance_footwork",
                "first_aid_bls_cpr",
                "pistols_handguns",
                "long_guns_rifles_carbines_shotguns",
                "tactical_carbine_cqb",
                "grappling_wrestling_submissions",
                "surveillance_reconnaissance_countersurveillance",
                "driving_wheeled_tracked",
                "high_speed_pursuit_evasive_driving",
                "paramedic_combat_trauma_care",
                "small_unit_tactics_cqb_doctrine",
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
