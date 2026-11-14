//! # Sci-Fi & Clinical Facility Sector Builders (`map_sector_scifi`)
//!
//! Companion module to [`crate::map_sector_gen`] implementing the four enclosed facility
//! architectures (`DockingBay`, `MedLab`, `DerelictCorridor`, `OrbitalAirlock`) with compact
//! `<= 20×14` layouts plus scaled `>= 24×18` multi-bay facility topologies.

use satya_schema::{EdgeSide, MaterialClass, PropKind, SurfaceFeatureKind, WallEdgeKind};

use crate::map_sector_gen::{Rect, SURF_CATWALK, SURF_HAZARD, SectorBuilder, Side};

fn add_rect_floor(b: &mut SectorBuilder, x0: f32, y0: f32, x1: f32, y1: f32, seed: u32) {
    if x1 > x0 && y1 > y0 {
        b.add_polygon_surface(
            SurfaceFeatureKind::InteriorFloor,
            &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)],
            0,
            seed,
        );
    }
}

fn add_full_floor(b: &mut SectorBuilder, seed: u32) {
    let (w, h) = (b.w, b.h);
    add_rect_floor(b, 0.0, 0.0, w as f32, h as f32, seed);
}

pub(crate) fn build_docking_bay(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    b.perimeter(MaterialClass::ConcreteBlock, MaterialClass::SheetSteel);
    b.add_corner_pilasters();

    if w >= 24 && h >= 18 {
        build_docking_bay_scaled(b);
        return;
    }

    if w >= 16 && h >= 12 {
        let (ax0, ay0, ay1) = ((w - 6) as f32, 3.0, (h - 1) as f32);
        add_rect_floor(b, 0.0, 0.0, ax0, h as f32, 501);
        add_rect_floor(b, ax0, 0.0, w as f32, ay0, 504);
        add_rect_floor(b, ax0, ay1, w as f32, h as f32, 505);
        b.add_polygon_surface(
            SurfaceFeatureKind::AsphaltRoad,
            &[(ax0 - 0.15, ay0 - 0.15), (w as f32, ay0 - 0.15), (w as f32, ay1 + 0.15), (ax0 - 0.15, ay1 + 0.15)],
            14,
            502,
        );
    } else {
        add_full_floor(b, 501);
    }
    let cat_y = if h >= 12 { 2 } else { 1 };
    for x in 3..(w - 3) {
        b.set_surf(x, cat_y, SURF_CATWALK);
    }
    b.set_surf(mid_x - 1, (h - 3).max(cat_y + 2), SURF_HAZARD);
    b.set_surf(mid_x, (h - 3).max(cat_y + 2), SURF_HAZARD);

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
    let crate_bounds = Rect { x0: if clear_ring { 3 } else { 2 }, y0: cat_y + 1, x1: w - if clear_ring { 3 } else { 2 }, y1: h - 1 };
    b.scatter_in(PropKind::WoodCrate, crate_bounds, 5, clear_ring);
    b.scatter_in(PropKind::DrumCluster, crate_bounds, 3, clear_ring);
}

fn build_docking_bay_scaled(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    let flip = b.rng.chance(1, 2);
    let bay_x0 = (w - (w / 3).clamp(7, 16)) as f32;
    let bay_y1 = (h - 2) as f32;
    add_rect_floor(b, bay_x0, 0.0, w as f32, 3.0, 504);
    add_rect_floor(b, bay_x0, bay_y1, w as f32, h as f32, 505);
    b.add_polygon_surface(
        SurfaceFeatureKind::AsphaltRoad,
        &[(bay_x0 - 0.15, 2.85), (w as f32, 2.85), (w as f32, bay_y1 + 0.15), (bay_x0 - 0.15, bay_y1 + 0.15)],
        18,
        502,
    );
    if w >= 34 {
        let (sy0, sy1, sx1) = ((h - 6) as f32, (h - 1) as f32, bay_x0 - 2.0);
        add_rect_floor(b, 0.0, 0.0, bay_x0, sy0, 501);
        add_rect_floor(b, 0.0, sy0, 2.0, h as f32, 506);
        add_rect_floor(b, sx1, sy0, bay_x0, h as f32, 507);
        add_rect_floor(b, 2.0, sy1, sx1, h as f32, 508);
        b.add_polygon_surface(
            SurfaceFeatureKind::AsphaltRoad,
            &[(1.85, sy0 - 0.15), (sx1 + 0.15, sy0 - 0.15), (sx1 + 0.15, sy1 + 0.15), (1.85, sy1 + 0.15)],
            18,
            503,
        );
    } else {
        add_rect_floor(b, 0.0, 0.0, bay_x0, h as f32, 501);
    }
    // U-shaped or cross elevated gantry catwalk.
    let top_y = 2;
    let bot_y = h - 3;
    for x in 3..(w - 3) {
        b.set_surf(x, top_y, SURF_CATWALK);
        if flip { b.set_surf(x, bot_y, SURF_CATWALK); }
    }
    for y in top_y..=bot_y {
        b.set_surf(mid_x - 1, y, SURF_CATWALK);
    }
    for dx in 0..3 {
        b.set_surf(mid_x + dx, (mid_y + 2).min(h - 3), SURF_HAZARD);
    }

    let dispatch = Rect { x0: 1, y0: if flip { 1 } else { h - 6 }, x1: 8, y1: if flip { 6 } else { h - 1 } };
    let cage = Rect { x0: 1, y0: if flip { h - 6 } else { 1 }, x1: 9, y1: if flip { h - 1 } else { 6 } };
    for (idx, r) in [dispatch, cage].into_iter().enumerate() {
        b.enclose_rect(r, if idx == 0 { MaterialClass::Drywall } else { MaterialClass::SheetSteel });
        let win_y = if r.y0 <= 2 { r.y1 } else { r.y0 };
        for x in (r.x0 + 1)..(r.x1 - 1) {
            b.put_edge((x, win_y), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
        }
        b.put_edge((r.x1, r.y0 + r.h() / 2), EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
    }
    b.add_perimeter_windows(4);
    b.reserve_spawns_and_doors();

    let whole = b.whole();
    let _ = b.against_wall(PropKind::Desk, dispatch, Side::West, None);
    let _ = b.against_wall(PropKind::SteelShelving, cage, Side::West, None);
    let area = w * h;
    let van_y0 = 4;
    let van_y1 = (h - 6).max(van_y0 + 3);
    let r0 = 90 + b.rng.range(-4, 5);
    let r1 = 90 + b.rng.range(-4, 5);
    let _ = b.try_place(PropKind::UtilityVan, (w - 6, van_y0), r0, None, whole);
    let _ = b.try_place(PropKind::UtilityVan, (w - 6, van_y1), r1, None, whole);
    b.scatter_in(PropKind::SteelShelving, Rect { x0: 3, y0: 4, x1: mid_x - 2, y1: h - 4 }, (area / 180).clamp(3, 8) as usize, true);
    b.scatter_in(PropKind::JerseyBarrier, Rect { x0: mid_x - 4, y0: 4, x1: w - 4, y1: h - 4 }, (area / 240).clamp(2, 5) as usize, true);
    b.scatter_in(PropKind::WoodCrate, Rect { x0: 3, y0: 3, x1: w - 3, y1: h - 2 }, (area / 95).clamp(6, 18) as usize, true);
    b.scatter_in(PropKind::DrumCluster, Rect { x0: 3, y0: 3, x1: w - 3, y1: h - 2 }, (area / 140).clamp(4, 12) as usize, true);
}

pub(crate) fn build_med_lab(b: &mut SectorBuilder) {
    let (w, h, mid_y) = (b.w, b.h, b.h / 2);
    b.perimeter(MaterialClass::ConcreteBlock, MaterialClass::SolidWood);
    b.add_corner_pilasters();
    add_full_floor(b, 511);

    if w >= 24 && h >= 18 {
        build_med_lab_scaled(b);
        return;
    }

    if w >= 12 {
        let x1 = (w / 3).clamp(3, w - 5);
        let x2 = (2 * w / 3).clamp(x1 + 2, w - 3);
        for y in 0..h {
            let (k1, m1) = if y == mid_y || y == 2 { (WallEdgeKind::DoorOpen, MaterialClass::Drywall) } else if (y - mid_y).abs() == 1 { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, MaterialClass::Drywall) };
            b.put_edge((x1, y), EdgeSide::West, k1, m1);
            let (k2, m2) = if y == mid_y || y == (h - 3).max(1) { (WallEdgeKind::DoorClosed, MaterialClass::SheetSteel) } else if y == 3 || y == (h - 4).max(4) { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete) };
            b.put_edge((x2, y), EdgeSide::West, k2, m2);
        }
        b.set_surf(x2 + 1, (mid_y + 1).min(h - 2), SURF_HAZARD);
        b.set_surf((x2 + 2).min(w - 2), (mid_y + 1).min(h - 2), SURF_HAZARD);
        b.add_perimeter_windows(4);
        b.reserve_spawns_and_doors();
        let (triage, trauma, lab) = (Rect { x0: 0, y0: 0, x1, y1: h }, Rect { x0: x1, y0: 0, x1: x2, y1: h }, Rect { x0: x2, y0: 0, x1: w, y1: h });
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
            b.put_edge((sx, y), EdgeSide::West, if y == mid_y { WallEdgeKind::DoorClosed } else { WallEdgeKind::Wall }, MaterialClass::ConcreteBlock);
        }
        b.set_surf(sx + 1, (mid_y + 1).min(h - 2), SURF_HAZARD);
        b.add_perimeter_windows(3);
        b.reserve_spawns_and_doors();
        let whole = b.whole();
        let _ = b.against_any_wall(PropKind::Desk, Rect { x0: 0, y0: 0, x1: sx, y1: h }, None);
        let _ = b.try_place(PropKind::Bed, (sx + 1, 1), 0, None, whole);
        b.scatter_in(PropKind::Bed, whole, 1, false);
        b.scatter_in(PropKind::DrumCluster, whole, 2, false);
        let _ = b.against_any_wall(PropKind::Fridge, whole, None);
    }
}

fn build_med_lab_scaled(b: &mut SectorBuilder) {
    let (w, h, mid_y) = (b.w, b.h, b.h / 2);
    let x1 = (w * 3 / 10).clamp(6, w - 12);
    let x2 = (w * 68 / 100).clamp(x1 + 6, w - 6);
    for y in 0..h {
        let (k1, m1) = if y == mid_y || y == 3 || y == h - 4 { (WallEdgeKind::DoorOpen, MaterialClass::Drywall) } else if y % 3 == 1 { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, MaterialClass::Drywall) };
        b.put_edge((x1, y), EdgeSide::West, k1, m1);
        let (k2, m2) = if y == mid_y || y == h / 4 || y == (3 * h) / 4 { (WallEdgeKind::DoorClosed, MaterialClass::SheetSteel) } else if y % 4 == 2 { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete) };
        b.put_edge((x2, y), EdgeSide::West, k2, m2);
    }
    // Horizontal triage exam cubicles and twin containment labs.
    let split_y = mid_y;
    for x in x2..w {
        let k = if x == (x2 + w) / 2 { WallEdgeKind::DoorOpen } else { WallEdgeKind::Window };
        b.put_edge((x, split_y), EdgeSide::North, k, MaterialClass::Glass);
    }
    let ward_top = h / 3;
    let ward_bot = (2 * h) / 3;
    for x in x1..x2 {
        let kt = if x == x1 + 2 || x == x2 - 3 { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall };
        b.put_edge((x, ward_top), EdgeSide::North, kt, MaterialClass::Drywall);
        b.put_edge((x, ward_bot), EdgeSide::North, kt, MaterialClass::Drywall);
    }
    b.set_surf(x2 + 2, (ward_top - 1).max(2), SURF_HAZARD);
    b.set_surf((x2 + 3).min(w - 2), (ward_bot + 1).min(h - 2), SURF_HAZARD);
    b.add_perimeter_windows(4);
    b.reserve_spawns_and_doors();

    let intake = Rect { x0: 0, y0: 0, x1, y1: h };
    let _ = b.against_any_wall(PropKind::DeskL, intake, None);
    b.scatter_in(PropKind::Loveseat, intake, 2, true);
    b.scatter_in(PropKind::Planter, intake, 2, true);
    for r in [
        Rect { x0: x1, y0: 0, x1: x2, y1: ward_top },
        Rect { x0: x1, y0: ward_top, x1: x2, y1: ward_bot },
        Rect { x0: x1, y0: ward_bot, x1: x2, y1: h },
    ] {
        b.scatter_in(PropKind::Bed, r, 2, true);
        let _ = b.against_any_wall(PropKind::KitchenCounter, r, Some(3));
        let _ = b.against_any_wall(PropKind::Fridge, r, None);
    }
    for lab in [Rect { x0: x2, y0: 0, x1: w, y1: split_y }, Rect { x0: x2, y0: split_y, x1: w, y1: h }] {
        let _ = b.against_any_wall(PropKind::SteelShelving, lab, None);
        let _ = b.against_any_wall(PropKind::Desk, lab, None);
        b.scatter_in(PropKind::DrumCluster, lab, 2, true);
    }
}

pub(crate) fn build_derelict_corridor(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    b.perimeter(MaterialClass::ReinforcedConcrete, MaterialClass::SheetSteel);
    b.add_corner_pilasters();
    add_full_floor(b, 521);

    let y_top = (h / 4).clamp(2, if h >= 18 { 6 } else { 4 });
    let y_bot = (h - y_top).max(y_top + 3);
    if w >= 12 && h >= 10 {
        for x in 0..w {
            let top_door = x == 2 || x == mid_x || x == w - 3 || (w >= 24 && (x == w / 4 || x == (3 * w) / 4));
            let bot_door = x == 3 || x == mid_x - 1 || x == w - 4 || (w >= 24 && (x == w / 4 + 1 || x == (3 * w) / 4 - 1));
            let top_kind = if top_door { WallEdgeKind::DoorOpen } else if x % 4 == 1 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
            let bot_kind = if bot_door { WallEdgeKind::DoorClosed } else if x % 4 == 2 { WallEdgeKind::Window } else { WallEdgeKind::Wall };
            b.put_edge((x, y_top), EdgeSide::North, top_kind, MaterialClass::ConcreteBlock);
            b.put_edge((x, y_bot), EdgeSide::North, bot_kind, MaterialClass::ConcreteBlock);
        }
        let divs: Vec<i32> = if w >= 28 { vec![w / 4, w / 2, (3 * w) / 4] } else { vec![w / 3, (2 * w) / 3] };
        for div_x in divs {
            for y in 0..y_top {
                b.put_edge((div_x, y), EdgeSide::West, WallEdgeKind::Wall, MaterialClass::SheetSteel);
            }
            for y in y_bot..h {
                b.put_edge((div_x, y), EdgeSide::West, WallEdgeKind::Wall, MaterialClass::SheetSteel);
            }
            for y in y_top..y_bot {
                let kind = if y == mid_y { WallEdgeKind::DoorOpen } else if y == (mid_y - 1).max(y_top) { WallEdgeKind::DoorClosed } else { WallEdgeKind::Wall };
                b.put_edge((div_x, y), EdgeSide::West, kind, MaterialClass::ReinforcedConcrete);
            }
        }
    } else {
        for y in 0..h {
            b.put_edge((mid_x, y), EdgeSide::West, if y == mid_y { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall }, MaterialClass::ReinforcedConcrete);
        }
    }

    b.set_surf((mid_x - 1).max(1), (mid_y + 1).min(h - 2), SURF_HAZARD);
    b.set_surf(mid_x, (mid_y + 1).min(h - 2), SURF_HAZARD);
    if w >= 24 && h >= 18 {
        b.set_surf((w / 4).max(2), (mid_y - 1).max(y_top), SURF_HAZARD);
    }
    b.reserve_spawns_and_doors();

    let whole = b.whole();
    let spine = Rect { x0: 1, y0: y_top.min(h - 2), x1: w - 1, y1: y_bot.min(h - 1).max(y_top + 1) };
    let _ = b.against_any_wall(PropKind::SteelShelving, Rect { x0: 1, y0: 0, x1: w - 1, y1: y_top }, None)
        .or_else(|| b.against_any_wall(PropKind::SteelShelving, whole, None));
    let _ = b.against_any_wall(PropKind::Desk, Rect { x0: 1, y0: y_bot, x1: w - 1, y1: h }, None)
        .or_else(|| b.against_any_wall(PropKind::Desk, whole, None));
    let clear_ring = w >= 14 && h >= 12;
    let scale = (w * h / 120).clamp(1, 5) as usize;
    b.scatter_in(PropKind::Boulder, spine, 2 * scale, clear_ring);
    b.scatter_in(PropKind::DrumCluster, whole, 4 * scale, clear_ring);
    b.scatter_in(PropKind::WoodCrate, whole, 4 * scale, clear_ring);
}

pub(crate) fn build_orbital_airlock(b: &mut SectorBuilder) {
    let (w, h, mid_x, mid_y) = (b.w, b.h, b.w / 2, b.h / 2);
    b.perimeter(MaterialClass::ArmorSteel, MaterialClass::ArmorSteel);
    add_full_floor(b, 531);

    if w >= 24 && h >= 18 {
        for &px in &[w / 4, mid_x - 3, mid_x + 3, (3 * w) / 4] {
            for &py in &[4, h - 5] {
                b.add_pillar(px, py);
            }
        }
    } else if w >= 14 && h >= 10 {
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
        let bx0 = (mid_x - (w / 6).clamp(3, 7)).max(4);
        let bx1 = (mid_x + (w / 6).clamp(3, 7)).min(w - 4);
        let bd = if h >= 18 { 4 } else { 3 };
        let north_booth = Rect { x0: bx0, y0: 0, x1: bx1, y1: bd };
        let south_booth = Rect { x0: bx0, y0: h - bd, x1: bx1, y1: h };
        b.enclose_rect(north_booth, MaterialClass::SheetSteel);
        b.enclose_rect(south_booth, MaterialClass::SheetSteel);
        for x in (bx0 + 1)..(bx1 - 1) {
            b.put_edge((x, bd), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
            b.put_edge((x, h - bd), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
        }
        b.put_edge((bx0, 1), EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        b.put_edge((bx1, h - 2), EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SheetSteel);
        booths.push(north_booth);
        booths.push(south_booth);
        for x in (bx0 + 1)..(bx1 - 1) {
            b.set_surf(x, bd, SURF_CATWALK);
        }
    }

    if w >= 16 && h >= 10 {
        let vest_xs: Vec<i32> = if w >= 28 { vec![4, w / 3, (2 * w) / 3, w - 4] } else { vec![4, w - 4] };
        for vx in vest_xs {
            for y in 2..(h - 2) {
                let (kind, mat) = if y == mid_y { (WallEdgeKind::DoorOpen, MaterialClass::ArmorSteel) } else if y == mid_y - 1 { (WallEdgeKind::DoorClosed, MaterialClass::ArmorSteel) } else if (y - mid_y).abs() == 2 { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, MaterialClass::ArmorSteel) };
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
    let cargo_bounds = Rect { x0: if clear_ring { 4 } else { 1 }, y0: if clear_ring { 3 } else { 1 }, x1: (w - if clear_ring { 4 } else { 1 }).max(5), y1: (h - if clear_ring { 3 } else { 1 }).max(4) };
    let scale = (w * h / 120).clamp(1, 5) as usize;
    b.scatter_in(PropKind::WoodCrate, cargo_bounds, 4 * scale, clear_ring);
    b.scatter_in(PropKind::DrumCluster, cargo_bounds, 2 * scale, clear_ring);
}
