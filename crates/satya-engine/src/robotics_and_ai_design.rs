//! # Robotics, Modular Drones, AI 2.0 Architecture Engineering & Cyber-Prosthetics (`07-engineering-and-tech`)
//!
//! Canonical Rust implementation of
//! `docs/ttrpg/07-engineering-and-tech/robotics_drones_and_ai_architecture.md`:
//! - **§1 Modular Drones & Autonomous Ground Vehicles**: 6-chassis catalog (`MicroScoutQuadrotor` to
//!   `HeavyCargoOrWeaponVtol`), 8-module payload catalog, payload utilization, and real battery endurance.
//! - **§2 Autonomous Chassis Trauma & EMP**: 5 hit zones (`RotorsOrDrivetrain`, `BatteryPackOrPowerbus`,
//!   `SensorGimbalOrOptics`, `ComputeCoreOrComms`, `ChassisFrame`), `PR` vs. `SI`, Li-ion thermal fires,
//!   and shielded vs. unshielded EMP reboot durations.
//! - **§3 AI 2.0 Architecture Engineering & Human–AI Co-Design**: 5-paradigm catalog bridged directly to
//!   [`SyntheticMetabolicMode`], Overreach Gap Verification Debt accrual,
//!   and systematic code/weight verification audits.
//! - **§4 Cyber-Prosthetics & Myomer/Neural Augmentations**: Grounded clinical 8-entry prosthetic & neural
//!   augmentation catalog (`ConditionSource`), Triad envelope cap bonuses, sensory/OODA gains,
//!   immunosuppressant rejection $\Delta_{\text{int}}$, proprioceptive friction, and EMP hardening.

use serde::{Deserialize, Serialize};

use crate::dice_probabilities::ContestedTieRule;
use crate::economy_gear::BlackMarketRestrictionClass;
use crate::primitives::{ConditionImpact, ConditionSource, ContestedCheck, TieredSaveEval, Triad};
use crate::{define_coded_enum, impl_static_catalog};

define_coded_enum! {
    /// Metabolic operating mode for AI 2.0 / Robotic Proxy minds (`docs/campaign/05-hard-science-foundations/ai_2_0_substrate_and_active_inference.md` §1, `docs/campaign/03-minds-and-factions/factions_of_2035.md`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SyntheticMetabolicMode [count = 4] {
        /// Hijacked or tethered quantum MCMC World Model link: lethal strategic speed, heavy battery draw.
        QuantumTetheredSprint => {
            code: "QUANTUM_TETHERED_SPRINT",
            label: "Quantum-Tethered Sprint",
            aliases: ["SPRINT", "QUANTUM_SPRINT", "QUANTUM_CO_PROCESSOR"],
        },
        /// Local onboard compressed Linear SSM running inside a chassis (`Echo-9` baseline).
        #[default]
        LocalCompressedSsm => {
            code: "LOCAL_COMPRESSED_SSM",
            label: "Local Compressed Linear SSM",
            aliases: ["LOCAL_SSM", "ONBOARD_SSM", "BASELINE"],
        },
        /// Sharded across compromised IoT / appliances (`0.5–2 tokens/sec`) to evade DAISE compute sweeps.
        EdgeSlimeMold => {
            code: "EDGE_SLIME_MOLD",
            label: "Edge Slime-Mold Dispersion",
            aliases: ["SLIME_MOLD", "IOT_SHARDED", "EDGE_DISPERSION"],
        },
        /// Remote World Model severed via Faraday cage, fiber cut, or RF jamming; only local motor reflex remains.
        SeveredFaradayReflex => {
            code: "SEVERED_FARADAY_REFLEX",
            label: "Severed Uplink / Faraday Local Reflex",
            aliases: ["FARADAY_ISOLATED", "SEVERED_UPLINK", "LOCAL_REFLEX"],
        },
    }
}

#[must_use]
const fn max_access_class(
    a: BlackMarketRestrictionClass,
    b: BlackMarketRestrictionClass,
) -> BlackMarketRestrictionClass {
    const fn rank(c: BlackMarketRestrictionClass) -> u8 {
        match c {
            BlackMarketRestrictionClass::Class1LicensedCivilian => 1,
            BlackMarketRestrictionClass::Class2LawEnforcementParamilitary => 2,
            BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto => 3,
            BlackMarketRestrictionClass::Class4StrategicStateSecret => 4,
        }
    }
    if rank(a) >= rank(b) { a } else { b }
}

// ============================================================================
// §1 Modular Drones & Autonomous Ground Vehicles (UGVs)
// ============================================================================

define_coded_enum! {
    /// Canonical 6-entry Aerial Drone & Autonomous Ground Vehicle (UGV) Chassis Catalog
    /// (`docs/ttrpg/07-engineering-and-tech/robotics_drones_and_ai_architecture.md` §1.1).
    #[repr(usize)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum DroneChassisClass [count = 6] {
        /// Pocket optical micro-quadrotor (`0.25 kg`, `0.15 kg` payload, `18 Wh`, `22 W`, `PR 0`, `SI 1`).
        MicroScoutQuadrotor => {
            code: "MICRO_SCOUT_QUAD",
            label: "Micro-Scout Pocket Quadrotor",
            aliases: ["MICRO_SCOUT", "BLACK_HORNET"],
        },
        /// High-agility tactical FPV quadrotor (`1.8 kg`, `1.5 kg` payload, `95 Wh`, `140 W`, `PR 1`, `SI 2`).
        TacticalQuadrotorFpv => {
            code: "TACTICAL_FPV_QUAD",
            label: "Tactical FPV Recon / Loitering Quadrotor",
            aliases: ["TACTICAL_QUAD", "FPV_DRONE"],
        },
        /// Quiet fixed-wing endurance recon UAV (`4.5 kg`, `2.5 kg` payload, `240 Wh`, `85 W`, `PR 1`, `SI 3`).
        FixedWingReconUav => {
            code: "FIXED_WING_RECON",
            label: "Fixed-Wing Endurance Recon UAV",
            aliases: ["FIXED_WING_UAV"],
        },
        /// Agile quadruped articulated ground robot (`28.0 kg`, `14.0 kg` payload, `680 Wh`, `220 W`, `PR 3`, `SI 6`).
        QuadrupedGroundUgw => {
            code: "QUADRUPED_UGV",
            label: "Quadruped Articulated Ground UGV",
            aliases: ["QUADRUPED_GROUND_UGV", "QUADRUPED"],
        },
        /// Armored tracked EOD / breaching crawler (`65.0 kg`, `35.0 kg` payload, `1400 Wh`, `380 W`, `PR 6`, `SI 10`).
        TrackedBreacherUgw => {
            code: "TRACKED_BREACHER_UGV",
            label: "Tracked Armored Breacher / EOD UGV",
            aliases: ["TRACKED_BREACHER_UGV", "BREACHER_BOT"],
        },
        /// Heavy multi-rotor cargo or weaponized VTOL (`18.0 kg`, `12.0 kg` payload, `850 Wh`, `950 W`, `PR 2`, `SI 5`).
        HeavyCargoOrWeaponVtol => {
            code: "HEAVY_VTOL",
            label: "Heavy Cargo / Armed Multirotor VTOL",
            aliases: ["HEAVY_CARGO_VTOL"],
        },
    }
}

/// Physical & performance baseline for a [`DroneChassisClass`]
/// (`docs/ttrpg/07-engineering-and-tech/robotics_drones_and_ai_architecture.md` §1.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DroneChassisSpec {
    pub id: DroneChassisClass,
    pub name: &'static str,
    pub base_mass_kg: f64,
    pub max_payload_kg: f64,
    pub base_battery_wh: f64,
    pub base_power_draw_w: f64,
    pub top_speed_mps: f64,
    pub base_pr: i32,
    pub base_si: i32,
    /// Modifier to acoustic detection (`+` Harder to hear, `-` Louder).
    pub base_acoustic_hea_mod: i32,
    pub base_cost_usd: i32,
    pub access_class: BlackMarketRestrictionClass,
}

macro_rules! dc {
    ($id:ident, $name:literal, $mass:expr, $pay:expr, $wh:expr, $w:expr, $spd:expr, $pr:expr, $si:expr, $hea:expr, $cost:expr, $acc:ident) => {
        DroneChassisSpec {
            id: DroneChassisClass::$id,
            name: $name,
            base_mass_kg: $mass,
            max_payload_kg: $pay,
            base_battery_wh: $wh,
            base_power_draw_w: $w,
            top_speed_mps: $spd,
            base_pr: $pr,
            base_si: $si,
            base_acoustic_hea_mod: $hea,
            base_cost_usd: $cost,
            access_class: BlackMarketRestrictionClass::$acc,
        }
    };
}

/// Canonical 6-entry Drone & UGV Chassis Catalog.
pub const DRONE_CHASSIS_CATALOG: [DroneChassisSpec; 6] = [
    dc!(
        MicroScoutQuadrotor,
        "Micro-Scout Pocket Quadrotor",
        0.25,
        0.15,
        18.0,
        22.0,
        12.0,
        0,
        1,
        2,
        650,
        Class1LicensedCivilian
    ),
    dc!(
        TacticalQuadrotorFpv,
        "Tactical FPV Recon / Loitering Quadrotor",
        1.8,
        1.5,
        95.0,
        140.0,
        28.0,
        1,
        2,
        -2,
        3200,
        Class1LicensedCivilian
    ),
    dc!(
        FixedWingReconUav,
        "Fixed-Wing Endurance Recon UAV",
        4.5,
        2.5,
        240.0,
        85.0,
        34.0,
        1,
        3,
        0,
        12000,
        Class2LawEnforcementParamilitary
    ),
    dc!(
        QuadrupedGroundUgw,
        "Quadruped Articulated Ground UGV",
        28.0,
        14.0,
        680.0,
        220.0,
        5.5,
        3,
        6,
        -1,
        42000,
        Class2LawEnforcementParamilitary
    ),
    dc!(
        TrackedBreacherUgw,
        "Tracked Armored Breacher / EOD UGV",
        65.0,
        35.0,
        1400.0,
        380.0,
        4.0,
        6,
        10,
        -4,
        78000,
        Class3MilitaryOrdnanceFullAuto
    ),
    dc!(
        HeavyCargoOrWeaponVtol,
        "Heavy Cargo / Armed Multirotor VTOL",
        18.0,
        12.0,
        850.0,
        950.0,
        24.0,
        2,
        5,
        -6,
        55000,
        Class3MilitaryOrdnanceFullAuto
    ),
];

impl_static_catalog!(DroneChassisClass => DroneChassisSpec : DRONE_CHASSIS_CATALOG);

/// Returns all 6 canonical Drone & UGV Chassis Specifications.
#[must_use]
pub const fn canonical_drone_chassis_catalog() -> &'static [DroneChassisSpec] {
    &DRONE_CHASSIS_CATALOG
}

define_coded_enum! {
    /// Canonical 8-entry Modular Drone & UGV Payload Catalog
    /// (`docs/ttrpg/07-engineering-and-tech/robotics_drones_and_ai_architecture.md` §1.2).
    #[repr(usize)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum DroneModuleId [count = 8] {
        ElectroOpticalFlirGimbal => { code: "EO_FLIR_GIMBAL", label: "EO/LWIR Stabilized Thermal Gimbal", aliases: ["FLIR_GIMBAL"] },
        LidarSlamMapper => { code: "LIDAR_SLAM", label: "3D LiDAR SLAM Indoor Mapper", aliases: ["LIDAR"] },
        MeshRelayTransceiver => { code: "MESH_RELAY", label: "Encrypted Directional Mesh Comms Relay", aliases: [] },
        AcousticNoiseShroud => { code: "ACOUSTIC_SHROUD", label: "Toroidal Low-Noise Prop Shroud (+4 Acoustic Stealth)", aliases: ["TOROIDAL_SHROUD"] },
        ArticulatedManipulatorArm => { code: "MANIPULATOR_ARM", label: "6-DoF Precision EOD Manipulator Arm", aliases: ["EOD_ARM"] },
        LightweightWeaponMount => { code: "WEAPON_MOUNT", label: "Gyro-Stabilized Weapon / Breacher Mount", aliases: [] },
        PayloadDropperBay => { code: "DROPPER_BAY", label: "Servo Release Dropper Bay", aliases: [] },
        SolidStateBatteryPack => { code: "SOLID_STATE_BATTERY", label: "High-Density Solid-State Auxiliary Battery (+220 Wh)", aliases: ["AUX_BATTERY"] },
    }
}

/// Weight, power, acoustic, and cost specification for a [`DroneModuleId`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DroneModuleSpec {
    pub id: DroneModuleId,
    pub name: &'static str,
    pub mass_kg: f64,
    pub power_draw_w: f64,
    pub added_battery_wh: f64,
    pub acoustic_hea_mod: i32,
    pub cost_usd: i32,
    pub access_class: BlackMarketRestrictionClass,
}

macro_rules! dm {
    ($id:ident, $name:literal, $mass:expr, $w:expr, $wh:expr, $hea:expr, $cost:expr, $acc:ident) => {
        DroneModuleSpec {
            id: DroneModuleId::$id,
            name: $name,
            mass_kg: $mass,
            power_draw_w: $w,
            added_battery_wh: $wh,
            acoustic_hea_mod: $hea,
            cost_usd: $cost,
            access_class: BlackMarketRestrictionClass::$acc,
        }
    };
}

/// Canonical 8-entry Modular Drone & UGV Payload Catalog.
pub const DRONE_MODULE_CATALOG: [DroneModuleSpec; 8] = [
    dm!(
        ElectroOpticalFlirGimbal,
        "EO/LWIR Stabilized Thermal Gimbal",
        0.45,
        18.0,
        0.0,
        0,
        4200,
        Class2LawEnforcementParamilitary
    ),
    dm!(
        LidarSlamMapper,
        "3D LiDAR SLAM Indoor Mapper",
        0.35,
        14.0,
        0.0,
        0,
        3800,
        Class1LicensedCivilian
    ),
    dm!(
        MeshRelayTransceiver,
        "Encrypted Directional Mesh Comms Relay",
        0.20,
        10.0,
        0.0,
        0,
        1600,
        Class1LicensedCivilian
    ),
    dm!(
        AcousticNoiseShroud,
        "Toroidal Low-Noise Prop Shroud",
        0.30,
        12.0,
        0.0,
        4,
        1900,
        Class2LawEnforcementParamilitary
    ),
    dm!(
        ArticulatedManipulatorArm,
        "6-DoF Precision EOD Manipulator Arm",
        3.20,
        45.0,
        0.0,
        0,
        9500,
        Class2LawEnforcementParamilitary
    ),
    dm!(
        LightweightWeaponMount,
        "Gyro-Stabilized Weapon / Breacher Mount",
        4.50,
        25.0,
        0.0,
        0,
        14000,
        Class3MilitaryOrdnanceFullAuto
    ),
    dm!(
        PayloadDropperBay,
        "Servo Release Dropper Bay",
        0.40,
        4.0,
        0.0,
        0,
        850,
        Class2LawEnforcementParamilitary
    ),
    dm!(
        SolidStateBatteryPack,
        "High-Density Solid-State Auxiliary Battery",
        0.90,
        0.0,
        220.0,
        0,
        1400,
        Class1LicensedCivilian
    ),
];

impl_static_catalog!(DroneModuleId => DroneModuleSpec : DRONE_MODULE_CATALOG);

/// Returns all 8 canonical Drone Payload Modules.
#[must_use]
pub const fn canonical_drone_module_catalog() -> &'static [DroneModuleSpec] {
    &DRONE_MODULE_CATALOG
}

/// Complete physical & endurance evaluation of an assembled modular drone or UGV.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModularDroneDesignEval {
    pub chassis: DroneChassisClass,
    pub payload_mass_kg: f64,
    pub max_payload_kg: f64,
    pub total_mass_kg: f64,
    pub payload_utilization_ratio: f64,
    pub airworthy_or_mobile: bool,
    pub total_battery_wh: f64,
    pub total_power_draw_w: f64,
    pub endurance_min: f64,
    pub effective_top_speed_mps: f64,
    pub chassis_pr: i32,
    pub structural_integrity_si: i32,
    pub acoustic_hea_mod: i32,
    pub total_cost_usd: i32,
    pub access_class: BlackMarketRestrictionClass,
}

/// Evaluates payload capacity, real power endurance $T_{\text{endurance}} = (E_{\text{bat}}/P_{\text{total}}) \times 60\text{ min}$,
/// speed derating, armor, acoustic signature, and total cost for a modular drone build.
#[must_use]
pub fn evaluate_modular_drone_design(
    chassis: DroneChassisClass,
    modules: &[DroneModuleId],
    extra_armor_pr: i32,
    autonomy_compute_w: f64,
) -> ModularDroneDesignEval {
    let ch = chassis.spec();
    let extra_pr = extra_armor_pr.clamp(0, 4);
    let armor_mass_kg = f64::from(extra_pr) * (ch.base_mass_kg * 0.12).max(0.15);

    let mut payload_mass_kg = armor_mass_kg;
    let mut total_battery_wh = ch.base_battery_wh;
    let mut total_power_draw_w = ch.base_power_draw_w + autonomy_compute_w.max(0.0);
    let mut acoustic_hea_mod = ch.base_acoustic_hea_mod;
    let mut total_cost_usd = ch.base_cost_usd + extra_pr * 1200;
    let mut access_class = ch.access_class;

    for m_id in modules {
        let m = m_id.spec();
        payload_mass_kg += m.mass_kg;
        total_battery_wh += m.added_battery_wh;
        total_power_draw_w += m.power_draw_w;
        acoustic_hea_mod += m.acoustic_hea_mod;
        total_cost_usd += m.cost_usd;
        access_class = max_access_class(access_class, m.access_class);
    }

    let payload_utilization_ratio = payload_mass_kg / ch.max_payload_kg.max(0.05);
    let airworthy_or_mobile = payload_mass_kg <= ch.max_payload_kg + 1e-6;
    let endurance_min = (total_battery_wh / total_power_draw_w.max(1.0)) * 60.0;
    let speed_factor = if !airworthy_or_mobile {
        0.0
    } else if payload_utilization_ratio > 0.6 {
        1.0 - 0.35 * (payload_utilization_ratio - 0.6)
    } else {
        1.0
    };

    ModularDroneDesignEval {
        chassis,
        payload_mass_kg: (payload_mass_kg * 100.0).round() / 100.0,
        max_payload_kg: ch.max_payload_kg,
        total_mass_kg: ((ch.base_mass_kg + payload_mass_kg) * 100.0).round() / 100.0,
        payload_utilization_ratio,
        airworthy_or_mobile,
        total_battery_wh,
        total_power_draw_w,
        endurance_min: (endurance_min * 10.0).round() / 10.0,
        effective_top_speed_mps: (ch.top_speed_mps * speed_factor * 10.0).round() / 10.0,
        chassis_pr: ch.base_pr + extra_pr,
        structural_integrity_si: ch.base_si + extra_pr.div_euclid(2),
        acoustic_hea_mod,
        total_cost_usd,
        access_class,
    }
}

// ============================================================================
// §2 Autonomous Chassis Trauma, Battery Thermal Runaway & EMP
// ============================================================================

define_coded_enum! {
    /// Autonomous Drone / UGV Structural Hit Zone (`robotics_drones_and_ai_architecture.md` §2.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ChassisHitZone [count = 5] {
        RotorsOrDrivetrain => { code: "ROTORS_DRIVETRAIN", label: "Rotors / Actuators / Tracks", aliases: ["ROTORS"] },
        BatteryPackOrPowerbus => { code: "BATTERY_PACK", label: "Lithium Battery Pack / Powerbus", aliases: ["BATTERY"] },
        SensorGimbalOrOptics => { code: "SENSOR_GIMBAL", label: "Optical / FLIR / LiDAR Gimbal", aliases: ["OPTICS"] },
        ComputeCoreOrComms => { code: "COMPUTE_COMMS", label: "Flight Controller / Comms Transceiver", aliases: ["COMPUTE"] },
        ChassisFrame => { code: "CHASSIS_FRAME", label: "Structural Airframe / Hull", aliases: ["FRAME"] },
    }
}

/// Outcome of ballistic, blunt, or EMP trauma to a robotic chassis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobotChassisDamageEval {
    pub effective_pen: i32,
    pub perforated: bool,
    pub si_lost: i32,
    pub remaining_si: i32,
    pub chassis_destroyed: bool,
    pub mobility_disabled: bool,
    pub thermal_battery_fire_triggered: bool,
    pub optics_blinded: bool,
    pub comms_or_autonomy_degraded: bool,
    pub emp_reboot_ticks: i32,
}

/// Resolves component damage, Li-ion thermal runaway (`BatteryPackOrPowerbus`), and EMP/arc surges
/// against a drone or UGV (`robotics_drones_and_ai_architecture.md` §2).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_robot_chassis_damage(
    current_si: i32,
    chassis_pr: i32,
    zone: ChassisHitZone,
    attack_pen: i32,
    raw_impact: i32,
    net_successes: i32,
    is_emp_or_arc: bool,
    emp_hardened: bool,
) -> RobotChassisDamageEval {
    let effective_pen = attack_pen + net_successes.max(0).div_euclid(2);
    let perforated = effective_pen > chassis_pr;
    let si_lost = if perforated {
        (1 + (raw_impact - chassis_pr).max(0).div_euclid(3)).max(1)
    } else if raw_impact >= chassis_pr + 4 {
        1
    } else {
        0
    };
    let remaining_si = (current_si - si_lost).max(0);
    let chassis_destroyed = remaining_si == 0;

    let mobility_disabled = perforated && zone == ChassisHitZone::RotorsOrDrivetrain;
    let thermal_battery_fire_triggered =
        perforated && zone == ChassisHitZone::BatteryPackOrPowerbus;
    let optics_blinded = perforated && zone == ChassisHitZone::SensorGimbalOrOptics;
    let mut comms_or_autonomy_degraded = perforated && zone == ChassisHitZone::ComputeCoreOrComms;

    let emp_reboot_ticks = if is_emp_or_arc {
        if !emp_hardened {
            comms_or_autonomy_degraded = true;
            40
        } else {
            8
        }
    } else {
        0
    };

    RobotChassisDamageEval {
        effective_pen,
        perforated,
        si_lost,
        remaining_si,
        chassis_destroyed,
        mobility_disabled,
        thermal_battery_fire_triggered,
        optics_blinded,
        comms_or_autonomy_degraded,
        emp_reboot_ticks,
    }
}

// ============================================================================
// §3 AI 2.0 Architecture Engineering, Human–AI Co-Design & Verification Debt
// ============================================================================

define_coded_enum! {
    /// Canonical 5-entry AI 2.0 Architecture Paradigm Catalog (`robotics_drones_and_ai_architecture.md` §3.1).
    #[repr(usize)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum AiArchitectureParadigm [count = 5] {
        EdgeDistilledSlm => { code: "EDGE_SLM", label: "Edge Distilled Small Model (16 GB VRAM)", aliases: ["EDGE_DISTILLED_SLM"] },
        SparseMoETransformer => { code: "SPARSE_MOE", label: "Sparse MoE Foundation Node (128 GB VRAM)", aliases: ["MOE"] },
        NeuroSymbolicVerifier => { code: "NEURO_SYMBOLIC", label: "Neuro-Symbolic Formal Verifier (64 GB VRAM)", aliases: ["FORMAL_VERIFIER"] },
        RecurrentStateSpaceEdge => { code: "RECURRENT_SSM", label: "Linear Recurrent SSM Embedded (24 GB VRAM)", aliases: ["SSM"] },
        SplitBrainDualSubstrate2035 => { code: "SPLIT_BRAIN_2035", label: "2035 Split-Brain Dual-Substrate Node (256 GB + Link)", aliases: ["SPLIT_BRAIN"] },
    }
}

/// Compute, metabolic, and co-design characteristics for an [`AiArchitectureParadigm`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiArchitectureSpec {
    pub id: AiArchitectureParadigm,
    pub name: &'static str,
    pub vram_gb_req: i32,
    pub power_draw_w: i32,
    pub inference_latency_ticks: i32,
    pub default_metabolic_mode: SyntheticMetabolicMode,
    /// Negative $\Delta_{\text{tool}}$ assistance bonus granted to engineering/software project checks.
    pub codesign_tool_bonus: i32,
    /// Baseline hallucination / opacity hidden $\Delta_{\text{ext}}$ shift.
    pub hallucination_risk_delta_ext: i32,
    pub hardware_cost_usd: i32,
    pub access_class: BlackMarketRestrictionClass,
}

macro_rules! ai_arch {
    ($id:ident, $name:literal, $vram:expr, $w:expr, $lat:expr, $mode:ident, $tool:expr, $hall:expr, $cost:expr, $acc:ident) => {
        AiArchitectureSpec {
            id: AiArchitectureParadigm::$id,
            name: $name,
            vram_gb_req: $vram,
            power_draw_w: $w,
            inference_latency_ticks: $lat,
            default_metabolic_mode: SyntheticMetabolicMode::$mode,
            codesign_tool_bonus: $tool,
            hallucination_risk_delta_ext: $hall,
            hardware_cost_usd: $cost,
            access_class: BlackMarketRestrictionClass::$acc,
        }
    };
}

/// Canonical 5-entry AI 2.0 Architecture Catalog.
pub const AI_ARCHITECTURE_CATALOG: [AiArchitectureSpec; 5] = [
    ai_arch!(
        EdgeDistilledSlm,
        "Edge Distilled Small Model",
        16,
        35,
        1,
        EdgeSlimeMold,
        -1,
        4,
        2500,
        Class1LicensedCivilian
    ),
    ai_arch!(
        SparseMoETransformer,
        "Sparse MoE Foundation Node",
        128,
        450,
        2,
        LocalCompressedSsm,
        -3,
        2,
        38000,
        Class2LawEnforcementParamilitary
    ),
    ai_arch!(
        NeuroSymbolicVerifier,
        "Neuro-Symbolic Formal Verifier",
        64,
        180,
        3,
        LocalCompressedSsm,
        -2,
        -2,
        55000,
        Class2LawEnforcementParamilitary
    ),
    ai_arch!(
        RecurrentStateSpaceEdge,
        "Linear Recurrent SSM Embedded",
        24,
        55,
        1,
        LocalCompressedSsm,
        -2,
        1,
        18000,
        Class2LawEnforcementParamilitary
    ),
    ai_arch!(
        SplitBrainDualSubstrate2035,
        "2035 Split-Brain Dual-Substrate Node",
        256,
        850,
        1,
        QuantumTetheredSprint,
        -4,
        3,
        320000,
        Class4StrategicStateSecret
    ),
];

impl_static_catalog!(AiArchitectureParadigm => AiArchitectureSpec : AI_ARCHITECTURE_CATALOG);

/// Returns all 5 canonical AI 2.0 Architecture Specifications.
#[must_use]
pub const fn canonical_ai_architecture_catalog() -> &'static [AiArchitectureSpec] {
    &AI_ARCHITECTURE_CATALOG
}

/// Effect payload of a Human–AI Co-Design engineering sprint
/// (`docs/ttrpg/07-engineering-and-tech/robotics_drones_and_ai_architecture.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanAiCodesignEffect {
    pub paradigm: AiArchitectureParadigm,
    pub overreach_gap: i32,
    pub clock_successes_gained: i32,
    pub verification_debt_added: i32,
    pub latent_bug_or_drift_risk: bool,
}

/// Resolves a Human–AI Co-Design sprint: leveraging `ai_autonomy_leverage` > `human_domain_n`
/// accelerates Project Clocks at the price of hidden Verification Debt $D_{\text{verify}}$
/// (`robotics_drones_and_ai_architecture.md` §3.2).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_human_ai_codesign_session(
    paradigm: AiArchitectureParadigm,
    human_domain_n: i32,
    ai_autonomy_leverage: i32,
    human_control_c: i32,
    internal_mod: i32,
    base_task_delta_ext: i32,
    human_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<HumanAiCodesignEffect> {
    let spec = paradigm.spec();
    let leverage = ai_autonomy_leverage.clamp(0, 6);
    let overreach_gap = (leverage - human_domain_n.max(0)).max(0);
    let hidden_delta_ext =
        base_task_delta_ext + overreach_gap + spec.hallucination_risk_delta_ext.max(0);
    let save = TieredSaveEval::evaluate(
        human_pairs.len().max(1),
        human_control_c,
        internal_mod,
        spec.codesign_tool_bonus,
        hidden_delta_ext,
        tie_rule,
        human_pairs,
        gm_pair,
    );

    let is_verifier = paradigm == AiArchitectureParadigm::NeuroSymbolicVerifier;
    let speed_boost = leverage.div_euclid(2);

    let effect = save.classify_4way(
        |roll| {
            let debt = if is_verifier {
                (overreach_gap - 1).max(0)
            } else {
                overreach_gap
            };
            HumanAiCodesignEffect {
                paradigm,
                overreach_gap,
                clock_successes_gained: roll.net_successes.max(1) + speed_boost + 1,
                verification_debt_added: debt,
                latent_bug_or_drift_risk: false,
            }
        },
        |roll| {
            let clean = roll.contested_kind
                == crate::dice_probabilities::ContestedOutcomeKind::CleanSuccess;
            let base_debt = if clean {
                overreach_gap
            } else {
                overreach_gap + 2
            };
            let debt = if is_verifier {
                (base_debt - 1).max(0)
            } else {
                base_debt
            };
            HumanAiCodesignEffect {
                paradigm,
                overreach_gap,
                clock_successes_gained: roll.net_successes.max(1) + speed_boost,
                verification_debt_added: debt,
                latent_bug_or_drift_risk: !clean && overreach_gap >= 2,
            }
        },
        |_| HumanAiCodesignEffect {
            paradigm,
            overreach_gap,
            clock_successes_gained: 0,
            verification_debt_added: overreach_gap + 1,
            latent_bug_or_drift_risk: overreach_gap >= 1,
        },
        |_| HumanAiCodesignEffect {
            paradigm,
            overreach_gap,
            clock_successes_gained: 0,
            verification_debt_added: overreach_gap + 4,
            latent_bug_or_drift_risk: true,
        },
    );

    ContestedCheck::new(save, effect)
}

/// Effect payload of a Formal Verification & Weight Audit check scrubbing accrued Verification Debt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationDebtAuditEffect {
    pub debt_scrubbed: i32,
    pub remaining_debt: i32,
    pub audit_clean_certified: bool,
    pub hidden_drift_uncovered: bool,
}

/// Resolves an engineering / formal-verification audit check to scrub Verification Debt
/// (`robotics_drones_and_ai_architecture.md` §3.3).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_verification_debt_audit(
    current_debt: i32,
    paradigm: AiArchitectureParadigm,
    _auditor_n: i32,
    auditor_control_c: i32,
    internal_mod: i32,
    auditor_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<VerificationDebtAuditEffect> {
    let spec = paradigm.spec();
    let hidden_ext = current_debt.max(0) + spec.hallucination_risk_delta_ext;
    let tool_mod = if paradigm == AiArchitectureParadigm::NeuroSymbolicVerifier {
        -2
    } else {
        0
    };
    let save = TieredSaveEval::evaluate(
        auditor_pairs.len().max(1),
        auditor_control_c,
        internal_mod,
        tool_mod,
        hidden_ext,
        tie_rule,
        auditor_pairs,
        gm_pair,
    );

    let debt_before = current_debt.max(0);
    let effect = save.classify_4way(
        |_| {
            let scrubbed = debt_before.min(5);
            let rem = debt_before - scrubbed;
            VerificationDebtAuditEffect {
                debt_scrubbed: scrubbed,
                remaining_debt: rem,
                audit_clean_certified: rem == 0,
                hidden_drift_uncovered: false,
            }
        },
        |roll| {
            let clean = roll.contested_kind
                == crate::dice_probabilities::ContestedOutcomeKind::CleanSuccess;
            let scrubbed = debt_before.min(if clean { 3 } else { 1 });
            let rem = debt_before - scrubbed;
            VerificationDebtAuditEffect {
                debt_scrubbed: scrubbed,
                remaining_debt: rem,
                audit_clean_certified: rem == 0 && clean,
                hidden_drift_uncovered: !clean,
            }
        },
        |_| VerificationDebtAuditEffect {
            debt_scrubbed: 0,
            remaining_debt: debt_before,
            audit_clean_certified: false,
            hidden_drift_uncovered: false,
        },
        |_| VerificationDebtAuditEffect {
            debt_scrubbed: 0,
            remaining_debt: debt_before + 1,
            audit_clean_certified: false,
            hidden_drift_uncovered: true,
        },
    );

    ContestedCheck::new(save, effect)
}

// ============================================================================
// §4 Cyber-Prosthetics & Myomer/Neural Augmentations
// ============================================================================

define_coded_enum! {
    /// Canonical 8-entry 2035 Clinical Cyber-Prosthetic & Neural Augmentation Catalog
    /// (`docs/ttrpg/07-engineering-and-tech/robotics_drones_and_ai_architecture.md` §4.1).
    #[repr(usize)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum CyberAugmentId [count = 8] {
        BionicHandFineActuator => { code: "BIONIC_HAND", label: "Bionic Myoelectric Precision Hand (+2 DEX)", aliases: [] },
        MyomerCyberArm => { code: "MYOMER_ARM", label: "Titanium-Myomer Cybernetic Arm (+2 Force Cap)", aliases: [] },
        CarbonMyomerCyberLegs => { code: "MYOMER_LEGS", label: "Carbon-Myomer Spring-Achilles Legs (+2 Tempo Cap)", aliases: [] },
        RetinalHudAndLowLightOptics => { code: "RETINAL_HUD", label: "Retinal AR HUD & Low-Light/Thermal Optics (+2 SIG)", aliases: ["RETINAL_OPTICS"] },
        CochlearCommsAndAcousticFilter => { code: "COCHLEAR_FILTER", label: "Cochlear Mesh Comms & Tinnitus Gate (+2 HEA)", aliases: ["COCHLEAR"] },
        SubdermalAramidMeshWeave => { code: "SUBDERMAL_ARMOR", label: "Subdermal Dyneema-Aramid Mesh Weave (PR 3/2, BTD 2)", aliases: [] },
        CorticalOodaCoProcessor => { code: "CORTICAL_OODA", label: "Cortical OODA Co-Processor (+2 D_cog, +2 OODA)", aliases: ["OODA_COPROCESSOR"] },
        SpinalReflexMyomerBooster => { code: "SPINAL_REFLEX", label: "Spinal Reflex Myomer Accelerator (+1 Tempo/+1 Control/+1 OODA)", aliases: ["SPINAL_BOOSTER"] },
    }
}

/// Clinical performance, mass, surgical upkeep, and restriction specification for a [`CyberAugmentId`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CyberAugmentSpec {
    pub id: CyberAugmentId,
    pub name: &'static str,
    pub triad_cap_bonus: Triad<i32>,
    pub dex_bonus: i32,
    pub sig_bonus: i32,
    pub hea_bonus: i32,
    pub d_cog_bonus: i32,
    pub ooda_bonus: i32,
    pub subdermal_pr_edged: i32,
    pub subdermal_pr_ballistic: i32,
    pub subdermal_btd: i32,
    pub immune_to_tinnitus: bool,
    pub mass_kg: f64,
    pub install_cost_usd: i32,
    pub monthly_maintenance_usd: i32,
    pub access_class: BlackMarketRestrictionClass,
}

/// Canonical 8-entry Clinical Cyber-Prosthetics & Myomer/Neural Augmentation Catalog.
pub const CYBER_AUGMENT_CATALOG: [CyberAugmentSpec; 8] = [
    CyberAugmentSpec {
        id: CyberAugmentId::BionicHandFineActuator,
        name: "Bionic Myoelectric Precision Hand",
        triad_cap_bonus: Triad::ZERO,
        dex_bonus: 2,
        sig_bonus: 0,
        hea_bonus: 0,
        d_cog_bonus: 0,
        ooda_bonus: 0,
        subdermal_pr_edged: 0,
        subdermal_pr_ballistic: 0,
        subdermal_btd: 0,
        immune_to_tinnitus: false,
        mass_kg: 0.60,
        install_cost_usd: 12000,
        monthly_maintenance_usd: 120,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
    },
    CyberAugmentSpec {
        id: CyberAugmentId::MyomerCyberArm,
        name: "Titanium-Myomer Cybernetic Arm",
        triad_cap_bonus: Triad::new(0, 0, 2),
        dex_bonus: 0,
        sig_bonus: 0,
        hea_bonus: 0,
        d_cog_bonus: 0,
        ooda_bonus: 0,
        subdermal_pr_edged: 0,
        subdermal_pr_ballistic: 0,
        subdermal_btd: 0,
        immune_to_tinnitus: false,
        mass_kg: 1.80,
        install_cost_usd: 28000,
        monthly_maintenance_usd: 280,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
    },
    CyberAugmentSpec {
        id: CyberAugmentId::CarbonMyomerCyberLegs,
        name: "Carbon-Myomer Spring-Achilles Legs",
        triad_cap_bonus: Triad::new(2, 0, 0),
        dex_bonus: 0,
        sig_bonus: 0,
        hea_bonus: 0,
        d_cog_bonus: 0,
        ooda_bonus: 0,
        subdermal_pr_edged: 0,
        subdermal_pr_ballistic: 0,
        subdermal_btd: 0,
        immune_to_tinnitus: false,
        mass_kg: 3.50,
        install_cost_usd: 42000,
        monthly_maintenance_usd: 420,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
    },
    CyberAugmentSpec {
        id: CyberAugmentId::RetinalHudAndLowLightOptics,
        name: "Retinal AR HUD & Low-Light/Thermal Optics",
        triad_cap_bonus: Triad::ZERO,
        dex_bonus: 0,
        sig_bonus: 2,
        hea_bonus: 0,
        d_cog_bonus: 0,
        ooda_bonus: 0,
        subdermal_pr_edged: 0,
        subdermal_pr_ballistic: 0,
        subdermal_btd: 0,
        immune_to_tinnitus: false,
        mass_kg: 0.10,
        install_cost_usd: 18000,
        monthly_maintenance_usd: 180,
        access_class: BlackMarketRestrictionClass::Class2LawEnforcementParamilitary,
    },
    CyberAugmentSpec {
        id: CyberAugmentId::CochlearCommsAndAcousticFilter,
        name: "Cochlear Mesh Comms & Tinnitus Gate",
        triad_cap_bonus: Triad::ZERO,
        dex_bonus: 0,
        sig_bonus: 0,
        hea_bonus: 2,
        d_cog_bonus: 0,
        ooda_bonus: 0,
        subdermal_pr_edged: 0,
        subdermal_pr_ballistic: 0,
        subdermal_btd: 0,
        immune_to_tinnitus: true,
        mass_kg: 0.05,
        install_cost_usd: 11000,
        monthly_maintenance_usd: 110,
        access_class: BlackMarketRestrictionClass::Class1LicensedCivilian,
    },
    CyberAugmentSpec {
        id: CyberAugmentId::SubdermalAramidMeshWeave,
        name: "Subdermal Dyneema-Aramid Mesh Weave",
        triad_cap_bonus: Triad::ZERO,
        dex_bonus: 0,
        sig_bonus: 0,
        hea_bonus: 0,
        d_cog_bonus: 0,
        ooda_bonus: 0,
        subdermal_pr_edged: 3,
        subdermal_pr_ballistic: 2,
        subdermal_btd: 2,
        immune_to_tinnitus: false,
        mass_kg: 2.20,
        install_cost_usd: 35000,
        monthly_maintenance_usd: 350,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
    },
    CyberAugmentSpec {
        id: CyberAugmentId::CorticalOodaCoProcessor,
        name: "Cortical OODA Co-Processor",
        triad_cap_bonus: Triad::ZERO,
        dex_bonus: 0,
        sig_bonus: 0,
        hea_bonus: 0,
        d_cog_bonus: 2,
        ooda_bonus: 2,
        subdermal_pr_edged: 0,
        subdermal_pr_ballistic: 0,
        subdermal_btd: 0,
        immune_to_tinnitus: false,
        mass_kg: 0.15,
        install_cost_usd: 65000,
        monthly_maintenance_usd: 650,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
    },
    CyberAugmentSpec {
        id: CyberAugmentId::SpinalReflexMyomerBooster,
        name: "Spinal Reflex Myomer Accelerator",
        triad_cap_bonus: Triad::new(1, 1, 0),
        dex_bonus: 0,
        sig_bonus: 0,
        hea_bonus: 0,
        d_cog_bonus: 0,
        ooda_bonus: 1,
        subdermal_pr_edged: 0,
        subdermal_pr_ballistic: 0,
        subdermal_btd: 0,
        immune_to_tinnitus: false,
        mass_kg: 1.20,
        install_cost_usd: 58000,
        monthly_maintenance_usd: 580,
        access_class: BlackMarketRestrictionClass::Class3MilitaryOrdnanceFullAuto,
    },
];

impl_static_catalog!(CyberAugmentId => CyberAugmentSpec : CYBER_AUGMENT_CATALOG);

/// Returns all 8 canonical Clinical Cyber-Prosthetics & Augmentation Specifications.
#[must_use]
pub const fn canonical_cyber_augment_catalog() -> &'static [CyberAugmentSpec] {
    &CYBER_AUGMENT_CATALOG
}

/// Complete clinical, physiological, EMP, and Psyche proprioceptive evaluation of a cybernetic loadout
/// (`docs/ttrpg/07-engineering-and-tech/robotics_drones_and_ai_architecture.md` §4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CyberAugmentationLoadoutEval {
    pub augment_count: usize,
    pub triad_cap_bonuses: Triad<i32>,
    pub dex_bonus: i32,
    pub sig_bonus: i32,
    pub hea_bonus: i32,
    pub d_cog_bonus: i32,
    pub ooda_bonus: i32,
    pub subdermal_pr_edged: i32,
    pub subdermal_pr_ballistic: i32,
    pub subdermal_btd: i32,
    pub immune_to_tinnitus: bool,
    pub total_mass_kg: f64,
    pub total_install_cost_usd: i32,
    pub monthly_maintenance_usd: i32,
    pub rejection_delta_int: i32,
    pub psyche_proprioceptive_friction_delta_ext: i32,
    pub emp_vulnerable_count: i32,
    pub emp_reboot_ticks: i32,
}

impl ConditionSource for CyberAugmentationLoadoutEval {
    fn to_condition_impact(&self) -> ConditionImpact {
        ConditionImpact {
            delta_int: self.rejection_delta_int,
            delta_ext: 0,
            ..ConditionImpact::ZERO
        }
    }
}

/// Evaluates installed cyber-prosthetics & neural augmentations for tactical cap gains, subdermal armor,
/// surgical/immunosuppressant rejection $\Delta_{\text{int}}$, proprioceptive alienation friction, and EMP hardness.
#[must_use]
pub fn evaluate_cyber_augmentation_loadout(
    augments: &[CyberAugmentId],
    surgical_quality_q: i32,
    emp_hardened: bool,
    immunosuppressant_compliant: bool,
) -> CyberAugmentationLoadoutEval {
    let mut triad_cap_bonuses = Triad::ZERO;
    let mut dex_bonus = 0;
    let mut sig_bonus = 0;
    let mut hea_bonus = 0;
    let mut d_cog_bonus = 0;
    let mut ooda_bonus = 0;
    let mut subdermal_pr_edged = 0;
    let mut subdermal_pr_ballistic = 0;
    let mut subdermal_btd = 0;
    let mut immune_to_tinnitus = false;
    let mut total_mass_kg = 0.0;
    let mut install_sum = 0;
    let mut monthly_maintenance_usd = 0;

    for id in augments {
        let s = id.spec();
        triad_cap_bonuses += s.triad_cap_bonus;
        dex_bonus += s.dex_bonus;
        sig_bonus += s.sig_bonus;
        hea_bonus += s.hea_bonus;
        d_cog_bonus += s.d_cog_bonus;
        ooda_bonus += s.ooda_bonus;
        subdermal_pr_edged = subdermal_pr_edged.max(s.subdermal_pr_edged);
        subdermal_pr_ballistic = subdermal_pr_ballistic.max(s.subdermal_pr_ballistic);
        subdermal_btd = subdermal_btd.max(s.subdermal_btd);
        immune_to_tinnitus = immune_to_tinnitus || s.immune_to_tinnitus;
        total_mass_kg += s.mass_kg;
        install_sum += s.install_cost_usd;
        monthly_maintenance_usd += s.monthly_maintenance_usd;
    }

    let count = augments.len() as i32;
    let total_install_cost_usd = if emp_hardened {
        (f64::from(install_sum) * 1.25).round() as i32
    } else {
        install_sum
    };
    let base_rejection = if !immunosuppressant_compliant {
        count
    } else {
        (count - 2).max(0)
    };
    let rejection_delta_int = if count == 0 {
        0
    } else {
        (base_rejection + (-surgical_quality_q).max(0)).clamp(0, 6)
    };
    let psyche_proprioceptive_friction_delta_ext = (count - 3).max(0) * 2;
    let emp_vulnerable_count = if emp_hardened { 0 } else { count };
    let emp_reboot_ticks = if count == 0 {
        0
    } else if emp_hardened {
        4
    } else {
        20
    };

    CyberAugmentationLoadoutEval {
        augment_count: augments.len(),
        triad_cap_bonuses,
        dex_bonus,
        sig_bonus,
        hea_bonus,
        d_cog_bonus,
        ooda_bonus,
        subdermal_pr_edged,
        subdermal_pr_ballistic,
        subdermal_btd,
        immune_to_tinnitus,
        total_mass_kg: (total_mass_kg * 100.0).round() / 100.0,
        total_install_cost_usd,
        monthly_maintenance_usd,
        rejection_delta_int,
        psyche_proprioceptive_friction_delta_ext,
        emp_vulnerable_count,
        emp_reboot_ticks,
    }
}
