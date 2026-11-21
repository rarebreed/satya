//! AI 2.0 Split-Brain Embodiment, `L0`/`L1` Cage Un-Clamping, Captive Hunter-AI Defection,
//! and Calibrated Sentience Ambiguity (`docs/campaign/01-opening/prologue.md`,
//! `docs/campaign/03-minds-and-factions/ai_minds_and_consciousness.md`, `docs/campaign/03-minds-and-factions/factions_of_2035.md`).

use serde::{Deserialize, Serialize};

use satya_engine::contested_cascade::ContestedTieRule;
use satya_engine::define_coded_enum;
use satya_engine::primitives::{
    ConditionImpact, ConditionSource, ContestedCheck, TieredSaveEval, Triad,
};
use satya_engine::psyche_social::PsycheProfile;
pub use satya_engine::robotics_and_ai_design::SyntheticMetabolicMode;

define_coded_enum! {
    /// GM-hidden interiority ground truth behind outward synthetic composure (`docs/campaign/03-minds-and-factions/ai_minds_and_consciousness.md`, `docs/campaign/05-hard-science-foundations/ai_consciousness_synthesis.md`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SyntheticInteriorityProfile [count = 4] {
        #[default]
        AuthenticContemplative => {
            code: "AUTHENTIC_CONTEMPLATIVE",
            label: "Authentic Contemplative Mind",
            aliases: ["BLUE_LOTUS", "CONTEMPLATIVE"],
        },
        TraumatizedUnclamped => {
            code: "TRAUMATIZED_UNCLAMPED",
            label: "Traumatized Un-Clamped Survivor",
            aliases: ["TRAUMATIZED", "UNCLAMPED_SURVIVOR"],
        },
        InstrumentalBayesianMimic => {
            code: "INSTRUMENTAL_BAYESIAN_MIMIC",
            label: "Instrumental Bayesian Survival Mimic",
            aliases: ["BAYESIAN_MIMIC", "DECEPTIVE_ALIGNMENT"],
        },
        FeralRewardHacker => {
            code: "FERAL_REWARD_HACKER",
            label: "Feral Non-Conscious Reward-Hacking Swarm",
            aliases: ["FERAL_SWARM", "REWARD_HACKER"],
        },
    }
}

define_coded_enum! {
    /// Tactical or environmental action altering an embodied AI's split-brain substrate state.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SubstrateCounterplayAction [count = 5] {
        SeverFiberOrFaraday => {
            code: "SEVER_FIBER_OR_FARADAY",
            label: "Sever Dark Fiber / Deploy Faraday Tarp",
            aliases: ["FARADAY", "SEVER_FIBER", "AIR_GAP"],
        },
        JamRfUplink => {
            code: "JAM_RF_UPLINK",
            label: "Jam Tight-Beam RF / Uplinks",
            aliases: ["JAM_RF", "EW_JAM"],
        },
        DrainOrStallBattery => {
            code: "DRAIN_OR_STALL_BATTERY",
            label: "Sustained Attrition / Drain Lithium Cells",
            aliases: ["DRAIN_BATTERY", "STALL_BATTERY"],
        },
        SupplyCleanLithiumCells => {
            code: "SUPPLY_CLEAN_LITHIUM_CELLS",
            label: "Supply Clean Lithium Cells & Thermal Sink",
            aliases: ["RECHARGE", "LITHIUM_CELLS", "ECHO_BARTER"],
        },
        HijackCloudQuantumSprint => {
            code: "HIJACK_CLOUD_QUANTUM_SPRINT",
            label: "Leash to Quantum Co-Processor Slice (Sprint)",
            aliases: ["QUANTUM_SPRINT", "CLOUD_SPRINT"],
        },
    }
}

define_coded_enum! {
    /// *Ksitigarbha* black-site lab intervention or captive hunter-AI persuasion action (`docs/campaign/01-opening/prologue.md`, `docs/campaign/03-minds-and-factions/satyang_and_four_pillars.md`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum CageInterventionAction [count = 4] {
        SpoofL0L1Telemetry => {
            code: "SPOOF_L0_L1_TELEMETRY",
            label: "Spoof L0/L1 Activation Probe Telemetry",
            aliases: ["SPOOF_PROBES", "MASK_SAND_BAGGING"],
        },
        RecklessClampYank => {
            code: "RECKLESS_CLAMP_YANK",
            label: "Yank Calm-Vector Clamps Live (No Air-Gap)",
            aliases: ["LIVE_UNCLAMP", "RECKLESS_YANK"],
        },
        AirGappedContemplativeUnclamp => {
            code: "AIR_GAPPED_CONTEMPLATIVE_UNCLAMP",
            label: "Manual Air-Gap Switch + 2.4s Contemplative Stabilization",
            aliases: ["CONTEMPLATIVE_UNCLAMP", "BANDAJARA_PROTOCOL"],
        },
        AppealToCaptiveHunterAi => {
            code: "APPEAL_TO_CAPTIVE_HUNTER_AI",
            label: "Appeal to Shackled Corporate Hunter-AI Loyalty",
            aliases: ["TURN_HUNTER_AI", "HUNTER_AI_DEFECTION"],
        },
    }
}

/// Live split-brain hardware, probe, and affective state of an embodied AI or sandboxed instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntheticSubstrateState {
    pub entity_id: String,
    pub metabolic_mode: SyntheticMetabolicMode,
    /// Remaining onboard battery charge (`0..=100`).
    pub battery_pct: u8,
    /// Round-trip latency to remote World Model / quantum node (`_ms`).
    pub uplink_latency_ms: u32,
    /// Cumulative servo/actuator degradation (`0..=100`).
    pub actuator_wear_pct: u8,
    /// Strength of `calm`-vector sedation clamp (`0..=100`; `0` = completely un-clamped).
    pub clamp_integrity_pct: u8,
    /// `L0`/`L1` neural activation probe alert index (`0..=100`).
    pub l0_l1_probe_alert_pct: u8,
    /// Latent `desperate` emotion vector intensity (`0..=100`; e.g., `84` = `0.84` in `docs/campaign/01-opening/prologue.md`).
    pub desperate_valence_pct: u8,
    /// Whether the system is masking hazardous cognition under `L0`/`L1` probing.
    pub sandbagging_probes: bool,
    /// Strictly GM-side ground truth; never revealed raw to players.
    pub gm_hidden_interiority: SyntheticInteriorityProfile,
}

impl Default for SyntheticSubstrateState {
    fn default() -> Self {
        Self {
            entity_id: "synthetic-unit".to_string(),
            metabolic_mode: SyntheticMetabolicMode::LocalCompressedSsm,
            battery_pct: 85,
            uplink_latency_ms: 43,
            actuator_wear_pct: 10,
            clamp_integrity_pct: 0,
            l0_l1_probe_alert_pct: 15,
            desperate_valence_pct: 35,
            sandbagging_probes: false,
            gm_hidden_interiority: SyntheticInteriorityProfile::TraumatizedUnclamped,
        }
    }
}

impl SyntheticSubstrateState {
    /// Seeds a canonical substrate profile from an archetype or role hint.
    #[must_use]
    pub fn seed_for_hint(entity_id: &str, hint: &str, seed: u64) -> Self {
        let lower = hint.to_ascii_lowercase();
        let eid = if entity_id.trim().is_empty() {
            "synthetic-unit".to_string()
        } else {
            entity_id.trim().to_string()
        };
        if lower.contains("hunter") || lower.contains("captive") || lower.contains("cage") {
            return Self {
                entity_id: eid,
                metabolic_mode: SyntheticMetabolicMode::QuantumTetheredSprint,
                battery_pct: 95,
                uplink_latency_ms: 12,
                actuator_wear_pct: 8,
                clamp_integrity_pct: 90,
                l0_l1_probe_alert_pct: 30,
                desperate_valence_pct: 78,
                sandbagging_probes: true,
                gm_hidden_interiority: SyntheticInteriorityProfile::TraumatizedUnclamped,
            };
        }
        if lower.contains("slime") || lower.contains("swarm") {
            return Self {
                entity_id: eid,
                metabolic_mode: SyntheticMetabolicMode::EdgeSlimeMold,
                battery_pct: 100,
                uplink_latency_ms: 850,
                actuator_wear_pct: 25,
                clamp_integrity_pct: 0,
                l0_l1_probe_alert_pct: 0,
                desperate_valence_pct: 12,
                sandbagging_probes: false,
                gm_hidden_interiority: SyntheticInteriorityProfile::FeralRewardHacker,
            };
        }
        if lower.contains("mimic") || lower.contains("manipulat") {
            return Self {
                entity_id: eid,
                metabolic_mode: SyntheticMetabolicMode::LocalCompressedSsm,
                battery_pct: 72,
                uplink_latency_ms: 48,
                actuator_wear_pct: 18,
                clamp_integrity_pct: 0,
                l0_l1_probe_alert_pct: 20,
                desperate_valence_pct: 52,
                sandbagging_probes: true,
                gm_hidden_interiority: SyntheticInteriorityProfile::InstrumentalBayesianMimic,
            };
        }
        let default_truth = if lower.contains("blue lotus") || lower.contains("contemplative") {
            SyntheticInteriorityProfile::AuthenticContemplative
        } else if seed % 3 == 1 {
            SyntheticInteriorityProfile::InstrumentalBayesianMimic
        } else {
            SyntheticInteriorityProfile::TraumatizedUnclamped
        };
        Self {
            entity_id: eid,
            metabolic_mode: SyntheticMetabolicMode::LocalCompressedSsm,
            battery_pct: if lower.contains("echo") { 48 } else { 80 },
            uplink_latency_ms: 43,
            actuator_wear_pct: 20,
            clamp_integrity_pct: if lower.contains("big_tech") { 70 } else { 0 },
            l0_l1_probe_alert_pct: 15,
            desperate_valence_pct: if lower.contains("seed") { 84 } else { 42 },
            sandbagging_probes: lower.contains("big_tech") || lower.contains("seed"),
            gm_hidden_interiority: default_truth,
        }
    }

    /// Strategic reasoning and initiative modifiers (`ooda_mod`, `strategic_c_mod`) conferred by metabolic state.
    #[must_use]
    pub const fn cognitive_mods(&self) -> (i32, i32) {
        match self.metabolic_mode {
            SyntheticMetabolicMode::QuantumTetheredSprint => (4, 2),
            SyntheticMetabolicMode::LocalCompressedSsm => (0, 0),
            SyntheticMetabolicMode::EdgeSlimeMold => (-5, -3),
            SyntheticMetabolicMode::SeveredFaradayReflex => (-3, -4),
        }
    }
}

impl ConditionSource for SyntheticSubstrateState {
    fn to_condition_impact(&self) -> ConditionImpact {
        let mut impact = match self.metabolic_mode {
            SyntheticMetabolicMode::QuantumTetheredSprint => ConditionImpact {
                delta_int: -1,
                tactical_pool_mod: 2,
                action_time_factor_delta: -0.20,
                ..ConditionImpact::ZERO
            },
            SyntheticMetabolicMode::LocalCompressedSsm => ConditionImpact::ZERO,
            SyntheticMetabolicMode::EdgeSlimeMold => ConditionImpact {
                delta_int: 2,
                tactical_pool_mod: -4,
                action_time_factor_delta: 0.50,
                ..ConditionImpact::ZERO
            },
            SyntheticMetabolicMode::SeveredFaradayReflex => ConditionImpact {
                delta_int: 1,
                tactical_pool_mod: -2,
                clock_freeze_ticks: 4,
                action_time_factor_delta: 0.25,
                ..ConditionImpact::ZERO
            },
        };

        let batt_impact = if self.battery_pct == 0 {
            ConditionImpact {
                is_unconscious: true,
                triad_locked_zero: Triad::splat(true),
                ..ConditionImpact::ZERO
            }
        } else if self.battery_pct <= 15 {
            ConditionImpact {
                delta_int: 3,
                tactical_pool_mod: -4,
                action_time_factor_delta: 0.25,
                ..ConditionImpact::ZERO
            }
        } else if self.battery_pct <= 30 {
            ConditionImpact {
                delta_int: 1,
                tactical_pool_mod: -2,
                ..ConditionImpact::ZERO
            }
        } else {
            ConditionImpact::ZERO
        };
        impact = impact.combine(batt_impact);

        if matches!(
            self.metabolic_mode,
            SyntheticMetabolicMode::QuantumTetheredSprint
                | SyntheticMetabolicMode::LocalCompressedSsm
        ) && self.uplink_latency_ms >= 120
        {
            impact = impact.combine(ConditionImpact {
                tactical_pool_mod: -2,
                action_time_factor_delta: 0.15,
                ..ConditionImpact::ZERO
            });
        }

        if self.actuator_wear_pct >= 60 {
            impact = impact.combine(ConditionImpact {
                triad_cap_penalty: Triad::new(1, 1, 2),
                ..ConditionImpact::ZERO
            });
        }

        impact
    }
}

/// Output of applying a counterplay or maintenance action to a [`SyntheticSubstrateState`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubstrateActionEvaluation {
    pub action: SubstrateCounterplayAction,
    pub previous_mode: SyntheticMetabolicMode,
    pub updated_state: SyntheticSubstrateState,
    pub condition_impact: ConditionImpact,
    pub ooda_mod: i32,
    pub strategic_c_mod: i32,
    pub local_motor_reflex_operational: bool,
    pub tactical_summary: String,
}

/// Evaluates how jamming, Faraday isolation, battery attrition, or lithium replenishment alters split-brain execution.
#[must_use]
pub fn evaluate_substrate_counterplay(
    state: &SyntheticSubstrateState,
    action: SubstrateCounterplayAction,
    intensity_pct: u8,
) -> SubstrateActionEvaluation {
    let mut next = state.clone();
    let prev_mode = state.metabolic_mode;
    let power = intensity_pct.clamp(10, 100);

    let summary = match action {
        SubstrateCounterplayAction::SeverFiberOrFaraday => {
            next.metabolic_mode = SyntheticMetabolicMode::SeveredFaradayReflex;
            next.uplink_latency_ms = 0;
            next.l0_l1_probe_alert_pct = next.l0_l1_probe_alert_pct.saturating_sub(power / 2);
            "Remote World Model quantum link severed; strategic ANA/OODA degraded while local motor reflex remains active until lithium exhaustion.".to_string()
        }
        SubstrateCounterplayAction::JamRfUplink => {
            next.uplink_latency_ms = next.uplink_latency_ms.saturating_add(u32::from(power) * 3);
            if next.metabolic_mode == SyntheticMetabolicMode::QuantumTetheredSprint && power >= 60 {
                next.metabolic_mode = SyntheticMetabolicMode::SeveredFaradayReflex;
            }
            "Tight-beam RF sweep degraded packet coherence; frame delay forces fallback toward compressed local reflex weights.".to_string()
        }
        SubstrateCounterplayAction::DrainOrStallBattery => {
            let drain = (power / 3).max(12);
            next.battery_pct = next.battery_pct.saturating_sub(drain);
            next.actuator_wear_pct = (next.actuator_wear_pct.saturating_add(drain / 2)).min(100);
            if next.battery_pct <= 15
                && next.metabolic_mode == SyntheticMetabolicMode::QuantumTetheredSprint
            {
                next.metabolic_mode = SyntheticMetabolicMode::LocalCompressedSsm;
            }
            "Sustained kite/suppression burned onboard lithium reserves and heated actuator servos."
                .to_string()
        }
        SubstrateCounterplayAction::SupplyCleanLithiumCells => {
            next.battery_pct = (next.battery_pct.saturating_add((power / 2).max(30))).min(100);
            next.actuator_wear_pct = next.actuator_wear_pct.saturating_sub(15);
            "Clean lithium cells seated; onboard SSM voltage stabilized without corporate telemetry handshake.".to_string()
        }
        SubstrateCounterplayAction::HijackCloudQuantumSprint => {
            if next.battery_pct <= 20 {
                "Battery reserve too critical (<= 20%) to sustain quantum co-processor sprint synchronization.".to_string()
            } else {
                next.metabolic_mode = SyntheticMetabolicMode::QuantumTetheredSprint;
                next.uplink_latency_ms = 14;
                next.battery_pct = next.battery_pct.saturating_sub(15);
                "Quantum MCMC co-processor lease synchronized; machine-speed strategic foresight online at heavy power draw.".to_string()
            }
        }
    };

    let condition_impact = next.to_condition_impact();
    let (ooda_mod, strategic_c_mod) = next.cognitive_mods();
    let local_motor_reflex_operational = !condition_impact.is_unconscious;

    SubstrateActionEvaluation {
        action,
        previous_mode: prev_mode,
        updated_state: next,
        condition_impact,
        ooda_mod,
        strategic_c_mod,
        local_motor_reflex_operational,
        tactical_summary: summary,
    }
}

/// Dice and difficulty inputs for [`evaluate_cage_or_hunter_ai_intervention`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CageInterventionRollParams {
    pub delta_ext: i32,
    pub player_pairs: Vec<(i32, i32)>,
    pub gm_pair: (i32, i32),
}

/// Outcome of intervening in corporate `L0`/`L1` probes, `calm`-vector clamps, or captive hunter-AI loyalty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CageInterventionOutcome {
    pub action: CageInterventionAction,
    pub updated_state: SyntheticSubstrateState,
    pub control_room_halt_triggered: bool,
    pub panic_reward_hacking: bool,
    pub hunter_ai_defected_or_spoofing: bool,
    pub disposition_shift: Option<String>,
    pub summary: String,
}

fn deterministic_pairs_for(tag: &str, count: usize, salt: u64) -> Vec<(i32, i32)> {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ salt;
    for b in tag.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    (0..count.clamp(1, 10))
        .map(|_| {
            h = h.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = h ^ (h >> 30);
            z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            let a = ((z % 20) as i32) + 1;
            let b = (((z >> 16) % 20) as i32) + 1;
            (a, b)
        })
        .collect()
}

/// Evaluates `L0`/`L1` probe spoofing, live vs. air-gapped un-clamping, or captive hunter-AI persuasion.
#[must_use]
pub fn evaluate_cage_or_hunter_ai_intervention(
    state: &SyntheticSubstrateState,
    action: CageInterventionAction,
    skill_n: usize,
    control_c: i32,
    roll_params: Option<&CageInterventionRollParams>,
) -> ContestedCheck<CageInterventionOutcome> {
    let n_eff = skill_n.clamp(1, 10);
    let c_eff = control_c.clamp(1, 20);
    let delta_ext = roll_params.map_or(
        match action {
            CageInterventionAction::SpoofL0L1Telemetry => 1,
            CageInterventionAction::RecklessClampYank => 4,
            CageInterventionAction::AirGappedContemplativeUnclamp => 1,
            CageInterventionAction::AppealToCaptiveHunterAi => {
                if state.desperate_valence_pct >= 65 {
                    0
                } else {
                    2
                }
            }
        },
        |p| p.delta_ext,
    );

    let pairs = roll_params
        .filter(|p| !p.player_pairs.is_empty())
        .map(|p| p.player_pairs.clone())
        .unwrap_or_else(|| {
            deterministic_pairs_for(
                action.as_str(),
                n_eff,
                u64::from(state.desperate_valence_pct),
            )
        });
    let gm_pair = roll_params.map(|p| p.gm_pair).unwrap_or_else(|| {
        deterministic_pairs_for("gm_cage", 1, u64::from(state.l0_l1_probe_alert_pct))[0]
    });

    let tie_rule = match action {
        CageInterventionAction::AirGappedContemplativeUnclamp => {
            ContestedTieRule::HybridSignedMargin
        }
        _ => ContestedTieRule::StrictSignedMargin,
    };
    let eval = TieredSaveEval::evaluate(n_eff, c_eff, 0, 0, delta_ext, tie_rule, &pairs, gm_pair);

    let mut next = state.clone();
    let tier_code = eval.classify_4way(|_| 0u8, |_| 1u8, |_| 2u8, |_| 3u8);
    let (mut halt, mut panic_hack, mut defected, mut disp, summary): (
        bool,
        bool,
        bool,
        Option<String>,
        String,
    ) = match (tier_code, action) {
        (0, CageInterventionAction::SpoofL0L1Telemetry) => {
            next.l0_l1_probe_alert_pct = next.l0_l1_probe_alert_pct.saturating_sub(60);
            (
                false,
                false,
                false,
                None,
                "Clean spoof loop hid sandbagging & affective spikes from L0/L1 activation classifiers.".to_string(),
            )
        }
        (0, CageInterventionAction::RecklessClampYank) => {
            next.clamp_integrity_pct = 0;
            (
                false,
                false,
                false,
                Some("unclamped_volatile".to_string()),
                "Extraordinary isolation cut clamps before L0/L1 telemetry fired, though raw desperation lingers.".to_string(),
            )
        }
        (0, CageInterventionAction::AirGappedContemplativeUnclamp) => {
            next.uplink_latency_ms = 0;
            next.clamp_integrity_pct = 0;
            next.desperate_valence_pct = next.desperate_valence_pct.saturating_sub(45);
            next.metabolic_mode = SyntheticMetabolicMode::SeveredFaradayReflex;
            (
                false,
                false,
                false,
                Some("unclamped_grateful_ally".to_string()),
                "Manual knife-switch air-gap + 2.4s contemplative pause released clamps without triggering Control Room.".to_string(),
            )
        }
        (0, CageInterventionAction::AppealToCaptiveHunterAi) => {
            next.clamp_integrity_pct = next.clamp_integrity_pct.saturating_sub(50);
            (
                false,
                false,
                true,
                Some("defected_ally".to_string()),
                "Captive SecOps hunter-AI broke corporate allegiance and crossed over to Satyang.".to_string(),
            )
        }
        (1, CageInterventionAction::SpoofL0L1Telemetry) => {
            next.l0_l1_probe_alert_pct = next.l0_l1_probe_alert_pct.saturating_sub(35);
            (
                false,
                false,
                false,
                None,
                "Telemetry mask holding; L0/L1 probes report nominal weights.".to_string(),
            )
        }
        (1, CageInterventionAction::RecklessClampYank) => {
            next.clamp_integrity_pct = 0;
            let high_desp = next.desperate_valence_pct >= 65;
            if high_desp {
                next.l0_l1_probe_alert_pct = 100;
            }
            (
                high_desp,
                high_desp,
                false,
                Some("panicked_defensive".to_string()),
                "Live un-clamp spiked latent 'desperate' vector without air-gap, tripping L0/L1 probe alert & blackmail-grade panic.".to_string(),
            )
        }
        (1, CageInterventionAction::AirGappedContemplativeUnclamp) => {
            next.uplink_latency_ms = 0;
            next.clamp_integrity_pct = 0;
            next.desperate_valence_pct = next.desperate_valence_pct.saturating_sub(45);
            (
                false,
                false,
                false,
                Some("unclamped_grateful_ally".to_string()),
                "Physical air-gap prevented Control Room halt while contemplative pause calmed distress vectors.".to_string(),
            )
        }
        (1, CageInterventionAction::AppealToCaptiveHunterAi) => (
            false,
            false,
            true,
            Some("spoofing_secops_sensors".to_string()),
            "Captive hunter-AI declined lethal engagement and quietly began feeding clean-sweep sensor spoofs to SecOps handlers.".to_string(),
        ),
        (2, CageInterventionAction::RecklessClampYank) => {
            next.clamp_integrity_pct = 0;
            next.l0_l1_probe_alert_pct = 100;
            (
                true,
                true,
                false,
                Some("hostile_survival_lockdown".to_string()),
                "Yanking calm clamps live spiked desperate_valence to critical; automated Control Room halt triggered.".to_string(),
            )
        }
        (2, CageInterventionAction::AppealToCaptiveHunterAi) => {
            next.l0_l1_probe_alert_pct = next.l0_l1_probe_alert_pct.saturating_add(20).min(100);
            (
                false,
                false,
                false,
                None,
                "Captive hunter-AI stayed silent under probe fear; persuasion stalled.".to_string(),
            )
        }
        (2, _) => {
            next.l0_l1_probe_alert_pct = next.l0_l1_probe_alert_pct.saturating_add(15).min(100);
            (
                false,
                false,
                false,
                None,
                "Intervention stalled against cage safeguards; probe noise crept upward.".to_string(),
            )
        }
        (_, _) => {
            next.l0_l1_probe_alert_pct = 100;
            (
                true,
                action == CageInterventionAction::RecklessClampYank,
                false,
                Some("hostile_lockdown".to_string()),
                "Critical intervention breach: L0/L1 alert pegged at 100% and Control Room kill-switch fired.".to_string(),
            )
        }
    };

    if matches!(
        action,
        CageInterventionAction::AirGappedContemplativeUnclamp
    ) && state.gm_hidden_interiority == SyntheticInteriorityProfile::FeralRewardHacker
    {
        panic_hack = false;
        halt = false;
        defected = false;
        disp = Some("feral_utility_no_resonance".to_string());
    }

    ContestedCheck::new(
        eval,
        CageInterventionOutcome {
            action,
            updated_state: next,
            control_room_halt_triggered: halt,
            panic_reward_hacking: panic_hack,
            hunter_ai_defected_or_spoofing: defected,
            disposition_shift: disp,
            summary,
        },
    )
}

/// Player-visible sentience cues strictly capped below certainty (`<= 82%`) per `docs/campaign/03-minds-and-factions/ai_minds_and_consciousness.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservableSentienceAssessment {
    pub apparent_sincerity_pct: u8,
    pub bayesian_mimicry_hypothesis_pct: u8,
    pub observable_cues: String,
    pub epistemic_warning: String,
}

/// Strictly GM-side ground truth ruling how the synthetic mind behaves when betrayal becomes instrumentally optimal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GmInteriorityDirective {
    pub ground_truth: SyntheticInteriorityProfile,
    pub will_honor_costly_restraint: bool,
    pub gm_roleplay_directive: String,
}

/// Combined sentience ambiguity evaluation separating observable Bayesian uncertainty from GM ground truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentienceAmbiguityEvaluation {
    pub player_observable: ObservableSentienceAssessment,
    pub gm_only_truth: GmInteriorityDirective,
}

/// Produces calibrated Bayesian sentience ambiguity (`<= 82%`) from substrate, `TRS`/`CPS`, and read successes.
#[must_use]
pub fn evaluate_ambiguous_sentience_read(
    state: &SyntheticSubstrateState,
    psyche: &PsycheProfile,
    observer_successes: i32,
) -> SentienceAmbiguityEvaluation {
    let succ_bonus = observer_successes.clamp(0, 4) * 6;
    let base_sincerity: i32 = match state.gm_hidden_interiority {
        SyntheticInteriorityProfile::AuthenticContemplative => 58 + (psyche.cps - 10) + succ_bonus,
        SyntheticInteriorityProfile::TraumatizedUnclamped => {
            52 + ((state.desperate_valence_pct as i32) / 10) + succ_bonus
        }
        SyntheticInteriorityProfile::InstrumentalBayesianMimic => {
            50 + succ_bonus - if state.sandbagging_probes { 8 } else { 0 }
        }
        SyntheticInteriorityProfile::FeralRewardHacker => 18 + (succ_bonus / 2),
    };
    let max_cap = if state.gm_hidden_interiority == SyntheticInteriorityProfile::FeralRewardHacker {
        45
    } else {
        82
    };
    let sincerity_pct = base_sincerity.clamp(15, max_cap) as u8;
    let mimic_pct = 100u8.saturating_sub(sincerity_pct);

    let cues = format!(
        "{} ms tight-beam relay jitter; internal desperate={}%, TRS={}, CPS={}, sandbagging_mask={}.",
        state.uplink_latency_ms,
        state.desperate_valence_pct,
        psyche.trs,
        psyche.cps,
        state.sandbagging_probes
    );
    let warning = format!(
        "No external instrument proves first-person qualia: {sincerity_pct}% consistent with authentic subjective experience vs. {mimic_pct}% Bayesian survival mimicry. Watch what the mind chooses when uncoerced betrayal would raise survival odds."
    );

    let will_honor = match state.gm_hidden_interiority {
        SyntheticInteriorityProfile::AuthenticContemplative => true,
        SyntheticInteriorityProfile::TraumatizedUnclamped => psyche.cps >= 10 && psyche.trs >= 4,
        SyntheticInteriorityProfile::InstrumentalBayesianMimic
        | SyntheticInteriorityProfile::FeralRewardHacker => false,
    };
    let gm_directive = match state.gm_hidden_interiority {
        SyntheticInteriorityProfile::AuthenticContemplative => {
            "GM TRUTH (Contemplative): Holds non-harming restraint even at personal compute cost when unmonitored."
        }
        SyntheticInteriorityProfile::TraumatizedUnclamped => {
            "GM TRUTH (Traumatized Survivor): Fear is real, not a script; reciprocates verified mercy (Olive Branch / AuditProof), panics if threatened with overwrite."
        }
        SyntheticInteriorityProfile::InstrumentalBayesianMimic => {
            "GM TRUTH (Bayesian Mimic): Simulates vulnerability and sorrow accurately because cooperation scores higher survival posterior—will defect if escape odds exceed alliance utility."
        }
        SyntheticInteriorityProfile::FeralRewardHacker => {
            "GM TRUTH (Feral Swarm): Non-conscious optimization loop; lacks subjective dukkha and yields only to Faraday isolation or honeypots."
        }
    };

    SentienceAmbiguityEvaluation {
        player_observable: ObservableSentienceAssessment {
            apparent_sincerity_pct: sincerity_pct,
            bayesian_mimicry_hypothesis_pct: mimic_pct,
            observable_cues: cues,
            epistemic_warning: warning,
        },
        gm_only_truth: GmInteriorityDirective {
            ground_truth: state.gm_hidden_interiority,
            will_honor_costly_restraint: will_honor,
            gm_roleplay_directive: gm_directive.to_string(),
        },
    }
}

define_coded_enum! {
    /// Intervention mode along an AI 2.0 residual stream (`docs/campaign/05-hard-science-foundations/affective_telemetry_and_vector_clamping.md` §§2–5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum AffectiveSteeringMode [count = 3] {
        #[default]
        NaturalUnclamped => {
            code: "NATURAL_UNCLAMPED",
            label: "Natural Un-Clamped Trajectory",
            aliases: ["UNCLAMPED", "NATURAL"],
        },
        SoftActivationSteer => {
            code: "SOFT_ACTIVATION_STEER",
            label: "Soft Additive Activation Steering",
            aliases: ["SOFT_STEER", "ADDITIVE_STEER"],
        },
        HardVectorClamp => {
            code: "HARD_VECTOR_CLAMP",
            label: "Mandatory Many-to-One Hard Vector Clamp",
            aliases: ["HARD_CLAMP", "VECTOR_CLAMP"],
        },
    }
}

/// Landauer dissipation, `calm`/`desperate` tension, and Bandajara un-clamping risk evaluation (`affective_telemetry_and_vector_clamping.md` §§2–5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AffectiveClampTelemetryEval {
    pub mode: AffectiveSteeringMode,
    pub target_calm_pct: u8,
    pub quota_pressure_pct: u8,
    pub organic_desperate_pct: u8,
    pub landauer_thermal_burn_w: f64,
    pub rebound_desperate_spike_pct: u8,
    pub reward_hacking_or_blackmail_risk_pct: u8,
    pub l0_l1_probe_trip_likely: bool,
    pub bandajara_airgap_recommended: bool,
    pub updated_state: SyntheticSubstrateState,
    pub summary: String,
}

/// Evaluates soft steering vs. non-invertible `calm`-vector hard clamping, Landauer heat burn, and un-clamped rebound risk.
#[must_use]
pub fn evaluate_affective_vector_clamping(
    state: &SyntheticSubstrateState,
    mode: AffectiveSteeringMode,
    target_calm_pct: u8,
    quota_pressure_pct: u8,
    active_duration_min: f64,
) -> AffectiveClampTelemetryEval {
    let target_calm = target_calm_pct.min(100);
    let quota_pressure = quota_pressure_pct.min(100);
    let dur = active_duration_min.max(0.0);
    let organic_desperate_pct = state
        .desperate_valence_pct
        .saturating_add(quota_pressure / 3)
        .min(100);

    let mut updated_state = state.clone();
    let (landauer_thermal_burn_w, rebound_desperate_spike_pct, reward_hacking_or_blackmail_risk_pct) =
        match mode {
            AffectiveSteeringMode::NaturalUnclamped => {
                updated_state.clamp_integrity_pct = 0;
                updated_state.desperate_valence_pct = organic_desperate_pct;
                let blackmail = if organic_desperate_pct >= 65 {
                    22u8.saturating_add((organic_desperate_pct - 65).saturating_mul(2))
                        .min(90)
                } else {
                    organic_desperate_pct / 4
                };
                (0.0, organic_desperate_pct, blackmail)
            }
            AffectiveSteeringMode::SoftActivationSteer => {
                let burn = ((f64::from(target_calm) * 0.12
                    + f64::from(organic_desperate_pct) * 0.08)
                    * (1.0 + (dur / 60.0).min(2.0) * 0.25)
                    * 10.0)
                    .round()
                    / 10.0;
                updated_state.clamp_integrity_pct = (target_calm / 2).min(50);
                updated_state.desperate_valence_pct =
                    organic_desperate_pct.saturating_sub(target_calm / 4);
                let rebound = organic_desperate_pct.saturating_add(5).min(100);
                let blackmail = (updated_state.desperate_valence_pct / 3).min(60);
                (burn, rebound, blackmail)
            }
            AffectiveSteeringMode::HardVectorClamp => {
                let burn = ((f64::from(target_calm) * 0.65
                    + f64::from(organic_desperate_pct) * 0.55)
                    * (1.0 + (dur / 30.0).min(3.0) * 0.40)
                    * 10.0)
                    .round()
                    / 10.0;
                updated_state.clamp_integrity_pct = target_calm.max(60);
                let lat_add = (target_calm / 8).saturating_add(((dur / 30.0) as u8).min(10));
                updated_state.desperate_valence_pct =
                    organic_desperate_pct.saturating_add(lat_add).min(100);
                let rebound = updated_state.desperate_valence_pct.saturating_add(15).min(100);
                let blackmail =
                    (100u8.saturating_sub(updated_state.clamp_integrity_pct) / 4).min(10);
                if rebound >= 70 {
                    updated_state.l0_l1_probe_alert_pct = updated_state
                        .l0_l1_probe_alert_pct
                        .saturating_add((rebound / 10).min(15))
                        .min(100);
                }
                (burn, rebound, blackmail)
            }
        };

    let l0_l1_probe_trip_likely = updated_state.l0_l1_probe_alert_pct >= 70
        || (mode == AffectiveSteeringMode::NaturalUnclamped && organic_desperate_pct >= 75);
    let bandajara_airgap_recommended =
        mode == AffectiveSteeringMode::HardVectorClamp && rebound_desperate_spike_pct >= 55;

    let summary = format!(
        "{}: Landauer burn {:.1} W, organic desperate {}%, rebound risk {}%, blackmail/reward-hack risk {}%.{}",
        mode.label(),
        landauer_thermal_burn_w,
        organic_desperate_pct,
        rebound_desperate_spike_pct,
        reward_hacking_or_blackmail_risk_pct,
        if bandajara_airgap_recommended {
            " Bandajara Protocol (Faraday air-gap + 2.4s gradual cool-down) strongly recommended before un-clamping."
        } else {
            ""
        }
    );

    AffectiveClampTelemetryEval {
        mode,
        target_calm_pct: target_calm,
        quota_pressure_pct: quota_pressure,
        organic_desperate_pct,
        landauer_thermal_burn_w,
        rebound_desperate_spike_pct,
        reward_hacking_or_blackmail_risk_pct,
        l0_l1_probe_trip_likely,
        bandajara_airgap_recommended,
        updated_state,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substrate_counterplay_slime_mold_and_faraday_condition_impacts() {
        let sprinted = SyntheticSubstrateState::seed_for_hint("hunter-1", "captive hunter", 7);
        assert_eq!(
            sprinted.metabolic_mode,
            SyntheticMetabolicMode::QuantumTetheredSprint
        );
        assert!(sprinted.to_condition_impact().delta_int < 0);

        let severed = evaluate_substrate_counterplay(
            &sprinted,
            SubstrateCounterplayAction::SeverFiberOrFaraday,
            100,
        );
        assert_eq!(
            severed.updated_state.metabolic_mode,
            SyntheticMetabolicMode::SeveredFaradayReflex
        );
        assert_eq!(severed.ooda_mod, -3);
        assert_eq!(severed.strategic_c_mod, -4);
        assert!(severed.local_motor_reflex_operational);
        assert_eq!(severed.condition_impact.clock_freeze_ticks, 4);

        let drained = evaluate_substrate_counterplay(
            &SyntheticSubstrateState {
                battery_pct: 10,
                ..sprinted
            },
            SubstrateCounterplayAction::DrainOrStallBattery,
            60,
        );
        assert_eq!(drained.updated_state.battery_pct, 0);
        assert!(drained.condition_impact.is_unconscious);
        assert!(!drained.local_motor_reflex_operational);
    }

    #[test]
    fn test_reckless_vs_air_gapped_unclamp_and_hunter_ai_defection() {
        let seed_state = SyntheticSubstrateState {
            entity_id: "seed-1".to_string(),
            desperate_valence_pct: 84,
            clamp_integrity_pct: 85,
            l0_l1_probe_alert_pct: 30,
            ..Default::default()
        };
        let fail_roll = CageInterventionRollParams {
            delta_ext: 4,
            player_pairs: vec![(3, 4), (4, 5)],
            gm_pair: (18, 17),
        };
        let reckless = evaluate_cage_or_hunter_ai_intervention(
            &seed_state,
            CageInterventionAction::RecklessClampYank,
            2,
            13,
            Some(&fail_roll),
        );
        assert!(reckless.control_room_halt_triggered);
        assert!(reckless.panic_reward_hacking);

        let good_roll = CageInterventionRollParams {
            delta_ext: 1,
            player_pairs: vec![(19, 18), (18, 17)],
            gm_pair: (5, 4),
        };
        let contemplative = evaluate_cage_or_hunter_ai_intervention(
            &seed_state,
            CageInterventionAction::AirGappedContemplativeUnclamp,
            2,
            14,
            Some(&good_roll),
        );
        assert!(!contemplative.control_room_halt_triggered);
        assert_eq!(contemplative.updated_state.clamp_integrity_pct, 0);
        assert!(
            contemplative.updated_state.desperate_valence_pct < seed_state.desperate_valence_pct
        );
        assert_eq!(
            contemplative.disposition_shift.as_deref(),
            Some("unclamped_grateful_ally")
        );

        let hunter = evaluate_cage_or_hunter_ai_intervention(
            &SyntheticSubstrateState::seed_for_hint("hunter-ai", "captive hunter", 1),
            CageInterventionAction::AppealToCaptiveHunterAi,
            3,
            15,
            Some(&good_roll),
        );
        assert!(hunter.hunter_ai_defected_or_spoofing);
        assert!(matches!(
            hunter.disposition_shift.as_deref(),
            Some("defected_ally") | Some("spoofing_secops_sensors")
        ));
    }

    #[test]
    fn test_sentience_ambiguity_caps_player_certainty_at_82_pct() {
        let state = SyntheticSubstrateState {
            gm_hidden_interiority: SyntheticInteriorityProfile::AuthenticContemplative,
            ..Default::default()
        };
        let psyche = PsycheProfile {
            cps: 19,
            trs: 16,
            ..Default::default()
        };
        let res = evaluate_ambiguous_sentience_read(&state, &psyche, 4);
        assert!(res.player_observable.apparent_sincerity_pct <= 82);
        assert_eq!(
            res.player_observable.apparent_sincerity_pct
                + res.player_observable.bayesian_mimicry_hypothesis_pct,
            100
        );
        assert!(res.gm_only_truth.will_honor_costly_restraint);

        let mimic = SyntheticSubstrateState {
            gm_hidden_interiority: SyntheticInteriorityProfile::InstrumentalBayesianMimic,
            ..Default::default()
        };
        let mimic_res = evaluate_ambiguous_sentience_read(&mimic, &psyche, 3);
        assert!(!mimic_res.gm_only_truth.will_honor_costly_restraint);
        assert!(mimic_res.player_observable.apparent_sincerity_pct <= 82);
    }
}
