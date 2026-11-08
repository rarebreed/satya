//! # Canonical Combat Technique Catalog & Tactical Triad (`TCF`) Interplay (`05-combat`)
//!
//! Separates **Technique Selection (What You Do)** from **Tactical Triad Allocation (`Tempo`, `Control`, `Force` — How You Execute It)**
//! across Unarmed Martial Arts, Armed Melee, and Ranged Combat (`docs/ttrpg/05-combat/tactical_triad_and_pools.md` §4.5,
//! `docs/ttrpg/05-combat/melee_combat.md` §3, `docs/ttrpg/05-combat/ranged_combat_and_armor.md` §2.1):
//! - **Control (`7`)**: `Grab`, `Disarm`, `Takedown`, `Throw`, `Trap`, `Lock`, `Push`
//!   (`Disarm`, `Push`, and `Trap` are also legal in Armed Melee; `Grab`, `Takedown`, `Throw`, and `Lock` require Unarmed/Grappling).
//! - **Strike (`7`)**: Unarmed (`Punch`, `Elbow`, `Knee`, `Kick`, `HeadButt`) & Armed Melee (`Thrust`, `Swing`).
//! - **Intercept (`2`)**: `Block` (Force + Control impact absorption) & `Parry` (Control + Tempo deflection & counter-window amplification).
//! - **Maneuver (`5`)**: `Feint`, `Dodge`, `Escape`, `Probe`, `Root`.
//! - **Ranged Shot (`5`)**: `SnapShot`, `CombatSightedShot`, `DeliberateAimedShot`, `ControlledDoubleTapBurst`, `SuppressiveFullAuto`.
//! - **Ranged Maneuver (`4`)**: `BraceWeapon`, `CoverPeekSlicePie`, `BoundingShootOnMove`, `OverwatchHoldSector`.

use serde::{Deserialize, Serialize};

use crate::action_economy::{
    ActionTimeFactorParams, TwoComponentActionCost, evaluate_combat_action_cost,
};
use crate::combat::{
    TacticalPoolEnvelope, compute_reach_and_clinch_delta_ext, evaluate_tactical_triad,
};
use crate::primitives::{StaticCatalog, Triad};
use crate::{define_coded_enum, impl_static_catalog};

define_coded_enum! {
    /// Combat engagement domain governing which [`CombatTechniqueId`] moves are legal.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum CombatDomain [count = 3] {
        /// Unarmed Martial Arts, Brawling, Striking & Grappling (`Control`, Unarmed `Strike`, `Intercept`, `Maneuver`).
        #[default]
        UnarmedMartialArts => {
            code: "unarmed_martial_arts",
            label: "Unarmed & Martial Arts",
            aliases: ["unarmed", "martial_arts", "grappling"]
        },
        /// Armed Melee Weapons (`Thrust`, `Swing`, weapon-applicable `Control` [`Disarm`, `Push`, `Trap`], `Intercept`, `Maneuver`).
        ArmedMelee => {
            code: "armed_melee",
            label: "Armed Melee Combat",
            aliases: ["melee", "armed", "blade", "blunt"]
        },
        /// Ranged Firearms & Projectiles (`RangedShot`, `RangedManeuver`, plus `Dodge`).
        Ranged => {
            code: "ranged",
            label: "Ranged Combat",
            aliases: ["firearms", "shooting", "marksmanship"]
        },
    }
}

define_coded_enum! {
    /// Functional move category in the Satya Combat Action & Technique taxonomy
    /// (`docs/ttrpg/05-combat/melee_combat.md` §3, `docs/ttrpg/05-combat/ranged_combat_and_armor.md` §2.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum CombatMoveCategory [count = 6] {
        /// Grappling, limb/weapon manipulation, off-balancing & submissions (`Grab`, `Disarm`, `Takedown`, `Throw`, `Trap`, `Lock`, `Push`).
        Control => {
            code: "control",
            label: "Control / Grapple",
            subtitle: "Grab, Disarm, Takedown, Throw, Trap, Lock, Push",
            aliases: ["grapple", "control_grapple", "manipulation"]
        },
        /// Unarmed anatomical strikes (`Punch`, `Elbow`, `Knee`, `Kick`, `HeadButt`) and Armed weapon strikes (`Thrust`, `Swing`).
        Strike => {
            code: "strike",
            label: "Strike",
            subtitle: "Unarmed (Punch, Elbow, Knee, Kick, Head Butt) & Armed (Thrust, Swing)",
            aliases: ["attack", "melee_strike"]
        },
        /// Active defensive & counter-interception reactions (`Block`, `Parry`).
        Intercept => {
            code: "intercept",
            label: "Intercept",
            subtitle: "Block (Force + Control Absorption) & Parry (Control + Tempo Deflection)",
            aliases: ["defense", "defend", "reaction"]
        },
        /// Close-quarters & tactical footwork, deception, reading, and grounding (`Feint`, `Dodge`, `Escape`, `Probe`, `Root`).
        Maneuver => {
            code: "maneuver",
            label: "Maneuver",
            subtitle: "Feint, Dodge, Escape, Probe, Root",
            aliases: ["tactical_maneuver", "footwork"]
        },
        /// Ranged firearm & projectile firing actions (`SnapShot`, `CombatSightedShot`, `DeliberateAimedShot`, `ControlledDoubleTapBurst`, `SuppressiveFullAuto`).
        RangedShot => {
            code: "ranged_shot",
            label: "Ranged Shot",
            subtitle: "Snap Shot, Sighted Shot, Deliberate Aimed Shot, Double-Tap/Burst, Suppressive Zone",
            aliases: ["shot", "fire", "ranged_fire"]
        },
        /// Ranged platform bracing, cover exposure, movement fire, and sector overwatch (`BraceWeapon`, `CoverPeekSlicePie`, `BoundingShootOnMove`, `OverwatchHoldSector`).
        RangedManeuver => {
            code: "ranged_maneuver",
            label: "Ranged Maneuver & Bracing",
            subtitle: "Brace/Rest Weapon, Cover Peek/Slice Pie, Shoot on the Move, Overwatch",
            aliases: ["ranged_stance", "bracing"]
        },
    }
}

define_coded_enum! {
    /// Canonical 30-entry Combat Technique identifier (`docs/ttrpg/05-combat/melee_combat.md` §3, `ranged_combat_and_armor.md` §2.1).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[repr(usize)]
    pub enum CombatTechniqueId [count = 30] {
        // === 1. Control Techniques (0..=6) ===
        Grab => { code: "grab", label: "Grab (Clinch / Grip Entry)", aliases: ["clinch_grab"] },
        Disarm => { code: "disarm", label: "Disarm (Weapon Strip)", aliases: ["weapon_strip"] },
        Takedown => { code: "takedown", label: "Takedown (Leg Shoot / Tackle / Sweep)", aliases: ["leg_takedown", "sweep"] },
        Throw => { code: "throw", label: "Throw (Kuzushi Hip / Shoulder Projection)", aliases: ["kuzushi_throw", "hip_throw"] },
        Trap => { code: "trap", label: "Trap (Limb / Weapon Bind)", aliases: ["limb_trap", "bind"] },
        Lock => { code: "lock", label: "Lock (Joint Lock / Choke / Submission)", aliases: ["joint_lock", "submission", "choke"] },
        Push => { code: "push", label: "Push (Shove / Frame Break)", aliases: ["shove", "frame_break"] },

        // === 2. Unarmed Strike Techniques (7..=11) ===
        Punch => { code: "punch", label: "Punch (Jab / Cross / Hook / Uppercut)", aliases: ["jab", "haymaker", "hook", "cross"] },
        Elbow => { code: "elbow", label: "Elbow Strike (Close / Clinch Slash)", aliases: ["elbow_strike"] },
        Knee => { code: "knee", label: "Knee Strike (Clinch / Step Piston)", aliases: ["knee_strike", "clinch_knee"] },
        Kick => { code: "kick", label: "Kick (Teep / Low Calf / Roundhouse / High)", aliases: ["low_kick", "high_kick", "roundhouse", "teep"] },
        HeadButt => { code: "head_butt", label: "Head Butt (Clinch Crown Strike)", aliases: ["headbutt"] },

        // === 3. Armed Melee Strike Techniques (12..=13) ===
        Thrust => { code: "thrust", label: "Thrust (Linear Point / Lunge / Stab)", aliases: ["stab", "lunge", "point_thrust"] },
        Swing => { code: "swing", label: "Swing (Arcing Cut / Cleave / Bludgeon)", aliases: ["cut", "cleave", "slash", "power_strike"] },

        // === 4. Intercept Techniques (14..=15) ===
        Block => { code: "block", label: "Block (Hard Structural Frame / Shield Check)", aliases: ["hard_block", "shield_block"] },
        Parry => { code: "parry", label: "Parry (Deflection / Redirection)", aliases: ["deflect", "active_parry", "stability"] },

        // === 5. Maneuver Techniques (16..=20) ===
        Feint => { code: "feint", label: "Feint (Deceptive Setup / Second Intention)", aliases: ["deceptive_feint"] },
        Dodge => { code: "dodge", label: "Dodge (Evasive Slip / Sidestep / Dive)", aliases: ["evade", "slip", "sidestep"] },
        Escape => { code: "escape", label: "Escape (Break Clinch / Slip Lock / Disengage)", aliases: ["break_clinch", "disengage"] },
        Probe => { code: "probe", label: "Probe (Range-Finding Check / Read Opponent)", aliases: ["maneuver", "range_probe"] },
        Root => { code: "root", label: "Root (Grounded Stance / Sprawl / Anchor)", aliases: ["sprawl", "grounded_root"] },

        // === 6. Ranged Shot Techniques (21..=25) ===
        SnapShot => { code: "snap_shot", label: "Snap Shot (Hip-Fire / Point-Shoot)", aliases: ["ranged_snap", "point_shoot", "hip_fire"] },
        CombatSightedShot => { code: "combat_sighted_shot", label: "Combat Sighted Shot (Flash Sight Picture)", aliases: ["standard_ranged_fire", "sighted_shot"] },
        DeliberateAimedShot => { code: "deliberate_aimed_shot", label: "Deliberate Aimed Shot (Precision Sighted)", aliases: ["ranged_aimed", "aimed_shot"] },
        ControlledDoubleTapBurst => { code: "controlled_double_tap_burst", label: "Controlled Double-Tap / Short Burst", aliases: ["double_tap", "short_burst"] },
        SuppressiveFullAuto => { code: "suppressive_full_auto", label: "Suppressive Full-Auto Zone", aliases: ["full_auto", "suppressive_fire"] },

        // === 7. Ranged Maneuver & Bracing Techniques (26..=29) ===
        BraceWeapon => { code: "brace_weapon", label: "Brace / Rest Weapon (Bipod / Barricade / Prone)", aliases: ["braced_shot", "bipod_brace"] },
        CoverPeekSlicePie => { code: "cover_peek_slice_pie", label: "Cover Peek / Slice the Pie (Micro-Exposure)", aliases: ["slice_the_pie", "cover_peek"] },
        BoundingShootOnMove => { code: "bounding_shoot_on_move", label: "Bounding Fire / Shoot on the Move", aliases: ["shoot_on_move", "bounding_fire"] },
        OverwatchHoldSector => { code: "overwatch_hold_sector", label: "Overwatch / Hold Sector (Triggered Release)", aliases: ["overwatch"] },
    }
}

/// Canonical biomechanical specification for a [`CombatTechniqueId`] move
/// (`docs/ttrpg/05-combat/melee_combat.md` §3, `docs/ttrpg/05-combat/ranged_combat_and_armor.md` §2.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatTechniqueSpec {
    pub id: CombatTechniqueId,
    pub category: CombatMoveCategory,
    pub name: &'static str,
    pub allowed_unarmed: bool,
    pub allowed_armed_melee: bool,
    pub allowed_ranged: bool,
    pub min_reach: i32,
    pub max_reach: i32,
    pub base_tc_decide: i32,
    pub base_tc_act: i32,
    pub base_total_tc: i32,
    pub min_floor_ticks: i32,
    /// Biomechanical `POW` multiplier (`0.0` for non-damaging maneuvers/intercepts).
    pub pow_multiplier: f64,
    /// Inherent centerline protection (`> 0`) or exposure (`< 0`) added to `Control` (`Guard`).
    pub guard_mod: i32,
    /// Inherent grounded stability modifier added to `Effective Root` ($F + \text{MMF}$).
    pub root_mod: i32,
    /// Armor penetration modifier ($\Delta\text{Pen}$, e.g. `+1` for `Thrust`).
    pub pen_mod: i32,
    /// Ranged fire-mode recoil adder ($\Delta R_{\text{mode}}$; `-3` for `BraceWeapon`).
    pub recoil_adder: i32,
    /// External target number modifier ($\Delta_{\text{ext}}$; `+2` for `SnapShot`, `-2` for `DeliberateAimedShot`).
    pub delta_ext_mod: i32,
    /// Whether this action unlocks a firearm's full positive Accuracy (`Acc > 0`) + Optic bonus.
    pub uses_full_weapon_acc: bool,
    /// Which [`Triad`] axes (`Tempo`, `Control`, `Force`) serve as the primary scaling drivers of this move.
    pub primary_triad_focus: Triad<bool>,
    pub tcf_interplay_notes: &'static str,
}

macro_rules! tech {
    (
        $id:ident, $cat:ident, $name:literal,
        domains: ($unarmed:expr, $armed:expr, $ranged:expr),
        reach: ($min_r:expr, $max_r:expr),
        tc: ($dec:expr, $act:expr, $floor:expr),
        mods: (pow: $pow:expr, guard: $guard:expr, root: $root:expr, pen: $pen:expr, rec: $rec:expr, dext: $dext:expr, acc: $acc:expr),
        focus: ($ft:expr, $fc:expr, $ff:expr),
        $notes:literal
    ) => {
        CombatTechniqueSpec {
            id: CombatTechniqueId::$id,
            category: CombatMoveCategory::$cat,
            name: $name,
            allowed_unarmed: $unarmed,
            allowed_armed_melee: $armed,
            allowed_ranged: $ranged,
            min_reach: $min_r,
            max_reach: $max_r,
            base_tc_decide: $dec,
            base_tc_act: $act,
            base_total_tc: $dec + $act,
            min_floor_ticks: $floor,
            pow_multiplier: $pow,
            guard_mod: $guard,
            root_mod: $root,
            pen_mod: $pen,
            recoil_adder: $rec,
            delta_ext_mod: $dext,
            uses_full_weapon_acc: $acc,
            primary_triad_focus: Triad::new($ft, $fc, $ff),
            tcf_interplay_notes: $notes,
        }
    };
}

/// Canonical 30-entry Combat Technique Catalog (`docs/ttrpg/05-combat/melee_combat.md` §3, `ranged_combat_and_armor.md` §2.1).
pub const COMBAT_TECHNIQUE_CATALOG: [CombatTechniqueSpec; 30] = [
    // === 1. Control Techniques (7) ===
    tech!(
        Grab, Control, "Grab (Clinch / Grip Entry)",
        domains: (true, false, false), reach: (0, 1), tc: (1, 6, 2),
        mods: (pow: 0.0, guard: 0, root: 1, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Establishes Clinch (Reach 0-1) & grips garment/limb; imposes -2 T_max on target until Escape/Push"
    ),
    tech!(
        Disarm, Control, "Disarm (Weapon Strip)",
        domains: (true, true, false), reach: (0, 2), tc: (2, 6, 2),
        mods: (pow: 0.0, guard: 0, root: 0, pen: 0, rec: 0, dext: 1, acc: false),
        focus: (false, true, true),
        "Contests C_att + floor(F_att/2) vs. C_def + F_def; strips held weapon on positive net successes"
    ),
    tech!(
        Takedown, Control, "Takedown (Leg Shoot / Tackle / Sweep)",
        domains: (true, false, false), reach: (1, 2), tc: (2, 7, 2),
        mods: (pow: 0.50, guard: -1, root: 1, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Drives through base (F_att + max(0, T_att - T_def) + max(0, C_att - C_def) vs. Root_def) -> Prone (+6 ticks)"
    ),
    tech!(
        Throw, Control, "Throw (Kuzushi Hip / Shoulder Projection)",
        domains: (true, false, false), reach: (0, 1), tc: (2, 7, 2),
        mods: (pow: 0.85, guard: 1, root: 1, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (false, true, true),
        "Converts Kuzushi off-balancing (max(0, C_att - C_def_eff)) + blitz Tempo into throw leverage vs. Root_def -> Blunt + Prone"
    ),
    tech!(
        Trap, Control, "Trap (Limb / Weapon Bind)",
        domains: (true, true, false), reach: (0, 2), tc: (2, 5, 2),
        mods: (pow: 0.0, guard: 1, root: 0, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, false),
        "Pins or binds striking limb/weapon for 1 exchange; slashes target Control cap by max(1, C_att - C_def)"
    ),
    tech!(
        Lock, Control, "Lock (Joint Lock / Choke / Submission)",
        domains: (true, false, false), reach: (0, 0), tc: (2, 7, 2),
        mods: (pow: 0.75, guard: 0, root: 0, pen: 4, rec: 0, dext: 0, acc: false),
        focus: (false, true, true),
        "Clinch/Ground submission; C_att bypasses armor weave while F_att torques joint (-C_max/-T_max fracture or choke)"
    ),
    tech!(
        Push, Control, "Push (Shove / Frame Break)",
        domains: (true, true, false), reach: (0, 1), tc: (1, 6, 2),
        mods: (pow: 0.25, guard: 0, root: 1, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (false, true, true),
        "Contests F_att + MMF + max(0, C_att - C_def) vs. Root_def; breaks Clinch (+1..+2 Reach) or shoves into obstacle"
    ),

    // === 2. Unarmed Strike Techniques (5) ===
    tech!(
        Punch, Strike, "Punch (Jab / Cross / Hook / Uppercut)",
        domains: (true, false, false), reach: (1, 1), tc: (1, 6, 2),
        mods: (pow: 1.00, guard: 0, root: 0, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Baseline hand strike: high T = fast jab (4-5 ticks); balanced CF = liver hook; high F = knockout haymaker"
    ),
    tech!(
        Elbow, Strike, "Elbow Strike (Close / Clinch Slash)",
        domains: (true, false, false), reach: (0, 1), tc: (1, 5, 2),
        mods: (pow: 1.15, guard: 1, root: 0, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Compact close/clinch strike (+1 Guard); dense olecranon bone inflicts +1 laceration/daze shock on head hits"
    ),
    tech!(
        Knee, Strike, "Knee Strike (Clinch / Step Piston)",
        domains: (true, false, false), reach: (0, 1), tc: (1, 6, 2),
        mods: (pow: 1.25, guard: 0, root: -1, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Heavy close/clinch piston strike (1.25x POW; +2 trauma in Clinch when holding a Grab)"
    ),
    tech!(
        Kick, Strike, "Kick (Teep / Low Calf / Roundhouse / High)",
        domains: (true, false, false), reach: (1, 2), tc: (2, 7, 2),
        mods: (pow: 1.35, guard: -1, root: -2, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Longest unarmed reach (Reach 2) and highest power (1.35x POW), but single-leg stance imposes -1 Guard & -2 Root"
    ),
    tech!(
        HeadButt, Strike, "Head Butt (Clinch Crown Strike)",
        domains: (true, false, false), reach: (0, 0), tc: (1, 4, 2),
        mods: (pow: 0.95, guard: 0, root: 1, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, false, true),
        "Ultra-fast Clinch-only (Reach 0) cranial strike; inflicts +2 Daze/vision delta_int on facial hit; ignores hand traps"
    ),

    // === 3. Armed Melee Strike Techniques (2) ===
    tech!(
        Thrust, Strike, "Thrust (Linear Point / Lunge / Stab)",
        domains: (false, true, false), reach: (1, 4), tc: (1, 6, 2),
        mods: (pow: 1.00, guard: 1, root: 0, pen: 1, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Linear point attack: -1 faster Base TC, +1 centerline Guard, +1 Base Pen (piercing/icepick vs. armor gaps)"
    ),
    tech!(
        Swing, Strike, "Swing (Arcing Cut / Cleave / Bludgeon)",
        domains: (false, true, false), reach: (1, 4), tc: (2, 7, 2),
        mods: (pow: 1.25, guard: -1, root: 0, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Arcing edge or blunt strike: +1 Base TC and -1 Guard, but multiplies weapon POW by 1.25x with +1 guard-crush"
    ),

    // === 4. Intercept Techniques (2) ===
    tech!(
        Block, Intercept, "Block (Hard Structural Frame / Shield Check)",
        domains: (true, true, false), reach: (0, 4), tc: (1, 5, 2),
        mods: (pow: 0.0, guard: 2, root: 1, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (false, true, true),
        "Force + Control interception: +2 Guard, +1 Root, and absorbs floor(F_def / 2) + weapon_block_bonus incoming impact"
    ),
    tech!(
        Parry, Intercept, "Parry (Deflection / Redirection)",
        domains: (true, true, false), reach: (0, 4), tc: (2, 4, 2),
        mods: (pow: 0.0, guard: 3, root: 0, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, false),
        "Control + Tempo deflection: +3 Guard (plus weapon Guard) and 2x recovery ticks inflicted when Seizing the Initiative"
    ),

    // === 5. Maneuver Techniques (5) ===
    tech!(
        Feint, Maneuver, "Feint (Deceptive Setup / Second Intention)",
        domains: (true, true, false), reach: (1, 4), tc: (4, 3, 2),
        mods: (pow: 0.0, guard: 1, root: 0, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, false),
        "Cognitive TC_decide = 4 (compressed by D_cog); each net success strips -2 Guard (Control) from target on next exchange"
    ),
    tech!(
        Dodge, Maneuver, "Dodge (Evasive Slip / Sidestep / Dive)",
        domains: (true, true, true), reach: (0, 4), tc: (2, 4, 2),
        mods: (pow: 0.0, guard: 1, root: -1, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, false),
        "Tempo + Control evasion: adds +floor(T_def / 2) to attacker delta_ext and shifts engagement Reach by +-1 step"
    ),
    tech!(
        Escape, Maneuver, "Escape (Break Clinch / Slip Lock / Disengage)",
        domains: (true, true, false), reach: (0, 2), tc: (2, 5, 2),
        mods: (pow: 0.0, guard: 1, root: 0, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Contests max(T_def, F_def) + C_def vs. grappler C + F to shed Grab/Trap/Lock and open Reach by +1..+2 steps"
    ),
    tech!(
        Probe, Maneuver, "Probe (Range-Finding Check / Read Opponent)",
        domains: (true, true, false), reach: (1, 4), tc: (3, 4, 2),
        mods: (pow: 0.35, guard: 2, root: 0, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, false),
        "Exploratory check (+2 Guard, D_cog compressed); gauges opponent envelope/morale and grants -2 delta_ext on next action"
    ),
    tech!(
        Root, Maneuver, "Root (Grounded Stance / Sprawl / Anchor)",
        domains: (true, true, false), reach: (0, 4), tc: (1, 5, 2),
        mods: (pow: 0.0, guard: 1, root: 4, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (false, true, true),
        "Force + Control grounding: adds +4 + floor(C / 2) to Effective Root (F + MMF) to stuff Takedowns, Throws, and Pushes"
    ),

    // === 6. Ranged Shot Techniques (5) ===
    tech!(
        SnapShot, RangedShot, "Snap Shot (Hip-Fire / Point-Shoot)",
        domains: (false, false, true), reach: (1, 5), tc: (1, 5, 2),
        mods: (pow: 0.0, guard: 0, root: 0, pen: 0, rec: 0, dext: 2, acc: false),
        focus: (true, true, false),
        "Fast CQB point-shoot (Base TC 6): beats slower draw/aim via Tempo, but +2 delta_ext and ignores positive weapon Acc"
    ),
    tech!(
        CombatSightedShot, RangedShot, "Combat Sighted Shot (Flash Sight Picture)",
        domains: (false, false, true), reach: (1, 6), tc: (2, 7, 2),
        mods: (pow: 0.0, guard: 1, root: 0, pen: 0, rec: 0, dext: 0, acc: false),
        focus: (true, true, true),
        "Standard eye-level flash sight picture (Base TC 9): +1 Guard, +0 delta_ext, applies up to +1 weapon Acc"
    ),
    tech!(
        DeliberateAimedShot, RangedShot, "Deliberate Aimed Shot (Precision Sighted)",
        domains: (false, false, true), reach: (2, 8), tc: (4, 8, 2),
        mods: (pow: 0.0, guard: 0, root: 1, pen: 0, rec: 1, dext: -2, acc: true),
        focus: (false, true, true),
        "Deliberate sighted fire (Base TC 12, D_cog compressed): -2 delta_ext aim bonus and unlocks full weapon Acc + Optic"
    ),
    tech!(
        ControlledDoubleTapBurst, RangedShot, "Controlled Double-Tap / Short Burst",
        domains: (false, false, true), reach: (1, 6), tc: (2, 7, 2),
        mods: (pow: 0.0, guard: 0, root: 1, pen: 0, rec: 2, dext: 0, acc: false),
        focus: (true, true, true),
        "Fires 2-3 rounds (Base TC 9, +2 Recoil adder): requires Force >= R_eff to brace muzzle climb; +1 hit per 2 net successes"
    ),
    tech!(
        SuppressiveFullAuto, RangedShot, "Suppressive Full-Auto Zone",
        domains: (false, false, true), reach: (1, 6), tc: (2, 7, 2),
        mods: (pow: 0.0, guard: -1, root: 1, pen: 0, rec: 4, dext: 2, acc: false),
        focus: (false, true, true),
        "Fires 5-15 rounds (Base TC 9, +4 Recoil adder): projects Suppression Morale Shock floor(F / 2) on beaten zone"
    ),

    // === 7. Ranged Maneuver & Bracing Techniques (4) ===
    tech!(
        BraceWeapon, RangedManeuver, "Brace / Rest Weapon (Bipod / Barricade / Prone)",
        domains: (false, false, true), reach: (1, 8), tc: (1, 4, 2),
        mods: (pow: 0.0, guard: 2, root: 3, pen: 0, rec: -3, dext: -1, acc: true),
        focus: (false, true, true),
        "Plants firearm on cover/bipod/prone: slashes Effective Recoil R_eff by -3, Wobble by -1, and delta_ext by -1"
    ),
    tech!(
        CoverPeekSlicePie, RangedManeuver, "Cover Peek / Slice the Pie (Micro-Exposure)",
        domains: (false, false, true), reach: (1, 7), tc: (3, 5, 2),
        mods: (pow: 0.0, guard: 3, root: 1, pen: 0, rec: 0, dext: 0, acc: true),
        focus: (true, true, false),
        "Exposes only eyes/muzzle around hard cover (TC_decide = 3): grants +3 Cover Guard (C_def) against return fire"
    ),
    tech!(
        BoundingShootOnMove, RangedManeuver, "Bounding Fire / Shoot on the Move",
        domains: (false, false, true), reach: (1, 5), tc: (2, 8, 2),
        mods: (pow: 0.0, guard: 0, root: -2, pen: 0, rec: 3, dext: 1, acc: false),
        focus: (true, true, true),
        "Fires while advancing/displacing (+3 moving Recoil adder): requires Tempo to bound and Force to absorb moving recoil"
    ),
    tech!(
        OverwatchHoldSector, RangedManeuver, "Overwatch / Hold Sector (Triggered Release)",
        domains: (false, false, true), reach: (1, 8), tc: (2, 2, 1),
        mods: (pow: 0.0, guard: 2, root: 1, pen: 0, rec: 0, dext: 0, acc: true),
        focus: (true, true, false),
        "Locks down a lane or doorway (4 setup ticks); triggers a 1-Tick reaction shot when a target crosses the sector"
    ),
];

impl_static_catalog!(CombatTechniqueId => CombatTechniqueSpec : COMBAT_TECHNIQUE_CATALOG);

/// Returns the full 30-entry canonical Combat Technique catalog.
#[must_use]
pub const fn canonical_combat_technique_catalog() -> &'static [CombatTechniqueSpec] {
    CombatTechniqueId::CATALOG
}

/// Returns the static [`CombatTechniqueSpec`] for a given [`CombatTechniqueId`].
#[must_use]
pub const fn get_combat_technique_spec(id: CombatTechniqueId) -> &'static CombatTechniqueSpec {
    id.spec()
}

/// Returns whether a [`CombatTechniqueId`] is legal within the given [`CombatDomain`].
#[must_use]
pub const fn is_technique_allowed_in_domain(tech: CombatTechniqueId, domain: CombatDomain) -> bool {
    let spec = tech.spec();
    match domain {
        CombatDomain::UnarmedMartialArts => spec.allowed_unarmed,
        CombatDomain::ArmedMelee => spec.allowed_armed_melee,
        CombatDomain::Ranged => spec.allowed_ranged,
    }
}

/// Filters the canonical Combat Technique catalog by [`CombatDomain`].
#[must_use]
pub fn combat_techniques_for_domain(domain: CombatDomain) -> Vec<&'static CombatTechniqueSpec> {
    COMBAT_TECHNIQUE_CATALOG
        .iter()
        .filter(|spec| match domain {
            CombatDomain::UnarmedMartialArts => spec.allowed_unarmed,
            CombatDomain::ArmedMelee => spec.allowed_armed_melee,
            CombatDomain::Ranged => spec.allowed_ranged,
        })
        .collect()
}

/// Filters the canonical Combat Technique catalog by [`CombatMoveCategory`].
#[must_use]
pub fn combat_techniques_for_category(
    category: CombatMoveCategory,
) -> Vec<&'static CombatTechniqueSpec> {
    COMBAT_TECHNIQUE_CATALOG
        .iter()
        .filter(|spec| spec.category == category)
        .collect()
}

/// Input parameters for evaluating a specific [`CombatTechniqueId`] combined with a
/// `Tempo + Control + Force` [`Triad`] allocation (`docs/ttrpg/05-combat/melee_combat.md` §3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechniqueTriadEvalParams {
    pub technique: CombatTechniqueId,
    pub domain: CombatDomain,
    pub envelope: TacticalPoolEnvelope,
    pub alloc: Triad<i32>,
    pub skill_level_n: usize,
    pub time_factor_params: ActionTimeFactorParams,
    /// Optional armed melee weapon Base `TC` override (`0` uses technique's `base_total_tc`).
    pub armed_weapon_base_tc: i32,
    /// Optional armed melee weapon `POW` multiplier (`1.0` for unarmed).
    pub armed_weapon_pow_mult: f64,
    /// Optional passive weapon Guard bonus added to `Control` (`0` for unarmed).
    pub weapon_guard_bonus: i32,
    /// Optional weapon block/sweep Force bonus (`+2` for Staff/Spear/Riot Shield, else `0`).
    pub weapon_block_force_bonus: i32,
    /// Optional firearm base Recoil rating $R_{\text{wpn}}$ (`0` for melee).
    pub weapon_recoil_r: i32,
    /// Optional firearm mechanical Accuracy rating $\text{Acc} \in [-3, +4]$ (`0` for melee).
    pub weapon_acc: i32,
    /// Whether the firearm is currently braced (`BraceWeapon` active from current or prior tick).
    pub weapon_is_braced: bool,
}

impl Default for TechniqueTriadEvalParams {
    fn default() -> Self {
        let envelope = crate::combat::compute_tactical_pool(&Default::default());
        let b = envelope.neutral_baseline_b;
        Self {
            technique: CombatTechniqueId::Punch,
            domain: CombatDomain::UnarmedMartialArts,
            envelope,
            alloc: Triad::new(b, b, b),
            skill_level_n: 3,
            time_factor_params: ActionTimeFactorParams::default(),
            armed_weapon_base_tc: 0,
            armed_weapon_pow_mult: 1.0,
            weapon_guard_bonus: 0,
            weapon_block_force_bonus: 0,
            weapon_recoil_r: 0,
            weapon_acc: 0,
            weapon_is_braced: false,
        }
    }
}

/// Complete resolution of a [`CombatTechniqueId`] executed with a specific `Tempo + Control + Force` [`Triad`] allocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechniqueTriadResolution {
    pub technique: CombatTechniqueId,
    pub category: CombatMoveCategory,
    pub domain: CombatDomain,
    pub alloc: Triad<i32>,
    pub valid: bool,
    pub violations: Vec<String>,
    pub action_cost: TwoComponentActionCost,
    pub effective_tc: i32,
    /// Effective centerline `Guard` ($C + \text{guard\_mod} + \text{weapon\_guard\_bonus}$).
    pub effective_guard: i32,
    /// Effective grounded `Root` ($F + \text{MMF} + \text{root\_mod} + \text{root\_stance\_bonus}$).
    pub effective_root: f64,
    /// Scaled melee Strike Impact ($\text{POW} \times \text{pow\_mult}_{\text{tech}} \times \text{pow\_mult}_{\text{wpn}} \times (F / B)$).
    pub strike_impact: f64,
    /// Armor Penetration modifier ($\Delta\text{Pen}$).
    pub effective_pen_mod: i32,
    /// Incoming strike impact absorbed when executing `Block` ($\lfloor F / 2 \rfloor + \text{weapon\_block\_force\_bonus}$).
    pub block_damage_reduction: i32,
    /// Multiplier on recovery ticks inflicted when **Seizing the Initiative** (`2` for `Parry`, else `1`).
    pub parry_counter_multiplier: i32,
    /// Guard (`Control`) stripped from target per net success when executing `Feint` (`2` for `Feint`, else `0`).
    pub feint_guard_strip_per_success: i32,
    /// Evasion $\Delta_{\text{ext}}$ penalty imposed on incoming attacks when executing `Dodge` ($\lfloor T / 2 \rfloor$).
    pub dodge_evasion_delta_ext: i32,
    /// Effective firearm Recoil rating $R_{\text{eff}}$ after mode adder and bracing.
    pub ranged_effective_recoil_r: i32,
    /// Unbraced firearm recoil shortfall $\max(0, R_{\text{eff}} - F)$ added to $\Delta_{\text{ext}}$.
    pub ranged_unbraced_dispersion_delta_ext: i32,
    /// Effective weapon `Acc` modifier applied on this shot (`SnapShot` ignores positive `Acc`; `DeliberateAimedShot` applies full `Acc`).
    pub ranged_effective_acc_bonus: i32,
    /// Total technique-level $\Delta_{\text{ext}}$ modifier (including `delta_ext_mod`, unbraced recoil, and negative/positive `Acc`).
    pub technique_delta_ext: i32,
    pub exertion_marks_gained: i32,
    pub archetype_label: String,
}

/// Evaluates how a [`CombatTechniqueId`] (What You Do) interacts with a `Tempo + Control + Force`
/// [`Triad`] allocation (How You Execute It).
#[must_use]
pub fn evaluate_technique_with_triad(params: &TechniqueTriadEvalParams) -> TechniqueTriadResolution {
    let spec = params.technique.spec();
    let mut base_act = spec.base_tc_act;
    if params.domain == CombatDomain::ArmedMelee && params.armed_weapon_base_tc > 0 {
        // Armed Thrust is 1 tick faster than weapon Base TC; Armed Swing is 1 tick slower.
        let wpn_act = (params.armed_weapon_base_tc - spec.base_tc_decide).max(3);
        base_act = match params.technique {
            CombatTechniqueId::Thrust => (wpn_act - 1).max(3),
            CombatTechniqueId::Swing => wpn_act + 1,
            _ => wpn_act,
        };
    }

    let base_total_tc = spec.base_tc_decide + base_act;
    let triad_eval = evaluate_tactical_triad(&params.envelope, base_total_tc, params.alloc);
    let mut violations = triad_eval.violations.clone();

    if !is_technique_allowed_in_domain(params.technique, params.domain) {
        violations.push(format!(
            "Technique '{}' ({}) is not allowed in combat domain '{}'.",
            spec.name,
            params.technique.as_str(),
            params.domain.label()
        ));
    }

    let apply_cognitive_offset = spec.base_tc_decide >= 3
        || matches!(
            params.technique,
            CombatTechniqueId::Feint
                | CombatTechniqueId::Probe
                | CombatTechniqueId::DeliberateAimedShot
                | CombatTechniqueId::CoverPeekSlicePie
        );

    let action_cost = evaluate_combat_action_cost(
        spec.base_tc_decide,
        base_act,
        params.envelope.d_cog,
        params.skill_level_n,
        apply_cognitive_offset,
        params.alloc.tempo,
        params.envelope.neutral_baseline_b,
        spec.min_floor_ticks,
        &params.time_factor_params,
    );

    let effective_guard =
        (params.alloc.control + spec.guard_mod + params.weapon_guard_bonus).max(0);
    let root_stance_bonus = if params.technique == CombatTechniqueId::Root {
        f64::from(params.alloc.control.max(0).div_euclid(2))
    } else {
        0.0
    };
    let effective_root = (f64::from(params.alloc.force)
        + params.envelope.momentum_mass_factor_mmf
        + f64::from(spec.root_mod)
        + root_stance_bonus)
        .max(0.0);

    let wpn_pow = if params.domain == CombatDomain::ArmedMelee && params.armed_weapon_pow_mult > 0.0
    {
        params.armed_weapon_pow_mult
    } else {
        1.0
    };
    let strike_impact = if spec.pow_multiplier > 0.0 {
        params.envelope.power_pow
            * spec.pow_multiplier
            * wpn_pow
            * (f64::from(params.alloc.force.max(0))
                / f64::from(params.envelope.neutral_baseline_b.max(1)))
    } else {
        0.0
    };

    let block_damage_reduction = if params.technique == CombatTechniqueId::Block {
        params.alloc.force.max(0).div_euclid(2) + params.weapon_block_force_bonus.max(0)
    } else {
        0
    };
    let parry_counter_multiplier = if params.technique == CombatTechniqueId::Parry {
        2
    } else {
        1
    };
    let feint_guard_strip_per_success = if params.technique == CombatTechniqueId::Feint {
        2
    } else {
        0
    };
    let dodge_evasion_delta_ext = if params.technique == CombatTechniqueId::Dodge {
        params.alloc.tempo.max(0).div_euclid(2)
    } else {
        0
    };

    let is_braced =
        params.weapon_is_braced || params.technique == CombatTechniqueId::BraceWeapon;
    let (ranged_effective_recoil_r, ranged_unbraced_dispersion_delta_ext, ranged_effective_acc_bonus) =
        if params.domain == CombatDomain::Ranged {
            let brace_mod = if is_braced && params.technique != CombatTechniqueId::BraceWeapon {
                -3
            } else {
                0
            };
            let r_eff = (params.weapon_recoil_r + spec.recoil_adder + brace_mod).max(0);
            let unbraced = if spec.category == CombatMoveCategory::RangedShot
                || params.technique == CombatTechniqueId::BoundingShootOnMove
            {
                (r_eff - params.alloc.force.max(0)).max(0)
            } else {
                0
            };
            let eff_acc = if spec.uses_full_weapon_acc {
                params.weapon_acc
            } else if matches!(
                params.technique,
                CombatTechniqueId::CombatSightedShot | CombatTechniqueId::ControlledDoubleTapBurst
            ) {
                params.weapon_acc.min(1)
            } else {
                // SnapShot and SuppressiveFullAuto suffer negative Acc (loose tolerances) but ignore positive sight Acc
                params.weapon_acc.min(0)
            };
            (r_eff, unbraced, eff_acc)
        } else {
            (0, 0, 0)
        };

    let brace_aim_bonus = if is_braced && spec.category == CombatMoveCategory::RangedShot {
        -1
    } else {
        0
    };
    let technique_delta_ext = spec.delta_ext_mod
        + ranged_unbraced_dispersion_delta_ext
        - ranged_effective_acc_bonus
        + brace_aim_bonus;

    TechniqueTriadResolution {
        technique: params.technique,
        category: spec.category,
        domain: params.domain,
        alloc: params.alloc,
        valid: violations.is_empty(),
        violations,
        effective_tc: action_cost.effective_total_tc,
        action_cost,
        effective_guard,
        effective_root,
        strike_impact,
        effective_pen_mod: spec.pen_mod,
        block_damage_reduction,
        parry_counter_multiplier,
        feint_guard_strip_per_success,
        dodge_evasion_delta_ext,
        ranged_effective_recoil_r,
        ranged_unbraced_dispersion_delta_ext,
        ranged_effective_acc_bonus,
        technique_delta_ext,
        exertion_marks_gained: triad_eval.exertion_marks_gained,
        archetype_label: triad_eval.archetype_label,
    }
}

/// Situational parameters for [`resolve_technique_exchange`] (`docs/ttrpg/05-combat/melee_combat.md` §1–§3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TechniqueExchangeContext {
    pub attacker_reach: i32,
    pub defender_reach: i32,
    pub in_clinch: bool,
    pub defender_has_keep_out_weapon: bool,
    /// Net cascade successes ($\Delta M = S_{\text{att}} - S_{\text{def}}$) scored by the attacker (`> 0` means hit/success).
    pub attacker_net_successes: i32,
    /// Target's active Armor Damage Reduction / Penetration Resistance (`PR` or `BTD`).
    pub defender_armor_dr: i32,
}

impl Default for TechniqueExchangeContext {
    fn default() -> Self {
        Self {
            attacker_reach: 1,
            defender_reach: 1,
            in_clinch: false,
            defender_has_keep_out_weapon: false,
            attacker_net_successes: 1,
            defender_armor_dr: 0,
        }
    }
}

/// Complete outcome of a contested exchange between an attacker's [`TechniqueTriadResolution`]
/// and a defender's [`TechniqueTriadResolution`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechniqueExchangeOutcome {
    pub attacker_effective_tc: i32,
    pub defender_effective_tc: i32,
    /// Total $\Delta_{\text{ext}}$ imposed on the attacker by the defender's `Guard`, `Dodge`, `Reach`, and `Clinch`.
    pub total_delta_ext_on_attacker: i32,
    pub attack_succeeded: bool,
    /// Effective Strike Impact after subtracting the defender's `Block` damage reduction and `Armor DR`.
    pub net_impact_trauma: f64,
    /// Whether `Takedown`, `Throw`, or `Push` overcame the defender's `Effective Root`.
    pub root_overcome: bool,
    pub target_knocked_prone: bool,
    pub target_disarmed: bool,
    pub target_trapped_or_grabbed: bool,
    /// Limb/Control ceiling slash inflicted by `Lock` or `Trap`.
    pub target_control_cap_slash: i32,
    /// Whether the defender **Seizes the Initiative** on a failed attack (`!attack_succeeded && defender_guard > attacker_guard`).
    pub defender_seizes_initiative: bool,
    /// Extra recovery ticks imposed on the attacker when the defender Seizes the Initiative (doubled on `Parry`!).
    pub attacker_recovery_tick_penalty: i32,
    /// Guard (`Control`) stripped from the defender on the next exchange by `Feint` or `Probe`.
    pub next_exchange_defender_guard_strip: i32,
    pub outcome_summary: String,
}

/// Resolves an opposed combat exchange between an attacker's [`TechniqueTriadResolution`] and
/// a defender's [`TechniqueTriadResolution`] (`docs/ttrpg/05-combat/melee_combat.md` §1–§3).
#[must_use]
pub fn resolve_technique_exchange(
    attacker: &TechniqueTriadResolution,
    defender: &TechniqueTriadResolution,
    ctx: &TechniqueExchangeContext,
) -> TechniqueExchangeOutcome {
    let reach_clinch_delta_ext = compute_reach_and_clinch_delta_ext(
        ctx.attacker_reach,
        ctx.defender_reach,
        ctx.in_clinch,
        attacker.alloc.tempo,
        defender.alloc.tempo,
        ctx.defender_has_keep_out_weapon,
    );

    let guard_delta = (defender.effective_guard - attacker.alloc.control).max(0);
    let total_delta_ext_on_attacker = guard_delta
        + defender.dodge_evasion_delta_ext
        + reach_clinch_delta_ext
        + attacker.technique_delta_ext;

    let net_s = ctx.attacker_net_successes;
    let attack_succeeded = net_s > 0;

    let defender_seizes_initiative =
        !attack_succeeded && defender.effective_guard > attacker.effective_guard;
    let attacker_recovery_tick_penalty = if defender_seizes_initiative {
        (defender.effective_guard - attacker.effective_guard) * defender.parry_counter_multiplier
    } else {
        0
    };

    let leveraged_force = match attacker.technique {
        CombatTechniqueId::Throw => {
            f64::from(attacker.alloc.force)
                + f64::from((attacker.alloc.control - defender.effective_guard).max(0))
                + if ctx.in_clinch {
                    f64::from(defender.alloc.tempo.max(0).div_euclid(2))
                } else {
                    0.0
                }
        }
        CombatTechniqueId::Takedown => {
            f64::from(attacker.alloc.force)
                + f64::from((attacker.alloc.tempo - defender.alloc.tempo).max(0))
                + f64::from((attacker.alloc.control - defender.effective_guard).max(0))
        }
        CombatTechniqueId::Push => {
            attacker.effective_root
                + f64::from((attacker.alloc.control - defender.effective_guard).max(0))
        }
        _ => 0.0,
    };

    let root_overcome = attack_succeeded
        && matches!(
            attacker.technique,
            CombatTechniqueId::Throw | CombatTechniqueId::Takedown | CombatTechniqueId::Push
        )
        && leveraged_force >= defender.effective_root;

    let target_knocked_prone = root_overcome
        && matches!(
            attacker.technique,
            CombatTechniqueId::Throw | CombatTechniqueId::Takedown
        );

    let target_disarmed = attack_succeeded
        && attacker.technique == CombatTechniqueId::Disarm
        && (attacker.alloc.control + attacker.alloc.force.div_euclid(2))
            >= (defender.effective_guard + defender.alloc.force.div_euclid(2));

    let target_trapped_or_grabbed = attack_succeeded
        && matches!(
            attacker.technique,
            CombatTechniqueId::Grab | CombatTechniqueId::Trap | CombatTechniqueId::Lock
        );

    let target_control_cap_slash = if !attack_succeeded {
        0
    } else {
        match attacker.technique {
            CombatTechniqueId::Trap => (attacker.alloc.control - defender.alloc.control).clamp(1, 6),
            CombatTechniqueId::Lock => {
                (attacker.alloc.force.div_euclid(3) + net_s).clamp(2, 10)
            }
            CombatTechniqueId::Grab => 2,
            _ => 0,
        }
    };

    let next_exchange_defender_guard_strip = if !attack_succeeded {
        0
    } else {
        match attacker.technique {
            CombatTechniqueId::Feint => net_s * attacker.feint_guard_strip_per_success,
            CombatTechniqueId::Probe => 2,
            _ => 0,
        }
    };

    let net_impact_trauma = if attack_succeeded && attacker.strike_impact > 0.0 {
        let clinched_knee_bonus =
            if attacker.technique == CombatTechniqueId::Knee && ctx.in_clinch {
                2.0
            } else {
                0.0
            };
        let raw = attacker.strike_impact + clinched_knee_bonus + f64::from(net_s.div_euclid(2));
        let after_block = raw - f64::from(defender.block_damage_reduction);
        let after_armor = if attacker.technique == CombatTechniqueId::Lock {
            after_block
        } else {
            after_block - f64::from((ctx.defender_armor_dr - attacker.effective_pen_mod).max(0))
        };
        after_armor.max(0.0)
    } else {
        0.0
    };

    let outcome_summary = if target_knocked_prone {
        format!(
            "{} overcomes Defender Root ({:.1} >= {:.1}) -> Target Prone (+6 Ticks)",
            attacker.technique.label(),
            leveraged_force,
            defender.effective_root
        )
    } else if target_disarmed {
        format!(
            "{} strips Defender's weapon!",
            attacker.technique.label()
        )
    } else if attacker.technique == CombatTechniqueId::Lock && attack_succeeded {
        format!(
            "Submission Lock applied -> -{target_control_cap_slash} Limb/Control Ceiling & {:.1} Joint Trauma",
            net_impact_trauma
        )
    } else if target_trapped_or_grabbed {
        format!(
            "{} established (-{target_control_cap_slash} target cap)",
            attacker.technique.label()
        )
    } else if next_exchange_defender_guard_strip > 0 {
        format!(
            "{} succeeds -> strips -{next_exchange_defender_guard_strip} Guard on next exchange",
            attacker.technique.label()
        )
    } else if attack_succeeded && net_impact_trauma > 0.0 {
        format!(
            "{} lands for {:.1} net impact trauma (after Block {} & Armor {})",
            attacker.technique.label(),
            net_impact_trauma,
            defender.block_damage_reduction,
            ctx.defender_armor_dr
        )
    } else if defender_seizes_initiative {
        format!(
            "Defender {} Seizes Initiative (+{} recovery ticks on Attacker)",
            defender.technique.label(),
            attacker_recovery_tick_penalty
        )
    } else {
        format!(
            "{} neutralized by Defender {}",
            attacker.technique.label(),
            defender.technique.label()
        )
    };

    TechniqueExchangeOutcome {
        attacker_effective_tc: attacker.effective_tc,
        defender_effective_tc: defender.effective_tc,
        total_delta_ext_on_attacker,
        attack_succeeded,
        net_impact_trauma,
        root_overcome,
        target_knocked_prone,
        target_disarmed,
        target_trapped_or_grabbed,
        target_control_cap_slash,
        defender_seizes_initiative,
        attacker_recovery_tick_penalty,
        next_exchange_defender_guard_strip,
        outcome_summary,
    }
}
