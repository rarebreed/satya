//! # Canonical 37-Action Reference Catalog & Stealth / Awareness / Investigation Framework
//! (`docs/ttrpg/02-core-mechanics/action_economy_and_ooda.md` §4.1–§4.4, §6;
//! `docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §1–§4).
//!
//! Separated from [crate::action_economy] (§1–§3, §5 Time Hierarchy, OODA Loop, Action Time Factor,
//! Timeline Sorting & Interrupts) so both modules fit comfortably within a single read window.

use serde::{Deserialize, Serialize};

use crate::action_economy::EncumbranceTier;
use crate::characteristics::{
    CharacterCharacteristics, CharacteristicCode, compute_general_awareness_score,
};
use crate::dice_probabilities::{ContestedOutcomeKind, ContestedTieRule};
use crate::primitives::{ContestedCheck, TieredSaveEval};
use crate::{define_coded_enum, impl_static_catalog};
/// Category of an action in the Reference Catalog (§4.1–§4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CatalogActionCategory {
    CombatAttackOrDefense,
    WeaponHandlingAndReload,
    MovementAndPosture,
    CognitivePerceptualAndComms,
}

define_coded_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[repr(usize)]
    pub enum CanonicalActionId [count = 37] {
        SnapShotPointBlank => { code: "SNAP_SHOT_POINT_BLANK", label: "Snap Shot / Point-Blank Reflex Strike" },
        StandardMeleeStrikeLight => { code: "STANDARD_MELEE_STRIKE_LIGHT", label: "Standard Melee Strike (Balanced / Light)" },
        StandardRangedFire => { code: "STANDARD_RANGED_FIRE", label: "Standard Ranged Fire (Combat Sight Picture)" },
        DefensiveGuardParryStance => { code: "DEFENSIVE_GUARD_PARRY_STANCE", label: "Defensive Guard / Active Parry Stance" },
        TacticalManeuverProbeFeint => { code: "TACTICAL_MANEUVER_PROBE_FEINT", label: "Tactical Maneuver / Probe / Feint" },
        AimedShotSingle => { code: "AIMED_SHOT_SINGLE", label: "Aimed Shot (Deliberate Single Fire)" },
        HeavyPowerStrikeCleave => { code: "HEAVY_POWER_STRIKE_CLEAVE", label: "Heavy Power Strike / Cleave" },
        SuppressiveFireBurstZone => { code: "SUPPRESSIVE_FIRE_BURST_ZONE", label: "Suppressive Fire (Burst / Automatic Zone)" },
        ActiveDodgeDiveForCover => { code: "ACTIVE_DODGE_DIVE_FOR_COVER", label: "Active Dodge / Dive for Cover (Interrupt)" },
        OverwatchTriggeredRelease => { code: "OVERWATCH_TRIGGERED_RELEASE", label: "Overwatch / Held Trigger Release" },
        DropWeaponOrItem => { code: "DROP_WEAPON_OR_ITEM", label: "Drop Weapon / Release Held Item" },
        SwitchFireSelectorOrOptic => { code: "SWITCH_FIRE_SELECTOR_OR_OPTIC", label: "Switch Fire Selector / Toggle Optic Magnification" },
        ClearSimpleMalfunctionTapRack => { code: "CLEAR_SIMPLE_MALFUNCTION_TAP_RACK", label: "Clear Simple Malfunction (Tap-Rack Drill)" },
        DrawHolsteredSidearmOrBlade => { code: "DRAW_HOLSTERED_SIDEARM_OR_BLADE", label: "Draw Holstered Sidearm / Sheathed Blade" },
        SlingTransitionRifleToSidearm => { code: "SLING_TRANSITION_RIFLE_TO_SIDEARM", label: "Sling Transition (Drop Primary on Sling -> Draw Sidearm)" },
        SpeedReloadDetachableMagDropEmpty => { code: "SPEED_RELOAD_DETACHABLE_MAG_DROP_EMPTY", label: "Speed Reload — Detachable Box Magazine (Drop Empty)" },
        TacticalReloadRetainPartialMag => { code: "TACTICAL_RELOAD_RETAIN_PARTIAL_MAG", label: "Tactical Reload — Retain Partial Magazine in Pouch" },
        ClearComplexDoubleFeedOrBoltOverride => { code: "CLEAR_COMPLEX_DOUBLE_FEED_OR_BOLT_OVERRIDE", label: "Clear Complex Malfunction (Double-Feed / Bolt Override)" },
        LooseRoundReloadPerCartridge => { code: "LOOSE_ROUND_RELOAD_PER_CARTRIDGE", label: "Revolver / Tube Shotgun Reload (Loose Rounds, per cartridge)" },
        BeltFedMgLinkReload => { code: "BELT_FED_MG_LINK_RELOAD", label: "Belt-Fed Machine Gun / Heavy Weapon Link Reload" },
        FiveFootTacticalStep => { code: "FIVE_FOOT_TACTICAL_STEP", label: "5-Foot Tactical Step / Shift Weight (<= 1.5 m)" },
        DropProneImmediate => { code: "DROP_PRONE_IMMEDIATE", label: "Drop Prone (Immediate)" },
        StandUpFromKneelingCrouch => { code: "STAND_UP_FROM_KNEELING_CROUCH", label: "Stand Up from Kneeling / Crouch" },
        TacticalBound2To3m => { code: "TACTICAL_BOUND_2_TO_3M", label: "Tactical Bound / Dash Between Cover (2–3 m)" },
        StandUpFromProne => { code: "STAND_UP_FROM_PRONE", label: "Stand Up from Prone" },
        VaultLowObstacleOrWindow => { code: "VAULT_LOW_OBSTACLE_OR_WINDOW", label: "Vault Low Obstacle / Window Breach" },
        OpenUnlockedDoorOrHatch => { code: "OPEN_UNLOCKED_DOOR_OR_HATCH", label: "Open Unlocked Door / Manipulate Hatch Lever" },
        BreachMechanicalKickRamDoor => { code: "BREACH_MECHANICAL_KICK_RAM_DOOR", label: "Breach (Mechanical Kick / Battering Ram on Door)" },
        MountOrDismountVehicle => { code: "MOUNT_OR_DISMOUNT_VEHICLE", label: "Mount / Dismount Vehicle or Saddle" },
        RetrieveOrStowPackItem => { code: "RETRIEVE_OR_STOW_PACK_ITEM", label: "Retrieve / Stow Item from Backpack or Load-Bearing Rig" },
        QuickVisualGlanceReactionCheck => { code: "QUICK_VISUAL_GLANCE_REACTION_CHECK", label: "Quick Visual Glance / Threat Reaction Check" },
        TacticalCalloutShortCommand => { code: "TACTICAL_CALLOUT_SHORT_COMMAND", label: "Tactical Callout / Short Verbal Command (<= 5 words)" },
        ActiveScanRoomOrSectorSweep => { code: "ACTIVE_SCAN_ROOM_OR_SECTOR_SWEEP", label: "Active Scan (Deliberate Room / Sector Sweep)" },
        AssessTacticalWeaknessAnalyzeTarget => { code: "ASSESS_TACTICAL_WEAKNESS_ANALYZE_TARGET", label: "Assess Tactical Weakness / Analyze Target Pattern" },
        ComplexCommsRelayCoordTransmission => { code: "COMPLEX_COMMS_RELAY_COORD_TRANSMISSION", label: "Complex Comms Relay / Coordinate Data Transmission" },
        EmergencyFieldTriageTourniquet => { code: "EMERGENCY_FIELD_TRIAGE_TOURNIQUET", label: "Emergency Field Triage / Apply Combat Tourniquet" },
        ImprovisedGadgetCyberdeckReset => { code: "IMPROVISED_GADGET_CYBERDECK_RESET", label: "Improvised Gadget / Cyberdeck Quick-Deploy or Reset" },
    }
}

/// Metadata entry for a canonical action in §4.1–§4.4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalActionSpec {
    pub id: CanonicalActionId,
    pub category: CatalogActionCategory,
    pub name: &'static str,
    pub base_tc_decide: i32,
    pub base_tc_act: i32,
    pub base_total_tc: i32,
    /// Minimum floor in ticks after all compression (`min_floor_ticks`).
    pub min_floor_ticks: i32,
    pub governing_notes: &'static str,
}

macro_rules! action_spec {
    ($id:ident, $cat:ident, $name:literal, ($decide:expr, $act:expr, $floor:expr), $notes:literal) => {
        CanonicalActionSpec {
            id: CanonicalActionId::$id,
            category: CatalogActionCategory::$cat,
            name: $name,
            base_tc_decide: $decide,
            base_tc_act: $act,
            base_total_tc: $decide + $act,
            min_floor_ticks: $floor,
            governing_notes: $notes,
        }
    };
}

const CANONICAL_ACTION_CATALOG_ARRAY: [CanonicalActionSpec; 37] = [
    // §4.1 Combat Actions (Attacks & Defenses)
    action_spec!(
        SnapShotPointBlank,
        CombatAttackOrDefense,
        "Snap Shot / Point-Blank Reflex Strike",
        (2, 5, 2),
        "SPD, DEX, ITU; Pistols / Blades; -2 precision penalty"
    ),
    action_spec!(
        StandardMeleeStrikeLight,
        CombatAttackOrDefense,
        "Standard Melee Strike (Balanced / Light)",
        (3, 6, 2),
        "SPD, KIN; Melee Combat; modified by Weapon Balance (B)"
    ),
    action_spec!(
        StandardRangedFire,
        CombatAttackOrDefense,
        "Standard Ranged Fire (Combat Sight Picture)",
        (3, 7, 2),
        "DEX, SIG, FOC; Pistols / Long Guns"
    ),
    action_spec!(
        DefensiveGuardParryStance,
        CombatAttackOrDefense,
        "Defensive Guard / Active Parry Stance",
        (3, 6, 2),
        "KIN, INS, SPD; Melee / Martial Arts; boosts Guard Pool"
    ),
    action_spec!(
        TacticalManeuverProbeFeint,
        CombatAttackOrDefense,
        "Tactical Maneuver / Probe / Feint",
        (4, 7, 2),
        "INS, CRE, KIN; Tactician / Melee; +1 defensive parry margin"
    ),
    action_spec!(
        AimedShotSingle,
        CombatAttackOrDefense,
        "Aimed Shot (Deliberate Single Fire)",
        (5, 7, 2),
        "SIG, FOC, DEX; Precision Marksmanship; -2 to -4 to TN"
    ),
    action_spec!(
        HeavyPowerStrikeCleave,
        CombatAttackOrDefense,
        "Heavy Power Strike / Cleave",
        (3, 11, 2),
        "FOR, SPD, MAS; Heavy Weapons; +2 Wound Shock on hit"
    ),
    action_spec!(
        SuppressiveFireBurstZone,
        CombatAttackOrDefense,
        "Suppressive Fire (Burst / Automatic Zone)",
        (3, 9, 2),
        "FOR, DIS, FOC; Fully Automatic / Heavy Weapons"
    ),
    action_spec!(
        ActiveDodgeDiveForCover,
        CombatAttackOrDefense,
        "Active Dodge / Dive for Cover (Interrupt)",
        (2, 6, 2),
        "ITU, SPD, KIN; Evasion; advances actor's Current Tick by +8"
    ),
    action_spec!(
        OverwatchTriggeredRelease,
        CombatAttackOrDefense,
        "Overwatch / Held Trigger Release",
        (1, 2, 1),
        "FOC, ITU, SPD; fires when trigger condition is met"
    ),
    // §4.2 Weapon Handling, Reloads & Malfunctions
    action_spec!(
        DropWeaponOrItem,
        WeaponHandlingAndReload,
        "Drop Weapon / Release Held Item",
        (1, 0, 1),
        "D_cog; Free reflex; 0 Ticks if combined with drawing sidearm"
    ),
    action_spec!(
        SwitchFireSelectorOrOptic,
        WeaponHandlingAndReload,
        "Switch Fire Selector / Toggle Optic Magnification",
        (1, 1, 1),
        "DEX; Firearms N >= 3 reduces to 1 Tick"
    ),
    action_spec!(
        ClearSimpleMalfunctionTapRack,
        WeaponHandlingAndReload,
        "Clear Simple Malfunction (Tap-Rack Drill)",
        (2, 3, 2),
        "DEX, TAC; Pistols / Long Guns; -1 Tick at N >= 3, -2 at N >= 5"
    ),
    action_spec!(
        DrawHolsteredSidearmOrBlade,
        WeaponHandlingAndReload,
        "Draw Holstered Sidearm / Sheathed Blade",
        (1, 3, 2),
        "SPD, DEX; Pistols / Blades; -1 Tick at N >= 3, -2 at N >= 5 (floor 2)"
    ),
    action_spec!(
        SlingTransitionRifleToSidearm,
        WeaponHandlingAndReload,
        "Sling Transition (Drop Primary on Sling -> Draw Sidearm)",
        (1, 3, 2),
        "SPD, DEX, KIN; requires tactical sling; -1 Tick at N >= 4"
    ),
    action_spec!(
        SpeedReloadDetachableMagDropEmpty,
        WeaponHandlingAndReload,
        "Speed Reload — Detachable Box Magazine (Drop Empty)",
        (1, 5, 3),
        "DEX, SPD, TAC; -1 Tick at N >= 3, -2 at N >= 5 (floor 3 Ticks = 1.5s)"
    ),
    action_spec!(
        TacticalReloadRetainPartialMag,
        WeaponHandlingAndReload,
        "Tactical Reload — Retain Partial Magazine in Pouch",
        (2, 7, 3),
        "DEX, TAC; -1 Tick at N >= 3, -2 at N >= 5"
    ),
    action_spec!(
        ClearComplexDoubleFeedOrBoltOverride,
        WeaponHandlingAndReload,
        "Clear Complex Malfunction (Double-Feed / Bolt Override)",
        (3, 9, 4),
        "DEX, TAC, FOC; Armourer / Firearms; requires tool or mag strip"
    ),
    action_spec!(
        LooseRoundReloadPerCartridge,
        WeaponHandlingAndReload,
        "Revolver / Tube Shotgun Reload (Loose Rounds, per cartridge)",
        (1, 2, 2),
        "+2 Ticks per additional shell; Speedloader / Violin-load reduces to 6 Ticks total"
    ),
    action_spec!(
        BeltFedMgLinkReload,
        WeaponHandlingAndReload,
        "Belt-Fed Machine Gun / Heavy Weapon Link Reload",
        (3, 13, 6),
        "DEX, FOR; Heavy Weapons; Dedicated Assistant Gunner reduces TC_act by -6 Ticks"
    ),
    // §4.3 Movement, Posture & Physical Interaction
    action_spec!(
        FiveFootTacticalStep,
        MovementAndPosture,
        "5-Foot Tactical Step / Shift Weight (<= 1.5 m)",
        (1, 2, 2),
        "KIN, SPD; Balance & Footwork; can combine with melee stance"
    ),
    action_spec!(
        DropProneImmediate,
        MovementAndPosture,
        "Drop Prone (Immediate)",
        (1, 2, 2),
        "SPD, KIN; Acrobatics N >= 3 reduces to 2 Ticks"
    ),
    action_spec!(
        StandUpFromKneelingCrouch,
        MovementAndPosture,
        "Stand Up from Kneeling / Crouch",
        (1, 3, 2),
        "SPD, KIN; penalized by Encumbrance Tier"
    ),
    action_spec!(
        TacticalBound2To3m,
        MovementAndPosture,
        "Tactical Bound / Dash Between Cover (2–3 m)",
        (1, 4, 2),
        "SPD, FIT; Running & Sprinting; scales with SPD"
    ),
    action_spec!(
        StandUpFromProne,
        MovementAndPosture,
        "Stand Up from Prone",
        (1, 5, 3),
        "SPD, KIN, FOR; Acrobatics / Martial Arts N >= 3 reduces by -2 Ticks"
    ),
    action_spec!(
        VaultLowObstacleOrWindow,
        MovementAndPosture,
        "Vault Low Obstacle / Window Breach",
        (1, 5, 2),
        "SPD, KIN, FOR; Climbing / Parkour"
    ),
    action_spec!(
        OpenUnlockedDoorOrHatch,
        MovementAndPosture,
        "Open Unlocked Door / Manipulate Hatch Lever",
        (1, 3, 2),
        "DEX, FOR"
    ),
    action_spec!(
        BreachMechanicalKickRamDoor,
        MovementAndPosture,
        "Breach (Mechanical Kick / Battering Ram on Door)",
        (2, 6, 3),
        "FOR, MAS, SPD; Strength & Lifting / Demolitions"
    ),
    action_spec!(
        MountOrDismountVehicle,
        MovementAndPosture,
        "Mount / Dismount Vehicle or Saddle",
        (1, 5, 2),
        "KIN, SPD; Riding / Driving N >= 3 reduces by -2 Ticks"
    ),
    action_spec!(
        RetrieveOrStowPackItem,
        MovementAndPosture,
        "Retrieve / Stow Item from Backpack or Load-Bearing Rig",
        (2, 6, 2),
        "DEX, TAC; Chest-rig / quick-release pouch reduces to 4 Ticks"
    ),
    // §4.4 Cognitive, Perceptual & Communication Actions
    action_spec!(
        QuickVisualGlanceReactionCheck,
        CognitivePerceptualAndComms,
        "Quick Visual Glance / Threat Reaction Check",
        (2, 0, 1),
        "SIG, ITU, D_cog; Alertness; reduced to 1 Tick if D_cog >= 14"
    ),
    action_spec!(
        TacticalCalloutShortCommand,
        CognitivePerceptualAndComms,
        "Tactical Callout / Short Verbal Command (<= 5 words)",
        (2, 1, 1),
        "D_cog, PRE, ELO; Leadership; 1 Tick if standard squad brevity code"
    ),
    action_spec!(
        ActiveScanRoomOrSectorSweep,
        CognitivePerceptualAndComms,
        "Active Scan (Deliberate Room / Sector Sweep)",
        (4, 2, 2),
        "SIG, HEA, FOC; Surveillance & Reconnaissance"
    ),
    action_spec!(
        AssessTacticalWeaknessAnalyzeTarget,
        CognitivePerceptualAndComms,
        "Assess Tactical Weakness / Analyze Target Pattern",
        (5, 1, 2),
        "ANA, INS, FOC; Tactician; compressed heavily by high D_cog"
    ),
    action_spec!(
        ComplexCommsRelayCoordTransmission,
        CognitivePerceptualAndComms,
        "Complex Comms Relay / Coordinate Data Transmission",
        (4, 4, 2),
        "ANA, MEM, ELO; Electronic Warfare / Comms"
    ),
    action_spec!(
        EmergencyFieldTriageTourniquet,
        CognitivePerceptualAndComms,
        "Emergency Field Triage / Apply Combat Tourniquet",
        (3, 9, 4),
        "ANA, DEX, FOC; First Aid / Paramedic; -2 Ticks at N >= 4"
    ),
    action_spec!(
        ImprovisedGadgetCyberdeckReset,
        CognitivePerceptualAndComms,
        "Improvised Gadget / Cyberdeck Quick-Deploy or Reset",
        (4, 4, 2),
        "CRE, ANA, DEX; Cybersecurity / Electronics"
    ),
];

impl_static_catalog!(CanonicalActionId => CanonicalActionSpec : CANONICAL_ACTION_CATALOG_ARRAY);

/// Full canonical 37-entry Reference Catalog of Base Action Time Costs (§4.1–§4.4 of `action_economy_and_ooda.md`).
pub const CANONICAL_ACTION_CATALOG: &[CanonicalActionSpec] = &CANONICAL_ACTION_CATALOG_ARRAY;

#[must_use]
pub const fn lookup_canonical_action(id: CanonicalActionId) -> &'static CanonicalActionSpec {
    id.spec()
}

impl CanonicalActionId {
    /// Maps combat-category entries in the 37-Action Reference Catalog (`§4.1`) to their
    /// default [`crate::combat_techniques::CombatTechniqueId`] counterpart.
    #[must_use]
    pub const fn to_combat_technique_id(
        self,
    ) -> Option<crate::combat_techniques::CombatTechniqueId> {
        use crate::combat_techniques::CombatTechniqueId;
        match self {
            Self::SnapShotPointBlank => Some(CombatTechniqueId::SnapShot),
            Self::StandardMeleeStrikeLight => Some(CombatTechniqueId::Punch),
            Self::StandardRangedFire => Some(CombatTechniqueId::CombatSightedShot),
            Self::DefensiveGuardParryStance => Some(CombatTechniqueId::Parry),
            Self::TacticalManeuverProbeFeint => Some(CombatTechniqueId::Probe),
            Self::AimedShotSingle => Some(CombatTechniqueId::DeliberateAimedShot),
            Self::HeavyPowerStrikeCleave => Some(CombatTechniqueId::Swing),
            Self::SuppressiveFireBurstZone => Some(CombatTechniqueId::SuppressiveFullAuto),
            Self::ActiveDodgeDiveForCover => Some(CombatTechniqueId::Dodge),
            Self::OverwatchTriggeredRelease => Some(CombatTechniqueId::OverwatchHoldSector),
            _ => None,
        }
    }
}

/// Movement gait during Stealth & Infiltration (§6.1, calibrated with `movement_encumbrance_and_vehicles.md` §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StealthGait {
    /// Stationary & Camouflaged (`0 m/Tick`, `+0` penalty, benefits from Camouflage & Concealment)
    StationaryCamouflaged,
    /// Stealth Creep / Crouch-Walk (`18%` of $V_{\text{sprint}}$, `~0.34 m/Tick` at `SPD 10`, `+0` movement penalty)
    StealthCreep,
    /// Combat Glide / Tactical Walk (`35%` of $V_{\text{sprint}}$, `~0.65 m/Tick` at `SPD 10`, `+4` $\Delta_{\text{ext}}$ penalty to Stealth)
    CombatGlide,
    /// Bounding Run / All-Out Sprint (`65–100%` of $V_{\text{sprint}}$, `1.22–1.88 m/Tick` at `SPD 10`, `+8` $\Delta_{\text{ext}}$ penalty to Stealth)
    RunningDash,
}

impl StealthGait {
    /// External modifier ($\Delta_{\text{ext}}$) imposed on the infiltrator's Stealth roll (§6.1).
    #[must_use]
    pub const fn stealth_movement_delta_ext(self) -> i32 {
        match self {
            Self::StationaryCamouflaged | Self::StealthCreep => 0,
            Self::CombatGlide => 4,
            Self::RunningDash => 8,
        }
    }
}

/// Sensory channel used by an observer in a Multi-Sensory Perception Contest (`action_economy_and_ooda.md` §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SensoryChannel {
    /// Visual Detection: `SIG / FOC` (`Alertness` or `Surveillance & Reconnaissance`)
    VisualSigFoc,
    /// Auditory Detection: `HEA / FOC` (`Alertness`)
    AuditoryHeaFoc,
    /// Olfactory / Chemical Detection: `OLF / INS` (`Alertness` or `Tracking`)
    OlfactoryOlfIns,
    /// Intuition / Sixth Sense ("Hair on the Back of the Neck"): `ITU / INS` (`Alertness`)
    SixthSenseItuIns,
}

impl SensoryChannel {
    /// Canonical `(primary, secondary)` [`CharacteristicCode`] pair governing this sensory channel (`action_economy_and_ooda.md` §6.1).
    #[must_use]
    pub const fn controllers(self) -> (CharacteristicCode, CharacteristicCode) {
        match self {
            Self::VisualSigFoc => (CharacteristicCode::SIG, CharacteristicCode::FOC),
            Self::AuditoryHeaFoc => (CharacteristicCode::HEA, CharacteristicCode::FOC),
            Self::OlfactoryOlfIns => (CharacteristicCode::OLF, CharacteristicCode::INS),
            Self::SixthSenseItuIns => (CharacteristicCode::ITU, CharacteristicCode::INS),
        }
    }
}

/// Illumination and atmospheric visibility conditions (`action_economy_and_ooda.md` §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IlluminationCondition {
    /// Bright Daylight / Well-Lit Room (`+0` visual $\Delta_{\text{ext}}$, `+4` visual stealth $\Delta_{\text{ext}}$)
    BrightDaylight,
    /// Dusk / Twilight / Urban Streetlights / Shadows (`+2` visual $\Delta_{\text{ext}}$, `-2` visual stealth $\Delta_{\text{ext}}$)
    DimLightDuskShadows,
    /// Moonlight / Starlight / Power Outage Indoors (`+6` unaided visual $\Delta_{\text{ext}}$; `+1` with Gen-3+ NVG; `-6` visual stealth $\Delta_{\text{ext}}$)
    MoonlightStarlight,
    /// Dense Fog / Heavy Rain / Thick Smoke (`+8` unaided visual $\Delta_{\text{ext}}$, `+2` auditory $\Delta_{\text{ext}}$; `+0` with Thermal/FLIR; `-8` visual stealth $\Delta_{\text{ext}}$)
    HeavyFogRainSmoke,
    /// Pitch Black / Total Darkness (`+10` / Blinded unaided visual $\Delta_{\text{ext}}$; `+0` with Thermal/FLIR)
    PitchBlackTotalDarkness,
}

impl IlluminationCondition {
    /// Unaided visual check (`SIG`) $\Delta_{\text{ext}}$ penalty per `action_economy_and_ooda.md` §6.2.
    #[must_use]
    pub const fn unaided_visual_check_delta_ext(self) -> i32 {
        match self {
            Self::BrightDaylight => 0,
            Self::DimLightDuskShadows => 2,
            Self::MoonlightStarlight => 6,
            Self::HeavyFogRainSmoke => 8,
            Self::PitchBlackTotalDarkness => 10,
        }
    }

    /// Visual stealth concealment modifier ($\Delta_{\text{ext}}$) per `action_economy_and_ooda.md` §6.2.
    #[must_use]
    pub const fn visual_stealth_delta_ext(self) -> i32 {
        match self {
            Self::BrightDaylight => 4,
            Self::DimLightDuskShadows => -2,
            Self::MoonlightStarlight => -6,
            Self::HeavyFogRainSmoke => -8,
            Self::PitchBlackTotalDarkness => -10,
        }
    }
}

/// Electro-optical countermeasure worn or mounted by the observer (`action_economy_and_ooda.md` §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VisualCountermeasure {
    /// Unaided human vision (`None`).
    None,
    /// Gen-3+ Night Vision Goggles (reduces `MoonlightStarlight` penalty to `+1`)
    Gen3PlusNvg,
    /// Thermal / FLIR Imaging (reduces `HeavyFogRainSmoke` and `PitchBlackTotalDarkness` to `+0` unless target is thermally shielded)
    ThermalFlir,
}

/// Structured input context for a Contested Stealth vs. Multi-Sensory Perception exchange (§6.1–§6.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StealthExchangeParams {
    pub infiltrator: CharacterCharacteristics,
    pub infiltrator_gait: StealthGait,
    pub infiltrator_encumbrance: EncumbranceTier,
    /// Camouflage & Concealment $\Delta_{\text{ext}}$ bonus (applied when stationary).
    pub infiltrator_camouflage_bonus_delta_ext: i32,
    pub infiltrator_thermally_shielded: bool,
    pub observer: CharacterCharacteristics,
    pub observer_channel: SensoryChannel,
    pub illumination: IlluminationCondition,
    pub countermeasure: VisualCountermeasure,
}

impl StealthExchangeParams {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_characteristics(
        infiltrator: &CharacterCharacteristics,
        infiltrator_gait: StealthGait,
        infiltrator_encumbrance: EncumbranceTier,
        infiltrator_camouflage_bonus_delta_ext: i32,
        infiltrator_thermally_shielded: bool,
        observer: &CharacterCharacteristics,
        observer_channel: SensoryChannel,
        illumination: IlluminationCondition,
        countermeasure: VisualCountermeasure,
    ) -> Self {
        Self {
            infiltrator: infiltrator.clone(),
            infiltrator_gait,
            infiltrator_encumbrance,
            infiltrator_camouflage_bonus_delta_ext,
            infiltrator_thermally_shielded,
            observer: observer.clone(),
            observer_channel,
            illumination,
            countermeasure,
        }
    }
}

/// Computed modifiers for a Contested Stealth vs. Multi-Sensory Perception exchange (§6.1–§6.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StealthPerceptionModifiers {
    /// Infiltrator's composite Stealth controller ($\lfloor(\text{KIN} + \text{ITU}) / 2\rfloor$).
    pub infiltrator_controller_c: i32,
    /// Total external modifier ($\Delta_{\text{ext}}$) on the infiltrator's Stealth check.
    pub infiltrator_delta_ext: i32,
    pub observer_channel: SensoryChannel,
    pub observer_controller_c: i32,
    /// Total environmental/illumination penalty ($\Delta_{\text{ext}}$) on the observer's Perception check.
    pub observer_delta_ext: i32,
}

/// Shared illumination + countermeasure visual channel penalty helper (`action_economy_and_ooda.md` §6,
/// `stealth_awareness_and_investigation.md` §3.1).
const fn compute_illumination_visual_delta_ext(
    illumination: IlluminationCondition,
    countermeasure: VisualCountermeasure,
    infiltrator_thermally_shielded: bool,
) -> i32 {
    match (illumination, countermeasure) {
        (IlluminationCondition::BrightDaylight, _) => 0,
        (IlluminationCondition::DimLightDuskShadows, VisualCountermeasure::None) => 2,
        (IlluminationCondition::DimLightDuskShadows, _) => 0,
        (IlluminationCondition::MoonlightStarlight, VisualCountermeasure::Gen3PlusNvg) => 1,
        (IlluminationCondition::MoonlightStarlight, VisualCountermeasure::ThermalFlir)
            if !infiltrator_thermally_shielded =>
        {
            0
        }
        (IlluminationCondition::MoonlightStarlight, _) => 6,
        (IlluminationCondition::HeavyFogRainSmoke, VisualCountermeasure::ThermalFlir)
            if !infiltrator_thermally_shielded =>
        {
            0
        }
        (IlluminationCondition::HeavyFogRainSmoke, _) => 8,
        (IlluminationCondition::PitchBlackTotalDarkness, VisualCountermeasure::ThermalFlir)
            if !infiltrator_thermally_shielded =>
        {
            0
        }
        (IlluminationCondition::PitchBlackTotalDarkness, VisualCountermeasure::Gen3PlusNvg) => 4,
        (IlluminationCondition::PitchBlackTotalDarkness, _) => 10,
    }
}

/// Compute the canonical Stealth (`KIN/ITU`) and Observer Perception (`SIG/FOC`, `HEA/FOC`, `OLF/INS`, `ITU/INS`)
/// controllers and $\Delta_{\text{ext}}$ modifiers (§6.1–§6.2).
#[must_use]
pub fn compute_stealth_and_perception_modifiers(
    params: &StealthExchangeParams,
) -> StealthPerceptionModifiers {
    let infiltrator_controller_c = params
        .infiltrator
        .compute_controller(CharacteristicCode::KIN, Some(CharacteristicCode::ITU));
    let cam_mod = if params.infiltrator_gait == StealthGait::StationaryCamouflaged {
        params.infiltrator_camouflage_bonus_delta_ext
    } else {
        0
    };
    let infiltrator_delta_ext = params.infiltrator_gait.stealth_movement_delta_ext()
        + params.infiltrator_encumbrance.stealth_penalty_delta_ext()
        + cam_mod;

    let (obs_primary, obs_secondary) = params.observer_channel.controllers();
    let observer_controller_c = params
        .observer
        .compute_controller(obs_primary, Some(obs_secondary));

    let observer_delta_ext = match params.observer_channel {
        SensoryChannel::VisualSigFoc => compute_illumination_visual_delta_ext(
            params.illumination,
            params.countermeasure,
            params.infiltrator_thermally_shielded,
        ),
        SensoryChannel::AuditoryHeaFoc => match params.illumination {
            IlluminationCondition::HeavyFogRainSmoke => 2,
            _ => 0,
        },
        SensoryChannel::OlfactoryOlfIns | SensoryChannel::SixthSenseItuIns => 0,
    };

    StealthPerceptionModifiers {
        infiltrator_controller_c,
        infiltrator_delta_ext,
        observer_channel: params.observer_channel,
        observer_controller_c,
        observer_delta_ext,
    }
}

// ============================================================================
// Phase 2 Expansion: Passive Awareness vs. Active Observation, Alertness Ladder,
// Multi-Spectrum & Social Stealth, Surveillance Tail Craft & Forensic Decay
// (`docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §1–§4)
// ============================================================================

/// Whether detection uses background Passive Awareness (`0 Ticks`, `AWR`) or deliberate Active
/// Observation (`6 Ticks` tactical / `15 min` thorough sweep, full skill pairs & sense isolation)
/// (`docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObservationMode {
    /// Background ambient notice (`0 Ticks`, GM-hidden): uses derived `AWR` and attention-capped `N`.
    PassiveAwareness,
    /// Deliberate room/sector scan (`6 Ticks` / `1.5s`, or `15 min` thorough search): unlocks full skill `N`.
    ActiveObservation,
}

/// Complete 6-sense channel selector for Active Observation sweeps and passive specialization
/// (`docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActiveSensoryChannel {
    /// Visual scan (`SIG / FOC`).
    VisualSigFoc,
    /// Auditory scan (`HEA / FOC`).
    AuditoryHeaFoc,
    /// Tactile / vibration / tripwire scan (`TAC / FOC`).
    TactileTacFoc,
    /// Olfactory / chemical residue scan (`OLF / INS`).
    OlfactoryOlfIns,
    /// Gustatory / chemical taste / water contamination check (`TAS / INS`).
    GustatoryTasIns,
    /// Intuitive / sixth-sense danger detection (`ITU / INS`).
    SixthSenseItuIns,
}

impl ActiveSensoryChannel {
    /// Canonical `(primary, secondary)` [`CharacteristicCode`] pair governing this channel.
    #[must_use]
    pub const fn controllers(self) -> (CharacteristicCode, CharacteristicCode) {
        match self {
            Self::VisualSigFoc => (CharacteristicCode::SIG, CharacteristicCode::FOC),
            Self::AuditoryHeaFoc => (CharacteristicCode::HEA, CharacteristicCode::FOC),
            Self::TactileTacFoc => (CharacteristicCode::TAC, CharacteristicCode::FOC),
            Self::OlfactoryOlfIns => (CharacteristicCode::OLF, CharacteristicCode::INS),
            Self::GustatoryTasIns => (CharacteristicCode::TAS, CharacteristicCode::INS),
            Self::SixthSenseItuIns => (CharacteristicCode::ITU, CharacteristicCode::INS),
        }
    }
}

impl From<SensoryChannel> for ActiveSensoryChannel {
    fn from(channel: SensoryChannel) -> Self {
        match channel {
            SensoryChannel::VisualSigFoc => Self::VisualSigFoc,
            SensoryChannel::AuditoryHeaFoc => Self::AuditoryHeaFoc,
            SensoryChannel::OlfactoryOlfIns => Self::OlfactoryOlfIns,
            SensoryChannel::SixthSenseItuIns => Self::SixthSenseItuIns,
        }
    }
}

define_coded_enum! {
    /// Attention posture of an observer prior to or during a detection check
    /// (`docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §1.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[repr(usize)]
    pub enum ObserverAttentionState [count = 4] {
        /// Distracted / Engrossed (`+4` $\Delta_{\text{ext}}$, passive pairs capped at `N = 1`).
        DistractedEngrossed => {
            code: "DISTRACTED",
            label: "Distracted / Engrossed in Task or Screen",
            aliases: ["DISTRACTED_ENGROSSED"],
        },
        /// Routine Patrol / Bored Sentry (`+2` $\Delta_{\text{ext}}$, passive pairs capped at `N = 1`).
        RoutinePatrolBored => {
            code: "ROUTINE_PATROL",
            label: "Routine Patrol / Repetitive Watch",
            aliases: ["ROUTINE_PATROL_BORED", "BORED"],
        },
        /// Vigilant Watch (`+0` $\Delta_{\text{ext}}$, passive `N = max(1, skill_n / 2)`).
        VigilantWatch => {
            code: "VIGILANT",
            label: "Vigilant Watch / Disciplined Overwatch",
            aliases: ["VIGILANT_WATCH"],
        },
        /// High-Alert Hunting (`-2` $\Delta_{\text{ext}}$, shifts automatically to Active `N = skill_n`).
        HighAlertHunting => {
            code: "HIGH_ALERT",
            label: "High Alert / Actively Hunting Anomalies",
            aliases: ["HIGH_ALERT_HUNTING", "HUNTING"],
        },
    }
}

impl ObserverAttentionState {
    /// External difficulty modifier ($\Delta_{\text{ext}}$) applied to the observer's check.
    #[must_use]
    pub const fn attention_delta_ext(self) -> i32 {
        match self {
            Self::DistractedEngrossed => 4,
            Self::RoutinePatrolBored => 2,
            Self::VigilantWatch => 0,
            Self::HighAlertHunting => -2,
        }
    }

    /// Effective number of `2d20` pairs the observer rolls given [`ObservationMode`] and trained skill rank `observer_skill_n`.
    #[must_use]
    pub const fn effective_observation_pairs(
        self,
        mode: ObservationMode,
        observer_skill_n: usize,
    ) -> usize {
        let full_n = if observer_skill_n < 1 {
            1
        } else {
            observer_skill_n
        };
        match (mode, self) {
            (ObservationMode::ActiveObservation, _) | (_, Self::HighAlertHunting) => full_n,
            (
                ObservationMode::PassiveAwareness,
                Self::DistractedEngrossed | Self::RoutinePatrolBored,
            ) => 1,
            (ObservationMode::PassiveAwareness, Self::VigilantWatch) => {
                let half = full_n / 2;
                if half < 1 { 1 } else { half }
            }
        }
    }
}

/// Computes Watch Duty Fatigue ($\Delta_{\text{ext}}$ penalty on static/monotonous watch):
/// `+1` per full `2 hours` continuously on post, capped at `+4` (`stealth_awareness_and_investigation.md` §1.1).
#[must_use]
pub fn compute_watch_duty_fatigue_delta_ext(watch_hours: f64) -> i32 {
    if watch_hours <= 0.0 {
        return 0;
    }
    ((watch_hours / 2.0).floor() as i32).clamp(0, 4)
}

define_coded_enum! {
    /// Four-stage escalating Alertness & Suspicion Ladder (`stealth_awareness_and_investigation.md` §2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[repr(usize)]
    pub enum AlertnessLevel [count = 4] {
        /// Level 0: Unaware (`+10` base / `+8..+12` Ambush OODA Freeze; Routine Patrol).
        Level0Unaware => {
            code: "UNAWARE",
            label: "Level 0: Unaware / Baseline Routine",
            aliases: ["LEVEL_0_UNAWARE"],
        },
        /// Level 1: Anomaly / Suspicious (`+4` Surprise Freeze; shifts observer to `HighAlertHunting`).
        Level1AnomalySuspicious => {
            code: "SUSPICIOUS",
            label: "Level 1: Anomaly Detected / Investigating Noise or Glitch",
            aliases: ["LEVEL_1_ANOMALY_SUSPICIOUS"],
        },
        /// Level 2: Alerted & Searching (`+0` Surprise Freeze; calls backup, sweeps sector).
        Level2AlertedSearching => {
            code: "ALERTED_SEARCHING",
            label: "Level 2: Alerted & Sweeping Sector (Backup Called)",
            aliases: ["LEVEL_2_ALERTED_SEARCHING"],
        },
        /// Level 3: Located / Compromised (Combat Clock engaged immediately, `+0` Surprise Freeze).
        Level3LocatedCompromised => {
            code: "LOCATED_COMPROMISED",
            label: "Level 3: Target Located / Active Engagement",
            aliases: ["LEVEL_3_LOCATED_COMPROMISED"],
        },
    }
}

impl AlertnessLevel {
    /// Surprise OODA Freeze ticks suffered by this observer if ambushed from this alertness tier.
    #[must_use]
    pub const fn surprise_freeze_ticks(self) -> i32 {
        match self {
            Self::Level0Unaware => 10,
            Self::Level1AnomalySuspicious => 4,
            Self::Level2AlertedSearching | Self::Level3LocatedCompromised => 0,
        }
    }

    /// Default attention posture associated with this Alertness tier.
    #[must_use]
    pub const fn default_attention_state(self) -> ObserverAttentionState {
        match self {
            Self::Level0Unaware => ObserverAttentionState::RoutinePatrolBored,
            Self::Level1AnomalySuspicious
            | Self::Level2AlertedSearching
            | Self::Level3LocatedCompromised => ObserverAttentionState::HighAlertHunting,
        }
    }

    /// Escalate alertness by `steps`, clamped at `Level3LocatedCompromised`.
    #[must_use]
    pub const fn escalate(self, steps: u8) -> Self {
        let next = (self as usize) + (steps as usize);
        if next >= Self::ALL.len() - 1 {
            Self::ALL[Self::ALL.len() - 1]
        } else {
            Self::ALL[next]
        }
    }

    /// De-escalate alertness by `steps`, clamped at `Level0Unaware`.
    #[must_use]
    pub const fn deescalate(self, steps: u8) -> Self {
        let cur = self as usize;
        let next = cur.saturating_sub(steps as usize);
        Self::ALL[next]
    }
}

/// Output of an Alertness Ladder transition after a Stealth vs. Perception contest
/// (`docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertnessTransitionEval {
    pub previous_level: AlertnessLevel,
    pub new_level: AlertnessLevel,
    pub surprise_freeze_ticks: i32,
    pub recommended_attention_state: ObserverAttentionState,
    pub triggered_combat_clock: bool,
    pub calls_backup: bool,
}

/// Advance or settle a guard/observer's [`AlertnessLevel`] from an infiltrator's contested
/// stealth roll outcome (`stealth_awareness_and_investigation.md` §2).
#[must_use]
pub const fn evaluate_alertness_transition(
    current: AlertnessLevel,
    infiltrator_outcome: ContestedOutcomeKind,
) -> AlertnessTransitionEval {
    let new_level = match infiltrator_outcome {
        ContestedOutcomeKind::ExtraordinarySuccess => match current {
            AlertnessLevel::Level1AnomalySuspicious => current.deescalate(1),
            _ => current,
        },
        ContestedOutcomeKind::CleanSuccess => current,
        ContestedOutcomeKind::GrazingTieSuccess
        | ContestedOutcomeKind::LuckyBreakSuccess
        | ContestedOutcomeKind::SuccessWithFloorBreach => current.escalate(1),
        ContestedOutcomeKind::CleanFailure => current.escalate(2),
        ContestedOutcomeKind::BotchFailure => AlertnessLevel::Level3LocatedCompromised,
    };
    AlertnessTransitionEval {
        previous_level: current,
        new_level,
        surprise_freeze_ticks: new_level.surprise_freeze_ticks(),
        recommended_attention_state: new_level.default_attention_state(),
        triggered_combat_clock: matches!(new_level, AlertnessLevel::Level3LocatedCompromised),
        calls_backup: matches!(
            new_level,
            AlertnessLevel::Level2AlertedSearching | AlertnessLevel::Level3LocatedCompromised
        ),
    }
}

crate::define_coded_enum! {
    /// Floor/terrain acoustic signature during movement (`stealth_awareness_and_investigation.md` §3.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum AcousticSurfaceKind [count = 5] {
        ThickCarpetOrTurf => { code: "THICK_CARPET_OR_TURF", label: "Thick Carpet / Turf" },
        ConcreteAsphaltOrTile => { code: "CONCRETE_ASPHALT_OR_TILE", label: "Concrete / Asphalt / Tile" },
        HardwoodOrCreakyStairs => { code: "HARDWOOD_OR_CREAKY_STAIRS", label: "Hardwood / Creaky Stairs" },
        GravelOrDryLeaves => { code: "GRAVEL_OR_DRY_LEAVES", label: "Gravel / Dry Leaves" },
        BrokenGlassOrMetalGrate => { code: "BROKEN_GLASS_OR_METAL_GRATE", label: "Broken Glass / Metal Grate" },
    }
}

impl AcousticSurfaceKind {
    /// Movement stealth modifier ($\Delta_{\text{ext}}$) when crossing this surface.
    #[must_use]
    pub const fn infiltrator_stealth_delta_ext(self) -> i32 {
        match self {
            Self::ThickCarpetOrTurf => -2,
            Self::ConcreteAsphaltOrTile => 0,
            Self::HardwoodOrCreakyStairs => 2,
            Self::GravelOrDryLeaves => 4,
            Self::BrokenGlassOrMetalGrate => 6,
        }
    }
}

/// Environmental background noise masking (`stealth_awareness_and_investigation.md` §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AmbientNoiseMask {
    DeadSilentVault,
    QuietSuburbanNight,
    HvacOrSteadyRain,
    UrbanTrafficOrCrowd,
    HeavyStormMachineryOrClub,
}

impl AmbientNoiseMask {
    /// Stealth modifier ($\Delta_{\text{ext}}$) granted to a moving infiltrator (negative = easier to mask noise).
    #[must_use]
    pub const fn infiltrator_stealth_delta_ext(self) -> i32 {
        match self {
            Self::DeadSilentVault => 2,
            Self::QuietSuburbanNight => 0,
            Self::HvacOrSteadyRain => -2,
            Self::UrbanTrafficOrCrowd => -4,
            Self::HeavyStormMachineryOrClub => -6,
        }
    }

    /// Auditory perception difficulty penalty ($\Delta_{\text{ext}}$) imposed on the listener.
    #[must_use]
    pub const fn observer_auditory_delta_ext(self) -> i32 {
        -self.infiltrator_stealth_delta_ext()
    }
}

/// Thermal/infrared signature profile (`stealth_awareness_and_investigation.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThermalStealthProfile {
    /// Unshielded human body heat (`+0` vs FLIR).
    UnshieldedStandard,
    /// Hot vehicle engine, generator, or recently fired suppressor (`-4` $\Delta_{\text{ext}}$ for FLIR observer: easier to spot).
    HotEquipmentOrSuppressor,
    /// Myomer-baffled thermal cloak or phase-change suit (`+6` $\Delta_{\text{ext}}$ penalty to FLIR observer).
    ThermalBaffledCloak,
}

impl ThermalStealthProfile {
    /// Observer penalty ($\Delta_{\text{ext}}$) applied specifically when using [`VisualCountermeasure::ThermalFlir`].
    #[must_use]
    pub const fn observer_flir_delta_ext(self) -> i32 {
        match self {
            Self::UnshieldedStandard => 0,
            Self::HotEquipmentOrSuppressor => -4,
            Self::ThermalBaffledCloak => 6,
        }
    }
}

/// Electronic / RF signature hygiene in 2035 surveillance grids (`stealth_awareness_and_investigation.md` §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DigitalRfHygiene {
    /// No active emitters / sealed Faraday pouch (`+0`).
    AirGappedFaradayPouch,
    /// Airplane mode / passive BLE sleep (`+2` electronic leak risk).
    AirplaneModePassiveWearable,
    /// Active phone, smartwatch, or unshielded cyberdeck (`+4` $\Delta_{\text{ext}}$; auto-pings checkpoints at `Heat 3+`).
    ActiveSmartphoneOrBleTrail,
}

impl DigitalRfHygiene {
    /// Infiltrator penalty ($\Delta_{\text{ext}}$) against electronic or RF-aware detection.
    #[must_use]
    pub const fn infiltrator_electronic_delta_ext(self) -> i32 {
        match self {
            Self::AirGappedFaradayPouch => 0,
            Self::AirplaneModePassiveWearable => 2,
            Self::ActiveSmartphoneOrBleTrail => 4,
        }
    }

    /// True if public IoT/IMSI scanners automatically flag a character carrying `Operational Heat >= 3`.
    #[must_use]
    pub const fn auto_pings_heat_3_plus(self) -> bool {
        matches!(self, Self::ActiveSmartphoneOrBleTrail)
    }
}

/// Wardrobe & credential alignment for Social Stealth ("Hiding in Plain Sight",
/// `stealth_awareness_and_investigation.md` §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SocialStealthWardrobeFit {
    AuthenticUniformOrCredentials,
    BlendsIntoCrowdNeutral,
    ConspicuousOrMismatched,
    ArmedOrBloodiedObvious,
}

impl SocialStealthWardrobeFit {
    /// Infiltrator $\Delta_{\text{ext}}$ modifier for social blending checks.
    #[must_use]
    pub const fn infiltrator_social_delta_ext(self) -> i32 {
        match self {
            Self::AuthenticUniformOrCredentials => -2,
            Self::BlendsIntoCrowdNeutral => 0,
            Self::ConspicuousOrMismatched => 4,
            Self::ArmedOrBloodiedObvious => 8,
        }
    }
}

/// Full Phase 2 input bundle for Passive Awareness vs. Active Observation and Multi-Spectrum / Social
/// Stealth (`docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §1–§3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiSpectrumStealthParams {
    pub infiltrator: CharacterCharacteristics,
    pub infiltrator_gait: StealthGait,
    pub infiltrator_encumbrance: EncumbranceTier,
    pub infiltrator_camouflage_bonus_delta_ext: i32,
    pub acoustic_surface: AcousticSurfaceKind,
    pub ambient_noise: AmbientNoiseMask,
    pub thermal_profile: ThermalStealthProfile,
    pub rf_hygiene: DigitalRfHygiene,
    /// When `Some`, resolves Social Stealth ("Hiding in Plain Sight") using `PRE/CRE`.
    pub social_wardrobe_fit: Option<SocialStealthWardrobeFit>,
    /// Cover identity scrutiny resistance (`PR_id`, subtracted from infiltrator $\Delta_{\text{ext}}$ in social stealth).
    pub cover_identity_pr_bonus: i32,
    pub observer: CharacterCharacteristics,
    pub observation_mode: ObservationMode,
    pub observer_attention: ObserverAttentionState,
    pub observer_skill_n: usize,
    pub watch_hours_elapsed: f64,
    pub observer_channel: ActiveSensoryChannel,
    pub illumination: IlluminationCondition,
    pub countermeasure: VisualCountermeasure,
    /// Negative values represent specialized sweep hardware (`NVG`/`FLIR`/Bug Sniffer `-2` to `-4`).
    pub sensor_sweep_tool_delta_mod: i32,
}

/// Output modifiers and clock parameters for [`evaluate_multi_spectrum_stealth_modifiers`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiSpectrumStealthEval {
    pub is_social_stealth: bool,
    pub infiltrator_controller_c: i32,
    pub infiltrator_delta_ext: i32,
    pub effective_observation_mode: ObservationMode,
    pub observer_channel: ActiveSensoryChannel,
    pub observer_controller_c: i32,
    pub observer_effective_pairs_n: usize,
    pub observer_delta_ext: i32,
    pub watch_fatigue_delta_ext: i32,
    pub observer_action_ticks: i32,
    pub auto_pings_heat_3_plus: bool,
}

/// Compute asymmetric Multi-Spectrum or Social Stealth modifiers alongside Passive Awareness (`AWR`)
/// vs. Active Observation (`6 Ticks`) observer parameters (`stealth_awareness_and_investigation.md` §1–§3).
#[must_use]
pub fn evaluate_multi_spectrum_stealth_modifiers(
    params: &MultiSpectrumStealthParams,
) -> MultiSpectrumStealthEval {
    let is_social_stealth = params.social_wardrobe_fit.is_some();
    let (infiltrator_controller_c, infiltrator_delta_ext) = match params.social_wardrobe_fit {
        Some(fit) => (
            params
                .infiltrator
                .compute_controller(CharacteristicCode::PRE, Some(CharacteristicCode::CRE)),
            fit.infiltrator_social_delta_ext() - params.cover_identity_pr_bonus
                + params.rf_hygiene.infiltrator_electronic_delta_ext(),
        ),
        None => {
            let cam_mod = if params.infiltrator_gait == StealthGait::StationaryCamouflaged {
                params.infiltrator_camouflage_bonus_delta_ext
            } else {
                0
            };
            let surface_mod = if params.infiltrator_gait == StealthGait::StationaryCamouflaged {
                0
            } else {
                params.acoustic_surface.infiltrator_stealth_delta_ext()
            };
            (
                params
                    .infiltrator
                    .compute_controller(CharacteristicCode::KIN, Some(CharacteristicCode::ITU)),
                params.infiltrator_gait.stealth_movement_delta_ext()
                    + params.infiltrator_encumbrance.stealth_penalty_delta_ext()
                    + cam_mod
                    + surface_mod
                    + params.ambient_noise.infiltrator_stealth_delta_ext()
                    + params.rf_hygiene.infiltrator_electronic_delta_ext(),
            )
        }
    };

    let effective_observation_mode =
        if params.observer_attention == ObserverAttentionState::HighAlertHunting {
            ObservationMode::ActiveObservation
        } else {
            params.observation_mode
        };
    let observer_effective_pairs_n = params
        .observer_attention
        .effective_observation_pairs(params.observation_mode, params.observer_skill_n);
    let observer_action_ticks = match effective_observation_mode {
        ObservationMode::PassiveAwareness => 0,
        ObservationMode::ActiveObservation => 6,
    };

    let (obs_primary, obs_secondary) = params.observer_channel.controllers();
    let observer_controller_c = match effective_observation_mode {
        ObservationMode::PassiveAwareness => compute_general_awareness_score(
            params.observer.sig,
            params.observer.hea,
            params.observer.foc,
            params.observer.itu,
        ),
        ObservationMode::ActiveObservation => params
            .observer
            .compute_controller(obs_primary, Some(obs_secondary)),
    };

    let watch_fatigue_delta_ext = compute_watch_duty_fatigue_delta_ext(params.watch_hours_elapsed);
    let env_channel_delta_ext = match params.observer_channel {
        ActiveSensoryChannel::VisualSigFoc => {
            let thermally_shielded =
                params.thermal_profile == ThermalStealthProfile::ThermalBaffledCloak;
            let base_vis = compute_illumination_visual_delta_ext(
                params.illumination,
                params.countermeasure,
                thermally_shielded,
            );
            let flir_mod = if params.countermeasure == VisualCountermeasure::ThermalFlir {
                params.thermal_profile.observer_flir_delta_ext()
            } else {
                0
            };
            base_vis + flir_mod
        }
        ActiveSensoryChannel::AuditoryHeaFoc => {
            params.ambient_noise.observer_auditory_delta_ext()
                + match params.illumination {
                    IlluminationCondition::HeavyFogRainSmoke => 2,
                    _ => 0,
                }
        }
        ActiveSensoryChannel::TactileTacFoc
        | ActiveSensoryChannel::OlfactoryOlfIns
        | ActiveSensoryChannel::GustatoryTasIns
        | ActiveSensoryChannel::SixthSenseItuIns => 0,
    };

    let observer_delta_ext = env_channel_delta_ext
        + params.observer_attention.attention_delta_ext()
        + watch_fatigue_delta_ext
        + params.sensor_sweep_tool_delta_mod;

    MultiSpectrumStealthEval {
        is_social_stealth,
        infiltrator_controller_c,
        infiltrator_delta_ext,
        effective_observation_mode,
        observer_channel: params.observer_channel,
        observer_controller_c,
        observer_effective_pairs_n,
        observer_delta_ext,
        watch_fatigue_delta_ext,
        observer_action_ticks,
        auto_pings_heat_3_plus: params.rf_hygiene.auto_pings_heat_3_plus(),
    }
}

/// Tail posture during an urban Surveillance / SDR leg (`stealth_awareness_and_investigation.md` §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TailStance {
    /// Distant interval (`-2` detection risk, `+4` risk of losing target at corners/transfers).
    LooseTail,
    /// Close contact (`+4` detection risk, `-2` tracking penalty).
    TightTail,
}

impl TailStance {
    /// Detection risk shift imposed if the target runs an SDR (`-2` Loose, `+4` Tight).
    #[must_use]
    pub const fn detection_risk_delta_ext(self) -> i32 {
        match self {
            Self::LooseTail => -2,
            Self::TightTail => 4,
        }
    }

    /// Tracking difficulty modifier on the shadower's follow roll (`+4` Loose, `-2` Tight).
    #[must_use]
    pub const fn tracking_delta_ext(self) -> i32 {
        match self {
            Self::LooseTail => 4,
            Self::TightTail => -2,
        }
    }
}

/// Outcome of a single 15–30 minute Surveillance / Tail Craft leg (`stealth_awareness_and_investigation.md` §4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurveillanceTailEffect {
    pub tail_stance: TailStance,
    pub tail_maintained: bool,
    pub sdr_alert_triggered: bool,
    pub target_burned_shadower: bool,
    pub next_leg_delta_ext_shift: i32,
    pub leg_duration_min: i32,
    pub summary: &'static str,
}

/// Resolve one leg of an urban tail or Surveillance Detection Route (`SDR`) contest
/// (`docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §4.1).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_surveillance_tail_exchange(
    tail_stance: TailStance,
    shadower_skill_n: usize,
    shadower_control_c: i32,
    shadower_internal_mod: i32,
    shadower_tool_delta_mod: i32,
    target_sdr_difficulty_delta_ext: i32,
    shadower_pairs: &[(i32, i32)],
    target_gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<SurveillanceTailEffect> {
    let total_ext = target_sdr_difficulty_delta_ext + tail_stance.tracking_delta_ext();
    let eval = TieredSaveEval::evaluate(
        shadower_skill_n,
        shadower_control_c,
        shadower_internal_mod,
        shadower_tool_delta_mod,
        total_ext,
        tie_rule,
        shadower_pairs,
        target_gm_pair,
    );
    let effect = eval.classify_4way(
        |_| SurveillanceTailEffect {
            tail_stance,
            tail_maintained: true,
            sdr_alert_triggered: false,
            target_burned_shadower: false,
            next_leg_delta_ext_shift: -2,
            leg_duration_min: 15,
            summary: "Clean Shadow: target tracked effortlessly across route leg (-2 Delta_ext next leg).",
        },
        |roll| match roll.contested_kind {
            ContestedOutcomeKind::CleanSuccess => SurveillanceTailEffect {
                tail_stance,
                tail_maintained: true,
                sdr_alert_triggered: false,
                target_burned_shadower: false,
                next_leg_delta_ext_shift: 0,
                leg_duration_min: 20,
                summary: "Steady Tail: target kept in sight with no SDR spike.",
            },
            _ => {
                let burned = tail_stance == TailStance::TightTail;
                SurveillanceTailEffect {
                    tail_stance,
                    tail_maintained: true,
                    sdr_alert_triggered: true,
                    target_burned_shadower: burned,
                    next_leg_delta_ext_shift: if burned { 4 } else { 2 },
                    leg_duration_min: 25,
                    summary: if burned {
                        "Tight Tail Spooked Target: maintained visual, but target burned shadower on SDR!"
                    } else {
                        "Close Brush on SDR: tail maintained, but target suspects someone is trailing (+2 next leg)."
                    },
                }
            }
        },
        |_| SurveillanceTailEffect {
            tail_stance,
            tail_maintained: false,
            sdr_alert_triggered: false,
            target_burned_shadower: false,
            next_leg_delta_ext_shift: 4,
            leg_duration_min: 30,
            summary: "Slipped the Tail: target broke visual at transit transfer or corner without spotting shadower.",
        },
        |_| SurveillanceTailEffect {
            tail_stance,
            tail_maintained: false,
            sdr_alert_triggered: true,
            target_burned_shadower: true,
            next_leg_delta_ext_shift: 6,
            leg_duration_min: 30,
            summary: "Burned on Route: tail lost AND target photographed/logged shadower for counter-surveillance!",
        },
    );
    ContestedCheck::new(eval, effect)
}

/// Environmental trace degradation context (`stealth_awareness_and_investigation.md` §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ForensicDecayContext {
    /// Protected interior (`+1` $\Delta_{\text{ext}}$ per full `24 hours`, cap `+8`).
    IndoorsControlled,
    /// Open exterior (`+2` $\Delta_{\text{ext}}$ per full `6 hours`, cap `+10`).
    OutdoorsExposed,
    /// Storm / washout (`+4` $\Delta_{\text{ext}}$ per full `6 hours`, cap `+12`).
    RainOrHeavyWeather,
    /// Bleach / solvent / thermal wipe (`+6` baseline + `+1` per `24 hours`, cap `+12`).
    ProfessionallyScrubbed,
}

impl ForensicDecayContext {
    /// Degradation penalty ($\Delta_{\text{ext}}$) accumulated after `hours_elapsed`.
    #[must_use]
    pub fn decay_delta_ext(self, hours_elapsed: f64) -> i32 {
        let hrs = hours_elapsed.max(0.0);
        match self {
            Self::IndoorsControlled => ((hrs / 24.0).floor() as i32).clamp(0, 8),
            Self::OutdoorsExposed => (((hrs / 6.0).floor() as i32) * 2).clamp(0, 10),
            Self::RainOrHeavyWeather => (((hrs / 6.0).floor() as i32) * 4).clamp(0, 12),
            Self::ProfessionallyScrubbed => (6 + (hrs / 24.0).floor() as i32).clamp(6, 12),
        }
    }
}

/// Investigative depth surfaced by a Forensics, DFIR, or Observation sweep
/// (`docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §4.2–§4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClueDiscoveryTier {
    NoClueFound,
    CoreLeadOnly,
    CoreAndDeepAnalysis,
    HiddenLinkOrZeroCostBreakthrough,
}

/// Outcome of a Forensic Scene Sweep or Deepfake/Synth-Media Audit (`stealth_awareness_and_investigation.md` §4.2–§4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForensicInvestigationEffect {
    pub decay_delta_ext: i32,
    pub total_hidden_delta_ext: i32,
    pub discovery_tier: ClueDiscoveryTier,
    pub core_lead_found: bool,
    pub deep_analysis_found: bool,
    pub hidden_link_zero_extra_time: bool,
    pub evidence_contaminated_or_delayed: bool,
    pub false_lead_generated: bool,
    pub summary: &'static str,
}

/// Resolve a Forensic Scene Sweep, Trace Evidence Recovery, or 2035 Synth-Media / Deepfake Audit
/// (`docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md` §4.2–§4.3).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_forensic_clue_search(
    decay_context: ForensicDecayContext,
    hours_elapsed: f64,
    base_obscurity_delta_ext: i32,
    investigator_skill_n: usize,
    investigator_control_c: i32,
    internal_mod: i32,
    tool_delta_mod: i32,
    investigator_pairs: &[(i32, i32)],
    gm_pair: (i32, i32),
    tie_rule: ContestedTieRule,
) -> ContestedCheck<ForensicInvestigationEffect> {
    let decay_delta_ext = decay_context.decay_delta_ext(hours_elapsed);
    let total_hidden_delta_ext = base_obscurity_delta_ext + decay_delta_ext;
    let eval = TieredSaveEval::evaluate(
        investigator_skill_n,
        investigator_control_c,
        internal_mod,
        tool_delta_mod,
        total_hidden_delta_ext,
        tie_rule,
        investigator_pairs,
        gm_pair,
    );
    let effect = eval.classify_4way(
        |_| ForensicInvestigationEffect {
            decay_delta_ext,
            total_hidden_delta_ext,
            discovery_tier: ClueDiscoveryTier::HiddenLinkOrZeroCostBreakthrough,
            core_lead_found: true,
            deep_analysis_found: true,
            hidden_link_zero_extra_time: true,
            evidence_contaminated_or_delayed: false,
            false_lead_generated: false,
            summary: "Breakthrough Sweep: core lead, deep physical/digital trace, and hidden cross-link recovered in one pass.",
        },
        |roll| match roll.contested_kind {
            ContestedOutcomeKind::CleanSuccess => ForensicInvestigationEffect {
                decay_delta_ext,
                total_hidden_delta_ext,
                discovery_tier: ClueDiscoveryTier::CoreAndDeepAnalysis,
                core_lead_found: true,
                deep_analysis_found: true,
                hidden_link_zero_extra_time: false,
                evidence_contaminated_or_delayed: false,
                false_lead_generated: false,
                summary: "Clean Forensic Recovery: core lead and deep corroborating trace recovered intact.",
            },
            _ => {
                let floor_breach =
                    roll.contested_kind == ContestedOutcomeKind::SuccessWithFloorBreach;
                ForensicInvestigationEffect {
                    decay_delta_ext,
                    total_hidden_delta_ext,
                    discovery_tier: ClueDiscoveryTier::CoreLeadOnly,
                    core_lead_found: true,
                    deep_analysis_found: false,
                    hidden_link_zero_extra_time: false,
                    evidence_contaminated_or_delayed: floor_breach,
                    false_lead_generated: false,
                    summary: if floor_breach {
                        "Partial Lead With Contamination: core clue surfaced, but secondary trace sample degraded."
                    } else {
                        "Partial Recovery: core operational lead recovered; deep trace remains inconclusive."
                    },
                }
            }
        },
        |_| ForensicInvestigationEffect {
            decay_delta_ext,
            total_hidden_delta_ext,
            discovery_tier: ClueDiscoveryTier::NoClueFound,
            core_lead_found: false,
            deep_analysis_found: false,
            hidden_link_zero_extra_time: false,
            evidence_contaminated_or_delayed: false,
            false_lead_generated: false,
            summary: "Trail Cold: trace degradation or obfuscation prevented actionable recovery on this pass.",
        },
        |_| ForensicInvestigationEffect {
            decay_delta_ext,
            total_hidden_delta_ext,
            discovery_tier: ClueDiscoveryTier::NoClueFound,
            core_lead_found: false,
            deep_analysis_found: false,
            hidden_link_zero_extra_time: false,
            evidence_contaminated_or_delayed: true,
            false_lead_generated: true,
            summary: "Botched Sweep: sample destroyed or artifact misread, generating an active false lead!",
        },
    );
    ContestedCheck::new(eval, effect)
}
