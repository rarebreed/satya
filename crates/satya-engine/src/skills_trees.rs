//! Canonical 8 Hierarchical Skill Trees Catalog (~105 Skills) & DAG Validator
//! (`03-character/skills_and_trees.md`).
//!
//! Implements:
//! - `SkillTreeId` (Trees 1–8), `SkillType` (`TAL`, `PHY`, `KNO`, `PHY/KNO`), multi-tag `SkillCategory`,
//!   and `SkillAccessTier` (`Universal`, `Academic`, `VocationalLicensed`, `Restricted`, `Illicit`) (§1)
//! - Universal Childhood Baseline initializer (`Native Language Speech` at $N = 2$; all 15 other `TAL` skills at $N = 1$) (§2.1)
//! - Complete 8-Tree DAG Registry covering all canonical nodes across Trees 1–8 (§4.1–§4.8)
//! - Prerequisite & Advancement Validator enforcing:
//!   1. `[Access]` Tag Gates
//!   2. Minimum Characteristic (`Min C`) Thresholds (with `-1` relief when linked Domain Aptitude $\ge +2$)
//!   3. Parent Skill Level Thresholds ($N_{\text{parent}} \ge X$, supporting both AND and OR parent groups)
//!   4. **Foundation Ceiling Rule**: $N_{\text{child}} \le N_{\text{parent}} + 2$ (§3.1)
//! - Tree & Sibling Defaulting Resolver (`+4` $\Delta_{\text{ext}}$ per tier step down-tree, `+4..+6` sibling, or `[No Default]` block) (§3.1)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::characteristics::{
    AptitudeCode, CharacterAptitudes, CharacterCharacteristics, CharacteristicCode,
    compute_aptitude_min_c_relief,
};
use crate::task_resolution::ToolDependency;

/// The 8 canonical Hierarchical Skill Trees (§4.1–§4.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SkillTreeId {
    /// Tree 1: Mathematics, Computing & Physical Sciences (`APT-QNT`, `APT-CMP`)
    Tree1MathCompPhysicalSciences,
    /// Tree 2: Mechanical, Craft & Pilotage (`APT-MEC`, `APT-FIN`)
    Tree2MechanicalCraftPilotage,
    /// Tree 3: Biology, Chemistry & Medicine (`APT-BIO`)
    Tree3BiologyChemistryMedicine,
    /// Tree 4: Language, Humanities & Social Influence (`APT-LNG`, `APT-SOC`)
    Tree4LanguageHumanitiesSocial,
    /// Tree 5: Athletics, Acrobatics & Melee Combat (`APT-KIN`)
    Tree5AthleticsAcrobaticsMelee,
    /// Tree 6: Marksmanship, Gunnery & Tactical Systems (`APT-FIN`)
    Tree6MarksmanshipGunneryTactical,
    /// Tree 7: Perception, Stealth & Survival (`APT-AUD`, `APT-KIN`)
    Tree7PerceptionStealthSurvival,
    /// Tree 8: Visual, Musical & Culinary Arts (`APT-FIN`, `APT-AUD`)
    Tree8VisualMusicalCulinaryArts,
}

impl SkillTreeId {
    /// Primary Domain Aptitude associated with this Skill Tree (§9 of `characteristics_and_aptitudes.md`).
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

    /// Canonical tree title from `skills_and_trees.md`.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Tree1MathCompPhysicalSciences => {
                "Tree 1: Mathematics, Computing & Physical Sciences"
            }
            Self::Tree2MechanicalCraftPilotage => "Tree 2: Mechanical, Craft & Pilotage",
            Self::Tree3BiologyChemistryMedicine => "Tree 3: Biology, Chemistry & Medicine",
            Self::Tree4LanguageHumanitiesSocial => {
                "Tree 4: Language, Humanities & Social Influence"
            }
            Self::Tree5AthleticsAcrobaticsMelee => "Tree 5: Athletics, Acrobatics & Melee Combat",
            Self::Tree6MarksmanshipGunneryTactical => {
                "Tree 6: Marksmanship, Gunnery & Tactical Systems"
            }
            Self::Tree7PerceptionStealthSurvival => "Tree 7: Perception, Stealth & Survival",
            Self::Tree8VisualMusicalCulinaryArts => "Tree 8: Visual, Musical & Culinary Arts",
        }
    }
}

/// Skill Type (`TAL`, `PHY`, `KNO`, `PHY/KNO`) (§1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillType {
    /// `TAL`: Innate Talent — universal childhood baseline ($N \ge 1$, `Native Language Speech` starts at $N = 2$)
    Tal,
    /// `PHY`: Physical / Motor Skill
    Phy,
    /// `KNO`: Knowledge / Academic / Technical Skill
    Kno,
    /// `PHY/KNO`: Hybrid Motor + Technical/Tactical Skill
    PhyKno,
}

/// Multi-category functional tags assigned to skills (§1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillCategory {
    Athletic,
    Combat,
    Technical,
    Science,
    Knowledge,
    Cyber,
    Medical,
    Artistic,
    Social,
    Fieldcraft,
}

/// Institutional or subcultural `[Access]` gate required to learn or advance a skill node (§1, §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillAccessTier {
    /// `[Access: Universal]`: Open to all characters through everyday life or self-study
    Universal,
    /// `[Access: Academic]`: Requires formal school, high school STEM, or university enrollment
    Academic,
    /// `[Access: Vocational/Licensed]`: Requires trade school, guild apprenticeship, clinical certification, or licensed range
    VocationalLicensed,
    /// `[Access: Restricted]`: Requires military, law enforcement, intelligence, or cleared corporate security access
    Restricted,
    /// `[Access: Illicit]`: Requires street gang, criminal syndicate, insurgent cell, or black-market mentor
    Illicit,
}

/// How multiple parent skill prerequisites are combined on a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrerequisiteMode {
    /// Every listed `(parent_skill_id, min_level)` must be met (`AND`).
    AllOf,
    /// At least one listed `(parent_skill_id, min_level)` must be met (`OR`).
    AnyOf,
}

/// Canonical specification for a single node in the 8 Hierarchical Skill Trees (§4.1–§4.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillNodeSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub tree: SkillTreeId,
    pub tier: u8,
    pub skill_type: SkillType,
    pub categories: &'static [SkillCategory],
    pub primary_char: CharacteristicCode,
    pub secondary_char: Option<CharacteristicCode>,
    pub min_chars: &'static [(CharacteristicCode, i32)],
    pub parent_mode: PrerequisiteMode,
    pub parent_prereqs: &'static [(&'static str, usize)],
    pub allowed_access: &'static [SkillAccessTier],
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
}

/// Complete canonical catalog of skills across all 8 Hierarchical Skill Trees (`03-character/skills_and_trees.md`).
pub const CANONICAL_SKILL_TREES: &[SkillNodeSpec] = &[
    // =========================================================================
    // TREE 1: MATHEMATICS, COMPUTING & PHYSICAL SCIENCES
    // =========================================================================
    SkillNodeSpec {
        id: "counting_basic_numeracy",
        name: "Counting & Basic Numeracy",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Knowledge],
        primary_char: CharacteristicCode::MEM,
        secondary_char: Some(CharacteristicCode::INS),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "arithmetic",
        name: "Arithmetic",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 1,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge, SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[(CharacteristicCode::ANA, 6)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("counting_basic_numeracy", 1)],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "digital_ai_literacy",
        name: "Digital & AI Literacy",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 1,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge, SkillCategory::Cyber],
        primary_char: CharacteristicCode::MEM,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("counting_basic_numeracy", 1)],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::Situational,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptCmp),
    },
    SkillNodeSpec {
        id: "elementary_algebra_geometry",
        name: "Elementary Algebra & Geometry",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science, SkillCategory::Knowledge],
        primary_char: CharacteristicCode::ANA,
        secondary_char: None,
        min_chars: &[(CharacteristicCode::ANA, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("arithmetic", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "trigonometry",
        name: "Trigonometry",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: None,
        min_chars: &[(CharacteristicCode::ANA, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("elementary_algebra_geometry", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "calculus_linear_algebra",
        name: "Calculus & Linear Algebra",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 4,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::ANA, 11)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("trigonometry", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "differential_equations_vector_calculus",
        name: "Differential Equations & Vector Calculus",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 5,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::ANA, 13)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("calculus_linear_algebra", 3)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "statistics_probability",
        name: "Statistics & Probability",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science, SkillCategory::Knowledge],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::INS),
        min_chars: &[(CharacteristicCode::ANA, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("elementary_algebra_geometry", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "programming_scripting",
        name: "Programming & Scripting",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Cyber, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::ANA, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("elementary_algebra_geometry", 1),
            ("digital_ai_literacy", 2),
        ],
        allowed_access: &[
            SkillAccessTier::Academic,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptCmp),
    },
    SkillNodeSpec {
        id: "algorithms_systems_architecture",
        name: "Algorithms & Systems Architecture",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Cyber, SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::ANA, 11)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("programming_scripting", 2),
            ("elementary_algebra_geometry", 2),
        ],
        allowed_access: &[
            SkillAccessTier::Academic,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptCmp),
    },
    SkillNodeSpec {
        id: "cybersecurity_network_intrusion",
        name: "Cybersecurity & Network Intrusion",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Cyber, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::INS),
        min_chars: &[(CharacteristicCode::ANA, 11), (CharacteristicCode::FOC, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("programming_scripting", 2)],
        allowed_access: &[
            SkillAccessTier::Academic,
            SkillAccessTier::Restricted,
            SkillAccessTier::Illicit,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptCmp),
    },
    SkillNodeSpec {
        id: "cryptography_quantum_computing",
        name: "Cryptography & Quantum Computing",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 5,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Cyber, SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::ANA, 14)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("algorithms_systems_architecture", 3),
            ("calculus_linear_algebra", 3),
        ],
        allowed_access: &[SkillAccessTier::Academic, SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptCmp),
    },
    SkillNodeSpec {
        id: "machine_learning_ai_engineering",
        name: "Machine Learning & AI Engineering",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 5,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Cyber, SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::ANA, 13)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("algorithms_systems_architecture", 2),
            ("statistics_probability", 2),
            ("calculus_linear_algebra", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptCmp),
    },
    SkillNodeSpec {
        id: "classical_mechanics_physics",
        name: "Classical Mechanics (Physics)",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 4,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: None,
        min_chars: &[(CharacteristicCode::ANA, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("trigonometry", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "electromagnetism_optics",
        name: "Electromagnetism & Optics",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 5,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: None,
        min_chars: &[(CharacteristicCode::ANA, 12)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("classical_mechanics_physics", 2),
            ("calculus_linear_algebra", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "thermodynamics_fluid_dynamics",
        name: "Thermodynamics & Fluid Dynamics",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 5,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: None,
        min_chars: &[(CharacteristicCode::ANA, 12)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("classical_mechanics_physics", 2),
            ("calculus_linear_algebra", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "quantum_relativity_nuclear_physics",
        name: "Quantum, Relativity & Nuclear Physics",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 6,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::ANA, 14)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("electromagnetism_optics", 3),
            ("differential_equations_vector_calculus", 3),
        ],
        allowed_access: &[SkillAccessTier::Academic, SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "astronomy_astrophysics_orbital_mechanics",
        name: "Astronomy, Astrophysics & Orbital Mechanics",
        tree: SkillTreeId::Tree1MathCompPhysicalSciences,
        tier: 5,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[(CharacteristicCode::ANA, 12)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("classical_mechanics_physics", 2),
            ("calculus_linear_algebra", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Situational,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    // =========================================================================
    // TREE 2: MECHANICAL, CRAFT & PILOTAGE
    // =========================================================================
    SkillNodeSpec {
        id: "manual_dexterity_tinkering",
        name: "Manual Dexterity & Tinkering",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Technical],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::TAC),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "carpentry_woodworking",
        name: "Carpentry & Woodworking",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 1,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical, SkillCategory::Artistic],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::FOR),
        min_chars: &[(CharacteristicCode::DEX, 7)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("manual_dexterity_tinkering", 1)],
        allowed_access: &[
            SkillAccessTier::Universal,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "masonry_construction",
        name: "Masonry & Construction",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 1,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical, SkillCategory::Athletic],
        primary_char: CharacteristicCode::FOR,
        secondary_char: Some(CharacteristicCode::DEX),
        min_chars: &[(CharacteristicCode::FOR, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("manual_dexterity_tinkering", 1)],
        allowed_access: &[
            SkillAccessTier::Universal,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "metalworking_machining_blacksmithing",
        name: "Metalworking, Machining & Blacksmithing",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 2,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::FOR),
        min_chars: &[(CharacteristicCode::DEX, 8), (CharacteristicCode::FOR, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("manual_dexterity_tinkering", 2)],
        allowed_access: &[SkillAccessTier::VocationalLicensed],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "lockpicking_physical_intrusion",
        name: "Lockpicking & Physical Intrusion",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 2,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::TAC),
        min_chars: &[(CharacteristicCode::DEX, 9), (CharacteristicCode::TAC, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("manual_dexterity_tinkering", 2)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
            SkillAccessTier::Illicit,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "automotive_small_engine_mechanics",
        name: "Automotive & Small Engine Mechanics",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 2,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::DEX, 8), (CharacteristicCode::ANA, 7)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("manual_dexterity_tinkering", 2), ("arithmetic", 1)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Universal,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "electrical_power_systems",
        name: "Electrical & Power Systems",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 2,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::DEX),
        min_chars: &[(CharacteristicCode::ANA, 8), (CharacteristicCode::DEX, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("manual_dexterity_tinkering", 2),
            ("elementary_algebra_geometry", 1),
        ],
        allowed_access: &[SkillAccessTier::VocationalLicensed],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "micro_electronics_avionics_cyberware_tech",
        name: "Micro-Electronics, Avionics & Cyberware Tech",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical, SkillCategory::Cyber],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::DEX, 11), (CharacteristicCode::ANA, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("electrical_power_systems", 2)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Academic,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "armourer_gunsmithing",
        name: "Armourer & Gunsmithing",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical, SkillCategory::Combat],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::DEX, 9), (CharacteristicCode::ANA, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("metalworking_machining_blacksmithing", 2)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
            SkillAccessTier::Illicit,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "mechanical_structural_engineering",
        name: "Mechanical & Structural Engineering",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 4,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::ANA, 11)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("classical_mechanics_physics", 2),
            ("calculus_linear_algebra", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Situational,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "aerospace_naval_robotics_engineering",
        name: "Aerospace, Naval & Robotics Engineering",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 5,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::ANA, 13)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("mechanical_structural_engineering", 3),
            ("thermodynamics_fluid_dynamics", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic, SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "driving_wheeled_tracked",
        name: "Driving (Wheeled & Tracked Ground Vehicles)",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 1,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Technical, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[(CharacteristicCode::DEX, 7), (CharacteristicCode::SIG, 6)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("manual_dexterity_tinkering", 1)],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "small_boat_seamanship",
        name: "Small Boat & Seamanship",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 1,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::KIN,
        secondary_char: Some(CharacteristicCode::INS),
        min_chars: &[(CharacteristicCode::KIN, 7)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("manual_dexterity_tinkering", 1)],
        allowed_access: &[
            SkillAccessTier::Universal,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "piloting_fixed_wing_rotary",
        name: "Piloting (Fixed-Wing & Rotary Aircraft)",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::FOC),
        min_chars: &[
            (CharacteristicCode::DEX, 10),
            (CharacteristicCode::SIG, 10),
            (CharacteristicCode::FOC, 10),
        ],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("driving_wheeled_tracked", 2), ("trigonometry", 1)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    SkillNodeSpec {
        id: "spacecraft_orbital_helm",
        name: "Spacecraft & Orbital Helm",
        tree: SkillTreeId::Tree2MechanicalCraftPilotage,
        tier: 4,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical, SkillCategory::Science],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::DEX, 11), (CharacteristicCode::ANA, 11)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("piloting_fixed_wing_rotary", 2),
            ("astronomy_astrophysics_orbital_mechanics", 2),
        ],
        allowed_access: &[SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptMec),
    },
    // =========================================================================
    // TREE 3: BIOLOGY, CHEMISTRY & MEDICINE
    // =========================================================================
    SkillNodeSpec {
        id: "hygiene_basic_self_care",
        name: "Hygiene & Basic Self-Care",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Medical],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "first_aid_bls_cpr",
        name: "First Aid & Basic Life Support (BLS / CPR)",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 1,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Medical, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::FOC),
        min_chars: &[(CharacteristicCode::FOC, 7)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("hygiene_basic_self_care", 1)],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::Situational,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "general_biology_natural_history",
        name: "General Biology & Natural History",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 1,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science, SkillCategory::Knowledge],
        primary_char: CharacteristicCode::MEM,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::MEM, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("hygiene_basic_self_care", 1)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "general_chemistry",
        name: "General Chemistry",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[(CharacteristicCode::ANA, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("elementary_algebra_geometry", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "organic_analytical_chemistry",
        name: "Organic & Analytical Chemistry",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[(CharacteristicCode::ANA, 11)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("general_chemistry", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "anatomy_physiology",
        name: "Anatomy & Physiology",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Medical, SkillCategory::Science],
        primary_char: CharacteristicCode::MEM,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::MEM, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("general_biology_natural_history", 2)],
        allowed_access: &[
            SkillAccessTier::Academic,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "paramedic_combat_trauma_care",
        name: "Paramedic & Combat Trauma Care",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Medical, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::FOC),
        min_chars: &[(CharacteristicCode::DEX, 9), (CharacteristicCode::FOC, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("first_aid_bls_cpr", 2), ("anatomy_physiology", 1)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "nursing_patient_care",
        name: "Nursing & Patient Care",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Medical],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[(CharacteristicCode::MEM, 9), (CharacteristicCode::DIS, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("first_aid_bls_cpr", 2), ("anatomy_physiology", 2)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Academic,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "pharmacology_toxicology",
        name: "Pharmacology & Toxicology",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 4,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Medical, SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[(CharacteristicCode::ANA, 11), (CharacteristicCode::MEM, 11)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("organic_analytical_chemistry", 2),
            ("anatomy_physiology", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic, SkillAccessTier::Illicit],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "clinical_diagnosis_internal_medicine",
        name: "Clinical Diagnosis & Internal Medicine",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 4,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Medical, SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::INS),
        min_chars: &[(CharacteristicCode::ANA, 12), (CharacteristicCode::MEM, 12)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("anatomy_physiology", 3), ("pharmacology_toxicology", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "trauma_orthopedic_general_surgery",
        name: "Trauma, Orthopedic & General Surgery",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 5,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Medical],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::FOC),
        min_chars: &[
            (CharacteristicCode::DEX, 12),
            (CharacteristicCode::FOC, 12),
            (CharacteristicCode::ANA, 11),
        ],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("clinical_diagnosis_internal_medicine", 2),
            ("paramedic_combat_trauma_care", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "genetics_biotech_cyber_implantation",
        name: "Genetics, Biotech & Cyber-Implantation",
        tree: SkillTreeId::Tree3BiologyChemistryMedicine,
        tier: 6,
        skill_type: SkillType::PhyKno,
        categories: &[
            SkillCategory::Medical,
            SkillCategory::Science,
            SkillCategory::Cyber,
        ],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::DEX),
        min_chars: &[(CharacteristicCode::ANA, 14), (CharacteristicCode::DEX, 13)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("trauma_orthopedic_general_surgery", 3),
            ("organic_analytical_chemistry", 3),
        ],
        allowed_access: &[SkillAccessTier::Academic, SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    // =========================================================================
    // TREE 4: LANGUAGE, HUMANITIES & SOCIAL INFLUENCE
    // =========================================================================
    SkillNodeSpec {
        id: "native_language_speech",
        name: "Native Language Speech",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Social, SkillCategory::Knowledge],
        primary_char: CharacteristicCode::ELO,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptLng),
    },
    SkillNodeSpec {
        id: "empathy_social_perception",
        name: "Empathy & Social Perception",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Social],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::ITU),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "persuasion_fast_talk",
        name: "Persuasion & Fast-Talk",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Social],
        primary_char: CharacteristicCode::ELO,
        secondary_char: Some(CharacteristicCode::PRE),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "deception_bluff",
        name: "Deception & Bluff",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Social],
        primary_char: CharacteristicCode::ELO,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "intimidation_coercion",
        name: "Intimidation & Coercion",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Social, SkillCategory::Combat],
        primary_char: CharacteristicCode::PRE,
        secondary_char: Some(CharacteristicCode::FOR),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "reading_comprehension",
        name: "Reading (Comprehension)",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 1,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge],
        primary_char: CharacteristicCode::MEM,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::MEM, 6)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("native_language_speech", 2)],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptLng),
    },
    SkillNodeSpec {
        id: "writing_composition",
        name: "Writing (Composition)",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 1,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge, SkillCategory::Artistic],
        primary_char: CharacteristicCode::ELO,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[(CharacteristicCode::MEM, 6)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("reading_comprehension", 1)],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptLng),
    },
    SkillNodeSpec {
        id: "foreign_language_specific",
        name: "Foreign Language (Specific)",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 1,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge, SkillCategory::Social],
        primary_char: CharacteristicCode::MEM,
        secondary_char: Some(CharacteristicCode::ELO),
        min_chars: &[(CharacteristicCode::MEM, 7)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("native_language_speech", 2)],
        allowed_access: &[SkillAccessTier::Universal, SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptLng),
    },
    SkillNodeSpec {
        id: "rhetoric_oratory_debate",
        name: "Rhetoric, Oratory & Debate",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Social, SkillCategory::Knowledge],
        primary_char: CharacteristicCode::ELO,
        secondary_char: Some(CharacteristicCode::PRE),
        min_chars: &[(CharacteristicCode::ELO, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("writing_composition", 2), ("persuasion_fast_talk", 2)],
        allowed_access: &[SkillAccessTier::Academic, SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "research_archival_science",
        name: "Research & Archival Science",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[(CharacteristicCode::ANA, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("reading_comprehension", 2), ("writing_composition", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Situational,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptLng),
    },
    SkillNodeSpec {
        id: "history_political_science",
        name: "History & Political Science",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge],
        primary_char: CharacteristicCode::MEM,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::MEM, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("reading_comprehension", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptLng),
    },
    SkillNodeSpec {
        id: "philosophy_formal_logic_ethics",
        name: "Philosophy, Formal Logic & Ethics",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::INS),
        min_chars: &[(CharacteristicCode::ANA, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("writing_composition", 2), ("rhetoric_oratory_debate", 1)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptLng),
    },
    SkillNodeSpec {
        id: "linguistics_cryptanalysis_philology",
        name: "Linguistics, Cryptanalysis & Philology",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge, SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[(CharacteristicCode::ANA, 11)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("reading_comprehension", 3),
            ("foreign_language_specific", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptLng),
    },
    SkillNodeSpec {
        id: "psychology_behavioral_analysis_counseling",
        name: "Psychology, Behavioral Analysis & Counseling",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[
            SkillCategory::Science,
            SkillCategory::Medical,
            SkillCategory::Social,
        ],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::INS, 10), (CharacteristicCode::ANA, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("empathy_social_perception", 2),
            ("reading_comprehension", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "economics_finance_accounting",
        name: "Economics, Finance & Accounting",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge, SkillCategory::Science],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::MEM),
        min_chars: &[(CharacteristicCode::ANA, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("statistics_probability", 1), ("reading_comprehension", 2)],
        allowed_access: &[
            SkillAccessTier::Academic,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "law_jurisprudence",
        name: "Law & Jurisprudence",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 4,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Knowledge, SkillCategory::Social],
        primary_char: CharacteristicCode::MEM,
        secondary_char: Some(CharacteristicCode::ELO),
        min_chars: &[(CharacteristicCode::MEM, 11), (CharacteristicCode::ANA, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("rhetoric_oratory_debate", 2),
            ("history_political_science", 2),
        ],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "negotiation_diplomacy",
        name: "Negotiation & Diplomacy",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Social],
        primary_char: CharacteristicCode::ELO,
        secondary_char: Some(CharacteristicCode::INS),
        min_chars: &[(CharacteristicCode::ELO, 10), (CharacteristicCode::INS, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("persuasion_fast_talk", 2),
            ("empathy_social_perception", 2),
        ],
        allowed_access: &[
            SkillAccessTier::Universal,
            SkillAccessTier::Academic,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "leadership_command",
        name: "Leadership & Command",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Social, SkillCategory::Combat],
        primary_char: CharacteristicCode::PRE,
        secondary_char: Some(CharacteristicCode::DIS),
        min_chars: &[(CharacteristicCode::PRE, 10), (CharacteristicCode::DIS, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("persuasion_fast_talk", 2)],
        allowed_access: &[SkillAccessTier::Universal, SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "interrogation_counter_interrogation",
        name: "Interrogation & Counter-Interrogation",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Social, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::PRE),
        min_chars: &[(CharacteristicCode::INS, 10), (CharacteristicCode::DIS, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("intimidation_coercion", 2), ("deception_bluff", 2)],
        allowed_access: &[SkillAccessTier::Restricted, SkillAccessTier::Illicit],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "streetwise_underworld_etiquette",
        name: "Streetwise & Underworld Etiquette",
        tree: SkillTreeId::Tree4LanguageHumanitiesSocial,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Social, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::ITU),
        min_chars: &[(CharacteristicCode::INS, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("empathy_social_perception", 2), ("deception_bluff", 1)],
        allowed_access: &[SkillAccessTier::Illicit, SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    // =========================================================================
    // TREE 5: ATHLETICS, ACROBATICS & MELEE COMBAT
    // =========================================================================
    SkillNodeSpec {
        id: "running_sprinting",
        name: "Running & Sprinting",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Athletic],
        primary_char: CharacteristicCode::SPD,
        secondary_char: Some(CharacteristicCode::FIT),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "jumping_leaping",
        name: "Jumping & Leaping",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Athletic],
        primary_char: CharacteristicCode::FOR,
        secondary_char: Some(CharacteristicCode::SPD),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "balance_footwork",
        name: "Balance & Footwork",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Athletic],
        primary_char: CharacteristicCode::KIN,
        secondary_char: Some(CharacteristicCode::SPD),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "strength_lifting",
        name: "Strength & Lifting",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Athletic],
        primary_char: CharacteristicCode::FOR,
        secondary_char: Some(CharacteristicCode::FIT),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "brawling_unarmed_striking",
        name: "Brawling (Unarmed Striking)",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Combat, SkillCategory::Athletic],
        primary_char: CharacteristicCode::SPD,
        secondary_char: Some(CharacteristicCode::FOR),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "climbing_scaling",
        name: "Climbing & Scaling",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 1,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Athletic, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::FOR,
        secondary_char: Some(CharacteristicCode::KIN),
        min_chars: &[(CharacteristicCode::FOR, 7), (CharacteristicCode::KIN, 7)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("balance_footwork", 1), ("strength_lifting", 1)],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::Situational,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "swimming_diving",
        name: "Swimming & Diving",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 1,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Athletic, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::FIT,
        secondary_char: Some(CharacteristicCode::KIN),
        min_chars: &[(CharacteristicCode::FIT, 6)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("balance_footwork", 1)],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "acrobatics_tumbling_parkour",
        name: "Acrobatics, Tumbling & Parkour",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 2,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Athletic],
        primary_char: CharacteristicCode::KIN,
        secondary_char: Some(CharacteristicCode::SPD),
        min_chars: &[(CharacteristicCode::KIN, 10), (CharacteristicCode::SPD, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("balance_footwork", 2), ("jumping_leaping", 2)],
        allowed_access: &[
            SkillAccessTier::Universal,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "zero_g_eva_operations",
        name: "Zero-G / EVA Operations",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Athletic, SkillCategory::Technical],
        primary_char: CharacteristicCode::KIN,
        secondary_char: Some(CharacteristicCode::FOC),
        min_chars: &[(CharacteristicCode::KIN, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("acrobatics_tumbling_parkour", 2)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "martial_arts_striking",
        name: "Martial Arts (Striking — Boxing, Muay Thai, Karate)",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 2,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Combat, SkillCategory::Athletic],
        primary_char: CharacteristicCode::SPD,
        secondary_char: Some(CharacteristicCode::KIN),
        min_chars: &[(CharacteristicCode::SPD, 9), (CharacteristicCode::KIN, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("brawling_unarmed_striking", 2), ("balance_footwork", 2)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
            SkillAccessTier::Illicit,
        ],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "grappling_wrestling_submissions",
        name: "Grappling, Wrestling & Submissions (Judo, BJJ, Sambo)",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 2,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Combat, SkillCategory::Athletic],
        primary_char: CharacteristicCode::FOR,
        secondary_char: Some(CharacteristicCode::KIN),
        min_chars: &[(CharacteristicCode::FOR, 9), (CharacteristicCode::KIN, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("brawling_unarmed_striking", 2), ("balance_footwork", 2)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
            SkillAccessTier::Illicit,
        ],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "short_blades_knife_fighting",
        name: "Short Blades & Knife Fighting",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 1,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Combat],
        primary_char: CharacteristicCode::SPD,
        secondary_char: Some(CharacteristicCode::DEX),
        min_chars: &[(CharacteristicCode::SPD, 8), (CharacteristicCode::DEX, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("brawling_unarmed_striking", 1), ("balance_footwork", 1)],
        allowed_access: &[
            SkillAccessTier::Universal,
            SkillAccessTier::Restricted,
            SkillAccessTier::Illicit,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "swords_fencing_long_blades",
        name: "Swords, Fencing & Long Blades",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 2,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Combat, SkillCategory::Athletic],
        primary_char: CharacteristicCode::SPD,
        secondary_char: Some(CharacteristicCode::KIN),
        min_chars: &[(CharacteristicCode::SPD, 9), (CharacteristicCode::KIN, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("short_blades_knife_fighting", 2), ("balance_footwork", 2)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "blunt_impact_weapons_polearms",
        name: "Blunt / Impact Weapons & Polearms",
        tree: SkillTreeId::Tree5AthleticsAcrobaticsMelee,
        tier: 2,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Combat],
        primary_char: CharacteristicCode::FOR,
        secondary_char: Some(CharacteristicCode::SPD),
        min_chars: &[(CharacteristicCode::FOR, 9), (CharacteristicCode::KIN, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("brawling_unarmed_striking", 1), ("balance_footwork", 2)],
        allowed_access: &[SkillAccessTier::Universal, SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    // =========================================================================
    // TREE 6: MARKSMANSHIP, GUNNERY & TACTICAL SYSTEMS
    // =========================================================================
    SkillNodeSpec {
        id: "throwing_lobbing",
        name: "Throwing & Lobbing",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Athletic, SkillCategory::Combat],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::KIN),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "archery_bows_crossbows",
        name: "Archery (Bows & Crossbows)",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 1,
        skill_type: SkillType::Phy,
        categories: &[
            SkillCategory::Combat,
            SkillCategory::Athletic,
            SkillCategory::Fieldcraft,
        ],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[(CharacteristicCode::FOR, 8), (CharacteristicCode::DEX, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("throwing_lobbing", 1)],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "pistols_handguns",
        name: "Pistols & Handguns",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 1,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Combat],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[(CharacteristicCode::DEX, 8), (CharacteristicCode::SIG, 7)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("manual_dexterity_tinkering", 1)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
            SkillAccessTier::Illicit,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "long_guns_rifles_carbines_shotguns",
        name: "Long Guns (Rifles, Carbines & Shotguns)",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 1,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Combat, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[(CharacteristicCode::DEX, 7), (CharacteristicCode::SIG, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("manual_dexterity_tinkering", 1)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Restricted,
            SkillAccessTier::Illicit,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "fully_automatic_burst_weapons",
        name: "Fully Automatic & Burst Weapons (SMGs, Assault Rifles, LMGs)",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 2,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Combat],
        primary_char: CharacteristicCode::FOR,
        secondary_char: Some(CharacteristicCode::DEX),
        min_chars: &[(CharacteristicCode::FOR, 10), (CharacteristicCode::DEX, 9)],
        // Note: "Pistols >= 2 OR Long Guns >= 2" in Table 4.6
        parent_mode: PrerequisiteMode::AnyOf,
        parent_prereqs: &[
            ("pistols_handguns", 2),
            ("long_guns_rifles_carbines_shotguns", 2),
        ],
        allowed_access: &[SkillAccessTier::Restricted, SkillAccessTier::Illicit],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "precision_marksmanship_sniping",
        name: "Precision Marksmanship & Sniping",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Combat, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::SIG,
        secondary_char: Some(CharacteristicCode::FOC),
        min_chars: &[
            (CharacteristicCode::SIG, 12),
            (CharacteristicCode::FOC, 12),
            (CharacteristicCode::DEX, 10),
        ],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("long_guns_rifles_carbines_shotguns", 3),
            ("trigonometry", 1),
        ],
        allowed_access: &[SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "heavy_weapons_mounted_gunnery_atgms",
        name: "Heavy Weapons, Mounted Gunnery & ATGMs",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Combat, SkillCategory::Technical],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[(CharacteristicCode::FOR, 10), (CharacteristicCode::SIG, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("long_guns_rifles_carbines_shotguns", 2)],
        allowed_access: &[SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "indirect_fire_mortars_artillery",
        name: "Indirect Fire, Mortars & Artillery",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Combat, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[(CharacteristicCode::ANA, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("trigonometry", 2),
            ("heavy_weapons_mounted_gunnery_atgms", 1),
        ],
        allowed_access: &[SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "demolitions_explosive_ordnance_eod",
        name: "Demolitions & Explosive Ordnance (EOD)",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Technical, SkillCategory::Combat],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::FOC),
        min_chars: &[
            (CharacteristicCode::DEX, 11),
            (CharacteristicCode::FOC, 11),
            (CharacteristicCode::ANA, 9),
        ],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("general_chemistry", 1), ("electrical_power_systems", 1)],
        allowed_access: &[SkillAccessTier::Restricted, SkillAccessTier::Illicit],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "small_unit_tactics_cqb_doctrine",
        name: "Small-Unit Tactics & CQB Doctrine",
        tree: SkillTreeId::Tree6MarksmanshipGunneryTactical,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Combat, SkillCategory::Knowledge],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::ANA),
        min_chars: &[(CharacteristicCode::INS, 9), (CharacteristicCode::FOC, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("long_guns_rifles_carbines_shotguns", 2),
            ("leadership_command", 1),
        ],
        allowed_access: &[SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    // =========================================================================
    // TREE 7: PERCEPTION, STEALTH & SURVIVAL
    // =========================================================================
    SkillNodeSpec {
        id: "alertness_situational_awareness",
        name: "Alertness (Visual & Auditory Situational Awareness)",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::SIG,
        secondary_char: Some(CharacteristicCode::HEA),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptAud),
    },
    SkillNodeSpec {
        id: "stealth_sneaking_hiding",
        name: "Stealth (Sneaking & Hiding)",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Fieldcraft, SkillCategory::Athletic],
        primary_char: CharacteristicCode::KIN,
        secondary_char: Some(CharacteristicCode::ITU),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "foraging_campcraft",
        name: "Foraging & Campcraft",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::OLF),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "camouflage_concealment",
        name: "Camouflage & Concealment",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 1,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::INS, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("stealth_sneaking_hiding", 2)],
        allowed_access: &[SkillAccessTier::Universal, SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Situational,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "sleight_of_hand_pickpocketing_palming",
        name: "Sleight of Hand, Pickpocketing & Palming",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 2,
        skill_type: SkillType::Phy,
        categories: &[SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::TAC),
        min_chars: &[(CharacteristicCode::DEX, 10), (CharacteristicCode::TAC, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("stealth_sneaking_hiding", 2),
            ("manual_dexterity_tinkering", 2),
        ],
        allowed_access: &[SkillAccessTier::Illicit, SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "disguise_impersonation",
        name: "Disguise & Impersonation",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 2,
        skill_type: SkillType::PhyKno,
        categories: &[
            SkillCategory::Social,
            SkillCategory::Fieldcraft,
            SkillCategory::Artistic,
        ],
        primary_char: CharacteristicCode::CRE,
        secondary_char: Some(CharacteristicCode::INS),
        min_chars: &[(CharacteristicCode::CRE, 9), (CharacteristicCode::INS, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("deception_bluff", 2), ("camouflage_concealment", 1)],
        allowed_access: &[
            SkillAccessTier::Restricted,
            SkillAccessTier::Illicit,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "tracking_spoor_analysis",
        name: "Tracking & Spoor Analysis",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 1,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::SIG,
        secondary_char: Some(CharacteristicCode::INS),
        min_chars: &[(CharacteristicCode::SIG, 9), (CharacteristicCode::INS, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("alertness_situational_awareness", 2),
            ("foraging_campcraft", 1),
        ],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptAud),
    },
    SkillNodeSpec {
        id: "wilderness_survival_biome_specific",
        name: "Wilderness Survival (Biome-Specific)",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::INS,
        secondary_char: Some(CharacteristicCode::DIS),
        min_chars: &[(CharacteristicCode::DIS, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("foraging_campcraft", 2),
            ("general_biology_natural_history", 1),
        ],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::Situational,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "land_navigation_cartography",
        name: "Land Navigation & Cartography",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Fieldcraft, SkillCategory::Technical],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[(CharacteristicCode::ANA, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("alertness_situational_awareness", 1),
            ("elementary_algebra_geometry", 1),
        ],
        allowed_access: &[
            SkillAccessTier::Universal,
            SkillAccessTier::Academic,
            SkillAccessTier::Restricted,
        ],
        tool_dependency: ToolDependency::Situational,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptQnt),
    },
    SkillNodeSpec {
        id: "surveillance_reconnaissance_countersurveillance",
        name: "Surveillance, Reconnaissance & Countersurveillance",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 2,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Fieldcraft, SkillCategory::Cyber],
        primary_char: CharacteristicCode::SIG,
        secondary_char: Some(CharacteristicCode::FOC),
        min_chars: &[(CharacteristicCode::FOC, 10), (CharacteristicCode::SIG, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("alertness_situational_awareness", 2),
            ("stealth_sneaking_hiding", 2),
        ],
        allowed_access: &[SkillAccessTier::Restricted, SkillAccessTier::Illicit],
        tool_dependency: ToolDependency::Situational,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptAud),
    },
    SkillNodeSpec {
        id: "forensics_crime_scene_investigation",
        name: "Forensics & Crime Scene Investigation",
        tree: SkillTreeId::Tree7PerceptionStealthSurvival,
        tier: 3,
        skill_type: SkillType::Kno,
        categories: &[
            SkillCategory::Science,
            SkillCategory::Fieldcraft,
            SkillCategory::Medical,
        ],
        primary_char: CharacteristicCode::ANA,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[(CharacteristicCode::ANA, 10), (CharacteristicCode::SIG, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("alertness_situational_awareness", 2),
            ("general_chemistry", 1),
            ("anatomy_physiology", 1),
        ],
        allowed_access: &[SkillAccessTier::Academic, SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    // =========================================================================
    // TREE 8: VISUAL, MUSICAL & CULINARY ARTS
    // =========================================================================
    SkillNodeSpec {
        id: "singing_vocal_rhythm",
        name: "Singing & Vocal Rhythm",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Artistic, SkillCategory::Social],
        primary_char: CharacteristicCode::HEA,
        secondary_char: Some(CharacteristicCode::ELO),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptAud),
    },
    SkillNodeSpec {
        id: "sketching_doodling",
        name: "Sketching & Doodling",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Artistic],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::Situational,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "basic_cooking",
        name: "Basic Cooking",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 0,
        skill_type: SkillType::Tal,
        categories: &[SkillCategory::Artistic, SkillCategory::Fieldcraft],
        primary_char: CharacteristicCode::OLF,
        secondary_char: Some(CharacteristicCode::TAS),
        min_chars: &[],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[],
        allowed_access: &[SkillAccessTier::Universal],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
    SkillNodeSpec {
        id: "drawing_painting_illustration",
        name: "Drawing, Painting & Illustration",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 1,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Artistic],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::DEX, 8), (CharacteristicCode::SIG, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("sketching_doodling", 2)],
        allowed_access: &[SkillAccessTier::Universal, SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "sculpting_ceramics_carving",
        name: "Sculpting, Ceramics & Carving",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 1,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Artistic, SkillCategory::Technical],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::TAC),
        min_chars: &[(CharacteristicCode::DEX, 8), (CharacteristicCode::TAC, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("sketching_doodling", 1), ("manual_dexterity_tinkering", 1)],
        allowed_access: &[
            SkillAccessTier::Universal,
            SkillAccessTier::VocationalLicensed,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "photography_cinematography_digital_media",
        name: "Photography, Cinematography & Digital Media",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 2,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Artistic, SkillCategory::Technical],
        primary_char: CharacteristicCode::SIG,
        secondary_char: Some(CharacteristicCode::CRE),
        min_chars: &[(CharacteristicCode::SIG, 9), (CharacteristicCode::CRE, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("sketching_doodling", 1), ("digital_ai_literacy", 1)],
        allowed_access: &[SkillAccessTier::Universal, SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "forgery_counterfeiting_document_alteration",
        name: "Forgery, Counterfeiting & Document Alteration",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 3,
        skill_type: SkillType::PhyKno,
        categories: &[
            SkillCategory::Artistic,
            SkillCategory::Technical,
            SkillCategory::Fieldcraft,
        ],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::SIG),
        min_chars: &[
            (CharacteristicCode::DEX, 11),
            (CharacteristicCode::SIG, 11),
            (CharacteristicCode::FOC, 10),
        ],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[
            ("drawing_painting_illustration", 2),
            ("writing_composition", 2),
        ],
        allowed_access: &[SkillAccessTier::Illicit, SkillAccessTier::Restricted],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptFin),
    },
    SkillNodeSpec {
        id: "musical_instrument_specific",
        name: "Musical Instrument (Specific)",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 1,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Artistic],
        primary_char: CharacteristicCode::DEX,
        secondary_char: Some(CharacteristicCode::HEA),
        min_chars: &[(CharacteristicCode::DEX, 8), (CharacteristicCode::HEA, 8)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("singing_vocal_rhythm", 1)],
        allowed_access: &[SkillAccessTier::Universal, SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptAud),
    },
    SkillNodeSpec {
        id: "music_theory_composition_sound_engineering",
        name: "Music Theory, Composition & Sound Engineering",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 2,
        skill_type: SkillType::Kno,
        categories: &[SkillCategory::Artistic, SkillCategory::Knowledge],
        primary_char: CharacteristicCode::CRE,
        secondary_char: Some(CharacteristicCode::HEA),
        min_chars: &[(CharacteristicCode::HEA, 10), (CharacteristicCode::CRE, 10)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("musical_instrument_specific", 2)],
        allowed_access: &[SkillAccessTier::Academic],
        tool_dependency: ToolDependency::Situational,
        defaultable: false,
        aptitude_override: Some(AptitudeCode::AptAud),
    },
    SkillNodeSpec {
        id: "dancing_choreography",
        name: "Dancing & Choreography",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 1,
        skill_type: SkillType::Phy,
        categories: &[
            SkillCategory::Artistic,
            SkillCategory::Athletic,
            SkillCategory::Social,
        ],
        primary_char: CharacteristicCode::KIN,
        secondary_char: Some(CharacteristicCode::HEA),
        min_chars: &[(CharacteristicCode::KIN, 8), (CharacteristicCode::HEA, 7)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("balance_footwork", 2), ("singing_vocal_rhythm", 1)],
        allowed_access: &[SkillAccessTier::Universal, SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptKin),
    },
    SkillNodeSpec {
        id: "acting_stagecraft_mimicry",
        name: "Acting, Stagecraft & Mimicry",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 2,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Artistic, SkillCategory::Social],
        primary_char: CharacteristicCode::PRE,
        secondary_char: Some(CharacteristicCode::ELO),
        min_chars: &[(CharacteristicCode::PRE, 9), (CharacteristicCode::CRE, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("deception_bluff", 2), ("native_language_speech", 2)],
        allowed_access: &[SkillAccessTier::Universal, SkillAccessTier::Academic],
        tool_dependency: ToolDependency::None,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptSoc),
    },
    SkillNodeSpec {
        id: "culinary_arts_gastronomy_brewing",
        name: "Culinary Arts, Gastronomy & Brewing",
        tree: SkillTreeId::Tree8VisualMusicalCulinaryArts,
        tier: 2,
        skill_type: SkillType::PhyKno,
        categories: &[SkillCategory::Artistic, SkillCategory::Science],
        primary_char: CharacteristicCode::TAS,
        secondary_char: Some(CharacteristicCode::OLF),
        min_chars: &[(CharacteristicCode::TAS, 9), (CharacteristicCode::OLF, 9)],
        parent_mode: PrerequisiteMode::AllOf,
        parent_prereqs: &[("basic_cooking", 2)],
        allowed_access: &[
            SkillAccessTier::VocationalLicensed,
            SkillAccessTier::Universal,
        ],
        tool_dependency: ToolDependency::Mandatory,
        defaultable: true,
        aptitude_override: Some(AptitudeCode::AptBio),
    },
];

/// Look up a canonical skill specification by its `id` (or case-insensitive `name`).
#[must_use]
pub fn lookup_skill_node(id_or_name: &str) -> Option<&'static SkillNodeSpec> {
    let trimmed = id_or_name.trim();
    CANONICAL_SKILL_TREES.iter().find(|node| {
        node.id.eq_ignore_ascii_case(trimmed) || node.name.eq_ignore_ascii_case(trimmed)
    })
}

/// Return all canonical skill nodes belonging to a specific `SkillTreeId`.
#[must_use]
pub fn skills_in_tree(tree: SkillTreeId) -> Vec<&'static SkillNodeSpec> {
    CANONICAL_SKILL_TREES
        .iter()
        .filter(|node| node.tree == tree)
        .collect()
}

/// Initialize the Universal Childhood Talent (`TAL`) baseline (§2.1 of `skills_and_trees.md`).
///
/// - `native_language_speech` starts at $N = 2$.
/// - All 15 other `TAL` nodes across the 8 trees start at $N = 1$ and never decay below $1$.
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

/// Specific reason why a skill unlock or advancement was rejected by the DAG validator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillValidationFailure {
    pub code: &'static str,
    pub detail: String,
}

/// Result of validating whether a character can unlock or advance a skill node to `target_level`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillAdvancementValidation {
    pub skill_id: String,
    pub skill_name: String,
    pub target_level: usize,
    pub is_valid: bool,
    pub effective_controller_c: i32,
    pub aptitude_min_c_relief: i32,
    pub failures: Vec<SkillValidationFailure>,
}

/// Validate whether a character meets all 4 canonical gates to learn or advance `skill_id` to `target_level` (§2.1, §3.1):
/// 1. **Academic Tree Freeze / `[Access]` Gate**: Character must hold at least one of the node's `allowed_access` tags
///    (`Universal` is always granted; if `academic_tree_frozen` is true, `Academic`-only nodes cannot be advanced above current level).
/// 2. **Minimum Characteristic (`Min C`) Threshold**: Checked against character's `1–20` scores, reduced by `-1` if the
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

/// Result of resolving a Skill Tree Default when a character has $N = 0$ in a target skill (§3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDefaultResolution {
    pub target_skill_id: String,
    pub can_attempt: bool,
    pub source_skill_id: Option<String>,
    pub effective_n: usize,
    pub default_penalty_delta_ext: i32,
    pub reason: String,
}

/// Resolve whether a character can attempt `target_skill_id` directly or by defaulting down-tree / across siblings (§3.1).
///
/// - If the character already has $N \ge 1$ in `target_skill_id`, returns `effective_n = N` and `default_penalty_delta_ext = 0`.
/// - If $N = 0$ and `spec.defaultable == false` (`[No Default]`), the task is **Gated** (`can_attempt = false`).
/// - Otherwise searches immediate parent (`+4` $\Delta_{\text{ext}}$), grandparent (`+8` $\Delta_{\text{ext}}$), or trained sibling
///   sharing a parent in the same tree (`+4` $\Delta_{\text{ext}}$).
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
        if p_n >= 1 && best_parent.map_or(true, |(_, best_n)| p_n > best_n) {
            best_parent = Some((parent_id, p_n));
        }
    }
    if let Some((parent_id, parent_n)) = best_parent {
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

    // 2. Check trained siblings sharing a parent in the same tree (+4 Delta_ext)
    for node in CANONICAL_SKILL_TREES {
        if node.tree == spec.tree && node.id != spec.id {
            let sib_n = current_skills.get(node.id).copied().unwrap_or(0);
            if sib_n >= 1 {
                let shares_parent = node
                    .parent_prereqs
                    .iter()
                    .any(|(p1, _)| spec.parent_prereqs.iter().any(|(p2, _)| p1 == p2));
                if shares_parent {
                    return SkillDefaultResolution {
                        target_skill_id: spec.id.to_string(),
                        can_attempt: true,
                        source_skill_id: Some(node.id.to_string()),
                        effective_n: sib_n,
                        default_penalty_delta_ext: 4,
                        reason: format!(
                            "Sibling default from '{}' (N={sib_n}) with +4 Delta_ext.",
                            node.id
                        ),
                    };
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universal_childhood_talents_and_8_trees() {
        let talents = initialize_universal_talents();
        assert_eq!(talents.get("native_language_speech"), Some(&2));
        assert_eq!(talents.get("counting_basic_numeracy"), Some(&1));
        assert_eq!(talents.get("manual_dexterity_tinkering"), Some(&1));
        assert_eq!(talents.get("hygiene_basic_self_care"), Some(&1));
        assert_eq!(talents.get("brawling_unarmed_striking"), Some(&1));
        assert_eq!(talents.get("alertness_situational_awareness"), Some(&1));
        assert_eq!(talents.len(), 20);
    }

    #[test]
    fn test_foundation_ceiling_and_prereq_validation() {
        let mut skills = initialize_universal_talents();
        skills.insert("pistols_handguns".to_string(), 2);

        let mut chars = CharacterCharacteristics::default();
        chars.for_stat = 11;
        chars.dex = 13;
        let aptitudes = CharacterAptitudes::default();
        let access = vec![SkillAccessTier::Illicit];

        // Advancing Fully Automatic Weapons to N = 4 fits Foundation Ceiling (parent Pistols N = 2 -> max 2 + 2 = 4)
        let ok_res = validate_skill_advancement(
            "fully_automatic_burst_weapons",
            4,
            &skills,
            &chars,
            &aptitudes,
            &access,
            false,
        );
        assert!(ok_res.is_valid);

        // Attempting N = 5 violates Foundation Ceiling (5 > 2 + 2)
        let ceil_res = validate_skill_advancement(
            "fully_automatic_burst_weapons",
            5,
            &skills,
            &chars,
            &aptitudes,
            &access,
            false,
        );
        assert!(!ceil_res.is_valid);
        assert!(
            ceil_res
                .failures
                .iter()
                .any(|f| f.code == "FOUNDATION_CEILING_EXCEEDED")
        );
    }

    #[test]
    fn test_tree_defaulting_and_no_default_block() {
        let mut skills = initialize_universal_talents();
        skills.insert("pistols_handguns".to_string(), 2);

        // Defaulting from Pistols (N=2) to Fully Automatic Weapons (N=0, defaultable=true) -> +4 Delta_ext
        let def = resolve_skill_tree_default("fully_automatic_burst_weapons", &skills);
        assert!(def.can_attempt);
        assert_eq!(def.effective_n, 2);
        assert_eq!(def.default_penalty_delta_ext, 4);

        // Trauma Surgery is [No Default] -> cannot be attempted at N=0
        let surg = resolve_skill_tree_default("trauma_orthopedic_general_surgery", &skills);
        assert!(!surg.can_attempt);
    }
}
