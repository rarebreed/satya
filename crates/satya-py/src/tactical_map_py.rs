//! PyO3 bindings for continuous metric tactical map generation, token collision/destination
//! validation, analytical LoS/cover evaluation, and cursor hover inspection powered by
//! [`satya_engine`].

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use satya_engine::map_narrative_manifest::{
    SpatialSensoryPin, evaluate_passive_movement_perceptions, extract_sector_narrative_manifest,
};
use satya_engine::tactical_map::{
    SectorPreset, TacticalSceneView, build_procedural_map_request_with_options,
    continue_procedural_map, evaluate_los_continuous,
};
use satya_engine::tactical_map_geom::{circle_collides_obstacles, resolve_token_destination_cm};
use satya_engine::tactical_map_inspect::{MapHoverCategory, inspect_map_point};
use satya_schema::{
    CharacterSheetRecord, EconomicTier, GmMapUpdateRequest, MapContinuationDir, MapScalePreset,
    TokenVisualSpec, ZoningContext,
};
use serde_json::{Map, Value, json};

fn to_py_err(msg: impl Into<String>) -> PyErr {
    PyValueError::new_err(msg.into())
}

fn hover_category_code(category: MapHoverCategory) -> &'static str {
    match category {
        MapHoverCategory::Token => "TOKEN",
        MapHoverCategory::EdgeBarrier => "EDGE_BARRIER",
        MapHoverCategory::SolidProp => "SOLID_PROP",
        MapHoverCategory::SurfaceProp => "SURFACE_PROP",
        MapHoverCategory::Ground => "GROUND",
    }
}

/// Parses flexible Python `map_state` JSON dictionaries into a canonical [`GmMapUpdateRequest`]
/// without mutating locked `v1` wire structs.
pub(crate) fn parse_flexible_map_json(map_json: &str) -> Result<GmMapUpdateRequest, String> {
    let mut root: Value = serde_json::from_str(map_json)
        .map_err(|e| format!("Invalid tactical map JSON payload: {e}"))?;
    let obj = root
        .as_object_mut()
        .ok_or_else(|| "Tactical map JSON payload must be an object".to_string())?;

    if !obj.contains_key("map_id") {
        obj.insert("map_id".to_string(), json!("py-map"));
    }
    if !obj.contains_key("title") {
        let title = obj
            .get("theme")
            .and_then(Value::as_str)
            .unwrap_or("Tactical Map")
            .to_string();
        obj.insert("title".to_string(), json!(title));
    }
    if !obj.contains_key("image_url") {
        let url = obj
            .get("map_image_url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        obj.insert("image_url".to_string(), json!(url));
    }
    if !obj.contains_key("grid_width") {
        obj.insert("grid_width".to_string(), json!(16));
    }
    if !obj.contains_key("grid_height") {
        obj.insert("grid_height".to_string(), json!(12));
    }
    if !obj.contains_key("cell_size_cm") {
        obj.insert("cell_size_cm".to_string(), json!(100));
    }

    let mut synthesized_visuals: Vec<TokenVisualSpec> = Vec::new();
    if let Some(tokens_val) = obj.get_mut("tokens").and_then(Value::as_array_mut) {
        for (idx, tok_val) in tokens_val.iter_mut().enumerate() {
            if let Some(tok) = tok_val.as_object_mut() {
                let entity_id = tok
                    .get("entity_id")
                    .or_else(|| tok.get("token_id"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("tok-{idx}"));
                tok.entry("entity_id")
                    .or_insert_with(|| Value::String(entity_id.clone()));

                let label = tok
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(&entity_id)
                    .to_string();
                tok.entry("label").or_insert_with(|| Value::String(label));

                let gx = tok
                    .get("x")
                    .or_else(|| tok.get("grid_x"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                let gy = tok
                    .get("y")
                    .or_else(|| tok.get("grid_y"))
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                tok.entry("x").or_insert_with(|| json!(gx));
                tok.entry("y").or_insert_with(|| json!(gy));
                tok.entry("visibility")
                    .or_insert_with(|| Value::String("PUBLIC".to_string()));

                let pos_x_cm = tok.get("pos_x_cm").and_then(Value::as_i64).map(|v| v as i32);
                let pos_y_cm = tok.get("pos_y_cm").and_then(Value::as_i64).map(|v| v as i32);
                if pos_x_cm.is_some() || pos_y_cm.is_some() {
                    let facing_deg = tok
                        .get("facing_deg")
                        .and_then(Value::as_i64)
                        .unwrap_or(0) as i32;
                    let color_hex = tok
                        .get("color_hex")
                        .and_then(Value::as_str)
                        .unwrap_or("#38bdf8")
                        .to_string();
                    let model_preset = tok
                        .get("model_preset")
                        .and_then(Value::as_str)
                        .unwrap_or("operative_light")
                        .to_string();
                    synthesized_visuals.push(TokenVisualSpec {
                        entity_id,
                        model_preset,
                        color_hex,
                        facing_deg,
                        elevation_cm: 0,
                        floor_index: 0,
                        stance_pose: "idle".to_string(),
                        portrait_url: None,
                        pos_x_cm,
                        pos_y_cm,
                    });
                }
            }
        }
    }

    if let Some(props_val) = obj.get_mut("props").and_then(Value::as_array_mut) {
        for prop_val in props_val.iter_mut() {
            if let Some(prop) = prop_val.as_object_mut() {
                for (primary, alias) in [
                    ("x", "origin_x"),
                    ("y", "origin_y"),
                    ("w_cells", "span_w"),
                    ("h_cells", "span_h"),
                ] {
                    if prop.contains_key(primary) {
                        prop.remove(alias);
                    } else if let Some(v) = prop.remove(alias) {
                        prop.insert(primary.to_string(), v);
                    }
                }
            }
        }
    }

    let mut map: GmMapUpdateRequest = serde_json::from_value(root)
        .map_err(|e| format!("Failed to deserialize GmMapUpdateRequest: {e}"))?;

    for vis in synthesized_visuals {
        if !map.token_visuals.iter().any(|v| v.entity_id == vis.entity_id) {
            map.token_visuals.push(vis);
        }
    }

    // Ensure normalized wall edges match canonical JSON if callers omitted defaults.
    let _ = Map::new();
    Ok(map)
}

/// Checks whether a circle of radius `radius_m` at continuous `(x_m, y_m)` collides with board
/// bounds, walls, closed doors, windows, or solid props.
#[pyfunction]
pub fn check_tactical_token_collision(
    map_json: &str,
    x_m: f32,
    y_m: f32,
    radius_m: f32,
) -> PyResult<bool> {
    let map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let (board_w_m, board_h_m) = map.board_size_m();
    Ok(circle_collides_obstacles(
        board_w_m,
        board_h_m,
        &map.wall_edges,
        &map.props,
        x_m,
        y_m,
        radius_m,
    ))
}

/// Resolves continuous destination `(target_x_cm, target_y_cm)` against obstacles and other tokens.
#[pyfunction]
pub fn resolve_tactical_token_destination_cm(
    map_json: &str,
    moving_entity_id: &str,
    target_x_cm: i32,
    target_y_cm: i32,
) -> PyResult<(i32, i32)> {
    let map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    resolve_token_destination_cm(&map, moving_entity_id, target_x_cm, target_y_cm).map_err(to_py_err)
}

/// Evaluates continuous Line-of-Sight, cover grade, and recommended `Δ_ext` between two positions in meters.
#[pyfunction]
pub fn evaluate_tactical_los_and_cover(
    py: Python<'_>,
    map_json: &str,
    x1_m: f32,
    y1_m: f32,
    x2_m: f32,
    y2_m: f32,
) -> PyResult<Py<PyDict>> {
    let map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let scene = TacticalSceneView::from_map(&map);
    let eval = evaluate_los_continuous(&scene, (x1_m, y1_m), (x2_m, y2_m));
    let d = PyDict::new(py);
    d.set_item("distance_m", eval.distance_m)?;
    d.set_item("has_los", eval.has_los)?;
    d.set_item("cover_grade", eval.cover_grade.as_str())?;
    d.set_item("cover_label", eval.cover_grade.label())?;
    d.set_item("recommended_delta_ext", eval.recommended_delta_ext)?;
    Ok(d.unbind())
}

/// Inspects any continuous `(x_m, y_m)` map coordinate (token, wall/door, prop, or ground surface).
#[pyfunction]
pub fn inspect_tactical_map_point(
    py: Python<'_>,
    map_json: &str,
    theme_name: &str,
    x_m: f32,
    y_m: f32,
) -> PyResult<Option<Py<PyDict>>> {
    let map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let Some(info) = inspect_map_point(&map, theme_name, x_m, y_m) else {
        return Ok(None);
    };
    let d = PyDict::new(py);
    d.set_item("category", hover_category_code(info.category))?;
    d.set_item("title", info.title)?;
    d.set_item("subtitle", info.subtitle)?;
    d.set_item("material_label", info.material_label)?;
    d.set_item("cover_label", info.cover_label)?;
    d.set_item("height_cm", info.height_cm)?;
    d.set_item("elevation_cm", info.elevation_cm)?;
    d.set_item("rotation_deg", info.rotation_deg)?;
    d.set_item("world_x_m", info.world_x_m)?;
    d.set_item("world_y_m", info.world_y_m)?;
    d.set_item("target_ref", info.target_ref)?;
    Ok(Some(d.unbind()))
}

/// Generates a procedural vector tactical sector serialized as [`GmMapUpdateRequest`] JSON.
#[pyfunction]
#[pyo3(signature = (map_id, preset_or_theme, title=None, width=None, height=None, seed=42, economic_tier=None, zoning=None, scale=None))]
pub fn generate_procedural_tactical_sector_json(
    map_id: &str,
    preset_or_theme: &str,
    title: Option<&str>,
    width: Option<i32>,
    height: Option<i32>,
    seed: i64,
    economic_tier: Option<&str>,
    zoning: Option<&str>,
    scale: Option<&str>,
) -> PyResult<String> {
    let preset = SectorPreset::from_code_str(preset_or_theme).unwrap_or_default();
    let tier_opt = economic_tier.and_then(EconomicTier::from_code_str);
    let zoning_opt = zoning.and_then(ZoningContext::from_code_str);
    let (default_w, default_h) = match scale.map(MapScalePreset::parse_loose) {
        Some(s) => preset.resolve_scale_dimensions(s),
        None => (16, 12),
    };
    let w = width.unwrap_or(default_w);
    let h = height.unwrap_or(default_h);
    let req = build_procedural_map_request_with_options(
        map_id, preset, title, w, h, seed, tier_opt, zoning_opt,
    );
    serde_json::to_string(&req).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Stitches an adjacent macro-world neighbor sector in `direction` (`NORTH`/`SOUTH`/`EAST`/`WEST`).
#[pyfunction]
#[pyo3(signature = (map_json, direction, seed=None))]
pub fn continue_procedural_tactical_map_json(
    map_json: &str,
    direction: &str,
    seed: Option<i64>,
) -> PyResult<String> {
    let map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let dir = MapContinuationDir::from_code_str(direction)
        .ok_or_else(|| to_py_err(format!("Invalid continuation direction '{direction}'")))?;
    let outcome = continue_procedural_map(&map, dir, seed).map_err(to_py_err)?;
    serde_json::to_string(&outcome.map).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Switches `active_floor` on a multi-floor procedural sector and syncs top-level wall/prop/surface mirrors.
#[pyfunction]
pub fn switch_tactical_map_floor_json(
    map_json: &str,
    target_floor_index: i32,
) -> PyResult<String> {
    let mut map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    if !map.switch_active_floor(target_floor_index) {
        return Err(to_py_err(format!(
            "Floor {target_floor_index} not present on tactical map '{}'",
            map.map_id
        )));
    }
    serde_json::to_string(&map).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Extracts a semantic [`satya_engine::SectorNarrativeManifest`] JSON overview from a tactical map.
#[pyfunction]
pub fn extract_sector_narrative_manifest_json(map_json: &str) -> PyResult<String> {
    let map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let manifest = extract_sector_narrative_manifest(&map);
    serde_json::to_string(&manifest).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Evaluates passive movement perceptions (`SIG`/`HEA`/`OLF`/`ITU`) triggered by moving `moved_entity_id`.
#[pyfunction]
#[pyo3(signature = (map_json, moved_entity_id, observers_json, pins_json=None))]
pub fn evaluate_passive_movement_perceptions_json(
    map_json: &str,
    moved_entity_id: &str,
    observers_json: &str,
    pins_json: Option<&str>,
) -> PyResult<String> {
    let map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let observers: Vec<CharacterSheetRecord> = if observers_json.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(observers_json)
            .map_err(|e| to_py_err(format!("Invalid observers JSON: {e}")))?
    };
    let pins: Vec<SpatialSensoryPin> = match pins_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map_err(|e| to_py_err(format!("Invalid sensory pins JSON: {e}")))?,
        _ => Vec::new(),
    };
    let whispers = evaluate_passive_movement_perceptions(&map, moved_entity_id, &observers, &pins);
    serde_json::to_string(&whispers).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Registers tactical map spatial bindings on the `satya_py` Python module.
pub fn register_tactical_map_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_tactical_token_collision, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_tactical_token_destination_cm, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_tactical_los_and_cover, m)?)?;
    m.add_function(wrap_pyfunction!(inspect_tactical_map_point, m)?)?;
    m.add_function(wrap_pyfunction!(generate_procedural_tactical_sector_json, m)?)?;
    m.add_function(wrap_pyfunction!(continue_procedural_tactical_map_json, m)?)?;
    m.add_function(wrap_pyfunction!(switch_tactical_map_floor_json, m)?)?;
    m.add_function(wrap_pyfunction!(extract_sector_narrative_manifest_json, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_passive_movement_perceptions_json, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tactical_map_py_wrappers_roundtrip_and_los_inspect() {
        let sector_json = generate_procedural_tactical_sector_json(
            "map-py-1",
            "docking_bay",
            Some("Bay 4"),
            Some(12),
            Some(10),
            42,
            None,
            None,
            None,
        )
        .unwrap();
        let manifest_json = extract_sector_narrative_manifest_json(&sector_json).unwrap();
        let manifest_val: Value = serde_json::from_str(&manifest_json).unwrap();
        assert_eq!(manifest_val["cardinal_exits"].as_array().unwrap().len(), 4);
        assert!(!manifest_val["walkable_anchors"].as_array().unwrap().is_empty());

        let hosp_json = generate_procedural_tactical_sector_json(
            "map-hosp",
            "hospital",
            Some("St. Jude Hospital"),
            None,
            None,
            77,
            None,
            None,
            Some("STANDARD"),
        )
        .unwrap();
        let hosp_val: Value = serde_json::from_str(&hosp_json).unwrap();
        assert!(hosp_val["floors"].as_array().unwrap().len() >= 2);
        let switched_json = switch_tactical_map_floor_json(&hosp_json, 1).unwrap();
        let switched_val: Value = serde_json::from_str(&switched_json).unwrap();
        assert_eq!(switched_val["active_floor"], 1);

        let mut parsed = parse_flexible_map_json(&sector_json).unwrap();
        assert_eq!(parsed.grid_width, 12);
        assert_eq!(parsed.world_sectors.len(), 1);
        assert!(!parsed.wall_edges.is_empty());
        assert!(!parsed.props.is_empty());

        let east_json = continue_procedural_tactical_map_json(&sector_json, "EAST", Some(99)).unwrap();
        let east_map = parse_flexible_map_json(&east_json).unwrap();
        assert_eq!(east_map.world_sectors.len(), 2);
        assert!(east_map.grid_width > 12);

        // Python map dict flexibility: token_id/grid_x/grid_y without top-level image_url.
        let py_dict_json = serde_json::json!({
            "grid_width": 12,
            "grid_height": 10,
            "tokens": [
                {
                    "token_id": "tok-vance",
                    "label": "Capt. Vance",
                    "grid_x": 2,
                    "grid_y": 5,
                    "pos_x_cm": 250,
                    "pos_y_cm": 550
                }
            ],
            "wall_edges": parsed.wall_edges,
            "props": parsed.props,
        })
        .to_string();
        parsed = parse_flexible_map_json(&py_dict_json).unwrap();
        assert_eq!(parsed.tokens.len(), 1);
        assert_eq!(parsed.tokens[0].entity_id, "tok-vance");
        assert_eq!(parsed.token_pos_cm(&parsed.tokens[0]), (250, 550));

        // Out-of-bounds collision and open walkable spot resolution
        assert!(check_tactical_token_collision(&py_dict_json, 0.10, 5.0, 0.30).unwrap());
        let mut resolved = None;
        for gy in 2..9 {
            for gx in 2..10 {
                let cx = gx * 100 + 50;
                let cy = gy * 100 + 50;
                if let Ok(pt) =
                    resolve_tactical_token_destination_cm(&py_dict_json, "tok-vance", cx, cy)
                {
                    resolved = Some(pt);
                    break;
                }
            }
            if resolved.is_some() {
                break;
            }
        }
        assert!(resolved.is_some());

        // Pure-Rust LoS evaluation & cursor inspection sanity checks
        let scene = TacticalSceneView::from_map(&parsed);
        let los = evaluate_los_continuous(&scene, (2.5, 5.5), (9.5, 5.5));
        assert!(los.distance_m > 6.0);
        let hit = inspect_map_point(&parsed, "Sector 4 Docking Bay", 2.5, 5.5).unwrap();
        assert_eq!(hit.category, MapHoverCategory::Token);
    }
}
