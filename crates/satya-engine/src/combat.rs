//! # Tactical Triad Extensions, Melee Combat, Ranged Firepower, Armor & Reliability (`05-combat`)
//!
//! Canonical Rust implementation of `docs/ttrpg/05-combat/`:
//! - **`tactical_triad_and_pools.md`**: Decision Tick Modifier ($\Delta\text{TC}_{\text{decide}} = -\lfloor(D_{\text{cog}}-10)/2\rfloor$)
//!   and Exertion Mark accumulation against Fatigue Threshold ($\text{FT} = \text{FIT} + \lfloor\text{DIS}/2\rfloor$).
//! - **`melee_combat.md`**: Starting Combat Modes (`§3.3`), Melee Weapon Catalog (`§4`), Reach Advantage &
//!   Clinch Polarity Flip (`§4`), Contested Melee Exchanges (`§1`), Counter-Hits & Seizing the Initiative (`§2.2`),
//!   Interrupting Telegraphed Strikes (`§1.2`), and Kuzushi Throws vs. Root (`§2.3`, `§5.2`).
//! - **`ranged_combat_and_armor.md`**: Ranged Fire Modes (`§1.1`), Recoil Bracing ($F \ge R$) & Muzzle Climb
//!   Dispersion (`§1.2`), Suppressive Fire Morale Shock ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$, `§1.3`),
//!   Armor Profiles Catalog (`§2.1`), Penetration (`PR`) vs. Behind-Armor Blunt Trauma (`BABT`, `§2.2`),
//!   and Weapon Reliability (`Rel` in `-4..=+4`) & Malfunction Mechanics (`§3.1–§3.2`).

use serde::{Deserialize, Serialize};

use crate::dice_probabilities::{CascadeOutcomeKind, CascadeRollOutcome};
use crate::rules_book::{TacticalPoolEnvelope, TacticalTriadEvaluation, evaluate_tactical_triad};

// ============================================================================
// Tactical Triad Extensions (Decision Tick Modifier & Exertion Progression)
// ============================================================================

/// Computes the **Cognitive Decision Tick Modifier ($\Delta\text{TC}_{\text{decide}}$)** from
/// `tactical_triad_and_pools.md` §1.2:
/// - For complex tactical reads, feints, or multi-target prioritization (`is_complex_or_feint == true`):
///   $$\Delta\text{TC}_{\text{decide}} = -\left\lfloor \frac{D_{\text{cog}} - 10}{2} \right\rfloor$$
/// - For simple drilled reflex actions (`is_complex_or_feint == false`): `0`.
pub fn compute_decision_tick_modifier(d_cog: i32, is_complex_or_feint: bool) -> i32 {
    if is_complex_or_feint {
        -((d_cog - 10).div_euclid(2))
    } else {
        0
    }
}

/// Evaluates a `Tempo + Control + Force` allocation including the optional `is_complex_or_feint`
/// Cognitive Decision Tick Modifier $\Delta\text{TC}_{\text{decide}}$ (`tactical_triad_and_pools.md` §1.2).
pub fn evaluate_tactical_triad_with_decision(
    envelope: &TacticalPoolEnvelope,
    base_weapon_tc: i32,
    tempo: i32,
    control: i32,
    force: i32,
    is_complex_or_feint: bool,
) -> TacticalTriadEvaluation {
    let mut eval = evaluate_tactical_triad(envelope, base_weapon_tc, tempo, control, force);
    let delta_decide = compute_decision_tick_modifier(envelope.d_cog, is_complex_or_feint);
    let scaled_base_tc = (base_weapon_tc as f64 * envelope.action_time_factor).round() as i32;
    eval.effective_tc =
        (scaled_base_tc - (tempo - envelope.neutral_baseline_b) + delta_decide).max(2);
    eval
}

/// Result of accumulating Exertion Marks after a combat exchange (`tactical_triad_and_pools.md` §1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExertionProgressionResult {
    /// Updated Exertion Marks remaining after checking against `fatigue_threshold_ft`.
    pub remaining_exertion_marks: i32,
    /// Updated systemic Fatigue Level (`-2` $P_{\text{eff}}$, `+2` $\Delta_{\text{int}}$, `+10%` `action_time_factor` per level).
    pub new_fatigue_level: i32,
    /// Number of new Fatigue Levels gained on this exchange.
    pub fatigue_levels_gained: i32,
}

/// Accumulates `exertion_marks_gained` against the combatant's Fatigue Threshold
/// $\text{FT} = \text{FIT} + \lfloor\text{DIS}/2\rfloor$ (`tactical_triad_and_pools.md` §1.3).
pub fn accumulate_exertion_marks(
    current_exertion_marks: i32,
    exertion_marks_gained: i32,
    current_fatigue_level: i32,
    fatigue_threshold_ft: i32,
) -> ExertionProgressionResult {
    let ft = fatigue_threshold_ft.max(1);
    let total_marks = (current_exertion_marks.max(0) + exertion_marks_gained.max(0)).max(0);
    let fatigue_levels_gained = total_marks.div_euclid(ft);
    let remaining_exertion_marks = total_marks.rem_euclid(ft);
    let new_fatigue_level = current_fatigue_level.max(0) + fatigue_levels_gained;

    ExertionProgressionResult {
        remaining_exertion_marks,
        new_fatigue_level,
        fatigue_levels_gained,
    }
}

// ============================================================================
// Melee Combat: Starting Modes, Weapon Catalog, Reach & Clinch, Exchanges
// ============================================================================

/// Starting Combat Engagement Mode declared when entering a combat scene (`melee_combat.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatEngagementMode {
    /// **Disengaging**: Prioritizes `Tempo` (footwork to break distance) + `Control` (warding off pursuit); `Force = 0`.
    Disengaging,
    /// **Defensive**: Prioritizes `Control >= 50%` of pool + moderate `Tempo`; looks to parry and win ticks via `Seizing the Initiative`.
    Defensive,
    /// **Opportunistic**: Balanced `Control` and `Tempo`; probes with light strikes while watching for openings.
    Opportunistic,
    /// **Offensive**: Prioritizes `Tempo + Force` with baseline `Control`; presses the attack to force defensive reactions.
    Offensive,
    /// **Aggressive**: Maximum `Force + Tempo`, minimal `Control`; all-out blitz or suppression to overwhelm quickly.
    Aggressive,
}

/// Canonical Melee Weapon & Unarmed Attack identifiers from `melee_combat.md` §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeleeWeaponId {
    /// **Unarmed (Jab / Low Kick / Clinch Knee)**: Reach `1`, Base TC `8`, Guard `+0`, Blunt (`1.0x POW`).
    UnarmedJabLowKickClinchKnee,
    /// **Unarmed (Haymaker / High Kick / Throw)**: Reach `1–2`, Base TC `10`, Guard `-1` (strike) / `+1` (grapple), Heavy Blunt (`1.25x POW`).
    UnarmedHaymakerHighKickThrow,
    /// **Combat Knife / Karambit / Shiv**: Reach `1` (Clinch), Base TC `7`, Guard `-1`, Piercing / Lacerating (`+2` Base Bleed).
    CombatKnifeKarambitShiv,
    /// **Expandable Baton / Tonfa / Club**: Reach `2`, Base TC `8`, Guard `+2 Control`, Blunt (`1.2x POW`), `+1` Control on Disarm/Bind.
    ExpandableBatonTonfaClub,
    /// **Machete / Wakizashi / Tactical Blade**: Reach `2–3`, Base TC `9`, Guard `+1 Control`, Lacerating / Impaling (`+4` Base Trauma).
    MacheteWakizashiTacticalBlade,
    /// **Two-Handed Staff / Spear / Riot Shield**: Reach `4`, Base TC `10–11`, Guard `+3 Control`, `+2` effective Force on blocks/sweeps, `+4` $\Delta_{\text{ext}}$ keep-out.
    TwoHandedStaffSpearRiotShield,
}

/// Canonical profile for a Melee Weapon or Unarmed Attack (`melee_combat.md` §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeleeWeaponProfile {
    /// Canonical weapon identifier.
    pub id: MeleeWeaponId,
    /// Display name from the §4 table.
    pub name: &'static str,
    /// Minimum effective Reach tier (`1..=4`).
    pub min_reach: i32,
    /// Maximum effective Reach tier (`1..=4`).
    pub max_reach: i32,
    /// Base Count-Up Tick Cost (`Base TC`).
    pub base_tc: i32,
    /// Passive Guard bonus added to `Control` when parrying/defending (`-1..=+3`).
    pub guard_bonus: i32,
    /// Multiplier applied to $\text{POW} \times (F / B)$ strike impact (`1.0`, `1.2`, `1.25`, etc.).
    pub pow_multiplier: f64,
    /// Flat bonus added to base trauma or bleed (`+2` knife bleed, `+4` tactical blade trauma).
    pub flat_trauma_or_bleed_bonus: i32,
    /// Bonus to effective `Force` on blocks and sweeps (`+2` for Staff/Spear/Riot Shield).
    pub block_sweep_force_bonus: i32,
    /// Whether this weapon imposes a `+4` $\Delta_{\text{ext}}$ keep-out penalty on shorter-reach attackers.
    pub has_keep_out_property: bool,
}

/// Returns the canonical 6-entry Melee Weapon & Unarmed Attack Catalog (`melee_combat.md` §4).
pub fn canonical_melee_weapon_catalog() -> Vec<MeleeWeaponProfile> {
    vec![
        MeleeWeaponProfile {
            id: MeleeWeaponId::UnarmedJabLowKickClinchKnee,
            name: "Unarmed (Jab / Low Kick / Clinch Knee)",
            min_reach: 1,
            max_reach: 1,
            base_tc: 8,
            guard_bonus: 0,
            pow_multiplier: 1.0,
            flat_trauma_or_bleed_bonus: 0,
            block_sweep_force_bonus: 0,
            has_keep_out_property: false,
        },
        MeleeWeaponProfile {
            id: MeleeWeaponId::UnarmedHaymakerHighKickThrow,
            name: "Unarmed (Haymaker / High Kick / Throw)",
            min_reach: 1,
            max_reach: 2,
            base_tc: 10,
            guard_bonus: -1,
            pow_multiplier: 1.25,
            flat_trauma_or_bleed_bonus: 0,
            block_sweep_force_bonus: 0,
            has_keep_out_property: false,
        },
        MeleeWeaponProfile {
            id: MeleeWeaponId::CombatKnifeKarambitShiv,
            name: "Combat Knife / Karambit / Shiv",
            min_reach: 1,
            max_reach: 1,
            base_tc: 7,
            guard_bonus: -1,
            pow_multiplier: 1.0,
            flat_trauma_or_bleed_bonus: 2,
            block_sweep_force_bonus: 0,
            has_keep_out_property: false,
        },
        MeleeWeaponProfile {
            id: MeleeWeaponId::ExpandableBatonTonfaClub,
            name: "Expandable Baton / Tonfa / Club",
            min_reach: 2,
            max_reach: 2,
            base_tc: 8,
            guard_bonus: 2,
            pow_multiplier: 1.2,
            flat_trauma_or_bleed_bonus: 0,
            block_sweep_force_bonus: 0,
            has_keep_out_property: false,
        },
        MeleeWeaponProfile {
            id: MeleeWeaponId::MacheteWakizashiTacticalBlade,
            name: "Machete / Wakizashi / Tactical Blade",
            min_reach: 2,
            max_reach: 3,
            base_tc: 9,
            guard_bonus: 1,
            pow_multiplier: 1.15,
            flat_trauma_or_bleed_bonus: 4,
            block_sweep_force_bonus: 0,
            has_keep_out_property: false,
        },
        MeleeWeaponProfile {
            id: MeleeWeaponId::TwoHandedStaffSpearRiotShield,
            name: "Two-Handed Staff / Spear / Riot Shield",
            min_reach: 4,
            max_reach: 4,
            base_tc: 10,
            guard_bonus: 3,
            pow_multiplier: 1.2,
            flat_trauma_or_bleed_bonus: 0,
            block_sweep_force_bonus: 2,
            has_keep_out_property: true,
        },
    ]
}

/// Computes the **Reach Advantage & Clinch Polarity Flip** external penalty $\Delta_{\text{ext}}$ (`melee_combat.md` §4):
/// - **At Standoff Distance (`!in_clinch`)**: A fighter with shorter Reach attacking a fighter with longer Reach
///   suffers `+2` $\Delta_{\text{ext}}$ per step of Reach disadvantage (`+4` minimum against a Reach 4 keep-out weapon)
///   *unless* they allocate higher `Tempo` (`attacker_tempo > defender_tempo`) to step inside the weapon's arc.
/// - **Inside the Clinch (`in_clinch`, Reach 1)**: The polarity flips! Long weapons (`attacker_reach >= 3`)
///   become crowded and suffer a `+4` $\Delta_{\text{ext}}$ penalty, while `Reach 1` weapons (knives, elbows, knees)
///   ignore reach penalties (`0`).
pub fn compute_reach_and_clinch_delta_ext(
    attacker_reach: i32,
    defender_reach: i32,
    in_clinch: bool,
    attacker_tempo: i32,
    defender_tempo: i32,
    defender_has_keep_out_weapon: bool,
) -> i32 {
    if in_clinch {
        if attacker_reach >= 3 { 4 } else { 0 }
    } else if attacker_reach < defender_reach && attacker_tempo <= defender_tempo {
        let base_penalty = 2 * (defender_reach - attacker_reach);
        if defender_has_keep_out_weapon {
            base_penalty.max(4)
        } else {
            base_penalty
        }
    } else {
        0
    }
}

/// Outcome of a Contested Melee Exchange (`melee_combat.md` §1, §2.2, §2.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeleeExchangeResolution {
    /// Defender's Effective Control including weapon guard ($C_{\text{def}} + \text{Guard}_{\text{def}}$).
    pub defender_effective_control: i32,
    /// Control Differential $\Delta_{\text{control}} = (C_{\text{def}} + \text{Guard}_{\text{def}}) - C_{\text{att}}$.
    pub delta_control: i32,
    /// External penalty $\Delta_{\text{ext}}$ imposed on the attacker by the defender's superior Control ($\max(0, \Delta_{\text{control}})$).
    pub control_delta_ext_on_attacker: i32,
    /// External penalty $\Delta_{\text{ext}}$ from Reach disadvantage or Clinch crowding.
    pub reach_clinch_delta_ext: i32,
    /// Whether the attacker's strike lands (`attacker_net_successes > 0`).
    pub strike_lands: bool,
    /// Whether the defender **Seizes the Initiative** (`attacker_net_successes <= 0` and $C_{\text{def}} > C_{\text{att}}$, `§2.2`).
    pub defender_seizes_initiative: bool,
    /// Recovery ticks added to the attacker's next action (or pool drain on next exchange) when the defender Seizes the Initiative:
    /// $\max(0, (C_{\text{def}} + \text{Guard}_{\text{def}}) - C_{\text{att}})$.
    pub initiative_seized_penalty: i32,
    /// Attacker's Leveraged Throw/Takedown Force ($F_{\text{att}} + \max(0, C_{\text{att}} - C_{\text{def}}) + \text{redirected\_blitz\_tempo}$).
    pub leveraged_throw_force: f64,
    /// Defender's Effective Root ($F_{\text{def}} + \text{MMF}_{\text{def}}$).
    pub defender_effective_root: f64,
    /// Whether a Kuzushi Throw / Sweep succeeds (`is_throw_or_sweep && strike_lands && leveraged_throw_force >= defender_effective_root`).
    pub kuzushi_throw_succeeded: bool,
    /// Recovery ticks imposed on a thrown/swept target (`6 ticks` to stand from Prone).
    pub prone_recovery_ticks: i32,
}

/// Evaluates a Contested Melee Exchange (`melee_combat.md` §1, §2.2, §2.3).
#[allow(clippy::too_many_arguments)]
pub fn resolve_contested_melee_exchange(
    attacker_tempo: i32,
    attacker_control: i32,
    attacker_force: i32,
    defender_tempo: i32,
    defender_control: i32,
    defender_force: i32,
    defender_guard_bonus: i32,
    defender_mmf: f64,
    attacker_reach: i32,
    defender_reach: i32,
    in_clinch: bool,
    defender_has_keep_out_weapon: bool,
    attacker_net_successes: i32,
    is_throw_or_sweep: bool,
    redirect_opponent_blitz_in_clinch: bool,
) -> MeleeExchangeResolution {
    let defender_effective_control = (defender_control + defender_guard_bonus).max(0);
    let delta_control = defender_effective_control - attacker_control;
    let control_delta_ext_on_attacker = delta_control.max(0);
    let reach_clinch_delta_ext = compute_reach_and_clinch_delta_ext(
        attacker_reach,
        defender_reach,
        in_clinch,
        attacker_tempo,
        defender_tempo,
        defender_has_keep_out_weapon,
    );

    let strike_lands = attacker_net_successes > 0;
    let defender_seizes_initiative = !strike_lands && defender_effective_control > attacker_control;
    let initiative_seized_penalty = if defender_seizes_initiative {
        defender_effective_control - attacker_control
    } else {
        0
    };

    let redirected_momentum = if is_throw_or_sweep && in_clinch && redirect_opponent_blitz_in_clinch
    {
        defender_tempo.max(0) as f64
    } else {
        0.0
    };
    let leveraged_throw_force = attacker_force as f64
        + (attacker_control - defender_effective_control).max(0) as f64
        + redirected_momentum;
    let defender_effective_root = defender_force as f64 + defender_mmf;

    let kuzushi_throw_succeeded =
        is_throw_or_sweep && strike_lands && leveraged_throw_force >= defender_effective_root;
    let prone_recovery_ticks = if kuzushi_throw_succeeded { 6 } else { 0 };

    MeleeExchangeResolution {
        defender_effective_control,
        delta_control,
        control_delta_ext_on_attacker,
        reach_clinch_delta_ext,
        strike_lands,
        defender_seizes_initiative,
        initiative_seized_penalty,
        leveraged_throw_force,
        defender_effective_root,
        kuzushi_throw_succeeded,
        prone_recovery_ticks,
    }
}

/// Checks whether a counter-attacker's strike interrupts an opponent's telegraphed attack before impact (`melee_combat.md` §1.2).
///
/// Returns `(interrupted, counter_land_tick)` where `counter_land_tick = counter_start_tick + counter_effective_tc`.
pub fn check_melee_interrupt_before_impact(
    telegraphed_land_tick: i32,
    counter_start_tick: i32,
    counter_effective_tc: i32,
    counter_inflicts_shock_or_knockback: bool,
) -> (bool, i32) {
    let counter_land_tick = counter_start_tick + counter_effective_tc;
    let interrupted =
        counter_land_tick < telegraphed_land_tick && counter_inflicts_shock_or_knockback;
    (interrupted, counter_land_tick)
}

// ============================================================================
// Ranged Combat: Fire Modes, Recoil Bracing & Suppression
// ============================================================================

/// Canonical Ranged Fire Mode identifiers (`ranged_combat_and_armor.md` §1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RangedFireModeId {
    /// **Single Snap Shot (Point-Shoot)**: Base TC `7`, Recoil `3–4`, prioritizes `Tempo` (`CQB`).
    SingleSnapShotPointShoot,
    /// **Deliberate Sighted / Aimed Shot**: Base TC `11–12`, Recoil `4–6`, prioritizes `Control` (called shots, weak spots).
    DeliberateSightedAimedShot,
    /// **Controlled Double-Tap / Short Burst**: Base TC `9`, Recoil `6–7`, balanced `Control + Force` (`+1` hit tier per `2` Net Successes).
    ControlledDoubleTapShortBurst,
    /// **Full-Auto Burst / Suppressive Fire**: Base TC `8–10`, Recoil `8–12`, prioritizes `Force` ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$).
    FullAutoBurstSuppressiveFire,
    /// **Bounding / Shooting on the Move**: Base TC `10`, `+3` unbraced recoil added to weapon `R`.
    BoundingShootingOnTheMove,
}

/// Canonical profile for a Ranged Fire Mode (`ranged_combat_and_armor.md` §1.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangedFireModeProfile {
    /// Canonical fire mode identifier.
    pub id: RangedFireModeId,
    /// Display name from the §1.1 table.
    pub name: &'static str,
    /// Minimum Base Count-Up Tick Cost (`Base TC`).
    pub min_base_tc: i32,
    /// Maximum Base Count-Up Tick Cost (`Base TC`).
    pub max_base_tc: i32,
    /// Typical minimum Recoil Rating (`R`).
    pub min_recoil_r: i32,
    /// Typical maximum Recoil Rating (`R`).
    pub max_recoil_r: i32,
    /// Extra unbraced recoil penalty added when shooting on the move (`+3` for `BoundingShootingOnTheMove`).
    pub movement_recoil_adder: i32,
}

/// Returns the canonical 5-entry Ranged Fire Mode catalog (`ranged_combat_and_armor.md` §1.1).
pub fn canonical_ranged_fire_modes() -> Vec<RangedFireModeProfile> {
    vec![
        RangedFireModeProfile {
            id: RangedFireModeId::SingleSnapShotPointShoot,
            name: "Single Snap Shot (Point-Shoot)",
            min_base_tc: 7,
            max_base_tc: 7,
            min_recoil_r: 3,
            max_recoil_r: 4,
            movement_recoil_adder: 0,
        },
        RangedFireModeProfile {
            id: RangedFireModeId::DeliberateSightedAimedShot,
            name: "Deliberate Sighted / Aimed Shot",
            min_base_tc: 11,
            max_base_tc: 12,
            min_recoil_r: 4,
            max_recoil_r: 6,
            movement_recoil_adder: 0,
        },
        RangedFireModeProfile {
            id: RangedFireModeId::ControlledDoubleTapShortBurst,
            name: "Controlled Double-Tap / Short Burst",
            min_base_tc: 9,
            max_base_tc: 9,
            min_recoil_r: 6,
            max_recoil_r: 7,
            movement_recoil_adder: 0,
        },
        RangedFireModeProfile {
            id: RangedFireModeId::FullAutoBurstSuppressiveFire,
            name: "Full-Auto Burst / Suppressive Fire",
            min_base_tc: 8,
            max_base_tc: 10,
            min_recoil_r: 8,
            max_recoil_r: 12,
            movement_recoil_adder: 0,
        },
        RangedFireModeProfile {
            id: RangedFireModeId::BoundingShootingOnTheMove,
            name: "Bounding / Shooting on the Move",
            min_base_tc: 10,
            max_base_tc: 10,
            min_recoil_r: 6,
            max_recoil_r: 10,
            movement_recoil_adder: 3,
        },
    ]
}

/// Evaluation of a ranged weapon triad allocation against Recoil (`R`) and Suppressive Fire rules (`ranged_combat_and_armor.md` §1.2–§1.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangedExchangeEvaluation {
    /// Effective Recoil demand $R_{\text{eff}} = R + \text{movement\_recoil\_adder}$.
    pub effective_recoil_r: i32,
    /// Whether the shooter's allocated `Force` fully braces the weapon ($F \ge R_{\text{eff}}$).
    pub recoil_fully_braced: bool,
    /// Unbraced recoil shortfall $\max(0, R_{\text{eff}} - F)$, applied directly as a muzzle-climb dispersion penalty to $\Delta_{\text{ext}}$.
    pub unbraced_dispersion_delta_ext: i32,
    /// Suppression Morale Shock $\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$ inflicted on targets in the beaten zone (when `is_suppressive_or_full_auto == true`).
    pub suppression_morale_delta: i32,
    /// Minimum `Control` a pinned target must allocate to return fire calmly without suffering the suppression penalty (`6`).
    pub calm_return_fire_control_threshold: i32,
}

/// Evaluates Recoil Bracing ($F \ge R$) and Suppressive Fire Morale Shock ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$)
/// from `ranged_combat_and_armor.md` §1.2–§1.3.
pub fn evaluate_ranged_recoil_and_suppression(
    _allocated_tempo: i32,
    _allocated_control: i32,
    allocated_force: i32,
    weapon_recoil_r: i32,
    is_bounding_on_move: bool,
    is_suppressive_or_full_auto: bool,
) -> RangedExchangeEvaluation {
    let effective_recoil_r = weapon_recoil_r.max(0) + if is_bounding_on_move { 3 } else { 0 };
    let unbraced_dispersion_delta_ext = (effective_recoil_r - allocated_force.max(0)).max(0);
    let recoil_fully_braced = unbraced_dispersion_delta_ext == 0;
    let suppression_morale_delta = if is_suppressive_or_full_auto {
        allocated_force.max(0).div_euclid(2)
    } else {
        0
    };

    RangedExchangeEvaluation {
        effective_recoil_r,
        recoil_fully_braced,
        unbraced_dispersion_delta_ext,
        suppression_morale_delta,
        calm_return_fire_control_threshold: 6,
    }
}

/// Evaluates Ranged Recoil Bracing and Suppressive Fire using the canonical 5-gait movement model
/// from `movement_vehicles_and_environment.md` §1.2 (`+0` Crawl/Creep, `+2` Combat Glide, `+4` Bounding Run).
pub fn evaluate_ranged_recoil_with_gait(
    allocated_tempo: i32,
    allocated_control: i32,
    allocated_force: i32,
    weapon_recoil_r: i32,
    movement_gait: crate::health_survival::CombatMovementGait,
    is_suppressive_or_full_auto: bool,
) -> RangedExchangeEvaluation {
    let effective_recoil_r = weapon_recoil_r.max(0) + movement_gait.ranged_recoil_adder();
    let unbraced_dispersion_delta_ext = (effective_recoil_r - allocated_force.max(0)).max(0);
    let recoil_fully_braced = unbraced_dispersion_delta_ext == 0;
    let suppression_morale_delta = if is_suppressive_or_full_auto {
        allocated_force.max(0).div_euclid(2)
    } else {
        0
    };
    let _ = (allocated_tempo, allocated_control);

    RangedExchangeEvaluation {
        effective_recoil_r,
        recoil_fully_braced,
        unbraced_dispersion_delta_ext,
        suppression_morale_delta,
        calm_return_fire_control_threshold: 6,
    }
}

// ============================================================================
// Armor Catalog & Penetration (PR) vs. Behind-Armor Blunt Trauma (BABT)
// ============================================================================

/// Canonical Armor Profile identifiers (`ranged_combat_and_armor.md` §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmorProfileId {
    /// **Heavy Leather / Synth-Weave Jacket**: `PR_Edged 2`, `PR_Ballistic 1`, `BTD 1`, `2.5 kg`.
    HeavyLeatherSynthWeaveJacket,
    /// **Concealable Soft Kevlar Vest (NIJ IIIA)**: `PR_Edged 2` (`1` vs. icepick/flechette), `PR_Ballistic 4`, `BTD 2`, `2.8 kg`.
    ConcealableSoftKevlarVestNij3a,
    /// **Tactical Plate Carrier (Level IV Ceramic)**: `PR_Edged 6`, `PR_Ballistic 8`, `BTD 5`, `8.5 kg`.
    TacticalPlateCarrierLevel4,
    /// **Ballistic Combat Helmet + Visor**: `PR_Edged 5`, `PR_Ballistic 4` (`3` visor), `BTD 4`, `1.6 kg`.
    BallisticCombatHelmetVisor,
    /// **Full Riot / Breacher Suit**: `PR_Edged 6`, `PR_Ballistic 5`, `BTD 6`, `16.0 kg`, `+6` Heat Retention.
    FullRiotBreacherSuit,
    /// **2035 "Valkyrie" Recon Exo-Carapace**: `PR_Edged 8`, `PR_Ballistic 9`, `BTD 8`, `18.5 kg` (`-20.0 kg` Exo Offset).
    ValkyrieReconExoCarapace2035,
}

/// Canonical Armor Profile from `ranged_combat_and_armor.md` §2.1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorProfile {
    /// Canonical armor identifier.
    pub id: ArmorProfileId,
    /// Display name from the §2.1 table.
    pub name: &'static str,
    /// Penetration Resistance against Edged/Stabbing attacks (`PR_Edged`).
    pub pr_edged: i32,
    /// Penetration Resistance against icepick/flechette thrusts (`1` for Soft Kevlar, else equals `pr_edged`).
    pub pr_icepick_flechette: i32,
    /// Penetration Resistance against Ballistic projectiles (`PR_Ballistic`).
    pub pr_ballistic: i32,
    /// Penetration Resistance of the visor/weak spot (`3` for Ballistic Helmet Visor, else equals `pr_ballistic`).
    pub pr_visor_weak_spot: i32,
    /// Blunt Trauma Dampening (`BTD`) absorbed before Behind-Armor Blunt Trauma (`BABT`) transfers to the wearer.
    pub blunt_trauma_dampening_btd: i32,
    /// Physical weight in kilograms.
    pub weight_kg: f64,
    /// Powered exoskeleton carry-load offset in kilograms (`20.0 kg` for Valkyrie Exo-Carapace, else `0.0`).
    pub exo_load_offset_kg: f64,
    /// Environmental Heat Retention penalty (`+6` for Full Riot/Breacher Suit, else `0`).
    pub heat_retention_delta: i32,
}

/// Returns the canonical 6-entry Armor Catalog (`ranged_combat_and_armor.md` §2.1).
pub fn canonical_armor_catalog() -> Vec<ArmorProfile> {
    vec![
        ArmorProfile {
            id: ArmorProfileId::HeavyLeatherSynthWeaveJacket,
            name: "Heavy Leather / Synth-Weave Jacket",
            pr_edged: 2,
            pr_icepick_flechette: 2,
            pr_ballistic: 1,
            pr_visor_weak_spot: 1,
            blunt_trauma_dampening_btd: 1,
            weight_kg: 2.5,
            exo_load_offset_kg: 0.0,
            heat_retention_delta: 0,
        },
        ArmorProfile {
            id: ArmorProfileId::ConcealableSoftKevlarVestNij3a,
            name: "Concealable Soft Kevlar Vest (NIJ IIIA)",
            pr_edged: 2,
            pr_icepick_flechette: 1,
            pr_ballistic: 4,
            pr_visor_weak_spot: 4,
            blunt_trauma_dampening_btd: 2,
            weight_kg: 2.8,
            exo_load_offset_kg: 0.0,
            heat_retention_delta: 0,
        },
        ArmorProfile {
            id: ArmorProfileId::TacticalPlateCarrierLevel4,
            name: "Tactical Plate Carrier (Level IV Ceramic)",
            pr_edged: 6,
            pr_icepick_flechette: 6,
            pr_ballistic: 8,
            pr_visor_weak_spot: 8,
            blunt_trauma_dampening_btd: 5,
            weight_kg: 8.5,
            exo_load_offset_kg: 0.0,
            heat_retention_delta: 0,
        },
        ArmorProfile {
            id: ArmorProfileId::BallisticCombatHelmetVisor,
            name: "Ballistic Combat Helmet + Visor",
            pr_edged: 5,
            pr_icepick_flechette: 5,
            pr_ballistic: 4,
            pr_visor_weak_spot: 3,
            blunt_trauma_dampening_btd: 4,
            weight_kg: 1.6,
            exo_load_offset_kg: 0.0,
            heat_retention_delta: 0,
        },
        ArmorProfile {
            id: ArmorProfileId::FullRiotBreacherSuit,
            name: "Full Riot / Breacher Suit",
            pr_edged: 6,
            pr_icepick_flechette: 6,
            pr_ballistic: 5,
            pr_visor_weak_spot: 5,
            blunt_trauma_dampening_btd: 6,
            weight_kg: 16.0,
            exo_load_offset_kg: 0.0,
            heat_retention_delta: 6,
        },
        ArmorProfile {
            id: ArmorProfileId::ValkyrieReconExoCarapace2035,
            name: "2035 \"Valkyrie\" Recon Exo-Carapace",
            pr_edged: 8,
            pr_icepick_flechette: 8,
            pr_ballistic: 9,
            pr_visor_weak_spot: 9,
            blunt_trauma_dampening_btd: 8,
            weight_kg: 18.5,
            exo_load_offset_kg: 20.0,
            heat_retention_delta: 0,
        },
    ]
}

/// Incoming weapon damage type for Armor `PR` vs. `BABT` resolution (`ranged_combat_and_armor.md` §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponDamageType {
    /// Standard Edged / Slashing / Stabbing weapon (`PR_Edged`).
    Edged,
    /// Narrow-point Icepick thrust or Flechette needle (`pr_icepick_flechette`).
    IcepickOrFlechette,
    /// Ballistic firearm projectile or high-velocity fragment (`PR_Ballistic`).
    Ballistic,
    /// Pure Blunt impact (`BABT = max(0, Raw Impact - BTD)`).
    Blunt,
}

/// Result of resolving an attack's Penetration (`Effective Pen`) and Impact against an [`ArmorProfile`] (`ranged_combat_and_armor.md` §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmorImpactOutcome {
    /// Effective Penetration $\text{Effective Pen} = \text{Base Pen} + \lfloor \max(0, \text{Net Successes}) / 2 \rfloor$.
    pub effective_pen: i32,
    /// Active Armor Penetration Resistance (`PR`) for the struck location and damage type.
    pub active_armor_pr: i32,
    /// Whether the attack penetrated the armor ($\text{Effective Pen} > \text{PR}$ for non-blunt attacks).
    pub penetrated: bool,
    /// Behind-Armor Blunt Trauma (`BABT`) inflicted when stopped by armor ($\max(0, \text{Raw Impact} - \text{BTD})$).
    pub babt_trauma: i32,
    /// Penetrating trauma inflicted when armor is overmatched ($\max(0, \text{Raw Impact} - \text{PR})$).
    pub penetrating_trauma: i32,
    /// Total effective trauma transferred to the target (`penetrating_trauma` if penetrated, else `babt_trauma`).
    pub total_effective_trauma: i32,
    /// Whether the wound causes internal/external bleeding (`true` when an Edged/Icepick/Ballistic attack penetrates).
    pub causes_bleed: bool,
    /// Armor `PR` ablation/degradation inflicted on the struck plate (`1..=2` on penetration, `0` when stopped).
    pub armor_pr_degradation: i32,
}

/// Resolves **Armor Penetration (`PR`) vs. Behind-Armor Blunt Trauma (`BABT`)** (`ranged_combat_and_armor.md` §2.2):
/// 1. $\text{Effective Pen} = \text{base\_pen} + \lfloor \max(0, \text{net\_successes}) / 2 \rfloor$.
/// 2. If $\text{Effective Pen} \le \text{PR}$ (or `damage_type == WeaponDamageType::Blunt`):
///    - Armor stops penetration (`penetrated = false`, `causes_bleed = false`).
///    - Target suffers $\text{BABT} = \max(0, \text{raw\_impact} - \text{BTD})$.
/// 3. If $\text{Effective Pen} > \text{PR}$:
///    - Projectile/blade overmatches the armor (`penetrated = true`, `causes_bleed = true`).
///    - Target suffers $\text{Effective Trauma} = \max(0, \text{raw\_impact} - \text{PR})$.
///    - Armor `PR` degrades by `1` (or `2` if $\text{Effective Pen} - \text{PR} \ge 4$).
pub fn resolve_armor_impact(
    armor: &ArmorProfile,
    damage_type: WeaponDamageType,
    raw_impact: i32,
    base_pen: i32,
    net_successes: i32,
    hits_visor_weak_spot: bool,
) -> ArmorImpactOutcome {
    let effective_pen = base_pen + (net_successes.max(0).div_euclid(2));
    let active_armor_pr = match damage_type {
        WeaponDamageType::Edged => armor.pr_edged,
        WeaponDamageType::IcepickOrFlechette => armor.pr_icepick_flechette,
        WeaponDamageType::Ballistic => {
            if hits_visor_weak_spot {
                armor.pr_visor_weak_spot
            } else {
                armor.pr_ballistic
            }
        }
        WeaponDamageType::Blunt => armor.pr_edged.max(armor.pr_ballistic),
    };

    if damage_type == WeaponDamageType::Blunt || effective_pen <= active_armor_pr {
        let babt_trauma = (raw_impact - armor.blunt_trauma_dampening_btd).max(0);
        ArmorImpactOutcome {
            effective_pen,
            active_armor_pr,
            penetrated: false,
            babt_trauma,
            penetrating_trauma: 0,
            total_effective_trauma: babt_trauma,
            causes_bleed: false,
            armor_pr_degradation: 0,
        }
    } else {
        let penetrating_trauma = (raw_impact - active_armor_pr).max(0);
        let overmatch = effective_pen - active_armor_pr;
        let armor_pr_degradation = if overmatch >= 4 { 2 } else { 1 };
        ArmorImpactOutcome {
            effective_pen,
            active_armor_pr,
            penetrated: true,
            babt_trauma: 0,
            penetrating_trauma,
            total_effective_trauma: penetrating_trauma,
            causes_bleed: true,
            armor_pr_degradation,
        }
    }
}

// ============================================================================
// Weapon & Equipment Reliability (`Rel`) & Malfunctions (§3.1 – §3.2)
// ============================================================================

/// Canonical Equipment Quality & Reliability (`Rel`) classes (`ranged_combat_and_armor.md` §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponReliabilityClass {
    /// **Improvised / Salvaged / Fouled (`Rel = -3`)**: Scrap firearms, corroded ammo, unmaintained cyber-joints.
    ImprovisedOrSalvaged,
    /// **Cheap / Street-Clone / Delicate High-Tech (`Rel = -1`)**: 3D-printed polymers, knockoff electronics.
    CheapStreetClone,
    /// **Standard Commercial / Military Issue (`Rel = 0`)**: Factory-spec service pistols, standard carbines.
    StandardCommercialOrIssue,
    /// **Rugged Military Classic (`Rel = +2`)**: Loose-tolerance battle rifles, revolvers, forged cold steel.
    RuggedMilitaryClassic,
    /// **Custom Match-Grade / Master-Tuned (`Rel = +3`)**: Hand-fitted aerospace-coated weapons maintained daily.
    CustomMatchGradeTuned,
}

impl WeaponReliabilityClass {
    /// Returns the canonical numeric Reliability rating $\text{Rel} \in [-4, +4]$.
    pub fn reliability_rating(self) -> i32 {
        match self {
            Self::ImprovisedOrSalvaged => -3,
            Self::CheapStreetClone => -1,
            Self::StandardCommercialOrIssue => 0,
            Self::RuggedMilitaryClassic => 2,
            Self::CustomMatchGradeTuned => 3,
        }
    }
}

/// Severity of a Weapon or Equipment Malfunction (`ranged_combat_and_armor.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MalfunctionKind {
    /// Weapon cycles normally (`0` clearance ticks).
    None,
    /// **Minor Malfunction (Stovepipe / Misfeed / Sensor Glitch)**:
    /// Current shot resolves, then weapon requires an `8-tick` Immediate Action (`Tap-Rack`) to clear (or drops `-1` Tool Tier).
    MinorMalfunctionStovepipe,
    /// **Major Malfunction (Double-Feed / Failure to Extract / Overheat)**:
    /// Shot fails; requires `20–32 ticks` (a full complex action) and a *Gunsmithing & Armorer* (`DEX/ANA`) check to strip and clear.
    MajorMalfunctionDoubleFeed,
}

/// Outcome of checking a cascade roll for Weapon/Equipment Malfunction (`ranged_combat_and_armor.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponMalfunctionOutcome {
    /// Effective Reliability rating (`Rel`) clamped to `-4..=+4`.
    pub effective_reliability: i32,
    /// Triggered malfunction severity.
    pub malfunction_kind: MalfunctionKind,
    /// Count-Up ticks required to clear the malfunction (`0`, `8`, or `26` average of `20..=32`).
    pub clearance_ticks: i32,
    /// Tool Tier degradation if the item is a non-firearm tool (`-1` on Minor, `-2` on Major, else `0`).
    pub tool_tier_degradation: i32,
}

/// Evaluates whether a [`CascadeRollOutcome`] triggers a Weapon or Equipment Malfunction (`ranged_combat_and_armor.md` §3.2):
/// 1. **On `SuccessWithComplication`** ($S \ge 1$ and $B_{\text{raw}} \ge 1$):
///    If $B_{\text{raw}} > \text{Rel}$, triggers [`MalfunctionKind::MinorMalfunctionStovepipe`] (`8 ticks` Tap-Rack).
///    (A rugged weapon with `Rel = +2` ignores `1` or `2` raw botches during a successful burst!)
/// 2. **On `BotchFailure`** ($S = 0$ and $B_{\text{eff}} \ge 2$):
///    If $B_{\text{eff}} + \text{environmental\_fouling} > 2 + \text{Rel}$, triggers [`MalfunctionKind::MajorMalfunctionDoubleFeed`] (`20..=32 ticks`).
pub fn evaluate_weapon_malfunction(
    roll: &CascadeRollOutcome,
    reliability_rel: i32,
    environmental_fouling: i32,
) -> WeaponMalfunctionOutcome {
    let rel = reliability_rel.clamp(-4, 4);
    let fouling = environmental_fouling.max(0);

    match roll.outcome_kind {
        CascadeOutcomeKind::SuccessWithComplication => {
            if (roll.raw_botches as i32) > rel {
                WeaponMalfunctionOutcome {
                    effective_reliability: rel,
                    malfunction_kind: MalfunctionKind::MinorMalfunctionStovepipe,
                    clearance_ticks: 8,
                    tool_tier_degradation: 1,
                }
            } else {
                WeaponMalfunctionOutcome {
                    effective_reliability: rel,
                    malfunction_kind: MalfunctionKind::None,
                    clearance_ticks: 0,
                    tool_tier_degradation: 0,
                }
            }
        }
        CascadeOutcomeKind::BotchFailure => {
            if (roll.effective_botches as i32) + fouling > 2 + rel {
                WeaponMalfunctionOutcome {
                    effective_reliability: rel,
                    malfunction_kind: MalfunctionKind::MajorMalfunctionDoubleFeed,
                    clearance_ticks: 26,
                    tool_tier_degradation: 2,
                }
            } else {
                WeaponMalfunctionOutcome {
                    effective_reliability: rel,
                    malfunction_kind: MalfunctionKind::MinorMalfunctionStovepipe,
                    clearance_ticks: 8,
                    tool_tier_degradation: 1,
                }
            }
        }
        _ => WeaponMalfunctionOutcome {
            effective_reliability: rel,
            malfunction_kind: MalfunctionKind::None,
            clearance_ticks: 0,
            tool_tier_degradation: 0,
        },
    }
}

/// Applies `30 minutes` of Downtime **Preventative Maintenance** with *Gunsmithing & Armorer* (`DEX/ANA`) (`ranged_combat_and_armor.md` §3.2):
/// Restores fouled reliability to baseline (`fouling = 0`) and grants `+1` to `Rel` for the next combat scene when `armorer_successes >= 1`.
pub fn apply_preventative_maintenance(
    base_reliability: i32,
    _current_fouling: i32,
    armorer_successes: i32,
) -> (i32, i32) {
    let bonus = i32::from(armorer_successes >= 1);
    let next_scene_rel = (base_reliability + bonus).clamp(-4, 4);
    (next_scene_rel, 0)
}

// ============================================================================
// Granular Firearm Engineering, Ammunition Physics & Terminal Ballistics (§3–§7)
// ============================================================================

/// Canonical Ammunition Caliber identifiers (`ranged_combat_and_armor.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmmoCaliberId {
    /// `9×19mm Parabellum` (`9.0 mm`, `124 gr` bullet, `5.2 gr` powder).
    Parabellum9x19,
    /// `.45 ACP` (`11.43 mm`, `230 gr` bullet, `5.6 gr` powder).
    Acp45,
    /// `5.7×28mm` (`5.7 mm`, `40 gr` bullet, `7.2 gr` powder).
    Fn57x28,
    /// `.357 Magnum` (`9.1 mm`, `158 gr` bullet, `14.5 gr` powder).
    Magnum357,
    /// `5.56×45mm NATO` (`5.56 mm`, `62 gr` bullet, `27.0 gr` powder).
    Nato556x45,
    /// `7.62×39mm` (`7.62 mm`, `123 gr` bullet, `25.5 gr` powder).
    Soviet762x39,
    /// `6.5mm Creedmoor` (`6.72 mm`, `140 gr` bullet, `42.5 gr` powder).
    Creedmoor65,
    /// `7.62×51mm NATO (.308)` (`7.62 mm`, `147 gr` bullet, `45.0 gr` powder).
    Nato762x51,
    /// `12-Gauge (2¾" 00 Buck / Slug)` (`18.5 mm`, `437 gr` payload, `32.0 gr` powder).
    Gauge12,
    /// `.338 Lapua Magnum` (`8.6 mm`, `250 gr` bullet, `90.0 gr` powder).
    Lapua338,
    /// `.50 BMG (12.7×99mm)` (`12.7 mm`, `660 gr` bullet, `235.0 gr` powder).
    Bmg50,
}

/// Canonical Firearm Feed Mechanism (`ranged_combat_and_armor.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponFeedType {
    /// Detachable Box Magazine (`Rel +0`, Reload `12–16 Ticks`).
    DetachableBoxMagazine,
    /// High-Capacity Drum or Helical Magazine (`Rel -2`, Reload `20 Ticks`).
    DrumOrHelical,
    /// Disintegrating-Link Belt-Fed (`Rel +0` braced, Reload `32 Ticks` solo / `14 Ticks` crew).
    BeltFed,
    /// Under-Barrel or Stock Tubular Magazine (`Rel +0`, Reload `6 Ticks` per shell, dynamic CoG).
    TubularMagazine,
    /// Revolving Cylinder (`Rel +1`, Reload `8 Ticks`/rd or `14 Ticks` speedloader).
    RevolvingCylinder,
    /// Single-Shot Breech or Fixed Internal Magazine (`Rel +1..+2`).
    SingleOrInternalFixed,
}

impl WeaponFeedType {
    /// Feed mechanism contribution to Weapon Reliability ($\text{Rel}_{\text{feed}}$).
    pub fn reliability_mod(self) -> i32 {
        match self {
            Self::DetachableBoxMagazine | Self::BeltFed | Self::TubularMagazine => 0,
            Self::DrumOrHelical => -2,
            Self::RevolvingCylinder | Self::SingleOrInternalFixed => 1,
        }
    }
}

/// Canonical Firearm Action Mechanism (`ranged_combat_and_armor.md` §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirearmActionType {
    /// Rotating-lug manual Bolt-Action ($\eta_{\text{action}} = 1.00$, $\Delta\text{Acc} = +1.5$, $\text{Rel} = +2$).
    BoltAction,
    /// Single-Action (`SA`) hammer-fired locked breech ($\eta_{\text{action}} = 0.99$, $\Delta\text{Acc} = +1.0$, $\text{Rel} = +1$).
    SingleAction,
    /// Striker-fired tilting-barrel locked breech ($\eta_{\text{action}} = 0.99$, $\Delta\text{Acc} = 0.0$, $\text{Rel} = +1$).
    StrikerFired,
    /// Direct Impingement (`DI`) internal gas piston ($\eta_{\text{action}} = 0.97$, $\Delta\text{Acc} = +0.5$, $\text{Rel} = 0$).
    DirectImpingement,
    /// Short-Stroke Gas Piston ($\eta_{\text{action}} = 0.97$, $\Delta\text{Acc} = 0.0$, $\text{Rel} = +1$).
    ShortStrokeGasPiston,
    /// Long-Stroke Gas Piston ($\eta_{\text{action}} = 0.97$, $\Delta\text{Acc} = -0.5$, $\text{Rel} = +2$).
    LongStrokeGasPiston,
    /// Manual Pump-Action ($\eta_{\text{action}} = 0.99$, $\Delta\text{Acc} = 0.0$, $\text{Rel} = +2$).
    PumpAction,
    /// Delayed Blowback ($\eta_{\text{action}} = 0.95$, $\Delta\text{Acc} = 0.0$, $\text{Rel} = 0$).
    DelayedBlowback,
    /// Open-Bolt or Sear-Conversion Auto-Switch ($\eta_{\text{action}} = 0.95$, $\Delta\text{Acc} = -1.5$, $\text{Rel} = -1$).
    OpenBoltOrAutoSwitch,
    /// Double/Single-Action Revolver with Cylinder Gap ($\eta_{\text{action}} = 0.92$, $\Delta\text{Acc} = +0.5$, $\text{Rel} = +2$).
    RevolverCylinderGap,
}

impl FirearmActionType {
    /// Propellant gas retention factor $\eta_{\text{action}} \in [0.92, 1.00]$.
    pub fn gas_efficiency_factor(self) -> f64 {
        match self {
            Self::BoltAction => 1.00,
            Self::SingleAction | Self::StrikerFired | Self::PumpAction => 0.99,
            Self::DirectImpingement | Self::ShortStrokeGasPiston | Self::LongStrokeGasPiston => {
                0.97
            }
            Self::DelayedBlowback | Self::OpenBoltOrAutoSwitch => 0.95,
            Self::RevolverCylinderGap => 0.92,
        }
    }

    /// Action contribution to Intrinsic Accuracy ($\Delta\text{Acc}_{\text{action}}$).
    pub fn accuracy_mod(self) -> f64 {
        match self {
            Self::BoltAction => 1.5,
            Self::SingleAction => 1.0,
            Self::DirectImpingement | Self::RevolverCylinderGap => 0.5,
            Self::StrikerFired
            | Self::ShortStrokeGasPiston
            | Self::PumpAction
            | Self::DelayedBlowback => 0.0,
            Self::LongStrokeGasPiston => -0.5,
            Self::OpenBoltOrAutoSwitch => -1.5,
        }
    }

    /// Action contribution to Weapon Reliability ($\text{Rel}_{\text{action}}$).
    pub fn reliability_mod(self) -> i32 {
        match self {
            Self::BoltAction
            | Self::LongStrokeGasPiston
            | Self::PumpAction
            | Self::RevolverCylinderGap => 2,
            Self::SingleAction | Self::StrikerFired | Self::ShortStrokeGasPiston => 1,
            Self::DirectImpingement | Self::DelayedBlowback => 0,
            Self::OpenBoltOrAutoSwitch => -1,
        }
    }
}

/// Barrel mounting method (`ranged_combat_and_armor.md` §5.2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarrelMountingType {
    /// Non-free-floated barrel (handguard, sling swivel, or gas/mag tube touches barrel: `-0.5` Acc).
    ClampedNonFreeFloated,
    /// Standard pistol tilting/fixed barrel (`0.0` Acc).
    StandardPistolMount,
    /// Free-floating barrel cantilevered only from the receiver barrel nut (`+1.0` Acc).
    FreeFloating,
}

impl BarrelMountingType {
    /// Mounting contribution to Intrinsic Accuracy ($\Delta\text{Acc}_{\text{mount}}$).
    pub fn accuracy_mod(self) -> f64 {
        match self {
            Self::ClampedNonFreeFloated => -0.5,
            Self::StandardPistolMount => 0.0,
            Self::FreeFloating => 1.0,
        }
    }
}

/// Muzzle attachment (`ranged_combat_and_armor.md` §3.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MuzzleDeviceType {
    /// Unthreaded crown or simple thread protector.
    BareCrown,
    /// Standard birdcage or prong flash hider.
    FlashHider,
    /// Baffled muzzle brake or compensator (`-2` Recoil, `-1%` velocity, `+2` side concussion).
    MuzzleBrakeCompensator,
    /// Sound suppressor / silencer (`+2%` freebore velocity boost, `-1` Recoil, `-8` to `-12` acoustic signature).
    SoundSuppressor,
}

impl MuzzleDeviceType {
    /// Velocity/energy efficiency shift ($\Delta\eta_{\text{muzzle}}$).
    pub fn energy_efficiency_delta(self) -> f64 {
        match self {
            Self::BareCrown | Self::FlashHider => 0.0,
            Self::MuzzleBrakeCompensator => -0.01,
            Self::SoundSuppressor => 0.02,
        }
    }

    /// Recoil reduction modifier ($\Delta R_{\text{muzzle}}$).
    pub fn recoil_mod(self) -> i32 {
        match self {
            Self::BareCrown => 0,
            Self::FlashHider | Self::SoundSuppressor => -1,
            Self::MuzzleBrakeCompensator => -2,
        }
    }
}

/// Center of Gravity relative to the chamber and firing-hand fulcrum (`ranged_combat_and_armor.md` §5.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CenterOfGravityProfile {
    /// Rear-biased Bullpup (`8–15 cm` behind chamber: `-0.5` Acc, `-1` Snap `TC`).
    RearBullpup,
    /// Chamber-centered / in-line neutral (`+0.5` Acc, `-1` Recoil).
    ChamberNeutral,
    /// Moderate forward bias (`5–14 cm` ahead of chamber: `+0.0` off-hand Acc, `-1` Recoil, `+1` Snap `TC`).
    ModerateForward,
    /// Extreme front-heavy / overhung (`15+ cm` ahead or heavy drum: `-1.0` off-hand Acc, `+2` Snap `TC`).
    ExtremeFrontHeavyOverhung,
}

impl CenterOfGravityProfile {
    /// Accuracy contribution ($\Delta\text{Acc}_{\text{CoG}}$).
    pub fn accuracy_mod(self) -> f64 {
        match self {
            Self::RearBullpup => -0.5,
            Self::ChamberNeutral => 0.5,
            Self::ModerateForward => 0.0,
            Self::ExtremeFrontHeavyOverhung => -1.0,
        }
    }

    /// Recoil modifier ($\Delta R_{\text{CoG}}$).
    pub fn recoil_mod(self) -> i32 {
        match self {
            Self::RearBullpup => 0,
            Self::ChamberNeutral | Self::ModerateForward | Self::ExtremeFrontHeavyOverhung => -1,
        }
    }
}

/// Physical and ballistic specification of a cartridge (`ranged_combat_and_armor.md` §3.1–§3.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmmunitionProfile {
    /// Canonical caliber identifier.
    pub caliber_id: AmmoCaliberId,
    /// Display name (e.g., `"9×19mm Parabellum"`).
    pub name: &'static str,
    /// Projectile bore diameter in millimeters ($d_{\text{cal}}$).
    pub caliber_mm: f64,
    /// Projectile mass in grains ($m_{\text{proj}}$, $1\text{ g} = 15.432\text{ gr}$).
    pub bullet_mass_grains: f64,
    /// Chemical gunpowder propellant charge in grains ($m_{\text{prop}}$).
    pub powder_charge_grains: f64,
    /// Optimal propellant-burn reference barrel length in centimeters ($L_{\text{opt}}$).
    pub optimal_barrel_cm: f64,
    /// Reference muzzle kinetic energy at $L_{\text{opt}}$ in a sealed barrel ($E_{\text{chem}}$ in Joules).
    pub reference_muzzle_energy_j: f64,
    /// Aerodynamic energy retention length scale in meters ($D_{\text{decay}}$).
    pub energy_decay_scale_m: f64,
    /// Hollow-point / soft-point expansion multiplier ($\epsilon_{\text{exp}} = 1.0$ for FMJ/AP, $1.55$ for JHP).
    pub expansion_factor: f64,
    /// Sectional density & penetrator bonus ($\text{SD}_{\text{bonus}} + \Delta\text{Pen}_{\text{ammo}}$).
    pub sectional_density_pen_bonus: i32,
    /// Ammunition manufacturing consistency ($Q_{\text{ammo}} \in [-1.5, +1.0]$).
    pub ammo_quality_q: f64,
    /// Whether this is a high-aspect-ratio spitzer that yaws early in soft tissue ($\Delta W_{\text{spitzer}} = +1$).
    pub is_spitzer_early_yaw: bool,
}

/// Complete granular engineering profile for a Firearm (`ranged_combat_and_armor.md` §7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GranularFirearmSpec {
    /// Model name & archetype.
    pub name: &'static str,
    /// Chambered ammunition profile.
    pub ammo: AmmunitionProfile,
    /// Feed system (`Box`, `Drum`, `Belt`, `Tubular`, `Cylinder`, `Single`).
    pub feed_type: WeaponFeedType,
    /// Rounds per feed unit (`Capacity`).
    pub capacity_rounds: i32,
    /// Operating action mechanism (`Bolt`, `Pump`, `DI`, `SingleAction`, `Piston`, `Striker`, etc.).
    pub action_type: FirearmActionType,
    /// Unloaded weapon mass in kilograms.
    pub unloaded_weight_kg: f64,
    /// Loaded weapon mass in kilograms ($M_{\text{wpn}}$).
    pub loaded_weight_kg: f64,
    /// Overall weapon length in centimeters ($\text{OAL}$).
    pub overall_length_cm: f64,
    /// Barrel length in centimeters ($L_{\text{bbl}}$).
    pub barrel_length_cm: f64,
    /// Barrel quality modifier ($Q_{\text{bbl}} \in [-2.0, +1.5]$).
    pub barrel_quality_q: f64,
    /// Barrel mounting method (`FreeFloating`, `ClampedNonFreeFloated`, `StandardPistolMount`).
    pub barrel_mounting: BarrelMountingType,
    /// Muzzle device (`BareCrown`, `FlashHider`, `MuzzleBrakeCompensator`, `SoundSuppressor`).
    pub muzzle_device: MuzzleDeviceType,
    /// Center of Gravity relative to chamber ($\Delta_{\text{CoG}}$).
    pub center_of_gravity: CenterOfGravityProfile,
    /// Machining tolerance ($\tau_{\text{tol}} \in [-1.5, +1.5]$: positive boosts `Acc` and reduces `Rel`).
    pub tolerance_tau: f64,
    /// Manufacturing & coating quality ($Q_{\text{mfg}} \in [-2, +2]$).
    pub manufacturing_quality_q: i32,
    /// Count-Up Ticks required to clear a Minor Malfunction ($\text{TC}_{\text{clear, minor}}$).
    pub clear_jam_minor_ticks: i32,
    /// Count-Up Ticks required to clear a Major Malfunction ($\text{TC}_{\text{clear, major}}$).
    pub clear_jam_major_ticks: i32,
    /// Count-Up Ticks required to field-strip the weapon ($\text{TC}_{\text{strip}}$).
    pub strip_time_ticks: i32,
    /// Street/retail price in 2032 USD / Credits.
    pub cost_usd: i32,
}

/// Complete physics-derived combat output for a [`GranularFirearmSpec`] at a target range $d$ (`ranged_combat_and_armor.md` §3–§6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GranularBallisticsEvaluation {
    /// Barrel length + muzzle device efficiency factor $\eta_{\text{bbl}}$.
    pub barrel_efficiency_eta: f64,
    /// Action gas retention factor $\eta_{\text{action}}$.
    pub action_efficiency_eta: f64,
    /// Muzzle kinetic energy $E_{\text{muzzle}}$ in Joules.
    pub muzzle_energy_joules: f64,
    /// Impact kinetic energy $E_k(d)$ at target distance $d$ in Joules.
    pub impact_energy_joules: f64,
    /// Logarithmic Kinetic Energy Index $I_{\text{KE}} \in [1, 9]$.
    pub ke_index: i32,
    /// Caliber & expansion permanent crush index $I_{\text{cal}} \in [1, 4]$.
    pub caliber_crush_index: i32,
    /// Intrinsic Weapon Accuracy rating $\text{Acc} \in [-4, +4]$.
    pub accuracy_acc: i32,
    /// Inverse-Accuracy Projectile Wobble / Yaw factor $W_{\text{yaw}} \in [0, 3]$.
    pub wobble_yaw_factor: i32,
    /// Total Raw Ballistic Impact ($\text{Raw Impact} = I_{\text{KE}} + I_{\text{cal}} + W_{\text{yaw}}$).
    pub raw_ballistic_impact: i32,
    /// Base Armor Penetration ($\text{Base Pen}$).
    pub base_armor_pen: i32,
    /// Base Single-Shot Recoil Rating ($R_{\text{wpn}}$).
    pub base_recoil_r: i32,
    /// Net Mechanical Reliability rating $\text{Rel} \in [-4, +4]$.
    pub reliability_rel: i32,
    /// CQB Overall Length crowding penalty in Clinch / tight corridors ($\max(0, \lfloor (\text{OAL} - 50)/20 \rfloor)$).
    pub cqb_length_crowding_penalty: i32,
    /// Minor jam clearance time in Count-Up Ticks ($\text{TC}_{\text{clear, minor}}$).
    pub clear_jam_minor_ticks: i32,
    /// Major jam clearance time in Count-Up Ticks ($\text{TC}_{\text{clear, major}}$).
    pub clear_jam_major_ticks: i32,
    /// Field-strip disassembly time in Count-Up Ticks ($\text{TC}_{\text{strip}}$).
    pub strip_time_ticks: i32,
}

/// Evaluates the complete internal/external/terminal ballistics, accuracy, recoil, and reliability
/// of a [`GranularFirearmSpec`] at target distance `distance_m` (`ranged_combat_and_armor.md` §3–§6).
pub fn evaluate_granular_firearm_ballistics(
    spec: &GranularFirearmSpec,
    distance_m: f64,
) -> GranularBallisticsEvaluation {
    // 1. Barrel efficiency eta_bbl & Action gas seal eta_action -> Kinetic Energy E_k(d)
    let ratio = (spec.barrel_length_cm / spec.ammo.optimal_barrel_cm).max(0.1);
    let barrel_efficiency_eta =
        (1.0 + 0.35 * ratio.ln()).clamp(0.60, 1.15) + spec.muzzle_device.energy_efficiency_delta();
    let action_efficiency_eta = spec.action_type.gas_efficiency_factor();
    let muzzle_energy_joules =
        spec.ammo.reference_muzzle_energy_j * barrel_efficiency_eta * action_efficiency_eta;
    let d = distance_m.max(0.0);
    let impact_energy_joules =
        muzzle_energy_joules * (-d / spec.ammo.energy_decay_scale_m.max(1.0)).exp();

    // 2. Kinetic Energy Index (I_KE) & Caliber Crush Index (I_cal)
    let ke_index = (1.85 * (1.0 + impact_energy_joules / 150.0).ln())
        .round()
        .clamp(1.0, 10.0) as i32;
    let caliber_crush_index = ((spec.ammo.caliber_mm * spec.ammo.expansion_factor) / 4.5)
        .floor()
        .clamp(1.0, 4.0) as i32;

    // 3. Intrinsic Accuracy (Acc) from 7 engineering subsystems
    let acc_barrel_len = if spec.barrel_length_cm < 9.0 {
        -1.5
    } else if spec.barrel_length_cm <= 25.0 {
        -0.5
    } else if spec.barrel_length_cm <= 44.0 {
        0.0
    } else if spec.barrel_length_cm <= 55.0 {
        0.5
    } else {
        1.0
    };
    let acc_weight = if spec.loaded_weight_kg < 0.65 {
        -0.5
    } else if spec.loaded_weight_kg > 4.5 {
        0.5
    } else {
        0.0
    };
    let raw_acc = spec.action_type.accuracy_mod()
        + acc_weight
        + spec.center_of_gravity.accuracy_mod()
        + acc_barrel_len
        + spec.barrel_quality_q
        + spec.barrel_mounting.accuracy_mod()
        + spec.tolerance_tau
        + spec.ammo.ammo_quality_q;
    let accuracy_acc = raw_acc.round().clamp(-4.0, 4.0) as i32;

    // 4. Inverse Accuracy Projectile Wobble / Yaw (W_yaw)
    let spitzer_bonus = i32::from(spec.ammo.is_spitzer_early_yaw && impact_energy_joules >= 800.0);
    let wobble_yaw_factor = ((1 - accuracy_acc).max(0) + spitzer_bonus).clamp(0, 3);

    // 5. Terminal Damage (Raw Ballistic Impact) & Base Armor Penetration (Pen)
    let raw_ballistic_impact = ke_index + caliber_crush_index + wobble_yaw_factor;
    let base_armor_pen = (ke_index + spec.ammo.sectional_density_pen_bonus
        - caliber_crush_index.div_euclid(2)
        - wobble_yaw_factor.div_euclid(2))
    .max(1);

    // 6. Recoil Rating (R_wpn) from bullet/powder impulse, weapon mass, CoG, and muzzle device
    let impulse_num = 0.025 * spec.ammo.bullet_mass_grains + 0.14 * spec.ammo.powder_charge_grains;
    let weight_sqrt = spec.loaded_weight_kg.max(0.3).sqrt();
    let base_recoil_r = ((impulse_num / weight_sqrt).round() as i32
        + spec.center_of_gravity.recoil_mod()
        + spec.muzzle_device.recoil_mod())
    .max(1);

    // 7. Mechanical Reliability (Rel) = Action + Feed - round(Tolerance) + Mfg Quality + Ammo Rel
    let ammo_rel_mod = if spec.ammo.ammo_quality_q < -1.0 {
        -1
    } else {
        0
    };
    let reliability_rel = (spec.action_type.reliability_mod() + spec.feed_type.reliability_mod()
        - (spec.tolerance_tau.round() as i32)
        + spec.manufacturing_quality_q
        + ammo_rel_mod)
        .clamp(-4, 4);

    let cqb_length_crowding_penalty = if spec.overall_length_cm > 50.0 {
        ((spec.overall_length_cm - 50.0) / 20.0).floor() as i32
    } else {
        0
    };

    GranularBallisticsEvaluation {
        barrel_efficiency_eta,
        action_efficiency_eta,
        muzzle_energy_joules,
        impact_energy_joules,
        ke_index,
        caliber_crush_index,
        accuracy_acc,
        wobble_yaw_factor,
        raw_ballistic_impact,
        base_armor_pen,
        base_recoil_r,
        reliability_rel,
        cqb_length_crowding_penalty,
        clear_jam_minor_ticks: spec.clear_jam_minor_ticks,
        clear_jam_major_ticks: spec.clear_jam_major_ticks,
        strip_time_ticks: spec.strip_time_ticks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psyche_social::{
        CombatMotivationKind, PsycheProfile, evaluate_combat_morale_roll,
        evaluate_combat_motivation_modifier,
    };
    use crate::rules_book::{MoraleState, TacticalPoolParams, compute_tactical_pool};

    #[test]
    fn test_viktor_vs_elena_melee_walkthrough() {
        // Canonical §5.1 Worked Example (`melee_combat.md`): Elena vs. Viktor ("The Sledge")
        let elena_params = TacticalPoolParams {
            spd: 13,
            for_stat: 10,
            dex: 12,
            kin: 13,
            sig: 12,
            ins: 12,
            itu: 13,
            ana: 12,
            cre: 10,
            foc: 12,
            dis: 13,
            fit: 12,
            mass_kg: 65.0,
            skill_c: 13,
            skill_n: 4,
            is_ranged: false,
            ..Default::default()
        };
        let mut viktor_params = TacticalPoolParams {
            spd: 9,
            for_stat: 15,
            dex: 10,
            kin: 10,
            sig: 10,
            ins: 8,
            itu: 8,
            ana: 8,
            cre: 8,
            foc: 8,
            dis: 9,
            fit: 13,
            mass_kg: 100.0,
            skill_c: 12,
            skill_n: 2,
            is_ranged: false,
            ..Default::default()
        };

        let elena_env = compute_tactical_pool(&elena_params);
        let viktor_env = compute_tactical_pool(&viktor_params);
        assert_eq!(elena_env.p_base, 24);
        assert_eq!(elena_env.neutral_baseline_b, 8);
        assert_eq!(viktor_env.p_base, 16);
        assert_eq!(viktor_env.neutral_baseline_b, 5);
        assert_eq!(viktor_env.tempo_cap, 9);
        assert_eq!(viktor_env.control_cap, 10);
        assert_eq!(viktor_env.force_cap, 15);
        assert!((viktor_env.power_pow - 13.5).abs() < 1e-6);

        // Exchange 1 (Tick 1): Elena allocates 11 Tempo / 11 Control / 2 Force (Base TC 8 -> Effective TC = 5, lands Tick 6)
        let elena_ex1 = evaluate_tactical_triad(&elena_env, 8, 11, 11, 2);
        assert!(elena_ex1.valid);
        assert_eq!(elena_ex1.effective_tc, 5);

        // Viktor suffers a Minor Leg Wound (-2 to Tempo Cap -> T_max = 7) and enters Rage (P_eff = 18) on Tick 3!
        viktor_params.leg_wound_penalty = 2;
        viktor_params.morale_state = MoraleState::Rage;
        let viktor_rage_env = compute_tactical_pool(&viktor_params);
        assert_eq!(viktor_rage_env.p_eff, 18);
        assert_eq!(viktor_rage_env.tempo_cap, 7);
        assert_eq!(viktor_rage_env.min_force_required, 9);
        assert_eq!(viktor_rage_env.max_control_allowed, 4);

        // Viktor allocates 3 Tempo / 3 Control / 12 Force for a Haymaker (Base TC 10 -> Effective TC = 10 - (3 - 5) = 12, lands Tick 3 + 12 = Tick 15)
        let viktor_haymaker = evaluate_tactical_triad(&viktor_rage_env, 10, 3, 3, 12);
        assert!(viktor_haymaker.valid);
        assert_eq!(viktor_haymaker.effective_tc, 12);

        // Exchange 2 (Tick 6): Elena allocates 6 Tempo / 10 Control / 8 Force (Base TC 10 -> Effective TC = 10 - (6 - 8) = 12, or faster counter)
        // Wait: Elena's counter has 10 Control vs Viktor's 3 Control -> +7 Control advantage!
        let elena_ex2 = evaluate_tactical_triad(&elena_env, 8, 6, 10, 8);
        assert!(elena_ex2.valid);
        let (interrupted, counter_land_tick) =
            check_melee_interrupt_before_impact(15, 6, elena_ex2.effective_tc - 2, true);
        assert!(interrupted);
        assert!(counter_land_tick < 15);

        // Viktor takes Wound Shock Tier 2 (-4 Pool) and drops out of Rage to Steady -> P_eff = 16 - 4 - 2 (Fatigue) = 10
        viktor_params.morale_state = MoraleState::Steady;
        viktor_params.wound_shock_index = 2;
        viktor_params.fatigue_level = 1;
        let viktor_after = compute_tactical_pool(&viktor_params);
        assert_eq!(viktor_after.p_eff, 10);
    }

    #[test]
    fn test_clinch_and_hip_throw_vs_knife_blitz() {
        // Canonical §5.2 Worked Example (`melee_combat.md`): Elena vs. Street Enforcer (Knife Blitz)
        // Enforcer: 70 kg, SPD 12 -> MMF = 12.0; allocates 10 Tempo / 2 Control / 8 Force -> Effective Root = 8 + 12 = 20
        // Inside Clinch, Elena allocates 4 Tempo / 12 Control / 8 Force and redirects Enforcer's 10 Tempo blitz
        let res = resolve_contested_melee_exchange(
            4,     // Elena Tempo
            12,    // Elena Control
            8,     // Elena Force
            10,    // Enforcer Tempo (blitzing)
            2,     // Enforcer Control
            8,     // Enforcer Force
            -1,    // Knife Guard (-1)
            12.0,  // Enforcer MMF
            1,     // Elena Clinch Reach 1
            1,     // Knife Reach 1
            true,  // in_clinch
            false, // keep-out
            2,     // Elena net successes
            true,  // is_throw_or_sweep
            true,  // redirect_opponent_blitz_in_clinch
        );
        assert!(res.kuzushi_throw_succeeded);
        assert!(res.leveraged_throw_force >= res.defender_effective_root);
        assert_eq!(res.prone_recovery_ticks, 6);
    }

    #[test]
    fn test_mateo_vs_sgt_vance_firefight_suppression_walkthrough() {
        // Canonical §4.1 Worked Example (`ranged_combat_and_armor.md`): Mateo vs. Sgt. Vance
        // Mateo: P_base = 18, allocates 8 Tempo / 2 Control / 8 Force on Full-Auto Burst (R = 8)
        let mateo_ranged = evaluate_ranged_recoil_and_suppression(8, 2, 8, 8, false, true);
        assert!(mateo_ranged.recoil_fully_braced);
        assert_eq!(mateo_ranged.unbraced_dispersion_delta_ext, 0);
        assert_eq!(mateo_ranged.suppression_morale_delta, 4); // floor(8 / 2) = +4 suppression morale penalty!

        // Sgt. Vance (DIS 14, Veteran N_P = 2) rolls Morale check against Delta_morale = +4 (TN_GM = 17, TN_P = 17)
        // Vance rolls (10, 9) = 19 > 17 (1 success), GM rolls (8, 7) = 15 <= 17 (0 successes) -> holds Steady!
        let vance_morale = evaluate_combat_morale_roll(
            14,
            0,
            0,
            mateo_ranged.suppression_morale_delta,
            true,
            MoraleState::Steady,
            &[(10, 9), (8, 8)],
            (8, 7),
        );
        assert_eq!(vance_morale.new_morale, MoraleState::Steady);

        // Vance allocates 7 Tempo / 12 Control / 5 Force for a Controlled Double-Tap (R = 5)
        let vance_ranged = evaluate_ranged_recoil_and_suppression(7, 12, 5, 5, false, false);
        assert!(vance_ranged.recoil_fully_braced);

        // Mateo takes a Moderate Arm Wound (-5 to F_max and C_max, Wound Shock Tier 2 -> -4 Pool)
        // Mateo (Mercenary SLF 15, BLG 7 -> Delta_mot = -2, DIS 9 -> C_morale = 7, TN_P = 24) botches his Morale check -> drops 2 steps to Broken!
        let mateo_profile = PsycheProfile {
            slf: 15,
            blg: 7,
            ..Default::default()
        };
        let mateo_mot = evaluate_combat_motivation_modifier(
            &mateo_profile,
            CombatMotivationKind::MercenaryTransactionalUnderFire,
        );
        assert_eq!(mateo_mot.delta_mot, -2);

        let mateo_morale_roll = evaluate_combat_morale_roll(
            9,
            mateo_mot.delta_mot,
            0,
            4,
            false,
            MoraleState::Steady,
            &[(4, 4)], // 8 <= 24 - 6 (18) -> BotchFailure!
            (10, 10),
        );
        assert_eq!(mateo_morale_roll.new_morale, MoraleState::Broken);

        // Mateo's pool collapses from 18 by -4 (Wound Shock 2) and -4 (Broken Morale) to P_eff = 10, with Force locked at 0!
        let mateo_broken_env = compute_tactical_pool(&TacticalPoolParams {
            spd: 12,
            for_stat: 12,
            dex: 8,
            sig: 8,
            ins: 12,
            itu: 12,
            ana: 12,
            foc: 12,
            skill_c: 12,
            skill_n: 2,
            is_ranged: true,
            wound_shock_index: 2,
            arm_head_wound_penalty: 5,
            torso_shoulder_wound_penalty: 5,
            morale_state: MoraleState::Broken,
            ..Default::default()
        });
        assert_eq!(mateo_broken_env.p_base, 18);
        assert_eq!(mateo_broken_env.p_eff, 10);
        assert_eq!(mateo_broken_env.max_force_allowed, 0);
        assert_eq!(mateo_broken_env.control_cap, 3); // 8 - 5 = 3

        // Mateo is forced to allocate 7 Tempo / 3 Control / 0 Force to scramble backward into cover
        let scramble = evaluate_tactical_triad(&mateo_broken_env, 8, 7, 3, 0);
        assert!(scramble.valid);
    }

    #[test]
    fn test_sniper_overwatch_and_grenade_dive_walkthrough() {
        // Canonical §4.2 Worked Example (`ranged_combat_and_armor.md`): Elena (P_eff = 24)
        let elena_env = compute_tactical_pool(&TacticalPoolParams {
            spd: 13,
            for_stat: 10,
            dex: 12,
            kin: 13,
            sig: 12,
            ins: 12,
            itu: 13,
            ana: 12,
            cre: 10,
            foc: 12,
            dis: 13,
            fit: 12,
            mass_kg: 65.0,
            skill_c: 13,
            skill_n: 4,
            is_ranged: false,
            ..Default::default()
        });
        // Bound 1 (Sniper Overwatch): 12 Tempo / 12 Control / 0 Force (TC — The Probe / Evasive Sprint)
        let bound1 = evaluate_tactical_triad(&elena_env, 8, 12, 12, 0);
        assert!(bound1.valid);
        assert!(bound1.archetype_label.starts_with("TC"));
        assert_eq!(bound1.exertion_marks_gained, 1); // 12 > 13/2

        // Bound 2 (Grenade Dive): 12 Tempo / 2 Control / 10 Force (TF — The Blitz / Explosive Dive)
        let bound2 = evaluate_tactical_triad(&elena_env, 8, 12, 2, 10);
        assert!(bound2.valid);
        assert!(bound2.archetype_label.starts_with("TF"));
        assert_eq!(bound2.exertion_marks_gained, 2); // 12 > 13/2 and 10 > 10/2
    }

    #[test]
    fn test_armor_penetration_vs_babt_and_weapon_malfunctions() {
        let catalog = canonical_armor_catalog();
        let kevlar = catalog
            .iter()
            .find(|a| a.id == ArmorProfileId::ConcealableSoftKevlarVestNij3a)
            .unwrap();
        // 9mm FMJ (Base Pen 2, Raw Impact 6, 1 Net Success -> Effective Pen = 2 <= PR_Ballistic 4)
        // Stopped by Kevlar! Inflicts BABT = 6 - 2 = 4 non-bleeding blunt trauma.
        let stopped = resolve_armor_impact(kevlar, WeaponDamageType::Ballistic, 6, 2, 1, false);
        assert!(!stopped.penetrated);
        assert!(!stopped.causes_bleed);
        assert_eq!(stopped.babt_trauma, 4);
        assert_eq!(stopped.total_effective_trauma, 4);
        assert_eq!(stopped.armor_pr_degradation, 0);

        // AP Carbine round (Base Pen 4, Raw Impact 9, 2 Net Successes -> Effective Pen = 4 + 1 = 5 > PR_Ballistic 4)
        // Overmatches Kevlar! Inflicts 9 - 4 = 5 penetrating trauma + bleed + 1 PR degradation.
        let overmatched = resolve_armor_impact(kevlar, WeaponDamageType::Ballistic, 9, 4, 2, false);
        assert!(overmatched.penetrated);
        assert!(overmatched.causes_bleed);
        assert_eq!(overmatched.penetrating_trauma, 5);
        assert_eq!(overmatched.armor_pr_degradation, 1);

        // Icepick thrust against Soft Kevlar uses pr_icepick_flechette = 1!
        let icepick =
            resolve_armor_impact(kevlar, WeaponDamageType::IcepickOrFlechette, 5, 1, 2, false);
        assert_eq!(icepick.active_armor_pr, 1);
        assert_eq!(icepick.effective_pen, 2);
        assert!(icepick.penetrated);
    }

    #[test]
    fn test_granular_firearm_ballistics_akm_vs_65_creedmoor_and_tolerance_tradeoff() {
        // 1. Loose-tolerance AKM (7.62x39mm, 123 gr, 25.5 gr powder, Long-Stroke Piston, Clamped non-FF barrel)
        let akm = GranularFirearmSpec {
            name: "Kalashnikov AKM (7.62x39mm)",
            ammo: AmmunitionProfile {
                caliber_id: AmmoCaliberId::Soviet762x39,
                name: "7.62x39mm FMJ",
                caliber_mm: 7.62,
                bullet_mass_grains: 123.0,
                powder_charge_grains: 25.5,
                optimal_barrel_cm: 41.5,
                reference_muzzle_energy_j: 2100.0,
                energy_decay_scale_m: 420.0,
                expansion_factor: 1.0,
                sectional_density_pen_bonus: 2,
                ammo_quality_q: 0.0,
                is_spitzer_early_yaw: false,
            },
            feed_type: WeaponFeedType::DetachableBoxMagazine,
            capacity_rounds: 30,
            action_type: FirearmActionType::LongStrokeGasPiston,
            unloaded_weight_kg: 3.30,
            loaded_weight_kg: 4.10,
            overall_length_cm: 88.0,
            barrel_length_cm: 41.5,
            barrel_quality_q: 0.0,
            barrel_mounting: BarrelMountingType::ClampedNonFreeFloated,
            muzzle_device: MuzzleDeviceType::BareCrown,
            center_of_gravity: CenterOfGravityProfile::ModerateForward,
            tolerance_tau: -1.0, // Loose combat tolerance: -1 Acc, +1 Rel!
            manufacturing_quality_q: 1,
            clear_jam_minor_ticks: 8,
            clear_jam_major_ticks: 20,
            strip_time_ticks: 32,
            cost_usd: 950,
        };

        // 2. Bolt-Action 6.5mm Creedmoor Precision Rifle (140 gr Match OTM, 42.5 gr powder, Free-Floating, Tight Tolerance)
        let creedmoor_rifle = GranularFirearmSpec {
            name: "Accuracy Int'l AT-X (6.5mm Creedmoor)",
            ammo: AmmunitionProfile {
                caliber_id: AmmoCaliberId::Creedmoor65,
                name: "6.5mm Creedmoor Match OTM",
                caliber_mm: 6.72,
                bullet_mass_grains: 140.0,
                powder_charge_grains: 42.5,
                optimal_barrel_cm: 56.0,
                reference_muzzle_energy_j: 3165.0,
                energy_decay_scale_m: 950.0,
                expansion_factor: 1.4,
                sectional_density_pen_bonus: 3,
                ammo_quality_q: 1.0,
                is_spitzer_early_yaw: false,
            },
            feed_type: WeaponFeedType::DetachableBoxMagazine,
            capacity_rounds: 10,
            action_type: FirearmActionType::BoltAction,
            unloaded_weight_kg: 5.40,
            loaded_weight_kg: 5.80,
            overall_length_cm: 112.0,
            barrel_length_cm: 61.0,
            barrel_quality_q: 1.5,
            barrel_mounting: BarrelMountingType::FreeFloating,
            muzzle_device: MuzzleDeviceType::MuzzleBrakeCompensator,
            center_of_gravity: CenterOfGravityProfile::ChamberNeutral,
            tolerance_tau: 1.0, // Tight match tolerance: +1 Acc, -1 Rel (offset by Masterwork +2)!
            manufacturing_quality_q: 2,
            clear_jam_minor_ticks: 6,
            clear_jam_major_ticks: 20,
            strip_time_ticks: 80,
            cost_usd: 5200,
        };

        // At CQB range (15m):
        let akm_15m = evaluate_granular_firearm_ballistics(&akm, 15.0);
        assert!(akm_15m.accuracy_acc < 0); // Loose tolerance + Long-stroke piston + Non-free-floated -> Acc < 0
        assert!(akm_15m.wobble_yaw_factor >= 2); // Inverse accuracy gives erratic wobbling wound bonus!
        assert_eq!(akm_15m.reliability_rel, 4); // +2 Long-Stroke + 0 Box - (-1 Loose) + 1 Mfg = +4 Rel!
        assert!(akm_15m.raw_ballistic_impact >= 8);

        // At 400m Overwatch range:
        let akm_400m = evaluate_granular_firearm_ballistics(&akm, 400.0);
        let cm_400m = evaluate_granular_firearm_ballistics(&creedmoor_rifle, 400.0);
        assert_eq!(cm_400m.action_efficiency_eta, 1.00); // Bolt-action loses zero gas!
        assert_eq!(cm_400m.accuracy_acc, 4); // Peak precision (+4 Acc)
        assert_eq!(cm_400m.wobble_yaw_factor, 0); // Zero wobble
        assert!(cm_400m.impact_energy_joules > 2.0 * akm_400m.impact_energy_joules);
        assert!(cm_400m.base_armor_pen >= 7);
        assert!(akm_400m.base_armor_pen <= 4);
    }
}
