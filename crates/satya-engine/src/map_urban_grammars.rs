//! Multi-topology procedural site grammars for `UrbanCrossroads`.
//!
//! Replaces the single fixed 4-corner template with four seeded urban street topologies across
//! standard through massive (`64×48` / `96×72`) sectors while keeping compact (`< 20×14`) maps balanced:
//! - [`UrbanTopology::AsymmetricCrossroads`]: Off-center boulevard intersection with four varied corner blocks.
//! - [`UrbanTopology::TJunctionCommercialStrip`]: Main boulevard feeding a continuous retail strip & stem avenue.
//! - [`UrbanTopology::CurvedAvenueAndCornerPlaza`]: Smooth S-curved arterial road, civic stone plaza & cafe tables.
//! - [`UrbanTopology::StaggeredOffsetCrossroads`]: Offset N/S side streets off a main avenue creating dog-leg lanes.

use crate::map_sector_gen::{Rect, SectorBuilder, SurfaceAllowance};
use satya_schema::{EdgeSide, MaterialClass, PropKind, SurfaceFeatureKind, WallEdgeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UrbanTopology {
    AsymmetricCrossroads,
    TJunctionCommercialStrip,
    CurvedAvenueAndCornerPlaza,
    StaggeredOffsetCrossroads,
}

fn poly_pts(r: Rect) -> [(f32, f32); 4] {
    [(r.x0 as f32, r.y0 as f32), (r.x1 as f32, r.y0 as f32), (r.x1 as f32, r.y1 as f32), (r.x0 as f32, r.y1 as f32)]
}

fn rw(r: Rect) -> i32 { r.x1 - r.x0 }
fn rh(r: Rect) -> i32 { r.y1 - r.y0 }

fn enclose_storefront(
    b: &mut SectorBuilder,
    r: Rect,
    idx: usize,
    door_east_or_west: bool,
    open_door: bool,
) {
    let seed = (b.rng.next() & 0xffff) as u32 + idx as u32;
    b.add_polygon_surface(SurfaceFeatureKind::InteriorFloor, &poly_pts(r), 0, seed);
    let mat = if idx.is_multiple_of(2) { MaterialClass::Brick } else { MaterialClass::ConcreteBlock };
    b.enclose_rect(r, mat);
    let dk = if open_door { WallEdgeKind::DoorOpen } else { WallEdgeKind::DoorClosed };
    let door_y = r.y0 + rh(r) / 2;
    let door_x = if r.x0 == 0 { r.x1 } else if r.x1 == b.w { r.x0 } else if door_east_or_west { r.x1 } else { r.x0 };
    b.put_edge((door_x, door_y), EdgeSide::West, dk, MaterialClass::SolidWood);
    let horiz_y = if r.y0 == 0 { r.y1 } else { r.y0 };
    for x in (r.x0 + 1)..(r.x1 - 1) {
        if (x - r.x0) % 2 == 1 {
            b.put_edge((x, horiz_y), EdgeSide::North, WallEdgeKind::Window, MaterialClass::Glass);
        }
    }
    if rh(r) >= 4 {
        let mid_x = r.x0 + rw(r) / 2;
        b.put_edge((mid_x, horiz_y), EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    }
}

fn furnish_storefront(b: &mut SectorBuilder, r: Rect, role_idx: usize) {
    if rw(r) < 3 || rh(r) < 3 {
        return;
    }
    match role_idx % 4 {
        0 => {
            let _ = b.against_any_wall(PropKind::KitchenCounter, r, Some((rw(r) / 2).clamp(2, 4)));
            b.scatter_in(PropKind::RoundTable, r, (rw(r) * rh(r) / 10).clamp(2, 5) as usize, false);
        }
        1 => {
            let _ = b.against_any_wall(PropKind::SteelShelving, r, None);
            let _ = b.against_any_wall(PropKind::Fridge, r, None);
            b.scatter_in(PropKind::WoodCrate, r, (rw(r) * rh(r) / 14).clamp(1, 4) as usize, false);
        }
        2 => {
            let _ = b.against_any_wall(PropKind::Desk, r, None);
            let _ = b.against_any_wall(PropKind::Bookshelf, r, None);
            let _ = b.against_any_wall(PropKind::Sofa, r, None);
        }
        _ => {
            let _ = b.against_any_wall(PropKind::SteelShelving, r, None);
            let _ = b.against_any_wall(PropKind::DrumCluster, r, None);
            b.scatter_in(PropKind::WoodCrate, r, 2, false);
        }
    }
}

/// Generates one of four urban street layouts (`>= 20×14`), or a compact 4-corner layout when `< 20×14`.
pub(crate) fn build_urban_crossroads(b: &mut SectorBuilder) {
    if b.w < 20 || b.h < 14 {
        build_crossroads_layout(b, false);
        return;
    }
    let topo = match b.rng.range(0, 4) {
        0 => UrbanTopology::AsymmetricCrossroads,
        1 => UrbanTopology::TJunctionCommercialStrip,
        2 => UrbanTopology::CurvedAvenueAndCornerPlaza,
        _ => UrbanTopology::StaggeredOffsetCrossroads,
    };
    match topo {
        UrbanTopology::AsymmetricCrossroads => build_crossroads_layout(b, true),
        UrbanTopology::TJunctionCommercialStrip => build_t_junction_strip(b),
        UrbanTopology::CurvedAvenueAndCornerPlaza => build_curved_avenue_plaza(b),
        UrbanTopology::StaggeredOffsetCrossroads => build_staggered_crossroads(b),
    }
}

fn add_sidewalk_apron(b: &mut SectorBuilder, r: Rect, seed: u32) {
    if rw(r) <= 0 || rh(r) <= 0 {
        return;
    }
    b.add_polygon_surface(SurfaceFeatureKind::ConcreteWalk, &poly_pts(r), 28, seed);
}

fn build_crossroads_layout(b: &mut SectorBuilder, asymmetric: bool) {
    let (w, h) = (b.w, b.h);
    let dx = if asymmetric { b.rng.range(-(w / 9).max(1), (w / 9).max(2)) } else { 0 };
    let dy = if asymmetric { b.rng.range(-(h / 9).max(1), (h / 9).max(2)) } else { 0 };

    let bw_w = (w / 4 + dx).clamp(2, (w / 2 - 2).min(14));
    let bw_e = (w / 4 - dx).clamp(2, (w / 2 - 2).min(14));
    let bh_n = (h / 4 + dy).clamp(2, (h / 2 - 2).min(11));
    let bh_s = (h / 4 - dy).clamp(2, (h / 2 - 2).min(11));

    let nw = Rect { x0: 0, y0: 0, x1: bw_w, y1: bh_n };
    let ne = Rect { x0: w - bw_e, y0: 0, x1: w, y1: bh_n };
    let sw = Rect { x0: 0, y0: h - bh_s, x1: bw_w, y1: h };
    let se = Rect { x0: w - bw_e, y0: h - bh_s, x1: w, y1: h };

    let ew_road_y = (bh_n as f32 + (h - bh_s) as f32) * 0.5;
    let ew_w_cm = ((h - bh_n - bh_s).max(4) * 100 - 240).clamp(180, 720);
    let ns_road_x = (bw_w as f32 + (w - bw_e) as f32) * 0.5;
    let ns_w_cm = ((w - bw_w - bw_e).max(4) * 100 - 240).clamp(180, 780);

    let s1 = b.rng.next() as u32;
    let s2 = b.rng.next() as u32;
    b.add_ribbon_spline(SurfaceFeatureKind::AsphaltRoad, &[(0.0, ew_road_y), (w as f32 * 0.5, ew_road_y), (w as f32, ew_road_y)], ew_w_cm, s1);
    b.add_ribbon_spline(SurfaceFeatureKind::AsphaltRoad, &[(ns_road_x, 0.0), (ns_road_x, h as f32 * 0.5), (ns_road_x, h as f32)], ns_w_cm, s2);

    let (rx_lo, rx_hi) = (ns_road_x.floor() as i32, ns_road_x.ceil() as i32);
    let (ry_lo, ry_hi) = (ew_road_y.floor() as i32, ew_road_y.ceil() as i32);
    for (idx, r) in [
        Rect { x0: 0, y0: 0, x1: rx_hi, y1: ry_hi },
        Rect { x0: rx_lo, y0: 0, x1: w, y1: ry_hi },
        Rect { x0: 0, y0: ry_lo, x1: rx_hi, y1: h },
        Rect { x0: rx_lo, y0: ry_lo, x1: w, y1: h },
    ]
    .into_iter()
    .enumerate()
    {
        add_sidewalk_apron(b, r, 20 + idx as u32);
    }

    for (i, r) in [nw, ne, sw, se].into_iter().enumerate() {
        enclose_storefront(b, r, i, i % 2 == 0, i == 0 || i == 3);
    }
    b.reserve_spawns_and_doors();
    for (i, r) in [nw, ne, sw, se].into_iter().enumerate() {
        furnish_storefront(b, r, i);
    }
    dress_urban_street_props(b, bw_w.min(bw_e), bh_n.min(bh_s), ns_road_x as i32, ew_road_y as i32);
}

fn build_t_junction_strip(b: &mut SectorBuilder) {
    let (w, h) = (b.w, b.h);
    let flip = b.rng.chance(1, 2);
    let strip_d = if h < 18 { 3 } else { (h / 4).clamp(4, 10) };
    let corner_d = (h / 4).clamp(3, 9);
    let (strip_y0, strip_y1, corner_y0, corner_y1) = if flip {
        (0, strip_d, (h - corner_d).max(strip_d + 5), h)
    } else {
        (h - strip_d, h, 0, corner_d.min(h - strip_d - 5))
    };
    let blvd_y = if flip {
        (strip_y1 as f32 + corner_y0 as f32) * 0.5
    } else {
        (corner_y1 as f32 + strip_y0 as f32) * 0.5
    };
    let blvd_w_cm = if h < 18 { 380 } else { 520 };
    let stem_x = (w / 2 + b.rng.range(-(w / 8), (w / 8) + 1)).clamp(6, w - 6) as f32;

    let s1 = b.rng.next() as u32;
    let s2 = b.rng.next() as u32;
    b.add_ribbon_spline(SurfaceFeatureKind::AsphaltRoad, &[(0.0, blvd_y), (w as f32 * 0.5, blvd_y), (w as f32, blvd_y)], blvd_w_cm, s1);
    let stem_pts = if flip {
        [(stem_x, blvd_y), (stem_x, (blvd_y + h as f32) * 0.5), (stem_x, h as f32)]
    } else {
        [(stem_x, 0.0), (stem_x, blvd_y * 0.5), (stem_x, blvd_y)]
    };
    b.add_ribbon_spline(SurfaceFeatureKind::AsphaltRoad, &stem_pts, 420, s2);

    let (ap_y0, ap_y1) = if flip { (0, (strip_y1 + 2).min(h)) } else { ((strip_y0 - 2).max(0), h) };
    add_sidewalk_apron(b, Rect { x0: 0, y0: ap_y0, x1: w, y1: ap_y1 }, 30);

    let units = (w / 7).clamp(3, 5);
    let mut buildings = Vec::new();
    let unit_w = w / units;
    for i in 0..units {
        let x0 = i * unit_w;
        let x1 = if i == units - 1 { w } else { (i + 1) * unit_w };
        buildings.push(Rect { x0, y0: strip_y0, x1: (x1 - 1).max(0), y1: strip_y1 });
    }
    let sx = stem_x as i32;
    let c_ap_y0 = if flip { (corner_y0 - 2).max(0) } else { corner_y0 };
    let c_ap_y1 = if flip { h } else { (corner_y1 + 2).min(h) };
    if sx - 4 >= 3 && corner_y1 - corner_y0 >= 3 {
        add_sidewalk_apron(b, Rect { x0: 0, y0: c_ap_y0, x1: (sx - 1).min(w), y1: c_ap_y1 }, 31);
        buildings.push(Rect { x0: 0, y0: corner_y0, x1: sx - 3, y1: corner_y1 });
    }
    if w - (sx + 3) >= 3 && corner_y1 - corner_y0 >= 3 {
        add_sidewalk_apron(b, Rect { x0: (sx + 1).max(0), y0: c_ap_y0, x1: w, y1: c_ap_y1 }, 32);
        buildings.push(Rect { x0: sx + 3, y0: corner_y0, x1: w, y1: corner_y1 });
    }
    for (i, &r) in buildings.iter().enumerate() {
        enclose_storefront(b, r, i, true, i % 2 == 0);
    }
    b.reserve_spawns_and_doors();
    for (i, &r) in buildings.iter().enumerate() {
        furnish_storefront(b, r, i);
    }
    dress_urban_street_props(b, 4, strip_d, sx, blvd_y as i32);
}

fn build_curved_avenue_plaza(b: &mut SectorBuilder) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let north_h = (h / 3).clamp(4, 10);
    let south_y0 = h - north_h;
    let road_w_cm = if h < 18 { 380 } else { 520 };
    let max_bend = (hf * 0.5 - north_h as f32 - road_w_cm as f32 / 200.0 - 0.35).max(0.45);
    let bend_mag = (hf * 0.12).min(max_bend);
    let bend = if b.rng.chance(1, 2) { bend_mag } else { -bend_mag };
    let pts = [(0.0, hf * 0.5 - bend), (wf * 0.35, hf * 0.5 + bend), (wf * 0.68, hf * 0.5 - bend * 0.8), (wf, hf * 0.5 + bend * 0.5)];
    let s1 = b.rng.next() as u32;
    b.add_ribbon_spline(SurfaceFeatureKind::AsphaltRoad, &pts, road_w_cm, s1);

    let plaza = Rect { x0: (w / 3).max(4), y0: 0, x1: (2 * w / 3).min(w - 4), y1: north_h };
    let (px0, py0, px1, py1) = (plaza.x0 as f32, plaza.y0 as f32, plaza.x1 as f32, plaza.y1 as f32);
    let walk_m = (rh(plaza) as f32 * 0.28).clamp(1.0, 2.2);
    let (gx0, gx1) = (px0 + walk_m, (px1 - walk_m).max(px0 + walk_m + 1.2));
    let (gy0, gy1) = (py0 + walk_m, (py1 - walk_m).max(py0 + walk_m + 1.0));
    b.add_polygon_surface(SurfaceFeatureKind::StoneWalk, &[(px0, py0), (px1, py0), (px1, gy0), (px0, gy0)], 24, 44);
    b.add_polygon_surface(SurfaceFeatureKind::StoneWalk, &[(px0, gy1), (px1, gy1), (px1, py1), (px0, py1)], 24, 45);
    b.add_polygon_surface(SurfaceFeatureKind::StoneWalk, &[(px0, gy0), (gx0, gy0), (gx0, gy1), (px0, gy1)], 24, 46);
    b.add_polygon_surface(SurfaceFeatureKind::StoneWalk, &[(gx1, gy0), (px1, gy0), (px1, gy1), (gx1, gy1)], 24, 47);
    b.add_polygon_surface(
        SurfaceFeatureKind::GardenBedZone,
        &[(gx0 - 0.2, gy0 - 0.2), (gx1 + 0.2, gy0 - 0.2), (gx1 + 0.2, gy1 + 0.2), (gx0 - 0.2, gy1 + 0.2)],
        75,
        48,
    );

    let blocks = [
        Rect { x0: 0, y0: 0, x1: (plaza.x0 - 1).max(3), y1: north_h },
        Rect { x0: (plaza.x1 + 1).min(w - 3), y0: 0, x1: w, y1: north_h },
        Rect { x0: 0, y0: south_y0, x1: (w / 2 - 2).max(4), y1: h },
        Rect { x0: (w / 2 + 2).min(w - 4), y0: south_y0, x1: w, y1: h },
    ];
    for (i, &r) in blocks.iter().enumerate() {
        add_sidewalk_apron(b, Rect { x0: r.x0, y0: (r.y0 - 2).max(0), x1: r.x1, y1: (r.y1 + 2).min(h) }, 60 + i as u32);
        enclose_storefront(b, r, i, i % 2 == 0, true);
    }
    b.reserve_spawns_and_doors();
    for (i, &r) in blocks.iter().enumerate() {
        furnish_storefront(b, r, i);
    }
    b.scatter_filtered(PropKind::RoundTable, plaza, 3, true, SurfaceAllowance::EXTERIOR_OFF_ROAD);
    dress_urban_street_props(b, 4, north_h, w / 2, h / 2);
}

fn build_staggered_crossroads(b: &mut SectorBuilder) {
    let (w, h) = (b.w, b.h);
    let ew_y = h as f32 * 0.5;
    let off = (w / 6).clamp(3, 8);
    let n_x = (w / 2 - off).clamp(5, w - 7) as f32;
    let s_x = (w / 2 + off).clamp(7, w - 5) as f32;

    let (sr1, sr2, sr3) = (b.rng.next() as u32, b.rng.next() as u32, b.rng.next() as u32);
    let ew_w_cm = if h < 18 { 380 } else { 520 };
    b.add_ribbon_spline(SurfaceFeatureKind::AsphaltRoad, &[(0.0, ew_y), (w as f32 * 0.5, ew_y), (w as f32, ew_y)], ew_w_cm, sr1);
    b.add_ribbon_spline(SurfaceFeatureKind::AsphaltRoad, &[(n_x, 0.0), (n_x, ew_y)], 420, sr2);
    b.add_ribbon_spline(SurfaceFeatureKind::AsphaltRoad, &[(s_x, ew_y), (s_x, h as f32)], 420, sr3);

    let bh = (h / 4).clamp(3, 10);
    let blocks = [
        Rect { x0: 0, y0: 0, x1: (n_x as i32 - 3).max(3), y1: bh },
        Rect { x0: (n_x as i32 + 3).min(w - 3), y0: 0, x1: w, y1: bh },
        Rect { x0: 0, y0: h - bh, x1: (s_x as i32 - 3).max(3), y1: h },
        Rect { x0: (s_x as i32 + 3).min(w - 3), y0: h - bh, x1: w, y1: h },
    ];
    for (i, &r) in blocks.iter().enumerate() {
        add_sidewalk_apron(
            b,
            Rect { x0: (r.x0 - 1).max(0), y0: (r.y0 - 2).max(0), x1: (r.x1 + 1).min(w), y1: (r.y1 + 2).min(h) },
            80 + i as u32,
        );
        enclose_storefront(b, r, i, i % 2 == 0, i == 0 || i == 3);
    }
    b.reserve_spawns_and_doors();
    for (i, &r) in blocks.iter().enumerate() {
        furnish_storefront(b, r, i);
    }
    dress_urban_street_props(b, 4, bh, w / 2, h / 2);
}

fn dress_urban_street_props(b: &mut SectorBuilder, bw: i32, bh: i32, cx: i32, cy: i32) {
    let (w, h) = (b.w, b.h);
    let whole = b.whole();
    let off_road = SurfaceAllowance::EXTERIOR_OFF_ROAD;
    let road_ok = SurfaceAllowance::EXTERIOR_ANY_ROAD;

    let t1_rot = b.rng.range(0, 360);
    if b.try_place_filtered(PropKind::MediumTree, (bw, bh), t1_rot, None, whole, off_road)
        .or_else(|| b.try_place_filtered(PropKind::MediumTree, (cx - 2, bh), t1_rot, None, whole, off_road))
        .or_else(|| b.try_place_filtered(PropKind::MediumTree, (bw + 1, h - 2), t1_rot, None, whole, off_road))
        .or_else(|| b.try_place_filtered(PropKind::MediumTree, (bw, bh), 0, None, whole, off_road))
        .is_none()
    {
        b.scatter_filtered(PropKind::MediumTree, whole, 1, false, off_road);
    }

    if w >= 16 && h >= 12 {
        let lane_x = (cx - 3).clamp(2, w - 5);
        let car_rot = 180 + b.rng.range(-4, 5);
        let van_rot = 90 + b.rng.range(-4, 5);
        if b.try_place_filtered(PropKind::Car, (lane_x, bh.clamp(2, h - 4)), car_rot, None, whole, road_ok)
            .or_else(|| b.try_place_filtered(PropKind::Car, (lane_x, bh.clamp(2, h - 4)), 180, None, whole, road_ok))
            .is_none()
        {
            b.scatter_filtered(PropKind::Car, whole, 1, false, road_ok);
        }
        if b.try_place_filtered(PropKind::UtilityVan, (w - bw - 5, (cy + 1).min(h - 4)), van_rot, None, whole, road_ok)
            .or_else(|| b.try_place_filtered(PropKind::UtilityVan, (w - bw - 5, (cy + 1).min(h - 4)), 90, None, whole, road_ok))
            .is_none()
        {
            b.scatter_filtered(PropKind::UtilityVan, whole, 1, false, road_ok);
        }
    }
    let jb1 = b.rng.range(-4, 5).rem_euclid(360);
    let jb2 = b.rng.range(-4, 5).rem_euclid(360);
    if b.try_place_filtered(PropKind::JerseyBarrier, ((cx - 3).max(2), (cy - 1).max(2)), jb1, Some(2), whole, road_ok)
        .or_else(|| b.try_place_filtered(PropKind::JerseyBarrier, ((cx - 3).max(2), (cy - 1).max(2)), 0, Some(2), whole, road_ok))
        .is_none()
    {
        b.scatter_filtered(PropKind::JerseyBarrier, whole, 1, false, road_ok);
    }
    let _ = b.try_place_filtered(PropKind::JerseyBarrier, ((cx + 1).min(w - 4), (cy + 1).min(h - 3)), jb2, Some(2), whole, road_ok)
        .or_else(|| b.try_place_filtered(PropKind::JerseyBarrier, ((cx + 1).min(w - 4), (cy + 1).min(h - 3)), 0, Some(2), whole, road_ok));
    let _ = b.try_place_filtered(PropKind::Dumpster, ((w - bw - 3).max(2), 1), 0, None, whole, off_road);
    if b.try_place_filtered(PropKind::ParkBench, ((bw + 1).min(w - 4), (h - bh - 1).max(2)), 0, None, whole, off_road).is_none() {
        b.scatter_filtered(PropKind::ParkBench, whole, 1, false, off_road);
    }
    let scale = (w * h / 140).clamp(2, 6) as usize;
    let planter_bounds = Rect { x0: bw.min(w.saturating_sub(4)), y0: bh.min(h.saturating_sub(4)), x1: (w - bw).max(bw + 2).min(w), y1: (h - bh).max(bh + 2).min(h) };
    if b.scatter_filtered(PropKind::Planter, planter_bounds, scale, w >= 12 && h >= 12, off_road) == 0 {
        b.scatter_filtered(PropKind::Planter, whole, scale.min(3), false, off_road);
    }
    if b.scatter_filtered(PropKind::WoodCrate, whole, scale, w >= 12 && h >= 12, off_road) == 0 {
        b.scatter_filtered(PropKind::WoodCrate, whole, 1, false, off_road);
    }
}
