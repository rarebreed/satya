//! Chronological 5-Stage Lifepath Creation, Socioeconomic Upkeep, Black-Market Procurement & Operational Heat
//! (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1–§1.7, §2.1–§2.3, §3.1–§3.2).
//!
//! Implements:
//! - Stage 1: Socioeconomic Origin & School Quality Modifier (`SQM`) (`2d20` table) (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1, in [`crate::economy_gear`])
//! - Educational Advancement Roll (`2d20` cascade with $N = 2$ pairs vs. $\text{TN}_{\text{edu}} = 31 - C_{\text{study}} + \text{SQM}$) (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.2)
//! - Stages 2, 3, 4A/4B/4C (High School Tracks vs. Dropout **Academic Tree Freeze**, Street Gang Archetypes, & Rural Youth),
//!   and Stage 5 Adult 4-Year Career Terms (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.3–§1.7)
//! - Canonical Character Walkthrough Builders & Skill Lock Auditors (**Mateo Vance** §2.1, **Dr. Priya Nair** §2.2, **Amara Okafor** §2.3)
//! - Campaign Socioeconomic Upkeep (`SQM 1–10`), Black-Market `[Access]` Procurement (`Class I–IV`),
//!   and Operational `Heat` (`0–10`) (`docs/ttrpg/03-character/lifepaths_and_economy.md` §3.1–§3.2, re-exported from [`crate::economy_gear`])

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::characteristics::{CharacterAptitudes, CharacterCharacteristics};
pub use crate::economy_gear::*;
pub use crate::lifepath_catalog::*;
use crate::skills_trees::{
    SkillAccessTier, SkillAdvancementValidation, initialize_universal_talents,
    validate_skill_advancement,
};
use crate::walkthroughs::insert_skill_ranks;
pub use crate::walkthroughs::{
    build_amara_okafor_lifepath, build_mateo_vance_lifepath, build_priya_nair_lifepath,
};

/// Complete Lifepath Character Profile produced by the 5-Stage Chronological Lifepath Engine
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1–§1.7, §2.1–§2.3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LifepathCharacterProfile {
    pub name: String,
    pub age_years: u32,
    pub origin: SocioeconomicOriginSpec,
    pub characteristics: CharacterCharacteristics,
    pub aptitudes: CharacterAptitudes,
    pub skills: BTreeMap<String, usize>,
    pub granted_access_tags: Vec<SkillAccessTier>,
    pub academic_tree_frozen: bool,
    pub stage_history: Vec<String>,
    pub campaign_sqm: u8,
    pub operational_heat: u8,
}

impl LifepathCharacterProfile {
    /// Initialize a new character at Stage 1 (`Ages 0–5`) and Stage 2 (`Ages 6–11`) baseline
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1, §1.3).
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

        if skills.contains_key(early_talent_choice)
            || origin
                .early_environment_bonus_talents
                .contains(&early_talent_choice)
        {
            let level = skills.entry(early_talent_choice.to_string()).or_insert(1);
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

    /// Apply Stage 4B Dropout / Street Gang Transition, triggering the **Academic Tree Freeze**
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.5).
    pub fn apply_stage4b_dropout_gang_package(&mut self, archetype: StreetGangArchetype) {
        self.age_years = 18;
        self.academic_tree_frozen = true;
        archetype.apply_grants(
            &mut self.skills,
            &mut self.granted_access_tags,
            self.characteristics.for_stat,
            true,
        );
        self.stage_history.push(format!(
            "Stage 4B (Ages 16–18): Dropped out into {:?} — Academic Tree Frozen; granted [Access: Illicit]",
            archetype
        ));
    }

    /// Apply Stage 4C Rural, Agrarian & Developing-Region Youth Transition (`Ages 15–18`)
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.5).
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

    /// Audit whether this character can unlock or advance `skill_id` to `target_level`
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §2.1, §2.2).
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

/// Configuration for a single 4-Year Adult Career Term in Stage 5 (`Ages 18+`)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CareerTermConfig {
    pub track: CareerTermTrack,
    pub term_roll_pairs: [(i32, i32); 2],
    pub override_successes: Option<usize>,
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

/// Complete player-configurable 5-Stage Lifepath state for the Character Creation Wizard
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1–§1.7, §3.1–§3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifepathBuildConfig {
    /// Stage 1: `2d20` roll (`2..=40`) for Socioeconomic Origin (or set via class override).
    pub origin_roll_2d20: i32,
    pub stage1_early_talent: String,
    pub stage2_lit_pairs: [(i32, i32); 2],
    pub stage2_lit_override: Option<usize>,
    pub stage2_math_pairs: [(i32, i32); 2],
    pub stage2_math_override: Option<usize>,
    pub stage2_playground_talent: String,
    pub stage3_math_pairs: [(i32, i32); 2],
    pub stage3_math_override: Option<usize>,
    pub stage3_sci_pairs: [(i32, i32); 2],
    pub stage3_sci_override: Option<usize>,
    pub stage3_hum_pairs: [(i32, i32); 2],
    pub stage3_hum_override: Option<usize>,
    pub age15_crisis_roll_2d20: i32,
    pub age15_dropout_override: Option<bool>,
    pub stage4_path_override: Option<Stage4Path>,
    pub stage4a_hs_track: HighSchoolTrack,
    pub stage4a_hs_pairs: [(i32, i32); 2],
    pub stage4a_hs_override: Option<usize>,
    pub stage4b_gang_archetype: StreetGangArchetype,
    pub stage4c_rural_archetype: RuralYouthArchetype,
    pub adult_ged_waiver: bool,
    pub career_terms: Vec<CareerTermConfig>,
    pub campaign_sqm_override: Option<u8>,
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

/// Detailed result of evaluating Stages 1–5 of a [`LifepathBuildConfig`]
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1–§1.7, §3.1–§3.2).
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
    pub baseline_skills: BTreeMap<String, usize>,
    pub granted_access_tags: Vec<SkillAccessTier>,
    pub academic_tree_frozen: bool,
    pub career_skill_budget: usize,
    pub elective_skill_budget: usize,
    pub max_post_school_gain_per_skill: usize,
    pub campaign_sqm: u8,
    pub operational_heat: u8,
    pub stage_history: Vec<String>,
}

/// Deterministically evaluates Stages 1–5 from [`LifepathBuildConfig`], returning the Stage 1–4 skill floor,
/// Stage 5 `[Access]` gates, Career/Elective skill budgets, and chronological history log
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.1–§1.7, §3.1–§3.2).
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
    let early_tal = if skills.contains_key(&cfg.stage1_early_talent)
        || origin
            .early_environment_bonus_talents
            .contains(&cfg.stage1_early_talent.as_str())
    {
        cfg.stage1_early_talent.as_str()
    } else {
        origin
            .early_environment_bonus_talents
            .first()
            .copied()
            .unwrap_or("alertness_situational_awareness")
    };
    if !skills.contains_key(early_tal) {
        skills.insert(early_tal.to_string(), 1);
    }
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
    let alg_before_s3 = skills
        .get("elementary_algebra_geometry")
        .copied()
        .unwrap_or(0);
    if arith_before_s3 >= 2 {
        bump_skill_floor(&mut skills, "elementary_algebra_geometry", 1);
        if stage3_math_eval.successes >= 1 {
            bump_skill_floor(&mut skills, "elementary_algebra_geometry", 2);
        }
        if stage3_math_eval.successes >= 2 {
            bump_skill_floor(
                &mut skills,
                "trigonometry",
                if alg_before_s3 >= 1 { 2 } else { 1 },
            );
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
