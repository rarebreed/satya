//! # Multi-Cell Furniture Glyphs for the 2D Battlemap (`map_2d_art_furniture`)
//!
//! Draws one top-down glyph per [`MapPropPlacement`], scaled to its whole footprint, so a 3-seat sofa
//! spans 3 cells, a bar counter spans its full run, and a car spans `2 × 5` cells. Glyphs are authored in a
//! local frame centered on the footprint (`±lw/2 × ±lh/2`) with the **back facing `-y`**, then rotated by
//! `rotation_deg`. Single-cell legacy glyphs (armchair, planter, crate, drums) reuse
//! [`crate::wasm_client::map_2d_art_props::push_battlemap_prop`].

use crate::wasm_client::map_2d_art_props::{
    BATTLEMAP_INK, BattlemapPropKind, battlemap_blob_path, battlemap_mix_u64,
    push_battlemap_prop_with_mode,
};
use crate::wasm_client::map_2d_art_textures::BattlemapTextureMode;
use satya_schema::{MapPropPlacement, PropKind};
use std::f32::consts::TAU;
use std::fmt::Write as _;

const FABRICS: &[&str] = &["#7d8e9b", "#a0705a", "#6d7f68", "#8c7f9e", "#b49a6a", "#5f6b78"];
const WOODS: &[&str] = &["#a47b55", "#c4a27c", "#7a5c43", "#b8926a"];
const BOTTLES: &[&str] = &["#3f7a3a", "#8a5a2b", "#d9c38a", "#6b2f2f", "#2f5a7a", "#c9c9c0"];
const CANOPY_OUTER: &[&str] = &["#3f6b35", "#355e3b", "#4a6f35", "#4f6a2f", "#2f5838"];
const CANOPY_MID: &[&str] = &["#5d8c47", "#4f814e", "#6b9348", "#6d8b3d", "#48784c"];
const CANOPY_SUN: &[&str] = &["#84ad5c", "#74a468", "#93b560", "#95ad57", "#699a64"];

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

fn fabric_overlay(out: &mut String, [x, y, w, h]: [f32; 4], rx: f32) {
    rect(
        out,
        [x, y, w, h],
        rx,
        "url(#bm-fabric)",
        r#"fill-opacity=".38" style="mix-blend-mode:multiply" stroke="none""#,
    );
}

fn circle(out: &mut String, cx: f32, cy: f32, r: f32, fill: &str, extra: &str) {
    let _ = write!(out, r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{fill}" {extra}/>"#);
}

/// Appends the footprint-scaled glyph for `p` (`cell_px` SVG units per `1.0m` meter).
pub fn push_furniture_glyph(out: &mut String, p: &MapPropPlacement, cell_px: f32) {
    push_furniture_glyph_with_mode(out, p, cell_px, BattlemapTextureMode::Painted);
}

/// Mode-aware variant of [`push_furniture_glyph`] applying WebP material patterns in painted mode.
pub fn push_furniture_glyph_with_mode(
    out: &mut String,
    p: &MapPropPlacement,
    cell_px: f32,
    tex_mode: BattlemapTextureMode,
) {
    let painted = tex_mode.is_painted();
    let fp = p.footprint_m();
    let (cx, cy) = (fp.cx_m * cell_px, fp.cy_m * cell_px);
    let (lw, lh) = (fp.span_w_m * cell_px, fp.span_h_m * cell_px);
    let rot = fp.rot_deg;
    let legacy = match p.kind {
        PropKind::Armchair => Some(BattlemapPropKind::Armchair),
        PropKind::Planter => Some(BattlemapPropKind::Planter),
        PropKind::WoodCrate => Some(BattlemapPropKind::WoodCrate),
        PropKind::DrumCluster => Some(BattlemapPropKind::DrumCluster),
        _ => None,
    };
    if let Some(kind) = legacy {
        push_battlemap_prop_with_mode(out, kind, cx, cy, rot as u16, u64::from(p.variant_seed), tex_mode);
        return;
    }
    let (l, t) = (lw * 0.5, lh * 0.5);
    let v = p.variant_seed;
    let thin = r#"stroke-width="1.1" stroke-opacity=".55""#;
    let wd_for = |seed: u32| if painted { "url(#bm-oak)" } else { pick(seed, WOODS) };
    let _ = write!(
        out,
        r#"<g transform="translate({cx:.1} {cy:.1}) rotate({rot})" stroke="{BATTLEMAP_INK}" stroke-width="2" stroke-linejoin="round">"#
    );
    match p.kind {
        PropKind::Sofa | PropKind::Loveseat => {
            let f = pick(v, FABRICS);
            rect(out, [-l + 3.0, -t + 8.0, lw - 6.0, lh - 14.0], 8.0, f, "");
            rect(out, [-l + 3.0, -t + 8.0, lw - 6.0, 15.0], 6.0, f, "");
            rect(out, [-l + 5.0, -t + 10.0, lw - 10.0, 11.0], 5.0, "#000", r#"fill-opacity=".16" stroke="none""#);
            rect(out, [-l + 3.0, -t + 14.0, 12.0, lh - 20.0], 5.0, f, "");
            rect(out, [l - 15.0, -t + 14.0, 12.0, lh - 20.0], 5.0, f, "");
            if painted {
                fabric_overlay(out, [-l + 3.0, -t + 8.0, lw - 6.0, lh - 14.0], 8.0);
            }
            let seats = p.w_cells.max(p.h_cells).max(2);
            let span = lw - 30.0;
            for i in 1..seats {
                let x = -l + 15.0 + span * i as f32 / seats as f32;
                let _ = write!(out, r#"<path d="M{x:.1} {:.1}V{:.1}" {thin}/>"#, -t + 23.0, t - 8.0);
            }
        }
        PropKind::CoffeeTable => {
            rect(out, [-l + 10.0, -t + 14.0, lw - 20.0, lh - 28.0], 5.0, wd_for(v), "");
            rect(out, [-l + 22.0, -t + 22.0, 16.0, 11.0], 1.0, "#e9e3d6", r#"stroke-width="1""#);
            circle(out, l - 26.0, 0.0, 5.0, "#f4f1ea", r#"stroke-width="1.2""#);
        }
        PropKind::Rug => {
            let c = pick(v, &["#9b3d33", "#3d5a80", "#6b5b3e", "#4f6d5a"]);
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 2.0, c, r#"stroke-width="1.2" opacity=".85""#);
            if painted {
                fabric_overlay(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 2.0);
            }
            rect(out, [-l + 12.0, -t + 12.0, lw - 24.0, lh - 24.0], 1.0, "none", r##"stroke="#f1e6cf" stroke-width="2" opacity=".7""##);
            let _ = write!(
                out,
                r##"<path d="M0 {:.1}L{:.1} 0L0 {:.1}L{:.1} 0Z" fill="#f1e6cf" fill-opacity=".35" stroke="none"/>"##,
                -t + 20.0,
                l - 24.0,
                t - 20.0,
                -l + 24.0
            );
        }
        PropKind::DiningTable => {
            for (x, y) in [(-l * 0.45, -t + 6.0), (l * 0.45 - 16.0, -t + 6.0), (-l * 0.45, t - 20.0), (l * 0.45 - 16.0, t - 20.0)] {
                rect(out, [x, y, 16.0, 14.0], 4.0, "#5b4a3a", "");
            }
            for (x, y) in [(-l + 4.0, -8.0), (l - 18.0, -8.0)] {
                rect(out, [x, y, 14.0, 16.0], 4.0, "#5b4a3a", "");
            }
            rect(out, [-l + 20.0, -t + 18.0, lw - 40.0, lh - 36.0], 4.0, wd_for(v), "");
            circle(out, 0.0, 0.0, 7.0, "#f4f1ea", r#"stroke-width="1.2""#);
        }
        PropKind::RoundTable => {
            circle(out, -21.0, 0.0, 7.5, "#3d4247", "");
            circle(out, 21.0, 0.0, 7.5, "#3d4247", "");
            circle(out, 0.0, 0.0, 15.0, wd_for(v), "");
            circle(out, 4.0, -3.0, 3.5, "#d9a441", r#"stroke-width="1""#);
        }
        PropKind::Booth => {
            let f = pick(v, &["#8b2e2e", "#2f4f6b", "#5a3d2b"]);
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, 24.0], 6.0, f, "");
            rect(out, [-l + 4.0, t - 28.0, lw - 8.0, 24.0], 6.0, f, "");
            if painted {
                fabric_overlay(out, [-l + 4.0, -t + 4.0, lw - 8.0, 24.0], 6.0);
                fabric_overlay(out, [-l + 4.0, t - 28.0, lw - 8.0, 24.0], 6.0);
            }
            rect(out, [-l + 6.0, -t + 4.0, lw - 12.0, 8.0], 3.0, "#000", r#"fill-opacity=".2" stroke="none""#);
            rect(out, [-l + 6.0, t - 12.0, lw - 12.0, 8.0], 3.0, "#000", r#"fill-opacity=".2" stroke="none""#);
            rect(out, [-l + 14.0, -t + 34.0, lw - 28.0, lh - 68.0], 3.0, wd_for(v ^ 7), "");
        }
        PropKind::Bed => {
            let duvet = pick(v, &["#8fb3c9", "#c9b8d6", "#d8cbb0", "#9fb8a0"]);
            rect(out, [-l + 6.0, -t + 4.0, lw - 12.0, lh - 8.0], 5.0, "#e8e4dc", "");
            rect(out, [-l + 6.0, -t + 4.0, lw - 12.0, 10.0], 3.0, wd_for(v ^ 3), "");
            rect(out, [-l + 14.0, -t + 18.0, l - 20.0, 18.0], 7.0, "#fbfbf8", "");
            rect(out, [6.0, -t + 18.0, l - 20.0, 18.0], 7.0, "#fbfbf8", "");
            rect(out, [-l + 6.0, -t + 44.0, lw - 12.0, lh - 48.0], 4.0, duvet, "");
            if painted {
                fabric_overlay(out, [-l + 6.0, -t + 44.0, lw - 12.0, lh - 48.0], 4.0);
            }
            let _ = write!(out, r#"<path d="M{:.1} {:.1}H{:.1}" {thin}/>"#, -l + 6.0, -t + 54.0, l - 6.0);
        }
        PropKind::Desk => {
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh * 0.5], 2.0, wd_for(v), "");
            rect(out, [-14.0, -t + 8.0, 28.0, 5.0], 1.0, "#2f3439", "");
            rect(out, [-11.0, -t + 18.0, 22.0, 6.0], 1.0, "#e8e6e1", r#"stroke-width="1""#);
            rect(out, [l - 26.0, -t + 10.0, 12.0, 14.0], 0.0, "#f4f1ea", r#"stroke-width="1" transform="rotate(10)""#);
            circle(out, 0.0, t - 18.0, 10.0, "#3d4247", "");
            let _ = write!(out, r#"<path d="M-9 {:.1}Q0 {:.1} 9 {:.1}" fill="none" stroke-width="3.2" stroke-linecap="round"/>"#, t - 11.0, t - 4.0, t - 11.0);
        }
        PropKind::Bookshelf | PropKind::SteelShelving => {
            let steel = p.kind == PropKind::SteelShelving;
            let body = if steel {
                if painted { "url(#bm-steel)" } else { "#7f878d" }
            } else {
                wd_for(v)
            };
            let depth = lh * 0.48;
            rect(out, [-l + 3.0, -t + 3.0, lw - 6.0, depth], 1.5, body, "");
            let n = (lw / 9.0) as i32;
            for i in 0..n {
                let x = -l + 7.0 + i as f32 * 8.6;
                let c = if steel { "#c8a27a" } else { pick(v.wrapping_add(i as u32 * 31), BOTTLES) };
                rect(out, [x, -t + 7.0, if steel { 7.0 } else { 5.0 }, depth - 9.0], 0.5, c, r#"stroke-width=".8""#);
            }
        }
        PropKind::Wardrobe => {
            let depth = lh * 0.55;
            rect(out, [-l + 3.0, -t + 3.0, lw - 6.0, depth], 2.0, wd_for(v), "");
            let _ = write!(out, r#"<path d="M0 {:.1}V{:.1}" {thin}/>"#, -t + 3.0, -t + 3.0 + depth);
            circle(out, -4.0, -t + depth - 6.0, 1.8, BATTLEMAP_INK, "");
            circle(out, 4.0, -t + depth - 6.0, 1.8, BATTLEMAP_INK, "");
        }
        PropKind::KitchenCounter => {
            let depth = lh * 0.62;
            let top = if painted { "url(#bm-ceramic)" } else { "#d9d4ca" };
            let sink = if painted { "url(#bm-steel)" } else { "#b7c1c7" };
            rect(out, [-l + 1.0, -t + 2.0, lw - 2.0, depth], 1.5, top, "");
            let _ = write!(out, r#"<path d="M{:.1} {:.1}H{:.1}" {thin}/>"#, -l + 1.0, -t + 7.0, l - 1.0);
            if p.w_cells.max(p.h_cells) >= 2 {
                rect(out, [-12.0, -t + 10.0, 24.0, depth - 14.0], 4.0, sink, "");
                circle(out, 0.0, -t + 8.0, 2.2, "#8b959b", r#"stroke-width="1""#);
            }
            circle(out, -l + 14.0, -t + depth * 0.55, 4.5, "#c9502f", r#"stroke-width="1""#);
        }
        PropKind::Stove => {
            let depth = lh * 0.62;
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, depth], 2.0, if painted { "url(#bm-steel)" } else { "#3a3d41" }, "");
            if painted {
                rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, depth], 2.0, "#1f2226", r#"fill-opacity=".55" stroke="none""#);
            }
            for (x, y) in [(-12.0, -t + 12.0), (12.0, -t + 12.0), (-12.0, -t + depth - 9.0), (12.0, -t + depth - 9.0)] {
                circle(out, x, y, 6.5, "none", r##"stroke="#9aa0a6" stroke-width="1.6""##);
            }
        }
        PropKind::Fridge => {
            rect(out, [-l + 6.0, -t + 3.0, lw - 12.0, lh * 0.72], 3.0, if painted { "url(#bm-steel)" } else { "#e9ebec" }, "");
            let _ = write!(out, r#"<path d="M{:.1} {:.1}H{:.1}" {thin}/>"#, -l + 6.0, -t + lh * 0.3, l - 6.0);
            rect(out, [l - 14.0, -t + 10.0, 3.0, 12.0], 1.0, "#9aa0a6", r#"stroke-width=".8""#);
        }
        PropKind::BarCounter => {
            rect(out, [-l + 1.0, -t + 10.0, lw - 2.0, lh - 20.0], 3.0, if painted { "url(#bm-oak)" } else { "#5a3a24" }, "");
            if painted {
                rect(out, [-l + 1.0, -t + 10.0, lw - 2.0, lh - 20.0], 3.0, "#2a1608", r#"fill-opacity=".35" stroke="none""#);
            }
            rect(out, [-l + 1.0, t - 18.0, lw - 2.0, 8.0], 3.0, if painted { "url(#bm-oak)" } else { "#8a6a48" }, r#"stroke-width="1.4""#);
            let n = p.w_cells.max(p.h_cells);
            for i in 0..n {
                let x = -l + cell_px * (i as f32 + 0.5);
                if i % 2 == 0 {
                    circle(out, x, -t + 18.0, 3.2, "#c9c9c0", r#"stroke-width="1""#);
                } else {
                    rect(out, [x - 4.0, -t + 14.0, 8.0, 8.0], 2.0, "#d9c38a", r#"stroke-width="1" fill-opacity=".8""#);
                }
            }
        }
        PropKind::BackBarShelf => {
            let depth = lh * 0.45;
            rect(out, [-l + 1.0, -t + 2.0, lw - 2.0, depth], 1.0, if painted { "url(#bm-oak)" } else { "#3e2a1b" }, "");
            let n = (lw / 7.0) as i32;
            for i in 0..n {
                let c = pick(v.wrapping_add(i as u32 * 17), BOTTLES);
                circle(out, -l + 5.0 + i as f32 * 7.0, -t + 2.0 + depth * 0.5, 2.6, c, r#"stroke-width=".7""#);
            }
        }
        PropKind::BarStool => {
            circle(out, 0.0, 0.0, 11.0, "#2f3337", "");
            circle(out, 0.0, 0.0, 7.0, "#7a2f2f", r#"stroke-width="1""#);
        }
        PropKind::PoolTable => {
            rect(out, [-l + 8.0, -t + 8.0, lw - 16.0, lh - 16.0], 8.0, if painted { "url(#bm-oak)" } else { "#5a3a24" }, "");
            rect(out, [-l + 18.0, -t + 18.0, lw - 36.0, lh - 36.0], 2.0, "#1f6b3e", r#"stroke-width="1.4""#);
            if painted {
                fabric_overlay(out, [-l + 18.0, -t + 18.0, lw - 36.0, lh - 36.0], 2.0);
            }
            for (x, y) in [(-l + 16.0, -t + 16.0), (0.0, -t + 14.0), (l - 16.0, -t + 16.0), (-l + 16.0, t - 16.0), (0.0, t - 14.0), (l - 16.0, t - 16.0)] {
                circle(out, x, y, 5.5, "#111", r#"stroke="none""#);
            }
            for (i, c) in ["#f4f1ea", "#d9a441", "#b5462f", "#2f5a7a", "#111"].iter().enumerate() {
                let h = battlemap_mix_u64(u64::from(v) + i as u64);
                let x = ((h % 1000) as f32 / 1000.0 - 0.5) * (lw - 60.0);
                let y = (((h >> 12) % 1000) as f32 / 1000.0 - 0.5) * (lh - 60.0);
                circle(out, x, y, 4.0, c, r#"stroke-width="1""#);
            }
            let _ = write!(out, r##"<path d="M{:.1} {:.1}L{:.1} {:.1}" stroke="#c4a27c" stroke-width="2.5" stroke-linecap="round"/>"##, -l + 30.0, t - 30.0, -l + 90.0, 0.0);
        }
        PropKind::Jukebox => {
            let _ = write!(
                out,
                r##"<path d="M{x0:.1} {y1:.1}V{ym:.1}A{r:.1} {r:.1} 0 0 1 {x1:.1} {ym:.1}V{y1:.1}Z" fill="#a8452f"/><circle cx="0" cy="{ym:.1}" r="{ri:.1}" fill="#f2c14e" stroke-width="1.2"/>"##,
                x0 = -l + 10.0,
                x1 = l - 10.0,
                y1 = t - 12.0,
                ym = -t + 10.0 + (l - 10.0),
                r = l - 10.0,
                ri = (l - 18.0).max(4.0)
            );
        }
        PropKind::Bathtub => {
            rect(out, [-l + 4.0, -t + 6.0, lw - 8.0, lh - 12.0], 14.0, if painted { "url(#bm-ceramic)" } else { "#f1f2f3" }, "");
            rect(out, [-l + 12.0, -t + 13.0, lw - 24.0, lh - 26.0], 12.0, "#cfe3ee", r#"stroke-width="1.2""#);
            circle(out, l - 22.0, 0.0, 2.5, "#8b959b", r#"stroke-width="1""#);
        }
        PropKind::Toilet => {
            rect(out, [-14.0, -t + 5.0, 28.0, 11.0], 3.0, "#f4f4f2", "");
            let _ = write!(out, r##"<ellipse cx="0" cy="{:.1}" rx="11" ry="14" fill="#f4f4f2"/><ellipse cx="0" cy="{:.1}" rx="6" ry="8" fill="#cfe3ee" stroke-width="1"/>"##, -t + 31.0, -t + 32.0);
        }
        PropKind::LargeTree | PropKind::MediumTree => {
            let big = p.kind == PropKind::LargeTree;
            let r = if big { lw.min(lh) * 0.24 } else { lw.min(lh) * 0.26 };
            let flare = battlemap_blob_path(0.0, 0.0, r * 1.38, 7, 0.24, u64::from(v) ^ 0x77);
            let _ = write!(out, r##"<path d="{flare}" fill="#593d26"/>"##);
            circle(out, 0.0, 0.0, r, "#6e4d32", "");
            circle(out, -r * 0.14, -r * 0.14, r * 0.55, "#8a6646", r#"stroke-width="1""#);
        }
        PropKind::Hedge => {
            let outer = pick(v, CANOPY_OUTER);
            let inner = pick(v, CANOPY_MID);
            let sun = pick(v, CANOPY_SUN);
            let cells = (lw / cell_px).round().max(1.0) as i32;
            let r_base = lh * 0.42;
            rect(out, [-l + 4.0, -t + 8.0, lw - 8.0, lh - 16.0], r_base * 0.75, if painted { "url(#bm-leaf-tex)" } else { outer }, "");
            for i in 0..(cells * 2) {
                let frac = (i as f32 + 0.5) / (cells * 2) as f32;
                let hx = -l + 10.0 + frac * (lw - 20.0);
                let hy = ((battlemap_mix_u64(u64::from(v) + i as u64 * 13) % 9) as f32 - 4.0) * 1.1;
                let blob = battlemap_blob_path(hx, hy, r_base * 0.78, 7, 0.18, u64::from(v).wrapping_add(i as u64 * 29));
                let _ = write!(out, r#"<path d="{blob}" fill="{inner}" fill-opacity="{op}" stroke-width="1.3"/>"#, op = if painted { ".6" } else { "1" });
                circle(out, hx - 3.0, hy - 3.0, r_base * 0.30, sun, r#"stroke="none" fill-opacity=".65""#);
            }
        }
        PropKind::Bush => {
            let outer = pick(v, CANOPY_OUTER);
            let mid = pick(v, CANOPY_MID);
            let sun = pick(v, CANOPY_SUN);
            let b0 = battlemap_blob_path(0.0, 1.0, l * 0.78, 9, 0.24, u64::from(v));
            let b1 = battlemap_blob_path(-5.0, -4.0, l * 0.54, 7, 0.22, u64::from(v) ^ 0x3A);
            let b2 = battlemap_blob_path(7.0, 4.0, l * 0.42, 7, 0.20, u64::from(v) ^ 0x91);
            let leaf_op = if painted { ".82" } else { ".68" };
            let _ = write!(
                out,
                r##"<path d="{b0}" fill="{outer}"/><path d="{b0}" fill="url(#bm-leaf-tex)" fill-opacity="{leaf_op}" stroke="none"/><path d="{b1}" fill="{mid}" fill-opacity=".65" stroke-width="1.2"/><path d="{b2}" fill="{sun}" stroke="none" fill-opacity=".65"/>"##
            );
            for i in 0..5 {
                let a = (i as f32 / 5.0) * TAU + (v as f32 * 0.2);
                let (lx, ly) = (a.cos() * l * 0.55, a.sin() * l * 0.55);
                let deg = a.to_degrees();
                let _ = write!(
                    out,
                    r##"<ellipse cx="{lx:.1}" cy="{ly:.1}" rx="6.2" ry="3.2" transform="rotate({deg:.1} {lx:.1} {ly:.1})" fill="{sun}" fill-opacity=".72" stroke-width=".9"/>"##
                );
            }
        }
        PropKind::Boulder => {
            let rock = battlemap_blob_path(0.0, 0.0, lw.min(lh) * 0.46, 7, 0.20, u64::from(v));
            let sx = (lw / lh.max(1.0)) * 0.92;
            let fill = if painted { "url(#bm-rock)" } else { "#7b7972" };
            let _ = write!(
                out,
                r##"<g transform="scale({sx:.2} 0.88)"><path d="{rock}" fill="{fill}"/><path d="{}" fill="#9e9b93" stroke="none" fill-opacity="{hi_op}"/><path d="M{:.1} -2L-4 -6L8 3L{:.1} 1" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.3" stroke-opacity=".65"/></g>"##,
                battlemap_blob_path(-3.0, -4.0, lw.min(lh) * 0.28, 6, 0.18, u64::from(v) ^ 0x4C),
                -lw.min(lh) * 0.30,
                lw.min(lh) * 0.28,
                hi_op = if painted { ".38" } else { ".82" },
            );
        }
        PropKind::ParkBench => {
            rect(out, [-l + 6.0, -t + 10.0, lw - 12.0, lh - 22.0], 3.0, wd_for(v), "");
            rect(out, [-l + 6.0, -t + 8.0, lw - 12.0, 9.0], 2.5, "#6b4c32", "");
            let _ = write!(
                out,
                r#"<path d="M{x0:.1} {y1:.1}H{x1:.1}M{x0:.1} {y2:.1}H{x1:.1}" {thin}/>"#,
                x0 = -l + 7.0,
                x1 = l - 7.0,
                y1 = -t + 25.0,
                y2 = -t + 34.0
            );
            rect(out, [-l + 4.0, -t + 9.0, 6.0, lh - 18.0], 2.0, "#2f3337", "");
            rect(out, [l - 10.0, -t + 9.0, 6.0, lh - 18.0], 2.0, "#2f3337", "");
        }
        PropKind::PicnicTable => {
            let wd = wd_for(v);
            rect(out, [-l + 8.0, -t + 6.0, lw - 16.0, 16.0], 3.0, wd, "");
            rect(out, [-l + 8.0, t - 22.0, lw - 16.0, 16.0], 3.0, wd, "");
            rect(out, [-l + 6.0, -t + 28.0, lw - 12.0, lh - 56.0], 4.0, wd, "");
            let _ = write!(
                out,
                r#"<path d="M{x0:.1} -8H{x1:.1}M{x0:.1} 8H{x1:.1}" {thin}/>"#,
                x0 = -l + 8.0,
                x1 = l - 8.0
            );
        }
        PropKind::Dumpster => {
            let body = pick(v, &["#2f5d41", "#2f4b69", "#524a3e"]);
            rect(out, [-l + 5.0, -t + 6.0, lw - 10.0, lh - 12.0], 3.0, body, "");
            if painted {
                rect(out, [-l + 5.0, -t + 6.0, lw - 10.0, lh - 12.0], 3.0, "url(#bm-steel)", r#"fill-opacity=".35" style="mix-blend-mode:multiply" stroke="none""#);
            }
            rect(out, [-l + 9.0, -t + 10.0, l - 11.0, lh - 20.0], 2.0, "#2b2e31", "");
            rect(out, [2.0, -t + 10.0, l - 11.0, lh - 20.0], 2.0, "#2b2e31", "");
            let _ = write!(
                out,
                r#"<path d="M{:.1} 0H{:.1}M2 0H{:.1}" {thin}/>"#,
                -l + 11.0,
                -3.0,
                l - 11.0
            );
            rect(out, [-l + 1.0, -6.0, 4.0, 12.0], 1.0, body, r#"stroke-width="1.3""#);
            rect(out, [l - 5.0, -6.0, 4.0, 12.0], 1.0, body, r#"stroke-width="1.3""#);
        }
        PropKind::JerseyBarrier => {
            rect(out, [-l + 2.0, -t + 14.0, lw - 4.0, lh - 28.0], 3.0, if painted { "url(#bm-concrete)" } else { "#b8b5ad" }, "");
            rect(out, [-l + 4.0, -6.0, lw - 8.0, 12.0], 2.0, "#d5d2ca", r#"stroke-width="1.2" fill-opacity=".6""#);
            rect(out, [-l + 4.0, -5.0, 8.0, 10.0], 1.0, "#e0a92e", r#"stroke-width="1""#);
            rect(out, [l - 12.0, -5.0, 8.0, 10.0], 1.0, "#e0a92e", r#"stroke-width="1""#);
        }
        PropKind::Car | PropKind::UtilityVan => {
            let van = p.kind == PropKind::UtilityVan;
            let body = if van {
                pick(v, &["#e2e4e6", "#3d4f60", "#586257", "#2c3034", "#8a3b32"])
            } else {
                pick(v, &["#4b6a8f", "#8f2f2f", "#d8d4cc", "#2f3337", "#6b7a5a"])
            };
            let corner_rx = if van { 14.0 } else { 22.0 };
            rect(out, [-l + 7.0, -t + 6.0, lw - 14.0, lh - 12.0], corner_rx, body, "");
            let (roof_y0, roof_h) = if van { (-t + 18.0, lh * 0.56) } else { (-t + lh * 0.30, lh * 0.38) };
            rect(out, [-l + 15.0, roof_y0, lw - 30.0, roof_h], 9.0, "#000", r#"fill-opacity=".12""#);
            if van {
                for ry in [roof_y0 + roof_h * 0.28, roof_y0 + roof_h * 0.72] {
                    rect(out, [-l + 13.0, ry, lw - 26.0, 4.0], 1.5, "#3a3f45", r#"stroke-width="1""#);
                }
            }
            let _ = write!(
                out,
                r##"<path d="M{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}Z" fill="#2c3e50" fill-opacity=".85"/><path d="M{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}Z" fill="#2c3e50" fill-opacity=".85"/>"##,
                -l + 17.0, t - lh * 0.28, l - 17.0, t - lh * 0.28, l - 23.0, t - lh * 0.18, -l + 23.0, t - lh * 0.18,
                -l + 19.0, roof_y0, l - 19.0, roof_y0, l - 24.0, roof_y0 - 14.0, -l + 24.0, roof_y0 - 14.0,
            );
            circle(out, -l + 22.0, t - 12.0, 5.0, "#f6e7a1", r#"stroke-width="1""#);
            circle(out, l - 22.0, t - 12.0, 5.0, "#f6e7a1", r#"stroke-width="1""#);
            rect(out, [-l + 16.0, -t + 7.0, 14.0, 4.5], 1.5, "#b83232", r#"stroke-width="1""#);
            rect(out, [l - 30.0, -t + 7.0, 14.0, 4.5], 1.5, "#b83232", r#"stroke-width="1""#);
            rect(out, [-l + 2.0, t - lh * 0.31, 7.0, 10.0], 2.0, body, r#"stroke-width="1.2""#);
            rect(out, [l - 9.0, t - lh * 0.31, 7.0, 10.0], 2.0, body, r#"stroke-width="1.2""#);
        }
        PropKind::StonePath => {
            rect(out, [-l, -t, lw, lh], 2.0, if painted { "url(#bm-paver)" } else { "#cfc7b8" }, r#"stroke-width="1" opacity=".9""#);
        }
        PropKind::AsphaltDrive => {
            rect(out, [-l, -t, lw, lh], 0.0, if painted { "url(#bm-tarmac)" } else { "#55585c" }, r#"stroke-width="1" opacity=".95""#);
        }
        PropKind::GravelPath => {
            rect(out, [-l, -t, lw, lh], 3.0, "url(#bm-gravel)", r#"stroke-width="1""#);
        }
        PropKind::DirtPath => {
            rect(out, [-l, -t, lw, lh], 3.0, "url(#bm-dirt)", r#"stroke-width="1""#);
        }
        PropKind::MulchBed => {
            rect(out, [-l, -t, lw, lh], 5.0, "url(#bm-mulch)", r#"stroke-width="1""#);
        }
        PropKind::CreekWater => {
            rect(out, [-l, -t, lw, lh], 4.0, "url(#bm-creek)", r##"stroke="#2a5d73" stroke-width="1.2""##);
        }
        PropKind::DrainageDitch => {
            rect(out, [-l, -t, lw, lh], 3.0, "url(#bm-ditch)", r##"stroke="#354032" stroke-width="1.2""##);
        }
        PropKind::DeskL => {
            let wd = wd_for(v);
            let x0 = -l + 4.0;
            let y0 = -t + 5.0;
            let x1 = l - 4.0;
            let y1 = t - 5.0;
            let xm = (-l + cell_px - 6.0).min(x1 - 12.0);
            let ym = (-t + cell_px - 6.0).min(y1 - 12.0);
            let _ = write!(
                out,
                r##"<path d="M{x0:.1} {y0:.1}H{x1:.1}V{ym:.1}H{xm:.1}V{y1:.1}H{x0:.1}Z" fill="{wd}"/>"##
            );
            let _ = write!(
                out,
                r##"<rect x="{:.1}" y="{:.1}" width="24" height="5" rx="1.5" transform="rotate(-35 {:.1} {:.1})" fill="#2a3038"/><rect x="{:.1}" y="{:.1}" width="18" height="7" rx="1.5" transform="rotate(-35 {:.1} {:.1})" fill="#3b424c" stroke-width="1"/>"##,
                x0 + 14.0, y0 + 16.0, x0 + 26.0, y0 + 18.0,
                x0 + 24.0, y0 + 26.0, x0 + 33.0, y0 + 29.0,
            );
            let (sx, sy) = (cell_px * 0.45, cell_px * 0.45);
            circle(out, sx, sy, 13.0, "#35393f", r#"stroke-width="1.5""#);
            rect(out, [sx - 10.0, sy + 6.0, 20.0, 6.0], 3.0, "#24272b", r#"stroke-width="1""#);
        }
        PropKind::DeskT => {
            let wd = wd_for(v);
            let x0 = -l + 4.0;
            let x1 = l - 4.0;
            let y0 = -t + 5.0;
            let y1 = t - 5.0;
            let ym = (-t + cell_px - 6.0).min(y1 - 12.0);
            let sx0 = -cell_px * 0.40;
            let sx1 = cell_px * 0.40;
            let _ = write!(
                out,
                r##"<path d="M{x0:.1} {y0:.1}H{x1:.1}V{ym:.1}H{sx1:.1}V{y1:.1}H{sx0:.1}V{ym:.1}H{x0:.1}Z" fill="{wd}"/>"##
            );
            rect(out, [-cell_px * 0.72, y0 + 5.0, 22.0, 5.0], 1.5, "#2a3038", r#"stroke-width="1""#);
            rect(out, [cell_px * 0.72 - 22.0, y0 + 5.0, 22.0, 5.0], 1.5, "#2a3038", r#"stroke-width="1""#);
            rect(out, [-10.0, 2.0, 20.0, 12.0], 2.0, "#dce4ec", r#"stroke-width="1""#);
            for cx_seat in [-cell_px * 0.96, cell_px * 0.96] {
                let sy = cell_px * 0.44;
                circle(out, cx_seat, sy, 12.0, "#35393f", r#"stroke-width="1.4""#);
                rect(out, [cx_seat - 9.0, sy + 5.0, 18.0, 5.5], 2.5, "#24272b", r#"stroke-width="1""#);
            }
        }
        PropKind::FallenLog => {
            let bw = lw - 12.0;
            let bh = (lh * 0.50).max(20.0);
            let bx0 = -bw * 0.5;
            let by0 = -bh * 0.5;
            rect(out, [-bw * 0.15, -bh * 0.85, 9.0, bh * 0.55], 3.0, "#5e432b", r#"stroke-width="1.4""#);
            rect(out, [bx0, by0, bw, bh], 7.0, "#6d4e33", "");
            let grain_end = if painted { "url(#bm-oak)" } else { "#c9a578" };
            let _ = write!(
                out,
                r##"<ellipse cx="{:.1}" cy="0" rx="5.5" ry="{:.1}" fill="{grain_end}"/><ellipse cx="{:.1}" cy="0" rx="2.6" ry="{:.1}" fill="#8a623f" stroke="none"/><ellipse cx="{:.1}" cy="0" rx="5.5" ry="{:.1}" fill="{grain_end}"/>"##,
                bx0 + 4.0, bh * 0.44,
                bx0 + 4.0, bh * 0.22,
                bx0 + bw - 4.0, bh * 0.44,
            );
            let _ = write!(
                out,
                r#"<path d="M{:.1} -5H{:.1}M{:.1} 4H{:.1}" {thin}/>"#,
                bx0 + 14.0, bx0 + bw - 14.0, bx0 + 18.0, bx0 + bw - 18.0
            );
            let moss1 = battlemap_blob_path(-bw * 0.20, -bh * 0.22, bh * 0.34, 6, 0.22, u64::from(v) ^ 0x19);
            let moss2 = battlemap_blob_path(bw * 0.16, bh * 0.18, bh * 0.30, 6, 0.22, u64::from(v) ^ 0x4E);
            let _ = write!(
                out,
                r##"<path d="{moss1}" fill="#6e9e44" fill-opacity=".86" stroke-width="1"/><path d="{moss2}" fill="#5c8a38" fill-opacity=".82" stroke-width="1"/>"##
            );
        }
        PropKind::GardenBed => {
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 4.0, if painted { "url(#bm-brick)" } else { "#755034" }, "");
            rect(out, [-l + 10.0, -t + 10.0, lw - 20.0, lh - 20.0], 2.0, if painted { "url(#bm-mulch)" } else { "#47301e" }, r#"stroke-width="1.2""#);
            let cols = ((lw - 20.0) / 28.0).round().max(2.0) as i32;
            let rows = ((lh - 20.0) / 24.0).round().max(2.0) as i32;
            const BLOOMS: &[&str] = &["#e25b70", "#f2c14e", "#a27ce0", "#f58a51"];
            for r in 0..rows {
                for c_idx in 0..cols {
                    let fx = -l + 20.0 + (c_idx as f32 + 0.5) * ((lw - 40.0) / cols as f32);
                    let fy = -t + 18.0 + (r as f32 + 0.5) * ((lh - 36.0) / rows as f32);
                    let lf = battlemap_blob_path(fx, fy, 10.0, 6, 0.22, u64::from(v).wrapping_add((r * 7 + c_idx) as u64));
                    let bloom = pick(v.wrapping_add((r * 13 + c_idx * 5) as u32), BLOOMS);
                    let _ = write!(out, r##"<path d="{lf}" fill="#5e9141" stroke-width="1"/>"##);
                    circle(out, fx + 1.5, fy - 1.5, 3.3, bloom, r#"stroke-width=".8""#);
                }
            }
        }
        PropKind::GardenEdging => {
            let stones = ((lw / 18.0).round() as i32).max(3);
            let sw = (lw - 6.0) / stones as f32;
            const STONES: &[&str] = &["#beb6a6", "#a8a090", "#cfc8ba", "#9c9486"];
            for i in 0..stones {
                let sx = -l + 3.0 + i as f32 * sw;
                let col = if painted { "url(#bm-rock)" } else { pick(v.wrapping_add(i as u32 * 11), STONES) };
                let h_var = if i % 2 == 0 { 15.0 } else { 13.0 };
                rect(out, [sx, -h_var * 0.5, (sw - 1.5).max(8.0), h_var], 5.0, col, r#"stroke-width="1.3""#);
            }
        }
        PropKind::StructuralPillar => {
            rect(out, [-l + 3.0, -t + 3.0, lw - 6.0, lh - 6.0], 3.0, if painted { "url(#bm-steel)" } else { "#6c737c" }, "");
            rect(out, [-l + 8.0, -t + 8.0, lw - 16.0, lh - 16.0], 2.0, if painted { "url(#bm-concrete)" } else { "#949ca6" }, r#"stroke-width="1.2""#);
            let _ = write!(
                out,
                r#"<path d="M{:.1} {:.1}L{:.1} {:.1}M{:.1} {:.1}L{:.1} {:.1}" {thin}/>"#,
                -l + 6.0, -t + 6.0, l - 6.0, t - 6.0,
                l - 6.0, -t + 6.0, -l + 6.0, t - 6.0,
            );
        }
        PropKind::ElevatedCatwalk => {
            rect(out, [-l + 1.0, -t + 1.0, lw - 2.0, lh - 2.0], 3.0, if painted { "url(#bm-steel)" } else { "#75808b" }, r#"fill-opacity=".92""#);
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 2.0, "none", r##"stroke="#d4dbe2" stroke-width="1.4" stroke-dasharray="6 4""##);
            let rungs = ((lw / 20.0).round() as i32).max(2);
            for i in 1..rungs {
                let rx = -l + (i as f32 / rungs as f32) * lw;
                let _ = write!(out, r#"<line x1="{rx:.1}" y1="{:.1}" x2="{rx:.1}" y2="{:.1}" {thin}/>"#, -t + 4.0, t - 4.0);
            }
        }
        PropKind::HazardSpill => {
            let spill = battlemap_blob_path(0.0, 0.0, lw.min(lh) * 0.48, 11, 0.24, u64::from(v));
            let inner = battlemap_blob_path(3.0, -2.0, lw.min(lh) * 0.28, 7, 0.20, u64::from(v) ^ 0x4B);
            let fill = if painted { "url(#bm-spill)" } else { "#c6d832" };
            let _ = write!(
                out,
                r##"<path d="{spill}" fill="{fill}" fill-opacity=".76" stroke="#6d7812" stroke-width="1.6"/><path d="{inner}" fill="#e6f74a" fill-opacity="{in_op}" stroke="none"/>"##,
                in_op = if painted { ".28" } else { ".55" },
            );
        }
        PropKind::Stairwell => {
            rect(
                out,
                [-l + 2.0, -t + 2.0, lw - 4.0, lh - 4.0],
                3.0,
                if painted { "url(#bm-concrete)" } else { "#c8ccd2" },
                "",
            );
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
            rect(
                out,
                [-l + 3.0, -t + 3.0, lw - 6.0, lh - 6.0],
                3.0,
                if painted { "url(#bm-steel)" } else { "#7c8794" },
                "",
            );
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
            let wd = wd_for(v);
            rect(out, [-l + 3.0, -t + 4.0, lw - 6.0, lh - 8.0], 3.0, wd, "");
            rect(out, [-l + 10.0, -t + 8.0, (lw * 0.46).max(16.0), lh - 16.0], 2.0, "#31363f", r#"stroke-width="1.2""#);
            rect(out, [l - 34.0, -t + 9.0, 22.0, 16.0], 2.0, "#dce4ec", r#"stroke-width="1.2""#);
            rect(out, [l - 30.0, -t + 12.0, 14.0, 7.0], 1.0, "#38bdf8", r#"stroke="none" fill-opacity=".8""#);
        }
        PropKind::Armchair | PropKind::Planter | PropKind::WoodCrate | PropKind::DrumCluster => {}
    }
    out.push_str("</g>");
}

/// Appends an overhanging, semi-translucent leafy crown for [`PropKind::LargeTree`] and
/// [`PropKind::MediumTree`] so canopies layer naturally above paths, benches, hedges, and walls while
/// keeping tokens and the visual `1.0m` scale grid legible above `#battlemap-art`.
pub fn push_tree_canopy_glyph(out: &mut String, p: &MapPropPlacement, cell_px: f32) {
    push_tree_canopy_glyph_with_mode(out, p, cell_px, BattlemapTextureMode::Painted);
}

/// Mode-aware variant of [`push_tree_canopy_glyph`] accentuating painterly foliage leaf textures.
pub fn push_tree_canopy_glyph_with_mode(
    out: &mut String,
    p: &MapPropPlacement,
    cell_px: f32,
    tex_mode: BattlemapTextureMode,
) {
    if !matches!(p.kind, PropKind::LargeTree | PropKind::MediumTree) {
        return;
    }
    let painted = tex_mode.is_painted();
    let (cx_m, cy_m) = p.center_m();
    let (cx, cy) = (cx_m * cell_px, cy_m * cell_px);
    let (sw_m, sh_m) = p.local_span_m();
    let (fw, fh) = (sw_m * cell_px, sh_m * cell_px);
    let v = p.variant_seed;
    let r = if p.kind == PropKind::LargeTree {
        fw.max(fh) * 0.94
    } else {
        cell_px * 1.15
    };
    let outer = pick(v, CANOPY_OUTER);
    let mid = pick(v, CANOPY_MID);
    let sun = pick(v, CANOPY_SUN);
    let d_outer = battlemap_blob_path(cx, cy, r, 15, 0.19, u64::from(v));
    let d_mid = battlemap_blob_path(cx - r * 0.08, cy - r * 0.08, r * 0.72, 11, 0.21, u64::from(v) ^ 0x55);
    let d_sun = battlemap_blob_path(cx - r * 0.20, cy - r * 0.20, r * 0.42, 8, 0.22, u64::from(v) ^ 0xAA);
    let leaf_op = if painted { ".78" } else { ".62" };
    let _ = write!(
        out,
        r##"<g stroke="{BATTLEMAP_INK}" stroke-linejoin="round"><path d="{d_outer}" fill="{outer}" fill-opacity=".84" stroke-width="2.2"/><path d="{d_outer}" fill="url(#bm-leaf-tex)" fill-opacity="{leaf_op}" stroke="none"/>"##
    );
    // Satellite leaf lobes around crown perimeter for organic silhouette
    for i in 0..6 {
        let h = battlemap_mix_u64(u64::from(v) ^ (i as u64 * 0x2F));
        let ang = (i as f32 / 6.0) * TAU + ((h % 100) as f32 * 0.005);
        let lx = cx + ang.cos() * r * 0.58;
        let ly = cy + ang.sin() * r * 0.58;
        let lobe = battlemap_blob_path(lx, ly, r * 0.36, 7, 0.18, h);
        let col = if i < 3 { mid } else { outer };
        let _ = write!(out, r#"<path d="{lobe}" fill="{col}" fill-opacity=".55" stroke-width="1.1" stroke-opacity=".5"/>"#);
    }
    let _ = write!(
        out,
        r#"<path d="{d_mid}" fill="{mid}" fill-opacity=".52" stroke-width="1.2" stroke-opacity=".4"/><path d="{d_sun}" fill="{sun}" fill-opacity=".52" stroke="none"/>"#
    );
    // Individual sunlit leaf clusters for crisp top-down botanical texture
    for i in 0..10 {
        let h = battlemap_mix_u64(u64::from(v).wrapping_add(i as u64 * 97));
        let ang = (i as f32 / 10.0) * TAU + ((h % 60) as f32 * 0.01);
        let dist = r * (0.28 + ((h >> 8) % 52) as f32 * 0.01);
        let lx = cx + ang.cos() * dist;
        let ly = cy + ang.sin() * dist;
        let deg = ang.to_degrees() + 35.0;
        let rx = if p.kind == PropKind::LargeTree { 8.5 } else { 6.2 };
        let ry = rx * 0.48;
        let _ = write!(
            out,
            r#"<ellipse cx="{lx:.1}" cy="{ly:.1}" rx="{rx:.1}" ry="{ry:.1}" transform="rotate({deg:.1} {lx:.1} {ly:.1})" fill="{sun}" fill-opacity=".65" stroke-width=".7" stroke-opacity=".4"/>"#
        );
    }
    out.push_str("</g>");
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_engine::map_dressing::make_prop;

    #[test]
    fn every_prop_kind_renders_a_balanced_group() {
        for kind in PropKind::ALL {
            for rot in [33, 90, 217] {
                let mut s = String::new();
                push_furniture_glyph(&mut s, &make_prop(kind, 2, 3, rot, Some(4), 11), 64.0);
                assert!(s.starts_with("<g ") && s.ends_with("</g>"), "{kind:?} rot={rot}");
                assert_eq!(s.matches("<g").count(), s.matches("</g>").count(), "{kind:?} rot={rot}");
                assert!(!s.contains("NaN"), "{kind:?} rot={rot}");
            }
        }
        for tree in [PropKind::LargeTree, PropKind::MediumTree] {
            let mut c = String::new();
            push_tree_canopy_glyph(&mut c, &make_prop(tree, 2, 2, 0, None, 7), 64.0);
            assert!(c.contains("url(#bm-leaf-tex)"));
            assert_eq!(c.matches("<g").count(), c.matches("</g>").count());
            assert!(!c.contains("NaN"));
        }
    }
}
