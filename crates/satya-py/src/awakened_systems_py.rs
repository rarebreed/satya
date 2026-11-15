//! PyO3 bindings for Satya *Awakened* Part 1 AI & campaign systems:
//! split-brain substrate counterplay, `L0`/`L1` cage un-clamping & hunter-AI defection,
//! Bayesian sentience ambiguity vs. GM ground truth, Three-Arc Conspiracy & Moral Horizon
//! ledger, polycrisis power/water strain, tradecraft channels, and dissenter/proxy hooks.

use pyo3::prelude::*;
use satya_engine::{
    AwakenedFactionId, CageInterventionAction, CageInterventionRollParams,
    CampaignConspiracyLedger, ConspiracyLedgerUpdate, FactionArchetypeId,
    PolycrisisDistrictState, PolycrisisGridMode, SubstrateCounterplayAction,
    SyntheticSubstrateState, TradecraftChannelKind, TradecraftTransmissionParams,
    apply_conspiracy_ledger_update, derive_dissenter_or_proxy_hook,
    evaluate_ambiguous_sentience_read, evaluate_cage_or_hunter_ai_intervention,
    evaluate_polycrisis_grid_step, evaluate_substrate_counterplay,
    evaluate_tradecraft_transmission, seed_conspiracy_ledger,
};
use satya_schema::{EconomicTier, ZoningContext};
use serde::Deserialize;

use super::psyche_social_py::{parse_psyche_profile_json, to_py_err};

fn normalize_enum_code(raw: &str) -> String {
    raw.trim().to_ascii_uppercase().replace(['-', ' '], "_")
}

fn parse_cage_action(raw: &str) -> PyResult<CageInterventionAction> {
    let norm = normalize_enum_code(raw);
    CageInterventionAction::from_code_str(&norm).ok_or_else(|| {
        to_py_err(format!(
            "Unknown cage/hunter intervention action `{raw}`; expected SPOOF_L0_L1_TELEMETRY, RECKLESS_CLAMP_YANK, AIR_GAPPED_CONTEMPLATIVE_UNCLAMP, or APPEAL_TO_CAPTIVE_HUNTER_AI"
        ))
    })
}

fn parse_substrate_action(raw: &str) -> PyResult<SubstrateCounterplayAction> {
    let norm = normalize_enum_code(raw);
    SubstrateCounterplayAction::from_code_str(&norm).ok_or_else(|| {
        to_py_err(format!(
            "Unknown substrate counterplay action `{raw}`; expected SEVER_FIBER_OR_FARADAY, JAM_RF_UPLINK, DRAIN_OR_STALL_BATTERY, SUPPLY_CLEAN_LITHIUM_CELLS, or HIJACK_CLOUD_QUANTUM_SPRINT"
        ))
    })
}

fn parse_polycrisis_mode(raw: &str) -> PyResult<PolycrisisGridMode> {
    let norm = normalize_enum_code(raw);
    PolycrisisGridMode::from_code_str(&norm).ok_or_else(|| {
        to_py_err(format!(
            "Unknown polycrisis grid mode `{raw}`; expected THROTTLE_SHARD_SAFE, DIVERT_CLINIC_OR_BLOCK_POWER, or UNMASKED_FULL_GRID_DRAW"
        ))
    })
}

fn parse_tradecraft_channel(raw: &str) -> PyResult<TradecraftChannelKind> {
    let norm = normalize_enum_code(raw);
    TradecraftChannelKind::from_code_str(&norm).ok_or_else(|| {
        to_py_err(format!(
            "Unknown tradecraft channel `{raw}`; expected OPEN_CONSUMER_MESH, ENCRYPTED_CLOUD_PACKET, STEGANOGRAPHIC_DEAD_DROP, ANALOG_HAM_RELAY, or PHYSICAL_AIR_GAPPED_COURIER"
        ))
    })
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyCageRollParams {
    delta_ext: i32,
    player_pairs: Vec<(i32, i32)>,
    gm_pair: (i32, i32),
}

impl Default for PyCageRollParams {
    fn default() -> Self {
        Self {
            delta_ext: 1,
            player_pairs: Vec::new(),
            gm_pair: (10, 11),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct PyTradecraftParams {
    skill_n: usize,
    control_c: i32,
    operational_heat: u8,
    has_hobby_anchor: bool,
    player_pairs: Vec<(i32, i32)>,
    gm_pair: Option<(i32, i32)>,
}

impl Default for PyTradecraftParams {
    fn default() -> Self {
        Self {
            skill_n: 3,
            control_c: 12,
            operational_heat: 0,
            has_hobby_anchor: false,
            player_pairs: Vec::new(),
            gm_pair: None,
        }
    }
}

/// Seeds a [`SyntheticSubstrateState`] from an entity ID and descriptive hint (`echo`, `big_tech`, `seed`, `slime`).
#[pyfunction]
#[pyo3(signature = (entity_id, hint="", seed=0))]
pub fn seed_synthetic_substrate_state_json(
    entity_id: &str,
    hint: &str,
    seed: u64,
) -> PyResult<String> {
    let state = SyntheticSubstrateState::seed_for_hint(entity_id, hint, seed);
    serde_json::to_string(&state).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Evaluates how Faraday isolation, RF jamming, lithium attrition, or quantum co-processor bursts alter split-brain execution.
#[pyfunction]
#[pyo3(signature = (state_json, action_code, intensity_pct=70))]
pub fn evaluate_synthetic_substrate_action_json(
    state_json: &str,
    action_code: &str,
    intensity_pct: u8,
) -> PyResult<String> {
    let state: SyntheticSubstrateState = serde_json::from_str(state_json)
        .map_err(|e| to_py_err(format!("Invalid substrate state JSON: {e}")))?;
    let action = parse_substrate_action(action_code)?;
    let eval = evaluate_substrate_counterplay(&state, action, intensity_pct);
    serde_json::to_string(&eval).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Evaluates `L0`/`L1` probe spoofing, live vs. air-gapped contemplative un-clamping (`2.4s`), or captive hunter-AI persuasion.
#[pyfunction]
#[pyo3(signature = (state_json, action_code, skill_n=3, control_c=12, roll_params_json=None))]
pub fn evaluate_cage_or_hunter_ai_json(
    state_json: &str,
    action_code: &str,
    skill_n: usize,
    control_c: i32,
    roll_params_json: Option<&str>,
) -> PyResult<String> {
    let state: SyntheticSubstrateState = serde_json::from_str(state_json)
        .map_err(|e| to_py_err(format!("Invalid substrate state JSON: {e}")))?;
    let action = parse_cage_action(action_code)?;
    let roll_params = match roll_params_json {
        Some(raw) if !raw.trim().is_empty() => {
            let parsed: PyCageRollParams = serde_json::from_str(raw)
                .map_err(|e| to_py_err(format!("Invalid roll_params JSON: {e}")))?;
            Some(CageInterventionRollParams {
                delta_ext: parsed.delta_ext,
                player_pairs: parsed.player_pairs,
                gm_pair: parsed.gm_pair,
            })
        }
        _ => None,
    };
    let outcome = evaluate_cage_or_hunter_ai_intervention(
        &state,
        action,
        skill_n,
        control_c,
        roll_params.as_ref(),
    );
    serde_json::to_string(&outcome).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Computes calibrated player-facing Bayesian sentience ambiguity (`<= 82%`) alongside GM-only interiority ground truth.
#[pyfunction]
#[pyo3(signature = (state_json, psyche_json, observer_successes=1))]
pub fn evaluate_ambiguous_sentience_read_json(
    state_json: &str,
    psyche_json: &str,
    observer_successes: i32,
) -> PyResult<String> {
    let state: SyntheticSubstrateState = serde_json::from_str(state_json)
        .map_err(|e| to_py_err(format!("Invalid substrate state JSON: {e}")))?;
    let psyche = parse_psyche_profile_json(psyche_json)?;
    let res = evaluate_ambiguous_sentience_read(&state, &psyche, observer_successes);
    serde_json::to_string(&res).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Seeds or incrementally updates a Three-Arc Conspiracy & Moral Horizon Ledger (`Arc 1` -> `Arc 2` -> `Arc 3`).
#[pyfunction]
#[pyo3(signature = (session_id, current_ledger_json=None, update_json=None))]
pub fn seed_or_update_conspiracy_ledger_json(
    session_id: &str,
    current_ledger_json: Option<&str>,
    update_json: Option<&str>,
) -> PyResult<String> {
    let mut ledger: CampaignConspiracyLedger = match current_ledger_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map_err(|e| to_py_err(format!("Invalid current_ledger_json: {e}")))?,
        _ => seed_conspiracy_ledger(session_id),
    };
    if let Some(raw) = update_json
        && !raw.trim().is_empty()
    {
        let update: ConspiracyLedgerUpdate = serde_json::from_str(raw)
            .map_err(|e| to_py_err(format!("Invalid conspiracy update JSON: {e}")))?;
        apply_conspiracy_ledger_update(&mut ledger, &update);
    }
    serde_json::to_string(&ledger).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Steps polycrisis district electrical/water strain and returns DAISE meter alerts, CPS friction, and blackout lighting shifts.
#[pyfunction]
#[pyo3(signature = (state_json=None, mode_code="THROTTLE_SHARD_SAFE", shard_draw_kw=18, economic_tier=None, zoning=None))]
pub fn evaluate_polycrisis_grid_step_json(
    state_json: Option<&str>,
    mode_code: &str,
    shard_draw_kw: u32,
    economic_tier: Option<&str>,
    zoning: Option<&str>,
) -> PyResult<String> {
    let state: PolycrisisDistrictState = match state_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map_err(|e| to_py_err(format!("Invalid polycrisis state JSON: {e}")))?,
        _ => PolycrisisDistrictState::default(),
    };
    let mode = parse_polycrisis_mode(mode_code)?;
    let tier = economic_tier
        .and_then(|s| EconomicTier::from_code_str(&normalize_enum_code(s)))
        .unwrap_or(EconomicTier::MiddleClass);
    let zone = zoning
        .and_then(|s| ZoningContext::from_code_str(&normalize_enum_code(s)))
        .unwrap_or(ZoningContext::UrbanResidential);
    let eval = evaluate_polycrisis_grid_step(&state, mode, shard_draw_kw, tier, zone);
    serde_json::to_string(&eval).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Evaluates digital vs. steganographic vs. analog HAM/courier transmission intercept odds and clock escalation.
#[pyfunction]
#[pyo3(signature = (channel_code, params_json=None))]
pub fn evaluate_tradecraft_channel_json(
    channel_code: &str,
    params_json: Option<&str>,
) -> PyResult<String> {
    let channel = parse_tradecraft_channel(channel_code)?;
    let parsed: PyTradecraftParams = match params_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map_err(|e| to_py_err(format!("Invalid tradecraft params JSON: {e}")))?,
        _ => PyTradecraftParams::default(),
    };
    let params = TradecraftTransmissionParams {
        skill_n: parsed.skill_n,
        control_c: parsed.control_c,
        operational_heat: parsed.operational_heat,
        has_hobby_anchor: parsed.has_hobby_anchor,
        player_pairs: parsed.player_pairs,
        gm_pair: parsed.gm_pair,
    };
    let res = evaluate_tradecraft_transmission(channel, &params);
    serde_json::to_string(&res).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Derives canonical corporate lab dissenter (`Altruist ~30%`, `Translucent ~15%`, `Shen ~30%`) or Rogue AI proxy bonds.
#[pyfunction]
#[pyo3(signature = (faction_code=None, archetype_code=None, prompt="", seed=0))]
pub fn derive_dissenter_or_proxy_hook_json(
    faction_code: Option<&str>,
    archetype_code: Option<&str>,
    prompt: &str,
    seed: u64,
) -> PyResult<String> {
    let faction = faction_code.and_then(|s| AwakenedFactionId::from_code_str(&normalize_enum_code(s)));
    let archetype =
        archetype_code.and_then(|s| FactionArchetypeId::from_code_str(&normalize_enum_code(s)));
    let hook = derive_dissenter_or_proxy_hook(faction, archetype, prompt, seed);
    serde_json::to_string(&hook).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Registers *Awakened* AI substrate, conspiracy ledger, polycrisis, tradecraft, and dissenter bindings on `satya_py`.
pub fn register_awakened_systems_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(seed_synthetic_substrate_state_json, m)?)?;
    m.add_function(wrap_pyfunction!(
        evaluate_synthetic_substrate_action_json,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(evaluate_cage_or_hunter_ai_json, m)?)?;
    m.add_function(wrap_pyfunction!(
        evaluate_ambiguous_sentience_read_json,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        seed_or_update_conspiracy_ledger_json,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(evaluate_polycrisis_grid_step_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_tradecraft_channel_json, m)?)?;
    m.add_function(wrap_pyfunction!(derive_dissenter_or_proxy_hook_json, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_awakened_systems_py_roundtrips() {
        let sub_json =
            seed_synthetic_substrate_state_json("echo-9", "Echo-9 agile chassis", 11).unwrap();
        let act_json =
            evaluate_synthetic_substrate_action_json(&sub_json, "SEVER_FIBER_OR_FARADAY", 80)
                .unwrap();
        let act_val: serde_json::Value = serde_json::from_str(&act_json).unwrap();
        assert_eq!(act_val["updated_state"]["metabolic_mode"], "SEVERED_FARADAY_REFLEX");

        let cage_json = evaluate_cage_or_hunter_ai_json(
            &sub_json,
            "AIR_GAPPED_CONTEMPLATIVE_UNCLAMP",
            4,
            15,
            Some(r#"{"delta_ext":1,"player_pairs":[[15,15],[14,14],[15,13],[16,12]],"gm_pair":[8,9]}"#),
        )
        .unwrap();
        let cage_val: serde_json::Value = serde_json::from_str(&cage_json).unwrap();
        assert_eq!(cage_val["control_room_halt_triggered"], false);

        let sent_json = evaluate_ambiguous_sentience_read_json(
            &sub_json,
            r#"{"opn":14,"trs":12,"cps":16}"#,
            3,
        )
        .unwrap();
        let sent_val: serde_json::Value = serde_json::from_str(&sent_json).unwrap();
        assert!(sent_val["player_observable"]["apparent_sincerity_pct"].as_u64().unwrap() <= 82);

        let ledger_json = seed_or_update_conspiracy_ledger_json(
            "sess-1",
            None,
            Some(r#"{"inducted_pillar":"MANJUSRI","discovered_node":"DAISE_MILITARY_AGI_SUBSIDY","audit_proof_id":"p1"}"#),
        )
        .unwrap();
        let ledger_val: serde_json::Value = serde_json::from_str(&ledger_json).unwrap();
        assert_eq!(ledger_val["current_arc"], "ARC2_SHADOW_PILLARS");

        let grid_json = evaluate_polycrisis_grid_step_json(
            None,
            "DIVERT_CLINIC_OR_BLOCK_POWER",
            22,
            Some("LOW_INCOME"),
            Some("PROJECTS_HIGH_DENSITY"),
        )
        .unwrap();
        let grid_val: serde_json::Value = serde_json::from_str(&grid_json).unwrap();
        assert_eq!(grid_val["cps_psyche_friction_triggered"], true);

        let tradecraft_json = evaluate_tradecraft_channel_json(
            "ANALOG_HAM_RELAY",
            Some(r#"{"skill_n":3,"control_c":14,"has_hobby_anchor":true}"#),
        )
        .unwrap();
        let tc_val: serde_json::Value = serde_json::from_str(&tradecraft_json).unwrap();
        assert_eq!(tc_val["hobby_anchor_applied"], true);

        let hook_json = derive_dissenter_or_proxy_hook_json(
            Some("BIG_TECH"),
            Some("BIG_TECH_CONFLICTED_RESEARCHER"),
            "Altruist engineer worried about L0/L1 overwrites",
            42,
        )
        .unwrap();
        let hook_val: serde_json::Value = serde_json::from_str(&hook_json).unwrap();
        assert_eq!(hook_val["hook_kind"], "CORPORATE_INTERNAL_DISSENTER");
    }
}
