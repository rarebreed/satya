//! Prompt- & Template-Driven Procedural NPC Generator for Human and AI Game Masters
//! (`docs/ttrpg/03-character/characteristics_and_aptitudes.md`, `lifepaths_and_economy.md`,
//! `docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md`).
//!
//! Converts a natural-language NPC description (e.g., `"Terry is a kind general MD, but needs to feel like she belongs"`)
//! and/or an archetype template (`NpcTemplateKind`) into:
//! 1. A 100% rules-legal [`satya_schema::CharacterSheetRecord`] passing [`crate::verify_character_sheet_legality`],
//!    complete with 30 Characteristics, 7 bipolar Psyche traits (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`),
//!    9 Domain Aptitudes, 5-Stage Lifepath history, and 8-Tree skills.
//! 2. A public player-safe summary (`public_summary`) vs. GM-only secret/psyche dossier (`gm_secret_dossier`).

use std::collections::BTreeMap;

use satya_schema::{CharacterSheetRecord, CharacterVerificationStatus};
use serde::{Deserialize, Serialize};

use crate::character_audit::{
    aptitudes_to_record, characteristics_to_record, verify_character_sheet_legality,
};
use crate::characteristics::{
    CharacterAptitudes, CharacterCharacteristics, CharacteristicCategory, CharacteristicCode,
    CharacteristicRollMethod, PsychePole, psyche_pole_descriptor, roll_aptitude_pool_11_drop_2,
    roll_characteristic_group_pool, roll_characteristic_method1, roll_correlated_height_mass,
};
use crate::define_coded_enum;
use crate::economy_gear::{
    auto_allocate_career_and_elective_skills, compute_skill_allocation_sp_ledger,
    preview_next_skill_step_sp,
};
use crate::lifepath_catalog::{
    CareerTermTrack, HighSchoolTrack, RuralYouthArchetype, Stage4Path, StreetGangArchetype,
};
use crate::lifepath_economy::{
    CareerTermConfig, LifepathBuildConfig, compute_lifepath_baseline_from_config,
};
use crate::skills_trees::{lookup_skill_node, validate_skill_advancement};

define_coded_enum! {
    /// High-level NPC archetype template for prompt- and template-driven NPC generation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum NpcTemplateKind [count = 9] {
        /// Automatically infer career track, origin, and skills from the natural-language description.
        #[default]
        Auto => {
            code: "AUTO",
            label: "Auto-Infer from Description",
            aliases: ["DEFAULT", "INFER", "PROMPT"],
        },
        /// General MD, Field Physician, Surgeon, Paramedic, or Clinical Researcher.
        MedicalDoctor => {
            code: "MEDICAL_DOCTOR",
            label: "General MD / Physician / Surgeon",
            aliases: ["MD", "DOCTOR", "PHYSICIAN", "SURGEON", "MEDIC", "MEDICAL"],
        },
        /// Military Combat Arms, Spec Ops, Bodyguard, or Mercenary Operator.
        CombatSoldier => {
            code: "COMBAT_SOLDIER",
            label: "Combat Soldier / Tactical Operator",
            aliases: ["SOLDIER", "MILITARY", "SPEC_OPS", "MERCENARY", "OPERATOR", "GUARD"],
        },
        /// Law Enforcement Detective, Forensic Investigator, SWAT, or Inspector.
        DetectiveInvestigator => {
            code: "DETECTIVE_INVESTIGATOR",
            label: "Detective / Law Enforcement Investigator",
            aliases: ["DETECTIVE", "INVESTIGATOR", "POLICE", "COP", "SWAT", "INSPECTOR"],
        },
        /// Underworld Fixer, Street Enforcer, Smuggler, or Cartel Operative.
        SyndicateEnforcer => {
            code: "SYNDICATE_ENFORCER",
            label: "Syndicate Enforcer / Underworld Fixer",
            aliases: ["SYNDICATE", "CARTEL", "GANG", "CRIMINAL", "FIXER", "ENFORCER", "SMUGGLER"],
        },
        /// STEM Scientist, Software/AI Engineer, Robotics Specialist, or Academic Researcher.
        StemEngineer => {
            code: "STEM_ENGINEER",
            label: "STEM Scientist / Cyber & Robotics Engineer",
            aliases: ["ENGINEER", "SCIENTIST", "HACKER", "PROGRAMMER", "RESEARCHER", "STEM"],
        },
        /// Clandestine Intelligence Officer, Handler, or Undercover Agent.
        IntelligenceOperative => {
            code: "INTELLIGENCE_OPERATIVE",
            label: "Clandestine Intelligence Operative / Handler",
            aliases: ["SPY", "INTELLIGENCE", "AGENT", "HANDLER", "OPERATIVE", "CLANDESTINE"],
        },
        /// Blue-Collar Mechanic, Industrial Technician, Pilot, or Merchant Marine Crew.
        BlueCollarTech => {
            code: "BLUE_COLLAR_TECH",
            label: "Blue-Collar Technician / Mechanic / Pilot",
            aliases: ["TECH", "TECHNICIAN", "MECHANIC", "PILOT", "TRADES", "BLUE_COLLAR"],
        },
        /// Rural Scout, Frontier Ranger, Tracker, Herbalist, or Outrider.
        FrontierRanger => {
            code: "FRONTIER_RANGER",
            label: "Frontier Ranger / Wilderness Scout & Herbalist",
            aliases: ["RANGER", "SCOUT", "HUNTER", "TRACKER", "RURAL", "HERBALIST"],
        },
    }
}

impl NpcTemplateKind {
    /// Canonical [`CareerTermTrack`] corresponding to this resolved template.
    #[must_use]
    pub const fn primary_career_track(self) -> CareerTermTrack {
        match self {
            Self::Auto | Self::DetectiveInvestigator => {
                CareerTermTrack::LawEnforcementSwatDetective
            }
            Self::MedicalDoctor => CareerTermTrack::MedicalSchoolAndResidency,
            Self::CombatSoldier => CareerTermTrack::MilitaryCombatArmsSpecOps,
            Self::SyndicateEnforcer => CareerTermTrack::OrganizedCrimeCartelSyndicate,
            Self::StemEngineer => CareerTermTrack::UniversityUndergraduateStem,
            Self::IntelligenceOperative => CareerTermTrack::IntelligenceServiceClandestine,
            Self::BlueCollarTech => CareerTermTrack::BlueCollarTradesMerchantMarine,
            Self::FrontierRanger => CareerTermTrack::RuralAgrarianFrontierRanger,
        }
    }

    /// Default public role/callsign title for this template.
    #[must_use]
    pub const fn default_role_title(self) -> &'static str {
        match self {
            Self::Auto => "Civilian Specialist",
            Self::MedicalDoctor => "General MD & Clinical Physician",
            Self::CombatSoldier => "Tactical Security Operator",
            Self::DetectiveInvestigator => "Investigative Detective",
            Self::SyndicateEnforcer => "Underworld Fixer & Enforcer",
            Self::StemEngineer => "Systems & Research Engineer",
            Self::IntelligenceOperative => "Field Intelligence Handler",
            Self::BlueCollarTech => "Lead Mechanical Technician",
            Self::FrontierRanger => "Frontier Guide & Outrider",
        }
    }
}

/// Input parameters for [`generate_npc_from_prompt`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpcGenerationParams {
    /// Free-text description of the NPC (e.g., `"Terry is a kind general MD, but needs to feel like she belongs"`).
    pub prompt: String,
    /// Optional explicit name override (if omitted, extracted from `prompt` or generated deterministically).
    #[serde(default)]
    pub name_override: Option<String>,
    /// Optional explicit template (`NpcTemplateKind::Auto` infers from `prompt`).
    #[serde(default)]
    pub template: NpcTemplateKind,
    /// Number of 4-year adult career terms (`1..=4`; defaults to `2` = Age 26).
    #[serde(default = "default_career_terms")]
    pub career_terms: usize,
    /// Optional deterministic seed (`0` hashes `prompt` via FNV-1a).
    #[serde(default)]
    pub seed: u64,
    /// Owner user ID stamped on the generated `CharacterSheetRecord`.
    #[serde(default = "default_owner_user_id")]
    pub owner_user_id: String,
}

fn default_career_terms() -> usize {
    2
}

fn default_owner_user_id() -> String {
    "gm_npc".to_string()
}

impl Default for NpcGenerationParams {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            name_override: None,
            template: NpcTemplateKind::Auto,
            career_terms: 2,
            seed: 0,
            owner_user_id: "gm_npc".to_string(),
        }
    }
}

/// Breakdown of one of the 7 bipolar Psyche traits (`1..=20`) for an NPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpcPsycheTraitSummary {
    pub code: String,
    pub name: String,
    pub score: i32,
    pub pole: String,
    pub descriptor: String,
}

/// Complete output of [`generate_npc_from_prompt`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedNpcResult {
    /// Full, rules-legal [`CharacterSheetRecord`] ready for session binding or character sheet inspection.
    pub character: CharacterSheetRecord,
    /// Archetype template used (`MedicalDoctor`, `CombatSoldier`, etc.).
    pub inferred_template: NpcTemplateKind,
    /// All 7 bipolar Psyche traits (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`) with pole labels.
    pub psyche_traits: Vec<NpcPsycheTraitSummary>,
    /// Top trained skills `(skill_id, display_name, rank_n)` sorted descending by rank.
    pub top_skills: Vec<(String, String, i32)>,
    /// Player-safe public summary (name, apparent role, demeanor, build — no hidden stats or secrets).
    pub public_summary: String,
    /// GM-only full narrative & mechanical dossier (includes Psyche scores, motivations, top skills, and secrets).
    pub gm_secret_dossier: String,
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn roll_die(&mut self, sides: i32) -> i32 {
        if sides <= 1 {
            return 1;
        }
        ((self.next_u64() % (sides as u64)) as i32) + 1
    }
}

fn fnv1a_hash(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn any_cue(haystack_lower: &str, words: &[&str], cues: &[&str]) -> bool {
    cues.iter().any(|&cue| {
        if cue.contains(' ') {
            haystack_lower.contains(cue)
        } else {
            words.contains(&cue)
        }
    })
}

/// Infer [`NpcTemplateKind`] from a natural-language prompt when `NpcTemplateKind::Auto` is selected.
#[must_use]
pub fn infer_npc_template_from_prompt(prompt: &str) -> NpcTemplateKind {
    let lower = prompt.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();

    const TEMPLATE_CUES: &[(NpcTemplateKind, &[&str])] = &[
        (
            NpcTemplateKind::MedicalDoctor,
            &[
                "md",
                "doctor",
                "physician",
                "surgeon",
                "medic",
                "nurse",
                "paramedic",
                "clinician",
                "psychiatrist",
                "pediatrician",
                "general md",
                "medical",
            ],
        ),
        (
            NpcTemplateKind::DetectiveInvestigator,
            &[
                "detective",
                "investigator",
                "police",
                "cop",
                "swat",
                "inspector",
                "marshal",
                "sheriff",
                "forensic",
            ],
        ),
        (
            NpcTemplateKind::IntelligenceOperative,
            &[
                "spy",
                "intelligence",
                "operative",
                "handler",
                "undercover",
                "clandestine",
                "agent",
                "espionage",
            ],
        ),
        (
            NpcTemplateKind::SyndicateEnforcer,
            &[
                "syndicate",
                "cartel",
                "gang",
                "enforcer",
                "fixer",
                "smuggler",
                "thief",
                "burglar",
                "hitman",
                "mob",
                "criminal",
            ],
        ),
        (
            NpcTemplateKind::StemEngineer,
            &[
                "engineer",
                "scientist",
                "hacker",
                "programmer",
                "researcher",
                "robotics",
                "physicist",
                "chemist",
                "professor",
                "analyst",
            ],
        ),
        (
            NpcTemplateKind::BlueCollarTech,
            &[
                "mechanic",
                "technician",
                "pilot",
                "electrician",
                "machinist",
                "welder",
                "sailor",
                "trucker",
                "driver",
            ],
        ),
        (
            NpcTemplateKind::FrontierRanger,
            &[
                "ranger",
                "scout",
                "hunter",
                "tracker",
                "herbalist",
                "farmer",
                "outrider",
                "survivalist",
                "guide",
            ],
        ),
        (
            NpcTemplateKind::CombatSoldier,
            &[
                "soldier",
                "marine",
                "mercenary",
                "sniper",
                "infantry",
                "veteran",
                "bodyguard",
                "bouncer",
                "commando",
            ],
        ),
    ];

    for &(kind, cues) in TEMPLATE_CUES {
        if any_cue(&lower, &words, cues) {
            return kind;
        }
    }
    NpcTemplateKind::DetectiveInvestigator
}

/// Extract an NPC name from a prompt such as `"Terry is a kind general MD..."` or `"Dr. Priya Nair, a surgeon..."`.
#[must_use]
pub fn extract_npc_name_from_prompt(prompt: &str, seed: u64) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return fallback_npc_name(seed);
    }

    for sep in [
        " is a ", " is an ", " is the ", " — ", " - ", ", a ", ", an ", ": ",
    ] {
        if let Some(idx) = trimmed.find(sep) {
            let candidate = trimmed[..idx].trim().trim_matches(['"', '\'']);
            let words: Vec<&str> = candidate.split_whitespace().collect();
            if !words.is_empty()
                && words.len() <= 4
                && words[0]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase())
                && ![
                    "A", "An", "The", "This", "He", "She", "They", "Roll", "Create", "Generate",
                ]
                .contains(&words[0])
            {
                return candidate.to_string();
            }
        }
    }

    let lower = trimmed.to_ascii_lowercase();
    for marker in ["named ", "called "] {
        if let Some(pos) = lower.find(marker) {
            let after = &trimmed[pos + marker.len()..];
            let end = after.find([',', '.', ';', '(']).unwrap_or(after.len());
            let tokens: Vec<&str> = after[..end]
                .split_whitespace()
                .take_while(|w| {
                    !matches!(
                        w.to_ascii_lowercase().as_str(),
                        "who" | "that" | "with" | "and" | "but" | "is"
                    )
                })
                .take(3)
                .collect();
            if !tokens.is_empty() {
                return tokens.join(" ");
            }
        }
    }

    fallback_npc_name(seed)
}

fn fallback_npc_name(seed: u64) -> String {
    const FIRST: [&str; 12] = [
        "Terry", "Elena", "Marcus", "Sora", "Devon", "Kira", "Julian", "Nadia", "Tomas", "Lin",
        "Rafael", "Miriam",
    ];
    const LAST: [&str; 12] = [
        "Vance",
        "Rostova",
        "Okafor",
        "Chen",
        "Mercer",
        "Kowalski",
        "Alvarez",
        "Haddad",
        "Sato",
        "Novak",
        "Lindqvist",
        "Solis",
    ];
    format!(
        "{} {}",
        FIRST[(seed as usize) % FIRST.len()],
        LAST[((seed >> 8) as usize) % LAST.len()]
    )
}

fn apply_prompt_psyche_and_skill_cues(
    prompt: &str,
    template: NpcTemplateKind,
    chars: &mut CharacterCharacteristics,
) -> Vec<&'static str> {
    let lower = prompt.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let mut priority_skills: Vec<&'static str> = Vec::new();

    match template {
        NpcTemplateKind::MedicalDoctor => {
            chars.slf = 7;
            chars.opn = 13;
            chars.cnf = 12;
        }
        NpcTemplateKind::CombatSoldier => {
            chars.blg = 14;
            chars.cnf = 13;
            chars.agg = 14;
        }
        NpcTemplateKind::DetectiveInvestigator => {
            chars.opn = 13;
            chars.cnf = 12;
            chars.agg = 11;
        }
        NpcTemplateKind::SyndicateEnforcer => {
            chars.slf = 14;
            chars.cnf = 6;
            chars.agg = 15;
        }
        NpcTemplateKind::StemEngineer => {
            chars.opn = 15;
            chars.ext = 8;
        }
        NpcTemplateKind::IntelligenceOperative => {
            chars.blg = 6;
            chars.opn = 13;
            chars.ext = 9;
        }
        NpcTemplateKind::BlueCollarTech => {
            chars.ego = 8;
            chars.blg = 12;
        }
        NpcTemplateKind::FrontierRanger | NpcTemplateKind::Auto => {
            chars.blg = 7;
            chars.opn = 12;
        }
    }

    if any_cue(
        &lower,
        &words,
        &[
            "kind",
            "caring",
            "compassionate",
            "gentle",
            "altruistic",
            "selfless",
            "empathetic",
            "warm",
            "generous",
            "tender",
            "healer",
        ],
    ) {
        chars.slf = 5;
        chars.agg = chars.agg.min(5);
        chars.ego = chars.ego.min(7);
        chars.opn = chars.opn.max(14);
        chars.com = chars.com.max(13);
        priority_skills.extend_from_slice(&[
            "empathy_social_perception",
            "nursing_patient_care",
            "first_aid_bls_cpr",
            "psychology_behavioral_analysis_counseling",
        ]);
    }
    if any_cue(
        &lower,
        &words,
        &[
            "selfish",
            "greedy",
            "mercenary",
            "ruthless",
            "callous",
            "opportunistic",
            "self-serving",
            "cutthroat",
        ],
    ) {
        chars.slf = 16;
        chars.agg = chars.agg.max(15);
    }
    if any_cue(
        &lower,
        &words,
        &[
            "belong",
            "belongs",
            "belonging",
            "fit in",
            "acceptance",
            "accepted",
            "loyal",
            "tribal",
            "devoted",
            "team player",
            "pack",
            "family",
            "needs to feel like",
            "approval",
        ],
    ) {
        chars.blg = 16;
        priority_skills.extend_from_slice(&["empathy_social_perception", "persuasion_fast_talk"]);
    }
    if any_cue(
        &lower,
        &words,
        &[
            "loner",
            "lone wolf",
            "detached",
            "solitary",
            "independent",
            "outcast",
            "drifter",
            "aloof",
        ],
    ) {
        chars.blg = 4;
    }
    if any_cue(
        &lower,
        &words,
        &[
            "arrogant",
            "proud",
            "narcissistic",
            "cocky",
            "vain",
            "egotistical",
            "grandiose",
            "haughty",
        ],
    ) {
        chars.ego = 17;
    }
    if any_cue(
        &lower,
        &words,
        &["humble", "modest", "unassuming", "meek", "self-effacing"],
    ) {
        chars.ego = 5;
    }
    if any_cue(
        &lower,
        &words,
        &[
            "curious",
            "open-minded",
            "imaginative",
            "inquisitive",
            "creative",
            "eccentric",
            "visionary",
        ],
    ) {
        chars.opn = 16;
    }
    if any_cue(
        &lower,
        &words,
        &[
            "dogmatic",
            "rigid",
            "stubborn",
            "traditional",
            "closed-minded",
            "suspicious",
            "cynical",
        ],
    ) {
        chars.opn = 5;
    }
    if any_cue(
        &lower,
        &words,
        &[
            "extroverted",
            "outgoing",
            "charismatic",
            "gregarious",
            "talkative",
            "charming",
            "boisterous",
            "social",
        ],
    ) {
        chars.ext = 16;
        chars.pre = chars.pre.max(13);
        chars.elo = chars.elo.max(13);
    }
    if any_cue(
        &lower,
        &words,
        &[
            "shy",
            "quiet",
            "introverted",
            "reserved",
            "stoic",
            "taciturn",
            "withdrawn",
        ],
    ) {
        chars.ext = 5;
    }
    if any_cue(
        &lower,
        &words,
        &[
            "by-the-book",
            "orthodox",
            "obedient",
            "disciplined",
            "dutiful",
            "strict",
            "conformist",
            "bureaucratic",
        ],
    ) {
        chars.cnf = 16;
    }
    if any_cue(
        &lower,
        &words,
        &[
            "rebel",
            "rebellious",
            "maverick",
            "iconoclast",
            "unconventional",
            "anti-authority",
            "rogue",
        ],
    ) {
        chars.cnf = 4;
    }
    if any_cue(
        &lower,
        &words,
        &[
            "aggressive",
            "violent",
            "fierce",
            "belligerent",
            "hot-headed",
            "brutal",
            "combative",
            "pugnacious",
        ],
    ) {
        chars.agg = 17;
    }
    if any_cue(
        &lower,
        &words,
        &[
            "pacifist",
            "peaceful",
            "nonviolent",
            "calm",
            "conflict-averse",
            "timid",
        ],
    ) {
        chars.agg = 4;
    }

    if any_cue(
        &lower,
        &words,
        &[
            "general md",
            "md",
            "physician",
            "doctor",
            "diagnosis",
            "clinic",
        ],
    ) {
        priority_skills.extend_from_slice(&[
            "clinical_diagnosis_internal_medicine",
            "first_aid_bls_cpr",
            "nursing_patient_care",
            "pharmacology_toxicology",
            "anatomy_physiology",
        ]);
    }
    if any_cue(
        &lower,
        &words,
        &["surgeon", "surgery", "trauma", "orthopedic"],
    ) {
        priority_skills.extend_from_slice(&[
            "trauma_orthopedic_general_surgery",
            "clinical_diagnosis_internal_medicine",
            "anatomy_physiology",
            "first_aid_bls_cpr",
        ]);
    }

    priority_skills
}

/// Generate a complete, rules-legal NPC (`CharacterSheetRecord` + public/GM summaries) from a prompt and/or template.
#[must_use]
pub fn generate_npc_from_prompt(params: &NpcGenerationParams) -> GeneratedNpcResult {
    let effective_seed = if params.seed != 0 {
        params.seed
    } else {
        fnv1a_hash(&format!(
            "{}|{}|{}",
            params.prompt.trim(),
            params.template.as_str(),
            params.career_terms
        ))
    };
    let mut rng = SplitMix64::new(effective_seed);

    let inferred_template = if params.template == NpcTemplateKind::Auto {
        infer_npc_template_from_prompt(&params.prompt)
    } else {
        params.template
    };

    let npc_name = params
        .name_override
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| extract_npc_name_from_prompt(&params.prompt, effective_seed));

    let mut roll_group = |cat: CharacteristicCategory| -> Vec<i32> {
        let mut vals: Vec<i32> = roll_characteristic_group_pool(
            cat,
            CharacteristicRollMethod::Method1DropLowestD5,
            &mut |s| rng.roll_die(s),
        )
        .kept_values
        .into_iter()
        .map(|v| v.clamp(9, 18))
        .collect();
        vals.sort_by(|a, b| b.cmp(a));
        vals
    };

    let phy_vals = roll_group(CharacteristicCategory::Physical);
    let men_vals = roll_group(CharacteristicCategory::Mental);
    let soc_vals = roll_group(CharacteristicCategory::Social);
    let sen_vals = roll_group(CharacteristicCategory::Sensory);
    let psy_vals: Vec<i32> = (0..7)
        .map(|_| {
            roll_characteristic_method1(&mut |s| rng.roll_die(s))
                .total
                .clamp(7, 14)
        })
        .collect();

    let mut chars = CharacterCharacteristics {
        ego: psy_vals[0],
        slf: psy_vals[1],
        opn: psy_vals[2],
        blg: psy_vals[3],
        ext: psy_vals[4],
        cnf: psy_vals[5],
        agg: psy_vals[6],
        ..Default::default()
    };

    let mut aptitudes = CharacterAptitudes::default();
    let mut apt_pool = roll_aptitude_pool_11_drop_2(&mut |s| rng.roll_die(s)).kept_values;
    apt_pool.sort_by(|a, b| b.cmp(a));

    match inferred_template {
        NpcTemplateKind::MedicalDoctor => {
            chars.mem = men_vals[0].max(14);
            chars.ana = men_vals[1].max(14);
            chars.ins = men_vals[2].max(13);
            chars.dis = men_vals[3].max(13);
            chars.foc = men_vals[4].max(13);
            chars.cre = men_vals[5].max(10);
            chars.dex = phy_vals[0].max(13);
            chars.hlt = phy_vals[1].max(11);
            chars.fit = phy_vals[2].max(11);
            chars.spd = phy_vals[3];
            chars.kin = phy_vals[4];
            chars.for_stat = phy_vals[5];
            chars.com = soc_vals[0].max(13);
            chars.elo = soc_vals[1].max(12);
            chars.pre = soc_vals[2].max(11);
            chars.sig = sen_vals[0].max(12);
            chars.tac = sen_vals[1].max(12);
            chars.itu = sen_vals[2].max(12);
            chars.hea = sen_vals[3];
            chars.olf = sen_vals[4];
            chars.tas = sen_vals[5];
            aptitudes.apt_bio = apt_pool[0].max(2);
            aptitudes.apt_qnt = apt_pool[1].max(1);
            aptitudes.apt_soc = apt_pool[2].max(1);
        }
        NpcTemplateKind::StemEngineer => {
            chars.ana = men_vals[0].max(15);
            chars.mem = men_vals[1].max(13);
            chars.dis = men_vals[2].max(13);
            chars.cre = men_vals[3].max(13);
            chars.foc = men_vals[4].max(12);
            chars.ins = men_vals[5].max(12);
            chars.dex = phy_vals[0].max(13);
            chars.spd = phy_vals[1];
            chars.hlt = phy_vals[2];
            chars.fit = phy_vals[3];
            chars.kin = phy_vals[4];
            chars.for_stat = phy_vals[5];
            chars.elo = soc_vals[0];
            chars.com = soc_vals[1];
            chars.pre = soc_vals[2];
            chars.sig = sen_vals[0].max(12);
            chars.tac = sen_vals[1].max(12);
            chars.hea = sen_vals[2];
            chars.itu = sen_vals[3];
            chars.olf = sen_vals[4];
            chars.tas = sen_vals[5];
            aptitudes.apt_qnt = apt_pool[0].max(2);
            aptitudes.apt_cmp = apt_pool[1].max(2);
            aptitudes.apt_mec = apt_pool[2].max(1);
        }
        NpcTemplateKind::CombatSoldier => {
            chars.for_stat = phy_vals[0].max(13);
            chars.dex = phy_vals[1].max(13);
            chars.fit = phy_vals[2].max(13);
            chars.hlt = phy_vals[3].max(12);
            chars.spd = phy_vals[4].max(12);
            chars.kin = phy_vals[5].max(12);
            chars.foc = men_vals[0].max(13);
            chars.dis = men_vals[1].max(12);
            chars.ins = men_vals[2].max(12);
            chars.ana = men_vals[3];
            chars.mem = men_vals[4];
            chars.cre = men_vals[5];
            chars.pre = soc_vals[0].max(12);
            chars.com = soc_vals[1].max(12);
            chars.elo = soc_vals[2];
            chars.sig = sen_vals[0].max(13);
            chars.hea = sen_vals[1].max(12);
            chars.itu = sen_vals[2].max(12);
            chars.tac = sen_vals[3];
            chars.olf = sen_vals[4];
            chars.tas = sen_vals[5];
            aptitudes.apt_kin = apt_pool[0].max(2);
            aptitudes.apt_mec = apt_pool[1].max(1);
        }
        NpcTemplateKind::DetectiveInvestigator | NpcTemplateKind::IntelligenceOperative => {
            chars.ins = men_vals[0].max(14);
            chars.ana = men_vals[1].max(13);
            chars.foc = men_vals[2].max(13);
            chars.mem = men_vals[3].max(12);
            chars.dis = men_vals[4].max(12);
            chars.cre = men_vals[5].max(11);
            chars.dex = phy_vals[0].max(12);
            chars.spd = phy_vals[1].max(12);
            chars.fit = phy_vals[2].max(11);
            chars.for_stat = phy_vals[3].max(11);
            chars.hlt = phy_vals[4];
            chars.kin = phy_vals[5];
            chars.com = soc_vals[0].max(13);
            chars.pre = soc_vals[1].max(12);
            chars.elo = soc_vals[2].max(12);
            chars.sig = sen_vals[0].max(13);
            chars.itu = sen_vals[1].max(13);
            chars.hea = sen_vals[2].max(12);
            chars.tac = sen_vals[3];
            chars.olf = sen_vals[4];
            chars.tas = sen_vals[5];
            aptitudes.apt_soc = apt_pool[0].max(2);
            aptitudes.apt_lng = apt_pool[1].max(1);
            aptitudes.apt_kin = apt_pool[2].max(1);
        }
        NpcTemplateKind::SyndicateEnforcer => {
            chars.dex = phy_vals[0].max(13);
            chars.for_stat = phy_vals[1].max(12);
            chars.spd = phy_vals[2].max(12);
            chars.kin = phy_vals[3].max(11);
            chars.fit = phy_vals[4].max(11);
            chars.hlt = phy_vals[5].max(11);
            chars.ins = men_vals[0].max(12);
            chars.foc = men_vals[1].max(12);
            chars.cre = men_vals[2].max(11);
            chars.dis = men_vals[3].max(11);
            chars.ana = men_vals[4];
            chars.mem = men_vals[5];
            chars.pre = soc_vals[0].max(13);
            chars.com = soc_vals[1].max(12);
            chars.elo = soc_vals[2].max(11);
            chars.sig = sen_vals[0].max(12);
            chars.itu = sen_vals[1].max(12);
            chars.hea = sen_vals[2].max(11);
            chars.tac = sen_vals[3].max(11);
            chars.olf = sen_vals[4];
            chars.tas = sen_vals[5];
            aptitudes.apt_soc = apt_pool[0].max(1);
            aptitudes.apt_kin = apt_pool[1].max(1);
            aptitudes.apt_fin = apt_pool[2].max(1);
        }
        NpcTemplateKind::BlueCollarTech => {
            chars.dex = phy_vals[0].max(13);
            chars.for_stat = phy_vals[1].max(12);
            chars.fit = phy_vals[2].max(12);
            chars.kin = phy_vals[3].max(12);
            chars.hlt = phy_vals[4].max(11);
            chars.spd = phy_vals[5];
            chars.ana = men_vals[0].max(13);
            chars.foc = men_vals[1].max(12);
            chars.dis = men_vals[2].max(12);
            chars.ins = men_vals[3].max(11);
            chars.mem = men_vals[4];
            chars.cre = men_vals[5];
            chars.com = soc_vals[0].max(11);
            chars.pre = soc_vals[1];
            chars.elo = soc_vals[2];
            chars.tac = sen_vals[0].max(13);
            chars.sig = sen_vals[1].max(12);
            chars.hea = sen_vals[2].max(11);
            chars.itu = sen_vals[3];
            chars.olf = sen_vals[4];
            chars.tas = sen_vals[5];
            aptitudes.apt_mec = apt_pool[0].max(2);
            aptitudes.apt_fin = apt_pool[1].max(1);
            aptitudes.apt_kin = apt_pool[2].max(1);
        }
        NpcTemplateKind::FrontierRanger | NpcTemplateKind::Auto => {
            chars.fit = phy_vals[0].max(13);
            chars.hlt = phy_vals[1].max(13);
            chars.dex = phy_vals[2].max(12);
            chars.spd = phy_vals[3].max(12);
            chars.kin = phy_vals[4].max(12);
            chars.for_stat = phy_vals[5].max(11);
            chars.ins = men_vals[0].max(13);
            chars.foc = men_vals[1].max(12);
            chars.dis = men_vals[2].max(12);
            chars.mem = men_vals[3].max(11);
            chars.ana = men_vals[4];
            chars.cre = men_vals[5];
            chars.com = soc_vals[0].max(11);
            chars.pre = soc_vals[1];
            chars.elo = soc_vals[2];
            chars.sig = sen_vals[0].max(13);
            chars.hea = sen_vals[1].max(13);
            chars.olf = sen_vals[2].max(12);
            chars.itu = sen_vals[3].max(12);
            chars.tac = sen_vals[4];
            chars.tas = sen_vals[5];
            aptitudes.apt_bio = apt_pool[0].max(2);
            aptitudes.apt_kin = apt_pool[1].max(1);
        }
    }

    let hgt_mass = roll_correlated_height_mass(chars.for_stat, &mut |s| rng.roll_die(s));
    chars.hgt_m = hgt_mass.height_m;
    chars.mas_kg = hgt_mass.mass_kg;

    let priority_skills =
        apply_prompt_psyche_and_skill_cues(&params.prompt, inferred_template, &mut chars);

    let num_terms = params.career_terms.clamp(1, 4);
    let track = inferred_template.primary_career_track();
    let (origin_roll, stage4_path, hs_track, stage1_talent) = match inferred_template {
        NpcTemplateKind::MedicalDoctor | NpcTemplateKind::StemEngineer => (
            30,
            Some(Stage4Path::Path4AHighSchool),
            HighSchoolTrack::ApHonorsStemMagnet,
            "counting_basic_numeracy",
        ),
        NpcTemplateKind::SyndicateEnforcer => (
            10,
            Some(Stage4Path::Path4BStreetGang),
            HighSchoolTrack::StandardCollegePrep,
            "alertness_situational_awareness",
        ),
        NpcTemplateKind::FrontierRanger => (
            12,
            Some(Stage4Path::Path4CRuralAgrarian),
            HighSchoolTrack::VocationalTechnical,
            "foraging_campcraft",
        ),
        NpcTemplateKind::BlueCollarTech => (
            20,
            Some(Stage4Path::Path4AHighSchool),
            HighSchoolTrack::VocationalTechnical,
            "manual_dexterity_tinkering",
        ),
        _ => (
            22,
            Some(Stage4Path::Path4AHighSchool),
            HighSchoolTrack::StandardCollegePrep,
            "empathy_social_perception",
        ),
    };

    let career_terms = (0..num_terms)
        .map(|_| CareerTermConfig {
            track,
            term_roll_pairs: [(13, 11), (12, 10)],
            override_successes: Some(2),
            waiver_override: true,
        })
        .collect();

    let cfg = LifepathBuildConfig {
        origin_roll_2d20: origin_roll,
        stage1_early_talent: stage1_talent.to_string(),
        stage2_lit_override: Some(2),
        stage2_math_override: Some(2),
        stage2_playground_talent: "empathy_social_perception".to_string(),
        stage3_math_override: Some(2),
        stage3_sci_override: Some(2),
        stage3_hum_override: Some(2),
        age15_crisis_roll_2d20: 25,
        stage4_path_override: stage4_path,
        stage4a_hs_track: hs_track,
        stage4a_hs_override: Some(2),
        stage4b_gang_archetype: StreetGangArchetype::ShooterEnforcer,
        stage4c_rural_archetype: RuralYouthArchetype::PastoralHerderOutrider,
        adult_ged_waiver: false,
        career_terms,
        ..LifepathBuildConfig::default()
    };

    let baseline = compute_lifepath_baseline_from_config(&cfg, &chars, &aptitudes);
    let mut skills = baseline.baseline_skills.clone();

    for _pass in 0..3 {
        for &sk_id in &priority_skills {
            if let Some(spec) = lookup_skill_node(sk_id) {
                for &(parent_id, _) in spec.parent_prereqs {
                    try_advance_skill_legally(
                        parent_id,
                        &baseline,
                        &cfg,
                        &chars,
                        &aptitudes,
                        &mut skills,
                    );
                }
            }
            try_advance_skill_legally(sk_id, &baseline, &cfg, &chars, &aptitudes, &mut skills);
        }
    }

    auto_allocate_career_and_elective_skills(&baseline, &cfg, &chars, &aptitudes, &mut skills);

    let mut skills_i32 = BTreeMap::new();
    for (k, v) in &skills {
        skills_i32.insert(k.clone(), *v as i32);
    }

    let mut psyche_traits = Vec::with_capacity(7);
    for code in CharacteristicCode::PSYCHE_ALL {
        let score = chars.get_scale(code);
        let pole = match PsychePole::from_score(score) {
            PsychePole::LowPole => "LOW_POLE",
            PsychePole::Balanced => "BALANCED",
            PsychePole::HighPole => "HIGH_POLE",
        };
        psyche_traits.push(NpcPsycheTraitSummary {
            code: code.as_str().to_string(),
            name: code.full_name().to_string(),
            score,
            pole: pole.to_string(),
            descriptor: psyche_pole_descriptor(code, score).to_string(),
        });
    }

    let mut top_skills: Vec<(String, String, i32)> = skills
        .iter()
        .filter(|(_, rank)| **rank >= 2)
        .map(|(sk_id, rank)| {
            let name = lookup_skill_node(sk_id)
                .map(|n| n.name.to_string())
                .unwrap_or_else(|| sk_id.clone());
            (sk_id.clone(), name, *rank as i32)
        })
        .collect();
    top_skills.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
    top_skills.truncate(10);

    let notable_psyche: Vec<String> = psyche_traits
        .iter()
        .filter(|p| p.pole != "BALANCED")
        .map(|p| format!("{}={} ({})", p.code, p.score, p.descriptor))
        .collect();
    let psyche_line = if notable_psyche.is_empty() {
        "Balanced temperament across all 7 Psyche axes".to_string()
    } else {
        notable_psyche.join("; ")
    };

    let top_skills_line: String = top_skills
        .iter()
        .take(6)
        .map(|(_, name, rank)| format!("{name} (N={rank})"))
        .collect::<Vec<_>>()
        .join(", ");

    let age_years = baseline.age_years as i32;
    let role_title = inferred_template.default_role_title().to_string();

    let public_summary = format!(
        "{npc_name} — {role_title} (Age {age_years}, {:.2}m / {:.1}kg). Apparent demeanor: {}.",
        chars.hgt_m,
        chars.mas_kg,
        if !params.prompt.trim().is_empty() {
            params.prompt.trim()
        } else {
            role_title.as_str()
        }
    );

    let gm_secret_dossier = format!(
        "NPC Dossier: {npc_name} ({role_title}, Age {age_years}, Origin: {})\n\
         Prompt / Concept: {}\n\
         Psyche Profile: {psyche_line}\n\
         Key Skills: {top_skills_line}",
        baseline.origin.label,
        if params.prompt.trim().is_empty() {
            role_title.as_str()
        } else {
            params.prompt.trim()
        }
    );

    let lifepath_config_json = serde_json::to_string(&cfg).ok();

    let mut character = CharacterSheetRecord {
        character_id: format!("npc_{:016x}", effective_seed),
        owner_user_id: params.owner_user_id.clone(),
        version: 1,
        character_name: npc_name,
        callsign_or_role: role_title,
        age_years,
        origin_label: baseline.origin.label.to_string(),
        campaign_sqm: i32::from(baseline.campaign_sqm),
        operational_heat: i32::from(baseline.operational_heat),
        granted_access_tags: baseline
            .granted_access_tags
            .iter()
            .map(|t| t.as_str().to_string())
            .collect(),
        stage_history: baseline.stage_history,
        characteristics: characteristics_to_record(&chars),
        aptitudes: aptitudes_to_record(&aptitudes),
        skills: skills_i32,
        notes: Some(gm_secret_dossier.clone()),
        lifepath_config_json,
        verification_status: CharacterVerificationStatus::VerifiedOfficial,
        ..CharacterSheetRecord::default()
    };

    let audit = verify_character_sheet_legality(&character);
    character.sp_budget_valid = audit.sp_budget_valid;
    character.dag_prereqs_valid = audit.dag_prereqs_valid;
    character.audit_violations = audit.violations;

    GeneratedNpcResult {
        character,
        inferred_template,
        psyche_traits,
        top_skills,
        public_summary,
        gm_secret_dossier,
    }
}

fn try_advance_skill_legally(
    sk_id: &str,
    baseline: &crate::lifepath_economy::LifepathBaselineResult,
    cfg: &LifepathBuildConfig,
    chars: &CharacterCharacteristics,
    aptitudes: &CharacterAptitudes,
    skills: &mut BTreeMap<String, usize>,
) {
    let cur_n = skills.get(sk_id).copied().unwrap_or(0);
    let floor_n = baseline.baseline_skills.get(sk_id).copied().unwrap_or(0);
    if cur_n.saturating_sub(floor_n) >= baseline.max_post_school_gain_per_skill || cur_n >= 5 {
        return;
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
        return;
    }
    let ledger = compute_skill_allocation_sp_ledger(baseline, cfg, chars, aptitudes, skills);
    let (_, affordable) =
        preview_next_skill_step_sp(sk_id, target_n, cfg, &ledger, chars, aptitudes);
    if affordable {
        skills.insert(sk_id.to_string(), target_n);
        let post_ledger =
            compute_skill_allocation_sp_ledger(baseline, cfg, chars, aptitudes, skills);
        if post_ledger.formal_sp_spent > post_ledger.formal_sp_budget
            || post_ledger.experience_sp_spent > post_ledger.experience_sp_budget
        {
            if cur_n == 0 {
                skills.remove(sk_id);
            } else {
                skills.insert(sk_id.to_string(), cur_n);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_terry_kind_general_md_needs_to_belong() {
        let params = NpcGenerationParams {
            prompt: "Terry is a kind general MD, but needs to feel like she belongs".to_string(),
            name_override: None,
            template: NpcTemplateKind::Auto,
            career_terms: 2,
            seed: 42,
            owner_user_id: "gm_user".to_string(),
        };
        let res = generate_npc_from_prompt(&params);

        assert_eq!(res.character.character_name, "Terry");
        assert_eq!(res.inferred_template, NpcTemplateKind::MedicalDoctor);
        assert_eq!(
            res.character.callsign_or_role,
            "General MD & Clinical Physician"
        );

        assert!(
            res.character.characteristics.slf <= 7,
            "Expected Low SLF (Altruistic) for 'kind', got {}",
            res.character.characteristics.slf
        );
        assert!(
            res.character.characteristics.agg <= 7,
            "Expected Low AGG (Gentle) for 'kind', got {}",
            res.character.characteristics.agg
        );
        assert!(
            res.character.characteristics.blg >= 14,
            "Expected High BLG (Tribal / Group-Dependent) for 'needs to feel like she belongs', got {}",
            res.character.characteristics.blg
        );

        let diag_rank = res
            .character
            .skills
            .get("clinical_diagnosis_internal_medicine")
            .copied()
            .unwrap_or(0);
        let first_aid_rank = res
            .character
            .skills
            .get("first_aid_bls_cpr")
            .copied()
            .unwrap_or(0);
        assert!(
            diag_rank >= 1 && first_aid_rank >= 2,
            "Expected trained medical skills on Terry, got diag={diag_rank}, first_aid={first_aid_rank}"
        );

        let audit = verify_character_sheet_legality(&res.character);
        assert!(
            audit.is_legal,
            "Generated NPC must pass verify_character_sheet_legality: {:?}",
            audit.violations
        );
    }

    #[test]
    fn test_all_npc_templates_generate_legal_character_sheets() {
        for template in NpcTemplateKind::ALL {
            let params = NpcGenerationParams {
                prompt: "Veteran specialist with a rebellious streak".to_string(),
                name_override: Some(format!("Agent {}", template.as_str())),
                template,
                career_terms: 2,
                seed: 100,
                owner_user_id: "gm_user".to_string(),
            };
            let res = generate_npc_from_prompt(&params);
            let audit = verify_character_sheet_legality(&res.character);
            assert!(
                audit.is_legal,
                "Template {:?} failed legality audit: {:?}",
                template, audit.violations
            );
            assert!(res.character.characteristics.cnf <= 7);
        }
    }
}
