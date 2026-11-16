# Guide 05: Close-Quarters Melee & The Tactical Triad (`Tempo · Control · Force`)

> **Reference Chapters**: [Combat Architecture, Tactical Triad & Pools](../ttrpg/05-combat/tactical_triad_and_pools.md) · [Melee Combat & Close-Quarters Tactics](../ttrpg/05-combat/melee_combat.md)

---

## 1. Scene Opening: Nose-to-Nose in the Concrete Stairwell

Linnea clutches the cold-storage cartridge against her chest as the crew pushes through the stairwell fire door toward the underground garage.

Halfway down the concrete landing stands **Kovacs**, a broad-shouldered *Translucent* night-shift supervisor (`91 kg`), blocking the iron handrail with a heavy **expandable steel baton (`Reach 2`)** drawn at his side. He sees three silhouettes in the stairwell red glow and slams his hand onto the security lock behind him.

Sam immediately reaches for Mateo's holster. "I pull my `9mm` and shoot him!"

"Hold on," Elena whispers urgently, pointing to the concrete ceiling. "Fire an unsuppressed gunshot inside an echo-chamber concrete stairwell and every SecOps unit on the block hears it—plus your `Heat 2` jumps straight to a SWAT manhunt!"

"Good catch," Maya nods. "And Kovacs isn't pulling a pistol either—he wants to keep things quiet until backup arrives, or drop Mateo cleanly. Sam, let's learn *Satya*'s **Tactical Triad (`Tempo`, `Control`, `Force`)** right here in close quarters."

---

## 2. Demystifying "Envelope & Fill" (Setup Takes 60 Seconds)

Sam blinks at the combat section of his sheet. "Wait—do I have to calculate `P_base` from scratch before every single punch?"

"No!" Maya laughs. "You compute your **Envelope** *once* at Session Zero and write it at the top of your scratchpad. It only changes if you get tired or wounded."

Let's watch Sam verify **Mateo Vance's** Unarmed Strike Envelope (*Boxing, Kickboxing & Striking* `N = 3`, governed by **`SPD / KIN`**):

1. **Skill Controller ($C_{\text{skill}}$)**:
   $$C_{\text{skill}} = \left\lfloor \frac{\text{SPD } 14 + \text{KIN } 12}{2} \right\rfloor = \mathbf{13}$$
2. **Combat Readiness ($C_{\text{combat}}$)**: Average $C_{\text{skill}}$ with Mateo's **`OODA` (`12`)**:
   $$C_{\text{combat}} = \left\lfloor \frac{13 + 12}{2} \right\rfloor = \mathbf{12}$$
3. **Base Tactical Pool ($P_{\text{base}}$)** ("Your **Fill**") & **Neutral Baseline ($B$)**:
   $$P_{\text{base}} = C_{\text{combat}} + (3 \times N) = 12 + (3 \times 3) = \mathbf{21\text{ points}}$$
   $$B = \left\lfloor \frac{P_{\text{base}}}{3} \right\rfloor = \left\lfloor \frac{21}{3} \right\rfloor = \mathbf{7\text{ per aspect}}$$
   *(At fresh baseline, `fatigue_level = 0`, `wound_shock = 0`, `Steady` morale $\implies$ **$P_{\text{eff}} = 21$**).*
4. **Aspect Ceilings** ("Your **Envelope**"):
   - **Tempo Cap ($T_{\max}$)**: `SPD` = **`14`**
   - **Control Cap ($C_{\max}$)**: `max(KIN 12, DEX 13)` = **`13`**
   - **Force Cap ($F_{\max}$)**: `FOR` = **`11`**
5. **Derived Physical Benchmarks**:
   - **Striking Power ($\text{POW}$)**: $(\text{FOR } 11 \times \text{SPD } 14) / 10 = \mathbf{15.4}$
   - **Rooted Momentum ($\text{MMF}$)**: $(77\text{ kg} / 70\text{ kg}) \times 14 = \mathbf{15.4}$
   - **Fatigue Threshold ($\text{FT}$)**: $\text{FIT } 12 + \lfloor \text{DIS } 11 / 2 \rfloor = \mathbf{17}$

Behind her screen, Maya writes down **Kovacs** (*Riot Batons & Impact Weapons* `N = 2`, `FOR 14 / KIN 11 -> C_skill = 12`, `OODA 10 -> C_combat = 11`, **`P_base = 17`**, **`B = 5`**, Ceilings **`T_max 10 / C_max 11 / F_max 14`**, `POW = 14.0`, `MMF = 13.0`, `FT = 16`).

### 2.1 Technique (*What* You Do) vs. Triad (*How* You Do It)

"Notice why *Satya* doesn't have forty confusing feats like *Power Attack*, *Defensive Expertise*, or *Flurry of Blows*," Maya explains. "Every exchange splits into **two clean choices**:

1. **Pick a Technique (`CombatTechniqueId`)**: *What* biomechanics you use (`Punch`, `Kick`, `Elbow`, `Swing`, `Thrust`, `Parry`, `Block`, `Grab`, `Throw`, `Dodge`, `Root`). That sets your `Reach`, `Base TC`, and `POW` multiplier.
2. **Split Your `21` Pool Points Across `T / C / F`**: *How* you throw it!
   - Throw a **`Punch`** (`Base TC 7`, `1.00× POW`) with **`11 Tempo / 8 Control / 2 Force` (`TC`)** and it becomes a lightning-fast, guarded range-finding **jab**!
   - Throw that *exact same* **`Punch`** with **`3 Tempo / 4 Control / 11 Force` (`Pure F`)** and it becomes a wide-open, bone-rattling **haymaker**!"

> [!TIP]
> **GM Coaching — Secret Simultaneous Allocation**
> In a duel, both the player and the GM privately scribble three numbers (`T / C / F`) on scrap paper (or click three sliders in the CARPG) before flipping them over on the Tick clock! That single habit creates thrilling mind-games: *"Is he going all-out Force so I can parry-trap him, or throwing a fast jab to interrupt my wind-up?"*

---

## 3. Four Stairwell Exchanges (Every Melee Rule in Action)

Maya drops the red bead (`Mateo`: `OODA 12 -> Tick 3`) and a black bead (`Kovacs`: `OODA 10 -> Tick 5`) onto the paper Tick strip. Linnea and Amara hold the upper stair landing to keep clear of Kovacs's baton arc.

### 3.1 Exchange 1: Interrupting a Telegraphed Wind-Up on the Clock

- **Tick 3 (Mateo)**: Sam steps down two concrete stairs into engagement edge (`Reach 2`, baton distance) and sizes up Kovacs (`Probe` / ready step: **`3 Ticks`**, moving Mateo's bead to **`Tick 6`**).
- **Tick 5 (Kovacs — Goes Big Early!)**: Kovacs sees an "alley punk" stepping down toward him and tries to end the fight in one brutal baton crack!
  - Maya secretly writes Kovacs's allocation for an armed **`Swing`** (`Base TC = Baton 8 + 1 Swing = 9`, `1.25× POW`, **`-1 Guard`**):
    **`2 Tempo / 4 Control / 11 Force`** (`2 + 4 + 11 = 17`).
  - Look at the Tick math! Kovacs dumped `Tempo` down to `2` (`-3` below his neutral baseline `B = 5`):
    $$\text{Effective TC} = 9 - (2 - 5) = 9 + 3 = \mathbf{12\text{ Ticks}}$$
  - Kovacs begins pulling his heavy steel baton way back over his shoulder on `Tick 5`—**his blow won't actually land until `Tick 5 + 12 = Tick 17`!** Maya pushes the black bead to `17`.

- **Tick 6 (Mateo — Interrupts Mid-Wind-Up!)**:
  - Sam's eyes go wide when Maya announces `Tick 17`. "Wait—Kovacs won't finish swinging for another *eleven ticks* (`2.75 seconds`)?! And he only put `4` in Control (`-1` for `Swing` = `3` effective Guard)!"
  - "Exactly!" Maya grins. "Because he telegraphed a massive haymaker, Mateo's clock lands on **`Tick 6`** while Kovacs's arm is still cocked back behind his ear! What do you commit?"
  - Sam scribbles **`11 Tempo / 8 Control / 2 Force`** (`11 + 8 + 2 = 21`) on a fast **`Punch` (`Jab`, `Base TC 7`, `1.00× POW`)** stepping cleanly into `Reach 1`!
  - Compute Mateo's `Effective TC` (baseline `B = 7`):
    $$\text{Effective TC} = \max\big(2,\; 7 - (11 - 7)\big) = 7 - 4 = \mathbf{3\text{ Ticks}}\text{ (`0.75s`)!}$$
  - **Resolution on `Tick 6 + 3 = Tick 9` (`8` Ticks BEFORE `Tick 17`!)**:
    - Control Differential: Mateo's `C_att = 8` vs. Kovacs's exposed `C_def = 3` (`4 - 1 Swing`) gives Mateo **`-5` $\Delta_{\text{ext}}$** (`TN_GM = 21 - (-5) = 26`, though stepping inside a `Reach 2` baton adds `+2` Reach step = net `-3 -> TN_GM = 24`)!
    - Sam rolls `3` pairs (`C_skill = 13 -> TN_P = 31 - 13 = 18`): `13 + 9 = 22 > 18` (`+6`), `10 + 7 + 6 = 23 > 18` (`+6`), `8 + 7 + 6 = 21 > 18` -> **`S_P = 3` Successes!** Maya rolls `17 <= 24` (`S_GM = 0`).
    - **Result (`ExtraordinarySuccess`, `ΔM = +3`)**: Because Sam only put `2` into `Force`, Strike Impact is light:
      $$\text{Strike Impact} = 15.4 \times 1.00 \times \left(\frac{2}{7}\right) = \mathbf{4.4}$$
      *(Plus `+1` for `ΔM = 3`: snaps Kovacs's head back into the concrete wall for a **`Tier 2` Facial Bruise / `Stunned` Check**—interrupting Kovacs's telegraphed swing completely and bumping the black bead to `Tick 14` with `Wound Shock +1 -> P_eff = 15`!)*

---

### 3.2 Exchange 2: `Parry` & Seizing the Initiative (`C_def > C_att`)

"Whoa, that felt just like an actual boxing match!" Sam grins.

"Wait until you see what happens when the heavier guy tries to bully back," Maya warns.

- **Tick 14 (Kovacs — Reckless `TF` Blitz)**: Embarrassed and bleeding from a split lip (`Wound Shock 1 -> P_eff = 15`, `B = 5`, `action_time_factor = 1.25`), Kovacs lunges straight down the steps with a desperate **`Tempo + Force` (`TF`) `Swing`** (`Base TC = 9 × 1.25 ≈ 11` Ticks):
  **`6 Tempo / 3 Control / 6 Force`** (`6 + 3 + 6 = 15`; `11 - (6 - 5) = 10 Ticks` -> lands `Tick 24`, with effective Attack Control `3 - 1 Swing = 2`!).
- **Mateo (`Tick 9` $\to$ Ready / Counter on `Tick 14`)**: Sam sees Kovacs lunging straight down the stairs with his elbow flying wide (`Low Control = 2`). Instead of trading punches with a `91 kg` guard, Sam picks an **Intercept `Parry`** (`Base TC 6`, **`+3 Guard`**, **`2×` Seize-the-Initiative ticks**!):
  Sam allocates **`6 Tempo / 12 Control / 3 Force`** (`6 + 12 + 3 = 21`).
  - Mateo's Defense Control = `12 + 3 (Parry) =` **`15`**!
- **Resolution at `Tick 24`**:
  - Kovacs (`N = 2`, `C = 12 -> TN_P = 19`) suffers a brutal **`+13` Control penalty** (`15 - 2 = +13 -> TN_GM = 21 - 13 = 8` on Maya's defense/parry, or opposed `S_P = 0` vs Mateo's `S_def = 3`!). Kovacs whiffs completely as Mateo slaps the forearm inside the baton's handle and redirects Kovacs's momentum straight past him!
  - **Seizing the Initiative (`C_def > C_att`)**:
    Look at the rule when an attacker misses a defender with higher `Control`:
    - Control Advantage: `C_def 15 - C_att 2 = 13` (capped/applied by exchange scale—and **`Parry` DOUBLES (`2×`) the recovery ticks** Kovacs suffers on the clock!).
    - Kovacs stumbles past Mateo with his back half-turned against the stair landing—pushed forward **`+8` extra recovery ticks** (all the way to `Tick 32`) while Mateo is already free on **`Tick 20`** (`14 + 6`)!

---

### 3.3 Exchange 3: Clinch Polarity Flip (`Reach 0`) & *Kuzushi* `Throw` vs. `Root`

"Kovacs's back is practically facing you on the landing," Maya narrates, "and his baton is jammed awkwardly behind your hip!"

- **Remember the Reach Polarity Flip**: When Kovacs held `Reach 2` at standoff distance, Mateo had to step inside (`+2 Δ_ext`). **Now that Mateo is chest-to-chest in `Reach 0` Clinch**, the polarity **flips**: Kovacs's long `Reach 2` steel baton is crowded uselessly (`+4 Δ_ext` to Kovacs!), while unarmed elbows, knees, and throws work at `100%` efficiency!
- **Tick 20 (Mateo — `Grab` into *Kuzushi* `Throw`)**:
  - Sam switches to **Wrestling, Judo & BJJ Grappling (`N = 2`, `KIN 12 / FOR 11 -> C_skill = 11`, `C_combat = floor((11+12)/2) = 11`, `P_base = 11 + 6 = 17`, `B = 5`)** to execute a **`Throw`** (`Base TC 9`, `0.85× POW`, `Guard +1`, `Root +1`).
  - Sam allocates **`4 Tempo / 8 Control / 5 Force`** (`4 + 8 + 5 = 17`; `Effective TC = 9 - (4 - 5) = 10 Ticks`).
  - *"Wait—Kovacs weighs `91 kg` (`MMF = 13.0`)," Sam checks. "Can Mateo actually throw a guy fourteen kilos heavier than him?"*
  - "Let's check **Effective Root**!" Maya points out:
    $$\text{Effective Root} = F_{\text{def}} + \text{MMF}_{\text{def}}$$
    - If Kovacs had planted his feet in a **`Root` maneuver (`+4 Root + floor(C/2)`)** with `Force 6`, his Effective Root would be a brick-wall `6 + 13 + 4 = 23+`!
    - **Instead**, Kovacs just stumbled forward off-balance from his `Control 2` lunge (`Effective Control = 2`, compromised Root!). Mateo's **`Control 8` (*Kuzushi* leverage!) + `Force 5`** converts the `+6` Control gap into pure rotational torque!
  - Sam rolls `N = 2` pairs (`TN_P = 31 - 11 = 20`) against Maya's staggered defense (`-6` Control edge -> `TN_GM = 27`): `15 + 11 = 26 > 20` (`+6`), `10 + 6 + 6 = 22 > 20` (`2` Successes vs. `0`) -> **`CleanSuccess`**!
  - **Result**: Mateo pivots his hip under Kovacs's center of mass and slams all `91 kg` of the supervisor flat onto the concrete steps (`0.85 × 15.4 × (5/5) ≈ 13.1` Blunt Impact -> **Tier 2/3 Rib & Back Shock**, **`Prone` (`-4 T_max`, `8 Ticks` just to stand up)**, and the baton skitters down the stairwell)!

---

### 3.4 Exchange 4: Watching `Exertion Marks` vs. `Fatigue Threshold`

Before Mateo grabs the baton and leaps down the last flight of stairs, Maya asks Sam to check one final column on his scratchpad: **Exertion Marks**.

"Look back at the three allocations Sam just ran," Maya says:
- **Exchange 1 (`11 Tempo / 8 Control / 2 Force`)**: Mateo's `Tempo 11` exceeded **`50%` of his `Tempo Cap` (`14 / 2 = 7`)**, while his `Force 2` stayed below `50%` of `F_max 11` (`5.5`). That adds **`+1` Exertion Mark** (`1 / 17 FT`).
- **Exchange 2 (`6 Tempo / 12 Control / 3 Force`)**: Neither `Tempo` (`6 <= 7`) nor `Force` (`3 <= 5.5`) exceeded `50%` of ceiling! Investing into **Control** cost **`0` Exertion Marks**—you conserved breath!
- **All-Out Blitz Warning**: If Sam had redlined *both* `Tempo > 7` AND `Force > 5.5` (say `10 Tempo / 2 Control / 9 Force`), every swing would pile on **`+2` Exertion Marks**! Once Marks hit **`FT = 17`**, Mateo gains **`+1 fatigue_level`** (`-2 Pool`, `+10% TC`, `+2 Δ_int`)—slowly draining his pool until an opponent with lower raw strength takes him apart cleanly!

"No wonder boxing coaches tell you to stop throwing wild haymakers in Round 1," Sam says, sliding his red bead off the stairwell strip as Mateo scoops Kovacs's keycard and races toward the parking garage.

---

## 4. Pocket Cheat Sheet: The Six Tactical Triad Archetypes

Keep this mini-card by your dice whenever splitting `P_eff = T + C + F` in melee:

| Triad Stance | Pool Bias | Classic Real-World Move | Signature Strength & Vulnerability |
| :--- | :--- | :--- | :--- |
| **`TC` (The Probe)** | High **Tempo + Control**, Low Force | Fast boxing jab, stop-thrust, parry-riposte | Finishes fast (`Low TC`), tight `Guard`, `0`–`1` Exertion—but light impact and low `Root`. |
| **`TF` (The Blitz)** | High **Tempo + Force**, Low Control | Lunging overhand, flying knee, bull-rush | Beats slow foes with full `POW`—but exposes `Guard` (`Low C`), begging to get parried (`2×`)! |
| **`CF` (The Finisher)** | High **Control + Force**, Low Tempo | Planted liver hook, hip throw, joint lock | High precision, devastating `POW`, unshakable `Root`—slow on the clock (`High TC`). |
| **Pure `T`** | `60%+` **Tempo** | Full disengage, desperate sprint, snap dodge | Maximum evasion (`+floor(T/2)` vs. hits) and speed—zero stopping power. |
| **Pure `C`** | `60%+` **Control** | Tight high-guard shell, counter-trap wait | Virtually untouchable (`0` Exertion); forces careless attackers to Seize-Initiative stumble. |
| **Pure `F`** | `60%+` **Force** | Telegraphing haymaker, heavy shield slam | Crushes static blocks—wildly slow and wide open to fast counter-jabs. |

---

> **Next Step**: **[Guide 06: Firefights, Ballistics, Zero-HP Trauma & Survival](06-firefights-trauma-and-survival.md)** — Rain, gunsmoke, Kevlar vs. Behind-Armor Blunt Trauma, weapon jams, suppression morale, `10.0`-Unit bleeding, and a `6`-second tourniquet under fire!
