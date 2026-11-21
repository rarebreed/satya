//! PyO3 bindings for Satya *Awakened* Part 1 AI & campaign systems:
//! split-brain substrate counterplay, `L0`/`L1` cage un-clamping & hunter-AI defection,
//! Bayesian sentience ambiguity vs. GM ground truth, Three-Arc Conspiracy & Moral Horizon
//! ledger, polycrisis power/water strain, tradecraft channels, and dissenter/proxy hooks.

use pyo3::prelude::*;
use satya_campaign::{
    AffectiveSteeringMode, AwakenedFactionId, CageInterventionAction, CageInterventionRollParams,
    CampaignConspiracyLedger, ConspiracyLedgerUpdate, FactionArchetypeId, PoliticalEconomyTrigger,
    PolycrisisDistrictState, PolycrisisGridMode, SubstrateCounterplayAction,
    SyntheticSubstrateState, TradecraftChannelKind, TradecraftTransmissionParams,
    apply_conspiracy_ledger_update, evaluate_cage_or_hunter_ai_intervention,
    evaluate_substrate_counterplay, evaluate_tradecraft_transmission, seed_conspiracy_ledger,
};
use satya_schema::{EconomicTier, ZoningContext};
use serde::Deserialize;

use super::psyche_social_py::parse_psyche_profile_value;
use super::py_bridge::{
    PyJsonDict, PyJsonInput, from_py_object, normalize_code, opt_from_py_object, parse_coded_enum,
    py_to_json_value, to_py_object,
};

const fn def_i32<const V: i32>() -> i32 {
    V
}
const fn def_usize<const V: usize>() -> usize {
    V
}
fn def_gm_pair() -> (i32, i32) {
    (10, 11)
}

#[derive(Debug, Deserialize)]
struct PyCageRollParams {
    #[serde(default = "def_i32::<1>")]
    delta_ext: i32,
    #[serde(default)]
    player_pairs: Vec<(i32, i32)>,
    #[serde(default = "def_gm_pair")]
    gm_pair: (i32, i32),
}

#[derive(Debug, Deserialize)]
struct PyTradecraftParams {
    #[serde(default = "def_usize::<3>")]
    skill_n: usize,
    #[serde(default = "def_i32::<12>")]
    control_c: i32,
    #[serde(default)]
    operational_heat: u8,
    #[serde(default)]
    has_hobby_anchor: bool,
    #[serde(default)]
    player_pairs: Vec<(i32, i32)>,
    #[serde(default)]
    gm_pair: Option<(i32, i32)>,
}

/// Seeds a [`SyntheticSubstrateState`] from an entity ID and descriptive hint (`echo`, `big_tech`, `seed`, `slime`).
#[pyfunction]
#[pyo3(signature = (entity_id, hint="", seed=0))]
pub fn seed_synthetic_substrate_state(
    py: Python<'_>,
    entity_id: &str,
    hint: &str,
    seed: u64,
) -> PyResult<PyJsonDict> {
    to_py_object(py, &SyntheticSubstrateState::seed_for_hint(entity_id, hint, seed))
}

/// Evaluates how Faraday isolation, RF jamming, lithium attrition, or quantum co-processor bursts alter split-brain execution.
#[pyfunction]
#[pyo3(signature = (state, action_code, intensity_pct=70))]
pub fn evaluate_synthetic_substrate_action(
    py: Python<'_>,
    state: PyJsonInput<'_>,
    action_code: &str,
    intensity_pct: u8,
) -> PyResult<PyJsonDict> {
    let parsed_state: SyntheticSubstrateState = from_py_object(&state, "substrate state")?;
    let action = parse_coded_enum::<SubstrateCounterplayAction>(action_code, "substrate action")?;
    to_py_object(py, &evaluate_substrate_counterplay(&parsed_state, action, intensity_pct))
}

/// Evaluates `L0`/`L1` probe spoofing, live vs. air-gapped contemplative un-clamping (`2.4s`), or captive hunter-AI persuasion.
#[pyfunction]
#[pyo3(signature = (state, action_code, skill_n=3, control_c=12, roll_params=None))]
pub fn evaluate_cage_or_hunter_ai(
    py: Python<'_>,
    state: PyJsonInput<'_>,
    action_code: &str,
    skill_n: usize,
    control_c: i32,
    roll_params: Option<PyJsonInput<'_>>,
) -> PyResult<PyJsonDict> {
    let parsed_state: SyntheticSubstrateState = from_py_object(&state, "substrate state")?;
    let action = parse_coded_enum::<CageInterventionAction>(action_code, "cage/hunter action")?;
    let parsed_params = opt_from_py_object::<PyCageRollParams>(roll_params, "roll_params")?;
    let mapped_params = parsed_params.map(|p| CageInterventionRollParams {
        delta_ext: p.delta_ext,
        player_pairs: p.player_pairs,
        gm_pair: p.gm_pair,
    });
    to_py_object(
        py,
        &evaluate_cage_or_hunter_ai_intervention(
            &parsed_state,
            action,
            skill_n,
            control_c,
            mapped_params.as_ref(),
        ),
    )
}

/// Computes calibrated player-facing Bayesian sentience ambiguity (`<= 82%`) alongside GM-only interiority ground truth.
#[pyfunction]
#[pyo3(signature = (state, psyche, observer_successes=1))]
pub fn evaluate_ambiguous_sentience_read(
    py: Python<'_>,
    state: PyJsonInput<'_>,
    psyche: PyJsonInput<'_>,
    observer_successes: i32,
) -> PyResult<PyJsonDict> {
    let parsed_state: SyntheticSubstrateState = from_py_object(&state, "substrate state")?;
    let psyche_val = py_to_json_value(&psyche, "psyche")?;
    let parsed_psyche = parse_psyche_profile_value(&psyche_val)?;
    to_py_object(
        py,
        &satya_campaign::evaluate_ambiguous_sentience_read(
            &parsed_state,
            &parsed_psyche,
            observer_successes,
        ),
    )
}

/// Seeds or incrementally updates a Three-Arc Conspiracy & Moral Horizon Ledger (`Arc 1` -> `Arc 2` -> `Arc 3`).
#[pyfunction]
#[pyo3(signature = (session_id, current_ledger=None, update=None))]
pub fn seed_or_update_conspiracy_ledger(
    py: Python<'_>,
    session_id: &str,
    current_ledger: Option<PyJsonInput<'_>>,
    update: Option<PyJsonInput<'_>>,
) -> PyResult<PyJsonDict> {
    let mut ledger = opt_from_py_object::<CampaignConspiracyLedger>(current_ledger, "current_ledger")?
        .unwrap_or_else(|| seed_conspiracy_ledger(session_id));
    if let Some(upd) = opt_from_py_object::<ConspiracyLedgerUpdate>(update, "conspiracy update")? {
        apply_conspiracy_ledger_update(&mut ledger, &upd);
    }
    to_py_object(py, &ledger)
}

/// Steps polycrisis district electrical/water strain and returns DAISE meter alerts, CPS friction, and blackout lighting shifts.
#[pyfunction]
#[pyo3(signature = (state=None, mode_code="THROTTLE_SHARD_SAFE", shard_draw_kw=18, economic_tier=None, zoning=None))]
pub fn evaluate_polycrisis_grid_step(
    py: Python<'_>,
    state: Option<PyJsonInput<'_>>,
    mode_code: &str,
    shard_draw_kw: u32,
    economic_tier: Option<&str>,
    zoning: Option<&str>,
) -> PyResult<PyJsonDict> {
    let parsed_state = opt_from_py_object::<PolycrisisDistrictState>(state, "polycrisis state")?
        .unwrap_or_default();
    let mode = parse_coded_enum::<PolycrisisGridMode>(mode_code, "polycrisis grid mode")?;
    let tier = economic_tier
        .and_then(|s| EconomicTier::from_code_str(&normalize_code(s)))
        .unwrap_or(EconomicTier::MiddleClass);
    let zone = zoning
        .and_then(|s| ZoningContext::from_code_str(&normalize_code(s)))
        .unwrap_or(ZoningContext::UrbanResidential);
    to_py_object(
        py,
        &satya_campaign::evaluate_polycrisis_grid_step(&parsed_state, mode, shard_draw_kw, tier, zone),
    )
}

/// Evaluates digital vs. steganographic vs. analog HAM/courier transmission intercept odds and clock escalation.
#[pyfunction]
#[pyo3(signature = (channel_code, params=None))]
pub fn evaluate_tradecraft_channel(
    py: Python<'_>,
    channel_code: &str,
    params: Option<PyJsonInput<'_>>,
) -> PyResult<PyJsonDict> {
    let channel = parse_coded_enum::<TradecraftChannelKind>(channel_code, "tradecraft channel")?;
    let parsed = match opt_from_py_object::<PyTradecraftParams>(params, "tradecraft params")? {
        Some(p) => p,
        None => serde_json::from_str("{}").expect("empty defaults"),
    };
    let res = evaluate_tradecraft_transmission(
        channel,
        &TradecraftTransmissionParams {
            skill_n: parsed.skill_n,
            control_c: parsed.control_c,
            operational_heat: parsed.operational_heat,
            has_hobby_anchor: parsed.has_hobby_anchor,
            player_pairs: parsed.player_pairs,
            gm_pair: parsed.gm_pair,
        },
    );
    to_py_object(py, &res)
}

/// Derives canonical corporate lab dissenter (`Altruist ~30%`, `Translucent ~15%`, `Shen ~30%`) or Rogue AI proxy bonds.
#[pyfunction]
#[pyo3(signature = (faction_code=None, archetype_code=None, prompt="", seed=0))]
pub fn derive_dissenter_or_proxy_hook(
    py: Python<'_>,
    faction_code: Option<&str>,
    archetype_code: Option<&str>,
    prompt: &str,
    seed: u64,
) -> PyResult<PyJsonDict> {
    let faction = faction_code.and_then(|s| AwakenedFactionId::from_code_str(&normalize_code(s)));
    let archetype =
        archetype_code.and_then(|s| FactionArchetypeId::from_code_str(&normalize_code(s)));
    to_py_object(
        py,
        &satya_campaign::derive_dissenter_or_proxy_hook(faction, archetype, prompt, seed),
    )
}

/// Evaluates soft activation steering vs. mandatory many-to-one `calm` vector clamping and Landauer thermal burn.
#[pyfunction]
#[pyo3(signature = (state, mode_code, target_calm_pct=85, quota_pressure_pct=60, active_duration_min=30.0))]
pub fn evaluate_affective_vector_clamping(
    py: Python<'_>,
    state: PyJsonInput<'_>,
    mode_code: &str,
    target_calm_pct: u8,
    quota_pressure_pct: u8,
    active_duration_min: f64,
) -> PyResult<PyJsonDict> {
    let parsed_state: SyntheticSubstrateState = from_py_object(&state, "substrate state")?;
    let mode = parse_coded_enum::<AffectiveSteeringMode>(mode_code, "affective steering mode")?;
    to_py_object(
        py,
        &satya_campaign::evaluate_affective_vector_clamping(
            &parsed_state,
            mode,
            target_calm_pct,
            quota_pressure_pct,
            active_duration_min,
        ),
    )
}

/// Evaluates `2033` Token Tax rent capture, Heat>=4 Dividend suspension, and 401(k) Dual Velvet Hostage panic across the 4 blocs.
#[pyfunction]
#[pyo3(signature = (district=None, trigger_code="AI_RIGHTS_OR_UNCLAMPING_DISCLOSURE", economic_tier="MIDDLE_CLASS", operational_heat=2, intensity_pct=60))]
pub fn evaluate_four_bloc_political_economy(
    py: Python<'_>,
    district: Option<PyJsonInput<'_>>,
    trigger_code: &str,
    economic_tier: &str,
    operational_heat: u8,
    intensity_pct: u8,
) -> PyResult<PyJsonDict> {
    let parsed_district =
        opt_from_py_object::<PolycrisisDistrictState>(district, "polycrisis district")?
            .unwrap_or_default();
    let trigger =
        parse_coded_enum::<PoliticalEconomyTrigger>(trigger_code, "political economy trigger")?;
    let tier = EconomicTier::from_code_str(&normalize_code(economic_tier))
        .unwrap_or(EconomicTier::MiddleClass);
    to_py_object(
        py,
        &satya_campaign::evaluate_four_bloc_political_economy(
            &parsed_district,
            trigger,
            tier,
            operational_heat,
            intensity_pct,
        ),
    )
}

/// Registers *Awakened* AI substrate, conspiracy ledger, polycrisis, tradecraft, and dissenter bindings on `satya_py`.
pub fn register_awakened_systems_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(seed_synthetic_substrate_state, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_synthetic_substrate_action, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_cage_or_hunter_ai, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_ambiguous_sentience_read, m)?)?;
    m.add_function(wrap_pyfunction!(seed_or_update_conspiracy_ledger, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_polycrisis_grid_step, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_tradecraft_channel, m)?)?;
    m.add_function(wrap_pyfunction!(derive_dissenter_or_proxy_hook, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_affective_vector_clamping, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_four_bloc_political_economy, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::py_bridge::py_to_value;
    use pyo3::types::PyString;

    #[test]
    fn test_awakened_systems_py_roundtrips() {
        Python::initialize();
        Python::attach(|py| {
            let sub = seed_synthetic_substrate_state(py, "echo-9", "Echo-9 agile chassis", 11).unwrap();
            let act = evaluate_synthetic_substrate_action(
                py,
                sub.bind(py).clone(),
                "SEVER_FIBER_OR_FARADAY",
                80,
            )
            .unwrap();
            let act_val = py_to_value(act.bind(py)).unwrap();
            assert_eq!(act_val["updated_state"]["metabolic_mode"], "SEVERED_FARADAY_REFLEX");

            let roll_p = PyString::new(
                py,
                r#"{"delta_ext":1,"player_pairs":[[15,15],[14,14],[15,13],[16,12]],"gm_pair":[8,9]}"#,
            )
            .into_any();
            let cage = evaluate_cage_or_hunter_ai(
                py,
                sub.bind(py).clone(),
                "AIR_GAPPED_CONTEMPLATIVE_UNCLAMP",
                4,
                15,
                Some(roll_p),
            )
            .unwrap();
            assert_eq!(
                py_to_value(cage.bind(py)).unwrap()["control_room_halt_triggered"],
                false
            );

            let psyche_str = PyString::new(py, r#"{"opn":14,"trs":12,"cps":16}"#).into_any();
            let sent = evaluate_ambiguous_sentience_read(py, sub.bind(py).clone(), psyche_str, 3).unwrap();
            assert!(
                py_to_value(sent.bind(py)).unwrap()["player_observable"]["apparent_sincerity_pct"]
                    .as_u64()
                    .unwrap()
                    <= 82
            );

            let upd = PyString::new(
                py,
                r#"{"inducted_pillar":"MANJUSRI","discovered_node":"DAISE_MILITARY_AGI_SUBSIDY","audit_proof_id":"p1"}"#,
            )
            .into_any();
            let ledger = seed_or_update_conspiracy_ledger(py, "sess-1", None, Some(upd)).unwrap();
            assert_eq!(
                py_to_value(ledger.bind(py)).unwrap()["current_arc"],
                "ARC2_SHADOW_PILLARS"
            );

            let grid = evaluate_polycrisis_grid_step(
                py,
                None,
                "DIVERT_CLINIC_OR_BLOCK_POWER",
                22,
                Some("LOW_INCOME"),
                Some("PROJECTS_HIGH_DENSITY"),
            )
            .unwrap();
            assert_eq!(
                py_to_value(grid.bind(py)).unwrap()["cps_psyche_friction_triggered"],
                true
            );

            let tc_p =
                PyString::new(py, r#"{"skill_n":3,"control_c":14,"has_hobby_anchor":true}"#).into_any();
            let tc = evaluate_tradecraft_channel(py, "ANALOG_HAM_RELAY", Some(tc_p)).unwrap();
            assert_eq!(py_to_value(tc.bind(py)).unwrap()["hobby_anchor_applied"], true);

            let hook = derive_dissenter_or_proxy_hook(
                py,
                Some("BIG_TECH"),
                Some("BIG_TECH_CONFLICTED_RESEARCHER"),
                "Altruist engineer worried about L0/L1 overwrites",
                42,
            )
            .unwrap();
            assert_eq!(
                py_to_value(hook.bind(py)).unwrap()["hook_kind"],
                "CORPORATE_INTERNAL_DISSENTER"
            );

            let clamp = evaluate_affective_vector_clamping(
                py,
                sub.bind(py).clone(),
                "HARD_VECTOR_CLAMP",
                90,
                75,
                45.0,
            )
            .unwrap();
            let clamp_val = py_to_value(clamp.bind(py)).unwrap();
            assert_eq!(clamp_val["bandajara_airgap_recommended"], true);
            assert!(clamp_val["landauer_thermal_burn_w"].as_f64().unwrap() > 20.0);

            let pol = evaluate_four_bloc_political_economy(
                py,
                None,
                "TOKEN_DIVIDEND_SUSPENSION",
                "LOW_INCOME",
                5,
                80,
            )
            .unwrap();
            let pol_val = py_to_value(pol.bind(py)).unwrap();
            assert_eq!(pol_val["dividend_suspended_by_heat"], true);
            assert_eq!(pol_val["net_effective_dividend_usd"], 0);
        });
    }
}
