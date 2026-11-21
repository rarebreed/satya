//! Multi-topology natural outdoor sector grammars for `WoodlandCreek`, `RoadsideDitch`, and `WoodedPark`.
//!
//! Synthesizes multi-archetype cubic Bezier curves (sweeping C-arcs, early/late hooks, asymmetric
//! S-meanders, diagonal dogleg shelves, random-walk multi-bend curves, 3-lobe meanders, and rare
//! tight oxbow/switchback bends), coordinates perpendicular [`SurfaceFeatureKind::BridgeDeck`]
//! crossings with connected `DirtTrail` approaches, and emits organic ground contours and `360°` props.

use std::f32::consts::TAU;

use crate::map_outdoor_gen::{BridgeCrossingSpec, OutdoorBuilder, RectM, Rng};
use crate::map_surface_geom::distance_point_to_polyline_m;
use satya_schema::{PropKind, SurfaceFeatureKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurveProfile {
    WildStream,
    ForestTrail,
    DrainageSwale,
    CountryRoad,
    GentleWalk,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CurveSpec {
    pub(crate) profile: CurveProfile,
    pub(crate) max_lateral_m: f32,
    pub(crate) ribbon_half_w_m: f32,
    pub(crate) clamp_rect: RectM,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurveArchetype {
    SingleCamberArc,
    AsymmetricWhipBend,
    RandomPhaseSMeander,
    DoglegStep,
    MultiBendWander,
    DoubleLoopMeander,
    TightOxbowOrSwitchback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreekTopology {
    NorthSouthMeander,
    BraidedConfluence,
    DiagonalSweepRavine,
    BraidedIslandSplit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoadsideTopology {
    EastWestCountryCurve,
    NorthSouthRuralLane,
    SweepingBendWithPullout,
    DivergingDitchGully,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParkTopology {
    WindingCrossTrail,
    ForkedGlade,
    OrganicWarpedLoop,
    DiagonalGladeWithSpurs,
}

#[inline]
fn clamp_pt(pt: (f32, f32), rect: RectM) -> (f32, f32) {
    (
        pt.0.clamp(rect.x0, rect.x1.max(rect.x0)),
        pt.1.clamp(rect.y0, rect.y1.max(rect.y0)),
    )
}

#[inline]
fn cubic_bezier_pt(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    t: f32,
) -> (f32, f32) {
    let u = 1.0 - t;
    let uu = u * u;
    let tt = t * t;
    let w0 = uu * u;
    let w1 = 3.0 * uu * t;
    let w2 = 3.0 * u * tt;
    let w3 = tt * t;
    (
        w0 * p0.0 + w1 * p1.0 + w2 * p2.0 + w3 * p3.0,
        w0 * p0.1 + w1 * p1.1 + w2 * p2.1 + w3 * p3.1,
    )
}

/// Converts a sequence of `(x_m, y_m)` local knots into smooth C1 cubic Bezier spans.
fn build_hermite_cubic_spans(knots: &[(f32, f32)], tension: f32) -> Vec<[(f32, f32); 4]> {
    let n = knots.len();
    if n < 2 {
        return Vec::new();
    }
    let mut tangents = Vec::with_capacity(n);
    for i in 0..n {
        let t = if i == 0 {
            (knots[1].0 - knots[0].0, knots[1].1 - knots[0].1)
        } else if i + 1 == n {
            (
                knots[n - 1].0 - knots[n - 2].0,
                knots[n - 1].1 - knots[n - 2].1,
            )
        } else {
            (
                0.5 * (knots[i + 1].0 - knots[i - 1].0),
                0.5 * (knots[i + 1].1 - knots[i - 1].1),
            )
        };
        tangents.push(t);
    }
    let scale = tension / 3.0;
    let mut spans = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let p0 = knots[i];
        let p3 = knots[i + 1];
        let p1 = (p0.0 + tangents[i].0 * scale, p0.1 + tangents[i].1 * scale);
        let p2 = (
            p3.0 - tangents[i + 1].0 * scale,
            p3.1 - tangents[i + 1].1 * scale,
        );
        spans.push([p0, p1, p2, p3]);
    }
    spans
}

fn select_curve_archetype(
    rng: &mut Rng,
    profile: CurveProfile,
    chord_len_m: f32,
    spec: CurveSpec,
) -> CurveArchetype {
    let oxbow_eligible = matches!(
        profile,
        CurveProfile::WildStream | CurveProfile::ForestTrail
    ) && chord_len_m >= 10.0
        && spec.max_lateral_m >= (spec.ribbon_half_w_m * 2.4).max(2.3);
    if oxbow_eligible && rng.chance(6, 100) {
        return CurveArchetype::TightOxbowOrSwitchback;
    }
    match profile {
        CurveProfile::CountryRoad | CurveProfile::GentleWalk => match rng.range(0, 9) {
            0..=2 => CurveArchetype::SingleCamberArc,
            3..=4 => CurveArchetype::RandomPhaseSMeander,
            5..=6 => CurveArchetype::AsymmetricWhipBend,
            7 => CurveArchetype::DoglegStep,
            _ => CurveArchetype::DoubleLoopMeander,
        },
        CurveProfile::WildStream | CurveProfile::ForestTrail | CurveProfile::DrainageSwale => {
            match rng.range(0, 6) {
                0 => CurveArchetype::SingleCamberArc,
                1 => CurveArchetype::AsymmetricWhipBend,
                2 => CurveArchetype::RandomPhaseSMeander,
                3 => CurveArchetype::DoglegStep,
                4 => CurveArchetype::MultiBendWander,
                _ => CurveArchetype::DoubleLoopMeander,
            }
        }
    }
}

/// Checks how far `(start + 0.5*chord) ± side*n` extends before hitting `clamp_rect`, picking a
/// side that can fit `needed_lat_m` if the random side would clip against a border.
fn choose_unclipped_side(
    rng: &mut Rng,
    mid: (f32, f32),
    normal: (f32, f32),
    needed_lat_m: f32,
    rect: RectM,
) -> f32 {
    let room = |sign: f32| -> f32 {
        let mut r = f32::INFINITY;
        if (normal.0 * sign) > 1e-4 {
            r = r.min((rect.x1 - mid.0) / (normal.0 * sign));
        } else if (normal.0 * sign) < -1e-4 {
            r = r.min((mid.0 - rect.x0) / (-normal.0 * sign));
        }
        if (normal.1 * sign) > 1e-4 {
            r = r.min((rect.y1 - mid.1) / (normal.1 * sign));
        } else if (normal.1 * sign) < -1e-4 {
            r = r.min((mid.1 - rect.y0) / (-normal.1 * sign));
        }
        r.max(0.0)
    };
    let pos_room = room(1.0);
    let neg_room = room(-1.0);
    if pos_room >= needed_lat_m && neg_room >= needed_lat_m {
        if rng.chance(1, 2) { 1.0 } else { -1.0 }
    } else if pos_room > neg_room + 0.25 {
        1.0
    } else if neg_room > pos_room + 0.25 {
        -1.0
    } else if rng.chance(1, 2) {
        1.0
    } else {
        -1.0
    }
}

/// Synthesizes diverse C1 cubic Bezier polylines from `start_m` to `end_m` across 7 archetypes.
pub(crate) fn synthesize_natural_curve_m(
    rng: &mut Rng,
    start_m: (f32, f32),
    end_m: (f32, f32),
    spec: CurveSpec,
) -> Vec<(f32, f32)> {
    let dx = end_m.0 - start_m.0;
    let dy = end_m.1 - start_m.1;
    let len = dx.hypot(dy);
    if len < 1.0 {
        return vec![start_m, end_m];
    }
    let ux = dx / len;
    let uy = dy / len;
    let nx = -uy;
    let ny = ux;

    let prof_scale = match spec.profile {
        CurveProfile::WildStream => 1.0,
        CurveProfile::ForestTrail => 0.92,
        CurveProfile::DrainageSwale => 0.78,
        CurveProfile::CountryRoad => 0.58,
        CurveProfile::GentleWalk => 0.55,
    };
    let max_normal_lat = spec.max_lateral_m.min(len * 0.20);
    let amp = (max_normal_lat * prof_scale).max(0.05);
    let mid_world = (start_m.0 + dx * 0.5, start_m.1 + dy * 0.5);
    let archetype = if len < 4.0 {
        if rng.chance(1, 2) {
            CurveArchetype::SingleCamberArc
        } else {
            CurveArchetype::RandomPhaseSMeander
        }
    } else {
        select_curve_archetype(rng, spec.profile, len, spec)
    };

    let side = if archetype == CurveArchetype::TightOxbowOrSwitchback {
        choose_unclipped_side(rng, mid_world, (nx, ny), 2.3, spec.clamp_rect)
    } else if rng.chance(1, 2) {
        1.0
    } else {
        -1.0
    };

    // Build cubic spans in local meter space `(x_loc in [0, len], y_loc)`.
    let spans: Vec<[(f32, f32); 4]> = match archetype {
        CurveArchetype::SingleCamberArc => {
            let t1 = rng.f32_range(0.18, 0.42);
            let t2 = rng.f32_range(0.58, 0.84);
            let y1 = side * amp * rng.f32_range(0.55, 1.32);
            let y2 = side * amp * rng.f32_range(0.45, 1.28);
            vec![[(0.0, 0.0), (len * t1, y1), (len * t2, y2), (len, 0.0)]]
        }
        CurveArchetype::AsymmetricWhipBend => {
            let late = rng.chance(1, 2);
            let (p1, p2) = if late {
                (
                    (
                        len * rng.f32_range(0.32, 0.52),
                        side * amp * rng.f32_range(-0.18, 0.14),
                    ),
                    (
                        len * rng.f32_range(0.68, 0.86),
                        side * amp * rng.f32_range(0.85, 1.35),
                    ),
                )
            } else {
                (
                    (
                        len * rng.f32_range(0.14, 0.32),
                        side * amp * rng.f32_range(0.85, 1.35),
                    ),
                    (
                        len * rng.f32_range(0.48, 0.68),
                        side * amp * rng.f32_range(-0.18, 0.14),
                    ),
                )
            };
            vec![[(0.0, 0.0), p1, p2, (len, 0.0)]]
        }
        CurveArchetype::RandomPhaseSMeander => {
            let t1 = rng.f32_range(0.20, 0.44);
            let t2 = rng.f32_range(0.56, 0.82);
            let y1 = side * amp * rng.f32_range(0.50, 1.35);
            let y2 = -side * amp * rng.f32_range(0.45, 1.35);
            vec![[(0.0, 0.0), (len * t1, y1), (len * t2, y2), (len, 0.0)]]
        }
        CurveArchetype::DoglegStep => {
            let s1 = rng.f32_range(0.24, 0.42);
            let s2 = rng.f32_range(0.58, 0.78);
            let y1 = side * amp * rng.f32_range(0.55, 0.96);
            let y2 = if rng.chance(2, 3) {
                side * amp * rng.f32_range(0.35, 0.85)
            } else {
                -side * amp * rng.f32_range(0.25, 0.60)
            };
            build_hermite_cubic_spans(
                &[(0.0, 0.0), (len * s1, y1), (len * s2, y2), (len, 0.0)],
                0.85,
            )
        }
        CurveArchetype::MultiBendWander => {
            let k = rng.range(3, 6) as usize;
            let mut knots = Vec::with_capacity(k + 2);
            knots.push((0.0, 0.0));
            let mut cur_sign = side;
            let mut y_cur = 0.0_f32;
            for i in 0..k {
                let base_s = (i + 1) as f32 / (k + 1) as f32;
                let jitter_s = (0.32 / (k + 1) as f32).min(0.08);
                let s = (base_s + rng.f32_range(-jitter_s, jitter_s)).clamp(0.08, 0.92);
                if i > 0 && rng.chance(3, 5) {
                    cur_sign = -cur_sign;
                }
                let target_y = cur_sign * amp * rng.f32_range(0.32, 0.98);
                y_cur = (0.28 * y_cur + 0.72 * target_y).clamp(-amp, amp);
                knots.push((len * s, y_cur));
            }
            knots.push((len, 0.0));
            build_hermite_cubic_spans(&knots, 1.0)
        }
        CurveArchetype::DoubleLoopMeander => {
            let s1 = rng.f32_range(0.24, 0.38);
            let s2 = rng.f32_range(0.62, 0.76);
            let y1 = side * amp * rng.f32_range(0.52, 0.98);
            let y2 = -side * amp * rng.f32_range(0.52, 0.98);
            let y_mid = side * amp * rng.f32_range(-0.35, 0.45);
            let s_mid = 0.5 * (s1 + s2) + rng.f32_range(-0.04, 0.04);
            build_hermite_cubic_spans(
                &[
                    (0.0, 0.0),
                    (len * s1, y1),
                    (len * s_mid, y_mid),
                    (len * s2, y2),
                    (len, 0.0),
                ],
                1.05,
            )
        }
        CurveArchetype::TightOxbowOrSwitchback => {
            let s_mid = rng.f32_range(0.38, 0.62);
            let apex_lat = (spec.max_lateral_m * rng.f32_range(0.84, 0.98)).max(2.3);
            let y_apex = side * apex_lat;
            let w_s = ((spec.ribbon_half_w_m * 2.1).max(2.0) / len).clamp(0.11, 0.22);
            let belly_s = (w_s * rng.f32_range(1.28, 1.65)).min(0.27);
            let neck_y = 0.18 * y_apex;
            let shoulder_y = 0.55 * y_apex;
            let left_neck = s_mid - w_s;
            let right_neck = s_mid + w_s;
            let c1_x = (left_neck + 0.35 * (belly_s - w_s)) * len;
            let c2_x = (right_neck - 0.35 * (belly_s - w_s)) * len;
            vec![
                [
                    (0.0, 0.0),
                    (left_neck * 0.45 * len, 0.0),
                    (c1_x, 0.05 * y_apex),
                    (left_neck * len, neck_y),
                ],
                [
                    (left_neck * len, neck_y),
                    ((s_mid - belly_s) * len, shoulder_y),
                    ((s_mid - belly_s * 0.78) * len, y_apex),
                    (s_mid * len, y_apex),
                ],
                [
                    (s_mid * len, y_apex),
                    ((s_mid + belly_s * 0.78) * len, y_apex),
                    ((s_mid + belly_s) * len, shoulder_y),
                    (right_neck * len, neck_y),
                ],
                [
                    (right_neck * len, neck_y),
                    (c2_x, 0.05 * y_apex),
                    ((1.0 - 0.45 * (1.0 - right_neck)) * len, 0.0),
                    (len, 0.0),
                ],
            ]
        }
    };

    let target_total = if archetype == CurveArchetype::TightOxbowOrSwitchback {
        13
    } else {
        ((len / 2.2).round() as usize).clamp(6, 12)
    };
    let steps_per_span = if spans.len() == 1 {
        target_total - 1
    } else {
        ((target_total.max(spans.len() * 3) - 1) / spans.len()).max(3)
    };
    let wobble_amp = if matches!(
        archetype,
        CurveArchetype::TightOxbowOrSwitchback
            | CurveArchetype::SingleCamberArc
            | CurveArchetype::AsymmetricWhipBend
    ) || matches!(
        spec.profile,
        CurveProfile::CountryRoad | CurveProfile::GentleWalk
    ) {
        0.0
    } else {
        (amp * 0.08).min(0.28)
    };

    let mut out = Vec::with_capacity(spans.len() * steps_per_span + 1);
    out.push(start_m);
    for (si, &[p0, p1, p2, p3]) in spans.iter().enumerate() {
        let is_last_span = si + 1 == spans.len();
        for step in 1..=steps_per_span {
            if is_last_span && step == steps_per_span {
                out.push(end_m);
                continue;
            }
            let t = step as f32 / steps_per_span as f32;
            let (lx, mut ly) = cubic_bezier_pt(p0, p1, p2, p3, t);
            if wobble_amp > 0.0 {
                ly += rng.f32_range(-wobble_amp, wobble_amp);
            }
            let wx = start_m.0 + ux * lx + nx * ly;
            let wy = start_m.1 + uy * lx + ny * ly;
            out.push(clamp_pt((wx, wy), spec.clamp_rect));
        }
    }
    out
}

/// Offsets a polyline along each vertex's local normal via a low-frequency sine bar wave (for cobble
/// shoals on N-S, E-W, and diagonal creeks alike), keeping endpoints anchored cleanly.
pub(crate) fn offset_polyline_along_normal_m(
    pts: &[(f32, f32)],
    rng: &mut Rng,
    amp_m: f32,
    clamp_rect: RectM,
) -> Vec<(f32, f32)> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let phase = rng.f32_range(0.0, TAU);
    let freq = rng.f32_range(0.32, 0.56);
    let mut arc_m = 0.0_f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i > 0 {
            arc_m += (pts[i].0 - pts[i - 1].0).hypot(pts[i].1 - pts[i - 1].1);
        }
        if i == 0 || i + 1 == n {
            out.push(pts[i]);
            continue;
        }
        let (dx, dy) = (pts[i + 1].0 - pts[i - 1].0, pts[i + 1].1 - pts[i - 1].1);
        let dlen = dx.hypot(dy).max(1e-4);
        let (nx, ny) = (-dy / dlen, dx / dlen);
        let shift = amp_m * (phase + arc_m * freq).sin();
        out.push(clamp_pt(
            (pts[i].0 + nx * shift, pts[i].1 + ny * shift),
            clamp_rect,
        ));
    }
    out
}

fn push_away_from_keepout(
    pt: (f32, f32),
    bank_anchor: (f32, f32),
    keepout_pts: &[(f32, f32)],
    min_clear_m: f32,
    clamp_rect: RectM,
) -> (f32, f32) {
    if keepout_pts.len() < 2 || distance_point_to_polyline_m(pt.0, pt.1, keepout_pts) >= min_clear_m
    {
        return pt;
    }
    let mut best_q = keepout_pts[0];
    let mut best_d = f32::INFINITY;
    for w in keepout_pts.windows(2) {
        let (d, t) = crate::tactical_map_geom::distance_point_to_segment(pt.0, pt.1, w[0], w[1]);
        if d < best_d {
            best_d = d;
            best_q = (
                w[0].0 + (w[1].0 - w[0].0) * t,
                w[0].1 + (w[1].1 - w[0].1) * t,
            );
        }
    }
    let to_pt = (pt.0 - best_q.0, pt.1 - best_q.1);
    let to_bank = (bank_anchor.0 - best_q.0, bank_anchor.1 - best_q.1);
    let raw_dir =
        if to_pt.0 * to_bank.0 + to_pt.1 * to_bank.1 > 0.0 && to_pt.0.hypot(to_pt.1) > 1e-3 {
            to_pt
        } else {
            to_bank
        };
    let dir_len = raw_dir.0.hypot(raw_dir.1).max(1e-4);
    let ux = raw_dir.0 / dir_len;
    let uy = raw_dir.1 / dir_len;
    clamp_pt(
        (
            best_q.0 + ux * (min_clear_m + 0.25),
            best_q.1 + uy * (min_clear_m + 0.25),
        ),
        clamp_rect,
    )
}

/// Builds a smooth C1 cubic-Bezier trail leg from `abut` outward to `far_pt` (returned in
/// `abut -> ... -> far_pt` order).
fn smooth_bridge_approach_leg(
    rng: &mut Rng,
    abut: (f32, f32),
    outward_dir: (f32, f32),
    far_pt: (f32, f32),
    clamp_rect: RectM,
    keepout: Option<(&[(f32, f32)], f32)>,
) -> Vec<(f32, f32)> {
    let dx = far_pt.0 - abut.0;
    let dy = far_pt.1 - abut.1;
    let dist = dx.hypot(dy);
    if dist < 0.05 {
        return vec![abut];
    }
    if dist < 1.6 {
        return vec![abut, far_pt];
    }
    let ux = dx / dist;
    let uy = dy / dist;
    let nx = -uy;
    let ny = ux;
    let h_bridge = (dist * 0.36).clamp(1.0, 3.0);
    let c1 = clamp_pt(
        (
            abut.0 + outward_dir.0 * h_bridge,
            abut.1 + outward_dir.1 * h_bridge,
        ),
        clamp_rect,
    );
    let h_outer = (dist * 0.34).clamp(0.8, 3.0);
    let wobble = rng.f32_range(-1.0, 1.0) * (dist * 0.22);
    let c2 = clamp_pt(
        (
            far_pt.0 - ux * h_outer + nx * wobble,
            far_pt.1 - uy * h_outer + ny * wobble,
        ),
        clamp_rect,
    );
    let steps = 6_usize;
    let mut pts = Vec::with_capacity(steps + 1);
    pts.push(abut);
    for i in 1..steps {
        let t = i as f32 / steps as f32;
        let mut p = clamp_pt(cubic_bezier_pt(abut, c1, c2, far_pt, t), clamp_rect);
        if let Some((k_pts, min_clear_m)) = keepout {
            p = push_away_from_keepout(p, far_pt, k_pts, min_clear_m, clamp_rect);
        }
        pts.push(p);
    }
    pts.push(far_pt);
    pts
}

/// Routes a meandering `DirtTrail` from `start_m` squarely across `bridge` to `end_m`.
pub(crate) fn route_trail_across_bridge_m(
    rng: &mut Rng,
    start_m: (f32, f32),
    bridge: BridgeCrossingSpec,
    end_m: (f32, f32),
    clamp_rect: RectM,
    keepout: Option<(&[(f32, f32)], f32)>,
) -> Vec<(f32, f32)> {
    let dot_in = (start_m.0 - bridge.center_m.0) * bridge.span_dir.0
        + (start_m.1 - bridge.center_m.1) * bridge.span_dir.1;
    let (abut_in, abut_out, in_sign, out_sign) = if dot_in <= 0.0 {
        (bridge.abutment_neg_m, bridge.abutment_pos_m, -1.0, 1.0)
    } else {
        (bridge.abutment_pos_m, bridge.abutment_neg_m, 1.0, -1.0)
    };

    let mut entry = smooth_bridge_approach_leg(
        rng,
        abut_in,
        (bridge.span_dir.0 * in_sign, bridge.span_dir.1 * in_sign),
        start_m,
        clamp_rect,
        keepout,
    );
    entry.reverse();

    let exit = smooth_bridge_approach_leg(
        rng,
        abut_out,
        (bridge.span_dir.0 * out_sign, bridge.span_dir.1 * out_sign),
        end_m,
        clamp_rect,
        keepout,
    );

    let mut out = Vec::with_capacity(entry.len() + 1 + exit.len());
    out.extend(entry);
    out.push(bridge.center_m);
    out.extend(exit);
    out
}

fn full_rect(b: &OutdoorBuilder, inset: f32) -> RectM {
    RectM {
        x0: inset,
        y0: inset,
        x1: (b.w as f32 - inset).max(inset + 1.0),
        y1: (b.h as f32 - inset).max(inset + 1.0),
    }
}

fn place_rot_m(
    b: &mut OutdoorBuilder,
    kind: PropKind,
    x_m: f32,
    y_m: f32,
    lo: i32,
    hi: i32,
    allow_on_pavement: bool,
) {
    let rot = b.rng.range(lo, hi);
    b.try_place_cm(
        kind,
        OutdoorBuilder::cm(x_m),
        OutdoorBuilder::cm(y_m),
        rot,
        None,
        allow_on_pavement,
    );
}

fn scatter_organic_beds(b: &mut OutdoorBuilder, count: i32, bounds: RectM, include_garden: bool) {
    for i in 0..count {
        let px = b
            .rng
            .f32_range(bounds.x0 + 1.0, (bounds.x1 - 1.0).max(bounds.x0 + 1.5));
        let py = b
            .rng
            .f32_range(bounds.y0 + 1.0, (bounds.y1 - 1.0).max(bounds.y0 + 1.5));
        let kind = if include_garden && i % 2 == 1 {
            SurfaceFeatureKind::GardenBedZone
        } else {
            SurfaceFeatureKind::MulchZone
        };
        let rx = b.rng.f32_range(1.0, 2.2);
        let ry = b.rng.f32_range(0.85, 1.85);
        b.try_add_organic_patch(kind, (px, py), (rx, ry), 0.35, 36);
    }
}

fn add_bridge_connector_spur(
    b: &mut OutdoorBuilder,
    bridge: BridgeCrossingSpec,
    clamp_rect: RectM,
) {
    let ext = bridge.half_span_m + 1.45;
    let p_a = clamp_pt(
        (
            bridge.center_m.0 - bridge.span_dir.0 * ext,
            bridge.center_m.1 - bridge.span_dir.1 * ext,
        ),
        clamp_rect,
    );
    let p_b = clamp_pt(
        (
            bridge.center_m.0 + bridge.span_dir.0 * ext,
            bridge.center_m.1 + bridge.span_dir.1 * ext,
        ),
        clamp_rect,
    );
    b.add_ribbon(
        SurfaceFeatureKind::DirtTrail,
        &[
            p_a,
            bridge.abutment_neg_m,
            bridge.center_m,
            bridge.abutment_pos_m,
            p_b,
        ],
        175,
    );
}

/// Winding `WoodlandCreek`: incised ravine gorge, cobble shoal bars, live creek(s), rim boulders & perpendicular footbridge(s).
pub(crate) fn build_woodland_creek(b: &mut OutdoorBuilder) {
    let (wf, hf) = (b.w as f32, b.h as f32);
    let topo = if b.w >= 26 && b.h >= 20 {
        match b.rng.range(0, 4) {
            0 => CreekTopology::NorthSouthMeander,
            1 => CreekTopology::BraidedConfluence,
            2 => CreekTopology::DiagonalSweepRavine,
            _ => CreekTopology::BraidedIslandSplit,
        }
    } else {
        match b.rng.range(0, 3) {
            0 => CreekTopology::NorthSouthMeander,
            1 => CreekTopology::BraidedConfluence,
            _ => CreekTopology::DiagonalSweepRavine,
        }
    };

    let creek_w_cm = (240 + (b.w - 20).max(0) * 3).clamp(240, 340);
    let shoal_w_cm = creek_w_cm + 135;
    let ravine_w_cm = creek_w_cm + (260 + (b.w - 20).max(0) * 4).clamp(260, 380);
    let bridge_span_m = (ravine_w_cm as f32 / 100.0 + 0.65).clamp(5.0, 8.2);
    let channel_clamp = RectM {
        x0: 2.2,
        y0: 1.4,
        x1: (wf - 2.2).max(3.0),
        y1: (hf - 1.4).max(3.0),
    };
    let trail_clamp = full_rect(b, 0.8);
    let half_ravine = ravine_w_cm as f32 / 200.0;

    let primary_creek = match topo {
        CreekTopology::NorthSouthMeander => {
            let x0 = wf * b.rng.f32_range(0.28, 0.72);
            let x1 = wf * b.rng.f32_range(0.28, 0.72);
            let edge_room = x0.min(wf - x0).min(x1).min(wf - x1);
            let max_lat = (edge_room - 2.0).clamp(2.2, wf * 0.24);
            let pts = synthesize_natural_curve_m(
                &mut b.rng,
                (x0, -0.4),
                (x1, hf + 0.4),
                CurveSpec {
                    profile: CurveProfile::WildStream,
                    max_lateral_m: max_lat,
                    ribbon_half_w_m: creek_w_cm as f32 / 200.0,
                    clamp_rect: channel_clamp,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::DefensiveTrench, &pts, ravine_w_cm);
            let shoal_pts = offset_polyline_along_normal_m(&pts, &mut b.rng, 0.48, channel_clamp);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &shoal_pts, shoal_w_cm);
            let creek = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &pts, creek_w_cm);
            if b.h >= 20 {
                let target_i = (pts.len() / 3).clamp(1, pts.len() - 2);
                let target = pts[target_i];
                let flank_x = if target.0 >= wf * 0.5 { wf + 0.4 } else { -0.4 };
                let g_y0 = target.1 + b.rng.f32_range(-2.2, 2.2);
                let g_pts = synthesize_natural_curve_m(
                    &mut b.rng,
                    (flank_x, g_y0),
                    target,
                    CurveSpec {
                        profile: CurveProfile::DrainageSwale,
                        max_lateral_m: 1.4,
                        ribbon_half_w_m: 0.95,
                        clamp_rect: trail_clamp,
                    },
                );
                b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &g_pts, 195);
            }
            let pref_frac = b.rng.f32_range(0.38, 0.62);
            let ty0 = hf * b.rng.f32_range(0.24, 0.76);
            let ty1 = hf * b.rng.f32_range(0.24, 0.76);
            if let Some(bridge) = b.place_perpendicular_bridge(creek, pref_frac, 2.3, bridge_span_m)
            {
                let trail_pts = route_trail_across_bridge_m(
                    &mut b.rng,
                    (-0.3, ty0),
                    bridge,
                    (wf + 0.3, ty1),
                    trail_clamp,
                    Some((&pts, half_ravine + 0.55)),
                );
                b.add_ribbon(SurfaceFeatureKind::DirtTrail, &trail_pts, 185);
            }
            if b.h >= 24 {
                let second_frac = if pref_frac < 0.5 { 0.78 } else { 0.22 };
                if let Some(br2) =
                    b.place_perpendicular_bridge(creek, second_frac, 2.0, bridge_span_m * 0.88)
                {
                    add_bridge_connector_spur(b, br2, trail_clamp);
                }
            }
            creek
        }
        CreekTopology::BraidedConfluence => {
            let main_left = b.rng.chance(1, 2);
            let start_x = wf
                * if main_left {
                    b.rng.f32_range(0.22, 0.42)
                } else {
                    b.rng.f32_range(0.58, 0.78)
                };
            let end_x = wf * b.rng.f32_range(0.30, 0.70);
            let main_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (start_x, -0.4),
                (end_x, hf + 0.4),
                CurveSpec {
                    profile: CurveProfile::WildStream,
                    max_lateral_m: (wf * 0.17).clamp(1.4, 3.8),
                    ribbon_half_w_m: creek_w_cm as f32 / 200.0,
                    clamp_rect: channel_clamp,
                },
            );
            let j_idx =
                ((main_pts.len() * 2) / 5).clamp(1, main_pts.len().saturating_sub(3).max(1));
            let (jx, jy) = main_pts[j_idx];

            let trib_start = match b.rng.range(0, 3) {
                0 => (
                    wf * if main_left {
                        b.rng.f32_range(0.68, 0.86)
                    } else {
                        b.rng.f32_range(0.14, 0.32)
                    },
                    -0.4,
                ),
                1 => (
                    if main_left { wf + 0.4 } else { -0.4 },
                    hf * b.rng.f32_range(0.14, 0.40),
                ),
                _ => (wf * if main_left { 0.76 } else { 0.24 }, -0.4),
            };
            let trib_pts = synthesize_natural_curve_m(
                &mut b.rng,
                trib_start,
                (jx, jy),
                CurveSpec {
                    profile: CurveProfile::WildStream,
                    max_lateral_m: (wf * 0.12).clamp(1.0, 2.8),
                    ribbon_half_w_m: (creek_w_cm as f32 * 0.75) / 200.0,
                    clamp_rect: channel_clamp,
                },
            );
            let trib_ravine_cm = (ravine_w_cm * 3) / 4;
            let trib_creek_cm = (creek_w_cm * 3) / 4;
            b.add_ribbon(SurfaceFeatureKind::DefensiveTrench, &main_pts, ravine_w_cm);
            b.add_ribbon(
                SurfaceFeatureKind::DefensiveTrench,
                &trib_pts,
                trib_ravine_cm,
            );
            let main_shoal =
                offset_polyline_along_normal_m(&main_pts, &mut b.rng, 0.45, channel_clamp);
            let trib_shoal =
                offset_polyline_along_normal_m(&trib_pts, &mut b.rng, 0.36, channel_clamp);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &main_shoal, shoal_w_cm);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &trib_shoal, creek_w_cm + 80);
            let main_idx = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &main_pts, creek_w_cm);
            let trib_idx = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &trib_pts, trib_creek_cm);

            let ty0 = hf * b.rng.f32_range(0.55, 0.80);
            let ty1 = hf * b.rng.f32_range(0.55, 0.80);
            let main_frac = b.rng.f32_range(0.68, 0.82);
            if let Some(br_main) =
                b.place_perpendicular_bridge(main_idx, main_frac, 2.3, bridge_span_m)
            {
                let trail_pts = route_trail_across_bridge_m(
                    &mut b.rng,
                    (-0.3, ty0),
                    br_main,
                    (wf + 0.3, ty1),
                    trail_clamp,
                    Some((&main_pts, half_ravine + 0.55)),
                );
                b.add_ribbon(SurfaceFeatureKind::DirtTrail, &trail_pts, 190);
            }
            if b.w >= 28 && b.h >= 20 {
                let trib_frac = b.rng.f32_range(0.20, 0.36);
                if let Some(br_trib) =
                    b.place_perpendicular_bridge(trib_idx, trib_frac, 2.0, bridge_span_m * 0.82)
                {
                    add_bridge_connector_spur(b, br_trib, trail_clamp);
                }
            }
            main_idx
        }
        CreekTopology::DiagonalSweepRavine => {
            let flip = b.rng.chance(1, 2);
            let x_a = wf
                * if flip {
                    b.rng.f32_range(0.66, 0.84)
                } else {
                    b.rng.f32_range(0.16, 0.34)
                };
            let x_b = wf
                * if flip {
                    b.rng.f32_range(0.16, 0.34)
                } else {
                    b.rng.f32_range(0.66, 0.84)
                };
            let pts = synthesize_natural_curve_m(
                &mut b.rng,
                (x_a, -0.4),
                (x_b, hf + 0.4),
                CurveSpec {
                    profile: CurveProfile::WildStream,
                    max_lateral_m: (wf * 0.16).clamp(1.8, 4.5),
                    ribbon_half_w_m: creek_w_cm as f32 / 200.0,
                    clamp_rect: channel_clamp,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::DefensiveTrench, &pts, ravine_w_cm);
            let j_idx = (pts.len() / 4).clamp(1, pts.len() - 2);
            let gully_x0 = if flip { -0.4 } else { wf + 0.4 };
            let gully_y0 = hf * b.rng.f32_range(0.14, 0.28);
            let gully_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (gully_x0, gully_y0),
                pts[j_idx],
                CurveSpec {
                    profile: CurveProfile::DrainageSwale,
                    max_lateral_m: 1.3,
                    ribbon_half_w_m: 0.95,
                    clamp_rect: trail_clamp,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &gully_pts, 200);
            let shoal_pts = offset_polyline_along_normal_m(&pts, &mut b.rng, 0.48, channel_clamp);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &shoal_pts, shoal_w_cm);
            let creek = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &pts, creek_w_cm);

            let ty0 = hf
                * if flip {
                    b.rng.f32_range(0.24, 0.44)
                } else {
                    b.rng.f32_range(0.56, 0.76)
                };
            let ty1 = hf
                * if flip {
                    b.rng.f32_range(0.56, 0.76)
                } else {
                    b.rng.f32_range(0.24, 0.44)
                };
            let br_frac = b.rng.f32_range(0.40, 0.62);
            if let Some(bridge) = b.place_perpendicular_bridge(creek, br_frac, 2.3, bridge_span_m) {
                let trail_pts = route_trail_across_bridge_m(
                    &mut b.rng,
                    (-0.3, ty0),
                    bridge,
                    (wf + 0.3, ty1),
                    trail_clamp,
                    Some((&pts, half_ravine + 0.55)),
                );
                b.add_ribbon(SurfaceFeatureKind::DirtTrail, &trail_pts, 190);
            }
            creek
        }
        CreekTopology::BraidedIslandSplit => {
            let cx = wf * b.rng.f32_range(0.40, 0.60);
            let sx0 = cx + b.rng.f32_range(-1.6, 1.6);
            let sx1 = cx + b.rng.f32_range(-1.6, 1.6);
            let pts = synthesize_natural_curve_m(
                &mut b.rng,
                (sx0, -0.4),
                (sx1, hf + 0.4),
                CurveSpec {
                    profile: CurveProfile::WildStream,
                    max_lateral_m: (wf * 0.16).clamp(2.0, 4.4),
                    ribbon_half_w_m: creek_w_cm as f32 / 200.0,
                    clamp_rect: channel_clamp,
                },
            );
            let i0 = (pts.len() / 5).clamp(1, pts.len() - 4);
            let i1 = ((pts.len() * 7) / 10).clamp(i0 + 2, pts.len() - 2);
            let side_dir = if b.rng.chance(1, 2) { 1.0 } else { -1.0 };
            let island_off = half_ravine + 2.6;
            let p_a = pts[i0];
            let p_b = pts[i1];
            let mid_pt = (
                (0.5 * (p_a.0 + p_b.0) + side_dir * island_off).clamp(2.4, wf - 2.4),
                0.5 * (p_a.1 + p_b.1),
            );
            let mut branch_pts = synthesize_natural_curve_m(
                &mut b.rng,
                p_a,
                mid_pt,
                CurveSpec {
                    profile: CurveProfile::WildStream,
                    max_lateral_m: 1.2,
                    ribbon_half_w_m: (creek_w_cm as f32 * 0.68) / 200.0,
                    clamp_rect: channel_clamp,
                },
            );
            let branch_back = synthesize_natural_curve_m(
                &mut b.rng,
                mid_pt,
                p_b,
                CurveSpec {
                    profile: CurveProfile::WildStream,
                    max_lateral_m: 1.2,
                    ribbon_half_w_m: (creek_w_cm as f32 * 0.68) / 200.0,
                    clamp_rect: channel_clamp,
                },
            );
            branch_pts.extend(branch_back.into_iter().skip(1));

            let br_creek_cm = (creek_w_cm * 2) / 3;
            let br_ravine_cm = br_creek_cm + 190;
            b.add_ribbon(SurfaceFeatureKind::DefensiveTrench, &pts, ravine_w_cm);
            b.add_ribbon(
                SurfaceFeatureKind::DefensiveTrench,
                &branch_pts,
                br_ravine_cm,
            );
            let main_shoal = offset_polyline_along_normal_m(&pts, &mut b.rng, 0.46, channel_clamp);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &main_shoal, shoal_w_cm);
            let creek = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &pts, creek_w_cm);
            b.add_ribbon(SurfaceFeatureKind::CreekChannel, &branch_pts, br_creek_cm);

            let br_frac = b.rng.f32_range(0.76, 0.86);
            let ty0 = hf * b.rng.f32_range(0.60, 0.84);
            let ty1 = hf * b.rng.f32_range(0.60, 0.84);
            if let Some(bridge) = b.place_perpendicular_bridge(creek, br_frac, 2.3, bridge_span_m) {
                let trail_pts = route_trail_across_bridge_m(
                    &mut b.rng,
                    (-0.3, ty0),
                    bridge,
                    (wf + 0.3, ty1),
                    trail_clamp,
                    Some((&pts, half_ravine + 0.55)),
                );
                b.add_ribbon(SurfaceFeatureKind::DirtTrail, &trail_pts, 185);
            }
            creek
        }
    };

    let rim_offset = ravine_w_cm as f32 / 200.0 + 0.95;
    let rim_boulders = (b.h / 5).clamp(4, 10) as usize;
    for i in 0..rim_boulders {
        let frac = 0.10 + 0.80 * ((i as f32 + 0.35) / rim_boulders as f32);
        let side = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
        b.place_along_ribbon(
            PropKind::Boulder,
            primary_creek,
            frac,
            rim_offset * side,
            35,
            false,
        );
    }
    let rim_logs = (b.h / 7).clamp(2, 6) as usize;
    for i in 0..rim_logs {
        let frac = 0.16 + 0.68 * ((i as f32 + 0.5) / rim_logs as f32);
        let side = if i.is_multiple_of(2) { -1.0 } else { 1.0 };
        b.place_along_ribbon(
            PropKind::FallenLog,
            primary_creek,
            frac,
            rim_offset * side,
            22,
            false,
        );
    }

    let area_scale = (b.w * b.h / 80).clamp(3, 18);
    scatter_organic_beds(
        b,
        (area_scale / 2 + 2).clamp(3, 10),
        full_rect(b, 1.4),
        true,
    );
    b.scatter_trees(PropKind::LargeTree, area_scale + 2, 3.2, full_rect(b, 1.5));
    b.scatter_trees(PropKind::MediumTree, area_scale + 4, 2.4, full_rect(b, 1.3));
    b.scatter_turf_prop_rot(PropKind::Boulder, area_scale, 30, full_rect(b, 1.4));
    b.scatter_turf_prop_rot(PropKind::Bush, area_scale + 4, 30, full_rect(b, 1.2));
}

/// Offsets a road spline along its local normal toward `side_sign` with gentle natural swell while
/// guaranteeing at least `min_sep_m` clearance from the road centerline.
fn offset_ditch_from_road(
    road_pts: &[(f32, f32)],
    rng: &mut Rng,
    side_sign: f32,
    base_sep_m: f32,
    min_sep_m: f32,
    clamp_rect: RectM,
) -> Vec<(f32, f32)> {
    let n = road_pts.len();
    if n < 2 {
        return road_pts.to_vec();
    }
    let phase = rng.f32_range(0.0, TAU);
    let freq = rng.f32_range(0.28, 0.50);
    let swell_amp = rng.f32_range(0.25, 0.75);
    let mut arc_m = 0.0_f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i > 0 {
            arc_m += (road_pts[i].0 - road_pts[i - 1].0).hypot(road_pts[i].1 - road_pts[i - 1].1);
        }
        let (dx, dy) = if i == 0 {
            (road_pts[1].0 - road_pts[0].0, road_pts[1].1 - road_pts[0].1)
        } else if i + 1 == n {
            (
                road_pts[n - 1].0 - road_pts[n - 2].0,
                road_pts[n - 1].1 - road_pts[n - 2].1,
            )
        } else {
            (
                road_pts[i + 1].0 - road_pts[i - 1].0,
                road_pts[i + 1].1 - road_pts[i - 1].1,
            )
        };
        let dlen = dx.hypot(dy).max(1e-4);
        let (nx, ny) = (-dy / dlen, dx / dlen);
        let sep = (base_sep_m + swell_amp * (phase + arc_m * freq).sin()).max(min_sep_m);
        let raw_pt = (
            road_pts[i].0 + nx * side_sign * sep,
            road_pts[i].1 + ny * side_sign * sep,
        );
        if i == 0 || i + 1 == n {
            out.push(raw_pt);
        } else {
            out.push(clamp_pt(raw_pt, clamp_rect));
        }
    }
    out
}

/// Sweeping `RoadsideDitch`: curved `AsphaltRoad`, parallel gravel berm, drainage ditch, culvert bridge & cover.
pub(crate) fn build_roadside_ditch(b: &mut OutdoorBuilder) {
    let (wf, hf) = (b.w as f32, b.h as f32);
    let topo = match b.rng.range(0, 4) {
        0 => RoadsideTopology::EastWestCountryCurve,
        1 => RoadsideTopology::NorthSouthRuralLane,
        2 => RoadsideTopology::SweepingBendWithPullout,
        _ => RoadsideTopology::DivergingDitchGully,
    };
    let trail_clamp = full_rect(b, 0.8);

    let (road_idx, ditch_idx) = match topo {
        RoadsideTopology::EastWestCountryCurve => {
            let north_side = b.rng.chance(1, 2);
            let ry = if north_side { hf * 0.26 } else { hf * 0.74 };
            let road_rect = RectM {
                x0: 1.0,
                y0: if north_side { 1.8 } else { hf * 0.56 },
                x1: wf - 1.0,
                y1: if north_side { hf * 0.44 } else { hf - 1.8 },
            };
            let ry0 = ry + b.rng.f32_range(-0.6, 0.6);
            let ry1 = ry + b.rng.f32_range(-0.6, 0.6);
            let road_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (-0.4, ry0),
                (wf + 0.4, ry1),
                CurveSpec {
                    profile: CurveProfile::CountryRoad,
                    max_lateral_m: (hf * 0.14).clamp(1.0, 2.8),
                    ribbon_half_w_m: 1.75,
                    clamp_rect: road_rect,
                },
            );
            let side_sign = if north_side { 1.0 } else { -1.0 };
            let ditch_rect = RectM {
                x0: 1.0,
                y0: 1.8,
                x1: wf - 1.0,
                y1: (hf - 1.8).max(2.5),
            };
            let ditch_pts =
                offset_ditch_from_road(&road_pts, &mut b.rng, side_sign, 3.3, 2.9, ditch_rect);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &road_pts, 460);
            let road = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &road_pts, 350);
            let ditch = b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &ditch_pts, 200);
            let pref_frac = b.rng.f32_range(0.36, 0.64);
            if let Some(bridge) = b.place_perpendicular_bridge(ditch, pref_frac, 2.3, 2.8) {
                let start_pt = (
                    bridge.center_m.0 - bridge.span_dir.0 * (bridge.half_span_m + 0.8) * side_sign,
                    bridge.center_m.1 - bridge.span_dir.1 * (bridge.half_span_m + 0.8) * side_sign,
                );
                let end_x = (bridge.center_m.0 + b.rng.f32_range(-3.0, 3.0)).clamp(1.5, wf - 1.5);
                let end_pt = (end_x, if north_side { hf - 0.5 } else { 0.5 });
                let trail_pts = route_trail_across_bridge_m(
                    &mut b.rng,
                    start_pt,
                    bridge,
                    end_pt,
                    trail_clamp,
                    Some((&ditch_pts, 1.5)),
                );
                b.add_ribbon(SurfaceFeatureKind::DirtTrail, &trail_pts, 185);
            }
            (road, ditch)
        }
        RoadsideTopology::NorthSouthRuralLane => {
            let west_side = b.rng.chance(1, 2);
            let rx = if west_side { wf * 0.28 } else { wf * 0.72 };
            let road_rect = RectM {
                x0: if west_side { 2.0 } else { wf * 0.54 },
                y0: 1.0,
                x1: if west_side { wf * 0.46 } else { wf - 2.0 },
                y1: hf - 1.0,
            };
            let rx0 = rx + b.rng.f32_range(-0.7, 0.7);
            let rx1 = rx + b.rng.f32_range(-0.7, 0.7);
            let road_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (rx0, -0.4),
                (rx1, hf + 0.4),
                CurveSpec {
                    profile: CurveProfile::CountryRoad,
                    max_lateral_m: (wf * 0.14).clamp(1.2, 3.0),
                    ribbon_half_w_m: 1.80,
                    clamp_rect: road_rect,
                },
            );
            // For downward +Y chord, right-hand normal (-dy, dx) points West (-X), left points East (+X).
            let side_sign = if west_side { -1.0 } else { 1.0 };
            let ditch_rect = RectM {
                x0: 2.0,
                y0: 1.0,
                x1: (wf - 2.0).max(3.0),
                y1: hf - 1.0,
            };
            let ditch_pts =
                offset_ditch_from_road(&road_pts, &mut b.rng, side_sign, 3.4, 3.0, ditch_rect);
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &road_pts, 480);
            let road = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &road_pts, 360);
            let ditch = b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &ditch_pts, 210);
            let pref_frac = b.rng.f32_range(0.36, 0.64);
            if let Some(bridge) = b.place_perpendicular_bridge(ditch, pref_frac, 2.4, 2.8) {
                let road_side_x = if west_side { -1.0 } else { 1.0 };
                let start_pt = (bridge.center_m.0 + road_side_x * 2.4, bridge.center_m.1);
                let end_y = (bridge.center_m.1 + b.rng.f32_range(-2.6, 2.6)).clamp(1.5, hf - 1.5);
                let end_pt = (if west_side { wf - 0.4 } else { 0.4 }, end_y);
                let trail_pts = route_trail_across_bridge_m(
                    &mut b.rng,
                    start_pt,
                    bridge,
                    end_pt,
                    trail_clamp,
                    Some((&ditch_pts, 1.5)),
                );
                b.add_ribbon(SurfaceFeatureKind::DirtTrail, &trail_pts, 190);
            }
            (road, ditch)
        }
        RoadsideTopology::SweepingBendWithPullout => {
            let flip = b.rng.chance(1, 2);
            let start = if flip {
                (-0.4, hf * b.rng.f32_range(0.20, 0.34))
            } else {
                (-0.4, hf * b.rng.f32_range(0.66, 0.80))
            };
            let end = if flip {
                (wf * b.rng.f32_range(0.68, 0.84), hf + 0.4)
            } else {
                (wf * b.rng.f32_range(0.68, 0.84), -0.4)
            };
            let road_rect = full_rect(b, 2.2);
            let road_pts = synthesize_natural_curve_m(
                &mut b.rng,
                start,
                end,
                CurveSpec {
                    profile: CurveProfile::CountryRoad,
                    max_lateral_m: (hf.min(wf) * 0.22).clamp(1.6, 3.8),
                    ribbon_half_w_m: 1.90,
                    clamp_rect: road_rect,
                },
            );
            let side_sign = if flip { 1.0 } else { -1.0 };
            let ditch_rect = full_rect(b, 1.8);
            let ditch_pts =
                offset_ditch_from_road(&road_pts, &mut b.rng, side_sign, 3.4, 3.0, ditch_rect);
            let pull_i = (road_pts.len() / 3).clamp(1, road_pts.len() - 2);
            let pull_c = road_pts[pull_i];
            let pull_sign = -side_sign;
            let py_off = pull_sign * 2.4;
            let pull_poly = [
                (
                    (pull_c.0 - 2.3).clamp(0.8, wf - 3.0),
                    (pull_c.1 + py_off * 0.35).clamp(0.8, hf - 0.8),
                ),
                (
                    (pull_c.0 + 2.3).clamp(3.0, wf - 0.8),
                    (pull_c.1 + py_off * 0.35).clamp(0.8, hf - 0.8),
                ),
                (
                    (pull_c.0 + 1.8).clamp(3.0, wf - 0.8),
                    (pull_c.1 + py_off * 1.15).clamp(0.8, hf - 0.8),
                ),
                (
                    (pull_c.0 - 1.8).clamp(0.8, wf - 3.0),
                    (pull_c.1 + py_off * 1.15).clamp(0.8, hf - 0.8),
                ),
            ];
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &road_pts, 520);
            b.add_poly(SurfaceFeatureKind::GravelRoad, &pull_poly, 90);
            let road = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &road_pts, 380);
            let ditch = b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &ditch_pts, 210);
            let br_frac = b.rng.f32_range(0.42, 0.62);
            if let Some(bridge) = b.place_perpendicular_bridge(ditch, br_frac, 2.3, 2.8) {
                let start_pt = road_pts[(road_pts.len() / 2).min(road_pts.len() - 1)];
                let end_x = wf * b.rng.f32_range(0.12, 0.28);
                let end_pt = (end_x, if flip { hf * 0.72 } else { hf * 0.28 });
                let trail_pts = route_trail_across_bridge_m(
                    &mut b.rng,
                    start_pt,
                    bridge,
                    end_pt,
                    trail_clamp,
                    Some((&ditch_pts, 1.5)),
                );
                b.add_ribbon(SurfaceFeatureKind::DirtTrail, &trail_pts, 185);
            }
            (road, ditch)
        }
        RoadsideTopology::DivergingDitchGully => {
            let north_road = b.rng.chance(1, 2);
            let ry = if north_road { hf * 0.20 } else { hf * 0.80 };
            let road_rect = RectM {
                x0: 1.0,
                y0: if north_road { 1.4 } else { hf * 0.64 },
                x1: wf - 1.0,
                y1: if north_road { hf * 0.36 } else { hf - 1.4 },
            };
            let ry_end = ry + b.rng.f32_range(-0.4, 0.4);
            let road_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (-0.4, ry),
                (wf + 0.4, ry_end),
                CurveSpec {
                    profile: CurveProfile::CountryRoad,
                    max_lateral_m: 1.4,
                    ribbon_half_w_m: 1.75,
                    clamp_rect: road_rect,
                },
            );
            let ditch_y0 = if north_road { ry + 3.1 } else { ry - 3.1 };
            let ditch_y1 = if north_road {
                hf * b.rng.f32_range(0.55, 0.78)
            } else {
                hf * b.rng.f32_range(0.22, 0.45)
            };
            let ditch_rect = RectM {
                x0: 1.2,
                y0: if north_road { ry + 3.0 } else { 1.8 },
                x1: wf - 1.2,
                y1: if north_road { hf - 1.8 } else { ry - 3.0 },
            };
            let ditch_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (-0.4, ditch_y0),
                (wf + 0.4, ditch_y1),
                CurveSpec {
                    profile: CurveProfile::DrainageSwale,
                    max_lateral_m: (hf * 0.16).clamp(1.1, 3.0),
                    ribbon_half_w_m: 1.05,
                    clamp_rect: ditch_rect,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::GravelRoad, &road_pts, 470);
            let road = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &road_pts, 355);
            let ditch = b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &ditch_pts, 205);
            let br_frac = b.rng.f32_range(0.34, 0.62);
            if let Some(bridge) = b.place_perpendicular_bridge(ditch, br_frac, 2.3, 2.8) {
                let start_pt = (bridge.center_m.0, ry);
                let end_x = (bridge.center_m.0 + b.rng.f32_range(-2.5, 2.5)).clamp(1.6, wf - 1.6);
                let end_pt = (end_x, if north_road { hf - 0.5 } else { 0.5 });
                let trail_pts = route_trail_across_bridge_m(
                    &mut b.rng,
                    start_pt,
                    bridge,
                    end_pt,
                    trail_clamp,
                    Some((&ditch_pts, 1.5)),
                );
                b.add_ribbon(SurfaceFeatureKind::DirtTrail, &trail_pts, 185);
            }
            (road, ditch)
        }
    };

    b.place_vehicle_along_ribbon(PropKind::Car, road_idx, 0.30, 0.15, 7);
    b.place_vehicle_along_ribbon(PropKind::UtilityVan, road_idx, 0.72, -0.2, 8);
    b.place_along_ribbon(PropKind::JerseyBarrier, road_idx, 0.48, -2.05, 4, true);
    b.place_along_ribbon(PropKind::FallenLog, ditch_idx, 0.25, 1.4, 22, false);

    let area_scale = (b.w * b.h / 90).clamp(3, 16);
    scatter_organic_beds(b, (area_scale / 2 + 2).clamp(3, 9), full_rect(b, 1.4), true);
    b.scatter_trees(PropKind::MediumTree, area_scale + 2, 2.6, full_rect(b, 1.4));
    b.scatter_trees(
        PropKind::LargeTree,
        area_scale / 2 + 1,
        3.6,
        full_rect(b, 1.6),
    );
    b.scatter_turf_prop_rot(PropKind::WoodCrate, 3, 25, full_rect(b, 2.0));
    b.scatter_turf_prop_rot(PropKind::Bush, area_scale + 3, 35, full_rect(b, 1.2));
}

fn warped_plaza_poly(rng: &mut Rng, cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<(f32, f32)> {
    let steps = rng.range(6, 9) as usize;
    let base_rot = rng.f32_range(0.0, TAU);
    (0..steps)
        .map(|i| {
            let ang = base_rot + TAU * (i as f32 / steps as f32) + rng.f32_range(-0.15, 0.15);
            let wob = rng.f32_range(0.82, 1.18);
            (cx + rx * wob * ang.cos(), cy + ry * wob * ang.sin())
        })
        .collect()
}

/// Winding `WoodedPark`: bordering road, meandering & branching trails, warped plaza/loop, benches & canopy.
pub(crate) fn build_wooded_park(b: &mut OutdoorBuilder) {
    let (wf, hf) = (b.w as f32, b.h as f32);
    let topo = match b.rng.range(0, 4) {
        0 => ParkTopology::WindingCrossTrail,
        1 => ParkTopology::ForkedGlade,
        2 => ParkTopology::OrganicWarpedLoop,
        _ => ParkTopology::DiagonalGladeWithSpurs,
    };

    let north_road = b.rng.chance(1, 2);
    let ry = if north_road { 1.75 } else { hf - 1.75 };
    let road_rect = RectM {
        x0: 1.0,
        y0: if north_road {
            1.15
        } else {
            (hf - 3.35).max(hf * 0.65)
        },
        x1: wf - 1.0,
        y1: if north_road {
            3.35_f32.min(hf * 0.35)
        } else {
            hf - 1.15
        },
    };
    let road_y0 = ry + b.rng.f32_range(-0.25, 0.25);
    let road_y1 = ry + b.rng.f32_range(-0.25, 0.25);
    let road_pts = synthesize_natural_curve_m(
        &mut b.rng,
        (-0.4, road_y0),
        (wf + 0.4, road_y1),
        CurveSpec {
            profile: CurveProfile::CountryRoad,
            max_lateral_m: 0.75,
            ribbon_half_w_m: 1.65,
            clamp_rect: road_rect,
        },
    );
    let road = b.add_ribbon(SurfaceFeatureKind::AsphaltRoad, &road_pts, 335);

    let mut clear_x = wf * b.rng.f32_range(0.36, 0.64);
    let mut clear_y = if north_road {
        hf * b.rng.f32_range(0.50, 0.66)
    } else {
        hf * b.rng.f32_range(0.34, 0.50)
    };
    let park_rect = RectM {
        x0: 1.0,
        y0: if north_road { 3.8 } else { 1.2 },
        x1: (wf - 1.0).max(3.0),
        y1: if north_road {
            hf - 1.2
        } else {
            (hf - 3.8).max(3.0)
        },
    };
    let walk_clamp = full_rect(b, 0.9);

    match topo {
        ParkTopology::WindingCrossTrail => {
            let ty_l = b.rng.f32_range(park_rect.y0 + 0.8, park_rect.y1 - 0.8);
            let ty_r = b.rng.f32_range(park_rect.y0 + 0.8, park_rect.y1 - 0.8);
            let full_trail = synthesize_natural_curve_m(
                &mut b.rng,
                (-0.3, ty_l),
                (wf + 0.3, ty_r),
                CurveSpec {
                    profile: CurveProfile::ForestTrail,
                    max_lateral_m: (hf * 0.22).clamp(1.2, 4.2),
                    ribbon_half_w_m: 0.98,
                    clamp_rect: park_rect,
                },
            );
            (clear_x, clear_y) = full_trail[full_trail.len() / 2];
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &full_trail, 195);

            let walk_sx = clear_x + b.rng.f32_range(-1.4, 1.4);
            let walk_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (walk_sx, ry),
                (clear_x, clear_y),
                CurveSpec {
                    profile: CurveProfile::GentleWalk,
                    max_lateral_m: 1.4,
                    ribbon_half_w_m: 0.9,
                    clamp_rect: walk_clamp,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::StoneWalk, &walk_pts, 180);
            let plaza = warped_plaza_poly(&mut b.rng, clear_x, clear_y, 2.2, 1.7);
            b.add_poly(SurfaceFeatureKind::StoneWalk, &plaza, 100);
        }
        ParkTopology::ForkedGlade => {
            let ty_l = b.rng.f32_range(park_rect.y0 + 0.8, park_rect.y1 - 0.8);
            let ty_r = b.rng.f32_range(park_rect.y0 + 0.8, park_rect.y1 - 0.8);
            let main_trail = synthesize_natural_curve_m(
                &mut b.rng,
                (-0.3, ty_l),
                (wf + 0.3, ty_r),
                CurveSpec {
                    profile: CurveProfile::ForestTrail,
                    max_lateral_m: (hf * 0.24).clamp(1.4, 4.4),
                    ribbon_half_w_m: 1.0,
                    clamp_rect: park_rect,
                },
            );
            (clear_x, clear_y) = main_trail[main_trail.len() / 2];
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &main_trail, 200);

            let spur_y = if north_road { hf + 0.3 } else { -0.3 };
            let spur_x = wf * b.rng.f32_range(0.24, 0.78);
            let spur_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (clear_x, clear_y),
                (spur_x, spur_y),
                CurveSpec {
                    profile: CurveProfile::ForestTrail,
                    max_lateral_m: (wf * 0.14).clamp(1.0, 2.8),
                    ribbon_half_w_m: 0.9,
                    clamp_rect: park_rect,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &spur_pts, 180);

            let walk_sx = clear_x + b.rng.f32_range(-1.6, 1.6);
            let walk_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (walk_sx, ry),
                (clear_x, clear_y),
                CurveSpec {
                    profile: CurveProfile::GentleWalk,
                    max_lateral_m: 1.6,
                    ribbon_half_w_m: 0.92,
                    clamp_rect: walk_clamp,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::StoneWalk, &walk_pts, 185);
            let plaza = warped_plaza_poly(&mut b.rng, clear_x, clear_y, 2.3, 1.9);
            b.add_poly(SurfaceFeatureKind::StoneWalk, &plaza, 115);
        }
        ParkTopology::OrganicWarpedLoop => {
            let rx = (wf * 0.24).clamp(3.6, 8.6);
            let ry_l = (hf * 0.22).clamp(2.4, 5.8);
            let n_ring = 10usize;
            let phase = b.rng.f32_range(0.0, TAU);
            let mut loop_pts: Vec<(f32, f32)> = (0..n_ring)
                .map(|i| {
                    let ang = TAU * (i as f32 / n_ring as f32);
                    let harm =
                        1.0 + 0.18 * (2.0 * ang + phase).sin() + 0.12 * (3.0 * ang - phase).cos();
                    let jx = rx * harm * b.rng.f32_range(0.90, 1.12);
                    let jy = ry_l * harm * b.rng.f32_range(0.90, 1.12);
                    clamp_pt(
                        (clear_x + jx * ang.cos(), clear_y + jy * ang.sin()),
                        park_rect,
                    )
                })
                .collect();
            if let Some(&first) = loop_pts.first() {
                loop_pts.push(first);
            }
            b.add_ribbon(SurfaceFeatureKind::StoneWalk, &loop_pts, 180);

            // Connect East and West meandering DirtTrails directly into the loop ring.
            let east_anchor = loop_pts[0];
            let west_anchor = loop_pts[n_ring / 2];
            let west_y = b.rng.f32_range(park_rect.y0 + 0.8, park_rect.y1 - 0.8);
            let east_y = b.rng.f32_range(park_rect.y0 + 0.8, park_rect.y1 - 0.8);
            let west_trail = synthesize_natural_curve_m(
                &mut b.rng,
                (-0.3, west_y),
                west_anchor,
                CurveSpec {
                    profile: CurveProfile::ForestTrail,
                    max_lateral_m: 1.6,
                    ribbon_half_w_m: 0.92,
                    clamp_rect: park_rect,
                },
            );
            let east_trail = synthesize_natural_curve_m(
                &mut b.rng,
                east_anchor,
                (wf + 0.3, east_y),
                CurveSpec {
                    profile: CurveProfile::ForestTrail,
                    max_lateral_m: 1.6,
                    ribbon_half_w_m: 0.92,
                    clamp_rect: park_rect,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &west_trail, 185);
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &east_trail, 185);

            let road_anchor = if north_road {
                loop_pts[(n_ring * 3) / 4]
            } else {
                loop_pts[n_ring / 4]
            };
            let walk_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (clear_x, ry),
                road_anchor,
                CurveSpec {
                    profile: CurveProfile::GentleWalk,
                    max_lateral_m: 1.1,
                    ribbon_half_w_m: 0.92,
                    clamp_rect: walk_clamp,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::StoneWalk, &walk_pts, 185);
            let island_poly = warped_plaza_poly(&mut b.rng, clear_x, clear_y, 1.9, 1.4);
            b.add_poly(SurfaceFeatureKind::GardenBedZone, &island_poly, 95);
        }
        ParkTopology::DiagonalGladeWithSpurs => {
            let flip = b.rng.chance(1, 2);
            let y_lo = park_rect.y0 + 1.0;
            let y_hi = park_rect.y1 - 1.0;
            let ty_l = if flip { y_lo } else { y_hi };
            let ty_r = if flip { y_hi } else { y_lo };
            let diag_trail = synthesize_natural_curve_m(
                &mut b.rng,
                (-0.3, ty_l),
                (wf + 0.3, ty_r),
                CurveSpec {
                    profile: CurveProfile::ForestTrail,
                    max_lateral_m: (hf.min(wf) * 0.22).clamp(1.4, 4.2),
                    ribbon_half_w_m: 0.98,
                    clamp_rect: park_rect,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &diag_trail, 195);

            let mid_knot = diag_trail[diag_trail.len() / 2];
            (clear_x, clear_y) = mid_knot;
            let opposite_y = if north_road { hf + 0.3 } else { -0.3 };
            let spur_x = wf * b.rng.f32_range(0.25, 0.75);
            let spur_trail = synthesize_natural_curve_m(
                &mut b.rng,
                mid_knot,
                (spur_x, opposite_y),
                CurveSpec {
                    profile: CurveProfile::ForestTrail,
                    max_lateral_m: 1.8,
                    ribbon_half_w_m: 0.9,
                    clamp_rect: park_rect,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::DirtTrail, &spur_trail, 180);

            let walk_pts = synthesize_natural_curve_m(
                &mut b.rng,
                (clear_x, ry),
                mid_knot,
                CurveSpec {
                    profile: CurveProfile::GentleWalk,
                    max_lateral_m: 1.3,
                    ribbon_half_w_m: 0.9,
                    clamp_rect: walk_clamp,
                },
            );
            b.add_ribbon(SurfaceFeatureKind::StoneWalk, &walk_pts, 180);
            let plaza = warped_plaza_poly(&mut b.rng, clear_x, clear_y, 2.0, 1.6);
            b.add_poly(SurfaceFeatureKind::StoneWalk, &plaza, 105);
        }
    }

    b.place_vehicle_along_ribbon(PropKind::Car, road, 0.26, 0.15, 6);
    place_rot_m(b, PropKind::MediumFountain, clear_x, clear_y, 0, 360, true);
    place_rot_m(
        b,
        PropKind::PicnicTable,
        clear_x + 2.4,
        clear_y + 1.2,
        0,
        90,
        true,
    );
    place_rot_m(
        b,
        PropKind::BbqGrill,
        clear_x + 2.8,
        clear_y - 0.9,
        0,
        360,
        true,
    );
    place_rot_m(
        b,
        PropKind::ParkBench,
        clear_x - 2.4,
        clear_y,
        -25,
        26,
        true,
    );
    place_rot_m(
        b,
        PropKind::TrashCan,
        clear_x - 2.8,
        clear_y + 1.1,
        0,
        360,
        true,
    );
    if b.w >= 26 && b.h >= 20 {
        b.scatter_turf_prop_rot(PropKind::Gazebo, 1, 0, full_rect(b, 3.0));
    }

    let area_scale = (b.w * b.h / 85).clamp(3, 18);
    scatter_organic_beds(
        b,
        (area_scale / 2 + 2).clamp(3, 10),
        full_rect(b, 1.4),
        true,
    );
    b.scatter_trees(PropKind::LargeTree, area_scale + 2, 3.3, full_rect(b, 1.5));
    b.scatter_trees(PropKind::MediumTree, area_scale + 4, 2.4, full_rect(b, 1.3));
    b.scatter_turf_prop_rot(
        PropKind::ParkBench,
        (area_scale / 2).clamp(2, 6),
        30,
        full_rect(b, 2.0),
    );
    b.scatter_turf_prop_rot(PropKind::Boulder, area_scale / 2 + 1, 30, full_rect(b, 1.6));
    b.scatter_turf_prop_rot(PropKind::Bush, area_scale + 4, 30, full_rect(b, 1.2));
}
