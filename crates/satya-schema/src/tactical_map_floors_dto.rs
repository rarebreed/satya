//! Multi-level floor DTOs, residential economic tiers, zoning contexts, and store archetypes.
//!
//! Companion to [`crate::tactical_map_dto`] providing additive `#[serde(default)]` wire types
//! for vertical floor switching (`B1`/`1F`/`2F`/`Roof`) and composable building generators without
//! altering any locked `v1` Apache Avro schema.

use serde::{Deserialize, Serialize};

use crate::define_wire_enum;
use crate::protocol::GmMapUpdateRequest;
use crate::tactical_map_dressing_dto::{MapPropPlacement, MapWallEdge};
use crate::tactical_map_dto::TokenVisualSpec;
use crate::tactical_map_surfaces_dto::MapSurfaceFeature;

define_wire_enum! {
    /// Residential architectural scale and furnishing affluence tier for procedural homes.
    #[derive(Default)]
    pub enum EconomicTier [count = 4] {
        LowIncome => (
            "LOW_INCOME",
            "Low-Income Compact",
            aliases: ["LOW", "BUDGET", "COMPACT", "PROJECTS"]
        ),
        #[default]
        MiddleClass => (
            "MIDDLE_CLASS",
            "Middle-Class Standard",
            aliases: ["MIDDLE", "STANDARD", "SUBURBAN"]
        ),
        UpperMiddleClass => (
            "UPPER_MIDDLE_CLASS",
            "Upper-Middle Multi-Wing",
            aliases: ["UPPER_MIDDLE", "AFFLUENT", "EXECUTIVE"]
        ),
        LuxuryMansion => (
            "LUXURY_MANSION",
            "Luxury Mansion Estate",
            aliases: ["LUXURY", "MANSION", "MANOR", "ESTATE"]
        ),
    }
}

define_wire_enum! {
    /// Macro land-use placement context wrapping procedural building and street generators.
    #[derive(Default)]
    pub enum ZoningContext [count = 5] {
        UrbanResidential => (
            "URBAN_RESIDENTIAL",
            "Urban Residential (Condo / Rowhouse)",
            aliases: ["URBAN", "CONDO", "PENTHOUSE", "ROWHOUSE"]
        ),
        ProjectsHighDensity => (
            "PROJECTS_HIGH_DENSITY",
            "High-Density Complex (Projects)",
            aliases: ["PROJECTS", "HIGH_DENSITY", "TENEMENT", "WALKUP"]
        ),
        #[default]
        SuburbanNeighborhood => (
            "SUBURBAN_NEIGHBORHOOD",
            "Suburban Neighborhood",
            aliases: ["SUBURBAN", "SUBURB", "NEIGHBORHOOD"]
        ),
        RuralCountryside => (
            "RURAL_COUNTRYSIDE",
            "Rural Homestead & Acreage",
            aliases: ["RURAL", "COUNTRYSIDE", "HOMESTEAD"]
        ),
        DowntownCommercial => (
            "DOWNTOWN_COMMERCIAL",
            "Downtown Commercial Strip",
            aliases: ["COMMERCIAL", "DOWNTOWN", "MAIN_STREET", "RETAIL"]
        ),
    }
}

define_wire_enum! {
    /// Procedural commercial interior layout archetype for streetfront, plaza, and mall retail bays.
    #[derive(Default)]
    pub enum StoreArchetype [count = 7] {
        Grocery => (
            "GROCERY",
            "Neighborhood Grocery",
            aliases: ["SUPERMARKET", "MARKET"]
        ),
        ApparelBoutique => (
            "APPAREL_BOUTIQUE",
            "Apparel Boutique",
            aliases: ["CLOTHING", "BOUTIQUE", "FASHION"]
        ),
        CafeRestaurant => (
            "CAFE_RESTAURANT",
            "Corner Cafe & Bistro",
            aliases: ["CAFE", "BISTRO", "RESTAURANT", "COFFEE"]
        ),
        Pharmacy => (
            "PHARMACY",
            "Pharmacy & Clinic",
            aliases: ["DRUGSTORE", "APOTHECARY"]
        ),
        Electronics => (
            "ELECTRONICS",
            "Electronics Showroom",
            aliases: ["TECH_STORE", "CYBERNETICS"]
        ),
        #[default]
        ConvenienceBodega => (
            "CONVENIENCE_BODEGA",
            "24h Bodega & Convenience",
            aliases: ["BODEGA", "CONVENIENCE", "CORNER_STORE"]
        ),
        DepartmentAnchor => (
            "DEPARTMENT_ANCHOR",
            "Department Anchor Store",
            aliases: ["ANCHOR_STORE", "DEPARTMENT", "SUPERSTORE"]
        ),
    }
}

/// One stacked architectural floor level in a multi-level [`GmMapUpdateRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MapFloorLevel {
    /// Signed floor index (`-1` = Basement `B1`, `0` = Ground `1F`, `1` = `2F`, `2` = `3F` / `Roof`).
    pub floor_index: i32,
    /// Display label rendered in floor switcher tooltips and headers (e.g., `"1F · Ground"`).
    pub label: String,
    /// Relative vertical floor elevation in centimeters (`-320` for `B1`, `0` for `1F`, `340` for `2F`).
    #[serde(default)]
    pub elevation_cm: i32,
    /// Structural wall, door, and window edges belonging to this floor level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wall_edges: Vec<MapWallEdge>,
    /// Continuous `360°` oriented props and stairwells belonging to this floor level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<MapPropPlacement>,
    /// Vector ground/floor surfaces belonging to this floor level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surfaces: Vec<MapSurfaceFeature>,
}

/// Canonical short floor badge (`"B1"`, `"1F"`, `"2F"`, ...) for a signed floor index.
#[must_use]
pub fn default_floor_badge(floor_index: i32) -> String {
    if floor_index < 0 {
        format!("B{}", -floor_index)
    } else {
        format!("{}F", floor_index + 1)
    }
}

impl MapFloorLevel {
    /// Compact button badge derived from `label` prefix before `'·'` or [`default_floor_badge`].
    #[must_use]
    pub fn short_badge(&self) -> String {
        let prefix = self.label.split('·').next().map(str::trim).unwrap_or("");
        if !prefix.is_empty() && prefix.len() <= 6 {
            prefix.to_string()
        } else {
            default_floor_badge(self.floor_index)
        }
    }
}

impl GmMapUpdateRequest {
    /// Whether this battlemap defines two or more vertical floor levels.
    #[must_use]
    pub fn has_multiple_floors(&self) -> bool {
        self.floors.len() > 1
    }

    /// Whether `floor_index` exists in this map (`active_floor` alone when `floors` is empty).
    #[must_use]
    pub fn has_floor(&self, floor_index: i32) -> bool {
        if self.floors.is_empty() {
            floor_index == self.active_floor
        } else {
            self.floors.iter().any(|f| f.floor_index == floor_index)
        }
    }

    /// Elevation offset in centimeters for `floor_index`.
    #[must_use]
    pub fn floor_elevation_cm(&self, floor_index: i32) -> i32 {
        self.floors
            .iter()
            .find(|f| f.floor_index == floor_index)
            .map_or(floor_index * 320, |f| f.elevation_cm)
    }

    /// Writes `self.wall_edges`, `self.props`, and `self.surfaces` back into the active [`MapFloorLevel`]
    /// entry so in-place edits persist across floor switches.
    pub fn sync_active_floor_snapshot(&mut self) {
        if let Some(cur) = self
            .floors
            .iter_mut()
            .find(|f| f.floor_index == self.active_floor)
        {
            cur.wall_edges = self.wall_edges.clone();
            cur.props = self.props.clone();
            cur.surfaces = self.surfaces.clone();
        }
    }

    /// Syncs current edits into `floors`, switches `active_floor` to `target_floor`, and loads that
    /// floor's geometry into the top-level `wall_edges`, `props`, and `surfaces` mirrors.
    pub fn switch_active_floor(&mut self, target_floor: i32) -> bool {
        if !self.floors.iter().any(|f| f.floor_index == target_floor) {
            return false;
        }
        self.sync_active_floor_snapshot();
        if let Some(target) = self.floors.iter().find(|f| f.floor_index == target_floor) {
            self.wall_edges = target.wall_edges.clone();
            self.props = target.props.clone();
            self.surfaces = target.surfaces.clone();
            self.active_floor = target_floor;
            self.tiles.clear();
            true
        } else {
            false
        }
    }

    /// Signed floor index occupied by `entity_id` (`active_floor` for single-floor maps).
    #[must_use]
    pub fn token_floor(&self, entity_id: &str) -> i32 {
        if !self.has_multiple_floors() {
            return self.active_floor;
        }
        if let Some(vis) = self.visual_for(entity_id)
            && self.has_floor(vis.floor_index)
        {
            return vis.floor_index;
        }
        if self.has_floor(0) {
            0
        } else {
            self.floors
                .first()
                .map_or(self.active_floor, |f| f.floor_index)
        }
    }

    /// Assigns `entity_id` to `floor_index` and updates its baseline `elevation_cm`.
    pub fn set_token_floor(&mut self, entity_id: &str, floor_index: i32) {
        let elev_cm = self.floor_elevation_cm(floor_index);
        if let Some(vis) = self
            .token_visuals
            .iter_mut()
            .find(|v| v.entity_id == entity_id)
        {
            vis.floor_index = floor_index;
            vis.elevation_cm = elev_cm;
        } else {
            self.token_visuals.push(TokenVisualSpec {
                entity_id: entity_id.to_string(),
                elevation_cm: elev_cm,
                floor_index,
                ..Default::default()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tactical_map_dressing_dto::{EdgeSide, MaterialClass, WallEdgeKind};

    #[test]
    fn multi_floor_dto_roundtrip_and_active_floor_switching_preserves_edits() {
        assert_eq!(default_floor_badge(-1), "B1");
        assert_eq!(default_floor_badge(0), "1F");
        assert_eq!(default_floor_badge(1), "2F");

        let edge_f1 = MapWallEdge {
            x: 2,
            y: 2,
            side: EdgeSide::North,
            kind: WallEdgeKind::Wall,
            material: MaterialClass::Brick,
        };
        let edge_f2 = MapWallEdge {
            x: 4,
            y: 4,
            side: EdgeSide::West,
            kind: WallEdgeKind::Window,
            material: MaterialClass::Glass,
        };
        let mut map = GmMapUpdateRequest {
            map_id: "mansion-1".to_string(),
            title: "Luxury Mansion".to_string(),
            grid_width: 20,
            grid_height: 14,
            active_floor: 0,
            wall_edges: vec![edge_f1],
            floors: vec![
                MapFloorLevel {
                    floor_index: 0,
                    label: "1F · Ground".to_string(),
                    elevation_cm: 0,
                    wall_edges: vec![edge_f1],
                    ..Default::default()
                },
                MapFloorLevel {
                    floor_index: 1,
                    label: "2F · Upper Suites".to_string(),
                    elevation_cm: 340,
                    wall_edges: vec![edge_f2],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert!(map.has_multiple_floors());
        assert_eq!(map.floors[1].short_badge(), "2F");
        // Mutate active floor 1F before switching to 2F.
        map.wall_edges.push(MapWallEdge {
            x: 3,
            y: 2,
            side: EdgeSide::North,
            kind: WallEdgeKind::DoorOpen,
            material: MaterialClass::SolidWood,
        });
        assert!(map.switch_active_floor(1));
        assert_eq!(map.active_floor, 1);
        assert_eq!(map.wall_edges, vec![edge_f2]);
        assert!(map.switch_active_floor(0));
        assert_eq!(map.wall_edges.len(), 2);
        assert!(!map.switch_active_floor(99));

        map.set_token_floor("tok-sara", 1);
        assert_eq!(map.token_floor("tok-sara"), 1);
        assert_eq!(map.visual_for("tok-sara").unwrap().elevation_cm, 340);

        let encoded = serde_json::to_string(&map).unwrap();
        let decoded: GmMapUpdateRequest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, map);
    }
}
