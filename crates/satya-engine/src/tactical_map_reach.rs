//! Continuous `0.25m` Any-Angle Gait Reachability (`tactical_map_reach`)
//!
//! Computes geodesic reachability over continuous metric obstacles, wall edges, prop footprints,
//! and vector [`MapSurfaceFeature`] difficulty tiers (water channels, ditches, trenches, bridges,
//! and planted beds) using 16-directional `0.25m` Dijkstra propagation.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use satya_schema::{
    MapSurfaceFeature, SurfaceFeatureKind, SurfaceShapeKind,
};
use serde::{Deserialize, Serialize};

use crate::environment_vehicles::{CombatMovementGait, compute_stride_and_sprint_velocity};
use crate::map_surface_geom::{
    distance_point_to_polyline_m, point_in_ring_m, sample_open_catmull_rom_m, sampled_closed_ring_m,
};
use crate::tactical_map::TacticalSceneView;
use crate::tactical_map_geom::{NAV_STEP_M, TOKEN_RADIUS_M};
use crate::tactical_map_spatial::{SpatialObstacleIndex, SpatialWallFilter};

/// Merged horizontal reachable contour strip (`0.25m` band) for crisp SVG and WebGPU overlays.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReachableStrip {
    pub y_m: f32,
    pub x_min_m: f32,
    pub x_max_m: f32,
    pub step_m: f32,
    pub min_ticks: u32,
}

/// Continuous `0.25m` Dijkstra gait reachability scalar field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReachableField {
    pub step_m: f32,
    pub cols: usize,
    pub rows: usize,
    pub m_per_tick: f32,
    /// Geodesic travel distance in millimeters (`i32::MAX` when unreachable).
    pub cost_mm: Vec<i32>,
}

impl ReachableField {
    /// Ticks required to reach `(x_m, y_m)`, or `None` if outside the movement envelope.
    #[must_use]
    pub fn ticks_at_m(&self, x_m: f32, y_m: f32) -> Option<u32> {
        let dist_m = self.distance_m_at(x_m, y_m)?;
        Some((f64::from(dist_m) / f64::from(self.m_per_tick)).ceil().max(1.0) as u32)
    }

    /// Geodesic travel distance in meters at `(x_m, y_m)`, or `None` if unreachable.
    #[must_use]
    pub fn distance_m_at(&self, x_m: f32, y_m: f32) -> Option<f32> {
        if self.cols == 0 || self.rows == 0 || x_m < 0.0 || y_m < 0.0 {
            return None;
        }
        let ix = (x_m / self.step_m).floor() as usize;
        let iy = (y_m / self.step_m).floor() as usize;
        if ix >= self.cols || iy >= self.rows {
            return None;
        }
        let c_mm = self.cost_mm[iy * self.cols + ix];
        if c_mm == i32::MAX {
            None
        } else {
            Some(((c_mm as f32 / 1000.0) * 100.0).round() / 100.0)
        }
    }

    /// Merges horizontally contiguous reachable strips sharing the same tick tier for fast vector drawing.
    #[must_use]
    pub fn merged_row_runs(&self) -> Vec<ReachableStrip> {
        let mut strips = Vec::new();
        for iy in 0..self.rows {
            let y_m = iy as f32 * self.step_m;
            let mut active: Option<(f32, f32, u32)> = None;
            for ix in 0..self.cols {
                let c_mm = self.cost_mm[iy * self.cols + ix];
                let ticks = if c_mm == i32::MAX || c_mm == 0 {
                    None
                } else {
                    Some(((f64::from(c_mm) / 1000.0) / f64::from(self.m_per_tick)).ceil().max(1.0) as u32)
                };
                match (active, ticks) {
                    (Some((x0, x1, t_cur)), Some(t)) if t == t_cur => {
                        active = Some((x0, x1 + self.step_m, t_cur));
                    }
                    (Some((x0, x1, t_cur)), next_t) => {
                        strips.push(ReachableStrip {
                            y_m,
                            x_min_m: x0,
                            x_max_m: x1,
                            step_m: self.step_m,
                            min_ticks: t_cur,
                        });
                        let x_start = ix as f32 * self.step_m;
                        active = next_t.map(|t| (x_start, x_start + self.step_m, t));
                    }
                    (None, Some(t)) => {
                        let x_start = ix as f32 * self.step_m;
                        active = Some((x_start, x_start + self.step_m, t));
                    }
                    (None, None) => {}
                }
            }
            if let Some((x0, x1, t_cur)) = active {
                strips.push(ReachableStrip {
                    y_m,
                    x_min_m: x0,
                    x_max_m: x1,
                    step_m: self.step_m,
                    min_ticks: t_cur,
                });
            }
        }
        strips
    }
}

#[derive(Debug, Clone, Copy)]
struct DijkstraNode {
    cost_mm: i32,
    ix: usize,
    iy: usize,
}

impl PartialEq for DijkstraNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost_mm == other.cost_mm && self.ix == other.ix && self.iy == other.iy
    }
}

impl Eq for DijkstraNode {}

impl Ord for DijkstraNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost_mm
            .cmp(&self.cost_mm)
            .then_with(|| self.iy.cmp(&other.iy))
            .then_with(|| self.ix.cmp(&other.ix))
    }
}

impl PartialOrd for DijkstraNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Context parameters for computing a character's continuous any-angle reachable movement envelope.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GaitReachQuery {
    pub start_m: (f32, f32),
    pub spd_eff: i32,
    pub hgt_m: f64,
    pub gait: CombatMovementGait,
    pub tick_budget: u32,
}

enum PreSampledShape {
    Ribbon { pts: Vec<(f32, f32)>, half_w: f32 },
    Ring { pts: Vec<(f32, f32)> },
}

struct PreSampledSurface {
    kind: SurfaceFeatureKind,
    prio: u8,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    shape: PreSampledShape,
}

impl PreSampledSurface {
    fn from_feature(s: &MapSurfaceFeature) -> Option<Self> {
        if s.points_cm.is_empty() {
            return None;
        }
        let prio = s.kind.layer_priority();
        match s.shape {
            SurfaceShapeKind::RibbonSpline => {
                let raw: Vec<(f32, f32)> = s.points_cm.iter().map(|p| p.to_m()).collect();
                let pts = sample_open_catmull_rom_m(&raw, 6);
                let half_w = (s.width_cm.max(10) as f32) / 200.0;
                let (mut min_x, mut min_y, mut max_x, mut max_y) =
                    (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
                for &(x, y) in &pts {
                    min_x = min_x.min(x - half_w);
                    min_y = min_y.min(y - half_w);
                    max_x = max_x.max(x + half_w);
                    max_y = max_y.max(y + half_w);
                }
                Some(Self {
                    kind: s.kind,
                    prio,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    shape: PreSampledShape::Ribbon { pts, half_w },
                })
            }
            SurfaceShapeKind::OrganicClosed | SurfaceShapeKind::Polygon => {
                let pts = sampled_closed_ring_m(s);
                if pts.len() < 3 {
                    return None;
                }
                let (mut min_x, mut min_y, mut max_x, mut max_y) =
                    (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
                for &(x, y) in &pts {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
                Some(Self {
                    kind: s.kind,
                    prio,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    shape: PreSampledShape::Ring { pts },
                })
            }
        }
    }

    fn contains(&self, px: f32, py: f32) -> bool {
        if px < self.min_x || px > self.max_x || py < self.min_y || py > self.max_y {
            return false;
        }
        match &self.shape {
            PreSampledShape::Ribbon { pts, half_w } => {
                distance_point_to_polyline_m(px, py, pts) <= *half_w
            }
            PreSampledShape::Ring { pts } => point_in_ring_m(px, py, pts),
        }
    }
}

/// Computes a smooth `0.25m` 16-directional metric [`ReachableField`] around `query.start_m`.
#[must_use]
pub fn compute_reachable_continuous(
    scene: &TacticalSceneView<'_>,
    occupied_tokens_m: &[(f32, f32)],
    query: GaitReachQuery,
) -> ReachableField {
    let step_m = NAV_STEP_M;
    let cols = (scene.width_m / step_m).floor().max(0.0) as usize;
    let rows = (scene.height_m / step_m).floor().max(0.0) as usize;
    let sprint = compute_stride_and_sprint_velocity(query.spd_eff, query.hgt_m);
    let m_per_tick = (sprint.v_sprint_m_per_tick * query.gait.sprint_fraction()).max(0.05) as f32;
    if cols == 0 || rows == 0 {
        return ReachableField {
            step_m,
            cols,
            rows,
            m_per_tick,
            cost_mm: Vec::new(),
        };
    }

    let max_budget_mm = (f64::from(m_per_tick) * f64::from(query.tick_budget) * 1000.0).round() as i32;
    let total = cols * rows;
    let mut walkable = vec![false; total];
    let mut cost_mult = vec![1.0_f32; total];
    let sampled_surfaces: Vec<PreSampledSurface> = scene
        .surfaces
        .iter()
        .filter_map(PreSampledSurface::from_feature)
        .collect();

    let sx = ((query.start_m.0 / step_m).floor() as isize).clamp(0, cols as isize - 1) as usize;
    let sy = ((query.start_m.1 / step_m).floor() as isize).clamp(0, rows as isize - 1) as usize;
    let max_reach_m = (max_budget_mm as f32 / 800.0) + 1.5;
    let ix_min = (((query.start_m.0 - max_reach_m) / step_m).floor() as isize)
        .clamp(0, cols as isize - 1) as usize;
    let ix_max = (((query.start_m.0 + max_reach_m) / step_m).ceil() as isize)
        .clamp(0, cols as isize - 1) as usize;
    let iy_min = (((query.start_m.1 - max_reach_m) / step_m).floor() as isize)
        .clamp(0, rows as isize - 1) as usize;
    let iy_max = (((query.start_m.1 + max_reach_m) / step_m).ceil() as isize)
        .clamp(0, rows as isize - 1) as usize;
    let ix_min = ix_min.min(sx);
    let ix_max = ix_max.max(sx);
    let iy_min = iy_min.min(sy);
    let iy_max = iy_max.max(sy);

    let node_pos = |ix: usize, iy: usize| -> (f32, f32) {
        ((ix as f32 + 0.5) * step_m, (iy as f32 + 0.5) * step_m)
    };
    let spatial = SpatialObstacleIndex::new(scene.width_m, scene.height_m, scene.edges, scene.props);

    for iy in iy_min..=iy_max {
        for ix in ix_min..=ix_max {
            let (px, py) = node_pos(ix, iy);
            let occupied = occupied_tokens_m
                .iter()
                .any(|&(tx, ty)| (px - tx).powi(2) + (py - ty).powi(2) < (TOKEN_RADIUS_M * 1.65).powi(2));
            let idx = iy * cols + ix;
            walkable[idx] = !occupied
                && !spatial.any_move_blocking_prop_intersects_circle(px, py, 0.22)
                && !spatial.circle_hits_wall(px, py, 0.20, SpatialWallFilter::MovementBlocking);
            if walkable[idx] {
                let mut best_surf: Option<SurfaceFeatureKind> = None;
                let mut best_prio = 0u8;
                for s in &sampled_surfaces {
                    if (best_surf.is_none() || s.prio >= best_prio) && s.contains(px, py) {
                        best_surf = Some(s.kind);
                        best_prio = s.prio;
                    }
                }
                let mut m = best_surf.map_or(1.0, SurfaceFeatureKind::move_cost_mult);
                if best_surf != Some(SurfaceFeatureKind::BridgeDeck) {
                    m = spatial.max_prop_move_cost_at(px, py, m);
                }
                cost_mult[idx] = m;
            }
        }
    }

    let mut cost_mm = vec![i32::MAX; total];
    cost_mm[sy * cols + sx] = 0;

    let mut heap = BinaryHeap::new();
    heap.push(DijkstraNode { cost_mm: 0, ix: sx, iy: sy });

    // 16-directional lattice neighborhood: axial (250mm), diagonal (354mm), knight (559mm).
    let neighbors: [(isize, isize, i32); 16] = [
        (1, 0, 250),
        (-1, 0, 250),
        (0, 1, 250),
        (0, -1, 250),
        (1, 1, 354),
        (1, -1, 354),
        (-1, 1, 354),
        (-1, -1, 354),
        (2, 1, 559),
        (2, -1, 559),
        (-2, 1, 559),
        (-2, -1, 559),
        (1, 2, 559),
        (-1, 2, 559),
        (1, -2, 559),
        (-1, -2, 559),
    ];

    while let Some(node) = heap.pop() {
        let cur_idx = node.iy * cols + node.ix;
        if node.cost_mm > cost_mm[cur_idx] {
            continue;
        }
        let a = node_pos(node.ix, node.iy);
        for &(dx, dy, base_mm) in &neighbors {
            let nx_i = node.ix as isize + dx;
            let ny_i = node.iy as isize + dy;
            if nx_i < 0 || ny_i < 0 || nx_i >= cols as isize || ny_i >= rows as isize {
                continue;
            }
            let (nx, ny) = (nx_i as usize, ny_i as usize);
            let nidx = ny * cols + nx;
            if !walkable[nidx] {
                continue;
            }
            if dx.abs() == 2 || dy.abs() == 2 {
                let mx = (node.ix as isize + dx / 2) as usize;
                let my = (node.iy as isize + dy / 2) as usize;
                if !walkable[my * cols + mx] {
                    continue;
                }
            }
            let b = node_pos(nx, ny);
            if spatial.segment_hits_movement_wall(a, b) {
                continue;
            }
            let step_cost = ((base_mm as f32) * cost_mult[nidx]).round() as i32;
            let next_cost = node.cost_mm + step_cost;
            if next_cost > max_budget_mm {
                continue;
            }
            if next_cost < cost_mm[nidx] {
                cost_mm[nidx] = next_cost;
                heap.push(DijkstraNode { cost_mm: next_cost, ix: nx, iy: ny });
            }
        }
    }

    ReachableField { step_m, cols, rows, m_per_tick, cost_mm }
}
