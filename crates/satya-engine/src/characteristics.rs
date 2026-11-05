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

use serde::{Deserialize, Serialize};

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

/// Canonical characteristic codes across Physical, Mental, Social, Sensory, and Psyche domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CharacteristicCode {
    // Physical (8)
    SPD,
    FOR,
    DEX,
    KIN,
    HLT,
    FIT,
    MAS,
    HGT,
    // Mental (6)
    MEM,
    ANA,
    INS,
    CRE,
    DIS,
    FOC,
    // Social (3)
    COM,
    PRE,
    ELO,
    // Sensory (6)
    SIG,
    HEA,
    TAC,
    OLF,
    TAS,
    ITU,
    // Psyche (7)
    EGO,
    SLF,
    OPN,
    BLG,
    EXT,
    CNF,
    AGG,
}

impl CharacteristicCode {
    /// Canonical 3-letter code string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SPD => "SPD",
            Self::FOR => "FOR",
            Self::DEX => "DEX",
            Self::KIN => "KIN",
            Self::HLT => "HLT",
            Self::FIT => "FIT",
            Self::MAS => "MAS",
            Self::HGT => "HGT",
            Self::MEM => "MEM",
            Self::ANA => "ANA",
            Self::INS => "INS",
            Self::CRE => "CRE",
            Self::DIS => "DIS",
            Self::FOC => "FOC",
            Self::COM => "COM",
            Self::PRE => "PRE",
            Self::ELO => "ELO",
            Self::SIG => "SIG",
            Self::HEA => "HEA",
            Self::TAC => "TAC",
            Self::OLF => "OLF",
            Self::TAS => "TAS",
            Self::ITU => "ITU",
            Self::EGO => "EGO",
            Self::SLF => "SLF",
            Self::OPN => "OPN",
            Self::BLG => "BLG",
            Self::EXT => "EXT",
            Self::CNF => "CNF",
            Self::AGG => "AGG",
        }
    }

    /// Full human-readable name from `characteristics_and_aptitudes.md`.
    #[must_use]
    pub const fn full_name(self) -> &'static str {
        match self {
            Self::SPD => "Speed",
            Self::FOR => "Force",
            Self::DEX => "Dexterity",
            Self::KIN => "Kinesthesia",
            Self::HLT => "Health",
            Self::FIT => "Fitness",
            Self::MAS => "Mass (kg)",
            Self::HGT => "Height (m)",
            Self::MEM => "Memory",
            Self::ANA => "Analysis",
            Self::INS => "Insight",
            Self::CRE => "Creativity",
            Self::DIS => "Discipline",
            Self::FOC => "Focus",
            Self::COM => "Comeliness",
            Self::PRE => "Presence",
            Self::ELO => "Eloquence",
            Self::SIG => "Sight",
            Self::HEA => "Hearing",
            Self::TAC => "Tactile",
            Self::OLF => "Olfaction",
            Self::TAS => "Taste",
            Self::ITU => "Intuition",
            Self::EGO => "Egoism",
            Self::SLF => "Self-Esteem",
            Self::OPN => "Openness",
            Self::BLG => "Belonging",
            Self::EXT => "Extraversion",
            Self::CNF => "Conformity",
            Self::AGG => "Aggression",
        }
    }

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
            Self::EGO
            | Self::SLF
            | Self::OPN
            | Self::BLG
            | Self::EXT
            | Self::CNF
            | Self::AGG => CharacteristicCategory::Psyche,
        }
    }

    /// Parse a 3-letter characteristic code (case-insensitive).
    #[must_use]
    pub fn from_code_str(code: &str) -> Option<Self> {
        match code.trim().to_ascii_uppercase().as_str() {
            "SPD" => Some(Self::SPD),
            "FOR" => Some(Self::FOR),
            "DEX" => Some(Self::DEX),
            "KIN" => Some(Self::KIN),
            "HLT" => Some(Self::HLT),
            "FIT" => Some(Self::FIT),
            "MAS" => Some(Self::MAS),
            "HGT" => Some(Self::HGT),
            "MEM" => Some(Self::MEM),
            "ANA" => Some(Self::ANA),
            "INS" => Some(Self::INS),
            "CRE" => Some(Self::CRE),
            "DIS" => Some(Self::DIS),
            "FOC" => Some(Self::FOC),
            "COM" => Some(Self::COM),
            "PRE" => Some(Self::PRE),
            "ELO" => Some(Self::ELO),
            "SIG" => Some(Self::SIG),
            "HEA" => Some(Self::HEA),
            "TAC" => Some(Self::TAC),
            "OLF" => Some(Self::OLF),
            "TAS" => Some(Self::TAS),
            "ITU" => Some(Self::ITU),
            "EGO" => Some(Self::EGO),
            "SLF" => Some(Self::SLF),
            "OPN" => Some(Self::OPN),
            "BLG" => Some(Self::BLG),
            "EXT" => Some(Self::EXT),
            "CNF" => Some(Self::CNF),
            "AGG" => Some(Self::AGG),
            _ => None,
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

/// The 9 canonical Domain Aptitudes (`APT-*`, ranging from `-2` to `+3`) (§9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AptitudeCode {
    /// `APT-QNT`: Quantitative & Mathematical (Tree 1)
    AptQnt,
    /// `APT-MEC`: Mechanical & Spatial (Tree 2)
    AptMec,
    /// `APT-CMP`: Computational & Algorithmic (Tree 1)
    AptCmp,
    /// `APT-BIO`: Biological & Chemical (Tree 3)
    AptBio,
    /// `APT-LNG`: Linguistic & Symbolic (Tree 4)
    AptLng,
    /// `APT-KIN`: Gross Kinesthetic & Athletic (Tree 5)
    AptKin,
    /// `APT-FIN`: Fine Motor & Ballistic Precision (Trees 2, 6, 8)
    AptFin,
    /// `APT-AUD`: Musical & Auditory-Rhythmic (Trees 7, 8)
    AptAud,
    /// `APT-SOC`: Interpersonal & Rhetorical (Tree 4)
    AptSoc,
}

impl AptitudeCode {
    /// Canonical hyphenated code (`APT-QNT`, etc.).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AptQnt => "APT-QNT",
            Self::AptMec => "APT-MEC",
            Self::AptCmp => "APT-CMP",
            Self::AptBio => "APT-BIO",
            Self::AptLng => "APT-LNG",
            Self::AptKin => "APT-KIN",
            Self::AptFin => "APT-FIN",
            Self::AptAud => "APT-AUD",
            Self::AptSoc => "APT-SOC",
        }
    }

    /// Full name of the Domain Aptitude (§9).
    #[must_use]
    pub const fn full_name(self) -> &'static str {
        match self {
            Self::AptQnt => "Quantitative & Mathematical",
            Self::AptMec => "Mechanical & Spatial",
            Self::AptCmp => "Computational & Algorithmic",
            Self::AptBio => "Biological & Chemical",
            Self::AptLng => "Linguistic & Symbolic",
            Self::AptKin => "Gross Kinesthetic & Athletic",
            Self::AptFin => "Fine Motor & Ballistic Precision",
            Self::AptAud => "Musical & Auditory-Rhythmic",
            Self::AptSoc => "Interpersonal & Rhetorical",
        }
    }

    /// Parse an aptitude code string (`APT-QNT`, `QNT`, etc.).
    #[must_use]
    pub fn from_code_str(code: &str) -> Option<Self> {
        match code.trim().to_ascii_uppercase().as_str() {
            "APT-QNT" | "QNT" => Some(Self::AptQnt),
            "APT-MEC" | "MEC" => Some(Self::AptMec),
            "APT-CMP" | "CMP" => Some(Self::AptCmp),
            "APT-BIO" | "BIO" => Some(Self::AptBio),
            "APT-LNG" | "LNG" => Some(Self::AptLng),
            "APT-KIN" | "KIN" => Some(Self::AptKin),
            "APT-FIN" | "FIN" => Some(Self::AptFin),
            "APT-AUD" | "AUD" => Some(Self::AptAud),
            "APT-SOC" | "SOC" => Some(Self::AptSoc),
            _ => None,
        }
    }
}

/// Character's 9 Domain Aptitude scores (`-2` to `+3`, default `0`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterAptitudes {
    pub apt_qnt: i32,
    pub apt_mec: i32,
    pub apt_cmp: i32,
    pub apt_bio: i32,
    pub apt_lng: i32,
    pub apt_kin: i32,
    pub apt_fin: i32,
    pub apt_aud: i32,
    pub apt_soc: i32,
}

impl Default for CharacterAptitudes {
    fn default() -> Self {
        Self {
            apt_qnt: 0,
            apt_mec: 0,
            apt_cmp: 0,
            apt_bio: 0,
            apt_lng: 0,
            apt_kin: 0,
            apt_fin: 0,
            apt_aud: 0,
            apt_soc: 0,
        }
    }
}

impl CharacterAptitudes {
    /// Look up the clamped (`-2..=+3`) score for a specific `AptitudeCode`.
    #[must_use]
    pub fn get(&self, code: AptitudeCode) -> i32 {
        let raw = match code {
            AptitudeCode::AptQnt => self.apt_qnt,
            AptitudeCode::AptMec => self.apt_mec,
            AptitudeCode::AptCmp => self.apt_cmp,
            AptitudeCode::AptBio => self.apt_bio,
            AptitudeCode::AptLng => self.apt_lng,
            AptitudeCode::AptKin => self.apt_kin,
            AptitudeCode::AptFin => self.apt_fin,
            AptitudeCode::AptAud => self.apt_aud,
            AptitudeCode::AptSoc => self.apt_soc,
        };
        raw.clamp(-2, 3)
    }

    /// Set the score for a specific `AptitudeCode` (clamped to `-2..=+3`).
    pub fn set(&mut self, code: AptitudeCode, value: i32) {
        let clamped = value.clamp(-2, 3);
        match code {
            AptitudeCode::AptQnt => self.apt_qnt = clamped,
            AptitudeCode::AptMec => self.apt_mec = clamped,
            AptitudeCode::AptCmp => self.apt_cmp = clamped,
            AptitudeCode::AptBio => self.apt_bio = clamped,
            AptitudeCode::AptLng => self.apt_lng = clamped,
            AptitudeCode::AptKin => self.apt_kin = clamped,
            AptitudeCode::AptFin => self.apt_fin = clamped,
            AptitudeCode::AptAud => self.apt_aud = clamped,
            AptitudeCode::AptSoc => self.apt_soc = clamped,
        }
    }
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
    if aptitude >= 2 {
        1
    } else {
        0
    }
}

/// Complete canonical 25+ Characteristic profile (`03-character/characteristics_and_aptitudes.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterCharacteristics {
    // Physical (6 on 1-20 scale + Mass in kg + Height in m)
    pub spd: i32,
    pub for_stat: i32,
    pub dex: i32,
    pub kin: i32,
    pub hlt: i32,
    pub fit: i32,
    pub mas_kg: f64,
    pub hgt_m: f64,

    // Mental (6 on 1-20 scale)
    pub mem: i32,
    pub ana: i32,
    pub ins: i32,
    pub cre: i32,
    pub dis: i32,
    pub foc: i32,

    // Social (3 on 1-20 scale)
    pub com: i32,
    pub pre: i32,
    pub elo: i32,

    // Sensory (6 on 1-20 scale)
    pub sig: i32,
    pub hea: i32,
    pub tac: i32,
    pub olf: i32,
    pub tas: i32,
    pub itu: i32,

    // Psyche (7 on 1-20 scale)
    pub ego: i32,
    pub slf: i32,
    pub opn: i32,
    pub blg: i32,
    pub ext: i32,
    pub cnf: i32,
    pub agg: i32,
}

impl Default for CharacterCharacteristics {
    fn default() -> Self {
        Self {
            spd: 10,
            for_stat: 10,
            dex: 10,
            kin: 10,
            hlt: 10,
            fit: 10,
            mas_kg: 75.0,
            hgt_m: 1.75,
            mem: 10,
            ana: 10,
            ins: 10,
            cre: 10,
            dis: 10,
            foc: 10,
            com: 10,
            pre: 10,
            elo: 10,
            sig: 10,
            hea: 10,
            tac: 10,
            olf: 10,
            tas: 10,
            itu: 10,
            ego: 10,
            slf: 10,
            opn: 10,
            blg: 10,
            ext: 10,
            cnf: 10,
            agg: 10,
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
            CharacteristicCode::SPD => self.spd,
            CharacteristicCode::FOR => self.for_stat,
            CharacteristicCode::DEX => self.dex,
            CharacteristicCode::KIN => self.kin,
            CharacteristicCode::HLT => self.hlt,
            CharacteristicCode::FIT => self.fit,
            CharacteristicCode::MAS => (10.0 + (self.mas_kg - 70.0) / 7.0).round() as i32,
            CharacteristicCode::HGT => (10.0 + (self.hgt_m - 1.75) / 0.05).round() as i32,
            CharacteristicCode::MEM => self.mem,
            CharacteristicCode::ANA => self.ana,
            CharacteristicCode::INS => self.ins,
            CharacteristicCode::CRE => self.cre,
            CharacteristicCode::DIS => self.dis,
            CharacteristicCode::FOC => self.foc,
            CharacteristicCode::COM => self.com,
            CharacteristicCode::PRE => self.pre,
            CharacteristicCode::ELO => self.elo,
            CharacteristicCode::SIG => self.sig,
            CharacteristicCode::HEA => self.hea,
            CharacteristicCode::TAC => self.tac,
            CharacteristicCode::OLF => self.olf,
            CharacteristicCode::TAS => self.tas,
            CharacteristicCode::ITU => self.itu,
            CharacteristicCode::EGO => self.ego,
            CharacteristicCode::SLF => self.slf,
            CharacteristicCode::OPN => self.opn,
            CharacteristicCode::BLG => self.blg,
            CharacteristicCode::EXT => self.ext,
            CharacteristicCode::CNF => self.cnf,
            CharacteristicCode::AGG => self.agg,
        };
        raw.clamp(1, 20)
    }

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
        let spd = self.spd.clamp(1, 20);
        let for_val = self.for_stat.clamp(1, 20);
        let kin = self.kin.clamp(1, 20);
        let hlt = self.hlt.clamp(1, 20);
        let fit = self.fit.clamp(1, 20);
        let ana = self.ana.clamp(1, 20);
        let ins = self.ins.clamp(1, 20);
        let cre = self.cre.clamp(1, 20);
        let dis = self.dis.clamp(1, 20);
        let foc = self.foc.clamp(1, 20);
        let sig = self.sig.clamp(1, 20);
        let hea = self.hea.clamp(1, 20);
        let itu = self.itu.clamp(1, 20);
        let mas_kg = self.mas_kg.max(20.0);
        let hgt_m = self.hgt_m.clamp(0.5, 3.0);

        // 1. Striking Power (POW) = (FOR * SPD) / 10.0
        let pow_striking_power = f64::from(for_val * spd) / 10.0;

        // 2. Mass-Momentum Factor (MMF) = (MAS / 70.0) * SPD
        let mmf_momentum_factor = (mas_kg / 70.0) * f64::from(spd);

        // 3. Melee Reach Index = Height Reach Edge ((HGT - 1.75) / 0.15) + Kinesthetic Extension ((KIN - 10) / 4) + Weapon Length
        let height_delta = hgt_m - 1.75;
        let height_reach_edge =
            ((height_delta + 1e-9 * height_delta.signum()) / 0.15).trunc() as i32;
        let kin_reach_edge = (kin - 10).div_euclid(4);
        let melee_reach_index = height_reach_edge + kin_reach_edge + weapon_reach_bonus;

        // 4. Quick Thinking (D_cog) = floor((max(ANA, CRE) + FOC) / 2)
        let d_cog_quick_thinking = (ana.max(cre) + foc).div_euclid(2);

        // 5. OODA Initiative (OODA) = floor((ITU + INS + D_cog + SPD) / 4)
        let ooda_initiative = (itu + ins + d_cog_quick_thinking + spd).div_euclid(4);

        // 6. General Awareness (AWR) = floor((max(SIG, HEA) + FOC + ITU) / 3)
        let awr_general_awareness = (sig.max(hea) + foc + itu).div_euclid(3);

        // 7. Fatigue Threshold (FT) = FIT + floor(DIS / 2)
        let ft_fatigue_threshold = fit + dis.div_euclid(2);

        // 8. Wound Shock Resistance (WSR) = HLT + floor(DIS / 2)
        let wsr_wound_shock_resistance = hlt + dis.div_euclid(2);

        // 9. Social Composure Threshold (CMP) = floor((DIS + FOC) / 4)
        let cmp_social_composure_threshold = (dis + foc).div_euclid(4);

        // 10. Base Academic Study Controller (C_study with APT = 0 using max(MEM, ANA))
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
    if mem >= ana {
        mem
    } else {
        ana
    }
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
}
