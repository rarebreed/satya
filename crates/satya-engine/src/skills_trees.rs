//! Canonical 8 Hierarchical Skill Trees Catalog & DAG Validator
//! (`docs/ttrpg/03-character/skills_and_trees.md` Â§1.1â€“Â§1.5, Â§2.1â€“Â§2.4, Â§3.1â€“Â§3.9;
//! `docs/ttrpg/03-character/lifepaths_and_economy.md` Â§1.7.2â€“Â§1.7.3).
//!
//! Implements:
//! - `SkillTreeId` (Trees 1â€“8), `SkillType` (`TAL`, `PHY`, `KNO`, `PHY/KNO`), multi-tag `SkillCategory`,
//!   and `SkillAccessTier` (`Universal`, `Academic`, `VocationalLicensed`, `Restricted`, `Illicit`) (`docs/ttrpg/03-character/skills_and_trees.md` Â§1.1â€“Â§1.5)
//! - Universal Childhood Baseline initializer (`Native Language Speech` at $N = 2$; all other `TAL` skills at $N = 1$) (`docs/ttrpg/03-character/skills_and_trees.md` Â§2.1, Â§3.1)
//! - Complete 8-Tree DAG Registry covering all canonical nodes across Trees 1â€“8 (`docs/ttrpg/03-character/skills_and_trees.md` Â§3.2â€“Â§3.9)
//! - Prerequisite & Advancement Validator enforcing:
//!   1. `[Access]` Tag Gates (`Â§1.5`)
//!   2. Minimum Characteristic (`Min C`) Thresholds (with `-1` relief when linked Domain Aptitude $\ge +2$) (`Â§2.1`)
//!   3. Parent Skill Level Thresholds ($N_{\text{parent}} \ge X$, supporting both AND and OR parent groups) (`Â§2.2`)
//!   4. **Foundation Ceiling Rule**: $N_{\text{child}} \le N_{\text{parent}} + 2$ (`Â§2.3`)
//! - Tree & Sibling Defaulting Resolver (`+4` $\Delta_{\text{ext}}$ per tier step down-tree, `+4` sibling default, or `[No Default]` block) (`docs/ttrpg/03-character/skills_and_trees.md` Â§2.4)
//! - 5-Factor Skill Point (`SP`) Step Cost Calculator (`docs/ttrpg/03-character/lifepaths_and_economy.md` Â§1.7.2â€“Â§1.7.3)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::characteristics::{
    AptitudeCode, CharacterAptitudes, CharacterCharacteristics, CharacteristicCode,
    compute_aptitude_min_c_relief, compute_aptitude_training_time_multiplier,
};
use crate::define_coded_enum;
use crate::task_resolution::ToolDependency;

define_coded_enum! {
    /// The 8 canonical Hierarchical Skill Trees (`docs/ttrpg/03-character/skills_and_trees.md` Â§3.2â€“Â§3.9).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum SkillTreeId [count = 8] {
        /// Tree 1: Mathematics, Computing & Physical Sciences (`APT-QNT`, `APT-CMP`) (`Â§3.2`)
        Tree1MathCompPhysicalSciences => { code: "TREE_1", label: "Tree 1: Mathematics, Computing & Physical Sciences", aliases: ["1"] },
        /// Tree 2: Mechanical, Craft & Pilotage (`APT-MEC`, `APT-FIN`) (`Â§3.3`)
        Tree2MechanicalCraftPilotage => { code: "TREE_2", label: "Tree 2: Mechanical, Craft & Pilotage", aliases: ["2"] },
        /// Tree 3: Biology, Chemistry & Medicine (`APT-BIO`) (`Â§3.4`)
        Tree3BiologyChemistryMedicine => { code: "TREE_3", label: "Tree 3: Biology, Chemistry & Medicine", aliases: ["3"] },
        /// Tree 4: Language, Humanities & Social Influence (`APT-LNG`, `APT-SOC`) (`Â§3.8`)
        Tree4LanguageHumanitiesSocial => { code: "TREE_4", label: "Tree 4: Language, Humanities & Social Influence", aliases: ["4"] },
        /// Tree 5: Athletics, Acrobatics & Melee Combat (`APT-KIN`) (`Â§3.5`)
        Tree5AthleticsAcrobaticsMelee => { code: "TREE_5", label: "Tree 5: Athletics, Acrobatics & Melee Combat", aliases: ["5"] },
        /// Tree 6: Marksmanship, Gunnery & Tactical Systems (`APT-FIN`) (`Â§3.6`)
        Tree6MarksmanshipGunneryTactical => { code: "TREE_6", label: "Tree 6: Marksmanship, Gunnery & Tactical Systems", aliases: ["6"] },
        /// Tree 7: Perception, Stealth & Survival (`APT-AUD`, `APT-KIN`) (`Â§3.9`)
        Tree7PerceptionStealthSurvival => { code: "TREE_7", label: "Tree 7: Perception, Stealth & Survival", aliases: ["7"] },
        /// Tree 8: Visual, Musical & Culinary Arts (`APT-FIN`, `APT-AUD`) (`Â§3.7`)
        Tree8VisualMusicalCulinaryArts => { code: "TREE_8", label: "Tree 8: Visual, Musical & Culinary Arts", aliases: ["8"] },
    }
}

impl SkillTreeId {
    /// Short tree code (`"TREE_1"` .. `"TREE_8"`).
    #[must_use]
    pub const fn short_code(self) -> &'static str {
        self.as_str()
    }

    /// Parse a `SkillTreeId` from its short code (`"TREE_1"` .. `"TREE_8"`).
    #[must_use]
    pub fn from_code(s: &str) -> Option<Self> {
        Self::from_code_str(s)
    }

    /// Canonical tree title from `docs/ttrpg/03-character/skills_and_trees.md` (Â§3.2â€“Â§3.9).
    #[must_use]
    pub const fn title(self) -> &'static str {
        self.label()
    }

    /// Primary Domain Aptitude associated with this Skill Tree (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` Â§9).
    #[must_use]
    pub const fn primary_aptitude(self) -> AptitudeCode {
        match self {
            Self::Tree1MathCompPhysicalSciences => AptitudeCode::AptQnt,
            Self::Tree2MechanicalCraftPilotage => AptitudeCode::AptMec,
            Self::Tree3BiologyChemistryMedicine => AptitudeCode::AptBio,
            Self::Tree4LanguageHumanitiesSocial => AptitudeCode::AptLng,
            Self::Tree5AthleticsAcrobaticsMelee => AptitudeCode::AptKin,
            Self::Tree6MarksmanshipGunneryTactical => AptitudeCode::AptFin,
            Self::Tree7PerceptionStealthSurvival => AptitudeCode::AptAud,
            Self::Tree8VisualMusicalCulinaryArts => AptitudeCode::AptFin,
        }
    }
}

define_coded_enum! {
    /// Skill Type (`TAL`, `PHY`, `KNO`, `PHY/KNO`) (`docs/ttrpg/03-character/skills_and_trees.md` Â§1.1â€“Â§1.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SkillType [count = 4] {
        /// `TAL`: Innate Talent â€” universal childhood baseline ($N \ge 1$, `Native Language Speech` starts at $N = 2$)
        Tal => { code: "TAL", label: "Innate Talent" },
        /// `PHY`: Physical / Motor Skill
        Phy => { code: "PHY", label: "Physical / Motor Skill" },
        /// `KNO`: Knowledge / Academic / Technical Skill
        Kno => { code: "KNO", label: "Knowledge / Academic Skill" },
        /// `PHY/KNO`: Hybrid Motor + Technical/Tactical Skill
        PhyKno => { code: "PHY/KNO", label: "Hybrid Physical & Knowledge Skill", aliases: ["PHY_KNO"] },
    }
}

define_coded_enum! {
    /// Multi-category functional tags assigned to skills (`docs/ttrpg/03-character/skills_and_trees.md` Â§1.3).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SkillCategory [count = 10] {
        /// Athletic & locomotive skills.
        Athletic => { code: "ATHLETIC", label: "Athletic" },
        /// Melee, ranged, and tactical combat skills.
        Combat => { code: "COMBAT", label: "Combat" },
        /// Mechanical, electrical, and craft skills.
        Technical => { code: "TECHNICAL", label: "Technical" },
        /// Mathematical, physical, and natural sciences.
        Science => { code: "SCIENCE", label: "Science" },
        /// Humanities, literacy, and academic scholarship.
        Knowledge => { code: "KNOWLEDGE", label: "Knowledge" },
        /// Computing, networks, and cyber-electronics.
        Cyber => { code: "CYBER", label: "Cyber" },
        /// First aid, clinical medicine, and surgery.
        Medical => { code: "MEDICAL", label: "Medical" },
        /// Visual, musical, performing, and culinary arts.
        Artistic => { code: "ARTISTIC", label: "Artistic" },
        /// Interpersonal persuasion, deception, and leadership.
        Social => { code: "SOCIAL", label: "Social" },
        /// Perception, stealth, survival, and outdoor fieldcraft.
        Fieldcraft => { code: "FIELDCRAFT", label: "Fieldcraft" },
    }
}

define_coded_enum! {
    /// Institutional or subcultural `[Access]` gate required to learn or advance a skill node
    /// (`docs/ttrpg/03-character/skills_and_trees.md` Â§1.5, Â§2.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SkillAccessTier [count = 5] {
        /// `[Access: Universal]`: Open to all characters through everyday life or self-study
        Universal => { code: "UNIVERSAL", label: "Universal" },
        /// `[Access: Academic]`: Requires formal school, high school STEM, or university enrollment
        Academic => { code: "ACADEMIC", label: "Academic" },
        /// `[Access: Vocational/Licensed]`: Requires trade school, guild apprenticeship, clinical certification, or licensed range
        VocationalLicensed => { code: "VOCATIONAL_LICENSED", label: "Vocational/Licensed", aliases: ["Vocational/Licensed"] },
        /// `[Access: Restricted]`: Requires military, law enforcement, intelligence, or cleared corporate security access
        Restricted => { code: "RESTRICTED", label: "Restricted" },
        /// `[Access: Illicit]`: Requires street gang, criminal syndicate, insurgent cell, or black-market mentor
        Illicit => { code: "ILLICIT", label: "Illicit" },
    }
}

/// How multiple parent skill prerequisites are combined on a node
/// (`docs/ttrpg/03-character/skills_and_trees.md` Â§2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrerequisiteMode {
    /// Every listed `(parent_skill_id, min_level)` must be met (`AND`).
    AllOf,
    /// At least one listed `(parent_skill_id, min_level)` must be met (`OR`).
    AnyOf,
}

/// Canonical specification for a single node in the 8 Hierarchical Skill Trees
/// (`docs/ttrpg/03-character/skills_and_trees.md` Â§3.2â€“Â§3.9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillNodeSpec {
    /// Canonical snake_case identifier.
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// Hierarchical tree (`TREE_1` .. `TREE_8`).
    pub tree: SkillTreeId,
    /// Tree depth tier (`0..=6`).
    pub tier: u8,
    /// Inherent learning difficulty rating (`1` = Routine/Easy .. `5` = Extreme/Apex), independent of tree tier.
    pub difficulty: u8,
    /// Skill type (`TAL`, `PHY`, `KNO`, `PHY/KNO`).
    pub skill_type: SkillType,
    /// Multi-category functional tags.
    pub categories: &'static [SkillCategory],
    /// Primary controlling characteristic (`C_1`).
    pub primary_char: CharacteristicCode,
    /// Optional secondary controlling characteristic (`C_2`).
    pub secondary_char: Option<CharacteristicCode>,
    /// Minimum characteristic (`Min C`) gates required to learn/advance this node.
    pub min_chars: &'static [(CharacteristicCode, i32)],
    /// Combinator (`AllOf` vs `AnyOf`) for `parent_prereqs`.
    pub parent_mode: PrerequisiteMode,
    /// Required parent skill IDs and minimum levels `(parent_id, min_n)`.
    pub parent_prereqs: &'static [(&'static str, usize)],
    /// Institutional or subcultural `[Access]` tags that unlock this node.
    pub allowed_access: &'static [SkillAccessTier],
    /// Tool/equipment requirement (`None`, `Situational`, `Mandatory`).
    pub tool_dependency: ToolDependency,
    /// If `false`, this node is marked `[No Default]` and cannot be attempted at $N = 0$ via tree defaulting.
    pub defaultable: bool,
    /// Associated Domain Aptitude override (defaults to `tree.primary_aptitude()` if `None`).
    pub aptitude_override: Option<AptitudeCode>,
}

impl SkillNodeSpec {
    /// Effective Domain Aptitude governing this skill node.
    #[must_use]
    pub const fn governing_aptitude(&self) -> AptitudeCode {
        match self.aptitude_override {
            Some(apt) => apt,
            None => self.tree.primary_aptitude(),
        }
    }

    /// Inherent Skill Point (SP) difficulty multiplier derived from `self.difficulty` (`1..=5`) and `self.skill_type`
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` Â§1.7.2).
    ///
    /// - `Difficulty 1` (`TAL`): `0.80x`
    /// - `Difficulty 1` (`PHY` / `KNO` / `PHY/KNO`, e.g. *Swimming*, *Driving*, *Reading*): `1.00x`
    /// - `Difficulty 2` (Standard Trade / Athletics / Basic Arms): `1.25x`
    /// - `Difficulty 3` (Challenging Specialist): `1.60x`
    /// - `Difficulty 4` (Complex Professional / Fixed-Wing Aviation / Engineering): `2.00x`
    /// - `Difficulty 5` (Extreme / Rotary-Wing Helicopter / Trauma Surgery / Frontier Science): `2.50x`
    #[must_use]
    pub const fn difficulty_multiplier(&self) -> f64 {
        match self.difficulty {
            0 | 1 => match self.skill_type {
                SkillType::Tal => 0.80,
                _ => 1.00,
            },
            2 => 1.25,
            3 => 1.60,
            4 => 2.00,
            _ => 2.50,
        }
    }

    /// Short human-readable label for the skill's inherent difficulty rating (`1..=5`).
    #[must_use]
    pub const fn difficulty_label(&self) -> &'static str {
        match self.difficulty {
            0 | 1 => "Diff 1 (Routine)",
            2 => "Diff 2 (Moderate)",
            3 => "Diff 3 (Challenging)",
            4 => "Diff 4 (Complex)",
            _ => "Diff 5 (Extreme)",
        }
    }
}

pub use crate::skills_catalog::*;
/// Specific reason why a skill unlock or advancement was rejected by the DAG validator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillValidationFailure {
    /// Machine-readable rejection code.
    pub code: &'static str,
    /// Human-readable diagnostic explanation.
    pub detail: String,
}

/// Result of validating whether a character can unlock or advance a skill node to `target_level`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillAdvancementValidation {
    /// Canonical skill identifier.
    pub skill_id: String,
    /// Human-readable skill name.
    pub skill_name: String,
    /// Requested target skill level (`target_level`).
    pub target_level: usize,
    /// True if all prerequisite, access, characteristic, and foundation ceiling gates passed.
    pub is_valid: bool,
    /// Effective controlling characteristic score $C_{\text{eff}}$.
    pub effective_controller_c: i32,
    /// Minimum characteristic relief (`0` or `1`) granted by Domain Aptitude $\ge +2$.
    pub aptitude_min_c_relief: i32,
    /// Any validation gate failures encountered.
    pub failures: Vec<SkillValidationFailure>,
}

/// Validate whether a character meets all 4 canonical gates to learn or advance `skill_id` to `target_level`
/// (`docs/ttrpg/03-character/skills_and_trees.md` Â§1.5, Â§2.1â€“Â§2.3):
/// 1. **Academic Tree Freeze / `[Access]` Gate**: Character must hold at least one of the node's `allowed_access` tags
///    (`Universal` is always granted; if `academic_tree_frozen` is true, `Academic`-only nodes cannot be advanced above current level).
/// 2. **Minimum Characteristic (`Min C`) Threshold**: Checked against character's `1â€“20` scores, reduced by `-1` if the
///    node's governing Domain Aptitude is $\ge +2$.
/// 3. **Parent Skill Threshold**: $N_{\text{parent}} \ge X$ (with `AllOf` or `AnyOf` semantics).
/// 4. **Foundation Ceiling Rule**: $N_{\text{target}} \le N_{\text{parent}} + 2$.
#[must_use]
pub fn validate_skill_advancement(
    skill_id: &str,
    target_level: usize,
    current_skills: &BTreeMap<String, usize>,
    characteristics: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
    granted_access_tags: &[SkillAccessTier],
    academic_tree_frozen: bool,
) -> SkillAdvancementValidation {
    let Some(spec) = lookup_skill_node(skill_id) else {
        return SkillAdvancementValidation {
            skill_id: skill_id.to_string(),
            skill_name: skill_id.to_string(),
            target_level,
            is_valid: false,
            effective_controller_c: 10,
            aptitude_min_c_relief: 0,
            failures: vec![SkillValidationFailure {
                code: "UNKNOWN_SKILL",
                detail: format!("Skill '{skill_id}' is not in the canonical 8-tree registry."),
            }],
        };
    };

    let mut failures = Vec::new();
    let effective_controller_c =
        characteristics.compute_controller(spec.primary_char, spec.secondary_char);
    let apt_score = aptitudes.get(spec.governing_aptitude());
    let min_c_relief = compute_aptitude_min_c_relief(apt_score);

    // 0. Talent floor check
    if spec.skill_type == SkillType::Tal && target_level == 0 {
        failures.push(SkillValidationFailure {
            code: "TALENT_FLOOR_VIOLATION",
            detail: format!(
                "Talent '{}' (TAL) has a universal floor of N >= 1 and cannot be reduced to 0.",
                spec.name
            ),
        });
    }

    // 1. Access Tag Gate & Academic Tree Freeze check
    let current_level = current_skills.get(spec.id).copied().unwrap_or(0);
    let is_universal = spec.allowed_access.contains(&SkillAccessTier::Universal);
    let has_matching_access = is_universal
        || spec.allowed_access.iter().any(|req| {
            if *req == SkillAccessTier::Academic && academic_tree_frozen {
                false
            } else {
                granted_access_tags.contains(req)
            }
        });

    if !has_matching_access && target_level > current_level {
        failures.push(SkillValidationFailure {
            code: "MISSING_ACCESS_GATE",
            detail: format!(
                "Skill '{}' requires one of {:?} (Academic Frozen: {}).",
                spec.name, spec.allowed_access, academic_tree_frozen
            ),
        });
    }

    // 2. Minimum Characteristic (Min C) Threshold check (with APT >= +2 relief of -1)
    for &(char_code, required_min_c) in spec.min_chars {
        let effective_min_c = (required_min_c - min_c_relief).max(1);
        let actual_c = characteristics.get_scale(char_code);
        if actual_c < effective_min_c {
            failures.push(SkillValidationFailure {
                code: "MIN_CHARACTERISTIC_SHORTFALL",
                detail: format!(
                    "Skill '{}' requires {} >= {} (actual: {}).",
                    spec.name,
                    char_code.as_str(),
                    effective_min_c,
                    actual_c
                ),
            });
        }
    }

    // 3. Parent Skill Threshold & 4. Foundation Ceiling (N_child <= N_parent + 2)
    if !spec.parent_prereqs.is_empty() {
        match spec.parent_mode {
            PrerequisiteMode::AllOf => {
                for &(parent_id, min_parent_n) in spec.parent_prereqs {
                    let parent_n = current_skills.get(parent_id).copied().unwrap_or(0);
                    if parent_n < min_parent_n {
                        failures.push(SkillValidationFailure {
                            code: "PARENT_PREREQ_SHORTFALL",
                            detail: format!(
                                "Skill '{}' requires parent '{}' >= {} (actual: N={}).",
                                spec.name, parent_id, min_parent_n, parent_n
                            ),
                        });
                    } else if target_level > parent_n + 2 {
                        failures.push(SkillValidationFailure {
                            code: "FOUNDATION_CEILING_EXCEEDED",
                            detail: format!(
                                "Foundation Ceiling violated: '{}' (target N={}) cannot exceed parent '{}' (N={}) + 2.",
                                spec.name, target_level, parent_id, parent_n
                            ),
                        });
                    }
                }
            }
            PrerequisiteMode::AnyOf => {
                let any_meets_min = spec
                    .parent_prereqs
                    .iter()
                    .any(|&(parent_id, min_parent_n)| {
                        current_skills.get(parent_id).copied().unwrap_or(0) >= min_parent_n
                    });
                if !any_meets_min {
                    failures.push(SkillValidationFailure {
                        code: "PARENT_PREREQ_SHORTFALL",
                        detail: format!(
                            "Skill '{}' requires at least one parent prerequisite from {:?}.",
                            spec.name, spec.parent_prereqs
                        ),
                    });
                } else {
                    let max_qualifying_parent_n = spec
                        .parent_prereqs
                        .iter()
                        .filter_map(|&(parent_id, min_parent_n)| {
                            let n = current_skills.get(parent_id).copied().unwrap_or(0);
                            if n >= min_parent_n { Some(n) } else { None }
                        })
                        .max()
                        .unwrap_or(0);
                    if target_level > max_qualifying_parent_n + 2 {
                        failures.push(SkillValidationFailure {
                            code: "FOUNDATION_CEILING_EXCEEDED",
                            detail: format!(
                                "Foundation Ceiling violated: '{}' (target N={}) cannot exceed qualifying parent (N={}) + 2.",
                                spec.name, target_level, max_qualifying_parent_n
                            ),
                        });
                    }
                }
            }
        }
    }

    SkillAdvancementValidation {
        skill_id: spec.id.to_string(),
        skill_name: spec.name.to_string(),
        target_level,
        is_valid: failures.is_empty(),
        effective_controller_c,
        aptitude_min_c_relief: min_c_relief,
        failures,
    }
}

/// Result of resolving a Skill Tree Default when a character has $N = 0$ in a target skill
/// (`docs/ttrpg/03-character/skills_and_trees.md` Â§2.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDefaultResolution {
    /// Requested target skill identifier.
    pub target_skill_id: String,
    /// True if the skill can be attempted (either directly or via defaulting).
    pub can_attempt: bool,
    /// Source skill ID used (`target_skill_id` if trained directly, or parent/sibling/grandparent ID if defaulted).
    pub source_skill_id: Option<String>,
    /// Effective skill level $N$ rolled.
    pub effective_n: usize,
    /// External difficulty penalty $\Delta_{\text{ext}}$ (`0`, `+4`, or `+8`).
    pub default_penalty_delta_ext: i32,
    /// Diagnostic explanation of how the default was resolved.
    pub reason: String,
}

/// Resolve whether a character can attempt `target_skill_id` directly or by defaulting down-tree / across siblings
/// (`docs/ttrpg/03-character/skills_and_trees.md` Â§2.4).
///
/// - If the character already has $N \ge 1$ in `target_skill_id`, returns `effective_n = N` and `default_penalty_delta_ext = 0`.
/// - If $N = 0$ and `spec.defaultable == false` (`[No Default]`), the task is **Gated** (`can_attempt = false`).
/// - Otherwise searches immediate parent (`+4` $\Delta_{\text{ext}}$), trained sibling sharing a parent in the same tree
///   (`+4` $\Delta_{\text{ext}}$; preferred over an untrained $N = 1$ `TAL` parent when the sibling has $N \ge 2$),
///   or grandparent (`+8` $\Delta_{\text{ext}}$).
#[must_use]
pub fn resolve_skill_tree_default(
    target_skill_id: &str,
    current_skills: &BTreeMap<String, usize>,
) -> SkillDefaultResolution {
    let Some(spec) = lookup_skill_node(target_skill_id) else {
        return SkillDefaultResolution {
            target_skill_id: target_skill_id.to_string(),
            can_attempt: false,
            source_skill_id: None,
            effective_n: 0,
            default_penalty_delta_ext: 0,
            reason: format!("Unknown skill '{target_skill_id}'."),
        };
    };

    let direct_n = current_skills.get(spec.id).copied().unwrap_or(0);
    if direct_n >= 1 {
        return SkillDefaultResolution {
            target_skill_id: spec.id.to_string(),
            can_attempt: true,
            source_skill_id: Some(spec.id.to_string()),
            effective_n: direct_n,
            default_penalty_delta_ext: 0,
            reason: format!("Trained directly in '{}' at N={direct_n}.", spec.name),
        };
    }

    if !spec.defaultable {
        return SkillDefaultResolution {
            target_skill_id: spec.id.to_string(),
            can_attempt: false,
            source_skill_id: None,
            effective_n: 0,
            default_penalty_delta_ext: 0,
            reason: format!(
                "Skill '{}' is hard-gated [No Default] and cannot be attempted untrained (N=0).",
                spec.name
            ),
        };
    }

    // 1. Check immediate parents (1 tier step -> +4 Delta_ext)
    let mut best_parent: Option<(&str, usize)> = None;
    for &(parent_id, _) in spec.parent_prereqs {
        let p_n = current_skills.get(parent_id).copied().unwrap_or(0);
        if p_n >= 1 && best_parent.is_none_or(|(_, best_n)| p_n > best_n) {
            best_parent = Some((parent_id, p_n));
        }
    }

    // 2. Check trained siblings sharing a parent in the same tree (+4 Delta_ext)
    let mut best_sibling: Option<(&str, usize)> = None;
    for node in CANONICAL_SKILL_TREES {
        if node.tree == spec.tree && node.id != spec.id {
            let sib_n = current_skills.get(node.id).copied().unwrap_or(0);
            if sib_n >= 1 {
                let shares_parent = node
                    .parent_prereqs
                    .iter()
                    .any(|(p1, _)| spec.parent_prereqs.iter().any(|(p2, _)| p1 == p2));
                if shares_parent && best_sibling.is_none_or(|(_, best_n)| sib_n > best_n) {
                    best_sibling = Some((node.id, sib_n));
                }
            }
        }
    }

    if let Some((parent_id, parent_n)) = best_parent {
        let parent_is_untrained_tal = parent_n == 1
            && lookup_skill_node(parent_id).is_some_and(|p| p.skill_type == SkillType::Tal);
        if parent_is_untrained_tal
            && let Some((sib_id, sib_n)) = best_sibling
            && sib_n > parent_n
        {
            return SkillDefaultResolution {
                target_skill_id: spec.id.to_string(),
                can_attempt: true,
                source_skill_id: Some(sib_id.to_string()),
                effective_n: sib_n,
                default_penalty_delta_ext: 4,
                reason: format!(
                    "Sibling default from '{}' (N={sib_n}) with +4 Delta_ext.",
                    sib_id
                ),
            };
        }
        return SkillDefaultResolution {
            target_skill_id: spec.id.to_string(),
            can_attempt: true,
            source_skill_id: Some(parent_id.to_string()),
            effective_n: parent_n,
            default_penalty_delta_ext: 4,
            reason: format!(
                "Down-tree default from parent '{}' (N={parent_n}) with +4 Delta_ext.",
                parent_id
            ),
        };
    }

    if let Some((sib_id, sib_n)) = best_sibling {
        return SkillDefaultResolution {
            target_skill_id: spec.id.to_string(),
            can_attempt: true,
            source_skill_id: Some(sib_id.to_string()),
            effective_n: sib_n,
            default_penalty_delta_ext: 4,
            reason: format!(
                "Sibling default from '{}' (N={sib_n}) with +4 Delta_ext.",
                sib_id
            ),
        };
    }

    // 3. Check grandparents (2 tier steps -> +8 Delta_ext)
    for &(parent_id, _) in spec.parent_prereqs {
        if let Some(parent_spec) = lookup_skill_node(parent_id) {
            for &(gp_id, _) in parent_spec.parent_prereqs {
                let gp_n = current_skills.get(gp_id).copied().unwrap_or(0);
                if gp_n >= 1 {
                    return SkillDefaultResolution {
                        target_skill_id: spec.id.to_string(),
                        can_attempt: true,
                        source_skill_id: Some(gp_id.to_string()),
                        effective_n: gp_n,
                        default_penalty_delta_ext: 8,
                        reason: format!(
                            "Down-tree default (2 steps) from '{}' (N={gp_n}) with +8 Delta_ext.",
                            gp_id
                        ),
                    };
                }
            }
        }
    }

    SkillDefaultResolution {
        target_skill_id: spec.id.to_string(),
        can_attempt: false,
        source_skill_id: None,
        effective_n: 0,
        default_penalty_delta_ext: 0,
        reason: format!(
            "No trained parent or sibling skill available to default into '{}'.",
            spec.name
        ),
    }
}

/// Whether a skill step is acquired through **Formal Instruction** (`FOC` + `INS` institutional curriculum)
/// or **Practical Experience / Self-Study** (`INS` elective / street practice)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` Â§1.7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillTrainingMode {
    /// Institutional / Career curriculum governed by Focus (`FOC`) and Insight (`INS`).
    FormalTraining,
    /// On-the-job / Elective / Hobby / Street experience governed by Insight (`INS`) and skill type (`TAL`/`PHY` vs `KNO`).
    Experience,
}

impl SkillTrainingMode {
    /// Human-readable label for this training mode.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::FormalTraining => "Formal Training (FOC + INS)",
            Self::Experience => "Experience (INS)",
        }
    }
}

/// Full 5-factor breakdown of the Skill Point (`SP`) cost to advance a skill by 1 level to `target_level` ($N$)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` Â§1.7.2â€“Â§1.7.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillStepCostBreakdown {
    /// Canonical skill identifier.
    pub skill_id: String,
    /// Target skill level $N$ being unlocked or advanced to.
    pub target_level: usize,
    /// Training mode (`FormalTraining` vs `Experience`).
    pub training_mode: SkillTrainingMode,
    /// Triangular step base cost: $\text{Base}(N) = 10 \times N$.
    pub base_level_sp: usize,
    /// Inherent skill difficulty rating ($1\dots 5$).
    pub difficulty_rating: u8,
    /// Difficulty multiplier ($0.80\times$ for D1 Tal, $1.00\times$ for D1 Phy/Kno, $1.25\times$ D2, $1.60\times$ D3, $2.00\times$ D4, $2.50\times$ D5).
    pub difficulty_multiplier: f64,
    /// Effective controlling characteristic $C_{\text{eff}} \in [1, 20]$ ($\lfloor(C_1 + C_2)/2\rfloor$ composite controller per `characteristics_and_aptitudes.md` Â§10.1).
    pub controller_c: i32,
    /// Controlling characteristic multiplier: $\text{clamp}(1.0 - 0.04 \times (C_{\text{eff}} - 10),\; 0.65,\; 1.40)$.
    pub controller_multiplier: f64,
    /// Governing Domain Aptitude score $A \in [-2, +3]$.
    pub aptitude_score: i32,
    /// Domain Aptitude multiplier ($-15\%$ per $+1$, $+20\%$ per $-1$).
    pub aptitude_multiplier: f64,
    /// Training mode multiplier (`FormalTraining` via `FOC`+`INS` vs `Experience` via `INS`).
    pub training_mode_multiplier: f64,
    /// Final rounded Skill Point (`SP`) cost for this single step (minimum `3 SP`).
    pub final_sp_cost: usize,
}

/// Compute the 5-factor Skill Point (`SP`) cost to advance `skill_id` from `target_level - 1` to `target_level` ($N$)
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` Â§1.7.2â€“Â§1.7.3):
///
/// $$\text{SP}(s, N) = \max\!\Big(3,\; \text{round}\big(\text{Base}(N) \times M_{\text{diff}}(s) \times M_{\text{ctrl}}(C_{\text{eff}}) \times M_{\text{apt}}(A) \times M_{\text{train}}(\text{mode}, \text{FOC}, \text{INS})\big)\Big)$$
///
/// 1. **Target Level Scaling**: $\text{Base}(N) = 10 \times N$ (`10 SP` for $N=1$, `20 SP` for $N=2$, `30 SP` for $N=3$, `40 SP` for $N=4$, `50 SP` for $N=5$).
/// 2. **Inherent Skill Difficulty ($1\dots 5$)**:
///    - `Difficulty 1 (Basic)`: `0.80x` (`TAL`) / `1.00x` (`PHY`/`KNO`, e.g., *Swimming & Diving*)
///    - `Difficulty 2 (Standard)`: `1.25x` (e.g., *Pistols & Handguns*, *Ground Vehicle Operation*)
///    - `Difficulty 3 (Complex)`: `1.60x` (e.g., *Calculus*, *Fully Automatic Weapons*)
///    - `Difficulty 4 (Advanced)`: `2.00x` (e.g., *Fixed-Wing Aircraft*, *Precision Marksmanship*)
///    - `Difficulty 5 (Extreme)`: `2.50x` (e.g., *Piloting (Rotary-Wing / Helicopter & VTOL)*, *Trauma Surgery*)
/// 3. **Controlling Characteristic ($C_{\text{eff}}$)**:
///    $$M_{\text{ctrl}} = \text{clamp}\big(1.0 - 0.04 \times (C_{\text{eff}} - 10),\; 0.65,\; 1.40\big)$$
/// 4. **Governing Domain Aptitude ($A \in [-2, +3]$)**:
///    $$M_{\text{apt}} = \begin{cases} 1.0 - 0.15 A & A \ge 0 \\ 1.0 + 0.20 |A| & A < 0 \end{cases}$$
/// 5. **Training Mode (`FormalTraining` via `FOC` + `INS` vs `Experience` via `INS`)**:
///    - `FormalTraining`: $\text{clamp}\big(0.85 \times (1.0 - 0.03(\text{FOC} - 10) - 0.03(\text{INS} - 10)),\; 0.45,\; 1.35\big)$
///    - `Experience`: $\text{clamp}\big(B_{\text{type}} \times (1.0 - 0.02(\text{INS} - 10)),\; 0.65,\; 1.60\big)$, where $B_{\text{type}} = 1.00$ for `TAL`/`PHY` and $1.25$ for `KNO`/`PHY/KNO`.
#[must_use]
pub fn compute_skill_step_sp_cost(
    skill_id: &str,
    target_level: usize,
    training_mode: SkillTrainingMode,
    characteristics: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
) -> SkillStepCostBreakdown {
    let n = target_level.max(1);
    let base_level_sp = 10 * n;

    let Some(spec) = lookup_skill_node(skill_id) else {
        return SkillStepCostBreakdown {
            skill_id: skill_id.to_string(),
            target_level: n,
            training_mode,
            base_level_sp,
            difficulty_rating: 2,
            difficulty_multiplier: 1.25,
            controller_c: 10,
            controller_multiplier: 1.0,
            aptitude_score: 0,
            aptitude_multiplier: 1.0,
            training_mode_multiplier: 1.0,
            final_sp_cost: base_level_sp,
        };
    };

    let difficulty_rating = spec.difficulty.clamp(1, 5);
    let difficulty_multiplier = spec.difficulty_multiplier();

    let controller_c = characteristics.compute_controller(spec.primary_char, spec.secondary_char);
    let controller_multiplier = (1.0 - 0.04 * f64::from(controller_c - 10)).clamp(0.65, 1.40);

    let aptitude_score = aptitudes.get(spec.governing_aptitude());
    let aptitude_multiplier = compute_aptitude_training_time_multiplier(aptitude_score);

    let foc = characteristics.foc.clamp(1, 20);
    let ins = characteristics.ins.clamp(1, 20);
    let training_mode_multiplier = match training_mode {
        SkillTrainingMode::FormalTraining => {
            let study_factor = 1.0 - 0.03 * f64::from(foc - 10) - 0.03 * f64::from(ins - 10);
            (0.85 * study_factor).clamp(0.45, 1.35)
        }
        SkillTrainingMode::Experience => {
            let type_base = match spec.skill_type {
                SkillType::Tal | SkillType::Phy => 1.00,
                SkillType::Kno | SkillType::PhyKno => 1.25,
            };
            let insight_factor = 1.0 - 0.02 * f64::from(ins - 10);
            (type_base * insight_factor).clamp(0.65, 1.60)
        }
    };

    let raw_sp = (base_level_sp as f64)
        * difficulty_multiplier
        * controller_multiplier
        * aptitude_multiplier
        * training_mode_multiplier;
    let final_sp_cost = (raw_sp.round().max(3.0)) as usize;

    SkillStepCostBreakdown {
        skill_id: spec.id.to_string(),
        target_level: n,
        training_mode,
        base_level_sp,
        difficulty_rating,
        difficulty_multiplier,
        controller_c,
        controller_multiplier,
        aptitude_score,
        aptitude_multiplier,
        training_mode_multiplier,
        final_sp_cost,
    }
}
