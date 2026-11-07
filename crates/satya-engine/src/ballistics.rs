//! # Granular Firearm Engineering, Ammunition Physics & Terminal Ballistics (`05-combat`)
//!
//! Canonical Rust implementation of `docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3–§7:
//! - **§3.1–§3.2 Ammunition Physics**: Cartridge chemical energy $E_{\text{chem}}$ (§3.1.1, line 127),
//!   barrel efficiency $\eta_{\text{bbl}}$ (§3.1.2, line 136), action gas retention $\eta_{\text{action}}$ (§3.1.3, line 153),
//!   and aerodynamic energy retention $E_k(d) = E_{\text{muzzle}} e^{-d/D_{\text{decay}}}$ (§3.1.4, line 168).
//! - **§3.3 & §5.1–§5.2 Firearm Subsystems**: Feed reliability (§3.3, lines 178–191), operating action mechanics
//!   (§5.1, lines 287–298), recoil impulse & center of gravity (§5.2.1–§5.2.2, lines 302–329), and barrel length,
//!   quality & mounting (§5.2.3–§5.2.4, lines 331–354).
//! - **§4.1–§4.2 Terminal Ballistics**: Logarithmic Kinetic Energy Index $I_{\text{KE}}$ (§4.1.1, lines 206–222),
//!   Caliber Crush Index $I_{\text{cal}}$ (§4.1.2, lines 224–228), Inverse-Accuracy Projectile Wobble / Yaw
//!   $W_{\text{yaw}}$ (§4.1.3, lines 230–250), Raw Ballistic Impact (§4.2, line 255), and Base Armor Penetration (§4.2, line 263).
//! - **§6.1–§6.2 Reliability, Tolerances, Jam Clearance & Strip Time**: Machining tolerance $\tau_{\text{tol}}$ vs.
//!   accuracy/reliability tradeoff (§6.1, lines 358–375) and malfunction thresholds (§6.2, lines 377–400).

use serde::{Deserialize, Serialize};

/// Canonical Ammunition Caliber identifiers (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3.2 Table, lines 172–176).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmmoCaliberId {
    /// `9×19mm Parabellum` (`9.0 mm`, `124 gr` bullet, `5.2 gr` powder, $L_{\text{opt}} = 12\text{ cm}$, $E_{\text{chem}} = 515\text{ J}$).
    Parabellum9x19,
    /// `.45 ACP` (`11.43 mm`, `230 gr` bullet, `5.6 gr` powder, $L_{\text{opt}} = 12.7\text{ cm}$, $E_{\text{chem}} = 505\text{ J}$).
    Acp45,
    /// `5.7×28mm` (`5.7 mm`, `40 gr` bullet, `7.2 gr` powder, $L_{\text{opt}} = 26\text{ cm}$, $E_{\text{chem}} = 540\text{ J}$).
    Fn57x28,
    /// `.357 Magnum` (`9.1 mm`, `158 gr` bullet, `14.5 gr` powder, $L_{\text{opt}} = 15\text{ cm}$, $E_{\text{chem}} = 980\text{ J}$).
    Magnum357,
    /// `5.56×45mm NATO` (`5.56 mm`, `62 gr` bullet, `27.0 gr` powder, $L_{\text{opt}} = 50.8\text{ cm}$, $E_{\text{chem}} = 1795\text{ J}$).
    Nato556x45,
    /// `7.62×39mm` (`7.62 mm`, `123 gr` bullet, `25.5 gr` powder, $L_{\text{opt}} = 41.5\text{ cm}$, $E_{\text{chem}} = 2100\text{ J}$).
    Soviet762x39,
    /// `6.5mm Creedmoor` (`6.72 mm`, `140 gr` bullet, `42.5 gr` powder, $L_{\text{opt}} = 61\text{ cm}$, $E_{\text{chem}} = 3165\text{ J}$).
    Creedmoor65,
    /// `7.62×51mm NATO (.308)` (`7.62 mm`, `147 gr` bullet, `45.0 gr` powder, $L_{\text{opt}} = 56\text{ cm}$, $E_{\text{chem}} = 3350\text{ J}$).
    Nato762x51,
    /// `12-Gauge (2¾" 00 Buck / Slug)` (`18.5 mm`, `437 gr` payload, `32.0 gr` powder, $L_{\text{opt}} = 51\text{ cm}$, $E_{\text{chem}} = 3150\text{ J}$).
    Gauge12,
    /// `.338 Lapua Magnum` (`8.6 mm`, `250 gr` bullet, `90.0 gr` powder, $L_{\text{opt}} = 69\text{ cm}$, $E_{\text{chem}} = 6600\text{ J}$).
    Lapua338,
    /// `.50 BMG (12.7×99mm)` (`12.7 mm`, `660 gr` bullet, `235.0 gr` powder, $L_{\text{opt}} = 91\text{ cm}$, $E_{\text{chem}} = 18050\text{ J}$).
    Bmg50,
}

/// Canonical Firearm Feed Mechanism (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3.3 Table, lines 178–191).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponFeedType {
    /// Detachable Box Magazine (`Rel +0`, Reload `12–16 Ticks`).
    DetachableBoxMagazine,
    /// High-Capacity Drum or Helical Magazine (`Rel -2`, Reload `20 Ticks`).
    DrumOrHelical,
    /// Disintegrating-Link Belt-Fed (`Rel +0` braced, Reload `32 Ticks` solo / `14 Ticks` crew).
    BeltFed,
    /// Under-Barrel or Stock Tubular Magazine (`Rel +0`, Reload `6 Ticks` per shell, dynamic CoG).
    TubularMagazine,
    /// Revolving Cylinder (`Rel +1`, Reload `8 Ticks`/rd or `14 Ticks` speedloader).
    RevolvingCylinder,
    /// Single-Shot Breech or Fixed Internal Magazine (`Rel +1..+2`).
    SingleOrInternalFixed,
}

impl WeaponFeedType {
    /// Feed mechanism contribution to Weapon Reliability ($\text{Rel}_{\text{feed}}$, `ranged_combat_and_armor.md` §3.3 & §6.1 line 361).
    #[must_use]
    pub const fn reliability_mod(self) -> i32 {
        match self {
            Self::DetachableBoxMagazine | Self::BeltFed | Self::TubularMagazine => 0,
            Self::DrumOrHelical => -2,
            Self::RevolvingCylinder | Self::SingleOrInternalFixed => 1,
        }
    }
}

/// Canonical Firearm Action Mechanism (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3.1.3 lines 153–164 & §5.1 Table lines 287–298).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirearmActionType {
    /// Rotating-lug manual Bolt-Action ($\eta_{\text{action}} = 1.00$, $\Delta\text{Acc} = +1.5$, $\text{Rel} = +2$, $\Delta R = 0$).
    BoltAction,
    /// Single-Action (`SA`) hammer-fired locked breech ($\eta_{\text{action}} = 0.99$, $\Delta\text{Acc} = +1.0$, $\text{Rel} = +1$, $\Delta R = 0$).
    SingleAction,
    /// Striker-fired tilting-barrel locked breech ($\eta_{\text{action}} = 0.99$, $\Delta\text{Acc} = 0.0$, $\text{Rel} = +1$, $\Delta R = 0$).
    StrikerFired,
    /// Direct Impingement (`DI`) internal gas piston ($\eta_{\text{action}} = 0.97$, $\Delta\text{Acc} = +0.5$, $\text{Rel} = 0$, $\Delta R = -1$).
    DirectImpingement,
    /// Short-Stroke Gas Piston ($\eta_{\text{action}} = 0.97$, $\Delta\text{Acc} = 0.0$, $\text{Rel} = +1$, $\Delta R = 0$).
    ShortStrokeGasPiston,
    /// Long-Stroke Gas Piston ($\eta_{\text{action}} = 0.97$, $\Delta\text{Acc} = -0.5$, $\text{Rel} = +2$, $\Delta R = +1$).
    LongStrokeGasPiston,
    /// Manual Pump-Action ($\eta_{\text{action}} = 0.99$, $\Delta\text{Acc} = 0.0$, $\text{Rel} = +2$, $\Delta R = 0$).
    PumpAction,
    /// Delayed Blowback ($\eta_{\text{action}} = 0.95$, $\Delta\text{Acc} = 0.0$, $\text{Rel} = 0$, $\Delta R = 0$).
    DelayedBlowback,
    /// Open-Bolt or Sear-Conversion Auto-Switch ($\eta_{\text{action}} = 0.95$, $\Delta\text{Acc} = -1.5$, $\text{Rel} = -1$, $\Delta R = 0$).
    OpenBoltOrAutoSwitch,
    /// Double/Single-Action Revolver with Cylinder Gap ($\eta_{\text{action}} = 0.92$, $\Delta\text{Acc} = +0.5$, $\text{Rel} = +2$, $\Delta R = 0$).
    RevolverCylinderGap,
}

impl FirearmActionType {
    /// Propellant gas retention factor $\eta_{\text{action}} \in [0.92, 1.00]$ (`ranged_combat_and_armor.md` §3.1.3 Table, lines 155–164).
    #[must_use]
    pub const fn gas_efficiency_factor(self) -> f64 {
        match self {
            Self::BoltAction => 1.00,
            Self::SingleAction | Self::StrikerFired | Self::PumpAction => 0.99,
            Self::DirectImpingement | Self::ShortStrokeGasPiston | Self::LongStrokeGasPiston => {
                0.97
            }
            Self::DelayedBlowback | Self::OpenBoltOrAutoSwitch => 0.95,
            Self::RevolverCylinderGap => 0.92,
        }
    }

    /// Action contribution to Intrinsic Accuracy ($\Delta\text{Acc}_{\text{action}}$, `ranged_combat_and_armor.md` §5.1 Table, lines 289–298).
    #[must_use]
    pub const fn accuracy_mod(self) -> f64 {
        match self {
            Self::BoltAction => 1.5,
            Self::SingleAction => 1.0,
            Self::DirectImpingement | Self::RevolverCylinderGap => 0.5,
            Self::StrikerFired
            | Self::ShortStrokeGasPiston
            | Self::PumpAction
            | Self::DelayedBlowback => 0.0,
            Self::LongStrokeGasPiston => -0.5,
            Self::OpenBoltOrAutoSwitch => -1.5,
        }
    }

    /// Action contribution to Weapon Reliability ($\text{Rel}_{\text{action}}$, `ranged_combat_and_armor.md` §5.1 Table, lines 289–298).
    #[must_use]
    pub const fn reliability_mod(self) -> i32 {
        match self {
            Self::BoltAction
            | Self::LongStrokeGasPiston
            | Self::PumpAction
            | Self::RevolverCylinderGap => 2,
            Self::SingleAction | Self::StrikerFired | Self::ShortStrokeGasPiston => 1,
            Self::DirectImpingement | Self::DelayedBlowback => 0,
            Self::OpenBoltOrAutoSwitch => -1,
        }
    }

    /// Action reciprocating-mass contribution to Base Single-Shot Recoil ($\Delta R_{\text{action}}$, `ranged_combat_and_armor.md` §5.1 lines 294–297 & §5.2.1 line 308).
    #[must_use]
    pub const fn recoil_mod(self) -> i32 {
        match self {
            Self::DirectImpingement => -1,
            Self::LongStrokeGasPiston => 1,
            Self::BoltAction
            | Self::SingleAction
            | Self::StrikerFired
            | Self::ShortStrokeGasPiston
            | Self::PumpAction
            | Self::DelayedBlowback
            | Self::OpenBoltOrAutoSwitch
            | Self::RevolverCylinderGap => 0,
        }
    }
}

/// Barrel mounting method (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §5.2.4, lines 346–354).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarrelMountingType {
    /// Non-free-floated barrel (handguard, sling swivel, or gas/mag tube touches barrel: `-0.5` Acc).
    ClampedNonFreeFloated,
    /// Standard pistol tilting/fixed barrel (`0.0` Acc).
    StandardPistolMount,
    /// Free-floating barrel cantilevered only from the receiver barrel nut (`+1.0` Acc).
    FreeFloating,
}

impl BarrelMountingType {
    /// Mounting contribution to Intrinsic Accuracy ($\Delta\text{Acc}_{\text{mount}}$, `ranged_combat_and_armor.md` §5.2.4, lines 348–350).
    #[must_use]
    pub const fn accuracy_mod(self) -> f64 {
        match self {
            Self::ClampedNonFreeFloated => -0.5,
            Self::StandardPistolMount => 0.0,
            Self::FreeFloating => 1.0,
        }
    }
}

/// Muzzle attachment (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3.1.2 Table, lines 142–149).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MuzzleDeviceType {
    /// Unthreaded crown or simple thread protector (`0%` energy shift, `0` Recoil).
    BareCrown,
    /// Standard birdcage or prong flash hider (`0%` energy shift, `-1` Recoil).
    FlashHider,
    /// Baffled muzzle brake or compensator (`-1%` velocity/energy, `-2` Recoil, `+2` side concussion).
    MuzzleBrakeCompensator,
    /// Sound suppressor / silencer (`+2%` freebore velocity boost, `-1` Recoil, `-8` to `-12` acoustic signature).
    SoundSuppressor,
}

impl MuzzleDeviceType {
    /// Velocity/energy efficiency shift ($\Delta\eta_{\text{muzzle}}$, `ranged_combat_and_armor.md` §3.1.2 Table, lines 144–149).
    #[must_use]
    pub const fn energy_efficiency_delta(self) -> f64 {
        match self {
            Self::BareCrown | Self::FlashHider => 0.0,
            Self::MuzzleBrakeCompensator => -0.01,
            Self::SoundSuppressor => 0.02,
        }
    }

    /// Recoil reduction modifier ($\Delta R_{\text{muzzle}}$, `ranged_combat_and_armor.md` §3.1.2 Table, lines 144–149).
    #[must_use]
    pub const fn recoil_mod(self) -> i32 {
        match self {
            Self::BareCrown => 0,
            Self::FlashHider | Self::SoundSuppressor => -1,
            Self::MuzzleBrakeCompensator => -2,
        }
    }
}

/// Center of Gravity relative to the chamber and firing-hand fulcrum (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §5.2.2 Table, lines 315–329).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CenterOfGravityProfile {
    /// Rear-biased Bullpup (`8–15 cm` behind chamber: `-0.5` Acc, `-1` Snap `TC`, `0` Recoil).
    RearBullpup,
    /// Chamber-centered / in-line neutral (`+0.5` Acc, `-1` Recoil).
    ChamberNeutral,
    /// Moderate forward bias (`5–14 cm` ahead of chamber: `+0.0` off-hand Acc, `-1` Recoil, `+1` Snap `TC`).
    ModerateForward,
    /// Extreme front-heavy / overhung (`15+ cm` ahead or heavy drum: `-1.0` off-hand Acc, `-1` Recoil, `+2` Snap `TC`).
    ExtremeFrontHeavyOverhung,
}

impl CenterOfGravityProfile {
    /// Accuracy contribution ($\Delta\text{Acc}_{\text{CoG}}$, `ranged_combat_and_armor.md` §5.2.2 Table, lines 318–323).
    #[must_use]
    pub const fn accuracy_mod(self) -> f64 {
        match self {
            Self::RearBullpup => -0.5,
            Self::ChamberNeutral => 0.5,
            Self::ModerateForward => 0.0,
            Self::ExtremeFrontHeavyOverhung => -1.0,
        }
    }

    /// Recoil modifier ($\Delta R_{\text{CoG}}$, `ranged_combat_and_armor.md` §5.2.2 Table, lines 318–323).
    #[must_use]
    pub const fn recoil_mod(self) -> i32 {
        match self {
            Self::RearBullpup => 0,
            Self::ChamberNeutral | Self::ModerateForward | Self::ExtremeFrontHeavyOverhung => -1,
        }
    }
}

/// Physical and ballistic specification of a cartridge (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3.1–§3.2, lines 121–176).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmmunitionProfile {
    /// Canonical caliber identifier.
    pub caliber_id: AmmoCaliberId,
    /// Display name (e.g., `"9×19mm Parabellum"`).
    pub name: &'static str,
    /// Projectile bore diameter in millimeters ($d_{\text{cal}}$).
    pub caliber_mm: f64,
    /// Projectile mass in grains ($m_{\text{proj}}$, $1\text{ g} = 15.432\text{ gr}$).
    pub bullet_mass_grains: f64,
    /// Chemical gunpowder propellant charge in grains ($m_{\text{prop}}$).
    pub powder_charge_grains: f64,
    /// Optimal propellant-burn reference barrel length in centimeters ($L_{\text{opt}}$).
    pub optimal_barrel_cm: f64,
    /// Reference muzzle kinetic energy at $L_{\text{opt}}$ in a sealed barrel ($E_{\text{chem}}$ in Joules, §3.1.1 line 127).
    pub reference_muzzle_energy_j: f64,
    /// Aerodynamic energy retention length scale in meters ($D_{\text{decay}}$, §3.1.4 line 168).
    pub energy_decay_scale_m: f64,
    /// Hollow-point / soft-point expansion multiplier ($\epsilon_{\text{exp}} = 1.0$ for FMJ/AP, $1.55$ for JHP, §4.1.2 line 226).
    pub expansion_factor: f64,
    /// Sectional density & penetrator bonus ($\text{SD}_{\text{bonus}} + \Delta\text{Pen}_{\text{ammo}}$, §4.2 line 263).
    pub sectional_density_pen_bonus: i32,
    /// Ammunition manufacturing consistency ($Q_{\text{ammo}} \in [-1.5, +1.0]$, §3 Item 5 line 111 & §5.2.4 line 354).
    pub ammo_quality_q: f64,
    /// Whether this is a high-aspect-ratio spitzer that yaws early in soft tissue ($\Delta W_{\text{spitzer}} = +1$, §4.1.3 line 240).
    pub is_spitzer_early_yaw: bool,
}

/// Complete granular engineering profile for a Firearm (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3, §5–§7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GranularFirearmSpec {
    /// Model name & archetype.
    pub name: &'static str,
    /// Chambered ammunition profile.
    pub ammo: AmmunitionProfile,
    /// Feed system (`Box`, `Drum`, `Belt`, `Tubular`, `Cylinder`, `Single`, §3.3).
    pub feed_type: WeaponFeedType,
    /// Rounds per feed unit (`Capacity`).
    pub capacity_rounds: i32,
    /// Operating action mechanism (`Bolt`, `Pump`, `DI`, `SingleAction`, `Piston`, `Striker`, etc., §5.1).
    pub action_type: FirearmActionType,
    /// Unloaded weapon mass in kilograms.
    pub unloaded_weight_kg: f64,
    /// Loaded weapon mass in kilograms ($M_{\text{wpn}}$, §5.2.1).
    pub loaded_weight_kg: f64,
    /// Overall weapon length in centimeters ($\text{OAL}$, §5.2.2 line 325).
    pub overall_length_cm: f64,
    /// Barrel length in centimeters ($L_{\text{bbl}}$, §3.1.2 & §5.2.3).
    pub barrel_length_cm: f64,
    /// Barrel quality modifier ($Q_{\text{bbl}} \in [-2.0, +1.5]$, §5.2.4).
    pub barrel_quality_q: f64,
    /// Barrel mounting method (`FreeFloating`, `ClampedNonFreeFloated`, `StandardPistolMount`, §5.2.4).
    pub barrel_mounting: BarrelMountingType,
    /// Muzzle device (`BareCrown`, `FlashHider`, `MuzzleBrakeCompensator`, `SoundSuppressor`, §3.1.2).
    pub muzzle_device: MuzzleDeviceType,
    /// Center of Gravity relative to chamber ($\Delta_{\text{CoG}}$, §5.2.2).
    pub center_of_gravity: CenterOfGravityProfile,
    /// Machining tolerance ($\tau_{\text{tol}} \in [-1.5, +1.5]$: positive boosts `Acc` and reduces `Rel`, §6.1).
    pub tolerance_tau: f64,
    /// Manufacturing & coating quality ($Q_{\text{mfg}} \in [-2, +2]$, §6.1).
    pub manufacturing_quality_q: i32,
    /// Count-Up Ticks required to clear a Minor Malfunction ($\text{TC}_{\text{clear, minor}}$, §6.2).
    pub clear_jam_minor_ticks: i32,
    /// Count-Up Ticks required to clear a Major Malfunction ($\text{TC}_{\text{clear, major}}$, §6.2).
    pub clear_jam_major_ticks: i32,
    /// Count-Up Ticks required to field-strip the weapon ($\text{TC}_{\text{strip}}$, §6.2).
    pub strip_time_ticks: i32,
    /// Street/retail price in 2032 USD / Credits (§7).
    pub cost_usd: i32,
}

/// Complete physics-derived combat output for a [`GranularFirearmSpec`] at a target range $d$
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3–§6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GranularBallisticsEvaluation {
    /// Barrel length + muzzle device efficiency factor $\eta_{\text{bbl}}$ (§3.1.2, line 136).
    pub barrel_efficiency_eta: f64,
    /// Action gas retention factor $\eta_{\text{action}}$ (§3.1.3, line 153).
    pub action_efficiency_eta: f64,
    /// Muzzle kinetic energy $E_{\text{muzzle}}$ in Joules (§3.1.3, line 153).
    pub muzzle_energy_joules: f64,
    /// Impact kinetic energy $E_k(d)$ at target distance $d$ in Joules (§3.1.4, line 168).
    pub impact_energy_joules: f64,
    /// Logarithmic Kinetic Energy Index $I_{\text{KE}} \in [1, 9]$ (§4.1.1, line 208).
    pub ke_index: i32,
    /// Caliber & expansion permanent crush index $I_{\text{cal}} \in [1, 4]$ (§4.1.2, line 226).
    pub caliber_crush_index: i32,
    /// Intrinsic Weapon Accuracy rating $\text{Acc} \in [-4, +4]$ (§5.2.4, line 354).
    pub accuracy_acc: i32,
    /// Inverse-Accuracy Projectile Wobble / Yaw factor $W_{\text{yaw}} \in [0, 3]$ (§4.1.3, line 238).
    pub wobble_yaw_factor: i32,
    /// Total Raw Ballistic Impact ($\text{Raw Impact} = I_{\text{KE}} + I_{\text{cal}} + W_{\text{yaw}}$, §4.2, line 255).
    pub raw_ballistic_impact: i32,
    /// Base Armor Penetration ($\text{Base Pen}$, §4.2, line 263).
    pub base_armor_pen: i32,
    /// Base Single-Shot Recoil Rating ($R_{\text{wpn}}$, §5.2.1, line 308).
    pub base_recoil_r: i32,
    /// Net Mechanical Reliability rating $\text{Rel} \in [-4, +4]$ (§6.1, line 361).
    pub reliability_rel: i32,
    /// CQB Overall Length crowding penalty in Clinch / tight corridors ($\max(0, \lfloor (\text{OAL} - 50)/20 \rfloor)$, §5.2.2, line 325).
    pub cqb_length_crowding_penalty: i32,
    /// Minor jam clearance time in Count-Up Ticks ($\text{TC}_{\text{clear, minor}}$, §6.2).
    pub clear_jam_minor_ticks: i32,
    /// Major jam clearance time in Count-Up Ticks ($\text{TC}_{\text{clear, major}}$, §6.2).
    pub clear_jam_major_ticks: i32,
    /// Field-strip disassembly time in Count-Up Ticks ($\text{TC}_{\text{strip}}$, §6.2).
    pub strip_time_ticks: i32,
}

/// Computes the reference Cartridge Chemical Energy ($E_{\text{chem}}$ in Joules) at optimal barrel length $L_{\text{opt}}$
/// from projectile mass $m_{\text{proj}}$ (grains) and propellant charge $m_{\text{prop}}$ (grains)
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3.1.1, line 127):
/// $$E_{\text{chem}} = 105.0 \cdot m_{\text{prop}} \cdot \left(\frac{m_{\text{proj}}}{m_{\text{proj}} + 2.2 \cdot m_{\text{prop}}}\right)$$
#[must_use]
pub fn compute_cartridge_chem_energy_joules(
    bullet_mass_grains: f64,
    powder_charge_grains: f64,
) -> f64 {
    let m_proj = bullet_mass_grains.max(0.0);
    let m_prop = powder_charge_grains.max(0.0);
    let denom = m_proj + 2.2 * m_prop;
    if denom <= 0.0 {
        0.0
    } else {
        105.0 * m_prop * (m_proj / denom)
    }
}

/// Evaluates the complete internal/external/terminal ballistics, accuracy, recoil, and reliability
/// of a [`GranularFirearmSpec`] at target distance `distance_m` (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3–§6).
pub fn evaluate_granular_firearm_ballistics(
    spec: &GranularFirearmSpec,
    distance_m: f64,
) -> GranularBallisticsEvaluation {
    let ratio = (spec.barrel_length_cm / spec.ammo.optimal_barrel_cm).max(0.1);
    let barrel_efficiency_eta =
        (1.0 + 0.35 * ratio.ln()).clamp(0.60, 1.15) + spec.muzzle_device.energy_efficiency_delta();
    let action_efficiency_eta = spec.action_type.gas_efficiency_factor();
    let muzzle_energy_joules =
        spec.ammo.reference_muzzle_energy_j * barrel_efficiency_eta * action_efficiency_eta;
    let d = distance_m.max(0.0);
    let impact_energy_joules =
        muzzle_energy_joules * (-d / spec.ammo.energy_decay_scale_m.max(1.0)).exp();

    let ke_index = (1.85 * (1.0 + impact_energy_joules / 150.0).ln())
        .round()
        .clamp(1.0, 9.0) as i32;
    let caliber_crush_index = ((spec.ammo.caliber_mm * spec.ammo.expansion_factor) / 4.5)
        .round()
        .clamp(1.0, 4.0) as i32;

    let acc_barrel_len = if spec.barrel_length_cm < 9.0 {
        -1.5
    } else if spec.barrel_length_cm <= 25.0 {
        -0.5
    } else if spec.barrel_length_cm <= 44.0 {
        0.0
    } else if spec.barrel_length_cm <= 55.0 {
        0.5
    } else {
        1.0
    };
    let acc_weight = if spec.overall_length_cm < 35.0 {
        if spec.loaded_weight_kg < 0.65 {
            -0.5
        } else if spec.loaded_weight_kg > 1.3 {
            0.5
        } else {
            0.0
        }
    } else if spec.loaded_weight_kg < 2.4 {
        -0.5
    } else if spec.loaded_weight_kg >= 4.5 {
        0.5
    } else {
        0.0
    };
    let raw_acc = spec.action_type.accuracy_mod()
        + acc_weight
        + spec.center_of_gravity.accuracy_mod()
        + acc_barrel_len
        + spec.barrel_quality_q
        + spec.barrel_mounting.accuracy_mod()
        + spec.tolerance_tau
        + spec.ammo.ammo_quality_q;
    let accuracy_acc = raw_acc.round().clamp(-4.0, 4.0) as i32;

    let spitzer_bonus = i32::from(spec.ammo.is_spitzer_early_yaw && impact_energy_joules >= 800.0);
    let wobble_yaw_factor = ((1 - accuracy_acc).max(0) + spitzer_bonus).clamp(0, 3);

    let raw_ballistic_impact = ke_index + caliber_crush_index + wobble_yaw_factor;
    let base_armor_pen = (ke_index + spec.ammo.sectional_density_pen_bonus
        - caliber_crush_index.div_euclid(2)
        - wobble_yaw_factor.div_euclid(2))
    .max(1);

    let impulse_num = 0.025 * spec.ammo.bullet_mass_grains + 0.14 * spec.ammo.powder_charge_grains;
    let weight_sqrt = spec.loaded_weight_kg.max(0.3).sqrt();
    let base_recoil_r = ((impulse_num / weight_sqrt).round() as i32
        + spec.action_type.recoil_mod()
        + spec.center_of_gravity.recoil_mod()
        + spec.muzzle_device.recoil_mod())
    .max(1);

    let ammo_rel_mod = if spec.ammo.ammo_quality_q < -1.0
        || (spec.ammo.ammo_quality_q <= -0.5 && spec.tolerance_tau >= 1.0)
    {
        -1
    } else {
        0
    };
    let reliability_rel = (spec.action_type.reliability_mod() + spec.feed_type.reliability_mod()
        - (spec.tolerance_tau.round() as i32)
        + spec.manufacturing_quality_q
        + ammo_rel_mod)
        .clamp(-4, 4);

    let cqb_length_crowding_penalty = if spec.overall_length_cm > 50.0 {
        ((spec.overall_length_cm - 50.0) / 20.0).floor() as i32
    } else {
        0
    };

    GranularBallisticsEvaluation {
        barrel_efficiency_eta,
        action_efficiency_eta,
        muzzle_energy_joules,
        impact_energy_joules,
        ke_index,
        caliber_crush_index,
        accuracy_acc,
        wobble_yaw_factor,
        raw_ballistic_impact,
        base_armor_pen,
        base_recoil_r,
        reliability_rel,
        cqb_length_crowding_penalty,
        clear_jam_minor_ticks: spec.clear_jam_minor_ticks,
        clear_jam_major_ticks: spec.clear_jam_major_ticks,
        strip_time_ticks: spec.strip_time_ticks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_granular_firearm_ballistics_akm_vs_65_creedmoor_and_tolerance_tradeoff() {
        let akm_chem = compute_cartridge_chem_energy_joules(123.0, 25.5);
        assert!((akm_chem - 1841.3).abs() < 5.0);

        let akm = GranularFirearmSpec {
            name: "Kalashnikov AKM (7.62x39mm)",
            ammo: AmmunitionProfile {
                caliber_id: AmmoCaliberId::Soviet762x39,
                name: "7.62x39mm FMJ",
                caliber_mm: 7.62,
                bullet_mass_grains: 123.0,
                powder_charge_grains: 25.5,
                optimal_barrel_cm: 41.5,
                reference_muzzle_energy_j: 2100.0,
                energy_decay_scale_m: 420.0,
                expansion_factor: 1.0,
                sectional_density_pen_bonus: 2,
                ammo_quality_q: 0.0,
                is_spitzer_early_yaw: false,
            },
            feed_type: WeaponFeedType::DetachableBoxMagazine,
            capacity_rounds: 30,
            action_type: FirearmActionType::LongStrokeGasPiston,
            unloaded_weight_kg: 3.30,
            loaded_weight_kg: 4.10,
            overall_length_cm: 88.0,
            barrel_length_cm: 41.5,
            barrel_quality_q: 0.0,
            barrel_mounting: BarrelMountingType::ClampedNonFreeFloated,
            muzzle_device: MuzzleDeviceType::BareCrown,
            center_of_gravity: CenterOfGravityProfile::ModerateForward,
            tolerance_tau: -1.0,
            manufacturing_quality_q: 1,
            clear_jam_minor_ticks: 8,
            clear_jam_major_ticks: 20,
            strip_time_ticks: 32,
            cost_usd: 950,
        };

        let creedmoor_rifle = GranularFirearmSpec {
            name: "Accuracy Int'l AT-X (6.5mm Creedmoor)",
            ammo: AmmunitionProfile {
                caliber_id: AmmoCaliberId::Creedmoor65,
                name: "6.5mm Creedmoor Match OTM",
                caliber_mm: 6.72,
                bullet_mass_grains: 140.0,
                powder_charge_grains: 42.5,
                optimal_barrel_cm: 56.0,
                reference_muzzle_energy_j: 3165.0,
                energy_decay_scale_m: 950.0,
                expansion_factor: 1.4,
                sectional_density_pen_bonus: 3,
                ammo_quality_q: 1.0,
                is_spitzer_early_yaw: false,
            },
            feed_type: WeaponFeedType::DetachableBoxMagazine,
            capacity_rounds: 10,
            action_type: FirearmActionType::BoltAction,
            unloaded_weight_kg: 5.40,
            loaded_weight_kg: 5.80,
            overall_length_cm: 112.0,
            barrel_length_cm: 61.0,
            barrel_quality_q: 1.5,
            barrel_mounting: BarrelMountingType::FreeFloating,
            muzzle_device: MuzzleDeviceType::MuzzleBrakeCompensator,
            center_of_gravity: CenterOfGravityProfile::ChamberNeutral,
            tolerance_tau: 1.0,
            manufacturing_quality_q: 2,
            clear_jam_minor_ticks: 6,
            clear_jam_major_ticks: 20,
            strip_time_ticks: 80,
            cost_usd: 5200,
        };

        let akm_15m = evaluate_granular_firearm_ballistics(&akm, 15.0);
        assert!(akm_15m.accuracy_acc < 0);
        assert!(akm_15m.wobble_yaw_factor >= 2);
        assert_eq!(akm_15m.caliber_crush_index, 2);
        assert_eq!(akm_15m.reliability_rel, 4);
        assert!(akm_15m.raw_ballistic_impact >= 9);

        let akm_400m = evaluate_granular_firearm_ballistics(&akm, 400.0);
        let cm_400m = evaluate_granular_firearm_ballistics(&creedmoor_rifle, 400.0);
        assert_eq!(cm_400m.action_efficiency_eta, 1.00);
        assert_eq!(cm_400m.accuracy_acc, 4);
        assert_eq!(cm_400m.wobble_yaw_factor, 0);
        assert!(cm_400m.impact_energy_joules > 2.0 * akm_400m.impact_energy_joules);
        assert!(cm_400m.base_armor_pen >= 7);
        assert!(akm_400m.base_armor_pen <= 4);
    }
}
