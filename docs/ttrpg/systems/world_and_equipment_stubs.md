# Satya TTRPG Equipment, Armor, Stealth, Vehicles & World Systems (Reference Stubs)

This chapter provides the architectural specifications and mechanical stubs for essential RPG subsystems that complement Satya's core [`2d20` Watershed Cascade](../satya_d20_rules.md), [Combat Tactical Triad](../combat/combat.md), [Health & Trauma](health_and_trauma.md), and [Movement & Environment](movement_and_environment.md) rules.

Every subsystem in this chapter is designed to plug directly into Satya's existing mathematical primitives (**$\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$**, **$\delta_{\text{under}} = 6 + \Delta_{\text{int}}$**, **`Tempo / Control / Force`**, and the **Count-Up Action Clock**) without introducing disjointed mini-games or exception-based Feats.

---

## 1. Armor, Penetration & Behind-Armor Blunt Trauma (BABT)

In traditional RPGs, armor either makes a character harder to hit ("Armor Class") or subtracts a flat number from a generic Hit Point pool. In reality, armor makes a wearer slightly *easier* to hit (by adding bulk and Encumbrance), while acting as a **physical barrier** that stops penetration up to its material limit—yet still transfers kinetic momentum (**Behind-Armor Blunt Trauma**) into the wearer's ribs or skull!

### 1.1 Armor Profiles (`Coverage`, `Protection Rating`, `Blunt Damping` & `Mass`)

Every piece of protective gear is defined by **four mechanical ratings**:
1. **Anatomical Coverage**: Which [Hit Locations](health_and_trauma.md#1-hit-locations--specificity-of-damage-effects) are covered (e.g., a *Plate Carrier* covers `Chest [11–14]` and `Upper Abdomen [8–9]`, leaving `Pelvis [7]`, `Shoulders/Arms [15–18]`, `Legs [1–6]`, and `Neck/Head [19–20]` exposed; a *Ballistic Helmet* covers `Cranium [20]`).
2. **Penetration Resistance (`PR` by Damage Type)**:
   - **`PR_Edged`**: Resistance against slashes, cuts, and stabs.
   - **`PR_Ballistic`**: Resistance against bullets and shrapnel (aligned to NIJ / 2035 standards).
3. **Blunt Trauma Damping (`BTD`)**: How much kinetic shock the armor's trauma pad or rigid shell absorbs before the remainder crushes the tissue underneath.
4. **Mass (`kg`) & Ergonomic Factor**: Feeds directly into the character's [Encumbrance Tier](movement_and_environment.md#2-encumbrance--load-bearing-architecture) (`×0.8` for articulated armor).

| Armor / Protective Suit | Covered Zones | `PR_Edged` | `PR_Ballistic` | Blunt Damping (`BTD`) | Mass (`kg`) | Notes & Era Benchmark |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- |
| **Heavy Leather / Synth-Weave Jacket** | Torso, Arms | `2` | `1` | `1` | `2.5 kg` | Concealable street protection against knives and road rash. |
| **Concealable Soft Kevlar Vest (NIJ II/IIIA)** | Chest, Abdomen | `2` *(Slash)* / `1` *(Icepick)* | `4` *(Stops Pistol/SMG)* | `2` | `2.8 kg` | Worn under clothing; rifle rounds (`Pen >= 6`) punch straight through! |
| **Tactical Plate Carrier (Level IV Ceramic/UHMWPE)** | Vital Chest & Upper Back | `6` | `8` *(Stops Rifle)* | `5` | `8.5 kg` | Stops `5.56mm` / `7.62mm` rifle hits, transferring **Blunt Rib Shock** (`BABT`). |
| **Ballistic Combat Helmet + Visor** | Head (`Cranium` / `Eyes`) | `5` | `4` *(Visor `3`)* | `4` | `1.6 kg` | Protects against shrapnel, pistol rounds, and blunt concussion. |
| **Full Riot / Breacher Suit** | Full Body | `6` | `5` | `6` | `16.0 kg` | High blunt/edged protection; `+6` Heat Retention penalty. |
| **2035 Valkyrie / Recon Exo-Carapace** | Full Body (Sealed) | `8` | `9` | `8` | `18.5 kg` *(`-20 kg` Exo Offset)* | Powered myomer frame offsets its own weight while battery is active. |

### 1.2 Resolving Armor vs. Incoming Hits (`Penetration` vs. `BABT`)

When an attack hits an armored Hit Location:
1. **Called Shots to Bypass Armor**: An attacker who sees the target's armor can use **`Control`** to target an unarmored gap (e.g., Thigh/Femoral `+2`, Neck `+8`, or Pelvis `+4`; see [Hit Locations](health_and_trauma.md#11-determining-hit-location-1d20-roll-vs-control-called-shots)).
2. **If Struck on an Armored Zone**: Compare the weapon's **Penetration Rating (`Pen`)** (plus `+1` per `2` net cascade successes) against the armor's **`PR`**:
   - **Case A — Armor Stops Penetration ($\text{Pen} \le \text{PR}$)**:
     The blade or bullet does **not** perforate the skin (**Zero Bleed**). However, the wearer suffers **Behind-Armor Blunt Trauma (`BABT`)** equal to:
     $$\text{Blunt Trauma} = \max(0,\; \text{Raw Strike/Bullet Impact} - \text{BTD})$$
     *(For example, taking a `7.62mm` rifle round in a Level IV Ceramic Plate stops the bullet from piercing the heart, but transfers `Tier 1–2` Blunt Chest Trauma—cracking a rib, knocking the wind out of the wearer for `+1` `wound_shock_index`, and testing their `Root` against knockdown!)*
   - **Case B — Armor Overmatched ($\text{Pen} > \text{PR}$)**:
     The projectile or blade punches through the armor with reduced energy ($\text{Effective Trauma} = \text{Raw Impact} - \text{PR}$), inflicting a penetrating **Structural Wound & Hemorrhage** and degrading the armor plate's `PR` in that zone by **`-1` to `-2`** (ceramic plate cracking).

---

## 2. Weapon & Equipment Reliability, Maintenance & Malfunctions

Modern firearms, cyberdecks, drones, and exo-rigs are precision machines that can foul, overheat, or jam when neglected or pushed to their limits.

### 2.1 Weapon & Tool Reliability Ratings (`Rel`)

Every mechanical or electronic device has a **Reliability Rating (`Rel`)** from **`Rel +4` (Legendary / Over-Engineered)** to **`Rel -4` (Improvised / jury-rigged / worn out)**:
- **Rugged Service Weapons** (e.g., stock AK-pattern rifle, factory Glock, solid crowbar): **`Rel +2`**
- **Standard Precision / Issue Gear** (e.g., standard carbine, trauma kit, commercial drone): **`Rel 0`**
- **Finicky / High-Tune / Modified Gear** (e.g., sear-modified machine pistol with drum mag, experimental prototype, unmaintained salvage): **`Rel -2` to `-4`**

### 2.2 When Malfunctions Trigger (Integration with `2d20` Botch Rules)

Satya does **not** require a separate "jam check" roll on every shot. Instead, malfunctions emerge naturally from the [`2d20` Cascade Outcome Tiers](../satya_d20_rules.md#2-signed-outcome-spectrum--n-to-n-botch-rules--outcometier):
1. **On `SuccessWithComplication` (`OutcomeTier::PartialSuccess` — `total_successes >= 1` AND `raw_botches >= 1`)**:
   - The shot or task **succeeds**, **but** if $\text{raw\_botches} > \text{Rel}$, the equipment suffers a **Minor Malfunction / Strain** *after* the action resolves (e.g., a firearm fires the hitting round but fails to eject the spent casing—a **Stovepipe / Failure-to-Feed** requiring an `8-tick` `"Tap, Rack, Bang"` clearance before it can fire again; or a lockpick bends, dropping its Tool Tier by `-1`).
2. **On `BotchFailure` (`OutcomeTier::CriticalFailure` — `total_successes == 0` AND `effective_botches >= 2`)**:
   - If $\text{effective\_botches} + \text{Environmental Mud/Fouling} > 2 + \text{Rel}$, the weapon suffers a **Major Malfunction / Double-Feed** (`20–32 ticks` to strip and clear) or runs dry on an uncounted magazine!
3. **Preventative Maintenance**: Spending `30 minutes` of Downtime with a cleaning kit and *Armorer & Gunsmithing* (`DEX/ANA`) restores fouled reliability to baseline and grants **`+1` `Rel`** for the next combat scene.

---

## 3. Stealth, Concealment, Illumination & Multi-Sensory Detection

Because Satya models **six distinct Sensory Characteristics (`SIG`, `HEA`, `TAC`, `OLF`, `TAS`, `ITU`)** rather than a single generic "Perception" stat, stealth and detection are resolved as an asymmetric **Multi-Channel Sensory Contest**.

### 3.1 Resolving Stealth vs. Detection (`2d20` Opposed Cascade)

- **The Infiltrator** rolls **Stealth & Concealment (`KIN / FOC`)** (or *Camouflage & Concealment* `INS/CRE` if stationary in a hide site, or *Shadowing & Urban Tail* `INS/FOC` in a crowd).
  - **Movement Gait & Encumbrance Modifiers**: Moving at a **Stealth Creep** (`0.34 m/Tick`) imposes `+0` $\Delta_{\text{ext}}$; moving at a **Combat Glide** imposes `+4` $\Delta_{\text{ext}}$; **Running** imposes `+8` $\Delta_{\text{ext}}$. Each **Encumbrance Tier** above Tier 1 adds **`+2` $\Delta_{\text{ext}}$** (gear rattle/footfall weight).
- **The Observer / Sentry** rolls **Observation & Awareness** using the **specific sense** suited to the environment:
  - **Visual Channel (`SIG / FOC`)**: Governed by **Illumination**, **Concealment/Cover**, and **Target Motion**.
  - **Auditory Channel (`HEA / FOC`)**: Governed by **Ambient Noise**, **Surface Material** (gravel/broken glass vs. concrete), and **Weapon Suppression**.
  - **Olfactory Channel (`OLF / INS`)**: Detects smoke, fuel, blood, sweat, or chemical residues in enclosed spaces (or K9/animal trackers).
  - **Intuitive Channel (`ITU / INS`)**: Subconscious "sixth sense" when being watched by a sniper or stalked in an eerie environment (can alert the target to drop behind cover even before `SIG` or `HEA` pinpoints the attacker's coordinates!).

### 3.2 Illumination, Visibility & Acoustic Modifiers ($\Delta_{\text{ext}}$)

| Environmental Condition | Visual Check (`SIG`) $\Delta_{\text{ext}}$ | Visual Stealth Bonus ($\Delta_{\text{ext}}$) | Countermeasures & Sensor Interaction |
| :--- | :---: | :---: | :--- |
| **Bright Daylight / Well-Lit Room** | `+0` | `+4` *(Harder to hide)* | Requires solid cover or urban crowd blending. |
| **Dusk / Twilight / Urban Streetlights / Shadows** | `+2` | `-2` | Flashlights create a cone of `+0` light but reveal the holder (`-6` to spot holder). |
| **Moonlight / Starlight / Power Outage Indoors** | `+6` | `-6` | **Gen-3+ Night Vision (`NVG`)** reduces visual penalty to `+1` (but narrows FOV, `-2` `OODA`). |
| **Dense Fog / Heavy Rain / Thick Smoke** | `+8` | `-8` | **Thermal / FLIR Optics** cut through smoke/darkness (`+0`), unless target wears thermal-baffled cloak. |
| **Pitch Black (Subterranean / Windowless Vault)** | **Impossible (`Blinded`)** | **Auto-Concealed vs. `SIG`** | Observer must rely on **Hearing (`HEA`)**, **Tactile (`TAC`)**, or active **IR / LiDAR**. |

---

## 4. Vehicles, High-Speed Chases & Crash / G-Force Physics (Stub)

Satya's **Tactical Triad (`Tempo`, `Control`, `Force`)** scales seamlessly from close-quarters foot combat to **vehicular pursuits, aerial dogfights, and orbital docking**:

1. **The Vehicle as a Physical Envelope**:
   - A vehicle provides its own mechanical **Envelope Ceilings**: **Top Speed / Acceleration (`Vehicle Tempo Cap`)**, **Handling & Braking (`Vehicle Control Cap`)**, and **Chassis Mass & Armor (`Vehicle Force / Root Cap`)**.
   - The driver/pilot fills this envelope using their **Driving / Piloting Skill ($N$)** and **`SPD / KIN`** (or **`DEX / FOC`** for remote drones), bounded by the vehicle's ceilings!
2. **Tactical Triad in a Chase or Dogfight**:
   - **Tempo ($T$)**: Throttle commitment—closing or opening the **Chase Range Band** (` bumper-to-bumper` $\leftrightarrow$ `Close [15m]` $\leftrightarrow$ `Medium [50m]` $\leftrightarrow$ `Long [150m+]` $\leftrightarrow$ `Lost Visual`).
   - **Control ($C$)**: Steering precision—threading through oncoming traffic, drifting a hairpin corner without spinning out, holding a steady platform for passengers firing from windows, and evading PIT maneuvers.
   - **Force ($F$)**: Ramming, PIT maneuvers, sideswiping, and holding the road (`Vehicle Root`) against collisions or crosswinds.
3. **Collision Kinetic Trauma & High-G Maneuvers**:
   - **Collisions**: Impact severity scales with relative velocity ($\Delta v$) and mass ratio ($m_1 / m_2$), inflicting **Blunt Trauma** across multiple random Hit Locations (mitigated by seatbelts, airbags, or crash harnesses).
   - **High-G Blackout (`G-LOC`)**: In high-performance aircraft or orbital burn maneuvers exceeding `5G+`, crew members must roll **`HLT / FIT`** (assisted by anti-G flight suits) or suffer **Tunnel Vision (`Dazzled`)** followed by **`G-LOC` (`Unconscious`)**.

---

## 5. Economy, Wealth, Upkeep (`SQM`), `[Access]` & Heat (Stub)

Rather than forcing players to track every cup of coffee while ignoring how a street operative acquires a military-grade anti-materiel rifle, Satya uses a two-tier economic model anchored by the character's Lifepath **Socioeconomic / Material Tier (`SQM`)** (see [Skill Design](../skill-design.md#5-chronological-lifepath-character-creation)):

### 5.1 Socioeconomic Tier (`SQM 1–10`) & Lifestyle Upkeep

| `SQM` Tier | Socioeconomic Bracket | Monthly Liquid Baseline (`2032 Credits / USD`) | Routine Purchases (No Roll) | Lifestyle & Safe-House Quality |
| :---: | :--- | :---: | :--- | :--- |
| **`1–2`** | **Destitute / Unhoused / Off-Grid Squat** | `< $500 / mo` | Street food, transit token | Exposed to street hazards; starts days at `+1` $\Delta_{\text{int}}$ unless shelter found. |
| **`3–4`** | **Working Class / Hustler** | `$2,000 – $4,000 / mo` | Basic tools, civilian clothes, burner phone | Cramped apartment, shared garage, older economy vehicle. |
| **`5–6`** | **Middle Class / Salaried Professional** | `$6,000 – $12,000 / mo` | Commercial electronics, legal firearms, clinic visits | Secure apartment, reliable sedan/SUV, clean credit identity. |
| **`7–8`** | **Affluent / Corporate Executive / Syndicate Boss** | `$25,000 – $100,000 / mo` | High-end cyberdecks, private charters, bribes | Gated penthouse, biometric security, private retainers/lawyers. |
| **`9–10`** | **Ultra-High Net Worth / Sovereign Oligarch** | `$500,000+ / mo` | Private security teams, orbital berths, shell corps | Compound, private VTOL/yacht, diplomatic/corporate immunity channels. |

### 5.2 Black-Market Procurement (`[Access]` Gates) & Law Enforcement **Heat**

1. **Legality & `[Access]` Gates**:
   - Money alone cannot buy restricted military ordnance, zero-day exploits, or Schedule-I combat stims at a retail store. Restricted items have an **`[Access]` Gate** (`Class I: Licensed`, `Class II: Illicit / Street`, `Class III: Syndicate / Paramilitary`, `Class IV: State Secret / Black-Ops`).
   - To locate and purchase a restricted item, the buyer must pass a **Streetwise & Underground Contacts (`INS/ELO`)** or **High Society / Corporate Networking (`PRE/ELO`)** cascade check with $\Delta_{\text{ext}}$ set by the item's `[Access]` class.
2. **Operational `Heat` & Digital Footprint (`0–10` Scale)**:
   - Leaving shell casings, biometric DNA, unlooped CCTV footage, loud unsuppressed gunfights, or dead law-enforcement officers generates **Operational `Heat`** in that jurisdiction:
     - **`Heat 1–2` (*Local Beat Inquiry*)**: Routine patrol awareness; basic disguise or alternate ID bypasses scrutiny.
     - **`Heat 3–5` (*Detective Task Force & APB*)**: Facial recognition alerts on public transit/cameras (`+4` $\Delta_{\text{ext}}$ to travel openly); known safe-houses surveilled.
     - **`Heat 6–8` (*Federal / Corporate Strike Team*)**: Active manhunt, frozen bank accounts (`SQM` temporarily locked to cash-in-hand), tactical SWAT/SecOps response within minutes of a spotted signature.
     - **`Heat 9–10` (*Burned / Most Wanted*)**: Shoot-on-sight or high-priority rendition; requires burning identity (*Forgery & Counterfeiting* `DEX/ANA`) and relocating out of theater.

---

## 6. Downtime, Training, Research & Extended Project Clocks (Stub)

When characters are between active missions—recovering from surgery, engineering a custom piece of hardware, decrypting an archive, or studying to raise a Skill Level ($N$)—Satya resolves progress using **Extended Project Cascades**:

1. **Extended Project Progress Clock**:
   - The GM sets a **Project Complexity Requirement** in **Total Cascade Successes Needed** (e.g., `10 Successes` to fabricate a custom drone frame, or `15 Successes` to trace a shell corporation across three offshore jurisdictions) and a **Time Interval per Roll** (`1 Hour`, `1 Day`, or `1 Week`).
   - At each interval, the character rolls their governing skill (`2d20` cascade) and adds their **`signed_margin`** to the Project Clock:
     - Positive successes (`+1` to `+N`) advance the project.
     - A **`SuccessWithComplication`** advances the project, but consumes extra raw materials, money, or leaves a forensic trace (`+1 Heat`).
     - A **`BotchFailure` (`< 0`)** sets progress back by the botch margin or damages a component.
2. **Post-Lifepath Skill Advancement (In-Campaign Training)**:
   - Improving an existing Skill from Level $N$ to $N+1$ (or learning a new `N = 1` skill) during a campaign uses the exact same **Exponential Study Curve** and **Discipline (`DIS + Mental`) Advancement Check** defined in [Skill System & Lifepath Architecture](../skill-design.md).
   - Combat, crisis, and high-stakes field experience count as **accelerated study hours** whenever a character succeeds (or survives a `SuccessWithComplication`) on a Challenging ($\Delta_{\text{ext}} \ge +4$) task using that skill!
