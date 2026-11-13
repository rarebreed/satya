//! # Illustrated 2D Battlemap Ground Textures & Meandering Surfaces (`map_2d_art_surfaces`)
//!
//! Companion module to [`crate::wasm_client::map_2d_art`] providing:
//! - Rich SVG `<pattern>` floor definitions (`Grass`, `Hardwood`, `PolishedConcrete`, `StainedConcrete`,
//!   `VinylTile`, `Asphalt`) plus landscaping textures (`#bm-gravel`, `#bm-dirt`, `#bm-mulch`, `#bm-creek`,
//!   `#bm-ditch`, and multi-tone foliage leaf canopy `#bm-leaf-tex`).
//! - Closed grid-contour extraction + Chaikin corner smoothing ([`battlemap_smooth_region_path`]) so
//!   creeks, ditches, gravel bars, dirt trails, mulch pockets, and flagstone walkways flow naturally
//!   instead of rendering with hard `90°` stair-stepped corners.

use std::fmt::Write as _;

use satya_schema::PropKind;

use crate::wasm_client::map_2d_art::{
    BATTLEMAP_CELL_PX, BattlemapArtTheme, BattlemapFloorPattern, battlemap_boundary_path,
    battlemap_cell_rects_path,
};
use crate::wasm_client::map_2d_art_props::{
    BATTLEMAP_INK, battlemap_cell_hash, battlemap_unit_f32,
};
use crate::wasm_client::map_3d::TacticalMapUiState;

/// Emits the primary `#bm-floor` `<pattern>` matching `t.floor`.
pub(crate) fn push_floor_pattern_def(out: &mut String, t: &BattlemapArtTheme) {
    let (base, line) = (t.floor_base, t.floor_line);
    match t.floor {
        BattlemapFloorPattern::Hardwood => {
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="128" height="64" patternUnits="userSpaceOnUse"><rect width="128" height="64" fill="{base}"/><rect y="16" width="128" height="16" fill="#000" opacity=".045"/><rect y="48" width="128" height="16" fill="#fff" opacity=".06"/><path d="M0 16H128M0 32H128M0 48H128M0 64H128M40 0V16M104 16V32M18 32V48M78 48V64" stroke="{line}" stroke-width="1.2" opacity=".55"/></pattern>"##
            );
        }
        BattlemapFloorPattern::PolishedConcrete | BattlemapFloorPattern::StainedConcrete => {
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="256" height="256" patternUnits="userSpaceOnUse"><rect width="256" height="256" fill="{base}"/><path d="M0 0H256M0 0V256" stroke="{line}" stroke-width="1.6" opacity=".6"/><circle cx="64" cy="190" r="1.4" fill="{line}" opacity=".5"/><circle cx="180" cy="70" r="1.1" fill="{line}" opacity=".5"/></pattern>"##
            );
        }
        BattlemapFloorPattern::VinylTile => {
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="128" height="128" patternUnits="userSpaceOnUse"><rect width="128" height="128" fill="{base}"/><rect x="32" y="0" width="32" height="32" fill="#000" opacity=".035"/><rect x="96" y="64" width="32" height="32" fill="#000" opacity=".035"/><rect x="0" y="96" width="32" height="32" fill="#9fb8b0" opacity=".18"/><path d="M0 0H128M0 32H128M0 64H128M0 96H128M0 0V128M32 0V128M64 0V128M96 0V128" stroke="{line}" stroke-width="1" opacity=".8"/></pattern>"##
            );
        }
        BattlemapFloorPattern::Asphalt => {
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="{base}"/><circle cx="7" cy="9" r="1" fill="#fff" opacity=".12"/><circle cx="31" cy="22" r="1.2" fill="#fff" opacity=".1"/><circle cx="18" cy="40" r=".9" fill="#000" opacity=".25"/><circle cx="42" cy="4" r="1" fill="{line}" opacity=".6"/></pattern>"##
            );
        }
        BattlemapFloorPattern::Grass => {
            // Multi-tone sunlit meadow turf with clover patches, soil depth flecks, and blade tufts
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="128" height="128" patternUnits="userSpaceOnUse"><rect width="128" height="128" fill="{base}"/><ellipse cx="28" cy="34" rx="22" ry="14" fill="#89a85d" opacity=".28"/><ellipse cx="92" cy="88" rx="26" ry="16" fill="#638042" opacity=".25"/><ellipse cx="98" cy="24" rx="16" ry="10" fill="#95b468" opacity=".22"/><ellipse cx="34" cy="102" rx="18" ry="12" fill="#5e793e" opacity=".22"/><path d="M12 20l2-7m2 7l4-5M46 42l-2-7m2 7l3-6m29-14l2-7m0 7l4-5M108 54l-2-6m2 6l3-5M22 86l2-7m0 7l4-4M64 70l-2-7m2 7l3-6m24 38l2-7m0 7l4-5M114 114l-2-6m2 6l3-5" stroke="{line}" stroke-width="1.35" stroke-linecap="round" opacity=".6"/><path d="M36 16l2-5M74 52l2-5M16 112l2-5M88 68l-2-5" stroke="#a6c478" stroke-width="1.1" stroke-linecap="round" opacity=".55"/><circle cx="54" cy="18" r="1.4" fill="#9fc06d" opacity=".55"/><circle cx="18" cy="58" r="1.2" fill="#4f6734" opacity=".45"/><circle cx="106" cy="74" r="1.5" fill="#a2be72" opacity=".5"/><circle cx="62" cy="110" r="1.3" fill="#4b6232" opacity=".45"/></pattern>"##
            );
        }
    }
}

/// Emits natural landscaping surface patterns (`#bm-gravel`, `#bm-dirt`, `#bm-mulch`, `#bm-creek`,
/// `#bm-ditch`) and layered leaf canopy pattern (`#bm-leaf-tex`).
pub(crate) fn push_surface_pattern_defs(out: &mut String) {
    out.push_str(
        r##"<pattern id="bm-gravel" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#b7afa2"/><ellipse cx="10" cy="12" rx="4.2" ry="3.1" fill="#ccc5b8" stroke="#857d70" stroke-width=".6" opacity=".8"/><ellipse cx="26" cy="8" rx="3.5" ry="2.8" fill="#9f9789" stroke="#766f63" stroke-width=".6" opacity=".8"/><ellipse cx="44" cy="15" rx="4.5" ry="3.2" fill="#d3ccbf" opacity=".75"/><ellipse cx="56" cy="30" rx="3.2" ry="4" fill="#9a9284" opacity=".8"/><ellipse cx="16" cy="34" rx="4.8" ry="3.4" fill="#cac2b4" stroke="#857d70" stroke-width=".6" opacity=".8"/><ellipse cx="36" cy="36" rx="3.4" ry="2.6" fill="#dcd6ca" opacity=".7"/><ellipse cx="12" cy="52" rx="3.6" ry="3.2" fill="#9e9688" opacity=".8"/><ellipse cx="30" cy="54" rx="4.4" ry="2.9" fill="#cbc3b5" stroke="#857d70" stroke-width=".6" opacity=".8"/><ellipse cx="50" cy="50" rx="3.8" ry="3.5" fill="#a59d8f" opacity=".8"/><circle cx="20" cy="22" r="1.4" fill="#70695d" opacity=".6"/><circle cx="52" cy="8" r="1.3" fill="#e5dfd3" opacity=".7"/><circle cx="42" cy="46" r="1.4" fill="#6f685c" opacity=".55"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-dirt" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#9e8260"/><ellipse cx="22" cy="20" rx="14" ry="7" fill="#8b6f4e" opacity=".45"/><ellipse cx="48" cy="46" rx="12" ry="6" fill="#ae926f" opacity=".4"/><path d="M4 14q14 3 28-1M24 40q16-2 32 2M8 56q12 2 22-2" stroke="#745b3e" stroke-width="1.2" fill="none" opacity=".35"/><circle cx="14" cy="30" r="1.5" fill="#6a5237" opacity=".55"/><circle cx="38" cy="12" r="1.2" fill="#c3a784" opacity=".6"/><circle cx="54" cy="28" r="1.4" fill="#6a5237" opacity=".5"/><circle cx="26" cy="52" r="1.3" fill="#c6aa88" opacity=".55"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-mulch" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#5c402c"/><rect x="6" y="8" width="9" height="3.2" rx="1.2" transform="rotate(18 10 9)" fill="#78543b"/><rect x="26" y="10" width="8" height="3" rx="1" transform="rotate(-28 30 11)" fill="#432c1d"/><rect x="44" y="6" width="10" height="3.4" rx="1.2" transform="rotate(34 49 7)" fill="#6f4d35"/><rect x="14" y="28" width="10" height="3.2" rx="1" transform="rotate(-14 19 29)" fill="#493121"/><rect x="36" y="26" width="8.5" height="3" rx="1.2" transform="rotate(24 40 27)" fill="#825d42"/><rect x="50" y="34" width="9" height="3.2" rx="1" transform="rotate(-38 54 35)" fill="#452e1e"/><rect x="8" y="48" width="10" height="3.2" rx="1.2" transform="rotate(12 13 49)" fill="#7a563d"/><rect x="28" y="46" width="9" height="3" rx="1" transform="rotate(-22 32 47)" fill="#3d281a"/><rect x="46" y="52" width="9.5" height="3.2" rx="1.2" transform="rotate(28 50 53)" fill="#6d4b34"/><circle cx="22" cy="40" r="1.1" fill="#2e1d12" opacity=".6"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-creek" width="96" height="96" patternUnits="userSpaceOnUse"><rect width="96" height="96" fill="#4689a3"/><ellipse cx="22" cy="24" rx="4.5" ry="3.2" fill="#637b7f" opacity=".45"/><ellipse cx="70" cy="68" rx="5.2" ry="3.6" fill="#5e7478" opacity=".4"/><ellipse cx="52" cy="18" rx="3.4" ry="2.6" fill="#778c8f" opacity=".38"/><path d="M6 18q18-6 36 0t36 0M14 48q20 6 40 0t34 0M4 78q18-5 36 0t40 0" fill="none" stroke="#8ed1e4" stroke-width="1.8" stroke-linecap="round" opacity=".55"/><path d="M22 32q12-3 24 0M56 60q14 3 28 0" fill="none" stroke="#bfeaf5" stroke-width="1.3" stroke-linecap="round" opacity=".65"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-ditch" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#546854"/><rect width="64" height="10" fill="#414b3c" opacity=".45"/><rect y="54" width="64" height="10" fill="#414b3c" opacity=".45"/><path d="M6 26q14-3 28 0t24 0M10 42q16 3 32 0t18 0" fill="none" stroke="#7d957c" stroke-width="1.4" stroke-linecap="round" opacity=".5"/><ellipse cx="44" cy="32" rx="4" ry="2" fill="#6d7b5e" opacity=".45"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-leaf-tex" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#4b7b37"/><ellipse cx="10" cy="10" rx="7" ry="4.5" transform="rotate(24 10 10)" fill="#5f9443" opacity=".75"/><ellipse cx="28" cy="12" rx="6.5" ry="4" transform="rotate(-18 28 12)" fill="#3b652a" opacity=".75"/><ellipse cx="40" cy="24" rx="7" ry="4.5" transform="rotate(42 40 24)" fill="#75ab4f" opacity=".65"/><ellipse cx="16" cy="28" rx="7.5" ry="4.5" transform="rotate(-32 16 28)" fill="#6a9e49" opacity=".72"/><ellipse cx="30" cy="36" rx="7" ry="4.5" transform="rotate(14 30 36)" fill="#3d692b" opacity=".75"/><ellipse cx="8" cy="42" rx="6" ry="3.8" transform="rotate(-12 8 42)" fill="#84ba59" opacity=".6"/><ellipse cx="42" cy="44" rx="5.5" ry="3.8" transform="rotate(28 42 44)" fill="#5c8e40" opacity=".7"/></pattern>"##,
    );
}

fn push_chaikin_loop(out: &mut String, ring: &[(f32, f32)], passes: usize) {
    if ring.len() < 3 {
        return;
    }
    let mut pts = ring.to_vec();
    for _ in 0..passes {
        let mut next = Vec::with_capacity(pts.len() * 2);
        let n = pts.len();
        for i in 0..n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            next.push((a.0 * 0.75 + b.0 * 0.25, a.1 * 0.75 + b.1 * 0.25));
            next.push((a.0 * 0.25 + b.0 * 0.75, a.1 * 0.25 + b.1 * 0.75));
        }
        pts = next;
    }
    if let Some(&(x0, y0)) = pts.first() {
        let _ = write!(out, "M{x0:.1} {y0:.1}");
        for &(x, y) in &pts[1..] {
            let _ = write!(out, "L{x:.1} {y:.1}");
        }
        out.push('Z');
    }
}

/// Extracts closed contour loops around every cell where `inside(x, y)` is true, adds deterministic
/// interior vertex wobble (`wobble_px`), and smooths corners with two Chaikin corner-cutting passes.
#[must_use]
pub fn battlemap_smooth_region_path(
    w: i32,
    h: i32,
    inside: impl Fn(i32, i32) -> bool,
    seed: i64,
    wobble_px: f32,
) -> String {
    let at = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && inside(x, y);
    let mut edges: Vec<((i32, i32), (i32, i32))> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if !at(x, y) {
                continue;
            }
            if !at(x, y - 1) {
                edges.push(((x, y), (x + 1, y)));
            }
            if !at(x + 1, y) {
                edges.push(((x + 1, y), (x + 1, y + 1)));
            }
            if !at(x, y + 1) {
                edges.push(((x + 1, y + 1), (x, y + 1)));
            }
            if !at(x - 1, y) {
                edges.push(((x, y + 1), (x, y)));
            }
        }
    }
    if edges.is_empty() {
        return String::new();
    }

    let c = BATTLEMAP_CELL_PX as f32;
    let mut used = vec![false; edges.len()];
    let mut path = String::new();

    for start_idx in 0..edges.len() {
        if used[start_idx] {
            continue;
        }
        used[start_idx] = true;
        let start_v = edges[start_idx].0;
        let mut prev_v = edges[start_idx].0;
        let mut cur_v = edges[start_idx].1;
        let mut loop_gv = vec![start_v];

        while cur_v != start_v {
            loop_gv.push(cur_v);
            let in_dx = cur_v.0 - prev_v.0;
            let in_dy = cur_v.1 - prev_v.1;
            // Pick sharpest right-hand turn when two diagonal regions touch at a corner vertex
            let mut best: Option<(usize, i32)> = None;
            for (ei, &(e_from, e_to)) in edges.iter().enumerate() {
                if used[ei] || e_from != cur_v {
                    continue;
                }
                let out_dx = e_to.0 - e_from.0;
                let out_dy = e_to.1 - e_from.1;
                let cross = in_dx * out_dy - in_dy * out_dx;
                let dot = in_dx * out_dx + in_dy * out_dy;
                let score = if cross > 0 { 0 } else if dot > 0 { 1 } else { 2 };
                if best.is_none_or(|(_, bs)| score < bs) {
                    best = Some((ei, score));
                }
            }
            let Some((next_ei, _)) = best else {
                break;
            };
            used[next_ei] = true;
            prev_v = cur_v;
            cur_v = edges[next_ei].1;
            if loop_gv.len() > edges.len() + 2 {
                break;
            }
        }

        let ring: Vec<(f32, f32)> = loop_gv
            .into_iter()
            .map(|(gx, gy)| {
                let on_border = gx == 0 || gy == 0 || gx == w || gy == h;
                if on_border || wobble_px <= 0.01 {
                    (gx as f32 * c, gy as f32 * c)
                } else {
                    let hv = battlemap_cell_hash(seed, gx, gy, 0xC4A1);
                    let dx = (battlemap_unit_f32(hv) - 0.5) * 2.0 * wobble_px;
                    let dy = (battlemap_unit_f32(hv ^ 0x5A) - 0.5) * 2.0 * wobble_px;
                    (gx as f32 * c + dx, gy as f32 * c + dy)
                }
            })
            .collect();
        push_chaikin_loop(&mut path, &ring, 2);
    }
    path
}

/// Renders all floor-layer ground surfaces (`AsphaltDrive`, `DirtPath`, `GravelPath`, `MulchBed`,
/// `DrainageDitch`, `CreekWater`, `StonePath`) with organic Chaikin shoreline contours and patterns.
pub(crate) fn push_landscaping_surfaces_svg(
    out: &mut String,
    ui: &TacticalMapUiState,
    w: i32,
    h: i32,
    seed: i64,
) {
    let has_surf = |kind: PropKind, x: i32, y: i32| {
        ui.map
            .props
            .iter()
            .any(|p| p.kind == kind && p.rotation_deg.rem_euclid(90) == 0 && p.covers(x, y))
    };

    let tarmac_d = battlemap_cell_rects_path(w, h, |x, y| has_surf(PropKind::AsphaltDrive, x, y));
    if !tarmac_d.is_empty() {
        let curb_d = battlemap_boundary_path(w, h, |x, y| has_surf(PropKind::AsphaltDrive, x, y));
        let _ = write!(
            out,
            r##"<path d="{tarmac_d}" fill="url(#bm-tarmac)"/><path d="{curb_d}" fill="none" stroke="#c5c0b5" stroke-width="6"/><path d="{curb_d}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.8" filter="url(#bm-ink)"/>"##
        );
    }

    for (kind, pat, bank_col, bank_w, stroke_col, stroke_w, wobble, salt) in [
        (PropKind::DirtPath, "#bm-dirt", "#7d6447", 4.5_f32, "#59452e", 1.2_f32, 6.5_f32, 11_i64),
        (PropKind::GravelPath, "#bm-gravel", "#938b7d", 4.2, "#6a6358", 1.2, 5.5, 23),
        (PropKind::MulchBed, "#bm-mulch", "#442e1f", 3.5, "#2f1e13", 1.2, 6.5, 37),
        (PropKind::DrainageDitch, "#bm-ditch", "#3c4a39", 6.0, "#242e23", 1.5, 5.0, 53),
        (PropKind::CreekWater, "#bm-creek", "#8b836f", 8.5, "#2a5d73", 1.7, 7.5, 71),
        (PropKind::StonePath, "#bm-paver", "#8b8273", 3.4, BATTLEMAP_INK, 1.4, 4.2, 89),
    ] {
        let d = battlemap_smooth_region_path(w, h, |x, y| has_surf(kind, x, y), seed ^ salt, wobble);
        if d.is_empty() {
            continue;
        }
        let _ = write!(
            out,
            r##"<path d="{d}" fill="none" stroke="{bank_col}" stroke-width="{bank_w}" stroke-linejoin="round" opacity=".55"/><path d="{d}" fill="url({pat})" fill-rule="evenodd" stroke="{stroke_col}" stroke-width="{stroke_w}" stroke-linejoin="round" filter="url(#bm-ink)"/>"##
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_region_traces_diagonal_band_and_holes_cleanly() {
        let d = battlemap_smooth_region_path(
            6,
            6,
            |x, y| (x - y).abs() <= 1 && !(x == 2 && y == 2),
            42,
            5.0,
        );
        assert!(d.starts_with('M'));
        assert!(d.matches('Z').count() >= 2, "outer band plus interior island hole");
        assert!(!d.contains("NaN"));
    }
}
