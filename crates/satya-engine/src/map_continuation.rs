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
    MaterialClass, PropKind, WallEdgeKind,
};

use crate::map_dressing::{PropObstruction, make_prop_at_cm, prop_spec};
use crate::tactical_map::{
    ProceduralSector, SectorPreset, build_procedural_map_request, generate_procedural_sector,
};
use crate::tactical_map_geom::{
    all_walkable_areas_connected, migrate_legacy_tiles_to_vector, prop_intersects_circle,
    TOKEN_RADIUS_M,
};

/// Default added width (cells = meters) when continuing East or West.
pub const CONTINUATION_STRIP_W: i32 = 20;
/// Default added height (cells = meters) when continuing North or South.
pub const CONTINUATION_STRIP_H: i32 = 14;
/// Maximum stitched arena width in cells (`1 cell = 1.0m` default).
pub const MAX_STITCHED_MAP_W: i32 = 80;
/// Maximum stitched arena height in cells (`1 cell = 1.0m` default).
pub const MAX_STITCHED_MAP_H: i32 = 56;

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

/// Returns whether `map` can expand further in `dir` within the stitched arena cap.
#[must_use]
pub fn can_continue_map(map: &GmMapUpdateRequest, dir: MapContinuationDir) -> bool {
    match dir {
        MapContinuationDir::East | MapContinuationDir::West => {
            map.grid_width + CONTINUATION_STRIP_W <= MAX_STITCHED_MAP_W
        }
        MapContinuationDir::North | MapContinuationDir::South => {
            map.grid_height + CONTINUATION_STRIP_H <= MAX_STITCHED_MAP_H
        }
    }
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

/// Partitions a tangential boundary span of `total_cells` into natural sector chunks (`<= 28` cells each).
#[must_use]
pub fn partition_span_into_chunks(total_cells: i32, target_chunk: i32) -> Vec<(i32, i32)> {
    if total_cells <= 28 {
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

const fn is_continuous_ribbon_prop(kind: PropKind) -> bool {
    matches!(
        kind,
        PropKind::CreekWater
            | PropKind::DrainageDitch
            | PropKind::AsphaltDrive
            | PropKind::GravelPath
            | PropKind::StonePath
            | PropKind::DirtPath
            | PropKind::ElevatedCatwalk
    )
}

#[derive(Debug, Clone)]
struct RibbonAnchor {
    kind: PropKind,
    local_t_m: f32,
    span_m: f32,
}

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

fn upsert_edge(edges: &mut Vec<MapWallEdge>, e: MapWallEdge) {
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
    let mut ribbons = Vec::new();

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

    // Detect surface ribbons touching the exited boundary inside [t0..t0 + t_len].
    for p in &map.props {
        if !is_continuous_ribbon_prop(p.kind) {
            continue;
        }
        let fp = p.footprint_m();
        let (norm_dist, tang_coord) = match dir {
            MapContinuationDir::North => (fp.cy_m - fp.hy_m.max(0.5), fp.cx_m),
            MapContinuationDir::South => (h as f32 - (fp.cy_m + fp.hy_m.max(0.5)), fp.cx_m),
            MapContinuationDir::West => (fp.cx_m - fp.hx_m.max(0.5), fp.cy_m),
            MapContinuationDir::East => (w as f32 - (fp.cx_m + fp.hx_m.max(0.5)), fp.cy_m),
        };
        if norm_dist <= 1.25 && tang_coord >= t0 as f32 - 0.5 && tang_coord <= (t0 + t_len) as f32 + 0.5 {
            let local_t_m = (tang_coord - t0 as f32).clamp(0.5, t_len as f32 - 0.5);
            let tang_span = match dir {
                MapContinuationDir::North | MapContinuationDir::South => (fp.hx_m * 2.0).clamp(1.0, 5.0),
                MapContinuationDir::East | MapContinuationDir::West => (fp.hy_m * 2.0).clamp(1.0, 5.0),
            };
            let dup = ribbons.iter().any(|r: &RibbonAnchor| {
                r.kind == p.kind && (r.local_t_m - local_t_m).abs() < 1.25
            });
            if !dup {
                ribbons.push(RibbonAnchor { kind: p.kind, local_t_m, span_m: tang_span });
            }
        }
    }

    BoundarySeamSlice { t0, t_len, edges, src_clear, ribbons }
}

fn translate_prop(p: MapPropPlacement, dx_cells: i32, dy_cells: i32, cell_cm: i32) -> MapPropPlacement {
    let mut out = p;
    out.x += dx_cells;
    out.y += dy_cells;
    let (cx_cm, cy_cm) = p.center_cm();
    out.center_x_cm = Some(cx_cm + dx_cells * cell_cm);
    out.center_y_cm = Some(cy_cm + dy_cells * cell_cm);
    out
}

fn mirror_prop_axis(p: MapPropPlacement, cw: i32, ch: i32, horizontal_flip: bool) -> MapPropPlacement {
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
    for p in &mut sec.props {
        *p = mirror_prop_axis(*p, cw, ch, horizontal_flip);
    }
}

fn find_chunk_entry_ribbon_coord(
    sec: &ProceduralSector,
    kind: PropKind,
    cw: i32,
    ch: i32,
    entry_dir: MapContinuationDir,
) -> Option<f32> {
    let horizontal_seam = matches!(entry_dir, MapContinuationDir::North | MapContinuationDir::South);
    sec.props
        .iter()
        .filter(|p| p.kind == kind)
        .filter_map(|p| {
            let fp = p.footprint_m();
            let norm_dist = match entry_dir {
                MapContinuationDir::South => ch as f32 - (fp.cy_m + fp.hy_m.max(0.5)),
                MapContinuationDir::North => fp.cy_m - fp.hy_m.max(0.5),
                MapContinuationDir::East => cw as f32 - (fp.cx_m + fp.hx_m.max(0.5)),
                MapContinuationDir::West => fp.cx_m - fp.hx_m.max(0.5),
            };
            (norm_dist <= 1.25).then_some(if horizontal_seam { fp.cx_m } else { fp.cy_m })
        })
        .next()
}

fn clear_props_near(props: &mut Vec<MapPropPlacement>, px_m: f32, py_m: f32, radius_m: f32) {
    props.retain(|p| {
        prop_spec(p.kind).floor_layer || !prop_intersects_circle(p, px_m, py_m, radius_m)
    });
}

fn align_chunk_ribbons(
    sec: &mut ProceduralSector,
    preset: SectorPreset,
    slice: &BoundarySeamSlice,
    cw: i32,
    ch: i32,
    dir: MapContinuationDir,
) {
    if slice.ribbons.is_empty() {
        return;
    }
    let horizontal_seam = matches!(dir, MapContinuationDir::North | MapContinuationDir::South);
    let entry_side = dir.opposite();
    let allow_mirror = preset.outdoor_style().is_some() || preset == SectorPreset::UrbanCrossroads;

    // Optional tangent reflection if it brings the primary linear ribbon into closer alignment.
    if allow_mirror
        && let Some(primary) = slice.ribbons.first()
        && let Some(dst_t) = find_chunk_entry_ribbon_coord(sec, primary.kind, cw, ch, entry_side)
    {
        let span_max = if horizontal_seam { cw as f32 } else { ch as f32 };
        let flipped_t = span_max - dst_t;
        if (flipped_t - primary.local_t_m).abs() + 0.5 < (dst_t - primary.local_t_m).abs() {
            mirror_sector(sec, cw, ch, horizontal_seam);
        }
    }

    // Stamp smooth connecting surface tiles from the seam entry into the chunk's matching ribbon (or 2m inward).
    let mut additions = Vec::new();
    for anchor in &slice.ribbons {
        let dst_t = find_chunk_entry_ribbon_coord(sec, anchor.kind, cw, ch, entry_side)
            .unwrap_or(anchor.local_t_m);
        let seam_cell_n = match entry_side {
            MapContinuationDir::North => 0.5,
            MapContinuationDir::South => ch as f32 - 0.5,
            MapContinuationDir::West => 0.5,
            MapContinuationDir::East => cw as f32 - 0.5,
        };
        let inward = match entry_side {
            MapContinuationDir::North | MapContinuationDir::West => 1.0,
            MapContinuationDir::South | MapContinuationDir::East => -1.0,
        };
        let steps = ((dst_t - anchor.local_t_m).abs().ceil() as i32 + 2).clamp(2, 6);
        for i in 0..steps {
            let frac = i as f32 / (steps - 1).max(1) as f32;
            let tang = anchor.local_t_m + (dst_t - anchor.local_t_m) * frac;
            let norm = seam_cell_n + inward * (frac * 1.5);
            let (wx, wy) = if horizontal_seam { (tang, norm) } else { (norm, tang) };
            let rot = if horizontal_seam { 0 } else { 90 };
            let len_cm = (anchor.span_m * 100.0).round() as i32;
            additions.push(make_prop_at_cm(
                anchor.kind,
                (wx * 100.0).round() as i32,
                (wy * 100.0).round() as i32,
                rot,
                Some(len_cm.clamp(100, 500)),
                0,
            ));
            clear_props_near(&mut sec.props, wx, wy, 0.75);
        }
    }
    // Insert surface decals before solid props so furniture/trees render on top.
    let insert_at = sec
        .props
        .iter()
        .position(|p| !prop_spec(p.kind).floor_layer)
        .unwrap_or(sec.props.len());
    sec.props.splice(insert_at..insert_at, additions);
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

    let ((new_w, new_h), (dx_cells, dy_cells), (added_w, added_h), chunks) = match dir {
        MapContinuationDir::East => (
            (old_w + CONTINUATION_STRIP_W, old_h),
            (0, 0),
            (CONTINUATION_STRIP_W, old_h),
            partition_span_into_chunks(old_h, CONTINUATION_STRIP_H),
        ),
        MapContinuationDir::West => (
            (old_w + CONTINUATION_STRIP_W, old_h),
            (CONTINUATION_STRIP_W, 0),
            (CONTINUATION_STRIP_W, old_h),
            partition_span_into_chunks(old_h, CONTINUATION_STRIP_H),
        ),
        MapContinuationDir::South => (
            (old_w, old_h + CONTINUATION_STRIP_H),
            (0, 0),
            (old_w, CONTINUATION_STRIP_H),
            partition_span_into_chunks(old_w, CONTINUATION_STRIP_W),
        ),
        MapContinuationDir::North => (
            (old_w, old_h + CONTINUATION_STRIP_H),
            (0, CONTINUATION_STRIP_H),
            (old_w, CONTINUATION_STRIP_H),
            partition_span_into_chunks(old_w, CONTINUATION_STRIP_W),
        ),
    };

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
            MapContinuationDir::East => (old_w, slice.t0, CONTINUATION_STRIP_W, slice.t_len),
            MapContinuationDir::West => (0, slice.t0, CONTINUATION_STRIP_W, slice.t_len),
            MapContinuationDir::South => (slice.t0, old_h, slice.t_len, CONTINUATION_STRIP_H),
            MapContinuationDir::North => (slice.t0, 0, slice.t_len, CONTINUATION_STRIP_H),
        };
        let mut sec = generate_procedural_sector(preset, cw, ch, chunk_seed);
        align_chunk_ribbons(&mut sec, preset, slice, cw, ch, dir);
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
        for p in sec.props {
            let shifted = translate_prop(p, x_off, y_off, 100);
            if prop_spec(shifted.kind).floor_layer {
                new_floor_props.push(shifted);
            } else {
                new_solid_props.push(shifted);
            }
        }
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
        MapContinuationDir::West => CONTINUATION_STRIP_W,
        MapContinuationDir::South => old_h,
        MapContinuationDir::North => CONTINUATION_STRIP_H,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn continue_all_presets_across_all_four_cardinal_edges() {
        for preset in SectorPreset::ALL {
            let base = build_procedural_map_request("m-base", preset, None, 20, 14, 42);
            let base_tok_pos = base.token_pos_cm(&base.tokens[0]);
            for dir in MapContinuationDir::ALL {
                let outcome = continue_procedural_map(&base, dir, None)
                    .unwrap_or_else(|e| panic!("{preset:?} {dir:?} failed: {e}"));
                let expected_size = match dir {
                    MapContinuationDir::East | MapContinuationDir::West => (40, 14),
                    MapContinuationDir::North | MapContinuationDir::South => (20, 28),
                };
                assert_eq!(
                    (outcome.map.grid_width, outcome.map.grid_height),
                    expected_size,
                    "{preset:?} {dir:?}"
                );
                assert_eq!(outcome.map.map_id, "m-base");
                // Existing token position stays pinned relative to existing geometry.
                let new_tok_pos = outcome.map.token_pos_cm(&outcome.map.tokens[0]);
                let expect_dx_cm = (outcome.existing_shift_m.0 * 100.0).round() as i32;
                let expect_dy_cm = (outcome.existing_shift_m.1 * 100.0).round() as i32;
                assert_eq!(
                    new_tok_pos,
                    (base_tok_pos.0 + expect_dx_cm, base_tok_pos.1 + expect_dy_cm),
                    "{preset:?} {dir:?} token shift"
                );
                // Zero duplicate edge keys.
                let mut seen_edges = BTreeSet::new();
                for e in &outcome.map.wall_edges {
                    assert!(
                        seen_edges.insert((e.x, e.y, e.side as u8)),
                        "{preset:?} {dir:?}: duplicate edge {:?}",
                        e
                    );
                }
                assert!(
                    all_walkable_areas_connected(
                        outcome.map.grid_width,
                        outcome.map.grid_height,
                        &outcome.map.wall_edges,
                        &outcome.map.props
                    ),
                    "{preset:?} {dir:?} disconnected after continuation"
                );
            }
        }
    }

    #[test]
    fn walled_north_south_adds_seam_doorway_and_outdoor_ribbons_continue() {
        let apt = build_procedural_map_request(
            "apt",
            SectorPreset::ResidentialApartment,
            None,
            20,
            14,
            42,
        );
        let south = continue_procedural_map(&apt, MapContinuationDir::South, None).unwrap();
        let has_seam_door = south.map.wall_edges.iter().any(|e| {
            e.side == EdgeSide::North
                && e.y == 14
                && matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen)
        });
        assert!(has_seam_door, "South continuation on Apartment must carve a connecting doorway");

        let creek = build_procedural_map_request("creek", SectorPreset::WoodlandCreek, None, 20, 14, 42);
        let creek_s = continue_procedural_map(&creek, MapContinuationDir::South, None).unwrap();
        let crosses_south_seam = creek_s.map.props.iter().any(|p| {
            p.kind == PropKind::CreekWater
                && p.center_m().1 >= 13.0
                && p.center_m().1 <= 15.6
        });
        assert!(crosses_south_seam, "WoodlandCreek South continuation should bridge creek across y=14");

        let park = build_procedural_map_request("park", SectorPreset::WoodedPark, None, 20, 14, 42);
        let park_e = continue_procedural_map(&park, MapContinuationDir::East, None).unwrap();
        let bridges_east_stone = park_e.map.props.iter().any(|p| {
            p.kind == PropKind::StonePath
                && p.center_m().0 >= 19.0
                && p.center_m().0 <= 21.8
        });
        assert!(bridges_east_stone, "WoodedPark East continuation should bridge stone trail across x=20");
    }

    #[test]
    fn multi_step_l_and_full_stitch_respects_max_arena_bounds() {
        let mut map = build_procedural_map_request("multi", SectorPreset::UrbanCrossroads, None, 20, 14, 42);
        map = continue_procedural_map(&map, MapContinuationDir::East, None).unwrap().map;
        assert_eq!((map.grid_width, map.grid_height), (40, 14));
        map = continue_procedural_map(&map, MapContinuationDir::North, None).unwrap().map;
        assert_eq!((map.grid_width, map.grid_height), (40, 28));
        assert!(all_walkable_areas_connected(
            map.grid_width,
            map.grid_height,
            &map.wall_edges,
            &map.props
        ));
        map = continue_procedural_map(&map, MapContinuationDir::East, None).unwrap().map;
        map = continue_procedural_map(&map, MapContinuationDir::East, None).unwrap().map;
        assert_eq!(map.grid_width, 80);
        assert!(!can_continue_map(&map, MapContinuationDir::East));
        assert!(continue_procedural_map(&map, MapContinuationDir::East, None).is_err());
    }
}
