//! # Illustrated 2D Battlemap Renderer (`map_2d_art`)
//!
//! Draws the tactical map as a modern, hand-inked top-down battlemap: floor materials, hatched walls,
//! door swings, raised grating, hazard spills and theme-appropriate props, topped by a crisp tactical
//! overlay (`1.0m` grid, gait reach envelope, LoS ray, top-down tokens).
//!
//! `#tabletop-map-svg-fallback` holds two `<svg>` layers that share one `viewBox` (`64` units per cell):
//! - `#battlemap-art` is the static layer with filters. It is rebuilt only when its `data-art-key`
//!   (a hash of the tiles, size, seed and theme) changes.
//! - `#battlemap-overlay` is the dynamic layer. It is cheap to redraw on hover, selection and movement,
//!   and uses one pointer handler that maps clicks to `window.satyaClickMapCell(x, y)`.
//!
//! Phase 1 derives all art from `TacticalTileKind`, `theme_preset` and `seed`, so the schema is unchanged.
//! [`BATTLEMAP_ART_THEMES`] is a `const` table, so other settings can add themes without touching the
//! composer. Prop glyphs live in [`crate::wasm_client::map_2d_art_props`].

pub use crate::wasm_client::map_2d_art_furniture::*;
pub use crate::wasm_client::map_2d_art_overlay::*;
pub use crate::wasm_client::map_2d_art_props::*;
pub use crate::wasm_client::map_2d_art_walls::*;

use satya_engine::map_dressing::{make_prop, prop_spec, solid_prop_at};

use crate::wasm_client::map_3d::TacticalMapUiState;
use satya_schema::{PropKind, TacticalTileKind};
use std::fmt::Write as _;

/// SVG user units per `1.0m` grid cell.
pub const BATTLEMAP_CELL_PX: i32 = 64;
/// Exterior margin around the board so perimeter edge walls are not clipped.
pub const BATTLEMAP_PAD_PX: i32 = 12;
const CELL_F: f32 = BATTLEMAP_CELL_PX as f32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlemapFloorPattern {
    Hardwood,
    PolishedConcrete,
    VinylTile,
    Asphalt,
    StainedConcrete,
    Grass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlemapWallStyle {
    /// Architectural floorplan poché (solid hatched wall mass).
    Poche,
    /// Building blocks seen from above (roof slab, parapet, HVAC units).
    Rooftop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlemapHazardStyle {
    ChemicalSpill,
    Biohazard,
    Fire,
    WaterLeak,
}

/// One visual theme: palette, materials and prop vocabulary for a setting or location type.
#[derive(Debug, Clone, Copy)]
pub struct BattlemapArtTheme {
    pub id: &'static str,
    pub label: &'static str,
    pub floor: BattlemapFloorPattern,
    pub floor_base: &'static str,
    pub floor_line: &'static str,
    pub wall_style: BattlemapWallStyle,
    pub wall_fill: &'static str,
    pub wall_hatch: &'static str,
    pub door_fill: &'static str,
    pub platform_fill: &'static str,
    pub hazard: BattlemapHazardStyle,
    /// Sidewalk/apron fill drawn around wall blocks (street themes).
    pub apron_fill: Option<&'static str>,
    pub lane_markings: bool,
    pub floor_stains: bool,
    /// Grid-overlay ink (dark on light floors, light on asphalt).
    pub grid_ink: &'static str,
    pub cover_props: &'static [BattlemapPropKind],
}

/// Canonical battlemap themes (modern-day defaults). Add rows here to support other settings.
pub const BATTLEMAP_ART_THEMES: &[BattlemapArtTheme] = &[
    BattlemapArtTheme {
        id: "modern_interior",
        label: "Modern Interior (Apartment / Condo)",
        floor: BattlemapFloorPattern::Hardwood,
        floor_base: "#c8a57e",
        floor_line: "#8a6a4a",
        wall_style: BattlemapWallStyle::Poche,
        wall_fill: "#3b3835",
        wall_hatch: "#5a5550",
        door_fill: "#9a6b43",
        platform_fill: "#8e969c",
        hazard: BattlemapHazardStyle::WaterLeak,
        apron_fill: None,
        lane_markings: false,
        floor_stains: false,
        grid_ink: "#1f1b16",
        cover_props: &[
            BattlemapPropKind::Sofa,
            BattlemapPropKind::Armchair,
            BattlemapPropKind::OfficeDesk,
            BattlemapPropKind::KitchenIsland,
            BattlemapPropKind::Planter,
            BattlemapPropKind::StorageCabinet,
        ],
    },
    BattlemapArtTheme {
        id: "office_interior",
        label: "Corporate Office Suite",
        floor: BattlemapFloorPattern::VinylTile,
        floor_base: "#d4d8dc",
        floor_line: "#aeb5bc",
        wall_style: BattlemapWallStyle::Poche,
        wall_fill: "#363b42",
        wall_hatch: "#525963",
        door_fill: "#7c8b99",
        platform_fill: "#8e969c",
        hazard: BattlemapHazardStyle::WaterLeak,
        apron_fill: None,
        lane_markings: false,
        floor_stains: false,
        grid_ink: "#1c2026",
        cover_props: &[
            BattlemapPropKind::OfficeDesk,
            BattlemapPropKind::StorageCabinet,
            BattlemapPropKind::Sofa,
            BattlemapPropKind::Planter,
        ],
    },
    BattlemapArtTheme {
        id: "industrial",
        label: "Industrial / Warehouse",
        floor: BattlemapFloorPattern::PolishedConcrete,
        floor_base: "#bdb9b0",
        floor_line: "#8f8a80",
        wall_style: BattlemapWallStyle::Poche,
        wall_fill: "#45474a",
        wall_hatch: "#606368",
        door_fill: "#6f7d88",
        platform_fill: "#7a8288",
        hazard: BattlemapHazardStyle::ChemicalSpill,
        apron_fill: None,
        lane_markings: false,
        floor_stains: false,
        grid_ink: "#1f1b16",
        cover_props: &[
            BattlemapPropKind::WoodCrate,
            BattlemapPropKind::PalletStack,
            BattlemapPropKind::DrumCluster,
            BattlemapPropKind::Workbench,
            BattlemapPropKind::IbcTote,
        ],
    },
    BattlemapArtTheme {
        id: "clinic",
        label: "Clinic / Hospital Ward",
        floor: BattlemapFloorPattern::VinylTile,
        floor_base: "#e2e5e1",
        floor_line: "#b9bfbb",
        wall_style: BattlemapWallStyle::Poche,
        wall_fill: "#4a4f55",
        wall_hatch: "#676d73",
        door_fill: "#7b8f9e",
        platform_fill: "#9aa3a8",
        hazard: BattlemapHazardStyle::Biohazard,
        apron_fill: None,
        lane_markings: false,
        floor_stains: false,
        grid_ink: "#1f1b16",
        cover_props: &[
            BattlemapPropKind::HospitalBed,
            BattlemapPropKind::SupplyCart,
            BattlemapPropKind::StorageCabinet,
            BattlemapPropKind::OfficeDesk,
            BattlemapPropKind::Planter,
        ],
    },
    BattlemapArtTheme {
        id: "urban_street",
        label: "Urban Street",
        floor: BattlemapFloorPattern::Asphalt,
        floor_base: "#55585c",
        floor_line: "#3f4246",
        wall_style: BattlemapWallStyle::Rooftop,
        wall_fill: "#6a6661",
        wall_hatch: "#837e77",
        door_fill: "#8a6a4a",
        platform_fill: "#7c8389",
        hazard: BattlemapHazardStyle::Fire,
        apron_fill: Some("#bcb8ae"),
        lane_markings: true,
        floor_stains: false,
        grid_ink: "#f4f1ea",
        cover_props: &[
            BattlemapPropKind::JerseyBarrier,
            BattlemapPropKind::Dumpster,
            BattlemapPropKind::StreetBench,
            BattlemapPropKind::Planter,
        ],
    },
    BattlemapArtTheme {
        id: "derelict_industrial",
        label: "Derelict Industrial",
        floor: BattlemapFloorPattern::StainedConcrete,
        floor_base: "#a8a397",
        floor_line: "#7d786d",
        wall_style: BattlemapWallStyle::Poche,
        wall_fill: "#3e3c39",
        wall_hatch: "#5b5853",
        door_fill: "#6b5b4a",
        platform_fill: "#6e6a63",
        hazard: BattlemapHazardStyle::ChemicalSpill,
        apron_fill: None,
        lane_markings: false,
        floor_stains: true,
        grid_ink: "#1f1b16",
        cover_props: &[
            BattlemapPropKind::Rubble,
            BattlemapPropKind::OverturnedTable,
            BattlemapPropKind::DrumCluster,
            BattlemapPropKind::WoodCrate,
        ],
    },
    BattlemapArtTheme {
        id: "bar_interior",
        label: "Bar / Tavern Interior",
        floor: BattlemapFloorPattern::Hardwood,
        floor_base: "#8a6646",
        floor_line: "#5a3f29",
        wall_style: BattlemapWallStyle::Poche,
        wall_fill: "#33302c",
        wall_hatch: "#4f4a44",
        door_fill: "#7a4f2f",
        platform_fill: "#6e6a63",
        hazard: BattlemapHazardStyle::WaterLeak,
        apron_fill: None,
        lane_markings: false,
        floor_stains: true,
        grid_ink: "#1f1b16",
        cover_props: &[
            BattlemapPropKind::OverturnedTable,
            BattlemapPropKind::DrumCluster,
            BattlemapPropKind::WoodCrate,
        ],
    },
    BattlemapArtTheme {
        id: "park_grounds",
        label: "Park / Estate Grounds",
        floor: BattlemapFloorPattern::Grass,
        floor_base: "#7d9a5a",
        floor_line: "#5c7840",
        wall_style: BattlemapWallStyle::Poche,
        wall_fill: "#5b554e",
        wall_hatch: "#787168",
        door_fill: "#4a4e52",
        platform_fill: "#8f897e",
        hazard: BattlemapHazardStyle::Fire,
        apron_fill: None,
        lane_markings: false,
        floor_stains: false,
        grid_ink: "#1c2214",
        cover_props: &[
            BattlemapPropKind::Planter,
            BattlemapPropKind::StreetBench,
            BattlemapPropKind::WoodCrate,
            BattlemapPropKind::JerseyBarrier,
        ],
    },
];

/// Maps engine `SectorPreset` codes onto modern-day art themes until presets carry an explicit theme.
const PRESET_ART_THEME_MAP: &[(&str, &str)] = &[
    ("orbital_airlock", "industrial"),
    ("docking_bay", "industrial"),
    ("med_lab", "clinic"),
    ("urban_crossroads", "urban_street"),
    ("derelict_corridor", "derelict_industrial"),
    ("residential_apartment", "modern_interior"),
    ("office_floor", "office_interior"),
    ("corner_bar", "bar_interior"),
    ("wooded_park", "park_grounds"),
    ("garden_estate", "park_grounds"),
];

/// Resolves a theme from a `theme_preset` that is either a theme `id` or a mapped `SectorPreset` code.
/// Falls back to `modern_interior`.
#[must_use]
pub fn battlemap_theme_for_preset(code: Option<&str>) -> &'static BattlemapArtTheme {
    let code = code.unwrap_or_default().trim();
    let theme_id = PRESET_ART_THEME_MAP
        .iter()
        .find(|(p, _)| *p == code)
        .map_or(code, |(_, t)| *t);
    BATTLEMAP_ART_THEMES
        .iter()
        .find(|t| t.id == theme_id)
        .unwrap_or(&BATTLEMAP_ART_THEMES[0])
}

fn grid_dims(ui: &TacticalMapUiState) -> (i32, i32) {
    (ui.map.grid_width.max(1), ui.map.grid_height.max(1))
}

fn tile_or_none(ui: &TacticalMapUiState, x: i32, y: i32) -> Option<TacticalTileKind> {
    let (w, h) = grid_dims(ui);
    if x < 0 || y < 0 || x >= w || y >= h {
        return None;
    }
    ui.map.tiles.get((y * w + x) as usize).copied()
}

fn is_tile(ui: &TacticalMapUiState, x: i32, y: i32, kind: TacticalTileKind) -> bool {
    tile_or_none(ui, x, y) == Some(kind)
}

/// Merged boundary-edge path (`d`) around every cell where `inside(x, y)` is true. Out-of-bounds counts
/// as outside. Collinear unit edges are merged into long `H`/`V` runs for clean, inkable strokes.
#[must_use]
pub fn battlemap_boundary_path(w: i32, h: i32, inside: impl Fn(i32, i32) -> bool) -> String {
    let at = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && inside(x, y);
    let c = BATTLEMAP_CELL_PX;
    let mut d = String::new();
    for y in 0..=h {
        let mut run: Option<i32> = None;
        for x in 0..=w {
            let edge = x < w && at(x, y - 1) != at(x, y);
            match (edge, run) {
                (true, None) => run = Some(x),
                (false, Some(x0)) => {
                    let _ = write!(d, "M{} {}H{}", x0 * c, y * c, x * c);
                    run = None;
                }
                _ => {}
            }
        }
    }
    for x in 0..=w {
        let mut run: Option<i32> = None;
        for y in 0..=h {
            let edge = y < h && at(x - 1, y) != at(x, y);
            match (edge, run) {
                (true, None) => run = Some(y),
                (false, Some(y0)) => {
                    let _ = write!(d, "M{} {}V{}", x * c, y0 * c, y * c);
                    run = None;
                }
                _ => {}
            }
        }
    }
    d
}

/// Union of axis-aligned cell squares (`d`) for every cell where `inside(x, y)` is true.
pub(crate) fn battlemap_cell_rects_path(w: i32, h: i32, inside: impl Fn(i32, i32) -> bool) -> String {
    let c = BATTLEMAP_CELL_PX;
    let mut d = String::new();
    for y in 0..h {
        for x in 0..w {
            if inside(x, y) {
                let _ = write!(d, "M{} {}h{c}v{c}h-{c}z", x * c, y * c);
            }
        }
    }
    d
}

/// FNV-1a hash over everything the static art layer depends on.
#[must_use]
pub fn battlemap_art_cache_key(ui: &TacticalMapUiState) -> String {
    const fn fnv(h: u64, b: u64) -> u64 {
        (h ^ b).wrapping_mul(0x0100_0000_01B3)
    }
    let (w, h) = grid_dims(ui);
    let theme = battlemap_theme_for_preset(ui.map.theme_preset.as_deref());
    let mut acc = fnv(0xCBF2_9CE4_8422_2325, w as u64);
    acc = fnv(acc, h as u64);
    acc = fnv(acc, ui.map.seed.unwrap_or(0) as u64);
    for b in theme.id.bytes() {
        acc = fnv(acc, u64::from(b));
    }
    for t in &ui.map.tiles {
        acc = fnv(acc, *t as u64);
    }
    for e in &ui.map.wall_edges {
        for v in [e.x as u64, e.y as u64, e.side as u64, e.kind as u64, e.material as u64] {
            acc = fnv(acc, v);
        }
    }
    for p in &ui.map.props {
        for v in [p.kind as u64, p.x as u64, p.y as u64, p.w_cells as u64, p.h_cells as u64, p.rotation_deg as u64, u64::from(p.variant_seed)] {
            acc = fnv(acc, v);
        }
    }
    format!("{acc:016x}")
}

fn push_floor_pattern_def(out: &mut String, t: &BattlemapArtTheme) {
    let (base, line) = (t.floor_base, t.floor_line);
    match t.floor {
        BattlemapFloorPattern::Hardwood => {
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="128" height="64" patternUnits="userSpaceOnUse"><rect width="128" height="64" fill="{base}"/><rect y="16" width="128" height="16" fill="#000" opacity=".045"/><rect y="48" width="128" height="16" fill="#fff" opacity=".06"/><path d="M0 16H128M0 32H128M0 48H128M0 64H128M40 0V16M104 16V32M18 32V48M78 48V64" stroke="{line}" stroke-width="1.2" opacity=".55"/></pattern>"##
            );
        }
        BattlemapFloorPattern::PolishedConcrete | BattlemapFloorPattern::StainedConcrete => {
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="256" height="256" patternUnits="userSpaceOnUse"><rect width="256" height="256" fill="{base}"/><path d="M0 0H256M0 0V256" stroke="{line}" stroke-width="1.6" opacity=".6"/><circle cx="64" cy="190" r="1.4" fill="{line}" opacity=".5"/><circle cx="180" cy="70" r="1.1" fill="{line}" opacity=".5"/></pattern>"##
            );
        }
        BattlemapFloorPattern::VinylTile => {
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="128" height="128" patternUnits="userSpaceOnUse"><rect width="128" height="128" fill="{base}"/><rect x="32" y="0" width="32" height="32" fill="#000" opacity=".035"/><rect x="96" y="64" width="32" height="32" fill="#000" opacity=".035"/><rect x="0" y="96" width="32" height="32" fill="#9fb8b0" opacity=".18"/><path d="M0 0H128M0 32H128M0 64H128M0 96H128M0 0V128M32 0V128M64 0V128M96 0V128" stroke="{line}" stroke-width="1" opacity=".8"/></pattern>"##
            );
        }
        BattlemapFloorPattern::Asphalt => {
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="{base}"/><circle cx="7" cy="9" r="1" fill="#fff" opacity=".12"/><circle cx="31" cy="22" r="1.2" fill="#fff" opacity=".1"/><circle cx="18" cy="40" r=".9" fill="#000" opacity=".25"/><circle cx="42" cy="4" r="1" fill="{line}" opacity=".6"/></pattern>"##
            );
        }
        BattlemapFloorPattern::Grass => {
            let _ = write!(
                out,
                r##"<pattern id="bm-floor" width="96" height="96" patternUnits="userSpaceOnUse"><rect width="96" height="96" fill="{base}"/><rect width="96" height="48" fill="#fff" opacity=".045"/><path d="M12 18l3-5m26 19l-2-6m38-4l3-5M22 66l-3-5m35 16l3-6m24-12l-2-5" stroke="{line}" stroke-width="1.3" stroke-linecap="round" opacity=".55"/><circle cx="44" cy="14" r="1.5" fill="#98b46e" opacity=".55"/><circle cx="80" cy="78" r="1.6" fill="#638044" opacity=".5"/></pattern>"##
            );
        }
    }
}

fn push_defs(out: &mut String, t: &BattlemapArtTheme) {
    out.push_str("<defs>");
    push_floor_pattern_def(out, t);
    let _ = write!(
        out,
        r##"<pattern id="bm-hatch" width="9" height="9" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="9" height="9" fill="{wall}"/><path d="M0 0V9" stroke="{hatch}" stroke-width="2.2"/></pattern><pattern id="bm-grate" width="8" height="8" patternUnits="userSpaceOnUse"><rect width="8" height="8" fill="{plat}"/><path d="M0 4H8M4 0V8" stroke="#000" stroke-opacity=".24" stroke-width="1.2"/></pattern><pattern id="bm-paver" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#cfc7b8"/><rect x="2" y="2" width="28" height="28" rx="3" fill="#dbd4c6" opacity=".6"/><rect x="34" y="34" width="28" height="28" rx="3" fill="#c3baa9" opacity=".55"/><path d="M0 32H64M32 0V32M16 32V64M48 32V64M0 0H64M0 0V64" stroke="#938a7a" stroke-width="1.3" opacity=".65"/></pattern><pattern id="bm-tarmac" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#55585c"/><circle cx="9" cy="11" r="1" fill="#fff" opacity=".12"/><circle cx="33" cy="24" r="1.2" fill="#fff" opacity=".1"/><circle cx="19" cy="39" r=".9" fill="#000" opacity=".28"/></pattern>"##,
        wall = t.wall_fill,
        hatch = t.wall_hatch,
        plat = t.platform_fill,
    );
    out.push_str(concat!(
        r##"<filter id="bm-ink" x="-5%" y="-5%" width="110%" height="110%"><feTurbulence type="fractalNoise" baseFrequency="0.035" numOctaves="2" seed="3" result="n"/><feDisplacementMap in="SourceGraphic" in2="n" scale="3.5" xChannelSelector="R" yChannelSelector="G"/></filter>"##,
        r##"<filter id="bm-prop" x="-10%" y="-10%" width="120%" height="120%"><feTurbulence type="fractalNoise" baseFrequency="0.04" numOctaves="2" seed="7" result="n"/><feDisplacementMap in="SourceGraphic" in2="n" scale="2.5" xChannelSelector="R" yChannelSelector="G" result="d"/><feDropShadow in="d" dx="3" dy="4" stdDeviation="2.5" flood-color="#1a140c" flood-opacity=".38"/></filter>"##,
        r##"<filter id="bm-shadow" x="-10%" y="-10%" width="120%" height="120%"><feDropShadow dx="2.5" dy="3.5" stdDeviation="3" flood-color="#120d08" flood-opacity=".42"/></filter>"##,
        r##"<filter id="bm-grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.8" numOctaves="2" seed="11" stitchTiles="stitch"/><feColorMatrix values="0 0 0 0 .16 0 0 0 0 .13 0 0 0 0 .1 0 0 0 .55 -.18"/></filter>"##,
        r##"<filter id="bm-mottle" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.012" numOctaves="3" seed="5"/><feColorMatrix values="0 0 0 0 .1 0 0 0 0 .08 0 0 0 0 .05 .42 0 0 0 -.19"/></filter>"##,
        "</defs>"
    ));
}

/// Orientation (`rot_deg`) so a prop's back (`-y`) rests against an adjacent wall, else a hashed quarter-turn.
fn prop_rotation_deg(ui: &TacticalMapUiState, x: i32, y: i32, h: u64) -> u16 {
    const SIDES: [(i32, i32, u16); 4] = [(0, -1, 0), (1, 0, 90), (0, 1, 180), (-1, 0, 270)];
    SIDES
        .iter()
        .find(|(dx, dy, _)| is_tile(ui, x + dx, y + dy, TacticalTileKind::FullWall))
        .map_or(((h >> 20) % 4) as u16 * 90, |s| s.2)
}

fn push_hazard(out: &mut String, style: BattlemapHazardStyle, cx: f32, cy: f32, h: u64) {
    let layers: &[(f32, &str, &str)] = match style {
        BattlemapHazardStyle::Fire => &[
            (0.62, "#2b2622", ".35"),
            (0.42, "#e2572b", ".7"),
            (0.26, "#f6b73c", ".8"),
            (0.12, "#fff0b0", ".75"),
        ],
        BattlemapHazardStyle::ChemicalSpill => &[(0.56, "#3a4a2c", ".55"), (0.3, "#8fb04a", ".4")],
        BattlemapHazardStyle::Biohazard => &[(0.56, "#b8c43a", ".5"), (0.3, "#e4ea7a", ".5")],
        BattlemapHazardStyle::WaterLeak => &[(0.58, "#5b8fb5", ".42"), (0.3, "#a9cbe3", ".45")],
    };
    for (i, (r_frac, fill, op)) in layers.iter().enumerate() {
        let d = battlemap_blob_path(cx, cy, CELL_F * r_frac, 9, 0.28, h ^ (i as u64 * 0x9E37));
        let _ = write!(out, r#"<path d="{d}" fill="{fill}" fill-opacity="{op}"/>"#);
    }
}

fn push_door(out: &mut String, ui: &TacticalMapUiState, t: &BattlemapArtTheme, x: i32, y: i32, open: bool) {
    let wallish = |xx: i32, yy: i32| {
        matches!(
            tile_or_none(ui, xx, yy),
            Some(TacticalTileKind::FullWall | TacticalTileKind::DoorClosed | TacticalTileKind::DoorOpen)
        )
    };
    let horizontal_run = wallish(x - 1, y) || wallish(x + 1, y);
    let (px, py) = ((x * BATTLEMAP_CELL_PX) as f32, (y * BATTLEMAP_CELL_PX) as f32);
    let (cx, cy) = (px + CELL_F * 0.5, py + CELL_F * 0.5);
    let rot = if horizontal_run { 0 } else { 90 };
    let _ = write!(out, r#"<g transform="rotate({rot} {cx} {cy})">"#);
    let (x0, x1, r) = (px + 4.0, px + CELL_F - 4.0, CELL_F - 8.0);
    if open {
        let _ = write!(
            out,
            r##"<path d="M{x1} {cy}A{r} {r} 0 0 0 {x0} {}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.3" stroke-dasharray="4 3" stroke-opacity=".7"/><rect x="{}" y="{}" width="5" height="{r}" fill="{door}" stroke="{BATTLEMAP_INK}" stroke-width="1.6"/>"##,
            cy - r,
            x0 - 2.5,
            cy - r,
            door = t.door_fill,
        );
    } else {
        let _ = write!(
            out,
            r##"<rect x="{x0}" y="{}" width="{r}" height="8" fill="{door}" stroke="{BATTLEMAP_INK}" stroke-width="1.8"/><path d="M{cx} {}V{}" stroke="{BATTLEMAP_INK}" stroke-width="1" stroke-opacity=".5"/>"##,
            cy - 4.0,
            cy - 4.0,
            cy + 4.0,
            door = t.door_fill,
        );
    }
    out.push_str("</g>");
}

/// Static illustrated art layer (inner markup of `#battlemap-art`).
#[must_use]
pub fn format_battlemap_art_layer(ui: &TacticalMapUiState) -> String {
    let t = battlemap_theme_for_preset(ui.map.theme_preset.as_deref());
    let (w, h) = grid_dims(ui);
    let (vw, vh) = (w * BATTLEMAP_CELL_PX, h * BATTLEMAP_CELL_PX);
    let seed = ui.map.seed.unwrap_or(42);
    // Cells under multi-cell props are drawn by the prop glyph, not as generic walls/crates.
    let covered = |x: i32, y: i32| solid_prop_at(&ui.map.props, x, y).is_some();
    let wall = |x: i32, y: i32| is_tile(ui, x, y, TacticalTileKind::FullWall) && !covered(x, y);
    let mut out = String::with_capacity(48 * 1024);
    push_defs(&mut out, t);

    // 1. Exterior margin, floor material + large-scale tonal mottling
    let pad = BATTLEMAP_PAD_PX;
    let _ = write!(
        out,
        r##"<rect x="-{pad}" y="-{pad}" width="{}" height="{}" fill="#2b2723"/><rect width="{vw}" height="{vh}" fill="url(#bm-floor)"/><rect width="{vw}" height="{vh}" filter="url(#bm-mottle)"/>"##,
        vw + 2 * pad,
        vh + 2 * pad
    );

    // 1b. Merged paved roadways (`AsphaltDrive`), walkways/plazas (`StonePath`), and rugs
    let has_surf = |kind: PropKind, x: i32, y: i32| {
        ui.map.props.iter().any(|p| p.kind == kind && p.covers(x, y))
    };
    let tarmac_d = battlemap_cell_rects_path(w, h, |x, y| has_surf(PropKind::AsphaltDrive, x, y));
    if !tarmac_d.is_empty() {
        let curb_d = battlemap_boundary_path(w, h, |x, y| has_surf(PropKind::AsphaltDrive, x, y));
        let _ = write!(
            out,
            r##"<path d="{tarmac_d}" fill="url(#bm-tarmac)"/><path d="{curb_d}" fill="none" stroke="#c5c0b5" stroke-width="6"/><path d="{curb_d}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.8" filter="url(#bm-ink)"/>"##
        );
    }
    let stone_d = battlemap_cell_rects_path(w, h, |x, y| has_surf(PropKind::StonePath, x, y));
    if !stone_d.is_empty() {
        let border_d = battlemap_boundary_path(w, h, |x, y| has_surf(PropKind::StonePath, x, y));
        let _ = write!(
            out,
            r##"<path d="{stone_d}" fill="url(#bm-paver)"/><path d="{border_d}" fill="none" stroke="#8b8273" stroke-width="3.2" filter="url(#bm-ink)"/><path d="{border_d}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.4" filter="url(#bm-ink)"/>"##
        );
    }
    for p in ui.map.props.iter().filter(|p| {
        prop_spec(p.kind).floor_layer && !matches!(p.kind, PropKind::StonePath | PropKind::AsphaltDrive)
    }) {
        push_furniture_glyph(&mut out, p, CELL_F);
    }

    // 2. Ground dressing: stains, sidewalks/aprons, lane markings
    if t.floor_stains {
        for i in 0..((w * h) / 16).max(3) {
            let hh = battlemap_cell_hash(seed, i, 0, 0x57A1);
            let sx = battlemap_unit_f32(hh) * vw as f32;
            let sy = battlemap_unit_f32(hh ^ 0x3C) * vh as f32;
            let r = CELL_F * (0.4 + battlemap_unit_f32(hh ^ 0x91) * 0.8);
            let d = battlemap_blob_path(sx, sy, r, 8, 0.3, hh);
            let _ = write!(out, r##"<path d="{d}" fill="#5b4a36" fill-opacity=".14"/>"##);
        }
    }
    if let Some(apron) = t.apron_fill {
        let near_wall = |x: i32, y: i32| {
            !wall(x, y) && (-1..=1).any(|dy| (-1..=1).any(|dx| wall(x + dx, y + dy)))
        };
        let d = battlemap_cell_rects_path(w, h, near_wall);
        let curb = battlemap_boundary_path(w, h, |x, y| near_wall(x, y) || wall(x, y));
        let _ = write!(
            out,
            r##"<path d="{d}" fill="{apron}"/><path d="{curb}" fill="none" stroke="#8d897f" stroke-width="3"/>"##
        );
    }
    if t.lane_markings {
        let road = |x: i32, y: i32| {
            tile_or_none(ui, x, y).is_some_and(|k| !k.blocks_full_los() && k != TacticalTileKind::ElevatedPlatform)
        };
        let (my, mx) = (h / 2, w / 2);
        let mut d = String::new();
        for x in 0..w {
            if road(x, my - 1) && road(x, my) {
                let _ = write!(d, "M{} {}h{}", x * BATTLEMAP_CELL_PX, my * BATTLEMAP_CELL_PX, BATTLEMAP_CELL_PX);
            }
        }
        for y in 0..h {
            if road(mx - 1, y) && road(mx, y) && !(y == my - 1 || y == my) {
                let _ = write!(d, "M{} {}v{}", mx * BATTLEMAP_CELL_PX, y * BATTLEMAP_CELL_PX, BATTLEMAP_CELL_PX);
            }
        }
        let _ = write!(
            out,
            r##"<path d="{d}" fill="none" stroke="#efe9d2" stroke-width="3.5" stroke-dasharray="20 14" stroke-opacity=".8"/>"##
        );
    }

    // 3. Hazards (organic spills / fire)
    for y in 0..h {
        for x in 0..w {
            if is_tile(ui, x, y, TacticalTileKind::HazardZone) {
                let (cx, cy) = ((x as f32 + 0.5) * CELL_F, (y as f32 + 0.5) * CELL_F);
                push_hazard(&mut out, t.hazard, cx, cy, battlemap_cell_hash(seed, x, y, 0x4A2));
            }
        }
    }

    // 4. Raised platforms / mezzanine grating with safety-yellow rails
    let plat = |x: i32, y: i32| is_tile(ui, x, y, TacticalTileKind::ElevatedPlatform);
    let plat_d = battlemap_cell_rects_path(w, h, plat);
    if !plat_d.is_empty() {
        let rail = battlemap_boundary_path(w, h, plat);
        let _ = write!(
            out,
            r##"<path d="{plat_d}" fill="url(#bm-grate)" filter="url(#bm-shadow)"/><path d="{rail}" fill="none" stroke="#e0b43a" stroke-width="4.5"/><path d="{rail}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.4" filter="url(#bm-ink)"/>"##
        );
    }

    // 5. Walls (poché or rooftops) + hand-inked outline with a light sketch echo
    let wall_d = battlemap_cell_rects_path(w, h, wall);
    let outline = battlemap_boundary_path(w, h, wall);
    match t.wall_style {
        BattlemapWallStyle::Poche => {
            let _ = write!(out, r##"<path d="{wall_d}" fill="url(#bm-hatch)" filter="url(#bm-shadow)"/>"##);
        }
        BattlemapWallStyle::Rooftop => {
            let _ = write!(
                out,
                r##"<path d="{wall_d}" fill="{}" filter="url(#bm-shadow)"/><path d="{outline}" fill="none" stroke="{}" stroke-width="9"/>"##,
                t.wall_fill, t.wall_hatch
            );
            for y in 0..h {
                for x in 0..w {
                    let interior = wall(x, y) && wall(x - 1, y) && wall(x + 1, y) && wall(x, y - 1) && wall(x, y + 1);
                    if interior && battlemap_cell_hash(seed, x, y, 0x4FAC).is_multiple_of(3) {
                        let (px, py) = (x * BATTLEMAP_CELL_PX + 14, y * BATTLEMAP_CELL_PX + 16);
                        let _ = write!(
                            out,
                            r##"<g filter="url(#bm-prop)"><rect x="{px}" y="{py}" width="36" height="30" rx="2" fill="#9ea3a6" stroke="{BATTLEMAP_INK}" stroke-width="1.6"/><circle cx="{}" cy="{}" r="9" fill="#7d8286" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/></g>"##,
                            px + 18,
                            py + 15
                        );
                    }
                }
            }
        }
    }
    let _ = write!(
        out,
        r##"<path d="{outline}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="3" stroke-linecap="round" filter="url(#bm-ink)"/><path d="{outline}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1" stroke-opacity=".4" transform="translate(1.6 -1.2)"/>"##
    );

    // 6. Multi-cell furniture/props (footprint-scaled glyphs) under the shared wobbly-ink shadow filter
    out.push_str(r#"<g filter="url(#bm-prop)">"#);
    for p in ui.map.props.iter().filter(|p| !prop_spec(p.kind).floor_layer) {
        push_furniture_glyph(&mut out, p, CELL_F);
    }
    out.push_str("</g>");

    // 7. Thin edge walls, doors & windows (architectural floorplan strokes)
    push_edge_walls_svg(&mut out, &ui.map.wall_edges, w, h, t.wall_fill, t.door_fill, CELL_F);

    // 8. Legacy cell doors and generic cover glyphs share the wobbly ink + soft drop-shadow filter
    out.push_str(r#"<g filter="url(#bm-prop)">"#);
    for y in 0..h {
        for x in 0..w {
            match tile_or_none(ui, x, y) {
                Some(TacticalTileKind::DoorClosed) => push_door(&mut out, ui, t, x, y, false),
                Some(TacticalTileKind::DoorOpen) => push_door(&mut out, ui, t, x, y, true),
                Some(TacticalTileKind::Concealment) if !covered(x, y) => {
                    let hh = battlemap_cell_hash(seed, x, y, 0xF011);
                    let bush = make_prop(PropKind::Bush, x, y, 0, None, hh as u32);
                    push_furniture_glyph(&mut out, &bush, CELL_F);
                }
                Some(TacticalTileKind::HalfCover) if !t.cover_props.is_empty() && !covered(x, y) => {
                    let hh = battlemap_cell_hash(seed, x, y, 0xC0FE);
                    let kind = t.cover_props[(hh % t.cover_props.len() as u64) as usize];
                    let (cx, cy) = ((x as f32 + 0.5) * CELL_F, (y as f32 + 0.5) * CELL_F);
                    push_battlemap_prop(&mut out, kind, cx, cy, prop_rotation_deg(ui, x, y, hh), hh);
                }
                _ => {}
            }
        }
    }
    out.push_str("</g>");

    // 8b. Overhanging tree canopies (drawn above ground props/walls so foliage drapes naturally)
    if ui.map.props.iter().any(|p| matches!(p.kind, PropKind::LargeTree | PropKind::MediumTree)) {
        out.push_str(r#"<g filter="url(#bm-prop)">"#);
        for p in &ui.map.props {
            push_tree_canopy_glyph(&mut out, p, CELL_F);
        }
        out.push_str("</g>");
    }

    // 9. Paper grain over everything
    let _ = write!(
        out,
        r##"<rect width="{vw}" height="{vh}" filter="url(#bm-grain)" opacity=".7" style="mix-blend-mode:multiply"/>"##
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_lookup_maps_presets_and_falls_back_to_modern_interior() {
        assert_eq!(battlemap_theme_for_preset(Some("med_lab")).id, "clinic");
        assert_eq!(battlemap_theme_for_preset(Some("urban_crossroads")).id, "urban_street");
        assert_eq!(battlemap_theme_for_preset(Some("industrial")).id, "industrial");
        assert_eq!(battlemap_theme_for_preset(None).id, "modern_interior");
        assert_eq!(battlemap_theme_for_preset(Some("unknown_setting")).id, "modern_interior");
        for t in BATTLEMAP_ART_THEMES {
            assert!(!t.cover_props.is_empty(), "theme {} needs cover props", t.id);
        }
    }

    #[test]
    fn boundary_path_merges_collinear_edges() {
        // 3x1 solid strip inside a 5x3 grid -> 2 horizontal runs + 2 vertical end caps
        let d = battlemap_boundary_path(5, 3, |x, y| y == 1 && (1..=3).contains(&x));
        assert_eq!(d.matches('M').count(), 4, "{d}");
        assert!(d.contains("M64 64H256"));
        assert!(d.contains("M64 128H256"));
    }

    #[test]
    fn layered_markup_has_cached_art_and_interactive_overlay() {
        let mut ui = TacticalMapUiState::default();
        let svg = format_tactical_map_svg_markup(&ui);
        assert!(svg.starts_with(r#"<div class="battlemap-stack">"#));
        assert!(svg.contains(r#"id="battlemap-art""#) && svg.contains(r#"id="battlemap-overlay""#));
        assert!(svg.contains("window.satyaClickMapPoint(fx,fy)"));
        assert!(svg.contains("window.satyaHoverMapCell(x,y)"));
        assert!(svg.contains("url(#bm-hatch)"), "walls should render as hatched poché");
        assert_eq!(svg.matches("<svg").count(), 2);
        assert!(!svg.contains("onclick=\"window.satyaClickMapCell(0,0)\""), "no per-cell handlers");

        let key_a = battlemap_art_cache_key(&ui);
        ui.hovered_cell = Some((3, 3));
        ui.show_grid_overlay = false;
        assert_eq!(key_a, battlemap_art_cache_key(&ui), "overlay state must not bust art cache");
        let idx = ui.map.tiles.iter().position(|k| *k == TacticalTileKind::Floor).unwrap();
        ui.map.tiles[idx] = TacticalTileKind::HalfCover;
        assert_ne!(key_a, battlemap_art_cache_key(&ui));
        assert!(!format_battlemap_overlay_inner(&ui).contains("bmo-grid"));
    }

    #[test]
    fn every_theme_renders_all_tile_kinds() {
        let mut ui = TacticalMapUiState::default();
        for (i, k) in TacticalTileKind::ALL.iter().enumerate() {
            ui.map.tiles[i + 21] = *k;
        }
        for t in BATTLEMAP_ART_THEMES {
            ui.map.theme_preset = Some(t.id.to_string());
            let art = format_battlemap_art_layer(&ui);
            assert!(art.contains("url(#bm-floor)") && art.contains("url(#bm-grate)"), "{}", t.id);
            assert_eq!(art.matches("<g").count(), art.matches("</g>").count(), "{}", t.id);
        }
        assert_eq!(token_initials("Sara (Valkyrie)"), "SV");
        assert_eq!(token_initials("--"), "?");
    }

    #[test]
    fn furnished_presets_render_edge_walls_and_multi_cell_props() {
        use satya_engine::tactical_map::{SectorPreset, build_procedural_map_request};
        for preset in SectorPreset::ALL {
            let mut ui = TacticalMapUiState {
                map: build_procedural_map_request("t", preset, None, 20, 14, 42),
                ..Default::default()
            };
            ui.recompute_overlays();
            if preset != SectorPreset::WoodedPark {
                assert!(!ui.map.wall_edges.is_empty(), "{preset:?}: missing wall_edges");
            }
            assert!(!ui.map.props.is_empty(), "{preset:?}: missing props");
            let art = format_battlemap_art_layer(&ui);
            if preset != SectorPreset::WoodedPark {
                assert!(art.contains("stroke-linecap=\"square\""), "{preset:?}: merged edge-wall runs");
            }
            assert_eq!(art.matches("<g").count(), art.matches("</g>").count(), "{preset:?}");
            let key = battlemap_art_cache_key(&ui);
            if !ui.map.wall_edges.is_empty() {
                ui.map.wall_edges.pop();
                assert_ne!(key, battlemap_art_cache_key(&ui), "edge edits bust the art cache");
            }
        }
        for preset in [
            SectorPreset::ResidentialApartment,
            SectorPreset::OfficeFloor,
            SectorPreset::CornerBar,
        ] {
            let ui = TacticalMapUiState {
                map: build_procedural_map_request("t", preset, None, 20, 14, 42),
                ..Default::default()
            };
            assert!(format_battlemap_art_layer(&ui).contains("#d6ebf5"), "{preset:?}: windows");
        }
        for preset in [SectorPreset::WoodedPark, SectorPreset::GardenEstate] {
            let mut ui = TacticalMapUiState {
                map: build_procedural_map_request("t", preset, None, 20, 14, 42),
                ..Default::default()
            };
            ui.recompute_overlays();
            let art = format_battlemap_art_layer(&ui);
            assert!(art.contains("url(#bm-paver)"), "{preset:?}: merged flagstone paths");
            assert!(art.contains("url(#bm-tarmac)"), "{preset:?}: merged asphalt road/driveway");
        }
    }
}
