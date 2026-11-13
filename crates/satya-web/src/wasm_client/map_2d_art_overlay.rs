//! # Dynamic Tactical Overlay & DOM Host Sync for the 2D Battlemap (`map_2d_art_overlay`)
//!
//! Companion to [`crate::wasm_client::map_2d_art`]. Formats `#battlemap-overlay` (toggleable `1.0m`/`5.0m`
//! grid, gait reach envelope, hover reticle, LoS ray, and top-down tokens), synchronizes
//! `#tabletop-map-svg-fallback` using `data-art-key` cache invalidation, and exposes the `#[wasm_bindgen]`
//! view-mode, grid, and hover handlers.

use crate::wasm_client::dom::{
    escape_html, get_element, local_storage_get, local_storage_set, set_button_disabled,
    set_display, set_html, set_select_value, set_text,
};
use crate::wasm_client::map_2d_art::{
    BATTLEMAP_CELL_PX, BATTLEMAP_PAD_PX, battlemap_art_cache_key, battlemap_boundary_path,
    battlemap_cell_rects_path, battlemap_theme_for_preset, format_battlemap_art_layer,
};
use crate::wasm_client::map_2d_tokens::{
    load_persisted_ink_style_pref, push_token_portrait_disc_svg,
};
use crate::wasm_client::map_3d::{
    TacticalMapUiState, format_map_tactical_hud_html, mark_map_wgpu_dirty,
    refresh_map_wgpu_if_active, resolve_token_visual, with_map_ui, with_map_ui_mut,
};
use satya_engine::map_dressing::{make_prop, solid_prop_at};
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

/// Dynamic tactical overlay (inner markup of `#battlemap-overlay`): grid, reach, hover, LoS, tokens.
#[must_use]
pub fn format_battlemap_overlay_inner(ui: &TacticalMapUiState) -> String {
    let t = battlemap_theme_for_preset(ui.map.theme_preset.as_deref());
    let (w, h) = (ui.map.grid_width.max(1), ui.map.grid_height.max(1));
    let c = BATTLEMAP_CELL_PX;
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

    // Gait reach envelope: soft fill + dashed outer boundary (origin included so there's no hole)
    let sel_xy = ui
        .selected_entity_id
        .as_deref()
        .and_then(|id| ui.map.tokens.iter().find(|tk| tk.entity_id == id))
        .map(|tk| (tk.x, tk.y));
    if !ui.reachable_cells.is_empty() {
        let mut mask = vec![false; (w * h) as usize];
        for r in &ui.reachable_cells {
            if r.x >= 0 && r.y >= 0 && r.x < w && r.y < h {
                mask[(r.y * w + r.x) as usize] = true;
            }
        }
        if let Some((sx, sy)) = sel_xy.filter(|&(sx, sy)| sx >= 0 && sy >= 0 && sx < w && sy < h) {
            mask[(sy * w + sx) as usize] = true;
        }
        let inside = |x: i32, y: i32| mask[(y * w + x) as usize];
        let fill_d = battlemap_cell_rects_path(w, h, inside);
        let edge_d = battlemap_boundary_path(w, h, inside);
        let _ = write!(
            out,
            r##"<g pointer-events="none"><path d="{fill_d}" fill="#10b981" fill-opacity=".16"/><path d="{edge_d}" fill="none" stroke="#047857" stroke-width="2.6" stroke-dasharray="7 5" stroke-linecap="round"/></g>"##
        );
    }

    if let Some((hx, hy)) = ui.hovered_cell.filter(|&(hx, hy)| hx >= 0 && hy >= 0 && hx < w && hy < h) {
        if ui.interaction_mode == "edit_terrain" && ui.edit_prop_brush == "__erase__" {
            let (px, py, pw, ph) = solid_prop_at(&ui.map.props, hx, hy)
                .or_else(|| ui.map.props.iter().rev().find(|p| p.covers(hx, hy)))
                .map_or((hx, hy, 1, 1), |p| (p.x, p.y, p.w_cells, p.h_cells));
            let _ = write!(
                out,
                r##"<rect x="{}" y="{}" width="{}" height="{}" rx="7" fill="#f43f5e" fill-opacity=".2" stroke="#e11d48" stroke-width="3" stroke-dasharray="7 4" pointer-events="none"/>"##,
                px * c + 2,
                py * c + 2,
                pw * c - 4,
                ph * c - 4
            );
        } else if ui.interaction_mode == "edit_terrain"
            && let Some(kind) = PropKind::from_code_str(&ui.edit_prop_brush)
        {
            let p = make_prop(kind, hx, hy, ui.edit_prop_rotation_deg, None, 0);
            let in_bounds = p.x >= 0 && p.y >= 0 && p.x + p.w_cells <= w && p.y + p.h_cells <= h;
            let (fill, stroke) = if in_bounds { ("#0ea5e9", "#0284c7") } else { ("#f43f5e", "#e11d48") };
            let _ = write!(
                out,
                r##"<rect x="{}" y="{}" width="{}" height="{}" rx="7" fill="{fill}" fill-opacity=".18" stroke="{stroke}" stroke-width="2.8" stroke-dasharray="7 4" pointer-events="none"/>"##,
                p.x * c + 3,
                p.y * c + 3,
                p.w_cells * c - 6,
                p.h_cells * c - 6
            );
        } else {
            let _ = write!(
                out,
                r##"<rect x="{}" y="{}" width="{}" height="{}" rx="7" fill="#0ea5e9" fill-opacity=".12" stroke="#0284c7" stroke-width="2.6" pointer-events="none"/>"##,
                hx * c + 3,
                hy * c + 3,
                c - 6,
                c - 6
            );
        }
    }

    if let Some(los) = ui.last_los.as_ref() {
        let half = c / 2;
        let (x1, y1) = (los.attacker_xy.0 * c + half, los.attacker_xy.1 * c + half);
        let (x2, y2) = (los.target_xy.0 * c + half, los.target_xy.1 * c + half);
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
        let (mx, my) = ((x1 + x2) / 2, (y1 + y2) / 2);
        let _ = write!(
            out,
            r##"<g pointer-events="none"><line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#fff" stroke-opacity=".7" stroke-width="8" stroke-linecap="round"/><line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="4" stroke-dasharray="11 7" stroke-linecap="round"/><circle cx="{x2}" cy="{y2}" r="13" fill="none" stroke="{stroke}" stroke-width="3"/><path d="M{x2} {}v10M{x2} {}v10M{} {y2}h10M{} {y2}h10" stroke="{stroke}" stroke-width="2.4"/>"##,
            y2 - 20,
            y2 + 10,
            x2 - 20,
            x2 + 10,
        );
        if los.distance_m >= 2.5 {
            let (dx, dy) = ((x2 - x1) as f32, (y2 - y1) as f32);
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let (nx, ny) = (-dy / len * 22.0, dx / len * 22.0);
            let (nx, ny) = if ny > 0.0 { (-nx, -ny) } else { (nx, ny) };
            let _ = write!(
                out,
                r##"<text x="{:.0}" y="{:.0}" text-anchor="middle" font-family="ui-sans-serif,system-ui,sans-serif" font-weight="700" font-size="17" fill="{stroke}" stroke="#fff" stroke-width="4.5" paint-order="stroke">{:.1}m · {}</text>"##,
                mx as f32 + nx,
                my as f32 + ny + 6.0,
                los.distance_m,
                escape_html(&ray_tag),
            );
        }
        out.push_str("</g>");
    }

    for (idx, tok) in ui.map.tokens.iter().enumerate() {
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
    let (w, h) = (ui.map.grid_width.max(1), ui.map.grid_height.max(1));
    let (vw, vh) = (w * BATTLEMAP_CELL_PX, h * BATTLEMAP_CELL_PX);
    let (pad, c) = (BATTLEMAP_PAD_PX, BATTLEMAP_CELL_PX);
    let (tw, th) = (vw + 2 * pad, vh + 2 * pad);
    let view_box = format!("-{pad} -{pad} {tw} {th}");
    let key = battlemap_art_cache_key(ui);
    let pick_js = format!(
        "var r=this.getBoundingClientRect(),fx=((event.clientX-r.left)*{tw}/r.width-{pad})/{c},fy=((event.clientY-r.top)*{th}/r.height-{pad})/{c},x=Math.floor(fx),y=Math.floor(fy)"
    );
    format!(
        r#"<div class="battlemap-stack"><svg id="battlemap-art" class="battlemap-art" data-art-key="{key}" xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" aria-hidden="true">{art}</svg><svg id="battlemap-overlay" class="battlemap-overlay" xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" role="img" aria-label="Illustrated 2D tactical battlemap ({w}m x {h}m, 1.0m grid)" onclick="{pick_js};window.satyaClickMapPoint(fx,fy)" onmousemove="{pick_js},k=x+','+y;this.dataset.hc!==k&amp;&amp;(this.dataset.hc=k,window.satyaHoverMapCell(x,y))" onmouseleave="this.dataset.hc='';window.satyaHoverMapCell(-1,-1)">{overlay}</svg></div>"#,
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
        return;
    }
    host.set_inner_html(&with_map_ui(format_tactical_map_svg_markup));
}

fn apply_battlemap_view_chrome(battlemap_on: bool) {
    set_text(
        "btn-map-view-mode",
        if battlemap_on { "Mode: 2D Battlemap" } else { "Mode: 3D Diorama" },
    );
    for id in ["btn-map-toggle-proj", "btn-map-wall-cutaway", "btn-map-reset-cam"] {
        set_button_disabled(id, battlemap_on);
    }
    set_display("btn-map-grid-overlay", if battlemap_on { "inline-flex" } else { "none" });
    set_display("map-ink-style-wrap", if battlemap_on { "inline-flex" } else { "none" });
    if battlemap_on {
        let style = with_map_ui(|ui| ui.ink_style);
        set_select_value("map-ink-style-select", style.as_str());
        set_display("wgpu-map-canvas-wrap", "none");
        set_display("tabletop-map-svg-fallback", "block");
    } else {
        set_display("tabletop-map-svg-fallback", "none");
        set_display("wgpu-map-canvas-wrap", "block");
    }
}

thread_local! {
    static VIEW_PREF_LOADED: Cell<bool> = const { Cell::new(false) };
}

/// Applies the persisted `localStorage` view mode (`"battlemap"` / `"diorama"`) and paper/ink style once per page load.
pub fn ensure_battlemap_view_pref_loaded() {
    if VIEW_PREF_LOADED.with(|c| c.replace(true)) {
        return;
    }
    let ink = load_persisted_ink_style_pref();
    with_map_ui_mut(|ui| {
        ui.ink_style = ink;
    });
    set_select_value("map-ink-style-select", ink.as_str());
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

/// Toggles the `1.0m` / `5.0m` grid overlay on the 2D battlemap (`#btn-map-grid-overlay`).
#[wasm_bindgen]
pub fn satya_toggle_battlemap_grid() {
    let on = with_map_ui_mut(|ui| {
        ui.show_grid_overlay = !ui.show_grid_overlay;
        ui.show_grid_overlay
    });
    set_text("btn-map-grid-overlay", if on { "Grid: On" } else { "Grid: Off" });
    sync_battlemap_svg_host();
}

/// Pointer hover over battlemap cell `(gx, gy)` (`-1, -1` on leave): updates hover reticle, auto-LoS and HUD.
#[wasm_bindgen]
pub fn satya_hover_map_cell(gx: i32, gy: i32) {
    let changed = with_map_ui_mut(|ui| {
        let cell = (gx >= 0 && gy >= 0 && gx < ui.map.grid_width && gy < ui.map.grid_height)
            .then_some((gx, gy));
        if ui.hovered_cell == cell {
            return false;
        }
        ui.hovered_cell = cell;
        ui.recompute_overlays();
        true
    });
    if changed {
        set_html("tabletop-map-los-hud", &with_map_ui(format_map_tactical_hud_html));
        sync_battlemap_svg_host();
    }
}
