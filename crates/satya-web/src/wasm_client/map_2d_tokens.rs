//! # 2D Battlemap Paper/Ink Filter Styles & Top-Down Token Figures (`map_2d_tokens`)
//!
//! Companion to [`crate::wasm_client::map_2d_art`], [`crate::wasm_client::map_2d_art_overlay`],
//! and [`crate::wasm_client::map_2d_token_sprites`].
//! Implements visual polish without changing `satya-engine` mechanics:
//! - Selectable paper/ink filter styles (`ModernVellum`, `CleanVector`, `Blueprint`, `FieldSketch`)
//!   persisted in `localStorage` and hashed into `battlemap_art_cache_key`.
//! - Top-down 90° tactical 2D token figures across 12 2035 archetypes × `{Standing, Kneeling, Prone}`
//!   rendered via Style C WebP sprites (`Painted`) or bespoke SVG silhouettes (`ProceduralVector`),
//!   while preserving `<clipPath>` circular medallions when an explicit `portrait_url` is set.

pub use crate::wasm_client::map_2d_token_sprites::*;

use crate::wasm_client::dom::{escape_html, local_storage_get, local_storage_set, select_value};
use crate::wasm_client::map_2d_art_overlay::{sync_battlemap_svg_host, token_initials};
use crate::wasm_client::map_2d_art_props::BATTLEMAP_INK;
use crate::wasm_client::map_2d_art_textures::BattlemapTextureMode;
use crate::wasm_client::map_3d::with_map_ui_mut;
use satya_schema::{MessageVisibility, TokenPlacement, TokenStanceKind, sanitize_portrait_url};
use std::fmt::Write as _;
use wasm_bindgen::prelude::*;

pub const INK_STYLE_PREF_KEY: &str = "satya.map.ink_style";

/// User-selectable SVG paper/ink finish preset for the illustrated 2D battlemap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BattlemapInkStyle {
    #[default]
    ModernVellum,
    CleanVector,
    Blueprint,
    FieldSketch,
}

impl BattlemapInkStyle {
    pub const ALL: [Self; 4] = [
        Self::ModernVellum,
        Self::CleanVector,
        Self::Blueprint,
        Self::FieldSketch,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModernVellum => "vellum",
            Self::CleanVector => "clean",
            Self::Blueprint => "blueprint",
            Self::FieldSketch => "sketch",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ModernVellum => "Modern Vellum",
            Self::CleanVector => "Crisp Vector",
            Self::Blueprint => "Tactical Blueprint",
            Self::FieldSketch => "Aged Field Sketch",
        }
    }

    #[must_use]
    pub fn from_code_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "clean" | "crisp" | "vector" => Self::CleanVector,
            "blueprint" | "cyanotype" => Self::Blueprint,
            "sketch" | "sepia" | "parchment" => Self::FieldSketch,
            _ => Self::ModernVellum,
        }
    }

    #[must_use]
    pub const fn margin_fill(self) -> &'static str {
        match self {
            Self::ModernVellum => "#2b2723",
            Self::CleanVector => "#23272e",
            Self::Blueprint => "#0b2545",
            Self::FieldSketch => "#3a2e21",
        }
    }

    #[must_use]
    pub const fn uses_color_tint(self) -> bool {
        matches!(self, Self::Blueprint | Self::FieldSketch)
    }

    #[must_use]
    pub const fn grid_ink_override(self) -> Option<&'static str> {
        match self {
            Self::Blueprint => Some("#dbeafe"),
            Self::FieldSketch => Some("#2c1d11"),
            Self::ModernVellum | Self::CleanVector => None,
        }
    }
}

/// Emits SVG `<filter>` definitions (`#bm-ink`, `#bm-prop`, `#bm-shadow`, `#bm-grain`, `#bm-mottle`,
/// and optional `#bm-tint`) tuned for `style`.
pub fn push_ink_style_filters(out: &mut String, style: BattlemapInkStyle) {
    match style {
        BattlemapInkStyle::CleanVector => {
            out.push_str(concat!(
                r##"<filter id="bm-ink" x="-5%" y="-5%" width="110%" height="110%"><feOffset dx="0" dy="0"/></filter>"##,
                r##"<filter id="bm-prop" x="-10%" y="-10%" width="120%" height="120%"><feDropShadow dx="2.5" dy="3.5" stdDeviation="2.2" flood-color="#0f172a" flood-opacity=".34"/></filter>"##,
            ));
        }
        BattlemapInkStyle::ModernVellum => {
            out.push_str(concat!(
                r##"<filter id="bm-ink" x="-5%" y="-5%" width="110%" height="110%"><feTurbulence type="fractalNoise" baseFrequency="0.035" numOctaves="2" seed="3" result="n"/><feDisplacementMap in="SourceGraphic" in2="n" scale="3.5" xChannelSelector="R" yChannelSelector="G"/></filter>"##,
                r##"<filter id="bm-prop" x="-10%" y="-10%" width="120%" height="120%"><feTurbulence type="fractalNoise" baseFrequency="0.04" numOctaves="2" seed="7" result="n"/><feDisplacementMap in="SourceGraphic" in2="n" scale="2.5" xChannelSelector="R" yChannelSelector="G" result="d"/><feDropShadow in="d" dx="3" dy="4" stdDeviation="2.5" flood-color="#1a140c" flood-opacity=".38"/></filter>"##,
            ));
        }
        BattlemapInkStyle::Blueprint => {
            out.push_str(concat!(
                r##"<filter id="bm-ink" x="-5%" y="-5%" width="110%" height="110%"><feTurbulence type="fractalNoise" baseFrequency="0.035" numOctaves="2" seed="3" result="n"/><feDisplacementMap in="SourceGraphic" in2="n" scale="0.8" xChannelSelector="R" yChannelSelector="G"/></filter>"##,
                r##"<filter id="bm-prop" x="-10%" y="-10%" width="120%" height="120%"><feDropShadow dx="2" dy="2.5" stdDeviation="2" flood-color="#06162d" flood-opacity=".45"/></filter>"##,
                r##"<filter id="bm-tint" x="0%" y="0%" width="100%" height="100%" color-interpolation-filters="sRGB"><feColorMatrix type="matrix" values="-0.22 -0.22 -0.22 0 0.24 -0.35 -0.35 -0.35 0 0.56 -0.38 -0.38 -0.38 0 0.84 0 0 0 1 0"/></filter>"##,
            ));
        }
        BattlemapInkStyle::FieldSketch => {
            out.push_str(concat!(
                r##"<filter id="bm-ink" x="-5%" y="-5%" width="110%" height="110%"><feTurbulence type="fractalNoise" baseFrequency="0.038" numOctaves="2" seed="3" result="n"/><feDisplacementMap in="SourceGraphic" in2="n" scale="4.8" xChannelSelector="R" yChannelSelector="G"/></filter>"##,
                r##"<filter id="bm-prop" x="-10%" y="-10%" width="120%" height="120%"><feTurbulence type="fractalNoise" baseFrequency="0.042" numOctaves="2" seed="7" result="n"/><feDisplacementMap in="SourceGraphic" in2="n" scale="3.4" xChannelSelector="R" yChannelSelector="G" result="d"/><feDropShadow in="d" dx="3.2" dy="4.2" stdDeviation="2.8" flood-color="#22170a" flood-opacity=".44"/></filter>"##,
                r##"<filter id="bm-tint" x="0%" y="0%" width="100%" height="100%" color-interpolation-filters="sRGB"><feColorMatrix type="matrix" values="0.39 0.72 0.16 0 0.04 0.34 0.64 0.14 0 0.02 0.25 0.46 0.11 0 -0.02 0 0 0 1 0"/></filter>"##,
            ));
        }
    }
    out.push_str(concat!(
        r##"<filter id="bm-shadow" x="-10%" y="-10%" width="120%" height="120%"><feDropShadow dx="2.5" dy="3.5" stdDeviation="3" flood-color="#120d08" flood-opacity=".42"/></filter>"##,
        r##"<filter id="bm-grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.8" numOctaves="2" seed="11" stitchTiles="stitch"/><feColorMatrix values="0 0 0 0 .16 0 0 0 0 .13 0 0 0 0 .1 0 0 0 .55 -.18"/></filter>"##,
        r##"<filter id="bm-mottle" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.012" numOctaves="3" seed="5"/><feColorMatrix values="0 0 0 0 .1 0 0 0 0 .08 0 0 0 0 .05 .42 0 0 0 -.19"/></filter>"##,
    ));
}

/// Emits the final paper tooth / blueprint drafting border / aged wash finish for `#battlemap-art`.
pub fn push_ink_style_finish(out: &mut String, style: BattlemapInkStyle, vw: i32, vh: i32) {
    match style {
        BattlemapInkStyle::ModernVellum => {
            let _ = write!(
                out,
                r##"<rect width="{vw}" height="{vh}" filter="url(#bm-grain)" opacity=".7" style="mix-blend-mode:multiply"/>"##
            );
        }
        BattlemapInkStyle::CleanVector => {}
        BattlemapInkStyle::Blueprint => {
            let _ = write!(
                out,
                r##"<rect width="{vw}" height="{vh}" filter="url(#bm-grain)" opacity=".35" style="mix-blend-mode:screen"/><rect x="4" y="4" width="{w1}" height="{h1}" fill="none" stroke="#bae6fd" stroke-width="2" stroke-opacity=".78"/><rect x="9" y="9" width="{w2}" height="{h2}" fill="none" stroke="#bae6fd" stroke-width=".9" stroke-opacity=".5"/>"##,
                w1 = (vw - 8).max(1),
                h1 = (vh - 8).max(1),
                w2 = (vw - 18).max(1),
                h2 = (vh - 18).max(1),
            );
        }
        BattlemapInkStyle::FieldSketch => {
            let _ = write!(
                out,
                r##"<rect width="{vw}" height="{vh}" fill="#f0dfb8" fill-opacity=".2" style="mix-blend-mode:multiply"/><rect width="{vw}" height="{vh}" filter="url(#bm-grain)" opacity=".85" style="mix-blend-mode:multiply"/>"##
            );
        }
    }
}

/// Loads persisted `localStorage` ink style into UI state when initializing battlemap chrome.
pub fn load_persisted_ink_style_pref() -> BattlemapInkStyle {
    local_storage_get(INK_STYLE_PREF_KEY)
        .map(|s| BattlemapInkStyle::from_code_str(&s))
        .unwrap_or_default()
}

/// Switches the 2D SVG battlemap paper/ink filter style (`#map-ink-style-select`).
#[wasm_bindgen]
pub fn satya_set_battlemap_ink_style() {
    let raw = select_value("map-ink-style-select");
    let style = BattlemapInkStyle::from_code_str(&raw);
    with_map_ui_mut(|ui| {
        ui.ink_style = style;
    });
    local_storage_set(INK_STYLE_PREF_KEY, style.as_str());
    sync_battlemap_svg_host();
}

/// Inputs for rendering a single token on the 2D SVG battlemap overlay.
pub struct TokenRenderInput<'a> {
    pub idx: usize,
    pub tok: &'a TokenPlacement,
    pub pos_m: (f32, f32),
    pub preset: &'a str,
    pub stance_pose: &'a str,
    pub color_hex: &'a str,
    pub facing_deg: f32,
    pub portrait_url: Option<&'a str>,
    pub is_selected: bool,
    pub cell_px: i32,
    pub texture_mode: BattlemapTextureMode,
}

/// Top-down SVG figure palette for [`BattlemapTextureMode::ProceduralVector`] fallbacks.
struct FigureVectorPalette {
    garment: &'static str,
    trim: &'static str,
    head: &'static str,
    weapon: Option<VectorWeaponKind>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VectorWeaponKind {
    None,
    Handgun,
    Rifle,
    Sniper,
    Knife,
}

fn vector_palette_for(canonical: &str) -> FigureVectorPalette {
    match canonical {
        "civilian_unarmed" => FigureVectorPalette {
            garment: "#5c6b57",
            trim: "#334155",
            head: "#3f2e26",
            weapon: Some(VectorWeaponKind::None),
        },
        "civilian_handgun" => FigureVectorPalette {
            garment: "#4f6d8a",
            trim: "#334155",
            head: "#4a372d",
            weapon: Some(VectorWeaponKind::Handgun),
        },
        "civilian_rifle" => FigureVectorPalette {
            garment: "#8a7254",
            trim: "#4c5c4a",
            head: "#3d2d23",
            weapon: Some(VectorWeaponKind::Rifle),
        },
        "civilian_knife" => FigureVectorPalette {
            garment: "#9c7a54",
            trim: "#334155",
            head: "#43464b",
            weapon: Some(VectorWeaponKind::Knife),
        },
        "security_handgun" => FigureVectorPalette {
            garment: "#253954",
            trim: "#1f2937",
            head: "#273445",
            weapon: Some(VectorWeaponKind::Handgun),
        },
        "soldier_rifle" => FigureVectorPalette {
            garment: "#5b6a5e",
            trim: "#3d4a41",
            head: "#4c5a50",
            weapon: Some(VectorWeaponKind::Rifle),
        },
        "sniper_rifle" => FigureVectorPalette {
            garment: "#4d5e52",
            trim: "#36433a",
            head: "#435247",
            weapon: Some(VectorWeaponKind::Sniper),
        },
        "android_domestic" => FigureVectorPalette {
            garment: "#e2e8f0",
            trim: "#94a3b8",
            head: "#f1f5f9",
            weapon: Some(VectorWeaponKind::None),
        },
        "android_industrial" => FigureVectorPalette {
            garment: "#94a3b8",
            trim: "#eab308",
            head: "#64748b",
            weapon: Some(VectorWeaponKind::None),
        },
        "android_security" => FigureVectorPalette {
            garment: "#475569",
            trim: "#556458",
            head: "#334155",
            weapon: Some(VectorWeaponKind::Rifle),
        },
        _ => FigureVectorPalette {
            garment: "#5c6b57",
            trim: "#334155",
            head: "#3f2e26",
            weapon: Some(VectorWeaponKind::None),
        },
    }
}

/// Renders one top-down 2D operator portrait silhouette inside the clipped circular disc.
fn push_preset_portrait_silhouette(out: &mut String, cx: i32, cy: i32, preset: &str, accent: &str) {
    let _ = write!(
        out,
        r##"<circle cx="{cx}" cy="{cy}" r="18" fill="#18212d"/><circle cx="{cx}" cy="{}" r="12" fill="{accent}" fill-opacity=".22"/><path d="M{} {}c2-7 16-7 18 0v6H{}z" fill="#2b3748" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##,
        cy - 4,
        cx - 9,
        cy + 13,
        cx - 9,
    );
    match preset.trim() {
        "valkyrie_rig" | "android_security" => {
            let _ = write!(
                out,
                r##"<path d="M{} {}l-5-7 6 3zm{} {}l5-7-6 3z" fill="{accent}" stroke="{BATTLEMAP_INK}" stroke-width=".9"/><circle cx="{cx}" cy="{}" r="7.5" fill="#d8e3ee" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/><path d="M{} {}h10l-2 4h-6z" fill="{accent}" stroke="{BATTLEMAP_INK}" stroke-width=".8"/>"##,
                cx - 6,
                cy - 3,
                cx + 6,
                cy - 3,
                cy - 3,
                cx - 5,
                cy - 5,
            );
        }
        "recon_mk4" | "sniper_rifle" => {
            let _ = write!(
                out,
                r##"<path d="M{} {}c0-8 16-8 16 0v5h-16z" fill="#2c3a47" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/><circle cx="{cx}" cy="{}" r="6" fill="#465666"/><circle cx="{}" cy="{}" r="2.3" fill="#34d399" stroke="{BATTLEMAP_INK}" stroke-width=".8"/><circle cx="{}" cy="{}" r="2.3" fill="#34d399" stroke="{BATTLEMAP_INK}" stroke-width=".8"/>"##,
                cx - 8,
                cy - 1,
                cy - 2,
                cx - 3,
                cy - 3,
                cx + 3,
                cy - 3,
            );
        }
        "secops_sentry" | "soldier_rifle" => {
            let _ = write!(
                out,
                r##"<rect x="{}" y="{}" width="16" height="15" rx="6" fill="#39404a" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><rect x="{}" y="{}" width="12" height="4" rx="1.5" fill="{accent}" stroke="{BATTLEMAP_INK}" stroke-width=".8"/><path d="M{cx} {}v5" stroke="{accent}" stroke-width="2.2" stroke-linecap="round"/>"##,
                cx - 8,
                cy - 11,
                cx - 6,
                cy - 6,
                cy - 3,
            );
        }
        _ => {
            let _ = write!(
                out,
                r##"<circle cx="{cx}" cy="{}" r="7" fill="#c9a585" stroke="{BATTLEMAP_INK}" stroke-width="1.1"/><path d="M{} {}c1-5 12-5 14 0v-2c-1-4-13-4-14 0z" fill="#2d333b"/><path d="M{} {}a8 8 0 0 1 16 0" fill="none" stroke="{accent}" stroke-width="2" stroke-linecap="round"/><rect x="{}" y="{}" width="10" height="3.2" rx="1.2" fill="{accent}" fill-opacity=".88"/>"##,
                cy - 3,
                cx - 7,
                cy - 5,
                cx - 8,
                cy - 4,
                cx - 5,
                cy - 5,
            );
        }
    }
}

fn push_vector_weapon(out: &mut String, kind: VectorWeaponKind, y_anchor: f32) {
    match kind {
        VectorWeaponKind::None => {}
        VectorWeaponKind::Handgun => {
            let _ = write!(
                out,
                r##"<rect x="-2.4" y="{:.1}" width="4.8" height="12" rx="1.5" fill="#1e293b" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/>"##,
                y_anchor - 18.0
            );
        }
        VectorWeaponKind::Rifle => {
            let _ = write!(
                out,
                r##"<rect x="1" y="{:.1}" width="5" height="24" rx="1.5" fill="#27272a" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/><rect x="2.2" y="{:.1}" width="2.6" height="9" fill="#52525b"/>"##,
                y_anchor - 26.0,
                y_anchor - 32.0
            );
        }
        VectorWeaponKind::Sniper => {
            let _ = write!(
                out,
                r##"<rect x="1" y="{:.1}" width="4.8" height="32" rx="1.2" fill="#1f2937" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/><rect x="2.2" y="{:.1}" width="2.4" height="13" fill="#374151"/><circle cx="3.4" cy="{:.1}" r="2.6" fill="#0f172a" stroke="#94a3b8" stroke-width=".9"/>"##,
                y_anchor - 30.0,
                y_anchor - 41.0,
                y_anchor - 20.0
            );
        }
        VectorWeaponKind::Knife => {
            let _ = write!(
                out,
                r##"<path d="M8 {:.1}c3-4 5-9 2-13-2 4-4 7-5 11z" fill="#334155" stroke="{BATTLEMAP_INK}" stroke-width="1.1"/>"##,
                y_anchor - 4.0
            );
        }
    }
}

/// Renders a bespoke top-down 90° SVG figure centered at `(0, 0)` facing North (`-Y`) for Vector mode.
fn push_topdown_vector_figure(
    out: &mut String,
    canonical: &str,
    stance: TokenStanceKind,
    accent: &str,
) {
    match canonical {
        "drone_quad" => {
            for (lx, ly) in [(-12, -13), (12, -13), (-12, 13), (12, 13)] {
                let _ = write!(
                    out,
                    r##"<rect x="{}" y="{}" width="6" height="11" rx="2" fill="#334155" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/>"##,
                    lx - 3,
                    ly - 5
                );
            }
            let _ = write!(
                out,
                r##"<rect x="-10" y="-17" width="20" height="34" rx="5" fill="#5b6658" stroke="{BATTLEMAP_INK}" stroke-width="1.8"/><rect x="-7" y="-11" width="14" height="18" rx="2" fill="#3f473e" stroke="{BATTLEMAP_INK}" stroke-width="1"/><rect x="-6" y="-21" width="12" height="6" rx="2" fill="#1e293b" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/><circle cx="0" cy="-19" r="2" fill="{accent}"/>"##
            );
            return;
        }
        "drone_aero" => {
            let _ = write!(
                out,
                r##"<path d="M-19 -19L19 19M19 -19L-19 19" stroke="#1e293b" stroke-width="4" stroke-linecap="round"/><ellipse cx="0" cy="0" rx="7" ry="15" fill="#334155" stroke="{BATTLEMAP_INK}" stroke-width="1.8"/><circle cx="0" cy="-11" r="3" fill="#0f172a" stroke="{accent}" stroke-width="1"/>"##
            );
            for (rx, ry) in [(-19, -19), (19, -19), (-19, 19), (19, 19)] {
                let _ = write!(
                    out,
                    r##"<circle cx="{rx}" cy="{ry}" r="10.5" fill="#94a3b8" fill-opacity=".24" stroke="#1e293b" stroke-width="1.6"/><circle cx="{rx}" cy="{ry}" r="2.6" fill="#0f172a"/>"##
                );
            }
            return;
        }
        _ => {}
    }

    let pal = vector_palette_for(canonical);
    let wpn = pal.weapon.unwrap_or(VectorWeaponKind::None);
    let is_android = canonical.starts_with("android_");
    match stance {
        TokenStanceKind::Prone => {
            // Elongated North-South prone figure: boots at +Y, torso mid, arms/weapon at -Y.
            let _ = write!(
                out,
                r##"<rect x="-10" y="14" width="8" height="22" rx="3.5" fill="#27303f" stroke="{BATTLEMAP_INK}" stroke-width="1.5"/><rect x="2" y="14" width="8" height="22" rx="3.5" fill="#27303f" stroke="{BATTLEMAP_INK}" stroke-width="1.5"/><rect x="-13" y="-14" width="26" height="32" rx="9" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.8"/><rect x="-9" y="-9" width="18" height="18" rx="4" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##,
                pal.garment, pal.trim
            );
            push_vector_weapon(out, wpn, -12.0);
            let _ = write!(
                out,
                r##"<circle cx="0" cy="-15" r="8.5" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.7"/>"##,
                pal.head
            );
        }
        TokenStanceKind::Kneeling => {
            // Compact crouched silhouette + tucked rear boot peeking South.
            let _ = write!(
                out,
                r##"<ellipse cx="4" cy="14" rx="6" ry="8" fill="#1f2937" stroke="{BATTLEMAP_INK}" stroke-width="1.4"/><ellipse cx="0" cy="1" rx="16" ry="11" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.8"/><rect x="-10" y="-6" width="20" height="12" rx="4" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##,
                pal.garment, pal.trim
            );
            if is_android {
                let _ = write!(
                    out,
                    r##"<path d="M-13 1H13" stroke="#0f172a" stroke-width="1.4"/>"##
                );
            }
            push_vector_weapon(out, wpn, -4.0);
            let _ = write!(
                out,
                r##"<circle cx="0" cy="-2" r="8" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.7"/>"##,
                pal.head
            );
        }
        TokenStanceKind::WalkingHolstered => {
            // Natural mid-stride boots + relaxed shoulders with weapon holstered/slung (no drawn gun/blade).
            let _ = write!(
                out,
                r##"<rect x="-8" y="-15" width="6" height="10" rx="2.5" fill="#1f2937" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><rect x="2" y="7" width="6" height="10" rx="2.5" fill="#1f2937" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><ellipse cx="0" cy="0" rx="18" ry="10.5" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.8"/><rect x="-10" y="-6" width="20" height="12" rx="4" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##,
                pal.garment, pal.trim
            );
            if matches!(wpn, VectorWeaponKind::Rifle | VectorWeaponKind::Sniper) {
                let _ = write!(
                    out,
                    r##"<line x1="-11" y1="-8" x2="11" y2="9" stroke="#27272a" stroke-width="3.6" stroke-linecap="round"/>"##
                );
            } else if matches!(wpn, VectorWeaponKind::Handgun | VectorWeaponKind::Knife) {
                let _ = write!(
                    out,
                    r##"<rect x="13" y="-3" width="4" height="7" rx="1.2" fill="#1e293b" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##
                );
            }
            let _ = write!(
                out,
                r##"<circle cx="0" cy="-1" r="8.5" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.7"/>"##,
                pal.head
            );
        }
        TokenStanceKind::TacticalAdvance => {
            // Staggered athletic combat-glide stride + weapon at controlled low-ready.
            let _ = write!(
                out,
                r##"<rect x="-9" y="-16" width="6.5" height="11" rx="2.5" fill="#1f2937" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><rect x="3" y="8" width="6.5" height="11" rx="2.5" fill="#1f2937" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><ellipse cx="0" cy="0" rx="17" ry="11" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.8"/><rect x="-10" y="-6" width="20" height="12" rx="4" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##,
                pal.garment, pal.trim
            );
            push_vector_weapon(out, wpn, 2.0);
            let _ = write!(
                out,
                r##"<circle cx="0" cy="-2" r="8.2" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.7"/>"##,
                pal.head
            );
        }
        TokenStanceKind::Sprinting => {
            // Wide running stride boots + forward-pitched torso + pumping arms.
            let _ = write!(
                out,
                r##"<rect x="-9" y="-21" width="6.5" height="13" rx="3" fill="#1f2937" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><rect x="3" y="10" width="7" height="14" rx="3" fill="#1f2937" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><ellipse cx="0" cy="-2" rx="17" ry="11" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.8"/><rect x="-10" y="-8" width="20" height="12" rx="4" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##,
                pal.garment, pal.trim
            );
            if matches!(wpn, VectorWeaponKind::Rifle | VectorWeaponKind::Sniper) {
                let _ = write!(
                    out,
                    r##"<line x1="-12" y1="-11" x2="12" y2="-4" stroke="#27272a" stroke-width="4" stroke-linecap="round"/>"##
                );
            } else {
                push_vector_weapon(out, wpn, -2.0);
            }
            let _ = write!(
                out,
                r##"<circle cx="0" cy="-4" r="8.2" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.7"/>"##,
                pal.head
            );
        }
        TokenStanceKind::Standing => {
            let _ = write!(
                out,
                r##"<ellipse cx="0" cy="0" rx="18" ry="11" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.8"/><rect x="-11" y="-7" width="22" height="14" rx="5" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##,
                pal.garment, pal.trim
            );
            if is_android {
                let _ = write!(
                    out,
                    r##"<circle cx="-13" cy="0" r="3" fill="#1e293b"/><circle cx="13" cy="0" r="3" fill="#1e293b"/>"##
                );
            }
            push_vector_weapon(out, wpn, -4.0);
            let _ = write!(
                out,
                r##"<circle cx="0" cy="-1" r="8.5" fill="{}" stroke="{BATTLEMAP_INK}" stroke-width="1.7"/>"##,
                pal.head
            );
        }
    }
}

/// Renders an explicit `portrait_url` token using the circular `<clipPath>` medallion framing.
fn push_portrait_disc_svg(
    out: &mut String,
    input: &TokenRenderInput<'_>,
    cx: i32,
    cy: i32,
    col: &str,
    safe_url: &str,
) {
    let clip_id = format!("bm-tok-clip-{}", input.idx);
    let _ = write!(
        out,
        r##"<ellipse cx="{}" cy="{}" rx="23" ry="21" fill="#120d08" fill-opacity=".35"/>"##,
        cx + 3,
        cy + 4
    );
    if input.is_selected {
        let _ = write!(
            out,
            r##"<circle cx="{cx}" cy="{cy}" r="29" fill="none" stroke="#f5b301" stroke-width="3.5" stroke-dasharray="6 4"/>"##
        );
    }
    let _ = write!(
        out,
        r##"<g transform="rotate({:.0} {cx} {cy})"><path d="M{cx} {}l-8 10h16z" fill="{col}" stroke="{BATTLEMAP_INK}" stroke-width="1.6" stroke-linejoin="round"/></g><clipPath id="{clip_id}"><circle cx="{cx}" cy="{cy}" r="17"/></clipPath><circle cx="{cx}" cy="{cy}" r="22" fill="{col}" stroke="{BATTLEMAP_INK}" stroke-width="2"/><g clip-path="url(#{clip_id})"><circle cx="{cx}" cy="{cy}" r="17.5" fill="#18212d"/><image href="{}" x="{}" y="{}" width="36" height="36" preserveAspectRatio="xMidYMid slice"/></g><circle cx="{cx}" cy="{cy}" r="17.2" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.5"/>"##,
        input.facing_deg,
        cy - 33,
        escape_html(safe_url),
        cx - 18,
        cy - 18,
    );
}

/// Renders one interactive 2D token onto `#battlemap-overlay` at metric `pos_m`:
/// - Uses a circular portrait medallion if an allowlisted `portrait_url` is set.
/// - Otherwise renders a true 90° top-down 2D figure (`Painted` Style C WebP sprite or
///   `ProceduralVector` SVG silhouette) rotated by `facing_deg` with Standing/Kneeling/Prone stances.
pub fn push_token_2d_svg(out: &mut String, input: &TokenRenderInput<'_>) {
    let cf = input.cell_px as f32;
    let (cx, cy) = (
        (input.pos_m.0 * cf).round() as i32,
        (input.pos_m.1 * cf).round() as i32,
    );
    let col = escape_html(input.color_hex);
    let op = if input.tok.visibility == MessageVisibility::GmOnly {
        ".55"
    } else {
        "1"
    };
    let _ = write!(out, r##"<g opacity="{op}" pointer-events="none">"##);

    if let Some(safe_url) = input.portrait_url.and_then(sanitize_portrait_url) {
        push_portrait_disc_svg(out, input, cx, cy, &col, &safe_url);
    } else {
        let stance = TokenStanceKind::from_stance_str(input.stance_pose);
        let canonical = canonical_2d_token_preset(input.preset);
        let resolved = resolve_token_sprite(input.preset, stance);
        let effective_stance = resolved.map_or(stance, |s| s.stance);
        let is_airborne = resolved.is_some_and(|s| s.is_airborne) || canonical == "drone_aero";
        let is_prone = effective_stance == TokenStanceKind::Prone;
        let ring_r = if effective_stance == TokenStanceKind::Kneeling {
            21
        } else {
            24
        };

        // 1. Ground shadow
        if is_airborne {
            let _ = write!(
                out,
                r##"<ellipse cx="{}" cy="{}" rx="22" ry="18" fill="#090d14" fill-opacity=".26"/>"##,
                cx + 11,
                cy + 13
            );
        } else if is_prone {
            let _ = write!(
                out,
                r##"<g transform="rotate({:.0} {cx} {cy})"><ellipse cx="3" cy="4" rx="18" ry="34" fill="#120d08" fill-opacity=".30"/></g>"##,
                input.facing_deg
            );
        } else {
            let _ = write!(
                out,
                r##"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="#120d08" fill-opacity=".32"/>"##,
                cx + 3,
                cy + 4,
                ring_r - 1,
                ring_r - 3,
            );
        }

        // 2. Selection highlight + subtle faction base ring + North facing chevron
        if input.is_selected {
            if is_prone {
                let _ = write!(
                    out,
                    r##"<g transform="rotate({:.0} {cx} {cy})"><ellipse cx="0" cy="0" rx="26" ry="44" fill="none" stroke="#f5b301" stroke-width="3.2" stroke-dasharray="6 4"/></g>"##,
                    input.facing_deg
                );
            } else {
                let _ = write!(
                    out,
                    r##"<circle cx="{cx}" cy="{cy}" r="{}" fill="none" stroke="#f5b301" stroke-width="3.2" stroke-dasharray="6 4"/>"##,
                    ring_r + 7
                );
            }
        }
        let _ = write!(
            out,
            r##"<g transform="rotate({:.0} {cx} {cy})"><circle cx="0" cy="0" r="{ring_r}" fill="{col}" fill-opacity=".14" stroke="{col}" stroke-width="2.2"/><path d="M0 {}l-5 8h10z" fill="{col}" stroke="{BATTLEMAP_INK}" stroke-width="1.2" stroke-linejoin="round"/></g>"##,
            input.facing_deg,
            -(ring_r + 8),
        );

        // 3. Top-down 90° figure (Style C painted WebP sprite or bespoke vector fallback)
        let _ = write!(
            out,
            r##"<g transform="translate({cx} {cy}) rotate({:.0})">"##,
            input.facing_deg
        );
        if input.texture_mode == BattlemapTextureMode::Painted
            && let Some(spec) = resolved
        {
            let w_px = spec.render_w_m * cf;
            let h_px = spec.render_h_m * cf;
            let file = spec.variant_file_for_entity(&input.tok.entity_id);
            let _ = write!(
                out,
                r#"<image href="{BATTLEMAP_TOKEN_SPRITE_BASE_URL}/{file}" x="{:.1}" y="{:.1}" width="{w_px:.1}" height="{h_px:.1}" preserveAspectRatio="xMidYMid meet"/>"#,
                -w_px * 0.5,
                -h_px * 0.5,
            );
        } else {
            push_topdown_vector_figure(out, canonical, effective_stance, &col);
        }
        out.push_str("</g>");
    }

    // 4. Upright corner callsign badge + optional stance indicator (`KN` / `PR`)
    let badge = escape_html(&token_initials(&input.tok.label));
    let _ = write!(
        out,
        r##"<rect x="{}" y="{}" width="21" height="14" rx="5" fill="#fbfaf7" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><text x="{}" y="{}" text-anchor="middle" font-family="ui-sans-serif,system-ui,sans-serif" font-weight="800" font-size="9.5" fill="{BATTLEMAP_INK}">{badge}</text>"##,
        cx + 8,
        cy + 11,
        cx + 18,
        cy + 21,
    );
    if let Some(st_tag) = TokenStanceKind::from_stance_str(input.stance_pose).short_badge() {
        let _ = write!(
            out,
            r##"<rect x="{}" y="{}" width="20" height="13" rx="4" fill="#0f172a" stroke="#f8fafc" stroke-width="1"/><text x="{}" y="{}" text-anchor="middle" font-family="ui-sans-serif,system-ui,sans-serif" font-weight="800" font-size="8.5" fill="#f8fafc">{st_tag}</text>"##,
            cx - 28,
            cy + 12,
            cx - 18,
            cy + 21,
        );
    }
    out.push_str("</g>");
}

/// Legacy signature preserved for callers outside `map_2d_art_overlay`; delegates to [`push_token_2d_svg`].
#[allow(clippy::too_many_arguments)]
pub fn push_token_portrait_disc_svg(
    out: &mut String,
    idx: usize,
    tok: &TokenPlacement,
    pos_m: (f32, f32),
    preset: &str,
    color_hex: &str,
    facing_deg: f32,
    portrait_url: Option<&str>,
    is_selected: bool,
    cell_px: i32,
) {
    let _ = push_preset_portrait_silhouette;
    push_token_2d_svg(
        out,
        &TokenRenderInput {
            idx,
            tok,
            pos_m,
            preset,
            stance_pose: "standing",
            color_hex,
            facing_deg,
            portrait_url,
            is_selected,
            cell_px,
            texture_mode: BattlemapTextureMode::Painted,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_2d_renders_painted_sprites_vector_figures_stances_and_portrait_fallback() {
        let tok = TokenPlacement {
            entity_id: "tok-1".to_string(),
            label: "DAISE Trooper".to_string(),
            x: 3,
            y: 4,
            visibility: MessageVisibility::Public,
        };
        let mut painted_kneel = String::new();
        push_token_2d_svg(
            &mut painted_kneel,
            &TokenRenderInput {
                idx: 0,
                tok: &tok,
                pos_m: (3.5, 4.5),
                preset: "soldier_rifle",
                stance_pose: "crouched",
                color_hex: "#f87171",
                facing_deg: 90.0,
                portrait_url: None,
                is_selected: true,
                cell_px: 64,
                texture_mode: BattlemapTextureMode::Painted,
            },
        );
        assert!(
            painted_kneel.contains("/assets/sprites/tokens/soldier_rifle_crouched")
                && painted_kneel.contains(".webp")
        );
        assert!(painted_kneel.contains(">KN<"));
        assert!(painted_kneel.contains(">DT<"));

        let mut vector_prone = String::new();
        push_token_2d_svg(
            &mut vector_prone,
            &TokenRenderInput {
                idx: 1,
                tok: &tok,
                pos_m: (3.5, 4.5),
                preset: "sniper_rifle",
                stance_pose: "prone",
                color_hex: "#38bdf8",
                facing_deg: 0.0,
                portrait_url: None,
                is_selected: false,
                cell_px: 64,
                texture_mode: BattlemapTextureMode::ProceduralVector,
            },
        );
        assert!(!vector_prone.contains("/assets/sprites/tokens/"));
        assert!(vector_prone.contains(">PR<"));
        assert_eq!(vector_prone.matches("<g ").count(), vector_prone.matches("</g>").count());

        for (stance_str, expected_tag, expected_stem) in [
            ("walking_holstered", ">WK<", "civilian_handgun_walking_holstered"),
            ("tactical_advance", ">TA<", "civilian_handgun_tactical_advance"),
            ("sprinting", ">SP<", "civilian_handgun_sprinting"),
        ] {
            let mut painted_move = String::new();
            push_token_2d_svg(
                &mut painted_move,
                &TokenRenderInput {
                    idx: 2,
                    tok: &tok,
                    pos_m: (3.5, 4.5),
                    preset: "civilian_handgun",
                    stance_pose: stance_str,
                    color_hex: "#38bdf8",
                    facing_deg: 45.0,
                    portrait_url: None,
                    is_selected: false,
                    cell_px: 64,
                    texture_mode: BattlemapTextureMode::Painted,
                },
            );
            assert!(painted_move.contains(expected_stem));
            assert!(painted_move.contains(expected_tag));

            let mut vector_move = String::new();
            push_token_2d_svg(
                &mut vector_move,
                &TokenRenderInput {
                    idx: 3,
                    tok: &tok,
                    pos_m: (3.5, 4.5),
                    preset: "civilian_handgun",
                    stance_pose: stance_str,
                    color_hex: "#38bdf8",
                    facing_deg: 45.0,
                    portrait_url: None,
                    is_selected: false,
                    cell_px: 64,
                    texture_mode: BattlemapTextureMode::ProceduralVector,
                },
            );
            assert!(!vector_move.contains("/assets/sprites/tokens/"));
            assert!(vector_move.contains(expected_tag));
        }

        let mut portrait = String::new();
        push_token_2d_svg(
            &mut portrait,
            &TokenRenderInput {
                idx: 0,
                tok: &tok,
                pos_m: (3.5, 4.5),
                preset: "civilian_knife",
                stance_pose: "standing",
                color_hex: "#38bdf8",
                facing_deg: 0.0,
                portrait_url: Some("/api/characters/sara/assets/portrait"),
                is_selected: false,
                cell_px: 64,
                texture_mode: BattlemapTextureMode::Painted,
            },
        );
        assert!(portrait.contains(r#"<clipPath id="bm-tok-clip-0""#));
        assert!(portrait.contains(r#"href="/api/characters/sara/assets/portrait""#));
    }
}
