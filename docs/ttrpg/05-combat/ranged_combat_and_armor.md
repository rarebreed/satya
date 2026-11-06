# Satya TTRPG Ranged Combat, Granular Ballistics, Firearms Engineering & Armor

This chapter defines the canonical rules for **Ranged Firefights, Internal/External/Terminal Ballistics, Granular Firearm Engineering, Ammunition Physics, Weapon Reliability (`Rel`), and Body Armor (`PR` vs. Behind-Armor Blunt Trauma `BABT`)** in **Satya**. It builds directly upon:
- **[Mathematical Notation, Variables & Formula Glossary](../01-foundation/mathematical_notation_glossary.md)** (for all mathematical variables and ballistics/combat symbols $E_k, m_{\text{proj}}, m_{\text{prop}}, L_{\text{bbl}}, \text{OAL}, M_{\text{wpn}}, \Delta_{\text{CoG}}, \tau_{\text{tol}}, \text{Acc}, W_{\text{yaw}}, \text{Pen}, R, \text{PR}_{\text{Edged}}, \text{PR}_{\text{Ballistic}}, \text{BABT}, \text{Rel}$),
- **[Combat Architecture, Tactical Triad & Pools](tactical_triad_and_pools.md)** (for the "Envelope & Fill" Tactical Pool $P_{\text{base}} = C_{\text{combat}} + 3N$),
- **[Psyche, Morale & Social Combat](../04-psyche-and-social/psyche_morale_and_social_combat.md#34-combat-morale-tactical-ooda-behavior--the-five-conditions-of-morale)** (for Suppression Morale Checks and the Five Conditions of Morale), and
- **[Health, Anatomical Trauma, Fatigue & Mental Conditions](../06-health-and-survival/health_and_trauma.md)** (for Hit Locations, Called Shots, Structural Wounds, and Hemorrhaging).

---

## 1. Why Guns Don't Feel the Same in Satya: Granular Engineering Architecture

In most tabletop RPGs, firearms are reduced to interchangeable reskins of the same stat block (`1d8` or `2d6` damage, a flat range increment, and a magazine size). A cheap street-printed `9mm` pistol, a match-grade competition handgun, a loose-tolerance stamped `7.62×39mm` AKM, and a free-floated direct-impingement `5.56mm` carbine feel virtually identical at the table.

**Satya rejects generic weapon dice.** Every firearm is a physical machine governed by **four coupled engineering causal chains**:
1. **Internal & External Ballistics ($\to$ Kinetic Energy $E_k$)**: How projectile mass ($m_{\text{proj}}$), gunpowder charge ($m_{\text{prop}}$ in grains), barrel length ($L_{\text{bbl}}$), muzzle devices (suppressors, muzzle brakes), and the action's gas seal ($\eta_{\text{action}}$) generate muzzle velocity and retain kinetic energy downrange.
2. **Terminal Ballistics ($\to$ Trauma Magnitude, Penetration & Wound Channels)**: How impact kinetic energy ($E_k$), projectile caliber ($d_{\text{cal}}$), bullet construction, and **inverse accuracy / projectile wobble ($W_{\text{yaw}}$)** interact with armor plates (`Pen` vs. `PR` & `BABT`) and soft tissue (permanent crush cavity, hydrostatic shock, and erratic tumbling/keyholing wound channels).
3. **Mechanical & Ergonomic Engineering ($\to$ Accuracy $\text{Acc}$, Recoil $R$, & Handling `TC`)**: How action type, weapon weight ($M_{\text{wpn}}$), Center of Gravity relative to the chamber ($\Delta_{\text{CoG}}$), Overall Length ($\text{OAL}$), barrel length ($L_{\text{bbl}}$), barrel quality ($Q_{\text{bbl}}$), barrel mounting (e.g., **free-floating** vs. clamped), mechanical tolerances ($\tau_{\text{tol}}$), and ammunition quality ($Q_{\text{ammo}}$) govern marksmanship precision and recoil recovery.
4. **Tribology, Tolerances & Feed Mechanics ($\to$ Reliability $\text{Rel}$, Clear Jam Time $\text{TC}_{\text{clear}}$, & Strip Time $\text{TC}_{\text{strip}}$)**: How action geometry, feed mechanism (box magazine, drum, belt, tubular, cylinder, single-shot), **machining tolerance ($\tau_{\text{tol}}$ — which helps accuracy but hurts reliability!)**, and manufacturing quality ($Q_{\text{mfg}}$) determine malfunction thresholds and how many Count-Up Ticks it takes to clear a jam or field-strip the weapon.

```mermaid
flowchart TD
    subgraph Ammo["1. Ammunition & Propellant"]
        Mass["<b>Bullet Mass (m_proj)</b><br/>Grains (gr) · Sectional Density"]
        Powder["<b>Gunpowder Charge (m_prop)</b><br/>Grains (gr) · Chamber Pressure"]
        Cal["<b>Caliber (d_cal) & Profile</b><br/>9mm · .45 · 5.56 · 7.62 · 6.5CM<br/>FMJ · JHP · Match OTM · AP"]
        QualA["<b>Ammo Quality (Q_ammo)</b><br/>Surplus · Duty · Match Handload"]
    end

    subgraph GunArch["2. Firearm Mechanical Architecture"]
        Act["<b>Action Type</b><br/>Bolt · Pump · Single-Action · Striker<br/>Direct Impingement · Short/Long Piston"]
        Bbl["<b>Barrel Specs</b><br/>Length (L_bbl) · Quality (Q_bbl)<br/>Mounting (Free-Float vs. Clamped)<br/>Muzzle Brake / Suppressor"]
        Phys["<b>Ergonomics & Mass</b><br/>Weight (M_wpn) · Overall Length (OAL)<br/>Center of Gravity vs. Chamber (Δ_CoG)"]
        Tol["<b>Tolerance (τ_tol) & Mfg (Q_mfg)</b><br/>Tight Match (+Acc / -Rel)<br/>Loose Combat (-Acc / +Rel)"]
        Feed["<b>Feed System & Capacity</b><br/>Box Mag · Drum · Belt · Tubular<br/>Cylinder · Single-Shot"]
    end

    subgraph Outputs["3. Emergent Combat Mechanics"]
        KE["<b>Impact Kinetic Energy (E_k)</b><br/>m_proj × m_prop × η_bbl × η_action"]
        Acc["<b>Intrinsic Accuracy (Acc & MOA)</b><br/>Shifts Player Target (TN_P)<br/>Low Acc → High Yaw Wobble (W_yaw)"]
        Rec["<b>Recoil (R) & Handling (TC)</b><br/>Braced by Force (F) · CQB OAL"]
        Rel["<b>Reliability (Rel) & Maintenance</b><br/>Clear Jam TC · Field-Strip TC"]
        Dmg["<b>Terminal Wound & Penetration</b><br/>Raw Impact = I_KE + I_cal + W_yaw<br/>Armor Pen vs. PR & BABT"]
    end

    Mass --> KE
    Powder --> KE
    Bbl --> KE
    Act --> KE

    Act --> Acc
    Phys --> Acc
    Phys --> Rec
    Bbl --> Acc
    Tol --> Acc
    QualA --> Acc

    Act --> Rel
    Tol --> Rel
    Feed --> Rel

    KE --> Dmg
    Cal --> Dmg
    Acc -->|"Inverse Accuracy:<br/>Wobble / Tumbling (W_yaw)"| Dmg
```

---

## 2. Ranged Combat in the Tactical Triad (`Tempo`, `Control`, `Force`)

Ranged combat uses the exact same **Tempo, Control, and Force** triad on the **[Count-Up Action Clock](../02-core-mechanics/action_economy_and_ooda.md)** (`1 Tick = 0.25s`, `4 Ticks = 1.0s`) as close-quarters combat:

- **Tempo (`T` — Aim Speed, Target Acquisition & CQB Handling)**: Governs how many ticks (`TC`) you spend bringing the weapon to bear—from a fast point-shoot or hip-fire snapshot (high **Tempo**, lower `TC`) to a deliberate multi-tick sighted shot (low **Tempo**, higher **Control**). Modified by the weapon's **Overall Length ($\text{OAL}$)** and **Center of Gravity ($\Delta_{\text{CoG}}$)**.
- **Control (`C` — Sight Alignment, Trigger Squeeze & Cover Exposure)**: Governs marksmanship precision (`DEX/SIG` or `DEX/FOC`), called shots to bypass armor, AND how much of your body is exposed out of cover while firing (`Guard`).
- **Force (`F` — Stance, Recoil Bracing & Volume of Fire)**: Governs how rooted and braced your shooting platform is (`FOR`/`MAS`) to absorb the weapon's physical **Recoil Rating ($R$)** during rapid or automatic fire (*Fully Automatic Weapons* `DEX/FOR`) and project suppressive pressure.

### 2.1 Ranged Fire Modes, Base `TC` & Recoil Bracing

Unlike melee weapons where **Force** directly multiplies muscle impact, a firearm's projectile energy is propelled by chemical propellant (`grains` of powder), while the shooter's **Force ($F$)** braces against the weapon's **Effective Recoil Rating ($R_{\text{eff}}$)**:

| Ranged Fire / Action Mode | Base `TC` | Mode Recoil Adder ($\Delta R_{\text{mode}}$) | Rounds Expended | How the Tactical Triad Governs the Shot |
| :--- | :---: | :---: | :---: | :--- |
| **Single Snap-Shot / Point-Shoot** | `7` *(+OAL/CoG mod)* | `+0` *(Base $R_{\text{wpn}}$)* | `1 rd` | Prioritizes **Tempo** (beating the target to the draw in CQB) + **Control** (point-aim & cover retention). |
| **Deliberate Sighted / Aimed Shot** | `11–12` | `+1` *(Follow-through)* | `1 rd` | Prioritizes **Control** (sight alignment, breathing pause, trigger squeeze, called shots) at lower **Tempo**. Unlocks full positive **Weapon Accuracy ($\text{Acc}$)**. |
| **Controlled Double-Tap / Short Burst** | `9` *(Semi/Burst)* | **`+2` to `+3`** | `2–3 rds` | Requires balanced **Force ($F \ge R_{\text{eff}}$)** to keep the second/third round on target alongside **Control** and **Tempo**. Each `+2` Net Successes lands an additional round! |
| **Full-Auto Burst / Suppressive Fire** | `8–10` | **`+4` to `+6`** | `5–15 rds` | Demands heavy **Force** (`FOR`/`MAS`) to muscle down muzzle climb and project a beaten zone of **Suppression** ($\Delta_{\text{morale}} = \lfloor F / 2 \rfloor$). |
| **Bounding / Shooting on the Move** | `10` | **`+3` Unbraced** *(or Gait adder)* | `1–3 rds` | High **Tempo** covers ground quickly, but moving starves **Force** (recoil stability) or **Control** (sight picture). |

- **Resolving Recoil Bracing ($F$ vs. $R_{\text{eff}}$)**:
  $$\text{Unbraced Dispersion Penalty } (\Delta_{\text{ext, recoil}}) = \max(0,\; R_{\text{eff}} - F)$$
  If the shooter allocates $F \ge R_{\text{eff}}$, their stance absorbs the recoil impulse cleanly (`0` penalty). Every point by which allocated **Force ($F$)** falls short of $R_{\text{eff}}$ adds **`+1` to $\Delta_{\text{ext}}$** due to muzzle climb!

---

## 3. Ammunition Architecture & Internal/External Ballistics

Every cartridge in Satya is defined by **six physical and operational parameters** that drive both kinetic energy and weapon handling:

1. **Caliber ($d_{\text{cal}}$ in $\text{mm}$ / inches)**: The bore diameter and frontal cross-section of the projectile (e.g., `9×19mm`, `.45 ACP` [`11.43mm`], `5.56×45mm`, `6.5mm Creedmoor` [`6.72mm`], `7.62×39mm`, `7.62×51mm NATO`, `12-Gauge` [`18.5mm`]).
2. **Projectile Mass ($m_{\text{proj}}$ in grains $\text{gr}$)**: Mass of the bullet ($1\text{ gram} = 15.432\text{ grains}$; $1\text{ grain} = 0.0648\text{ grams}$). Combined with caliber, this defines **Sectional Density ($\text{SD} = \frac{m_{\text{proj}}}{7000 \cdot d_{\text{in}}^2}$)**—how much mass sits behind each square millimeter of frontal area to punch through armor and resist air drag.
3. **Gunpowder / Propellant Charge ($m_{\text{prop}}$ in grains $\text{gr}$)**: The mass of chemical propellant inside the cartridge case. Propellant grains provide the thermochemical energy that accelerates the bullet down the barrel—and also create the high-pressure gas jet that adds to recoil and muzzle blast!
4. **Bullet Profile & Construction**:
   - **Full Metal Jacket (`FMJ` / Ball)**: Standard lead/copper-alloy core in a gilding-metal jacket ($\epsilon_{\text{exp}} = 1.0$, standard penetration).
   - **Jacketed Hollow Point (`JHP` / Expanding)**: Mushrooms on soft-tissue impact to `1.5×–1.8×` caliber diameter (`+1` Caliber Crush $I_{\text{cal}}$, **`-2` Armor Penetration `Pen`**).
   - **Open-Tip Match Boat-Tail (`OTM` / Match)**: Aerodynamic boat-tail with precision jacket concentricity (`+1.0` Accuracy, extended energy retention $D_{\text{decay}}$).
   - **Armor-Piercing (`AP` — Tungsten-Carbide / Hardened Steel Penetrator)**: Sub-caliber dense core (`+2` to `+3` Armor Penetration `Pen`, **`-1` Soft-Tissue Crush $I_{\text{cal}}$** on clean through-and-through hits unless it yaws).
   - **Subsonic Heavy Load**: Reduced powder charge ($m_{\text{prop}}$) paired with a heavy projectile so muzzle velocity stays below Mach 1 (`< 340 m/s`), eliminating the supersonic ballistic crack when paired with a suppressor (`-25%` to `-40%` $E_k$).
   - **Overpressure (`+P` / `+P+` / Hybrid-Case High-Pressure)**: `+15–25%` higher powder charge/chamber pressure (`+1` $I_{\text{KE}}$ or `Pen`, **`+1` Recoil $R$**, **`-1` Reliability $\text{Rel}$** from accelerated extractor/parts wear).
5. **Ammunition Quality ($Q_{\text{ammo}}$)**:
   - **Salvaged / Corroded / Street Hand-Loads (`Q_ammo = -1.5`)**: Inconsistent powder weights and seated depths (`-1.5` Accuracy, `-1` Reliability, `+1` Wobble $W_{\text{yaw}}$).
   - **Bulk Steel-Cased / Military Surplus (`Q_ammo = -0.5`)**: Bimetal jacket, lacquered steel case (`-0.5` Accuracy, `-1` Reliability in tight chambers due to lower case obturation/extraction lubricity).
   - **Standard Commercial Brass / Mil-Spec Duty (`Q_ammo = 0.0`)**: Baseline (`0.0` Accuracy, `0` Reliability).
   - **Match-Grade / Precision Hand-Loaded (`Q_ammo = +1.0`)**: Extruded temperature-stable propellant weighed to $\pm 0.02\text{ gr}$ and concentric cases (`+1.0` Accuracy).
6. **Feed System & Capacity**: How ammunition is stored and presented to the breech (detailed in **Section 3.3**).

---

### 3.1 How Kinetic Energy ($E_k$) Is Calculated

The **Kinetic Energy at Impact ($E_k$, measured in Joules $\text{J}$)** is not a static lookup—it emerges from the cartridge's **bullet mass ($m_{\text{proj}}$)**, **gunpowder charge ($m_{\text{prop}}$)**, **barrel length & muzzle devices ($\eta_{\text{bbl}}$)**, **action gas seal ($\eta_{\text{action}}$)**, and **target distance ($d$)**:

$$E_k(d) = \underbrace{E_{\text{chem}}(m_{\text{proj}}, m_{\text{prop}})}_{\text{Cartridge Base Energy}} \times \underbrace{\eta_{\text{bbl}}(L_{\text{bbl}}, \text{Muzzle})}_{\text{Barrel & Muzzle Factor}} \times \underbrace{\eta_{\text{action}}}_{\text{Action Gas Factor}} \times \underbrace{e^{-d / D_{\text{decay}}}}_{\text{Aerodynamic Retention}}$$

#### 1. Cartridge Base Thermochemical Energy ($E_{\text{chem}}$)
Each grain of smokeless propellant releases $\approx 260\text{ J}$ of chemical energy. The fraction transferred into the projectile depends on the mass ratio between the bullet ($m_{\text{proj}}$ in $\text{gr}$) and the expanding powder gas ($m_{\text{prop}}$ in $\text{gr}$):

$$E_{\text{chem}} \approx 105 \cdot m_{\text{prop}} \cdot \left(\frac{m_{\text{proj}}}{m_{\text{proj}} + 2.2\,m_{\text{prop}}}\right) \quad [\text{in Joules}]$$

- *Why Both Bullet Mass and Powder Grains Matter*:
  - **`.45 ACP`** has a massive **`230 gr` bullet** but only **`5.6 gr` of powder**: the heavy slug extracts a high fraction of that small charge ($E_{\text{chem}} \approx 560\text{ J}$) at slow, subsonic velocity (`260 m/s`).
  - **`5.56×45mm NATO`** has a light **`62 gr` bullet** backed by **`27.0 gr` of powder**: the huge powder-to-bullet ratio drives the projectile to `~920 m/s` ($E_{\text{chem}} \approx 1,850\text{ J}$).
  - **`6.5mm Creedmoor`** pairs a long, sleek **`140 gr` bullet** with **`42.5 gr` of powder** ($E_{\text{chem}} \approx 3,300\text{ J}$ at `835 m/s`), while **`7.62×51mm NATO (.308)`** pairs a **`147–168 gr` bullet** with **`45.0 gr` of powder** ($E_{\text{chem}} \approx 3,500\text{ J}$).

#### 2. Barrel Length ($\eta_{\text{bbl}}$) & Muzzle Device Effects
Gunpowder requires barrel length ($L_{\text{bbl}}$) to burn completely and expand behind the bullet. Every cartridge class has an **Optimal Burn Reference Barrel Length ($L_{\text{opt}}$)**:
- **Pistol / SMG Calibers (`9mm`, `.45 ACP`)**: $L_{\text{opt}} = 12.0\text{ cm}$ (`4.7 in`)
- **Magnum Handgun / PDW (`5.7×28mm`, `.357 Mag`)**: $L_{\text{opt}} = 16.5\text{ cm}$ (`6.5 in`)
- **Intermediate Rifles & Shotguns (`5.56×45mm`, `7.62×39mm`, `12-Gauge`)**: $L_{\text{opt}} = 45.7\text{ cm}$ (`18.0 in`)
- **Full-Power Battle & Precision Rifles (`6.5mm Creedmoor`, `7.62×51mm`)**: $L_{\text{opt}} = 56.0\text{ cm}$ (`22.0 in`)
- **Magnum & Anti-Materiel Rifles (`.338 Lapua`, `.50 BMG`)**: $L_{\text{opt}} = 68.5\text{ cm}$ (`27.0 in`)

$$\eta_{\text{bbl}} = \text{clamp}\!\left(0.60,\; 1.15,\; 1.0 + 0.35 \ln\!\left(\frac{L_{\text{bbl}}}{L_{\text{opt}}}\right)\right) + \Delta\eta_{\text{muzzle}}$$

- **Short-Barrel Velocity Loss & Blast**: Cutting a `5.56×45mm` barrel down from `50.8 cm (20")` to a `26.2 cm (10.3")` CQB Mk18 drops $\eta_{\text{bbl}}$ from `1.04` down to **`0.81`** (`-22%` kinetic energy!) because unburned powder detonates outside the muzzle—imposing **`+2` Muzzle Flash / Indoor Concussion (`Tinnitus` risk)**!
- **Muzzle Attachments ($\Delta\eta_{\text{muzzle}}$ & Handling)**:
  - **Bare Crown / Thread Protector**: $\Delta\eta_{\text{muzzle}} = 0.00$, Recoil `+0`, Length `+0 cm`.
  - **Flash Hider / Birdcage**: $\Delta\eta_{\text{muzzle}} = 0.00$, Recoil **`-0.5`**, suppresses night-vision flash bloom, Length `+4 cm`.
  - **Muzzle Brake / Compensator**: Redirects propellant gas laterally/upward against baffle plates: $\Delta\eta_{\text{muzzle}} = -0.01$, **Recoil $R$ reduced by `-1` to `-2`**, Length `+5 cm`, **but** vents a violent overpressure wave sideways (`+2` `Tinnitus`/`Dazzled` hazard to anyone within `2m` of the muzzle indoors!).
  - **Sound Suppressor / Silencer**: Traps and cools expanding gases inside internal baffles: **$\Delta\eta_{\text{muzzle}} = +0.02$ ("Freebore Boost")**, **Recoil $R$ reduced by `-1`**, reduces gunshot hearing detection (`HEA`) by `-8` (`-12` with subsonic ammo) and eliminates indoor `Tinnitus`, **but** adds **`+16 to +20 cm` to Overall Length ($\text{OAL}$)**, adds **`+0.40 to +0.55 kg`** at the muzzle tip (shifting $\Delta_{\text{CoG}}$ forward by `+1` tier), and increases gas blowback/carbon fouling in Direct Impingement actions (**`-1` $\text{Rel}$** after `60` rounds unless using an adjustable gas block or flow-through suppressor)!

#### 3. Action Type Gas Efficiency ($\eta_{\text{action}}$)
Different firearm actions bleed off different amounts of chamber gas before the bullet exits the crown:

| Action Mechanism | Gas Seal Efficiency ($\eta_{\text{action}}$) | Physical Explanation |
| :--- | :---: | :--- |
| **Bolt-Action / Break-Action / Single-Shot** | **`1.00×` (`100%`)** | Rotating lugs lock the breech completely shut; **zero gas is bled off** to cycle an action. Delivers maximum velocity and kinetic energy! |
| **Single-Action (SA) / Striker / Pump-Action** | **`0.99×` (`99%`)** | Locked breech (tilting barrel or rotating slide lock) remains sealed during peak pressure; tiny energy fraction goes into unlocking recoil mass. |
| **Short-Stroke / Long-Stroke Gas Piston** | **`0.97×` (`97%`)** | Bleeds off `~3%` of high-pressure propellant gas through a barrel port to drive the piston and bolt carrier group. |
| **Direct Impingement (`DI`)** | **`0.97×` (`97%`)** | Pipes propellant gas down a narrow gas tube directly into the bolt carrier's internal expansion chamber. |
| **Simple / Delayed Blowback** | **`0.95×` (`95%`)** | Breech is held closed only by bolt inertia and spring/roller mechanical advantage, moving slightly rearward during pressure peak. |
| **Revolver (Standard Cylinder Gap)** | **`0.92×` (`92%`)** | Vents `~8%` of propellant gas sideways through the `0.15 mm` clearance gap between the revolving cylinder and barrel forcing cone *(cannot be suppressed unless gas-sealed)*. |

---

### 3.2 Master Ammunition Reference Table

| Cartridge / Caliber ($d_{\text{cal}}$) | Standard Bullet Mass ($m_{\text{proj}}$) | Powder Charge ($m_{\text{prop}}$) | Optimal Barrel ($L_{\text{opt}}$) | Reference Muzzle Velocity & Energy ($E_0$ at $L_{\text{opt}}$) | Energy Decay Scale ($D_{\text{decay}}$) | Base KE Index ($I_{\text{KE}}$) | Caliber Crush ($I_{\text{cal}}$) | Sectional Density Bonus ($\text{SD}_{\text{bonus}}$) | Typical Cost (`50 rds`) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **`9×19mm Parabellum`** *(9.0 mm)* | `124 gr` *(8.0 g)* | `5.2 gr` | `12.0 cm` *(4.7")* | `360 m/s` · **`520 J`** | `220 m` | **`3`** | **`2`** *(FMJ)* / **`3`** *(JHP)* | `+0` | `$20` |
| **`.45 ACP`** *(11.43 mm)* | `230 gr` *(14.9 g)* | `5.6 gr` | `12.7 cm` *(5.0")* | `260 m/s` · **`505 J`** *(Subsonic)* | `180 m` | **`3`** | **`3`** *(FMJ)* / **`4`** *(JHP)* | `-1` *(Fat/Slow)* | `$32` |
| **`5.7×28mm`** *(5.7 mm)* | `40 gr` *(2.6 g)* | `7.2 gr` | `16.0 cm` *(6.3")* | `620 m/s` · **`500 J`** | `280 m` | **`3`** | **`1`** | **`+2`** *(Spitzer)* | `$45` |
| **`.357 Magnum`** *(9.1 mm)* | `158 gr` *(10.2 g)* | `14.5 gr` | `16.5 cm` *(6.5")* | `435 m/s` · **`970 J`** | `260 m` | **`4`** | **`2`** *(FMJ)* / **`3`** *(JHP)* | `+1` | `$40` |
| **`5.56×45mm NATO`** *(5.56 mm)* | `62 gr` *(4.0 g)* | `27.0 gr` | `45.7 cm` *(18.0")* | `915 m/s` · **`1,680 J`** | `500 m` | **`5`** | **`1`** *(+1 Yaw)* | **`+2`** | `$38` |
| **`7.62×39mm`** *(7.62 mm)* | `123 gr` *(8.0 g)* | `25.5 gr` | `41.5 cm` *(16.3")* | `715 m/s` · **`2,040 J`** | `420 m` | **`5`** | **`2`** | **`+2`** | `$30` |
| **`6.5mm Creedmoor`** *(6.72 mm)* | `140 gr` *(9.1 g)* | `42.5 gr` | `56.0 cm` *(22.0")* | `835 m/s` · **`3,165 J`** | **`950 m`** *(High BC)* | **`6`** | **`1`** *(FMJ)* / **`2`** *(OTM)* | **`+3`** *(Ultra SD)* | `$75` |
| **`7.62×51mm NATO (.308)`** *(7.62 mm)* | `147 gr` *(9.5 g)* | `45.0 gr` | `56.0 cm` *(22.0")* | `840 m/s` · **`3,355 J`** | `750 m` | **`6`** | **`2`** | **`+2`** | `$55` |
| **`12-Gauge (2¾" 00 Buck / Slug)`** *(18.5 mm)* | `437 gr` *(28.3 g)* | `32.0 gr` | `47.0 cm` *(18.5")* | `475 m/s` · **`3,200 J`** | `80 m` *(Buck)* / `200 m` *(Slug)* | **`6`** *(Close)* | **`4`** | `-2` *(Buck)* / `+0` *(Slug)* | `$45` |
| **`.338 Lapua Magnum`** *(8.6 mm)* | `250 gr` *(16.2 g)* | `90.0 gr` | `68.5 cm` *(27.0")* | `900 m/s` · **`6,560 J`** | `1,300 m` | **`7`** | **`2`** | **`+3`** | `$220` |
| **`.50 BMG (12.7×99mm)`** *(12.7 mm)* | `660 gr` *(42.8 g)* | `235.0 gr` | `73.7 cm` *(29.0")* | `890 m/s` · **`16,950 J`** | `1,800 m` | **`9`** | **`3`** | **`+4`** | `$380` |

---

### 3.3 Feed Mechanisms, Capacity & Reload Dynamics

How a weapon stores and feeds cartridges directly affects its **Capacity**, **Reliability ($\text{Rel}_{\text{feed}}$)**, **Center of Gravity ($\Delta_{\text{CoG}}$)**, **Reload Tick Cost ($\text{TC}_{\text{reload}}$)**, and **Clear Jam Time ($\text{TC}_{\text{clear}}$)**:

| Feed Mechanism | Typical Capacity | Reliability Mod ($\text{Rel}_{\text{feed}}$) | Reload Time Cost (`Base TC`) | Operational Mechanics, Ergonomics & Tactical Trade-Offs |
| :--- | :---: | :---: | :---: | :--- |
| **1. Detachable Box Magazine** *(Single or Staggered Double-Stack)* | `7–20 rds` *(Pistol/BR)*<br/>`30–40 rds` *(Carbine)* | **`+0`** *(Standard)*<br/>`-1` *(Quad-stack `60-rd` casket)* | **`12 Ticks` (3.0s)** *(Speed drop)*<br/>**`16 Ticks` (4.0s)** *(Retain mag)* | Fastest tactical reload. Closed-bolt weapons hold **`+1` round in the chamber**. Weak/worn magazine follower springs or dented feed lips impose `-2 Rel` for that specific magazine. |
| **2. Drum / Helical Magazine** *(`50–100 rds`)* | `50–100 rds` | **`-2`** | **`20 Ticks` (5.0s)** | Sustained fire without reloading, **but** heavy clockwork spring tension and feed-tower friction reduce reliability (`-2 Rel`), add `+1.2–2.2 kg` loaded mass, shift $\Delta_{\text{CoG}}$ forward/downward (`+1` Base `TC` on Snap Shots), and rattle audibly (`+2` $\Delta_{\text{ext}}$ on Stealth). |
| **3. Belt-Fed** *(Disintegrating Link Box / Assault Pouch)* | `100–250 rds` | **`+0`** *(Braced)*<br/>**`-1`** *(Moving / Unbagged)* | **`32 Ticks` (8.0s)** *(Solo)*<br/>**`14 Ticks` (3.5s)** *(With Asst. Gunner)* | Ideal for sustained suppressive fire. Opening the feed tray cover, seating the first link on the pawl, and closing the cover takes `32 Ticks` solo (`8s`), or `14 Ticks` (`3.5s`) with a trained Assistant Gunner. |
| **4. Tubular Magazine** *(Under-Barrel / Stock Tube)* | `4–8 rds` *(+1 chamber)* | **`+0`** *(Immune to mag lip dents)* | **`6 Ticks` (1.5s)** *per shell* (`+4 Ticks` combat-load into open port) | Cannot swap a whole magazine at once—topped off **one shell at a time** between exchanges (`Tactical Topping-Off`), allowing instant ammo-type switching ("Slug Select Drill": `8 Ticks` to thumb a slug into the tube and rack). **Dynamic CoG**: Starts Front-Heavy when full, shifting toward Neutral as shells are fired! Pointed spitzer bullets cannot be loaded tip-to-primer in a tube without risking chain-detonation (uses flat-nose, buckshot, or slugs). |
| **5. Revolving Cylinder** *(Swing-Out or Loading Gate)* | `5–8 rds` | **`+1`** *(Zero feed ramp)* | **`8 Ticks` (2.0s)** *per round*<br/>**`14 Ticks` (3.5s)** *(Speedloader / Moon Clip)* | Each chamber is its own feed ramp—if a primer fails to ignite (dud), pulling a Double-Action trigger immediately indexes the next live chamber (`2 Ticks`, zero tap-rack needed!). High-pressure dirt inside the extractor star or bent crane rod requires `28 Ticks` to clear. |
| **6. Single-Shot / Internal Fixed Magazine** *(Break-Action, Bolt Internal Box, Stripper Clip)* | `1 rd` *(Single)*<br/>`5 rds` *(Internal Box)* | **`+1` to `+2`** | **`10 Ticks` (2.5s)** *(Single/Clip)*<br/>**`6 Ticks`/rd** *(Loose top-load)* | No detachable feed lips to bend or lose; ultra-reliable in mud/arctic cold. Internal box magazines can be charged `5` rounds at once via a stripper clip (`12 Ticks`) or topped off individually (`6 Ticks` per cartridge). |

---

## 4. Terminal Ballistics: How Kinetic Energy, Caliber & Inverse Accuracy Determine Damage

When a projectile strikes a target, **Satya** computes **Raw Ballistic Impact (`Raw Impact`)** and **Base Armor Penetration (`Pen`)** directly from three physical inputs:
1. **Impact Kinetic Energy ($E_k(d)$ $\to$ KE Index $I_{\text{KE}}$)**
2. **Projectile Caliber & Expansion ($d_{\text{cal}}$ $\to$ Caliber Crush Index $I_{\text{cal}}$)**
3. **Inverse of Accuracy ($\to$ Projectile Wobble / Yaw Factor $W_{\text{yaw}}$)**

### 4.1 The Three Components of Ballistic Damage

#### 1. Kinetic Energy Index ($I_{\text{KE}}$)
Kinetic energy at the moment of impact ($E_k(d)$ in Joules, after accounting for barrel length $\eta_{\text{bbl}}$, action type $\eta_{\text{action}}$, and range decay $e^{-d/D_{\text{decay}}}$) governs overall tissue disruption and armor-cracking force:

$$I_{\text{KE}} = \text{round}\!\left(1.85 \cdot \ln\!\left(1 + \frac{E_k(d)}{150\text{ J}}\right)\right)$$

| Impact Kinetic Energy $E_k(d)$ | KE Index ($I_{\text{KE}}$) | Ballistic Benchmark Examples |
| :---: | :---: | :--- |
| `< 180 J` | **`1`** | `.22 LR` pistol, spent long-range ricochet, sub-lethal beanbag. |
| `180 – 380 J` | **`2`** | `.380 ACP`, short-barrel `5.7×28mm`, `9mm` at `150m`. |
| `381 – 700 J` | **`3`** | Duty **`9×19mm`** (`520 J`), **`.45 ACP`** (`505 J`), `5.7×28mm` carbine (`500 J`). |
| `701 – 1,350 J` | **`4`** | **`.357 Magnum`** (`970 J`), `10mm Auto`, `5.56mm` from `10.3"` CQB barrel at `100m`. |
| `1,351 – 2,500 J` | **`5`** | **`5.56×45mm NATO`** (`1,680–1,800 J`), **`7.62×39mm`** (`2,040 J`). |
| `2,501 – 4,500 J` | **`6`** | **`6.5mm Creedmoor`** (`3,165 J`), **`7.62×51mm NATO`** (`3,355 J`), **`12-Gauge`** (`3,200 J`). |
| `4,501 – 9,000 J` | **`7`** | **`.338 Lapua Magnum`** (`6,560 J`), `.300 Norma Magnum`. |
| `9,001 – 14,000 J` | **`8`** | `.338 Lapua` AP point-blank / heavy anti-materiel at `800m`. |
| `> 14,000 J` | **`9`** | **`.50 BMG (12.7×99mm)`** (`16,950 J`). |

#### 2. Caliber Crush Index ($I_{\text{cal}}$)
While kinetic energy stretches tissue temporarily, **caliber ($d_{\text{cal}}$)** determines the cross-sectional diameter of the permanent crush channel (and spreads blunt impact over a wider surface area):

$$I_{\text{cal}} = \left\lfloor \frac{d_{\text{cal}} \times \epsilon_{\text{exp}}}{4.5\text{ mm}} \right\rfloor \quad \text{where } \epsilon_{\text{exp}} = \begin{cases} 1.00 & \text{FMJ / AP / Solid} \\ 1.55 & \text{JHP / Soft-Point Expanding (in unarmored flesh)} \end{cases}$$

- **`5.56mm / 5.7mm / 6.5mm` FMJ**: $I_{\text{cal}} = \mathbf{1}$ (narrow frontal area; relies on velocity/KE and yaw to cavitate).
- **`7.62mm / 9mm / .338` FMJ** *(or expanding `5.56mm/6.5mm` OTM)*: $I_{\text{cal}} = \mathbf{2}$.
- **`.45 ACP` (`11.43mm`) FMJ / `9mm` JHP / `.50 BMG` (`12.7mm`)**: $I_{\text{cal}} = \mathbf{3}$.
- **`.45 ACP` JHP / `12-Gauge` (`18.5mm` Bore)**: $I_{\text{cal}} = \mathbf{4}$.

#### 3. Inverse of Accuracy: Projectile Wobble & Erratic Wound Channels ($W_{\text{yaw}}$)
In real-world terminal ballistics, a bullet that flies with laser-tight gyroscopic stability (`High Accuracy`, $\text{Acc} \ge +2$) strikes nose-first at a $0^\circ$ angle of attack. If it is a non-expanding FMJ or AP bullet, it often drills a clean, narrow "icepick" channel straight through soft tissue before dumping its kinetic energy.

Conversely, a bullet fired from an **inaccurate weapon** ($\text{Acc} < 0$—due to loose combat tolerances, worn/short rifling, non-free-floated barrel whip, or poor-quality surplus ammo) exits the crown with significant **nutation and precession (wobble/yaw)**. Although this wobble makes the round harder to place on small targets at long range, **any wobbling round that hits a target strikes at an oblique angle of attack**—causing the bullet to immediately **tumble end-over-end (`keyholing`), deform, or tear a wide, jagged, erratic wound channel** through muscle, organs, and blood vessels!

$$\text{Wobble / Yaw Factor } (W_{\text{yaw}}) = \text{clamp}\!\big(0,\; 3,\; \max(0,\; 1 - \text{Acc}) + \Delta W_{\text{spitzer}}\big)$$

*(where $\Delta W_{\text{spitzer}} = +1$ for high-aspect-ratio spitzer bullets like `5.56×45mm` or `7.62×39mm M67` impacting at $> 700\text{ m/s}$).*

- **Mechanical Effect of $W_{\text{yaw}}$**:
  1. **Erratic Soft-Tissue Cavitation**: Adds **$+W_{\text{yaw}}$ directly to `Raw Ballistic Impact`** against unarmored targets (or when armor is penetrated), and if $W_{\text{yaw}} \ge 2$, increases the resulting **Hemorrhage Grade by `+1`** (up to Grade 4 Arterial)!
  2. **Sideways Armor Strike Penalty**: Because a wobbling bullet strikes hard armor slightly sideways rather than point-first, it suffers **$-\lfloor W_{\text{yaw}} / 2 \rfloor$ to Armor Penetration (`Pen`)**!

---

### 4.2 Unified Ballistic Impact, Penetration & Wound Tier Formulas

Combining all three terms yields the weapon's **Raw Ballistic Impact** and **Base Armor Penetration (`Pen`)**:

$$\text{Raw Ballistic Impact} = I_{\text{KE}} + I_{\text{cal}} + W_{\text{yaw}}$$

$$\text{Base Armor Penetration (Pen)} = \max\!\left(1,\; I_{\text{KE}} + \text{SD}_{\text{bonus}} + \Delta\text{Pen}_{\text{ammo}} - \left\lfloor \frac{I_{\text{cal}}}{2} \right\rfloor - \left\lfloor \frac{W_{\text{yaw}}}{2} \right\rfloor\right)$$

When a shot hits (scoring $\text{Net Successes } \Delta M \ge 1$ on the `2d20` cascade), the shooter's marksmanship placement adds **$\lfloor \Delta M / 2 \rfloor$** to both **Effective Penetration** and **Effective Trauma Magnitude**. After resolving Armor (`PR` vs. `BABT`; see **Section 7.2**), the net **Effective Trauma** maps directly to **[Anatomical Structural Wound Tiers (`Tiers 1–5`) and Hemorrhaging](../06-health-and-survival/health_and_trauma.md#31-structural-wound-severity-tiers-tiers-15)**:

| Net Effective Trauma | Structural Wound Tier | Aspect Ceiling Slashed | Wound Shock Added | Base Hemorrhage Grade *(if Penetrated; `+1` if $W_{\text{yaw}} \ge 2$)* |
| :---: | :--- | :---: | :---: | :--- |
| **`1 – 2`** | **Tier 1 — Superficial / Graze** | **`-2`** | `+0` | **Grade 1** *(Capillary: `0.25 Units/hr`)* |
| **`3 – 5`** | **Tier 2 — Moderate Trauma** | **`-5`** | **`+1` Tier** | **Grade 2** *(Venous: `0.5 Units/min`)* |
| **`6 – 8`** | **Tier 3 — Severe / Fracture** | **`-10` (Limb Disabled)** | **`+2` Tiers** | **Grade 3** *(Major Venous/Visceral: `1.0 Unit / 20 Ticks`)* |
| **`9 – 11`** | **Tier 4 — Critical / Maiming** | **Locked at `0`** | **`+3` Tiers** | **Grade 4** *(Arterial Severance: `1.0 Unit / 4 Ticks`)* |
| **`12+`** | **Tier 5 — Catastrophic / Lethal** | **Destroyed (`0`)** | **`+4` Tiers** | **Grade 4+** *(Immediate traumatic amputation / fatal organ rupture)* |

> [!IMPORTANT]
> **Why `9mm`, `.45 ACP`, `5.56mm`, `7.62×39mm`, and `6.5mm Creedmoor` Now Behave Completely Differently**:
> - **`.45 ACP` (`230 gr` JHP, `5.6 gr` powder)**: Low velocity (`260 m/s`, $I_{\text{KE}} = 3$), huge expanded frontal diameter ($I_{\text{cal}} = 4$) $\implies$ **`Raw Impact = 7–8` (`Tier 3 Severe Wound` on unarmored flesh!)**, **but `Base Pen = 1–2`**—stopped cold by even a light NIJ II/IIIA Kevlar vest (`PR 4`), transferring heavy blunt rib shock (`BABT`) instead!
> - **`5.7×28mm` (`40 gr` AP, `7.2 gr` powder)**: Same ~`500 J` energy ($I_{\text{KE}} = 3$), tiny frontal area ($I_{\text{cal}} = 1$), high sectional/AP bonus (`+3`) $\implies$ **`Base Pen = 6`** (punches right through a `PR 4` Soft Kevlar Vest!), but deals a narrower **`Raw Impact = 4–5`** (`Tier 2` wound unless placed into a vital box via high `Control`).
> - **Loose-Tolerance `7.62×39mm` AKM (`123 gr` FMJ, `25.5 gr` powder, $\text{Acc} = -1$)**: $I_{\text{KE}} = 5, I_{\text{cal}} = 2$, plus **Inverse Accuracy Wobble $W_{\text{yaw}} = 2$** $\implies$ **`Raw Impact = 9` (`Tier 4 Critical Maiming Wound` with `+1` erratic tumbling bleed!)** at close/medium range, while its `-1` Accuracy and `420m` drag decay make it much harder to hit small targets past `300m`.
> - **Free-Floated Bolt-Action `6.5mm Creedmoor` (`140 gr` Match OTM, `42.5 gr` powder, $\text{Acc} = +4$)**: Sealed bolt ($\eta_{\text{action}} = 1.00$), long `61 cm` match barrel ($\eta_{\text{bbl}} = 1.03$), ultra-low drag ($D_{\text{decay}} = 950\text{ m}$), zero wobble ($W_{\text{yaw}} = 0$) $\implies$ retains $I_{\text{KE}} = 6$ and **`Base Pen = 8`** all the way out to `600m+`, with `-4` to the shooter's Target Number ($\text{TN}_P$)!

---

## 5. Firearm Architecture, Accuracy ($\text{Acc}$) & Recoil Engineering

A weapon's **Intrinsic Accuracy ($\text{Acc} \in [-4, +4]$)** measures its mechanical precision (dispersion in **Minutes of Angle, $\text{MOA}$**, where $1.0\text{ MOA} \approx 2.9\text{ cm}$ at `100m`). In Satya, $\text{Acc}$ is not an arbitrary bonus—it is the sum of **seven physical engineering factors**:

$$\text{Acc} = \text{round}\!\Big(\Delta\text{Acc}_{\text{action}} + \Delta\text{Acc}_{\text{weight}} + \Delta\text{Acc}_{\text{CoG}} + \Delta\text{Acc}_{\text{bbl\_len}} + Q_{\text{bbl}} + \Delta\text{Acc}_{\text{mount}} + \tau_{\text{tol}} + Q_{\text{ammo}}\Big)$$

- **How $\text{Acc}$ Works in the `2d20` Cascade**:
  - On any **Deliberate Sighted / Aimed Shot** (or Braced Overwatch shot), the weapon's $\text{Acc}$ applies directly to the shooter's Known Tool Modifier ($\Delta_{\text{tool}} = -\text{Acc}$, so a **`+3` Accuracy** rifle lowers $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$ by **`-3`**, while a **`-2` Accuracy** street gun raises $\text{TN}_P$ by **`+2`**!).
  - On a rushed **Single Snap-Shot / Point-Shoot**, positive barrel/scope accuracy above `+1` cannot be exploited (capped at `+1`), though negative accuracy ($\text{Acc} < 0$) still penalizes the shot in full—and always feeds into the projectile's **Inverse Accuracy Wobble ($W_{\text{yaw}} = \max(0, 1 - \text{Acc})$)**!

---

### 5.1 Action Types: Cycle Mechanics, Gas Loss, Accuracy & Reliability

| Action Type | Gas Efficiency ($\eta_{\text{action}}$) | Accuracy Mod ($\Delta\text{Acc}_{\text{action}}$) | Reliability Mod ($\text{Rel}_{\text{action}}$) | Follow-Up / Cycling `TC` Mod | Mechanical & Tactical Engineering Profile |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **1. Bolt-Action** *(Rotating Mauser / 3-Lug Manual Breech)* | **`1.00×`** *(Zero gas loss)* | **`+1.5`** | **`+2`** | **`+4 Ticks` (1.0s)** *to cycle bolt between shots* | **The Precision & Power Benchmark**: Locked rigid breech loses zero gas; zero moving carrier parts during firing; short firing-pin lock time; camming lugs extract stuck cases with massive leverage. Cannot fire semi-auto bursts. |
| **2. Single-Action (`SA`)** *(Cocked Hammer Pistol / Revolver)* | **`0.99×`** *(Pistol)*<br/>**`0.92×`** *(Revolver)* | **`+1.0`** | **`+1`** *(Auto)*<br/>**`+2`** *(Revolver)* | `+0` *(Semi-Auto)*<br/>`+3 Ticks` *(Thumb-cock SA Revolver)* | Hammer is already cocked; pulling the trigger only releases the sear (`1.5–2.0 kg` crisp glass-rod break), minimizing sight disturbance at the moment of ignition. |
| **3. Striker-Fired** *(Partially Cocked Internal Striker + Tilting Barrel)* | **`0.99×`** | **`+0.0`** | **`+1`** | **`+0 Ticks`** | Consistent trigger pull from first to last shot, low bore axis (reduces muzzle flip `-1 R`), enclosed action seals out lint/holster debris, few moving parts. Slight striker take-up creep vs. a tuned Single-Action sear. |
| **4. Direct Impingement (`DI`)** *(Gas Tube into Internal Bolt Carrier Piston)* | **`0.97×`** | **`+0.5`** | **`+0`** *(`-1` when fouled/unlubed)* | **`+0 Ticks`** | **In-Line Precision Carbine**: Eliminates the operating rod and piston block above the barrel; light reciprocating mass travels dead-center down the bore axis (`+0.5` Acc, `-1` Recoil $R$). **Trade-Off**: Vents hot, carbon-laden propellant gas directly into the receiver and bolt group—drying out lubricant and dropping `Rel` faster under sustained fire! |
| **5. Short-Stroke Gas Piston** *(Tappet / Pusher Rod with Independent Return Spring)* | **`0.97×`** | **`+0.0`** | **`+1`** | **`+0 Ticks`** | A short gas piston strikes the bolt carrier and vents hot gas out the front gas block—keeping the receiver cool and clean even with a suppressor (`+1` Rel). Slightly heavier front-end and sharp piston tap (`0.0` Acc). |
| **6. Long-Stroke Gas Piston** *(Piston Rod Rigidly Attached to Heavy Bolt Carrier)* | **`0.97×`** | **`-0.5`** | **`+2`** | **`+0 Ticks`** | **Unstoppable Service Workhorse** (AK/PKM pattern): Massive, heavy bolt-carrier + piston assembly travels the full length of the receiver, smashing through carbon, ice, and mud (`+2` Rel). However, that heavy mass reciprocating above the bore causes harmonic barrel whip and rocks the sight picture (`-0.5` Acc, `+1` Recoil $R$). |
| **7. Pump-Action** *(Manual Sliding Forend + Dual Action Bars)* | **`0.99×`** | **`+0.0`** | **`+2`** | **`+2 Ticks` (0.5s)** *to rack slide between shots* | Powered by the shooter's arm rather than cartridge gas—so it reliably cycles low-pressure specialty rounds (beanbags, flares, subsonic, dragon's breath) that fail to cycle semi-auto shotguns! Under extreme stress (`Wavering/Broken` morale), a failed roll can cause "short-shucking" the slide. |
| **8. Blowback / Open-Bolt** *(Simple Inertia or Sear-Held Open Bolt)* | **`0.95×`** | **`-1.0`** *(Open-Bolt `-1.5`)* | **`+0`** *(Closed)*<br/>**`-1`** *(Open-Bolt / Auto-Sear)* | **`+0 Ticks`** | Cheap to manufacture and runs cool in open-bolt SMGs/LMGs, but heavy bolt slamming forward on trigger pull pitches the muzzle off-target before the primer even fires! |

---

### 5.2 Weight, Center of Gravity ($\Delta_{\text{CoG}}$), Length ($\text{OAL}$), Barrel Specs & Tolerances

#### 1. Weapon Weight ($M_{\text{wpn}}$) & Recoil Physics ($R$)
By conservation of momentum, the free recoil velocity of a firearm is inversely proportional to its mass ($M_{\text{wpn}}$ in $\text{kg}$), while the upward torque depends on the cartridge impulse ($m_{\text{proj}}, m_{\text{prop}}$):

$$\text{Base Weapon Recoil } (R_{\text{wpn}}) = \max\!\left(1,\; \text{round}\!\left(\frac{0.025 \cdot m_{\text{proj}} + 0.14 \cdot m_{\text{prop}}}{\sqrt{M_{\text{wpn}}}}\right) + \Delta R_{\text{CoG}} + \Delta R_{\text{muzzle}} + \Delta R_{\text{action}}\right)$$

- **How Weapon Weight Affects Accuracy ($\Delta\text{Acc}_{\text{weight}}$)**:
  - **Ultralight (`< 0.65 kg` Pistol or `< 2.4 kg` Rifle)**: Easy to carry all day, **but** violent recoil impulse causes sight jump and shooter flinch (**`-0.5` Accuracy**, high $R_{\text{wpn}}$).
  - **Standard Duty Weight (`0.7–1.2 kg` Pistol; `2.8–4.0 kg` Carbine/Rifle)**: **`+0.0` Accuracy**.
  - **Heavy Target / Bull-Barrel Chassis (`> 1.3 kg` Pistol; `4.5–7.5+ kg` Precision Rifle / LMG)**: Mass damps micro-tremors and recoil impulse (**`+0.5` to `+1.0` Accuracy when braced**, reduces $R_{\text{wpn}}$ by `-1` to `-2`), at the cost of Encumbrance (`kg`) and **`+1` Exertion Mark per `20 Ticks`** if fired standing unsupported!

#### 2. Center of Gravity Compared to the Chamber ($\Delta_{\text{CoG}}$)
Where a weapon's Center of Gravity sits relative to the chamber and the firing hand's fulcrum determines how the weapon pivots during target transitions (`Tempo`) and under recoil (`Control` / `Force`):

| Center of Gravity Profile ($\Delta_{\text{CoG}}$) | Typical Weapon Examples | Accuracy Mod ($\Delta\text{Acc}_{\text{CoG}}$) | Recoil Mod ($\Delta R_{\text{CoG}}$) | Snap-Shot `TC` Mod | Biomechanical & Handling Behavior |
| :--- | :--- | :---: | :---: | :---: | :--- |
| **Rear-Biased (`Bullpup`)** *(CoG `8–15 cm` **behind** firing grip / chamber)* | Steyr AUG, IWI Tavor X95, Desert Tech SRS | **`-0.5`** *(Long trigger linkage)* | `+0` | **`-1 Tick`** *(Fast pivot)* | Weight sits tucked into the shoulder pocket—effortless to hold one-handed or pivot around tight hallways (`-1` Snap `TC`, compact $\text{OAL}$ with full barrel length!). However, the light front end can feel "whippy," the long trigger transfer bar adds creep (`-0.5` Acc), and the chamber sits right beside the shooter's cheek/ear (`+2` Reload/Clear Jam `TC`). |
| **Chamber-Centered (`Neutral / In-Line`)** *(CoG at or within `4 cm` of chamber)* | AR-15/M4 Carbine, Duty Striker Pistols, MP5 | **`+0.5`** | **`-1`** *(In-line bore)* | **`+0 Ticks`** | Ideal ergonomic balance between the firing grip and support hand. Recoil pushes straight back into the shoulder/wrist rather than levering the muzzle upward. |
| **Moderate Forward-Biased** *(CoG `5–14 cm` **ahead** of chamber)* | Long-barrel Battle Rifles, Full-Size Revolvers, Half-Full Tubular Shotguns | **`+0.5`** *(Braced)*<br/>`+0.0` *(Off-hand)* | **`-1`** *(Muzzle weight damps climb)* | **`+1 Tick`** | Forward mass acts as a natural counterweight against muzzle flip (`-1 R`) and steadies deliberate tracking swings, at the cost of a slightly slower initial mount (`+1` Snap `TC`). |
| **Extreme Front-Heavy (`Overhung`)** *(CoG `15+ cm` ahead of chamber)* | Heavy Rifle + Suppressor + Laser Box, Full `8-rd` Tube Shotgun, `100-rd` Drum | **`-1.0`** *(Unsupported)*<br/>`+0.5` *(Bipod)* | **`-1`** | **`+2 Ticks`** | High rotational moment of inertia—slow to swing between targets (`+2` Snap `TC`), pulls the support arm down during off-hand aiming (`-1.0` Acc unsupported; `+1` Exertion Mark every `20 Ticks`), unless rested on a bipod/parapet! |

#### 3. Overall Length ($\text{OAL}$) vs. Barrel Length ($L_{\text{bbl}}$) in Close Quarters
- **Overall Length ($\text{OAL}$ in $\text{cm}$)**:
  - **Concealability**: Concealing a weapon under clothing (*Sleight of Hand* or *Concealed Draw*) applies a modifier based on $\text{OAL}$ and weight:
    - **Subcompact / Pocket (`OAL < 18 cm`)**: **`-2` $\Delta_{\text{ext}}$** (Easy to conceal in pocket/waistband; `-1` Draw `TC`).
    - **Full-Size Duty Handgun (`OAL 18–23 cm`)**: **`+0` $\Delta_{\text{ext}}$** (Standard IWB/shoulder holster).
    - **PDW / Folded Stock SMG (`OAL 35–55 cm`)**: **`+4` $\Delta_{\text{ext}}$** (Requires long coat, sling bag, or briefcase).
    - **Carbine / Bullpup Rifle (`OAL 65–85 cm`)**: **`+8` $\Delta_{\text{ext}}$** (Cannot be concealed on the body without bulky trench coat + stiffness penalty; fits in tennis bag/guitar case).
    - **Full-Length Rifle / Suppressed Sniper (`OAL > 90 cm`)**: **Impossible to conceal on body** (`+12` or requires long hard case).
  - **Close-Quarters Crowding (`Reach 1` Clinch & Tight Interior Corridors)**:
    - In Clinch Range (`Reach 1`) or confined spaces (vehicles, ventilation shafts, narrow stairwells), any weapon with **$\text{OAL} > 50\text{ cm}$** suffers **`+1` $\Delta_{\text{ext}}$ and `+1` `TC` per `20 cm` over `50 cm`** (because the long barrel must be canted or pulled back across the chest to clear doorways, and gives a close opponent a long lever to grab/deflect with `+2` to their Disarm `Control`!).
    - *Why Bullpups & Folding Stocks Exist*: A bullpup rifle packs a full **`42–51 cm` barrel** into a **`67–72 cm` Overall Length**—achieving full `5.56mm` / `7.62mm` muzzle velocity ($\eta_{\text{bbl}} = 1.00$) while suffering `20 cm` less CQB crowding than a conventional rifle!

#### 4. Barrel Length ($L_{\text{bbl}}$), Barrel Quality ($Q_{\text{bbl}}$) & Barrel Mounting ($\Delta\text{Acc}_{\text{mount}}$)

| Engineering Subsystem | Tier / Specification | Accuracy Mod | Mechanical & Physical Rationale |
| :--- | :--- | :---: | :--- |
| **Barrel Length ($L_{\text{bbl}}$)** | **Snub / Micro** (`< 9 cm / 3.5"`)<br/>**Compact / Pistol** (`9–25 cm / 3.5–10"`)<br/>**Standard Carbine** (`26–44 cm / 10.5–17"`)<br/>**Full Rifle** (`45–55 cm / 18–21.5"`)<br/>**Match Long Barrel** (`56–74 cm / 22–29"`) | **`-1.5`**<br/>**`-0.5`**<br/>**`+0.0`**<br/>**`+0.5`**<br/>**`+1.0`** | Longer barrels provide a longer sight radius, higher muzzle velocity (flatter trajectory, less wind drift time-of-flight), and lower uncorking pressure at the muzzle crown so escaping gas doesn't tip the bullet base as it leaves the rifling. |
| **Barrel Quality ($Q_{\text{bbl}}$)** | **Improvised / Worn / Pitted**<br/>**Commercial Button-Rifled / Stamped**<br/>**Mil-Spec Cold-Hammer Forged (`CHF`) Chrome-Lined**<br/>**Stainless 5R Match / Hand-Lapped**<br/>**Cut-Rifled / Carbon-Wrapped Cryo Match** | **`-2.0`**<br/>**`-0.5`**<br/>**`+0.0`**<br/>**`+1.0`**<br/>**`+1.5`** | *Chrome-lined CHF* barrels resist throat erosion and heat distortion during full-auto fire (`+1` durability), whereas *Hand-Lapped Stainless / Cut-Rifled Match* barrels have sub-micron bore uniformity (`< 0.5 MOA`). |
| **Barrel Mounting ($\Delta\text{Acc}_{\text{mount}}$)** | **Clamped / Non-Free-Floated** *(Handguard, sling swivel, or gas/mag tube touches barrel)* | **`-0.5`** *(`-1.5` when braced/slung!)* | Whenever the shooter rests a non-free-floated handguard on a wall, deploys a barrel-mounted bipod, or pulls tight on a sling, external force bends the barrel slightly and alters its harmonic whip node—shifting point of impact! |
| **Barrel Mounting ($\Delta\text{Acc}_{\text{mount}}$)** | **Free-Floating Barrel** *(Cantilevered from receiver barrel nut only; rigid chassis)* | **`+1.0`** | The handguard/chassis bridges over the barrel without touching it anywhere forward of the chamber. Sling tension, barricade pressure, and bipod loading transmit straight into the receiver, leaving barrel harmonics completely undisturbed! |

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
