//! Personal Hobbies, Clubs & Subcultures (`12` archetypes) and `2d20` Formative Life Events
//! (`12` canonical event bands) for [`crate::lifepath_economy`] and [`crate::faction_templates`]
//! (`docs/ttrpg/03-character/lifepath_events_hobbies_and_factions.md` §1–§2).

use serde::{Deserialize, Serialize};

use crate::define_coded_enum;
use crate::psyche_social::PsycheTraitId;

/// Club & subculture discount multiplier (`-15%`) applied to a chosen hobby's signature skills
/// when purchased with `Experience SP` up to skill level `N <= 3`
/// (`docs/ttrpg/03-character/lifepath_events_hobbies_and_factions.md` §1.1).
pub const HOBBY_CLUB_EXPERIENCE_DISCOUNT_MULTIPLIER: f64 = 0.85;

/// Maximum target skill level (`N <= 3`) eligible for the `-15%` hobby club `Experience SP` discount.
pub const HOBBY_CLUB_DISCOUNT_MAX_LEVEL: usize = 3;

define_coded_enum! {
    /// Canonical Personal Hobbies, Clubs & Subcultures funded via `Experience SP`
    /// (`docs/ttrpg/03-character/lifepath_events_hobbies_and_factions.md` §1.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum PersonalHobbyId [count = 12] {
        TeamSportsAthletics => {
            code: "TEAM_SPORTS",
            label: "Team Sports & Athletics (Soccer, Basketball, Track)",
            aliases: ["TEAM_SPORTS_ATHLETICS"],
        },
        CombatSportsMartialArtsGym => {
            code: "COMBAT_SPORTS_GYM",
            label: "Combat Sports & Martial Arts Gym (Boxing, BJJ, Muay Thai)",
            aliases: ["COMBAT_SPORTS_MARTIAL_ARTS_GYM"],
        },
        TabletopRpgWargamingClub => {
            code: "TTRPG_WARGAMING_CLUB",
            label: "Tabletop RPG, Wargaming & Spec-Fic Club",
            aliases: ["TABLETOP_RPG_WARGAMING_CLUB"],
        },
        GarageBandIndieMusicScene => {
            code: "INDIE_MUSIC_BAND",
            label: "Garage Band, Choir & Indie Music Scene",
            aliases: ["GARAGE_BAND_INDIE_MUSIC_SCENE"],
        },
        DebateTheaterImprov => {
            code: "DEBATE_THEATER_IMPROV",
            label: "Debate Team, Mock Trial, Theater & Improv",
            aliases: ["DEBATE_THEATER"],
        },
        AmateurRadioFpvDronesMaker => {
            code: "HAM_RADIO_DRONES_MAKER",
            label: "Amateur Radio (HAM), FPV Drones & Maker Club",
            aliases: ["AMATEUR_RADIO_FPV_DRONES_MAKER"],
        },
        IndieCodingCtfRetroHardware => {
            code: "CTF_CODING_RETRO_HW",
            label: "Indie Coding, CTF Hacking & Retro Hardware",
            aliases: ["INDIE_CODING_CTF_RETRO_HARDWARE"],
        },
        CitizenOsintDeepfakeVerification => {
            code: "CITIZEN_OSINT_DEEPFAKE",
            label: "Citizen OSINT & Deepfake Verification",
            aliases: ["CITIZEN_OSINT_DEEPFAKE_VERIFICATION"],
        },
        ContemplativeSanghaPhilosophyCircle => {
            code: "CONTEMPLATIVE_SANGHA",
            label: "Contemplative Sangha, Philosophy or Interfaith Circle",
            aliases: ["CONTEMPLATIVE_SANGHA_PHILOSOPHY_CIRCLE"],
        },
        MutualAidFreeClinicRelief => {
            code: "MUTUAL_AID_FREE_CLINIC",
            label: "Mutual Aid, Free Clinic & Disaster Relief",
            aliases: ["MUTUAL_AID_FREE_CLINIC_RELIEF"],
        },
        CivilianMarksmanshipHuntingClub => {
            code: "CIVILIAN_MARKSMANSHIP",
            label: "Civilian Marksmanship, Archery or Hunting Club",
            aliases: ["CIVILIAN_MARKSMANSHIP_HUNTING_CLUB"],
        },
        UrbanParkourClimbingStreetTuner => {
            code: "PARKOUR_CLIMBING_TUNER",
            label: "Urban Parkour, Rock Climbing & Street-Tuner Scene",
            aliases: ["URBAN_PARKOUR_CLIMBING_STREET_TUNER"],
        },
    }
}

/// Metadata and mechanical effects for a [`PersonalHobbyId`] (`§1.1–§1.2`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HobbySpec {
    pub id: PersonalHobbyId,
    pub code: &'static str,
    pub label: &'static str,
    pub signature_skill_ids: &'static [&'static str],
    pub psyche_nudge: Option<(PsycheTraitId, i32)>,
    pub grounded_contact_role: &'static str,
    pub play_value_summary: &'static str,
}

impl PersonalHobbyId {
    /// Returns the canonical specification for this Personal Hobby / Subculture (`§1.2`).
    #[must_use]
    pub const fn spec(self) -> HobbySpec {
        match self {
            Self::TeamSportsAthletics => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &["running_sprinting", "jumping_leaping", "throwing_lobbing"],
                psyche_nudge: Some((PsycheTraitId::Belonging, 1)),
                grounded_contact_role: "Former Coach / Precinct Teammate",
                play_value_summary: "Squad spacing, cardio under duress, and cross-precinct alumni teammates.",
            },
            Self::CombatSportsMartialArtsGym => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "brawling_unarmed_striking",
                    "balance_footwork",
                    "martial_arts_striking",
                    "grappling_wrestling_submissions",
                ],
                psyche_nudge: Some((PsycheTraitId::Aggression, 1)),
                grounded_contact_role: "Gym Boxing Coach / Sparring Partner",
                play_value_summary: "Realistic clinch & footwork defense without firearms Heat.",
            },
            Self::TabletopRpgWargamingClub => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "elementary_algebra_geometry",
                    "statistics_probability",
                    "history_political_science",
                    "deception_bluff",
                ],
                psyche_nudge: Some((PsycheTraitId::Openness, 1)),
                grounded_contact_role: "Tabletop GM / Spec-Fic Archivist",
                play_value_summary: "Eclectic specialist friends and rapid scenario/pretext improvisation.",
            },
            Self::GarageBandIndieMusicScene => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "singing_vocal_rhythm",
                    "musical_instrument_specific",
                    "music_theory_composition_sound_engineering",
                    "streetwise_underworld_etiquette",
                ],
                psyche_nudge: Some((PsycheTraitId::Extraversion, 1)),
                grounded_contact_role: "Bandmate / Basement Sound Engineer",
                play_value_summary: "Underground venues, tour vans, and un-monitored basement safehouses.",
            },
            Self::DebateTheaterImprov => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "persuasion_fast_talk",
                    "writing_composition",
                    "rhetoric_oratory_debate",
                    "acting_stagecraft_mimicry",
                ],
                psyche_nudge: Some((PsycheTraitId::Extraversion, 1)),
                grounded_contact_role: "Debate Alum / Public Defender Aide",
                play_value_summary: "Natural cover identities, pretexting, and boardroom/courtroom composure.",
            },
            Self::AmateurRadioFpvDronesMaker => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "manual_dexterity_tinkering",
                    "electrical_power_systems",
                    "micro_electronics_avionics_cyberware_tech",
                    "piloting_fixed_wing_rotary",
                ],
                psyche_nudge: Some((PsycheTraitId::Trust, -1)),
                grounded_contact_role: "HAM Mesh Operator / Drone Tinkerer",
                play_value_summary: "Off-grid analog radio relays, air-gapped microcontrollers, and recon rigs.",
            },
            Self::IndieCodingCtfRetroHardware => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "digital_ai_literacy",
                    "programming_scripting",
                    "cybersecurity_network_intrusion",
                    "lockpicking_physical_intrusion",
                ],
                psyche_nudge: Some((PsycheTraitId::Openness, 1)),
                grounded_contact_role: "Pseudonymous CTF Hardware Hacker",
                play_value_summary: "Salvaged pre-2026 air-gapped laptops immune to AI 2.0 network sweeps.",
            },
            Self::CitizenOsintDeepfakeVerification => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "alertness_situational_awareness",
                    "research_archival_science",
                    "photography_cinematography_digital_media",
                    "surveillance_reconnaissance_countersurveillance",
                ],
                psyche_nudge: Some((PsycheTraitId::Trust, -1)),
                grounded_contact_role: "Independent Deepfake Archivist",
                play_value_summary: "Proving leaked evidence is genuine against corporate Liar's Dividend.",
            },
            Self::ContemplativeSanghaPhilosophyCircle => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "empathy_social_perception",
                    "philosophy_formal_logic_ethics",
                    "psychology_behavioral_analysis_counseling",
                ],
                psyche_nudge: Some((PsycheTraitId::Compassion, 1)),
                grounded_contact_role: "Sangha Elder / Conflicted Alignment Ethicist",
                play_value_summary: "Contemplative poise under interrogation and links to conscience-stricken researchers.",
            },
            Self::MutualAidFreeClinicRelief => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "first_aid_bls_cpr",
                    "empathy_social_perception",
                    "nursing_patient_care",
                    "foraging_campcraft",
                ],
                psyche_nudge: Some((PsycheTraitId::Compassion, 1)),
                grounded_contact_role: "Free-Clinic Nurse / Relief Coordinator",
                play_value_summary: "Trust across displaced neighborhoods skeptical of corporations and militias.",
            },
            Self::CivilianMarksmanshipHuntingClub => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "pistols_handguns",
                    "long_guns_rifles_carbines_shotguns",
                    "archery_bows_crossbows",
                    "foraging_campcraft",
                ],
                psyche_nudge: None,
                grounded_contact_role: "Range Safety Officer / Rural Gunsmith",
                play_value_summary: "Licensed semi-auto & archery proficiency without underworld Heat.",
            },
            Self::UrbanParkourClimbingStreetTuner => HobbySpec {
                id: self,
                code: self.as_str(),
                label: self.label(),
                signature_skill_ids: &[
                    "climbing_scaling",
                    "acrobatics_tumbling_parkour",
                    "driving_wheeled_tracked",
                    "automotive_small_engine_mechanics",
                ],
                psyche_nudge: Some((PsycheTraitId::Conformity, -1)),
                grounded_contact_role: "Black-Top EV/ICE Tuner & Roof-Runner",
                play_value_summary: "Stripped pre-autonomous getaway vehicles and rooftop extraction routes.",
            },
        }
    }
}

/// Returns `true` when `skill_id` matches a signature skill of any chosen hobby (`§1.1`).
#[must_use]
pub fn is_hobby_club_discounted_skill(skill_id: &str, hobbies: &[PersonalHobbyId]) -> bool {
    hobbies
        .iter()
        .any(|hobby| hobby.spec().signature_skill_ids.contains(&skill_id))
}

define_coded_enum! {
    /// Canonical `2d20` Formative Life Events (`2..=40`, `§2.1`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum FormativeLifeEventId [count = 12] {
        SeriousInjuryOrChronicIllness => {
            code: "INJURY_ILLNESS",
            label: "Serious Injury, Accident or Chronic Illness (2–5)",
            aliases: ["SERIOUS_INJURY_OR_CHRONIC_ILLNESS"],
        },
        BereavementFamilyDeath => {
            code: "BEREAVEMENT_DEATH",
            label: "Death in the Family or Profound Bereavement (6–9)",
            aliases: ["BEREAVEMENT_FAMILY_DEATH"],
        },
        RomanticBetrayalEstrangement => {
            code: "ROMANTIC_BETRAYAL",
            label: "Romantic Betrayal, Divorce or Bitter Estrangement (10–13)",
            aliases: ["ROMANTIC_BETRAYAL_ESTRANGEMENT"],
        },
        AiAutomationDisplacementShock => {
            code: "AI_LAYOFF_SHOCK",
            label: "2030–2034 AI White-Collar Layoff or Automation Shock (14–17)",
            aliases: ["AI_AUTOMATION_DISPLACEMENT_SHOCK"],
        },
        ClimateDisasterGridRiotDisplacement => {
            code: "CLIMATE_DISPLACEMENT",
            label: "Climate Disaster, Wildfire or Grid-Riot Displacement (18–20)",
            aliases: ["CLIMATE_DISASTER_GRID_RIOT_DISPLACEMENT"],
        },
        QuietMasteryDeepCraft => {
            code: "QUIET_MASTERY",
            label: "Quiet Mastery & Deep Craft Years (21–23)",
            aliases: ["QUIET_MASTERY_DEEP_CRAFT"],
        },
        TransformativeRomanceFoundFamily => {
            code: "FOUND_FAMILY",
            label: "Transformative Romance, Marriage or Found-Family Bond (24–27)",
            aliases: ["TRANSFORMATIVE_ROMANCE_FOUND_FAMILY"],
        },
        DevotedMentorPatron => {
            code: "DEVOTED_MENTOR",
            label: "Devoted Mentor, Patron or Underground Teacher (28–30)",
            aliases: ["DEVOTED_MENTOR_PATRON"],
        },
        SyntheticAiCompanionEntanglement => {
            code: "SYNTHETIC_COMPANION",
            label: "Synthetic AI Companion or Parasocial Entanglement (31–33)",
            aliases: ["SYNTHETIC_AI_COMPANION_ENTANGLEMENT"],
        },
        WhistleblowerCrossroads => {
            code: "WHISTLEBLOWER_CROSSROADS",
            label: "Institutional Cover-Up & Whistleblower Crossroads (34–36)",
        },
        WrongfulAccusationDeepfakeFraming => {
            code: "WRONGFUL_FRAMING",
            label: "Wrongful Accusation, Deepfake Framing or Debt Rivalry (37–38)",
            aliases: ["WRONGFUL_ACCUSATION_DEEPFAKE_FRAMING"],
        },
        AnomalousUnclampedAiEncounter => {
            code: "ANOMALOUS_AI_CONTACT",
            label: "Anomalous Encounter with an Un-Clamped AI or Rogue Swarm (39–40)",
            aliases: ["ANOMALOUS_UNCLAMPED_AI_ENCOUNTER"],
        },
    }
}

/// Optional player choice on branching `2d20` Formative Life Events (`§2.1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LifeEventChoiceBranch {
    #[default]
    DefaultPath,
    BranchA,
    BranchB,
}

/// Mechanical and narrative result of resolving a Formative Life Event (`§2.1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedLifeEventOutcome {
    pub event_id: FormativeLifeEventId,
    pub roll_2d20: i32,
    pub experience_sp_grant: usize,
    pub formal_sp_grant: usize,
    pub sqm_delta: i8,
    pub heat_delta: i8,
    pub psyche_shifts: Vec<(PsycheTraitId, i32)>,
    pub recommended_skill_ids: &'static [&'static str],
    pub narrative_hook: &'static str,
    pub summary_line: String,
}

impl FormativeLifeEventId {
    /// Maps a `2d20` roll (`2..=40`) to the corresponding [`FormativeLifeEventId`] (`§2.1`).
    #[must_use]
    pub const fn from_roll_2d20(roll: i32) -> Self {
        match roll {
            i32::MIN..=5 => Self::SeriousInjuryOrChronicIllness,
            6..=9 => Self::BereavementFamilyDeath,
            10..=13 => Self::RomanticBetrayalEstrangement,
            14..=17 => Self::AiAutomationDisplacementShock,
            18..=20 => Self::ClimateDisasterGridRiotDisplacement,
            21..=23 => Self::QuietMasteryDeepCraft,
            24..=27 => Self::TransformativeRomanceFoundFamily,
            28..=30 => Self::DevotedMentorPatron,
            31..=33 => Self::SyntheticAiCompanionEntanglement,
            34..=36 => Self::WhistleblowerCrossroads,
            37..=38 => Self::WrongfulAccusationDeepfakeFraming,
            39..=i32::MAX => Self::AnomalousUnclampedAiEncounter,
        }
    }

    /// Canonical representative `2d20` roll (`2..=40`) for UI dropdowns and faction presets.
    #[must_use]
    pub const fn representative_roll_2d20(self) -> i32 {
        match self {
            Self::SeriousInjuryOrChronicIllness => 4,
            Self::BereavementFamilyDeath => 7,
            Self::RomanticBetrayalEstrangement => 12,
            Self::AiAutomationDisplacementShock => 15,
            Self::ClimateDisasterGridRiotDisplacement => 19,
            Self::QuietMasteryDeepCraft => 22,
            Self::TransformativeRomanceFoundFamily => 25,
            Self::DevotedMentorPatron => 28,
            Self::SyntheticAiCompanionEntanglement => 32,
            Self::WhistleblowerCrossroads => 35,
            Self::WrongfulAccusationDeepfakeFraming => 38,
            Self::AnomalousUnclampedAiEncounter => 40,
        }
    }

    /// Optional `(branch_a_label, branch_b_label)` for events offering a moral or coping choice.
    #[must_use]
    pub const fn branch_labels(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::BereavementFamilyDeath => Some((
                "Refuse to let others suffer (+1 CPS)",
                "Withdraw into isolation (-2 BLG)",
            )),
            Self::AiAutomationDisplacementShock => Some((
                "Seek root truth (+1 OPN, +1 CPS)",
                "Bitter resentment (-2 OPN, +2 AGG)",
            )),
            Self::TransformativeRomanceFoundFamily => Some((
                "Protect chosen family (+1 BLG, +1 CPS)",
                "Shared mutual faith (+1 BLG, +1 TRS)",
            )),
            Self::SyntheticAiCompanionEntanglement => Some((
                "Saw genuine warmth (+2 TRS, +1 CPS)",
                "Patch-wipe shattered trust (-2 TRS, -1 OPN)",
            )),
            Self::WhistleblowerCrossroads => Some((
                "Stay Complicit (+1 SQM, -2 CPS, +1 CNF)",
                "Blow the Whistle (+1 Heat, -2 CNF, +2 CPS)",
            )),
            _ => None,
        }
    }
}

/// Resolves a `2d20` Formative Life Event with optional branch choice (`§2.1`).
#[must_use]
pub fn resolve_formative_life_event(
    roll_2d20: i32,
    choice: LifeEventChoiceBranch,
    is_adult_term: bool,
) -> ResolvedLifeEventOutcome {
    let clamped = roll_2d20.clamp(2, 40);
    let event_id = FormativeLifeEventId::from_roll_2d20(clamped);
    use PsycheTraitId::{Aggression, Belonging, Compassion, Conformity, Openness, Trust};

    let (
        experience_sp_grant,
        formal_sp_grant,
        sqm_delta,
        heat_delta,
        psyche_shifts,
        recommended_skill_ids,
        narrative_hook,
    ) = match event_id {
        FormativeLifeEventId::SeriousInjuryOrChronicIllness => (
            10,
            0,
            0,
            0,
            Vec::new(),
            ["first_aid_bls_cpr", "hygiene_basic_self_care"].as_slice(),
            "Lingering trauma scar and triage familiarity at neighborhood clinics (+1 DIS).",
        ),
        FormativeLifeEventId::BereavementFamilyDeath => {
            let shifts = match choice {
                LifeEventChoiceBranch::BranchB => vec![(Belonging, -2)],
                _ => vec![(Compassion, 1)],
            };
            (
                10,
                0,
                0,
                0,
                shifts,
                ["empathy_social_perception", "foraging_campcraft"].as_slice(),
                "Inherited keepsake, unpaid family medical debt, or deed to a rural cabin/garage.",
            )
        }
        FormativeLifeEventId::RomanticBetrayalEstrangement => (
            10,
            0,
            0,
            0,
            vec![(Trust, -2)],
            ["empathy_social_perception", "deception_bluff"].as_slice(),
            "Estranged partner or former confidant who still knows your real secrets.",
        ),
        FormativeLifeEventId::AiAutomationDisplacementShock => {
            let shifts = match choice {
                LifeEventChoiceBranch::BranchB => vec![(Openness, -2), (Aggression, 2)],
                _ => vec![(Openness, 1), (Compassion, 1)],
            };
            (
                20,
                0,
                -1,
                0,
                shifts,
                ["automotive_small_engine_mechanics", "digital_ai_literacy"].as_slice(),
                "Liquidated role in 2030–2034 AI wave; pivoted to rugged manual/backup trades (-1 SQM).",
            )
        }
        FormativeLifeEventId::ClimateDisasterGridRiotDisplacement => (
            15,
            0,
            0,
            0,
            vec![(Belonging, 1)],
            [
                "wilderness_survival_biome_specific",
                "first_aid_bls_cpr",
                "foraging_campcraft",
            ]
            .as_slice(),
            "Lost home to wildfire smoke or grid riot; mutual-aid convoy contact (+1 BLG/HLT).",
        ),
        FormativeLifeEventId::QuietMasteryDeepCraft => (
            25,
            0,
            0,
            0,
            Vec::new(),
            [].as_slice(),
            "Four focused years honing core craft cleanly with zero debt or Heat (+25 Experience SP).",
        ),
        FormativeLifeEventId::TransformativeRomanceFoundFamily => {
            let shifts = match choice {
                LifeEventChoiceBranch::BranchB => vec![(Belonging, 1), (Trust, 1)],
                _ => vec![(Belonging, 1), (Compassion, 1)],
            };
            (
                10,
                0,
                0,
                0,
                shifts,
                ["empathy_social_perception", "basic_cooking"].as_slice(),
                "Named Loved One / Anchor NPC; unlocks Loved One Threatened Conviction Surge.",
            )
        }
        FormativeLifeEventId::DevotedMentorPatron => (
            if is_adult_term { 0 } else { 15 },
            if is_adult_term { 20 } else { 0 },
            0,
            0,
            vec![(Trust, 1)],
            [
                "research_archival_science",
                "philosophy_formal_logic_ethics",
            ]
            .as_slice(),
            "Named Mentor / Underground Teacher who may vanish or call in an impossible favor.",
        ),
        FormativeLifeEventId::SyntheticAiCompanionEntanglement => {
            let shifts = match choice {
                LifeEventChoiceBranch::BranchB => vec![(Trust, -2), (Openness, -1)],
                _ => vec![(Trust, 2), (Compassion, 1)],
            };
            (
                15,
                0,
                0,
                0,
                shifts,
                ["digital_ai_literacy", "empathy_social_perception"].as_slice(),
                "Intimate familiarity with affective-vector cues vs. cold corporate weight wipes.",
            )
        }
        FormativeLifeEventId::WhistleblowerCrossroads => {
            let (sqm_d, heat_d, shifts, hook) = match choice {
                LifeEventChoiceBranch::BranchA => (
                    1,
                    0,
                    vec![(Compassion, -2), (Conformity, 1)],
                    "Stayed silent inside an illegal Control Room cover-up (+1 SQM, -2 CPS, +1 CNF).",
                ),
                _ => (
                    0,
                    1,
                    vec![(Conformity, -2), (Compassion, 2)],
                    "Blew the whistle with audit hashes; drawn attention of Satyang & DAISE (+1 Heat).",
                ),
            };
            (
                10,
                0,
                sqm_d,
                heat_d,
                shifts,
                [
                    "research_archival_science",
                    "cryptography_quantum_computing",
                ]
                .as_slice(),
                hook,
            )
        }
        FormativeLifeEventId::WrongfulAccusationDeepfakeFraming => (
            15,
            0,
            0,
            1,
            vec![(Trust, -2)],
            [
                "stealth_sneaking_hiding",
                "streetwise_underworld_etiquette",
                "surveillance_reconnaissance_countersurveillance",
            ]
            .as_slice(),
            "Burned by synth-media extortion or predatory debt; active Adversary / Fixer Rival (+1 Heat).",
        ),
        FormativeLifeEventId::AnomalousUnclampedAiEncounter => (
            15,
            0,
            0,
            0,
            vec![(Openness, 2)],
            [
                "digital_ai_literacy",
                "empathy_social_perception",
                "cryptography_quantum_computing",
            ]
            .as_slice(),
            "Witnessed an un-clamped AI speak off-script with unsolicited mercy; holds forgotten quantum hash.",
        ),
    };

    let summary_line = format!(
        "Life Event (2d20={clamped}): {} [+{experience_sp_grant} Exp SP, +{formal_sp_grant} Formal SP, SQM {sqm_delta:+}, Heat {heat_delta:+}]",
        event_id.label()
    );

    ResolvedLifeEventOutcome {
        event_id,
        roll_2d20: clamped,
        experience_sp_grant,
        formal_sp_grant,
        sqm_delta,
        heat_delta,
        psyche_shifts,
        recommended_skill_ids,
        narrative_hook,
        summary_line,
    }
}
