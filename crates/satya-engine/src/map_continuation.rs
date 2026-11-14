//! # Edge-Continuous Procedural Tactical Map Stitching (`map_continuation`)
//!
//! Expands an active [`GmMapUpdateRequest`] across a [`MapContinuationDir`] (`North`, `South`,
//! `East`, `West`) by procedurally generating one or more [`ProceduralSector`] strips whose boundary
//! seams align logically with the source edge (preserving existing perimeter walls/windows/doors,
//! carving passage doors into walled buildings/facilities, aligning surface ribbons such as
//! `CreekWater`, `DrainageDitch`, `AsphaltDrive`, `GravelPath`, `StonePath`, and `DirtPath`,
//! shifting existing walls/props/tokens when growing `North` or `West`, and verifying full
//! `0.5m`-lattice connectivity).

use satya_schema::{
    EdgeSide, GmMapUpdateRequest, MapContinuationDir, MapPropPlacement, MapWallEdge,
    MaterialClass, WallEdgeKind,
};

use crate::map_dressing::{PropObstruction, make_prop_at_cm, prop_spec};
use crate::tactical_map::{
    ProceduralSector, SectorPreset, build_procedural_map_request, generate_procedural_sector,
};
use crate::tactical_map_geom::{
    all_walkable_areas_connected, migrate_legacy_tiles_to_vector, prop_intersects_circle,
    TOKEN_RADIUS_M,
};

/// Default added width (cells = meters) when continuing East or West for compact `20x14` maps.
pub const CONTINUATION_STRIP_W: i32 = 20;
/// Default added height (cells = meters) when continuing North or South for compact `20x14` maps.
pub const CONTINUATION_STRIP_H: i32 = 14;
/// Maximum stitched arena width in cells (`1 cell = 1.0m` default).
pub const MAX_STITCHED_MAP_W: i32 = 240;
/// Maximum stitched arena height in cells (`1 cell = 1.0m` default).
pub const MAX_STITCHED_MAP_H: i32 = 180;

/// Computes the strip thickness (cells) added when continuing `map` along `dir`,
/// preserving `20`/`14` increments on compact `20x14` grids and scaling proportionally on larger sectors.
#[must_use]
pub fn continuation_strip_depth(map: &GmMapUpdateRequest, dir: MapContinuationDir) -> i32 {
    let remaining = match dir {
        MapContinuationDir::East | MapContinuationDir::West => {
            (MAX_STITCHED_MAP_W - map.grid_width).max(0)
        }
        MapContinuationDir::North | MapContinuationDir::South => {
            (MAX_STITCHED_MAP_H - map.grid_height).max(0)
        }
    };
    if remaining < crate::tactical_map::MIN_SECTOR_DIM {
        return 0;
    }
    let desired = if map.grid_width % CONTINUATION_STRIP_W == 0
        && map.grid_height % CONTINUATION_STRIP_H == 0
    {
        match dir {
            MapContinuationDir::East | MapContinuationDir::West => CONTINUATION_STRIP_W,
            MapContinuationDir::North | MapContinuationDir::South => CONTINUATION_STRIP_H,
        }
    } else {
        match dir {
            MapContinuationDir::East | MapContinuationDir::West => {
                (map.grid_height * 4 / 3).clamp(24, 36)
            }
            MapContinuationDir::North | MapContinuationDir::South => {
                (map.grid_width * 3 / 4).clamp(18, 28)
            }
        }
    };
    desired.min(remaining)
}

/// Returns `true` when extending `map` along `dir` stays within [`MAX_STITCHED_MAP_W`] × [`MAX_STITCHED_MAP_H`].
#[must_use]
pub fn can_continue_map(map: &GmMapUpdateRequest, dir: MapContinuationDir) -> bool {
    continuation_strip_depth(map, dir) >= crate::tactical_map::MIN_SECTOR_DIM
}

/// Result of stitching a procedurally generated continuation onto an existing battlemap.
#[derive(Debug, Clone, PartialEq)]
pub struct MapContinuationOutcome {
    pub map: GmMapUpdateRequest,
    /// Coordinate translation `(shift_x_m, shift_y_m)` applied to pre-existing map elements
    /// (`> 0` when continuing `West` or `North`; `(0.0, 0.0)` for `East` or `South`).
    pub existing_shift_m: (f32, f32),
    /// Dimensions `(added_w_cells, added_h_cells)` of the newly generated region.
    pub added_size_cells: (i32, i32),
}

/// Deterministic SplitMix64 seed derivation for sequential edge continuations.
#[must_use]
pub fn next_continuation_seed(
    prev_seed: i64,
    grid_w: i32,
    grid_h: i32,
    dir: MapContinuationDir,
) -> i64 {
    let dir_salt: u64 = match dir {
        MapContinuationDir::North => 0x9E37_79B9_7F4A_7C15,
        MapContinuationDir::East => 0xBF58_476D_1CE4_E5B9,
        MapContinuationDir::South => 0x94D0_49BB_1331_11EB,
        MapContinuationDir::West => 0xD6E8_FEB8_6659_FD93,
    };
    let mut z = (prev_seed as u64)
        ^ dir_salt
        ^ ((grid_w as u64).wrapping_mul(0x517C_C1B7_2722_0A95))
        ^ ((grid_h as u64).rotate_left(17));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)) as i64
}

/// Partitions a tangential boundary span of `total_cells` into natural sector chunks (`<= 48` cells each).
#[must_use]
pub fn partition_span_into_chunks(total_cells: i32, target_chunk: i32) -> Vec<(i32, i32)> {
    if total_cells <= 48 {
        return vec![(0, total_cells)];
    }
    let count = ((total_cells + target_chunk / 2) / target_chunk).max(2);
    let base = total_cells / count;
    let rem = total_cells % count;
    let mut out = Vec::with_capacity(count as usize);
    let mut offset = 0;
    for i in 0..count {
        let len = base + i32::from(i < rem);
        out.push((offset, len));
        offset += len;
    }
    out
}

use crate::map_continuation_ribbons::{
    RibbonAnchor, align_chunk_ribbons, clear_props_near, extract_seam_ribbons,
    mirror_sector_surfaces,
};

#[derive(Debug, Clone)]
struct BoundarySeamSlice {
    t0: i32,
    t_len: i32,
    edges: Vec<Option<(WallEdgeKind, MaterialClass)>>,
    src_clear: Vec<bool>,
    ribbons: Vec<RibbonAnchor>,
}

fn edge_slot_at(edges: &[MapWallEdge], x: i32, y: i32, side: EdgeSide) -> Option<(WallEdgeKind, MaterialClass)> {
    edges
        .iter()
        .rev()
        .find(|e| e.x == x && e.y == y && e.side == side)
        .map(|e| (e.kind, e.material))
}

pub(crate) fn upsert_edge(edges: &mut Vec<MapWallEdge>, e: MapWallEdge) {
    if let Some(slot) = edges
        .iter_mut()
        .find(|o| o.x == e.x && o.y == e.y && o.side == e.side)
    {
        *slot = e;
    } else {
        edges.push(e);
    }
}

fn remove_edge(edges: &mut Vec<MapWallEdge>, x: i32, y: i32, side: EdgeSide) {
    edges.retain(|e| !(e.x == x && e.y == y && e.side == side));
}

fn extract_seam_slice(
    map: &GmMapUpdateRequest,
    dir: MapContinuationDir,
    t0: i32,
    t_len: i32,
) -> BoundarySeamSlice {
    let (w, h) = (map.grid_width, map.grid_height);
    let mut edges = Vec::with_capacity(t_len as usize);
    let mut src_clear = Vec::with_capacity(t_len as usize);

    for dt in 0..t_len {
        let t = t0 + dt;
        let (ex, ey, side, cell_x, cell_y) = match dir {
            MapContinuationDir::North => (t, 0, EdgeSide::North, t as f32 + 0.5, 0.5),
            MapContinuationDir::South => (t, h, EdgeSide::North, t as f32 + 0.5, h as f32 - 0.5),
            MapContinuationDir::West => (0, t, EdgeSide::West, 0.5, t as f32 + 0.5),
            MapContinuationDir::East => (w, t, EdgeSide::West, w as f32 - 0.5, t as f32 + 0.5),
        };
        edges.push(edge_slot_at(&map.wall_edges, ex, ey, side));
        let blocked = map.props.iter().any(|p| {
            prop_spec(p.kind).obstruction == PropObstruction::FullObstacle
                && prop_intersects_circle(p, cell_x, cell_y, TOKEN_RADIUS_M)
        });
        src_clear.push(!blocked);
    }

    let ribbons = extract_seam_ribbons(map, dir, t0, t_len);
    BoundarySeamSlice { t0, t_len, edges, src_clear, ribbons }
}

pub(crate) fn translate_prop(p: MapPropPlacement, dx_cells: i32, dy_cells: i32, cell_cm: i32) -> MapPropPlacement {
    let mut out = p;
    out.x += dx_cells;
    out.y += dy_cells;
    let (cx_cm, cy_cm) = p.center_cm();
    out.center_x_cm = Some(cx_cm + dx_cells * cell_cm);
    out.center_y_cm = Some(cy_cm + dy_cells * cell_cm);
    out
}

pub(crate) fn mirror_edge_axis(e: &mut MapWallEdge, cw: i32, ch: i32, horizontal_flip: bool) {
    if horizontal_flip {
        match e.side {
            EdgeSide::West => e.x = cw - e.x,
            EdgeSide::North => e.x = cw - 1 - e.x,
        }
    } else {
        match e.side {
            EdgeSide::North => e.y = ch - e.y,
            EdgeSide::West => e.y = ch - 1 - e.y,
        }
    }
}

pub(crate) fn mirror_prop_axis(p: MapPropPlacement, cw: i32, ch: i32, horizontal_flip: bool) -> MapPropPlacement {
    let (cx_cm, cy_cm) = p.center_cm();
    let (sw_cm, sh_cm) = p.local_span_cm();
    let len_cm = prop_spec(p.kind).stretchable.then_some(sw_cm.max(sh_cm));
    if horizontal_flip {
        let rot = (360 - p.rotation_deg).rem_euclid(360);
        make_prop_at_cm(p.kind, cw * 100 - cx_cm, cy_cm, rot, len_cm, p.variant_seed)
    } else {
        let rot = (180 - p.rotation_deg).rem_euclid(360);
        make_prop_at_cm(p.kind, cx_cm, ch * 100 - cy_cm, rot, len_cm, p.variant_seed)
    }
}

fn mirror_sector(sec: &mut ProceduralSector, cw: i32, ch: i32, horizontal_flip: bool) {
    for e in &mut sec.wall_edges {
        mirror_edge_axis(e, cw, ch, horizontal_flip);
    }
    for p in &mut sec.props {
        *p = mirror_prop_axis(*p, cw, ch, horizontal_flip);
    }
    mirror_sector_surfaces(&mut sec.surfaces, cw, ch, horizontal_flip);
    crate::map_continuation_ribbons::mirror_floor_levels(&mut sec.floors, cw, ch, horizontal_flip);
}

#[derive(Clone, Copy)]
struct SeamPlacement {
    x_off: i32,
    y_off: i32,
    cw: i32,
    ch: i32,
    new_w: i32,
    new_h: i32,
    dir: MapContinuationDir,
}

fn reconcile_chunk_entry_seam(
    sec: &mut ProceduralSector,
    preset: SectorPreset,
    slice: &BoundarySeamSlice,
    seam: SeamPlacement,
    out_edges: &mut Vec<MapWallEdge>,
) {
    let SeamPlacement {
        x_off,
        y_off,
        cw,
        ch,
        new_w,
        new_h,
        dir,
    } = seam;
    let walled_shell = preset.has_structural_wall_edges()
        && !matches!(preset, SectorPreset::UrbanCrossroads | SectorPreset::GardenEstate);
    let door_mat = if preset.interior_style().is_some() {
        MaterialClass::SolidWood
    } else {
        MaterialClass::SheetSteel
    };

    // Strip the chunk's outer edges along its entry face (shared seam is authored coherently below).
    sec.wall_edges.retain(|e| match dir {
        MapContinuationDir::East => !(e.side == EdgeSide::West && e.x == 0),
        MapContinuationDir::West => !(e.side == EdgeSide::West && e.x == cw),
        MapContinuationDir::South => !(e.side == EdgeSide::North && e.y == 0),
        MapContinuationDir::North => !(e.side == EdgeSide::North && e.y == ch),
    });

    // Clear solid obstacles on the new chunk's side of any existing doorway on the source seam.
    let mut seam_has_door = false;
    for (dt, edge_opt) in slice.edges.iter().enumerate() {
        if let Some((kind, _)) = edge_opt
            && matches!(kind, WallEdgeKind::DoorOpen | WallEdgeKind::DoorClosed)
        {
            seam_has_door = true;
            let t_local = dt as f32 + 0.5;
            let (px, py) = match dir {
                MapContinuationDir::East => (0.5, t_local),
                MapContinuationDir::West => (cw as f32 - 0.5, t_local),
                MapContinuationDir::South => (t_local, 0.5),
                MapContinuationDir::North => (t_local, ch as f32 - 0.5),
            };
            clear_props_near(&mut sec.props, px, py, 1.20);
        }
    }

    // If a walled interior/facility expands across a doorless edge (typically North or South),
    // pick the cleanest walkable boundary slot on the existing side and cut a connecting doorway.
    if walled_shell && !seam_has_door && slice.t_len >= 3 {
        let center = slice.t_len / 2;
        let pick_dt = (1..(slice.t_len - 1))
            .min_by_key(|&dt| {
                let clear_score = i32::from(!slice.src_clear[dt as usize]) * 100;
                let mat_score = match slice.edges[dt as usize].map(|(k, _)| k) {
                    Some(WallEdgeKind::Window) => 2,
                    Some(WallEdgeKind::Wall) | None => 0,
                    _ => 1,
                };
                clear_score + mat_score * 5 + (dt - center).abs()
            })
            .unwrap_or(center);
        let t_abs = slice.t0 + pick_dt;
        let (seam_x, seam_y, seam_side) = match dir {
            MapContinuationDir::East => (x_off, t_abs, EdgeSide::West),
            MapContinuationDir::West => (cw, t_abs, EdgeSide::West),
            MapContinuationDir::South => (t_abs, y_off, EdgeSide::North),
            MapContinuationDir::North => (t_abs, ch, EdgeSide::North),
        };
        upsert_edge(
            out_edges,
            MapWallEdge {
                x: seam_x,
                y: seam_y,
                side: seam_side,
                kind: WallEdgeKind::DoorClosed,
                material: door_mat,
            },
        );
        let t_local = pick_dt as f32 + 0.5;
        let (px, py) = match dir {
            MapContinuationDir::East => (0.5, t_local),
            MapContinuationDir::West => (cw as f32 - 0.5, t_local),
            MapContinuationDir::South => (t_local, 0.5),
            MapContinuationDir::North => (t_local, ch as f32 - 0.5),
        };
        clear_props_near(&mut sec.props, px, py, 1.25);
        // If a perpendicular interior partition inside the new chunk touches the threshold directly,
        // ensure at least one flanking cell opens into the room.
        if let Some((nx_adj, ny_adj)) = match dir {
            MapContinuationDir::East => Some((0, pick_dt)),
            MapContinuationDir::West => Some((cw - 1, pick_dt)),
            MapContinuationDir::South => Some((pick_dt, 0)),
            MapContinuationDir::North => Some((pick_dt, ch - 1)),
        } {
            let perp = if matches!(dir, MapContinuationDir::East | MapContinuationDir::West) {
                EdgeSide::North
            } else {
                EdgeSide::West
            };
            remove_edge(&mut sec.wall_edges, nx_adj, ny_adj, perp);
        }
    }

    // Clamp or discard any stray out-of-bounds edges just in case.
    sec.wall_edges.retain(|e| match e.side {
        EdgeSide::North => x_off + e.x >= 0 && x_off + e.x < new_w && y_off + e.y >= 0 && y_off + e.y <= new_h,
        EdgeSide::West => x_off + e.x >= 0 && x_off + e.x <= new_w && y_off + e.y >= 0 && y_off + e.y < new_h,
    });
}

fn ensure_seam_connectivity(
    w: i32,
    h: i32,
    edges: &mut Vec<MapWallEdge>,
    props: &mut Vec<MapPropPlacement>,
    preset: SectorPreset,
    dir: MapContinuationDir,
    seam_coord: i32,
) {
    if all_walkable_areas_connected(w, h, edges, props) {
        return;
    }
    let door_mat = if preset.interior_style().is_some() {
        MaterialClass::SolidWood
    } else {
        MaterialClass::SheetSteel
    };
    let span = if matches!(dir, MapContinuationDir::East | MapContinuationDir::West) { h } else { w };
    // Try opening/adding doors along the seam where blocking walls exist until connected.
    for step in 0..span {
        let t = (span / 2 + if step % 2 == 0 { step / 2 } else { -(step + 1) / 2 }).clamp(0, span - 1);
        let (x, y, side, px_a, py_a, px_b, py_b) = match dir {
            MapContinuationDir::East | MapContinuationDir::West => (
                seam_coord,
                t,
                EdgeSide::West,
                seam_coord as f32 - 0.5,
                t as f32 + 0.5,
                seam_coord as f32 + 0.5,
                t as f32 + 0.5,
            ),
            MapContinuationDir::North | MapContinuationDir::South => (
                t,
                seam_coord,
                EdgeSide::North,
                t as f32 + 0.5,
                seam_coord as f32 - 0.5,
                t as f32 + 0.5,
                seam_coord as f32 + 0.5,
            ),
        };
        if let Some((kind, _)) = edge_slot_at(edges, x, y, side)
            && matches!(kind, WallEdgeKind::Wall | WallEdgeKind::Window)
        {
            clear_props_near(props, px_a, py_a, 1.15);
            clear_props_near(props, px_b, py_b, 1.15);
            upsert_edge(edges, MapWallEdge { x, y, side, kind: WallEdgeKind::DoorClosed, material: door_mat });
            if all_walkable_areas_connected(w, h, edges, props) {
                return;
            }
        }
    }
    // Fallback: remove blocking FullObstacle props one by one if interior furniture blocked a narrow room.
    while !all_walkable_areas_connected(w, h, edges, props) {
        match props
            .iter()
            .rposition(|p| prop_spec(p.kind).obstruction == PropObstruction::FullObstacle)
        {
            Some(i) => {
                props.remove(i);
            }
            None => break,
        }
    }
}

/// Procedurally generates an edge-matched sector strip in `dir` and stitches it seamlessly onto `existing`.
pub fn continue_procedural_map(
    existing: &GmMapUpdateRequest,
    dir: MapContinuationDir,
    explicit_seed: Option<i64>,
) -> Result<MapContinuationOutcome, &'static str> {
    let old_w = existing.grid_width;
    let old_h = existing.grid_height;
    if old_w < 8 || old_h < 8 {
        return Err("Active tactical map is too small to continue procedurally");
    }
    if !can_continue_map(existing, dir) {
        return Err("Tactical map has reached the maximum stitched arena size in this direction");
    }

    let mut base = existing.clone();
    if !base.tiles.is_empty() {
        migrate_legacy_tiles_to_vector(
            &base.tiles,
            old_w,
            old_h,
            &mut base.wall_edges,
            &mut base.props,
        );
        base.tiles.clear();
    }

    let preset = base
        .theme_preset
        .as_deref()
        .and_then(SectorPreset::from_code_str)
        .unwrap_or_default();
    let seed = explicit_seed
        .unwrap_or_else(|| next_continuation_seed(base.seed.unwrap_or(42), old_w, old_h, dir));
    let cell_cm = base.cell_size_cm.max(25);
    let cell_m = cell_cm as f32 / 100.0;

    let strip_depth = continuation_strip_depth(existing, dir);
    let ((new_w, new_h), (dx_cells, dy_cells), (added_w, added_h), chunks) = match dir {
        MapContinuationDir::East => (
            (old_w + strip_depth, old_h),
            (0, 0),
            (strip_depth, old_h),
            partition_span_into_chunks(old_h, 26),
        ),
        MapContinuationDir::West => (
            (old_w + strip_depth, old_h),
            (strip_depth, 0),
            (strip_depth, old_h),
            partition_span_into_chunks(old_h, 26),
        ),
        MapContinuationDir::South => (
            (old_w, old_h + strip_depth),
            (0, 0),
            (old_w, strip_depth),
            partition_span_into_chunks(old_w, 32),
        ),
        MapContinuationDir::North => (
            (old_w, old_h + strip_depth),
            (0, strip_depth),
            (old_w, strip_depth),
            partition_span_into_chunks(old_w, 32),
        ),
    };

    let prev_active_floor = base.active_floor;
    if base.has_multiple_floors() {
        base.sync_active_floor_snapshot();
        if prev_active_floor != 0 {
            base.switch_active_floor(0);
        }
    }

    // Snapshot boundary features on `base` before translating coordinates.
    let seam_slices: Vec<BoundarySeamSlice> = chunks
        .iter()
        .map(|&(t0, t_len)| extract_seam_slice(&base, dir, t0, t_len))
        .collect();

    // Preserve continuous token cm positions before resizing.
    let prev_token_cm: Vec<(String, i32, i32)> = base
        .tokens
        .iter()
        .map(|t| {
            let (tx, ty) = base.token_pos_cm(t);
            (t.entity_id.clone(), tx, ty)
        })
        .collect();

    base.grid_width = new_w;
    base.grid_height = new_h;
    base.seed = Some(seed);
    base.theme_preset = Some(preset.as_str().to_string());

    for e in &mut base.wall_edges {
        e.x += dx_cells;
        e.y += dy_cells;
    }
    for p in &mut base.props {
        *p = translate_prop(*p, dx_cells, dy_cells, cell_cm);
    }
    for s in &mut base.surfaces {
        *s = s.translated(dx_cells * cell_cm, dy_cells * cell_cm);
    }
    crate::map_continuation_ribbons::translate_floor_levels(
        &mut base.floors,
        dx_cells,
        dy_cells,
        cell_cm,
    );
    for (entity_id, old_x_cm, old_y_cm) in prev_token_cm {
        base.set_token_pos_cm(
            &entity_id,
            old_x_cm + dx_cells * cell_cm,
            old_y_cm + dy_cells * cell_cm,
        );
    }

    let mut new_floor_props = Vec::new();
    let mut new_solid_props = Vec::new();
    for (idx, slice) in seam_slices.iter().enumerate() {
        let chunk_seed = next_continuation_seed(seed, idx as i32 + 1, slice.t_len, dir);
        let (x_off, y_off, cw, ch) = match dir {
            MapContinuationDir::East => (old_w, slice.t0, strip_depth, slice.t_len),
            MapContinuationDir::West => (0, slice.t0, strip_depth, slice.t_len),
            MapContinuationDir::South => (slice.t0, old_h, slice.t_len, strip_depth),
            MapContinuationDir::North => (slice.t0, 0, slice.t_len, strip_depth),
        };
        let mut sec = generate_procedural_sector(preset, cw, ch, chunk_seed);
        align_chunk_ribbons(&mut sec, preset, &slice.ribbons, cw, ch, dir, mirror_sector);
        reconcile_chunk_entry_seam(
            &mut sec,
            preset,
            slice,
            SeamPlacement {
                x_off,
                y_off,
                cw,
                ch,
                new_w,
                new_h,
                dir,
            },
            &mut base.wall_edges,
        );
        for mut e in sec.wall_edges {
            e.x += x_off;
            e.y += y_off;
            upsert_edge(&mut base.wall_edges, e);
        }
        for s in sec.surfaces {
            base.surfaces.push(s.translated(x_off * 100, y_off * 100));
        }
        for p in sec.props {
            let shifted = translate_prop(p, x_off, y_off, 100);
            if prop_spec(shifted.kind).floor_layer {
                new_floor_props.push(shifted);
            } else {
                new_solid_props.push(shifted);
            }
        }
        crate::map_continuation_ribbons::stitch_continued_floors(
            &mut base,
            &sec.floors,
            x_off,
            y_off,
            cell_cm,
        );
    }

    // Interleave surface decals underneath existing solid props so rendering Z-order stays crisp.
    let first_solid = base
        .props
        .iter()
        .position(|p| !prop_spec(p.kind).floor_layer)
        .unwrap_or(base.props.len());
    base.props.splice(first_solid..first_solid, new_floor_props);
    base.props.extend(new_solid_props);

    let seam_coord = match dir {
        MapContinuationDir::East => old_w,
        MapContinuationDir::West => strip_depth,
        MapContinuationDir::South => old_h,
        MapContinuationDir::North => strip_depth,
    };
    ensure_seam_connectivity(new_w, new_h, &mut base.wall_edges, &mut base.props, preset, dir, seam_coord);

    // Also verify sibling seams when the continued strip spans > 1 chunk.
    for slice in seam_slices.iter().skip(1) {
        let sib_dir = if matches!(dir, MapContinuationDir::East | MapContinuationDir::West) {
            MapContinuationDir::South
        } else {
            MapContinuationDir::East
        };
        ensure_seam_connectivity(new_w, new_h, &mut base.wall_edges, &mut base.props, preset, sib_dir, slice.t0);
    }

    if !base.floors.is_empty() {
        if let Some(f0) = base.floors.iter_mut().find(|f| f.floor_index == 0) {
            f0.wall_edges = base.wall_edges.clone();
            f0.props = base.props.clone();
            f0.surfaces = base.surfaces.clone();
        }
        base.floors.sort_by_key(|f| f.floor_index);
        if prev_active_floor != 0 {
            base.switch_active_floor(prev_active_floor);
        }
    }

    base.notes = Some(format!(
        "{} · Continuous Metric Arena ({}m x {}m, Seed {})",
        preset.subtitle(),
        new_w,
        new_h,
        seed
    ));

    Ok(MapContinuationOutcome {
        map: base,
        existing_shift_m: (dx_cells as f32 * cell_m, dy_cells as f32 * cell_m),
        added_size_cells: (added_w, added_h),
    })
}

/// Ensures an initialized starter map exists when `active_map` is empty, returning a vector [`GmMapUpdateRequest`].
#[must_use]
pub fn default_starter_vector_map(seed: i64) -> GmMapUpdateRequest {
    build_procedural_map_request(
        "map-starter",
        SectorPreset::default(),
        None,
        CONTINUATION_STRIP_W,
        CONTINUATION_STRIP_H,
        seed,
    )
}
