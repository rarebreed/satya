# Guide 06: Firefights, Ballistics, Zero-HP Trauma & Survival

> **Reference Chapters**: [Ballistics & Firearm Engineering](../ttrpg/05-combat/firearms_and_ballistics.md) · [Ranged Combat, Reliability & Armor](../ttrpg/05-combat/ranged_combat_and_armor.md) · [Health, Trauma & Triage](../ttrpg/06-health-and-survival/health_and_trauma.md) · [Movement, Encumbrance & Vehicles](../ttrpg/06-health-and-survival/movement_vehicles_and_environment.md)

---

## 1. Scene Opening: The Rain-Lashed Parking Deck

The heavy garage exit bursts open onto a slick concrete parking deck. Driving coastal rain hammers the asphalt thirty-five meters from Mateo's parked, pre-autonomous sedan.

Two figures step out behind concrete support pillars near the exit ramp twenty meters away, cutting off the driveway:
- **Pursuer 1 (`Translucent` SecOps Rifleman)**: Wearing an overt **Lightweight UHMWPE Patrol Carrier (`PR_Ballistic = 6, BTD = 4`)**, leveling a **`5.56×45mm` M4A1 Carbine** (`R_wpn = 3`, `Acc +2`, `Rel +1`, `Raw Impact 7`, `Base Pen 6`, `Wobble 1`).
- **Pursuer 2 (Hired Street Enforcer)**: Wearing a heavy synth-leather jacket (`PR_Ballistic = 1, BTD = 1`), holding a sear-modified **`"Switch"` Machine Pistol** with a bulky **`50-rd` drum** (`9×19mm`, `R_wpn = 4`, Full-Auto `R = 8`, `Acc -2`, **`Rel -2`**, `Clear Jam 12/28 Ticks`, `Raw Impact 7`, `Base Pen 2`, `Wobble 3`).

"Alright," Maya says, placing beads on the `0–40` Tick strip:
- **Linnea (`OODA 13 -> Tick 2`)** sprints behind a concrete stairwell wall to stay out of the crossfire (`Pure Tempo`).
- **Mateo (`OODA 12 -> Tick 3`)** draws his **Glock 19 Gen6 (`9×19mm` `124 gr` JHP**: `R_wpn = 3`, `Acc 0`, `Rel +2`, `Clear Jam 8/20`, `Raw Impact 7`, `Base Pen 1`) and ducks behind a thick concrete pillar wearing his **Concealable NIJ IIIA Soft Vest (`PR_Ballistic = 4`, `BTD = 2`, covers Chest `[11–14]` & Abdomen `[8–10]`)**.
  - *Mateo's Pistols Envelope (`N = 4`, `DEX 13 / SIG 12 -> C_skill = 12`, `C_combat = 12`)*: **`P_base = 24`** (`B = 8`, `T_max 14`, Ranged `C_max = floor((13+12)/2) = 12`, `F_max 11`).
- **Amara (`OODA 11 -> Tick 4`)** drops behind a reinforced concrete parking bollard with her **semi-auto `5.56mm` patrol carbine** (*Long Guns* `N = 3`, `DEX 12 / SIG 14 -> C_skill = 13`, `C_combat = 12`, **`P_base = 21`**, `B = 7`, `T_max 10`, Ranged `C_max 13`, `F_max 11`).
  - *Wait—check Devon's pack weight!* Amara is lugging `35 kg` of trauma kits, tools, and extra hardware from the annex against her `L_base = 13.2 kg` (**`2.65× L_base` = Encumbrance Tier 3!**), which currently slashes **`-6 T_max` (`10 -> 4`)**, adds **`+0.25` physical `TC`**, and **locks out sprinting**! She stays anchored behind the bollard for now.

---

## 2. Ranged Triad, Bracing, Recoil Climb & Weapon Jams

### 2.1 Why Amara Braces First (`Brace: TC 5` $\to$ `Deliberate Aimed Shot: TC 12`)

**Tick 4 (Amara)**: Devon checks the **Ranged Maneuvers** table. "If Amara shoots right away off-hand while exhausted under heavy pack weight, she'll need to spend `Force` just to hold her recoil (`R = 3`). Can she brace her carbine across the top of that flat concrete bollard?"

"Smart play!" Maya approves. "**`Brace / Rest Weapon`** costs **`5 Ticks`** (`4 + 1` -> `5 × 1.25 ≈ 6` with Tier 3 load, finishing on **`Tick 10`**), adds **`+2 Guard`**, **`+3 Root`**, **`-1 Δ_ext`**, AND **slashes Effective Recoil $R_{\text{eff}}$ by `-3`!**"

$$R_{\text{eff}} = \max(0,\; R_{\text{wpn}} + \Delta R_{\text{mode}} - 3 \cdot \mathbb{I}_{\text{braced}}) = \max(0,\; 3 + 0 - 3) = \mathbf{0}$$

"Notice what just happened!" Maya points out as Devon preps Amara's follow-up **`Deliberate Aimed Shot` (`Base TC 12`: `4 Decide + 8 Act`)**:
- Because bracing reduced her Effective Recoil to **`0`**, Devon puts **`0` or `1` point into `Force`** and pours **`12` points straight into `Control` (`8 Tempo / 12 Control / 1 Force`)**!
- A **Deliberate Aimed Shot** unlocks full weapon `Acc` plus **`-2 Δ_ext` aim bonus** (plus `-1` from Brace!), letting Amara line up a pinpoint shot past cover!

### 2.2 The `"Switch"` Gunner's Unbraced Burst & Drum Jam (`Rel -2`)

**Tick 5 (Pursuer 2 — `"Switch"` Machine Pistol)**: Behind Maya's screen, the hired gun steps out and hoses Mateo's pillar with a **`Suppressive Full-Auto Zone` burst (`Base TC 9`, Mode Recoil `+4 -> R_eff = 4 + 4 = 8`)**!

Look at how firearm physics catches sloppy gunfighting:
- With a modest pool (`P_base = 18`), he only allocates **`6 Tempo / 4 Control / 4 Force`**—Wait: `Force 4` is **`4` points short** of `R_eff = 8`!
- Compute **Unbraced Recoil Dispersion Penalty**:
  $$\Delta_{\text{ext, recoil}} = \max(0,\; R_{\text{eff}} - F) = 8 - 4 = \mathbf{+4}$$
  The lightweight `9mm` kicks upward wildly (`+4 Δ_ext`, plus `Acc -2`!) across the concrete wall.
- Maya rolls his `N = 2` cascade (`TN_P = 21`):
  - **Pair 1**: `13 + 9 = 22 > 21` (**`1` stray tumbling bullet tags Mateo's pillar side!**).
  - **Pair 2**: `3 + 4 - 6 = 1 < 2` (**Floor Breach `B_P = 1`, `SuccessWithComplication`**!).
- **Malfunction Check**: Because **`raw_botches (1) > Weapon Rel (-2)`**, the `50-rd` drum feeds crookedly! One stray `9mm` round clips Mateo—**and the machine pistol stovepipes (`Minor Malfunction`: `12 Ticks` / `3.0s` just to tap-rack and clear!)**.

> [!TIP]
> **GM Coaching — Tolerance ($\tau_{\text{tol}}$) vs. Reliability ($\text{Rel}$) Tells a Story**
> That single `Rel -2` number saved you from rolling separate jam tables every shot! Cheap street-modified drums (`Rel -2`) jam on *any* floor breach (`1 > -2`), duty Glocks (`Rel +2`) only jam if a combatant pushes through `3+` botches, and rugged AKMs (`Rel +4`) chew through rain and mud untouched—while sacrificing the tight-tolerance `Acc +4` precision of match rifles!

---

## 3. Suppression & Morale Checks (`DIS` Under Fire)

Concrete dust explodes off the pillar inches from Linnea's face as the SecOps rifleman (`Force = 6` -> **`Δ_morale = floor(6 / 2) = +3`**) opens up down the lane!

- **Linnea's Composure Check**:
  - Even hiding behind a wall, cracking supersonic `5.56mm` rounds force a **Suppression Morale Check**.
  - Linnea has `DIS 13`, plus **`-2` Motivation (`SLF 5`, shielding her crew)** $\implies C_{\text{morale}} = 15 \implies \text{TN}_P = 31 - 15 = \mathbf{16}$ (`N = 1` pair).
  - Against Maya's hidden `TN_GM = 21 - 3 = 18`: Elena rolls `11 + 8 = 19 > 16` (`1` Success) vs. Maya's `15 <= 18` (`0`). Linnea stays **`Steady`**!
- **Wait—watch what happens to the `"Switch"` Gunner!**
  - Two seconds later, Amara's braced carbine round punches cleanly into the `"Switch"` gunner's thigh, while bullets snap overhead (`Δ_morale = +4`).
  - He has weak Discipline (`DIS 9`) and mercenary motives (`SLF 14, BLG 7 -> +2` penalty, $\text{TN}_P = 24$). He fails cleanly once (`Steady -> Wavering`: **`-2` Pool**, **`Force <= Control`**), then fails a second shock check (`Wavering -> Broken`: **`-4` Pool**, **active offensive `Force = 0`**)—dropping back behind a concrete pillar trying desperately to disengage!

---

## 4. Zero Hit Points: Armor (`PR` vs. `BABT`), Hit Locations & Hemorrhage

Now rewind three seconds to resolve those two bullets hitting **Mateo**—because **this is where *Satya* feels completely unlike any Hit-Point game.**

### 4.1 Hit 1: Stopped by Kevlar, Cracked by `BABT`

The `"Switch"` gunner's stray `9mm` FMJ round (`Raw Impact = 7`, `Base Pen = 2`, `Wobble = 3`, `ΔM = +1`) strikes Mateo (`1d20` Hit Location = **`12` -> Chest / Ribcage**).

Sam gulps. "Wait—Mateo's wearing his Concealable **NIJ IIIA Soft Vest (`PR_Ballistic = 4`, `BTD = 2`)**, covering Chest `[11–14]`. Does he take zero damage?"

"Check **Penetration vs. `BABT`**!" Maya says, reading off the two-step rule:

1. **Effective Pen & Impact**:
   $$\text{Effective Pen} = 2 + \left\lfloor \frac{1}{2} \right\rfloor = \mathbf{2}, \qquad \text{Effective Raw Impact} = 7 + \left\lfloor \frac{1}{2} \right\rfloor = \mathbf{7}$$
2. **Compare $\text{Effective Pen (2)} \le \text{PR (4)}$**:
   - **The Kevlar weave catches the tumbling lead!** **Zero perforation, zero bleeding (`causes_bleed = false`)!**
   - **HOWEVER**, soft aramid bends inward into his chest wall, delivering **Behind-Armor Blunt Trauma (`BABT`)**:
     $$\text{BABT} = \max(0,\; \text{Effective Raw Impact} - \text{BTD}) = 7 - 2 = \mathbf{5}$$
3. **Result (`Trauma 5` = Tier 2 Moderate Structural Wound on Chest)**:
   - **Cracked Rib**: Mateo's **`Force Cap (F_max)` immediately drops by `-5`** (`11 -> 6`!).
   - **`+1` Wound Shock (`wound_shock_index = 1`)**: Shrinks his Tactical Pool by **`-2`** (`24 -> 22`) and slows all physical/mental actions by **`+25%` (`action_time_factor = 1.25`)**!
   - **Immediate `WSR` Save (`HLT 12 / DIS 11 -> C_P = 11`, `TN_P = 20` vs. `TN_GM = 21 - 2 = 19`)**: Sam rolls `14 + 8 = 22 > 20` (`1` Success vs. `0`) -> Mateo stays upright, wheezing hard around a bruised sternum!

*(Meanwhile, Mateo fires back from **`Cover Peek` (`+3 Cover Guard`, `7 Tempo / 12 Control / 5 Force -> 22` Pool)**—landing two `9mm` JHP rounds into the `"Switch"` shooter's unarmored flank before he breaks!)*

---

### 4.2 Hit 2: Unarmored Thigh Wound, Shock & `10.0`-Unit Bleeding

Before the SecOps rifleman backs down under Amara's suppressive fire, his **`5.56mm` M4A1 (`Raw Impact = 7`, `Base Pen = 6`)** snaps around the pillar with **`+2` Net Successes**:
- **Effective Impact = `7 + floor(2/2) = 8`**, **Effective Pen = `6 + 1 = 7`**.
- Maya rolls `1d20` for Hit Location: **`5` -> Right Thigh (`Femur / Femoral`)**!
- Mateo's soft executive vest only covers `[8–14]` (Abdomen & Chest)—his legs have **`PR = 0`**!

Watch the cascade of anatomical consequences hit Sam's scratchpad all at once:

1. **Tier 3 Severe Structural Wound (`Trauma 8`)**:
   - **Leg Ceiling Slashed**: Mateo's **`Tempo Cap (T_max)` immediately collapses by `-10` (`14 -> 4`)**! Because `T_max (4)` is now *below* his neutral baseline `B = 8`, every physical move automatically adds **`+4` extra Ticks**—and sprinting is **impossible**.
   - **Wound Shock Jumps by `+2` Tiers**: Total `wound_shock_index` hits **`3`** (`-6` Tactical Pool -> `24 - 6 = 18` Pool; **`action_time_factor = 1.75`**!).
   - **Immediate `WSR` Consciousness Save (`TN_P = 20` vs. `TN_GM = 21 - 6 = 15`)**: Sam rolls `10 + 7 = 17 <= 20` (`CleanFailure`)! Mateo collapses to one knee (**`Stunned` for `8 Ticks`**, drops behind the pillar!).
2. **Grade 3 Major Venous Hemorrhage (`1.0 Unit per 20 Ticks` / `5 seconds`)**:
   - Every human carries **`10.0 Units`** (`~5.0 liters`) of circulating blood.
   - Left untended, Mateo loses **`1.0 Unit` every `5 seconds`**! Once he loses `2.0–3.0 Units` (`Class II`), he takes *another* `+1` Wound Shock and `+2` Fatigue; at `5.0+ Units` lost (`Class IV`), he falls **Unconscious** and suffers fatal cardiac arrest within `60` seconds!

---

## 5. Quick-Release Pack, `6`-Second Tourniquet & Vehicle Escape

Devon looks at the Tick strip in alarm: Amara is behind a bollard `10` meters away, carrying a `35 kg` **Encumbrance Tier 3** pack that stops her from sprinting—and Mateo is bleeding `1.0 Unit` every `20` Ticks!

### 5.1 `4-Tick` Quick-Release & `6`-Second `CAT` Tourniquet

1. **Drop the Heavy Pack (`4 Ticks` / `1.0s`)**:
   - "Amara hits the quick-release shoulder buckle on her heavy pack!" Devon declares.
   - Dropping `22 kg` leaves only her carbine and hip trauma pouch (`~13 kg <= L_base 13.2 kg` -> **Encumbrance Tier 0!**), instantly restoring **`T_max = 10`** and full sprint velocity:
     $$V_{\text{sprint}} = \big(4.0 + 0.35 \times 10\big) \times 1.0 = \mathbf{7.50\text{ m/s}} \quad (\mathbf{1.88\text{ m/Tick}})$$
   - Amara sprints `10` meters across the lane into cover beside Mateo in **`~6 Ticks` (`1.5s`)** (`Pure Tempo`, while the SecOps rifleman ducks away under the parking deck alarm!).
2. **Apply Emergency Combat Tourniquet (`CAT` — `Base TC = 24` = `4 Decide + 20 Act` = `6.0 Seconds`)**:
   - Under cover, Devon rolls Amara's **First Aid (`N = 1`, `DEX 12 / DIS 12 -> C_P = 12`, `TN_P = 19`)** using a real **CAT Tourniquet (`Δ_tool = 0`)** against `TN_GM = 21` (`Δ_ext = 0`).
   - Devon rolls `13 + 9 = 22 > 19` (`1` Success vs. Maya's `19 <= 21` = `0`).
   - **Clean Success!** At `6.0` seconds (`24 Ticks`), the windlass clicks home above Mateo's thigh wound—clamping the hemorrhage cold at **`2.0 Units` lost** (`8.0 / 10.0 Units` remaining!).

### 5.2 Cover Materials, Vehicle Triad Getaway & Survival Realities

Linnea drags the sedan's rear door open while Amara helps a pale, shivering Mateo into the back seat (`+2` Fatigue from `Class II` blood loss, `Wound Shock 3`). As Linnea turns the ignition, the SecOps rifleman snaps a last `5.56mm` round (`Raw Impact 7, Pen 6`) straight at the car door!

"Does hiding behind a car door stop rifle bullets like in movies?" Sam asks nervously.

"Nope—only the **front engine block (`PR_barrier = 9`)** stops rifle rounds!" Maya warns, checking the **Barrier Ballistics** table:
- A **Car Door (`VehicleBody`: `PR_barrier = 3`)** stops `9mm` handguns (`Pen 1–3 <= 3`), **but** is completely overmatched by `5.56mm` (`Pen 6 > 3`)!
- The bullet punches straight through the sheet metal with **Residual `Pen = 6 - 3 = 3` and `Impact = 7 - 3 = 4`**, smacking Mateo's door-side Kevlar vest (`PR 4 >= 3`) harmlessly as a dull thud instead of killing him!
- *(Lesson for players: crouch behind **engine blocks (`PR 9`)** or **concrete pillars (`PR 9`)**, never sofas (`PR 1`), wooden desks (`PR 2`), or passenger car doors (`PR 3`)!)*

Linnea floors the sedan up the wet ramp as a SecOps patrol SUV screeches behind them!

"How do car chases work?" Elena asks quickly.

"Exact same **Tactical Triad**!" Maya smiles:
- **Tempo (`T`)** pushes throttle and speed to open the **Chase Range Band** (`Close [15m] -> Medium [50m] -> Long [150m+] -> Lost Visual`).
- **Control (`C`)** threads the rain-slick concrete hairpin corner without spinning out.
- **Force (`F`)** holds traction (`Vehicle Root`) if someone tries a PIT bump or clips a plastic bollard.

Elena splits `5 Tempo / 6 Control / 2 Force` (`Control` biased for the wet ramp curve!), hits `2` successes on *Driving*, and shoots cleanly out onto the dark, rain-swept arterial road—vanishing into Seattle's brownout traffic before the patrol SUV rounds the gate.

### 5.3 What Happens Tomorrow Morning (Why Violence & Weather Have Weight)

As everyone packs up dice, Maya reminds them of the aftermath before next week's session:
- Mateo didn't lose "34 of 60 Hit Points" that vanish after an 8-hour nap.
- His **Tier 2 Cracked Rib** takes **`2–4 weeks`** to knit (`-5 F_max` until healed); his **Tier 3 Thigh Wound** requires sterile stitching/extraction at Linnea's trusted underground clinic and **`6–12 weeks` of rehabilitation** before he sprints normally again—AND his `9mm` casings in the garage raise citywide **Heat** (`Heat 2 -> 4`).
- Getting everyone inside a warm, dry safehouse with clean water and blankets matters too: staying rain-soaked outside drops skin temperature into **Cold Stress (`-3 DEX/TAC`, `+2 Δ_int`)**, while forgetting to hydrate (`3–5 L/day`) pins a **`fatigue_level` floor** that sleep alone cannot erase.
- Next session, **Linnea and Amara** will carry the physical legwork while Mateo runs tactical overwatch, lock-prep, and street intel from a back brace and cane—making every bullet fired tonight echo straight into the story.

---

> **End of Core Play Guides**: Ready for the exact lookup tables, equations, and full 8-Tree skill lists? Browse the **[18-Chapter Satya TTRPG Rules Book](../ttrpg/SUMMARY.md)**.
