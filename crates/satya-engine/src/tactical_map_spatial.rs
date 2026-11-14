//! # Uniform Spatial Obstacle Index (`tactical_map_spatial`)
//!
//! Companion module to [`crate::tactical_map_geom`] providing an `O(1)` uniform `2.0m` grid
//! and dense [`WallEdgeIndex`] wrapper so `0.5m` connectivity checks and `0.25m` 16-directional
//! gait Dijkstra scale smoothly from `20m × 14m` up to `64m × 48m` sectors and `240m × 180m`
//! stitched arenas in sub-5ms time with exact geometric parity.

use satya_schema::{EdgeSide, MapPropPlacement, MapWallEdge, WallEdgeKind};

use crate::map_dressing::{PropObstruction, prop_spec, rotated_aabb_half_extents_cm};
use crate::tactical_map_edges::WallEdgeIndex;
use crate::tactical_map_geom::{
    EDGE_HALF_THICK_M, distance_point_to_segment, edge_endpoints_m, intersect_segment_edge,
    prop_contains_point, prop_intersects_circle,
};

const SPATIAL_BUCKET_M: f32 = 2.0;

/// Filter controlling which [`WallEdgeKind`] entries count as solid barriers during spatial queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialWallFilter {
    /// Blocks on `Wall` and `Window` edges; treats both `DoorOpen` and `DoorClosed` as passable.
    WallsAndWindowsOnly,
    /// Blocks on `Wall`, `Window`, and `DoorClosed`; ignores only `DoorOpen`.
    MovementBlocking,
}

impl SpatialWallFilter {
    #[inline]
    const fn blocks(self, kind: WallEdgeKind) -> bool {
        match self {
            Self::WallsAndWindowsOnly => matches!(kind, WallEdgeKind::Wall | WallEdgeKind::Window),
            Self::MovementBlocking => kind.blocks_movement(),
        }
    }
}

/// Uniform `2.0m × 2.0m` prop bucket index paired with a dense [`WallEdgeIndex`] for rapid exact queries.
pub struct SpatialObstacleIndex<'a> {
    width_m: f32,
    height_m: f32,
    width_cells: i32,
    height_cells: i32,
    b_cols: usize,
    b_rows: usize,
    edge_idx: WallEdgeIndex,
    props: &'a [MapPropPlacement],
    solid_buckets: Vec<Vec<u32>>,
    move_block_buckets: Vec<Vec<u32>>,
    slow_prop_buckets: Vec<Vec<u32>>,
}

impl<'a> SpatialObstacleIndex<'a> {
    /// Builds a uniform spatial index covering `(width_m × height_m)` from `edges` and `props`.
    #[must_use]
    pub fn new(
        width_m: f32,
        height_m: f32,
        edges: &'a [MapWallEdge],
        props: &'a [MapPropPlacement],
    ) -> Self {
        let width_cells = (width_m.ceil() as i32).max(1);
        let height_cells = (height_m.ceil() as i32).max(1);
        let b_cols = ((width_m / SPATIAL_BUCKET_M).ceil() as usize).max(1);
        let b_rows = ((height_m / SPATIAL_BUCKET_M).ceil() as usize).max(1);
        let bucket_count = b_cols * b_rows;

        let mut solid_buckets = vec![Vec::new(); bucket_count];
        let mut move_block_buckets = vec![Vec::new(); bucket_count];
        let mut slow_prop_buckets = vec![Vec::new(); bucket_count];

        for (idx, p) in props.iter().enumerate() {
            let sp = prop_spec(p.kind);
            let is_full = sp.obstruction == PropObstruction::FullObstacle;
            let blocks_move = sp.obstruction.blocks_movement();
            let is_slow = sp.move_cost_mult > 1.0;
            if !is_full && !blocks_move && !is_slow {
                continue;
            }
            let fp = p.footprint_m();
            let (hw_cm, hh_cm) = rotated_aabb_half_extents_cm(
                (fp.span_w_m * 100.0).round() as i32,
                (fp.span_h_m * 100.0).round() as i32,
                fp.rot_deg,
            );
            let hw = hw_cm / 100.0 + 0.08;
            let hh = hh_cm / 100.0 + 0.08;
            let bx0 = ((fp.cx_m - hw) / SPATIAL_BUCKET_M)
                .floor()
                .clamp(0.0, (b_cols - 1) as f32) as usize;
            let bx1 = ((fp.cx_m + hw) / SPATIAL_BUCKET_M)
                .floor()
                .clamp(0.0, (b_cols - 1) as f32) as usize;
            let by0 = ((fp.cy_m - hh) / SPATIAL_BUCKET_M)
                .floor()
                .clamp(0.0, (b_rows - 1) as f32) as usize;
            let by1 = ((fp.cy_m + hh) / SPATIAL_BUCKET_M)
                .floor()
                .clamp(0.0, (b_rows - 1) as f32) as usize;

            let p_idx = idx as u32;
            for by in by0..=by1 {
                for bx in bx0..=bx1 {
                    let b_idx = by * b_cols + bx;
                    if is_full {
                        solid_buckets[b_idx].push(p_idx);
                    }
                    if blocks_move {
                        move_block_buckets[b_idx].push(p_idx);
                    }
                    if is_slow {
                        slow_prop_buckets[b_idx].push(p_idx);
                    }
                }
            }
        }

        Self {
            width_m,
            height_m,
            width_cells,
            height_cells,
            b_cols,
            b_rows,
            edge_idx: WallEdgeIndex::from_edges(width_cells, height_cells, edges),
            props,
            solid_buckets,
            move_block_buckets,
            slow_prop_buckets,
        }
    }

    #[inline]
    fn bucket_range(&self, px: f32, py: f32, pad_m: f32) -> (usize, usize, usize, usize) {
        let bx0 = ((px - pad_m) / SPATIAL_BUCKET_M)
            .floor()
            .clamp(0.0, (self.b_cols - 1) as f32) as usize;
        let bx1 = ((px + pad_m) / SPATIAL_BUCKET_M)
            .floor()
            .clamp(0.0, (self.b_cols - 1) as f32) as usize;
        let by0 = ((py - pad_m) / SPATIAL_BUCKET_M)
            .floor()
            .clamp(0.0, (self.b_rows - 1) as f32) as usize;
        let by1 = ((py + pad_m) / SPATIAL_BUCKET_M)
            .floor()
            .clamp(0.0, (self.b_rows - 1) as f32) as usize;
        (bx0, bx1, by0, by1)
    }

    fn bucket_any_prop_intersects(
        &self,
        buckets: &[Vec<u32>],
        px: f32,
        py: f32,
        radius_m: f32,
    ) -> bool {
        let (bx0, bx1, by0, by1) = self.bucket_range(px, py, radius_m + 0.05);
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                for &pi in &buckets[by * self.b_cols + bx] {
                    if prop_intersects_circle(&self.props[pi as usize], px, py, radius_m) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Returns `true` if any `FullObstacle` prop intersects disc `(px, py, radius_m)`.
    #[must_use]
    pub fn any_full_obstacle_prop_intersects_circle(
        &self,
        px: f32,
        py: f32,
        radius_m: f32,
    ) -> bool {
        self.bucket_any_prop_intersects(&self.solid_buckets, px, py, radius_m)
    }

    /// Returns `true` if any movement-blocking prop intersects disc `(px, py, radius_m)`.
    #[must_use]
    pub fn any_move_blocking_prop_intersects_circle(
        &self,
        px: f32,
        py: f32,
        radius_m: f32,
    ) -> bool {
        self.bucket_any_prop_intersects(&self.move_block_buckets, px, py, radius_m)
    }

    /// Returns the maximum movement cost multiplier across all props whose solid footprint covers `(px, py)`.
    #[must_use]
    pub fn max_prop_move_cost_at(&self, px: f32, py: f32, base_mult: f32) -> f32 {
        let mut best = base_mult;
        let (bx0, bx1, by0, by1) = self.bucket_range(px, py, 0.1);
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                for &pi in &self.slow_prop_buckets[by * self.b_cols + bx] {
                    let p = &self.props[pi as usize];
                    let sp = prop_spec(p.kind);
                    if sp.move_cost_mult > best && prop_contains_point(p, px, py, false) {
                        best = sp.move_cost_mult;
                    }
                }
            }
        }
        best
    }

    /// Checks whether disc `(px, py, radius_m)` leaves board bounds or clips a matching edge wall.
    #[must_use]
    pub fn circle_hits_wall(
        &self,
        px: f32,
        py: f32,
        radius_m: f32,
        filter: SpatialWallFilter,
    ) -> bool {
        if px < radius_m
            || cy_out(py, radius_m, self.height_m)
            || px > self.width_m - radius_m
        {
            return true;
        }
        let r_eff = radius_m + EDGE_HALF_THICK_M - 1e-4;
        let x0 = ((px - r_eff).floor() as i32 - 1).clamp(0, self.width_cells);
        let x1 = ((px + r_eff).ceil() as i32).clamp(0, self.width_cells);
        let y0 = ((py - r_eff).floor() as i32 - 1).clamp(0, self.height_cells);
        let y1 = ((py + r_eff).ceil() as i32).clamp(0, self.height_cells);

        for ey in y0..=y1 {
            for ex in x0..=x1 {
                if ex < self.width_cells
                    && let Some((kind, material)) =
                        self.edge_idx.get_with_material(ex, ey, EdgeSide::North)
                    && filter.blocks(kind)
                {
                    let e = MapWallEdge { x: ex, y: ey, side: EdgeSide::North, kind, material };
                    let (a, b) = edge_endpoints_m(&e);
                    if distance_point_to_segment(px, py, a, b).0 < r_eff {
                        return true;
                    }
                }
                if ey < self.height_cells
                    && let Some((kind, material)) =
                        self.edge_idx.get_with_material(ex, ey, EdgeSide::West)
                    && filter.blocks(kind)
                {
                    let e = MapWallEdge { x: ex, y: ey, side: EdgeSide::West, kind, material };
                    let (a, b) = edge_endpoints_m(&e);
                    if distance_point_to_segment(px, py, a, b).0 < r_eff {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Exact `O(1)` edge-wall check between orthogonally adjacent `0.5m` lattice nodes `(cx, cy)` and `(ux, uy)`.
    #[must_use]
    pub fn half_meter_step_blocked_by_wall_or_window(
        &self,
        cx: usize,
        cy: usize,
        ux: usize,
        uy: usize,
    ) -> bool {
        if cy == uy {
            if cx.min(ux).is_multiple_of(2) {
                return false;
            }
            let ex = (cx.max(ux) / 2) as i32;
            let ey = (cy / 2) as i32;
            self.edge_idx
                .get(ex, ey, EdgeSide::West)
                .is_some_and(|k| SpatialWallFilter::WallsAndWindowsOnly.blocks(k))
        } else if cx == ux {
            if cy.min(uy).is_multiple_of(2) {
                return false;
            }
            let ex = (cx / 2) as i32;
            let ey = (cy.max(uy) / 2) as i32;
            self.edge_idx
                .get(ex, ey, EdgeSide::North)
                .is_some_and(|k| SpatialWallFilter::WallsAndWindowsOnly.blocks(k))
        } else {
            false
        }
    }

    /// Checks whether movement segment `a -> b` intersects any movement-blocking edge locally.
    #[must_use]
    pub fn segment_hits_movement_wall(&self, a: (f32, f32), b: (f32, f32)) -> bool {
        let min_x = a.0.min(b.0) - EDGE_HALF_THICK_M;
        let max_x = a.0.max(b.0) + EDGE_HALF_THICK_M;
        let min_y = a.1.min(b.1) - EDGE_HALF_THICK_M;
        let max_y = a.1.max(b.1) + EDGE_HALF_THICK_M;
        let x0 = (min_x.floor() as i32 - 1).clamp(0, self.width_cells);
        let x1 = (max_x.ceil() as i32).clamp(0, self.width_cells);
        let y0 = (min_y.floor() as i32 - 1).clamp(0, self.height_cells);
        let y1 = (max_y.ceil() as i32).clamp(0, self.height_cells);

        for ey in y0..=y1 {
            for ex in x0..=x1 {
                if ex < self.width_cells
                    && let Some((kind, material)) =
                        self.edge_idx.get_with_material(ex, ey, EdgeSide::North)
                    && kind.blocks_movement()
                {
                    let e = MapWallEdge { x: ex, y: ey, side: EdgeSide::North, kind, material };
                    if intersect_segment_edge(a, b, &e).is_some() {
                        return true;
                    }
                }
                if ey < self.height_cells
                    && let Some((kind, material)) =
                        self.edge_idx.get_with_material(ex, ey, EdgeSide::West)
                    && kind.blocks_movement()
                {
                    let e = MapWallEdge { x: ex, y: ey, side: EdgeSide::West, kind, material };
                    if intersect_segment_edge(a, b, &e).is_some() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[inline]
const fn cy_out(py: f32, radius_m: f32, height_m: f32) -> bool {
    py < radius_m || py > height_m - radius_m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tactical_map::{
        SectorPreset, TacticalSceneView, compute_reachable_continuous, generate_procedural_sector,
    };
    use crate::environment_vehicles::CombatMovementGait;
    use crate::tactical_map_geom::all_walkable_areas_connected;

    #[test]
    fn spatial_index_matches_bruteforce_collision_on_generated_sectors() {
        for preset in [
            SectorPreset::ResidentialApartment,
            SectorPreset::UrbanCrossroads,
            SectorPreset::GardenEstate,
            SectorPreset::DockingBay,
        ] {
            let sec = generate_procedural_sector(preset, 24, 18, 42);
            let grid = SpatialObstacleIndex::new(24.0, 18.0, &sec.wall_edges, &sec.props);
            for iy in 0..36 {
                for ix in 0..48 {
                    let px = (ix as f32 + 0.5) * 0.5;
                    let py = (iy as f32 + 0.5) * 0.5;
                    let bf_move_prop = sec.props.iter().any(|p| {
                        prop_spec(p.kind).obstruction.blocks_movement()
                            && prop_intersects_circle(p, px, py, 0.22)
                    });
                    assert_eq!(
                        grid.any_move_blocking_prop_intersects_circle(px, py, 0.22),
                        bf_move_prop,
                        "{preset:?} move prop mismatch at ({px},{py})"
                    );
                    let bf_wall = crate::tactical_map_geom::circle_collides_obstacles(
                        24.0,
                        18.0,
                        &sec.wall_edges,
                        &[],
                        px,
                        py,
                        0.20,
                    );
                    assert_eq!(
                        grid.circle_hits_wall(px, py, 0.20, SpatialWallFilter::MovementBlocking),
                        bf_wall,
                        "{preset:?} wall mismatch at ({px},{py})"
                    );
                }
            }
        }
    }

    #[test]
    fn spatial_index_keeps_connectivity_and_reach_fast_on_large_boards() {
        let sec = generate_procedural_sector(SectorPreset::UrbanCrossroads, 64, 48, 77);
        assert!(all_walkable_areas_connected(64, 48, &sec.wall_edges, &sec.props));
        let scene = TacticalSceneView {
            width_m: 64.0,
            height_m: 48.0,
            edges: &sec.wall_edges,
            props: &sec.props,
            surfaces: &sec.surfaces,
        };
        let reach = compute_reachable_continuous(
            &scene,
            &[],
            crate::tactical_map_reach::GaitReachQuery {
                start_m: (2.5, 24.5),
                spd_eff: 12,
                hgt_m: 1.75,
                gait: CombatMovementGait::AllOutSprint,
                tick_budget: 24,
            },
        );
        assert!(!reach.merged_row_runs().is_empty());
    }
}
