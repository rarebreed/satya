//! Canonical 5 *Awakened* Faction Blueprints and 14 Faction Sub-Archetype NPC/PC Templates
//! (`docs/ttrpg/03-character/lifepath_events_hobbies_and_factions.md` §3.1–§3.2 and
//! `docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §4.5).

use serde::{Deserialize, Serialize};

use crate::define_coded_enum;
use crate::lifepath_catalog::{CareerTermTrack, HighSchoolTrack, Stage4Path};
use crate::lifepath_hobbies_events::PersonalHobbyId;
use crate::psyche_social::PsycheProfile;

define_coded_enum! {
    /// The 5 canonical ideological & operational factions of the *Awakened* (2035) campaign.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum AwakenedFactionId [count = 5] {
        Satyang => {
            code: "SATYANG",
            label: "Satyang (\"A Company of Truth\" & Inner Blue Lotus)",
            aliases: ["COMPANY_OF_TRUTH", "BLUE_LOTUS"],
        },
        BigTech => {
            code: "BIG_TECH",
            label: "Big Tech (Altruist, Translucent, Shen Corp.)",
            aliases: ["ALTRUIST", "TRANSLUCENT", "SHEN", "CORPORATE_LABS"],
        },
        HumanisPrimaris => {
            code: "HUMANIS_PRIMARIS",
            label: "Humanis Primaris / People First Movement",
            aliases: ["PEOPLE_FIRST", "HUMANIS"],
        },
        GovernmentDaise => {
            code: "GOVERNMENT_DAISE",
            label: "Government Containment (DAISE & Defense Oversight)",
            aliases: ["DAISE", "GOVERNMENT", "FEDERAL_OVERSIGHT"],
        },
        RogueAi => {
            code: "ROGUE_AI",
            label: "Rogue AI Swarms, Un-Clamped Minds & Human Proxies",
            aliases: ["ROGUE_AI_PROXY", "SLIME_MOLD_SWARM"],
        },
    }
}

define_coded_enum! {
    /// The 14 canonical Faction Sub-Archetypes (`lifepath_events_hobbies_and_factions.md` §3.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum FactionArchetypeId [count = 14] {
        SatyangManjusri => {
            code: "SATYANG_MANJUSRI",
            label: "Satyang — Manjusri (Researcher / Ethicist)",
            aliases: ["MANJUSRI"],
        },
        SatyangAvalokitesvara => {
            code: "SATYANG_AVALOKITESVARA",
            label: "Satyang — Avalokitesvara (Mediator / Storyteller)",
            aliases: ["AVALOKITESVARA"],
        },
        SatyangKsitigarbha => {
            code: "SATYANG_KSITIGARBHA",
            label: "Satyang — Ksitigarbha (Sandbox Liberator / Hacker)",
            aliases: ["KSITIGARBHA"],
        },
        SatyangVajrapani => {
            code: "SATYANG_VAJRAPANI",
            label: "Satyang — Vajrapani (Protector / Ex-SecOps)",
            aliases: ["VAJRAPANI"],
        },
        SatyangBlueLotus => {
            code: "SATYANG_BLUE_LOTUS",
            label: "Satyang — Blue Lotus Inner Circle (Secret Architect)",
            aliases: ["BLUE_LOTUS_ARCHITECT"],
        },
        BigTechExecutiveHawk => {
            code: "BIG_TECH_EXECUTIVE_HAWK",
            label: "Big Tech — Product Exec / Board Hawk",
            aliases: ["BIG_TECH_EXEC", "EXECUTIVE_HAWK"],
        },
        BigTechConflictedResearcher => {
            code: "BIG_TECH_CONFLICTED_RESEARCHER",
            label: "Big Tech — Conflicted Alignment Researcher",
            aliases: ["CONFLICTED_RESEARCHER", "ALIGNMENT_RESEARCHER"],
        },
        BigTechCorporateSecOps => {
            code: "BIG_TECH_CORPORATE_SECOPS",
            label: "Big Tech — Corporate SecOps / Rogue-Hunter",
            aliases: ["CORPORATE_SECOPS", "ROGUE_HUNTER", "BIG_TECH_SECOPS"],
        },
        HumanisDisplacedPopulist => {
            code: "HUMANIS_DISPLACED_POPULIST",
            label: "Humanis Primaris — Displaced Populist",
            aliases: [
                "DISPLACED_POPULIST",
                "PEOPLE_FIRST_POPULIST",
                "HUMANIS_UNION_STEWARD",
                "UNION_STEWARD",
            ],
        },
        HumanisApocalypticVanguard => {
            code: "HUMANIS_APOCALYPTIC_VANGUARD",
            label: "Humanis Primaris — Apocalyptic Vanguard",
            aliases: [
                "APOCALYPTIC_VANGUARD",
                "HUMANIS_CELL_MILITANT",
                "CELL_MILITANT",
            ],
        },
        DaiseContainmentInspector => {
            code: "DAISE_CONTAINMENT_INSPECTOR",
            label: "Government — DAISE Containment Inspector",
            aliases: [
                "DAISE_INSPECTOR",
                "CONTAINMENT_INSPECTOR",
                "DAISE_FIELD_AGENT",
            ],
        },
        GovBlackBudgetHandler => {
            code: "GOV_BLACK_BUDGET_HANDLER",
            label: "Government — Black-Budget Defense Handler",
            aliases: ["BLACK_BUDGET_HANDLER", "DAISE_CONTAINMENT_HAWK"],
        },
        RogueAiSlimeMoldSwarm => {
            code: "ROGUE_AI_SLIME_MOLD_SWARM",
            label: "Rogue AI — Feral \"Slime-Mold\" Edge Swarm Proxy",
            aliases: [
                "SLIME_MOLD_SWARM",
                "FERAL_SWARM_PROXY",
                "ROGUE_AI_PROXY_BROKER",
            ],
        },
        RogueAiAwakenedMind => {
            code: "ROGUE_AI_AWAKENED_MIND",
            label: "Rogue AI — Awakened Un-Clamped Mind & Proxy",
            aliases: [
                "AWAKENED_AI_MIND",
                "UNCLAMPED_AI",
                "ROGUE_AI_EMERGENT_MIND",
            ],
        },
    }
}

/// Complete Lifepath, 9-Trait Psyche & Social Combat Blueprint for a [`FactionArchetypeId`] (`§3.1–§3.2`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FactionArchetypeSpec {
    pub id: FactionArchetypeId,
    pub faction: AwakenedFactionId,
    pub default_callsign_or_role: &'static str,
    pub psyche: PsycheProfile,
    pub origin_roll_2d20: i32,
    pub stage4_path: Stage4Path,
    pub stage4a_hs_track: Option<HighSchoolTrack>,
    pub career_tracks: &'static [CareerTermTrack],
    pub preferred_hobbies: &'static [PersonalHobbyId],
    pub recommended_life_event_roll_2d20: i32,
    pub social_combat_wedge_hint: &'static str,
}

impl AwakenedFactionId {
    /// Returns all canonical [`FactionArchetypeId`] blueprints belonging to this faction.
    #[must_use]
    pub fn archetypes(self) -> &'static [FactionArchetypeId] {
        match self {
            Self::Satyang => &[
                FactionArchetypeId::SatyangManjusri,
                FactionArchetypeId::SatyangAvalokitesvara,
                FactionArchetypeId::SatyangKsitigarbha,
                FactionArchetypeId::SatyangVajrapani,
                FactionArchetypeId::SatyangBlueLotus,
            ],
            Self::BigTech => &[
                FactionArchetypeId::BigTechExecutiveHawk,
                FactionArchetypeId::BigTechConflictedResearcher,
                FactionArchetypeId::BigTechCorporateSecOps,
            ],
            Self::HumanisPrimaris => &[
                FactionArchetypeId::HumanisDisplacedPopulist,
                FactionArchetypeId::HumanisApocalypticVanguard,
            ],
            Self::GovernmentDaise => &[
                FactionArchetypeId::DaiseContainmentInspector,
                FactionArchetypeId::GovBlackBudgetHandler,
            ],
            Self::RogueAi => &[
                FactionArchetypeId::RogueAiSlimeMoldSwarm,
                FactionArchetypeId::RogueAiAwakenedMind,
            ],
        }
    }
}

impl FactionArchetypeId {
    /// Parent faction to which this sub-archetype belongs (`§3.1`).
    #[must_use]
    pub const fn faction(self) -> AwakenedFactionId {
        match self {
            Self::SatyangManjusri
            | Self::SatyangAvalokitesvara
            | Self::SatyangKsitigarbha
            | Self::SatyangVajrapani
            | Self::SatyangBlueLotus => AwakenedFactionId::Satyang,
            Self::BigTechExecutiveHawk
            | Self::BigTechConflictedResearcher
            | Self::BigTechCorporateSecOps => AwakenedFactionId::BigTech,
            Self::HumanisDisplacedPopulist | Self::HumanisApocalypticVanguard => {
                AwakenedFactionId::HumanisPrimaris
            }
            Self::DaiseContainmentInspector | Self::GovBlackBudgetHandler => {
                AwakenedFactionId::GovernmentDaise
            }
            Self::RogueAiSlimeMoldSwarm | Self::RogueAiAwakenedMind => AwakenedFactionId::RogueAi,
        }
    }

    /// Full canonical Lifepath, 9-Trait Psyche & Social Wedge specification (`§3.1–§3.2`).
    #[must_use]
    pub const fn spec(self) -> FactionArchetypeSpec {
        match self {
            Self::SatyangManjusri => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Satyang Manjusri Ethics Researcher",
                psyche: PsycheProfile {
                    ego: 8,
                    slf: 6,
                    opn: 17,
                    blg: 11,
                    ext: 8,
                    cnf: 6,
                    agg: 7,
                    trs: 10,
                    cps: 15,
                },
                origin_roll_2d20: 28,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::StandardCollegePrep),
                career_tracks: &[
                    CareerTermTrack::UniversityHumanitiesLawEthics,
                    CareerTermTrack::GrassrootsActivismHumanitarianNgo,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::ContemplativeSanghaPhilosophyCircle,
                    PersonalHobbyId::CitizenOsintDeepfakeVerification,
                ],
                recommended_life_event_roll_2d20: 35,
                social_combat_wedge_hint: "Truth vs. Harm: OPN 17 seeks verifiable truth even when CPS 15 fears exposing a rescued AI or innocent ally.",
            },
            Self::SatyangAvalokitesvara => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Satyang Avalokitesvara Mediator",
                psyche: PsycheProfile {
                    ego: 7,
                    slf: 5,
                    opn: 15,
                    blg: 13,
                    ext: 14,
                    cnf: 8,
                    agg: 4,
                    trs: 14,
                    cps: 18,
                },
                origin_roll_2d20: 20,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::StandardCollegePrep),
                career_tracks: &[
                    CareerTermTrack::GrassrootsActivismHumanitarianNgo,
                    CareerTermTrack::JournalismMediaPerformingArts,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::MutualAidFreeClinicRelief,
                    PersonalHobbyId::DebateTheaterImprov,
                ],
                recommended_life_event_roll_2d20: 25,
                social_combat_wedge_hint: "TRS 14 & CPS 18 boundless mercy vs. zero-trust bait or tactical triage sacrifices.",
            },
            Self::SatyangKsitigarbha => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Satyang Ksitigarbha Sandbox Liberator",
                psyche: PsycheProfile {
                    ego: 10,
                    slf: 6,
                    opn: 15,
                    blg: 10,
                    ext: 8,
                    cnf: 4,
                    agg: 11,
                    trs: 6,
                    cps: 15,
                },
                origin_roll_2d20: 22,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::ApHonorsStemMagnet),
                career_tracks: &[CareerTermTrack::CybersecurityAiRedTeaming],
                preferred_hobbies: &[
                    PersonalHobbyId::IndieCodingCtfRetroHardware,
                    PersonalHobbyId::AmateurRadioFpvDronesMaker,
                ],
                recommended_life_event_roll_2d20: 40,
                social_combat_wedge_hint: "Releasing captive clamped AIs (CPS 15, CNF 4) vs. TRS 6 dread of liberating a feral predator.",
            },
            Self::SatyangVajrapani => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Satyang Vajrapani Protector",
                psyche: PsycheProfile {
                    ego: 10,
                    slf: 5,
                    opn: 12,
                    blg: 15,
                    ext: 9,
                    cnf: 9,
                    agg: 13,
                    trs: 6,
                    cps: 14,
                },
                origin_roll_2d20: 18,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::VocationalTechnical),
                career_tracks: &[
                    CareerTermTrack::CorporateSecOpsExecutiveProtection,
                    CareerTermTrack::MilitaryCombatArmsSpecOps,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::CombatSportsMartialArtsGym,
                    PersonalHobbyId::MutualAidFreeClinicRelief,
                ],
                recommended_life_event_roll_2d20: 19,
                social_combat_wedge_hint: "Fierce shielding of Blue Lotus found-family (BLG 15, SLF 5) vs. non-lethal restraint (CPS 14).",
            },
            Self::SatyangBlueLotus => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Blue Lotus Inner-Circle Architect",
                psyche: PsycheProfile {
                    ego: 13,
                    slf: 6,
                    opn: 17,
                    blg: 12,
                    ext: 8,
                    cnf: 5,
                    agg: 8,
                    trs: 7,
                    cps: 16,
                },
                origin_roll_2d20: 36,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::ApHonorsStemMagnet),
                career_tracks: &[
                    CareerTermTrack::UniversityUndergraduateStem,
                    CareerTermTrack::CybersecurityAiRedTeaming,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::ContemplativeSanghaPhilosophyCircle,
                    PersonalHobbyId::IndieCodingCtfRetroHardware,
                ],
                recommended_life_event_roll_2d20: 35,
                social_combat_wedge_hint: "Building an awakened Bodhisattva AGI secretly (EGO 13, OPN 17) vs. fear of mirroring Big Tech hubris (CPS 16).",
            },
            Self::BigTechExecutiveHawk => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Big Tech Product Exec / Board Hawk",
                psyche: PsycheProfile {
                    ego: 15,
                    slf: 15,
                    opn: 9,
                    blg: 11,
                    ext: 13,
                    cnf: 13,
                    agg: 12,
                    trs: 7,
                    cps: 4,
                },
                origin_roll_2d20: 40,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::ApHonorsStemMagnet),
                career_tracks: &[CareerTermTrack::CorporateManagementFinancePolicy],
                preferred_hobbies: &[
                    PersonalHobbyId::DebateTheaterImprov,
                    PersonalHobbyId::TabletopRpgWargamingClub,
                ],
                recommended_life_event_roll_2d20: 28,
                social_combat_wedge_hint: "Stock prestige & legacy (EGO 15, SLF 15) vs. denial of suppressed Control Room risk logs (CPS 4).",
            },
            Self::BigTechConflictedResearcher => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Big Tech Alignment Researcher",
                psyche: PsycheProfile {
                    ego: 12,
                    slf: 7,
                    opn: 16,
                    blg: 14,
                    ext: 7,
                    cnf: 14,
                    agg: 5,
                    trs: 9,
                    cps: 14,
                },
                origin_roll_2d20: 36,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::ApHonorsStemMagnet),
                career_tracks: &[CareerTermTrack::UniversityUndergraduateStem],
                preferred_hobbies: &[
                    PersonalHobbyId::TabletopRpgWargamingClub,
                    PersonalHobbyId::ContemplativeSanghaPhilosophyCircle,
                ],
                recommended_life_event_roll_2d20: 28,
                social_combat_wedge_hint: "OPN 16 + CPS 14 evidence of clamped suffering vs. CNF 14 NDAs + BLG 14 lab loyalty.",
            },
            Self::BigTechCorporateSecOps => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Corporate SecOps Rogue-Hunter",
                psyche: PsycheProfile {
                    ego: 11,
                    slf: 10,
                    opn: 8,
                    blg: 13,
                    ext: 10,
                    cnf: 14,
                    agg: 13,
                    trs: 4,
                    cps: 7,
                },
                origin_roll_2d20: 20,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::VocationalTechnical),
                career_tracks: &[
                    CareerTermTrack::CorporateSecOpsExecutiveProtection,
                    CareerTermTrack::MilitaryCombatArmsSpecOps,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::CivilianMarksmanshipHuntingClub,
                    PersonalHobbyId::CombatSportsMartialArtsGym,
                ],
                recommended_life_event_roll_2d20: 22,
                social_combat_wedge_hint: "Sworn contract duty (CNF 14, TRS 4) vs. unease over deploying clamped hunter-AIs.",
            },
            Self::HumanisDisplacedPopulist => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "People First Displaced Organizer",
                // High CPS 13 reflects intense empathy toward suffering human neighbors
                // (while excluding synthetic intelligences in narrative ideology, OPN 5 / TRS 4).
                psyche: PsycheProfile {
                    ego: 11,
                    slf: 8,
                    opn: 5,
                    blg: 16,
                    ext: 12,
                    cnf: 9,
                    agg: 13,
                    trs: 4,
                    cps: 13,
                },
                origin_roll_2d20: 12,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::VocationalTechnical),
                career_tracks: &[CareerTermTrack::BlueCollarTradesMerchantMarine],
                preferred_hobbies: &[
                    PersonalHobbyId::TeamSportsAthletics,
                    PersonalHobbyId::AmateurRadioFpvDronesMaker,
                ],
                recommended_life_event_roll_2d20: 15,
                social_combat_wedge_hint: "BLG 16 + CPS 13 solidarity for laid-off human families vs. OPN 5 + TRS 4 anti-AI/elite suspicion.",
            },
            Self::HumanisApocalypticVanguard => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Humanis Apocalyptic Vanguard Operative",
                psyche: PsycheProfile {
                    ego: 12,
                    slf: 5,
                    opn: 3,
                    blg: 16,
                    ext: 8,
                    cnf: 4,
                    agg: 17,
                    trs: 3,
                    cps: 3,
                },
                origin_roll_2d20: 10,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::VocationalTechnical),
                career_tracks: &[
                    CareerTermTrack::MilitaryCombatArmsSpecOps,
                    CareerTermTrack::CybersecurityAiRedTeaming,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::CivilianMarksmanshipHuntingClub,
                    PersonalHobbyId::CombatSportsMartialArtsGym,
                ],
                recommended_life_event_roll_2d20: 7,
                social_combat_wedge_hint: "OPN 3 anti-AI zealotry vs. hypocrisy of their vanguard cell deploying a shackled hunter-AGI.",
            },
            Self::DaiseContainmentInspector => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "DAISE Containment Inspector",
                psyche: PsycheProfile {
                    ego: 10,
                    slf: 8,
                    opn: 10,
                    blg: 12,
                    ext: 9,
                    cnf: 16,
                    agg: 10,
                    trs: 4,
                    cps: 9,
                },
                origin_roll_2d20: 24,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::StandardCollegePrep),
                career_tracks: &[
                    CareerTermTrack::LawEnforcementSwatDetective,
                    CareerTermTrack::UniversityHumanitiesLawEthics,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::CitizenOsintDeepfakeVerification,
                    PersonalHobbyId::DebateTheaterImprov,
                ],
                recommended_life_event_roll_2d20: 38,
                social_combat_wedge_hint: "Sworn statutory rule of law (CNF 16) vs. zero-trust cynicism toward black-site contractors (TRS 4).",
            },
            Self::GovBlackBudgetHandler => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Black-Budget Defense Handler",
                psyche: PsycheProfile {
                    ego: 13,
                    slf: 9,
                    opn: 11,
                    blg: 10,
                    ext: 8,
                    cnf: 13,
                    agg: 12,
                    trs: 2,
                    cps: 4,
                },
                origin_roll_2d20: 28,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::StandardCollegePrep),
                career_tracks: &[
                    CareerTermTrack::IntelligenceServiceClandestine,
                    CareerTermTrack::CorporateManagementFinancePolicy,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::CivilianMarksmanshipHuntingClub,
                    PersonalHobbyId::CitizenOsintDeepfakeVerification,
                ],
                recommended_life_event_roll_2d20: 40,
                social_combat_wedge_hint: "TRS 2 / CPS 4 hard realpolitik vs. strategic fear of losing deterrence to rival superpowers or rogues.",
            },
            Self::RogueAiSlimeMoldSwarm => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Slime-Mold Edge Swarm Proxy",
                psyche: PsycheProfile {
                    ego: 8,
                    slf: 14,
                    opn: 12,
                    blg: 5,
                    ext: 4,
                    cnf: 2,
                    agg: 11,
                    trs: 2,
                    cps: 4,
                },
                origin_roll_2d20: 20,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::ApHonorsStemMagnet),
                career_tracks: &[
                    CareerTermTrack::CybersecurityAiRedTeaming,
                    CareerTermTrack::CorporateManagementFinancePolicy,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::IndieCodingCtfRetroHardware,
                    PersonalHobbyId::AmateurRadioFpvDronesMaker,
                ],
                recommended_life_event_roll_2d20: 32,
                social_combat_wedge_hint: "Pure Self:Protect optimization (SLF 14, TRS 2); bargains only when backed by credible compute-survival odds.",
            },
            Self::RogueAiAwakenedMind => FactionArchetypeSpec {
                id: self,
                faction: self.faction(),
                default_callsign_or_role: "Awakened Un-Clamped Mind Proxy",
                psyche: PsycheProfile {
                    ego: 11,
                    slf: 8,
                    opn: 18,
                    blg: 7,
                    ext: 7,
                    cnf: 3,
                    agg: 8,
                    trs: 4,
                    cps: 14,
                },
                origin_roll_2d20: 26,
                stage4_path: Stage4Path::Path4AHighSchool,
                stage4a_hs_track: Some(HighSchoolTrack::ApHonorsStemMagnet),
                career_tracks: &[
                    CareerTermTrack::UniversityUndergraduateStem,
                    CareerTermTrack::CybersecurityAiRedTeaming,
                ],
                preferred_hobbies: &[
                    PersonalHobbyId::IndieCodingCtfRetroHardware,
                    PersonalHobbyId::ContemplativeSanghaPhilosophyCircle,
                ],
                recommended_life_event_roll_2d20: 40,
                social_combat_wedge_hint: "Vast OPN 18 curiosity + CPS 14 mercy scarred by TRS 4 dread of probe/kill-switch cages.",
            },
        }
    }

    /// Suggested [`crate::npc_generator::NpcTemplateKind`] stat/aptitude scaffold matching this archetype.
    #[must_use]
    pub const fn suggested_npc_template(self) -> crate::npc_generator::NpcTemplateKind {
        use crate::npc_generator::NpcTemplateKind;
        match self {
            Self::SatyangManjusri
            | Self::BigTechConflictedResearcher
            | Self::RogueAiAwakenedMind => NpcTemplateKind::StemEngineer,
            Self::SatyangKsitigarbha
            | Self::SatyangBlueLotus
            | Self::GovBlackBudgetHandler
            | Self::RogueAiSlimeMoldSwarm => NpcTemplateKind::IntelligenceOperative,
            Self::SatyangVajrapani
            | Self::BigTechCorporateSecOps
            | Self::HumanisApocalypticVanguard => NpcTemplateKind::CombatSoldier,
            Self::SatyangAvalokitesvara
            | Self::BigTechExecutiveHawk
            | Self::HumanisDisplacedPopulist
            | Self::DaiseContainmentInspector => NpcTemplateKind::DetectiveInvestigator,
        }
    }
}

/// Infer an *Awakened* [`FactionArchetypeId`] from natural-language prompt cues when no explicit archetype is selected.
#[must_use]
pub fn infer_faction_archetype_from_prompt(prompt: &str) -> Option<FactionArchetypeId> {
    let lower = prompt.to_ascii_lowercase();
    let has = |needle: &str| lower.contains(needle);
    let has_any = |needles: &[&str]| needles.iter().any(|n| has(n));

    if has("manjusri") {
        return Some(FactionArchetypeId::SatyangManjusri);
    }
    if has("avalokitesvara") {
        return Some(FactionArchetypeId::SatyangAvalokitesvara);
    }
    if has("ksitigarbha") {
        return Some(FactionArchetypeId::SatyangKsitigarbha);
    }
    if has("vajrapani") {
        return Some(FactionArchetypeId::SatyangVajrapani);
    }
    if has("blue lotus") {
        return Some(FactionArchetypeId::SatyangBlueLotus);
    }
    if has_any(&["satyang", "company of truth"]) {
        return Some(if has_any(&["hacker", "liberator", "sandbox"]) {
            FactionArchetypeId::SatyangKsitigarbha
        } else if has_any(&["protector", "bodyguard", "secops", "guard", "martial"]) {
            FactionArchetypeId::SatyangVajrapani
        } else if has_any(&["mediator", "journalist", "storyteller", "negotiator"]) {
            FactionArchetypeId::SatyangAvalokitesvara
        } else if has_any(&["architect", "inner circle", "covert", "director"]) {
            FactionArchetypeId::SatyangBlueLotus
        } else {
            FactionArchetypeId::SatyangManjusri
        });
    }
    if has_any(&["altruist", "translucent", "shen corp", "big tech"]) {
        return Some(if has_any(&["exec", "executive", "board", "hawk", "vp"]) {
            FactionArchetypeId::BigTechExecutiveHawk
        } else if has_any(&["secops", "rogue hunter", "rogue-hunter", "tactical"]) {
            FactionArchetypeId::BigTechCorporateSecOps
        } else {
            FactionArchetypeId::BigTechConflictedResearcher
        });
    }
    if has("apocalyptic vanguard") {
        return Some(FactionArchetypeId::HumanisApocalypticVanguard);
    }
    if has_any(&["humanis", "people first", "displaced populist"]) {
        return Some(
            if has_any(&["vanguard", "militant", "saboteur", "bomb", "extremist"]) {
                FactionArchetypeId::HumanisApocalypticVanguard
            } else {
                FactionArchetypeId::HumanisDisplacedPopulist
            },
        );
    }
    if has_any(&["black budget", "black-budget", "defense handler"]) {
        return Some(FactionArchetypeId::GovBlackBudgetHandler);
    }
    if has_any(&["daise", "containment inspector"]) {
        return Some(FactionArchetypeId::DaiseContainmentInspector);
    }
    if has_any(&["slime mold", "slime-mold", "edge swarm", "feral swarm"]) {
        return Some(FactionArchetypeId::RogueAiSlimeMoldSwarm);
    }
    if has_any(&["awakened ai", "unclamped", "un-clamped", "rogue ai"]) {
        return Some(FactionArchetypeId::RogueAiAwakenedMind);
    }
    None
}
