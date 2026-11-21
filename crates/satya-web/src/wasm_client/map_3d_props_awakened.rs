//! # 3D Diorama Props & Translucent Spatial Overlays for *Awakened* 2035 (`map_3d_props_awakened`)
//!
//! Companion to [`crate::wasm_client::map_3d_dressing_geom`] and [`crate::wasm_client::map_3d_diorama_mesh`]
//! that emits:
//! - Multi-part oriented 3D meshes for all 10 *Awakened* 2035 tech/grid [`PropKind`]s.
//! - Translucent late-pass active-floor [`MapFaradayZone`] copper perimeter frames and
//!   [`MapSurveillanceNode`] compass-accurate FOV sector fans (rendered after `opaque_vcount` using
//!   `TRANSLUCENT_CANOPY_ZONE`).

use crate::wasm_client::glb_model::GlbVertex;
use crate::wasm_client::map_3d::{TacticalMapUiState, metric_to_world_xz};
use crate::wasm_client::map_3d_diorama_mesh::push_quad;
use crate::wasm_client::map_3d_dressing_geom::{lighten, push_obb};
use crate::wasm_client::map_3d_props_lab_urban::apply_prop_tint;
use crate::wasm_client::map_3d_surfaces_geom::push_tri_up;
use satya_maps::map_dressing::prop_spec;
use satya_schema::{MapPropPlacement, PropKind};

/// Translucent spatial overlay sentinel matching `SHARED_GLB_DIORAMA_WGSL` (`alpha * 0.46`).
const TRANSLUCENT_CANOPY_ZONE: f32 = -0.35;

struct AwakenedPropBuilder<'a> {
    verts: &'a mut Vec<GlbVertex>,
    cx: f32,
    cz: f32,
    sn: f32,
    cs: f32,
    rot: i32,
}

impl AwakenedPropBuilder<'_> {
    fn obb(&mut self, local_xz: [f32; 2], y_span: [f32; 2], half_xz: [f32; 2], rgb: [f32; 3]) {
        let [lx, lz] = local_xz;
        let wx = self.cx + lx * self.cs - lz * self.sn;
        let wz = self.cz + lx * self.sn + lz * self.cs;
        push_obb(
            self.verts,
            wx,
            wz,
            y_span[0],
            y_span[1],
            half_xz[0].max(0.02),
            half_xz[1].max(0.02),
            self.rot,
            rgb,
            0.0,
        );
    }
}

/// Emits multi-part opaque 3D geometry for *Awakened* 2035 props. Returns `false` for other kinds.
pub(crate) fn push_awakened_prop_3d(
    verts: &mut Vec<GlbVertex>,
    p: &MapPropPlacement,
    center_xz: [f32; 2],
    half_xz: [f32; 2],
    top: f32,
    _cutaway: bool,
) -> bool {
    let fp = p.footprint_m();
    let [cx, cz] = center_xz;
    let [hx, hz] = half_xz;
    let rgb = apply_prop_tint(prop_spec(p.kind).albedo_rgb, p.tint_rgb);
    let mut b = AwakenedPropBuilder {
        verts,
        cx,
        cz,
        sn: fp.sin_rot,
        cs: fp.cos_rot,
        rot: fp.rot_deg,
    };

    match p.kind {
        PropKind::CryoComputeCluster => {
            b.obb([0.0, 0.0], [0.0, 0.08], [hx * 0.96, hz * 0.92], [0.18, 0.22, 0.27]);
            b.obb([0.0, 0.0], [0.08, top], [hx * 0.90, hz * 0.84], rgb);
            b.obb([0.0, hz * 0.62], [top * 0.45, top * 0.92], [hx * 0.78, hz * 0.14], [0.16, 0.80, 0.94]);
            b.obb([-hx * 0.45, -hz * 0.35], [top, top + 0.09], [hx * 0.26, hz * 0.26], [0.76, 0.45, 0.20]);
            b.obb([hx * 0.45, -hz * 0.35], [top, top + 0.09], [hx * 0.26, hz * 0.26], [0.76, 0.45, 0.20]);
        }
        PropKind::BioServerVat => {
            b.obb([0.0, 0.0], [0.0, 0.18], [hx * 0.88, hz * 0.88], [0.22, 0.26, 0.31]);
            b.obb([0.0, 0.0], [0.18, top * 0.88], [hx * 0.74, hz * 0.74], rgb);
            b.obb([0.0, 0.0], [top * 0.88, top], [hx * 0.82, hz * 0.82], [0.68, 0.44, 0.20]);
            b.obb([0.0, 0.0], [top * 0.35, top * 0.72], [hx * 0.34, hz * 0.34], [0.42, 0.92, 0.96]);
        }
        PropKind::AirGappedTerminal => {
            b.obb([0.0, 0.0], [0.0, top * 0.56], [hx * 0.88, hz * 0.84], [0.30, 0.34, 0.39]);
            b.obb([0.0, -hz * 0.22], [top * 0.56, top], [hx * 0.72, hz * 0.16], rgb);
            b.obb([0.0, hz * 0.25], [top * 0.56, top * 0.64], [hx * 0.52, hz * 0.22], [0.75, 0.80, 0.86]);
            b.obb([0.0, -hz * 0.48], [0.12, top * 0.56], [hx * 0.78, 0.04], [0.76, 0.45, 0.16]);
        }
        PropKind::BiometricTurnstile => {
            b.obb([0.0, -hz * 0.64], [0.0, top], [hx * 0.92, hz * 0.22], rgb);
            b.obb([0.0, hz * 0.64], [0.0, top], [hx * 0.92, hz * 0.22], rgb);
            b.obb([0.0, 0.0], [0.18, top * 0.85], [0.04, hz * 0.42], [0.38, 0.78, 0.95]);
            b.obb([hx * 0.62, -hz * 0.64], [top, top + 0.06], [0.08, 0.08], [0.18, 0.84, 0.42]);
        }
        PropKind::SecurityCameraMast => {
            b.obb([0.0, 0.0], [0.0, 0.10], [hx * 0.60, hz * 0.60], [0.28, 0.32, 0.36]);
            b.obb([0.0, 0.0], [0.10, top], [0.07, 0.07], rgb);
            b.obb([hx * 0.26, 0.0], [top * 0.86, top], [0.22, 0.10], [0.88, 0.91, 0.95]);
            b.obb([hx * 0.48, 0.0], [top * 0.88, top * 0.96], [0.05, 0.07], [0.92, 0.24, 0.20]);
        }
        PropKind::DroneDockPad => {
            b.obb([0.0, 0.0], [0.0, top], [hx * 0.96, hz * 0.96], rgb);
            let beacon = [0.96, 0.80, 0.18];
            b.obb([-hx * 0.72, -hz * 0.72], [top, top + 0.03], [0.12, 0.12], beacon);
            b.obb([hx * 0.72, -hz * 0.72], [top, top + 0.03], [0.12, 0.12], beacon);
            b.obb([-hx * 0.72, hz * 0.72], [top, top + 0.03], [0.12, 0.12], beacon);
            b.obb([hx * 0.72, hz * 0.72], [top, top + 0.03], [0.12, 0.12], beacon);
            b.obb([0.0, 0.0], [top, top + 0.02], [hx * 0.44, 0.09], beacon);
        }
        PropKind::HvTransformer => {
            b.obb([0.0, 0.0], [0.0, 0.10], [hx * 0.94, hz * 0.94], [0.28, 0.30, 0.33]);
            b.obb([0.0, 0.0], [0.10, top * 0.86], [hx * 0.78, hz * 0.74], rgb);
            for fx in [-hx * 0.36, 0.0, hx * 0.36] {
                b.obb([fx, -hz * 0.30], [top * 0.86, top + 0.12], [0.09, 0.09], [0.88, 0.90, 0.92]);
            }
            b.obb([0.0, hz * 0.75], [top * 0.38, top * 0.62], [hx * 0.24, 0.04], [0.96, 0.78, 0.14]);
        }
        PropKind::DieselGenerator => {
            b.obb([0.0, 0.0], [0.0, 0.10], [hx * 0.96, hz * 0.90], [0.20, 0.22, 0.25]);
            b.obb([0.0, 0.0], [0.10, top * 0.90], [hx * 0.90, hz * 0.82], rgb);
            b.obb([-hx * 0.15, 0.0], [top * 0.24, top * 0.78], [hx * 0.48, hz * 0.85], [0.22, 0.26, 0.31]);
            b.obb([hx * 0.68, -hz * 0.45], [top * 0.90, top + 0.16], [0.08, 0.08], [0.16, 0.18, 0.22]);
        }
        PropKind::BatteryBankWall => {
            b.obb([0.0, 0.0], [0.0, top], [hx * 0.95, hz * 0.88], rgb);
            b.obb([0.0, 0.0], [top * 0.96, top + 0.03], [hx * 0.88, 0.06], [0.80, 0.46, 0.16]);
            b.obb([0.0, hz * 0.89], [top * 0.55, top * 0.70], [hx * 0.72, 0.03], [0.20, 0.84, 0.42]);
        }
        PropKind::WaterPurifierKiosk => {
            let cap = lighten(rgb, 1.15);
            b.obb([0.0, 0.0], [0.0, top * 0.90], [hx * 0.78, hz * 0.78], rgb);
            b.obb([-hx * 0.28, 0.0], [top * 0.20, top * 0.86], [hx * 0.22, hz * 0.22], [0.86, 0.94, 0.98]);
            b.obb([hx * 0.28, 0.0], [top * 0.20, top * 0.86], [hx * 0.22, hz * 0.22], [0.86, 0.94, 0.98]);
            b.obb([0.0, 0.0], [top * 0.90, top], [hx * 0.84, hz * 0.84], cap);
        }
        _ => return false,
    }
    true
}

/// Appends translucent active-floor 3D copper perimeter strips for [`MapFaradayZone`]s and
/// compass-accurate FOV sector wedges for [`MapSurveillanceNode`]s (`zone_id = -0.35`).
pub fn push_spatial_overlays_3d(verts: &mut Vec<GlbVertex>, ui: &TacticalMapUiState) {
    let active_floor = ui.map.active_floor;
    let (bw_m, bh_m) = ui.map.board_size_m();
    let faraday_col = [0.86_f32, 0.48, 0.14, 0.0];

    for z in &ui.map.faraday_zones {
        if z.floor_index != active_floor {
            continue;
        }
        let x0_m = z.x0_cm.min(z.x1_cm) as f32 * 0.01;
        let x1_m = z.x0_cm.max(z.x1_cm) as f32 * 0.01;
        let y0_m = z.y0_cm.min(z.y1_cm) as f32 * 0.01;
        let y1_m = z.y0_cm.max(z.y1_cm) as f32 * 0.01;
        let [ax, az] = metric_to_world_xz(x0_m, y0_m, bw_m, bh_m);
        let [bx, bz] = metric_to_world_xz(x1_m, y1_m, bw_m, bh_m);
        let y0 = 0.03_f32;
        let y1 = 0.09_f32;
        let rim = 0.06_f32;
        // 4 perimeter curb bars (copper-amber) so miniatures/floor remain crisp inside.
        for [rx0, rz0, rx1, rz1] in [
            [ax, az, bx, az + rim],
            [ax, bz - rim, bx, bz],
            [ax, az, ax + rim, bz],
            [bx - rim, az, bx, bz],
        ] {
            push_quad(
                verts,
                [[rx0, y1, rz0], [rx0, y1, rz1], [rx1, y1, rz1], [rx1, y1, rz0]],
                faraday_col,
                TRANSLUCENT_CANOPY_ZONE,
            );
            push_quad(
                verts,
                [[rx0, y0, rz1], [rx1, y0, rz1], [rx1, y1, rz1], [rx0, y1, rz1]],
                faraday_col,
                TRANSLUCENT_CANOPY_ZONE,
            );
        }
    }

    for n in &ui.map.surveillance_nodes {
        if n.floor_index != active_floor {
            continue;
        }
        let [nx, nz] = metric_to_world_xz(n.x_cm as f32 * 0.01, n.y_cm as f32 * 0.01, bw_m, bh_m);
        let range = (n.range_cm.max(100) as f32 * 0.01).min(18.0);
        let fov = n.fov_deg.clamp(15, 360) as f32;
        let segs = ((fov / 18.0).ceil() as usize).clamp(4, 24);
        let cone_rgb = if n.powered {
            [0.95_f32, 0.28, 0.22]
        } else {
            [0.42_f32, 0.48, 0.56]
        };
        let cone_col = [cone_rgb[0], cone_rgb[1], cone_rgb[2], 0.0];
        let a0 = (n.facing_deg as f32 - fov * 0.5).to_radians();
        let step = fov.to_radians() / segs as f32;
        for i in 0..segs {
            let t0 = a0 + i as f32 * step;
            let t1 = t0 + step;
            let p0 = [nx + range * t0.sin(), nz - range * t0.cos()];
            let p1 = [nx + range * t1.sin(), nz - range * t1.cos()];
            push_tri_up(verts, [nx, nz], p0, p1, 0.04, cone_col, TRANSLUCENT_CANOPY_ZONE);
        }
        push_obb(
            verts,
            nx,
            nz,
            0.04,
            0.24,
            0.09,
            0.09,
            n.facing_deg,
            cone_rgb,
            TRANSLUCENT_CANOPY_ZONE,
        );
    }
}
