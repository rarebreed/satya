//! PyO3 bindings for continuous metric tactical map generation, token collision/destination
//! validation, analytical LoS/cover evaluation, and cursor hover inspection powered by
//! [`satya_maps`].

use pyo3::prelude::*;
use pyo3::types::PyDict;
use satya_engine::ballistics::lookup_firearm_preset;
use satya_engine::environment_vehicles::CombatMovementGait;
use satya_engine::explosives_and_arsenal::ExplosiveOrdnanceId;
use satya_maps::barrier_ballistics::BallisticShotQuery;
use satya_maps::map_dressing::{PROP_CATALOG, PropObstruction, make_prop_at_cm};
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
    CodedEnum, EconomicTier, EdgeSide, GmMapUpdateRequest, MapContinuationDir, MapScalePreset,
    MapWallEdge, MaterialClass, PropKind, TokenVisualSpec, WallEdgeKind, ZoningContext,
    default_floor_badge,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::py_bridge::{
    PyJsonDict, PyJsonInput, PyJsonList, PyListInput, normalize_code, normalize_material_code,
    parse_coded_enum, py_to_json_value, to_py_err, to_py_object,
};

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

fn infer_model_preset(label: &str, is_hidden: bool) -> &'static str {
    let low = label.to_ascii_lowercase();
    if low.contains("valkyrie") || low.contains("sara") {
        "valkyrie_rig"
    } else if low.contains("recon") || low.contains("marcus") || low.contains("vance") {
        "recon_mk4"
    } else if is_hidden
        || low.contains("sentry")
        || low.contains("secops")
        || low.contains("hostile")
    {
        "secops_sentry"
    } else {
        "operative_light"
    }
}

fn strip_null_fields(map: &mut Map<String, Value>, keys: &[&str]) {
    for &k in keys {
        if map.get(k).is_some_and(Value::is_null) {
            map.remove(k);
        }
    }
}

/// Normalizes flexible Python `map_state` values into a canonical [`GmMapUpdateRequest`]
/// without mutating locked `v1` wire structs.
pub(crate) fn parse_flexible_map_value(mut root: Value) -> Result<GmMapUpdateRequest, String> {
    let obj = root
        .as_object_mut()
        .ok_or_else(|| "Tactical map payload must be a dict/object".to_string())?;

    strip_null_fields(
        obj,
        &[
            "map_id",
            "title",
            "image_url",
            "grid_width",
            "grid_height",
            "cell_size_cm",
            "tiles",
            "wall_edges",
            "props",
            "tokens",
            "token_visuals",
            "surfaces",
            "floors",
            "world_sectors",
            "active_floor",
            "ambient_light_delta_ext",
            "faraday_zones",
            "surveillance_nodes",
        ],
    );

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
    let cell_cm = obj
        .get("cell_size_cm")
        .and_then(Value::as_i64)
        .unwrap_or(100)
        .max(25);
    let cell_cm_i32 = cell_cm as i32;

    struct TokenVisualOverride {
        entity_id: String,
        label: String,
        is_hidden: bool,
        gx: i32,
        gy: i32,
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
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("tok-{idx}"));
                tok.insert("entity_id".to_string(), Value::String(entity_id.clone()));

                let label = tok
                    .get("label")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&entity_id)
                    .to_string();
                tok.insert("label".to_string(), Value::String(label.clone()));

                let gx = tok
                    .get("x")
                    .or_else(|| tok.get("grid_x"))
                    .and_then(Value::as_i64)
                    .unwrap_or_else(|| {
                        tok.get("pos_x_cm")
                            .and_then(Value::as_i64)
                            .map(|cm| (cm / cell_cm).max(0))
                            .unwrap_or(0)
                    });
                let gy = tok
                    .get("y")
                    .or_else(|| tok.get("grid_y"))
                    .and_then(Value::as_i64)
                    .unwrap_or_else(|| {
                        tok.get("pos_y_cm")
                            .and_then(Value::as_i64)
                            .map(|cm| (cm / cell_cm).max(0))
                            .unwrap_or(0)
                    });
                if tok.get("x").is_none_or(Value::is_null) {
                    tok.insert("x".to_string(), json!(gx));
                }
                if tok.get("y").is_none_or(Value::is_null) {
                    tok.insert("y".to_string(), json!(gy));
                }

                let raw_vis = tok
                    .get("visibility")
                    .and_then(Value::as_str)
                    .unwrap_or("PUBLIC")
                    .trim()
                    .to_ascii_uppercase();
                let is_hidden = tok.get("is_hidden").and_then(Value::as_bool).unwrap_or(false)
                    || tok.get("hidden").and_then(Value::as_bool).unwrap_or(false)
                    || tok.get("gm_only").and_then(Value::as_bool).unwrap_or(false)
                    || raw_vis == "GM_ONLY";
                let norm_vis = if is_hidden {
                    "GM_ONLY"
                } else {
                    match raw_vis.as_str() {
                        "GM_ONLY" => "GM_ONLY",
                        "PRIVATE" => "PRIVATE",
                        "HOST_ONLY" => "HOST_ONLY",
                        _ => "PUBLIC",
                    }
                };
                tok.insert("visibility".to_string(), Value::String(norm_vis.to_string()));

                let opt_i32 = |k: &str| tok.get(k).and_then(Value::as_i64).map(|v| v as i32);
                let opt_str = |k: &str| {
                    tok.get(k)
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                };
                let ov = TokenVisualOverride {
                    entity_id,
                    label,
                    is_hidden,
                    gx: gx as i32,
                    gy: gy as i32,
                    model_preset: opt_str("model_preset"),
                    color_hex: opt_str("color_hex"),
                    facing_deg: opt_i32("facing_deg"),
                    elevation_cm: opt_i32("elevation_cm"),
                    floor_index: opt_i32("floor_index"),
                    stance_pose: opt_str("stance_pose"),
                    portrait_url: opt_str("portrait_url"),
                    pos_x_cm: opt_i32("pos_x_cm"),
                    pos_y_cm: opt_i32("pos_y_cm"),
                };
                if ov.pos_x_cm.is_some()
                    || ov.pos_y_cm.is_some()
                    || ov.floor_index.is_some()
                    || ov.elevation_cm.is_some()
                    || ov.model_preset.is_some()
                    || ov.color_hex.is_some()
                    || ov.facing_deg.is_some()
                    || ov.stance_pose.is_some()
                    || ov.portrait_url.is_some()
                {
                    visual_overrides.push(ov);
                }
            }
        }
    }

    normalize_wall_edges_and_props(obj);
    if let Some(floors_val) = obj.get_mut("floors").and_then(Value::as_array_mut) {
        for floor_val in floors_val.iter_mut() {
            if let Some(floor) = floor_val.as_object_mut() {
                strip_null_fields(
                    floor,
                    &["wall_edges", "props", "surfaces", "elevation_cm"],
                );
                if floor.get("label").is_none_or(Value::is_null) {
                    let fidx = floor
                        .get("floor_index")
                        .and_then(Value::as_i64)
                        .unwrap_or(0) as i32;
                    floor.insert("label".to_string(), Value::String(default_floor_badge(fidx)));
                }
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
                    .unwrap_or_else(|| infer_model_preset(&ov.label, ov.is_hidden).to_string()),
                color_hex: ov.color_hex.unwrap_or_else(|| "#38bdf8".to_string()),
                facing_deg: ov.facing_deg.unwrap_or(0),
                elevation_cm: ov.elevation_cm.unwrap_or(0),
                floor_index: ov.floor_index.unwrap_or(map.active_floor),
                stance_pose: ov.stance_pose.unwrap_or_else(|| "idle".to_string()),
                portrait_url: ov.portrait_url,
                pos_x_cm: ov
                    .pos_x_cm
                    .or_else(|| Some(ov.gx * cell_cm_i32 + cell_cm_i32 / 2)),
                pos_y_cm: ov
                    .pos_y_cm
                    .or_else(|| Some(ov.gy * cell_cm_i32 + cell_cm_i32 / 2)),
            });
        }
    }

    if !map.tokens.is_empty() {
        map.token_visuals
            .retain(|v| map.tokens.iter().any(|t| t.entity_id == v.entity_id));
    }

    Ok(map)
}

fn parse_flexible_map_obj(obj: &Bound<'_, PyAny>) -> PyResult<GmMapUpdateRequest> {
    let val = py_to_json_value(obj, "tactical map")?;
    parse_flexible_map_value(val).map_err(to_py_err)
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
#[pyo3(signature = (map_state, x_m, y_m, radius_m=0.30, floor_index=None))]
pub fn check_tactical_token_collision(
    map_state: PyJsonInput<'_>,
    x_m: f32,
    y_m: f32,
    radius_m: f32,
    floor_index: Option<i32>,
) -> PyResult<bool> {
    let map = map_for_floor(parse_flexible_map_obj(&map_state)?, floor_index)?;
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
#[pyo3(signature = (map_state, moving_entity_id, target_x_cm, target_y_cm, floor_index=None))]
pub fn resolve_tactical_token_destination_cm(
    map_state: PyJsonInput<'_>,
    moving_entity_id: &str,
    target_x_cm: i32,
    target_y_cm: i32,
    floor_index: Option<i32>,
) -> PyResult<(i32, i32)> {
    let raw_map = parse_flexible_map_obj(&map_state)?;
    let target_floor = floor_index
        .or_else(|| (!raw_map.floors.is_empty()).then(|| raw_map.token_floor(moving_entity_id)));
    let map = map_for_floor(raw_map, target_floor)?;
    resolve_token_destination_cm(&map, moving_entity_id, target_x_cm, target_y_cm)
        .map_err(to_py_err)
}

/// Evaluates continuous Line-of-Sight, cover grade, and recommended `Δ_ext` between two positions in meters.
#[pyfunction]
#[pyo3(signature = (map_state, x1_m, y1_m, x2_m, y2_m, floor_index=None))]
pub fn evaluate_tactical_los_and_cover(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
    x1_m: f32,
    y1_m: f32,
    x2_m: f32,
    y2_m: f32,
    floor_index: Option<i32>,
) -> PyResult<PyJsonDict> {
    let map = map_for_floor(parse_flexible_map_obj(&map_state)?, floor_index)?;
    let scene = TacticalSceneView::from_map(&map);
    let eval = evaluate_los_continuous(&scene, (x1_m, y1_m), (x2_m, y2_m));
    let d = PyDict::new(py);
    d.set_item("distance_m", eval.distance_m)?;
    d.set_item("has_los", eval.has_los)?;
    d.set_item("cover_grade", eval.cover_grade.as_str())?;
    d.set_item("cover_label", eval.cover_grade.label())?;
    d.set_item("recommended_delta_ext", eval.recommended_delta_ext)?;
    Ok(d.into_any().unbind())
}

/// Inspects any continuous `(x_m, y_m)` map coordinate (token, wall/door, prop, or ground surface).
#[pyfunction]
#[pyo3(signature = (map_state, theme_name, x_m, y_m, floor_index=None))]
pub fn inspect_tactical_map_point(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
    theme_name: &str,
    x_m: f32,
    y_m: f32,
    floor_index: Option<i32>,
) -> PyResult<Option<PyJsonDict>> {
    let map = map_for_floor(parse_flexible_map_obj(&map_state)?, floor_index)?;
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
    d.set_item("acoustic_surface", info.acoustic_surface.map(CodedEnum::code))?;
    d.set_item("acoustic_stealth_delta_ext", info.acoustic_stealth_delta_ext)?;
    d.set_item("traction_tier", info.traction_tier.map(CodedEnum::code))?;
    d.set_item("in_faraday_zone", info.in_faraday_zone)?;
    d.set_item("ambient_light_delta_ext", info.ambient_light_delta_ext)?;
    Ok(Some(d.into_any().unbind()))
}

/// Generates a procedural multi-floor tactical sector returned as a native Python dict.
#[pyfunction]
#[pyo3(signature = (map_id, preset_or_theme, title=None, width=None, height=None, seed=42, economic_tier=None, zoning=None, scale=None))]
pub fn generate_procedural_sector(
    py: Python<'_>,
    map_id: &str,
    preset_or_theme: &str,
    title: Option<&str>,
    width: Option<i32>,
    height: Option<i32>,
    seed: i64,
    economic_tier: Option<&str>,
    zoning: Option<&str>,
    scale: Option<&str>,
) -> PyResult<PyJsonDict> {
    let preset = SectorPreset::from_code_str(preset_or_theme)
        .ok_or_else(|| to_py_err(format!("Unknown sector preset '{preset_or_theme}'")))?;
    let tier_opt = economic_tier.and_then(EconomicTier::from_code_str);
    let zoning_opt = zoning.and_then(ZoningContext::from_code_str);
    let effective_scale = scale.map(MapScalePreset::parse_loose).unwrap_or(MapScalePreset::Auto);
    let (default_w, default_h) = preset.resolve_scale_dimensions(effective_scale);
    let req = build_procedural_map_request_with_options(
        map_id,
        preset,
        title,
        width.unwrap_or(default_w),
        height.unwrap_or(default_h),
        seed,
        tier_opt,
        zoning_opt,
    );
    to_py_object(py, &req)
}

/// Stitches an adjacent macro-world neighbor sector in `direction` (`NORTH`/`SOUTH`/`EAST`/`WEST`).
#[pyfunction]
#[pyo3(signature = (map_state, direction, seed=None, preset_override=None))]
pub fn continue_procedural_map(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
    direction: &str,
    seed: Option<i64>,
    preset_override: Option<&str>,
) -> PyResult<PyJsonDict> {
    let map = parse_flexible_map_obj(&map_state)?;
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
    to_py_object(py, &outcome.map)
}

/// Switches `active_floor` on a multi-floor procedural sector and syncs top-level wall/prop/surface mirrors.
#[pyfunction]
#[pyo3(signature = (map_state, target_floor_index))]
pub fn switch_map_floor(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
    target_floor_index: i32,
) -> PyResult<PyJsonDict> {
    let mut map = parse_flexible_map_obj(&map_state)?;
    if !map.switch_active_floor(target_floor_index) {
        return Err(to_py_err(format!(
            "Floor {target_floor_index} not present on tactical map '{}'",
            map.map_id
        )));
    }
    to_py_object(py, &map)
}

/// Extracts a semantic [`satya_maps::SectorNarrativeManifest`] dict from a tactical map.
#[pyfunction]
#[pyo3(signature = (map_state))]
pub fn extract_narrative_manifest(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
) -> PyResult<PyJsonDict> {
    let map = parse_flexible_map_obj(&map_state)?;
    to_py_object(py, &extract_sector_narrative_manifest(&map))
}

/// Evaluates passive movement perceptions (`SIG`/`HEA`/`OLF`/`ITU`) triggered by moving `moved_entity_id`.
#[pyfunction]
#[pyo3(signature = (map_state, moved_entity_id, observers, pins=None))]
pub fn evaluate_passive_perceptions(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
    moved_entity_id: &str,
    observers: PyListInput<'_>,
    pins: Option<PyListInput<'_>>,
) -> PyResult<PyJsonList> {
    let map = parse_flexible_map_obj(&map_state)?;
    let obs_val = py_to_json_value(&observers, "observers")?;
    let parsed_obs =
        super::schema_py::parse_flexible_character_list(obs_val).map_err(to_py_err)?;
    let parsed_pins: Vec<SpatialSensoryPin> =
        super::py_bridge::opt_from_py_object(pins, "sensory pins")?.unwrap_or_default();
    to_py_object(
        py,
        &evaluate_passive_movement_perceptions(&map, moved_entity_id, &parsed_obs, &parsed_pins),
    )
}

fn lookup_firearm_flexible(
    id: &str,
) -> Option<&'static satya_engine::ballistics::GranularFirearmSpec> {
    lookup_firearm_preset(id).or_else(|| {
        let norm = id.trim().to_ascii_lowercase().replace(['-', ' '], "_");
        lookup_firearm_preset(&norm)
    })
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
#[pyo3(signature = (map_state, x1_m, y1_m, x2_m, y2_m, firearm_preset_id="hk416_carbine_556", net_successes=0, hit_frame_or_stud=false, floor_index=None))]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_ballistic_shot(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
    x1_m: f32,
    y1_m: f32,
    x2_m: f32,
    y2_m: f32,
    firearm_preset_id: &str,
    net_successes: i32,
    hit_frame_or_stud: bool,
    floor_index: Option<i32>,
) -> PyResult<PyJsonDict> {
    let firearm = lookup_firearm_flexible(firearm_preset_id)
        .ok_or_else(|| to_py_err(format!("Unknown firearm preset '{firearm_preset_id}'")))?;
    let map = map_for_floor(parse_flexible_map_obj(&map_state)?, floor_index)?;
    let scene = TacticalSceneView::from_map(&map);
    to_py_object(
        py,
        &satya_maps::barrier_ballistics::evaluate_ballistic_shot(BallisticShotQuery {
            scene: &scene,
            attacker_m: (x1_m, y1_m),
            target_m: (x2_m, y2_m),
            firearm,
            net_successes,
            hit_frame_or_stud,
        }),
    )
}

/// Computes continuous `0.25m` 16-directional gait reachability strips around `(x_m, y_m)`.
#[pyfunction]
#[pyo3(signature = (map_state, x_m, y_m, spd_eff=10, hgt_m=1.75, gait_code="COMBAT_GLIDE", tick_budget=4, floor_index=None, exclude_entity_id=None))]
#[allow(clippy::too_many_arguments)]
pub fn compute_reachable_zone(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
    x_m: f32,
    y_m: f32,
    spd_eff: i32,
    hgt_m: f64,
    gait_code: &str,
    tick_budget: u32,
    floor_index: Option<i32>,
    exclude_entity_id: Option<&str>,
) -> PyResult<PyJsonDict> {
    let gait = parse_coded_enum::<CombatMovementGait>(gait_code, "movement gait")?;
    let map = map_for_floor(parse_flexible_map_obj(&map_state)?, floor_index)?;
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
    to_py_object(
        py,
        &json!({
            "floor_index": map.active_floor,
            "start_m": [x_m, y_m],
            "gait": gait,
            "tick_budget": budget,
            "step_m": field.step_m,
            "m_per_tick": field.m_per_tick,
            "max_nominal_reach_m": max_nominal_reach_m,
            "reachable_cell_count": reachable_cell_count,
            "strips": strips,
        }),
    )
}

/// Evaluates indoor blast overpressure, barrier-shielded shrapnel, and structural wall breaches.
#[pyfunction]
#[pyo3(signature = (map_state, x_m, y_m, ordnance_code="M67_FRAG", demolitions_net_successes=0, apply_wall_breaches=true, floor_index=None))]
pub fn evaluate_spatial_blast_and_breach(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
    x_m: f32,
    y_m: f32,
    ordnance_code: &str,
    demolitions_net_successes: i32,
    apply_wall_breaches: bool,
    floor_index: Option<i32>,
) -> PyResult<PyJsonDict> {
    let ordnance = parse_coded_enum::<ExplosiveOrdnanceId>(ordnance_code, "explosive ordnance")?;
    let mut map = parse_flexible_map_obj(&map_state)?;
    let target_floor = floor_index.unwrap_or(map.active_floor);
    if !map.floors.is_empty() && !map.has_floor(target_floor) {
        return Err(to_py_err(format!(
            "Floor {target_floor} not present on tactical map '{}'",
            map.map_id
        )));
    }
    let outcome = satya_maps::barrier_ballistics::evaluate_spatial_blast_and_breach(
        &mut map,
        (x_m, y_m),
        target_floor,
        ordnance,
        demolitions_net_successes,
        apply_wall_breaches,
    );
    to_py_object(py, &json!({ "outcome": outcome, "updated_map": map }))
}

fn default_drywall() -> String {
    "DRYWALL".to_string()
}
fn default_m67() -> String {
    "M67_FRAG".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "SCREAMING_SNAKE_CASE")]
enum MapFeatureEditRequest {
    ToggleDoor {
        x: i32,
        y: i32,
        side: String,
    },
    SetEdge {
        x: i32,
        y: i32,
        side: String,
        #[serde(alias = "edge_kind")]
        kind: String,
        #[serde(default = "default_drywall")]
        material: String,
    },
    RemoveEdge {
        x: i32,
        y: i32,
        side: String,
    },
    PlaceProp {
        #[serde(alias = "kind")]
        prop_kind: String,
        x_m: f32,
        y_m: f32,
        #[serde(default)]
        rotation_deg: i32,
        #[serde(default)]
        length_cm: Option<i32>,
        #[serde(default)]
        variant_seed: u32,
    },
    RemoveProp {
        x_m: f32,
        y_m: f32,
    },
    SetBlackout {
        #[serde(default = "default_true")]
        blackout_active: bool,
        #[serde(default)]
        ambient_light_delta_ext: Option<i32>,
    },
    BlastOrBreach {
        x_m: f32,
        y_m: f32,
        #[serde(default = "default_m67")]
        ordnance_code: String,
        #[serde(default)]
        demolitions_net_successes: i32,
        #[serde(default = "default_true")]
        apply_wall_breaches: bool,
    },
}

/// Applies an authoritative vector map edit (`TOGGLE_DOOR`, `SET_EDGE`, `REMOVE_EDGE`,
/// `PLACE_PROP`, `REMOVE_PROP`, `SET_BLACKOUT`, `BLAST_OR_BREACH`) and returns updated map state.
#[pyfunction]
#[pyo3(signature = (map_state, action, floor_index=None))]
pub fn edit_map_feature(
    py: Python<'_>,
    map_state: PyJsonInput<'_>,
    action: PyJsonInput<'_>,
    floor_index: Option<i32>,
) -> PyResult<PyJsonDict> {
    let mut act_val = py_to_json_value(&action, "map edit action")?;
    let action_name = act_val
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| to_py_err("Missing string field 'action'"))
        .map(normalize_code)?;
    if let Some(obj) = act_val.as_object_mut() {
        obj.insert("action".to_string(), Value::String(action_name.clone()));
    }
    let req: MapFeatureEditRequest = serde_json::from_value(act_val)
        .map_err(|e| to_py_err(format!("Invalid map edit action '{action_name}': {e}")))?;

    let raw_map = parse_flexible_map_obj(&map_state)?;
    let orig_floor = raw_map.active_floor;
    let mut map = map_for_floor(raw_map, floor_index)?;
    let edited_floor = map.active_floor;

    let edit_summary: Value = match req {
        MapFeatureEditRequest::ToggleDoor { x, y, side } => {
            let side = parse_coded_enum::<EdgeSide>(&side, "edge side")?;
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
            json!({ "x": x, "y": y, "side": side.as_str(), "kind": next_kind.as_str() })
        }
        MapFeatureEditRequest::SetEdge {
            x,
            y,
            side,
            kind,
            material,
        } => {
            let side = parse_coded_enum::<EdgeSide>(&side, "edge side")?;
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
            let kind = parse_coded_enum::<WallEdgeKind>(&kind, "wall edge kind")?;
            let mat = parse_coded_enum::<MaterialClass>(
                &normalize_material_code(&material),
                "material class",
            )?;
            let new_edge = MapWallEdge {
                x,
                y,
                side,
                kind,
                material: mat,
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
                "material": mat.as_str(),
            })
        }
        MapFeatureEditRequest::RemoveEdge { x, y, side } => {
            let side = parse_coded_enum::<EdgeSide>(&side, "edge side")?;
            let idx = map
                .wall_edges
                .iter()
                .position(|e| e.x == x && e.y == y && e.side == side)
                .ok_or_else(|| {
                    to_py_err(format!("No edge found at ({x}, {y}, {})", side.as_str()))
                })?;
            let removed = map.wall_edges.remove(idx);
            json!({ "removed_kind": removed.kind.as_str(), "x": x, "y": y, "side": side.as_str() })
        }
        MapFeatureEditRequest::PlaceProp {
            prop_kind,
            x_m,
            y_m,
            rotation_deg,
            length_cm,
            variant_seed,
        } => {
            let kind = parse_coded_enum::<PropKind>(&prop_kind, "prop kind")?;
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
        MapFeatureEditRequest::RemoveProp { x_m, y_m } => {
            let removed = remove_prop_at_point(&mut map.props, x_m, y_m)
                .ok_or_else(|| to_py_err(format!("No prop found near ({x_m:.2}m, {y_m:.2}m)")))?;
            json!({
                "removed_kind": removed.kind.as_str(),
                "x_m": removed.center_m().0,
                "y_m": removed.center_m().1,
            })
        }
        MapFeatureEditRequest::SetBlackout {
            blackout_active,
            ambient_light_delta_ext,
        } => {
            map.ambient_light_delta_ext = ambient_light_delta_ext.unwrap_or(if blackout_active {
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
        MapFeatureEditRequest::BlastOrBreach {
            x_m,
            y_m,
            ordnance_code,
            demolitions_net_successes,
            apply_wall_breaches,
        } => {
            let ordnance = parse_coded_enum::<ExplosiveOrdnanceId>(&ordnance_code, "ordnance")?;
            let outcome = satya_maps::barrier_ballistics::evaluate_spatial_blast_and_breach(
                &mut map,
                (x_m, y_m),
                edited_floor,
                ordnance,
                demolitions_net_successes,
                apply_wall_breaches,
            );
            json!({ "blast_outcome": outcome })
        }
    };

    map.sync_active_floor_snapshot();
    if !map.floors.is_empty() && map.active_floor != orig_floor {
        let _ = map.switch_active_floor(orig_floor);
    }

    to_py_object(
        py,
        &json!({
            "action": action_name,
            "edited_floor_index": edited_floor,
            "edit_summary": edit_summary,
            "updated_map": map,
        }),
    )
}

/// Returns canonical `PropKind` codes that are walkable floor layers (never solid obstacles).
pub(crate) fn non_solid_prop_kind_codes() -> Vec<String> {
    PROP_CATALOG
        .iter()
        .filter(|spec| spec.floor_layer || matches!(spec.obstruction, PropObstruction::FloorLayer))
        .map(|spec| spec.kind.as_str().to_string())
        .collect()
}

/// Lists all canonical non-solid/floor-layer `PropKind` codes from `satya-maps`'s `PROP_CATALOG`.
#[pyfunction]
pub fn list_non_solid_prop_kinds() -> Vec<String> {
    non_solid_prop_kind_codes()
}

/// Registers tactical map spatial bindings on the `satya_py` Python module.
pub fn register_tactical_map_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_tactical_token_collision, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_tactical_token_destination_cm, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_tactical_los_and_cover, m)?)?;
    m.add_function(wrap_pyfunction!(inspect_tactical_map_point, m)?)?;
    m.add_function(wrap_pyfunction!(generate_procedural_sector, m)?)?;
    m.add_function(wrap_pyfunction!(continue_procedural_map, m)?)?;
    m.add_function(wrap_pyfunction!(switch_map_floor, m)?)?;
    m.add_function(wrap_pyfunction!(extract_narrative_manifest, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_passive_perceptions, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_ballistic_shot, m)?)?;
    m.add_function(wrap_pyfunction!(compute_reachable_zone, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_spatial_blast_and_breach, m)?)?;
    m.add_function(wrap_pyfunction!(edit_map_feature, m)?)?;
    m.add_function(wrap_pyfunction!(list_non_solid_prop_kinds, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::py_bridge::py_to_value;
    use pyo3::types::PyString;

    fn parse_flexible_map_json(raw: &str) -> Result<GmMapUpdateRequest, String> {
        let val: Value =
            serde_json::from_str(raw).map_err(|e| format!("Invalid tactical map JSON: {e}"))?;
        parse_flexible_map_value(val)
    }

    #[test]
    fn test_tactical_map_py_wrappers_roundtrip_and_los_inspect() {
        Python::initialize();
        Python::attach(|py| {
            let sector = generate_procedural_sector(
                py,
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
            let manifest = extract_narrative_manifest(py, sector.bind(py).clone()).unwrap();
            let manifest_val = py_to_value(manifest.bind(py)).unwrap();
            assert_eq!(manifest_val["cardinal_exits"].as_array().unwrap().len(), 4);
            assert!(!manifest_val["walkable_anchors"].as_array().unwrap().is_empty());

            let hosp = generate_procedural_sector(
                py,
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
            let hosp_val = py_to_value(hosp.bind(py)).unwrap();
            assert!(hosp_val["floors"].as_array().unwrap().len() >= 2);
            let switched = switch_map_floor(py, hosp.bind(py).clone(), 1).unwrap();
            assert_eq!(py_to_value(switched.bind(py)).unwrap()["active_floor"], 1);

            let mut parsed = parse_flexible_map_obj(sector.bind(py)).unwrap();
            assert_eq!(parsed.grid_width, 12);
            assert_eq!(parsed.world_sectors.len(), 1);
            assert!(!parsed.wall_edges.is_empty());
            assert!(!parsed.props.is_empty());

            let east =
                continue_procedural_map(py, sector.bind(py).clone(), "EAST", Some(99), None)
                    .unwrap();
            let east_map = parse_flexible_map_obj(east.bind(py)).unwrap();
            assert_eq!(east_map.world_sectors.len(), 2);
            assert!(east_map.grid_width > 12);

            let py_dict_json = json!({
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

            let py_dict_obj = PyString::new(py, &py_dict_json).into_any();
            assert!(
                check_tactical_token_collision(py_dict_obj.clone(), 0.10, 5.0, 0.30, None).unwrap()
            );
            let mut resolved = None;
            for gy in 2..9 {
                for gx in 2..10 {
                    let cx = gx * 100 + 50;
                    let cy = gy * 100 + 50;
                    if let Ok(pt) = resolve_tactical_token_destination_cm(
                        py_dict_obj.clone(),
                        "tok-vance",
                        cx,
                        cy,
                        None,
                    ) {
                        resolved = Some(pt);
                        break;
                    }
                }
                if resolved.is_some() {
                    break;
                }
            }
            assert!(resolved.is_some());

            let scene = TacticalSceneView::from_map(&parsed);
            let los = evaluate_los_continuous(&scene, (2.5, 5.5), (9.5, 5.5));
            assert!(los.distance_m > 6.0);
            let hit = inspect_map_point(&parsed, "Sector 4 Docking Bay", 2.5, 5.5).unwrap();
            assert_eq!(hit.category, MapHoverCategory::Token);
        });
    }

    #[test]
    fn test_wave1_multifloor_parse_auto_scale_and_floor_spatial_queries() {
        Python::initialize();
        Python::attach(|py| {
            let auto_map = generate_procedural_sector(
                py,
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
            let auto_val = py_to_value(auto_map.bind(py)).unwrap();
            let w = auto_val["grid_width"].as_i64().unwrap();
            let h = auto_val["grid_height"].as_i64().unwrap();
            assert!(w >= 16 && h >= 12);
            assert!(auto_val["floors"].as_array().unwrap().len() >= 2);

            let merged_json = json!({
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
                    {"entity_id": "tok-f1", "label": "Nurse Lin", "x": 3, "y": 4, "pos_x_cm": 350, "pos_y_cm": 450}
                ],
                "token_visuals": [
                    {"entity_id": "tok-f1", "floor_index": 1, "color_hex": "#34d399"}
                ]
            })
            .to_string();
            let merged = parse_flexible_map_json(&merged_json).unwrap();
            assert_eq!(merged.token_floor("tok-f1"), 1);

            let collides_f1 =
                check_tactical_token_collision(auto_map.bind(py).clone(), 0.05, 0.05, 0.30, Some(1))
                    .unwrap();
            assert!(collides_f1);

            let f1_parsed = map_for_floor(parse_flexible_map_obj(auto_map.bind(py)).unwrap(), Some(1)).unwrap();
            let mut f1_clear = None;
            for gy in 2..((f1_parsed.grid_height - 2).max(3)) {
                for gx in 2..((f1_parsed.grid_width - 2).max(3)) {
                    let cx = gx * 100 + 50;
                    let cy = gy * 100 + 50;
                    if let Ok(pt) = resolve_tactical_token_destination_cm(
                        auto_map.bind(py).clone(),
                        "tok-f1",
                        cx,
                        cy,
                        Some(1),
                    ) {
                        f1_clear = Some(pt);
                        break;
                    }
                }
                if f1_clear.is_some() {
                    break;
                }
            }
            assert!(f1_clear.is_some());

            let los_f1 = evaluate_tactical_los_and_cover(
                py,
                auto_map.bind(py).clone(),
                2.5,
                2.5,
                8.5,
                2.5,
                Some(1),
            )
            .unwrap();
            assert!(py_to_value(los_f1.bind(py)).unwrap().get("cover_grade").is_some());

            let hover_f1 = inspect_tactical_map_point(
                py,
                auto_map.bind(py).clone(),
                "Auto Hospital",
                2.5,
                2.5,
                Some(1),
            )
            .unwrap()
            .expect("hover on floor 1 returns inspection dict");
            assert!(py_to_value(hover_f1.bind(py)).unwrap().get("category").is_some());
        });
    }

    #[test]
    fn test_parse_flexible_map_json_normalizes_legacy_concrete_material() {
        let legacy_json = json!({
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
        assert_eq!(parsed.wall_edges[0].material, MaterialClass::ConcreteBlock);
        assert_eq!(
            parsed.floors[0].wall_edges[0].material,
            MaterialClass::ConcreteBlock
        );
    }

    #[test]
    fn test_wave5_py_spatial_ballistics_reach_blast_and_edits() {
        Python::initialize();
        Python::attach(|py| {
            let py_s = |s: &str| PyString::new(py, s).into_any();
            let bunker = generate_procedural_sector(
                py,
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
            let bunker_val = py_to_value(bunker.bind(py)).unwrap();
            let fz = &bunker_val["faraday_zones"][0];
            let fz_x = ((fz["x0_cm"].as_f64().unwrap() + fz["x1_cm"].as_f64().unwrap()) / 200.0) as f32;
            let fz_y = ((fz["y0_cm"].as_f64().unwrap() + fz["y1_cm"].as_f64().unwrap()) / 200.0) as f32;

            let pt = inspect_tactical_map_point(py, bunker.bind(py).clone(), "Bunker", fz_x, fz_y, Some(0))
                .unwrap()
                .expect("point inside Faraday zone inspects cleanly");
            let pt_val = py_to_value(pt.bind(py)).unwrap();
            assert_eq!(pt_val["in_faraday_zone"], true);
            assert!(pt_val.get("acoustic_surface").is_some());
            assert!(pt_val.get("traction_tier").is_some());
            assert!(pt_val.get("ambient_light_delta_ext").is_some());

            let base_map = json!({
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

            let edge_res = edit_map_feature(
                py,
                py_s(&base_map),
                py_s(r#"{"action":"SET_EDGE","x":6,"y":5,"side":"NORTH","kind":"WALL","material":"DRYWALL"}"#),
                None,
            )
            .unwrap();
            let after_wall = edge_res.bind(py).get_item("updated_map").unwrap();

            let shot = evaluate_ballistic_shot(
                py,
                after_wall.clone(),
                6.5,
                4.2,
                6.5,
                5.8,
                "hk416_carbine_556",
                2,
                false,
                None,
            )
            .unwrap();
            let shot_v = py_to_value(shot.bind(py)).unwrap();
            assert!(shot_v["weapon_name"].as_str().unwrap().contains("HK416"));
            assert!(shot_v["effective_delta_ext"].as_i64().is_some());

            let reach = compute_reachable_zone(
                py,
                after_wall.clone(),
                2.5,
                2.5,
                11,
                1.75,
                "COMBAT_GLIDE",
                3,
                None,
                None,
            )
            .unwrap();
            let reach_v = py_to_value(reach.bind(py)).unwrap();
            assert!(reach_v["reachable_cell_count"].as_u64().unwrap() > 0);
            assert!(!reach_v["strips"].as_array().unwrap().is_empty());

            let door_set = edit_map_feature(
                py,
                after_wall.clone(),
                py_s(r#"{"action":"SET_EDGE","x":4,"y":5,"side":"NORTH","kind":"DOOR_CLOSED","material":"SOLID_WOOD"}"#),
                None,
            )
            .unwrap();
            let toggled = edit_map_feature(
                py,
                door_set.bind(py).get_item("updated_map").unwrap(),
                py_s(r#"{"action":"TOGGLE_DOOR","x":4,"y":5,"side":"NORTH"}"#),
                None,
            )
            .unwrap();
            assert_eq!(
                py_to_value(toggled.bind(py)).unwrap()["edit_summary"]["kind"],
                "DOOR_OPEN"
            );

            let placed = edit_map_feature(
                py,
                after_wall.clone(),
                py_s(r#"{"action":"PLACE_PROP","prop_kind":"WOOD_CRATE","x_m":3.5,"y_m":3.5,"rotation_deg":15}"#),
                None,
            )
            .unwrap();
            let removed = edit_map_feature(
                py,
                placed.bind(py).get_item("updated_map").unwrap(),
                py_s(r#"{"action":"REMOVE_PROP","x_m":3.5,"y_m":3.5}"#),
                None,
            )
            .unwrap();
            assert_eq!(
                py_to_value(removed.bind(py)).unwrap()["edit_summary"]["removed_kind"],
                "WOOD_CRATE"
            );

            let blackout = edit_map_feature(
                py,
                after_wall.clone(),
                py_s(r#"{"action":"SET_BLACKOUT","blackout_active":true}"#),
                None,
            )
            .unwrap();
            let b_val = py_to_value(blackout.bind(py)).unwrap();
            assert!(b_val["updated_map"]["ambient_light_delta_ext"].as_i64().unwrap() >= 3);
            assert_eq!(b_val["updated_map"]["surveillance_nodes"][0]["powered"], false);

            let blast = evaluate_spatial_blast_and_breach(
                py,
                after_wall,
                6.5,
                4.9,
                "C4",
                2,
                true,
                None,
            )
            .unwrap();
            assert!(
                py_to_value(blast.bind(py)).unwrap()["outcome"]["breached_edges_count"]
                    .as_u64()
                    .unwrap()
                    >= 1
            );
        });
    }
}
