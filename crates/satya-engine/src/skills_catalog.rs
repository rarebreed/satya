//! Canonical 129-Node Skill Catalog across the 8 Hierarchical Skill Trees
//! (docs/ttrpg/03-character/skills_and_trees.md §3.1–§3.9).
//!
//! Separated from [crate::skills_trees] so that the static DAG catalog and the
//! algorithmic DAG validator / 5-factor SP calculator each fit comfortably within
//! a single agent context window.

use std::collections::BTreeMap;

use crate::characteristics::{AptitudeCode, CharacteristicCode};
use crate::skills_trees::{
    PrerequisiteMode, SkillAccessTier, SkillCategory, SkillNodeSpec, SkillTreeId, SkillType,
};
use crate::task_resolution::ToolDependency;

#[allow(unused_imports)]
use AptitudeCode::*;
#[allow(unused_imports)]
use CharacteristicCode::*;
use SkillTreeId::{
    Tree1MathCompPhysicalSciences as T1, Tree2MechanicalCraftPilotage as T2,
    Tree3BiologyChemistryMedicine as T3, Tree4LanguageHumanitiesSocial as T4,
    Tree5AthleticsAcrobaticsMelee as T5, Tree6MarksmanshipGunneryTactical as T6,
    Tree7PerceptionStealthSurvival as T7, Tree8VisualMusicalCulinaryArts as T8,
};

macro_rules! sn {
    (
        $tree:ident, $id:literal, $name:literal, ($tier:expr, $diff:expr, $stype:ident),
        [$($cat:ident),*], ($pchar:ident, $schar:expr), [$(($mc:ident, $mv:expr)),*],
        $pmode:ident [$($prereq:tt)*], [$($acc:ident),*],
        $tool:ident, $def:expr, $apt:ident
    ) => {
        SkillNodeSpec {
            id: $id,
            name: $name,
            tree: $tree,
            tier: $tier,
            difficulty: $diff,
            skill_type: SkillType::$stype,
            categories: &[$(SkillCategory::$cat),*],
            primary_char: CharacteristicCode::$pchar,
            secondary_char: $schar,
            min_chars: &[$( (CharacteristicCode::$mc, $mv) ),*],
            parent_mode: PrerequisiteMode::$pmode,
            parent_prereqs: &[$($prereq)*],
            allowed_access: &[$(SkillAccessTier::$acc),*],
            tool_dependency: ToolDependency::$tool,
            defaultable: $def,
            aptitude_override: Some(AptitudeCode::$apt),
        }
    };
}

/// Complete static catalog of all canonical skill nodes across the 8 Hierarchical Skill Trees
/// (docs/ttrpg/03-character/skills_and_trees.md §3.2–§3.9).
pub const CANONICAL_SKILL_TREES: &[SkillNodeSpec] = &[
    // === Tree1MathCompPhysicalSciences ===
    sn!(T1, "counting_basic_numeracy", "Counting & Basic Numeracy", (0, 1, Tal), [Knowledge], (MEM, Some(INS)), [], AllOf [], [Universal], None, true, AptQnt),
    sn!(T1, "arithmetic", "Arithmetic", (1, 1, Kno), [Knowledge, Science], (ANA, Some(MEM)), [(ANA, 6)], AllOf [("counting_basic_numeracy", 1)], [Universal], None, true, AptQnt),
    sn!(T1, "digital_ai_literacy", "Digital & AI Literacy", (1, 1, Kno), [Knowledge, Cyber], (MEM, Some(ANA)), [], AllOf [("counting_basic_numeracy", 1)], [Universal], Situational, true, AptCmp),
    sn!(T1, "general_physical_science", "General Physical Science", (2, 2, Kno), [Science, Knowledge], (ANA, Some(MEM)), [(ANA, 7)], AllOf [("arithmetic", 1)], [Academic, Universal], None, true, AptQnt),
    sn!(T1, "elementary_algebra_geometry", "Elementary Algebra & Geometry", (2, 2, Kno), [Science, Knowledge], (ANA, None), [(ANA, 8)], AllOf [("arithmetic", 2)], [Academic], None, true, AptQnt),
    sn!(T1, "trigonometry", "Trigonometry", (3, 3, Kno), [Science, Technical], (ANA, None), [(ANA, 9)], AllOf [("elementary_algebra_geometry", 2)], [Academic], None, true, AptQnt),
    sn!(T1, "discrete_math_graph_theory", "Discrete Mathematics & Graph Theory", (3, 3, Kno), [Science, Cyber], (ANA, Some(INS)), [(ANA, 10)], AllOf [("elementary_algebra_geometry", 2)], [Academic], None, false, AptQnt),
    sn!(T1, "calculus_linear_algebra", "Calculus & Linear Algebra", (4, 4, Kno), [Science], (ANA, Some(CRE)), [(ANA, 11)], AllOf [("trigonometry", 2)], [Academic], None, false, AptQnt),
    sn!(T1, "differential_equations_vector_calculus", "Differential Equations & Vector Calculus", (5, 5, Kno), [Science], (ANA, Some(CRE)), [(ANA, 13)], AllOf [("calculus_linear_algebra", 3)], [Academic], None, false, AptQnt),
    sn!(T1, "statistics_probability", "Statistics & Probability", (3, 3, Kno), [Science, Knowledge], (ANA, Some(INS)), [(ANA, 10)], AllOf [("elementary_algebra_geometry", 2)], [Academic], None, false, AptQnt),
    sn!(T1, "programming_scripting", "Programming & Scripting", (2, 3, Kno), [Cyber, Technical], (ANA, Some(CRE)), [(ANA, 9)], AllOf [("elementary_algebra_geometry", 1), ("digital_ai_literacy", 2)], [Academic, VocationalLicensed], Mandatory, false, AptCmp),
    sn!(T1, "algorithms_systems_architecture", "Algorithms & Systems Architecture", (3, 4, Kno), [Cyber, Science], (ANA, Some(CRE)), [(ANA, 11)], AllOf [("programming_scripting", 2), ("elementary_algebra_geometry", 2)], [Academic, VocationalLicensed], Mandatory, false, AptCmp),
    sn!(T1, "cybersecurity_network_intrusion", "Cybersecurity & Network Intrusion", (3, 4, Kno), [Cyber, Technical], (ANA, Some(INS)), [(ANA, 11), (FOC, 10)], AllOf [("programming_scripting", 2)], [Academic, Restricted, Illicit], Mandatory, false, AptCmp),
    sn!(T1, "cryptography_quantum_computing", "Cryptography & Quantum Computing", (5, 5, Kno), [Cyber, Science], (ANA, Some(CRE)), [(ANA, 14)], AllOf [("algorithms_systems_architecture", 3), ("calculus_linear_algebra", 3)], [Academic, Restricted], Mandatory, false, AptCmp),
    sn!(T1, "machine_learning_ai_engineering", "Machine Learning & AI Engineering", (5, 5, Kno), [Cyber, Science], (ANA, Some(CRE)), [(ANA, 13)], AllOf [("algorithms_systems_architecture", 2), ("statistics_probability", 2), ("calculus_linear_algebra", 2)], [Academic], Mandatory, false, AptCmp),
    sn!(T1, "classical_mechanics_physics", "Classical Mechanics (Physics)", (4, 3, Kno), [Science], (ANA, None), [(ANA, 10)], AllOf [("trigonometry", 2)], [Academic], None, false, AptQnt),
    sn!(T1, "electromagnetism_optics", "Electromagnetism & Optics", (5, 4, Kno), [Science, Technical], (ANA, None), [(ANA, 12)], AllOf [("classical_mechanics_physics", 2), ("calculus_linear_algebra", 2)], [Academic], None, false, AptQnt),
    sn!(T1, "thermodynamics_fluid_dynamics", "Thermodynamics & Fluid Dynamics", (5, 4, Kno), [Science, Technical], (ANA, None), [(ANA, 12)], AllOf [("classical_mechanics_physics", 2), ("calculus_linear_algebra", 2)], [Academic], None, false, AptQnt),
    sn!(T1, "quantum_relativity_nuclear_physics", "Quantum, Relativity & Nuclear Physics", (6, 5, Kno), [Science], (ANA, Some(CRE)), [(ANA, 14)], AllOf [("electromagnetism_optics", 3), ("differential_equations_vector_calculus", 3)], [Academic, Restricted], None, false, AptQnt),
    sn!(T1, "astronomy_astrophysics_orbital_mechanics", "Astronomy, Astrophysics & Orbital Mechanics", (5, 5, Kno), [Science, Technical], (ANA, Some(SIG)), [(ANA, 12)], AllOf [("classical_mechanics_physics", 2), ("calculus_linear_algebra", 2)], [Academic], Situational, false, AptQnt),
    // === Tree2MechanicalCraftPilotage ===
    sn!(T2, "manual_dexterity_tinkering", "Manual Dexterity & Tinkering", (0, 1, Tal), [Technical], (DEX, Some(TAC)), [], AllOf [], [Universal], None, true, AptMec),
    sn!(T2, "carpentry_woodworking", "Carpentry & Woodworking", (1, 2, PhyKno), [Technical, Artistic], (DEX, Some(FOR)), [(DEX, 7)], AllOf [("manual_dexterity_tinkering", 1)], [Universal, VocationalLicensed], Mandatory, true, AptMec),
    sn!(T2, "masonry_construction", "Masonry & Construction", (1, 2, PhyKno), [Technical, Athletic], (FOR, Some(DEX)), [(FOR, 8)], AllOf [("manual_dexterity_tinkering", 1)], [Universal, VocationalLicensed], Mandatory, true, AptMec),
    sn!(T2, "metalworking_machining_blacksmithing", "Metalworking, Machining & Blacksmithing", (2, 3, PhyKno), [Technical], (DEX, Some(FOR)), [(DEX, 8), (FOR, 8)], AllOf [("manual_dexterity_tinkering", 2)], [VocationalLicensed], Mandatory, false, AptMec),
    sn!(T2, "lockpicking_physical_intrusion", "Lockpicking & Physical Intrusion", (2, 3, PhyKno), [Technical, Fieldcraft], (DEX, Some(TAC)), [(DEX, 9), (TAC, 9)], AllOf [("manual_dexterity_tinkering", 2)], [VocationalLicensed, Restricted, Illicit], Mandatory, false, AptFin),
    sn!(T2, "vehicle_hotwiring_transponder_spoofing", "Vehicle Hotwiring & Transponder Spoofing", (2, 2, PhyKno), [Technical, Fieldcraft, Cyber], (DEX, Some(ANA)), [(DEX, 8), (ANA, 8)], AllOf [("manual_dexterity_tinkering", 2)], [Illicit, Restricted], Mandatory, false, AptMec),
    sn!(T2, "improvised_munitions_street_mods", "Improvised Munitions & Street Mods", (2, 3, PhyKno), [Technical, Combat], (DEX, Some(CRE)), [(DEX, 8), (CRE, 8)], AllOf [("manual_dexterity_tinkering", 2)], [Illicit, Restricted], Mandatory, false, AptMec),
    sn!(T2, "automotive_small_engine_mechanics", "Automotive & Small Engine Mechanics", (2, 2, PhyKno), [Technical], (DEX, Some(ANA)), [(DEX, 8), (ANA, 7)], AllOf [("manual_dexterity_tinkering", 2), ("arithmetic", 1)], [VocationalLicensed, Universal], Mandatory, true, AptMec),
    sn!(T2, "electrical_power_systems", "Electrical & Power Systems", (2, 3, PhyKno), [Technical], (ANA, Some(DEX)), [(ANA, 8), (DEX, 8)], AllOf [("manual_dexterity_tinkering", 2), ("elementary_algebra_geometry", 1)], [VocationalLicensed], Mandatory, false, AptMec),
    sn!(T2, "micro_electronics_avionics_cyberware_tech", "Micro-Electronics, Avionics & Cyberware Tech", (3, 4, PhyKno), [Technical, Cyber], (DEX, Some(ANA)), [(DEX, 11), (ANA, 10)], AllOf [("electrical_power_systems", 2)], [VocationalLicensed, Academic], Mandatory, false, AptMec),
    sn!(T2, "armourer_gunsmithing", "Armourer & Gunsmithing", (3, 3, PhyKno), [Technical, Combat], (DEX, Some(ANA)), [(DEX, 9), (ANA, 8)], AllOf [("metalworking_machining_blacksmithing", 2)], [VocationalLicensed, Restricted, Illicit], Mandatory, false, AptMec),
    sn!(T2, "mechanical_structural_engineering", "Mechanical & Structural Engineering", (4, 4, Kno), [Science, Technical], (ANA, Some(CRE)), [(ANA, 11)], AllOf [("classical_mechanics_physics", 2), ("calculus_linear_algebra", 2)], [Academic], Situational, false, AptMec),
    sn!(T2, "aerospace_naval_robotics_engineering", "Aerospace, Naval & Robotics Engineering", (5, 5, Kno), [Science, Technical], (ANA, Some(CRE)), [(ANA, 13)], AllOf [("mechanical_structural_engineering", 3), ("thermodynamics_fluid_dynamics", 2)], [Academic, Restricted], Mandatory, false, AptMec),
    sn!(T2, "driving_wheeled_tracked", "Driving (Wheeled & Tracked Ground Vehicles)", (1, 1, Phy), [Technical, Fieldcraft], (DEX, Some(SIG)), [(DEX, 7), (SIG, 6)], AllOf [("manual_dexterity_tinkering", 1)], [Universal], Mandatory, true, AptMec),
    sn!(T2, "high_speed_pursuit_evasive_driving", "High-Speed Pursuit & Evasive Driving", (2, 3, Phy), [Technical, Fieldcraft], (SPD, Some(DEX)), [(SPD, 9), (DEX, 9)], AllOf [("driving_wheeled_tracked", 2)], [Restricted, Illicit, VocationalLicensed], Mandatory, true, AptMec),
    sn!(T2, "small_boat_seamanship", "Small Boat & Seamanship", (1, 1, PhyKno), [Technical, Fieldcraft], (KIN, Some(INS)), [(KIN, 7)], AllOf [("manual_dexterity_tinkering", 1)], [Universal, VocationalLicensed], Mandatory, true, AptMec),
    sn!(T2, "piloting_fixed_wing_rotary", "Piloting (Fixed-Wing Aircraft)", (3, 4, PhyKno), [Technical], (DEX, Some(FOC)), [(DEX, 10), (SIG, 10), (FOC, 10)], AllOf [("driving_wheeled_tracked", 2), ("trigonometry", 1)], [VocationalLicensed, Restricted], Mandatory, false, AptMec),
    sn!(T2, "piloting_rotary_wing_helicopter", "Piloting (Rotary-Wing / Helicopter & VTOL)", (4, 5, PhyKno), [Technical], (KIN, Some(DEX)), [(KIN, 12), (DEX, 11), (FOC, 11)], AllOf [("driving_wheeled_tracked", 2), ("trigonometry", 1)], [VocationalLicensed, Restricted], Mandatory, false, AptKin),
    sn!(T2, "spacecraft_orbital_helm", "Spacecraft & Orbital Helm", (4, 5, PhyKno), [Technical, Science], (DEX, Some(ANA)), [(DEX, 11), (ANA, 11)], AllOf [("piloting_fixed_wing_rotary", 2), ("astronomy_astrophysics_orbital_mechanics", 2)], [Restricted], Mandatory, false, AptMec),
    // === Tree3BiologyChemistryMedicine ===
    sn!(T3, "hygiene_basic_self_care", "Hygiene & Basic Self-Care", (0, 1, Tal), [Medical], (INS, Some(MEM)), [], AllOf [], [Universal], None, true, AptBio),
    sn!(T3, "first_aid_bls_cpr", "First Aid & Basic Life Support (BLS / CPR)", (1, 1, PhyKno), [Medical, Fieldcraft], (DEX, Some(FOC)), [(FOC, 7)], AllOf [("hygiene_basic_self_care", 1)], [Universal], Situational, true, AptBio),
    sn!(T3, "general_biology_natural_history", "General Biology & Natural History", (1, 2, Kno), [Science, Knowledge], (MEM, Some(ANA)), [(MEM, 8)], AllOf [("hygiene_basic_self_care", 1)], [Academic, Universal], None, true, AptBio),
    sn!(T3, "agriculture_agronomy_pastoralism", "Agriculture, Agronomy & Crop Husbandry", (1, 2, PhyKno), [Science, Technical, Fieldcraft], (INS, Some(FOR)), [(INS, 7), (FIT, 7)], AnyOf [("foraging_campcraft", 1), ("hygiene_basic_self_care", 1)], [Universal, VocationalLicensed, Academic], Situational, true, AptBio),
    sn!(T3, "herbalism_ethnobotany_natural_remedies", "Herbalism, Ethnobotany & Field Remedies", (2, 2, Kno), [Medical, Fieldcraft, Science], (MEM, Some(OLF)), [(MEM, 8), (OLF, 8)], AllOf [("foraging_campcraft", 2), ("first_aid_bls_cpr", 1)], [Universal, VocationalLicensed], Situational, true, AptBio),
    sn!(T3, "veterinary_medicine_livestock_care", "Veterinary Medicine & Livestock Care", (2, 3, PhyKno), [Medical, Fieldcraft, Science], (INS, Some(DEX)), [(INS, 8), (DEX, 8)], AllOf [("animal_handling_husbandry_riding", 1), ("first_aid_bls_cpr", 1)], [Universal, VocationalLicensed, Academic], Situational, true, AptBio),
    sn!(T3, "general_chemistry", "General Chemistry", (2, 3, Kno), [Science], (ANA, Some(MEM)), [(ANA, 9)], AllOf [("elementary_algebra_geometry", 2)], [Academic], Mandatory, false, AptBio),
    sn!(T3, "clandestine_narcotics_synthesis", "Clandestine Narcotics Synthesis", (2, 3, PhyKno), [Science, Technical], (MEM, Some(DEX)), [(MEM, 8), (DEX, 8)], AnyOf [("general_chemistry", 1), ("basic_cooking", 2)], [Illicit], Mandatory, false, AptBio),
    sn!(T3, "organic_analytical_chemistry", "Organic & Analytical Chemistry", (3, 4, Kno), [Science], (ANA, Some(MEM)), [(ANA, 11)], AllOf [("general_chemistry", 2)], [Academic], Mandatory, false, AptBio),
    sn!(T3, "anatomy_physiology", "Anatomy & Physiology", (2, 3, Kno), [Medical, Science], (MEM, Some(ANA)), [(MEM, 10)], AllOf [("general_biology_natural_history", 2)], [Academic, VocationalLicensed], None, false, AptBio),
    sn!(T3, "paramedic_combat_trauma_care", "Paramedic & Combat Trauma Care", (3, 3, PhyKno), [Medical, Fieldcraft], (DEX, Some(FOC)), [(DEX, 9), (FOC, 10)], AllOf [("first_aid_bls_cpr", 2), ("anatomy_physiology", 1)], [VocationalLicensed, Restricted], Mandatory, false, AptBio),
    sn!(T3, "nursing_patient_care", "Nursing & Patient Care", (3, 3, PhyKno), [Medical], (INS, Some(MEM)), [(MEM, 9), (DIS, 9)], AllOf [("first_aid_bls_cpr", 2), ("anatomy_physiology", 2)], [VocationalLicensed, Academic], Mandatory, false, AptBio),
    sn!(T3, "pharmacology_toxicology", "Pharmacology & Toxicology", (4, 4, Kno), [Medical, Science], (ANA, Some(MEM)), [(ANA, 11), (MEM, 11)], AllOf [("organic_analytical_chemistry", 2), ("anatomy_physiology", 2)], [Academic, Illicit], Mandatory, false, AptBio),
    sn!(T3, "clinical_diagnosis_internal_medicine", "Clinical Diagnosis & Internal Medicine", (4, 4, Kno), [Medical, Science], (ANA, Some(INS)), [(ANA, 12), (MEM, 12)], AllOf [("anatomy_physiology", 3), ("pharmacology_toxicology", 2)], [Academic], Mandatory, false, AptBio),
    sn!(T3, "trauma_orthopedic_general_surgery", "Trauma, Orthopedic & General Surgery", (5, 5, PhyKno), [Medical], (DEX, Some(FOC)), [(DEX, 12), (FOC, 12), (ANA, 11)], AllOf [("clinical_diagnosis_internal_medicine", 2), ("paramedic_combat_trauma_care", 2)], [Academic], Mandatory, false, AptBio),
    sn!(T3, "genetics_biotech_cyber_implantation", "Genetics, Biotech & Cyber-Implantation", (6, 5, PhyKno), [Medical, Science, Cyber], (ANA, Some(DEX)), [(ANA, 14), (DEX, 13)], AllOf [("trauma_orthopedic_general_surgery", 3), ("organic_analytical_chemistry", 3)], [Academic, Restricted], Mandatory, false, AptBio),
    // === Tree4LanguageHumanitiesSocial ===
    sn!(T4, "native_language_speech", "Native Language Speech", (0, 1, Tal), [Social, Knowledge], (ELO, Some(MEM)), [], AllOf [], [Universal], None, true, AptLng),
    sn!(T4, "empathy_social_perception", "Empathy & Social Perception", (0, 1, Tal), [Social], (INS, Some(ITU)), [], AllOf [], [Universal], None, true, AptSoc),
    sn!(T4, "charm_rapport", "Charm & Rapport", (0, 1, Tal), [Social], (PRE, Some(ELO)), [], AllOf [], [Universal], None, true, AptSoc),
    sn!(T4, "persuasion_fast_talk", "Persuasion & Fast-Talk", (0, 1, Tal), [Social], (ELO, Some(ANA)), [], AllOf [], [Universal], None, true, AptSoc),
    sn!(T4, "deception_bluff", "Deception & Bluff", (0, 1, Tal), [Social], (ELO, Some(CRE)), [], AllOf [], [Universal], None, true, AptSoc),
    sn!(T4, "intimidation_coercion", "Intimidation & Coercion", (0, 1, Tal), [Social, Combat], (PRE, Some(DIS)), [], AllOf [], [Universal], None, true, AptSoc),
    sn!(T4, "reading_comprehension", "Reading (Comprehension)", (1, 1, Kno), [Knowledge], (MEM, Some(ANA)), [(MEM, 6)], AllOf [("native_language_speech", 2)], [Universal], None, true, AptLng),
    sn!(T4, "writing_composition", "Writing (Composition)", (1, 1, Kno), [Knowledge, Artistic], (ELO, Some(MEM)), [(MEM, 6)], AllOf [("reading_comprehension", 1)], [Universal], None, true, AptLng),
    sn!(T4, "foreign_language_specific", "Foreign Language (Specific)", (1, 2, Kno), [Knowledge, Social], (MEM, Some(ELO)), [(MEM, 7)], AllOf [("native_language_speech", 2)], [Universal, Academic], None, false, AptLng),
    sn!(T4, "seduction_honey_trap", "Seduction & Honey-Trap", (2, 2, PhyKno), [Social, Fieldcraft], (COM, Some(PRE)), [(COM, 9), (PRE, 9)], AllOf [("charm_rapport", 2), ("deception_bluff", 1)], [Universal, Restricted, Illicit], None, true, AptSoc),
    sn!(T4, "rhetoric_oratory_debate", "Rhetoric, Oratory & Debate", (2, 2, Kno), [Social, Knowledge], (ELO, Some(PRE)), [(ELO, 9)], AllOf [("writing_composition", 2), ("persuasion_fast_talk", 2)], [Academic, Universal], None, true, AptSoc),
    sn!(T4, "research_archival_science", "Research & Archival Science", (2, 2, Kno), [Knowledge], (ANA, Some(MEM)), [(ANA, 8)], AllOf [("reading_comprehension", 2), ("writing_composition", 2)], [Academic], Situational, true, AptLng),
    sn!(T4, "history_political_science", "History & Political Science", (2, 2, Kno), [Knowledge], (MEM, Some(ANA)), [(MEM, 8)], AllOf [("reading_comprehension", 2)], [Academic], None, true, AptLng),
    sn!(T4, "philosophy_formal_logic_ethics", "Philosophy, Formal Logic & Ethics", (3, 3, Kno), [Knowledge], (ANA, Some(INS)), [(ANA, 10)], AllOf [("writing_composition", 2), ("rhetoric_oratory_debate", 1)], [Academic], None, true, AptLng),
    sn!(T4, "linguistics_cryptanalysis_philology", "Linguistics, Cryptanalysis & Philology", (3, 4, Kno), [Knowledge, Science], (ANA, Some(MEM)), [(ANA, 11)], AllOf [("reading_comprehension", 3), ("foreign_language_specific", 2)], [Academic], None, false, AptLng),
    sn!(T4, "psychology_behavioral_analysis_counseling", "Psychology, Behavioral Analysis & Counseling", (3, 3, Kno), [Science, Medical, Social], (INS, Some(ANA)), [(INS, 10), (ANA, 9)], AllOf [("empathy_social_perception", 2), ("reading_comprehension", 2)], [Academic], None, false, AptSoc),
    sn!(T4, "economics_finance_accounting", "Economics, Finance & Accounting", (3, 3, Kno), [Knowledge, Science], (ANA, Some(MEM)), [(ANA, 10)], AllOf [("statistics_probability", 1), ("reading_comprehension", 2)], [Academic, VocationalLicensed], None, false, AptQnt),
    sn!(T4, "law_jurisprudence", "Law & Jurisprudence", (4, 4, Kno), [Knowledge, Social], (MEM, Some(ELO)), [(MEM, 11), (ANA, 10)], AllOf [("rhetoric_oratory_debate", 2), ("history_political_science", 2)], [Academic], None, false, AptSoc),
    sn!(T4, "negotiation_diplomacy", "Negotiation & Diplomacy", (3, 3, Kno), [Social], (ELO, Some(INS)), [(ELO, 10), (INS, 10)], AllOf [("persuasion_fast_talk", 2), ("empathy_social_perception", 2)], [Universal, Academic, VocationalLicensed], None, true, AptSoc),
    sn!(T4, "leadership_command", "Leadership & Command", (2, 2, Kno), [Social, Combat], (PRE, Some(DIS)), [(PRE, 10), (DIS, 9)], AllOf [("persuasion_fast_talk", 2)], [Universal, Restricted], None, true, AptSoc),
    sn!(T4, "interrogation_counter_interrogation", "Interrogation & Counter-Interrogation", (3, 3, Kno), [Social, Fieldcraft], (INS, Some(PRE)), [(INS, 10), (DIS, 10)], AllOf [("intimidation_coercion", 2), ("deception_bluff", 2)], [Restricted, Illicit], None, true, AptSoc),
    sn!(T4, "streetwise_underworld_etiquette", "Streetwise & Underworld Etiquette", (2, 2, Kno), [Social, Fieldcraft], (INS, Some(ITU)), [(INS, 8)], AllOf [("empathy_social_perception", 2), ("deception_bluff", 1)], [Illicit, Universal], None, true, AptSoc),
    sn!(T4, "black_market_smuggling_fencing", "Black-Market Smuggling & Fencing", (2, 2, Kno), [Social, Fieldcraft], (INS, Some(ELO)), [(INS, 8), (ELO, 8)], AllOf [("streetwise_underworld_etiquette", 1), ("deception_bluff", 1)], [Illicit], None, true, AptSoc),
    // === Tree5AthleticsAcrobaticsMelee ===
    sn!(T5, "running_sprinting", "Running & Sprinting", (0, 1, Tal), [Athletic], (SPD, Some(FIT)), [], AllOf [], [Universal], None, true, AptKin),
    sn!(T5, "jumping_leaping", "Jumping & Leaping", (0, 1, Tal), [Athletic], (FOR, Some(SPD)), [], AllOf [], [Universal], None, true, AptKin),
    sn!(T5, "balance_footwork", "Balance & Footwork", (0, 1, Tal), [Athletic], (KIN, Some(SPD)), [], AllOf [], [Universal], None, true, AptKin),
    sn!(T5, "strength_lifting", "Strength & Lifting", (0, 1, Tal), [Athletic], (FOR, Some(FIT)), [], AllOf [], [Universal], None, true, AptKin),
    sn!(T5, "brawling_unarmed_striking", "Brawling (Unarmed Striking)", (0, 1, Tal), [Combat, Athletic], (SPD, Some(FOR)), [], AllOf [], [Universal], None, true, AptKin),
    sn!(T5, "climbing_scaling", "Climbing & Scaling", (1, 1, Phy), [Athletic, Fieldcraft], (FOR, Some(KIN)), [(FOR, 7), (KIN, 7)], AllOf [("balance_footwork", 1), ("strength_lifting", 1)], [Universal], Situational, true, AptKin),
    sn!(T5, "swimming_diving", "Swimming & Diving", (1, 1, Phy), [Athletic, Fieldcraft], (FIT, Some(KIN)), [(FIT, 6)], AllOf [("balance_footwork", 1)], [Universal], None, true, AptKin),
    sn!(T5, "acrobatics_tumbling_parkour", "Acrobatics, Tumbling & Parkour", (2, 2, Phy), [Athletic], (KIN, Some(SPD)), [(KIN, 10), (SPD, 9)], AllOf [("balance_footwork", 2), ("jumping_leaping", 2)], [Universal, VocationalLicensed], None, true, AptKin),
    sn!(T5, "zero_g_eva_operations", "Zero-G / EVA Operations", (3, 4, PhyKno), [Athletic, Technical], (KIN, Some(FOC)), [(KIN, 10)], AllOf [("acrobatics_tumbling_parkour", 2)], [VocationalLicensed, Restricted], Mandatory, false, AptKin),
    sn!(T5, "martial_arts_striking", "Martial Arts (Striking — Boxing, Muay Thai, Karate)", (2, 3, Phy), [Combat, Athletic], (SPD, Some(KIN)), [(SPD, 9), (KIN, 9)], AllOf [("brawling_unarmed_striking", 2), ("balance_footwork", 2)], [VocationalLicensed, Restricted, Illicit], None, true, AptKin),
    sn!(T5, "grappling_wrestling_submissions", "Grappling, Wrestling & Submissions (Judo, BJJ, Sambo)", (2, 3, Phy), [Combat, Athletic], (FOR, Some(KIN)), [(FOR, 9), (KIN, 9)], AllOf [("brawling_unarmed_striking", 2), ("balance_footwork", 2)], [VocationalLicensed, Restricted, Illicit], None, true, AptKin),
    sn!(T5, "short_blades_knife_fighting", "Short Blades & Knife Fighting", (1, 2, Phy), [Combat], (SPD, Some(DEX)), [(SPD, 8), (DEX, 8)], AllOf [("brawling_unarmed_striking", 1), ("balance_footwork", 1)], [Universal, Restricted, Illicit], Mandatory, true, AptKin),
    sn!(T5, "swords_fencing_long_blades", "Swords, Fencing & Long Blades", (2, 3, Phy), [Combat, Athletic], (SPD, Some(KIN)), [(SPD, 9), (KIN, 9)], AllOf [("short_blades_knife_fighting", 2), ("balance_footwork", 2)], [VocationalLicensed, Restricted], Mandatory, true, AptKin),
    sn!(T5, "blunt_impact_weapons_polearms", "Blunt / Impact Weapons & Polearms", (2, 2, Phy), [Combat], (FOR, Some(SPD)), [(FOR, 9), (KIN, 8)], AllOf [("brawling_unarmed_striking", 1), ("balance_footwork", 2)], [Universal, Restricted], Mandatory, true, AptKin),
    // === Tree6MarksmanshipGunneryTactical ===
    sn!(T6, "throwing_lobbing", "Throwing & Lobbing", (0, 1, Tal), [Athletic, Combat], (DEX, Some(KIN)), [], AllOf [], [Universal], None, true, AptFin),
    sn!(T6, "archery_bows_crossbows", "Archery (Bows & Crossbows)", (1, 2, Phy), [Combat, Athletic, Fieldcraft], (DEX, Some(SIG)), [(FOR, 8), (DEX, 8)], AllOf [("throwing_lobbing", 1)], [Universal], Mandatory, true, AptFin),
    sn!(T6, "pistols_handguns", "Pistols & Handguns", (1, 2, Phy), [Combat], (DEX, Some(SIG)), [(DEX, 8), (SIG, 7)], AllOf [("manual_dexterity_tinkering", 1)], [Universal, VocationalLicensed, Restricted, Illicit], Mandatory, true, AptFin),
    sn!(T6, "concealed_draw_street_shooting", "Concealed Draw & Street Shooting", (2, 2, Phy), [Combat], (SPD, Some(DEX)), [(SPD, 9), (DEX, 8)], AllOf [("pistols_handguns", 2)], [Illicit, Restricted, VocationalLicensed], Mandatory, true, AptFin),
    sn!(T6, "long_guns_rifles_carbines_shotguns", "Long Guns (Rifles, Carbines & Shotguns)", (1, 2, Phy), [Combat, Fieldcraft], (DEX, Some(SIG)), [(DEX, 7), (SIG, 8)], AllOf [("manual_dexterity_tinkering", 1)], [Universal, VocationalLicensed, Restricted, Illicit], Mandatory, true, AptFin),
    sn!(T6, "tactical_carbine_cqb", "Tactical Carbine & CQB Marksmanship", (2, 3, Phy), [Combat], (SPD, Some(DEX)), [(SPD, 9), (DEX, 9)], AllOf [("long_guns_rifles_carbines_shotguns", 2)], [Restricted, VocationalLicensed], Mandatory, true, AptFin),
    sn!(T6, "fully_automatic_burst_weapons", "Fully Automatic & Burst Weapons (SMGs, Assault Rifles, LMGs)", (2, 3, Phy), [Combat], (FOR, Some(DEX)), [(FOR, 10), (DEX, 9)], AnyOf [("pistols_handguns", 2), ("long_guns_rifles_carbines_shotguns", 2)], [Restricted, Illicit], Mandatory, true, AptFin),
    sn!(T6, "precision_marksmanship_sniping", "Precision Marksmanship & Sniping", (3, 4, PhyKno), [Combat, Fieldcraft], (DEX, Some(FOC)), [(SIG, 12), (FOC, 12), (DEX, 10)], AllOf [("long_guns_rifles_carbines_shotguns", 3), ("trigonometry", 1)], [Restricted], Mandatory, false, AptFin),
    sn!(T6, "heavy_weapons_mounted_gunnery_atgms", "Heavy Weapons, Mounted Gunnery & ATGMs", (3, 3, PhyKno), [Combat, Technical], (DEX, Some(SIG)), [(FOR, 10), (SIG, 9)], AllOf [("long_guns_rifles_carbines_shotguns", 2)], [Restricted], Mandatory, false, AptFin),
    sn!(T6, "indirect_fire_mortars_artillery", "Indirect Fire, Mortars & Artillery", (3, 3, Kno), [Combat, Technical], (ANA, Some(SIG)), [(ANA, 10)], AllOf [("trigonometry", 2), ("heavy_weapons_mounted_gunnery_atgms", 1)], [Restricted], Mandatory, false, AptQnt),
    sn!(T6, "demolitions_explosive_ordnance_eod", "Demolitions & Explosive Ordnance (EOD)", (3, 4, PhyKno), [Technical, Combat], (DEX, Some(FOC)), [(DEX, 11), (FOC, 11), (ANA, 9)], AllOf [("general_chemistry", 1), ("electrical_power_systems", 1)], [Restricted, Illicit], Mandatory, false, AptFin),
    sn!(T6, "small_unit_tactics_cqb_doctrine", "Small-Unit Tactics & CQB Doctrine", (2, 3, Kno), [Combat, Knowledge], (INS, Some(ANA)), [(INS, 9), (FOC, 9)], AllOf [("long_guns_rifles_carbines_shotguns", 2), ("leadership_command", 1)], [Restricted], None, true, AptSoc),
    // === Tree7PerceptionStealthSurvival ===
    sn!(T7, "alertness_situational_awareness", "Alertness (Visual & Auditory Situational Awareness)", (0, 1, Tal), [Fieldcraft], (SIG, Some(HEA)), [], AllOf [], [Universal], None, true, AptAud),
    sn!(T7, "stealth_sneaking_hiding", "Stealth (Sneaking & Hiding)", (0, 1, Tal), [Fieldcraft, Athletic], (KIN, Some(ITU)), [], AllOf [], [Universal], None, true, AptKin),
    sn!(T7, "foraging_campcraft", "Foraging & Campcraft", (0, 1, Tal), [Fieldcraft], (INS, Some(OLF)), [], AllOf [], [Universal], None, true, AptBio),
    sn!(T7, "camouflage_concealment", "Camouflage & Concealment", (1, 2, PhyKno), [Fieldcraft], (INS, Some(CRE)), [(INS, 8)], AllOf [("stealth_sneaking_hiding", 2)], [Universal, Restricted], Situational, true, AptKin),
    sn!(T7, "sleight_of_hand_pickpocketing_palming", "Sleight of Hand, Pickpocketing & Palming", (2, 3, Phy), [Fieldcraft], (DEX, Some(TAC)), [(DEX, 10), (TAC, 9)], AllOf [("stealth_sneaking_hiding", 2), ("manual_dexterity_tinkering", 2)], [Illicit, Universal], None, true, AptFin),
    sn!(T7, "disguise_impersonation", "Disguise & Impersonation", (2, 3, PhyKno), [Social, Fieldcraft, Artistic], (CRE, Some(INS)), [(CRE, 9), (INS, 9)], AllOf [("deception_bluff", 2), ("camouflage_concealment", 1)], [Restricted, Illicit, VocationalLicensed], Mandatory, true, AptSoc),
    sn!(T7, "tracking_spoor_analysis", "Tracking & Spoor Analysis", (1, 2, Kno), [Fieldcraft], (SIG, Some(INS)), [(SIG, 9), (INS, 8)], AllOf [("alertness_situational_awareness", 2), ("foraging_campcraft", 1)], [Universal], None, true, AptAud),
    sn!(T7, "hunting_trapping_field_butchery", "Hunting, Trapping & Field Butchery", (1, 2, PhyKno), [Fieldcraft, Combat], (SIG, Some(INS)), [(SIG, 8), (DIS, 7)], AllOf [("foraging_campcraft", 1), ("stealth_sneaking_hiding", 1)], [Universal, VocationalLicensed], Mandatory, true, AptBio),
    sn!(T7, "animal_handling_husbandry_riding", "Animal Handling, Husbandry & Riding", (1, 2, PhyKno), [Fieldcraft, Athletic, Social], (INS, Some(KIN)), [(INS, 7), (KIN, 7)], AnyOf [("empathy_social_perception", 1), ("foraging_campcraft", 1)], [Universal, VocationalLicensed], Situational, true, AptBio),
    sn!(T7, "wilderness_survival_biome_specific", "Wilderness Survival (Biome-Specific)", (2, 2, Kno), [Fieldcraft], (INS, Some(HLT)), [(DIS, 8)], AllOf [("foraging_campcraft", 2), ("general_biology_natural_history", 1)], [Universal], Situational, true, AptBio),
    sn!(T7, "land_navigation_cartography", "Land Navigation & Cartography", (2, 2, Kno), [Fieldcraft, Technical], (ANA, Some(SIG)), [(ANA, 8)], AllOf [("alertness_situational_awareness", 1), ("elementary_algebra_geometry", 1)], [Universal, Academic, Restricted], Situational, true, AptQnt),
    sn!(T7, "surveillance_reconnaissance_countersurveillance", "Surveillance, Reconnaissance & Countersurveillance", (2, 3, PhyKno), [Fieldcraft, Cyber], (SIG, Some(FOC)), [(FOC, 10), (SIG, 9)], AllOf [("alertness_situational_awareness", 2), ("stealth_sneaking_hiding", 2)], [Restricted, Illicit], Situational, true, AptAud),
    sn!(T7, "forensics_crime_scene_investigation", "Forensics & Crime Scene Investigation", (3, 4, Kno), [Science, Fieldcraft, Medical], (ANA, Some(SIG)), [(ANA, 10), (SIG, 10)], AllOf [("alertness_situational_awareness", 2), ("general_chemistry", 1), ("anatomy_physiology", 1)], [Academic, Restricted], Mandatory, false, AptBio),
    // === Tree8VisualMusicalCulinaryArts ===
    sn!(T8, "singing_vocal_rhythm", "Singing & Vocal Rhythm", (0, 1, Tal), [Artistic, Social], (HEA, Some(ELO)), [], AllOf [], [Universal], None, true, AptAud),
    sn!(T8, "sketching_doodling", "Sketching & Doodling", (0, 1, Tal), [Artistic], (DEX, Some(SIG)), [], AllOf [], [Universal], Situational, true, AptFin),
    sn!(T8, "basic_cooking", "Basic Cooking", (0, 1, Tal), [Artistic, Fieldcraft], (OLF, Some(TAS)), [], AllOf [], [Universal], Mandatory, true, AptBio),
    sn!(T8, "drawing_painting_illustration", "Drawing, Painting & Illustration", (1, 2, PhyKno), [Artistic], (DEX, Some(CRE)), [(DEX, 8), (SIG, 8)], AllOf [("sketching_doodling", 2)], [Universal, Academic], Mandatory, true, AptFin),
    sn!(T8, "sculpting_ceramics_carving", "Sculpting, Ceramics & Carving", (1, 2, PhyKno), [Artistic, Technical], (DEX, Some(TAC)), [(DEX, 8), (TAC, 8)], AllOf [("sketching_doodling", 1), ("manual_dexterity_tinkering", 1)], [Universal, VocationalLicensed], Mandatory, true, AptFin),
    sn!(T8, "photography_cinematography_digital_media", "Photography, Cinematography & Digital Media", (2, 2, PhyKno), [Artistic, Technical], (SIG, Some(CRE)), [(SIG, 9), (CRE, 8)], AllOf [("sketching_doodling", 1), ("digital_ai_literacy", 1)], [Universal, Academic], Mandatory, true, AptFin),
    sn!(T8, "forgery_counterfeiting_document_alteration", "Forgery, Counterfeiting & Document Alteration", (3, 4, PhyKno), [Artistic, Technical, Fieldcraft], (DEX, Some(SIG)), [(DEX, 11), (SIG, 11), (FOC, 10)], AllOf [("drawing_painting_illustration", 2), ("writing_composition", 2)], [Illicit, Restricted], Mandatory, false, AptFin),
    sn!(T8, "musical_instrument_specific", "Musical Instrument (Specific)", (1, 2, PhyKno), [Artistic], (DEX, Some(HEA)), [(DEX, 8), (HEA, 8)], AllOf [("singing_vocal_rhythm", 1)], [Universal, Academic], Mandatory, false, AptAud),
    sn!(T8, "music_theory_composition_sound_engineering", "Music Theory, Composition & Sound Engineering", (2, 3, Kno), [Artistic, Knowledge], (CRE, Some(HEA)), [(HEA, 10), (CRE, 10)], AllOf [("musical_instrument_specific", 2)], [Academic], Situational, false, AptAud),
    sn!(T8, "dancing_choreography", "Dancing & Choreography", (1, 1, Phy), [Artistic, Athletic, Social], (KIN, Some(HEA)), [(KIN, 8), (HEA, 7)], AllOf [("balance_footwork", 2), ("singing_vocal_rhythm", 1)], [Universal, Academic], None, true, AptKin),
    sn!(T8, "acting_stagecraft_mimicry", "Acting, Stagecraft & Mimicry", (2, 2, PhyKno), [Artistic, Social], (PRE, Some(ELO)), [(PRE, 9), (CRE, 9)], AllOf [("deception_bluff", 2), ("native_language_speech", 2)], [Universal, Academic], None, true, AptSoc),
    sn!(T8, "culinary_arts_gastronomy_brewing", "Culinary Arts, Gastronomy & Brewing", (2, 2, PhyKno), [Artistic, Science], (TAS, Some(OLF)), [(TAS, 9), (OLF, 9)], AllOf [("basic_cooking", 2)], [VocationalLicensed, Universal], Mandatory, true, AptBio),
];
#[must_use]
pub fn lookup_skill_node(id_or_name: &str) -> Option<&'static SkillNodeSpec> {
    let trimmed = id_or_name.trim();
    CANONICAL_SKILL_TREES.iter().find(|node| {
        node.id.eq_ignore_ascii_case(trimmed) || node.name.eq_ignore_ascii_case(trimmed)
    })
}

#[must_use]
pub fn skills_in_tree(tree: SkillTreeId) -> Vec<&'static SkillNodeSpec> {
    CANONICAL_SKILL_TREES
        .iter()
        .filter(|node| node.tree == tree)
        .collect()
}

/// Initialize the Universal Childhood Talent (`TAL`) baseline
/// (`docs/ttrpg/03-character/skills_and_trees.md` §2.1, §3.1).
///
/// - `native_language_speech` starts at $N = 2$.
/// - All other `TAL` nodes across the 8 trees start at $N = 1$ and never decay below $1$.
#[must_use]
pub fn initialize_universal_talents() -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    for node in CANONICAL_SKILL_TREES {
        if node.skill_type == SkillType::Tal {
            let level = if node.id == "native_language_speech" {
                2
            } else {
                1
            };
            map.insert(node.id.to_string(), level);
        }
    }
    map
}
