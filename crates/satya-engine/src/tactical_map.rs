//! # Tactical `1.0m` Map Grid, Procedural Sector Generator, Supercover LoS/Cover & Gait Reach
//!
//! Deterministic spatial geometry engine shared across `satya-engine`, `satya-signaling`, and `satya-web`:
//! - **Procedural `1.0m` Sector Templates**: [`SectorPreset`] (`OrbitalAirlock`, `DockingBay`, `MedLabSector`,
//!   `UrbanCrossroads`, `DerelictCorridor`) with seeded cover/hazard variation.
//! - **Supercover Raycast & Corner-Peek Cover**: [`evaluate_tactical_los_and_cover`] computing metric range
//!   (`distance_m`), [`CoverGrade`] (`Clear`, `LowCover`, `HighCover`, `Blocked`), and recommended $\Delta_{\text{ext}}$.
//! - **Metric Gait Movement Envelope**: [`compute_reachable_grid_cells`] coupling [`CombatMovementGait`] and
//!   [`compute_stride_and_sprint_velocity`] directly to `1.0m` 8-directional Dijkstra pathing.
//! - **Edge Walls & Multi-Cell Props**: [`TacticalGridView`] pairs the tile grid with a
//!   [`WallEdgeIndex`] so LoS ([`evaluate_los_on_grid`]) and reach ([`compute_reachable_on_grid`]) respect thin
//!   walls, doors, and windows. Furnished modern-day presets come from [`crate::map_interior_gen`], and prop
//!   catalogs from [`crate::map_dressing`].

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use satya_schema::{
    CoverGrade, GmMapUpdateRequest, MapPropPlacement, MapWallEdge, MessageVisibility,
    TacticalTileKind, TokenPlacement, TokenVisualSpec, WallEdgeKind,
};
use serde::{Deserialize, Serialize};

use crate::define_coded_enum;
use crate::environment_vehicles::{CombatMovementGait, compute_stride_and_sprint_velocity};
use crate::map_interior_gen::{InteriorStyle, generate_furnished_interior};
pub use crate::tactical_map_edges::{EdgeObstruction, WallEdgeIndex};

/// Borrowed view of everything spatial rules need: row-major tiles plus the thin edge-wall index.
#[derive(Debug, Clone, Copy)]
pub struct TacticalGridView<'a> {
    pub tiles: &'a [TacticalTileKind],
    pub width: i32,
    pub height: i32,
    pub edges: &'a WallEdgeIndex,
}

impl TacticalGridView<'_> {
    fn tile(&self, x: i32, y: i32) -> TacticalTileKind {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return TacticalTileKind::FullWall;
        }
        self.tiles
            .get((y * self.width + x) as usize)
            .copied()
            .unwrap_or(TacticalTileKind::Floor)
    }
}

/// Builds the [`WallEdgeIndex`] for a wire map (empty for legacy cell-wall maps).
#[must_use]
pub fn wall_edge_index_for_map(map: &GmMapUpdateRequest) -> WallEdgeIndex {
    WallEdgeIndex::from_edges(map.grid_width, map.grid_height, &map.wall_edges)
}

define_coded_enum! {
    /// Canonical procedural sector layout presets for `1.0m` tactical battlemaps.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum SectorPreset [count = 7] {
        #[default]
        OrbitalAirlock => {
            code: "orbital_airlock",
            label: "Sector 4 Orbital Airlock",
            subtitle: "Pressurized transfer chamber with blast shields & dual airlocks",
            aliases: ["airlock", "sector4_airlock", "station"],
        },
        DockingBay => {
            code: "docking_bay",
            label: "Aethelgard Cargo Docking Bay",
            subtitle: "Wide cargo deck with elevated catwalk, crate clusters & fuel hazard",
            aliases: ["cargo", "hangar", "bay"],
        },
        MedLabSector => {
            code: "med_lab",
            label: "Sector 4 Trauma & Med-Lab",
            subtitle: "Partitioned surgical ward with quarantine doors & bio-spill hazard",
            aliases: ["medlab", "medical", "lab"],
        },
        UrbanCrossroads => {
            code: "urban_crossroads",
            label: "Rain-Slick Urban Crossroads",
            subtitle: "Four-way transit intersection with corner buildings & vehicle cover",
            aliases: ["urban", "street", "city"],
        },
        DerelictCorridor => {
            code: "derelict_corridor",
            label: "Breached Derelict Corridor",
            subtitle: "Segmented bulkhead passage with collapsed conduits & vacuum breach",
            aliases: ["corridor", "derelict", "breach"],
        },
        ResidentialApartment => {
            code: "residential_apartment",
            label: "Residential Apartment",
            subtitle: "Furnished multi-room apartment with drywall partitions, doors & windows",
            aliases: ["apartment", "flat", "house", "residential"],
        },
        CornerBar => {
            code: "corner_bar",
            label: "Corner Bar",
            subtitle: "Neighborhood bar with a long counter, booths, pool table & back rooms",
            aliases: ["bar", "pub", "tavern", "saloon"],
        },
    }
}

impl SectorPreset {
    /// Furnished edge-wall interior style for modern-day presets (`None` for legacy cell-wall sectors).
    #[must_use]
    pub const fn interior_style(self) -> Option<InteriorStyle> {
        match self {
            Self::ResidentialApartment => Some(InteriorStyle::Apartment),
            Self::CornerBar => Some(InteriorStyle::Bar),
            _ => None,
        }
    }
}

/// Complete procedural sector: stamped tiles, thin edge walls, and multi-cell props.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProceduralSector {
    pub tiles: Vec<TacticalTileKind>,
    pub wall_edges: Vec<MapWallEdge>,
    pub props: Vec<MapPropPlacement>,
}

/// Generates the full [`ProceduralSector`] for `preset` (edge walls and props are empty for legacy presets).
#[must_use]
pub fn generate_procedural_sector(preset: SectorPreset, width: i32, height: i32, seed: i64) -> ProceduralSector {
    match preset.interior_style() {
        Some(style) => generate_furnished_interior(style, width.clamp(8, 48), height.clamp(8, 48), seed),
        None => ProceduralSector {
            tiles: generate_procedural_tactical_grid(preset, width, height, seed),
            ..Default::default()
        },
    }
}

/// Deterministic `SplitMix64` helper used for seeded procedural prop/cover scattering.
#[derive(Debug, Clone, Copy)]
struct SplitMix64(u64);

impl SplitMix64 {
    const fn new(seed: i64) -> Self {
        Self((seed as u64).wrapping_add(0x9E37_79B9_7F4A_7C15))
    }

    fn next_u64(&mut self) -> u64 {
        let mut z = self.0;
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range_i32(&mut self, lo_inclusive: i32, hi_exclusive: i32) -> i32 {
        if hi_exclusive <= lo_inclusive {
            return lo_inclusive;
        }
        let span = (hi_exclusive - lo_inclusive) as u64;
        lo_inclusive + (self.next_u64() % span) as i32
    }
}

fn set_tile(tiles: &mut [TacticalTileKind], w: i32, h: i32, x: i32, y: i32, kind: TacticalTileKind) {
    if x >= 0 && y >= 0 && x < w && y < h {
        tiles[(y * w + x) as usize] = kind;
    }
}

/// Generates a deterministic `width * height` row-major array of [`TacticalTileKind`] (`1 cell = 1.0m`)
/// for the requested [`SectorPreset`] and `seed`.
#[must_use]
pub fn generate_procedural_tactical_grid(
    preset: SectorPreset,
    width: i32,
    height: i32,
    seed: i64,
) -> Vec<TacticalTileKind> {
    if let Some(style) = preset.interior_style() {
        return generate_furnished_interior(style, width.clamp(8, 48), height.clamp(8, 48), seed).tiles;
    }
    let w = width.clamp(8, 48);
    let h = height.clamp(8, 48);
    let mut tiles = vec![TacticalTileKind::Floor; (w * h) as usize];
    let mut rng = SplitMix64::new(seed);

    // Outer perimeter bulkheads for indoor sectors, with West/East airlock thresholds
    if preset != SectorPreset::UrbanCrossroads {
        for x in 0..w {
            set_tile(&mut tiles, w, h, x, 0, TacticalTileKind::FullWall);
            set_tile(&mut tiles, w, h, x, h - 1, TacticalTileKind::FullWall);
        }
        for y in 0..h {
            set_tile(&mut tiles, w, h, 0, y, TacticalTileKind::FullWall);
            set_tile(&mut tiles, w, h, w - 1, y, TacticalTileKind::FullWall);
        }
        let mid_y = h / 2;
        set_tile(&mut tiles, w, h, 0, mid_y, TacticalTileKind::DoorClosed);
        set_tile(&mut tiles, w, h, 0, (mid_y - 1).max(1), TacticalTileKind::DoorOpen);
        set_tile(&mut tiles, w, h, w - 1, mid_y, TacticalTileKind::DoorClosed);
    }

    let mid_x = w / 2;
    let mid_y = h / 2;

    match preset {
        SectorPreset::OrbitalAirlock => {
            // Structural Mag-Rail pillars + central blast-shield half-cover + plasma vent hazard
            for &(px, py) in &[
                (mid_x - 4, 3),
                (mid_x + 3, 3),
                (mid_x - 4, h - 4),
                (mid_x + 3, h - 4),
            ] {
                set_tile(&mut tiles, w, h, px, py, TacticalTileKind::FullWall);
            }
            for dy in -1..=1 {
                set_tile(&mut tiles, w, h, mid_x - 1, mid_y + dy, TacticalTileKind::HalfCover);
            }
            set_tile(&mut tiles, w, h, 4, 3, TacticalTileKind::HalfCover);
            set_tile(&mut tiles, w, h, 4, h - 4, TacticalTileKind::HalfCover);
            set_tile(&mut tiles, w, h, w - 5, 3, TacticalTileKind::HalfCover);
            set_tile(&mut tiles, w, h, w - 5, h - 4, TacticalTileKind::HalfCover);
            set_tile(&mut tiles, w, h, mid_x + 1, mid_y - 2, TacticalTileKind::HazardZone);
            set_tile(&mut tiles, w, h, mid_x + 2, mid_y - 2, TacticalTileKind::HazardZone);
        }
        SectorPreset::DockingBay => {
            // North elevated catwalk + asymmetric cargo crate clusters + fuel spill
            for x in 3..(w - 3) {
                set_tile(&mut tiles, w, h, x, 2, TacticalTileKind::ElevatedPlatform);
            }
            for &(cx, cy) in &[
                (5, mid_y - 2),
                (5, mid_y - 1),
                (mid_x - 2, mid_y + 1),
                (mid_x - 1, mid_y + 1),
                (mid_x + 2, mid_y - 1),
                (mid_x + 2, mid_y),
                (w - 6, mid_y + 2),
            ] {
                set_tile(&mut tiles, w, h, cx, cy, TacticalTileKind::HalfCover);
            }
            set_tile(&mut tiles, w, h, mid_x, 4, TacticalTileKind::FullWall);
            set_tile(&mut tiles, w, h, mid_x, h - 4, TacticalTileKind::FullWall);
            set_tile(&mut tiles, w, h, mid_x - 1, h - 3, TacticalTileKind::HazardZone);
            set_tile(&mut tiles, w, h, mid_x, h - 3, TacticalTileKind::HazardZone);
        }
        SectorPreset::MedLabSector => {
            // Central quarantine bulkhead with doors + auto-doc beds + bio-hazard spill
            for y in 1..(h - 1) {
                let kind = if y == mid_y || y == 2 {
                    TacticalTileKind::DoorOpen
                } else if y == h - 3 {
                    TacticalTileKind::DoorClosed
                } else {
                    TacticalTileKind::FullWall
                };
                set_tile(&mut tiles, w, h, mid_x, y, kind);
            }
            for &(bx, by) in &[(4, 3), (6, 3), (4, h - 4), (mid_x + 3, 4), (mid_x + 3, h - 5)] {
                set_tile(&mut tiles, w, h, bx, by, TacticalTileKind::HalfCover);
            }
            set_tile(&mut tiles, w, h, mid_x + 2, mid_y + 1, TacticalTileKind::HazardZone);
            set_tile(&mut tiles, w, h, mid_x + 2, mid_y + 2, TacticalTileKind::HazardZone);
        }
        SectorPreset::UrbanCrossroads => {
            // 4 corner building blocks + street jersey barriers / vehicles
            let bw = (w / 4).max(2);
            let bh = (h / 4).max(2);
            for y in 0..bh {
                for x in 0..bw {
                    set_tile(&mut tiles, w, h, x, y, TacticalTileKind::FullWall);
                    set_tile(&mut tiles, w, h, w - 1 - x, y, TacticalTileKind::FullWall);
                    set_tile(&mut tiles, w, h, x, h - 1 - y, TacticalTileKind::FullWall);
                    set_tile(&mut tiles, w, h, w - 1 - x, h - 1 - y, TacticalTileKind::FullWall);
                }
            }
            set_tile(&mut tiles, w, h, bw - 1, bh / 2, TacticalTileKind::DoorClosed);
            set_tile(&mut tiles, w, h, w - bw, h - 1 - bh / 2, TacticalTileKind::DoorOpen);
            for &(cx, cy) in &[
                (mid_x - 3, mid_y - 1),
                (mid_x - 2, mid_y - 1),
                (mid_x + 2, mid_y + 1),
                (mid_x + 3, mid_y + 1),
                (mid_x, bh + 1),
                (mid_x - 1, h - bh - 2),
            ] {
                set_tile(&mut tiles, w, h, cx, cy, TacticalTileKind::HalfCover);
            }
        }
        // Furnished edge-wall interiors returned early above
        SectorPreset::ResidentialApartment | SectorPreset::CornerBar => {}
        SectorPreset::DerelictCorridor => {
            // Narrowed corridor walls top/bottom + segmented airlocks + vacuum breach
            for x in 1..(w - 1) {
                if x % 5 != 0 {
                    set_tile(&mut tiles, w, h, x, 1, TacticalTileKind::FullWall);
                    set_tile(&mut tiles, w, h, x, h - 2, TacticalTileKind::FullWall);
                }
            }
            for &door_x in &[w / 3, (2 * w) / 3] {
                for y in 2..(h - 2) {
                    let kind = if y == mid_y {
                        TacticalTileKind::DoorOpen
                    } else if y == mid_y - 1 {
                        TacticalTileKind::DoorClosed
                    } else {
                        TacticalTileKind::FullWall
                    };
                    set_tile(&mut tiles, w, h, door_x, y, kind);
                }
            }
            set_tile(&mut tiles, w, h, mid_x - 1, mid_y + 2, TacticalTileKind::HazardZone);
            set_tile(&mut tiles, w, h, mid_x, mid_y + 2, TacticalTileKind::HazardZone);
            set_tile(&mut tiles, w, h, mid_x + 1, mid_y - 2, TacticalTileKind::HalfCover);
        }
    }

    // Deterministic seed-driven extra tactical cover crates (avoiding West/East spawn zones)
    let extra_crates = 2 + (rng.next_u64() % 3) as usize;
    for _ in 0..extra_crates {
        let rx = rng.range_i32(4, (w - 4).max(5));
        let ry = rng.range_i32(2, (h - 2).max(3));
        if rx != mid_x && tiles[(ry * w + rx) as usize] == TacticalTileKind::Floor {
            set_tile(&mut tiles, w, h, rx, ry, TacticalTileKind::HalfCover);
        }
    }

    // Guarantee clear West `(2, mid_y)` and East `(w - 3, mid_y)` deployment pads
    for &(sx, sy) in &[(2, mid_y), (2, (mid_y - 1).max(1)), (w - 3, mid_y), (w - 3, (mid_y + 1).min(h - 2))] {
        set_tile(&mut tiles, w, h, sx, sy, TacticalTileKind::Floor);
    }

    tiles
}

/// Complete Line-of-Sight, metric range, and cover evaluation between two tactical grid cells.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalLosEvaluation {
    pub attacker_xy: (i32, i32),
    pub target_xy: (i32, i32),
    /// Center-to-center Euclidean distance in meters (`1 cell = 1.0m` default).
    pub distance_m: f32,
    pub has_los: bool,
    pub cover_grade: CoverGrade,
    /// Recommended Satya Ranged External Modifier (`+0`, `+4`, `+6`, or `+16` when blocked).
    pub recommended_delta_ext: i32,
    /// Intervening grid cells traversed along the primary supercover ray.
    pub ray_cells: Vec<(i32, i32)>,
}

/// Returns all interior cells intersected by a segment `(x0, y0) -> (x1, y1)` in continuous cell-space
/// using Amanatides-Woo / supercover grid traversal (excluding the start and end cells themselves).
#[must_use]
pub fn trace_supercover_segment_cells(
    start_pos: (f32, f32),
    end_pos: (f32, f32),
    start_cell: (i32, i32),
    end_cell: (i32, i32),
) -> Vec<(i32, i32)> {
    let mut visited = Vec::new();
    let dx = end_pos.0 - start_pos.0;
    let dy = end_pos.1 - start_pos.1;
    let steps = (dx.abs().max(dy.abs()) * 6.0).ceil() as i32;
    if steps <= 0 {
        return visited;
    }
    for step in 1..steps {
        let t = (step as f32) / (steps as f32);
        let px = (start_pos.0 + dx * t).floor() as i32;
        let py = (start_pos.1 + dy * t).floor() as i32;
        let cell = (px, py);
        if cell != start_cell && cell != end_cell && visited.last().copied() != Some(cell) {
            visited.push(cell);
        }
    }
    visited
}

fn eval_single_ray(
    grid: &TacticalGridView<'_>,
    start_pos: (f32, f32),
    end_pos: (f32, f32),
    attacker_xy: (i32, i32),
) -> (bool, bool, Vec<(i32, i32)>) {
    let (w, h) = (grid.width, grid.height);
    let target_xy = (end_pos.0.floor() as i32, end_pos.1.floor() as i32);
    let cells = trace_supercover_segment_cells(start_pos, end_pos, attacker_xy, target_xy);
    let mut blocked = false;
    let mut half_cover = false;
    for &(cx, cy) in &cells {
        if cx < 0 || cy < 0 || cx >= w || cy >= h {
            blocked = true;
            break;
        }
        let tile = grid.tile(cx, cy);
        if tile.blocks_full_los() {
            blocked = true;
            break;
        }
        if tile.provides_half_cover() {
            // An attacker braced directly against an adjacent (`<= 1` Chebyshev step) half-cover crate
            // shoots over it without penalizing their own outgoing shot.
            let adj_to_attacker =
                (cx - attacker_xy.0).abs() <= 1 && (cy - attacker_xy.1).abs() <= 1;
            if !adj_to_attacker {
                half_cover = true;
            }
        }
    }
    if !blocked {
        let edge = grid.edges.trace_obstruction(start_pos, end_pos, attacker_xy);
        blocked = edge.blocked;
        half_cover |= edge.half_cover;
    }
    (blocked, half_cover, cells)
}

/// Evaluates metric distance, supercover Line of Sight, corner-peeking, and [`CoverGrade`]
/// between `attacker_xy` and `target_xy` on a legacy cell-wall grid (no edge walls).
#[must_use]
pub fn evaluate_tactical_los_and_cover(
    tiles: &[TacticalTileKind],
    width: i32,
    height: i32,
    cell_size_m: f32,
    attacker_xy: (i32, i32),
    target_xy: (i32, i32),
) -> TacticalLosEvaluation {
    let edges = WallEdgeIndex::default();
    let grid = TacticalGridView { tiles, width, height, edges: &edges };
    evaluate_los_on_grid(&grid, cell_size_m, attacker_xy, target_xy)
}

/// Edge-aware LoS / cover evaluation: tiles **and** thin walls, closed doors, and windows on `grid`.
#[must_use]
pub fn evaluate_los_on_grid(
    grid: &TacticalGridView<'_>,
    cell_size_m: f32,
    attacker_xy: (i32, i32),
    target_xy: (i32, i32),
) -> TacticalLosEvaluation {
    let (width, height) = (grid.width, grid.height);
    let scale_m = cell_size_m.max(0.25);
    let dx = (target_xy.0 - attacker_xy.0) as f32;
    let dy = (target_xy.1 - attacker_xy.1) as f32;
    let distance_m = ((dx * dx + dy * dy).sqrt() * scale_m * 100.0).round() / 100.0;

    if attacker_xy == target_xy {
        return TacticalLosEvaluation {
            attacker_xy,
            target_xy,
            distance_m: 0.0,
            has_los: true,
            cover_grade: CoverGrade::Clear,
            recommended_delta_ext: 0,
            ray_cells: Vec::new(),
        };
    }

    let a_center = (attacker_xy.0 as f32 + 0.5, attacker_xy.1 as f32 + 0.5);
    let t_center = (target_xy.0 as f32 + 0.5, target_xy.1 as f32 + 0.5);
    let (center_blocked, center_half_cover, primary_cells) =
        eval_single_ray(grid, a_center, t_center, attacker_xy);

    if !center_blocked {
        let grade = if center_half_cover {
            CoverGrade::LowCover
        } else {
            CoverGrade::Clear
        };
        return TacticalLosEvaluation {
            attacker_xy,
            target_xy,
            distance_m,
            has_los: true,
            cover_grade: grade,
            recommended_delta_ext: grade.recommended_delta_ext(),
            ray_cells: primary_cells,
        };
    }

    // Check orthogonal tactical lean / step-out rays (±0.80m into an adjacent unblocked cell):
    // if leaning out from cover clears full bulkheads, the shot has High/Corner Cover (+6 Δ_ext).
    let offsets: [(f32, f32); 4] = [(-0.80, 0.0), (0.80, 0.0), (0.0, -0.80), (0.0, 0.80)];
    for &(ox, oy) in &offsets {
        let a_peek = (a_center.0 + ox, a_center.1 + oy);
        let ax_cell = (a_peek.0.floor() as i32, a_peek.1.floor() as i32);
        // Leaning must not pass through a thin wall or closed door bounding the attacker's own cell.
        let lean_through_wall = grid.edges.trace_obstruction(a_center, a_peek, attacker_xy).blocked;
        let peek_origin_clear = !lean_through_wall
            && (ax_cell == attacker_xy
                || (ax_cell.0 >= 0
                    && ax_cell.1 >= 0
                    && ax_cell.0 < width
                    && ax_cell.1 < height
                    && !grid.tile(ax_cell.0, ax_cell.1).blocks_full_los()));
        if !peek_origin_clear {
            continue;
        }
        let (peek_blocked, _, peek_cells) = eval_single_ray(grid, a_peek, t_center, attacker_xy);
        if !peek_blocked {
            return TacticalLosEvaluation {
                attacker_xy,
                target_xy,
                distance_m,
                has_los: true,
                cover_grade: CoverGrade::HighCover,
                recommended_delta_ext: CoverGrade::HighCover.recommended_delta_ext(),
                ray_cells: peek_cells,
            };
        }
    }

    TacticalLosEvaluation {
        attacker_xy,
        target_xy,
        distance_m,
        has_los: false,
        cover_grade: CoverGrade::Blocked,
        recommended_delta_ext: CoverGrade::Blocked.recommended_delta_ext(),
        ray_cells: primary_cells,
    }
}

/// Single reachable destination cell and its minimum movement cost in meters and ticks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReachableCell {
    pub x: i32,
    pub y: i32,
    pub distance_cost_m: f32,
    pub ticks_required: u32,
}

#[derive(Debug, Clone, Copy)]
struct DijkstraNode {
    cost_mm: i32,
    x: i32,
    y: i32,
}

impl PartialEq for DijkstraNode {
    fn eq(&self, other: &Self) -> bool {
        self.cost_mm == other.cost_mm && self.x == other.x && self.y == other.y
    }
}

impl Eq for DijkstraNode {}

impl Ord for DijkstraNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost_mm
            .cmp(&self.cost_mm)
            .then_with(|| self.y.cmp(&other.y))
            .then_with(|| self.x.cmp(&other.x))
    }
}

impl PartialOrd for DijkstraNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Context parameters for computing a character's reachable movement envelope on the tactical grid.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GaitReachQuery {
    pub start_xy: (i32, i32),
    pub cell_size_m: f32,
    pub spd_eff: i32,
    pub hgt_m: f64,
    pub gait: CombatMovementGait,
    pub tick_budget: u32,
}

/// Computes all unblocked `(x, y)` cells reachable within `query.tick_budget` using 8-directional
/// metric Dijkstra pathing (`1.0m` orthogonal, `1.414m` diagonal, `2.0x` hazard/vaulting) on a legacy
/// cell-wall grid (no edge walls).
#[must_use]
pub fn compute_reachable_grid_cells(
    tiles: &[TacticalTileKind],
    width: i32,
    height: i32,
    occupied_cells: &[(i32, i32)],
    query: GaitReachQuery,
) -> Vec<ReachableCell> {
    let edges = WallEdgeIndex::default();
    let grid = TacticalGridView { tiles, width, height, edges: &edges };
    compute_reachable_on_grid(&grid, occupied_cells, query)
}

/// Edge-aware gait reach: thin walls, closed doors, and windows block steps; diagonal steps through
/// wall junctions are rejected via [`WallEdgeIndex::step_blocked`].
#[must_use]
pub fn compute_reachable_on_grid(
    grid: &TacticalGridView<'_>,
    occupied_cells: &[(i32, i32)],
    query: GaitReachQuery,
) -> Vec<ReachableCell> {
    let (width, height) = (grid.width, grid.height);
    if width <= 0 || height <= 0 {
        return Vec::new();
    }
    let sprint = compute_stride_and_sprint_velocity(query.spd_eff, query.hgt_m);
    let m_per_tick = (sprint.v_sprint_m_per_tick * query.gait.sprint_fraction()).max(0.05);
    let max_budget_m = m_per_tick * f64::from(query.tick_budget);
    let max_budget_mm = (max_budget_m * 1000.0).round() as i32;
    let cell_mm = (f64::from(query.cell_size_m.max(0.25)) * 1000.0).round() as i32;
    let diag_mm = (f64::from(query.cell_size_m.max(0.25)) * std::f64::consts::SQRT_2 * 1000.0)
        .round() as i32;

    let total = (width * height) as usize;
    let mut dist_mm = vec![i32::MAX; total];
    let mut heap = BinaryHeap::new();

    let (sx, sy) = query.start_xy;
    if sx < 0 || sy < 0 || sx >= width || sy >= height {
        return Vec::new();
    }
    dist_mm[(sy * width + sx) as usize] = 0;
    heap.push(DijkstraNode {
        cost_mm: 0,
        x: sx,
        y: sy,
    });

    let dirs: [(i32, i32, bool); 8] = [
        (1, 0, false),
        (-1, 0, false),
        (0, 1, false),
        (0, -1, false),
        (1, 1, true),
        (1, -1, true),
        (-1, 1, true),
        (-1, -1, true),
    ];

    while let Some(node) = heap.pop() {
        let cur_idx = (node.y * width + node.x) as usize;
        if node.cost_mm > dist_mm[cur_idx] {
            continue;
        }
        for &(dx, dy, is_diag) in &dirs {
            let nx = node.x + dx;
            let ny = node.y + dy;
            if nx < 0 || ny < 0 || nx >= width || ny >= height {
                continue;
            }
            let tile = grid.tile(nx, ny);
            if matches!(tile, TacticalTileKind::FullWall | TacticalTileKind::DoorClosed) {
                continue;
            }
            // Disallow squeezing diagonally between two solid wall/closed-door corners
            if is_diag {
                let adj1 = grid.tile(nx, node.y);
                let adj2 = grid.tile(node.x, ny);
                if adj1.blocks_full_los() && adj2.blocks_full_los() {
                    continue;
                }
            }
            if grid.edges.step_blocked((node.x, node.y), dx, dy, WallEdgeKind::blocks_movement) {
                continue;
            }

            let step_base_mm = if is_diag { diag_mm } else { cell_mm };
            let mult = if tile == TacticalTileKind::HalfCover {
                // Vaulting across a 0.9m crate costs 2x movement
                2.0
            } else {
                tile.move_cost_multiplier()
            };
            let next_cost_mm = node.cost_mm + ((f64::from(step_base_mm) * mult).round() as i32);
            if next_cost_mm > max_budget_mm {
                continue;
            }
            let n_idx = (ny * width + nx) as usize;
            if next_cost_mm < dist_mm[n_idx] {
                dist_mm[n_idx] = next_cost_mm;
                heap.push(DijkstraNode {
                    cost_mm: next_cost_mm,
                    x: nx,
                    y: ny,
                });
            }
        }
    }

    let mut out = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let c_mm = dist_mm[(y * width + x) as usize];
            if c_mm == i32::MAX || (x == sx && y == sy) {
                continue;
            }
            let tile = grid.tile(x, y);
            if tile.blocks_occupancy() {
                continue;
            }
            if occupied_cells.contains(&(x, y)) {
                continue;
            }
            let dist_m = (c_mm as f32) / 1000.0;
            let ticks = ((f64::from(c_mm) / 1000.0) / m_per_tick).ceil().max(1.0) as u32;
            out.push(ReachableCell {
                x,
                y,
                distance_cost_m: (dist_m * 100.0).round() / 100.0,
                ticks_required: ticks,
            });
        }
    }
    out
}

/// Builds a complete [`GmMapUpdateRequest`] with procedural `1.0m` tiles, default starter miniatures
/// when `starter_tokens` is empty, and an inline SVG data-URI fallback.
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
    let ProceduralSector { tiles, wall_edges, props } = generate_procedural_sector(preset, w, h, seed);
    let mid_y = h / 2;
    let title = title_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| preset.full_name())
        .to_string();

    let tokens = vec![
        TokenPlacement {
            entity_id: "tok-sara".to_string(),
            label: "Sara (Valkyrie)".to_string(),
            x: 2,
            y: (mid_y - 1).max(1),
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
        },
        TokenVisualSpec {
            entity_id: "tok-marcus".to_string(),
            model_preset: "recon_mk4".to_string(),
            color_hex: "#22d3ee".to_string(),
            facing_deg: 90,
            elevation_cm: 0,
            stance_pose: "idle".to_string(),
        },
        TokenVisualSpec {
            entity_id: "tok-secops-1".to_string(),
            model_preset: "secops_sentry".to_string(),
            color_hex: "#f87171".to_string(),
            facing_deg: 270,
            elevation_cm: 0,
            stance_pose: "aimed".to_string(),
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
            "{} · 1.0m Tactical Grid ({}m x {}m, Seed {})",
            preset.subtitle(),
            w,
            h,
            seed
        )),
        cell_size_cm: 100,
        theme_preset: Some(preset.as_str().to_string()),
        seed: Some(seed),
        tiles,
        token_visuals,
        wall_edges,
        props,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedural_sector_presets_and_los_cover_math() {
        for preset in SectorPreset::ALL {
            let tiles = generate_procedural_tactical_grid(preset, 20, 14, 42);
            assert_eq!(tiles.len(), 20 * 14);
            assert!(tiles.contains(&TacticalTileKind::HalfCover));
            assert!(tiles.contains(&TacticalTileKind::FullWall));
            // West and East deployment pads must remain clear floor
            assert_eq!(tiles[(7 * 20 + 2) as usize], TacticalTileKind::Floor);
            assert_eq!(tiles[(7 * 20 + 17) as usize], TacticalTileKind::Floor);
        }

        // Verify Clear LoS vs. LowCover (+4) vs. adjacent attacker brace vs. HighCover corner peek vs. Blocked
        let w = 12;
        let h = 8;
        let mut tiles = vec![TacticalTileKind::Floor; (w * h) as usize];
        // Clear shot
        let clear = evaluate_tactical_los_and_cover(&tiles, w, h, 1.0, (2, 4), (9, 4));
        assert!(clear.has_los);
        assert_eq!(clear.cover_grade, CoverGrade::Clear);
        assert_eq!(clear.recommended_delta_ext, 0);
        assert!((clear.distance_m - 7.0).abs() < 0.01);

        // Place HalfCover at (6, 4) mid-field -> imposes LowCover (+4)
        tiles[(4 * w + 6) as usize] = TacticalTileKind::HalfCover;
        let low = evaluate_tactical_los_and_cover(&tiles, w, h, 1.0, (2, 4), (9, 4));
        assert!(low.has_los);
        assert_eq!(low.cover_grade, CoverGrade::LowCover);
        assert_eq!(low.recommended_delta_ext, 4);

        // Attacker braced directly behind (6, 4) at (5, 4) shooting East -> Clear outgoing shot!
        let braced = evaluate_tactical_los_and_cover(&tiles, w, h, 1.0, (5, 4), (9, 4));
        assert_eq!(braced.cover_grade, CoverGrade::Clear);

        // Single FullWall pillar at (6, 4) with open cells at (6, 3) and (6, 5):
        // Attacker at (5, 4) can corner-peek around the 1m pillar -> HighCover (+6)
        tiles[(4 * w + 6) as usize] = TacticalTileKind::FullWall;
        let corner = evaluate_tactical_los_and_cover(&tiles, w, h, 1.0, (5, 4), (9, 4));
        assert!(corner.has_los);
        assert_eq!(corner.cover_grade, CoverGrade::HighCover);
        assert_eq!(corner.recommended_delta_ext, 6);

        // Solid bulkhead wall across y=0..8 at x=6 -> completely Blocked
        for y in 0..h {
            tiles[(y * w + 6) as usize] = TacticalTileKind::FullWall;
        }
        let blocked = evaluate_tactical_los_and_cover(&tiles, w, h, 1.0, (2, 4), (9, 4));
        assert!(!blocked.has_los);
        assert_eq!(blocked.cover_grade, CoverGrade::Blocked);
    }

    #[test]
    fn test_metric_gait_reach_scales_with_combat_movement_gait() {
        let tiles = vec![TacticalTileKind::Floor; 20 * 14];
        let walk = compute_reachable_grid_cells(
            &tiles,
            20,
            14,
            &[],
            GaitReachQuery {
                start_xy: (5, 7),
                cell_size_m: 1.0,
                spd_eff: 10,
                hgt_m: 1.75,
                gait: CombatMovementGait::CombatGlideTacticalWalk,
                tick_budget: 8,
            },
        );
        let sprint = compute_reachable_grid_cells(
            &tiles,
            20,
            14,
            &[],
            GaitReachQuery {
                start_xy: (5, 7),
                cell_size_m: 1.0,
                spd_eff: 10,
                hgt_m: 1.75,
                gait: CombatMovementGait::AllOutSprint,
                tick_budget: 8,
            },
        );
        assert!(!walk.is_empty());
        assert!(sprint.len() > walk.len());
    }
}
