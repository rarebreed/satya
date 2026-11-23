//! Tactical `1.0m` Grid Terrain Tiles, Cover Grades, 3D Miniature Visual Specs,
//! and Interactive Map Generation / Token Movement REST DTOs.
//!
//! Edge walls, multi-cell props, vector surfaces, multi-floor DTOs, world-sector grids, and material classes live in
//! [`crate::tactical_map_dressing_dto`], [`crate::tactical_map_floors_dto`],
//! [`crate::tactical_map_surfaces_dto`], and [`crate::tactical_map_world_dto`].

pub use crate::tactical_map_dressing_dto::*;
pub use crate::tactical_map_floors_dto::*;
pub use crate::tactical_map_surfaces_dto::*;
pub use crate::tactical_map_world_dto::*;

use crate::events::MessageVisibility;
use crate::protocol_map_ai::GmMapUpdateRequest;
use serde::{Deserialize, Serialize};

/// Default metric tactical grid cell scale in centimeters (`100 cm = 1.00 m`).
pub const fn default_cell_size_cm() -> i32 {
    100
}

fn default_map_width() -> i32 {
    20
}

fn default_map_height() -> i32 {
    14
}

fn default_true() -> bool {
    true
}

crate::define_wire_enum! {
    /// Terrain, cover, concealment, obstacle, and elevation classification for a `1.0m` cell on a Satya tactical map.
    #[derive(Default)]
    pub enum TacticalTileKind [count = 8] {
        #[default]
        Floor => ("FLOOR", "Open Deck (0.0m)"),
        Concealment => ("CONCEALMENT", "Foliage / Concealment (1.2m, +3 Δ_ext, 0 PR)", aliases: ["FOLIAGE", "HEDGE", "BUSH", "SMOKE"]),
        HalfCover => ("HALF_COVER", "Half Cover Crate/Console (0.9m, +4 Δ_ext)", aliases: ["CRATE", "LOW_COVER", "COVER"]),
        FullWall => ("FULL_WALL", "Full Bulkhead/Pillar (2.4m, Blocks LoS)", aliases: ["WALL", "BULKHEAD", "PILLAR"]),
        DoorClosed => ("DOOR_CLOSED", "Closed Airlock Door (2.4m, Blocks LoS)", aliases: ["DOOR", "AIRLOCK_CLOSED"]),
        DoorOpen => ("DOOR_OPEN", "Open Airlock Threshold (0.0m)", aliases: ["AIRLOCK_OPEN"]),
        HazardZone => ("HAZARD_ZONE", "Environmental Hazard (×2 Move Cost)", aliases: ["HAZARD", "DIFFICULT"]),
        ElevatedPlatform => ("ELEVATED_PLATFORM", "Elevated Catwalk (+1.5m)", aliases: ["ELEVATED", "PLATFORM", "CATWALK"]),
    }
}

impl TacticalTileKind {
    /// Parses a legacy/loose wire string into [`TacticalTileKind`], defaulting unknown tokens to `Floor`.
    #[must_use]
    pub fn from_code_str_or_floor(raw: &str) -> Self {
        Self::from_code_str(raw).unwrap_or(Self::Floor)
    }

    /// Physical obstacle or deck elevation height in centimeters (`0`, `90`, `120`, `150`, or `240`).
    #[must_use]
    pub const fn height_cm(self) -> i32 {
        match self {
            Self::Floor | Self::DoorOpen | Self::HazardZone => 0,
            Self::HalfCover => 90,
            Self::Concealment => 120,
            Self::ElevatedPlatform => 150,
            Self::FullWall | Self::DoorClosed => 240,
        }
    }

    /// Physical obstacle or deck elevation height in meters (`0.0`, `0.9`, `1.2`, `1.5`, or `2.4`).
    #[must_use]
    pub fn height_m(self) -> f32 {
        self.height_cm() as f32 / 100.0
    }

    /// Whether a standard combatant cannot end their movement inside this cell.
    #[must_use]
    pub const fn blocks_occupancy(self) -> bool {
        matches!(self, Self::FullWall | Self::DoorClosed | Self::HalfCover)
    }

    /// Whether this cell completely blocks normal horizontal Line of Sight across interior rays.
    #[must_use]
    pub const fn blocks_full_los(self) -> bool {
        matches!(self, Self::FullWall | Self::DoorClosed)
    }

    /// Whether this cell obscures visual sightlines (`+3 Δ_ext` Concealment) without providing ballistic protection (`0 PR`).
    #[must_use]
    pub const fn provides_concealment(self) -> bool {
        matches!(self, Self::Concealment)
    }

    /// Whether an intervening ray crossing this cell imposes Low/Half Cover (`+4 Δ_ext`).
    #[must_use]
    pub const fn provides_half_cover(self) -> bool {
        matches!(self, Self::HalfCover)
    }

    /// Movement cost multiplier (`1.0` normal deck, `2.0` brush/hazard/difficult or vaulting).
    #[must_use]
    pub const fn move_cost_multiplier(self) -> f64 {
        match self {
            Self::HazardZone | Self::Concealment => 2.0,
            Self::ElevatedPlatform => 1.25,
            _ => 1.0,
        }
    }
}

crate::define_wire_enum! {
    /// Evaluated Line-of-Sight, visual concealment, and tactical cover classification between two positions.
    #[derive(Default)]
    pub enum CoverGrade [count = 5] {
        #[default]
        Clear => ("CLEAR", "Clear LoS (+0 Δ_ext)"),
        Concealment => ("CONCEALMENT", "Concealment (+3 Δ_ext, 0 PR)"),
        LowCover => ("LOW_COVER", "Low Cover (+4 Δ_ext)"),
        HighCover => ("HIGH_COVER", "High / Corner Cover (+6 Δ_ext)"),
        Blocked => ("BLOCKED", "Blocked (No LoS)"),
    }
}

impl CoverGrade {
    /// Canonical Satya Ranged External Modifier (`Δ_ext`) imposed by this sight/cover state:
    /// `+0` Clear, `+3` Concealment (visual obscurement only, `0 PR`), `+4` Low/Half Cover (`0.9m`),
    /// `+6` High/Corner Cover (`2.4m` edge peek), or `+16` Blocked.
    #[must_use]
    pub const fn recommended_delta_ext(self) -> i32 {
        match self {
            Self::Clear => 0,
            Self::Concealment => 3,
            Self::LowCover => 4,
            Self::HighCover => 6,
            Self::Blocked => 16,
        }
    }
}

/// 3D miniature model, color, facing angle, and pose metadata associated with a [`crate::events::TokenPlacement`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenVisualSpec {
    pub entity_id: String,
    /// Built-in low-poly `.glb` preset (`"valkyrie_rig"`, `"recon_mk4"`, `"secops_sentry"`, `"operative_light"`) or `"custom_glb"`.
    #[serde(default = "default_token_model_preset")]
    pub model_preset: String,
    /// Primary armor/faction hex color (`"#38bdf8"` player cyan, `"#f87171"` hostile red, `"#fbbf24"` neutral amber).
    #[serde(default = "default_token_color_hex")]
    pub color_hex: String,
    /// Yaw orientation in degrees (`0 = North/-Z`, `90 = East/+X`, `180 = South/+Z`, `270 = West/-X`).
    #[serde(default)]
    pub facing_deg: i32,
    /// Base elevation offset in centimeters (`0` floor, `150` elevated catwalk).
    #[serde(default)]
    pub elevation_cm: i32,
    /// Miniature pose tag (`"idle"`, `"aimed"`, `"crouched"`, `"prone"`).
    #[serde(default = "default_token_pose")]
    pub stance_pose: String,
    /// Optional safe portrait image URL/path rendered inside the 2D battlemap token disc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portrait_url: Option<String>,
    /// Authoritative continuous X center coordinate in centimeters (`None` falls back to `(tok.x * cell_size_cm) + cell_size_cm / 2`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos_x_cm: Option<i32>,
    /// Authoritative continuous Y center coordinate in centimeters (`None` falls back to `(tok.y * cell_size_cm) + cell_size_cm / 2`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos_y_cm: Option<i32>,
    /// Signed architectural floor index occupied by this miniature (`0` = `1F`, `-1` = `B1`, `1` = `2F`).
    #[serde(default)]
    pub floor_index: i32,
}

/// Validates and normalizes an optional character/NPC token portrait URL, rejecting script vectors
/// (`javascript:`, `data:image/svg+xml`, control chars, quotes, and protocol-relative `//` URLs).
#[must_use]
pub fn sanitize_portrait_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 4096 {
        return None;
    }
    if trimmed.bytes().any(|b| {
        b.is_ascii_control() || b.is_ascii_whitespace() || matches!(b, b'<' | b'>' | b'"' | b'\'')
    }) {
        return None;
    }
    let low = trimmed.to_ascii_lowercase();
    let is_safe_same_origin = (trimmed.starts_with("/api/characters/")
        || trimmed.starts_with("/maps/")
        || trimmed.starts_with("/pkg/")
        || trimmed.starts_with("/assets/"))
        && !trimmed.starts_with("//")
        && !trimmed.contains("..");
    let is_safe_https = low.starts_with("https://") && trimmed.len() > "https://".len();
    let is_safe_raster_data = low.starts_with("data:image/png;base64,")
        || low.starts_with("data:image/jpeg;base64,")
        || low.starts_with("data:image/webp;base64,");
    (is_safe_same_origin || is_safe_https || is_safe_raster_data).then(|| trimmed.to_string())
}

fn default_token_model_preset() -> String {
    "operative_light".to_string()
}

fn default_token_color_hex() -> String {
    "#38bdf8".to_string()
}

fn default_token_pose() -> String {
    "idle".to_string()
}

impl Default for TokenVisualSpec {
    fn default() -> Self {
        Self {
            entity_id: String::new(),
            model_preset: default_token_model_preset(),
            color_hex: default_token_color_hex(),
            facing_deg: 0,
            elevation_cm: 0,
            stance_pose: default_token_pose(),
            portrait_url: None,
            pos_x_cm: None,
            pos_y_cm: None,
            floor_index: 0,
        }
    }
}

crate::define_wire_enum! {
    /// Selectable procedural tactical battlemap arena scale tier.
    #[derive(Default)]
    pub enum MapScalePreset [count = 5] {
        #[default]
        Auto => ("AUTO", "Auto (Sector Default)", aliases: ["DEFAULT"]),
        Compact => ("COMPACT", "Compact (24m × 18m)", aliases: ["SMALL", "24X18", "24_18"]),
        Standard => ("STANDARD", "Standard (36m × 26m)", aliases: ["MEDIUM", "36X26", "36_26"]),
        Large => ("LARGE", "Large (48m × 36m)", aliases: ["48X36", "48_36"]),
        Massive => ("MASSIVE", "Massive (64m × 48m)", aliases: ["XL", "HUGE", "64X48", "64_48"]),
    }
}

impl MapScalePreset {
    /// Fixed grid dimensions `(width_m, height_m)` for explicit scale tiers, or `None` for `Auto`.
    #[must_use]
    pub const fn fixed_dimensions(self) -> Option<(i32, i32)> {
        match self {
            Self::Auto => None,
            Self::Compact => Some((24, 18)),
            Self::Standard => Some((36, 26)),
            Self::Large => Some((48, 36)),
            Self::Massive => Some((64, 48)),
        }
    }

    /// Resolves `(width_m, height_m)` using `preset_default` when `self == MapScalePreset::Auto`.
    #[must_use]
    pub const fn resolve_dimensions(self, preset_default: (i32, i32)) -> (i32, i32) {
        match self.fixed_dimensions() {
            Some(dims) => dims,
            None => preset_default,
        }
    }

    /// Loose case-insensitive parser accepting UI strings, aliases, or dimension tokens (`"36x26"`),
    /// defaulting to [`Self::Auto`].
    #[must_use]
    pub fn parse_loose(raw: &str) -> Self {
        Self::from_code_str(raw).unwrap_or_default()
    }
}

/// HTTP request body for `POST /api/sessions/:session_id/map` generating a procedural tactical sector
/// or editing walls, doors, windows, and continuous props on the active session map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateSessionMapRequest {
    /// Sector generator preset (`"orbital_airlock"`, `"docking_bay"`, `"med_lab"`, `"urban_crossroads"`, `"derelict_corridor"`).
    #[serde(default = "default_sector_preset")]
    pub preset: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_map_width")]
    pub grid_width: i32,
    #[serde(default = "default_map_height")]
    pub grid_height: i32,
    /// Optional arena scale tier (`Auto`, `Compact 24×18`, `Standard 36×26`, `Large 48×36`, `Massive 64×48`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<MapScalePreset>,
    #[serde(default = "default_cell_size_cm")]
    pub cell_size_cm: i32,
    #[serde(default)]
    pub seed: Option<i64>,
    #[serde(default = "default_true")]
    pub populate_roster_tokens: bool,
    /// Optional single-cell mutation X coordinate (`0..grid_width`, legacy door toggle fallback).
    #[serde(default)]
    pub edit_cell_x: Option<i32>,
    /// Optional single-cell mutation Y coordinate (`0..grid_height`, legacy door toggle fallback).
    #[serde(default)]
    pub edit_cell_y: Option<i32>,
    /// Legacy tile override field retained for wire compatibility.
    #[serde(default)]
    pub set_tile_kind: Option<TacticalTileKind>,
    /// Optional single **edge** mutation on the existing active map (door toggle or GM wall painting).
    #[serde(default)]
    pub edit_edge: Option<MapEdgeRef>,
    /// Target [`WallEdgeKind`] for `edit_edge` (`None` toggles a door open/closed).
    #[serde(default)]
    pub set_edge_kind: Option<WallEdgeKind>,
    /// Optional [`MaterialClass`] override when painting an edge with `edit_edge`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_edge_material: Option<MaterialClass>,
    /// When `true` with `edit_edge`, removes the edge entirely (GM only).
    #[serde(default)]
    pub clear_edge: bool,
    /// Optional batch wall/door/window edge upserts from Editor drag strokes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub batch_edges: Vec<MapWallEdge>,
    /// Optional batch edge deletions from Editor drag strokes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clear_edges: Vec<MapEdgeRef>,
    /// Optional GM continuous prop placement/replacement on the existing active map.
    #[serde(default)]
    pub edit_prop: Option<MapPropPlacement>,
    /// Optional in-place prop update `(index, placement)` for Editor move/rotate/stretch/tint/decorative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_prop_at_index: Option<(usize, MapPropPlacement)>,
    /// Optional prop removal by index in `map.props`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove_prop_index: Option<usize>,
    /// Optional GM prop removal covering integer cell `(x, y)` on the existing active map.
    #[serde(default)]
    pub clear_prop_at: Option<(i32, i32)>,
    /// Optional GM prop removal at continuous centimeter point `(x_cm, y_cm)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_prop_at_cm: Option<(i32, i32)>,
    /// Optional Editor addition of a vector ground surface feature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add_surface: Option<MapSurfaceFeature>,
    /// Optional in-place Editor update `(index, surface)` on `map.surfaces`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_surface_at_index: Option<(usize, MapSurfaceFeature)>,
    /// Optional Editor removal of a vector ground surface feature by index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove_surface_index: Option<usize>,
    /// Optional Editor base ground pattern override (`Some("")` clears override).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_ground_floor_override: Option<String>,
    /// Optional Editor base ground hex tint override (`Some("")` clears tint).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_ground_tint_hex: Option<String>,
    /// Optional atomic active-floor geometry snapshot replacement for Undo/Redo synchronization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace_floor_geometry: Option<MapFloorLevel>,
    /// Optional cardinal boundary (`NORTH`, `SOUTH`, `EAST`, `WEST`) to seamlessly expand and stitch
    /// onto the active tactical map using edge-matched procedural generation (GM or Solo AI-GM only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_direction: Option<MapContinuationDir>,
    /// Switches the active floor level (`-1` = `B1`, `0` = `1F`, `1` = `2F`) on the existing session map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_active_floor: Option<i32>,
    /// Optional residential architectural economic tier override for composable home presets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub economic_tier: Option<EconomicTier>,
    /// Optional macro zoning wrapper override for composable building presets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoning: Option<ZoningContext>,
    /// Optional complete battlemap payload to load as the active session map (GM or Solo Host only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_map: Option<Box<GmMapUpdateRequest>>,
}

crate::define_wire_enum! {
    /// Cardinal boundary direction across which procedural map continuation expands.
    #[derive(Default)]
    pub enum MapContinuationDir [count = 4] {
        #[default]
        North => ("NORTH", "North"),
        South => ("SOUTH", "South"),
        East => ("EAST", "East"),
        West => ("WEST", "West"),
    }
}

impl MapContinuationDir {
    /// Opposite cardinal direction (`North <-> South`, `East <-> West`).
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::South => Self::North,
            Self::East => Self::West,
            Self::West => Self::East,
        }
    }

    /// Signed 2D cell-step unit vector `(dx, dy)` in screen/map coordinates (`+Y` South, `+X` East).
    #[must_use]
    pub const fn unit_step(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::South => (0, 1),
            Self::East => (1, 0),
            Self::West => (-1, 0),
        }
    }
}

fn default_sector_preset() -> String {
    "orbital_airlock".to_string()
}

impl Default for GenerateSessionMapRequest {
    fn default() -> Self {
        Self {
            preset: default_sector_preset(),
            title: None,
            grid_width: default_map_width(),
            grid_height: default_map_height(),
            scale: None,
            cell_size_cm: default_cell_size_cm(),
            seed: None,
            populate_roster_tokens: true,
            edit_cell_x: None,
            edit_cell_y: None,
            set_tile_kind: None,
            edit_edge: None,
            set_edge_kind: None,
            set_edge_material: None,
            clear_edge: false,
            batch_edges: Vec::new(),
            clear_edges: Vec::new(),
            edit_prop: None,
            update_prop_at_index: None,
            remove_prop_index: None,
            clear_prop_at: None,
            clear_prop_at_cm: None,
            add_surface: None,
            update_surface_at_index: None,
            remove_surface_index: None,
            set_ground_floor_override: None,
            set_ground_tint_hex: None,
            replace_floor_geometry: None,
            continue_direction: None,
            set_active_floor: None,
            economic_tier: None,
            zoning: None,
            load_map: None,
        }
    }
}

/// HTTP request body for `POST /api/sessions/:session_id/map/move` moving, orienting, spawning,
/// or removing a character/NPC miniature continuously on the active tactical map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MoveMapTokenRequest {
    pub entity_id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub to_x: i32,
    #[serde(default)]
    pub to_y: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_x_cm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_y_cm: Option<i32>,
    /// Optional target floor index when traversing a stairwell/elevator across floors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_floor: Option<i32>,
    #[serde(default)]
    pub facing_deg: Option<i32>,
    #[serde(default)]
    pub model_preset: Option<String>,
    #[serde(default)]
    pub color_hex: Option<String>,
    #[serde(default)]
    pub stance_pose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portrait_url: Option<String>,
    #[serde(default)]
    pub visibility: Option<MessageVisibility>,
    #[serde(default)]
    pub remove: bool,
}

impl MoveMapTokenRequest {
    /// Returns requested destination in centimeters, preferring `to_x_cm`/`to_y_cm` over cell center `(to_x, to_y)`.
    #[must_use]
    pub fn target_cm(&self, cell_size_cm: i32) -> (i32, i32) {
        let s = cell_size_cm.max(25);
        (
            self.to_x_cm.unwrap_or(self.to_x * s + s / 2),
            self.to_y_cm.unwrap_or(self.to_y * s + s / 2),
        )
    }
}

/// Format version for portable `.satyamap` JSON bundles.
pub const SAVED_MAP_FORMAT_VERSION: u16 = 1;

const fn default_saved_map_format_version() -> u16 {
    SAVED_MAP_FORMAT_VERSION
}

/// Portable `.satyamap` JSON bundle capturing a saved battlemap, all floors, props, walls,
/// vector surfaces, and tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedMapBundle {
    /// Snapshot schema format version (`1`).
    #[serde(default = "default_saved_map_format_version")]
    pub format_version: u16,
    /// Save/export timestamp in UTC microseconds since Unix epoch.
    #[serde(default)]
    pub saved_at_us: i64,
    /// Complete tactical battlemap payload.
    pub map: GmMapUpdateRequest,
}

impl SavedMapBundle {
    /// Parses either a wrapped [`SavedMapBundle`] or a bare [`GmMapUpdateRequest`] JSON payload.
    pub fn parse_json_bytes(bytes: &[u8]) -> Result<GmMapUpdateRequest, String> {
        if let Ok(bundle) = serde_json::from_slice::<Self>(bytes) {
            return Ok(bundle.map);
        }
        serde_json::from_slice::<GmMapUpdateRequest>(bytes)
            .map_err(|err| format!("Invalid .satyamap or battlemap JSON: {err}"))
    }
}

/// Lightweight metadata summary of a saved battlemap in the Map Vault or browser storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedMapSummary {
    /// Unique identifier for the saved battlemap.
    pub map_id: String,
    /// Human-readable battlemap title.
    pub title: String,
    /// Grid width in cells (`1 cell = 1.0m` default).
    pub grid_width: i32,
    /// Grid height in cells (`1 cell = 1.0m` default).
    pub grid_height: i32,
    /// Physical cell edge scale in centimeters (`100 cm = 1.00 m`).
    #[serde(default = "default_cell_size_cm")]
    pub cell_size_cm: i32,
    /// Optional procedural sector preset identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_preset: Option<String>,
    /// Number of architectural floor levels (`>= 1`).
    #[serde(default)]
    pub floor_count: usize,
    /// Total placed props across all floors.
    #[serde(default)]
    pub prop_count: usize,
    /// Total wall/door/window edges across all floors.
    #[serde(default)]
    pub wall_count: usize,
    /// Total vector ground surface zones across all floors.
    #[serde(default)]
    pub surface_count: usize,
    /// Creation timestamp in UTC microseconds since Unix epoch.
    #[serde(default)]
    pub created_at_us: i64,
    /// Last updated timestamp in UTC microseconds since Unix epoch.
    pub updated_at_us: i64,
}

impl SavedMapSummary {
    /// Builds a [`SavedMapSummary`] from a [`GmMapUpdateRequest`] and timestamps.
    #[must_use]
    pub fn from_map(map: &GmMapUpdateRequest, created_at_us: i64, updated_at_us: i64) -> Self {
        let floor_count = map.floors.len().max(1);
        let (prop_count, wall_count, surface_count) = if map.floors.is_empty() {
            (map.props.len(), map.wall_edges.len(), map.surfaces.len())
        } else {
            map.floors.iter().fold((0, 0, 0), |(p, w, s), fl| {
                if fl.floor_index == map.active_floor {
                    (
                        p + map.props.len(),
                        w + map.wall_edges.len(),
                        s + map.surfaces.len(),
                    )
                } else {
                    (
                        p + fl.props.len(),
                        w + fl.wall_edges.len(),
                        s + fl.surfaces.len(),
                    )
                }
            })
        };
        Self {
            map_id: map.map_id.clone(),
            title: map.title.clone(),
            grid_width: map.grid_width,
            grid_height: map.grid_height,
            cell_size_cm: map.cell_size_cm.max(25),
            theme_preset: map.theme_preset.clone(),
            floor_count,
            prop_count,
            wall_count,
            surface_count,
            created_at_us,
            updated_at_us,
        }
    }
}

/// HTTP request body for `POST /api/maps` saving or updating a battlemap in the user's Map Vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveMapRequest {
    /// Complete battlemap state to persist.
    pub map: GmMapUpdateRequest,
    /// When `true`, forces minting a fresh `map_id` instead of overwriting an existing saved map.
    #[serde(default)]
    pub save_as_new: bool,
}

/// HTTP response body for `POST /api/maps`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveMapResponse {
    /// Summary metadata for the saved battlemap.
    pub summary: SavedMapSummary,
    /// Canonical sanitized battlemap stored on the server (with resolved `map_id` and `title`).
    pub map: GmMapUpdateRequest,
    /// Local filesystem mirror path under `~/Satya/maps/*.satyamap`, if written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_file_path: Option<String>,
}

/// HTTP response body for `GET /api/maps`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedMapListResponse {
    /// Saved battlemaps owned by the authenticated user, ordered by `updated_at_us` descending.
    pub maps: Vec<SavedMapSummary>,
    /// Local directory where `.satyamap` backups are mirrored (`~/Satya/maps`), if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_directory: Option<String>,
}

/// HTTP response body for `GET /api/maps/:map_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedMapRecordResponse {
    /// Summary metadata for the saved battlemap.
    pub summary: SavedMapSummary,
    /// Complete saved battlemap payload ready to load into an active session or offline viewport.
    pub map: GmMapUpdateRequest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_portrait_url_allows_safe_origins_and_rejects_scripts() {
        assert_eq!(
            sanitize_portrait_url(" /api/characters/sara/assets/p1 "),
            Some("/api/characters/sara/assets/p1".to_string())
        );
        assert_eq!(
            sanitize_portrait_url("https://cdn.example.com/portraits/sara.webp"),
            Some("https://cdn.example.com/portraits/sara.webp".to_string())
        );
        assert_eq!(
            sanitize_portrait_url("data:image/png;base64,iVBORw0KGgo="),
            Some("data:image/png;base64,iVBORw0KGgo=".to_string())
        );
        for bad in [
            "",
            "javascript:alert(1)",
            "data:image/svg+xml;base64,PHN2Zz4=",
            "//evil.example.com/x.png",
            "/api/characters/../etc/passwd",
            "https://cdn.example.com/a\"b.png",
        ] {
            assert_eq!(
                sanitize_portrait_url(bad),
                None,
                "expected reject for {bad}"
            );
        }
    }

    #[test]
    fn map_continuation_dir_and_request_roundtrip() {
        assert_eq!(
            MapContinuationDir::North.opposite(),
            MapContinuationDir::South
        );
        assert_eq!(
            MapContinuationDir::South.opposite(),
            MapContinuationDir::North
        );
        assert_eq!(
            MapContinuationDir::East.opposite(),
            MapContinuationDir::West
        );
        assert_eq!(
            MapContinuationDir::West.opposite(),
            MapContinuationDir::East
        );
        assert_eq!(MapContinuationDir::East.unit_step(), (1, 0));
        assert_eq!(MapContinuationDir::West.label(), "West");
        assert_eq!(
            MapContinuationDir::from_code_str("south"),
            Some(MapContinuationDir::South)
        );

        let req = GenerateSessionMapRequest {
            continue_direction: Some(MapContinuationDir::East),
            seed: Some(777),
            ..Default::default()
        };
        let encoded = serde_json::to_string(&req).unwrap();
        assert!(encoded.contains(r#""continue_direction":"EAST""#));
        let decoded: GenerateSessionMapRequest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, req);

        // Omitted continue_direction and scale default to None cleanly for older clients.
        let legacy: GenerateSessionMapRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(legacy.continue_direction, None);
        assert_eq!(legacy.scale, None);
        assert_eq!((legacy.grid_width, legacy.grid_height), (20, 14));

        // MapScalePreset roundtrip & resolution
        assert_eq!(MapScalePreset::Auto.resolve_dimensions((36, 26)), (36, 26));
        assert_eq!(
            MapScalePreset::Compact.resolve_dimensions((36, 26)),
            (24, 18)
        );
        assert_eq!(
            MapScalePreset::Standard.resolve_dimensions((48, 36)),
            (36, 26)
        );
        assert_eq!(MapScalePreset::Large.resolve_dimensions((24, 18)), (48, 36));
        assert_eq!(
            MapScalePreset::Massive.resolve_dimensions((24, 18)),
            (64, 48)
        );
        assert_eq!(
            MapScalePreset::parse_loose("standard"),
            MapScalePreset::Standard
        );
        assert_eq!(
            MapScalePreset::parse_loose("64x48"),
            MapScalePreset::Massive
        );
        assert_eq!(MapScalePreset::parse_loose("unknown"), MapScalePreset::Auto);

        let scaled_req = GenerateSessionMapRequest {
            scale: Some(MapScalePreset::Massive),
            ..Default::default()
        };
        let scaled_json = serde_json::to_string(&scaled_req).unwrap();
        assert!(scaled_json.contains(r#""scale":"MASSIVE""#));
        let scaled_dec: GenerateSessionMapRequest = serde_json::from_str(&scaled_json).unwrap();
        assert_eq!(scaled_dec.scale, Some(MapScalePreset::Massive));
    }

    #[test]
    fn saved_map_bundle_summary_and_load_map_roundtrip() {
        use crate::tactical_map_dressing_dto::{EdgeSide, MapWallEdge, MaterialClass, WallEdgeKind};
        use crate::tactical_map_floors_dto::MapFloorLevel;

        let mut map = GmMapUpdateRequest {
            map_id: "map_vault_1".to_string(),
            title: "Dockyard Warehouse".to_string(),
            grid_width: 24,
            grid_height: 18,
            wall_edges: vec![MapWallEdge {
                x: 2,
                y: 3,
                side: EdgeSide::North,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::ConcreteBlock,
            }],
            ..Default::default()
        };

        // Single-floor summary
        let single_sum = SavedMapSummary::from_map(&map, 100, 200);
        assert_eq!(single_sum.floor_count, 1);
        assert_eq!(single_sum.wall_count, 1);
        assert_eq!(single_sum.prop_count, 0);
        assert_eq!(single_sum.surface_count, 0);
        assert_eq!(single_sum.created_at_us, 100);
        assert_eq!(single_sum.updated_at_us, 200);

        // Multi-floor summary sums across all floors
        map.floors = vec![
            MapFloorLevel {
                floor_index: 0,
                label: "1F".to_string(),
                elevation_cm: 0,
                wall_edges: map.wall_edges.clone(),
                props: Vec::new(),
                surfaces: Vec::new(),
                ground_floor_override: None,
                ground_tint_rgb: None,
            },
            MapFloorLevel {
                floor_index: 1,
                label: "2F".to_string(),
                elevation_cm: 300,
                wall_edges: vec![
                    MapWallEdge {
                        x: 4,
                        y: 5,
                        side: EdgeSide::West,
                        kind: WallEdgeKind::DoorClosed,
                        material: MaterialClass::SheetSteel,
                    },
                    MapWallEdge {
                        x: 4,
                        y: 6,
                        side: EdgeSide::West,
                        kind: WallEdgeKind::Window,
                        material: MaterialClass::Glass,
                    },
                ],
                props: Vec::new(),
                surfaces: Vec::new(),
                ground_floor_override: None,
                ground_tint_rgb: None,
            },
        ];
        let multi_sum = SavedMapSummary::from_map(&map, 100, 300);
        assert_eq!(multi_sum.floor_count, 2);
        assert_eq!(multi_sum.wall_count, 3);

        // SavedMapBundle::parse_json_bytes accepts both wrapped bundle and raw GmMapUpdateRequest
        let bundle = SavedMapBundle {
            format_version: SAVED_MAP_FORMAT_VERSION,
            saved_at_us: 300,
            map: map.clone(),
        };
        let bundle_bytes = serde_json::to_vec(&bundle).unwrap();
        assert_eq!(SavedMapBundle::parse_json_bytes(&bundle_bytes).unwrap(), map);

        let raw_bytes = serde_json::to_vec(&map).unwrap();
        assert_eq!(SavedMapBundle::parse_json_bytes(&raw_bytes).unwrap(), map);

        assert!(SavedMapBundle::parse_json_bytes(b"{not valid json").is_err());

        // GenerateSessionMapRequest load_map roundtrip
        let load_req = GenerateSessionMapRequest {
            load_map: Some(Box::new(map.clone())),
            populate_roster_tokens: false,
            ..Default::default()
        };
        let load_json = serde_json::to_string(&load_req).unwrap();
        assert!(load_json.contains(r#""load_map""#));
        let load_dec: GenerateSessionMapRequest = serde_json::from_str(&load_json).unwrap();
        assert_eq!(load_dec, load_req);
    }
}
