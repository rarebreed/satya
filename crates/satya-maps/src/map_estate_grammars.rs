//! Multi-topology procedural site grammars for `GardenEstate` ("Walled Estate & Courtyard").
//!
//! Generates architectural residential compounds using [`crate::map_gen_residential::generate_home_in_lot`]
//! and [`crate::map_gen_zoning::FloorStack`] (`B1`/`1F`/`2F` levels with aligned `Stairwell`s and
//! wall-flush room furnishings), furnished brick carriage houses, occasional meandering
//! [`SurfaceFeatureKind::CreekChannel`] streams with perpendicular [`SurfaceFeatureKind::BridgeDeck`]
//! footbridges, randomized multi-shape [`SurfaceFeatureKind::PoolWater`] terraces, and styled
//! Jacuzzi/Spa courts across compact (`20×14`) through massive (`96×72`) arenas:
//! - [`EstateTopology::ForecourtTurnaroundManor`]: Outer avenue, gate-pier entry, circular
//!   turnaround drive around a sculptural fountain parterre, multi-level grand manor & carriage house.
//! - [`EstateTopology::LWingCarriageCompound`]: Quadrant-mirrored L-wing mansion framing a paved
//!   courtyard terrace, opposite carriage wing, pool/spa terrace & sweeping driveway.
//! - [`EstateTopology::CloisteredPavilionVilla`]: Opposed main villa & guest pavilion flanking a
//!   pillared cloister courtyard with water garden / pool / creek & perimeter shade orchard.

use crate::map_gen_residential::{
    CardinalEntry, LotBox, generate_annex_wing_in_lot, generate_home_in_lot,
};
use crate::map_gen_zoning::FloorStack;
use crate::map_interior_gen::{Furnisher, Room};
use crate::map_outdoor_gen::{OutdoorBuilder, RectM};
use crate::map_outdoor_natural::{CurveProfile, CurveSpec, synthesize_natural_curve_m};
use crate::map_pool_spa_gen::{OutdoorPoolSpaOpts, try_place_outdoor_pool_or_spa};
use crate::map_surface_geom::sample_open_catmull_rom_m;
use crate::map_surface_queries::surface_contains_point_m;
use crate::tactical_map::ProceduralSector;
use satya_schema::{
    EconomicTier, EdgeSide, MaterialClass, PropKind, SurfaceFeatureKind, WallEdgeKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EstateTopology {
    ForecourtTurnaroundManor,
    LWingCarriageCompound,
    CloisteredPavilionVilla,
}

#[derive(Debug, Clone, Copy)]
struct BoxI {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl BoxI {
    const fn w(self) -> i32 {
        self.x1 - self.x0
    }
    const fn h(self) -> i32 {
        self.y1 - self.y0
    }
    const fn lot(self) -> LotBox {
        LotBox::new(self.x0, self.y0, self.x1, self.y1)
    }
    fn rect_m(self, inset: f32) -> RectM {
        RectM {
            x0: self.x0 as f32 + inset,
            y0: self.y0 as f32 + inset,
            x1: self.x1 as f32 - inset,
            y1: self.y1 as f32 - inset,
        }
    }
}

/// Entry point for [`crate::map_outdoor_gen::OutdoorStyle::GardenEstate`].
pub(crate) fn build_garden_estate(b: OutdoorBuilder) -> ProceduralSector {
    let (w, h) = (b.w, b.h);
    if w < 24 || h < 18 {
        return build_compact_estate(b);
    }
    let mut b = b;
    let topo = match b.rng.range(0, 3) {
        0 => EstateTopology::ForecourtTurnaroundManor,
        1 => EstateTopology::LWingCarriageCompound,
        _ => EstateTopology::CloisteredPavilionVilla,
    };
    let mut stack = FloorStack::default();
    match topo {
        EstateTopology::ForecourtTurnaroundManor => build_forecourt_turnaround(&mut b, &mut stack),
        EstateTopology::LWingCarriageCompound => build_lwing_compound(&mut b, &mut stack),
        EstateTopology::CloisteredPavilionVilla => build_cloistered_villa(&mut b, &mut stack),
    }
    stack.into_sector(b, w, h)
}

fn rect_poly(x0: f32, y0: f32, x1: f32, y1: f32) -> [(f32, f32); 4] {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn ellipse_poly(cx: f32, cy: f32, rx: f32, ry: f32, steps: usize) -> Vec<(f32, f32)> {
    (0..steps)
        .map(|i| {
            let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
            (cx + a.cos() * rx, cy + a.sin() * ry)
        })
        .collect()
}

fn reserve_door_clearance(b: &mut OutdoorBuilder, x: i32, y: i32, side: EdgeSide) {
    match side {
        EdgeSide::North => {
            b.spawn_pads_m.push((x as f32 + 0.5, y as f32 + 0.5));
            if y > 0 {
                b.spawn_pads_m.push((x as f32 + 0.5, y as f32 - 0.5));
            }
        }
        EdgeSide::West => {
            b.spawn_pads_m.push((x as f32 + 0.5, y as f32 + 0.5));
            if x > 0 {
                b.spawn_pads_m.push((x as f32 - 0.5, y as f32 + 0.5));
            }
        }
    }
}

fn put_door(
    b: &mut OutdoorBuilder,
    x: i32,
    y: i32,
    side: EdgeSide,
    open: bool,
    mat: MaterialClass,
) {
    let kind = if open {
        WallEdgeKind::DoorOpen
    } else {
        WallEdgeKind::DoorClosed
    };
    b.put_edge(x, y, side, kind, mat);
    reserve_door_clearance(b, x, y, side);
}

fn place_rot_m(
    b: &mut OutdoorBuilder,
    kind: PropKind,
    x_m: f32,
    y_m: f32,
    rot_lo: i32,
    rot_hi: i32,
    allow_on_pavement: bool,
) {
    let rot = b.rng.range(rot_lo, rot_hi);
    b.try_place_cm(
        kind,
        OutdoorBuilder::cm(x_m),
        OutdoorBuilder::cm(y_m),
        rot,
        None,
        allow_on_pavement,
    );
}

fn build_compound_wall(
    b: &mut OutdoorBuilder,
    comp: BoxI,
    north_gate_x0: i32,
    north_gate_w: i32,
    south_gate_x0: i32,
    south_gate_w: i32,
) {
    for x in comp.x0..comp.x1 {
        let is_grille = (x - comp.x0) % 4 == 1 || (x - comp.x0) % 4 == 2;
        let (kind, mat) = if is_grille {
            (WallEdgeKind::Window, MaterialClass::SheetSteel)
        } else {
            (WallEdgeKind::Wall, MaterialClass::Brick)
        };
        if north_gate_w > 0 && x >= north_gate_x0 && x < north_gate_x0 + north_gate_w {
            b.remove_edge(x, comp.y0, EdgeSide::North);
            reserve_door_clearance(b, x, comp.y0, EdgeSide::North);
        } else {
            b.put_edge(x, comp.y0, EdgeSide::North, kind, mat);
        }
        if south_gate_w > 0 && x >= south_gate_x0 && x < south_gate_x0 + south_gate_w {
            b.remove_edge(x, comp.y1, EdgeSide::North);
            reserve_door_clearance(b, x, comp.y1, EdgeSide::North);
        } else {
            b.put_edge(x, comp.y1, EdgeSide::North, kind, mat);
        }
    }
    let mid_y = (comp.y0 + comp.y1) / 2;
    let board_mid = b.h / 2;
    for y in comp.y0..comp.y1 {
        let is_gate_y = y == mid_y || y == mid_y - 1 || y == board_mid || y == board_mid - 1;
        if is_gate_y {
            b.remove_edge(comp.x0, y, EdgeSide::West);
            b.remove_edge(comp.x1, y, EdgeSide::West);
            reserve_door_clearance(b, comp.x0, y, EdgeSide::West);
            reserve_door_clearance(b, comp.x1, y, EdgeSide::West);
        } else {
            let (kind, mat) = if (y - comp.y0) % 4 == 1 {
                (WallEdgeKind::Window, MaterialClass::SheetSteel)
            } else {
                (WallEdgeKind::Wall, MaterialClass::Brick)
            };
            b.put_edge(comp.x0, y, EdgeSide::West, kind, mat);
            b.put_edge(comp.x1, y, EdgeSide::West, kind, mat);
        }
    }
}

fn build_carriage_house(b: &mut OutdoorBuilder, r: BoxI, bay_on_south: bool) {
    b.add_poly(
        SurfaceFeatureKind::StainedConcreteFloor,
        &rect_poly(r.x0 as f32, r.y0 as f32, r.x1 as f32, r.y1 as f32),
        0,
    );
    for x in r.x0..r.x1 {
        b.put_edge(
            x,
            r.y0,
            EdgeSide::North,
            WallEdgeKind::Wall,
            MaterialClass::Brick,
        );
        b.put_edge(
            x,
            r.y1,
            EdgeSide::North,
            WallEdgeKind::Wall,
            MaterialClass::Brick,
        );
    }
    for y in r.y0..r.y1 {
        let (kw, mat) = if y == r.y0 + 1 {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, MaterialClass::Brick)
        };
        b.put_edge(r.x0, y, EdgeSide::West, kw, mat);
        b.put_edge(r.x1, y, EdgeSide::West, kw, mat);
    }
    let bay_y = if bay_on_south { r.y1 } else { r.y0 };
    let mid_x = (r.x0 + r.x1) / 2;
    put_door(
        b,
        mid_x - 1,
        bay_y,
        EdgeSide::North,
        true,
        MaterialClass::SolidWood,
    );
    put_door(
        b,
        mid_x,
        bay_y,
        EdgeSide::North,
        true,
        MaterialClass::SolidWood,
    );
    put_door(
        b,
        r.x0,
        (r.y0 + r.y1) / 2,
        EdgeSide::West,
        false,
        MaterialClass::SolidWood,
    );

    let mut furn = Furnisher::new(b.w, b.h, b.rng.next());
    furn.reserve_doors(&b.edges);
    for &(sx, sy) in &b.spawn_pads_m {
        furn.reserve_cell(sx.floor() as i32, sy.floor() as i32);
    }
    let rm = Room {
        x0: r.x0,
        y0: r.y0,
        x1: r.x1,
        y1: r.y1,
    };
    let _ = furn.against_any_wall(PropKind::SteelShelving, rm, None);
    let _ = furn
        .against_any_wall(PropKind::TestBench, rm, None)
        .or_else(|| furn.against_any_wall(PropKind::SolderingStation, rm, None));
    let _ = furn.against_any_wall(PropKind::AirCompressor, rm, None);
    let _ = furn.against_any_wall(PropKind::WoodCrate, rm, None);
    let _ = furn.against_any_wall(PropKind::WoodCrate, rm, None);
    let _ = furn.against_any_wall(PropKind::DrumCluster, rm, None);
    let _ = furn.in_corner(PropKind::FireExtinguisher, rm);
    b.props.extend(furn.props);
}

fn try_add_estate_creek(b: &mut OutdoorBuilder, comp: BoxI, band: RectM, horizontal: bool) -> bool {
    if band.x1 - band.x0 < 3.4 || band.y1 - band.y0 < 3.4 {
        return false;
    }
    let (start_m, end_m, max_lat) = if horizontal {
        let ym = 0.5 * (band.y0 + band.y1);
        let lat = ((band.y1 - band.y0) * 0.34).clamp(0.45, 1.35);
        (
            (band.x0, ym + b.rng.f32_range(-lat * 0.4, lat * 0.4)),
            (band.x1, ym + b.rng.f32_range(-lat * 0.4, lat * 0.4)),
            lat,
        )
    } else {
        let xm = 0.5 * (band.x0 + band.x1);
        let lat = ((band.x1 - band.x0) * 0.34).clamp(0.45, 1.35);
        (
            (xm + b.rng.f32_range(-lat * 0.4, lat * 0.4), band.y0),
            (xm + b.rng.f32_range(-lat * 0.4, lat * 0.4), band.y1),
            lat,
        )
    };
    let pts = synthesize_natural_curve_m(
        &mut b.rng,
        start_m,
        end_m,
        CurveSpec {
            profile: CurveProfile::WildStream,
            max_lateral_m: max_lat,
            ribbon_half_w_m: 0.92,
            clamp_rect: band,
        },
    );
    let dense = sample_open_catmull_rom_m(&pts, 4);
    for &(px, py) in &dense {
        for s in &b.surfaces {
            if (s.kind.is_built_floor_or_deck()
                || matches!(
                    s.kind,
                    SurfaceFeatureKind::AsphaltRoad
                        | SurfaceFeatureKind::ConcreteWalk
                        | SurfaceFeatureKind::BrickPaverWalk
                        | SurfaceFeatureKind::StoneWalk
                        | SurfaceFeatureKind::PoolWater
                ))
                && (surface_contains_point_m(s, px, py)
                    || surface_contains_point_m(s, px + 0.9, py)
                    || surface_contains_point_m(s, px - 0.9, py)
                    || surface_contains_point_m(s, px, py + 0.9)
                    || surface_contains_point_m(s, px, py - 0.9))
            {
                return false;
            }
        }
        // Keep clear of building perimeter walls (compound perimeter walls become grilles).
        for e in &b.edges {
            let on_comp = (e.side == EdgeSide::West && (e.x == comp.x0 || e.x == comp.x1))
                || (e.side == EdgeSide::North && (e.y == comp.y0 || e.y == comp.y1));
            if on_comp {
                continue;
            }
            let (ex, ey) = match e.side {
                EdgeSide::West => (e.x as f32, e.y as f32 + 0.5),
                EdgeSide::North => (e.x as f32 + 0.5, e.y as f32),
            };
            if (px - ex).hypot(py - ey) < 1.25 {
                return false;
            }
        }
    }

    let width_cm = b.rng.range(165, 206);
    let saved_surf_len = b.surfaces.len();
    let saved_props_len = b.props.len();
    let saved_pads_len = b.spawn_pads_m.len();
    let creek_idx = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &pts, width_cm);
    let abutments_dry = |b: &OutdoorBuilder, neg: (f32, f32), pos: (f32, f32)| -> bool {
        !b.surfaces.iter().any(|s| {
            s.kind.is_water_or_channel()
                && (surface_contains_point_m(s, neg.0, neg.1)
                    || surface_contains_point_m(s, pos.0, pos.1))
        })
    };
    let Some(primary_bridge) = b.place_perpendicular_bridge(creek_idx, 0.50, 2.2, 2.8) else {
        b.surfaces.truncate(saved_surf_len);
        b.props.truncate(saved_props_len);
        b.spawn_pads_m.truncate(saved_pads_len);
        return false;
    };
    if !abutments_dry(
        b,
        primary_bridge.abutment_neg_m,
        primary_bridge.abutment_pos_m,
    ) {
        b.surfaces.truncate(saved_surf_len);
        b.props.truncate(saved_props_len);
        b.spawn_pads_m.truncate(saved_pads_len);
        return false;
    }

    let grille_r = width_cm as f32 / 200.0 + 0.45;
    let grille_edges: Vec<(i32, i32, EdgeSide)> = b
        .edges
        .iter()
        .filter(|e| {
            (e.side == EdgeSide::West && (e.x == comp.x0 || e.x == comp.x1))
                || (e.side == EdgeSide::North && (e.y == comp.y0 || e.y == comp.y1))
        })
        .filter(|e| {
            let (ex, ey) = match e.side {
                EdgeSide::West => (e.x as f32, e.y as f32 + 0.5),
                EdgeSide::North => (e.x as f32 + 0.5, e.y as f32),
            };
            dense
                .iter()
                .any(|&(px, py)| (px - ex).hypot(py - ey) <= grille_r)
        })
        .map(|e| (e.x, e.y, e.side))
        .collect();
    for (ex_i, ey_i, side) in grille_edges {
        b.put_edge(
            ex_i,
            ey_i,
            side,
            WallEdgeKind::Window,
            MaterialClass::SheetSteel,
        );
    }

    let chord_len = (end_m.0 - start_m.0).hypot(end_m.1 - start_m.1);
    if chord_len >= 18.0 {
        let br2_surf_len = b.surfaces.len();
        let br2_props_len = b.props.len();
        let br2_pads_len = b.spawn_pads_m.len();
        if let Some(br2) = b.place_perpendicular_bridge(creek_idx, 0.25, 2.0, 2.7)
            && !abutments_dry(b, br2.abutment_neg_m, br2.abutment_pos_m)
        {
            b.surfaces.truncate(br2_surf_len);
            b.props.truncate(br2_props_len);
            b.spawn_pads_m.truncate(br2_pads_len);
        }
    }

    let bank_off = width_cm as f32 / 200.0 + 0.72;
    for (idx, frac) in [0.22_f32, 0.78_f32].iter().enumerate() {
        let si = ((dense.len() - 1) as f32 * frac).round() as usize;
        let (px, py) = dense[si.min(dense.len() - 1)];
        let (bx, by) = if horizontal {
            (px, py + if idx % 2 == 0 { bank_off } else { -bank_off })
        } else {
            (px + if idx % 2 == 0 { bank_off } else { -bank_off }, py)
        };
        let boulder_rot = b.rng.range(0, 360);
        let _ = b.try_place_cm(
            PropKind::Boulder,
            OutdoorBuilder::cm(bx),
            OutdoorBuilder::cm(by),
            boulder_rot,
            None,
            false,
        );
    }
    true
}

fn dress_estate_grounds(
    b: &mut OutdoorBuilder,
    ground_rect: RectM,
    hedge_anchors: &[(f32, f32, i32)],
    tree_count: i32,
) {
    for &(hx, hy, rot) in hedge_anchors {
        b.try_place_cm(
            PropKind::Hedge,
            OutdoorBuilder::cm(hx),
            OutdoorBuilder::cm(hy),
            rot,
            Some(220),
            false,
        );
    }
    let patch_count = ((tree_count / 3).clamp(3, 8)) as usize;
    for i in 0..patch_count {
        let px = b.rng.f32_range(
            ground_rect.x0 + 1.5,
            (ground_rect.x1 - 1.5).max(ground_rect.x0 + 2.0),
        );
        let py = b.rng.f32_range(
            ground_rect.y0 + 1.5,
            (ground_rect.y1 - 1.5).max(ground_rect.y0 + 2.0),
        );
        let kind = if i % 2 == 0 {
            SurfaceFeatureKind::GardenBedZone
        } else {
            SurfaceFeatureKind::MulchZone
        };
        let rx = b.rng.f32_range(1.1, 2.0);
        let ry = b.rng.f32_range(0.9, 1.7);
        b.try_add_organic_patch(kind, (px, py), (rx, ry), 0.35, 36);
    }
    b.scatter_trees(PropKind::LargeTree, tree_count / 2 + 1, 3.2, ground_rect);
    b.scatter_trees(PropKind::MediumTree, tree_count / 2 + 2, 2.4, ground_rect);
    b.scatter_turf_prop_rot(PropKind::Bush, tree_count, 35, ground_rect);
    b.scatter_turf_prop_rot(
        PropKind::ParkBench,
        (patch_count / 2 + 2) as i32,
        30,
        ground_rect,
    );
    b.scatter_turf_prop_rot(
        PropKind::Planter,
        (patch_count / 2 + 2) as i32,
        45,
        ground_rect,
    );
    b.scatter_turf_prop_rot(
        PropKind::Boulder,
        (patch_count / 3 + 1) as i32,
        30,
        ground_rect,
    );
}

/// Compact `20×14`-compatible estate with furnished multi-room manor, driveway & optional pool/spa or creek.
fn build_compact_estate(mut b: OutdoorBuilder) -> ProceduralSector {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let street_y = hf - 1.65;
    let dy = b.rng.f32_range(-0.2, 0.2);
    let street = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (0.0, street_y + dy),
            (wf * 0.35, street_y + 0.15),
            (wf * 0.7, street_y - 0.15),
            (wf, street_y),
        ],
        320,
    );
    let manor_right = b.rng.chance(1, 2);
    let mx0 = if w < 14 {
        1
    } else if manor_right {
        (w - 12).max(4)
    } else {
        4
    };
    let mx1 = if w < 14 {
        (w - 1).max(mx0 + 2)
    } else if manor_right {
        (w - 4).max(mx0 + 6).min(w - 1)
    } else {
        (mx0 + 8).min(w - 4).max(mx0 + 2)
    };
    let manor = BoxI {
        x0: mx0,
        y0: if h < 12 { 1 } else { 2 },
        x1: mx1,
        y1: if h < 12 {
            (h - 2).max(4).min(h - 1)
        } else {
            (h - 4).max(8).min(h - 1)
        },
    };
    let gate_x = if manor_right {
        4
    } else {
        (w - 7).max(manor.x1)
    };
    b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (gate_x as f32 + 1.0, street_y),
            (gate_x as f32 + 1.1, hf - 3.5),
            (gate_x as f32 + 1.3, (hf * 0.56).max(5.5)),
        ],
        240,
    );
    let walk_y = manor.y1 as f32 + 0.4;
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[
            (gate_x as f32 + 1.3, hf - 3.6),
            (wf * 0.5, walk_y.min(hf - 3.4)),
            ((manor.x0 + manor.x1) as f32 * 0.5, manor.y1 as f32),
        ],
        155,
    );

    let comp = BoxI {
        x0: 1,
        y0: 1,
        x1: w - 1,
        y1: h - 3,
    };
    build_compound_wall(&mut b, comp, w / 2, 2, gate_x, 2);

    let pads = b.spawn_pads_m.clone();
    let home = generate_home_in_lot(
        w,
        h,
        manor.lot(),
        EconomicTier::UpperMiddleClass,
        CardinalEntry::South,
        manor.w() >= 6 && manor.h() >= 5,
        false,
        true,
        &pads,
        b.rng.next(),
    );
    let mut stack = FloorStack::default();
    stack.absorb_home(&mut b, home, false);

    let open_rect = if manor_right {
        RectM {
            x0: 1.6,
            y0: 1.6,
            x1: (manor.x0 as f32 - 0.6).max(1.6),
            y1: (hf - 4.0).max(2.0),
        }
    } else {
        RectM {
            x0: manor.x1 as f32 + 0.6,
            y0: 1.6,
            x1: (wf - 1.6).max(manor.x1 as f32 + 0.6),
            y1: (hf - 4.0).max(2.0),
        }
    };
    let has_pool = b
        .surfaces
        .iter()
        .any(|s| s.kind == SurfaceFeatureKind::PoolWater);
    let roll = b.rng.range(0, 100);
    if roll < 45 {
        let prefer_spa_only = has_pool || b.rng.chance(1, 2);
        let _ = try_place_outdoor_pool_or_spa(
            &mut b,
            open_rect,
            OutdoorPoolSpaOpts {
                allow_pool: !has_pool,
                allow_spa: true,
                prefer_spa_only,
            },
        );
    } else if roll < 75 {
        let creek_band = RectM {
            x0: open_rect.x0 + 0.4,
            y0: open_rect.y0,
            x1: (open_rect.x1 - 0.4).max(open_rect.x0 + 0.4),
            y1: open_rect.y1,
        };
        let _ = try_add_estate_creek(&mut b, comp, creek_band, false);
    } else {
        let _ = b.try_place_cm(
            PropKind::SmallFountain,
            OutdoorBuilder::cm(0.5 * (open_rect.x0 + open_rect.x1)),
            OutdoorBuilder::cm(open_rect.y0 + 2.2),
            0,
            None,
            false,
        );
    }

    b.place_vehicle_along_ribbon(PropKind::Car, street, 0.32, 0.2, 5);
    place_rot_m(
        &mut b,
        PropKind::UtilityVan,
        gate_x as f32 + 1.1,
        hf - 4.3,
        78,
        103,
        true,
    );
    dress_estate_grounds(
        &mut b,
        comp.rect_m(0.8),
        &[(wf * 0.28, 2.8, 0), (wf * 0.72, 2.8, 0)],
        4,
    );
    stack.into_sector(b, w, h)
}

/// Topology 0: Street avenue, gate piers, elliptical turnaround around sculptural island, grand manor & carriage house.
fn build_forecourt_turnaround(b: &mut OutdoorBuilder, stack: &mut FloorStack) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let gate_top = b.rng.chance(1, 2);
    let street_y = if gate_top { 2.0 } else { hf - 2.0 };
    let comp = BoxI {
        x0: 2,
        y0: if gate_top { 4 } else { 2 },
        x1: w - 2,
        y1: if gate_top { h - 2 } else { h - 4 },
    };

    let (d0, d1, d2, d3) = (
        b.rng.f32_range(-0.3, 0.3),
        b.rng.f32_range(-0.35, 0.35),
        b.rng.f32_range(-0.35, 0.35),
        b.rng.f32_range(-0.25, 0.25),
    );
    let street = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (0.0, street_y + d0),
            (wf * 0.33, street_y + d1),
            (wf * 0.67, street_y + d2),
            (wf, street_y + d3),
        ],
        380,
    );

    let rear_pad = if comp.h() >= 22 {
        5
    } else if comp.h() >= 18 {
        3
    } else {
        1
    };
    let carriage_w = (comp.w() / 6).clamp(4, 8);
    let carriage_h = 5.min(comp.h() / 3).max(4);
    let carriage_right = b.rng.chance(1, 2);
    let cx0 = if carriage_right {
        (w - 4 - carriage_w).min(comp.x1 - carriage_w - 2)
    } else {
        4.max(comp.x0 + 2)
    };
    let cy0 = if gate_top {
        comp.y0 + 2
    } else {
        comp.y1 - carriage_h - 2
    };
    let carriage = BoxI {
        x0: cx0,
        y0: cy0,
        x1: cx0 + carriage_w,
        y1: cy0 + carriage_h,
    };

    let manor_w = (comp.w() * 52 / 100)
        .clamp(10, 26)
        .min((comp.w() - carriage_w - 6).max(10));
    let manor_h = (comp.h() * 38 / 100).clamp(6, 15);
    let desired_mx0 = (w / 2 - manor_w / 2).clamp(
        4.max(comp.x0 + 2),
        (w - 4 - manor_w).min(comp.x1 - manor_w - 2),
    );
    let (my0, my1) = if gate_top {
        (comp.y1 - manor_h - rear_pad, comp.y1 - rear_pad)
    } else {
        (comp.y0 + rear_pad, comp.y0 + rear_pad + manor_h)
    };
    let carriage_y_clear = cy0 + carriage_h + 2 <= my0 || cy0 >= my1 + 2;
    let mx0 = if !carriage_y_clear && carriage_right && desired_mx0 + manor_w + 2 > cx0 {
        (cx0 - 2 - manor_w).max(comp.x0 + 2)
    } else if !carriage_y_clear && !carriage_right && desired_mx0 < cx0 + carriage_w + 2 {
        (cx0 + carriage_w + 2).min(comp.x1 - 2 - manor_w)
    } else {
        desired_mx0
    };
    let manor = BoxI {
        x0: mx0,
        y0: my0,
        x1: (mx0 + manor_w).min(w - 4),
        y1: my1,
    };

    let front_depth = if gate_top {
        (manor.y0 - comp.y0) as f32
    } else {
        (comp.y1 - manor.y1) as f32
    };
    let loop_w_cm = if front_depth < 9.0 { 210 } else { 280 };
    let half_loop = loop_w_cm as f32 / 200.0;
    let ry = ((front_depth - half_loop * 2.0 - 1.2) * 0.5).clamp(1.2, 2.8);
    let gate_x = (w / 2 + b.rng.range(-2, 3)).clamp(comp.x0 + 4, comp.x1 - 7);
    let gx_m = gate_x as f32 + 1.5;
    let island_off = (half_loop + ry + 0.55).min(front_depth * 0.52);
    let island_y = if gate_top {
        comp.y0 as f32 + island_off
    } else {
        comp.y1 as f32 - island_off
    };
    let rx = (comp.w() as f32 * 0.16).clamp(3.0, 5.8);
    let gate_edge_y = if gate_top {
        comp.y0 as f32
    } else {
        comp.y1 as f32
    };
    let loop_far_y = if gate_top {
        (island_y + ry + 0.4).min(manor.y0 as f32 - half_loop - 0.35)
    } else {
        (island_y - ry - 0.4).max(manor.y1 as f32 + half_loop + 0.35)
    };
    b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (gx_m, gate_edge_y),
            (gx_m - rx, island_y),
            (gx_m, loop_far_y),
            (gx_m + rx, island_y),
            (gx_m, gate_edge_y),
        ],
        loop_w_cm,
    );
    b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (gx_m, street_y),
            (
                gx_m,
                if gate_top {
                    comp.y0 as f32 + 0.5
                } else {
                    comp.y1 as f32 - 0.5
                },
            ),
        ],
        280,
    );
    b.add_poly(
        SurfaceFeatureKind::GardenBedZone,
        &ellipse_poly(
            gx_m,
            island_y,
            (rx - half_loop - 0.25).max(1.2),
            (ry - 0.35).clamp(0.85, 1.45),
            16,
        ),
        120,
    );
    b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (gx_m + if carriage_right { rx } else { -rx }, island_y),
            (
                (carriage.x0 + carriage.x1) as f32 * 0.5,
                if gate_top {
                    carriage.y1 as f32 + 1.0
                } else {
                    carriage.y0 as f32 - 1.0
                },
            ),
        ],
        240,
    );
    let manor_front_y = if gate_top {
        my0 as f32 - 0.6
    } else {
        my1 as f32 + 0.6
    };
    let walk_side_x = if carriage_right {
        mx0 as f32 - 1.2
    } else {
        manor.x1 as f32 + 1.2
    };
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[
            (gx_m, loop_far_y),
            ((manor.x0 + manor.x1) as f32 * 0.5, manor_front_y),
            (walk_side_x, (my0 + my1) as f32 * 0.5),
        ],
        170,
    );

    build_compound_wall(
        b,
        comp,
        if gate_top { gate_x } else { w / 2 - 1 },
        if gate_top { 3 } else { 2 },
        if !gate_top { gate_x } else { w / 2 - 1 },
        if !gate_top { 3 } else { 2 },
    );

    let pads = b.spawn_pads_m.clone();
    let entry = if gate_top {
        CardinalEntry::North
    } else {
        CardinalEntry::South
    };
    let home = generate_home_in_lot(
        w,
        h,
        manor.lot(),
        EconomicTier::LuxuryMansion,
        entry,
        true,
        rear_pad >= 5,
        true,
        &pads,
        b.rng.next(),
    );
    stack.absorb_home(b, home, true);

    build_carriage_house(b, carriage, gate_top);

    let has_pool = b
        .surfaces
        .iter()
        .any(|s| s.kind == SurfaceFeatureKind::PoolWater);
    let side_lawn = if carriage_right {
        RectM {
            x0: comp.x0 as f32 + 1.2,
            y0: manor.y0 as f32 - 1.0,
            x1: (manor.x0 as f32 - 1.5).max(comp.x0 as f32 + 1.2),
            y1: manor.y1 as f32 + 1.0,
        }
    } else {
        RectM {
            x0: manor.x1 as f32 + 1.5,
            y0: manor.y0 as f32 - 1.0,
            x1: (comp.x1 as f32 - 1.2).max(manor.x1 as f32 + 1.5),
            y1: manor.y1 as f32 + 1.0,
        }
    };
    if b.rng.chance(3, 4) {
        let prefer_spa_only = has_pool || b.rng.chance(1, 3);
        let _ = try_place_outdoor_pool_or_spa(
            b,
            side_lawn,
            OutdoorPoolSpaOpts {
                allow_pool: !has_pool,
                allow_spa: true,
                prefer_spa_only,
            },
        );
    }

    if b.rng.chance(2, 5) {
        let creek_band = if carriage_right {
            RectM {
                x0: manor.x1 as f32 + 1.8,
                y0: if gate_top {
                    carriage.y1 as f32 + 1.5
                } else {
                    comp.y0 as f32 + 1.2
                },
                x1: (comp.x1 as f32 - 1.0).max(manor.x1 as f32 + 1.8),
                y1: if gate_top {
                    comp.y1 as f32 - 1.2
                } else {
                    carriage.y0 as f32 - 1.5
                },
            }
        } else {
            RectM {
                x0: comp.x0 as f32 + 1.0,
                y0: if gate_top {
                    carriage.y1 as f32 + 1.5
                } else {
                    comp.y0 as f32 + 1.2
                },
                x1: (manor.x0 as f32 - 1.8).max(comp.x0 as f32 + 1.0),
                y1: if gate_top {
                    comp.y1 as f32 - 1.2
                } else {
                    carriage.y0 as f32 - 1.5
                },
            }
        };
        if !try_add_estate_creek(b, comp, creek_band, false) && rear_pad >= 4 {
            let rear_band = if gate_top {
                RectM {
                    x0: comp.x0 as f32 + 1.0,
                    y0: manor.y1 as f32 + 1.2,
                    x1: comp.x1 as f32 - 1.0,
                    y1: comp.y1 as f32 - 0.6,
                }
            } else {
                RectM {
                    x0: comp.x0 as f32 + 1.0,
                    y0: comp.y0 as f32 + 0.6,
                    x1: comp.x1 as f32 - 1.0,
                    y1: manor.y0 as f32 - 1.2,
                }
            };
            let _ = try_add_estate_creek(b, comp, rear_band, true);
        }
    }

    b.try_place_cm(
        PropKind::MediumFountain,
        OutdoorBuilder::cm(gx_m),
        OutdoorBuilder::cm(island_y),
        0,
        None,
        true,
    );
    b.place_vehicle_along_ribbon(PropKind::Car, street, 0.25, 0.2, 6);
    place_rot_m(b, PropKind::UtilityVan, gx_m - rx, island_y, 0, 360, true);
    dress_estate_grounds(
        b,
        comp.rect_m(1.0),
        &[
            (
                manor.x0 as f32 - 2.0,
                (manor.y0 + manor.y1) as f32 * 0.5,
                90,
            ),
            (
                manor.x1 as f32 + 2.0,
                (manor.y0 + manor.y1) as f32 * 0.5,
                90,
            ),
        ],
        (w * h / 110).clamp(6, 16),
    );
    if w >= 28 && h >= 22 {
        b.scatter_trees(PropKind::Gazebo, 1, 2.5, comp.rect_m(3.0));
    }
}

/// Topology 1: Quadrant-mirrored L-wing mansion, paved terrace court & detached stable/carriage wing.
fn build_lwing_compound(b: &mut OutdoorBuilder, stack: &mut FloorStack) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let flip_x = b.rng.chance(1, 2);
    let comp = BoxI {
        x0: 2,
        y0: 2,
        x1: w - 2,
        y1: h - 3,
    };

    let street_y = hf - 1.5;
    let street = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (0.0, street_y),
            (wf * 0.4, street_y + 0.25),
            (wf * 0.8, street_y - 0.2),
            (wf, street_y),
        ],
        340,
    );

    let bar_w = (comp.w() * 48 / 100)
        .clamp(9, 24)
        .min((comp.w() - 7).max(9));
    let bar_h = (comp.h() * 38 / 100).clamp(6, 12);
    let wing_w = (bar_w / 2).clamp(6, 12);
    let max_wing_h = (comp.y1 - 3) - (comp.y0 + 2 + bar_h);
    let has_wing = max_wing_h >= 5;
    let wing_h = if has_wing {
        (comp.h() * 32 / 100).clamp(5, 11).min(max_wing_h)
    } else {
        0
    };

    let (bar, wing, car) = if !flip_x {
        let bx0 = 4.max(comp.x0 + 2);
        let cx0 = (w - 9).min(comp.x1 - 7);
        (
            BoxI {
                x0: bx0,
                y0: comp.y0 + 2,
                x1: (bx0 + bar_w).min(w - 4),
                y1: comp.y0 + 2 + bar_h,
            },
            BoxI {
                x0: bx0,
                y0: comp.y0 + 2 + bar_h,
                x1: (bx0 + wing_w).min(w - 4),
                y1: comp.y0 + 2 + bar_h + wing_h,
            },
            BoxI {
                x0: cx0,
                y0: comp.y1 - 6,
                x1: cx0 + 5,
                y1: comp.y1 - 2,
            },
        )
    } else {
        let bx1 = (w - 4).min(comp.x1 - 2);
        let cx0 = 4.max(comp.x0 + 2);
        (
            BoxI {
                x0: (bx1 - bar_w).max(4),
                y0: comp.y0 + 2,
                x1: bx1,
                y1: comp.y0 + 2 + bar_h,
            },
            BoxI {
                x0: (bx1 - wing_w).max(4),
                y0: comp.y0 + 2 + bar_h,
                x1: bx1,
                y1: comp.y0 + 2 + bar_h + wing_h,
            },
            BoxI {
                x0: cx0,
                y0: comp.y1 - 6,
                x1: cx0 + 5,
                y1: comp.y1 - 2,
            },
        )
    };

    let gate_x = ((car.x0 + car.x1) / 2 - 1).clamp(comp.x0 + 3, comp.x1 - 5);
    let drive_pts = [
        (gate_x as f32 + 1.5, street_y),
        (gate_x as f32 + 1.5, comp.y1 as f32 - 0.5),
        ((car.x0 + car.x1) as f32 * 0.5, car.y0 as f32 - 1.4),
        (wf * 0.5, (bar.y1 as f32 + 2.5).min(comp.y1 as f32 - 4.0)),
    ];
    b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &drive_pts, 270);

    let (tx0, tx1) = if !flip_x {
        (
            if has_wing {
                wing.x1 as f32 + 0.5
            } else {
                bar.x0 as f32 + 0.5
            },
            (bar.x1 as f32 + 1.5).min(comp.x1 as f32 - 2.0),
        )
    } else {
        (
            (bar.x0 as f32 - 1.5).max(comp.x0 as f32 + 2.0),
            if has_wing {
                wing.x0 as f32 - 0.5
            } else {
                bar.x1 as f32 - 0.5
            },
        )
    };
    let (ty0, ty1) = if has_wing {
        (bar.y1 as f32 + 0.4, wing.y1 as f32 - 0.2)
    } else {
        (
            bar.y1 as f32 + 0.4,
            (bar.y1 as f32 + 3.6).min(comp.y1 as f32 - 1.2),
        )
    };
    if tx1 - tx0 >= 3.0 && ty1 - ty0 >= 3.0 {
        b.add_poly(
            SurfaceFeatureKind::ConcreteWalk,
            &rect_poly(tx0, ty0, tx1, ty1),
            85,
        );
    }
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[
            ((tx0 + tx1) * 0.5, (ty0 + ty1) * 0.5),
            (wf * 0.52, (ty1 + comp.y1 as f32) * 0.5),
            ((car.x0 + car.x1) as f32 * 0.5, car.y0 as f32 - 1.0),
        ],
        175,
    );

    build_compound_wall(b, comp, w / 2 - 1, 2, gate_x, 3);

    let pads = b.spawn_pads_m.clone();
    let bar_home = generate_home_in_lot(
        w,
        h,
        bar.lot(),
        EconomicTier::LuxuryMansion,
        CardinalEntry::South,
        true,
        false,
        true,
        &pads,
        b.rng.next(),
    );
    stack.absorb_home(b, bar_home, true);

    if has_wing {
        let wing_entry = if !flip_x {
            CardinalEntry::East
        } else {
            CardinalEntry::West
        };
        let wing_home = generate_annex_wing_in_lot(
            w,
            h,
            wing.lot(),
            EconomicTier::UpperMiddleClass,
            wing_entry,
            false,
            false,
            false,
            &pads,
            b.rng.next(),
        );
        FloorStack::absorb_ground(b, wing_home.ground);

        let seam_door_x = (wing.x0 + wing.x1) / 2;
        put_door(
            b,
            seam_door_x,
            bar.y1,
            EdgeSide::North,
            true,
            MaterialClass::SolidWood,
        );
        // Clear non-portal props placed flush against the internal bar<->wing seam doorway.
        b.props.retain(|p| {
            p.kind.is_vertical_portal()
                || !(p.covers(seam_door_x, bar.y1 - 1) || p.covers(seam_door_x, bar.y1))
        });
    }

    build_carriage_house(b, car, false);

    let has_pool = b
        .surfaces
        .iter()
        .any(|s| s.kind == SurfaceFeatureKind::PoolWater);
    let side_lawn = if !flip_x {
        RectM {
            x0: bar.x1 as f32 + 1.5,
            y0: comp.y0 as f32 + 1.2,
            x1: comp.x1 as f32 - 1.2,
            y1: (car.y0 as f32 - 1.2).max(comp.y0 as f32 + 1.2),
        }
    } else {
        RectM {
            x0: comp.x0 as f32 + 1.2,
            y0: comp.y0 as f32 + 1.2,
            x1: (bar.x0 as f32 - 1.5).max(comp.x0 as f32 + 1.2),
            y1: (car.y0 as f32 - 1.2).max(comp.y0 as f32 + 1.2),
        }
    };
    if b.rng.chance(2, 5) {
        let creek_band = if !flip_x {
            RectM {
                x0: (bar.x1 as f32 + 1.5).max(tx1 + 1.0),
                y0: comp.y0 as f32 + 0.8,
                x1: comp.x1 as f32 - 0.8,
                y1: (car.y0 as f32 - 1.2).max(comp.y0 as f32 + 0.8),
            }
        } else {
            RectM {
                x0: comp.x0 as f32 + 0.8,
                y0: comp.y0 as f32 + 0.8,
                x1: (bar.x0 as f32 - 1.5).min(tx0 - 1.0),
                y1: (car.y0 as f32 - 1.2).max(comp.y0 as f32 + 0.8),
            }
        };
        let _ = try_add_estate_creek(b, comp, creek_band, false);
    }

    if b.rng.chance(4, 5) {
        let prefer_spa_only = has_pool || b.rng.chance(1, 3);
        let _ = try_place_outdoor_pool_or_spa(
            b,
            side_lawn,
            OutdoorPoolSpaOpts {
                allow_pool: !has_pool,
                allow_spa: true,
                prefer_spa_only,
            },
        );
    }

    b.try_place_cm(
        PropKind::RoundTable,
        OutdoorBuilder::cm((tx0 + tx1) * 0.5),
        OutdoorBuilder::cm((ty0 + ty1) * 0.5),
        18,
        None,
        true,
    );
    b.place_vehicle_along_ribbon(PropKind::Car, street, 0.72, 0.2, 8);
    place_rot_m(
        b,
        PropKind::Car,
        (car.x0 + car.x1) as f32 * 0.5,
        (car.y0 as f32 - 2.2).max(2.5),
        -25,
        26,
        true,
    );
    dress_estate_grounds(
        b,
        comp.rect_m(1.0),
        &[
            ((tx0 + tx1) * 0.5, ty1 + 0.8, 0),
            (wf * 0.5, comp.y0 as f32 + 1.4, 0),
        ],
        (w * h / 105).clamp(6, 16),
    );
}

/// Topology 2: Twin opposed wing pavilions around a central cloistered garden courtyard, pool/spa & occasional stream.
fn build_cloistered_villa(b: &mut OutdoorBuilder, stack: &mut FloorStack) {
    let (w, h) = (b.w, b.h);
    let wf = w as f32;
    let comp = BoxI {
        x0: 2,
        y0: 4,
        x1: w - 2,
        y1: h - 2,
    };
    let street = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[(0.0, 2.0), (wf * 0.35, 2.3), (wf * 0.7, 1.75), (wf, 2.0)],
        350,
    );

    let west_w = (comp.w() * 28 / 100).clamp(7, 13);
    let east_w = (comp.w() * 24 / 100).clamp(6, 11);
    let v_pad = (comp.h() / 5).max(3);
    let west_villa = BoxI {
        x0: 4.max(comp.x0 + 2),
        y0: comp.y0 + v_pad,
        x1: 4.max(comp.x0 + 2) + west_w,
        y1: (comp.y1 - v_pad).max(comp.y0 + v_pad + 6),
    };
    let east_x1 = (w - 4).min(comp.x1 - 2);
    let east_pav = BoxI {
        x0: east_x1 - east_w,
        y0: comp.y0 + v_pad + 1,
        x1: east_x1,
        y1: (comp.y1 - v_pad - 1).max(comp.y0 + v_pad + 6),
    };

    let court = BoxI {
        x0: west_villa.x1,
        y0: west_villa.y0 + 1,
        x1: east_pav.x0,
        y1: west_villa.y1 - 1,
    };
    let mid_x = (court.x0 + court.x1) as f32 * 0.5;
    let mid_y = (court.y0 + court.y1) as f32 * 0.5;

    let gate_x = w / 2 - 1;
    let drive_end_y = (court.y0 as f32 - 1.4).max(comp.y0 as f32 + 1.4);
    b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (gate_x as f32 + 1.5, 2.0),
            (gate_x as f32 + 1.5, comp.y0 as f32 + 1.5),
            (mid_x, drive_end_y),
        ],
        280,
    );
    let cx0 = (court.x0 as f32 + 0.9).min(mid_x - 0.5);
    let cx1 = (court.x1 as f32 - 0.9).max(mid_x + 0.5);
    let cy0 = court.y0 as f32 + 0.9;
    let cy1 = (court.y1 as f32 - 0.9).max(cy0 + 1.5);
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[(mid_x, drive_end_y), (mid_x, cy0)],
        195,
    );
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[(cx0, cy0), (cx1, cy0), (cx1, cy1), (cx0, cy1), (cx0, cy0)],
        185,
    );
    if court.w() >= 4 {
        b.add_ribbon(
            SurfaceFeatureKind::StoneWalk,
            &[(court.x0 as f32, mid_y), (cx0 + 0.4, mid_y)],
            170,
        );
        b.add_ribbon(
            SurfaceFeatureKind::StoneWalk,
            &[(cx1 - 0.4, mid_y), (court.x1 as f32, mid_y)],
            170,
        );
    }

    build_compound_wall(b, comp, gate_x, 3, w / 2 - 1, 2);

    let pads = b.spawn_pads_m.clone();
    let west_home = generate_home_in_lot(
        w,
        h,
        west_villa.lot(),
        EconomicTier::LuxuryMansion,
        CardinalEntry::East,
        true,
        west_villa.x0 - comp.x0 >= 5,
        true,
        &pads,
        b.rng.next(),
    );
    stack.absorb_home(b, west_home, true);

    let east_home = generate_annex_wing_in_lot(
        w,
        h,
        east_pav.lot(),
        EconomicTier::UpperMiddleClass,
        CardinalEntry::West,
        true,
        false,
        true,
        &pads,
        b.rng.next(),
    );
    stack.absorb_home(b, east_home, false);

    let has_pool = b
        .surfaces
        .iter()
        .any(|s| s.kind == SurfaceFeatureKind::PoolWater);
    let south_garden = RectM {
        x0: comp.x0 as f32 + 1.0,
        y0: (west_villa.y1 as f32 + 0.8).max(court.y1 as f32 + 0.8),
        x1: comp.x1 as f32 - 1.0,
        y1: comp.y1 as f32 - 0.8,
    };
    let cloister_roll = b.rng.range(0, 100);
    if cloister_roll < 42 && !has_pool {
        let court_inner = RectM {
            x0: cx0 + 1.0,
            y0: cy0 + 1.0,
            x1: (cx1 - 1.0).max(cx0 + 1.0),
            y1: (cy1 - 1.0).max(cy0 + 1.0),
        };
        if !try_place_outdoor_pool_or_spa(
            b,
            court_inner,
            OutdoorPoolSpaOpts {
                allow_pool: true,
                allow_spa: true,
                prefer_spa_only: false,
            },
        ) {
            if cy1 - cy0 >= 1.2 {
                b.add_ribbon(
                    SurfaceFeatureKind::StoneWalk,
                    &[(mid_x, cy0), (mid_x, cy1)],
                    175,
                );
            }
            let _ = try_place_outdoor_pool_or_spa(
                b,
                south_garden,
                OutdoorPoolSpaOpts {
                    allow_pool: true,
                    allow_spa: true,
                    prefer_spa_only: false,
                },
            );
        }
    } else if cloister_roll < 82 {
        if cy1 - cy0 >= 1.2 {
            b.add_ribbon(
                SurfaceFeatureKind::StoneWalk,
                &[(mid_x, cy0), (mid_x, cy1)],
                175,
            );
        }
        if court.w() >= 6 && court.h() >= 6 {
            let basin_rx = ((court.w() as f32) * 0.25).clamp(1.3, 4.0);
            let basin_ry = ((court.h() as f32) * 0.22).clamp(1.2, 3.2);
            b.add_poly(
                SurfaceFeatureKind::StoneWalk,
                &ellipse_poly(mid_x, mid_y, basin_rx + 0.75, basin_ry + 0.75, 18),
                45,
            );
            b.add_poly(
                SurfaceFeatureKind::GardenBedZone,
                &ellipse_poly(mid_x, mid_y, basin_rx, basin_ry, 18),
                120,
            );
            let _ = b.try_place_cm(
                PropKind::MediumFountain,
                OutdoorBuilder::cm(mid_x),
                OutdoorBuilder::cm(mid_y),
                0,
                None,
                true,
            );
        }
        if b.rng.chance(1, 2) {
            let _ = try_add_estate_creek(b, comp, south_garden, true);
        }
        let allow_pool = !has_pool && b.rng.chance(1, 2);
        let prefer_spa_only = has_pool || b.rng.chance(1, 2);
        let _ = try_place_outdoor_pool_or_spa(
            b,
            south_garden,
            OutdoorPoolSpaOpts {
                allow_pool,
                allow_spa: true,
                prefer_spa_only,
            },
        );
    } else {
        if cy1 - cy0 >= 1.2 {
            b.add_ribbon(
                SurfaceFeatureKind::StoneWalk,
                &[(mid_x, cy0), (mid_x, cy1)],
                175,
            );
        }
        let _ = try_add_estate_creek(b, comp, south_garden, true);
    }

    if court.w() >= 6 {
        for &(px, py) in &[
            (court.x0 as f32 + 1.0, court.y0 as f32 + 1.0),
            (court.x1 as f32 - 1.0, court.y0 as f32 + 1.0),
            (court.x0 as f32 + 1.0, court.y1 as f32 - 1.0),
            (court.x1 as f32 - 1.0, court.y1 as f32 - 1.0),
        ] {
            b.try_place_cm(
                PropKind::StructuralPillar,
                OutdoorBuilder::cm(px),
                OutdoorBuilder::cm(py),
                0,
                None,
                true,
            );
        }
    }
    b.place_vehicle_along_ribbon(PropKind::UtilityVan, street, 0.68, -0.2, 6);
    b.try_place_cm(
        PropKind::Car,
        OutdoorBuilder::cm(gate_x as f32 + 4.2),
        OutdoorBuilder::cm(comp.y0 as f32 + 1.5),
        85,
        None,
        true,
    );
    dress_estate_grounds(
        b,
        comp.rect_m(0.8),
        &[
            (mid_x, court.y0 as f32 - 0.8, 0),
            (mid_x, court.y1 as f32 + 0.8, 0),
        ],
        (w * h / 100).clamp(6, 18),
    );
}
