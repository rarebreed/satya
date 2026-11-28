//! # Public Restroom Suite Generator (`map_restroom_gen`)
//!
//! Splits commercial, hospitality, civic, and industrial restroom bays into separate **Men's**
//! and **Women's** restrooms (plus an optional **Family Room** in larger businesses), complete
//! with [`WallEdgeKind::StallPartition`] privacy dividers, stall doors, [`PropKind::Urinal`]
//! rows, [`PropKind::Toilet`] stalls, [`PropKind::ChangingTable`], and stretched
//! [`PropKind::BathroomSink`] vanities.

use satya_schema::{
    EdgeSide, MapPropPlacement, MapSurfaceFeature, MapWallEdge, MaterialClass, PropKind,
    SurfaceFeatureKind, WallEdgeKind,
};

use crate::map_dressing::make_prop;
use crate::map_gen_residential::{CardinalEntry, LotBox};

fn upsert_edge(
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

/// Converts any existing windows/doors along `bay`'s perimeter to solid `Wall` edges (preserving
/// structural masonry on exterior faces) so restroom plumbing walls stay private and solid-backed.
fn solidify_restroom_perimeter(
    bay: LotBox,
    entry_side: CardinalEntry,
    wall_mat: MaterialClass,
    edges: &mut Vec<MapWallEdge>,
) {
    for x in bay.x0..bay.x1 {
        let n_mat = if entry_side == CardinalEntry::North {
            MaterialClass::Drywall
        } else {
            wall_mat
        };
        let s_mat = if entry_side == CardinalEntry::South {
            MaterialClass::Drywall
        } else {
            wall_mat
        };
        upsert_edge(edges, x, bay.y0, EdgeSide::North, WallEdgeKind::Wall, n_mat);
        upsert_edge(edges, x, bay.y1, EdgeSide::North, WallEdgeKind::Wall, s_mat);
    }
    for y in bay.y0..bay.y1 {
        let w_mat = if entry_side == CardinalEntry::West {
            MaterialClass::Drywall
        } else {
            wall_mat
        };
        let e_mat = if entry_side == CardinalEntry::East {
            MaterialClass::Drywall
        } else {
            wall_mat
        };
        upsert_edge(edges, bay.x0, y, EdgeSide::West, WallEdgeKind::Wall, w_mat);
        upsert_edge(edges, bay.x1, y, EdgeSide::West, WallEdgeKind::Wall, e_mat);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestroomKind {
    Women,
    Men,
    Family,
}

#[derive(Debug, Clone, Copy)]
struct RestroomSubBay {
    lot: LotBox,
    kind: RestroomKind,
}

/// Maps `(span_idx, depth_idx)` inside `sub` (where `depth_idx = 0` is on `entry_side` and
/// `depth_idx = depth - 1` is the opposite back wall) to `(cell_x, cell_y)`.
const fn sub_cell(sub: LotBox, entry_side: CardinalEntry, s: i32, d: i32) -> (i32, i32) {
    match entry_side {
        CardinalEntry::West => (sub.x0 + d, sub.y0 + s),
        CardinalEntry::East => (sub.x1 - 1 - d, sub.y0 + s),
        CardinalEntry::North => (sub.x0 + s, sub.y0 + d),
        CardinalEntry::South => (sub.x0 + s, sub.y1 - 1 - d),
    }
}

/// Rotation facing away from the back wall (opposite `entry_side`) into the room.
const fn back_wall_prop_rot(entry_side: CardinalEntry) -> i32 {
    match entry_side {
        CardinalEntry::West => 90,
        CardinalEntry::East => 270,
        CardinalEntry::North => 180,
        CardinalEntry::South => 0,
    }
}

/// Rotation facing away from the `entry_side` wall into the room.
const fn front_wall_prop_rot(entry_side: CardinalEntry) -> i32 {
    match entry_side {
        CardinalEntry::West => 270,
        CardinalEntry::East => 90,
        CardinalEntry::North => 0,
        CardinalEntry::South => 180,
    }
}

fn put_entry_door(
    sub: LotBox,
    entry_side: CardinalEntry,
    s_idx: i32,
    edges: &mut Vec<MapWallEdge>,
    open: bool,
) {
    let kind = if open {
        WallEdgeKind::DoorOpen
    } else {
        WallEdgeKind::DoorClosed
    };
    let (cx, cy) = sub_cell(sub, entry_side, s_idx, 0);
    match entry_side {
        CardinalEntry::West => upsert_edge(edges, cx, cy, EdgeSide::West, kind, MaterialClass::SolidWood),
        CardinalEntry::East => upsert_edge(edges, cx + 1, cy, EdgeSide::West, kind, MaterialClass::SolidWood),
        CardinalEntry::North => upsert_edge(edges, cx, cy, EdgeSide::North, kind, MaterialClass::SolidWood),
        CardinalEntry::South => upsert_edge(edges, cx, cy + 1, EdgeSide::North, kind, MaterialClass::SolidWood),
    }
}

/// Emits a `WallEdgeKind::StallPartition` between span slot `s_idx` and `s_idx + 1` at `depth_idx`.
fn put_stall_divider(
    sub: LotBox,
    entry_side: CardinalEntry,
    s_idx: i32,
    d_idx: i32,
    edges: &mut Vec<MapWallEdge>,
) {
    let (cx, cy) = sub_cell(sub, entry_side, s_idx + 1, d_idx);
    match entry_side {
        CardinalEntry::West | CardinalEntry::East => {
            upsert_edge(
                edges,
                cx,
                cy,
                EdgeSide::North,
                WallEdgeKind::StallPartition,
                MaterialClass::SheetSteel,
            );
        }
        CardinalEntry::North | CardinalEntry::South => {
            upsert_edge(
                edges,
                cx,
                cy,
                EdgeSide::West,
                WallEdgeKind::StallPartition,
                MaterialClass::SheetSteel,
            );
        }
    }
}

/// Emits a restroom stall door on the aisle-facing edge of `(s_idx, d_idx)`.
fn put_stall_door(
    sub: LotBox,
    entry_side: CardinalEntry,
    s_idx: i32,
    d_idx: i32,
    open: bool,
    edges: &mut Vec<MapWallEdge>,
) {
    let kind = if open {
        WallEdgeKind::DoorOpen
    } else {
        WallEdgeKind::DoorClosed
    };
    let (cx, cy) = sub_cell(sub, entry_side, s_idx, d_idx);
    match entry_side {
        CardinalEntry::West => upsert_edge(edges, cx, cy, EdgeSide::West, kind, MaterialClass::SheetSteel),
        CardinalEntry::East => upsert_edge(edges, cx + 1, cy, EdgeSide::West, kind, MaterialClass::SheetSteel),
        CardinalEntry::North => upsert_edge(edges, cx, cy, EdgeSide::North, kind, MaterialClass::SheetSteel),
        CardinalEntry::South => upsert_edge(edges, cx, cy + 1, EdgeSide::North, kind, MaterialClass::SheetSteel),
    }
}

fn populate_single_restroom(
    sub: LotBox,
    kind: RestroomKind,
    entry_side: CardinalEntry,
    seed: u64,
    edges: &mut Vec<MapWallEdge>,
    props: &mut Vec<MapPropPlacement>,
) {
    let (span, depth) = match entry_side {
        CardinalEntry::West | CardinalEntry::East => (sub.h(), sub.w()),
        CardinalEntry::North | CardinalEntry::South => (sub.w(), sub.h()),
    };
    if span <= 0 || depth <= 0 {
        return;
    }

    let door_s = 0;
    put_entry_door(sub, entry_side, door_s, edges, (seed & 1) == 0);

    let back_d = depth - 1;
    let back_rot = back_wall_prop_rot(entry_side);
    let front_rot = front_wall_prop_rot(entry_side);
    let has_aisle = depth >= 3;
    let vseed = (seed & 0xffff_ffff) as u32;

    match kind {
        RestroomKind::Family => {
            let (tx, ty) = sub_cell(sub, entry_side, 0, back_d);
            props.push(make_prop(PropKind::Toilet, tx, ty, back_rot, None, vseed));
            if span >= 2 {
                let (sx, sy) = if depth >= 2 {
                    sub_cell(sub, entry_side, span - 1, 0)
                } else {
                    sub_cell(sub, entry_side, span - 1, back_d)
                };
                props.push(make_prop(
                    PropKind::BathroomSink,
                    sx,
                    sy,
                    if depth >= 2 { front_rot } else { back_rot },
                    None,
                    vseed.wrapping_add(1),
                ));
            }
            if span >= 3 && depth >= 2 {
                let (cx, cy) = sub_cell(sub, entry_side, 1, back_d);
                props.push(make_prop(
                    PropKind::ChangingTable,
                    cx,
                    cy,
                    back_rot,
                    None,
                    vseed.wrapping_add(2),
                ));
            }
            if span >= 2 && depth >= 3 {
                let (bx, by) = sub_cell(sub, entry_side, span - 1, back_d);
                props.push(make_prop(
                    PropKind::TrashCan,
                    bx,
                    by,
                    back_rot,
                    None,
                    vseed.wrapping_add(3),
                ));
            }
        }
        RestroomKind::Women => {
            let stall_count = span.clamp(1, 5);
            for i in 0..stall_count {
                let (tx, ty) = sub_cell(sub, entry_side, i, back_d);
                props.push(make_prop(
                    PropKind::Toilet,
                    tx,
                    ty,
                    back_rot,
                    None,
                    vseed.wrapping_add(i as u32),
                ));
                if i + 1 < stall_count {
                    put_stall_divider(sub, entry_side, i, back_d, edges);
                }
                if has_aisle {
                    let is_open = seed.wrapping_add(i as u64).is_multiple_of(3);
                    put_stall_door(sub, entry_side, i, back_d, is_open, edges);
                }
            }
            if depth >= 2 && span >= 2 {
                let sink_avail = span - 1;
                let sink_len = sink_avail.clamp(1, 3);
                let sink_s = 1;
                let (sx, sy) = sub_cell(sub, entry_side, sink_s, 0);
                // When BathroomSink spans multiple cells along West/East entry wall, place at min y.
                let (ax, ay) = match entry_side {
                    CardinalEntry::West | CardinalEntry::East => (sx, sy),
                    CardinalEntry::North | CardinalEntry::South => (sx, sy),
                };
                props.push(make_prop(
                    PropKind::BathroomSink,
                    ax,
                    ay,
                    front_rot,
                    (sink_len > 1).then_some(sink_len),
                    vseed.wrapping_add(10),
                ));
                if span > 1 + sink_len {
                    let (bx, by) = sub_cell(sub, entry_side, span - 1, 0);
                    props.push(make_prop(
                        PropKind::TrashCan,
                        bx,
                        by,
                        front_rot,
                        None,
                        vseed.wrapping_add(11),
                    ));
                }
            }
        }
        RestroomKind::Men => {
            let n_toilets = (span / 2).clamp(1, 2);
            let n_urinals = (span - n_toilets).clamp(0, 4);
            for i in 0..n_toilets {
                let (tx, ty) = sub_cell(sub, entry_side, i, back_d);
                props.push(make_prop(
                    PropKind::Toilet,
                    tx,
                    ty,
                    back_rot,
                    None,
                    vseed.wrapping_add(i as u32),
                ));
                if i + 1 < n_toilets + n_urinals {
                    put_stall_divider(sub, entry_side, i, back_d, edges);
                }
                if has_aisle {
                    let is_open = seed.wrapping_add(i as u64).is_multiple_of(2);
                    put_stall_door(sub, entry_side, i, back_d, is_open, edges);
                }
            }
            for u in 0..n_urinals {
                let s_idx = n_toilets + u;
                let (ux, uy) = sub_cell(sub, entry_side, s_idx, back_d);
                props.push(make_prop(
                    PropKind::Urinal,
                    ux,
                    uy,
                    back_rot,
                    None,
                    vseed.wrapping_add(20 + u as u32),
                ));
                if u + 1 < n_urinals {
                    put_stall_divider(sub, entry_side, s_idx, back_d, edges);
                }
            }
            if depth >= 2 && span >= 2 {
                let sink_avail = span - 1;
                let sink_len = sink_avail.clamp(1, 3);
                let (sx, sy) = sub_cell(sub, entry_side, 1, 0);
                props.push(make_prop(
                    PropKind::BathroomSink,
                    sx,
                    sy,
                    front_rot,
                    (sink_len > 1).then_some(sink_len),
                    vseed.wrapping_add(30),
                ));
                if span > 1 + sink_len {
                    let (bx, by) = sub_cell(sub, entry_side, span - 1, 0);
                    props.push(make_prop(
                        PropKind::TrashCan,
                        bx,
                        by,
                        front_rot,
                        None,
                        vseed.wrapping_add(31),
                    ));
                }
            }
        }
    }
}

/// Builds a complete public restroom suite inside `bay` with entry doors facing `entry_side`:
/// - `span >= 6 && depth >= 2`: split into separate **Men's** (stalls + urinals) and **Women's**
///   (stalls) rooms with `StallPartition` dividers, stall doors (when `depth >= 3`), and stretched
///   vanity sinks.
/// - `include_family_if_large && span >= 10 && depth >= 3`: carves a 3rd compact **Family Room**
///   with a `ChangingTable`, `Toilet`, and `BathroomSink`.
/// - Compact bays (`span < 6 || depth < 2`): single accessible **Family Room**.
#[allow(clippy::too_many_arguments)]
pub fn build_public_restroom_suite_in_lot(
    bay: LotBox,
    entry_side: CardinalEntry,
    include_family_if_large: bool,
    wall_mat: MaterialClass,
    seed: u64,
    edges: &mut Vec<MapWallEdge>,
    props: &mut Vec<MapPropPlacement>,
    surfaces: &mut Vec<MapSurfaceFeature>,
) {
    if bay.w() < 2 || bay.h() < 2 {
        return;
    }
    solidify_restroom_perimeter(bay, entry_side, wall_mat, edges);
    surfaces.push(bay.polygon_feature(
        SurfaceFeatureKind::CeramicTileFloor,
        0,
        (seed & 0xffff_ffff) as u32,
    ));

    let (span, depth) = match entry_side {
        CardinalEntry::West | CardinalEntry::East => (bay.h(), bay.w()),
        CardinalEntry::North | CardinalEntry::South => (bay.w(), bay.h()),
    };
    let split_sub = |s0: i32, s1: i32| -> LotBox {
        match entry_side {
            CardinalEntry::West | CardinalEntry::East => {
                LotBox::new(bay.x0, bay.y0 + s0, bay.x1, bay.y0 + s1)
            }
            CardinalEntry::North | CardinalEntry::South => {
                LotBox::new(bay.x0 + s0, bay.y0, bay.x0 + s1, bay.y1)
            }
        }
    };
    let add_dividing_wall = |s_cut: i32, edges: &mut Vec<MapWallEdge>| match entry_side {
        CardinalEntry::West | CardinalEntry::East => {
            for x in bay.x0..bay.x1 {
                upsert_edge(
                    edges,
                    x,
                    bay.y0 + s_cut,
                    EdgeSide::North,
                    WallEdgeKind::Wall,
                    MaterialClass::Drywall,
                );
            }
        }
        CardinalEntry::North | CardinalEntry::South => {
            for y in bay.y0..bay.y1 {
                upsert_edge(
                    edges,
                    bay.x0 + s_cut,
                    y,
                    EdgeSide::West,
                    WallEdgeKind::Wall,
                    MaterialClass::Drywall,
                );
            }
        }
    };

    let subs: Vec<RestroomSubBay> = if span >= 10 && depth >= 3 && include_family_if_large {
        let fam_w = if span >= 12 { 3 } else { 2 };
        let rem = span - fam_w;
        let women_w = rem / 2;
        add_dividing_wall(women_w, edges);
        add_dividing_wall(rem, edges);
        vec![
            RestroomSubBay {
                lot: split_sub(0, women_w),
                kind: RestroomKind::Women,
            },
            RestroomSubBay {
                lot: split_sub(women_w, rem),
                kind: RestroomKind::Men,
            },
            RestroomSubBay {
                lot: split_sub(rem, span),
                kind: RestroomKind::Family,
            },
        ]
    } else if span >= 6 && depth >= 2 {
        let women_w = span / 2;
        add_dividing_wall(women_w, edges);
        vec![
            RestroomSubBay {
                lot: split_sub(0, women_w),
                kind: RestroomKind::Women,
            },
            RestroomSubBay {
                lot: split_sub(women_w, span),
                kind: RestroomKind::Men,
            },
        ]
    } else {
        vec![RestroomSubBay {
            lot: bay,
            kind: RestroomKind::Family,
        }]
    };

    for (idx, sub) in subs.into_iter().enumerate() {
        populate_single_restroom(
            sub.lot,
            sub.kind,
            entry_side,
            seed.wrapping_add((idx as u64 + 1).wrapping_mul(0x9E37_79B9)),
            edges,
            props,
        );
    }
}
