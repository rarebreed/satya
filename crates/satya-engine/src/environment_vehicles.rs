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

use crate::action_economy::EncumbranceTier;
use crate::health_survival::{PhysiologicalStatusCondition, StructuralWoundTier};
use crate::impl_static_catalog;
use crate::primitives::{ConditionImpact, ConditionSource, StaticCatalog, Triad};

// ============================================================================
// 1. Movement Architecture, Combat Gaits, Special Modes & Overland (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.1–§1.4)
// ============================================================================

/// Baseline Stride Factor ($\text{SF}$) and Sprint Velocity ($V_{\text{sprint}}$)
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SprintVelocityMetrics {
    /// Stride Factor $\text{SF} = \text{HGT} / 1.75\text{ m}$.
    pub stride_factor_sf: f64,
    /// Maximum Sprint Velocity $V_{\text{sprint}} = (4.0 + 0.35 \times \text{SPD}_{\text{eff}}) \times \text{SF}$ in meters per second (`m/s`).
    pub v_sprint_m_per_sec: f64,
    /// Maximum Sprint Velocity in kilometers per hour (`km/h`).
    pub v_sprint_km_per_hr: f64,
    /// Maximum Sprint Velocity in meters per Tick (`m/Tick`, where `1 second = 4 Ticks`).
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
}

/// Complete combat movement and triad interaction metrics for a chosen [`CombatMovementGait`]
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.2 Table).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CombatGaitEvaluation {
    /// Chosen gait mode.
    pub gait: CombatMovementGait,
    /// Speed in meters per second (`m/s`).
    pub speed_m_per_sec: f64,
    /// Speed in meters per Tick (`m/Tick`, `1 Tick = 0.25s`).
    pub speed_m_per_tick: f64,
    /// Distance covered over `10 Ticks` (`2.5s`) in meters.
    pub distance_per_10_ticks_m: f64,
    /// Ranged weapon Recoil (`R`) adder (`+0`, `+2`, `+4`).
    pub ranged_recoil_adder: i32,
    /// Effective Ranged `Control Cap` after gait constraints ($\lfloor C_{\max}/2\rfloor$ in `BoundingRunHustle`, `2` in `AllOutSprint`).
    pub effective_ranged_control_cap: i32,
    /// Whether aimed ranged shots are permitted (`false` in `AllOutSprint`).
    pub allows_aimed_ranged_fire: bool,
    /// Base Exertion Marks accumulated per `20 Ticks` (`5 seconds`).
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
    /// Maximum Wall Scaling / Ledge Grab height (`running_high_jump_no_hands_m + 0.8 * HGT`).
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
    /// Physical mass in kilograms (`kg`).
    pub mass_kg: f64,
    /// How the weight is worn or carried on the body.
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
    /// Unmodified total physical mass carried in kilograms.
    pub raw_carried_mass_kg: f64,
    /// Effective Load $L_{\text{eff}} = \max(0, \sum (\text{Mass}_i \times \text{Ergonomic}_i) - \text{Exo Offset})$.
    pub effective_load_kg: f64,
    /// Load Ratio $L_{\text{eff}} / L_{\text{base}}$.
    pub load_ratio: f64,
    /// Derived Encumbrance Tier (`Tier 0..=4`).
    pub tier: LoadEncumbranceTier,
    /// Penalty to Tempo Cap ($T_{\max}$).
    pub tempo_cap_penalty: i32,
    /// Penalty to `KIN`.
    pub kin_penalty: i32,
    /// Physical `action_time_factor` delta ($\Delta_{\text{enc}}$).
    pub action_time_factor_delta: f64,
    /// Whether the character can perform an All-Out Sprint (`false` at `Tier 3+`).
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
        can_sprint: tier.tier_number() < 3,
        max_deadlift_drag_limit_kg: 8.0 * base_comfort_load_kg,
    }
}

impl ConditionSource for EncumbranceEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_ext: self.tier.stealth_penalty_delta_ext(),
            action_time_factor_delta: self.action_time_factor_delta,
            triad_cap_penalty: Triad::new(self.tempo_cap_penalty, 0, 0),
            ..ConditionImpact::ZERO
        }
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
    /// Display name of the vehicle.
    pub name: String,
    /// Vehicular Triad ceilings `(tempo_cap, control_cap, force_cap)`.
    pub caps: Triad<i32>,
    /// Vehicle mass in kilograms (`kg`).
    pub mass_kg: f64,
    /// Armor Protection Rating (`PR`) of the chassis.
    pub armor_pr: i32,
}

/// Outcome of a Vehicular Chase Exchange
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §3.1–§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VehicleChaseOutcome {
    /// Pilot's effective `(tempo, control, force)` triad clamped to the vehicle's envelope ceilings.
    pub effective_pilot_triad: Triad<i32>,
    /// Updated [`ChaseRangeBand`] after the exchange.
    pub next_band: ChaseRangeBand,
    /// Whether an attempted PIT maneuver or ram succeeded at `BumperToBumper` range.
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

// ============================================================================
// 4. Environment & Survival Systems (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.1–§4.4)
// ============================================================================

/// Climate & operational activity level governing Daily Water Requirement ($W_{\text{req}}$,
/// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HydrationEnvironment {
    /// **Temperate Climate, Low Activity**: `3.0 Liters / Day`
    TemperateLowActivity,
    /// **Heavy Physical Labor / Forced March**: `5.0 Liters / Day`
    HeavyLaborOrForcedMarch,
    /// **Hot / Arid Desert or Tropical Jungle (`> 35°C`) + Tactical Gear**: `9.0 Liters / Day` (`8.0–10.0 L/day`)
    HotAridDesertOrJungleTactical,
}

impl HydrationEnvironment {
    /// Canonical Daily Water Requirement ($W_{\text{req}}$) in Liters (`3.0`, `5.0`, `9.0`,
    /// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.1).
    #[must_use]
    pub const fn daily_water_requirement_liters(self) -> f64 {
        match self {
            Self::TemperateLowActivity => 3.0,
            Self::HeavyLaborOrForcedMarch => 5.0,
            Self::HotAridDesertOrJungleTactical => 9.0,
        }
    }
}

/// The Five Stages of Dehydration (`Stages 0–4`,
/// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.1 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DehydrationStage {
    /// **Stage 0 (`< 2%`)**: Euhydrated baseline (`0` $\Delta_{\text{int}}$, `0` fatigue floor).
    Stage0Euhydrated,
    /// **Stage 1: Thirsty (`2–4%`)**: `+2` $\Delta_{\text{int}}$, `-1 FIT`, `-1 FOC`, `+1` `fatigue_level` floor.
    Stage1Thirsty,
    /// **Stage 2: Parched (`5–7%`)**: `+4` $\Delta_{\text{int}}$, `-3 FIT`, `-2 SPD/HLT/ANA`, `-2 KIN` (`Dizzy`), `+2` `fatigue_level` floor.
    Stage2Parched,
    /// **Stage 3: Severe (`8–10%`)**: `+8` $\Delta_{\text{int}}$, `-6 FIT/HLT/ANA/FOC`, `+4` `fatigue_level` floor, roll `HLT` every `4 hrs` or `Unconscious`.
    Stage3Severe,
    /// **Stage 4: Lethal (`> 12%`)**: `+10` $\Delta_{\text{int}}$, Comatose (`All Caps = 0`), fatal within `1–6 hours` without IV resuscitation.
    Stage4Lethal,
}

/// Mechanical evaluation of a character's Hydration / Dehydration state
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.1 Table).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DehydrationEvaluation {
    /// Daily water requirement $W_{\text{req}}$ in Liters.
    pub daily_requirement_liters: f64,
    /// Unreplaced water deficit in Liters.
    pub water_deficit_liters: f64,
    /// Resulting Dehydration Stage (`0..=4`).
    pub stage: DehydrationStage,
    /// Internal difficulty modifier $\Delta_{\text{int}}$ (`0`, `2`, `4`, `8`, `10`).
    pub delta_int: i32,
    /// Minimum `fatigue_level` floor that cannot be removed by rest until rehydrated (`0`, `1`, `2`, `4`, `5`).
    pub fatigue_level_floor: i32,
    /// Penalty to `FIT` (`0`, `-1`, `-3`, `-6`).
    pub fit_penalty: i32,
    /// Penalty to `FOC` (`0`, `-1`, `0`, `-6`).
    pub foc_penalty: i32,
    /// Penalty to `SPD`, `HLT`, `ANA` (`0`, `0`, `-2`, `-6`).
    pub spd_hlt_ana_penalty: i32,
    /// Whether the character is Comatose (`All Caps = 0`) at Stage 4.
    pub is_comatose: bool,
}

/// Evaluates a character's [`DehydrationStage`] from their daily water requirement ($W_{\text{req}}$) and
/// unreplaced water deficit (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.1).
///
/// Advances `+1` stage for every `25%` of Daily Water Requirement missed plus `+1` stage for every `2.0 Liters`
/// of unreplaced extra sweat deficit.
#[must_use]
pub fn evaluate_dehydration(
    env: HydrationEnvironment,
    water_consumed_liters: f64,
    extra_sweat_deficit_liters: f64,
) -> DehydrationEvaluation {
    let req = env.daily_water_requirement_liters();
    let daily_missed = (req - water_consumed_liters.max(0.0)).max(0.0);
    let sweat_missed = extra_sweat_deficit_liters.max(0.0);
    let missed = daily_missed + sweat_missed;
    let stages_by_pct = ((daily_missed / (0.25 * req)) + 1e-9).floor() as i32;
    let stages_by_sweat = ((sweat_missed / 2.0) + 1e-9).floor() as i32;
    let stage_num = (stages_by_pct + stages_by_sweat).clamp(0, 4);

    let (
        stage,
        delta_int,
        fatigue_level_floor,
        fit_penalty,
        foc_penalty,
        spd_hlt_ana_penalty,
        is_comatose,
    ) = match stage_num {
        0 => (DehydrationStage::Stage0Euhydrated, 0, 0, 0, 0, 0, false),
        1 => (DehydrationStage::Stage1Thirsty, 2, 1, 1, 1, 0, false),
        2 => (DehydrationStage::Stage2Parched, 4, 2, 3, 0, 2, false),
        3 => (DehydrationStage::Stage3Severe, 8, 4, 6, 6, 6, false),
        _ => (DehydrationStage::Stage4Lethal, 10, 5, 10, 10, 10, true),
    };

    DehydrationEvaluation {
        daily_requirement_liters: req,
        water_deficit_liters: missed,
        stage,
        delta_int,
        fatigue_level_floor,
        fit_penalty,
        foc_penalty,
        spd_hlt_ana_penalty,
        is_comatose,
    }
}

impl ConditionSource for DehydrationEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.delta_int,
            fatigue_floor: self.fatigue_level_floor,
            triad_locked_zero: Triad::splat(self.is_comatose),
            is_unconscious: self.is_comatose,
            ..ConditionImpact::ZERO
        }
    }
}

/// Enforces all active **Fatigue Floors** (from Dehydration, Starvation, and Sleep Deprivation) so that
/// rest cannot reduce `fatigue_level` below the highest active floor
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.1).
#[must_use]
pub fn enforce_fatigue_floor(current_fatigue_level: i32, active_floors: &[i32]) -> i32 {
    let max_floor = active_floors.iter().copied().max().unwrap_or(0).clamp(0, 5);
    current_fatigue_level.max(max_floor).clamp(0, 5)
}

/// Evaluation of Short-Term Fasting and Prolonged Caloric Deficit / Starvation
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StarvationEvaluation {
    /// Internal difficulty modifier $\Delta_{\text{int}}$.
    pub delta_int: i32,
    /// Minimum `fatigue_level` floor (`+1` at `48h+` without food).
    pub fatigue_level_floor: i32,
    /// Penalty to `FOC` and `FIT` from glycogen depletion (`-2` at `48h+`).
    pub glycogen_foc_fit_penalty: i32,
    /// Number of 3-day / `10,000 kcal` catabolic starvation intervals suffered (`Days 4+`).
    pub catabolic_intervals: i32,
    /// Cumulative Body Mass (`MAS`) lost in kilograms (`1.5 kg` per catabolic interval).
    pub mass_lost_kg: f64,
    /// Cumulative erosion to `FOR`, `FIT`, and `HLT` (`1` point per catabolic interval).
    pub for_fit_hlt_erosion: i32,
    /// Whether fatal organ failure occurs (`FOR <= 0` or `HLT <= 0`).
    pub fatal_organ_failure: bool,
}

impl ConditionSource for StarvationEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.delta_int,
            fatigue_floor: self.fatigue_level_floor,
            triad_locked_zero: Triad::splat(self.fatal_organ_failure),
            is_unconscious: self.fatal_organ_failure,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates Short-Term Fasting (`Days 1–3`) and Prolonged Starvation (`Days 4–30+` or cumulative `10,000 kcal` deficits)
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.2).
#[must_use]
pub fn evaluate_starvation_and_caloric_deficit(
    hours_without_food: i32,
    cumulative_kcal_deficit: i32,
    dis: i32,
    base_for: i32,
    base_hlt: i32,
) -> StarvationEvaluation {
    let hrs = hours_without_food.max(0);
    let days = hrs.div_euclid(24);

    let (base_delta_int, fatigue_level_floor, glycogen_foc_fit_penalty) = if hrs < 24 {
        (0, 0, 0)
    } else if hrs < 48 {
        (if dis < 10 { 1 } else { 0 }, 0, 0)
    } else {
        (2, 1, 2)
    };

    let intervals_by_days = if days >= 4 { days.div_euclid(3) } else { 0 };
    let intervals_by_kcal = cumulative_kcal_deficit.max(0).div_euclid(10_000);
    let catabolic_intervals = intervals_by_days.max(intervals_by_kcal);

    let weeks = catabolic_intervals.div_euclid(2);
    let delta_int = base_delta_int + 2 * weeks;
    let mass_lost_kg = 1.5 * f64::from(catabolic_intervals);
    let for_fit_hlt_erosion = catabolic_intervals;
    let fatal_organ_failure =
        (base_for - for_fit_hlt_erosion) <= 0 || (base_hlt - for_fit_hlt_erosion) <= 0;

    StarvationEvaluation {
        delta_int,
        fatigue_level_floor,
        glycogen_foc_fit_penalty,
        catabolic_intervals,
        mass_lost_kg,
        for_fit_hlt_erosion,
        fatal_organ_failure,
    }
}

/// The 5 Canonical Clothing / Suit Insulation (`CLO`) Tiers
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3 Table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClothingSuitTier {
    /// **Light Streetwear / Tropical Uniform**: Comfort `+18°C` to `+32°C`, $\Delta_{\text{cold}} = 0$, $\Delta_{\text{heat}} = 0$.
    LightStreetwearTropical,
    /// **Cold-Weather Field Jacket & Layers**: Comfort `-5°C` to `+15°C`, $\Delta_{\text{cold}} = -4$, $\Delta_{\text{heat}} = +4$ (`> 28°C`).
    ColdWeatherFieldJacket,
    /// **Extreme Arctic Parka / ECWCS Level 7**: Comfort `-35°C` to `-5°C`, $\Delta_{\text{cold}} = -8$, $\Delta_{\text{heat}} = +8$ (`> 20°C`).
    ExtremeArcticParkaEcwcs7,
    /// **Sealed NBC / Heavy Ballistic Armor**: Comfort `+10°C` to `+22°C`, $\Delta_{\text{cold}} = -2$, $\Delta_{\text{heat}} = +6$ (blocks sweat evaporation).
    SealedNbcOrHeavyArmor,
    /// **Climate-Controlled EVA / Exo-Rig**: Comfort `-150°C` to `+80°C`, Immune while powered.
    ClimateControlledEvaExoRig,
}

impl ClothingSuitTier {
    /// Comfort zone `(min_temp_c, max_temp_c)` in degrees Celsius
    /// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3 Table).
    #[must_use]
    pub const fn comfort_zone_celsius(self) -> (f64, f64) {
        match self {
            Self::LightStreetwearTropical => (18.0, 32.0),
            Self::ColdWeatherFieldJacket => (-5.0, 15.0),
            Self::ExtremeArcticParkaEcwcs7 => (-35.0, -5.0),
            Self::SealedNbcOrHeavyArmor => (10.0, 22.0),
            Self::ClimateControlledEvaExoRig => (-150.0, 80.0),
        }
    }

    /// Cold Protection Bonus ($\Delta_{\text{cold}}$ applied to $\Delta_{\text{ext}}$ vs. Cold,
    /// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3 Table).
    #[must_use]
    pub const fn cold_protection_delta_ext(self) -> i32 {
        match self {
            Self::LightStreetwearTropical => 0,
            Self::ColdWeatherFieldJacket => -4,
            Self::ExtremeArcticParkaEcwcs7 => -8,
            Self::SealedNbcOrHeavyArmor => -2,
            Self::ClimateControlledEvaExoRig => -20,
        }
    }

    /// Heat Retention Penalty ($\Delta_{\text{heat}}$ applied to $\Delta_{\text{ext}}$ in high heat,
    /// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3 Table).
    #[must_use]
    pub const fn heat_retention_delta_ext(self, ambient_temp_c: f64) -> i32 {
        match self {
            Self::LightStreetwearTropical | Self::ClimateControlledEvaExoRig => 0,
            Self::ColdWeatherFieldJacket => {
                if ambient_temp_c > 28.0 {
                    4
                } else {
                    0
                }
            }
            Self::ExtremeArcticParkaEcwcs7 => {
                if ambient_temp_c > 20.0 {
                    8
                } else {
                    0
                }
            }
            Self::SealedNbcOrHeavyArmor => 6,
        }
    }
}

/// Cold Exposure & Hypothermia stages (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3.A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothermiaStage {
    /// Normothermic (`Core ~37.0°C`)
    None,
    /// **1. Cold Stress & Shivering (`Core 36.0–35.0°C`)**: `DEX` and `TAC` `-3`, `+2` $\Delta_{\text{int}}$, `+1` `fatigue_level`.
    Stage1ColdStressShivering,
    /// **2. Mild-to-Moderate Hypothermia (`Core 34.9–32.0°C`)**: `DEX, TAC, SPD, KIN, ANA, FOC` `-6`, `+0.30` `action_time_factor`, `+6` $\Delta_{\text{int}}$.
    Stage2MildModerateHypothermia,
    /// **3. Severe Hypothermia (`Core < 32.0°C`)**: Delirium, `Unconsciousness`, and ventricular fibrillation risk.
    Stage3SevereHypothermia,
}

/// Heat Exposure, Heat Exhaustion & Heatstroke stages (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3.B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HyperthermiaStage {
    /// Normothermic (`Core ~37.0°C`)
    None,
    /// **Stage 1: Heat Cramps**: `-2` to $T_{\max}$ and $F_{\max}$, `+1` `fatigue_level`.
    Stage1HeatCramps,
    /// **Stage 2: Heat Exhaustion (`Core 38.5–39.9°C`)**: `Dizzy` (`-4 KIN`), `Nausea` (`+4` $\Delta_{\text{int}}$), `+3` `fatigue_level`.
    Stage2HeatExhaustion,
    /// **Stage 3: Heatstroke (`Core >= 40.0°C`)**: CNS delirium, seizures, `Coma`; inflicts `+1` Tier of systemic organ trauma every `15 minutes`.
    Stage3Heatstroke,
}

/// Structured evaluation of Cold (`HypothermiaStage`) and Heat (`HyperthermiaStage`) penalties
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThermalStressEvaluation {
    /// Internal difficulty modifier $\Delta_{\text{int}}$.
    pub delta_int: i32,
    /// Penalty to `DEX` and `TAC`.
    pub dex_tac_penalty: i32,
    /// Broad attribute penalty (`SPD`, `KIN`, `ANA`, `FOC`).
    pub all_stat_penalty: i32,
    /// Penalty to $T_{\max}$ and $F_{\max}$ (`2` when `HyperthermiaStage::Stage1HeatCramps` or higher, `§4.3.B.1`).
    pub tempo_and_force_cap_penalty: i32,
    /// Added `action_time_factor` multiplier delta.
    pub action_time_factor_added: f64,
    /// Added systemic `fatigue_level`.
    pub added_fatigue_level: i32,
    /// Whether the character is unconscious or comatose.
    pub is_unconscious_or_coma: bool,
}

impl ConditionSource for ThermalStressEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.delta_int,
            action_time_factor_delta: self.action_time_factor_added,
            fatigue_added: self.added_fatigue_level,
            triad_cap_penalty: Triad::new(
                self.tempo_and_force_cap_penalty,
                0,
                self.tempo_and_force_cap_penalty,
            ),
            triad_locked_zero: Triad::splat(self.is_unconscious_or_coma),
            is_unconscious: self.is_unconscious_or_coma,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates Cold Exposure (`HypothermiaStage`) or Heat Exposure (`HyperthermiaStage`) penalties
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3).
///
/// Returns `(delta_int, dex_tac_penalty, all_stat_penalty, action_time_factor_added, added_fatigue_level, is_unconscious_or_coma)`.
#[must_use]
pub const fn evaluate_thermal_stage_penalties(
    hypothermia: HypothermiaStage,
    hyperthermia: HyperthermiaStage,
) -> (i32, i32, i32, f64, i32, bool) {
    match (hypothermia, hyperthermia) {
        (HypothermiaStage::Stage3SevereHypothermia, _)
        | (_, HyperthermiaStage::Stage3Heatstroke) => (10, 6, 6, 0.50, 5, true),
        (HypothermiaStage::Stage2MildModerateHypothermia, _) => (6, 6, 6, 0.30, 2, false),
        (_, HyperthermiaStage::Stage2HeatExhaustion) => (4, 0, 4, 0.0, 3, false),
        (HypothermiaStage::Stage1ColdStressShivering, _) => (2, 3, 0, 0.0, 1, false),
        (_, HyperthermiaStage::Stage1HeatCramps) => (0, 0, 0, 0.0, 1, false),
        (HypothermiaStage::None, HyperthermiaStage::None) => (0, 0, 0, 0.0, 0, false),
    }
}

/// Evaluates Cold/Heat stage penalties as a structured [`ThermalStressEvaluation`] implementing [`ConditionSource`]
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3).
#[must_use]
pub const fn evaluate_thermal_stress(
    hypothermia: HypothermiaStage,
    hyperthermia: HyperthermiaStage,
) -> ThermalStressEvaluation {
    let (
        delta_int,
        dex_tac_penalty,
        all_stat_penalty,
        action_time_factor_added,
        added_fatigue_level,
        is_unconscious_or_coma,
    ) = evaluate_thermal_stage_penalties(hypothermia, hyperthermia);
    let tempo_and_force_cap_penalty = match hyperthermia {
        HyperthermiaStage::None => 0,
        HyperthermiaStage::Stage1HeatCramps
        | HyperthermiaStage::Stage2HeatExhaustion
        | HyperthermiaStage::Stage3Heatstroke => 2,
    };
    ThermalStressEvaluation {
        delta_int,
        dex_tac_penalty,
        all_stat_penalty,
        tempo_and_force_cap_penalty,
        action_time_factor_added,
        added_fatigue_level,
        is_unconscious_or_coma,
    }
}

/// Evaluates localized **Frostbite** structural tissue trauma on exposed extremities in sub-zero windchill
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.3.A.4).
///
/// In sub-zero windchill (`< -10.0°C`), exposed skin on a failed `HLT` check suffers localized [`StructuralWoundTier`]
/// damage (`Tier 1` Frostnip down to `Tier 3` / `Tier 4` Deep Frostbite on Fingers/Toes/Face).
#[must_use]
pub fn evaluate_frostbite_exposure(
    effective_windchill_c: f64,
    has_exposed_skin: bool,
    hlt_save_succeeded: bool,
) -> Option<StructuralWoundTier> {
    if !has_exposed_skin || hlt_save_succeeded || effective_windchill_c >= -10.0 {
        None
    } else if effective_windchill_c <= -40.0 {
        Some(StructuralWoundTier::Tier4CriticalMaiming)
    } else if effective_windchill_c <= -25.0 {
        Some(StructuralWoundTier::Tier3SevereFracture)
    } else if effective_windchill_c <= -18.0 {
        Some(StructuralWoundTier::Tier2Moderate)
    } else {
        Some(StructuralWoundTier::Tier1Superficial)
    }
}

/// Evaluation of Sleep Deprivation & Circadian Debt
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SleepDeprivationEvaluation {
    /// Total hours awake without required sleep (`6–8h` standard, `4h` minimum with `DIS` check).
    pub hours_awake: i32,
    /// Internal difficulty modifier $\Delta_{\text{int}}$ (`0`, `+2` at `24h`, `+4` at `48h`, `+8` at `72h+`).
    pub delta_int: i32,
    /// Penalty to `FOC`, `INS`, `ITU`, `OODA` (and `ANA`, `MEM` at `48h+`; all Mental & Sensory at `72h+`).
    pub mental_sensory_ooda_penalty: i32,
    /// Minimum `fatigue_level` floor (`0` at `<48h`, `+2` at `48h+`).
    pub fatigue_level_floor: i32,
    /// Whether being stationary for `> 5 minutes` requires a `DIS` check ($\Delta_{\text{ext}} = +6$) to avoid involuntary Microsleep (`Unconscious`).
    pub requires_microsleep_dis_check: bool,
}

/// Evaluates Sleep Deprivation penalties from `hours_awake`
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.1).
#[must_use]
pub const fn evaluate_sleep_deprivation(hours_awake: i32) -> SleepDeprivationEvaluation {
    let hrs = if hours_awake < 0 { 0 } else { hours_awake };
    let (delta_int, mental_sensory_ooda_penalty, fatigue_level_floor, requires_microsleep_dis_check) =
        match hrs {
            72.. => (8, 6, 2, true),
            48..=71 => (4, 3, 2, false),
            24..=47 => (2, 1, 0, false),
            _ => (0, 0, 0, false),
        };
    SleepDeprivationEvaluation {
        hours_awake: hrs,
        delta_int,
        mental_sensory_ooda_penalty,
        fatigue_level_floor,
        requires_microsleep_dis_check,
    }
}

impl ConditionSource for SleepDeprivationEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.delta_int,
            fatigue_floor: self.fatigue_level_floor,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluation of Altitude Hypoxia and Hard Vacuum Decompression
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AltitudeVacuumEvaluation {
    /// Penalty to `Fitness (FIT)` (`-2` per `1,000 meters` above `2,000 m`, minimum `-2` when unacclimatized above `2,500 m`).
    pub fit_penalty: i32,
    /// Whether Acute Mountain Sickness (`Dizzy`, `Nausea`) and HAPE/HACE checks apply (`> 5,500 m` without $\text{O}_2$).
    pub acute_mountain_sickness_risk: bool,
    /// Useful consciousness window in ticks when exposed to Hard Vacuum (`48–64 Ticks` = `12–16s`).
    pub vacuum_useful_consciousness_ticks: i32,
    /// Whether the character has lost consciousness in Hard Vacuum.
    pub vacuum_unconscious: bool,
    /// Cumulative `HLT` cold/barotrauma erosion suffered after losing consciousness in Hard Vacuum (`-2 HLT` per `20 ticks`).
    pub vacuum_hlt_erosion: i32,
}

impl ConditionSource for AltitudeVacuumEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: if self.acute_mountain_sickness_risk {
                4
            } else {
                0
            },
            triad_locked_zero: Triad::splat(self.vacuum_unconscious),
            is_unconscious: self.vacuum_unconscious,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates High-Altitude Hypoxia (`> 2,500 m`) and Hard Vacuum (`0 atm`) exposure
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.2).
#[must_use]
pub const fn evaluate_altitude_and_vacuum_hypoxia(
    altitude_meters: i32,
    has_supplemental_oxygen_or_acclimatized: bool,
    is_hard_vacuum: bool,
    ticks_in_vacuum: i32,
    fit: i32,
) -> AltitudeVacuumEvaluation {
    if is_hard_vacuum {
        let fit_c = if fit < 1 {
            1
        } else if fit > 20 {
            20
        } else {
            fit
        };
        let useful_ticks = 48 + ((fit_c - 10) * 16).div_euclid(10);
        let clamped_useful = if useful_ticks < 48 {
            48
        } else if useful_ticks > 64 {
            64
        } else {
            useful_ticks
        };
        let t = if ticks_in_vacuum < 0 {
            0
        } else {
            ticks_in_vacuum
        };
        let overrun = if t > clamped_useful {
            t - clamped_useful
        } else {
            0
        };
        return AltitudeVacuumEvaluation {
            fit_penalty: 20,
            acute_mountain_sickness_risk: true,
            vacuum_useful_consciousness_ticks: clamped_useful,
            vacuum_unconscious: t >= clamped_useful,
            vacuum_hlt_erosion: 2 * overrun.div_euclid(20),
        };
    }

    if has_supplemental_oxygen_or_acclimatized || altitude_meters <= 2500 {
        return AltitudeVacuumEvaluation {
            fit_penalty: 0,
            acute_mountain_sickness_risk: false,
            vacuum_useful_consciousness_ticks: 0,
            vacuum_unconscious: false,
            vacuum_hlt_erosion: 0,
        };
    }

    let brackets = (altitude_meters - 2000).div_euclid(1000);
    let min_one_bracket = if brackets < 1 { 1 } else { brackets };

    AltitudeVacuumEvaluation {
        fit_penalty: 2 * min_one_bracket,
        acute_mountain_sickness_risk: altitude_meters > 5500,
        vacuum_useful_consciousness_ticks: 0,
        vacuum_unconscious: false,
        vacuum_hlt_erosion: 0,
    }
}

/// Exposure vector for a Toxin, Venom, Chemical Agent, or Pathogen
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxinVector {
    /// Airborne gas, aerosol, smoke, or spore (`Inhalation`).
    Inhalation,
    /// Percutaneous skin/mucosa absorption (`Contact`).
    Contact,
    /// Swallowed in food or water (`Ingestion`).
    Ingestion,
    /// Dart, fang, needle, or contaminated blade (`InjectionOrWound`).
    InjectionOrWound,
}

/// Canonical Toxin or Chemical Agent identifier
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.3).
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxinAgentId {
    /// **Tear Gas / CS Riot Agent** (`Inhalation`, Onset `4 ticks`, Virulence `+2`).
    TearGasCsRiotAgent = 0,
    /// **Neurotoxin / Sarin (GB) Nerve Agent** (`Inhalation`, Onset `12 ticks`, Virulence `+10`).
    SarinNerveAgent = 1,
}

impl ToxinAgentId {
    /// All canonical toxin identifiers in declaration order
    /// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.3).
    pub const ALL: [Self; 2] = [Self::TearGasCsRiotAgent, Self::SarinNerveAgent];
}

/// Canonical 4-Parameter Toxin, Chemical Agent, or Pathogen profile
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToxinOrPathogenProfile {
    /// Canonical toxin agent identifier.
    pub id: ToxinAgentId,
    /// Agent name (`"Tear Gas (CS)"`, `"Sarin Nerve Agent"`, etc.).
    pub name: &'static str,
    /// Parameter 1: Transmission / Exposure Vector.
    pub vector: ToxinVector,
    /// Parameter 2: Onset delay in Ticks before symptoms take effect.
    pub onset_delay_ticks: i32,
    /// Parameter 3: Virulence ($\Delta_{\text{ext}}$ on the victim's `HLT` cascade save).
    pub virulence_delta_ext: i32,
    /// Parameter 4: Target physiological status effects inflicted on a failed `HLT` save.
    pub target_conditions: &'static [PhysiologicalStatusCondition],
    /// Narrative description of target effects.
    pub effect_summary: &'static str,
}

/// Static catalog of canonical toxin and chemical agent profiles
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.3).
pub const TOXIN_CATALOG: [ToxinOrPathogenProfile; 2] = [
    ToxinOrPathogenProfile {
        id: ToxinAgentId::TearGasCsRiotAgent,
        name: "Tear Gas / CS Riot Agent",
        vector: ToxinVector::Inhalation,
        onset_delay_ticks: 4,
        virulence_delta_ext: 2,
        target_conditions: &[
            PhysiologicalStatusCondition::Blinded,
            PhysiologicalStatusCondition::DazzledBlurredVision,
            PhysiologicalStatusCondition::NauseaSickened,
        ],
        effect_summary: "Onset 4 ticks, Virulence +2, Effect: Blinded/Dazzled + Coughing",
    },
    ToxinOrPathogenProfile {
        id: ToxinAgentId::SarinNerveAgent,
        name: "Neurotoxin / Sarin (GB) Nerve Agent",
        vector: ToxinVector::Inhalation,
        onset_delay_ticks: 12,
        virulence_delta_ext: 10,
        target_conditions: &[
            PhysiologicalStatusCondition::Paralyzed,
            PhysiologicalStatusCondition::UnconsciousComatose,
        ],
        effect_summary: "Onset 12 ticks, Virulence +10, Effect: Paralysis + Asphyxiation + Convulsions",
    },
];

impl_static_catalog!(ToxinAgentId => ToxinOrPathogenProfile : TOXIN_CATALOG);

/// Returns the canonical example Toxin & Chemical Agent profiles from
/// `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.3.
#[must_use]
pub const fn canonical_toxin_catalog() -> &'static [ToxinOrPathogenProfile] {
    ToxinAgentId::CATALOG
}

/// Acute Radiation Sickness (`ARS`) tiers by cumulative Sievert (`Sv`) dose
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadiationExposureTier {
    /// `< 0.5 Sv`: Subclinical (no acute symptoms).
    Subclinical,
    /// `0.5 – 1.5 Sv` (*Mild ARS*): `Nausea` and `+1` `fatigue_level` after `6–12 hours`; temporary `-2 HLT`.
    MildArs,
    /// `1.5 – 4.0 Sv` (*Moderate-Severe Hematopoietic ARS*): Vomiting within `2 hours`, hair loss, `HLT` halved against infection over `2–6 weeks`.
    ModerateSevereHematopoieticArs,
    /// `> 4.0 / 6.0+ Sv` (*Lethal GI / Neurovascular ARS*): Acute shock, internal mucosal hemorrhage, fatal within days without nanomedical regeneration.
    LethalGiNeurovascularArs,
}

/// Structured evaluation of Ionizing Radiation (`Sv`) exposure
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcuteRadiationEvaluation {
    /// Derived Acute Radiation Sickness tier.
    pub tier: RadiationExposureTier,
    /// Subtractive penalty to `HLT`.
    pub hlt_penalty: i32,
    /// Whether `HLT` is halved against infection over `2–6 weeks`.
    pub hlt_halved_vs_infection: bool,
    /// Added systemic `fatigue_level`.
    pub added_fatigue_level: i32,
    /// Whether acute radiation nausea (`+4` $\Delta_{\text{int}}$) is inflicted.
    pub inflicts_nausea: bool,
    /// Whether the dose is lethal without nanomedical regeneration (`> 6.0 Sv`).
    pub is_lethal_without_nanomed: bool,
}

impl ConditionSource for AcuteRadiationEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: if self.inflicts_nausea { 4 } else { 0 },
            fatigue_added: self.added_fatigue_level,
            triad_locked_zero: Triad::splat(self.is_lethal_without_nanomed),
            is_unconscious: self.is_lethal_without_nanomed,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates Ionizing Radiation (`Sv`) exposure tier and physiological penalties
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.4).
///
/// Returns `(tier, hlt_penalty, hlt_halved_vs_infection, added_fatigue_level, inflicts_nausea, is_lethal_without_nanomed)`.
#[must_use]
pub fn evaluate_radiation_exposure(
    cumulative_sieverts_sv: f64,
) -> (RadiationExposureTier, i32, bool, i32, bool, bool) {
    let sv = cumulative_sieverts_sv.max(0.0);
    if sv < 0.5 {
        (
            RadiationExposureTier::Subclinical,
            0,
            false,
            0,
            false,
            false,
        )
    } else if sv <= 1.5 {
        (RadiationExposureTier::MildArs, 2, false, 1, true, false)
    } else if sv <= 4.0 {
        (
            RadiationExposureTier::ModerateSevereHematopoieticArs,
            4,
            true,
            2,
            true,
            false,
        )
    } else {
        (
            RadiationExposureTier::LethalGiNeurovascularArs,
            10,
            true,
            4,
            true,
            sv > 6.0,
        )
    }
}

/// Evaluates Ionizing Radiation (`Sv`) exposure as a structured [`AcuteRadiationEvaluation`] implementing [`ConditionSource`]
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.4).
#[must_use]
pub fn evaluate_acute_radiation(cumulative_sieverts_sv: f64) -> AcuteRadiationEvaluation {
    let (
        tier,
        hlt_penalty,
        hlt_halved_vs_infection,
        added_fatigue_level,
        inflicts_nausea,
        is_lethal_without_nanomed,
    ) = evaluate_radiation_exposure(cumulative_sieverts_sv);
    AcuteRadiationEvaluation {
        tier,
        hlt_penalty,
        hlt_halved_vs_infection,
        added_fatigue_level,
        inflicts_nausea,
        is_lethal_without_nanomed,
    }
}
