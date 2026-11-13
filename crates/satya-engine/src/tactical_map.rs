//! # Continuous Metric Tactical Map, Procedural Sectors, Analytical LoS/Cover & Gait Reach
//!
//! Deterministic spatial geometry engine shared across `satya-engine`, `satya-signaling`, and `satya-web`:
//! - **Procedural Metric Sector Templates**: [`SectorPreset`] (`ResidentialApartment`, `OfficeFloor`,
//!   `CornerBar`, `UrbanCrossroads`, `WoodedPark`, `GardenEstate`, `WoodlandCreek`, `RoadsideDitch`,
//!   `DockingBay`, `MedLabSector`, `DerelictCorridor`, `OrbitalAirlock`) — all emitting thin
//!   [`MapWallEdge`]s and `360°` oriented multi-cell props (no tile matrix).
//! - **Analytical Raycast & Corner-Peek Cover**: [`evaluate_los_continuous`] computing metric range
//!   (`distance_m`), [`CoverGrade`] (`Clear`, `Concealment`, `LowCover`, `HighCover`, `Blocked`), and
//!   recommended $\Delta_{\text{ext}}$.
//! - **Continuous `0.25m` Any-Angle Gait Reachability**: [`compute_reachable_continuous`] coupling
//!   [`CombatMovementGait`] and [`compute_stride_and_sprint_velocity`] to `16`-directional `0.25m`
//!   Dijkstra pathing merged into smooth [`ReachableStrip`] contours.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use satya_schema::{
    CoverGrade, GmMapUpdateRequest, MapPropPlacement, MapWallEdge, MessageVisibility,
    TokenPlacement, TokenVisualSpec, WallEdgeKind,
};
use serde::{Deserialize, Serialize};

use crate::define_coded_enum;
use crate::environment_vehicles::{CombatMovementGait, compute_stride_and_sprint_velocity};
use crate::map_dressing::{prop_spec, PropObstruction};
use crate::map_interior_gen::{InteriorStyle, generate_furnished_interior};
use crate::map_outdoor_gen::{OutdoorStyle, generate_outdoor_sector};
use crate::map_sector_gen::{FacilitySectorStyle, generate_facility_sector};
pub use crate::tactical_map_edges::{EdgeObstruction, WallEdgeIndex};
use crate::tactical_map_geom::{
    circle_collides_obstacles, intersect_segment_edge, intersect_segment_prop, prop_contains_point,
    prop_intersects_circle, NAV_STEP_M, TOKEN_RADIUS_M,
};

/// Borrowed view of everything continuous spatial rules need: board extents in meters,
/// thin wall/door/window edges, and `360°` oriented props.
#[derive(Debug, Clone, Copy)]
pub struct TacticalSceneView<'a> {
    pub width_m: f32,
    pub height_m: f32,
    pub edges: &'a [MapWallEdge],
    pub props: &'a [MapPropPlacement],
}

impl<'a> TacticalSceneView<'a> {
    /// Constructs a [`TacticalSceneView`] from a wire [`GmMapUpdateRequest`].
    #[must_use]
    pub fn from_map(map: &'a GmMapUpdateRequest) -> Self {
        let (width_m, height_m) = map.board_size_m();
        Self {
            width_m,
            height_m,
            edges: &map.wall_edges,
            props: &map.props,
        }
    }
}

/// Builds the [`WallEdgeIndex`] for legacy helpers needing per-cell edge lookups.
#[must_use]
pub fn wall_edge_index_for_map(map: &GmMapUpdateRequest) -> WallEdgeIndex {
    WallEdgeIndex::from_edges(map.grid_width, map.grid_height, &map.wall_edges)
}

define_coded_enum! {
    /// Canonical procedural sector layout presets for `1.0m` tactical battlemaps.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum SectorPreset [count = 12] {
        #[default]
        ResidentialApartment => {
            code: "residential_apartment",
            label: "Residential Apartment",
            subtitle: "Furnished multi-room apartment with drywall partitions, doors & windows",
            aliases: ["apartment", "flat", "house", "residential"],
        },
        OfficeFloor => {
            code: "office_floor",
            label: "Corporate Office Suite",
            subtitle: "Open-plan office bullpen with glass conference room, reception lounge & server closet",
            aliases: ["office", "corporate", "agency", "commercial", "suite"],
        },
        CornerBar => {
            code: "corner_bar",
            label: "Neighborhood Corner Bar",
            subtitle: "Neighborhood bar with a long counter, booths, pool table & back rooms",
            aliases: ["bar", "pub", "tavern", "saloon"],
        },
        UrbanCrossroads => {
            code: "urban_crossroads",
            label: "Urban Street Crossroads",
            subtitle: "Four-way city intersection with furnished corner storefronts, sidewalks & vehicle cover",
            aliases: ["urban", "street", "city", "crossroads"],
        },
        WoodedPark => {
            code: "wooded_park",
            label: "Wooded Park & Trail",
            subtitle: "Tree-lined municipal park with flagstone plaza, hedges, boulders & roadside pullout",
            aliases: ["park", "woods", "forest", "trail", "clearing"],
        },
        GardenEstate => {
            code: "garden_estate",
            label: "Walled Estate & Courtyard",
            subtitle: "Gated compound with driveway vehicles, landscaped courtyard, hedges & furnished pavilion",
            aliases: ["estate", "compound", "courtyard", "garden", "yard", "villa"],
        },
        WoodlandCreek => {
            code: "woodland_creek",
            label: "Woodland Creek & Ravine",
            subtitle: "Meandering forest creek with gravel shoals, fallen-log & stone crossings, boulders & dense canopy",
            aliases: ["creek", "stream", "brook", "river", "ravine"],
        },
        RoadsideDitch => {
            code: "roadside_ditch",
            label: "Country Lane & Drainage Ditch",
            subtitle: "Gravel country lane bordered by a sunken drainage ditch, culvert bridge, garden beds & timber cover",
            aliases: ["ditch", "trench", "lane", "country_road", "culvert"],
        },
        DockingBay => {
            code: "docking_bay",
            label: "Industrial Logistics & Loading Bay",
            subtitle: "Warehouse cargo deck with elevated catwalk, shipping office, utility van & hazmat spill",
            aliases: ["cargo", "hangar", "bay", "warehouse", "industrial", "plant"],
        },
        MedLabSector => {
            code: "med_lab",
            label: "Trauma Clinic & Bio-Containment Lab",
            subtitle: "Partitioned triage ward, surgical bay & quarantine containment lab with bio-spill",
            aliases: ["medlab", "medical", "lab", "clinic", "hospital"],
        },
        DerelictCorridor => {
            code: "derelict_corridor",
            label: "Breached Utility Tunnel & Sub-Level",
            subtitle: "Segmented concrete service spine with flanking conduit bays, blast doors & chemical rupture",
            aliases: ["corridor", "derelict", "breach", "tunnel", "bunker"],
        },
        OrbitalAirlock => {
            code: "orbital_airlock",
            label: "Pressurized Transfer Airlock",
            subtitle: "Armored transfer chamber with dual airlock vestibules, security booths & vent hazard",
            aliases: ["airlock", "sector4_airlock", "station"],
        },
    }
}

impl SectorPreset {
    /// Furnished edge-wall interior style (`None` for outdoor/facility presets).
    #[must_use]
    pub const fn interior_style(self) -> Option<InteriorStyle> {
        match self {
            Self::ResidentialApartment => Some(InteriorStyle::Apartment),
            Self::OfficeFloor => Some(InteriorStyle::Office),
            Self::CornerBar => Some(InteriorStyle::Bar),
            _ => None,
        }
    }

    /// Multi-cell outdoor / compound style (`None` for indoor/facility presets).
    #[must_use]
    pub const fn outdoor_style(self) -> Option<OutdoorStyle> {
        match self {
            Self::WoodedPark => Some(OutdoorStyle::WoodedPark),
            Self::GardenEstate => Some(OutdoorStyle::GardenEstate),
            Self::WoodlandCreek => Some(OutdoorStyle::WoodlandCreek),
            Self::RoadsideDitch => Some(OutdoorStyle::RoadsideDitch),
            _ => None,
        }
    }

    /// Whether this preset generates architectural [`MapWallEdge`] boundaries.
    #[must_use]
    pub const fn has_structural_wall_edges(self) -> bool {
        !matches!(self, Self::WoodedPark | Self::WoodlandCreek)
    }

    /// Upgraded facility / clinical / transit / urban sector style (`None` for residential/outdoor presets).
    #[must_use]
    pub const fn facility_style(self) -> Option<FacilitySectorStyle> {
        match self {
            Self::UrbanCrossroads => Some(FacilitySectorStyle::UrbanCrossroads),
            Self::DockingBay => Some(FacilitySectorStyle::DockingBay),
            Self::MedLabSector => Some(FacilitySectorStyle::MedLab),
            Self::DerelictCorridor => Some(FacilitySectorStyle::DerelictCorridor),
            Self::OrbitalAirlock => Some(FacilitySectorStyle::OrbitalAirlock),
            _ => None,
        }
    }
}

/// Complete procedural sector: thin edge walls and `360°` oriented props (zero tile array).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProceduralSector {
    pub wall_edges: Vec<MapWallEdge>,
    pub props: Vec<MapPropPlacement>,
}

/// Generates the complete [`ProceduralSector`] for `preset`, emitting thin edge walls and props.
#[must_use]
pub fn generate_procedural_sector(preset: SectorPreset, width: i32, height: i32, seed: i64) -> ProceduralSector {
    let (w, h) = (width.clamp(8, 48), height.clamp(8, 48));
    if let Some(style) = preset.interior_style() {
        return generate_furnished_interior(style, w, h, seed);
    }
    if let Some(style) = preset.outdoor_style() {
        return generate_outdoor_sector(style, w, h, seed);
    }
    if let Some(style) = preset.facility_style() {
        return generate_facility_sector(style, w, h, seed);
    }
    ProceduralSector::default()
}

/// Complete Line-of-Sight, metric range, and cover evaluation between two continuous points in meters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalLosEvaluation {
    pub attacker_m: (f32, f32),
    pub target_m: (f32, f32),
    /// Euclidean distance in meters.
    pub distance_m: f32,
    pub has_los: bool,
    pub cover_grade: CoverGrade,
    /// Recommended Satya Ranged External Modifier (`+0`, `+3`, `+4`, `+6`, or `+16` when blocked).
    pub recommended_delta_ext: i32,
    /// Sampled sightline segment `(start_m, target_m)` for 2D/3D overlay drawing.
    pub ray_points_m: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Copy, Default)]
struct RayObstruction {
    blocked: bool,
    half_cover: bool,
    concealment: bool,
}

fn eval_continuous_ray(
    scene: &TacticalSceneView<'_>,
    origin_m: (f32, f32),
    target_m: (f32, f32),
) -> RayObstruction {
    let mut obs = RayObstruction::default();
    let dx = target_m.0 - origin_m.0;
    let dy = target_m.1 - origin_m.1;
    let seg_len = (dx * dx + dy * dy).sqrt();
    if seg_len <= 1e-4 {
        return obs;
    }

    for edge in scene.edges {
        if let Some(t) = intersect_segment_edge(origin_m, target_m, edge) {
            let hit_dist = t * seg_len;
            match edge.kind {
                WallEdgeKind::Wall | WallEdgeKind::DoorClosed => {
                    obs.blocked = true;
                    return obs;
                }
                WallEdgeKind::Window => {
                    if hit_dist > 0.85 {
                        obs.half_cover = true;
                    }
                }
                WallEdgeKind::DoorOpen => {}
            }
        }
    }

    for p in scene.props {
        if let Some(hit) = intersect_segment_prop(p, origin_m, target_m) {
            let enter_dist = hit.t_enter * seg_len;
            let braced_skip = hit.obstruction != PropObstruction::FullObstacle
                && enter_dist <= 0.85
                && hit.t_exit < 0.99;
            if braced_skip {
                continue;
            }
            match hit.obstruction {
                PropObstruction::FullObstacle => {
                    obs.blocked = true;
                    return obs;
                }
                PropObstruction::HalfCover => obs.half_cover = true,
                PropObstruction::Concealment => obs.concealment = true,
                PropObstruction::FloorLayer => {}
            }
        }
    }
    obs
}

/// Analytical continuous Line-of-Sight, corner-peek, and cover evaluation between `attacker_m` and `target_m`.
#[must_use]
pub fn evaluate_los_continuous(
    scene: &TacticalSceneView<'_>,
    attacker_m: (f32, f32),
    target_m: (f32, f32),
) -> TacticalLosEvaluation {
    let dx = target_m.0 - attacker_m.0;
    let dy = target_m.1 - attacker_m.1;
    let raw_dist = (dx * dx + dy * dy).sqrt();
    let distance_m = (raw_dist * 100.0).round() / 100.0;

    if raw_dist <= 0.05 {
        return TacticalLosEvaluation {
            attacker_m,
            target_m,
            distance_m: 0.0,
            has_los: true,
            cover_grade: CoverGrade::Clear,
            recommended_delta_ext: 0,
            ray_points_m: Vec::new(),
        };
    }

    let primary_obs = eval_continuous_ray(scene, attacker_m, target_m);
    if !primary_obs.blocked {
        let grade = if primary_obs.half_cover {
            CoverGrade::LowCover
        } else if primary_obs.concealment {
            CoverGrade::Concealment
        } else {
            CoverGrade::Clear
        };
        return TacticalLosEvaluation {
            attacker_m,
            target_m,
            distance_m,
            has_los: true,
            cover_grade: grade,
            recommended_delta_ext: grade.recommended_delta_ext(),
            ray_points_m: vec![attacker_m, target_m],
        };
    }

    // Perpendicular ±0.65m / ±0.85m step-out lean rays (plus fallback orthogonal ±0.65m).
    let ux = dx / raw_dist;
    let uy = dy / raw_dist;
    let leans = [
        (-uy * 0.65, ux * 0.65),
        (uy * 0.65, -ux * 0.65),
        (-uy * 0.85, ux * 0.85),
        (uy * 0.85, -ux * 0.85),
        (-0.65, 0.0),
        (0.65, 0.0),
        (0.0, -0.65),
        (0.0, 0.65),
    ];
    for &(ox, oy) in &leans {
        let peek = (attacker_m.0 + ox, attacker_m.1 + oy);
        if circle_collides_obstacles(scene.width_m, scene.height_m, scene.edges, scene.props, peek.0, peek.1, 0.18) {
            continue;
        }
        let lean_clear = !eval_continuous_ray(scene, attacker_m, peek).blocked;
        if !lean_clear {
            continue;
        }
        if !eval_continuous_ray(scene, peek, target_m).blocked {
            return TacticalLosEvaluation {
                attacker_m,
                target_m,
                distance_m,
                has_los: true,
                cover_grade: CoverGrade::HighCover,
                recommended_delta_ext: CoverGrade::HighCover.recommended_delta_ext(),
                ray_points_m: vec![peek, target_m],
            };
        }
    }

    TacticalLosEvaluation {
        attacker_m,
        target_m,
        distance_m,
        has_los: false,
        cover_grade: CoverGrade::Blocked,
        recommended_delta_ext: CoverGrade::Blocked.recommended_delta_ext(),
        ray_points_m: vec![attacker_m, target_m],
    }
}

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

    let node_pos = |ix: usize, iy: usize| -> (f32, f32) {
        ((ix as f32 + 0.5) * step_m, (iy as f32 + 0.5) * step_m)
    };

    for iy in 0..rows {
        for ix in 0..cols {
            let (px, py) = node_pos(ix, iy);
            let occupied = occupied_tokens_m
                .iter()
                .any(|&(tx, ty)| (px - tx).powi(2) + (py - ty).powi(2) < (TOKEN_RADIUS_M * 1.65).powi(2));
            let blocked_prop = scene.props.iter().any(|p| {
                prop_spec(p.kind).obstruction.blocks_movement() && prop_intersects_circle(p, px, py, 0.22)
            });
            let idx = iy * cols + ix;
            walkable[idx] = !occupied && !blocked_prop && !circle_collides_obstacles(scene.width_m, scene.height_m, scene.edges, &[], px, py, 0.20);
            if walkable[idx] {
                let mut m = 1.0_f32;
                for p in scene.props {
                    let sp = prop_spec(p.kind);
                    if sp.move_cost_mult > m && prop_contains_point(p, px, py, false) {
                        m = sp.move_cost_mult;
                    }
                }
                cost_mult[idx] = m;
            }
        }
    }

    let sx = ((query.start_m.0 / step_m).floor() as isize).clamp(0, cols as isize - 1) as usize;
    let sy = ((query.start_m.1 / step_m).floor() as isize).clamp(0, rows as isize - 1) as usize;
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

    let blocking_edges: Vec<&MapWallEdge> = scene
        .edges
        .iter()
        .filter(|e| WallEdgeKind::blocks_movement(e.kind))
        .collect();

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
            if blocking_edges
                .iter()
                .any(|e| intersect_segment_edge(a, b, e).is_some())
            {
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

/// Builds a complete vector [`GmMapUpdateRequest`] with continuous centimeter token coordinates,
/// thin structural [`MapWallEdge`]s, `360°` oriented props, and `tiles: Vec::new()`.
#[must_use]
pub fn build_procedural_map_request(
    map_id: &str,
    preset: SectorPreset,
    title_override: Option<&str>,
    width: i32,
    height: i32,
    seed: i64,
) -> GmMapUpdateRequest {
    let w = width.clamp(8, 48);
    let h = height.clamp(8, 48);
    let ProceduralSector { wall_edges, props } = generate_procedural_sector(preset, w, h, seed);
    let mid_y = h / 2;
    let title = title_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| preset.full_name())
        .to_string();

    let sara_y = (mid_y - 1).max(1);
    let tokens = vec![
        TokenPlacement {
            entity_id: "tok-sara".to_string(),
            label: "Sara (Valkyrie)".to_string(),
            x: 2,
            y: sara_y,
            visibility: MessageVisibility::Public,
        },
        TokenPlacement {
            entity_id: "tok-marcus".to_string(),
            label: "Marcus (Recon)".to_string(),
            x: 2,
            y: mid_y,
            visibility: MessageVisibility::Public,
        },
        TokenPlacement {
            entity_id: "tok-secops-1".to_string(),
            label: "SecOps Sentry".to_string(),
            x: w - 3,
            y: mid_y,
            visibility: MessageVisibility::Public,
        },
    ];
    let token_visuals = vec![
        TokenVisualSpec {
            entity_id: "tok-sara".to_string(),
            model_preset: "valkyrie_rig".to_string(),
            color_hex: "#38bdf8".to_string(),
            facing_deg: 90,
            elevation_cm: 0,
            stance_pose: "aimed".to_string(),
            portrait_url: None,
            pos_x_cm: Some(250),
            pos_y_cm: Some(sara_y * 100 + 50),
        },
        TokenVisualSpec {
            entity_id: "tok-marcus".to_string(),
            model_preset: "recon_mk4".to_string(),
            color_hex: "#22d3ee".to_string(),
            facing_deg: 90,
            elevation_cm: 0,
            stance_pose: "idle".to_string(),
            portrait_url: None,
            pos_x_cm: Some(250),
            pos_y_cm: Some(mid_y * 100 + 50),
        },
        TokenVisualSpec {
            entity_id: "tok-secops-1".to_string(),
            model_preset: "secops_sentry".to_string(),
            color_hex: "#f87171".to_string(),
            facing_deg: 270,
            elevation_cm: 0,
            stance_pose: "aimed".to_string(),
            portrait_url: None,
            pos_x_cm: Some((w - 3) * 100 + 50),
            pos_y_cm: Some(mid_y * 100 + 50),
        },
    ];

    GmMapUpdateRequest {
        map_id: map_id.to_string(),
        title,
        image_url: String::new(),
        grid_width: w,
        grid_height: h,
        tokens,
        notes: Some(format!(
            "{} · Continuous Metric Arena ({}m x {}m, Seed {})",
            preset.subtitle(),
            w,
            h,
            seed
        )),
        cell_size_cm: 100,
        theme_preset: Some(preset.as_str().to_string()),
        seed: Some(seed),
        tiles: Vec::new(),
        token_visuals,
        wall_edges,
        props,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_dressing::{PropObstruction, make_prop};
    use crate::tactical_map_geom::all_walkable_areas_connected;
    use satya_schema::{EdgeSide, MaterialClass, PropKind};

    #[test]
    fn test_procedural_sector_presets_and_continuous_los_cover() {
        for preset in SectorPreset::ALL {
            let sec = generate_procedural_sector(preset, 20, 14, 42);
            if preset.has_structural_wall_edges() {
                assert!(!sec.wall_edges.is_empty(), "{preset:?} missing wall_edges");
            }
            assert!(!sec.props.is_empty(), "{preset:?} missing props");
            assert!(
                sec.props.iter().any(|p| prop_spec(p.kind).obstruction == PropObstruction::HalfCover),
                "{preset:?} missing half cover prop"
            );
            assert!(
                all_walkable_areas_connected(20, 14, &sec.wall_edges, &sec.props),
                "{preset:?} disconnected walkable area"
            );
            // West and East deployment pads must remain clear of blocking obstacles.
            assert!(!circle_collides_obstacles(20.0, 14.0, &sec.wall_edges, &sec.props, 2.5, 7.5, TOKEN_RADIUS_M));
            assert!(!circle_collides_obstacles(20.0, 14.0, &sec.wall_edges, &sec.props, 17.5, 7.5, TOKEN_RADIUS_M));
        }

        // Clear LoS
        let empty = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &[], props: &[] };
        let clear = evaluate_los_continuous(&empty, (2.5, 4.5), (9.5, 4.5));
        assert!(clear.has_los);
        assert_eq!(clear.cover_grade, CoverGrade::Clear);
        assert_eq!(clear.recommended_delta_ext, 0);
        assert!((clear.distance_m - 7.0).abs() < 0.01);

        // Mid-field JerseyBarrier -> LowCover (+4)
        let barrier = make_prop(PropKind::JerseyBarrier, 6, 4, 90, Some(1), 0);
        let low_scene = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &[], props: &[barrier] };
        let low = evaluate_los_continuous(&low_scene, (2.5, 4.5), (9.5, 4.5));
        assert!(low.has_los && low.cover_grade == CoverGrade::LowCover && low.recommended_delta_ext == 4);

        // Braced right behind barrier -> Clear outgoing shot!
        let braced = evaluate_los_continuous(&low_scene, (5.8, 4.5), (9.5, 4.5));
        assert_eq!(braced.cover_grade, CoverGrade::Clear);

        // Single StructuralPillar with open sides -> corner-peek HighCover (+6)
        let pillar = make_prop(PropKind::StructuralPillar, 6, 4, 0, None, 0);
        let pillar_scene = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &[], props: &[pillar] };
        let corner = evaluate_los_continuous(&pillar_scene, (5.6, 4.5), (9.5, 4.5));
        assert!(corner.has_los && corner.cover_grade == CoverGrade::HighCover && corner.recommended_delta_ext == 6);

        // Continuous brick wall across y=0..8 at x=6 -> completely Blocked
        let wall: Vec<MapWallEdge> = (0..8)
            .map(|y| MapWallEdge {
                x: 6,
                y,
                side: EdgeSide::West,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::Brick,
            })
            .collect();
        let blocked_scene = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &wall, props: &[] };
        let blocked = evaluate_los_continuous(&blocked_scene, (2.5, 4.5), (9.5, 4.5));
        assert!(!blocked.has_los && blocked.cover_grade == CoverGrade::Blocked);
    }

    #[test]
    fn test_metric_gait_reach_scales_continuously() {
        let scene = TacticalSceneView { width_m: 20.0, height_m: 14.0, edges: &[], props: &[] };
        let walk = compute_reachable_continuous(
            &scene,
            &[],
            GaitReachQuery {
                start_m: (5.5, 7.5),
                spd_eff: 10,
                hgt_m: 1.75,
                gait: CombatMovementGait::CombatGlideTacticalWalk,
                tick_budget: 8,
            },
        );
        let sprint = compute_reachable_continuous(
            &scene,
            &[],
            GaitReachQuery {
                start_m: (5.5, 7.5),
                spd_eff: 10,
                hgt_m: 1.75,
                gait: CombatMovementGait::AllOutSprint,
                tick_budget: 8,
            },
        );
        let walk_count = walk.cost_mm.iter().filter(|&&c| c < i32::MAX).count();
        let sprint_count = sprint.cost_mm.iter().filter(|&&c| c < i32::MAX).count();
        assert!(walk_count > 16);
        assert!(sprint_count > walk_count);
        assert!(!walk.merged_row_runs().is_empty());
        assert!(walk.ticks_at_m(6.0, 7.5).is_some());
    }
}
