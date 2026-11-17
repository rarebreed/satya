//! # Medical, Laboratory, Bath, Elevator & Streetscape 2D SVG Glyphs (`map_2d_art_props_lab_urban`)
//!
//! Companion to [`crate::wasm_client::map_2d_art_furniture`] implementing top-down SVG glyphs
//! for cots/gurneys/nurse stations/telemetry/elevators, laboratory instrumentation (`LabBench`,
//! `DeskClutter`, `SolderingStation`, `TestBench`, `Printer3dStation`, `AirCompressor`, `CncMill`,
//! `ServerRack`, `FumeHood`), modern bath/laundry fixtures (`ShowerStall`, `SpaBath`, `LinenCloset`,
//! `WasherDryer`, `SingleBed`), office amenities (`FilingCabinet`, `WaterCooler`), and urban
//! streetscapes (`BrownstoneStoop`, `AwningCanopy`).

use crate::wasm_client::map_2d_art_furniture::{FURNITURE_WOODS, wood_veneer_overlay};
use crate::wasm_client::map_2d_art_props::{BATTLEMAP_INK, battlemap_mix_u64};
use crate::wasm_client::map_2d_art_textures::BattlemapTextureMode;
use satya_schema::{MapPropPlacement, PropKind, format_rgb_hex};
use std::fmt::Write as _;

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
    let _ = write!(
        out,
        r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{fill}" {extra}/>"#
    );
}

/// Renders medical, lab, elevator, bath, office, and urban props inside an already opened `<g>`.
/// Returns `false` if unhandled.
pub(crate) fn push_lab_urban_furniture_glyph(
    out: &mut String,
    kind: PropKind,
    lw: f32,
    lh: f32,
    painted: bool,
    v: u32,
) -> bool {
    let (l, t) = (lw * 0.5, lh * 0.5);
    let thin = r#"stroke-width="1.1" stroke-opacity=".65""#;
    let wd = pick(v, FURNITURE_WOODS);
    match kind {
        PropKind::Cot => {
            rect(out, [-l + 6.0, -t + 5.0, lw - 12.0, lh - 10.0], 3.0, "#4b5563", "");
            rect(out, [-l + 9.0, -t + 8.0, lw - 18.0, lh - 16.0], 2.5, "#6b7b5c", thin);
            rect(out, [-l + 10.0, -10.0, 12.0, 20.0], 2.0, "#e5e7eb", thin);
        }
        PropKind::Gurney => {
            rect(out, [-l + 4.0, -t + 6.0, lw - 8.0, lh - 12.0], 4.0, "#94a3b8", "");
            rect(out, [-l + 8.0, -t + 10.0, lw - 16.0, lh - 20.0], 3.0, "#f8fafc", thin);
            rect(out, [-l + 9.0, -11.0, 13.0, 22.0], 2.0, "#ffffff", thin);
            let _ = write!(
                out,
                r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#0284c7" stroke-width="2"/><line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#0284c7" stroke-width="2"/>"##,
                -l + 12.0,
                -t + 7.0,
                l - 12.0,
                -t + 7.0,
                -l + 12.0,
                t - 7.0,
                l - 12.0,
                t - 7.0
            );
        }
        PropKind::ReclinerChair => {
            let c = pick(v, &["#4f6d8a", "#5f7468", "#70607a"]);
            rect(out, [-l + 8.0, -t + 7.0, lw - 16.0, lh - 14.0], 6.0, c, "");
            rect(out, [-l + 8.0, -t + 7.0, lw - 16.0, 12.0], 4.0, "#334155", thin);
            rect(out, [-l + 11.0, 2.0, lw - 22.0, t - 10.0], 3.0, "#64748b", thin);
        }
        PropKind::MedicalCounter => {
            rect(out, [-l + 3.0, -t + 4.0, lw - 6.0, lh - 10.0], 3.0, "#cbd5e1", "");
            rect(out, [-l + 5.0, -t + 6.0, lw - 10.0, lh - 14.0], 2.0, "#f1f5f9", thin);
            rect(out, [l - 22.0, -t + 9.0, 14.0, 12.0], 2.0, "#94a3b8", thin);
            circle(out, l - 15.0, -t + 15.0, 2.0, "#334155", "stroke='none'");
        }
        PropKind::NurseStation => {
            let outer = [-l + 4.0, -t + 6.0, lw - 8.0, lh - 12.0];
            rect(out, outer, 6.0, "#d6dee5", "");
            wood_veneer_overlay(out, outer, 6.0, painted);
            rect(out, [-l + 16.0, -t + 18.0, lw - 32.0, lh - 32.0], 4.0, "#f8fafc", thin);
            for (mx, my) in [(-l + 24.0, -t + 10.0), (0.0, -t + 10.0), (l - 38.0, -t + 10.0)] {
                rect(out, [mx, my, 16.0, 7.0], 1.5, "#1e293b", thin);
                rect(out, [mx + 2.0, my + 2.0, 12.0, 3.0], 1.0, "#38bdf8", r#"stroke="none""#);
            }
            circle(out, -l + 28.0, 6.0, 8.5, "#334155", thin);
            circle(out, l - 28.0, 6.0, 8.5, "#334155", thin);
        }
        PropKind::TelemetryMonitor => {
            circle(out, -6.0, 0.0, 9.0, "#334155", "");
            rect(out, [-11.0, -10.0, 14.0, 10.0], 2.0, "#0f172a", thin);
            rect(out, [-9.0, -8.0, 10.0, 6.0], 1.0, "#22c55e", r#"stroke="none""#);
            circle(out, 9.0, 2.0, 6.0, "#94a3b8", thin);
            circle(out, 9.0, 2.0, 3.0, "#e0f2fe", r#"stroke="none""#);
        }
        PropKind::BiohazardBin => {
            rect(out, [-18.0, -14.0, 16.0, 22.0], 3.0, "#b91c1c", "");
            circle(out, -10.0, -3.0, 4.0, "#fef08a", r#"stroke="none""#);
            rect(out, [2.0, -14.0, 16.0, 22.0], 3.0, "#2563eb", "");
            rect(out, [4.0, -11.0, 12.0, 16.0], 2.0, "#dbeafe", thin);
        }
        PropKind::LinenCart => {
            rect(out, [-l + 5.0, -t + 6.0, lw - 10.0, lh - 12.0], 4.0, "#94a3b8", "");
            rect(out, [-l + 9.0, -t + 10.0, lw - 18.0, (lh - 20.0) * 0.55], 2.0, "#f8fafc", thin);
            rect(out, [-l + 9.0, 0.0, lw - 18.0, (lh - 22.0) * 0.45], 2.0, "#bae6fd", thin);
        }
        PropKind::ElevatorCar | PropKind::ServiceElevator => {
            let steel = if painted { "url(#bm-steel)" } else { "#9aa4ae" };
            rect(out, [-l + 3.0, -t + 3.0, lw - 6.0, lh - 6.0], 4.0, steel, "");
            rect(out, [-l + 8.0, -t + 8.0, lw - 16.0, lh - 16.0], 2.5, "#64748b", thin);
            let _ = write!(
                out,
                r##"<line x1="0" y1="{:.1}" x2="0" y2="{:.1}" stroke="#1e293b" stroke-width="2.4"/><path d="M-8 -8L0 -18L8 -8M-8 8L0 18L8 8" fill="none" stroke="#f8fafc" stroke-width="2" stroke-linecap="round"/>"##,
                t - 7.0,
                t - 1.0
            );
            if kind == PropKind::ServiceElevator {
                rect(
                    out,
                    [-l + 10.0, -t + 10.0, lw - 20.0, 5.0],
                    1.0,
                    "#facc15",
                    r#"stroke="none" opacity=".75""#,
                );
            }
        }
        PropKind::LabBench => {
            rect(out, [-l + 3.0, -t + 4.0, lw - 6.0, lh - 10.0], 3.0, "#1e293b", "");
            rect(out, [-l + 6.0, -t + 7.0, lw - 12.0, lh - 16.0], 2.0, "#334155", thin);
            rect(out, [-l + 12.0, -t + 10.0, 18.0, 12.0], 1.5, "#64748b", thin);
            circle(out, l - 18.0, -t + 16.0, 5.0, "#38bdf8", thin);
        }
        PropKind::DeskClutter => {
            rect(out, [-16.0, -10.0, 13.0, 17.0], 1.0, "#f8fafc", r#"transform="rotate(-11 -9 -2)" stroke-width="1""#);
            rect(out, [-4.0, -8.0, 12.0, 15.0], 1.0, "#fef3c7", r#"transform="rotate(9 2 0)" stroke-width="1""#);
            rect(out, [-12.0, 4.0, 22.0, 7.0], 1.5, "#334155", r#"stroke-width=".9""#);
            circle(out, 15.0, 6.0, 4.5, "#b45309", r#"stroke-width="1""#);
            circle(out, 15.0, 6.0, 2.8, "#451a03", r#"stroke="none""#);
        }
        PropKind::SolderingStation => {
            rect(out, [-l + 3.0, -t + 4.0, lw - 6.0, lh - 10.0], 3.0, "#1e3a3a", "");
            rect(out, [-18.0, -t + 10.0, 36.0, lh - 22.0], 2.0, "#15803d", thin);
            rect(out, [-l + 8.0, -t + 8.0, 14.0, 14.0], 2.0, "#38bdf8", thin);
            let _ = write!(
                out,
                r##"<line x1="{:.1}" y1="-4" x2="8" y2="8" stroke="#f97316" stroke-width="2" stroke-linecap="round"/>"##,
                l - 12.0
            );
        }
        PropKind::TestBench => {
            rect(out, [-l + 3.0, -t + 4.0, lw - 6.0, lh - 10.0], 3.0, "#334155", "");
            rect(out, [-l + 7.0, -t + 7.0, 28.0, 18.0], 2.0, "#0f172a", thin);
            rect(out, [-l + 10.0, -t + 10.0, 22.0, 12.0], 1.0, "#22c55e", r#"stroke="none""#);
            rect(out, [4.0, -t + 7.0, 24.0, 16.0], 2.0, "#1e293b", thin);
            circle(out, 10.0, -t + 15.0, 3.0, "#38bdf8", r#"stroke="none""#);
            circle(out, 19.0, -t + 15.0, 3.0, "#facc15", r#"stroke="none""#);
        }
        PropKind::Printer3dStation => {
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 3.0, "#475569", "");
            rect(out, [-l + 10.0, -t + 9.0, 26.0, lh - 18.0], 2.0, "#0f172a", thin);
            rect(out, [-l + 14.0, -t + 13.0, 18.0, lh - 26.0], 1.0, "#38bdf8", thin);
            rect(out, [8.0, -t + 10.0, 20.0, lh - 20.0], 2.0, "#7c3aed", thin);
        }
        PropKind::AirCompressor => {
            circle(out, -4.0, 0.0, 16.0, "#b91c1c", "");
            rect(out, [-14.0, -9.0, 26.0, 18.0], 6.0, "#dc2626", thin);
            circle(out, 12.0, 0.0, 6.0, "#cbd5e1", thin);
            circle(out, 0.0, 0.0, 5.0, "#1e293b", thin);
        }
        PropKind::CncMill => {
            let steel = if painted { "url(#bm-steel)" } else { "#64748b" };
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 5.0, steel, "");
            rect(out, [-l + 14.0, -t + 12.0, lw - 42.0, lh - 24.0], 3.0, "#1e293b", thin);
            circle(out, -8.0, 0.0, 12.0, "#94a3b8", thin);
            rect(out, [l - 26.0, -t + 12.0, 16.0, 26.0], 2.0, "#0f172a", thin);
            rect(out, [l - 23.0, -t + 16.0, 10.0, 10.0], 1.0, "#38bdf8", r#"stroke="none""#);
        }
        PropKind::ServerRack => {
            rect(out, [-l + 5.0, -t + 5.0, lw - 10.0, lh - 10.0], 2.5, "#0f172a", "");
            rect(out, [-l + 9.0, -t + 9.0, lw - 18.0, lh - 18.0], 1.5, "#1e293b", thin);
            let _ = write!(
                out,
                r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#38bdf8" stroke-width="2"/><circle cx="{:.1}" cy="{:.1}" r="2.2" fill="#22c55e" stroke="none"/><circle cx="{:.1}" cy="{:.1}" r="2.2" fill="#38bdf8" stroke="none"/>"##,
                -l + 12.0,
                t - 12.0,
                l - 12.0,
                t - 12.0,
                -l + 14.0,
                -t + 14.0,
                -l + 22.0,
                -t + 14.0
            );
        }
        PropKind::FumeHood => {
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 3.0, "#e2e8f0", "");
            rect(out, [-l + 8.0, -t + 10.0, lw - 16.0, lh - 18.0], 2.0, "#bae6fd", thin);
            rect(out, [-l + 12.0, t - 9.0, lw - 24.0, 4.0], 1.0, "#38bdf8", thin);
        }
        PropKind::ShowerStall => {
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 4.0, "#e0f2fe", "");
            rect(out, [-l + 7.0, -t + 7.0, lw - 14.0, lh - 14.0], 2.5, "#bae6fd", thin);
            circle(out, 0.0, 0.0, 4.0, "#64748b", thin);
            circle(out, 0.0, -t + 11.0, 4.5, "#94a3b8", thin);
        }
        PropKind::SpaBath => {
            rect(out, [-l + 5.0, -t + 5.0, lw - 10.0, lh - 10.0], 10.0, "#f1f5f9", "");
            rect(out, [-l + 12.0, -t + 12.0, lw - 24.0, lh - 24.0], 16.0, "#38bdf8", thin);
            for (jx, jy) in [(-14.0, -14.0), (14.0, -14.0), (-14.0, 14.0), (14.0, 14.0)] {
                circle(out, jx, jy, 4.0, "#e0f2fe", r#"stroke="none" fill-opacity=".65""#);
            }
        }
        PropKind::LinenCloset => {
            rect(out, [-l + 5.0, -t + 4.0, lw - 10.0, lh - 8.0], 3.0, "#dfd8cc", "");
            wood_veneer_overlay(out, [-l + 5.0, -t + 4.0, lw - 10.0, lh - 8.0], 3.0, painted);
            let _ = write!(
                out,
                r##"<line x1="0" y1="{:.1}" x2="0" y2="{:.1}" stroke="#5b4d3e" stroke-width="1.3"/>"##,
                -t + 6.0,
                t - 6.0
            );
        }
        PropKind::WasherDryer => {
            let half = lw * 0.5 - 6.0;
            rect(out, [-l + 4.0, -t + 5.0, half, lh - 10.0], 3.0, "#f1f5f9", "");
            rect(out, [2.0, -t + 5.0, half, lh - 10.0], 3.0, "#e2e8f0", "");
            circle(out, -l * 0.5, 2.0, 9.5, "#334155", thin);
            circle(out, l * 0.5, 2.0, 9.5, "#475569", thin);
        }
        PropKind::SingleBed => {
            rect(out, [-l + 8.0, -t + 6.0, lw - 16.0, lh - 12.0], 4.0, wd, "");
            rect(out, [-l + 11.0, -t + 9.0, lw - 22.0, lh - 18.0], 3.0, "#93c5fd", thin);
            rect(out, [-l + 13.0, -t + 11.0, 16.0, lh - 22.0], 2.5, "#ffffff", thin);
        }
        PropKind::FilingCabinet => {
            rect(out, [-l + 6.0, -t + 6.0, lw - 12.0, lh - 12.0], 2.5, "#64748b", "");
            rect(out, [-l + 9.0, -t + 9.0, lw - 18.0, 8.0], 1.0, "#94a3b8", thin);
            rect(out, [-6.0, t - 12.0, 12.0, 3.0], 1.0, "#cbd5e1", r#"stroke="none""#);
        }
        PropKind::WaterCooler => {
            rect(out, [-12.0, -12.0, 24.0, 24.0], 4.0, "#e2e8f0", "");
            circle(out, 0.0, 0.0, 9.0, "#38bdf8", thin);
            circle(out, -2.5, -2.5, 3.0, "#e0f2fe", r#"stroke="none""#);
        }
        PropKind::BrownstoneStoop => {
            let brick = if painted { "url(#bm-herringbone-brick)" } else { "#9c5a49" };
            rect(out, [-l + 4.0, -t + 2.0, lw - 8.0, lh - 4.0], 3.0, brick, "");
            for i in 1..4 {
                let y = -t + (lh / 4.0) * i as f32;
                let _ = write!(
                    out,
                    r##"<line x1="{:.1}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="#57534e" stroke-width="1.6"/>"##,
                    -l + 6.0,
                    l - 6.0
                );
            }
            let _ = write!(
                out,
                r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#1c1917" stroke-width="3"/><line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#1c1917" stroke-width="3"/>"##,
                -l + 6.0,
                -t + 4.0,
                -l + 6.0,
                t - 4.0,
                l - 6.0,
                -t + 4.0,
                l - 6.0,
                t - 4.0
            );
        }
        PropKind::AwningCanopy => {
            // Ground footprint subtle shadow curb strip; striped roof canvas renders in canopy pass.
            rect(
                out,
                [-l + 4.0, -t + 6.0, lw - 8.0, lh - 12.0],
                3.0,
                "#1e293b",
                r#"fill-opacity=".14" stroke="none""#,
            );
        }
        _ => return false,
    }
    true
}

/// Renders an overhanging striped storefront awning canopy in the upper canopy pass.
pub(crate) fn push_awning_canopy_glyph_with_mode(
    out: &mut String,
    p: &MapPropPlacement,
    cell_px: f32,
    _tex_mode: BattlemapTextureMode,
) {
    if p.kind != PropKind::AwningCanopy {
        return;
    }
    let (cx_m, cy_m) = p.center_m();
    let (sw_m, sh_m) = p.local_span_m();
    let (cx, cy) = (cx_m * cell_px, cy_m * cell_px);
    let (lw, lh) = (sw_m * cell_px, sh_m * cell_px);
    let (l, t) = (lw * 0.5, lh * 0.5);
    let base_hex = p
        .tint_rgb
        .map(format_rgb_hex)
        .unwrap_or_else(|| "#c53030".to_string());
    let _ = write!(
        out,
        r##"<g transform="translate({cx:.1} {cy:.1}) rotate({})" stroke="{BATTLEMAP_INK}" stroke-width="1.6" stroke-linejoin="round" opacity=".92">"##,
        p.rotation_deg
    );
    rect(out, [-l + 2.0, -t + 3.0, lw - 4.0, lh - 6.0], 4.0, &base_hex, "");
    let stripes = ((lw - 8.0) / 14.0).round().max(4.0) as i32;
    let step = (lw - 8.0) / stripes as f32;
    for i in (0..stripes).step_by(2) {
        let sx = -l + 4.0 + i as f32 * step;
        rect(
            out,
            [sx, -t + 5.0, (step * 0.72).max(2.0), lh - 10.0],
            1.0,
            "#fff7ed",
            r#"fill-opacity=".62" stroke="none""#,
        );
    }
    rect(
        out,
        [-l + 2.0, t - 10.0, lw - 4.0, 7.0],
        1.5,
        &base_hex,
        r#"fill-opacity=".85""#,
    );
    out.push_str("</g>");
}
