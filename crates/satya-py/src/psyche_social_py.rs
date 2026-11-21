//! PyO3 bindings for Satya's Social Combat, Psyche Friction/Drift, Conviction Surges,
//! and Faction/Heat World Clocks (`psyche_social`, `social_combat`, `world_clocks`).

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use satya_campaign::{apply_world_ledger_update, seed_default_world_ledger};
use satya_engine::{
    BattlefieldMoraleSituation, CombatMotivationKind, ConvictionPrincipleArchetype,
    DissonanceResolutionPath, EmotionalTriggerKind, MoraleState, PsycheProfile, PsycheTraitId,
    SocialArgumentVector, SocialComposureState, apply_psyche_drift_mark,
    compute_battlefield_morale_delta, evaluate_combat_morale_roll,
    evaluate_combat_motivation_modifier, evaluate_conviction_surge, evaluate_emotional_trigger,
    evaluate_empathy_read, evaluate_social_exchange_wounds, resolve_cognitive_dissonance,
};
use satya_schema::{
    EconomicTier, SessionWorldLedger, UpdateWorldLedgerRequest, WorldLedgerUpdateResponse,
    ZoningContext,
};

pub(crate) fn to_py_err(msg: impl Into<String>) -> PyErr {
    PyValueError::new_err(msg.into())
}

fn read_i32(map: &serde_json::Map<String, serde_json::Value>, keys: &[&str], default: i32) -> i32 {
    for k in keys {
        if let Some(v) = map.get(*k)
            && let Some(n) = v.as_i64()
        {
            return (n as i32).clamp(1, 20);
        }
    }
    default
}

pub(crate) fn parse_psyche_profile_json(raw: &str) -> PyResult<PsycheProfile> {
    let val: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| to_py_err(format!("Invalid psyche JSON: {e}")))?;
    let root = val
        .as_object()
        .ok_or_else(|| to_py_err("Psyche JSON must be an object"))?;
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
    let norm = snake.trim_matches('_').to_string();
    (norm, audit_hint, triage_hint)
}

fn map_vector_code(
    raw: &str,
    flags: Option<&serde_json::Map<String, serde_json::Value>>,
) -> PyResult<SocialArgumentVector> {
    let (norm, audit_hint, triage_hint) = normalize_vector_code_and_hints(raw);
    let flag = |key: &str| -> bool {
        flags
            .and_then(|m| m.get(key))
            .and_then(|v| v.as_bool())
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

fn parse_social_vectors_json(raw: &str) -> PyResult<Vec<SocialArgumentVector>> {
    let val: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| to_py_err(format!("Invalid vectors JSON: {e}")))?;
    let arr = val
        .as_array()
        .ok_or_else(|| to_py_err("Social vectors JSON must be an array"))?;
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
            return Err(to_py_err("Each vector entry must be a string or object"));
        }
    }
    Ok(out)
}

fn parse_trait_id(raw: &str) -> PyResult<PsycheTraitId> {
    PsycheTraitId::from_code_str(raw)
        .ok_or_else(|| to_py_err(format!("Unknown PsycheTraitId '{raw}'")))
}

/// Evaluates an Empathy & Read Person check (`INS/ITU` or `INS/ELO`) against a target's Psyche.
#[pyfunction]
#[pyo3(signature = (target_psyche_json, net_successes))]
pub fn evaluate_empathy_read_json(
    target_psyche_json: &str,
    net_successes: i32,
) -> PyResult<String> {
    let psyche = parse_psyche_profile_json(target_psyche_json)?;
    let res = evaluate_empathy_read(&psyche, net_successes);
    let revealed: Vec<serde_json::Value> = res
        .revealed_traits
        .iter()
        .map(|&(tid, score)| {
            serde_json::json!({
                "trait_id": serde_json::to_value(tid).unwrap_or_default(),
                "code": tid.code(),
                "label": tid.label(),
                "score": score,
            })
        })
        .collect();
    serde_json::to_string(&serde_json::json!({
        "net_successes": res.net_successes,
        "revealed_traits": revealed,
        "detects_psyche_wedge_tension": res.detects_psyche_wedge_tension,
        "leverage_delta_ext_bonus": res.leverage_delta_ext_bonus,
    }))
    .map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Evaluates a Contested Social Exchange against a target's Psyche profile and `SocialComposureState`.
#[pyfunction]
#[pyo3(signature = (
    target_psyche_json,
    vectors_json,
    net_successes,
    composure_state_json=None,
    target_dis=12,
    target_foc=12,
    ego_wound_dis_check_passed=None
))]
pub fn evaluate_social_exchange_json(
    target_psyche_json: &str,
    vectors_json: &str,
    net_successes: i32,
    composure_state_json: Option<&str>,
    target_dis: i32,
    target_foc: i32,
    ego_wound_dis_check_passed: Option<bool>,
) -> PyResult<String> {
    let psyche = parse_psyche_profile_json(target_psyche_json)?;
    let vectors = parse_social_vectors_json(vectors_json)?;
    let state: SocialComposureState = match composure_state_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map_err(|e| to_py_err(format!("Invalid composure_state_json: {e}")))?,
        _ => SocialComposureState::new(target_dis, target_foc),
    };
    let outcome = evaluate_social_exchange_wounds(
        &psyche,
        &state,
        &vectors,
        net_successes,
        ego_wound_dis_check_passed,
    );
    serde_json::to_string(&outcome).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Resolves active Cognitive Dissonance via one of the 4 canonical Dissonance Resolution Paths.
#[pyfunction]
#[pyo3(signature = (
    composure_state_json,
    dis=12,
    resolution_path="compartmentalization",
    violated_trait=None,
    dis_check_successes=1
))]
pub fn resolve_cognitive_dissonance_json(
    composure_state_json: &str,
    dis: i32,
    resolution_path: &str,
    violated_trait: Option<&str>,
    dis_check_successes: i32,
) -> PyResult<String> {
    let state: SocialComposureState = serde_json::from_str(composure_state_json)
        .map_err(|e| to_py_err(format!("Invalid composure_state_json: {e}")))?;
    let path_norm = resolution_path
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_");
    let path = match path_norm.as_str() {
        "conversion_defection" | "conversion" | "defection" => {
            DissonanceResolutionPath::ConversionDefection {
                violated_trait: parse_trait_id(violated_trait.unwrap_or("OPN"))?,
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
    let outcome = resolve_cognitive_dissonance(&state, dis, path);
    serde_json::to_string(&outcome).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Evaluates whether a character triggers a Conviction Surge under severe internal strain.
#[pyfunction]
#[pyo3(signature = (psyche_json, current_delta_int, already_used_this_scene=false, principle=None))]
pub fn evaluate_conviction_surge_json(
    psyche_json: &str,
    current_delta_int: i32,
    already_used_this_scene: bool,
    principle: Option<&str>,
) -> PyResult<String> {
    let psyche = parse_psyche_profile_json(psyche_json)?;
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
                parse_trait_id(raw)?,
            )),
        },
    };
    let res = evaluate_conviction_surge(
        &psyche,
        current_delta_int,
        already_used_this_scene,
        parsed_principle,
    );
    serde_json::to_string(&res).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Applies Psyche Drift Marks toward `target_pole_score` (default `10` equilibrium).
#[pyfunction]
#[pyo3(signature = (trait_score, current_marks=0, marks_added=1, dis=12, target_pole_score=None))]
pub fn apply_psyche_drift_mark_json(
    trait_score: i32,
    current_marks: i32,
    marks_added: i32,
    dis: i32,
    target_pole_score: Option<i32>,
) -> PyResult<String> {
    let res = apply_psyche_drift_mark(
        trait_score,
        current_marks,
        marks_added,
        dis,
        target_pole_score,
    );
    serde_json::to_string(&res).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Seeds or updates a [`SessionWorldLedger`] (faction clocks + `Heat` x [`EconomicTier`]) as JSON.
#[pyfunction]
#[pyo3(signature = (session_id, current_ledger_json=None, update_req_json=None, pack_id=None, campaign_tag=None))]
pub fn seed_or_update_world_ledger_json(
    session_id: &str,
    current_ledger_json: Option<&str>,
    update_req_json: Option<&str>,
    pack_id: Option<&str>,
    campaign_tag: Option<&str>,
) -> PyResult<String> {
    let update_req: Option<UpdateWorldLedgerRequest> = match update_req_json {
        Some(raw) if !raw.trim().is_empty() => Some(
            serde_json::from_str(raw)
                .map_err(|e| to_py_err(format!("Invalid update_req_json: {e}")))?,
        ),
        _ => None,
    };

    let mut ledger: SessionWorldLedger = match current_ledger_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map_err(|e| to_py_err(format!("Invalid current_ledger_json: {e}")))?,
        _ => {
            let eff_pack = pack_id
                .or_else(|| update_req.as_ref().and_then(|r| r.pack_id.as_deref()))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("awakened");
            let tier = update_req
                .as_ref()
                .and_then(|r| r.economic_tier)
                .unwrap_or(EconomicTier::MiddleClass);
            let zoning = update_req
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
        }
    };

    let triggered_events = if let Some(ref req) = update_req {
        apply_world_ledger_update(&mut ledger, req)
    } else {
        Vec::new()
    };

    serde_json::to_string(&WorldLedgerUpdateResponse {
        ledger,
        triggered_events,
    })
    .map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

fn parse_morale_state_code(raw: &str) -> MoraleState {
    MoraleState::from_code_str(raw).unwrap_or(MoraleState::Steady)
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

fn parse_roll_pairs(raw: Option<&str>, fallback: Vec<(i32, i32)>) -> PyResult<Vec<(i32, i32)>> {
    match raw {
        Some(s) if !s.trim().is_empty() => serde_json::from_str::<Vec<(i32, i32)>>(s)
            .map_err(|e| to_py_err(format!("Invalid roll_pairs JSON: {e}"))),
        _ => Ok(fallback),
    }
}

fn parse_battlefield_morale_situation(raw: &str) -> PyResult<BattlefieldMoraleSituation> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(BattlefieldMoraleSituation::default());
    }
    let val: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| to_py_err(format!("Invalid situation JSON: {e}")))?;
    let mut sit = BattlefieldMoraleSituation::default();
    if let Some(obj) = val.as_object() {
        if let Some(v) = obj.get("ally_casualty_ratio").and_then(|v| v.as_f64()) {
            sit.ally_casualty_ratio = v;
        }
        if let Some(v) = obj.get("outnumbered_ratio").and_then(|v| v.as_f64()) {
            sit.outnumbered_ratio = v;
        }
        if let Some(v) = obj
            .get("leader_incapacitated_or_comms_cut")
            .and_then(|v| v.as_bool())
        {
            sit.leader_incapacitated_or_comms_cut = v;
        }
        if let Some(v) = obj
            .get("position_overrun_flanked_or_ambushed")
            .and_then(|v| v.as_bool())
        {
            sit.position_overrun_flanked_or_ambushed = v;
        }
        if let Some(v) = obj.get("ammo_fraction_remaining").and_then(|v| v.as_f64()) {
            sit.ammo_fraction_remaining = v;
        }
        if let Some(v) = obj.get("tracks_ammo").and_then(|v| v.as_bool()) {
            sit.tracks_ammo = v;
        }
        if let Some(v) = obj
            .get("primary_weapon_compromised")
            .and_then(|v| v.as_bool())
        {
            sit.primary_weapon_compromised = v;
        }
        if let Some(v) = obj
            .get("attacks_ineffective_against_heavy_armor")
            .and_then(|v| v.as_bool())
        {
            sit.attacks_ineffective_against_heavy_armor = v;
        }
        if let Some(v) = obj
            .get("started_fatigued_or_sleep_deprived")
            .and_then(|v| v.as_bool())
        {
            sit.started_fatigued_or_sleep_deprived = v;
        }
        if let Some(v) = obj
            .get("additional_suppression_or_shock_delta")
            .and_then(|v| v.as_i64())
        {
            sit.additional_suppression_or_shock_delta = v as i32;
        }
    }
    Ok(sit)
}

/// Evaluates [`evaluate_combat_motivation_modifier`] (`§3.2`) for an NPC/PC Psyche profile.
#[pyfunction]
#[pyo3(signature = (target_psyche_json, motivation_kind, command_intact=true))]
pub fn evaluate_combat_motivation_json(
    target_psyche_json: &str,
    motivation_kind: &str,
    command_intact: bool,
) -> PyResult<String> {
    let psyche = parse_psyche_profile_json(target_psyche_json)?;
    let kind = parse_combat_motivation_kind(motivation_kind, command_intact);
    let mod_res = evaluate_combat_motivation_modifier(&psyche, kind);
    serde_json::to_string(&mod_res).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Evaluates a `DIS` Composure check against a canonical [`EmotionalTriggerKind`] (`§3.3`).
#[pyfunction]
#[pyo3(signature = (target_psyche_json, dis, current_morale, current_delta_int, trigger_kind, wound_tier=1, roll_pairs_json=None))]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_emotional_trigger_json(
    target_psyche_json: &str,
    dis: i32,
    current_morale: &str,
    current_delta_int: i32,
    trigger_kind: &str,
    wound_tier: i32,
    roll_pairs_json: Option<&str>,
) -> PyResult<String> {
    let psyche = parse_psyche_profile_json(target_psyche_json)?;
    let morale = parse_morale_state_code(current_morale);
    let trigger = parse_emotional_trigger_kind(trigger_kind, wound_tier)?;
    let pairs = parse_roll_pairs(roll_pairs_json, vec![(11, 10)])?;
    let outcome =
        evaluate_emotional_trigger(&psyche, dis, morale, current_delta_int, trigger, &pairs);
    serde_json::to_string(&outcome).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Evaluates a Contested Combat Morale Roll (`§3.4`) combining motivation & battlefield penalties.
#[pyfunction]
#[pyo3(signature = (target_psyche_json, dis, motivation_kind, command_intact, delta_int, situation_json, is_veteran_or_sere, current_morale, player_pairs_json, gm_pair_json))]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_combat_morale_roll_json(
    target_psyche_json: &str,
    dis: i32,
    motivation_kind: &str,
    command_intact: bool,
    delta_int: i32,
    situation_json: &str,
    is_veteran_or_sere: bool,
    current_morale: &str,
    player_pairs_json: &str,
    gm_pair_json: &str,
) -> PyResult<String> {
    let psyche = parse_psyche_profile_json(target_psyche_json)?;
    let mot = parse_combat_motivation_kind(motivation_kind, command_intact);
    let mot_mod = evaluate_combat_motivation_modifier(&psyche, mot);
    let sit = parse_battlefield_morale_situation(situation_json)?;
    let delta_morale =
        compute_battlefield_morale_delta(&sit, psyche.cnf) + mot_mod.command_breakdown_morale_shock;
    let morale = parse_morale_state_code(current_morale);
    let player_pairs: Vec<(i32, i32)> = serde_json::from_str(player_pairs_json)
        .map_err(|e| to_py_err(format!("Invalid player_pairs JSON: {e}")))?;
    let gm_pair: (i32, i32) = serde_json::from_str(gm_pair_json)
        .map_err(|e| to_py_err(format!("Invalid gm_pair JSON: {e}")))?;

    let roll_outcome = evaluate_combat_morale_roll(
        dis,
        mot_mod.delta_mot,
        delta_int,
        delta_morale,
        is_veteran_or_sere,
        morale,
        &player_pairs,
        gm_pair,
    );
    let payload = serde_json::json!({
        "delta_mot": mot_mod.delta_mot,
        "delta_morale": delta_morale,
        "command_breakdown_morale_shock": mot_mod.command_breakdown_morale_shock,
        "conviction_surge_pool_bonus": mot_mod.conviction_surge_pool_bonus,
        "roll_outcome": roll_outcome,
    });
    serde_json::to_string(&payload).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Registers social combat, psyche, and world-clock helpers on the `satya_py` module.
pub fn register_psyche_social_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(evaluate_empathy_read_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_social_exchange_json, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_cognitive_dissonance_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_conviction_surge_json, m)?)?;
    m.add_function(wrap_pyfunction!(apply_psyche_drift_mark_json, m)?)?;
    m.add_function(wrap_pyfunction!(seed_or_update_world_ledger_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_combat_motivation_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_emotional_trigger_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_combat_morale_roll_json, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psyche_social_and_world_clock_py_roundtrip() {
        let npc_psyche = r#"{"opn": 16, "slf": 5, "blg": 16, "agg": 7, "ego": 10, "cnf": 14, "ext": 11, "trs": 6, "cps": 16}"#;
        let emp_json = evaluate_empathy_read_json(npc_psyche, 2).unwrap();
        let emp_val: serde_json::Value = serde_json::from_str(&emp_json).unwrap();
        assert_eq!(emp_val["revealed_traits"].as_array().unwrap().len(), 2);
        assert_eq!(emp_val["detects_psyche_wedge_tension"], true);

        let emp3_json = evaluate_empathy_read_json(npc_psyche, 3).unwrap();
        let emp3_val: serde_json::Value = serde_json::from_str(&emp3_json).unwrap();
        assert_eq!(emp3_val["revealed_traits"].as_array().unwrap().len(), 9);

        let ex_json = evaluate_social_exchange_json(
            npc_psyche,
            r#"["group_loyalty", "break_law_orders"]"#,
            2,
            None,
            12,
            12,
            None,
        )
        .unwrap();
        let ex_val: serde_json::Value = serde_json::from_str(&ex_json).unwrap();
        assert_eq!(ex_val["argument_eval"]["triggers_dual_psyche_wedge"], true);
        assert_eq!(ex_val["dissonance_stacks_added"], 2);

        let ex_9axis_json = evaluate_social_exchange_json(
            npc_psyche,
            r#"["moral_sanctity", "transparency_good_faith"]"#,
            2,
            None,
            12,
            12,
            None,
        )
        .unwrap();
        let ex_9axis_val: serde_json::Value = serde_json::from_str(&ex_9axis_json).unwrap();
        assert_eq!(
            ex_9axis_val["argument_eval"]["triggers_dual_psyche_wedge"],
            true
        );

        let ex_pascal_json = evaluate_social_exchange_json(
            npc_psyche,
            r#"["TransparencyVulnerabilityGoodFaith (with verifiable audit proof)", "MoralSanctityWitnessedSufferingMercy", "EmpiricalEvidenceLogicNovelIdeas"]"#,
            2,
            None,
            12,
            12,
            None,
        )
        .unwrap();
        let ex_pascal_val: serde_json::Value = serde_json::from_str(&ex_pascal_json).unwrap();
        assert_eq!(ex_pascal_val["argument_eval"]["resonance_delta_ext"], -16);
        assert_eq!(ex_pascal_val["argument_eval"]["clash_delta_ext"], 0);

        let ledger_json = seed_or_update_world_ledger_json(
            "sess-py-wc",
            None,
            Some(r#"{"economic_tier":"LUXURY_MANSION","incident_severity":"loud_firefight","faction_id":"daise","clock_delta":2}"#),
            Some("awakened"),
            None,
        )
        .unwrap();
        let ledger_val: serde_json::Value = serde_json::from_str(&ledger_json).unwrap();
        assert_eq!(ledger_val["ledger"]["operational_heat"], 4);
        assert!(
            !ledger_val["triggered_events"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_combat_motivation_triggers_and_morale_roll_roundtrip() {
        let high_trust_compassionate = r#"{"trs": 16, "cps": 16, "blg": 12, "cnf": 14}"#;
        let hardened_low_trust = r#"{"trs": 4, "cps": 6, "blg": 8, "cnf": 10}"#;

        let mot_val: serde_json::Value = serde_json::from_str(
            &evaluate_combat_motivation_json(
                high_trust_compassionate,
                "shielding_innocents_sanctity_of_life",
                true,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(mot_val["delta_mot"], 2);

        let betray_high: serde_json::Value = serde_json::from_str(
            &evaluate_emotional_trigger_json(
                high_trust_compassionate,
                10,
                "steady",
                0,
                "betrayed_by_trusted_ally",
                1,
                Some("[[5, 5]]"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(betray_high["susceptible"], true);
        assert_eq!(betray_high["new_morale_state"], "wavering");
        assert_eq!(betray_high["delta_int_inflicted"], 2);

        let betray_low: serde_json::Value = serde_json::from_str(
            &evaluate_emotional_trigger_json(
                hardened_low_trust,
                10,
                "steady",
                0,
                "betrayed_by_trusted_ally",
                1,
                Some("[[5, 5]]"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(betray_low["susceptible"], false);

        let mercy_trig: serde_json::Value = serde_json::from_str(
            &evaluate_emotional_trigger_json(
                high_trust_compassionate,
                12,
                "rage",
                0,
                "seeing_innocent_suffering",
                1,
                None,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(mercy_trig["susceptible"], true);
        assert_eq!(mercy_trig["new_morale_state"], "steady");
        assert_eq!(mercy_trig["unlocks_protective_conviction"], true);

        let morale_roll: serde_json::Value = serde_json::from_str(
            &evaluate_combat_morale_roll_json(
                high_trust_compassionate,
                10,
                "shielding_innocents_sanctity_of_life",
                true,
                0,
                r#"{"ally_casualty_ratio": 0.6, "outnumbered_ratio": 2.5}"#,
                false,
                "steady",
                "[[4, 3]]",
                "[19, 18]",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(morale_roll["delta_mot"], 2);
        assert_eq!(morale_roll["delta_morale"], 6);
        assert!(morale_roll["roll_outcome"]["new_morale"].is_string());
    }
}
