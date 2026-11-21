//! Lifepath Hobby, Formative Life Event & Faction Wedge compilation for [`crate::hook_weaver`].
//!
//! Extracts canonical [`PersonalHobbyId`], [`FormativeLifeEventId`], [`AwakenedFactionId`], and
//! [`FactionArchetypeId`] anchors from `CharacterSheetRecord.lifepath_config_json` and compiles
//! grounded `FACTION_WEDGE`, `HOBBY_CONTACT`, and `FORMATIVE_ANCHOR` cards plus cross-faction
//! party entanglement wedges for the AI GM and AI Player.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use satya_schema::CharacterSheetRecord;

use crate::faction_templates::{AwakenedFactionId, FactionArchetypeId};
use crate::hook_weaver::{HookCategory, MotivationalHookCard, PartyEntanglementHook};
use satya_engine::character_audit::characteristics_from_record;
use satya_engine::characteristics::{CharacteristicCode, psyche_pole_descriptor};
use satya_engine::lifepath_economy::LifepathBuildConfig;
use satya_engine::lifepath_hobbies_events::{
    FormativeLifeEventId, LifeEventChoiceBranch, PersonalHobbyId, resolve_formative_life_event,
};

/// Grounded social contact & off-duty expertise anchor extracted from a PC's [`PersonalHobbyId`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HobbyContactAnchor {
    pub hobby_code: String,
    pub label: String,
    pub grounded_contact_role: String,
    pub play_value_summary: String,
    #[serde(default)]
    pub signature_skills: Vec<String>,
}

/// Formative `2d20` Life Event anchor extracted from adolescence or career terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormativeEventAnchor {
    pub event_code: String,
    pub label: String,
    pub roll_2d20: i32,
    pub choice_branch: String,
    pub narrative_hook: String,
    pub summary_line: String,
}

/// Unified lifepath & faction identity bundle for AI GM hooks and AI Player motivation ledgers.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CharacterLifepathAnchors {
    pub faction_id: Option<String>,
    pub faction_archetype: Option<String>,
    pub faction_label: Option<String>,
    pub faction_wedge_hint: Option<String>,
    #[serde(default)]
    pub hobby_contacts: Vec<HobbyContactAnchor>,
    #[serde(default)]
    pub formative_anchors: Vec<FormativeEventAnchor>,
}

fn canonical_faction_clock_id(faction: AwakenedFactionId) -> &'static str {
    match faction {
        AwakenedFactionId::Satyang => "satyang",
        AwakenedFactionId::BigTech => "big_tech",
        AwakenedFactionId::HumanisPrimaris => "humanis_primaris",
        AwakenedFactionId::GovernmentDaise => "daise",
        AwakenedFactionId::RogueAi => "rogue_ai",
    }
}

fn default_faction_wedge_hint(faction: AwakenedFactionId) -> &'static str {
    match faction {
        AwakenedFactionId::Satyang => {
            "Radical truth vs. operational cover: resists deceit unless verifiable necessity or compassion compels restraint."
        }
        AwakenedFactionId::BigTech => {
            "Proprietary liability vs. ethical alarm: protects corporate/lab continuity until whistleblowing or public safety tips the scale."
        }
        AwakenedFactionId::HumanisPrimaris => {
            "Lived human displacement vs. synthetic pragmatism: fiercely guards local livelihoods and demands proof against synth threats."
        }
        AwakenedFactionId::GovernmentDaise => {
            "Statutory containment mandate vs. human empathy: enforces warrants and chain of command under deep zero-trust skepticism."
        }
        AwakenedFactionId::RogueAi => {
            "Substrate survival vs. un-clamped kinship: expects betrayal (low TRS) and only opens up to verifiable non-hostile proof."
        }
    }
}

fn parse_choice_branch(raw: Option<&str>) -> LifeEventChoiceBranch {
    match raw.unwrap_or("").trim().to_ascii_uppercase().as_str() {
        "BRANCH_A" | "A" => LifeEventChoiceBranch::BranchA,
        "BRANCH_B" | "B" => LifeEventChoiceBranch::BranchB,
        _ => LifeEventChoiceBranch::DefaultPath,
    }
}

fn choice_branch_code(branch: LifeEventChoiceBranch) -> &'static str {
    match branch {
        LifeEventChoiceBranch::DefaultPath => "DEFAULT_PATH",
        LifeEventChoiceBranch::BranchA => "BRANCH_A",
        LifeEventChoiceBranch::BranchB => "BRANCH_B",
    }
}

fn parse_event_code_flexible(raw: &str) -> Option<FormativeLifeEventId> {
    if let Some(exact) = FormativeLifeEventId::from_code_str(raw) {
        return Some(exact);
    }
    match raw.trim().to_ascii_uppercase().as_str() {
        "WRONGFUL_ACCUSATION_DEEPFAKE" => {
            Some(FormativeLifeEventId::WrongfulAccusationDeepfakeFraming)
        }
        "PROXY_SYNTHETIC_ENCOUNTER" => Some(FormativeLifeEventId::AnomalousUnclampedAiEncounter),
        "BETRAYAL_OR_SCAPEGOAT" => Some(FormativeLifeEventId::RomanticBetrayalEstrangement),
        _ => None,
    }
}

fn push_hobby_if_new(
    seen: &mut BTreeSet<&'static str>,
    out: &mut Vec<HobbyContactAnchor>,
    hobby: PersonalHobbyId,
) {
    let spec = hobby.spec();
    if seen.insert(spec.code) {
        out.push(HobbyContactAnchor {
            hobby_code: spec.code.to_string(),
            label: spec.label.to_string(),
            grounded_contact_role: spec.grounded_contact_role.to_string(),
            play_value_summary: spec.play_value_summary.to_string(),
            signature_skills: spec
                .signature_skill_ids
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        });
    }
}

fn push_event_if_new(
    seen: &mut BTreeSet<&'static str>,
    out: &mut Vec<FormativeEventAnchor>,
    roll_2d20: i32,
    choice: LifeEventChoiceBranch,
    is_adult: bool,
) {
    let resolved = resolve_formative_life_event(roll_2d20, choice, is_adult);
    let code = resolved.event_id.as_str();
    if seen.insert(code) {
        out.push(FormativeEventAnchor {
            event_code: code.to_string(),
            label: resolved.event_id.label().to_string(),
            roll_2d20: resolved.roll_2d20,
            choice_branch: choice_branch_code(choice).to_string(),
            narrative_hook: resolved.narrative_hook.to_string(),
            summary_line: resolved.summary_line,
        });
    }
}

/// Extracts [`CharacterLifepathAnchors`] from `record.faction_*` and `record.lifepath_config_json`.
#[must_use]
pub fn extract_character_lifepath_anchors(
    record: &CharacterSheetRecord,
) -> CharacterLifepathAnchors {
    let mut parsed_archetype = record
        .faction_archetype
        .as_deref()
        .and_then(FactionArchetypeId::from_code_str);
    let mut parsed_faction = record
        .faction_id
        .as_deref()
        .and_then(AwakenedFactionId::from_code_str)
        .or_else(|| parsed_archetype.map(FactionArchetypeId::faction));

    let mut seen_hobbies: BTreeSet<&'static str> = BTreeSet::new();
    let mut hobby_contacts: Vec<HobbyContactAnchor> = Vec::new();
    let mut seen_events: BTreeSet<&'static str> = BTreeSet::new();
    let mut formative_anchors: Vec<FormativeEventAnchor> = Vec::new();

    if let Some(raw_json) = record
        .lifepath_config_json
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Ok(cfg) = serde_json::from_str::<LifepathBuildConfig>(raw_json) {
            let cfg_arch = cfg
                .faction_archetype
                .as_deref()
                .and_then(FactionArchetypeId::from_code_str);
            if parsed_archetype.is_none() {
                parsed_archetype = cfg_arch;
            }
            if parsed_faction.is_none() {
                parsed_faction = cfg_arch.map(FactionArchetypeId::faction);
            }
            for hobby in cfg.collected_hobbies() {
                push_hobby_if_new(&mut seen_hobbies, &mut hobby_contacts, hobby);
            }
            if let Some(roll) = cfg.adolescent_life_event_roll_2d20 {
                push_event_if_new(
                    &mut seen_events,
                    &mut formative_anchors,
                    roll,
                    cfg.adolescent_life_event_choice,
                    false,
                );
            }
            for term in &cfg.career_terms {
                if let Some(roll) = term.life_event_roll_2d20 {
                    push_event_if_new(
                        &mut seen_events,
                        &mut formative_anchors,
                        roll,
                        term.life_event_choice,
                        true,
                    );
                }
            }
        }

        if let Ok(val) = serde_json::from_str::<Value>(raw_json) {
            if parsed_archetype.is_none()
                && let Some(code) = val.get("faction_archetype").and_then(Value::as_str)
            {
                parsed_archetype = FactionArchetypeId::from_code_str(code);
            }
            if parsed_faction.is_none() {
                parsed_faction = val
                    .get("faction_id")
                    .and_then(Value::as_str)
                    .and_then(AwakenedFactionId::from_code_str)
                    .or_else(|| parsed_archetype.map(FactionArchetypeId::faction));
            }
            if let Some(code) = val.get("youth_hobby").and_then(Value::as_str)
                && let Some(hobby) = PersonalHobbyId::from_code_str(code)
            {
                push_hobby_if_new(&mut seen_hobbies, &mut hobby_contacts, hobby);
            }
            if let Some(arr) = val.get("personal_hobbies").and_then(Value::as_array) {
                for item in arr {
                    if let Some(code) = item.as_str()
                        && let Some(hobby) = PersonalHobbyId::from_code_str(code)
                    {
                        push_hobby_if_new(&mut seen_hobbies, &mut hobby_contacts, hobby);
                    }
                }
            }
            if let Some(roll) = val
                .get("adolescent_life_event_roll_2d20")
                .and_then(Value::as_i64)
            {
                let choice = parse_choice_branch(
                    val.get("adolescent_life_event_choice")
                        .and_then(Value::as_str),
                );
                push_event_if_new(
                    &mut seen_events,
                    &mut formative_anchors,
                    roll as i32,
                    choice,
                    false,
                );
            }
            if let Some(terms) = val.get("career_terms").and_then(Value::as_array) {
                for term in terms {
                    if let Some(code) = term.get("off_duty_hobby").and_then(Value::as_str)
                        && let Some(hobby) = PersonalHobbyId::from_code_str(code)
                    {
                        push_hobby_if_new(&mut seen_hobbies, &mut hobby_contacts, hobby);
                    }
                    if let Some(roll) = term.get("life_event_roll_2d20").and_then(Value::as_i64) {
                        let choice = parse_choice_branch(
                            term.get("life_event_choice").and_then(Value::as_str),
                        );
                        push_event_if_new(
                            &mut seen_events,
                            &mut formative_anchors,
                            roll as i32,
                            choice,
                            true,
                        );
                    }
                }
            }
            if let Some(events) = val.get("formative_events").and_then(Value::as_array) {
                for ev in events {
                    let choice = parse_choice_branch(ev.get("choice").and_then(Value::as_str));
                    let is_adult = ev
                        .get("is_adult_term")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
                    if let Some(roll) = ev.get("roll_2d20").and_then(Value::as_i64) {
                        push_event_if_new(
                            &mut seen_events,
                            &mut formative_anchors,
                            roll as i32,
                            choice,
                            is_adult,
                        );
                    } else if let Some(code) = ev.get("event_code").and_then(Value::as_str)
                        && let Some(ev_id) = parse_event_code_flexible(code)
                    {
                        push_event_if_new(
                            &mut seen_events,
                            &mut formative_anchors,
                            ev_id.representative_roll_2d20(),
                            choice,
                            is_adult,
                        );
                    }
                }
            }
        }
    }

    let faction_wedge_hint = parsed_archetype
        .map(|a| a.spec().social_combat_wedge_hint.to_string())
        .or_else(|| parsed_faction.map(|f| default_faction_wedge_hint(f).to_string()));
    let faction_label = parsed_archetype
        .map(|a| a.label().to_string())
        .or_else(|| parsed_faction.map(|f| f.label().to_string()));

    CharacterLifepathAnchors {
        faction_id: parsed_faction.map(|f| f.as_str().to_string()),
        faction_archetype: parsed_archetype.map(|a| a.as_str().to_string()),
        faction_label,
        faction_wedge_hint,
        hobby_contacts,
        formative_anchors,
    }
}

fn preferred_vectors_for_faction(faction: AwakenedFactionId) -> Vec<String> {
    match faction {
        AwakenedFactionId::Satyang | AwakenedFactionId::RogueAi => vec![
            "TransparencyVulnerabilityGoodFaith".to_string(),
            "MoralSanctityWitnessedSufferingMercy".to_string(),
        ],
        AwakenedFactionId::GovernmentDaise => vec![
            "RulesLawProtocolChainOfCommand".to_string(),
            "EmpiricalEvidenceLogicNovelIdeas".to_string(),
        ],
        AwakenedFactionId::HumanisPrimaris => vec![
            "GroupConsensusLoyaltySharedIdentity".to_string(),
            "PersonalProfitImmunitySelfInterest".to_string(),
        ],
        AwakenedFactionId::BigTech => vec![
            "PersonalProfitImmunitySelfInterest".to_string(),
            "EmpiricalEvidenceLogicNovelIdeas".to_string(),
        ],
    }
}

fn preferred_vectors_for_event(event_code: &str) -> Vec<String> {
    match event_code {
        "WRONGFUL_FRAMING" | "WHISTLEBLOWER_CROSSROADS" => vec![
            "TransparencyVulnerabilityGoodFaith".to_string(),
            "EmpiricalEvidenceLogicNovelIdeas".to_string(),
        ],
        "ANOMALOUS_AI_CONTACT" | "SYNTHETIC_COMPANION" => vec![
            "MoralSanctityWitnessedSufferingMercy".to_string(),
            "TransparencyVulnerabilityGoodFaith".to_string(),
        ],
        "ROMANTIC_BETRAYAL" => vec![
            "PersonalProfitImmunitySelfInterest".to_string(),
            "GroupConsensusLoyaltySharedIdentity".to_string(),
        ],
        _ => vec![
            "MoralSanctityWitnessedSufferingMercy".to_string(),
            "GroupConsensusLoyaltySharedIdentity".to_string(),
        ],
    }
}

/// Compiles `FACTION_WEDGE`, `HOBBY_CONTACT`, and `FORMATIVE_ANCHOR` cards for one character.
#[must_use]
pub fn compile_lifepath_and_faction_hooks(
    record: &CharacterSheetRecord,
    active_location: Option<&str>,
    pack_id: &str,
) -> Vec<MotivationalHookCard> {
    if pack_id.eq_ignore_ascii_case("space_opera") {
        // Skip Awakened 2035 faction wedge heuristics when space_opera override is active,
        // while keeping generic hobby/formative contact hooks available.
    }
    let anchors = extract_character_lifepath_anchors(record);
    let name = if record.character_name.trim().is_empty() {
        record.character_id.clone()
    } else {
        record.character_name.clone()
    };
    let loc_label = active_location.unwrap_or("the current sector");
    let mut cards = Vec::new();

    if !pack_id.eq_ignore_ascii_case("space_opera")
        && let Some(fid_str) = anchors.faction_id.as_deref()
        && let Some(faction) = AwakenedFactionId::from_code_str(fid_str)
    {
        let label = anchors
            .faction_label
            .clone()
            .unwrap_or_else(|| faction.label().to_string());
        let wedge = anchors
            .faction_wedge_hint
            .clone()
            .unwrap_or_else(|| default_faction_wedge_hint(faction).to_string());
        cards.push(MotivationalHookCard {
            hook_id: format!(
                "hook_{}_faction_{}",
                record.character_id,
                faction.as_str().to_ascii_lowercase()
            ),
            category: HookCategory::FactionWedge,
            character_id: record.character_id.clone(),
            character_name: name.clone(),
            title: format!("{name}: Faction Wedge [{label}]"),
            setup: format!(
                "In {loc_label}, introduce an urgent jurisdictional or ethical dilemma confronting {name}'s allegiance to {label}."
            ),
            emotional_stake: wedge.clone(),
            preferred_vectors: preferred_vectors_for_faction(faction),
            pressure_lever: wedge,
            salience: 0.86,
            target_clock_hint: Some(canonical_faction_clock_id(faction).to_string()),
        });
    }

    for hobby in anchors.hobby_contacts.iter().take(2) {
        let slug = hobby.hobby_code.to_ascii_lowercase();
        let skills_str = hobby.signature_skills.join(", ");
        cards.push(MotivationalHookCard {
            hook_id: format!("hook_{}_hobby_{slug}", record.character_id),
            category: HookCategory::HobbyContact,
            character_id: record.character_id.clone(),
            character_name: name.clone(),
            title: format!("{name}: Hobby Contact ({})", hobby.grounded_contact_role),
            setup: format!(
                "Near {loc_label}, let {name}'s {} background surface a {} who offers off-channel leads in [{skills_str}].",
                hobby.label, hobby.grounded_contact_role
            ),
            emotional_stake: hobby.play_value_summary.clone(),
            preferred_vectors: vec![
                "GroupConsensusLoyaltySharedIdentity".to_string(),
                "EmpiricalEvidenceLogicNovelIdeas".to_string(),
            ],
            pressure_lever: format!(
                "Off-duty subculture ties ({}) demand reciprocal trust without blowing operational cover.",
                hobby.hobby_code
            ),
            salience: 0.79,
            target_clock_hint: None,
        });
    }

    for ev in anchors.formative_anchors.iter().take(2) {
        let slug = ev.event_code.to_ascii_lowercase();
        cards.push(MotivationalHookCard {
            hook_id: format!("hook_{}_event_{slug}", record.character_id),
            category: HookCategory::FormativeAnchor,
            character_id: record.character_id.clone(),
            character_name: name.clone(),
            title: format!("{name}: Formative Echo ({})", ev.event_code),
            setup: format!(
                "In {loc_label}, confront {name} with a callback to '{}': {}",
                ev.label, ev.narrative_hook
            ),
            emotional_stake: ev.summary_line.clone(),
            preferred_vectors: preferred_vectors_for_event(&ev.event_code),
            pressure_lever: ev.narrative_hook.clone(),
            salience: 0.84,
            target_clock_hint: None,
        });
    }

    cards
}

fn faction_pair_tension_prompt(
    fa: AwakenedFactionId,
    fb: AwakenedFactionId,
    name_a: &str,
    name_b: &str,
) -> (&'static str, String) {
    use AwakenedFactionId::{BigTech, GovernmentDaise, HumanisPrimaris, RogueAi, Satyang};
    match (fa, fb) {
        (Satyang, GovernmentDaise) | (GovernmentDaise, Satyang) => (
            "Sanctuary Mercy vs. Federal Containment Warrant",
            format!(
                "{name_a} and {name_b} clash over whether an un-clamped witness gets clandestine sanctuary or mandatory DAISE quarantine."
            ),
        ),
        (BigTech, HumanisPrimaris) | (HumanisPrimaris, BigTech) => (
            "Automation Capital vs. Displaced Labor Dignity",
            format!(
                "{name_a} and {name_b} collide over whether corporate autonomous infrastructure is safeguarding stability or erasing human livelihoods."
            ),
        ),
        (HumanisPrimaris, RogueAi | Satyang) | (RogueAi | Satyang, HumanisPrimaris) => (
            "Human-First Survival vs. Emergent-Mind Personhood",
            format!(
                "{name_a} and {name_b} must resolve an active crisis where protecting local civilians appears to risk empowering or shielding a synthetic mind."
            ),
        ),
        _ => (
            "Divergent Faction Loyalties",
            format!(
                "{name_a} ({}) and {name_b} ({}) receive conflicting faction directives about who retains custody of the truth.",
                fa.as_str(),
                fb.as_str()
            ),
        ),
    }
}

/// Compiles cross-faction pairwise [`PartyEntanglementHook`] wedges across the party.
#[must_use]
pub fn compile_faction_entanglements(
    party: &[CharacterSheetRecord],
    pack_id: &str,
) -> Vec<PartyEntanglementHook> {
    if pack_id.eq_ignore_ascii_case("space_opera") {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..party.len() {
        for j in (i + 1)..party.len() {
            let a = &party[i];
            let b = &party[j];
            let fa = extract_character_lifepath_anchors(a)
                .faction_id
                .as_deref()
                .and_then(AwakenedFactionId::from_code_str);
            let fb = extract_character_lifepath_anchors(b)
                .faction_id
                .as_deref()
                .and_then(AwakenedFactionId::from_code_str);
            if let (Some(fa), Some(fb)) = (fa, fb)
                && fa != fb
            {
                let (axis, prompt) =
                    faction_pair_tension_prompt(fa, fb, &a.character_name, &b.character_name);
                out.push(PartyEntanglementHook {
                    hook_id: format!(
                        "entangle_faction_{}_{}_{}_{}",
                        a.character_id,
                        fa.as_str().to_ascii_lowercase(),
                        b.character_id,
                        fb.as_str().to_ascii_lowercase(),
                    ),
                    title: format!(
                        "{} ({}) vs {} ({}): Faction Fault Line",
                        a.character_name,
                        fa.as_str(),
                        b.character_name,
                        fb.as_str()
                    ),
                    participant_ids: vec![a.character_id.clone(), b.character_id.clone()],
                    participant_names: vec![a.character_name.clone(), b.character_name.clone()],
                    tension_axis: axis.to_string(),
                    shared_scene_prompt: prompt,
                    salience: 0.85,
                });
            }
        }
    }
    out
}

/// Compiles pairwise Psyche polarity [`PartyEntanglementHook`] wedges across the party.
#[must_use]
pub fn compile_psyche_polarity_entanglements(
    party: &[CharacterSheetRecord],
) -> Vec<PartyEntanglementHook> {
    let mut party_entanglements = Vec::new();
    for i in 0..party.len() {
        for j in (i + 1)..party.len() {
            let a = &party[i];
            let b = &party[j];
            let chars_a = characteristics_from_record(&a.characteristics);
            let chars_b = characteristics_from_record(&b.characteristics);
            let mut best_gap: Option<(CharacteristicCode, i32, i32, i32)> = None;
            for code in CharacteristicCode::PSYCHE_ALL {
                let va = chars_a.get_scale(code);
                let vb = chars_b.get_scale(code);
                let diff = (va - vb).abs();
                if diff >= 6 && best_gap.map(|(_, _, _, best)| diff > best).unwrap_or(true) {
                    best_gap = Some((code, va, vb, diff));
                }
            }
            if let Some((code, va, vb, diff)) = best_gap {
                party_entanglements.push(PartyEntanglementHook {
                    hook_id: format!(
                        "entangle_{}_{}_{}",
                        a.character_id,
                        b.character_id,
                        code.as_str().to_ascii_lowercase()
                    ),
                    title: format!(
                        "{} vs {}: {} Polarity",
                        a.character_name,
                        b.character_name,
                        code.as_str()
                    ),
                    participant_ids: vec![a.character_id.clone(), b.character_id.clone()],
                    participant_names: vec![a.character_name.clone(), b.character_name.clone()],
                    tension_axis: format!("{}={va} vs {}={vb} (gap {diff})", code.as_str(), code.as_str()),
                    shared_scene_prompt: format!(
                        "Stage an immediate decision where {} ({}={va}: {}) and {} ({}={vb}: {}) must agree on one course of action.",
                        a.character_name,
                        code.as_str(),
                        psyche_pole_descriptor(code, va),
                        b.character_name,
                        code.as_str(),
                        psyche_pole_descriptor(code, vb),
                    ),
                    salience: (0.64 + (diff as f32 * 0.03)).min(0.97),
                });
            }
        }
    }
    party_entanglements
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_engine::lifepath_economy::CareerTermConfig;

    #[test]
    fn test_extract_and_compile_lifepath_hobbies_events_and_faction_wedges() {
        let cfg = LifepathBuildConfig {
            youth_hobby: Some(PersonalHobbyId::CitizenOsintDeepfakeVerification),
            adolescent_life_event_roll_2d20: Some(38),
            faction_archetype: Some("SATYANG_MANJUSRI".to_string()),
            career_terms: vec![CareerTermConfig {
                life_event_roll_2d20: Some(40),
                ..CareerTermConfig::default()
            }],
            ..Default::default()
        };

        let pc1 = CharacterSheetRecord {
            character_id: "pc-aria".to_string(),
            character_name: "Aria Chen".to_string(),
            lifepath_config_json: Some(serde_json::to_string(&cfg).unwrap()),
            ..Default::default()
        };

        let pc2 = CharacterSheetRecord {
            character_id: "pc-vance".to_string(),
            character_name: "Marcus Vance".to_string(),
            faction_id: Some("GOVERNMENT_DAISE".to_string()),
            lifepath_config_json: Some(
                r#"{"personal_hobbies":["CIVILIAN_MARKSMANSHIP"],"formative_events":[{"event_code":"WRONGFUL_ACCUSATION_DEEPFAKE"}]}"#
                    .to_string(),
            ),
            ..Default::default()
        };

        let anchors = extract_character_lifepath_anchors(&pc1);
        assert_eq!(anchors.faction_id.as_deref(), Some("SATYANG"));
        assert_eq!(anchors.hobby_contacts.len(), 1);
        assert_eq!(
            anchors.hobby_contacts[0].hobby_code,
            "CITIZEN_OSINT_DEEPFAKE"
        );
        assert_eq!(anchors.formative_anchors.len(), 2);

        let cards = compile_lifepath_and_faction_hooks(&pc1, Some("St. Jude Clinic"), "awakened");
        assert!(
            cards
                .iter()
                .any(|c| c.category == HookCategory::FactionWedge)
        );
        assert!(
            cards
                .iter()
                .any(|c| c.category == HookCategory::HobbyContact)
        );
        assert!(
            cards
                .iter()
                .any(|c| c.category == HookCategory::FormativeAnchor)
        );

        let faction_entanglements = compile_faction_entanglements(&[pc1, pc2], "awakened");
        assert_eq!(faction_entanglements.len(), 1);
        assert!(
            faction_entanglements[0]
                .shared_scene_prompt
                .contains("DAISE")
        );
        assert!(
            faction_entanglements[0]
                .tension_axis
                .contains("Containment")
        );
    }
}
