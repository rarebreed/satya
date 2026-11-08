//! # Canonical 37-Action Reference Catalog & Multi-Sensory Stealth / Perception Framework
//! (docs/ttrpg/02-core-mechanics/action_economy_and_ooda.md §4.1–§4.4, §6.1–§6.3).
//!
//! Separated from [crate::action_economy] (§1–§3, §5 Time Hierarchy, OODA Loop, Action Time Factor,
//! Timeline Sorting & Interrupts) so both modules fit comfortably within a single read window.

use serde::{Deserialize, Serialize};

use crate::action_economy::EncumbranceTier;
use crate::characteristics::{CharacterCharacteristics, CharacteristicCode};
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
    action_spec!(SnapShotPointBlank, CombatAttackOrDefense, "Snap Shot / Point-Blank Reflex Strike", (2, 5, 2), "SPD, DEX, ITU; Pistols / Blades; -2 precision penalty"),
    action_spec!(StandardMeleeStrikeLight, CombatAttackOrDefense, "Standard Melee Strike (Balanced / Light)", (3, 6, 2), "SPD, KIN; Melee Combat; modified by Weapon Balance (B)"),
    action_spec!(StandardRangedFire, CombatAttackOrDefense, "Standard Ranged Fire (Combat Sight Picture)", (3, 7, 2), "DEX, SIG, FOC; Pistols / Long Guns"),
    action_spec!(DefensiveGuardParryStance, CombatAttackOrDefense, "Defensive Guard / Active Parry Stance", (3, 6, 2), "KIN, INS, SPD; Melee / Martial Arts; boosts Guard Pool"),
    action_spec!(TacticalManeuverProbeFeint, CombatAttackOrDefense, "Tactical Maneuver / Probe / Feint", (4, 7, 2), "INS, CRE, KIN; Tactician / Melee; +1 defensive parry margin"),
    action_spec!(AimedShotSingle, CombatAttackOrDefense, "Aimed Shot (Deliberate Single Fire)", (5, 7, 2), "SIG, FOC, DEX; Precision Marksmanship; -2 to -4 to TN"),
    action_spec!(HeavyPowerStrikeCleave, CombatAttackOrDefense, "Heavy Power Strike / Cleave", (3, 11, 2), "FOR, SPD, MAS; Heavy Weapons; +2 Wound Shock on hit"),
    action_spec!(SuppressiveFireBurstZone, CombatAttackOrDefense, "Suppressive Fire (Burst / Automatic Zone)", (3, 9, 2), "FOR, DIS, FOC; Fully Automatic / Heavy Weapons"),
    action_spec!(ActiveDodgeDiveForCover, CombatAttackOrDefense, "Active Dodge / Dive for Cover (Interrupt)", (2, 6, 2), "ITU, SPD, KIN; Evasion; advances actor's Current Tick by +8"),
    action_spec!(OverwatchTriggeredRelease, CombatAttackOrDefense, "Overwatch / Held Trigger Release", (1, 2, 1), "FOC, ITU, SPD; fires when trigger condition is met"),
    // §4.2 Weapon Handling, Reloads & Malfunctions
    action_spec!(DropWeaponOrItem, WeaponHandlingAndReload, "Drop Weapon / Release Held Item", (1, 0, 1), "D_cog; Free reflex; 0 Ticks if combined with drawing sidearm"),
    action_spec!(SwitchFireSelectorOrOptic, WeaponHandlingAndReload, "Switch Fire Selector / Toggle Optic Magnification", (1, 1, 1), "DEX; Firearms N >= 3 reduces to 1 Tick"),
    action_spec!(ClearSimpleMalfunctionTapRack, WeaponHandlingAndReload, "Clear Simple Malfunction (Tap-Rack Drill)", (2, 3, 2), "DEX, TAC; Pistols / Long Guns; -1 Tick at N >= 3, -2 at N >= 5"),
    action_spec!(DrawHolsteredSidearmOrBlade, WeaponHandlingAndReload, "Draw Holstered Sidearm / Sheathed Blade", (1, 3, 2), "SPD, DEX; Pistols / Blades; -1 Tick at N >= 3, -2 at N >= 5 (floor 2)"),
    action_spec!(SlingTransitionRifleToSidearm, WeaponHandlingAndReload, "Sling Transition (Drop Primary on Sling -> Draw Sidearm)", (1, 3, 2), "SPD, DEX, KIN; requires tactical sling; -1 Tick at N >= 4"),
    action_spec!(SpeedReloadDetachableMagDropEmpty, WeaponHandlingAndReload, "Speed Reload — Detachable Box Magazine (Drop Empty)", (1, 5, 3), "DEX, SPD, TAC; -1 Tick at N >= 3, -2 at N >= 5 (floor 3 Ticks = 1.5s)"),
    action_spec!(TacticalReloadRetainPartialMag, WeaponHandlingAndReload, "Tactical Reload — Retain Partial Magazine in Pouch", (2, 7, 3), "DEX, TAC; -1 Tick at N >= 3, -2 at N >= 5"),
    action_spec!(ClearComplexDoubleFeedOrBoltOverride, WeaponHandlingAndReload, "Clear Complex Malfunction (Double-Feed / Bolt Override)", (3, 9, 4), "DEX, TAC, FOC; Armourer / Firearms; requires tool or mag strip"),
    action_spec!(LooseRoundReloadPerCartridge, WeaponHandlingAndReload, "Revolver / Tube Shotgun Reload (Loose Rounds, per cartridge)", (1, 2, 2), "+2 Ticks per additional shell; Speedloader / Violin-load reduces to 6 Ticks total"),
    action_spec!(BeltFedMgLinkReload, WeaponHandlingAndReload, "Belt-Fed Machine Gun / Heavy Weapon Link Reload", (3, 13, 6), "DEX, FOR; Heavy Weapons; Dedicated Assistant Gunner reduces TC_act by -6 Ticks"),
    // §4.3 Movement, Posture & Physical Interaction
    action_spec!(FiveFootTacticalStep, MovementAndPosture, "5-Foot Tactical Step / Shift Weight (<= 1.5 m)", (1, 2, 2), "KIN, SPD; Balance & Footwork; can combine with melee stance"),
    action_spec!(DropProneImmediate, MovementAndPosture, "Drop Prone (Immediate)", (1, 2, 2), "SPD, KIN; Acrobatics N >= 3 reduces to 2 Ticks"),
    action_spec!(StandUpFromKneelingCrouch, MovementAndPosture, "Stand Up from Kneeling / Crouch", (1, 3, 2), "SPD, KIN; penalized by Encumbrance Tier"),
    action_spec!(TacticalBound2To3m, MovementAndPosture, "Tactical Bound / Dash Between Cover (2–3 m)", (1, 4, 2), "SPD, FIT; Running & Sprinting; scales with SPD"),
    action_spec!(StandUpFromProne, MovementAndPosture, "Stand Up from Prone", (1, 5, 3), "SPD, KIN, FOR; Acrobatics / Martial Arts N >= 3 reduces by -2 Ticks"),
    action_spec!(VaultLowObstacleOrWindow, MovementAndPosture, "Vault Low Obstacle / Window Breach", (1, 5, 2), "SPD, KIN, FOR; Climbing / Parkour"),
    action_spec!(OpenUnlockedDoorOrHatch, MovementAndPosture, "Open Unlocked Door / Manipulate Hatch Lever", (1, 3, 2), "DEX, FOR"),
    action_spec!(BreachMechanicalKickRamDoor, MovementAndPosture, "Breach (Mechanical Kick / Battering Ram on Door)", (2, 6, 3), "FOR, MAS, SPD; Strength & Lifting / Demolitions"),
    action_spec!(MountOrDismountVehicle, MovementAndPosture, "Mount / Dismount Vehicle or Saddle", (1, 5, 2), "KIN, SPD; Riding / Driving N >= 3 reduces by -2 Ticks"),
    action_spec!(RetrieveOrStowPackItem, MovementAndPosture, "Retrieve / Stow Item from Backpack or Load-Bearing Rig", (2, 6, 2), "DEX, TAC; Chest-rig / quick-release pouch reduces to 4 Ticks"),
    // §4.4 Cognitive, Perceptual & Communication Actions
    action_spec!(QuickVisualGlanceReactionCheck, CognitivePerceptualAndComms, "Quick Visual Glance / Threat Reaction Check", (2, 0, 1), "SIG, ITU, D_cog; Alertness; reduced to 1 Tick if D_cog >= 14"),
    action_spec!(TacticalCalloutShortCommand, CognitivePerceptualAndComms, "Tactical Callout / Short Verbal Command (<= 5 words)", (2, 1, 1), "D_cog, PRE, ELO; Leadership; 1 Tick if standard squad brevity code"),
    action_spec!(ActiveScanRoomOrSectorSweep, CognitivePerceptualAndComms, "Active Scan (Deliberate Room / Sector Sweep)", (4, 2, 2), "SIG, HEA, FOC; Surveillance & Reconnaissance"),
    action_spec!(AssessTacticalWeaknessAnalyzeTarget, CognitivePerceptualAndComms, "Assess Tactical Weakness / Analyze Target Pattern", (5, 1, 2), "ANA, INS, FOC; Tactician; compressed heavily by high D_cog"),
    action_spec!(ComplexCommsRelayCoordTransmission, CognitivePerceptualAndComms, "Complex Comms Relay / Coordinate Data Transmission", (4, 4, 2), "ANA, MEM, ELO; Electronic Warfare / Comms"),
    action_spec!(EmergencyFieldTriageTourniquet, CognitivePerceptualAndComms, "Emergency Field Triage / Apply Combat Tourniquet", (3, 9, 4), "ANA, DEX, FOC; First Aid / Paramedic; -2 Ticks at N >= 4"),
    action_spec!(ImprovisedGadgetCyberdeckReset, CognitivePerceptualAndComms, "Improvised Gadget / Cyberdeck Quick-Deploy or Reset", (4, 4, 2), "CRE, ANA, DEX; Cybersecurity / Electronics"),
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

/// Movement gait during Stealth & Infiltration (§6.1, calibrated with `movement_vehicles_and_environment.md` §1.2).
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
        SensoryChannel::VisualSigFoc => match (params.illumination, params.countermeasure) {
            (IlluminationCondition::BrightDaylight, _) => 0,
            (IlluminationCondition::DimLightDuskShadows, VisualCountermeasure::None) => 2,
            (IlluminationCondition::DimLightDuskShadows, _) => 0,
            (IlluminationCondition::MoonlightStarlight, VisualCountermeasure::Gen3PlusNvg) => 1,
            (IlluminationCondition::MoonlightStarlight, VisualCountermeasure::ThermalFlir)
                if !params.infiltrator_thermally_shielded =>
            {
                0
            }
            (IlluminationCondition::MoonlightStarlight, _) => 6,
            (IlluminationCondition::HeavyFogRainSmoke, VisualCountermeasure::ThermalFlir)
                if !params.infiltrator_thermally_shielded =>
            {
                0
            }
            (IlluminationCondition::HeavyFogRainSmoke, _) => 8,
            (IlluminationCondition::PitchBlackTotalDarkness, VisualCountermeasure::ThermalFlir)
                if !params.infiltrator_thermally_shielded =>
            {
                0
            }
            (IlluminationCondition::PitchBlackTotalDarkness, VisualCountermeasure::Gen3PlusNvg) => {
                4
            }
            (IlluminationCondition::PitchBlackTotalDarkness, _) => 10,
        },
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
