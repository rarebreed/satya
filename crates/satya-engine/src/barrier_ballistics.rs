//! # Ballistic Barrier Penetration vs. Visual Concealment (`barrier_ballistics`)
//!
//! Couples spatial ray traversal (`tactical_map`, `tactical_map_edges`, `map_dressing`) directly to
//! granular firearm physics (`ballistics`, `ranged_combat` — `docs/ttrpg/05-combat/ranged_combat_and_armor.md` §4 & §8):
//! - **Concealment (`0 PR`)**: Foliage (`Hedge`, `Bush`) obscures sightlines (`+3 Δ_ext`) without stripping
//!   projectile penetration or trauma.
//! - **Light Partitions & Interior Furniture (`1–2 PR`)**: `Glass` (`1`), `Drywall` (`1`, or `3` on a `10%`
//!   `2×4` wood-stud hit), `Upholstery` (`1`), and `WoodFurniture` (`2`) stop or degrade pistol hollow-points
//!   (`9×19mm` `Pen 2`) while offering almost no ballistic protection against rifles (`5.56mm` `Pen 6`, `7.62mm` `Pen 5–7`).
//! - **Structural Wood, Sheet Metal & Vehicles (`3 PR` body / `9 PR` engine block)**: `SolidWood`, `SheetSteel`,
//!   and `VehicleBody` doors stop standard handguns (`Pen 1–3`) but are perforated by carbines and rifles; a
//!   vehicle's front engine-bay cell upgrades to `9 PR` (`ArmorSteel`), stopping `5.56×45mm` and `7.62×51mm`.
//! - **Masonry, Earth & Heavy Hardened Cover (`6–10 PR`)**: `Brick` and `ConcreteBlock` (`6 PR`) defeat handguns,
//!   shotguns (`Pen 4`), intermediate `7.62×39mm` (`Pen 5`), and un-boosted `5.56×45mm` (`Pen 6`), **but are
//!   perforated by full-power `7.62×51mm NATO` (`Pen 7`) and `6.5mm Creedmoor` (`Pen 8`)**. `ReinforcedConcrete`
//!   (`9 PR`) and `ArmorSteel` (`10 PR`) require anti-materiel calibres (`.338 Lapua` AP / `.50 BMG`).

use satya_schema::{CoverGrade, MaterialClass, PropKind, WallEdgeKind};
use serde::{Deserialize, Serialize};

use crate::ballistics::{GranularFirearmSpec, evaluate_granular_firearm_ballistics};
use crate::map_dressing::{PropObstruction, prop_spec};
use crate::ranged_combat::{
    ArmorImpactOutcome, ArmorProfile, WeaponDamageType, resolve_armor_impact,
};
use crate::tactical_map::{TacticalSceneView, evaluate_los_continuous};
use crate::tactical_map_geom::{PropRayHit, intersect_segment_edge, intersect_segment_prop};

/// Static ballistic resistance profile for one [`MaterialClass`] (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §8.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialBallisticProfile {
    pub material: MaterialClass,
    /// Barrier Penetration Resistance (`0..=10`) on the same integer scale as [`crate::ballistics::GranularBallisticsEvaluation::base_armor_pen`].
    pub barrier_pr: i32,
    /// Stronger internal structural member `PR` (`Some(3)` for `2×4` wood studs inside `Drywall`; `Some(5)` for `Appliance` compressors).
    pub stud_or_frame_pr: Option<i32>,
    /// Probability (`0..=100`) that a random shot through this material strikes its internal stud/frame.
    pub stud_hit_chance_pct: u8,
    /// Approximate kinetic energy shed (Joules) when perforating a standard thickness of this material.
    pub energy_loss_joules: f64,
    /// Post-barrier projectile tumble / keyholing added (`0..=2`).
    pub wobble_added: i32,
    /// Granular/fibrous media (`Brick`, `ConcreteBlock`, `Earth`, `LiveWood`, `ReinforcedConcrete`) penalize
    /// ultra-light `< 60 gr` low-momentum PDW darts by `-2 Pen`.
    pub is_granular_medium: bool,
}

macro_rules! mat_profiles {
    ($( $mat:ident: pr $pr:literal, frame $fpr:expr, pct $pct:literal, loss $ej:literal J, yaw $yaw:literal, granular $gran:ident );+ $(;)?) => {
        /// Ballistic barrier table in [`MaterialClass::ALL`] declaration order (indexed by `mat as usize`).
        pub const MATERIAL_BALLISTIC_PROFILES: [MaterialBallisticProfile; MaterialClass::ALL.len()] = [
            $(MaterialBallisticProfile {
                material: MaterialClass::$mat,
                barrier_pr: $pr,
                stud_or_frame_pr: $fpr,
                stud_hit_chance_pct: $pct,
                energy_loss_joules: $ej as f64,
                wobble_added: $yaw,
                is_granular_medium: $gran,
            }),+
        ];
    };
}

mat_profiles! {
    Drywall:            pr 1,  frame Some(3), pct 10, loss 80 J,   yaw 0, granular false;
    Brick:              pr 6,  frame None,    pct 0,  loss 1550 J, yaw 2, granular true;
    ConcreteBlock:      pr 6,  frame None,    pct 0,  loss 1600 J, yaw 2, granular true;
    ReinforcedConcrete: pr 9,  frame None,    pct 0,  loss 3400 J, yaw 2, granular true;
    Glass:              pr 1,  frame None,    pct 0,  loss 60 J,   yaw 1, granular false;
    SolidWood:          pr 3,  frame None,    pct 0,  loss 380 J,  yaw 1, granular false;
    WoodFurniture:      pr 2,  frame None,    pct 0,  loss 190 J,  yaw 1, granular false;
    Upholstery:         pr 1,  frame None,    pct 0,  loss 90 J,   yaw 0, granular false;
    SheetSteel:         pr 3,  frame None,    pct 0,  loss 410 J,  yaw 1, granular false;
    ArmorSteel:         pr 10, frame None,    pct 0,  loss 4200 J, yaw 2, granular false;
    Appliance:          pr 3,  frame Some(5), pct 25, loss 450 J,  yaw 1, granular false;
    Ceramic:            pr 2,  frame None,    pct 0,  loss 210 J,  yaw 1, granular false;
    Earth:              pr 7,  frame None,    pct 0,  loss 2100 J, yaw 2, granular true;
    Foliage:            pr 0,  frame None,    pct 0,  loss 5 J,    yaw 0, granular false;
    LiveWood:           pr 7,  frame None,    pct 0,  loss 2000 J, yaw 2, granular true;
    VehicleBody:        pr 3,  frame None,    pct 0,  loss 420 J,  yaw 1, granular false;
}

/// Ballistic resistance profile for `material`.
#[must_use]
pub const fn material_ballistic_profile(material: MaterialClass) -> &'static MaterialBallisticProfile {
    &MATERIAL_BALLISTIC_PROFILES[material as usize]
}

/// One physical barrier (thin wall/door/window edge, multi-cell prop, or legacy cover/wall cell)
/// intersected along a shot ray.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterceptedBarrier {
    pub label: String,
    pub material: MaterialClass,
    pub barrier_pr: i32,
    pub stud_or_frame_pr: Option<i32>,
    pub stud_hit_chance_pct: u8,
    pub blocks_sight: bool,
}

/// High-level terminal outcome of firing a specific firearm across the barriers on a sightline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BarrierPenetrationVerdict {
    /// Open sightline with zero intervening cover or foliage.
    Unobstructed,
    /// Visual concealment only (`0 PR` foliage/smoke): `+3 Δ_ext` to hit, full impact on hit.
    ConcealmentOnly,
    /// Intervening cover is heavily overmatched (`Pen - ΣPR >= 3`, e.g. `5.56mm` vs. sofa/desk).
    OvermatchedCover,
    /// Projectile perforates all intervening barriers with reduced residual penetration and trauma.
    PenetratesWithLoss,
    /// At least one intervening barrier arrests the projectile (`Effective Pen <= Barrier PR`).
    StoppedByBarrier,
}

impl BarrierPenetrationVerdict {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unobstructed => "Clear Trajectory",
            Self::ConcealmentOnly => "Concealment Only (0 PR)",
            Self::OvermatchedCover => "Overmatches Cover",
            Self::PenetratesWithLoss => "Perforates Barrier",
            Self::StoppedByBarrier => "Stopped by Barrier",
        }
    }
}

/// Spatial and weapon inputs for evaluating a firearm shot across continuous tactical map cover and barriers.
#[derive(Debug, Clone, Copy)]
pub struct BallisticShotQuery<'a> {
    pub scene: &'a TacticalSceneView<'a>,
    pub attacker_m: (f32, f32),
    pub target_m: (f32, f32),
    pub firearm: &'a GranularFirearmSpec,
    pub net_successes: i32,
    pub hit_frame_or_stud: bool,
}

/// Complete range, concealment, barrier perforation, and residual terminal-ballistics evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BallisticShotEvaluation {
    pub weapon_name: String,
    pub distance_m: f32,
    pub sight_cover_grade: CoverGrade,
    pub sight_delta_ext: i32,
    /// Recommended $\Delta_{\text{ext}}$ accounting for weapon overmatch vs. soft furniture (`+2`) or
    /// blind wall-bang fire through penetrable opaque partitions (`+6`, vs. `+16` when truly unpenetrable).
    pub effective_delta_ext: i32,
    /// Whether the target can be harmed (either via exposed Low/High Cover sightline or barrier perforation).
    pub can_reach_target: bool,
    /// `true` when visual LoS is `Blocked`, requiring blind/known-cell fire through a penetrable barrier.
    pub is_blind_fire: bool,
    pub verdict: BarrierPenetrationVerdict,
    pub initial_pen: i32,
    pub initial_impact: i32,
    pub residual_pen: i32,
    pub residual_impact: i32,
    pub residual_energy_joules: f64,
    pub post_barrier_wobble: i32,
    /// Chance (`0..=100`) that an otherwise-penetrating shot is stopped by an internal stud/compressor.
    pub stud_stop_risk_pct: u8,
    pub barriers: Vec<InterceptedBarrier>,
    pub summary_label: String,
}

impl BallisticShotEvaluation {
    /// Chains post-barrier residual penetration and impact directly into [`resolve_armor_impact`] for an
    /// armored target behind the barrier (`None` when `residual_impact == 0` because the barrier stopped the round).
    #[must_use]
    pub fn resolve_against_target_armor(
        &self,
        armor: &ArmorProfile,
        hits_visor_weak_spot: bool,
    ) -> Option<ArmorImpactOutcome> {
        if self.residual_impact <= 0 {
            return None;
        }
        Some(resolve_armor_impact(
            armor,
            WeaponDamageType::Ballistic,
            self.residual_impact,
            self.residual_pen,
            0,
            hits_visor_weak_spot,
        ))
    }
}

fn edge_barrier_entry(kind: WallEdgeKind, material: MaterialClass) -> InterceptedBarrier {
    let prof = material_ballistic_profile(material);
    let label = match kind {
        WallEdgeKind::Wall => format!("{} wall", material.label()),
        WallEdgeKind::DoorClosed => format!("Closed {} door", material.label()),
        WallEdgeKind::Window => "Window pane / sill".to_string(),
        WallEdgeKind::DoorOpen => "Open doorway".to_string(),
    };
    InterceptedBarrier {
        label,
        material,
        barrier_pr: prof.barrier_pr,
        stud_or_frame_pr: prof.stud_or_frame_pr,
        stud_hit_chance_pct: prof.stud_hit_chance_pct,
        blocks_sight: kind.blocks_sight(),
    }
}

fn prop_barrier_entry(kind: PropKind, hit: PropRayHit) -> InterceptedBarrier {
    let prof = material_ballistic_profile(hit.material);
    let (pr, label) = if hit.is_engine_bay {
        (9, format!("{} (engine block)", kind.label()))
    } else if kind == PropKind::LargeTree && hit.is_tree_trunk {
        (prof.barrier_pr + 2, format!("{} (massive trunk)", kind.label()))
    } else {
        (prof.barrier_pr, kind.label().to_string())
    };
    InterceptedBarrier {
        label,
        material: hit.material,
        barrier_pr: pr,
        stud_or_frame_pr: prof.stud_or_frame_pr,
        stud_hit_chance_pct: prof.stud_hit_chance_pct,
        blocks_sight: hit.obstruction == PropObstruction::FullObstacle,
    }
}

/// Collects all thin edge barriers and oriented `360°` props intersected analytically along
/// `attacker_m -> target_m`, sorted from muzzle to target.
#[must_use]
pub fn collect_ray_barriers(
    scene: &TacticalSceneView<'_>,
    attacker_m: (f32, f32),
    target_m: (f32, f32),
) -> Vec<InterceptedBarrier> {
    let seg_len = ((target_m.0 - attacker_m.0).powi(2) + (target_m.1 - attacker_m.1).powi(2)).sqrt();
    if seg_len <= 0.05 {
        return Vec::new();
    }

    type OrderedBarrierHit = (f32, Option<(WallEdgeKind, MaterialClass)>, InterceptedBarrier);
    let mut ordered: Vec<OrderedBarrierHit> = Vec::new();
    for edge in scene.edges {
        if matches!(edge.kind, WallEdgeKind::DoorOpen) {
            continue;
        }
        if let Some(t) = intersect_segment_edge(attacker_m, target_m, edge) {
            let dist_m = t * seg_len;
            if matches!(edge.kind, WallEdgeKind::Window) && dist_m <= 0.85 {
                continue;
            }
            ordered.push((
                dist_m,
                Some((edge.kind, edge.material)),
                edge_barrier_entry(edge.kind, edge.material),
            ));
        }
    }

    for p in scene.props {
        if let Some(hit) = intersect_segment_prop(p, attacker_m, target_m) {
            let enter_m = hit.t_enter * seg_len;
            let braced_skip = hit.obstruction != PropObstruction::FullObstacle
                && enter_m <= 0.85
                && hit.t_exit < 0.99;
            if braced_skip {
                continue;
            }
            let _ = prop_spec(p.kind);
            ordered.push((enter_m, None, prop_barrier_entry(p.kind, hit)));
        }
    }

    ordered.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut out = Vec::with_capacity(ordered.len());
    let mut last_edge: Option<(f32, WallEdgeKind, MaterialClass)> = None;
    for (dist_m, edge_tag, entry) in ordered {
        if let Some((ek, em)) = edge_tag {
            if let Some((prev_d, pk, pm)) = last_edge
                && (dist_m - prev_d).abs() < 0.08
                && pk == ek
                && pm == em
            {
                continue;
            }
            last_edge = Some((dist_m, ek, em));
        }
        out.push(entry);
    }
    out
}

/// Evaluates concealment, multi-barrier ballistic perforation, overmatch cover degradation, and residual
/// terminal impact for `query`.
#[must_use]
pub fn evaluate_ballistic_shot(query: BallisticShotQuery<'_>) -> BallisticShotEvaluation {
    let los = evaluate_los_continuous(query.scene, query.attacker_m, query.target_m);
    let bal = evaluate_granular_firearm_ballistics(query.firearm, f64::from(los.distance_m));
    let bonus_margin = query.net_successes.max(0).div_euclid(2);
    let initial_pen = bal.base_armor_pen + bonus_margin;
    let initial_impact = bal.raw_ballistic_impact + bonus_margin;

    // When corner-peeking clears a solid pillar (`CoverGrade::HighCover`), no barrier blocks the leaned ray.
    let barriers = if los.cover_grade == CoverGrade::HighCover {
        Vec::new()
    } else {
        collect_ray_barriers(query.scene, query.attacker_m, query.target_m)
    };

    let mut cur_pen = initial_pen;
    let mut cur_impact = initial_impact;
    let mut cur_energy = bal.impact_energy_joules;
    let mut wobble = bal.wobble_yaw_factor;
    let mut total_pr_spent = 0;
    let mut stud_stop_risk_pct = 0_u8;
    let mut stopped_at: Option<String> = None;

    for b in &barriers {
        let prof = material_ballistic_profile(b.material);
        let active_pr = if query.hit_frame_or_stud {
            b.stud_or_frame_pr.unwrap_or(b.barrier_pr)
        } else {
            b.barrier_pr
        };
        if active_pr == 0 {
            cur_energy = (cur_energy - prof.energy_loss_joules).max(0.0);
            continue;
        }
        let granular_pen_mod = if prof.is_granular_medium
            && query.firearm.ammo.bullet_mass_grains < 60.0
            && cur_energy < 900.0
        {
            -2
        } else {
            0
        };
        let eff_pen_vs_barrier = cur_pen + granular_pen_mod;
        if !query.hit_frame_or_stud
            && let Some(frame_pr) = b.stud_or_frame_pr
            && eff_pen_vs_barrier > active_pr
            && eff_pen_vs_barrier <= frame_pr
        {
            stud_stop_risk_pct = stud_stop_risk_pct.max(b.stud_hit_chance_pct);
        }
        if eff_pen_vs_barrier <= active_pr {
            cur_pen = 0;
            cur_impact = 0;
            cur_energy = 0.0;
            stopped_at = Some(format!("{} (PR {active_pr})", b.label));
            break;
        }
        cur_pen = eff_pen_vs_barrier - active_pr;
        cur_impact = (cur_impact - active_pr).max(1);
        cur_energy = (cur_energy - prof.energy_loss_joules).max(25.0);
        wobble = (wobble + prof.wobble_added).min(3);
        total_pr_spent += active_pr;
    }

    let is_blind_fire = !los.has_los;
    let verdict = if stopped_at.is_some() {
        BarrierPenetrationVerdict::StoppedByBarrier
    } else if barriers.is_empty() {
        BarrierPenetrationVerdict::Unobstructed
    } else if total_pr_spent == 0 {
        BarrierPenetrationVerdict::ConcealmentOnly
    } else if initial_pen - total_pr_spent >= 3 && !is_blind_fire {
        BarrierPenetrationVerdict::OvermatchedCover
    } else {
        BarrierPenetrationVerdict::PenetratesWithLoss
    };

    let can_reach_target = los.has_los || stopped_at.is_none();
    let effective_delta_ext = match (los.cover_grade, verdict) {
        (CoverGrade::Blocked, BarrierPenetrationVerdict::StoppedByBarrier) => 16,
        (CoverGrade::Blocked, _) => 6,
        (CoverGrade::LowCover, BarrierPenetrationVerdict::OvermatchedCover) => 2,
        (CoverGrade::LowCover, BarrierPenetrationVerdict::PenetratesWithLoss) => 3,
        _ => los.recommended_delta_ext,
    };

    let summary_label = match (&stopped_at, verdict) {
        (Some(who), _) if los.has_los => format!(
            "Stopped by {who} — exposed zone only (+{} Δ_ext)",
            effective_delta_ext
        ),
        (Some(who), _) => format!("Blocked — stopped by {who}"),
        (None, BarrierPenetrationVerdict::Unobstructed) => {
            format!("Clear trajectory (Impact {cur_impact}, Pen {cur_pen})")
        }
        (None, BarrierPenetrationVerdict::ConcealmentOnly) => format!(
            "Concealment only (0 PR · full Impact {cur_impact}, +{effective_delta_ext} Δ_ext)"
        ),
        (None, _) => {
            let stud_note = if stud_stop_risk_pct > 0 {
                format!(", {stud_stop_risk_pct}% frame-stop risk")
            } else {
                String::new()
            };
            let blind = if is_blind_fire { "Blind fire · " } else { "" };
            format!(
                "{blind}{} → Res Impact {cur_impact}, Pen {cur_pen} (+{effective_delta_ext} Δ_ext{stud_note})",
                verdict.label()
            )
        }
    };

    BallisticShotEvaluation {
        weapon_name: query.firearm.name.to_string(),
        distance_m: los.distance_m,
        sight_cover_grade: los.cover_grade,
        sight_delta_ext: los.recommended_delta_ext,
        effective_delta_ext,
        can_reach_target,
        is_blind_fire,
        verdict,
        initial_pen,
        initial_impact,
        residual_pen: cur_pen,
        residual_impact: cur_impact,
        residual_energy_joules: (cur_energy * 10.0).round() / 10.0,
        post_barrier_wobble: wobble,
        stud_stop_risk_pct,
        barriers,
        summary_label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ballistics::lookup_firearm_preset;
    use crate::map_dressing::{make_prop, make_prop_at_cm};
    use crate::ranged_combat::{ArmorProfileId, canonical_armor_catalog};
    use satya_schema::{EdgeSide, MapWallEdge};

    fn shot_across_prop(kind: PropKind, rot_deg: i32, target_xy: (i32, i32), weapon_id: &str) -> BallisticShotEvaluation {
        let prop = make_prop(kind, 6, 4, rot_deg, None, 0);
        let props = [prop];
        let scene = TacticalSceneView { width_m: 14.0, height_m: 10.0, edges: &[], props: &props };
        evaluate_ballistic_shot(BallisticShotQuery {
            scene: &scene,
            attacker_m: (2.5, 4.5),
            target_m: (target_xy.0 as f32 + 0.5, target_xy.1 as f32 + 0.5),
            firearm: lookup_firearm_preset(weapon_id).expect(weapon_id),
            net_successes: 0,
            hit_frame_or_stud: false,
        })
    }

    #[test]
    fn material_catalog_matches_wire_enum_and_foliage_is_concealment_only() {
        for (i, m) in MaterialClass::ALL.iter().enumerate() {
            assert_eq!(MATERIAL_BALLISTIC_PROFILES[i].material, *m);
        }
        let hedge_9mm = shot_across_prop(PropKind::Hedge, 0, (10, 4), "sig_p320_9mm");
        assert_eq!(hedge_9mm.sight_cover_grade, CoverGrade::Concealment);
        assert_eq!(hedge_9mm.effective_delta_ext, 3);
        assert_eq!(hedge_9mm.verdict, BarrierPenetrationVerdict::ConcealmentOnly);
        assert_eq!(hedge_9mm.residual_impact, hedge_9mm.initial_impact);
        assert_eq!(hedge_9mm.residual_pen, hedge_9mm.initial_pen);
    }

    #[test]
    fn furniture_and_drywall_grade_handguns_vs_long_arms_and_stud_hits() {
        // Desk (WoodFurniture PR 2): stops 9x19mm JHP (Pen 2), overmatched by 5.56mm (Pen 6 -> +2 Δ_ext)!
        let desk_9mm = shot_across_prop(PropKind::Desk, 0, (10, 4), "sig_p320_9mm");
        assert_eq!(desk_9mm.verdict, BarrierPenetrationVerdict::StoppedByBarrier);
        assert_eq!(desk_9mm.residual_impact, 0);
        assert_eq!(desk_9mm.effective_delta_ext, 4);

        let desk_556 = shot_across_prop(PropKind::Desk, 0, (10, 4), "hk416_carbine_556");
        assert_eq!(desk_556.verdict, BarrierPenetrationVerdict::OvermatchedCover);
        assert_eq!(desk_556.residual_impact, 4);
        assert_eq!(desk_556.effective_delta_ext, 2);

        // Sofa (Upholstery PR 1): degrades 9mm (-1 impact), overmatched by 5.56mm!
        let sofa_9mm = shot_across_prop(PropKind::Sofa, 0, (10, 4), "sig_p320_9mm");
        assert_eq!(sofa_9mm.verdict, BarrierPenetrationVerdict::PenetratesWithLoss);
        assert_eq!(sofa_9mm.residual_impact, 5);
        let sofa_556 = shot_across_prop(PropKind::Sofa, 0, (10, 4), "hk416_carbine_556");
        assert_eq!(sofa_556.verdict, BarrierPenetrationVerdict::OvermatchedCover);
        assert_eq!(sofa_556.residual_impact, 5);

        // Thin Drywall partition (PR 1 cavity vs. PR 3 2x4 wood stud):
        let wall_edges: Vec<MapWallEdge> = (0..8)
            .map(|y| MapWallEdge {
                x: 6,
                y,
                side: EdgeSide::West,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::Drywall,
            })
            .collect();
        let scene = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &wall_edges, props: &[] };
        let p320 = lookup_firearm_preset("sig_p320_9mm").unwrap();
        let hk416 = lookup_firearm_preset("hk416_carbine_556").unwrap();

        let cavity_9mm = evaluate_ballistic_shot(BallisticShotQuery {
            scene: &scene, attacker_m: (2.5, 4.5), target_m: (9.5, 4.5),
            firearm: p320, net_successes: 0, hit_frame_or_stud: false,
        });
        assert!(cavity_9mm.is_blind_fire && cavity_9mm.can_reach_target);
        assert_eq!(cavity_9mm.effective_delta_ext, 6);
        assert_eq!(cavity_9mm.residual_impact, 5);
        assert_eq!(cavity_9mm.stud_stop_risk_pct, 10);

        let stud_9mm = evaluate_ballistic_shot(BallisticShotQuery {
            scene: &scene, attacker_m: (2.5, 4.5), target_m: (9.5, 4.5),
            firearm: p320, net_successes: 0, hit_frame_or_stud: true,
        });
        assert!(!stud_9mm.can_reach_target && stud_9mm.residual_impact == 0);

        let stud_556 = evaluate_ballistic_shot(BallisticShotQuery {
            scene: &scene, attacker_m: (2.5, 4.5), target_m: (9.5, 4.5),
            firearm: hk416, net_successes: 0, hit_frame_or_stud: true,
        });
        assert!(stud_556.can_reach_target && stud_556.residual_impact == 3);
    }

    #[test]
    fn brick_walls_stop_556_and_handguns_but_are_defeated_by_762_nato_and_65_cm() {
        let brick_edges: Vec<MapWallEdge> = (0..8)
            .map(|y| MapWallEdge {
                x: 6,
                y,
                side: EdgeSide::West,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::Brick,
            })
            .collect();
        let scene = TacticalSceneView { width_m: 12.0, height_m: 8.0, edges: &brick_edges, props: &[] };
        let eval = |wid: &str| {
            evaluate_ballistic_shot(BallisticShotQuery {
                scene: &scene,
                attacker_m: (2.5, 4.5),
                target_m: (9.5, 4.5),
                firearm: lookup_firearm_preset(wid).unwrap(),
                net_successes: 0,
                hit_frame_or_stud: false,
            })
        };

        for stopped_id in ["sig_p320_9mm", "mp7_pdw_57", "benelli_m4_12ga", "akm_762x39", "hk416_carbine_556"] {
            let r = eval(stopped_id);
            assert_eq!(r.verdict, BarrierPenetrationVerdict::StoppedByBarrier, "{stopped_id} vs Brick");
            assert!(!r.can_reach_target && r.effective_delta_ext == 16);
        }

        let nato_762 = eval("scar_h_762x51");
        assert_eq!(nato_762.verdict, BarrierPenetrationVerdict::PenetratesWithLoss);
        assert!(nato_762.can_reach_target && nato_762.residual_impact == 3 && nato_762.residual_pen == 1);
        let vest = &canonical_armor_catalog()[ArmorProfileId::ConcealableSoftKevlarVestNij3a as usize];
        let armor_out = nato_762.resolve_against_target_armor(vest, false).unwrap();
        assert!(!armor_out.penetrated && armor_out.babt_trauma == 1);

        let cm_65 = eval("ai_atx_65cm");
        assert!(cm_65.can_reach_target && cm_65.residual_pen == 2);

        let jersey_762 = shot_across_prop(PropKind::JerseyBarrier, 90, (10, 4), "scar_h_762x51");
        assert_eq!(jersey_762.verdict, BarrierPenetrationVerdict::StoppedByBarrier);
    }

    #[test]
    fn vehicle_cabin_doors_perforate_under_rifle_fire_while_engine_block_stops_762_nato() {
        let car = make_prop(PropKind::Car, 6, 3, 0, None, 0);
        let props = [car];
        let scene = TacticalSceneView { width_m: 14.0, height_m: 12.0, edges: &[], props: &props };

        let door_9mm = evaluate_ballistic_shot(BallisticShotQuery {
            scene: &scene, attacker_m: (2.5, 5.5), target_m: (11.5, 5.5),
            firearm: lookup_firearm_preset("sig_p320_9mm").unwrap(), net_successes: 0, hit_frame_or_stud: false,
        });
        assert_eq!(door_9mm.verdict, BarrierPenetrationVerdict::StoppedByBarrier);

        let door_556 = evaluate_ballistic_shot(BallisticShotQuery {
            scene: &scene, attacker_m: (2.5, 5.5), target_m: (11.5, 5.5),
            firearm: lookup_firearm_preset("hk416_carbine_556").unwrap(), net_successes: 0, hit_frame_or_stud: false,
        });
        assert_eq!(door_556.verdict, BarrierPenetrationVerdict::OvermatchedCover);
        assert_eq!(door_556.barriers.len(), 1);
        assert_eq!(door_556.residual_impact, 3);

        let engine_762 = evaluate_ballistic_shot(BallisticShotQuery {
            scene: &scene, attacker_m: (2.5, 7.5), target_m: (11.5, 7.5),
            firearm: lookup_firearm_preset("scar_h_762x51").unwrap(), net_successes: 0, hit_frame_or_stud: false,
        });
        assert_eq!(engine_762.verdict, BarrierPenetrationVerdict::StoppedByBarrier);
        assert_eq!(engine_762.barriers[0].barrier_pr, 9);

        // Angled 30° parked car still identifies ArmorSteel engine bay accurately.
        let angled_car = make_prop_at_cm(PropKind::Car, 700, 600, 30, None, 0);
        let angled_props = [angled_car];
        let angled_scene = TacticalSceneView { width_m: 14.0, height_m: 12.0, edges: &[], props: &angled_props };
        let rad = 30.0_f32.to_radians();
        let ey = 6.0 + rad.cos() * 2.0;
        let angled_engine = evaluate_ballistic_shot(BallisticShotQuery {
            scene: &angled_scene, attacker_m: (2.0, ey), target_m: (12.0, ey),
            firearm: lookup_firearm_preset("scar_h_762x51").unwrap(), net_successes: 0, hit_frame_or_stud: false,
        });
        assert_eq!(angled_engine.verdict, BarrierPenetrationVerdict::StoppedByBarrier);
        assert_eq!(angled_engine.barriers[0].barrier_pr, 9);
    }
}
