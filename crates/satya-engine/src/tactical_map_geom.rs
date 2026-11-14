//! # Continuous 2D Tactical Map Geometry, Collision & Point Movement
//!
//! Replaces fixed `1m` tile stamping with exact continuous metric geometry (`1 cell = 1.0m = 100 cm`):
//! - [`TOKEN_RADIUS_M`] (`0.30m`) miniature discs with cover/wall slide-and-snap movement resolution,
//! - full `360°` oriented bounding boxes, compound `L`/`T` desks, and radial props,
//! - analytical 2D ray/segment intersections against wall edges and props (including vehicle engine bays
//!   and tree trunks vs. foliage canopies).
//!
//! Legacy tile migration, connectivity checks, and hover inspection live in [`crate::tactical_map_inspect`].

pub use crate::tactical_map_inspect::*;
pub use crate::tactical_map_spatial::*;

use satya_schema::{
    EdgeSide, GmMapUpdateRequest, MapPropPlacement, MapWallEdge, MaterialClass, PropKind,
    WallEdgeKind,
};

use crate::map_dressing::{prop_spec, rotated_aabb_half_extents_cm, PropObstruction};

/// Physical collision radius of a standing miniature on the metric battlemap.
pub const TOKEN_RADIUS_M: f32 = 0.30;
/// Generous hover/click pick radius for selecting or inspecting a miniature.
pub const TOKEN_PICK_RADIUS_M: f32 = 0.42;
/// Half-thickness of a structural wall, window, or door edge (`14 cm` total thickness).
pub const EDGE_HALF_THICK_M: f32 = 0.07;
/// Generous hover/click half-thickness for picking or inspecting a thin wall edge.
pub const EDGE_PICK_HALF_THICK_M: f32 = 0.18;
/// Sampling step (`0.25m`) for continuous any-angle gait reachability and connectivity checks.
pub const NAV_STEP_M: f32 = 0.25;
/// Maximum slide/snap displacement when clicking directly adjacent to cover or a wall.
pub const COVER_SNAP_MAX_M: f32 = 0.35;

/// Continuous ray hit against a prop placement (`t_enter`, `t_exit` in `[0.0, 1.0]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropRayHit {
    pub t_enter: f32,
    pub t_exit: f32,
    pub thickness_m: f32,
    pub material: MaterialClass,
    pub height_cm: i32,
    pub obstruction: PropObstruction,
    pub is_engine_bay: bool,
    pub is_tree_trunk: bool,
}

/// Local coordinate shape primitives shared between point hit-testing, circle push, and raycasts.
#[derive(Debug, Clone, Copy)]
enum LocalShape {
    Obb { cx: f32, cy: f32, hx: f32, hy: f32 },
    Circle { cx: f32, cy: f32, r: f32 },
}

fn vehicle_hood_y_min_local(p: &MapPropPlacement) -> f32 {
    let fp = p.footprint_m();
    let hood_depth = (fp.span_h_m * 0.22).max(0.85).min(fp.span_w_m * 0.65);
    fp.hy_m - hood_depth
}

/// Returns the solid local sub-shapes (in meters relative to `p.center_m()`) used for movement
/// collision and primary solid hit-testing.
fn prop_collision_shapes(p: &MapPropPlacement) -> [Option<LocalShape>; 2] {
    let fp = p.footprint_m();
    let shape = |cx, cy, sx, sy| Some(LocalShape::Obb { cx, cy, hx: sx, hy: sy });
    match p.kind {
        PropKind::DeskL => [shape(0.0, -0.5, 1.0, 0.5), shape(-0.5, 0.5, 0.5, 0.5)],
        PropKind::DeskT => [shape(0.0, -0.5, 1.5, 0.5), shape(0.0, 0.5, 0.5, 0.5)],
        PropKind::LargeTree => [Some(LocalShape::Circle { cx: 0.0, cy: 0.0, r: 0.44 }), None],
        PropKind::MediumTree => [Some(LocalShape::Circle { cx: 0.0, cy: 0.0, r: 0.26 }), None],
        PropKind::Bush | PropKind::Planter | PropKind::DrumCluster => {
            [Some(LocalShape::Circle { cx: 0.0, cy: 0.0, r: 0.46 }), None]
        }
        PropKind::RoundTable => [Some(LocalShape::Circle { cx: 0.0, cy: 0.0, r: 0.45 }), None],
        PropKind::BarStool => [Some(LocalShape::Circle { cx: 0.0, cy: 0.0, r: 0.28 }), None],
        PropKind::StructuralPillar => [shape(0.0, 0.0, 0.45, 0.45), None],
        _ => [shape(0.0, 0.0, fp.hx_m, fp.hy_m), None],
    }
}

/// Canopy radius in meters for tree props, or `0.0` for non-trees.
#[must_use]
pub const fn tree_canopy_radius_m(kind: PropKind) -> f32 {
    match kind {
        PropKind::LargeTree => 0.96,
        PropKind::MediumTree => 0.48,
        _ => 0.0,
    }
}

/// Closest point distance and push vector (`wx - closest`) from local point `(lx, ly)` to `shape`.
fn push_out_local_shape(lx: f32, ly: f32, radius: f32, shape: LocalShape) -> Option<(f32, f32)> {
    match shape {
        LocalShape::Obb { cx, cy, hx, hy } => {
            let rx = lx - cx;
            let ry = ly - cy;
            let clamped_x = rx.clamp(-hx, hx);
            let clamped_y = ry.clamp(-hy, hy);
            let dx = rx - clamped_x;
            let dy = ry - clamped_y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq >= radius * radius {
                return None;
            }
            if dist_sq > 1e-8 {
                let dist = dist_sq.sqrt();
                let pen = radius - dist;
                Some((dx / dist * pen, dy / dist * pen))
            } else {
                // Inside OBB: push out along nearest face.
                let px_right = (hx + radius) - rx;
                let px_left = rx + (hx + radius);
                let py_bot = (hy + radius) - ry;
                let py_top = ry + (hy + radius);
                let min_p = px_right.min(px_left).min(py_bot).min(py_top);
                if (min_p - px_right).abs() < 1e-5 {
                    Some((px_right, 0.0))
                } else if (min_p - px_left).abs() < 1e-5 {
                    Some((-px_left, 0.0))
                } else if (min_p - py_bot).abs() < 1e-5 {
                    Some((0.0, py_bot))
                } else {
                    Some((0.0, -py_top))
                }
            }
        }
        LocalShape::Circle { cx, cy, r } => {
            let dx = lx - cx;
            let dy = ly - cy;
            let min_dist = r + radius;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq >= min_dist * min_dist {
                return None;
            }
            let dist = dist_sq.sqrt();
            if dist > 1e-5 {
                let pen = min_dist - dist;
                Some((dx / dist * pen, dy / dist * pen))
            } else {
                Some((min_dist, 0.0))
            }
        }
    }
}

/// Whether world point `(wx_m, wy_m)` falls inside a prop's solid footprint (or tree canopy when `include_canopy`).
#[must_use]
pub fn prop_contains_point(p: &MapPropPlacement, wx_m: f32, wy_m: f32, include_canopy: bool) -> bool {
    let (lx, ly) = p.footprint_m().world_to_local(wx_m, wy_m);
    if include_canopy {
        let canopy_r = tree_canopy_radius_m(p.kind);
        if canopy_r > 0.0 && lx * lx + ly * ly <= canopy_r * canopy_r {
            return true;
        }
    }
    prop_collision_shapes(p).into_iter().flatten().any(|s| match s {
        LocalShape::Obb { cx, cy, hx, hy } => (lx - cx).abs() <= hx && (ly - cy).abs() <= hy,
        LocalShape::Circle { cx, cy, r } => (lx - cx).powi(2) + (ly - cy).powi(2) <= r * r,
    })
}

/// Whether world disc `(cx_m, cy_m, radius_m)` intersects the prop's solid collision footprint.
#[must_use]
pub fn prop_intersects_circle(p: &MapPropPlacement, cx_m: f32, cy_m: f32, radius_m: f32) -> bool {
    let (lx, ly) = p.footprint_m().world_to_local(cx_m, cy_m);
    prop_collision_shapes(p)
        .into_iter()
        .flatten()
        .any(|s| push_out_local_shape(lx, ly, radius_m, s).is_some())
}

/// Whether world point `(wx_m, wy_m)` hits a vehicle's front hood / engine block at any `0..360°` orientation.
#[must_use]
pub fn vehicle_point_is_engine_bay(p: &MapPropPlacement, wx_m: f32, wy_m: f32) -> bool {
    if !matches!(p.kind, PropKind::Car | PropKind::UtilityVan) || !prop_contains_point(p, wx_m, wy_m, false) {
        return false;
    }
    let (_, ly) = p.footprint_m().world_to_local(wx_m, wy_m);
    ly >= vehicle_hood_y_min_local(p)
}

/// World-space `(start, end)` endpoints in meters for a [`MapWallEdge`].
#[must_use]
pub fn edge_endpoints_m(edge: &MapWallEdge) -> ((f32, f32), (f32, f32)) {
    let x = edge.x as f32;
    let y = edge.y as f32;
    match edge.side {
        EdgeSide::North => ((x, y), (x + 1.0, y)),
        EdgeSide::West => ((x, y), (x, y + 1.0)),
    }
}

/// Euclidean distance and projection parameter `t ∈ [0.0, 1.0]` from point `(px, py)` to segment `a..b`.
#[must_use]
pub fn distance_point_to_segment(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= 1e-8 {
        return (((px - a.0).powi(2) + (py - a.1).powi(2)).sqrt(), 0.0);
    }
    let t = (((px - a.0) * dx + (py - a.1) * dy) / len_sq).clamp(0.0, 1.0);
    let qx = a.0 + dx * t;
    let qy = a.1 + dy * t;
    (((px - qx).powi(2) + (py - qy).powi(2)).sqrt(), t)
}

/// Intersects segment `(ax, ay) -> (bx, by)` with local axis-aligned box `[cx - hx, cx + hx] x [cy - hy, cy + hy]`.
fn intersect_segment_aabb(
    a: (f32, f32),
    b: (f32, f32),
    cx: f32,
    cy: f32,
    hx: f32,
    hy: f32,
) -> Option<(f32, f32)> {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let mut tmin = 0.0_f32;
    let mut tmax = 1.0_f32;
    for (p0, d, min_b, max_b) in [
        (a.0, dx, cx - hx, cx + hx),
        (a.1, dy, cy - hy, cy + hy),
    ] {
        if d.abs() < 1e-7 {
            if p0 < min_b || p0 > max_b {
                return None;
            }
        } else {
            let inv = 1.0 / d;
            let mut t1 = (min_b - p0) * inv;
            let mut t2 = (max_b - p0) * inv;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax + 1e-6 {
                return None;
            }
        }
    }
    if tmax <= tmin + 1e-6 {
        None
    } else {
        Some((tmin, tmax))
    }
}

/// Intersects segment `a -> b` with circle `(cx, cy, r)`, returning `(t_enter, t_exit)` in `[0.0, 1.0]`.
#[must_use]
pub fn intersect_segment_circle(
    a: (f32, f32),
    b: (f32, f32),
    cx: f32,
    cy: f32,
    r: f32,
) -> Option<(f32, f32)> {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let fx = a.0 - cx;
    let fy = a.1 - cy;
    let qa = dx * dx + dy * dy;
    if qa <= 1e-8 {
        return None;
    }
    let qb = 2.0 * (fx * dx + fy * dy);
    let qc = fx * fx + fy * fy - r * r;
    let disc = qb * qb - 4.0 * qa * qc;
    if disc < 0.0 {
        return None;
    }
    let sqrt_d = disc.sqrt();
    let t0 = ((-qb - sqrt_d) / (2.0 * qa)).clamp(0.0, 1.0);
    let t1 = ((-qb + sqrt_d) / (2.0 * qa)).clamp(0.0, 1.0);
    if t1 <= t0 + 1e-5 {
        None
    } else {
        Some((t0, t1))
    }
}

/// Intersects world segment `a -> b` with an oriented box centered at `center`, half-extents `(hx, hy)`,
/// rotated `rotation_deg` clockwise.
#[must_use]
pub fn intersect_segment_obb(
    a: (f32, f32),
    b: (f32, f32),
    center: (f32, f32),
    hx: f32,
    hy: f32,
    rotation_deg: i32,
) -> Option<(f32, f32)> {
    let rad = (rotation_deg.rem_euclid(360) as f32).to_radians();
    let (s, c) = rad.sin_cos();
    let to_local = |(px, py): (f32, f32)| {
        let dx = px - center.0;
        let dy = py - center.1;
        (dx * c + dy * s, -dx * s + dy * c)
    };
    intersect_segment_aabb(to_local(a), to_local(b), 0.0, 0.0, hx, hy)
}

/// Intersects world segment `a -> b` against a thin structural [`MapWallEdge`] (`EDGE_HALF_THICK_M`),
/// returning the entry parameter `t ∈ [0.0, 1.0]`.
#[must_use]
pub fn intersect_segment_edge(a: (f32, f32), b: (f32, f32), edge: &MapWallEdge) -> Option<f32> {
    let (p0, p1) = edge_endpoints_m(edge);
    let center = ((p0.0 + p1.0) * 0.5, (p0.1 + p1.1) * 0.5);
    let (hx, hy) = match edge.side {
        EdgeSide::North => (0.5 + EDGE_HALF_THICK_M, EDGE_HALF_THICK_M),
        EdgeSide::West => (EDGE_HALF_THICK_M, 0.5 + EDGE_HALF_THICK_M),
    };
    intersect_segment_aabb(a, b, center.0, center.1, hx, hy).map(|(t0, _)| t0)
}

/// Analytical segment hit against a solid (or canopy) prop in continuous meters.
#[must_use]
pub fn intersect_segment_prop(
    p: &MapPropPlacement,
    a: (f32, f32),
    b: (f32, f32),
) -> Option<PropRayHit> {
    let spec = prop_spec(p.kind);
    if spec.floor_layer {
        return None;
    }
    let fp = p.footprint_m();
    let la = fp.world_to_local(a.0, a.1);
    let lb = fp.world_to_local(b.0, b.1);
    let seg_len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    if seg_len <= 1e-6 {
        return None;
    }
    let mut best_t: Option<(f32, f32)> = None;
    for s in prop_collision_shapes(p).into_iter().flatten() {
        let hit = match s {
            LocalShape::Obb { cx, cy, hx, hy } => intersect_segment_aabb(la, lb, cx, cy, hx, hy),
            LocalShape::Circle { cx, cy, r } => intersect_segment_circle(la, lb, cx, cy, r),
        };
        if let Some((t0, t1)) = hit {
            best_t = Some(match best_t {
                Some((bt0, bt1)) => (bt0.min(t0), bt1.max(t1)),
                None => (t0, t1),
            });
        }
    }

    if matches!(p.kind, PropKind::LargeTree | PropKind::MediumTree) {
        if let Some((t0, t1)) = best_t {
            return Some(PropRayHit {
                t_enter: t0,
                t_exit: t1,
                thickness_m: ((t1 - t0) * seg_len).max(0.1),
                material: MaterialClass::LiveWood,
                height_cm: spec.height_cm,
                obstruction: PropObstruction::FullObstacle,
                is_engine_bay: false,
                is_tree_trunk: true,
            });
        }
        let canopy_r = tree_canopy_radius_m(p.kind);
        if let Some((t0, t1)) = intersect_segment_circle(la, lb, 0.0, 0.0, canopy_r) {
            return Some(PropRayHit {
                t_enter: t0,
                t_exit: t1,
                thickness_m: ((t1 - t0) * seg_len).max(0.1),
                material: MaterialClass::Foliage,
                height_cm: spec.height_cm,
                obstruction: PropObstruction::Concealment,
                is_engine_bay: false,
                is_tree_trunk: false,
            });
        }
        return None;
    }

    let (t0, t1) = best_t?;
    let mut is_engine_bay = false;
    let mut material = spec.material;
    if matches!(p.kind, PropKind::Car | PropKind::UtilityVan) {
        let hood_y = vehicle_hood_y_min_local(p);
        if intersect_segment_aabb(
            la,
            lb,
            0.0,
            (hood_y + fp.hy_m) * 0.5,
            fp.hx_m,
            (fp.hy_m - hood_y) * 0.5,
        )
        .is_some()
        {
            is_engine_bay = true;
            material = MaterialClass::ArmorSteel;
        }
    }

    Some(PropRayHit {
        t_enter: t0,
        t_exit: t1,
        thickness_m: ((t1 - t0) * seg_len).max(0.1),
        material,
        height_cm: spec.height_cm,
        obstruction: spec.obstruction,
        is_engine_bay,
        is_tree_trunk: false,
    })
}

pub(crate) fn circle_collides_with_filter(
    board_size_m: (f32, f32),
    edges: &[MapWallEdge],
    props: &[MapPropPlacement],
    center_m: (f32, f32),
    radius_m: f32,
    full_props_only: bool,
) -> bool {
    let (board_w_m, board_h_m) = board_size_m;
    let (cx, cy) = center_m;
    if cx < radius_m || cy < radius_m || cx > board_w_m - radius_m || cy > board_h_m - radius_m {
        return true;
    }
    for edge in edges {
        if matches!(edge.kind, WallEdgeKind::DoorOpen) {
            continue;
        }
        let (a, b) = edge_endpoints_m(edge);
        if distance_point_to_segment(cx, cy, a, b).0 < radius_m + EDGE_HALF_THICK_M - 1e-4 {
            return true;
        }
    }
    props.iter().any(|p| {
        let obs = prop_spec(p.kind).obstruction;
        let blocks = if full_props_only {
            obs == PropObstruction::FullObstacle
        } else {
            obs.blocks_movement()
        };
        blocks && prop_intersects_circle(p, cx, cy, radius_m - 1e-4)
    })
}

/// Checks whether a disc of `radius_m` centered at `(cx, cy)` collides with any blocking wall edge
/// or blocking prop, ignoring [`WallEdgeKind::DoorOpen`] and non-blocking props (`FloorLayer`, `Concealment`).
#[must_use]
pub fn circle_collides_obstacles(
    board_w_m: f32,
    board_h_m: f32,
    edges: &[MapWallEdge],
    props: &[MapPropPlacement],
    cx: f32,
    cy: f32,
    radius_m: f32,
) -> bool {
    circle_collides_with_filter((board_w_m, board_h_m), edges, props, (cx, cy), radius_m, false)
}

/// Resolves a point-and-click continuous destination in centimeters for `moving_entity_id`.
///
/// Clicking directly adjacent to a wall or piece of cover gently slides/snaps the miniature outward
/// (up to [`COVER_SNAP_MAX_M`] = `0.35m`), while clicking deep inside a wall, solid obstacle, or
/// over another living miniature is rejected cleanly.
pub fn resolve_token_destination_cm(
    req: &GmMapUpdateRequest,
    moving_entity_id: &str,
    target_x_cm: i32,
    target_y_cm: i32,
) -> Result<(i32, i32), &'static str> {
    let (board_w_m, board_h_m) = req.board_size_m();
    let click_x = target_x_cm as f32 / 100.0;
    let click_y = target_y_cm as f32 / 100.0;
    if click_x < 0.0 || click_y < 0.0 || click_x >= board_w_m || click_y >= board_h_m {
        return Err("Target coordinates are outside tactical map bounds");
    }

    // Clamp to perimeter keeping full miniature disc inside bounds.
    let mut x = click_x.clamp(TOKEN_RADIUS_M, (board_w_m - TOKEN_RADIUS_M).max(TOKEN_RADIUS_M));
    let mut y = click_y.clamp(TOKEN_RADIUS_M, (board_h_m - TOKEN_RADIUS_M).max(TOKEN_RADIUS_M));

    // Reject if clicked squarely onto another occupying token (center within 0.58m).
    for tok in &req.tokens {
        if tok.entity_id == moving_entity_id {
            continue;
        }
        let (tx, ty) = req.token_pos_m(tok);
        let d_click = ((click_x - tx).powi(2) + (click_y - ty).powi(2)).sqrt();
        if d_click < TOKEN_RADIUS_M * 1.95 {
            return Err("Target position is occupied by another miniature");
        }
    }

    // Iterative slide/snap relaxation against edges and blocking props.
    for _ in 0..4 {
        for edge in &req.wall_edges {
            if matches!(edge.kind, WallEdgeKind::DoorOpen) {
                continue;
            }
            let (a, b) = edge_endpoints_m(edge);
            let (dist, t) = distance_point_to_segment(x, y, a, b);
            let min_clear = TOKEN_RADIUS_M + EDGE_HALF_THICK_M + 0.005;
            if dist < min_clear {
                let qx = a.0 + (b.0 - a.0) * t;
                let qy = a.1 + (b.1 - a.1) * t;
                let (nx, ny) = if dist > 1e-4 {
                    ((x - qx) / dist, (y - qy) / dist)
                } else {
                    match edge.side {
                        EdgeSide::North => (0.0, if y >= qy { 1.0 } else { -1.0 }),
                        EdgeSide::West => (if x >= qx { 1.0 } else { -1.0 }, 0.0),
                    }
                };
                x += nx * (min_clear - dist);
                y += ny * (min_clear - dist);
            }
        }
        for p in &req.props {
            if !prop_spec(p.kind).obstruction.blocks_movement() {
                continue;
            }
            let fp = p.footprint_m();
            let (lx, ly) = fp.world_to_local(x, y);
            for shape in prop_collision_shapes(p).into_iter().flatten() {
                if let Some((plx, ply)) = push_out_local_shape(lx, ly, TOKEN_RADIUS_M + 0.005, shape) {
                    let (wx, wy) = fp.local_dir_to_world(plx, ply);
                    x += wx;
                    y += wy;
                }
            }
        }
        x = x.clamp(TOKEN_RADIUS_M, (board_w_m - TOKEN_RADIUS_M).max(TOKEN_RADIUS_M));
        y = y.clamp(TOKEN_RADIUS_M, (board_h_m - TOKEN_RADIUS_M).max(TOKEN_RADIUS_M));
    }

    let disp = ((x - click_x).powi(2) + (y - click_y).powi(2)).sqrt();
    if disp > COVER_SNAP_MAX_M {
        return Err("Target destination is blocked by solid cover or structural wall");
    }
    if circle_collides_obstacles(board_w_m, board_h_m, &req.wall_edges, &req.props, x, y, TOKEN_RADIUS_M) {
        return Err("Insufficient miniature clearance at target destination");
    }
    for tok in &req.tokens {
        if tok.entity_id == moving_entity_id {
            continue;
        }
        let (tx, ty) = req.token_pos_m(tok);
        if ((x - tx).powi(2) + (y - ty).powi(2)).sqrt() < TOKEN_RADIUS_M * 1.9 {
            return Err("Target destination overlaps another miniature");
        }
    }

    Ok(((x * 100.0).round() as i32, (y * 100.0).round() as i32))
}

/// Returns `true` when two prop footprints overlap in continuous XZ ground coordinates.
#[must_use]
pub fn props_overlap_continuous(a: &MapPropPlacement, b: &MapPropPlacement) -> bool {
    let afp = a.footprint_m();
    let (ahw, ahh) = rotated_aabb_half_extents_cm(
        (afp.span_w_m * 100.0) as i32,
        (afp.span_h_m * 100.0) as i32,
        afp.rot_deg,
    );
    let bfp = b.footprint_m();
    let (bhw, bhh) = rotated_aabb_half_extents_cm(
        (bfp.span_w_m * 100.0) as i32,
        (bfp.span_h_m * 100.0) as i32,
        bfp.rot_deg,
    );
    if (afp.cx_m - bfp.cx_m).abs() * 100.0 >= ahw + bhw
        || (afp.cy_m - bfp.cy_m).abs() * 100.0 >= ahh + bhh
    {
        return false;
    }
    // Check dense sample across a's center and corners against b's exact shape (and vice versa).
    for (fp, q) in [(afp, b), (bfp, a)] {
        let hx = fp.span_w_m * 0.44;
        let hy = fp.span_h_m * 0.44;
        for &(ox, oy) in &[
            (0.0, 0.0),
            (-hx, -hy),
            (hx, -hy),
            (-hx, hy),
            (hx, hy),
            (0.0, -hy),
            (0.0, hy),
            (-hx, 0.0),
            (hx, 0.0),
        ] {
            let (wx, wy) = fp.local_offset_to_world(ox, oy);
            if prop_contains_point(q, wx, wy, false) {
                return true;
            }
        }
    }
    false
}

/// Returns `true` if `prop` slices across any structural wall or closed door segment.
#[must_use]
pub fn prop_crosses_wall_edge(prop: &MapPropPlacement, edges: &[MapWallEdge]) -> bool {
    edges
        .iter()
        .filter(|e| matches!(e.kind, WallEdgeKind::Wall | WallEdgeKind::DoorClosed))
        .any(|e| {
            let (a, b) = edge_endpoints_m(e);
            intersect_segment_prop(prop, a, b).is_some_and(|hit| hit.thickness_m > 0.25)
        })
}

/// Validates and places a continuous `360°` prop placement onto `props` without touching any tile grid.
pub fn place_prop_on_map(
    props: &mut Vec<MapPropPlacement>,
    edges: &[MapWallEdge],
    occupied_tokens_m: &[(f32, f32)],
    board_size_m: (f32, f32),
    placement: MapPropPlacement,
) -> Result<MapPropPlacement, &'static str> {
    let (board_w_m, board_h_m) = board_size_m;
    if board_w_m <= 0.0 || board_h_m <= 0.0 {
        return Err("Map dimensions are invalid for prop placement");
    }
    let spec = prop_spec(placement.kind);
    let rot = crate::map_dressing::normalize_prop_rotation_deg(placement.kind, placement.rotation_deg);
    let (cx_cm, cy_cm) = placement.center_cm();
    let (raw_sw, raw_sh) = placement.local_span_cm();
    let length_cm = if spec.stretchable {
        Some(raw_sw.max(raw_sh).clamp(25, (board_w_m.max(board_h_m) * 100.0) as i32))
    } else {
        None
    };
    let norm = crate::map_dressing::make_prop_at_cm(
        placement.kind,
        cx_cm,
        cy_cm,
        rot,
        length_cm,
        placement.variant_seed,
    );
    let (sw_cm, sh_cm) = norm.local_span_cm();
    let (hw_cm, hh_cm) = rotated_aabb_half_extents_cm(sw_cm, sh_cm, rot);
    let (ncx, ncy) = norm.center_m();
    let tol = 0.06;
    if ncx - hw_cm / 100.0 < -tol
        || ncy - hh_cm / 100.0 < -tol
        || ncx + hw_cm / 100.0 > board_w_m + tol
        || ncy + hh_cm / 100.0 > board_h_m + tol
    {
        return Err("Prop footprint extends outside map bounds");
    }
    if spec.obstruction.blocks_movement()
        && occupied_tokens_m
            .iter()
            .any(|&(tx, ty)| prop_intersects_circle(&norm, tx, ty, TOKEN_RADIUS_M - 0.02))
    {
        return Err("Prop footprint overlaps an occupied miniature");
    }
    if !spec.floor_layer
        && edges
            .iter()
            .filter(|e| matches!(e.kind, WallEdgeKind::Wall | WallEdgeKind::DoorClosed))
            .any(|e| {
                let (a, b) = edge_endpoints_m(e);
                intersect_segment_prop(&norm, a, b).is_some_and(|hit| hit.thickness_m > 0.25)
            })
    {
        return Err("Prop footprint crosses an interior structural wall or closed door");
    }

    if !spec.floor_layer {
        props.retain(|existing| {
            prop_spec(existing.kind).floor_layer || !props_overlap_continuous(existing, &norm)
        });
    } else {
        props.retain(|p| {
            !(prop_spec(p.kind).floor_layer
                && p.kind == norm.kind
                && (p.center_cm().0 - cx_cm).abs() < 40
                && (p.center_cm().1 - cy_cm).abs() < 40)
        });
    }
    props.push(norm);
    Ok(norm)
}

/// Removes the topmost solid prop (or topmost floor-layer surface prop) covering `(x_m, y_m)`.
pub fn remove_prop_at_point(
    props: &mut Vec<MapPropPlacement>,
    x_m: f32,
    y_m: f32,
) -> Option<MapPropPlacement> {
    let idx = props
        .iter()
        .rposition(|p| !prop_spec(p.kind).floor_layer && prop_contains_point(p, x_m, y_m, true))
        .or_else(|| props.iter().rposition(|p| prop_contains_point(p, x_m, y_m, false)))?;
    Some(props.remove(idx))
}
