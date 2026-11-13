//! # 2D Battlemap Paper/Ink Filter Styles & Token Portrait Discs (`map_2d_tokens`)
//!
//! Companion to [`crate::wasm_client::map_2d_art`] and [`crate::wasm_client::map_2d_art_overlay`].
//! Implements Phase 4 visual polish without changing `satya-engine` mechanics:
//! - Selectable paper/ink filter styles (`ModernVellum`, `CleanVector`, `Blueprint`, `FieldSketch`)
//!   persisted in `localStorage` and hashed into `battlemap_art_cache_key`.
//! - Top-down token **portrait discs** with circular `<clipPath>` medallions, safe optional
//!   `portrait_url` `<image>` rendering, per-preset vector operator busts, compass facing wedge,
//!   and corner callsign pills.

use crate::wasm_client::dom::{escape_html, local_storage_get, local_storage_set, select_value};
use crate::wasm_client::map_2d_art_overlay::{sync_battlemap_svg_host, token_initials};
use crate::wasm_client::map_2d_art_props::BATTLEMAP_INK;
use crate::wasm_client::map_3d::with_map_ui_mut;
use satya_schema::{MessageVisibility, TokenPlacement, sanitize_portrait_url};
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

/// Renders one top-down operator portrait silhouette inside the clipped circular disc.
fn push_preset_portrait_silhouette(out: &mut String, cx: i32, cy: i32, preset: &str, accent: &str) {
    // Subtle dark vignette backdrop + shoulders bust
    let _ = write!(
        out,
        r##"<circle cx="{cx}" cy="{cy}" r="18" fill="#18212d"/><circle cx="{cx}" cy="{}" r="12" fill="{accent}" fill-opacity=".22"/><path d="M{} {}c2-7 16-7 18 0v6H{}z" fill="#2b3748" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##,
        cy - 4,
        cx - 9,
        cy + 13,
        cx - 9,
    );
    match preset.trim() {
        "valkyrie_rig" => {
            // Swept valkyrie wing fins + glowing chevron visor helm
            let _ = write!(
                out,
                r##"<path d="M{} {}l-5-7 6 3zm{} {}l5-7-6 3z" fill="{accent}" stroke="{BATTLEMAP_INK}" stroke-width=".9"/><circle cx="{cx}" cy="{}" r="7.5" fill="#d8e3ee" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/><path d="M{} {}h10l-2 4h-6z" fill="{accent}" stroke="{BATTLEMAP_INK}" stroke-width=".8"/>"##,
                cx - 6, cy - 3, cx + 6, cy - 3, cy - 3, cx - 5, cy - 5,
            );
        }
        "recon_mk4" => {
            // Hooded cowl + twin emerald/cyan NVG optics
            let _ = write!(
                out,
                r##"<path d="M{} {}c0-8 16-8 16 0v5h-16z" fill="#2c3a47" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/><circle cx="{cx}" cy="{}" r="6" fill="#465666"/><circle cx="{}" cy="{}" r="2.3" fill="#34d399" stroke="{BATTLEMAP_INK}" stroke-width=".8"/><circle cx="{}" cy="{}" r="2.3" fill="#34d399" stroke="{BATTLEMAP_INK}" stroke-width=".8"/>"##,
                cx - 8, cy - 1, cy - 2, cx - 3, cy - 3, cx + 3, cy - 3,
            );
        }
        "secops_sentry" => {
            // Heavy juggernaut collar + crimson/amber tactical T-visor
            let _ = write!(
                out,
                r##"<rect x="{}" y="{}" width="16" height="15" rx="6" fill="#39404a" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><rect x="{}" y="{}" width="12" height="4" rx="1.5" fill="{accent}" stroke="{BATTLEMAP_INK}" stroke-width=".8"/><path d="M{cx} {}v5" stroke="{accent}" stroke-width="2.2" stroke-linecap="round"/>"##,
                cx - 8, cy - 11, cx - 6, cy - 6, cy - 3,
            );
        }
        "custom_glb" => {
            // Armored star/diamond crest + halo visor
            let _ = write!(
                out,
                r##"<circle cx="{cx}" cy="{}" r="7.5" fill="#334155" stroke="{BATTLEMAP_INK}" stroke-width="1.2"/><polygon points="{cx},{} {},{} {cx},{} {},{}" fill="{accent}" stroke="{BATTLEMAP_INK}" stroke-width=".9"/>"##,
                cy - 3, cy - 9, cx + 5, cy - 3, cy + 3, cx - 5, cy - 3,
            );
        }
        _ => {
            // "operative_light": headset comms band + crisp operator bust
            let _ = write!(
                out,
                r##"<circle cx="{cx}" cy="{}" r="7" fill="#c9a585" stroke="{BATTLEMAP_INK}" stroke-width="1.1"/><path d="M{} {}c1-5 12-5 14 0v-2c-1-4-13-4-14 0z" fill="#2d333b"/><path d="M{} {}a8 8 0 0 1 16 0" fill="none" stroke="{accent}" stroke-width="2" stroke-linecap="round"/><rect x="{}" y="{}" width="10" height="3.2" rx="1.2" fill="{accent}" fill-opacity=".88"/>"##,
                cy - 3, cx - 7, cy - 5, cx - 8, cy - 4, cx - 5, cy - 5,
            );
        }
    }
}

/// Renders one interactive token **portrait disc** into `#battlemap-overlay` with `<clipPath>` framing,
/// optional allowlisted `portrait_url` `<image>`, preset bust fallback, facing wedge, and initials pill.
#[allow(clippy::too_many_arguments)]
pub fn push_token_portrait_disc_svg(
    out: &mut String,
    idx: usize,
    tok: &TokenPlacement,
    preset: &str,
    color_hex: &str,
    facing_deg: f32,
    portrait_url: Option<&str>,
    is_selected: bool,
    cell_px: i32,
) {
    let (cx, cy) = (tok.x * cell_px + cell_px / 2, tok.y * cell_px + cell_px / 2);
    let col = escape_html(color_hex);
    let op = if tok.visibility == MessageVisibility::GmOnly { ".55" } else { "1" };
    let clip_id = format!("bm-tok-clip-{idx}");
    let _ = write!(
        out,
        r##"<g opacity="{op}" pointer-events="none"><ellipse cx="{}" cy="{}" rx="23" ry="21" fill="#120d08" fill-opacity=".35"/>"##,
        cx + 3,
        cy + 4
    );
    if is_selected {
        let _ = write!(
            out,
            r##"<circle cx="{cx}" cy="{cy}" r="29" fill="none" stroke="#f5b301" stroke-width="3.5" stroke-dasharray="6 4"/>"##
        );
    }
    let _ = write!(
        out,
        r##"<g transform="rotate({facing_deg:.0} {cx} {cy})"><path d="M{cx} {}l-8 10h16z" fill="{col}" stroke="{BATTLEMAP_INK}" stroke-width="1.6" stroke-linejoin="round"/></g><clipPath id="{clip_id}"><circle cx="{cx}" cy="{cy}" r="17"/></clipPath><circle cx="{cx}" cy="{cy}" r="22" fill="{col}" stroke="{BATTLEMAP_INK}" stroke-width="2"/><g clip-path="url(#{clip_id})">"##,
        cy - 33,
    );
    if let Some(safe_url) = portrait_url.and_then(sanitize_portrait_url) {
        let _ = write!(
            out,
            r##"<circle cx="{cx}" cy="{cy}" r="17.5" fill="#18212d"/><image href="{}" x="{}" y="{}" width="36" height="36" preserveAspectRatio="xMidYMid slice"/>"##,
            escape_html(&safe_url),
            cx - 18,
            cy - 18,
        );
    } else {
        push_preset_portrait_silhouette(out, cx, cy, preset, &col);
    }
    let badge = escape_html(&token_initials(&tok.label));
    let _ = write!(
        out,
        r##"</g><circle cx="{cx}" cy="{cy}" r="17.2" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.5"/><rect x="{}" y="{}" width="21" height="14" rx="5" fill="#fbfaf7" stroke="{BATTLEMAP_INK}" stroke-width="1.3"/><text x="{}" y="{}" text-anchor="middle" font-family="ui-sans-serif,system-ui,sans-serif" font-weight="800" font-size="9.5" fill="{BATTLEMAP_INK}">{badge}</text></g>"##,
        cx + 2,
        cy + 8,
        cx + 12,
        cy + 18,
    );
}
