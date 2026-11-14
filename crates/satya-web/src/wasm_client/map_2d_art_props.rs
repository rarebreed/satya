//! # Top-Down Battlemap Prop Glyphs & Organic Shape Helpers (`map_2d_art_props`)
//!
//! Hand-authored, code-only SVG glyphs for the illustrated 2D battlemap in
//! [`crate::wasm_client::map_2d_art`]. Every glyph is drawn in a local `64 × 64` cell frame centered on
//! `(0, 0)` (extent `±29`), with its **back side facing `-y` (North)** so the composer can rotate the
//! back against an adjacent wall. Colors vary deterministically per cell via a `variant` hash.
//!
//! Phase 1 maps mechanical `HalfCover` tiles onto theme-appropriate props; a later dressing layer
//! (`PropKind` in `satya-engine`) will select glyphs explicitly with real ballistic material data.

use crate::wasm_client::map_2d_art_textures::BattlemapTextureMode;
use std::f32::consts::TAU;
use std::fmt::Write as _;

/// Ink color shared by every prop outline (warm near-black, modern technical pen).
pub const BATTLEMAP_INK: &str = "#2a2622";

/// Deterministic `SplitMix64` finalizer used for per-cell art variation (never for game mechanics).
#[must_use]
pub const fn battlemap_mix_u64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Stable per-cell art hash from the map `seed`, cell `(x, y)`, and a layer `salt`.
#[must_use]
pub const fn battlemap_cell_hash(seed: i64, x: i32, y: i32, salt: u64) -> u64 {
    battlemap_mix_u64(
        (seed as u64)
            ^ ((x as u32 as u64) << 20)
            ^ ((y as u32 as u64) << 40)
            ^ salt.wrapping_mul(0x2545_F491_4F6C_DD1D),
    )
}

/// Unit-interval float `[0, 1)` derived from a hash.
#[must_use]
pub fn battlemap_unit_f32(h: u64) -> f32 {
    ((h >> 40) as f32) / ((1_u64 << 24) as f32)
}

/// Smooth closed organic blob path (`d` attribute) around `(cx, cy)` with radius `r_px`, `n` control
/// points, and `jitter_frac` radial noise. Drawn with quadratic curves through edge midpoints.
#[must_use]
pub fn battlemap_blob_path(cx: f32, cy: f32, r_px: f32, n: usize, jitter_frac: f32, variant: u64) -> String {
    let n = n.max(3);
    let mut h = variant;
    let pts: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            h = battlemap_mix_u64(h);
            let j = 1.0 - jitter_frac + battlemap_unit_f32(h) * 2.0 * jitter_frac;
            let a = (i as f32 / n as f32) * TAU + battlemap_unit_f32(h ^ 0xA5A5) * 0.35;
            (cx + r_px * j * a.cos(), cy + r_px * j * a.sin())
        })
        .collect();
    let mid = |a: (f32, f32), b: (f32, f32)| ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
    let start = mid(pts[n - 1], pts[0]);
    let mut d = format!("M{:.1} {:.1}", start.0, start.1);
    for i in 0..n {
        let p = pts[i];
        let m = mid(p, pts[(i + 1) % n]);
        let _ = write!(d, "Q{:.1} {:.1} {:.1} {:.1}", p.0, p.1, m.0, m.1);
    }
    d.push('Z');
    d
}

/// Theme-selectable top-down prop glyphs (Phase 1 depictions of `HalfCover` tiles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlemapPropKind {
    Sofa,
    Armchair,
    OfficeDesk,
    KitchenIsland,
    Planter,
    StorageCabinet,
    WoodCrate,
    PalletStack,
    DrumCluster,
    Workbench,
    IbcTote,
    HospitalBed,
    SupplyCart,
    JerseyBarrier,
    Dumpster,
    StreetBench,
    Rubble,
    OverturnedTable,
}

fn pick(variant: u64, opts: &[&'static str]) -> &'static str {
    opts[((variant >> 8) % opts.len() as u64) as usize]
}

const FABRICS: &[&str] = &["#7d8e9b", "#a0705a", "#6d7f68", "#8c7f9e", "#b49a6a", "#5f6b78"];
const WOODS: &[&str] = &["#a47b55", "#c4a27c", "#7a5c43", "#b8926a"];

/// Appends one prop glyph centered at SVG `(cx, cy)` and rotated `rot_deg` clockwise.
pub fn push_battlemap_prop(out: &mut String, kind: BattlemapPropKind, cx: f32, cy: f32, rot_deg: u16, variant: u64) {
    push_battlemap_prop_with_mode(
        out,
        kind,
        cx,
        cy,
        rot_deg,
        variant,
        BattlemapTextureMode::Painted,
    );
}

/// Mode-aware variant of [`push_battlemap_prop`] applying oak, fabric, rock, steel, ceramic, and leaf WebP patterns.
pub fn push_battlemap_prop_with_mode(
    out: &mut String,
    kind: BattlemapPropKind,
    cx: f32,
    cy: f32,
    rot_deg: u16,
    variant: u64,
    tex_mode: BattlemapTextureMode,
) {
    let painted = tex_mode.is_painted();
    let _ = write!(
        out,
        r#"<g transform="translate({cx:.1} {cy:.1}) rotate({rot_deg})" stroke="{BATTLEMAP_INK}" stroke-width="2" stroke-linejoin="round">"#
    );
    match kind {
        BattlemapPropKind::Sofa => {
            let f = pick(variant, FABRICS);
            let _ = write!(
                out,
                r##"<rect x="-28" y="-20" width="56" height="40" rx="7" fill="{f}"/><rect x="-28" y="-20" width="56" height="12" rx="5" fill="{f}"/><rect x="-26" y="-18.5" width="52" height="9" rx="4" fill="#000" fill-opacity=".16" stroke="none"/><rect x="-28" y="-14" width="9" height="34" rx="4" fill="{f}"/><rect x="19" y="-14" width="9" height="34" rx="4" fill="{f}"/><path d="M0 -7V19" fill="none" stroke-width="1.2" stroke-opacity=".55"/>"##
            );
            if painted {
                out.push_str(
                    r##"<rect x="-28" y="-20" width="56" height="40" rx="7" fill="url(#bm-fabric)" fill-opacity=".38" style="mix-blend-mode:multiply" stroke="none"/>"##,
                );
            }
        }
        BattlemapPropKind::Armchair => {
            let f = pick(variant, FABRICS);
            let side = if painted { "url(#bm-oak)" } else { "#c4a27c" };
            let _ = write!(
                out,
                r##"<rect x="-20" y="-20" width="40" height="40" rx="8" fill="{f}"/><rect x="-20" y="-20" width="40" height="11" rx="5" fill="{f}"/><rect x="-18" y="-18.5" width="36" height="8" rx="4" fill="#000" fill-opacity=".16" stroke="none"/><rect x="-20" y="-13" width="8" height="33" rx="4" fill="{f}"/><rect x="12" y="-13" width="8" height="33" rx="4" fill="{f}"/><circle cx="21" cy="21" r="6" fill="{side}" stroke-width="1.4"/>"##
            );
            if painted {
                out.push_str(
                    r##"<rect x="-20" y="-20" width="40" height="40" rx="8" fill="url(#bm-fabric)" fill-opacity=".38" style="mix-blend-mode:multiply" stroke="none"/>"##,
                );
            }
        }
        BattlemapPropKind::OfficeDesk => {
            let w = if painted { "url(#bm-oak)" } else { pick(variant, WOODS) };
            let _ = write!(
                out,
                r##"<rect x="-28" y="-27" width="56" height="27" rx="2" fill="{w}"/><rect x="-11" y="-23" width="22" height="5" rx="1" fill="#2f3439"/><rect x="-9" y="-13" width="18" height="5" rx="1" fill="#e8e6e1" stroke-width="1"/><rect x="15" y="-21" width="8" height="10" fill="#f4f1ea" stroke-width="1" transform="rotate(14 19 -16)"/><circle cx="0" cy="13" r="10" fill="#3d4247"/><path d="M-9 20Q0 27 9 20" fill="none" stroke-width="3.2" stroke-linecap="round"/>"##
            );
        }
        BattlemapPropKind::KitchenIsland => {
            let top = if painted { "url(#bm-ceramic)" } else { "#dcd8cf" };
            let sink = if painted { "url(#bm-steel)" } else { "#b7c1c7" };
            let _ = write!(
                out,
                r##"<rect x="-28" y="-17" width="56" height="34" rx="3" fill="{top}"/><rect x="-24" y="-13" width="48" height="26" rx="2" fill="none" stroke-width="1" stroke-opacity=".35"/><rect x="-12" y="-8" width="22" height="16" rx="4" fill="{sink}"/><circle cx="-1" cy="-10.5" r="2.2" fill="#8b959b" stroke-width="1"/><circle cx="18" cy="1" r="5.5" fill="#e2b04a" stroke-width="1.2"/><circle cx="17" cy="0" r="1.6" fill="#b5462f" stroke="none"/>"##,
            );
        }
        BattlemapPropKind::Planter => {
            let pot = pick(variant, &["#b06a48", "#55595d", "#d8d2c4"]);
            let leaf_d = battlemap_blob_path(0.0, 0.0, 18.0, 9, 0.22, variant);
            let hi_d = battlemap_blob_path(-4.0, -4.0, 10.0, 7, 0.25, variant ^ 0x55);
            let leaf_fill = if painted { "url(#bm-leaf-tex)" } else { "#4f7a45" };
            let _ = write!(
                out,
                r##"<circle r="21" fill="{pot}"/><path d="{leaf_d}" fill="{leaf_fill}"/><path d="{hi_d}" fill="#79a35d" fill-opacity=".58" stroke="none"/>"##
            );
        }
        BattlemapPropKind::StorageCabinet => {
            let c = if painted { "url(#bm-steel)" } else { pick(variant, &["#9aa3a8", "#b8ad98", "#6e7b84"]) };
            let _ = write!(
                out,
                r##"<rect x="-26" y="-14" width="52" height="28" rx="2" fill="{c}"/><rect x="-21" y="-9" width="11" height="14" fill="#e9e3d6" stroke-width="1"/><rect x="4" y="-7" width="15" height="11" fill="#c8a27a" stroke-width="1"/><path d="M4 -1.5H19" stroke="#e8dcc0" stroke-width="1.6"/>"##
            );
        }
        BattlemapPropKind::WoodCrate => {
            let w = if painted { "url(#bm-oak)" } else { pick(variant, &["#b98b55", "#c79a62", "#a77a48"]) };
            let _ = write!(
                out,
                r##"<rect x="-24" y="-24" width="48" height="48" rx="2" fill="{w}"/><rect x="-19" y="-19" width="38" height="38" fill="none" stroke-width="1.2" stroke-opacity=".6"/><path d="M-19 -6.5H19M-19 6.5H19" fill="none" stroke-width="1" stroke-opacity=".45"/><path d="M-19 -19L19 19" fill="none" stroke-width="2.4"/>"##
            );
        }
        BattlemapPropKind::PalletStack => {
            let b = if painted { "url(#bm-oak)" } else { pick(variant, &["#c8a27a", "#b88f63", "#d2b48c"]) };
            let base = if painted { "url(#bm-oak)" } else { "#c6a46f" };
            let _ = write!(
                out,
                r##"<rect x="-26" y="-24" width="52" height="48" rx="1" fill="{base}"/><rect x="-22" y="-20" width="21" height="19" fill="{b}"/><rect x="1" y="-20" width="21" height="19" fill="{b}"/><rect x="-22" y="1" width="44" height="19" fill="{b}"/><path d="M-11.5 -20V-1M11.5 -20V-1M-22 10.5H22" fill="none" stroke="#efe4c8" stroke-width="2.6"/><path d="M-23 -22H-6" stroke="#fff" stroke-opacity=".55" stroke-width="2"/>"##
            );
        }
        BattlemapPropKind::DrumCluster => {
            const DRUMS: [&str; 4] = ["#3f6d93", "#c9a227", "#b5462f", "#4b6b45"];
            for (i, (dx, dy)) in [(-12, -10), (12, -10), (0, 11)].into_iter().enumerate() {
                let c = DRUMS[((variant >> 12) as usize + i) % DRUMS.len()];
                let _ = write!(
                    out,
                    r##"<circle cx="{dx}" cy="{dy}" r="12" fill="{c}"/>{steel}<circle cx="{dx}" cy="{dy}" r="8.5" fill="none" stroke-width="1" stroke-opacity=".5"/><circle cx="{}" cy="{}" r="1.9" fill="{BATTLEMAP_INK}" stroke="none"/>"##,
                    dx + 4,
                    dy - 4,
                    steel = if painted {
                        format!(r#"<circle cx="{dx}" cy="{dy}" r="12" fill="url(#bm-steel)" fill-opacity=".35" style="mix-blend-mode:multiply" stroke="none"/>"#)
                    } else {
                        String::new()
                    },
                );
            }
        }
        BattlemapPropKind::Workbench => {
            let top = if painted { "url(#bm-oak)" } else { "#8a6a4a" };
            let _ = write!(
                out,
                r##"<rect x="-28" y="-15" width="56" height="30" rx="1.5" fill="{top}"/><path d="M-28 -5H28M-28 5H28" fill="none" stroke-width=".9" stroke-opacity=".35"/><rect x="18" y="-20" width="10" height="9" fill="#5b6770"/><path d="M-18 -6l10 6" stroke="#3f4448" stroke-width="3.2" stroke-linecap="round"/><circle cx="-1" cy="6" r="4" fill="#c9502f" stroke-width="1.2"/><rect x="-24" y="3" width="10" height="7" fill="#d9c38a" stroke-width="1"/>"##,
            );
        }
        BattlemapPropKind::IbcTote => {
            let cage = if painted { "url(#bm-steel)" } else { "#7f8a91" };
            let _ = write!(
                out,
                r##"<rect x="-25" y="-25" width="50" height="50" rx="3" fill="{cage}"/><rect x="-21" y="-21" width="42" height="42" rx="6" fill="#ecefe9"/><path d="M-25 -12.5H25M-25 0H25M-25 12.5H25M-12.5 -25V25M0 -25V25M12.5 -25V25" fill="none" stroke="#7f8a91" stroke-width="1.6"/><circle r="6" fill="#3a3f44"/>"##,
            );
        }
        BattlemapPropKind::HospitalBed => {
            let b = pick(variant, &["#8fb3c9", "#a9c7b4", "#c9b8d6"]);
            let frame = if painted { "url(#bm-steel)" } else { "#d9dfe3" };
            let _ = write!(
                out,
                r##"<rect x="-17" y="-29" width="34" height="58" rx="4" fill="{frame}"/><rect x="-13" y="-26" width="26" height="11" rx="4" fill="#fbfbf8"/><rect x="-17" y="-8" width="34" height="37" rx="3" fill="{b}"/>{fab}<path d="M-17 -6Q0 -1 17 -6" fill="none" stroke-width="1" stroke-opacity=".5"/><path d="M-21 -18V18M21 -18V18" stroke="#7d8790" stroke-width="2.6" stroke-linecap="round"/>"##,
                fab = if painted {
                    r#"<rect x="-17" y="-8" width="34" height="37" rx="3" fill="url(#bm-fabric)" fill-opacity=".35" style="mix-blend-mode:multiply" stroke="none"/>"#
                } else {
                    ""
                },
            );
        }
        BattlemapPropKind::SupplyCart => {
            let frame = if painted { "url(#bm-steel)" } else { "#cfd6db" };
            let _ = write!(
                out,
                r##"<rect x="-19" y="-15" width="38" height="30" rx="3" fill="{frame}"/><path d="M-23 -9V9" stroke-width="3.2" stroke-linecap="round"/><rect x="-13" y="-10" width="9" height="8" fill="#f2f2ee" stroke-width="1"/><rect x="0" y="-11" width="12" height="9" fill="#a8d0e6" stroke-width="1"/><circle cx="6" cy="7" r="4" fill="#d94b4b" stroke-width="1"/><rect x="-14" y="3" width="10" height="6" fill="#e8d8a8" stroke-width="1"/>"##,
            );
        }
        BattlemapPropKind::JerseyBarrier => {
            let fill = if painted { "url(#bm-concrete)" } else { "#cbc7bd" };
            let _ = write!(
                out,
                r##"<rect x="-29" y="-9" width="58" height="18" rx="3" fill="{fill}"/><rect x="-25" y="-4" width="50" height="8" rx="2" fill="#000" fill-opacity=".08" stroke="none"/><path d="M-29 0H29" fill="none" stroke-width="1" stroke-opacity=".5"/><rect x="-3" y="-9" width="6" height="18" fill="#e07a2e" stroke-width="1"/>"##,
            );
        }
        BattlemapPropKind::Dumpster => {
            let c = pick(variant, &["#3f6b4f", "#2f5a7a", "#6b6f3c"]);
            let _ = write!(
                out,
                r##"<rect x="-27" y="-19" width="54" height="38" rx="2.5" fill="{c}"/>{steel}<rect x="-26" y="-18" width="52" height="6" fill="#000" fill-opacity=".2" stroke="none"/><path d="M0 -19V19" fill="none" stroke-width="1.4"/><path d="M-20 4l6 3M14 -6l5 4" stroke="#8a5a2b" stroke-width="2" stroke-opacity=".6" stroke-linecap="round"/>"##,
                steel = if painted {
                    r#"<rect x="-27" y="-19" width="54" height="38" rx="2.5" fill="url(#bm-steel)" fill-opacity=".35" style="mix-blend-mode:multiply" stroke="none"/>"#
                } else {
                    ""
                },
            );
        }
        BattlemapPropKind::StreetBench => {
            let wood = if painted { "url(#bm-oak)" } else { "#8b6a48" };
            let _ = write!(
                out,
                r##"<rect x="-27" y="-9" width="54" height="18" rx="2" fill="{wood}"/><path d="M-27 -3H27M-27 3H27" fill="none" stroke-width="1" stroke-opacity=".55"/><path d="M-20 -12V12M20 -12V12" stroke="#3b3f43" stroke-width="3.2"/>"##,
            );
        }
        BattlemapPropKind::Rubble => {
            const STONES: [&str; 4] = ["#8d8a84", "#a6a29a", "#6f6c67", "#b7a58c"];
            let mut h = variant;
            for i in 0..7 {
                h = battlemap_mix_u64(h);
                let px = (battlemap_unit_f32(h) - 0.5) * 34.0;
                let py = (battlemap_unit_f32(h ^ 0x77) - 0.5) * 34.0;
                let r = 8.0 + battlemap_unit_f32(h ^ 0x1F) * 7.0;
                let d = battlemap_blob_path(px, py, r, 5, 0.35, h);
                let fill = if painted { "url(#bm-rock)" } else { STONES[i % STONES.len()] };
                let _ = write!(out, r#"<path d="{d}" fill="{fill}" stroke-width="1.5"/>"#);
            }
            out.push_str(
                r##"<circle cx="-22" cy="18" r="2" fill="#7d7a74" stroke-width="1"/><circle cx="20" cy="-21" r="1.6" fill="#7d7a74" stroke-width="1"/><circle cx="24" cy="14" r="2.4" fill="#9a968e" stroke-width="1"/>"##,
            );
        }
        BattlemapPropKind::OverturnedTable => {
            let w = if painted { "url(#bm-oak)" } else { pick(variant, WOODS) };
            let _ = write!(
                out,
                r##"<rect x="-27" y="-8" width="54" height="13" rx="2" fill="{w}"/><path d="M-27 -1.5H27" fill="none" stroke-width=".9" stroke-opacity=".4"/><path d="M-21 5V22M21 5V22" stroke="#5a4330" stroke-width="4.5" stroke-linecap="round"/><rect x="-6" y="12" width="7" height="5" fill="#f4f1ea" stroke-width="1" transform="rotate(-18 -2 14)"/>"##
            );
        }
    }
    out.push_str("</g>");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_paths_are_closed_and_deterministic() {
        let a = battlemap_blob_path(10.0, 10.0, 20.0, 8, 0.2, 42);
        let b = battlemap_blob_path(10.0, 10.0, 20.0, 8, 0.2, 42);
        assert_eq!(a, b);
        assert!(a.starts_with('M') && a.ends_with('Z'));
        assert_eq!(a.matches('Q').count(), 8);
        assert_ne!(battlemap_cell_hash(1, 2, 3, 0), battlemap_cell_hash(1, 3, 2, 0));
    }

    #[test]
    fn every_prop_glyph_emits_balanced_group() {
        for kind in [
            BattlemapPropKind::Sofa,
            BattlemapPropKind::DrumCluster,
            BattlemapPropKind::Rubble,
            BattlemapPropKind::HospitalBed,
            BattlemapPropKind::JerseyBarrier,
        ] {
            let mut s = String::new();
            push_battlemap_prop(&mut s, kind, 32.0, 32.0, 90, 7);
            assert!(s.starts_with("<g ") && s.ends_with("</g>"));
            assert_eq!(s.matches("<g").count(), s.matches("</g>").count());
        }
    }
}
