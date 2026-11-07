//! Shared foundational primitives, generic containers, composable status impacts,
//! tiered save helpers, and declarative enum/catalog macros used across `satya-engine`.

use serde::{Deserialize, Serialize};

use crate::dice_probabilities::{
    ContestedCascadeRollOutcome, ContestedCascadeTaskParams, ContestedOutcomeKind, ContestedTieRule,
};

/// Generic 3-axis Tactical Triad container (`tempo`, `control`, `force`).
///
/// Used across `combat`, `health_survival`, and `rules_book` to represent triad allocations,
/// envelope caps, hit-location cap slashes (`Triad<bool>`), and vehicle/status penalties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Triad<T = i32> {
    /// Tempo axis (initiative, rate of action, maneuvering speed).
    pub tempo: T,
    /// Control axis (positional dominance, accuracy, reach/clinch control).
    pub control: T,
    /// Force axis (impact energy, follow-through, structural power).
    pub force: T,
}

impl<T> Triad<T> {
    /// Construct a new `Triad` from `(tempo, control, force)`.
    #[must_use]
    pub const fn new(tempo: T, control: T, force: T) -> Self {
        Self {
            tempo,
            control,
            force,
        }
    }

    /// Convert into a `(tempo, control, force)` 3-tuple.
    #[must_use]
    pub fn into_tuple(self) -> (T, T, T) {
        (self.tempo, self.control, self.force)
    }

    /// Transform each axis of the triad using `f`.
    #[must_use]
    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> Triad<U> {
        Triad {
            tempo: f(self.tempo),
            control: f(self.control),
            force: f(self.force),
        }
    }

    /// Combine two triads element-wise using `f`.
    #[must_use]
    pub fn zip_with<U, V, F: FnMut(T, U) -> V>(self, other: Triad<U>, mut f: F) -> Triad<V> {
        Triad {
            tempo: f(self.tempo, other.tempo),
            control: f(self.control, other.control),
            force: f(self.force, other.force),
        }
    }
}

impl<T: Copy> Triad<T> {
    /// Construct a `Triad` with the same value on all three axes.
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
    /// Zero triad `(0, 0, 0)`.
    pub const ZERO: Self = Self::new(0, 0, 0);

    /// Total points allocated across all three axes (`tempo + control + force`).
    #[must_use]
    pub const fn sum(self) -> i32 {
        self.tempo + self.control + self.force
    }

    /// Clamp each axis to at least `floor`.
    #[must_use]
    pub fn clamp_min(self, floor: i32) -> Self {
        Self {
            tempo: self.tempo.max(floor),
            control: self.control.max(floor),
            force: self.force.max(floor),
        }
    }

    /// Clamp each axis between the corresponding axes of `min_triad` and `max_triad`.
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
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ConditionImpact {
    /// Internal/Physiological TN penalty (`Delta_int`, typically `<= 0`).
    pub delta_int: i32,
    /// External/Environmental TN penalty (`Delta_ext`, typically `<= 0`).
    pub delta_ext: i32,
    /// Direct modifier to total Tactical Pool (`P_tactical`, typically `<= 0`).
    pub tactical_pool_mod: i32,
    /// Additive penalty to the character's Action Time Factor (`+0.25 = +25%` slower).
    pub action_time_factor_delta: f64,
    /// Fatigue levels added by this condition.
    pub fatigue_added: i32,
    /// Minimum Fatigue Level floor enforced while this condition persists (`0..=5`).
    pub fatigue_floor: i32,
    /// Wound Shock (`WS`) points added by this condition.
    pub wound_shock_added: i32,
    /// Initiative clock freeze duration in ticks (`0` if none).
    pub clock_freeze_ticks: i32,
    /// Subtractive penalty to `(Tempo, Control, Force)` envelope caps.
    pub triad_cap_penalty: Triad<i32>,
    /// Axes of `(Tempo, Control, Force)` hard-locked to `0`.
    pub triad_locked_zero: Triad<bool>,
    /// True if this condition causes immediate unconsciousness or incapacitation.
    pub is_unconscious: bool,
}

impl ConditionImpact {
    /// Neutral `ConditionImpact` with zero penalties.
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

/// Shared evaluation result for a 1-Pair Tiered GM Resistance check (`ContestedCascadeTaskParams::new_tiered_environment`).
///
/// Used across `psyche_social` (Psyche Friction, Combat Morale, Social Exchange) and
/// `health_survival` (`WSR` Consciousness Saves) to eliminate repetitivecontested-save boilerplate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TieredSaveEval {
    /// Player's effective Target Number (`TN`).
    pub player_tn: i32,
    /// GM's 1-Pair Resistance Target Number (`TN_GM`).
    pub gm_tn: i32,
    /// Full contested cascade roll breakdown.
    pub contested_roll: ContestedCascadeRollOutcome,
}

impl TieredSaveEval {
    /// Evaluate a player's pairs against a 1-Pair Tiered GM Resistance (`hidden_external_difficulty`).
    #[must_use]
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
            /// All canonical variants of this enum in declaration order.
            pub const ALL: [Self; $Count] = [
                $(Self::$Variant),+
            ];

            /// Canonical code string for this variant.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $code,)+
                }
            }

            /// Human-readable label for this variant.
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

            /// Full human-readable name (alias for [`Self::label`]).
            #[must_use]
            pub const fn full_name(self) -> &'static str {
                self.label()
            }

            /// Optional descriptive subtitle for this variant (empty if none specified).
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

            /// Parse a variant from its code string or any registered alias (case-insensitive, trimmed).
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
    };
}
