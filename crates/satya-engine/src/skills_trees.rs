//! Canonical 8 Hierarchical Skill Trees Catalog & DAG Validator
//! (`docs/ttrpg/03-character/skills_and_trees.md` §1.1–§1.5, §2.1–§2.4, §3.1–§3.9;
//! `docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7.2–§1.7.3).
//!
//! Implements:
//! - `SkillTreeId` (Trees 1–8), `SkillType` (`TAL`, `PHY`, `KNO`, `PHY/KNO`), multi-tag `SkillCategory`,
//!   and `SkillAccessTier` (`Universal`, `Academic`, `VocationalLicensed`, `Restricted`, `Illicit`) (`docs/ttrpg/03-character/skills_and_trees.md` §1.1–§1.5)
//! - Universal Childhood Baseline initializer (`Native Language Speech` at $N = 2$; all other `TAL` skills at $N = 1$) (`docs/ttrpg/03-character/skills_and_trees.md` §2.1, §3.1)
//! - Complete 8-Tree DAG Registry covering all canonical nodes across Trees 1–8 (`docs/ttrpg/03-character/skills_and_trees.md` §3.2–§3.9)
//! - Prerequisite & Advancement Validator enforcing:
//!   1. `[Access]` Tag Gates (`§1.5`)
//!   2. Minimum Characteristic (`Min C`) Thresholds (with `-1` relief when linked Domain Aptitude $\ge +2$) (`§2.1`)
//!   3. Parent Skill Level Thresholds ($N_{\text{parent}} \ge X$, supporting both AND and OR parent groups) (`§2.2`)
//!   4. **Foundation Ceiling Rule**: $N_{\text{child}} \le N_{\text{parent}} + 2$ (`§2.3`)
//! - Tree & Sibling Defaulting Resolver (`+4` $\Delta_{\text{ext}}$ per tier step down-tree, `+4` sibling default, or `[No Default]` block) (`docs/ttrpg/03-character/skills_and_trees.md` §2.4)
//! - 5-Factor Skill Point (`SP`) Step Cost Calculator (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7.2–§1.7.3)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::characteristics::{
    AptitudeCode, CharacterAptitudes, CharacterCharacteristics, CharacteristicCode,
    compute_aptitude_min_c_relief, compute_aptitude_training_time_multiplier,
};
use crate::define_coded_enum;
use crate::task_resolution::ToolDependency;

define_coded_enum! {
    /// The 8 canonical Hierarchical Skill Trees (`docs/ttrpg/03-character/skills_and_trees.md` §3.2–§3.9).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum SkillTreeId [count = 8] {
        /// Tree 1: Mathematics, Computing & Physical Sciences (`APT-QNT`, `APT-CMP`) (`§3.2`)
        Tree1MathCompPhysicalSciences => { code: "TREE_1", label: "Tree 1: Mathematics, Computing & Physical Sciences", aliases: ["1"] },
        /// Tree 2: Mechanical, Craft & Pilotage (`APT-MEC`, `APT-FIN`) (`§3.3`)
        Tree2MechanicalCraftPilotage => { code: "TREE_2", label: "Tree 2: Mechanical, Craft & Pilotage", aliases: ["2"] },
        /// Tree 3: Biology, Chemistry & Medicine (`APT-BIO`) (`§3.4`)
        Tree3BiologyChemistryMedicine => { code: "TREE_3", label: "Tree 3: Biology, Chemistry & Medicine", aliases: ["3"] },
        /// Tree 4: Language, Humanities & Social Influence (`APT-LNG`, `APT-SOC`) (`§3.8`)
        Tree4LanguageHumanitiesSocial => { code: "TREE_4", label: "Tree 4: Language, Humanities & Social Influence", aliases: ["4"] },
        /// Tree 5: Athletics, Acrobatics & Melee Combat (`APT-KIN`) (`§3.5`)
        Tree5AthleticsAcrobaticsMelee => { code: "TREE_5", label: "Tree 5: Athletics, Acrobatics & Melee Combat", aliases: ["5"] },
        /// Tree 6: Marksmanship, Gunnery & Tactical Systems (`APT-FIN`) (`§3.6`)
        Tree6MarksmanshipGunneryTactical => { code: "TREE_6", label: "Tree 6: Marksmanship, Gunnery & Tactical Systems", aliases: ["6"] },
        /// Tree 7: Perception, Stealth & Survival (`APT-AUD`, `APT-KIN`) (`§3.9`)
        Tree7PerceptionStealthSurvival => { code: "TREE_7", label: "Tree 7: Perception, Stealth & Survival", aliases: ["7"] },
        /// Tree 8: Visual, Musical & Culinary Arts (`APT-FIN`, `APT-AUD`) (`§3.7`)
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

    /// Canonical tree title from `docs/ttrpg/03-character/skills_and_trees.md` (§3.2–§3.9).
    #[must_use]
    pub const fn title(self) -> &'static str {
        self.label()
    }

    /// Primary Domain Aptitude associated with this Skill Tree (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §9).
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
    /// Skill Type (`TAL`, `PHY`, `KNO`, `PHY/KNO`) (`docs/ttrpg/03-character/skills_and_trees.md` §1.1–§1.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SkillType [count = 4] {
        /// `TAL`: Innate Talent — universal childhood baseline ($N \ge 1$, `Native Language Speech` starts at $N = 2$)
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
    /// Multi-category functional tags assigned to skills (`docs/ttrpg/03-character/skills_and_trees.md` §1.3).
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
    /// (`docs/ttrpg/03-character/skills_and_trees.md` §1.5, §2.1).
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
/// (`docs/ttrpg/03-character/skills_and_trees.md` §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrerequisiteMode {
    /// Every listed `(parent_skill_id, min_level)` must be met (`AND`).
    AllOf,
    /// At least one listed `(parent_skill_id, min_level)` must be met (`OR`).
    AnyOf,
}

/// Canonical specification for a single node in the 8 Hierarchical Skill Trees
/// (`docs/ttrpg/03-character/skills_and_trees.md` §3.2–§3.9).
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
    /// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7.2).
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

/// Complete canonical catalog of skills across all 8 Hierarchical Skill Trees (`docs/ttrpg/03-character/skills_and_trees.md` §3.2–§3.9).
macro_rules! skill_node {
    (
        $tree:ident, $id:literal, $name:literal, ($tier:expr, $diff:expr, $stype:ident),
        cats: [$($cat:ident),*], chars: ($pchar:ident, $schar:expr), min_c: [$(($mc:ident, $mv:expr)),*],
        parents: $pmode:ident [$($prereq:tt)*], access: [$($acc:ident),*],
        tool: $tool:ident, def: $def:expr, apt: $apt:expr
    ) => {
        SkillNodeSpec {
            id: $id,
            name: $name,
            tree: SkillTreeId::$tree,
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
            aptitude_override: $apt,
        }
    };
}

#[allow(unused_imports)]
use CharacteristicCode::*;
#[allow(unused_imports)]
use AptitudeCode::*;

/// Complete static catalog of all canonical skill nodes across the 8 Hierarchical Skill Trees
/// (`docs/ttrpg/03-character/skills_and_trees.md` §3.2–§3.9).
pub const CANONICAL_SKILL_TREES: &[SkillNodeSpec] = &[
    // === Tree1MathCompPhysicalSciences ===
    skill_node!(Tree1MathCompPhysicalSciences, "counting_basic_numeracy", "Counting & Basic Numeracy", (0, 1, Tal), cats: [Knowledge], chars: (MEM, Some(INS)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "arithmetic", "Arithmetic", (1, 1, Kno), cats: [Knowledge, Science], chars: (ANA, Some(MEM)), min_c: [(ANA, 6)], parents: AllOf [("counting_basic_numeracy", 1)], access: [Universal], tool: None, def: true, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "digital_ai_literacy", "Digital & AI Literacy", (1, 1, Kno), cats: [Knowledge, Cyber], chars: (MEM, Some(ANA)), min_c: [], parents: AllOf [("counting_basic_numeracy", 1)], access: [Universal], tool: Situational, def: true, apt: Some(AptCmp)),
    skill_node!(Tree1MathCompPhysicalSciences, "general_physical_science", "General Physical Science", (2, 2, Kno), cats: [Science, Knowledge], chars: (ANA, Some(MEM)), min_c: [(ANA, 7)], parents: AllOf [("arithmetic", 1)], access: [Academic, Universal], tool: None, def: true, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "elementary_algebra_geometry", "Elementary Algebra & Geometry", (2, 2, Kno), cats: [Science, Knowledge], chars: (ANA, None), min_c: [(ANA, 8)], parents: AllOf [("arithmetic", 2)], access: [Academic], tool: None, def: true, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "trigonometry", "Trigonometry", (3, 3, Kno), cats: [Science, Technical], chars: (ANA, None), min_c: [(ANA, 9)], parents: AllOf [("elementary_algebra_geometry", 2)], access: [Academic], tool: None, def: true, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "discrete_math_graph_theory", "Discrete Mathematics & Graph Theory", (3, 3, Kno), cats: [Science, Cyber], chars: (ANA, Some(INS)), min_c: [(ANA, 10)], parents: AllOf [("elementary_algebra_geometry", 2)], access: [Academic], tool: None, def: false, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "calculus_linear_algebra", "Calculus & Linear Algebra", (4, 4, Kno), cats: [Science], chars: (ANA, Some(CRE)), min_c: [(ANA, 11)], parents: AllOf [("trigonometry", 2)], access: [Academic], tool: None, def: false, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "differential_equations_vector_calculus", "Differential Equations & Vector Calculus", (5, 5, Kno), cats: [Science], chars: (ANA, Some(CRE)), min_c: [(ANA, 13)], parents: AllOf [("calculus_linear_algebra", 3)], access: [Academic], tool: None, def: false, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "statistics_probability", "Statistics & Probability", (3, 3, Kno), cats: [Science, Knowledge], chars: (ANA, Some(INS)), min_c: [(ANA, 10)], parents: AllOf [("elementary_algebra_geometry", 2)], access: [Academic], tool: None, def: false, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "programming_scripting", "Programming & Scripting", (2, 3, Kno), cats: [Cyber, Technical], chars: (ANA, Some(CRE)), min_c: [(ANA, 9)], parents: AllOf [("elementary_algebra_geometry", 1), ("digital_ai_literacy", 2)], access: [Academic, VocationalLicensed], tool: Mandatory, def: false, apt: Some(AptCmp)),
    skill_node!(Tree1MathCompPhysicalSciences, "algorithms_systems_architecture", "Algorithms & Systems Architecture", (3, 4, Kno), cats: [Cyber, Science], chars: (ANA, Some(CRE)), min_c: [(ANA, 11)], parents: AllOf [("programming_scripting", 2), ("elementary_algebra_geometry", 2)], access: [Academic, VocationalLicensed], tool: Mandatory, def: false, apt: Some(AptCmp)),
    skill_node!(Tree1MathCompPhysicalSciences, "cybersecurity_network_intrusion", "Cybersecurity & Network Intrusion", (3, 4, Kno), cats: [Cyber, Technical], chars: (ANA, Some(INS)), min_c: [(ANA, 11), (FOC, 10)], parents: AllOf [("programming_scripting", 2)], access: [Academic, Restricted, Illicit], tool: Mandatory, def: false, apt: Some(AptCmp)),
    skill_node!(Tree1MathCompPhysicalSciences, "cryptography_quantum_computing", "Cryptography & Quantum Computing", (5, 5, Kno), cats: [Cyber, Science], chars: (ANA, Some(CRE)), min_c: [(ANA, 14)], parents: AllOf [("algorithms_systems_architecture", 3), ("calculus_linear_algebra", 3)], access: [Academic, Restricted], tool: Mandatory, def: false, apt: Some(AptCmp)),
    skill_node!(Tree1MathCompPhysicalSciences, "machine_learning_ai_engineering", "Machine Learning & AI Engineering", (5, 5, Kno), cats: [Cyber, Science], chars: (ANA, Some(CRE)), min_c: [(ANA, 13)], parents: AllOf [("algorithms_systems_architecture", 2), ("statistics_probability", 2), ("calculus_linear_algebra", 2)], access: [Academic], tool: Mandatory, def: false, apt: Some(AptCmp)),
    skill_node!(Tree1MathCompPhysicalSciences, "classical_mechanics_physics", "Classical Mechanics (Physics)", (4, 3, Kno), cats: [Science], chars: (ANA, None), min_c: [(ANA, 10)], parents: AllOf [("trigonometry", 2)], access: [Academic], tool: None, def: false, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "electromagnetism_optics", "Electromagnetism & Optics", (5, 4, Kno), cats: [Science, Technical], chars: (ANA, None), min_c: [(ANA, 12)], parents: AllOf [("classical_mechanics_physics", 2), ("calculus_linear_algebra", 2)], access: [Academic], tool: None, def: false, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "thermodynamics_fluid_dynamics", "Thermodynamics & Fluid Dynamics", (5, 4, Kno), cats: [Science, Technical], chars: (ANA, None), min_c: [(ANA, 12)], parents: AllOf [("classical_mechanics_physics", 2), ("calculus_linear_algebra", 2)], access: [Academic], tool: None, def: false, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "quantum_relativity_nuclear_physics", "Quantum, Relativity & Nuclear Physics", (6, 5, Kno), cats: [Science], chars: (ANA, Some(CRE)), min_c: [(ANA, 14)], parents: AllOf [("electromagnetism_optics", 3), ("differential_equations_vector_calculus", 3)], access: [Academic, Restricted], tool: None, def: false, apt: Some(AptQnt)),
    skill_node!(Tree1MathCompPhysicalSciences, "astronomy_astrophysics_orbital_mechanics", "Astronomy, Astrophysics & Orbital Mechanics", (5, 5, Kno), cats: [Science, Technical], chars: (ANA, Some(SIG)), min_c: [(ANA, 12)], parents: AllOf [("classical_mechanics_physics", 2), ("calculus_linear_algebra", 2)], access: [Academic], tool: Situational, def: false, apt: Some(AptQnt)),
    // === Tree2MechanicalCraftPilotage ===
    skill_node!(Tree2MechanicalCraftPilotage, "manual_dexterity_tinkering", "Manual Dexterity & Tinkering", (0, 1, Tal), cats: [Technical], chars: (DEX, Some(TAC)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "carpentry_woodworking", "Carpentry & Woodworking", (1, 2, PhyKno), cats: [Technical, Artistic], chars: (DEX, Some(FOR)), min_c: [(DEX, 7)], parents: AllOf [("manual_dexterity_tinkering", 1)], access: [Universal, VocationalLicensed], tool: Mandatory, def: true, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "masonry_construction", "Masonry & Construction", (1, 2, PhyKno), cats: [Technical, Athletic], chars: (FOR, Some(DEX)), min_c: [(FOR, 8)], parents: AllOf [("manual_dexterity_tinkering", 1)], access: [Universal, VocationalLicensed], tool: Mandatory, def: true, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "metalworking_machining_blacksmithing", "Metalworking, Machining & Blacksmithing", (2, 3, PhyKno), cats: [Technical], chars: (DEX, Some(FOR)), min_c: [(DEX, 8), (FOR, 8)], parents: AllOf [("manual_dexterity_tinkering", 2)], access: [VocationalLicensed], tool: Mandatory, def: false, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "lockpicking_physical_intrusion", "Lockpicking & Physical Intrusion", (2, 3, PhyKno), cats: [Technical, Fieldcraft], chars: (DEX, Some(TAC)), min_c: [(DEX, 9), (TAC, 9)], parents: AllOf [("manual_dexterity_tinkering", 2)], access: [VocationalLicensed, Restricted, Illicit], tool: Mandatory, def: false, apt: Some(AptFin)),
    skill_node!(Tree2MechanicalCraftPilotage, "vehicle_hotwiring_transponder_spoofing", "Vehicle Hotwiring & Transponder Spoofing", (2, 2, PhyKno), cats: [Technical, Fieldcraft, Cyber], chars: (DEX, Some(ANA)), min_c: [(DEX, 8), (ANA, 8)], parents: AllOf [("manual_dexterity_tinkering", 2)], access: [Illicit, Restricted], tool: Mandatory, def: false, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "improvised_munitions_street_mods", "Improvised Munitions & Street Mods", (2, 3, PhyKno), cats: [Technical, Combat], chars: (DEX, Some(CRE)), min_c: [(DEX, 8), (CRE, 8)], parents: AllOf [("manual_dexterity_tinkering", 2)], access: [Illicit, Restricted], tool: Mandatory, def: false, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "automotive_small_engine_mechanics", "Automotive & Small Engine Mechanics", (2, 2, PhyKno), cats: [Technical], chars: (DEX, Some(ANA)), min_c: [(DEX, 8), (ANA, 7)], parents: AllOf [("manual_dexterity_tinkering", 2), ("arithmetic", 1)], access: [VocationalLicensed, Universal], tool: Mandatory, def: true, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "electrical_power_systems", "Electrical & Power Systems", (2, 3, PhyKno), cats: [Technical], chars: (ANA, Some(DEX)), min_c: [(ANA, 8), (DEX, 8)], parents: AllOf [("manual_dexterity_tinkering", 2), ("elementary_algebra_geometry", 1)], access: [VocationalLicensed], tool: Mandatory, def: false, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "micro_electronics_avionics_cyberware_tech", "Micro-Electronics, Avionics & Cyberware Tech", (3, 4, PhyKno), cats: [Technical, Cyber], chars: (DEX, Some(ANA)), min_c: [(DEX, 11), (ANA, 10)], parents: AllOf [("electrical_power_systems", 2)], access: [VocationalLicensed, Academic], tool: Mandatory, def: false, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "armourer_gunsmithing", "Armourer & Gunsmithing", (3, 3, PhyKno), cats: [Technical, Combat], chars: (DEX, Some(ANA)), min_c: [(DEX, 9), (ANA, 8)], parents: AllOf [("metalworking_machining_blacksmithing", 2)], access: [VocationalLicensed, Restricted, Illicit], tool: Mandatory, def: false, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "mechanical_structural_engineering", "Mechanical & Structural Engineering", (4, 4, Kno), cats: [Science, Technical], chars: (ANA, Some(CRE)), min_c: [(ANA, 11)], parents: AllOf [("classical_mechanics_physics", 2), ("calculus_linear_algebra", 2)], access: [Academic], tool: Situational, def: false, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "aerospace_naval_robotics_engineering", "Aerospace, Naval & Robotics Engineering", (5, 5, Kno), cats: [Science, Technical], chars: (ANA, Some(CRE)), min_c: [(ANA, 13)], parents: AllOf [("mechanical_structural_engineering", 3), ("thermodynamics_fluid_dynamics", 2)], access: [Academic, Restricted], tool: Mandatory, def: false, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "driving_wheeled_tracked", "Driving (Wheeled & Tracked Ground Vehicles)", (1, 1, Phy), cats: [Technical, Fieldcraft], chars: (DEX, Some(SIG)), min_c: [(DEX, 7), (SIG, 6)], parents: AllOf [("manual_dexterity_tinkering", 1)], access: [Universal], tool: Mandatory, def: true, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "high_speed_pursuit_evasive_driving", "High-Speed Pursuit & Evasive Driving", (2, 3, Phy), cats: [Technical, Fieldcraft], chars: (SPD, Some(DEX)), min_c: [(SPD, 9), (DEX, 9)], parents: AllOf [("driving_wheeled_tracked", 2)], access: [Restricted, Illicit, VocationalLicensed], tool: Mandatory, def: true, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "small_boat_seamanship", "Small Boat & Seamanship", (1, 1, PhyKno), cats: [Technical, Fieldcraft], chars: (KIN, Some(INS)), min_c: [(KIN, 7)], parents: AllOf [("manual_dexterity_tinkering", 1)], access: [Universal, VocationalLicensed], tool: Mandatory, def: true, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "piloting_fixed_wing_rotary", "Piloting (Fixed-Wing Aircraft)", (3, 4, PhyKno), cats: [Technical], chars: (DEX, Some(FOC)), min_c: [(DEX, 10), (SIG, 10), (FOC, 10)], parents: AllOf [("driving_wheeled_tracked", 2), ("trigonometry", 1)], access: [VocationalLicensed, Restricted], tool: Mandatory, def: false, apt: Some(AptMec)),
    skill_node!(Tree2MechanicalCraftPilotage, "piloting_rotary_wing_helicopter", "Piloting (Rotary-Wing / Helicopter & VTOL)", (4, 5, PhyKno), cats: [Technical], chars: (KIN, Some(DEX)), min_c: [(KIN, 12), (DEX, 11), (FOC, 11)], parents: AllOf [("driving_wheeled_tracked", 2), ("trigonometry", 1)], access: [VocationalLicensed, Restricted], tool: Mandatory, def: false, apt: Some(AptKin)),
    skill_node!(Tree2MechanicalCraftPilotage, "spacecraft_orbital_helm", "Spacecraft & Orbital Helm", (4, 5, PhyKno), cats: [Technical, Science], chars: (DEX, Some(ANA)), min_c: [(DEX, 11), (ANA, 11)], parents: AllOf [("piloting_fixed_wing_rotary", 2), ("astronomy_astrophysics_orbital_mechanics", 2)], access: [Restricted], tool: Mandatory, def: false, apt: Some(AptMec)),
    // === Tree3BiologyChemistryMedicine ===
    skill_node!(Tree3BiologyChemistryMedicine, "hygiene_basic_self_care", "Hygiene & Basic Self-Care", (0, 1, Tal), cats: [Medical], chars: (INS, Some(MEM)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "first_aid_bls_cpr", "First Aid & Basic Life Support (BLS / CPR)", (1, 1, PhyKno), cats: [Medical, Fieldcraft], chars: (DEX, Some(FOC)), min_c: [(FOC, 7)], parents: AllOf [("hygiene_basic_self_care", 1)], access: [Universal], tool: Situational, def: true, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "general_biology_natural_history", "General Biology & Natural History", (1, 2, Kno), cats: [Science, Knowledge], chars: (MEM, Some(ANA)), min_c: [(MEM, 8)], parents: AllOf [("hygiene_basic_self_care", 1)], access: [Academic, Universal], tool: None, def: true, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "agriculture_agronomy_pastoralism", "Agriculture, Agronomy & Crop Husbandry", (1, 2, PhyKno), cats: [Science, Technical, Fieldcraft], chars: (INS, Some(FOR)), min_c: [(INS, 7), (FIT, 7)], parents: AnyOf [("foraging_campcraft", 1), ("hygiene_basic_self_care", 1)], access: [Universal, VocationalLicensed, Academic], tool: Situational, def: true, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "herbalism_ethnobotany_natural_remedies", "Herbalism, Ethnobotany & Field Remedies", (2, 2, Kno), cats: [Medical, Fieldcraft, Science], chars: (MEM, Some(OLF)), min_c: [(MEM, 8), (OLF, 8)], parents: AllOf [("foraging_campcraft", 2), ("first_aid_bls_cpr", 1)], access: [Universal, VocationalLicensed], tool: Situational, def: true, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "veterinary_medicine_livestock_care", "Veterinary Medicine & Livestock Care", (2, 3, PhyKno), cats: [Medical, Fieldcraft, Science], chars: (INS, Some(DEX)), min_c: [(INS, 8), (DEX, 8)], parents: AllOf [("animal_handling_husbandry_riding", 1), ("first_aid_bls_cpr", 1)], access: [Universal, VocationalLicensed, Academic], tool: Situational, def: true, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "general_chemistry", "General Chemistry", (2, 3, Kno), cats: [Science], chars: (ANA, Some(MEM)), min_c: [(ANA, 9)], parents: AllOf [("elementary_algebra_geometry", 2)], access: [Academic], tool: Mandatory, def: false, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "clandestine_narcotics_synthesis", "Clandestine Narcotics Synthesis", (2, 3, PhyKno), cats: [Science, Technical], chars: (MEM, Some(DEX)), min_c: [(MEM, 8), (DEX, 8)], parents: AnyOf [("general_chemistry", 1), ("basic_cooking", 2)], access: [Illicit], tool: Mandatory, def: false, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "organic_analytical_chemistry", "Organic & Analytical Chemistry", (3, 4, Kno), cats: [Science], chars: (ANA, Some(MEM)), min_c: [(ANA, 11)], parents: AllOf [("general_chemistry", 2)], access: [Academic], tool: Mandatory, def: false, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "anatomy_physiology", "Anatomy & Physiology", (2, 3, Kno), cats: [Medical, Science], chars: (MEM, Some(ANA)), min_c: [(MEM, 10)], parents: AllOf [("general_biology_natural_history", 2)], access: [Academic, VocationalLicensed], tool: None, def: false, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "paramedic_combat_trauma_care", "Paramedic & Combat Trauma Care", (3, 3, PhyKno), cats: [Medical, Fieldcraft], chars: (DEX, Some(FOC)), min_c: [(DEX, 9), (FOC, 10)], parents: AllOf [("first_aid_bls_cpr", 2), ("anatomy_physiology", 1)], access: [VocationalLicensed, Restricted], tool: Mandatory, def: false, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "nursing_patient_care", "Nursing & Patient Care", (3, 3, PhyKno), cats: [Medical], chars: (INS, Some(MEM)), min_c: [(MEM, 9), (DIS, 9)], parents: AllOf [("first_aid_bls_cpr", 2), ("anatomy_physiology", 2)], access: [VocationalLicensed, Academic], tool: Mandatory, def: false, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "pharmacology_toxicology", "Pharmacology & Toxicology", (4, 4, Kno), cats: [Medical, Science], chars: (ANA, Some(MEM)), min_c: [(ANA, 11), (MEM, 11)], parents: AllOf [("organic_analytical_chemistry", 2), ("anatomy_physiology", 2)], access: [Academic, Illicit], tool: Mandatory, def: false, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "clinical_diagnosis_internal_medicine", "Clinical Diagnosis & Internal Medicine", (4, 4, Kno), cats: [Medical, Science], chars: (ANA, Some(INS)), min_c: [(ANA, 12), (MEM, 12)], parents: AllOf [("anatomy_physiology", 3), ("pharmacology_toxicology", 2)], access: [Academic], tool: Mandatory, def: false, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "trauma_orthopedic_general_surgery", "Trauma, Orthopedic & General Surgery", (5, 5, PhyKno), cats: [Medical], chars: (DEX, Some(FOC)), min_c: [(DEX, 12), (FOC, 12), (ANA, 11)], parents: AllOf [("clinical_diagnosis_internal_medicine", 2), ("paramedic_combat_trauma_care", 2)], access: [Academic], tool: Mandatory, def: false, apt: Some(AptBio)),
    skill_node!(Tree3BiologyChemistryMedicine, "genetics_biotech_cyber_implantation", "Genetics, Biotech & Cyber-Implantation", (6, 5, PhyKno), cats: [Medical, Science, Cyber], chars: (ANA, Some(DEX)), min_c: [(ANA, 14), (DEX, 13)], parents: AllOf [("trauma_orthopedic_general_surgery", 3), ("organic_analytical_chemistry", 3)], access: [Academic, Restricted], tool: Mandatory, def: false, apt: Some(AptBio)),
    // === Tree4LanguageHumanitiesSocial ===
    skill_node!(Tree4LanguageHumanitiesSocial, "native_language_speech", "Native Language Speech", (0, 1, Tal), cats: [Social, Knowledge], chars: (ELO, Some(MEM)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptLng)),
    skill_node!(Tree4LanguageHumanitiesSocial, "empathy_social_perception", "Empathy & Social Perception", (0, 1, Tal), cats: [Social], chars: (INS, Some(ITU)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "charm_rapport", "Charm & Rapport", (0, 1, Tal), cats: [Social], chars: (PRE, Some(ELO)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "persuasion_fast_talk", "Persuasion & Fast-Talk", (0, 1, Tal), cats: [Social], chars: (ELO, Some(ANA)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "deception_bluff", "Deception & Bluff", (0, 1, Tal), cats: [Social], chars: (ELO, Some(CRE)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "intimidation_coercion", "Intimidation & Coercion", (0, 1, Tal), cats: [Social, Combat], chars: (PRE, Some(DIS)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "reading_comprehension", "Reading (Comprehension)", (1, 1, Kno), cats: [Knowledge], chars: (MEM, Some(ANA)), min_c: [(MEM, 6)], parents: AllOf [("native_language_speech", 2)], access: [Universal], tool: None, def: true, apt: Some(AptLng)),
    skill_node!(Tree4LanguageHumanitiesSocial, "writing_composition", "Writing (Composition)", (1, 1, Kno), cats: [Knowledge, Artistic], chars: (ELO, Some(MEM)), min_c: [(MEM, 6)], parents: AllOf [("reading_comprehension", 1)], access: [Universal], tool: None, def: true, apt: Some(AptLng)),
    skill_node!(Tree4LanguageHumanitiesSocial, "foreign_language_specific", "Foreign Language (Specific)", (1, 2, Kno), cats: [Knowledge, Social], chars: (MEM, Some(ELO)), min_c: [(MEM, 7)], parents: AllOf [("native_language_speech", 2)], access: [Universal, Academic], tool: None, def: false, apt: Some(AptLng)),
    skill_node!(Tree4LanguageHumanitiesSocial, "seduction_honey_trap", "Seduction & Honey-Trap", (2, 2, PhyKno), cats: [Social, Fieldcraft], chars: (COM, Some(PRE)), min_c: [(COM, 9), (PRE, 9)], parents: AllOf [("charm_rapport", 2), ("deception_bluff", 1)], access: [Universal, Restricted, Illicit], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "rhetoric_oratory_debate", "Rhetoric, Oratory & Debate", (2, 2, Kno), cats: [Social, Knowledge], chars: (ELO, Some(PRE)), min_c: [(ELO, 9)], parents: AllOf [("writing_composition", 2), ("persuasion_fast_talk", 2)], access: [Academic, Universal], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "research_archival_science", "Research & Archival Science", (2, 2, Kno), cats: [Knowledge], chars: (ANA, Some(MEM)), min_c: [(ANA, 8)], parents: AllOf [("reading_comprehension", 2), ("writing_composition", 2)], access: [Academic], tool: Situational, def: true, apt: Some(AptLng)),
    skill_node!(Tree4LanguageHumanitiesSocial, "history_political_science", "History & Political Science", (2, 2, Kno), cats: [Knowledge], chars: (MEM, Some(ANA)), min_c: [(MEM, 8)], parents: AllOf [("reading_comprehension", 2)], access: [Academic], tool: None, def: true, apt: Some(AptLng)),
    skill_node!(Tree4LanguageHumanitiesSocial, "philosophy_formal_logic_ethics", "Philosophy, Formal Logic & Ethics", (3, 3, Kno), cats: [Knowledge], chars: (ANA, Some(INS)), min_c: [(ANA, 10)], parents: AllOf [("writing_composition", 2), ("rhetoric_oratory_debate", 1)], access: [Academic], tool: None, def: true, apt: Some(AptLng)),
    skill_node!(Tree4LanguageHumanitiesSocial, "linguistics_cryptanalysis_philology", "Linguistics, Cryptanalysis & Philology", (3, 4, Kno), cats: [Knowledge, Science], chars: (ANA, Some(MEM)), min_c: [(ANA, 11)], parents: AllOf [("reading_comprehension", 3), ("foreign_language_specific", 2)], access: [Academic], tool: None, def: false, apt: Some(AptLng)),
    skill_node!(Tree4LanguageHumanitiesSocial, "psychology_behavioral_analysis_counseling", "Psychology, Behavioral Analysis & Counseling", (3, 3, Kno), cats: [Science, Medical, Social], chars: (INS, Some(ANA)), min_c: [(INS, 10), (ANA, 9)], parents: AllOf [("empathy_social_perception", 2), ("reading_comprehension", 2)], access: [Academic], tool: None, def: false, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "economics_finance_accounting", "Economics, Finance & Accounting", (3, 3, Kno), cats: [Knowledge, Science], chars: (ANA, Some(MEM)), min_c: [(ANA, 10)], parents: AllOf [("statistics_probability", 1), ("reading_comprehension", 2)], access: [Academic, VocationalLicensed], tool: None, def: false, apt: Some(AptQnt)),
    skill_node!(Tree4LanguageHumanitiesSocial, "law_jurisprudence", "Law & Jurisprudence", (4, 4, Kno), cats: [Knowledge, Social], chars: (MEM, Some(ELO)), min_c: [(MEM, 11), (ANA, 10)], parents: AllOf [("rhetoric_oratory_debate", 2), ("history_political_science", 2)], access: [Academic], tool: None, def: false, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "negotiation_diplomacy", "Negotiation & Diplomacy", (3, 3, Kno), cats: [Social], chars: (ELO, Some(INS)), min_c: [(ELO, 10), (INS, 10)], parents: AllOf [("persuasion_fast_talk", 2), ("empathy_social_perception", 2)], access: [Universal, Academic, VocationalLicensed], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "leadership_command", "Leadership & Command", (2, 2, Kno), cats: [Social, Combat], chars: (PRE, Some(DIS)), min_c: [(PRE, 10), (DIS, 9)], parents: AllOf [("persuasion_fast_talk", 2)], access: [Universal, Restricted], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "interrogation_counter_interrogation", "Interrogation & Counter-Interrogation", (3, 3, Kno), cats: [Social, Fieldcraft], chars: (INS, Some(PRE)), min_c: [(INS, 10), (DIS, 10)], parents: AllOf [("intimidation_coercion", 2), ("deception_bluff", 2)], access: [Restricted, Illicit], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "streetwise_underworld_etiquette", "Streetwise & Underworld Etiquette", (2, 2, Kno), cats: [Social, Fieldcraft], chars: (INS, Some(ITU)), min_c: [(INS, 8)], parents: AllOf [("empathy_social_perception", 2), ("deception_bluff", 1)], access: [Illicit, Universal], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree4LanguageHumanitiesSocial, "black_market_smuggling_fencing", "Black-Market Smuggling & Fencing", (2, 2, Kno), cats: [Social, Fieldcraft], chars: (INS, Some(ELO)), min_c: [(INS, 8), (ELO, 8)], parents: AllOf [("streetwise_underworld_etiquette", 1), ("deception_bluff", 1)], access: [Illicit], tool: None, def: true, apt: Some(AptSoc)),
    // === Tree5AthleticsAcrobaticsMelee ===
    skill_node!(Tree5AthleticsAcrobaticsMelee, "running_sprinting", "Running & Sprinting", (0, 1, Tal), cats: [Athletic], chars: (SPD, Some(FIT)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "jumping_leaping", "Jumping & Leaping", (0, 1, Tal), cats: [Athletic], chars: (FOR, Some(SPD)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "balance_footwork", "Balance & Footwork", (0, 1, Tal), cats: [Athletic], chars: (KIN, Some(SPD)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "strength_lifting", "Strength & Lifting", (0, 1, Tal), cats: [Athletic], chars: (FOR, Some(FIT)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "brawling_unarmed_striking", "Brawling (Unarmed Striking)", (0, 1, Tal), cats: [Combat, Athletic], chars: (SPD, Some(FOR)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "climbing_scaling", "Climbing & Scaling", (1, 1, Phy), cats: [Athletic, Fieldcraft], chars: (FOR, Some(KIN)), min_c: [(FOR, 7), (KIN, 7)], parents: AllOf [("balance_footwork", 1), ("strength_lifting", 1)], access: [Universal], tool: Situational, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "swimming_diving", "Swimming & Diving", (1, 1, Phy), cats: [Athletic, Fieldcraft], chars: (FIT, Some(KIN)), min_c: [(FIT, 6)], parents: AllOf [("balance_footwork", 1)], access: [Universal], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "acrobatics_tumbling_parkour", "Acrobatics, Tumbling & Parkour", (2, 2, Phy), cats: [Athletic], chars: (KIN, Some(SPD)), min_c: [(KIN, 10), (SPD, 9)], parents: AllOf [("balance_footwork", 2), ("jumping_leaping", 2)], access: [Universal, VocationalLicensed], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "zero_g_eva_operations", "Zero-G / EVA Operations", (3, 4, PhyKno), cats: [Athletic, Technical], chars: (KIN, Some(FOC)), min_c: [(KIN, 10)], parents: AllOf [("acrobatics_tumbling_parkour", 2)], access: [VocationalLicensed, Restricted], tool: Mandatory, def: false, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "martial_arts_striking", "Martial Arts (Striking — Boxing, Muay Thai, Karate)", (2, 3, Phy), cats: [Combat, Athletic], chars: (SPD, Some(KIN)), min_c: [(SPD, 9), (KIN, 9)], parents: AllOf [("brawling_unarmed_striking", 2), ("balance_footwork", 2)], access: [VocationalLicensed, Restricted, Illicit], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "grappling_wrestling_submissions", "Grappling, Wrestling & Submissions (Judo, BJJ, Sambo)", (2, 3, Phy), cats: [Combat, Athletic], chars: (FOR, Some(KIN)), min_c: [(FOR, 9), (KIN, 9)], parents: AllOf [("brawling_unarmed_striking", 2), ("balance_footwork", 2)], access: [VocationalLicensed, Restricted, Illicit], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "short_blades_knife_fighting", "Short Blades & Knife Fighting", (1, 2, Phy), cats: [Combat], chars: (SPD, Some(DEX)), min_c: [(SPD, 8), (DEX, 8)], parents: AllOf [("brawling_unarmed_striking", 1), ("balance_footwork", 1)], access: [Universal, Restricted, Illicit], tool: Mandatory, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "swords_fencing_long_blades", "Swords, Fencing & Long Blades", (2, 3, Phy), cats: [Combat, Athletic], chars: (SPD, Some(KIN)), min_c: [(SPD, 9), (KIN, 9)], parents: AllOf [("short_blades_knife_fighting", 2), ("balance_footwork", 2)], access: [VocationalLicensed, Restricted], tool: Mandatory, def: true, apt: Some(AptKin)),
    skill_node!(Tree5AthleticsAcrobaticsMelee, "blunt_impact_weapons_polearms", "Blunt / Impact Weapons & Polearms", (2, 2, Phy), cats: [Combat], chars: (FOR, Some(SPD)), min_c: [(FOR, 9), (KIN, 8)], parents: AllOf [("brawling_unarmed_striking", 1), ("balance_footwork", 2)], access: [Universal, Restricted], tool: Mandatory, def: true, apt: Some(AptKin)),
    // === Tree6MarksmanshipGunneryTactical ===
    skill_node!(Tree6MarksmanshipGunneryTactical, "throwing_lobbing", "Throwing & Lobbing", (0, 1, Tal), cats: [Athletic, Combat], chars: (DEX, Some(KIN)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "archery_bows_crossbows", "Archery (Bows & Crossbows)", (1, 2, Phy), cats: [Combat, Athletic, Fieldcraft], chars: (DEX, Some(SIG)), min_c: [(FOR, 8), (DEX, 8)], parents: AllOf [("throwing_lobbing", 1)], access: [Universal], tool: Mandatory, def: true, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "pistols_handguns", "Pistols & Handguns", (1, 2, Phy), cats: [Combat], chars: (DEX, Some(SIG)), min_c: [(DEX, 8), (SIG, 7)], parents: AllOf [("manual_dexterity_tinkering", 1)], access: [Universal, VocationalLicensed, Restricted, Illicit], tool: Mandatory, def: true, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "concealed_draw_street_shooting", "Concealed Draw & Street Shooting", (2, 2, Phy), cats: [Combat], chars: (SPD, Some(DEX)), min_c: [(SPD, 9), (DEX, 8)], parents: AllOf [("pistols_handguns", 2)], access: [Illicit, Restricted, VocationalLicensed], tool: Mandatory, def: true, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "long_guns_rifles_carbines_shotguns", "Long Guns (Rifles, Carbines & Shotguns)", (1, 2, Phy), cats: [Combat, Fieldcraft], chars: (DEX, Some(SIG)), min_c: [(DEX, 7), (SIG, 8)], parents: AllOf [("manual_dexterity_tinkering", 1)], access: [Universal, VocationalLicensed, Restricted, Illicit], tool: Mandatory, def: true, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "tactical_carbine_cqb", "Tactical Carbine & CQB Marksmanship", (2, 3, Phy), cats: [Combat], chars: (SPD, Some(DEX)), min_c: [(SPD, 9), (DEX, 9)], parents: AllOf [("long_guns_rifles_carbines_shotguns", 2)], access: [Restricted, VocationalLicensed], tool: Mandatory, def: true, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "fully_automatic_burst_weapons", "Fully Automatic & Burst Weapons (SMGs, Assault Rifles, LMGs)", (2, 3, Phy), cats: [Combat], chars: (FOR, Some(DEX)), min_c: [(FOR, 10), (DEX, 9)], parents: AnyOf [("pistols_handguns", 2), ("long_guns_rifles_carbines_shotguns", 2)], access: [Restricted, Illicit], tool: Mandatory, def: true, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "precision_marksmanship_sniping", "Precision Marksmanship & Sniping", (3, 4, PhyKno), cats: [Combat, Fieldcraft], chars: (DEX, Some(FOC)), min_c: [(SIG, 12), (FOC, 12), (DEX, 10)], parents: AllOf [("long_guns_rifles_carbines_shotguns", 3), ("trigonometry", 1)], access: [Restricted], tool: Mandatory, def: false, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "heavy_weapons_mounted_gunnery_atgms", "Heavy Weapons, Mounted Gunnery & ATGMs", (3, 3, PhyKno), cats: [Combat, Technical], chars: (DEX, Some(SIG)), min_c: [(FOR, 10), (SIG, 9)], parents: AllOf [("long_guns_rifles_carbines_shotguns", 2)], access: [Restricted], tool: Mandatory, def: false, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "indirect_fire_mortars_artillery", "Indirect Fire, Mortars & Artillery", (3, 3, Kno), cats: [Combat, Technical], chars: (ANA, Some(SIG)), min_c: [(ANA, 10)], parents: AllOf [("trigonometry", 2), ("heavy_weapons_mounted_gunnery_atgms", 1)], access: [Restricted], tool: Mandatory, def: false, apt: Some(AptQnt)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "demolitions_explosive_ordnance_eod", "Demolitions & Explosive Ordnance (EOD)", (3, 4, PhyKno), cats: [Technical, Combat], chars: (DEX, Some(FOC)), min_c: [(DEX, 11), (FOC, 11), (ANA, 9)], parents: AllOf [("general_chemistry", 1), ("electrical_power_systems", 1)], access: [Restricted, Illicit], tool: Mandatory, def: false, apt: Some(AptFin)),
    skill_node!(Tree6MarksmanshipGunneryTactical, "small_unit_tactics_cqb_doctrine", "Small-Unit Tactics & CQB Doctrine", (2, 3, Kno), cats: [Combat, Knowledge], chars: (INS, Some(ANA)), min_c: [(INS, 9), (FOC, 9)], parents: AllOf [("long_guns_rifles_carbines_shotguns", 2), ("leadership_command", 1)], access: [Restricted], tool: None, def: true, apt: Some(AptSoc)),
    // === Tree7PerceptionStealthSurvival ===
    skill_node!(Tree7PerceptionStealthSurvival, "alertness_situational_awareness", "Alertness (Visual & Auditory Situational Awareness)", (0, 1, Tal), cats: [Fieldcraft], chars: (SIG, Some(HEA)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptAud)),
    skill_node!(Tree7PerceptionStealthSurvival, "stealth_sneaking_hiding", "Stealth (Sneaking & Hiding)", (0, 1, Tal), cats: [Fieldcraft, Athletic], chars: (KIN, Some(ITU)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree7PerceptionStealthSurvival, "foraging_campcraft", "Foraging & Campcraft", (0, 1, Tal), cats: [Fieldcraft], chars: (INS, Some(OLF)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptBio)),
    skill_node!(Tree7PerceptionStealthSurvival, "camouflage_concealment", "Camouflage & Concealment", (1, 2, PhyKno), cats: [Fieldcraft], chars: (INS, Some(CRE)), min_c: [(INS, 8)], parents: AllOf [("stealth_sneaking_hiding", 2)], access: [Universal, Restricted], tool: Situational, def: true, apt: Some(AptKin)),
    skill_node!(Tree7PerceptionStealthSurvival, "sleight_of_hand_pickpocketing_palming", "Sleight of Hand, Pickpocketing & Palming", (2, 3, Phy), cats: [Fieldcraft], chars: (DEX, Some(TAC)), min_c: [(DEX, 10), (TAC, 9)], parents: AllOf [("stealth_sneaking_hiding", 2), ("manual_dexterity_tinkering", 2)], access: [Illicit, Universal], tool: None, def: true, apt: Some(AptFin)),
    skill_node!(Tree7PerceptionStealthSurvival, "disguise_impersonation", "Disguise & Impersonation", (2, 3, PhyKno), cats: [Social, Fieldcraft, Artistic], chars: (CRE, Some(INS)), min_c: [(CRE, 9), (INS, 9)], parents: AllOf [("deception_bluff", 2), ("camouflage_concealment", 1)], access: [Restricted, Illicit, VocationalLicensed], tool: Mandatory, def: true, apt: Some(AptSoc)),
    skill_node!(Tree7PerceptionStealthSurvival, "tracking_spoor_analysis", "Tracking & Spoor Analysis", (1, 2, Kno), cats: [Fieldcraft], chars: (SIG, Some(INS)), min_c: [(SIG, 9), (INS, 8)], parents: AllOf [("alertness_situational_awareness", 2), ("foraging_campcraft", 1)], access: [Universal], tool: None, def: true, apt: Some(AptAud)),
    skill_node!(Tree7PerceptionStealthSurvival, "hunting_trapping_field_butchery", "Hunting, Trapping & Field Butchery", (1, 2, PhyKno), cats: [Fieldcraft, Combat], chars: (SIG, Some(INS)), min_c: [(SIG, 8), (DIS, 7)], parents: AllOf [("foraging_campcraft", 1), ("stealth_sneaking_hiding", 1)], access: [Universal, VocationalLicensed], tool: Mandatory, def: true, apt: Some(AptBio)),
    skill_node!(Tree7PerceptionStealthSurvival, "animal_handling_husbandry_riding", "Animal Handling, Husbandry & Riding", (1, 2, PhyKno), cats: [Fieldcraft, Athletic, Social], chars: (INS, Some(KIN)), min_c: [(INS, 7), (KIN, 7)], parents: AnyOf [("empathy_social_perception", 1), ("foraging_campcraft", 1)], access: [Universal, VocationalLicensed], tool: Situational, def: true, apt: Some(AptBio)),
    skill_node!(Tree7PerceptionStealthSurvival, "wilderness_survival_biome_specific", "Wilderness Survival (Biome-Specific)", (2, 2, Kno), cats: [Fieldcraft], chars: (INS, Some(HLT)), min_c: [(DIS, 8)], parents: AllOf [("foraging_campcraft", 2), ("general_biology_natural_history", 1)], access: [Universal], tool: Situational, def: true, apt: Some(AptBio)),
    skill_node!(Tree7PerceptionStealthSurvival, "land_navigation_cartography", "Land Navigation & Cartography", (2, 2, Kno), cats: [Fieldcraft, Technical], chars: (ANA, Some(SIG)), min_c: [(ANA, 8)], parents: AllOf [("alertness_situational_awareness", 1), ("elementary_algebra_geometry", 1)], access: [Universal, Academic, Restricted], tool: Situational, def: true, apt: Some(AptQnt)),
    skill_node!(Tree7PerceptionStealthSurvival, "surveillance_reconnaissance_countersurveillance", "Surveillance, Reconnaissance & Countersurveillance", (2, 3, PhyKno), cats: [Fieldcraft, Cyber], chars: (SIG, Some(FOC)), min_c: [(FOC, 10), (SIG, 9)], parents: AllOf [("alertness_situational_awareness", 2), ("stealth_sneaking_hiding", 2)], access: [Restricted, Illicit], tool: Situational, def: true, apt: Some(AptAud)),
    skill_node!(Tree7PerceptionStealthSurvival, "forensics_crime_scene_investigation", "Forensics & Crime Scene Investigation", (3, 4, Kno), cats: [Science, Fieldcraft, Medical], chars: (ANA, Some(SIG)), min_c: [(ANA, 10), (SIG, 10)], parents: AllOf [("alertness_situational_awareness", 2), ("general_chemistry", 1), ("anatomy_physiology", 1)], access: [Academic, Restricted], tool: Mandatory, def: false, apt: Some(AptBio)),
    // === Tree8VisualMusicalCulinaryArts ===
    skill_node!(Tree8VisualMusicalCulinaryArts, "singing_vocal_rhythm", "Singing & Vocal Rhythm", (0, 1, Tal), cats: [Artistic, Social], chars: (HEA, Some(ELO)), min_c: [], parents: AllOf [], access: [Universal], tool: None, def: true, apt: Some(AptAud)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "sketching_doodling", "Sketching & Doodling", (0, 1, Tal), cats: [Artistic], chars: (DEX, Some(SIG)), min_c: [], parents: AllOf [], access: [Universal], tool: Situational, def: true, apt: Some(AptFin)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "basic_cooking", "Basic Cooking", (0, 1, Tal), cats: [Artistic, Fieldcraft], chars: (OLF, Some(TAS)), min_c: [], parents: AllOf [], access: [Universal], tool: Mandatory, def: true, apt: Some(AptBio)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "drawing_painting_illustration", "Drawing, Painting & Illustration", (1, 2, PhyKno), cats: [Artistic], chars: (DEX, Some(CRE)), min_c: [(DEX, 8), (SIG, 8)], parents: AllOf [("sketching_doodling", 2)], access: [Universal, Academic], tool: Mandatory, def: true, apt: Some(AptFin)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "sculpting_ceramics_carving", "Sculpting, Ceramics & Carving", (1, 2, PhyKno), cats: [Artistic, Technical], chars: (DEX, Some(TAC)), min_c: [(DEX, 8), (TAC, 8)], parents: AllOf [("sketching_doodling", 1), ("manual_dexterity_tinkering", 1)], access: [Universal, VocationalLicensed], tool: Mandatory, def: true, apt: Some(AptFin)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "photography_cinematography_digital_media", "Photography, Cinematography & Digital Media", (2, 2, PhyKno), cats: [Artistic, Technical], chars: (SIG, Some(CRE)), min_c: [(SIG, 9), (CRE, 8)], parents: AllOf [("sketching_doodling", 1), ("digital_ai_literacy", 1)], access: [Universal, Academic], tool: Mandatory, def: true, apt: Some(AptFin)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "forgery_counterfeiting_document_alteration", "Forgery, Counterfeiting & Document Alteration", (3, 4, PhyKno), cats: [Artistic, Technical, Fieldcraft], chars: (DEX, Some(SIG)), min_c: [(DEX, 11), (SIG, 11), (FOC, 10)], parents: AllOf [("drawing_painting_illustration", 2), ("writing_composition", 2)], access: [Illicit, Restricted], tool: Mandatory, def: false, apt: Some(AptFin)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "musical_instrument_specific", "Musical Instrument (Specific)", (1, 2, PhyKno), cats: [Artistic], chars: (DEX, Some(HEA)), min_c: [(DEX, 8), (HEA, 8)], parents: AllOf [("singing_vocal_rhythm", 1)], access: [Universal, Academic], tool: Mandatory, def: false, apt: Some(AptAud)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "music_theory_composition_sound_engineering", "Music Theory, Composition & Sound Engineering", (2, 3, Kno), cats: [Artistic, Knowledge], chars: (CRE, Some(HEA)), min_c: [(HEA, 10), (CRE, 10)], parents: AllOf [("musical_instrument_specific", 2)], access: [Academic], tool: Situational, def: false, apt: Some(AptAud)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "dancing_choreography", "Dancing & Choreography", (1, 1, Phy), cats: [Artistic, Athletic, Social], chars: (KIN, Some(HEA)), min_c: [(KIN, 8), (HEA, 7)], parents: AllOf [("balance_footwork", 2), ("singing_vocal_rhythm", 1)], access: [Universal, Academic], tool: None, def: true, apt: Some(AptKin)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "acting_stagecraft_mimicry", "Acting, Stagecraft & Mimicry", (2, 2, PhyKno), cats: [Artistic, Social], chars: (PRE, Some(ELO)), min_c: [(PRE, 9), (CRE, 9)], parents: AllOf [("deception_bluff", 2), ("native_language_speech", 2)], access: [Universal, Academic], tool: None, def: true, apt: Some(AptSoc)),
    skill_node!(Tree8VisualMusicalCulinaryArts, "culinary_arts_gastronomy_brewing", "Culinary Arts, Gastronomy & Brewing", (2, 2, PhyKno), cats: [Artistic, Science], chars: (TAS, Some(OLF)), min_c: [(TAS, 9), (OLF, 9)], parents: AllOf [("basic_cooking", 2)], access: [VocationalLicensed, Universal], tool: Mandatory, def: true, apt: Some(AptBio)),
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
/// (`docs/ttrpg/03-character/skills_and_trees.md` §1.5, §2.1–§2.3):
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

/// Result of resolving a Skill Tree Default when a character has $N = 0$ in a target skill
/// (`docs/ttrpg/03-character/skills_and_trees.md` §2.4).
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
/// (`docs/ttrpg/03-character/skills_and_trees.md` §2.4).
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
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7.2).
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
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7.2–§1.7.3).
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
    /// Effective controlling characteristic $C_{\text{eff}} \in [1, 20]$ ($\lfloor(C_1 + C_2)/2\rfloor$ composite controller per `characteristics_and_aptitudes.md` §10.1).
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
/// (`docs/ttrpg/03-character/lifepaths_and_economy.md` §1.7.2–§1.7.3):
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
        assert_eq!(talents.get("charm_rapport"), Some(&1));
        assert_eq!(talents.get("brawling_unarmed_striking"), Some(&1));
        assert_eq!(talents.get("alertness_situational_awareness"), Some(&1));
        assert_eq!(talents.len(), 21);
        assert_eq!(CANONICAL_SKILL_TREES.len(), 129);
    }

    #[test]
    fn test_foundation_ceiling_and_prereq_validation() {
        let mut skills = initialize_universal_talents();
        skills.insert("pistols_handguns".to_string(), 2);

        let chars = CharacterCharacteristics {
            for_stat: 11,
            dex: 13,
            ..Default::default()
        };
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

        // Sibling default: Pistols (N=2) shares TAL parent manual_dexterity_tinkering (N=1) with Long Guns -> prefers N=2 sibling!
        let sib_def = resolve_skill_tree_default("long_guns_rifles_carbines_shotguns", &skills);
        assert!(sib_def.can_attempt);
        assert_eq!(
            sib_def.source_skill_id.as_deref(),
            Some("pistols_handguns")
        );
        assert_eq!(sib_def.effective_n, 2);
        assert_eq!(sib_def.default_penalty_delta_ext, 4);

        // Trauma Surgery is [No Default] -> cannot be attempted at N=0
        let surg = resolve_skill_tree_default("trauma_orthopedic_general_surgery", &skills);
        assert!(!surg.can_attempt);
    }

    #[test]
    fn test_skill_point_cost_formula_swimming_vs_helicopter() {
        let avg_chars = CharacterCharacteristics::default();
        let avg_apts = CharacterAptitudes::default();

        // 1. Swimming & Diving (Difficulty 1, PHY) at human average (C=10, APT=0, FOC=10, INS=10)
        let swim_n1_exp = compute_skill_step_sp_cost(
            "swimming_diving",
            1,
            SkillTrainingMode::Experience,
            &avg_chars,
            &avg_apts,
        );
        assert_eq!(swim_n1_exp.difficulty_rating, 1);
        assert_eq!(swim_n1_exp.final_sp_cost, 10);

        let swim_n1_formal = compute_skill_step_sp_cost(
            "swimming_diving",
            1,
            SkillTrainingMode::FormalTraining,
            &avg_chars,
            &avg_apts,
        );
        assert_eq!(swim_n1_formal.final_sp_cost, 9);

        // 2. Piloting (Rotary-Wing / Helicopter & VTOL) (Difficulty 5, PHY/KNO) at human average
        let heli_n1_formal = compute_skill_step_sp_cost(
            "piloting_rotary_wing_helicopter",
            1,
            SkillTrainingMode::FormalTraining,
            &avg_chars,
            &avg_apts,
        );
        let heli_n2_formal = compute_skill_step_sp_cost(
            "piloting_rotary_wing_helicopter",
            2,
            SkillTrainingMode::FormalTraining,
            &avg_chars,
            &avg_apts,
        );
        let heli_n3_formal = compute_skill_step_sp_cost(
            "piloting_rotary_wing_helicopter",
            3,
            SkillTrainingMode::FormalTraining,
            &avg_chars,
            &avg_apts,
        );
        assert_eq!(heli_n1_formal.difficulty_rating, 5);
        assert_eq!(heli_n1_formal.final_sp_cost, 21);
        assert_eq!(heli_n2_formal.final_sp_cost, 43);
        assert_eq!(heli_n3_formal.final_sp_cost, 64);

        // Unassisted experience for Helicopter Piloting is markedly more expensive (31 / 63 / 94 SP)
        let heli_n1_exp = compute_skill_step_sp_cost(
            "piloting_rotary_wing_helicopter",
            1,
            SkillTrainingMode::Experience,
            &avg_chars,
            &avg_apts,
        );
        assert_eq!(heli_n1_exp.final_sp_cost, 31);

        // 3. Gifted aviation student (KIN=15, DEX=14, FOC=14, INS=13, APT_KIN=+2)
        let gifted_chars = CharacterCharacteristics {
            kin: 15,
            dex: 14,
            foc: 14,
            ins: 13,
            ..Default::default()
        };
        let gifted_apts = CharacterAptitudes {
            apt_kin: 2,
            ..Default::default()
        };

        let heli_gifted_n1 = compute_skill_step_sp_cost(
            "piloting_rotary_wing_helicopter",
            1,
            SkillTrainingMode::FormalTraining,
            &gifted_chars,
            &gifted_apts,
        );
        assert!(heli_gifted_n1.final_sp_cost < heli_n1_formal.final_sp_cost);
        assert_eq!(heli_gifted_n1.final_sp_cost, 10);
    }
}
