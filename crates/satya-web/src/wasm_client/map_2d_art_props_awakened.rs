//! # 2D Battlemap Props & Spatial Overlays for *Awakened* 2035 (`map_2d_art_props_awakened`)
//!
//! Renders top-down architectural glyphs for the 10 *Awakened* 2035 tech/grid [`PropKind`]s
//! (`CryoComputeCluster` through `WaterPurifierKiosk`) and draws active-floor spatial overlays:
//! - Enclosed [`MapFaradayZone`] copper-mesh RF shielding perimeters + `dB` labels.
//! - Active/offline [`MapSurveillanceNode`] compass-accurate FOV cones & emitter markers.
//! - Polycrisis brownout/blackout viewport Dusk wash when `ambient_light_delta_ext > 0`.

use std::fmt::Write as _;

use satya_schema::{MessageVisibility, PropKind};

use crate::wasm_client::map_2d_art::grid_dims;
use crate::wasm_client::map_2d_art_furniture::{metallic_coat_overlay, porcelain_overlay};
use crate::wasm_client::map_3d::TacticalMapUiState;

fn rect(out: &mut String, [x, y, w, h]: [f32; 4], rx: f32, fill: &str, extra: &str) {
    let _ = write!(
        out,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{:.1}" height="{:.1}" rx="{rx:.1}" fill="{fill}" {extra}/>"#,
        w.max(1.0),
        h.max(1.0)
    );
}

fn circle(out: &mut String, cx: f32, cy: f32, r: f32, fill: &str, extra: &str) {
    let _ = write!(
        out,
        r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{fill}" {extra}/>"#
    );
}

fn line(out: &mut String, x1: f32, y1: f32, x2: f32, y2: f32, extra: &str) {
    let _ = write!(
        out,
        r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" {extra}/>"#
    );
}

/// Renders top-down 2D art for an *Awakened* 2035 prop inside the caller's rotated `<g>`.
///
/// Returns `false` if `kind` belongs to another prop glyph module.
pub(crate) fn push_awakened_furniture_glyph(
    out: &mut String,
    kind: PropKind,
    lw: f32,
    lh: f32,
    painted: bool,
    v: u32,
) -> bool {
    let (l, t) = (lw * 0.5, lh * 0.5);
    let thin = r#"stroke-width="1.1" stroke-opacity=".65""#;
    match kind {
        PropKind::CryoComputeCluster => {
            let chassis = [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0];
            rect(out, chassis, 3.5, "#1e293b", "");
            metallic_coat_overlay(out, chassis, 3.5, painted);
            rect(
                out,
                [-l + 9.0, -t + 8.0, lw - 18.0, lh - 16.0],
                2.0,
                "#0f172a",
                thin,
            );
            let led = if v.is_multiple_of(2) {
                "#22d3ee"
            } else {
                "#38bdf8"
            };
            rect(
                out,
                [-l + 12.0, -t + 11.0, (lw - 24.0).max(8.0), 5.0],
                1.5,
                led,
                r#"stroke="none" fill-opacity=".85""#,
            );
            line(
                out,
                -l + 12.0,
                t - 12.0,
                l - 12.0,
                t - 12.0,
                r##"stroke="#d97706" stroke-width="2.2""##,
            );
            circle(out, -l + 15.0, 2.0, 3.5, "#94a3b8", thin);
            circle(out, l - 15.0, 2.0, 3.5, "#94a3b8", thin);
        }
        PropKind::BioServerVat => {
            let r_outer = lw.min(lh) * 0.42;
            circle(out, 0.0, 0.0, r_outer, "#334155", "");
            circle(out, 0.0, 0.0, (r_outer - 5.0).max(6.0), "#0891b2", thin);
            circle(
                out,
                0.0,
                0.0,
                (r_outer * 0.52).max(4.0),
                "#67e8f9",
                r#"stroke="none" fill-opacity=".72""#,
            );
            circle(
                out,
                -2.0,
                -2.0,
                (r_outer * 0.22).max(2.0),
                "#ecfeff",
                r#"stroke="none" fill-opacity=".85""#,
            );
            line(
                out,
                -r_outer + 3.0,
                0.0,
                r_outer - 3.0,
                0.0,
                r##"stroke="#b45309" stroke-width="1.6""##,
            );
        }
        PropKind::AirGappedTerminal => {
            let desk = [-l + 5.0, -t + 6.0, lw - 10.0, lh - 12.0];
            rect(out, desk, 3.0, "#475569", "");
            metallic_coat_overlay(out, desk, 3.0, painted);
            rect(
                out,
                [-l + 9.0, -t + 8.0, lw - 18.0, 4.0],
                1.0,
                "#b45309",
                r#"stroke="none""#,
            );
            rect(out, [-15.0, -4.0, 13.0, 10.0], 1.5, "#0f172a", thin);
            rect(out, [2.0, -4.0, 13.0, 10.0], 1.5, "#0f172a", thin);
            rect(
                out,
                [-13.0, -2.0, 9.0, 5.0],
                0.8,
                "#22c55e",
                r#"stroke="none" fill-opacity=".8""#,
            );
            rect(
                out,
                [-10.0, 9.0, 20.0, 7.0],
                1.2,
                "#cbd5e1",
                r#"stroke-width=".8""#,
            );
        }
        PropKind::BiometricTurnstile => {
            let span = (lw - 10.0).max(16.0);
            let p_top = [-l + 5.0, -t + 4.0, span, 11.0];
            let p_bot = [-l + 5.0, t - 15.0, span, 11.0];
            rect(out, p_top, 3.0, "#64748b", "");
            rect(out, p_bot, 3.0, "#64748b", "");
            metallic_coat_overlay(out, p_top, 3.0, painted);
            metallic_coat_overlay(out, p_bot, 3.0, painted);
            rect(
                out,
                [-6.0, -t + 14.0, 12.0, (lh - 28.0).max(6.0)],
                1.5,
                "#93c5fd",
                r##"fill-opacity=".55" stroke="#0284c7" stroke-width="1""##,
            );
            circle(out, l - 11.0, -t + 9.5, 2.5, "#22c55e", r#"stroke="none""#);
            circle(out, l - 11.0, t - 9.5, 2.5, "#ef4444", r#"stroke="none""#);
        }
        PropKind::SecurityCameraMast => {
            let base = [-l * 0.48, -t * 0.48, (lw * 0.48).max(10.0), (lh * 0.48).max(10.0)];
            rect(out, base, 3.0, "#475569", "");
            metallic_coat_overlay(out, base, 3.0, painted);
            circle(out, 0.0, 0.0, 7.0, "#e2e8f0", thin);
            rect(out, [0.0, -4.5, 16.0, 9.0], 2.0, "#cbd5e1", thin);
            circle(out, 15.0, 0.0, 3.0, "#020617", r#"stroke="none""#);
            circle(out, 15.0, 0.0, 1.3, "#ef4444", r#"stroke="none""#);
        }
        PropKind::DroneDockPad => {
            let pad = [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0];
            rect(out, pad, 6.0, "#334155", "");
            metallic_coat_overlay(out, pad, 6.0, painted);
            let ring_r = lw.min(lh) * 0.30;
            circle(
                out,
                0.0,
                0.0,
                ring_r,
                "none",
                r##"stroke="#facc15" stroke-width="2.4" stroke-dasharray="9 4""##,
            );
            line(
                out,
                -ring_r * 0.42,
                -ring_r * 0.48,
                -ring_r * 0.42,
                ring_r * 0.48,
                r##"stroke="#eab308" stroke-width="2.4""##,
            );
            line(
                out,
                ring_r * 0.42,
                -ring_r * 0.48,
                ring_r * 0.42,
                ring_r * 0.48,
                r##"stroke="#eab308" stroke-width="2.4""##,
            );
            line(
                out,
                -ring_r * 0.42,
                0.0,
                ring_r * 0.42,
                0.0,
                r##"stroke="#eab308" stroke-width="2.4""##,
            );
        }
        PropKind::HvTransformer => {
            let core = [-l + 8.0, -t + 8.0, lw - 16.0, lh - 16.0];
            rect(out, core, 4.0, "#4b5563", "");
            metallic_coat_overlay(out, core, 4.0, painted);
            let fin_span = (lw - 22.0).max(12.0);
            for offset in [-0.28_f32, 0.0, 0.28] {
                let y = offset * lh;
                line(
                    out,
                    -fin_span * 0.5,
                    y,
                    fin_span * 0.5,
                    y,
                    r##"stroke="#1e293b" stroke-width="2""##,
                );
            }
            for x in [-l * 0.42, 0.0, l * 0.42] {
                let bush = [x - 5.0, -t + 2.0, 10.0, 10.0];
                circle(out, x, -t + 7.0, 5.0, "#e2e8f0", thin);
                porcelain_overlay(out, bush, 4.0, painted);
            }
            rect(out, [-9.0, -7.0, 18.0, 14.0], 2.0, "#facc15", thin);
        }
        PropKind::DieselGenerator => {
            let tint = if v.is_multiple_of(3) {
                "#ca8a04"
            } else {
                "#b45309"
            };
            let body = [-l + 5.0, -t + 5.0, lw - 10.0, lh - 10.0];
            rect(out, body, 4.0, tint, "");
            metallic_coat_overlay(out, body, 4.0, painted);
            rect(
                out,
                [-l + 11.0, -t + 9.0, (lw * 0.56).max(12.0), lh - 18.0],
                2.0,
                "#334155",
                thin,
            );
            circle(out, l - 14.0, -t + 11.0, 4.5, "#111827", thin);
            circle(out, l - 14.0, t - 11.0, 3.0, "#ef4444", r#"stroke="none""#);
        }
        PropKind::BatteryBankWall => {
            let rack = [-l + 4.0, -t + 5.0, lw - 8.0, lh - 10.0];
            rect(out, rack, 3.0, "#1e293b", "");
            metallic_coat_overlay(out, rack, 3.0, painted);
            let blades = 4;
            let bw = ((lw - 16.0) / blades as f32).max(6.0);
            for i in 0..blades {
                let bx = -l + 8.0 + i as f32 * bw;
                rect(out, [bx + 1.0, -t + 9.0, bw - 3.0, lh - 18.0], 1.5, "#334155", thin);
                rect(
                    out,
                    [bx + 3.0, -2.5, (bw - 7.0).max(2.0), 5.0],
                    1.0,
                    "#22c55e",
                    r#"stroke="none""#,
                );
            }
            line(
                out,
                -l + 8.0,
                -t + 7.0,
                l - 8.0,
                -t + 7.0,
                r##"stroke="#d97706" stroke-width="2""##,
            );
        }
        PropKind::WaterPurifierKiosk => {
            let tank = [-l + 7.0, -t + 7.0, lw - 14.0, lh - 14.0];
            rect(out, tank, 5.0, "#0284c7", "");
            metallic_coat_overlay(out, tank, 5.0, painted);
            circle(out, -lw * 0.14, 0.0, 6.5, "#e0f2fe", thin);
            circle(out, lw * 0.14, 0.0, 6.5, "#e0f2fe", thin);
            rect(
                out,
                [-10.0, t - 13.0, 20.0, 6.0],
                1.5,
                "#0f172a",
                r#"stroke="none""#,
            );
        }
        _ => return false,
    }
    true
}

/// Emits active-floor *Awakened* 2035 spatial overlays (blackout tint, Faraday cages, and
/// surveillance FOV cones) inside `battlemap_art_layer` above architectural walls and canopies.
pub(crate) fn push_spatial_overlays_svg(out: &mut String, ui: &TacticalMapUiState, cell_px: f32) {
    let active_floor = ui.map.active_floor;
    let has_faraday = ui
        .map
        .faraday_zones
        .iter()
        .any(|z| z.floor_index == active_floor);
    let has_nodes = ui
        .map
        .surveillance_nodes
        .iter()
        .any(|n| n.floor_index == active_floor);
    let has_blackout = ui.map.ambient_light_delta_ext > 0;
    if !has_faraday && !has_nodes && !has_blackout {
        return;
    }
    let (w, h) = grid_dims(ui);
    let (vw, vh) = (w as f32 * cell_px, h as f32 * cell_px);

    out.push_str(r#"<g id="bm-awakened-overlays" pointer-events="none">"#);
    if has_blackout {
        let op = (ui.map.ambient_light_delta_ext as f32 * 0.085).clamp(0.08, 0.34);
        let _ = write!(
            out,
            r##"<rect class="bm-blackout-wash" x="0" y="0" width="{vw:.1}" height="{vh:.1}" fill="#09101d" fill-opacity="{op:.2}"/>"##
        );
    }

    for z in &ui.map.faraday_zones {
        if z.floor_index != active_floor {
            continue;
        }
        let x0 = z.x0_cm.min(z.x1_cm) as f32 * 0.01 * cell_px;
        let y0 = z.y0_cm.min(z.y1_cm) as f32 * 0.01 * cell_px;
        let zw = ((z.x1_cm - z.x0_cm).abs() as f32 * 0.01 * cell_px).max(8.0);
        let zh = ((z.y1_cm - z.y0_cm).abs() as f32 * 0.01 * cell_px).max(8.0);
        let _ = write!(
            out,
            r##"<rect class="bm-faraday-zone" x="{x0:.1}" y="{y0:.1}" width="{zw:.1}" height="{zh:.1}" rx="4" fill="#d97706" fill-opacity=".11" stroke="#b45309" stroke-width="2" stroke-dasharray="8 4"/><rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="none" stroke="#f59e0b" stroke-opacity=".45" stroke-width="1" stroke-dasharray="2 6"/><rect x="{:.1}" y="{:.1}" width="88" height="14" rx="3" fill="#451a03" fill-opacity=".82"/><text x="{:.1}" y="{:.1}" fill="#fcd34d" font-family="ui-monospace,monospace" font-size="9" font-weight="700">FARADAY -{} dB</text>"##,
            x0 + 3.0,
            y0 + 3.0,
            (zw - 6.0).max(2.0),
            (zh - 6.0).max(2.0),
            x0 + 4.0,
            y0 + 4.0,
            x0 + 9.0,
            y0 + 13.5,
            z.attenuation_db
        );
    }

    for n in &ui.map.surveillance_nodes {
        if n.floor_index != active_floor {
            continue;
        }
        let nx = n.x_cm as f32 * 0.01 * cell_px;
        let ny = n.y_cm as f32 * 0.01 * cell_px;
        let range = (n.range_cm.max(100) as f32 * 0.01 * cell_px).min(18.0 * cell_px);
        let fov = n.fov_deg.clamp(15, 360);
        let (fill, stroke, dot, fill_op, dash) = match (n.powered, n.visibility) {
            (true, MessageVisibility::GmOnly) => ("#a855f7", "#c084fc", "#a855f7", ".14", "6 3"),
            (true, _) => ("#ef4444", "#f59e0b", "#ef4444", ".14", "6 3"),
            (false, _) => ("#64748b", "#64748b", "#475569", ".07", "2 5"),
        };
        if fov >= 359 {
            let _ = write!(
                out,
                r#"<circle class="bm-surv-fov" cx="{nx:.1}" cy="{ny:.1}" r="{range:.1}" fill="{fill}" fill-opacity="{fill_op}" stroke="{stroke}" stroke-width="1.3" stroke-dasharray="{dash}"/>"#
            );
        } else {
            let half = fov as f32 * 0.5;
            let a0 = (n.facing_deg as f32 - half).to_radians();
            let a1 = (n.facing_deg as f32 + half).to_radians();
            let (x0, y0) = (nx + range * a0.sin(), ny - range * a0.cos());
            let (x1, y1) = (nx + range * a1.sin(), ny - range * a1.cos());
            let large = i32::from(fov > 180);
            let _ = write!(
                out,
                r#"<path class="bm-surv-fov" d="M{nx:.1} {ny:.1} L{x0:.1} {y0:.1} A{range:.1} {range:.1} 0 {large} 1 {x1:.1} {y1:.1} Z" fill="{fill}" fill-opacity="{fill_op}" stroke="{stroke}" stroke-width="1.3" stroke-dasharray="{dash}"/>"#
            );
        }
        let aim = (n.facing_deg as f32).to_radians();
        let _ = write!(
            out,
            r##"<circle cx="{nx:.1}" cy="{ny:.1}" r="4.2" fill="{dot}" stroke="#0f172a" stroke-width="1.2"/><line x1="{nx:.1}" y1="{ny:.1}" x2="{:.1}" y2="{:.1}" stroke="{dot}" stroke-width="1.6"/>"##,
            nx + 8.5 * aim.sin(),
            ny - 8.5 * aim.cos()
        );
    }

    out.push_str("</g>");
}
