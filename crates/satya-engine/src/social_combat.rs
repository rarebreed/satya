//! # Social Combat, Debate & Cognitive Dissonance ("Mental Wounds") (`04-psyche-and-social §4)
//!
//! Canonical Rust implementation of docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md §4.1–§4.7:
//! - **§4.1 Reading the Target**: Empathy & Read Person (INS/ITU) / Investigative Elicitation (INS/ELO) setup
//! - **§4.2–§4.3 The 7 Argument Vectors & Social Stances**: Resonance (-2/-4/-6 $\Delta_{\text{ext}}$) vs. Clash (+2/+4/+6 $\Delta_{\text{ext}}$)
//! - **§4.4–§4.5 Social Composure (CMP) & The 4 Social Wounds**: RhetoricalStrain, Doubt, CognitiveDissonance, EgoWound, and the Dual-Psyche Wedge
//! - **§4.6–§4.7 Interrogation & Dissonance Resolution Paths**: Compartmentalize, DogmaticDenial, AtonementAction, ConversionDefection

use serde::{Deserialize, Serialize};

use crate::characteristics::compute_social_composure_score;
use crate::combat::MoraleState;
use crate::psyche_social::{HasPsyche, PsycheTraitId};
// ============================================================================
// §4 Social Combat, Debate & Cognitive Dissonance ("Mental Wounds")
// ============================================================================

/// Result of an **Empathy & Read Person** (`INS/ITU`) or **Investigative Elicitation** (`INS/ELO`) setup roll
/// (`docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §4.1).
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

/// Evaluates a `§4.1 Reading the Target` (*Empathy & Read Person* `INS/ITU` or *Investigative Elicitation* `INS/ELO`)
/// check against a target's [`HasPsyche`] profile (`docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §4.1):
/// - **1 Net Success**: Reveals the target's single most extreme Psyche trait (furthest from `10`).
/// - **2 Net Successes**: Reveals the top two most extreme traits and whether they are in potential tension.
/// - **3+ Net Successes**: Reveals the full 7-trait [`CharacterCharacteristics`](crate::characteristics::CharacterCharacteristics) Psyche profile and grants `-2` $\Delta_{\text{ext}}$ on the next maneuver.
pub fn evaluate_empathy_read(
    target_psyche: &impl HasPsyche,
    net_successes: i32,
) -> EmpathyReadResult {
    let sorted = target_psyche.as_psyche_profile().traits_by_extremity();
    let revealed_count = match net_successes {
        n if n <= 0 => 0,
        1 => 1,
        2 => 2,
        _ => PsycheTraitId::ALL.len(),
    };
    let revealed_traits: Vec<(PsycheTraitId, i32)> = sorted
        .iter()
        .take(revealed_count)
        .map(|&(id, score, _)| (id, score))
        .collect();

    let detects_psyche_wedge_tension = net_successes >= 2
        && sorted.len() >= 2
        && ((sorted[0].1 >= 13 && sorted[1].1 >= 13) || (sorted[0].2 >= 3 && sorted[1].2 >= 3));

    EmpathyReadResult {
        net_successes,
        revealed_traits,
        detects_psyche_wedge_tension,
        leverage_delta_ext_bonus: if net_successes >= 3 { -2 } else { 0 },
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

/// Evaluation of how a single or compound [`SocialArgumentVector`] interacts with a target's [`CharacterCharacteristics`](crate::characteristics::CharacterCharacteristics) Psyche profile
/// (`docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §4.2 & §4.4).
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
    /// and clashes with another high Psyche trait ($P_2 \ge 13$) — the **Dual-Psyche Wedge** (`§4.3–§4.4`).
    pub triggers_dual_psyche_wedge: bool,
}

fn eval_trait_pole(
    score: i32,
    trait_id: PsycheTraitId,
    high_pole_resonates: bool,
    allow_low_pole: bool,
) -> (i32, Option<PsycheTraitId>, Option<PsycheTraitId>, bool) {
    if score >= 13 {
        if high_pole_resonates {
            (-(score - 10), Some(trait_id), None, true)
        } else {
            (score - 10, None, Some(trait_id), true)
        }
    } else if allow_low_pole && score <= 7 {
        if high_pole_resonates {
            (10 - score, None, Some(trait_id), false)
        } else {
            (-(10 - score), Some(trait_id), None, false)
        }
    } else {
        (0, None, None, false)
    }
}

/// Computes the $\Delta_{\text{ext}}$ contribution of a single [`SocialArgumentVector`] (`§4.2`).
pub fn compute_single_vector_psyche_modifier(
    psyche: &impl HasPsyche,
    vector: SocialArgumentVector,
) -> (i32, Option<PsycheTraitId>, Option<PsycheTraitId>, bool) {
    let p = psyche.as_psyche_profile();
    use PsycheTraitId::{Aggression, Belonging, Conformity, Ego, Openness, Selfishness};
    match vector {
        SocialArgumentVector::EmpiricalEvidenceLogicNovelIdeas => {
            eval_trait_pole(p.opn, Openness, true, true)
        }
        SocialArgumentVector::GroupConsensusLoyaltySharedIdentity {
            asks_to_betray_group,
        } => eval_trait_pole(p.blg, Belonging, !asks_to_betray_group, !asks_to_betray_group),
        SocialArgumentVector::FlatteryPrestigeLegacyStatus {
            humiliates_or_demands_fault_admission,
        } => eval_trait_pole(p.ego, Ego, !humiliates_or_demands_fault_admission, false),
        SocialArgumentVector::PersonalProfitImmunitySelfInterest => {
            eval_trait_pole(p.slf, Selfishness, true, true)
        }
        SocialArgumentVector::SelfSacrificeHumanitarianDutyMercy => {
            eval_trait_pole(p.slf, Selfishness, false, true)
        }
        SocialArgumentVector::RulesLawProtocolChainOfCommand {
            asks_to_break_law_or_orders,
        } => eval_trait_pole(
            p.cnf,
            Conformity,
            !asks_to_break_law_or_orders,
            !asks_to_break_law_or_orders,
        ),
        SocialArgumentVector::DominanceThreatDareConfrontation {
            aims_to_bait_into_fight,
        } => eval_trait_pole(
            p.agg,
            Aggression,
            aims_to_bait_into_fight,
            !aims_to_bait_into_fight,
        ),
    }
}

/// Evaluates one or more [`SocialArgumentVector`] components against a target's [`HasPsyche`] profile,
/// computing Resonance, Clash, Net $\Delta_{\text{ext}}$, and whether a **Dual-Psyche Wedge** (`§4.3.3`) is formed.
pub fn compute_argument_psyche_modifier(
    psyche: &impl HasPsyche,
    vectors: &[SocialArgumentVector],
) -> ArgumentPsycheEvaluation {
    let mut resonance_delta_ext = 0;
    let mut clash_delta_ext = 0;
    let mut resonated_trait = None;
    let mut clashed_trait = None;
    let mut high_pole_resonated = false;
    let mut high_pole_clashed = false;

    for &vec in vectors {
        let (delta, res_t, cls_t, is_high_pole) = compute_single_vector_psyche_modifier(psyche, vec);
        if delta < 0 {
            resonance_delta_ext += delta;
            resonated_trait = resonated_trait.or(res_t);
            high_pole_resonated |= is_high_pole;
        } else if delta > 0 {
            clash_delta_ext += delta;
            clashed_trait = clashed_trait.or(cls_t);
            high_pole_clashed |= is_high_pole;
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
    compute_social_composure_score(dis, foc)
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

    /// Applies the per-turn/per-exchange **Psychological Bleeding** (`§4.3`) from active `dissonance_index`:
    /// adds $+2 \times \text{dissonance\_index}$ to `accumulated_delta_int` and returns the bleed amount.
    pub fn apply_turn_dissonance_bleed(&mut self) -> i32 {
        let bleed = 2 * self.dissonance_index.max(0);
        self.accumulated_delta_int += bleed;
        bleed
    }

    /// Computes the target's effective Skill Level $N_{\text{eff}} = \max(1, N_{\text{base}} - \text{doubt\_fractures})$
    /// when defending a fractured Pillar of Conviction (`docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §4.3, line 231).
    #[must_use]
    pub fn effective_skill_pairs(&self, base_n: usize) -> usize {
        base_n
            .saturating_sub(self.doubt_fractures.max(0) as usize)
            .max(1)
    }
}

/// Computes the Target Number $\text{TN} = 31 - \text{DIS} + (\text{EGO} - 10)$ for resisting an **Ego Wound**
/// (`docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md` §4.3, line 233).
#[must_use]
pub fn ego_wound_dis_check_tn(dis: i32, ego: i32) -> i32 {
    31 - dis.clamp(1, 20) + (ego.clamp(1, 20) - 10).max(0)
}

/// Outcome of a single Contested Social Exchange (`§4.2–§4.4`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocialExchangeOutcome {
    /// Argument Resonance vs. Clash evaluation against the target's [`CharacterCharacteristics`](crate::characteristics::CharacterCharacteristics) Psyche profile.
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

/// Evaluates the Social Wounds inflicted by a contested social exchange with `net_successes` (`§4.3`).
pub fn evaluate_social_exchange_wounds(
    target_psyche: &impl HasPsyche,
    state: &SocialComposureState,
    vectors: &[SocialArgumentVector],
    net_successes: i32,
    ego_wound_dis_check_passed: Option<bool>,
) -> SocialExchangeOutcome {
    let target_profile = target_psyche.as_psyche_profile();
    let argument_eval = compute_argument_psyche_modifier(&target_profile, vectors);
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

    let has_logical_or_empathetic_vector = vectors.iter().any(|v| {
        matches!(
            v,
            SocialArgumentVector::EmpiricalEvidenceLogicNovelIdeas
                | SocialArgumentVector::PersonalProfitImmunitySelfInterest
                | SocialArgumentVector::SelfSacrificeHumanitarianDutyMercy
                | SocialArgumentVector::RulesLawProtocolChainOfCommand { .. }
                | SocialArgumentVector::GroupConsensusLoyaltySharedIdentity { .. }
        )
    });

    if net_successes >= 2 && has_logical_or_empathetic_vector {
        wounds_inflicted.push(SocialWoundKind::Doubt);
        updated.current_cmp = (updated.current_cmp - 1).max(0);
        updated.doubt_fractures += 1;
        updated.position_demolished |= updated.doubt_fractures >= 3;
    } else {
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

    SocialExchangeOutcome {
        argument_eval,
        net_successes,
        wounds_inflicted,
        dissonance_stacks_added,
        next_turn_dissonance_bleed_delta_int: 2 * updated.dissonance_index,
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
    let mut out = DissonanceResolutionOutcome {
        updated_state: updated.clone(),
        dogmatic_denial_tn: None,
        psyche_drift_mark_awarded: None,
        openness_scene_penalty: 0,
        residual_delta_int_until_rest: 0,
        ticks_spent: 0,
        resulting_morale_state: None,
    };
    match path {
        DissonanceResolutionPath::ConversionDefection { violated_trait } => {
            updated.dissonance_index = 0;
            out.psyche_drift_mark_awarded = Some(violated_trait);
            out.resulting_morale_state = Some(MoraleState::Steady);
        }
        DissonanceResolutionPath::DogmaticDenial {
            dis_check_successes,
        } => {
            out.dogmatic_denial_tn = Some(31 - dis.clamp(1, 20) + (2 * updated.dissonance_index.max(0)));
            if dis_check_successes > 0 {
                updated.dissonance_index = 0;
                out.openness_scene_penalty = -4;
                out.resulting_morale_state = Some(MoraleState::Rage);
            }
        }
        DissonanceResolutionPath::Compartmentalization {
            dis_check_successes,
        } => {
            updated.dissonance_index = (updated.dissonance_index - dis_check_successes.max(0)).max(0);
            updated.accumulated_delta_int = 2;
            out.residual_delta_int_until_rest = 2;
            out.ticks_spent = 12;
        }
        DissonanceResolutionPath::EmotionalLashingOut => {
            updated.dissonance_index = 0;
            out.resulting_morale_state = Some(MoraleState::Rage);
        }
    }
    out.updated_state = updated;
    out
}

/// Outcome of a Social Combat / Cognitive Dissonance check from Chapter 9.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialDissonanceOutcome {
    /// Total cognitive dissonance pressure generated by argument margin, evidence, and audience.
    pub dissonance_pressure: i32,
    /// Target's psychological resistance threshold derived from core conviction.
    pub resistance_threshold: i32,
    /// Net shift (`dissonance_pressure - resistance_threshold`).
    pub net_dissonance_shift: i32,
    /// Canonical narrative social outcome tier.
    pub social_outcome: String,
}

/// Computes Cognitive Dissonance pressure, Dual-Psyche Wedge bleed, and Social Wound tier
/// using the canonical rules from `psyche_morale_and_social_combat.md` §4.
pub fn compute_social_dissonance_check(
    argument_margin: i32,
    target_conviction: i32,
    evidence_tier: i32,
    shared_values_bonus: i32,
    public_audience_pressure: i32,
) -> SocialDissonanceOutcome {
    let conviction = target_conviction.clamp(1, 20);
    let delta_psyche = (conviction - 10).abs();
    let evidence = evidence_tier.clamp(0, 5);
    let shared = shared_values_bonus.clamp(-3, 5);
    let audience = public_audience_pressure.clamp(-3, 5);

    let dissonance_pressure = (argument_margin * 2 + evidence * 3 + shared * 2 + audience).max(0);
    let resistance_threshold = 10 + delta_psyche;
    let net_shift = dissonance_pressure - resistance_threshold;
    let bleed_delta_int = argument_margin.max(0) * 2;

    let social_outcome = if argument_margin <= 0 && net_shift < -4 {
        "Dogmatic Denial / Entrenchment (Target locks OPN -4, clears Dissonance)".to_string()
    } else if net_shift < 0 {
        format!(
            "Rhetorical Strain (-1 CMP, +{} Delta_int; Target rationalizes tension)",
            bleed_delta_int.max(2)
        )
    } else if net_shift < 5 {
        format!(
            "Doubt Fracture & Cognitive Dissonance (+1 Doubt, +{bleed_delta_int} Delta_int/turn bleed)"
        )
    } else if net_shift < 10 {
        format!(
            "Dual-Psyche Wedge & Ego Wound (Dissonance Index {}, +{bleed_delta_int} Delta_int bleed/turn)",
            argument_margin.max(1)
        )
    } else {
        "Conversion / Defection (Core Belief Shift + 1 Permanent Psyche Drift Mark)".to_string()
    };

    SocialDissonanceOutcome {
        dissonance_pressure,
        resistance_threshold,
        net_dissonance_shift: net_shift,
        social_outcome,
    }
}
