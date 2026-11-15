//! # Facility, Medical, Retail & Industrial 2D SVG Prop Glyphs (`map_2d_art_props_facility`)
//!
//! Companion module to [`crate::wasm_client::map_2d_art_furniture`] keeping each file compact
//! (`<= 380` lines) while rendering detailed top-down SVG glyphs for [`PropKind::Stairwell`],
//! [`PropKind::DisplayRack`], [`PropKind::CheckoutCounter`], and all 12 Phase 5 facility props
//! (`HospitalBed`, `SurgicalTable`, `ImagingScanner`, `PrivacyCurtain`, `MallKiosk`,
//! `ConveyorBelt`, `IndustrialPress`, `PalletRack`, `Forklift`, `ShippingContainer`, `CropRow`,
//! and `PlanterTable`).

use crate::wasm_client::map_2d_art_props::{BATTLEMAP_INK, battlemap_mix_u64};
use satya_schema::PropKind;
use std::fmt::Write as _;

const CONTAINER_HUES: &[&str] = &["#2b5d76", "#8c422c", "#2d5a47", "#3f4c6b"];
const CROP_FELS: &[&str] = &["#4d8b31", "#5d9b3b", "#71a843"];

fn pick(seed: u32, opts: &[&'static str]) -> &'static str {
    opts[(battlemap_mix_u64(u64::from(seed)) % opts.len() as u64) as usize]
}

fn rect(out: &mut String, [x, y, w, h]: [f32; 4], rx: f32, fill: &str, extra: &str) {
    let _ = write!(
        out,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{:.1}" height="{:.1}" rx="{rx:.1}" fill="{fill}" {extra}/>"#,
        w.max(1.0),
        h.max(1.0)
    );
}

fn circle(out: &mut String, cx: f32, cy: f32, r: f32, fill: &str, extra: &str) {
    let _ = write!(out, r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{fill}" {extra}/>"#);
}

/// Renders specialized commercial, medical, agricultural, and industrial props centered at `(0, 0)`
/// spanning `[-lw/2..lw/2] x [-lh/2..lh/2]`. Returns `false` if `kind` belongs in the core matcher.
pub(crate) fn push_facility_furniture_glyph(
    out: &mut String,
    kind: PropKind,
    lw: f32,
    lh: f32,
    painted: bool,
    v: u32,
) -> bool {
    let (l, t) = (lw * 0.5, lh * 0.5);
    let thin = r#"stroke-width="1.1" stroke-opacity=".6""#;
    let wd = if painted { "url(#bm-oak)" } else { "#9e7752" };
    match kind {
        PropKind::Stairwell => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, if painted { "url(#bm-concrete)" } else { "#c8ccd2" }, "");
            let steps = ((lh / 16.0).round() as i32).clamp(5, 10);
            let step_h = (lh - 8.0) / steps as f32;
            for i in 0..steps {
                let sy = -t + 4.0 + i as f32 * step_h;
                let shade = if i % 2 == 0 { "#b8bdc4" } else { "#d5d9df" };
                rect(out, [-l + 5.0, sy, lw - 10.0, (step_h - 1.5).max(3.0)], 1.0, shade, r#"stroke-width="1""#);
            }
            let _ = write!(
                out,
                r##"<line x1="0" y1="{:.1}" x2="0" y2="{:.1}" stroke="#1f2937" stroke-width="2.2"/><path d="M-7 {:.1}L0 {:.1}L7 {:.1}" fill="none" stroke="#1f2937" stroke-width="2.2"/><line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#64748b" stroke-width="3"/><line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#64748b" stroke-width="3"/>"##,
                t - 9.0, -t + 11.0, -t + 18.0, -t + 9.0, -t + 18.0, -l + 4.0, -t + 5.0, -l + 4.0, t - 5.0, l - 4.0, -t + 5.0, l - 4.0, t - 5.0,
            );
        }
        PropKind::DisplayRack => {
            rect(out, [-l + 3.0, -t + 3.0, lw - 6.0, lh - 6.0], 3.0, if painted { "url(#bm-steel)" } else { "#7c8794" }, "");
            let bays = ((lw - 10.0) / 22.0).round().max(2.0) as i32;
            let bw = (lw - 12.0) / bays as f32;
            const PKGS: &[&str] = &["#d97757", "#4f86b6", "#6da665", "#d9b44a", "#9674b8"];
            for b in 0..bays {
                let bx = -l + 6.0 + b as f32 * bw;
                let c1 = pick(v.wrapping_add(b as u32 * 5), PKGS);
                let c2 = pick(v.wrapping_add(b as u32 * 9 + 3), PKGS);
                rect(out, [bx + 2.0, -t + 5.0, (bw - 4.0).max(5.0), (t - 7.0).max(4.0)], 1.5, c1, r#"stroke-width="1""#);
                rect(out, [bx + 2.0, 2.0, (bw - 4.0).max(5.0), (t - 7.0).max(4.0)], 1.5, c2, r#"stroke-width="1""#);
            }
            let _ = write!(out, r##"<line x1="{:.1}" y1="0" x2="{:.1}" y2="0" stroke="#2d343d" stroke-width="2.4"/>"##, -l + 4.0, l - 4.0);
        }
        PropKind::CheckoutCounter => {
            rect(out, [-l + 3.0, -t + 4.0, lw - 6.0, lh - 8.0], 3.0, wd, "");
            rect(out, [-l + 10.0, -t + 8.0, (lw * 0.46).max(16.0), lh - 16.0], 2.0, "#31363f", r#"stroke-width="1.2""#);
            rect(out, [l - 34.0, -t + 9.0, 22.0, 16.0], 2.0, "#dce4ec", r#"stroke-width="1.2""#);
            rect(out, [l - 30.0, -t + 12.0, 14.0, 7.0], 1.0, "#38bdf8", r#"stroke="none" fill-opacity=".8""#);
        }
        PropKind::HospitalBed => {
            rect(out, [-l + 3.0, -t + 4.0, lw - 6.0, lh - 8.0], 4.0, "#94a3b8", "");
            rect(out, [-l + 7.0, -t + 8.0, lw - 14.0, lh - 16.0], 3.0, "#f1f5f9", thin);
            rect(out, [-l + 9.0, -t + 12.0, 18.0, lh - 24.0], 3.0, "#ffffff", thin);
            rect(out, [-l + 30.0, -t + 9.0, lw - 38.0, lh - 18.0], 2.0, "#bae6fd", thin);
            circle(out, -l + 10.0, t - 10.0, 5.5, "#cbd5e1", thin);
        }
        PropKind::SurgicalTable => {
            rect(out, [-l + 6.0, -t + 10.0, lw - 12.0, lh - 20.0], 4.0, "#cbd5e1", "");
            rect(out, [-l + 18.0, -t + 13.0, lw - 32.0, lh - 26.0], 3.0, "#99f6e4", thin);
            rect(out, [-l + 8.0, -10.0, 12.0, 20.0], 3.0, "#e2e8f0", thin);
            circle(out, 0.0, 0.0, 7.0, "#64748b", thin);
            let _ = write!(out, r##"<circle cx="{:.1}" cy="{:.1}" r="11" fill="#e0f2fe" fill-opacity=".55" stroke="#38bdf8" stroke-width="1.4"/>"##, l * 0.25, -t + 8.0);
        }
        PropKind::ImagingScanner => {
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 10.0, "#e2e8f0", "");
            rect(out, [-l + 10.0, -t * 0.45, lw - 20.0, lh * 0.90], 6.0, "#f8fafc", thin);
            circle(out, 0.0, 0.0, (lw.min(lh) * 0.27).max(16.0), "#1e293b", r##"stroke="#38bdf8" stroke-width="2.4""##);
            rect(out, [-18.0, 0.0, 36.0, t - 6.0], 3.0, "#cbd5e1", thin);
        }
        PropKind::PrivacyCurtain => {
            let _ = write!(out, r##"<line x1="{:.1}" y1="0" x2="{:.1}" y2="0" stroke="#64748b" stroke-width="3.2"/>"##, -l + 4.0, l - 4.0);
            let waves = ((lw - 8.0) / 14.0).round().max(4.0) as i32;
            let step = (lw - 8.0) / waves as f32;
            let mut path = format!("M{:.1} 0", -l + 4.0);
            for i in 0..waves {
                let x0 = -l + 4.0 + i as f32 * step;
                let y_peak = if i % 2 == 0 { -9.0 } else { 9.0 };
                let _ = write!(path, " Q{:.1} {:.1} {:.1} 0", x0 + step * 0.5, y_peak, x0 + step);
            }
            let _ = write!(out, r##"<path d="{path}" fill="none" stroke="#67e8f9" stroke-width="6" stroke-linecap="round" opacity=".88"/><path d="{path}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.6"/>"##);
        }
        PropKind::MallKiosk => {
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 10.0, wd, "");
            rect(out, [-l + 12.0, -t + 12.0, lw - 24.0, lh - 24.0], 6.0, "#bfdbfe", r##"fill-opacity=".72" stroke-width="1.4""##);
            rect(out, [-16.0, -10.0, 32.0, 20.0], 3.0, "#334155", thin);
            for (px, py) in [(-l + 9.0, -t + 9.0), (l - 9.0, -t + 9.0), (-l + 9.0, t - 9.0), (l - 9.0, t - 9.0)] {
                circle(out, px, py, 3.5, "#94a3b8", thin);
            }
        }
        PropKind::ConveyorBelt => {
            rect(out, [-l + 2.0, -t + 4.0, lw - 4.0, lh - 8.0], 3.0, "#64748b", "");
            rect(out, [-l + 5.0, -t + 9.0, lw - 10.0, lh - 18.0], 2.0, "#27272a", thin);
            let rollers = ((lw - 14.0) / 14.0).round().max(4.0) as i32;
            let step = (lw - 14.0) / rollers as f32;
            for i in 0..=rollers {
                let rx = -l + 7.0 + i as f32 * step;
                let _ = write!(out, r##"<line x1="{rx:.1}" y1="{:.1}" x2="{rx:.1}" y2="{:.1}" stroke="#52525b" stroke-width="1.3"/>"##, -t + 10.0, t - 10.0);
            }
            rect(out, [-l * 0.45, -10.0, 18.0, 20.0], 2.0, "#c29258", thin);
            rect(out, [l * 0.15, -9.0, 16.0, 18.0], 2.0, "#b8864c", thin);
        }
        PropKind::IndustrialPress => {
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 5.0, "#d97706", "");
            rect(out, [-l + 12.0, -t + 12.0, lw - 24.0, lh - 24.0], 3.0, "#475569", thin);
            rect(out, [-l + 6.0, t - 11.0, lw - 12.0, 6.0], 1.0, "#facc15", r#"stroke="none""#);
            circle(out, -l * 0.34, 0.0, 11.0, "#94a3b8", thin);
            circle(out, l * 0.34, 0.0, 11.0, "#94a3b8", thin);
        }
        PropKind::PalletRack => {
            rect(out, [-l + 2.0, -t + 4.0, lw - 4.0, lh - 8.0], 2.0, "#c2410c", "");
            rect(out, [-l + 6.0, -t + 8.0, lw - 12.0, lh - 16.0], 1.5, "#78716c", thin);
            let bays = ((lw - 14.0) / 36.0).round().max(2.0) as i32;
            let bw = (lw - 16.0) / bays as f32;
            for b in 0..bays {
                let bx = -l + 8.0 + b as f32 * bw;
                rect(out, [bx + 2.0, -t + 10.0, (bw - 4.0).max(10.0), lh - 20.0], 2.0, "#caa066", thin);
            }
        }
        PropKind::Forklift => {
            rect(out, [-l + 4.0, -t + 10.0, lw * 0.60, lh - 20.0], 6.0, "#eab308", "");
            rect(out, [-l + 10.0, -t + 15.0, lw * 0.36, lh - 30.0], 3.0, "#1e293b", thin);
            rect(out, [l * 0.16, -t + 14.0, l * 0.78, 5.0], 1.0, "#cbd5e1", thin);
            rect(out, [l * 0.16, t - 19.0, l * 0.78, 5.0], 1.0, "#cbd5e1", thin);
            rect(out, [l * 0.08, -t + 8.0, 7.0, lh - 16.0], 1.5, "#334155", thin);
        }
        PropKind::ShippingContainer => {
            let hull = pick(v, CONTAINER_HUES);
            rect(out, [-l + 2.0, -t + 3.0, lw - 4.0, lh - 6.0], 2.5, hull, "");
            let ribs = ((lw - 16.0) / 16.0).round().max(6.0) as i32;
            let step = (lw - 16.0) / ribs as f32;
            for i in 0..=ribs {
                let rx = -l + 8.0 + i as f32 * step;
                let _ = write!(out, r##"<line x1="{rx:.1}" y1="{:.1}" x2="{rx:.1}" y2="{:.1}" stroke="#0f172a" stroke-opacity=".35" stroke-width="2.2"/>"##, -t + 7.0, t - 7.0);
            }
            for (cx, cy) in [(-l + 4.0, -t + 5.0), (l - 10.0, -t + 5.0), (-l + 4.0, t - 11.0), (l - 10.0, t - 11.0)] {
                rect(out, [cx, cy, 6.0, 6.0], 1.0, "#94a3b8", thin);
            }
        }
        PropKind::CropRow => {
            rect(out, [-l + 2.0, -t + 8.0, lw - 4.0, lh - 16.0], 8.0, "#5c4028", r#"stroke="none" opacity=".72""#);
            let tufts = ((lw - 12.0) / 22.0).round().max(4.0) as i32;
            let step = (lw - 14.0) / tufts as f32;
            for i in 0..tufts {
                let cx = -l + 7.0 + (i as f32 + 0.5) * step;
                let leaf = pick(v.wrapping_add(i as u32 * 7), CROP_FELS);
                circle(out, cx, 0.0, (t * 0.48).clamp(9.0, 17.0), leaf, thin);
                circle(out, cx - 3.0, -2.0, 4.5, "#a3e635", r#"stroke="none" fill-opacity=".65""#);
            }
        }
        PropKind::PlanterTable => {
            rect(out, [-l + 3.0, -t + 4.0, lw - 6.0, lh - 8.0], 3.0, wd, "");
            rect(out, [-l + 7.0, -t + 8.0, lw - 14.0, lh - 16.0], 2.0, "#4a3321", thin);
            let plants = ((lw - 18.0) / 24.0).round().max(2.0) as i32;
            let step = (lw - 18.0) / plants as f32;
            for i in 0..plants {
                let px = -l + 9.0 + (i as f32 + 0.5) * step;
                circle(out, px, 0.0, (t * 0.44).clamp(8.0, 15.0), "#3f7d3a", thin);
                circle(out, px, 0.0, 4.2, "#c2410c", r#"stroke="none" fill-opacity=".75""#);
            }
        }
        _ => return false,
    }
    true
}
