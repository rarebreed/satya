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

use satya_engine::map_surface_geom::{
    extrude_surface_ribbon_m, point_masked_by_higher_surface, sample_open_catmull_rom_m,
    sampled_closed_ring_m,
};
use satya_schema::{MapSurfaceFeature, PropKind, SurfaceFeatureKind, SurfaceShapeKind};

use crate::wasm_client::map_2d_art::{
    BATTLEMAP_CELL_PX, BattlemapArtTheme, BattlemapFloorPattern, battlemap_boundary_path,
    battlemap_cell_rects_path,
};
use crate::wasm_client::map_2d_art_props::{
    BATTLEMAP_INK, battlemap_cell_hash, battlemap_unit_f32,
};
use crate::wasm_client::map_2d_art_textures::BattlemapTextureMode;
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

fn push_ring_d(out: &mut String, ring_m: &[(f32, f32)], cell_px: f32) {
    if let Some(&(x0, y0)) = ring_m.first() {
        let _ = write!(out, "M{:.1} {:.1}", x0 * cell_px, y0 * cell_px);
        for &(x, y) in &ring_m[1..] {
            let _ = write!(out, "L{:.1} {:.1}", x * cell_px, y * cell_px);
        }
        out.push('Z');
    }
}

fn emit_vector_surfaces_svg(
    out: &mut String,
    ui: &TacticalMapUiState,
    cell_px: f32,
    tex_mode: BattlemapTextureMode,
) {
    let painted = tex_mode.is_painted();
    let (bw_m, bh_m) = ui.map.board_size_m();
    let theme_id = ui.map.theme_preset.as_deref().unwrap_or_default();
    let indoor_theme = matches!(
        theme_id,
        "residential_apartment"
            | "office_floor"
            | "corner_bar"
            | "modern_interior"
            | "office"
            | "bar"
            | "bar_interior"
            | "docking_bay"
            | "med_lab"
            | "derelict_corridor"
            | "orbital_airlock"
            | "clinic"
            | "industrial"
            | "station"
    );

    let mut order: Vec<(usize, &MapSurfaceFeature)> = ui.map.surfaces.iter().enumerate().collect();
    order.sort_by_key(|(idx, s)| (s.kind.layer_priority(), *idx));

    // Pre-pass: paint concrete curbs beneath intersecting AsphaltRoads so crossroads merge seamlessly.
    let mut road_curb_d = String::new();
    for (_, s) in &order {
        if s.kind == SurfaceFeatureKind::AsphaltRoad {
            let ring = match s.shape {
                SurfaceShapeKind::RibbonSpline => extrude_surface_ribbon_m(s, bw_m, bh_m).to_closed_ring_m(),
                _ => sampled_closed_ring_m(s),
            };
            push_ring_d(&mut road_curb_d, &ring, cell_px);
        }
    }
    if !road_curb_d.is_empty() {
        let _ = write!(
            out,
            r##"<path d="{road_curb_d}" fill="none" stroke="#c5c0b5" stroke-width="6.5" stroke-linejoin="round"/><path d="{road_curb_d}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="2" stroke-linejoin="round" filter="url(#bm-ink)"/>"##
        );
    }

    for &(_, s) in &order {
        let ring = match s.shape {
            SurfaceShapeKind::RibbonSpline => extrude_surface_ribbon_m(s, bw_m, bh_m).to_closed_ring_m(),
            _ => sampled_closed_ring_m(s),
        };
        if ring.len() < 3 {
            continue;
        }
        let mut d = String::with_capacity(ring.len() * 14);
        push_ring_d(&mut d, &ring, cell_px);

        match s.kind {
            SurfaceFeatureKind::AsphaltRoad => {
                let _ = write!(out, r##"<path d="{d}" fill="url(#bm-tarmac)"/>"##);
            }
            SurfaceFeatureKind::ConcreteWalk => {
                let fill = if painted { "url(#bm-concrete)" } else { "#cac6bc" };
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="{fill}" stroke="#9e988c" stroke-width="2" stroke-linejoin="round"/>"##
                );
            }
            SurfaceFeatureKind::StoneWalk => {
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="none" stroke="#8b8273" stroke-width="3.6" stroke-linejoin="round" opacity=".55"/><path d="{d}" fill="url(#bm-paver)" stroke="{BATTLEMAP_INK}" stroke-width="1.4" stroke-linejoin="round" filter="url(#bm-ink)"/>"##
                );
            }
            SurfaceFeatureKind::GravelRoad => {
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="none" stroke="#938b7d" stroke-width="4.2" stroke-linejoin="round" opacity=".55"/><path d="{d}" fill="url(#bm-gravel)" stroke="#6a6358" stroke-width="1.2" stroke-linejoin="round" filter="url(#bm-ink)"/>"##
                );
            }
            SurfaceFeatureKind::DirtTrail => {
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="none" stroke="#7d6447" stroke-width="4.5" stroke-linejoin="round" opacity=".55"/><path d="{d}" fill="url(#bm-dirt)" stroke="#59452e" stroke-width="1.2" stroke-linejoin="round" filter="url(#bm-ink)"/>"##
                );
            }
            SurfaceFeatureKind::MulchZone => {
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="none" stroke="#442e1f" stroke-width="3.5" stroke-linejoin="round" opacity=".55"/><path d="{d}" fill="url(#bm-mulch)" stroke="#2f1e13" stroke-width="1.3" stroke-linejoin="round" filter="url(#bm-ink)"/>"##
                );
            }
            SurfaceFeatureKind::GardenBedZone => {
                let edge_attr = if s.border_style_cm > 0 {
                    r##" stroke="#a69a88" stroke-width="4.2" stroke-dasharray="10 3""##
                } else {
                    r##" stroke="#365425" stroke-width="1.4""##
                };
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="url(#bm-mulch)"/><path d="{d}" fill="url(#bm-leaf-tex)" fill-opacity=".86"{edge_attr} stroke-linejoin="round" filter="url(#bm-ink)"/>"##
                );
                let cx = ring.iter().map(|p| p.0).sum::<f32>() / ring.len() as f32;
                let cy = ring.iter().map(|p| p.1).sum::<f32>() / ring.len() as f32;
                let blooms = ["#e05770", "#f2c247", "#a875e0", "#f08552"];
                for (bi, pt) in ring.iter().step_by((ring.len() / 6).max(1)).take(6).enumerate() {
                    let fx = (cx + (pt.0 - cx) * 0.54) * cell_px;
                    let fy = (cy + (pt.1 - cy) * 0.54) * cell_px;
                    let col = blooms[(bi + s.variant_seed as usize) % blooms.len()];
                    let _ = write!(out, r##"<circle cx="{fx:.1}" cy="{fy:.1}" r="4.5" fill="{col}" stroke="{BATTLEMAP_INK}" stroke-width=".9"/>"##);
                }
            }
            SurfaceFeatureKind::CreekChannel => {
                if s.shape == SurfaceShapeKind::RibbonSpline {
                    let mut bank = s.clone();
                    bank.width_cm += 90;
                    let bank_ring = extrude_surface_ribbon_m(&bank, bw_m, bh_m).to_closed_ring_m();
                    let mut bd = String::with_capacity(bank_ring.len() * 14);
                    push_ring_d(&mut bd, &bank_ring, cell_px);
                    let _ = write!(
                        out,
                        r##"<path d="{bd}" fill="none" stroke="#384736" stroke-width="4.5" stroke-linejoin="round" opacity=".48"/><path d="{bd}" fill="url(#bm-gravel)" stroke="#7d7563" stroke-width="1.4" stroke-linejoin="round"/>"##
                    );
                }
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="url(#bm-creek)" stroke="#2a5d73" stroke-width="1.8" stroke-linejoin="round" filter="url(#bm-ink)"/>"##
                );
                if s.shape == SurfaceShapeKind::RibbonSpline {
                    let ctrl_m: Vec<(f32, f32)> = s.points_cm.iter().map(|p| p.to_m()).collect();
                    let cl = sample_open_catmull_rom_m(&ctrl_m, 12);
                    if cl.len() >= 2 {
                        let _ = write!(out, r##"<path d="M{:.1} {:.1}"##, cl[0].0 * cell_px, cl[0].1 * cell_px);
                        for pt in &cl[1..] {
                            let _ = write!(out, "L{:.1} {:.1}", pt.0 * cell_px, pt.1 * cell_px);
                        }
                        out.push_str(r##"" fill="none" stroke="#c7eff9" stroke-width="2" stroke-dasharray="14 10" stroke-linecap="round" opacity=".55"/>"##);
                    }
                }
            }
            SurfaceFeatureKind::DrainageDitch | SurfaceFeatureKind::DefensiveTrench => {
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="none" stroke="#3c4a39" stroke-width="6" stroke-linejoin="round" opacity=".55"/><path d="{d}" fill="url(#bm-ditch)" stroke="#242e23" stroke-width="1.6" stroke-linejoin="round" filter="url(#bm-ink)"/>"##
                );
                if s.shape == SurfaceShapeKind::RibbonSpline {
                    let mesh = extrude_surface_ribbon_m(s, bw_m, bh_m);
                    let min_prio = s.kind.layer_priority() + 1;
                    let mut hd = String::new();
                    for (ci, edge) in [&mesh.left, &mesh.right].iter().enumerate() {
                        let sign = if ci == 0 { -1.0 } else { 1.0 };
                        for i in (1..edge.len()).step_by(3) {
                            let p = edge[i];
                            let prev = edge[i - 1];
                            if point_masked_by_higher_surface(&ui.map.surfaces, min_prio, p.0, p.1) {
                                continue;
                            }
                            let dx = p.0 - prev.0;
                            let dy = p.1 - prev.1;
                            let len = dx.hypot(dy).max(1e-4);
                            let (nx, ny) = ((-dy / len) * sign * 0.22, (dx / len) * sign * 0.22);
                            let _ = write!(
                                hd,
                                "M{:.1} {:.1}L{:.1} {:.1}",
                                p.0 * cell_px,
                                p.1 * cell_px,
                                (p.0 + nx) * cell_px,
                                (p.1 + ny) * cell_px
                            );
                        }
                    }
                    if !hd.is_empty() {
                        let _ = write!(out, r##"<path d="{hd}" fill="none" stroke="#3b2f22" stroke-width="1.4" opacity=".65"/>"##);
                    }
                }
            }
            SurfaceFeatureKind::InteriorFloor => {
                if !indoor_theme {
                    let fill = if painted { "url(#bm-hardwood)" } else { "#9b7854" };
                    let _ = write!(
                        out,
                        r##"<path d="{d}" fill="{fill}" stroke="#664b30" stroke-width="1.5" stroke-linejoin="round"/>"##
                    );
                }
            }
            SurfaceFeatureKind::BridgeDeck => {
                let fill = if painted { "url(#bm-oak)" } else { "#8c623d" };
                let _ = write!(
                    out,
                    r##"<path d="{d}" fill="{fill}" stroke="{BATTLEMAP_INK}" stroke-width="2" stroke-linejoin="round" filter="url(#bm-prop)"/>"##
                );
                if s.points_cm.len() == 4 {
                    let p = [
                        s.points_cm[0].to_m(),
                        s.points_cm[1].to_m(),
                        s.points_cm[2].to_m(),
                        s.points_cm[3].to_m(),
                    ];
                    let planks = 7;
                    let mut pd = String::new();
                    for k in 1..planks {
                        let t = k as f32 / planks as f32;
                        let ax = p[0].0 + (p[1].0 - p[0].0) * t;
                        let ay = p[0].1 + (p[1].1 - p[0].1) * t;
                        let bx = p[3].0 + (p[2].0 - p[3].0) * t;
                        let by = p[3].1 + (p[2].1 - p[3].1) * t;
                        let _ = write!(pd, "M{:.1} {:.1}L{:.1} {:.1}", ax * cell_px, ay * cell_px, bx * cell_px, by * cell_px);
                    }
                    let _ = write!(
                        out,
                        r##"<path d="{pd}" fill="none" stroke="#5c3e24" stroke-width="1.3"/><path d="M{:.1} {:.1}L{:.1} {:.1}M{:.1} {:.1}L{:.1} {:.1}" fill="none" stroke="#3f2713" stroke-width="4" stroke-linecap="round"/>"##,
                        p[0].0 * cell_px, p[0].1 * cell_px, p[1].0 * cell_px, p[1].1 * cell_px,
                        p[2].0 * cell_px, p[2].1 * cell_px, p[3].0 * cell_px, p[3].1 * cell_px
                    );
                }
            }
        }
    }
}

/// Renders all floor-layer ground surfaces with vector curves/polygons when present, or legacy Chaikin contours.
pub(crate) fn push_landscaping_surfaces_svg(
    out: &mut String,
    ui: &TacticalMapUiState,
    w: i32,
    h: i32,
    seed: i64,
    tex_mode: BattlemapTextureMode,
) {
    if !ui.map.surfaces.is_empty() {
        emit_vector_surfaces_svg(out, ui, BATTLEMAP_CELL_PX as f32, tex_mode);
        return;
    }

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

/// Emits floor stain blobs (strictly masked off roads/pavement/water) and curved/legacy road lane markings.
pub(crate) fn push_ground_dressing_and_lanes_svg(
    out: &mut String,
    ui: &TacticalMapUiState,
    t: &BattlemapArtTheme,
    w: i32,
    h: i32,
    seed: i64,
) {
    let cell_f = BATTLEMAP_CELL_PX as f32;
    let (vw, vh) = (w as f32 * cell_f, h as f32 * cell_f);
    let has_vector_surfaces = !ui.map.surfaces.is_empty();

    if t.floor_stains {
        for i in 0..((w * h) / 16).max(3) {
            let hh = battlemap_cell_hash(seed, i, 0, 0x57A1);
            let sx = battlemap_unit_f32(hh) * vw;
            let sy = battlemap_unit_f32(hh ^ 0x3C) * vh;
            let r = cell_f * (0.4 + battlemap_unit_f32(hh ^ 0x91) * 0.8);
            let (xm, ym, rm) = (sx / cell_f, sy / cell_f, (r / cell_f) * 0.55);
            let blocked_by_surface = has_vector_surfaces
                && [(0.0, 0.0), (rm, 0.0), (-rm, 0.0), (0.0, rm), (0.0, -rm)]
                    .into_iter()
                    .any(|(dx, dy)| {
                        satya_engine::map_surface_geom::topmost_surface_at_m(
                            &ui.map.surfaces,
                            xm + dx,
                            ym + dy,
                        )
                        .is_some_and(|s| s.kind.is_road_or_pavement() || s.kind.is_water_or_channel())
                    });
            if blocked_by_surface {
                continue;
            }
            let d = crate::wasm_client::map_2d_art_props::battlemap_blob_path(sx, sy, r, 8, 0.3, hh);
            let _ = write!(out, r##"<path d="{d}" fill="#5b4a36" fill-opacity=".14"/>"##);
        }
    }

    if t.lane_markings {
        let mut d = String::new();
        if has_vector_surfaces {
            let roads: Vec<&MapSurfaceFeature> = ui
                .map
                .surfaces
                .iter()
                .filter(|s| {
                    s.kind == SurfaceFeatureKind::AsphaltRoad
                        && s.shape == SurfaceShapeKind::RibbonSpline
                        && s.width_cm >= 320
                        && s.points_cm.len() >= 2
                })
                .collect();
            for (r_idx, road) in roads.iter().enumerate() {
                let ctrl_m: Vec<(f32, f32)> = road.points_cm.iter().map(|p| p.to_m()).collect();
                let line = sample_open_catmull_rom_m(&ctrl_m, 14);
                let min_prio = road.kind.layer_priority() + 1;
                let mut pen_down = false;
                for pt in line {
                    let masked = point_masked_by_higher_surface(&ui.map.surfaces, min_prio, pt.0, pt.1)
                        || roads[..r_idx]
                            .iter()
                            .any(|prev| satya_engine::map_surface_geom::surface_contains_point_m(prev, pt.0, pt.1));
                    if masked {
                        pen_down = false;
                        continue;
                    }
                    let (px, py) = (pt.0 * cell_f, pt.1 * cell_f);
                    if pen_down {
                        let _ = write!(d, "L{px:.1} {py:.1}");
                    } else {
                        let _ = write!(d, "M{px:.1} {py:.1}");
                        pen_down = true;
                    }
                }
            }
        } else {
            let on_road = |x: i32, y: i32| {
                ui.map
                    .props
                    .iter()
                    .any(|p| p.kind == PropKind::AsphaltDrive && p.covers(x, y))
            };
            let (my, mx) = (h / 2, w / 2);
            for x in 0..w {
                if on_road(x, my - 1) && on_road(x, my) {
                    let _ = write!(d, "M{} {}h{}", x * BATTLEMAP_CELL_PX, my * BATTLEMAP_CELL_PX, BATTLEMAP_CELL_PX);
                }
            }
            for y in 0..h {
                if on_road(mx - 1, y) && on_road(mx, y) && !(y == my - 1 || y == my) {
                    let _ = write!(d, "M{} {}v{}", mx * BATTLEMAP_CELL_PX, y * BATTLEMAP_CELL_PX, BATTLEMAP_CELL_PX);
                }
            }
        }
        if !d.is_empty() {
            let _ = write!(
                out,
                r##"<path d="{d}" fill="none" stroke="#efe9d2" stroke-width="3.5" stroke-dasharray="20 14" stroke-opacity=".8"/>"##
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::map_2d_art::{battlemap_art_cache_key, format_battlemap_art_layer};
    use satya_engine::tactical_map::{SectorPreset, build_procedural_map_request};
    use satya_schema::MapPointCm;

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

    #[test]
    fn vector_surfaces_svg_cache_bridges_and_road_stain_protection() {
        let mut ui = TacticalMapUiState {
            map: build_procedural_map_request("v2d", SectorPreset::RoadsideDitch, None, 20, 14, 42),
            ..Default::default()
        };
        let art = format_battlemap_art_layer(&ui);
        assert!(art.contains("url(#bm-tarmac)"));
        assert!(art.contains("url(#bm-ditch)"));
        assert!(art.contains(r##"stroke="#3f2713""##), "culvert BridgeDeck rails must render");
        assert!(!art.contains("NaN"));

        let key_before = battlemap_art_cache_key(&ui);
        ui.map.surfaces.push(MapSurfaceFeature {
            kind: SurfaceFeatureKind::MulchZone,
            shape: SurfaceShapeKind::OrganicClosed,
            points_cm: vec![
                MapPointCm::new(200, 200),
                MapPointCm::new(350, 200),
                MapPointCm::new(350, 350),
                MapPointCm::new(200, 350),
            ],
            width_cm: 0,
            border_style_cm: 18,
            variant_seed: 9,
        });
        assert_ne!(key_before, battlemap_art_cache_key(&ui), "surface edits bust art cache");

        // Full-board AsphaltRoad protects every stain sample under urban_crossroads theme.
        let mut road_ui = TacticalMapUiState {
            map: build_procedural_map_request("road", SectorPreset::UrbanCrossroads, None, 12, 10, 42),
            ..Default::default()
        };
        road_ui.map.surfaces = vec![MapSurfaceFeature {
            kind: SurfaceFeatureKind::AsphaltRoad,
            shape: SurfaceShapeKind::Polygon,
            points_cm: vec![
                MapPointCm::new(0, 0),
                MapPointCm::new(1200, 0),
                MapPointCm::new(1200, 1000),
                MapPointCm::new(0, 1000),
            ],
            width_cm: 0,
            border_style_cm: 0,
            variant_seed: 1,
        }];
        let road_art = format_battlemap_art_layer(&road_ui);
        assert!(!road_art.contains(r##"fill="#5b4a36""##), "dirt stains must never appear on AsphaltRoad");
    }

    #[test]
    fn woodland_creek_svg_paints_creek_water_above_ravine_and_gravel_shoals() {
        let ui = TacticalMapUiState {
            map: build_procedural_map_request("creek2d", SectorPreset::WoodlandCreek, None, 36, 26, 42),
            ..Default::default()
        };
        let art = format_battlemap_art_layer(&ui);
        assert!(art.contains("url(#bm-creek)"), "WoodlandCreek SVG missing creek water fill");
        assert!(art.contains("url(#bm-ditch)"), "WoodlandCreek SVG missing ravine hachured trench fill");
        let last_creek = art.rfind("url(#bm-creek)").unwrap();
        let first_ravine = art.find("url(#bm-ditch)").unwrap();
        assert!(last_creek > first_ravine, "creek water must draw after ravine banks");
        if let Some(last_gravel) = art.rfind("url(#bm-gravel)") {
            assert!(last_creek > last_gravel, "creek water must draw above gravel shoals");
        }
    }
}
