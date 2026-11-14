//! # Edge-Seam Ribbon Matching & Alignment (`map_continuation_ribbons`)
//!
//! Companion to [`crate::map_continuation`] that extracts [`MapSurfaceFeature`] spline endpoints
//! and legacy ribbon props touching an exited seam, mirrors generated chunks along the seam axis
//! when doing so reduces offset, and stitches smooth bridge `RibbonSpline` curves across seams.

use satya_schema::{
    GmMapUpdateRequest, MapContinuationDir, MapFloorLevel, MapPointCm, MapPropPlacement,
    MapSurfaceFeature, PropKind, SurfaceFeatureKind, SurfaceShapeKind,
};

use crate::map_continuation::{mirror_edge_axis, mirror_prop_axis, translate_prop, upsert_edge};
use crate::map_dressing::{make_prop_at_cm, prop_spec};
use crate::map_surface_geom::surface_contains_point_m;
use crate::tactical_map::{ProceduralSector, SectorPreset};
use crate::tactical_map_geom::prop_intersects_circle;

pub(crate) const fn is_continuous_ribbon_prop(kind: PropKind) -> bool {
    matches!(
        kind,
        PropKind::CreekWater
            | PropKind::DrainageDitch
            | PropKind::AsphaltDrive
            | PropKind::GravelPath
            | PropKind::StonePath
            | PropKind::DirtPath
            | PropKind::ElevatedCatwalk
    )
}

#[derive(Debug, Clone)]
pub(crate) enum RibbonTargetKind {
    Surface(SurfaceFeatureKind),
    LegacyProp(PropKind),
}

#[derive(Debug, Clone)]
pub(crate) struct RibbonAnchor {
    pub(crate) target: RibbonTargetKind,
    pub(crate) local_t_m: f32,
    pub(crate) span_cm: i32,
    pub(crate) variant_seed: u32,
}

#[inline]
fn point_seam_coords(
    pt: (f32, f32),
    dir: MapContinuationDir,
    w: i32,
    h: i32,
) -> (f32, f32) {
    match dir {
        MapContinuationDir::North => (pt.1, pt.0),
        MapContinuationDir::South => (h as f32 - pt.1, pt.0),
        MapContinuationDir::West => (pt.0, pt.1),
        MapContinuationDir::East => (w as f32 - pt.0, pt.1),
    }
}

/// Collects continuous surface splines and legacy ribbon props exiting `map` along `dir` within `[t0..t0 + t_len]`.
pub(crate) fn extract_seam_ribbons(
    map: &GmMapUpdateRequest,
    dir: MapContinuationDir,
    t0: i32,
    t_len: i32,
) -> Vec<RibbonAnchor> {
    let (w, h) = (map.grid_width, map.grid_height);
    let mut ribbons = Vec::new();

    for s in &map.surfaces {
        if s.shape != SurfaceShapeKind::RibbonSpline || s.points_cm.len() < 2 {
            continue;
        }
        let endpoints = [s.points_cm[0].to_m(), s.points_cm[s.points_cm.len() - 1].to_m()];
        for pt in endpoints {
            let (norm_dist, tang_coord) = point_seam_coords(pt, dir, w, h);
            if norm_dist <= 1.50 && tang_coord >= t0 as f32 - 0.5 && tang_coord <= (t0 + t_len) as f32 + 0.5 {
                let local_t_m = (tang_coord - t0 as f32).clamp(0.5, t_len as f32 - 0.5);
                let dup = ribbons.iter().any(|r: &RibbonAnchor| {
                    matches!(r.target, RibbonTargetKind::Surface(k) if k == s.kind)
                        && (r.local_t_m - local_t_m).abs() < 1.25
                });
                if !dup {
                    ribbons.push(RibbonAnchor {
                        target: RibbonTargetKind::Surface(s.kind),
                        local_t_m,
                        span_cm: s.width_cm.clamp(100, 650),
                        variant_seed: s.variant_seed,
                    });
                }
            }
        }
    }

    for p in &map.props {
        if !is_continuous_ribbon_prop(p.kind) {
            continue;
        }
        let fp = p.footprint_m();
        let (norm_dist, tang_coord) = match dir {
            MapContinuationDir::North => (fp.cy_m - fp.hy_m.max(0.5), fp.cx_m),
            MapContinuationDir::South => (h as f32 - (fp.cy_m + fp.hy_m.max(0.5)), fp.cx_m),
            MapContinuationDir::West => (fp.cx_m - fp.hx_m.max(0.5), fp.cy_m),
            MapContinuationDir::East => (w as f32 - (fp.cx_m + fp.hx_m.max(0.5)), fp.cy_m),
        };
        if norm_dist <= 1.25 && tang_coord >= t0 as f32 - 0.5 && tang_coord <= (t0 + t_len) as f32 + 0.5 {
            let local_t_m = (tang_coord - t0 as f32).clamp(0.5, t_len as f32 - 0.5);
            let tang_span = match dir {
                MapContinuationDir::North | MapContinuationDir::South => (fp.hx_m * 2.0).clamp(1.0, 5.0),
                MapContinuationDir::East | MapContinuationDir::West => (fp.hy_m * 2.0).clamp(1.0, 5.0),
            };
            let dup = ribbons.iter().any(|r: &RibbonAnchor| {
                matches!(r.target, RibbonTargetKind::LegacyProp(k) if k == p.kind)
                    && (r.local_t_m - local_t_m).abs() < 1.25
            });
            if !dup {
                ribbons.push(RibbonAnchor {
                    target: RibbonTargetKind::LegacyProp(p.kind),
                    local_t_m,
                    span_cm: (tang_span * 100.0).round() as i32,
                    variant_seed: p.variant_seed,
                });
            }
        }
    }

    ribbons
}

/// Mirrors all vector surfaces inside `sec` across X (`horizontal_flip = true`) or Y (`false`).
pub(crate) fn mirror_sector_surfaces(
    surfaces: &mut [MapSurfaceFeature],
    cw: i32,
    ch: i32,
    horizontal_flip: bool,
) {
    let (w_cm, h_cm) = (cw * 100, ch * 100);
    for s in surfaces {
        *s = s.mirrored(w_cm, h_cm, horizontal_flip);
    }
}

pub(crate) fn clear_props_near(props: &mut Vec<MapPropPlacement>, px_m: f32, py_m: f32, radius_m: f32) {
    props.retain(|p| {
        prop_spec(p.kind).floor_layer || !prop_intersects_circle(p, px_m, py_m, radius_m)
    });
}

fn find_chunk_entry_coord(
    sec: &ProceduralSector,
    target: &RibbonTargetKind,
    cw: i32,
    ch: i32,
    entry_side: MapContinuationDir,
) -> Option<f32> {
    let horizontal_seam = matches!(entry_side, MapContinuationDir::North | MapContinuationDir::South);
    match target {
        RibbonTargetKind::Surface(kind) => sec
            .surfaces
            .iter()
            .filter(|s| s.kind == *kind && s.shape == SurfaceShapeKind::RibbonSpline && s.points_cm.len() >= 2)
            .filter_map(|s| {
                let endpoints = [s.points_cm[0].to_m(), s.points_cm[s.points_cm.len() - 1].to_m()];
                endpoints.into_iter().find_map(|pt| {
                    let (nd, tc) = point_seam_coords(pt, entry_side, cw, ch);
                    (nd <= 1.50).then_some(tc)
                })
            })
            .next(),
        RibbonTargetKind::LegacyProp(kind) => sec
            .props
            .iter()
            .filter(|p| p.kind == *kind)
            .filter_map(|p| {
                let fp = p.footprint_m();
                let norm_dist = match entry_side {
                    MapContinuationDir::South => ch as f32 - (fp.cy_m + fp.hy_m.max(0.5)),
                    MapContinuationDir::North => fp.cy_m - fp.hy_m.max(0.5),
                    MapContinuationDir::East => cw as f32 - (fp.cx_m + fp.hx_m.max(0.5)),
                    MapContinuationDir::West => fp.cx_m - fp.hx_m.max(0.5),
                };
                (norm_dist <= 1.25).then_some(if horizontal_seam { fp.cx_m } else { fp.cy_m })
            })
            .next(),
    }
}

/// Aligns `sec`'s orientation (`mirror_sector`) and splices smooth cross-seam spline bridges or legacy tiles.
pub(crate) fn align_chunk_ribbons(
    sec: &mut ProceduralSector,
    preset: SectorPreset,
    ribbons: &[RibbonAnchor],
    cw: i32,
    ch: i32,
    dir: MapContinuationDir,
    mirror_fn: fn(&mut ProceduralSector, i32, i32, bool),
) {
    if ribbons.is_empty() {
        return;
    }
    let horizontal_seam = matches!(dir, MapContinuationDir::North | MapContinuationDir::South);
    let entry_side = dir.opposite();
    let allow_mirror = preset.outdoor_style().is_some() || preset == SectorPreset::UrbanCrossroads;

    if allow_mirror
        && let Some(primary) = ribbons.first()
        && let Some(dst_t) = find_chunk_entry_coord(sec, &primary.target, cw, ch, entry_side)
    {
        let span_max = if horizontal_seam { cw as f32 } else { ch as f32 };
        let flipped_t = span_max - dst_t;
        if (flipped_t - primary.local_t_m).abs() + 0.5 < (dst_t - primary.local_t_m).abs() {
            mirror_fn(sec, cw, ch, horizontal_seam);
        }
    }

    let mut legacy_additions = Vec::new();
    let mut new_bridges = Vec::new();
    for anchor in ribbons {
        let dst_opt = find_chunk_entry_coord(sec, &anchor.target, cw, ch, entry_side);
        let dst_t = dst_opt.unwrap_or(anchor.local_t_m);
        match anchor.target {
            RibbonTargetKind::Surface(kind) => {
                // Smooth 3-point bridge overlapping (-0.4m .. 0 .. +2.2m) into the adjacent chunk.
                let (seam_n, inward) = match entry_side {
                    MapContinuationDir::North => (0.0_f32, 1.0_f32),
                    MapContinuationDir::South => (ch as f32, -1.0_f32),
                    MapContinuationDir::West => (0.0_f32, 1.0_f32),
                    MapContinuationDir::East => (cw as f32, -1.0_f32),
                };
                let reach_in = if dst_opt.is_some() { 1.6_f32 } else { 2.4_f32 };
                let mid_t = 0.5 * (anchor.local_t_m + dst_t);
                let pt_m = |n_pos: f32, t_pos: f32| -> (f32, f32) {
                    if horizontal_seam { (t_pos, n_pos) } else { (n_pos, t_pos) }
                };
                let knots_m = [
                    pt_m(seam_n - inward * 0.45, anchor.local_t_m),
                    pt_m(seam_n + inward * (reach_in * 0.45), mid_t),
                    pt_m(seam_n + inward * reach_in, dst_t),
                ];
                for &(wx, wy) in &knots_m {
                    clear_props_near(&mut sec.props, wx, wy, 0.90);
                }
                let bridge = MapSurfaceFeature {
                    kind,
                    shape: SurfaceShapeKind::RibbonSpline,
                    points_cm: knots_m
                        .into_iter()
                        .map(|(x, y)| MapPointCm::new((x * 100.0).round() as i32, (y * 100.0).round() as i32))
                        .collect(),
                    width_cm: anchor.span_cm,
                    border_style_cm: 0,
                    variant_seed: anchor.variant_seed ^ 0x5E43,
                };
                if kind.is_road_or_pavement() {
                    // Protect seam bridges from any overlapping organic mulch/flowerbed centroid inside `sec`.
                    sec.surfaces.retain(|s| {
                        s.kind.layer_priority() >= SurfaceFeatureKind::DirtTrail.layer_priority()
                            || !knots_m
                                .iter()
                                .any(|&(kx, ky)| surface_contains_point_m(s, kx, ky))
                    });
                }
                new_bridges.push(bridge);
            }
            RibbonTargetKind::LegacyProp(kind) => {
                let seam_cell_n = match entry_side {
                    MapContinuationDir::North => 0.5,
                    MapContinuationDir::South => ch as f32 - 0.5,
                    MapContinuationDir::West => 0.5,
                    MapContinuationDir::East => cw as f32 - 0.5,
                };
                let inward = match entry_side {
                    MapContinuationDir::North | MapContinuationDir::West => 1.0,
                    MapContinuationDir::South | MapContinuationDir::East => -1.0,
                };
                let steps = ((dst_t - anchor.local_t_m).abs().ceil() as i32 + 2).clamp(2, 6);
                for i in 0..steps {
                    let frac = i as f32 / (steps - 1).max(1) as f32;
                    let tang = anchor.local_t_m + (dst_t - anchor.local_t_m) * frac;
                    let norm = seam_cell_n + inward * (frac * 1.5);
                    let (wx, wy) = if horizontal_seam { (tang, norm) } else { (norm, tang) };
                    let rot = if horizontal_seam { 0 } else { 90 };
                    legacy_additions.push(make_prop_at_cm(
                        kind,
                        (wx * 100.0).round() as i32,
                        (wy * 100.0).round() as i32,
                        rot,
                        Some(anchor.span_cm.clamp(100, 500)),
                        0,
                    ));
                    clear_props_near(&mut sec.props, wx, wy, 0.75);
                }
            }
        }
    }

    sec.surfaces.extend(new_bridges);
    if !legacy_additions.is_empty() {
        let insert_at = sec
            .props
            .iter()
            .position(|p| !prop_spec(p.kind).floor_layer)
            .unwrap_or(sec.props.len());
        sec.props.splice(insert_at..insert_at, legacy_additions);
    }
}

/// Shifts every floor level (`wall_edges`, `props`, `surfaces`) by `(dx_cells, dy_cells)`.
pub(crate) fn translate_floor_levels(
    floors: &mut [MapFloorLevel],
    dx_cells: i32,
    dy_cells: i32,
    cell_cm: i32,
) {
    if dx_cells == 0 && dy_cells == 0 {
        return;
    }
    for fl in floors {
        for e in &mut fl.wall_edges {
            e.x += dx_cells;
            e.y += dy_cells;
        }
        for p in &mut fl.props {
            *p = translate_prop(*p, dx_cells, dy_cells, cell_cm);
        }
        for s in &mut fl.surfaces {
            *s = s.translated(dx_cells * cell_cm, dy_cells * cell_cm);
        }
    }
}

/// Mirrors additional vertical floor levels (`B1`, `1F`, `2F`) identically alongside ground geometry.
pub(crate) fn mirror_floor_levels(
    floors: &mut [MapFloorLevel],
    cw: i32,
    ch: i32,
    horizontal_flip: bool,
) {
    for fl in floors {
        for e in &mut fl.wall_edges {
            mirror_edge_axis(e, cw, ch, horizontal_flip);
        }
        for p in &mut fl.props {
            *p = mirror_prop_axis(*p, cw, ch, horizontal_flip);
        }
        mirror_sector_surfaces(&mut fl.surfaces, cw, ch, horizontal_flip);
    }
}

/// Merges non-ground floor levels (`B1`, `2F`, etc.) from a newly continued sector chunk into `base.floors`.
pub(crate) fn stitch_continued_floors(
    base: &mut GmMapUpdateRequest,
    sec_floors: &[MapFloorLevel],
    x_off: i32,
    y_off: i32,
    cell_cm: i32,
) {
    for src in sec_floors {
        if src.floor_index == 0 {
            continue;
        }
        if base.floors.is_empty() {
            base.floors.push(MapFloorLevel {
                floor_index: 0,
                label: "1F · Ground".to_string(),
                elevation_cm: 0,
                wall_edges: base.wall_edges.clone(),
                props: base.props.clone(),
                surfaces: base.surfaces.clone(),
            });
        }
        if let Some(dst) = base.floors.iter_mut().find(|f| f.floor_index == src.floor_index) {
            for e in &src.wall_edges {
                let mut shifted = *e;
                shifted.x += x_off;
                shifted.y += y_off;
                upsert_edge(&mut dst.wall_edges, shifted);
            }
            let mut floor_props = Vec::new();
            let mut solid_props = Vec::new();
            for &p in &src.props {
                let sp = translate_prop(p, x_off, y_off, cell_cm);
                if prop_spec(sp.kind).floor_layer {
                    floor_props.push(sp);
                } else {
                    solid_props.push(sp);
                }
            }
            let first_solid = dst
                .props
                .iter()
                .position(|p| !prop_spec(p.kind).floor_layer)
                .unwrap_or(dst.props.len());
            dst.props.splice(first_solid..first_solid, floor_props);
            dst.props.extend(solid_props);
            for s in &src.surfaces {
                dst.surfaces.push(s.translated(x_off * cell_cm, y_off * cell_cm));
            }
        } else {
            let mut added = src.clone();
            translate_floor_levels(std::slice::from_mut(&mut added), x_off, y_off, cell_cm);
            base.floors.push(added);
            base.floors.sort_by_key(|f| f.floor_index);
        }
    }
}
