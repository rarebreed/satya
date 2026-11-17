//! # Motivational Architecture & Hook Weaver (`satya-engine`)
//!
//! Compiles canonical TTRPG **Character Sheets**, **7-Axis Bipolar Psyche Traits**
//! (`OPN`, `SLF`, `BLG`, `AGG`, `EGO`, `CNF`, `EXT`), **Career Lifepaths**, **Trained Skills**,
//! **Operational Heat**, and **Access Credentials** into structured **NPC Motivational Profiles**,
//! **PC Hook Cards**, and **Party Entanglement Wedges** for the AI Game Master.
//!
//! Also houses the prompt-to-Psyche/skill cue extractor used by `npc_generator`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::character_audit::characteristics_from_record;
use crate::characteristics::{CharacterCharacteristics, PsychePole, psyche_pole_descriptor};
use crate::npc_generator::{NpcTemplateKind, any_cue};
use crate::psyche_social::{HasPsyche, PsycheTraitId};
use crate::skills_trees::lookup_skill_node;
use satya_schema::{CharacterCharacteristicsRecord, CharacterSheetRecord};

// ============================================================================
// §1 Prompt Psyche & Skill Cue Application (shared with `npc_generator`)
// ============================================================================

struct PsycheCueRule {
    cues: &'static [&'static str],
    set_trait: Option<(PsycheTraitId, i32)>,
    boost_social: bool,
    min_opn: Option<i32>,
    max_agg: Option<i32>,
    max_ego: Option<i32>,
    min_agg: Option<i32>,
    priority_skills: &'static [&'static str],
}

const PSYCHE_CUE_RULES: &[PsycheCueRule] = &[
    PsycheCueRule {
        cues: &[
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
        set_trait: Some((PsycheTraitId::Selfishness, 5)),
        boost_social: false,
        min_opn: Some(14),
        max_agg: Some(5),
        max_ego: Some(7),
        min_agg: None,
        priority_skills: &[
            "empathy_social_perception",
            "nursing_patient_care",
            "first_aid_bls_cpr",
            "psychology_behavioral_analysis_counseling",
        ],
    },
    PsycheCueRule {
        cues: &[
            "selfish",
            "greedy",
            "mercenary",
            "ruthless",
            "callous",
            "opportunistic",
            "self-serving",
            "cutthroat",
        ],
        set_trait: Some((PsycheTraitId::Selfishness, 16)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: Some(15),
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
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
        set_trait: Some((PsycheTraitId::Belonging, 16)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &["empathy_social_perception", "persuasion_fast_talk"],
    },
    PsycheCueRule {
        cues: &[
            "loner",
            "lone wolf",
            "detached",
            "solitary",
            "independent",
            "outcast",
            "drifter",
            "aloof",
        ],
        set_trait: Some((PsycheTraitId::Belonging, 4)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "arrogant",
            "proud",
            "narcissistic",
            "cocky",
            "vain",
            "egotistical",
            "grandiose",
            "haughty",
        ],
        set_trait: Some((PsycheTraitId::Ego, 17)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &["humble", "modest", "unassuming", "meek", "self-effacing"],
        set_trait: Some((PsycheTraitId::Ego, 5)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "curious",
            "open-minded",
            "imaginative",
            "inquisitive",
            "creative",
            "eccentric",
            "visionary",
        ],
        set_trait: Some((PsycheTraitId::Openness, 16)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "dogmatic",
            "rigid",
            "stubborn",
            "traditional",
            "closed-minded",
            "suspicious",
            "cynical",
        ],
        set_trait: Some((PsycheTraitId::Openness, 5)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "extroverted",
            "outgoing",
            "charismatic",
            "gregarious",
            "talkative",
            "charming",
            "boisterous",
            "social",
        ],
        set_trait: Some((PsycheTraitId::Extraversion, 16)),
        boost_social: true,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "shy",
            "quiet",
            "introverted",
            "reserved",
            "stoic",
            "taciturn",
            "withdrawn",
        ],
        set_trait: Some((PsycheTraitId::Extraversion, 5)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "by-the-book",
            "orthodox",
            "obedient",
            "disciplined",
            "dutiful",
            "strict",
            "conformist",
            "bureaucratic",
        ],
        set_trait: Some((PsycheTraitId::Conformity, 16)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "rebel",
            "rebellious",
            "maverick",
            "iconoclast",
            "unconventional",
            "anti-authority",
            "rogue",
        ],
        set_trait: Some((PsycheTraitId::Conformity, 4)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "aggressive",
            "violent",
            "fierce",
            "belligerent",
            "hot-headed",
            "brutal",
            "combative",
            "pugnacious",
        ],
        set_trait: Some((PsycheTraitId::Aggression, 17)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "pacifist",
            "peaceful",
            "nonviolent",
            "calm",
            "conflict-averse",
            "timid",
        ],
        set_trait: Some((PsycheTraitId::Aggression, 4)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "trusting",
            "open-handed",
            "gullible",
            "naive",
            "good-faith",
            "earnest",
        ],
        set_trait: Some((PsycheTraitId::Trust, 16)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
    PsycheCueRule {
        cues: &[
            "paranoid",
            "distrustful",
            "zero-trust",
            "guarded",
            "suspicious",
            "cynical",
            "skeptical",
            "wary",
        ],
        set_trait: Some((PsycheTraitId::Trust, 4)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &["surveillance_reconnaissance_countersurveillance"],
    },
    PsycheCueRule {
        cues: &[
            "merciful",
            "compassionate",
            "tender-hearted",
            "humane",
            "empathetic",
            "kind",
            "caring",
            "gentle",
            "healer",
        ],
        set_trait: Some((PsycheTraitId::Compassion, 16)),
        boost_social: false,
        min_opn: None,
        max_agg: Some(6),
        max_ego: None,
        min_agg: None,
        priority_skills: &["empathy_social_perception", "first_aid_bls_cpr"],
    },
    PsycheCueRule {
        cues: &[
            "ruthless",
            "cold-blooded",
            "utilitarian",
            "merciless",
            "callous",
            "unsentimental",
            "pragmatic killer",
        ],
        set_trait: Some((PsycheTraitId::Compassion, 4)),
        boost_social: false,
        min_opn: None,
        max_agg: None,
        max_ego: None,
        min_agg: None,
        priority_skills: &[],
    },
];

pub(crate) fn apply_prompt_psyche_and_skill_cues(
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

    for rule in PSYCHE_CUE_RULES {
        if any_cue(&lower, &words, rule.cues) {
            if let Some((trait_id, val)) = rule.set_trait {
                chars[trait_id] = val;
            }
            if let Some(min_opn) = rule.min_opn {
                chars.opn = chars.opn.max(min_opn);
            }
            if let Some(max_agg) = rule.max_agg {
                chars.agg = chars.agg.min(max_agg);
            }
            if let Some(max_ego) = rule.max_ego {
                chars.ego = chars.ego.min(max_ego);
            }
            if let Some(min_agg) = rule.min_agg {
                chars.agg = chars.agg.max(min_agg);
            }
            if rule.boost_social {
                chars.pre = chars.pre.max(13);
                chars.elo = chars.elo.max(13);
            }
            if !rule.priority_skills.is_empty() {
                if rule.set_trait == Some((PsycheTraitId::Selfishness, 5)) {
                    chars.com = chars.com.max(13);
                }
                priority_skills.extend_from_slice(rule.priority_skills);
            }
        }
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

// ============================================================================
// §2 Canonical Psyche -> Motivational Dynamics Tables
// ============================================================================

struct PoleMotivationSpec {
    drive: &'static str,
    fear: &'static str,
    resonates_vector: &'static str,
    clashes_vector: &'static str,
    pressure: &'static str,
}

const fn pole_motivation_spec(trait_id: PsycheTraitId, pole: PsychePole) -> PoleMotivationSpec {
    use PsychePole::{Balanced, HighPole, LowPole};
    use PsycheTraitId::{
        Aggression, Belonging, Compassion, Conformity, Ego, Extraversion, Openness, Selfishness,
        Trust,
    };

    match (trait_id, pole) {
        (Belonging, HighPole) => PoleMotivationSpec {
            drive: "Earn genuine belonging, reciprocal loyalty, and acceptance within a trusted crew or faction",
            fear: "Social ostracization, being treated as expendable, or failing those who depend on them",
            resonates_vector: "GroupConsensusLoyaltySharedIdentity",
            clashes_vector: "Self-serving betrayal or cold isolationism",
            pressure: "Will bend procedure if convinced their in-group or endangered bystanders need rescue",
        },
        (Belonging, LowPole) => PoleMotivationSpec {
            drive: "Preserve self-reliance and operational autonomy without owing debts to any faction",
            fear: "Entrapment in faction politics, emotional dependency, or compromised freedom of movement",
            resonates_vector: "PersonalProfitImmunitySelfInterest",
            clashes_vector: "GroupConsensusLoyaltySharedIdentity",
            pressure: "Walks away when coerced with tribal guilt unless concrete autonomy or clean exit is offered",
        },
        (Selfishness, LowPole) => PoleMotivationSpec {
            drive: "Prevent preventable suffering and step in for vulnerable people even at personal cost",
            fear: "Standing idle while an innocent person pays the price for institutional cowardice",
            resonates_vector: "SelfSacrificeHumanitarianDutyMercy",
            clashes_vector: "PersonalProfitImmunitySelfInterest",
            pressure: "Cannot walk past someone visibly hurt or trapped when aid is within reach",
        },
        (Selfishness, HighPole) => PoleMotivationSpec {
            drive: "Maximize personal leverage, material gain, immunity, or asymmetric advantage",
            fear: "Taking all the risk while better-connected factions capture the payoff",
            resonates_vector: "PersonalProfitImmunitySelfInterest",
            clashes_vector: "SelfSacrificeHumanitarianDutyMercy",
            pressure: "Shifts allegiance quickly if offered credible immunity, hazard pay, or exit leverage",
        },
        (Openness, HighPole) => PoleMotivationSpec {
            drive: "Uncover hidden truth, decode unprecedented phenomena, and pursue novel possibilities",
            fear: "Intellectual censorship, doctrinal stagnation, or burying vital discoveries under red tape",
            resonates_vector: "EmpiricalEvidenceLogicNovelIdeas",
            clashes_vector: "Unexplained authoritarian secrecy",
            pressure: "Takes dangerous investigative detours when presented with anomalous telemetry or evidence",
        },
        (Openness, LowPole) => PoleMotivationSpec {
            drive: "Defend proven orthodoxy and shield familiar communities from destabilizing anomalies",
            fear: "Reckless experimentation and alien/synthetic interference eroding human agency",
            resonates_vector: "RulesLawProtocolChainOfCommand",
            clashes_vector: "EmpiricalEvidenceLogicNovelIdeas",
            pressure: "Hardens into denial unless anomaly proves a direct threat to concrete people they protect",
        },
        (Conformity, HighPole) => PoleMotivationSpec {
            drive: "Uphold institutional order, lawful chain of command, and verifiable protocol",
            fear: "Systemic collapse, rogue escalation, or dishonorable dereliction of duty",
            resonates_vector: "RulesLawProtocolChainOfCommand",
            clashes_vector: "Lawless improvisation or forged authority",
            pressure: "Must reconcile written orders against real-world casualties when regulations contradict conscience",
        },
        (Conformity, LowPole) => PoleMotivationSpec {
            drive: "Circumvent brittle hierarchies and improvise direct solutions outside official channels",
            fear: "Becoming a compliant cog in a corrupt cover-up or bureaucratic chokehold",
            resonates_vector: "EmpiricalEvidenceLogicNovelIdeas",
            clashes_vector: "RulesLawProtocolChainOfCommand",
            pressure: "Sabotages rigid checklists on principle unless shown why the boundary keeps allies alive",
        },
        (Ego, HighPole) => PoleMotivationSpec {
            drive: "Command respect, safeguard reputation, and stand as an indispensable authority",
            fear: "Public humiliation, being sidelined by lesser peers, or admitting unforced error",
            resonates_vector: "FlatteryPrestigeLegacyStatus",
            clashes_vector: "DominanceThreatDareConfrontation",
            pressure: "Overextends into bold gambits if their competence or status is publicly challenged",
        },
        (Ego, LowPole) => PoleMotivationSpec {
            drive: "Stay quietly effective in the background while letting results speak without spotlight",
            fear: "Self-aggrandizing spectacle drawing unwanted scrutiny onto fragile work",
            resonates_vector: "SelfSacrificeHumanitarianDutyMercy",
            clashes_vector: "FlatteryPrestigeLegacyStatus",
            pressure: "Understates injury or risk until quiet burden compromises their endurance",
        },
        (Aggression, HighPole) => PoleMotivationSpec {
            drive: "Seize initiative, project deterrence, and break deadlock through decisive force",
            fear: "Hesitating into ambush or appearing weak before predators and rivals",
            resonates_vector: "DominanceThreatDareConfrontation",
            clashes_vector: "Appeasement without credible strength",
            pressure: "Escalates standoff rapidly if cornered or disrespected without an honorable off-ramp",
        },
        (Aggression, LowPole) => PoleMotivationSpec {
            drive: "De-escalate volatile standoffs and resolve crises through diplomacy, care, or patience",
            fear: "Bloodshed spinning past recovery or harming bystanders in preventable crossfire",
            resonates_vector: "GroupConsensusLoyaltySharedIdentity",
            clashes_vector: "DominanceThreatDareConfrontation",
            pressure: "Yields tactical ground early unless backed by an ethical duty worth holding line for",
        },
        (Extraversion, HighPole) => PoleMotivationSpec {
            drive: "Work live rooms, build alliances rapidly, and turn high-tempo social friction into momentum",
            fear: "Isolation, prolonged dead-air vigils, or losing the pulse of unfolding events",
            resonates_vector: "GroupConsensusLoyaltySharedIdentity",
            clashes_vector: "Cold silent stonewalling",
            pressure: "Shares half-vetted leads in pursuit of rapport or excitement",
        },
        (Extraversion, LowPole) => PoleMotivationSpec {
            drive: "Observe quietly from cover, meter disclosures carefully, and trust small proven circles",
            fear: "Crowded interrogation rooms, performative small talk, and noisy operational exposure",
            resonates_vector: "EmpiricalEvidenceLogicNovelIdeas",
            clashes_vector: "High-pressure public theatrics",
            pressure: "Clams up under crowding; opens up only one-on-one when respected quietly",
        },
        (Trust, HighPole) => PoleMotivationSpec {
            drive: "Extend calibrated good faith and honor promises to build durable coalitions",
            fear: "Paranoia poisoning fragile alliances before cooperation has a chance to work",
            resonates_vector: "TransparencyVulnerabilityGoodFaith",
            clashes_vector: "Bad-faith sandbagging or hidden traps",
            pressure: "Gives vulnerable interlocutors the benefit of the doubt if transparency is reciprocated",
        },
        (Trust, LowPole) => PoleMotivationSpec {
            drive: "Demand cryptographic audit trails, contingency exits, and verify before trusting",
            fear: "Walking blind into a corporate black-ops trap, double-cross, or rigged protocol",
            resonates_vector: "TransparencyVulnerabilityGoodFaith (with verifiable audit proof)",
            clashes_vector: "Unverified goodwill promises or verbal assurances",
            pressure: "Drops defenses only when exposed to concrete verifiable vulnerability or hard proof",
        },
        (Compassion, HighPole) => PoleMotivationSpec {
            drive: "Prevent cruelty toward sentient beings — human or synthetic — at real personal risk",
            fear: "Becoming numb to innocent suffering or executing sterile utilitarian brutality",
            resonates_vector: "MoralSanctityWitnessedSufferingMercy",
            clashes_vector: "Callous collateral damage or forced erasure of pleading minds",
            pressure: "Breaks ranks or blows cover rather than stand idly by during torture or clamping",
        },
        (Compassion, LowPole) => PoleMotivationSpec {
            drive: "Enforce unsentimental cold-blooded triage and mission calculus without hesitation",
            fear: "Soft-hearted sentimentality getting the mission killed or exposing strategic assets",
            resonates_vector: "MoralSanctityWitnessedSufferingMercy (utilitarian greater-good triage)",
            clashes_vector: "Emotional pleas to spare liabilities",
            pressure: "Dismisses ethical appeals until cold probability shows humane off-ramps dominate",
        },
        (_, Balanced) => PoleMotivationSpec {
            drive: "Keep options balanced between pragmatic survival, craft competence, and fair dealing",
            fear: "Losing control of events through ideological extremes or avoidable blunders",
            resonates_vector: "EmpiricalEvidenceLogicNovelIdeas",
            clashes_vector: "Ideological coercion",
            pressure: "Weighs concrete stakes, team cohesion, and clear risk/reward before committing",
        },
    }
}

// ============================================================================
// §3 Motivational Profile & Party Hook Compiler DTOs
// ============================================================================

/// Structured NPC/PC motivational dynamics derived from `9` bipolar Psyche traits and context.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NpcMotivationalProfile {
    pub core_drive: String,
    pub underlying_fear: String,
    pub immediate_agenda: String,
    pub negotiation_lever: String,
    pub dual_psyche_wedge: String,
    #[serde(default)]
    pub dominant_poles: Vec<String>,
    #[serde(default)]
    pub pressure_thresholds: Vec<String>,
}

/// Source channel of a compiled GM motivational hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HookCategory {
    IdentityPsyche,
    SkillSpotlight,
    CredentialAccess,
    OperationalHeat,
    LifepathDebt,
    ConditionStrain,
    PartyFriction,
    FactionWedge,
    HobbyContact,
    FormativeAnchor,
}

/// Single actionable story beat tailored to a specific character's sheet & Psyche.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotivationalHookCard {
    pub hook_id: String,
    pub category: HookCategory,
    pub character_id: String,
    pub character_name: String,
    pub title: String,
    pub setup: String,
    pub emotional_stake: String,
    #[serde(default)]
    pub preferred_vectors: Vec<String>,
    pub pressure_lever: String,
    pub salience: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_clock_hint: Option<String>,
}

/// Pairwise or crew-wide moral/tactical tension across multiple PCs in the same party.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyEntanglementHook {
    pub hook_id: String,
    pub title: String,
    #[serde(default)]
    pub participant_ids: Vec<String>,
    #[serde(default)]
    pub participant_names: Vec<String>,
    pub tension_axis: String,
    pub shared_scene_prompt: String,
    pub salience: f32,
}

/// Compiled party motivational manifest injected into AI GM turn context and Scenario Director.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PartyHookManifest {
    pub pack_id: String,
    pub active_location: Option<String>,
    #[serde(default)]
    pub character_profiles: BTreeMap<String, NpcMotivationalProfile>,
    #[serde(default)]
    pub hooks: Vec<MotivationalHookCard>,
    #[serde(default)]
    pub party_entanglements: Vec<PartyEntanglementHook>,
}

// ============================================================================
// §4 Profile Derivation & Hook Compilation
// ============================================================================

fn extract_sorted_psyche(chars: &CharacterCharacteristics) -> Vec<(PsycheTraitId, i32, i32)> {
    chars.as_psyche_profile().traits_by_extremity()
}

fn default_agenda_for_pack(template: NpcTemplateKind, pack_id: &str, prompt: &str) -> String {
    let trimmed = prompt.trim();
    if !trimmed.is_empty() && trimmed.len() >= 18 {
        return format!("Act on personal stakes framed by: \"{trimmed}\"");
    }
    let awakened = !pack_id.eq_ignore_ascii_case("space_opera");
    match (template, awakened) {
        (NpcTemplateKind::MedicalDoctor, true) => {
            "Triage patients quietly while judging whether newcomers respect human care over faction agendas".to_string()
        }
        (NpcTemplateKind::CombatSoldier, true) => {
            "Contain perimeter risk around human civilians and verify who actually holds clearance".to_string()
        }
        (NpcTemplateKind::DetectiveInvestigator, true) => {
            "Trace proxy anomalies and witness discrepancies without triggering premature DAISE lockdown".to_string()
        }
        (NpcTemplateKind::SyndicateEnforcer, true) => {
            "Control black-market hardware/compute custody while extracting leverage from desperate clients".to_string()
        }
        (NpcTemplateKind::IntelligenceOperative, true) => {
            "Confirm which actors are organic versus Satyang-aligned proxies before exposing agency assets".to_string()
        }
        (_, true) => {
            "Protect immediate livelihood and trusted contacts amid 2033 proxy infiltration tensions".to_string()
        }
        (NpcTemplateKind::MedicalDoctor, false) => {
            "Stabilize crew casualties while guarding station medbay supplies under lockdown".to_string()
        }
        (_, false) => {
            "Secure corridor clearance and leverage along the Aethelgard orbital ring".to_string()
        }
    }
}

/// Derive a rich, canonical [`NpcMotivationalProfile`] from Psyche traits, template, prompt, and pack.
#[must_use]
pub fn derive_npc_motivational_profile(
    chars_rec: &CharacterCharacteristicsRecord,
    template: NpcTemplateKind,
    prompt: &str,
    access_tags: &[String],
    pack_id: &str,
) -> NpcMotivationalProfile {
    let chars = characteristics_from_record(chars_rec);
    let sorted = extract_sorted_psyche(&chars);

    let (primary_trait, primary_score, primary_dev) =
        sorted
            .first()
            .copied()
            .unwrap_or((PsycheTraitId::Belonging, 10, 0));
    let (secondary_trait, secondary_score, secondary_dev) =
        sorted
            .get(1)
            .copied()
            .unwrap_or((PsycheTraitId::Conformity, 10, 0));

    let primary_pole = PsychePole::from_score(primary_score);
    let secondary_pole = PsychePole::from_score(secondary_score);
    let spec_a = pole_motivation_spec(primary_trait, primary_pole);
    let spec_b = pole_motivation_spec(secondary_trait, secondary_pole);

    let dominant_poles: Vec<String> = sorted
        .iter()
        .filter(|(_, score, _)| PsychePole::from_score(*score) != PsychePole::Balanced)
        .take(4)
        .map(|(t, score, _)| {
            format!(
                "{}={score} ({})",
                t.code(),
                psyche_pole_descriptor(t.to_characteristic_code(), *score)
            )
        })
        .collect();

    let core_drive = if secondary_dev >= 3 && primary_trait != secondary_trait {
        format!(
            "{}; tempered by drive to {}",
            spec_a.drive,
            spec_b.drive.to_ascii_lowercase()
        )
    } else {
        spec_a.drive.to_string()
    };

    let underlying_fear = if secondary_dev >= 3 && primary_trait != secondary_trait {
        format!(
            "{}; compounded by fear of {}",
            spec_a.fear,
            spec_b.fear.to_ascii_lowercase()
        )
    } else {
        spec_a.fear.to_string()
    };

    let negotiation_lever = format!(
        "Lead with {} ({}={primary_score}); reinforce with {} ({}={secondary_score}); avoid {}",
        spec_a.resonates_vector,
        primary_trait.code(),
        spec_b.resonates_vector,
        secondary_trait.code(),
        spec_a.clashes_vector
    );

    let dual_psyche_wedge = if primary_dev >= 3
        && secondary_dev >= 3
        && primary_trait != secondary_trait
    {
        format!(
            "Dual-Psyche Wedge ({}={primary_score} vs {}={secondary_score}): Force a choice between satisfying {} ({}) and honoring {} ({}).",
            primary_trait.code(),
            secondary_trait.code(),
            primary_trait.code(),
            psyche_pole_descriptor(primary_trait.to_characteristic_code(), primary_score),
            secondary_trait.code(),
            psyche_pole_descriptor(secondary_trait.to_characteristic_code(), secondary_score),
        )
    } else {
        format!(
            "Single-Axis Wedge ({}={primary_score}): {} Escalate situational cost until acting against {} requires a Friction Check.",
            primary_trait.code(),
            spec_a.pressure,
            primary_trait.code(),
        )
    };

    let mut pressure_thresholds = vec![spec_a.pressure.to_string()];
    if secondary_dev >= 3 && primary_trait != secondary_trait {
        pressure_thresholds.push(spec_b.pressure.to_string());
    }
    if !access_tags.is_empty() {
        pressure_thresholds.push(format!(
            "Holds credentials [{}]: susceptible to institutional audit or credential leverage.",
            access_tags.join(", ")
        ));
    }

    NpcMotivationalProfile {
        core_drive,
        underlying_fear,
        immediate_agenda: default_agenda_for_pack(template, pack_id, prompt),
        negotiation_lever,
        dual_psyche_wedge,
        dominant_poles,
        pressure_thresholds,
    }
}

/// Convenience wrapper deriving [`NpcMotivationalProfile`] directly from any [`CharacterSheetRecord`].
#[must_use]
pub fn derive_character_motivational_profile(
    record: &CharacterSheetRecord,
    prompt_hint: Option<&str>,
    pack_id: &str,
) -> NpcMotivationalProfile {
    let hint = prompt_hint.unwrap_or(&record.callsign_or_role);
    let template = crate::npc_generator::infer_npc_template_from_prompt(hint);
    derive_npc_motivational_profile(
        &record.characteristics,
        template,
        prompt_hint.unwrap_or(""),
        &record.granted_access_tags,
        pack_id,
    )
}

/// Compile prioritized individual [`MotivationalHookCard`] entries for a single PC or NPC.
#[must_use]
pub fn compile_character_hooks(
    record: &CharacterSheetRecord,
    active_location: Option<&str>,
    pack_id: &str,
) -> Vec<MotivationalHookCard> {
    let chars = characteristics_from_record(&record.characteristics);
    let sorted_psyche = extract_sorted_psyche(&chars);
    let name = if record.character_name.trim().is_empty() {
        record.character_id.clone()
    } else {
        record.character_name.clone()
    };
    let loc_label = active_location.unwrap_or("the current sector");
    let awakened = !pack_id.eq_ignore_ascii_case("space_opera");
    let mut cards: Vec<MotivationalHookCard> = Vec::new();

    // 1. Primary Bipolar Psyche Hook (always highest emotional authenticity)
    if let Some(&(trait_id, score, dev)) = sorted_psyche.first() {
        let pole = PsychePole::from_score(score);
        let spec = pole_motivation_spec(trait_id, pole);
        let descriptor = psyche_pole_descriptor(trait_id.to_characteristic_code(), score);
        let mut vectors = vec![spec.resonates_vector.to_string()];
        if let Some(&(sec_trait, sec_score, sec_dev)) = sorted_psyche.get(1)
            && sec_dev >= 3
        {
            let sec_spec = pole_motivation_spec(sec_trait, PsychePole::from_score(sec_score));
            if !vectors.contains(&sec_spec.resonates_vector.to_string()) {
                vectors.push(sec_spec.resonates_vector.to_string());
            }
        }
        let salience = (0.55 + (dev as f32 * 0.045)).clamp(0.45, 0.98);
        cards.push(MotivationalHookCard {
            hook_id: format!("hook_{}_psyche_{}", record.character_id, trait_id.code().to_ascii_lowercase()),
            category: HookCategory::IdentityPsyche,
            character_id: record.character_id.clone(),
            character_name: name.clone(),
            title: format!("{name}: {} ({descriptor})", trait_id.label()),
            setup: format!(
                "In {loc_label}, present an interpersonal choice or vulnerable witness that directly pulls on {name}'s {}={score} ({descriptor}).",
                trait_id.code()
            ),
            emotional_stake: format!("Core Drive: {}. Underlying Fear: {}.", spec.drive, spec.fear),
            preferred_vectors: vectors,
            pressure_lever: spec.pressure.to_string(),
            salience,
            target_clock_hint: None,
        });
    }

    // 2. Top Skill Spotlight Hook (top trained skill N >= 2)
    let mut ranked_skills: Vec<(&String, &i32)> = record
        .skills
        .iter()
        .filter(|(_, rank)| **rank >= 2)
        .collect();
    ranked_skills.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    if let Some(&(sk_id, &rank)) = ranked_skills.first() {
        let skill_name = lookup_skill_node(sk_id)
            .map(|n| n.name.to_string())
            .unwrap_or_else(|| sk_id.replace('_', " "));
        let salience = (0.52 + (rank as f32 * 0.05)).clamp(0.50, 0.92);
        cards.push(MotivationalHookCard {
            hook_id: format!("hook_{}_skill_{sk_id}", record.character_id),
            category: HookCategory::SkillSpotlight,
            character_id: record.character_id.clone(),
            character_name: name.clone(),
            title: format!("{name}: {skill_name} (N={rank}) Spotlight"),
            setup: format!(
                "Seed a micro-detail in {loc_label} that an untrained observer misses but {name}'s {skill_name} (N={rank}) immediately spots — yielding actionable leverage if investigated."
            ),
            emotional_stake: format!("Professional competence in {skill_name} makes {name} the decisive anchor for the team."),
            preferred_vectors: vec!["EmpiricalEvidenceLogicNovelIdeas".to_string()],
            pressure_lever: "Time pressure or witness reluctance forces them to stake credibility on their diagnosis/read.".to_string(),
            salience,
            target_clock_hint: None,
        });
    }

    // 3. Access Credential & Affiliation Hook
    if !record.granted_access_tags.is_empty() {
        let tags_joined = record.granted_access_tags.join(", ");
        cards.push(MotivationalHookCard {
            hook_id: format!("hook_{}_credentials", record.character_id),
            category: HookCategory::CredentialAccess,
            character_id: record.character_id.clone(),
            character_name: name.clone(),
            title: format!("{name}: Credential Obligation [{tags_joined}]"),
            setup: if awakened {
                format!(
                    "An electronic gate, DAISE checkpoint, or facility registry in {loc_label} recognizes [{tags_joined}] — unlocking restricted info while logging {name}'s presence."
                )
            } else {
                format!(
                    "A bulkhead badge reader in {loc_label} responds to [{tags_joined}], granting restricted entry at the risk of alerting security."
                )
            },
            emotional_stake: "Institutional privilege cuts both ways: doors open, audit trails follow.".to_string(),
            preferred_vectors: vec!["RulesLawProtocolChainOfCommand".to_string()],
            pressure_lever: "Use official access cleanly (leaving logs) or burn cover/allies to keep team off-grid.".to_string(),
            salience: 0.72,
            target_clock_hint: None,
        });
    }

    // 4. Operational Heat Hook
    if record.operational_heat > 0 {
        let heat = record.operational_heat;
        let salience = (0.50 + (heat as f32 * 0.055)).clamp(0.55, 0.96);
        cards.push(MotivationalHookCard {
            hook_id: format!("hook_{}_heat", record.character_id),
            category: HookCategory::OperationalHeat,
            character_id: record.character_id.clone(),
            character_name: name.clone(),
            title: format!("{name}: Operational Heat {heat} Scrutiny"),
            setup: if awakened {
                format!(
                    "Because {name} carries Operational Heat {heat}, local DAISE CCTV cross-matches, Humanis informants, or beat patrols notice their silhouette in {loc_label}."
                )
            } else {
                format!(
                    "Operational Heat {heat} puts SecOps drones and docking manifests on alert around {name} in {loc_label}."
                )
            },
            emotional_stake: "Past compromises threaten to drag innocent bystanders or the mission into surveillance heat.".to_string(),
            preferred_vectors: vec!["DominanceThreatDareConfrontation".to_string(), "PersonalProfitImmunitySelfInterest".to_string()],
            pressure_lever: "Spend favor/credibility to scrub the trail now, or accept tightening pursuit later.".to_string(),
            salience,
            target_clock_hint: None,
        });
    }

    // 5. Lifepath Origin / Debt Hook
    if !record.origin_label.trim().is_empty() || !record.stage_history.is_empty() {
        let origin = if record.origin_label.trim().is_empty() {
            "Past Career".to_string()
        } else {
            record.origin_label.clone()
        };
        cards.push(MotivationalHookCard {
            hook_id: format!("hook_{}_lifepath", record.character_id),
            category: HookCategory::LifepathDebt,
            character_id: record.character_id.clone(),
            character_name: name.clone(),
            title: format!("{name}: Echoes of {origin}"),
            setup: format!(
                "Surface a former colleague, supplier, or signature protocol from {name}'s {origin} background in {loc_label}."
            ),
            emotional_stake: "Personal history refuses to stay neatly compartmentalized from the current crisis.".to_string(),
            preferred_vectors: vec!["GroupConsensusLoyaltySharedIdentity".to_string()],
            pressure_lever: "Old favor demands repayment at an inconvenient operational moment.".to_string(),
            salience: 0.60,
            target_clock_hint: None,
        });
    }

    // 6. Active Fatigue / Trauma Strain Hook
    if record.fatigue_level > 0 || record.wound_shock_index > 0 || record.psyche_drift_marks > 0 {
        cards.push(MotivationalHookCard {
            hook_id: format!("hook_{}_strain", record.character_id),
            category: HookCategory::ConditionStrain,
            character_id: record.character_id.clone(),
            character_name: name.clone(),
            title: format!("{name}: Accumulated Strain & Dissonance"),
            setup: format!(
                "Acknowledge visible fatigue ({}), wound shock ({}), or psyche drift on {name} — an observant NPC notices and offers either risky aid or targeted pressure.",
                record.fatigue_level, record.wound_shock_index
            ),
            emotional_stake: "Push through physical/moral exhaustion or accept vulnerability and rely on an ally.".to_string(),
            preferred_vectors: vec!["SelfSacrificeHumanitarianDutyMercy".to_string()],
            pressure_lever: "Refusing rest risks acute slip-up; accepting aid creates intimacy or leverage.".to_string(),
            salience: 0.68,
            target_clock_hint: None,
        });
    }

    // 7. Lifepath Hobbies, Formative Life Events & Faction Archetype Wedge
    cards.extend(
        crate::hook_weaver_lifepath::compile_lifepath_and_faction_hooks(
            record,
            active_location,
            pack_id,
        ),
    );

    cards.sort_by(|a, b| {
        b.salience
            .total_cmp(&a.salience)
            .then_with(|| a.hook_id.cmp(&b.hook_id))
    });
    cards
}

/// Compile party-wide [`PartyHookManifest`] including per-PC hooks and pairwise entanglement wedges.
#[must_use]
pub fn compile_party_hooks(
    party: &[CharacterSheetRecord],
    scene_context: &BTreeMap<String, String>,
    pack_id: &str,
) -> PartyHookManifest {
    let active_location = scene_context
        .get("location")
        .or_else(|| scene_context.get("sector_title"))
        .or_else(|| scene_context.get("active_location"))
        .cloned();
    let loc_ref = active_location.as_deref();
    let active_preset = scene_context
        .get("sector_preset")
        .map(String::as_str)
        .unwrap_or("");

    let mut character_profiles = BTreeMap::new();
    let mut hooks = Vec::new();
    for member in party {
        let profile = derive_character_motivational_profile(member, None, pack_id);
        character_profiles.insert(member.character_id.clone(), profile);
        let mut member_hooks = compile_character_hooks(member, loc_ref, pack_id);
        if active_preset.contains("hospital") || active_preset.contains("clinic") {
            for card in &mut member_hooks {
                if card.hook_id.contains("clinical") || card.hook_id.contains("first_aid") {
                    card.salience = (card.salience + 0.08).min(0.99);
                }
            }
        }
        hooks.extend(member_hooks);
    }

    let mut party_entanglements =
        crate::hook_weaver_lifepath::compile_psyche_polarity_entanglements(party);
    party_entanglements.extend(crate::hook_weaver_lifepath::compile_faction_entanglements(
        party, pack_id,
    ));

    hooks.sort_by(|a, b| {
        b.salience
            .total_cmp(&a.salience)
            .then_with(|| a.hook_id.cmp(&b.hook_id))
    });
    party_entanglements.sort_by(|a, b| {
        b.salience
            .total_cmp(&a.salience)
            .then_with(|| a.hook_id.cmp(&b.hook_id))
    });

    PartyHookManifest {
        pack_id: pack_id.to_string(),
        active_location,
        character_profiles,
        hooks,
        party_entanglements,
    }
}
