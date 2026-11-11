//! Pure Rust enum parsers, dropdown `<select>` option tables, and AI GM provider model defaults.

use crate::wasm_client::chat_and_ttrpg::satya_visibility_changed;
use crate::wasm_client::dom::get_element;
use satya_schema::{ChannelContext, CombatStance, MessageVisibility, SessionMode, SessionRole};
use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;

/// Parses a `<select>` option string (`"TTRPG"`, `"TTRPG_GM"`, `"TTRPG_GM_AI"`, `"TTRPG_AI_GM"`, or `"MEETING"`) into a [`SessionMode`].
pub fn parse_session_mode(raw: &str) -> SessionMode {
    match raw {
        "MEETING" => SessionMode::Meeting,
        _ => SessionMode::Ttrpg,
    }
}

/// Returns `true` when the selected business mode option enables the AI GM
/// (`"TTRPG_GM_AI"` for GM + AI + Players, or `"TTRPG_AI_GM"` for AI GM + Players).
pub fn is_ai_gm_mode(raw: &str) -> bool {
    matches!(raw, "TTRPG_GM_AI" | "TTRPG_AI_GM")
}

/// Parses a `<select>` option string into a [`SessionRole`].
pub fn parse_session_role(raw: &str) -> SessionRole {
    match raw {
        "GM" => SessionRole::Gm,
        "AI_GM" => SessionRole::AiGm,
        "HOST" => SessionRole::Host,
        "PARTICIPANT" => SessionRole::Participant,
        "OBSERVER" => SessionRole::Observer,
        _ => SessionRole::Player,
    }
}

/// Parses a `<select>` option string into a [`MessageVisibility`].
pub fn parse_visibility(raw: &str) -> MessageVisibility {
    match raw {
        "PRIVATE" => MessageVisibility::Private,
        "GM_ONLY" => MessageVisibility::GmOnly,
        "HOST_ONLY" => MessageVisibility::HostOnly,
        _ => MessageVisibility::Public,
    }
}

/// Parses a `<select>` option string into a [`ChannelContext`].
pub fn parse_channel_context(raw: &str) -> ChannelContext {
    match raw {
        "OUT_OF_CHARACTER" => ChannelContext::OutOfCharacter,
        "GM_NARRATION" => ChannelContext::GmNarration,
        "MEETING_PUBLIC" => ChannelContext::MeetingPublic,
        "MEETING_BACKCHANNEL" => ChannelContext::MeetingBackchannel,
        "SYSTEM_NOTICE" => ChannelContext::SystemNotice,
        _ => ChannelContext::InCharacter,
    }
}

/// Parses a `<select>` option string into an optional Satya TTRPG [`CombatStance`].
///
/// Accepts both the 5 canonical [`CombatStance`] stance codes (`"MANEUVER"`, `"POWER"`,
/// `"STABILITY"`, `"RANGED_AIMED"`, `"RANGED_SNAP"`) and any of the 30 canonical
/// [`satya_engine::CombatTechniqueId`] technique codes (`"grab"`, `"punch"`, `"parry"`,
/// `"feint"`, `"thrust"`, `"swing"`, `"snap_shot"`, `"deliberate_aimed_shot"`, etc.).
pub fn parse_combat_stance(raw: &str) -> Option<CombatStance> {
    match raw {
        "POWER" => return Some(CombatStance::Power),
        "MANEUVER" => return Some(CombatStance::Maneuver),
        "STABILITY" => return Some(CombatStance::Stability),
        "RANGED_AIMED" => return Some(CombatStance::RangedAimed),
        "RANGED_SNAP" => return Some(CombatStance::RangedSnap),
        "" => return None,
        _ => {}
    }
    let tech_id = satya_engine::CombatTechniqueId::from_code_str(raw)?;
    let spec = satya_engine::get_combat_technique_spec(tech_id);
    match spec.category {
        satya_engine::CombatMoveCategory::Strike => Some(CombatStance::Power),
        satya_engine::CombatMoveCategory::Intercept => Some(CombatStance::Stability),
        satya_engine::CombatMoveCategory::Control
        | satya_engine::CombatMoveCategory::Maneuver
        | satya_engine::CombatMoveCategory::RangedManeuver => Some(CombatStance::Maneuver),
        satya_engine::CombatMoveCategory::RangedShot => match tech_id {
            satya_engine::CombatTechniqueId::SnapShot
            | satya_engine::CombatTechniqueId::SuppressiveFullAuto => {
                Some(CombatStance::RangedSnap)
            }
            _ => Some(CombatStance::RangedAimed),
        },
    }
}

/// Returns the default recommended model identifier for the given AI GM provider (`ollama`, `google_genai`, `openai`, `anthropic`).
pub fn default_model_for_provider(provider: &str) -> &'static str {
    match provider {
        "google_genai" => "gemini-2.5-flash",
        "openai" => "gpt-4o",
        "anthropic" => "claude-3-7-sonnet-latest",
        _ => "gemma4:12b",
    }
}

/// Static slice of `(value, label)` pairs used to populate `<select>` dropdown elements.
pub type SelectOptionSlice = &'static [(&'static str, &'static str)];

/// Canonical color theme options (`(theme_id, label)`) rendered in `Settings -> Appearance`.
pub const APPEARANCE_THEME_OPTIONS: SelectOptionSlice = &[
    ("obsidian", "Obsidian Default (Dark Slate & Cyan)"),
    ("amber-hud", "Amber Terminal / Tactical HUD"),
    ("parchment", "Solarized / Parchment Light"),
    ("cyberpunk", "Cyberpunk Neon / Midnight Synth"),
    ("ranger", "Forest / Ranger Green"),
    ("dusk-orange", "Dusk Orange (Autumn & Purple)"),
    ("nebula", "Cosmic Nebula (Purple, Orange & Bronze)"),
    ("navy-gold", "US Navy (Deep Navy & Gold)"),
    ("marine-corps", "US Marine Corps (Forest Green & Maroon)"),
    ("lcars", "Star Trek: TNG (LCARS Okudagram)"),
    ("mil-spec", "MIL-SPEC C4ISR (Olive Drab, Stencil & NVG)"),
    ("pulp-1930s", "1930s Pulp Noir (Art Deco, Sepia & Crimson)"),
    (
        "wehrmacht",
        "WW2 Wehrmacht (Feldgrau, Panzer Grey & Crimson)",
    ),
];

/// Canonical panel frame / structural layout style options (`(style_id, label)`).
pub const APPEARANCE_STYLE_OPTIONS: SelectOptionSlice = &[
    ("default", "Theme Default Frame"),
    ("mil-spec", "MIL-SPEC C4ISR (Stencil, Grid & Bezel)"),
    ("lcars", "Starfleet LCARS (Sweep Elbows & Pills)"),
    ("pulp", "1930s Pulp Dossier (Art Deco Double-Rule)"),
    ("wehrmacht", "Command Bunker (Stark Trim & Top-Bar)"),
];

/// Canonical UI density / spacing presets (`(density_id, label)`).
pub const APPEARANCE_DENSITY_OPTIONS: SelectOptionSlice = &[
    ("default", "Default Spacing"),
    ("compact", "Compact (Tighter Padding & Gaps)"),
    ("spacious", "Spacious (Generous Padding & Gaps)"),
];

/// Canonical corner radius style options (`(corners_id, label)`).
pub const APPEARANCE_CORNERS_OPTIONS: SelectOptionSlice = &[
    ("default", "Default Rounded (8–10px)"),
    ("sharp", "Sharp / Tactical (2px)"),
    ("rounded", "Soft / Pill-Edged (16px)"),
];

/// Canonical UI font family options (`(font_id, label)`).
pub const APPEARANCE_FONT_OPTIONS: SelectOptionSlice = &[
    ("sans", "Clean Sans-Serif (Inter / System)"),
    ("mono", "Tactical Monospace (Consolas / Fira)"),
    ("serif", "Editorial Serif (Georgia / Garamond)"),
    ("lcars", "Starfleet Condensed (Antonio / LCARS)"),
    ("military", "Military Field HUD (Chakra Petch / Stencil)"),
    ("stencil", "Ordnance Stencil (Black Ops One)"),
    (
        "pulp",
        "1930s Pulp Typewriter (Courier Prime / Art Deco)",
    ),
];

/// Canonical UI font size / scale options (`(font_size_id, label)`).
pub const APPEARANCE_FONT_SIZE_OPTIONS: SelectOptionSlice = &[
    ("default", "Default (100% · 16px)"),
    ("sm", "Small (90% · 14.4px)"),
    ("lg", "Large (110% · 17.6px)"),
    ("xl", "Extra Large (120% · 19.2px)"),
];

/// Validates `raw` against `options`, returning the matching canonical key or `options[0].0`.
fn normalize_from_options(raw: &str, options: SelectOptionSlice) -> &'static str {
    let trimmed = raw.trim();
    options
        .iter()
        .find_map(|(k, _)| (*k == trimmed).then_some(*k))
        .unwrap_or(options[0].0)
}

/// Normalizes a theme identifier to one of [`APPEARANCE_THEME_OPTIONS`] (defaulting to `"obsidian"`).
pub fn normalize_theme_id(raw: &str) -> &'static str {
    normalize_from_options(raw, APPEARANCE_THEME_OPTIONS)
}

/// Normalizes a structural panel style identifier to one of [`APPEARANCE_STYLE_OPTIONS`] (defaulting to `"default"`).
pub fn normalize_style_id(raw: &str) -> &'static str {
    normalize_from_options(raw, APPEARANCE_STYLE_OPTIONS)
}

/// Normalizes a UI density identifier to one of [`APPEARANCE_DENSITY_OPTIONS`] (defaulting to `"default"`).
pub fn normalize_density_id(raw: &str) -> &'static str {
    normalize_from_options(raw, APPEARANCE_DENSITY_OPTIONS)
}

/// Normalizes a corner radius identifier to one of [`APPEARANCE_CORNERS_OPTIONS`] (defaulting to `"default"`).
pub fn normalize_corners_id(raw: &str) -> &'static str {
    normalize_from_options(raw, APPEARANCE_CORNERS_OPTIONS)
}

/// Normalizes a font family identifier to one of [`APPEARANCE_FONT_OPTIONS`] (defaulting to `"sans"`).
pub fn normalize_font_id(raw: &str) -> &'static str {
    normalize_from_options(raw, APPEARANCE_FONT_OPTIONS)
}

/// Normalizes a font size identifier to one of [`APPEARANCE_FONT_SIZE_OPTIONS`] (defaulting to `"default"`).
pub fn normalize_font_size_id(raw: &str) -> &'static str {
    normalize_from_options(raw, APPEARANCE_FONT_SIZE_OPTIONS)
}

/// Returns the valid `(value, label)` option pairs for `#session-role-select`
/// scoped to a given business mode option string (`"TTRPG"`, `"TTRPG_GM_AI"`, `"TTRPG_AI_GM"`, or `"MEETING"`).
pub fn role_options_for_mode(mode_raw: &str) -> SelectOptionSlice {
    match mode_raw {
        "MEETING" => &[
            ("HOST", "Meeting Host"),
            ("PARTICIPANT", "Meeting Participant"),
            ("OBSERVER", "Observer"),
        ],
        "TTRPG_AI_GM" => &[("PLAYER", "Player"), ("OBSERVER", "Observer")],
        _ => &[
            ("GM", "Game Master (GM)"),
            ("PLAYER", "Player"),
            ("OBSERVER", "Observer"),
        ],
    }
}

pub(crate) fn sync_role_options_for_mode(mode_raw: &str, default_role: Option<&str>) {
    let opts = role_options_for_mode(mode_raw);
    if let Some(role_sel) =
        get_element("session-role-select").and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
    {
        let prev = role_sel.value();
        let html: String = opts
            .iter()
            .map(|(val, label)| format!(r#"<option value="{val}">{label}</option>"#))
            .collect();
        role_sel.set_inner_html(&html);
        if let Some(def) = default_role
            && opts.iter().any(|(val, _)| *val == def)
        {
            role_sel.set_value(def);
        } else if opts.iter().any(|(val, _)| *val == prev) {
            role_sel.set_value(&prev);
        } else {
            role_sel.set_value(opts[0].0);
        }
    }
}

/// Returns the valid `(value, label)` option pairs for `(visibility_options, context_options)`
/// scoped to a given [`SessionMode`].
pub fn chat_options_for_mode(mode: SessionMode) -> (SelectOptionSlice, SelectOptionSlice) {
    match mode {
        SessionMode::Ttrpg => (
            &[
                ("PUBLIC", "PUBLIC (Broadcast to All)"),
                ("PRIVATE", "PRIVATE (Selected Recipients Only)"),
                ("GM_ONLY", "GM_ONLY (Secret to GM / AI-GM)"),
            ],
            &[
                ("IN_CHARACTER", "IN_CHARACTER"),
                ("OUT_OF_CHARACTER", "OUT_OF_CHARACTER"),
                ("GM_NARRATION", "GM_NARRATION"),
            ],
        ),
        SessionMode::Meeting => (
            &[
                ("PUBLIC", "PUBLIC (Broadcast to All)"),
                ("PRIVATE", "PRIVATE (Selected Recipients Only)"),
                ("HOST_ONLY", "HOST_ONLY (Meeting Host Only)"),
            ],
            &[
                ("MEETING_PUBLIC", "MEETING_PUBLIC"),
                ("MEETING_BACKCHANNEL", "MEETING_BACKCHANNEL"),
            ],
        ),
    }
}

pub(crate) fn sync_chat_options_for_mode(_mode: SessionMode) {
    satya_visibility_changed();
}
