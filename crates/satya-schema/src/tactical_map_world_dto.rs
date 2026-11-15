//! Macro-world stitched sector grid DTOs and `N/S/E/W` boundary edge contracts.
//!
//! Companion to [`crate::tactical_map_dto`] providing `Eq`-safe integer/enum wire types
//! (`WorldSectorCell`, `SectorBoundaryProfile`, `SectorBoundaryEdge`) persisted on
//! [`crate::protocol_map_ai::GmMapUpdateRequest`] with `#[serde(default)]` backwards compatibility
//! and zero changes to locked `v1` Avro schemas.

use serde::{Deserialize, Serialize};

use crate::tactical_map_dto::MapContinuationDir;
use crate::tactical_map_floors_dto::{EconomicTier, ZoningContext};

/// Edge contract summarizing roads, trails, water channels, and walls along one cardinal border of a sector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SectorBoundaryEdge {
    /// `true` when an `AsphaltRoad` or `GravelRoad` runs parallel within `450 cm` (`4.5 m`) of this border across `>= 35%` of its span.
    #[serde(default)]
    pub road_parallel: bool,
    /// Mean distance in centimeters from this border to the parallel road centerline (`0` when `!road_parallel`).
    #[serde(default)]
    pub road_offset_cm: i32,
    /// Tangential coordinates (`cm` along the edge) where road splines exit across this border.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crossing_roads_cm: Vec<i32>,
    /// Tangential coordinates (`cm` along the edge) where water channels (`CreekChannel` / `DrainageDitch`) exit across this border.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crossing_water_cm: Vec<i32>,
    /// Tangential coordinates (`cm` along the edge) where trails or walkways exit across this border.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crossing_trails_cm: Vec<i32>,
    /// Percentage (`0..=100`) of boundary cell edges occupied by structural walls, windows, or doors.
    #[serde(default)]
    pub walled_pct: u8,
}

impl SectorBoundaryEdge {
    /// Returns `true` if either a parallel border road or at least one perpendicular crossing road touches this edge.
    #[must_use]
    pub fn has_any_road(&self) -> bool {
        self.road_parallel || !self.crossing_roads_cm.is_empty()
    }

    /// Returns `true` if a creek or drainage ditch exits across this boundary edge.
    #[must_use]
    pub fn has_water(&self) -> bool {
        !self.crossing_water_cm.is_empty()
    }
}

/// Complete `N/S/E/W` boundary profile for a generated [`WorldSectorCell`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SectorBoundaryProfile {
    /// North boundary contract (`y == 0` in sector-local coordinates).
    #[serde(default)]
    pub north: SectorBoundaryEdge,
    /// South boundary contract (`y == height` in sector-local coordinates).
    #[serde(default)]
    pub south: SectorBoundaryEdge,
    /// East boundary contract (`x == width` in sector-local coordinates).
    #[serde(default)]
    pub east: SectorBoundaryEdge,
    /// West boundary contract (`x == 0` in sector-local coordinates).
    #[serde(default)]
    pub west: SectorBoundaryEdge,
}

impl SectorBoundaryProfile {
    /// Returns the immutable boundary contract for `dir`.
    #[must_use]
    pub const fn edge(&self, dir: MapContinuationDir) -> &SectorBoundaryEdge {
        match dir {
            MapContinuationDir::North => &self.north,
            MapContinuationDir::South => &self.south,
            MapContinuationDir::East => &self.east,
            MapContinuationDir::West => &self.west,
        }
    }

    /// Returns a mutable reference to the boundary contract for `dir`.
    #[must_use]
    pub const fn edge_mut(&mut self, dir: MapContinuationDir) -> &mut SectorBoundaryEdge {
        match dir {
            MapContinuationDir::North => &mut self.north,
            MapContinuationDir::South => &mut self.south,
            MapContinuationDir::East => &mut self.east,
            MapContinuationDir::West => &mut self.west,
        }
    }
}

/// One sector cell within a multi-sector stitched battlemap arena.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorldSectorCell {
    /// Signed macro-world X coordinate (`0` at initial sector; `East` = `+1`, `West` = `-1`).
    pub sector_x: i32,
    /// Signed macro-world Y coordinate (`0` at initial sector; `South` = `+1`, `North` = `-1`).
    pub sector_y: i32,
    /// Left X origin in arena grid cells (`1 cell = cell_size_cm`).
    pub origin_x: i32,
    /// Top Y origin in arena grid cells (`1 cell = cell_size_cm`).
    pub origin_y: i32,
    /// Sector width in arena grid cells.
    pub width: i32,
    /// Sector height in arena grid cells.
    pub height: i32,
    /// Canonical `SectorPreset` snake-case identifier for this sector.
    pub preset: String,
    /// Optional macro zoning wrapper used when generating this sector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoning: Option<ZoningContext>,
    /// Optional residential economic tier used when generating this sector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub economic_tier: Option<EconomicTier>,
    /// Consecutive steps inside the same macro zone family (`0` at origin or after zone transition).
    #[serde(default)]
    pub zone_depth: u8,
    /// Deterministic procedural seed for this sector cell.
    #[serde(default)]
    pub seed: i64,
    /// Measured `N/S/E/W` boundary profile along this sector's perimeter.
    #[serde(default)]
    pub boundary: SectorBoundaryProfile,
}

impl WorldSectorCell {
    /// Returns `true` if integer arena grid cell `(gx, gy)` lies inside this sector's rectangle.
    #[must_use]
    pub const fn contains_cell(&self, gx: i32, gy: i32) -> bool {
        gx >= self.origin_x
            && gx < self.origin_x + self.width
            && gy >= self.origin_y
            && gy < self.origin_y + self.height
    }

    /// Returns `true` if continuous arena centimeter coordinate `(x_cm, y_cm)` lies inside this sector.
    #[must_use]
    pub fn contains_cm(&self, x_cm: i32, y_cm: i32, cell_size_cm: i32) -> bool {
        let s = cell_size_cm.max(25);
        let x0 = self.origin_x * s;
        let y0 = self.origin_y * s;
        let x1 = (self.origin_x + self.width) * s;
        let y1 = (self.origin_y + self.height) * s;
        x_cm >= x0 && x_cm < x1 && y_cm >= y0 && y_cm < y1
    }

    /// Translates the sector's arena cell origin by `(dx_cells, dy_cells)` (used when prepending `West`/`North`).
    pub fn shift_origin(&mut self, dx_cells: i32, dy_cells: i32) {
        self.origin_x += dx_cells;
        self.origin_y += dy_cells;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::GmMapUpdateRequest;

    #[test]
    fn world_sector_cell_serde_roundtrip_legacy_compat_and_bounds() {
        let mut cell = WorldSectorCell {
            sector_x: 0,
            sector_y: 0,
            origin_x: 0,
            origin_y: 0,
            width: 20,
            height: 14,
            preset: "downtown_commercial".to_string(),
            zoning: Some(ZoningContext::DowntownCommercial),
            economic_tier: Some(EconomicTier::UpperMiddleClass),
            zone_depth: 1,
            seed: 42,
            boundary: SectorBoundaryProfile {
                south: SectorBoundaryEdge {
                    road_parallel: true,
                    road_offset_cm: 145,
                    crossing_roads_cm: vec![],
                    crossing_water_cm: vec![],
                    crossing_trails_cm: vec![850],
                    walled_pct: 12,
                },
                ..Default::default()
            },
        };
        assert!(cell.contains_cell(0, 0));
        assert!(cell.contains_cell(19, 13));
        assert!(!cell.contains_cell(20, 13));
        assert!(cell.contains_cm(1999, 1399, 100));
        assert!(!cell.contains_cm(2000, 1399, 100));
        assert!(cell.boundary.edge(MapContinuationDir::South).has_any_road());
        assert!(!cell.boundary.edge(MapContinuationDir::North).has_water());

        cell.shift_origin(20, 14);
        assert!(cell.contains_cell(20, 14));
        assert!(!cell.contains_cell(19, 14));

        let req = GmMapUpdateRequest {
            world_sectors: vec![cell.clone()],
            ..Default::default()
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: GmMapUpdateRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.world_sectors, vec![cell]);

        // Legacy map payloads omitting world_sectors deserialize cleanly.
        let legacy: GmMapUpdateRequest =
            serde_json::from_str(r#"{"map_id":"m","title":"T","image_url":"","grid_width":20,"grid_height":14}"#).unwrap();
        assert!(legacy.world_sectors.is_empty());
    }
}
