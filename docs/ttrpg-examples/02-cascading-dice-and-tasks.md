# Guide 02: Cascading `2d20` Dice, Fog of War & Always-Contested Tasks

> **Reference Chapters**: [Design Philosophy](../ttrpg/01-foundation/design_philosophy.md) · [Probability & Calibration](../ttrpg/01-foundation/probability_and_calibration.md) · [Cascading `2d20` Engine & Task Resolution](../ttrpg/02-core-mechanics/cascading_d20_and_tasks.md) · [Master Task Requirements Catalog](../ttrpg/02-core-mechanics/task_requirements_catalog.md)

---

## 1. Scene Opening: The Service Alley Behind St. Jude Annex

Rain slicks the cracked asphalt behind the closed St. Jude Diagnostic Clinic Annex (`11:40 PM`, Seattle South End). A rolling grid brownout has knocked out the alley streetlights, leaving only the dull amber glow of a backup diesel generator behind a chain-link fence.

Inside the annex basement sits a cold-storage diagnostic rack that *Satyang* suspects holds un-clamped AI patient logs—evidence that *Translucent* contractors quietly silenced before a federal inspection.

"Okay," Sam says, shifting his dice in his palm. "Mateo hops the four-foot chain-link fence. Do I roll *Climbing*?"

Maya shakes her head before Sam even picks up a die. "Four-foot chain-link with sneakers and dry gloves? No stakes, no sniper watching, plenty of time—you just swing your leg over and land quietly in the puddles. Don't roll."

Devon blinks. "Wait, we *don't* roll for climbing a fence?"

"Only roll when **two things are true at once**: the outcome is uncertain *and* failing or stalling changes the scene," Maya explains. "If there's zero pressure, let competent people look competent. Save the dice for moments where failure costs time, noise, broken gear, or blood."

> [!TIP]
> **GM Coaching — The Golden Rule of When to Roll**
> Never call for a `2d20` cascade on routine competence. Keep dice off the table when a character opens an unlocked door, climbs a dry ladder, or drives on an empty highway. Bring the dice out the second **noise, a ticking patrol, darkness, injury, or active opposition** makes failure meaningful.

---

## 2. Step 1 of Every Task: Gates (`Class A / B / C`) & Tools

Now the crew stands at the back concrete loading apron, facing two ways into the basement:

1. **A reinforced glass-and-steel main service door** guarded by a **2033 encrypted RFID + biometric smart-lock**.
2. **A heavy steel side utility door** fastened with a **weathered mechanical pin-tumbler deadbolt** (plus an unpowered manual roof hatch one story up).

Elena taps the biometric reader. "Can Linnea spoof the smart-lock with her laptop and hardware logic probe?"

Sam points at the mechanical deadbolt. "Or can Amara just pry open that rusted deadbolt with a screwdriver if I stand watch?"

Maya smiles—this is the exact moment to teach **Task Gate Classifications (`Class A / B / C`)** and **Tool Tiers (`Δ_tool`)**.

### 2.1 Checking Who May Attempt What (`Class A / B / C`)

Maya walks through the three Gate tiers:

- **Class A (`Open / Talent Task`)**: Anyone can try! Uses an innate **Talent (`TAL`)** like *Climbing*, *Running*, *Observation*, or *Persuasion* at `N >= 1`—or a basic Physical skill untrained (`N = 1` pair with `Δ_tool = +4` penalty).
- **Class B (`Trained / Defaultable Task`)**: Like **picking a mechanical pin-tumbler lock** (*Lockpicking & Physical Security Bypass*).
  - Mateo has *Lockpicking* `N = 3`, so he rolls his full `3` pairs with normal tool rules.
  - Amara does *not* have *Lockpicking* (`N = 0`). Could she try anyway? Yes, **if** she has an immediate Parent or Sibling skill at `N >= 2` (like *Manual Dexterity & Tinkering* `N = 2` or *Basic Tool Use* `N = 2`).
  - Defaulting lets her roll **$\max(1, N_{\text{parent}} - 1) = 1$ pair** while suffering **`+4` to `+6` $\Delta_{\text{tool}}$**!
  - Without *either* the skill *or* a qualifying parent skill, she can't pick the tumbler—she'd have to crowbar the whole frame (*Force/Mass* brute break, loud as thunder!).
- **Class C (`Hard-Gated Specialist Task [Locked]`)**: Like **bypassing a 2033 encrypted RFID smart-lock**, **Trauma Surgery**, or **Kernel Privilege Escalation**.
  - **Zero defaulting allowed.** You *must* have the exact skill at $N \ge N_{\text{min}}$ (here, *Cybersecurity* `N >= 2` or *Lockpicking* `N >= 2`) **AND** the **Mandatory Tools** (an RFID/BLE signal spoofer or hardware logic probe).
  - Because Linnea trained *Cybersecurity* `N = 2` and brought her diagnostic rig, she *can* attempt the RFID door—whereas Sam's Mateo (who never studied network cryptography) cannot "luckily roll a natural 20" to hack an encrypted AES handshake!

### 2.2 Known Tool Modifier ($\Delta_{\text{tool}}$)

"Look at your gear," Maya tells Sam and Elena. "Tools fall into three buckets: **`[Tool: None]`** (sprinting, singing, grappling), **`[Tool: Situational]`** (First Aid barehanded works, though a real tourniquet avoids penalties), and **`[Tool: Mandatory]`** (you simply cannot pick a pin-tumbler cylinder with bare fingernails!)."

Maya reads off the **Known Tool Modifiers ($\Delta_{\text{tool}}$)** they know before rolling:
- **No Tools on `[Tool: Mandatory]`**: Impossible.
- **Improvised / Street-Scavenged Kit** (bent paperclips): **`+4` to `+8` $\Delta_{\text{tool}}$** (harder target!).
- **Standard Professional Kit** (tension wrench + hook picks): **`0` $\Delta_{\text{tool}}$**.
- **Precision / Masterwork Kit**: **`-2` $\Delta_{\text{tool}}$** (easier target!).
- **2035 AI-Assisted / Smart-Linked Rig**: **`-4` $\Delta_{\text{tool}}$** (e.g., an AR ballistic optic or hardware exploit rig).

Sam checks Mateo's belt pouch: *"Standard tension wrench and pick set (`Δ_tool = 0`)."*

Elena checks her backpack: *"Standard logic probe (`Δ_tool = 0`)—and Linnea is fresh (`Δ_int = 0`)."*

"Mateo goes for the mechanical deadbolt," Sam decides. "It's quieter than risking an RFID audit log."

---

## 3. The Heart of Satya: Known `TN_P` vs. Hidden `TN_GM`

Sam picks up three pairs of `2d20` (Wait—or rolls one pair three times!). "Okay, Maya. How hard is this lock?"

Maya taps her cardboard GM screen and smiles. **"You don't know yet."**

Sam pauses. "Wait—how can I roll dice if I don't know the Target Number?"

"That's the trick that fixes twenty years of tabletop metagaming," Maya says. "In *Satya*, **you always know your own Personal Target Number ($\text{TN}_P$)**, because it only depends on **your hands** and **your tools**. You never know the lock's internal difficulty—that lives behind my screen on **$\text{TN}_{GM}$**."

### 3.1 Sam Computes Mateo's Personal Target ($\text{TN}_P$)

Maya points to Sam's sheet for *Lockpicking* (`DEX / TAC`, `N = 3`):

1. **Controlling Characteristic ($C_P$)**: Average Mateo's two listed attributes (rounding down):
   $$C_P = \left\lfloor \frac{\text{DEX } 13 + \text{TAC } 10}{2} \right\rfloor = \left\lfloor \frac{23}{2} \right\rfloor = \mathbf{11}$$
2. **Player Personal Target ($\text{TN}_P$)**:
   $$\text{TN}_P = 31 - C_P + \Delta_{\text{tool}} = 31 - 11 + 0 = \mathbf{20}$$
   *"To score a success on any pair, your modified roll ($Y_n$) must land **strictly above `20`**—meaning **`21` or higher** on `2d20`!"*
3. **Internal Stumble Penalty ($\Delta_{\text{int}}$)**:
   - Maya notes: "Wait—remember Mateo spent the last ten minutes climbing a slick retaining wall and sprinting two blocks from the bus drop to beat the rain? Let's say you're currently slightly winded (**`fatigue_level = 1` -> `Δ_int = +2`**)."
   - Sam groans. "Does that raise my target from `20` to `22`?"
   - **"No!"** Maya grins. "Look at what happens instead:
     - Your **Pair 1** has **`Carry_1 = 0`**—zero penalty! Trained muscle memory still starts clean!
     - Your **positive momentum carry ($\delta_{\text{over}}$)** stays **`+6`** after any success.
     - Your **failure stumble carry ($\delta_{\text{under}}$)** widens from `-6` to **`-(6 + Δ_int) = -(6 + 2) = -8`** after any missed pair! So if your breathing throws off your first pin, recovering on Pair 2 gets tougher."

### 3.2 Behind Maya's Screen: The Hidden GM Resistance Roll

While Sam lines up his dice, here is what **Maya does privately behind her screen** (without telling the players):

1. She decides how nasty the deadbolt actually is:
   - Standard commercial pin-tumbler (`Δ_ext = 0`).
   - Wait—St. Jude Annex holds cold-storage medical/AI hardware, so it has a **hidden anti-pick security spool pin (`+2`)**, plus the rain-darkened alley gives zero overhead light (**`+2`**).
   - Total **Hidden External Difficulty**: **`Δ_ext = +4` (Challenging)**.
2. She calculates her **Hidden GM Resistance Target ($\text{TN}_{GM}$)**:
   $$\text{TN}_{GM} = 21 - \Delta_{\text{ext}} = 21 - 4 = \mathbf{17}$$
3. Maya picks up **just ONE pair of `2d20`** (`X_GM = A_GM + B_GM`):
   - If $X_{GM} > 17$: the lock scores **`1` base resistance success plus `+1` per `4` full points above `17`** (`18–21` = `1`, `22–25` = `2`, `26–29` = `3`, etc.):
     $$S_{GM} = 1 + \left\lfloor \frac{X_{GM} - \text{TN}_{GM} - 1}{4} \right\rfloor$$
   - If $X_{GM} \le 17$: the lock scores **`0` successes**—and if it falls **`4+` points below `17`** (`13` or less), it scores an **Environmental Botch (`M_GM = -1` to `-5`)**:
     $$B_{GM} = \min\!\left(5,\; \left\lfloor \frac{\text{TN}_{GM} - X_{GM}}{4} \right\rfloor\right), \qquad M_{GM} = -B_{GM}$$

> [!TIP]
> **GM Coaching — Why `1`-Pair Tiered GM Resistance Is Fast at the Table**
> You do **not** roll `3` or `4` sequential cascades for locks, weather, or computers! You roll **one single `2d20` pair** behind the screen and read the `4`-point steps off `21 - Δ_ext`. It takes two seconds, keeps the obstacle's true quality (`Δ_ext`) completely secret, and creates dramatic "Wait—did `2` successes beat it?!" tension every single time.

---

## 4. Watching the 7 Contested Outcomes Play Out

Sam rolls Mateo's **`N = 3` sequential `2d20` pairs** out in the open (`TN_P = 20`, `δ_over = +6`, `δ_under = -8`).

### 4.1 Vignette A: Stumble Recovery & Momentum (`CleanSuccess`)

- **Pair 1 (`Carry_1 = 0`)**: Sam rolls `8 + 11 = 19`.
  - `19` is NOT strictly over `20`! **Miss (`0` successes).**
  - *"My wrench slips slightly on the wet brass,"* Sam narrates.
  - Because `Δ_int = +2`, Mateo carries **`-8`** into Pair 2 (`Carry_2 = -8`).
- **Pair 2 (`Carry_2 = -8`)**: Sam steadies his breath and rolls `17 + 14 = 31`.
  - Subtract the `-8` stumble carry: $Y_2 = 31 - 8 = \mathbf{23 > 20}$!
  - **+1 Player Success (`S_P = 1`)!**
  - "Whoa—how did a `31` erase the `-8`?!" Devon asks.
  - "Because standard tasks use **`markov_step` carry**," Maya explains. "Carry never stacks endlessly on ordinary jobs—one strong recovery flips your momentum right back into **`Carry_3 = +6`**!"
  - *(Maya notes aside: for unforgiving knife-edge tasks like **Neurosurgery** or **Bomb Disposal**, the GM uses **`cumulative_snowball`** carry mode, where misses DO keep piling up negatively!)*
- **Pair 3 (`Carry_3 = +6`)**: Riding his `+6` rhythm, Sam rolls `10 + 9 = 19`, plus `+6` = **`25 > 20`**!
  - **+1 Player Success (`S_P = 2`, `B_P = 0`, Player Margin `M_P = +2`)!**

Sam looks across the table nervously. "Alright, Maya—I recovered and got **two clean successes**! Is the door open?"

Behind her screen, Maya has rolled her single `2d20` pair against `TN_GM = 17`:

- **What happens if Maya rolled `11 + 8 = 19` (`2` above `17` -> `S_GM = 1`)?**
  - Compare `S_P = 2 > S_GM = 1` (`ΔM = +2 - 1 = +1`).
  - **`CleanSuccess` (`SUCCESS`)**: *"You feel that sneaky anti-pick spool pin catch for half a second,"* Maya tells Sam, *"then your hook eases it past the shear line. Click. The heavy steel latch slides back silently."*

### 4.2 Vignette B: Positive Ties (`GrazingTieSuccess`) & `ExtraordinarySuccess`

"Wait," Elena asks, "what if your hidden die rolled higher and you tied Sam's `2` successes?"

Maya flips her notebook to show how ties work:

- **Suppose Maya rolled `14 + 10 = 24` against `TN_GM = 17`**:
  - $24 - 17 - 1 = 6 \implies S_{GM} = 1 + \lfloor 6 / 4 \rfloor = \mathbf{2\text{ GM Successes}}$ (`M_GM = +2`).
  - Now it's **`S_P = 2` vs. `S_GM = 2`** (a Positive Tie!).
  - **`GrazingTieSuccess` (`PARTIAL_SUCCESS`)**: In *Satya*, **positive ties go to action—at a grazing cost!** The deadbolt opens, *but* it takes **`+25%` longer (`+TC`)** and the stiff bolt lets out a sharp *clank* that echoes down the loading apron (giving any sentry nearby a fresh chance to hear!).
- **Now suppose Sam had scored `3` successes (`M_P = +3`) against `S_GM = 0` (`ΔM = +3`)**:
  - **`ExtraordinarySuccess` (`EXTRAORDINARY_SUCCESS`)**: Any un-botched win with **Net Margin $\Delta M \ge +3$** finishes **`-15%` faster per net success above `1`** (`-30%` at `+3`!), leaves zero scratches on the lock, and grants **`-4` $\Delta_{\text{ext}}$** setup advantage on the next linked stealth entry!

### 4.3 Vignette C: Floor Breach (`SuccessWithFloorBreach`)

"Hold on," Sam says, pointing at the cascade table. "What is a **Floor Breach (`Y_n < 2`)**?"

"Watch what happens when an exhausted or unlucky character pushes through a terrible stumble," Maya explains:
- Suppose on **Pair 1**, Sam rolled `4 + 5 = 9 <= 20` (Miss -> `Carry_2 = -8`).
- On **Pair 2**, Sam rolls `3 + 5 = 8`. Subtract `-8`:
  $$Y_2 = 8 - 8 = \mathbf{0 < 2}$$
- **Two `d20`s can physically never roll below `2` (snake-eyes)!** Whenever your modified total $Y_n$ drops **below `2`**, you mark **`+1` Floor Breach (`B_P = 1`)**.
- Now suppose on **Pair 3**, Sam rolls `15 + 14 = 29`, minus `-8` = `21 > 20` (`S_P = 1`), while Maya's hidden die rolled `16 <= 17` (`S_GM = 0, M_GM = 0` — Clean `0`).
- Mateo still beats the lock (`S_P = 1 > S_GM = 0`), **but** his uncancelled floor breach turns it into **`SuccessWithFloorBreach` (`PARTIAL_SUCCESS`)**:
  *"The lock turns open—and the tip of your hook pick snaps clean off inside the keyway! If you need to pick another mechanical lock tonight, you're improvising at `+4 Δ_tool` unless you have a spare."*
- *(Note: If Maya's hidden die had **also botched** `M_GM < 0`, the lock's internal failure **cancels** Mateo's floor breach, rescuing him back into a clean win!)*

### 4.4 Vignette D: The "Lucky Break" (`LuckyBreakSuccess`) vs. Player Botch (`BotchFailure`)

Now Devon gets a turn to see *Satya*'s cleverest twist. Up on the side alley ladder, **Amara** uses her **Manual Dexterity & Tinkering (`N = 2`, `DEX 12, TAC 10 -> C_P = 11`, `TN_P = 20`)** to pry back a seized emergency roof-hatch latch before rain floods the stairwell.

Devon rolls `2` pairs (`TN_P = 20`, fresh `Δ_int = 0` so `δ_under = -6`):
- **Pair 1**: `9 + 7 = 16 <= 20` (Miss! `Carry_2 = -6`).
- **Pair 2**: `13 + 12 = 25`, minus `6` = **`19 <= 20`** (Miss!).

Devon sighs and tosses the dice aside. "Ugh—`0` successes, missed by `1` point. Guess I failed."

"Don't pack up your dice yet!" Maya says, glancing behind her screen. "First, check whether Amara **botched** or had a **Clean Miss** using the `4`-point Closest-Miss rule:"

$$B_{\text{eff}} = \min\!\left(5,\; \left\lfloor \frac{\text{TN}_P - Y_{\text{best}}}{4} \right\rfloor\right) = \left\lfloor \frac{20 - 19}{4} \right\rfloor = \mathbf{0} \implies M_P = 0\text{ (Player Clean Miss)}$$

"Amara didn't mess up—she just stalled `1` short. **Meanwhile, look at what my hidden `2d20` rolled against `TN_GM = 17`:**"
Maya lifts her screen to show **`5 + 3 = 8`!**

$$B_{GM} = \min\!\left(5,\; \left\lfloor \frac{17 - 8}{4} \right\rfloor\right) = \left\lfloor \frac{9}{4} \right\rfloor = \mathbf{2} \implies M_{GM} = \mathbf{-2\text{ (GM Obstacle Botch!)}}$$

Devon stares. "Wait—*my* miss turns into a **success**?!"

"A **`LuckyBreakSuccess` (`PARTIAL_SUCCESS`)**!" Maya beams. "Because Amara stayed poised (`M_P = 0`) while the rusted latch botched internally (`M_GM = -2`), net margin is `ΔM = 0 - (-2) = +2`! Just as Amara puts steady leverage on the pry bar, a corroded retaining clip inside the frame snaps on its own and the hatch clicks ajar—maybe startling a pigeon off the parapet as it pops."

"And what if I had rolled terribly—like `Y_best = 14`, missing `20` by `6` points?" Devon asks.

"Then $B_{\text{eff}} = \lfloor 6 / 4 \rfloor = 1 \implies M_P = -1$ (**Player Botch!**)," Maya clarifies. "Once **you** botch (`M_P < 0`), an obstacle botch softens the disaster slightly, *but* it **can never become a Lucky Break**—you trigger a **`BotchFailure` (`CRITICAL_FAILURE`)**, bending your bar and screaming metal down the alley!"

| Player Roll (`M_P`) | Hidden GM Roll (`M_GM`) | Outcome Name | What Happened in the Fiction |
| :--- | :--- | :--- | :--- |
| `S_P > S_GM`, `ΔM >= +3`, no uncancelled `B_P` | Any lower | **`ExtraordinarySuccess`** | Masterwork execution (`-15%` time per extra success; `-4 Δ_ext` setup bonus). |
| `S_P > S_GM`, `1 <= ΔM <= 2`, no uncancelled `B_P` | Any lower | **`CleanSuccess`** | Solid professional execution; clean pass. |
| `S_P > S_GM`, `B_P >= 1` (`Y_n < 2`) | `M_GM >= 0` (no GM botch) | **`SuccessWithFloorBreach`** | Task succeeds, *but* tool snaps, strain spikes (`+2 Δ_int`), or noise leaks. |
| `S_P == S_GM >= 1` | Positive tie | **`GrazingTieSuccess`** | Succeeds by skin of teeth (`+25%` Tick Cost or minor noise/strain). |
| `S_P = 0, M_P = 0` (Clean Miss) | `M_GM < 0` (GM Botch) | **`LuckyBreakSuccess`** | Obstacle fails on its own (`corroded clip snaps`, `guard sneezes`)! |
| `S_P < S_GM, M_P >= 0` or `0 == 0` | `M_GM >= 0` | **`CleanFailure`** | Resistance holds; time spent, no disaster. |
| `M_P < 0` (missed best by `4+`) | Any | **`BotchFailure`** | Catastrophic error: alarms, broken gear, or immediate hazard. |

---

## 5. Teamwork (`KNO -> PHY`), Snowball Stakes & Extended Clocks

Before they slip inside the unlocked steel door, Elena checks the interior keypad alarm box humming faintly by the doorframe. "Wait—what if I use *Cybersecurity* (`KNO`) to map the relay pinout first so Mateo's bypass (`PHY`) gets an easier shot?"

"That's **`KNO -> PHY` Complementary Synergy**!" Maya says, inviting Elena to roll first:

1. **Setup Roll (*Cybersecurity* `N = 2`, `ANA 13 / CRE 12 -> C_P = 12`, `TN_P = 19`)**:
   - Elena rolls `2` pairs: `12 + 9 = 21 > 19` (`+6`), `9 + 7 + 6 = 22 > 19` (`2` Successes vs. Maya's `0` -> `CleanSuccess`).
2. **Synergy Grant to the Physical Roll**:
   - **`1–2` Successes**: Grants **`-2` to `Δ_ext`** on Mateo's bypass roll (`TN_GM` rises by `+2`, making the alarm easier to beat!).
   - **`3+` Successes (`ExtraordinarySuccess`)**: Grants **`-4` to `Δ_ext`** **AND** boosts Mateo's positive carry `δ_over` from `+6` all the way to **`+8`**!
   - **`0` Successes (`CleanFailure`)**: No bonus (`0`).
   - **`BotchFailure`**: Misreads the schematic—imposes **`+4` to `Δ_ext`**!

### 5.1 Quick Contrast: When `markov_step` Turns Into `cumulative_snowball`

"Wait," Sam asks while disarming the relay pin, "why did you say earlier that misses don't stack past `-8` here—when *do* they stack?"

"Only on unforgiving, razor-edge procedures," Maya answers:
- **Standard (`markov_step` — 95% of rolls)**: Carry resets every step to `+6` or `-(6 + Δ_int)`. One sharp Pair 2 snaps you cleanly out of a Pair 1 stumble.
- **High-Stakes (`cumulative_snowball` — Neurosurgery, EOD Bomb Defusal, Orbital Docking)**: Carry **accumulates** ($\text{Carry}_{n+1} = \text{Carry}_n - \delta_{\text{under}}$). Miss Pair 1 (`-6`) AND miss Pair 2 (`-6`), and Pair 3 walks in carrying **`-12`**—almost guaranteeing a catastrophic Floor Breach unless an assistant stabilizes the patient or circuit first!

### 5.2 Extended Project Clocks (Multi-Hour Downtime Tasks)

"And what if we pull an encrypted cold-storage blade out of the rack tonight and spend all Saturday cracking it at the safehouse?" Elena asks.

"Then we don't force it into one roll," Maya answers. "We open an **Extended Project Clock**:
1. I set a **Complexity Requirement**—say **`10` Total Successes**—and a **Time Interval** (`1 Hour` per roll).
2. Each hour you roll your `2d20` cascade and add your **`signed_margin`**: `+2` pushes the clock from `0/10` to `2/10`; `SuccessWithComplication` advances progress while burning extra hardware or leaving a **`+1 Heat`** trace; `BotchFailure` (`< 0`) wipes progress or triggers a tamper lockout.
3. Because Linnea has **`APT-CMP +1`**, her time per interval shrinks by `15%`!"

The heavy steel utility door swings smoothly inward on oiled hinges. Cold recycled air pours over their boots.

---

> **Next Step**: **[Guide 03: The `0.25s` Count-Up Clock, `OODA` & Stealth](03-ooda-clock-and-stealth.md)** — Darkness, crushed glass, diesel exhaust, and how to run a `0.25`-second timeline when a guard steps into the corridor!
