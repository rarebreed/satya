//! PyO3 bindings for Satya's Motivational Hook Weaver (`hook_weaver`),
//! prompt-driven NPC generator (`npc_generator`), and embedded `config/campaign.toml`.

use std::collections::BTreeMap;

use pyo3::prelude::*;
use satya_campaign::{
    AwakenedFactionId, FactionArchetypeId, NpcGenerationParams, NpcTemplateKind,
    compile_party_hooks, derive_dissenter_or_proxy_hook, embedded_campaign_config,
    generate_npc_from_prompt_with_pack,
};

use super::py_bridge::{
    PyJsonDict, PyJsonInput, PyListInput, opt_from_py_object, py_to_json_value, to_py_err,
    to_py_object,
};

/// Compiles per-character motivational hook cards and pairwise party entanglement wedges from
/// [`satya_schema::CharacterSheetRecord`] mappings/JSON and scene context metadata.
#[pyfunction]
#[pyo3(signature = (characters, scene_context=None, pack_id=None))]
pub fn compile_party_motivational_hooks(
    py: Python<'_>,
    characters: PyListInput<'_>,
    scene_context: Option<PyJsonInput<'_>>,
    pack_id: Option<&str>,
) -> PyResult<PyJsonDict> {
    let chars_val = py_to_json_value(&characters, "party characters")?;
    let party = super::schema_py::parse_flexible_character_list(chars_val).map_err(to_py_err)?;
    let ctx = opt_from_py_object::<BTreeMap<String, String>>(scene_context, "scene_context")?
        .unwrap_or_default();
    let effective_pack = pack_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("awakened");
    to_py_object(py, &compile_party_hooks(&party, &ctx, effective_pack))
}

/// Generates a full, rules-legal [`satya_campaign::GeneratedNpcResult`] (including structured
/// [`satya_campaign::NpcMotivationalProfile`]) returned as a native Python dict.
#[pyfunction]
#[pyo3(signature = (prompt, template=None, name_override=None, career_terms=2, seed=0, pack_id=None))]
pub fn generate_npc_profile(
    py: Python<'_>,
    prompt: &str,
    template: Option<&str>,
    name_override: Option<&str>,
    career_terms: usize,
    seed: u64,
    pack_id: Option<&str>,
) -> PyResult<PyJsonDict> {
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
    let mut payload = serde_json::to_value(&result)
        .map_err(|e| super::py_bridge::to_py_err(format!("NPC encode failed: {e}")))?;
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
    super::py_bridge::value_to_bound_py(py, &payload).map(|b| b.unbind())
}

/// Extracts canonical Faction, Personal Hobby Contacts & Formative Life Event anchors from a
/// partial or full [`satya_schema::CharacterSheetRecord`] dict or JSON string.
#[pyfunction]
#[pyo3(signature = (character, index=0))]
pub fn extract_character_lifepath_anchors(
    py: Python<'_>,
    character: PyJsonInput<'_>,
    index: usize,
) -> PyResult<PyJsonDict> {
    let val = py_to_json_value(&character, "character")?;
    let record = super::schema_py::parse_flexible_character_record(val, index).map_err(to_py_err)?;
    to_py_object(py, &satya_campaign::extract_character_lifepath_anchors(&record))
}

/// Returns the single-source compile-time embedded `config/campaign.toml` structure (`active_pack`, `packs`).
#[pyfunction]
pub fn get_embedded_campaign_config(py: Python<'_>) -> PyResult<PyJsonDict> {
    to_py_object(py, embedded_campaign_config())
}

/// Registers motivational hook, NPC generator, and campaign config helpers on the `satya_py` module.
pub fn register_hooks_and_npc_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compile_party_motivational_hooks, m)?)?;
    m.add_function(wrap_pyfunction!(generate_npc_profile, m)?)?;
    m.add_function(wrap_pyfunction!(extract_character_lifepath_anchors, m)?)?;
    m.add_function(wrap_pyfunction!(get_embedded_campaign_config, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::py_bridge::py_to_value;
    use pyo3::types::{PyList, PyString};

    #[test]
    fn test_hooks_and_npc_py_roundtrip() {
        Python::initialize();
        Python::attach(|py| {
            let cfg = get_embedded_campaign_config(py).unwrap();
            let cfg_val = py_to_value(cfg.bind(py)).unwrap();
            assert_eq!(cfg_val["active_pack"], "awakened");
            assert_eq!(
                cfg_val["packs"]["awakened"]["default_sector_preset"],
                "community_clinic_annex"
            );

            let npc = generate_npc_profile(
                py,
                "Dr. Bandajara is a compassionate field surgeon who refuses to abandon clinics",
                None,
                Some("Dr. Bandajara"),
                2,
                101,
                Some("awakened"),
            )
            .unwrap();
            let npc_val = py_to_value(npc.bind(py)).unwrap();
            assert_eq!(npc_val["character"]["character_name"], "Dr. Bandajara");
            assert!(npc_val["motivation"]["core_drive"].as_str().unwrap_or("").len() > 10);

            let char_obj = npc.bind(py).get_item("character").unwrap();
            let sheets = PyList::new(py, [&char_obj]).unwrap().into_any();
            let ctx = PyString::new(py, r#"{"location":"St. Jude Clinic","sector_preset":"hospital"}"#)
                .into_any();
            let hooks =
                compile_party_motivational_hooks(py, sheets, Some(ctx), Some("awakened")).unwrap();
            let hooks_val = py_to_value(hooks.bind(py)).unwrap();
            assert_eq!(hooks_val["pack_id"], "awakened");
            assert!(!hooks_val["hooks"].as_array().unwrap().is_empty());

            let satyang = generate_npc_profile(
                py,
                "Archivist monitoring clinic power draws",
                Some("SATYANG_MANJUSRI"),
                Some("Linnea Cho"),
                3,
                303,
                Some("awakened"),
            )
            .unwrap();
            let satyang_val = py_to_value(satyang.bind(py)).unwrap();
            assert_eq!(satyang_val["psyche_traits"].as_array().unwrap().len(), 9);
            assert_eq!(satyang_val["character"]["faction_id"], "SATYANG");
            assert_eq!(satyang_val["character"]["faction_archetype"], "SATYANG_MANJUSRI");
            assert!(satyang_val["awakened_social_origin_hook"]["hook_kind"].is_string());

            let sat_char = satyang.bind(py).get_item("character").unwrap();
            let anchors = extract_character_lifepath_anchors(py, sat_char, 0).unwrap();
            let anchors_val = py_to_value(anchors.bind(py)).unwrap();
            assert_eq!(anchors_val["faction_id"], "SATYANG");
            assert_eq!(anchors_val["faction_archetype"], "SATYANG_MANJUSRI");
            assert!(!anchors_val["hobby_contacts"].as_array().unwrap().is_empty());

            let partial_json = PyString::new(
                py,
                r#"{"character_name":"Partial Runner","characteristics":{"for":14,"trs":4,"cps":17},"youth_hobby":"CITIZEN_OSINT_DEEPFAKE","faction_archetype":"SATYANG_MANJUSRI"}"#,
            )
            .into_any();
            let partial_anchors = extract_character_lifepath_anchors(py, partial_json.clone(), 2).unwrap();
            let pa_val = py_to_value(partial_anchors.bind(py)).unwrap();
            assert_eq!(pa_val["faction_id"], "SATYANG");
            assert_eq!(pa_val["faction_archetype"], "SATYANG_MANJUSRI");
            assert_eq!(
                pa_val["hobby_contacts"][0]["hobby_code"],
                "CITIZEN_OSINT_DEEPFAKE"
            );

            let partial_party = PyList::new(py, [&partial_json]).unwrap().into_any();
            let partial_hooks =
                compile_party_motivational_hooks(py, partial_party, None, Some("awakened"))
                    .unwrap();
            let ph_val = py_to_value(partial_hooks.bind(py)).unwrap();
            assert!(ph_val["character_profiles"]["pc-0"].is_object());
            assert!(!ph_val["hooks"].as_array().unwrap().is_empty());
        });
    }
}
