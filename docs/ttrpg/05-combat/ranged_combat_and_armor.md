# Satya TTRPG Ranged Combat, Granular Ballistics, Firearms Engineering & Armor

> **Rust Engine Implementation**: [`crates/satya-engine/src/ranged_combat.rs`](../../../crates/satya-engine/src/ranged_combat.rs) (`RangedFireModeId`, `ArmorProfileId`, `ArmorProfile`, `evaluate_ranged_recoil_and_suppression`, `resolve_armor_impact`, `canonical_armor_catalog`), [`crates/satya-engine/src/barrier_ballistics.rs`](../../../crates/satya-engine/src/barrier_ballistics.rs) (`MATERIAL_BALLISTIC_PROFILES`, `evaluate_ballistic_shot`, `collect_ray_barriers`, `BarrierPenetrationVerdict`), [`crates/satya-engine/src/combat_techniques.rs`](../../../crates/satya-engine/src/combat_techniques.rs) (`CombatDomain::Ranged`, `CombatMoveCategory::RangedShot`, `CombatMoveCategory::RangedManeuver`, `CombatTechniqueId`, `evaluate_technique_with_triad`), [`crates/satya-engine/src/ballistics.rs`](../../../crates/satya-engine/src/ballistics.rs), and [`crates/satya-engine/src/walkthroughs.rs`](../../../crates/satya-engine/src/walkthroughs.rs) (`simulate_mateo_vs_vance_firefight`). Companion chapter to **[Granular Ballistics, Ammunition Physics & Firearm Engineering (`firearms_and_ballistics.md`)](firearms_and_ballistics.md)**.

This chapter defines the canonical rules for **Ranged Firefights, Internal/External/Terminal Ballistics, Granular Firearm Engineering, Ammunition Physics, Weapon Reliability (`Rel`), and Body Armor (`PR` vs. Behind-Armor Blunt Trauma `BABT`)** in **Satya**. It builds directly upon:
- **[Mathematical Notation, Variables & Formula Glossary](../01-foundation/mathematical_notation_glossary.md)** (for all mathematical variables and ballistics/combat symbols $E_k, m_{\text{proj}}, m_{\text{prop}}, L_{\text{bbl}}, \text{OAL}, M_{\text{wpn}}, \Delta_{\text{CoG}}, \tau_{\text{tol}}, \text{Acc}, W_{\text{yaw}}, \text{Pen}, R, \text{PR}_{\text{Edged}}, \text{PR}_{\text{Ballistic}}, \text{BABT}, \text{Rel}$),
- **[Combat Architecture, Tactical Triad & Pools](tactical_triad_and_pools.md)** (for the "Envelope & Fill" Tactical Pool $P_{\text{base}} = C_{\text{combat}} + 3N$),
- **[Psyche, Morale & Social Combat](../04-psyche-and-social/psyche_morale_and_social_combat.md#34-combat-morale-tactical-ooda-behavior--the-five-conditions-of-morale)** (for Suppression Morale Checks and the Five Conditions of Morale), and
- **[Health, Anatomical Trauma, Fatigue & Mental Conditions](../06-health-and-survival/health_and_trauma.md)** (for Hit Locations, Called Shots, Structural Wounds, and Hemorrhaging).

---

## 2. Ranged Combat in the Tactical Triad (`Tempo`, `Control`, `Force`)

Ranged combat uses the exact same **Tempo, Control, and Force** triad on the **[Count-Up Action Clock](../02-core-mechanics/action_economy_and_ooda.md)** (`1 Tick = 0.25s`, `4 Ticks = 1.0s`) as close-quarters combat:

- **Tempo (`T` — Aim Speed, Target Acquisition & CQB Handling)**: Governs how many ticks (`TC`) you spend bringing the weapon to bear—from a fast point-shoot or hip-fire snapshot (high **Tempo**, lower `TC`) to a deliberate multi-tick sighted shot (low **Tempo**, higher **Control**). Modified by the weapon's **Overall Length ($\text{OAL}$)** and **Center of Gravity ($\Delta_{\text{CoG}}$)**.
- **Control (`C` — Sight Alignment, Trigger Squeeze & Cover Exposure)**: Governs marksmanship precision (`DEX/SIG` or `DEX/FOC`), called shots to bypass armor, AND how much of your body is exposed out of cover while firing (`Guard`).
- **Force (`F` — Stance, Recoil Bracing & Volume of Fire)**: Governs how rooted and braced your shooting platform is (`FOR`/`MAS`) to absorb the weapon's physical **Recoil Rating ($R$)** during rapid or automatic fire (*Fully Automatic Weapons* `DEX/FOR`) and project suppressive pressure.

### 2.1 Ranged Shot Actions (`5`) & Ranged Maneuvers / Bracing (`4`)

Unlike melee weapons where **Force** directly multiplies muscle impact, a firearm's projectile energy is propelled by chemical propellant (`grains` of powder), while the shooter's **Technique (`CombatTechniqueId`)** and **Tactical Triad (`Tempo`, `Control`, `Force`)** govern target acquisition speed (`Base TC = TC_decide + TC_act`), sight alignment (`Acc`), cover exposure (`Guard`), and **Recoil Bracing ($F \ge R_{\text{eff}}$)**:

#### A. Ranged `Shot` Actions (`RangedShot`)

| Shot Technique (`CombatTechniqueId`) | Base `TC` (`Decide + Act`) | Mode Recoil Adder ($\Delta R_{\text{mode}}$) | Weapon `Acc` Applied? | Precision / Cover Mod | How the Tactical Triad (`TCF`) Interplays with the Shot |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **`Snap Shot`** *(Hip-Fire / Point-Shoot)* | **`6`** (`1 + 5`) | `+0` *(Base $R_{\text{wpn}}$)* | Negative `Acc` only *(ignores `+Acc`)* | **`+2` $\Delta_{\text{ext}}$**, `Guard +0` | Ultra-fast CQB point-shoot (`TC_decide = 1`). High **Tempo** beats the target to the draw at close range (`<= 15m`), but point-indexing adds `+2` $\Delta_{\text{ext}}$ and forfeits positive sight/optic `Acc`. |
| **`Combat Sighted Shot`** *(Flash Sight Picture)* | **`9`** (`2 + 7`) | `+0` *(Base $R_{\text{wpn}}$)* | Up to **`+1` `Acc`** | **`+0` $\Delta_{\text{ext}}$**, `Guard +1` | Standard combat presentation: brings sights to eye level (`+1 Guard`), balances **Tempo**, **Control**, and **Force** ($F \ge R_{\text{wpn}}$). |
| **`Deliberate Aimed Shot`** *(Precision Sighted)* | **`12`** (`4 + 8`) | `+1` *(Follow-through)* | **Full `Acc`** (`+Acc` + Optic) | **`-2` $\Delta_{\text{ext}}$** *(Aim)*, `Root +1` | Cognitive `TC_decide = 4` (compressed by $D_{\text{cog}}$). Prioritizes **Control** (sight alignment, breathing pause, trigger squeeze, called shots) at lower **Tempo**; unlocks full positive **Weapon Accuracy ($\text{Acc}$)** and `-2` $\Delta_{\text{ext}}$ aim bonus. |
| **`Controlled Double-Tap / Short Burst`** | **`9`** (`2 + 7`) | **`+2` to `+3`** | Up to **`+1` `Acc`** | `2–3 rds` (`+1 hit` / `2` Net Successes) | Requires balanced **Force ($F \ge R_{\text{eff}}$)** to keep the second/third round on target alongside **Control** and **Tempo**. |
| **`Suppressive Full-Auto Zone`** | **`9`** (`2 + 7`) | **`+4` to `+6`** | Negative `Acc` only | `5–15 rds`, `+2` $\Delta_{\text{ext}}$, `Guard -1` | Demands heavy **Force** (`FOR`/`MAS`) to muscle down muzzle climb and project a beaten zone of **Suppression** ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$). |

#### B. Ranged `Maneuver` & Bracing Actions (`RangedManeuver`)

| Ranged Maneuver (`CombatTechniqueId`) | Base `TC` (`Decide + Act`) | Recoil / Accuracy / Cover Mod | How the Tactical Triad (`TCF`) Interplays with the Maneuver |
| :--- | :---: | :---: | :--- |
| **`Brace / Rest Weapon`** *(Bipod / Barricade / Prone)* | **`5`** (`1 + 4`) | **`-3` $R_{\text{eff}}$**, **`-1` Wobble**, **`-1` $\Delta_{\text{ext}}$**, `Guard +2`, `Root +3` | Anchors the firearm on cover, a barricade, or a bipod (`Root` for ranged combat). Slashes Effective Recoil $R_{\text{eff}}$ by **`-3`** on subsequent shots—freeing points from **Force** so the shooter can pour their pool into **Control**! |
| **`Cover Peek / Slice the Pie`** *(Micro-Exposure)* | **`8`** (`3 + 5`) | **`+3` Cover Guard ($C_{\text{def}}$)**, `Root +1` | Uses `D_cog` (`TC_decide = 3`) + **Control** to expose only eyes and muzzle around hard cover, adding **`+3` Guard** against return fire. |
| **`Bounding Fire / Shoot on the Move`** | **`10`** (`2 + 8`) | **`+3` Unbraced $\Delta R_{\text{mode}}$**, `+1` $\Delta_{\text{ext}}$, `Root -2` | High **Tempo** covers ground across open space (`1–3 rds`), but moving demands extra **Force** (`+3` recoil adder) to keep rounds on target. |
| **`Overwatch / Hold Sector`** *(Triggered Release)* | **`4`** (`2 + 2` setup, `1` trigger) | `Guard +2`, `Root +1`, Full `Acc` | Commits **Control** + `FOC/ITU` to lock down a doorway or lane; interrupts any target crossing the sector with a `1-Tick` reaction shot. |

- **Resolving Recoil Bracing ($F$ vs. $R_{\text{eff}}$)**:
  $$\text{Effective Recoil } (R_{\text{eff}}) = \max(0,\; R_{\text{wpn}} + \Delta R_{\text{mode}} - 3 \cdot \mathbb{I}_{\text{braced}})$$
  $$\text{Unbraced Dispersion Penalty } (\Delta_{\text{ext, recoil}}) = \max(0,\; R_{\text{eff}} - F)$$
  If the shooter allocates $F \ge R_{\text{eff}}$, their stance (and any active `Brace / Rest Weapon` support) absorbs the recoil impulse cleanly (`0` penalty). Every point by which allocated **Force ($F$)** falls short of $R_{\text{eff}}$ adds **`+1` to $\Delta_{\text{ext}}$** due to muzzle climb!

---

> **Ballistics & Firearm Engineering Reference**: See **[Granular Ballistics, Ammunition Physics & Firearm Engineering (`firearms_and_ballistics.md`)](firearms_and_ballistics.md)** for **Sections 1 & 3–5** (Kinetic Energy $E_k$, Master Ammunition Reference Table, Feed Mechanisms, Terminal Ballistics $I_{\text{KE}} + I_{\text{cal}} + W_{\text{yaw}}$, Action Types, Weight, Center of Gravity $\Delta_{\text{CoG}}$, Overall Length $\text{OAL}$, and Barrel Specs).

---

## 6. Reliability ($\text{Rel}$), Tolerances ($\tau_{\text{tol}}$), Jam Clearance & Strip Time

### 6.1 The Accuracy–Reliability Tolerance Trade-Off ($\tau_{\text{tol}}$)

One of the fundamental laws of mechanical engineering in firearms is the trade-off between **Machining Clearance Tolerance ($\tau_{\text{tol}}$)**, **Accuracy ($\text{Acc}$)**, and **Reliability ($\text{Rel}$)**:

$$\text{Rel} = \text{clamp}\!\Big(-4,\; +4,\; \text{Rel}_{\text{action}} + \text{Rel}_{\text{feed}} - \text{round}(\tau_{\text{tol}}) + Q_{\text{mfg}} + \Delta\text{Rel}_{\text{ammo}}\Big)$$

| Tolerance Spec ($\tau_{\text{tol}}$) | Effect on **Accuracy ($\text{Acc}$)** | Effect on **Reliability ($\text{Rel}$)** | Effect on **Wobble ($W_{\text{yaw}}$)** | Engineering Reality |
| :--- | :---: | :---: | :---: | :--- |
| **Loose / Combat Clearance ($\tau_{\text{tol}} = -1.0$)** | **`-1.0` Acc** *(3–5 MOA)* | **`+1` to `+2` Rel** | **`+1` Wobble** | Generous clearance between rails, bolt lugs, and chamber walls (e.g., Kalashnikov pattern). Parts rattle when shaken and bullets yaw slightly at launch, **but** mud, sand, carbon, and frozen condensation have room to be pushed aside without binding the action! |
| **Standard Service Tolerance ($\tau_{\text{tol}} = 0.0$)** | **`+0.0` Acc** *(1.5–2.5 MOA)* | **`+0` Rel** | `+0` | Balanced military/law-enforcement specification (e.g., Glock 17, M4A1, Beretta M9). |
| **Tight / Tactical Match ($\tau_{\text{tol}} = +1.0$)** | **`+1.0` Acc** *(0.75–1.0 MOA)* | **`-1` Rel** | `0` | Tightly fitted slide-to-frame rails and minimum-headspace match chamber (`SAAMI Match` / `.223 Wylde`). Locks up in the exact same micron position every shot, **but** fails to go into battery if fouled with carbon or fed slightly out-of-spec ammo! |
| **Hand-Lapped Zero-Slop ($\tau_{\text{tol}} = +1.5$)** | **`+1.5` Acc** *(< 0.5 MOA)* | **`-2` Rel** | `0` | Custom competition / benchrest fit. Glass-smooth when immaculate and oiled, but a single grain of desert sand or unburned powder flake locks the slide or bolt solid! |

#### Manufacturing Quality ($Q_{\text{mfg}}$)
While tight tolerance reduces reliability, superior **metallurgy, surface treatment, and quality control ($Q_{\text{mfg}}$)** can preserve reliability—at a steep financial cost:
- **`Q_mfg = -2` (Improvised / 3D-Printed Street Polymer / Pipe Scrap)**: Soft unhardened steel or FDM polymer frames that warp under heat; **`-2` $\text{Rel}$**, `0.2×` base cost.
- **`Q_mfg = -1` (Budget Clone / Worn Third-World Surplus)**: Cast zinc/pot-metal small parts, pitted chambers, weakened extractor springs; **`-1` $\text{Rel}$**, `0.5×` base cost.
- **`Q_mfg = 0` (Standard Commercial / Mil-Spec Issue)**: Forged 7075-T6 aluminum / 4140 ordnance steel, Parkerized or nitride finish; **`+0` $\text{Rel}$**, `1.0×` base cost.
- **`Q_mfg = +1` (Duty-Grade Tier-1)**: Cold-hammer forged, magnetic-particle inspected (`MPI`) bolt, chrome/phosphate coated carrier, enhanced extractor springs; **`+1` $\text{Rel}$**, `1.8×` base cost.
- **`Q_mfg = +2` (Masterwork / Aerospace DLC & NP3 Coated)**: Precision wire-EDM machining coated in self-lubricating Diamond-Like Carbon (`DLC`) or Nickel-PTFE (`NP3`) with dry-film micro-lubricity; **`+2` $\text{Rel}$** (allowing a gunsmith to build a **`+1.0` Tight-Tolerance Match weapon** that *still* has net positive reliability!), `3.5×–5.0×` base cost.

---

### 6.2 When Malfunctions Trigger, Clear Jam Time ($\text{TC}_{\text{clear}}$) & Strip Time ($\text{TC}_{\text{strip}}$)

Satya integrates weapon malfunctions directly into the **[`2d20` Cascade Outcome Tiers](../02-core-mechanics/cascading_d20_and_tasks.md#2-signed-outcome-spectrum--n-to-n-botch-rules--outcometier)** without requiring a separate roll on every shot:

1. **Minor Malfunction (`Stovepipe / Failure-to-Feed / Misfire`)**:
   - **Trigger**: On **`SuccessWithComplication`** (`total_successes >= 1` AND `raw_botches >= 1`), if $\text{raw\_botches} > \text{Rel}$, the current shot resolves normally, **but** the weapon fails to cycle the *next* round!
   - **Clearance (`Immediate Action` — $\text{TC}_{\text{clear, minor}}$)**:
     - **Double-Action Revolver**: **`2 Ticks (0.5s)`** (pull trigger again to rotate past the dead primer!).
     - **Pump-Action / Bolt-Action**: **`4–6 Ticks (1.0–1.5s)`** (rack slide/bolt to eject spent shell).
     - **Box-Fed Pistol / Standard Carbine (DI/Piston/Striker)**: **`8 Ticks (2.0s)`** (*"Tap, Rack, Bang"* — slap magazine baseplate, rack charging handle/slide to eject stovepipe).
     - **Bullpup Rifle**: **`10 Ticks (2.5s)`** (must break cheek weld to inspect/clear rear ejection port).
     - **Drum-Fed / Belt-Fed**: **`12 Ticks (3.0s)`**.
2. **Major Malfunction (`Double-Feed / Bolt-Override / Stuck Casing / Link Jam`)**:
   - **Trigger**: On **`BotchFailure`** (`total_successes == 0` AND `effective_botches >= 2`), if $\text{effective\_botches} + \text{Fouling} > 2 + \text{Rel}$, the shot fails to fire and two rounds wedge against the feed ramp (or a spent case rips its rim off inside the chamber).
   - **Clearance (`Remedial Action` — $\text{TC}_{\text{clear, major}}$)**:
     Requires a `DEX/ANA` (*Gunsmithing* or weapon skill) check while spending **$\text{TC}_{\text{clear, major}}$** ticks (`20–24 Ticks` `[5–6s]` for box-fed pistol/carbine; `28–32 Ticks` `[7–8s]` for bullpup/drum; `36 Ticks` `[9s]` for tubular shell-lifter jam or belt-fed link jam) to lock the bolt back, rip out the magazine/belt, cycle the action clear, and re-insert a fresh feed.
3. **Field-Strip Time ($\text{TC}_{\text{strip}}$)**:
   - Represents the time required to disassemble the weapon into its primary field-strip groups (receiver/frame, barrel, slide/bolt-carrier, recoil assembly, feed unit) for **Preventative Maintenance** (`30 min` cleaning grants **`+1` $\text{Rel}$** for the next combat scene), clearing a catastrophic squib/mud obstruction in the field, or breaking a rifle down to fit inside a covert courier bag:
     - **Quick-Takedown Lever / Captive Pin** *(Glock, SIG P320, AKM, Steyr AUG)*: **`28–40 Ticks` (`7–10 seconds`)**.
     - **Push-Pin Hinged Receiver** *(AR-15/M4, HK416, SCAR, MP5)*: **`48–72 Ticks` (`12–18 seconds`)**.
     - **Barrel Bushing / Interlocking Link** *(1911 Government, Remington 870, Revolvers)*: **`120–160 Ticks` (`30–40 seconds`)**.
     - **Precision Chassis / Multi-Fastener / Belt-Fed GPMG** *(M240B, Mk14 EBR, Accuracy International)*: **`240–480 Ticks` (`60–120 seconds`)**.

---

## 7. Master Granular Firearms Catalog (`2032–2035` Benchmark Weapons)

Every weapon below is derived directly from the engineering equations in **Sections 3–6**:

| Weapon Model & Archetype | Ammunition (`Caliber` · `Mass` · `Powder`) | Feed System & Capacity | Action Type | Weight (`Unloaded / Loaded`) | Length (`OAL / Barrel` & Mount) | Center of Gravity ($\Delta_{\text{CoG}}$) | Tolerance ($\tau_{\text{tol}}$) & Mfg ($Q_{\text{mfg}}$) | Accuracy ($\text{Acc}$ / `MOA`) | Reliability ($\text{Rel}$) | Clear Jam (`Minor / Major`) | Strip Time ($\text{TC}_{\text{strip}}$) | Recoil ($R_{\text{wpn}}$) & Base `TC` | Damage (`Raw Impact` / `Base Pen` / `Wobble`) | Cost & `[Access]` |
| :--- | :--- | :--- | :--- | :---: | :---: | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **1. Glock 19 Gen6 (`9mm` Service Pistol)** | **`9×19mm`**<br/>`124 gr` · `5.2 gr` | **Box Mag**<br/>`15 + 1 rds` | **Striker-Fired**<br/>($\eta = 0.99$) | `0.67 kg`<br/>*(0.86 kg)* | `18.5 cm` OAL<br/>**`10.2 cm` (`4.0"`)**<br/>*Tilting Non-FF* | **Neutral**<br/>*(Low Bore)* | **Service** (`0.0`)<br/>**Duty** (`+1`) | **`0`**<br/>*(3.0 MOA)* | **`+2`** | **`8 Ticks`** *(2.0s)*<br/>**`20 Ticks`** *(5.0s)* | **`28 Ticks`**<br/>*(7.0s)* | **`R = 3`**<br/>Snap `TC 7`<br/>Aim `TC 11` | **Impact `6`** *(FMJ)* / **`7`** *(JHP)*<br/>**Pen `3`** *(FMJ)* / **`1`** *(JHP)*<br/>*(Wobble $W=1$)* | **`$650`**<br/>`[Class I: Licensed]` |
| **2. Sear-Modified "Switch" Machine Pistol** | **`9×19mm`**<br/>`115 gr` · `5.0 gr` *(Bulk)* | **50-rd Drum**<br/>`50 + 1 rds` | **Striker / Conversion Auto** ($\eta = 0.98$) | `0.70 kg`<br/>*(1.55 kg)* | `18.5 cm` OAL<br/>**`10.2 cm` (`4.0"`)**<br/>*Non-FF* | **Bottom-Heavy**<br/>*(Drum Pendulum)* | **Loose** (`-1.0`)<br/>**Street Mod** (`-1`) | **`-2`**<br/>*(6.0 MOA)* | **`-2`** | **`12 Ticks`** *(3.0s)*<br/>**`28 Ticks`** *(7.0s)* | **`36 Ticks`**<br/>*(9.0s)* | **`R = 4`** *(Full-Auto `R = 8`)*<br/>Snap `TC 7`<br/>Burst `TC 8` | **Impact `7`** *(Erratic Tumbling)*<br/>**Pen `2`**<br/>*(Wobble $W=3$)* | **`$900`**<br/>`[Class II: Illicit]` |
| **3. Custom STI/Staccato 2011 Match (`.45 ACP`)** | **`.45 ACP`**<br/>`230 gr` JHP · `5.6 gr` | **Box Mag**<br/>`12 + 1 rds` | **Single-Action (`SA`)**<br/>($\eta = 0.99$) | `1.12 kg`<br/>*(1.38 kg)* | `22.0 cm` OAL<br/>**`12.7 cm` (`5.0"`)**<br/>*Match Bushing* | **Neutral**<br/>*(Steel Frame)* | **Tight Match** (`+1.0`)<br/>**Masterwork DLC** (`+2`) | **`+2`**<br/>*(1.2 MOA)* | **`+2`** *(DLC offsets tight fit)* | **`8 Ticks`** *(2.0s)*<br/>**`20 Ticks`** *(5.0s)* | **`120 Ticks`**<br/>*(30.0s)* | **`R = 4`**<br/>Snap `TC 7`<br/>Aim `TC 11` | **Impact `7`** *(Wide `.45` JHP Crush)*<br/>**Pen `1`** *(FMJ `Pen 2`)*<br/>*(Wobble $W=0$)* | **`$3,800`**<br/>`[Class I: Licensed]` |
| **4. S&W 686 Plus (`.357 Magnum` Revolver)** | **`.357 Magnum`**<br/>`158 gr` · `14.5 gr` | **Cylinder**<br/>`7 rds` | **DA / SA Revolver**<br/>($\eta = 0.92$ *Gap*) | `1.25 kg`<br/>*(1.37 kg)* | `29.2 cm` OAL<br/>**`15.2 cm` (`6.0"`)**<br/>*Fixed Pinned* | **Moderate Forward** *(Full Underlug)* | **Service** (`0.0`)<br/>**Duty** (`+1`) | **`+1`** *(SA)*<br/>**`0`** *(DA)*<br/>*(1.8 MOA)* | **`+3`** | **`2 Ticks`** *(0.5s DA pull)*<br/>**`28 Ticks`** *(7.0s Star)* | **`160 Ticks`**<br/>*(40.0s)* | **`R = 5`**<br/>Snap `TC 8`<br/>Aim `TC 12` | **Impact `6`** *(FMJ)* / **`7`** *(JHP)*<br/>**Pen `4`** *(FMJ)* / **`2`** *(JHP)*<br/>*(Wobble $W=0$)* | **`$1,050`**<br/>`[Class I: Licensed]` |
| **5. FN P90 / 2033 PDW (`5.7×28mm`)** | **`5.7×28mm`**<br/>`40 gr` AP · `7.2 gr` | **Horizontal Top Box**<br/>`50 + 1 rds` *(Turntable)* | **Delayed Blowback**<br/>($\eta = 0.95$) | `2.60 kg`<br/>*(3.00 kg)* | **`50.5 cm` OAL**<br/>**`26.4 cm` (`10.4"`)**<br/>*Fixed Shroud* | **Rear / Neutral**<br/>*(Compact Bullpup)* | **Service** (`0.0`)<br/>**Duty** (`+1`) | **`+1`**<br/>*(2.0 MOA)* | **`+1`** *(Turntable feed `0`)* | **`10 Ticks`** *(2.5s)*<br/>**`26 Ticks`** *(6.5s)* | **`40 Ticks`**<br/>*(10.0s)* | **`R = 2`** *(Full-Auto `R = 5`)*<br/>Snap `TC 6`<br/>Burst `TC 8` | **Impact `4`** *(Narrow Needle)*<br/>**Pen `6`** *(Punches Soft Kevlar!)*<br/>*(Wobble $W=0$)* | **`$2,400`**<br/>`[Class III: Restricted]` |
| **6. M4A1 SOPMOD Block III (`5.56mm` DI Carbine)** | **`5.56×45mm`**<br/>`62 gr` · `27.0 gr` | **Box Mag**<br/>`30 + 1 rds` | **Direct Impingement (`DI`)**<br/>($\eta = 0.97$) | `3.05 kg`<br/>*(3.50 kg)* | `80.0 cm` OAL<br/>**`36.8 cm` (`14.5"`)**<br/>**Free-Floating** | **Neutral**<br/>*(In-Line Buffer)* | **Service** (`0.0`)<br/>**Duty CHF** (`+1`) | **`+2`**<br/>*(1.2 MOA)* | **`+1`** *(`0` if fouled/dry)* | **`8 Ticks`** *(2.0s)*<br/>**`22 Ticks`** *(5.5s)* | **`56 Ticks`**<br/>*(14.0s)* | **`R = 3`** *(Full-Auto `R = 7`)*<br/>Snap `TC 7`<br/>Aim `TC 11` | **Impact `7`** *(Spitzer Yaw `+1`)*<br/>**Pen `6`** *(M855A1 `Pen 7`)*<br/>*(Wobble $W=1$)* | **`$1,850`**<br/>`[Class III: Restricted]` |
| **7. HK416A5 / SIG MCX (`5.56mm` CQB Suppressed)** | **`5.56×45mm`**<br/>`62 gr` · `27.0 gr` | **Box Mag**<br/>`30 + 1 rds` | **Short-Stroke Gas Piston**<br/>($\eta = 0.97$) | `3.55 kg`<br/>*(4.45 kg w/ Suppressor)* | `86.0 cm` *(w/ Sup)*<br/>**`28.0 cm` (`11.0"`)**<br/>**Free-Floating** | **Moderate Forward** *(Piston + Suppressor)* | **Tight Duty** (`+0.5`)<br/>**Tier-1** (`+1`) | **`+1`**<br/>*(1.5 MOA)* | **`+2`** *(Runs clean suppressed)* | **`8 Ticks`** *(2.0s)*<br/>**`22 Ticks`** *(5.5s)* | **`60 Ticks`**<br/>*(15.0s)* | **`R = 2`** *(Suppressed `-1 R`)*<br/>Snap `TC 8`<br/>Aim `TC 11` | **Impact `6`** *(11" Barrel `-15%` KE)*<br/>**Pen `5`** *(M855A1 `Pen 6`)*<br/>*(Wobble $W=1$)* | **`$3,400`**<br/>`[Class III: Restricted]` |
| **8. Steyr AUG A3 / Tavor X95 (`5.56mm` Bullpup)** | **`5.56×45mm`**<br/>`62 gr` · `27.0 gr` | **Box Mag**<br/>`30 + 1 rds` | **Short-Stroke Gas Piston**<br/>($\eta = 0.97$) | `3.60 kg`<br/>*(4.05 kg)* | **`69.0 cm` OAL**<br/>**`45.7 cm` (`18.0"`)**<br/>*Quick-Change* | **Rear-Biased**<br/>*(Bullpup)* | **Service** (`0.0`)<br/>**Duty CHF** (`+1`) | **`+1`**<br/>*(1.8 MOA)* | **`+2`** | **`10 Ticks`** *(2.5s)*<br/>**`28 Ticks`** *(7.0s)* | **`36 Ticks`**<br/>*(9.0s Quick-Bbl)* | **`R = 3`** *(Full-Auto `R = 7`)*<br/>**Snap `TC 6`**<br/>Aim `TC 11` | **Impact `7`** *(Full 18" Velocity!)*<br/>**Pen `6`**<br/>*(Wobble $W=1$)* | **`$2,300`**<br/>`[Class III: Restricted]` |
| **9. Kalashnikov AKM / AK-103 (`7.62×39mm`)** | **`7.62×39mm`**<br/>`123 gr` · `25.5 gr` | **Box Mag**<br/>`30 + 1 rds` *(Rock-and-Lock)* | **Long-Stroke Gas Piston**<br/>($\eta = 0.97$) | `3.30 kg`<br/>*(4.10 kg)* | `88.0 cm` OAL<br/>**`41.5 cm` (`16.3"`)**<br/>*Clamped Non-FF* | **Moderate Forward** | **Loose Combat** (`-1.0`)<br/>**Rugged Mil** (`+1`) | **`-1`**<br/>*(3.8 MOA)* | **`+4`** *(Legendary mud/sand tolerance)* | **`8 Ticks`** *(2.0s)*<br/>**`20 Ticks`** *(5.0s)* | **`32 Ticks`**<br/>*(8.0s)* | **`R = 5`** *(Full-Auto `R = 9`)*<br/>Snap `TC 8`<br/>Aim `TC 12` | **Impact `9`** *(Heavy `7.62` + Wobble `W=2`!)*<br/>**Pen `5`** *(Sideways yaw `-1 Pen`)*<br/>*(Wobble $W=2$)* | **`$950`**<br/>`[Class II/III: Illicit/Mil]` |
| **10. Remington 870 / Benelli M4 (`12-Gauge` Shotgun)** | **`12-Gauge`**<br/>`437 gr` · `32.0 gr` *(00 Buck / Slug)* | **Tubular Mag**<br/>`7 + 1 rds` | **Pump-Action**<br/>($\eta = 0.99$) | `3.40 kg`<br/>*(3.85 kg)* | `97.0 cm` OAL<br/>**`47.0 cm` (`18.5"`)**<br/>*Tube-Clamped* | **Forward** *(Full)* $\to$ **Neutral** *(Empty)* | **Loose/Service** (`-0.5`)<br/>**Duty** (`+1`) | **`0`**<br/>*(Spread / 3.5 MOA Slug)* | **`+3`** | **`5 Ticks`** *(1.25s Rack)*<br/>**`36 Ticks`** *(9.0s Lifter)* | **`120 Ticks`**<br/>*(30.0s)* | **`R = 6`**<br/>Snap `TC 8` *(+2 Pump)*<br/>Aim `TC 11` | **Impact `11`** *(Close Buck)* / **`10`** *(Slug)*<br/>**Pen `3`** *(Buck)* / **`5`** *(Slug)*<br/>*(Wobble $W=1$)* | **`$750`**<br/>`[Class I: Licensed]` |
| **11. Accuracy Int'l AT-X / Tikka TAC A1 (`6.5mm Creedmoor`)** | **`6.5mm Creedmoor`**<br/>`140 gr` Match OTM · `42.5 gr` | **Box Mag**<br/>`10 + 1 rds` | **Bolt-Action**<br/>(**$\eta = 1.00$** *Zero Gas Loss*) | `5.40 kg`<br/>*(5.80 kg)* | `112.0 cm` OAL<br/>**`61.0 cm` (`24.0"`)**<br/>**Free-Floating** + Muzzle Brake | **Neutral / Braced** *(Heavy Chassis)* | **Tight Match** (`+1.0`)<br/>**Masterwork** (`+2`) | **`+4`**<br/>*(0.35 MOA)* | **`+3`** *(Bolt cam leverage + DLC)* | **`6 Ticks`** *(1.5s Bolt)*<br/>**`20 Ticks`** *(5.0s)* | **`80 Ticks`**<br/>*(20.0s KeySlot)* | **`R = 4`** *(Muzzle Brake `-2 R`)*<br/>Snap `TC 10` *(+4 Bolt)*<br/>Aim `TC 12` | **Impact `8`** *(Retains `8` past `500m`!)*<br/>**Pen `8`** *(AP `Pen 10`)*<br/>*(Wobble $W=0$)* | **`$5,200`**<br/>`[Class I/III: Licensed/Mil]` |
| **12. FN MAG / M240L (`7.62×51mm` Belt-Fed GPMG)** | **`7.62×51mm NATO`**<br/>`147 gr` · `45.0 gr` | **Belt-Fed**<br/>`100-rd` Assault Pouch / `200-rd` Box | **Long-Stroke Gas Piston** *(Open-Bolt)* ($\eta = 0.97$) | `9.90 kg`<br/>*(12.8 kg w/ 100-rd belt)* | `113.0 cm` OAL<br/>**`55.0 cm` (`21.7"`)**<br/>*Quick-Change Heavy* | **Forward-Biased** *(Bipod Braced)* | **Service** (`-0.5`)<br/>**Duty CHF** (`+1`) | **`0`** *(Bipod)*<br/>**`-2`** *(Hip/Off-hand)* | **`+2`** *(Braced)*<br/>**`+1`** *(Moving)* | **`12 Ticks`** *(3.0s)*<br/>**`36 Ticks`** *(9.0s Feed Tray)* | **`180 Ticks`**<br/>*(45.0s; Bbl swap `20 Ticks`)* | **`R = 5`** *(Bipod Burst `R = 8`; Off-hand `R = 12`)*<br/>Burst `TC 9` | **Impact `9`**<br/>**Pen `7`** *(AP `Pen 9`)*<br/>*(Wobble $W=1$)* | **`$9,500`**<br/>`[Class III/IV: Mil]` |

---

## 8. Granular Armor Engineering, Penetration (`PR`) vs. `BABT` & Degradation

In traditional RPGs, armor either makes a character harder to hit ("Armor Class") or subtracts a flat number from a generic Hit Point pool. In reality, armor adds physical **Mass (`kg`)**, **Thermal Insulation (`Heat Retention`)**, and **Encumbrance**, while acting as a **material barrier** over specific anatomical hit locations that stops penetration up to its structural limit (`PR`)—yet still transfers kinetic momentum (**Behind-Armor Blunt Trauma, `BABT`**) into the wearer's ribs, sternum, or skull!

### 8.1 Armor Engineering Properties & Material Physics

Every piece of protective gear is defined by **six granular attributes**:
1. **Anatomical Coverage (`1d20` Hit Locations)**: Which exact sub-zones from the **[Master Hit Location Table](../06-health-and-survival/health_and_trauma.md#12-master-anatomical-hit-location--specificity-table)** the armor physically covers. Uncovered zones (or gaps targeted via `Control` Called Shots) receive **`0` Protection**!
2. **Material Architecture & Multi-Hit Degradation**:
   - **Soft Ballistic Weave (`Para-Aramid / Kevlar / Twaron / Dyneema`)**: Catch-net of high-tensile fibers. Stops wide, low-sectional-density pistol bullets (`PR_Ballistic = 4`, `PR_Edged = 2`), **but** narrow high-velocity rifle spitzers (`5.56mm`, `5.7×28mm`, `6.5mm Creedmoor`) or icepick thrusts (`PR_Icepick = 1`) part the fibers and punch straight through! Flexible (`BTD = 2`, transfers high blunt deformation into the ribs).
   - **Rigid UHMWPE (Ultra-High-Molecular-Weight Polyethylene)**: Compressed unidirectional polyethylene laminate. Very lightweight (floats on water!), absorbs multi-hit `5.56mm` / `7.62×39mm` FMJ (`PR_Ballistic = 6`, `BTD = 4`), but defeated by tungsten/hardened-steel Armor-Piercing (`AP`) cores.
   - **Monolithic / Mosaic Ceramic Strike Face (`Boron Carbide / Silicon Carbide` + UHMWPE Backer — NIJ Level IV / RF3)**: Ultra-hard ceramic shatters and blunts incoming high-velocity rifle/AP projectiles (`PR_Ballistic = 8`, `BTD = 5`), **but fractures locally on impact** (`-1` to `-2` `PR` degradation per rifle hit on that plate!).
   - **2035 Non-Newtonian Shear-Thickening Fluid (`STF`) + Nanotube Myomer Carapace**: Flexible under normal movement, instantaneously locking into a rigid lattice upon supersonic impact (`PR_Ballistic = 9`, `BTD = 8`, powered exoskeleton offsets its own weight).
3. **Penetration Resistance (`PR_Edged` & `PR_Ballistic`)**: Structural threshold required to perforate the armor.
4. **Blunt Trauma Damping (`BTD`)**: How much kinetic energy the rigid trauma plate and closed-cell foam pad dissipate before the remainder crushes the wearer's underlying bones and organs (`BABT`).
5. **Mass (`kg`), Ergonomic Load Multiplier & Concealability**:
   - Well-fitted body armor distributes its mass across the torso/hips, counting at **`×0.8` of its physical weight** toward the wearer's **[Encumbrance Tier](../06-health-and-survival/movement_vehicles_and_environment.md#2-encumbrance--load-bearing-architecture)** (or subtracted by a powered exoskeleton's load offset).
6. **Don / Doff Time & Cost (`$` & `[Access]`)**: How quickly the armor can be put on or emergency-jettisoned (e.g., pulling a plate carrier's quick-release cable in deep water takes `8 Ticks` `[2.0s]`).

| Armor / Protective Suit | Covered Hit Locations (`1d20`) | Material Composition | `PR_Edged` *(Icepick)* | `PR_Ballistic` | Blunt Damping (`BTD`) | Multi-Hit Plate Durability | Mass (`kg`) & Concealability | Cost & `[Access]` |
| :--- | :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **1. Heavy Leather / Synth-Weave Street Jacket** | Chest `[11–13]`, Abdomen `[8–10]`, Shoulders/Arms `[15–17]` | Abrasion-resistant aramid-lined synth-leather | **`2`** *(`2`)* | **`1`** | **`1`** | Non-degrading (`-1` on `Pen >= 5`) | `2.5 kg` · Everyday streetwear (`+0` Conceal) | **`$350`**<br/>`[Universal]` |
| **2. Concealable Executive Soft Vest (NIJ IIIA)** | Chest `[11–14]`, Abdomen `[8–10]` | Multi-layer Para-Aramid + Shear-Thickening trauma liner | **`2`** *(`1` Icepick)* | **`4`** *(Stops `.45` / `9mm` / `.357`)* | **`2`** | Degrades `-1 PR` only when penetrated | `2.2 kg` · Concealable under suit/jacket (`-2` SIG to spot) | **`$950`**<br/>`[Class I: Licensed]` |
| **3. Lightweight UHMWPE Patrol Carrier (NIJ III / RF1)** | Vital Chest `[11–14]` & Upper Abdomen `[8–9]` | Pressed Polyethylene rigid swimmer plates | **`5`** *(`5`)* | **`6`** *(Stops `5.56` / `7.62×39` Ball)* | **`4`** | Multi-hit resilient (`-1 PR` only on `AP` or `Pen >= 7`) | `4.8 kg` · Overt carrier (or `+4` under winter coat) | **`$1,600`**<br/>`[Class I: Licensed]` |
| **4. Tactical Plate Carrier (Level IV / RF3 Ceramic)** | Vital Chest `[11–14]`, Upper Abdomen `[8–9]` *(+Deltoid Collars `[15]` optional)* | Boron-Carbide ceramic tile matrix + Aramid backer | **`6`** *(`6`)* | **`8`** *(Stops `6.5CM` / `7.62×51` / `M855A1`)* | **`5`** | Ceramic cracks: `-1 PR` per rifle hit (`Rifle Raw >= 6`) even when stopped! | `8.5 kg` · Overt tactical (`8 Ticks` quick-jettison pull) | **`$2,800`**<br/>`[Class II/III: Restricted]` |
| **5. Ballistic Combat Helmet + Mandible/Visor** | Head / Cranium `[20]` *(Neck `[19]` w/ Gorget)* | UHMWPE shell + Polycarbonate ballistic visor | **`5`** *(`5`)* | **`4`** *(Visor `3`)* | **`4`** | Stops pistol/shrapnel & glancing rifle (`-1 PR` on hit) | `1.6 kg` · Overt helmet (mounts NVGs / Comms ear-pro) | **`$1,200`**<br/>`[Class I/III]` |
| **6. Full Riot / Heavy Breacher Suit** | Full Body `[1–20]` *(Joint gaps `+6` Called Shot)* | Ceramic torso plates + polycarbonate/kevlar limb shells | **`6`** *(`5`)* | **`5`** *(Torso `8`)* | **`6`** | Limb shells resilient; torso ceramic degrades `-1 PR` on rifle hit | `16.0 kg` · Overt (`+6` Heat Retention penalty, `-2` $T_{\max}$) | **`$6,500`**<br/>`[Class III: Restricted]` |
| **7. 2035 "Valkyrie" Recon Exo-Carapace** | Full Body Sealed `[1–20]` *(Optic slit `+8` Called Shot)* | Boron-Nitride Nanotube + STF + Powered Myomer Frame | **`8`** *(`8`)* | **`9`** | **`8`** | Self-sealing STF matrix (`-1 PR` only when overmatched by `.338`/`.50`) | `18.5 kg` *(`-20.0 kg` Exo Offset while battery active!)* | **`$65,000`**<br/>`[Class IV: Black-Ops]` |

---

### 8.2 Resolving Armor vs. Incoming Hits (`Penetration` vs. `BABT`)

When an attack hits an armored Hit Location with $\text{Net Successes } \Delta M \ge 1$:
1. **Compute Effective Penetration and Effective Raw Impact**:
   $$\text{Effective Pen} = \text{Base Pen} + \left\lfloor \frac{\Delta M}{2} \right\rfloor, \qquad \text{Effective Raw Impact} = \text{Raw Impact} + \left\lfloor \frac{\Delta M}{2} \right\rfloor$$
2. **Compare $\text{Effective Pen}$ against the Struck Zone's Active $\text{PR}$**:
   - **Case A — Armor Stops Penetration ($\text{Effective Pen} \le \text{PR}$)**:
     The blade or bullet does **not** perforate the skin (**Zero Bleed, `causes_bleed = false`**). However, the kinetic momentum deforms the backface of the armor, inflicting **Behind-Armor Blunt Trauma (`BABT`)**:
     $$\text{Blunt Trauma (BABT)} = \max(0,\; \text{Effective Raw Impact} - \text{BTD})$$
     - *Example*: Taking a **`7.62×39mm` AKM hit** (`Raw Impact = 9`, `Base Pen = 5`) on the chest of a **Level IV Ceramic Plate Carrier (`PR = 8, BTD = 5`)** with $\Delta M = 2$ (`Effective Pen = 6 <= 8`, `Effective Raw Impact = 10`):
       - The ceramic plate **stops the bullet** (`Zero Bleed` — saving the wearer from a fatal pierced heart/lung!).
       - However, the wearer suffers $\text{BABT} = 10 - 5 = \mathbf{5}$ (**Tier 2 Blunt Chest Trauma**: cracked rib, `-5` to `Force Cap`, `+1` `wound_shock_index`, and the ceramic plate cracks by `-1 PR` down to `PR 7`).
   - **Case B — Armor Overmatched ($\text{Effective Pen} > \text{PR}$)**:
     The projectile punches through the armor with reduced energy:
     $$\text{Penetrating Trauma} = \max(0,\; \text{Effective Raw Impact} - \text{PR})$$
     - This inflicts a **penetrating Structural Wound & Hemorrhage (`Grades 1–4`)** and degrades the struck armor zone's `PR` by **`-1`** (or **`-2`** if $\text{Effective Pen} - \text{PR} \ge 4$).

---

### 8.3 Tactical Concealment vs. Structural Cover & Barrier Penetration ($\text{PR}_{\text{barrier}}$)

Most indoor furniture, interior partitions, and vehicle body panels conceal a target's exact silhouette without reliably stopping rifle fire. Satya separates **visual obscurement** (`CoverGrade`) from **material ballistic resistance** (`MaterialClass` $\to \text{PR}_{\text{barrier}} \in [0, 10]$ in `barrier_ballistics.rs`):

1. **Concealment (`CoverGrade::Concealment`, `+3` $\Delta_{\text{ext}}$, $\text{PR}_{\text{barrier}} = 0$)**:
   - Trimmed hedges, dense bushes, tall grass, curtains, and smoke obscure aim (`+3` $\Delta_{\text{ext}}$ when $> 1\text{ m}$ from the shooter) while shedding negligible energy (`0` Penetration or Impact loss).
2. **Sequential Barrier Perforation & Granular Momentum Loss**:
   - Each traversed wall edge or deduplicated prop applies its active $\text{PR}_{\text{barrier}}$ (for granular masonry/earth/live wood, ultra-light $< 60\text{ gr}$ / $< 900\text{ J}$ PDW darts suffer `-2` Penetration against the barrier):
     $$\text{Pen}_{\text{res}} = \text{Pen}_{\text{eff}} - \text{PR}_{\text{barrier}}, \qquad \text{Impact}_{\text{res}} = \max(1,\; \text{Impact}_{\text{eff}} - \text{PR}_{\text{barrier}})$$
   - If $\text{Pen}_{\text{eff}} \le \text{PR}_{\text{barrier}}$, the barrier **stops** the projectile (`0` through-cover trauma; shooter must hit an exposed zone at full `+4` Low / `+6` High Cover $\Delta_{\text{ext}}$, or `+16` Blocked if no LoS exists).
   - **Overmatched Cover (`Pen - ΣPR >= 3`)**: When long-arm ammunition (`5.56mm` `Pen 6`, `7.62 NATO` `Pen 7`) fires into soft furniture (`Sofa` `PR 1`, `Desk` `PR 2`, `Car` cabin door `PR 3`), Low Cover degrades from `+4` to **`+2` $\Delta_{\text{ext}}$** because the shooter can fire straight through the cover into the crouched silhouette.
   - **Blind Partition Fire (`!has_los` + Perforates)**: Firing through an opaque `Drywall` wall (`9mm` or rifle) or `Brick` wall (`7.62×51mm NATO` `Pen 7 > PR 6`) is permitted as a **`+6` $\Delta_{\text{ext}}$** blind/known-cell shot, chaining $(\text{Pen}_{\text{res}}, \text{Impact}_{\text{res}})$ directly into the target's worn body armor (§8.2).

| Material Class (`MaterialClass`) | Representative Map Edges & Props | $\text{PR}_{\text{barrier}}$ *(Stud / Sub-Zone)* | Ballistic Reality Across Calibers |
| :--- | :--- | :---: | :--- |
| **`Foliage`** | `Hedge`, `Bush`, brush | **`0`** | Pure concealment (`+3` $\Delta_{\text{ext}}$); stops nothing. |
| **`Glass` / `Upholstery`** | `Window` pane; `Sofa`, `Loveseat`, `Armchair`, `Booth`, `Bed` | **`1`** | Shaves `-1` off `9mm` (`Pen 2 -> Res Impact 5`); negligible vs. rifles. |
| **`Drywall`** | Interior partition wall | **`1`** *(**`3`** on `10%` `2×4` wood stud)* | Gypsum cavity is punched by `9mm` & rifles; hitting a wood stud (`PR 3`) stops `9mm` (`Pen 2`) while `5.56mm` (`Pen 6`) punches straight through. |
| **`WoodFurniture` / `Ceramic`** | `Desk`, `DiningTable`, `Bookshelf`, `Wardrobe`, `WoodCrate`; `Toilet` | **`2`** | Stops standard `9mm` JHP (`Pen 2 <= 2`); completely overmatched by `5.56mm` / `7.62mm` rifles (`+2` $\Delta_{\text{ext}}$). |
| **`SolidWood` / `SheetSteel` / `Appliance`** | `BarCounter`, `PicnicTable`; `Dumpster`, `DrumCluster`; `Fridge` | **`3`** *(**`5`** on `25%` compressor)* | Stops standard handguns (`Pen 1–3`); perforated by shotguns (`Pen 4`) and rifles (`Pen 5–8`). |
| **`VehicleBody`** | `Car`, `UtilityVan` cabin & doors *(vs. front hood)* | **`3`** *(**`9`** engine block)* | Rifles punch cleanly through car doors (`PR 3`), whereas the front engine-bay cell (`PR 9`) stops `5.56mm` and `7.62 NATO`. |
| **`Brick` / `ConcreteBlock`** | Exterior brick masonry & CMU compound walls | **`6`** *(Granular)* | Stops pistols, `12-Ga` (`Pen 4`), `7.62×39mm` (`Pen 5`), `5.7mm` (`eff 4`), and `5.56mm` (`Pen 6`); **defeated by `7.62×51mm NATO` (`Pen 7`) and `6.5mm CM` (`Pen 8`)**. |
| **`Earth` / `LiveWood`** | Soil-filled `Planter`, `MediumTree` *(vs. `LargeTree`)* | **`7`** *(**`9`** `2×2` trunk)* | Packed soil and mature trunks stop `7.62×51mm NATO` (`Pen 7 <= 7`). |
| **`ReinforcedConcrete` / `ArmorSteel`** | `JerseyBarrier`, `Boulder`, structural bulkhead / blast door | **`9` / `10`** | Stops all service rifles (`Pen 5–8`); defeated only by `.338` AP / `.50 BMG` (`Pen 10–12`). |

---

## 9. Complete Worked Examples

### 9.1 Worked Example 1: Ranged Firefight (Alley Ambush — Sear-Modified Drum Pistol vs. Duty Service Pistol)

This example shows how **Tempo**, **Control**, and **Force** resolve a 15-meter gunfight behind partial cover, and how **Ammunition, Action Type, Drum Feed, Tolerances, and Inverse Accuracy Wobble** play out tick-by-tick.

#### The Combatants (15 meters apart in a rain-slicked alley behind concrete pillars)
1. **Mateo — Gang Enforcer with a Sear-Modified Machine Pistol (`#2` in Catalog: Glock w/ Auto-Switch + `50-rd` Drum)**:
   - **Weapon Engineering Profile**:
     - **Ammo**: Bulk `9×19mm` (`115 gr` FMJ, `5.0 gr` powder, `50-rd` Drum Mag $\to \text{Rel}_{\text{feed}} = -2$).
     - **Architecture**: Striker w/ conversion auto-switch ($\eta_{\text{action}} = 0.98$, $\text{Rel}_{\text{action}} = -1$), `10.2 cm` non-free-floated barrel, Loose/Street tolerance ($\tau_{\text{tol}} = -1.0, Q_{\text{mfg}} = -1$).
     - **Emergent Stats**: **Accuracy $\text{Acc} = -2$** (`6.0 MOA`), **Reliability $\text{Rel} = -2$**, **Clear Jam `12 / 28 Ticks`**, **Recoil `R = 4` (`R = 8` on Full-Auto Burst)**, **Inverse Accuracy Wobble $W_{\text{yaw}} = 3$** $\implies$ **`Raw Impact = 7` (tumbling keyholing rounds!), `Base Pen = 2`**.
   - **Character & Pool**: `SPD 13`, `FOR 11`, `DEX 13`, `SIG 12`, `OODA 12`, *Fully Automatic Weapons* (`DEX/FOR`, $C_{\text{skill}} = 12$, **`N = 2`**) $\implies P_{\text{base}} = 12 + 3(2) = \mathbf{18}$ points ($B = 6$; Ceilings: `T_max 13 / C_max 12 / F_max 11`).

2. **Sgt. Vance — Veteran Detective with a Duty `9mm` Service Pistol (`#1` in Catalog: Glock 19 Gen6 + Smart-Optic)**:
   - **Weapon Engineering Profile**:
     - **Ammo**: Duty `9×19mm` (`124 gr` JHP, `5.2 gr` powder, `15+1` Box Mag $\to \text{Rel}_{\text{feed}} = 0$).
     - **Architecture**: Striker-Fired ($\eta_{\text{action}} = 0.99$, $\text{Rel}_{\text{action}} = +1$), `10.2 cm` barrel, Service tolerance ($\tau_{\text{tol}} = 0.0, Q_{\text{mfg}} = +1$), plus Micro Red-Dot Smart-Optic (`-2` $\Delta_{\text{tool}}$).
     - **Emergent Stats**: **Accuracy $\text{Acc} = 0$** (`+2` with Optic), **Reliability $\text{Rel} = +2$**, **Clear Jam `8 / 20 Ticks`**, **Recoil `R = 3` (`R = 5` on Double-Tap)**, **Wobble $W_{\text{yaw}} = 1$** $\implies$ **`Raw Impact = 7` (`124 gr` JHP expansion $I_{\text{cal}} = 3$), `Base Pen = 1`**.
   - **Armor**: Wearing a **Concealable Executive Soft Vest (`NIJ IIIA`: `PR_Ballistic = 4`, `BTD = 2`)** covering Chest `[11–14]` and Abdomen `[8–10]`.
   - **Character & Pool**: `SPD 10`, `FOR 11`, `DEX 14`, `FOC 13`, `SIG 13`, `DIS 14`, `OODA 11`, *Tactical Handgun* (`DEX/FOC`, $C_{\text{skill}} = 13$, **`N = 4`**) $\implies P_{\text{base}} = 12 + 3(4) = \mathbf{24}$ points ($B = 8$; Ceilings: `T_max 10 / C_max 13 / F_max 11`).

#### Tick-by-Tick Firefight Exchange
- **Tick 1 — Mateo's Full-Auto Suppressive Blitz (`Tempo + Force`, Low `Control`)**:
  - Mateo acts first (`OODA 12`, `SPD 13`). He steps out from behind a dumpster and hoses Vance's concrete pillar with a **10-round full-auto burst** (`Base TC 8`, Recoil Rating `R = 8`):
    - Mateo allocates **`8 Tempo / 2 Control / 8 Force`** ($8 + 2 + 8 = 18$).
    - **Why he needs `8 Force`**: Firing a lightweight machine pistol on full-auto has a massive Recoil Rating (`R = 8`). By committing `8 Force`, Mateo matches `R = 8` and negates the recoil dispersion penalty!
    - **Why his `Control` is only `2`**: Because his *Full-Auto* skill is only `N = 2` (Pool = `18`) and he spent `8 Tempo` (for a fast `6-tick` burst) and `8 Force` (for recoil bracing), he only has **`2` points left for `Control`** (`Guard = 2`, leaning half his body out from cover) and suffers his weapon's **`Acc = -2`** penalty!
    - **Malfunction Trigger Check**: Mateo rolls `N = 2` pairs: Pair 1 succeeds (`1` stray bullet clips Vance's chest!), but Pair 2 fails with a floor-breach (`raw_botches = 1`, `SuccessWithComplication`). Because $\text{raw\_botches } (1) > \text{Mateo's Drum Pistol Rel } (-2)$, the `50-rd` drum suffers a **Minor Malfunction (Stovepipe)** at the end of the burst, requiring **`12 Ticks` (`3.0s`)** to clear!
    - **Resolving the Stray Hit Against Vance's Soft Kevlar Vest**:
      - Mateo's wobbling `9mm` FMJ round (`Raw Impact = 7`, `Base Pen = 2`) hits Vance's Chest (`Location 12`), which is protected by his **NIJ IIIA Soft Vest (`PR = 4, BTD = 2`)**.
      - Since $\text{Effective Pen } (2) \le \text{PR } (4)$, the Kevlar **stops the bullet** (`Zero Bleed`), and the wobbling round's sideways impact is caught by the aramid weave! Vance takes $\text{BABT} = 7 - 2 = \mathbf{5}$ (**Bruised Ribs**: `-2` to `Force Cap`, `+1` `wound_shock_index`, which Vance shrugs off with an `ExtraordinarySuccess` on his `HLT/DIS` `WSR` save!).

- **Tick 2 — Vance's Composure Check & "Slicing the Pie" Counter-Shot (`Control + Tempo`)**:
  - Concrete chips spray across Vance's face from Mateo's `8-Force` suppressive burst, triggering a **Suppression Morale Check** ($\Delta_{\text{morale}} = \lfloor 8/2 \rfloor = +4$). Vance rolls **Discipline (`DIS 14`, `N = 2`)** and scores `2` successes—he stays **Steady**.
  - On **Tick 2**, while Mateo is still recovering from his burst until **Tick 7** (`1 + 6 ticks`) with his torso exposed (`Control = 2`) and a stovepiped drum, Vance leans just an inch around the pillar ("slicing the pie") and fires a **controlled double-tap** (`Base TC 9`, Recoil `R = 5`):
    - Vance allocates **`7 Tempo / 12 Control / 5 Force`** ($7 + 12 + 5 = 24$).
    - **`5 Force`** matches the `R = 5` recoil of a 9mm double-tap.
    - **`12 Control`** gives Vance pinpoint sight alignment **and** keeps `90%` of his body behind the concrete pillar (`Guard = 12`), while exploiting Mateo's exposed `Control 2` posture!
    - Vance rolls `N = 4` pairs of `2d20` and scores `4` Net Successes ($\Delta M = +4$)! His two `124 gr` JHP rounds (`Raw Impact = 7 + 2 = 9`) slam into Mateo's unarmored right shoulder (`Location 15`), inflicting a **Tier 3 Severe Shoulder Wound** (`-10` to `Force` and `Control` Caps, `+2` `wound_shock_index`, and **Grade 3 Hemorrhage**)!

- **Tick 7 — Mateo Drops to `Broken` Morale (Psychological Allocation Lock)**:
  - With a shattered shoulder, a jammed drum (`12 Ticks` to clear), and `Wound Shock 2` (`-4` Pool), Mateo fails his **Discipline (`DIS 11`)** Morale Check by 2 tiers (`Botch`) and drops to **Broken Morale** (`-4` Pool, **active offensive `Force` locked to `0`**), forcing him to scramble backward into cover.

---

### 9.2 Worked Example 2: Engineering Comparison — Loose-Tolerance `7.62×39mm` AKM vs. Free-Floated Bolt-Action `6.5mm Creedmoor` at 400 Meters

Why does a sniper choose a heavy, tight-tolerance bolt-action **`6.5mm Creedmoor` (`#11` in Catalog)** over a rugged **`7.62×39mm` AKM (`#9` in Catalog)** for a 400-meter overwatch shot against an armored target wearing a **UHMWPE Patrol Carrier (`PR_Ballistic = 6, BTD = 4`)**?

1. **Aerodynamic Energy Retention at `d = 400 m` & Action Gas Efficiency**:
   - **AKM (`7.62×39mm`, `123 gr` bullet, `25.5 gr` powder, `41.5 cm` clamped barrel, Long-Stroke Piston $\eta_{\text{action}} = 0.97$, $D_{\text{decay}} = 420\text{ m}$)**:
     - Muzzle Energy $E_0 = 2,040\text{ J}$, but at $d = 400\text{ m}$, air drag reduces impact energy to:
       $$E_k(400\text{ m}) = 2,040 \times e^{-400 / 420} = \mathbf{787\text{ J}} \implies I_{\text{KE}} = \mathbf{3}\text{ (dropped from } 5\text{ to } 3\text{!)}$$
     - Because its loose combat tolerance ($\tau_{\text{tol}} = -1.0$) and non-free-floated barrel give it **$\text{Acc} = -1$ (`3.8 MOA`)**, its mechanical dispersion cone at `400m` is `43 cm` wide (`+4` $\Delta_{\text{ext}}$ range dispersion + `+1` $\Delta_{\text{tool}}$ from `Acc -1`), and its wobbling bullet (`W_yaw = 2`) has its Armor Penetration reduced at `400m` to **`Base Pen = 3`**—which **cannot penetrate** the target's `PR 6` Patrol Plate!
   - **AT-X Precision Rifle (`6.5mm Creedmoor`, `140 gr` Match OTM, `42.5 gr` powder, `61.0 cm` Free-Floating Match Barrel, Bolt-Action $\eta_{\text{action}} = 1.00$, $D_{\text{decay}} = 950\text{ m}$)**:
     - Sealed bolt loses **zero gas** (`1.00×`) and the `61 cm` barrel extracts `+3%` bonus velocity ($E_0 = 3,260\text{ J}$). Thanks to the sleek boat-tail bullet's `950m` decay scale, impact energy at `400m` is still:
       $$E_k(400\text{ m}) = 3,260 \times e^{-400 / 950} = \mathbf{2,140\text{ J}} \implies I_{\text{KE}} = \mathbf{5}$$
     - Its free-floated match barrel (`+1.0` Mount, `+1.5` Barrel), bolt action (`+1.5`), heavy chassis (`+0.5`), tight tolerance (`+1.0`), and match ammo (`+1.0`) yield **`Acc = +4` (`0.35 MOA` — a `4 cm` group at `400m`!)**, lowering the sniper's $\text{TN}_P$ by **`-4`**, while its high sectional density (`+3`) gives **`Base Pen = 7`** even at `400 meters`—punching clean through the target's `PR 6` plate!
2. **When the Polarity Flips (Muddy Trench CQB at `15 Meters`)**:
   - In a flooded, mud-choked trench at `15m` (`Environmental Fouling = +2`), the sniper rifle's `112 cm` Overall Length imposes **`+3` CQB crowding penalty**, its bolt action adds **`+4 Ticks` (`1.0s`)** between every single shot, and its tight match chamber requires careful maintenance.
   - Meanwhile, the AKM's **`Rel = +4`** (`+2` Long-Stroke Piston, `+1` Loose Tolerance, `+1` Rugged Mfg) completely ignores `1–4` raw botches and `+2` mud fouling without jamming, cycles semi-auto or full-auto with zero bolt-cycling delay, and its **`W_yaw = 2` wobbling `7.62mm` rounds** inflict horrific **`Raw Impact = 9` (`Tier 4`)** erratic wound channels at close range!
