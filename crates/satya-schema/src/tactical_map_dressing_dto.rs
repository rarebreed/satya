//! Tactical Map Dressing Wire DTOs: thin **edge walls** (walls, doors, windows on cell boundaries),
//! **multi-cell props** (sofas, bar counters, beds, trees, vehicles) and their **material classes**.
//!
//! Re-exported through [`crate::tactical_map_dto`]. Mechanical behavior (prop footprints, stamped
//! `TacticalTileKind`s, and edge-aware LoS/pathing) lives in `satya-engine::{map_dressing, tactical_map_edges}`.
//!
//! Edge addressing: every cell `(x, y)` owns its **North** edge (shared with `(x, y - 1)`) and its
//! **West** edge (shared with `(x - 1, y)`). The board's South/East borders use `y = grid_height` /
//! `x = grid_width`.

use crate::define_wire_enum;
use serde::{Deserialize, Serialize};

const fn default_one_cell() -> i32 {
    1
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
        SolidWood => ("SOLID_WOOD", "Solid hardwood", aliases: ["SOLID_OAK", "OAK", "WOOD"]),
        WoodFurniture => ("WOOD_FURNITURE", "Wood furniture / particleboard"),
        Upholstery => ("UPHOLSTERY", "Upholstered furniture"),
        SheetSteel => ("SHEET_STEEL", "Sheet steel"),
        ArmorSteel => ("ARMOR_STEEL", "Armor-grade steel plate"),
        Appliance => ("APPLIANCE", "Appliance (sheet steel + internals)"),
        Ceramic => ("CERAMIC", "Ceramic / porcelain"),
        Earth => ("EARTH", "Packed earth / soil-filled planter"),
        Foliage => ("FOLIAGE", "Foliage (concealment only)", aliases: ["NONE"]),
        LiveWood => ("LIVE_WOOD", "Living tree trunk"),
        VehicleBody => ("VEHICLE_BODY", "Vehicle body panels"),
    }
}

define_wire_enum! {
    /// Catalog of placeable continuous props & surface zones. Dimensions, heights, and materials are in
    /// `satya_engine::map_dressing::prop_spec`.
    pub enum PropKind [count = 55] {
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
        DeskL => ("DESK_L", "L-shaped corner desk"),
        DeskT => ("DESK_T", "T-shaped workstation desk"),
        FallenLog => ("FALLEN_LOG", "Fallen log"),
        CreekWater => ("CREEK_WATER", "Creek / shallow stream"),
        DrainageDitch => ("DRAINAGE_DITCH", "Drainage ditch / trench"),
        GardenBed => ("GARDEN_BED", "Decorative garden bed"),
        GardenEdging => ("GARDEN_EDGING", "Landscape stone edging"),
        GravelPath => ("GRAVEL_PATH", "Gravel surface / path"),
        DirtPath => ("DIRT_PATH", "Packed dirt surface / trail"),
        MulchBed => ("MULCH_BED", "Bark mulch ground"),
        StructuralPillar => ("STRUCTURAL_PILLAR", "Structural bulkhead / pillar"),
        ElevatedCatwalk => ("ELEVATED_CATWALK", "Elevated gantry / catwalk"),
        HazardSpill => ("HAZARD_SPILL", "Chemical / plasma hazard spill"),
        Stairwell => ("STAIRWELL", "Stairwell"),
        DisplayRack => ("DISPLAY_RACK", "Retail display gondola"),
        CheckoutCounter => ("CHECKOUT_COUNTER", "Checkout counter"),
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

/// Precomputed continuous world-space center, canonical unrotated span/half-extents,
/// and `0..360°` clockwise rotation basis for a [`MapPropPlacement`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropFootprintM {
    pub cx_m: f32,
    pub cy_m: f32,
    pub span_w_m: f32,
    pub span_h_m: f32,
    pub hx_m: f32,
    pub hy_m: f32,
    pub rot_deg: i32,
    pub rot_rad: f32,
    pub sin_rot: f32,
    pub cos_rot: f32,
}

impl PropFootprintM {
    /// Transforms a world point `(wx_m, wy_m)` into this prop's unrotated local frame (`cx_m, cy_m = 0, 0`).
    #[must_use]
    pub fn world_to_local(self, wx_m: f32, wy_m: f32) -> (f32, f32) {
        let dx = wx_m - self.cx_m;
        let dy = wy_m - self.cy_m;
        match self.rot_deg {
            0 => (dx, dy),
            90 => (dy, -dx),
            180 => (-dx, -dy),
            270 => (-dy, dx),
            _ => (
                dx * self.cos_rot + dy * self.sin_rot,
                -dx * self.sin_rot + dy * self.cos_rot,
            ),
        }
    }

    /// Rotates a local direction vector `(lx_m, ly_m)` by `rot_deg` clockwise into world space.
    #[must_use]
    pub fn local_dir_to_world(self, lx_m: f32, ly_m: f32) -> (f32, f32) {
        match self.rot_deg {
            0 => (lx_m, ly_m),
            90 => (-ly_m, lx_m),
            180 => (-lx_m, -ly_m),
            270 => (ly_m, -lx_m),
            _ => (
                lx_m * self.cos_rot - ly_m * self.sin_rot,
                lx_m * self.sin_rot + ly_m * self.cos_rot,
            ),
        }
    }

    /// Maps a local-frame offset `(ox_m, oy_m)` into world coordinates `(wx_m, wy_m)`.
    #[must_use]
    pub fn local_offset_to_world(self, ox_m: f32, oy_m: f32) -> (f32, f32) {
        let (dwx, dwy) = self.local_dir_to_world(ox_m, oy_m);
        (self.cx_m + dwx, self.cy_m + dwy)
    }
}

/// A placed continuous prop. `(x, y)` and `(w_cells, h_cells)` represent the coarse integer AABB in
/// meter cells for quick spatial indexing and legacy consumers, while optional `center_x_cm` /
/// `center_y_cm` and canonical unrotated `span_w_cm` / `span_h_cm` store exact metric coordinates.
/// `rotation_deg` supports full `0..360°` clockwise orientation where back faces North (`-Y`) at `0°`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapPropPlacement {
    pub kind: PropKind,
    #[serde(default, alias = "origin_x")]
    pub x: i32,
    #[serde(default, alias = "origin_y")]
    pub y: i32,
    #[serde(default = "default_one_cell", alias = "span_w")]
    pub w_cells: i32,
    #[serde(default = "default_one_cell", alias = "span_h")]
    pub h_cells: i32,
    #[serde(default)]
    pub rotation_deg: i32,
    #[serde(default)]
    pub variant_seed: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_x_cm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_y_cm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_w_cm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_h_cm: Option<i32>,
}

impl MapPropPlacement {
    /// Exact center coordinate `(cx_cm, cy_cm)` in centimeters (`100 cm = 1.00 m`).
    #[must_use]
    pub const fn center_cm(&self) -> (i32, i32) {
        let cx = match self.center_x_cm {
            Some(v) => v,
            None => self.x * 100 + self.w_cells * 50,
        };
        let cy = match self.center_y_cm {
            Some(v) => v,
            None => self.y * 100 + self.h_cells * 50,
        };
        (cx, cy)
    }

    /// Exact center coordinate `(cx_m, cy_m)` in meters.
    #[must_use]
    pub fn center_m(&self) -> (f32, f32) {
        let (cx_cm, cy_cm) = self.center_cm();
        (cx_cm as f32 / 100.0, cy_cm as f32 / 100.0)
    }

    /// Canonical unrotated dimensions `(w_cm, h_cm)` in centimeters.
    #[must_use]
    pub const fn local_span_cm(&self) -> (i32, i32) {
        match (self.span_w_cm, self.span_h_cm) {
            (Some(w), Some(h)) => (if w > 10 { w } else { 10 }, if h > 10 { h } else { 10 }),
            _ => {
                let w = if self.w_cells > 0 { self.w_cells * 100 } else { 100 };
                let h = if self.h_cells > 0 { self.h_cells * 100 } else { 100 };
                if self.rotation_deg.rem_euclid(180) == 90 {
                    (h, w)
                } else {
                    (w, h)
                }
            }
        }
    }

    /// Canonical unrotated dimensions `(w_m, h_m)` in meters.
    #[must_use]
    pub fn local_span_m(&self) -> (f32, f32) {
        let (w_cm, h_cm) = self.local_span_cm();
        (w_cm as f32 / 100.0, h_cm as f32 / 100.0)
    }

    /// Computes canonical center, unrotated metric span/half-extents, and rotation basis for this prop.
    #[must_use]
    pub fn footprint_m(&self) -> PropFootprintM {
        let (cx_m, cy_m) = self.center_m();
        let (span_w_m, span_h_m) = self.local_span_m();
        let rot_deg = self.rotation_deg.rem_euclid(360);
        let rot_rad = (rot_deg as f32).to_radians();
        let (sin_rot, cos_rot) = rot_rad.sin_cos();
        PropFootprintM {
            cx_m,
            cy_m,
            span_w_m,
            span_h_m,
            hx_m: span_w_m * 0.5,
            hy_m: span_h_m * 0.5,
            rot_deg,
            rot_rad,
            sin_rot,
            cos_rot,
        }
    }

    /// Whether cell `(cx, cy)` lies inside this prop's axis-aligned bounding box.
    #[must_use]
    pub const fn covers_bbox(&self, cx: i32, cy: i32) -> bool {
        cx >= self.x && cy >= self.y && cx < self.x + self.w_cells && cy < self.y + self.h_cells
    }

    /// Whether cell `(cx, cy)` is physically occupied by this prop, honoring arbitrary `0..360°`
    /// rotations as well as legacy diagonal (`45°/135°`) and asymmetric compound shapes (`DeskL`, `DeskT`).
    #[must_use]
    pub fn covers(&self, cx: i32, cy: i32) -> bool {
        if !self.covers_bbox(cx, cy) {
            return false;
        }
        let rot = self.rotation_deg.rem_euclid(360);
        if rot % 90 != 0 && self.span_w_cm.is_none() && self.w_cells == self.h_cells && self.w_cells >= 2 {
            let rx = cx - self.x;
            let ry = cy - self.y;
            return if rot == 45 || rot == 225 {
                rx == ry
            } else {
                rx + ry == self.w_cells - 1
            };
        }
        if rot % 90 == 0 && self.span_w_cm.is_none() {
            let rx = cx - self.x;
            let ry = cy - self.y;
            let (lx, ly, bw, _bh) = match rot {
                90 => (ry, self.w_cells - 1 - rx, self.h_cells, self.w_cells),
                180 => (
                    self.w_cells - 1 - rx,
                    self.h_cells - 1 - ry,
                    self.w_cells,
                    self.h_cells,
                ),
                270 => (self.h_cells - 1 - ry, rx, self.h_cells, self.w_cells),
                _ => (rx, ry, self.w_cells, self.h_cells),
            };
            return match self.kind {
                PropKind::DeskL => ly == 0 || lx == 0,
                PropKind::DeskT => ly == 0 || lx == bw / 2,
                _ => true,
            };
        }

        // Continuous rotated local-frame check against cell center `(cx + 0.5, cy + 0.5)`.
        let fp = self.footprint_m();
        let (lx, ly) = fp.world_to_local(cx as f32 + 0.5, cy as f32 + 0.5);
        if lx.abs() > fp.hx_m + 0.05 || ly.abs() > fp.hy_m + 0.05 {
            return false;
        }
        match self.kind {
            PropKind::DeskL => lx <= -fp.hx_m + 1.02 || ly <= -fp.hy_m + 1.02,
            PropKind::DeskT => ly <= -fp.hy_m + 1.02 || lx.abs() <= 0.52,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn mk_prop(kind: PropKind, x: i32, y: i32, w_cells: i32, h_cells: i32, rotation_deg: i32) -> MapPropPlacement {
        MapPropPlacement {
            kind,
            x,
            y,
            w_cells,
            h_cells,
            rotation_deg,
            variant_seed: 0,
            center_x_cm: None,
            center_y_cm: None,
            span_w_cm: None,
            span_h_cm: None,
        }
    }

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
            assert_eq!(PropKind::from_code_str(&k.as_str().to_ascii_lowercase()), Some(k));
        }
        assert_eq!(PropKind::from_code_str("sofa"), Some(PropKind::Sofa));
        assert_eq!(PropKind::from_code_str("structural_pillar"), Some(PropKind::StructuralPillar));
        assert_eq!(PropKind::from_code_str("elevated_catwalk"), Some(PropKind::ElevatedCatwalk));
        assert_eq!(PropKind::from_code_str("hazard_spill"), Some(PropKind::HazardSpill));
        assert_eq!(PropKind::from_code_str("nonexistent_prop"), None);
        let p = mk_prop(PropKind::BarCounter, 2, 1, 5, 1, 0);
        assert!(p.covers(6, 1) && !p.covers(7, 1) && !p.covers(2, 2));
        assert_eq!(p.center_cm(), (450, 150));
        assert_eq!(p.local_span_cm(), (500, 100));
    }

    #[test]
    fn compound_l_t_desks_and_diagonal_props_cover_only_shape_cells() {
        // DeskL 2x2: at 0° SE corner (1,1) is open; at 90° SW (0,1); at 180° NW (0,0); at 270° NE (1,0)
        for (rot, open_xy) in [(0, (1, 1)), (90, (0, 1)), (180, (0, 0)), (270, (1, 0))] {
            let l_desk = mk_prop(PropKind::DeskL, 0, 0, 2, 2, rot);
            let covered: Vec<(i32, i32)> = (0..2)
                .flat_map(|y| (0..2).map(move |x| (x, y)))
                .filter(|&(x, y)| l_desk.covers(x, y))
                .collect();
            assert_eq!(covered.len(), 3, "rot={rot}");
            assert!(!l_desk.covers(open_xy.0, open_xy.1), "rot={rot} open={open_xy:?}");
        }

        // DeskT 3x2: at 0° (0,1) & (2,1) open; at 90° (2x3) (0,0) & (0,2) open
        let t0 = mk_prop(PropKind::DeskT, 0, 0, 3, 2, 0);
        assert!(t0.covers(0, 0) && t0.covers(1, 0) && t0.covers(2, 0) && t0.covers(1, 1));
        assert!(!t0.covers(0, 1) && !t0.covers(2, 1));

        let t90 = mk_prop(PropKind::DeskT, 0, 0, 2, 3, 90);
        assert!(t90.covers(1, 0) && t90.covers(1, 1) && t90.covers(1, 2) && t90.covers(0, 1));
        assert!(!t90.covers(0, 0) && !t90.covers(0, 2));

        let t180 = mk_prop(PropKind::DeskT, 0, 0, 3, 2, 180);
        assert!(t180.covers(0, 1) && t180.covers(1, 1) && t180.covers(2, 1) && t180.covers(1, 0));
        assert!(!t180.covers(0, 0) && !t180.covers(2, 0));

        let t270 = mk_prop(PropKind::DeskT, 0, 0, 2, 3, 270);
        assert!(t270.covers(0, 0) && t270.covers(0, 1) && t270.covers(0, 2) && t270.covers(1, 1));
        assert!(!t270.covers(1, 0) && !t270.covers(1, 2));

        // FallenLog 3x3 at 45° covers main diagonal; at 135° covers anti-diagonal
        let log45 = mk_prop(PropKind::FallenLog, 1, 1, 3, 3, 45);
        assert!(log45.covers(1, 1) && log45.covers(2, 2) && log45.covers(3, 3));
        assert!(!log45.covers(3, 1) && !log45.covers(1, 3) && log45.covers_bbox(3, 1));

        let log135 = mk_prop(PropKind::FallenLog, 1, 1, 3, 3, 135);
        assert!(log135.covers(3, 1) && log135.covers(2, 2) && log135.covers(1, 3));
        assert!(!log135.covers(1, 1) && !log135.covers(3, 3));

        // Free 37° continuous rotated sofa preserves unrotated canonical span
        let free_sofa = MapPropPlacement {
            kind: PropKind::Sofa,
            x: 3,
            y: 3,
            w_cells: 4,
            h_cells: 4,
            rotation_deg: 37,
            variant_seed: 9,
            center_x_cm: Some(525),
            center_y_cm: Some(475),
            span_w_cm: Some(300),
            span_h_cm: Some(100),
        };
        assert_eq!(free_sofa.center_cm(), (525, 475));
        assert_eq!(free_sofa.local_span_cm(), (300, 100));
    }

    #[test]
    fn prop_footprint_roundtrip_and_python_json_aliases() {
        for rot in [0, 37, 90, 215] {
            let p = MapPropPlacement {
                kind: PropKind::Sofa,
                x: 3,
                y: 2,
                w_cells: 3,
                h_cells: 2,
                rotation_deg: rot,
                variant_seed: 0,
                center_x_cm: Some(450),
                center_y_cm: Some(325),
                span_w_cm: Some(260),
                span_h_cm: Some(110),
            };
            let fp = p.footprint_m();
            let (wx, wy) = fp.local_offset_to_world(0.85, -0.35);
            let (lx, ly) = fp.world_to_local(wx, wy);
            assert!((lx - 0.85).abs() < 1e-4 && (ly - (-0.35)).abs() < 1e-4, "rot={rot}: ({lx}, {ly})");
        }

        let py_json = r#"{
            "kind": "JERSEY_BARRIER",
            "origin_x": 6,
            "origin_y": 4,
            "span_w": 2,
            "span_h": 1,
            "center_x_cm": 700,
            "center_y_cm": 475,
            "span_w_cm": 140,
            "span_h_cm": 80,
            "rotation_deg": 15
        }"#;
        let parsed: MapPropPlacement = serde_json::from_str(py_json).unwrap();
        assert_eq!(parsed.kind, PropKind::JerseyBarrier);
        assert_eq!((parsed.x, parsed.y, parsed.w_cells, parsed.h_cells), (6, 4, 2, 1));
        assert_eq!(parsed.center_cm(), (700, 475));
        assert_eq!(parsed.local_span_cm(), (140, 80));
        assert_eq!(MaterialClass::from_code_str("SOLID_OAK"), Some(MaterialClass::SolidWood));
    }
}
