//! # Movement Architecture, Encumbrance, Vehicles, Chases & Environmental Survival (`06-health-and-survival/movement_vehicles_and_environment.md`)
//!
//! Canonical Rust implementation of `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md`:
//! - **§1 Movement Architecture (`§1.1–§1.4`)**: Stride Factor ($\text{SF} = \text{HGT}/1.75$), Maximum Sprint Velocity
//!   ($V_{\text{sprint}} = (4.0 + 0.35\times\text{SPD}_{\text{eff}})\times\text{SF}$), 5 Combat Movement Gaits,
//!   Jumping/Climbing/Swimming/Zero-G EVA, and Overland Travel Paces & Terrain Multipliers.
//! - **§2 Encumbrance & Load-Bearing (`§2.1–§2.3`, `Tiers 0–4`)**: Base Comfort Load
//!   ($L_{\text{base}} = (\text{MAS}/70.0)\times(\text{FOR}\times 1.2\text{ kg})$), Ergonomic Load Multipliers
//!   (`×0.8` to `×1.5` and Powered Exo-Rig offsets), and the 5 Encumbrance Tiers (`0–4`).
//! - **§3 Vehicles, Chases & High-G / Crash Physics (`§3.1–§3.3`)**: Vehicle Envelope Ceilings (`Tempo`, `Control`, `Force/Root`),
//!   Chase Range Bands, Collision Kinetic Trauma, and High-G `G-LOC` Checks.
//! - **§4 Environmental Survival (`§4.1–§4.4`)**: Hydration & Dehydration (`Stages 0–4` with Fatigue Floors),
//!   Caloric Deficit & Starvation, Thermal Extremes (`CLO` Suit Tiers, Hypothermia, Frostbite, Heatstroke),
//!   Sleep Deprivation (`24h/48h/72h+`), Altitude/Vacuum Hypoxia, 4-Parameter Toxins, and Ionizing Radiation (`Sv`).

use serde::{Deserialize, Serialize};

use crate::action_economy::{EncumbranceTier, StealthGait};
use crate::primitives::{ConditionImpact, ConditionSource, Triad};

pub use crate::environmental_hazards::*;
// ============================================================================
// 1. Movement Architecture, Combat Gaits, Special Modes & Overland (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.1–§1.4)
// ============================================================================

/// Baseline Stride Factor ($\text{SF}$) and Sprint Velocity ($V_{\text{sprint}}$)
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SprintVelocityMetrics {
    pub stride_factor_sf: f64,
    /// Maximum Sprint Velocity $V_{\text{sprint}} = (4.0 + 0.35 \times \text{SPD}_{\text{eff}}) \times \text{SF}$ in meters per second (`m/s`).
    pub v_sprint_m_per_sec: f64,
    pub v_sprint_km_per_hr: f64,
    pub v_sprint_m_per_tick: f64,
}

/// Computes Stride Factor ($\text{SF}$) and Maximum Sprint Velocity ($V_{\text{sprint}}$)
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.1).
#[must_use]
pub fn compute_stride_and_sprint_velocity(spd_eff: i32, hgt_m: f64) -> SprintVelocityMetrics {
    let spd = spd_eff.clamp(0, 25);
    let hgt = hgt_m.clamp(0.5, 3.0);
    let stride_factor_sf = hgt / 1.75;
    let v_sprint_m_per_sec = (4.0 + 0.35 * f64::from(spd)) * stride_factor_sf;
    let v_sprint_km_per_hr = v_sprint_m_per_sec * 3.6;
    let v_sprint_m_per_tick = v_sprint_m_per_sec / 4.0;

    SprintVelocityMetrics {
        stride_factor_sf,
        v_sprint_m_per_sec,
        v_sprint_km_per_hr,
        v_sprint_m_per_tick,
    }
}

/// The 5 Canonical Tactical Combat Movement Gaits
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.2 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatMovementGait {
    /// **1. Prone / Low Crawl** (`8%` of $V_{\text{sprint}}$): `+4` low cover bonus vs. ranged; melee `Tempo Cap -4`; `+1` Exertion Mark / `20 ticks`.
    ProneLowCrawl,
    /// **2. Stealth Creep / Crouch-Walk** (`18%` of $V_{\text{sprint}}$): `+0` $\Delta_{\text{ext}}$ on Stealth; full `Control` uncapped.
    StealthCreepCrouchWalk,
    /// **3. Combat Glide / Tactical Walk** (`35%` of $V_{\text{sprint}}$): Simultaneous aimed fire at `+2` Recoil (`R`), `0` Exertion Marks.
    CombatGlideTacticalWalk,
    /// **4. Bounding Run / Hustle** (`65%` of $V_{\text{sprint}}$): Requires `Tempo >= B`; `+4` Recoil (`R`); caps ranged `Control` at $\lfloor C_{\max}/2\rfloor$; `+1` Exertion Mark / `20 ticks`.
    BoundingRunHustle,
    /// **5. All-Out Sprint (`Pure Tempo`)** (`100%` of $V_{\text{sprint}}$): Requires `Force = 0, Control <= 2`; cannot fire aimed shots; `+2` Exertion Marks / `20 ticks`.
    AllOutSprint,
}

impl CombatMovementGait {
    /// Fraction of $V_{\text{sprint}}$ (`0.08`, `0.18`, `0.35`, `0.65`, `1.00`,
    /// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.2 Table).
    #[must_use]
    pub const fn sprint_fraction(self) -> f64 {
        match self {
            Self::ProneLowCrawl => 0.08,
            Self::StealthCreepCrouchWalk => 0.18,
            Self::CombatGlideTacticalWalk => 0.35,
            Self::BoundingRunHustle => 0.65,
            Self::AllOutSprint => 1.00,
        }
    }

    /// Extra Recoil (`R`) added when firing a ranged weapon while moving at this gait (`0`, `0`, `+2`, `+4`, or un-aimable,
    /// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.2 Table).
    #[must_use]
    pub const fn ranged_recoil_adder(self) -> i32 {
        match self {
            Self::ProneLowCrawl | Self::StealthCreepCrouchWalk => 0,
            Self::CombatGlideTacticalWalk => 2,
            Self::BoundingRunHustle => 4,
            Self::AllOutSprint => 8,
        }
    }

    /// Exertion Marks accumulated per `20 Ticks` (`5 seconds`) of continuous movement in this gait
    /// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.2 Table).
    #[must_use]
    pub const fn exertion_marks_per_20_ticks(self) -> i32 {
        match self {
            Self::StealthCreepCrouchWalk | Self::CombatGlideTacticalWalk => 0,
            Self::ProneLowCrawl | Self::BoundingRunHustle => 1,
            Self::AllOutSprint => 2,
        }
    }

    #[must_use]
    pub const fn to_stealth_gait(self) -> StealthGait {
        match self {
            Self::ProneLowCrawl | Self::StealthCreepCrouchWalk => StealthGait::StealthCreep,
            Self::CombatGlideTacticalWalk => StealthGait::CombatGlide,
            Self::BoundingRunHustle | Self::AllOutSprint => StealthGait::RunningDash,
        }
    }
}

impl From<CombatMovementGait> for StealthGait {
    fn from(gait: CombatMovementGait) -> Self {
        gait.to_stealth_gait()
    }
}

/// Complete combat movement and triad interaction metrics for a chosen [`CombatMovementGait`]
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.2 Table).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CombatGaitEvaluation {
    pub gait: CombatMovementGait,
    pub speed_m_per_sec: f64,
    pub speed_m_per_tick: f64,
    pub distance_per_10_ticks_m: f64,
    pub ranged_recoil_adder: i32,
    /// Effective Ranged `Control Cap` after gait constraints ($\lfloor C_{\max}/2\rfloor$ in `BoundingRunHustle`, `2` in `AllOutSprint`).
    pub effective_ranged_control_cap: i32,
    pub allows_aimed_ranged_fire: bool,
    pub exertion_marks_per_20_ticks: i32,
}

/// Evaluates velocity (`m/s`, `m/Tick`, distance per `10 Ticks`) and Tactical Triad constraints for a [`CombatMovementGait`]
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.2 Table).
#[must_use]
pub fn evaluate_combat_movement_gait(
    gait: CombatMovementGait,
    spd_eff: i32,
    hgt_m: f64,
    base_ranged_control_cap: i32,
) -> CombatGaitEvaluation {
    let sprint = compute_stride_and_sprint_velocity(spd_eff, hgt_m);
    let frac = gait.sprint_fraction();
    let speed_m_per_sec = sprint.v_sprint_m_per_sec * frac;
    let speed_m_per_tick = sprint.v_sprint_m_per_tick * frac;
    let distance_per_10_ticks_m = speed_m_per_tick * 10.0;
    let c_cap = base_ranged_control_cap.max(0);

    let (effective_ranged_control_cap, allows_aimed_ranged_fire) = match gait {
        CombatMovementGait::ProneLowCrawl
        | CombatMovementGait::StealthCreepCrouchWalk
        | CombatMovementGait::CombatGlideTacticalWalk => (c_cap, true),
        CombatMovementGait::BoundingRunHustle => (c_cap.div_euclid(2), true),
        CombatMovementGait::AllOutSprint => (c_cap.min(2), false),
    };

    CombatGaitEvaluation {
        gait,
        speed_m_per_sec,
        speed_m_per_tick,
        distance_per_10_ticks_m,
        ranged_recoil_adder: gait.ranged_recoil_adder(),
        effective_ranged_control_cap,
        allows_aimed_ranged_fire,
        exertion_marks_per_20_ticks: gait.exertion_marks_per_20_ticks(),
    }
}

/// Jump & Vault distances in meters (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct JumpDistances {
    /// Running Broad Jump: $2.0 + 0.25\times\text{POW} + 0.3\times\text{Successes}$ (`4.5 m` baseline at `POW 10`).
    pub running_broad_jump_m: f64,
    /// Standing Broad Jump: `55%` of Running Broad Jump (`~2.475 m` at `POW 10`).
    pub standing_broad_jump_m: f64,
    /// Running High Jump / Waist-Chest Vault without hands: $0.6 + 0.06\times\text{POW} + 0.1\times\text{Successes}$ (`1.2 m` at `POW 10`).
    pub running_high_jump_no_hands_m: f64,
    pub ledge_grab_pullup_height_m: f64,
}

/// Computes Running Broad Jump, Standing Broad Jump, Running High Jump, and Ledge-Grab Scaling heights
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.3.1).
#[must_use]
pub fn compute_jump_distances(pow: f64, hgt_m: f64, cascade_successes: i32) -> JumpDistances {
    let p = pow.max(0.0);
    let s = f64::from(cascade_successes.max(0));
    let hgt = hgt_m.clamp(0.5, 3.0);

    let running_broad_jump_m = 2.0 + 0.25 * p + 0.3 * s;
    let standing_broad_jump_m = 0.55 * running_broad_jump_m;
    let running_high_jump_no_hands_m = 0.6 + 0.06 * p + 0.1 * s;
    let ledge_grab_pullup_height_m = running_high_jump_no_hands_m + 0.8 * hgt;

    JumpDistances {
        running_broad_jump_m,
        standing_broad_jump_m,
        running_high_jump_no_hands_m,
        ledge_grab_pullup_height_m,
    }
}

/// Evaluates vertical climbing speed and `FOR`/`KIN` check difficulty
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.3.2).
///
/// Returns `(speed_m_per_tick, speed_m_per_s, requires_check, total_delta_ext)`:
/// - Ladder or knotted rope: `0.5 m/Tick` (`2.0 m/s`) without a check (`(0.5, 2.0, false, 0)`).
/// - Rough wall / rock face / pipe: `0.15 m/Tick` (`0.6 m/s`) with a `FOR` + `KIN` check and `+2` $\Delta_{\text{ext}}$ per Encumbrance Tier.
#[must_use]
pub const fn evaluate_climbing_movement(
    is_ladder_or_knotted_rope: bool,
    encumbrance: LoadEncumbranceTier,
    surface_base_delta_ext: i32,
) -> (f64, f64, bool, i32) {
    if is_ladder_or_knotted_rope {
        (0.5, 2.0, false, 0)
    } else {
        let base_ext = if surface_base_delta_ext < 0 {
            0
        } else {
            surface_base_delta_ext
        };
        let total_delta_ext = base_ext + 2 * (encumbrance.tier_number() as i32);
        (0.15, 0.6, true, total_delta_ext)
    }
}

/// Evaluates surface swimming speed (`20%` of $V_{\text{sprint}}$) and water encumbrance hazards
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.3.3).
///
/// Returns `(swim_speed_m_per_sec, swim_speed_m_per_tick, stay_afloat_delta_ext, sinks_at_1m_per_sec)`.
#[must_use]
pub fn evaluate_swimming_movement(
    spd_eff: i32,
    hgt_m: f64,
    load_encumbrance_tier: u8,
) -> (f64, f64, i32, bool) {
    let sprint = compute_stride_and_sprint_velocity(spd_eff, hgt_m);
    let swim_m_per_sec = 0.20 * sprint.v_sprint_m_per_sec;
    let swim_m_per_tick = 0.20 * sprint.v_sprint_m_per_tick;
    let stay_afloat_delta_ext = match load_encumbrance_tier {
        0 => 0,
        1 => 1,
        2 => 4,
        _ => 8,
    };
    let sinks_at_1m_per_sec = load_encumbrance_tier >= 3;
    (
        swim_m_per_sec,
        swim_m_per_tick,
        stay_afloat_delta_ext,
        sinks_at_1m_per_sec,
    )
}

/// Evaluates a Microgravity / Zero-G EVA push-off or thruster maneuver
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.3.4).
///
/// Returns `(velocity_m_per_s, is_adrift_tempo_zero)` where $v = \text{Impulse} / \text{MAS}$ and
/// a failed `KIN` maneuver check leaves the character **Adrift** (`Tempo = 0` until grabbing a handhold or firing thrusters).
#[must_use]
pub fn evaluate_zero_g_maneuver(
    impulse_newton_seconds: f64,
    mass_kg: f64,
    maneuver_check_succeeded: bool,
) -> (f64, bool) {
    let velocity_m_per_s = if mass_kg > 0.0 {
        impulse_newton_seconds.max(0.0) / mass_kg
    } else {
        0.0
    };
    (velocity_m_per_s, !maneuver_check_succeeded)
}

/// Non-Combat Overland Travel Pace (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.4 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlandTravelPace {
    /// **Cautious Recon / Trail-Finding**: `3.0 km/h` (`50 m/min`), `24 km/day`; active Observation & Stealth without penalty.
    CautiousRecon,
    /// **Standard Marching / Walking Pace**: `5.0 km/h` (`83 m/min`), `40 km/day`; `+1` fatigue after `8 hrs` (`4 hrs` at Tier 2+ load).
    StandardMarching,
    /// **Forced March / Double-Time**: `7.0 km/h` (`117 m/min`), `56 km/day`; roll `FIT/DIS` every `2 hrs` ($\Delta_{\text{ext}} = +2 \times \text{Load Tier}$) or `+1` fatigue.
    ForcedMarchDoubleTime,
    /// **Sustained Cross-Country Run**: `11.0 km/h` (`183 m/min`); roll `FIT` every `15 min` or `+1` fatigue.
    SustainedCrossCountryRun,
}

/// Terrain multiplier to movement speed and exertion
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlandTerrainKind {
    /// Paved Road / Corridor / Dry Sidewalk (`×1.00`, `+0` exertion)
    PavedRoadCorridor,
    /// Broken Ground / Tall Grass / Light Woods / Stairs (`×0.75`, `+0` exertion)
    BrokenGroundLightWoods,
    /// Dense Jungle / Mud / Sand Dunes / Rubble (`×0.50`, `+1` Exertion Mark per interval)
    DenseJungleMudSandRubble,
    /// Deep Snow / Chest-Deep Marsh / Steep Scree (`×0.25`, `+2` Exertion Marks per interval)
    DeepSnowMarshSteepScree,
}

impl OverlandTerrainKind {
    /// Speed multiplier (`1.00`, `0.75`, `0.50`, `0.25`,
    /// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.4).
    #[must_use]
    pub const fn speed_multiplier(self) -> f64 {
        match self {
            Self::PavedRoadCorridor => 1.00,
            Self::BrokenGroundLightWoods => 0.75,
            Self::DenseJungleMudSandRubble => 0.50,
            Self::DeepSnowMarshSteepScree => 0.25,
        }
    }

    /// Extra Exertion Marks per travel interval (`0`, `0`, `1`, `2`,
    /// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.4).
    #[must_use]
    pub const fn extra_exertion_marks(self) -> i32 {
        match self {
            Self::PavedRoadCorridor | Self::BrokenGroundLightWoods => 0,
            Self::DenseJungleMudSandRubble => 1,
            Self::DeepSnowMarshSteepScree => 2,
        }
    }
}

/// Evaluates effective overland travel speed (`km/h`) and daily range (`km` over `8 hours`)
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.4).
#[must_use]
pub fn evaluate_overland_travel(
    pace: OverlandTravelPace,
    terrain: OverlandTerrainKind,
) -> (f64, f64, i32) {
    let base_km_per_hr = match pace {
        OverlandTravelPace::CautiousRecon => 3.0,
        OverlandTravelPace::StandardMarching => 5.0,
        OverlandTravelPace::ForcedMarchDoubleTime => 7.0,
        OverlandTravelPace::SustainedCrossCountryRun => 11.0,
    };
    let eff_km_per_hr = base_km_per_hr * terrain.speed_multiplier();
    let daily_8hr_km = eff_km_per_hr * 8.0;
    (eff_km_per_hr, daily_8hr_km, terrain.extra_exertion_marks())
}

// ============================================================================
// 2. Encumbrance & Load-Bearing Architecture: Tiers 0–4 (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §2.1–§2.3)
// ============================================================================

/// Ergonomic distribution method for carried equipment
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarryErgonomicMethod {
    /// **Tailored Clothing / Articulated Body Armor / Duty Belt**: `×0.8`
    TailoredArmorOrDutyBelt,
    /// **Standard Tactical Webbing / Balanced Internal-Frame Rucksack**: `×1.0`
    BalancedWebbingOrRucksack,
    /// **Slung Bag / Asymmetric Duffel / Unbalanced Pack**: `×1.3`
    SlungOrAsymmetricPack,
    /// **Carried in Hands / Fireman-Carrying a Body**: `×1.5`
    CarriedInHandsOrBody,
}

impl CarryErgonomicMethod {
    /// Canonical ergonomic mass multiplier (`0.8`, `1.0`, `1.3`, `1.5`,
    /// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §2.2).
    #[must_use]
    pub const fn ergonomic_factor(self) -> f64 {
        match self {
            Self::TailoredArmorOrDutyBelt => 0.8,
            Self::BalancedWebbingOrRucksack => 1.0,
            Self::SlungOrAsymmetricPack => 1.3,
            Self::CarriedInHandsOrBody => 1.5,
        }
    }
}

/// A carried item or load group with physical mass (`kg`) and [`CarryErgonomicMethod`]
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CarriedLoadItem {
    pub mass_kg: f64,
    pub carry_method: CarryErgonomicMethod,
}

/// The 5 Canonical Encumbrance Tiers (`Tiers 0–4`) from `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §2.3 Table,
/// unified with [`crate::action_economy::EncumbranceTier`].
pub type LoadEncumbranceTier = EncumbranceTier;

/// Complete evaluation of a character's Base Comfort Load ($L_{\text{base}}$), Effective Load ($L_{\text{eff}}$),
/// and resulting [`LoadEncumbranceTier`] (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §2.1–§2.3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EncumbranceEvaluation {
    /// Base Uninhibited Comfort Load $L_{\text{base}} = (\text{MAS}/70.0) \times (\text{FOR} \times 1.2\text{ kg})$.
    pub base_comfort_load_kg: f64,
    pub raw_carried_mass_kg: f64,
    /// Effective Load $L_{\text{eff}} = \max(0, \sum (\text{Mass}_i \times \text{Ergonomic}_i) - \text{Exo Offset})$.
    pub effective_load_kg: f64,
    pub load_ratio: f64,
    pub tier: LoadEncumbranceTier,
    pub tempo_cap_penalty: i32,
    pub kin_penalty: i32,
    /// Physical `action_time_factor` delta ($\Delta_{\text{enc}}$).
    pub action_time_factor_delta: f64,
    pub can_sprint: bool,
    /// Maximum deadlift / drag limit in kilograms ($8.0 \times L_{\text{base}}$).
    pub max_deadlift_drag_limit_kg: f64,
}

/// Computes Base Comfort Load $L_{\text{base}} = (\text{MAS}/70.0) \times (\text{FOR} \times 1.2\text{ kg})$
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §2.1).
#[must_use]
pub fn compute_base_comfort_load_kg(mas_kg: f64, for_stat: i32) -> f64 {
    let mas = mas_kg.max(20.0);
    let f = f64::from(for_stat.clamp(1, 25));
    (mas / 70.0) * (f * 1.2)
}

/// Evaluates a character's carried items and powered exoskeleton offset against their Base Comfort Load ($L_{\text{base}}$)
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §2.1–§2.3).
#[must_use]
pub fn evaluate_encumbrance_load(
    mas_kg: f64,
    for_stat: i32,
    items: &[CarriedLoadItem],
    exo_frame_offset_kg: f64,
) -> EncumbranceEvaluation {
    let base_comfort_load_kg = compute_base_comfort_load_kg(mas_kg, for_stat);
    let raw_carried_mass_kg: f64 = items.iter().map(|i| i.mass_kg.max(0.0)).sum();
    let weighted_sum: f64 = items
        .iter()
        .map(|i| i.mass_kg.max(0.0) * i.carry_method.ergonomic_factor())
        .sum();
    let effective_load_kg = (weighted_sum - exo_frame_offset_kg.max(0.0)).max(0.0);
    let load_ratio = if base_comfort_load_kg > 0.0 {
        effective_load_kg / base_comfort_load_kg
    } else {
        0.0
    };

    let tier = LoadEncumbranceTier::from_load_ratio(load_ratio);

    EncumbranceEvaluation {
        base_comfort_load_kg,
        raw_carried_mass_kg,
        effective_load_kg,
        load_ratio,
        tier,
        tempo_cap_penalty: tier.tempo_cap_penalty(),
        kin_penalty: tier.kin_penalty(),
        action_time_factor_delta: tier.action_time_factor_delta(),
        can_sprint: tier.can_sprint(),
        max_deadlift_drag_limit_kg: 8.0 * base_comfort_load_kg,
    }
}

impl ConditionSource for EncumbranceEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        self.tier.to_condition_impact()
    }
}

// ============================================================================
// 3. Vehicles, High-Speed Chases & Crash / G-Force Physics (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §3.1–§3.3)
// ============================================================================

/// Chase Range Bands in a vehicular pursuit or aerial dogfight
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChaseRangeBand {
    /// **Bumper-to-Bumper / Contact** (`0–3 m`): Ramming, PIT maneuvers, sideswiping, boarding.
    BumperToBumper,
    /// **Close Range** (`~15 m`): CQB window fire, tight urban weaving.
    Close15m,
    /// **Medium Range** (`~50 m`): Standard highway/street pursuit distance.
    Medium50m,
    /// **Long Range** (`150m+`): High-speed standoff pursuit; requires visual/sensor lock.
    Long150mPlus,
    /// **Lost Visual / Escaped**: Pursuit broken unless reacquired by aerial drone or comms net.
    LostVisual,
}

impl ChaseRangeBand {
    /// Shifts the range band by `steps` (`-1` closes distance toward `BumperToBumper`, `+1` opens distance toward `LostVisual`,
    /// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §3.2).
    #[must_use]
    pub const fn shift(self, steps: i32) -> Self {
        const BANDS: [ChaseRangeBand; 5] = [
            ChaseRangeBand::BumperToBumper,
            ChaseRangeBand::Close15m,
            ChaseRangeBand::Medium50m,
            ChaseRangeBand::Long150mPlus,
            ChaseRangeBand::LostVisual,
        ];
        let idx = match self {
            Self::BumperToBumper => 0,
            Self::Close15m => 1,
            Self::Medium50m => 2,
            Self::Long150mPlus => 3,
            Self::LostVisual => 4,
        };
        let sum = idx + steps;
        let next = if sum < 0 {
            0
        } else if sum > 4 {
            4
        } else {
            sum as usize
        };
        BANDS[next]
    }
}

/// Mechanical Envelope Ceilings of a ground, maritime, aerial, or orbital vehicle
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §3.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleEnvelopeProfile {
    pub name: String,
    pub caps: Triad<i32>,
    pub mass_kg: f64,
    pub armor_pr: i32,
}

/// Outcome of a Vehicular Chase Exchange
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §3.1–§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VehicleChaseOutcome {
    /// Pilot's effective `(tempo, control, force)` triad clamped to the vehicle's envelope ceilings.
    pub effective_pilot_triad: Triad<i32>,
    pub next_band: ChaseRangeBand,
    pub pit_or_ram_succeeded: bool,
}

/// Evaluates a driver/pilot's Effective Vehicular Triad Ceilings (bounded by both pilot attributes and the
/// [`VehicleEnvelopeProfile`]) and resolves a chase exchange
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §3.1–§3.2).
#[must_use]
pub fn evaluate_vehicle_chase_exchange(
    vehicle: &VehicleEnvelopeProfile,
    pilot: Triad<i32>,
    opponent: Triad<i32>,
    current_band: ChaseRangeBand,
    wants_to_close_distance: bool,
    attempts_pit_or_ram: bool,
) -> VehicleChaseOutcome {
    let eff = Triad::new(
        pilot.tempo.clamp(0, vehicle.caps.tempo),
        pilot.control.clamp(0, vehicle.caps.control),
        pilot.force.clamp(0, vehicle.caps.force),
    );
    let opp = opponent.clamp_min(0);

    let net_tempo = eff.tempo - opp.tempo;
    let band_shift = if net_tempo >= 3 {
        if wants_to_close_distance { -1 } else { 1 }
    } else if net_tempo <= -3 {
        if wants_to_close_distance { 1 } else { -1 }
    } else {
        0
    };
    let next_band = current_band.shift(band_shift);

    let leveraged_pit_force = eff.force + (eff.control - opp.control).max(0);
    let pit_or_ram_succeeded = attempts_pit_or_ram
        && next_band == ChaseRangeBand::BumperToBumper
        && leveraged_pit_force >= opp.force.max(1);

    VehicleChaseOutcome {
        effective_pilot_triad: eff,
        next_band,
        pit_or_ram_succeeded,
    }
}

/// Evaluates **Collision Kinetic Trauma** and **High-G Blackout (`G-LOC`)**
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §3.3).
///
/// Returns `(raw_collision_blunt_trauma, g_loc_delta_ext, suffers_tunnel_vision_dazzled, suffers_g_loc_unconscious)`.
#[must_use]
pub fn evaluate_collision_and_g_force(
    relative_velocity_m_per_sec: f64,
    striking_mass_kg: f64,
    target_mass_kg: f64,
    has_crash_harness_or_airbag: bool,
    sustained_g_load: f64,
    wears_anti_g_suit: bool,
    crew_hlt_fit_check_margin: i32,
) -> (i32, i32, bool, bool) {
    let mass_ratio = (striking_mass_kg.max(1.0) / target_mass_kg.max(1.0)).clamp(0.25, 4.0);
    let raw_kinetic = (relative_velocity_m_per_sec.max(0.0) * 0.5 * mass_ratio).round() as i32;
    let collision_trauma = if has_crash_harness_or_airbag {
        (raw_kinetic - 6).max(0)
    } else {
        raw_kinetic
    };

    let over_g = (sustained_g_load - 5.0).max(0.0);
    let suit_relief = if wears_anti_g_suit { 4 } else { 0 };
    let g_loc_delta_ext = if sustained_g_load > 5.0 {
        ((over_g * 2.0).round() as i32 - suit_relief).max(0)
    } else {
        0
    };

    let (suffers_tunnel_vision_dazzled, suffers_g_loc_unconscious) = if sustained_g_load <= 5.0 {
        (false, false)
    } else if crew_hlt_fit_check_margin < -1 {
        (true, true)
    } else if crew_hlt_fit_check_margin < 0 {
        (true, false)
    } else {
        (false, false)
    };

    (
        collision_trauma,
        g_loc_delta_ext,
        suffers_tunnel_vision_dazzled,
        suffers_g_loc_unconscious,
    )
}

