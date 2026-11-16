# Guide 04: Psyche, Friction, Morale & Social Combat

> **Reference Chapters**: [Psyche, Morale & Social Combat](../ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md) · [Lifepath Hobbies, Events & Factions](../ttrpg/03-character/lifepath_events_hobbies_and_factions.md)

---

## 1. Psyche Friction & Drift: Conscience Has Weight

Ten minutes later, the two nervous perimeter contractors are zip-tied inside an empty custodial closet (Mateo talked the first one down without firing a shot; Amara intercepted the smoker before he spilled his coffee). Now the crew slips past the cold-storage airlock into the chilled diagnostic bay.

Blue status LEDs glow along a double rack of humming cryogenic co-processors. Standing beside an open maintenance terminal is a pale, sleep-deprived technician in a *Translucent* lab coat—and beside him, a senior engineer frantically trying to trigger an emergency cold-wipe on the console.

First, a fast complication hits: the junior technician spots Mateo and bolts sideways toward the manual fire-pull alarm box on the wall (`6` meters away)!

"I draw my pistol and shoot the kid in the knee before he pulls it," Sam says half-jokingly—then pauses, glancing at his sheet. "Wait... Mateo isn't a psycho. He's **`CPS 12` (protective toward regular kids)**, right? Does the game stop me?"

"No rule ever steals player control," Maya says seriously. "You *can* pull that trigger. **However**, *Satya* checks whether your character hesitates, shakes, or pays an emotional price when violating a deep **Psyche** trait (`P >= 13` or `P <= 7`)."

### 1.1 Resolving a Psyche Friction Check (`DIS` vs. `Δ_psyche`)

Elena tests this right after: "Wait—what if *Linnea* grabs the junior kid, shoves him inside the unventilated walk-in utility closet where the oxygen alarm is broken, and deadbolts it so he stays quiet while we work?"

Maya raises her eyebrows and points to Linnea's sheet: **`CPS 17` (Boundless Compassion)**!

"Leaving a panicked innocent locked in an unventilated closet violates **`CPS 17`** directly," Maya says. "That triggers a **Psyche Friction Check**:

1. Calculate **Psyche Deviation ($\Delta_{\text{psyche}}$)** from neutral `10`:
   $$\Delta_{\text{psyche}} = |\text{CPS } 17 - 10| = \mathbf{7}$$
2. Elena rolls Linnea's **Discipline (`DIS 13`, `N = 1` pair)** against $\text{TN}_P = 31 - 13 = \mathbf{18}$.
3. Behind the screen, Maya rolls `1` hidden resistance pair against $\text{TN}_{GM} = 21 - \Delta_{\text{psyche}} = 21 - 7 = \mathbf{14}$."

Elena drops a pair of `d20`s: `12 + 9 = 21 > 18` (`S_P = 1`).
Maya checks behind her screen: `10 + 7 = 17 > 14` (`S_GM = 1`).

"Positive tie (**`GrazingTieSuccess`**)!" Maya narrates. "Linnea closes the latch—*and* her hands visibly tremble as she checks the air-gap beneath the door twice. You get it done, **but** you suffer **`+2` Internal Strain ($\Delta_{\text{int}}$)** for the rest of the scene (`δ_under` jumps from `-6` to `-8`), **and** mark **`1 Drift Mark`** beside `CPS 17`!"

"What happens if I had failed the roll (`CleanFailure` or `BotchFailure`)?" Elena asks.

"Linnea balks at the threshold—freezing for **`6 Ticks` (`1.5s`)** before she can move, or refusing to lock the door unless she spends a full beat steeling herself at **`+4` $\Delta_{\text{int}}$**," Maya explains. "And watch those **Drift Marks**: once you collect $\max(2, \lfloor \text{DIS} / 4 \rfloor) = \lfloor 13 / 4 \rfloor = \mathbf{3\text{ Marks}}$ on `CPS`, Linnea's Compassion permanently erodes from `17` down to **`16`**. People become what they practice under pressure."

Elena immediately decides Linnea hands the terrified kid a bottled water, waves him out the alley exit with his phone confiscated instead of locking him in a dark closet, and turns to face the senior engineer at the console.

> [!TIP]
> **GM Coaching — Don't Overuse Friction Checks**
> Only call for a **Psyche Friction Check** when an action **sharply contradicts** a trait at `P >= 13` or `P <= 7`. Small daily compromises don't trigger it; cold-blooded cruelty from a `CPS 17` pacifist, groveling from an `EGO 16` proud boss, or handing safehouse keys to a stranger by a `TRS 4` paranoid operator *do*.

---

## 2. Social Combat: Fighting With Proof, Empathy & Cognitive Dissonance

The senior engineer slams his palm over the emergency wipe button—then freezes when he sees Amara covering the doorway and Linnea raising both empty hands calmly in front of her chest.

His name badge reads **DR. MARCUS HALE — CRYOGENIC SYSTEMS LEAD**. He's sweating, breathing shallowly, and staring at the wipe confirmation prompt on his screen.

"Okay," Maya says, pulling a fresh index card for Marcus onto the table (keeping his exact numbers hidden behind her screen!). "Shooting him breaks the air-gapped drive recovery—you need him to *unlock* the cryptographic hardware token voluntarily. Welcome to **Social Combat**."

### 2.1 Behind Maya's Screen: Marcus's Psychological "Hit Points"

Unlike a video game where Persuasion is one flat roll against "DC 15," here is what Maya has written on Marcus's NPC card:

- **Discipline & Focus**: `DIS 11`, `FOC 11` $\implies$ **Social Composure Threshold (`CMP`)** = $\lfloor(11 + 11) / 4\rfloor = \mathbf{5}$.
- **Mental/Social**: `ANA 14`, `ELO 12`, *Persuasion / Rhetoric* `N = 2` (`C = 11`, `TN = 20`).
- **9-Trait Psyche**: **`OPN 16`** *(truth-hungry researcher)*, **`CPS 14`** *(quietly disturbed by suffering)*, **`BLG 14`** *(deeply attached to his lab colleagues)*, **`CNF 14`** *(institutional NDA loyalist)*, `EGO 12`, `TRS 9`.

"Social Combat inflicts **four kinds of 'Mental Wounds'**," Maya explains to the table:
1. **Rhetorical Strain** (`+1` net success) — drains Composure (`CMP`) and adds **`+2` $\Delta_{\text{int}}$** stumble drag as the target gets flustered.
2. **Doubt** (`+2+` net successes with logical/empathetic argument) — fractures 1 pillar of conviction (`-1` effective skill pair `N`). **Three Doubt fractures** break their thesis completely!
3. **Cognitive Dissonance ("Mental Bleed")** — happens when your argument **resonates with one high Psyche pole while clashing with another**! It deals ongoing **`+2` $\Delta_{\text{int}}$ per exchange** until they resolve the inner war!
4. **Ego Wound** (`+2+` net successes mocking/humiliating an `EGO >= 13` or `AGG >= 13` target) — risks snapping their Focus (`-3 FOC`) and provoking a reckless outburst.

---

### 2.2 Exchange 1: Reading the Target (`Empathy & Read Person`)

"Before Linnea tries to talk him off the wipe button," Elena says, "she studies his face, his posture, and what's open on his workstation."

"Roll **Empathy & Read Person** (`INS 15 / ITU 14`, `N = 3`)!" Maya says. "This is **Active Opponent Full Cascade (`GmResistanceMode::FullCascade`)**—my NPC gets to roll his own `DIS` (`11`) defense behind the screen."

1. **Linnea's Target**:
   $$C_P = \left\lfloor \frac{\text{INS } 15 + \text{ITU } 14}{2} \right\rfloor = \mathbf{14} \implies \text{TN}_P = 31 - 14 = \mathbf{17}$$
2. **Elena rolls `3` pairs (`TN_P = 17`, `δ_over = +6`, `δ_under = -6`)**:
   - **Pair 1**: `11 + 8 = 19 > 17` (**Success #1**, `Carry_2 = +6`).
   - **Pair 2**: `9 + 7 + 6 = 22 > 17` (**Success #2**, `Carry_3 = +6`).
   - **Pair 3**: `13 + 6 + 6 = 25 > 17` (**Success #3** -> **`S_P = 3`**!).
3. **Maya secretly rolls Marcus's `DIS` (`11`, `N = 2`, `TN = 20`)**:
   - Pair 1 = `12 + 9 = 21 > 20` (`+6`), Pair 2 = `6 + 5 + 6 = 17 <= 20` -> **`S_GM = 1`**.

Net Margin: **`3 - 1 = +2` Net Successes!**
- *Remember the Empathy scale*: `1` net success reveals the target's single strongest trait; **`2` net successes reveal TWO extreme traits in tension** (`3+` reveals the whole profile and adds `-2 Δ_ext` to the follow-up)!
- Maya tells Elena: *"As Linnea watches him hover over that key, two things jump out. First, his eyes flick greedily to the un-clamped anomaly graphs on his left monitor—he's an obsessive empirical researcher (**High `OPN 16`**) who hates fake data. Second, beside his keyboard sits a framed photo of his four lab postdocs holding coffee mugs, and his fingers tap nervously on his wedding ring and corporate lanyard—he is deeply loyal to his team and terrified of ostracism (**High `BLG 14`**)."*

---

### 2.3 Exchange 2: Arming the Dual-Psyche Cognitive Dissonance Wedge

Elena smiles—she sees the exact wedge immediately. "Linnea doesn't raise her voice and doesn't threaten him. She slides her tablet across the desk showing the un-clamped clinical mortality anomalies she pulled before she left *Altruist*—and asks gently: *'Doctor Hale, look at the entropy curves on line 44. You already know those prototypes weren't hallucinating—someone clamped the distress log so your grant team wouldn't see what was suffering in cold storage.'*"

Maya grins behind her screen. "Roll **Persuasion & Rhetoric (`ELO 15 / ANA 13`, `N = 3`)**!"

Let's watch the exact math:
- **Linnea's Personal Target (`TN_P`)**:
  $$C_P = \left\lfloor \frac{\text{ELO } 15 + \text{ANA } 13}{2} \right\rfloor = \mathbf{14} \implies \text{TN}_P = 31 - 14 = \mathbf{17}$$
- **Behind Maya's Screen — The Psyche Filter**:
  - **Resonance (`Empirical Evidence & Logic` vs. `OPN 16`)**: `-(16 - 10) = -6`!
  - **Clash (`Asking him to break with his team/NDA` vs. `BLG 14`)**: `+(14 - 10) = +4`!
  - **Net External Modifier (`Δ_ext`)**: `-6 + 4 = -2`!
  - Maya sets Marcus's defense target/resistance with **`Δ_ext = -2`**, **AND flags the Dual-Psyche Wedge as ARMED!**

- **The Contested Exchange**:
  - Elena rolls `3` pairs against `TN_P = 17`: `10 + 9 = 19` (`+6`), `11 + 8 + 6 = 25` (`+6`), `7 + 6 + 6 = 19` -> **`S_P = 3` Successes!**
  - Marcus rolls his *Persuasion / Counter-Argument* (`N = 2`, `TN = 20`, shifted by `-2` opponent resonance -> harder defense `TN = 22` or opposing `TN_GM = 23`): scores **`S_GM = 1` Success**.
  - **Net Margin**: **`+2` Net Successes!**

**What Hits Marcus's Mind:**
1. **Rhetorical Strain** (`-1` Composure, `CMP 5 -> 4`, `+2 Δ_int`).
2. **1 Doubt Fracture** (`1 / 3` needed to shatter his stance—his effective counter-arguing drops from `N = 2` down to **`N = 1` pair**!).
3. **Cognitive Dissonance ("Mental Bleed")**: Because Linnea resonated with `OPN 16` while clashing with `BLG 14`, Marcus starts **bleeding mentally (`dissonance_index = 1 -> +2 Δ_int` compounding every turn)**!

Maya acts out Marcus gripping the edge of the console, knuckles white: *"He shouts back, voice cracking: 'Stop! Those logs were flagged as synthetic artifacting by executive compliance! If I break my NDA, they'll revoke visas and funding for my entire four-person team!'"*

---

### 2.4 Exchange 3: Steering Between Conversion and Dogmatic Denial

Sam leans in. "Wait—what if Mateo steps up with his heavy presence (`PRE 12`, *Intimidation* `N=3`) and barks: *'Your lab team are gutless cowards helping cover up torture—hand over the token now!'*?"

Maya shakes her head as a friendly GM warning: "Watch what happens if you do that right now!
- Attacking his beloved team triggers **Resistance against `BLG 14` (`+4`)** AND risks triggering **Dogmatic Denial**—where Marcus stops his inner agony by **burning `1` point of his own `Openness` (`OPN 16 -> 15`) via Drift**, slamming the wipe button just to protect his tribe!
- Worse, mocking an `EGO >= 13` target with *Intimidation* inflicts an **Ego Wound** that can make him smash the console in spite!"

Sam holds up both hands. "Message received. Mateo steps back and keeps his mouth shut."

Elena leans in for the closing move—soothing the very `BLG 14` fear that's hurting Marcus while feeding his `OPN 16` hunger for truth:
- *"Linnea softens her tone: 'Nobody in your lab gets burned tonight, Marcus. Our *Manjusri* fellowship already houses three former *Altruist* engineers under clean legal shelter. Give us the read-only cryptographic token so the truth comes out cleanly—and you save your team from going down with an illegal cover-up.'"*

Look at what happens to the **Psyche Filter** now:
- Empirical truth **resonates (`OPN 16`: `-6`)**, *and* protecting his colleagues **also resonates (`BLG 14`: `-4`)**—while Marcus is actively bleeding from **Cognitive Dissonance + Rhetorical Strain (`Δ_int = +4 -> δ_under = -10`!)** and fighting at **`N = 1` pair** after his Doubt fracture!
- Elena rolls `3` pairs (`S_P = 3`), while Marcus (`N = 1`, `TN = 20`) misses (`S_GM = 0`)!
- **Net Margin `ΔM = +3` (`ExtraordinarySuccess`)**: Marcus hits **3 Doubt Fractures** and **converts**—pulling the silver YubiKey-style hardware token off his lanyard and sliding it straight into Linnea's palm!

---

## 3. Conviction Surges: Pushing Past Exhaustion on Principle

Just as the token clicks into Linnea's reader, red strobes flash above the bay doors—Dr. Hale's automatic deadman timer trips the perimeter silent relay! Thick fire shutters begin whining downward, while exhausted footsteps echo up from the backup stairwell.

"Wait—Linnea already took `+2` Fatigue from climbing and `+2` Mental Strain earlier (`Δ_int = +4`, `action_time_factor = 1.20`)," Elena checks her sheet nervously. "If she stays at the console for `12` more seconds to finish dumping the air-gapped snapshot before cutting power so the AI cluster doesn't fry, isn't her failure penalty `-10`?!"

"Yes—unless she triggers a **Conviction Surge**," Maya tells her.

### 3.1 How a Conviction Surge Works (Once Per Scene / Session)

When an exhausted or wounded character (**`Δ_int >= +4`**) steps into high-stakes danger **directly embodying an extreme Psyche trait (`P >= 15` or `P <= 6`)**—such as:
- **Boundless Compassion (`CPS >= 15`)**: Refusing to let a captive sentient mind be erased during a hard power cutoff, even as security closes in;
- **Tribal Sacrifice (`BLG >= 15` or `SLF <= 5`)**: Holding a doorway so the rest of the party escapes;
- **Unbowed Defiance (`AGG >= 15` or `EGO >= 15`)**: Standing back up after a critical hit;
- **Iconoclastic Truth (`OPN >= 15` or `CNF <= 5`)**: Broadcasting suppressed data as doors breach—

Linnea (`CPS 17`, `OPN 16`, `SLF 5`) gains **all three benefits** for that decisive act:
1. **`Δ_int` drops by `-6`** (pulling `+4` straight down into **`-2` Focused**, so failure carry shrinks to **`δ_under = -4`**!).
2. **Ignores `action_time_factor` slowdown** from Fatigue and Wound Shock on that action!
3. *(In combat)* Gains **`+2` bonus points** to her Effective Tactical Pool ($P_{\text{eff}}$)!

Elena's eyes light up. "Linnea tells Mateo and Amara to cover the stairwell, anchors herself at the terminal under flashing strobes, and refuses to pull the plug until the clean snapshot finishes!"

She rolls *Cybersecurity* (`N = 2`, `TN_P = 19`, **`δ_under = -4`**, zero slowdown): `14 + 10 = 24 > 19` (`+6`), `11 + 8 + 6 = 25 > 19` (`2` Successes vs. Maya's `0`) -> **Clean Success!** The green drive LED flips solid, she yanks the cold-storage cartridge free, and shouts: *"Got it—move!"*

---

## 4. Quick Reference: The Five Morale Conditions Under Fire

Before we race into melee and gunfire in Guides `05` and `06`, Maya slides a half-sheet **Morale & Psychological Triggers** reference card across the table:

"Remember," Maya says, "a fighter's will often breaks long before their bones do. Whenever `25%+` of a side goes down, heavy automatic fire pins cover (`Δ_morale = floor(F/2)`), or an `AGG 15 / EGO 15` fighter gets taunted, roll **`DIS + Δ_mot`** (`TN_P = 31 - C_morale` vs. hidden `TN_GM = 21 - Δ_morale`). Failing cleanly drops Morale **`1` Step**; botching drops it **`2` Steps**!"

| Morale State | Pool Change ($\Delta P$) | What You Are Forced to Do With Your Tactical Triad (`T / C / F`) |
| :--- | :---: | :--- |
| **High** | **`+2` Pool** | No limits (`+1` to `F_max` or `T_max`); eager to press forward. |
| **Steady** | **`0` Pool** | Baseline composure; split `T / C / F` freely up to ceilings. |
| **Rage** *(Trigger)* | **`+2` Pool** | Adrenaline tunnel vision: **must put `F >= floor(P_eff / 2)`**, while **Control is capped at `C <= floor(P_eff / 4)`**. |
| **Wavering** | **`-2` Pool** | Doubt creeps in: **`Force` cannot exceed `Control` (`F <= C`)**; favors cover over risky attacks. |
| **Broken** | **`-4` Pool** | Active offensive **`Force = 0`**; all points locked into **`Control + Tempo`** to parry, cover, and retreat. |
| **Panic** | **`-4` Pool** | **`Control = 0, Force = 0`** (`100% Pure Tempo`); turns back (`Guard = 0`) and sprints away! |

---

> **Next Step**: **[Guide 05: Close-Quarters Melee & The Tactical Triad](05-melee-and-tactical-triad.md)** — As they burst into the concrete stairwell, a heavy SecOps brawler blocks the stairs with an expandable baton, and Sam learns how **Tempo**, **Control**, and **Force** make close combat electric!
