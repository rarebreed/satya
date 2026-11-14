//! Scaled multi-topology architectural grammars for interior sectors (`Apartment`, `Bar`, `Office`).
//!
//! When board dimensions reach `>= 24×18`, replaces single full-bleed floor plans with three
//! seeded site topologies:
//! - [`InteriorTopology::StreetfrontSetback`]: Border avenue + concrete sidewalk apron + setback facade.
//! - [`InteriorTopology::CourtyardWing`]: U-shaped three-wing layout around an open stone & garden court.
//! - [`InteriorTopology::CentralCorridorSuite`]: Spine corridor with West/East entry doors and flanking suites.

use crate::map_interior_gen::{InteriorStyle, Role, Room, Shell};
use satya_schema::{
    EdgeSide, MapPointCm, MapSurfaceFeature, MaterialClass, SurfaceFeatureKind, SurfaceShapeKind,
    WallEdgeKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteriorTopology {
    StreetfrontSetback,
    CourtyardWing,
    CentralCorridorSuite,
}

fn poly_surface(
    kind: SurfaceFeatureKind,
    pts_m: &[(f32, f32)],
    corner_radius_cm: i32,
    seed: u32,
) -> MapSurfaceFeature {
    MapSurfaceFeature {
        kind,
        shape: SurfaceShapeKind::Polygon,
        points_cm: pts_m
            .iter()
            .map(|&(x, y)| MapPointCm::new((x * 100.0).round() as i32, (y * 100.0).round() as i32))
            .collect(),
        width_cm: 0,
        border_style_cm: corner_radius_cm,
        variant_seed: seed,
    }
}

fn rect_floor(r: Room, seed: u32) -> MapSurfaceFeature {
    poly_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[
            (r.x0 as f32, r.y0 as f32),
            (r.x1 as f32, r.y0 as f32),
            (r.x1 as f32, r.y1 as f32),
            (r.x0 as f32, r.y1 as f32),
        ],
        0,
        seed,
    )
}

fn ribbon_surface(
    kind: SurfaceFeatureKind,
    pts_m: &[(f32, f32)],
    width_cm: i32,
    seed: u32,
) -> MapSurfaceFeature {
    MapSurfaceFeature {
        kind,
        shape: SurfaceShapeKind::RibbonSpline,
        points_cm: pts_m
            .iter()
            .map(|&(x, y)| MapPointCm::new((x * 100.0).round() as i32, (y * 100.0).round() as i32))
            .collect(),
        width_cm,
        border_style_cm: 0,
        variant_seed: seed,
    }
}

fn enclose_building_rect(shell: &mut Shell, r: Room, wall_mat: MaterialClass, win_step: i32) {
    let off = shell.rng.range(0, win_step.max(2));
    for x in r.x0..r.x1 {
        let is_win = x > r.x0 && x + 1 < r.x1 && (x - r.x0 + off) % win_step == 0;
        let (k, m) = if is_win { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, wall_mat) };
        shell.put(x, r.y0, EdgeSide::North, k, m);
        shell.put(x, r.y1, EdgeSide::North, k, m);
    }
    for y in r.y0..r.y1 {
        let is_win = y > r.y0 && y + 1 < r.y1 && (y - r.y0 + off) % win_step == 0;
        let (k, m) = if is_win { (WallEdgeKind::Window, MaterialClass::Glass) } else { (WallEdgeKind::Wall, wall_mat) };
        shell.put(r.x0, y, EdgeSide::West, k, m);
        shell.put(r.x1, y, EdgeSide::West, k, m);
    }
}

fn subdivide_zone_by_style(
    style: InteriorStyle,
    shell: &mut Shell,
    zone: Room,
    out: &mut Vec<(Room, Role)>,
) {
    match style {
        InteriorStyle::Apartment => {
            let mut rooms = Vec::new();
            let min_dim = if zone.w() >= 12 && zone.h() >= 10 { 5 } else { 4 };
            shell.bsp(zone, 0, min_dim, 3, &mut rooms);
            rooms.sort_by_key(|r| -r.area());
            let n = rooms.len();
            for (i, r) in rooms.into_iter().enumerate() {
                let role = match i {
                    0 => Role::Living,
                    1 => Role::Kitchen,
                    _ if i == n - 1 && n >= 4 => Role::Bathroom,
                    _ if i % 2 == 0 => Role::Bedroom,
                    _ => Role::Office,
                };
                out.push((r, role));
            }
        }
        InteriorStyle::Bar => {
            if zone.w() >= 12 {
                let strip_w = (zone.w() * 3 / 10).clamp(4, 9);
                let sx = zone.x1 - strip_w;
                let n_back = if zone.h() >= 12 { 3 } else { 2 };
                let mut cuts = vec![zone.y0];
                for i in 1..n_back {
                    let base = zone.y0 + zone.h() * i / n_back;
                    cuts.push((base + shell.rng.range(-1, 2)).clamp(zone.y0 + 3, zone.y1 - 3));
                }
                cuts.push(zone.y1);
                let mut doors = Vec::new();
                const BACK: [Role; 4] = [Role::Storage, Role::Restroom, Role::BackKitchen, Role::Office];
                for i in 0..n_back as usize {
                    let (y0, y1) = (cuts[i], cuts[i + 1]);
                    doors.push(shell.rng.range(y0 + 1, (y1 - 1).max(y0 + 2)));
                    if i > 0 {
                        shell.partition_h(y0, sx, zone.x1, &[]);
                    }
                    out.push((Room { x0: sx, y0, x1: zone.x1, y1 }, BACK[i % BACK.len()]));
                }
                shell.partition_v(sx, zone.y0, zone.y1, &doors);
                out.push((Room { x0: zone.x0, y0: zone.y0, x1: sx, y1: zone.y1 }, Role::BarHall));
            } else {
                out.push((zone, Role::BarHall));
            }
        }
        InteriorStyle::Office => {
            if zone.w() >= 12 {
                let sx = zone.x0 + (zone.w() * 6 / 10).clamp(6, zone.w() - 4);
                let n_east = if zone.h() >= 14 { 4 } else if zone.h() >= 10 { 3 } else { 2 };
                let mut cuts = vec![zone.y0];
                for i in 1..n_east {
                    let base = zone.y0 + zone.h() * i / n_east;
                    cuts.push((base + shell.rng.range(-1, 2)).clamp(zone.y0 + 3, zone.y1 - 3));
                }
                cuts.push(zone.y1);
                const EAST: [Role; 4] = [
                    Role::ConferenceRoom,
                    Role::Office,
                    Role::Breakroom,
                    Role::ServerCloset,
                ];
                for i in 0..n_east as usize {
                    let (y0, y1) = (cuts[i], cuts[i + 1]);
                    let door = shell.rng.range(y0 + 1, (y1 - 1).max(y0 + 2));
                    if i > 0 {
                        shell.partition_h_styled(y0, sx, zone.x1, &[], (WallEdgeKind::Wall, MaterialClass::Drywall));
                    }
                    if i == 0 {
                        shell.partition_v_styled(sx, y0, y1, &[door], (WallEdgeKind::Window, MaterialClass::Glass));
                        shell.put(sx, y0, EdgeSide::West, WallEdgeKind::Wall, MaterialClass::Drywall);
                    } else {
                        shell.partition_v(sx, y0, y1, &[door]);
                    }
                    out.push((Room { x0: sx, y0, x1: zone.x1, y1 }, EAST[i % EAST.len()]));
                }
                out.push((Room { x0: zone.x0, y0: zone.y0, x1: sx, y1: zone.y1 }, Role::Bullpen));
            } else {
                out.push((zone, Role::Office));
            }
        }
    }
}

/// Generates one of three architectural topologies for `w >= 24 && h >= 18`.
pub(crate) fn build_scaled_interior_layout(
    style: InteriorStyle,
    shell: &mut Shell,
) -> (Vec<(Room, Role)>, Vec<MapSurfaceFeature>, Vec<Room>) {
    let topo = match shell.rng.range(0, 3) {
        0 => InteriorTopology::StreetfrontSetback,
        1 => InteriorTopology::CourtyardWing,
        _ => InteriorTopology::CentralCorridorSuite,
    };
    let wall_mat = match style {
        InteriorStyle::Office => MaterialClass::ConcreteBlock,
        InteriorStyle::Apartment | InteriorStyle::Bar => MaterialClass::Brick,
    };
    match topo {
        InteriorTopology::StreetfrontSetback => build_streetfront_setback(style, shell, wall_mat),
        InteriorTopology::CourtyardWing => build_courtyard_wing(style, shell, wall_mat),
        InteriorTopology::CentralCorridorSuite => build_central_corridor_suite(style, shell, wall_mat),
    }
}

fn build_streetfront_setback(
    style: InteriorStyle,
    shell: &mut Shell,
    wall_mat: MaterialClass,
) -> (Vec<(Room, Role)>, Vec<MapSurfaceFeature>, Vec<Room>) {
    let (w, h) = (shell.w, shell.h);
    let mid_y = h / 2;
    let street_south = shell.rng.chance(1, 2);
    let apron_d = (h / 5).clamp(4, 6);
    let (bldg, sidewalk, road_y) = if street_south {
        (
            Room { x0: 0, y0: 0, x1: w, y1: h - apron_d },
            Room { x0: 0, y0: h - apron_d, x1: w, y1: h - 2 },
            h as f32 - 1.1,
        )
    } else {
        (
            Room { x0: 0, y0: apron_d, x1: w, y1: h },
            Room { x0: 0, y0: 2, x1: w, y1: apron_d },
            1.1,
        )
    };

    let s1 = shell.rng.next() as u32;
    let s2 = shell.rng.next() as u32;
    let s3 = shell.rng.next() as u32;
    let surfaces = vec![
        ribbon_surface(SurfaceFeatureKind::AsphaltRoad, &[(-0.4, road_y), (w as f32 * 0.5, road_y + 0.15), (w as f32 + 0.4, road_y - 0.1)], 240, s1),
        poly_surface(
            SurfaceFeatureKind::ConcreteWalk,
            &[(0.0, sidewalk.y0 as f32), (w as f32, sidewalk.y0 as f32), (w as f32, sidewalk.y1 as f32), (0.0, sidewalk.y1 as f32)],
            0,
            s2,
        ),
        rect_floor(bldg, s3),
    ];

    enclose_building_rect(shell, bldg, wall_mat, 3);
    let facade_y = if street_south { bldg.y1 } else { bldg.y0 };
    for &dx in &[w / 3, (2 * w) / 3] {
        shell.put(dx, facade_y, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    }
    shell.put(0, mid_y, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SolidWood);
    shell.put(w, mid_y, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SolidWood);

    let mut rooms = Vec::new();
    subdivide_zone_by_style(style, shell, bldg, &mut rooms);
    let patio = Room { x0: 2, y0: sidewalk.y0, x1: w - 2, y1: sidewalk.y1 };
    (rooms, surfaces, vec![patio])
}

fn build_courtyard_wing(
    style: InteriorStyle,
    shell: &mut Shell,
    wall_mat: MaterialClass,
) -> (Vec<(Room, Role)>, Vec<MapSurfaceFeature>, Vec<Room>) {
    let (w, h) = (shell.w, shell.h);
    let mid_y = h / 2;
    let wing_w = (w * 3 / 10).clamp(6, 18);
    let spine_h = (h * 4 / 10).clamp(6, 16);
    let open_south = shell.rng.chance(1, 2);
    let cx0 = wing_w;
    let cx1 = w - wing_w;

    let left = Room { x0: 0, y0: 0, x1: cx0, y1: h };
    let right = Room { x0: cx1, y0: 0, x1: w, y1: h };
    let (spine, court, mouth_y) = if open_south {
        (
            Room { x0: cx0, y0: 0, x1: cx1, y1: spine_h },
            Room { x0: cx0, y0: spine_h, x1: cx1, y1: h },
            h,
        )
    } else {
        (
            Room { x0: cx0, y0: h - spine_h, x1: cx1, y1: h },
            Room { x0: cx0, y0: 0, x1: cx1, y1: h - spine_h },
            0,
        )
    };

    let (s1, s2, s3, s4, s5) = (
        shell.rng.next() as u32,
        shell.rng.next() as u32,
        shell.rng.next() as u32,
        shell.rng.next() as u32,
        shell.rng.next() as u32,
    );
    let (cx0_f, cy0_f, cx1_f, cy1_f) = (
        court.x0 as f32,
        court.y0 as f32,
        court.x1 as f32,
        court.y1 as f32,
    );
    let walk_m = (court.w().min(court.h()) as f32 * 0.22).clamp(2.0, 3.2);
    let (gx0, gx1) = (cx0_f + walk_m, (cx1_f - walk_m).max(cx0_f + walk_m + 1.6));
    let (gy0, gy1) = (cy0_f + walk_m, (cy1_f - walk_m).max(cy0_f + walk_m + 1.4));
    let surfaces = vec![
        poly_surface(SurfaceFeatureKind::StoneWalk, &[(cx0_f, cy0_f), (cx1_f, cy0_f), (cx1_f, gy0), (cx0_f, gy0)], 0, s1),
        poly_surface(SurfaceFeatureKind::StoneWalk, &[(cx0_f, gy1), (cx1_f, gy1), (cx1_f, cy1_f), (cx0_f, cy1_f)], 0, s1.wrapping_add(1)),
        poly_surface(SurfaceFeatureKind::StoneWalk, &[(cx0_f, gy0), (gx0, gy0), (gx0, gy1), (cx0_f, gy1)], 0, s1.wrapping_add(2)),
        poly_surface(SurfaceFeatureKind::StoneWalk, &[(gx1, gy0), (cx1_f, gy0), (cx1_f, gy1), (gx1, gy1)], 0, s1.wrapping_add(3)),
        poly_surface(
            SurfaceFeatureKind::GardenBedZone,
            &[(gx0 - 0.25, gy0 - 0.25), (gx1 + 0.25, gy0 - 0.25), (gx1 + 0.25, gy1 + 0.25), (gx0 - 0.25, gy1 + 0.25)],
            85,
            s2,
        ),
        rect_floor(left, s3),
        rect_floor(right, s4),
        rect_floor(spine, s5),
    ];

    enclose_building_rect(shell, left, wall_mat, 4);
    enclose_building_rect(shell, right, wall_mat, 4);
    enclose_building_rect(shell, spine, wall_mat, 3);
    // Connect wings internally across shared seams and open courtyard doors on each wing.
    let spine_door_y = (spine.y0 + spine.y1) / 2;
    shell.put(cx0, spine_door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
    shell.put(cx1, spine_door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
    let court_door_y = (court.y0 + court.y1) / 2;
    shell.put(cx0, court_door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    shell.put(cx1, court_door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    let spine_seam_y = if open_south { spine.y1 } else { spine.y0 };
    shell.put((cx0 + cx1) / 2, spine_seam_y, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
    // Exterior spawn entrances on West & East wings + open stone gate along courtyard mouth.
    shell.put(0, mid_y, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SolidWood);
    shell.put(w, mid_y, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SolidWood);
    let _ = mouth_y;

    let mut rooms = Vec::new();
    subdivide_zone_by_style(style, shell, left, &mut rooms);
    subdivide_zone_by_style(style, shell, spine, &mut rooms);
    subdivide_zone_by_style(style, shell, right, &mut rooms);
    (rooms, surfaces, vec![court])
}

fn build_central_corridor_suite(
    style: InteriorStyle,
    shell: &mut Shell,
    wall_mat: MaterialClass,
) -> (Vec<(Room, Role)>, Vec<MapSurfaceFeature>, Vec<Room>) {
    let (w, h) = (shell.w, shell.h);
    let mid_y = h / 2;
    let cor_y0 = (mid_y - 1).max(4);
    let cor_y1 = (mid_y + 1 + i32::from(h >= 24)).min(h - 4);
    let whole = Room { x0: 0, y0: 0, x1: w, y1: h };
    let surfaces = vec![rect_floor(whole, shell.rng.next() as u32)];

    enclose_building_rect(shell, whole, wall_mat, 3);
    shell.put(0, mid_y, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SolidWood);
    shell.put(w, mid_y, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SolidWood);

    let n_suites = (w / 7).clamp(3, 6);
    let mut xs = vec![0];
    for i in 1..n_suites {
        let base = (w * i) / n_suites;
        xs.push((base + shell.rng.range(-1, 2)).clamp(xs[i as usize - 1] + 4, w - 4));
    }
    xs.push(w);

    let mut rooms = Vec::new();
    let roles: &[Role] = match style {
        InteriorStyle::Apartment => &[Role::Living, Role::Kitchen, Role::Bedroom, Role::Office, Role::Bathroom],
        InteriorStyle::Bar => &[Role::BarHall, Role::BackKitchen, Role::Storage, Role::Restroom, Role::Office],
        InteriorStyle::Office => &[Role::Bullpen, Role::ConferenceRoom, Role::Office, Role::Breakroom, Role::ServerCloset],
    };
    let cor_mat = if style == InteriorStyle::Office { MaterialClass::Glass } else { MaterialClass::Drywall };
    let mut north_doors = Vec::new();
    let mut south_doors = Vec::new();
    for i in 0..xs.len() - 1 {
        let (x0, x1) = (xs[i], xs[i + 1]);
        north_doors.push((x0 + x1) / 2);
        south_doors.push(((x0 + x1) / 2).clamp(x0 + 1, x1 - 1));
        if i > 0 {
            shell.partition_v(x0, 0, cor_y0, &[]);
            shell.partition_v(x0, cor_y1, h, &[]);
        }
        let nr = Room { x0, y0: 0, x1, y1: cor_y0 };
        let sr = Room { x0, y0: cor_y1, x1, y1: h };
        rooms.push((nr, roles[i % roles.len()]));
        rooms.push((sr, roles[(i + 2) % roles.len()]));
    }
    let cor_wall = (if style == InteriorStyle::Office { WallEdgeKind::Window } else { WallEdgeKind::Wall }, cor_mat);
    shell.partition_h_styled(cor_y0, 0, w, &north_doors, cor_wall);
    shell.partition_h_styled(cor_y1, 0, w, &south_doors, cor_wall);
    let corridor = Room { x0: 0, y0: cor_y0, x1: w, y1: cor_y1 };
    rooms.push((corridor, Role::Office));
    (rooms, surfaces, Vec::new())
}
