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

use serde::{Deserialize, Serialize};

use crate::dice_probabilities::{
    CascadeRollOutcome, CascadeTaskParams, ContestedCascadeOdds, ContestedCascadeRollOutcome,
    ContestedCascadeTaskParams, ContestedOutcomeKind, ContestedTieRule,
};
use crate::rules_book::MoraleState;

// ============================================================================
// §1 Core Psyche Profile & Trait Definitions
// ============================================================================

/// The 6 canonical Bipolar Psyche Traits (`1..=20`, neutral equilibrium `10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PsycheTraitId {
    /// **Openness (`OPN`)**: Low (`1–7`) Dogmatic / Traditionalist vs. High (`13–20`) Curious / Iconoclastic.
    Openness,
    /// **Selfishness (`SLF`)**: Low (`1–7`) Altruistic / Self-Sacrificing vs. High (`13–20`) Mercenary / Self-Preserving.
    Selfishness,
    /// **Belonging (`BLG`)**: Low (`1–7`) Lone Wolf / Detached vs. High (`13–20`) Tribal / Devoted to Crew.
    Belonging,
    /// **Aggression (`AGG`)**: Low (`1–7`) Pacifist / Conflict-Averse vs. High (`13–20`) Belligerent / Ruthless.
    Aggression,
    /// **Ego (`EGO`)**: Low (`1–7`) Humble / Self-Effacing vs. High (`13–20`) Proud / Glory-Seeking.
    Ego,
    /// **Conformity (`CNF`)**: Low (`1–7`) Rebellious / Maverick vs. High (`13–20`) Lawful / Disciplined / Dutiful.
    Conformity,
}

impl PsycheTraitId {
    /// All 6 Psyche traits in canonical order.
    pub const ALL: [Self; 6] = [
        Self::Openness,
        Self::Selfishness,
        Self::Belonging,
        Self::Aggression,
        Self::Ego,
        Self::Conformity,
    ];

    /// Three-letter canonical code (`OPN`, `SLF`, `BLG`, `AGG`, `EGO`, `CNF`).
    pub fn code(self) -> &'static str {
        match self {
            Self::Openness => "OPN",
            Self::Selfishness => "SLF",
            Self::Belonging => "BLG",
            Self::Aggression => "AGG",
            Self::Ego => "EGO",
            Self::Conformity => "CNF",
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

/// A character's 6-trait Psyche profile (`1..=20`, default `10` for all traits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsycheProfile {
    /// Openness (`OPN`, `1..=20`).
    pub opn: i32,
    /// Selfishness (`SLF`, `1..=20`).
    pub slf: i32,
    /// Belonging (`BLG`, `1..=20`).
    pub blg: i32,
    /// Aggression (`AGG`, `1..=20`).
    pub agg: i32,
    /// Ego (`EGO`, `1..=20`).
    pub ego: i32,
    /// Conformity (`CNF`, `1..=20`).
    pub cnf: i32,
}

impl Default for PsycheProfile {
    fn default() -> Self {
        Self {
            opn: 10,
            slf: 10,
            blg: 10,
            agg: 10,
            ego: 10,
            cnf: 10,
        }
    }
}

impl PsycheProfile {
    /// Returns the score (`1..=20`) of the requested [`PsycheTraitId`].
    pub fn get(&self, trait_id: PsycheTraitId) -> i32 {
        match trait_id {
            PsycheTraitId::Openness => self.opn,
            PsycheTraitId::Selfishness => self.slf,
            PsycheTraitId::Belonging => self.blg,
            PsycheTraitId::Aggression => self.agg,
            PsycheTraitId::Ego => self.ego,
            PsycheTraitId::Conformity => self.cnf,
        }
    }

    /// Sets the score (`1..=20`) of the requested [`PsycheTraitId`].
    pub fn set(&mut self, trait_id: PsycheTraitId, value: i32) {
        let clamped = value.clamp(1, 20);
        match trait_id {
            PsycheTraitId::Openness => self.opn = clamped,
            PsycheTraitId::Selfishness => self.slf = clamped,
            PsycheTraitId::Belonging => self.blg = clamped,
            PsycheTraitId::Aggression => self.agg = clamped,
            PsycheTraitId::Ego => self.ego = clamped,
            PsycheTraitId::Conformity => self.cnf = clamped,
        }
    }

    /// Returns all traits sorted by descending absolute deviation $\Delta_{\text{psyche}} = |P - 10|$.
    pub fn traits_by_extremity(&self) -> Vec<(PsycheTraitId, i32, i32)> {
        let mut list: Vec<(PsycheTraitId, i32, i32)> = PsycheTraitId::ALL
            .into_iter()
            .map(|id| {
                let val = self.get(id);
                (id, val, psyche_deviation(val))
            })
            .collect();
        list.sort_by(|a, b| b.2.cmp(&a.2));
        list
    }
}

/// Computes the Psyche Deviation $\Delta_{\text{psyche}} = |P - 10|$ (`§1.1`).
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
    /// Which Psyche trait is being violated.
    pub trait_id: PsycheTraitId,
    /// Current score of the violated trait (`1..=20`).
    pub trait_score: i32,
    /// Which pole of the trait is being violated.
    pub violated_pole: PsycheViolationPole,
    /// Character's Mental Discipline (`DIS`, `1..=20`).
    pub dis: i32,
    /// Whether the character is trained in *Counter-Interrogation & Resistance (SERE)* ($N_P = 2$ vs. $N_P = 1$).
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
    /// Whether the trait score was extreme enough ($P \ge 13$ or $P \le 7$) to trigger a check.
    pub triggered: bool,
    /// Psyche deviation $\Delta_{\text{psyche}} = |P - 10|$.
    pub delta_psyche: i32,
    /// Player target number $\text{TN}_P = 31 - \text{DIS} + \Delta_{\text{ext}}$.
    pub player_tn: i32,
    /// GM 1-Pair Tiered Resistance target number $\text{TN}_{GM} = 21 - \Delta_{\text{psyche}}$.
    pub gm_tn: i32,
    /// Resolution tier of the friction check.
    pub resolution_kind: PsycheFrictionResolutionKind,
    /// Internal penalty $\Delta_{\text{int}}$ added for the remainder of the scene (`0`, `+2`, or `+4` if forced after balking).
    pub delta_int_penalty_gained: i32,
    /// Action Clock hesitation freeze in ticks (`6` on `BalkOrFreeze`, else `0`).
    pub hesitation_ticks: i32,
    /// Number of Psyche Drift Marks gained (`1` when overriding the trait, `0` when balking).
    pub drift_marks_gained: i32,
    /// Detailed contested cascade roll outcome (if a roll was triggered).
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
    /// Previous trait score before checking threshold.
    pub previous_score: i32,
    /// Updated trait score (`1..=20`), shifted by `1` if `shifted == true`.
    pub new_score: i32,
    /// Remaining Drift Marks on this trait (`0` if a permanent shift occurred).
    pub remaining_drift_marks: i32,
    /// Drift threshold $\max(2, \lfloor\text{DIS}/4\rfloor)$ required for a permanent shift.
    pub drift_threshold: i32,
    /// Whether the trait permanently shifted by `1` point.
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
    let threshold = psyche_drift_threshold(dis);
    let total_marks = (current_marks.max(0) + marks_added.max(0)).max(0);
    let target = target_pole_score.unwrap_or(10).clamp(1, 20);

    if total_marks >= threshold && previous_score != target {
        let step = if target > previous_score { 1 } else { -1 };
        let new_score = (previous_score + step).clamp(1, 20);
        PsycheDriftResult {
            previous_score,
            new_score,
            remaining_drift_marks: total_marks - threshold,
            drift_threshold: threshold,
            shifted: true,
        }
    } else {
        PsycheDriftResult {
            previous_score,
            new_score: previous_score,
            remaining_drift_marks: total_marks,
            drift_threshold: threshold,
            shifted: false,
        }
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
    /// Whether the character meets all requirements to trigger the Conviction Surge.
    pub eligible: bool,
    /// Matched Core Principle archetype (if eligible).
    pub matched_principle: Option<ConvictionPrincipleArchetype>,
    /// Original internal penalty $\Delta_{\text{int}}$ before the surge (must be $\ge +4$ to trigger).
    pub previous_delta_int: i32,
    /// Adjusted internal penalty $\Delta_{\text{int}} = \max(-2, \text{previous\_delta\_int} - 6)$.
    pub adjusted_delta_int: i32,
    /// Effective cascade carry bonus $\delta_{\text{under}} = (6 - \text{adjusted\_delta\_int}).\text{clamp}(0, 8)$
    /// (Wait: in Satya, negative $\Delta_{\text{int}}$ increases $\delta_{\text{under}}$ up to `4` below base or `8` total;
    /// specifically §2 states: *"reducing $\Delta_{\text{int}}$ by `-6` (down to a minimum of `-2`, granting $\delta_{\text{under}} = 4$)"*).
    pub surge_delta_under: i32,
    /// Whether Fatigue and Wound Shock `action_time_factor` penalties are ignored for this exchange (`1.0`).
    pub ignores_fatigue_and_shock_time_penalty: bool,
    /// Immediate bonus to Effective Tactical Pool $P_{\text{eff}}$ (`+2`).
    pub tactical_pool_bonus: i32,
}

/// Evaluates whether a character can trigger a **Conviction Surge** (`§2`) and computes its effects.
///
/// Requirements (`§2`):
/// 1. Once per scene/session (`!already_used_this_scene`).
/// 2. Character is currently suffering from severe internal strain ($\Delta_{\text{int}} \ge +4$).
/// 3. The action directly embodies a Core Principle / extreme Psyche trait ($P \ge 15$ or $P \le 6$,
///    or one of the three named archetypes).
pub fn evaluate_conviction_surge(
    profile: &PsycheProfile,
    current_delta_int: i32,
    already_used_this_scene: bool,
    requested_principle: Option<ConvictionPrincipleArchetype>,
) -> ConvictionSurgeResult {
    if already_used_this_scene || current_delta_int < 4 {
        return ConvictionSurgeResult {
            eligible: false,
            matched_principle: None,
            previous_delta_int: current_delta_int,
            adjusted_delta_int: current_delta_int,
            surge_delta_under: (6 - current_delta_int).max(0),
            ignores_fatigue_and_shock_time_penalty: false,
            tactical_pool_bonus: 0,
        };
    }

    let matched = match requested_principle {
        Some(ConvictionPrincipleArchetype::AltruisticOrTribalSacrifice) => {
            if profile.blg >= 15 || profile.slf <= 5 {
                Some(ConvictionPrincipleArchetype::AltruisticOrTribalSacrifice)
            } else {
                None
            }
        }
        Some(ConvictionPrincipleArchetype::UnbowedDefiance) => {
            if profile.agg >= 15 || profile.ego >= 15 {
                Some(ConvictionPrincipleArchetype::UnbowedDefiance)
            } else {
                None
            }
        }
        Some(ConvictionPrincipleArchetype::IconoclasticTruth) => {
            if profile.opn >= 15 || profile.cnf <= 5 {
                Some(ConvictionPrincipleArchetype::IconoclasticTruth)
            } else {
                None
            }
        }
        Some(ConvictionPrincipleArchetype::ExtremeTraitEmbodiment(t)) => {
            let score = profile.get(t);
            if score >= 15 || score <= 6 {
                Some(ConvictionPrincipleArchetype::ExtremeTraitEmbodiment(t))
            } else {
                None
            }
        }
        None => {
            if profile.blg >= 15 || profile.slf <= 5 {
                Some(ConvictionPrincipleArchetype::AltruisticOrTribalSacrifice)
            } else if profile.agg >= 15 || profile.ego >= 15 {
                Some(ConvictionPrincipleArchetype::UnbowedDefiance)
            } else if profile.opn >= 15 || profile.cnf <= 5 {
                Some(ConvictionPrincipleArchetype::IconoclasticTruth)
            } else {
                PsycheTraitId::ALL
                    .into_iter()
                    .find(|&t| {
                        let s = profile.get(t);
                        s >= 15 || s <= 6
                    })
                    .map(ConvictionPrincipleArchetype::ExtremeTraitEmbodiment)
            }
        }
    };

    if let Some(principle) = matched {
        let adjusted_delta_int = (current_delta_int - 6).max(-2);
        // Per §2: "reducing Delta_int by -6 (down to a minimum of -2, granting delta_under = 4)"
        let surge_delta_under = if adjusted_delta_int <= -2 {
            4
        } else {
            (6 - adjusted_delta_int).clamp(0, 6)
        };
        ConvictionSurgeResult {
            eligible: true,
            matched_principle: Some(principle),
            previous_delta_int: current_delta_int,
            adjusted_delta_int,
            surge_delta_under,
            ignores_fatigue_and_shock_time_penalty: true,
            tactical_pool_bonus: 2,
        }
    } else {
        ConvictionSurgeResult {
            eligible: false,
            matched_principle: None,
            previous_delta_int: current_delta_int,
            adjusted_delta_int: current_delta_int,
            surge_delta_under: (6 - current_delta_int).max(0),
            ignores_fatigue_and_shock_time_penalty: false,
            tactical_pool_bonus: 0,
        }
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
    SwornDutyChainOfCommand {
        /// Whether the commanding officer is present and issuing orders (`true`) or incapacitated/cut off (`false`).
        command_intact: bool,
    },
    /// Standard / neutral combat motivation.
    NeutralOrSurvival,
}

/// Computed motivation modifier ($\Delta_{\text{mot}}$) for Combat Morale checks (`§3.2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatMotivationModifier {
    /// Modifier $\Delta_{\text{mot}}$ added to $C_{\text{morale}} = \text{DIS} + \Delta_{\text{mot}}$
    /// (equivalent to $-\Delta_{\text{mot}}$ on $\text{TN}_P = 31 - C_{\text{morale}}$).
    pub delta_mot: i32,
    /// Additional Morale Shock ($\Delta_{\text{morale}}$) if command structure breaks down for a High-`CNF` soldier.
    pub command_breakdown_morale_shock: i32,
    /// Bonus to Tactical Pool during a Conviction Surge (`+2` for Crew/Family/Homeland).
    pub conviction_surge_pool_bonus: i32,
}

/// Evaluates the Motivation & Psyche Alignment modifier ($\Delta_{\text{mot}}$) from `§3.2`.
pub fn evaluate_combat_motivation_modifier(
    profile: &PsycheProfile,
    motivation: CombatMotivationKind,
) -> CombatMotivationModifier {
    match motivation {
        CombatMotivationKind::FightingForCrewFamilyHomeland => {
            if profile.blg >= 14 || profile.slf <= 6 {
                let bonus = if profile.blg >= 16 || profile.slf <= 4 {
                    4
                } else {
                    2
                };
                CombatMotivationModifier {
                    delta_mot: bonus,
                    command_breakdown_morale_shock: 0,
                    conviction_surge_pool_bonus: 2,
                }
            } else {
                CombatMotivationModifier {
                    delta_mot: 0,
                    command_breakdown_morale_shock: 0,
                    conviction_surge_pool_bonus: 0,
                }
            }
        }
        CombatMotivationKind::MercenaryTransactionalUnderFire => {
            if profile.slf >= 14 && profile.blg <= 8 {
                let penalty = if profile.slf >= 16 && profile.blg <= 6 {
                    -4
                } else {
                    -2
                };
                CombatMotivationModifier {
                    delta_mot: penalty,
                    command_breakdown_morale_shock: 0,
                    conviction_surge_pool_bonus: 0,
                }
            } else {
                CombatMotivationModifier {
                    delta_mot: 0,
                    command_breakdown_morale_shock: 0,
                    conviction_surge_pool_bonus: 0,
                }
            }
        }
        CombatMotivationKind::SwornDutyChainOfCommand { command_intact } => {
            if profile.cnf >= 14 {
                if command_intact {
                    CombatMotivationModifier {
                        delta_mot: 2,
                        command_breakdown_morale_shock: 0,
                        conviction_surge_pool_bonus: 0,
                    }
                } else {
                    CombatMotivationModifier {
                        delta_mot: 0,
                        command_breakdown_morale_shock: 4,
                        conviction_surge_pool_bonus: 0,
                    }
                }
            } else {
                CombatMotivationModifier {
                    delta_mot: 0,
                    command_breakdown_morale_shock: 0,
                    conviction_surge_pool_bonus: 0,
                }
            }
        }
        CombatMotivationKind::NeutralOrSurvival => CombatMotivationModifier {
            delta_mot: 0,
            command_breakdown_morale_shock: 0,
            conviction_surge_pool_bonus: 0,
        },
    }
}

/// Canonical Emotional Triggers during combat (`§3.3 Emotional Triggers Table`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmotionalTriggerKind {
    /// **Being Wounded** (`wound_tier` $1..=4$): $\Delta_{\text{ext}} = +2 \times \text{Wound Tier}$
    /// (`+2` if `SLF >= 13`, `-2` if `AGG >= 14`).
    BeingWounded {
        /// Severity tier of the wound sustained (`1` Minor, `2` Moderate, `3` Severe, `4` Critical).
        wound_tier: i32,
    },
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
    /// Whether the character's Psyche profile makes them susceptible to this trigger.
    pub susceptible: bool,
    /// Computed external penalty $\Delta_{\text{ext}}$ on the `DIS` Composure Check.
    pub delta_ext: i32,
    /// Resulting [`MoraleState`] after the trigger.
    pub new_morale_state: MoraleState,
    /// Additional internal strain $\Delta_{\text{int}}$ inflicted (e.g. `+2` Ego Wound on `PubliclyHumiliated`).
    pub delta_int_inflicted: i32,
    /// Whether an Altruistic Conviction Surge or protective resolve is unlocked.
    pub unlocks_protective_conviction: bool,
    /// Roll outcome if a `DIS` Composure check was rolled.
    pub composure_roll: Option<CascadeRollOutcome>,
}

/// Evaluates an [`EmotionalTriggerKind`] against a character's [`PsycheProfile`] and `DIS` Composure roll (`§3.3`).
pub fn evaluate_emotional_trigger(
    profile: &PsycheProfile,
    dis: i32,
    current_morale: MoraleState,
    current_delta_int: i32,
    trigger: EmotionalTriggerKind,
    composure_roll_pairs: &[(i32, i32)],
) -> EmotionalTriggerOutcome {
    match trigger {
        EmotionalTriggerKind::BeingWounded { wound_tier } => {
            let tier = wound_tier.max(1);
            let mut delta_ext = 2 * tier;
            if profile.slf >= 13 {
                delta_ext += 2;
            }
            if profile.agg >= 14 {
                delta_ext = (delta_ext - 2).max(0);
            }
            let task = CascadeTaskParams::new_k20(
                composure_roll_pairs.len().max(1),
                dis.clamp(1, 20),
                current_delta_int,
                delta_ext,
            );
            let roll = task.evaluate_pairs(composure_roll_pairs);
            let new_morale_state = if roll.total_successes > 0 {
                current_morale
            } else if profile.agg >= 15 && profile.slf <= 8 {
                MoraleState::Rage
            } else {
                MoraleState::Wavering
            };
            EmotionalTriggerOutcome {
                susceptible: true,
                delta_ext,
                new_morale_state,
                delta_int_inflicted: 0,
                unlocks_protective_conviction: false,
                composure_roll: Some(roll),
            }
        }
        EmotionalTriggerKind::FriendOrSquadmateWounded => {
            if profile.blg < 13 {
                return EmotionalTriggerOutcome {
                    susceptible: false,
                    delta_ext: 0,
                    new_morale_state: current_morale,
                    delta_int_inflicted: 0,
                    unlocks_protective_conviction: false,
                    composure_roll: None,
                };
            }
            let delta_ext = 2;
            let task = CascadeTaskParams::new_k20(
                composure_roll_pairs.len().max(1),
                dis.clamp(1, 20),
                current_delta_int,
                delta_ext,
            );
            let roll = task.evaluate_pairs(composure_roll_pairs);
            let new_morale_state = if roll.total_successes > 0 {
                current_morale
            } else if profile.agg >= 12 {
                MoraleState::Rage
            } else if profile.agg <= 8 {
                step_morale_condition(current_morale, -1)
            } else {
                current_morale
            };
            EmotionalTriggerOutcome {
                susceptible: true,
                delta_ext,
                new_morale_state,
                delta_int_inflicted: 0,
                unlocks_protective_conviction: false,
                composure_roll: Some(roll),
            }
        }
        EmotionalTriggerKind::LovedOneAttackedOrWounded => {
            let susceptible = profile.blg >= 14 || profile.slf <= 7;
            if !susceptible {
                return EmotionalTriggerOutcome {
                    susceptible: false,
                    delta_ext: 0,
                    new_morale_state: current_morale,
                    delta_int_inflicted: 0,
                    unlocks_protective_conviction: false,
                    composure_roll: None,
                };
            }
            let delta_ext = 4;
            let task = CascadeTaskParams::new_k20(
                composure_roll_pairs.len().max(1),
                dis.clamp(1, 20),
                current_delta_int,
                delta_ext,
            );
            let roll = task.evaluate_pairs(composure_roll_pairs);
            let new_morale_state = if roll.total_successes > 0 {
                MoraleState::High
            } else {
                MoraleState::Rage
            };
            EmotionalTriggerOutcome {
                susceptible: true,
                delta_ext,
                new_morale_state,
                delta_int_inflicted: 0,
                unlocks_protective_conviction: true,
                composure_roll: Some(roll),
            }
        }
        EmotionalTriggerKind::InsultedOrTaunted => {
            let susceptible = profile.ego >= 13 || profile.agg >= 13;
            if !susceptible {
                return EmotionalTriggerOutcome {
                    susceptible: false,
                    delta_ext: 0,
                    new_morale_state: current_morale,
                    delta_int_inflicted: 0,
                    unlocks_protective_conviction: false,
                    composure_roll: None,
                };
            }
            let delta_ext = psyche_deviation(profile.ego).max(psyche_deviation(profile.agg));
            let task = CascadeTaskParams::new_k20(
                composure_roll_pairs.len().max(1),
                dis.clamp(1, 20),
                current_delta_int,
                delta_ext,
            );
            let roll = task.evaluate_pairs(composure_roll_pairs);
            let new_morale_state = if roll.total_successes > 0 {
                current_morale
            } else {
                MoraleState::Rage
            };
            EmotionalTriggerOutcome {
                susceptible: true,
                delta_ext,
                new_morale_state,
                delta_int_inflicted: 0,
                unlocks_protective_conviction: false,
                composure_roll: Some(roll),
            }
        }
        EmotionalTriggerKind::PubliclyHumiliated => {
            if profile.ego < 14 {
                return EmotionalTriggerOutcome {
                    susceptible: false,
                    delta_ext: 0,
                    new_morale_state: current_morale,
                    delta_int_inflicted: 0,
                    unlocks_protective_conviction: false,
                    composure_roll: None,
                };
            }
            let delta_ext = 4;
            let task = CascadeTaskParams::new_k20(
                composure_roll_pairs.len().max(1),
                dis.clamp(1, 20),
                current_delta_int,
                delta_ext,
            );
            let roll = task.evaluate_pairs(composure_roll_pairs);
            if roll.total_successes > 0 {
                EmotionalTriggerOutcome {
                    susceptible: true,
                    delta_ext,
                    new_morale_state: current_morale,
                    delta_int_inflicted: 0,
                    unlocks_protective_conviction: false,
                    composure_roll: Some(roll),
                }
            } else {
                let new_morale_state = if profile.agg >= 12 {
                    MoraleState::Rage
                } else if profile.agg <= 8 {
                    MoraleState::Broken
                } else {
                    MoraleState::Wavering
                };
                EmotionalTriggerOutcome {
                    susceptible: true,
                    delta_ext,
                    new_morale_state,
                    delta_int_inflicted: 2,
                    unlocks_protective_conviction: false,
                    composure_roll: Some(roll),
                }
            }
        }
        EmotionalTriggerKind::SeeingInnocentSuffering => {
            let susceptible = profile.slf <= 7;
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
    /// Fraction of squad/allies wounded or down (`0.0..=1.0`; `>=0.25 -> +2`, `>=0.50 -> +4`, `>=0.75 -> +6`).
    pub ally_casualty_ratio: f64,
    /// Numerical enemy-to-ally ratio (`>=2.0 -> +2`, `>=3.0 -> +4`, `>4.0 -> +6`).
    pub outnumbered_ratio: f64,
    /// Commanding officer incapacitated or comms cut off (`+2`, or `+4` if `CNF >= 14`).
    pub leader_incapacitated_or_comms_cut: bool,
    /// Position overrun, flanked, or ambushed from cover (`+6`).
    pub position_overrun_flanked_or_ambushed: bool,
    /// Remaining ammunition fraction (`0.0..=1.0`; `<= 0.20 -> +4`).
    pub ammo_fraction_remaining: f64,
    /// Whether ammo tracking applies to this combatant (if `false`, `ammo_fraction_remaining` is ignored).
    pub tracks_ammo: bool,
    /// Primary weapon compromised or destroyed (`+4`).
    pub primary_weapon_compromised: bool,
    /// Own attacks doing nothing against heavy armor (`+6`).
    pub attacks_ineffective_against_heavy_armor: bool,
    /// Started engagement already fatigued or sleep-deprived (`+2`).
    pub started_fatigued_or_sleep_deprived: bool,
    /// Additional Morale Shock (e.g., $\lfloor F / 2 \rfloor$ from incoming suppressive fire).
    pub additional_suppression_or_shock_delta: i32,
}

/// Computes the total Circumstantial Morale Penalty $\Delta_{\text{morale}}$ from `§3.4`.
pub fn compute_battlefield_morale_delta(situation: &BattlefieldMoraleSituation, cnf: i32) -> i32 {
    let mut delta = 0;

    if situation.ally_casualty_ratio >= 0.75 {
        delta += 6;
    } else if situation.ally_casualty_ratio >= 0.50 {
        delta += 4;
    } else if situation.ally_casualty_ratio >= 0.25 {
        delta += 2;
    }

    if situation.outnumbered_ratio > 4.0 {
        delta += 6;
    } else if situation.outnumbered_ratio >= 3.0 {
        delta += 4;
    } else if situation.outnumbered_ratio >= 2.0 {
        delta += 2;
    }

    if situation.leader_incapacitated_or_comms_cut {
        delta += if cnf >= 14 { 4 } else { 2 };
    }

    if situation.position_overrun_flanked_or_ambushed {
        delta += 6;
    }

    if (situation.tracks_ammo && situation.ammo_fraction_remaining <= 0.20)
        || situation.primary_weapon_compromised
    {
        delta += 4;
    }

    if situation.attacks_ineffective_against_heavy_armor {
        delta += 6;
    }

    if situation.started_fatigued_or_sleep_deprived {
        delta += 2;
    }

    delta + situation.additional_suppression_or_shock_delta.max(0)
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
    let next_idx = (current_idx - rally_or_drop_steps).clamp(0, 4) as usize;
    LADDER[next_idx]
}

/// Outcome of a Contested Combat Morale Roll (`§3.4`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatMoraleRollOutcome {
    /// Base Morale Controller $C_{\text{morale}} = \text{DIS} + \Delta_{\text{mot}}$.
    pub c_morale: i32,
    /// Player target number $\text{TN}_P = 31 - C_{\text{morale}}$.
    pub player_tn: i32,
    /// GM 1-Pair Tiered Resistance target number $\text{TN}_{GM} = 21 - \Delta_{\text{morale}}$.
    pub gm_tn: i32,
    /// Previous [`MoraleState`] before the check.
    pub previous_morale: MoraleState,
    /// Updated [`MoraleState`] after the check.
    pub new_morale: MoraleState,
    /// Condition step change (`+1` rally, `0` hold, `-1` clean failure, `-2` botch failure).
    pub condition_step_delta: i32,
    /// Full contested cascade roll details.
    pub contested_roll: ContestedCascadeRollOutcome,
}

/// Evaluates a Contested Combat Morale Roll (`§3.4 Rolling for Morale`):
/// - $C_{\text{morale}} = \text{DIS} + \Delta_{\text{mot}}$
/// - $N_P = 2$ if veteran/SERE/Command trained, else $N_P = 1$
/// - $\text{TN}_P = 31 - C_{\text{morale}}$ vs. GM 1-Pair Tiered Resistance $\text{TN}_{GM} = 21 - \Delta_{\text{morale}}$
/// - **Extraordinary Success** ($\Delta M \ge +2$): Rallies `+1` step toward `High`.
/// - **Clean / Partial Success**: Holds current Morale State (`0` steps).
/// - **Clean Failure**: Drops `1` Condition Step (`-1`).
/// - **Botch Failure**: Drops `2` Condition Steps (`-2`).
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
    let contest = ContestedCascadeTaskParams::new_tiered_environment(
        n_p,
        c_morale,
        delta_int,
        0,
        delta_morale,
        ContestedTieRule::HybridSignedMargin,
    );
    let roll = contest.evaluate_contested_pairs(player_pairs, &[gm_pair]);

    let step_delta = match roll.contested_kind {
        ContestedOutcomeKind::ExtraordinarySuccess if roll.net_signed_margin >= 2 => 1,
        ContestedOutcomeKind::ExtraordinarySuccess
        | ContestedOutcomeKind::CleanSuccess
        | ContestedOutcomeKind::GrazingTieSuccess
        | ContestedOutcomeKind::LuckyBreakSuccess
        | ContestedOutcomeKind::SuccessWithFloorBreach => 0,
        ContestedOutcomeKind::CleanFailure => -1,
        ContestedOutcomeKind::BotchFailure => -2,
    };

    let new_morale = step_morale_condition(current_morale, step_delta);

    CombatMoraleRollOutcome {
        c_morale,
        player_tn: contest.player.tn_eff(),
        gm_tn: contest.gm.tn_eff(),
        previous_morale: current_morale,
        new_morale,
        condition_step_delta: step_delta,
        contested_roll: roll,
    }
}

// ============================================================================
// §4 Social Combat, Debate & Cognitive Dissonance ("Mental Wounds")
// ============================================================================

/// Result of an **Empathy & Read Person** (`INS/ITU`) or **Psychology** (`INS/ANA`) setup roll (`§4.1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmpathyReadResult {
    /// Net successes achieved on the Read Person check.
    pub net_successes: i32,
    /// Revealed Psyche traits and their exact scores.
    pub revealed_traits: Vec<(PsycheTraitId, i32)>,
    /// Whether the reader detects an active or potential Dual-Psyche Wedge tension (two traits $\ge 13$).
    pub detects_psyche_wedge_tension: bool,
    /// Leverage modifier $\Delta_{\text{ext}}$ granted on the subsequent social maneuver (`-2` at `3+` successes, else `0`).
    pub leverage_delta_ext_bonus: i32,
}

/// Evaluates a `§4.1 Reading the Room` (`INS/ITU` or `INS/ANA`) check against a target's [`PsycheProfile`]:
/// - **1 Net Success**: Reveals the target's single most extreme Psyche trait (furthest from `10`).
/// - **2 Net Successes**: Reveals the top two most extreme traits and whether they are in potential tension.
/// - **3+ Net Successes**: Reveals the full 6-trait [`PsycheProfile`] and grants `-2` $\Delta_{\text{ext}}$ on the next maneuver.
pub fn evaluate_empathy_read(
    target_profile: &PsycheProfile,
    net_successes: i32,
) -> EmpathyReadResult {
    let sorted = target_profile.traits_by_extremity();
    let revealed_count = match net_successes {
        n if n <= 0 => 0,
        1 => 1,
        2 => 2,
        _ => 6,
    };
    let revealed_traits: Vec<(PsycheTraitId, i32)> = sorted
        .iter()
        .take(revealed_count)
        .map(|&(id, score, _)| (id, score))
        .collect();

    let detects_psyche_wedge_tension = net_successes >= 2
        && sorted.len() >= 2
        && ((sorted[0].1 >= 13 && sorted[1].1 >= 13) || (sorted[0].2 >= 3 && sorted[1].2 >= 3));

    let leverage_delta_ext_bonus = if net_successes >= 3 { -2 } else { 0 };

    EmpathyReadResult {
        net_successes,
        revealed_traits,
        detects_psyche_wedge_tension,
        leverage_delta_ext_bonus,
    }
}

/// The 7 canonical Social Argument Vectors from `§4.2 Argument Vectors & Psyche Resonance vs. Resistance`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialArgumentVector {
    /// **1. Empirical Evidence, Logic, Novel Ideas** (*Persuasion & Rhetoric* `ELO/ANA`):
    /// Resonates with High `OPN >= 13` ($-(\text{OPN}-10)$); clashes with Low `OPN <= 7` ($+(10-\text{OPN})$).
    EmpiricalEvidenceLogicNovelIdeas,
    /// **2. Group Consensus, Loyalty, Shared Identity** (*Persuasion* `ELO/INS` / *Charm* `PRE/ELO`):
    /// Resonates with High `BLG >= 13` ($-(\text{BLG}-10)$); clashes with Low `BLG <= 7` ($+(10-\text{BLG})$)
    /// or when asking a High `BLG >= 13` target to betray their group ($+(\text{BLG}-10)$).
    GroupConsensusLoyaltySharedIdentity {
        /// True if the argument asks a High-`BLG` target to betray or indict their own group.
        asks_to_betray_group: bool,
    },
    /// **3. Flattery, Prestige, Legacy, Status** (*Charm* `PRE/ELO` / *Deception* `ELO/CRE`):
    /// Resonates with High `EGO >= 13` ($-(\text{EGO}-10)$); clashes when humiliating or demanding fault admission from `EGO >= 13` ($+(\text{EGO}-10)$).
    FlatteryPrestigeLegacyStatus {
        /// True if the argument publicly humiliates or demands admission of fault from the target.
        humiliates_or_demands_fault_admission: bool,
    },
    /// **4. Personal Profit, Immunity, Self-Interest** (*Negotiation* `ELO/INS` / *Bribery*):
    /// Resonates with High `SLF >= 13` ($-(\text{SLF}-10)$); clashes with Low `SLF <= 7` ($+(10-\text{SLF})$).
    PersonalProfitImmunitySelfInterest,
    /// **5. Self-Sacrifice, Humanitarian Duty, Mercy** (*Persuasion* `ELO/INS`):
    /// Resonates with Low `SLF <= 7` ($-(10-\text{SLF})$); clashes with High `SLF >= 13` ($+(\text{SLF}-10)$).
    SelfSacrificeHumanitarianDutyMercy,
    /// **6. Rules, Law, Protocol, Chain of Command** (*Diplomacy* / *Command* `PRE/DIS`):
    /// Resonates with High `CNF >= 13` ($-(\text{CNF}-10)$); clashes with Low `CNF <= 7` ($+(10-\text{CNF})$)
    /// or when asking a High `CNF >= 13` target to break the law ($+(\text{CNF}-10)$).
    RulesLawProtocolChainOfCommand {
        /// True if the argument asks a High-`CNF` target to violate protocol or break the law.
        asks_to_break_law_or_orders: bool,
    },
    /// **7. Dominance, Threat, Dare, Confrontation** (*Intimidation* `PRE/DIS`):
    /// Resonates with Low `AGG <= 7` ($-(10-\text{AGG})$ to yield) or High `AGG >= 13` ($-(\text{AGG}-10)$ to bait into fight);
    /// clashes with High `AGG >= 13` when forcing submission ($+(\text{AGG}-10)$).
    DominanceThreatDareConfrontation {
        /// True if the speaker is trying to bait a High-`AGG` target into a rash fight rather than force submission.
        aims_to_bait_into_fight: bool,
    },
}

/// Evaluation of how a single or compound [`SocialArgumentVector`] interacts with a target's [`PsycheProfile`] (`§4.2`, `§4.3.3`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgumentPsycheEvaluation {
    /// Total Resonance bonus ($\le 0$, reduces $\Delta_{\text{ext}}$).
    pub resonance_delta_ext: i32,
    /// Total Clash resistance penalty ($\ge 0$, increases $\Delta_{\text{ext}}$).
    pub clash_delta_ext: i32,
    /// Net external modifier $\Delta_{\text{ext}} = \text{resonance\_delta\_ext} + \text{clash\_delta\_ext}$.
    pub net_delta_ext: i32,
    /// High Psyche trait ($P_1 \ge 13$ or Low pole $\le 7$) that resonated with the argument.
    pub resonated_trait: Option<PsycheTraitId>,
    /// High Psyche trait ($P_2 \ge 13$ or Low pole $\le 7$) that clashed with the argument.
    pub clashed_trait: Option<PsycheTraitId>,
    /// True when the argument simultaneously resonates with one high Psyche trait ($P_1 \ge 13$)
    /// and clashes with another high Psyche trait ($P_2 \ge 13$) — the **Dual-Psyche Wedge** (`§4.3.3`).
    pub triggers_dual_psyche_wedge: bool,
}

/// Computes the $\Delta_{\text{ext}}$ contribution of a single [`SocialArgumentVector`] (`§4.2`).
pub fn compute_single_vector_psyche_modifier(
    profile: &PsycheProfile,
    vector: SocialArgumentVector,
) -> (i32, Option<PsycheTraitId>, Option<PsycheTraitId>, bool) {
    match vector {
        SocialArgumentVector::EmpiricalEvidenceLogicNovelIdeas => {
            if profile.opn >= 13 {
                (
                    -(profile.opn - 10),
                    Some(PsycheTraitId::Openness),
                    None,
                    true,
                )
            } else if profile.opn <= 7 {
                (10 - profile.opn, None, Some(PsycheTraitId::Openness), false)
            } else {
                (0, None, None, false)
            }
        }
        SocialArgumentVector::GroupConsensusLoyaltySharedIdentity {
            asks_to_betray_group,
        } => {
            if asks_to_betray_group && profile.blg >= 13 {
                (profile.blg - 10, None, Some(PsycheTraitId::Belonging), true)
            } else if !asks_to_betray_group && profile.blg >= 13 {
                (
                    -(profile.blg - 10),
                    Some(PsycheTraitId::Belonging),
                    None,
                    true,
                )
            } else if !asks_to_betray_group && profile.blg <= 7 {
                (
                    10 - profile.blg,
                    None,
                    Some(PsycheTraitId::Belonging),
                    false,
                )
            } else {
                (0, None, None, false)
            }
        }
        SocialArgumentVector::FlatteryPrestigeLegacyStatus {
            humiliates_or_demands_fault_admission,
        } => {
            if profile.ego >= 13 {
                if humiliates_or_demands_fault_admission {
                    (profile.ego - 10, None, Some(PsycheTraitId::Ego), true)
                } else {
                    (-(profile.ego - 10), Some(PsycheTraitId::Ego), None, true)
                }
            } else {
                (0, None, None, false)
            }
        }
        SocialArgumentVector::PersonalProfitImmunitySelfInterest => {
            if profile.slf >= 13 {
                (
                    -(profile.slf - 10),
                    Some(PsycheTraitId::Selfishness),
                    None,
                    true,
                )
            } else if profile.slf <= 7 {
                (
                    10 - profile.slf,
                    None,
                    Some(PsycheTraitId::Selfishness),
                    false,
                )
            } else {
                (0, None, None, false)
            }
        }
        SocialArgumentVector::SelfSacrificeHumanitarianDutyMercy => {
            if profile.slf <= 7 {
                (
                    -(10 - profile.slf),
                    Some(PsycheTraitId::Selfishness),
                    None,
                    false,
                )
            } else if profile.slf >= 13 {
                (
                    profile.slf - 10,
                    None,
                    Some(PsycheTraitId::Selfishness),
                    true,
                )
            } else {
                (0, None, None, false)
            }
        }
        SocialArgumentVector::RulesLawProtocolChainOfCommand {
            asks_to_break_law_or_orders,
        } => {
            if asks_to_break_law_or_orders && profile.cnf >= 13 {
                (
                    profile.cnf - 10,
                    None,
                    Some(PsycheTraitId::Conformity),
                    true,
                )
            } else if !asks_to_break_law_or_orders && profile.cnf >= 13 {
                (
                    -(profile.cnf - 10),
                    Some(PsycheTraitId::Conformity),
                    None,
                    true,
                )
            } else if !asks_to_break_law_or_orders && profile.cnf <= 7 {
                (
                    10 - profile.cnf,
                    None,
                    Some(PsycheTraitId::Conformity),
                    false,
                )
            } else {
                (0, None, None, false)
            }
        }
        SocialArgumentVector::DominanceThreatDareConfrontation {
            aims_to_bait_into_fight,
        } => {
            if profile.agg <= 7 && !aims_to_bait_into_fight {
                (
                    -(10 - profile.agg),
                    Some(PsycheTraitId::Aggression),
                    None,
                    false,
                )
            } else if profile.agg >= 13 && aims_to_bait_into_fight {
                (
                    -(profile.agg - 10),
                    Some(PsycheTraitId::Aggression),
                    None,
                    true,
                )
            } else if profile.agg >= 13 && !aims_to_bait_into_fight {
                (
                    profile.agg - 10,
                    None,
                    Some(PsycheTraitId::Aggression),
                    true,
                )
            } else {
                (0, None, None, false)
            }
        }
    }
}

/// Evaluates one or more [`SocialArgumentVector`] components against a target's [`PsycheProfile`],
/// computing Resonance, Clash, Net $\Delta_{\text{ext}}$, and whether a **Dual-Psyche Wedge** (`§4.3.3`) is formed.
pub fn compute_argument_psyche_modifier(
    profile: &PsycheProfile,
    vectors: &[SocialArgumentVector],
) -> ArgumentPsycheEvaluation {
    let mut resonance_delta_ext = 0;
    let mut clash_delta_ext = 0;
    let mut resonated_trait = None;
    let mut clashed_trait = None;
    let mut high_pole_resonated = false;
    let mut high_pole_clashed = false;

    for &vec in vectors {
        let (delta, res_t, cls_t, is_high_pole) =
            compute_single_vector_psyche_modifier(profile, vec);
        if delta < 0 {
            resonance_delta_ext += delta;
            if resonated_trait.is_none() {
                resonated_trait = res_t;
            }
            if is_high_pole {
                high_pole_resonated = true;
            }
        } else if delta > 0 {
            clash_delta_ext += delta;
            if clashed_trait.is_none() {
                clashed_trait = cls_t;
            }
            if is_high_pole {
                high_pole_clashed = true;
            }
        }
    }

    let triggers_dual_psyche_wedge = high_pole_resonated
        && high_pole_clashed
        && resonated_trait.is_some()
        && clashed_trait.is_some()
        && resonated_trait != clashed_trait;

    ArgumentPsycheEvaluation {
        resonance_delta_ext,
        clash_delta_ext,
        net_delta_ext: resonance_delta_ext + clash_delta_ext,
        resonated_trait,
        clashed_trait,
        triggers_dual_psyche_wedge,
    }
}

/// Computes the **Social Composure Threshold (`CMP`)** from `§4.3`:
/// $$\text{CMP} = \left\lfloor \frac{\text{DIS} + \text{FOC}}{4} \right\rfloor$$
pub fn compute_social_composure_threshold(dis: i32, foc: i32) -> i32 {
    (dis.clamp(1, 20) + foc.clamp(1, 20)).div_euclid(4)
}

/// The 4 canonical Social Wounds from `§4.3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialWoundKind {
    /// **1. Rhetorical Strain (Mental Fatigue)**: `-1` `CMP`, `+2` $\Delta_{\text{int}}$.
    RhetoricalStrain,
    /// **2. Doubt (Conviction Fracture)**: `+1` Doubt fracture (`-1` effective skill pair $N$ when defending that point; `3` fractures demolishes argument).
    Doubt,
    /// **3. Cognitive Dissonance (Psychological Bleeding)**: Stacked by Dual-Psyche Wedge net successes; bleeds $+2 \times \text{Dissonance Index}$ to $\Delta_{\text{int}}$ every turn/exchange.
    CognitiveDissonance,
    /// **4. Ego Wound (Provocation / Loss of Cool)**: Forces a `DIS` check against $\text{TN} = 31 - \text{DIS} + (\text{EGO} - 10)$ or loses `-3` `FOC` and lashes out.
    EgoWound,
}

/// Tracked psychological composure, Doubt fractures, and Cognitive Dissonance state of a character in Social Combat (`§4.3–§4.4`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialComposureState {
    /// Maximum Social Composure Threshold $\text{CMP} = \lfloor(\text{DIS} + \text{FOC})/4\rfloor$.
    pub max_cmp: i32,
    /// Remaining Social Composure before being rhetorically cornered.
    pub current_cmp: i32,
    /// Accumulated **Doubt** fractures (`0..=3`; at `3`, the defended position is intellectually demolished).
    pub doubt_fractures: i32,
    /// Active **Cognitive Dissonance Index** stacks (each stack bleeds `+2` $\Delta_{\text{int}}$ per turn/exchange).
    pub dissonance_index: i32,
    /// Accumulated internal strain $\Delta_{\text{int}}$ from Rhetorical Strain, Dissonance bleed, or Ego Wounds.
    pub accumulated_delta_int: i32,
    /// Penalty to `FOC` from unresisted Ego Wounds (`-3` per failed Ego Wound check).
    pub focus_penalty: i32,
    /// Whether the character's rhetorical position has been demolished (`doubt_fractures >= 3`).
    pub position_demolished: bool,
    /// Whether an unresisted Ego Wound caused the character to lose their cool and lash out.
    pub lashed_out_from_ego_wound: bool,
}

impl SocialComposureState {
    /// Initializes a fresh [`SocialComposureState`] from `DIS` and `FOC`.
    pub fn new(dis: i32, foc: i32) -> Self {
        let cmp = compute_social_composure_threshold(dis, foc);
        Self {
            max_cmp: cmp,
            current_cmp: cmp,
            doubt_fractures: 0,
            dissonance_index: 0,
            accumulated_delta_int: 0,
            focus_penalty: 0,
            position_demolished: false,
            lashed_out_from_ego_wound: false,
        }
    }

    /// Applies the per-turn/per-exchange **Psychological Bleeding** (`§4.3.3`) from active `dissonance_index`:
    /// adds $+2 \times \text{dissonance\_index}$ to `accumulated_delta_int` and returns the bleed amount.
    pub fn apply_turn_dissonance_bleed(&mut self) -> i32 {
        let bleed = 2 * self.dissonance_index.max(0);
        self.accumulated_delta_int += bleed;
        bleed
    }
}

/// Outcome of a single Contested Social Exchange (`§4.2–§4.4`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocialExchangeOutcome {
    /// Argument Resonance vs. Clash evaluation against the target's [`PsycheProfile`].
    pub argument_eval: ArgumentPsycheEvaluation,
    /// Net successes scored by the speaker ($\max(0, S_{\text{speaker}} - S_{\text{target}})$).
    pub net_successes: i32,
    /// Social wounds inflicted on this exchange.
    pub wounds_inflicted: Vec<SocialWoundKind>,
    /// Dissonance Index stacks added on this exchange.
    pub dissonance_stacks_added: i32,
    /// Immediate $\Delta_{\text{int}}$ bleed at the start of the target's next turn ($2 \times \text{dissonance\_index}$).
    pub next_turn_dissonance_bleed_delta_int: i32,
    /// Updated [`SocialComposureState`] of the target.
    pub updated_state: SocialComposureState,
}

/// Evaluates the Social Wounds inflicted by a contested social exchange with `net_successes` (`§4.3`):
/// - `+1` Net Success: Inflicts **Rhetorical Strain** (`-1` `CMP`, `+2` $\Delta_{\text{int}}$).
/// - `+2+` Net Successes with a logical/evidentiary/empathetic vector: Inflicts `1` **Doubt** fracture (`-1` `CMP`, `+1` `doubt_fractures`).
/// - **Dual-Psyche Wedge** (`argument_eval.triggers_dual_psyche_wedge && net_successes >= 1`):
///   Inflicts **Cognitive Dissonance** (`dissonance_index += net_successes`).
/// - `+2+` Net Successes with a humiliating/dominance vector against `EGO >= 13 || AGG >= 13`:
///   Inflicts an **Ego Wound** (if `ego_wound_dis_check_passed == Some(false)`).
pub fn evaluate_social_exchange_wounds(
    target_profile: &PsycheProfile,
    state: &SocialComposureState,
    vectors: &[SocialArgumentVector],
    net_successes: i32,
    ego_wound_dis_check_passed: Option<bool>,
) -> SocialExchangeOutcome {
    let argument_eval = compute_argument_psyche_modifier(target_profile, vectors);
    let mut updated = state.clone();
    let mut wounds_inflicted = Vec::new();
    let mut dissonance_stacks_added = 0;

    if net_successes <= 0 {
        let next_bleed = 2 * updated.dissonance_index;
        return SocialExchangeOutcome {
            argument_eval,
            net_successes,
            wounds_inflicted,
            dissonance_stacks_added,
            next_turn_dissonance_bleed_delta_int: next_bleed,
            updated_state: updated,
        };
    }

    if net_successes == 1 {
        wounds_inflicted.push(SocialWoundKind::RhetoricalStrain);
        updated.current_cmp = (updated.current_cmp - 1).max(0);
        updated.accumulated_delta_int += 2;
    }

    let has_logical_or_empathetic_vector = vectors.iter().any(|v| {
        matches!(
            v,
            SocialArgumentVector::EmpiricalEvidenceLogicNovelIdeas
                | SocialArgumentVector::SelfSacrificeHumanitarianDutyMercy
                | SocialArgumentVector::RulesLawProtocolChainOfCommand { .. }
                | SocialArgumentVector::GroupConsensusLoyaltySharedIdentity { .. }
        )
    });

    if net_successes >= 2 && has_logical_or_empathetic_vector {
        wounds_inflicted.push(SocialWoundKind::Doubt);
        updated.current_cmp = (updated.current_cmp - 1).max(0);
        updated.doubt_fractures += 1;
        if updated.doubt_fractures >= 3 {
            updated.position_demolished = true;
        }
    } else if net_successes >= 2 {
        wounds_inflicted.push(SocialWoundKind::RhetoricalStrain);
        updated.current_cmp = (updated.current_cmp - 1).max(0);
        updated.accumulated_delta_int += 2;
    }

    if argument_eval.triggers_dual_psyche_wedge {
        wounds_inflicted.push(SocialWoundKind::CognitiveDissonance);
        dissonance_stacks_added = net_successes;
        updated.dissonance_index += net_successes;
    }

    let has_provocation_vector = vectors.iter().any(|v| {
        matches!(
            v,
            SocialArgumentVector::FlatteryPrestigeLegacyStatus {
                humiliates_or_demands_fault_admission: true
            } | SocialArgumentVector::DominanceThreatDareConfrontation { .. }
        )
    });

    if net_successes >= 2
        && has_provocation_vector
        && (target_profile.ego >= 13 || target_profile.agg >= 13)
    {
        wounds_inflicted.push(SocialWoundKind::EgoWound);
        if ego_wound_dis_check_passed == Some(false) {
            updated.focus_penalty += 3;
            updated.lashed_out_from_ego_wound = true;
        }
    }

    let next_turn_dissonance_bleed_delta_int = 2 * updated.dissonance_index;

    SocialExchangeOutcome {
        argument_eval,
        net_successes,
        wounds_inflicted,
        dissonance_stacks_added,
        next_turn_dissonance_bleed_delta_int,
        updated_state: updated,
    }
}

/// The 4 canonical ways to resolve active **Cognitive Dissonance** (`§4.4 Resolving Cognitive Dissonance`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DissonanceResolutionPath {
    /// **1. Conversion / Defection (Yielding to the New Truth)**:
    /// Clears `dissonance_index` to `0` and records `1` **Psyche Drift Mark** on the violated trait.
    ConversionDefection {
        /// The Psyche trait that was violated and receives `1` Drift Mark.
        violated_trait: PsycheTraitId,
    },
    /// **2. Dogmatic Denial (Doubling Down)**:
    /// Requires a `DIS` check against $\text{TN} = 31 - \text{DIS} + 2 \times \text{Dissonance Index}$.
    /// On success, clears `dissonance_index` to `0`, penalizes `OPN` by `-4` for the scene, and shifts Morale to `Rage` or `High`.
    DogmaticDenial {
        /// Successes scored on the `DIS` check.
        dis_check_successes: i32,
    },
    /// **3. Compartmentalization (Time & Rationalization)**:
    /// Spends `12 ticks` (or `1` quiet scene); reduces `dissonance_index` by `1` per `DIS` success,
    /// leaving a residual `+2` $\Delta_{\text{int}}$ until a full `8-hour` rest.
    Compartmentalization {
        /// Successes scored on the `DIS` rationalization roll.
        dis_check_successes: i32,
    },
    /// **4. Emotional Lashing Out (Fight or Flight)**:
    /// Converts all `dissonance_index` stacks into immediate `Rage` (`+2` Tactical Pool, `Control` capped at $\lfloor P_{\text{eff}}/4\rfloor$).
    EmotionalLashingOut,
}

/// Result of resolving **Cognitive Dissonance** via one of the 4 canonical paths (`§4.4`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DissonanceResolutionOutcome {
    /// Updated [`SocialComposureState`] after resolution.
    pub updated_state: SocialComposureState,
    /// Target number required if `DogmaticDenial` was chosen ($\text{TN} = 31 - \text{DIS} + 2 \times \text{Dissonance Index}$).
    pub dogmatic_denial_tn: Option<i32>,
    /// Psyche Drift Mark awarded (if `ConversionDefection` was chosen).
    pub psyche_drift_mark_awarded: Option<PsycheTraitId>,
    /// Scene penalty to `OPN` (`-4` on successful `DogmaticDenial`, else `0`).
    pub openness_scene_penalty: i32,
    /// Residual $\Delta_{\text{int}}$ until an `8-hour` rest (`+2` on `Compartmentalization`, else `0`).
    pub residual_delta_int_until_rest: i32,
    /// Ticks spent rationalizing (`12` on `Compartmentalization`, else `0`).
    pub ticks_spent: i32,
    /// Resulting combat [`MoraleState`] shift (e.g., `Rage` on `EmotionalLashingOut` or `DogmaticDenial`).
    pub resulting_morale_state: Option<MoraleState>,
}

/// Resolves active **Cognitive Dissonance** using one of the 4 canonical paths (`§4.4`).
pub fn resolve_cognitive_dissonance(
    state: &SocialComposureState,
    dis: i32,
    path: DissonanceResolutionPath,
) -> DissonanceResolutionOutcome {
    let mut updated = state.clone();
    match path {
        DissonanceResolutionPath::ConversionDefection { violated_trait } => {
            updated.dissonance_index = 0;
            DissonanceResolutionOutcome {
                updated_state: updated,
                dogmatic_denial_tn: None,
                psyche_drift_mark_awarded: Some(violated_trait),
                openness_scene_penalty: 0,
                residual_delta_int_until_rest: 0,
                ticks_spent: 0,
                resulting_morale_state: Some(MoraleState::Steady),
            }
        }
        DissonanceResolutionPath::DogmaticDenial {
            dis_check_successes,
        } => {
            let tn = 31 - dis.clamp(1, 20) + (2 * updated.dissonance_index.max(0));
            if dis_check_successes > 0 {
                updated.dissonance_index = 0;
                DissonanceResolutionOutcome {
                    updated_state: updated,
                    dogmatic_denial_tn: Some(tn),
                    psyche_drift_mark_awarded: None,
                    openness_scene_penalty: -4,
                    residual_delta_int_until_rest: 0,
                    ticks_spent: 0,
                    resulting_morale_state: Some(MoraleState::Rage),
                }
            } else {
                DissonanceResolutionOutcome {
                    updated_state: updated,
                    dogmatic_denial_tn: Some(tn),
                    psyche_drift_mark_awarded: None,
                    openness_scene_penalty: 0,
                    residual_delta_int_until_rest: 0,
                    ticks_spent: 0,
                    resulting_morale_state: None,
                }
            }
        }
        DissonanceResolutionPath::Compartmentalization {
            dis_check_successes,
        } => {
            updated.dissonance_index =
                (updated.dissonance_index - dis_check_successes.max(0)).max(0);
            updated.accumulated_delta_int = 2;
            DissonanceResolutionOutcome {
                updated_state: updated,
                dogmatic_denial_tn: None,
                psyche_drift_mark_awarded: None,
                openness_scene_penalty: 0,
                residual_delta_int_until_rest: 2,
                ticks_spent: 12,
                resulting_morale_state: None,
            }
        }
        DissonanceResolutionPath::EmotionalLashingOut => {
            updated.dissonance_index = 0;
            DissonanceResolutionOutcome {
                updated_state: updated,
                dogmatic_denial_tn: None,
                psyche_drift_mark_awarded: None,
                openness_scene_penalty: 0,
                residual_delta_int_until_rest: 0,
                ticks_spent: 0,
                resulting_morale_state: Some(MoraleState::Rage),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psyche_friction_check_and_drift_threshold() {
        // High BLG (16) character ordered to abandon wounded squadmate (violates HighPole)
        let params = PsycheFrictionCheckParams {
            trait_id: PsycheTraitId::Belonging,
            trait_score: 16,
            violated_pole: PsycheViolationPole::HighPole,
            dis: 12,
            has_sere_training: false,
            delta_int: 0,
            delta_ext: 0,
        };
        assert_eq!(psyche_deviation(16), 6);
        assert!(triggers_psyche_friction_check(
            16,
            PsycheViolationPole::HighPole
        ));

        let contest = build_psyche_friction_contest(&params);
        assert_eq!(contest.player.tn_eff(), 19); // 31 - 12 = 19
        assert_eq!(contest.gm.tn_eff(), 15); // 21 - 6 = 15

        // In Satya roll-over cascade, X > TN scores a success:
        // Player rolls (12, 10) = 22 > 19 (1 success); GM rolls (7, 7) = 14 <= 15 (0 successes) -> CleanOverride
        let clean = evaluate_psyche_friction_roll(&params, &[(12, 10)], (7, 7));
        assert!(clean.triggered);
        assert_eq!(
            clean.resolution_kind,
            PsycheFrictionResolutionKind::CleanOverride
        );
        assert_eq!(clean.delta_int_penalty_gained, 0);
        assert_eq!(clean.drift_marks_gained, 1);

        // Player rolls (12, 10) = 22 > 19 (1 success); GM rolls (9, 8) = 17 > 15 (1 success) -> GrazingTie / StrainedOverride (+2 Delta_int)
        let strained = evaluate_psyche_friction_roll(&params, &[(12, 10)], (9, 8));
        assert_eq!(
            strained.resolution_kind,
            PsycheFrictionResolutionKind::StrainedOverride
        );
        assert_eq!(strained.delta_int_penalty_gained, 2);
        assert_eq!(strained.drift_marks_gained, 1);

        // Player rolls (9, 8) = 17 <= 19 (0 successes); GM rolls (9, 8) = 17 > 15 (1 success) -> BalkOrFreeze (6 ticks)
        let balk = evaluate_psyche_friction_roll(&params, &[(9, 8)], (9, 8));
        assert_eq!(
            balk.resolution_kind,
            PsycheFrictionResolutionKind::BalkOrFreeze
        );
        assert_eq!(balk.hesitation_ticks, 6);
        assert_eq!(balk.drift_marks_gained, 0);

        // Psyche Drift threshold for DIS 12 is max(2, 12/4) = 3 marks
        assert_eq!(psyche_drift_threshold(12), 3);
        let d1 = apply_psyche_drift_mark(16, 2, 1, 12, None);
        assert!(d1.shifted);
        assert_eq!(d1.new_score, 15);
        assert_eq!(d1.remaining_drift_marks, 0);
    }

    #[test]
    fn test_conviction_surge_mechanics() {
        let profile = PsycheProfile {
            blg: 16,
            slf: 5,
            ..Default::default()
        };
        let surge = evaluate_conviction_surge(
            &profile,
            4,
            false,
            Some(ConvictionPrincipleArchetype::AltruisticOrTribalSacrifice),
        );
        assert!(surge.eligible);
        assert_eq!(surge.adjusted_delta_int, -2);
        assert_eq!(surge.surge_delta_under, 4);
        assert!(surge.ignores_fatigue_and_shock_time_penalty);
        assert_eq!(surge.tactical_pool_bonus, 2);
    }

    #[test]
    fn test_marcus_vs_nadia_cognitive_dissonance_walkthrough() {
        // Canonical §4.4 Worked Example: Marcus vs. Dr. Nadia Vance
        // Nadia: OPN 16, BLG 16, DIS 11, FOC 11 -> CMP = floor((11 + 11) / 4) = 5
        let nadia = PsycheProfile {
            opn: 16,
            blg: 16,
            ..Default::default()
        };
        let state = SocialComposureState::new(11, 11);
        assert_eq!(state.max_cmp, 5);

        // Step 1: Marcus's Empathy read scores 2 net successes -> reveals OPN 16 & BLG 16 + detects wedge tension
        let read = evaluate_empathy_read(&nadia, 2);
        assert_eq!(read.revealed_traits.len(), 2);
        assert!(read.detects_psyche_wedge_tension);

        // Step 2: Marcus presents raw telemetry logs proving Nadia's lab director falsified safety data
        // - Resonates with OPN 16: -(16 - 10) = -6
        // - Clashes with BLG 16 (asking her to indict her own team): +(16 - 10) = +6
        // - Net Delta_ext = 0, triggers Dual-Psyche Wedge!
        let vectors = [
            SocialArgumentVector::EmpiricalEvidenceLogicNovelIdeas,
            SocialArgumentVector::GroupConsensusLoyaltySharedIdentity {
                asks_to_betray_group: true,
            },
        ];
        // Step 3: Marcus scores 3 successes vs. Nadia's 1 success -> 2 Net Successes
        let exchange = evaluate_social_exchange_wounds(&nadia, &state, &vectors, 2, None);
        assert_eq!(exchange.argument_eval.resonance_delta_ext, -6);
        assert_eq!(exchange.argument_eval.clash_delta_ext, 6);
        assert_eq!(exchange.argument_eval.net_delta_ext, 0);
        assert!(exchange.argument_eval.triggers_dual_psyche_wedge);
        assert!(exchange.wounds_inflicted.contains(&SocialWoundKind::Doubt));
        assert!(
            exchange
                .wounds_inflicted
                .contains(&SocialWoundKind::CognitiveDissonance)
        );
        assert_eq!(exchange.updated_state.doubt_fractures, 1);
        assert_eq!(exchange.updated_state.dissonance_index, 2);
        assert_eq!(exchange.next_turn_dissonance_bleed_delta_int, 4);

        // Step 4: Next exchange, Nadia bleeds +4 Delta_int from her Dissonance Index of 2.
        // If she attempts Dogmatic Denial, her TN is 31 - 11 + 2*2 = 24.
        let denial = resolve_cognitive_dissonance(
            &exchange.updated_state,
            11,
            DissonanceResolutionPath::DogmaticDenial {
                dis_check_successes: 0,
            },
        );
        assert_eq!(denial.dogmatic_denial_tn, Some(24));

        // She chooses Conversion / Defection: clears Dissonance Index to 0 and records 1 Drift Mark on BLG.
        let conversion = resolve_cognitive_dissonance(
            &exchange.updated_state,
            11,
            DissonanceResolutionPath::ConversionDefection {
                violated_trait: PsycheTraitId::Belonging,
            },
        );
        assert_eq!(conversion.updated_state.dissonance_index, 0);
        assert_eq!(
            conversion.psyche_drift_mark_awarded,
            Some(PsycheTraitId::Belonging)
        );
    }
}
