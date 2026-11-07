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

/// Category of a primary characteristic in the Satya system.
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
    /// Canonical characteristic codes across Physical, Mental, Social, Sensory, and Psyche domains.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum CharacteristicCode [count = 30] {
        // Physical (8)
        SPD => { code: "SPD", label: "Speed" },
        FOR => { code: "FOR", label: "Force" },
        DEX => { code: "DEX", label: "Dexterity" },
        KIN => { code: "KIN", label: "Kinesthesia" },
        HLT => { code: "HLT", label: "Health" },
        FIT => { code: "FIT", label: "Fitness" },
        MAS => { code: "MAS", label: "Mass (kg)" },
        HGT => { code: "HGT", label: "Height (m)" },
        // Mental (6)
        MEM => { code: "MEM", label: "Memory" },
        ANA => { code: "ANA", label: "Analysis" },
        INS => { code: "INS", label: "Insight" },
        CRE => { code: "CRE", label: "Creativity" },
        DIS => { code: "DIS", label: "Discipline" },
        FOC => { code: "FOC", label: "Focus" },
        // Social (3)
        COM => { code: "COM", label: "Comeliness" },
        PRE => { code: "PRE", label: "Presence" },
        ELO => { code: "ELO", label: "Eloquence" },
        // Sensory (6)
        SIG => { code: "SIG", label: "Sight" },
        HEA => { code: "HEA", label: "Hearing" },
        TAC => { code: "TAC", label: "Tactile" },
        OLF => { code: "OLF", label: "Olfaction" },
        TAS => { code: "TAS", label: "Taste" },
        ITU => { code: "ITU", label: "Intuition" },
        // Psyche (7)
        EGO => { code: "EGO", label: "Egoism" },
        SLF => { code: "SLF", label: "Self-Esteem" },
        OPN => { code: "OPN", label: "Openness" },
        BLG => { code: "BLG", label: "Belonging" },
        EXT => { code: "EXT", label: "Extraversion" },
        CNF => { code: "CNF", label: "Conformity" },
        AGG => { code: "AGG", label: "Aggression" },
    }
}

impl CharacteristicCode {
    /// Domain category of the characteristic.
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

/// Population distribution tier on the `1–20` Controlling Scale (`0.0–100.0` Spectrum) (§2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PopulationTier {
    /// `1–3` (`0.0–15.0`): Severely Impaired / Deficient
    SeverelyImpaired,
    /// `4–6` (`15.1–30.0`): Below Average / Weak
    BelowAverage,
    /// `7–8` (`30.1–42.5`): Low Average
    LowAverage,
    /// `9–11` (`42.6–57.5`): Human Median (`10 = 50.0`)
    HumanMedian,
    /// `12–13` (`57.6–67.5`): Trained / Competent Professional
    TrainedCompetent,
    /// `14–15` (`67.6–77.5`): Gifted / Veteran Specialist
    GiftedVeteran,
    /// `16–17` (`77.6–87.5`): World-Class / Olympic / Savant
    WorldClass,
    /// `18` (`87.6–92.5`): Peak Unaugmented Human Limit
    PeakUnaugmented,
    /// `19–20` (`92.6–100.0`): Augmented / Transhuman Apex
    AugmentedApex,
}

impl PopulationTier {
    /// Classify a `1–20` controlling characteristic score into its canonical Population Tier.
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

    /// Human-readable label from §2 table.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SeverelyImpaired => "Severely Impaired / Deficient",
            Self::BelowAverage => "Below Average / Weak",
            Self::LowAverage => "Low Average",
            Self::HumanMedian => "Human Median",
            Self::TrainedCompetent => "Trained / Competent Professional",
            Self::GiftedVeteran => "Gifted / Veteran Specialist",
            Self::WorldClass => "World-Class / Olympic / Savant",
            Self::PeakUnaugmented => "Peak Unaugmented Human Limit",
            Self::AugmentedApex => "Augmented / Transhuman Apex",
        }
    }

    /// Base Target Number (`31 - C`) for a single characteristic score `c`.
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

/// Convert a continuous `0.0–100.0` Character Spectrum value to the `1–20` Controlling Scale (§2).
///
/// `5.0` spectrum points = `1` d20 scale step (`50.0 -> 10`).
#[must_use]
pub fn spectrum_to_controlling_scale(spectrum: f64) -> i32 {
    let clamped = spectrum.clamp(0.0, 100.0);
    let step = (clamped / 5.0).round() as i32;
    step.clamp(1, 20)
}

/// Convert a `1–20` Controlling Scale integer to its canonical `0.0–100.0` Spectrum midpoint (`C * 5.0`).
#[must_use]
pub fn controlling_scale_to_spectrum(scale: i32) -> f64 {
    f64::from(scale.clamp(1, 20)) * 5.0
}

/// Bipolar disposition pole for the 7 Psyche characteristics (§7).
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
    /// Classify a `1–20` Psyche score into `LowPole (1–7)`, `Balanced (8–12)`, or `HighPole (13–20)`.
    #[must_use]
    pub const fn from_score(score: i32) -> Self {
        match score {
            ..=7 => Self::LowPole,
            8..=12 => Self::Balanced,
            13.. => Self::HighPole,
        }
    }
}

/// Return the canonical Low Pole / Balanced / High Pole descriptor for a Psyche characteristic (§7).
#[must_use]
pub const fn psyche_pole_descriptor(code: CharacteristicCode, score: i32) -> &'static str {
    let pole = PsychePole::from_score(score);
    match (code, pole) {
        (CharacteristicCode::EGO, PsychePole::LowPole) => "Selfless / Altruistic",
        (CharacteristicCode::EGO, PsychePole::Balanced) => "Pragmatic Self-Interest",
        (CharacteristicCode::EGO, PsychePole::HighPole) => "Self-Serving / Narcissistic",

        (CharacteristicCode::SLF, PsychePole::LowPole) => "Skeptical / Insecure",
        (CharacteristicCode::SLF, PsychePole::Balanced) => "Grounded Confidence",
        (CharacteristicCode::SLF, PsychePole::HighPole) => "Unshakeable / Arrogant",

        (CharacteristicCode::OPN, PsychePole::LowPole) => "Guarded / Secretive",
        (CharacteristicCode::OPN, PsychePole::Balanced) => "Selective Candor",
        (CharacteristicCode::OPN, PsychePole::HighPole) => "Candid / Transparent",

        (CharacteristicCode::BLG, PsychePole::LowPole) => "Lone Wolf / Detached",
        (CharacteristicCode::BLG, PsychePole::Balanced) => "Cooperative Autonomous",
        (CharacteristicCode::BLG, PsychePole::HighPole) => "Tribal / Loyal / Codependent",

        (CharacteristicCode::EXT, PsychePole::LowPole) => "Introverted / Reserved",
        (CharacteristicCode::EXT, PsychePole::Balanced) => "Ambiverted",
        (CharacteristicCode::EXT, PsychePole::HighPole) => "Gregarious / Extraverted",

        (CharacteristicCode::CNF, PsychePole::LowPole) => "Iconoclastic / Rebellious",
        (CharacteristicCode::CNF, PsychePole::Balanced) => "Situational Pragmatist",
        (CharacteristicCode::CNF, PsychePole::HighPole) => "Orthodox / Rule-Bound",

        (CharacteristicCode::AGG, PsychePole::LowPole) => "Avoidant / De-escalatory",
        (CharacteristicCode::AGG, PsychePole::Balanced) => "Measured Assertiveness",
        (CharacteristicCode::AGG, PsychePole::HighPole) => "Confrontational / Ruthless",

        _ => "Non-Psyche Characteristic",
    }
}

define_coded_enum! {
    /// The 9 canonical Domain Aptitudes (`APT-*`, ranging from `-2` to `+3`) (§9).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum AptitudeCode [count = 9] {
        /// `APT-QNT`: Quantitative & Mathematical (Tree 1)
        AptQnt => { code: "APT-QNT", label: "Quantitative & Mathematical", aliases: ["QNT"] },
        /// `APT-MEC`: Mechanical & Spatial (Tree 2)
        AptMec => { code: "APT-MEC", label: "Mechanical & Spatial", aliases: ["MEC"] },
        /// `APT-CMP`: Computational & Algorithmic (Tree 1)
        AptCmp => { code: "APT-CMP", label: "Computational & Algorithmic", aliases: ["CMP"] },
        /// `APT-BIO`: Biological & Chemical (Tree 3)
        AptBio => { code: "APT-BIO", label: "Biological & Chemical", aliases: ["BIO"] },
        /// `APT-LNG`: Linguistic & Symbolic (Tree 4)
        AptLng => { code: "APT-LNG", label: "Linguistic & Symbolic", aliases: ["LNG"] },
        /// `APT-KIN`: Gross Kinesthetic & Athletic (Tree 5)
        AptKin => { code: "APT-KIN", label: "Gross Kinesthetic & Athletic", aliases: ["KIN"] },
        /// `APT-FIN`: Fine Motor & Ballistic Precision (Trees 2, 6, 8)
        AptFin => { code: "APT-FIN", label: "Fine Motor & Ballistic Precision", aliases: ["FIN"] },
        /// `APT-AUD`: Musical & Auditory-Rhythmic (Trees 7, 8)
        AptAud => { code: "APT-AUD", label: "Musical & Auditory-Rhythmic", aliases: ["AUD"] },
        /// `APT-SOC`: Interpersonal & Rhetorical (Tree 4)
        AptSoc => { code: "APT-SOC", label: "Interpersonal & Rhetorical", aliases: ["SOC"] },
    }
}

macro_rules! impl_aptitudes_struct {
    ($($Variant:ident => $field:ident),+ $(,)?) => {
        /// Character's 9 Domain Aptitude scores (`-2` to `+3`, default `0`).
        #[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
        pub struct CharacterAptitudes {
            $(pub $field: i32,)+
        }

        impl CharacterAptitudes {
            /// Look up the clamped (`-2..=+3`) score for a specific `AptitudeCode`.
            #[must_use]
            pub fn get(&self, code: AptitudeCode) -> i32 {
                self[code].clamp(-2, 3)
            }

            /// Set the score for a specific `AptitudeCode` (clamped to `-2..=+3`).
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
    ($($Variant:ident => $field:ident),+ $(,)?) => {
        /// Complete canonical 25+ Characteristic profile (`03-character/characteristics_and_aptitudes.md`).
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct CharacterCharacteristics {
            $(pub $field: i32,)+
            pub mas_kg: f64,
            pub hgt_m: f64,
        }

        impl Default for CharacterCharacteristics {
            fn default() -> Self {
                Self {
                    $($field: 10,)+
                    mas_kg: 75.0,
                    hgt_m: 1.75,
                }
            }
        }

        impl CharacterCharacteristics {
            /// Retrieve the `1–20` integer score for a characteristic code.
            ///
            /// For `MAS` and `HGT`, converts `70.0 kg -> 10` (`7 kg` per step) and `1.75 m -> 10` (`0.05 m` per step).
            #[must_use]
            pub fn get_scale(&self, code: CharacteristicCode) -> i32 {
                let raw = match code {
                    $(CharacteristicCode::$Variant => self.$field,)+
                    CharacteristicCode::MAS => (10.0 + (self.mas_kg - 70.0) / 7.0).round() as i32,
                    CharacteristicCode::HGT => (10.0 + (self.hgt_m - 1.75) / 0.05).round() as i32,
                };
                raw.clamp(1, 20)
            }

            /// Set a `1–20` integer score for a characteristic code.
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

/// Compute Quick Thinking ($D_{\text{cog}} = \lfloor(\max(\text{ANA}, \text{CRE}) + \text{FOC}) / 2\rfloor$) (§8).
#[must_use]
pub fn compute_d_cog_score(ana: i32, cre: i32, foc: i32) -> i32 {
    let a = ana.clamp(1, 20);
    let c = cre.clamp(1, 20);
    let f = foc.clamp(1, 20);
    (a.max(c) + f).div_euclid(2)
}

/// Compute OODA Initiative ($\text{OODA} = \lfloor(\text{ITU} + \text{INS} + D_{\text{cog}} + \text{SPD}) / 4\rfloor$) (§8).
#[must_use]
pub fn compute_ooda_score(itu: i32, ins: i32, d_cog: i32, spd: i32) -> i32 {
    let i = itu.clamp(1, 20);
    let n = ins.clamp(1, 20);
    let d = d_cog.clamp(1, 20);
    let s = spd.clamp(1, 20);
    (i + n + d + s).div_euclid(4)
}

/// Compute Striking Power ($\text{POW} = (\text{FOR} \times \text{SPD}) / 10.0$) (§8).
#[must_use]
pub fn compute_pow_striking_power(for_stat: i32, spd: i32) -> f64 {
    f64::from(for_stat.clamp(1, 20) * spd.clamp(1, 20)) / 10.0
}

/// Compute Mass-Momentum Factor ($\text{MMF} = (\text{MAS} / 70.0) \times \text{SPD}$) (§8).
#[must_use]
pub fn compute_mmf_momentum_factor(mas_kg: f64, spd: i32) -> f64 {
    (mas_kg.max(20.0) / 70.0) * f64::from(spd.clamp(1, 20))
}

/// Compute Fatigue Threshold ($\text{FT} = \text{FIT} + \lfloor\text{DIS} / 2\rfloor$) (§8).
#[must_use]
pub fn compute_fatigue_threshold_score(fit: i32, dis: i32) -> i32 {
    fit.clamp(1, 20) + dis.clamp(1, 20).div_euclid(2)
}

/// Compute Wound Shock Resistance ($\text{WSR} = \text{HLT} + \lfloor\text{DIS} / 2\rfloor$) (§8).
#[must_use]
pub fn compute_wound_shock_resistance_score(hlt: i32, dis: i32) -> i32 {
    hlt.clamp(1, 20) + dis.clamp(1, 20).div_euclid(2)
}

/// Compute Social Composure Threshold ($\text{CMP} = \lfloor(\text{DIS} + \text{FOC}) / 4\rfloor$) (§8).
#[must_use]
pub fn compute_social_composure_score(dis: i32, foc: i32) -> i32 {
    (dis.clamp(1, 20) + foc.clamp(1, 20)).div_euclid(4)
}

/// Compute General Awareness ($\text{AWR} = \lfloor(\max(\text{SIG}, \text{HEA}) + \text{FOC} + \text{ITU}) / 3\rfloor$) (§8).
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
    /// Height-based reach edge (`+1` per `0.15m` over `1.75m`)
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
    /// The 6 non-dimensional Physical characteristics (`SPD`, `FOR`, `DEX`, `KIN`, `HLT`, `FIT`).
    pub const PHYSICAL_SCALED: [Self; 6] = [
        Self::SPD,
        Self::FOR,
        Self::DEX,
        Self::KIN,
        Self::HLT,
        Self::FIT,
    ];

    /// The 6 Mental characteristics (`MEM`, `ANA`, `INS`, `CRE`, `DIS`, `FOC`).
    pub const MENTAL_ALL: [Self; 6] = [
        Self::MEM,
        Self::ANA,
        Self::INS,
        Self::CRE,
        Self::DIS,
        Self::FOC,
    ];

    /// The 6 Sensory characteristics (`SIG`, `HEA`, `TAC`, `OLF`, `TAS`, `ITU`).
    pub const SENSORY_ALL: [Self; 6] = [
        Self::SIG,
        Self::HEA,
        Self::TAC,
        Self::OLF,
        Self::TAS,
        Self::ITU,
    ];

    /// The 3 Social characteristics (`COM`, `PRE`, `ELO`).
    pub const SOCIAL_ALL: [Self; 3] = [Self::COM, Self::PRE, Self::ELO];

    /// The 7 Psyche characteristics (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`).
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
    /// The two `d4` outcomes (`1..=4`).
    pub d4_rolls: [i32; 2],
    /// The `d5` outcomes (`3` dice for Method 1, `2` dice for Method 2, each `1..=5`).
    pub d5_rolls: Vec<i32>,
    /// Index in `d5_rolls` of the dropped lowest `d5` (only `Some` for Method 1).
    pub dropped_d5_index: Option<usize>,
    /// Resulting characteristic score (`4..=18`).
    pub total: i32,
    /// True if this roll was dropped as one of the lowest rolls in a Method 2 pool (`8 -> 6` or `4 -> 3`).
    pub dropped_from_pool: bool,
}

/// Rolled characteristic pool for a domain group (`Physical`, `Mental`, `Sensory`, or `Social`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacteristicGroupPool {
    /// Domain category (`Physical`, `Mental`, `Sensory`, or `Social`).
    pub category: CharacteristicCategory,
    /// Generation method used.
    pub method: CharacteristicRollMethod,
    /// All individual rolls generated (`6` or `3` for Method 1; `8` or `4` for Method 2).
    pub raw_rolls: Vec<SingleCharacteristicRoll>,
    /// Kept assignable values (`6` for Physical/Mental/Sensory; `3` for Social).
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
    let (num_rolls, num_keep) = match (category, method) {
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

    if num_rolls > num_keep {
        let drop_count = num_rolls - num_keep;
        let mut indices_by_val: Vec<(usize, i32)> = raw_rolls
            .iter()
            .enumerate()
            .map(|(i, r)| (i, r.total))
            .collect();
        indices_by_val.sort_by_key(|&(_, val)| val);
        for &(drop_idx, _) in indices_by_val.iter().take(drop_count) {
            raw_rolls[drop_idx].dropped_from_pool = true;
        }
    }

    let kept_values: Vec<i32> = raw_rolls
        .iter()
        .filter(|r| !r.dropped_from_pool)
        .map(|r| r.total)
        .collect();

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
    /// Resulting Height in meters (`1.51 m ..= 1.93 m` from roll, `±0.03 m` per point from `11`).
    pub height_m: f64,
    /// Resulting Body Mass Index (`kg / m²`) combining build roll and `FOR`.
    pub bmi: f64,
    /// Resulting Mass in kilograms (rounded to nearest `0.5 kg`).
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
    /// The three `d6` outcomes (`1..=6`).
    pub d6_rolls: [i32; 3],
    /// Sum of the three `d6` (`3..=18`).
    pub sum_3d6: i32,
    /// Mapped Domain Aptitude modifier (`-2..=+3`).
    pub aptitude_value: i32,
    /// True if this roll is one of the 2 lowest rolls dropped from the 11-roll pool.
    pub dropped_from_pool: bool,
}

/// Complete `11 x 3d6 (drop 2 lowest -> 9 kept)` Domain Aptitude pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AptitudePoolRoll {
    /// All 11 `3d6` rolls in order.
    pub raw_rolls: Vec<SingleAptitudeRoll>,
    /// The 9 kept aptitude values (`-2..=+3`) after dropping the 2 lowest rolls.
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

    let mut indices_by_sum: Vec<(usize, i32)> = raw_rolls
        .iter()
        .enumerate()
        .map(|(i, r)| (i, r.sum_3d6))
        .collect();
    indices_by_sum.sort_by_key(|&(_, sum)| sum);
    for &(drop_idx, _) in indices_by_sum.iter().take(2) {
        raw_rolls[drop_idx].dropped_from_pool = true;
    }

    let kept_values = raw_rolls
        .iter()
        .filter(|r| !r.dropped_from_pool)
        .map(|r| r.aptitude_value)
        .collect();

    AptitudePoolRoll {
        raw_rolls,
        kept_values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectrum_and_population_tiers() {
        assert_eq!(spectrum_to_controlling_scale(50.0), 10);
        assert_eq!(spectrum_to_controlling_scale(0.0), 1);
        assert_eq!(spectrum_to_controlling_scale(100.0), 20);
        assert!((controlling_scale_to_spectrum(10) - 50.0).abs() < 1e-6);

        assert_eq!(
            PopulationTier::from_controlling_scale(2),
            PopulationTier::SeverelyImpaired
        );
        assert_eq!(
            PopulationTier::from_controlling_scale(10),
            PopulationTier::HumanMedian
        );
        assert_eq!(
            PopulationTier::from_controlling_scale(18),
            PopulationTier::PeakUnaugmented
        );
        assert_eq!(
            PopulationTier::from_controlling_scale(20),
            PopulationTier::AugmentedApex
        );
        assert_eq!(PopulationTier::base_tn(10), 21);
        assert_eq!(PopulationTier::base_tn(15), 16);
    }

    #[test]
    fn test_all_10_derived_characteristics_and_aptitudes() {
        let mut chars = CharacterCharacteristics::default();
        chars.spd = 14;
        chars.for_stat = 12;
        chars.kin = 14;
        chars.hlt = 13;
        chars.fit = 12;
        chars.mas_kg = 84.0;
        chars.hgt_m = 1.90;
        chars.ana = 15;
        chars.cre = 12;
        chars.foc = 13;
        chars.itu = 14;
        chars.ins = 12;
        chars.sig = 15;
        chars.hea = 11;
        chars.dis = 14;

        let derived = chars.compute_derived(2);
        // POW = (12 * 14) / 10 = 16.8
        assert!((derived.pow_striking_power - 16.8).abs() < 1e-6);
        // MMF = (84 / 70) * 14 = 16.8
        assert!((derived.mmf_momentum_factor - 16.8).abs() < 1e-6);
        // Height edge = (1.90 - 1.75)/0.15 = +1, KIN edge = (14 - 10)/4 = +1, weapon = +2 -> 4
        assert_eq!(derived.height_reach_edge, 1);
        assert_eq!(derived.melee_reach_index, 4);
        // D_cog = floor((max(15, 12) + 13) / 2) = 14
        assert_eq!(derived.d_cog_quick_thinking, 14);
        // OODA = floor((14 + 12 + 14 + 14) / 4) = 13
        assert_eq!(derived.ooda_initiative, 13);
        // AWR = floor((max(15, 11) + 13 + 14) / 3) = 14
        assert_eq!(derived.awr_general_awareness, 14);
        // FT = 12 + floor(14 / 2) = 19
        assert_eq!(derived.ft_fatigue_threshold, 19);
        // WSR = 13 + floor(14 / 2) = 20
        assert_eq!(derived.wsr_wound_shock_resistance, 20);
        // CMP = floor((14 + 13) / 4) = 6
        assert_eq!(derived.cmp_social_composure_threshold, 6);

        // Mateo Vance C_study check: DIS 11, ANA 13, APT-QNT +3 -> floor((11+13)/2) + 3 = 15
        assert_eq!(compute_academic_study_controller(11, 13, 3), 15);

        // Aptitude training time multipliers: +2 -> 0.70 (-30%), -2 -> 1.40 (+40%)
        assert!((compute_aptitude_training_time_multiplier(2) - 0.70).abs() < 1e-6);
        assert!((compute_aptitude_training_time_multiplier(-2) - 1.40).abs() < 1e-6);
        assert_eq!(compute_aptitude_min_c_relief(2), 1);
        assert_eq!(compute_aptitude_min_c_relief(1), 0);
    }

    #[test]
    fn test_characteristic_roll_methods_height_mass_and_aptitude_pool() {
        // Method 1: 2d4 + 3d5 drop lowest d5
        // Sequence: d4=[3, 4], d5=[2, 5, 4] -> drops 2 (index 0), total = 3 + 4 + 5 + 4 = 16
        let seq = [3, 4, 2, 5, 4];
        let mut idx = 0usize;
        let r1 = roll_characteristic_method1(&mut |_sides| {
            let v = seq[idx % seq.len()];
            idx += 1;
            v
        });
        assert_eq!(r1.d4_rolls, [3, 4]);
        assert_eq!(r1.d5_rolls, vec![2, 5, 4]);
        assert_eq!(r1.dropped_d5_index, Some(0));
        assert_eq!(r1.total, 16);

        // Method 1 Physical Pool: 6 rolls, 6 kept
        let mut counter = 1i32;
        let phys_m1 = roll_characteristic_group_pool(
            CharacteristicCategory::Physical,
            CharacteristicRollMethod::Method1DropLowestD5,
            &mut |sides| {
                let v = ((counter - 1) % sides) + 1;
                counter += 1;
                v
            },
        );
        assert_eq!(phys_m1.raw_rolls.len(), 6);
        assert_eq!(phys_m1.kept_values.len(), 6);
        assert!(phys_m1.kept_values.iter().all(|&v| (4..=18).contains(&v)));

        // Method 1 Social Pool: 3 rolls, 3 kept
        let soc_m1 = roll_characteristic_group_pool(
            CharacteristicCategory::Social,
            CharacteristicRollMethod::Method1DropLowestD5,
            &mut |sides| {
                let v = ((counter - 1) % sides) + 1;
                counter += 1;
                v
            },
        );
        assert_eq!(soc_m1.raw_rolls.len(), 3);
        assert_eq!(soc_m1.kept_values.len(), 3);

        // Method 2 Physical Pool: 8 rolls of 2d4+2d5, drops 2 lowest -> keeps 6 highest
        let phys_m2 = roll_characteristic_group_pool(
            CharacteristicCategory::Physical,
            CharacteristicRollMethod::Method2PoolKeepHighest,
            &mut |sides| {
                let v = ((counter - 1) % sides) + 1;
                counter += 1;
                v
            },
        );
        assert_eq!(phys_m2.raw_rolls.len(), 8);
        assert_eq!(
            phys_m2
                .raw_rolls
                .iter()
                .filter(|r| r.dropped_from_pool)
                .count(),
            2
        );
        assert_eq!(phys_m2.kept_values.len(), 6);

        // Method 2 Social Pool: 4 rolls of 2d4+2d5, drops 1 lowest -> keeps 3 highest
        let soc_m2 = roll_characteristic_group_pool(
            CharacteristicCategory::Social,
            CharacteristicRollMethod::Method2PoolKeepHighest,
            &mut |sides| {
                let v = ((counter - 1) % sides) + 1;
                counter += 1;
                v
            },
        );
        assert_eq!(soc_m2.raw_rolls.len(), 4);
        assert_eq!(
            soc_m2
                .raw_rolls
                .iter()
                .filter(|r| r.dropped_from_pool)
                .count(),
            1
        );
        assert_eq!(soc_m2.kept_values.len(), 3);

        // Correlated Height & Mass around mean (roll 11 -> 1.72m, FOR 10 -> BMI 23.0 -> 68.0 kg)
        let (hgt, bmi, mas) = compute_correlated_height_mass(11, 11, 10);
        assert!((hgt - 1.72).abs() < 1e-6);
        assert!((bmi - 23.0).abs() < 1e-6);
        assert!((mas - 68.0).abs() < 1e-6);

        // Aptitude 3d6 table & 11-roll drop-2-lowest pool
        assert_eq!(aptitude_from_3d6_sum(3), -2);
        assert_eq!(aptitude_from_3d6_sum(4), -2);
        assert_eq!(aptitude_from_3d6_sum(6), -1);
        assert_eq!(aptitude_from_3d6_sum(10), 0);
        assert_eq!(aptitude_from_3d6_sum(13), 0);
        assert_eq!(aptitude_from_3d6_sum(14), 1);
        assert_eq!(aptitude_from_3d6_sum(17), 2);
        assert_eq!(aptitude_from_3d6_sum(18), 3);

        let apt_pool = roll_aptitude_pool_11_drop_2(&mut |sides| {
            let v = ((counter - 1) % sides) + 1;
            counter += 1;
            v
        });
        assert_eq!(apt_pool.raw_rolls.len(), 11);
        assert_eq!(
            apt_pool
                .raw_rolls
                .iter()
                .filter(|r| r.dropped_from_pool)
                .count(),
            2
        );
        assert_eq!(apt_pool.kept_values.len(), 9);
        assert!(apt_pool.kept_values.iter().all(|&v| (-2..=3).contains(&v)));
    }
}
