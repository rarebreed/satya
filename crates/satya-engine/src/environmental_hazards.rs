//! # Environmental Hazards, Survival, Sepsis & Field Pharmacology (`06-health-and-survival/environmental_hazards_and_survival.md`)
//!
//! Canonical Rust implementation of `docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md`:
//! - **§1 Metabolic & Atmospheric Survival (`§1.1–§1.7`)**: Hydration & Dehydration, Starvation, Thermal Stress (`CLO`),
//!   Sleep Deprivation, Altitude/Vacuum Hypoxia, Toxins/Pathogens, and Ionizing Radiation (`Sv`).
//! - **§2 Falling & Gravity Impact (`§2.1`)**: Impact velocity, breakfall mitigation, and surface hardness trauma.
//! - **§3 Fire, Thermal Burns & Smoke Inhalation (`§3.1–§3.3`)**: Burn degrees, fluid loss, smoke/CO saves, and respirators.
//! - **§4 Electricity & High Voltage (`§4`)**: 4 voltage tiers, tetanic grip lock, wet conductivity, PPE, and EMP reboot ticks.
//! - **§5 Dynamic Weather, Wind, Footing & Diving Pressure (`§5.1–§5.2`)**: Wind ballistic/UAV effects, traction, narcosis, and bends.
//! - **§6 Wound Infection, Sepsis & Field Pharmacology (`§6.1–§6.2`)**: Sepsis staging and 6-agent pharmacology catalog.
//!
//! Re-exported from [crate::environment_vehicles] so downstream crates see the full Part VI surface.

use serde::{Deserialize, Serialize};

use crate::dice_probabilities::ContestedTieRule;
use crate::economy_gear::BlackMarketRestrictionClass;
use crate::health_survival::{
    PhysiologicalStatusCondition, StructuralWoundTier, classify_structural_wound_tier,
};
use crate::primitives::{
    ConditionImpact, ConditionSource, ContestedCheck, StaticCatalog, TieredSaveEval, Triad,
};
use crate::{define_coded_enum, impl_static_catalog};
// ============================================================================
// 4. Environment & Survival Systems (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.1–§4.4)
// ============================================================================

/// Climate & operational activity level governing Daily Water Requirement ($W_{\text{req}}$,
/// `docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.1).
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
    /// `docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.1).
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
/// `docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.1 Table).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.1 Table).
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
/// unreplaced water deficit (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.1).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.1).
#[must_use]
pub fn enforce_fatigue_floor(current_fatigue_level: i32, active_floors: &[i32]) -> i32 {
    let max_floor = active_floors.iter().copied().max().unwrap_or(0).clamp(0, 5);
    current_fatigue_level.max(max_floor).clamp(0, 5)
}

/// Evaluation of Short-Term Fasting and Prolonged Caloric Deficit / Starvation
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.2).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.2).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3 Table).
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
    /// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3 Table).
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
    /// `docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3 Table).
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
    /// `docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3 Table).
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

/// Cold Exposure & Hypothermia stages (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3.A).
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

/// Heat Exposure, Heat Exhaustion & Heatstroke stages (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3.B).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.3.A.4).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.1).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.1).
#[must_use]
pub const fn evaluate_sleep_deprivation(hours_awake: i32) -> SleepDeprivationEvaluation {
    let hrs = if hours_awake < 0 { 0 } else { hours_awake };
    let (
        delta_int,
        mental_sensory_ooda_penalty,
        fatigue_level_floor,
        requires_microsleep_dis_check,
    ) = match hrs {
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.2).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.2).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToxinVector {
    Inhalation,
    Contact,
    Ingestion,
    InjectionOrWound,
}

/// Canonical Toxin or Chemical Agent identifier
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.3).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.3).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.3).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.4).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.4).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.4).
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
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4.4.4).
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

// ============================================================================
// 2. Falling & Gravity Impact Physics (`environmental_hazards_and_survival.md` §2)
// ============================================================================

/// Impact surface stiffness class governing fall trauma multiplier $M_{\text{surface}}$
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandingSurfaceClass {
    /// Deep snow, canvas awning, or car roof crumple zone (`0.50x`).
    DeepSnowAwningCarRoof,
    /// Packed dirt, turf, or deep water (`0.75x`; water hardens to `1.00x` above `15 m` unless clean vertical dive).
    PackedDirtTurfWater,
    /// Hardwood, asphalt, or concrete (`1.00x`).
    HardwoodAsphaltConcrete,
    /// Jagged steel, rebar, or shallow rocky water (`1.35x`).
    JaggedSteelRebarShallow,
}

impl LandingSurfaceClass {
    /// Surface hardness multiplier $M_{\text{surface}}$ (`0.50`, `0.75`, `1.00`, `1.35`).
    #[must_use]
    pub fn hardness_multiplier(self, height_m: f64, clean_water_dive: bool) -> f64 {
        match self {
            Self::DeepSnowAwningCarRoof => 0.50,
            Self::PackedDirtTurfWater => {
                if height_m > 15.0 && !clean_water_dive {
                    1.00
                } else {
                    0.75
                }
            }
            Self::HardwoodAsphaltConcrete => 1.00,
            Self::JaggedSteelRebarShallow => 1.35,
        }
    }
}

/// Evaluation of falling velocity, breakfall height reduction, and resulting blunt trauma
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FallImpactEvaluation {
    pub height_m: f64,
    pub effective_height_m: f64,
    pub impact_velocity_m_per_s: f64,
    pub surface: LandingSurfaceClass,
    pub surface_multiplier: f64,
    pub raw_fall_trauma: i32,
    pub wound_tier: Option<StructuralWoundTier>,
    pub wound_shock_added: i32,
    pub redirected_off_vitals: bool,
    pub laceration_bleed_risk: bool,
}

impl ConditionSource for FallImpactEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        let cap_pen = self.wound_tier.map_or(0, |t| t.aspect_ceiling_penalty());
        let locked = self
            .wound_tier
            .is_some_and(StructuralWoundTier::locks_ceiling_to_zero);
        ConditionImpact {
            wound_shock_added: self.wound_shock_added,
            triad_cap_penalty: Triad::splat(cap_pen),
            triad_locked_zero: Triad::splat(locked),
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates gravitational fall impact velocity ($v = \sqrt{2gh}$), breakfall height reduction
/// (`1.5 m` per net *Parkour/Acrobatics* success), and resulting [`StructuralWoundTier`]
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §2.1).
#[must_use]
pub fn evaluate_fall_impact(
    height_m: f64,
    surface: LandingSurfaceClass,
    breakfall_net_successes: i32,
    clean_water_dive: bool,
) -> FallImpactEvaluation {
    let h = height_m.max(0.0);
    let impact_velocity_m_per_s = (2.0 * 9.81 * h).sqrt();
    let breakfall_reduction = 1.5 * f64::from(breakfall_net_successes.max(0));
    let effective_height_m = (h - breakfall_reduction).max(0.0);
    let surface_multiplier = surface.hardness_multiplier(h, clean_water_dive);
    let raw_fall_trauma = if effective_height_m <= 1.5 {
        0
    } else {
        (((effective_height_m - 1.5) * 1.25 * surface_multiplier).round() as i32).max(1)
    };
    let wound_tier = classify_structural_wound_tier(raw_fall_trauma);
    let wound_shock_added = wound_tier.map_or(0, |t| t.wound_shock_added(10));

    FallImpactEvaluation {
        height_m: h,
        effective_height_m,
        impact_velocity_m_per_s,
        surface,
        surface_multiplier,
        raw_fall_trauma,
        wound_tier,
        wound_shock_added,
        redirected_off_vitals: breakfall_net_successes >= 1,
        laceration_bleed_risk: surface == LandingSurfaceClass::JaggedSteelRebarShallow
            && raw_fall_trauma >= 3,
    }
}

// ============================================================================
// 3. Fire, Thermal Burns & Smoke Inhalation (`environmental_hazards_and_survival.md` §3)
// ============================================================================

/// Fire exposure source class (`environmental_hazards_and_survival.md` §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FireExposureSource {
    FlashOrMolotovSplash,
    OpenFlameEngulfed,
    IndustrialOrEvThermalRunaway,
}

impl FireExposureSource {
    /// Base raw thermal trauma (`3` brief flash, `4` per 4 ticks open flame, `7` per 4 ticks EV/industrial jet).
    #[must_use]
    pub const fn base_raw_trauma(self) -> i32 {
        match self {
            Self::FlashOrMolotovSplash => 3,
            Self::OpenFlameEngulfed => 4,
            Self::IndustrialOrEvThermalRunaway => 7,
        }
    }

    /// Whether the fire source persists every `4 Ticks` until actively extinguished.
    #[must_use]
    pub const fn persists_until_extinguished(self) -> bool {
        !matches!(self, Self::FlashOrMolotovSplash)
    }
}

/// Clinical thermal burn degree (`environmental_hazards_and_survival.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BurnDegree {
    None,
    FirstDegreeSuperficial,
    SecondDegreePartialThickness,
    ThirdDegreeFullThickness,
}

/// Mechanical evaluation of thermal burn trauma, dehydration loss, and sepsis risk
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BurnEvaluation {
    pub degree: BurnDegree,
    pub wound_tier: Option<StructuralWoundTier>,
    pub delta_int: i32,
    pub wound_shock_added: i32,
    pub dehydration_stages_added: i32,
    pub sepsis_delta_ext: i32,
    pub locks_limb_zero: bool,
}

impl ConditionSource for BurnEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        let cap_pen = self.wound_tier.map_or(0, |t| t.aspect_ceiling_penalty());
        ConditionImpact {
            delta_int: self.delta_int,
            wound_shock_added: self.wound_shock_added,
            triad_cap_penalty: Triad::splat(cap_pen),
            triad_locked_zero: Triad::splat(self.locks_limb_zero),
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates [`BurnDegree`], [`StructuralWoundTier`], dehydration stage escalation, and sepsis risk
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §3.2).
#[must_use]
pub const fn evaluate_burn_trauma(raw_thermal_trauma: i32, burned_zones: i32) -> BurnEvaluation {
    let zones = if burned_zones < 0 { 0 } else { burned_zones };
    if raw_thermal_trauma <= 0 {
        BurnEvaluation {
            degree: BurnDegree::None,
            wound_tier: None,
            delta_int: 0,
            wound_shock_added: 0,
            dehydration_stages_added: 0,
            sepsis_delta_ext: 0,
            locks_limb_zero: false,
        }
    } else if raw_thermal_trauma <= 2 {
        BurnEvaluation {
            degree: BurnDegree::FirstDegreeSuperficial,
            wound_tier: Some(StructuralWoundTier::Tier1Superficial),
            delta_int: 1,
            wound_shock_added: 0,
            dehydration_stages_added: 0,
            sepsis_delta_ext: 0,
            locks_limb_zero: false,
        }
    } else if raw_thermal_trauma <= 6 {
        BurnEvaluation {
            degree: BurnDegree::SecondDegreePartialThickness,
            wound_tier: classify_structural_wound_tier(raw_thermal_trauma),
            delta_int: 2,
            wound_shock_added: 1,
            dehydration_stages_added: zones.div_euclid(2),
            sepsis_delta_ext: 4,
            locks_limb_zero: false,
        }
    } else {
        BurnEvaluation {
            degree: BurnDegree::ThirdDegreeFullThickness,
            wound_tier: Some(StructuralWoundTier::Tier4CriticalMaiming),
            delta_int: 4,
            wound_shock_added: 3,
            dehydration_stages_added: if zones < 1 { 1 } else { zones },
            sepsis_delta_ext: 8,
            locks_limb_zero: true,
        }
    }
}

/// Smoke / toxic gas density tier (`environmental_hazards_and_survival.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmokeDensityTier {
    LightHaze,
    HeavyStructuralSmoke,
    ToxicChemicalOrLithiumFumes,
}

/// Respiratory protection worn against smoke and toxic fumes (`environmental_hazards_and_survival.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RespiratoryProtectionTier {
    None,
    WetCloth,
    ParticulateOrCbrnRespirator,
    SelfContainedScba,
}

/// Evaluation of smoke inhalation / carbon monoxide exposure per `20-Tick` interval
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmokeInhalationEvaluation {
    pub effective_save_delta_ext: i32,
    pub is_immune: bool,
    pub delta_int: i32,
    pub fatigue_added: i32,
    pub inflicts_dazzled: bool,
    pub inflicts_nausea: bool,
}

impl ConditionSource for SmokeInhalationEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.delta_int,
            fatigue_added: self.fatigue_added,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates smoke inhalation save difficulty ($\Delta_{\text{ext}}$), respirator immunity, and acute
/// effects when a `20-Tick` `HLT` save fails (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §3.3).
#[must_use]
pub const fn evaluate_smoke_inhalation(
    density: SmokeDensityTier,
    protection: RespiratoryProtectionTier,
    hlt_save_failed: bool,
) -> SmokeInhalationEvaluation {
    let base_ext = match density {
        SmokeDensityTier::LightHaze => 0,
        SmokeDensityTier::HeavyStructuralSmoke => 4,
        SmokeDensityTier::ToxicChemicalOrLithiumFumes => 8,
    };
    let mask_relief = match protection {
        RespiratoryProtectionTier::None => 0,
        RespiratoryProtectionTier::WetCloth => -2,
        RespiratoryProtectionTier::ParticulateOrCbrnRespirator => -6,
        RespiratoryProtectionTier::SelfContainedScba => -20,
    };
    let is_immune = matches!(protection, RespiratoryProtectionTier::SelfContainedScba)
        || (matches!(
            protection,
            RespiratoryProtectionTier::ParticulateOrCbrnRespirator
        ) && !matches!(density, SmokeDensityTier::ToxicChemicalOrLithiumFumes));
    let raw_eff = base_ext + mask_relief;
    let effective_save_delta_ext = if raw_eff < 0 { 0 } else { raw_eff };

    if is_immune || !hlt_save_failed {
        return SmokeInhalationEvaluation {
            effective_save_delta_ext,
            is_immune,
            delta_int: 0,
            fatigue_added: 0,
            inflicts_dazzled: false,
            inflicts_nausea: false,
        };
    }

    match density {
        SmokeDensityTier::LightHaze => SmokeInhalationEvaluation {
            effective_save_delta_ext,
            is_immune: false,
            delta_int: 0,
            fatigue_added: 0,
            inflicts_dazzled: false,
            inflicts_nausea: false,
        },
        SmokeDensityTier::HeavyStructuralSmoke => SmokeInhalationEvaluation {
            effective_save_delta_ext,
            is_immune: false,
            delta_int: 2,
            fatigue_added: 1,
            inflicts_dazzled: true,
            inflicts_nausea: false,
        },
        SmokeDensityTier::ToxicChemicalOrLithiumFumes => SmokeInhalationEvaluation {
            effective_save_delta_ext,
            is_immune: false,
            delta_int: 4,
            fatigue_added: 2,
            inflicts_dazzled: true,
            inflicts_nausea: true,
        },
    }
}

// ============================================================================
// 4. Electricity, High Voltage & EMP Surges (`environmental_hazards_and_survival.md` §4)
// ============================================================================

/// Electrical shock severity tier (`environmental_hazards_and_survival.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElectricalHazardTier {
    TaserCewPulsed,
    HouseholdMains120To240V,
    IndustrialThreePhase480VTo5kV,
    HighTensionOrLightning10kVPlus,
}

/// Outcome of an electrical shock or EMP surge hazard (`environmental_hazards_and_survival.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectricalShockResult {
    pub tier: ElectricalHazardTier,
    pub effective_raw_trauma: i32,
    pub effective_save_delta_ext: i32,
    pub tetanic_grip_lock: bool,
    pub locks_triad_for_pulse: bool,
    pub stun_clock_freeze_ticks: i32,
    pub causes_unconscious: bool,
    pub cardiac_arrest_risk: bool,
    pub wound_shock_added: i32,
    pub electronics_reboot_ticks: i32,
}

impl ConditionSource for ElectricalShockResult {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            wound_shock_added: self.wound_shock_added,
            clock_freeze_ticks: self.stun_clock_freeze_ticks,
            triad_locked_zero: Triad::splat(self.locks_triad_for_pulse || self.causes_unconscious),
            is_unconscious: self.causes_unconscious,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates an electrical shock hazard using a contested 1-Pair Tiered GM Resistance `HLT/DIS` save
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §4).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_electrical_shock(
    tier: ElectricalHazardTier,
    wet_or_submerged: bool,
    insulated_ppe: bool,
    hlt: i32,
    dis: i32,
    player_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<ElectricalShockResult> {
    let (base_raw, base_ext, base_shock, reboot_ticks) = match tier {
        ElectricalHazardTier::TaserCewPulsed => (1, 4, 0, 20),
        ElectricalHazardTier::HouseholdMains120To240V => (3, 2, 1, 40),
        ElectricalHazardTier::IndustrialThreePhase480VTo5kV => (7, 6, 2, 60),
        ElectricalHazardTier::HighTensionOrLightning10kVPlus => (12, 10, 4, 80),
    };
    let wet_raw = if wet_or_submerged { 2 } else { 0 };
    let wet_ext = if wet_or_submerged { 4 } else { 0 };
    let ppe_raw = if insulated_ppe { -3 } else { 0 };
    let ppe_ext = if insulated_ppe { -4 } else { 0 };
    let effective_raw_trauma = (base_raw + wet_raw + ppe_raw).max(0);
    let effective_save_delta_ext = (base_ext + wet_ext + ppe_ext).max(0);

    let c_p = ((hlt.clamp(1, 20) + dis.clamp(1, 20)) / 2).clamp(1, 20);
    let eval = TieredSaveEval::evaluate(
        1,
        c_p,
        0,
        0,
        effective_save_delta_ext,
        tie_rule,
        player_pairs,
        gm_pair,
    );

    let (passed, is_botch) = eval.classify_4way(
        |_| (true, false),
        |_| (true, false),
        |_| (false, false),
        |_| (false, true),
    );

    let locks_triad_for_pulse =
        tier == ElectricalHazardTier::TaserCewPulsed && effective_raw_trauma > 0;
    let tetanic_grip_lock =
        tier == ElectricalHazardTier::HouseholdMains120To240V && effective_raw_trauma > 0 && !insulated_ppe;
    let stun_clock_freeze_ticks = if !passed && tier == ElectricalHazardTier::TaserCewPulsed {
        8
    } else {
        0
    };
    let causes_unconscious = !passed
        && matches!(
            tier,
            ElectricalHazardTier::IndustrialThreePhase480VTo5kV
                | ElectricalHazardTier::HighTensionOrLightning10kVPlus
        );
    let cardiac_arrest_risk = (!passed && tier == ElectricalHazardTier::HighTensionOrLightning10kVPlus)
        || (is_botch && tier == ElectricalHazardTier::IndustrialThreePhase480VTo5kV);

    ContestedCheck::new(
        eval,
        ElectricalShockResult {
            tier,
            effective_raw_trauma,
            effective_save_delta_ext,
            tetanic_grip_lock,
            locks_triad_for_pulse,
            stun_clock_freeze_ticks,
            causes_unconscious,
            cardiac_arrest_risk,
            wound_shock_added: if effective_raw_trauma > 0 { base_shock } else { 0 },
            electronics_reboot_ticks: reboot_ticks,
        },
    )
}

// ============================================================================
// 5. Dynamic Weather, Wind, Footing & Diving Pressure (`environmental_hazards_and_survival.md` §5)
// ============================================================================

/// Wind intensity tier (`environmental_hazards_and_survival.md` §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindIntensityTier {
    CalmUnder15Kmh,
    FreshBreeze15To35Kmh,
    Gale36To65Kmh,
    StormHurricane65PlusKmh,
}

/// Surface traction and footing tier (`environmental_hazards_and_survival.md` §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceTractionTier {
    DrySolid,
    WetRainPavement,
    MudGravelOilSlick,
    BlackIce,
}

/// Combined wind, ballistic drift, drone stability, auditory masking, and traction evaluation
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeatherAndFootingEvaluation {
    pub wind: WindIntensityTier,
    pub traction: SurfaceTractionTier,
    pub ballistic_delta_ext: i32,
    pub micro_uav_delta_ext: i32,
    pub grounds_small_uavs: bool,
    pub hea_stealth_masking_bonus: i32,
    pub smoke_dissipation_ticks: Option<i32>,
    pub sprint_dodge_delta_ext: i32,
    pub caps_gait_at_combat_glide: bool,
}

impl ConditionSource for WeatherAndFootingEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_ext: self.sprint_dodge_delta_ext,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates wind effects on long-range ballistics, UAV flight, auditory stealth masking, and surface footing
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §5.1).
#[must_use]
pub fn evaluate_weather_and_footing(
    wind: WindIntensityTier,
    traction: SurfaceTractionTier,
    range_m: f64,
    has_crampons: bool,
) -> WeatherAndFootingEvaluation {
    let range_brackets = if range_m <= 0.0 {
        0
    } else {
        (range_m / 200.0).ceil() as i32
    };
    let (
        per_200m_ext,
        micro_uav_delta_ext,
        grounds_small_uavs,
        hea_stealth_masking_bonus,
        smoke_dissipation_ticks,
    ) = match wind {
        WindIntensityTier::CalmUnder15Kmh => (0, 0, false, 0, None),
        WindIntensityTier::FreshBreeze15To35Kmh => (1, 1, false, 0, Some(20)),
        WindIntensityTier::Gale36To65Kmh => (3, 4, false, -2, Some(8)),
        WindIntensityTier::StormHurricane65PlusKmh => (6, 8, true, -6, Some(4)),
    };
    let (sprint_dodge_delta_ext, caps_gait_at_combat_glide) = match traction {
        SurfaceTractionTier::DrySolid => (0, false),
        SurfaceTractionTier::WetRainPavement => (1, false),
        SurfaceTractionTier::MudGravelOilSlick => (3, false),
        SurfaceTractionTier::BlackIce => {
            if has_crampons {
                (0, false)
            } else {
                (6, true)
            }
        }
    };

    WeatherAndFootingEvaluation {
        wind,
        traction,
        ballistic_delta_ext: per_200m_ext * range_brackets,
        micro_uav_delta_ext,
        grounds_small_uavs,
        hea_stealth_masking_bonus,
        smoke_dissipation_ticks,
        sprint_dodge_delta_ext,
        caps_gait_at_combat_glide,
    }
}

/// Underwater breathing gas mixture (`environmental_hazards_and_survival.md` §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreathingGasMix {
    CompressedAir,
    Nitrox,
    TrimixHeliox,
}

/// Decompression sickness ("The Bends") tier (`environmental_hazards_and_survival.md` §5.2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecompressionSicknessTier {
    None,
    MildJointBends,
    SevereNeuroPulmonaryBends,
}

/// Underwater hydrostatic pressure, Nitrogen Narcosis, and Decompression Sickness evaluation
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DivingPressureEvaluation {
    pub depth_m: f64,
    pub ambient_pressure_atm: f64,
    pub air_consumption_multiplier: f64,
    pub nitrogen_narcosis_stat_penalty: i32,
    pub nitrogen_narcosis_delta_int: i32,
    pub deco_check_required: bool,
    pub deco_delta_ext: i32,
    pub deco_sickness: DecompressionSicknessTier,
    pub wound_shock_added: i32,
    pub tempo_and_control_cap_penalty: i32,
}

impl ConditionSource for DivingPressureEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        let bends_delta_int = match self.deco_sickness {
            DecompressionSicknessTier::None => 0,
            DecompressionSicknessTier::MildJointBends => 4,
            DecompressionSicknessTier::SevereNeuroPulmonaryBends => 6,
        };
        ConditionImpact {
            delta_int: self.nitrogen_narcosis_delta_int + bends_delta_int,
            wound_shock_added: self.wound_shock_added,
            triad_cap_penalty: Triad::new(
                self.tempo_and_control_cap_penalty,
                self.tempo_and_control_cap_penalty,
                0,
            ),
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates underwater pressure ($P_{\text{atm}} = 1.0 + \text{depth\_m}/10.0$), Nitrogen Narcosis below `30 m`,
/// and contested `HLT` Decompression Sickness ("The Bends") checks on rapid ascents (`> 9 m/min` from `> 12 m`)
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §5.2).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_diving_pressure_and_deco(
    depth_m: f64,
    gas: BreathingGasMix,
    ascent_rate_m_per_min: f64,
    missed_deco_stops: i32,
    hlt: i32,
    player_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<DivingPressureEvaluation> {
    let d = depth_m.max(0.0);
    let ambient_pressure_atm = 1.0 + d / 10.0;
    let narcosis_brackets = if d > 30.0 && gas != BreathingGasMix::TrimixHeliox {
        ((d - 30.0) / 10.0).floor() as i32 + 1
    } else {
        0
    };
    let nitrogen_narcosis_stat_penalty = 2 * narcosis_brackets;
    let nitrogen_narcosis_delta_int = if narcosis_brackets > 0 { 2 } else { 0 };

    let deco_check_required = d > 12.0 && (ascent_rate_m_per_min > 9.0 || missed_deco_stops > 0);
    let deco_delta_ext = if deco_check_required {
        ((d / 5.0).round() as i32 + 2 * missed_deco_stops.max(0)).max(0)
    } else {
        0
    };

    let eval = TieredSaveEval::evaluate(
        1,
        hlt.clamp(1, 20),
        nitrogen_narcosis_delta_int,
        0,
        deco_delta_ext,
        tie_rule,
        player_pairs,
        gm_pair,
    );

    let (deco_sickness, wound_shock_added, tempo_and_control_cap_penalty) = if !deco_check_required
    {
        (DecompressionSicknessTier::None, 0, 0)
    } else {
        eval.classify_4way(
            |_| (DecompressionSicknessTier::None, 0, 0),
            |_| (DecompressionSicknessTier::None, 0, 0),
            |_| (DecompressionSicknessTier::MildJointBends, 0, 4),
            |_| (DecompressionSicknessTier::SevereNeuroPulmonaryBends, 3, 6),
        )
    };

    ContestedCheck::new(
        eval,
        DivingPressureEvaluation {
            depth_m: d,
            ambient_pressure_atm,
            air_consumption_multiplier: ambient_pressure_atm,
            nitrogen_narcosis_stat_penalty,
            nitrogen_narcosis_delta_int,
            deco_check_required,
            deco_delta_ext,
            deco_sickness,
            wound_shock_added,
            tempo_and_control_cap_penalty,
        },
    )
}

// ============================================================================
// 6. Wound Infection, Sepsis & Field Pharmacology (`environmental_hazards_and_survival.md` §6)
// ============================================================================

/// Wound contamination level driving daily `HLT` sepsis checks (`environmental_hazards_and_survival.md` §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WoundContaminationLevel {
    CleanIrrigated,
    DirtyFieldOrSecondDegreeBurn,
    PerforatedGutThirdDegreeOrLeukopenia,
}

impl WoundContaminationLevel {
    /// Base hidden external difficulty $\Delta_{\text{ext}}$ (`0`, `+4`, `+8`).
    #[must_use]
    pub const fn delta_ext(self) -> i32 {
        match self {
            Self::CleanIrrigated => 0,
            Self::DirtyFieldOrSecondDegreeBurn => 4,
            Self::PerforatedGutThirdDegreeOrLeukopenia => 8,
        }
    }
}

/// Clinical wound infection and sepsis stage (`Stages 0–3`, `environmental_hazards_and_survival.md` §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SepsisStage {
    Stage0Clean,
    Stage1LocalizedInfection,
    Stage2SystemicSepsis,
    Stage3SepticShock,
}

impl SepsisStage {
    #[must_use]
    pub const fn stage_number(self) -> i32 {
        match self {
            Self::Stage0Clean => 0,
            Self::Stage1LocalizedInfection => 1,
            Self::Stage2SystemicSepsis => 2,
            Self::Stage3SepticShock => 3,
        }
    }

    #[must_use]
    pub const fn from_stage_number(n: i32) -> Self {
        match n {
            ..=0 => Self::Stage0Clean,
            1 => Self::Stage1LocalizedInfection,
            2 => Self::Stage2SystemicSepsis,
            3.. => Self::Stage3SepticShock,
        }
    }
}

/// Evaluation of daily wound sepsis progression or antibiotic recovery (`environmental_hazards_and_survival.md` §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SepsisEvaluation {
    pub stage: SepsisStage,
    pub save_delta_ext: i32,
    pub delta_int: i32,
    pub fit_hlt_foc_penalty: i32,
    pub fatigue_level_floor: i32,
    pub wound_shock_added: i32,
    pub halts_natural_healing: bool,
    pub is_unconscious: bool,
}

impl ConditionSource for SepsisEvaluation {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.delta_int,
            fatigue_floor: self.fatigue_level_floor,
            wound_shock_added: self.wound_shock_added,
            triad_locked_zero: Triad::splat(self.is_unconscious),
            is_unconscious: self.is_unconscious,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates daily wound infection / sepsis `HLT` save against contamination level and antibiotic coverage
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §6.1).
#[must_use]
pub fn evaluate_wound_sepsis(
    current_stage: SepsisStage,
    contamination: WoundContaminationLevel,
    antibiotics_active: bool,
    hlt: i32,
    player_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<SepsisEvaluation> {
    let save_delta_ext = contamination.delta_ext() + if antibiotics_active { -6 } else { 0 };
    let eval = TieredSaveEval::evaluate(
        1,
        hlt.clamp(1, 20),
        0,
        0,
        save_delta_ext,
        tie_rule,
        player_pairs,
        gm_pair,
    );
    let cur = current_stage.stage_number();
    let next_num = eval.classify_4way(
        |_| (cur - if antibiotics_active { 2 } else { 1 }).max(0),
        |_| {
            if antibiotics_active {
                (cur - 1).max(0)
            } else {
                cur
            }
        },
        |_| (cur + 1).min(3),
        |_| (cur + 2).min(3),
    );
    let stage = SepsisStage::from_stage_number(next_num);
    let (delta_int, fit_hlt_foc_penalty, fatigue_level_floor, wound_shock_added, halts, uncon) =
        match stage {
            SepsisStage::Stage0Clean => (0, 0, 0, 0, false, false),
            SepsisStage::Stage1LocalizedInfection => (2, 0, 0, 0, true, false),
            SepsisStage::Stage2SystemicSepsis => (4, 3, 2, 0, true, false),
            SepsisStage::Stage3SepticShock => (8, 6, 4, 3, true, true),
        };

    ContestedCheck::new(
        eval,
        SepsisEvaluation {
            stage,
            save_delta_ext,
            delta_int,
            fit_hlt_foc_penalty,
            fatigue_level_floor,
            wound_shock_added,
            halts_natural_healing: halts,
            is_unconscious: uncon,
        },
    )
}

define_coded_enum! {
    /// Canonical 6-entry Field Pharmacology & Combat Drugs identifiers (`environmental_hazards_and_survival.md` §6.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[repr(usize)]
    pub enum FieldDrugId [count = 6] {
        CombatStimulantAutoInjector => {
            code: "COMBAT_STIMULANT_AUTO_INJECTOR",
            label: "Combat Stimulant Auto-Injector (Modafinil / Amphetamine)",
            aliases: ["COMBAT_STIM", "MODAFINIL", "STIMULANT"],
        },
        FieldAnalgesicLozenge => {
            code: "FIELD_ANALGESIC_LOZENGE",
            label: "Field Analgesic Lozenge / Auto-Injector (Ketamine / Fentanyl)",
            aliases: ["ANALGESIC", "FENTANYL_LOZENGE", "KETAMINE"],
        },
        MarksmanshipBetaBlocker => {
            code: "MARKSMANSHIP_BETA_BLOCKER",
            label: "Marksmanship Beta-Blocker (Propranolol)",
            aliases: ["BETA_BLOCKER", "PROPRANOLOL"],
        },
        BroadSpectrumAntibioticPack => {
            code: "BROAD_SPECTRUM_ANTIBIOTIC_PACK",
            label: "Broad-Spectrum Antibiotic Pack (IV / Oral)",
            aliases: ["ANTIBIOTICS", "ANTIBIOTIC_PACK"],
        },
        AntiemeticAutoInjector => {
            code: "ANTIEMETIC_AUTO_INJECTOR",
            label: "Antiemetic Auto-Injector (Ondansetron)",
            aliases: ["ANTIEMETIC", "ONDANSETRON"],
        },
        SedativeTranquilizerDart => {
            code: "SEDATIVE_TRANQUILIZER_DART",
            label: "Sedative / Tranquilizer Dart (Midazolam / Medetomidine)",
            aliases: ["TRANQUILIZER_DART", "SEDATIVE_DART"],
        },
    }
}

/// Specification of a field pharmacological agent (`environmental_hazards_and_survival.md` §6.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDrugSpec {
    pub id: FieldDrugId,
    pub name: &'static str,
    pub onset_ticks: i32,
    pub duration_minutes: i32,
    pub fatigue_suppression_levels: i32,
    pub shock_suppression_tiers: i32,
    pub precision_tool_bonus: i32,
    pub sepsis_save_bonus: i32,
    pub sedation_stat_penalty: i32,
    pub overdose_delta_ext: i32,
    pub cost_usd: i32,
    pub restriction_class: Option<BlackMarketRestrictionClass>,
    pub summary: &'static str,
}

macro_rules! drug_spec {
    (
        $id:ident, $name:literal, ($onset:expr, $dur_min:expr),
        ($fat_sup:expr, $shk_sup:expr, $tool_bon:expr, $sep_bon:expr, $sed_pen:expr, $od_ext:expr),
        $cost:expr, $restr:expr, $summary:literal
    ) => {
        FieldDrugSpec {
            id: FieldDrugId::$id,
            name: $name,
            onset_ticks: $onset,
            duration_minutes: $dur_min,
            fatigue_suppression_levels: $fat_sup,
            shock_suppression_tiers: $shk_sup,
            precision_tool_bonus: $tool_bon,
            sepsis_save_bonus: $sep_bon,
            sedation_stat_penalty: $sed_pen,
            overdose_delta_ext: $od_ext,
            cost_usd: $cost,
            restriction_class: $restr,
            summary: $summary,
        }
    };
}

/// Canonical 6-entry Field Pharmacology & Combat Drugs Catalog (`environmental_hazards_and_survival.md` §6.2).
pub const FIELD_PHARMACOLOGY_CATALOG: [FieldDrugSpec; 6] = [
    drug_spec!(
        CombatStimulantAutoInjector,
        "Combat Stimulant Auto-Injector (Modafinil / Amphetamine)",
        (4, 240),
        (2, 0, 0, 0, 0, 4),
        120,
        Some(BlackMarketRestrictionClass::Class2LawEnforcementParamilitary),
        "Suppresses 2 fatigue levels +2 OODA/FOC for 4h; crash adds +2 fatigue & +2 delta_int"
    ),
    drug_spec!(
        FieldAnalgesicLozenge,
        "Field Analgesic Lozenge / Auto-Injector (Ketamine / Fentanyl)",
        (8, 120),
        (0, 2, 0, 0, 3, 6),
        85,
        Some(BlackMarketRestrictionClass::Class1LicensedCivilian),
        "Reduces wound_shock_index by -2 tiers; imposes -3 FOC/ANA/SPD sedation"
    ),
    drug_spec!(
        MarksmanshipBetaBlocker,
        "Marksmanship Beta-Blocker (Propranolol)",
        (3600, 240),
        (0, 0, -2, 0, 0, 2),
        45,
        Some(BlackMarketRestrictionClass::Class1LicensedCivilian),
        "Grants -2 delta_tool on Precision Rifle / Microsurgery; -2 FIT on aerobic sprint"
    ),
    drug_spec!(
        BroadSpectrumAntibioticPack,
        "Broad-Spectrum Antibiotic Pack (IV / Oral)",
        (86400, 10080),
        (0, 0, 0, -6, 0, 0),
        60,
        None,
        "Grants -6 delta_ext on Sepsis HLT saves for 7 days and reverses Stage 1-2 infections"
    ),
    drug_spec!(
        AntiemeticAutoInjector,
        "Antiemetic Auto-Injector (Ondansetron)",
        (480, 360),
        (0, 0, 0, 0, 0, 0),
        40,
        Some(BlackMarketRestrictionClass::Class1LicensedCivilian),
        "Suppresses Nausea (-4 delta_int relief) for 6h with negligible sedation"
    ),
    drug_spec!(
        SedativeTranquilizerDart,
        "Sedative / Tranquilizer Dart (Midazolam / Medetomidine)",
        (12, 60),
        (0, 0, 0, 0, 6, 8),
        140,
        Some(BlackMarketRestrictionClass::Class2LawEnforcementParamilitary),
        "Target HLT save (delta_ext +6) or Dazed (-6 SPD/FOC/ANA) -> Unconscious on failure"
    ),
];

impl_static_catalog!(FieldDrugId => FieldDrugSpec : FIELD_PHARMACOLOGY_CATALOG);

/// Returns all canonical Field Pharmacology & Combat Drug specifications
/// (`docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md` §6.2).
#[must_use]
pub const fn canonical_field_pharmacology_catalog() -> &'static [FieldDrugSpec] {
    FieldDrugId::CATALOG
}
