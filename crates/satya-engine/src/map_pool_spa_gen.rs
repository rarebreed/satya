//! # Multi-Shape Swimming Pool & Jacuzzi/Spa Generator (`map_pool_spa_gen`)
//!
//! Canonical generator for randomized swimming pool silhouettes ([`PoolShape::RoundedRect`],
//! [`PoolShape::Oval`], [`PoolShape::LShaped`], [`PoolShape::Kidney`]) and 4-style Jacuzzi /
//! spa sanctuaries ([`SpaStyle`]) shared across attached home patios, estate courtyards,
//! suburban/rural/brownstone yards, and apartment courtyard villas.

use std::f32::consts::TAU;

use satya_schema::{
    EconomicTier, MapPointCm, MapPropPlacement, MapSurfaceFeature, PropKind, SurfaceFeatureKind,
    SurfaceShapeKind,
};

use crate::map_dressing::{make_prop_at_cm, prop_spec};
use crate::map_gen_residential::LotBox;
use crate::map_interior_gen::{Furnisher, Rng as InteriorRng};
use crate::map_outdoor_gen::{OutdoorBuilder, RectM, Rng as OutdoorRng};
use crate::map_surface_geom::vehicle_forward_rot_deg;
use crate::map_surface_queries::{
    point_near_protected_surface_m, surface_contains_point_m, topmost_surface_at_m,
};
use crate::tactical_map_geom::{TOKEN_RADIUS_M, circle_collides_obstacles, prop_crosses_wall_edge};

/// Canonical swimming pool planform shapes emitted as smooth vector `PoolWater` basins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolShape {
    RoundedRect,
    Oval,
    LShaped,
    Kidney,
}

impl PoolShape {
    pub const ALL: [Self; 4] = [Self::RoundedRect, Self::Oval, Self::LShaped, Self::Kidney];
}

/// Visual/architectural archetype encoded into the bottom 2 bits of `HotTub::variant_seed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaStyle {
    CedarDeckTub,
    OctagonalTravertineSpa,
    SunkenMosaicGrotto,
    ModernArchitecturalSpa,
}

impl SpaStyle {
    pub const ALL: [Self; 4] = [
        Self::CedarDeckTub,
        Self::OctagonalTravertineSpa,
        Self::SunkenMosaicGrotto,
        Self::ModernArchitecturalSpa,
    ];

    #[must_use]
    pub const fn style_index(self) -> u32 {
        match self {
            Self::CedarDeckTub => 0,
            Self::OctagonalTravertineSpa => 1,
            Self::SunkenMosaicGrotto => 2,
            Self::ModernArchitecturalSpa => 3,
        }
    }

    #[must_use]
    pub const fn from_variant_seed(seed: u32) -> Self {
        match seed & 3 {
            0 => Self::CedarDeckTub,
            1 => Self::OctagonalTravertineSpa,
            2 => Self::SunkenMosaicGrotto,
            _ => Self::ModernArchitecturalSpa,
        }
    }
}

/// Encodes `style` into `variant_seed % 4` so 2D/3D renderers select matching Jacuzzi geometry.
#[must_use]
pub const fn spa_variant_seed(raw_seed: u32, style: SpaStyle) -> u32 {
    (raw_seed & !3) | style.style_index()
}

/// Generated `PoolWater` vector feature plus optional dry pocket inside L-notches or kidney waists.
#[derive(Debug, Clone)]
pub struct GeneratedPoolBasin {
    pub shape: PoolShape,
    pub pool_surface: MapSurfaceFeature,
    pub dry_pocket_m: Option<(f32, f32)>,
}

/// Selects a pool shape suited to `(w_m, h_m)` bounds using `rng_val`.
#[must_use]
pub fn pick_pool_shape(rng_val: u64, w_m: f32, h_m: f32) -> PoolShape {
    let long = w_m.max(h_m);
    let short = w_m.min(h_m);
    let mut eligible = [PoolShape::RoundedRect; 4];
    let mut len = 1usize;
    if long >= 3.0 && short >= 2.2 {
        eligible[len] = PoolShape::Oval;
        len += 1;
    }
    if long >= 3.4 && short >= 2.6 {
        eligible[len] = PoolShape::Kidney;
        len += 1;
    }
    if w_m >= 3.6 && h_m >= 3.0 {
        eligible[len] = PoolShape::LShaped;
        len += 1;
    }
    eligible[(rng_val as usize) % len]
}

fn rotate_and_fit_to_bounds(
    pts: &mut [(f32, f32)],
    pocket: &mut Option<(f32, f32)>,
    bounds_m: (f32, f32, f32, f32),
    tilt_rad: f32,
) {
    let (x0, y0, x1, y1) = bounds_m;
    let hw_avail = ((x1 - x0) * 0.5).max(0.5);
    let hh_avail = ((y1 - y0) * 0.5).max(0.5);
    let (cx, cy) = ((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let (sin_t, cos_t) = tilt_rad.sin_cos();

    let mut max_ax = 1e-4_f32;
    let mut max_ay = 1e-4_f32;
    for p in pts.iter_mut() {
        let rx = p.0 * cos_t - p.1 * sin_t;
        let ry = p.0 * sin_t + p.1 * cos_t;
        *p = (rx, ry);
        max_ax = max_ax.max(rx.abs());
        max_ay = max_ay.max(ry.abs());
    }
    if let Some(pk) = pocket.as_mut() {
        *pk = (pk.0 * cos_t - pk.1 * sin_t, pk.0 * sin_t + pk.1 * cos_t);
    }

    let fit = (hw_avail / max_ax).min(hh_avail / max_ay).min(1.0);
    for p in pts.iter_mut() {
        p.0 = (cx + p.0 * fit).clamp(x0, x1);
        p.1 = (cy + p.1 * fit).clamp(y0, y1);
    }
    if let Some(pk) = pocket.as_mut() {
        pk.0 = (cx + pk.0 * fit).clamp(x0, x1);
        pk.1 = (cy + pk.1 * fit).clamp(y0, y1);
    }
}

/// Builds a randomized `PoolWater` [`MapSurfaceFeature`] strictly contained inside `bounds_m`.
#[must_use]
pub fn build_pool_surface(
    shape: PoolShape,
    bounds_m: (f32, f32, f32, f32),
    max_tilt_deg: f32,
    seed: u64,
) -> GeneratedPoolBasin {
    let mut rng = OutdoorRng(seed ^ 0x5001_BA51);
    let (x0, y0, x1, y1) = bounds_m;
    let bw = (x1 - x0).max(1.6);
    let bh = (y1 - y0).max(1.4);
    let tilt_deg = if max_tilt_deg > 0.5 {
        rng.f32_range(-max_tilt_deg, max_tilt_deg)
    } else {
        0.0
    };
    let tilt_rad = tilt_deg.to_radians();
    let variant_seed = (rng.next() & 0xffff_ffff) as u32;

    let (surf_shape, border_cm, mut pts, mut pocket) = match shape {
        PoolShape::RoundedRect => {
            let hw = bw * 0.5 * rng.f32_range(0.84, 0.96);
            let hh = bh * 0.5 * rng.f32_range(0.82, 0.96);
            let corner_cm = rng.range(38, 87);
            (
                SurfaceShapeKind::Polygon,
                corner_cm,
                vec![(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)],
                None,
            )
        }
        PoolShape::Oval => {
            let rx = bw * 0.5 * rng.f32_range(0.86, 0.96);
            let ry = bh * 0.5 * rng.f32_range(0.84, 0.96);
            let p = rng.f32_range(1.75, 2.20);
            let exp = 2.0 / p;
            let steps = 14usize;
            let ring = (0..steps)
                .map(|i| {
                    let t = (i as f32 / steps as f32) * TAU;
                    let (s, c) = t.sin_cos();
                    (
                        c.signum() * c.abs().powf(exp) * rx,
                        s.signum() * s.abs().powf(exp) * ry,
                    )
                })
                .collect();
            (SurfaceShapeKind::OrganicClosed, 0, ring, None)
        }
        PoolShape::LShaped => {
            let hw = bw * 0.48;
            let hh = bh * 0.48;
            let arm_w = (bw * rng.f32_range(0.44, 0.58))
                .max(1.45)
                .min((bw - 1.25).max(1.2));
            let arm_h = (bh * rng.f32_range(0.44, 0.58))
                .max(1.45)
                .min((bh - 1.25).max(1.2));
            let corner_cm = rng.range(42, 69);
            let quad = rng.range(0, 4);
            let (verts, pk) = match quad {
                // NE dry notch (+x, -y)
                0 => {
                    let xc = -hw + arm_w;
                    let yc = hh - arm_h;
                    (
                        vec![
                            (-hw, -hh),
                            (-hw, hh),
                            (hw, hh),
                            (hw, yc),
                            (xc, yc),
                            (xc, -hh),
                        ],
                        ((xc + hw) * 0.5, (-hh + yc) * 0.5),
                    )
                }
                // NW dry notch (-x, -y)
                1 => {
                    let xc = hw - arm_w;
                    let yc = hh - arm_h;
                    (
                        vec![
                            (xc, -hh),
                            (xc, yc),
                            (-hw, yc),
                            (-hw, hh),
                            (hw, hh),
                            (hw, -hh),
                        ],
                        ((-hw + xc) * 0.5, (-hh + yc) * 0.5),
                    )
                }
                // SE dry notch (+x, +y)
                2 => {
                    let xc = -hw + arm_w;
                    let yc = -hh + arm_h;
                    (
                        vec![
                            (-hw, -hh),
                            (-hw, hh),
                            (xc, hh),
                            (xc, yc),
                            (hw, yc),
                            (hw, -hh),
                        ],
                        ((xc + hw) * 0.5, (yc + hh) * 0.5),
                    )
                }
                // SW dry notch (-x, +y)
                _ => {
                    let xc = hw - arm_w;
                    let yc = -hh + arm_h;
                    (
                        vec![
                            (-hw, -hh),
                            (-hw, yc),
                            (xc, yc),
                            (xc, hh),
                            (hw, hh),
                            (hw, -hh),
                        ],
                        ((-hw + xc) * 0.5, (yc + hh) * 0.5),
                    )
                }
            };
            (SurfaceShapeKind::Polygon, corner_cm, verts, Some(pk))
        }
        PoolShape::Kidney => {
            let horiz = bw >= bh;
            let indent_sign = if rng.chance(1, 2) { 1.0_f32 } else { -1.0_f32 };
            let deep_sign = if rng.chance(1, 2) { 1.0_f32 } else { -1.0_f32 };
            let steps = 14usize;
            let mut raw = Vec::with_capacity(steps);
            for i in 0..steps {
                let t = (i as f32 / steps as f32) * TAU;
                let (v, u) = t.sin_cos();
                let spine_shift = -indent_sign * 0.28 * (1.0 - u * u);
                let lobe_asym = 1.0 + 0.14 * u * deep_sign;
                let waist_pinch = 1.0 - 0.26 * (1.0 - u * u).max(0.0) * (v * indent_sign).max(0.0);
                let half_thick = 0.74 * lobe_asym * waist_pinch;
                let mx = u;
                let my = spine_shift + v * half_thick;
                raw.push(if horiz { (mx, my) } else { (my, mx) });
            }
            let (mut min_x, mut max_x, mut min_y, mut max_y) = (
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
            );
            for &(px, py) in &raw {
                min_x = min_x.min(px);
                max_x = max_x.max(px);
                min_y = min_y.min(py);
                max_y = max_y.max(py);
            }
            let mid_x = 0.5 * (min_x + max_x);
            let mid_y = 0.5 * (min_y + max_y);
            let sx = (bw * 0.90) / (max_x - min_x).max(1e-3);
            let sy = (bh * 0.90) / (max_y - min_y).max(1e-3);
            for p in &mut raw {
                p.0 = (p.0 - mid_x) * sx;
                p.1 = (p.1 - mid_y) * sy;
            }
            let waist_offset = indent_sign * 0.68;
            let pk = if horiz {
                (0.0, (waist_offset - mid_y) * sy)
            } else {
                ((waist_offset - mid_x) * sx, 0.0)
            };
            (SurfaceShapeKind::OrganicClosed, 0, raw, Some(pk))
        }
    };

    rotate_and_fit_to_bounds(&mut pts, &mut pocket, bounds_m, tilt_rad);
    let pool_surface = MapSurfaceFeature {
        kind: SurfaceFeatureKind::PoolWater,
        shape: surf_shape,
        points_cm: pts
            .into_iter()
            .map(|(x, y)| MapPointCm::new((x * 100.0).round() as i32, (y * 100.0).round() as i32))
            .collect(),
        width_cm: 0,
        border_style_cm: border_cm,
        variant_seed,
        tint_rgb: None,
    };

    GeneratedPoolBasin {
        shape,
        pool_surface,
        dry_pocket_m: pocket,
    }
}

fn rect_poly(x0: f32, y0: f32, x1: f32, y1: f32) -> [(f32, f32); 4] {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn prop_footprints_avoid_surface(prop: &MapPropPlacement, surf: &MapSurfaceFeature) -> bool {
    let fp = prop.footprint_m();
    let hx = fp.span_w_m * 0.48;
    let hy = fp.span_h_m * 0.48;
    for &(ox, oy) in &[
        (0.0, 0.0),
        (-hx, -hy),
        (hx, -hy),
        (-hx, hy),
        (hx, hy),
        (0.0, -hy),
        (0.0, hy),
    ] {
        let (wx, wy) = fp.local_offset_to_world(ox, oy);
        if surface_contains_point_m(surf, wx, wy) {
            return false;
        }
    }
    true
}

fn try_place_patio_cm(
    furn: &mut Furnisher,
    patio: LotBox,
    pool: Option<&MapSurfaceFeature>,
    kind: PropKind,
    cx_m: f32,
    cy_m: f32,
    rot_deg: i32,
) -> Option<MapPropPlacement> {
    let seed = furn.rng.next() as u32;
    let cand = make_prop_at_cm(
        kind,
        (cx_m * 100.0).round() as i32,
        (cy_m * 100.0).round() as i32,
        rot_deg,
        None,
        seed,
    );
    let fp = cand.footprint_m();
    let (hw, hh) = crate::map_dressing::rotated_aabb_half_extents_cm(
        (fp.span_w_m * 100.0) as i32,
        (fp.span_h_m * 100.0) as i32,
        cand.rotation_deg,
    );
    if fp.cx_m - hw / 100.0 < patio.x0 as f32 + 0.05
        || fp.cy_m - hh / 100.0 < patio.y0 as f32 + 0.05
        || fp.cx_m + hw / 100.0 > patio.x1 as f32 - 0.05
        || fp.cy_m + hh / 100.0 > patio.y1 as f32 - 0.05
    {
        return None;
    }
    if let Some(p_surf) = pool
        && !prop_footprints_avoid_surface(&cand, p_surf)
    {
        return None;
    }
    let decal = prop_spec(kind).floor_layer;
    if !decal {
        for yy in cand.y..(cand.y + cand.h_cells) {
            for xx in cand.x..(cand.x + cand.w_cells) {
                if xx < patio.x0
                    || yy < patio.y0
                    || xx >= patio.x1
                    || yy >= patio.y1
                    || furn.blocked[(yy * furn.w + xx) as usize]
                {
                    return None;
                }
            }
        }
        for yy in cand.y..(cand.y + cand.h_cells) {
            for xx in cand.x..(cand.x + cand.w_cells) {
                if cand.covers(xx, yy) {
                    furn.reserve_cell(xx, yy);
                }
            }
        }
    }
    furn.props.push(cand);
    Some(cand)
}

/// Populates an attached home `patio` with a randomized multi-shape `PoolWater` basin and/or a
/// styled Jacuzzi/Spa (`HotTub`, optional `Gazebo`, `ReclinerChair` loungers, `BbqGrill`, etc.).
pub(crate) fn populate_patio_pool_or_spa(
    furn: &mut Furnisher,
    patio: LotBox,
    tier: EconomicTier,
    want_pool: bool,
    want_spa: bool,
    surfaces: &mut Vec<MapSurfaceFeature>,
    rng: &mut InteriorRng,
) {
    let use_stone = (want_pool && matches!(tier, EconomicTier::LuxuryMansion))
        || (!want_pool && want_spa && rng.chance(1, 2));
    let patio_kind = if use_stone {
        SurfaceFeatureKind::StoneWalk
    } else {
        SurfaceFeatureKind::WoodDeck
    };
    surfaces.push(patio.polygon_feature(patio_kind, 22, rng.next() as u32));
    let pr = patio.as_room();

    if want_pool && patio.w() >= 5 && patio.h() >= 4 {
        let inset_x = if patio.w() >= 6 { 0.85 } else { 0.72 };
        let inset_y = if patio.h() >= 5 { 0.85 } else { 0.70 };
        let bounds = (
            patio.x0 as f32 + inset_x,
            patio.y0 as f32 + inset_y,
            patio.x1 as f32 - inset_x,
            patio.y1 as f32 - inset_y,
        );
        let max_tilt = if patio.w() >= 7 && patio.h() >= 6 {
            6.0
        } else {
            0.0
        };
        let shape = pick_pool_shape(rng.next(), bounds.2 - bounds.0, bounds.3 - bounds.1);
        let basin = build_pool_surface(shape, bounds, max_tilt, rng.next());
        let pool_cx = 0.5 * (bounds.0 + bounds.2);
        let pool_cy = 0.5 * (bounds.1 + bounds.3);

        for yy in patio.y0..patio.y1 {
            for xx in patio.x0..patio.x1 {
                let cx = xx as f32 + 0.5;
                let cy = yy as f32 + 0.5;
                let hits_pool = [
                    (0.0, 0.0),
                    (-0.35, -0.35),
                    (0.35, -0.35),
                    (-0.35, 0.35),
                    (0.35, 0.35),
                ]
                .iter()
                .any(|&(ox, oy)| surface_contains_point_m(&basin.pool_surface, cx + ox, cy + oy));
                if hits_pool {
                    furn.reserve_cell(xx, yy);
                }
            }
        }

        if want_spa || (matches!(tier, EconomicTier::LuxuryMansion) && rng.chance(2, 5)) {
            let style = SpaStyle::ALL[(rng.next() as usize) % SpaStyle::ALL.len()];
            let seed = spa_variant_seed(rng.next() as u32, style);
            let mut placed_spa = false;
            if let Some((px, py)) = basin.dry_pocket_m {
                let gx = (px - 1.0).round() as i32;
                let gy = (py - 1.0).round() as i32;
                if let Some(p) = try_place_patio_cm(
                    furn,
                    patio,
                    Some(&basin.pool_surface),
                    PropKind::HotTub,
                    gx as f32 + 1.0,
                    gy as f32 + 1.0,
                    0,
                ) {
                    if let Some(last) = furn.props.last_mut()
                        && last.kind == p.kind
                    {
                        last.variant_seed = seed;
                    }
                    placed_spa = true;
                }
            }
            if !placed_spa {
                'spa_scan: for yy in patio.y0..=(patio.y1 - 2) {
                    for xx in patio.x0..=(patio.x1 - 2) {
                        if try_place_patio_cm(
                            furn,
                            patio,
                            Some(&basin.pool_surface),
                            PropKind::HotTub,
                            xx as f32 + 1.0,
                            yy as f32 + 1.0,
                            0,
                        )
                        .is_some()
                        {
                            if let Some(last) = furn.props.last_mut() {
                                last.variant_seed = seed;
                            }
                            break 'spa_scan;
                        }
                    }
                }
            }
        }

        let mut loungers = 0usize;
        for yy in patio.y0..patio.y1 {
            for xx in patio.x0..patio.x1 {
                if loungers >= 2 {
                    break;
                }
                if (xx + yy) % 2 != 0 {
                    continue;
                }
                let cx = xx as f32 + 0.5;
                let cy = yy as f32 + 0.5;
                let rot = vehicle_forward_rot_deg(pool_cx - cx, pool_cy - cy);
                if try_place_patio_cm(
                    furn,
                    patio,
                    Some(&basin.pool_surface),
                    PropKind::ReclinerChair,
                    cx,
                    cy,
                    rot,
                )
                .is_some()
                {
                    loungers += 1;
                }
            }
        }
        surfaces.push(basin.pool_surface);
    } else if want_spa {
        let style = SpaStyle::ALL[(rng.next() as usize) % SpaStyle::ALL.len()];
        let seed = spa_variant_seed(rng.next() as u32, style);
        let hx = patio.x0 + (patio.w() - 2).max(0) / 2;
        let hy = patio.y0 + (patio.h() - 2).max(0) / 2;
        let spa_cx = hx as f32 + 1.0;
        let spa_cy = hy as f32 + 1.0;
        let mut placed = false;
        if patio.w() >= 5 && patio.h() >= 5 && rng.chance(2, 5) {
            let gx = (hx - 1).clamp(patio.x0, patio.x1 - 4);
            let gy = (hy - 1).clamp(patio.y0, patio.y1 - 4);
            if let Some(ht) = furn.try_place(PropKind::HotTub, gx + 1, gy + 1, 0, None, pr) {
                if let Some(last) = furn.props.last_mut()
                    && last.kind == ht.kind
                {
                    last.variant_seed = seed;
                }
                let _ = furn.try_place(PropKind::Gazebo, gx, gy, 0, None, pr);
                placed = true;
            }
        }
        if !placed
            && let Some(ht) = furn.try_place(PropKind::HotTub, hx, hy, 0, None, pr)
            && let Some(last) = furn.props.last_mut()
            && last.kind == ht.kind
        {
            last.variant_seed = seed;
        }
        for &(lx, ly) in &[
            (patio.x0 as f32 + 0.5, patio.y0 as f32 + 0.5),
            (patio.x1 as f32 - 0.5, patio.y0 as f32 + 0.5),
        ] {
            let rot = vehicle_forward_rot_deg(spa_cx - lx, spa_cy - ly);
            let _ = try_place_patio_cm(furn, patio, None, PropKind::ReclinerChair, lx, ly, rot);
        }
        let _ = furn.against_any_wall(PropKind::StandingLamp, pr, None);
    }

    let _ = furn.against_any_wall(PropKind::BbqGrill, pr, None);
    let _ = furn
        .against_any_wall(PropKind::RoundTable, pr, None)
        .or_else(|| furn.against_any_wall(PropKind::PicnicTable, pr, None));
    let _ = furn.against_any_wall(PropKind::Planter, pr, None);
    let _ = furn.against_any_wall(PropKind::AcUnit, pr, None);
    let _ = furn.against_any_wall(PropKind::TrashCan, pr, None);
}

/// Configuration for outdoor yard/courtyard pool & Jacuzzi/Spa vignette placement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OutdoorPoolSpaOpts {
    pub(crate) allow_pool: bool,
    pub(crate) allow_spa: bool,
    pub(crate) prefer_spa_only: bool,
}

fn area_clear_for_terrace(b: &OutdoorBuilder, rect: RectM) -> bool {
    if rect.x0 < 1.2 || rect.y0 < 1.2 || rect.x1 > b.w as f32 - 1.2 || rect.y1 > b.h as f32 - 1.2 {
        return false;
    }
    let steps_x = ((rect.x1 - rect.x0) / 0.70).ceil().max(2.0) as i32;
    let steps_y = ((rect.y1 - rect.y0) / 0.70).ceil().max(2.0) as i32;
    for iy in 0..=steps_y {
        let py = rect.y0 + (rect.y1 - rect.y0) * (iy as f32 / steps_y as f32);
        for ix in 0..=steps_x {
            let px = rect.x0 + (rect.x1 - rect.x0) * (ix as f32 / steps_x as f32);
            if point_near_protected_surface_m(&b.surfaces, px, py, 0.55) {
                return false;
            }
            if b.surfaces.iter().any(|s| {
                (s.kind.is_built_floor_or_deck()
                    || s.kind.is_road_or_pavement()
                    || s.kind.is_water_or_channel())
                    && surface_contains_point_m(s, px, py)
            }) {
                return false;
            }
            if circle_collides_obstacles(
                b.w as f32,
                b.h as f32,
                &b.edges,
                &b.props,
                px,
                py,
                TOKEN_RADIUS_M + 0.15,
            ) {
                return false;
            }
            if b.spawn_pads_m
                .iter()
                .any(|&(sx, sy)| (sx - px).hypot(sy - py) < 1.35)
            {
                return false;
            }
        }
    }
    true
}

fn find_clear_terrace_rect(
    b: &mut OutdoorBuilder,
    cand_rect: RectM,
    target_w: f32,
    target_h: f32,
) -> Option<RectM> {
    if cand_rect.x1 - cand_rect.x0 < target_w || cand_rect.y1 - cand_rect.y0 < target_h {
        return None;
    }
    let x_hi = cand_rect.x1 - target_w;
    let y_hi = cand_rect.y1 - target_h;
    let x_mid = 0.5 * (cand_rect.x0 + x_hi);
    let y_mid = 0.5 * (cand_rect.y0 + y_hi);
    let mut anchors = vec![
        (x_mid, y_mid),
        (cand_rect.x0, y_mid),
        (x_hi, y_mid),
        (x_mid, cand_rect.y0),
        (x_mid, y_hi),
    ];
    for _ in 0..10 {
        anchors.push((
            b.rng.f32_range(cand_rect.x0, x_hi),
            b.rng.f32_range(cand_rect.y0, y_hi),
        ));
    }
    for (ax, ay) in anchors {
        let r = RectM {
            x0: ax,
            y0: ay,
            x1: ax + target_w,
            y1: ay + target_h,
        };
        if area_clear_for_terrace(b, r) {
            return Some(r);
        }
    }
    None
}

fn place_terrace_prop(
    b: &mut OutdoorBuilder,
    pool: Option<&MapSurfaceFeature>,
    kind: PropKind,
    cx_m: f32,
    cy_m: f32,
    rot_deg: i32,
    variant_override: Option<u32>,
) -> bool {
    let seed = variant_override.unwrap_or_else(|| (b.rng.next() & 0xffff_ffff) as u32);
    let cand = make_prop_at_cm(
        kind,
        OutdoorBuilder::cm(cx_m),
        OutdoorBuilder::cm(cy_m),
        rot_deg,
        None,
        seed,
    );
    if !prop_spec(kind).floor_layer && prop_crosses_wall_edge(&cand, &b.edges) {
        return false;
    }
    if let Some(ps) = pool
        && !prop_footprints_avoid_surface(&cand, ps)
    {
        return false;
    }
    let ok = b.try_place_cm(
        kind,
        OutdoorBuilder::cm(cx_m),
        OutdoorBuilder::cm(cy_m),
        rot_deg,
        None,
        true,
    );
    if ok
        && let Some(ov) = variant_override
        && let Some(last) = b.props.last_mut()
    {
        last.variant_seed = ov;
    }
    ok
}

fn try_place_outdoor_pool_terrace(
    b: &mut OutdoorBuilder,
    cand_rect: RectM,
    allow_spa: bool,
) -> bool {
    let avail_w = cand_rect.x1 - cand_rect.x0;
    let avail_h = cand_rect.y1 - cand_rect.y0;
    let tw = b.rng.f32_range(6.0, 9.6).min(avail_w);
    let th = b.rng.f32_range(4.8, 7.2).min(avail_h);
    if tw < 5.2 || th < 4.2 {
        return false;
    }
    let Some(deck) = find_clear_terrace_rect(b, cand_rect, tw, th) else {
        return false;
    };

    let deck_kind = match b.rng.range(0, 3) {
        0 => SurfaceFeatureKind::WoodDeck,
        1 => SurfaceFeatureKind::BrickPaverWalk,
        _ => SurfaceFeatureKind::StoneWalk,
    };
    let corner_cm = b.rng.range(60, 121);
    b.add_poly(
        deck_kind,
        &rect_poly(deck.x0, deck.y0, deck.x1, deck.y1),
        corner_cm,
    );

    let margin = 1.05_f32;
    let pool_bounds = (
        deck.x0 + margin,
        deck.y0 + margin,
        deck.x1 - margin,
        deck.y1 - margin,
    );
    let pw = pool_bounds.2 - pool_bounds.0;
    let ph = pool_bounds.3 - pool_bounds.1;
    let shape = pick_pool_shape(b.rng.next(), pw, ph);
    let max_tilt = if pw >= 4.4 && ph >= 3.2 { 14.0 } else { 0.0 };
    let basin = build_pool_surface(shape, pool_bounds, max_tilt, b.rng.next());
    let pool_cx = 0.5 * (pool_bounds.0 + pool_bounds.2);
    let pool_cy = 0.5 * (pool_bounds.1 + pool_bounds.3);

    // Optional spillover Jacuzzi/Spa in dry pocket or dry corner of deck.
    if allow_spa && b.rng.chance(3, 5) {
        let style = SpaStyle::ALL[(b.rng.next() as usize) % SpaStyle::ALL.len()];
        let seed = spa_variant_seed(b.rng.next() as u32, style);
        let mut spa_cands = Vec::new();
        if let Some(pk) = basin.dry_pocket_m {
            spa_cands.push(pk);
        }
        spa_cands.extend_from_slice(&[
            (deck.x0 + 1.15, deck.y0 + 1.15),
            (deck.x1 - 1.15, deck.y0 + 1.15),
            (deck.x0 + 1.15, deck.y1 - 1.15),
            (deck.x1 - 1.15, deck.y1 - 1.15),
        ]);
        for (sx, sy) in spa_cands {
            let spa_rot = b.rng.range(-12, 13);
            if place_terrace_prop(
                b,
                Some(&basin.pool_surface),
                PropKind::HotTub,
                sx,
                sy,
                spa_rot,
                Some(seed),
            ) {
                break;
            }
        }
    }

    // Place poolside loungers facing the water & patio dining / grill props on the dry deck rim.
    let lounger_cands = [
        (deck.x0 + 0.65, pool_cy - 0.8),
        (deck.x0 + 0.65, pool_cy + 0.8),
        (deck.x1 - 0.65, pool_cy - 0.8),
        (pool_cx - 0.9, deck.y0 + 0.62),
        (pool_cx + 0.9, deck.y1 - 0.62),
    ];
    let mut loungers = 0;
    for &(lx, ly) in &lounger_cands {
        if loungers >= 3 {
            break;
        }
        let rot = vehicle_forward_rot_deg(pool_cx - lx, pool_cy - ly);
        if place_terrace_prop(
            b,
            Some(&basin.pool_surface),
            PropKind::ReclinerChair,
            lx,
            ly,
            rot,
            None,
        ) {
            loungers += 1;
        }
    }
    for &(tx, ty, kind) in &[
        (deck.x1 - 0.72, deck.y0 + 0.72, PropKind::RoundTable),
        (deck.x0 + 0.65, deck.y1 - 0.65, PropKind::BbqGrill),
        (deck.x1 - 0.55, deck.y1 - 0.55, PropKind::Planter),
        (deck.x0 + 0.55, deck.y0 + 0.55, PropKind::Planter),
    ] {
        let _ = place_terrace_prop(b, Some(&basin.pool_surface), kind, tx, ty, 0, None);
    }

    b.surfaces.push(basin.pool_surface);
    true
}

fn try_place_outdoor_spa_vignette(b: &mut OutdoorBuilder, cand_rect: RectM) -> bool {
    let avail_w = cand_rect.x1 - cand_rect.x0;
    let avail_h = cand_rect.y1 - cand_rect.y0;
    let pad_s = b.rng.f32_range(3.8, 5.2).min(avail_w).min(avail_h);
    if pad_s < 3.6 {
        return false;
    }
    let Some(pad) = find_clear_terrace_rect(b, cand_rect, pad_s, pad_s) else {
        return false;
    };

    let pad_kind = match b.rng.range(0, 3) {
        0 => SurfaceFeatureKind::WoodDeck,
        1 => SurfaceFeatureKind::BrickPaverWalk,
        _ => SurfaceFeatureKind::StoneWalk,
    };
    b.add_poly(
        pad_kind,
        &rect_poly(pad.x0, pad.y0, pad.x1, pad.y1),
        (pad_s * 45.0).round() as i32,
    );

    let cx = 0.5 * (pad.x0 + pad.x1);
    let cy = 0.5 * (pad.y0 + pad.y1);
    let style = SpaStyle::ALL[(b.rng.next() as usize) % SpaStyle::ALL.len()];
    let seed = spa_variant_seed(b.rng.next() as u32, style);
    let rot = b.rng.range(0, 4) * 45;

    let mut grotto_pool: Option<MapSurfaceFeature> = None;
    if style == SpaStyle::SunkenMosaicGrotto
        && pad_kind == SurfaceFeatureKind::StoneWalk
        && pad_s >= 4.8
        && b.rng.chance(1, 2)
    {
        let basin = build_pool_surface(
            PoolShape::Oval,
            (cx - 1.15, cy - 1.05, cx + 1.15, cy + 1.05),
            12.0,
            b.rng.next(),
        );
        let pool_ref = basin.pool_surface.clone();
        b.surfaces.push(basin.pool_surface);
        let _ = place_terrace_prop(
            b,
            Some(&pool_ref),
            PropKind::Boulder,
            pad.x0 + 0.85,
            pad.y0 + 0.85,
            25,
            None,
        );
        grotto_pool = Some(pool_ref);
    }

    if grotto_pool.is_none()
        && place_terrace_prop(b, None, PropKind::HotTub, cx, cy, rot, Some(seed))
        && pad_s >= 4.6
        && b.rng.chance(1, 2)
    {
        let _ = b.try_place_cm(
            PropKind::Gazebo,
            OutdoorBuilder::cm(cx),
            OutdoorBuilder::cm(cy),
            0,
            None,
            true,
        );
    }

    for &(lx, ly) in &[(pad.x0 + 0.62, cy), (pad.x1 - 0.62, cy)] {
        let lrot = vehicle_forward_rot_deg(cx - lx, cy - ly);
        let _ = place_terrace_prop(
            b,
            grotto_pool.as_ref(),
            PropKind::ReclinerChair,
            lx,
            ly,
            lrot,
            None,
        );
    }
    let _ = place_terrace_prop(
        b,
        grotto_pool.as_ref(),
        PropKind::RoundTable,
        cx,
        pad.y1 - 0.65,
        0,
        None,
    );
    let _ = place_terrace_prop(
        b,
        grotto_pool.as_ref(),
        PropKind::Planter,
        pad.x0 + 0.55,
        pad.y0 + 0.55,
        0,
        None,
    );
    let _ = place_terrace_prop(
        b,
        grotto_pool.as_ref(),
        PropKind::Planter,
        pad.x1 - 0.55,
        pad.y0 + 0.55,
        0,
        None,
    );

    // Privacy hedges strictly outside the paved pad (allow_on_pavement = false).
    for &(hx, hy, hrot) in &[(cx, pad.y0 - 0.72, 0), (pad.x0 - 0.72, cy, 90)] {
        let _ = b.try_place_cm(
            PropKind::Hedge,
            OutdoorBuilder::cm(hx),
            OutdoorBuilder::cm(hy),
            hrot,
            Some(220),
            false,
        );
    }
    true
}

/// Attempts to place either a multi-shape swimming pool terrace or a Jacuzzi/Spa court inside `cand_rect`.
pub(crate) fn try_place_outdoor_pool_or_spa(
    b: &mut OutdoorBuilder,
    cand_rect: RectM,
    opts: OutdoorPoolSpaOpts,
) -> bool {
    if opts.allow_pool
        && !opts.prefer_spa_only
        && try_place_outdoor_pool_terrace(b, cand_rect, opts.allow_spa)
    {
        return true;
    }
    if opts.allow_spa && try_place_outdoor_spa_vignette(b, cand_rect) {
        return true;
    }
    if opts.allow_pool {
        return try_place_outdoor_pool_terrace(b, cand_rect, opts.allow_spa);
    }
    false
}

/// Check whether `point` sits on top of any `PoolWater` feature in `surfaces`.
#[must_use]
pub fn point_in_any_pool(surfaces: &[MapSurfaceFeature], x_m: f32, y_m: f32) -> bool {
    topmost_surface_at_m(surfaces, x_m, y_m)
        .is_some_and(|s| s.kind == SurfaceFeatureKind::PoolWater)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_surface_geom::{
        sampled_closed_ring_m, signed_area_2d, triangulate_simple_polygon_m,
    };

    #[test]
    fn all_pool_shapes_produce_valid_simple_meshes_and_dry_pockets() {
        let bounds = (2.0, 2.0, 9.0, 7.0);
        for shape in PoolShape::ALL {
            for seed in 1_u64..=12 {
                let basin = build_pool_surface(shape, bounds, 12.0, seed);
                assert_eq!(basin.pool_surface.kind, SurfaceFeatureKind::PoolWater);
                let ring = sampled_closed_ring_m(&basin.pool_surface);
                assert!(ring.len() >= 4, "{shape:?} ring too short");
                for &(x, y) in &ring {
                    assert!(
                        x >= bounds.0 - 0.05
                            && x <= bounds.2 + 0.05
                            && y >= bounds.1 - 0.05
                            && y <= bounds.3 + 0.05,
                        "{shape:?} vertex ({x}, {y}) escaped bounds {bounds:?}"
                    );
                }
                let area = signed_area_2d(&ring).abs();
                assert!(area > 4.0, "{shape:?} seed {seed} tiny area {area}");
                let (verts, tris) = triangulate_simple_polygon_m(&ring);
                assert!(!tris.is_empty(), "{shape:?} triangulation empty");
                assert!(verts.iter().all(|(x, y)| x.is_finite() && y.is_finite()));

                match shape {
                    PoolShape::LShaped => {
                        assert_eq!(basin.pool_surface.shape, SurfaceShapeKind::Polygon);
                        assert_eq!(basin.pool_surface.points_cm.len(), 6);
                        assert!(basin.pool_surface.border_style_cm > 0);
                        let pk = basin.dry_pocket_m.expect("LShaped has dry pocket");
                        assert!(
                            !surface_contains_point_m(&basin.pool_surface, pk.0, pk.1),
                            "LShaped dry pocket {pk:?} landed inside pool"
                        );
                    }
                    PoolShape::Kidney => {
                        assert_eq!(basin.pool_surface.shape, SurfaceShapeKind::OrganicClosed);
                        let pk = basin.dry_pocket_m.expect("Kidney has dry pocket");
                        assert!(
                            !surface_contains_point_m(&basin.pool_surface, pk.0, pk.1),
                            "Kidney dry pocket {pk:?} landed inside pool"
                        );
                    }
                    PoolShape::Oval => {
                        assert_eq!(basin.pool_surface.shape, SurfaceShapeKind::OrganicClosed);
                    }
                    PoolShape::RoundedRect => {
                        assert_eq!(basin.pool_surface.shape, SurfaceShapeKind::Polygon);
                        assert!(basin.pool_surface.border_style_cm >= 38);
                    }
                }
            }
        }
    }
}
