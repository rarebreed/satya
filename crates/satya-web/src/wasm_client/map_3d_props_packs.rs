//! # 3D Diorama Props for Expanded Asset Packs (`map_3d_props_packs`)
//!
//! Companion to [`crate::wasm_client::map_3d_dressing_geom`] that emits multi-part oriented
//! 3D geometry for the 26 Cafe, Retail, Industrial, Farm, Conservatory, Sci-Fi, Office & Streetscape
//! [`PropKind`]s (Batches 8–10).

use crate::wasm_client::glb_model::GlbVertex;
use crate::wasm_client::map_3d_dressing_geom::{lighten, push_obb};
use crate::wasm_client::map_3d_props_lab_urban::apply_prop_tint;
use satya_maps::map_dressing::prop_spec;
use satya_schema::{MapPropPlacement, PropKind};

struct PackPropBuilder<'a> {
    verts: &'a mut Vec<GlbVertex>,
    cx: f32,
    cz: f32,
    sn: f32,
    cs: f32,
    rot: i32,
}

impl PackPropBuilder<'_> {
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

/// Emits multi-part opaque 3D geometry for Batches 8–10 props. Returns `false` for other kinds.
pub(crate) fn push_pack_prop_3d(
    verts: &mut Vec<GlbVertex>,
    p: &MapPropPlacement,
    center_xz: [f32; 2],
    half_xz: [f32; 2],
    top: f32,
    cutaway: bool,
) -> bool {
    let fp = p.footprint_m();
    let [cx, cz] = center_xz;
    let [hx, hz] = half_xz;
    let rgb = apply_prop_tint(prop_spec(p.kind).albedo_rgb, p.tint_rgb);
    let mut b = PackPropBuilder {
        verts,
        cx,
        cz,
        sn: fp.sin_rot,
        cs: fp.cos_rot,
        rot: fp.rot_deg,
    };

    match p.kind {
        PropKind::EspressoCounter => {
            b.obb([0.0, 0.0], [0.0, top * 0.78], [hx * 0.94, hz * 0.88], rgb);
            b.obb([0.0, 0.0], [top * 0.78, top * 0.84], [hx * 0.98, hz * 0.92], lighten(rgb, 1.18));
            b.obb([-hx * 0.42, -hz * 0.10], [top * 0.84, top * 1.15], [hx * 0.34, hz * 0.42], [0.78, 0.82, 0.86]);
            b.obb([hx * 0.38, 0.0], [top * 0.84, top * 1.08], [hx * 0.38, hz * 0.50], [0.72, 0.88, 0.96]);
        }
        PropKind::CommercialPrepTable | PropKind::BuffetCounter => {
            b.obb([0.0, 0.0], [0.0, top * 0.25], [hx * 0.88, hz * 0.80], [0.46, 0.50, 0.54]);
            b.obb([0.0, 0.0], [top * 0.78, top], [hx * 0.95, hz * 0.90], rgb);
            b.obb([-hx * 0.35, 0.0], [top, top + 0.06], [hx * 0.32, hz * 0.55], [0.80, 0.68, 0.48]);
        }
        PropKind::CommercialGrillHood => {
            b.obb([0.0, 0.0], [0.0, top * 0.82], [hx * 0.94, hz * 0.88], rgb);
            b.obb([-hx * 0.32, 0.0], [top * 0.82, top * 0.88], [hx * 0.48, hz * 0.68], [0.22, 0.24, 0.27]);
            if !cutaway {
                b.obb([0.0, -hz * 0.12], [top * 1.55, top * 1.85], [hx * 0.96, hz * 0.78], [0.78, 0.81, 0.84]);
            }
        }
        PropKind::PatioUmbrellaTable => {
            b.obb([0.0, 0.0], [0.0, 0.74], [hx * 0.46, hz * 0.46], [0.42, 0.28, 0.18]);
            b.obb([0.0, 0.0], [0.74, top * 0.88], [0.04, 0.04], [0.48, 0.36, 0.24]);
            b.obb([0.0, 0.0], [top * 0.88, top], [hx * 0.84, hz * 0.84], rgb);
        }
        PropKind::RefrigeratedDisplayCase | PropKind::VendingMachine | PropKind::EvaSuitLocker => {
            b.obb([0.0, 0.0], [0.0, top], [hx * 0.92, hz * 0.90], rgb);
            b.obb([0.0, hz * 0.78], [0.18, top * 0.88], [hx * 0.80, hz * 0.12], [0.68, 0.88, 0.96]);
        }
        PropKind::ClothingRack => {
            b.obb([0.0, 0.0], [0.0, 0.08], [hx * 0.88, hz * 0.72], [0.28, 0.30, 0.34]);
            b.obb([0.0, 0.0], [0.08, top], [0.04, 0.04], [0.80, 0.83, 0.86]);
            b.obb([0.0, 0.0], [top * 0.38, top * 0.92], [hx * 0.82, hz * 0.54], rgb);
        }
        PropKind::PharmacyCounter | PropKind::CommandConsole => {
            b.obb([0.0, 0.0], [0.0, top * 0.78], [hx * 0.94, hz * 0.88], rgb);
            b.obb([0.0, -hz * 0.30], [top * 0.78, top * 1.15], [hx * 0.76, hz * 0.12], [0.22, 0.76, 0.90]);
        }
        PropKind::AtmKiosk | PropKind::CopyMachine => {
            b.obb([0.0, 0.0], [0.0, top], [hx * 0.86, hz * 0.86], rgb);
            b.obb([0.0, hz * 0.76], [top * 0.50, top * 0.82], [hx * 0.58, hz * 0.12], [0.26, 0.72, 0.92]);
        }
        PropKind::RoboticAssemblyArm => {
            b.obb([0.0, 0.0], [0.0, 0.24], [hx * 0.68, hz * 0.68], [0.24, 0.26, 0.30]);
            b.obb([0.0, 0.0], [0.24, top * 0.65], [hx * 0.30, hz * 0.30], rgb);
            b.obb([hx * 0.32, 0.0], [top * 0.55, top * 0.85], [hx * 0.45, hz * 0.16], lighten(rgb, 1.12));
        }
        PropKind::ToolChestWorkbench => {
            b.obb([0.0, 0.0], [0.0, top * 0.90], [hx * 0.92, hz * 0.86], rgb);
            b.obb([0.0, 0.0], [top * 0.90, top], [hx * 0.96, hz * 0.90], [0.74, 0.56, 0.34]);
        }
        PropKind::CargoPallet | PropKind::HayBaleStack => {
            b.obb([0.0, 0.0], [0.0, 0.14], [hx * 0.92, hz * 0.90], [0.52, 0.38, 0.22]);
            b.obb([0.0, 0.0], [0.14, top], [hx * 0.86, hz * 0.84], rgb);
        }
        PropKind::ElectricalSwitchgear => {
            b.obb([0.0, 0.0], [0.0, top], [hx * 0.92, hz * 0.88], rgb);
            b.obb([-hx * 0.45, 0.0], [top, top + 0.14], [0.08, 0.08], [0.76, 0.48, 0.20]);
            b.obb([hx * 0.45, 0.0], [top, top + 0.14], [0.08, 0.08], [0.76, 0.48, 0.20]);
        }
        PropKind::Tractor => {
            b.obb([0.0, -hz * 0.24], [0.18, top * 0.64], [hx * 0.58, hz * 0.56], rgb);
            b.obb([0.0, hz * 0.28], [0.24, top], [hx * 0.72, hz * 0.42], [0.18, 0.22, 0.26]);
            b.obb([-hx * 0.84, hz * 0.32], [0.0, top * 0.48], [hx * 0.16, hz * 0.34], [0.14, 0.15, 0.17]);
            b.obb([hx * 0.84, hz * 0.32], [0.0, top * 0.48], [hx * 0.16, hz * 0.34], [0.14, 0.15, 0.17]);
        }
        PropKind::LivestockTrough => {
            b.obb([0.0, 0.0], [0.0, top], [hx * 0.92, hz * 0.86], rgb);
            b.obb([0.0, 0.0], [top * 0.55, top * 0.92], [hx * 0.80, hz * 0.70], [0.16, 0.56, 0.78]);
        }
        PropKind::ConservatorySpecimenIsland => {
            b.obb([0.0, 0.0], [0.0, 0.38], [hx * 0.90, hz * 0.90], [0.68, 0.62, 0.54]);
            b.obb([0.0, 0.0], [0.38, top * 0.85], [hx * 0.78, hz * 0.78], rgb);
            b.obb([0.0, 0.0], [top * 0.75, top], [hx * 0.46, hz * 0.46], lighten(rgb, 1.25));
        }
        PropKind::SciFiCargoPod => {
            b.obb([0.0, 0.0], [0.0, top], [hx * 0.90, hz * 0.86], rgb);
            b.obb([0.0, 0.0], [top, top + 0.06], [hx * 0.68, hz * 0.42], [0.22, 0.72, 0.90]);
        }
        PropKind::HullDebris => {
            b.obb([-hx * 0.20, 0.0], [0.0, top * 0.85], [hx * 0.65, hz * 0.68], rgb);
            b.obb([hx * 0.25, hz * 0.15], [0.0, top], [hx * 0.45, hz * 0.52], lighten(rgb, 1.15));
        }
        PropKind::ConferenceTable => {
            b.obb([0.0, 0.0], [0.0, top * 0.85], [hx * 0.42, hz * 0.36], [0.28, 0.30, 0.34]);
            b.obb([0.0, 0.0], [top * 0.85, top], [hx * 0.86, hz * 0.74], rgb);
        }
        PropKind::StreetLamp => {
            b.obb([0.0, 0.0], [0.0, 0.14], [0.18, 0.18], [0.24, 0.26, 0.30]);
            b.obb([0.0, 0.0], [0.14, top], [0.06, 0.06], rgb);
            b.obb([0.24, 0.0], [top * 0.92, top], [0.26, 0.10], [0.96, 0.92, 0.70]);
        }
        PropKind::BusShelter => {
            b.obb([0.0, -hz * 0.72], [0.0, top * 0.92], [hx * 0.94, 0.05], [0.58, 0.78, 0.90]);
            b.obb([0.0, 0.0], [top * 0.92, top], [hx * 0.98, hz * 0.92], rgb);
            b.obb([0.0, 0.0], [0.0, 0.48], [hx * 0.68, hz * 0.36], [0.46, 0.34, 0.24]);
        }
        _ => return false,
    }
    true
}
