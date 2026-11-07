//! Action Economy, 4-Pillar OODA Loop, Continuous Tick Timeline & Multi-Sensory Stealth
//! (`docs/ttrpg/02-core-mechanics/action_economy_and_ooda.md`).
//!
//! Implements:
//! - The 6 Temporal Layers (`0.25s` Tick through `4-Year` Lifepath Term) (§1.1)
//! - The 4-Pillar OODA Loop (`ITU`, `INS`, `D_cog`, `SPD`) and Quick Thinking ($D_{\text{cog}}$) (§2.2)
//! - Two-Component Action Cost ($\text{Base TC} = \text{TC}_{\text{decide}} + \text{TC}_{\text{act}}$) (§2.3, §2.3.1)
//! - Continuous Count-Up Tick Timeline, Starting Tick, Tie-Breaking & Surprise/Ambush Freeze (§3 Items 1–4, §3.1)
//! - Action Declaration vs. Completion Phases & Interrupt/Spoiling Resolution (§3.2)
//! - Canonical 37-Action Reference Catalog across Combat, Weapon Handling, Movement & Cognitive tasks (§4.1–§4.4)
//! - Full `action_time_factor` (Fatigue, Wound Shock, Encumbrance Tier 1–4, Concussed/Dazed status),
//!   Skill Level ($N \ge 4$, $N \ge 6$) Reflex Compression, Layer 4 Non-Combat Dual Scaling & OODA Reset Pivot (§5.1–§5.5)
//! - Multi-Sensory Perception (`SIG/FOC`, `HEA/FOC`, `OLF/INS`, `ITU/INS`), Stealth Gait/Encumbrance & Illumination/NVG/FLIR (§6.1–§6.2)

use serde::{Deserialize, Serialize};

use crate::characteristics::{compute_d_cog_score, compute_ooda_score};
use crate::primitives::{ConditionImpact, ConditionSource, Triad};

pub use crate::action_catalog_stealth::*;
/// The 6 canonical time scales of the Satya Engine (§1.1, calibrated at `1 Tick = 0.25s`, `4 Ticks = 1s`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalLayer {
    /// Layer 1: `0.25 seconds` per Tick (`4 Ticks = 1s`, continuous count-up timeline)
    Layer1TacticalTick,
    /// Layer 2: `3–7 seconds` (`12–28 Ticks`) single combat or emergency action window
    Layer2ActionWindow,
    /// Layer 3: `5 minutes` post-combat triage, quick lockpick, room search, breather
    Layer3BreatherTurn,
    /// Layer 4: `15–60 minutes` per roll (Extended Project Clocks: surgery, hacking, repairs)
    Layer4ExtendedProject,
    /// Layer 5: `4–8 hours` overland travel, long-term surveillance, rest & fatigue recovery
    Layer5WatchShift,
    /// Layer 6: `1 week – 1 month` (or `4-year` Lifepath career term)
    Layer6CampaignDowntime,
}

/// The 5 Canonical Encumbrance Tiers (`Tiers 0–4`) governing physical action speed, Tempo Cap, `KIN`,
/// and stealth noise/silhouette (`action_economy_and_ooda.md` §5.1, §6.1; `movement_vehicles_and_environment.md` §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EncumbranceTier {
    /// **Tier 0: Unencumbered** ($\le 1.0 \times L_{\text{base}}$, `+0.00` physical time factor, `0` $T_{\max}$, `+0` stealth $\Delta_{\text{ext}}$).
    #[default]
    #[serde(alias = "TIER1_UNENCUMBERED", alias = "tier0_unencumbered")]
    Tier0Unencumbered,
    /// **Tier 1: Light / Patrol Load** ($1.01\text{–}1.5 \times L_{\text{base}}$, `+0.05` physical time factor, `-1` $T_{\max}$, `+1` stealth/parkour $\Delta_{\text{ext}}$).
    #[serde(alias = "TIER1_LIGHT_PATROL", alias = "tier1_light_patrol_load")]
    Tier1LightPatrolLoad,
    /// **Tier 2: Tactical / Fighting Load** ($1.51\text{–}2.5 \times L_{\text{base}}$, `+0.10` physical time factor, `-3` $T_{\max}$, `-2` `KIN`, `+2` stealth $\Delta_{\text{ext}}$).
    #[serde(alias = "TIER2_LIGHT", alias = "tier2_tactical_fighting_load")]
    Tier2TacticalFightingLoad,
    /// **Tier 3: Heavy / March Load** ($2.51\text{–}4.0 \times L_{\text{base}}$, `+0.25` physical time factor, `-6` $T_{\max}$, `-4` `KIN`, `+4` stealth $\Delta_{\text{ext}}$, cannot sprint).
    #[serde(alias = "TIER3_MEDIUM", alias = "tier3_heavy_march_load")]
    Tier3HeavyMarchLoad,
    /// **Tier 4: Overloaded / Staggering** ($4.01\text{–}6.0 \times L_{\text{base}}$, `+0.50` physical time factor, `-10` $T_{\max}$, `-8` `KIN`, `+6` stealth $\Delta_{\text{ext}}$, cannot sprint).
    #[serde(alias = "TIER4_HEAVY", alias = "tier4_overloaded_staggering")]
    Tier4OverloadedStaggering,
}

impl EncumbranceTier {
    /// Classify a load ratio $L_{\text{eff}} / L_{\text{base}}$ into its canonical `EncumbranceTier` (`0..=4`).
    #[must_use]
    pub fn from_load_ratio(load_ratio: f64) -> Self {
        if load_ratio <= 1.0 + 1e-9 {
            Self::Tier0Unencumbered
        } else if load_ratio <= 1.5 + 1e-9 {
            Self::Tier1LightPatrolLoad
        } else if load_ratio <= 2.5 + 1e-9 {
            Self::Tier2TacticalFightingLoad
        } else if load_ratio <= 4.0 + 1e-9 {
            Self::Tier3HeavyMarchLoad
        } else {
            Self::Tier4OverloadedStaggering
        }
    }

    #[must_use]
    pub const fn tier_number(self) -> u8 {
        match self {
            Self::Tier0Unencumbered => 0,
            Self::Tier1LightPatrolLoad => 1,
            Self::Tier2TacticalFightingLoad => 2,
            Self::Tier3HeavyMarchLoad => 3,
            Self::Tier4OverloadedStaggering => 4,
        }
    }

    #[must_use]
    pub const fn from_tier_number(tier: u8) -> Self {
        match tier {
            ..=1 => Self::Tier0Unencumbered,
            2 => Self::Tier2TacticalFightingLoad,
            3 => Self::Tier3HeavyMarchLoad,
            4.. => Self::Tier4OverloadedStaggering,
        }
    }

    #[must_use]
    pub const fn from_load_tier_0_to_4(tier_0_to_4: u8) -> Self {
        match tier_0_to_4 {
            0 => Self::Tier0Unencumbered,
            1 => Self::Tier1LightPatrolLoad,
            2 => Self::Tier2TacticalFightingLoad,
            3 => Self::Tier3HeavyMarchLoad,
            4.. => Self::Tier4OverloadedStaggering,
        }
    }

    /// Whether the character can perform an All-Out Sprint (`false` at `Tier 3+`, `movement_vehicles_and_environment.md` §2.3).
    #[must_use]
    pub const fn can_sprint(self) -> bool {
        self.tier_number() < 3
    }

    /// Penalty to **Tempo Cap ($T_{\max}$)** (`0`, `1`, `3`, `6`, `10`).
    #[must_use]
    pub const fn tempo_cap_penalty(self) -> i32 {
        match self {
            Self::Tier0Unencumbered => 0,
            Self::Tier1LightPatrolLoad => 1,
            Self::Tier2TacticalFightingLoad => 3,
            Self::Tier3HeavyMarchLoad => 6,
            Self::Tier4OverloadedStaggering => 10,
        }
    }

    /// Penalty to **Kinesthesis (`KIN`)** (`0`, `0`, `2`, `4`, `8`).
    #[must_use]
    pub const fn kin_penalty(self) -> i32 {
        match self {
            Self::Tier0Unencumbered | Self::Tier1LightPatrolLoad => 0,
            Self::Tier2TacticalFightingLoad => 2,
            Self::Tier3HeavyMarchLoad => 4,
            Self::Tier4OverloadedStaggering => 8,
        }
    }

    /// Physical `action_time_factor` delta ($\Delta_{\text{enc}}$) (§5.1, `movement_vehicles_and_environment.md` §2.3).
    ///
    /// Note: Applies **only** to physical $\text{TC}_{\text{act}}$; purely mental actions ignore physical load.
    #[must_use]
    pub const fn physical_time_factor_delta(self) -> f64 {
        match self {
            Self::Tier0Unencumbered => 0.0,
            Self::Tier1LightPatrolLoad => 0.05,
            Self::Tier2TacticalFightingLoad => 0.10,
            Self::Tier3HeavyMarchLoad => 0.25,
            Self::Tier4OverloadedStaggering => 0.50,
        }
    }

    #[must_use]
    pub const fn action_time_factor_delta(self) -> f64 {
        self.physical_time_factor_delta()
    }

    /// Acoustic & silhouette penalty (`0`, `1`, `2`, `4`, `6` $\Delta_{\text{ext}}$) for Stealth (§6.1).
    #[must_use]
    pub const fn stealth_penalty_delta_ext(self) -> i32 {
        match self {
            Self::Tier0Unencumbered => 0,
            Self::Tier1LightPatrolLoad => 1,
            Self::Tier2TacticalFightingLoad => 2,
            Self::Tier3HeavyMarchLoad => 4,
            Self::Tier4OverloadedStaggering => 6,
        }
    }
}

impl ConditionSource for EncumbranceTier {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_ext: self.stealth_penalty_delta_ext(),
            action_time_factor_delta: self.physical_time_factor_delta(),
            triad_cap_penalty: Triad::new(self.tempo_cap_penalty(), 0, 0),
            ..ConditionImpact::ZERO
        }
    }
}

/// Parameters for computing the canonical `action_time_factor` and cognitive status penalties (§5.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionTimeFactorParams {
    pub fatigue_level: i32,
    /// Current Wound Shock Index (`>= 0.0`, `+0.25` per point)
    pub wound_shock_index: f64,
    pub encumbrance_tier: EncumbranceTier,
    /// Status penalty (`+0.25` to `+0.50`) from Concussed / Dazed / Acute Mental Shock (`0.0` if none)
    pub status_time_penalty: f64,
    pub is_concussed_or_dazed: bool,
}

impl Default for ActionTimeFactorParams {
    fn default() -> Self {
        Self {
            fatigue_level: 0,
            wound_shock_index: 0.0,
            encumbrance_tier: EncumbranceTier::Tier0Unencumbered,
            status_time_penalty: 0.0,
            is_concussed_or_dazed: false,
        }
    }
}

impl ActionTimeFactorParams {
    /// Compute the physical `action_time_factor` (includes $\Delta_{\text{enc}}$) (§5.1).
    #[must_use]
    pub fn physical_action_time_factor(&self) -> f64 {
        let fatigue = f64::from(self.fatigue_level.max(0)) * 0.10;
        let shock = self.wound_shock_index.max(0.0) * 0.25;
        let enc = self.encumbrance_tier.physical_time_factor_delta();
        let status = self.status_time_penalty.clamp(0.0, 1.0);
        1.0 + fatigue + shock + enc + status
    }

    /// Compute the purely cognitive `action_time_factor` (ignores physical $\Delta_{\text{enc}}$) (§5.1, §5.4.1).
    #[must_use]
    pub fn cognitive_action_time_factor(&self) -> f64 {
        let fatigue = f64::from(self.fatigue_level.max(0)) * 0.10;
        let shock = self.wound_shock_index.max(0.0) * 0.25;
        let status = self.status_time_penalty.clamp(0.0, 1.0);
        1.0 + fatigue + shock + status
    }

    /// Effective $D_{\text{cog}}$ after applying the `-4` Concussed / Dazed penalty (§5.1).
    #[must_use]
    pub const fn effective_d_cog(&self, base_d_cog: i32) -> i32 {
        let raw = if self.is_concussed_or_dazed {
            base_d_cog - 4
        } else {
            base_d_cog
        };
        if raw < 1 {
            1
        } else if raw > 20 {
            20
        } else {
            raw
        }
    }
}

/// Compute Quick Thinking ($D_{\text{cog}}$) and 4-Pillar OODA Initiative (§2.2, §3.1).
///
/// Delegates to [`compute_d_cog_score`] and [`compute_ooda_score`] after applying the
/// Concussed / Dazed `-4` $D_{\text{cog}}$ penalty when `is_concussed_or_dazed` is true.
#[must_use]
pub fn compute_d_cog_and_ooda(
    itu: i32,
    ins: i32,
    ana: i32,
    cre: i32,
    foc: i32,
    spd: i32,
    is_concussed_or_dazed: bool,
) -> (i32, i32) {
    let base_d_cog = compute_d_cog_score(ana, cre, foc);
    let eff_d_cog = if is_concussed_or_dazed {
        (base_d_cog - 4).max(1)
    } else {
        base_d_cog
    };
    let ooda = compute_ooda_score(itu, ins, eff_d_cog, spd);
    (eff_d_cog, ooda)
}

/// Compute the Cognitive Decision Offset $\Delta\text{TC}_{\text{decide}} = -\lfloor(D_{\text{cog}} - 10) / 2\rfloor$ (§2.3, §5.2).
#[must_use]
pub const fn compute_delta_tc_decide(d_cog: i32) -> i32 {
    -(d_cog - 10).div_euclid(2)
}

/// Compute Skill Level ($N$) Reflex Compression on $\text{TC}_{\text{decide}}$ (`action_economy_and_ooda.md` §2.3.1, §5.2).
///
/// - $N \in [0, 3]$: `0` ticks compression
/// - $N \in [4, 5]$: `1` tick compression (`-1 Tick` on $\text{TC}_{\text{decide}}$)
/// - $N \ge 6$: `2` ticks compression (`-2 Ticks` on $\text{TC}_{\text{decide}}$)
#[must_use]
pub const fn compute_skill_reflex_compression(skill_level_n: usize) -> i32 {
    match skill_level_n {
        0..=3 => 0,
        4..=5 => 1,
        _ => 2,
    }
}

/// Result of evaluating a two-component ($\text{TC}_{\text{decide}} + \text{TC}_{\text{act}}$) action cost (`action_economy_and_ooda.md` §2.3, §3.2, §5.1–§5.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwoComponentActionCost {
    pub base_tc_decide: i32,
    pub base_tc_act: i32,
    pub base_total_tc: i32,
    /// Effective $D_{\text{cog}}$ after Concussed/Dazed penalty.
    pub effective_d_cog: i32,
    /// Cognitive decision offset $\Delta\text{TC}_{\text{decide}}$.
    pub delta_tc_decide: i32,
    pub skill_reflex_compression: i32,
    pub effective_tc_decide: i32,
    pub effective_tc_act: i32,
    pub physical_time_factor: f64,
    pub effective_total_tc: i32,
    pub wind_up_ticks: i32,
    pub recovery_ticks: i32,
}

/// Evaluate the canonical two-component Combat / Tactical Action Tick Cost (`action_economy_and_ooda.md` §2.3, §5.1–§5.3).
///
/// - Splits `Base TC = base_tc_decide + base_tc_act`.
/// - Scales $\text{TC}_{\text{decide}}$ by `cognitive_action_time_factor()` (§5.1–§5.2: Fatigue, Wound Shock, and Status slow both `TC_decide` and `TC_act`)
///   and applies $\Delta\text{TC}_{\text{decide}} = -\lfloor(D_{\text{cog}} - 10)/2\rfloor$ (when `apply_cognitive_offset` is true)
///   and Skill Level Reflex Compression ($N \ge 4 \implies -1$, $N \ge 6 \implies -2$).
/// - Scales $\text{TC}_{\text{act}}$ by the physical `action_time_factor` and subtracts `(tempo_allocated - weapon_balance_b)`.
/// - Enforces `min_total_tc_floor` (`2` ticks for standard actions, `1` tick for reflexes/free callouts).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_combat_action_cost(
    base_tc_decide: i32,
    base_tc_act: i32,
    d_cog: i32,
    skill_level_n: usize,
    apply_cognitive_offset: bool,
    tempo_allocated: i32,
    weapon_balance_b: i32,
    min_total_tc_floor: i32,
    time_factor_params: &ActionTimeFactorParams,
) -> TwoComponentActionCost {
    let base_decide = base_tc_decide.max(0);
    let base_act = base_tc_act.max(0);
    let base_total_tc = base_decide + base_act;

    let effective_d_cog = time_factor_params.effective_d_cog(d_cog);
    let delta_tc_decide = if apply_cognitive_offset && base_decide > 0 {
        compute_delta_tc_decide(effective_d_cog)
    } else {
        0
    };
    let skill_reflex_compression = if base_decide > 0 {
        compute_skill_reflex_compression(skill_level_n)
    } else {
        0
    };

    // Floor for TC_decide: if base_decide > 0, N >= 6 can compress it to 0, otherwise floor is 1.
    let decide_floor = if base_decide == 0 || skill_level_n >= 6 {
        0
    } else {
        1
    };
    let cognitive_time_factor = time_factor_params.cognitive_action_time_factor();
    let scaled_decide = if base_decide > 0 {
        (f64::from(base_decide) * cognitive_time_factor).round() as i32
    } else {
        0
    };
    let raw_decide = scaled_decide + delta_tc_decide - skill_reflex_compression;
    let effective_tc_decide = raw_decide.max(decide_floor);

    let physical_time_factor = if base_act > 0 {
        time_factor_params.physical_action_time_factor()
    } else {
        cognitive_time_factor
    };

    let effective_tc_act = if base_act > 0 {
        let scaled_act = (f64::from(base_act) * physical_time_factor).round() as i32;
        let tempo_delta = tempo_allocated - weapon_balance_b;
        (scaled_act - tempo_delta).max(1)
    } else {
        0
    };

    let floor = min_total_tc_floor.max(1);
    let effective_total_tc = (effective_tc_decide + effective_tc_act).max(floor);
    let wind_up_ticks =
        (effective_tc_decide + (effective_tc_act + 1).div_euclid(2)).min(effective_total_tc);
    let recovery_ticks = (effective_total_tc - wind_up_ticks).max(0);

    TwoComponentActionCost {
        base_tc_decide: base_decide,
        base_tc_act: base_act,
        base_total_tc,
        effective_d_cog,
        delta_tc_decide,
        skill_reflex_compression,
        effective_tc_decide,
        effective_tc_act,
        physical_time_factor,
        effective_total_tc,
        wind_up_ticks,
        recovery_ticks,
    }
}

/// Result of evaluating a Layer 3 / Layer 4 Non-Combat Action or Extended Project interval (§5.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonCombatActionDuration {
    pub base_tc_decide: i32,
    pub base_tc_act: i32,
    /// Cognitive decision offset $\Delta\text{TC}_{\text{decide}}$.
    pub delta_tc_decide: i32,
    /// Physical motor execution offset $\Delta\text{TC}_{\text{act}}$.
    pub delta_tc_act: i32,
    pub skill_reflex_compression: i32,
    pub adjusted_decide_units: i32,
    pub adjusted_act_units: i32,
    pub time_factor: f64,
    pub outcome_duration_multiplier: f64,
    pub effective_duration_units: f64,
}

/// Evaluate a Non-Combat / Extended Task Duration with Dual Cognitive & Motor Scaling (§5.4.1–§5.4.2).
///
/// - Cognitive phase scales with $\Delta\text{TC}_{\text{decide}} = -\lfloor(D_{\text{cog}} - 10)/2\rfloor$ and Skill Reflex Compression.
/// - Physical phase scales with $\Delta\text{TC}_{\text{act}} = -\lfloor(C_{\text{motor}} - 10)/2\rfloor$ (`DEX`, `SPD`, or `KIN`).
/// - Purely mental tasks (`base_tc_act == 0`) ignore physical encumbrance ($\Delta_{\text{enc}}$).
/// - Extraordinary successes compress total duration by `15%` per net success above `1` (floor `0.25`).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_non_combat_action_duration(
    base_tc_decide: i32,
    base_tc_act: i32,
    d_cog: i32,
    c_motor: i32,
    skill_level_n: usize,
    net_successes: i32,
    grazing_tie: bool,
    time_factor_params: &ActionTimeFactorParams,
) -> NonCombatActionDuration {
    let eff_d_cog = time_factor_params.effective_d_cog(d_cog);
    let delta_tc_decide = if base_tc_decide > 0 {
        compute_delta_tc_decide(eff_d_cog)
    } else {
        0
    };
    let skill_reflex_compression = if base_tc_decide > 0 {
        compute_skill_reflex_compression(skill_level_n)
    } else {
        0
    };
    let adjusted_decide_units = if base_tc_decide > 0 {
        (base_tc_decide + delta_tc_decide - skill_reflex_compression).max(1)
    } else {
        0
    };

    let delta_tc_act = if base_tc_act > 0 {
        -(c_motor.clamp(1, 20) - 10).div_euclid(2)
    } else {
        0
    };
    let adjusted_act_units = if base_tc_act > 0 {
        (base_tc_act + delta_tc_act).max(1)
    } else {
        0
    };

    let cog_factor = time_factor_params.cognitive_action_time_factor();
    let phys_factor = time_factor_params.physical_action_time_factor();
    let time_factor = if base_tc_act > 0 {
        phys_factor
    } else {
        cog_factor
    };

    let outcome_duration_multiplier = if grazing_tie {
        1.25
    } else if net_successes <= 0 {
        1.0
    } else {
        let extra = net_successes - 1;
        (1.0 - 0.15 * f64::from(extra)).max(0.25)
    };

    let scaled_combined = (f64::from(adjusted_decide_units) * cog_factor
        + f64::from(adjusted_act_units) * phys_factor)
        .max(1.0);
    let effective_duration_units = (scaled_combined * outcome_duration_multiplier).max(0.25);

    NonCombatActionDuration {
        base_tc_decide: base_tc_decide.max(0),
        base_tc_act: base_tc_act.max(0),
        delta_tc_decide,
        delta_tc_act,
        skill_reflex_compression,
        adjusted_decide_units,
        adjusted_act_units,
        time_factor,
        outcome_duration_multiplier,
        effective_duration_units,
    }
}

/// Compute Starting Tick on the continuous timeline, including Surprise / Ambush Orientation Freeze (§3.1).
///
/// $$\text{Tick}_{\text{start}} = \max(1, 15 - \text{OODA} - \text{Awareness Margin}) + \text{Effective Surprise Freeze}$$
///
/// When surprised (`base_surprise_penalty_ticks > 0`, typically `8..=12` ticks), the freeze is reduced by
/// $\max(0, \lfloor(D_{\text{cog}} - 10)/2\rfloor) + \max(0, \lfloor(\text{DIS} - 10)/2\rfloor)$.
#[must_use]
pub fn compute_starting_tick(
    ooda: i32,
    awareness_margin: i32,
    base_surprise_penalty_ticks: i32,
    d_cog: i32,
    dis: i32,
) -> i32 {
    let base_start = (15 - ooda - awareness_margin).max(1);
    if base_surprise_penalty_ticks <= 0 {
        return base_start;
    }
    let cog_mitigation = ((d_cog - 10).div_euclid(2)).max(0);
    let dis_mitigation = ((dis - 10).div_euclid(2)).max(0);
    let effective_surprise = (base_surprise_penalty_ticks - cog_mitigation - dis_mitigation).max(0);
    base_start + effective_surprise
}

/// Combatant entry on the continuous Count-Up Tick Timeline (`action_economy_and_ooda.md` §3 Items 1–4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickTimelineCombatant {
    pub combatant_id: String,
    pub current_tick: i32,
    /// 4-Pillar OODA Initiative score (`1..=20`).
    pub ooda: i32,
    /// Quick Thinking score $D_{\text{cog}}$ (`1..=20`).
    pub d_cog: i32,
    /// Speed characteristic (`SPD`, `1..=20`).
    pub spd: i32,
}

/// Sort combatants according to the canonical Count-Up Tick Timeline & Tie-Breaking rules (`action_economy_and_ooda.md` §3 Items 1–4):
/// 1. Lowest `current_tick` acts first
/// 2. Highest `ooda` breaks ties
/// 3. Highest `d_cog` breaks remaining ties
/// 4. Highest `spd` breaks remaining ties
pub fn sort_tick_timeline(combatants: &mut [TickTimelineCombatant]) {
    combatants.sort_by(|a, b| {
        a.current_tick
            .cmp(&b.current_tick)
            .then_with(|| b.ooda.cmp(&a.ooda))
            .then_with(|| b.d_cog.cmp(&a.d_cog))
            .then_with(|| b.spd.cmp(&a.spd))
            .then_with(|| a.combatant_id.cmp(&b.combatant_id))
    });
}

/// Compute the OODA Reset / Action Pivot Penalty in ticks when aborting a declared action mid-stream (`action_economy_and_ooda.md` §3.2, §5.5).
///
/// $$\text{OODA Reset Ticks} = \max\left(1, 5 - \left\lfloor\frac{D_{\text{cog}} + \text{ITU}}{5}\right\rfloor\right)$$
#[must_use]
pub const fn compute_ooda_reset_pivot_ticks(d_cog: i32, itu: i32) -> i32 {
    let bonus = (d_cog + itu).div_euclid(5);
    let raw = 5 - bonus;
    if raw < 1 { 1 } else { raw }
}

/// Outcome of checking whether an incoming fast attack or suppression interrupts a declared action (`action_economy_and_ooda.md` §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionInterruptOutcome {
    pub declared_tick: i32,
    pub wind_up_end_tick: i32,
    pub scheduled_completion_tick: i32,
    pub interrupt_tick: i32,
    pub action_spoiled: bool,
    pub struck_during_wind_up: bool,
    pub resulting_actor_tick: i32,
}

/// Evaluate whether an incoming event at `interrupt_tick` interrupts and spoils a declared action (`action_economy_and_ooda.md` §3.2).
#[must_use]
pub fn evaluate_action_interrupt(
    declared_tick: i32,
    cost: &TwoComponentActionCost,
    interrupt_tick: i32,
    causes_shock_or_failed_composure: bool,
    shock_delay_ticks: i32,
) -> ActionInterruptOutcome {
    let wind_up_end_tick = declared_tick + cost.wind_up_ticks;
    let scheduled_completion_tick = declared_tick + cost.effective_total_tc;
    let struck_before_completion =
        interrupt_tick >= declared_tick && interrupt_tick < scheduled_completion_tick;
    let struck_during_wind_up =
        interrupt_tick >= declared_tick && interrupt_tick <= wind_up_end_tick;
    let action_spoiled = struck_before_completion && causes_shock_or_failed_composure;

    let resulting_actor_tick = if action_spoiled {
        interrupt_tick + shock_delay_ticks.max(1)
    } else {
        scheduled_completion_tick
    };

    ActionInterruptOutcome {
        declared_tick,
        wind_up_end_tick,
        scheduled_completion_tick,
        interrupt_tick,
        action_spoiled,
        struck_during_wind_up,
        resulting_actor_tick,
    }
}

