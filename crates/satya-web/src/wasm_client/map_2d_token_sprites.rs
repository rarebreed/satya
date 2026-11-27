//! # Top-Down Style C 2D Token Sprite Registry (`map_2d_token_sprites`)
//!
//! Maps the 12 near-future 2035 character, drone, and android figure presets × `{Standing, Kneeling, Prone}`
//! stances onto `29` alpha-matted Style C WebP sprites served from `/assets/sprites/tokens/*.webp`
//! when [`BattlemapTextureMode::Painted`] is active.

use crate::wasm_client::map_2d_art_props::battlemap_mix_u64;
use satya_schema::TokenStanceKind;

/// Base HTTP route prefix mounted by `satya-signaling` for Style C 2D token sprites.
pub const BATTLEMAP_TOKEN_SPRITE_BASE_URL: &str = "/assets/sprites/tokens";

/// Static catalog entry pairing `(preset, stance)` with its Style C WebP asset(s) and metric footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenSpriteSpec {
    pub preset: &'static str,
    pub stance: TokenStanceKind,
    pub variant_files: &'static [&'static str],
    pub render_w_m: f32,
    pub render_h_m: f32,
    pub is_airborne: bool,
}

impl TokenSpriteSpec {
    /// Primary WebP filename for this `(preset, stance)` entry.
    #[must_use]
    pub const fn primary_file(&self) -> &'static str {
        self.variant_files[0]
    }

    /// Deterministically selects one of the `3` visual variants for `entity_id`, keeping
    /// the same outfit/chassis across moves and Standing/Kneeling/Prone stance changes.
    #[must_use]
    pub fn variant_file_for_entity(&self, entity_id: &str) -> &'static str {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in entity_id.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        let idx = (battlemap_mix_u64(h) as usize) % self.variant_files.len();
        self.variant_files[idx]
    }
}

/// Normalizes any legacy or unknown `model_preset` slug onto one of the 12 canonical 2D figures.
#[must_use]
pub fn canonical_2d_token_preset(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "civilian_unarmed" => "civilian_unarmed",
        "civilian_handgun" => "civilian_handgun",
        "civilian_rifle" => "civilian_rifle",
        "civilian_knife" => "civilian_knife",
        "security_handgun" | "operative_light" => "security_handgun",
        "soldier_rifle" | "secops_sentry" => "soldier_rifle",
        "sniper_rifle" | "recon_mk4" => "sniper_rifle",
        "drone_quad" => "drone_quad",
        "drone_aero" => "drone_aero",
        "android_domestic" => "android_domestic",
        "android_industrial" => "android_industrial",
        "android_security" | "valkyrie_rig" | "custom_glb" => "android_security",
        _ => "civilian_unarmed",
    }
}

macro_rules! token_sprite_specs {
    ($($preset:literal : $stance:ident => $stem:literal ($w:literal x $h:literal $(, air=$air:literal)?));+ $(;)?) => {
        /// Complete `59`-pose × `3`-variant (`177` Style C WebP sprites) top-down token catalog
        /// synchronized with `src/satya_ml/texture_gen/token_catalog.py`.
        pub const BATTLEMAP_TOKEN_SPRITE_SPECS: &[TokenSpriteSpec] = &[
            $(TokenSpriteSpec {
                preset: $preset,
                stance: TokenStanceKind::$stance,
                variant_files: &[
                    concat!($stem, ".webp"),
                    concat!($stem, "_v1.webp"),
                    concat!($stem, "_v2.webp"),
                ],
                render_w_m: $w,
                render_h_m: $h,
                is_airborne: token_sprite_specs!(@air $($air)?),
            }),+
        ];
    };
    (@air $a:literal) => { $a };
    (@air) => { false };
}

token_sprite_specs! {
    // --- Civilians (11 static + 12 movement) ---
    "civilian_unarmed" : Standing => "civilian_unarmed_standing" (1.18 x 1.18);
    "civilian_unarmed" : Kneeling => "civilian_unarmed_crouched" (1.08 x 1.08);
    "civilian_unarmed" : Prone => "civilian_unarmed_prone" (1.05 x 1.62);
    "civilian_unarmed" : WalkingHolstered => "civilian_unarmed_walking_holstered" (1.18 x 1.18);
    "civilian_unarmed" : TacticalAdvance => "civilian_unarmed_tactical_advance" (1.20 x 1.22);
    "civilian_unarmed" : Sprinting => "civilian_unarmed_sprinting" (1.22 x 1.32);
    "civilian_handgun" : Standing => "civilian_handgun_standing" (1.18 x 1.18);
    "civilian_handgun" : Kneeling => "civilian_handgun_crouched" (1.08 x 1.08);
    "civilian_handgun" : Prone => "civilian_handgun_prone" (1.05 x 1.62);
    "civilian_handgun" : WalkingHolstered => "civilian_handgun_walking_holstered" (1.18 x 1.18);
    "civilian_handgun" : TacticalAdvance => "civilian_handgun_tactical_advance" (1.20 x 1.22);
    "civilian_handgun" : Sprinting => "civilian_handgun_sprinting" (1.22 x 1.32);
    "civilian_rifle" : Standing => "civilian_rifle_standing" (1.18 x 1.18);
    "civilian_rifle" : Kneeling => "civilian_rifle_crouched" (1.08 x 1.08);
    "civilian_rifle" : Prone => "civilian_rifle_prone" (1.05 x 1.62);
    "civilian_rifle" : WalkingHolstered => "civilian_rifle_walking_holstered" (1.18 x 1.18);
    "civilian_rifle" : TacticalAdvance => "civilian_rifle_tactical_advance" (1.20 x 1.22);
    "civilian_rifle" : Sprinting => "civilian_rifle_sprinting" (1.22 x 1.32);
    "civilian_knife" : Standing => "civilian_knife_standing" (1.18 x 1.18);
    "civilian_knife" : Kneeling => "civilian_knife_crouched" (1.08 x 1.08);
    "civilian_knife" : WalkingHolstered => "civilian_knife_walking_holstered" (1.18 x 1.18);
    "civilian_knife" : TacticalAdvance => "civilian_knife_tactical_advance" (1.20 x 1.22);
    "civilian_knife" : Sprinting => "civilian_knife_sprinting" (1.22 x 1.32);
    // --- Police, Security & Military (9 static + 9 movement) ---
    "security_handgun" : Standing => "security_handgun_standing" (1.18 x 1.18);
    "security_handgun" : Kneeling => "security_handgun_crouched" (1.08 x 1.08);
    "security_handgun" : Prone => "security_handgun_prone" (1.05 x 1.62);
    "security_handgun" : WalkingHolstered => "security_handgun_walking_holstered" (1.18 x 1.18);
    "security_handgun" : TacticalAdvance => "security_handgun_tactical_advance" (1.20 x 1.22);
    "security_handgun" : Sprinting => "security_handgun_sprinting" (1.22 x 1.32);
    "soldier_rifle" : Standing => "soldier_rifle_standing" (1.18 x 1.18);
    "soldier_rifle" : Kneeling => "soldier_rifle_crouched" (1.08 x 1.08);
    "soldier_rifle" : Prone => "soldier_rifle_prone" (1.05 x 1.62);
    "soldier_rifle" : WalkingHolstered => "soldier_rifle_walking_holstered" (1.18 x 1.18);
    "soldier_rifle" : TacticalAdvance => "soldier_rifle_tactical_advance" (1.20 x 1.22);
    "soldier_rifle" : Sprinting => "soldier_rifle_sprinting" (1.22 x 1.32);
    "sniper_rifle" : Standing => "sniper_rifle_standing" (1.24 x 1.24);
    "sniper_rifle" : Kneeling => "sniper_rifle_crouched" (1.14 x 1.14);
    "sniper_rifle" : Prone => "sniper_rifle_prone" (1.05 x 1.68);
    "sniper_rifle" : WalkingHolstered => "sniper_rifle_walking_holstered" (1.18 x 1.18);
    "sniper_rifle" : TacticalAdvance => "sniper_rifle_tactical_advance" (1.22 x 1.24);
    "sniper_rifle" : Sprinting => "sniper_rifle_sprinting" (1.24 x 1.34);
    // --- Drones (2) ---
    "drone_quad" : Standing => "drone_quad_standing" (0.96 x 1.16);
    "drone_aero" : Standing => "drone_aero_standing" (1.14 x 1.14, air=true);
    // --- Biped Androids (7 static + 9 movement) ---
    "android_domestic" : Standing => "android_domestic_standing" (1.18 x 1.18);
    "android_domestic" : Kneeling => "android_domestic_crouched" (1.08 x 1.08);
    "android_domestic" : WalkingHolstered => "android_domestic_walking_holstered" (1.18 x 1.18);
    "android_domestic" : TacticalAdvance => "android_domestic_tactical_advance" (1.20 x 1.22);
    "android_domestic" : Sprinting => "android_domestic_sprinting" (1.22 x 1.32);
    "android_industrial" : Standing => "android_industrial_standing" (1.22 x 1.22);
    "android_industrial" : Kneeling => "android_industrial_crouched" (1.12 x 1.12);
    "android_industrial" : WalkingHolstered => "android_industrial_walking_holstered" (1.22 x 1.22);
    "android_industrial" : TacticalAdvance => "android_industrial_tactical_advance" (1.24 x 1.24);
    "android_industrial" : Sprinting => "android_industrial_sprinting" (1.26 x 1.34);
    "android_security" : Standing => "android_security_standing" (1.20 x 1.20);
    "android_security" : Kneeling => "android_security_crouched" (1.10 x 1.10);
    "android_security" : Prone => "android_security_prone" (1.05 x 1.62);
    "android_security" : WalkingHolstered => "android_security_walking_holstered" (1.20 x 1.20);
    "android_security" : TacticalAdvance => "android_security_tactical_advance" (1.22 x 1.24);
    "android_security" : Sprinting => "android_security_sprinting" (1.24 x 1.34);
}

fn find_exact(preset: &str, stance: TokenStanceKind) -> Option<&'static TokenSpriteSpec> {
    BATTLEMAP_TOKEN_SPRITE_SPECS
        .iter()
        .find(|s| s.preset == preset && s.stance == stance && !s.variant_files.is_empty())
}

/// Resolves the best matching [`TokenSpriteSpec`] for `(preset, stance)` with graceful stance fallback:
/// `Prone -> Kneeling -> Standing`, and movement poses -> `Standing` for drones.
#[must_use]
pub fn resolve_token_sprite(preset: &str, stance: TokenStanceKind) -> Option<&'static TokenSpriteSpec> {
    let canonical = canonical_2d_token_preset(preset);
    if let Some(exact) = find_exact(canonical, stance) {
        return Some(exact);
    }
    if stance == TokenStanceKind::Prone
        && let Some(kneeling) = find_exact(canonical, TokenStanceKind::Kneeling)
    {
        return Some(kneeling);
    }
    find_exact(canonical, TokenStanceKind::Standing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::TOKEN_FIGURE_PRESETS;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn all_177_token_sprite_variants_exist_and_resolve_stances() {
        assert_eq!(BATTLEMAP_TOKEN_SPRITE_SPECS.len(), 59);
        let files: BTreeSet<&str> = BATTLEMAP_TOKEN_SPRITE_SPECS
            .iter()
            .flat_map(|s| s.variant_files.iter().copied())
            .collect();
        assert_eq!(files.len(), 177);

        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/sprites/tokens");
        for spec in BATTLEMAP_TOKEN_SPRITE_SPECS {
            assert_eq!(spec.variant_files.len(), 3);
            for file in spec.variant_files {
                assert!(file.ends_with(".webp"));
                assert!(
                    root.join(file).is_file(),
                    "missing baked token sprite {file}"
                );
            }
        }

        for meta in TOKEN_FIGURE_PRESETS {
            for stance in TokenStanceKind::ALL {
                let resolved = resolve_token_sprite(meta.preset, stance)
                    .expect("every canonical figure must resolve a sprite");
                assert_eq!(resolved.preset, meta.preset);
            }
        }

        // Entity ID hashing covers all 3 variants and keeps variant index consistent across stances.
        let picked_stand: BTreeSet<&str> = (0..32)
            .map(|i| {
                resolve_token_sprite("soldier_rifle", TokenStanceKind::Standing)
                    .unwrap()
                    .variant_file_for_entity(&format!("tok-squad-{i}"))
            })
            .collect();
        assert_eq!(picked_stand.len(), 3, "expected 3 variants: {picked_stand:?}");

        let s_stand = resolve_token_sprite("soldier_rifle", TokenStanceKind::Standing)
            .unwrap()
            .variant_file_for_entity("tok-alpha-7");
        let s_walk = resolve_token_sprite("soldier_rifle", TokenStanceKind::WalkingHolstered)
            .unwrap()
            .variant_file_for_entity("tok-alpha-7");
        let s_tac = resolve_token_sprite("soldier_rifle", TokenStanceKind::TacticalAdvance)
            .unwrap()
            .variant_file_for_entity("tok-alpha-7");
        let s_sprint = resolve_token_sprite("soldier_rifle", TokenStanceKind::Sprinting)
            .unwrap()
            .variant_file_for_entity("tok-alpha-7");
        let s_kneel = resolve_token_sprite("soldier_rifle", TokenStanceKind::Kneeling)
            .unwrap()
            .variant_file_for_entity("tok-alpha-7");
        let stand_suffix = s_stand.trim_start_matches("soldier_rifle_standing");
        assert_eq!(stand_suffix, s_walk.trim_start_matches("soldier_rifle_walking_holstered"));
        assert_eq!(stand_suffix, s_tac.trim_start_matches("soldier_rifle_tactical_advance"));
        assert_eq!(stand_suffix, s_sprint.trim_start_matches("soldier_rifle_sprinting"));
        assert_eq!(stand_suffix, s_kneel.trim_start_matches("soldier_rifle_crouched"));

        assert!(resolve_token_sprite("drone_aero", TokenStanceKind::Standing).unwrap().is_airborne);
        assert_eq!(
            resolve_token_sprite("drone_quad", TokenStanceKind::Sprinting)
                .unwrap()
                .stance,
            TokenStanceKind::Standing
        );
        assert_eq!(
            resolve_token_sprite("civilian_knife", TokenStanceKind::Prone)
                .unwrap()
                .stance,
            TokenStanceKind::Kneeling
        );
        assert_eq!(
            resolve_token_sprite("operative_light", TokenStanceKind::Standing)
                .unwrap()
                .preset,
            "security_handgun"
        );
    }
}
