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
use crate::rules_book::{
    TacticalPoolEnvelope, TacticalTriadEvaluation, evaluate_tactical_triad,
};

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
        if attacker_reach >= 3 {
            4
        } else {
            0
        }
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

    let kuzushi_throw_succeeded = is_throw_or_sweep
        && strike_lands
        && leveraged_throw_force >= defender_effective_root;
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
        let overmatched =
            resolve_armor_impact(kevlar, WeaponDamageType::Ballistic, 9, 4, 2, false);
        assert!(overmatched.penetrated);
        assert!(overmatched.causes_bleed);
        assert_eq!(overmatched.penetrating_trauma, 5);
        assert_eq!(overmatched.armor_pr_degradation, 1);

        // Icepick thrust against Soft Kevlar uses pr_icepick_flechette = 1!
        let icepick = resolve_armor_impact(
            kevlar,
            WeaponDamageType::IcepickOrFlechette,
            5,
            1,
            2,
            false,
        );
        assert_eq!(icepick.active_armor_pr, 1);
        assert_eq!(icepick.effective_pen, 2);
        assert!(icepick.penetrated);
    }
}
