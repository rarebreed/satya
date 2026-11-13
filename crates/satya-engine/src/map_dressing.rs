//! # Multi-Cell Prop Catalog & Footprint Stamping (`map_dressing`)
//!
//! Each [`PropKind`] has a [`PropSpec`] giving its canonical footprint (back facing North), the
//! mechanical [`TacticalTileKind`] stamped into every covered cell (`Concealment`, `HalfCover`, `FullWall`),
//! its [`MaterialClass`], height, and albedo. Per-material ballistic penetration vs. concealment grading
//! lives in [`crate::barrier_ballistics`].

use satya_schema::{MapPropPlacement, MaterialClass, PropKind, TacticalTileKind};

/// Static mechanical and visual data for one [`PropKind`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropSpec {
    pub kind: PropKind,
    /// Canonical footprint width (East–West) in cells with the back facing North.
    pub base_w_cells: i32,
    /// Canonical footprint depth (North–South) in cells.
    pub base_h_cells: i32,
    pub stamp: TacticalTileKind,
    pub material: MaterialClass,
    pub height_cm: i32,
    /// Variable-length runs (counters, shelves): `w_cells` may exceed `base_w_cells`.
    pub stretchable: bool,
    /// Floor decals (rugs) that never block placement or movement.
    pub floor_layer: bool,
    pub albedo_rgb: [f32; 3],
}

macro_rules! prop_catalog {
    ($( $kind:ident: $w:literal x $h:literal, $stamp:ident, $mat:ident, $hcm:literal cm, [$r:literal, $g:literal, $b:literal] $(, $flag:ident)* );+ $(;)?) => {
        /// Prop catalog in [`PropKind::ALL`] order (indexed by `kind as usize`).
        pub const PROP_CATALOG: [PropSpec; PropKind::ALL.len()] = [
            $(PropSpec {
                kind: PropKind::$kind,
                base_w_cells: $w,
                base_h_cells: $h,
                stamp: TacticalTileKind::$stamp,
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
    (@has $want:ident $other:ident $($rest:ident)*) => { prop_catalog!(@has $want $($rest)*) };
    (@has $want:ident) => { false };
}

prop_catalog! {
    Sofa: 3 x 1, HalfCover, Upholstery, 85 cm, [0.45, 0.52, 0.58];
    Loveseat: 2 x 1, HalfCover, Upholstery, 85 cm, [0.58, 0.42, 0.34];
    Armchair: 1 x 1, HalfCover, Upholstery, 90 cm, [0.50, 0.44, 0.55];
    CoffeeTable: 2 x 1, HalfCover, WoodFurniture, 45 cm, [0.55, 0.40, 0.28];
    Rug: 3 x 2, Floor, Upholstery, 1 cm, [0.62, 0.30, 0.26], floor;
    DiningTable: 2 x 2, HalfCover, WoodFurniture, 76 cm, [0.60, 0.44, 0.30];
    RoundTable: 1 x 1, HalfCover, WoodFurniture, 76 cm, [0.42, 0.30, 0.22];
    Booth: 2 x 2, HalfCover, Upholstery, 110 cm, [0.52, 0.16, 0.16];
    Bed: 2 x 2, HalfCover, Upholstery, 60 cm, [0.80, 0.82, 0.84];
    Desk: 2 x 1, HalfCover, WoodFurniture, 76 cm, [0.58, 0.43, 0.30];
    Bookshelf: 2 x 1, FullWall, WoodFurniture, 200 cm, [0.45, 0.33, 0.22];
    Wardrobe: 2 x 1, FullWall, WoodFurniture, 200 cm, [0.52, 0.40, 0.30];
    KitchenCounter: 1 x 1, HalfCover, WoodFurniture, 92 cm, [0.84, 0.82, 0.78], stretch;
    Stove: 1 x 1, HalfCover, Appliance, 92 cm, [0.30, 0.31, 0.33];
    Fridge: 1 x 1, FullWall, Appliance, 180 cm, [0.86, 0.87, 0.88];
    BarCounter: 1 x 1, HalfCover, SolidWood, 110 cm, [0.36, 0.22, 0.14], stretch;
    BackBarShelf: 1 x 1, FullWall, WoodFurniture, 200 cm, [0.30, 0.20, 0.13], stretch;
    BarStool: 1 x 1, Floor, WoodFurniture, 75 cm, [0.25, 0.25, 0.27];
    PoolTable: 3 x 2, HalfCover, SolidWood, 80 cm, [0.12, 0.42, 0.24];
    Jukebox: 1 x 1, FullWall, Appliance, 150 cm, [0.70, 0.30, 0.20];
    Bathtub: 2 x 1, HalfCover, SheetSteel, 55 cm, [0.92, 0.93, 0.94];
    Toilet: 1 x 1, HalfCover, Ceramic, 40 cm, [0.95, 0.95, 0.95];
    Planter: 1 x 1, HalfCover, Earth, 90 cm, [0.31, 0.48, 0.27];
    WoodCrate: 1 x 1, HalfCover, WoodFurniture, 90 cm, [0.72, 0.55, 0.33];
    DrumCluster: 1 x 1, HalfCover, SheetSteel, 90 cm, [0.25, 0.43, 0.58];
    SteelShelving: 2 x 1, FullWall, SheetSteel, 200 cm, [0.48, 0.52, 0.55];
    LargeTree: 2 x 2, FullWall, LiveWood, 600 cm, [0.36, 0.26, 0.18];
    Car: 2 x 5, HalfCover, VehicleBody, 145 cm, [0.30, 0.38, 0.52];
    MediumTree: 1 x 1, FullWall, LiveWood, 480 cm, [0.34, 0.25, 0.17];
    Hedge: 1 x 1, Concealment, Foliage, 130 cm, [0.22, 0.44, 0.20], stretch;
    Bush: 1 x 1, Concealment, Foliage, 105 cm, [0.27, 0.48, 0.23];
    Boulder: 2 x 1, HalfCover, ReinforcedConcrete, 115 cm, [0.52, 0.51, 0.49];
    ParkBench: 2 x 1, HalfCover, WoodFurniture, 85 cm, [0.52, 0.37, 0.24];
    PicnicTable: 2 x 2, HalfCover, SolidWood, 78 cm, [0.58, 0.42, 0.27];
    Dumpster: 2 x 1, HalfCover, SheetSteel, 135 cm, [0.20, 0.36, 0.26];
    JerseyBarrier: 2 x 1, HalfCover, ReinforcedConcrete, 95 cm, [0.66, 0.65, 0.62], stretch;
    UtilityVan: 2 x 5, HalfCover, VehicleBody, 188 cm, [0.78, 0.79, 0.80];
    StonePath: 1 x 1, Floor, Ceramic, 2 cm, [0.74, 0.70, 0.63], stretch, floor;
    AsphaltDrive: 1 x 1, Floor, ReinforcedConcrete, 2 cm, [0.33, 0.35, 0.37], stretch, floor;
}

/// Catalog entry for `kind`.
#[must_use]
pub const fn prop_spec(kind: PropKind) -> &'static PropSpec {
    &PROP_CATALOG[kind as usize]
}

/// Placed `(w_cells, h_cells)` for a canonical `(base_w, base_h)` turned `rotation_deg` clockwise.
#[must_use]
pub const fn rotated_footprint(base_w: i32, base_h: i32, rotation_deg: i32) -> (i32, i32) {
    if rotation_deg.rem_euclid(180) == 90 { (base_h, base_w) } else { (base_w, base_h) }
}

/// Builds a placement anchored at top-left `(x, y)`. `length_cells` overrides the width of stretchable props.
#[must_use]
pub fn make_prop(kind: PropKind, x: i32, y: i32, rotation_deg: i32, length_cells: Option<i32>, variant_seed: u32) -> MapPropPlacement {
    let spec = prop_spec(kind);
    let base_w = match length_cells {
        Some(l) if spec.stretchable => l.max(1),
        _ => spec.base_w_cells,
    };
    let rot = rotation_deg.rem_euclid(360) / 90 * 90;
    let (w_cells, h_cells) = rotated_footprint(base_w, spec.base_h_cells, rot);
    MapPropPlacement { kind, x, y, w_cells, h_cells, rotation_deg: rot, variant_seed }
}

const fn tile_rank(t: TacticalTileKind) -> u8 {
    match t {
        TacticalTileKind::Floor => 0,
        TacticalTileKind::HazardZone | TacticalTileKind::ElevatedPlatform => 1,
        TacticalTileKind::DoorOpen => 2,
        TacticalTileKind::Concealment => 3,
        TacticalTileKind::HalfCover => 4,
        TacticalTileKind::DoorClosed => 5,
        TacticalTileKind::FullWall => 6,
    }
}

/// Stamps every prop's mechanical tile into the cells it covers (the stronger obstruction wins; floor decals are skipped).
pub fn stamp_props_into_tiles(tiles: &mut [TacticalTileKind], width: i32, height: i32, props: &[MapPropPlacement]) {
    for p in props {
        let stamp = prop_spec(p.kind).stamp;
        if stamp == TacticalTileKind::Floor {
            continue;
        }
        for y in p.y.max(0)..(p.y + p.h_cells).min(height) {
            for x in p.x.max(0)..(p.x + p.w_cells).min(width) {
                let cell = &mut tiles[(y * width + x) as usize];
                if tile_rank(stamp) > tile_rank(*cell) {
                    *cell = stamp;
                }
            }
        }
    }
}

/// Index and reference of the solid (non-decal) prop covering `(x, y)`, if any.
#[must_use]
pub fn solid_prop_index_at(props: &[MapPropPlacement], x: i32, y: i32) -> Option<(usize, &MapPropPlacement)> {
    props
        .iter()
        .enumerate()
        .find(|(_, p)| !prop_spec(p.kind).floor_layer && p.covers(x, y))
}

/// The solid (non-decal) prop covering `(x, y)`, if any.
#[must_use]
pub fn solid_prop_at(props: &[MapPropPlacement], x: i32, y: i32) -> Option<&MapPropPlacement> {
    solid_prop_index_at(props, x, y).map(|(_, p)| p)
}

fn clear_prop_footprint_from_tiles(tiles: &mut [TacticalTileKind], width: i32, height: i32, prop: &MapPropPlacement) {
    let stamp = prop_spec(prop.kind).stamp;
    if stamp == TacticalTileKind::Floor {
        return;
    }
    for y in prop.y.max(0)..(prop.y + prop.h_cells).min(height) {
        for x in prop.x.max(0)..(prop.x + prop.w_cells).min(width) {
            let idx = (y * width + x) as usize;
            if let Some(cell) = tiles.get_mut(idx)
                && *cell == stamp
            {
                *cell = TacticalTileKind::Floor;
            }
        }
    }
}

/// Removes the solid prop (or topmost floor decal) covering `(gx, gy)`, resets its stamped tiles back
/// to [`TacticalTileKind::Floor`], and re-stamps remaining props so overlaps remain consistent.
pub fn remove_prop_at_on_grid(
    tiles: &mut [TacticalTileKind],
    props: &mut Vec<MapPropPlacement>,
    width: i32,
    height: i32,
    gx: i32,
    gy: i32,
) -> Option<MapPropPlacement> {
    let idx = solid_prop_index_at(props, gx, gy)
        .map(|(i, _)| i)
        .or_else(|| props.iter().rposition(|p| p.covers(gx, gy)))?;
    let removed = props.remove(idx);
    clear_prop_footprint_from_tiles(tiles, width, height, &removed);
    stamp_props_into_tiles(tiles, width, height, props);
    Some(removed)
}

/// Validates and places (or replaces overlapping props with) a prop on the tactical grid,
/// stamping its mechanical cover/obstruction tiles cleanly.
pub fn place_prop_on_grid(
    tiles: &mut [TacticalTileKind],
    props: &mut Vec<MapPropPlacement>,
    occupied_tokens: &[(i32, i32)],
    width: i32,
    height: i32,
    placement: MapPropPlacement,
) -> Result<MapPropPlacement, &'static str> {
    if width <= 0 || height <= 0 || tiles.len() < (width * height) as usize {
        return Err("Grid dimensions are invalid for prop placement");
    }
    let spec = prop_spec(placement.kind);
    let rot = placement.rotation_deg.rem_euclid(360) / 90 * 90;
    let stretch_len = if spec.stretchable {
        let raw = if rot % 180 == 90 { placement.h_cells } else { placement.w_cells };
        Some(raw.clamp(1, width.max(height).max(1)))
    } else {
        None
    };
    let norm = make_prop(placement.kind, placement.x, placement.y, rot, stretch_len, placement.variant_seed);

    if norm.x < 0 || norm.y < 0 || norm.x + norm.w_cells > width || norm.y + norm.h_cells > height {
        return Err("Prop footprint extends outside grid bounds");
    }
    if spec.stamp.blocks_occupancy()
        && occupied_tokens.iter().any(|&(tx, ty)| norm.covers(tx, ty))
    {
        return Err("Prop footprint overlaps an occupied miniature cell");
    }
    if !spec.floor_layer {
        for cy in norm.y..norm.y + norm.h_cells {
            for cx in norm.x..norm.x + norm.w_cells {
                let tile = tiles[(cy * width + cx) as usize];
                if matches!(tile, TacticalTileKind::FullWall | TacticalTileKind::DoorClosed)
                    && solid_prop_at(props, cx, cy).is_none()
                {
                    return Err("Prop footprint collides with a structural wall or closed door");
                }
            }
        }
    }

    if !spec.floor_layer {
        let mut kept = Vec::with_capacity(props.len() + 1);
        for existing in props.drain(..) {
            let overlaps = existing.x < norm.x + norm.w_cells
                && norm.x < existing.x + existing.w_cells
                && existing.y < norm.y + norm.h_cells
                && norm.y < existing.y + existing.h_cells;
            if !prop_spec(existing.kind).floor_layer && overlaps {
                clear_prop_footprint_from_tiles(tiles, width, height, &existing);
            } else {
                kept.push(existing);
            }
        }
        *props = kept;
    } else {
        props.retain(|p| {
            !(prop_spec(p.kind).floor_layer
                && p.kind == norm.kind
                && p.x == norm.x
                && p.y == norm.y
                && p.w_cells == norm.w_cells
                && p.h_cells == norm.h_cells)
        });
    }

    props.push(norm);
    stamp_props_into_tiles(tiles, width, height, props);
    Ok(norm)
}

/// Whether cell `(gx, gy)` inside a [`PropKind::Car`] or [`PropKind::UtilityVan`] covers the front
/// hood / engine-block row (matching the headlight end in `map_2d_art_furniture`).
#[must_use]
pub const fn vehicle_cell_is_engine_bay(p: &MapPropPlacement, gx: i32, gy: i32) -> bool {
    if !matches!(p.kind, PropKind::Car | PropKind::UtilityVan) || !p.covers(gx, gy) {
        return false;
    }
    match p.rotation_deg.rem_euclid(360) {
        0 => gy == p.y + p.h_cells - 1,
        90 => gx == p.x,
        180 => gy == p.y,
        270 => gx == p.x + p.w_cells - 1,
        _ => false,
    }
}

/// Effective [`MaterialClass`] of `p` at cell `(gx, gy)` (upgrades a vehicle's front hood cell from
/// thin [`MaterialClass::VehicleBody`] sheet metal to [`MaterialClass::ArmorSteel`] engine block).
#[must_use]
pub const fn prop_effective_material_at(p: &MapPropPlacement, gx: i32, gy: i32) -> MaterialClass {
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
    fn catalog_order_footprints_and_stamping() {
        for (i, k) in PropKind::ALL.iter().enumerate() {
            assert_eq!(PROP_CATALOG[i].kind, *k, "PROP_CATALOG out of order at {i}");
        }
        assert!(prop_spec(PropKind::BarCounter).stretchable && prop_spec(PropKind::Rug).floor_layer);
        assert!(!prop_spec(PropKind::Sofa).stretchable);

        let sofa = make_prop(PropKind::Sofa, 2, 1, 90, None, 0);
        assert_eq!((sofa.w_cells, sofa.h_cells), (1, 3));
        let bar = make_prop(PropKind::BarCounter, 1, 4, 0, Some(6), 0);
        assert_eq!((bar.w_cells, bar.h_cells), (6, 1));
        let rug = make_prop(PropKind::Rug, 0, 0, 0, None, 0);

        let (w, h) = (10, 8);
        let mut tiles = vec![TacticalTileKind::Floor; (w * h) as usize];
        let shelf = make_prop(PropKind::Bookshelf, 5, 6, 0, None, 0);
        stamp_props_into_tiles(&mut tiles, w, h, &[rug, sofa, bar, shelf]);
        assert_eq!(tiles[(3 * w + 2) as usize], TacticalTileKind::HalfCover);
        assert_eq!(tiles[(4 * w + 6) as usize], TacticalTileKind::HalfCover);
        assert_eq!(tiles[(6 * w + 6) as usize], TacticalTileKind::FullWall);
        assert_eq!(tiles[0], TacticalTileKind::Floor, "rugs never stamp");
        assert_eq!(solid_prop_at(&[rug, sofa], 2, 2).map(|p| p.kind), Some(PropKind::Sofa));
        assert!(solid_prop_at(&[rug], 0, 0).is_none());
    }

    #[test]
    fn place_and_remove_prop_on_grid_enforces_constraints_and_restamps() {
        let (w, h) = (10, 8);
        let mut tiles = vec![TacticalTileKind::Floor; (w * h) as usize];
        tiles[0] = TacticalTileKind::FullWall;
        let mut props = Vec::new();
        let occupied = [(5, 5)];

        // Bounds and collision checks
        assert!(place_prop_on_grid(&mut tiles, &mut props, &occupied, w, h, make_prop(PropKind::Sofa, 8, 1, 0, None, 1)).is_err());
        assert!(place_prop_on_grid(&mut tiles, &mut props, &occupied, w, h, make_prop(PropKind::Sofa, 4, 5, 0, None, 1)).is_err());
        assert!(place_prop_on_grid(&mut tiles, &mut props, &occupied, w, h, make_prop(PropKind::Sofa, 0, 0, 0, None, 1)).is_err());

        // Valid placement stamps Bookshelf FullWall across (2..4, 2)
        let shelf = place_prop_on_grid(&mut tiles, &mut props, &occupied, w, h, make_prop(PropKind::Bookshelf, 2, 2, 0, None, 9)).unwrap();
        assert_eq!((shelf.w_cells, shelf.h_cells), (2, 1));
        assert_eq!(tiles[(2 * w + 2) as usize], TacticalTileKind::FullWall);
        assert_eq!(tiles[(2 * w + 3) as usize], TacticalTileKind::FullWall);

        // Replacing overlapping solid prop downgrades previous FullWall stamp to Sofa HalfCover
        let sofa = place_prop_on_grid(&mut tiles, &mut props, &occupied, w, h, make_prop(PropKind::Sofa, 2, 2, 0, None, 3)).unwrap();
        assert_eq!((sofa.w_cells, sofa.h_cells), (3, 1));
        assert_eq!(props.len(), 1);
        assert_eq!(tiles[(2 * w + 2) as usize], TacticalTileKind::HalfCover);
        assert_eq!(tiles[(2 * w + 4) as usize], TacticalTileKind::HalfCover);

        // Removing the Sofa clears stamped tiles back to Floor
        let removed = remove_prop_at_on_grid(&mut tiles, &mut props, w, h, 3, 2).unwrap();
        assert_eq!(removed.kind, PropKind::Sofa);
        assert!(props.is_empty());
        assert_eq!(tiles[(2 * w + 2) as usize], TacticalTileKind::Floor);
        assert_eq!(tiles[(2 * w + 4) as usize], TacticalTileKind::Floor);
    }
}
