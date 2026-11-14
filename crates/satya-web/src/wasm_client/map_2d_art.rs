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
//! Phase 1+ renders continuous vector `wall_edges`, 360°-rotatable `props`, `theme_preset`, and `seed`.
//! [`BATTLEMAP_ART_THEMES`] is a `const` table, so other settings can add themes without touching the
//! composer. Prop glyphs live in [`crate::wasm_client::map_2d_art_props`].

pub use crate::wasm_client::map_2d_art_furniture::*;
pub use crate::wasm_client::map_2d_art_overlay::*;
pub use crate::wasm_client::map_2d_art_props::*;
pub use crate::wasm_client::map_2d_art_surfaces::*;
pub use crate::wasm_client::map_2d_art_walls::*;
pub use crate::wasm_client::map_2d_camera::*;
pub use crate::wasm_client::map_2d_tokens::*;

use satya_engine::map_dressing::prop_spec;

use crate::wasm_client::map_3d::TacticalMapUiState;
use satya_schema::PropKind;
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
    ("woodland_creek", "park_grounds"),
    ("roadside_ditch", "park_grounds"),
    ("urban_residential", "urban_street"),
    ("projects_high_density", "urban_street"),
    ("suburban_neighborhood", "park_grounds"),
    ("rural_countryside", "park_grounds"),
    ("luxury_mansion", "park_grounds"),
    ("downtown_commercial", "urban_street"),
];

pub fn battlemap_theme_for_map(map: &satya_schema::GmMapUpdateRequest) -> &'static BattlemapArtTheme {
    if map.active_floor < 0 {
        return battlemap_theme_for_preset(Some("industrial"));
    } else if map.active_floor > 0 {
        return battlemap_theme_for_preset(Some("modern_interior"));
    }
    battlemap_theme_for_preset(map.theme_preset.as_deref())
}

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

/// FNV-1a hash over everything the static vector art layer depends on (`wall_edges`, `props`, theme, ink).
#[must_use]
pub fn battlemap_art_cache_key(ui: &TacticalMapUiState) -> String {
    const fn fnv(h: u64, b: u64) -> u64 {
        (h ^ b).wrapping_mul(0x0100_0000_01B3)
    }
    let (w, h) = grid_dims(ui);
    let theme = battlemap_theme_for_map(&ui.map);
    let mut acc = fnv(0xCBF2_9CE4_8422_2325, w as u64);
    acc = fnv(acc, h as u64);
    acc = fnv(acc, ui.map.seed.unwrap_or(0) as u64);
    acc = fnv(acc, ui.map.active_floor as u64);
    for b in theme.id.bytes() {
        acc = fnv(acc, u64::from(b));
    }
    for b in ui.ink_style.as_str().bytes() {
        acc = fnv(acc, u64::from(b));
    }
    for e in &ui.map.wall_edges {
        for v in [e.x as u64, e.y as u64, e.side as u64, e.kind as u64, e.material as u64] {
            acc = fnv(acc, v);
        }
    }
    for p in &ui.map.props {
        let (cx_cm, cy_cm) = p.center_cm();
        let (sw_cm, sh_cm) = p.local_span_cm();
        for v in [
            p.kind as u64,
            cx_cm as u64,
            cy_cm as u64,
            sw_cm as u64,
            sh_cm as u64,
            p.rotation_deg as u64,
            u64::from(p.variant_seed),
        ] {
            acc = fnv(acc, v);
        }
    }
    for s in &ui.map.surfaces {
        for v in [
            s.kind as u64,
            s.shape as u64,
            s.width_cm as u64,
            s.border_style_cm as u64,
            u64::from(s.variant_seed),
        ] {
            acc = fnv(acc, v);
        }
        for pt in &s.points_cm {
            acc = fnv(acc, pt.x_cm as u64);
            acc = fnv(acc, pt.y_cm as u64);
        }
    }
    format!("{acc:016x}")
}

fn push_defs(out: &mut String, t: &BattlemapArtTheme, style: BattlemapInkStyle) {
    out.push_str("<defs>");
    push_floor_pattern_def(out, t);
    push_surface_pattern_defs(out);
    let _ = write!(
        out,
        r##"<pattern id="bm-hatch" width="9" height="9" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="9" height="9" fill="{wall}"/><path d="M0 0V9" stroke="{hatch}" stroke-width="2.2"/></pattern><pattern id="bm-grate" width="8" height="8" patternUnits="userSpaceOnUse"><rect width="8" height="8" fill="{plat}"/><path d="M0 4H8M4 0V8" stroke="#000" stroke-opacity=".24" stroke-width="1.2"/></pattern><pattern id="bm-paver" width="64" height="64" patternUnits="userSpaceOnUse"><rect width="64" height="64" fill="#cfc7b8"/><rect x="2" y="2" width="28" height="28" rx="3" fill="#dbd4c6" opacity=".6"/><rect x="34" y="34" width="28" height="28" rx="3" fill="#c3baa9" opacity=".55"/><path d="M0 32H64M32 0V32M16 32V64M48 32V64M0 0H64M0 0V64" stroke="#938a7a" stroke-width="1.3" opacity=".65"/></pattern><pattern id="bm-tarmac" width="48" height="48" patternUnits="userSpaceOnUse"><rect width="48" height="48" fill="#55585c"/><circle cx="9" cy="11" r="1" fill="#fff" opacity=".12"/><circle cx="33" cy="24" r="1.2" fill="#fff" opacity=".1"/><circle cx="19" cy="39" r=".9" fill="#000" opacity=".28"/></pattern>"##,
        wall = t.wall_fill,
        hatch = t.wall_hatch,
        plat = t.platform_fill,
    );
    push_ink_style_filters(out, style);
    out.push_str("</defs>");
}

/// Static illustrated art layer (inner markup of `#battlemap-art`).
#[must_use]
pub fn format_battlemap_art_layer(ui: &TacticalMapUiState) -> String {
    let t = battlemap_theme_for_map(&ui.map);
    let style = ui.ink_style;
    let (w, h) = grid_dims(ui);
    let (vw, vh) = (w * BATTLEMAP_CELL_PX, h * BATTLEMAP_CELL_PX);
    let seed = ui.map.seed.unwrap_or(42);
    let mut out = String::with_capacity(48 * 1024);
    push_defs(&mut out, t, style);

    // 1. Exterior margin, floor material + large-scale tonal mottling
    let pad = BATTLEMAP_PAD_PX;
    let _ = write!(
        out,
        r##"<rect x="-{pad}" y="-{pad}" width="{}" height="{}" fill="{}"/>"##,
        vw + 2 * pad,
        vh + 2 * pad,
        style.margin_fill(),
    );
    if style.uses_color_tint() {
        out.push_str(r#"<g filter="url(#bm-tint)">"#);
    }
    let _ = write!(
        out,
        r##"<rect width="{vw}" height="{vh}" fill="url(#bm-floor)"/><rect width="{vw}" height="{vh}" filter="url(#bm-mottle)"/>"##,
    );

    // 1b. Merged paved roadways, organic landscaping surfaces, rugs, catwalks, and hazard spills
    let has_vector_surfaces = !ui.map.surfaces.is_empty();
    push_landscaping_surfaces_svg(&mut out, ui, w, h, seed);
    for p in ui.map.props.iter().filter(|p| {
        prop_spec(p.kind).floor_layer
            && (if has_vector_surfaces {
                !matches!(
                    p.kind,
                    PropKind::StonePath
                        | PropKind::AsphaltDrive
                        | PropKind::GravelPath
                        | PropKind::DirtPath
                        | PropKind::MulchBed
                        | PropKind::CreekWater
                        | PropKind::DrainageDitch
                        | PropKind::GardenBed
                        | PropKind::GardenEdging
                )
            } else {
                p.rotation_deg.rem_euclid(90) != 0
                    || !matches!(
                        p.kind,
                        PropKind::StonePath
                            | PropKind::AsphaltDrive
                            | PropKind::GravelPath
                            | PropKind::DirtPath
                            | PropKind::MulchBed
                            | PropKind::CreekWater
                            | PropKind::DrainageDitch
                    )
            })
    }) {
        push_furniture_glyph(&mut out, p, CELL_F);
    }

    // 2. Ground dressing: stains and roadway centerlines (roads, sidewalks & channels protected from dirt stains)
    crate::wasm_client::map_2d_art_surfaces::push_ground_dressing_and_lanes_svg(
        &mut out, ui, t, w, h, seed,
    );

    // 3. Solid 360°-rotated furniture, vehicles, pillars & tree trunks under the shared wobbly-ink shadow filter
    out.push_str(r#"<g filter="url(#bm-prop)">"#);
    for p in ui.map.props.iter().filter(|p| !prop_spec(p.kind).floor_layer) {
        push_furniture_glyph(&mut out, p, CELL_F);
    }
    out.push_str("</g>");

    // 4. Architectural vector walls, doors & windows
    push_edge_walls_svg(&mut out, &ui.map.wall_edges, w, h, t.wall_fill, t.door_fill, CELL_F);

    // 5. Overhanging tree canopies (drawn above ground props/walls so foliage drapes naturally)
    if ui.map.props.iter().any(|p| matches!(p.kind, PropKind::LargeTree | PropKind::MediumTree)) {
        out.push_str(r#"<g filter="url(#bm-prop)">"#);
        for p in &ui.map.props {
            push_tree_canopy_glyph(&mut out, p, CELL_F);
        }
        out.push_str("</g>");
    }

    // 6. Paper grain / ink finish + optional color-matrix group close
    if style.uses_color_tint() {
        out.push_str("</g>");
    }
    push_ink_style_finish(&mut out, style, vw, vh);
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
        assert!(svg.contains(r#"preserveAspectRatio="xMidYMid meet""#));
        assert!(svg.contains("window.satyaBattlemapWheel(event.deltaY,event.clientX,event.clientY)"));
        assert!(svg.contains("window.satyaBattlemapMouseDown(event.button,event.clientX,event.clientY)"));
        assert!(svg.contains("window.satyaBattlemapMouseMove(event.buttons,event.clientX,event.clientY)"));
        assert!(svg.contains("window.satyaBattlemapMouseUp(event.button,event.clientX,event.clientY)"));
        assert!(svg.contains("window.satyaBattlemapMouseLeave()"));
        assert!(svg.ends_with("</svg></div>"));
        assert_eq!(svg.matches("<svg").count(), 2);
        assert!(!svg.contains("onclick=\"window.satyaClickMapCell(0,0)\""), "no per-cell handlers");

        let key_a = battlemap_art_cache_key(&ui);
        ui.hovered_point_m = Some((3.4, 3.6));
        ui.show_grid_overlay = false;
        assert_eq!(key_a, battlemap_art_cache_key(&ui), "overlay state must not bust art cache");
        if let Some(p) = ui.map.props.first_mut() {
            p.rotation_deg = (p.rotation_deg + 15) % 360;
        }
        assert_ne!(key_a, battlemap_art_cache_key(&ui));
        assert!(!format_battlemap_overlay_inner(&ui).contains("bmo-grid"));
    }

    #[test]
    fn every_theme_renders_vector_scene_and_props() {
        use satya_engine::map_dressing::make_prop_at_cm;
        let mut ui = TacticalMapUiState::default();
        ui.map.props.push(make_prop_at_cm(PropKind::ElevatedCatwalk, 400, 400, 0, None, 1));
        ui.map.props.push(make_prop_at_cm(PropKind::HazardSpill, 650, 450, 0, None, 2));
        for t in BATTLEMAP_ART_THEMES {
            ui.map.theme_preset = Some(t.id.to_string());
            let art = format_battlemap_art_layer(&ui);
            assert!(art.contains("url(#bm-floor)"), "{}", t.id);
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
            if preset.has_structural_wall_edges() {
                assert!(!ui.map.wall_edges.is_empty(), "{preset:?}: missing wall_edges");
            }
            assert!(!ui.map.props.is_empty(), "{preset:?}: missing props");
            let art = format_battlemap_art_layer(&ui);
            if preset.has_structural_wall_edges() {
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
        let creek_ui = TacticalMapUiState {
            map: build_procedural_map_request("tc", SectorPreset::WoodlandCreek, None, 20, 14, 42),
            ..Default::default()
        };
        let creek_art = format_battlemap_art_layer(&creek_ui);
        assert!(creek_art.contains("url(#bm-creek)") && creek_art.contains("url(#bm-leaf-tex)"));
        let ditch_ui = TacticalMapUiState {
            map: build_procedural_map_request("td", SectorPreset::RoadsideDitch, None, 20, 14, 42),
            ..Default::default()
        };
        let ditch_art = format_battlemap_art_layer(&ditch_ui);
        assert!(ditch_art.contains("url(#bm-ditch)") && ditch_art.contains("url(#bm-gravel)"));
    }

    #[test]
    fn phase4_ink_styles_portrait_discs_and_prop_hover_preview() {
        use satya_engine::tactical_map::{SectorPreset, build_procedural_map_request};

        let mut ui = TacticalMapUiState {
            map: build_procedural_map_request("p4", SectorPreset::ResidentialApartment, None, 20, 14, 42),
            ..Default::default()
        };
        let key_vellum = battlemap_art_cache_key(&ui);
        for style in BattlemapInkStyle::ALL {
            ui.ink_style = style;
            let art = format_battlemap_art_layer(&ui);
            assert_eq!(art.matches("<g").count(), art.matches("</g>").count(), "{style:?}: balanced <g>");
            if style != BattlemapInkStyle::ModernVellum {
                assert_ne!(battlemap_art_cache_key(&ui), key_vellum, "{style:?} should bust cache");
            }
            if style.uses_color_tint() {
                assert!(art.contains(r#"id="bm-tint""#) && art.contains(r#"filter="url(#bm-tint)""#));
            }
        }
        ui.ink_style = BattlemapInkStyle::CleanVector;
        assert!(format_battlemap_art_layer(&ui).contains(r#"<feOffset dx="0" dy="0"/>"#));

        // Token portrait discs: custom allowlisted portrait URL vs unsafe rejected + preset silhouettes
        if let Some(vis) = ui.map.token_visuals.first_mut() {
            vis.portrait_url = Some("/api/characters/sara/assets/portrait".to_string());
        }
        if let Some(vis) = ui.map.token_visuals.get_mut(1) {
            vis.portrait_url = Some("javascript:alert(1)".to_string());
        }
        let overlay = format_battlemap_overlay_inner(&ui);
        assert!(overlay.contains(r#"<clipPath id="bm-tok-clip-0""#));
        assert!(overlay.contains(r#"href="/api/characters/sara/assets/portrait""#));
        assert!(!overlay.contains("javascript:alert(1)"));

        // Prop brush continuous hover preview
        ui.interaction_mode = "edit_terrain".to_string();
        ui.edit_prop_brush = "BAR_COUNTER".to_string();
        ui.hovered_point_m = Some((5.5, 5.5));
        let brush_ov = format_battlemap_overlay_inner(&ui);
        assert!(brush_ov.contains(r#"stroke-dasharray="7 4""#));
    }
}
