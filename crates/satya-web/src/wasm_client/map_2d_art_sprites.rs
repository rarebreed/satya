//! # Painted Top-Down Battlemap Prop Sprite Registry (`map_2d_art_sprites`)
//!
//! Maps [`PropKind`] placements onto alpha-matted top-down WebP sprites served from
//! `/assets/sprites/props/*.webp` when [`BattlemapTextureMode::Painted`] is active, while keeping
//! procedural SVG glyphs as instant fallback for [`BattlemapTextureMode::ProceduralVector`], custom
//! GM tinting, or stretched runs.

use std::fmt::Write as _;

use satya_schema::{MapPropPlacement, PropKind};

/// Base HTTP route prefix mounted by `satya-signaling` for painted prop sprites.
pub const BATTLEMAP_PROP_SPRITE_BASE_URL: &str = "/assets/sprites/props";

/// Static catalog entry pairing one [`PropKind`] with an alpha-matted WebP prop sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropSpriteSpec {
    pub kind: PropKind,
    pub asset_file: &'static str,
    pub base_w_cells: i32,
    pub base_h_cells: i32,
}

/// Clinic pilot pack of 12 Nano Banana 2 top-down prop sprites synchronized with
/// `src/satya_ml/texture_gen/prop_catalog.py`.
pub const BATTLEMAP_PROP_SPRITE_SPECS: &[PropSpriteSpec] = &[
    PropSpriteSpec {
        kind: PropKind::HospitalBed,
        asset_file: "hospital_bed.webp",
        base_w_cells: 2,
        base_h_cells: 1,
    },
    PropSpriteSpec {
        kind: PropKind::Gurney,
        asset_file: "gurney.webp",
        base_w_cells: 1,
        base_h_cells: 2,
    },
    PropSpriteSpec {
        kind: PropKind::SurgicalTable,
        asset_file: "surgical_table.webp",
        base_w_cells: 2,
        base_h_cells: 1,
    },
    PropSpriteSpec {
        kind: PropKind::ImagingScanner,
        asset_file: "imaging_scanner.webp",
        base_w_cells: 2,
        base_h_cells: 2,
    },
    PropSpriteSpec {
        kind: PropKind::NurseStation,
        asset_file: "nurse_station.webp",
        base_w_cells: 3,
        base_h_cells: 2,
    },
    PropSpriteSpec {
        kind: PropKind::TelemetryMonitor,
        asset_file: "telemetry_monitor.webp",
        base_w_cells: 1,
        base_h_cells: 1,
    },
    PropSpriteSpec {
        kind: PropKind::LinenCart,
        asset_file: "linen_cart.webp",
        base_w_cells: 2,
        base_h_cells: 1,
    },
    PropSpriteSpec {
        kind: PropKind::BiohazardBin,
        asset_file: "biohazard_bin.webp",
        base_w_cells: 1,
        base_h_cells: 1,
    },
    PropSpriteSpec {
        kind: PropKind::CryoComputeCluster,
        asset_file: "cryo_compute_cluster.webp",
        base_w_cells: 2,
        base_h_cells: 1,
    },
    PropSpriteSpec {
        kind: PropKind::BioServerVat,
        asset_file: "bio_server_vat.webp",
        base_w_cells: 1,
        base_h_cells: 1,
    },
    PropSpriteSpec {
        kind: PropKind::DieselGenerator,
        asset_file: "diesel_generator.webp",
        base_w_cells: 2,
        base_h_cells: 1,
    },
    PropSpriteSpec {
        kind: PropKind::UtilityVan,
        asset_file: "utility_van.webp",
        base_w_cells: 2,
        base_h_cells: 5,
    },
];

/// Resolves a painted sprite spec for `p` unless custom-tinted or stretched beyond natural aspect.
#[must_use]
pub fn prop_sprite_for_placement(p: &MapPropPlacement) -> Option<&'static PropSpriteSpec> {
    if p.tint_rgb.is_some() {
        return None;
    }
    let spec = BATTLEMAP_PROP_SPRITE_SPECS
        .iter()
        .find(|s| s.kind == p.kind)?;
    let fp = p.footprint_m();
    let actual_aspect = fp.span_w_m / fp.span_h_m.max(0.05);
    let base_aspect = spec.base_w_cells as f32 / spec.base_h_cells as f32;
    let ratio = actual_aspect / base_aspect;
    (0.72..=1.38).contains(&ratio).then_some(spec)
}

/// Emits an SVG `<image>` element centered at `(0, 0)` spanning `lw × lh` SVG units.
pub fn push_prop_sprite_image(out: &mut String, spec: &PropSpriteSpec, lw: f32, lh: f32) {
    let (l, t) = (lw * 0.5, lh * 0.5);
    let _ = write!(
        out,
        r#"<image href="{BATTLEMAP_PROP_SPRITE_BASE_URL}/{}" x="{:.1}" y="{:.1}" width="{lw:.1}" height="{lh:.1}" preserveAspectRatio="xMidYMid meet"/>"#,
        spec.asset_file, -l, -t,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::map_2d_art_furniture::push_furniture_glyph_with_mode;
    use crate::wasm_client::map_2d_art_textures::BattlemapTextureMode;
    use satya_maps::map_dressing::make_prop;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn clinic_pilot_sprites_exist_and_toggle_with_texture_mode() {
        assert_eq!(BATTLEMAP_PROP_SPRITE_SPECS.len(), 12);
        let kinds: BTreeSet<u16> = BATTLEMAP_PROP_SPRITE_SPECS
            .iter()
            .map(|s| s.kind as u16)
            .collect();
        let files: BTreeSet<&str> = BATTLEMAP_PROP_SPRITE_SPECS
            .iter()
            .map(|s| s.asset_file)
            .collect();
        assert_eq!(kinds.len(), 12);
        assert_eq!(files.len(), 12);

        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/sprites/props");
        for spec in BATTLEMAP_PROP_SPRITE_SPECS {
            assert!(spec.asset_file.ends_with(".webp"));
            assert!(
                root.join(spec.asset_file).is_file(),
                "missing baked prop sprite {}",
                spec.asset_file
            );
        }

        let bed = make_prop(PropKind::HospitalBed, 4, 4, 90, None, 1);
        let mut painted = String::new();
        push_furniture_glyph_with_mode(&mut painted, &bed, 64.0, BattlemapTextureMode::Painted);
        assert!(painted.contains("/assets/sprites/props/hospital_bed.webp"));
        assert_eq!(painted.matches("<g ").count(), painted.matches("</g>").count());

        let mut vector = String::new();
        push_furniture_glyph_with_mode(
            &mut vector,
            &bed,
            64.0,
            BattlemapTextureMode::ProceduralVector,
        );
        assert!(!vector.contains("/assets/sprites/props/"));
        assert_eq!(vector.matches("<g ").count(), vector.matches("</g>").count());

        let mut tinted_bed = bed.clone();
        tinted_bed.tint_rgb = Some([200, 120, 120]);
        assert!(prop_sprite_for_placement(&tinted_bed).is_none());
    }
}
