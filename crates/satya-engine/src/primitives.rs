//! Shared foundational primitives, generic containers, composable status impacts,
//! tiered save helpers, and declarative enum/catalog macros used across `satya-engine`.
//!
//! # TTRPG Rules Origins
//! - `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1, §2.2, §5)
//! - `docs/ttrpg/01-foundation/design_philosophy.md` (§5.10, §5.11)
//! - `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.2–§6.3)
//! - `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.2, §5)
//! - `docs/ttrpg/05-combat/tactical_triad_and_pools.md` (§1–§4)
//! - `docs/ttrpg/06-health-and-survival/health_and_trauma.md` (§2–§5)

use std::ops::{Add, AddAssign, Deref, DerefMut, Index, IndexMut, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

use crate::dice_probabilities::{
    ContestedCascadeRollOutcome, ContestedCascadeTaskParams, ContestedOutcomeKind, ContestedTieRule,
};

/// Axis selector for a [`Triad`].
///
/// Origin: `docs/ttrpg/01-foundation/design_philosophy.md` (§5.10),
/// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§5), and
/// `docs/ttrpg/05-combat/tactical_triad_and_pools.md` (§1–§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(usize)]
pub enum TriadAxis {
    Tempo = 0,
    Control = 1,
    Force = 2,
}

impl TriadAxis {
    pub const ALL: [Self; 3] = [Self::Tempo, Self::Control, Self::Force];
}

/// Generic 3-axis Tactical Triad container (`tempo`, `control`, `force`).
///
/// Used across `combat`, `health_survival`, and `rules_book` to represent triad allocations,
/// envelope caps, hit-location cap slashes (`Triad<bool>`), and vehicle/status penalties.
///
/// Origin: `docs/ttrpg/01-foundation/design_philosophy.md` (§5.10),
/// `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§5), and
/// `docs/ttrpg/05-combat/tactical_triad_and_pools.md` (§1–§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Triad<T = i32> {
    pub tempo: T,
    pub control: T,
    pub force: T,
}

impl<T> Triad<T> {
    #[must_use]
    pub const fn new(tempo: T, control: T, force: T) -> Self {
        Self {
            tempo,
            control,
            force,
        }
    }

    #[must_use]
    pub fn into_tuple(self) -> (T, T, T) {
        (self.tempo, self.control, self.force)
    }

    #[must_use]
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> Triad<U> {
        Triad {
            tempo: f(self.tempo),
            control: f(self.control),
            force: f(self.force),
        }
    }

    #[must_use]
    pub fn zip_with<U, V, F: FnMut(T, U) -> V>(self, other: Triad<U>, mut f: F) -> Triad<V> {
        Triad {
            tempo: f(self.tempo, other.tempo),
            control: f(self.control, other.control),
            force: f(self.force, other.force),
        }
    }
}

impl<T> From<(T, T, T)> for Triad<T> {
    fn from((tempo, control, force): (T, T, T)) -> Self {
        Self::new(tempo, control, force)
    }
}

impl<T> From<Triad<T>> for (T, T, T) {
    fn from(triad: Triad<T>) -> Self {
        (triad.tempo, triad.control, triad.force)
    }
}

impl<T> Index<TriadAxis> for Triad<T> {
    type Output = T;

    fn index(&self, axis: TriadAxis) -> &Self::Output {
        match axis {
            TriadAxis::Tempo => &self.tempo,
            TriadAxis::Control => &self.control,
            TriadAxis::Force => &self.force,
        }
    }
}

impl<T> IndexMut<TriadAxis> for Triad<T> {
    fn index_mut(&mut self, axis: TriadAxis) -> &mut Self::Output {
        match axis {
            TriadAxis::Tempo => &mut self.tempo,
            TriadAxis::Control => &mut self.control,
            TriadAxis::Force => &mut self.force,
        }
    }
}

impl<T: Add<Output = U>, U> Add for Triad<T> {
    type Output = Triad<U>;

    fn add(self, rhs: Self) -> Self::Output {
        self.zip_with(rhs, |a, b| a + b)
    }
}

impl<T: Sub<Output = U>, U> Sub for Triad<T> {
    type Output = Triad<U>;

    fn sub(self, rhs: Self) -> Self::Output {
        self.zip_with(rhs, |a, b| a - b)
    }
}

impl<T: AddAssign> AddAssign for Triad<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.tempo += rhs.tempo;
        self.control += rhs.control;
        self.force += rhs.force;
    }
}

impl<T: SubAssign> SubAssign for Triad<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.tempo -= rhs.tempo;
        self.control -= rhs.control;
        self.force -= rhs.force;
    }
}

impl<T: Neg<Output = U>, U> Neg for Triad<T> {
    type Output = Triad<U>;

    fn neg(self) -> Self::Output {
        self.map(|v| -v)
    }
}

impl<T: Copy> Triad<T> {
    #[must_use]
    pub const fn splat(value: T) -> Self {
        Self {
            tempo: value,
            control: value,
            force: value,
        }
    }
}

impl Triad<i32> {
    pub const ZERO: Self = Self::new(0, 0, 0);

    #[must_use]
    pub const fn sum(self) -> i32 {
        self.tempo + self.control + self.force
    }

    #[must_use]
    pub const fn has_negative(self) -> bool {
        self.tempo < 0 || self.control < 0 || self.force < 0
    }

    #[must_use]
    pub fn clamp_min(self, floor: i32) -> Self {
        Self {
            tempo: self.tempo.max(floor),
            control: self.control.max(floor),
            force: self.force.max(floor),
        }
    }

    #[must_use]
    pub fn clamp_each(self, min_triad: Self, max_triad: Self) -> Self {
        Self {
            tempo: self.tempo.clamp(min_triad.tempo, max_triad.tempo),
            control: self.control.clamp(min_triad.control, max_triad.control),
            force: self.force.clamp(min_triad.force, max_triad.force),
        }
    }
}

/// Composable physiological, mental, and environmental penalty bundle.
///
/// Unifies the recurring modifier fields shared across Fatigue, Status Conditions,
/// Acute Mental Trauma, Dehydration, Starvation, Sleep Deprivation, Thermal Stress,
/// Radiation, and High-G Collisions.
///
/// Origin: `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1, §5),
/// `docs/ttrpg/01-foundation/design_philosophy.md` (§5.11), and
/// `docs/ttrpg/06-health-and-survival/health_and_trauma.md` (§2–§5).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ConditionImpact {
    /// Internal/Physiological penalty modifier (\Delta_{int}, typically >= 0 for penalties, -2 for Focused/Surge; increases \delta_{under} = 6 + \Delta_{int} per mathematical_notation_glossary.md §2.1).
    pub delta_int: i32,
    /// External/Environmental difficulty modifier (\Delta_{ext}, typically >= 0 for penalties, < 0 for bonuses; shifts TN_{GM} = 21 - \Delta_{ext} per mathematical_notation_glossary.md §2.1).
    pub delta_ext: i32,
    /// Direct modifier to Effective Tactical Pool (P_{eff}, typically <= 0; mathematical_notation_glossary.md §5).
    pub tactical_pool_mod: i32,
    /// Additive penalty to the character's Action Time Factor (`+0.25 = +25%` slower).
    pub action_time_factor_delta: f64,
    pub fatigue_added: i32,
    /// Minimum Fatigue Level floor enforced while this condition persists (`0..=5`).
    pub fatigue_floor: i32,
    pub wound_shock_added: i32,
    pub clock_freeze_ticks: i32,
    /// Subtractive penalty to `(Tempo, Control, Force)` envelope caps.
    pub triad_cap_penalty: Triad<i32>,
    pub triad_locked_zero: Triad<bool>,
    pub is_unconscious: bool,
}

impl ConditionImpact {
    pub const ZERO: Self = Self {
        delta_int: 0,
        delta_ext: 0,
        tactical_pool_mod: 0,
        action_time_factor_delta: 0.0,
        fatigue_added: 0,
        fatigue_floor: 0,
        wound_shock_added: 0,
        clock_freeze_ticks: 0,
        triad_cap_penalty: Triad::ZERO,
        triad_locked_zero: Triad::splat(false),
        is_unconscious: false,
    };

    /// Combine two `ConditionImpact` values additively (taking max for floors and logical OR for locks).
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        Self {
            delta_int: self.delta_int + other.delta_int,
            delta_ext: self.delta_ext + other.delta_ext,
            tactical_pool_mod: self.tactical_pool_mod + other.tactical_pool_mod,
            action_time_factor_delta: self.action_time_factor_delta
                + other.action_time_factor_delta,
            fatigue_added: self.fatigue_added + other.fatigue_added,
            fatigue_floor: self.fatigue_floor.max(other.fatigue_floor),
            wound_shock_added: self.wound_shock_added + other.wound_shock_added,
            clock_freeze_ticks: self.clock_freeze_ticks + other.clock_freeze_ticks,
            triad_cap_penalty: self
                .triad_cap_penalty
                .zip_with(other.triad_cap_penalty, |a, b| a + b),
            triad_locked_zero: self
                .triad_locked_zero
                .zip_with(other.triad_locked_zero, |a, b| a || b),
            is_unconscious: self.is_unconscious || other.is_unconscious,
        }
    }
}

/// Trait implemented by any physiological, psychological, encumbrance, or environmental state
/// that produces a [`ConditionImpact`] modifier bundle.
///
/// Origin: `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1, §5) and
/// `docs/ttrpg/06-health-and-survival/health_and_trauma.md` (§2–§5).
pub trait ConditionSource {
    /// Convert this condition or evaluation state into its canonical [`ConditionImpact`].
    #[must_use]
    fn to_condition_impact(&self) -> ConditionImpact;
}

impl ConditionSource for ConditionImpact {
    fn to_condition_impact(&self) -> ConditionImpact {
        *self
    }
}

/// Aggregate an arbitrary collection of [`ConditionSource`] references into a single combined [`ConditionImpact`].
///
/// Origin: `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.1, §5) and
/// `docs/ttrpg/06-health-and-survival/health_and_trauma.md` (§2–§5).
#[must_use]
pub fn aggregate_condition_impacts<'a>(
    sources: impl IntoIterator<Item = &'a dyn ConditionSource>,
) -> ConditionImpact {
    sources
        .into_iter()
        .fold(ConditionImpact::ZERO, |acc, src| {
            acc.combine(src.to_condition_impact())
        })
}

/// Shared evaluation result for a 1-Pair Tiered GM Resistance check (`ContestedCascadeTaskParams::new_tiered_environment`).
///
/// Used across `psyche_social` (Psyche Friction, Combat Morale, Social Exchange) and
/// `health_survival` (`WSR` Consciousness Saves) to eliminate repetitive contested-save boilerplate.
///
/// Origin: `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2),
/// `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.2–§6.3), and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.2, §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TieredSaveEval {
    pub player_tn: i32,
    pub gm_tn: i32,
    pub contested_roll: ContestedCascadeRollOutcome,
}

/// Generic wrapper pairing a 1-Pair Tiered GM Resistance evaluation (`player_tn`, `gm_tn`, `contested_roll`)
/// with a domain-specific outcome payload `D`.
///
/// Implements [`Deref<Target = D>`] so all domain fields on `D` can be accessed directly on
/// `ContestedCheck<D>`, while `#[serde(flatten)]` preserves flat JSON serialization compatibility.
///
/// Origin: `docs/ttrpg/01-foundation/mathematical_notation_glossary.md` (§2.2),
/// `docs/ttrpg/01-foundation/probability_and_calibration.md` (§6.2–§6.3), and
/// `docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md` (§4.2, §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContestedCheck<D> {
    pub player_tn: i32,
    pub gm_tn: i32,
    pub contested_roll: ContestedCascadeRollOutcome,
    #[serde(flatten)]
    pub effect: D,
}

impl<D> ContestedCheck<D> {
    #[must_use]
    pub fn new(eval: TieredSaveEval, effect: D) -> Self {
        Self {
            player_tn: eval.player_tn,
            gm_tn: eval.gm_tn,
            contested_roll: eval.contested_roll,
            effect,
        }
    }
}

impl<D> Deref for ContestedCheck<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.effect
    }
}

impl<D> DerefMut for ContestedCheck<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.effect
    }
}

impl TieredSaveEval {
    /// Evaluate a player's pairs against a 1-Pair Tiered GM Resistance (`hidden_external_difficulty`).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate(
        player_skill_n: usize,
        player_control_c: i32,
        player_internal_mod: i32,
        player_known_tool_mod: i32,
        hidden_external_difficulty: i32,
        tie_rule: ContestedTieRule,
        player_pairs: &[(i32, i32)],
        gm_pair: (i32, i32),
    ) -> Self {
        let task = ContestedCascadeTaskParams::new_tiered_environment(
            player_skill_n,
            player_control_c,
            player_internal_mod,
            player_known_tool_mod,
            hidden_external_difficulty,
            tie_rule,
        );
        let player_tn = task.player.tn_eff();
        let gm_tn = task.gm.tn_eff();
        let contested_roll = task.evaluate_contested_pairs(player_pairs, &[gm_pair]);
        Self {
            player_tn,
            gm_tn,
            contested_roll,
        }
    }

    /// Map the `ContestedOutcomeKind` into a domain-specific 4-way save outcome:
    /// - `ExtraordinarySuccess` -> `on_extraordinary`
    /// - `CleanSuccess` | `GrazingTieSuccess` | `LuckyBreakSuccess` | `SuccessWithFloorBreach` -> `on_clean_or_partial`
    /// - `CleanFailure` -> `on_clean_fail`
    /// - `BotchFailure` -> `on_botch`
    #[must_use]
    pub fn classify_4way<T, F1, F2, F3, F4>(
        &self,
        on_extraordinary: F1,
        on_clean_or_partial: F2,
        on_clean_fail: F3,
        on_botch: F4,
    ) -> T
    where
        F1: FnOnce(&ContestedCascadeRollOutcome) -> T,
        F2: FnOnce(&ContestedCascadeRollOutcome) -> T,
        F3: FnOnce(&ContestedCascadeRollOutcome) -> T,
        F4: FnOnce(&ContestedCascadeRollOutcome) -> T,
    {
        match self.contested_roll.contested_kind {
            ContestedOutcomeKind::ExtraordinarySuccess => on_extraordinary(&self.contested_roll),
            ContestedOutcomeKind::CleanSuccess
            | ContestedOutcomeKind::GrazingTieSuccess
            | ContestedOutcomeKind::LuckyBreakSuccess
            | ContestedOutcomeKind::SuccessWithFloorBreach => {
                on_clean_or_partial(&self.contested_roll)
            }
            ContestedOutcomeKind::CleanFailure => on_clean_fail(&self.contested_roll),
            ContestedOutcomeKind::BotchFailure => on_botch(&self.contested_roll),
        }
    }
}

/// Declarative macro to define a canonical coded enum with `ALL`, `as_str`, `label`, `full_name`,
/// optional `subtitle`, and case-insensitive `from_code_str` (with optional aliases).
#[macro_export]
macro_rules! define_coded_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $Name:ident [count = $Count:expr] {
            $(
                $(#[$vmeta:meta])*
                $Variant:ident => {
                    code: $code:literal
                    $(, label: $label:literal)?
                    $(, subtitle: $subtitle:literal)?
                    $(, aliases: [$($alias:literal),* $(,)?])?
                    $(,)?
                }
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $Name {
            $(
                $(#[$vmeta])*
                $Variant,
            )+
        }

        impl $Name {
            pub const ALL: [Self; $Count] = [
                $(Self::$Variant),+
            ];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $code,)+
                }
            }

            #[must_use]
            pub const fn label(self) -> &'static str {
                match self {
                    $(
                        Self::$Variant => {
                            #[allow(unused_variables)]
                            let l: &'static str = $code;
                            $(let l: &'static str = $label;)?
                            l
                        }
                    )+
                }
            }

            #[must_use]
            pub const fn full_name(self) -> &'static str {
                self.label()
            }

            #[must_use]
            pub const fn subtitle(self) -> &'static str {
                match self {
                    $(
                        Self::$Variant => {
                            #[allow(unused_variables)]
                            let s: &'static str = "";
                            $(let s: &'static str = $subtitle;)?
                            s
                        }
                    )+
                }
            }

            #[must_use]
            pub fn from_code_str(code: &str) -> Option<Self> {
                let normalized = code.trim();
                $(
                    if normalized.eq_ignore_ascii_case($code)
                        $($(|| normalized.eq_ignore_ascii_case($alias))*)?
                    {
                        return Some(Self::$Variant);
                    }
                )+
                None
            }
        }

        impl ::std::fmt::Display for $Name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::std::str::FromStr for $Name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::from_code_str(s).ok_or_else(|| format!("unknown {}: {s}", stringify!($Name)))
            }
        }
    };
}

pub trait StaticCatalog: Copy + Eq + 'static {
    type Spec: 'static;

    const ALL_IDS: &'static [Self];

    const CATALOG: &'static [Self::Spec];

    #[must_use]
    fn spec(self) -> &'static Self::Spec;
}

#[macro_export]
macro_rules! impl_static_catalog {
    ($Id:ty => $Spec:ty : $CATALOG:ident) => {
        impl $Id {
            #[must_use]
            pub const fn spec(self) -> &'static $Spec {
                &$CATALOG[self as usize]
            }
        }

        impl $crate::primitives::StaticCatalog for $Id {
            type Spec = $Spec;
            const ALL_IDS: &'static [Self] = &Self::ALL;
            const CATALOG: &'static [$Spec] = &$CATALOG;

            fn spec(self) -> &'static Self::Spec {
                Self::spec(self)
            }
        }
    };
}
