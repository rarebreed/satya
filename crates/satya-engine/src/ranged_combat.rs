//! # Ranged Combat, Fire Modes, Recoil Bracing, Suppression, Armor & Malfunctions (`05-combat/ranged_combat_and_armor.md)
//!
//! Canonical Rust implementation of docs/ttrpg/05-combat/ranged_combat_and_armor.md:
//! - **§2.1 Ranged Fire Modes**: SingleSnapShotPointShoot, DeliberateSightedAimedShot, ControlledDoubleTapShortBurst, FullAutoBurstSuppressiveFire, BoundingShootingOnTheMove
//! - **§2.2–§2.3 Recoil Bracing ( \ge R$), Muzzle Climb Dispersion & Suppressive Fire Morale Shock ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$)**
//! - **§4.2 Effective Penetration & Impact Scaling**
//! - **§6.1–§6.2 Weapon Reliability (Rel in -4..=+4), Minor/Major Malfunctions & Preventative Maintenance**
//! - **§8.1–§8.2 Armor Profiles Catalog (8 tiers) & Penetration (PR) vs. Behind-Armor Blunt Trauma (BABT)**

use serde::{Deserialize, Serialize};

use crate::dice_probabilities::{CascadeOutcomeKind, CascadeRollOutcome};
use crate::impl_static_catalog;
use crate::primitives::StaticCatalog;
// ============================================================================
// Ranged Combat: Fire Modes, Recoil Bracing & Suppression
// ============================================================================

/// Canonical Ranged Fire Mode identifiers (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §2.1).
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RangedFireModeId {
    /// **Single Snap Shot (Point-Shoot)**: Base TC `7`, Mode Recoil `+0` (`1 rd`), prioritizes `Tempo` (`CQB`).
    SingleSnapShotPointShoot = 0,
    /// **Deliberate Sighted / Aimed Shot**: Base TC `11–12`, Mode Recoil `+1` (`1 rd`), prioritizes `Control` (called shots, weak spots).
    DeliberateSightedAimedShot = 1,
    /// **Controlled Double-Tap / Short Burst**: Base TC `9`, Mode Recoil `+2..+3` (`2–3 rds`), balanced `Control + Force` (`+1` hit tier per `2` Net Successes).
    ControlledDoubleTapShortBurst = 2,
    /// **Full-Auto Burst / Suppressive Fire**: Base TC `8–10`, Mode Recoil `+4..+6` (`5–15 rds`), prioritizes `Force` ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$).
    FullAutoBurstSuppressiveFire = 3,
    /// **Bounding / Shooting on the Move**: Base TC `10`, Mode Recoil `+3` (`1–3 rds`), unbraced recoil added to weapon `R`.
    BoundingShootingOnTheMove = 4,
}

impl RangedFireModeId {
    /// All 5 canonical ranged fire mode identifiers in declaration order.
    pub const ALL: [Self; 5] = [
        Self::SingleSnapShotPointShoot,
        Self::DeliberateSightedAimedShot,
        Self::ControlledDoubleTapShortBurst,
        Self::FullAutoBurstSuppressiveFire,
        Self::BoundingShootingOnTheMove,
    ];
}

/// Canonical profile for a Ranged Fire Mode (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §2.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangedFireModeProfile {
    /// Canonical fire mode identifier.
    pub id: RangedFireModeId,
    /// Display name from the §2.1 table.
    pub name: &'static str,
    /// Minimum Base Count-Up Tick Cost (`Base TC`).
    pub min_base_tc: i32,
    /// Maximum Base Count-Up Tick Cost (`Base TC`).
    pub max_base_tc: i32,
    /// Typical minimum total Recoil Rating (`R`).
    pub min_recoil_r: i32,
    /// Typical maximum total Recoil Rating (`R`).
    pub max_recoil_r: i32,
    /// Minimum Fire-Mode Recoil Adder ($\Delta R_{\text{mode}}$, `ranged_combat_and_armor.md` §2.1 Table).
    pub min_mode_recoil_adder: i32,
    /// Maximum Fire-Mode Recoil Adder ($\Delta R_{\text{mode}}$, `ranged_combat_and_armor.md` §2.1 Table).
    pub max_mode_recoil_adder: i32,
    /// Minimum ammunition rounds expended per trigger press (`ranged_combat_and_armor.md` §2.1 Table).
    pub min_rounds_expended: i32,
    /// Maximum ammunition rounds expended per trigger press (`ranged_combat_and_armor.md` §2.1 Table).
    pub max_rounds_expended: i32,
    /// Extra unbraced recoil penalty added when shooting on the move (`+3` for `BoundingShootingOnTheMove`).
    pub movement_recoil_adder: i32,
}

macro_rules! fm {
    ($id:ident, $name:literal, $min_tc:expr, $max_tc:expr, $min_r:expr, $max_r:expr, $min_mr:expr, $max_mr:expr, $min_rds:expr, $max_rds:expr, $move_r:expr) => {
        RangedFireModeProfile {
            id: RangedFireModeId::$id,
            name: $name,
            min_base_tc: $min_tc,
            max_base_tc: $max_tc,
            min_recoil_r: $min_r,
            max_recoil_r: $max_r,
            min_mode_recoil_adder: $min_mr,
            max_mode_recoil_adder: $max_mr,
            min_rounds_expended: $min_rds,
            max_rounds_expended: $max_rds,
            movement_recoil_adder: $move_r,
        }
    };
}

/// Canonical 5-entry Ranged Fire Mode catalog (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §2.1).
pub const RANGED_FIRE_MODES: [RangedFireModeProfile; 5] = [
    fm!(SingleSnapShotPointShoot, "Single Snap Shot (Point-Shoot)", 7, 7, 3, 4, 0, 0, 1, 1, 0),
    fm!(DeliberateSightedAimedShot, "Deliberate Sighted / Aimed Shot", 11, 12, 4, 6, 1, 1, 1, 1, 0),
    fm!(ControlledDoubleTapShortBurst, "Controlled Double-Tap / Short Burst", 9, 9, 6, 7, 2, 3, 2, 3, 0),
    fm!(FullAutoBurstSuppressiveFire, "Full-Auto Burst / Suppressive Fire", 8, 10, 8, 12, 4, 6, 5, 15, 0),
    fm!(BoundingShootingOnTheMove, "Bounding / Shooting on the Move", 10, 10, 6, 10, 3, 3, 1, 3, 3),
];

impl_static_catalog!(RangedFireModeId => RangedFireModeProfile : RANGED_FIRE_MODES);

/// Returns the canonical 5-entry Ranged Fire Mode catalog (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §2.1).
#[must_use]
pub const fn canonical_ranged_fire_modes() -> &'static [RangedFireModeProfile] {
    RangedFireModeId::CATALOG
}

/// Evaluation of a ranged weapon triad allocation against Recoil (`R`) and Suppressive Fire rules
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §2.2–§2.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangedExchangeEvaluation {
    /// Effective Recoil demand $R_{\text{eff}} = R + \text{movement\_recoil\_adder}$.
    pub effective_recoil_r: i32,
    /// Whether the shooter's allocated `Force` fully braces the weapon ($F \ge R_{\text{eff}}$).
    pub recoil_fully_braced: bool,
    /// Unbraced recoil shortfall $\max(0, R_{\text{eff}} - F)$, applied directly as a muzzle-climb dispersion penalty to $\Delta_{\text{ext}}$.
    pub unbraced_dispersion_delta_ext: i32,
    /// Suppression Morale Shock $\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$ inflicted on targets in the beaten zone (when `is_suppressive_or_full_auto == true`).
    pub suppression_morale_delta: i32,
    /// Minimum `Control` a pinned target must allocate to return fire calmly without suffering the suppression penalty (`6`).
    pub calm_return_fire_control_threshold: i32,
}

/// Evaluates Recoil Bracing ($F \ge R$) and Suppressive Fire Morale Shock ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$)
/// from `docs/ttrpg/05-combat/ranged_combat_and_armor.md` §2.2–§2.3.
pub fn evaluate_ranged_recoil_and_suppression(
    allocated_force: i32,
    weapon_recoil_r: i32,
    is_bounding_on_move: bool,
    is_suppressive_or_full_auto: bool,
) -> RangedExchangeEvaluation {
    let effective_recoil_r = weapon_recoil_r.max(0) + if is_bounding_on_move { 3 } else { 0 };
    let unbraced_dispersion_delta_ext = (effective_recoil_r - allocated_force.max(0)).max(0);
    let recoil_fully_braced = unbraced_dispersion_delta_ext == 0;
    let suppression_morale_delta = if is_suppressive_or_full_auto {
        allocated_force.max(0).div_euclid(2)
    } else {
        0
    };

    RangedExchangeEvaluation {
        effective_recoil_r,
        recoil_fully_braced,
        unbraced_dispersion_delta_ext,
        suppression_morale_delta,
        calm_return_fire_control_threshold: 6,
    }
}

/// Evaluates Ranged Recoil Bracing and Suppressive Fire using the canonical 5-gait movement model
/// from `docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md` §1.2
/// (`+0` Crawl/Creep, `+2` Combat Glide, `+4` Bounding Run).
pub fn evaluate_ranged_recoil_with_gait(
    allocated_force: i32,
    weapon_recoil_r: i32,
    movement_gait: crate::health_survival::CombatMovementGait,
    is_suppressive_or_full_auto: bool,
) -> RangedExchangeEvaluation {
    evaluate_ranged_recoil_and_suppression(
        allocated_force,
        weapon_recoil_r.max(0) + movement_gait.ranged_recoil_adder(),
        false,
        is_suppressive_or_full_auto,
    )
}

// ============================================================================
// Armor Catalog & Penetration (PR) vs. Behind-Armor Blunt Trauma (BABT)
// ============================================================================

/// Canonical Armor Profile identifiers (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §8.1).
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmorProfileId {
    /// **Heavy Leather / Synth-Weave Jacket**: `PR_Edged 2`, `PR_Ballistic 1`, `BTD 1`, `2.5 kg`.
    HeavyLeatherSynthWeaveJacket = 0,
    /// **Concealable Soft Kevlar Vest (NIJ IIIA)**: `PR_Edged 2` (`1` vs. icepick/flechette), `PR_Ballistic 4`, `BTD 2`, `2.2 kg`.
    ConcealableSoftKevlarVestNij3a = 1,
    /// **Lightweight UHMWPE Patrol Carrier (NIJ III / RF1)**: `PR_Edged 5`, `PR_Ballistic 6`, `BTD 4`, `4.8 kg`.
    LightweightUhmwpePatrolCarrierNij3 = 2,
    /// **Tactical Plate Carrier (Level IV Ceramic)**: `PR_Edged 6`, `PR_Ballistic 8`, `BTD 5`, `8.5 kg`.
    TacticalPlateCarrierLevel4 = 3,
    /// **Ballistic Combat Helmet + Visor**: `PR_Edged 5`, `PR_Ballistic 4` (`3` visor), `BTD 4`, `1.6 kg`.
    BallisticCombatHelmetVisor = 4,
    /// **Full Riot / Breacher Suit**: `PR_Edged 6` (`5` vs. icepick/flechette), `PR_Ballistic 5`, `BTD 6`, `16.0 kg`, `+6` Heat Retention.
    FullRiotBreacherSuit = 5,
    /// **2035 "Valkyrie" Recon Exo-Carapace**: `PR_Edged 8`, `PR_Ballistic 9`, `BTD 8`, `18.5 kg` (`-20.0 kg` Exo Offset).
    ValkyrieReconExoCarapace2035 = 6,
}

impl ArmorProfileId {
    /// All 7 canonical armor profile identifiers in declaration order (`ranged_combat_and_armor.md` §8.1).
    pub const ALL: [Self; 7] = [
        Self::HeavyLeatherSynthWeaveJacket,
        Self::ConcealableSoftKevlarVestNij3a,
        Self::LightweightUhmwpePatrolCarrierNij3,
        Self::TacticalPlateCarrierLevel4,
        Self::BallisticCombatHelmetVisor,
        Self::FullRiotBreacherSuit,
        Self::ValkyrieReconExoCarapace2035,
    ];

    /// Canonical machine-readable code for this armor profile.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::HeavyLeatherSynthWeaveJacket => "HEAVY_LEATHER_SYNTH_WEAVE_JACKET",
            Self::ConcealableSoftKevlarVestNij3a => "CONCEALABLE_SOFT_KEVLAR_VEST_NIJ_3A",
            Self::LightweightUhmwpePatrolCarrierNij3 => {
                "LIGHTWEIGHT_UHMWPE_PATROL_CARRIER_NIJ_3"
            }
            Self::TacticalPlateCarrierLevel4 => "TACTICAL_PLATE_CARRIER_LEVEL_4",
            Self::BallisticCombatHelmetVisor => "BALLISTIC_COMBAT_HELMET_VISOR",
            Self::FullRiotBreacherSuit => "FULL_RIOT_BREACHER_SUIT",
            Self::ValkyrieReconExoCarapace2035 => "VALKYRIE_RECON_EXO_CARAPACE_2035",
        }
    }

    /// Human-readable display label from `ranged_combat_and_armor.md` §8.1.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::HeavyLeatherSynthWeaveJacket => "Heavy Leather / Synth-Weave Jacket",
            Self::ConcealableSoftKevlarVestNij3a => "Concealable Soft Kevlar Vest (NIJ IIIA)",
            Self::LightweightUhmwpePatrolCarrierNij3 => {
                "Lightweight UHMWPE Patrol Carrier (NIJ III / RF1)"
            }
            Self::TacticalPlateCarrierLevel4 => "Tactical Plate Carrier (Level IV Ceramic)",
            Self::BallisticCombatHelmetVisor => "Ballistic Combat Helmet + Visor",
            Self::FullRiotBreacherSuit => "Full Riot / Breacher Suit",
            Self::ValkyrieReconExoCarapace2035 => "2035 \"Valkyrie\" Recon Exo-Carapace",
        }
    }

    /// Recognized shorthand aliases for this armor profile.
    #[must_use]
    pub const fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::HeavyLeatherSynthWeaveJacket => &["SYNTH_WEAVE_JACKET", "LEATHER_JACKET"],
            Self::ConcealableSoftKevlarVestNij3a => &["SOFT_KEVLAR_VEST", "NIJ_3A", "KEVLAR_3A"],
            Self::LightweightUhmwpePatrolCarrierNij3 => {
                &["UHMWPE_PATROL_CARRIER", "NIJ_3", "RF1"]
            }
            Self::TacticalPlateCarrierLevel4 => &["PLATE_CARRIER", "LEVEL_4", "NIJ_4"],
            Self::BallisticCombatHelmetVisor => &["COMBAT_HELMET", "BALLISTIC_HELMET"],
            Self::FullRiotBreacherSuit => &["RIOT_SUIT", "BREACHER_SUIT"],
            Self::ValkyrieReconExoCarapace2035 => &["VALKYRIE_EXO", "EXO_CARAPACE"],
        }
    }
}

/// Canonical Armor Profile from `docs/ttrpg/05-combat/ranged_combat_and_armor.md` §8.1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorProfile {
    /// Canonical armor identifier.
    pub id: ArmorProfileId,
    /// Display name from the §8.1 table.
    pub name: &'static str,
    /// Penetration Resistance against Edged/Stabbing attacks (`PR_Edged`).
    pub pr_edged: i32,
    /// Penetration Resistance against icepick/flechette thrusts (`1` for Soft Kevlar, `5` for Riot Suit, else equals `pr_edged`).
    pub pr_icepick_flechette: i32,
    /// Penetration Resistance against Ballistic projectiles (`PR_Ballistic`).
    pub pr_ballistic: i32,
    /// Penetration Resistance of the visor/weak spot (`3` for Ballistic Helmet Visor, else equals `pr_ballistic`).
    pub pr_visor_weak_spot: i32,
    /// Blunt Trauma Dampening (`BTD`) absorbed before Behind-Armor Blunt Trauma (`BABT`) transfers to the wearer.
    pub blunt_trauma_dampening_btd: i32,
    /// Physical weight in kilograms.
    pub weight_kg: f64,
    /// Protected / encumbered body zones (`ranged_combat_and_armor.md` §8.1 Table).
    pub encumbered_zones: &'static str,
    /// Street/retail price in 2032 USD / Credits (`ranged_combat_and_armor.md` §8.1 Table).
    pub cost_usd: i32,
    /// Powered exoskeleton carry-load offset in kilograms (`20.0 kg` for Valkyrie Exo-Carapace, else `0.0`).
    pub exo_load_offset_kg: f64,
    /// Environmental Heat Retention penalty (`+6` for Full Riot/Breacher Suit, else `0`).
    pub heat_retention_delta: i32,
}

macro_rules! ar {
    ($id:ident, $name:literal, $edged:expr, $ice:expr, $bal:expr, $vis:expr, $btd:expr, $wt:expr, $zones:literal, $cost:expr, $exo:expr, $heat:expr) => {
        ArmorProfile {
            id: ArmorProfileId::$id,
            name: $name,
            pr_edged: $edged,
            pr_icepick_flechette: $ice,
            pr_ballistic: $bal,
            pr_visor_weak_spot: $vis,
            blunt_trauma_dampening_btd: $btd,
            weight_kg: $wt,
            encumbered_zones: $zones,
            cost_usd: $cost,
            exo_load_offset_kg: $exo,
            heat_retention_delta: $heat,
        }
    };
}

/// Canonical 7-entry Armor Catalog (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §8.1).
pub const ARMOR_CATALOG: [ArmorProfile; 7] = [
    ar!(HeavyLeatherSynthWeaveJacket, "Heavy Leather / Synth-Weave Jacket", 2, 2, 1, 1, 1, 2.5, "Torso, Arms", 350, 0.0, 0),
    ar!(ConcealableSoftKevlarVestNij3a, "Concealable Soft Kevlar Vest (NIJ IIIA)", 2, 1, 4, 4, 2, 2.2, "Torso (Concealed)", 800, 0.0, 0),
    ar!(LightweightUhmwpePatrolCarrierNij3, "Lightweight UHMWPE Patrol Carrier (NIJ III / RF1)", 5, 5, 6, 6, 4, 4.8, "Torso (Vital Organs)", 1600, 0.0, 0),
    ar!(TacticalPlateCarrierLevel4, "Tactical Plate Carrier (Level IV Ceramic)", 6, 6, 8, 8, 5, 8.5, "Torso (Vital Organs)", 2400, 0.0, 0),
    ar!(BallisticCombatHelmetVisor, "Ballistic Combat Helmet + Visor", 5, 5, 4, 3, 4, 1.6, "Head, Neck, Face", 950, 0.0, 0),
    ar!(FullRiotBreacherSuit, "Full Riot / Breacher Suit", 6, 5, 5, 5, 6, 16.0, "Full Body", 4500, 0.0, 6),
    ar!(ValkyrieReconExoCarapace2035, "2035 \"Valkyrie\" Recon Exo-Carapace", 8, 8, 9, 9, 8, 18.5, "Full Body (Powered)", 45000, 20.0, 0),
];

impl_static_catalog!(ArmorProfileId => ArmorProfile : ARMOR_CATALOG);

/// Returns the canonical 7-entry Armor Catalog (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §8.1).
#[must_use]
pub const fn canonical_armor_catalog() -> &'static [ArmorProfile] {
    ArmorProfileId::CATALOG
}

/// Incoming weapon damage type for Armor `PR` vs. `BABT` resolution
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponDamageType {
    /// Standard Edged / Slashing / Stabbing weapon (`PR_Edged`).
    Edged,
    /// Narrow-point Icepick thrust or Flechette needle (`pr_icepick_flechette`).
    IcepickOrFlechette,
    /// Ballistic firearm projectile or high-velocity fragment (`PR_Ballistic`).
    Ballistic,
    /// Pure Blunt impact (`BABT = max(0, Effective Raw Impact - BTD)`).
    Blunt,
}

/// Result of resolving an attack's Penetration (`Effective Pen`) and Impact against an [`ArmorProfile`]
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §4.2 & §8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmorImpactOutcome {
    /// Effective Penetration $\text{Effective Pen} = \text{Base Pen} + \lfloor \max(0, \text{Net Successes}) / 2 \rfloor$.
    pub effective_pen: i32,
    /// Active Armor Penetration Resistance (`PR`) for the struck location and damage type.
    pub active_armor_pr: i32,
    /// Whether the attack penetrated the armor ($\text{Effective Pen} > \text{PR}$ for non-blunt attacks).
    pub penetrated: bool,
    /// Behind-Armor Blunt Trauma (`BABT`) inflicted when stopped by armor ($\max(0, \text{Effective Raw Impact} - \text{BTD})$).
    pub babt_trauma: i32,
    /// Penetrating trauma inflicted when armor is overmatched ($\max(0, \text{Effective Raw Impact} - \text{PR})$).
    pub penetrating_trauma: i32,
    /// Total effective trauma transferred to the target (`penetrating_trauma` if penetrated, else `babt_trauma`).
    pub total_effective_trauma: i32,
    /// Whether the wound causes internal/external bleeding (`true` when an Edged/Icepick/Ballistic attack penetrates).
    pub causes_bleed: bool,
    /// Armor `PR` ablation/degradation inflicted on the struck plate (`1..=2` on penetration, `1` when a Level IV Ceramic plate stops a hit with `raw_impact >= 6`, else `0`).
    pub armor_pr_degradation: i32,
}

/// Resolves **Armor Penetration (`PR`) vs. Behind-Armor Blunt Trauma (`BABT`)**
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §4.2, line 257 & §8.2, lines 453–465).
pub fn resolve_armor_impact(
    armor: &ArmorProfile,
    damage_type: WeaponDamageType,
    raw_impact: i32,
    base_pen: i32,
    net_successes: i32,
    hits_visor_weak_spot: bool,
) -> ArmorImpactOutcome {
    let bonus_margin = net_successes.max(0).div_euclid(2);
    let effective_pen = base_pen + bonus_margin;
    let effective_raw_impact = raw_impact + bonus_margin;
    let active_armor_pr = match damage_type {
        WeaponDamageType::Edged => armor.pr_edged,
        WeaponDamageType::IcepickOrFlechette => armor.pr_icepick_flechette,
        WeaponDamageType::Ballistic => {
            if hits_visor_weak_spot {
                armor.pr_visor_weak_spot
            } else {
                armor.pr_ballistic
            }
        }
        WeaponDamageType::Blunt => armor.pr_edged.max(armor.pr_ballistic),
    };

    if damage_type == WeaponDamageType::Blunt || effective_pen <= active_armor_pr {
        let babt_trauma = (effective_raw_impact - armor.blunt_trauma_dampening_btd).max(0);
        let armor_pr_degradation = if damage_type != WeaponDamageType::Blunt
            && armor.id == ArmorProfileId::TacticalPlateCarrierLevel4
            && raw_impact >= 6
        {
            1
        } else {
            0
        };
        ArmorImpactOutcome {
            effective_pen,
            active_armor_pr,
            penetrated: false,
            babt_trauma,
            penetrating_trauma: 0,
            total_effective_trauma: babt_trauma,
            causes_bleed: false,
            armor_pr_degradation,
        }
    } else {
        let penetrating_trauma = (effective_raw_impact - active_armor_pr).max(0);
        let overmatch = effective_pen - active_armor_pr;
        let armor_pr_degradation = if overmatch >= 4 { 2 } else { 1 };
        ArmorImpactOutcome {
            effective_pen,
            active_armor_pr,
            penetrated: true,
            babt_trauma: 0,
            penetrating_trauma,
            total_effective_trauma: penetrating_trauma,
            causes_bleed: true,
            armor_pr_degradation,
        }
    }
}

// ============================================================================
// Weapon & Equipment Reliability (`Rel`) & Malfunctions (§6.1 – §6.2)
// ============================================================================

/// Canonical Equipment Quality & Reliability (`Rel`) classes
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponReliabilityClass {
    /// **Improvised / Salvaged / Fouled (`Rel = -3`)**: Scrap firearms, corroded ammo, unmaintained cyber-joints.
    ImprovisedOrSalvaged,
    /// **Cheap / Street-Clone / Delicate High-Tech (`Rel = -1`)**: 3D-printed polymers, knockoff electronics.
    CheapStreetClone,
    /// **Standard Commercial / Military Issue (`Rel = 0`)**: Factory-spec service pistols, standard carbines.
    StandardCommercialOrIssue,
    /// **Rugged Military Classic (`Rel = +2`)**: Loose-tolerance battle rifles, revolvers, forged cold steel.
    RuggedMilitaryClassic,
    /// **Custom Match-Grade / Master-Tuned (`Rel = +3`)**: Hand-fitted aerospace-coated weapons maintained daily.
    CustomMatchGradeTuned,
}

impl WeaponReliabilityClass {
    /// Returns the canonical numeric Reliability rating $\text{Rel} \in [-4, +4]$.
    #[must_use]
    pub const fn reliability_rating(self) -> i32 {
        match self {
            Self::ImprovisedOrSalvaged => -3,
            Self::CheapStreetClone => -1,
            Self::StandardCommercialOrIssue => 0,
            Self::RuggedMilitaryClassic => 2,
            Self::CustomMatchGradeTuned => 3,
        }
    }
}

/// Severity of a Weapon or Equipment Malfunction (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MalfunctionKind {
    /// Weapon cycles normally (`0` clearance ticks).
    None,
    /// **Minor Malfunction (Stovepipe / Misfeed / Sensor Glitch)**:
    /// Current shot resolves, then weapon requires an `8-tick` Immediate Action (`Tap-Rack`) to clear (or drops `-1` Tool Tier).
    MinorMalfunctionStovepipe,
    /// **Major Malfunction (Double-Feed / Failure to Extract / Overheat)**:
    /// Shot fails; requires `20–32 ticks` (a full complex action) and a *Gunsmithing & Armorer* (`DEX/ANA`) check to strip and clear.
    MajorMalfunctionDoubleFeed,
}

/// Outcome of checking a cascade roll for Weapon/Equipment Malfunction
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponMalfunctionOutcome {
    /// Effective Reliability rating (`Rel`) clamped to `-4..=+4`.
    pub effective_reliability: i32,
    /// Triggered malfunction severity.
    pub malfunction_kind: MalfunctionKind,
    /// Count-Up ticks required to clear the malfunction (`0`, `8`, or `26` average of `20..=32`).
    pub clearance_ticks: i32,
    /// Tool Tier degradation if the item is a non-firearm tool (`-1` on Minor, `-2` on Major, else `0`).
    pub tool_tier_degradation: i32,
}

/// Evaluates whether a [`CascadeRollOutcome`] triggers a Weapon or Equipment Malfunction
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §6.2 & §9.2 line 530).
pub fn evaluate_weapon_malfunction(
    roll: &CascadeRollOutcome,
    reliability_rel: i32,
    environmental_fouling: i32,
) -> WeaponMalfunctionOutcome {
    let rel = reliability_rel.clamp(-4, 4);
    let fouling = environmental_fouling.max(0);

    let (malfunction_kind, clearance_ticks, tool_tier_degradation) = match roll.outcome_kind {
        CascadeOutcomeKind::SuccessWithComplication if roll.raw_botches > rel => {
            (MalfunctionKind::MinorMalfunctionStovepipe, 8, 1)
        }
        CascadeOutcomeKind::BotchFailure => {
            if roll.effective_botches + fouling > 2 + rel {
                (MalfunctionKind::MajorMalfunctionDoubleFeed, 26, 2)
            } else if roll.effective_botches + fouling > rel {
                (MalfunctionKind::MinorMalfunctionStovepipe, 8, 1)
            } else {
                (MalfunctionKind::None, 0, 0)
            }
        }
        _ => (MalfunctionKind::None, 0, 0),
    };

    WeaponMalfunctionOutcome {
        effective_reliability: rel,
        malfunction_kind,
        clearance_ticks,
        tool_tier_degradation,
    }
}

/// Applies `30 minutes` of Downtime **Preventative Maintenance** with *Gunsmithing & Armorer* (`DEX/ANA`)
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §6.2).
pub fn apply_preventative_maintenance(
    base_reliability: i32,
    _current_fouling: i32,
    armorer_successes: i32,
) -> (i32, i32) {
    let bonus = i32::from(armorer_successes >= 1);
    ((base_reliability + bonus).clamp(-4, 4), 0)
}
