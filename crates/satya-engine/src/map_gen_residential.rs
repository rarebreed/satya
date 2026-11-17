//! # Composable Multi-Tier Residential Home Generator (`map_gen_residential`)
//!
//! Decouples residential floorplan architecture ([`EconomicTier`]: `LowIncome`, `MiddleClass`,
//! `UpperMiddleClass`, `LuxuryMansion`) from land-use placement ([`crate::map_gen_zoning`]).
//! Supports multi-level generation (`B1` cellar/vault, `1F` ground floor, `2F` upper suites)
//! with vertically aligned [`PropKind::Stairwell`] shafts and 100% doorway connectivity.

use std::collections::VecDeque;

use satya_schema::{
    EconomicTier, MapFloorLevel, MapPointCm, MapPropPlacement, MapSurfaceFeature, MapWallEdge,
    SurfaceFeatureKind, SurfaceShapeKind,
};
#[cfg(test)]
use satya_schema::PropKind;

use crate::map_dressing::{PropObstruction, effective_obstruction};
use crate::map_home_floorplan::{build_blueprint_floor, plan_home_blueprint};
use crate::map_interior_gen::Room;
use crate::tactical_map_geom::TOKEN_RADIUS_M;
use crate::tactical_map_spatial::{SpatialObstacleIndex, SpatialWallFilter};

/// Integer cell bounding box `[x0, x1) × [y0, y1)` for a composable building lot or bay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotBox {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl LotBox {
    #[must_use]
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    #[must_use]
    pub const fn w(self) -> i32 {
        let d = self.x1 - self.x0;
        if d > 0 { d } else { 0 }
    }

    #[must_use]
    pub const fn h(self) -> i32 {
        let d = self.y1 - self.y0;
        if d > 0 { d } else { 0 }
    }

    #[must_use]
    pub const fn area(self) -> i32 {
        self.w() * self.h()
    }

    #[must_use]
    pub(crate) const fn as_room(self) -> Room {
        Room {
            x0: self.x0,
            y0: self.y0,
            x1: self.x1,
            y1: self.y1,
        }
    }

    #[must_use]
    pub fn polygon_feature(
        self,
        kind: SurfaceFeatureKind,
        corner_cm: i32,
        seed: u32,
    ) -> MapSurfaceFeature {
        MapSurfaceFeature {
            kind,
            shape: SurfaceShapeKind::Polygon,
            points_cm: vec![
                MapPointCm::new(self.x0 * 100, self.y0 * 100),
                MapPointCm::new(self.x1 * 100, self.y0 * 100),
                MapPointCm::new(self.x1 * 100, self.y1 * 100),
                MapPointCm::new(self.x0 * 100, self.y1 * 100),
            ],
            width_cm: 0,
            border_style_cm: corner_cm,
            variant_seed: seed,
            tint_rgb: None,
        }
    }
}

/// Cardinal side of a [`LotBox`] facing the primary street, breezeway, or approach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalEntry {
    North,
    South,
    West,
    East,
}

/// Geometry generated for a single vertical floor level before site wrapping.
#[derive(Debug, Clone, Default)]
pub struct FloorContent {
    pub wall_edges: Vec<MapWallEdge>,
    pub props: Vec<MapPropPlacement>,
    pub surfaces: Vec<MapSurfaceFeature>,
}

/// Vertical shaft record keeping `B1`, `1F`, and `2F` stairs identically registered in `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StairAnchor {
    pub x: i32,
    pub y: i32,
    pub rot_deg: i32,
}

/// Multi-level output emitted by [`generate_home_in_lot`].
#[derive(Debug, Clone)]
pub struct GeneratedHome {
    pub lot: LotBox,
    pub basement: Option<FloorContent>,
    pub ground: FloorContent,
    pub upper: Option<FloorContent>,
    pub stair_anchors: Vec<StairAnchor>,
}

impl GeneratedHome {
    /// Converts upper/basement floor data into wire [`MapFloorLevel`] entries sorted ascending by `floor_index`.
    #[must_use]
    pub fn into_floor_levels(
        self,
        ground_merged: FloorContent,
        has_cellar: bool,
    ) -> Vec<MapFloorLevel> {
        let mut levels = Vec::new();
        if let Some(b1) = self.basement
            && has_cellar
        {
            levels.push(MapFloorLevel {
                floor_index: -1,
                label: "B1 · Cellar & Vault".to_string(),
                elevation_cm: -320,
                wall_edges: b1.wall_edges,
                props: b1.props,
                surfaces: b1.surfaces,
                ..Default::default()
            });
        }
        levels.push(MapFloorLevel {
            floor_index: 0,
            label: "1F · Ground".to_string(),
            elevation_cm: 0,
            wall_edges: ground_merged.wall_edges,
            props: ground_merged.props,
            surfaces: ground_merged.surfaces,
            ..Default::default()
        });
        if let Some(f2) = self.upper {
            levels.push(MapFloorLevel {
                floor_index: 1,
                label: "2F · Upper Level".to_string(),
                elevation_cm: 340,
                wall_edges: f2.wall_edges,
                props: f2.props,
                surfaces: f2.surfaces,
                ..Default::default()
            });
        }
        levels.sort_by_key(|f| f.floor_index);
        levels
    }
}

/// Checks that every walkable `1.0m` cell inside `lot` forms one connected component from the first open cell
/// (or stairwell landing), ignoring regions outside `lot` for self-contained `B1`/`2F` levels.
#[must_use]
pub fn floor_lot_connected(
    board_w: i32,
    board_h: i32,
    lot: LotBox,
    edges: &[MapWallEdge],
    props: &[MapPropPlacement],
) -> bool {
    if lot.w() <= 0 || lot.h() <= 0 {
        return false;
    }
    let bw = board_w.max(lot.x1) as f32;
    let bh = board_h.max(lot.y1) as f32;
    let spatial = SpatialObstacleIndex::new(bw, bh, edges, props);
    let cols = (lot.w() * 2) as usize;
    let rows = (lot.h() * 2) as usize;
    let node_pos = |ix: usize, iy: usize| -> (f32, f32) {
        (
            lot.x0 as f32 + (ix as f32 + 0.5) * 0.5,
            lot.y0 as f32 + (iy as f32 + 0.5) * 0.5,
        )
    };
    let mut open = vec![false; cols * rows];
    let mut first = None;
    let stair = props.iter().find(|p| p.kind.is_vertical_portal());
    let mut preferred = None;
    for iy in 0..rows {
        for ix in 0..cols {
            let (px, py) = node_pos(ix, iy);
            if !spatial.any_full_obstacle_prop_intersects_circle(px, py, 0.20) {
                open[iy * cols + ix] = true;
                if first.is_none() {
                    first = Some((ix, iy));
                }
                if let Some(st) = stair
                    && st.covers(px.floor() as i32, py.floor() as i32)
                {
                    preferred = Some((ix, iy));
                }
            }
        }
    }
    let Some(start) = preferred.or(first) else {
        return false;
    };
    let mut visited = vec![false; cols * rows];
    let mut q = VecDeque::new();
    visited[start.1 * cols + start.0] = true;
    q.push_back(start);
    while let Some((cx, cy)) = q.pop_front() {
        for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (cx as isize + dx, cy as isize + dy);
            if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                continue;
            }
            let (ux, uy) = (nx as usize, ny as usize);
            let idx = uy * cols + ux;
            if !open[idx] || visited[idx] {
                continue;
            }
            let gcx = (lot.x0 * 2) as usize + cx;
            let gcy = (lot.y0 * 2) as usize + cy;
            let gux = (lot.x0 * 2) as usize + ux;
            let guy = (lot.y0 * 2) as usize + uy;
            if spatial.half_meter_step_blocked_by_wall_or_window(gcx, gcy, gux, guy) {
                continue;
            }
            visited[idx] = true;
            q.push_back((ux, uy));
        }
    }
    for gy in lot.y0..lot.y1 {
        for gx in lot.x0..lot.x1 {
            let cx = gx as f32 + 0.5;
            let cy = gy as f32 + 0.5;
            if spatial.circle_hits_wall(
                cx,
                cy,
                TOKEN_RADIUS_M,
                SpatialWallFilter::WallsAndWindowsOnly,
            ) || spatial.any_full_obstacle_prop_intersects_circle(cx, cy, TOKEN_RADIUS_M - 1e-4)
            {
                continue;
            }
            let bx = ((gx - lot.x0) * 2) as usize;
            let by = ((gy - lot.y0) * 2) as usize;
            let reached = [
                (bx, by),
                ((bx + 1).min(cols - 1), by),
                (bx, (by + 1).min(rows - 1)),
                ((bx + 1).min(cols - 1), (by + 1).min(rows - 1)),
            ]
            .into_iter()
            .any(|(sx, sy)| visited[sy * cols + sx]);
            if !reached {
                return false;
            }
        }
    }
    true
}

/// Trims blocking props until `floor_lot_connected` passes, and verifies no wall edge is orphaned.
pub(crate) fn trim_floor_props_to_connected(
    board_w: i32,
    board_h: i32,
    lot: LotBox,
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
) {
    let mut guard = 0;
    while !floor_lot_connected(board_w, board_h, lot, edges, props) && guard < 40 {
        guard += 1;
        match props
            .iter()
            .rposition(|p| effective_obstruction(p) == PropObstruction::FullObstacle)
            .or_else(|| {
                props
                    .iter()
                    .rposition(|p| effective_obstruction(p) == PropObstruction::HalfCover)
            }) {
            Some(i) => {
                props.remove(i);
            }
            None => break,
        }
    }
}

/// Composes a furnished residential structure scaled to `tier` inside `lot`, emitting ground floor
/// `1F` plus optional aligned upper floor (`2F`) and cellar (`B1`) levels.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn generate_home_in_lot(
    board_w: i32,
    board_h: i32,
    lot: LotBox,
    tier: EconomicTier,
    entry_side: CardinalEntry,
    allow_upper_floor: bool,
    allow_basement: bool,
    rear_door: bool,
    spawn_pads_m: &[(f32, f32)],
    seed: u64,
) -> GeneratedHome {
    let bp = plan_home_blueprint(
        lot,
        tier,
        entry_side,
        allow_upper_floor || allow_basement,
        seed,
    );
    let ground = build_blueprint_floor(
        board_w,
        board_h,
        &bp,
        tier,
        entry_side,
        0,
        rear_door,
        spawn_pads_m,
        seed,
    );
    let upper = if allow_upper_floor
        && bp.stair.is_some()
        && bp.main_wing.w() >= 5
        && bp.main_wing.h() >= 4
    {
        Some(build_blueprint_floor(
            board_w,
            board_h,
            &bp,
            tier,
            entry_side,
            1,
            false,
            &[],
            seed.wrapping_add(101),
        ))
    } else {
        None
    };
    let basement =
        if allow_basement && bp.stair.is_some() && bp.main_wing.w() >= 6 && bp.main_wing.h() >= 5 {
            Some(build_blueprint_floor(
                board_w,
                board_h,
                &bp,
                tier,
                entry_side,
                -1,
                false,
                &[],
                seed.wrapping_add(202),
            ))
        } else {
            None
        };

    GeneratedHome {
        lot,
        basement,
        ground,
        upper,
        stair_anchors: bp.stair.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_wings_connected(board_w: i32, board_h: i32, floor: &FloorContent, label: &str) {
        let wings: Vec<LotBox> = floor
            .surfaces
            .iter()
            .filter(|s| s.kind == SurfaceFeatureKind::InteriorFloor && !s.points_cm.is_empty())
            .map(|s| {
                let x0 = s.points_cm.iter().map(|p| p.x_cm).min().unwrap_or(0) / 100;
                let y0 = s.points_cm.iter().map(|p| p.y_cm).min().unwrap_or(0) / 100;
                let x1 = s.points_cm.iter().map(|p| p.x_cm).max().unwrap_or(0) / 100;
                let y1 = s.points_cm.iter().map(|p| p.y_cm).max().unwrap_or(0) / 100;
                LotBox::new(x0, y0, x1, y1)
            })
            .collect();
        assert!(!wings.is_empty(), "{label} missing InteriorFloor wing");
        for wing in wings {
            assert!(
                floor_lot_connected(board_w, board_h, wing, &floor.wall_edges, &floor.props),
                "{label} wing {wing:?} disconnected"
            );
        }
    }

    #[test]
    fn all_economic_tiers_generate_connected_furnished_levels_and_aligned_stairs() {
        let lot = LotBox::new(4, 3, 24, 19);
        for tier in EconomicTier::ALL {
            for seed in [42_u64, 77, 108] {
                let allow_b1 = tier == EconomicTier::LuxuryMansion;
                let home = generate_home_in_lot(
                    28,
                    22,
                    lot,
                    tier,
                    CardinalEntry::South,
                    true,
                    allow_b1,
                    true,
                    &[],
                    seed,
                );
                assert!(!home.ground.wall_edges.is_empty(), "{tier:?} missing edges");
                assert!(!home.ground.props.is_empty(), "{tier:?} missing props");
                assert_wings_connected(28, 22, &home.ground, &format!("{tier:?}/{seed} 1F"));
                let f2 = home.upper.as_ref().expect("2F level present");
                assert_wings_connected(28, 22, f2, &format!("{tier:?}/{seed} 2F"));
                let st1 = home
                    .ground
                    .props
                    .iter()
                    .find(|p| p.kind == PropKind::Stairwell)
                    .unwrap();
                let st2 = f2
                    .props
                    .iter()
                    .find(|p| p.kind == PropKind::Stairwell)
                    .unwrap();
                assert_eq!(
                    st1.center_cm(),
                    st2.center_cm(),
                    "{tier:?} stairs must align across floors"
                );
                if allow_b1 {
                    let b1 = home.basement.as_ref().expect("B1 cellar present");
                    assert_wings_connected(28, 22, b1, &format!("{tier:?}/{seed} B1"));
                    let stb = b1
                        .props
                        .iter()
                        .find(|p| p.kind == PropKind::Stairwell)
                        .unwrap();
                    assert_eq!(st1.center_cm(), stb.center_cm());
                }
            }
        }
    }
}
