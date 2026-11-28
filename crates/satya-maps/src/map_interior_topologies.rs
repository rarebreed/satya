//! Scaled multi-topology architectural grammars for interior sectors (`Apartment`, `Bar`, `Office`).
//!
//! When board dimensions reach `>= 24×18`, replaces single full-bleed floor plans with three
//! seeded site topologies:
//! - [`InteriorTopology::StreetfrontSetback`]: Border avenue + concrete sidewalk apron + setback facade.
//! - [`InteriorTopology::CourtyardWing`]: U-shaped three-wing layout around an open stone & garden court.
//! - [`InteriorTopology::CentralCorridorSuite`]: Spine corridor with West/East entry doors and flanking suites.

use crate::map_interior_gen::{InteriorStyle, Role, Room, Shell};
use crate::map_pool_spa_gen::{build_pool_surface, pick_pool_shape};
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
        tint_rgb: None,
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
        tint_rgb: None,
    }
}

fn enclose_building_rect(shell: &mut Shell, r: Room, wall_mat: MaterialClass, win_step: i32) {
    let off = shell.rng.range(0, win_step.max(2));
    for x in r.x0..r.x1 {
        let is_win = x > r.x0 && x + 1 < r.x1 && (x - r.x0 + off) % win_step == 0;
        let (k, m) = if is_win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        shell.put(x, r.y0, EdgeSide::North, k, m);
        shell.put(x, r.y1, EdgeSide::North, k, m);
    }
    for y in r.y0..r.y1 {
        let is_win = y > r.y0 && y + 1 < r.y1 && (y - r.y0 + off) % win_step == 0;
        let (k, m) = if is_win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        shell.put(r.x0, y, EdgeSide::West, k, m);
        shell.put(r.x1, y, EdgeSide::West, k, m);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HallAdj {
    North,
    South,
    West,
    East,
}

#[derive(Debug, Clone, Copy)]
struct VerticalStrip {
    x0: i32,
    x1: i32,
    wall_x: i32,
    adj: HallAdj,
}

fn push_capped_bath_or_room(
    shell: &mut Shell,
    r: Room,
    role: Role,
    adj: HallAdj,
    out: &mut Vec<(Room, Role)>,
) {
    if !matches!(role, Role::Bathroom | Role::MasterBath) || r.area() <= 14 {
        out.push((r, role));
        return;
    }
    match adj {
        HallAdj::South if r.h() > 4 => {
            let cut_h = (14 / r.w().max(1)).clamp(2, 4);
            let sy = (r.y1 - cut_h).max(r.y0 + 1);
            let bath = Room { y0: sy, ..r };
            let rear = Room { y1: sy, ..r };
            shell.partition_h(sy, r.x0, r.x1, &[r.x0 + r.w() / 2]);
            out.push((bath, role));
            let rear_role = if rear.area() >= 12 {
                Role::Office
            } else {
                Role::Closet
            };
            out.push((rear, rear_role));
        }
        HallAdj::North if r.h() > 4 => {
            let cut_h = (14 / r.w().max(1)).clamp(2, 4);
            let sy = (r.y0 + cut_h).min(r.y1 - 1);
            let bath = Room { y1: sy, ..r };
            let rear = Room { y0: sy, ..r };
            shell.partition_h(sy, r.x0, r.x1, &[r.x0 + r.w() / 2]);
            out.push((bath, role));
            let rear_role = if rear.area() >= 12 {
                Role::Office
            } else {
                Role::Closet
            };
            out.push((rear, rear_role));
        }
        HallAdj::East if r.w() > 4 => {
            let cut_w = (14 / r.h().max(1)).clamp(2, 4);
            let sx = (r.x1 - cut_w).max(r.x0 + 1);
            let bath = Room { x0: sx, ..r };
            let rear = Room { x1: sx, ..r };
            shell.partition_v(sx, r.y0, r.y1, &[r.y0 + r.h() / 2]);
            out.push((bath, role));
            let rear_role = if rear.area() >= 12 {
                Role::Office
            } else {
                Role::Closet
            };
            out.push((rear, rear_role));
        }
        HallAdj::West if r.w() > 4 => {
            let cut_w = (14 / r.h().max(1)).clamp(2, 4);
            let sx = (r.x0 + cut_w).min(r.x1 - 1);
            let bath = Room { x1: sx, ..r };
            let rear = Room { x0: sx, ..r };
            shell.partition_v(sx, r.y0, r.y1, &[r.y0 + r.h() / 2]);
            out.push((bath, role));
            let rear_role = if rear.area() >= 12 {
                Role::Office
            } else {
                Role::Closet
            };
            out.push((rear, rear_role));
        }
        _ => {
            if r.w() > 3 && (r.h() <= 4 || r.w() >= r.h()) {
                let cut_w = (14 / r.h().max(1)).clamp(2, 3);
                let sx = if adj == HallAdj::West {
                    r.x0 + cut_w
                } else {
                    r.x1 - cut_w
                }
                .clamp(r.x0 + 2, r.x1 - 2);
                let (bath, rear) = if adj == HallAdj::West {
                    (Room { x1: sx, ..r }, Room { x0: sx, ..r })
                } else {
                    (Room { x0: sx, ..r }, Room { x1: sx, ..r })
                };
                shell.partition_v(sx, r.y0, r.y1, &[r.y0 + r.h() / 2]);
                out.push((bath, role));
                out.push((
                    rear,
                    if rear.area() >= 12 {
                        Role::Bedroom
                    } else {
                        Role::Closet
                    },
                ));
            } else if r.h() > 3 {
                let cut_h = (14 / r.w().max(1)).clamp(2, 3);
                let sy = if adj == HallAdj::North {
                    r.y0 + cut_h
                } else {
                    r.y1 - cut_h
                }
                .clamp(r.y0 + 2, r.y1 - 2);
                let (bath, rear) = if adj == HallAdj::North {
                    (Room { y1: sy, ..r }, Room { y0: sy, ..r })
                } else {
                    (Room { y0: sy, ..r }, Room { y1: sy, ..r })
                };
                shell.partition_h(sy, r.x0, r.x1, &[r.x0 + r.w() / 2]);
                out.push((bath, role));
                out.push((
                    rear,
                    if rear.area() >= 12 {
                        Role::Bedroom
                    } else {
                        Role::Closet
                    },
                ));
            } else {
                out.push((r, Role::Bedroom));
            }
        }
    }
}

fn slice_apartment_span(shell: &mut Shell, len: i32, allow_compact: bool) -> Vec<(i32, bool)> {
    if len <= 0 {
        return Vec::new();
    }
    if len < 5 {
        return vec![(len, allow_compact && len <= 3)];
    }
    let mut out = Vec::new();
    let mut rem = len;
    let mut trailing_compact = None;
    if allow_compact {
        let cw = if rem >= 8 { 3 } else { 2 };
        if rem - cw >= 3 {
            rem -= cw;
            if shell.rng.chance(1, 2) {
                out.push((cw, true));
            } else {
                trailing_compact = Some(cw);
            }
        }
    }
    let target = if rem >= 10 { 4 } else { 3 };
    let count = ((rem + target / 2) / target).clamp(1, (rem / 3).max(1));
    let base = rem / count;
    let extra = rem % count;
    for i in 0..count {
        out.push((base + i32::from(i < extra), false));
    }
    if let Some(cw) = trailing_compact {
        out.push((cw, true));
    }
    out
}

fn assign_apartment_bay_roles(
    shell: &mut Shell,
    bays: &[(Room, HallAdj, bool, bool)],
    allow_kitchen: bool,
    want_living: bool,
    max_baths: usize,
    out: &mut Vec<(Room, Role)>,
) {
    let mut assigned = vec![None::<Role>; bays.len()];
    for (i, &(_, _, _, is_living)) in bays.iter().enumerate() {
        if is_living {
            assigned[i] = Some(Role::Living);
        }
    }
    let mut bath_count = 0usize;
    // Place Bathroom / MasterBath on compact bays (or smallest bay fallback).
    for (i, &(r, _, compact, _)) in bays.iter().enumerate() {
        if assigned[i].is_some() || !compact || bath_count >= max_baths {
            continue;
        }
        let role = if bath_count == 0 {
            Role::Bathroom
        } else {
            Role::MasterBath
        };
        if r.w() >= 2 && r.h() >= 2 {
            assigned[i] = Some(role);
            bath_count += 1;
        }
    }
    if bath_count == 0
        && max_baths > 0
        && let Some((bi, _)) = bays
            .iter()
            .enumerate()
            .filter(|&(i, bay)| assigned[i].is_none() && bay.0.w() >= 2 && bay.0.h() >= 2)
            .min_by_key(|&(_, bay)| bay.0.area())
    {
        assigned[bi] = Some(Role::Bathroom);
    }

    let mut have_living = !want_living || assigned.contains(&Some(Role::Living));
    if !have_living
        && let Some((li, _)) = bays
            .iter()
            .enumerate()
            .filter(|&(i, bay)| assigned[i].is_none() && bay.0.w().min(bay.0.h()) >= 3)
            .max_by_key(|&(_, bay)| bay.0.area())
            .or_else(|| {
                bays.iter()
                    .enumerate()
                    .filter(|&(i, _)| assigned[i].is_none())
                    .max_by_key(|&(_, bay)| bay.0.area())
            })
    {
        assigned[li] = Some(Role::Living);
        have_living = true;
    }
    let _ = have_living;

    if allow_kitchen
        && !assigned.contains(&Some(Role::Kitchen))
        && let Some((ki, _)) = bays
            .iter()
            .enumerate()
            .filter(|&(i, bay)| assigned[i].is_none() && bay.0.w().min(bay.0.h()) >= 3)
            .min_by_key(|&(_, bay)| (bay.0.area() - 16).abs())
            .or_else(|| {
                bays.iter()
                    .enumerate()
                    .filter(|&(i, _)| assigned[i].is_none())
                    .max_by_key(|&(_, bay)| bay.0.area())
            })
    {
        assigned[ki] = Some(Role::Kitchen);
    }

    let mut palette = [
        Role::MasterBedroom,
        Role::Dining,
        Role::Bedroom,
        Role::Office,
        Role::Library,
        Role::Sunroom,
    ];
    let rot = (shell.rng.next() as usize) % 3;
    palette.rotate_left(rot);
    let mut pi = 0usize;
    let mut did_master = false;
    let mut did_dining = false;
    let mut did_closet = false;

    let mut open: Vec<usize> = (0..bays.len()).filter(|&i| assigned[i].is_none()).collect();
    open.sort_by_key(|&i| -bays[i].0.area());
    for i in open {
        let r = bays[i].0;
        if (r.area() <= 8 || r.w() <= 2 || r.h() <= 2) && !did_closet {
            assigned[i] = Some(Role::Closet);
            did_closet = true;
        } else if !did_master && r.w().min(r.h()) >= 3 {
            assigned[i] = Some(Role::MasterBedroom);
            did_master = true;
        } else if !did_dining && want_living && r.w().min(r.h()) >= 3 && bays.len() >= 4 {
            assigned[i] = Some(Role::Dining);
            did_dining = true;
        } else {
            let role = loop {
                let c = palette[pi % palette.len()];
                pi += 1;
                if matches!(c, Role::MasterBedroom | Role::Dining) {
                    continue;
                }
                break c;
            };
            assigned[i] = Some(role);
        }
    }

    for (i, &(r, adj, _, _)) in bays.iter().enumerate() {
        let role = assigned[i].unwrap_or(Role::Bedroom);
        push_capped_bath_or_room(shell, r, role, adj, out);
    }
}

/// Subdivides an apartment building or wing around a continuous `Role::Hall` spine,
/// keeping small compact units foyer-free (`Living`/`Dining`/`Kitchen` entry), capping
/// bathroom sizes at `<= 14 m²`, and preventing duplicate kitchen/bath spam.
pub(crate) fn partition_apartment_zone(
    shell: &mut Shell,
    zone: Room,
    allow_kitchen: bool,
    _allow_foyer: bool,
    direct_west_entry_y: Option<i32>,
    out: &mut Vec<(Room, Role)>,
) {
    // Branch A: Compact Direct-Entry Unit (no Foyer, entry opens directly into Living/Dining/Kitchen -> Hall).
    if let Some(mid_y) = direct_west_entry_y
        && zone.w() >= 9
        && zone.h() >= 7
    {
        let ex1 = (zone.x0 + (zone.w() * 30 / 100).clamp(4, 5)).min(zone.x1 - 5);
        let hy0 = (mid_y - 1).clamp(zone.y0 + 2, zone.y1 - 4);
        let hy1 = hy0 + 2;
        let entry_role = if !allow_kitchen {
            if shell.rng.chance(3, 4) {
                Role::Living
            } else {
                Role::Dining
            }
        } else {
            match shell.rng.range(0, 10) {
                0..=5 => Role::Living,
                6..=7 => Role::Dining,
                _ => Role::Kitchen,
            }
        };
        let mut have_kitchen = entry_role == Role::Kitchen;
        let mut have_living = entry_role == Role::Living;
        if hy0 - zone.y0 >= 3 {
            let top_w = Room {
                x0: zone.x0,
                y0: zone.y0,
                x1: ex1,
                y1: hy0,
            };
            let entry_r = Room {
                x0: zone.x0,
                y0: hy0,
                x1: ex1,
                y1: zone.y1,
            };
            let door_x = (zone.x0 + 2).min(ex1 - 1);
            shell.partition_h(hy0, zone.x0, ex1, &[door_x]);
            let top_role = if allow_kitchen && !have_kitchen {
                have_kitchen = true;
                Role::Kitchen
            } else if !have_living {
                have_living = true;
                Role::Living
            } else {
                Role::Dining
            };
            out.push((top_w, top_role));
            out.push((entry_r, entry_role));
        } else {
            let entry_r = Room {
                x0: zone.x0,
                y0: zone.y0,
                x1: ex1,
                y1: zone.y1,
            };
            out.push((entry_r, entry_role));
        }

        // Open flow between entry bay and Hall along hy0..hy1; wall the rest of x = ex1.
        shell.partition_v(ex1, zone.y0, hy0, &[]);
        shell.partition_v(ex1, hy1, zone.y1, &[]);
        out.push((
            Room {
                x0: ex1,
                y0: hy0,
                x1: zone.x1,
                y1: hy1,
            },
            Role::Hall,
        ));

        let bath_north = shell.rng.chance(1, 2);
        let mut bays = Vec::new();
        for (is_north, sy0, sy1, wall_y, adj) in [
            (true, zone.y0, hy0, hy0, HallAdj::South),
            (false, hy1, zone.y1, hy1, HallAdj::North),
        ] {
            let segs = slice_apartment_span(shell, zone.x1 - ex1, is_north == bath_north);
            let mut cur_x = ex1;
            let mut doors = Vec::new();
            for (slen, compact) in segs {
                if cur_x > ex1 {
                    shell.partition_v(cur_x, sy0, sy1, &[]);
                }
                let r = Room {
                    x0: cur_x,
                    y0: sy0,
                    x1: cur_x + slen,
                    y1: sy1,
                };
                doors.push(cur_x + slen / 2);
                bays.push((r, adj, compact, false));
                cur_x += slen;
            }
            shell.partition_h(wall_y, ex1, zone.x1, &doors);
        }
        assign_apartment_bay_roles(
            shell,
            &bays,
            allow_kitchen && !have_kitchen,
            !have_living,
            1,
            out,
        );
        return;
    }

    // Branch B: Horizontal Hallway Spine.
    if zone.w() >= 8 && zone.h() >= 7 && zone.w() >= zone.h() {
        let mid_y = shell.h / 2;
        let hy0 = if mid_y >= zone.y0 + 2 && mid_y < zone.y1 - 2 {
            mid_y - 1
        } else {
            zone.y0 + (zone.h() - 2) / 2
        };
        let hy1 = hy0 + 2;
        out.push((
            Room {
                x0: zone.x0,
                y0: hy0,
                x1: zone.x1,
                y1: hy1,
            },
            Role::Hall,
        ));

        let want_two_baths = allow_kitchen && zone.area() >= 180;
        let bath_north = want_two_baths || shell.rng.chance(1, 2);
        let bath_south = want_two_baths || !bath_north;
        let mut bays = Vec::new();
        for (compact_ok, sy0, sy1, wall_y, adj) in [
            (bath_north, zone.y0, hy0, hy0, HallAdj::South),
            (bath_south, hy1, zone.y1, hy1, HallAdj::North),
        ] {
            let segs = slice_apartment_span(shell, zone.w(), compact_ok);
            let mut cur_x = zone.x0;
            let mut doors = Vec::new();
            for (slen, compact) in segs {
                if cur_x > zone.x0 {
                    shell.partition_v(cur_x, sy0, sy1, &[]);
                }
                let r = Room {
                    x0: cur_x,
                    y0: sy0,
                    x1: cur_x + slen,
                    y1: sy1,
                };
                doors.push(cur_x + slen / 2);
                bays.push((r, adj, compact, false));
                cur_x += slen;
            }
            shell.partition_h(wall_y, zone.x0, zone.x1, &doors);
        }
        assign_apartment_bay_roles(
            shell,
            &bays,
            allow_kitchen,
            true,
            if want_two_baths { 2 } else { 1 },
            out,
        );
        return;
    }

    // Branch C: Vertical Hallway Spine (e.g., CourtyardWing side wings).
    if zone.w() >= 6 && zone.h() >= 8 && zone.h() > zone.w() {
        let mid_y = shell.h / 2;
        let (hx0, hx1, strips) = if zone.x0 == 0 && zone.x1 < shell.w {
            let x = zone.x1 - 2;
            (
                x,
                zone.x1,
                vec![VerticalStrip {
                    x0: zone.x0,
                    x1: x,
                    wall_x: x,
                    adj: HallAdj::East,
                }],
            )
        } else if zone.x1 == shell.w && zone.x0 > 0 {
            let x = zone.x0;
            (
                x,
                x + 2,
                vec![VerticalStrip {
                    x0: x + 2,
                    x1: zone.x1,
                    wall_x: x + 2,
                    adj: HallAdj::West,
                }],
            )
        } else {
            let x = zone.x0 + (zone.w() - 2) / 2;
            (
                x,
                x + 2,
                vec![
                    VerticalStrip {
                        x0: zone.x0,
                        x1: x,
                        wall_x: x,
                        adj: HallAdj::East,
                    },
                    VerticalStrip {
                        x0: x + 2,
                        x1: zone.x1,
                        wall_x: x + 2,
                        adj: HallAdj::West,
                    },
                ],
            )
        };
        out.push((
            Room {
                x0: hx0,
                y0: zone.y0,
                x1: hx1,
                y1: zone.y1,
            },
            Role::Hall,
        ));

        let mut bays = Vec::new();
        for (
            strip_idx,
            VerticalStrip {
                x0: sx0,
                x1: sx1,
                wall_x,
                adj,
            },
        ) in strips.into_iter().enumerate()
        {
            let touches_ext = sx0 == 0 || sx1 == shell.w;
            let mut band0 = (mid_y - 1).min(zone.y1 - 3).max(zone.y0);
            if band0 > zone.y0 && band0 - zone.y0 < 3 {
                band0 = zone.y0;
            }
            let mut band1 = (mid_y + 2).max(band0 + 3).min(zone.y1);
            if band1 < zone.y1 && zone.y1 - band1 < 3 {
                band1 = zone.y1;
            }

            let top_len = band0 - zone.y0;
            let bot_len = zone.y1 - band1;
            let top_segs = slice_apartment_span(shell, top_len, top_len >= 5);
            let bot_segs = slice_apartment_span(shell, bot_len, top_segs.iter().all(|s| !s.1));
            let mut seg_spans: Vec<(i32, i32, bool, bool)> = Vec::new();
            let mut cy = zone.y0;
            for (slen, compact) in top_segs {
                seg_spans.push((cy, cy + slen, compact, false));
                cy += slen;
            }
            seg_spans.push((band0, band1, false, touches_ext && strip_idx == 0));
            cy = band1;
            for (slen, compact) in bot_segs {
                seg_spans.push((cy, cy + slen, compact, false));
                cy += slen;
            }

            let mut doors = Vec::new();
            for (bi, (ry0, ry1, compact, is_living)) in seg_spans.into_iter().enumerate() {
                if bi > 0 {
                    shell.partition_h(ry0, sx0, sx1, &[]);
                }
                let r = Room {
                    x0: sx0,
                    y0: ry0,
                    x1: sx1,
                    y1: ry1,
                };
                doors.push(ry0 + (ry1 - ry0) / 2);
                bays.push((r, adj, compact, is_living));
            }
            shell.partition_v(wall_x, zone.y0, zone.y1, &doors);
        }
        assign_apartment_bay_roles(shell, &bays, allow_kitchen, true, 1, out);
        return;
    }

    // Branch D: Compact Small-Zone Fallback (zero Foyer, connected rooms, capped Bath).
    if zone.w() >= 6 && zone.h() >= 5 {
        let vx = (zone.x0 + zone.w() / 2).clamp(zone.x0 + 3, zone.x1 - 3);
        let my = (zone.y0 + zone.h() / 2).clamp(zone.y0 + 2, zone.y1 - 2);
        shell.partition_v(vx, zone.y0, zone.y1, &[zone.y0 + 1]);
        shell.partition_h(my, vx, zone.x1, &[vx + (zone.x1 - vx) / 2]);
        let living = Room {
            x0: zone.x0,
            y0: zone.y0,
            x1: vx,
            y1: zone.y1,
        };
        let r_top = Room {
            x0: vx,
            y0: zone.y0,
            x1: zone.x1,
            y1: my,
        };
        let r_bot = Room {
            x0: vx,
            y0: my,
            x1: zone.x1,
            y1: zone.y1,
        };
        out.push((living, Role::Living));
        out.push((
            r_top,
            if allow_kitchen {
                Role::Kitchen
            } else {
                Role::Bedroom
            },
        ));
        push_capped_bath_or_room(shell, r_bot, Role::Bathroom, HallAdj::West, out);
    } else {
        out.push((zone, Role::Living));
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
            partition_apartment_zone(shell, zone, true, true, None, out);
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
                const BACK: [Role; 4] = [
                    Role::Storage,
                    Role::Restroom,
                    Role::BackKitchen,
                    Role::Office,
                ];
                for i in 0..n_back as usize {
                    let (y0, y1) = (cuts[i], cuts[i + 1]);
                    doors.push(shell.rng.range(y0 + 1, (y1 - 1).max(y0 + 2)));
                    if i > 0 {
                        shell.partition_h(y0, sx, zone.x1, &[]);
                    }
                    out.push((
                        Room {
                            x0: sx,
                            y0,
                            x1: zone.x1,
                            y1,
                        },
                        BACK[i % BACK.len()],
                    ));
                }
                shell.partition_v(sx, zone.y0, zone.y1, &doors);
                out.push((
                    Room {
                        x0: zone.x0,
                        y0: zone.y0,
                        x1: sx,
                        y1: zone.y1,
                    },
                    Role::BarHall,
                ));
            } else {
                out.push((zone, Role::BarHall));
            }
        }
        InteriorStyle::Office => {
            if zone.w() >= 12 {
                let sx = zone.x0 + (zone.w() * 6 / 10).clamp(6, zone.w() - 4);
                let n_east = if zone.h() >= 14 {
                    4
                } else if zone.h() >= 10 {
                    3
                } else {
                    2
                };
                let mut cuts = vec![zone.y0];
                for i in 1..n_east {
                    let base = zone.y0 + zone.h() * i / n_east;
                    cuts.push((base + shell.rng.range(-1, 2)).clamp(zone.y0 + 3, zone.y1 - 3));
                }
                cuts.push(zone.y1);
                const EAST: [Role; 5] = [
                    Role::ConferenceRoom,
                    Role::Restroom,
                    Role::Office,
                    Role::Breakroom,
                    Role::ServerCloset,
                ];
                for i in 0..n_east as usize {
                    let (y0, y1) = (cuts[i], cuts[i + 1]);
                    let door = shell.rng.range(y0 + 1, (y1 - 1).max(y0 + 2));
                    if i > 0 {
                        shell.partition_h_styled(
                            y0,
                            sx,
                            zone.x1,
                            &[],
                            (WallEdgeKind::Wall, MaterialClass::Drywall),
                        );
                    }
                    if i == 0 {
                        shell.partition_v_styled(
                            sx,
                            y0,
                            y1,
                            &[door],
                            (WallEdgeKind::Window, MaterialClass::Glass),
                        );
                        shell.put(
                            sx,
                            y0,
                            EdgeSide::West,
                            WallEdgeKind::Wall,
                            MaterialClass::Drywall,
                        );
                    } else {
                        shell.partition_v(sx, y0, y1, &[door]);
                    }
                    out.push((
                        Room {
                            x0: sx,
                            y0,
                            x1: zone.x1,
                            y1,
                        },
                        EAST[i % EAST.len()],
                    ));
                }
                out.push((
                    Room {
                        x0: zone.x0,
                        y0: zone.y0,
                        x1: sx,
                        y1: zone.y1,
                    },
                    Role::Bullpen,
                ));
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
        InteriorTopology::CentralCorridorSuite => {
            build_central_corridor_suite(style, shell, wall_mat)
        }
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
            Room {
                x0: 0,
                y0: 0,
                x1: w,
                y1: h - apron_d,
            },
            Room {
                x0: 0,
                y0: h - apron_d,
                x1: w,
                y1: h - 2,
            },
            h as f32 - 1.1,
        )
    } else {
        (
            Room {
                x0: 0,
                y0: apron_d,
                x1: w,
                y1: h,
            },
            Room {
                x0: 0,
                y0: 2,
                x1: w,
                y1: apron_d,
            },
            1.1,
        )
    };

    let s1 = shell.rng.next() as u32;
    let s2 = shell.rng.next() as u32;
    let s3 = shell.rng.next() as u32;
    let surfaces = vec![
        ribbon_surface(
            SurfaceFeatureKind::AsphaltRoad,
            &[
                (-0.4, road_y),
                (w as f32 * 0.5, road_y + 0.15),
                (w as f32 + 0.4, road_y - 0.1),
            ],
            240,
            s1,
        ),
        poly_surface(
            SurfaceFeatureKind::ConcreteWalk,
            &[
                (0.0, sidewalk.y0 as f32),
                (w as f32, sidewalk.y0 as f32),
                (w as f32, sidewalk.y1 as f32),
                (0.0, sidewalk.y1 as f32),
            ],
            0,
            s2,
        ),
        rect_floor(bldg, s3),
    ];

    enclose_building_rect(shell, bldg, wall_mat, 3);
    let facade_y = if street_south { bldg.y1 } else { bldg.y0 };
    for &dx in &[w / 3, (2 * w) / 3] {
        shell.put(
            dx,
            facade_y,
            EdgeSide::North,
            WallEdgeKind::DoorOpen,
            MaterialClass::Glass,
        );
    }
    shell.put(
        0,
        mid_y,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::SolidWood,
    );
    shell.put(
        w,
        mid_y,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::SolidWood,
    );

    let mut rooms = Vec::new();
    subdivide_zone_by_style(style, shell, bldg, &mut rooms);
    let patio = Room {
        x0: 2,
        y0: sidewalk.y0,
        x1: w - 2,
        y1: sidewalk.y1,
    };
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

    let left = Room {
        x0: 0,
        y0: 0,
        x1: cx0,
        y1: h,
    };
    let right = Room {
        x0: cx1,
        y0: 0,
        x1: w,
        y1: h,
    };
    let (spine, court, mouth_y) = if open_south {
        (
            Room {
                x0: cx0,
                y0: 0,
                x1: cx1,
                y1: spine_h,
            },
            Room {
                x0: cx0,
                y0: spine_h,
                x1: cx1,
                y1: h,
            },
            h,
        )
    } else {
        (
            Room {
                x0: cx0,
                y0: h - spine_h,
                x1: cx1,
                y1: h,
            },
            Room {
                x0: cx0,
                y0: 0,
                x1: cx1,
                y1: h - spine_h,
            },
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
    let mut surfaces = vec![
        poly_surface(
            SurfaceFeatureKind::StoneWalk,
            &[(cx0_f, cy0_f), (cx1_f, cy0_f), (cx1_f, gy0), (cx0_f, gy0)],
            0,
            s1,
        ),
        poly_surface(
            SurfaceFeatureKind::StoneWalk,
            &[(cx0_f, gy1), (cx1_f, gy1), (cx1_f, cy1_f), (cx0_f, cy1_f)],
            0,
            s1.wrapping_add(1),
        ),
        poly_surface(
            SurfaceFeatureKind::StoneWalk,
            &[(cx0_f, gy0), (gx0, gy0), (gx0, gy1), (cx0_f, gy1)],
            0,
            s1.wrapping_add(2),
        ),
        poly_surface(
            SurfaceFeatureKind::StoneWalk,
            &[(gx1, gy0), (cx1_f, gy0), (cx1_f, gy1), (gx1, gy1)],
            0,
            s1.wrapping_add(3),
        ),
        rect_floor(left, s3),
        rect_floor(right, s4),
        rect_floor(spine, s5),
    ];
    let gw = gx1 - gx0;
    let gh = gy1 - gy0;
    let center_roll = if style == InteriorStyle::Apartment && gw >= 3.6 && gh >= 3.0 {
        shell.rng.range(0, 100)
    } else {
        99
    };
    if center_roll < 45 {
        surfaces.push(poly_surface(
            SurfaceFeatureKind::StoneWalk,
            &[
                (gx0 - 0.15, gy0 - 0.15),
                (gx1 + 0.15, gy0 - 0.15),
                (gx1 + 0.15, gy1 + 0.15),
                (gx0 - 0.15, gy1 + 0.15),
            ],
            0,
            s2,
        ));
        let pbounds = (gx0 + 0.15, gy0 + 0.15, gx1 - 0.15, gy1 - 0.15);
        let shape = pick_pool_shape(
            shell.rng.next(),
            pbounds.2 - pbounds.0,
            pbounds.3 - pbounds.1,
        );
        let basin = build_pool_surface(shape, pbounds, 8.0, shell.rng.next());
        surfaces.push(basin.pool_surface);
    } else if center_roll < 65 {
        surfaces.push(poly_surface(
            SurfaceFeatureKind::WoodDeck,
            &[
                (gx0 - 0.15, gy0 - 0.15),
                (gx1 + 0.15, gy0 - 0.15),
                (gx1 + 0.15, gy1 + 0.15),
                (gx0 - 0.15, gy1 + 0.15),
            ],
            45,
            s2,
        ));
    } else {
        surfaces.push(poly_surface(
            SurfaceFeatureKind::GardenBedZone,
            &[
                (gx0 - 0.25, gy0 - 0.25),
                (gx1 + 0.25, gy0 - 0.25),
                (gx1 + 0.25, gy1 + 0.25),
                (gx0 - 0.25, gy1 + 0.25),
            ],
            85,
            s2,
        ));
    }

    enclose_building_rect(shell, left, wall_mat, 4);
    enclose_building_rect(shell, right, wall_mat, 4);
    enclose_building_rect(shell, spine, wall_mat, 3);
    // Connect wings internally across shared seams and open courtyard doors on each wing.
    let spine_door_y = (spine.y0 + spine.y1) / 2;
    shell.put(
        cx0,
        spine_door_y,
        EdgeSide::West,
        WallEdgeKind::DoorOpen,
        MaterialClass::SolidWood,
    );
    shell.put(
        cx1,
        spine_door_y,
        EdgeSide::West,
        WallEdgeKind::DoorOpen,
        MaterialClass::SolidWood,
    );
    let court_door_y = (court.y0 + court.y1) / 2;
    shell.put(
        cx0,
        court_door_y,
        EdgeSide::West,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    shell.put(
        cx1,
        court_door_y,
        EdgeSide::West,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    let spine_seam_y = if open_south { spine.y1 } else { spine.y0 };
    shell.put(
        (cx0 + cx1) / 2,
        spine_seam_y,
        EdgeSide::North,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    // Exterior spawn entrances on West & East wings + open stone gate along courtyard mouth.
    shell.put(
        0,
        mid_y,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::SolidWood,
    );
    shell.put(
        w,
        mid_y,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::SolidWood,
    );
    let _ = mouth_y;

    let mut rooms = Vec::new();
    if style == InteriorStyle::Apartment {
        partition_apartment_zone(shell, left, false, false, None, &mut rooms);
        partition_apartment_zone(shell, spine, true, false, None, &mut rooms);
        partition_apartment_zone(shell, right, false, false, None, &mut rooms);
    } else {
        subdivide_zone_by_style(style, shell, left, &mut rooms);
        subdivide_zone_by_style(style, shell, spine, &mut rooms);
        subdivide_zone_by_style(style, shell, right, &mut rooms);
    }
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
    let whole = Room {
        x0: 0,
        y0: 0,
        x1: w,
        y1: h,
    };
    let surfaces = vec![rect_floor(whole, shell.rng.next() as u32)];

    enclose_building_rect(shell, whole, wall_mat, 3);
    shell.put(
        0,
        mid_y,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::SolidWood,
    );
    shell.put(
        w,
        mid_y,
        EdgeSide::West,
        WallEdgeKind::DoorClosed,
        MaterialClass::SolidWood,
    );

    let n_suites = (w / 7).clamp(3, 6);
    let mut xs = vec![0];
    for i in 1..n_suites {
        let base = (w * i) / n_suites;
        xs.push((base + shell.rng.range(-1, 2)).clamp(xs[i as usize - 1] + 4, w - 4));
    }
    xs.push(w);

    let mut rooms = Vec::new();
    let cor_mat = if style == InteriorStyle::Office {
        MaterialClass::Glass
    } else {
        MaterialClass::Drywall
    };
    let mut north_doors = Vec::new();
    let mut south_doors = Vec::new();
    if style == InteriorStyle::Apartment {
        let n_cols = xs.len() - 1;
        let bath_n_col = n_cols - 1;
        let bath_s_col = (n_suites >= 4).then_some(n_cols - 1);
        const SUITE_CYCLE: [Role; 6] = [
            Role::MasterBedroom,
            Role::Dining,
            Role::Bedroom,
            Role::Library,
            Role::Office,
            Role::Sunroom,
        ];
        for i in 0..n_cols {
            let (x0, x1) = (xs[i], xs[i + 1]);
            if i > 0 {
                shell.partition_v(x0, 0, cor_y0, &[]);
                shell.partition_v(x0, cor_y1, h, &[]);
            }
            let nr = Room {
                x0,
                y0: 0,
                x1,
                y1: cor_y0,
            };
            let sr = Room {
                x0,
                y0: cor_y1,
                x1,
                y1: h,
            };
            if i == bath_n_col {
                if x1 - x0 >= 6 {
                    let sx = x0 + 3;
                    shell.partition_v(sx, 0, cor_y0, &[]);
                    north_doors.push(((x0 + sx) / 2).clamp(x0 + 1, sx - 1));
                    north_doors.push(((sx + x1) / 2).clamp(sx + 1, x1 - 1));
                    push_capped_bath_or_room(
                        shell,
                        Room {
                            x0,
                            y0: 0,
                            x1: sx,
                            y1: cor_y0,
                        },
                        Role::Bathroom,
                        HallAdj::South,
                        &mut rooms,
                    );
                    rooms.push((
                        Room {
                            x0: sx,
                            y0: 0,
                            x1,
                            y1: cor_y0,
                        },
                        Role::Office,
                    ));
                } else {
                    north_doors.push(((x0 + x1) / 2).clamp(x0 + 1, x1 - 1));
                    push_capped_bath_or_room(shell, nr, Role::Bathroom, HallAdj::South, &mut rooms);
                }
            } else {
                north_doors.push(((x0 + x1) / 2).clamp(x0 + 1, x1 - 1));
                let n_role = if i == 0 {
                    Role::Kitchen
                } else {
                    SUITE_CYCLE[(i - 1) % SUITE_CYCLE.len()]
                };
                rooms.push((nr, n_role));
            }

            if bath_s_col == Some(i) {
                if x1 - x0 >= 6 {
                    let sx = x0 + 3;
                    shell.partition_v(sx, cor_y1, h, &[]);
                    south_doors.push(((x0 + sx) / 2).clamp(x0 + 1, sx - 1));
                    south_doors.push(((sx + x1) / 2).clamp(sx + 1, x1 - 1));
                    push_capped_bath_or_room(
                        shell,
                        Room {
                            x0,
                            y0: cor_y1,
                            x1: sx,
                            y1: h,
                        },
                        Role::MasterBath,
                        HallAdj::North,
                        &mut rooms,
                    );
                    rooms.push((
                        Room {
                            x0: sx,
                            y0: cor_y1,
                            x1,
                            y1: h,
                        },
                        Role::Bedroom,
                    ));
                } else {
                    south_doors.push(((x0 + x1) / 2).clamp(x0 + 1, x1 - 1));
                    push_capped_bath_or_room(
                        shell,
                        sr,
                        Role::MasterBath,
                        HallAdj::North,
                        &mut rooms,
                    );
                }
            } else {
                south_doors.push(((x0 + x1) / 2).clamp(x0 + 1, x1 - 1));
                let s_role = if i == 0 {
                    Role::Living
                } else {
                    SUITE_CYCLE[(i + 1) % SUITE_CYCLE.len()]
                };
                rooms.push((sr, s_role));
            }
        }
    } else {
        let roles: &[Role] = match style {
            InteriorStyle::Apartment => unreachable!(),
            InteriorStyle::Bar => &[
                Role::BarHall,
                Role::BackKitchen,
                Role::Storage,
                Role::Restroom,
                Role::Office,
            ],
            InteriorStyle::Office => &[
                Role::Bullpen,
                Role::ConferenceRoom,
                Role::Office,
                Role::Breakroom,
                Role::ServerCloset,
            ],
        };
        for i in 0..xs.len() - 1 {
            let (x0, x1) = (xs[i], xs[i + 1]);
            north_doors.push((x0 + x1) / 2);
            south_doors.push(((x0 + x1) / 2).clamp(x0 + 1, x1 - 1));
            if i > 0 {
                shell.partition_v(x0, 0, cor_y0, &[]);
                shell.partition_v(x0, cor_y1, h, &[]);
            }
            let nr = Room {
                x0,
                y0: 0,
                x1,
                y1: cor_y0,
            };
            let sr = Room {
                x0,
                y0: cor_y1,
                x1,
                y1: h,
            };
            rooms.push((nr, roles[i % roles.len()]));
            rooms.push((sr, roles[(i + 2) % roles.len()]));
        }
    }
    let cor_wall = (
        if style == InteriorStyle::Office {
            WallEdgeKind::Window
        } else {
            WallEdgeKind::Wall
        },
        cor_mat,
    );
    shell.partition_h_styled(cor_y0, 0, w, &north_doors, cor_wall);
    shell.partition_h_styled(cor_y1, 0, w, &south_doors, cor_wall);
    let corridor = Room {
        x0: 0,
        y0: cor_y0,
        x1: w,
        y1: cor_y1,
    };
    rooms.push((corridor, Role::Hall));
    (rooms, surfaces, Vec::new())
}
