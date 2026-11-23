//! Battlemap Save/Load Vault (`SQLite` `/api/maps/*` + `~/Satya/maps/*.satyamap`),
//! offline browser `localStorage` persistence, and portable `.satyamap` file export/import.

use crate::wasm_client::dom::{
    escape_html, fetch_json, input_value, local_storage_get, local_storage_set, prompt_save_bytes,
    read_first_file_bytes, select_value, set_button_disabled, set_class, set_display, set_html,
    set_input_value, set_text, slugify_identifier,
};
use crate::wasm_client::map_2d_camera::local_user_can_author_map;
use crate::wasm_client::map_3d::{
    apply_incoming_map_update, refresh_map_dom_and_wgpu, with_map_ui, with_map_ui_mut,
};
use crate::wasm_client::state::with_state;
use satya_schema::{
    GenerateSessionMapRequest, GmMapUpdateRequest, MapSurfaceFeature, SAVED_MAP_FORMAT_VERSION,
    SaveMapRequest, SaveMapResponse, SavedMapBundle, SavedMapListResponse, SavedMapRecordResponse,
    SavedMapSummary, SurfaceShapeKind,
};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// `localStorage` key storing offline saved battlemap `.satyamap` bundles.
pub const LOCAL_SAVED_MAPS_STORAGE_KEY: &str = "satya.saved_maps.v1";

const MAX_LOCAL_SAVED_MAPS: usize = 20;
const MIN_MAP_DIM: i32 = 8;
const MAX_MAP_WIDTH: i32 = 240;
const MAX_MAP_HEIGHT: i32 = 180;

#[derive(Debug, Clone, Default)]
struct MapVaultState {
    open: bool,
    saved_maps: Vec<SavedMapSummary>,
    selected_map_id: Option<String>,
    feedback: String,
    feedback_is_error: bool,
    synced_map_key: String,
}

thread_local! {
    static MAP_VAULT_STATE: RefCell<MapVaultState> = RefCell::new(MapVaultState::default());
}

fn with_vault_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut MapVaultState) -> R,
{
    MAP_VAULT_STATE.with(|c| f(&mut c.borrow_mut()))
}

fn now_utc_us() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() * 1000.0) as i64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

/// Formats `<option>` elements for `#map-saved-select` from [`SavedMapSummary`] records.
#[must_use]
pub fn format_saved_maps_options_html(
    maps: &[SavedMapSummary],
    selected_id: Option<&str>,
) -> String {
    if maps.is_empty() {
        return r#"<option value="">-- No saved maps yet --</option>"#.to_string();
    }
    let mut out = String::with_capacity(maps.len() * 128);
    for (idx, m) in maps.iter().enumerate() {
        let is_selected = match selected_id.filter(|v| !v.is_empty()) {
            Some(sel) => sel == m.map_id,
            None => idx == 0,
        };
        let sel_attr = if is_selected { " selected" } else { "" };
        out.push_str(&format!(
            r#"<option value="{id}"{sel_attr}>{title} ({w}m×{h}m · {floors}F · {props} props)</option>"#,
            id = escape_html(&m.map_id),
            title = escape_html(&m.title),
            w = m.grid_width,
            h = m.grid_height,
            floors = m.floor_count.max(1),
            props = m.prop_count,
        ));
    }
    out
}

/// Builds a safe `.satyamap` export filename from a battlemap title.
#[must_use]
pub fn build_saved_map_export_filename(title: &str) -> String {
    format!("{}.satyamap", slugify_identifier(title, "satya-battlemap"))
}

/// Serializes a [`GmMapUpdateRequest`] into a pretty-printed [`SavedMapBundle`] JSON payload.
pub fn serialize_saved_map_bundle_json(
    map: &GmMapUpdateRequest,
    saved_at_us: i64,
) -> Result<Vec<u8>, String> {
    let mut normalized = map.clone();
    normalized.sync_active_floor_snapshot();
    let bundle = SavedMapBundle {
        format_version: SAVED_MAP_FORMAT_VERSION,
        saved_at_us,
        map: normalized,
    };
    serde_json::to_vec_pretty(&bundle).map_err(|err| format!("Failed to serialize battlemap: {err}"))
}

fn validate_surface_pts(sf: &MapSurfaceFeature) -> Result<(), String> {
    let min_pts = match sf.shape {
        SurfaceShapeKind::Polygon | SurfaceShapeKind::OrganicClosed => 3,
        SurfaceShapeKind::RibbonSpline => 2,
    };
    if sf.points_cm.len() < min_pts {
        return Err(format!(
            "Surface feature '{}' requires at least {min_pts} control points",
            sf.kind.label()
        ));
    }
    Ok(())
}

/// Parses and validates an imported `.satyamap` bundle or bare [`GmMapUpdateRequest`] JSON bytes.
pub fn parse_imported_map_bytes(bytes: &[u8]) -> Result<GmMapUpdateRequest, String> {
    let mut map = SavedMapBundle::parse_json_bytes(bytes)?;
    if !(MIN_MAP_DIM..=MAX_MAP_WIDTH).contains(&map.grid_width)
        || !(MIN_MAP_DIM..=MAX_MAP_HEIGHT).contains(&map.grid_height)
    {
        return Err(format!(
            "Map dimensions {}x{} out of valid bounds ({MIN_MAP_DIM}..={MAX_MAP_WIDTH} x {MIN_MAP_DIM}..={MAX_MAP_HEIGHT})",
            map.grid_width, map.grid_height
        ));
    }
    map.cell_size_cm = if map.cell_size_cm <= 0 {
        100
    } else {
        map.cell_size_cm.clamp(25, 250)
    };
    let trimmed = map.title.trim();
    map.title = if trimmed.is_empty() {
        "Imported Battlemap".to_string()
    } else {
        trimmed.chars().take(120).collect()
    };
    for sf in &map.surfaces {
        validate_surface_pts(sf)?;
    }
    for fl in &map.floors {
        for sf in &fl.surfaces {
            validate_surface_pts(sf)?;
        }
    }
    map.sync_active_floor_snapshot();
    Ok(map)
}

fn upsert_local_saved_map_bundle(
    bundles: &mut Vec<SavedMapBundle>,
    mut map: GmMapUpdateRequest,
    save_as_new: bool,
    now_us: i64,
) -> SavedMapSummary {
    let trimmed = map.title.trim();
    map.title = if trimmed.is_empty() {
        "Untitled Battlemap".to_string()
    } else {
        trimmed.chars().take(120).collect()
    };
    map.sync_active_floor_snapshot();

    let incoming_id = map.map_id.trim().to_string();
    let is_ephemeral =
        incoming_id.is_empty() || incoming_id == "map-default" || incoming_id == "map-starter";

    if !save_as_new && !is_ephemeral
        && let Some(slot) = bundles.iter_mut().find(|b| b.map.map_id == incoming_id)
    {
        slot.map = map.clone();
        slot.saved_at_us = now_us;
    } else {
        let resolved_id = if save_as_new || is_ephemeral {
            let mut seq = bundles.len() + 1;
            loop {
                let cand = format!("local-map-{now_us}-{seq}");
                if !bundles.iter().any(|b| b.map.map_id == cand) {
                    break cand;
                }
                seq += 1;
            }
        } else {
            incoming_id
        };
        map.map_id = resolved_id;
        bundles.push(SavedMapBundle {
            format_version: SAVED_MAP_FORMAT_VERSION,
            saved_at_us: now_us,
            map: map.clone(),
        });
    }

    bundles.sort_by_key(|a| std::cmp::Reverse(a.saved_at_us));
    if bundles.len() > MAX_LOCAL_SAVED_MAPS {
        bundles.truncate(MAX_LOCAL_SAVED_MAPS);
    }
    SavedMapSummary::from_map(&map, now_us, now_us)
}

fn summaries_from_local_bundles(bundles: &[SavedMapBundle]) -> Vec<SavedMapSummary> {
    bundles
        .iter()
        .map(|b| SavedMapSummary::from_map(&b.map, b.saved_at_us, b.saved_at_us))
        .collect()
}

fn find_local_saved_map<'a>(
    bundles: &'a [SavedMapBundle],
    map_id: &str,
) -> Option<&'a GmMapUpdateRequest> {
    bundles
        .iter()
        .find(|b| b.map.map_id == map_id)
        .map(|b| &b.map)
}

fn remove_local_saved_map(bundles: &mut Vec<SavedMapBundle>, map_id: &str) -> bool {
    let before = bundles.len();
    bundles.retain(|b| b.map.map_id != map_id);
    bundles.len() != before
}

fn load_local_saved_map_bundles() -> Vec<SavedMapBundle> {
    local_storage_get(LOCAL_SAVED_MAPS_STORAGE_KEY)
        .and_then(|raw| serde_json::from_str::<Vec<SavedMapBundle>>(&raw).ok())
        .unwrap_or_default()
}

fn save_local_saved_map_bundles(bundles: &[SavedMapBundle]) -> Result<(), String> {
    let json = serde_json::to_string(bundles)
        .map_err(|err| format!("Failed to encode local saved maps: {err}"))?;
    local_storage_set(LOCAL_SAVED_MAPS_STORAGE_KEY, &json);
    Ok(())
}

/// Synchronizes `#btn-map-vault-toggle`, `#map-vault-bar`, `#map-save-title-input`,
/// `#map-saved-select`, and `#map-vault-feedback`.
pub fn sync_map_vault_dom() {
    let can_author = local_user_can_author_map();
    set_display(
        "btn-map-vault-toggle",
        if can_author { "inline-flex" } else { "none" },
    );
    if !can_author {
        set_display("map-vault-bar", "none");
        return;
    }

    let (map_id, map_title) = with_map_ui(|ui| (ui.map.map_id.clone(), ui.map.title.clone()));
    let next_key = format!("{map_id}|{map_title}");

    let (open, maps, selected_id, feedback, feedback_is_error, should_sync_title) =
        with_vault_mut(|v| {
            let sync_title = v.synced_map_key != next_key;
            if sync_title {
                v.synced_map_key = next_key;
            }
            (
                v.open,
                v.saved_maps.clone(),
                v.selected_map_id.clone(),
                v.feedback.clone(),
                v.feedback_is_error,
                sync_title,
            )
        });

    set_class(
        "btn-map-vault-toggle",
        if open {
            "btn btn-sm btn-accent"
        } else {
            "btn btn-sm btn-secondary"
        },
    );
    set_display("map-vault-bar", if open { "flex" } else { "none" });

    if should_sync_title {
        set_input_value("map-save-title-input", &map_title);
    }

    if open {
        let opts_html = format_saved_maps_options_html(&maps, selected_id.as_deref());
        set_html("map-saved-select", &opts_html);
        let no_maps = maps.is_empty();
        set_button_disabled("btn-map-load-saved", no_maps);
        set_button_disabled("btn-map-delete-saved", no_maps);
    }

    set_text("map-vault-feedback", &feedback);
    let fb_cls = if feedback.is_empty() {
        "map-editor-status-pill"
    } else if feedback_is_error {
        "map-editor-status-pill map-vault-feedback-err"
    } else {
        "map-editor-status-pill map-vault-feedback-ok"
    };
    set_class("map-vault-feedback", fb_cls);
}

/// Clears cached Map Vault records and feedback when logging out or leaving a session.
pub fn reset_map_vault_client_cache() {
    with_vault_mut(|v| {
        v.open = false;
        v.saved_maps.clear();
        v.selected_map_id = None;
        v.feedback.clear();
        v.feedback_is_error = false;
        v.synced_map_key.clear();
    });
    sync_map_vault_dom();
}

fn push_loaded_map_to_active_session_if_connected(map: GmMapUpdateRequest) {
    let (auth_token, session_id) = with_state(|st| (st.auth_token.clone(), st.session_id.clone()));
    let (Some(token), Some(sid)) = (auth_token, session_id) else {
        return;
    };
    if !local_user_can_author_map() {
        return;
    }
    let req = GenerateSessionMapRequest {
        load_map: Some(Box::new(map)),
        populate_roster_tokens: false,
        ..Default::default()
    };
    spawn_local(async move {
        match fetch_json::<_, GmMapUpdateRequest>(
            "POST",
            &format!("/api/sessions/{sid}/map"),
            Some(&token),
            Some(&req),
        )
        .await
        {
            Ok(updated) => apply_incoming_map_update(&updated),
            Err(err) => {
                with_vault_mut(|v| {
                    v.feedback = format!("Loaded locally, but failed to sync to session: {err}");
                    v.feedback_is_error = true;
                });
                sync_map_vault_dom();
            }
        }
    });
}

fn apply_loaded_battlemap(map: GmMapUpdateRequest, source_note: String) {
    let loaded_id = map.map_id.clone();
    let loaded_title = map.title.clone();
    apply_incoming_map_update(&map);
    with_vault_mut(|v| {
        v.selected_map_id = Some(loaded_id.clone());
        v.feedback = source_note;
        v.feedback_is_error = false;
        v.synced_map_key = format!("{loaded_id}|{loaded_title}");
    });
    set_input_value("map-save-title-input", &loaded_title);
    sync_map_vault_dom();
    push_loaded_map_to_active_session_if_connected(map);
}

/// Toggles visibility of the Save / Load Battlemap bar (`#map-vault-bar`).
#[wasm_bindgen]
pub fn satya_toggle_map_vault() {
    if !local_user_can_author_map() {
        return;
    }
    let opened = with_vault_mut(|v| {
        v.open = !v.open;
        v.open
    });
    if opened {
        satya_refresh_saved_maps();
    }
    sync_map_vault_dom();
}

/// Fetches saved battlemaps from `GET /api/maps` (when signed in) or `localStorage` (offline).
#[wasm_bindgen]
pub fn satya_refresh_saved_maps() {
    let auth_token = with_state(|st| st.auth_token.clone());
    if let Some(token) = auth_token {
        spawn_local(async move {
            match fetch_json::<(), SavedMapListResponse>("GET", "/api/maps", Some(&token), None)
                .await
            {
                Ok(resp) => {
                    with_vault_mut(|v| {
                        v.saved_maps = resp.maps;
                        if let Some(sel) = v.selected_map_id.as_deref()
                            && !v.saved_maps.iter().any(|m| m.map_id == sel)
                        {
                            v.selected_map_id = v.saved_maps.first().map(|m| m.map_id.clone());
                        } else if v.selected_map_id.is_none() {
                            v.selected_map_id = v.saved_maps.first().map(|m| m.map_id.clone());
                        }
                    });
                    sync_map_vault_dom();
                }
                Err(err) => {
                    with_vault_mut(|v| {
                        v.feedback = err;
                        v.feedback_is_error = true;
                    });
                    sync_map_vault_dom();
                }
            }
        });
        return;
    }

    let bundles = load_local_saved_map_bundles();
    let summaries = summaries_from_local_bundles(&bundles);
    with_vault_mut(|v| {
        v.saved_maps = summaries;
        if let Some(sel) = v.selected_map_id.as_deref()
            && !v.saved_maps.iter().any(|m| m.map_id == sel)
        {
            v.selected_map_id = v.saved_maps.first().map(|m| m.map_id.clone());
        } else if v.selected_map_id.is_none() {
            v.selected_map_id = v.saved_maps.first().map(|m| m.map_id.clone());
        }
    });
    sync_map_vault_dom();
}

/// Saves the active battlemap (`save_as_new = false` updates existing saved ID; `true` creates a new copy).
#[wasm_bindgen]
pub fn satya_save_battlemap(save_as_new: bool) {
    if !local_user_can_author_map() {
        return;
    }
    let title_input = input_value("map-save-title-input").trim().to_string();
    let (map, pill_txt) = with_map_ui_mut(|ui| {
        if !title_input.is_empty() {
            ui.map.title = title_input.chars().take(120).collect();
        }
        ui.map.sync_active_floor_snapshot();
        let (bw_m, bh_m) = ui.map.board_size_m();
        let pill = format!("{} ({bw_m:.0}m × {bh_m:.0}m · Free Metric)", ui.map.title);
        (ui.map.clone(), pill)
    });
    set_text("canvas-map-pill", &pill_txt);

    let (auth_token, in_session) =
        with_state(|st| (st.auth_token.clone(), st.session_id.is_some()));

    if let Some(token) = auth_token {
        let req = SaveMapRequest { map, save_as_new };
        spawn_local(async move {
            match fetch_json::<_, SaveMapResponse>("POST", "/api/maps", Some(&token), Some(&req))
                .await
            {
                Ok(resp) => {
                    with_map_ui_mut(|ui| {
                        ui.map.map_id = resp.map.map_id.clone();
                        ui.map.title = resp.map.title.clone();
                    });
                    let note = match resp.local_file_path {
                        Some(p) => format!("Saved '{}' ({p})", resp.summary.title),
                        None => format!("Saved '{}' to Map Vault", resp.summary.title),
                    };
                    with_vault_mut(|v| {
                        v.selected_map_id = Some(resp.summary.map_id.clone());
                        v.feedback = note.clone();
                        v.feedback_is_error = false;
                        v.synced_map_key = format!("{}|{}", resp.map.map_id, resp.map.title);
                    });
                    satya_refresh_saved_maps();
                    refresh_map_dom_and_wgpu();
                    set_text("map-editor-status-pill", &note);
                    if in_session {
                        push_loaded_map_to_active_session_if_connected(resp.map);
                    }
                }
                Err(err) => {
                    with_vault_mut(|v| {
                        v.feedback = err.clone();
                        v.feedback_is_error = true;
                    });
                    sync_map_vault_dom();
                    set_text("map-editor-status-pill", &err);
                }
            }
        });
        return;
    }

    let mut bundles = load_local_saved_map_bundles();
    let summary = upsert_local_saved_map_bundle(&mut bundles, map, save_as_new, now_utc_us());
    match save_local_saved_map_bundles(&bundles) {
        Ok(()) => {
            with_map_ui_mut(|ui| {
                ui.map.map_id = summary.map_id.clone();
                ui.map.title = summary.title.clone();
            });
            let summaries = summaries_from_local_bundles(&bundles);
            let note = format!(
                "Saved '{}' locally in browser (log in to sync to Server Vault & ~/Satya/maps)",
                summary.title
            );
            with_vault_mut(|v| {
                v.saved_maps = summaries;
                v.selected_map_id = Some(summary.map_id.clone());
                v.feedback = note.clone();
                v.feedback_is_error = false;
                v.synced_map_key = format!("{}|{}", summary.map_id, summary.title);
            });
            refresh_map_dom_and_wgpu();
            set_text("map-editor-status-pill", &note);
        }
        Err(err) => {
            with_vault_mut(|v| {
                v.feedback = err.clone();
                v.feedback_is_error = true;
            });
            sync_map_vault_dom();
            set_text("map-editor-status-pill", &err);
        }
    }
}

/// Loads the currently selected saved battlemap from `#map-saved-select`.
#[wasm_bindgen]
pub fn satya_load_saved_map() {
    if !local_user_can_author_map() {
        return;
    }
    let map_id = select_value("map-saved-select").trim().to_string();
    if map_id.is_empty() {
        with_vault_mut(|v| {
            v.feedback = "Select a saved battlemap first.".to_string();
            v.feedback_is_error = true;
        });
        sync_map_vault_dom();
        return;
    }

    let auth_token = with_state(|st| st.auth_token.clone());
    if let Some(token) = auth_token {
        spawn_local(async move {
            match fetch_json::<(), SavedMapRecordResponse>(
                "GET",
                &format!("/api/maps/{map_id}"),
                Some(&token),
                None,
            )
            .await
            {
                Ok(rec) => {
                    let note = format!("Loaded '{}'", rec.summary.title);
                    apply_loaded_battlemap(rec.map, note);
                }
                Err(err) => {
                    with_vault_mut(|v| {
                        v.feedback = err;
                        v.feedback_is_error = true;
                    });
                    sync_map_vault_dom();
                }
            }
        });
        return;
    }

    let bundles = load_local_saved_map_bundles();
    if let Some(map) = find_local_saved_map(&bundles, &map_id).cloned() {
        let note = format!("Loaded '{}' from browser storage", map.title);
        apply_loaded_battlemap(map, note);
    } else {
        with_vault_mut(|v| {
            v.feedback = format!("Saved map '{map_id}' was not found in browser storage.");
            v.feedback_is_error = true;
        });
        sync_map_vault_dom();
    }
}

/// Deletes the currently selected saved battlemap from `#map-saved-select`.
#[wasm_bindgen]
pub fn satya_delete_saved_map() {
    if !local_user_can_author_map() {
        return;
    }
    let map_id = select_value("map-saved-select").trim().to_string();
    if map_id.is_empty() {
        with_vault_mut(|v| {
            v.feedback = "Select a saved battlemap to delete.".to_string();
            v.feedback_is_error = true;
        });
        sync_map_vault_dom();
        return;
    }

    let auth_token = with_state(|st| st.auth_token.clone());
    if let Some(token) = auth_token {
        spawn_local(async move {
            match fetch_json::<(), serde_json::Value>(
                "DELETE",
                &format!("/api/maps/{map_id}"),
                Some(&token),
                None,
            )
            .await
            {
                Ok(_) => {
                    with_vault_mut(|v| {
                        if v.selected_map_id.as_deref() == Some(map_id.as_str()) {
                            v.selected_map_id = None;
                        }
                        v.feedback = "Deleted saved battlemap.".to_string();
                        v.feedback_is_error = false;
                    });
                    satya_refresh_saved_maps();
                    sync_map_vault_dom();
                }
                Err(err) => {
                    with_vault_mut(|v| {
                        v.feedback = err;
                        v.feedback_is_error = true;
                    });
                    sync_map_vault_dom();
                }
            }
        });
        return;
    }

    let mut bundles = load_local_saved_map_bundles();
    if remove_local_saved_map(&mut bundles, &map_id) {
        let _ = save_local_saved_map_bundles(&bundles);
        let summaries = summaries_from_local_bundles(&bundles);
        with_vault_mut(|v| {
            v.saved_maps = summaries;
            v.selected_map_id = v.saved_maps.first().map(|m| m.map_id.clone());
            v.feedback = "Deleted saved battlemap from browser storage.".to_string();
            v.feedback_is_error = false;
        });
    } else {
        with_vault_mut(|v| {
            v.feedback = "Saved map not found in browser storage.".to_string();
            v.feedback_is_error = true;
        });
    }
    sync_map_vault_dom();
}

/// Exports the active battlemap as a portable `.satyamap` JSON file.
#[wasm_bindgen]
pub fn satya_export_battlemap_file() {
    let title_input = input_value("map-save-title-input").trim().to_string();
    let map = with_map_ui_mut(|ui| {
        if !title_input.is_empty() {
            ui.map.title = title_input.chars().take(120).collect();
        }
        ui.map.sync_active_floor_snapshot();
        ui.map.clone()
    });
    let filename = build_saved_map_export_filename(&map.title);
    let bytes = match serialize_saved_map_bundle_json(&map, now_utc_us()) {
        Ok(b) => b,
        Err(err) => {
            with_vault_mut(|v| {
                v.feedback = err;
                v.feedback_is_error = true;
            });
            sync_map_vault_dom();
            return;
        }
    };

    spawn_local(async move {
        let ok = prompt_save_bytes(&filename, &bytes).await;
        with_vault_mut(|v| {
            if ok {
                v.feedback = format!("Exported '{filename}'");
                v.feedback_is_error = false;
            } else {
                v.feedback = "Battlemap file export was cancelled.".to_string();
                v.feedback_is_error = true;
            }
        });
        sync_map_vault_dom();
    });
}

/// Imports a `.satyamap` or raw battlemap `.json` file selected in `#map-import-file-input`.
#[wasm_bindgen]
pub fn satya_import_battlemap_file() {
    if !local_user_can_author_map() {
        return;
    }
    spawn_local(async move {
        let file_opt = read_first_file_bytes("map-import-file-input").await;
        set_input_value("map-import-file-input", "");
        let Some((file_name, _mime, bytes)) = file_opt else {
            return;
        };
        match parse_imported_map_bytes(&bytes) {
            Ok(map) => {
                let note = format!("Imported '{}' from '{file_name}'", map.title);
                apply_loaded_battlemap(map, note);
            }
            Err(err) => {
                with_vault_mut(|v| {
                    v.feedback = format!("Failed to import '{file_name}': {err}");
                    v.feedback_is_error = true;
                });
                sync_map_vault_dom();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::{EdgeSide, MapFloorLevel, MapWallEdge, MaterialClass, WallEdgeKind};

    #[test]
    fn saved_maps_options_and_export_filename_formatting() {
        assert!(format_saved_maps_options_html(&[], None).contains("No saved maps yet"));

        let list = vec![
            SavedMapSummary {
                map_id: "m1".to_string(),
                title: "Neon Alley <North>".to_string(),
                grid_width: 24,
                grid_height: 18,
                cell_size_cm: 100,
                theme_preset: None,
                floor_count: 2,
                prop_count: 7,
                wall_count: 14,
                surface_count: 3,
                created_at_us: 10,
                updated_at_us: 20,
            },
            SavedMapSummary {
                map_id: "m2".to_string(),
                title: "Dockyard".to_string(),
                grid_width: 36,
                grid_height: 26,
                cell_size_cm: 100,
                theme_preset: None,
                floor_count: 1,
                prop_count: 12,
                wall_count: 20,
                surface_count: 5,
                created_at_us: 5,
                updated_at_us: 15,
            },
        ];
        let html = format_saved_maps_options_html(&list, Some("m2"));
        assert!(html.contains("Neon Alley &lt;North&gt; (24m×18m · 2F · 7 props)"));
        assert!(html.contains(r#"value="m2" selected>Dockyard (36m×26m · 1F · 12 props)"#));
        assert_eq!(
            build_saved_map_export_filename("Sector 4: Orbital Airlock!"),
            "sector-4-orbital-airlock.satyamap"
        );
    }

    #[test]
    fn serialize_and_parse_imported_map_bytes_handles_bundle_raw_and_invalid() {
        let mut map = GmMapUpdateRequest {
            map_id: "map_round".to_string(),
            title: "Sub-Level Vault".to_string(),
            grid_width: 20,
            grid_height: 14,
            wall_edges: vec![MapWallEdge {
                x: 1,
                y: 2,
                side: EdgeSide::North,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::ConcreteBlock,
            }],
            floors: vec![MapFloorLevel {
                floor_index: 0,
                label: "1F".to_string(),
                elevation_cm: 0,
                wall_edges: Vec::new(),
                props: Vec::new(),
                surfaces: Vec::new(),
                ground_floor_override: None,
                ground_tint_rgb: None,
            }],
            ..Default::default()
        };

        let bytes = serialize_saved_map_bundle_json(&map, 999).unwrap();
        let parsed = parse_imported_map_bytes(&bytes).unwrap();
        assert_eq!(parsed.title, "Sub-Level Vault");
        assert_eq!(parsed.floors[0].wall_edges.len(), 1);

        // Bare GmMapUpdateRequest JSON with blank title normalizes to "Imported Battlemap"
        map.title = "   ".to_string();
        let raw = serde_json::to_vec(&map).unwrap();
        let parsed_raw = parse_imported_map_bytes(&raw).unwrap();
        assert_eq!(parsed_raw.title, "Imported Battlemap");

        // Out-of-bounds dimensions fail cleanly
        let mut bad = map.clone();
        bad.grid_width = 4;
        let bad_bytes = serde_json::to_vec(&bad).unwrap();
        assert!(parse_imported_map_bytes(&bad_bytes).is_err());
    }

    #[test]
    fn local_store_upsert_save_as_new_delete_and_cap() {
        let mut store = Vec::new();
        let starter = GmMapUpdateRequest {
            map_id: "map-starter".to_string(),
            title: "First Draft".to_string(),
            ..Default::default()
        };
        let s1 = upsert_local_saved_map_bundle(&mut store, starter, false, 100);
        assert!(s1.map_id.starts_with("local-map-"));
        assert_eq!(store.len(), 1);

        // Saving again with same resolved ID updates in place
        let mut edit = find_local_saved_map(&store, &s1.map_id).unwrap().clone();
        edit.title = "First Draft v2".to_string();
        let s1_upd = upsert_local_saved_map_bundle(&mut store, edit.clone(), false, 200);
        assert_eq!(s1_upd.map_id, s1.map_id);
        assert_eq!(store.len(), 1);
        assert_eq!(store[0].map.title, "First Draft v2");

        // save_as_new mints a separate entry
        let s2 = upsert_local_saved_map_bundle(&mut store, edit, true, 300);
        assert_ne!(s2.map_id, s1.map_id);
        assert_eq!(store.len(), 2);
        assert_eq!(summaries_from_local_bundles(&store)[0].map_id, s2.map_id);

        assert!(remove_local_saved_map(&mut store, &s1.map_id));
        assert_eq!(store.len(), 1);
        assert!(!remove_local_saved_map(&mut store, "nonexistent"));

        // Cap at MAX_LOCAL_SAVED_MAPS
        for idx in 0..25 {
            let m = GmMapUpdateRequest {
                map_id: "map-starter".to_string(),
                title: format!("Map {idx}"),
                ..Default::default()
            };
            upsert_local_saved_map_bundle(&mut store, m, false, 1000 + idx);
        }
        assert_eq!(store.len(), MAX_LOCAL_SAVED_MAPS);
    }
}
