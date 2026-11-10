//! Canonical Character Lifepath Walkthroughs (`docs/ttrpg/03-character/lifepaths_and_economy.md` §2.1–§2.3;
//! `docs/ttrpg/05-combat/ranged_combat_and_armor.md` §9.1).
//!
//! Extracted from `lifepath_economy` so hardcoded character fixtures and narrative walkthroughs
//! remain cleanly separated from the core Lifepath & Economy engine rules.

use std::collections::BTreeMap;

use crate::characteristics::{AptitudeCode, CharacterAptitudes, CharacterCharacteristics};
use crate::lifepath_economy::{LifepathCharacterProfile, RuralYouthArchetype, StreetGangArchetype};
use crate::skills_trees::SkillAccessTier;

pub(crate) fn insert_skill_ranks(skills: &mut BTreeMap<String, usize>, ranks: &[(&str, usize)]) {
    for &(id, rank) in ranks {
        skills.insert(id.to_string(), rank);
    }
}

/// Build the canonical walkthrough profile for **Mateo Vance — The Gifted Dropout & Gang Enforcer**
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §2.1; `docs/ttrpg/05-combat/ranged_combat_and_armor.md` §9.1).
#[must_use]
pub fn build_mateo_vance_lifepath() -> LifepathCharacterProfile {
    let chars = CharacterCharacteristics {
        ana: 13,
        mem: 11,
        foc: 12,
        ins: 12,
        dis: 11,
        spd: 14,
        dex: 13,
        kin: 12,
        for_stat: 11,
        fit: 12,
        hlt: 12,
        sig: 12,
        hea: 11,
        itu: 13,
        pre: 12,
        elo: 10,
        ..Default::default()
    };
    let mut aptitudes = CharacterAptitudes::default();
    aptitudes[AptitudeCode::AptQnt] = 3;
    aptitudes[AptitudeCode::AptFin] = 1;

    let mut mateo = LifepathCharacterProfile::new_with_stage2_baseline(
        "Mateo Vance",
        8,
        "alertness_situational_awareness",
        chars,
        aptitudes,
    );

    insert_skill_ranks(
        &mut mateo.skills,
        &[
            ("reading_comprehension", 2),
            ("arithmetic", 2),
            ("brawling_unarmed_striking", 2),
            ("elementary_algebra_geometry", 2),
            ("trigonometry", 2),
            ("general_biology_natural_history", 1),
        ],
    );
    mateo.stage_history.push(
        "Stage 3 (Ages 12–15): Reached Elementary Algebra N=2 & Trigonometry N=2 via APT-QNT +3 (C_study=15, TN_edu=18); forced out of school at Age 15.".to_string(),
    );

    mateo.apply_stage4b_dropout_gang_package(StreetGangArchetype::ShooterEnforcer);
    mateo.age_years = 22;
    insert_skill_ranks(
        &mut mateo.skills,
        &[
            ("driving_wheeled_tracked", 1),
            ("pistols_handguns", 4),
            ("fully_automatic_burst_weapons", 2),
            ("intimidation_coercion", 3),
            ("streetwise_underworld_etiquette", 3),
        ],
    );
    mateo.campaign_sqm = 4;
    mateo.operational_heat = 2;
    mateo.stage_history.push(
        "Stage 5 (Ages 18–22): Organized Crime / Syndicate Enforcer (Pistols N=4, Fully Automatic Weapons N=2, Intimidation N=3, Streetwise N=3).".to_string(),
    );

    mateo
}

/// Build the canonical walkthrough profile for **Dr. Priya Nair — Corporate AI & Robotics Scientist**
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §2.2).
#[must_use]
pub fn build_priya_nair_lifepath() -> LifepathCharacterProfile {
    let chars = CharacterCharacteristics {
        ana: 16,
        mem: 15,
        dis: 14,
        foc: 14,
        ins: 13,
        dex: 11,
        sig: 11,
        hea: 10,
        for_stat: 8,
        spd: 9,
        fit: 9,
        hlt: 10,
        kin: 9,
        elo: 12,
        pre: 11,
        ..Default::default()
    };
    let mut aptitudes = CharacterAptitudes::default();
    aptitudes[AptitudeCode::AptQnt] = 2;
    aptitudes[AptitudeCode::AptCmp] = 3;
    aptitudes[AptitudeCode::AptMec] = 1;

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
    priya.age_years = 26;
    insert_skill_ranks(
        &mut priya.skills,
        &[
            ("reading_comprehension", 3),
            ("writing_composition", 3),
            ("arithmetic", 3),
            ("digital_ai_literacy", 3),
            ("elementary_algebra_geometry", 3),
            ("trigonometry", 3),
            ("classical_mechanics_physics", 2),
            ("calculus_linear_algebra", 4),
            ("differential_equations_vector_calculus", 3),
            ("statistics_probability", 3),
            ("programming_scripting", 4),
            ("algorithms_systems_architecture", 4),
            ("machine_learning_ai_engineering", 5),
            ("thermodynamics_fluid_dynamics", 2),
            ("mechanical_structural_engineering", 3),
            ("aerospace_naval_robotics_engineering", 4),
            ("electrical_power_systems", 2),
            ("micro_electronics_avionics_cyberware_tech", 3),
        ],
    );
    priya.campaign_sqm = 8;
    priya.operational_heat = 0;

    priya
}

/// Build the canonical walkthrough profile for **Amara Okafor — Rural Bush Tracker, Herbalist & Anti-Poaching Ranger**
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §2.3).
#[must_use]
pub fn build_amara_okafor_lifepath() -> LifepathCharacterProfile {
    let chars = CharacterCharacteristics {
        ins: 14,
        sig: 14,
        hea: 13,
        olf: 12,
        fit: 13,
        hlt: 13,
        for_stat: 11,
        dex: 12,
        kin: 12,
        dis: 12,
        mem: 12,
        ana: 10,
        ..Default::default()
    };
    let mut aptitudes = CharacterAptitudes::default();
    aptitudes[AptitudeCode::AptBio] = 2;
    aptitudes[AptitudeCode::AptKin] = 1;
    aptitudes[AptitudeCode::AptFin] = 1;

    let mut amara = LifepathCharacterProfile::new_with_stage2_baseline(
        "Amara Okafor",
        5,
        "foraging_campcraft",
        chars,
        aptitudes,
    );
    insert_skill_ranks(
        &mut amara.skills,
        &[
            ("reading_comprehension", 2),
            ("arithmetic", 2),
            ("stealth_sneaking_hiding", 2),
            ("general_biology_natural_history", 2),
            ("first_aid_bls_cpr", 1),
        ],
    );
    amara.stage_history.push(
        "Stage 3 (Ages 12–15): Village Middle School — General Biology & Natural History N=2; transitioned at Age 15 to Path 4C Rural / Subsistence Youth.".to_string(),
    );

    amara.apply_stage4c_rural_agrarian_package(RuralYouthArchetype::SubsistenceHunterTrapper);
    amara.age_years = 22;
    insert_skill_ranks(
        &mut amara.skills,
        &[
            ("agriculture_agronomy_pastoralism", 1),
            ("animal_handling_husbandry_riding", 2),
            ("herbalism_ethnobotany_natural_remedies", 2),
            ("foraging_campcraft", 3),
            ("stealth_sneaking_hiding", 3),
            ("hunting_trapping_field_butchery", 4),
            ("wilderness_survival_biome_specific", 3),
            ("long_guns_rifles_carbines_shotguns", 3),
            ("land_navigation_cartography", 2),
            ("camouflage_concealment", 2),
            ("veterinary_medicine_livestock_care", 2),
        ],
    );
    amara.campaign_sqm = 4;
    amara.operational_heat = 0;
    amara.stage_history.push(
        "Stage 5 (Ages 18–22): Rural, Agrarian, Wilderness Guide & Frontier Ranger (Hunting & Trapping N=4, Wilderness Survival N=3, Long Guns N=3, Herbalism N=2, Veterinary Care N=2).".to_string(),
    );

    amara
}
