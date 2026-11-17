//! # Home Decor, Outdoor Structures & Street/Room Clutter 2D SVG Glyphs (`map_2d_art_props_home`)
//!
//! Companion to [`crate::wasm_client::map_2d_art_furniture`] implementing top-down SVG glyphs
//! for lamps, bedroom suites, entertainment units, bathroom sinks, rugs, fountains, gazebos
//! (ground timber deck + overhanging shingled cupola canopy), spas, HVAC units, sheds, grills,
//! refuse bins/piles, and floor/street clutter.

use crate::wasm_client::map_2d_art_furniture::{FURNITURE_WOODS, wood_veneer_overlay};
use crate::wasm_client::map_2d_art_props::{BATTLEMAP_INK, battlemap_blob_path, battlemap_mix_u64};
use crate::wasm_client::map_2d_art_textures::BattlemapTextureMode;
use satya_schema::{MapPropPlacement, PropKind};
use std::f32::consts::TAU;
use std::fmt::Write as _;

const RUG_PALETTE: &[&str] = &["#8f3e34", "#3d5a7a", "#5f6e52", "#7a5e44", "#6e5370"];
const CARPET_TINTS: &[&str] = &["#b58d58", "#7c9473", "#8899ad", "#b47363"];

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

fn push_fountain_glyph(out: &mut String, l: f32, t: f32, tiers: u8, painted: bool) {
    let r = l.min(t) * 0.92;
    let basin_fill = if painted {
        "url(#bm-pool-water)"
    } else {
        "#44b3cf"
    };
    let rim_fill = if painted { "url(#bm-rock)" } else { "#cbc6bc" };
    circle(out, 0.0, 0.0, r, rim_fill, "");
    circle(
        out,
        0.0,
        0.0,
        r * 0.84,
        basin_fill,
        r##"stroke="#277996" stroke-width="1.5""##,
    );
    circle(
        out,
        0.0,
        0.0,
        r * 0.62,
        "none",
        r##"stroke="#b8f2ff" stroke-width="1.2" stroke-dasharray="10 6" stroke-opacity=".65""##,
    );
    if tiers >= 2 {
        circle(out, 0.0, 0.0, r * 0.42, rim_fill, r#"stroke-width="1.5""#);
        circle(
            out,
            0.0,
            0.0,
            r * 0.34,
            basin_fill,
            r##"stroke="#277996" stroke-width="1""##,
        );
    }
    if tiers >= 3 {
        circle(out, 0.0, 0.0, r * 0.21, rim_fill, r#"stroke-width="1.3""#);
    }
    circle(
        out,
        0.0,
        0.0,
        (r * 0.11).max(4.5),
        "#e6e1d7",
        r#"stroke-width="1.2""#,
    );
    circle(
        out,
        0.0,
        0.0,
        (r * 0.06).max(2.5),
        "#e0f9ff",
        r#"stroke="none" fill-opacity=".85""#,
    );
}

/// Renders specialized residential decor, outdoor amenities, and scatter clutter inside an already
/// opened `<g transform="translate(...) rotate(...)" ...>` group. Returns `false` if unhandled.
pub(crate) fn push_home_furniture_glyph(
    out: &mut String,
    kind: PropKind,
    lw: f32,
    lh: f32,
    painted: bool,
    v: u32,
) -> bool {
    let (l, t) = (lw * 0.5, lh * 0.5);
    let thin = r#"stroke-width="1.1" stroke-opacity=".6""#;
    let wd = pick(v, FURNITURE_WOODS);
    match kind {
        PropKind::DeskLamp => {
            circle(
                out,
                0.0,
                -4.0,
                12.0,
                "#fde68a",
                r#"fill-opacity=".28" stroke="none""#,
            );
            circle(out, 0.0, 4.0, 6.5, "#3f4347", r#"stroke-width="1.4""#);
            let _ = write!(
                out,
                r##"<path d="M0 4L-3 -6" stroke="#b59449" stroke-width="2.4" stroke-linecap="round"/>"##
            );
            let _ = write!(
                out,
                r##"<path d="M-11 -4L3 -13L8 -4L-3 1Z" fill="#d97745" stroke-width="1.3"/>"##
            );
            circle(
                out,
                -2.0,
                -7.0,
                3.0,
                "#fef3c7",
                r#"stroke="none" fill-opacity=".9""#,
            );
        }
        PropKind::StandingLamp => {
            circle(
                out,
                0.0,
                0.0,
                19.0,
                "#fef08a",
                r#"fill-opacity=".24" stroke="none""#,
            );
            circle(out, 0.0, 0.0, 10.5, "#f5e6c4", r#"stroke-width="1.6""#);
            circle(out, 0.0, 0.0, 4.2, "#fbbf24", r#"stroke-width="1""#);
            circle(out, 0.0, 0.0, 1.8, "#5a4632", r#"stroke="none""#);
        }
        PropKind::Nightstand => {
            let inset = l.min(t) * 0.18;
            let coords = [-l + inset, -t + 4.0, lw - inset * 2.0, lh * 0.72];
            rect(out, coords, 3.0, wd, "");
            wood_veneer_overlay(out, coords, 3.0, painted);
            rect(
                out,
                [-6.0, -t + 10.0, 12.0, 16.0],
                1.5,
                "#3b5978",
                r#"stroke-width=".9""#,
            );
            circle(
                out,
                l - inset - 11.0,
                -t + 16.0,
                6.5,
                "#f6e7bd",
                r#"stroke-width="1""#,
            );
            let _ = write!(
                out,
                r##"<line x1="-6" y1="{:.1}" x2="6" y2="{:.1}" stroke="#d4af37" stroke-width="2"/>"##,
                -t + lh * 0.72 + 2.0,
                -t + lh * 0.72 + 2.0
            );
        }
        PropKind::Dresser => {
            let depth = lh * 0.60;
            let coords = [-l + 4.0, -t + 3.0, lw - 8.0, depth];
            rect(out, coords, 3.0, wd, "");
            wood_veneer_overlay(out, coords, 3.0, painted);
            rect(
                out,
                [-l + 14.0, -t + 6.0, lw - 28.0, 5.0],
                1.5,
                "#dbeafe",
                r#"stroke-width="1""#,
            );
            circle(
                out,
                -l + 22.0,
                -t + depth * 0.55,
                6.0,
                "#e5ddd0",
                r#"stroke-width=".9""#,
            );
            let _ = write!(
                out,
                r##"<path d="M{:.1} {:.1}H{:.1}" stroke="#d4af37" stroke-width="2.2"/><path d="M{:.1} {:.1}H{:.1}" stroke="#d4af37" stroke-width="2.2"/>"##,
                -l * 0.45 - 6.0,
                -t + depth + 2.0,
                -l * 0.45 + 6.0,
                l * 0.45 - 6.0,
                -t + depth + 2.0,
                l * 0.45 + 6.0
            );
        }
        PropKind::BathroomSink => {
            let depth = lh * 0.64;
            rect(
                out,
                [-l + 2.0, -t + 3.0, lw - 4.0, depth],
                3.0,
                "#eef1f4",
                "",
            );
            let rx = (l - 12.0).max(10.0);
            let ry = (depth * 0.34).max(7.0);
            let cy = -t + 5.0 + depth * 0.52;
            let _ = write!(
                out,
                r##"<ellipse cx="0" cy="{cy:.1}" rx="{rx:.1}" ry="{ry:.1}" fill="#cfe5f0" stroke-width="1.3"/><circle cx="0" cy="{:.1}" r="2.6" fill="#64748b" stroke="none"/><circle cx="0" cy="{:.1}" r="2.2" fill="#94a3b8" stroke-width="1"/>"##,
                cy + 2.0,
                -t + 7.5
            );
        }
        PropKind::FireExtinguisher => {
            rect(
                out,
                [-7.0, -t + 6.0, 14.0, 4.0],
                1.0,
                "#64748b",
                r#"stroke-width="1""#,
            );
            circle(out, 0.0, 2.0, 10.5, "#cf2929", r#"stroke-width="1.6""#);
            circle(out, 0.0, 2.0, 4.0, "#1f2937", r#"stroke-width="1""#);
            let _ = write!(
                out,
                r##"<path d="M6 -3Q13 2 10 11" fill="none" stroke="#18181b" stroke-width="2.4" stroke-linecap="round"/>"##
            );
        }
        PropKind::TvStand => {
            let depth = lh * 0.56;
            let coords = [-l + 4.0, -t + 4.0, lw - 8.0, depth];
            rect(out, coords, 3.0, "#3a2e27", "");
            wood_veneer_overlay(out, coords, 3.0, painted);
            rect(
                out,
                [-l + 10.0, -t + 7.0, lw - 20.0, 8.0],
                2.0,
                "#111827",
                r#"stroke-width="1.4""#,
            );
            rect(
                out,
                [-l + 14.0, -t + 9.0, lw - 28.0, 3.0],
                1.0,
                "#38bdf8",
                r#"fill-opacity=".45" stroke="none""#,
            );
            rect(
                out,
                [-18.0, -t + depth - 10.0, 36.0, 5.0],
                1.5,
                "#27272a",
                r#"stroke-width=".9""#,
            );
        }
        PropKind::Stereo => {
            rect(
                out,
                [-l + 5.0, -t + 8.0, 14.0, lh - 18.0],
                2.0,
                "#24272b",
                "",
            );
            circle(out, -l + 12.0, 0.0, 4.5, "#52525b", r#"stroke-width=".9""#);
            rect(
                out,
                [-l + 22.0, -t + 10.0, lw - 44.0, lh - 22.0],
                2.0,
                "#374151",
                "",
            );
            rect(
                out,
                [-l + 26.0, -t + 14.0, lw - 52.0, 5.0],
                1.0,
                "#93c5fd",
                r#"stroke="none" fill-opacity=".85""#,
            );
            rect(
                out,
                [l - 19.0, -t + 8.0, 14.0, lh - 18.0],
                2.0,
                "#24272b",
                "",
            );
            circle(out, l - 12.0, 0.0, 4.5, "#52525b", r#"stroke-width=".9""#);
        }
        PropKind::RunnerRug => {
            let c = pick(v, RUG_PALETTE);
            let coords = [-l + 3.0, -t + 6.0, lw - 6.0, lh - 12.0];
            rect(out, coords, 2.0, c, r#"stroke-width="1.2" opacity=".88""#);
            if painted {
                rect(
                    out,
                    coords,
                    2.0,
                    "url(#bm-fabric)",
                    r#"fill-opacity=".38" style="mix-blend-mode:multiply" stroke="none""#,
                );
            }
            rect(
                out,
                [-l + 10.0, -t + 12.0, lw - 20.0, lh - 24.0],
                1.0,
                "none",
                r##"stroke="#efe3c8" stroke-width="1.8" opacity=".7""##,
            );
        }
        PropKind::RoundRug => {
            let r = l.min(t) * 0.90;
            let c = pick(v, RUG_PALETTE);
            circle(out, 0.0, 0.0, r, c, r#"stroke-width="1.2" opacity=".88""#);
            if painted {
                circle(
                    out,
                    0.0,
                    0.0,
                    r,
                    "url(#bm-fabric)",
                    r#"fill-opacity=".38" style="mix-blend-mode:multiply" stroke="none""#,
                );
            }
            circle(
                out,
                0.0,
                0.0,
                r * 0.72,
                "none",
                r##"stroke="#efe3c8" stroke-width="2" opacity=".72""##,
            );
            circle(
                out,
                0.0,
                0.0,
                r * 0.34,
                pick(v ^ 5, CARPET_TINTS),
                r#"stroke-width="1" fill-opacity=".65""#,
            );
        }
        PropKind::SmallFountain => push_fountain_glyph(out, l, t, 1, painted),
        PropKind::MediumFountain => push_fountain_glyph(out, l, t, 2, painted),
        PropKind::LargeFountain => push_fountain_glyph(out, l, t, 3, painted),
        PropKind::Gazebo => {
            let r = l.min(t) * 0.94;
            let mut pts = String::new();
            for i in 0..8 {
                let a = (i as f32 / 8.0) * TAU + TAU / 16.0;
                let _ = write!(pts, "{:.1},{:.1} ", a.cos() * r, a.sin() * r);
            }
            let deck = if painted {
                "url(#bm-wood-deck)"
            } else {
                "#9c6a47"
            };
            let _ = write!(
                out,
                r#"<polygon points="{}" fill="{deck}" stroke-width="2.2"/>"#,
                pts.trim()
            );
            for i in 0..8 {
                let a0 = (i as f32 / 8.0) * TAU + TAU / 16.0;
                let a1 = ((i + 1) as f32 / 8.0) * TAU + TAU / 16.0;
                let mid = (a0 + a1) * 0.5;
                if mid.sin() < 0.75 {
                    let _ = write!(
                        out,
                        r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#5b3e26" stroke-width="4.2"/>"##,
                        a0.cos() * r * 0.92,
                        a0.sin() * r * 0.92,
                        a1.cos() * r * 0.92,
                        a1.sin() * r * 0.92
                    );
                }
                circle(
                    out,
                    a0.cos() * r * 0.92,
                    a0.sin() * r * 0.92,
                    5.5,
                    "#4a301b",
                    r#"stroke-width="1.3""#,
                );
            }
        }
        PropKind::BbqGrill => {
            rect(
                out,
                [-l + 8.0, -t + 12.0, lw - 16.0, lh - 26.0],
                4.0,
                "#24272b",
                "",
            );
            rect(
                out,
                [-l + 13.0, -t + 16.0, lw - 26.0, lh - 34.0],
                2.0,
                "#4b5563",
                thin,
            );
            let _ = write!(
                out,
                r##"<path d="M-8 {:.1}H8M-8 {:.1}H8" stroke="#9ca3af" stroke-width="1.6"/>"##,
                -4.0, 4.0
            );
            rect(
                out,
                [-10.0, t - 15.0, 20.0, 4.0],
                1.5,
                "#dc2626",
                r#"stroke-width=".9""#,
            );
        }
        PropKind::GardenShed => {
            let coords = [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0];
            rect(out, coords, 3.0, "#7a583b", "");
            wood_veneer_overlay(out, coords, 3.0, painted);
            let _ = write!(
                out,
                r##"<path d="M{:.1} 0H{:.1}" stroke="#3e2715" stroke-width="3.6"/>"##,
                -l + 4.0,
                l - 4.0
            );
            for k in [-2, -1, 1, 2] {
                let py = (k as f32) * (t * 0.36);
                let _ = write!(
                    out,
                    r#"<line x1="{:.1}" y1="{py:.1}" x2="{:.1}" y2="{py:.1}" {thin}/>"#,
                    -l + 6.0,
                    l - 6.0
                );
            }
            rect(
                out,
                [-16.0, t - 7.0, 32.0, 5.0],
                1.0,
                "#c8a982",
                r#"stroke-width="1""#,
            );
        }
        PropKind::TrashCan => {
            circle(out, 0.0, 0.0, 15.5, "#3f464c", "");
            circle(out, 0.0, 0.0, 11.0, "#525a61", r#"stroke-width="1.2""#);
            rect(
                out,
                [-4.0, -2.0, 8.0, 4.0],
                1.5,
                "#1f2326",
                r#"stroke-width="1""#,
            );
            let _ = write!(
                out,
                r#"<path d="M-18 -3V3M18 -3V3" stroke="{BATTLEMAP_INK}" stroke-width="3" stroke-linecap="round"/>"#
            );
        }
        PropKind::GarbagePile => {
            rect(
                out,
                [-l + 10.0, -t + 10.0, 28.0, 18.0],
                2.0,
                "#9c7b56",
                r#"transform="rotate(-12)" stroke-width="1.3""#,
            );
            for (i, (bx, by, r, col)) in [
                (-18.0, -4.0, 15.0, "#272b30"),
                (6.0, 4.0, 16.0, "#353c3a"),
                (24.0, -6.0, 13.0, "#2d3238"),
            ]
            .into_iter()
            .enumerate()
            {
                let d = battlemap_blob_path(bx, by, r, 7, 0.25, u64::from(v) ^ (i as u64 * 31));
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="{col}" stroke-width="1.5"/><circle cx="{bx:.1}" cy="{by:.1}" r="2.5" fill="#eab308" stroke="none"/>"##
                );
            }
        }
        PropKind::AcUnit => {
            rect(
                out,
                [-l + 7.0, -t + 8.0, lw - 14.0, lh - 16.0],
                3.0,
                if painted { "url(#bm-steel)" } else { "#a3aab0" },
                "",
            );
            circle(out, 0.0, 0.0, 15.0, "#24282e", r#"stroke-width="1.4""#);
            let _ = write!(
                out,
                r##"<path d="M-11 0H11M0 -11V11" stroke="#64748b" stroke-width="2.4" stroke-linecap="round"/>"##
            );
            circle(out, 0.0, 0.0, 3.5, "#94a3b8", r#"stroke-width="1""#);
        }
        PropKind::HotTub => {
            let rim = [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0];
            rect(out, rim, 12.0, "#7d4b2d", "");
            wood_veneer_overlay(out, rim, 12.0, painted);
            rect(
                out,
                [-l + 13.0, -t + 13.0, lw - 26.0, lh - 26.0],
                14.0,
                if painted {
                    "url(#bm-pool-water)"
                } else {
                    "#44bfd8"
                },
                r##"stroke="#e5eef2" stroke-width="4""##,
            );
            for (hx, hy) in [
                (-l + 22.0, -t + 20.0),
                (l - 22.0, -t + 20.0),
                (-l + 22.0, t - 20.0),
                (l - 22.0, t - 20.0),
            ] {
                circle(
                    out,
                    hx,
                    hy,
                    5.5,
                    "#e0f7fc",
                    r#"stroke="none" fill-opacity=".75""#,
                );
            }
        }
        PropKind::FloorClothes => {
            for (i, (dx, dy, r, col)) in [
                (-9.0, -5.0, 11.0, "#4f6d8a"),
                (8.0, 5.0, 10.0, "#b8644f"),
                (1.0, -10.0, 8.0, "#d8cfb8"),
            ]
            .into_iter()
            .enumerate()
            {
                let d = battlemap_blob_path(dx, dy, r, 6, 0.28, u64::from(v) ^ (i as u64 * 17));
                let _ = write!(out, r#"<path d="{d}" fill="{col}" stroke-width="1.3"/>"#);
            }
        }
        PropKind::FloorToys => {
            rect(
                out,
                [-16.0, -10.0, 9.0, 9.0],
                1.5,
                "#e14c4c",
                r#"transform="rotate(20 -11 -5)" stroke-width="1.1""#,
            );
            rect(
                out,
                [4.0, -12.0, 12.0, 7.0],
                2.0,
                "#3b82f6",
                r#"transform="rotate(-25 10 -8)" stroke-width="1.1""#,
            );
            circle(out, -4.0, 10.0, 6.0, "#facc15", r#"stroke-width="1.1""#);
            rect(
                out,
                [10.0, 6.0, 8.0, 8.0],
                1.5,
                "#22c55e",
                r#"transform="rotate(14 14 10)" stroke-width="1.1""#,
            );
        }
        PropKind::StreetDebris => {
            for i in 0..6 {
                let h = battlemap_mix_u64(u64::from(v) ^ (i as u64 * 41));
                let px = -l * 0.72 + ((h % 100) as f32 / 100.0) * (lw * 0.72);
                let py = -t * 0.55 + (((h >> 10) % 100) as f32 / 100.0) * (lh * 0.55);
                let col = ["#d6d0c4", "#78716c", "#9a7b56", "#a8b3bd"][i % 4];
                rect(out, [px, py, 9.0, 6.0], 1.0, col, r#"stroke-width=".9""#);
            }
            circle(out, -14.0, 4.0, 3.0, "#b91c1c", r#"stroke-width=".9""#);
            circle(out, 18.0, -5.0, 2.8, "#94a3b8", r#"stroke-width=".9""#);
        }
        _ => return false,
    }
    true
}

/// Emits an overhanging semi-translucent octagonal shingled pergola/gazebo roof in Step 5 (`y`-sorted canopies).
pub(crate) fn push_gazebo_canopy_glyph_with_mode(
    out: &mut String,
    p: &MapPropPlacement,
    cell_px: f32,
    tex_mode: BattlemapTextureMode,
) {
    let fp = p.footprint_m();
    let (cx, cy) = (fp.cx_m * cell_px, fp.cy_m * cell_px);
    let r = (fp.span_w_m.min(fp.span_h_m) * cell_px) * 0.50;
    let mut pts = String::new();
    let mut hip_lines = String::new();
    for i in 0..8 {
        let a = (i as f32 / 8.0) * TAU + TAU / 16.0;
        let vx = a.cos() * r;
        let vy = a.sin() * r;
        let _ = write!(pts, "{vx:.1},{vy:.1} ");
        let _ = write!(hip_lines, "M0 0L{vx:.1} {vy:.1}");
    }
    let fill = if tex_mode.is_painted() {
        "url(#bm-roof-gravel)"
    } else {
        "#7b5e46"
    };
    let _ = write!(
        out,
        r##"<g transform="translate({cx:.1} {cy:.1}) rotate({})" opacity=".50" stroke="{BATTLEMAP_INK}" stroke-linejoin="round"><polygon points="{}" fill="{fill}" stroke-width="2.2"/><path d="{hip_lines}" fill="none" stroke-width="1.6" stroke-opacity=".75"/><circle cx="0" cy="0" r="{:.1}" fill="#9c6f46" stroke-width="1.6"/></g>"##,
        fp.rot_deg,
        pts.trim(),
        (r * 0.14).max(7.0),
    );
}
