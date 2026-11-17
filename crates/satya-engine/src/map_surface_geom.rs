//! Continuous Vector Surface Geometry (`map_surface_geom`)
//!
//! Evaluates [`MapSurfaceFeature`] splines and closed contours once for both engine gameplay
//! (reachability cost, hover inspection, road-purity clearance, tangent-aligned prop placement,
//! continuation seam bridging) and frontend rendering (2D SVG `<path>` and 3D WebGPU triangles).
//!
//! No triangle clipping, ear-cutting, or Catmull-Rom math should be duplicated in `satya-web`.

use satya_schema::tactical_map_surfaces_dto::{
    MapPointCm, MapSurfaceFeature, SurfaceFeatureKind, SurfaceShapeKind,
};

pub use crate::map_surface_queries::*;

/// Minimum segment length threshold (m) used to ignore degenerate duplicate control points.
const EPS_LEN_M: f32 = 1e-4;

/// Returns `(rel_fraction_of_half_w, max_amp_m)` controlling 1D fBm edge modulation for natural ribbons.
#[must_use]
pub const fn ribbon_edge_roughness_spec(kind: SurfaceFeatureKind) -> (f32, f32) {
    match kind {
        SurfaceFeatureKind::CreekChannel => (0.26, 0.42),
        SurfaceFeatureKind::DefensiveTrench => (0.22, 0.48),
        SurfaceFeatureKind::DrainageDitch => (0.22, 0.28),
        SurfaceFeatureKind::DirtTrail => (0.20, 0.24),
        SurfaceFeatureKind::GravelRoad => (0.14, 0.22),
        SurfaceFeatureKind::StoneWalk => (0.05, 0.05),
        _ => (0.0, 0.0),
    }
}

#[inline]
fn hash_1d_knot(knot: i32, seed: u32) -> f32 {
    let mut x = (knot as u32).wrapping_mul(0x9E37_79B1) ^ seed.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    ((x & 0x00FF_FFFF) as f32 / 16_777_215.0) * 2.0 - 1.0
}

/// Deterministic 3-octave 1D fBm noise in `[-1.0, 1.0]` for organic ribbon edge modulation.
#[must_use]
pub(crate) fn fbm_ribbon_edge_1d(arc_m: f32, seed: u32) -> f32 {
    const OCTAVES: [(f32, f32, u32); 3] = [
        (4.2, 0.52, 0x1357_9BDF),
        (1.65, 0.32, 0x2468_ACE0),
        (0.68, 0.16, 0xA5A5_5A5A),
    ];
    let mut sum = 0.0_f32;
    for (wl, w, salt) in OCTAVES {
        let pos = arc_m / wl;
        let i0 = pos.floor() as i32;
        let f = pos - i0 as f32;
        let u = f * f * (3.0 - 2.0 * f);
        let v0 = hash_1d_knot(i0, seed ^ salt);
        let v1 = hash_1d_knot(i0 + 1, seed ^ salt);
        sum += (v0 + (v1 - v0) * u) * w;
    }
    sum.clamp(-1.0, 1.0)
}

/// Computes local left/right bank half-width at `arc_m` with independent side seeds and pinch guard.
#[must_use]
pub(crate) fn local_ribbon_half_width_m(
    base_half_w_m: f32,
    kind: SurfaceFeatureKind,
    variant_seed: u32,
    is_left: bool,
    arc_m: f32,
    edge_taper: f32,
) -> f32 {
    let (rel_frac, max_amp_m) = ribbon_edge_roughness_spec(kind);
    if rel_frac <= 0.0 || edge_taper <= 0.0 {
        return base_half_w_m;
    }
    let side_seed = variant_seed ^ if is_left { 0x9E37_79B9 } else { 0x51A7_F00D };
    let fbm = fbm_ribbon_edge_1d(arc_m, side_seed);
    let amp = (base_half_w_m * rel_frac).min(max_amp_m) * edge_taper.clamp(0.0, 1.0);
    (base_half_w_m + amp * fbm).max(base_half_w_m * 0.58)
}

fn resample_polyline_max_step_m(pts: &[(f32, f32)], max_step_m: f32) -> Vec<(f32, f32)> {
    if pts.len() < 2 || max_step_m <= EPS_LEN_M {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(pts.len() * 2);
    out.push(pts[0]);
    for w in pts.windows(2) {
        let seg_len = dist_m(w[0], w[1]);
        let steps = (seg_len / max_step_m).ceil().max(1.0) as usize;
        for s in 1..=steps {
            out.push(lerp_pt(w[0], w[1], s as f32 / steps as f32));
        }
    }
    out
}

/// Samples an open polyline through centripetal Catmull-Rom interpolation (`alpha = 0.5`).
///
/// Phantom endpoints extend along the end tangents (`2*p0 - p1`, `2*p_{n-1} - p_{n-2}`) so
/// roads, creeks, ditches, and trails enter and exit cleanly without pinching.
#[must_use]
pub fn sample_open_catmull_rom_m(pts: &[(f32, f32)], subdivs_per_seg: usize) -> Vec<(f32, f32)> {
    let clean = dedup_consecutive_points(pts);
    if clean.len() < 2 {
        return clean;
    }
    let steps = subdivs_per_seg.max(1);
    if clean.len() == 2 {
        let mut out = Vec::with_capacity(steps + 1);
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            out.push((
                clean[0].0 + (clean[1].0 - clean[0].0) * t,
                clean[0].1 + (clean[1].1 - clean[0].1) * t,
            ));
        }
        return out;
    }

    let n = clean.len();
    let p_start = (2.0 * clean[0].0 - clean[1].0, 2.0 * clean[0].1 - clean[1].1);
    let p_end = (
        2.0 * clean[n - 1].0 - clean[n - 2].0,
        2.0 * clean[n - 1].1 - clean[n - 2].1,
    );

    let mut ext = Vec::with_capacity(n + 2);
    ext.push(p_start);
    ext.extend_from_slice(&clean);
    ext.push(p_end);

    let mut out = Vec::with_capacity((n - 1) * steps + 1);
    out.push(clean[0]);
    for i in 1..ext.len() - 2 {
        let (p0, p1, p2, p3) = (ext[i - 1], ext[i], ext[i + 1], ext[i + 2]);
        for s in 1..=steps {
            let u = s as f32 / steps as f32;
            out.push(centripetal_cr_eval(p0, p1, p2, p3, u));
        }
    }
    out
}

/// Streams open centripetal Catmull-Rom segments `(a_m, b_m)` on the stack with zero heap
/// allocations for typical splines (`<= 32` control points).
pub(crate) fn visit_open_catmull_rom_segments(
    points_cm: &[MapPointCm],
    subdivs_per_seg: usize,
    mut f: impl FnMut((f32, f32), (f32, f32)),
) {
    if points_cm.len() > 32 {
        let raw: Vec<(f32, f32)> = points_cm.iter().map(|p| p.to_m()).collect();
        let cl = sample_open_catmull_rom_m(&raw, subdivs_per_seg);
        if cl.len() == 1 {
            f(cl[0], cl[0]);
            return;
        }
        for w in cl.windows(2) {
            f(w[0], w[1]);
        }
        return;
    }

    let mut clean = [(0.0_f32, 0.0_f32); 32];
    let mut n = 0usize;
    for p in points_cm {
        let m = p.to_m();
        if n == 0 || dist_sq(clean[n - 1], m) > EPS_LEN_M * EPS_LEN_M {
            clean[n] = m;
            n += 1;
        }
    }
    if n == 0 {
        return;
    }
    if n == 1 {
        f(clean[0], clean[0]);
        return;
    }
    if n == 2 {
        f(clean[0], clean[1]);
        return;
    }

    let steps = subdivs_per_seg.max(1);
    let p_start = (2.0 * clean[0].0 - clean[1].0, 2.0 * clean[0].1 - clean[1].1);
    let p_end = (
        2.0 * clean[n - 1].0 - clean[n - 2].0,
        2.0 * clean[n - 1].1 - clean[n - 2].1,
    );
    let mut prev = clean[0];
    for i in 0..n - 1 {
        let p0 = if i == 0 { p_start } else { clean[i - 1] };
        let p1 = clean[i];
        let p2 = clean[i + 1];
        let p3 = if i + 2 == n { p_end } else { clean[i + 2] };
        for s in 1..=steps {
            let u = s as f32 / steps as f32;
            let cur = centripetal_cr_eval(p0, p1, p2, p3, u);
            f(prev, cur);
            prev = cur;
        }
    }
}

/// Samples a closed loop via cyclic centripetal Catmull-Rom interpolation (`alpha = 0.5`).
///
/// The returned ring has distinct first/last vertices (implicitly wraps last -> first).
#[must_use]
pub fn sample_closed_catmull_rom_m(pts: &[(f32, f32)], subdivs_per_seg: usize) -> Vec<(f32, f32)> {
    let mut clean = dedup_consecutive_points(pts);
    if clean.len() >= 2 && dist_sq(clean[0], clean[clean.len() - 1]) < EPS_LEN_M * EPS_LEN_M {
        clean.pop();
    }
    if clean.len() < 3 {
        return clean;
    }
    let steps = subdivs_per_seg.max(1);
    let n = clean.len();
    let mut out = Vec::with_capacity(n * steps);
    for i in 0..n {
        let p0 = clean[(i + n - 1) % n];
        let p1 = clean[i];
        let p2 = clean[(i + 1) % n];
        let p3 = clean[(i + 2) % n];
        out.push(p1);
        for s in 1..steps {
            let u = s as f32 / steps as f32;
            out.push(centripetal_cr_eval(p0, p1, p2, p3, u));
        }
    }
    out
}

/// Evaluates centripetal Catmull-Rom (`alpha = 0.5`) between `p1` (`u = 0`) and `p2` (`u = 1`).
#[inline]
fn centripetal_cr_eval(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    u: f32,
) -> (f32, f32) {
    let dt01 = dist_m(p0, p1).sqrt().max(EPS_LEN_M);
    let dt12 = dist_m(p1, p2).sqrt().max(EPS_LEN_M);
    let dt23 = dist_m(p2, p3).sqrt().max(EPS_LEN_M);

    let t0 = 0.0_f32;
    let t1 = t0 + dt01;
    let t2 = t1 + dt12;
    let t3 = t2 + dt23;
    let t = t1 + u.clamp(0.0, 1.0) * (t2 - t1);

    let a1 = lerp_pt(p0, p1, (t - t0) / (t1 - t0));
    let a2 = lerp_pt(p1, p2, (t - t1) / (t2 - t1));
    let a3 = lerp_pt(p2, p3, (t - t2) / (t3 - t2));

    let b1 = lerp_pt(a1, a2, (t - t0) / (t2 - t0));
    let b2 = lerp_pt(a2, a3, (t - t1) / (t3 - t1));

    lerp_pt(b1, b2, (t - t1) / (t2 - t1))
}

/// Subdivides polygon corners with quadratic Bezier fillets of radius `corner_radius_m`.
#[must_use]
pub fn sample_rounded_polygon_m(pts: &[(f32, f32)], corner_radius_m: f32) -> Vec<(f32, f32)> {
    let mut clean = dedup_consecutive_points(pts);
    if clean.len() >= 2 && dist_sq(clean[0], clean[clean.len() - 1]) < EPS_LEN_M * EPS_LEN_M {
        clean.pop();
    }
    if clean.len() < 3 || corner_radius_m <= 0.01 {
        return clean;
    }

    let n = clean.len();
    let arc_steps = 4usize;
    let mut out = Vec::with_capacity(n * (arc_steps + 1));
    for i in 0..n {
        let prev = clean[(i + n - 1) % n];
        let curr = clean[i];
        let next = clean[(i + 1) % n];
        let (vin_x, vin_y, lin) = unit_vec(curr, prev);
        let (vout_x, vout_y, lout) = unit_vec(curr, next);
        if lin < EPS_LEN_M || lout < EPS_LEN_M {
            out.push(curr);
            continue;
        }
        let trim = corner_radius_m.min(0.35 * lin).min(0.35 * lout);
        if trim <= 0.01 {
            out.push(curr);
            continue;
        }
        let a = (curr.0 + vin_x * trim, curr.1 + vin_y * trim);
        let b = (curr.0 + vout_x * trim, curr.1 + vout_y * trim);
        for s in 0..=arc_steps {
            let t = s as f32 / arc_steps as f32;
            let inv = 1.0 - t;
            out.push((
                inv * inv * a.0 + 2.0 * inv * t * curr.0 + t * t * b.0,
                inv * inv * a.1 + 2.0 * inv * t * curr.1 + t * t * b.1,
            ));
        }
    }
    dedup_consecutive_points(&out)
}

/// Extruded 2D ribbon strip with left/right boundary rails in continuous world meters.
#[derive(Debug, Clone)]
pub struct RibbonMesh2d {
    pub centerline: Vec<(f32, f32)>,
    pub left: Vec<(f32, f32)>,
    pub right: Vec<(f32, f32)>,
    pub half_w_m: f32,
    pub cap_start: bool,
    pub cap_end: bool,
}

impl RibbonMesh2d {
    /// Returns a closed polygon ring tracing `left -> end cap -> reversed right -> start cap`.
    #[must_use]
    pub fn to_closed_ring_m(&self) -> Vec<(f32, f32)> {
        if self.centerline.len() < 2 {
            return Vec::new();
        }
        let mut ring = Vec::with_capacity(self.left.len() + self.right.len() + 10);
        ring.extend_from_slice(&self.left);
        if self.cap_end {
            append_semicircle_cap(
                &mut ring,
                self.centerline[self.centerline.len() - 1],
                self.left[self.left.len() - 1],
                self.right[self.right.len() - 1],
                4,
            );
        }
        for &pt in self.right.iter().rev() {
            ring.push(pt);
        }
        if self.cap_start {
            append_semicircle_cap(
                &mut ring,
                self.centerline[0],
                self.right[0],
                self.left[0],
                4,
            );
        }
        dedup_consecutive_points(&ring)
    }

    /// Triangulates the ribbon strip (plus round interior end caps when enabled) as 2D `[x, y]` vertices and triangle indices.
    #[must_use]
    pub fn build_strip_triangles_m(&self) -> (Vec<[f32; 2]>, Vec<u32>) {
        let n = self.centerline.len();
        if n < 2 || self.left.len() != n || self.right.len() != n {
            return (Vec::new(), Vec::new());
        }
        let mut verts = Vec::with_capacity(n * 2 + 12);
        let mut indices = Vec::with_capacity((n - 1) * 6 + 24);
        for i in 0..n {
            verts.push([self.left[i].0, self.left[i].1]);
            verts.push([self.right[i].0, self.right[i].1]);
        }
        for i in 0..n - 1 {
            let l0 = (i * 2) as u32;
            let r0 = l0 + 1;
            let l1 = ((i + 1) * 2) as u32;
            let r1 = l1 + 1;
            // Clockwise/CCW doesn't hurt for unculled top surfaces, keep consistent CCW in +X/+Y screen frame
            indices.extend_from_slice(&[l0, r0, l1, r0, r1, l1]);
        }
        if self.cap_start {
            append_cap_fan(
                &mut verts,
                &mut indices,
                self.centerline[0],
                self.right[0],
                self.left[0],
                5,
            );
        }
        if self.cap_end {
            append_cap_fan(
                &mut verts,
                &mut indices,
                self.centerline[n - 1],
                self.left[n - 1],
                self.right[n - 1],
                5,
            );
        }
        (verts, indices)
    }
}

#[derive(Clone, Copy)]
struct RibbonStyleSpec {
    kind: SurfaceFeatureKind,
    variant_seed: u32,
    board_m: Option<(f32, f32)>,
}

/// Extrudes a smooth centerline polyline into left/right parallel rails with clamped miters.
#[must_use]
pub fn extrude_ribbon_strip_m(
    centerline: &[(f32, f32)],
    half_w_m: f32,
    cap_start: bool,
    cap_end: bool,
) -> RibbonMesh2d {
    extrude_styled_ribbon_strip_m(centerline, half_w_m, cap_start, cap_end, None)
}

fn extrude_styled_ribbon_strip_m(
    centerline: &[(f32, f32)],
    half_w_m: f32,
    cap_start: bool,
    cap_end: bool,
    style: Option<RibbonStyleSpec>,
) -> RibbonMesh2d {
    let mut pts = dedup_consecutive_points(centerline);
    if style.is_some_and(|st| ribbon_edge_roughness_spec(st.kind).0 > 0.0) && pts.len() >= 2 {
        pts = dedup_consecutive_points(&resample_polyline_max_step_m(&pts, 0.32));
    }
    let hw = half_w_m.max(0.02);
    let n = pts.len();
    if n < 2 {
        return RibbonMesh2d {
            centerline: pts,
            left: Vec::new(),
            right: Vec::new(),
            half_w_m: hw,
            cap_start: false,
            cap_end: false,
        };
    }

    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    let mut arc_m = 0.0_f32;
    for i in 0..n {
        if i > 0 {
            arc_m += dist_m(pts[i - 1], pts[i]);
        }
        let (tx, ty, miter_scale) = if i == 0 {
            let (dx, dy, _) = unit_vec(pts[0], pts[1]);
            (dx, dy, 1.0_f32)
        } else if i + 1 == n {
            let (dx, dy, _) = unit_vec(pts[n - 2], pts[n - 1]);
            (dx, dy, 1.0_f32)
        } else {
            let (dx0, dy0, _) = unit_vec(pts[i - 1], pts[i]);
            let (dx1, dy1, _) = unit_vec(pts[i], pts[i + 1]);
            let sx = dx0 + dx1;
            let sy = dy0 + dy1;
            let slen = sx.hypot(sy);
            if slen < EPS_LEN_M {
                (dx1, dy1, 1.0_f32)
            } else {
                let (ux, uy) = (sx / slen, sy / slen);
                let dot = (dx0 * ux + dy0 * uy).max(0.55);
                (ux, uy, (1.0 / dot).min(1.4))
            }
        };
        let nx = -ty;
        let ny = tx;
        let edge_taper = if let Some((bw_m, bh_m)) = style.and_then(|st| st.board_m) {
            let d_edge = pts[i]
                .0
                .min(bw_m - pts[i].0)
                .min(pts[i].1)
                .min(bh_m - pts[i].1);
            if d_edge < 0.75 {
                let t = (d_edge / 0.75).clamp(0.0, 1.0);
                0.30 + 0.70 * t
            } else {
                1.0
            }
        } else {
            1.0
        };
        let (hw_l, hw_r) = match style {
            Some(st) => (
                local_ribbon_half_width_m(hw, st.kind, st.variant_seed, true, arc_m, edge_taper),
                local_ribbon_half_width_m(hw, st.kind, st.variant_seed, false, arc_m, edge_taper),
            ),
            None => (hw, hw),
        };
        left.push((
            pts[i].0 + nx * hw_l * miter_scale,
            pts[i].1 + ny * hw_l * miter_scale,
        ));
        right.push((
            pts[i].0 - nx * hw_r * miter_scale,
            pts[i].1 - ny * hw_r * miter_scale,
        ));
    }

    RibbonMesh2d {
        centerline: pts,
        left,
        right,
        half_w_m: hw,
        cap_start,
        cap_end,
    }
}

/// Evaluates and extrudes a [`SurfaceShapeKind::RibbonSpline`] feature, leaving ends flat when
/// within `0.35 m` of the board boundary `(0..board_w_m, 0..board_h_m)` and capped inside.
#[must_use]
pub fn extrude_surface_ribbon_m(
    surface: &MapSurfaceFeature,
    board_w_m: f32,
    board_h_m: f32,
) -> RibbonMesh2d {
    let raw: Vec<(f32, f32)> = surface.points_cm.iter().map(|p| p.to_m()).collect();
    let cl = sample_open_catmull_rom_m(&raw, 8);
    let half_w = (surface.width_cm.max(10) as f32) / 200.0;
    let cap_start = cl
        .first()
        .is_some_and(|&(x, y)| !is_near_board_edge(x, y, board_w_m, board_h_m, 0.35));
    let cap_end = cl
        .last()
        .is_some_and(|&(x, y)| !is_near_board_edge(x, y, board_w_m, board_h_m, 0.35));
    extrude_styled_ribbon_strip_m(
        &cl,
        half_w,
        cap_start,
        cap_end,
        Some(RibbonStyleSpec {
            kind: surface.kind,
            variant_seed: surface.variant_seed,
            board_m: Some((board_w_m, board_h_m)),
        }),
    )
}

/// Returns true if `(x, y)` lies near or outside any board boundary within `margin_m`.
#[must_use]
pub fn is_near_board_edge(x: f32, y: f32, board_w_m: f32, board_h_m: f32, margin_m: f32) -> bool {
    x <= margin_m || y <= margin_m || x >= board_w_m - margin_m || y >= board_h_m - margin_m
}

/// Evaluates the closed outer outline ring in world meters for any [`MapSurfaceFeature`].
#[must_use]
pub fn sampled_closed_ring_m(surface: &MapSurfaceFeature) -> Vec<(f32, f32)> {
    let raw: Vec<(f32, f32)> = surface.points_cm.iter().map(|p| p.to_m()).collect();
    match surface.shape {
        SurfaceShapeKind::OrganicClosed => sample_closed_catmull_rom_m(&raw, 6),
        SurfaceShapeKind::Polygon => {
            let radius_m = (surface.border_style_cm.max(0) as f32) / 100.0;
            sample_rounded_polygon_m(&raw, radius_m)
        }
        SurfaceShapeKind::RibbonSpline => {
            let cl = sample_open_catmull_rom_m(&raw, 8);
            let half_w = (surface.width_cm.max(10) as f32) / 200.0;
            extrude_styled_ribbon_strip_m(
                &cl,
                half_w,
                true,
                true,
                Some(RibbonStyleSpec {
                    kind: surface.kind,
                    variant_seed: surface.variant_seed,
                    board_m: None,
                }),
            )
            .to_closed_ring_m()
        }
    }
}

/// Computes signed shoelace area (`> 0` for CCW in standard +X/+Y frame).
#[must_use]
pub fn signed_area_2d(ring: &[(f32, f32)]) -> f32 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        sum += a.0 * b.1 - b.0 * a.1;
    }
    0.5 * sum
}

/// Triangulates a simple concave or convex polygon ring via ear-clipping.
///
/// Returns normalized CCW vertices and `[i0, i1, i2]` triangle index triples safe for WebGPU.
#[must_use]
pub fn triangulate_simple_polygon_m(ring: &[(f32, f32)]) -> (Vec<(f32, f32)>, Vec<[usize; 3]>) {
    let mut verts = remove_collinear_and_dups(ring);
    if verts.len() < 3 {
        return (verts, Vec::new());
    }
    if signed_area_2d(&verts) < 0.0 {
        verts.reverse();
    }

    let n = verts.len();
    let mut active: Vec<usize> = (0..n).collect();
    let mut tris = Vec::with_capacity(n - 2);
    let mut guard = 0usize;
    let max_iters = n * n + 8;

    while active.len() > 3 && guard < max_iters {
        guard += 1;
        let m = active.len();
        let mut clipped = false;
        for pos in 0..m {
            let ip = active[(pos + m - 1) % m];
            let ic = active[pos];
            let inx = active[(pos + 1) % m];
            let (a, b, c) = (verts[ip], verts[ic], verts[inx]);
            if cross_2d(a, b, c) <= 1e-6 {
                continue;
            }
            let mut empty = true;
            for (kpos, &ik) in active.iter().enumerate() {
                if kpos == pos || kpos == (pos + m - 1) % m || kpos == (pos + 1) % m {
                    continue;
                }
                if point_in_triangle_ccw(verts[ik], a, b, c) {
                    empty = false;
                    break;
                }
            }
            if empty {
                tris.push([ip, ic, inx]);
                active.remove(pos);
                clipped = true;
                break;
            }
        }
        if !clipped {
            // Fallback ear without strict point-in-triangle check on nearly degenerate vertices.
            let mut best_pos = 0usize;
            let mut best_cross = f32::NEG_INFINITY;
            for pos in 0..m {
                let ip = active[(pos + m - 1) % m];
                let ic = active[pos];
                let inx = active[(pos + 1) % m];
                let cr = cross_2d(verts[ip], verts[ic], verts[inx]);
                if cr > best_cross {
                    best_cross = cr;
                    best_pos = pos;
                }
            }
            let ip = active[(best_pos + m - 1) % m];
            let ic = active[best_pos];
            let inx = active[(best_pos + 1) % m];
            tris.push([ip, ic, inx]);
            active.remove(best_pos);
        }
    }
    if active.len() == 3 {
        tris.push([active[0], active[1], active[2]]);
    }
    (verts, tris)
}

/// Converts tangent vector `(dx, dy)` into continuous `[0, 360)` degrees for local-`+X` props
/// (`ParkBench`, `Hedge`, `JerseyBarrier`, `FallenLog`, `GardenEdging`).
#[must_use]
pub fn along_tangent_rot_deg(dx: f32, dy: f32) -> i32 {
    if dx.hypot(dy) < EPS_LEN_M {
        return 0;
    }
    (dy.atan2(dx).to_degrees().round() as i32).rem_euclid(360)
}

/// Converts forward vector `(dx, dy)` into continuous `[0, 360)` degrees for local-`+Y`-front props
/// (`Car`, `UtilityVan`, where `rot = 0°` faces South `+Y`).
#[must_use]
pub fn vehicle_forward_rot_deg(dx: f32, dy: f32) -> i32 {
    if dx.hypot(dy) < EPS_LEN_M {
        return 0;
    }
    ((-dx).atan2(dy).to_degrees().round() as i32).rem_euclid(360)
}

struct SampledArcTable {
    pts: Vec<(f32, f32)>,
    cum_m: Vec<f32>,
    total_m: f32,
}

fn build_sampled_arc_table(
    surface: &MapSurfaceFeature,
    subdivs_per_seg: usize,
) -> Option<SampledArcTable> {
    let raw: Vec<(f32, f32)> = surface.points_cm.iter().map(|p| p.to_m()).collect();
    let cl = sample_open_catmull_rom_m(&raw, subdivs_per_seg);
    if cl.len() < 2 {
        return None;
    }
    let mut cum = Vec::with_capacity(cl.len());
    cum.push(0.0_f32);
    for w in cl.windows(2) {
        cum.push(cum[cum.len() - 1] + dist_m(w[0], w[1]));
    }
    let total = *cum.last().unwrap_or(&0.0);
    Some(SampledArcTable {
        pts: cl,
        cum_m: cum,
        total_m: total,
    })
}

fn eval_polyline_pos_tan_at_arc(
    cl: &[(f32, f32)],
    cum: &[f32],
    target_arc_m: f32,
) -> ((f32, f32), (f32, f32)) {
    for i in 0..cl.len() - 1 {
        let d0 = cum[i];
        let d1 = cum[i + 1];
        if target_arc_m <= d1 || i + 2 == cl.len() {
            let seg = (d1 - d0).max(EPS_LEN_M);
            let t = ((target_arc_m - d0) / seg).clamp(0.0, 1.0);
            let pos = lerp_pt(cl[i], cl[i + 1], t);
            let (tx, ty, _) = unit_vec(cl[i], cl[i + 1]);
            return (pos, (tx, ty));
        }
    }
    (cl[0], (1.0, 0.0))
}

/// Samples a `RibbonSpline` by arc length at normalized fraction `frac in [0, 1]`, returning
/// `(position_m, unit_tangent)`.
#[must_use]
pub fn ribbon_point_and_tangent_at_fraction(
    surface: &MapSurfaceFeature,
    frac: f32,
) -> Option<((f32, f32), (f32, f32))> {
    let tbl = build_sampled_arc_table(surface, 8)?;
    if tbl.total_m < EPS_LEN_M {
        return Some((tbl.pts[0], (1.0, 0.0)));
    }
    let target = frac.clamp(0.0, 1.0) * tbl.total_m;
    Some(eval_polyline_pos_tan_at_arc(&tbl.pts, &tbl.cum_m, target))
}

/// Position `(x_m, y_m)`, macro unit tangent `(tx, ty)`, and chord bend sagitta (m).
pub type RibbonMacroChord = ((f32, f32), (f32, f32), f32);

/// Samples a `RibbonSpline` by arc length at `frac in [0, 1]`, returning
/// `(position_m, macro_unit_tangent, bend_sagitta_m)` averaged over `±half_window_m`.
#[must_use]
pub fn ribbon_macro_chord_at_fraction(
    surface: &MapSurfaceFeature,
    frac: f32,
    half_window_m: f32,
) -> Option<RibbonMacroChord> {
    let tbl = build_sampled_arc_table(surface, 10)?;
    if tbl.total_m < EPS_LEN_M {
        return Some((tbl.pts[0], (1.0, 0.0), 0.0));
    }
    let s_mid = frac.clamp(0.0, 1.0) * tbl.total_m;
    let hw = half_window_m.max(0.2);
    let s_lo = (s_mid - hw).max(0.0);
    let s_hi = (s_mid + hw).min(tbl.total_m);
    let (pos, t_instant) = eval_polyline_pos_tan_at_arc(&tbl.pts, &tbl.cum_m, s_mid);
    let (p_lo, _) = eval_polyline_pos_tan_at_arc(&tbl.pts, &tbl.cum_m, s_lo);
    let (p_hi, _) = eval_polyline_pos_tan_at_arc(&tbl.pts, &tbl.cum_m, s_hi);
    let (tx, ty, chord_len) = unit_vec(p_lo, p_hi);
    let chord_dir = if chord_len < EPS_LEN_M {
        t_instant
    } else {
        (tx, ty)
    };
    let mid_chord = ((p_lo.0 + p_hi.0) * 0.5, (p_lo.1 + p_hi.1) * 0.5);
    let sagitta_m = dist_m(pos, mid_chord);
    Some((pos, chord_dir, sagitta_m))
}

fn dedup_consecutive_points(pts: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = Vec::with_capacity(pts.len());
    for &p in pts {
        if out
            .last()
            .is_none_or(|&prev| dist_sq(prev, p) > EPS_LEN_M * EPS_LEN_M)
        {
            out.push(p);
        }
    }
    out
}

fn remove_collinear_and_dups(ring: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut clean = dedup_consecutive_points(ring);
    if clean.len() >= 2 && dist_sq(clean[0], clean[clean.len() - 1]) < EPS_LEN_M * EPS_LEN_M {
        clean.pop();
    }
    if clean.len() <= 3 {
        return clean;
    }
    let mut out = Vec::with_capacity(clean.len());
    let n = clean.len();
    for i in 0..n {
        let a = clean[(i + n - 1) % n];
        let b = clean[i];
        let c = clean[(i + 1) % n];
        if cross_2d(a, b, c).abs() > 1e-6 {
            out.push(b);
        }
    }
    out
}

fn cap_arc_angles_and_radii(
    center: (f32, f32),
    from: (f32, f32),
    to: (f32, f32),
) -> (f32, f32, f32, f32) {
    let a0 = (from.1 - center.1).atan2(from.0 - center.0);
    let a1 = (to.1 - center.1).atan2(to.0 - center.0);
    let mut delta = a1 - a0;
    while delta > std::f32::consts::PI {
        delta -= std::f32::consts::TAU;
    }
    while delta < -std::f32::consts::PI {
        delta += std::f32::consts::TAU;
    }
    (a0, delta, dist_m(center, from), dist_m(center, to))
}

fn append_semicircle_cap(
    ring: &mut Vec<(f32, f32)>,
    center: (f32, f32),
    from: (f32, f32),
    to: (f32, f32),
    steps: usize,
) {
    let (a0, delta, r0, r1) = cap_arc_angles_and_radii(center, from, to);
    for s in 1..steps {
        let frac = s as f32 / steps as f32;
        let ang = a0 + delta * frac;
        let r = r0 + (r1 - r0) * frac;
        ring.push((center.0 + r * ang.cos(), center.1 + r * ang.sin()));
    }
}

fn append_cap_fan(
    verts: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    center: (f32, f32),
    from: (f32, f32),
    to: (f32, f32),
    steps: usize,
) {
    let (a0, delta, r0, r1) = cap_arc_angles_and_radii(center, from, to);
    let c_idx = verts.len() as u32;
    verts.push([center.0, center.1]);
    let mut prev_idx = verts.len() as u32;
    verts.push([from.0, from.1]);
    for s in 1..=steps {
        let frac = s as f32 / steps as f32;
        let ang = a0 + delta * frac;
        let r = r0 + (r1 - r0) * frac;
        let cur_idx = verts.len() as u32;
        verts.push([center.0 + r * ang.cos(), center.1 + r * ang.sin()]);
        indices.extend_from_slice(&[c_idx, prev_idx, cur_idx]);
        prev_idx = cur_idx;
    }
}

#[inline]
fn dist_sq(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    dx * dx + dy * dy
}

#[inline]
fn dist_m(a: (f32, f32), b: (f32, f32)) -> f32 {
    dist_sq(a, b).sqrt()
}

#[inline]
fn lerp_pt(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

#[inline]
fn unit_vec(from: (f32, f32), to: (f32, f32)) -> (f32, f32, f32) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let len = dx.hypot(dy);
    if len < EPS_LEN_M {
        (1.0, 0.0, 0.0)
    } else {
        (dx / len, dy / len, len)
    }
}

#[inline]
fn cross_2d(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn point_in_triangle_ccw(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let c1 = cross_2d(a, b, p);
    let c2 = cross_2d(b, c, p);
    let c3 = cross_2d(c, a, p);
    c1 >= -1e-6 && c2 >= -1e-6 && c3 >= -1e-6
}
