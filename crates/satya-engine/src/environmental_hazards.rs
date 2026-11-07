//! # Environmental Survival Hazards: Dehydration, Starvation, Thermal Stress, Sleep, Hypoxia, Toxins & Radiation
//! (docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md §4.1–§4.4).
//!
//! Separated from [crate::environment_vehicles] (§1–§3 Movement, Encumbrance & Vehicles)
//! so both modules remain compact and focused.

use serde::{Deserialize, Serialize};

use crate::health_survival::{PhysiologicalStatusCondition, StructuralWoundTier};
use crate::impl_static_catalog;
use crate::primitives::{ConditionImpact, ConditionSource, StaticCatalog, Triad};
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
    pub daily_requirement_liters: f64,
    pub water_deficit_liters: f64,
    pub stage: DehydrationStage,
    /// Internal difficulty modifier $\Delta_{\text{int}}$ (`0`, `2`, `4`, `8`, `10`).
    pub delta_int: i32,
    /// Minimum `fatigue_level` floor that cannot be removed by rest until rehydrated (`0`, `1`, `2`, `4`, `5`).
    pub fatigue_level_floor: i32,
    pub fit_penalty: i32,
    pub foc_penalty: i32,
    pub spd_hlt_ana_penalty: i32,
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
    pub glycogen_foc_fit_penalty: i32,
    pub catabolic_intervals: i32,
    pub mass_lost_kg: f64,
    pub for_fit_hlt_erosion: i32,
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
    pub dex_tac_penalty: i32,
    /// Broad attribute penalty (`SPD`, `KIN`, `ANA`, `FOC`).
    pub all_stat_penalty: i32,
    pub tempo_and_force_cap_penalty: i32,
    pub action_time_factor_added: f64,
    pub added_fatigue_level: i32,
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
    pub hours_awake: i32,
    /// Internal difficulty modifier $\Delta_{\text{int}}$ (`0`, `+2` at `24h`, `+4` at `48h`, `+8` at `72h+`).
    pub delta_int: i32,
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
    pub fit_penalty: i32,
    pub acute_mountain_sickness_risk: bool,
    pub vacuum_useful_consciousness_ticks: i32,
    pub vacuum_unconscious: bool,
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
    Inhalation,
    Contact,
    Ingestion,
    InjectionOrWound,
}

/// Canonical Toxin or Chemical Agent identifier
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.3).
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxinAgentId {
    TearGasCsRiotAgent = 0,
    SarinNerveAgent = 1,
}

impl ToxinAgentId {
    pub const ALL: [Self; 2] = [Self::TearGasCsRiotAgent, Self::SarinNerveAgent];
}

/// Canonical 4-Parameter Toxin, Chemical Agent, or Pathogen profile
/// (`docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §4.4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToxinOrPathogenProfile {
    pub id: ToxinAgentId,
    pub name: &'static str,
    pub vector: ToxinVector,
    pub onset_delay_ticks: i32,
    /// Parameter 3: Virulence ($\Delta_{\text{ext}}$ on the victim's `HLT` cascade save).
    pub virulence_delta_ext: i32,
    pub target_conditions: &'static [PhysiologicalStatusCondition],
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
    pub tier: RadiationExposureTier,
    /// Subtractive penalty to `HLT`.
    pub hlt_penalty: i32,
    /// Whether `HLT` is halved against infection over `2–6 weeks`.
    pub hlt_halved_vs_infection: bool,
    pub added_fatigue_level: i32,
    pub inflicts_nausea: bool,
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
