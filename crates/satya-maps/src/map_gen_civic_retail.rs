//! # Specialized Civic & Commercial Facility Generators (`map_gen_civic_retail`)
//!
//! Procedural vector generators for three signature civic and retail presets:
//! - [`SectorPreset::Hospital`]: multi-floor (`1F`/`2F`) hospital campus with exterior ambulance bay,
//!   canopy sidewalk, triage waiting room, patient wards (`HospitalBed`, `PrivacyCurtain`), surgical
//!   suite (`SurgicalTable`), diagnostic imaging (`ImagingScanner`), and aligned [`PropKind::Stairwell`].
//! - [`SectorPreset::ShoppingPlaza`]: curving landscaped parking lot, shaded outdoor concrete
//!   promenade with patio seating & [`PropKind::MallKiosk`], and `3–6` varied store units.
//! - [`SectorPreset::ShoppingMall`]: multi-floor (`1F`/`2F`) enclosed galleria with concourse kiosks,
//!   [`PropKind::PlanterTable`] atrium greenery, department anchors, boutiques, food court, and a `2F`
//!   [`SurfaceFeatureKind::BridgeDeck`] skybridge overlook.

use satya_schema::{
    EdgeSide, MapFloorLevel, MapPropPlacement, MapSurfaceFeature, MapWallEdge, MaterialClass,
    PropKind, StoreArchetype, SurfaceFeatureKind, WallEdgeKind,
};

use crate::map_dressing::{
    PropObstruction, effective_obstruction, make_prop, make_prop_at_cm, prop_spec,
};
use crate::map_gen_commercial::generate_store_unit;
use crate::map_gen_residential::{
    CardinalEntry, LotBox, floor_lot_connected, trim_floor_props_to_connected,
};
use crate::map_outdoor_gen::{OutdoorBuilder, RectM};
use crate::map_surface_geom::topmost_surface_at_m;
use crate::map_surface_queries::prune_occluded_surfaces;
use crate::tactical_map::{ProceduralSector, SectorPreset};

fn safe_x_span(w: i32) -> (i32, i32) {
    let pad = if w >= 16 {
        4
    } else if w >= 12 {
        3
    } else {
        2
    };
    (pad, (w - pad).max(pad + 4).min(w - 1))
}

fn enclose_campus_shell(
    b: &mut OutdoorBuilder,
    lot: LotBox,
    wall_mat: MaterialClass,
    win_step: i32,
) {
    for x in lot.x0..lot.x1 {
        let win = win_step > 0 && (x - lot.x0) % win_step == 1 && x + 1 < lot.x1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        b.put_edge(x, lot.y0, EdgeSide::North, k, m);
        b.put_edge(x, lot.y1, EdgeSide::North, k, m);
    }
    for y in lot.y0..lot.y1 {
        let win = win_step > 0 && (y - lot.y0) % win_step == 1 && y + 1 < lot.y1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        b.put_edge(lot.x0, y, EdgeSide::West, k, m);
        b.put_edge(lot.x1, y, EdgeSide::West, k, m);
    }
}

fn edge_upsert(
    edges: &mut Vec<MapWallEdge>,
    x: i32,
    y: i32,
    side: EdgeSide,
    kind: WallEdgeKind,
    material: MaterialClass,
) {
    let e = MapWallEdge {
        x,
        y,
        side,
        kind,
        material,
    };
    match edges
        .iter_mut()
        .find(|o| o.x == x && o.y == y && o.side == side)
    {
        Some(slot) => *slot = e,
        None => edges.push(e),
    }
}

fn enclose_upper_shell(
    edges: &mut Vec<MapWallEdge>,
    lot: LotBox,
    wall_mat: MaterialClass,
    win_step: i32,
) {
    for x in lot.x0..lot.x1 {
        let win = win_step > 0 && (x - lot.x0) % win_step == 1 && x + 1 < lot.x1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        edge_upsert(edges, x, lot.y0, EdgeSide::North, k, m);
        edge_upsert(edges, x, lot.y1, EdgeSide::North, k, m);
    }
    for y in lot.y0..lot.y1 {
        let win = win_step > 0 && (y - lot.y0) % win_step == 1 && y + 1 < lot.y1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        edge_upsert(edges, lot.x0, y, EdgeSide::West, k, m);
        edge_upsert(edges, lot.x1, y, EdgeSide::West, k, m);
    }
}

fn h_partition_on_edges(
    edges: &mut Vec<MapWallEdge>,
    y: i32,
    x0: i32,
    x1: i32,
    doors: &[i32],
    wall_mat: MaterialClass,
) {
    for x in x0..x1 {
        if doors.contains(&x) {
            edge_upsert(
                edges,
                x,
                y,
                EdgeSide::North,
                WallEdgeKind::DoorOpen,
                MaterialClass::SheetSteel,
            );
        } else {
            edge_upsert(edges, x, y, EdgeSide::North, WallEdgeKind::Wall, wall_mat);
        }
    }
}

fn v_partition_on_edges(
    edges: &mut Vec<MapWallEdge>,
    x: i32,
    y0: i32,
    y1: i32,
    doors: &[i32],
    wall_mat: MaterialClass,
) {
    for y in y0..y1 {
        if doors.contains(&y) {
            edge_upsert(
                edges,
                x,
                y,
                EdgeSide::West,
                WallEdgeKind::DoorClosed,
                wall_mat,
            );
        } else {
            edge_upsert(edges, x, y, EdgeSide::West, WallEdgeKind::Wall, wall_mat);
        }
    }
}

fn hospital_blocks_doorway(edges: &[MapWallEdge], cand: &MapPropPlacement) -> bool {
    edges.iter().any(|e| {
        matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen)
            && match e.side {
                EdgeSide::West => cand.covers(e.x - 1, e.y) || cand.covers(e.x, e.y),
                EdgeSide::North => cand.covers(e.x, e.y - 1) || cand.covers(e.x, e.y),
            }
    })
}

/// Checks AABB wall-crossing, doorway clearance, + solid prop overlap inside `bay`.
fn place_indoor_checked(
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    bay: LotBox,
    cand: MapPropPlacement,
) -> bool {
    let outside = cand.x < bay.x0
        || cand.y < bay.y0
        || cand.x + cand.w_cells > bay.x1
        || cand.y + cand.h_cells > bay.y1;
    if outside {
        return false;
    }
    if prop_spec(cand.kind).floor_layer {
        let wall_touch = crate::tactical_map_geom::prop_crosses_wall_edge(&cand, edges);
        let door_block = !cand.kind.is_vertical_portal()
            && !cand.kind.is_tabletop_clutter()
            && hospital_blocks_doorway(edges, &cand);
        if wall_touch || door_block {
            return false;
        }
        if !cand.kind.is_tabletop_clutter()
            && !cand.kind.is_vertical_portal()
            && props.iter().any(|e| {
                (!prop_spec(e.kind).floor_layer || e.kind.is_vertical_portal())
                    && crate::tactical_map_geom::props_overlap_continuous(&cand, e)
            })
        {
            return false;
        }
        props.push(cand);
        return true;
    }
    if hospital_blocks_doorway(edges, &cand)
        || crate::tactical_map_geom::prop_crosses_wall_edge(&cand, edges)
    {
        return false;
    }
    if props.iter().any(|e| {
        (!prop_spec(e.kind).floor_layer || e.kind.is_vertical_portal())
            && crate::tactical_map_geom::props_overlap_continuous(&cand, e)
    }) {
        return false;
    }
    props.push(cand);
    true
}

fn hospital_poly_surface(kind: SurfaceFeatureKind, bx: LotBox, seed: u32) -> MapSurfaceFeature {
    bx.polygon_feature(kind, 0, seed)
}

/// Splits `[start, end)` into sub-spans of width `>= min_w` (`<= 4` rooms).
fn split_wing_spans(start: i32, end: i32, target_w: i32, min_w: i32) -> Vec<(i32, i32)> {
    let total = end - start;
    if total < min_w * 2 {
        return vec![(start, end)];
    }
    let count = (total / target_w.max(min_w)).clamp(2, 4);
    let mut xs = Vec::with_capacity(count as usize + 1);
    xs.push(start);
    for i in 1..count {
        xs.push(start + (total * i) / count);
    }
    xs.push(end);
    xs.windows(2).map(|w| (w[0], w[1])).collect()
}

/// Carves a `2x2` or `3x2` partitioned en-suite bathroom inside a corner of `room` opposite `door_top`
/// (`true` when the room's corridor doorway is along its South border `room.y1`). Returns the bath
/// rectangle when carved so surrounding flooring avoids covering `CeramicTileFloor`.
fn add_ensuite_bathroom(
    room: LotBox,
    corridor_on_south: bool,
    edges: &mut Vec<MapWallEdge>,
    props: &mut Vec<MapPropPlacement>,
    surfaces: &mut Vec<MapSurfaceFeature>,
    s: u32,
) -> Option<LotBox> {
    if room.w() < 5 || room.h() < 4 {
        return None;
    }
    let bath_w = if room.w() >= 6 && room.h() >= 5 { 3 } else { 2 };
    let bath_h = 2;
    let bx0 = room.x1 - bath_w;
    let bx1 = room.x1;
    let (by0, by1, h_wall_y, door_x) = if corridor_on_south {
        (room.y0, room.y0 + bath_h, room.y0 + bath_h, bx0)
    } else {
        (room.y1 - bath_h, room.y1, room.y1 - bath_h, bx0)
    };
    let bath = LotBox::new(bx0, by0, bx1, by1);
    v_partition_on_edges(edges, bx0, by0, by1, &[], MaterialClass::Drywall);
    h_partition_on_edges(edges, h_wall_y, bx0, bx1, &[door_x], MaterialClass::Drywall);
    surfaces.push(hospital_poly_surface(
        SurfaceFeatureKind::CeramicTileFloor,
        bath,
        s.wrapping_add(5),
    ));

    // Place fixtures strictly away from the bathroom doorway threshold `(door_x, by0..by1)`.
    let fixture_y = if corridor_on_south { by0 } else { by1 - 1 };
    let sink_x = bx1 - 1;
    let toilet_x = if bath_w >= 3 { bx1 - 2 } else { bx0 };
    let _ = place_indoor_checked(
        edges,
        props,
        bath,
        make_prop(
            PropKind::Toilet,
            toilet_x,
            fixture_y,
            if corridor_on_south { 0 } else { 180 },
            None,
            s + 1,
        ),
    );
    let _ = place_indoor_checked(
        edges,
        props,
        bath,
        make_prop(PropKind::BathroomSink, sink_x, fixture_y, 90, None, s + 2),
    );
    if bath_w >= 3 {
        let _ = place_indoor_checked(
            edges,
            props,
            bath,
            make_prop(PropKind::ShowerStall, bx0, fixture_y, 0, None, s + 3),
        );
    }
    let bin_y = if corridor_on_south { by1 - 1 } else { by0 };
    let _ = place_indoor_checked(
        edges,
        props,
        bath,
        make_prop(PropKind::TrashCan, bx1 - 1, bin_y, 0, None, s + 4),
    );
    Some(bath)
}

/// Dresses an individual hospital patient room (`HospitalBed` / `Cot` / `Gurney`, centered or gently
/// angled `±8°..±14°`, `TelemetryMonitor`, visitor seating, counter/sink, biohazard & linen bins).
#[allow(clippy::too_many_arguments)]
fn furnish_patient_room(
    room: LotBox,
    idx: usize,
    corridor_on_south: bool,
    is_upper: bool,
    edges: &mut Vec<MapWallEdge>,
    props: &mut Vec<MapPropPlacement>,
    surfaces: &mut Vec<MapSurfaceFeature>,
    s: u32,
) {
    if room.w() <= 0 || room.h() <= 0 {
        return;
    }
    if let Some(bath) =
        add_ensuite_bathroom(room, corridor_on_south, edges, props, surfaces, s + 70)
    {
        if bath.x0 > room.x0 {
            surfaces.push(hospital_poly_surface(
                SurfaceFeatureKind::ClinicalVinylFloor,
                LotBox::new(room.x0, room.y0, bath.x0, room.y1),
                s.wrapping_add(31),
            ));
        }
        let rem = if corridor_on_south {
            LotBox::new(bath.x0, bath.y1, room.x1, room.y1)
        } else {
            LotBox::new(bath.x0, room.y0, room.x1, bath.y0)
        };
        if rem.w() > 0 && rem.h() > 0 {
            surfaces.push(hospital_poly_surface(
                SurfaceFeatureKind::ClinicalVinylFloor,
                rem,
                s.wrapping_add(33),
            ));
        }
    } else {
        surfaces.push(hospital_poly_surface(
            SurfaceFeatureKind::ClinicalVinylFloor,
            room,
            s.wrapping_add(31),
        ));
    }

    let bed_kind = match (is_upper, idx % 4) {
        (false, 2) => PropKind::Gurney,
        (_, 3) => PropKind::Cot,
        _ => PropKind::HospitalBed,
    };
    let angle_offset = match (idx + usize::from(is_upper)) % 3 {
        1 => 10,
        2 => -11,
        _ => 0,
    };
    let cx_cm = (room.x0 + (room.w() / 2).max(1)) * 100 + 25;
    let cy_cm = if corridor_on_south {
        (room.y0 + ((room.h() * 2) / 3).max(1)) * 100 - 30
    } else {
        (room.y0 + (room.h() / 3).max(1)) * 100 + 30
    };
    let bx_lo = room.x0 * 100 + 110;
    let bx_hi = (room.x1 * 100 - 110).max(bx_lo);
    let by_lo = room.y0 * 100 + 85;
    let by_hi = (room.y1 * 100 - 85).max(by_lo);
    let centered_bed = make_prop_at_cm(
        bed_kind,
        cx_cm.clamp(bx_lo, bx_hi),
        cy_cm.clamp(by_lo, by_hi),
        angle_offset,
        None,
        s + 10,
    );
    let fallback_y = if corridor_on_south {
        room.y1 - 2
    } else {
        room.y0 + 1
    };
    let bed_placed = if place_indoor_checked(edges, props, room, centered_bed) {
        Some(centered_bed)
    } else {
        let ax = make_prop(bed_kind, room.x0, room.y0, 90 + angle_offset, None, s + 11);
        if place_indoor_checked(edges, props, room, ax) {
            Some(ax)
        } else {
            let b2 = make_prop(bed_kind, room.x0, fallback_y, 0, None, s + 12);
            place_indoor_checked(edges, props, room, b2).then_some(b2)
        }
    };

    if let Some(bed) = bed_placed {
        let mon_x = (bed.x - 1).clamp(room.x0, room.x1 - 1);
        let mon_y = bed.y.clamp(room.y0, room.y1 - 1);
        let _ = place_indoor_checked(
            edges,
            props,
            room,
            make_prop(PropKind::TelemetryMonitor, mon_x, mon_y, 0, None, s + 15),
        );
        if room.w() >= 5 {
            let curt_y = if corridor_on_south {
                room.y1 - 2
            } else {
                room.y0
            };
            let curt_alt_y = if corridor_on_south {
                room.y0 + 1
            } else {
                room.y1 - 3
            };
            for cand in [
                make_prop(
                    PropKind::PrivacyCurtain,
                    (bed.x + bed.w_cells).min(room.x1 - 1),
                    curt_y.clamp(room.y0, (room.y1 - 2).max(room.y0)),
                    90,
                    Some(2),
                    s + 16,
                ),
                make_prop(
                    PropKind::PrivacyCurtain,
                    (room.x0 + 1).min(room.x1 - 2),
                    curt_alt_y.clamp(room.y0, room.y1 - 1),
                    0,
                    Some(2),
                    s + 17,
                ),
            ] {
                if place_indoor_checked(edges, props, room, cand) {
                    break;
                }
            }
        }
    }

    // Staff medical counter & scrub sink, visitor chair/loveseat, waste & linen bins.
    let counter_y = if corridor_on_south {
        room.y1 - 1
    } else {
        room.y0
    };
    let seat_y = if corridor_on_south {
        room.y0 + 2
    } else {
        room.y1 - 2
    };
    let seat_kind = if room.w() >= 6 && idx.is_multiple_of(2) {
        PropKind::Loveseat
    } else {
        PropKind::ReclinerChair
    };
    for cand in [
        make_prop(
            PropKind::MedicalCounter,
            room.x0,
            room.y0,
            0,
            Some(2),
            s + 20,
        ),
        make_prop(
            PropKind::MedicalCounter,
            room.x0,
            counter_y,
            0,
            None,
            s + 21,
        ),
    ] {
        if place_indoor_checked(edges, props, room, cand) {
            break;
        }
    }
    let _ = place_indoor_checked(
        edges,
        props,
        room,
        make_prop(
            seat_kind,
            room.x0,
            seat_y.clamp(room.y0, room.y1 - 1),
            90,
            None,
            s + 22,
        ),
    );
    let _ = place_indoor_checked(
        edges,
        props,
        room,
        make_prop(
            PropKind::BiohazardBin,
            room.x1 - 1,
            counter_y,
            0,
            None,
            s + 23,
        ),
    );
    let _ = place_indoor_checked(
        edges,
        props,
        room,
        make_prop(
            PropKind::TrashCan,
            (room.x0 + 1).min(room.x1 - 1),
            room.y0,
            0,
            None,
            s + 24,
        ),
    );
    for cand in [
        make_prop(
            PropKind::LinenCart,
            room.x0,
            (room.y0 + 1).min(room.y1 - 1),
            0,
            None,
            s + 25,
        ),
        make_prop(
            PropKind::LinenCart,
            (room.x1 - 2).max(room.x0),
            if corridor_on_south {
                room.y1 - 1
            } else {
                room.y0
            },
            0,
            None,
            s + 26,
        ),
        make_prop(PropKind::LinenCart, room.x0, room.y0, 90, None, s + 27),
    ] {
        if place_indoor_checked(edges, props, room, cand) {
            break;
        }
    }
}

fn hospital_corridor_bounds(lot: LotBox) -> (i32, i32, i32, i32, i32, i32) {
    let bw = lot.w();
    let bh = lot.h();
    let v_pad = if bh >= 8 { 3 } else { 2.min((bh - 1).max(0) / 2) };
    let corr_h = if bh >= 13 { 3 } else { 2.min((bh - 2 * v_pad).max(1)) };
    let mid_lo = lot.y0 + v_pad;
    let mid_hi = (lot.y1 - v_pad - 1).max(mid_lo);
    let mid_y = (lot.y0 + bh / 2).clamp(mid_lo, mid_hi);
    let cy0_lo = lot.y0 + v_pad;
    let cy0_hi = (lot.y1 - v_pad - corr_h).max(cy0_lo);
    let corr_y0 = (mid_y - corr_h / 2).clamp(cy0_lo, cy0_hi);
    let corr_y1 = (corr_y0 + corr_h)
        .min(lot.y1 - v_pad)
        .max(corr_y0 + 1)
        .min(lot.y1);
    let wing_pad = if bw >= 12 { 4 } else { (bw / 3).max(1) };
    let tx_span = if bw >= 12 { 4 } else { (bw - 2 * wing_pad).max(1) };
    let tx0_lo = lot.x0 + wing_pad;
    let tx0_hi = (lot.x1 - wing_pad - tx_span).max(tx0_lo);
    let tx0 = (lot.x0 + bw / 2 - tx_span / 2).clamp(tx0_lo, tx0_hi);
    let tx1 = (tx0 + tx_span)
        .min(lot.x1 - wing_pad)
        .max(tx0 + 1)
        .min(lot.x1);
    (corr_y0, corr_y1, tx0, tx1, bw, bh)
}

fn hospital_core_x_span(lot: LotBox, tx0: i32) -> (i32, i32) {
    let bw = lot.w();
    let pad = if bw >= 10 { 3 } else { (bw / 3).max(1) };
    let nx_a_lo = lot.x0 + pad;
    let nx_a_hi = tx0.max(nx_a_lo).min((lot.x1 - pad - 1).max(nx_a_lo));
    let nx_a = (lot.x0 + bw / 3).clamp(nx_a_lo, nx_a_hi);
    let min_core = if bw >= 10 { 4 } else { 2 };
    let nx_b = (nx_a + (bw / 3).max(min_core))
        .min(lot.x1 - pad)
        .max(nx_a + 1)
        .min(lot.x1);
    (nx_a, nx_b)
}

#[allow(clippy::too_many_arguments)]
fn fill_hospital_floor(
    lot: LotBox,
    edges: &mut Vec<MapWallEdge>,
    props: &mut Vec<MapPropPlacement>,
    surfaces: &mut Vec<MapSurfaceFeature>,
    stair_xy: (i32, i32),
    elev_pass_xy: (i32, i32),
    elev_serv_xy: (i32, i32),
    is_upper: bool,
    seed: u64,
) {
    let (corr_y0, corr_y1, tx0, tx1, _bw, _bh) = hospital_corridor_bounds(lot);
    let s = seed as u32;

    // North-Central Stairwell & Elevator Lobby bay `[nx_a, nx_b) × [lot.y0, corr_y0)`.
    let (nx_a, nx_b) = hospital_core_x_span(lot, tx0);

    // Main E-W corridor floor finish (Terrazzo), while triage/waiting keeps InteriorFloor exposed.
    surfaces.push(hospital_poly_surface(
        SurfaceFeatureKind::TerrazzoFloor,
        LotBox::new(lot.x0, corr_y0, lot.x1, corr_y1),
        s.wrapping_add(101),
    ));

    // North-West triage/waiting (1 bay) & North-East inpatient ward spans.
    let nw_room = LotBox::new(lot.x0, lot.y0, nx_a, corr_y0);
    let core_bay = LotBox::new(nx_a, lot.y0, nx_b, corr_y0);
    let ne_spans = split_wing_spans(nx_b, lot.x1, 5, 4);
    let mut north_doors = vec![lot.x0 + 1, nx_a + (core_bay.w() / 2).max(1)];
    for &(sx0, sx1) in &ne_spans {
        north_doors.push((sx0 + 1).min(sx1 - 1));
    }
    h_partition_on_edges(
        edges,
        corr_y0,
        lot.x0,
        lot.x1,
        &north_doors,
        MaterialClass::Drywall,
    );
    v_partition_on_edges(edges, nx_a, lot.y0, corr_y0, &[], MaterialClass::Drywall);
    v_partition_on_edges(
        edges,
        nx_b,
        lot.y0,
        corr_y0,
        &[corr_y0 - 1],
        MaterialClass::Drywall,
    );
    for &(sx0, _sx1) in ne_spans.iter().skip(1) {
        v_partition_on_edges(edges, sx0, lot.y0, corr_y0, &[], MaterialClass::Drywall);
    }

    // South-West surgical suite spans & South-East imaging + patient spans.
    let sw_spans = split_wing_spans(lot.x0, tx0, 5, 4);
    let se_spans = split_wing_spans(tx1, lot.x1, 5, 4);
    let mut sw_doors = Vec::new();
    for &(sx0, sx1) in &sw_spans {
        sw_doors.push((sx0 + 1).min(sx1 - 1));
    }
    let mut se_doors = Vec::new();
    for &(sx0, sx1) in &se_spans {
        se_doors.push((sx0 + 1).min(sx1 - 1));
    }
    h_partition_on_edges(
        edges,
        corr_y1,
        lot.x0,
        tx0,
        &sw_doors,
        MaterialClass::Drywall,
    );
    h_partition_on_edges(
        edges,
        corr_y1,
        tx1,
        lot.x1,
        &se_doors,
        MaterialClass::Drywall,
    );
    v_partition_on_edges(
        edges,
        tx0,
        corr_y1,
        lot.y1,
        &[corr_y1],
        MaterialClass::Drywall,
    );
    v_partition_on_edges(
        edges,
        tx1,
        corr_y1,
        lot.y1,
        &[corr_y1],
        MaterialClass::Drywall,
    );
    for &(sx0, _) in sw_spans.iter().skip(1) {
        v_partition_on_edges(edges, sx0, corr_y1, lot.y1, &[], MaterialClass::Drywall);
    }
    for &(sx0, _) in se_spans.iter().skip(1) {
        v_partition_on_edges(edges, sx0, corr_y1, lot.y1, &[], MaterialClass::Drywall);
    }

    // Aligned vertical circulation (`Stairwell`, passenger `ElevatorCar`, hospital `ServiceElevator`).
    props.push(make_prop(
        PropKind::Stairwell,
        stair_xy.0,
        stair_xy.1,
        0,
        None,
        s,
    ));
    let elev_candidates = [
        make_prop(
            PropKind::ElevatorCar,
            elev_pass_xy.0,
            elev_pass_xy.1,
            0,
            None,
            s + 1,
        ),
        make_prop(PropKind::ElevatorCar, nx_a, corr_y0, 0, None, s + 1),
        make_prop(
            PropKind::ElevatorCar,
            (nx_b - 2).max(nx_a),
            corr_y0,
            0,
            None,
            s + 1,
        ),
    ];
    for cand in elev_candidates {
        if !crate::tactical_map_geom::prop_crosses_wall_edge(&cand, edges) {
            props.push(cand);
            break;
        }
    }
    props.push(make_prop(
        PropKind::ServiceElevator,
        elev_serv_xy.0,
        elev_serv_xy.1,
        0,
        None,
        s + 2,
    ));

    // North-West triage / reception room keeps InteriorFloor exposed for texture variety.
    if is_upper {
        let _ = place_indoor_checked(
            edges,
            props,
            nw_room,
            make_prop(
                PropKind::HospitalBed,
                nw_room.x0,
                nw_room.y0,
                0,
                None,
                s + 3,
            ),
        );
        let _ = place_indoor_checked(
            edges,
            props,
            nw_room,
            make_prop(
                PropKind::TelemetryMonitor,
                (nw_room.x0 + 2).min(nw_room.x1 - 1),
                nw_room.y0,
                0,
                None,
                s + 4,
            ),
        );
        let _ = place_indoor_checked(
            edges,
            props,
            nw_room,
            make_prop(
                PropKind::ReclinerChair,
                nw_room.x1 - 1,
                nw_room.y0,
                90,
                None,
                s + 5,
            ),
        );
        let _ = place_indoor_checked(
            edges,
            props,
            nw_room,
            make_prop(
                PropKind::BiohazardBin,
                nw_room.x0,
                nw_room.y1 - 1,
                0,
                None,
                s + 6,
            ),
        );
    } else {
        let _ = place_indoor_checked(
            edges,
            props,
            nw_room,
            make_prop(
                PropKind::CheckoutCounter,
                nw_room.x0 + 1,
                nw_room.y0 + 1,
                0,
                Some(2),
                s + 3,
            ),
        );
        let _ = place_indoor_checked(
            edges,
            props,
            nw_room,
            make_prop(PropKind::Sofa, nw_room.x0, nw_room.y0, 0, None, s + 4),
        );
        let _ = place_indoor_checked(
            edges,
            props,
            nw_room,
            make_prop(
                PropKind::ReclinerChair,
                nw_room.x1 - 1,
                nw_room.y0,
                180,
                None,
                s + 5,
            ),
        );
        let _ = place_indoor_checked(
            edges,
            props,
            nw_room,
            make_prop(
                PropKind::WaterCooler,
                nw_room.x0,
                nw_room.y1 - 1,
                0,
                None,
                s + 6,
            ),
        );
    }

    // North-East partitioned inpatient patient rooms + en-suite bathrooms.
    for (idx, &(sx0, sx1)) in ne_spans.iter().enumerate() {
        furnish_patient_room(
            LotBox::new(sx0, lot.y0, sx1, corr_y0),
            idx,
            true,
            is_upper,
            edges,
            props,
            surfaces,
            s + 100 + idx as u32 * 90,
        );
    }

    // South-West surgical theater(s) + secondary recovery bay.
    for (idx, &(sx0, sx1)) in sw_spans.iter().enumerate() {
        let bay = LotBox::new(sx0, corr_y1, sx1, lot.y1);
        if idx == 0 {
            let sx = bay.x0 + (bay.w() / 2 - 1).max(0);
            let sy = bay.y0 + (bay.h() / 2 - 1).max(0);
            let _ = place_indoor_checked(
                edges,
                props,
                bay,
                make_prop(PropKind::SurgicalTable, sx, sy, 0, None, s + 40),
            );
            let _ = place_indoor_checked(
                edges,
                props,
                bay,
                make_prop(
                    PropKind::TelemetryMonitor,
                    (sx + 2).min(bay.x1 - 1),
                    sy,
                    0,
                    None,
                    s + 41,
                ),
            );
            let _ = place_indoor_checked(
                edges,
                props,
                bay,
                make_prop(
                    PropKind::MedicalCounter,
                    bay.x0,
                    bay.y1 - 1,
                    0,
                    Some(bay.w().min(2)),
                    s + 42,
                ),
            );
            let _ = place_indoor_checked(
                edges,
                props,
                bay,
                make_prop(
                    PropKind::BiohazardBin,
                    bay.x1 - 1,
                    bay.y1 - 1,
                    0,
                    None,
                    s + 43,
                ),
            );
            for cand in [
                make_prop(
                    PropKind::PrivacyCurtain,
                    (sx + 2).min(bay.x1 - 1),
                    sy,
                    90,
                    Some(2),
                    s + 44,
                ),
                make_prop(
                    PropKind::PrivacyCurtain,
                    bay.x0,
                    bay.y0 + 1,
                    90,
                    Some(2),
                    s + 45,
                ),
            ] {
                if place_indoor_checked(edges, props, bay, cand) {
                    break;
                }
            }
        } else {
            furnish_patient_room(
                bay,
                idx + 2,
                false,
                is_upper,
                edges,
                props,
                surfaces,
                s + 300 + idx as u32 * 90,
            );
        }
    }

    // South-East diagnostic imaging suite + additional patient room(s).
    for (idx, &(sx0, sx1)) in se_spans.iter().enumerate() {
        let bay = LotBox::new(sx0, corr_y1, sx1, lot.y1);
        if idx == 0 {
            let ix = bay.x0 + (bay.w() / 2 - 1).max(0);
            let iy = bay.y0 + (bay.h() / 2 - 1).max(0);
            let _ = place_indoor_checked(
                edges,
                props,
                bay,
                make_prop(PropKind::ImagingScanner, ix, iy, 0, None, s + 50),
            );
            if place_indoor_checked(
                edges,
                props,
                bay,
                make_prop(PropKind::Desk, bay.x1 - 2, bay.y0, 0, None, s + 51),
            ) {
                let _ = place_indoor_checked(
                    edges,
                    props,
                    bay,
                    make_prop(PropKind::DeskClutter, bay.x1 - 2, bay.y0, 12, None, s + 52),
                );
            }
            let _ = place_indoor_checked(
                edges,
                props,
                bay,
                make_prop(
                    PropKind::MedicalCounter,
                    bay.x0,
                    bay.y1 - 1,
                    0,
                    Some(2),
                    s + 53,
                ),
            );
            let _ = place_indoor_checked(
                edges,
                props,
                bay,
                make_prop(
                    PropKind::BiohazardBin,
                    bay.x1 - 1,
                    bay.y1 - 1,
                    0,
                    None,
                    s + 54,
                ),
            );
        } else {
            furnish_patient_room(
                bay,
                idx + 4,
                false,
                is_upper,
                edges,
                props,
                surfaces,
                s + 500 + idx as u32 * 90,
            );
        }
    }

    // Nurse stations distributed along main corridor every ~8m + supply carts + monitors.
    let corr_bay = LotBox::new(lot.x0, corr_y0, lot.x1, corr_y1);
    let nurse_step = 9;
    let mut nx = lot.x0 + 2;
    let mut n_idx = 0u32;
    while nx + 3 <= lot.x1 - 2 {
        if !(nx < elev_serv_xy.0 + 3 && nx + 3 > elev_serv_xy.0) {
            let ny = corr_y0;
            if place_indoor_checked(
                edges,
                props,
                corr_bay,
                make_prop(PropKind::NurseStation, nx, ny, 0, None, s + 700 + n_idx),
            ) {
                let _ = place_indoor_checked(
                    edges,
                    props,
                    corr_bay,
                    make_prop(PropKind::TelemetryMonitor, nx, ny, 0, None, s + 710 + n_idx),
                );
                let _ = place_indoor_checked(
                    edges,
                    props,
                    corr_bay,
                    make_prop(
                        PropKind::BiohazardBin,
                        (nx + 3).min(lot.x1 - 1),
                        ny,
                        0,
                        None,
                        s + 720 + n_idx,
                    ),
                );
                let _ = place_indoor_checked(
                    edges,
                    props,
                    corr_bay,
                    make_prop(
                        PropKind::LinenCart,
                        (nx - 2).max(lot.x0),
                        ny,
                        0,
                        None,
                        s + 730 + n_idx,
                    ),
                );
            }
        }
        nx += nurse_step;
        n_idx += 1;
    }
    let _ = place_indoor_checked(
        edges,
        props,
        lot,
        make_prop(PropKind::WaterCooler, tx0, corr_y1, 0, None, s + 750),
    );
    let _ = place_indoor_checked(
        edges,
        props,
        lot,
        make_prop(
            PropKind::FireExtinguisher,
            lot.x0,
            corr_y0,
            0,
            None,
            s + 751,
        ),
    );
}

fn generate_hospital(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let (bx0, bx1) = safe_x_span(w);
    let by0 = 1;
    let by1 = (h - 4).max(by0 + 7);
    let lot = LotBox::new(bx0, by0, bx1, by1);
    let (corr_y0, _corr_y1, tx0, tx1, _bw, _bh) = hospital_corridor_bounds(lot);

    // Exterior ambulance bay AsphaltRoad & covered ER ConcreteWalk canopy (strictly South of building).
    let road_y = h as f32 - 1.35;
    let ctrl_road = [
        (-0.5, road_y - 0.15),
        (w as f32 * 0.25, road_y + b.rng.f32_range(-0.25, 0.25)),
        (w as f32 * 0.50, road_y - 0.20),
        (w as f32 * 0.75, road_y + b.rng.f32_range(-0.25, 0.25)),
        (w as f32 + 0.5, road_y - 0.10),
    ];
    let road_idx = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &ctrl_road, 210);
    let walk_y = by1 as f32 + 0.65;
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[
            (bx0 as f32 + 1.0, walk_y),
            (w as f32 * 0.5, walk_y + 0.08),
            (bx1 as f32 - 1.0, walk_y),
        ],
        115,
    );

    // Building 1F InteriorFloor polygon strictly inside campus walls.
    let floor_seed = (b.rng.next() & 0xffff_ffff) as u32;
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[
            (bx0 as f32, by0 as f32),
            (bx1 as f32, by0 as f32),
            (bx1 as f32, by1 as f32),
            (bx0 as f32, by1 as f32),
        ],
        0,
        floor_seed,
    );

    enclose_campus_shell(&mut b, lot, MaterialClass::ReinforcedConcrete, 3);
    let er_x = (tx0 + (tx1 - tx0) / 2 - 1).clamp(bx0 + 1, (bx1 - 2).max(bx0 + 1));
    // Wide ER double sliding glass doors + West/East campus doors near main corridor.
    b.put_edge(
        er_x,
        by1,
        EdgeSide::North,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    b.put_edge(
        er_x + 1,
        by1,
        EdgeSide::North,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    b.put_edge(
        bx0,
        corr_y0,
        EdgeSide::West,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    b.put_edge(
        bx0,
        corr_y0 + 1,
        EdgeSide::West,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    b.put_edge(
        bx1,
        corr_y0,
        EdgeSide::West,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    b.put_edge(
        bx1,
        corr_y0 + 1,
        EdgeSide::West,
        WallEdgeKind::DoorOpen,
        MaterialClass::Glass,
    );
    b.put_edge(
        bx0 + 1,
        by0,
        EdgeSide::North,
        WallEdgeKind::DoorClosed,
        MaterialClass::SheetSteel,
    );

    let (nx_a, _nx_b) = hospital_core_x_span(lot, tx0);
    let stair_xy = (nx_a, by0);
    let elev_pass_xy = ((nx_a + 2).min((bx1 - 3).max(nx_a)), by0);
    let elev_serv_xy = (tx0, corr_y0);

    let mut f1_props = Vec::new();
    let mut f1_extra_surfaces = Vec::new();
    fill_hospital_floor(
        lot,
        &mut b.edges,
        &mut f1_props,
        &mut f1_extra_surfaces,
        stair_xy,
        elev_pass_xy,
        elev_serv_xy,
        false,
        seed as u64,
    );
    trim_floor_props_to_connected(w, h, lot, &b.edges, &mut f1_props);
    for sf in f1_extra_surfaces {
        b.surfaces.push(sf);
    }
    for p in f1_props {
        let (cx, cy) = p.center_cm();
        let len = prop_spec(p.kind).stretchable.then_some(p.local_span_cm().0);
        if is_non_blocking_prop_safe(&p) {
            b.props.push(p);
        } else {
            let _ = b.try_place_cm(p.kind, cx, cy, p.rotation_deg, len, true);
        }
    }

    // Exterior ambulances, cars, landscaped flowerbeds & campus trees strictly outside building/road.
    let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, road_idx, 0.42, 0.0, 5);
    let _ = b.place_vehicle_along_ribbon(PropKind::Car, road_idx, 0.70, 0.0, 6);
    let _ = b.try_add_organic_patch(
        SurfaceFeatureKind::GardenBedZone,
        (2.0, 2.3),
        (1.3, 1.1),
        0.30,
        28,
    );
    let _ = b.try_add_organic_patch(
        SurfaceFeatureKind::GardenBedZone,
        (w as f32 - 2.0, 2.3),
        (1.3, 1.1),
        0.30,
        28,
    );
    b.scatter_trees(
        PropKind::MediumTree,
        4,
        2.8,
        RectM {
            x0: 1.2,
            y0: 1.2,
            x1: bx0 as f32 - 0.5,
            y1: by1 as f32,
        },
    );
    b.scatter_trees(
        PropKind::MediumTree,
        4,
        2.8,
        RectM {
            x0: bx1 as f32 + 0.5,
            y0: 1.2,
            x1: w as f32 - 1.2,
            y1: by1 as f32,
        },
    );
    b.scatter_turf_prop_rot(
        PropKind::ParkBench,
        2,
        15,
        RectM {
            x0: 1.2,
            y0: 1.2,
            x1: bx0 as f32 - 0.4,
            y1: h as f32 - 3.5,
        },
    );

    // Build 2F · Inpatient & Surgical Ward with aligned stairwell + elevators.
    let mut f2_edges = Vec::new();
    let mut f2_props = Vec::new();
    let mut f2_surfaces = vec![lot.polygon_feature(
        SurfaceFeatureKind::InteriorFloor,
        0,
        floor_seed.wrapping_add(19),
    )];
    enclose_upper_shell(&mut f2_edges, lot, MaterialClass::ReinforcedConcrete, 3);
    fill_hospital_floor(
        lot,
        &mut f2_edges,
        &mut f2_props,
        &mut f2_surfaces,
        stair_xy,
        elev_pass_xy,
        elev_serv_xy,
        true,
        (seed as u64).wrapping_add(77),
    );
    trim_floor_props_to_connected(w, h, lot, &f2_edges, &mut f2_props);
    prune_occluded_surfaces(&mut f2_surfaces, w, h);

    let mut sec = b.finalize_connected();
    let f1_floor = MapFloorLevel {
        floor_index: 0,
        label: "1F · Emergency & Trauma Wing".to_string(),
        elevation_cm: 0,
        wall_edges: sec.wall_edges.clone(),
        props: sec.props.clone(),
        surfaces: sec.surfaces.clone(),
        ..Default::default()
    };
    let f2_floor = MapFloorLevel {
        floor_index: 1,
        label: "2F · Inpatient & Surgical Ward".to_string(),
        elevation_cm: 360,
        wall_edges: f2_edges,
        props: f2_props,
        surfaces: f2_surfaces,
        ..Default::default()
    };
    sec.floors = vec![f1_floor, f2_floor];
    sec.active_floor = 0;
    sec
}

fn is_non_blocking_prop_safe(p: &MapPropPlacement) -> bool {
    effective_obstruction(p) == PropObstruction::FloorLayer
}

fn generate_shopping_plaza(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let (bx0, bx1) = safe_x_span(w);
    let bw = bx1 - bx0;
    let by0 = 1;
    let store_h = ((h * 38) / 100).clamp(5, (h - 7).max(5));
    let by1 = (by0 + store_h).min(h - 6);

    // Shaded outdoor concrete canopy promenade immediately in front of storefronts.
    let prom_y = by1 as f32 + 1.05;
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[
            (1.0, prom_y),
            (w as f32 * 0.5, prom_y + 0.05),
            (w as f32 - 1.0, prom_y),
        ],
        200,
    );

    // Curving parking drive aisle across the South lot with generous landscaped islands left clear.
    let lot_y = (by1 as f32 + 3.45).min(h as f32 - 1.45);
    let wob = b.rng.f32_range(-0.35, 0.35);
    let park_ctrl = [
        (-0.5, lot_y - wob),
        (w as f32 * 0.25, lot_y + wob),
        (w as f32 * 0.50, lot_y - wob * 0.6),
        (w as f32 * 0.75, lot_y + wob * 0.8),
        (w as f32 + 0.5, lot_y),
    ];
    let road_idx = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &park_ctrl, 250);

    // Partition storefront strip into 3-6 varied StoreArchetypes reusing map_gen_commercial.
    let store_count = (bw / 7).clamp(3, 6);
    let archetypes = [
        StoreArchetype::CafeRestaurant,
        StoreArchetype::ConvenienceBodega,
        StoreArchetype::Pharmacy,
        StoreArchetype::ApparelBoutique,
        StoreArchetype::Electronics,
        StoreArchetype::DepartmentAnchor,
    ];
    let offset = (seed.unsigned_abs() as usize) % archetypes.len();
    let mut cur_x = bx0;
    for i in 0..store_count {
        let next_x = if i + 1 == store_count {
            bx1
        } else {
            bx0 + ((i + 1) * bw) / store_count
        };
        let bay = LotBox::new(cur_x, by0, next_x, by1);
        cur_x = next_x;
        let arch = archetypes[(offset + i as usize) % archetypes.len()];
        let unit = generate_store_unit(
            w,
            h,
            bay,
            arch,
            CardinalEntry::South,
            true,
            &b.spawn_pads_m,
            (seed as u64).wrapping_add(i as u64 * 131),
        );
        for e in unit.wall_edges {
            b.put_edge(e.x, e.y, e.side, e.kind, e.material);
        }
        b.surfaces.extend(unit.surfaces);
        b.props.extend(unit.props);
    }

    // Promenade furniture: cafe patio tables, benches, and outdoor MallKiosk cart strictly on promenade.
    let kiosk_cx = OutdoorBuilder::cm(w as f32 * 0.5 + b.rng.f32_range(-1.5, 1.5));
    let kiosk_rot = b.rng.range(-12, 13);
    let prom_cy = OutdoorBuilder::cm(prom_y - 0.15);
    let _ = b.try_place_off_road_cm(PropKind::MallKiosk, kiosk_cx, prom_cy, kiosk_rot, None);
    let patio_x = OutdoorBuilder::cm(bx0 as f32 + 2.5);
    let _ = b.try_place_off_road_cm(PropKind::RoundTable, patio_x, prom_cy, 15, None);
    let _ = b.try_place_off_road_cm(
        PropKind::ParkBench,
        OutdoorBuilder::cm(bx1 as f32 - 2.5),
        prom_cy,
        180,
        None,
    );

    // Angled parking vehicles along parking lane.
    for (idx, frac) in [0.18_f32, 0.38, 0.62, 0.82].iter().enumerate() {
        let veh = if idx % 3 == 2 {
            PropKind::UtilityVan
        } else {
            PropKind::Car
        };
        let _ = b.place_vehicle_along_ribbon(veh, road_idx, *frac, 0.0, 22);
    }

    // Landscaped turf islands + trees on flank margins strictly outside roads, promenade & interiors.
    let _ = b.try_add_organic_patch(
        SurfaceFeatureKind::GardenBedZone,
        (1.9, 2.6),
        (1.3, 1.2),
        0.30,
        28,
    );
    let _ = b.try_add_organic_patch(
        SurfaceFeatureKind::GardenBedZone,
        (w as f32 - 1.9, 2.6),
        (1.3, 1.2),
        0.30,
        28,
    );
    b.scatter_trees(
        PropKind::MediumTree,
        4,
        2.5,
        RectM {
            x0: 1.0,
            y0: 1.0,
            x1: bx0 as f32 - 0.5,
            y1: prom_y - 1.2,
        },
    );
    b.scatter_trees(
        PropKind::MediumTree,
        4,
        2.5,
        RectM {
            x0: bx1 as f32 + 0.5,
            y0: 1.0,
            x1: w as f32 - 1.0,
            y1: prom_y - 1.2,
        },
    );
    b.scatter_turf_prop_rot(
        PropKind::Bush,
        4,
        25,
        RectM {
            x0: 0.8,
            y0: 0.8,
            x1: w as f32 - 0.8,
            y1: prom_y - 1.0,
        },
    );

    b.finalize_connected()
}

fn build_mall_concourse_shell(
    edges: &mut Vec<MapWallEdge>,
    w: i32,
    h: i32,
    conc_y0: i32,
    conc_y1: i32,
    is_upper: bool,
) {
    let mid_x = w / 2;
    for x in 0..w {
        let is_portal = !is_upper && (x == mid_x || x == mid_x - 1);
        let (k, m) = if is_portal {
            (WallEdgeKind::DoorOpen, MaterialClass::Glass)
        } else if x % 3 == 1 && x + 1 < w {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete)
        };
        edge_upsert(edges, x, 0, EdgeSide::North, k, m);
        edge_upsert(edges, x, h, EdgeSide::North, k, m);
    }
    for y in 0..h {
        let is_portal = !is_upper && y >= conc_y0 && y < conc_y1 && (y == h / 2 || y == h / 2 - 1);
        let (k, m) = if is_portal {
            (WallEdgeKind::DoorOpen, MaterialClass::Glass)
        } else if y % 3 == 1 && y + 1 < h {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, MaterialClass::ReinforcedConcrete)
        };
        edge_upsert(edges, 0, y, EdgeSide::West, k, m);
        edge_upsert(edges, w, y, EdgeSide::West, k, m);
    }
}

fn generate_shopping_mall(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let conc_half = if h >= 28 { 3 } else { 2 };
    let conc_y0 = (h / 2 - conc_half).clamp(4, (h - 7).max(4));
    let conc_y1 = (h / 2 + conc_half).clamp(conc_y0 + 3, (h - 4).max(conc_y0 + 3));
    let trans_half = if w >= 36 { 3 } else { 2 };
    let cx0_lo = 4.min((w - 3).max(1));
    let cx0 = (w / 2 - trans_half).clamp(cx0_lo, (w - 7).max(cx0_lo));
    let cx1_lo = (cx0 + 3).min((w - 1).max(cx0 + 1));
    let cx1 = (w / 2 + trans_half).clamp(cx1_lo, (w - 4).max(cx1_lo));

    let full_lot = LotBox::new(0, 0, w, h);
    let floor_seed = (seed as u32).wrapping_add(0x4A11);
    b.add_polygon_surface(
        SurfaceFeatureKind::InteriorFloor,
        &[
            (0.0, 0.0),
            (w as f32, 0.0),
            (w as f32, h as f32),
            (0.0, h as f32),
        ],
        0,
        floor_seed,
    );

    // Partition NW, NE, SW, SE store wings (splitting wide quadrants into anchor + boutique bays).
    let archetypes = [
        StoreArchetype::DepartmentAnchor,
        StoreArchetype::ApparelBoutique,
        StoreArchetype::CafeRestaurant,
        StoreArchetype::Electronics,
        StoreArchetype::Pharmacy,
        StoreArchetype::Grocery,
    ];
    let mut wing_lots: Vec<(LotBox, CardinalEntry, StoreArchetype)> = Vec::new();
    let quadrants = [
        (0, cx0, 0, conc_y0, CardinalEntry::South),
        (cx1, w, 0, conc_y0, CardinalEntry::South),
        (0, cx0, conc_y1, h, CardinalEntry::North),
        (cx1, w, conc_y1, h, CardinalEntry::North),
    ];
    for (qi, &(qx0, qx1, qy0, qy1, entry)) in quadrants.iter().enumerate() {
        let qw = qx1 - qx0;
        if qw >= 14 {
            let split = qx0 + qw / 2;
            wing_lots.push((
                LotBox::new(qx0, qy0, split, qy1),
                entry,
                archetypes[(qi * 2) % archetypes.len()],
            ));
            wing_lots.push((
                LotBox::new(split, qy0, qx1, qy1),
                entry,
                archetypes[(qi * 2 + 1) % archetypes.len()],
            ));
        } else if qw >= 4 && (qy1 - qy0) >= 4 {
            wing_lots.push((
                LotBox::new(qx0, qy0, qx1, qy1),
                entry,
                archetypes[qi % archetypes.len()],
            ));
        }
    }

    for (bi, &(bay, entry, arch)) in wing_lots.iter().enumerate() {
        let unit = generate_store_unit(
            w,
            h,
            bay,
            arch,
            entry,
            false,
            &b.spawn_pads_m,
            (seed as u64).wrapping_add(bi as u64 * 97),
        );
        for e in unit.wall_edges {
            b.put_edge(e.x, e.y, e.side, e.kind, e.material);
        }
        b.props.extend(unit.props);
    }

    // Apply enclosing 0..w, 0..h galleria perimeter shell on top so portal doors and glazed entries stay exact.
    build_mall_concourse_shell(&mut b.edges, w, h, conc_y0, conc_y1, false);

    // Central mall atrium stairwell + MallKiosk + PlanterTable seating strictly inside concourse/transept.
    let stair_x = (cx0 + 1).clamp(4, (w - 6).max(4));
    let stair_y = conc_y0;
    let stair_1f = make_prop(PropKind::Stairwell, stair_x, stair_y, 0, None, floor_seed);
    b.props.push(stair_1f);

    let cy_mid_cm = OutdoorBuilder::cm((conc_y0 + conc_y1) as f32 * 0.5);
    let _ = b.try_place_cm(
        PropKind::MallKiosk,
        OutdoorBuilder::cm((cx0 + cx1) as f32 * 0.5 + 1.5),
        cy_mid_cm,
        12,
        None,
        true,
    );
    let _ = b.try_place_cm(
        PropKind::PlanterTable,
        OutdoorBuilder::cm((cx0 + 1) as f32),
        OutdoorBuilder::cm(conc_y1 as f32 - 0.6),
        0,
        Some(200),
        true,
    );
    if w >= 28 {
        let _ = b.try_place_cm(
            PropKind::MallKiosk,
            OutdoorBuilder::cm(w as f32 * 0.28),
            cy_mid_cm,
            348,
            None,
            true,
        );
        let _ = b.try_place_cm(
            PropKind::MallKiosk,
            OutdoorBuilder::cm(w as f32 * 0.72),
            cy_mid_cm,
            15,
            None,
            true,
        );
        let _ = b.try_place_cm(
            PropKind::SmallFountain,
            OutdoorBuilder::cm(w as f32 * 0.40),
            cy_mid_cm,
            0,
            None,
            true,
        );
        let _ = b.try_place_cm(
            PropKind::ParkBench,
            OutdoorBuilder::cm(w as f32 * 0.60),
            OutdoorBuilder::cm(conc_y0 as f32 + 0.7),
            0,
            None,
            true,
        );
        let _ = b.try_place_cm(
            PropKind::TrashCan,
            OutdoorBuilder::cm(w as f32 * 0.62),
            OutdoorBuilder::cm(conc_y0 as f32 + 0.7),
            0,
            None,
            true,
        );
    }

    let mut sec = b.finalize_connected();

    // Build 2F · Mezzanine Gallery with aligned stairwell, skylight BridgeDeck span, and mezzanine shops.
    let mut f2_edges = Vec::new();
    let mut f2_props = Vec::new();
    for (bi, &(bay, entry, arch)) in wing_lots.iter().enumerate() {
        let f2_arch = match arch {
            StoreArchetype::DepartmentAnchor => StoreArchetype::ApparelBoutique,
            StoreArchetype::Grocery => StoreArchetype::CafeRestaurant,
            other => other,
        };
        let unit = generate_store_unit(
            w,
            h,
            bay,
            f2_arch,
            entry,
            false,
            &[],
            (seed as u64).wrapping_add(500 + bi as u64 * 59),
        );
        for e in unit.wall_edges {
            edge_upsert(&mut f2_edges, e.x, e.y, e.side, e.kind, e.material);
        }
        f2_props.extend(unit.props);
    }
    build_mall_concourse_shell(&mut f2_edges, w, h, conc_y0, conc_y1, true);
    f2_props.push(make_prop(
        PropKind::Stairwell,
        stair_x,
        stair_y,
        0,
        None,
        floor_seed + 1,
    ));
    let kiosk_x_lo = 3.min((w - 1).max(0));
    let kiosk_2f = make_prop(
        PropKind::MallKiosk,
        (w / 2 - 1).clamp(kiosk_x_lo, (w - 4).max(kiosk_x_lo)),
        conc_y0 + 1,
        0,
        None,
        floor_seed + 2,
    );
    let _ = place_indoor_checked(&f2_edges, &mut f2_props, full_lot, kiosk_2f);
    let planter_2f = make_prop(
        PropKind::PlanterTable,
        cx0,
        conc_y1 - 1,
        0,
        Some(2),
        floor_seed + 3,
    );
    let _ = place_indoor_checked(&f2_edges, &mut f2_props, full_lot, planter_2f);
    trim_floor_props_to_connected(w, h, full_lot, &f2_edges, &mut f2_props);

    let mut f2_surfaces = vec![
        full_lot.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, floor_seed + 7),
        MapSurfaceFeature {
            kind: SurfaceFeatureKind::BridgeDeck,
            shape: satya_schema::SurfaceShapeKind::Polygon,
            points_cm: vec![
                satya_schema::MapPointCm::new(cx0 * 100, conc_y0 * 100),
                satya_schema::MapPointCm::new(cx1 * 100, conc_y0 * 100),
                satya_schema::MapPointCm::new(cx1 * 100, conc_y1 * 100),
                satya_schema::MapPointCm::new(cx0 * 100, conc_y1 * 100),
            ],
            width_cm: 0,
            border_style_cm: 12,
            variant_seed: floor_seed + 8,
            tint_rgb: None,
        },
    ];
    prune_occluded_surfaces(&mut f2_surfaces, w, h);
    debug_assert!(floor_lot_connected(w, h, full_lot, &f2_edges, &f2_props));

    let f1_floor = MapFloorLevel {
        floor_index: 0,
        label: "1F · Galleria Concourse".to_string(),
        elevation_cm: 0,
        wall_edges: sec.wall_edges.clone(),
        props: sec.props.clone(),
        surfaces: sec.surfaces.clone(),
        ..Default::default()
    };
    let f2_floor = MapFloorLevel {
        floor_index: 1,
        label: "2F · Mezzanine Gallery".to_string(),
        elevation_cm: 420,
        wall_edges: f2_edges,
        props: f2_props,
        surfaces: f2_surfaces,
        ..Default::default()
    };
    sec.floors = vec![f1_floor, f2_floor];
    sec.active_floor = 0;
    sec
}

/// Dispatches Phase 5 civic and commercial facility sector presets (`Hospital`, `ShoppingPlaza`, `ShoppingMall`).
#[must_use]
pub fn generate_civic_or_retail_sector(
    preset: SectorPreset,
    width: i32,
    height: i32,
    seed: i64,
) -> ProceduralSector {
    let w = width.clamp(
        crate::tactical_map::MIN_SECTOR_DIM,
        crate::tactical_map::MAX_SECTOR_DIM,
    );
    let h = height.clamp(
        crate::tactical_map::MIN_SECTOR_DIM,
        crate::tactical_map::MAX_SECTOR_DIM,
    );
    match preset {
        SectorPreset::Hospital => generate_hospital(w, h, seed),
        SectorPreset::ShoppingPlaza => generate_shopping_plaza(w, h, seed),
        SectorPreset::ShoppingMall => generate_shopping_mall(w, h, seed),
        _ => ProceduralSector::default(),
    }
}

/// Ensures at least one non-blocking surface check exists cleanly (used by sibling test verification).
#[allow(dead_code)]
fn has_exposed_half_cover(sec: &ProceduralSector) -> bool {
    let _ = topmost_surface_at_m;
    sec.props
        .iter()
        .any(|p| prop_spec(p.kind).obstruction == PropObstruction::HalfCover)
}
