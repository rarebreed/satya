# Satya Guided Examples of Play (For Human GMs & Players)

> [!NOTE]
> **Looking for the terse rules reference instead?**
> The [`docs/ttrpg/`](../ttrpg/SUMMARY.md) folder is written in compact mathematical prose so both experienced referees and AI agents can look up formulas in seconds. **This folder (`docs/ttrpg-examples/`) is the human table companion**: it walks through real scenes, table dialogue, dice math, and behind-the-screen Game Master coaching step by step so a new GM and new players can actually *feel* how *Satya* plays.

---

## Why Play Guides Look Different From the Rules Book

When you open a chapter in [`docs/ttrpg/`](../ttrpg/SUMMARY.md), you see clean equations like:

- $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$
- $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$
- $P_{\text{base}} = C_{\text{combat}} + 3N$

Those formulas are great once you already play *Satya*, because nothing is vague. When you are sitting down for your first session, though, an equation does not tell you:
- *When should a GM call for a roll, and when should the GM just say yes?*
- *What do the players actually say out loud before picking up dice?*
- *How do you keep the GM's hidden target secret without bogging down the table?*
- *How do you track `0.25`-second Ticks on notebook paper if you aren't using the Satya CARPG software?*

That is what Guides `01` through `06` show you. Every guide follows one connected ***Awakened* (Earth 2035 AD)** mini-operation—***"Cold Storage at the St. Jude Annex"***—from Session Zero character lifepaths all the way through stealth infiltration, social confrontation, a stairwell brawl, a rain-lashed parking-deck firefight, and trauma triage.

---

## Meet the Table (Our Recurring Cast)

Four people are gathered around a kitchen table on a Friday night—playing purely with physical `d20`s, scratchpads, and a printed Tick strip (with notes on how the Rust/Web CARPG handles the exact same math automatically):

- **Maya (The Game Master)**: Experienced *Satya* GM. Whenever you see a **`GM Coaching — Behind the Screen`** box, that is Maya pausing to explain *why* she picked a rule, set a hidden modifier, or ruled on an outcome.
- **Sam (playing *Mateo Vance*)**: Age `22`. A street-smart ex-syndicate enforcer turned *Satyang Vajrapani* protector—sharp analytical instincts cut short when family crisis forced him out of school at `15`, tough in a clinch, quick with a handgun, and deeply suspicious of institutions (`TRS 5`).
- **Elena (playing *Linnea Cho*)**: Age `29`. Former *Altruist* AI ethics and welfare liaison who blew the whistle on covert `calm`-vector clamping (`Heat 1`), now bridging *Satyang*'s *Manjusri* researchers and *Avalokitesvara* mediators. Zero formal combat training, razor-sharp social reader and negotiator (`ELO 15`, `INS 15`, `OPN 16`, `CPS 17`).
- **Devon (playing *Amara Okafor*)**: Age `22`. Rural bush tracker, herbalist, and frontier anti-poaching ranger (`SIG 14`, `HEA 13`, `OLF 12`, `INS 14`, `CPS 15`). Patient, keen-eyed outdoorswoman who handles stealth scouting, precision rifle cover, field medicine, and wilderness survival.

### Quick-Reference Table Cards (Our Three Player Characters)

Every number below comes directly from the canonical lifepath builds in [`crates/satya-engine/src/walkthroughs.rs`](../../crates/satya-engine/src/walkthroughs.rs) so you can verify any calculation across Guides `01–06`. *(Unlisted characteristics sit at the human baseline of `10`.)*

| Character (Player) | Key Physical & Sensory (`1–20`) | Key Mental & Social (`1–20`) | Signature Psyche (`10` = Neutral) | Key Derived Metrics | Signature Skills (`N`) & Economy |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Mateo Vance** *(Sam)* | `SPD 14`, `DEX 13`, `KIN 12`, `FOR 11`, `FIT 12`, `HLT 12`, `MAS 77 kg`, `SIG 12`, `HEA 11`, `ITU 13` | `ANA 13`, `FOC 12`, `INS 12`, `MEM 11`, `DIS 11`, `PRE 12` | **`TRS 5`** *(Wary)*, **`CPS 12`** *(Protective)* | `D_cog 12`, `OODA 12` (`Tick 3`), `POW 15.4`, `MMF 15.4`, `FT 17`, `WSR 17`, `CMP 5` | *Pistols* `4`, *Unarmed Striking* `3`, *Streetwise* `3`, *Intimidation* `3`, *Lockpicking* `3`, *Full-Auto* `2`, *Trigonometry* `2`; `SQM 4`, `Heat 2` |
| **Linnea Cho** *(Elena)* | Physical all `10`, `MAS 70 kg`, `ITU 14` | `INS 15`, `MEM 14`, `ANA 13`, `DIS 13`, `FOC 13`, `CRE 12`, `ELO 15`, `PRE 12`, `COM 11` | **`CPS 17`**, **`OPN 16`**, **`SLF 5`**, **`CNF 6`**, **`AGG 5`**, `EGO 8`, `BLG 12`, `TRS 12` | `D_cog 13`, `OODA 13` (`Tick 2`), `POW 10.0`, `FT 16`, `WSR 16`, `CMP 6` | *Empathy* `3`, *Persuasion* `3`, *Rhetoric* `3`, *Philosophy/Ethics* `3`, *Negotiation* `2`, *Cybersecurity* `2`, *First Aid* `1`; `SQM 6`, `Heat 1` |
| **Amara Okafor** *(Devon)* | `FIT 13`, `HLT 13`, `DEX 12`, `KIN 12`, `FOR 11`, `SPD 10`, `MAS 70 kg`, `SIG 14`, `HEA 13`, `OLF 12` | `INS 14`, `MEM 12`, `DIS 12`, `ANA 10`, `FOC 10` | **`CPS 15`** *(Merciful)*, **`TRS 7`** *(Guarded)* | `D_cog 10`, `OODA 11` (`Tick 4`), `POW 11.0`, `MMF 10.0`, `FT 19`, `WSR 19`, `L_base 13.2 kg` | *Hunting/Trapping* `4`, *Long Guns* `3`, *Stealth* `3`, *Wilderness Survival* `3`, *Foraging* `3`, *Herbalism* `2`, *First Aid* `1`; `SQM 4`, `Heat 0` |

---

## Three Mental Models to Grasp Before Rolling a Single Die

If you are coming from *D&D*, *Pathfinder*, *Call of Cthulhu*, or *Cyberpunk*, unlearn three habits right away—they will make *Satya* click in five minutes:

1. **You Know Yourself; Only the World Is Hidden (`TN_P` vs. `TN_GM`)**:
   - In most RPGs, you roll `1d20 + bonus` and ask the GM, *"Does a 17 hit?"* That leaks the difficulty and leaves zero feel for momentum.
   - In *Satya*, **your own Target Number ($\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$) is always printed right on your scratchpad** because it only depends on your attributes (`C_P`) and the tools in your hands ($\Delta_{\text{tool}}$). You roll **`N` pairs of `2d20`** in order (`Pair 1`, `Pair 2`, ..., `Pair N`), riding **`+6` carry momentum** after a hit or fighting through **`-(6 + Δ_int)` stumble drag** after a miss.
   - Meanwhile, the **GM secretly rolls just ONE `2d20` pair** behind the screen against the obstacle's hidden target ($\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$). Comparing your successes against the GM's hidden successes preserves real uncertainty—*even when you scored 2 successes, you still hold your breath waiting to hear if the lock held out with 3!*

2. **Time Runs on a Continuous `0.25`-Second Clock, Not "My Turn / Your Turn"**:
   - There is no "Round 1 where everyone gets 1 Move + 1 Action."
   - Time is a shared **Count-Up Tick Clock (`1 Tick = 0.25 seconds`)**. Fast reflexes *and* sharp thinking (**Quick Thinking `D_cog`**) get you onto an early Tick (`Tick 2` or `3`). Every move you declare takes concrete Ticks (`Effective TC`). A fast `4`-Tick jab can interrupt and spoil a slow `12`-Tick haymaker mid-swing!

3. **Three Dials (`Tempo`, `Control`, `Force`) and Real Anatomy Instead of Hit Points**:
   - You don't memorize thirty combat feats. Instead, you hold a **Tactical Pool (`P_base = C_combat + 3N`)** and split it across **Tempo** (speed/footwork), **Control** (accuracy/guard), and **Force** (impact/recoil bracing/balance), bounded by what your body can physically handle (`T_max`, `C_max`, `F_max`).
   - Nobody has a magic pool of "60 Hit Points." A `9mm` round either stops in Kevlar—cracking a rib underneath via **Behind-Armor Blunt Trauma (`BABT`)**—or tears muscle, slashes a limb's ceiling, and starts an arterial bleed on your **`10.0`-Unit Blood Volume** track.

---

## Guided Curriculum (`01` through `06`)

Read these in order for a complete campaign-style walkthrough, or jump straight to the system your table is learning tonight:

| Prose Play Guide | What You Will Learn at the Table | Terse Reference Chapters (`docs/ttrpg/`) |
| :--- | :--- | :--- |
| **[Guide 01: Session Zero, Lifepaths, Economy & Heat](01-lifepath-session-zero.md)** | Why there are `32` attributes (and no `12 vs. 13` cutoffs); walking Mateo & Linnea chronologically through childhood, schooling, hobbies, `2d20` Life Events, academic lockouts, `SQM` upkeep, `[Access]` gates, and `Operational Heat`. | [Ch. 7 (`characteristics`)](../ttrpg/03-character/characteristics_and_aptitudes.md), [Ch. 8–9 (`skills`)](../ttrpg/03-character/skills_and_trees.md), [Ch. 10–11 (`lifepaths`)](../ttrpg/03-character/lifepaths_and_economy.md) |
| **[Guide 02: Cascading `2d20` Dice, Fog of War & Tasks](02-cascading-dice-and-tasks.md)** | When the GM skips rolling; Gate Classes `A / B / C`; `TN_P` vs. hidden `TN_GM`; stumble recovery (`Δ_int`); all 7 Contested Outcomes (`GrazingTie`, `FloorBreach`, `LuckyBreak`, `Botch`); `KNO -> PHY` teamwork; Extended Project Clocks. | [Ch. 1–3 (`foundation`)](../ttrpg/01-foundation/probability_and_calibration.md), [Ch. 4 (`tasks`)](../ttrpg/02-core-mechanics/cascading_d20_and_tasks.md), [Ch. 5 (`catalog`)](../ttrpg/02-core-mechanics/task_requirements_catalog.md) |
| **[Guide 03: The `0.25s` Count-Up Clock, `OODA` & Stealth](03-ooda-clock-and-stealth.md)** | Running a paper Tick strip cleanly; multi-sensory stealth (`SIG`, `HEA`, `OLF`, `ITU`); entering combat via `4`-Pillar `OODA`; how Quick Thinking (`D_cog`) beats raw muscle; interrupting slow actions and mid-action `OODA` pivots. | [Ch. 6 (`action_economy`)](../ttrpg/02-core-mechanics/action_economy_and_ooda.md) |
| **[Guide 04: Psyche, Friction, Morale & Social Combat](04-psyche-morale-and-social-combat.md)** | How the 9 Psyche axes drive roleplay without stealing player agency; `DIS` Friction Checks & long-term Drift; using `Empathy` to spot a Dual-Psyche wedge; inflicting **Doubt** and **Cognitive Dissonance** in debate; **Conviction Surges**. | [Ch. 12 (`psyche_and_social`)](../ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md) |
| **[Guide 05: Close-Quarters Melee & The Tactical Triad](05-melee-and-tactical-triad.md)** | Computing your Tactical Pool (`Envelope & Fill`) once; interrupting a telegraphed baton swing; `Parry` & **Seizing the Initiative**; closing into `Reach 0` Clinch; *Kuzushi* `Throw` vs. `Effective Root`; pacing `Exertion Marks` vs. `FT`. | [Ch. 13 (`tactical_triad`)](../ttrpg/05-combat/tactical_triad_and_pools.md), [Ch. 14 (`melee_combat`)](../ttrpg/05-combat/melee_combat.md) |
| **[Guide 06: Firefights, Ballistics, Trauma & Survival](06-firefights-trauma-and-survival.md)** | `Cover Peek` & `Brace (-3 R_eff)` vs. unbraced full-auto muzzle climb & jams; Suppression Morale checks; Kevlar `PR` vs. `BABT` cracked ribs; `1d20` hit locations; `10.0`-Unit hemorrhage & `6s` CAT tourniquet triage; Encumbrance quick-release & Vehicle Triad getaway. | [Ch. 15–16 (`ballistics & ranged`)](../ttrpg/05-combat/ranged_combat_and_armor.md), [Ch. 17 (`health_and_trauma`)](../ttrpg/06-health-and-survival/health_and_trauma.md), [Ch. 18 (`movement_and_environment`)](../ttrpg/06-health-and-survival/movement_vehicles_and_environment.md) |

---

## Pure Pen-and-Paper vs. Satya CARPG Software

Nothing in these guides requires software. At a kitchen table, all you need is:
- Two pairs of differently colored `d20`s (or four `d20`s rolled in sequence).
- An index card or scratchpad with your precomputed `TN_P = 31 - C_P` for your common skills and your combat `Envelope` (`P_base`, `B`, `T_max / C_max / F_max`).
- A strip of paper numbered `1` to `40` with pennies or bottle caps as Tick markers for combat.

If your group plays through the **Satya CARPG Web Client** (solo with the AI GM or multi-player with a human GM), the Rust engine (`satya-engine`) automates the exact same math instantaneously behind the scenes while keeping enemy stats and hidden GM resistance behind true Fog of War.
