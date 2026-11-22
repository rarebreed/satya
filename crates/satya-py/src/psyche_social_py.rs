//! PyO3 bindings for Satya's Social Combat, Psyche Friction/Drift, Conviction Surges,
//! and Faction/Heat World Clocks (`psyche_social`, `social_combat`, `world_clocks`).

use pyo3::prelude::*;
use satya_campaign::{apply_world_ledger_update, seed_default_world_ledger};
use satya_engine::{
    BattlefieldMoraleSituation, CareerEchelonTier, CareerTermConfig, CareerTermTrack,
    CharacterAptitudes, CharacterCharacteristics, CombatMotivationKind,
    ConvictionPrincipleArchetype, DissonanceResolutionPath, EmotionalTriggerKind,
    LifepathBuildConfig, MoraleState, PsycheProfile, PsycheTraitId, SocialArenaKind,
    SocialArgumentVector, SocialComposureState, compute_battlefield_morale_delta,
    compute_lifepath_baseline_from_config, evaluate_combat_motivation_modifier, evaluate_social_exchange_wounds,
    evaluate_social_standing_modifier,
};
use satya_schema::{
    EconomicTier, SessionWorldLedger, UpdateWorldLedgerRequest, WorldLedgerUpdateResponse,
    ZoningContext,
};
use serde::Serialize;
use serde_json::{Map, Value, json};

use super::py_bridge::{
    PyEnum, PyJsonDict, PyJsonInput, PyJsonList, PyListInput, from_py_object,
    merge_object_defaults, opt_from_py_object, parse_coded_enum, py_to_json_value, to_py_err,
    to_py_object, unwrap_py_enum_or,
};

fn read_i32(map: &Map<String, Value>, keys: &[&str], default: i32) -> i32 {
    for k in keys {
        if let Some(n) = map.get(*k).and_then(Value::as_i64) {
            return (n as i32).clamp(1, 20);
        }
    }
    default
}

/// Normalizes a psyche mapping (`characteristics` / `psyche` / `psyche_traits` / flat dict) to [`PsycheProfile`].
pub(crate) fn parse_psyche_profile_value(val: &Value) -> PyResult<PsycheProfile> {
    let root = val
        .as_object()
        .ok_or_else(|| to_py_err("Psyche payload must be a mapping/object"))?;
    let map = root
        .get("characteristics")
        .or_else(|| root.get("psyche"))
        .or_else(|| root.get("psyche_traits"))
        .and_then(|v| v.as_object())
        .unwrap_or(root);

    Ok(PsycheProfile {
        opn: read_i32(map, &["opn", "OPN", "openness"], 10),
        slf: read_i32(map, &["slf", "SLF", "selfishness"], 10),
        blg: read_i32(map, &["blg", "BLG", "belonging", "belongingness"], 10),
        agg: read_i32(map, &["agg", "AGG", "aggression", "aggressiveness"], 10),
        ego: read_i32(map, &["ego", "EGO"], 10),
        cnf: read_i32(map, &["cnf", "CNF", "conformity"], 10),
        ext: read_i32(map, &["ext", "EXT", "extraversion"], 10),
        trs: read_i32(map, &["trs", "TRS", "trust", "trusting"], 10),
        cps: read_i32(map, &["cps", "CPS", "compassion"], 10),
    })
}

fn normalize_vector_code_and_hints(raw: &str) -> (String, bool, bool) {
    let lower = raw.trim().to_ascii_lowercase();
    let audit_hint = lower.contains("audit") || lower.contains("verif");
    let triage_hint = lower.contains("utilitarian")
        || lower.contains("triage")
        || lower.contains("greater_good")
        || lower.contains("greater good");

    let base = raw.split('(').next().unwrap_or(raw).trim();
    let mut snake = String::with_capacity(base.len() + 8);
    let mut prev_is_lower_or_digit = false;
    let mut prev_is_underscore = true;
    for ch in base.chars() {
        if ch == '-' || ch == '_' || ch.is_whitespace() {
            if !prev_is_underscore {
                snake.push('_');
                prev_is_underscore = true;
            }
            prev_is_lower_or_digit = false;
        } else if ch.is_ascii_uppercase() {
            if prev_is_lower_or_digit && !prev_is_underscore {
                snake.push('_');
            }
            snake.push(ch.to_ascii_lowercase());
            prev_is_lower_or_digit = false;
            prev_is_underscore = false;
        } else {
            snake.push(ch.to_ascii_lowercase());
            prev_is_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
            prev_is_underscore = false;
        }
    }
    (snake.trim_matches('_').to_string(), audit_hint, triage_hint)
}

fn map_vector_code(raw: &str, flags: Option<&Map<String, Value>>) -> PyResult<SocialArgumentVector> {
    let (norm, audit_hint, triage_hint) = normalize_vector_code_and_hints(raw);
    let flag = |key: &str| -> bool {
        flags
            .and_then(|m| m.get(key))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    match norm.as_str() {
        "empirical_logic" | "empirical_evidence_logic_novel_ideas" | "logic" | "evidence" => {
            Ok(SocialArgumentVector::EmpiricalEvidenceLogicNovelIdeas)
        }
        "group_loyalty" | "group_consensus_loyalty_shared_identity" | "loyalty" => {
            Ok(SocialArgumentVector::GroupConsensusLoyaltySharedIdentity {
                asks_to_betray_group: flag("asks_to_betray_group"),
            })
        }
        "betray_group" => Ok(SocialArgumentVector::GroupConsensusLoyaltySharedIdentity {
            asks_to_betray_group: true,
        }),
        "flattery_prestige" | "flattery_prestige_legacy_status" | "flattery" | "prestige" => {
            Ok(SocialArgumentVector::FlatteryPrestigeLegacyStatus {
                humiliates_or_demands_fault_admission: flag(
                    "humiliates_or_demands_fault_admission",
                ),
            })
        }
        "humiliate_fault" => Ok(SocialArgumentVector::FlatteryPrestigeLegacyStatus {
            humiliates_or_demands_fault_admission: true,
        }),
        "personal_profit" | "personal_profit_immunity_self_interest" | "profit" | "bribe" => {
            Ok(SocialArgumentVector::PersonalProfitImmunitySelfInterest)
        }
        "humanitarian_mercy"
        | "self_sacrifice_humanitarian_duty_mercy"
        | "mercy"
        | "humanitarian" => Ok(SocialArgumentVector::SelfSacrificeHumanitarianDutyMercy),
        "rules_protocol" | "rules_law_protocol_chain_of_command" | "rules" | "protocol" => {
            Ok(SocialArgumentVector::RulesLawProtocolChainOfCommand {
                asks_to_break_law_or_orders: flag("asks_to_break_law_or_orders"),
            })
        }
        "break_law_orders" => Ok(SocialArgumentVector::RulesLawProtocolChainOfCommand {
            asks_to_break_law_or_orders: true,
        }),
        "dominance_threat" | "dominance_threat_dare_confrontation" | "threat" | "intimidation" => {
            Ok(SocialArgumentVector::DominanceThreatDareConfrontation {
                aims_to_bait_into_fight: flag("aims_to_bait_into_fight"),
            })
        }
        "bait_into_fight" => Ok(SocialArgumentVector::DominanceThreatDareConfrontation {
            aims_to_bait_into_fight: true,
        }),
        "transparency_good_faith"
        | "transparency_vulnerability_good_faith"
        | "trust_alliance_guarantee_reciprocity"
        | "transparency"
        | "good_faith" => Ok(SocialArgumentVector::TransparencyVulnerabilityGoodFaith {
            verifies_with_audit_or_proof: flag("verifies_with_audit_or_proof") || audit_hint,
        }),
        "audit_proof" | "verified_audit_proof" => {
            Ok(SocialArgumentVector::TransparencyVulnerabilityGoodFaith {
                verifies_with_audit_or_proof: true,
            })
        }
        "moral_sanctity"
        | "moral_sanctity_witnessed_suffering_mercy"
        | "moral_witness_vulnerability_compassion"
        | "sanctity"
        | "witnessed_suffering"
        | "compassion" => Ok(SocialArgumentVector::MoralSanctityWitnessedSufferingMercy {
            utilitarian_greater_good_triage: flag("utilitarian_greater_good_triage") || triage_hint,
        }),
        "utilitarian_triage" | "greater_good_triage" => {
            Ok(SocialArgumentVector::MoralSanctityWitnessedSufferingMercy {
                utilitarian_greater_good_triage: true,
            })
        }
        _ => Err(to_py_err(format!("Unknown social argument vector '{raw}'"))),
    }
}

fn parse_social_vectors_value(val: Value) -> PyResult<Vec<SocialArgumentVector>> {
    let arr = val
        .as_array()
        .ok_or_else(|| to_py_err("Social vectors payload must be a sequence/list"))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        if let Some(s) = item.as_str() {
            out.push(map_vector_code(s, None)?);
        } else if let Some(obj) = item.as_object() {
            if let Ok(parsed) = serde_json::from_value::<SocialArgumentVector>(item.clone()) {
                out.push(parsed);
            } else {
                let name = obj
                    .get("vector")
                    .or_else(|| obj.get("kind"))
                    .or_else(|| obj.get("name"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| to_py_err("Vector dict must contain 'vector' or 'kind'"))?;
                out.push(map_vector_code(name, Some(obj))?);
            }
        } else {
            return Err(to_py_err("Each vector entry must be a string or dict"));
        }
    }
    Ok(out)
}

/// Evaluates an Empathy & Read Person check (`INS/ITU` or `INS/ELO`) against a target's Psyche.
#[pyfunction]
#[pyo3(signature = (target_psyche, net_successes))]
pub fn evaluate_empathy_read(
    py: Python<'_>,
    target_psyche: PyJsonInput<'_>,
    net_successes: i32,
) -> PyResult<PyJsonDict> {
    let psyche = parse_psyche_profile_value(&py_to_json_value(&target_psyche, "target_psyche")?)?;
    let res = satya_engine::evaluate_empathy_read(&psyche, net_successes);
    let revealed: Vec<Value> = res
        .revealed_traits
        .iter()
        .map(|&(tid, score)| {
            json!({
                "trait_id": serde_json::to_value(tid).unwrap_or_default(),
                "code": tid.code(),
                "label": tid.label(),
                "score": score,
            })
        })
        .collect();
    to_py_object(
        py,
        &json!({
            "net_successes": res.net_successes,
            "revealed_traits": revealed,
            "detects_psyche_wedge_tension": res.detects_psyche_wedge_tension,
            "leverage_delta_ext_bonus": res.leverage_delta_ext_bonus,
        }),
    )
}

/// Evaluates a Contested Social Exchange against a target's Psyche profile and `SocialComposureState`.
#[pyfunction]
#[pyo3(signature = (
    target_psyche,
    vectors,
    net_successes,
    composure_state=None,
    target_dis=12,
    target_foc=12,
    ego_wound_dis_check_passed=None
))]
pub fn evaluate_social_exchange(
    py: Python<'_>,
    target_psyche: PyJsonInput<'_>,
    vectors: PyListInput<'_>,
    net_successes: i32,
    composure_state: Option<PyJsonInput<'_>>,
    target_dis: i32,
    target_foc: i32,
    ego_wound_dis_check_passed: Option<bool>,
) -> PyResult<PyJsonDict> {
    let psyche = parse_psyche_profile_value(&py_to_json_value(&target_psyche, "target_psyche")?)?;
    let parsed_vectors = parse_social_vectors_value(py_to_json_value(&vectors, "vectors")?)?;
    let state = opt_from_py_object::<SocialComposureState>(composure_state, "composure_state")?
        .unwrap_or_else(|| SocialComposureState::new(target_dis, target_foc));
    let outcome = evaluate_social_exchange_wounds(
        &psyche,
        &state,
        &parsed_vectors,
        net_successes,
        ego_wound_dis_check_passed,
    );
    to_py_object(py, &outcome)
}

/// Resolves active Cognitive Dissonance via one of the 4 canonical Dissonance Resolution Paths.
#[pyfunction]
#[pyo3(signature = (
    composure_state,
    dis=12,
    resolution_path="compartmentalization",
    violated_trait=None,
    dis_check_successes=1
))]
pub fn resolve_cognitive_dissonance(
    py: Python<'_>,
    composure_state: PyJsonInput<'_>,
    dis: i32,
    resolution_path: &str,
    violated_trait: Option<PyEnum<PsycheTraitId>>,
    dis_check_successes: i32,
) -> PyResult<PyJsonDict> {
    let state: SocialComposureState = from_py_object(&composure_state, "composure_state")?;
    let path_norm = resolution_path.trim().to_ascii_lowercase().replace('-', "_");
    let path = match path_norm.as_str() {
        "conversion_defection" | "conversion" | "defection" => {
            DissonanceResolutionPath::ConversionDefection {
                violated_trait: unwrap_py_enum_or(violated_trait, PsycheTraitId::Openness),
            }
        }
        "dogmatic_denial" | "denial" => DissonanceResolutionPath::DogmaticDenial {
            dis_check_successes,
        },
        "compartmentalization" | "compartmentalize" => {
            DissonanceResolutionPath::Compartmentalization {
                dis_check_successes,
            }
        }
        "emotional_lashing_out" | "lashing_out" | "lash_out" => {
            DissonanceResolutionPath::EmotionalLashingOut
        }
        _ => {
            return Err(to_py_err(format!(
                "Unknown resolution_path '{resolution_path}'"
            )));
        }
    };
    to_py_object(py, &satya_engine::resolve_cognitive_dissonance(&state, dis, path))
}

/// Evaluates whether a character triggers a Conviction Surge under severe internal strain.
#[pyfunction]
#[pyo3(signature = (psyche, current_delta_int, already_used_this_scene=false, principle=None))]
pub fn evaluate_conviction_surge(
    py: Python<'_>,
    psyche: PyJsonInput<'_>,
    current_delta_int: i32,
    already_used_this_scene: bool,
    principle: Option<&str>,
) -> PyResult<PyJsonDict> {
    let parsed_psyche = parse_psyche_profile_value(&py_to_json_value(&psyche, "psyche")?)?;
    let parsed_principle = match principle.map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(raw) => match raw.to_ascii_lowercase().replace('-', "_").as_str() {
            "altruistic_or_tribal_sacrifice" | "altruistic" | "sacrifice" => {
                Some(ConvictionPrincipleArchetype::AltruisticOrTribalSacrifice)
            }
            "boundless_compassion" => Some(ConvictionPrincipleArchetype::BoundlessCompassion),
            "unbowed_defiance" | "defiance" => Some(ConvictionPrincipleArchetype::UnbowedDefiance),
            "iconoclastic_truth" | "truth" => Some(ConvictionPrincipleArchetype::IconoclasticTruth),
            _ => Some(ConvictionPrincipleArchetype::ExtremeTraitEmbodiment(
                parse_coded_enum::<PsycheTraitId>(raw, "PsycheTraitId")?,
            )),
        },
    };
    to_py_object(
        py,
        &satya_engine::evaluate_conviction_surge(
            &parsed_psyche,
            current_delta_int,
            already_used_this_scene,
            parsed_principle,
        ),
    )
}

/// Applies Psyche Drift Marks toward `target_pole_score` (default `10` equilibrium).
#[pyfunction]
#[pyo3(signature = (trait_score, current_marks=0, marks_added=1, dis=12, target_pole_score=None))]
pub fn apply_psyche_drift_mark(
    py: Python<'_>,
    trait_score: i32,
    current_marks: i32,
    marks_added: i32,
    dis: i32,
    target_pole_score: Option<i32>,
) -> PyResult<PyJsonDict> {
    to_py_object(
        py,
        &satya_engine::apply_psyche_drift_mark(
            trait_score,
            current_marks,
            marks_added,
            dis,
            target_pole_score,
        ),
    )
}

/// Seeds or updates a [`SessionWorldLedger`] (faction clocks + `Heat` x [`EconomicTier`]) returning a native Python dict.
#[pyfunction]
#[pyo3(signature = (session_id, current_ledger=None, update_req=None, pack_id=None, campaign_tag=None))]
pub fn seed_or_update_world_ledger(
    py: Python<'_>,
    session_id: &str,
    current_ledger: Option<PyJsonInput<'_>>,
    update_req: Option<PyJsonInput<'_>>,
    pack_id: Option<&str>,
    campaign_tag: Option<&str>,
) -> PyResult<PyJsonDict> {
    let parsed_update =
        opt_from_py_object::<UpdateWorldLedgerRequest>(update_req, "update_req")?;
    let mut ledger = opt_from_py_object::<SessionWorldLedger>(current_ledger, "current_ledger")?
        .unwrap_or_else(|| {
            let eff_pack = pack_id
                .or_else(|| parsed_update.as_ref().and_then(|r| r.pack_id.as_deref()))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("awakened");
            let tier = parsed_update
                .as_ref()
                .and_then(|r| r.economic_tier)
                .unwrap_or(EconomicTier::MiddleClass);
            let zoning = parsed_update
                .as_ref()
                .and_then(|r| r.zoning)
                .unwrap_or(ZoningContext::UrbanResidential);
            seed_default_world_ledger(
                session_id,
                campaign_tag.unwrap_or(session_id),
                eff_pack,
                tier,
                zoning,
            )
        });
    let triggered_events = match parsed_update {
        Some(ref req) => apply_world_ledger_update(&mut ledger, req),
        None => Vec::new(),
    };
    to_py_object(
        py,
        &WorldLedgerUpdateResponse {
            ledger,
            triggered_events,
        },
    )
}

fn parse_combat_motivation_kind(raw: &str, command_intact: bool) -> CombatMotivationKind {
    match raw.trim().to_ascii_lowercase().as_str() {
        "fighting_for_crew_family_homeland" | "crew_loyalty" | "family" | "homeland" => {
            CombatMotivationKind::FightingForCrewFamilyHomeland
        }
        "shielding_innocents_sanctity_of_life" | "shielding_innocents" | "sanctity_of_life" => {
            CombatMotivationKind::ShieldingInnocentsSanctityOfLife
        }
        "mercenary_transactional_under_fire" | "mercenary" | "transactional" => {
            CombatMotivationKind::MercenaryTransactionalUnderFire
        }
        "sworn_duty_chain_of_command" | "sworn_duty" | "chain_of_command" => {
            CombatMotivationKind::SwornDutyChainOfCommand { command_intact }
        }
        _ => CombatMotivationKind::NeutralOrSurvival,
    }
}

fn parse_emotional_trigger_kind(raw: &str, wound_tier: i32) -> PyResult<EmotionalTriggerKind> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "being_wounded" | "wounded" => Ok(EmotionalTriggerKind::BeingWounded {
            wound_tier: wound_tier.clamp(1, 4),
        }),
        "friend_or_squadmate_wounded" | "squadmate_wounded" | "ally_wounded" => {
            Ok(EmotionalTriggerKind::FriendOrSquadmateWounded)
        }
        "loved_one_attacked_or_wounded" | "loved_one_attacked" | "loved_one_wounded" => {
            Ok(EmotionalTriggerKind::LovedOneAttackedOrWounded)
        }
        "betrayed_by_trusted_ally" | "betrayed" | "trap_sprung" => {
            Ok(EmotionalTriggerKind::BetrayedByTrustedAlly)
        }
        "insulted_or_taunted" | "taunted_or_insulted" | "insulted" | "taunted" => {
            Ok(EmotionalTriggerKind::InsultedOrTaunted)
        }
        "publicly_humiliated" | "humiliated" => Ok(EmotionalTriggerKind::PubliclyHumiliated),
        "seeing_innocent_suffering" | "innocent_suffering" => {
            Ok(EmotionalTriggerKind::SeeingInnocentSuffering)
        }
        _ => Err(to_py_err(format!("Unknown emotional trigger '{raw}'"))),
    }
}

/// Evaluates [`evaluate_combat_motivation_modifier`] (`§3.2`) for an NPC/PC Psyche profile.
#[pyfunction]
#[pyo3(signature = (target_psyche, motivation_kind, command_intact=true))]
pub fn evaluate_combat_motivation(
    py: Python<'_>,
    target_psyche: PyJsonInput<'_>,
    motivation_kind: &str,
    command_intact: bool,
) -> PyResult<PyJsonDict> {
    let psyche = parse_psyche_profile_value(&py_to_json_value(&target_psyche, "target_psyche")?)?;
    let kind = parse_combat_motivation_kind(motivation_kind, command_intact);
    to_py_object(py, &evaluate_combat_motivation_modifier(&psyche, kind))
}

/// Evaluates a `DIS` Composure check against a canonical [`EmotionalTriggerKind`] (`§3.3`).
#[pyfunction]
#[pyo3(signature = (target_psyche, dis, current_morale, current_delta_int, trigger_kind, wound_tier=1, roll_pairs=None))]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_emotional_trigger(
    py: Python<'_>,
    target_psyche: PyJsonInput<'_>,
    dis: i32,
    current_morale: PyEnum<MoraleState>,
    current_delta_int: i32,
    trigger_kind: &str,
    wound_tier: i32,
    roll_pairs: Option<PyListInput<'_>>,
) -> PyResult<PyJsonDict> {
    let psyche = parse_psyche_profile_value(&py_to_json_value(&target_psyche, "target_psyche")?)?;
    let trigger = parse_emotional_trigger_kind(trigger_kind, wound_tier)?;
    let pairs = opt_from_py_object::<Vec<(i32, i32)>>(roll_pairs, "roll_pairs")?
        .unwrap_or_else(|| vec![(11, 10)]);
    to_py_object(
        py,
        &satya_engine::evaluate_emotional_trigger(
            &psyche,
            dis,
            current_morale.0,
            current_delta_int,
            trigger,
            &pairs,
        ),
    )
}

/// Evaluates a Contested Combat Morale Roll (`§3.4`) combining motivation & battlefield penalties.
#[pyfunction]
#[pyo3(signature = (target_psyche, dis, motivation_kind, command_intact, delta_int, situation, is_veteran_or_sere, current_morale, player_pairs, gm_pair))]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_combat_morale_roll(
    py: Python<'_>,
    target_psyche: PyJsonInput<'_>,
    dis: i32,
    motivation_kind: &str,
    command_intact: bool,
    delta_int: i32,
    situation: PyJsonInput<'_>,
    is_veteran_or_sere: bool,
    current_morale: PyEnum<MoraleState>,
    player_pairs: PyListInput<'_>,
    gm_pair: PyListInput<'_>,
) -> PyResult<PyJsonDict> {
    let psyche = parse_psyche_profile_value(&py_to_json_value(&target_psyche, "target_psyche")?)?;
    let mot = parse_combat_motivation_kind(motivation_kind, command_intact);
    let mot_mod = evaluate_combat_motivation_modifier(&psyche, mot);
    let sit_parsed: BattlefieldMoraleSituation =
        opt_from_py_object::<BattlefieldMoraleSituation>(Some(situation), "situation")?
            .unwrap_or_default();
    let delta_morale = compute_battlefield_morale_delta(&sit_parsed, psyche.cnf)
        + mot_mod.command_breakdown_morale_shock;
    let p_pairs: Vec<(i32, i32)> = from_py_object(&player_pairs, "player_pairs")?;
    let g_pair: (i32, i32) = from_py_object(&gm_pair, "gm_pair")?;
    let roll_outcome = satya_engine::evaluate_combat_morale_roll(
        dis,
        mot_mod.delta_mot,
        delta_int,
        delta_morale,
        is_veteran_or_sere,
        current_morale.0,
        &p_pairs,
        g_pair,
    );
    to_py_object(
        py,
        &json!({
            "delta_mot": mot_mod.delta_mot,
            "delta_morale": delta_morale,
            "command_breakdown_morale_shock": mot_mod.command_breakdown_morale_shock,
            "conviction_surge_pool_bonus": mot_mod.conviction_surge_pool_bonus,
            "roll_outcome": roll_outcome,
        }),
    )
}

#[derive(Serialize)]
struct PsycheTraitDescriptor {
    code: &'static str,
    label: &'static str,
    low_pole: &'static str,
    high_pole: &'static str,
}

/// Returns the canonical 9-trait Psyche descriptor catalog (`code`, `label`, `low_pole`, `high_pole`).
#[pyfunction]
pub fn list_psyche_trait_descriptors(py: Python<'_>) -> PyResult<PyJsonList> {
    let rows: Vec<PsycheTraitDescriptor> = PsycheTraitId::ALL
        .iter()
        .map(|trait_id| {
            let (low_pole, high_pole) = trait_id.pole_hints();
            PsycheTraitDescriptor {
                code: trait_id.code(),
                label: trait_id.label(),
                low_pole,
                high_pole,
            }
        })
        .collect();
    to_py_object(py, &rows)
}

fn normalize_lifepath_config_for_py(mut lp_val: Value) -> PyResult<LifepathBuildConfig> {
    if let Some(obj) = lp_val.as_object_mut()
        && let Some(Value::Array(raw_terms)) = obj.get_mut("career_terms")
    {
        let mut normalized_terms = Vec::with_capacity(raw_terms.len());
        for entry in raw_terms.iter() {
            match entry {
                Value::String(code) => {
                    let track: CareerTermTrack = parse_coded_enum(code, "CareerTermTrack")?;
                    let term = CareerTermConfig {
                        track,
                        ..CareerTermConfig::default()
                    };
                    normalized_terms.push(serde_json::to_value(term).map_err(|e| {
                        to_py_err(format!("CareerTermConfig serialize failed: {e}"))
                    })?);
                }
                Value::Object(map) => {
                    let mut term_obj = map.clone();
                    if let Some(Value::String(trk_str)) = term_obj.get("track") {
                        let trk: CareerTermTrack = parse_coded_enum(trk_str, "CareerTermTrack")?;
                        term_obj.insert(
                            "track".to_string(),
                            serde_json::to_value(trk).map_err(|e| {
                                to_py_err(format!("CareerTermTrack serialize failed: {e}"))
                            })?,
                        );
                    }
                    if let Some(Value::String(ech_str)) = term_obj.get("echelon_override") {
                        let ech: CareerEchelonTier =
                            parse_coded_enum(ech_str, "CareerEchelonTier")?;
                        term_obj.insert(
                            "echelon_override".to_string(),
                            serde_json::to_value(ech).map_err(|e| {
                                to_py_err(format!("CareerEchelonTier serialize failed: {e}"))
                            })?,
                        );
                    }
                    let merged: CareerTermConfig = merge_object_defaults(
                        Some(&Value::Object(term_obj)),
                        &[],
                        "CareerTermConfig",
                    )?;
                    normalized_terms.push(serde_json::to_value(merged).map_err(|e| {
                        to_py_err(format!("CareerTermConfig serialize failed: {e}"))
                    })?);
                }
                other => normalized_terms.push(other.clone()),
            }
        }
        obj.insert("career_terms".to_string(), Value::Array(normalized_terms));
    }
    merge_object_defaults(Some(&lp_val), &[], "LifepathBuildConfig")
}

/// Evaluates how speaker and target Institutional Status (`1..=10`) and Street Cred (`0..=5`)
/// modify a Social Combat or persuasion check in `arena`, filtered through target Psyche (`CNF`, `EGO`, `TRS`).
#[pyfunction]
#[pyo3(signature = (speaker_institutional_status, speaker_street_cred, target_institutional_status, target_street_cred, target_psyche, arena = "INSTITUTIONAL_FORMAL"))]
pub fn evaluate_social_standing_leverage(
    py: Python<'_>,
    speaker_institutional_status: u8,
    speaker_street_cred: u8,
    target_institutional_status: u8,
    target_street_cred: u8,
    target_psyche: PyJsonInput<'_>,
    arena: &str,
) -> PyResult<PyJsonDict> {
    let arena_kind: SocialArenaKind = parse_coded_enum(arena, "SocialArenaKind")?;
    let psyche_val = py_to_json_value(&target_psyche, "target_psyche")?;
    let psyche = parse_psyche_profile_value(&psyche_val)?;
    let eval = evaluate_social_standing_modifier(
        speaker_institutional_status,
        speaker_street_cred,
        target_institutional_status,
        target_street_cred,
        &psyche,
        arena_kind,
    );
    to_py_object(py, &eval)
}

/// Computes the chronological Lifepath Socio-Economic profile, Career Entry/Echelon evaluations,
/// SQM, Operational Heat, Earned Contacts, and stage history from a partial `LifepathBuildConfig` dict.
#[pyfunction]
#[pyo3(signature = (lifepath_config, characteristics = None, aptitudes = None))]
pub fn compute_lifepath_socioeconomic_profile(
    py: Python<'_>,
    lifepath_config: PyJsonInput<'_>,
    characteristics: Option<PyJsonInput<'_>>,
    aptitudes: Option<PyJsonInput<'_>>,
) -> PyResult<PyJsonDict> {
    let lp_val = py_to_json_value(&lifepath_config, "lifepath_config")?;
    let cfg = normalize_lifepath_config_for_py(lp_val)?;
    let chars_val = characteristics
        .map(|c| py_to_json_value(&c, "characteristics"))
        .transpose()?;
    let chars: CharacterCharacteristics = merge_object_defaults(
        chars_val.as_ref(),
        &[
            ("for", "for_stat"),
            ("str", "for_stat"),
            ("height_m", "hgt_m"),
            ("mass_kg", "mas_kg"),
            ("social_savvy", "sav"),
            ("bearing", "bea"),
            ("poise", "bea"),
            ("warmth", "wrm"),
        ],
        "CharacterCharacteristics",
    )?;
    let apts_val = aptitudes
        .map(|a| py_to_json_value(&a, "aptitudes"))
        .transpose()?;
    let apts: CharacterAptitudes =
        merge_object_defaults(apts_val.as_ref(), &[], "CharacterAptitudes")?;

    let baseline = compute_lifepath_baseline_from_config(&cfg, &chars, &apts);
    to_py_object(
        py,
        &json!({
            "age_years": baseline.age_years,
            "origin_class": baseline.origin.class,
            "origin_label": baseline.origin.label,
            "campaign_sqm": baseline.campaign_sqm,
            "operational_heat": baseline.operational_heat,
            "socioeconomic": baseline.socioeconomic,
            "term_entry_evals": baseline.term_entry_evals,
            "stage_history": baseline.stage_history,
        }),
    )
}

/// Registers social combat, psyche, and world-clock helpers on the `satya_py` module.
pub fn register_psyche_social_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(evaluate_empathy_read, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_social_exchange, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_social_standing_leverage, m)?)?;
    m.add_function(wrap_pyfunction!(compute_lifepath_socioeconomic_profile, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_cognitive_dissonance, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_conviction_surge, m)?)?;
    m.add_function(wrap_pyfunction!(apply_psyche_drift_mark, m)?)?;
    m.add_function(wrap_pyfunction!(seed_or_update_world_ledger, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_combat_motivation, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_emotional_trigger, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_combat_morale_roll, m)?)?;
    m.add_function(wrap_pyfunction!(list_psyche_trait_descriptors, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::py_bridge::py_to_value;
    use pyo3::types::PyString;

    #[test]
    fn test_psyche_social_and_world_clock_py_roundtrip() {
        Python::initialize();
        Python::attach(|py| {
            let py_s = |s: &str| PyString::new(py, s).into_any();
            let npc_psyche = r#"{"opn": 16, "slf": 5, "blg": 16, "agg": 7, "ego": 10, "cnf": 14, "ext": 11, "trs": 6, "cps": 16}"#;

            let emp = evaluate_empathy_read(py, py_s(npc_psyche), 2).unwrap();
            let emp_val = py_to_value(emp.bind(py)).unwrap();
            assert_eq!(emp_val["revealed_traits"].as_array().unwrap().len(), 2);
            assert_eq!(emp_val["detects_psyche_wedge_tension"], true);

            let emp3 = evaluate_empathy_read(py, py_s(npc_psyche), 3).unwrap();
            assert_eq!(
                py_to_value(emp3.bind(py)).unwrap()["revealed_traits"]
                    .as_array()
                    .unwrap()
                    .len(),
                9
            );

            let ex = evaluate_social_exchange(
                py,
                py_s(npc_psyche),
                py_s(r#"["group_loyalty", "break_law_orders"]"#),
                2,
                None,
                12,
                12,
                None,
            )
            .unwrap();
            let ex_val = py_to_value(ex.bind(py)).unwrap();
            assert_eq!(ex_val["argument_eval"]["triggers_dual_psyche_wedge"], true);
            assert_eq!(ex_val["dissonance_stacks_added"], 2);

            let ex_9axis = evaluate_social_exchange(
                py,
                py_s(npc_psyche),
                py_s(r#"["moral_sanctity", "transparency_good_faith"]"#),
                2,
                None,
                12,
                12,
                None,
            )
            .unwrap();
            assert_eq!(
                py_to_value(ex_9axis.bind(py)).unwrap()["argument_eval"]
                    ["triggers_dual_psyche_wedge"],
                true
            );

            let ex_pascal = evaluate_social_exchange(
                py,
                py_s(npc_psyche),
                py_s(
                    r#"["TransparencyVulnerabilityGoodFaith (with verifiable audit proof)", "MoralSanctityWitnessedSufferingMercy", "EmpiricalEvidenceLogicNovelIdeas"]"#,
                ),
                2,
                None,
                12,
                12,
                None,
            )
            .unwrap();
            let ex_pascal_val = py_to_value(ex_pascal.bind(py)).unwrap();
            assert_eq!(ex_pascal_val["argument_eval"]["resonance_delta_ext"], -16);
            assert_eq!(ex_pascal_val["argument_eval"]["clash_delta_ext"], 0);

            let ledger = seed_or_update_world_ledger(
                py,
                "sess-py-wc",
                None,
                Some(py_s(
                    r#"{"economic_tier":"LUXURY_MANSION","incident_severity":"loud_firefight","faction_id":"daise","clock_delta":2}"#,
                )),
                Some("awakened"),
                None,
            )
            .unwrap();
            let ledger_val = py_to_value(ledger.bind(py)).unwrap();
            assert_eq!(ledger_val["ledger"]["operational_heat"], 4);
            assert!(!ledger_val["triggered_events"].as_array().unwrap().is_empty());
        });
    }

    #[test]
    fn test_combat_motivation_triggers_and_morale_roll_roundtrip() {
        Python::initialize();
        Python::attach(|py| {
            let py_s = |s: &str| PyString::new(py, s).into_any();
            let high_trust_compassionate = r#"{"trs": 16, "cps": 16, "blg": 12, "cnf": 14}"#;
            let hardened_low_trust = r#"{"trs": 4, "cps": 6, "blg": 8, "cnf": 10}"#;

            let mot = evaluate_combat_motivation(
                py,
                py_s(high_trust_compassionate),
                "shielding_innocents_sanctity_of_life",
                true,
            )
            .unwrap();
            assert_eq!(py_to_value(mot.bind(py)).unwrap()["delta_mot"], 2);

            let betray_high = evaluate_emotional_trigger(
                py,
                py_s(high_trust_compassionate),
                10,
                PyEnum(MoraleState::Steady),
                0,
                "betrayed_by_trusted_ally",
                1,
                Some(py_s("[[5, 5]]")),
            )
            .unwrap();
            let bh_val = py_to_value(betray_high.bind(py)).unwrap();
            assert_eq!(bh_val["susceptible"], true);
            assert_eq!(bh_val["new_morale_state"], "wavering");
            assert_eq!(bh_val["delta_int_inflicted"], 2);

            let betray_low = evaluate_emotional_trigger(
                py,
                py_s(hardened_low_trust),
                10,
                PyEnum(MoraleState::Steady),
                0,
                "betrayed_by_trusted_ally",
                1,
                Some(py_s("[[5, 5]]")),
            )
            .unwrap();
            assert_eq!(py_to_value(betray_low.bind(py)).unwrap()["susceptible"], false);

            let mercy_trig = evaluate_emotional_trigger(
                py,
                py_s(high_trust_compassionate),
                12,
                PyEnum(MoraleState::Rage),
                0,
                "seeing_innocent_suffering",
                1,
                None,
            )
            .unwrap();
            let mt_val = py_to_value(mercy_trig.bind(py)).unwrap();
            assert_eq!(mt_val["susceptible"], true);
            assert_eq!(mt_val["new_morale_state"], "steady");
            assert_eq!(mt_val["unlocks_protective_conviction"], true);

            let morale_roll = evaluate_combat_morale_roll(
                py,
                py_s(high_trust_compassionate),
                10,
                "shielding_innocents_sanctity_of_life",
                true,
                0,
                py_s(r#"{"ally_casualty_ratio": 0.6, "outnumbered_ratio": 2.5}"#),
                false,
                PyEnum(MoraleState::Steady),
                py_s("[[4, 3]]"),
                py_s("[19, 18]"),
            )
            .unwrap();
            let mr_val = py_to_value(morale_roll.bind(py)).unwrap();
            assert_eq!(mr_val["delta_mot"], 2);
            assert_eq!(mr_val["delta_morale"], 6);
            assert!(mr_val["roll_outcome"]["new_morale"].is_string());

            let descriptors = list_psyche_trait_descriptors(py).unwrap();
            let desc_val = py_to_value(descriptors.bind(py)).unwrap();
            let arr = desc_val.as_array().unwrap();
            assert_eq!(arr.len(), 9);
            assert_eq!(arr[0]["code"], "OPN");
            assert!(arr[0]["low_pole"].is_string());
            assert!(arr[0]["high_pole"].is_string());

            let lev_inst = evaluate_social_standing_leverage(
                py,
                9,
                0,
                4,
                1,
                py_s(r#"{"cnf": 15, "ego": 13, "trs": 10}"#),
                "INSTITUTIONAL_FORMAL",
            )
            .unwrap();
            let lev_inst_v = py_to_value(lev_inst.bind(py)).unwrap();
            assert!(lev_inst_v["delta_ext"].as_i64().unwrap() < 0);
            assert_eq!(lev_inst_v["psyche_amplified"], true);

            let lp_res = compute_lifepath_socioeconomic_profile(
                py,
                py_s(r#"{"origin_roll_2d20": 10, "career_terms": ["ORGANIZED_CRIME_SYNDICATE"]}"#),
                Some(py_s(r#"{"sav": 13, "bea": 12, "elo": 12}"#)),
                None,
            )
            .unwrap();
            let lp_v = py_to_value(lp_res.bind(py)).unwrap();
            assert!(lp_v["socioeconomic"]["street_cred"].as_u64().unwrap() >= 2);
            assert_eq!(lp_v["term_entry_evals"].as_array().unwrap().len(), 1);
        });
    }
}
