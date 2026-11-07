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

use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::action_economy::compute_delta_tc_decide;
pub use crate::ballistics::*;
use crate::characteristics::{
    CharacterCharacteristics, compute_d_cog_score, compute_fatigue_threshold_score,
    compute_mmf_momentum_factor, compute_ooda_score, compute_pow_striking_power,
};
use crate::dice_probabilities::{CascadeOutcomeKind, CascadeRollOutcome};
use crate::primitives::{StaticCatalog, Triad};
use crate::{define_coded_enum, impl_static_catalog};

// ============================================================================
// Combat Tactical Pool ("Envelope & Fill" — Tempo, Control, Force) Calculator
// ============================================================================

define_coded_enum! {
    /// Psychological & Morale condition governing a combatant's Tactical Pool size and allocation constraints.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum MoraleState [count = 6] {
        /// Eager and ready to attack (`+2` Pool; `+1` to Force or Tempo ceiling).
        High => { code: "high", label: "High", aliases: ["conviction", "conviction_surge"] },
        /// Default composed condition (`0` Pool modifier; full allocation freedom up to ceilings).
        #[default]
        Steady => { code: "steady", label: "Steady", aliases: ["normal"] },
        /// Adrenaline bloodlust surge (`+2` Pool; must allocate `Force >= P_eff / 2`, and `Control <= P_eff / 4`).
        Rage => { code: "rage", label: "Rage", aliases: ["bloodlust"] },
        /// Doubt creeping in (`-2` Pool; `Force` cannot exceed `Control`, `F <= C`).
        Wavering => { code: "wavering", label: "Wavering", aliases: ["shaken"] },
        /// Disengaging / withdrawing (`-4` Pool; active offensive `Force == 0`).
        Broken => { code: "broken", label: "Broken", aliases: ["withdrawing"] },
        /// Full-blown rout (`-4` Pool; `Control == 0` and `Force == 0`, `100%` locked into `Tempo`).
        Panic => { code: "panic", label: "Panic", aliases: ["routed"] },
    }
}

impl MoraleState {
    /// Returns the Tactical Pool modifier ($\Delta P_{\text{morale}}$) for this morale state.
    #[must_use]
    pub const fn pool_modifier(self) -> i32 {
        match self {
            Self::High | Self::Rage => 2,
            Self::Steady => 0,
            Self::Wavering => -2,
            Self::Broken | Self::Panic => -4,
        }
    }
}

/// Character attributes, skill level, and physical/psychological state inputs for the
/// Satya Combat Tactical Pool ("Envelope & Fill") calculator (`docs/ttrpg/combat/combat.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalPoolParams {
    /// Physical Speed (`SPD`, `1..=20`).
    pub spd: i32,
    /// Physical Force (`FOR`, `1..=20`).
    pub for_stat: i32,
    /// Physical Dexterity (`DEX`, `1..=20`).
    pub dex: i32,
    /// Physical Kinesthesis (`KIN`, `1..=20`).
    pub kin: i32,
    /// Sensory Sight (`SIG`, `1..=20`).
    pub sig: i32,
    /// Mental Insight (`INS`, `1..=20`).
    pub ins: i32,
    /// Sensory Intuition (`ITU`, `1..=20`).
    pub itu: i32,
    /// Mental Analysis (`ANA`, `1..=20`).
    pub ana: i32,
    /// Mental Creativity (`CRE`, `1..=20`).
    pub cre: i32,
    /// Mental Focus (`FOC`, `1..=20`).
    pub foc: i32,
    /// Mental Discipline (`DIS`, `1..=20`).
    pub dis: i32,
    /// Physical Fitness (`FIT`, `1..=20`).
    pub fit: i32,
    /// Character body mass in kilograms (default `70.0 kg`).
    pub mass_kg: f64,
    /// Controlling Characteristic $C_{\text{skill}}$ of the active combat skill (`1..=20`).
    pub skill_c: i32,
    /// Active Combat Skill Level $N \in [1, 10]$.
    pub skill_n: usize,
    /// Whether the action is a ranged weapon exchange (`true`) or melee/unarmed exchange (`false`).
    pub is_ranged: bool,
    /// Accumulated systemic Fatigue Level (`-2` pool per level, `+10%` `action_time_factor` per level).
    pub fatigue_level: i32,
    /// Accumulated systemic Wound Shock Tier (`-2` pool per tier, `+25%` `action_time_factor` per tier).
    pub wound_shock_index: i32,
    /// Localized structural wound penalty to Leg/Hip/Foot (slashes `Tempo Cap`).
    pub leg_wound_penalty: i32,
    /// Localized structural wound penalty to Arm/Wrist/Head/Eye (slashes `Control Cap`).
    pub arm_head_wound_penalty: i32,
    /// Localized structural wound penalty to Arm/Shoulder/Torso (slashes `Force Cap`).
    pub torso_shoulder_wound_penalty: i32,
    /// Active psychological Morale Condition.
    pub morale_state: MoraleState,
}

impl Default for TacticalPoolParams {
    fn default() -> Self {
        Self {
            spd: 10,
            for_stat: 10,
            dex: 10,
            kin: 10,
            sig: 10,
            ins: 10,
            itu: 10,
            ana: 10,
            cre: 10,
            foc: 10,
            dis: 10,
            fit: 10,
            mass_kg: 70.0,
            skill_c: 10,
            skill_n: 3,
            is_ranged: false,
            fatigue_level: 0,
            wound_shock_index: 0,
            leg_wound_penalty: 0,
            arm_head_wound_penalty: 0,
            torso_shoulder_wound_penalty: 0,
            morale_state: MoraleState::Steady,
        }
    }
}

impl TacticalPoolParams {
    /// Build a [`TacticalPoolParams`] directly from a [`CharacterCharacteristics`] profile.
    #[must_use]
    pub fn from_characteristics(
        chars: &CharacterCharacteristics,
        skill_c: i32,
        skill_n: usize,
        is_ranged: bool,
    ) -> Self {
        Self {
            spd: chars.spd,
            for_stat: chars.for_stat,
            dex: chars.dex,
            kin: chars.kin,
            sig: chars.sig,
            ins: chars.ins,
            itu: chars.itu,
            ana: chars.ana,
            cre: chars.cre,
            foc: chars.foc,
            dis: chars.dis,
            fit: chars.fit,
            mass_kg: chars.mas_kg,
            skill_c,
            skill_n,
            is_ranged,
            ..Default::default()
        }
    }
}

/// Computed Tactical Pool ("Fill") and Characteristic Aspect Ceilings ("Envelope") for a combatant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalPoolEnvelope {
    /// Cognitive Decision Score ("Quick Thinking"): $D_{\text{cog}} = \lfloor(\max(\text{ANA}, \text{CRE}) + \text{FOC}) / 2\rfloor$.
    pub d_cog: i32,
    /// 4-Pillar OODA Initiative rating: $\lfloor(\text{ITU} + \text{INS} + D_{\text{cog}} + \text{SPD}) / 4\rfloor$.
    pub ooda_init: i32,
    /// Combat Readiness controller: $C_{\text{combat}} = \lfloor(C_{\text{skill}} + \text{OODA}) / 2\rfloor$.
    pub combat_readiness_c: i32,
    /// Base Tactical Pool before fatigue, shock, or morale: $P_{\text{base}} = C_{\text{combat}} + 3N$.
    pub p_base: i32,
    /// Effective Tactical Pool available to allocate: $P_{\text{eff}} = \max(0, P_{\text{base}} - 2\cdot\text{Fatigue} - 2\cdot\text{Shock} + \Delta P_{\text{morale}})$.
    pub p_eff: i32,
    /// Neutral per-aspect baseline: $B = \lfloor P_{\text{base}} / 3 \rfloor$.
    pub neutral_baseline_b: i32,
    /// Aspect ceiling triad `(tempo_cap, control_cap, force_cap)`.
    pub caps: Triad<i32>,
    /// Maximum points allocatable to **Tempo ($T_{\max}$)**.
    pub tempo_cap: i32,
    /// Maximum points allocatable to **Control ($C_{\max}$)**.
    pub control_cap: i32,
    /// Maximum points allocatable to **Force ($F_{\max}$)**.
    pub force_cap: i32,
    /// Mastery technique overflow bonus ($\lfloor(N - 3)/2\rfloor$ for $N \ge 5$) added to Control & Force caps.
    pub mastery_overflow: i32,
    /// Derived kinetic Power: $\text{POW} = (\text{FOR} \times \text{SPD}) / 10.0$.
    pub power_pow: f64,
    /// Derived Momentum & Mass Factor: $\text{MMF} = (\text{MAS} / 70.0) \times \text{SPD}$.
    pub momentum_mass_factor_mmf: f64,
    /// Derived Fatigue Threshold: $\text{FT} = \text{FIT} + \lfloor\text{DIS} / 2\rfloor$.
    pub fatigue_threshold_ft: i32,
    /// Action Time Factor multiplier: $1.0 + 0.10\cdot\text{Fatigue} + 0.25\cdot\text{Shock}$.
    pub action_time_factor: f64,
    /// Active psychological Morale Condition.
    pub morale_state: MoraleState,
    /// Minimum Force allocation required by Morale/Rage ($\lfloor P_{\text{eff}} / 2 \rfloor$ in `Rage`, else `0`).
    pub min_force_required: i32,
    /// Maximum Control allocation permitted by Morale/Rage + Ceiling.
    pub max_control_allowed: i32,
    /// Maximum Force allocation permitted by Morale/Rage + Ceiling.
    pub max_force_allowed: i32,
    /// Whether `Force` is constrained to not exceed `Control` ($F \le C$, active in `Wavering` morale).
    pub force_must_not_exceed_control: bool,
}

impl TacticalPoolEnvelope {
    /// Return `(tempo_cap, control_cap, force_cap)` as a [`Triad`].
    #[must_use]
    pub const fn caps_triad(&self) -> Triad<i32> {
        self.caps
    }
}

/// Evaluation of a specific `Tempo + Control + Force` Tactical Triad allocation against a [`TacticalPoolEnvelope`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalTriadEvaluation {
    /// Allocated `(tempo, control, force)` triad.
    pub alloc: Triad<i32>,
    /// Whether the allocation satisfies all Pool, Ceiling, and Morale/Rage constraints.
    pub valid: bool,
    /// Human-readable descriptions of any violated constraints (empty when `valid == true`).
    pub violations: Vec<String>,
    /// Effective Count-Up Action Tick Cost (`TC`) after Tempo and `action_time_factor`.
    pub effective_tc: i32,
    /// Scaled melee Strike Impact ($\text{POW} \times (F / B)$).
    pub strike_impact: f64,
    /// Effective Rooted stability against throws, sweeps, and knockback ($F + \text{MMF}$).
    pub effective_root: f64,
    /// Exertion marks accumulated by this action (`0`, `1`, or `2`).
    pub exertion_marks_gained: i32,
    /// Canonical Tactical Archetype classification.
    pub archetype_label: String,
}

impl Deref for TacticalTriadEvaluation {
    type Target = Triad<i32>;

    fn deref(&self) -> &Self::Target {
        &self.alloc
    }
}

impl TacticalTriadEvaluation {
    /// Return `(tempo, control, force)` as a [`Triad`].
    #[must_use]
    pub const fn triad(&self) -> Triad<i32> {
        self.alloc
    }
}

/// Computes the [`TacticalPoolEnvelope`] ("Envelope & Fill") for a combatant.
pub fn compute_tactical_pool(params: &TacticalPoolParams) -> TacticalPoolEnvelope {
    let d_cog = compute_d_cog_score(params.ana, params.cre, params.foc);
    let ooda_init = compute_ooda_score(params.itu, params.ins, d_cog, params.spd);
    let combat_readiness_c = (params.skill_c + ooda_init).div_euclid(2);
    let n_i32 = params.skill_n as i32;
    let p_base = combat_readiness_c + (3 * n_i32);
    let neutral_baseline_b = p_base.div_euclid(3).max(1);

    let morale_mod = params.morale_state.pool_modifier();
    let p_eff =
        (p_base - (2 * params.fatigue_level.max(0)) - (2 * params.wound_shock_index.max(0))
            + morale_mod)
            .max(0);

    let mastery_overflow = if n_i32 >= 5 {
        (n_i32 - 3).div_euclid(2)
    } else {
        0
    };
    let high_morale_bonus = i32::from(params.morale_state == MoraleState::High);

    let fatigue_spd_for_cap_penalty = if params.fatigue_level >= 3 { 2 } else { 0 };
    let is_metabolic_or_shock_unconscious =
        params.fatigue_level >= 5 || params.wound_shock_index >= 5;

    let base_control_attr = if params.is_ranged {
        (params.dex + params.sig).div_euclid(2)
    } else {
        params.kin.max(params.dex)
    };
    let caps = if is_metabolic_or_shock_unconscious {
        Triad::ZERO
    } else {
        Triad::new(
            params.spd - fatigue_spd_for_cap_penalty - params.leg_wound_penalty.max(0)
                + high_morale_bonus,
            base_control_attr + mastery_overflow - params.arm_head_wound_penalty.max(0),
            params.for_stat - fatigue_spd_for_cap_penalty + mastery_overflow + high_morale_bonus
                - params.torso_shoulder_wound_penalty.max(0),
        )
        .clamp_min(0)
    };

    let power_pow = compute_pow_striking_power(params.for_stat, params.spd);
    let momentum_mass_factor_mmf = compute_mmf_momentum_factor(params.mass_kg, params.spd);
    let fatigue_threshold_ft = compute_fatigue_threshold_score(params.fit, params.dis);
    let action_time_factor = 1.0
        + (0.10 * params.fatigue_level.max(0) as f64)
        + (0.25 * params.wound_shock_index.max(0) as f64);

    let (min_force_required, max_control_allowed, max_force_allowed, force_must_not_exceed_control) =
        match params.morale_state {
            MoraleState::High | MoraleState::Steady => (0, caps.control, caps.force, false),
            MoraleState::Rage => (
                p_eff.div_euclid(2),
                caps.control.min(p_eff.div_euclid(4)),
                caps.force,
                false,
            ),
            MoraleState::Wavering => (0, caps.control, caps.force, true),
            MoraleState::Broken => (0, caps.control, 0, false),
            MoraleState::Panic => (0, 0, 0, false),
        };

    TacticalPoolEnvelope {
        d_cog,
        ooda_init,
        combat_readiness_c,
        p_base,
        p_eff,
        neutral_baseline_b,
        caps,
        tempo_cap: caps.tempo,
        control_cap: caps.control,
        force_cap: caps.force,
        mastery_overflow,
        power_pow,
        momentum_mass_factor_mmf,
        fatigue_threshold_ft,
        action_time_factor,
        morale_state: params.morale_state,
        min_force_required,
        max_control_allowed,
        max_force_allowed,
        force_must_not_exceed_control,
    }
}

/// Evaluates a [`Triad`] `(tempo, control, force)` allocation against a [`TacticalPoolEnvelope`]
/// and computes the resulting Count-Up Tick Cost (`TC`), Strike Impact, Root, Exertion Marks, and Archetype.
pub fn evaluate_tactical_triad(
    envelope: &TacticalPoolEnvelope,
    base_weapon_tc: i32,
    alloc: Triad<i32>,
) -> TacticalTriadEvaluation {
    let mut violations = Vec::new();
    let Triad {
        tempo,
        control,
        force,
    } = alloc;

    if alloc.has_negative() {
        violations
            .push("Aspect allocations (Tempo, Control, Force) cannot be negative.".to_string());
    }
    let sum = alloc.sum();
    if sum > envelope.p_eff {
        violations.push(format!(
            "Total allocated points ({sum}) exceed Effective Tactical Pool P_eff ({}).",
            envelope.p_eff
        ));
    }
    if tempo > envelope.caps.tempo && envelope.morale_state != MoraleState::Panic {
        violations.push(format!(
            "Tempo ({tempo}) exceeds Tempo Ceiling T_max ({}).",
            envelope.caps.tempo
        ));
    }
    if control > envelope.max_control_allowed {
        violations.push(format!(
            "Control ({control}) exceeds maximum allowed Control ({}) under {:?} morale.",
            envelope.max_control_allowed, envelope.morale_state
        ));
    }
    if force > envelope.max_force_allowed {
        violations.push(format!(
            "Force ({force}) exceeds maximum allowed Force ({}) under {:?} morale.",
            envelope.max_force_allowed, envelope.morale_state
        ));
    }
    if force < envelope.min_force_required {
        violations.push(format!(
            "Force ({force}) is below minimum required Force ({}) under {:?} morale.",
            envelope.min_force_required, envelope.morale_state
        ));
    }
    if envelope.force_must_not_exceed_control && force > control {
        violations.push(format!(
            "Under Wavering morale, Force ({force}) cannot exceed Control ({control})."
        ));
    }

    let scaled_base_tc = (base_weapon_tc as f64 * envelope.action_time_factor).round() as i32;
    let effective_tc = (scaled_base_tc - (tempo - envelope.neutral_baseline_b)).max(2);

    let strike_impact = envelope.power_pow * (force as f64 / envelope.neutral_baseline_b as f64);
    let effective_root = force as f64 + envelope.momentum_mass_factor_mmf;

    let exertion_marks_gained =
        i32::from(envelope.caps.tempo > 0 && tempo * 2 > envelope.caps.tempo)
            + i32::from(envelope.caps.force > 0 && force * 2 > envelope.caps.force);

    let shares = alloc.map(|x| x as f64 / (sum.max(1) as f64));
    let archetype_label = if shares.tempo >= 0.60 {
        "Pure T — All-Out Tempo (Sprint / Snap / Evade)"
    } else if shares.control >= 0.60 {
        "Pure C — All-Out Control (Turtle Guard / Counter-Trap)"
    } else if shares.force >= 0.60 {
        "Pure F — All-Out Force (Haymaker / Mag-Dump)"
    } else if shares.tempo >= 0.35 && shares.control >= 0.35 && shares.force < 0.25 {
        "TC — Tempo + Control (The Probe / Counter)"
    } else if shares.tempo >= 0.35 && shares.force >= 0.35 && shares.control < 0.25 {
        "TF — Tempo + Force (The Blitz / Dive)"
    } else if shares.control >= 0.35 && shares.force >= 0.35 && shares.tempo < 0.25 {
        "CF — Control + Force (The Finisher / Anchor)"
    } else {
        "Balanced Triad Allocation (T / C / F)"
    }
    .to_string();

    TacticalTriadEvaluation {
        alloc,
        valid: violations.is_empty(),
        violations,
        effective_tc,
        strike_impact,
        effective_root,
        exertion_marks_gained,
        archetype_label,
    }
}

/// Summary of the Combat Tactical Pool ("Envelope & Fill") interactive lab calculation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalPoolEnvelopeSummary {
    /// Maximum capacity of the tactical envelope (`E_base`).
    pub base_envelope: i32,
    /// Uncapped tactical pool generated from cascade successes, skill, and reach (`P_raw`).
    pub raw_pool: i32,
    /// Effective pool available after clamping to `base_envelope` (`P_eff`).
    pub effective_pool: i32,
    /// Excess points above `base_envelope` lost to envelope overflow.
    pub overflow_wasted: i32,
    /// Net weapon reach modifier (`-2..=+3`).
    pub reach_modifier: i32,
    /// Maximum Tempo allocation cap from [`TacticalPoolEnvelope`].
    pub max_tempo: i32,
    /// Maximum Control allocation cap from [`TacticalPoolEnvelope`].
    pub max_control: i32,
    /// Maximum Force allocation cap from [`TacticalPoolEnvelope`].
    pub max_force: i32,
}

/// Computes the Satya Combat Tactical Pool ("Envelope & Fill") interactive lab summary.
pub fn compute_tactical_pool_envelope(
    control_c: i32,
    skill_level: i32,
    delta_int: i32,
    delta_ext: i32,
    num_successes: i32,
    weapon_reach: i32,
    opponent_reach: i32,
) -> TacticalPoolEnvelopeSummary {
    let c_clamped = control_c.clamp(1, 20);
    let n_clamped = skill_level.clamp(0, 10);
    let s_clamped = num_successes.max(0);

    let env = compute_tactical_pool(&TacticalPoolParams {
        spd: c_clamped,
        for_stat: c_clamped,
        dex: c_clamped,
        kin: c_clamped,
        sig: c_clamped,
        ins: c_clamped,
        itu: c_clamped,
        ana: c_clamped,
        cre: c_clamped,
        foc: c_clamped,
        dis: c_clamped,
        fit: c_clamped,
        skill_c: c_clamped,
        skill_n: n_clamped as usize,
        fatigue_level: delta_int.max(0).div_euclid(2),
        ..Default::default()
    });

    let base_envelope = env.caps.sum().max(1);
    let reach_delta = weapon_reach - opponent_reach;
    let reach_modifier = (reach_delta * 2).clamp(-4, 4);
    let raw_pool = (env.p_eff - delta_ext.max(0) + (s_clamped * 2) + reach_modifier).max(0);
    let effective_pool = raw_pool.min(base_envelope);
    let overflow_wasted = (raw_pool - base_envelope).max(0);

    TacticalPoolEnvelopeSummary {
        base_envelope,
        raw_pool,
        effective_pool,
        overflow_wasted,
        reach_modifier,
        max_tempo: env.caps.tempo,
        max_control: env.caps.control,
        max_force: env.caps.force,
    }
}

/// Outcome of a Close-Quarters Tactical Triad (`Tempo`, `Control`, `Force`) contested exchange.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalTriadContestOutcome {
    /// Net Tempo differential (`attacker.tempo - defender.tempo`).
    pub net_tempo: i32,
    /// Net Control differential (`attacker.control - defender.control`).
    pub net_control: i32,
    /// Whether the attacker seizes the initiative (`net_tempo >= 2 && net_control >= 0`).
    pub seizes_initiative: bool,
    /// Whether the defender's guard is broken.
    pub guard_broken: bool,
    /// Whether a Kuzushi throw/sweep is triggered against `attacker_root`.
    pub throw_triggered: bool,
    /// Net penetrating force after armor DR and guard mitigation.
    pub penetrating_force: i32,
    /// Count-Up tick cost of the attacker's next exchange.
    pub next_tick_cost: i32,
    /// Canonical Tactical Archetype classification of the attacker's triad.
    pub archetype_label: String,
    /// Human-readable summary of the exchange outcome.
    pub outcome_summary: String,
}

/// Evaluates a Close-Quarters Tactical Triad (`Tempo`, `Control`, `Force`) exchange.
pub fn evaluate_tactical_triad_contest(
    attacker: Triad<i32>,
    defender: Triad<i32>,
    attacker_root: i32,
    defender_guard: i32,
    armor_dr: i32,
) -> TacticalTriadContestOutcome {
    let env = compute_tactical_pool(&TacticalPoolParams {
        skill_c: 14,
        skill_n: 5,
        ..Default::default()
    });
    let eval_res = evaluate_tactical_triad(&env, 8, attacker);

    let atk = attacker.clamp_min(0);
    let def = defender.clamp_min(0);
    let def_eff_c = def.control + defender_guard;
    let delta = atk - def;

    let net_tempo = delta.tempo;
    let net_control = delta.control;
    let seizes_initiative = net_tempo >= 2 && atk.control >= def_eff_c;
    let guard_broken = (atk.control + (atk.force / 2).max(0)) > def_eff_c.max(0);
    let leveraged_throw_force = atk.force + (atk.control - def_eff_c).max(0);
    let effective_defender_root = def.force + (attacker_root / 3).max(1);
    let throw_triggered = leveraged_throw_force >= effective_defender_root;
    let raw_impact = atk.force + if guard_broken { net_control.max(0) } else { 0 };
    let penetrating_force =
        (raw_impact - armor_dr.max(0) - if guard_broken { 0 } else { def.control }).max(0);
    let defender_initiative_penalty = (def_eff_c - atk.control).max(0);
    let next_tick_cost = (eval_res.effective_tc + defender_initiative_penalty).max(2);

    let outcome_summary = if throw_triggered && penetrating_force >= 3 {
        "Decisive Takedown & Armor Breach (Prone +6 Ticks + Shock)".to_string()
    } else if throw_triggered {
        "Kuzushi Throw / Sweep (Leveraged Force Overcomes Root -> Prone +6 Ticks)".to_string()
    } else if guard_broken && penetrating_force > 0 {
        "Guard Broken & Penetrating Strike".to_string()
    } else if seizes_initiative {
        "Initiative Seized (Tempo & Control Advantage on Next Exchange)".to_string()
    } else if defender_initiative_penalty > 0 && penetrating_force == 0 {
        format!(
            "Defender Seizes Initiative (+{defender_initiative_penalty} Recovery Ticks / Pool Drain on Attacker)"
        )
    } else if penetrating_force > 0 {
        "Glancing Penetrating Blow (Behind-Armor Blunt Trauma)".to_string()
    } else {
        "Deflected on Guard / Bind Equilibrium".to_string()
    };

    TacticalTriadContestOutcome {
        net_tempo,
        net_control,
        seizes_initiative,
        guard_broken,
        throw_triggered,
        penetrating_force,
        next_tick_cost,
        archetype_label: eval_res.archetype_label,
        outcome_summary,
    }
}

// ============================================================================
// Tactical Triad Extensions (Decision Tick Modifier & Exertion Progression)
// ============================================================================

/// Computes the **Cognitive Decision Tick Modifier ($\Delta\text{TC}_{\text{decide}}$)** from
/// `tactical_triad_and_pools.md` §1.2.
pub fn compute_decision_tick_modifier(d_cog: i32, is_complex_or_feint: bool) -> i32 {
    if is_complex_or_feint {
        compute_delta_tc_decide(d_cog)
    } else {
        0
    }
}

/// Evaluates a `Tempo + Control + Force` [`Triad`] allocation including the optional `is_complex_or_feint`
/// Cognitive Decision Tick Modifier $\Delta\text{TC}_{\text{decide}}$ (`tactical_triad_and_pools.md` §1.2).
pub fn evaluate_tactical_triad_with_decision(
    envelope: &TacticalPoolEnvelope,
    base_weapon_tc: i32,
    alloc: Triad<i32>,
    is_complex_or_feint: bool,
) -> TacticalTriadEvaluation {
    let mut eval = evaluate_tactical_triad(envelope, base_weapon_tc, alloc);
    let delta_decide = compute_decision_tick_modifier(envelope.d_cog, is_complex_or_feint);
    let scaled_base_tc = (base_weapon_tc as f64 * envelope.action_time_factor).round() as i32;
    eval.effective_tc =
        (scaled_base_tc - (alloc.tempo - envelope.neutral_baseline_b) + delta_decide).max(2);
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
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeleeWeaponId {
    /// **Unarmed (Jab / Low Kick / Clinch Knee)**: Reach `1`, Base TC `8`, Guard `+0`, Blunt (`1.0x POW`).
    UnarmedJabLowKickClinchKnee = 0,
    /// **Unarmed (Haymaker / High Kick / Throw)**: Reach `1–2`, Base TC `10`, Guard `-1` (strike) / `+1` (grapple), Heavy Blunt (`1.25x POW`).
    UnarmedHaymakerHighKickThrow = 1,
    /// **Combat Knife / Karambit / Shiv**: Reach `1` (Clinch), Base TC `7`, Guard `-1`, Piercing / Lacerating (`+2` Base Bleed).
    CombatKnifeKarambitShiv = 2,
    /// **Expandable Baton / Tonfa / Club**: Reach `2`, Base TC `8`, Guard `+2 Control`, Blunt (`1.2x POW`), `+1` Control on Disarm/Bind.
    ExpandableBatonTonfaClub = 3,
    /// **Machete / Wakizashi / Tactical Blade**: Reach `2–3`, Base TC `9`, Guard `+1 Control`, Lacerating / Impaling (`+4` Base Trauma).
    MacheteWakizashiTacticalBlade = 4,
    /// **Two-Handed Staff / Spear / Riot Shield**: Reach `4`, Base TC `10–11`, Guard `+3 Control`, `+2` effective Force on blocks/sweeps, `+4` $\Delta_{\text{ext}}$ keep-out.
    TwoHandedStaffSpearRiotShield = 5,
}

impl MeleeWeaponId {
    /// All 6 canonical melee weapon identifiers in declaration order.
    pub const ALL: [Self; 6] = [
        Self::UnarmedJabLowKickClinchKnee,
        Self::UnarmedHaymakerHighKickThrow,
        Self::CombatKnifeKarambitShiv,
        Self::ExpandableBatonTonfaClub,
        Self::MacheteWakizashiTacticalBlade,
        Self::TwoHandedStaffSpearRiotShield,
    ];
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

macro_rules! mw {
    ($id:ident, $name:literal, $min_r:expr, $max_r:expr, $tc:expr, $guard:expr, $pow:expr, $flat:expr, $blk:expr, $keep:expr) => {
        MeleeWeaponProfile {
            id: MeleeWeaponId::$id,
            name: $name,
            min_reach: $min_r,
            max_reach: $max_r,
            base_tc: $tc,
            guard_bonus: $guard,
            pow_multiplier: $pow,
            flat_trauma_or_bleed_bonus: $flat,
            block_sweep_force_bonus: $blk,
            has_keep_out_property: $keep,
        }
    };
}

/// Canonical 6-entry Melee Weapon & Unarmed Attack Catalog (`melee_combat.md` §4).
pub const MELEE_WEAPON_CATALOG: [MeleeWeaponProfile; 6] = [
    mw!(UnarmedJabLowKickClinchKnee, "Unarmed (Jab / Low Kick / Clinch Knee)", 1, 1, 8, 0, 1.0, 0, 0, false),
    mw!(UnarmedHaymakerHighKickThrow, "Unarmed (Haymaker / High Kick / Throw)", 1, 2, 10, -1, 1.25, 0, 0, false),
    mw!(CombatKnifeKarambitShiv, "Combat Knife / Karambit / Shiv", 1, 1, 7, -1, 1.0, 2, 0, false),
    mw!(ExpandableBatonTonfaClub, "Expandable Baton / Tonfa / Club", 2, 2, 8, 2, 1.2, 0, 0, false),
    mw!(MacheteWakizashiTacticalBlade, "Machete / Wakizashi / Tactical Blade", 2, 3, 9, 1, 1.15, 4, 0, false),
    mw!(TwoHandedStaffSpearRiotShield, "Two-Handed Staff / Spear / Riot Shield", 4, 4, 10, 3, 1.2, 0, 2, true),
];

impl_static_catalog!(MeleeWeaponId => MeleeWeaponProfile : MELEE_WEAPON_CATALOG);

/// Returns the canonical 6-entry Melee Weapon & Unarmed Attack Catalog (`melee_combat.md` §4).
#[must_use]
pub const fn canonical_melee_weapon_catalog() -> &'static [MeleeWeaponProfile] {
    MeleeWeaponId::CATALOG
}

/// Computes the **Reach Advantage & Clinch Polarity Flip** external penalty $\Delta_{\text{ext}}$ (`melee_combat.md` §4).
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

/// Situational weapon, reach, root, and clinch parameters for [`resolve_contested_melee_exchange`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeleeExchangeContext {
    /// Passive weapon guard bonus added to the defender's `Control`.
    pub defender_guard_bonus: i32,
    /// Defender's Momentum & Mass Factor ($\text{MMF}_{\text{def}}$).
    pub defender_mmf: f64,
    /// Attacker's weapon Reach tier (`1..=4`).
    pub attacker_reach: i32,
    /// Defender's weapon Reach tier (`1..=4`).
    pub defender_reach: i32,
    /// Whether the fighters are locked inside the Clinch (`Reach 1`).
    pub in_clinch: bool,
    /// Whether the defender wields a keep-out weapon (`Reach 4` staff/spear/shield).
    pub defender_has_keep_out_weapon: bool,
    /// Net cascade successes scored by the attacker ($S_{\text{att}} - S_{\text{def}}$).
    pub attacker_net_successes: i32,
    /// Whether the attacker is attempting a Kuzushi Throw or Sweep.
    pub is_throw_or_sweep: bool,
    /// Whether the attacker is redirecting an opponent's high-Tempo blitz inside the clinch.
    pub redirect_opponent_blitz_in_clinch: bool,
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
    /// Recovery ticks added to the attacker's next action (or pool drain on next exchange) when the defender Seizes the Initiative.
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
pub fn resolve_contested_melee_exchange(
    attacker: Triad<i32>,
    defender: Triad<i32>,
    ctx: &MeleeExchangeContext,
) -> MeleeExchangeResolution {
    let defender_effective_control = (defender.control + ctx.defender_guard_bonus).max(0);
    let delta_control = defender_effective_control - attacker.control;
    let control_delta_ext_on_attacker = delta_control.max(0);
    let reach_clinch_delta_ext = compute_reach_and_clinch_delta_ext(
        ctx.attacker_reach,
        ctx.defender_reach,
        ctx.in_clinch,
        attacker.tempo,
        defender.tempo,
        ctx.defender_has_keep_out_weapon,
    );

    let strike_lands = ctx.attacker_net_successes > 0;
    let defender_seizes_initiative =
        !strike_lands && defender_effective_control > attacker.control;
    let initiative_seized_penalty = if defender_seizes_initiative {
        defender_effective_control - attacker.control
    } else {
        0
    };

    let redirected_momentum =
        if ctx.is_throw_or_sweep && ctx.in_clinch && ctx.redirect_opponent_blitz_in_clinch {
            defender.tempo.max(0) as f64
        } else {
            0.0
        };
    let leveraged_throw_force = attacker.force as f64
        + (attacker.control - defender_effective_control).max(0) as f64
        + redirected_momentum;
    let defender_effective_root = defender.force as f64 + ctx.defender_mmf;

    let kuzushi_throw_succeeded =
        ctx.is_throw_or_sweep && strike_lands && leveraged_throw_force >= defender_effective_root;
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
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RangedFireModeId {
    /// **Single Snap Shot (Point-Shoot)**: Base TC `7`, Recoil `3–4`, prioritizes `Tempo` (`CQB`).
    SingleSnapShotPointShoot = 0,
    /// **Deliberate Sighted / Aimed Shot**: Base TC `11–12`, Recoil `4–6`, prioritizes `Control` (called shots, weak spots).
    DeliberateSightedAimedShot = 1,
    /// **Controlled Double-Tap / Short Burst**: Base TC `9`, Recoil `6–7`, balanced `Control + Force` (`+1` hit tier per `2` Net Successes).
    ControlledDoubleTapShortBurst = 2,
    /// **Full-Auto Burst / Suppressive Fire**: Base TC `8–10`, Recoil `8–12`, prioritizes `Force` ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$).
    FullAutoBurstSuppressiveFire = 3,
    /// **Bounding / Shooting on the Move**: Base TC `10`, `+3` unbraced recoil added to weapon `R`.
    BoundingShootingOnTheMove = 4,
}

impl RangedFireModeId {
    /// All 5 canonical ranged fire mode identifiers in declaration order.
    pub const ALL: [Self; 5] = [
        Self::SingleSnapShotPointShoot,
        Self::DeliberateSightedAimedShot,
        Self::ControlledDoubleTapShortBurst,
        Self::FullAutoBurstSuppressiveFire,
        Self::BoundingShootingOnTheMove,
    ];
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

macro_rules! fm {
    ($id:ident, $name:literal, $min_tc:expr, $max_tc:expr, $min_r:expr, $max_r:expr, $move_r:expr) => {
        RangedFireModeProfile {
            id: RangedFireModeId::$id,
            name: $name,
            min_base_tc: $min_tc,
            max_base_tc: $max_tc,
            min_recoil_r: $min_r,
            max_recoil_r: $max_r,
            movement_recoil_adder: $move_r,
        }
    };
}

/// Canonical 5-entry Ranged Fire Mode catalog (`ranged_combat_and_armor.md` §1.1).
pub const RANGED_FIRE_MODES: [RangedFireModeProfile; 5] = [
    fm!(SingleSnapShotPointShoot, "Single Snap Shot (Point-Shoot)", 7, 7, 3, 4, 0),
    fm!(DeliberateSightedAimedShot, "Deliberate Sighted / Aimed Shot", 11, 12, 4, 6, 0),
    fm!(ControlledDoubleTapShortBurst, "Controlled Double-Tap / Short Burst", 9, 9, 6, 7, 0),
    fm!(FullAutoBurstSuppressiveFire, "Full-Auto Burst / Suppressive Fire", 8, 10, 8, 12, 0),
    fm!(BoundingShootingOnTheMove, "Bounding / Shooting on the Move", 10, 10, 6, 10, 3),
];

impl_static_catalog!(RangedFireModeId => RangedFireModeProfile : RANGED_FIRE_MODES);

/// Returns the canonical 5-entry Ranged Fire Mode catalog (`ranged_combat_and_armor.md` §1.1).
#[must_use]
pub const fn canonical_ranged_fire_modes() -> &'static [RangedFireModeProfile] {
    RangedFireModeId::CATALOG
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
    allocated_force: i32,
    weapon_recoil_r: i32,
    movement_gait: crate::health_survival::CombatMovementGait,
    is_suppressive_or_full_auto: bool,
) -> RangedExchangeEvaluation {
    evaluate_ranged_recoil_and_suppression(
        allocated_force,
        weapon_recoil_r.max(0) + movement_gait.ranged_recoil_adder(),
        false,
        is_suppressive_or_full_auto,
    )
}

// ============================================================================
// Armor Catalog & Penetration (PR) vs. Behind-Armor Blunt Trauma (BABT)
// ============================================================================

/// Canonical Armor Profile identifiers (`ranged_combat_and_armor.md` §2.1).
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmorProfileId {
    /// **Heavy Leather / Synth-Weave Jacket**: `PR_Edged 2`, `PR_Ballistic 1`, `BTD 1`, `2.5 kg`.
    HeavyLeatherSynthWeaveJacket = 0,
    /// **Concealable Soft Kevlar Vest (NIJ IIIA)**: `PR_Edged 2` (`1` vs. icepick/flechette), `PR_Ballistic 4`, `BTD 2`, `2.8 kg`.
    ConcealableSoftKevlarVestNij3a = 1,
    /// **Tactical Plate Carrier (Level IV Ceramic)**: `PR_Edged 6`, `PR_Ballistic 8`, `BTD 5`, `8.5 kg`.
    TacticalPlateCarrierLevel4 = 2,
    /// **Ballistic Combat Helmet + Visor**: `PR_Edged 5`, `PR_Ballistic 4` (`3` visor), `BTD 4`, `1.6 kg`.
    BallisticCombatHelmetVisor = 3,
    /// **Full Riot / Breacher Suit**: `PR_Edged 6`, `PR_Ballistic 5`, `BTD 6`, `16.0 kg`, `+6` Heat Retention.
    FullRiotBreacherSuit = 4,
    /// **2035 "Valkyrie" Recon Exo-Carapace**: `PR_Edged 8`, `PR_Ballistic 9`, `BTD 8`, `18.5 kg` (`-20.0 kg` Exo Offset).
    ValkyrieReconExoCarapace2035 = 5,
}

impl ArmorProfileId {
    /// All 6 canonical armor profile identifiers in declaration order.
    pub const ALL: [Self; 6] = [
        Self::HeavyLeatherSynthWeaveJacket,
        Self::ConcealableSoftKevlarVestNij3a,
        Self::TacticalPlateCarrierLevel4,
        Self::BallisticCombatHelmetVisor,
        Self::FullRiotBreacherSuit,
        Self::ValkyrieReconExoCarapace2035,
    ];
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

macro_rules! ar {
    ($id:ident, $name:literal, $edged:expr, $ice:expr, $bal:expr, $vis:expr, $btd:expr, $wt:expr, $exo:expr, $heat:expr) => {
        ArmorProfile {
            id: ArmorProfileId::$id,
            name: $name,
            pr_edged: $edged,
            pr_icepick_flechette: $ice,
            pr_ballistic: $bal,
            pr_visor_weak_spot: $vis,
            blunt_trauma_dampening_btd: $btd,
            weight_kg: $wt,
            exo_load_offset_kg: $exo,
            heat_retention_delta: $heat,
        }
    };
}

/// Canonical 6-entry Armor Catalog (`ranged_combat_and_armor.md` §2.1).
pub const ARMOR_CATALOG: [ArmorProfile; 6] = [
    ar!(HeavyLeatherSynthWeaveJacket, "Heavy Leather / Synth-Weave Jacket", 2, 2, 1, 1, 1, 2.5, 0.0, 0),
    ar!(ConcealableSoftKevlarVestNij3a, "Concealable Soft Kevlar Vest (NIJ IIIA)", 2, 1, 4, 4, 2, 2.8, 0.0, 0),
    ar!(TacticalPlateCarrierLevel4, "Tactical Plate Carrier (Level IV Ceramic)", 6, 6, 8, 8, 5, 8.5, 0.0, 0),
    ar!(BallisticCombatHelmetVisor, "Ballistic Combat Helmet + Visor", 5, 5, 4, 3, 4, 1.6, 0.0, 0),
    ar!(FullRiotBreacherSuit, "Full Riot / Breacher Suit", 6, 6, 5, 5, 6, 16.0, 0.0, 6),
    ar!(ValkyrieReconExoCarapace2035, "2035 \"Valkyrie\" Recon Exo-Carapace", 8, 8, 9, 9, 8, 18.5, 20.0, 0),
];

impl_static_catalog!(ArmorProfileId => ArmorProfile : ARMOR_CATALOG);

/// Returns the canonical 6-entry Armor Catalog (`ranged_combat_and_armor.md` §2.1).
#[must_use]
pub const fn canonical_armor_catalog() -> &'static [ArmorProfile] {
    ArmorProfileId::CATALOG
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

/// Resolves **Armor Penetration (`PR`) vs. Behind-Armor Blunt Trauma (`BABT`)** (`ranged_combat_and_armor.md` §2.2).
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
    #[must_use]
    pub const fn reliability_rating(self) -> i32 {
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

/// Evaluates whether a [`CascadeRollOutcome`] triggers a Weapon or Equipment Malfunction (`ranged_combat_and_armor.md` §3.2).
pub fn evaluate_weapon_malfunction(
    roll: &CascadeRollOutcome,
    reliability_rel: i32,
    environmental_fouling: i32,
) -> WeaponMalfunctionOutcome {
    let rel = reliability_rel.clamp(-4, 4);
    let fouling = environmental_fouling.max(0);

    let (malfunction_kind, clearance_ticks, tool_tier_degradation) = match roll.outcome_kind {
        CascadeOutcomeKind::SuccessWithComplication if roll.raw_botches > rel => {
            (MalfunctionKind::MinorMalfunctionStovepipe, 8, 1)
        }
        CascadeOutcomeKind::BotchFailure => {
            if roll.effective_botches + fouling > 2 + rel {
                (MalfunctionKind::MajorMalfunctionDoubleFeed, 26, 2)
            } else {
                (MalfunctionKind::MinorMalfunctionStovepipe, 8, 1)
            }
        }
        _ => (MalfunctionKind::None, 0, 0),
    };

    WeaponMalfunctionOutcome {
        effective_reliability: rel,
        malfunction_kind,
        clearance_ticks,
        tool_tier_degradation,
    }
}

/// Applies `30 minutes` of Downtime **Preventative Maintenance** with *Gunsmithing & Armorer* (`DEX/ANA`) (`ranged_combat_and_armor.md` §3.2).
pub fn apply_preventative_maintenance(
    base_reliability: i32,
    _current_fouling: i32,
    armorer_successes: i32,
) -> (i32, i32) {
    let bonus = i32::from(armorer_successes >= 1);
    ((base_reliability + bonus).clamp(-4, 4), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psyche_social::{
        CombatMotivationKind, PsycheProfile, evaluate_combat_morale_roll,
        evaluate_combat_motivation_modifier,
    };

    #[test]
    fn test_viktor_vs_elena_melee_walkthrough() {
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
        assert_eq!((elena_env.p_base, elena_env.neutral_baseline_b), (24, 8));
        assert_eq!((viktor_env.p_base, viktor_env.neutral_baseline_b), (16, 5));
        assert_eq!(viktor_env.caps, Triad::new(9, 10, 15));
        assert!((viktor_env.power_pow - 13.5).abs() < 1e-6);

        let elena_ex1 = evaluate_tactical_triad(&elena_env, 8, Triad::new(11, 11, 2));
        assert!(elena_ex1.valid && elena_ex1.effective_tc == 5);

        viktor_params.leg_wound_penalty = 2;
        viktor_params.morale_state = MoraleState::Rage;
        let viktor_rage_env = compute_tactical_pool(&viktor_params);
        assert_eq!(
            (viktor_rage_env.p_eff, viktor_rage_env.tempo_cap, viktor_rage_env.min_force_required, viktor_rage_env.max_control_allowed),
            (18, 7, 9, 4)
        );

        let viktor_haymaker = evaluate_tactical_triad(&viktor_rage_env, 10, Triad::new(3, 3, 12));
        assert!(viktor_haymaker.valid && viktor_haymaker.effective_tc == 12);

        let elena_ex2 = evaluate_tactical_triad(&elena_env, 8, Triad::new(6, 10, 8));
        assert!(elena_ex2.valid);
        let (interrupted, counter_land_tick) =
            check_melee_interrupt_before_impact(15, 6, elena_ex2.effective_tc - 2, true);
        assert!(interrupted && counter_land_tick < 15);

        viktor_params.morale_state = MoraleState::Steady;
        viktor_params.wound_shock_index = 2;
        viktor_params.fatigue_level = 1;
        let viktor_after = compute_tactical_pool(&viktor_params);
        assert_eq!(viktor_after.p_eff, 10);
    }

    #[test]
    fn test_clinch_and_hip_throw_vs_knife_blitz() {
        let res = resolve_contested_melee_exchange(
            Triad::new(4, 12, 8),
            Triad::new(10, 2, 8),
            &MeleeExchangeContext {
                defender_guard_bonus: -1,
                defender_mmf: 12.0,
                attacker_reach: 1,
                defender_reach: 1,
                in_clinch: true,
                defender_has_keep_out_weapon: false,
                attacker_net_successes: 2,
                is_throw_or_sweep: true,
                redirect_opponent_blitz_in_clinch: true,
            },
        );
        assert!(res.kuzushi_throw_succeeded);
        assert!(res.leveraged_throw_force >= res.defender_effective_root);
        assert_eq!(res.prone_recovery_ticks, 6);
    }

    #[test]
    fn test_mateo_vs_sgt_vance_firefight_suppression_walkthrough() {
        let mateo_ranged = evaluate_ranged_recoil_and_suppression(8, 8, false, true);
        assert!(mateo_ranged.recoil_fully_braced);
        assert_eq!((mateo_ranged.unbraced_dispersion_delta_ext, mateo_ranged.suppression_morale_delta), (0, 4));

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

        let vance_ranged = evaluate_ranged_recoil_and_suppression(5, 5, false, false);
        assert!(vance_ranged.recoil_fully_braced);

        let mateo_profile = PsycheProfile { slf: 15, blg: 7, ..Default::default() };
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
            &[(4, 4)],
            (10, 10),
        );
        assert_eq!(mateo_morale_roll.new_morale, MoraleState::Broken);

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
        assert_eq!((mateo_broken_env.p_base, mateo_broken_env.p_eff, mateo_broken_env.max_force_allowed, mateo_broken_env.control_cap), (18, 10, 0, 3));

        let scramble = evaluate_tactical_triad(&mateo_broken_env, 8, Triad::new(7, 3, 0));
        assert!(scramble.valid);
    }

    #[test]
    fn test_sniper_overwatch_and_grenade_dive_walkthrough() {
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
        let bound1 = evaluate_tactical_triad(&elena_env, 8, Triad::new(12, 12, 0));
        assert!(bound1.valid && bound1.archetype_label.starts_with("TC") && bound1.exertion_marks_gained == 1);

        let bound2 = evaluate_tactical_triad(&elena_env, 8, Triad::new(12, 2, 10));
        assert!(bound2.valid && bound2.archetype_label.starts_with("TF") && bound2.exertion_marks_gained == 2);
    }

    #[test]
    fn test_armor_penetration_vs_babt_and_weapon_malfunctions() {
        let kevlar = ArmorProfileId::ConcealableSoftKevlarVestNij3a.spec();
        let stopped = resolve_armor_impact(kevlar, WeaponDamageType::Ballistic, 6, 2, 1, false);
        assert!(!stopped.penetrated && !stopped.causes_bleed && stopped.babt_trauma == 4 && stopped.total_effective_trauma == 4);

        let overmatched = resolve_armor_impact(kevlar, WeaponDamageType::Ballistic, 9, 4, 2, false);
        assert!(overmatched.penetrated && overmatched.causes_bleed && overmatched.penetrating_trauma == 5 && overmatched.armor_pr_degradation == 1);

        let icepick = resolve_armor_impact(kevlar, WeaponDamageType::IcepickOrFlechette, 5, 1, 2, false);
        assert_eq!((icepick.active_armor_pr, icepick.effective_pen), (1, 2));
        assert!(icepick.penetrated);
    }
}
