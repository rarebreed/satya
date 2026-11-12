//! # Thin Edge Walls: Index, Movement Blocking & Exact Ray Traversal (`tactical_map_edges`)
//!
//! Walls, doors, and windows in [`satya_schema::MapWallEdge`] sit on **cell boundaries** rather than
//! filling whole cells. This module provides:
//! - [`WallEdgeIndex`]: an `O(1)` lookup of North/West edges built from the wire list.
//! - [`WallEdgeIndex::step_blocked`]: an orthogonal or diagonal step test. A diagonal step is blocked only if
//!   **both** L-shaped routes through the neighboring cells cross a blocking edge, so tokens can't slip
//!   through wall junctions but can round a free-standing wall end.
//! - [`WallEdgeIndex::trace_obstruction`]: an exact Amanatides–Woo grid traversal of a sight ray in
//!   continuous cell space. It reports sight-blocking crossings and Low Cover (window) crossings, and handles
//!   lattice-corner crossings with the same L-route rule.
//!
//! Consumed by [`crate::tactical_map`] LoS/cover and gait-reach Dijkstra through [`crate::TacticalGridView`].

use satya_schema::{EdgeSide, MapWallEdge, WallEdgeKind};

/// Dense North/West edge lookup for a `width × height` board.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WallEdgeIndex {
    width: i32,
    height: i32,
    /// `width × (height + 1)` North edges, row-major by `y`.
    north: Vec<Option<WallEdgeKind>>,
    /// `(width + 1) × height` West edges, row-major by `y`.
    west: Vec<Option<WallEdgeKind>>,
}

/// Result of tracing a sight ray against edge walls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EdgeObstruction {
    pub blocked: bool,
    pub half_cover: bool,
}

impl WallEdgeIndex {
    /// Builds the index from wire edges. Records outside the board are ignored, and later records override earlier ones.
    #[must_use]
    pub fn from_edges(width: i32, height: i32, edges: &[MapWallEdge]) -> Self {
        let (w, h) = (width.max(0), height.max(0));
        let mut idx = Self {
            width: w,
            height: h,
            north: vec![None; (w * (h + 1)) as usize],
            west: vec![None; ((w + 1) * h) as usize],
        };
        for e in edges {
            idx.set(e.x, e.y, e.side, Some(e.kind));
        }
        idx
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.north.iter().chain(self.west.iter()).all(Option::is_none)
    }

    fn slot(&self, x: i32, y: i32, side: EdgeSide) -> Option<usize> {
        match side {
            EdgeSide::North if x >= 0 && x < self.width && y >= 0 && y <= self.height => {
                Some((y * self.width + x) as usize)
            }
            EdgeSide::West if x >= 0 && x <= self.width && y >= 0 && y < self.height => {
                Some((y * (self.width + 1) + x) as usize)
            }
            _ => None,
        }
    }

    /// Sets or clears one edge (out-of-board addresses are ignored).
    pub fn set(&mut self, x: i32, y: i32, side: EdgeSide, kind: Option<WallEdgeKind>) {
        if let Some(i) = self.slot(x, y, side) {
            match side {
                EdgeSide::North => self.north[i] = kind,
                EdgeSide::West => self.west[i] = kind,
            }
        }
    }

    /// Edge occupying `(x, y, side)`, if any.
    #[must_use]
    pub fn get(&self, x: i32, y: i32, side: EdgeSide) -> Option<WallEdgeKind> {
        let i = self.slot(x, y, side)?;
        match side {
            EdgeSide::North => self.north[i],
            EdgeSide::West => self.west[i],
        }
    }

    /// Edge separating two orthogonally adjacent cells `a` and `b` (`None` if not adjacent or open).
    #[must_use]
    pub fn between(&self, a: (i32, i32), b: (i32, i32)) -> Option<WallEdgeKind> {
        if a.1 == b.1 && (a.0 - b.0).abs() == 1 {
            self.get(a.0.max(b.0), a.1, EdgeSide::West)
        } else if a.0 == b.0 && (a.1 - b.1).abs() == 1 {
            self.get(a.0, a.1.max(b.1), EdgeSide::North)
        } else {
            None
        }
    }

    /// Whether a step from `from` by `(dx, dy)` (each `-1..=1`) crosses an edge matching `blocks`.
    #[must_use]
    pub fn step_blocked(&self, from: (i32, i32), dx: i32, dy: i32, blocks: impl Fn(WallEdgeKind) -> bool) -> bool {
        let hit = |a: (i32, i32), b: (i32, i32)| self.between(a, b).is_some_and(&blocks);
        let to = (from.0 + dx, from.1 + dy);
        if dx == 0 || dy == 0 {
            return hit(from, to);
        }
        let via_x = (from.0 + dx, from.1);
        let via_y = (from.0, from.1 + dy);
        (hit(from, via_x) || hit(via_x, to)) && (hit(from, via_y) || hit(via_y, to))
    }

    /// Traces a ray in continuous cell space (`cell (x, y)` spans `[x, x+1) × [y, y+1)`) from `start` to `end`.
    /// Crossings out of `attacker_cell` never impose Low Cover, since the shooter braces on their own sill.
    #[must_use]
    pub fn trace_obstruction(&self, start: (f32, f32), end: (f32, f32), attacker_cell: (i32, i32)) -> EdgeObstruction {
        let mut out = EdgeObstruction::default();
        if self.is_empty() {
            return out;
        }
        const EPS: f32 = 1e-5;
        let (dx, dy) = (end.0 - start.0, end.1 - start.1);
        let (mut cx, mut cy) = (start.0.floor() as i32, start.1.floor() as i32);
        let (ex, ey) = (end.0.floor() as i32, end.1.floor() as i32);
        let step_x = (dx > EPS) as i32 - (dx < -EPS) as i32;
        let step_y = (dy > EPS) as i32 - (dy < -EPS) as i32;
        let axis = |s: i32, d: f32, p: f32, c: i32| -> (f32, f32) {
            match s {
                1 => (((c + 1) as f32 - p) / d, 1.0 / d),
                -1 => ((p - c as f32) / -d, 1.0 / -d),
                _ => (f32::INFINITY, f32::INFINITY),
            }
        };
        let (mut t_max_x, t_delta_x) = axis(step_x, dx, start.0, cx);
        let (mut t_max_y, t_delta_y) = axis(step_y, dy, start.1, cy);
        let max_steps = 2 * (self.width + self.height) + 8;
        for _ in 0..max_steps {
            if (cx, cy) == (ex, ey) || t_max_x.min(t_max_y) > 1.0 + EPS {
                break;
            }
            let from = (cx, cy);
            let (sx, sy) = if (t_max_x - t_max_y).abs() < EPS {
                t_max_x += t_delta_x;
                t_max_y += t_delta_y;
                (step_x, step_y)
            } else if t_max_x < t_max_y {
                t_max_x += t_delta_x;
                (step_x, 0)
            } else {
                t_max_y += t_delta_y;
                (0, step_y)
            };
            if self.step_blocked(from, sx, sy, WallEdgeKind::blocks_sight) {
                out.blocked = true;
                return out;
            }
            if from != attacker_cell
                && self.step_blocked(from, sx, sy, |k| k.blocks_sight() || k.provides_half_cover())
            {
                out.half_cover = true;
            }
            cx += sx;
            cy += sy;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::MaterialClass;

    fn edge(x: i32, y: i32, side: EdgeSide, kind: WallEdgeKind) -> MapWallEdge {
        MapWallEdge { x, y, side, kind, material: MaterialClass::Drywall }
    }

    #[test]
    fn steps_respect_walls_doors_and_corner_junctions() {
        // Vertical wall on the West edges of column 3 for rows 0..4, with a door at row 2.
        let mut edges: Vec<MapWallEdge> = (0..4).map(|y| edge(3, y, EdgeSide::West, WallEdgeKind::Wall)).collect();
        edges[2].kind = WallEdgeKind::DoorClosed;
        let mut idx = WallEdgeIndex::from_edges(6, 5, &edges);
        assert!(idx.step_blocked((2, 1), 1, 0, WallEdgeKind::blocks_movement));
        assert!(idx.step_blocked((2, 2), 1, 0, WallEdgeKind::blocks_movement));
        idx.set(3, 2, EdgeSide::West, Some(WallEdgeKind::DoorOpen));
        assert!(!idx.step_blocked((2, 2), 1, 0, WallEdgeKind::blocks_movement));
        // Diagonal across a continuous wall is blocked; rounding the free wall end at row 4 is allowed.
        assert!(idx.step_blocked((2, 0), 1, 1, WallEdgeKind::blocks_movement));
        assert!(!idx.step_blocked((2, 4), 1, -1, WallEdgeKind::blocks_movement));
        assert!(!idx.step_blocked((2, 3), 1, 1, WallEdgeKind::blocks_movement));
    }

    #[test]
    fn ray_traversal_detects_walls_windows_and_corner_cases() {
        let edges = vec![
            edge(3, 1, EdgeSide::West, WallEdgeKind::Wall),
            edge(3, 2, EdgeSide::West, WallEdgeKind::Window),
            // ┌ corner at lattice point (6, 6)
            edge(6, 6, EdgeSide::North, WallEdgeKind::Wall),
            edge(6, 6, EdgeSide::West, WallEdgeKind::Wall),
        ];
        let idx = WallEdgeIndex::from_edges(10, 10, &edges);
        let wall = idx.trace_obstruction((1.5, 1.5), (5.5, 1.5), (1, 1));
        assert!(wall.blocked);
        let window = idx.trace_obstruction((1.5, 2.5), (5.5, 2.5), (1, 2));
        assert!(!window.blocked && window.half_cover);
        let braced = idx.trace_obstruction((2.5, 2.5), (5.5, 2.5), (2, 2));
        assert!(!braced.blocked && !braced.half_cover, "shooter braced on their own sill");
        // Diagonal through the ┌ vertex into the room is blocked; the tangent diagonal is not.
        assert!(idx.trace_obstruction((5.5, 5.5), (6.5, 6.5), (5, 5)).blocked);
        assert!(!idx.trace_obstruction((5.5, 6.5), (6.5, 5.5), (5, 6)).blocked);
        assert!(!WallEdgeIndex::default().trace_obstruction((0.5, 0.5), (3.5, 3.5), (0, 0)).blocked);
    }
}
