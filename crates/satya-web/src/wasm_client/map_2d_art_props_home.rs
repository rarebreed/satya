//! # Home Decor, Outdoor Structures & Street/Room Clutter 2D SVG Glyphs (`map_2d_art_props_home`)
//!
//! Companion to [`crate::wasm_client::map_2d_art_furniture`] implementing top-down SVG glyphs
//! for lamps, bedroom suites, entertainment units, bathroom sinks, rugs, fountains, gazebos
//! (ground timber deck + overhanging shingled cupola canopy), spas, HVAC units, sheds, grills,
//! refuse bins/piles, and floor/street clutter.

use crate::wasm_client::map_2d_art_furniture::{
    FURNITURE_WOODS, metallic_coat_overlay, porcelain_overlay, rug_weave_overlay,
    wood_veneer_overlay,
};
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
        "url(#bm-spa-water)"
    } else {
        "#44b3cf"
    };
    let rim_fill = if painted {
        "url(#bm-travertine)"
    } else {
        "#cbc6bc"
    };
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
            let coords = [-l + 2.0, -t + 3.0, lw - 4.0, depth];
            rect(out, coords, 3.0, "#eef1f4", "");
            porcelain_overlay(out, coords, 3.0, painted);
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
            rug_weave_overlay(out, coords, 2.0, painted);
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
                    "url(#bm-rug-weave)",
                    r#"fill-opacity=".42" style="mix-blend-mode:multiply" stroke="none""#,
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
            let hood = [-l + 8.0, -t + 12.0, lw - 16.0, lh - 26.0];
            rect(out, hood, 4.0, "#24272b", "");
            metallic_coat_overlay(out, hood, 4.0, painted);
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
            if painted {
                rect(
                    out,
                    coords,
                    3.0,
                    "url(#bm-shingle)",
                    r#"fill-opacity=".46" style="mix-blend-mode:multiply" stroke="none""#,
                );
            }
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
            let water = if painted {
                "url(#bm-spa-water)"
            } else {
                "#44bfd8"
            };
            match v & 3 {
                0 => {
                    // CedarDeckTub: warm cedar surround, front timber step & 4 corner hydro-jets.
                    let rim = [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0];
                    rect(out, rim, 12.0, "#7d4b2d", "");
                    wood_veneer_overlay(out, rim, 12.0, painted);
                    rect(out, [-l * 0.55, t - 6.0, lw * 0.55, 6.0], 2.0, "#8b5a36", thin);
                    rect(
                        out,
                        [-l + 13.0, -t + 13.0, lw - 26.0, lh - 26.0],
                        14.0,
                        water,
                        r##"stroke="#e5eef2" stroke-width="3.6""##,
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
                            5.0,
                            "#e0f7fc",
                            r#"stroke="none" fill-opacity=".75""#,
                        );
                    }
                }
                1 => {
                    // OctagonalTravertineSpa: 8-sided travertine coping, inner octagon water & radial jets.
                    let ro = l.min(t) * 0.92;
                    let ri = ro * 0.78;
                    let mut outer_pts = String::new();
                    let mut inner_pts = String::new();
                    for i in 0..8 {
                        let a = (i as f32 / 8.0) * TAU + TAU / 16.0;
                        let (s, c) = a.sin_cos();
                        let _ = write!(outer_pts, "{:.1},{:.1} ", c * ro, s * ro);
                        let _ = write!(inner_pts, "{:.1},{:.1} ", c * ri, s * ri);
                    }
                    let coping = if painted {
                        "url(#bm-travertine)"
                    } else {
                        "#ddd5c7"
                    };
                    let _ = write!(
                        out,
                        r##"<polygon points="{}" fill="{coping}" stroke-width="2.0"/><polygon points="{}" fill="{water}" stroke="#e5eef2" stroke-width="2.6"/>"##,
                        outer_pts.trim(),
                        inner_pts.trim()
                    );
                    for i in 0..8 {
                        let a = (i as f32 / 8.0) * TAU + TAU / 16.0;
                        let rj = ri * 0.72;
                        circle(
                            out,
                            a.cos() * rj,
                            a.sin() * rj,
                            3.4,
                            "#e0f7fc",
                            r#"stroke="none" fill-opacity=".72""#,
                        );
                    }
                }
                2 => {
                    // SunkenMosaicGrotto: organic stone rim, corner accent boulders & foam swirl arc.
                    let ro = l.min(t) * 0.93;
                    let rim_d = battlemap_blob_path(0.0, 0.0, ro, 10, 0.14, u64::from(v) ^ 0x3A);
                    let coping = if painted {
                        "url(#bm-travertine)"
                    } else {
                        "#cfc8bc"
                    };
                    let rock = if painted { "url(#bm-rock)" } else { "#8d867c" };
                    let _ = write!(out, r#"<path d="{rim_d}" fill="{coping}" stroke-width="2.0"/>"#);
                    for (idx, (bx, by, br)) in [
                        (-l * 0.64, -t * 0.58, 9.5),
                        (l * 0.62, -t * 0.52, 8.5),
                        (-l * 0.54, t * 0.58, 8.0),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let bd = battlemap_blob_path(
                            bx,
                            by,
                            br,
                            7,
                            0.22,
                            u64::from(v) ^ (idx as u64 * 19 + 7),
                        );
                        let _ = write!(out, r#"<path d="{bd}" fill="{rock}" stroke-width="1.3"/>"#);
                    }
                    let rx = (l - 17.0).max(12.0);
                    let ry = (t - 17.0).max(12.0);
                    let _ = write!(
                        out,
                        r##"<ellipse cx="0" cy="0" rx="{rx:.1}" ry="{ry:.1}" fill="{water}" stroke="#e5eef2" stroke-width="3.0"/><path d="M{:.1} 0A{:.1} {:.1} 0 1 1 0 {:.1}" fill="none" stroke="#e0f7fc" stroke-width="2.2" stroke-opacity=".68" stroke-linecap="round"/>"##,
                        -rx * 0.55,
                        rx * 0.55,
                        ry * 0.55,
                        ry * 0.55,
                    );
                }
                _ => {
                    // ModernArchitecturalSpa: charcoal chassis, porcelain bezel & infinity spillway.
                    let outer = [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0];
                    rect(out, outer, 6.0, "#2b3138", "");
                    metallic_coat_overlay(out, outer, 6.0, painted);
                    let porcelain = if painted {
                        "url(#bm-porcelain)"
                    } else {
                        "#eef2f5"
                    };
                    rect(
                        out,
                        [-l + 9.0, -t + 9.0, lw - 18.0, lh - 18.0],
                        4.0,
                        porcelain,
                        thin,
                    );
                    rect(
                        out,
                        [-l + 14.0, -t + 14.0, lw - 28.0, lh - 28.0],
                        3.0,
                        water,
                        r##"stroke="#277996" stroke-width="1.6""##,
                    );
                    rect(
                        out,
                        [-l * 0.45, -t + 16.0, lw * 0.45, 4.0],
                        1.5,
                        "#cbd5e1",
                        r#"stroke="none" fill-opacity=".82""#,
                    );
                }
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
        "url(#bm-shingle)"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::map_2d_art_furniture::push_furniture_glyph_with_mode;
    use satya_engine::map_dressing::make_prop;

    #[test]
    fn hot_tub_and_new_object_textures_render_valid_balanced_svg_in_painted_and_vector_modes() {
        let mut painted_outputs = Vec::new();
        for variant_seed in 0..4_u32 {
            let spa = make_prop(PropKind::HotTub, 2, 2, 0, None, variant_seed);
            let mut painted = String::new();
            push_furniture_glyph_with_mode(&mut painted, &spa, 64.0, BattlemapTextureMode::Painted);
            assert_eq!(painted.matches("<g ").count(), painted.matches("</g>").count());
            assert!(
                painted.contains("url(#bm-spa-water)"),
                "HotTub style {variant_seed} missing url(#bm-spa-water): {painted}"
            );
            painted_outputs.push(painted);

            let mut vector = String::new();
            push_furniture_glyph_with_mode(
                &mut vector,
                &spa,
                64.0,
                BattlemapTextureMode::ProceduralVector,
            );
            assert_eq!(vector.matches("<g ").count(), vector.matches("</g>").count());
            assert!(
                !vector.contains("url(#bm-"),
                "HotTub style {variant_seed} leaked texture ref in Vector mode: {vector}"
            );
        }
        assert!(painted_outputs[0].contains("url(#bm-wood-veneer)"));
        assert!(painted_outputs[1].contains("url(#bm-travertine)"));
        assert!(painted_outputs[2].contains("url(#bm-rock)"));
        assert!(painted_outputs[3].contains("url(#bm-porcelain)"));

        let sofa_fabric = make_prop(PropKind::Sofa, 1, 1, 0, None, 1);
        let sofa_leather = make_prop(PropKind::Sofa, 1, 1, 0, None, 0);
        let rug = make_prop(PropKind::Rug, 3, 3, 0, None, 7);
        let car = make_prop(PropKind::Car, 6, 4, 0, None, 3);
        let tree = make_prop(PropKind::LargeTree, 8, 8, 0, None, 5);
        let gazebo = make_prop(PropKind::Gazebo, 10, 10, 0, None, 2);

        let mut svg = String::new();
        push_furniture_glyph_with_mode(&mut svg, &sofa_fabric, 64.0, BattlemapTextureMode::Painted);
        assert!(svg.contains("url(#bm-fabric)"));

        svg.clear();
        push_furniture_glyph_with_mode(&mut svg, &sofa_leather, 64.0, BattlemapTextureMode::Painted);
        assert!(svg.contains("url(#bm-leather)"));

        svg.clear();
        push_furniture_glyph_with_mode(&mut svg, &rug, 64.0, BattlemapTextureMode::Painted);
        assert!(svg.contains("url(#bm-rug-weave)"));

        svg.clear();
        push_furniture_glyph_with_mode(&mut svg, &car, 64.0, BattlemapTextureMode::Painted);
        assert!(svg.contains("url(#bm-car-coat)"));

        svg.clear();
        push_furniture_glyph_with_mode(&mut svg, &tree, 64.0, BattlemapTextureMode::Painted);
        assert!(svg.contains("url(#bm-bark)"));

        svg.clear();
        push_gazebo_canopy_glyph_with_mode(
            &mut svg,
            &gazebo,
            64.0,
            BattlemapTextureMode::Painted,
        );
        assert!(svg.contains("url(#bm-shingle)"));
        assert!(svg.contains(r#"opacity=".50""#));
    }
}
