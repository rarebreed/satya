//! Tactical `1.0m` Grid Terrain Tiles, Cover Grades, 3D Miniature Visual Specs,
//! and Interactive Map Generation / Token Movement REST DTOs.
//!
//! Edge walls, multi-cell props, and material classes live in [`crate::tactical_map_dressing_dto`].

pub use crate::tactical_map_dressing_dto::*;

use crate::events::MessageVisibility;
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
}

/// Validates and normalizes an optional character/NPC token portrait URL, rejecting script vectors
/// (`javascript:`, `data:image/svg+xml`, control chars, quotes, and protocol-relative `//` URLs).
#[must_use]
pub fn sanitize_portrait_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 4096 {
        return None;
    }
    if trimmed
        .bytes()
        .any(|b| b.is_ascii_control() || b.is_ascii_whitespace() || matches!(b, b'<' | b'>' | b'"' | b'\''))
    {
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
        }
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
    /// When `true` with `edit_edge`, removes the edge entirely (GM only).
    #[serde(default)]
    pub clear_edge: bool,
    /// Optional GM continuous prop placement/replacement on the existing active map.
    #[serde(default)]
    pub edit_prop: Option<MapPropPlacement>,
    /// Optional GM prop removal covering integer cell `(x, y)` on the existing active map.
    #[serde(default)]
    pub clear_prop_at: Option<(i32, i32)>,
    /// Optional GM prop removal at continuous centimeter point `(x_cm, y_cm)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_prop_at_cm: Option<(i32, i32)>,
    /// Optional cardinal boundary (`NORTH`, `SOUTH`, `EAST`, `WEST`) to seamlessly expand and stitch
    /// onto the active tactical map using edge-matched procedural generation (GM or Solo AI-GM only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_direction: Option<MapContinuationDir>,
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
            cell_size_cm: default_cell_size_cm(),
            seed: None,
            populate_roster_tokens: true,
            edit_cell_x: None,
            edit_cell_y: None,
            set_tile_kind: None,
            edit_edge: None,
            set_edge_kind: None,
            clear_edge: false,
            edit_prop: None,
            clear_prop_at: None,
            clear_prop_at_cm: None,
            continue_direction: None,
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
            assert_eq!(sanitize_portrait_url(bad), None, "expected reject for {bad}");
        }
    }

    #[test]
    fn map_continuation_dir_and_request_roundtrip() {
        assert_eq!(MapContinuationDir::North.opposite(), MapContinuationDir::South);
        assert_eq!(MapContinuationDir::South.opposite(), MapContinuationDir::North);
        assert_eq!(MapContinuationDir::East.opposite(), MapContinuationDir::West);
        assert_eq!(MapContinuationDir::West.opposite(), MapContinuationDir::East);
        assert_eq!(MapContinuationDir::East.unit_step(), (1, 0));
        assert_eq!(MapContinuationDir::West.label(), "West");
        assert_eq!(MapContinuationDir::from_code_str("south"), Some(MapContinuationDir::South));

        let req = GenerateSessionMapRequest {
            continue_direction: Some(MapContinuationDir::East),
            seed: Some(777),
            ..Default::default()
        };
        let encoded = serde_json::to_string(&req).unwrap();
        assert!(encoded.contains(r#""continue_direction":"EAST""#));
        let decoded: GenerateSessionMapRequest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, req);

        // Omitted continue_direction defaults to None cleanly for older clients.
        let legacy: GenerateSessionMapRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(legacy.continue_direction, None);
    }
}
