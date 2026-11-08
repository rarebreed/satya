//! # Tactical Triad Extensions, Melee Combat, Ranged Firepower, Armor & Reliability (`05-combat`)
//!
//! Canonical Rust implementation of `docs/ttrpg/05-combat/`:
//! - **`docs/ttrpg/05-combat/tactical_triad_and_pools.md`**: Tactical Pool ("Envelope & Fill", `§3`),
//!   Tempo & Decision Tick Modifier ($\Delta\text{TC}_{\text{decide}} = -\lfloor(D_{\text{cog}}-10)/2\rfloor$, `§4.1`),
//!   Control (`§4.2`), Force (`§4.3`), and Exertion Mark accumulation against Fatigue Threshold
//!   ($\text{FT} = \text{FIT} + \lfloor\text{DIS}/2\rfloor$, `§4.4`).
//! - **`docs/ttrpg/05-combat/melee_combat.md`**: Contested Melee Exchanges (`§1`), Interrupting Telegraphed
//!   Strikes (`§1.2`), Counter-Hits & Seizing the Initiative (`§2.2`), Kuzushi Throws vs. Root (`§2.3`, `§5.2`),
//!   Starting Combat Modes (`§3.3`), Melee Weapon Catalog & Reach/Clinch Polarity Flip (`§4`), and Viktor vs. Elena
//!   Walkthrough (`§5.1`).
//! - **`docs/ttrpg/05-combat/ranged_combat_and_armor.md`**: Ranged Fire Modes (`§2.1`), Recoil Bracing ($F \ge R$)
//!   & Muzzle Climb Dispersion (`§2.2`), Suppressive Fire Morale Shock ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$, `§2.3`),
//!   Effective Penetration & Impact Scaling (`§4.2`), Weapon Reliability (`Rel` in `-4..=+4`) & Malfunction Mechanics
//!   (`§6.1–§6.2`), and Armor Profiles Catalog (`§8.1`) & Penetration (`PR`) vs. Behind-Armor Blunt Trauma (`BABT`, `§8.2`).

use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::action_economy::compute_delta_tc_decide;
pub use crate::ballistics::*;
use crate::characteristics::{
    CharacterCharacteristics, compute_d_cog_score, compute_fatigue_threshold_score,
    compute_mmf_momentum_factor, compute_ooda_score, compute_pow_striking_power,
};
use crate::primitives::{ConditionImpact, StaticCatalog, Triad};
use crate::{define_coded_enum, impl_static_catalog};

pub use crate::combat_techniques::*;
pub use crate::ranged_combat::*;
// ============================================================================
// Combat Tactical Pool ("Envelope & Fill" — Tempo, Control, Force) Calculator
// ============================================================================

define_coded_enum! {
    /// Psychological & Morale condition governing a combatant's Tactical Pool size and allocation constraints
    /// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §3.2).
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
    /// Returns the Tactical Pool modifier ($\Delta P_{\text{morale}}$) for this morale state
    /// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §3.2).
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
/// Satya Combat Tactical Pool ("Envelope & Fill") calculator (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §3–§4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalPoolParams {
    pub chars: CharacterCharacteristics,
    /// Controlling Characteristic $C_{\text{skill}}$ of the active combat skill (`1..=20`).
    pub skill_c: i32,
    pub skill_n: usize,
    pub is_ranged: bool,
    pub fatigue_level: i32,
    pub wound_shock_index: i32,
    /// Localized structural wound penalties to `(Tempo, Control, Force)` ceilings (`(leg_wound, arm_head_wound, torso_shoulder_wound)`).
    pub wound_cap_penalties: Triad<i32>,
    pub condition: ConditionImpact,
    pub morale_state: MoraleState,
}

impl Default for TacticalPoolParams {
    fn default() -> Self {
        Self {
            chars: CharacterCharacteristics::default(),
            skill_c: 10,
            skill_n: 3,
            is_ranged: false,
            fatigue_level: 0,
            wound_shock_index: 0,
            wound_cap_penalties: Triad::ZERO,
            condition: ConditionImpact::ZERO,
            morale_state: MoraleState::Steady,
        }
    }
}

impl TacticalPoolParams {
    #[must_use]
    pub fn from_characteristics(
        chars: &CharacterCharacteristics,
        skill_c: i32,
        skill_n: usize,
        is_ranged: bool,
    ) -> Self {
        Self {
            chars: chars.clone(),
            skill_c,
            skill_n,
            is_ranged,
            ..Default::default()
        }
    }
}

/// Computed Tactical Pool ("Fill") and Characteristic Aspect Ceilings ("Envelope") for a combatant
/// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §3–§4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalPoolEnvelope {
    /// Cognitive Decision Score ("Quick Thinking"): $D_{\text{cog}} = \lfloor(\max(\text{ANA}, \text{CRE}) + \text{FOC}) / 2\rfloor$.
    pub d_cog: i32,
    /// 4-Pillar OODA Initiative rating: $\lfloor(\text{ITU} + \text{INS} + D_{\text{cog}} + \text{SPD}) / 4\rfloor$.
    pub ooda_init: i32,
    /// Combat Readiness controller: $C_{\text{combat}} = \lfloor(C_{\text{skill}} + \text{OODA}) / 2\rfloor$.
    pub combat_readiness_c: i32,
    pub p_base: i32,
    /// Effective Tactical Pool available to allocate: $P_{\text{eff}} = \max(0, P_{\text{base}} - 2\cdot\text{Fatigue} - 2\cdot\text{Shock} + \Delta P_{\text{morale}})$.
    pub p_eff: i32,
    /// Neutral per-aspect baseline: $B = \lfloor P_{\text{base}} / 3 \rfloor$.
    pub neutral_baseline_b: i32,
    pub caps: Triad<i32>,
    pub tempo_cap: i32,
    pub control_cap: i32,
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
    pub morale_state: MoraleState,
    /// Minimum Force allocation required by Morale/Rage ($\lfloor P_{\text{eff}} / 2 \rfloor$ in `Rage`, else `0`).
    pub min_force_required: i32,
    pub max_control_allowed: i32,
    pub max_force_allowed: i32,
    pub force_must_not_exceed_control: bool,
}

impl TacticalPoolEnvelope {
    #[must_use]
    pub const fn caps_triad(&self) -> Triad<i32> {
        self.caps
    }
}

/// Evaluation of a specific `Tempo + Control + Force` Tactical Triad allocation against a [`TacticalPoolEnvelope`]
/// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §4.1–§4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalTriadEvaluation {
    pub alloc: Triad<i32>,
    pub valid: bool,
    pub violations: Vec<String>,
    pub effective_tc: i32,
    /// Scaled melee Strike Impact ($\text{POW} \times (F / B)$).
    pub strike_impact: f64,
    /// Effective Rooted stability against throws, sweeps, and knockback ($F + \text{MMF}$).
    pub effective_root: f64,
    pub exertion_marks_gained: i32,
    pub archetype_label: String,
}

impl Deref for TacticalTriadEvaluation {
    type Target = Triad<i32>;

    fn deref(&self) -> &Self::Target {
        &self.alloc
    }
}

impl TacticalTriadEvaluation {
    #[must_use]
    pub const fn triad(&self) -> Triad<i32> {
        self.alloc
    }
}

/// Computes the [`TacticalPoolEnvelope`] ("Envelope & Fill") for a combatant
/// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §3).
pub fn compute_tactical_pool(params: &TacticalPoolParams) -> TacticalPoolEnvelope {
    let c = &params.chars;
    let d_cog = compute_d_cog_score(c.ana, c.cre, c.foc);
    let ooda_init = compute_ooda_score(c.itu, c.ins, d_cog, c.spd);
    let combat_readiness_c = (params.skill_c + ooda_init).div_euclid(2);
    let n_i32 = params.skill_n as i32;
    let p_base = combat_readiness_c + (3 * n_i32);
    let neutral_baseline_b = p_base.div_euclid(3).max(1);

    let morale_mod = params.morale_state.pool_modifier();
    let p_eff = (p_base
        - (2 * params.fatigue_level.max(0))
        - (2 * params.wound_shock_index.max(0))
        + morale_mod
        + params.condition.tactical_pool_mod)
        .max(0);

    let mastery_overflow = if n_i32 >= 5 {
        (n_i32 - 3).div_euclid(2)
    } else {
        0
    };
    let high_morale_bonus = i32::from(params.morale_state == MoraleState::High);

    let fatigue_spd_for_cap_penalty = if params.fatigue_level >= 3 { 2 } else { 0 };
    let is_metabolic_or_shock_unconscious = params.condition.is_unconscious
        || params.fatigue_level >= 5
        || params.wound_shock_index >= 5;

    let base_control_attr = if params.is_ranged {
        (c.dex + c.sig).div_euclid(2)
    } else {
        c.kin.max(c.dex)
    };
    let caps = if is_metabolic_or_shock_unconscious {
        Triad::ZERO
    } else {
        let base_caps = Triad::new(
            c.spd - fatigue_spd_for_cap_penalty + high_morale_bonus,
            base_control_attr + mastery_overflow,
            c.for_stat - fatigue_spd_for_cap_penalty + mastery_overflow + high_morale_bonus,
        );
        (base_caps
            - params.wound_cap_penalties.clamp_min(0)
            - params.condition.triad_cap_penalty.clamp_min(0))
        .clamp_min(0)
    };

    let power_pow = compute_pow_striking_power(c.for_stat, c.spd);
    let momentum_mass_factor_mmf = compute_mmf_momentum_factor(c.mas_kg, c.spd);
    let fatigue_threshold_ft = compute_fatigue_threshold_score(c.fit, c.dis);
    let action_time_factor = 1.0
        + (0.10 * f64::from(params.fatigue_level.max(0)))
        + (0.25 * f64::from(params.wound_shock_index.max(0)))
        + params.condition.action_time_factor_delta;

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
/// and computes the resulting Count-Up Tick Cost (`TC`), Strike Impact, Root, Exertion Marks, and Archetype
/// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §4.1–§4.4).
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

/// Summary of the Combat Tactical Pool ("Envelope & Fill") interactive lab calculation
/// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalPoolEnvelopeSummary {
    pub base_envelope: i32,
    pub raw_pool: i32,
    pub effective_pool: i32,
    pub overflow_wasted: i32,
    /// Net weapon reach modifier (`-2..=+3`).
    pub reach_modifier: i32,
    pub max_tempo: i32,
    pub max_control: i32,
    pub max_force: i32,
}

/// Computes the Satya Combat Tactical Pool ("Envelope & Fill") interactive lab summary
/// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §3).
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
        chars: CharacterCharacteristics {
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
            ..Default::default()
        },
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

/// Outcome of a Close-Quarters Tactical Triad (`Tempo`, `Control`, `Force`) contested exchange
/// (`docs/ttrpg/05-combat/melee_combat.md` §1–§2.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalTriadContestOutcome {
    pub net_tempo: i32,
    pub net_control: i32,
    /// Whether the attacker seizes the initiative (`net_tempo >= 2 && net_control >= 0`).
    pub seizes_initiative: bool,
    pub guard_broken: bool,
    pub throw_triggered: bool,
    pub penetrating_force: i32,
    pub next_tick_cost: i32,
    pub archetype_label: String,
    pub outcome_summary: String,
}

/// Evaluates a Close-Quarters Tactical Triad (`Tempo`, `Control`, `Force`) exchange
/// (`docs/ttrpg/05-combat/melee_combat.md` §1–§2.3).
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
    let seizes_initiative = net_tempo >= 2 && net_control >= 0;
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
/// `docs/ttrpg/05-combat/tactical_triad_and_pools.md` §4.1.
pub fn compute_decision_tick_modifier(d_cog: i32, is_complex_or_feint: bool) -> i32 {
    if is_complex_or_feint {
        compute_delta_tc_decide(d_cog)
    } else {
        0
    }
}

/// Evaluates a `Tempo + Control + Force` [`Triad`] allocation including the optional `is_complex_or_feint`
/// Cognitive Decision Tick Modifier $\Delta\text{TC}_{\text{decide}}$
/// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §4.1).
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

/// Result of accumulating Exertion Marks after a combat exchange
/// (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExertionProgressionResult {
    pub remaining_exertion_marks: i32,
    /// Updated systemic Fatigue Level (`-2` $P_{\text{eff}}$, `+2` $\Delta_{\text{int}}$, `+10%` `action_time_factor` per level).
    pub new_fatigue_level: i32,
    pub fatigue_levels_gained: i32,
}

/// Accumulates `exertion_marks_gained` against the combatant's Fatigue Threshold
/// $\text{FT} = \text{FIT} + \lfloor\text{DIS}/2\rfloor$ (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §4.4).
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

/// Starting Combat Engagement Mode declared when entering a combat scene
/// (`docs/ttrpg/05-combat/melee_combat.md` §3.3).
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

/// Canonical Melee Weapon & Unarmed Attack identifiers from `docs/ttrpg/05-combat/melee_combat.md` §4.
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeleeWeaponId {
    UnarmedJabLowKickClinchKnee = 0,
    UnarmedHaymakerHighKickThrow = 1,
    CombatKnifeKarambitShiv = 2,
    ExpandableBatonTonfaClub = 3,
    MacheteWakizashiTacticalBlade = 4,
    TwoHandedStaffSpearRiotShield = 5,
}

impl MeleeWeaponId {
    pub const ALL: [Self; 6] = [
        Self::UnarmedJabLowKickClinchKnee,
        Self::UnarmedHaymakerHighKickThrow,
        Self::CombatKnifeKarambitShiv,
        Self::ExpandableBatonTonfaClub,
        Self::MacheteWakizashiTacticalBlade,
        Self::TwoHandedStaffSpearRiotShield,
    ];
}

/// Canonical profile for a Melee Weapon or Unarmed Attack (`docs/ttrpg/05-combat/melee_combat.md` §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeleeWeaponProfile {
    pub id: MeleeWeaponId,
    pub name: &'static str,
    pub min_reach: i32,
    pub max_reach: i32,
    pub base_tc: i32,
    /// Passive Guard bonus added to `Control` when parrying/defending (`-1..=+3`).
    pub guard_bonus: i32,
    /// Multiplier applied to $\text{POW} \times (F / B)$ strike impact (`1.0`, `1.2`, `1.25`, etc.).
    pub pow_multiplier: f64,
    /// Flat bonus added to base trauma or bleed (`+2` knife bleed, `+4` tactical blade trauma).
    pub flat_trauma_or_bleed_bonus: i32,
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

/// Canonical 6-entry Melee Weapon & Unarmed Attack Catalog (`docs/ttrpg/05-combat/melee_combat.md` §4).
pub const MELEE_WEAPON_CATALOG: [MeleeWeaponProfile; 6] = [
    mw!(UnarmedJabLowKickClinchKnee, "Unarmed (Jab / Low Kick / Clinch Knee)", 1, 1, 8, 0, 1.0, 0, 0, false),
    mw!(UnarmedHaymakerHighKickThrow, "Unarmed (Haymaker / High Kick / Throw)", 1, 2, 10, -1, 1.25, 0, 0, false),
    mw!(CombatKnifeKarambitShiv, "Combat Knife / Karambit / Shiv", 1, 1, 7, -1, 1.0, 2, 0, false),
    mw!(ExpandableBatonTonfaClub, "Expandable Baton / Tonfa / Club", 2, 2, 8, 2, 1.2, 0, 0, false),
    mw!(MacheteWakizashiTacticalBlade, "Machete / Wakizashi / Tactical Blade", 2, 3, 9, 1, 1.15, 4, 0, false),
    mw!(TwoHandedStaffSpearRiotShield, "Two-Handed Staff / Spear / Riot Shield", 4, 4, 10, 3, 1.2, 0, 2, true),
];

impl_static_catalog!(MeleeWeaponId => MeleeWeaponProfile : MELEE_WEAPON_CATALOG);

impl MeleeWeaponId {
    /// Returns the default [`CombatDomain`] for this weapon profile (`UnarmedMartialArts` vs. `ArmedMelee`).
    #[must_use]
    pub const fn combat_domain(self) -> crate::combat_techniques::CombatDomain {
        match self {
            Self::UnarmedJabLowKickClinchKnee | Self::UnarmedHaymakerHighKickThrow => {
                crate::combat_techniques::CombatDomain::UnarmedMartialArts
            }
            Self::CombatKnifeKarambitShiv
            | Self::ExpandableBatonTonfaClub
            | Self::MacheteWakizashiTacticalBlade
            | Self::TwoHandedStaffSpearRiotShield => {
                crate::combat_techniques::CombatDomain::ArmedMelee
            }
        }
    }
}

#[must_use]
pub const fn canonical_melee_weapon_catalog() -> &'static [MeleeWeaponProfile] {
    MeleeWeaponId::CATALOG
}

/// Computes the **Reach Advantage & Clinch Polarity Flip** external penalty $\Delta_{\text{ext}}$
/// (`docs/ttrpg/05-combat/melee_combat.md` §4).
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

/// Situational weapon, reach, root, and clinch parameters for [`resolve_contested_melee_exchange`]
/// (`docs/ttrpg/05-combat/melee_combat.md` §1, §2.2, §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeleeExchangeContext {
    /// Passive weapon guard bonus added to the defender's `Control`.
    pub defender_guard_bonus: i32,
    /// Defender's Momentum & Mass Factor ($\text{MMF}_{\text{def}}$).
    pub defender_mmf: f64,
    pub attacker_reach: i32,
    pub defender_reach: i32,
    pub in_clinch: bool,
    pub defender_has_keep_out_weapon: bool,
    pub attacker_net_successes: i32,
    pub is_throw_or_sweep: bool,
    pub redirect_opponent_blitz_in_clinch: bool,
}

/// Outcome of a Contested Melee Exchange (`docs/ttrpg/05-combat/melee_combat.md` §1, §2.2, §2.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeleeExchangeResolution {
    pub defender_effective_control: i32,
    /// Control Differential $\Delta_{\text{control}} = (C_{\text{def}} + \text{Guard}_{\text{def}}) - C_{\text{att}}$.
    pub delta_control: i32,
    /// External penalty $\Delta_{\text{ext}}$ imposed on the attacker by the defender's superior Control ($\max(0, \Delta_{\text{control}})$).
    pub control_delta_ext_on_attacker: i32,
    /// External penalty $\Delta_{\text{ext}}$ from Reach disadvantage or Clinch crowding.
    pub reach_clinch_delta_ext: i32,
    pub strike_lands: bool,
    /// Whether the defender **Seizes the Initiative** (`attacker_net_successes <= 0` and $C_{\text{def}} > C_{\text{att}}$, `§2.2`).
    pub defender_seizes_initiative: bool,
    pub initiative_seized_penalty: i32,
    pub leveraged_throw_force: f64,
    /// Defender's Effective Root ($F_{\text{def}} + \text{MMF}_{\text{def}}$).
    pub defender_effective_root: f64,
    pub kuzushi_throw_succeeded: bool,
    pub prone_recovery_ticks: i32,
}

/// Evaluates a Contested Melee Exchange (`docs/ttrpg/05-combat/melee_combat.md` §1, §2.2, §2.3).
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

/// Checks whether a counter-attacker's strike interrupts an opponent's telegraphed attack before impact
/// (`docs/ttrpg/05-combat/melee_combat.md` §1.2).
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

