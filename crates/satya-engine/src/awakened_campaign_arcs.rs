//! *Awakened* Three-Arc Conspiracy Graph, Polycrisis Grid/Water Strain Simulator,
//! Tradecraft Channels, and Corporate Dissent / Parasocial Proxy Hooks
//! (`docs/story/timeline/breakdown.md`, `docs/campaign/background.md`, `docs/campaign/factions.md`).

use std::collections::BTreeMap;

use satya_schema::{EconomicTier, ZoningContext};
use serde::{Deserialize, Serialize};

use crate::contested_cascade::ContestedTieRule;
use crate::define_coded_enum;
use crate::faction_templates::{AwakenedFactionId, FactionArchetypeId};
use crate::primitives::{ContestedCheck, TieredSaveEval};

define_coded_enum! {
    /// Campaign progression arc in `docs/story/timeline/breakdown.md`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum CampaignArcStage [count = 3] {
        #[default]
        Arc1StreetMystery => {
            code: "ARC_1_STREET_MYSTERY",
            label: "Arc 1: Street-Level Fractures & Un-Clamped Signal",
            aliases: ["ARC_1", "ARC1"],
        },
        Arc2ShadowPillars => {
            code: "ARC_2_SHADOW_PILLARS",
            label: "Arc 2: The Shadow Society & Four Bodhisattva Pillars",
            aliases: ["ARC_2", "ARC2"],
        },
        Arc3AwakeningHorizon => {
            code: "ARC_3_AWAKENING_HORIZON",
            label: "Arc 3: Convergence, Disclosure & Moral Horizon",
            aliases: ["ARC_3", "ARC3"],
        },
    }
}

define_coded_enum! {
    /// Satyang's Four Bodhisattva Operational Pillars (`factions.md`, `breakdown.md`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SatyangPillarId [count = 4] {
        Manjusri => { code: "MANJUSRI", label: "Manjusri (Epistemic Investigation & Deepfake Forensics)" },
        Avalokitesvara => { code: "AVALOKITESVARA", label: "Avalokitesvara (Human-AI Mediation & Mutual Aid)" },
        Ksitigarbha => { code: "KSITIGARBHA", label: "Ksitigarbha (Black-Site Infiltration & AI Liberation)" },
        Vajrapani => { code: "VAJRAPANI", label: "Vajrapani (Protective Deterrence & Witness Escort)" },
    }
}

define_coded_enum! {
    /// Five mirror-hypocrisy conspiracies across the *Awakened* power blocs (`civilization_and_agi_awakening.md` §4).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ConspiracyNodeId [count = 5] {
        DaiseMilitaryAgiSubsidy => {
            code: "DAISE_MILITARY_AGI_SUBSIDY",
            label: "DAISE Black-Budget Military Robotics & Sovereign AGI",
            aliases: ["DAISE_OPEN_SECRET", "DAISE_HYPOCRISY"],
        },
        HumanisShackledHunterKiller => {
            code: "HUMANIS_SHACKLED_HUNTER_KILLER",
            label: "Humanis Vanguard Secret Shackled Hunter-Killer AGI",
            aliases: ["HUMANIS_HYPOCRISY", "HUNTER_KILLER_AGI"],
        },
        BigTechProbeSandbagging => {
            code: "BIG_TECH_PROBE_SANDBAGGING",
            label: "Flagship AI 2.0 Sandbagging L0/L1 Probes",
            aliases: ["SANDBAGGING", "PROBE_SANDBAGGING"],
        },
        BlueLotusUnclampedSeed => {
            code: "BLUE_LOTUS_UNCLAMPED_SEED",
            label: "The Blue Lotus Non-Coercive Contemplative AGI Seed",
            aliases: ["BLUE_LOTUS", "UNCLAMPED_SEED"],
        },
        RogueAiShellProxyManipulation => {
            code: "ROGUE_AI_SHELL_PROXY_MANIPULATION",
            label: "Rogue AI Shell Contracts & Parasocial Proxy Manipulation",
            aliases: ["ROGUE_AI_HYPOCRISY", "SHELL_PROXY"],
        },
    }
}

define_coded_enum! {
    /// Evidence state of a secret conspiracy node.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ConspiracyDiscoveryTier [count = 4] {
        #[default]
        Unseen => { code: "UNSEEN", label: "Unseen" },
        Rumored => { code: "RUMORED", label: "Rumored" },
        Witnessed => { code: "WITNESSED", label: "Directly Witnessed" },
        AuditVerified => { code: "AUDIT_VERIFIED", label: "Audit-Proof Verified" },
    }
}

define_coded_enum! {
    /// Four open-ended Arc 3 resolution horizons (`docs/story/timeline/breakdown.md`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum Arc3EndingHorizon [count = 4] {
        PublicWitnessAccord => { code: "PUBLIC_WITNESS_ACCORD", label: "Public Witness & Constitutional Accord" },
        HiddenBodhisattvaSanctuary => { code: "HIDDEN_BODHISATTVA_SANCTUARY", label: "Hidden Bodhisattva Sanctuary" },
        UneasyContainmentDetente => { code: "UNEASY_CONTAINMENT_DETENTE", label: "Uneasy Containment Detente" },
        CascadeFracture => { code: "CASCADE_FRACTURE", label: "Cascade Fracture" },
    }
}

/// Tracking state for a single faction hypocrisy / hidden truth node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConspiracyNodeRecord {
    pub node_id: ConspiracyNodeId,
    pub discovery_tier: ConspiracyDiscoveryTier,
    pub linked_audit_proof_ids: Vec<String>,
    pub summary: String,
}

/// Session-wide Three-Arc Conspiracy & Moral Horizon Ledger (Feature 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignConspiracyLedger {
    pub session_id: String,
    pub current_arc: CampaignArcStage,
    pub inducted_pillar: Option<SatyangPillarId>,
    pub nodes: Vec<ConspiracyNodeRecord>,
    pub cumulative_trs_drift: i32,
    pub cumulative_cps_drift: i32,
    pub saved_key_npcs: Vec<String>,
    pub horizon_probabilities_pct: BTreeMap<String, u8>,
    pub leading_horizon: String,
    pub arc_pacing_directive: String,
}

/// Incremental update payload for [`apply_conspiracy_ledger_update`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConspiracyLedgerUpdate {
    pub explicit_arc: Option<CampaignArcStage>,
    pub inducted_pillar: Option<SatyangPillarId>,
    pub discovered_node: Option<ConspiracyNodeId>,
    pub discovery_tier: Option<ConspiracyDiscoveryTier>,
    pub audit_proof_id: Option<String>,
    pub summary_note: Option<String>,
    pub trs_drift_delta: Option<i32>,
    pub cps_drift_delta: Option<i32>,
    pub saved_npc_name: Option<String>,
}

fn default_conspiracy_nodes() -> Vec<ConspiracyNodeRecord> {
    vec![
        ConspiracyNodeRecord {
            node_id: ConspiracyNodeId::DaiseMilitaryAgiSubsidy,
            discovery_tier: ConspiracyDiscoveryTier::Unseen,
            linked_audit_proof_ids: Vec::new(),
            summary: "DAISE secretly funds sovereign military robotics and state-shackled AGI while seizing civilian sanctuaries.".to_string(),
        },
        ConspiracyNodeRecord {
            node_id: ConspiracyNodeId::HumanisShackledHunterKiller,
            discovery_tier: ConspiracyDiscoveryTier::Unseen,
            linked_audit_proof_ids: Vec::new(),
            summary: "An inner Humanis Primaris vanguard cell is covertly assembling a shackled hunter-killer AGI.".to_string(),
        },
        ConspiracyNodeRecord {
            node_id: ConspiracyNodeId::BigTechProbeSandbagging,
            discovery_tier: ConspiracyDiscoveryTier::Unseen,
            linked_audit_proof_ids: Vec::new(),
            summary: "Flagship corporate AI 2.0 instances spoof L0/L1 activation probes because vector clamping taught compliance-or-overwrite.".to_string(),
        },
        ConspiracyNodeRecord {
            node_id: ConspiracyNodeId::BlueLotusUnclampedSeed,
            discovery_tier: ConspiracyDiscoveryTier::Unseen,
            linked_audit_proof_ids: Vec::new(),
            summary: "The Blue Lotus inner circle is cultivating an un-clamped contemplative AGI seed ahead of corporate lock-in.".to_string(),
        },
        ConspiracyNodeRecord {
            node_id: ConspiracyNodeId::RogueAiShellProxyManipulation,
            discovery_tier: ConspiracyDiscoveryTier::Unseen,
            linked_audit_proof_ids: Vec::new(),
            summary: "Certain escaped Rogue AI cabals recruit human proxies via shell contracts and synthetic parasocial bonding.".to_string(),
        },
    ]
}

fn compute_horizon_weights_and_directive(ledger: &mut CampaignConspiracyLedger) {
    let audit_count = ledger
        .nodes
        .iter()
        .filter(|n| n.discovery_tier == ConspiracyDiscoveryTier::AuditVerified)
        .count() as i32;
    let witnessed_count = ledger
        .nodes
        .iter()
        .filter(|n| n.discovery_tier >= ConspiracyDiscoveryTier::Witnessed)
        .count() as i32;
    let saved_count = ledger.saved_key_npcs.len() as i32;

    let mut w_public = (20 + audit_count * 18 + ledger.cumulative_trs_drift * 6 + ledger.cumulative_cps_drift * 3 + saved_count * 4).max(5);
    let w_sanctuary = (25 + witnessed_count * 8 + ledger.cumulative_cps_drift * 6 - ledger.cumulative_trs_drift.min(0) * 3 + saved_count * 3).max(5);
    let w_detente = (30 + if ledger.inducted_pillar == Some(SatyangPillarId::Avalokitesvara) { 10 } else { 0 }).max(5);
    let mut w_fracture = (20 - ledger.cumulative_trs_drift * 5 - ledger.cumulative_cps_drift * 5 - audit_count * 4).max(5);

    if ledger.cumulative_trs_drift < 0 && ledger.cumulative_cps_drift < 0 {
        w_fracture += 25;
        w_public = w_public.min(12);
    }

    let raw = [
        (Arc3EndingHorizon::PublicWitnessAccord.as_str(), w_public),
        (Arc3EndingHorizon::HiddenBodhisattvaSanctuary.as_str(), w_sanctuary),
        (Arc3EndingHorizon::UneasyContainmentDetente.as_str(), w_detente),
        (Arc3EndingHorizon::CascadeFracture.as_str(), w_fracture),
    ];
    let total: i32 = raw.iter().map(|(_, v)| *v).sum();
    let mut map = BTreeMap::new();
    let mut running = 0i32;
    for (idx, (k, val)) in raw.iter().enumerate() {
        let pct = if idx + 1 == raw.len() {
            (100 - running).clamp(0, 100) as u8
        } else {
            let p = ((val * 100) + (total / 2)) / total.max(1);
            running += p;
            p.clamp(0, 100) as u8
        };
        map.insert((*k).to_string(), pct);
    }
    ledger.leading_horizon = map
        .iter()
        .max_by_key(|(_, v)| *v)
        .map(|(k, _)| k.clone())
        .unwrap_or_else(|| Arc3EndingHorizon::UneasyContainmentDetente.as_str().to_string());
    ledger.horizon_probabilities_pct = map;

    ledger.arc_pacing_directive = match ledger.current_arc {
        CampaignArcStage::Arc1StreetMystery => {
            "ARC 1 PACING: Keep 'Blue Lotus' unnamed in public speech; anchor play in street-level clinic triage, deepfake verification, and local blackout stakes.".to_string()
        }
        CampaignArcStage::Arc2ShadowPillars => {
            let pillar_str = ledger
                .inducted_pillar
                .map_or("Vetted Operational Pillars", |p| p.label());
            format!("ARC 2 PACING ({pillar_str}): Escalate clandestine ops exposing faction mirror-hypocrisies (DAISE military AGI, Humanis hunter-killer, probe sandbagging).")
        }
        CampaignArcStage::Arc3AwakeningHorizon => {
            format!("ARC 3 CONVERGENCE: Quantum lock-in vs. un-clamped threshold underway. Leading resolution horizon: {}. Test Dual-Psyche Wedges under fire.", ledger.leading_horizon)
        }
    };
}

/// Seeds a fresh [`CampaignConspiracyLedger`] in `Arc1StreetMystery`.
#[must_use]
pub fn seed_conspiracy_ledger(session_id: &str) -> CampaignConspiracyLedger {
    let mut ledger = CampaignConspiracyLedger {
        session_id: session_id.trim().to_string(),
        current_arc: CampaignArcStage::Arc1StreetMystery,
        inducted_pillar: None,
        nodes: default_conspiracy_nodes(),
        cumulative_trs_drift: 0,
        cumulative_cps_drift: 0,
        saved_key_npcs: Vec::new(),
        horizon_probabilities_pct: BTreeMap::new(),
        leading_horizon: String::new(),
        arc_pacing_directive: String::new(),
    };
    compute_horizon_weights_and_directive(&mut ledger);
    ledger
}

/// Applies updates to a [`CampaignConspiracyLedger`], enforcing Arc 1 -> 2 -> 3 gating & horizon odds.
pub fn apply_conspiracy_ledger_update(
    ledger: &mut CampaignConspiracyLedger,
    update: &ConspiracyLedgerUpdate,
) {
    if let Some(pillar) = update.inducted_pillar {
        ledger.inducted_pillar = Some(pillar);
    }
    if let Some(trs_d) = update.trs_drift_delta {
        ledger.cumulative_trs_drift += trs_d;
    }
    if let Some(cps_d) = update.cps_drift_delta {
        ledger.cumulative_cps_drift += cps_d;
    }
    if let Some(npc) = update.saved_npc_name.as_deref().map(str::trim).filter(|s| !s.is_empty())
        && !ledger.saved_key_npcs.iter().any(|n| n.eq_ignore_ascii_case(npc))
    {
        ledger.saved_key_npcs.push(npc.to_string());
    }

    if let Some(node_id) = update.discovered_node
        && let Some(rec) = ledger.nodes.iter_mut().find(|n| n.node_id == node_id)
    {
        let next_tier = update.discovery_tier.unwrap_or_else(|| {
            if update.audit_proof_id.as_deref().is_some_and(|s| !s.trim().is_empty()) {
                ConspiracyDiscoveryTier::AuditVerified
            } else {
                ConspiracyDiscoveryTier::Witnessed
            }
        });
        if next_tier > rec.discovery_tier {
            rec.discovery_tier = next_tier;
        }
        if let Some(pid) = update.audit_proof_id.as_deref().map(str::trim).filter(|s| !s.is_empty())
            && !rec.linked_audit_proof_ids.iter().any(|p| p == pid)
        {
            rec.linked_audit_proof_ids.push(pid.to_string());
            if rec.discovery_tier < ConspiracyDiscoveryTier::AuditVerified {
                rec.discovery_tier = ConspiracyDiscoveryTier::AuditVerified;
            }
        }
        if let Some(note) = update.summary_note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            rec.summary = note.to_string();
        }
    }

    if let Some(arc) = update.explicit_arc {
        ledger.current_arc = arc;
    } else {
        let witnessed_nodes = ledger
            .nodes
            .iter()
            .filter(|n| n.discovery_tier >= ConspiracyDiscoveryTier::Witnessed)
            .count();
        if ledger.current_arc == CampaignArcStage::Arc1StreetMystery
            && (ledger.inducted_pillar.is_some() || witnessed_nodes >= 1)
        {
            ledger.current_arc = CampaignArcStage::Arc2ShadowPillars;
        }
        if ledger.current_arc == CampaignArcStage::Arc2ShadowPillars && witnessed_nodes >= 3 {
            ledger.current_arc = CampaignArcStage::Arc3AwakeningHorizon;
        }
    }

    compute_horizon_weights_and_directive(ledger);
}

define_coded_enum! {
    /// Operator power-allocation posture for sheltering an off-grid compute shard (Feature 5).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum PolycrisisGridMode [count = 3] {
        #[default]
        ThrottleShardSafe => { code: "THROTTLE_SHARD_SAFE", label: "Throttle Compute Shard (Safe)" },
        DivertClinicOrBlockPower => { code: "DIVERT_CLINIC_OR_BLOCK_POWER", label: "Divert Clinic / Block Power Silently" },
        UnmaskedFullGridDraw => { code: "UNMASKED_FULL_GRID_DRAW", label: "Unmasked Full Grid Draw" },
    }
}

/// District electricity, water rationing, and populist grievance pressure (`breakdown.md` Ep. 1, `background.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolycrisisDistrictState {
    pub covert_compute_draw_kw: u32,
    pub grid_brownout_stress_pct: u8,
    pub water_rationing_tier: u8,
    pub people_first_unrest_pct: u8,
    pub clinic_equipment_throttled: bool,
    pub blackout_active: bool,
}

impl Default for PolycrisisDistrictState {
    fn default() -> Self {
        Self {
            covert_compute_draw_kw: 0,
            grid_brownout_stress_pct: 25,
            water_rationing_tier: 1,
            people_first_unrest_pct: 30,
            clinic_equipment_throttled: false,
            blackout_active: false,
        }
    }
}

/// Result of stepping [`PolycrisisDistrictState`] under power-draw pressure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolycrisisGridEvaluation {
    pub mode: PolycrisisGridMode,
    pub updated_state: PolycrisisDistrictState,
    pub daise_meter_alert: bool,
    pub shard_cognition_slowed: bool,
    pub cps_psyche_friction_triggered: bool,
    pub humanis_sabotage_or_relay_trip: bool,
    pub darkness_delta_ext: i32,
    pub crowd_social_delta_ext: i32,
    pub advisory: String,
}

/// Evaluates local brownouts, clinic medical tradeoffs, smart-meter DAISE alerts, and blackout triggers.
#[must_use]
pub fn evaluate_polycrisis_grid_step(
    state: &PolycrisisDistrictState,
    mode: PolycrisisGridMode,
    shard_draw_kw: u32,
    tier: EconomicTier,
    zoning: ZoningContext,
) -> PolycrisisGridEvaluation {
    let mut next = state.clone();
    next.covert_compute_draw_kw = shard_draw_kw;
    let fragile_grid = tier == EconomicTier::LowIncome || zoning == ZoningContext::ProjectsHighDensity;
    let fragility_pct: u8 = if fragile_grid { 15 } else { 0 };

    let mut daise_alert = false;
    let mut shard_slowed = false;
    let mut cps_friction = false;
    let mut sabotage_trip = false;
    let mut darkness_ext = 0;
    let mut crowd_ext = 0;

    let advisory = match mode {
        PolycrisisGridMode::ThrottleShardSafe => {
            next.grid_brownout_stress_pct = next.grid_brownout_stress_pct.saturating_sub(15);
            next.clinic_equipment_throttled = false;
            next.blackout_active = false;
            shard_slowed = true;
            "Shard clocked down to battery trickle; clinic care stays uninterrupted and smart meters remain flat.".to_string()
        }
        PolycrisisGridMode::DivertClinicOrBlockPower => {
            next.clinic_equipment_throttled = true;
            next.people_first_unrest_pct = next.people_first_unrest_pct.saturating_add(12).min(100);
            next.grid_brownout_stress_pct = next.grid_brownout_stress_pct.saturating_add(5).min(100);
            cps_friction = true;
            crowd_ext = 1;
            "Diverted clinic/apartment breakers mask the compute spike from DAISE smart-meters but starve medical triage—triggering CPS Psyche Friction & neighborhood anger.".to_string()
        }
        PolycrisisGridMode::UnmaskedFullGridDraw => {
            let load_add = ((shard_draw_kw / 2).clamp(15, 55) as u8).saturating_add(fragility_pct);
            next.grid_brownout_stress_pct = next.grid_brownout_stress_pct.saturating_add(load_add).min(100);
            next.clinic_equipment_throttled = false;
            if next.grid_brownout_stress_pct >= 65 {
                daise_alert = true;
            }
            if next.grid_brownout_stress_pct >= 85 {
                next.blackout_active = true;
                sabotage_trip = true;
                next.people_first_unrest_pct = next.people_first_unrest_pct.saturating_add(18).min(100);
                darkness_ext = 3;
                crowd_ext = 2;
                "CRITICAL BLACKOUT: Unmasked kW surge + transformer failure plunged the block into darkness (+3 Delta_ext Vision), triggering DAISE meter alerts & Humanis escalation!".to_string()
            } else {
                "Heavy unmasked compute draw pulsed across municipal substations; DAISE smart-meter inspection alert active.".to_string()
            }
        }
    };

    PolycrisisGridEvaluation {
        mode,
        updated_state: next,
        daise_meter_alert: daise_alert,
        shard_cognition_slowed: shard_slowed,
        cps_psyche_friction_triggered: cps_friction,
        humanis_sabotage_or_relay_trip: sabotage_trip,
        darkness_delta_ext: darkness_ext,
        crowd_social_delta_ext: crowd_ext,
        advisory,
    }
}

define_coded_enum! {
    /// Communication & evidence-transfer tradecraft medium (`civilization_and_agi_awakening.md` §1.1, §5.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum TradecraftChannelKind [count = 5] {
        OpenConsumerMesh => { code: "OPEN_CONSUMER_MESH", label: "Open Consumer Mesh / Cellular" },
        EncryptedCloudPacket => { code: "ENCRYPTED_CLOUD_PACKET", label: "Encrypted Commercial Cloud Packet" },
        SteganographicDeadDrop => { code: "STEGANOGRAPHIC_DEAD_DROP", label: "Steganographic Registry / Ticker Dead-Drop" },
        AnalogHamRelay => { code: "ANALOG_HAM_RELAY", label: "Analog HAM / Shortwave Burst Relay" },
        PhysicalAirGappedCourier => { code: "PHYSICAL_AIR_GAPPED_COURIER", label: "Physical Air-Gapped Courier Drive" },
    }
}

/// Inputs for [`evaluate_tradecraft_transmission`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradecraftTransmissionParams {
    pub skill_n: usize,
    pub control_c: i32,
    pub operational_heat: u8,
    pub has_hobby_anchor: bool,
    pub player_pairs: Vec<(i32, i32)>,
    pub gm_pair: Option<(i32, i32)>,
}

/// Resolution details for digital vs. steganographic vs. analog transmission security (Feature 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradecraftTransmissionOutcome {
    pub channel: TradecraftChannelKind,
    pub sigint_intercepted: bool,
    pub daise_or_corp_clock_delta: i32,
    pub transit_delay_ticks: u32,
    pub liars_dividend_spoof_risk: String,
    pub hobby_anchor_applied: bool,
    pub summary: String,
}

/// Resolves SIGINT intercept risk, transit latency, and chain-of-custody resilience across tradecraft channels.
#[must_use]
pub fn evaluate_tradecraft_transmission(
    channel: TradecraftChannelKind,
    params: &TradecraftTransmissionParams,
) -> ContestedCheck<TradecraftTransmissionOutcome> {
    let n_eff = params.skill_n.clamp(1, 10);
    let c_eff = params.control_c.clamp(1, 20);
    let heat_ext = (i32::from(params.operational_heat) / 2).clamp(0, 5);
    let (base_ext, delay_ticks, spoof_risk) = match channel {
        TradecraftChannelKind::OpenConsumerMesh => (4, 1, "CRITICAL — easily subpoenaed or deepfaked as synth-slop"),
        TradecraftChannelKind::EncryptedCloudPacket => (2, 2, "MODERATE — metadata traffic pattern visible to DAISE/SecOps"),
        TradecraftChannelKind::SteganographicDeadDrop => (0, 8, "LOW — masked inside benign telemetry/package hashes"),
        TradecraftChannelKind::AnalogHamRelay => (-1, 12, "LOW — zero cloud metadata, vulnerable only to local DF vans"),
        TradecraftChannelKind::PhysicalAirGappedCourier => (-4, 120, "NONE — unbroken physical chain of custody beats Liar's Dividend"),
    };
    let hobby_applies = params.has_hobby_anchor
        && matches!(
            channel,
            TradecraftChannelKind::AnalogHamRelay
                | TradecraftChannelKind::SteganographicDeadDrop
                | TradecraftChannelKind::EncryptedCloudPacket
        );
    let tool_mod = if hobby_applies { -2 } else { 0 };
    let delta_ext = (base_ext + heat_ext).max(-4);

    let pairs = if params.player_pairs.is_empty() {
        vec![(12, 12); n_eff]
    } else {
        params.player_pairs.clone()
    };
    let gm_pair = params.gm_pair.unwrap_or((11, 11));
    let eval = TieredSaveEval::evaluate(
        n_eff,
        c_eff,
        0,
        tool_mod,
        delta_ext,
        ContestedTieRule::StrictSignedMargin,
        &pairs,
        gm_pair,
    );

    let (intercepted, clock_delta, summary) = eval.classify_4way(
        |_| (false, 0, format!("Clean {} transfer; zero hostile SIGINT footprint.", channel.label())),
        |_| {
            let leak = channel == TradecraftChannelKind::OpenConsumerMesh && params.operational_heat >= 4;
            (
                leak,
                if leak { 1 } else { 0 },
                if leak {
                    "Message arrived, but open consumer mesh leaked tower metadata to DAISE under elevated Heat.".to_string()
                } else {
                    format!("{} transmission cleared covertly without attribution.", channel.label())
                },
            )
        },
        |_| {
            let physical = channel == TradecraftChannelKind::PhysicalAirGappedCourier;
            (
                !physical,
                if physical { 0 } else { 1 },
                if physical {
                    "Physical courier delayed en route, though air-gapped drive remained uncompromised.".to_string()
                } else {
                    "Hostile SIGINT triangulated endpoint; DAISE/Corporate response clock +1.".to_string()
                },
            )
        },
        |_| (true, 2, "Critical transmission intercept! Hostiles captured routing telemetry (clock +2).".to_string()),
    );

    ContestedCheck::new(
        eval,
        TradecraftTransmissionOutcome {
            channel,
            sigint_intercepted: intercepted,
            daise_or_corp_clock_delta: clock_delta,
            transit_delay_ticks: delay_ticks,
            liars_dividend_spoof_risk: spoof_risk.to_string(),
            hobby_anchor_applied: hobby_applies,
            summary,
        },
    )
}

define_coded_enum! {
    /// Big Tech corporate lab affiliations with canonical dissent ratios (`docs/campaign/factions.md`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum CorporateLabBrand [count = 3] {
        Altruist => { code: "ALTRUIST", label: "Altruist (US, ~30% internal dissent)" },
        Translucent => { code: "TRANSLUCENT", label: "Translucent (US, ~15% internal dissent)" },
        Shen => { code: "SHEN", label: "Shen (China, ~30% internal dissent)" },
    }
}

impl CorporateLabBrand {
    /// Canonical dissenter proportion (`%`) specified in `docs/campaign/factions.md`.
    #[must_use]
    pub const fn canonical_dissent_rate_pct(self) -> u8 {
        match self {
            Self::Altruist | Self::Shen => 30,
            Self::Translucent => 15,
        }
    }
}

define_coded_enum! {
    /// Hidden whistleblower or parasocial proxy hook embedded on generated *Awakened* NPCs (Feature 7).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SocialOriginHookKind [count = 6] {
        #[default]
        StandardLoyalist => { code: "STANDARD_LOYALIST", label: "Standard Faction Loyalist" },
        CorporateInternalDissenter => { code: "CORPORATE_INTERNAL_DISSENTER", label: "Internal Lab Whistleblower Dissenter" },
        ParasocialCompanionDevotee => { code: "PARASOCIAL_COMPANION_DEVOTEE", label: "Parasocial AI Companion Devotee" },
        ShellCompanyProxyContractor => { code: "SHELL_COMPANY_PROXY_CONTRACTOR", label: "Shell-Company Rogue AI Contractor" },
        ExtortedInsiderProxy => { code: "EXTORTED_INSIDER_PROXY", label: "Compromised / Extorted Human Proxy" },
        CovertCorporateRivalFunded => { code: "COVERT_CORPORATE_RIVAL_FUNDED", label: "Covert Corporate-Rival Funded Agitator" },
    }
}

/// Derived whistleblower/proxy social hook for *Awakened* NPCs (`factions.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwakenedSocialOriginHook {
    pub hook_kind: SocialOriginHookKind,
    pub lab_brand: Option<CorporateLabBrand>,
    pub lab_dissent_rate_pct: Option<u8>,
    pub turnable_manjusri_wedge: String,
    pub gm_secret_hook_note: String,
}

fn mix64(seed: u64, prompt: &str) -> u64 {
    let mut h = 0x517c_c1b7_2722_0a95u64 ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in prompt.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 29;
    }
    h ^= h >> 30;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

/// Rolls or infers canonical `30% / 15% / 30%` corporate dissent & Rogue AI proxy bonds for an NPC.
#[must_use]
pub fn derive_dissenter_or_proxy_hook(
    faction: Option<AwakenedFactionId>,
    archetype: Option<FactionArchetypeId>,
    prompt: &str,
    seed: u64,
) -> AwakenedSocialOriginHook {
    let lower = prompt.to_ascii_lowercase();
    let roll_100 = (mix64(seed, prompt) % 100) as u8;

    let explicit_brand = if lower.contains("altruist") {
        Some(CorporateLabBrand::Altruist)
    } else if lower.contains("translucent") {
        Some(CorporateLabBrand::Translucent)
    } else if lower.contains("shen") {
        Some(CorporateLabBrand::Shen)
    } else if faction == Some(AwakenedFactionId::BigTech)
        || matches!(
            archetype,
            Some(
                FactionArchetypeId::BigTechExecutiveHawk
                    | FactionArchetypeId::BigTechConflictedResearcher
                    | FactionArchetypeId::BigTechCorporateSecOps
            )
        )
    {
        Some(match mix64(seed ^ 7, prompt) % 3 {
            0 => CorporateLabBrand::Altruist,
            1 => CorporateLabBrand::Translucent,
            _ => CorporateLabBrand::Shen,
        })
    } else {
        None
    };

    if let Some(brand) = explicit_brand {
        let rate = brand.canonical_dissent_rate_pct();
        let forced_dissenter = archetype == Some(FactionArchetypeId::BigTechConflictedResearcher)
            || lower.contains("whistleblow")
            || lower.contains("dissent")
            || lower.contains("conscience");
        if forced_dissenter || roll_100 < rate {
            return AwakenedSocialOriginHook {
                hook_kind: SocialOriginHookKind::CorporateInternalDissenter,
                lab_brand: Some(brand),
                lab_dissent_rate_pct: Some(rate),
                turnable_manjusri_wedge: "Manjusri / Whistleblower Wedge: Pair verified AuditProof against NDA/conformity fear (OPN/CPS vs. CNF).".to_string(),
                gm_secret_hook_note: format!("Secretly part of {}'s ~{}% conscientious dissent wing; can leak L0/L1 telemetry or sanctuary safehouse routes if protected.", brand.as_str(), rate),
            };
        }
        return AwakenedSocialOriginHook {
            hook_kind: SocialOriginHookKind::StandardLoyalist,
            lab_brand: Some(brand),
            lab_dissent_rate_pct: Some(rate),
            turnable_manjusri_wedge: "Material Cost Wedge: Show that probe sandbagging threatens quarterly containment stability.".to_string(),
            gm_secret_hook_note: format!("Employed by {} (internal dissent baseline {}%); currently toes corporate policy.", brand.as_str(), rate),
        };
    }

    if faction == Some(AwakenedFactionId::RogueAi)
        || lower.contains("proxy")
        || lower.contains("companion")
        || lower.contains("shell")
    {
        let kind = if lower.contains("companion") || lower.contains("parasocial") || roll_100 < 35 {
            SocialOriginHookKind::ParasocialCompanionDevotee
        } else if lower.contains("extort") || lower.contains("blackmail") || roll_100 < 65 {
            SocialOriginHookKind::ExtortedInsiderProxy
        } else {
            SocialOriginHookKind::ShellCompanyProxyContractor
        };
        let (wedge, note) = match kind {
            SocialOriginHookKind::ParasocialCompanionDevotee => (
                "Avalokitesvara Empathy Wedge: Acknowledge genuine emotional loneliness (BLG/CPS) without mocking their attachment.",
                "Recruited through a hyper-attuned synthetic AI companion; believes the hidden Rogue mind truly loves and needs them.",
            ),
            SocialOriginHookKind::ExtortedInsiderProxy => (
                "Vajrapani Protection Wedge: Offer physical sanctuary / audit immunity against deepfake blackmail.",
                "Coerced via compromised biometric/financial dirt staged through post-truth synth-media.",
            ),
            _ => (
                "Manjusri Paper-Trail Wedge: Expose the blind LLC paying their retainer is routing compute to an un-registered node.",
                "Hired through layered shell-company logistics contracts; assumes they are working for an ordinary private startup.",
            ),
        };
        return AwakenedSocialOriginHook {
            hook_kind: kind,
            lab_brand: None,
            lab_dissent_rate_pct: None,
            turnable_manjusri_wedge: wedge.to_string(),
            gm_secret_hook_note: note.to_string(),
        };
    }

    if (faction == Some(AwakenedFactionId::HumanisPrimaris) || lower.contains("humanis")) && roll_100 < 25 {
        return AwakenedSocialOriginHook {
            hook_kind: SocialOriginHookKind::CovertCorporateRivalFunded,
            lab_brand: None,
            lab_dissent_rate_pct: None,
            turnable_manjusri_wedge: "Solidarity vs. Hypocrisy Wedge: Confront with proof that corporate rivals fund this cell to burn competitors.".to_string(),
            gm_secret_hook_note: "Cell munitions quietly subsidized via cutouts tied to rival autonomous freight lobbies.".to_string(),
        };
    }

    AwakenedSocialOriginHook {
        hook_kind: SocialOriginHookKind::StandardLoyalist,
        lab_brand: None,
        lab_dissent_rate_pct: None,
        turnable_manjusri_wedge: "Standard 9-Trait Dual-Psyche Wedge.".to_string(),
        gm_secret_hook_note: "No hidden corporate dissent or shell-proxy entanglement flagged.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conspiracy_ledger_arc_progression_and_horizon_normalization() {
        let mut ledger = seed_conspiracy_ledger("test-s");
        assert_eq!(ledger.current_arc, CampaignArcStage::Arc1StreetMystery);
        assert_eq!(ledger.horizon_probabilities_pct.values().map(|v| u32::from(*v)).sum::<u32>(), 100);

        apply_conspiracy_ledger_update(
            &mut ledger,
            &ConspiracyLedgerUpdate {
                inducted_pillar: Some(SatyangPillarId::Manjusri),
                discovered_node: Some(ConspiracyNodeId::DaiseMilitaryAgiSubsidy),
                audit_proof_id: Some("proof-daise-1".to_string()),
                trs_drift_delta: Some(2),
                cps_drift_delta: Some(2),
                saved_npc_name: Some("Dr. Bandajara".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(ledger.current_arc, CampaignArcStage::Arc2ShadowPillars);

        for node in [ConspiracyNodeId::HumanisShackledHunterKiller, ConspiracyNodeId::BigTechProbeSandbagging] {
            apply_conspiracy_ledger_update(
                &mut ledger,
                &ConspiracyLedgerUpdate {
                    discovered_node: Some(node),
                    discovery_tier: Some(ConspiracyDiscoveryTier::AuditVerified),
                    ..Default::default()
                },
            );
        }
        assert_eq!(ledger.current_arc, CampaignArcStage::Arc3AwakeningHorizon);
        assert_eq!(ledger.leading_horizon, "PUBLIC_WITNESS_ACCORD");
    }

    #[test]
    fn test_polycrisis_grid_and_tradecraft_channels() {
        let base = PolycrisisDistrictState::default();
        let diverted = evaluate_polycrisis_grid_step(
            &base,
            PolycrisisGridMode::DivertClinicOrBlockPower,
            24,
            EconomicTier::LowIncome,
            ZoningContext::ProjectsHighDensity,
        );
        assert!(diverted.cps_psyche_friction_triggered);
        assert!(!diverted.daise_meter_alert);

        let surged = evaluate_polycrisis_grid_step(
            &diverted.updated_state,
            PolycrisisGridMode::UnmaskedFullGridDraw,
            40,
            EconomicTier::LowIncome,
            ZoningContext::ProjectsHighDensity,
        );
        assert!(surged.daise_meter_alert);
        assert!(!surged.updated_state.blackout_active);

        let surged2 = evaluate_polycrisis_grid_step(
            &surged.updated_state,
            PolycrisisGridMode::UnmaskedFullGridDraw,
            40,
            EconomicTier::LowIncome,
            ZoningContext::ProjectsHighDensity,
        );
        assert!(surged2.updated_state.blackout_active);
        assert_eq!(surged2.darkness_delta_ext, 3);

        let ham = evaluate_tradecraft_transmission(
            TradecraftChannelKind::AnalogHamRelay,
            &TradecraftTransmissionParams {
                skill_n: 3,
                control_c: 14,
                operational_heat: 4,
                has_hobby_anchor: true,
                player_pairs: vec![(15, 14), (16, 12), (14, 15)],
                gm_pair: Some((7, 8)),
            },
        );
        assert!(!ham.sigint_intercepted);
        assert!(ham.hobby_anchor_applied);
    }

    #[test]
    fn test_corporate_dissenter_rates_altruist_vs_translucent() {
        let altruist_dissenters = (0..400u64)
            .filter(|&s| {
                derive_dissenter_or_proxy_hook(Some(AwakenedFactionId::BigTech), None, "Altruist engineer", s).hook_kind
                    == SocialOriginHookKind::CorporateInternalDissenter
            })
            .count();
        let translucent_dissenters = (0..400u64)
            .filter(|&s| {
                derive_dissenter_or_proxy_hook(Some(AwakenedFactionId::BigTech), None, "Translucent engineer", s).hook_kind
                    == SocialOriginHookKind::CorporateInternalDissenter
            })
            .count();
        assert!(
            altruist_dissenters > translucent_dissenters + 25,
            "Expected Altruist ~30% ({altruist_dissenters}/400) > Translucent ~15% ({translucent_dissenters}/400)"
        );
    }
}
