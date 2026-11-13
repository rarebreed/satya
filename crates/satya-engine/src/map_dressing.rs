//! # Multi-Cell Prop Catalog & 360° Oriented Footprints (`map_dressing`)
//!
//! Each [`PropKind`] has a [`PropSpec`] giving its canonical footprint (back facing North), its
//! [`PropObstruction`] tier (`FloorLayer`, `Concealment`, `HalfCover`, `FullObstacle`), its
//! movement-cost multiplier, [`MaterialClass`], height, and albedo. Per-material ballistic
//! penetration vs. concealment grading lives in [`crate::barrier_ballistics`], and continuous 2D
//! oriented-box / radial hit-testing lives in [`crate::tactical_map_geom`].

use satya_schema::{CoverGrade, MapPropPlacement, MaterialClass, PropKind};

/// Continuous obstruction category for a placed prop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropObstruction {
    /// Ground-level surfaces, rugs, catwalks, spills, and sit-on furnishings.
    FloorLayer,
    /// Sight-obscuring foliage that tokens can move through.
    Concealment,
    /// Waist-high solid obstacle providing half cover and blocking footsteps.
    HalfCover,
    /// Full-height solid obstacle blocking movement, cover, and sightlines.
    FullObstacle,
}

impl PropObstruction {
    /// Whether this obstruction tier blocks miniature movement and occupancy.
    #[must_use]
    pub const fn blocks_movement(self) -> bool {
        matches!(self, Self::HalfCover | Self::FullObstacle)
    }

    /// Cover grade contributed by this obstruction tier.
    #[must_use]
    pub const fn cover_grade(self) -> CoverGrade {
        match self {
            Self::FloorLayer => CoverGrade::Clear,
            Self::Concealment => CoverGrade::Concealment,
            Self::HalfCover => CoverGrade::LowCover,
            Self::FullObstacle => CoverGrade::Blocked,
        }
    }
}

/// Static mechanical and visual data for one [`PropKind`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropSpec {
    pub kind: PropKind,
    /// Canonical footprint width (East–West) in meters-cells (`100 cm`) with the back facing North.
    pub base_w_cells: i32,
    /// Canonical footprint depth (North–South) in meters-cells (`100 cm`).
    pub base_h_cells: i32,
    pub obstruction: PropObstruction,
    /// Continuous movement cost multiplier (`1.0` normal, `> 1.0` difficult/hazardous ground).
    pub move_cost_mult: f32,
    pub material: MaterialClass,
    pub height_cm: i32,
    /// Variable-length runs (counters, shelves, catwalks): width may exceed `base_w_cells`.
    pub stretchable: bool,
    /// Floor decals / ground layers that sit flush on the surface and never block prop stacking.
    pub floor_layer: bool,
    pub albedo_rgb: [f32; 3],
}

macro_rules! prop_catalog {
    ($( $kind:ident: $w:literal x $h:literal, $obs:ident, $mat:ident, $hcm:literal cm, [$r:literal, $g:literal, $b:literal] $(, $flag:ident)* );+ $(;)?) => {
        /// Prop catalog in [`PropKind::ALL`] order (indexed by `kind as usize`).
        pub const PROP_CATALOG: [PropSpec; PropKind::ALL.len()] = [
            $(PropSpec {
                kind: PropKind::$kind,
                base_w_cells: $w,
                base_h_cells: $h,
                obstruction: PropObstruction::$obs,
                move_cost_mult: if prop_catalog!(@has hazard $($flag)*) {
                    2.0
                } else if matches!(PropObstruction::$obs, PropObstruction::Concealment) {
                    1.25
                } else {
                    1.0
                },
                material: MaterialClass::$mat,
                height_cm: $hcm,
                stretchable: prop_catalog!(@has stretch $($flag)*),
                floor_layer: prop_catalog!(@has floor $($flag)*),
                albedo_rgb: [$r, $g, $b],
            }),+
        ];
    };
    (@has stretch stretch $($rest:ident)*) => { true };
    (@has floor floor $($rest:ident)*) => { true };
    (@has hazard hazard $($rest:ident)*) => { true };
    (@has $want:ident $other:ident $($rest:ident)*) => { prop_catalog!(@has $want $($rest)*) };
    (@has $want:ident) => { false };
}

prop_catalog! {
    Sofa: 3 x 1, HalfCover, Upholstery, 85 cm, [0.45, 0.52, 0.58];
    Loveseat: 2 x 1, HalfCover, Upholstery, 85 cm, [0.58, 0.42, 0.34];
    Armchair: 1 x 1, HalfCover, Upholstery, 90 cm, [0.50, 0.44, 0.55];
    CoffeeTable: 2 x 1, HalfCover, WoodFurniture, 45 cm, [0.55, 0.40, 0.28];
    Rug: 3 x 2, FloorLayer, Upholstery, 1 cm, [0.62, 0.30, 0.26], floor;
    DiningTable: 2 x 2, HalfCover, WoodFurniture, 76 cm, [0.60, 0.44, 0.30];
    RoundTable: 1 x 1, HalfCover, WoodFurniture, 76 cm, [0.42, 0.30, 0.22];
    Booth: 2 x 2, HalfCover, Upholstery, 110 cm, [0.52, 0.16, 0.16];
    Bed: 2 x 2, HalfCover, Upholstery, 60 cm, [0.80, 0.82, 0.84];
    Desk: 2 x 1, HalfCover, WoodFurniture, 76 cm, [0.58, 0.43, 0.30];
    Bookshelf: 2 x 1, FullObstacle, WoodFurniture, 200 cm, [0.45, 0.33, 0.22];
    Wardrobe: 2 x 1, FullObstacle, WoodFurniture, 200 cm, [0.52, 0.40, 0.30];
    KitchenCounter: 1 x 1, HalfCover, WoodFurniture, 92 cm, [0.84, 0.82, 0.78], stretch;
    Stove: 1 x 1, HalfCover, Appliance, 92 cm, [0.30, 0.31, 0.33];
    Fridge: 1 x 1, FullObstacle, Appliance, 180 cm, [0.86, 0.87, 0.88];
    BarCounter: 1 x 1, HalfCover, SolidWood, 110 cm, [0.36, 0.22, 0.14], stretch;
    BackBarShelf: 1 x 1, FullObstacle, WoodFurniture, 200 cm, [0.30, 0.20, 0.13], stretch;
    BarStool: 1 x 1, FloorLayer, WoodFurniture, 75 cm, [0.25, 0.25, 0.27];
    PoolTable: 3 x 2, HalfCover, SolidWood, 80 cm, [0.12, 0.42, 0.24];
    Jukebox: 1 x 1, FullObstacle, Appliance, 150 cm, [0.70, 0.30, 0.20];
    Bathtub: 2 x 1, HalfCover, SheetSteel, 55 cm, [0.92, 0.93, 0.94];
    Toilet: 1 x 1, HalfCover, Ceramic, 40 cm, [0.95, 0.95, 0.95];
    Planter: 1 x 1, HalfCover, Earth, 90 cm, [0.31, 0.48, 0.27];
    WoodCrate: 1 x 1, HalfCover, WoodFurniture, 90 cm, [0.72, 0.55, 0.33];
    DrumCluster: 1 x 1, HalfCover, SheetSteel, 90 cm, [0.25, 0.43, 0.58];
    SteelShelving: 2 x 1, FullObstacle, SheetSteel, 200 cm, [0.48, 0.52, 0.55];
    LargeTree: 2 x 2, FullObstacle, LiveWood, 600 cm, [0.36, 0.26, 0.18];
    Car: 2 x 5, HalfCover, VehicleBody, 145 cm, [0.30, 0.38, 0.52];
    MediumTree: 1 x 1, FullObstacle, LiveWood, 480 cm, [0.34, 0.25, 0.17];
    Hedge: 1 x 1, Concealment, Foliage, 130 cm, [0.22, 0.44, 0.20], stretch;
    Bush: 1 x 1, Concealment, Foliage, 105 cm, [0.27, 0.48, 0.23];
    Boulder: 2 x 1, HalfCover, ReinforcedConcrete, 115 cm, [0.52, 0.51, 0.49];
    ParkBench: 2 x 1, HalfCover, WoodFurniture, 85 cm, [0.52, 0.37, 0.24];
    PicnicTable: 2 x 2, HalfCover, SolidWood, 78 cm, [0.58, 0.42, 0.27];
    Dumpster: 2 x 1, HalfCover, SheetSteel, 135 cm, [0.20, 0.36, 0.26];
    JerseyBarrier: 2 x 1, HalfCover, ReinforcedConcrete, 95 cm, [0.66, 0.65, 0.62], stretch;
    UtilityVan: 2 x 5, HalfCover, VehicleBody, 188 cm, [0.78, 0.79, 0.80];
    StonePath: 1 x 1, FloorLayer, Ceramic, 2 cm, [0.74, 0.70, 0.63], stretch, floor;
    AsphaltDrive: 1 x 1, FloorLayer, ReinforcedConcrete, 2 cm, [0.33, 0.35, 0.37], stretch, floor;
    DeskL: 2 x 2, HalfCover, WoodFurniture, 76 cm, [0.58, 0.43, 0.30];
    DeskT: 3 x 2, HalfCover, WoodFurniture, 76 cm, [0.56, 0.41, 0.29];
    FallenLog: 3 x 1, HalfCover, LiveWood, 75 cm, [0.40, 0.28, 0.17];
    CreekWater: 1 x 1, FloorLayer, Earth, 4 cm, [0.20, 0.46, 0.58], stretch, floor, hazard;
    DrainageDitch: 1 x 1, FloorLayer, Earth, 6 cm, [0.38, 0.31, 0.23], stretch, floor, hazard;
    GardenBed: 2 x 1, FloorLayer, Foliage, 25 cm, [0.30, 0.46, 0.24], stretch, floor;
    GardenEdging: 1 x 1, FloorLayer, Brick, 12 cm, [0.64, 0.56, 0.48], stretch, floor;
    GravelPath: 1 x 1, FloorLayer, Ceramic, 2 cm, [0.62, 0.61, 0.58], stretch, floor;
    DirtPath: 1 x 1, FloorLayer, Earth, 2 cm, [0.54, 0.42, 0.30], stretch, floor;
    MulchBed: 1 x 1, FloorLayer, Earth, 3 cm, [0.34, 0.22, 0.14], stretch, floor;
    StructuralPillar: 1 x 1, FullObstacle, ReinforcedConcrete, 300 cm, [0.56, 0.56, 0.58];
    ElevatedCatwalk: 1 x 1, FloorLayer, SheetSteel, 12 cm, [0.46, 0.50, 0.54], stretch, floor;
    HazardSpill: 1 x 1, FloorLayer, Ceramic, 2 cm, [0.78, 0.62, 0.16], stretch, floor, hazard;
}

/// Catalog entry for `kind`.
#[must_use]
pub const fn prop_spec(kind: PropKind) -> &'static PropSpec {
    &PROP_CATALOG[kind as usize]
}

/// Normalizes any integer angle into `[0, 360)` with full `1°` (`360°`) freedom.
#[must_use]
pub const fn normalize_prop_rotation_deg(_kind: PropKind, rotation_deg: i32) -> i32 {
    rotation_deg.rem_euclid(360)
}

/// World-space AABB half-extents in centimeters for a canonical `(span_w_cm, span_h_cm)` prop rotated `rotation_deg` clockwise.
#[must_use]
pub fn rotated_aabb_half_extents_cm(span_w_cm: i32, span_h_cm: i32, rotation_deg: i32) -> (f32, f32) {
    let r = rotation_deg.rem_euclid(360);
    let sw = span_w_cm.max(1) as f32;
    let sh = span_h_cm.max(1) as f32;
    if r % 180 == 0 {
        (sw * 0.5, sh * 0.5)
    } else if r % 180 == 90 {
        (sh * 0.5, sw * 0.5)
    } else {
        let rad = (r as f32).to_radians();
        let c = rad.cos().abs();
        let s = rad.sin().abs();
        (0.5 * (sw * c + sh * s), 0.5 * (sw * s + sh * c))
    }
}

/// Placed enclosing `(w_cells, h_cells)` for a canonical `(base_w, base_h)` turned `rotation_deg` clockwise.
#[must_use]
pub fn rotated_footprint(base_w: i32, base_h: i32, rotation_deg: i32) -> (i32, i32) {
    let r = rotation_deg.rem_euclid(360);
    if r % 180 == 0 {
        (base_w.max(1), base_h.max(1))
    } else if r % 180 == 90 {
        (base_h.max(1), base_w.max(1))
    } else {
        let (hw, hh) = rotated_aabb_half_extents_cm(base_w * 100, base_h * 100, r);
        (
            ((hw * 2.0 - 1e-3) / 100.0).ceil().max(1.0) as i32,
            ((hh * 2.0 - 1e-3) / 100.0).ceil().max(1.0) as i32,
        )
    }
}

/// Builds a placement anchored with its enclosing box top-left at `(x, y)` cells and exact continuous center/span metadata.
/// `length_cells` overrides the canonical length of stretchable props at any angle.
#[must_use]
pub fn make_prop(
    kind: PropKind,
    x: i32,
    y: i32,
    rotation_deg: i32,
    length_cells: Option<i32>,
    variant_seed: u32,
) -> MapPropPlacement {
    let spec = prop_spec(kind);
    let rot = normalize_prop_rotation_deg(kind, rotation_deg);
    let base_w = match length_cells {
        Some(l) if spec.stretchable => l.max(1),
        _ => spec.base_w_cells,
    };
    let (w_cells, h_cells) = rotated_footprint(base_w, spec.base_h_cells, rot);
    let span_w_cm = base_w * 100;
    let span_h_cm = spec.base_h_cells * 100;
    MapPropPlacement {
        kind,
        x,
        y,
        w_cells,
        h_cells,
        rotation_deg: rot,
        variant_seed,
        center_x_cm: Some(x * 100 + w_cells * 50),
        center_y_cm: Some(y * 100 + h_cells * 50),
        span_w_cm: Some(span_w_cm),
        span_h_cm: Some(span_h_cm),
    }
}

/// Builds a placement centered at continuous `(center_x_cm, center_y_cm)` with full `360°` orientation
/// and an enclosing integer `(x, y, w_cells, h_cells)` AABB.
#[must_use]
pub fn make_prop_at_cm(
    kind: PropKind,
    center_x_cm: i32,
    center_y_cm: i32,
    rotation_deg: i32,
    length_cm: Option<i32>,
    variant_seed: u32,
) -> MapPropPlacement {
    let spec = prop_spec(kind);
    let rot = normalize_prop_rotation_deg(kind, rotation_deg);
    let span_w_cm = match length_cm {
        Some(l) if spec.stretchable => l.max(25),
        _ => spec.base_w_cells * 100,
    };
    let span_h_cm = spec.base_h_cells * 100;
    let (hw, hh) = rotated_aabb_half_extents_cm(span_w_cm, span_h_cm, rot);
    let cx = center_x_cm as f32;
    let cy = center_y_cm as f32;
    let min_x = ((cx - hw + 1e-2) / 100.0).floor() as i32;
    let min_y = ((cy - hh + 1e-2) / 100.0).floor() as i32;
    let max_x = ((cx + hw - 1e-2) / 100.0).ceil() as i32;
    let max_y = ((cy + hh - 1e-2) / 100.0).ceil() as i32;
    MapPropPlacement {
        kind,
        x: min_x,
        y: min_y,
        w_cells: (max_x - min_x).max(1),
        h_cells: (max_y - min_y).max(1),
        rotation_deg: rot,
        variant_seed,
        center_x_cm: Some(center_x_cm),
        center_y_cm: Some(center_y_cm),
        span_w_cm: Some(span_w_cm),
        span_h_cm: Some(span_h_cm),
    }
}

/// Whether cell `(x, y)` is covered by a hazardous floor-layer prop (`CreekWater`, `DrainageDitch`, or `HazardSpill`).
#[must_use]
pub fn terrain_hazard_prop_at(props: &[MapPropPlacement], x: i32, y: i32) -> bool {
    props.iter().any(|p| {
        matches!(p.kind, PropKind::CreekWater | PropKind::DrainageDitch | PropKind::HazardSpill) && p.covers(x, y)
    })
}

/// Index and reference of the solid (non-floor-layer) prop covering `(x, y)`, if any.
#[must_use]
pub fn solid_prop_index_at(props: &[MapPropPlacement], x: i32, y: i32) -> Option<(usize, &MapPropPlacement)> {
    props
        .iter()
        .enumerate()
        .find(|(_, p)| !prop_spec(p.kind).floor_layer && p.covers(x, y))
}

/// The solid (non-floor-layer) prop covering `(x, y)`, if any.
#[must_use]
pub fn solid_prop_at(props: &[MapPropPlacement], x: i32, y: i32) -> Option<&MapPropPlacement> {
    solid_prop_index_at(props, x, y).map(|(_, p)| p)
}

/// Whether two prop placements physically overlap inside at least one enclosing grid cell.
#[must_use]
pub fn props_share_covered_cell(a: &MapPropPlacement, b: &MapPropPlacement) -> bool {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.w_cells).min(b.x + b.w_cells);
    let y1 = (a.y + a.h_cells).min(b.y + b.h_cells);
    for y in y0..y1 {
        for x in x0..x1 {
            if a.covers(x, y) && b.covers(x, y) {
                return true;
            }
        }
    }
    false
}

/// Whether cell `(gx, gy)` inside a [`PropKind::Car`] or [`PropKind::UtilityVan`] covers the front
/// hood / engine-block row (matching the headlight end in `map_2d_art_furniture`).
#[must_use]
pub fn vehicle_cell_is_engine_bay(p: &MapPropPlacement, gx: i32, gy: i32) -> bool {
    if !matches!(p.kind, PropKind::Car | PropKind::UtilityVan) || !p.covers(gx, gy) {
        return false;
    }
    let (cx_m, cy_m) = p.center_m();
    let (sw_m, sh_m) = p.local_span_m();
    let rad = (p.rotation_deg as f32).to_radians();
    let dx = (gx as f32 + 0.5) - cx_m;
    let dy = (gy as f32 + 0.5) - cy_m;
    let local_y = -dx * rad.sin() + dy * rad.cos();
    local_y >= sh_m * 0.5 - (sh_m * 0.22).max(0.85).min(sw_m * 0.65)
}

/// Effective [`MaterialClass`] of `p` at cell `(gx, gy)` (upgrades a vehicle's front hood cell from
/// thin [`MaterialClass::VehicleBody`] sheet metal to [`MaterialClass::ArmorSteel`] engine block).
#[must_use]
pub fn prop_effective_material_at(p: &MapPropPlacement, gx: i32, gy: i32) -> MaterialClass {
    if vehicle_cell_is_engine_bay(p, gx, gy) {
        MaterialClass::ArmorSteel
    } else {
        prop_spec(p.kind).material
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_order_continuous_angles_and_metadata() {
        for (i, k) in PropKind::ALL.iter().enumerate() {
            assert_eq!(PROP_CATALOG[i].kind, *k, "PROP_CATALOG out of order at {i}");
        }
        assert!(prop_spec(PropKind::BarCounter).stretchable && prop_spec(PropKind::Rug).floor_layer);
        assert!(prop_spec(PropKind::ElevatedCatwalk).floor_layer && prop_spec(PropKind::ElevatedCatwalk).stretchable);
        assert_eq!(prop_spec(PropKind::StructuralPillar).obstruction, PropObstruction::FullObstacle);
        assert_eq!(prop_spec(PropKind::HazardSpill).move_cost_mult, 2.0);
        assert_eq!(normalize_prop_rotation_deg(PropKind::Sofa, 23), 23);
        assert_eq!(normalize_prop_rotation_deg(PropKind::Car, -30), 330);

        let sofa = make_prop(PropKind::Sofa, 2, 1, 90, None, 0);
        assert_eq!((sofa.w_cells, sofa.h_cells), (1, 3));
        assert_eq!(sofa.center_cm(), (250, 250));
        assert_eq!(sofa.local_span_cm(), (300, 100));

        let custom = make_prop_at_cm(PropKind::Car, 550, 420, 33, None, 7);
        assert_eq!(custom.rotation_deg, 33);
        assert_eq!(custom.center_cm(), (550, 420));
        assert_eq!(custom.local_span_cm(), (200, 500));
        assert!(custom.w_cells >= 4 && custom.h_cells >= 5);
    }

    #[test]
    fn diagonal_logs_compound_desks_and_hazard_props() {
        let desk_l = make_prop(PropKind::DeskL, 2, 1, 0, None, 1);
        assert!(desk_l.covers(2, 1) && desk_l.covers(3, 1) && desk_l.covers(2, 2));
        assert!(!desk_l.covers(3, 2));

        let desk_t = make_prop(PropKind::DeskT, 5, 1, 0, None, 2);
        assert!(desk_t.covers(6, 1) && desk_t.covers(6, 2));
        assert!(!desk_t.covers(5, 2) && !desk_t.covers(7, 2));

        let log45 = make_prop(PropKind::FallenLog, 1, 4, 45, None, 3);
        assert_eq!((log45.w_cells, log45.h_cells, log45.rotation_deg), (3, 3, 45));
        assert!(log45.covers(1, 4) && log45.covers(2, 5) && log45.covers(3, 6));
        assert!(!log45.covers(3, 4) && !log45.covers(1, 6));

        let creek = make_prop(PropKind::CreekWater, 0, 7, 0, Some(4), 4);
        let spill = make_prop(PropKind::HazardSpill, 4, 7, 0, Some(2), 5);
        assert!(terrain_hazard_prop_at(&[creek, spill], 2, 7));
        assert!(terrain_hazard_prop_at(&[creek, spill], 5, 7));
    }
}
