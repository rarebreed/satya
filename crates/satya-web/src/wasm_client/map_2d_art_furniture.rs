//! # Multi-Cell Furniture Glyphs for the 2D Battlemap (`map_2d_art_furniture`)
//!
//! Draws one top-down glyph per [`MapPropPlacement`], scaled to its whole footprint, so a 3-seat sofa
//! spans 3 cells, a bar counter spans its full run, and a car spans `2 × 5` cells. Glyphs are authored in a
//! local frame centered on the footprint (`±lw/2 × ±lh/2`) with the **back facing `-y`**, then rotated by
//! `rotation_deg`. Single-cell legacy glyphs (armchair, planter, crate, drums) reuse
//! [`crate::wasm_client::map_2d_art_props::push_battlemap_prop`].

use crate::wasm_client::map_2d_art_props::{
    BATTLEMAP_INK, BattlemapPropKind, battlemap_blob_path, battlemap_mix_u64, push_battlemap_prop,
};
use satya_schema::{MapPropPlacement, PropKind};
use std::fmt::Write as _;

const FABRICS: &[&str] = &["#7d8e9b", "#a0705a", "#6d7f68", "#8c7f9e", "#b49a6a", "#5f6b78"];
const WOODS: &[&str] = &["#a47b55", "#c4a27c", "#7a5c43", "#b8926a"];
const BOTTLES: &[&str] = &["#3f7a3a", "#8a5a2b", "#d9c38a", "#6b2f2f", "#2f5a7a", "#c9c9c0"];

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

/// Appends the footprint-scaled glyph for `p` (`cell_px` SVG units per cell).
pub fn push_furniture_glyph(out: &mut String, p: &MapPropPlacement, cell_px: f32) {
    let (fw, fh) = (p.w_cells as f32 * cell_px, p.h_cells as f32 * cell_px);
    let (cx, cy) = (p.x as f32 * cell_px + fw * 0.5, p.y as f32 * cell_px + fh * 0.5);
    let rot = p.rotation_deg.rem_euclid(360);
    let legacy = match p.kind {
        PropKind::Armchair => Some(BattlemapPropKind::Armchair),
        PropKind::Planter => Some(BattlemapPropKind::Planter),
        PropKind::WoodCrate => Some(BattlemapPropKind::WoodCrate),
        PropKind::DrumCluster => Some(BattlemapPropKind::DrumCluster),
        _ => None,
    };
    if let Some(kind) = legacy {
        push_battlemap_prop(out, kind, cx, cy, rot as u16, u64::from(p.variant_seed));
        return;
    }
    let (lw, lh) = if rot % 180 == 90 { (fh, fw) } else { (fw, fh) };
    let (l, t) = (lw * 0.5, lh * 0.5);
    let v = p.variant_seed;
    let thin = r#"stroke-width="1.1" stroke-opacity=".55""#;
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
            let seats = p.w_cells.max(p.h_cells).max(2);
            let span = lw - 30.0;
            for i in 1..seats {
                let x = -l + 15.0 + span * i as f32 / seats as f32;
                let _ = write!(out, r#"<path d="M{x:.1} {:.1}V{:.1}" {thin}/>"#, -t + 23.0, t - 8.0);
            }
        }
        PropKind::CoffeeTable => {
            let wd = pick(v, WOODS);
            rect(out, [-l + 10.0, -t + 14.0, lw - 20.0, lh - 28.0], 5.0, wd, "");
            rect(out, [-l + 22.0, -t + 22.0, 16.0, 11.0], 1.0, "#e9e3d6", r#"stroke-width="1""#);
            circle(out, l - 26.0, 0.0, 5.0, "#f4f1ea", r#"stroke-width="1.2""#);
        }
        PropKind::Rug => {
            let c = pick(v, &["#9b3d33", "#3d5a80", "#6b5b3e", "#4f6d5a"]);
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh - 8.0], 2.0, c, r#"stroke-width="1.2" opacity=".85""#);
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
            let wd = pick(v, WOODS);
            for (x, y) in [(-l * 0.45, -t + 6.0), (l * 0.45 - 16.0, -t + 6.0), (-l * 0.45, t - 20.0), (l * 0.45 - 16.0, t - 20.0)] {
                rect(out, [x, y, 16.0, 14.0], 4.0, "#5b4a3a", "");
            }
            for (x, y) in [(-l + 4.0, -8.0), (l - 18.0, -8.0)] {
                rect(out, [x, y, 14.0, 16.0], 4.0, "#5b4a3a", "");
            }
            rect(out, [-l + 20.0, -t + 18.0, lw - 40.0, lh - 36.0], 4.0, wd, "");
            circle(out, 0.0, 0.0, 7.0, "#f4f1ea", r#"stroke-width="1.2""#);
        }
        PropKind::RoundTable => {
            circle(out, -21.0, 0.0, 7.5, "#3d4247", "");
            circle(out, 21.0, 0.0, 7.5, "#3d4247", "");
            circle(out, 0.0, 0.0, 15.0, pick(v, WOODS), "");
            circle(out, 4.0, -3.0, 3.5, "#d9a441", r#"stroke-width="1""#);
        }
        PropKind::Booth => {
            let f = pick(v, &["#8b2e2e", "#2f4f6b", "#5a3d2b"]);
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, 24.0], 6.0, f, "");
            rect(out, [-l + 4.0, t - 28.0, lw - 8.0, 24.0], 6.0, f, "");
            rect(out, [-l + 6.0, -t + 4.0, lw - 12.0, 8.0], 3.0, "#000", r#"fill-opacity=".2" stroke="none""#);
            rect(out, [-l + 6.0, t - 12.0, lw - 12.0, 8.0], 3.0, "#000", r#"fill-opacity=".2" stroke="none""#);
            rect(out, [-l + 14.0, -t + 34.0, lw - 28.0, lh - 68.0], 3.0, pick(v ^ 7, WOODS), "");
        }
        PropKind::Bed => {
            let duvet = pick(v, &["#8fb3c9", "#c9b8d6", "#d8cbb0", "#9fb8a0"]);
            rect(out, [-l + 6.0, -t + 4.0, lw - 12.0, lh - 8.0], 5.0, "#e8e4dc", "");
            rect(out, [-l + 6.0, -t + 4.0, lw - 12.0, 10.0], 3.0, pick(v ^ 3, WOODS), "");
            rect(out, [-l + 14.0, -t + 18.0, l - 20.0, 18.0], 7.0, "#fbfbf8", "");
            rect(out, [6.0, -t + 18.0, l - 20.0, 18.0], 7.0, "#fbfbf8", "");
            rect(out, [-l + 6.0, -t + 44.0, lw - 12.0, lh - 48.0], 4.0, duvet, "");
            let _ = write!(out, r#"<path d="M{:.1} {:.1}H{:.1}" {thin}/>"#, -l + 6.0, -t + 54.0, l - 6.0);
        }
        PropKind::Desk => {
            let wd = pick(v, WOODS);
            rect(out, [-l + 4.0, -t + 4.0, lw - 8.0, lh * 0.5], 2.0, wd, "");
            rect(out, [-14.0, -t + 8.0, 28.0, 5.0], 1.0, "#2f3439", "");
            rect(out, [-11.0, -t + 18.0, 22.0, 6.0], 1.0, "#e8e6e1", r#"stroke-width="1""#);
            rect(out, [l - 26.0, -t + 10.0, 12.0, 14.0], 0.0, "#f4f1ea", r#"stroke-width="1" transform="rotate(10)""#);
            circle(out, 0.0, t - 18.0, 10.0, "#3d4247", "");
            let _ = write!(out, r#"<path d="M-9 {:.1}Q0 {:.1} 9 {:.1}" fill="none" stroke-width="3.2" stroke-linecap="round"/>"#, t - 11.0, t - 4.0, t - 11.0);
        }
        PropKind::Bookshelf | PropKind::SteelShelving => {
            let steel = p.kind == PropKind::SteelShelving;
            let body = if steel { "#7f878d" } else { pick(v, WOODS) };
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
            rect(out, [-l + 3.0, -t + 3.0, lw - 6.0, depth], 2.0, pick(v, WOODS), "");
            let _ = write!(out, r#"<path d="M0 {:.1}V{:.1}" {thin}/>"#, -t + 3.0, -t + 3.0 + depth);
            circle(out, -4.0, -t + depth - 6.0, 1.8, BATTLEMAP_INK, "");
            circle(out, 4.0, -t + depth - 6.0, 1.8, BATTLEMAP_INK, "");
        }
        PropKind::KitchenCounter => {
            let depth = lh * 0.62;
            rect(out, [-l + 1.0, -t + 2.0, lw - 2.0, depth], 1.5, "#d9d4ca", "");
            let _ = write!(out, r#"<path d="M{:.1} {:.1}H{:.1}" {thin}/>"#, -l + 1.0, -t + 7.0, l - 1.0);
            if p.w_cells.max(p.h_cells) >= 2 {
                rect(out, [-12.0, -t + 10.0, 24.0, depth - 14.0], 4.0, "#b7c1c7", "");
                circle(out, 0.0, -t + 8.0, 2.2, "#8b959b", r#"stroke-width="1""#);
            }
            circle(out, -l + 14.0, -t + depth * 0.55, 4.5, "#c9502f", r#"stroke-width="1""#);
        }
        PropKind::Stove => {
            let depth = lh * 0.62;
            rect(out, [-l + 2.0, -t + 2.0, lw - 4.0, depth], 2.0, "#3a3d41", "");
            for (x, y) in [(-12.0, -t + 12.0), (12.0, -t + 12.0), (-12.0, -t + depth - 9.0), (12.0, -t + depth - 9.0)] {
                circle(out, x, y, 6.5, "none", r##"stroke="#9aa0a6" stroke-width="1.6""##);
            }
        }
        PropKind::Fridge => {
            rect(out, [-l + 6.0, -t + 3.0, lw - 12.0, lh * 0.72], 3.0, "#e9ebec", "");
            let _ = write!(out, r#"<path d="M{:.1} {:.1}H{:.1}" {thin}/>"#, -l + 6.0, -t + lh * 0.3, l - 6.0);
            rect(out, [l - 14.0, -t + 10.0, 3.0, 12.0], 1.0, "#9aa0a6", r#"stroke-width=".8""#);
        }
        PropKind::BarCounter => {
            rect(out, [-l + 1.0, -t + 10.0, lw - 2.0, lh - 20.0], 3.0, "#5a3a24", "");
            rect(out, [-l + 1.0, t - 18.0, lw - 2.0, 8.0], 3.0, "#8a6a48", r#"stroke-width="1.4""#);
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
            rect(out, [-l + 1.0, -t + 2.0, lw - 2.0, depth], 1.0, "#3e2a1b", "");
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
            rect(out, [-l + 8.0, -t + 8.0, lw - 16.0, lh - 16.0], 8.0, "#5a3a24", "");
            rect(out, [-l + 18.0, -t + 18.0, lw - 36.0, lh - 36.0], 2.0, "#1f6b3e", r#"stroke-width="1.4""#);
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
            rect(out, [-l + 4.0, -t + 6.0, lw - 8.0, lh - 12.0], 14.0, "#f1f2f3", "");
            rect(out, [-l + 12.0, -t + 13.0, lw - 24.0, lh - 26.0], 12.0, "#cfe3ee", r#"stroke-width="1.2""#);
            circle(out, l - 22.0, 0.0, 2.5, "#8b959b", r#"stroke-width="1""#);
        }
        PropKind::Toilet => {
            rect(out, [-14.0, -t + 5.0, 28.0, 11.0], 3.0, "#f4f4f2", "");
            let _ = write!(out, r##"<ellipse cx="0" cy="{:.1}" rx="11" ry="14" fill="#f4f4f2"/><ellipse cx="0" cy="{:.1}" rx="6" ry="8" fill="#cfe3ee" stroke-width="1"/>"##, -t + 31.0, -t + 32.0);
        }
        PropKind::LargeTree => {
            let r = lw.min(lh) * 0.22;
            circle(out, 0.0, 0.0, r, "#6b4a2f", "");
            let canopy = battlemap_blob_path(0.0, 0.0, lw.max(lh) * 0.85, 11, 0.18, u64::from(v));
            let inner = battlemap_blob_path(-lw * 0.12, -lh * 0.12, lw.max(lh) * 0.5, 9, 0.22, u64::from(v) ^ 0x55);
            let _ = write!(out, r##"<path d="{canopy}" fill="#3f6b35" fill-opacity=".82"/><path d="{inner}" fill="#6f9b4f" fill-opacity=".7" stroke="none"/>"##);
        }
        PropKind::Car => {
            let body = pick(v, &["#4b6a8f", "#8f2f2f", "#d8d4cc", "#2f3337", "#6b7a5a"]);
            rect(out, [-l + 8.0, -t + 6.0, lw - 16.0, lh - 12.0], 22.0, body, "");
            rect(out, [-l + 16.0, -t + lh * 0.30, lw - 32.0, lh * 0.38], 10.0, "#000", r#"fill-opacity=".12""#);
            let _ = write!(
                out,
                r##"<path d="M{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}Z" fill="#2c3e50" fill-opacity=".85"/><path d="M{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}Z" fill="#2c3e50" fill-opacity=".85"/>"##,
                -l + 18.0, t - lh * 0.30, l - 18.0, t - lh * 0.30, l - 24.0, t - lh * 0.20, -l + 24.0, t - lh * 0.20,
                -l + 20.0, -t + lh * 0.30, l - 20.0, -t + lh * 0.30, l - 26.0, -t + lh * 0.22, -l + 26.0, -t + lh * 0.22,
            );
            circle(out, -l + 22.0, t - 12.0, 5.0, "#f6e7a1", r#"stroke-width="1""#);
            circle(out, l - 22.0, t - 12.0, 5.0, "#f6e7a1", r#"stroke-width="1""#);
            rect(out, [-l + 2.0, t - lh * 0.33, 7.0, 10.0], 2.0, body, r#"stroke-width="1.2""#);
            rect(out, [l - 9.0, t - lh * 0.33, 7.0, 10.0], 2.0, body, r#"stroke-width="1.2""#);
        }
        PropKind::Armchair | PropKind::Planter | PropKind::WoodCrate | PropKind::DrumCluster => {}
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
            let mut s = String::new();
            push_furniture_glyph(&mut s, &make_prop(kind, 2, 3, 90, Some(4), 11), 64.0);
            assert!(s.starts_with("<g ") && s.ends_with("</g>"), "{kind:?}");
            assert_eq!(s.matches("<g").count(), s.matches("</g>").count(), "{kind:?}");
            assert!(!s.contains("NaN"), "{kind:?}");
        }
    }
}
