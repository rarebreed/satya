//! # Macro-World Stitched Sector Grid & Realistic Adjacency Transitions (`map_world_graph`)
//!
//! Computes `N/S/E/W` [`SectorBoundaryProfile`] edge contracts (parallel border roads, perpendicular
//! spline crossings for roads/water/trails, and perimeter wall coverage), partitions continuation
//! seams to align with existing [`WorldSectorCell`]s, and drives deterministic distance-weighted +
//! road/water-gated neighbor transitions across urban, suburban, estate, park, rural, indoor, and
//! facility sectors.

use satya_schema::{
    EconomicTier, EdgeSide, GmMapUpdateRequest, MapContinuationDir, MapPropPlacement,
    MapSurfaceFeature, MapWallEdge, PropKind, SectorBoundaryProfile, SurfaceFeatureKind,
    SurfaceShapeKind, WorldSectorCell, ZoningContext,
};

use crate::map_continuation::{
    continuation_strip_depth, next_continuation_seed, partition_span_into_chunks,
};
use crate::map_gen_zoning::default_zoning_and_tier;
use crate::tactical_map::{MAX_SECTOR_DIM, MIN_SECTOR_DIM, ProceduralSector, SectorPreset};

pub use crate::map_world_transitions::{
    MacroZoneFamily, NeighborSectorSpec, select_neighbor_sector_transition,
};

const BORDER_ROAD_BAND_CM: i32 = 450;
const MIN_PRIMARY_ROAD_WIDTH_CM: i32 = 220;
const CROSSING_EDGE_BAND_CM: i32 = 95;

/// Planned continuation chunk along one seam sub-span tied to its adjacent source [`WorldSectorCell`].
#[derive(Debug, Clone)]
pub struct SeamSectorChunkPlan {
    pub t0: i32,
    pub t_len: i32,
    pub source_cell: WorldSectorCell,
    pub target_sector_coord: (i32, i32),
}

fn push_unique_crossing(list: &mut Vec<i32>, coord_cm: i32) {
    if !list.iter().any(|&v| (v - coord_cm).abs() < 150) {
        list.push(coord_cm);
        list.sort_unstable();
    }
}

fn surface_crossing_kind(kind: SurfaceFeatureKind) -> Option<u8> {
    match kind {
        SurfaceFeatureKind::AsphaltRoad | SurfaceFeatureKind::GravelRoad => Some(0),
        SurfaceFeatureKind::CreekChannel | SurfaceFeatureKind::DrainageDitch => Some(1),
        SurfaceFeatureKind::DirtTrail
        | SurfaceFeatureKind::StoneWalk
        | SurfaceFeatureKind::ConcreteWalk => Some(2),
        _ => None,
    }
}

/// Measures `N/S/E/W` parallel roadways, perpendicular spline crossings, and perimeter wall coverage.
#[must_use]
pub fn compute_sector_boundary_profile(
    width: i32,
    height: i32,
    wall_edges: &[MapWallEdge],
    surfaces: &[MapSurfaceFeature],
    props: &[MapPropPlacement],
) -> SectorBoundaryProfile {
    let w_cm = width.max(1) * 100;
    let h_cm = height.max(1) * 100;
    let mut profile = SectorBoundaryProfile::default();

    let (mut n_span, mut n_wsum) = (0_i64, 0_i64);
    let (mut s_span, mut s_wsum) = (0_i64, 0_i64);
    let (mut w_span, mut w_wsum) = (0_i64, 0_i64);
    let (mut e_span, mut e_wsum) = (0_i64, 0_i64);

    for surf in surfaces {
        if surf.shape != SurfaceShapeKind::RibbonSpline || surf.points_cm.len() < 2 {
            continue;
        }
        let pts = &surf.points_cm;
        if matches!(
            surf.kind,
            SurfaceFeatureKind::AsphaltRoad | SurfaceFeatureKind::GravelRoad
        ) && surf.width_cm >= MIN_PRIMARY_ROAD_WIDTH_CM
        {
            for pair in pts.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let dx_raw = (b.x_cm - a.x_cm).abs();
                let dy_raw = (b.y_cm - a.y_cm).abs();
                let mx = (a.x_cm + b.x_cm) / 2;
                let my = (a.y_cm + b.y_cm) / 2;
                if dx_raw >= dy_raw {
                    let dx = (b.x_cm.clamp(0, w_cm) - a.x_cm.clamp(0, w_cm)).abs() as i64;
                    if (0..=BORDER_ROAD_BAND_CM).contains(&my) {
                        n_span += dx;
                        n_wsum += dx * my.max(1) as i64;
                    }
                    if my <= h_cm && my >= h_cm - BORDER_ROAD_BAND_CM {
                        s_span += dx;
                        s_wsum += dx * (h_cm - my).max(1) as i64;
                    }
                }
                if dy_raw >= dx_raw {
                    let dy = (b.y_cm.clamp(0, h_cm) - a.y_cm.clamp(0, h_cm)).abs() as i64;
                    if (0..=BORDER_ROAD_BAND_CM).contains(&mx) {
                        w_span += dy;
                        w_wsum += dy * mx.max(1) as i64;
                    }
                    if mx <= w_cm && mx >= w_cm - BORDER_ROAD_BAND_CM {
                        e_span += dy;
                        e_wsum += dy * (w_cm - mx).max(1) as i64;
                    }
                }
            }
        }

        if let Some(bucket) = surface_crossing_kind(surf.kind) {
            for (end_idx, adj_idx) in [(0, 1), (pts.len() - 1, pts.len() - 2)] {
                let pt = pts[end_idx];
                let adj = pts[adj_idx];
                let dx = (pt.x_cm - adj.x_cm).abs();
                let dy = (pt.y_cm - adj.y_cm).abs();
                let check_ends = [
                    (
                        MapContinuationDir::North,
                        pt.y_cm <= CROSSING_EDGE_BAND_CM,
                        pt.x_cm.clamp(0, w_cm),
                        dx < dy * 2 + 25,
                    ),
                    (
                        MapContinuationDir::South,
                        pt.y_cm >= h_cm - CROSSING_EDGE_BAND_CM,
                        pt.x_cm.clamp(0, w_cm),
                        dx < dy * 2 + 25,
                    ),
                    (
                        MapContinuationDir::West,
                        pt.x_cm <= CROSSING_EDGE_BAND_CM,
                        pt.y_cm.clamp(0, h_cm),
                        dy < dx * 2 + 25,
                    ),
                    (
                        MapContinuationDir::East,
                        pt.x_cm >= w_cm - CROSSING_EDGE_BAND_CM,
                        pt.y_cm.clamp(0, h_cm),
                        dy < dx * 2 + 25,
                    ),
                ];
                for (dir, near_edge, t_cm, approach_ok) in check_ends {
                    if !near_edge || (bucket == 0 && !approach_ok) {
                        continue;
                    }
                    let edge = profile.edge_mut(dir);
                    match bucket {
                        0 => push_unique_crossing(&mut edge.crossing_roads_cm, t_cm),
                        1 => push_unique_crossing(&mut edge.crossing_water_cm, t_cm),
                        _ => push_unique_crossing(&mut edge.crossing_trails_cm, t_cm),
                    }
                }
            }
        }
    }

    if surfaces.is_empty() {
        for p in props {
            let cx = p.center_cm().0;
            let cy = p.center_cm().1;
            let bucket = match p.kind {
                PropKind::AsphaltDrive | PropKind::GravelPath => Some(0),
                PropKind::CreekWater | PropKind::DrainageDitch => Some(1),
                PropKind::DirtPath | PropKind::StonePath => Some(2),
                _ => None,
            };
            let Some(b) = bucket else {
                continue;
            };
            for (dir, hit, tc) in [
                (MapContinuationDir::North, cy <= 150, cx.clamp(0, w_cm)),
                (
                    MapContinuationDir::South,
                    cy >= h_cm - 150,
                    cx.clamp(0, w_cm),
                ),
                (MapContinuationDir::West, cx <= 150, cy.clamp(0, h_cm)),
                (
                    MapContinuationDir::East,
                    cx >= w_cm - 150,
                    cy.clamp(0, h_cm),
                ),
            ] {
                if hit {
                    let edge = profile.edge_mut(dir);
                    if b == 0 {
                        push_unique_crossing(&mut edge.crossing_roads_cm, tc);
                    } else if b == 1 {
                        push_unique_crossing(&mut edge.crossing_water_cm, tc);
                    } else {
                        push_unique_crossing(&mut edge.crossing_trails_cm, tc);
                    }
                }
            }
        }
    }

    let min_x_span = (w_cm as i64 * 35) / 100;
    let min_y_span = (h_cm as i64 * 35) / 100;
    for (dir, span, wsum, req) in [
        (MapContinuationDir::North, n_span, n_wsum, min_x_span),
        (MapContinuationDir::South, s_span, s_wsum, min_x_span),
        (MapContinuationDir::West, w_span, w_wsum, min_y_span),
        (MapContinuationDir::East, e_span, e_wsum, min_y_span),
    ] {
        if span >= req && span > 0 {
            let edge = profile.edge_mut(dir);
            edge.road_parallel = true;
            edge.road_offset_cm = (wsum / span).max(1) as i32;
        }
    }

    let (mut n_w, mut s_w, mut w_w, mut e_w) = (0_i32, 0_i32, 0_i32, 0_i32);
    for e in wall_edges {
        match e.side {
            EdgeSide::North if e.y == 0 => n_w += 1,
            EdgeSide::North if e.y == height => s_w += 1,
            EdgeSide::West if e.x == 0 => w_w += 1,
            EdgeSide::West if e.x == width => e_w += 1,
            _ => {}
        }
    }
    profile.north.walled_pct = ((n_w * 100) / width.max(1)).min(100) as u8;
    profile.south.walled_pct = ((s_w * 100) / width.max(1)).min(100) as u8;
    profile.west.walled_pct = ((w_w * 100) / height.max(1)).min(100) as u8;
    profile.east.walled_pct = ((e_w * 100) / height.max(1)).min(100) as u8;
    profile
}

/// Constructs the root `(0, 0)` [`WorldSectorCell`] from generated geometry and metadata.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn build_root_world_sector_cell(
    preset: SectorPreset,
    width: i32,
    height: i32,
    seed: i64,
    zoning_opt: Option<ZoningContext>,
    tier_opt: Option<EconomicTier>,
    wall_edges: &[MapWallEdge],
    surfaces: &[MapSurfaceFeature],
    props: &[MapPropPlacement],
) -> WorldSectorCell {
    let (zoning, economic_tier) = if preset.is_zoned_preset() {
        let (dz, dt) = default_zoning_and_tier(preset);
        (Some(zoning_opt.unwrap_or(dz)), Some(tier_opt.unwrap_or(dt)))
    } else {
        (zoning_opt, tier_opt)
    };
    WorldSectorCell {
        sector_x: 0,
        sector_y: 0,
        origin_x: 0,
        origin_y: 0,
        width,
        height,
        preset: preset.as_str().to_string(),
        zoning,
        economic_tier,
        zone_depth: 0,
        seed,
        boundary: compute_sector_boundary_profile(width, height, wall_edges, surfaces, props),
    }
}

/// Ensures any legacy or pre-Phase-4 [`GmMapUpdateRequest`] yields a valid `world_sectors` vector.
#[must_use]
pub fn bootstrap_world_sectors_if_empty(map: &GmMapUpdateRequest) -> Vec<WorldSectorCell> {
    if !map.world_sectors.is_empty() {
        return map.world_sectors.clone();
    }
    let preset = map
        .theme_preset
        .as_deref()
        .and_then(SectorPreset::from_code_str)
        .unwrap_or_default();
    vec![build_root_world_sector_cell(
        preset,
        map.grid_width,
        map.grid_height,
        map.seed.unwrap_or(42),
        None,
        None,
        &map.wall_edges,
        &map.surfaces,
        &map.props,
    )]
}

/// Partitions a stitched continuation seam along `dir` into chunks aligned with existing boundary [`WorldSectorCell`]s.
#[must_use]
pub fn partition_continuation_seam_by_world_sectors(
    world_sectors: &[WorldSectorCell],
    old_w: i32,
    old_h: i32,
    dir: MapContinuationDir,
) -> Vec<SeamSectorChunkPlan> {
    let horizontal_seam = matches!(dir, MapContinuationDir::North | MapContinuationDir::South);
    let total_span = if horizontal_seam { old_w } else { old_h };
    let target_chunk = if horizontal_seam { 32 } else { 26 };
    let (ux, uy) = dir.unit_step();

    let mut boundary_cells: Vec<(i32, i32, &WorldSectorCell)> = world_sectors
        .iter()
        .filter_map(|c| {
            let touches = match dir {
                MapContinuationDir::North => c.origin_y == 0,
                MapContinuationDir::South => c.origin_y + c.height == old_h,
                MapContinuationDir::West => c.origin_x == 0,
                MapContinuationDir::East => c.origin_x + c.width == old_w,
            };
            if !touches {
                return None;
            }
            let (t0, t_len) = if horizontal_seam {
                (c.origin_x, c.width)
            } else {
                (c.origin_y, c.height)
            };
            (t_len >= MIN_SECTOR_DIM).then_some((t0, t_len, c))
        })
        .collect();
    boundary_cells.sort_by_key(|(t0, _, _)| *t0);

    let mut cursor = 0;
    let exact_cover = !boundary_cells.is_empty()
        && boundary_cells.iter().all(|&(t0, t_len, _)| {
            let ok = t0 == cursor && (MIN_SECTOR_DIM..=MAX_SECTOR_DIM).contains(&t_len);
            cursor += t_len;
            ok
        })
        && cursor == total_span;

    if exact_cover {
        let mut plans = Vec::with_capacity(boundary_cells.len());
        for (t0, t_len, cell) in boundary_cells {
            if t_len <= 64 {
                plans.push(SeamSectorChunkPlan {
                    t0,
                    t_len,
                    source_cell: cell.clone(),
                    target_sector_coord: (cell.sector_x + ux, cell.sector_y + uy),
                });
            } else {
                for (sub_idx, (sub_t0, sub_len)) in partition_span_into_chunks(t_len, target_chunk)
                    .into_iter()
                    .enumerate()
                {
                    let extra = sub_idx as i32;
                    let target_sector_coord = if horizontal_seam {
                        (cell.sector_x + ux + extra, cell.sector_y + uy)
                    } else {
                        (cell.sector_x + ux, cell.sector_y + uy + extra)
                    };
                    plans.push(SeamSectorChunkPlan {
                        t0: t0 + sub_t0,
                        t_len: sub_len,
                        source_cell: cell.clone(),
                        target_sector_coord,
                    });
                }
            }
        }
        return plans;
    }

    let fallback_src = world_sectors.first().cloned().unwrap_or_default();
    partition_span_into_chunks(total_span, target_chunk)
        .into_iter()
        .enumerate()
        .map(|(idx, (t0, t_len))| {
            let mid = t0 + t_len / 2;
            let src = boundary_cells
                .iter()
                .find(|(bt0, blen, _)| mid >= *bt0 && mid < bt0 + blen)
                .map_or(&fallback_src, |(_, _, c)| *c)
                .clone();
            let extra = if boundary_cells.len() <= 1 {
                idx as i32
            } else {
                0
            };
            let target_sector_coord = if horizontal_seam {
                (src.sector_x + ux + extra, src.sector_y + uy)
            } else {
                (src.sector_x + ux, src.sector_y + uy + extra)
            };
            SeamSectorChunkPlan {
                t0,
                t_len,
                source_cell: src,
                target_sector_coord,
            }
        })
        .collect()
}

/// Mirrors a generated sector along its normal axis when its primary border road lies opposite `preferred_road_side`.
pub(crate) fn orient_sector_to_shared_road(
    sec: &mut ProceduralSector,
    cw: i32,
    ch: i32,
    preferred_road_side: Option<MapContinuationDir>,
    mirror_fn: fn(&mut ProceduralSector, i32, i32, bool),
) {
    let Some(target_side) = preferred_road_side else {
        return;
    };
    let profile =
        compute_sector_boundary_profile(cw, ch, &sec.wall_edges, &sec.surfaces, &sec.props);
    if !profile.edge(target_side).road_parallel
        && profile.edge(target_side.opposite()).road_parallel
    {
        let horizontal_flip = matches!(
            target_side,
            MapContinuationDir::East | MapContinuationDir::West
        );
        mirror_fn(sec, cw, ch, horizontal_flip);
    }
}

/// Formats an informative map footer summarizing stitched multi-sector district coverage.
#[must_use]
pub fn format_stitched_world_notes(
    world_sectors: &[WorldSectorCell],
    fallback_preset: SectorPreset,
    width: i32,
    height: i32,
    seed: i64,
) -> String {
    if world_sectors.len() <= 1 {
        return format!(
            "{} · Continuous Metric Arena ({width}m x {height}m, Seed {seed})",
            fallback_preset.subtitle()
        );
    }
    let mut distinct_labels: Vec<&'static str> = Vec::new();
    for cell in world_sectors {
        let label = SectorPreset::from_code_str(&cell.preset)
            .unwrap_or(fallback_preset)
            .label();
        if !distinct_labels.contains(&label) {
            distinct_labels.push(label);
        }
    }
    let summary = distinct_labels
        .iter()
        .take(4)
        .copied()
        .collect::<Vec<_>>()
        .join(" + ");
    let suffix = if distinct_labels.len() > 4 { "…" } else { "" };
    format!(
        "{} Sectors ({summary}{suffix}) · Stitched Metric Arena ({width}m x {height}m, Seed {seed})",
        world_sectors.len()
    )
}

/// Generates a user-facing tooltip previewing the neighbor sector along `dir`.
#[must_use]
pub fn preview_continuation_tooltip(map: &GmMapUpdateRequest, dir: MapContinuationDir) -> String {
    let strip = continuation_strip_depth(map, dir);
    let sectors = bootstrap_world_sectors_if_empty(map);
    let plans = partition_continuation_seam_by_world_sectors(
        &sectors,
        map.grid_width,
        map.grid_height,
        dir,
    );
    let Some(plan) = plans.first() else {
        return format!("Continue {} (+{strip}m)", dir.label());
    };
    let next_seed =
        next_continuation_seed(map.seed.unwrap_or(42), map.grid_width, map.grid_height, dir);
    let spec = select_neighbor_sector_transition(
        &plan.source_cell,
        &sectors,
        plan.target_sector_coord,
        dir,
        next_seed,
    );
    let via_road = if plan.source_cell.boundary.edge(dir).road_parallel {
        " · across border road"
    } else if plan.source_cell.boundary.edge(dir).has_water() {
        " · following waterway"
    } else {
        ""
    };
    let tier_note = spec
        .economic_tier
        .map(|t| format!(" ({}{via_road})", t.label()))
        .unwrap_or_else(|| {
            if via_road.is_empty() {
                String::new()
            } else {
                format!(" ({})", &via_road[3..])
            }
        });
    if plans.len() > 1 {
        format!(
            "Continue {} (+{strip}m, {} sectors) → {}{}",
            dir.label(),
            plans.len(),
            spec.preset.label(),
            tier_note
        )
    } else {
        format!(
            "Continue {} (+{strip}m) → {}{}",
            dir.label(),
            spec.preset.label(),
            tier_note
        )
    }
}
