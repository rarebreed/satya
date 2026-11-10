//! # Psyche, Morale, Motivation & Social Combat (`04-psyche-and-social`)
//!
//! Canonical Rust implementation of `docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md`:
//! - **§1 Acting Against One's Psyche**: Friction Checks (`DIS` vs. GM 1-Pair Tiered Resistance
//!   $\text{TN}_{GM} = 21 - \Delta_{\text{psyche}}$) and Long-Term **Psyche Drift** ($\max(2, \lfloor\text{DIS}/4\rfloor)$ marks).
//! - **§2 Purpose, Principles & Conviction Surges**: Once-per-scene/session surge when $\Delta_{\text{int}} \ge +4$
//!   and an action embodies an extreme Psyche trait ($P \ge 15$ or $P \le 6$), granting `-6` to $\Delta_{\text{int}}$
//!   (down to `-2`, $\delta_{\text{under}} = 4$), ignoring Fatigue/Shock `action_time_factor` penalties, and `+2` $P_{\text{eff}}$.
//! - **§3 Psychological State of Combat, Aggression & Morale**: Motivation modifiers ($\Delta_{\text{mot}}$),
//!   Emotional Triggers (`DIS` Composure checks), Battlefield Circumstantial Morale penalties ($\Delta_{\text{morale}}$),
//!   and Contested Morale Rolls across the `High -> Steady -> Wavering -> Broken -> Panic` (plus `Rage`) ladder.
//! - **§4 Social Combat, Debate & Cognitive Dissonance ("Mental Wounds")**: Empathy & Read Person setup,
//!   the 7 Argument Vectors ($\Delta_{\text{ext}}$ Resonance vs. Clash), Social Composure Threshold
//!   ($\text{CMP} = \lfloor(\text{DIS} + \text{FOC}) / 4\rfloor$), the 4 Social Wounds (`RhetoricalStrain`,
//!   `Doubt`, `CognitiveDissonance`, `EgoWound`), the Dual-Psyche Wedge, and the 4 Dissonance Resolution Paths.

use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};

use crate::characteristics::{CharacterCharacteristics, CharacteristicCode};
use crate::combat::MoraleState;
use crate::define_coded_enum;
use crate::dice_probabilities::{
    CascadeRollOutcome, CascadeTaskParams, ContestedCascadeOdds, ContestedCascadeRollOutcome,
    ContestedCascadeTaskParams, ContestedOutcomeKind, ContestedTieRule,
};
use crate::primitives::{ContestedCheck, TieredSaveEval};

pub use crate::social_combat::*;
// ============================================================================
// §1 Core Psyche Profile & Trait Definitions
// ============================================================================

define_coded_enum! {
    /// The 7 canonical Bipolar Psyche Traits (`1..=20`, neutral equilibrium `10`)
    /// from `docs/ttrpg/03-character/characteristics_and_aptitudes.md` §7.2 and
    /// `docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §1.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum PsycheTraitId [count = 7] {
        /// **Openness (`OPN`)**: Low (`1–7`) Dogmatic / Rigid / Closed-Minded vs. High (`13–20`) Curious / Receptive / Open-Minded.
        Openness => { code: "OPN", label: "Openness" },
        /// **Selfishness (`SLF`)**: Low (`1–7`) Altruistic / Self-Sacrificing vs. High (`13–20`) Entitled / Mercenary / Acquisitive.
        Selfishness => { code: "SLF", label: "Selfishness" },
        /// **Belonging (`BLG`)**: Low (`1–7`) Detached / Lone Wolf vs. High (`13–20`) Tribal / Group-Dependent.
        Belonging => { code: "BLG", label: "Belonging", aliases: ["BELONGINGNESS"] },
        /// **Aggression (`AGG`)**: Low (`1–7`) Passive / Conflict-Averse vs. High (`13–20`) Combative / Dominant.
        Aggression => { code: "AGG", label: "Aggression", aliases: ["AGGRESSIVENESS"] },
        /// **Ego (`EGO`)**: Low (`1–7`) Humble / Self-Effacing vs. High (`13–20`) Proud / Grandiose / Narcissistic.
        Ego => { code: "EGO", label: "Ego" },
        /// **Conformity (`CNF`)**: Low (`1–7`) Rebellious / Iconoclastic / Maverick vs. High (`13–20`) Institutionalist / By-the-Book.
        Conformity => { code: "CNF", label: "Conformity" },
        /// **Extraversion (`EXT`)**: Low (`1–7`) Introverted / Solitary / Guarded vs. High (`13–20`) Gregarious / Thrill-Seeking.
        Extraversion => { code: "EXT", label: "Extraversion" },
    }
}

impl PsycheTraitId {
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.as_str()
    }

    #[must_use]
    pub const fn to_characteristic_code(self) -> CharacteristicCode {
        match self {
            Self::Openness => CharacteristicCode::OPN,
            Self::Selfishness => CharacteristicCode::SLF,
            Self::Belonging => CharacteristicCode::BLG,
            Self::Aggression => CharacteristicCode::AGG,
            Self::Ego => CharacteristicCode::EGO,
            Self::Conformity => CharacteristicCode::CNF,
            Self::Extraversion => CharacteristicCode::EXT,
        }
    }
}

/// Which pole of a Psyche trait is being violated by a proposed action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PsycheViolationPole {
    /// Action violates a High-Pole inclination (triggers when trait score $P \ge 13$).
    HighPole,
    /// Action violates a Low-Pole inclination (triggers when trait score $P \le 7$).
    LowPole,
}

macro_rules! impl_psyche_profile {
    ($($Variant:ident => $field:ident),+ $(,)?) => {
        /// A character's 6-trait Psyche profile (`1..=20`, default `10` for all traits).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub struct PsycheProfile {
            $(
                pub $field: i32,
            )+
        }

        impl Default for PsycheProfile {
            fn default() -> Self {
                Self { $($field: 10,)+ }
            }
        }

        impl PsycheProfile {
            #[must_use]
            pub fn get(&self, trait_id: PsycheTraitId) -> i32 {
                self[trait_id]
            }

            pub fn set(&mut self, trait_id: PsycheTraitId, value: i32) {
                self[trait_id] = value.clamp(1, 20);
            }
        }

        impl Index<PsycheTraitId> for PsycheProfile {
            type Output = i32;

            fn index(&self, trait_id: PsycheTraitId) -> &Self::Output {
                match trait_id {
                    $(PsycheTraitId::$Variant => &self.$field,)+
                }
            }
        }

        impl IndexMut<PsycheTraitId> for PsycheProfile {
            fn index_mut(&mut self, trait_id: PsycheTraitId) -> &mut Self::Output {
                match trait_id {
                    $(PsycheTraitId::$Variant => &mut self.$field,)+
                }
            }
        }

        impl Index<PsycheTraitId> for CharacterCharacteristics {
            type Output = i32;

            fn index(&self, trait_id: PsycheTraitId) -> &Self::Output {
                match trait_id {
                    $(PsycheTraitId::$Variant => &self.$field,)+
                }
            }
        }

        impl IndexMut<PsycheTraitId> for CharacterCharacteristics {
            fn index_mut(&mut self, trait_id: PsycheTraitId) -> &mut Self::Output {
                match trait_id {
                    $(PsycheTraitId::$Variant => &mut self.$field,)+
                }
            }
        }

        impl From<&CharacterCharacteristics> for PsycheProfile {
            fn from(chars: &CharacterCharacteristics) -> Self {
                Self {
                    $($field: chars.$field.clamp(1, 20),)+
                }
            }
        }

        impl CharacterCharacteristics {
            #[must_use]
            pub fn psyche_profile(&self) -> PsycheProfile {
                PsycheProfile::from(self)
            }

            pub fn set_psyche_profile(&mut self, profile: PsycheProfile) {
                $(self.$field = profile.$field.clamp(1, 20);)+
            }
        }
    };
}

impl_psyche_profile! {
    Openness => opn,
    Selfishness => slf,
    Belonging => blg,
    Aggression => agg,
    Ego => ego,
    Conformity => cnf,
    Extraversion => ext,
}

/// Trait implemented by any type that exposes the 7 canonical Psyche scores ([`PsycheProfile`] and [`CharacterCharacteristics`]).
pub trait HasPsyche: Index<PsycheTraitId, Output = i32> {
    fn as_psyche_profile(&self) -> PsycheProfile;
}

impl HasPsyche for PsycheProfile {
    fn as_psyche_profile(&self) -> PsycheProfile {
        *self
    }
}

impl HasPsyche for CharacterCharacteristics {
    fn as_psyche_profile(&self) -> PsycheProfile {
        self.psyche_profile()
    }
}

impl PsycheProfile {
    /// Returns all traits sorted by descending absolute deviation $\Delta_{\text{psyche}} = |P - 10|$.
    pub fn traits_by_extremity(&self) -> Vec<(PsycheTraitId, i32, i32)> {
        let mut list: Vec<_> = PsycheTraitId::ALL
            .into_iter()
            .map(|id| (id, self[id], psyche_deviation(self[id])))
            .collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.2));
        list
    }
}

/// Computes the Psyche Deviation $\Delta_{\text{psyche}} = |P - 10|$
/// (`docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §1.2, line 27).
pub fn psyche_deviation(trait_score: i32) -> i32 {
    (trait_score.clamp(1, 20) - 10).abs()
}

/// Determines whether an action violating `pole` triggers a Psyche Friction Check (`§1.1`):
/// - High-Pole violation triggers when $P \ge 13$.
/// - Low-Pole violation triggers when $P \le 7$.
pub fn triggers_psyche_friction_check(trait_score: i32, pole: PsycheViolationPole) -> bool {
    match pole {
        PsycheViolationPole::HighPole => trait_score >= 13,
        PsycheViolationPole::LowPole => trait_score <= 7,
    }
}

// ============================================================================
// §1.2 – §1.3 Psyche Friction Checks & Long-Term Psyche Drift
// ============================================================================

/// Parameters for a Contested Psyche Friction Check (`§1.2`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsycheFrictionCheckParams {
    pub trait_id: PsycheTraitId,
    /// Current score of the violated trait (`1..=20`).
    pub trait_score: i32,
    pub violated_pole: PsycheViolationPole,
    /// Character's Mental Discipline (`DIS`, `1..=20`).
    pub dis: i32,
    pub has_sere_training: bool,
    /// Current internal penalty $\Delta_{\text{int}}$ on the character.
    pub delta_int: i32,
    /// Current external penalty $\Delta_{\text{ext}}$ on the friction check.
    pub delta_ext: i32,
}

/// Narrative and mechanical resolution tier of a Psyche Friction Check (`§1.2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PsycheFrictionResolutionKind {
    /// Trait is near neutral ($8 \le P \le 12$); no Friction Check required.
    NoFrictionRequired,
    /// **Clean Override ($S_P > S_{GM}$)**: Executes action without hesitation (`0` strain, `+1` Drift Mark).
    CleanOverride,
    /// **Strained Override (`PartialSuccess`)**: Executes action, suffers `+2` $\Delta_{\text{int}}$ for the scene (`+1` Drift Mark).
    StrainedOverride,
    /// **Balk / Freeze (`Failure`)**: Character balks (`6 ticks` freeze on Action Clock) or spends a full turn steeling at `+4` $\Delta_{\text{int}}$.
    BalkOrFreeze,
}

/// Complete outcome of a Psyche Friction Check (`§1.2`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PsycheFrictionOutcome {
    pub triggered: bool,
    /// Psyche deviation $\Delta_{\text{psyche}} = |P - 10|$.
    pub delta_psyche: i32,
    pub player_tn: i32,
    pub gm_tn: i32,
    pub resolution_kind: PsycheFrictionResolutionKind,
    /// Internal penalty $\Delta_{\text{int}}$ added for the remainder of the scene (`0`, `+2`, or `+4` if forced after balking).
    pub delta_int_penalty_gained: i32,
    pub hesitation_ticks: i32,
    pub drift_marks_gained: i32,
    pub contested_roll: Option<ContestedCascadeRollOutcome>,
}

/// Builds the canonical [`ContestedCascadeTaskParams`] for a Psyche Friction Check (`§1.2`):
/// - Player rolls `DIS` ($N_P = 1$, or $N_P = 2$ with SERE training) against $\text{TN}_P = 31 - \text{DIS}$.
/// - GM rolls `1` Tiered Resistance Pair (`4`-point over-tiers) against $\text{TN}_{GM} = 21 - \Delta_{\text{psyche}}$.
pub fn build_psyche_friction_contest(
    params: &PsycheFrictionCheckParams,
) -> ContestedCascadeTaskParams {
    let n_p = if params.has_sere_training { 2 } else { 1 };
    let delta_psyche = psyche_deviation(params.trait_score);
    ContestedCascadeTaskParams::new_tiered_environment(
        n_p,
        params.dis.clamp(1, 20),
        params.delta_int,
        params.delta_ext,
        delta_psyche,
        ContestedTieRule::HybridSignedMargin,
    )
}

/// Computes exact combinatorial odds for a Psyche Friction Check (`§1.2`).
pub fn compute_psyche_friction_exact_odds(
    params: &PsycheFrictionCheckParams,
) -> ContestedCascadeOdds {
    build_psyche_friction_contest(params).exact_contested_odds()
}

/// Evaluates a live Contested Psyche Friction Roll (`§1.2`).
pub fn evaluate_psyche_friction_roll(
    params: &PsycheFrictionCheckParams,
    player_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
) -> PsycheFrictionOutcome {
    let delta_psyche = psyche_deviation(params.trait_score);
    let contest = build_psyche_friction_contest(params);
    let player_tn = contest.player.tn_eff();
    let gm_tn = contest.gm.tn_eff();

    if !triggers_psyche_friction_check(params.trait_score, params.violated_pole) {
        return PsycheFrictionOutcome {
            triggered: false,
            delta_psyche,
            player_tn,
            gm_tn,
            resolution_kind: PsycheFrictionResolutionKind::NoFrictionRequired,
            delta_int_penalty_gained: 0,
            hesitation_ticks: 0,
            drift_marks_gained: 0,
            contested_roll: None,
        };
    }

    let roll = contest.evaluate_contested_pairs(player_pairs, &[gm_pair]);
    let (resolution_kind, delta_int_penalty_gained, hesitation_ticks, drift_marks_gained) =
        match roll.contested_kind {
            ContestedOutcomeKind::CleanSuccess | ContestedOutcomeKind::ExtraordinarySuccess => {
                (PsycheFrictionResolutionKind::CleanOverride, 0, 0, 1)
            }
            ContestedOutcomeKind::GrazingTieSuccess
            | ContestedOutcomeKind::LuckyBreakSuccess
            | ContestedOutcomeKind::SuccessWithFloorBreach => {
                (PsycheFrictionResolutionKind::StrainedOverride, 2, 0, 1)
            }
            ContestedOutcomeKind::CleanFailure | ContestedOutcomeKind::BotchFailure => {
                (PsycheFrictionResolutionKind::BalkOrFreeze, 4, 6, 0)
            }
        };

    PsycheFrictionOutcome {
        triggered: true,
        delta_psyche,
        player_tn,
        gm_tn,
        resolution_kind,
        delta_int_penalty_gained,
        hesitation_ticks,
        drift_marks_gained,
        contested_roll: Some(roll),
    }
}

/// Computes the Psyche Drift Mark threshold (`§1.3`):
/// $$\text{Drift Threshold} = \max\!\left(2,\; \left\lfloor \frac{\text{DIS}}{4} \right\rfloor\right)$$
pub fn psyche_drift_threshold(dis: i32) -> i32 {
    (dis.clamp(1, 20).div_euclid(4)).max(2)
}

/// Result of recording a Psyche Drift Mark (`§1.3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsycheDriftResult {
    pub previous_score: i32,
    pub new_score: i32,
    pub remaining_drift_marks: i32,
    /// Drift threshold $\max(2, \lfloor\text{DIS}/4\rfloor)$ required for a permanent shift.
    pub drift_threshold: i32,
    pub shifted: bool,
}

/// Applies `marks_added` Psyche Drift Marks to a trait (`§1.3`).
/// When `current_marks + marks_added >= psyche_drift_threshold(dis)`, the trait score permanently
/// shifts by `1` point toward `target_pole_score` (default `10` equilibrium) and resets marks.
pub fn apply_psyche_drift_mark(
    trait_score: i32,
    current_marks: i32,
    marks_added: i32,
    dis: i32,
    target_pole_score: Option<i32>,
) -> PsycheDriftResult {
    let previous_score = trait_score.clamp(1, 20);
    let drift_threshold = psyche_drift_threshold(dis);
    let total_marks = (current_marks.max(0) + marks_added.max(0)).max(0);
    let target = target_pole_score.unwrap_or(10).clamp(1, 20);
    let shifted = total_marks >= drift_threshold && previous_score != target;
    let step = if !shifted {
        0
    } else if target > previous_score {
        1
    } else {
        -1
    };
    PsycheDriftResult {
        previous_score,
        new_score: (previous_score + step).clamp(1, 20),
        remaining_drift_marks: if shifted {
            total_marks - drift_threshold
        } else {
            total_marks
        },
        drift_threshold,
        shifted,
    }
}

// ============================================================================
// §2 Purpose, Principles & Conviction Surges
// ============================================================================

/// Canonical Core Principle archetypes that can trigger a Conviction Surge (`§2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConvictionPrincipleArchetype {
    /// **Altruistic / Tribal Sacrifice** (`BLG >= 15` or `SLF <= 5`): Holding a doorway or taking a bullet for the crew.
    AltruisticOrTribalSacrifice,
    /// **Unbowed Defiance** (`AGG >= 15` or `EGO >= 15`): Refusing to kneel or yield to an overwhelming adversary.
    UnbowedDefiance,
    /// **Iconoclastic Truth** (`OPN >= 15` or `CNF <= 5`): Exposing a conspiracy or defying corrupt authority at personal risk.
    IconoclasticTruth,
    /// **General Extreme Trait Embodiment**: Acting in direct accordance with any Psyche trait at $P \ge 15$ or $P \le 6$.
    ExtremeTraitEmbodiment(PsycheTraitId),
}

/// Mechanical benefits of triggering a **Conviction Surge** (`§2`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvictionSurgeResult {
    pub eligible: bool,
    pub matched_principle: Option<ConvictionPrincipleArchetype>,
    /// Original internal penalty $\Delta_{\text{int}}$ before the surge (must be $\ge +4$ to trigger).
    pub previous_delta_int: i32,
    /// Adjusted internal penalty $\Delta_{\text{int}} = \max(-2, \text{previous\_delta\_int} - 6)$.
    pub adjusted_delta_int: i32,
    /// Effective failure stumble magnitude $\delta_{\text{under}} = \max(0, 6 + \Delta_{\text{int}})$
    /// (reaching $\delta_{\text{under}} = 4$ at Focused/Resolved $\Delta_{\text{int}} = -2$ per
    /// `docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §2).
    pub surge_delta_under: i32,
    pub ignores_fatigue_and_shock_time_penalty: bool,
    /// Immediate bonus to Effective Tactical Pool $P_{\text{eff}}$ (`+2`).
    pub tactical_pool_bonus: i32,
}

/// Evaluates whether a character can trigger a **Conviction Surge** (`§2`) and computes its effects.
///
/// Requirements (`docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §2):
/// 1. Once per scene/session (`!already_used_this_scene`).
/// 2. Character is currently suffering from severe internal strain ($\Delta_{\text{int}} \ge +4$).
/// 3. The action directly embodies a Core Principle / extreme Psyche trait ($P \ge 15$ or $P \le 6$,
///    or one of the three named archetypes).
pub fn evaluate_conviction_surge(
    psyche: &impl HasPsyche,
    current_delta_int: i32,
    already_used_this_scene: bool,
    requested_principle: Option<ConvictionPrincipleArchetype>,
) -> ConvictionSurgeResult {
    let p = psyche.as_psyche_profile();
    let ineligible = ConvictionSurgeResult {
        eligible: false,
        matched_principle: None,
        previous_delta_int: current_delta_int,
        adjusted_delta_int: current_delta_int,
        surge_delta_under: (6 + current_delta_int).max(0),
        ignores_fatigue_and_shock_time_penalty: false,
        tactical_pool_bonus: 0,
    };
    if already_used_this_scene || current_delta_int < 4 {
        return ineligible;
    }

    let check = |arch: ConvictionPrincipleArchetype| -> bool {
        match arch {
            ConvictionPrincipleArchetype::AltruisticOrTribalSacrifice => p.blg >= 15 || p.slf <= 5,
            ConvictionPrincipleArchetype::UnbowedDefiance => p.agg >= 15 || p.ego >= 15,
            ConvictionPrincipleArchetype::IconoclasticTruth => p.opn >= 15 || p.cnf <= 5,
            ConvictionPrincipleArchetype::ExtremeTraitEmbodiment(t) => p[t] >= 15 || p[t] <= 6,
        }
    };

    let matched = match requested_principle {
        Some(arch) => check(arch).then_some(arch),
        None => [
            ConvictionPrincipleArchetype::AltruisticOrTribalSacrifice,
            ConvictionPrincipleArchetype::UnbowedDefiance,
            ConvictionPrincipleArchetype::IconoclasticTruth,
        ]
        .into_iter()
        .find(|&arch| check(arch))
        .or_else(|| {
            PsycheTraitId::ALL
                .into_iter()
                .map(ConvictionPrincipleArchetype::ExtremeTraitEmbodiment)
                .find(|&arch| check(arch))
        }),
    };

    let Some(principle) = matched else {
        return ineligible;
    };
    let adjusted_delta_int = (current_delta_int - 6).max(-2);
    let surge_delta_under = (6 + adjusted_delta_int).max(0);
    ConvictionSurgeResult {
        eligible: true,
        matched_principle: Some(principle),
        previous_delta_int: current_delta_int,
        adjusted_delta_int,
        surge_delta_under,
        ignores_fatigue_and_shock_time_penalty: true,
        tactical_pool_bonus: 2,
    }
}

// ============================================================================
// §3 Psychological State of Combat, Aggression & Morale
// ============================================================================

/// Why a combatant is fighting (`§3.2 Motivation & Psyche Alignment`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatMotivationKind {
    /// **Fighting for Crew, Family, or Homeland** (`BLG >= 14` or `SLF <= 6`).
    FightingForCrewFamilyHomeland,
    /// **Mercenary / Transactional Motive** (`SLF >= 14` and `BLG <= 8`), when wounded or odds turn against them.
    MercenaryTransactionalUnderFire,
    /// **Sworn Duty / Chain of Command** (`CNF >= 14`), with commanding officer present (`true`) or command broken (`false`).
    SwornDutyChainOfCommand { command_intact: bool },
    /// Standard / neutral combat motivation.
    NeutralOrSurvival,
}

/// Computed motivation modifier ($\Delta_{\text{mot}}$) for Combat Morale checks (`§3.2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CombatMotivationModifier {
    /// Modifier $\Delta_{\text{mot}}$ added to $C_{\text{morale}} = \text{DIS} + \Delta_{\text{mot}}$
    /// (equivalent to $-\Delta_{\text{mot}}$ on $\text{TN}_P = 31 - C_{\text{morale}}$).
    pub delta_mot: i32,
    /// Additional Morale Shock ($\Delta_{\text{morale}}$) if command structure breaks down for a High-`CNF` soldier.
    pub command_breakdown_morale_shock: i32,
    pub conviction_surge_pool_bonus: i32,
}

/// Evaluates the Motivation & Psyche Alignment modifier ($\Delta_{\text{mot}}$) from `§3.2`.
pub fn evaluate_combat_motivation_modifier(
    psyche: &impl HasPsyche,
    motivation: CombatMotivationKind,
) -> CombatMotivationModifier {
    let p = psyche.as_psyche_profile();
    match motivation {
        CombatMotivationKind::FightingForCrewFamilyHomeland if p.blg >= 14 || p.slf <= 6 => {
            CombatMotivationModifier {
                delta_mot: if p.blg >= 16 || p.slf <= 4 { 4 } else { 2 },
                command_breakdown_morale_shock: 0,
                conviction_surge_pool_bonus: 2,
            }
        }
        CombatMotivationKind::MercenaryTransactionalUnderFire if p.slf >= 14 && p.blg <= 8 => {
            CombatMotivationModifier {
                delta_mot: if p.slf >= 16 && p.blg <= 6 { -4 } else { -2 },
                ..Default::default()
            }
        }
        CombatMotivationKind::SwornDutyChainOfCommand { command_intact } if p.cnf >= 14 => {
            if command_intact {
                CombatMotivationModifier {
                    delta_mot: 2,
                    ..Default::default()
                }
            } else {
                CombatMotivationModifier {
                    command_breakdown_morale_shock: 4,
                    ..Default::default()
                }
            }
        }
        _ => CombatMotivationModifier::default(),
    }
}

/// Canonical Emotional Triggers during combat (`§3.3 Emotional Triggers Table`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmotionalTriggerKind {
    /// **Being Wounded** (`wound_tier` $1..=4$): $\Delta_{\text{ext}} = +2 \times \text{Wound Tier}$
    /// (`+2` if `SLF >= 13`, `-2` if `AGG >= 14`).
    BeingWounded { wound_tier: i32 },
    /// **Friend / Squadmate Wounded**: Relevant when `BLG >= 13` ($\Delta_{\text{ext}} = +2$).
    FriendOrSquadmateWounded,
    /// **Loved One Attacked / Wounded**: Relevant when `BLG >= 14` or `SLF <= 7` ($\Delta_{\text{ext}} = +4$).
    LovedOneAttackedOrWounded,
    /// **Insulted / Taunted**: Relevant when `EGO >= 13` or `AGG >= 13` ($\Delta_{\text{ext}} = \max(|\text{EGO}-10|, |\text{AGG}-10|)$).
    InsultedOrTaunted,
    /// **Publicly Humiliated / Outclassed**: Relevant when `EGO >= 14` ($\Delta_{\text{ext}} = +4$).
    PubliclyHumiliated,
    /// **Seeing Innocent Suffering**: Relevant when `SLF <= 7` (lessens aggression toward surrendering foes or triggers protective resolve).
    SeeingInnocentSuffering,
}

/// Outcome of a `DIS` Composure Check against an [`EmotionalTriggerKind`] (`§3.3`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmotionalTriggerOutcome {
    pub susceptible: bool,
    /// Computed external penalty $\Delta_{\text{ext}}$ on the `DIS` Composure Check.
    pub delta_ext: i32,
    pub new_morale_state: MoraleState,
    /// Additional internal strain $\Delta_{\text{int}}$ inflicted (e.g. `+2` Ego Wound on `PubliclyHumiliated`).
    pub delta_int_inflicted: i32,
    pub unlocks_protective_conviction: bool,
    pub composure_roll: Option<CascadeRollOutcome>,
}

impl EmotionalTriggerOutcome {
    const fn not_susceptible(current_morale: MoraleState) -> Self {
        Self {
            susceptible: false,
            delta_ext: 0,
            new_morale_state: current_morale,
            delta_int_inflicted: 0,
            unlocks_protective_conviction: false,
            composure_roll: None,
        }
    }
}

fn eval_composure_roll(
    dis: i32,
    current_delta_int: i32,
    delta_ext: i32,
    composure_roll_pairs: &[(i32, i32)],
) -> CascadeRollOutcome {
    CascadeTaskParams::new_k20(
        composure_roll_pairs.len().max(1),
        dis.clamp(1, 20),
        current_delta_int,
        delta_ext,
    )
    .evaluate_pairs(composure_roll_pairs)
}

/// Evaluates an [`EmotionalTriggerKind`] against a character's [`HasPsyche`] profile and `DIS` Composure roll (`§3.3`).
pub fn evaluate_emotional_trigger(
    psyche: &impl HasPsyche,
    dis: i32,
    current_morale: MoraleState,
    current_delta_int: i32,
    trigger: EmotionalTriggerKind,
    composure_roll_pairs: &[(i32, i32)],
) -> EmotionalTriggerOutcome {
    let p = psyche.as_psyche_profile();
    let roll_outcome = |delta_ext: i32,
                        unlocks: bool,
                        on_pass: MoraleState,
                        on_fail: (MoraleState, i32)|
     -> EmotionalTriggerOutcome {
        let roll = eval_composure_roll(dis, current_delta_int, delta_ext, composure_roll_pairs);
        let (new_morale_state, delta_int_inflicted) = if roll.total_successes > 0 {
            (on_pass, 0)
        } else {
            on_fail
        };
        EmotionalTriggerOutcome {
            susceptible: true,
            delta_ext,
            new_morale_state,
            delta_int_inflicted,
            unlocks_protective_conviction: unlocks,
            composure_roll: Some(roll),
        }
    };

    match trigger {
        EmotionalTriggerKind::BeingWounded { wound_tier } => {
            let mut delta_ext = 2 * wound_tier.max(1) + if p.slf >= 13 { 2 } else { 0 };
            if p.agg >= 14 {
                delta_ext = (delta_ext - 2).max(0);
            }
            let fail_st = if p.agg >= 15 && p.slf <= 8 {
                MoraleState::Rage
            } else {
                MoraleState::Wavering
            };
            roll_outcome(delta_ext, false, current_morale, (fail_st, 0))
        }
        EmotionalTriggerKind::FriendOrSquadmateWounded => {
            if p.blg < 13 {
                return EmotionalTriggerOutcome::not_susceptible(current_morale);
            }
            let fail_st = if p.agg >= 12 {
                MoraleState::Rage
            } else if p.agg <= 8 {
                step_morale_condition(current_morale, -1)
            } else {
                current_morale
            };
            roll_outcome(2, false, current_morale, (fail_st, 0))
        }
        EmotionalTriggerKind::LovedOneAttackedOrWounded => {
            if !(p.blg >= 14 || p.slf <= 7) {
                return EmotionalTriggerOutcome::not_susceptible(current_morale);
            }
            roll_outcome(4, true, MoraleState::High, (MoraleState::Rage, 0))
        }
        EmotionalTriggerKind::InsultedOrTaunted => {
            if !(p.ego >= 13 || p.agg >= 13) {
                return EmotionalTriggerOutcome::not_susceptible(current_morale);
            }
            let delta_ext = psyche_deviation(p.ego).max(psyche_deviation(p.agg));
            roll_outcome(delta_ext, false, current_morale, (MoraleState::Rage, 0))
        }
        EmotionalTriggerKind::PubliclyHumiliated => {
            if p.ego < 14 {
                return EmotionalTriggerOutcome::not_susceptible(current_morale);
            }
            let fail_st = if p.agg >= 12 {
                MoraleState::Rage
            } else if p.agg <= 8 {
                MoraleState::Broken
            } else {
                MoraleState::Wavering
            };
            roll_outcome(4, false, current_morale, (fail_st, 2))
        }
        EmotionalTriggerKind::SeeingInnocentSuffering => {
            let susceptible = p.slf <= 7;
            EmotionalTriggerOutcome {
                susceptible,
                delta_ext: 0,
                new_morale_state: if susceptible && current_morale == MoraleState::Rage {
                    MoraleState::Steady
                } else {
                    current_morale
                },
                delta_int_inflicted: 0,
                unlocks_protective_conviction: susceptible,
                composure_roll: None,
            }
        }
    }
}

/// Battlefield situational inputs for Circumstantial Morale Penalties ($\Delta_{\text{morale}}$, `§3.4`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BattlefieldMoraleSituation {
    pub ally_casualty_ratio: f64,
    pub outnumbered_ratio: f64,
    pub leader_incapacitated_or_comms_cut: bool,
    pub position_overrun_flanked_or_ambushed: bool,
    /// Remaining ammunition fraction (`0.0..=1.0`; `<= 0.20 -> +4`).
    pub ammo_fraction_remaining: f64,
    pub tracks_ammo: bool,
    pub primary_weapon_compromised: bool,
    pub attacks_ineffective_against_heavy_armor: bool,
    pub started_fatigued_or_sleep_deprived: bool,
    /// Additional Morale Shock (e.g., $\lfloor F / 2 \rfloor$ from incoming suppressive fire).
    pub additional_suppression_or_shock_delta: i32,
}

/// Computes the total Circumstantial Morale Penalty $\Delta_{\text{morale}}$ from `§3.4`.
pub fn compute_battlefield_morale_delta(situation: &BattlefieldMoraleSituation, cnf: i32) -> i32 {
    let casualty = if situation.ally_casualty_ratio >= 0.75 {
        6
    } else if situation.ally_casualty_ratio >= 0.50 {
        4
    } else if situation.ally_casualty_ratio >= 0.25 {
        2
    } else {
        0
    };
    let outnumbered = if situation.outnumbered_ratio > 4.0 {
        6
    } else if situation.outnumbered_ratio >= 3.0 {
        4
    } else if situation.outnumbered_ratio >= 2.0 {
        2
    } else {
        0
    };
    let leader = if situation.leader_incapacitated_or_comms_cut {
        if cnf >= 14 { 4 } else { 2 }
    } else {
        0
    };
    let ammo_or_wpn = if (situation.tracks_ammo && situation.ammo_fraction_remaining <= 0.20)
        || situation.primary_weapon_compromised
    {
        4
    } else {
        0
    };

    casualty
        + outnumbered
        + leader
        + ammo_or_wpn
        + if situation.position_overrun_flanked_or_ambushed {
            6
        } else {
            0
        }
        + if situation.attacks_ineffective_against_heavy_armor {
            6
        } else {
            0
        }
        + if situation.started_fatigued_or_sleep_deprived {
            2
        } else {
            0
        }
        + situation.additional_suppression_or_shock_delta.max(0)
}

/// Moves a combatant along the canonical 5-step Morale Condition Ladder (`§3.1`, `§3.4`):
/// `High (0) <-> Steady (1) <-> Wavering (2) <-> Broken (3) <-> Panic (4)`.
///
/// Positive `rally_or_drop_steps` moves toward `High`; negative moves toward `Panic`.
/// If starting from `Rage`, a negative step drops into `Wavering` (`-1`) or `Broken` (`-2`).
pub fn step_morale_condition(current: MoraleState, rally_or_drop_steps: i32) -> MoraleState {
    const LADDER: [MoraleState; 5] = [
        MoraleState::High,
        MoraleState::Steady,
        MoraleState::Wavering,
        MoraleState::Broken,
        MoraleState::Panic,
    ];
    let current_idx: i32 = match current {
        MoraleState::High => 0,
        MoraleState::Steady | MoraleState::Rage => 1,
        MoraleState::Wavering => 2,
        MoraleState::Broken => 3,
        MoraleState::Panic => 4,
    };
    if current == MoraleState::Rage && rally_or_drop_steps >= 0 {
        return if rally_or_drop_steps > 0 {
            MoraleState::High
        } else {
            MoraleState::Rage
        };
    }
    LADDER[(current_idx - rally_or_drop_steps).clamp(0, 4) as usize]
}

/// Domain-specific morale transition payload for [`CombatMoraleRollOutcome`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatMoraleEffect {
    /// Base Morale Controller $C_{\text{morale}} = \text{DIS} + \Delta_{\text{mot}}$.
    pub c_morale: i32,
    pub previous_morale: MoraleState,
    pub new_morale: MoraleState,
    pub condition_step_delta: i32,
}

/// Outcome of a Contested Combat Morale Roll (`§3.4`).
pub type CombatMoraleRollOutcome = ContestedCheck<CombatMoraleEffect>;

/// Evaluates a Contested Combat Morale Roll (`§3.4 Rolling for Morale`):
/// - $C_{\text{morale}} = \text{DIS} + \Delta_{\text{mot}}$
/// - $N_P = 2$ if veteran/SERE/Command trained, else $N_P = 1$
/// - $\text{TN}_P = 31 - C_{\text{morale}}$ vs. GM 1-Pair Tiered Resistance $\text{TN}_{GM} = 21 - \Delta_{\text{morale}}$
/// - **Extraordinary Success** ($\Delta M \ge +2$): Rallies `+1` step toward `High`.
/// - **Clean / Partial Success**: Holds current Morale State (`0` steps).
/// - **Clean Failure**: Drops `1` Condition Step (`-1`).
/// - **Botch Failure**: Drops `2` Condition Steps (`-2`).
#[allow(clippy::too_many_arguments)]
pub fn evaluate_combat_morale_roll(
    dis: i32,
    delta_mot: i32,
    delta_int: i32,
    delta_morale: i32,
    is_veteran_or_sere: bool,
    current_morale: MoraleState,
    player_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
) -> CombatMoraleRollOutcome {
    let c_morale = (dis + delta_mot).clamp(1, 25);
    let n_p = if is_veteran_or_sere { 2 } else { 1 };
    let save = TieredSaveEval::evaluate(
        n_p,
        c_morale,
        delta_int,
        0,
        delta_morale,
        ContestedTieRule::HybridSignedMargin,
        player_pairs,
        gm_pair,
    );

    let step_delta = save.classify_4way(
        |_| 1,
        |r| {
            if r.contested_kind == ContestedOutcomeKind::CleanSuccess && r.net_signed_margin >= 2 {
                1
            } else {
                0
            }
        },
        |_| -1,
        |_| -2,
    );

    ContestedCheck::new(
        save,
        CombatMoraleEffect {
            c_morale,
            previous_morale: current_morale,
            new_morale: step_morale_condition(current_morale, step_delta),
            condition_step_delta: step_delta,
        },
    )
}
