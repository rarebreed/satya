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
    pub caliber_id: AmmoCaliberId,
    pub name: &'static str,
    pub caliber_mm: f64,
    pub bullet_mass_grains: f64,
    pub powder_charge_grains: f64,
    pub optimal_barrel_cm: f64,
    pub reference_muzzle_energy_j: f64,
    pub energy_decay_scale_m: f64,
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
    pub name: &'static str,
    pub ammo: AmmunitionProfile,
    pub feed_type: WeaponFeedType,
    pub capacity_rounds: i32,
    pub action_type: FirearmActionType,
    pub unloaded_weight_kg: f64,
    pub loaded_weight_kg: f64,
    pub overall_length_cm: f64,
    pub barrel_length_cm: f64,
    /// Barrel quality modifier ($Q_{\text{bbl}} \in [-2.0, +1.5]$, §5.2.4).
    pub barrel_quality_q: f64,
    pub barrel_mounting: BarrelMountingType,
    pub muzzle_device: MuzzleDeviceType,
    pub center_of_gravity: CenterOfGravityProfile,
    /// Machining tolerance ($\tau_{\text{tol}} \in [-1.5, +1.5]$: positive boosts `Acc` and reduces `Rel`, §6.1).
    pub tolerance_tau: f64,
    /// Manufacturing & coating quality ($Q_{\text{mfg}} \in [-2, +2]$, §6.1).
    pub manufacturing_quality_q: i32,
    pub clear_jam_minor_ticks: i32,
    pub clear_jam_major_ticks: i32,
    pub strip_time_ticks: i32,
    pub cost_usd: i32,
}

/// Complete physics-derived combat output for a [`GranularFirearmSpec`] at a target range $d$
/// (`docs/ttrpg/05-combat/ranged_combat_and_armor.md` §3–§6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GranularBallisticsEvaluation {
    pub barrel_efficiency_eta: f64,
    pub action_efficiency_eta: f64,
    pub muzzle_energy_joules: f64,
    pub impact_energy_joules: f64,
    pub ke_index: i32,
    pub caliber_crush_index: i32,
    /// Intrinsic Weapon Accuracy rating $\text{Acc} \in [-4, +4]$ (§5.2.4, line 354).
    pub accuracy_acc: i32,
    pub wobble_yaw_factor: i32,
    pub raw_ballistic_impact: i32,
    pub base_armor_pen: i32,
    pub base_recoil_r: i32,
    /// Net Mechanical Reliability rating $\text{Rel} \in [-4, +4]$ (§6.1, line 361).
    pub reliability_rel: i32,
    /// CQB Overall Length crowding penalty in Clinch / tight corridors ($\max(0, \lfloor (\text{OAL} - 50)/20 \rfloor)$, §5.2.2, line 325).
    pub cqb_length_crowding_penalty: i32,
    pub clear_jam_minor_ticks: i32,
    pub clear_jam_major_ticks: i32,
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

macro_rules! firearm_preset {
    (
        $id:literal, $name:literal,
        ($cal:ident, $aname:literal, $mm:expr, $b_gr:expr, $p_gr:expr, $opt_cm:expr, $e_j:expr, $dec_m:expr, $exp:expr, $sd:expr, $aq:expr, $spitz:expr),
        ($feed:ident, $cap:expr, $act:ident, $u_kg:expr, $l_kg:expr, $oal_cm:expr, $bbl_cm:expr, $bq:expr, $mnt:ident, $muz:ident, $cog:ident, $tau:expr, $mq:expr, $j_min:expr, $j_maj:expr, $strp:expr, $cost:expr)
    ) => {
        (
            $id,
            GranularFirearmSpec {
                name: $name,
                ammo: AmmunitionProfile {
                    caliber_id: AmmoCaliberId::$cal,
                    name: $aname,
                    caliber_mm: $mm,
                    bullet_mass_grains: $b_gr,
                    powder_charge_grains: $p_gr,
                    optimal_barrel_cm: $opt_cm,
                    reference_muzzle_energy_j: $e_j,
                    energy_decay_scale_m: $dec_m,
                    expansion_factor: $exp,
                    sectional_density_pen_bonus: $sd,
                    ammo_quality_q: $aq,
                    is_spitzer_early_yaw: $spitz,
                },
                feed_type: WeaponFeedType::$feed,
                capacity_rounds: $cap,
                action_type: FirearmActionType::$act,
                unloaded_weight_kg: $u_kg,
                loaded_weight_kg: $l_kg,
                overall_length_cm: $oal_cm,
                barrel_length_cm: $bbl_cm,
                barrel_quality_q: $bq,
                barrel_mounting: BarrelMountingType::$mnt,
                muzzle_device: MuzzleDeviceType::$muz,
                center_of_gravity: CenterOfGravityProfile::$cog,
                tolerance_tau: $tau,
                manufacturing_quality_q: $mq,
                clear_jam_minor_ticks: $j_min,
                clear_jam_major_ticks: $j_maj,
                strip_time_ticks: $strp,
                cost_usd: $cost,
            },
        )
    };
}

/// Canonical preset ranged firearms for operator equipment loadouts and ballistics benchmarks.
pub const CANONICAL_FIREARM_PRESETS: &[(&str, GranularFirearmSpec)] = &[
    firearm_preset!(
        "hk416_carbine_556",
        "HK416A5 Tactical Carbine (5.56x45mm)",
        (
            Nato556x45,
            "5.56x45mm NATO M855A1",
            5.56,
            62.0,
            27.0,
            50.8,
            1795.0,
            520.0,
            1.15,
            2,
            0.5,
            true
        ),
        (
            DetachableBoxMagazine,
            30,
            ShortStrokeGasPiston,
            3.12,
            3.60,
            80.5,
            36.8,
            1.0,
            FreeFloating,
            FlashHider,
            ChamberNeutral,
            0.5,
            2,
            6,
            18,
            36,
            2800
        )
    ),
    firearm_preset!(
        "sig_p320_9mm",
        "SIG P320 / M18 Sidearm (9x19mm)",
        (
            Parabellum9x19,
            "9x19mm +P JHP (124 gr)",
            9.0,
            124.0,
            5.2,
            12.0,
            515.0,
            180.0,
            1.5,
            0,
            0.5,
            false
        ),
        (
            DetachableBoxMagazine,
            17,
            StrikerFired,
            0.76,
            0.98,
            18.8,
            9.9,
            0.5,
            StandardPistolMount,
            BareCrown,
            ChamberNeutral,
            0.0,
            1,
            5,
            14,
            20,
            750
        )
    ),
    firearm_preset!(
        "mp7_pdw_57",
        "MP7A2 / P90 Suppressed PDW (5.7x28mm)",
        (
            Fn57x28,
            "5.7x28mm SS190 AP",
            5.7,
            40.0,
            7.2,
            26.0,
            540.0,
            260.0,
            1.0,
            3,
            0.5,
            true
        ),
        (
            DetachableBoxMagazine,
            40,
            ShortStrokeGasPiston,
            1.90,
            2.30,
            41.5,
            18.0,
            1.0,
            FreeFloating,
            SoundSuppressor,
            RearBullpup,
            0.5,
            2,
            6,
            16,
            30,
            2200
        )
    ),
    firearm_preset!(
        "akm_762x39",
        "Kalashnikov AKM Assault Rifle (7.62x39mm)",
        (
            Soviet762x39,
            "7.62x39mm M43 FMJ",
            7.62,
            123.0,
            25.5,
            41.5,
            2100.0,
            420.0,
            1.0,
            2,
            0.0,
            false
        ),
        (
            DetachableBoxMagazine,
            30,
            LongStrokeGasPiston,
            3.30,
            4.10,
            88.0,
            41.5,
            0.0,
            ClampedNonFreeFloated,
            BareCrown,
            ModerateForward,
            -1.0,
            1,
            8,
            20,
            32,
            950
        )
    ),
    firearm_preset!(
        "benelli_m4_12ga",
        "Benelli M4 Combat Shotgun (12-Gauge)",
        (
            Gauge12,
            "12-Gauge 00 Buck / Breaching Slug",
            18.5,
            437.0,
            32.0,
            51.0,
            3150.0,
            140.0,
            1.35,
            1,
            0.5,
            false
        ),
        (
            TubularMagazine,
            7,
            ShortStrokeGasPiston,
            3.82,
            4.25,
            88.5,
            47.0,
            0.5,
            ClampedNonFreeFloated,
            BareCrown,
            ModerateForward,
            0.0,
            2,
            7,
            18,
            40,
            1950
        )
    ),
    firearm_preset!(
        "ai_atx_65cm",
        "Accuracy Int'l AT-X Precision Rifle (6.5mm CM)",
        (
            Creedmoor65,
            "6.5mm Creedmoor 140gr Match OTM",
            6.72,
            140.0,
            42.5,
            56.0,
            3165.0,
            950.0,
            1.4,
            3,
            1.0,
            false
        ),
        (
            DetachableBoxMagazine,
            10,
            BoltAction,
            5.40,
            5.80,
            112.0,
            61.0,
            1.5,
            FreeFloating,
            MuzzleBrakeCompensator,
            ChamberNeutral,
            1.0,
            2,
            6,
            20,
            80,
            5200
        )
    ),
];

/// Looks up a canonical firearm preset by its slug ID (`"hk416_carbine_556"`, `"sig_p320_9mm"`, etc.).
#[must_use]
pub fn lookup_firearm_preset(id: &str) -> Option<&'static GranularFirearmSpec> {
    CANONICAL_FIREARM_PRESETS
        .iter()
        .find(|(pid, _)| *pid == id)
        .map(|(_, spec)| spec)
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
