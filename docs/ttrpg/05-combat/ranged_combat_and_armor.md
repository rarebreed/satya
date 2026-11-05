# Satya TTRPG Ranged Combat, Suppression, Armor & Weapon Reliability

This chapter defines the canonical rules for **Ranged Firefights, Recoil & Suppression, Ballistic & Edged Armor (`PR` vs. Behind-Armor Blunt Trauma `BABT`), and Weapon/Equipment Reliability (`Rel`)** in **Satya**. It builds directly upon:
- **[Mathematical Notation, Variables & Formula Glossary](../01-foundation/mathematical_notation_glossary.md)** (for all mathematical variables and ballistics/combat symbols $R, \text{PR}_{\text{Edged}}, \text{PR}_{\text{Ballistic}}, \text{BABT}, \text{Rel}$),
- **[Combat Architecture, Tactical Triad & Pools](tactical_triad_and_pools.md)** (for the "Envelope & Fill" Tactical Pool $P_{\text{base}} = C_{\text{combat}} + 3N$),
- **[Psyche, Morale & Social Combat](../04-psyche-and-social/psyche_morale_and_social_combat.md#34-combat-morale-tactical-ooda-behavior--the-five-conditions-of-morale)** (for Suppression Morale Checks and the Five Conditions of Morale), and
- **[Health, Anatomical Trauma, Fatigue & Mental Conditions](../06-health-and-survival/health_and_trauma.md)** (for Hit Locations, Called Shots, Structural Wounds, and Hemorrhaging).

---

## 1. Ranged Combat, Suppression & Firefight Mechanics

Ranged combat uses the exact same **Tempo, Control, and Force** triad on the **[Count-Up Action Clock](../02-core-mechanics/action_economy_and_ooda.md)** as close-quarters combat, interacting with weapon properties, cover, and awareness:

- **Tempo (Aim & Action Time)**: Governs how many ticks (`TC`) you spend bringing the weapon to bear—from a fast point-shoot or hip-fire snapshot (high **Tempo**, lower `TC`) to a deliberate multi-tick sighted shot (low **Tempo**, higher **Control**).
- **Control (Sight Alignment & Cover Exposure)**: Governs marksmanship precision (`DEX/SIG` or `DEX/FOC`) AND how much of your body is exposed out of cover while firing (`Guard`).
- **Force (Stance, Recoil Bracing & Volume of Fire)**: Governs how rooted and braced your shooting platform is (`FOR`/`MAS`) to absorb recoil during rapid/automatic fire (*Fully Automatic Weapons* `DEX/FOR`) and maintain suppressive pressure.
- **Awareness**: Visual (`SIG`), auditory (`HEA`), and intuitive (`ITU`) tracking of targets, crossfires, and incoming suppression (`OODA`).

### 1.1 Ranged Weapon Recoil & Action Modes in the Triad

Unlike melee weapons where **Force** directly multiplies muscle impact, ranged weapons have a fixed ballistic energy per round, a **Base Time Cost (`TC`)**, and a **Recoil / Bracing Rating (`R`)** based on caliber and fire mode:

| Ranged Fire / Action Mode | Base `TC` | Recoil / Bracing Need (`Force`) | How the Tactical Triad Governs the Shot |
| :--- | :---: | :---: | :--- |
| **Single Snap-Shot / Point-Shoot** | `7` | Low (`R = 3–4`) | Prioritizes **Tempo** (beating the target to the draw) + **Control** (point-aim & cover retention). |
| **Deliberate Sighted / Aimed Shot** | `11–12` | Moderate (`R = 4–6`) | Prioritizes **Control** (sight alignment, trigger squeeze, called shots, minimal cover exposure) at lower **Tempo**. |
| **Controlled Double-Tap / Short Burst (2–3 rds)** | `9` | Medium (`R = 6–7`) | Requires balanced **Force** to keep the second/third round on target alongside **Control** and **Tempo**. |
| **Full-Auto Burst / Suppressive Fire (5–15 rds)** | `8–10` | High (`R = 8–12`) | Demands heavy **Force** (`FOR`/`MAS`) to muscle down muzzle climb and project a beaten zone of **Suppression** (`Force` vs. target's `DIS` Morale). |
| **Bounding / Shooting on the Move** | `10` | Unbraced (`+3` to `R`) | High **Tempo** covers ground quickly, but moving starves **Force** (recoil stability) or **Control** (sight picture). |

---

## 2. Armor, Penetration & Behind-Armor Blunt Trauma (BABT)

In traditional RPGs, armor either makes a character harder to hit ("Armor Class") or subtracts a flat number from a generic Hit Point pool. In reality, armor makes a wearer slightly *easier* to hit (by adding bulk and Encumbrance), while acting as a **physical barrier** that stops penetration up to its material limit—yet still transfers kinetic momentum (**Behind-Armor Blunt Trauma**) into the wearer's ribs or skull!

### 2.1 Armor Profiles (`Coverage`, `Protection Rating`, `Blunt Damping` & `Mass`)

Every piece of protective gear is defined by **four mechanical ratings**:
1. **Anatomical Coverage**: Which **[Hit Locations](../06-health-and-survival/health_and_trauma.md#1-hit-locations--specificity-of-damage-effects)** are covered (e.g., a *Plate Carrier* covers `Chest [11–14]` and `Upper Abdomen [8–9]`, leaving `Pelvis [7]`, `Shoulders/Arms [15–18]`, `Legs [1–6]`, and `Neck/Head [19–20]` exposed; a *Ballistic Helmet* covers `Cranium [20]`).
2. **Penetration Resistance (`PR` by Damage Type)**:
   - **`PR_Edged`**: Resistance against slashes, cuts, and stabs.
   - **`PR_Ballistic`**: Resistance against bullets and shrapnel (aligned to NIJ / 2035 standards).
3. **Blunt Trauma Damping (`BTD`)**: How much kinetic shock the armor's trauma pad or rigid shell absorbs before the remainder crushes the tissue underneath.
4. **Mass (`kg`) & Ergonomic Factor**: Feeds directly into the character's **[Encumbrance Tier](../06-health-and-survival/movement_vehicles_and_environment.md#2-encumbrance--load-bearing-architecture)** (`×0.8` for articulated armor).

| Armor / Protective Suit | Covered Zones | `PR_Edged` | `PR_Ballistic` | Blunt Damping (`BTD`) | Mass (`kg`) | Notes & Era Benchmark |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- |
| **Heavy Leather / Synth-Weave Jacket** | Torso, Arms | `2` | `1` | `1` | `2.5 kg` | Concealable street protection against knives and road rash. |
| **Concealable Soft Kevlar Vest (NIJ II/IIIA)** | Chest, Abdomen | `2` *(Slash)* / `1` *(Icepick)* | `4` *(Stops Pistol/SMG)* | `2` | `2.8 kg` | Worn under clothing; rifle rounds (`Pen >= 6`) punch straight through! |
| **Tactical Plate Carrier (Level IV Ceramic/UHMWPE)** | Vital Chest & Upper Back | `6` | `8` *(Stops Rifle)* | `5` | `8.5 kg` | Stops `5.56mm` / `7.62mm` rifle hits, transferring **Blunt Rib Shock** (`BABT`). |
| **Ballistic Combat Helmet + Visor** | Head (`Cranium` / `Eyes`) | `5` | `4` *(Visor `3`)* | `4` | `1.6 kg` | Protects against shrapnel, pistol rounds, and blunt concussion. |
| **Full Riot / Breacher Suit** | Full Body | `6` | `5` | `6` | `16.0 kg` | High blunt/edged protection; `+6` Heat Retention penalty. |
| **2035 Valkyrie / Recon Exo-Carapace** | Full Body (Sealed) | `8` | `9` | `8` | `18.5 kg` *(`-20 kg` Exo Offset)* | Powered myomer frame offsets its own weight while battery is active. |

### 2.2 Resolving Armor vs. Incoming Hits (`Penetration` vs. `BABT`)

When an attack hits an armored Hit Location:
1. **Called Shots to Bypass Armor**: An attacker who sees the target's armor can use **`Control`** to target an unarmored gap (e.g., Thigh/Femoral `+2`, Neck `+8`, or Pelvis `+4`; see **[Hit Locations](../06-health-and-survival/health_and_trauma.md#11-determining-hit-location-1d20-roll-vs-control-called-shots)**).
2. **If Struck on an Armored Zone**: Compare the weapon's **Penetration Rating (`Pen`)** (plus `+1` per `2` net cascade successes) against the armor's **`PR`**:
   - **Case A — Armor Stops Penetration ($\text{Pen} \le \text{PR}$)**:
     The blade or bullet does **not** perforate the skin (**Zero Bleed**). However, the wearer suffers **Behind-Armor Blunt Trauma (`BABT`)** equal to:
     $$\text{Blunt Trauma} = \max(0,\; \text{Raw Strike/Bullet Impact} - \text{BTD})$$
     *(For example, taking a `7.62mm` rifle round in a Level IV Ceramic Plate stops the bullet from piercing the heart, but transfers `Tier 1–2` Blunt Chest Trauma—cracking a rib, knocking the wind out of the wearer for `+1` `wound_shock_index`, and testing their `Root` against knockdown!)*
   - **Case B — Armor Overmatched ($\text{Pen} > \text{PR}$)**:
     The projectile or blade punches through the armor with reduced energy ($\text{Effective Trauma} = \text{Raw Impact} - \text{PR}$), inflicting a penetrating **Structural Wound & Hemorrhage** and degrading the armor plate's `PR` in that zone by **`-1` to `-2`** (ceramic plate cracking).

---

## 3. Weapon & Equipment Reliability, Maintenance & Malfunctions

Modern firearms, cyberdecks, drones, and exo-rigs are precision machines that can foul, overheat, or jam when neglected or pushed to their limits.

### 3.1 Weapon & Tool Reliability Ratings (`Rel`)

Every mechanical or electronic device has a **Reliability Rating (`Rel`)** from **`Rel +4` (Legendary / Over-Engineered)** to **`Rel -4` (Improvised / jury-rigged / worn out)**:
- **Rugged Service Weapons** (e.g., stock AK-pattern rifle, factory Glock, solid crowbar): **`Rel +2`**
- **Standard Precision / Issue Gear** (e.g., standard carbine, trauma kit, commercial drone): **`Rel 0`**
- **Finicky / High-Tune / Modified Gear** (e.g., sear-modified machine pistol with drum mag, experimental prototype, unmaintained salvage): **`Rel -2` to `-4`**

### 3.2 When Malfunctions Trigger (Integration with `2d20` Botch Rules)

Satya does **not** require a separate "jam check" roll on every shot. Instead, malfunctions emerge naturally from the **[`2d20` Cascade Outcome Tiers](../02-core-mechanics/cascading_d20_and_tasks.md#2-signed-outcome-spectrum--n-to-n-botch-rules--outcometier)**:
1. **On `SuccessWithComplication` (`OutcomeTier::PartialSuccess` — `total_successes >= 1` AND `raw_botches >= 1`)**:
   - The shot or task **succeeds**, **but** if $\text{raw\_botches} > \text{Rel}$, the equipment suffers a **Minor Malfunction / Strain** *after* the action resolves (e.g., a firearm fires the hitting round but fails to eject the spent casing—a **Stovepipe / Failure-to-Feed** requiring an `8-tick` `"Tap, Rack, Bang"` clearance before it can fire again; or a lockpick bends, dropping its Tool Tier by `-1`).
2. **On `BotchFailure` (`OutcomeTier::CriticalFailure` — `total_successes == 0` AND `effective_botches >= 2`)**:
   - If $\text{effective\_botches} + \text{Environmental Mud/Fouling} > 2 + \text{Rel}$, the weapon suffers a **Major Malfunction / Double-Feed** (`20–32 ticks` to strip and clear) or runs dry on an uncounted magazine!
3. **Preventative Maintenance**: Spending `30 minutes` of Downtime with a cleaning kit and *Gunsmithing & Armorer* (`DEX/ANA`) restores fouled reliability to baseline and grants **`+1` `Rel`** for the next combat scene.

---

## 4. Complete Worked Examples

### 4.1 Worked Example 1: Ranged Firefight (Alley Ambush — Street SMG vs. Tactical Handgun)

This example shows how **Tempo**, **Control**, and **Force** resolve a 15-meter gunfight behind partial cover, and how **Suppression** and **Wound Shock** trigger **Morale Allocation Constraints**.

#### The Combatants (15 meters apart in a rain-slicked alley behind concrete pillars)
1. **Mateo — Gang Enforcer with a Sear-Modified Machine Pistol (Glock w/ Auto-Switch)**:
   - **Characteristics**: `SPD 13`, `FOR 11`, `DEX 13`, `KIN 12`, `SIG 12`, `INS 12`, `ITU 13`, `CRE 11`, `FOC 11` ($D_{\text{cog}} = 11$), `DIS 11` ($\text{OODA} = \lfloor(13 + 12 + 11 + 13)/4\rfloor = 12$).
   - **Psyche**: `AGG 14`, `BLG 15`, `SLF 11`.
   - **Skills**: *Pistols & Revolvers* (`DEX/SIG`, `N = 3`), *Fully Automatic Weapons* (`DEX/FOR`, $C_{\text{skill}} = 12$, **`N = 2`**).
   - **Tactical Pool (when firing Full-Auto `N = 2`)**: $C_{\text{combat}} = \lfloor(12 + 12)/2\rfloor = 12 \implies P_{\text{base}} = 12 + 3(2) = \mathbf{18}$ points (Neutral baseline $B = 6$).
   - **Aspect Ceilings**: **`Tempo Cap = 13`** (`SPD 13`), **`Control Cap = 12`** (`DEX/SIG 12`), **`Force Cap = 11`** (`FOR 11`).

2. **Sgt. Vance — Veteran Detective with a 9mm Service Pistol (Semi-Auto + Smart-Optic `-2`)**:
   - **Characteristics**: `SPD 10`, `FOR 11`, `DEX 14`, `ANA 11`, `FOC 13` ($D_{\text{cog}} = 12$), `SIG 13`, `INS 12`, `ITU 12`, `DIS 14` ($\text{OODA} = \lfloor(12 + 12 + 12 + 10)/4\rfloor = 11$).
   - **Psyche**: `AGG 9`, `CNF 14`, `DIS 14` *(Unflappable veteran)*.
   - **Skill**: *Tactical Handgun & Point Shooting* (`DEX/FOC`, $C_{\text{skill}} = 13$, **`N = 4`**).
   - **Tactical Pool ($P_{\text{base}}$)**: $C_{\text{combat}} = \lfloor(13 + 11)/2\rfloor = 12 \implies P_{\text{base}} = 12 + 3(4) = \mathbf{24}$ points (Neutral baseline $B = 8$).
   - **Aspect Ceilings**: **`Tempo Cap = 10`** (`SPD 10`), **`Control Cap = 13`** (`DEX/SIG 13`), **`Force Cap = 11`** (`FOR 11`).

#### Tick-by-Tick Firefight Exchange
- **Tick 1 — Mateo's Full-Auto Suppressive Blitz (`Tempo + Force`, Low `Control`)**:
  - Mateo acts first (`OODA 12`, `SPD 13`). He steps out from behind a dumpster and hoses Vance's concrete pillar with a **10-round full-auto burst** (`Base TC 8`, Recoil Rating `R = 8`):
    - Mateo allocates **`8 Tempo / 2 Control / 8 Force`** ($8 + 2 + 8 = 18$).
    - **Why he needs `8 Force`**: Firing a lightweight machine pistol on full-auto has a massive Recoil Rating (`R = 8`). By committing `8 Force` (planting his stance and muscling down the muzzle climb), Mateo matches `R = 8` and negates the recoil penalty!
    - **Why his `Control` is only `2`**: Because his *Full-Auto* skill is only `N = 2` (Pool = `18`) and he spent `8 Tempo` (for a fast `6-tick` burst) and `8 Force` (for recoil bracing), he only has **`2` points left for `Control`**!
    - **Mechanical Consequence of `Control 2`**:
      1. *Poor Precision*: He cannot thread a bullet around Vance's concrete cover; his burst chews up the concrete pillar, creating heavy **Suppression** ($\Delta_{\text{morale}} = +4$ based on his `8 Force`).
      2. *Wide-Open Exposure (`Guard = 2`)*: To muscle the SMG burst quickly, Mateo leaned half his upper body out from behind the dumpster!

- **Tick 2 — Vance's Composure Check & "Slicing the Pie" Counter-Shot (`Control + Tempo`)**:
  - Concrete chips spray across Vance's face from Mateo's `8-Force` suppressive burst, triggering a **Suppression Morale Check** ($\Delta_{\text{morale}} = +4$).
  - Vance rolls his **Discipline (`DIS 14`, `N = 2`)** against $\text{TN}_{\text{morale}} = 31 - 14 + 4 = 21$ and scores `2` successes—he stays **Steady** (`+0` pool loss) instead of flinching into **Wavering**!
  - Now on **Tick 2**, while Mateo is still recovering from his burst until **Tick 7** (`1 + 6 ticks`) with his torso exposed (`Control = 2`), Vance leans just an inch around the right edge of the pillar ("slicing the pie") and fires a **controlled double-tap** (`Base TC 9`, Recoil `R = 5`):
    - Vance allocates **`7 Tempo / 12 Control / 5 Force`** ($7 + 12 + 5 = 24$).
    - **`5 Force`** matches the mild `R = 5` recoil of a 9mm double-tap.
    - **`12 Control`** gives Vance pinpoint sight alignment **and** keeps `90%` of his body behind the concrete pillar (`Guard = 12`), while exploiting Mateo's exposed `Control 2` posture!
    - Vance rolls `N = 4` pairs of `2d20` with a massive precision advantage (`Control 12` vs. Mateo's `Control 2`), hitting Mateo's exposed right shoulder and chest protector!

- **Tick 7 — Mateo Drops to `Broken` Morale (Psychological Allocation Lock)**:
  - Hit in the shoulder (`Moderate Arm Wound: -5 to Force & Control Caps`, dropping Mateo's $F_{\max}$ to `6` and $C_{\max}$ to `7`; `Wound Shock 2: -4 to Pool`), Mateo fails his **Discipline (`DIS 11`)** Morale Check by 2 tiers (`Botch`) and drops from **Steady** straight to **Broken Morale** (`-4` to Pool)!
  - **Mateo's Degraded State**:
    - His Tactical Pool drops from `18` down to $18 - 4\text{ (Shock)} - 4\text{ (Broken)} = \mathbf{10}$ points.
    - Because his morale is **Broken**, **active offensive `Force` is locked to `0`**—he is psychologically incapable of stepping out to press the attack.
    - Mateo must spend all `10` remaining points on **`Tempo` (`7`)** and **`Control` (`3`)** to scramble backward behind the dumpster and retreat down the alley while firing blind one-handed suppression shots (`Force 0`) behind him!

---

### 4.2 Worked Example 2: Ranged Maneuver & Evasion (Sniper Overwatch vs. Grenade Flush)

What happens when a combatant uses the Tactical Triad purely to **maneuver between cover** or **evade an explosive**?

- **Bounding Across an Open Street Under Sniper Overwatch (`Tempo + Control`)**:
  - Elena (`P_eff = 24`, `Tempo Cap = 13`, `Control Cap = 13`, `Force Cap = 10`) needs to cross a 10-meter gap between two burned-out cars while an enemy marksman watches the street.
  - If she runs upright in a straight line (**Pure `Tempo`**: `13 Tempo / 2 Control / 9 Force`), she crosses in minimal ticks, but her low **`Control 2`** means she takes a predictable path with a high visual profile—giving the marksman an easy shot!
  - Instead, she executes a **Low Evasive Bound (`12 Tempo / 12 Control / 0 Force`)**: she sprints low, weaves unpredictably from shadow to shadow, and slides behind the engine block (`Control 12` imposes a high difficulty penalty on the sniper's tracking shot, while `Tempo 12` gets her across before the sniper's `Aimed Shot TC 12` finishes!).
- **Diving Clear of a Fragmentation Grenade (`Tempo + Force`, Low `Control`)**:
  - A grenade lands 2 meters from Elena's cover, detonating in **`4 ticks`**.
  - Here, subtle cover discipline (`Control`) won't save her from shrapnel inside the blast radius—she needs raw reaction time (**`Tempo`**) to beat the `4-tick` fuse AND explosive physical leap distance (**`Force`**, powering her `Jumping` / `Parkour` push-off)!
  - She dumps `Control` to `2` and commits **`12 Tempo / 2 Control / 10 Force`** (`TF — The Dive`): she launches herself headfirst over a low wall (`Force 10` maximizes leap distance; `Tempo 12` completes the dive in `3 ticks`, beating the `4-tick` detonation), landing prone and temporarily unguarded (`Control 2`) on the far side, but unharmed by the blast!
