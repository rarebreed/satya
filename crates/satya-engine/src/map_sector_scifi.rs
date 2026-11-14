//! # Sci-Fi & Clinical Facility Sector Builders (`map_sector_scifi`)
//!
//! Companion module to [`crate::map_sector_gen`] implementing the four enclosed facility
//! architectures (`DockingBay`, `MedLab`, `DerelictCorridor`, `OrbitalAirlock`) with full
//! interior flooring surfaces, loading-apron polygons, elevated catwalks, and hazard spills.

use satya_schema::{EdgeSide, MaterialClass, PropKind, SurfaceFeatureKind, WallEdgeKind};

use crate::map_sector_gen::{Rect, SURF_CATWALK, SURF_HAZARD, SectorBuilder, Side};

pub(crate) fn build_docking_bay(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    b.perimeter(MaterialClass::ConcreteBlock, MaterialClass::SheetSteel);
    b.add_corner_pilasters();

    // Base structural bay floor plus East tarmac loading apron.
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[(0.0, 0.0), (w as f32, 0.0), (w as f32, h as f32), (0.0, h as f32)],
        0,
        501,
    );
    if w >= 16 && h >= 12 {
        b.add_polygon_surface(
            SurfaceFeatureKind::AsphaltRoad,
            &[
                ((w - 6) as f32, 3.0),
                ((w - 1) as f32, 3.0),
                ((w - 1) as f32, (h - 1) as f32),
                ((w - 6) as f32, (h - 1) as f32),
            ],
            14,
            502,
        );
    }

    // Elevated steel catwalk along North gantry + industrial fuel/chemical hazard spill
    let cat_y = if h >= 12 { 2 } else { 1 };
    for x in 3..(w - 3) {
        b.set_surf(x, cat_y, SURF_CATWALK);
    }
    b.set_surf(mid_x - 1, (h - 3).max(cat_y + 2), SURF_HAZARD);
    b.set_surf(mid_x, (h - 3).max(cat_y + 2), SURF_HAZARD);

    // Glazed SW shipping/dispatch booth (InteriorFloor draws over East asphalt apron only inside booth)
    let office = if w >= 14 && h >= 11 {
        let r = Rect { x0: 1, y0: h - 4, x1: 6, y1: h };
        b.enclose_rect(r, MaterialClass::Drywall);
        for x in 2..5 {
            b.put_edge((x, r.y0), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
        }
        b.put_edge((r.x1, h - 2), EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
        Some(r)
    } else {
        None
    };
    b.add_perimeter_windows(4);
    b.reserve_spawns_and_doors();

    let whole = b.whole();
    if let Some(off) = office {
        let _ = b.against_wall(PropKind::Desk, off, Side::West, None);
        let _ = b.against_wall(PropKind::SteelShelving, off, Side::South, None);
    }
    if w >= 16 && h >= 14 {
        let van_rot = 90 + b.rng.range(-3, 4);
        let _ = b.try_place(PropKind::UtilityVan, (w - 6, 4), van_rot, None, whole)
            .or_else(|| b.try_place(PropKind::UtilityVan, (w - 6, 4), 90, None, whole));
    }
    let _ = b.try_place(PropKind::SteelShelving, (mid_x - 1, 4), 0, None, whole)
        .or_else(|| b.try_place(PropKind::SteelShelving, (4, 1), 0, None, whole));
    let _ = b.try_place(PropKind::SteelShelving, (mid_x + 1, h - 2), 0, None, whole);
    let _ = b.try_place(PropKind::JerseyBarrier, (mid_x - 4, mid_y + 1), 90, Some(2), whole)
        .or_else(|| b.try_place(PropKind::JerseyBarrier, (1, 1), 0, Some(2), whole));
    let clear_ring = w >= 12;
    let crate_bounds = Rect {
        x0: if clear_ring { 3 } else { 2 },
        y0: cat_y + 1,
        x1: w - if clear_ring { 3 } else { 2 },
        y1: h - 1,
    };
    b.scatter_in(PropKind::WoodCrate, crate_bounds, 5, clear_ring);
    b.scatter_in(PropKind::DrumCluster, crate_bounds, 3, clear_ring);
}

pub(crate) fn build_med_lab(b: &mut SectorBuilder) {
    let (w, h, mid_y) = (b.w, b.h, b.h / 2);
    b.perimeter(MaterialClass::ConcreteBlock, MaterialClass::SolidWood);
    b.add_corner_pilasters();
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[(0.0, 0.0), (w as f32, 0.0), (w as f32, h as f32), (0.0, h as f32)],
        0,
        511,
    );

    if w >= 12 {
        let x1 = (w / 3).clamp(3, w - 5);
        let x2 = (2 * w / 3).clamp(x1 + 2, w - 3);
        // West drywall partition: intake triage ward -> main surgical bay
        for y in 0..h {
            let (kind, mat) = if y == mid_y || y == 2 {
                (WallEdgeKind::DoorOpen, MaterialClass::Drywall)
            } else if (y - mid_y).abs() == 1 {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, MaterialClass::Drywall)
            };
            b.put_edge((x1, y), EdgeSide::West, kind, mat);
        }
        // East reinforced bio-containment bulkhead
        for y in 0..h {
            let (kind, mat) = if y == mid_y || y == (h - 3).max(1) {
                (WallEdgeKind::DoorClosed, MaterialClass::SheetSteel)
            } else if y == 3 || y == (h - 4).max(4) {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete)
            };
            b.put_edge((x2, y), EdgeSide::West, kind, mat);
        }
        b.set_surf(x2 + 1, (mid_y + 1).min(h - 2), SURF_HAZARD);
        b.set_surf((x2 + 2).min(w - 2), (mid_y + 1).min(h - 2), SURF_HAZARD);
        b.add_perimeter_windows(4);
        b.reserve_spawns_and_doors();

        let triage = Rect { x0: 0, y0: 0, x1, y1: h };
        let trauma = Rect { x0: x1, y0: 0, x1: x2, y1: h };
        let lab = Rect { x0: x2, y0: 0, x1: w, y1: h };

        let _ = b.against_any_wall(PropKind::Desk, triage, None);
        let _ = b.against_any_wall(PropKind::Loveseat, triage, None);
        let _ = b.against_any_wall(PropKind::Planter, triage, None);
        let _ = b.against_any_wall(PropKind::Fridge, trauma, None);
        let _ = b.against_any_wall(PropKind::SteelShelving, trauma, None);
        let _ = b.against_any_wall(PropKind::KitchenCounter, trauma, Some(3));
        b.scatter_in(PropKind::Bed, trauma, 2, true);
        let _ = b.against_any_wall(PropKind::Fridge, lab, None);
        let _ = b.against_any_wall(PropKind::Desk, lab, None);
        b.scatter_in(PropKind::DrumCluster, lab, 2, true);
    } else {
        let sx = w / 2;
        for y in 0..h {
            let kind = if y == mid_y { WallEdgeKind::DoorClosed } else { WallEdgeKind::Wall };
            b.put_edge((sx, y), EdgeSide::West, kind, MaterialClass::ConcreteBlock);
        }
        b.set_surf(sx + 1, (mid_y + 1).min(h - 2), SURF_HAZARD);
        b.add_perimeter_windows(3);
        b.reserve_spawns_and_doors();
        let whole = b.whole();
        let west_bay = Rect { x0: 0, y0: 0, x1: sx, y1: h };
        let _ = b.against_any_wall(PropKind::Desk, west_bay, None);
        let _ = b.try_place(PropKind::Bed, (sx + 1, 1), 0, None, whole);
        b.scatter_in(PropKind::Bed, whole, 1, false);
        b.scatter_in(PropKind::DrumCluster, whole, 2, false);
        let _ = b.against_any_wall(PropKind::Fridge, whole, None);
    }
}

pub(crate) fn build_derelict_corridor(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    b.perimeter(MaterialClass::ReinforcedConcrete, MaterialClass::SheetSteel);
    b.add_corner_pilasters();
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[(0.0, 0.0), (w as f32, 0.0), (w as f32, h as f32), (0.0, h as f32)],
        0,
        521,
    );

    let y_top = (h / 4).clamp(2, 4);
    let y_bot = (h - y_top).max(y_top + 3);
    if w >= 12 && h >= 10 {
        for x in 0..w {
            let top_door = x == 2 || x == mid_x || x == w - 3;
            let bot_door = x == 3 || x == mid_x - 1 || x == w - 4;
            let top_kind = if top_door { WallEdgeKind::DoorOpen } else if x % 4 == 1 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
            let bot_kind = if bot_door { WallEdgeKind::DoorClosed } else if x % 4 == 2 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
            b.put_edge((x, y_top), EdgeSide::North, top_kind, MaterialClass::ConcreteBlock);
            b.put_edge((x, y_bot), EdgeSide::North, bot_kind, MaterialClass::ConcreteBlock);
        }
        for &div_x in &[w / 3, (2 * w) / 3] {
            for y in 0..y_top {
                b.put_edge((div_x, y), EdgeSide::West, WallEdgeKind::Wall, MaterialClass::SheetSteel);
            }
            for y in y_bot..h {
                b.put_edge((div_x, y), EdgeSide::West, WallEdgeKind::Wall, MaterialClass::SheetSteel);
            }
            for y in y_top..y_bot {
                let kind = if y == mid_y {
                    WallEdgeKind::DoorOpen
                } else if y == (mid_y - 1).max(y_top) {
                    WallEdgeKind::DoorClosed
                } else {
                    WallEdgeKind::Wall
                };
                b.put_edge((div_x, y), EdgeSide::West, kind, MaterialClass::ReinforcedConcrete);
            }
        }
    } else {
        for y in 0..h {
            let kind = if y == mid_y { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall };
            b.put_edge((mid_x, y), EdgeSide::West, kind, MaterialClass::ReinforcedConcrete);
        }
    }

    b.set_surf((mid_x - 1).max(1), (mid_y + 1).min(h - 2), SURF_HAZARD);
    b.set_surf(mid_x, (mid_y + 1).min(h - 2), SURF_HAZARD);
    b.reserve_spawns_and_doors();

    let whole = b.whole();
    let spine = Rect { x0: 1, y0: y_top.min(h - 2), x1: w - 1, y1: y_bot.min(h - 1).max(y_top + 1) };
    let _ = b.against_any_wall(PropKind::SteelShelving, Rect { x0: 1, y0: 0, x1: w - 1, y1: y_top }, None)
        .or_else(|| b.against_any_wall(PropKind::SteelShelving, whole, None));
    let _ = b.against_any_wall(PropKind::Desk, Rect { x0: 1, y0: y_bot, x1: w - 1, y1: h }, None)
        .or_else(|| b.against_any_wall(PropKind::Desk, whole, None));
    let clear_ring = w >= 14 && h >= 12;
    b.scatter_in(PropKind::Boulder, spine, 2, clear_ring);
    b.scatter_in(PropKind::DrumCluster, whole, 4, clear_ring);
    b.scatter_in(PropKind::WoodCrate, whole, 4, clear_ring);
}

pub(crate) fn build_orbital_airlock(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    b.perimeter(MaterialClass::ArmorSteel, MaterialClass::ArmorSteel);
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[(0.0, 0.0), (w as f32, 0.0), (w as f32, h as f32), (0.0, h as f32)],
        0,
        531,
    );

    if w >= 14 && h >= 10 {
        for &(px, py) in &[(mid_x - 4, 3), (mid_x + 3, 3), (mid_x - 4, h - 4), (mid_x + 3, h - 4)] {
            b.add_pillar(px, py);
        }
    } else {
        b.add_corner_pilasters();
    }

    b.set_surf(mid_x + 1, (mid_y - 2).max(1), SURF_HAZARD);
    b.set_surf((mid_x + 2).min(w - 2), (mid_y - 2).max(1), SURF_HAZARD);

    let mut booths = Vec::new();
    if w >= 14 && h >= 12 {
        let bx0 = (mid_x - 3).max(4);
        let bx1 = (mid_x + 3).min(w - 4);
        let north_booth = Rect { x0: bx0, y0: 0, x1: bx1, y1: 3 };
        let south_booth = Rect { x0: bx0, y0: h - 3, x1: bx1, y1: h };
        b.enclose_rect(north_booth, MaterialClass::SheetSteel);
        b.enclose_rect(south_booth, MaterialClass::SheetSteel);
        for x in (bx0 + 1)..(bx1 - 1) {
            b.put_edge((x, 3), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
            b.put_edge((x, h - 3), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
        }
        b.put_edge((bx0, 1), EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        b.put_edge((bx1, h - 2), EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SheetSteel);
        booths.push(north_booth);
        booths.push(south_booth);
        for x in (bx0 + 1)..(bx1 - 1) {
            b.set_surf(x, 3, SURF_CATWALK);
        }
    }

    if w >= 16 && h >= 10 {
        for &vx in &[4, w - 4] {
            for y in 2..(h - 2) {
                let (kind, mat) = if y == mid_y {
                    (WallEdgeKind::DoorOpen, MaterialClass::ArmorSteel)
                } else if y == mid_y - 1 {
                    (WallEdgeKind::DoorClosed, MaterialClass::ArmorSteel)
                } else if (y - mid_y).abs() == 2 {
                    (WallEdgeKind::Window, MaterialClass::Glass)
                } else {
                    (WallEdgeKind::Wall, MaterialClass::ArmorSteel)
                };
                b.put_edge((vx, y), EdgeSide::West, kind, mat);
            }
        }
    }

    b.reserve_spawns_and_doors();
    let whole = b.whole();
    for booth in booths {
        let _ = b.against_any_wall(PropKind::Desk, booth, None);
        let _ = b.against_any_wall(PropKind::Wardrobe, booth, None);
    }
    let _ = b.try_place(PropKind::JerseyBarrier, (mid_x - 1, (mid_y - 1).max(3)), 90, Some(2), whole)
        .or_else(|| b.try_place(PropKind::JerseyBarrier, (mid_x - 1, 1), 0, Some(2), whole));
    let _ = b.try_place(PropKind::SteelShelving, (mid_x, h - 4), 0, None, whole)
        .or_else(|| b.against_any_wall(PropKind::SteelShelving, whole, None));
    let clear_ring = w >= 14 && h >= 10;
    let cargo_bounds = Rect {
        x0: if clear_ring { 4 } else { 1 },
        y0: if clear_ring { 3 } else { 1 },
        x1: (w - if clear_ring { 4 } else { 1 }).max(5),
        y1: (h - if clear_ring { 3 } else { 1 }).max(4),
    };
    b.scatter_in(PropKind::WoodCrate, cargo_bounds, 4, clear_ring);
    b.scatter_in(PropKind::DrumCluster, cargo_bounds, 2, clear_ring);
}
