//! PyO3 bindings for Satya's Motivational Hook Weaver (`hook_weaver`) and
//! prompt-driven NPC generator (`npc_generator`) for the AI Game Master.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use satya_campaign::{
    AwakenedFactionId, FactionArchetypeId, NpcGenerationParams, NpcTemplateKind,
    compile_party_hooks, derive_dissenter_or_proxy_hook, extract_character_lifepath_anchors,
    generate_npc_from_prompt_with_pack,
};
use satya_schema::CharacterSheetRecord;
use std::collections::BTreeMap;

fn to_py_err(msg: impl Into<String>) -> PyErr {
    PyValueError::new_err(msg.into())
}

/// Compiles per-character motivational hook cards and pairwise party entanglement wedges from
/// JSON-encoded [`CharacterSheetRecord`] sheets and scene context metadata.
#[pyfunction]
#[pyo3(signature = (party_sheets_json, scene_context_json=None, pack_id=None))]
pub fn compile_party_motivational_hooks_json(
    party_sheets_json: &str,
    scene_context_json: Option<&str>,
    pack_id: Option<&str>,
) -> PyResult<String> {
    let party: Vec<CharacterSheetRecord> = serde_json::from_str(party_sheets_json)
        .map_err(|e| to_py_err(format!("Invalid party sheets JSON: {e}")))?;
    let scene_context: BTreeMap<String, String> = match scene_context_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map_err(|e| to_py_err(format!("Invalid scene context JSON: {e}")))?,
        _ => BTreeMap::new(),
    };
    let effective_pack = pack_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("awakened");
    let manifest = compile_party_hooks(&party, &scene_context, effective_pack);
    serde_json::to_string(&manifest).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Generates a full, rules-legal [`satya_engine::GeneratedNpcResult`] (including structured
/// [`satya_engine::NpcMotivationalProfile`]) serialized as JSON.
#[pyfunction]
#[pyo3(signature = (prompt, template=None, name_override=None, career_terms=2, seed=0, pack_id=None))]
pub fn generate_npc_profile_json(
    prompt: &str,
    template: Option<&str>,
    name_override: Option<&str>,
    career_terms: usize,
    seed: u64,
    pack_id: Option<&str>,
) -> PyResult<String> {
    let parsed_faction_archetype = template.and_then(FactionArchetypeId::from_code_str);
    let parsed_template = if parsed_faction_archetype.is_some() {
        NpcTemplateKind::Auto
    } else {
        template
            .and_then(NpcTemplateKind::from_code_str)
            .unwrap_or(NpcTemplateKind::Auto)
    };
    let effective_pack = pack_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("awakened");
    let result = generate_npc_from_prompt_with_pack(
        &NpcGenerationParams {
            prompt: prompt.to_string(),
            name_override: name_override.map(str::to_string),
            template: parsed_template,
            faction_archetype: parsed_faction_archetype,
            career_terms,
            seed,
            owner_user_id: "ai-gm".to_string(),
        },
        effective_pack,
    );
    let mut payload =
        serde_json::to_value(&result).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))?;
    if effective_pack.eq_ignore_ascii_case("awakened")
        && let Some(obj) = payload.as_object_mut()
    {
        let char_faction = obj
            .get("character")
            .and_then(|c| c.get("faction_id"))
            .and_then(|v| v.as_str())
            .and_then(AwakenedFactionId::from_code_str);
        let char_arch = obj
            .get("character")
            .and_then(|c| c.get("faction_archetype"))
            .and_then(|v| v.as_str())
            .and_then(FactionArchetypeId::from_code_str);
        let origin_hook = derive_dissenter_or_proxy_hook(char_faction, char_arch, prompt, seed);
        if let Ok(hook_val) = serde_json::to_value(&origin_hook) {
            obj.insert("awakened_social_origin_hook".to_string(), hook_val);
        }
    }
    serde_json::to_string(&payload).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Extracts canonical Faction, Personal Hobby Contacts & Formative Life Event anchors from a
/// JSON-encoded [`CharacterSheetRecord`] for the AI GM and Embodied AI Player.
#[pyfunction]
#[pyo3(signature = (character_json))]
pub fn extract_character_lifepath_anchors_json(character_json: &str) -> PyResult<String> {
    let record: CharacterSheetRecord = serde_json::from_str(character_json)
        .map_err(|e| to_py_err(format!("Invalid character JSON: {e}")))?;
    let anchors = extract_character_lifepath_anchors(&record);
    serde_json::to_string(&anchors).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Registers motivational hook and NPC generator helpers on the `satya_py` module.
pub fn register_hooks_and_npc_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compile_party_motivational_hooks_json, m)?)?;
    m.add_function(wrap_pyfunction!(generate_npc_profile_json, m)?)?;
    m.add_function(wrap_pyfunction!(
        extract_character_lifepath_anchors_json,
        m
    )?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_and_npc_py_roundtrip() {
        let npc_json = generate_npc_profile_json(
            "Dr. Bandajara is a compassionate field surgeon who refuses to abandon clinics",
            None,
            Some("Dr. Bandajara"),
            2,
            101,
            Some("awakened"),
        )
        .unwrap();
        let npc_val: serde_json::Value = serde_json::from_str(&npc_json).unwrap();
        assert_eq!(npc_val["character"]["character_name"], "Dr. Bandajara");
        assert!(
            npc_val["motivation"]["core_drive"]
                .as_str()
                .unwrap_or("")
                .len()
                > 10
        );

        let sheets_json = serde_json::to_string(&vec![npc_val["character"].clone()]).unwrap();
        let hooks_json = compile_party_motivational_hooks_json(
            &sheets_json,
            Some(r#"{"location":"St. Jude Clinic","sector_preset":"hospital"}"#),
            Some("awakened"),
        )
        .unwrap();
        let hooks_val: serde_json::Value = serde_json::from_str(&hooks_json).unwrap();
        assert_eq!(hooks_val["pack_id"], "awakened");
        assert!(!hooks_val["hooks"].as_array().unwrap().is_empty());

        let satyang_json = generate_npc_profile_json(
            "Archivist monitoring clinic power draws",
            Some("SATYANG_MANJUSRI"),
            Some("Linnea Cho"),
            3,
            303,
            Some("awakened"),
        )
        .unwrap();
        let satyang_val: serde_json::Value = serde_json::from_str(&satyang_json).unwrap();
        assert_eq!(satyang_val["psyche_traits"].as_array().unwrap().len(), 9);
        assert_eq!(satyang_val["character"]["faction_id"], "SATYANG");
        assert_eq!(
            satyang_val["character"]["faction_archetype"],
            "SATYANG_MANJUSRI"
        );
        assert!(satyang_val["awakened_social_origin_hook"]["hook_kind"].is_string());

        let anchors_raw = extract_character_lifepath_anchors_json(
            &serde_json::to_string(&satyang_val["character"]).unwrap(),
        )
        .unwrap();
        let anchors_val: serde_json::Value = serde_json::from_str(&anchors_raw).unwrap();
        assert_eq!(anchors_val["faction_id"], "SATYANG");
        assert_eq!(anchors_val["faction_archetype"], "SATYANG_MANJUSRI");
        assert!(!anchors_val["hobby_contacts"].as_array().unwrap().is_empty());
    }
}
