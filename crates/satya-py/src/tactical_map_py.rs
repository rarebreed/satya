//! PyO3 bindings for continuous metric tactical map generation, token collision/destination
//! validation, analytical LoS/cover evaluation, and cursor hover inspection powered by
//! [`satya_maps`].

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use satya_engine::ballistics::lookup_firearm_preset;
use satya_engine::environment_vehicles::CombatMovementGait;
use satya_engine::explosives_and_arsenal::ExplosiveOrdnanceId;
use satya_maps::barrier_ballistics::{
    BallisticShotQuery, evaluate_ballistic_shot, evaluate_spatial_blast_and_breach,
};
use satya_maps::map_dressing::make_prop_at_cm;
use satya_maps::map_narrative_manifest::{
    SpatialSensoryPin, evaluate_passive_movement_perceptions, extract_sector_narrative_manifest,
};
use satya_maps::tactical_map::{
    GaitReachQuery, SectorPreset, TacticalSceneView, build_procedural_map_request_with_options,
    compute_reachable_continuous, continue_procedural_map_with_preset, evaluate_los_continuous,
};
use satya_maps::tactical_map_geom::{
    circle_collides_obstacles, place_prop_on_map, remove_prop_at_point,
    resolve_token_destination_cm,
};
use satya_maps::tactical_map_inspect::{MapHoverCategory, inspect_map_point};
use satya_schema::{
    CharacterSheetRecord, EconomicTier, EdgeSide, GmMapUpdateRequest, MapContinuationDir,
    MapScalePreset, MapWallEdge, MaterialClass, PropKind, TokenVisualSpec, WallEdgeKind,
    ZoningContext,
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

fn normalize_wall_edges_and_props(container: &mut Map<String, Value>) {
    if let Some(edges_val) = container.get_mut("wall_edges").and_then(Value::as_array_mut) {
        for edge_val in edges_val.iter_mut() {
            if let Some(edge) = edge_val.as_object_mut()
                && edge.get("material").and_then(Value::as_str) == Some("CONCRETE")
            {
                edge.insert(
                    "material".to_string(),
                    Value::String("CONCRETE_BLOCK".to_string()),
                );
            }
        }
    }
    if let Some(props_val) = container.get_mut("props").and_then(Value::as_array_mut) {
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
                if prop.get("material").and_then(Value::as_str) == Some("CONCRETE") {
                    prop.insert(
                        "material".to_string(),
                        Value::String("CONCRETE_BLOCK".to_string()),
                    );
                }
            }
        }
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

    struct TokenVisualOverride {
        entity_id: String,
        model_preset: Option<String>,
        color_hex: Option<String>,
        facing_deg: Option<i32>,
        elevation_cm: Option<i32>,
        floor_index: Option<i32>,
        stance_pose: Option<String>,
        portrait_url: Option<String>,
        pos_x_cm: Option<i32>,
        pos_y_cm: Option<i32>,
    }

    let mut visual_overrides: Vec<TokenVisualOverride> = Vec::new();
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

                let pos_x_cm = tok
                    .get("pos_x_cm")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32);
                let pos_y_cm = tok
                    .get("pos_y_cm")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32);
                let floor_index = tok
                    .get("floor_index")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32);
                let elevation_cm = tok
                    .get("elevation_cm")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32);
                let facing_deg = tok
                    .get("facing_deg")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32);
                let color_hex = tok
                    .get("color_hex")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let model_preset = tok
                    .get("model_preset")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let stance_pose = tok
                    .get("stance_pose")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let portrait_url = tok
                    .get("portrait_url")
                    .and_then(Value::as_str)
                    .map(str::to_string);

                if pos_x_cm.is_some()
                    || pos_y_cm.is_some()
                    || floor_index.is_some()
                    || elevation_cm.is_some()
                    || model_preset.is_some()
                    || color_hex.is_some()
                    || facing_deg.is_some()
                    || stance_pose.is_some()
                    || portrait_url.is_some()
                {
                    visual_overrides.push(TokenVisualOverride {
                        entity_id,
                        model_preset,
                        color_hex,
                        facing_deg,
                        elevation_cm,
                        floor_index,
                        stance_pose,
                        portrait_url,
                        pos_x_cm,
                        pos_y_cm,
                    });
                }
            }
        }
    }

    normalize_wall_edges_and_props(obj);
    if let Some(floors_val) = obj.get_mut("floors").and_then(Value::as_array_mut) {
        for floor_val in floors_val.iter_mut() {
            if let Some(floor) = floor_val.as_object_mut() {
                normalize_wall_edges_and_props(floor);
            }
        }
    }

    let mut map: GmMapUpdateRequest = serde_json::from_value(root)
        .map_err(|e| format!("Failed to deserialize GmMapUpdateRequest: {e}"))?;

    for ov in visual_overrides {
        if let Some(existing) = map
            .token_visuals
            .iter_mut()
            .find(|v| v.entity_id == ov.entity_id)
        {
            if let Some(p) = ov.model_preset {
                existing.model_preset = p;
            }
            if let Some(c) = ov.color_hex {
                existing.color_hex = c;
            }
            if let Some(f) = ov.facing_deg {
                existing.facing_deg = f;
            }
            if let Some(e) = ov.elevation_cm {
                existing.elevation_cm = e;
            }
            if let Some(fl) = ov.floor_index {
                existing.floor_index = fl;
            }
            if let Some(s) = ov.stance_pose {
                existing.stance_pose = s;
            }
            if let Some(u) = ov.portrait_url {
                existing.portrait_url = Some(u);
            }
            if let Some(x) = ov.pos_x_cm {
                existing.pos_x_cm = Some(x);
            }
            if let Some(y) = ov.pos_y_cm {
                existing.pos_y_cm = Some(y);
            }
        } else {
            map.token_visuals.push(TokenVisualSpec {
                entity_id: ov.entity_id,
                model_preset: ov
                    .model_preset
                    .unwrap_or_else(|| "operative_light".to_string()),
                color_hex: ov.color_hex.unwrap_or_else(|| "#38bdf8".to_string()),
                facing_deg: ov.facing_deg.unwrap_or(0),
                elevation_cm: ov.elevation_cm.unwrap_or(0),
                floor_index: ov.floor_index.unwrap_or(map.active_floor),
                stance_pose: ov.stance_pose.unwrap_or_else(|| "idle".to_string()),
                portrait_url: ov.portrait_url,
                pos_x_cm: ov.pos_x_cm,
                pos_y_cm: ov.pos_y_cm,
            });
        }
    }

    Ok(map)
}

fn map_for_floor(
    mut map: GmMapUpdateRequest,
    floor_index: Option<i32>,
) -> PyResult<GmMapUpdateRequest> {
    if let Some(f) = floor_index
        && !map.floors.is_empty()
        && map.active_floor != f
        && !map.switch_active_floor(f)
    {
        return Err(to_py_err(format!(
            "Floor {f} not present on tactical map '{}'",
            map.map_id
        )));
    }
    Ok(map)
}

/// Checks whether a circle of radius `radius_m` at continuous `(x_m, y_m)` collides with board
/// bounds, walls, closed doors, windows, or solid props.
#[pyfunction]
#[pyo3(signature = (map_json, x_m, y_m, radius_m, floor_index=None))]
pub fn check_tactical_token_collision(
    map_json: &str,
    x_m: f32,
    y_m: f32,
    radius_m: f32,
    floor_index: Option<i32>,
) -> PyResult<bool> {
    let map = map_for_floor(
        parse_flexible_map_json(map_json).map_err(to_py_err)?,
        floor_index,
    )?;
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
#[pyo3(signature = (map_json, moving_entity_id, target_x_cm, target_y_cm, floor_index=None))]
pub fn resolve_tactical_token_destination_cm(
    map_json: &str,
    moving_entity_id: &str,
    target_x_cm: i32,
    target_y_cm: i32,
    floor_index: Option<i32>,
) -> PyResult<(i32, i32)> {
    let raw_map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let target_floor = floor_index
        .or_else(|| (!raw_map.floors.is_empty()).then(|| raw_map.token_floor(moving_entity_id)));
    let map = map_for_floor(raw_map, target_floor)?;
    resolve_token_destination_cm(&map, moving_entity_id, target_x_cm, target_y_cm)
        .map_err(to_py_err)
}

/// Evaluates continuous Line-of-Sight, cover grade, and recommended `Δ_ext` between two positions in meters.
#[pyfunction]
#[pyo3(signature = (map_json, x1_m, y1_m, x2_m, y2_m, floor_index=None))]
pub fn evaluate_tactical_los_and_cover(
    py: Python<'_>,
    map_json: &str,
    x1_m: f32,
    y1_m: f32,
    x2_m: f32,
    y2_m: f32,
    floor_index: Option<i32>,
) -> PyResult<Py<PyDict>> {
    let map = map_for_floor(
        parse_flexible_map_json(map_json).map_err(to_py_err)?,
        floor_index,
    )?;
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
#[pyo3(signature = (map_json, theme_name, x_m, y_m, floor_index=None))]
pub fn inspect_tactical_map_point(
    py: Python<'_>,
    map_json: &str,
    theme_name: &str,
    x_m: f32,
    y_m: f32,
    floor_index: Option<i32>,
) -> PyResult<Option<Py<PyDict>>> {
    let map = map_for_floor(
        parse_flexible_map_json(map_json).map_err(to_py_err)?,
        floor_index,
    )?;
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
    d.set_item(
        "acoustic_surface",
        info.acoustic_surface
            .map(|a| serialize_enum_code(&a)),
    )?;
    d.set_item("acoustic_stealth_delta_ext", info.acoustic_stealth_delta_ext)?;
    d.set_item(
        "traction_tier",
        info.traction_tier.map(|t| serialize_enum_code(&t)),
    )?;
    d.set_item("in_faraday_zone", info.in_faraday_zone)?;
    d.set_item("ambient_light_delta_ext", info.ambient_light_delta_ext)?;
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
    let preset = SectorPreset::from_code_str(preset_or_theme)
        .ok_or_else(|| to_py_err(format!("Unknown sector preset '{preset_or_theme}'")))?;
    let tier_opt = economic_tier.and_then(EconomicTier::from_code_str);
    let zoning_opt = zoning.and_then(ZoningContext::from_code_str);
    let effective_scale = scale.map(MapScalePreset::parse_loose).unwrap_or(MapScalePreset::Auto);
    let (default_w, default_h) = preset.resolve_scale_dimensions(effective_scale);
    let w = width.unwrap_or(default_w);
    let h = height.unwrap_or(default_h);
    let req = build_procedural_map_request_with_options(
        map_id, preset, title, w, h, seed, tier_opt, zoning_opt,
    );
    serde_json::to_string(&req).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Stitches an adjacent macro-world neighbor sector in `direction` (`NORTH`/`SOUTH`/`EAST`/`WEST`).
#[pyfunction]
#[pyo3(signature = (map_json, direction, seed=None, preset_override=None))]
pub fn continue_procedural_tactical_map_json(
    map_json: &str,
    direction: &str,
    seed: Option<i64>,
    preset_override: Option<&str>,
) -> PyResult<String> {
    let map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let dir = MapContinuationDir::from_code_str(direction)
        .ok_or_else(|| to_py_err(format!("Invalid continuation direction '{direction}'")))?;
    let override_preset = match preset_override.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => Some(
            SectorPreset::from_code_str(raw)
                .ok_or_else(|| to_py_err(format!("Unknown continuation preset '{raw}'")))?,
        ),
        None => None,
    };
    let outcome =
        continue_procedural_map_with_preset(&map, dir, seed, override_preset).map_err(to_py_err)?;
    serde_json::to_string(&outcome.map).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Switches `active_floor` on a multi-floor procedural sector and syncs top-level wall/prop/surface mirrors.
#[pyfunction]
pub fn switch_tactical_map_floor_json(map_json: &str, target_floor_index: i32) -> PyResult<String> {
    let mut map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    if !map.switch_active_floor(target_floor_index) {
        return Err(to_py_err(format!(
            "Floor {target_floor_index} not present on tactical map '{}'",
            map.map_id
        )));
    }
    serde_json::to_string(&map).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Extracts a semantic [`satya_maps::SectorNarrativeManifest`] JSON overview from a tactical map.
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

fn serialize_enum_code<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn lookup_firearm_flexible(
    id: &str,
) -> Option<&'static satya_engine::ballistics::GranularFirearmSpec> {
    lookup_firearm_preset(id).or_else(|| {
        let norm = id.trim().to_ascii_lowercase().replace(['-', ' '], "_");
        lookup_firearm_preset(&norm)
    })
}

fn parse_movement_gait(raw: &str) -> PyResult<CombatMovementGait> {
    let norm = raw.trim().to_ascii_uppercase().replace(['-', ' '], "_");
    match norm.as_str() {
        "PRONE" | "PRONE_LOW_CRAWL" | "CRAWL" => Ok(CombatMovementGait::ProneLowCrawl),
        "STEALTH" | "STEALTH_CREEP" | "STEALTH_CREEP_CROUCH_WALK" | "CROUCH_WALK" | "CROUCH" => {
            Ok(CombatMovementGait::StealthCreepCrouchWalk)
        }
        "COMBAT_GLIDE" | "COMBAT_GLIDE_TACTICAL_WALK" | "TACTICAL_WALK" | "WALK" => {
            Ok(CombatMovementGait::CombatGlideTacticalWalk)
        }
        "BOUNDING_RUN" | "BOUNDING_RUN_HUSTLE" | "HUSTLE" | "RUN" => {
            Ok(CombatMovementGait::BoundingRunHustle)
        }
        "SPRINT" | "ALL_OUT_SPRINT" => Ok(CombatMovementGait::AllOutSprint),
        _ => Err(to_py_err(format!("Unknown movement gait '{raw}'"))),
    }
}

fn parse_ordnance_code(raw: &str) -> PyResult<ExplosiveOrdnanceId> {
    let trimmed = raw.trim();
    let norm = trimmed.to_ascii_uppercase().replace(['-', ' '], "_");
    ExplosiveOrdnanceId::from_code_str(trimmed)
        .or_else(|| ExplosiveOrdnanceId::from_code_str(&norm))
        .ok_or_else(|| to_py_err(format!("Unknown explosive ordnance '{raw}'")))
}

fn floor_occupied_tokens_m(
    map: &GmMapUpdateRequest,
    exclude_entity_id: Option<&str>,
) -> Vec<(f32, f32)> {
    let multi = map.has_multiple_floors();
    map.tokens
        .iter()
        .filter(|t| Some(t.entity_id.as_str()) != exclude_entity_id)
        .filter(|t| !multi || map.token_floor(&t.entity_id) == map.active_floor)
        .map(|t| {
            let (tx_cm, ty_cm) = map.token_pos_cm(t);
            (tx_cm as f32 / 100.0, ty_cm as f32 / 100.0)
        })
        .collect()
}

/// Evaluates continuous ballistic range, cover, multi-barrier wall/prop perforation, and residual
/// impact between `(x1_m, y1_m)` and `(x2_m, y2_m)`.
#[pyfunction]
#[pyo3(signature = (map_json, x1_m, y1_m, x2_m, y2_m, firearm_preset_id="hk416_carbine_556", net_successes=0, hit_frame_or_stud=false, floor_index=None))]
#[expect(clippy::too_many_arguments)]
pub fn evaluate_ballistic_shot_on_map_json(
    map_json: &str,
    x1_m: f32,
    y1_m: f32,
    x2_m: f32,
    y2_m: f32,
    firearm_preset_id: &str,
    net_successes: i32,
    hit_frame_or_stud: bool,
    floor_index: Option<i32>,
) -> PyResult<String> {
    let firearm = lookup_firearm_flexible(firearm_preset_id)
        .ok_or_else(|| to_py_err(format!("Unknown firearm preset '{firearm_preset_id}'")))?;
    let map = map_for_floor(
        parse_flexible_map_json(map_json).map_err(to_py_err)?,
        floor_index,
    )?;
    let scene = TacticalSceneView::from_map(&map);
    let eval = evaluate_ballistic_shot(BallisticShotQuery {
        scene: &scene,
        attacker_m: (x1_m, y1_m),
        target_m: (x2_m, y2_m),
        firearm,
        net_successes,
        hit_frame_or_stud,
    });
    serde_json::to_string(&eval).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Computes continuous `0.25m` 16-directional gait reachability strips around `(x_m, y_m)`.
#[pyfunction]
#[pyo3(signature = (map_json, x_m, y_m, spd_eff=10, hgt_m=1.75, gait_code="COMBAT_GLIDE", tick_budget=4, floor_index=None, exclude_entity_id=None))]
#[expect(clippy::too_many_arguments)]
pub fn compute_reachable_zone_json(
    map_json: &str,
    x_m: f32,
    y_m: f32,
    spd_eff: i32,
    hgt_m: f64,
    gait_code: &str,
    tick_budget: u32,
    floor_index: Option<i32>,
    exclude_entity_id: Option<&str>,
) -> PyResult<String> {
    let gait = parse_movement_gait(gait_code)?;
    let map = map_for_floor(
        parse_flexible_map_json(map_json).map_err(to_py_err)?,
        floor_index,
    )?;
    let scene = TacticalSceneView::from_map(&map);
    let occupied = floor_occupied_tokens_m(&map, exclude_entity_id);
    let budget = tick_budget.max(1);
    let field = compute_reachable_continuous(
        &scene,
        &occupied,
        GaitReachQuery {
            start_m: (x_m, y_m),
            spd_eff,
            hgt_m: hgt_m.max(0.5),
            gait,
            tick_budget: budget,
        },
    );
    let strips = field.merged_row_runs();
    let max_nominal_reach_m = ((field.m_per_tick * budget as f32) * 100.0).round() / 100.0;
    let reachable_cell_count = field
        .cost_mm
        .iter()
        .filter(|&&c| c != i32::MAX && c > 0)
        .count();
    let out = json!({
        "floor_index": map.active_floor,
        "start_m": [x_m, y_m],
        "gait": gait,
        "tick_budget": budget,
        "step_m": field.step_m,
        "m_per_tick": field.m_per_tick,
        "max_nominal_reach_m": max_nominal_reach_m,
        "reachable_cell_count": reachable_cell_count,
        "strips": strips,
    });
    serde_json::to_string(&out).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Evaluates indoor blast overpressure, barrier-shielded shrapnel, and structural wall breaches.
#[pyfunction]
#[pyo3(signature = (map_json, x_m, y_m, ordnance_code="M67_FRAG", demolitions_net_successes=0, apply_wall_breaches=true, floor_index=None))]
pub fn evaluate_spatial_blast_and_breach_json(
    map_json: &str,
    x_m: f32,
    y_m: f32,
    ordnance_code: &str,
    demolitions_net_successes: i32,
    apply_wall_breaches: bool,
    floor_index: Option<i32>,
) -> PyResult<String> {
    let ordnance = parse_ordnance_code(ordnance_code)?;
    let mut map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let target_floor = floor_index.unwrap_or(map.active_floor);
    if !map.floors.is_empty() && !map.has_floor(target_floor) {
        return Err(to_py_err(format!(
            "Floor {target_floor} not present on tactical map '{}'",
            map.map_id
        )));
    }
    let outcome = evaluate_spatial_blast_and_breach(
        &mut map,
        (x_m, y_m),
        target_floor,
        ordnance,
        demolitions_net_successes,
        apply_wall_breaches,
    );
    let out = json!({
        "outcome": outcome,
        "updated_map": map,
    });
    serde_json::to_string(&out).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

fn require_i32_field(val: &Value, key: &str) -> PyResult<i32> {
    val.get(key)
        .and_then(Value::as_i64)
        .map(|v| v as i32)
        .ok_or_else(|| to_py_err(format!("Missing integer field '{key}'")))
}

fn require_f32_field(val: &Value, key: &str) -> PyResult<f32> {
    val.get(key)
        .and_then(Value::as_f64)
        .map(|v| v as f32)
        .ok_or_else(|| to_py_err(format!("Missing numeric field '{key}'")))
}

fn parse_edge_side_field(val: &Value) -> PyResult<EdgeSide> {
    let raw = val
        .get("side")
        .and_then(Value::as_str)
        .ok_or_else(|| to_py_err("Missing string field 'side'"))?;
    EdgeSide::from_code_str(raw).ok_or_else(|| to_py_err(format!("Unknown edge side '{raw}'")))
}

/// Applies an authoritative vector map edit (`TOGGLE_DOOR`, `SET_EDGE`, `REMOVE_EDGE`,
/// `PLACE_PROP`, `REMOVE_PROP`, `SET_BLACKOUT`, `BLAST_OR_BREACH`) and returns updated map JSON.
#[pyfunction]
#[pyo3(signature = (map_json, action_json, floor_index=None))]
pub fn edit_tactical_map_feature_json(
    map_json: &str,
    action_json: &str,
    floor_index: Option<i32>,
) -> PyResult<String> {
    let action_val: Value = serde_json::from_str(action_json)
        .map_err(|e| to_py_err(format!("Invalid action JSON: {e}")))?;
    let action_name = action_val
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| to_py_err("Missing string field 'action'"))?
        .trim()
        .to_ascii_uppercase()
        .replace(['-', ' '], "_");

    let raw_map = parse_flexible_map_json(map_json).map_err(to_py_err)?;
    let orig_floor = raw_map.active_floor;
    let mut map = map_for_floor(raw_map, floor_index)?;
    let edited_floor = map.active_floor;

    let edit_summary: Value = match action_name.as_str() {
        "TOGGLE_DOOR" => {
            let x = require_i32_field(&action_val, "x")?;
            let y = require_i32_field(&action_val, "y")?;
            let side = parse_edge_side_field(&action_val)?;
            let edge = map
                .wall_edges
                .iter_mut()
                .find(|e| e.x == x && e.y == y && e.side == side)
                .ok_or_else(|| {
                    to_py_err(format!("No edge found at ({x}, {y}, {})", side.as_str()))
                })?;
            let next_kind = match edge.kind {
                WallEdgeKind::DoorClosed => WallEdgeKind::DoorOpen,
                WallEdgeKind::DoorOpen => WallEdgeKind::DoorClosed,
                other => {
                    return Err(to_py_err(format!(
                        "Edge at ({x}, {y}, {}) is {}, not a door",
                        side.as_str(),
                        other.as_str()
                    )));
                }
            };
            edge.kind = next_kind;
            json!({
                "x": x,
                "y": y,
                "side": side.as_str(),
                "kind": next_kind.as_str(),
            })
        }
        "SET_EDGE" => {
            let x = require_i32_field(&action_val, "x")?;
            let y = require_i32_field(&action_val, "y")?;
            let side = parse_edge_side_field(&action_val)?;
            let in_bounds = match side {
                EdgeSide::West => x >= 0 && x <= map.grid_width && y >= 0 && y < map.grid_height,
                EdgeSide::North => x >= 0 && x < map.grid_width && y >= 0 && y <= map.grid_height,
            };
            if !in_bounds {
                return Err(to_py_err(format!(
                    "Edge coordinates ({x}, {y}, {}) out of map bounds ({}x{})",
                    side.as_str(),
                    map.grid_width,
                    map.grid_height
                )));
            }
            let kind_raw = action_val
                .get("kind")
                .or_else(|| action_val.get("edge_kind"))
                .and_then(Value::as_str)
                .ok_or_else(|| to_py_err("Missing 'kind' for SET_EDGE"))?;
            let kind = WallEdgeKind::from_code_str(kind_raw)
                .ok_or_else(|| to_py_err(format!("Unknown wall edge kind '{kind_raw}'")))?;
            let mat_raw = action_val
                .get("material")
                .and_then(Value::as_str)
                .unwrap_or("DRYWALL");
            let material = MaterialClass::from_code_str(mat_raw)
                .ok_or_else(|| to_py_err(format!("Unknown material class '{mat_raw}'")))?;
            let new_edge = MapWallEdge {
                x,
                y,
                side,
                kind,
                material,
            };
            if let Some(slot) = map
                .wall_edges
                .iter_mut()
                .find(|e| e.x == x && e.y == y && e.side == side)
            {
                *slot = new_edge;
            } else {
                map.wall_edges.push(new_edge);
            }
            json!({
                "x": x,
                "y": y,
                "side": side.as_str(),
                "kind": kind.as_str(),
                "material": material.as_str(),
            })
        }
        "REMOVE_EDGE" => {
            let x = require_i32_field(&action_val, "x")?;
            let y = require_i32_field(&action_val, "y")?;
            let side = parse_edge_side_field(&action_val)?;
            let idx = map
                .wall_edges
                .iter()
                .position(|e| e.x == x && e.y == y && e.side == side)
                .ok_or_else(|| {
                    to_py_err(format!("No edge found at ({x}, {y}, {})", side.as_str()))
                })?;
            let removed = map.wall_edges.remove(idx);
            json!({
                "removed_kind": removed.kind.as_str(),
                "x": x,
                "y": y,
                "side": side.as_str(),
            })
        }
        "PLACE_PROP" => {
            let prop_raw = action_val
                .get("prop_kind")
                .or_else(|| action_val.get("kind"))
                .and_then(Value::as_str)
                .ok_or_else(|| to_py_err("Missing 'prop_kind' for PLACE_PROP"))?;
            let norm_prop = prop_raw
                .trim()
                .to_ascii_uppercase()
                .replace(['-', ' '], "_");
            let kind = PropKind::from_code_str(prop_raw)
                .or_else(|| PropKind::from_code_str(&norm_prop))
                .ok_or_else(|| to_py_err(format!("Unknown prop kind '{prop_raw}'")))?;
            let x_m = require_f32_field(&action_val, "x_m")?;
            let y_m = require_f32_field(&action_val, "y_m")?;
            let rotation_deg = action_val
                .get("rotation_deg")
                .and_then(Value::as_i64)
                .unwrap_or(0) as i32;
            let length_cm = action_val
                .get("length_cm")
                .and_then(Value::as_i64)
                .map(|v| v as i32);
            let variant_seed = action_val
                .get("variant_seed")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            let placement = make_prop_at_cm(
                kind,
                (x_m * 100.0).round() as i32,
                (y_m * 100.0).round() as i32,
                rotation_deg,
                length_cm,
                variant_seed,
            );
            let occupied = floor_occupied_tokens_m(&map, None);
            let board_size_m = map.board_size_m();
            let placed = place_prop_on_map(
                &mut map.props,
                &map.wall_edges,
                &occupied,
                board_size_m,
                placement,
            )
            .map_err(to_py_err)?;
            json!({
                "placed_kind": placed.kind.as_str(),
                "x_m": placed.center_m().0,
                "y_m": placed.center_m().1,
                "rotation_deg": placed.rotation_deg,
            })
        }
        "REMOVE_PROP" => {
            let x_m = require_f32_field(&action_val, "x_m")?;
            let y_m = require_f32_field(&action_val, "y_m")?;
            let removed = remove_prop_at_point(&mut map.props, x_m, y_m)
                .ok_or_else(|| to_py_err(format!("No prop found near ({x_m:.2}m, {y_m:.2}m)")))?;
            json!({
                "removed_kind": removed.kind.as_str(),
                "x_m": removed.center_m().0,
                "y_m": removed.center_m().1,
            })
        }
        "SET_BLACKOUT" => {
            let blackout_active = action_val
                .get("blackout_active")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let explicit_delta = action_val
                .get("ambient_light_delta_ext")
                .and_then(Value::as_i64)
                .map(|v| v as i32);
            map.ambient_light_delta_ext = explicit_delta.unwrap_or(if blackout_active {
                map.ambient_light_delta_ext.max(3)
            } else {
                0
            });
            for node in &mut map.surveillance_nodes {
                node.powered = !blackout_active;
            }
            json!({
                "blackout_active": blackout_active,
                "ambient_light_delta_ext": map.ambient_light_delta_ext,
                "surveillance_nodes_updated": map.surveillance_nodes.len(),
            })
        }
        "BLAST_OR_BREACH" => {
            let x_m = require_f32_field(&action_val, "x_m")?;
            let y_m = require_f32_field(&action_val, "y_m")?;
            let ordnance_raw = action_val
                .get("ordnance_code")
                .and_then(Value::as_str)
                .unwrap_or("M67_FRAG");
            let ordnance = parse_ordnance_code(ordnance_raw)?;
            let demos = action_val
                .get("demolitions_net_successes")
                .and_then(Value::as_i64)
                .unwrap_or(0) as i32;
            let apply_breach = action_val
                .get("apply_wall_breaches")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let outcome = evaluate_spatial_blast_and_breach(
                &mut map,
                (x_m, y_m),
                edited_floor,
                ordnance,
                demos,
                apply_breach,
            );
            json!({ "blast_outcome": outcome })
        }
        other => {
            return Err(to_py_err(format!("Unknown map edit action '{other}'")));
        }
    };

    map.sync_active_floor_snapshot();
    if !map.floors.is_empty() && map.active_floor != orig_floor {
        let _ = map.switch_active_floor(orig_floor);
    }

    let out = json!({
        "action": action_name,
        "edited_floor_index": edited_floor,
        "edit_summary": edit_summary,
        "updated_map": map,
    });
    serde_json::to_string(&out).map_err(|e| to_py_err(format!("JSON encode failed: {e}")))
}

/// Registers tactical map spatial bindings on the `satya_py` Python module.
pub fn register_tactical_map_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_tactical_token_collision, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_tactical_token_destination_cm, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_tactical_los_and_cover, m)?)?;
    m.add_function(wrap_pyfunction!(inspect_tactical_map_point, m)?)?;
    m.add_function(wrap_pyfunction!(
        generate_procedural_tactical_sector_json,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(continue_procedural_tactical_map_json, m)?)?;
    m.add_function(wrap_pyfunction!(switch_tactical_map_floor_json, m)?)?;
    m.add_function(wrap_pyfunction!(extract_sector_narrative_manifest_json, m)?)?;
    m.add_function(wrap_pyfunction!(
        evaluate_passive_movement_perceptions_json,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(evaluate_ballistic_shot_on_map_json, m)?)?;
    m.add_function(wrap_pyfunction!(compute_reachable_zone_json, m)?)?;
    m.add_function(wrap_pyfunction!(
        evaluate_spatial_blast_and_breach_json,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(edit_tactical_map_feature_json, m)?)?;
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
        assert!(
            !manifest_val["walkable_anchors"]
                .as_array()
                .unwrap()
                .is_empty()
        );

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

        let east_json =
            continue_procedural_tactical_map_json(&sector_json, "EAST", Some(99), None).unwrap();
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
        assert!(check_tactical_token_collision(&py_dict_json, 0.10, 5.0, 0.30, None).unwrap());
        let mut resolved = None;
        for gy in 2..9 {
            for gx in 2..10 {
                let cx = gx * 100 + 50;
                let cy = gy * 100 + 50;
                if let Ok(pt) =
                    resolve_tactical_token_destination_cm(&py_dict_json, "tok-vance", cx, cy, None)
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

    #[test]
    fn test_wave1_multifloor_parse_auto_scale_and_floor_spatial_queries() {
        // 1. Omitted dimensions + omitted scale resolve via MapScalePreset::Auto
        let auto_json = generate_procedural_tactical_sector_json(
            "map-auto",
            "hospital",
            Some("Auto Hospital"),
            None,
            None,
            51,
            None,
            None,
            None,
        )
        .unwrap();
        let auto_val: Value = serde_json::from_str(&auto_json).unwrap();
        let w = auto_val["grid_width"].as_i64().unwrap();
        let h = auto_val["grid_height"].as_i64().unwrap();
        assert!(w >= 16 && h >= 12);
        assert!(auto_val["floors"].as_array().unwrap().len() >= 2);

        // 2. parse_flexible_map_json merges token_visuals without clobbering floor_index
        let merged_json = serde_json::json!({
            "map_id": "map-merge",
            "title": "Hospital Floors",
            "grid_width": 20,
            "grid_height": 16,
            "active_floor": 0,
            "floors": [
                {"floor_index": 0, "label": "1F", "wall_edges": [], "props": [], "surfaces": []},
                {"floor_index": 1, "label": "2F", "wall_edges": [], "props": [], "surfaces": []}
            ],
            "tokens": [
                {
                    "entity_id": "tok-f1",
                    "label": "Nurse Lin",
                    "x": 3,
                    "y": 4,
                    "pos_x_cm": 350,
                    "pos_y_cm": 450
                }
            ],
            "token_visuals": [
                {
                    "entity_id": "tok-f1",
                    "floor_index": 1,
                    "color_hex": "#34d399"
                }
            ]
        })
        .to_string();
        let merged = parse_flexible_map_json(&merged_json).unwrap();
        assert_eq!(merged.token_floor("tok-f1"), 1);

        // 3. Explicit floor_index on spatial queries hits floor 1 vs floor 0 geometry cleanly
        let collides_f1 =
            check_tactical_token_collision(&auto_json, 0.05, 0.05, 0.30, Some(1)).unwrap();
        assert!(collides_f1);

        let f1_parsed =
            map_for_floor(parse_flexible_map_json(&auto_json).unwrap(), Some(1)).unwrap();
        let mut f1_clear = None;
        for gy in 2..((f1_parsed.grid_height - 2).max(3)) {
            for gx in 2..((f1_parsed.grid_width - 2).max(3)) {
                let cx = gx * 100 + 50;
                let cy = gy * 100 + 50;
                if let Ok(pt) =
                    resolve_tactical_token_destination_cm(&auto_json, "tok-f1", cx, cy, Some(1))
                {
                    f1_clear = Some(pt);
                    break;
                }
            }
            if f1_clear.is_some() {
                break;
            }
        }
        assert!(f1_clear.is_some());

        Python::initialize();
        Python::attach(|py| {
            let los_f1 =
                evaluate_tactical_los_and_cover(py, &auto_json, 2.5, 2.5, 8.5, 2.5, Some(1))
                    .unwrap();
            assert!(los_f1.bind(py).contains("cover_grade").unwrap());

            let hover_f1 =
                inspect_tactical_map_point(py, &auto_json, "Auto Hospital", 2.5, 2.5, Some(1))
                    .unwrap()
                    .expect("hover on floor 1 returns inspection dict");
            assert!(hover_f1.bind(py).contains("category").unwrap());
        });
    }

    #[test]
    fn test_parse_flexible_map_json_normalizes_legacy_concrete_material() {
        let legacy_json = serde_json::json!({
            "map_id": "map-legacy-concrete",
            "title": "Legacy Concrete Clinic",
            "grid_width": 12,
            "grid_height": 10,
            "wall_edges": [
                {"x": 5, "y": 5, "side": "WEST", "kind": "WALL", "material": "CONCRETE"}
            ],
            "props": [],
            "floors": [
                {
                    "floor_index": 0,
                    "label": "1F",
                    "wall_edges": [
                        {"x": 2, "y": 3, "side": "NORTH", "kind": "WALL", "material": "CONCRETE"}
                    ],
                    "props": [],
                    "surfaces": []
                }
            ],
            "tokens": []
        })
        .to_string();

        let parsed = parse_flexible_map_json(&legacy_json).unwrap();
        assert_eq!(
            parsed.wall_edges[0].material,
            satya_schema::MaterialClass::ConcreteBlock
        );
        assert_eq!(
            parsed.floors[0].wall_edges[0].material,
            satya_schema::MaterialClass::ConcreteBlock
        );
    }

    #[test]
    fn test_wave5_py_spatial_ballistics_reach_blast_and_edits() {
        let bunker_json = generate_procedural_tactical_sector_json(
            "map-bunker-py",
            "corporate_server_bunker",
            Some("Substation QPU Bunker"),
            Some(20),
            Some(16),
            2035,
            None,
            None,
            None,
        )
        .unwrap();
        let bunker_val: Value = serde_json::from_str(&bunker_json).unwrap();
        let fz = &bunker_val["faraday_zones"][0];
        let fz_x = ((fz["x0_cm"].as_f64().unwrap() + fz["x1_cm"].as_f64().unwrap()) / 200.0) as f32;
        let fz_y = ((fz["y0_cm"].as_f64().unwrap() + fz["y1_cm"].as_f64().unwrap()) / 200.0) as f32;

        Python::initialize();
        Python::attach(|py| {
            let pt = inspect_tactical_map_point(py, &bunker_json, "Bunker", fz_x, fz_y, Some(0))
                .unwrap()
                .expect("point inside Faraday zone inspects cleanly");
            let pt_dict = pt.bind(py);
            assert!(pt_dict.get_item("in_faraday_zone").unwrap().unwrap().extract::<bool>().unwrap());
            assert!(pt_dict.contains("acoustic_surface").unwrap());
            assert!(pt_dict.contains("traction_tier").unwrap());
            assert!(pt_dict.contains("ambient_light_delta_ext").unwrap());
        });

        let base_map = serde_json::json!({
            "map_id": "map-spatial-edit",
            "title": "Clinic Annex Bay",
            "grid_width": 12,
            "grid_height": 10,
            "wall_edges": [],
            "props": [],
            "surfaces": [],
            "surveillance_nodes": [
                {
                    "node_id": "cam-1",
                    "label": "Door Dome Cam",
                    "node_kind_code": "DOME_CAMERA",
                    "floor_index": 0,
                    "x_cm": 600,
                    "y_cm": 200,
                    "facing_deg": 90,
                    "fov_deg": 90,
                    "range_cm": 600,
                    "powered": true,
                    "visibility": "PUBLIC"
                }
            ],
            "tokens": []
        })
        .to_string();

        let edge_res: Value = serde_json::from_str(
            &edit_tactical_map_feature_json(
                &base_map,
                r#"{"action":"SET_EDGE","x":6,"y":5,"side":"NORTH","kind":"WALL","material":"DRYWALL"}"#,
                None,
            )
            .unwrap(),
        )
        .unwrap();
        let after_wall = serde_json::to_string(&edge_res["updated_map"]).unwrap();

        let shot: Value = serde_json::from_str(
            &evaluate_ballistic_shot_on_map_json(
                &after_wall,
                6.5,
                4.2,
                6.5,
                5.8,
                "hk416_carbine_556",
                2,
                false,
                None,
            )
            .unwrap(),
        )
        .unwrap();
        assert!(shot["weapon_name"].as_str().unwrap().contains("HK416"));
        assert!(shot["effective_delta_ext"].as_i64().is_some());

        let reach: Value = serde_json::from_str(
            &compute_reachable_zone_json(&after_wall, 2.5, 2.5, 11, 1.75, "COMBAT_GLIDE", 3, None, None)
                .unwrap(),
        )
        .unwrap();
        assert!(reach["reachable_cell_count"].as_u64().unwrap() > 0);
        assert!(!reach["strips"].as_array().unwrap().is_empty());

        let door_set: Value = serde_json::from_str(
            &edit_tactical_map_feature_json(
                &after_wall,
                r#"{"action":"SET_EDGE","x":4,"y":5,"side":"NORTH","kind":"DOOR_CLOSED","material":"SOLID_WOOD"}"#,
                None,
            )
            .unwrap(),
        )
        .unwrap();
        let toggled: Value = serde_json::from_str(
            &edit_tactical_map_feature_json(
                &serde_json::to_string(&door_set["updated_map"]).unwrap(),
                r#"{"action":"TOGGLE_DOOR","x":4,"y":5,"side":"NORTH"}"#,
                None,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(toggled["edit_summary"]["kind"], "DOOR_OPEN");

        let placed: Value = serde_json::from_str(
            &edit_tactical_map_feature_json(
                &after_wall,
                r#"{"action":"PLACE_PROP","prop_kind":"WOOD_CRATE","x_m":3.5,"y_m":3.5,"rotation_deg":15}"#,
                None,
            )
            .unwrap(),
        )
        .unwrap();
        let removed: Value = serde_json::from_str(
            &edit_tactical_map_feature_json(
                &serde_json::to_string(&placed["updated_map"]).unwrap(),
                r#"{"action":"REMOVE_PROP","x_m":3.5,"y_m":3.5}"#,
                None,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(removed["edit_summary"]["removed_kind"], "WOOD_CRATE");

        let blackout: Value = serde_json::from_str(
            &edit_tactical_map_feature_json(
                &after_wall,
                r#"{"action":"SET_BLACKOUT","blackout_active":true}"#,
                None,
            )
            .unwrap(),
        )
        .unwrap();
        assert!(blackout["updated_map"]["ambient_light_delta_ext"].as_i64().unwrap() >= 3);
        assert_eq!(blackout["updated_map"]["surveillance_nodes"][0]["powered"], false);

        let blast: Value = serde_json::from_str(
            &evaluate_spatial_blast_and_breach_json(&after_wall, 6.5, 4.9, "C4", 2, true, None)
                .unwrap(),
        )
        .unwrap();
        assert!(blast["outcome"]["breached_edges_count"].as_u64().unwrap() >= 1);
    }
}
