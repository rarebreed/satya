//! Action Economy, 4-Pillar OODA Loop, Continuous Tick Timeline & Multi-Sensory Stealth
//! (`02-core-mechanics/action_economy_and_ooda.md`).
//!
//! Implements:
//! - The 6 Temporal Layers (`0.5s` Tick through `4-Year` Lifepath Term) (§1.1)
//! - The 4-Pillar OODA Loop (`ITU`, `INS`, `D_cog`, `SPD`) and Quick Thinking ($D_{\text{cog}}$) (§2.2)
//! - Two-Component Action Cost ($\text{Base TC} = \text{TC}_{\text{decide}} + \text{TC}_{\text{act}}$) (§2.3)
//! - Continuous Count-Up Tick Timeline, Starting Tick, Tie-Breaking & Surprise/Ambush Freeze (§3.1–§3.3)
//! - Action Declaration vs. Completion Phases & Interrupt/Spoiling Resolution (§3.4)
//! - Canonical 37-Action Reference Catalog across Combat, Weapon Handling, Movement & Cognitive tasks (§4.1–§4.4)
//! - Full `action_time_factor` (Fatigue, Wound Shock, Encumbrance Tier 1–4, Concussed/Dazed status),
//!   Skill Level ($N \ge 4$, $N \ge 6$) Reflex Compression, Layer 4 Non-Combat Dual Scaling & OODA Reset Pivot (§5.1–§5.5)
//! - Multi-Sensory Perception (`SIG/FOC`, `HEA/FOC`, `OLF/INS`, `ITU/INS`), Stealth Gait/Encumbrance & Illumination/NVG/FLIR (§6.1–§6.2)

use serde::{Deserialize, Serialize};

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

/// Encumbrance tier governing physical action speed and stealth noise/silhouette (§5.1, §6.1, and `movement_vehicles_and_environment.md` §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EncumbranceTier {
    /// Tier 0 / 1: Unencumbered ($\le 1.0 \times L_{\text{base}}$, `+0.00` time factor, `+0` stealth $\Delta_{\text{ext}}$)
    Tier1Unencumbered,
    /// Tier 1 Patrol Load ($1.01\text{–}1.5 \times L_{\text{base}}$, `+0.05` physical time factor, `-1` $T_{\max}$, `+1` stealth/parkour $\Delta_{\text{ext}}$)
    Tier1LightPatrol,
    /// Tier 2 Tactical / Fighting Load ($1.51\text{–}2.5 \times L_{\text{base}}$, `+0.10` physical time factor, `-3` $T_{\max}$, `-2` `KIN`, `+2` stealth $\Delta_{\text{ext}}$)
    Tier2Light,
    /// Tier 3 Heavy / March Load ($2.51\text{–}4.0 \times L_{\text{base}}$, `+0.25` physical time factor, `-6` $T_{\max}$, `-4` `KIN`, `+4` stealth $\Delta_{\text{ext}}$)
    Tier3Medium,
    /// Tier 4 Overloaded / Staggering ($4.01\text{–}6.0 \times L_{\text{base}}$, `+0.50` physical time factor, `-10` $T_{\max}$, `-8` `KIN`, `+6` stealth $\Delta_{\text{ext}}$)
    Tier4Heavy,
}

impl EncumbranceTier {
    /// Convert a legacy `1..=4` numeric tier into `EncumbranceTier`.
    #[must_use]
    pub const fn from_tier_number(tier: u8) -> Self {
        match tier {
            ..=1 => Self::Tier1Unencumbered,
            2 => Self::Tier2Light,
            3 => Self::Tier3Medium,
            4.. => Self::Tier4Heavy,
        }
    }

    /// Convert a `0..=4` [`crate::health_survival::LoadEncumbranceTier`] number (`0..=4`) into `EncumbranceTier`.
    #[must_use]
    pub const fn from_load_tier_0_to_4(tier_0_to_4: u8) -> Self {
        match tier_0_to_4 {
            0 => Self::Tier1Unencumbered,
            1 => Self::Tier1LightPatrol,
            2 => Self::Tier2Light,
            3 => Self::Tier3Medium,
            4.. => Self::Tier4Heavy,
        }
    }

    /// Physical `action_time_factor` delta ($\Delta_{\text{enc}}$) (§5.1, `movement_vehicles_and_environment.md` §2.3).
    ///
    /// Note: Applies **only** to physical $\text{TC}_{\text{act}}$; purely mental actions ignore physical load.
    #[must_use]
    pub const fn physical_time_factor_delta(self) -> f64 {
        match self {
            Self::Tier1Unencumbered => 0.0,
            Self::Tier1LightPatrol => 0.05,
            Self::Tier2Light => 0.10,
            Self::Tier3Medium => 0.25,
            Self::Tier4Heavy => 0.50,
        }
    }

    /// Acoustic & silhouette penalty (`+2` $\Delta_{\text{ext}}$ per tier) for Stealth (§6.1).
    #[must_use]
    pub const fn stealth_penalty_delta_ext(self) -> i32 {
        match self {
            Self::Tier1Unencumbered => 0,
            Self::Tier1LightPatrol => 1,
            Self::Tier2Light => 2,
            Self::Tier3Medium => 4,
            Self::Tier4Heavy => 6,
        }
    }
}

/// Parameters for computing the canonical `action_time_factor` and cognitive status penalties (§5.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionTimeFactorParams {
    /// Current Fatigue Level (`0..=6+`, `+0.10` per level)
    pub fatigue_level: i32,
    /// Current Wound Shock Index (`>= 0.0`, `+0.25` per point)
    pub wound_shock_index: f64,
    /// Physical Encumbrance Tier (`Tier 1..=4`, applies only to physical `TC_act`)
    pub encumbrance_tier: EncumbranceTier,
    /// Status penalty (`+0.25` to `+0.50`) from Concussed / Dazed / Acute Mental Shock (`0.0` if none)
    pub status_time_penalty: f64,
    /// Whether the character is currently Concussed / Dazed / in Acute Mental Shock (`-4` to $D_{\text{cog}}$)
    pub is_concussed_or_dazed: bool,
}

impl Default for ActionTimeFactorParams {
    fn default() -> Self {
        Self {
            fatigue_level: 0,
            wound_shock_index: 0.0,
            encumbrance_tier: EncumbranceTier::Tier1Unencumbered,
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
#[must_use]
pub const fn compute_d_cog_and_ooda(
    itu: i32,
    ins: i32,
    ana: i32,
    cre: i32,
    foc: i32,
    spd: i32,
    is_concussed_or_dazed: bool,
) -> (i32, i32) {
    let max_cog = if ana >= cre { ana } else { cre };
    let base_d_cog = (max_cog + foc).div_euclid(2);
    let eff_d_cog = if is_concussed_or_dazed {
        let v = base_d_cog - 4;
        if v < 1 {
            1
        } else {
            v
        }
    } else {
        base_d_cog
    };
    let ooda = (itu + ins + eff_d_cog + spd).div_euclid(4);
    (eff_d_cog, ooda)
}

/// Compute the Cognitive Decision Offset $\Delta\text{TC}_{\text{decide}} = -\lfloor(D_{\text{cog}} - 10) / 2\rfloor$ (§2.3, §5.2).
#[must_use]
pub const fn compute_delta_tc_decide(d_cog: i32) -> i32 {
    -(d_cog - 10).div_euclid(2)
}

/// Compute Skill Level ($N$) Reflex Compression on $\text{TC}_{\text{decide}}$ (§5.3).
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

/// Result of evaluating a two-component ($\text{TC}_{\text{decide}} + \text{TC}_{\text{act}}$) action cost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwoComponentActionCost {
    pub base_tc_decide: i32,
    pub base_tc_act: i32,
    pub base_total_tc: i32,
    pub effective_d_cog: i32,
    pub delta_tc_decide: i32,
    pub skill_reflex_compression: i32,
    pub effective_tc_decide: i32,
    pub effective_tc_act: i32,
    pub physical_time_factor: f64,
    pub effective_total_tc: i32,
    /// Wind-up duration in ticks before the action executes (`effective_tc_decide + ceil(effective_tc_act / 2)`) (§3.4)
    pub wind_up_ticks: i32,
    /// Release & recovery ticks (`effective_total_tc - wind_up_ticks`) (§3.4)
    pub recovery_ticks: i32,
}

/// Evaluate the canonical two-component Combat / Tactical Action Tick Cost (§2.3, §5.1–§5.3).
///
/// - Splits `Base TC = base_tc_decide + base_tc_act`.
/// - Applies $\Delta\text{TC}_{\text{decide}} = -\lfloor(D_{\text{cog}} - 10)/2\rfloor$ (when `apply_cognitive_offset` is true)
///   and Skill Level Reflex Compression ($N \ge 4 \implies -1$, $N \ge 6 \implies -2$) to $\text{TC}_{\text{decide}}$.
/// - Scales $\text{TC}_{\text{act}}$ by the physical `action_time_factor` and subtracts `(tempo_allocated - weapon_balance_b)`.
/// - Enforces `min_total_tc_floor` (`2` ticks for standard actions, `1` tick for reflexes/free callouts).
#[must_use]
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
    let raw_decide = base_decide + delta_tc_decide - skill_reflex_compression;
    let effective_tc_decide = raw_decide.max(decide_floor);

    let physical_time_factor = if base_act > 0 {
        time_factor_params.physical_action_time_factor()
    } else {
        time_factor_params.cognitive_action_time_factor()
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
    pub delta_tc_decide: i32,
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

    let time_factor = if base_tc_act > 0 {
        time_factor_params.physical_action_time_factor()
    } else {
        time_factor_params.cognitive_action_time_factor()
    };

    let outcome_duration_multiplier = if grazing_tie {
        1.25
    } else if net_successes <= 0 {
        1.0
    } else {
        let extra = net_successes - 1;
        (1.0 - 0.15 * f64::from(extra)).max(0.25)
    };

    let base_combined = f64::from(adjusted_decide_units + adjusted_act_units).max(1.0);
    let effective_duration_units =
        (base_combined * time_factor * outcome_duration_multiplier).max(0.25);

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
    let effective_surprise =
        (base_surprise_penalty_ticks - cog_mitigation - dis_mitigation).max(0);
    base_start + effective_surprise
}

/// Combatant entry on the continuous Count-Up Tick Timeline (§3.1–§3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickTimelineCombatant {
    pub combatant_id: String,
    pub current_tick: i32,
    pub ooda: i32,
    pub d_cog: i32,
    pub spd: i32,
}

/// Sort combatants according to the canonical Count-Up Tick Timeline & Tie-Breaking rules (§3.1):
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

/// Compute the OODA Reset / Action Pivot Penalty in ticks when aborting a declared action mid-stream (§3.4, §5.5).
///
/// $$\text{OODA Reset Ticks} = \max\left(1, 5 - \left\lfloor\frac{D_{\text{cog}} + \text{ITU}}{5}\right\rfloor\right)$$
#[must_use]
pub const fn compute_ooda_reset_pivot_ticks(d_cog: i32, itu: i32) -> i32 {
    let bonus = (d_cog + itu).div_euclid(5);
    let raw = 5 - bonus;
    if raw < 1 {
        1
    } else {
        raw
    }
}

/// Outcome of checking whether an incoming fast attack or suppression interrupts a declared action (§3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionInterruptOutcome {
    pub declared_tick: i32,
    pub wind_up_end_tick: i32,
    pub scheduled_completion_tick: i32,
    pub interrupt_tick: i32,
    /// True if the interrupt arrived before `scheduled_completion_tick` and caused shock/knockdown/failed composure.
    pub action_spoiled: bool,
    /// True if the interrupt struck during the vulnerable Wind-Up phase (`interrupt_tick <= wind_up_end_tick`).
    pub struck_during_wind_up: bool,
    /// Resulting tick position of the actor after the interrupt or normal completion.
    pub resulting_actor_tick: i32,
}

/// Evaluate whether an incoming event at `interrupt_tick` interrupts and spoils a declared action (§3.4).
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

/// Category of an action in the Reference Catalog (§4.1–§4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CatalogActionCategory {
    CombatAttackOrDefense,
    WeaponHandlingAndReload,
    MovementAndPosture,
    CognitivePerceptualAndComms,
}

/// Canonical identifier for all 37 actions in the Reference Catalog of Base Action Time Costs (§4.1–§4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CanonicalActionId {
    // §4.1 Combat Actions (Attacks & Defenses)
    SnapShotPointBlank,
    StandardMeleeStrikeLight,
    StandardRangedFire,
    DefensiveGuardParryStance,
    TacticalManeuverProbeFeint,
    AimedShotSingle,
    HeavyPowerStrikeCleave,
    SuppressiveFireBurstZone,
    ActiveDodgeDiveForCover,
    OverwatchTriggeredRelease,

    // §4.2 Weapon Handling, Reloads & Malfunctions
    DropWeaponOrItem,
    SwitchFireSelectorOrOptic,
    ClearSimpleMalfunctionTapRack,
    DrawHolsteredSidearmOrBlade,
    SlingTransitionRifleToSidearm,
    SpeedReloadDetachableMagDropEmpty,
    TacticalReloadRetainPartialMag,
    ClearComplexDoubleFeedOrBoltOverride,
    LooseRoundReloadPerCartridge,
    BeltFedMgLinkReload,

    // §4.3 Movement, Posture & Physical Interaction
    FiveFootTacticalStep,
    DropProneImmediate,
    StandUpFromKneelingCrouch,
    TacticalBound2To3m,
    StandUpFromProne,
    VaultLowObstacleOrWindow,
    OpenUnlockedDoorOrHatch,
    BreachMechanicalKickRamDoor,
    MountOrDismountVehicle,
    RetrieveOrStowPackItem,

    // §4.4 Cognitive, Perceptual & Communication Actions
    QuickVisualGlanceReactionCheck,
    TacticalCalloutShortCommand,
    ActiveScanRoomOrSectorSweep,
    AssessTacticalWeaknessAnalyzeTarget,
    ComplexCommsRelayCoordTransmission,
    EmergencyFieldTriageTourniquet,
    ImprovisedGadgetCyberdeckReset,
}

/// Metadata entry for a canonical action in §4.1–§4.4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalActionSpec {
    pub id: CanonicalActionId,
    pub category: CatalogActionCategory,
    pub name: &'static str,
    pub base_tc_decide: i32,
    pub base_tc_act: i32,
    pub base_total_tc: i32,
    pub min_floor_ticks: i32,
    pub governing_notes: &'static str,
}

/// Full canonical 37-entry Reference Catalog of Base Action Time Costs (§4.1–§4.4 of `action_economy_and_ooda.md`).
pub const CANONICAL_ACTION_CATALOG: &[CanonicalActionSpec] = &[
    // §4.1 Combat Actions (Attacks & Defenses)
    CanonicalActionSpec {
        id: CanonicalActionId::SnapShotPointBlank,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Snap Shot / Point-Blank Reflex Strike",
        base_tc_decide: 2,
        base_tc_act: 5,
        base_total_tc: 7,
        min_floor_ticks: 2,
        governing_notes: "SPD, DEX, ITU; Pistols / Blades; -2 precision penalty",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::StandardMeleeStrikeLight,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Standard Melee Strike (Balanced / Light)",
        base_tc_decide: 3,
        base_tc_act: 6,
        base_total_tc: 9,
        min_floor_ticks: 2,
        governing_notes: "SPD, KIN; Melee Combat; modified by Weapon Balance (B)",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::StandardRangedFire,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Standard Ranged Fire (Combat Sight Picture)",
        base_tc_decide: 3,
        base_tc_act: 7,
        base_total_tc: 10,
        min_floor_ticks: 2,
        governing_notes: "DEX, SIG, FOC; Pistols / Long Guns",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::DefensiveGuardParryStance,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Defensive Guard / Active Parry Stance",
        base_tc_decide: 3,
        base_tc_act: 6,
        base_total_tc: 9,
        min_floor_ticks: 2,
        governing_notes: "KIN, INS, SPD; Melee / Martial Arts; boosts Guard Pool",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::TacticalManeuverProbeFeint,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Tactical Maneuver / Probe / Feint",
        base_tc_decide: 4,
        base_tc_act: 7,
        base_total_tc: 11,
        min_floor_ticks: 2,
        governing_notes: "INS, CRE, KIN; Tactician / Melee; +1 defensive parry margin",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::AimedShotSingle,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Aimed Shot (Deliberate Single Fire)",
        base_tc_decide: 5,
        base_tc_act: 7,
        base_total_tc: 12,
        min_floor_ticks: 2,
        governing_notes: "SIG, FOC, DEX; Precision Marksmanship; -2 to -4 to TN",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::HeavyPowerStrikeCleave,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Heavy Power Strike / Cleave",
        base_tc_decide: 3,
        base_tc_act: 11,
        base_total_tc: 14,
        min_floor_ticks: 2,
        governing_notes: "FOR, SPD, MAS; Heavy Weapons; +2 Wound Shock on hit",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::SuppressiveFireBurstZone,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Suppressive Fire (Burst / Automatic Zone)",
        base_tc_decide: 3,
        base_tc_act: 9,
        base_total_tc: 12,
        min_floor_ticks: 2,
        governing_notes: "FOR, DIS, FOC; Fully Automatic / Heavy Weapons",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::ActiveDodgeDiveForCover,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Active Dodge / Dive for Cover (Interrupt)",
        base_tc_decide: 2,
        base_tc_act: 6,
        base_total_tc: 8,
        min_floor_ticks: 2,
        governing_notes: "ITU, SPD, KIN; Evasion; advances actor's Current Tick by +8",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::OverwatchTriggeredRelease,
        category: CatalogActionCategory::CombatAttackOrDefense,
        name: "Overwatch / Held Trigger Release",
        base_tc_decide: 1,
        base_tc_act: 2,
        base_total_tc: 3,
        min_floor_ticks: 1,
        governing_notes: "FOC, ITU, SPD; fires when trigger condition is met",
    },
    // §4.2 Weapon Handling, Reloads & Malfunctions
    CanonicalActionSpec {
        id: CanonicalActionId::DropWeaponOrItem,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Drop Weapon / Release Held Item",
        base_tc_decide: 1,
        base_tc_act: 0,
        base_total_tc: 1,
        min_floor_ticks: 1,
        governing_notes: "D_cog; Free reflex; 0 Ticks if combined with drawing sidearm",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::SwitchFireSelectorOrOptic,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Switch Fire Selector / Toggle Optic Magnification",
        base_tc_decide: 1,
        base_tc_act: 1,
        base_total_tc: 2,
        min_floor_ticks: 1,
        governing_notes: "DEX; Firearms N >= 3 reduces to 1 Tick",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::ClearSimpleMalfunctionTapRack,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Clear Simple Malfunction (Tap-Rack Drill)",
        base_tc_decide: 2,
        base_tc_act: 3,
        base_total_tc: 5,
        min_floor_ticks: 2,
        governing_notes: "DEX, TAC; Pistols / Long Guns; -1 Tick at N >= 3, -2 at N >= 5",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::DrawHolsteredSidearmOrBlade,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Draw Holstered Sidearm / Sheathed Blade",
        base_tc_decide: 1,
        base_tc_act: 3,
        base_total_tc: 4,
        min_floor_ticks: 2,
        governing_notes: "SPD, DEX; Pistols / Blades; -1 Tick at N >= 3, -2 at N >= 5 (floor 2)",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::SlingTransitionRifleToSidearm,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Sling Transition (Drop Primary on Sling -> Draw Sidearm)",
        base_tc_decide: 1,
        base_tc_act: 3,
        base_total_tc: 4,
        min_floor_ticks: 2,
        governing_notes: "SPD, DEX, KIN; requires tactical sling; -1 Tick at N >= 4",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::SpeedReloadDetachableMagDropEmpty,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Speed Reload — Detachable Box Magazine (Drop Empty)",
        base_tc_decide: 1,
        base_tc_act: 5,
        base_total_tc: 6,
        min_floor_ticks: 3,
        governing_notes: "DEX, SPD, TAC; -1 Tick at N >= 3, -2 at N >= 5 (floor 3 Ticks = 1.5s)",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::TacticalReloadRetainPartialMag,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Tactical Reload — Retain Partial Magazine in Pouch",
        base_tc_decide: 2,
        base_tc_act: 7,
        base_total_tc: 9,
        min_floor_ticks: 3,
        governing_notes: "DEX, TAC; -1 Tick at N >= 3, -2 at N >= 5",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::ClearComplexDoubleFeedOrBoltOverride,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Clear Complex Malfunction (Double-Feed / Bolt Override)",
        base_tc_decide: 3,
        base_tc_act: 9,
        base_total_tc: 12,
        min_floor_ticks: 4,
        governing_notes: "DEX, TAC, FOC; Armourer / Firearms; requires tool or mag strip",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::LooseRoundReloadPerCartridge,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Revolver / Tube Shotgun Reload (Loose Rounds, per cartridge)",
        base_tc_decide: 1,
        base_tc_act: 2,
        base_total_tc: 3,
        min_floor_ticks: 2,
        governing_notes: "+2 Ticks per additional shell; Speedloader / Violin-load reduces to 6 Ticks total",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::BeltFedMgLinkReload,
        category: CatalogActionCategory::WeaponHandlingAndReload,
        name: "Belt-Fed Machine Gun / Heavy Weapon Link Reload",
        base_tc_decide: 3,
        base_tc_act: 13,
        base_total_tc: 16,
        min_floor_ticks: 6,
        governing_notes: "DEX, FOR; Heavy Weapons; Dedicated Assistant Gunner reduces TC_act by -6 Ticks",
    },
    // §4.3 Movement, Posture & Physical Interaction
    CanonicalActionSpec {
        id: CanonicalActionId::FiveFootTacticalStep,
        category: CatalogActionCategory::MovementAndPosture,
        name: "5-Foot Tactical Step / Shift Weight (<= 1.5 m)",
        base_tc_decide: 1,
        base_tc_act: 2,
        base_total_tc: 3,
        min_floor_ticks: 2,
        governing_notes: "KIN, SPD; Balance & Footwork; can combine with melee stance",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::DropProneImmediate,
        category: CatalogActionCategory::MovementAndPosture,
        name: "Drop Prone (Immediate)",
        base_tc_decide: 1,
        base_tc_act: 2,
        base_total_tc: 3,
        min_floor_ticks: 2,
        governing_notes: "SPD, KIN; Acrobatics N >= 3 reduces to 2 Ticks",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::StandUpFromKneelingCrouch,
        category: CatalogActionCategory::MovementAndPosture,
        name: "Stand Up from Kneeling / Crouch",
        base_tc_decide: 1,
        base_tc_act: 3,
        base_total_tc: 4,
        min_floor_ticks: 2,
        governing_notes: "SPD, KIN; penalized by Encumbrance Tier",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::TacticalBound2To3m,
        category: CatalogActionCategory::MovementAndPosture,
        name: "Tactical Bound / Dash Between Cover (2–3 m)",
        base_tc_decide: 1,
        base_tc_act: 4,
        base_total_tc: 5,
        min_floor_ticks: 2,
        governing_notes: "SPD, FIT; Running & Sprinting; scales with SPD",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::StandUpFromProne,
        category: CatalogActionCategory::MovementAndPosture,
        name: "Stand Up from Prone",
        base_tc_decide: 1,
        base_tc_act: 5,
        base_total_tc: 6,
        min_floor_ticks: 3,
        governing_notes: "SPD, KIN, FOR; Acrobatics / Martial Arts N >= 3 reduces by -2 Ticks",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::VaultLowObstacleOrWindow,
        category: CatalogActionCategory::MovementAndPosture,
        name: "Vault Low Obstacle / Window Breach",
        base_tc_decide: 1,
        base_tc_act: 5,
        base_total_tc: 6,
        min_floor_ticks: 2,
        governing_notes: "SPD, KIN, FOR; Climbing / Parkour",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::OpenUnlockedDoorOrHatch,
        category: CatalogActionCategory::MovementAndPosture,
        name: "Open Unlocked Door / Manipulate Hatch Lever",
        base_tc_decide: 1,
        base_tc_act: 3,
        base_total_tc: 4,
        min_floor_ticks: 2,
        governing_notes: "DEX, FOR",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::BreachMechanicalKickRamDoor,
        category: CatalogActionCategory::MovementAndPosture,
        name: "Breach (Mechanical Kick / Battering Ram on Door)",
        base_tc_decide: 2,
        base_tc_act: 6,
        base_total_tc: 8,
        min_floor_ticks: 3,
        governing_notes: "FOR, MAS, SPD; Strength & Lifting / Demolitions",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::MountOrDismountVehicle,
        category: CatalogActionCategory::MovementAndPosture,
        name: "Mount / Dismount Vehicle or Saddle",
        base_tc_decide: 1,
        base_tc_act: 5,
        base_total_tc: 6,
        min_floor_ticks: 2,
        governing_notes: "KIN, SPD; Riding / Driving N >= 3 reduces by -2 Ticks",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::RetrieveOrStowPackItem,
        category: CatalogActionCategory::MovementAndPosture,
        name: "Retrieve / Stow Item from Backpack or Load-Bearing Rig",
        base_tc_decide: 2,
        base_tc_act: 6,
        base_total_tc: 8,
        min_floor_ticks: 2,
        governing_notes: "DEX, TAC; Chest-rig / quick-release pouch reduces to 4 Ticks",
    },
    // §4.4 Cognitive, Perceptual & Communication Actions
    CanonicalActionSpec {
        id: CanonicalActionId::QuickVisualGlanceReactionCheck,
        category: CatalogActionCategory::CognitivePerceptualAndComms,
        name: "Quick Visual Glance / Threat Reaction Check",
        base_tc_decide: 2,
        base_tc_act: 0,
        base_total_tc: 2,
        min_floor_ticks: 1,
        governing_notes: "SIG, ITU, D_cog; Alertness; reduced to 1 Tick if D_cog >= 14",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::TacticalCalloutShortCommand,
        category: CatalogActionCategory::CognitivePerceptualAndComms,
        name: "Tactical Callout / Short Verbal Command (<= 5 words)",
        base_tc_decide: 2,
        base_tc_act: 1,
        base_total_tc: 3,
        min_floor_ticks: 1,
        governing_notes: "D_cog, PRE, ELO; Leadership; 1 Tick if standard squad brevity code",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::ActiveScanRoomOrSectorSweep,
        category: CatalogActionCategory::CognitivePerceptualAndComms,
        name: "Active Scan (Deliberate Room / Sector Sweep)",
        base_tc_decide: 4,
        base_tc_act: 2,
        base_total_tc: 6,
        min_floor_ticks: 2,
        governing_notes: "SIG, HEA, FOC; Surveillance & Reconnaissance",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::AssessTacticalWeaknessAnalyzeTarget,
        category: CatalogActionCategory::CognitivePerceptualAndComms,
        name: "Assess Tactical Weakness / Analyze Target Pattern",
        base_tc_decide: 5,
        base_tc_act: 1,
        base_total_tc: 6,
        min_floor_ticks: 2,
        governing_notes: "ANA, INS, FOC; Tactician; compressed heavily by high D_cog",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::ComplexCommsRelayCoordTransmission,
        category: CatalogActionCategory::CognitivePerceptualAndComms,
        name: "Complex Comms Relay / Coordinate Data Transmission",
        base_tc_decide: 4,
        base_tc_act: 4,
        base_total_tc: 8,
        min_floor_ticks: 2,
        governing_notes: "ANA, MEM, ELO; Electronic Warfare / Comms",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::EmergencyFieldTriageTourniquet,
        category: CatalogActionCategory::CognitivePerceptualAndComms,
        name: "Emergency Field Triage / Apply Combat Tourniquet",
        base_tc_decide: 3,
        base_tc_act: 9,
        base_total_tc: 12,
        min_floor_ticks: 4,
        governing_notes: "ANA, DEX, FOC; First Aid / Paramedic; -2 Ticks at N >= 4",
    },
    CanonicalActionSpec {
        id: CanonicalActionId::ImprovisedGadgetCyberdeckReset,
        category: CatalogActionCategory::CognitivePerceptualAndComms,
        name: "Improvised Gadget / Cyberdeck Quick-Deploy or Reset",
        base_tc_decide: 4,
        base_tc_act: 4,
        base_total_tc: 8,
        min_floor_ticks: 2,
        governing_notes: "CRE, ANA, DEX; Cybersecurity / Electronics",
    },
];

/// Look up a canonical action specification by its `CanonicalActionId`.
#[must_use]
pub fn lookup_canonical_action(id: CanonicalActionId) -> &'static CanonicalActionSpec {
    CANONICAL_ACTION_CATALOG
        .iter()
        .find(|spec| spec.id == id)
        .expect("All CanonicalActionId variants exist in CANONICAL_ACTION_CATALOG")
}

/// Movement gait during Stealth & Infiltration (§6.1, calibrated with `movement_vehicles_and_environment.md` §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StealthGait {
    /// Stationary & Camouflaged (`0 m/Tick`, `+0` penalty, benefits from Camouflage & Concealment)
    StationaryCamouflaged,
    /// Stealth Creep / Crouch-Walk (`18%` of $V_{\text{sprint}}$, `~0.34 m/Tick` at `SPD 10`, `+0` movement penalty)
    StealthCreep,
    /// Combat Glide / Tactical Walk (`35%` of $V_{\text{sprint}}$, `~0.65 m/Tick` at `SPD 10`, `+4` $\Delta_{\text{ext}}$ penalty to Stealth)
    CombatGlide,
    /// Bounding Run / All-Out Sprint (`65–100%` of $V_{\text{sprint}}$, `1.22–1.88 m/Tick` at `SPD 10`, `+8` $\Delta_{\text{ext}}$ penalty to Stealth)
    RunningDash,
}

impl StealthGait {
    /// External modifier ($\Delta_{\text{ext}}$) imposed on the infiltrator's Stealth roll (§6.1).
    #[must_use]
    pub const fn stealth_movement_delta_ext(self) -> i32 {
        match self {
            Self::StationaryCamouflaged | Self::StealthCreep => 0,
            Self::CombatGlide => 4,
            Self::RunningDash => 8,
        }
    }
}

/// Sensory channel used by an observer in a Multi-Sensory Perception Contest (§6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SensoryChannel {
    /// Visual Detection: `SIG / FOC` (`Alertness` or `Surveillance & Reconnaissance`)
    VisualSigFoc,
    /// Auditory Detection: `HEA / FOC` (`Alertness`)
    AuditoryHeaFoc,
    /// Olfactory / Chemical Detection: `OLF / INS` (`Alertness` or `Tracking`)
    OlfactoryOlfIns,
    /// Intuition / Sixth Sense ("Hair on the Back of the Neck"): `ITU / INS` (`Alertness`)
    SixthSenseItuIns,
}

/// Illumination and atmospheric visibility conditions (§6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IlluminationCondition {
    /// Bright Daylight / Well-Lit Interior (`+0` visual $\Delta_{\text{ext}}$)
    BrightDaylight,
    /// Dim Light / Dusk / Urban Shadows (`+2` visual $\Delta_{\text{ext}}$)
    DimLightDuskShadows,
    /// Moonlight / Starlight (`+4` visual $\Delta_{\text{ext}}$; `+1` with Gen-3+ NVG)
    MoonlightStarlight,
    /// Heavy Fog / Rain / Dense Smoke (`+6` visual, `+2` auditory $\Delta_{\text{ext}}$; `+1` with Thermal/FLIR)
    HeavyFogRainSmoke,
    /// Pitch Black / Total Darkness (`+10` visual $\Delta_{\text{ext}}$; `+0` with Thermal/FLIR)
    PitchBlackTotalDarkness,
}

/// Electro-optical countermeasure worn or mounted by the observer (§6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VisualCountermeasure {
    None,
    /// Gen-3+ Night Vision Goggles (reduces `MoonlightStarlight` penalty to `+1`)
    Gen3PlusNvg,
    /// Thermal / FLIR Imaging (reduces `HeavyFogRainSmoke` to `+1` and `PitchBlackTotalDarkness` to `+0` unless target is thermally shielded)
    ThermalFlir,
}

/// Computed modifiers for a Contested Stealth vs. Multi-Sensory Perception exchange (§6.1–§6.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StealthPerceptionModifiers {
    pub infiltrator_controller_c: i32,
    pub infiltrator_delta_ext: i32,
    pub observer_channel: SensoryChannel,
    pub observer_controller_c: i32,
    pub observer_delta_ext: i32,
}

/// Compute the canonical Stealth (`KIN/ITU`) and Observer Perception (`SIG/FOC`, `HEA/FOC`, `OLF/INS`, `ITU/INS`)
/// controllers and $\Delta_{\text{ext}}$ modifiers (§6.1–§6.2).
#[must_use]
pub fn compute_stealth_and_perception_modifiers(
    infiltrator_kin: i32,
    infiltrator_itu: i32,
    infiltrator_gait: StealthGait,
    infiltrator_encumbrance: EncumbranceTier,
    infiltrator_camouflage_bonus_delta_ext: i32,
    infiltrator_thermally_shielded: bool,
    observer_channel: SensoryChannel,
    observer_sig: i32,
    observer_hea: i32,
    observer_olf: i32,
    observer_itu: i32,
    observer_foc: i32,
    observer_ins: i32,
    illumination: IlluminationCondition,
    countermeasure: VisualCountermeasure,
) -> StealthPerceptionModifiers {
    let infiltrator_controller_c =
        (infiltrator_kin.clamp(1, 20) + infiltrator_itu.clamp(1, 20)).div_euclid(2);
    let cam_mod = if infiltrator_gait == StealthGait::StationaryCamouflaged {
        infiltrator_camouflage_bonus_delta_ext
    } else {
        0
    };
    let infiltrator_delta_ext = infiltrator_gait.stealth_movement_delta_ext()
        + infiltrator_encumbrance.stealth_penalty_delta_ext()
        + cam_mod;

    let observer_controller_c = match observer_channel {
        SensoryChannel::VisualSigFoc => {
            (observer_sig.clamp(1, 20) + observer_foc.clamp(1, 20)).div_euclid(2)
        }
        SensoryChannel::AuditoryHeaFoc => {
            (observer_hea.clamp(1, 20) + observer_foc.clamp(1, 20)).div_euclid(2)
        }
        SensoryChannel::OlfactoryOlfIns => {
            (observer_olf.clamp(1, 20) + observer_ins.clamp(1, 20)).div_euclid(2)
        }
        SensoryChannel::SixthSenseItuIns => {
            (observer_itu.clamp(1, 20) + observer_ins.clamp(1, 20)).div_euclid(2)
        }
    };

    let observer_delta_ext = match observer_channel {
        SensoryChannel::VisualSigFoc => match (illumination, countermeasure) {
            (IlluminationCondition::BrightDaylight, _) => 0,
            (IlluminationCondition::DimLightDuskShadows, VisualCountermeasure::None) => 2,
            (IlluminationCondition::DimLightDuskShadows, _) => 0,
            (IlluminationCondition::MoonlightStarlight, VisualCountermeasure::Gen3PlusNvg) => 1,
            (IlluminationCondition::MoonlightStarlight, VisualCountermeasure::ThermalFlir)
                if !infiltrator_thermally_shielded =>
            {
                0
            }
            (IlluminationCondition::MoonlightStarlight, _) => 4,
            (IlluminationCondition::HeavyFogRainSmoke, VisualCountermeasure::ThermalFlir)
                if !infiltrator_thermally_shielded =>
            {
                1
            }
            (IlluminationCondition::HeavyFogRainSmoke, _) => 6,
            (IlluminationCondition::PitchBlackTotalDarkness, VisualCountermeasure::ThermalFlir)
                if !infiltrator_thermally_shielded =>
            {
                0
            }
            (IlluminationCondition::PitchBlackTotalDarkness, VisualCountermeasure::Gen3PlusNvg) => {
                4
            }
            (IlluminationCondition::PitchBlackTotalDarkness, _) => 10,
        },
        SensoryChannel::AuditoryHeaFoc => match illumination {
            IlluminationCondition::HeavyFogRainSmoke => 2,
            _ => 0,
        },
        SensoryChannel::OlfactoryOlfIns | SensoryChannel::SixthSenseItuIns => 0,
    };

    StealthPerceptionModifiers {
        infiltrator_controller_c,
        infiltrator_delta_ext,
        observer_channel,
        observer_controller_c,
        observer_delta_ext,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_component_action_cost_and_reflex_compression() {
        // Worked check from §4.1 & §5.2-5.3:
        // Assess Tactical Weakness: base_tc_decide = 5, base_tc_act = 1 (Total 6).
        // Character with D_cog = 16 -> delta_tc_decide = -floor((16-10)/2) = -3.
        // Skill level N = 4 -> reflex compression = -1.
        // Effective TC_decide = max(1, 5 - 3 - 1) = 1.
        // Effective TC_act = 1. Total = 2 ticks!
        let params = ActionTimeFactorParams::default();
        let cost = evaluate_combat_action_cost(5, 1, 16, 4, true, 0, 0, 2, &params);
        assert_eq!(cost.delta_tc_decide, -3);
        assert_eq!(cost.skill_reflex_compression, 1);
        assert_eq!(cost.effective_tc_decide, 1);
        assert_eq!(cost.effective_tc_act, 1);
        assert_eq!(cost.effective_total_tc, 2);
    }

    #[test]
    fn test_encumbrance_and_concussed_action_time_factor() {
        let params = ActionTimeFactorParams {
            fatigue_level: 1,
            wound_shock_index: 1.0,
            encumbrance_tier: EncumbranceTier::Tier4Heavy,
            status_time_penalty: 0.25,
            is_concussed_or_dazed: true,
        };
        // Physical: 1.0 + 0.10 + 0.25 + 0.50 (Tier 4) + 0.25 (status) = 2.10
        assert!((params.physical_action_time_factor() - 2.10).abs() < 1e-6);
        // Cognitive (ignores physical encumbrance): 1.0 + 0.10 + 0.25 + 0.25 = 1.60
        assert!((params.cognitive_action_time_factor() - 1.60).abs() < 1e-6);
        // Concussed reduces D_cog by -4
        assert_eq!(params.effective_d_cog(14), 10);
    }

    #[test]
    fn test_starting_tick_and_interrupt_spoiling() {
        // OODA = 12, Awareness Margin = +2 -> Tick_start = max(1, 15 - 12 - 2) = 1
        assert_eq!(compute_starting_tick(12, 2, 0, 14, 12), 1);
        // Surprised (+10 ticks), D_cog = 14 (-2), DIS = 12 (-1) -> +7 freeze -> 1 + 7 = 8
        assert_eq!(compute_starting_tick(12, 2, 10, 14, 12), 8);

        // Heavy Power Strike (TC = 3 + 11 = 14) declared at Tick 2 -> completes at Tick 16.
        // Snap Shot lands at Tick 9 and causes Wound Shock (+2 delay) -> spoils the Heavy Strike!
        let params = ActionTimeFactorParams::default();
        let heavy_cost = evaluate_combat_action_cost(3, 11, 10, 2, true, 0, 0, 2, &params);
        let interrupt = evaluate_action_interrupt(2, &heavy_cost, 9, true, 2);
        assert!(interrupt.action_spoiled);
        assert!(interrupt.struck_during_wind_up);
        assert_eq!(interrupt.resulting_actor_tick, 11);
    }

    #[test]
    fn test_canonical_action_catalog_and_stealth_perception() {
        assert_eq!(CANONICAL_ACTION_CATALOG.len(), 37);
        let speed_reload = lookup_canonical_action(CanonicalActionId::SpeedReloadDetachableMagDropEmpty);
        assert_eq!(speed_reload.base_tc_decide, 1);
        assert_eq!(speed_reload.base_tc_act, 5);
        assert_eq!(speed_reload.base_total_tc, 6);

        let stealth = compute_stealth_and_perception_modifiers(
            14,
            12,
            StealthGait::CombatGlide,
            EncumbranceTier::Tier2Light,
            0,
            false,
            SensoryChannel::VisualSigFoc,
            13,
            11,
            10,
            12,
            13,
            11,
            IlluminationCondition::MoonlightStarlight,
            VisualCountermeasure::Gen3PlusNvg,
        );
        // Infiltrator: floor((14+12)/2) = 13, Glide (+4) + Tier 2 (+2) = +6
        assert_eq!(stealth.infiltrator_controller_c, 13);
        assert_eq!(stealth.infiltrator_delta_ext, 6);
        // Observer: floor((13+13)/2) = 13, Moonlight with Gen3+ NVG = +1
        assert_eq!(stealth.observer_controller_c, 13);
        assert_eq!(stealth.observer_delta_ext, 1);
    }
}
