//! # 2D Battlemap Props for Expanded Asset Packs (`map_2d_art_props_packs`)
//!
//! Renders top-down architectural SVG fallback glyphs for the 26 Cafe, Retail, Industrial,
//! Farm, Conservatory, Sci-Fi, Office & Streetscape [`PropKind`]s (Batches 8–10).

use std::fmt::Write as _;

use satya_schema::PropKind;

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

/// Renders top-down 2D art for a Batches 8–10 prop inside the caller's rotated `<g>`.
///
/// Returns `false` if `kind` belongs to another prop glyph module.
pub(crate) fn push_pack_furniture_glyph(
    out: &mut String,
    kind: PropKind,
    lw: f32,
    lh: f32,
    _painted: bool,
    v: u32,
) -> bool {
    let (l, t) = (lw * 0.5, lh * 0.5);
    let dark = r##"stroke="#0f172a" stroke-width="1.1""##;
    match kind {
        PropKind::EspressoCounter => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 4.0, "#7c4d2d", dark);
            rect(out, [-l + 5.0, -t + 5.0, lw * 0.42, lh - 10.0], 2.0, "#cbd5e1", dark);
            rect(out, [2.0, -t + 5.0, lw * 0.42, lh - 10.0], 2.0, "#bae6fd", dark);
            circle(out, -l + 12.0, 0.0, 4.0, "#1e293b", "");
            circle(out, -l + 22.0, 0.0, 4.0, "#1e293b", "");
        }
        PropKind::CommercialPrepTable => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, "#cbd5e1", dark);
            rect(out, [-l + 6.0, -t + 6.0, lw * 0.35, lh - 12.0], 2.0, "#e2c799", dark);
            rect(out, [4.0, -t + 6.0, lw * 0.28, lh - 12.0], 2.0, "#94a3b8", dark);
        }
        PropKind::CommercialGrillHood => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, "#94a3b8", dark);
            rect(out, [-l + 6.0, -t + 5.0, lw * 0.45, lh - 10.0], 2.0, "#334155", dark);
            rect(out, [4.0, -t + 5.0, lw * 0.36, lh - 10.0], 2.0, "#f59e0b", r##"fill-opacity=".45" stroke="#1e293b" stroke-width="1""##);
        }
        PropKind::PatioUmbrellaTable => {
            let cap = if v.is_multiple_of(3) { "#c2410c" } else if v % 3 == 1 { "#0f766e" } else { "#1d4ed8" };
            for (cx, cy) in [(-l * 0.48, 0.0), (l * 0.48, 0.0), (0.0, -t * 0.48), (0.0, t * 0.48)] {
                circle(out, cx, cy, (lw.min(lh) * 0.14).max(5.0), "#78350f", dark);
            }
            let r = (lw.min(lh) * 0.36).max(10.0);
            circle(out, 0.0, 0.0, r, cap, dark);
            for i in 0..4 {
                let a = (i as f32) * std::f32::consts::FRAC_PI_4;
                line(out, -a.cos() * r, -a.sin() * r, a.cos() * r, a.sin() * r, r##"stroke="#fef3c7" stroke-width="1.1""##);
            }
            circle(out, 0.0, 0.0, 3.0, "#fef3c7", "");
        }
        PropKind::BuffetCounter => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, "#85583a", dark);
            for i in 0..4 {
                let bx = -l + 6.0 + (i as f32) * ((lw - 14.0) * 0.25);
                rect(out, [bx, -t + 6.0, ((lw - 20.0) * 0.22).max(5.0), lh - 12.0], 2.0, "#e2e8f0", dark);
            }
        }
        PropKind::RefrigeratedDisplayCase => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, "#334155", dark);
            rect(out, [-l + 5.0, -t + 5.0, lw - 10.0, lh - 10.0], 2.0, "#bae6fd", dark);
            line(out, 0.0, -t + 5.0, 0.0, t - 5.0, r##"stroke="#0f172a" stroke-width="1.4""##);
        }
        PropKind::ClothingRack => {
            rect(out, [-l + 3.0, -t + 4.0, lw - 6.0, lh - 8.0], 3.0, "#475569", dark);
            let cols = ["#1e3a8a", "#065f46", "#b45309", "#374151", "#78350f", "#0e7490"];
            let step = ((lw - 10.0) / 6.0).max(4.0);
            for (i, &c) in cols.iter().enumerate() {
                rect(out, [-l + 5.0 + (i as f32) * step, -t + 6.0, step - 1.0, lh - 12.0], 1.5, c, "");
            }
            line(out, -l + 3.0, 0.0, l - 3.0, 0.0, r##"stroke="#e2e8f0" stroke-width="1.6""##);
        }
        PropKind::PharmacyCounter => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, "#e2e8f0", dark);
            rect(out, [-l + 5.0, -t + 5.0, lw * 0.55, 6.0], 1.5, "#0d9488", "");
            rect(out, [4.0, -t + 5.0, (lw * 0.30).max(6.0), lh - 10.0], 1.5, "#38bdf8", dark);
        }
        PropKind::VendingMachine => {
            let body = if v.is_multiple_of(2) { "#1d4ed8" } else { "#b91c1c" };
            rect(out, [-l + 3.0, -t + 3.0, lw - 6.0, lh - 6.0], 3.0, body, dark);
            rect(out, [-l + 6.0, t - 9.0, lw - 12.0, 5.0], 1.5, "#bae6fd", "");
        }
        PropKind::AtmKiosk => {
            rect(out, [-l + 3.0, -t + 3.0, lw - 6.0, lh - 6.0], 3.0, "#1e293b", dark);
            rect(out, [-l + 6.0, t - 10.0, lw - 12.0, 6.0], 1.5, "#38bdf8", "");
            rect(out, [-l + 7.0, -t + 6.0, lw - 14.0, 6.0], 1.5, "#64748b", "");
        }
        PropKind::RoboticAssemblyArm => {
            let base = (lw.min(lh) * 0.34).max(8.0);
            rect(out, [-base, -base, base * 2.0, base * 2.0], 4.0, "#334155", dark);
            circle(out, 0.0, 0.0, base * 0.65, "#f59e0b", dark);
            line(out, 0.0, 0.0, l * 0.65, -t * 0.45, r##"stroke="#ea580c" stroke-width="5""##);
            circle(out, l * 0.65, -t * 0.45, 4.5, "#94a3b8", dark);
        }
        PropKind::ToolChestWorkbench => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, "#b91c1c", dark);
            rect(out, [-l + 5.0, -t + 5.0, lw - 10.0, lh * 0.45], 2.0, "#cbd5e1", dark);
            line(out, -l + 6.0, 3.0, l - 6.0, 3.0, r##"stroke="#fef2f2" stroke-width="1.3""##);
        }
        PropKind::CargoPallet => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 2.0, "#a16207", dark);
            rect(out, [-l + 5.0, -t + 5.0, lw - 10.0, lh - 10.0], 2.0, "#d97706", dark);
            line(out, -l + 5.0, -t + 5.0, l - 5.0, t - 5.0, r##"stroke="#78350f" stroke-width="1.2""##);
            line(out, l - 5.0, -t + 5.0, -l + 5.0, t - 5.0, r##"stroke="#78350f" stroke-width="1.2""##);
        }
        PropKind::ElectricalSwitchgear => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 2.0, "#475569", dark);
            for i in 0..3 {
                let bx = -l + 5.0 + (i as f32) * ((lw - 10.0) / 3.0);
                rect(out, [bx, -t + 5.0, ((lw - 16.0) / 3.0).max(5.0), lh - 10.0], 1.5, "#64748b", dark);
                circle(out, bx + 4.0, 0.0, 2.0, "#facc15", "");
            }
        }
        PropKind::Tractor => {
            rect(out, [-l + 5.0, -t + 4.0, lw - 10.0, lh - 8.0], 4.0, "#15803d", dark);
            rect(out, [-l + 7.0, -t + 7.0, lw - 14.0, lh * 0.42], 3.0, "#38bdf8", dark);
            rect(out, [-l + 1.0, -t + 5.0, 4.0, lh * 0.36], 1.5, "#111827", "");
            rect(out, [l - 5.0, -t + 5.0, 4.0, lh * 0.36], 1.5, "#111827", "");
        }
        PropKind::HayBaleStack => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, "#ca8a04", dark);
            line(out, -l * 0.35, -t + 3.0, -l * 0.35, t - 3.0, r##"stroke="#713f12" stroke-width="1.5""##);
            line(out, l * 0.35, -t + 3.0, l * 0.35, t - 3.0, r##"stroke="#713f12" stroke-width="1.5""##);
        }
        PropKind::LivestockTrough => {
            rect(out, [-l + 2.0, -t + 3.0, lw - 4.0, lh - 6.0], 3.0, "#94a3b8", dark);
            rect(out, [-l + 5.0, -t + 6.0, lw - 10.0, lh - 12.0], 2.0, "#0284c7", "");
        }
        PropKind::ConservatorySpecimenIsland => {
            let r = (lw.min(lh) * 0.45).max(10.0);
            circle(out, 0.0, 0.0, r, "#78350f", dark);
            circle(out, 0.0, 0.0, r * 0.84, "#15803d", dark);
            for &(ox, oy, rad, col) in &[
                (-r * 0.35, -r * 0.25, r * 0.32, "#22c55e"),
                (r * 0.32, r * 0.20, r * 0.28, "#16a34a"),
                (0.0, 0.0, r * 0.25, "#4ade80"),
            ] {
                circle(out, ox, oy, rad, col, "");
            }
        }
        PropKind::CommandConsole => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, "#1e293b", dark);
            for i in 0..3 {
                let mx = -l + 6.0 + (i as f32) * ((lw - 14.0) / 3.0);
                rect(out, [mx, -t + 5.0, ((lw - 18.0) / 3.0).max(5.0), 6.0], 1.5, "#38bdf8", "");
            }
        }
        PropKind::EvaSuitLocker => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 3.0, "#94a3b8", dark);
            circle(out, -l * 0.45, 0.0, 5.0, "#f59e0b", dark);
            circle(out, l * 0.45, 0.0, 5.0, "#f59e0b", dark);
        }
        PropKind::SciFiCargoPod => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 6.0, "#475569", dark);
            rect(out, [-l + 7.0, -t + 6.0, lw - 14.0, lh - 12.0], 3.0, "#64748b", dark);
            line(out, -l + 8.0, 0.0, l - 8.0, 0.0, r##"stroke="#38bdf8" stroke-width="1.6""##);
        }
        PropKind::HullDebris => {
            rect(out, [-l + 3.0, -t + 4.0, lw * 0.62, lh * 0.58], 2.0, "#475569", dark);
            rect(out, [-4.0, -2.0, lw * 0.46, lh * 0.48], 2.0, "#64748b", dark);
            line(out, -l + 6.0, -t + 8.0, l - 5.0, t - 6.0, r##"stroke="#b45309" stroke-width="1.4""##);
        }
        PropKind::ConferenceTable => {
            rect(out, [-l + 5.0, -t + 5.0, lw - 10.0, lh - 10.0], 7.0, "#7c4d2d", dark);
            for i in 0..3 {
                let cx = -l + 10.0 + (i as f32) * ((lw - 20.0) * 0.42);
                circle(out, cx, -t + 3.5, 4.0, "#334155", "");
                circle(out, cx, t - 3.5, 4.0, "#334155", "");
            }
        }
        PropKind::CopyMachine => {
            rect(out, [-l + 3.0, -t + 3.0, lw - 6.0, lh - 6.0], 3.0, "#e2e8f0", dark);
            rect(out, [-l + 6.0, -t + 6.0, lw - 14.0, lh * 0.45], 2.0, "#94a3b8", dark);
        }
        PropKind::StreetLamp => {
            circle(out, 0.0, 0.0, (lw.min(lh) * 0.24).max(5.0), "#fef08a", r##"fill-opacity=".35""##);
            circle(out, 0.0, 0.0, 4.0, "#334155", dark);
            circle(out, 2.0, 0.0, 2.5, "#fef9c3", "");
        }
        PropKind::BusShelter => {
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0], 4.0, "#38bdf8", r##"fill-opacity=".42" stroke="#1e293b" stroke-width="1.3""##);
            rect(out, [-l + 6.0, 0.0, lw - 12.0, lh * 0.35], 2.0, "#64748b", dark);
        }
        _ => return false,
    }
    true
}
