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

/// Terrain, cover, obstacle, and elevation classification for a `1.0m` cell on a Satya tactical map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TacticalTileKind {
    #[default]
    Floor,
    HalfCover,
    FullWall,
    DoorClosed,
    DoorOpen,
    HazardZone,
    ElevatedPlatform,
}

impl TacticalTileKind {
    pub const ALL: [Self; 7] = [
        Self::Floor,
        Self::HalfCover,
        Self::FullWall,
        Self::DoorClosed,
        Self::DoorOpen,
        Self::HazardZone,
        Self::ElevatedPlatform,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Floor => "FLOOR",
            Self::HalfCover => "HALF_COVER",
            Self::FullWall => "FULL_WALL",
            Self::DoorClosed => "DOOR_CLOSED",
            Self::DoorOpen => "DOOR_OPEN",
            Self::HazardZone => "HAZARD_ZONE",
            Self::ElevatedPlatform => "ELEVATED_PLATFORM",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Floor => "Open Deck (0.0m)",
            Self::HalfCover => "Half Cover Crate/Console (0.9m, +4 Δ_ext)",
            Self::FullWall => "Full Bulkhead/Pillar (2.4m, Blocks LoS)",
            Self::DoorClosed => "Closed Airlock Door (2.4m, Blocks LoS)",
            Self::DoorOpen => "Open Airlock Threshold (0.0m)",
            Self::HazardZone => "Environmental Hazard (×2 Move Cost)",
            Self::ElevatedPlatform => "Elevated Catwalk (+1.5m)",
        }
    }

    #[must_use]
    pub fn from_code_str(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "HALF_COVER" | "CRATE" | "LOW_COVER" | "COVER" => Self::HalfCover,
            "FULL_WALL" | "WALL" | "BULKHEAD" | "PILLAR" => Self::FullWall,
            "DOOR_CLOSED" | "DOOR" | "AIRLOCK_CLOSED" => Self::DoorClosed,
            "DOOR_OPEN" | "AIRLOCK_OPEN" => Self::DoorOpen,
            "HAZARD_ZONE" | "HAZARD" | "DIFFICULT" => Self::HazardZone,
            "ELEVATED_PLATFORM" | "ELEVATED" | "PLATFORM" | "CATWALK" => Self::ElevatedPlatform,
            _ => Self::Floor,
        }
    }

    /// Physical obstacle or deck elevation height in centimeters (`0`, `90`, `150`, or `240`).
    #[must_use]
    pub const fn height_cm(self) -> i32 {
        match self {
            Self::Floor | Self::DoorOpen | Self::HazardZone => 0,
            Self::HalfCover => 90,
            Self::ElevatedPlatform => 150,
            Self::FullWall | Self::DoorClosed => 240,
        }
    }

    /// Physical obstacle or deck elevation height in meters (`0.0`, `0.9`, `1.5`, or `2.4`).
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

    /// Whether an intervening ray crossing this cell imposes Low/Half Cover (`+4 Δ_ext`).
    #[must_use]
    pub const fn provides_half_cover(self) -> bool {
        matches!(self, Self::HalfCover)
    }

    /// Movement cost multiplier (`1.0` normal deck, `2.0` hazard/difficult or vaulting).
    #[must_use]
    pub const fn move_cost_multiplier(self) -> f64 {
        match self {
            Self::HazardZone => 2.0,
            Self::ElevatedPlatform => 1.25,
            _ => 1.0,
        }
    }
}

/// Evaluated Line-of-Sight and tactical cover classification between two cells on the `1.0m` grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoverGrade {
    #[default]
    Clear,
    LowCover,
    HighCover,
    Blocked,
}

impl CoverGrade {
    /// Canonical Satya Ranged External Modifier (`Δ_ext`) imposed by this cover state:
    /// `+0` Clear, `+4` Low/Half Cover (`0.9m`), `+6` High/Corner Cover (`2.4m` edge peek), or `+16` Blocked.
    #[must_use]
    pub const fn recommended_delta_ext(self) -> i32 {
        match self {
            Self::Clear => 0,
            Self::LowCover => 4,
            Self::HighCover => 6,
            Self::Blocked => 16,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Clear => "Clear LoS (+0 Δ_ext)",
            Self::LowCover => "Low Cover (+4 Δ_ext)",
            Self::HighCover => "High / Corner Cover (+6 Δ_ext)",
            Self::Blocked => "Blocked (No LoS)",
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
        }
    }
}

/// HTTP request body for `POST /api/sessions/:session_id/map` generating a procedural tactical sector
/// or editing individual terrain/door tiles on the active session map.
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
    /// Optional single-cell terrain/door mutation X coordinate (`0..grid_width`) on the existing active map.
    #[serde(default)]
    pub edit_cell_x: Option<i32>,
    /// Optional single-cell terrain/door mutation Y coordinate (`0..grid_height`) on the existing active map.
    #[serde(default)]
    pub edit_cell_y: Option<i32>,
    /// Target [`TacticalTileKind`] when `edit_cell_x` and `edit_cell_y` are provided (`None` toggles doors/cover).
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
        }
    }
}

/// HTTP request body for `POST /api/sessions/:session_id/map/move` moving, orienting, spawning,
/// or removing a character/NPC miniature on the active `1.0m` tactical grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MoveMapTokenRequest {
    pub entity_id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub to_x: i32,
    pub to_y: i32,
    #[serde(default)]
    pub facing_deg: Option<i32>,
    #[serde(default)]
    pub model_preset: Option<String>,
    #[serde(default)]
    pub color_hex: Option<String>,
    #[serde(default)]
    pub stance_pose: Option<String>,
    #[serde(default)]
    pub visibility: Option<MessageVisibility>,
    #[serde(default)]
    pub remove: bool,
}
