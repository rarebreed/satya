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

/// Complete 33-tile battlemap texture catalog synchronized with `src/satya_ml/texture_gen/catalog.py`.
pub const BATTLEMAP_TEXTURE_SPECS: &[BattlemapTextureSpec] = &[
    BattlemapTextureSpec {
        pattern_id: "bm-grass",
        asset_file: "grass_meadow.webp",
        span_px: 256,
        fallback_hex: "#638045",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-gravel",
        asset_file: "gravel_shoal.webp",
        span_px: 128,
        fallback_hex: "#b7afa2",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-tarmac",
        asset_file: "asphalt_road.webp",
        span_px: 256,
        fallback_hex: "#55585c",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-oak",
        asset_file: "wood_oak.webp",
        span_px: 128,
        fallback_hex: "#a47b55",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-dirt",
        asset_file: "packed_dirt.webp",
        span_px: 192,
        fallback_hex: "#9e8260",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-mulch",
        asset_file: "bark_mulch.webp",
        span_px: 128,
        fallback_hex: "#5c402c",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-creek",
        asset_file: "creek_water.webp",
        span_px: 192,
        fallback_hex: "#4689a3",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-ditch",
        asset_file: "drainage_ditch.webp",
        span_px: 128,
        fallback_hex: "#546854",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-paver",
        asset_file: "flagstone_paver.webp",
        span_px: 128,
        fallback_hex: "#cfc7b8",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-concrete",
        asset_file: "concrete_slab.webp",
        span_px: 256,
        fallback_hex: "#bdb9b0",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-stained-conc",
        asset_file: "stained_concrete.webp",
        span_px: 256,
        fallback_hex: "#a8a397",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-hardwood",
        asset_file: "hardwood_plank.webp",
        span_px: 128,
        fallback_hex: "#c8a57e",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-vinyl",
        asset_file: "vinyl_tile.webp",
        span_px: 128,
        fallback_hex: "#d4d8dc",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-leaf-tex",
        asset_file: "foliage_canopy.webp",
        span_px: 128,
        fallback_hex: "#4b7b37",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-rock",
        asset_file: "granite_boulder.webp",
        span_px: 128,
        fallback_hex: "#7b7972",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-fabric",
        asset_file: "woven_fabric.webp",
        span_px: 96,
        fallback_hex: "#d6d3cd",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-steel",
        asset_file: "brushed_steel.webp",
        span_px: 128,
        fallback_hex: "#9aa2a9",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-roof-gravel",
        asset_file: "rooftop_gravel.webp",
        span_px: 128,
        fallback_hex: "#736f68",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-ceramic",
        asset_file: "ceramic_tile.webp",
        span_px: 96,
        fallback_hex: "#dfe5e8",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-brick",
        asset_file: "brick_masonry.webp",
        span_px: 96,
        fallback_hex: "#9c5a49",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-spill",
        asset_file: "hazard_spill.webp",
        span_px: 128,
        fallback_hex: "#9db82c",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-marble",
        asset_file: "marble_floor.webp",
        span_px: 128,
        fallback_hex: "#eceae6",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-terracotta",
        asset_file: "terracotta_tile.webp",
        span_px: 112,
        fallback_hex: "#c67856",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-carpet",
        asset_file: "carpet_plush.webp",
        span_px: 96,
        fallback_hex: "#c5bdb1",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-pool-water",
        asset_file: "pool_water.webp",
        span_px: 160,
        fallback_hex: "#38b6d6",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-wood-deck",
        asset_file: "wood_deck.webp",
        span_px: 128,
        fallback_hex: "#9c6a47",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-hardwood-walnut",
        asset_file: "hardwood_walnut.webp",
        span_px: 128,
        fallback_hex: "#5d3f2c",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-hardwood-ash",
        asset_file: "hardwood_ash.webp",
        span_px: 128,
        fallback_hex: "#d9c6a3",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-gravel-pea",
        asset_file: "gravel_pea.webp",
        span_px: 128,
        fallback_hex: "#c8b38d",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-gravel-slate",
        asset_file: "gravel_slate.webp",
        span_px: 128,
        fallback_hex: "#5b6169",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-grass-lush",
        asset_file: "grass_lush.webp",
        span_px: 192,
        fallback_hex: "#4e8442",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-grass-dry",
        asset_file: "grass_dry.webp",
        span_px: 192,
        fallback_hex: "#9e9555",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-wood-veneer",
        asset_file: "wood_veneer.webp",
        span_px: 128,
        fallback_hex: "#d4cfc7",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-clinical-vinyl",
        asset_file: "clinical_vinyl.webp",
        span_px: 128,
        fallback_hex: "#d2dce0",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-linoleum-check",
        asset_file: "linoleum_check.webp",
        span_px: 128,
        fallback_hex: "#cbd2d6",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-terrazzo",
        asset_file: "terrazzo_stone.webp",
        span_px: 128,
        fallback_hex: "#d8d4cb",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-herringbone-brick",
        asset_file: "herringbone_brick.webp",
        span_px: 128,
        fallback_hex: "#a2614c",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-server-tile",
        asset_file: "server_raised_tile.webp",
        span_px: 128,
        fallback_hex: "#2d353e",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-spa-water",
        asset_file: "spa_jacuzzi_water.webp",
        span_px: 128,
        fallback_hex: "#3ec6dc",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-travertine",
        asset_file: "travertine_stone.webp",
        span_px: 128,
        fallback_hex: "#ddd5c7",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-porcelain",
        asset_file: "glazed_porcelain.webp",
        span_px: 96,
        fallback_hex: "#eef2f5",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-rug-weave",
        asset_file: "persian_rug.webp",
        span_px: 128,
        fallback_hex: "#d4cdc3",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-leather",
        asset_file: "tufted_leather.webp",
        span_px: 96,
        fallback_hex: "#d2ccc4",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-shingle",
        asset_file: "cedar_shingle.webp",
        span_px: 96,
        fallback_hex: "#825e42",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-bark",
        asset_file: "tree_bark.webp",
        span_px: 96,
        fallback_hex: "#6b4c33",
    },
    BattlemapTextureSpec {
        pattern_id: "bm-car-coat",
        asset_file: "metallic_coat.webp",
        span_px: 128,
        fallback_hex: "#cfd4da",
    },
];

fn floor_webp_spec_for_theme_seeded(
    t: &BattlemapArtTheme,
    seed: i64,
) -> (&'static str, i32, &'static str) {
    if t.id == "clinic" {
        return ("ceramic_tile.webp", 96, t.floor_base);
    }
    let variant = (seed.unsigned_abs() % 3) as usize;
    match t.floor {
        BattlemapFloorPattern::Grass => [
            ("grass_meadow.webp", 256, "#638045"),
            ("grass_lush.webp", 192, "#4e8442"),
            ("grass_dry.webp", 192, "#9e9555"),
        ][variant],
        BattlemapFloorPattern::Hardwood => {
            if t.id == "bar_interior" {
                ("hardwood_plank.webp", 128, t.floor_base)
            } else {
                [
                    ("hardwood_plank.webp", 128, t.floor_base),
                    ("hardwood_walnut.webp", 128, "#5d3f2c"),
                    ("hardwood_ash.webp", 128, "#d9c6a3"),
                ][variant]
            }
        }
        BattlemapFloorPattern::PolishedConcrete => ("concrete_slab.webp", 256, t.floor_base),
        BattlemapFloorPattern::StainedConcrete => ("stained_concrete.webp", 256, t.floor_base),
        BattlemapFloorPattern::VinylTile => ("vinyl_tile.webp", 128, t.floor_base),
        BattlemapFloorPattern::Asphalt => ("asphalt_road.webp", 256, t.floor_base),
    }
}

fn push_extra_vector_pattern_defs(out: &mut String) {
    out.push_str(
        r##"<pattern id="bm-paver" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#cfc7b8"/><rect x="2" y="2" width="28" height="28" rx="3" fill="#dbd4c6" opacity=".6"/><rect x="34" y="34" width="28" height="28" rx="3" fill="#c3baa9" opacity=".55"/><path d="M0 32H64M32 0V32M16 32V64M48 32V64M0 0H64M0 0V64" stroke="#938a7a" stroke-width="1.3" opacity=".65"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-tarmac" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#55585c"/><circle cx="9" cy="11" r="1" fill="#fff" opacity=".12"/><circle cx="33" cy="24" r="1.2" fill="#fff" opacity=".1"/><circle cx="19" cy="39" r=".9" fill="#000" opacity=".28"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-marble" width="128" height="128" patternUnits="userSpaceOnUse"><rect width="128" height="128" fill="#eceae6"/><path d="M12 18q28 16 54 6t44 18M4 88q36-12 68 8t48-10" fill="none" stroke="#c8c4bc" stroke-width="1.8" opacity=".6"/><path d="M0 64H128M64 0V128M0 0H128M0 0V128" fill="none" stroke="#b8b4ac" stroke-width="1.1"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-terracotta" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#c67856"/><rect x="1" y="1" width="30" height="30" fill="#cf815e" opacity=".45"/><rect x="33" y="33" width="30" height="30" fill="#b46747" opacity=".4"/><path d="M0 32H64M32 0V64M0 0H64M0 0V64" fill="none" stroke="#7c5646" stroke-width="1.4"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-ceramic" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#dfe5e8"/><path d="M0 16H48M0 32H48M16 0V48M32 0V48M0 0H48M0 0V48" fill="none" stroke="#aeb7bd" stroke-width="1"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-carpet" width="32" height="32" patternUnits="userSpaceOnUse"><rect width="32" height="32" fill="#c5bdb1"/><circle cx="7" cy="9" r="1.2" fill="#9f968a" opacity=".5"/><circle cx="21" cy="24" r="1.2" fill="#dfd9cf" opacity=".55"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-pool-water" width="80" height="80" patternUnits="userSpaceOnUse"><rect width="80" height="80" fill="#38b6d6"/><path d="M4 22q18-7 36 0t36 0M6 58q18 7 36 0t36 0" fill="none" stroke="#a6ecfb" stroke-width="1.8" opacity=".65"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-wood-deck" width="96" height="48" patternUnits="userSpaceOnUse"><rect width="96" height="48" fill="#9c6a47"/><path d="M0 12H96M0 24H96M0 36H96M0 48H96" stroke="#4f331d" stroke-width="1.6"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-hardwood" width="96" height="48" patternUnits="userSpaceOnUse"><rect width="96" height="48" fill="#c8a57e"/><path d="M0 12H96M0 24H96M0 36H96M0 48H96M32 0V12M70 12V24M18 24V36" stroke="#8a6a4a" stroke-width="1"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-hardwood-walnut" width="96" height="48" patternUnits="userSpaceOnUse"><rect width="96" height="48" fill="#5d3f2c"/><path d="M0 12H96M0 24H96M0 36H96M0 48H96M42 0V12M14 12V24" stroke="#362216" stroke-width="1.1"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-hardwood-ash" width="96" height="48" patternUnits="userSpaceOnUse"><rect width="96" height="48" fill="#d9c6a3"/><path d="M0 12H96M0 24H96M0 36H96M0 48H96M28 0V12M64 12V24" stroke="#ab9875" stroke-width="1"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-gravel-pea" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#c8b38d"/><circle cx="11" cy="14" r="3" fill="#decba7" opacity=".75"/><circle cx="31" cy="28" r="2.8" fill="#a8936e" opacity=".75"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-gravel-slate" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#5b6169"/><polygon points="8,14 16,10 18,19 9,20" fill="#707780" opacity=".75"/><polygon points="28,26 36,24 38,34 27,32" fill="#464b52" opacity=".75"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-grass-lush" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#4e8442"/><path d="M12 20l2-6M42 46l2-6" stroke="#6aa65a" stroke-width="1.3"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-grass-dry" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#9e9555"/><path d="M14 24l3-6M46 44l3-6" stroke="#b5ad6e" stroke-width="1.3"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-oak" width="64" height="32" patternUnits="userSpaceOnUse"><rect width="64" height="32" fill="#a47b55"/><path d="M0 16H64" stroke="#6f4f33" stroke-width="1.1"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-wood-veneer" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#d4cfc7"/><path d="M0 16q16 3 32 0t32 0M0 44q18-3 36 0t28 0" fill="none" stroke="#b9b2a8" stroke-width="1" opacity=".55"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-brick" width="48" height="24" patternUnits="userSpaceOnUse"><rect width="48" height="24" fill="#9c5a49"/><path d="M0 12H48M24 0V12M0 12V24" stroke="#c8beb3" stroke-width="1.1" opacity=".65"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-concrete" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#bdb9b0"/><circle cx="18" cy="40" r="1.2" fill="#9a958b"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-steel" width="32" height="32" patternUnits="userSpaceOnUse"><rect width="32" height="32" fill="#9aa2a9"/><path d="M0 8H32M0 24H32" stroke="#b5bdc4" stroke-width=".8" opacity=".45"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-fabric" width="24" height="24" patternUnits="userSpaceOnUse"><rect width="24" height="24" fill="#d6d3cd"/><path d="M0 12H24M12 0V24" stroke="#bab6af" stroke-width=".7" opacity=".4"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-rock" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#7b7972"/><path d="M8 14L24 22L38 12" fill="none" stroke="#575550" stroke-width="1.2"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-spill" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#9db82c"/><ellipse cx="22" cy="24" rx="14" ry="8" fill="#c7e048" opacity=".45"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-roof-gravel" width="32" height="32" patternUnits="userSpaceOnUse"><rect width="32" height="32" fill="#736f68"/><circle cx="10" cy="14" r="2" fill="#59554f"/><circle cx="24" cy="22" r="2" fill="#8b877f"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-clinical-vinyl" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#d2dce0"/><path d="M0 32H64M32 0V64" stroke="#b9c7ce" stroke-width=".9"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-linoleum-check" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#cbd2d6"/><rect width="24" height="24" fill="#b4bec4"/><rect x="24" y="24" width="24" height="24" fill="#b4bec4"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-terrazzo" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#d8d4cb"/><circle cx="14" cy="18" r="2.2" fill="#8e8376"/><circle cx="38" cy="12" r="1.8" fill="#b79678"/><circle cx="46" cy="42" r="2.5" fill="#7a8288"/><circle cx="18" cy="50" r="1.9" fill="#a86d5c"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-herringbone-brick" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#a2614c"/><path d="M0 12H24V24H48M12 24V48M36 0V24" fill="none" stroke="#c8beb3" stroke-width="1.3"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-server-tile" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#2d353e"/><rect x="2" y="2" width="44" height="44" fill="none" stroke="#475569" stroke-width="1.4"/><path d="M12 14h24M12 24h24M12 34h24" stroke="#38bdf8" stroke-opacity=".22" stroke-width="1.2"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-spa-water" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#3ec6dc"/><path d="M4 18q14-8 28 0t28 0M4 46q14-8 28 0t28 0" fill="none" stroke="#c8f8ff" stroke-opacity=".55" stroke-width="1.6"/><circle cx="16" cy="14" r="3" fill="#ecfeff" fill-opacity=".5"/><circle cx="46" cy="34" r="4" fill="#ecfeff" fill-opacity=".42"/><circle cx="24" cy="52" r="2.5" fill="#ecfeff" fill-opacity=".55"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-travertine" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#ddd5c7"/><path d="M0 16q22-5 44 2t20-2M0 44q20 4 40-3t24 3" fill="none" stroke="#bfb49f" stroke-opacity=".48" stroke-width="1.4"/><circle cx="18" cy="28" r="1.4" fill="#b0a48e" fill-opacity=".4"/><circle cx="50" cy="12" r="1.2" fill="#b0a48e" fill-opacity=".35"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-porcelain" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#eef2f5"/><path d="M6 12q14-4 28 2M10 34q16 3 30-3" fill="none" stroke="#ffffff" stroke-opacity=".65" stroke-width="1.4"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-rug-weave" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#d4cdc3"/><path d="M24 4L44 24L24 44L4 24Z" fill="none" stroke="#9f9282" stroke-opacity=".48" stroke-width="1.3"/><path d="M0 12h48M0 24h48M0 36h48" stroke="#ece6dc" stroke-opacity=".32" stroke-width="0.9"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-leather" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#d2ccc4"/><circle cx="12" cy="14" r="4" fill="none" stroke="#b0a89d" stroke-opacity=".35" stroke-width="1"/><circle cx="34" cy="28" r="4.5" fill="none" stroke="#b0a89d" stroke-opacity=".32" stroke-width="1"/><circle cx="16" cy="38" r="3.5" fill="none" stroke="#e8e2da" stroke-opacity=".35" stroke-width="0.9"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-shingle" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#825e42"/><path d="M0 16h48M0 32h48M0 48h48" stroke="#4d3320" stroke-width="1.6" stroke-opacity=".68"/><path d="M12 0v16M32 0v16M22 16v16M42 16v16M8 32v16M30 32v16" stroke="#5b3e26" stroke-width="1.2" stroke-opacity=".6"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-bark" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#6b4c33"/><path d="M8 0v48M20 0q3 24-2 48M32 0q-2 22 2 48M42 0v48" fill="none" stroke="#3e2918" stroke-width="1.8" stroke-opacity=".65"/><path d="M14 6v14M26 20v18M38 10v16" fill="none" stroke="#8b694c" stroke-width="1.1" stroke-opacity=".45"/></pattern>"##,
    );
    out.push_str(
        r##"<pattern id="bm-car-coat" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#cfd4da"/><circle cx="14" cy="18" r="1.2" fill="#ffffff" fill-opacity=".4"/><circle cx="36" cy="12" r="1" fill="#ffffff" fill-opacity=".35"/><circle cx="26" cy="36" r="1.2" fill="#9ea6b2" fill-opacity=".3"/></pattern>"##,
    );
}

/// Resolves a base ground floor pattern override key (e.g. `"grass_lush"`, `"clinical_vinyl"`)
/// onto `(pattern_id, asset_file, span_px, fallback_hex)`.
#[must_use]
pub fn ground_override_texture_spec(key: &str) -> Option<&'static BattlemapTextureSpec> {
    let needle = key.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return None;
    }
    BATTLEMAP_TEXTURE_SPECS.iter().find(|spec| {
        spec.asset_file.strip_suffix(".webp") == Some(needle.as_str())
            || spec.pattern_id == needle.as_str()
    })
}

/// Appends architectural and material `<pattern>` definitions inside `<defs>` according to `mode`.
pub fn push_battlemap_material_defs(
    out: &mut String,
    t: &BattlemapArtTheme,
    mode: BattlemapTextureMode,
) {
    push_battlemap_material_defs_seeded(out, t, mode, 42);
}

/// Seed- and Editor-ground-aware variant of [`push_battlemap_material_defs`] selecting grass/hardwood varieties,
/// optional `ground_floor_override` texture, and optional `ground_tint_rgb` multiply wash.
pub fn push_battlemap_material_defs_with_ground(
    out: &mut String,
    t: &BattlemapArtTheme,
    mode: BattlemapTextureMode,
    seed: i64,
    ground_override: Option<&str>,
    ground_tint_rgb: Option<[u8; 3]>,
) {
    let _ = write!(
        out,
        r##"<pattern id="bm-hatch" width="9" height="9" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="9" height="9" fill="{wall}"/><path d="M0 0V9" stroke="{hatch}" stroke-width="2.2"/></pattern><pattern id="bm-grate" width="8" height="8" patternUnits="userSpaceOnUse"><rect width="8" height="8" fill="{plat}"/><path d="M0 4H8M4 0V8" stroke="#000" stroke-opacity=".24" stroke-width="1.2"/></pattern>"##,
        wall = t.wall_fill,
        hatch = t.wall_hatch,
        plat = t.platform_fill,
    );
    let override_spec = ground_override.and_then(ground_override_texture_spec);

    if !mode.is_painted() {
        push_surface_pattern_defs(out);
        push_extra_vector_pattern_defs(out);
        if let Some(spec) = override_spec {
            let s = spec.span_px;
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="{s}" height="{s}" patternUnits="userSpaceOnUse"><rect width="{s}" height="{s}" fill="url(#{pid})"/>"##,
                pid = spec.pattern_id
            );
            if let Some(rgb) = ground_tint_rgb {
                let hex = satya_schema::format_rgb_hex(rgb);
                let _ = write!(
                    out,
                    r##"<rect width="{s}" height="{s}" fill="{hex}" fill-opacity=".30" style="mix-blend-mode:multiply"/>"##
                );
            }
            out.push_str("</pattern>");
        } else if let Some(rgb) = ground_tint_rgb {
            let mut inner = String::new();
            push_floor_pattern_def(&mut inner, t);
            let hex = satya_schema::format_rgb_hex(rgb);
            if let Some(body) = inner.strip_suffix("</pattern>") {
                let _ = write!(
                    out,
                    r##"{body}<rect width="256" height="256" fill="{hex}" fill-opacity=".28" style="mix-blend-mode:multiply"/></pattern>"##
                );
            } else {
                out.push_str(&inner);
            }
        } else {
            push_floor_pattern_def(out, t);
        }
        return;
    }

    let (floor_file, floor_span, floor_fallback) = if let Some(spec) = override_spec {
        (spec.asset_file, spec.span_px, spec.fallback_hex)
    } else {
        floor_webp_spec_for_theme_seeded(t, seed)
    };
    let _ = write!(
        out,
        r##"<pattern id="bm-floor" width="{floor_span}" height="{floor_span}" patternUnits="userSpaceOnUse"><rect width="{floor_span}" height="{floor_span}" fill="{floor_fallback}"/><image href="{BATTLEMAP_TEXTURE_BASE_URL}/{floor_file}" width="{floor_span}" height="{floor_span}" preserveAspectRatio="none"/>"##,
    );
    if t.id == "bar_interior" && override_spec.is_none() {
        let _ = write!(
            out,
            r##"<rect width="{floor_span}" height="{floor_span}" fill="#3b2310" fill-opacity=".22"/>"##
        );
    }
    if let Some(rgb) = ground_tint_rgb {
        let hex = satya_schema::format_rgb_hex(rgb);
        let _ = write!(
            out,
            r##"<rect width="{floor_span}" height="{floor_span}" fill="{hex}" fill-opacity=".32" style="mix-blend-mode:multiply"/>"##
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

/// Seed-aware variant of [`push_battlemap_material_defs`] selecting grass/hardwood palette varieties.
pub fn push_battlemap_material_defs_seeded(
    out: &mut String,
    t: &BattlemapArtTheme,
    mode: BattlemapTextureMode,
    seed: i64,
) {
    push_battlemap_material_defs_with_ground(out, t, mode, seed, None, None);
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
    use satya_maps::map_dressing::make_prop_at_cm;
    use satya_maps::tactical_map::{SectorPreset, build_procedural_map_request};
    use satya_schema::{EdgeSide, MapWallEdge, MaterialClass, PropKind, WallEdgeKind};
    use std::collections::BTreeSet;
    use std::path::Path;

    #[test]
    fn texture_spec_catalog_covers_all_46_webp_assets() {
        assert_eq!(BATTLEMAP_TEXTURE_SPECS.len(), 46);
        let pids: BTreeSet<&str> = BATTLEMAP_TEXTURE_SPECS
            .iter()
            .map(|s| s.pattern_id)
            .collect();
        let files: BTreeSet<&str> = BATTLEMAP_TEXTURE_SPECS
            .iter()
            .map(|s| s.asset_file)
            .collect();
        assert_eq!(pids.len(), 46);
        assert_eq!(files.len(), 46);
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/textures/map2d");
        for spec in BATTLEMAP_TEXTURE_SPECS {
            assert!(
                root.join(spec.asset_file).is_file(),
                "missing texture asset {}",
                spec.asset_file
            );
        }

        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/textures/map2d");
        for spec in BATTLEMAP_TEXTURE_SPECS {
            assert!(spec.asset_file.ends_with(".webp"));
            assert!(spec.span_px >= 64);
            let full = dir.join(spec.asset_file);
            assert!(
                full.exists(),
                "missing baked WebP texture: {}",
                full.display()
            );
        }
    }

    #[test]
    fn painted_and_vector_modes_toggle_patterns_and_cache_key() {
        let mut ui = TacticalMapUiState {
            map: build_procedural_map_request("tex", SectorPreset::WoodlandCreek, None, 20, 14, 42),
            ..Default::default()
        };
        ui.map
            .props
            .push(make_prop_at_cm(PropKind::Sofa, 350, 350, 15, None, 1));
        ui.map.props.push(make_prop_at_cm(
            PropKind::DiningTable,
            650,
            350,
            45,
            None,
            2,
        ));
        ui.map
            .props
            .push(make_prop_at_cm(PropKind::Boulder, 850, 550, 10, None, 3));
        ui.map.props.push(make_prop_at_cm(
            PropKind::StructuralPillar,
            950,
            450,
            0,
            None,
            4,
        ));
        ui.map.props.push(make_prop_at_cm(
            PropKind::HazardSpill,
            1050,
            450,
            20,
            None,
            5,
        ));
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
        assert!(painted_art.contains("url(#bm-wood-veneer)"));
        assert!(painted_art.contains("url(#bm-oak)"));
        assert!(painted_art.contains("url(#bm-fabric)"));
        assert!(painted_art.contains("url(#bm-rock)"));
        assert!(painted_art.contains("url(#bm-steel)"));
        assert!(painted_art.contains("url(#bm-spill)"));
        assert_eq!(
            painted_art.matches("<g").count(),
            painted_art.matches("</g>").count()
        );

        ui.texture_mode = BattlemapTextureMode::ProceduralVector;
        let vec_key = battlemap_art_cache_key(&ui);
        assert_ne!(painted_key, vec_key, "texture mode toggle must bust cache");
        let vec_art = format_battlemap_art_layer(&ui);
        assert!(!vec_art.contains("/assets/textures/map2d/"));
        assert_eq!(
            vec_art.matches("<g").count(),
            vec_art.matches("</g>").count()
        );
    }
}
