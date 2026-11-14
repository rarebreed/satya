//! # Painterly 2D Battlemap WebP Texture Registry & Pattern Emitter (`map_2d_art_textures`)
//!
//! Maps theme floors, curved surfaces, walls, and top-down props onto seamless `512×512` WebP
//! tiles served from `/assets/textures/map2d/*.webp` in [`BattlemapTextureMode::Painted`], while
//! preserving crisp procedural vector patterns in [`BattlemapTextureMode::ProceduralVector`].

use std::fmt::Write as _;

use wasm_bindgen::prelude::*;

use crate::wasm_client::dom::{local_storage_get, local_storage_set, set_text};
use crate::wasm_client::map_2d_art::{BattlemapArtTheme, BattlemapFloorPattern};
use crate::wasm_client::map_2d_art_overlay::sync_battlemap_svg_host;
use crate::wasm_client::map_2d_art_surfaces::{push_floor_pattern_def, push_surface_pattern_defs};
use crate::wasm_client::map_3d::with_map_ui_mut;

/// `localStorage` persistence key for 2D battlemap painterly vs. vector texture mode.
pub const TEXTURE_MODE_PREF_KEY: &str = "satya.map.texture_mode";
/// Base HTTP route prefix mounted by `satya-signaling` for static WebP battlemap tiles.
pub const BATTLEMAP_TEXTURE_BASE_URL: &str = "/assets/textures/map2d";

/// Rendering toggle between Gemini Nano Banana painted WebP patterns and crisp SVG vector fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BattlemapTextureMode {
    #[default]
    Painted,
    ProceduralVector,
}

impl BattlemapTextureMode {
    pub const ALL: [Self; 2] = [Self::Painted, Self::ProceduralVector];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Painted => "painted",
            Self::ProceduralVector => "vector",
        }
    }

    #[must_use]
    pub const fn button_label(self) -> &'static str {
        match self {
            Self::Painted => "Textures: Painted",
            Self::ProceduralVector => "Textures: Vector",
        }
    }

    #[must_use]
    pub fn from_code_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "vector" | "procedural" | "flat" => Self::ProceduralVector,
            _ => Self::Painted,
        }
    }

    #[must_use]
    pub const fn is_painted(self) -> bool {
        matches!(self, Self::Painted)
    }

    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Painted => Self::ProceduralVector,
            Self::ProceduralVector => Self::Painted,
        }
    }
}

/// Static catalog entry pairing one SVG `<pattern>` ID with a seamless WebP tile under
/// `/assets/textures/map2d/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlemapTextureSpec {
    pub pattern_id: &'static str,
    pub asset_file: &'static str,
    pub span_px: i32,
    pub fallback_hex: &'static str,
}

/// Complete 21-tile battlemap texture catalog synchronized with `src/satya_ml/texture_gen/catalog.py`.
pub const BATTLEMAP_TEXTURE_SPECS: &[BattlemapTextureSpec] = &[
    BattlemapTextureSpec { pattern_id: "bm-grass", asset_file: "grass_meadow.webp", span_px: 256, fallback_hex: "#638045" },
    BattlemapTextureSpec { pattern_id: "bm-gravel", asset_file: "gravel_shoal.webp", span_px: 128, fallback_hex: "#b7afa2" },
    BattlemapTextureSpec { pattern_id: "bm-tarmac", asset_file: "asphalt_road.webp", span_px: 256, fallback_hex: "#55585c" },
    BattlemapTextureSpec { pattern_id: "bm-oak", asset_file: "wood_oak.webp", span_px: 128, fallback_hex: "#a47b55" },
    BattlemapTextureSpec { pattern_id: "bm-dirt", asset_file: "packed_dirt.webp", span_px: 192, fallback_hex: "#9e8260" },
    BattlemapTextureSpec { pattern_id: "bm-mulch", asset_file: "bark_mulch.webp", span_px: 128, fallback_hex: "#5c402c" },
    BattlemapTextureSpec { pattern_id: "bm-creek", asset_file: "creek_water.webp", span_px: 192, fallback_hex: "#4689a3" },
    BattlemapTextureSpec { pattern_id: "bm-ditch", asset_file: "drainage_ditch.webp", span_px: 128, fallback_hex: "#546854" },
    BattlemapTextureSpec { pattern_id: "bm-paver", asset_file: "flagstone_paver.webp", span_px: 128, fallback_hex: "#cfc7b8" },
    BattlemapTextureSpec { pattern_id: "bm-concrete", asset_file: "concrete_slab.webp", span_px: 256, fallback_hex: "#bdb9b0" },
    BattlemapTextureSpec { pattern_id: "bm-stained-conc", asset_file: "stained_concrete.webp", span_px: 256, fallback_hex: "#a8a397" },
    BattlemapTextureSpec { pattern_id: "bm-hardwood", asset_file: "hardwood_plank.webp", span_px: 128, fallback_hex: "#c8a57e" },
    BattlemapTextureSpec { pattern_id: "bm-vinyl", asset_file: "vinyl_tile.webp", span_px: 128, fallback_hex: "#d4d8dc" },
    BattlemapTextureSpec { pattern_id: "bm-leaf-tex", asset_file: "foliage_canopy.webp", span_px: 128, fallback_hex: "#4b7b37" },
    BattlemapTextureSpec { pattern_id: "bm-rock", asset_file: "granite_boulder.webp", span_px: 128, fallback_hex: "#7b7972" },
    BattlemapTextureSpec { pattern_id: "bm-fabric", asset_file: "woven_fabric.webp", span_px: 96, fallback_hex: "#d6d3cd" },
    BattlemapTextureSpec { pattern_id: "bm-steel", asset_file: "brushed_steel.webp", span_px: 128, fallback_hex: "#9aa2a9" },
    BattlemapTextureSpec { pattern_id: "bm-roof-gravel", asset_file: "rooftop_gravel.webp", span_px: 128, fallback_hex: "#736f68" },
    BattlemapTextureSpec { pattern_id: "bm-ceramic", asset_file: "ceramic_tile.webp", span_px: 96, fallback_hex: "#dfe5e8" },
    BattlemapTextureSpec { pattern_id: "bm-brick", asset_file: "brick_masonry.webp", span_px: 96, fallback_hex: "#9c5a49" },
    BattlemapTextureSpec { pattern_id: "bm-spill", asset_file: "hazard_spill.webp", span_px: 128, fallback_hex: "#9db82c" },
];

fn floor_webp_spec_for_theme(t: &BattlemapArtTheme) -> (&'static str, i32, &'static str) {
    if t.id == "clinic" {
        return ("ceramic_tile.webp", 96, t.floor_base);
    }
    match t.floor {
        BattlemapFloorPattern::Grass => ("grass_meadow.webp", 256, "#638045"),
        BattlemapFloorPattern::Hardwood => ("hardwood_plank.webp", 128, t.floor_base),
        BattlemapFloorPattern::PolishedConcrete => ("concrete_slab.webp", 256, t.floor_base),
        BattlemapFloorPattern::StainedConcrete => ("stained_concrete.webp", 256, t.floor_base),
        BattlemapFloorPattern::VinylTile => ("vinyl_tile.webp", 128, t.floor_base),
        BattlemapFloorPattern::Asphalt => ("asphalt_road.webp", 256, t.floor_base),
    }
}

/// Appends architectural and material `<pattern>` definitions inside `<defs>` according to `mode`.
pub fn push_battlemap_material_defs(
    out: &mut String,
    t: &BattlemapArtTheme,
    mode: BattlemapTextureMode,
) {
    let _ = write!(
        out,
        r##"<pattern id="bm-hatch" width="9" height="9" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="9" height="9" fill="{wall}"/><path d="M0 0V9" stroke="{hatch}" stroke-width="2.2"/></pattern><pattern id="bm-grate" width="8" height="8" patternUnits="userSpaceOnUse"><rect width="8" height="8" fill="{plat}"/><path d="M0 4H8M4 0V8" stroke="#000" stroke-opacity=".24" stroke-width="1.2"/></pattern>"##,
        wall = t.wall_fill,
        hatch = t.wall_hatch,
        plat = t.platform_fill,
    );

    if !mode.is_painted() {
        push_floor_pattern_def(out, t);
        push_surface_pattern_defs(out);
        out.push_str(
            r##"<pattern id="bm-paver" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#cfc7b8"/><rect x="2" y="2" width="28" height="28" rx="3" fill="#dbd4c6" opacity=".6"/><rect x="34" y="34" width="28" height="28" rx="3" fill="#c3baa9" opacity=".55"/><path d="M0 32H64M32 0V32M16 32V64M48 32V64M0 0H64M0 0V64" stroke="#938a7a" stroke-width="1.3" opacity=".65"/></pattern><pattern id="bm-tarmac" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#55585c"/><circle cx="9" cy="11" r="1" fill="#fff" opacity=".12"/><circle cx="33" cy="24" r="1.2" fill="#fff" opacity=".1"/><circle cx="19" cy="39" r=".9" fill="#000" opacity=".28"/></pattern>"##,
        );
        return;
    }

    let (floor_file, floor_span, floor_fallback) = floor_webp_spec_for_theme(t);
    let _ = write!(
        out,
        r##"<pattern id="bm-floor" width="{floor_span}" height="{floor_span}" patternUnits="userSpaceOnUse"><rect width="{floor_span}" height="{floor_span}" fill="{floor_fallback}"/><image href="{BATTLEMAP_TEXTURE_BASE_URL}/{floor_file}" width="{floor_span}" height="{floor_span}" preserveAspectRatio="none"/>"##,
    );
    if t.id == "bar_interior" {
        let _ = write!(
            out,
            r##"<rect width="{floor_span}" height="{floor_span}" fill="#3b2310" fill-opacity=".22"/>"##
        );
    }
    out.push_str("</pattern>");

    for spec in BATTLEMAP_TEXTURE_SPECS {
        let s = spec.span_px;
        let _ = write!(
            out,
            r##"<pattern id="{pid}" width="{s}" height="{s}" patternUnits="userSpaceOnUse"><rect width="{s}" height="{s}" fill="{fb}"/><image href="{BATTLEMAP_TEXTURE_BASE_URL}/{file}" width="{s}" height="{s}" preserveAspectRatio="none"/></pattern>"##,
            pid = spec.pattern_id,
            fb = spec.fallback_hex,
            file = spec.asset_file,
        );
    }
}

/// Loads the persisted `localStorage` texture mode (`"painted"` / `"vector"`).
#[must_use]
pub fn load_persisted_texture_mode_pref() -> BattlemapTextureMode {
    local_storage_get(TEXTURE_MODE_PREF_KEY)
        .map(|s| BattlemapTextureMode::from_code_str(&s))
        .unwrap_or_default()
}

/// Toggles between painted Gemini WebP textures and procedural SVG vector patterns (`#btn-map-tex-mode`).
#[wasm_bindgen]
pub fn satya_toggle_battlemap_textures() {
    let next = with_map_ui_mut(|ui| {
        ui.texture_mode = ui.texture_mode.toggled();
        ui.texture_mode
    });
    local_storage_set(TEXTURE_MODE_PREF_KEY, next.as_str());
    set_text("btn-map-tex-mode", next.button_label());
    sync_battlemap_svg_host();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::map_2d_art::{battlemap_art_cache_key, format_battlemap_art_layer};
    use crate::wasm_client::map_3d::TacticalMapUiState;
    use satya_engine::map_dressing::make_prop_at_cm;
    use satya_engine::tactical_map::{SectorPreset, build_procedural_map_request};
    use satya_schema::{EdgeSide, MapWallEdge, MaterialClass, PropKind, WallEdgeKind};
    use std::collections::BTreeSet;
    use std::path::Path;

    #[test]
    fn texture_spec_catalog_covers_all_21_webp_assets() {
        assert_eq!(BATTLEMAP_TEXTURE_SPECS.len(), 21);
        let pids: BTreeSet<&str> = BATTLEMAP_TEXTURE_SPECS.iter().map(|s| s.pattern_id).collect();
        let files: BTreeSet<&str> = BATTLEMAP_TEXTURE_SPECS.iter().map(|s| s.asset_file).collect();
        assert_eq!(pids.len(), 21);
        assert_eq!(files.len(), 21);

        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/textures/map2d");
        for spec in BATTLEMAP_TEXTURE_SPECS {
            assert!(spec.asset_file.ends_with(".webp"));
            assert!(spec.span_px >= 64);
            let full = dir.join(spec.asset_file);
            assert!(full.exists(), "missing baked WebP texture: {}", full.display());
        }
    }

    #[test]
    fn painted_and_vector_modes_toggle_patterns_and_cache_key() {
        let mut ui = TacticalMapUiState {
            map: build_procedural_map_request("tex", SectorPreset::WoodlandCreek, None, 20, 14, 42),
            ..Default::default()
        };
        ui.map.props.push(make_prop_at_cm(PropKind::Sofa, 350, 350, 15, None, 1));
        ui.map.props.push(make_prop_at_cm(PropKind::DiningTable, 650, 350, 45, None, 2));
        ui.map.props.push(make_prop_at_cm(PropKind::Boulder, 850, 550, 10, None, 3));
        ui.map.props.push(make_prop_at_cm(PropKind::StructuralPillar, 950, 450, 0, None, 4));
        ui.map.props.push(make_prop_at_cm(PropKind::HazardSpill, 1050, 450, 20, None, 5));
        ui.map.wall_edges.push(MapWallEdge {
            x: 1,
            y: 1,
            side: EdgeSide::North,
            kind: WallEdgeKind::DoorClosed,
            material: MaterialClass::Drywall,
        });

        assert_eq!(ui.texture_mode, BattlemapTextureMode::Painted);
        let painted_key = battlemap_art_cache_key(&ui);
        let painted_art = format_battlemap_art_layer(&ui);
        assert!(painted_art.contains("/assets/textures/map2d/grass_meadow.webp"));
        assert!(painted_art.contains("url(#bm-oak)"));
        assert!(painted_art.contains("url(#bm-fabric)"));
        assert!(painted_art.contains("url(#bm-rock)"));
        assert!(painted_art.contains("url(#bm-steel)"));
        assert!(painted_art.contains("url(#bm-spill)"));
        assert_eq!(painted_art.matches("<g").count(), painted_art.matches("</g>").count());

        ui.texture_mode = BattlemapTextureMode::ProceduralVector;
        let vec_key = battlemap_art_cache_key(&ui);
        assert_ne!(painted_key, vec_key, "texture mode toggle must bust cache");
        let vec_art = format_battlemap_art_layer(&ui);
        assert!(!vec_art.contains("/assets/textures/map2d/"));
        assert_eq!(vec_art.matches("<g").count(), vec_art.matches("</g>").count());
    }
}
