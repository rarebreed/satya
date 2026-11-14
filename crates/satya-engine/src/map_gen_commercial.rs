//! # Composable Commercial Retail Store Generator (`map_gen_commercial`)
//!
//! Generates shop bays (`StoreArchetype`: `Grocery`, `ApparelBoutique`, `CafeRestaurant`,
//! `Pharmacy`, `Electronics`, `ConvenienceBodega`, `DepartmentAnchor`) inside an arbitrary
//! [`LotBox`] with glazed storefronts, service backrooms/restrooms, checkout banks, and
//! navigable [`PropKind::DisplayRack`] aisles.

use satya_schema::{
    EdgeSide, MaterialClass, PropKind, StoreArchetype, SurfaceFeatureKind, WallEdgeKind,
};

use crate::map_gen_residential::{
    CardinalEntry, FloorContent, LotBox, trim_floor_props_to_connected,
};
use crate::map_interior_gen::{Furnisher, Rng, Role, Room, Shell};

/// Storefront wall & interior door materials tuned to [`StoreArchetype`].
const fn store_materials(archetype: StoreArchetype) -> (MaterialClass, MaterialClass) {
    match archetype {
        StoreArchetype::DepartmentAnchor | StoreArchetype::Electronics => {
            (MaterialClass::ReinforcedConcrete, MaterialClass::Glass)
        }
        StoreArchetype::ConvenienceBodega | StoreArchetype::Grocery => {
            (MaterialClass::ConcreteBlock, MaterialClass::SheetSteel)
        }
        StoreArchetype::ApparelBoutique
        | StoreArchetype::CafeRestaurant
        | StoreArchetype::Pharmacy => (MaterialClass::Brick, MaterialClass::SolidWood),
    }
}

/// Builds perimeter walls for `bay` with a wide glazed facade + main entry on `front_side`,
/// and an optional steel rear service door on the opposite face.
fn enclose_store_shell(
    shell: &mut Shell,
    bay: LotBox,
    archetype: StoreArchetype,
    front_side: CardinalEntry,
    rear_door: bool,
) {
    let (wall_mat, interior_door_mat) = store_materials(archetype);
    for x in bay.x0..bay.x1 {
        let glazed_n = match front_side {
            CardinalEntry::North => x > bay.x0 && x + 1 < bay.x1,
            _ => false,
        };
        let glazed_s = match front_side {
            CardinalEntry::South => x > bay.x0 && x + 1 < bay.x1,
            _ => false,
        };
        let (kn, mn) = if glazed_n {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        let (ks, ms) = if glazed_s {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        shell.put(x, bay.y0, EdgeSide::North, kn, mn);
        shell.put(x, bay.y1, EdgeSide::North, ks, ms);
    }
    for y in bay.y0..bay.y1 {
        let glazed_w = match front_side {
            CardinalEntry::West => y > bay.y0 && y + 1 < bay.y1,
            _ => false,
        };
        let glazed_e = match front_side {
            CardinalEntry::East => y > bay.y0 && y + 1 < bay.y1,
            _ => false,
        };
        let (kw, mw) = if glazed_w {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        let (ke, me) = if glazed_e {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        shell.put(bay.x0, y, EdgeSide::West, kw, mw);
        shell.put(bay.x1, y, EdgeSide::West, ke, me);
    }

    let mx = bay.x0 + bay.w() / 2;
    let my = bay.y0 + bay.h() / 2;
    let service_mat = if matches!(
        archetype,
        StoreArchetype::CafeRestaurant | StoreArchetype::ApparelBoutique
    ) {
        interior_door_mat
    } else {
        MaterialClass::SheetSteel
    };
    match front_side {
        CardinalEntry::North => {
            shell.put(mx, bay.y0, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
            if bay.w() >= 9 {
                shell.put(mx - 1, bay.y0, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
            }
            if rear_door {
                shell.put(mx, bay.y1, EdgeSide::North, WallEdgeKind::DoorClosed, service_mat);
            }
        }
        CardinalEntry::South => {
            shell.put(mx, bay.y1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
            if bay.w() >= 9 {
                shell.put(mx - 1, bay.y1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
            }
            if rear_door {
                shell.put(mx, bay.y0, EdgeSide::North, WallEdgeKind::DoorClosed, service_mat);
            }
        }
        CardinalEntry::West => {
            shell.put(bay.x0, my, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
            if bay.h() >= 9 {
                shell.put(bay.x0, my - 1, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
            }
            if rear_door {
                shell.put(bay.x1, my, EdgeSide::West, WallEdgeKind::DoorClosed, service_mat);
            }
        }
        CardinalEntry::East => {
            shell.put(bay.x1, my, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
            if bay.h() >= 9 {
                shell.put(bay.x1, my - 1, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Glass);
            }
            if rear_door {
                shell.put(bay.x0, my, EdgeSide::West, WallEdgeKind::DoorClosed, service_mat);
            }
        }
    }
}

/// Splits `bay` into `(sales_floor, back_rooms)` oriented away from `front_side`.
fn partition_sales_and_backroom(
    shell: &mut Shell,
    bay: LotBox,
    archetype: StoreArchetype,
    front_side: CardinalEntry,
) -> (Room, Vec<(Room, Role)>) {
    let mut back_rooms = Vec::new();
    let back_role = match archetype {
        StoreArchetype::CafeRestaurant => Role::BackKitchen,
        StoreArchetype::Electronics => Role::ServerCloset,
        StoreArchetype::Pharmacy => Role::Office,
        _ => Role::Storage,
    };
    let (horizontal_front, front_is_min) = match front_side {
        CardinalEntry::North => (true, true),
        CardinalEntry::South => (true, false),
        CardinalEntry::West => (false, true),
        CardinalEntry::East => (false, false),
    };
    let depth = if horizontal_front { bay.h() } else { bay.w() };
    let span = if horizontal_front { bay.w() } else { bay.h() };
    if depth < 7 || span < 5 {
        return (bay.as_room(), back_rooms);
    }

    let back_depth = if depth >= 11 { 3 } else { 2 };
    let (sales, back_strip) = if horizontal_front {
        let split_y = if front_is_min {
            bay.y1 - back_depth
        } else {
            bay.y0 + back_depth
        };
        let door_x = (bay.x0 + 1).min(bay.x1 - 1);
        shell.partition_h(split_y, bay.x0, bay.x1, &[door_x]);
        if front_is_min {
            (
                Room { x0: bay.x0, y0: bay.y0, x1: bay.x1, y1: split_y },
                Room { x0: bay.x0, y0: split_y, x1: bay.x1, y1: bay.y1 },
            )
        } else {
            (
                Room { x0: bay.x0, y0: split_y, x1: bay.x1, y1: bay.y1 },
                Room { x0: bay.x0, y0: bay.y0, x1: bay.x1, y1: split_y },
            )
        }
    } else {
        let split_x = if front_is_min {
            bay.x1 - back_depth
        } else {
            bay.x0 + back_depth
        };
        let door_y = (bay.y0 + 1).min(bay.y1 - 1);
        shell.partition_v(split_x, bay.y0, bay.y1, &[door_y]);
        if front_is_min {
            (
                Room { x0: bay.x0, y0: bay.y0, x1: split_x, y1: bay.y1 },
                Room { x0: split_x, y0: bay.y0, x1: bay.x1, y1: bay.y1 },
            )
        } else {
            (
                Room { x0: split_x, y0: bay.y0, x1: bay.x1, y1: bay.y1 },
                Room { x0: bay.x0, y0: bay.y0, x1: split_x, y1: bay.y1 },
            )
        }
    };

    // Split a restroom off the opposite corner of the back strip when wide enough.
    if horizontal_front && back_strip.w() >= 7 {
        let wc_split = back_strip.x1 - 3;
        shell.partition_v(wc_split, back_strip.y0, back_strip.y1, &[back_strip.y0]);
        back_rooms.push((
            Room { x0: back_strip.x0, y0: back_strip.y0, x1: wc_split, y1: back_strip.y1 },
            back_role,
        ));
        back_rooms.push((
            Room { x0: wc_split, y0: back_strip.y0, x1: back_strip.x1, y1: back_strip.y1 },
            Role::Bathroom,
        ));
    } else if !horizontal_front && back_strip.h() >= 7 {
        let wc_split = back_strip.y1 - 3;
        shell.partition_h(wc_split, back_strip.x0, back_strip.x1, &[back_strip.x0]);
        back_rooms.push((
            Room { x0: back_strip.x0, y0: back_strip.y0, x1: back_strip.x1, y1: wc_split },
            back_role,
        ));
        back_rooms.push((
            Room { x0: back_strip.x0, y0: wc_split, x1: back_strip.x1, y1: back_strip.y1 },
            Role::Bathroom,
        ));
    } else {
        back_rooms.push((back_strip, back_role));
    }

    (sales, back_rooms)
}

/// Checks whether a candidate interior prop is clear of blocking any adjacent cell against `edges`
/// and avoids covering front-door ingress lanes.
fn place_store_prop_checked(
    furn: &mut Furnisher,
    kind: PropKind,
    pos: (i32, i32),
    rot_deg: i32,
    len: Option<i32>,
    room: Room,
) -> bool {
    furn.try_place(kind, pos.0, pos.1, rot_deg, len, room).is_some()
}

/// Places a register / service counter bank inset `1–2m` from the storefront (or rear pharmacy counter).
fn place_checkout_bank(
    furn: &mut Furnisher,
    sales: Room,
    front_side: CardinalEntry,
    count: usize,
    len_cells: i32,
    rear_counter: bool,
) {
    let bank_side = if rear_counter {
        match front_side {
            CardinalEntry::North => CardinalEntry::South,
            CardinalEntry::South => CardinalEntry::North,
            CardinalEntry::West => CardinalEntry::East,
            CardinalEntry::East => CardinalEntry::West,
        }
    } else {
        front_side
    };
    let len = Some(len_cells.clamp(2, 4));
    match bank_side {
        CardinalEntry::North | CardinalEntry::South => {
            let cy = if bank_side == CardinalEntry::North {
                (sales.y0 + 1).min(sales.y1 - 2)
            } else {
                (sales.y1 - 2).max(sales.y0 + 1)
            };
            let start_x = sales.x0 + 1;
            let mut placed = 0usize;
            let mut cx = start_x;
            while cx + len_cells < sales.x1 && placed < count {
                if place_store_prop_checked(furn, PropKind::CheckoutCounter, (cx, cy), 0, len, sales) {
                    placed += 1;
                }
                cx += len_cells + 2;
            }
        }
        CardinalEntry::West | CardinalEntry::East => {
            let cx = if bank_side == CardinalEntry::West {
                (sales.x0 + 1).min(sales.x1 - 2)
            } else {
                (sales.x1 - 2).max(sales.x0 + 1)
            };
            let start_y = sales.y0 + 1;
            let mut placed = 0usize;
            let mut cy = start_y;
            while cy + len_cells < sales.y1 && placed < count {
                if place_store_prop_checked(furn, PropKind::CheckoutCounter, (cx, cy), 90, len, sales) {
                    placed += 1;
                }
                cy += len_cells + 2;
            }
        }
    }
}

/// Places parallel navigable [`PropKind::DisplayRack`] aisles inside `sales`, leaving a `1m` loop
/// around the room perimeter and between adjacent fixture rows.
fn place_display_aisles(
    furn: &mut Furnisher,
    sales: Room,
    along_x: bool,
    rack_len: i32,
    row_step: i32,
    rot_jitter: i32,
    seed: u64,
) {
    let mut jitter_rng = Rng(seed ^ 0xA151_E501);
    let inner = Room {
        x0: (sales.x0 + 1).min(sales.x1),
        y0: (sales.y0 + 1).min(sales.y1),
        x1: (sales.x1 - 1).max(sales.x0),
        y1: (sales.y1 - 1).max(sales.y0),
    };
    if inner.w() < 2 || inner.h() < 2 {
        return;
    }
    let rot_base = if along_x { 0 } else { 90 };
    if along_x {
        let step_y = row_step.max(2);
        let mut y = inner.y0 + 1;
        while y < inner.y1 - 1 {
            let mut x = inner.x0;
            while x + rack_len <= inner.x1 {
                let jitter = if rot_jitter > 0 {
                    jitter_rng.range(-rot_jitter, rot_jitter + 1)
                } else {
                    0
                };
                let rot = (rot_base + jitter).rem_euclid(360);
                let _ = place_store_prop_checked(
                    furn,
                    PropKind::DisplayRack,
                    (x, y),
                    rot,
                    Some(rack_len),
                    sales,
                );
                x += rack_len + 1;
            }
            y += step_y;
        }
    } else {
        let step_x = row_step.max(2);
        let mut x = inner.x0 + 1;
        while x < inner.x1 - 1 {
            let mut y = inner.y0;
            while y + rack_len <= inner.y1 {
                let jitter = if rot_jitter > 0 {
                    jitter_rng.range(-rot_jitter, rot_jitter + 1)
                } else {
                    0
                };
                let rot = (rot_base + jitter).rem_euclid(360);
                let _ = place_store_prop_checked(
                    furn,
                    PropKind::DisplayRack,
                    (x, y),
                    rot,
                    Some(rack_len),
                    sales,
                );
                y += rack_len + 1;
            }
            x += step_x;
        }
    }
}

/// Places structural columns on wide [`StoreArchetype::DepartmentAnchor`] sales floors.
fn place_anchor_pillars(furn: &mut Furnisher, sales: Room) {
    if sales.w() < 10 || sales.h() < 8 {
        return;
    }
    for py in ((sales.y0 + 3)..(sales.y1 - 2)).step_by(5) {
        for px in ((sales.x0 + 3)..(sales.x1 - 2)).step_by(5) {
            let _ = furn.try_place(PropKind::StructuralPillar, px, py, 0, None, sales);
        }
    }
}

/// Composes a furnished commercial retail bay (`StoreArchetype`) inside `bay` on ground floor `1F`.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn generate_store_unit(
    board_w: i32,
    board_h: i32,
    bay: LotBox,
    archetype: StoreArchetype,
    front_side: CardinalEntry,
    rear_door: bool,
    extra_reserves: &[(f32, f32)],
    seed: u64,
) -> FloorContent {
    let mut shell = Shell {
        w: board_w,
        h: board_h,
        rng: Rng(seed ^ ((archetype as u64 + 1).wrapping_mul(0xC033_E4C1))),
        edges: Vec::new(),
    };
    enclose_store_shell(&mut shell, bay, archetype, front_side, rear_door);
    let (sales, back_rooms) = partition_sales_and_backroom(&mut shell, bay, archetype, front_side);

    let mut furn = Furnisher::new(board_w, board_h, shell.rng.next());
    furn.reserve_doors(&shell.edges);
    for &(sx, sy) in extra_reserves {
        furn.reserve_cell(sx.floor() as i32, sy.floor() as i32);
    }

    // Populate back-of-house rooms first so stockroom shelving & prep counters stay intact.
    for (sub_room, role) in back_rooms {
        furn.furnish(role, sub_room);
        if role == Role::Storage {
            furn.scatter(PropKind::WoodCrate, sub_room, 2, 0);
        }
    }

    let along_x = matches!(front_side, CardinalEntry::West | CardinalEntry::East)
        || (sales.w() >= sales.h() + 2);
    match archetype {
        StoreArchetype::Grocery => {
            place_checkout_bank(&mut furn, sales, front_side, 2, 2, false);
            let _ = furn.against_any_wall(PropKind::SteelShelving, sales, Some(3));
            let len = if (along_x && sales.w() >= 10) || (!along_x && sales.h() >= 10) { 3 } else { 2 };
            place_display_aisles(&mut furn, sales, along_x, len, 2, 0, seed);
        }
        StoreArchetype::ApparelBoutique => {
            place_checkout_bank(&mut furn, sales, front_side, 1, 2, false);
            place_display_aisles(&mut furn, sales, along_x, 2, 3, 12, seed);
        }
        StoreArchetype::CafeRestaurant => {
            place_checkout_bank(&mut furn, sales, front_side, 1, 3, true);
            furn.furnish(Role::BarHall, sales);
        }
        StoreArchetype::Pharmacy => {
            place_checkout_bank(&mut furn, sales, front_side, 1, 3, true);
            place_display_aisles(&mut furn, sales, along_x, 2, 2, 0, seed);
            let _ = furn.against_any_wall(PropKind::SteelShelving, sales, Some(2));
        }
        StoreArchetype::Electronics => {
            place_checkout_bank(&mut furn, sales, front_side, 1, 2, true);
            place_display_aisles(&mut furn, sales, !along_x, 2, 3, 0, seed);
            let _ = furn.against_any_wall(PropKind::SteelShelving, sales, Some(2));
        }
        StoreArchetype::ConvenienceBodega => {
            place_checkout_bank(&mut furn, sales, front_side, 1, 2, false);
            place_display_aisles(&mut furn, sales, along_x, 2, 2, 6, seed);
        }
        StoreArchetype::DepartmentAnchor => {
            place_anchor_pillars(&mut furn, sales);
            place_checkout_bank(&mut furn, sales, front_side, 3, 2, false);
            let _ = furn.against_any_wall(PropKind::SteelShelving, sales, Some(4));
            place_display_aisles(&mut furn, sales, along_x, 3, 3, 0, seed);
        }
    }

    // Guarantee at least one iconic retail fixture even on compact sub-lots.
    if !furn
        .props
        .iter()
        .any(|p| matches!(p.kind, PropKind::DisplayRack | PropKind::CheckoutCounter))
    {
        let fallback_x = sales.x0 + (sales.w() / 2 - 1).max(0);
        let fallback_y = sales.y0 + (sales.h() / 2).max(0);
        let _ = furn.try_place(PropKind::CheckoutCounter, fallback_x, fallback_y, 0, Some(2), sales);
    }

    trim_floor_props_to_connected(board_w, board_h, bay, &shell.edges, &mut furn.props);

    let surfaces = vec![bay.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, seed as u32 ^ 0x5103)];
    FloorContent {
        wall_edges: shell.edges,
        props: furn.props,
        surfaces,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_gen_residential::floor_lot_connected;

    #[test]
    fn all_seven_store_archetypes_generate_connected_furnished_bays() {
        let entries = [
            CardinalEntry::North,
            CardinalEntry::South,
            CardinalEntry::West,
            CardinalEntry::East,
        ];
        for (idx, &arch) in StoreArchetype::ALL.iter().enumerate() {
            let entry = entries[idx % entries.len()];
            let compact = generate_store_unit(
                14,
                10,
                LotBox::new(2, 1, 12, 9),
                arch,
                entry,
                true,
                &[],
                900 + idx as u64,
            );
            assert!(
                compact.surfaces.iter().any(|s| s.kind == SurfaceFeatureKind::InteriorFloor),
                "{arch:?} should emit InteriorFloor surface"
            );
            assert!(
                compact.wall_edges.iter().any(|e| matches!(e.kind, WallEdgeKind::DoorOpen | WallEdgeKind::DoorClosed)),
                "{arch:?} should include doors"
            );
            assert!(
                compact
                    .props
                    .iter()
                    .any(|p| matches!(p.kind, PropKind::DisplayRack | PropKind::CheckoutCounter)),
                "{arch:?} should emit commercial DisplayRack or CheckoutCounter"
            );
            assert!(
                floor_lot_connected(14, 10, LotBox::new(2, 1, 12, 9), &compact.wall_edges, &compact.props),
                "{arch:?} compact bay must stay 100% connected"
            );

            let full = generate_store_unit(
                20,
                14,
                LotBox::new(2, 2, 18, 12),
                arch,
                entry,
                false,
                &[(2.5, 7.5), (17.5, 7.5)],
                1200 + idx as u64,
            );
            assert!(
                floor_lot_connected(20, 14, LotBox::new(2, 2, 18, 12), &full.wall_edges, &full.props),
                "{arch:?} 20x14 bay must stay 100% connected"
            );
        }
    }
}
