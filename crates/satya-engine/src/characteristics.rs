//! Canonical 25+ Characteristics, Senses, Psyche & 9 Domain Aptitudes (`03-character/characteristics_and_aptitudes.md`).
//!
//! Implements:
//! - Spectrum (`0.0–100.0`) <-> Controlling Scale (`1–20`) conversion and Population Tier taxonomy (§2)
//! - All Physical (`SPD`, `FOR`, `DEX`, `KIN`, `HLT`, `FIT`, `MAS`, `HGT`), Mental (`MEM`, `ANA`, `INS`,
//!   `CRE`, `DIS`, `FOC`), Social (`COM`, `PRE`, `ELO`), Sensory (`SIG`, `HEA`, `TAC`, `OLF`, `TAS`, `ITU`),
//!   and Psyche (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`) characteristics (§3–§7)
//! - All 10 Secondary & Derived Characteristics (`POW`, `MMF`, `Melee Reach Index`, `D_cog`, `OODA`,
//!   `AWR`, `FT`, `WSR`, `CMP`, and `C_study`) (§8)
//! - 9 Domain Aptitudes (`APT-QNT` .. `APT-SOC`, `-2` to `+3`) with prerequisite relief and training time scaling (§9)

use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};

use crate::define_coded_enum;

/// Category of a primary characteristic in the Satya system (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §1 & §3–§7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacteristicCategory {
    Physical,
    Mental,
    Social,
    Sensory,
    Psyche,
}

define_coded_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum CharacteristicCode [count = 30] {
        SPD => { code: "SPD", label: "Speed" },
        FOR => { code: "FOR", label: "Force" },
        DEX => { code: "DEX", label: "Dexterity" },
        KIN => { code: "KIN", label: "Kinesthesis", aliases: ["KINESTHESIA"] },
        HLT => { code: "HLT", label: "Health" },
        FIT => { code: "FIT", label: "Fitness" },
        /// Mass in kilograms (`MAS`, Physical §3; `70.0 kg` baseline).
        MAS => { code: "MAS", label: "Mass (kg)" },
        /// Height in meters (`HGT`, Physical §3; `1.75 m` baseline).
        HGT => { code: "HGT", label: "Height (m)" },
        MEM => { code: "MEM", label: "Memory" },
        ANA => { code: "ANA", label: "Analysis" },
        INS => { code: "INS", label: "Insight" },
        CRE => { code: "CRE", label: "Creativity" },
        DIS => { code: "DIS", label: "Discipline" },
        FOC => { code: "FOC", label: "Focus" },
        COM => { code: "COM", label: "Comeliness" },
        PRE => { code: "PRE", label: "Presence" },
        ELO => { code: "ELO", label: "Eloquence" },
        SIG => { code: "SIG", label: "Sight" },
        HEA => { code: "HEA", label: "Hearing" },
        TAC => { code: "TAC", label: "Tactile" },
        OLF => { code: "OLF", label: "Olfactory", aliases: ["OLFACTION"] },
        TAS => { code: "TAS", label: "Taste" },
        ITU => { code: "ITU", label: "Intuition" },
        /// Ego (`EGO`, Psyche §7.2: Humble / Self-Effacing vs. Proud / Grandiose / Narcissistic).
        EGO => { code: "EGO", label: "Ego", aliases: ["EGOISM"] },
        /// Selfishness (`SLF`, Psyche §7.2: Altruistic / Self-Sacrificing vs. Entitled / Mercenary / Acquisitive).
        SLF => { code: "SLF", label: "Selfishness", aliases: ["SELF_ESTEEM"] },
        /// Openness (`OPN`, Psyche §7.2: Dogmatic / Rigid vs. Curious / Receptive / Open-Minded).
        OPN => { code: "OPN", label: "Openness" },
        /// Belongingness (`BLG`, Psyche §7.2: Detached / Lone Wolf vs. Tribal / Group-Dependent).
        BLG => { code: "BLG", label: "Belongingness", aliases: ["BELONGING"] },
        /// Extraversion (`EXT`, Psyche §7.2: Introverted / Solitary vs. Gregarious / Thrill-Seeking).
        EXT => { code: "EXT", label: "Extraversion" },
        /// Conformity (`CNF`, Psyche §7.2: Rebellious / Iconoclastic vs. Institutionalist / By-the-Book).
        CNF => { code: "CNF", label: "Conformity" },
        /// Aggressiveness (`AGG`, Psyche §7.2: Passive / Conflict-Averse vs. Combative / Dominant).
        AGG => { code: "AGG", label: "Aggressiveness", aliases: ["AGGRESSION"] },
    }
}

impl CharacteristicCode {
    #[must_use]
    pub const fn category(self) -> CharacteristicCategory {
        match self {
            Self::SPD
            | Self::FOR
            | Self::DEX
            | Self::KIN
            | Self::HLT
            | Self::FIT
            | Self::MAS
            | Self::HGT => CharacteristicCategory::Physical,
            Self::MEM | Self::ANA | Self::INS | Self::CRE | Self::DIS | Self::FOC => {
                CharacteristicCategory::Mental
            }
            Self::COM | Self::PRE | Self::ELO => CharacteristicCategory::Social,
            Self::SIG | Self::HEA | Self::TAC | Self::OLF | Self::TAS | Self::ITU => {
                CharacteristicCategory::Sensory
            }
            Self::EGO | Self::SLF | Self::OPN | Self::BLG | Self::EXT | Self::CNF | Self::AGG => {
                CharacteristicCategory::Psyche
            }
        }
    }
}

/// Population distribution tier on the `1–20` Controlling Scale (`0.0–100.0` Spectrum)
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §2 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PopulationTier {
    /// `1–3` (`0.0–17.4`): Severely Impaired / Extreme Low Pole
    SeverelyImpaired,
    /// `4–6` (`17.5–32.4`): Deficient / Strong Low Pole
    BelowAverage,
    /// `7–8` (`32.5–42.4`): Below Average / Moderate Low Pole
    LowAverage,
    /// `9–11` (`42.5–57.4`): Human Average (`10 = 50.0`)
    HumanMedian,
    /// `12–13` (`57.5–67.4`): Above Average / Gifted
    TrainedCompetent,
    /// `14–15` (`67.5–77.4`): Exceptional / Elite Professional
    GiftedVeteran,
    /// `16–17` (`77.5–87.4`): World-Class / Olympic / Genius
    WorldClass,
    /// `18` (`87.5–92.4`): Peak Unaugmented Human Cap
    PeakUnaugmented,
    /// `19–20` (`92.5–100.0`): Augmented / Transhuman Apex
    AugmentedApex,
}

impl PopulationTier {
    /// Classify a `1–20` controlling characteristic score into its canonical Population Tier (§2).
    #[must_use]
    pub const fn from_controlling_scale(c: i32) -> Self {
        match c {
            ..=3 => Self::SeverelyImpaired,
            4..=6 => Self::BelowAverage,
            7..=8 => Self::LowAverage,
            9..=11 => Self::HumanMedian,
            12..=13 => Self::TrainedCompetent,
            14..=15 => Self::GiftedVeteran,
            16..=17 => Self::WorldClass,
            18 => Self::PeakUnaugmented,
            19.. => Self::AugmentedApex,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SeverelyImpaired => "Severely Impaired / Extreme Low Pole",
            Self::BelowAverage => "Deficient / Strong Low Pole",
            Self::LowAverage => "Below Average / Moderate Low Pole",
            Self::HumanMedian => "Human Average",
            Self::TrainedCompetent => "Above Average / Gifted",
            Self::GiftedVeteran => "Exceptional / Elite Professional",
            Self::WorldClass => "World-Class / Olympic / Genius",
            Self::PeakUnaugmented => "Peak Unaugmented Human Cap",
            Self::AugmentedApex => "Augmented / Transhuman Apex",
        }
    }

    /// Base Personal Target Number ($\text{TN}_P = 31 - C$) for a single characteristic score `c` (§2).
    #[must_use]
    pub const fn base_tn(c: i32) -> i32 {
        let clamped = if c < 1 {
            1
        } else if c > 20 {
            20
        } else {
            c
        };
        31 - clamped
    }
}

/// Convert a continuous `0.0–100.0` Character Spectrum value to the `1–20` Controlling Scale
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §2).
///
/// `5.0` spectrum points = `1` d20 scale step (`50.0 -> 10`).
#[must_use]
pub fn spectrum_to_controlling_scale(spectrum: f64) -> i32 {
    let clamped = spectrum.clamp(0.0, 100.0);
    let step = (clamped / 5.0).round() as i32;
    step.clamp(1, 20)
}

/// Convert a `1–20` Controlling Scale integer to its canonical `0.0–100.0` Spectrum midpoint (`C * 5.0`) (§2).
#[must_use]
pub fn controlling_scale_to_spectrum(scale: i32) -> f64 {
    f64::from(scale.clamp(1, 20)) * 5.0
}

/// Bipolar disposition pole for the 7 Psyche characteristics (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PsychePole {
    /// `1–7`: Low Pole
    LowPole,
    /// `8–12`: Balanced / Pragmatic
    Balanced,
    /// `13–20`: High Pole
    HighPole,
}

impl PsychePole {
    /// Classify a `1–20` Psyche score into `LowPole (1–7)`, `Balanced (8–12)`, or `HighPole (13–20)` (§7.2).
    #[must_use]
    pub const fn from_score(score: i32) -> Self {
        match score {
            ..=7 => Self::LowPole,
            8..=12 => Self::Balanced,
            13.. => Self::HighPole,
        }
    }
}

/// Return the canonical Low Pole / Balanced / High Pole descriptor for a Psyche characteristic
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §7.2 Table).
#[must_use]
pub const fn psyche_pole_descriptor(code: CharacteristicCode, score: i32) -> &'static str {
    let pole = PsychePole::from_score(score);
    match (code, pole) {
        (CharacteristicCode::EGO, PsychePole::LowPole) => "Humble / Self-Effacing / Servile",
        (CharacteristicCode::EGO, PsychePole::Balanced) => "Grounded Self-Respect",
        (CharacteristicCode::EGO, PsychePole::HighPole) => "Proud / Grandiose / Narcissistic",

        (CharacteristicCode::SLF, PsychePole::LowPole) => "Altruistic / Self-Sacrificing",
        (CharacteristicCode::SLF, PsychePole::Balanced) => "Pragmatic Fairness",
        (CharacteristicCode::SLF, PsychePole::HighPole) => "Entitled / Mercenary / Acquisitive",

        (CharacteristicCode::OPN, PsychePole::LowPole) => "Dogmatic / Rigid / Closed-Minded",
        (CharacteristicCode::OPN, PsychePole::Balanced) => "Discerning",
        (CharacteristicCode::OPN, PsychePole::HighPole) => "Curious / Receptive / Open-Minded",

        (CharacteristicCode::BLG, PsychePole::LowPole) => "Detached / Lone Wolf",
        (CharacteristicCode::BLG, PsychePole::Balanced) => "Healthy Camaraderie",
        (CharacteristicCode::BLG, PsychePole::HighPole) => "Tribal / Group-Dependent",

        (CharacteristicCode::EXT, PsychePole::LowPole) => "Introverted / Solitary / Guarded",
        (CharacteristicCode::EXT, PsychePole::Balanced) => "Ambivert",
        (CharacteristicCode::EXT, PsychePole::HighPole) => "Gregarious / Thrill-Seeking",

        (CharacteristicCode::CNF, PsychePole::LowPole) => "Rebellious / Iconoclastic / Maverick",
        (CharacteristicCode::CNF, PsychePole::Balanced) => "Adaptable",
        (CharacteristicCode::CNF, PsychePole::HighPole) => "Institutionalist / By-the-Book",

        (CharacteristicCode::AGG, PsychePole::LowPole) => "Passive / Conflict-Averse / Gentle",
        (CharacteristicCode::AGG, PsychePole::Balanced) => "Measured Assertiveness",
        (CharacteristicCode::AGG, PsychePole::HighPole) => {
            "Combative / Dominant / Hyper-Competitive"
        }

        _ => "Non-Psyche Characteristic",
    }
}

define_coded_enum! {
    /// The 9 canonical Domain Aptitudes (`APT-*`, ranging from `-2` to `+3`)
    /// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §9 Table).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum AptitudeCode [count = 9] {
        AptQnt => { code: "APT-QNT", label: "Quantitative & Mathematical", aliases: ["QNT"] },
        AptMec => { code: "APT-MEC", label: "Mechanical & Spatial", aliases: ["MEC"] },
        AptCmp => { code: "APT-CMP", label: "Computational & Algorithmic", aliases: ["CMP"] },
        AptBio => { code: "APT-BIO", label: "Biomedical & Chemical", aliases: ["BIO"] },
        AptLng => { code: "APT-LNG", label: "Linguistic & Humanities", aliases: ["LNG"] },
        AptKin => { code: "APT-KIN", label: "Kinesthetic & Somatic", aliases: ["KIN"] },
        AptFin => { code: "APT-FIN", label: "Fine-Motor & Tactile", aliases: ["FIN"] },
        AptAud => { code: "APT-AUD", label: "Auditory & Musical", aliases: ["AUD"] },
        AptSoc => { code: "APT-SOC", label: "Interpersonal & Street", aliases: ["SOC"] },
    }
}

macro_rules! impl_aptitudes_struct {
    ($($(#[$meta:meta])* $Variant:ident => $field:ident),+ $(,)?) => {
        /// Character's 9 Domain Aptitude scores (`-2..=+3`, default `0`; `docs/ttrpg/03-character/characteristics_and_aptitudes.md` §9).
        #[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
        pub struct CharacterAptitudes {
            $(
                $(#[$meta])*
                pub $field: i32,
            )+
        }

        impl CharacterAptitudes {
            #[must_use]
            pub fn get(&self, code: AptitudeCode) -> i32 {
                self[code].clamp(-2, 3)
            }

            pub fn set(&mut self, code: AptitudeCode, value: i32) {
                self[code] = value.clamp(-2, 3);
            }
        }

        impl Index<AptitudeCode> for CharacterAptitudes {
            type Output = i32;

            fn index(&self, code: AptitudeCode) -> &Self::Output {
                match code {
                    $(AptitudeCode::$Variant => &self.$field,)+
                }
            }
        }

        impl IndexMut<AptitudeCode> for CharacterAptitudes {
            fn index_mut(&mut self, code: AptitudeCode) -> &mut Self::Output {
                match code {
                    $(AptitudeCode::$Variant => &mut self.$field,)+
                }
            }
        }
    };
}

impl_aptitudes_struct! {
    AptQnt => apt_qnt,
    AptMec => apt_mec,
    AptCmp => apt_cmp,
    AptBio => apt_bio,
    AptLng => apt_lng,
    AptKin => apt_kin,
    AptFin => apt_fin,
    AptAud => apt_aud,
    AptSoc => apt_soc,
}

/// Compute the Training Time Multiplier for a Domain Aptitude score $A \in [-2, +3]$ (§9.1.3).
///
/// - Each `+1` of Aptitude reduces training time / Extended Project Clock requirements by `15%` (`-0.15`).
/// - Each `-1` of Aptitude increases training time requirements by `20%` (`+0.20`).
#[must_use]
pub fn compute_aptitude_training_time_multiplier(aptitude: i32) -> f64 {
    let clamped = aptitude.clamp(-2, 3);
    if clamped >= 0 {
        1.0 - 0.15 * f64::from(clamped)
    } else {
        1.0 + 0.20 * f64::from(-clamped)
    }
}

/// Compute Minimum Characteristic (`Min C`) prerequisite relief from a Domain Aptitude (§9.1.2).
///
/// Characters with `APT >= +2` in a domain have the Minimum Characteristic (`Min C`) threshold
/// of skills in that tree reduced by `1` point.
#[must_use]
pub const fn compute_aptitude_min_c_relief(aptitude: i32) -> i32 {
    if aptitude >= 2 { 1 } else { 0 }
}

const SCALE_TABLE: [i32; 21] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
];

macro_rules! impl_characteristics_struct {
    ($($(#[$meta:meta])* $Variant:ident => $field:ident),+ $(,)?) => {
        /// Complete canonical 30-characteristic profile (`1..=20` controlling scale for 28 attributes, plus `mas_kg` and `hgt_m`; `docs/ttrpg/03-character/characteristics_and_aptitudes.md` §3–§7).
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct CharacterCharacteristics {
            $(
                $(#[$meta])*
                pub $field: i32,
            )+
            pub mas_kg: f64,
            pub hgt_m: f64,
        }

        impl Default for CharacterCharacteristics {
            fn default() -> Self {
                Self {
                    $($field: 10,)+
                    mas_kg: 70.0,
                    hgt_m: 1.75,
                }
            }
        }

        impl CharacterCharacteristics {
            #[must_use]
            pub fn get_scale(&self, code: CharacteristicCode) -> i32 {
                let raw = match code {
                    $(CharacteristicCode::$Variant => self.$field,)+
                    CharacteristicCode::MAS => (10.0 + (self.mas_kg - 70.0) / 7.0).round() as i32,
                    CharacteristicCode::HGT => (10.0 + (self.hgt_m - 1.75) / 0.05).round() as i32,
                };
                raw.clamp(1, 20)
            }

            pub fn set_scale(&mut self, code: CharacteristicCode, value: i32) {
                let clamped = value.clamp(1, 20);
                match code {
                    $(CharacteristicCode::$Variant => self.$field = clamped,)+
                    CharacteristicCode::MAS => self.mas_kg = 70.0 + f64::from(clamped - 10) * 7.0,
                    CharacteristicCode::HGT => self.hgt_m = 1.75 + f64::from(clamped - 10) * 0.05,
                }
            }
        }

        impl Index<CharacteristicCode> for CharacterCharacteristics {
            type Output = i32;

            fn index(&self, code: CharacteristicCode) -> &Self::Output {
                match code {
                    $(CharacteristicCode::$Variant => &self.$field,)+
                    CharacteristicCode::MAS | CharacteristicCode::HGT => {
                        &SCALE_TABLE[self.get_scale(code) as usize]
                    }
                }
            }
        }

        impl IndexMut<CharacteristicCode> for CharacterCharacteristics {
            fn index_mut(&mut self, code: CharacteristicCode) -> &mut Self::Output {
                match code {
                    $(CharacteristicCode::$Variant => &mut self.$field,)+
                    CharacteristicCode::MAS | CharacteristicCode::HGT => {
                        panic!("use set_scale or mas_kg/hgt_m directly to mutate MAS/HGT")
                    }
                }
            }
        }
    };
}

impl_characteristics_struct! {
    SPD => spd,
    FOR => for_stat,
    DEX => dex,
    KIN => kin,
    HLT => hlt,
    FIT => fit,
    MEM => mem,
    ANA => ana,
    INS => ins,
    CRE => cre,
    DIS => dis,
    FOC => foc,
    COM => com,
    PRE => pre,
    ELO => elo,
    SIG => sig,
    HEA => hea,
    TAC => tac,
    OLF => olf,
    TAS => tas,
    ITU => itu,
    EGO => ego,
    SLF => slf,
    OPN => opn,
    BLG => blg,
    EXT => ext,
    CNF => cnf,
    AGG => agg,
}

/// Compute Quick Thinking ($D_{\text{cog}} = \lfloor(\max(\text{ANA}, \text{CRE}) + \text{FOC}) / 2\rfloor$)
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §8 Table).
#[must_use]
pub fn compute_d_cog_score(ana: i32, cre: i32, foc: i32) -> i32 {
    let a = ana.clamp(1, 20);
    let c = cre.clamp(1, 20);
    let f = foc.clamp(1, 20);
    (a.max(c) + f).div_euclid(2)
}

/// Compute OODA Initiative ($\text{OODA} = \lfloor(\text{ITU} + \text{INS} + D_{\text{cog}} + \text{SPD}) / 4\rfloor$,
/// $\text{Tick}_{\text{start}} = \max(1, 15 - \text{OODA})$)
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §8 Table).
#[must_use]
pub fn compute_ooda_score(itu: i32, ins: i32, d_cog: i32, spd: i32) -> i32 {
    let i = itu.clamp(1, 20);
    let n = ins.clamp(1, 20);
    let d = d_cog.clamp(1, 20);
    let s = spd.clamp(1, 20);
    (i + n + d + s).div_euclid(4)
}

/// Compute Striking Power ($\text{POW} = (\text{FOR} \times \text{SPD}) / 10.0$)
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §8 Table).
#[must_use]
pub fn compute_pow_striking_power(for_stat: i32, spd: i32) -> f64 {
    f64::from(for_stat.clamp(1, 20) * spd.clamp(1, 20)) / 10.0
}

/// Compute Mass-Momentum Factor ($\text{MMF} = (\text{MAS} / 70.0) \times \text{SPD}$)
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §8 Table).
#[must_use]
pub fn compute_mmf_momentum_factor(mas_kg: f64, spd: i32) -> f64 {
    (mas_kg.max(20.0) / 70.0) * f64::from(spd.clamp(1, 20))
}

/// Compute Fatigue Threshold ($\text{FT} = \text{FIT} + \lfloor\text{DIS} / 2\rfloor$)
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §8 Table).
#[must_use]
pub fn compute_fatigue_threshold_score(fit: i32, dis: i32) -> i32 {
    fit.clamp(1, 20) + dis.clamp(1, 20).div_euclid(2)
}

/// Compute Wound Shock Resistance ($\text{WSR} = \text{HLT} + \lfloor\text{DIS} / 2\rfloor$)
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §8 Table).
#[must_use]
pub fn compute_wound_shock_resistance_score(hlt: i32, dis: i32) -> i32 {
    hlt.clamp(1, 20) + dis.clamp(1, 20).div_euclid(2)
}

/// Compute Social Composure Threshold ($\text{CMP} = \lfloor(\text{DIS} + \text{FOC}) / 4\rfloor$)
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §8 Table).
#[must_use]
pub fn compute_social_composure_score(dis: i32, foc: i32) -> i32 {
    (dis.clamp(1, 20) + foc.clamp(1, 20)).div_euclid(4)
}

/// Compute General Awareness ($\text{AWR} = \lfloor(\max(\text{SIG}, \text{HEA}) + \text{FOC} + \text{ITU}) / 3\rfloor$)
/// (`docs/ttrpg/03-character/characteristics_and_aptitudes.md` §8 Table).
#[must_use]
pub fn compute_general_awareness_score(sig: i32, hea: i32, foc: i32, itu: i32) -> i32 {
    (sig.clamp(1, 20).max(hea.clamp(1, 20)) + foc.clamp(1, 20) + itu.clamp(1, 20)).div_euclid(3)
}

impl CharacterCharacteristics {
    /// Compute the effective controlling score $C$ for a skill governed by `(primary, Option<secondary>)` (§10.1).
    ///
    /// - Single controller: $C = C_1$
    /// - Composite controller: $C = \lfloor(C_1 + C_2) / 2\rfloor$
    #[must_use]
    pub fn compute_controller(
        &self,
        primary: CharacteristicCode,
        secondary: Option<CharacteristicCode>,
    ) -> i32 {
        let c1 = self.get_scale(primary);
        match secondary {
            Some(sec) => (c1 + self.get_scale(sec)).div_euclid(2),
            None => c1,
        }
    }

    /// Compute all 10 Secondary & Derived Characteristics (§8).
    #[must_use]
    pub fn compute_derived(&self, weapon_reach_bonus: i32) -> DerivedCharacteristics {
        let kin = self.kin.clamp(1, 20);
        let ana = self.ana.clamp(1, 20);
        let dis = self.dis.clamp(1, 20);
        let hgt_m = self.hgt_m.clamp(0.5, 3.0);

        let pow_striking_power = compute_pow_striking_power(self.for_stat, self.spd);
        let mmf_momentum_factor = compute_mmf_momentum_factor(self.mas_kg, self.spd);

        let height_delta = hgt_m - 1.75;
        let height_reach_edge =
            ((height_delta + 1e-9 * height_delta.signum()) / 0.15).trunc() as i32;
        let kin_reach_edge = (kin - 10).div_euclid(4);
        let melee_reach_index = height_reach_edge + kin_reach_edge + weapon_reach_bonus;

        let d_cog_quick_thinking = compute_d_cog_score(self.ana, self.cre, self.foc);
        let ooda_initiative =
            compute_ooda_score(self.itu, self.ins, d_cog_quick_thinking, self.spd);
        let awr_general_awareness =
            compute_general_awareness_score(self.sig, self.hea, self.foc, self.itu);
        let ft_fatigue_threshold = compute_fatigue_threshold_score(self.fit, self.dis);
        let wsr_wound_shock_resistance = compute_wound_shock_resistance_score(self.hlt, self.dis);
        let cmp_social_composure_threshold = compute_social_composure_score(self.dis, self.foc);
        let base_c_study = compute_academic_study_controller(dis, mem_or_ana(self.mem, ana), 0);

        DerivedCharacteristics {
            pow_striking_power,
            mmf_momentum_factor,
            height_reach_edge,
            melee_reach_index,
            d_cog_quick_thinking,
            ooda_initiative,
            awr_general_awareness,
            ft_fatigue_threshold,
            wsr_wound_shock_resistance,
            cmp_social_composure_threshold,
            base_c_study,
        }
    }
}

const fn mem_or_ana(mem: i32, ana: i32) -> i32 {
    if mem >= ana { mem } else { ana }
}

/// Compute the Academic Study Controller $C_{\text{study}} = \lfloor(\text{DIS} + C_{\text{mental}}) / 2\rfloor + \text{APT}$ (§8, §9.1.1).
#[must_use]
pub const fn compute_academic_study_controller(dis: i32, c_mental: i32, aptitude: i32) -> i32 {
    let d = if dis < 1 {
        1
    } else if dis > 20 {
        20
    } else {
        dis
    };
    let m = if c_mental < 1 {
        1
    } else if c_mental > 20 {
        20
    } else {
        c_mental
    };
    let a = if aptitude < -2 {
        -2
    } else if aptitude > 3 {
        3
    } else {
        aptitude
    };
    (d + m).div_euclid(2) + a
}

/// All 10 Secondary & Derived Characteristics (§8 of `characteristics_and_aptitudes.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedCharacteristics {
    /// $\text{POW} = (\text{FOR} \times \text{SPD}) / 10$
    pub pow_striking_power: f64,
    /// $\text{MMF} = (\text{MAS} / 70.0) \times \text{SPD}$
    pub mmf_momentum_factor: f64,
    pub height_reach_edge: i32,
    /// Total Melee Reach Index (`height_reach_edge + floor((KIN - 10)/4) + weapon_reach_bonus`)
    pub melee_reach_index: i32,
    /// $D_{\text{cog}} = \lfloor(\max(\text{ANA}, \text{CRE}) + \text{FOC}) / 2\rfloor$
    pub d_cog_quick_thinking: i32,
    /// $\text{OODA} = \lfloor(\text{ITU} + \text{INS} + D_{\text{cog}} + \text{SPD}) / 4\rfloor$
    pub ooda_initiative: i32,
    /// $\text{AWR} = \lfloor(\max(\text{SIG}, \text{HEA}) + \text{FOC} + \text{ITU}) / 3\rfloor$
    pub awr_general_awareness: i32,
    /// $\text{FT} = \text{FIT} + \lfloor\text{DIS} / 2\rfloor$
    pub ft_fatigue_threshold: i32,
    /// $\text{WSR} = \text{HLT} + \lfloor\text{DIS} / 2\rfloor$
    pub wsr_wound_shock_resistance: i32,
    /// $\text{CMP} = \lfloor(\text{DIS} + \text{FOC}) / 4\rfloor$
    pub cmp_social_composure_threshold: i32,
    /// $\lfloor(\text{DIS} + \max(\text{MEM}, \text{ANA})) / 2\rfloor$ (before specific domain aptitude)
    pub base_c_study: i32,
}

impl CharacteristicCode {
    pub const PHYSICAL_SCALED: [Self; 6] = [
        Self::SPD,
        Self::FOR,
        Self::DEX,
        Self::KIN,
        Self::HLT,
        Self::FIT,
    ];

    pub const MENTAL_ALL: [Self; 6] = [
        Self::MEM,
        Self::ANA,
        Self::INS,
        Self::CRE,
        Self::DIS,
        Self::FOC,
    ];

    pub const SENSORY_ALL: [Self; 6] = [
        Self::SIG,
        Self::HEA,
        Self::TAC,
        Self::OLF,
        Self::TAS,
        Self::ITU,
    ];

    pub const SOCIAL_ALL: [Self; 3] = [Self::COM, Self::PRE, Self::ELO];

    pub const PSYCHE_ALL: [Self; 7] = [
        Self::EGO,
        Self::SLF,
        Self::OPN,
        Self::BLG,
        Self::EXT,
        Self::CNF,
        Self::AGG,
    ];
}

/// Dice generation method for rolling Characteristic groups in the Character Creation Wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CharacteristicRollMethod {
    /// Method 1: Roll `2d4 + 3d5` and drop the lowest `d5` (`4–18`, skewed mean `~11.96`).
    /// Rolled `6` times for Physical/Mental/Sensory and `3` times for Social.
    Method1DropLowestD5,
    /// Method 2: Roll `2d4 + 2d5` (`4–18`, mean `11.0`) `8` times and keep the `6` highest
    /// for Physical/Mental/Sensory; roll `4` times and keep the `3` highest for Social.
    Method2PoolKeepHighest,
}

/// Detailed breakdown of a single `4–18` characteristic roll under Method 1 or Method 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleCharacteristicRoll {
    pub d4_rolls: [i32; 2],
    pub d5_rolls: Vec<i32>,
    pub dropped_d5_index: Option<usize>,
    /// Resulting characteristic score (`4..=18`).
    pub total: i32,
    pub dropped_from_pool: bool,
}

/// Rolled characteristic pool for a domain group (`Physical`, `Mental`, `Sensory`, or `Social`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacteristicGroupPool {
    pub category: CharacteristicCategory,
    pub method: CharacteristicRollMethod,
    pub raw_rolls: Vec<SingleCharacteristicRoll>,
    pub kept_values: Vec<i32>,
}

/// Roll a single `2d4 + 3d5` (dropping the lowest `d5`) using a caller-supplied die roller `roll_die(sides) -> 1..=sides`.
pub fn roll_characteristic_method1<R>(roller: &mut R) -> SingleCharacteristicRoll
where
    R: FnMut(i32) -> i32,
{
    let d4_1 = roller(4).clamp(1, 4);
    let d4_2 = roller(4).clamp(1, 4);
    let d5_1 = roller(5).clamp(1, 5);
    let d5_2 = roller(5).clamp(1, 5);
    let d5_3 = roller(5).clamp(1, 5);
    let d5_rolls = vec![d5_1, d5_2, d5_3];

    let mut min_idx = 0usize;
    for idx in 1..3 {
        if d5_rolls[idx] < d5_rolls[min_idx] {
            min_idx = idx;
        }
    }
    let kept_d5_sum: i32 = d5_rolls
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| if i == min_idx { None } else { Some(v) })
        .sum();
    let total = (d4_1 + d4_2 + kept_d5_sum).clamp(4, 18);

    SingleCharacteristicRoll {
        d4_rolls: [d4_1, d4_2],
        d5_rolls,
        dropped_d5_index: Some(min_idx),
        total,
        dropped_from_pool: false,
    }
}

/// Roll a single `2d4 + 2d5` (`4..=18`, mean `11.0`) using a caller-supplied die roller `roll_die(sides) -> 1..=sides`.
pub fn roll_characteristic_2d4_2d5<R>(roller: &mut R) -> SingleCharacteristicRoll
where
    R: FnMut(i32) -> i32,
{
    let d4_1 = roller(4).clamp(1, 4);
    let d4_2 = roller(4).clamp(1, 4);
    let d5_1 = roller(5).clamp(1, 5);
    let d5_2 = roller(5).clamp(1, 5);
    let total = (d4_1 + d4_2 + d5_1 + d5_2).clamp(4, 18);

    SingleCharacteristicRoll {
        d4_rolls: [d4_1, d4_2],
        d5_rolls: vec![d5_1, d5_2],
        dropped_d5_index: None,
        total,
        dropped_from_pool: false,
    }
}

fn mark_and_extract_drop_lowest<T, K, V>(
    raw_rolls: &mut [T],
    drop_count: usize,
    sort_key: impl Fn(&T) -> K,
    mark_dropped: impl Fn(&mut T),
    kept_value: impl Fn(&T) -> V,
) -> Vec<V>
where
    K: Ord,
{
    let mut dropped_flags = vec![false; raw_rolls.len()];
    if drop_count > 0 && drop_count <= raw_rolls.len() {
        let mut indices_by_key: Vec<(usize, K)> = raw_rolls
            .iter()
            .enumerate()
            .map(|(i, item)| (i, sort_key(item)))
            .collect();
        indices_by_key.sort_by(|a, b| a.1.cmp(&b.1));
        for &(drop_idx, _) in indices_by_key.iter().take(drop_count) {
            mark_dropped(&mut raw_rolls[drop_idx]);
            dropped_flags[drop_idx] = true;
        }
    }
    raw_rolls
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            if dropped_flags[i] {
                None
            } else {
                Some(kept_value(item))
            }
        })
        .collect()
}

/// Roll a complete assignable pool for `category` (`Physical`, `Mental`, `Sensory`, or `Social`)
/// using either `Method1DropLowestD5` or `Method2PoolKeepHighest`.
pub fn roll_characteristic_group_pool<R>(
    category: CharacteristicCategory,
    method: CharacteristicRollMethod,
    roller: &mut R,
) -> CharacteristicGroupPool
where
    R: FnMut(i32) -> i32,
{
    let (num_rolls, num_keep): (usize, usize) = match (category, method) {
        (CharacteristicCategory::Social, CharacteristicRollMethod::Method1DropLowestD5) => (3, 3),
        (CharacteristicCategory::Social, CharacteristicRollMethod::Method2PoolKeepHighest) => {
            (4, 3)
        }
        (_, CharacteristicRollMethod::Method1DropLowestD5) => (6, 6),
        (_, CharacteristicRollMethod::Method2PoolKeepHighest) => (8, 6),
    };

    let mut raw_rolls: Vec<SingleCharacteristicRoll> = (0..num_rolls)
        .map(|_| match method {
            CharacteristicRollMethod::Method1DropLowestD5 => roll_characteristic_method1(roller),
            CharacteristicRollMethod::Method2PoolKeepHighest => roll_characteristic_2d4_2d5(roller),
        })
        .collect();

    let kept_values = mark_and_extract_drop_lowest(
        &mut raw_rolls,
        num_rolls.saturating_sub(num_keep),
        |r| r.total,
        |r| r.dropped_from_pool = true,
        |r| r.total,
    );

    CharacteristicGroupPool {
        category,
        method,
        raw_rolls,
        kept_values,
    }
}

/// Correlated Height (`m`) and Build/Force-derived Mass (`kg`) roll outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorrelatedHeightMassRoll {
    /// `2d4 + 2d5` roll (`4..=18`, mean `11`) governing stature around `1.72 m`.
    pub height_roll: SingleCharacteristicRoll,
    /// `2d4 + 2d5` roll (`4..=18`, mean `11`) governing somatic build/frame.
    pub build_roll: SingleCharacteristicRoll,
    /// Force (`FOR`) score (`1..=20`) contributing muscular mass to BMI.
    pub for_stat: i32,
    pub height_m: f64,
    pub bmi: f64,
    pub mass_kg: f64,
}

/// Compute correlated Height (`m`) and Mass (`kg`) from a `height_roll_total` (`4..=18`),
/// `build_roll_total` (`4..=18`), and `for_stat` (`1..=20`).
#[must_use]
pub fn compute_correlated_height_mass(
    height_roll_total: i32,
    build_roll_total: i32,
    for_stat: i32,
) -> (f64, f64, f64) {
    let h_clamped = height_roll_total.clamp(4, 18);
    let b_clamped = build_roll_total.clamp(4, 18);
    let f_clamped = for_stat.clamp(1, 20);

    let height_m = ((1.72 + f64::from(h_clamped - 11) * 0.03) * 100.0).round() / 100.0;
    let bmi = 23.0 + 0.55 * f64::from(b_clamped - 11) + 0.45 * f64::from(f_clamped - 10);
    let raw_mass = bmi * height_m * height_m;
    let mass_kg = (raw_mass * 2.0).round() / 2.0;

    (height_m, bmi, mass_kg)
}

/// Roll correlated Height (`m`) and Mass (`kg`) using `2d4 + 2d5` for stature and build, combined with `for_stat`.
pub fn roll_correlated_height_mass<R>(for_stat: i32, roller: &mut R) -> CorrelatedHeightMassRoll
where
    R: FnMut(i32) -> i32,
{
    let height_roll = roll_characteristic_2d4_2d5(roller);
    let build_roll = roll_characteristic_2d4_2d5(roller);
    let (height_m, bmi, mass_kg) =
        compute_correlated_height_mass(height_roll.total, build_roll.total, for_stat);

    CorrelatedHeightMassRoll {
        height_roll,
        build_roll,
        for_stat: for_stat.clamp(1, 20),
        height_m,
        bmi,
        mass_kg,
    }
}

/// Map a `3d6` sum (`3..=18`) to a canonical Satya Domain Aptitude score (`-2..=+3`).
///
/// Distribution on `3d6` (`216` permutations):
/// - `3–4` $\to -2$ (`4/216 = 1.85%`)
/// - `5–7` $\to -1$ (`31/216 = 14.35%`)
/// - `8–13` $\to 0$ (`135/216 = 62.50%`)
/// - `14–15` $\to +1$ (`35/216 = 16.20%`)
/// - `16–17` $\to +2$ (`10/216 = 4.63%`)
/// - `18` $\to +3$ (`1/216 = 0.46%` per roll, `~4.98%` chance of at least one `+3` across 11 rolls)
#[must_use]
pub const fn aptitude_from_3d6_sum(sum_3d6: i32) -> i32 {
    match sum_3d6 {
        ..=4 => -2,
        5..=7 => -1,
        8..=13 => 0,
        14..=15 => 1,
        16..=17 => 2,
        18.. => 3,
    }
}

/// Single `3d6` Domain Aptitude roll in the `11 rolls, drop 2 lowest` pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleAptitudeRoll {
    pub d6_rolls: [i32; 3],
    pub sum_3d6: i32,
    /// Mapped Domain Aptitude modifier (`-2..=+3`).
    pub aptitude_value: i32,
    pub dropped_from_pool: bool,
}

/// Complete `11 x 3d6 (drop 2 lowest -> 9 kept)` Domain Aptitude pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AptitudePoolRoll {
    pub raw_rolls: Vec<SingleAptitudeRoll>,
    pub kept_values: Vec<i32>,
}

/// Roll `3d6` 11 times and drop the 2 lowest rolls (by `sum_3d6`), leaving 9 Domain Aptitude scores (`-2..=+3`).
pub fn roll_aptitude_pool_11_drop_2<R>(roller: &mut R) -> AptitudePoolRoll
where
    R: FnMut(i32) -> i32,
{
    let mut raw_rolls: Vec<SingleAptitudeRoll> = (0..11)
        .map(|_| {
            let d1 = roller(6).clamp(1, 6);
            let d2 = roller(6).clamp(1, 6);
            let d3 = roller(6).clamp(1, 6);
            let sum_3d6 = d1 + d2 + d3;
            SingleAptitudeRoll {
                d6_rolls: [d1, d2, d3],
                sum_3d6,
                aptitude_value: aptitude_from_3d6_sum(sum_3d6),
                dropped_from_pool: false,
            }
        })
        .collect();

    let kept_values = mark_and_extract_drop_lowest(
        &mut raw_rolls,
        2,
        |r| r.sum_3d6,
        |r| r.dropped_from_pool = true,
        |r| r.aptitude_value,
    );

    AptitudePoolRoll {
        raw_rolls,
        kept_values,
    }
}
