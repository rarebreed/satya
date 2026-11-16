# Guide 03: The `0.25s` Count-Up Clock, `OODA` & Multi-Sensory Stealth

> **Reference Chapters**: [Action Economy, OODA Loop, Time Scales & Stealth](../ttrpg/02-core-mechanics/action_economy_and_ooda.md) · [Characteristics & Derived Metrics](../ttrpg/03-character/characteristics_and_aptitudes.md)

---

## 1. Leaving Turn-Based Habits at the Door

With the steel service door eased open, cold conditioned air washes out across the crew's boots. Ahead stretches a windowless cinderblock corridor leading toward the cold-storage server bay. A single emergency LED hums weakly fifty feet down the hall.

"Okay," Sam says, reaching for a single `d20`. "Let's roll Initiative—and on my turn I'll take one Move action down the hall and ready an attack."

Maya holds up a hand with a laugh. "Leave *D&D*'s 6-second turn carousel in the parking lot! In *Satya*, nobody waits patiently in a circle while everyone else freezes for six seconds. Time runs on a **continuous `0.25`-second Count-Up Tick Clock**."

She slides a long strip of cardstock across the center of the table, numbered cleanly from `0` to `40`, and places three colored glass beads beside it: red for Mateo, blue for Linnea, green for Amara.

"Don't panic—you don't track Ticks while walking down an empty hallway," Maya adds. "Right now we stay in normal **Narrative Time**. We only drop beads onto this Tick strip the *instant* split-second timing decides who shoots, ducks, interrupts, or gets caught first."

### 1.1 The Four Time Benchmarks Every Player Should Memorize

Maya taps the margin of her cardstock strip:

| Unit | Ticks | Real Time | What Happens at That Pace |
| :--- | :---: | :---: | :--- |
| **`1 Tick`** | `1` | **`0.25 seconds`** | Visual reaction latency (`250 ms`); dropping an item; a single shouted word; pulling a trigger already on target. |
| **`1 Second`** | `4` | **`1.00 second`** | A snap-jab, a quick sidestep, or `~7.5 m` of full-tilt sprinting. |
| **`1 Tactical Window`** | `20` | **`5.00 seconds`** | One HUD cycle on the Combat Tracker—usually `2–3` crisp exchanges. |
| **`1 Minute`** | `240` | **`60.0 seconds`** | Arterial/venous bleed ticks, holding breath underwater, and longer lock/hack sequences. |

> [!TIP]
> **GM Coaching — Running the Paper Tick Strip Effortlessly**
> Never pre-roll `20` turns ahead. Keep a strip numbered `1–40` (or reuse `1–20` looping like a clock face) and simply call out: **"Clock is on Tick 6—who's lowest?"** Whoever holds the lowest Tick moves their bead forward by their action's **`Effective TC`**, and play advances smoothly to whoever is lowest next. In the **Satya CARPG software**, the Combat Tracker advances this exact strip automatically.

---

## 2. Multi-Sensory Stealth in the Blacked-Out Corridor (`Observe`)

"Before anyone draws a bead on a clock," Maya says, "we have to see if the night patrol catches you slipping past the generator hall. How fast are you moving down this corridor, and what's in your hands?"

Devon checks Amara's gear weight (`< 13.2 kg` -> **Encumbrance Tier 0**). "Amara moves ahead slowly—silent heel-to-toe."

"Good," Maya nods. "That's **Stealth Creep** (`18%` of sprint speed, about `0.34 m/Tick` or `1.35 m/s`—so forty feet takes roughly ten seconds). Moving at **Stealth Creep** gives **`+0` penalty** to your *Stealth* check. If someone tries a faster **Combat Glide** (`0.65 m/Tick`), that adds **`+4` $\Delta_{\text{ext}}$** to *Stealth*, and **Running** adds **`+8`**!"

Sam glances at Mateo's sling bag. "What about gear clatter?"

"Every **Encumbrance Tier above Tier 1** adds **`+2` $\Delta_{\text{ext}}$** to *Stealth*," Maya tells him. "Mateo dropped his heavy duffel by the fence, so he's wearing light patrol gear (`Tier 1`)—zero noise penalty."

### 2.1 Four Separate Sensory Channels (No Generic "Spot Check")

"Watch closely how *Satya* replaces a single 'Perception' roll with **four distinct sensory channels**," Maya says, pulling four separate dice cups behind her screen:

1. **Sight (`SIG / FOC`) — Darkness & Flashlight Geometry**:
   - The power outage leaves the corridor in near-total pitch darkness (**Moonlight / Power Outage Indoors**: **`+6` $\Delta_{\text{ext}}$** against a guard's `SIG` check—or **`-6` bonus** to the infiltrators' *Stealth*!).
   - Suddenly, a pale beam sweeps across the far T-junction. "A patrol guard is checking doors with a tactical flashlight!" Maya warns. "Inside that narrow beam, lighting drops to **`+0`**—*but* waving an active flashlight across a dark hall gives **you** a massive **`-6` bonus** to spot *him* from fifty feet away!"
2. **Hearing (`HEA / FOC`) — Broken Tile & Glass**:
   - "Amara spots dry drywall dust and a broken fluorescent tube near the stairwell door," Maya narrates. "Devon, roll Amara's *Stealth* (`KIN 12 / FOC 10 -> C_P = 11`, `TN_P = 20`, `N = 3` pairs)."
   - Devon rolls: `14 + 9 = 23 > 20` (`+6`), `10 + 7 + 6 = 23 > 20` (`+6`), `11 + 8 + 6 = 25 > 20` -> **`S_P = 3` Successes!**
   - Behind the screen, the patrol guard (`HEA 10 / FOC 10 -> C = 10`, *Observation* `N=1`, `TN_GM = 21`) rolls `13 + 9 = 22 > 21` -> `S_GM = 1`. Amara beats him `3` to `1`—her soft moccasins step right over every shard without a whisper.
3. **Smell (`OLF / INS`) — Hidden Information Through Nose Alone**:
   - Maya turns to Devon: "You don't even have to roll—with Amara's **`OLF 12`**, the recycled vent draft carries warm **gun-cleaning solvent and stale menthol smoke** right around the stairwell corner. Someone's stationed *inside* the stairwell landing, not just walking the hall!"
4. **Intuition (`ITU / INS`) — The Sixth-Sense Warning**:
   - As Elena's Linnea steps up beside Mateo, Maya checks Linnea's high **`ITU 14` & `INS 15`** (`C_P = floor(29/2) = 14`, `TN_P = 17`, `Observation N=1`) privately against a shifting vibration in the corridor.
   - Elena rolls `12 + 9 = 21 > 17` (`1` success) right as a second door clicks behind them. *"You don't hear boots yet,"* Maya whispers to Elena, *"but the hair on the back of Linnea's neck stands straight up. Someone on the far side of the cold-storage fire door just touched the panic bar."*

---

## 3. Entering the Clock: 4-Pillar `OODA` & "Quick Thinking" (`D_cog`)

*Click-clack.* The heavy steel fire door swings inward thirty feet ahead! A startled *Translucent* SecOps guard steps straight into the corridor holding a flashlight and a belted baton.

"Everyone—onto the Tick strip!" Maya calls out. "Let's calculate where each of you enters the clock using **Boyd's 4-Pillar OODA Loop**."

### 3.1 How Your Mind and Body Set Your Starting Tick

Maya writes the two quick formulas on the corner of the map:

$$D_{\text{cog}} = \left\lfloor \frac{\max(\text{ANA}, \text{CRE}) + \text{FOC}}{2} \right\rfloor, \qquad \text{OODA} = \left\lfloor \frac{\text{ITU} + \text{INS} + D_{\text{cog}} + \text{SPD}}{4} \right\rfloor$$

$$\text{Tick}_{\text{start}} = \max(1,\; 15 - \text{OODA} - \text{Awareness Margin})$$

"Look at the four letters of **O-O-D-A**," Maya points out while everyone checks their sheet:
- **Observe (`ITU`)**: Spotting the danger first.
- **Orient (`INS`)**: Understanding what that shadow actually means.
- **Decide (`D_cog`: `ANA` or `CRE` + `FOC`)**: **"Quick Thinking"**—choosing a move fast without freezing!
- **Act (`SPD`)**: Physical muscle snap.

Watch how the glass beads land on the paper strip:

1. **Elena (Linnea Cho)** — `SPD 10`, `ANA 13 / CRE 12 / FOC 13 -> D_cog = 13`, `ITU 14`, `INS 15`:
   $$\text{OODA} = \left\lfloor \frac{14 + 15 + 13 + 10}{4} \right\rfloor = \left\lfloor \frac{52}{4} \right\rfloor = \mathbf{13} \implies \text{Tick}_{\text{start}} = 15 - 13 = \mathbf{Tick\ 2}$$
   *(Notice Elena's jaw drop: even though Linnea has average `SPD 10`, her razor-sharp `ITU 14`, `INS 15`, and `D_cog 13` put the non-combatant ethicist **ahead** of Mateo on the clock!)*
2. **Sam (Mateo Vance)** — `SPD 14`, `ANA 13 / CRE 10 / FOC 12 -> D_cog = 12`, `ITU 13`, `INS 12`:
   $$\text{OODA} = \left\lfloor \frac{13 + 12 + 12 + 14}{4} \right\rfloor = \left\lfloor \frac{51}{4} \right\rfloor = \mathbf{12} \implies \text{Tick}_{\text{start}} = 15 - 12 = \mathbf{Tick\ 3}$$
3. **Devon (Amara Okafor)** — `SPD 10`, `D_cog = 10`, `ITU 10`, `INS 14`:
   $$\text{OODA} = \left\lfloor \frac{10 + 14 + 10 + 10}{4} \right\rfloor = \mathbf{11} \implies \text{Tick}_{\text{start}} = 15 - 11 = \mathbf{Tick\ 4}$$
4. **The Startled Guard (Ambushed!)**:
   - Base `OODA 9` would put him on `Tick 6`—**but** because Amara smelled him coming and Linnea read the door click while he walked blind out of bright breakroom light into darkness, he suffers a **`+8` to `+12` Tick Orientation Freeze**!
   - Maya drops his bead all the way back at **`Tick 14`** (`6 + 8` freeze Ticks = `2.0 seconds` of *"Wait—who's there?!"* confusion).

---

## 4. Two-Component Time Costs: Why `D_cog` Beats Raw Muscle

"Wait," Sam says, studying the `Base TC` column. "Why does an action show two numbers in parentheses, like `8 (5 + 3)`?"

"Because *Satya* splits every action's Base Time Cost into **Decision Ticks (`TC_decide`)** plus **Execution Ticks (`TC_act`)**!" Maya explains:

$$\text{Base TC} = \text{TC}_{\text{decide}} + \text{TC}_{\text{act}}$$

- **Drilled Reflexes (`TC_decide = 1–2`)**: A jab, a parry, or dropping an empty mag uses muscle memory (`Orient -> Act`), bypassing long deliberation. Even better, once a skill reaches **`N >= 4`**, **Skill Reflex Compression** shaves `-1 Tick` off `TC_decide` (`-2 Ticks` at `N >= 6`)!
- **Complex or Improvised Decisions (`TC_decide = 3–6`)**: Scanning a chaotic room, faking out a defender, clearing a mystery jam, or improvising an environmental trick forces your brain to think—and **Quick Thinking (`D_cog`)** shaves **`-1 Tick` per `+2` points above `10`**:
  $$\Delta\text{TC}_{\text{decide}} = -\left\lfloor \frac{D_{\text{cog}} - 10}{2} \right\rfloor$$

### 4.1 Quick Thinking vs. Pure Physical Speed (Worked Contrast)

"Here's why you never dump `ANA`, `CRE`, or `FOC` just to min-max `SPD`," Maya illustrates on her scratchpad.

Suppose on **Tick 2**, a heavy magnetic fire-door lock is about to slam shut, and **Linnea (`D_cog 13`, `SPD 10`)** wants to jam a folding aluminum doorstop/fire extinguisher under the closer arm (**Improvise Environmental Action**: `Base TC = 8`, split into **`5 Decide + 3 Act`**).

- **Linnea (`D_cog 13`)**:
  $$\Delta\text{TC}_{\text{decide}} = -\left\lfloor \frac{13 - 10}{2} \right\rfloor = \mathbf{-1\text{ Tick}}$$
  Her effective time cost is `(5 - 1) + 3 = 7 Ticks` (`Tick 2 + 7 = Tick 9`!).
- **Compare a Sharp Tactician (`D_cog 16, SPD 10`) vs. a Slow-Thinking Athlete (`D_cog 8, SPD 15`)** doing that exact same `8-Tick` improvisation:
  - **Sharp Tactician (`D_cog 16`)**: Shaves `-3` off `TC_decide` (`5 - 3 = 2` Decide + `3` Act) = **`5 Ticks` (`1.25s`)**!
  - **Slow Athlete (`D_cog 8, SPD 15`)**: Hesitates `+1` on `TC_decide` (`5 + 1 = 6` Decide) while shaving `-2` physical Ticks off `TC_act` (`3 - 2 = 1` Act) = **`7 Ticks` (`1.75s`)**!
  - **Result**: The Quick Thinker finishes **`0.5 seconds` faster** despite having `5` less physical `SPD`!

---

## 5. Declaration vs. Completion & Mid-Action `OODA` Pivots

Back at the corridor, Maya walks the beads forward tick by tick!

- **Tick 2 (Linnea — Lowest Bead!)**: Elena decides Linnea doesn't want a gunshot echoing down a clinic annex. She starts a **Deliberate Tactical Scan ("Read the Room": `Base TC 6 = 5 Decide + 1 Act`)** on the startled guard's badge, belt, and radio mic (`ΔTC_decide = -1` from `D_cog 13` -> **`5 Ticks`**, finishing on **`Tick 7`**). Maya moves the blue bead to `7`.
- **Tick 3 (Mateo — Next Lowest Bead!)**: Sam checks the strip: the guard is frozen until `Tick 14`.
  - "Mateo slips forward six meters in a quiet **Combat Glide** and grabs the guard's radio lapel mic before he can yell," Sam says.
  - Maya estimates the fast glide + grab (`Base TC = 10` ticks total, finishing on **`Tick 13`**). Sam slides the red bead to `13`.
- **Tick 4 (Amara)**: Devon keeps Amara crouched low (`Cover` / `Ready` posture, **`TC 4`** -> **`Tick 8`**) watching the far stairwell corner where she smelled tobacco smoke!

### 5.1 The Surprise Interruption at `Tick 6` (`OODA Reset`)

Maya counts up: `"Tick 5... Tick 6!"`

Suddenly, the stairwell door behind Amara rattles open! A *second* guard—the smoker Amara smelled—steps out holding a cup of coffee and locks eyes on Mateo mid-stride down the hall (`Tick 6`)!

Sam freezes. "Wait—Mateo isn't done gliding toward Guard #1 until `Tick 13`! Is he stuck walking helplessly in the open for another `7` Ticks?!"

"Never!" Maya shakes her head. "In *Satya*, you can **abort any in-progress action on the spot** and pivot to a new reaction—like diving behind a heavy cryo-freezer cart or drawing your Glock. You only pay the **`OODA Reset Penalty`** to dump your old plan and lock onto the new threat:"

$$\text{OODA Reset Ticks} = \max\!\left(1,\; 5 - \left\lfloor \frac{D_{\text{cog}} + \text{ITU}}{5} \right\rfloor\right)$$

Sam runs Mateo's numbers (`D_cog 12`, `ITU 13`):
1. Add Quick Thinking and Intuition: `12 + 13 = 25`.
2. Divide by `5`: `floor(25 / 5) = 5`.
3. Subtract from `5` (minimum `1` Tick):
   $$\text{OODA Reset} = \max(1,\; 5 - 5) = \mathbf{1\text{ Tick}}\text{ (`0.25s`)!}$$

"Mateo dumps his old plan in **one single quarter-second (`Tick 6 -> Tick 7`)**," Maya announces, "and immediately drops behind the steel freezer cart (**Drop Behind Cover: `3 Ticks`** -> finished by **`Tick 10`**)!"

"Compare what happens if a tunnel-visioned guard (`D_cog 7, ITU 7 -> 14`, $\lfloor 14/5 \rfloor = 2$) tries to pivot mid-swing," Maya adds. "He burns **`5 - 2 = 3 Ticks` (`0.75s`)** just getting his brain out of gear before he can even start moving! When both minds and bodies run on the same `0.25`-second clock, sharp awareness and clear thinking keep you alive."

- **Tick 7 (Linnea finishes her Scan!)**: Linnea's blue bead lands right on `Tick 7`. From her quick read, Maya whispers to Elena: *"Guard #1 isn't some hardened mercenary—his hand is shaking near a cheap plastic zip-tie pouch, and his *Translucent* contractor patch looks three weeks old. He's terrified."*

### 5.2 Wait—What Slows Ticks Down? (`action_time_factor` & Reflexes)

"Quick question before we jump into the server room," Devon checks. "What if Amara was exhausted (`Fatigue 2`) and wounded (`Wound Shock 1`)—does her `10-Tick` sprint or tourniquet still take `10 Ticks`?"

"No—that's where **`action_time_factor`** bites hard," Maya explains:

$$\text{action\_time\_factor} = 1.0 + (0.10 \times \text{fatigue\_level}) + (0.25 \times \text{wound\_shock\_index}) + \Delta_{\text{enc}}$$

- **Fatigue `2` (`+0.20`) + Wound Shock `1` (`+0.25`)** puts your body at **`1.45×`**! A crisp `10-Tick` (`2.5s`) action suddenly stretches to **`round(10 × 1.45) = 15 Ticks` (`3.75s`)**—while draining **`-6`** off your Tactical Pool so you can't easily buy that time back with `Tempo`!
- Conversely, **drilled mastery (`N >= 4`)** unlocks Boyd's *Implicit Guidance & Control* shortcut, shaving **`-1` Decide Tick** (`-2` at `N >= 6`) down to the reflex floor so practiced hands keep moving smoothly even under chaos.

### 5.3 Table Cheat Sheet: Watching the Beads Move (`Tick 1` $\to$ `Tick 14`)

Here is exactly how Maya's `0–40` paper strip looked across those `3.5 seconds` so you can picture it on your own table:

| Current Clock | Lowest Bead | Who Acts / Finishes | Action & Effective Cost | Bead Moves To |
| :---: | :---: | :--- | :--- | :---: |
| `Tick 2` | Blue | **Linnea** enters (`OODA 13`) | Starts *Tactical Scan* (`5 Ticks`) | `Tick 7` |
| `Tick 3` | Red | **Mateo** enters (`OODA 12`) | Starts *Combat Glide + Mic Grab* (`10 Ticks`) | `Tick 13` |
| `Tick 4` | Green | **Amara** enters (`OODA 11`) | Takes *Overwatch / Ready* on stairwell (`4 Ticks`) | `Tick 8` |
| `Tick 6` | Interrupt! | Guard #2 opens stairwell! | **Mateo aborts (`1-Tick` OODA Reset)** -> *Drop Behind Cover* (`3 Ticks`) | `Tick 10` |
| `Tick 7` | Blue | **Linnea** finishes Scan | Spots Guard #1's nervous contractor badge & shaking hand | `Tick 7` (Next move) |
| `Tick 8` | Green | **Amara** ready trigger | Steps smoothly to intercept Guard #2 before he drops his coffee! | `Tick 14` |
| `Tick 14` | Black | **Guard #1** (`Tick 6 + 8` freeze) | Finally finishes blinking out of bright light—too late! | — |

---

> **Next Step**: **[Guide 04: Psyche, Friction, Morale & Social Combat](04-psyche-morale-and-social-combat.md)** — Inside the cold-storage vault, the party corners lead engineer Dr. Marcus Hale—and Elena fights a full tactical battle using words, empathy, and **Cognitive Dissonance** instead of bullets!
