//! Docked Object & Ground Hover Inspector (`[Alt / I]` `#map-hover-tooltip` overlay pane).
//!
//! Translates continuous pointer coordinates `(x_m, y_m)` on either the 2D SVG battlemap or the
//! 3D WebGPU diorama into deterministic [`MapHoverInspection`] metadata (tokens, walls/doors/windows,
//! multi-part vehicles & trees, furniture, floor surfaces, and base ground), displaying a
//! fixed-position docked pop-up pane inside `#tabletop-map-viewport` when Inspect mode is active
//! (`Alt`-hold, `I` hotkey, or `#btn-map-inspect-toggle`) so map layout never jumps or reflows.

use crate::wasm_client::dom::{escape_html, get_element, set_class, set_display, set_html, set_text};
use crate::wasm_client::dom_tooltip::is_keyboard_typing_target;
use crate::wasm_client::map_2d_art::battlemap_theme_for_preset;
use crate::wasm_client::map_2d_art_overlay::sync_battlemap_svg_host;
use crate::wasm_client::map_3d::{
    TacticalMapUiState, format_map_tactical_hud_html, mark_map_wgpu_dirty,
    refresh_map_wgpu_if_active, with_map_ui, with_map_ui_mut,
};
use satya_engine::tactical_map_geom::{MapHoverCategory, MapHoverInspection, inspect_map_point};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Maps `theme_preset` / battlemap art theme to a player-friendly base ground surface description.
#[must_use]
pub fn ground_surface_label_for_preset(preset: Option<&str>) -> &'static str {
    let theme = battlemap_theme_for_preset(preset);
    match theme.id {
        "modern_interior" => "Oak Hardwood Floor",
        "office_interior" => "Polished Concrete Deck",
        "clinic" => "Antiseptic Vinyl Lab Tile",
        "urban_street" => "Asphalt Roadway & Curb",
        "industrial" | "derelict_industrial" => "Stained Industrial Concrete",
        "bar_interior" => "Weathered Timber Plank Floor",
        "park_grounds" => "Meadow Grass Turf",
        _ => "Brushed Deck Floor",
    }
}

/// Evaluates [`inspect_map_point`] for cursor `(x_m, y_m)` against `ui.map`.
#[must_use]
pub fn compute_hover_inspection(
    ui: &TacticalMapUiState,
    x_m: f32,
    y_m: f32,
) -> Option<MapHoverInspection> {
    let ground_label = ground_surface_label_for_preset(ui.map.theme_preset.as_deref());
    inspect_map_point(&ui.map, ground_label, x_m, y_m)
}

const fn category_badge(cat: MapHoverCategory) -> (&'static str, &'static str) {
    match cat {
        MapHoverCategory::Token => ("TOKEN", "map-hover-badge-token"),
        MapHoverCategory::EdgeBarrier => ("WALL / DOOR", "map-hover-badge-edge"),
        MapHoverCategory::SolidProp => ("OBJECT", "map-hover-badge-prop"),
        MapHoverCategory::SurfaceProp => ("SURFACE", "map-hover-badge-surface"),
        MapHoverCategory::Ground => ("GROUND", "map-hover-badge-ground"),
    }
}

/// Formats the detailed object/ground/ballistic card shown in the docked `#map-hover-tooltip` pane.
#[must_use]
pub fn format_map_hover_tooltip_html(ins: &MapHoverInspection) -> String {
    let (badge, cls) = category_badge(ins.category);
    let rot_row = ins
        .rotation_deg
        .map(|d| format!(r#"<div><span class="muted">Facing:</span> {d}°</div>"#))
        .unwrap_or_default();
    let elev_row = if ins.elevation_cm != 0 {
        format!(
            r#"<div><span class="muted">Deck Elevation:</span> +{}cm</div>"#,
            ins.elevation_cm
        )
    } else {
        String::new()
    };
    format!(
        r#"<div class="map-hover-tip-head"><span class="map-hover-badge {cls}">{badge}</span> <strong>{}</strong> <span class="mono muted">({:.2}m, {:.2}m)</span></div>
<div class="map-hover-tip-sub">{}</div>
<div class="map-hover-tip-grid">
  <div><span class="muted">Material:</span> {}</div>
  <div><span class="muted">Height:</span> {}cm</div>
  <div><span class="muted">Cover / LoS:</span> {}</div>
  {rot_row}
  {elev_row}
</div>"#,
        escape_html(&ins.title),
        ins.world_x_m,
        ins.world_y_m,
        escape_html(&ins.subtitle),
        escape_html(&ins.material_label),
        ins.height_cm,
        escape_html(&ins.cover_label),
    )
}

/// Formats the complete docked top-right Inspector pane HTML (`#map-hover-tooltip`) from `ui`,
/// including idle sector guidance when the cursor is not over the map and active weapon ballistics.
#[must_use]
pub fn format_map_inspect_popup_html(ui: &TacticalMapUiState) -> String {
    let foot = if ui.inspect_mode_pinned {
        "Inspect Pinned [I] · press I or click Inspect to close"
    } else {
        "Holding [Alt] · release Alt to hide"
    };
    let Some(ins) = ui.hover_inspection.as_ref() else {
        let ground = escape_html(ground_surface_label_for_preset(
            ui.map.theme_preset.as_deref(),
        ));
        return format!(
            r#"<div class="map-hover-tip-head"><span class="map-hover-badge map-hover-badge-ground">INSPECT</span> <strong>{ground}</strong></div>
<div class="map-hover-tip-sub">Move pointer over any token, wall, door, prop, or floor spot to inspect material, cover, and ballistics.</div>
<div class="map-hover-tip-foot muted">{foot}</div>"#
        );
    };
    let base = format_map_hover_tooltip_html(ins);
    let bal_row = ui
        .last_ballistic
        .as_ref()
        .map(|b| {
            format!(
                r#"<div class="map-hover-tip-ballistic"><span class="muted">Active Shot vs. Ray:</span> {}</div>"#,
                escape_html(&b.summary_label)
            )
        })
        .unwrap_or_default();
    format!(r#"{base}{bal_row}<div class="map-hover-tip-foot muted">{foot}</div>"#)
}

/// Refreshes the docked Inspector pop-up pane and coordinate readout pill when visible.
pub fn refresh_inspect_popup_if_visible() {
    sync_hover_tooltip_dom();
}

fn sync_hover_tooltip_dom() {
    let (pinned, alt_held, coords_txt, popup_html) = with_map_ui(|ui| {
        let coords = ui
            .hovered_point_m
            .map(|(x, y)| format!("{x:05.2}m, {y:05.2}m"))
            .unwrap_or_else(|| "Hover: --".to_string());
        let visible = ui.inspect_mode_pinned || ui.alt_inspect_held;
        let html = if visible {
            format_map_inspect_popup_html(ui)
        } else {
            String::new()
        };
        (ui.inspect_mode_pinned, ui.alt_inspect_held, coords, html)
    });
    set_text("map-hover-coords-pill", &coords_txt);
    set_text(
        "btn-map-inspect-toggle",
        if pinned { "Inspect: ON [I]" } else { "Inspect [Alt/I]" },
    );
    set_class(
        "btn-map-inspect-toggle",
        if pinned || alt_held {
            "btn btn-sm btn-accent"
        } else {
            "btn btn-sm btn-secondary"
        },
    );

    if pinned || alt_held {
        set_html("map-hover-tooltip", &popup_html);
        set_display("map-hover-tooltip", "block");
    } else {
        set_display("map-hover-tooltip", "none");
    }
}

/// Toggles persistent docked Inspect pop-up mode (`#btn-map-inspect-toggle` / `I` key).
#[wasm_bindgen]
pub fn satya_toggle_map_inspect_mode() {
    with_map_ui_mut(|ui| {
        ui.inspect_mode_pinned = !ui.inspect_mode_pinned;
    });
    sync_hover_tooltip_dom();
}

/// Updates continuous hover inspection at `(x_m, y_m)`.
#[wasm_bindgen]
pub fn satya_hover_map_point(x_m: f32, y_m: f32, _client_x: f32, _client_y: f32) {
    let (redraw_overlay, redraw_wgpu) = with_map_ui_mut(|ui| {
        let (bw, bh) = ui.map.board_size_m();
        if !x_m.is_finite() || !y_m.is_finite() || x_m < 0.0 || y_m < 0.0 || x_m > bw || y_m > bh {
            let had = ui.hovered_point_m.is_some();
            ui.hovered_point_m = None;
            ui.hovered_cell = None;
            ui.hover_inspection = None;
            if had {
                ui.recompute_overlays();
            }
            return (had, false);
        }
        let cell_m = ui.map.cell_size_m().max(0.25);
        let cell = (
            (x_m / cell_m).floor().clamp(0.0, (ui.map.grid_width - 1) as f32) as i32,
            (y_m / cell_m).floor().clamp(0.0, (ui.map.grid_height - 1) as f32) as i32,
        );
        let prev_ref = ui
            .hover_inspection
            .as_ref()
            .map(|i| i.target_ref.clone());
        let prev_cell = ui.hovered_cell;
        let next_ins = compute_hover_inspection(ui, x_m, y_m);
        let next_ref = next_ins.as_ref().map(|i| i.target_ref.clone());

        ui.hovered_point_m = Some((x_m, y_m));
        ui.hovered_cell = Some(cell);
        ui.hover_inspection = next_ins;
        let significant = prev_ref != next_ref
            || prev_cell != ui.hovered_cell
            || ui.interaction_mode == "measure_los"
            || ui.interaction_mode == "edit_terrain";
        if significant {
            ui.recompute_overlays();
        }
        (significant || !ui.view_battlemap, significant && !ui.view_battlemap)
    });

    sync_hover_tooltip_dom();
    if redraw_overlay {
        set_html("tabletop-map-los-hud", &with_map_ui(format_map_tactical_hud_html));
        sync_battlemap_svg_host();
    }
    if redraw_wgpu {
        mark_map_wgpu_dirty();
        refresh_map_wgpu_if_active();
    }
}

/// Clears hover reticle & hides the floating inspector tooltip when the pointer exits the map.
#[wasm_bindgen]
pub fn satya_leave_map_hover() {
    let changed = with_map_ui_mut(|ui| {
        let had = ui.hovered_point_m.is_some() || ui.hover_inspection.is_some();
        ui.hovered_point_m = None;
        ui.hovered_cell = None;
        ui.hover_inspection = None;
        if had {
            ui.recompute_overlays();
        }
        had
    });
    sync_hover_tooltip_dom();
    if changed {
        set_html("tabletop-map-los-hud", &with_map_ui(format_map_tactical_hud_html));
        sync_battlemap_svg_host();
        mark_map_wgpu_dirty();
        refresh_map_wgpu_if_active();
    }
}

/// Legacy cell-level hover entrypoint retained for existing callers/tests.
#[wasm_bindgen]
pub fn satya_hover_map_cell(gx: i32, gy: i32) {
    if gx < 0 || gy < 0 {
        satya_leave_map_hover();
        return;
    }
    let (xm, ym) = with_map_ui(|ui| {
        let cell_m = ui.map.cell_size_m().max(0.25);
        ((gx as f32 + 0.5) * cell_m, (gy as f32 + 0.5) * cell_m)
    });
    satya_hover_map_point(xm, ym, 0.0, 0.0);
}

fn is_maps_pane_active() -> bool {
    get_element("notebook-pane-maps")
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        .is_some_and(|h| h.style().get_property_value("display").unwrap_or_default() != "none")
}

/// Handles `Alt` hold, `I` inspect toggle, and `R` brush rotation shortcuts while the Map pane is visible.
pub fn handle_map_global_keydown(ev: &web_sys::KeyboardEvent) {
    if is_keyboard_typing_target(ev) {
        return;
    }
    if ev.key() == "Alt" {
        if is_maps_pane_active() {
            with_map_ui_mut(|ui| ui.alt_inspect_held = true);
            sync_hover_tooltip_dom();
        }
        return;
    }
    if !is_maps_pane_active() || ev.ctrl_key() || ev.meta_key() {
        return;
    }
    match ev.key().as_str() {
        "i" | "I" => {
            satya_toggle_map_inspect_mode();
        }
        "r" | "R" => {
            let delta = if ev.shift_key() { -15 } else { 15 };
            let (is_edit, rot) = with_map_ui_mut(|ui| {
                if ui.interaction_mode != "edit_terrain" {
                    return (false, ui.edit_prop_rotation_deg);
                }
                ui.edit_prop_rotation_deg = (ui.edit_prop_rotation_deg + delta).rem_euclid(360);
                (true, ui.edit_prop_rotation_deg)
            });
            if is_edit {
                crate::wasm_client::dom::set_input_value("map-prop-rot-deg", &rot.to_string());
                set_text("btn-map-prop-rotate", &format!("Rot {rot}°"));
                sync_battlemap_svg_host();
            }
        }
        _ => {}
    }
}

/// Releases `Alt`-hold floating tooltip preview cleanly.
pub fn handle_map_global_keyup(ev: &web_sys::KeyboardEvent) {
    if ev.key() == "Alt" {
        let was_held = with_map_ui_mut(|ui| {
            let prev = ui.alt_inspect_held;
            ui.alt_inspect_held = false;
            prev
        });
        if was_held {
            sync_hover_tooltip_dom();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_engine::map_dressing::make_prop_at_cm;
    use satya_schema::{EdgeSide, MapWallEdge, MaterialClass, PropKind, TokenPlacement, WallEdgeKind};

    #[test]
    fn hover_inspector_identifies_tokens_walls_car_parts_trees_surfaces_and_ground() {
        let mut ui = TacticalMapUiState::default();
        ui.map.theme_preset = Some("urban_crossroads".to_string());
        ui.map.wall_edges = vec![MapWallEdge {
            x: 4,
            y: 4,
            side: EdgeSide::North,
            kind: WallEdgeKind::Wall,
            material: MaterialClass::Brick,
        }];
        ui.map.props = vec![
            make_prop_at_cm(PropKind::Car, 750, 550, 33, None, 1),
            make_prop_at_cm(PropKind::LargeTree, 1250, 550, 0, None, 2),
            make_prop_at_cm(PropKind::StonePath, 350, 950, 0, None, 3),
        ];
        ui.map.tokens = vec![TokenPlacement {
            entity_id: "pc-vance".to_string(),
            label: "Capt. Vance".to_string(),
            x: 2,
            y: 5,
            visibility: satya_schema::MessageVisibility::Public,
        }];
        ui.map.set_token_pos_cm("pc-vance", 240, 540);

        let tok = compute_hover_inspection(&ui, 2.40, 5.40).expect("token hover");
        assert_eq!(tok.category, MapHoverCategory::Token);
        assert!(tok.title.contains("Capt. Vance"));

        let edge = compute_hover_inspection(&ui, 4.50, 4.02).expect("wall edge hover");
        assert_eq!(edge.category, MapHoverCategory::EdgeBarrier);
        assert_eq!(edge.title, "Brick masonry Wall");

        let eng_rad = 33.0_f32.to_radians();
        let cab_pt = compute_hover_inspection(&ui, 7.50, 5.50).expect("car cabin hover");
        let eng_pt = compute_hover_inspection(
            &ui,
            7.50 - 2.00 * eng_rad.sin(),
            5.50 + 2.00 * eng_rad.cos(),
        )
        .expect("car engine bay hover");
        assert!(eng_pt.title.contains("Engine Block"));
        assert!(eng_pt.material_label.contains("Armor") || eng_pt.subtitle.contains("Engine"));
        assert!(cab_pt.material_label.contains("Vehicle") || cab_pt.cover_label.contains("Cover"));

        let trunk = compute_hover_inspection(&ui, 12.50, 5.50).expect("tree trunk hover");
        assert_eq!(trunk.category, MapHoverCategory::SolidProp);
        let canopy = compute_hover_inspection(&ui, 13.20, 5.50).expect("tree canopy hover");
        assert_eq!(canopy.category, MapHoverCategory::SolidProp);
        assert!(canopy.title.contains("Canopy"));

        let stone = compute_hover_inspection(&ui, 3.50, 9.50).expect("stone path hover");
        assert_eq!(stone.category, MapHoverCategory::SurfaceProp);

        let ground = compute_hover_inspection(&ui, 1.50, 1.50).expect("ground hover");
        assert_eq!(ground.category, MapHoverCategory::Ground);
        assert!(ground.title.contains("Asphalt Roadway"));

        let idle_popup = format_map_inspect_popup_html(&ui);
        assert!(idle_popup.contains("INSPECT") && idle_popup.contains("Asphalt Roadway"));

        ui.hover_inspection = Some(eng_pt.clone());
        let popup = format_map_inspect_popup_html(&ui);
        let tip = format_map_hover_tooltip_html(&eng_pt);
        assert!(popup.contains("OBJECT") && popup.contains("map-hover-badge"));
        assert!(tip.contains("Armor") || tip.contains("Car"));
    }
}
