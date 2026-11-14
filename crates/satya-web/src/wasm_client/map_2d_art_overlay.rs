//! # Dynamic Tactical Overlay & DOM Host Sync for the 2D Battlemap (`map_2d_art_overlay`)
//!
//! Companion to [`crate::wasm_client::map_2d_art`]. Formats `#battlemap-overlay` (optional `1.0m`/`5.0m`
//! visual reference grid, continuous gait reach envelope, hover inspection highlight, LoS ray, and tokens),
//! synchronizes `#tabletop-map-svg-fallback` using `data-art-key` cache invalidation, and exposes view-mode
//! and grid-overlay toggles.

use crate::wasm_client::dom::{
    escape_html, get_element, local_storage_get, local_storage_set, set_button_disabled,
    set_display, set_select_value, set_text,
};
use crate::wasm_client::map_2d_art::{
    BATTLEMAP_CELL_PX, battlemap_art_cache_key, battlemap_theme_for_preset,
    format_battlemap_art_layer,
};
use crate::wasm_client::map_2d_art_textures::load_persisted_texture_mode_pref;
use crate::wasm_client::map_2d_tokens::{
    load_persisted_ink_style_pref, push_token_portrait_disc_svg,
};
use crate::wasm_client::map_3d::{
    TacticalMapUiState, mark_map_wgpu_dirty, refresh_map_wgpu_if_active, resolve_token_visual,
    with_map_ui, with_map_ui_mut,
};
use satya_engine::map_dressing::make_prop_at_cm;
use satya_engine::tactical_map_geom::MapHoverCategory;
use satya_schema::{CoverGrade, PropKind};
use std::cell::Cell;
use std::fmt::Write as _;
use wasm_bindgen::prelude::*;

const VIEW_PREF_KEY: &str = "satya.map.view_mode";

pub(crate) fn token_initials(label: &str) -> String {
    let mut s: String = label
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .take(2)
        .filter_map(|p| p.chars().next())
        .flat_map(char::to_uppercase)
        .collect();
    if s.is_empty() {
        s.push('?');
    }
    s
}

/// Dynamic tactical overlay (inner markup of `#battlemap-overlay`): grid, reach strips, hover, LoS, tokens.
#[must_use]
pub fn format_battlemap_overlay_inner(ui: &TacticalMapUiState) -> String {
    let t = battlemap_theme_for_preset(ui.map.theme_preset.as_deref());
    let (w, h) = (ui.map.grid_width.max(1), ui.map.grid_height.max(1));
    let c = BATTLEMAP_CELL_PX;
    let cf = c as f32;
    let (vw, vh) = (w * c, h * c);
    let mut out = String::with_capacity(16 * 1024);
    let _ = write!(out, r##"<rect width="{vw}" height="{vh}" fill="#000" fill-opacity="0"/>"##);

    if ui.show_grid_overlay {
        let ink = ui.ink_style.grid_ink_override().unwrap_or(t.grid_ink);
        let _ = write!(
            out,
            r##"<defs><pattern id="bmo-grid" width="{c}" height="{c}" patternUnits="userSpaceOnUse"><path d="M{c} 0H0V{c}" fill="none" stroke="{ink}" stroke-opacity=".24" stroke-width="1.1"/></pattern><pattern id="bmo-grid5" width="{c5}" height="{c5}" patternUnits="userSpaceOnUse"><path d="M{c5} 0H0V{c5}" fill="none" stroke="{ink}" stroke-opacity=".42" stroke-width="1.8"/></pattern></defs><rect width="{vw}" height="{vh}" fill="url(#bmo-grid)" pointer-events="none"/><rect width="{vw}" height="{vh}" fill="url(#bmo-grid5)" pointer-events="none"/><rect width="{vw}" height="{vh}" fill="none" stroke="{ink}" stroke-opacity=".42" stroke-width="2" pointer-events="none"/>"##,
            c5 = c * 5
        );
    }

    // Continuous 0.25m gait reach strips
    if !ui.reachable_strips.is_empty() {
        out.push_str(
            r##"<g pointer-events="none" fill="#10b981" fill-opacity=".17" stroke="#047857" stroke-opacity=".28" stroke-width="1">"##,
        );
        for s in &ui.reachable_strips {
            let x = s.x_min_m * cf;
            let y = (s.y_m - s.step_m * 0.5) * cf;
            let w_px = (s.x_max_m - s.x_min_m).max(0.05) * cf;
            let h_px = s.step_m * cf;
            let _ = write!(
                out,
                r#"<rect x="{x:.1}" y="{y:.1}" width="{w_px:.1}" height="{h_px:.1}" rx="3"/>"#
            );
        }
        out.push_str("</g>");
    }

    if let Some((hx_m, hy_m)) = ui.hovered_point_m {
        if ui.interaction_mode == "edit_terrain" && ui.edit_prop_brush == "__erase__" {
            let _ = write!(
                out,
                r##"<circle cx="{:.1}" cy="{:.1}" r="24" fill="#f43f5e" fill-opacity=".2" stroke="#e11d48" stroke-width="3" stroke-dasharray="7 4" pointer-events="none"/>"##,
                hx_m * cf,
                hy_m * cf,
            );
        } else if ui.interaction_mode == "edit_terrain"
            && let Some(kind) = PropKind::from_code_str(&ui.edit_prop_brush)
        {
            let (px_m, py_m) = if ui.snap_to_half_meter {
                ((hx_m * 2.0).round() * 0.5, (hy_m * 2.0).round() * 0.5)
            } else {
                (hx_m, hy_m)
            };
            let p = make_prop_at_cm(
                kind,
                (px_m * 100.0).round() as i32,
                (py_m * 100.0).round() as i32,
                ui.edit_prop_rotation_deg,
                None,
                0,
            );
            let (pcx, pcy) = p.center_m();
            let (psw, psh) = p.local_span_m();
            let (bw, bh) = (psw * cf, psh * cf);
            let _ = write!(
                out,
                r##"<g transform="translate({:.1} {:.1}) rotate({})" pointer-events="none"><rect x="{:.1}" y="{:.1}" width="{bw:.1}" height="{bh:.1}" rx="5" fill="#0ea5e9" fill-opacity=".18" stroke="#0284c7" stroke-width="2.8" stroke-dasharray="7 4"/></g>"##,
                pcx * cf,
                pcy * cf,
                p.rotation_deg,
                -bw * 0.5,
                -bh * 0.5,
            );
        } else {
            if let Some(ins) = ui.hover_inspection.as_ref()
                && ins.category == MapHoverCategory::SolidProp
                && let Some(idx_str) = ins.target_ref.strip_prefix("prop:")
                && let Ok(idx) = idx_str.parse::<usize>()
                && let Some(p) = ui.map.props.get(idx)
            {
                let (pcx, pcy) = p.center_m();
                let (psw, psh) = p.local_span_m();
                let (bw, bh) = (psw * cf, psh * cf);
                let _ = write!(
                    out,
                    r##"<g transform="translate({:.1} {:.1}) rotate({})" pointer-events="none"><rect x="{:.1}" y="{:.1}" width="{bw:.1}" height="{bh:.1}" rx="6" fill="#0ea5e9" fill-opacity=".10" stroke="#0284c7" stroke-width="2.4"/></g>"##,
                    pcx * cf,
                    pcy * cf,
                    p.rotation_deg,
                    -bw * 0.5,
                    -bh * 0.5,
                );
            }
            let _ = write!(
                out,
                r##"<circle cx="{:.1}" cy="{:.1}" r="14" fill="#0ea5e9" fill-opacity=".12" stroke="#0284c7" stroke-width="2.2" pointer-events="none"/>"##,
                hx_m * cf,
                hy_m * cf,
            );
        }
    }

    if let Some(los) = ui.last_los.as_ref() {
        let (x1, y1) = (los.attacker_m.0 * cf, los.attacker_m.1 * cf);
        let (x2, y2) = (los.target_m.0 * cf, los.target_m.1 * cf);
        let blind_pen = ui
            .last_ballistic
            .as_ref()
            .is_some_and(|b| b.is_blind_fire && b.can_reach_target);
        let stroke = match los.cover_grade {
            CoverGrade::Clear => "#0891b2",
            CoverGrade::Concealment => "#16a34a",
            CoverGrade::LowCover => "#d97706",
            CoverGrade::HighCover => "#ea580c",
            CoverGrade::Blocked if blind_pen => "#9333ea",
            CoverGrade::Blocked => "#e11d48",
        };
        let ray_tag = ui
            .last_ballistic
            .as_ref()
            .filter(|b| b.effective_delta_ext != los.recommended_delta_ext)
            .map(|b| {
                format!(
                    "{} (+{} Δ_ext)",
                    b.verdict.label(),
                    b.effective_delta_ext
                )
            })
            .unwrap_or_else(|| los.cover_grade.label().to_string());
        let (mx, my) = ((x1 + x2) * 0.5, (y1 + y2) * 0.5);
        let _ = write!(
            out,
            r##"<g pointer-events="none"><line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="#fff" stroke-opacity=".7" stroke-width="8" stroke-linecap="round"/><line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{stroke}" stroke-width="4" stroke-dasharray="11 7" stroke-linecap="round"/><circle cx="{x2:.1}" cy="{y2:.1}" r="13" fill="none" stroke="{stroke}" stroke-width="3"/><path d="M{x2:.1} {:.1}v10M{x2:.1} {:.1}v10M{:.1} {y2:.1}h10M{:.1} {y2:.1}h10" stroke="{stroke}" stroke-width="2.4"/>"##,
            y2 - 20.0,
            y2 + 10.0,
            x2 - 20.0,
            x2 + 10.0,
        );
        if los.distance_m >= 2.5 {
            let (dx, dy) = (x2 - x1, y2 - y1);
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let (nx, ny) = (-dy / len * 22.0, dx / len * 22.0);
            let (nx, ny) = if ny > 0.0 { (-nx, -ny) } else { (nx, ny) };
            let _ = write!(
                out,
                r##"<text x="{:.0}" y="{:.0}" text-anchor="middle" font-family="ui-sans-serif,system-ui,sans-serif" font-weight="700" font-size="17" fill="{stroke}" stroke="#fff" stroke-width="4.5" paint-order="stroke">{:.1}m · {}</text>"##,
                mx + nx,
                my + ny + 6.0,
                los.distance_m,
                escape_html(&ray_tag),
            );
        }
        out.push_str("</g>");
    }

    for (idx, tok) in ui.map.tokens.iter().enumerate() {
        if ui.map.token_floor(&tok.entity_id) != ui.map.active_floor {
            continue;
        }
        let (preset, col, facing) = resolve_token_visual(&ui.map, tok);
        let portrait_url = ui
            .map
            .visual_for(&tok.entity_id)
            .and_then(|v| v.portrait_url.as_deref());
        let is_sel = ui.selected_entity_id.as_deref() == Some(tok.entity_id.as_str());
        push_token_portrait_disc_svg(
            &mut out,
            idx,
            tok,
            ui.map.token_pos_m(tok),
            &preset,
            &col,
            facing,
            portrait_url,
            is_sel,
            c,
        );
    }
    out
}

/// Full layered battlemap markup for `#tabletop-map-svg-fallback` (static art and interactive overlay).
#[must_use]
pub fn format_tactical_map_svg_markup(ui: &TacticalMapUiState) -> String {
    let (bw_m, bh_m) = ui.map.board_size_m();
    let view_box = ui.camera_2d.view_box_attr((bw_m, bh_m), ui.map.cell_size_m());
    let key = battlemap_art_cache_key(ui);
    format!(
        r#"<div class="battlemap-stack"><svg id="battlemap-art" class="battlemap-art" data-art-key="{key}" xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" preserveAspectRatio="xMidYMid meet" aria-hidden="true">{art}</svg><svg id="battlemap-overlay" class="battlemap-overlay" xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" preserveAspectRatio="xMidYMid meet" role="img" aria-label="Illustrated 2D tactical battlemap ({bw_m:.0}m x {bh_m:.0}m)" onwheel="event.preventDefault();window.satyaBattlemapWheel(event.deltaY,event.clientX,event.clientY)" onmousedown="window.satyaBattlemapMouseDown(event.button,event.clientX,event.clientY)" onmousemove="window.satyaBattlemapMouseMove(event.buttons,event.clientX,event.clientY)" onmouseup="window.satyaBattlemapMouseUp(event.button,event.clientX,event.clientY)" onmouseleave="window.satyaBattlemapMouseLeave()" ondblclick="window.satyaResetMapCamera()" oncontextmenu="event.preventDefault()">{overlay}</svg></div>"#,
        art = format_battlemap_art_layer(ui),
        overlay = format_battlemap_overlay_inner(ui),
    )
}

/// Syncs `#tabletop-map-svg-fallback`, rebuilding the static art only when `data-art-key` changes.
pub fn sync_battlemap_svg_host() {
    let Some(host) = get_element("tabletop-map-svg-fallback") else {
        return;
    };
    let key = with_map_ui(battlemap_art_cache_key);
    let current_key = get_element("battlemap-art").and_then(|e| e.get_attribute("data-art-key"));
    if current_key.as_deref() == Some(key.as_str())
        && let Some(overlay) = get_element("battlemap-overlay")
    {
        overlay.set_inner_html(&with_map_ui(format_battlemap_overlay_inner));
        crate::wasm_client::map_2d_camera::sync_map_camera_and_edge_controls_dom();
        return;
    }
    host.set_inner_html(&with_map_ui(format_tactical_map_svg_markup));
    crate::wasm_client::map_2d_camera::sync_map_camera_and_edge_controls_dom();
}

fn apply_battlemap_view_chrome(battlemap_on: bool) {
    set_text(
        "btn-map-view-mode",
        if battlemap_on { "Mode: 2D Battlemap" } else { "Mode: 3D Diorama" },
    );
    for id in ["btn-map-toggle-proj", "btn-map-wall-cutaway"] {
        set_button_disabled(id, battlemap_on);
    }
    set_display("btn-map-grid-overlay", if battlemap_on { "inline-flex" } else { "none" });
    set_display("btn-map-tex-mode", if battlemap_on { "inline-flex" } else { "none" });
    set_display("map-ink-style-wrap", if battlemap_on { "inline-flex" } else { "none" });
    if battlemap_on {
        let (style, tex_mode) = with_map_ui(|ui| (ui.ink_style, ui.texture_mode));
        set_select_value("map-ink-style-select", style.as_str());
        set_text("btn-map-tex-mode", tex_mode.button_label());
        set_display("wgpu-map-canvas-wrap", "none");
        set_display("tabletop-map-svg-fallback", "block");
    } else {
        set_display("tabletop-map-svg-fallback", "none");
        set_display("wgpu-map-canvas-wrap", "block");
    }
    crate::wasm_client::map_2d_camera::sync_map_camera_and_edge_controls_dom();
}

thread_local! {
    static VIEW_PREF_LOADED: Cell<bool> = const { Cell::new(false) };
}

/// Applies the persisted `localStorage` view mode (`"battlemap"` / `"diorama"`), paper/ink style,
/// and painted/vector texture mode once per page load.
pub fn ensure_battlemap_view_pref_loaded() {
    if VIEW_PREF_LOADED.with(|c| c.replace(true)) {
        return;
    }
    let ink = load_persisted_ink_style_pref();
    let tex_mode = load_persisted_texture_mode_pref();
    with_map_ui_mut(|ui| {
        ui.ink_style = ink;
        ui.texture_mode = tex_mode;
    });
    set_select_value("map-ink-style-select", ink.as_str());
    set_text("btn-map-tex-mode", tex_mode.button_label());
    if local_storage_get(VIEW_PREF_KEY).as_deref() == Some("battlemap") {
        with_map_ui_mut(|ui| ui.view_battlemap = true);
        apply_battlemap_view_chrome(true);
    }
}

/// Toggles between the WebGPU 3D diorama and the illustrated 2D battlemap (`#btn-map-view-mode`).
#[wasm_bindgen]
pub fn satya_toggle_battlemap_view() {
    ensure_battlemap_view_pref_loaded();
    let on = with_map_ui_mut(|ui| {
        ui.view_battlemap = !ui.view_battlemap;
        ui.view_battlemap
    });
    local_storage_set(VIEW_PREF_KEY, if on { "battlemap" } else { "diorama" });
    apply_battlemap_view_chrome(on);
    if on {
        sync_battlemap_svg_host();
    } else {
        mark_map_wgpu_dirty();
        refresh_map_wgpu_if_active();
    }
}

/// Toggles the visual `1.0m` / `5.0m` reference grid overlay on the 2D battlemap (`#btn-map-grid-overlay`).
#[wasm_bindgen]
pub fn satya_toggle_battlemap_grid() {
    let on = with_map_ui_mut(|ui| {
        ui.show_grid_overlay = !ui.show_grid_overlay;
        ui.show_grid_overlay
    });
    set_text("btn-map-grid-overlay", if on { "Grid: On" } else { "Grid: Off" });
    sync_battlemap_svg_host();
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_engine::tactical_map::{SectorPreset, build_procedural_map_request};
    use satya_schema::{MessageVisibility, TokenPlacement};

    #[test]
    fn multi_floor_2d_art_cache_key_and_overlay_token_filter() {
        let mut ui = TacticalMapUiState {
            map: build_procedural_map_request(
                "mansion-2d",
                SectorPreset::LuxuryMansion,
                None,
                24,
                18,
                42,
            ),
            ..Default::default()
        };
        assert!(ui.map.has_multiple_floors(), "LuxuryMansion must have >= 2 floors");
        ui.map.tokens.push(TokenPlacement {
            entity_id: "tok-mansion-1".to_string(),
            label: "Quantum Knight".to_string(),
            x: 4,
            y: 4,
            visibility: MessageVisibility::Public,
        });
        ui.map.set_token_floor("tok-mansion-1", 0);

        let key_1f = battlemap_art_cache_key(&ui);
        let ov_1f = format_battlemap_overlay_inner(&ui);
        assert!(ov_1f.contains(">QK<"), "1F overlay should render floor-0 token");

        assert!(ui.map.switch_active_floor(1), "switch to 2F must succeed");
        let key_2f = battlemap_art_cache_key(&ui);
        assert_ne!(key_1f, key_2f, "cache key must change on floor switch");
        let ov_2f_before = format_battlemap_overlay_inner(&ui);
        assert!(
            !ov_2f_before.contains(">QK<"),
            "2F overlay must hide floor-0 token"
        );

        ui.map.set_token_floor("tok-mansion-1", 1);
        let ov_2f_after = format_battlemap_overlay_inner(&ui);
        assert!(
            ov_2f_after.contains(">QK<"),
            "2F overlay must show token after moving to floor 1"
        );
    }

    #[test]
    #[ignore]
    fn export_phase4_showcase_svgs() {
        use crate::wasm_client::map_2d_art::{BATTLEMAP_PAD_PX, BattlemapInkStyle};
        use satya_engine::map_dressing::make_prop_at_cm;
        let Ok(out_dir) = std::env::var("SATYA_SVG_PREVIEW_DIR") else {
            return;
        };
        let mut ui = TacticalMapUiState {
            map: build_procedural_map_request(
                "p4-demo",
                SectorPreset::ResidentialApartment,
                None,
                20,
                14,
                42,
            ),
            ..Default::default()
        };
        let occupied: Vec<(f32, f32)> = ui.map.tokens.iter().map(|t| ui.map.token_pos_m(t)).collect();
        let (bw_m, bh_m) = ui.map.board_size_m();
        let _ = satya_engine::tactical_map_geom::place_prop_on_map(
            &mut ui.map.props,
            &ui.map.wall_edges,
            &occupied,
            (bw_m, bh_m),
            make_prop_at_cm(PropKind::PoolTable, 1150, 550, 30, None, 7),
        );
        ui.recompute_overlays();
        let (w, h) = (ui.map.grid_width.max(1), ui.map.grid_height.max(1));
        let (tw, th) = (
            w * BATTLEMAP_CELL_PX + 2 * BATTLEMAP_PAD_PX,
            h * BATTLEMAP_CELL_PX + 2 * BATTLEMAP_PAD_PX,
        );
        let vb = format!("-{BATTLEMAP_PAD_PX} -{BATTLEMAP_PAD_PX} {tw} {th}");
        for style in BattlemapInkStyle::ALL {
            ui.ink_style = style;
            let doc = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb}">{}{}</svg>"#,
                format_battlemap_art_layer(&ui),
                format_battlemap_overlay_inner(&ui),
            );
            let path =
                std::path::Path::new(&out_dir).join(format!("phase4_style_{}.svg", style.as_str()));
            std::fs::write(path, doc).expect("write phase4 svg");
        }
    }
}
