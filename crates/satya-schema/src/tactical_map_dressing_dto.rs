//! Tactical Map Dressing Wire DTOs: thin **edge walls** (walls, doors, windows on cell boundaries),
//! **multi-cell props** (sofas, bar counters, beds, trees, vehicles) and their **material classes**.
//!
//! Re-exported through [`crate::tactical_map_dto`]. Mechanical behavior (prop footprints, stamped
//! `TacticalTileKind`s, and edge-aware LoS/pathing) lives in `satya-engine::{map_dressing, tactical_map_edges}`.
//!
//! Edge addressing: every cell `(x, y)` owns its **North** edge (shared with `(x, y - 1)`) and its
//! **West** edge (shared with `(x - 1, y)`). The board's South/East borders use `y = grid_height` /
//! `x = grid_width`.

use serde::{Deserialize, Serialize};

/// Declares a fieldless `SCREAMING_SNAKE_CASE` wire enum with `ALL`, `as_str`, and `label`.
macro_rules! define_wire_enum {
    (
        $(#[$meta:meta])*
        pub enum $Name:ident [count = $Count:expr] {
            $( $(#[$vmeta:meta])* $Variant:ident => ($code:literal, $label:literal) ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $Name { $( $(#[$vmeta])* $Variant ),+ }

        impl $Name {
            pub const ALL: [Self; $Count] = [$(Self::$Variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$Variant => $code),+ }
            }

            #[must_use]
            pub const fn label(self) -> &'static str {
                match self { $(Self::$Variant => $label),+ }
            }
        }
    };
}

define_wire_enum! {
    /// What occupies a cell-boundary edge.
    pub enum WallEdgeKind [count = 4] {
        Wall => ("WALL", "Wall"),
        DoorClosed => ("DOOR_CLOSED", "Closed Door"),
        DoorOpen => ("DOOR_OPEN", "Open Door"),
        Window => ("WINDOW", "Window (waist-high sill)"),
    }
}

impl WallEdgeKind {
    /// Whether a combatant cannot step across this edge.
    #[must_use]
    pub const fn blocks_movement(self) -> bool {
        matches!(self, Self::Wall | Self::DoorClosed | Self::Window)
    }

    /// Whether this edge completely blocks Line of Sight.
    #[must_use]
    pub const fn blocks_sight(self) -> bool {
        matches!(self, Self::Wall | Self::DoorClosed)
    }

    /// Whether shooting across this edge imposes Low Cover (window sill / frame).
    #[must_use]
    pub const fn provides_half_cover(self) -> bool {
        matches!(self, Self::Window)
    }
}

define_wire_enum! {
    /// Which of a cell's two owned edges an edge record refers to.
    pub enum EdgeSide [count = 2] {
        North => ("NORTH", "North edge"),
        West => ("WEST", "West edge"),
    }
}

define_wire_enum! {
    /// Physical construction material of a wall edge or prop, graded against projectile caliber,
    /// impact energy, and penetration in `satya_engine::barrier_ballistics` (`0 PR` concealment through `10 PR` armor steel).
    #[derive(Default)]
    pub enum MaterialClass [count = 16] {
        #[default]
        Drywall => ("DRYWALL", "Drywall / stud partition"),
        Brick => ("BRICK", "Brick masonry"),
        ConcreteBlock => ("CONCRETE_BLOCK", "Concrete block (CMU)"),
        ReinforcedConcrete => ("REINFORCED_CONCRETE", "Reinforced concrete"),
        Glass => ("GLASS", "Glass"),
        SolidWood => ("SOLID_WOOD", "Solid hardwood"),
        WoodFurniture => ("WOOD_FURNITURE", "Wood furniture / particleboard"),
        Upholstery => ("UPHOLSTERY", "Upholstered furniture"),
        SheetSteel => ("SHEET_STEEL", "Sheet steel"),
        ArmorSteel => ("ARMOR_STEEL", "Armor-grade steel plate"),
        Appliance => ("APPLIANCE", "Appliance (sheet steel + internals)"),
        Ceramic => ("CERAMIC", "Ceramic / porcelain"),
        Earth => ("EARTH", "Packed earth / soil-filled planter"),
        Foliage => ("FOLIAGE", "Foliage (concealment only)"),
        LiveWood => ("LIVE_WOOD", "Living tree trunk"),
        VehicleBody => ("VEHICLE_BODY", "Vehicle body panels"),
    }
}

define_wire_enum! {
    /// Catalog of placeable multi-cell props. Footprints, stamped tiles, heights, and materials are in
    /// `satya_engine::map_dressing::prop_spec`.
    pub enum PropKind [count = 39] {
        Sofa => ("SOFA", "Sofa (3-seat)"),
        Loveseat => ("LOVESEAT", "Loveseat"),
        Armchair => ("ARMCHAIR", "Armchair"),
        CoffeeTable => ("COFFEE_TABLE", "Coffee table"),
        Rug => ("RUG", "Area rug"),
        DiningTable => ("DINING_TABLE", "Dining table & chairs"),
        RoundTable => ("ROUND_TABLE", "Bistro table & chairs"),
        Booth => ("BOOTH", "Booth seating"),
        Bed => ("BED", "Double bed"),
        Desk => ("DESK", "Desk & chair"),
        Bookshelf => ("BOOKSHELF", "Bookshelf"),
        Wardrobe => ("WARDROBE", "Wardrobe"),
        KitchenCounter => ("KITCHEN_COUNTER", "Kitchen counter"),
        Stove => ("STOVE", "Stove / range"),
        Fridge => ("FRIDGE", "Refrigerator"),
        BarCounter => ("BAR_COUNTER", "Bar counter"),
        BackBarShelf => ("BACK_BAR_SHELF", "Back-bar bottle shelf"),
        BarStool => ("BAR_STOOL", "Bar stool"),
        PoolTable => ("POOL_TABLE", "Pool table"),
        Jukebox => ("JUKEBOX", "Jukebox"),
        Bathtub => ("BATHTUB", "Bathtub"),
        Toilet => ("TOILET", "Toilet"),
        Planter => ("PLANTER", "Planter"),
        WoodCrate => ("WOOD_CRATE", "Wooden crate"),
        DrumCluster => ("DRUM_CLUSTER", "Steel drums"),
        SteelShelving => ("STEEL_SHELVING", "Steel shelving"),
        LargeTree => ("LARGE_TREE", "Large tree"),
        Car => ("CAR", "Sedan"),
        MediumTree => ("MEDIUM_TREE", "Tree"),
        Hedge => ("HEDGE", "Trimmed hedge"),
        Bush => ("BUSH", "Shrub / bush"),
        Boulder => ("BOULDER", "Granite boulder"),
        ParkBench => ("PARK_BENCH", "Park bench"),
        PicnicTable => ("PICNIC_TABLE", "Picnic table"),
        Dumpster => ("DUMPSTER", "Commercial dumpster"),
        JerseyBarrier => ("JERSEY_BARRIER", "Concrete Jersey barrier"),
        UtilityVan => ("UTILITY_VAN", "Utility van / SUV"),
        StonePath => ("STONE_PATH", "Flagstone path"),
        AsphaltDrive => ("ASPHALT_DRIVE", "Asphalt road / drive"),
    }
}

/// One wall, door, or window on a cell-boundary edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapWallEdge {
    pub x: i32,
    pub y: i32,
    pub side: EdgeSide,
    pub kind: WallEdgeKind,
    #[serde(default)]
    pub material: MaterialClass,
}

/// Address of a single cell-boundary edge (`(x, y)` + owned `side`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MapEdgeRef {
    pub x: i32,
    pub y: i32,
    pub side: EdgeSide,
}

/// A placed multi-cell prop. `(x, y)` is the top-left cell of the **placed** footprint
/// (`w_cells × h_cells`, already rotated). `rotation_deg` (`0/90/180/270`) is the clockwise turn from the
/// canonical orientation, where the prop's back faces North and its front faces South.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapPropPlacement {
    pub kind: PropKind,
    pub x: i32,
    pub y: i32,
    pub w_cells: i32,
    pub h_cells: i32,
    #[serde(default)]
    pub rotation_deg: i32,
    #[serde(default)]
    pub variant_seed: u32,
}

impl MapPropPlacement {
    /// Whether cell `(cx, cy)` lies inside this prop's placed footprint.
    #[must_use]
    pub const fn covers(&self, cx: i32, cy: i32) -> bool {
        cx >= self.x && cy >= self.y && cx < self.x + self.w_cells && cy < self.y + self.h_cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dressing_wire_codes_roundtrip_and_edge_semantics() {
        let edge = MapWallEdge {
            x: 3,
            y: 4,
            side: EdgeSide::West,
            kind: WallEdgeKind::Window,
            material: MaterialClass::Glass,
        };
        let json = serde_json::to_string(&edge).unwrap();
        assert!(json.contains(r#""side":"WEST""#) && json.contains(r#""kind":"WINDOW""#));
        assert_eq!(serde_json::from_str::<MapWallEdge>(&json).unwrap(), edge);
        let legacy: MapWallEdge =
            serde_json::from_str(r#"{"x":0,"y":0,"side":"NORTH","kind":"WALL"}"#).unwrap();
        assert_eq!(legacy.material, MaterialClass::Drywall);
        assert!(WallEdgeKind::Window.blocks_movement() && !WallEdgeKind::Window.blocks_sight());
        assert!(!WallEdgeKind::DoorOpen.blocks_movement());
        for k in PropKind::ALL {
            assert_eq!(serde_json::to_string(&k).unwrap(), format!("\"{}\"", k.as_str()));
        }
        let p = MapPropPlacement {
            kind: PropKind::BarCounter,
            x: 2,
            y: 1,
            w_cells: 5,
            h_cells: 1,
            rotation_deg: 0,
            variant_seed: 0,
        };
        assert!(p.covers(6, 1) && !p.covers(7, 1) && !p.covers(2, 2));
    }
}
