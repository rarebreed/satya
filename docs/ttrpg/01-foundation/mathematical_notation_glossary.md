# Mathematical Notation, Variables & Formula Glossary

Because **Satya** is a Computer-Assisted RPG (CARPG) built on an exact combinatorial and physics-grounded simulation engine, the rules chapters use consistent mathematical notation across task resolution, probability distributions, character attributes, lifepaths, action economy, combat pools, and survival mechanics.

This chapter serves as the **single canonical glossary of every mathematical symbol, subscript, acronym, and formula** used throughout the Satya TTRPG Rules Book. Whenever you encounter a variable such as $\text{TN}_P$, $\text{TN}_{GM}$, $\delta_{\text{under}}$, $D_{\text{cog}}$, or $P_{\text{eff}}$ in any chapter, refer here for its full name, scale, and definition.

---

## 1. Universal Subscript & Prefix Conventions

Across all equations in Satya, subscripts and prefixes follow strict naming rules so you can immediately tell *who* or *what* a variable belongs to:

| Prefix / Subscript | Meaning | Examples |
| :--- | :--- | :--- |
| **Subscript $P$ (`_P`)** | **Player (Active Character)** — Refers to the player character (or active initiator) attempting a task or contested exchange. | $\text{TN}_P$ (*Target Number of the Player*), $C_P$ (*Controlling Characteristic of the Player*), $N_P$ (*Skill Level of the Player*), $S_P, B_P, M_P$ (*Player Successes, Botches, Signed Margin*). |
| **Subscript $GM$ (`_GM`)** | **Game Master (Hidden Resistance / Environment)** — Refers to the Game Master's hidden opposing roll representing environmental difficulty, lock/system security, or hazard virulence. | $\text{TN}_{GM}$ (*Target Number of the Game Master*), $C_{GM}$ (*Equivalent Controller of the GM*), $X_{GM}$ (*GM's `2d20` Roll Sum*), $S_{GM}, B_{GM}, M_{GM}$ (*GM Successes, Botches, Signed Margin*). |
| **Subscript $NPC$ (`_NPC`)** | **Non-Player Character (Active Opponent)** — Refers to an active NPC opponent rolling a full opposing cascade in combat, stealth, or social duels. | $\text{TN}_{NPC}$ (*Target Number of the NPC*), $C_{NPC}$ (*NPC Controlling Characteristic*), $N_{NPC}$ (*NPC Skill Level*). |
| **Subscript $n$ (`_n`)** | **Cascade Step Index ($n \in \{1, \dots, N\}$)** — Refers to the $n$-th sequential `2d20` dice pair in a multi-pair skill cascade. | $A_n, B_n$ (*Die A and Die B on step $n$*), $X_n$ (*Raw pair sum on step $n$*), $\text{Carry}_n$ (*Carry-over modifier into step $n$*), $Y_n$ (*Modified pair total on step $n$*). |
| **Subscript $\text{eff}$ (`_eff`)** | **Effective Value** — The net value of a target number, skill rank, pool, or time cost after all situational modifiers, wounds, or synergies are applied. | $\text{TN}_{\text{eff}}$ (*Effective Uncontested Target Number*), $N_{\text{eff}}$ (*Effective Dual-Skill Rank*), $P_{\text{eff}}$ (*Effective Tactical Pool*), $\text{TC}_{\text{eff}}$ (*Effective Tick Cost*). |
| **Subscript $\text{base}$ (`_base`)** | **Baseline (Unmodified) Value** — The starting value before situational, fatigue, or trauma modifiers are applied. | $\delta_{\text{base}} = 6$ (*Baseline Failure Carry*), $P_{\text{base}}$ (*Base Tactical Pool*), $\text{TC}_{\text{base}}$ (*Base Tick Cost*). |
| **Subscript $\min / \max$** | **Minimum Floor / Maximum Ceiling** — A hard lower requirement or upper physical cap. | $N_{\min}$ (*Minimum Required Skill Level*), $T_{\max}, C_{\max}, F_{\max}$ (*Tempo, Control, and Force Aspect Ceilings*). |
| **Capital Delta $\Delta$ (`Delta_...`)** | **Additive Shift / Situational Modifier** — A signed integer shift ($+$ or $-$) applied to a Target Number, Carry Penalty, or Margin. | $\Delta_{\text{tool}}$ (*Known Tool Modifier*), $\Delta_{\text{ext}}$ (*Hidden External Difficulty*), $\Delta_{\text{int}}$ (*Internal Character Strain*), $\Delta M$ (*Net Contested Signed Margin*). |
| **Lowercase Delta $\delta$ (`delta_...`)** | **Cascade Carry Step Size** — The magnitude of momentum bonus or stumble penalty passed from Pair $n$ to Pair $n+1$. | $\delta_{\text{over}} = +6$ (*Positive Carry Bonus on Success*), $\delta_{\text{under}} = 6 + \Delta_{\text{int}}$ (*Negative Carry Penalty on Failure*). |

---

## 2. Core `2d20` Cascade, Task Resolution & Probability Variables

Used primarily in [Probability & Calibration](probability_and_calibration.md) and [Core `2d20` Cascade & Task Resolution](../02-core-mechanics/cascading_d20_and_tasks.md):

### 2.1 Dice, Targets & Cascade Step Variables

| Symbol | Full Name | Scale / Range | Definition & Formula |
| :--- | :--- | :---: | :--- |
| $K$ | **Die Size (Faces per Die)** | $K = 20$ | Number of faces on each die (`d20`, rolling $1\text{–}20$). A pair of dice (`2d20`) has a minimum sum of $2$, a maximum sum of $2K = 40$, and an exact 50% midpoint mean of $K + 1 = 21$. |
| $N$ *(or $N_P, N_{GM}, N_{NPC}$)* | **Skill Level (Number of `2d20` Pairs)** | $0\text{–}8$ *(Mythic `10`)* | Number of sequential `2d20` pairs rolled in a cascade (`0` = Untrained, `1` = Novice/Talent, `3` = Competent Journeyman, `5` = Veteran Specialist, `8` = Grandmaster). |
| $N_{\min}$ | **Minimum Skill Gate** | $1\text{–}4$ | Minimum Skill Level required to attempt a **Class B** or **Class C (`[Locked]`)** specialist task. |
| $N_{\text{parent}}$ | **Parent / Sibling Skill Level** | $0\text{–}8$ | Rank in an immediate parent or sibling skill node when defaulting on a **Class B** task ($N_{\text{default}} = \max(1, N_{\text{parent}} - 1)$). |
| $N_{\text{eff}}$ | **Effective Dual-Skill Level** | $1\text{–}9$ | Number of `2d20` pairs rolled when an action simultaneously requires two skills $N_1$ and $N_2$: $N_{\text{eff}} = \min(N_1, N_2) + 1$ (if $|N_1 - N_2| \ge 2$) else $\min(N_1, N_2)$. |
| $A_n, B_n$ | **List A & List B Die Rolls at Step $n$** | $1\text{–}20$ each | The two individual 20-sided dice rolled on step $n \in \{1, \dots, N\}$ of the cascade. ($A_{GM}, B_{GM}$ are the two `d20` dice rolled secretly by the GM). |
| $X_n$ *(or $X_{GM}$)* | **Unmodified Pair Sum** | $2\text{–}40$ | The raw sum of the two `d20` dice on step $n$ before any carry-over modifier is applied: $X_n = A_n + B_n$. For the GM's hidden 1-pair roll, $X_{GM} = A_{GM} + B_{GM}$. |
| $\text{Carry}_n$ | **Step $n$ Carry-Over Modifier** | $\mathbb{Z}$ (`0` at $n=1$) | The momentum bonus or stumble penalty carried into Pair $n$ from the result of Pair $n-1$. Always starts at $\text{Carry}_1 = 0$ on Pair 1. |
| $Y_n$ | **Effective (Modified) Pair Total** | $\mathbb{Z}$ | The total value of Pair $n$ after adding carry-over: $Y_n = X_n + \text{Carry}_n = A_n + B_n + \text{Carry}_n$. |
| $Y_{\text{best}}$ | **Closest Attempt (Maximum Pair Total)** | $\mathbb{Z}$ | The highest modified pair total achieved across all $N$ steps: $Y_{\text{best}} = \max(Y_1, Y_2, \dots, Y_N)$. Used to determine Clean Miss (`0`) vs. Botch severity (`-1` to `-5`) when $S_P = 0$. |
| $C$ *(or $C_P, C_{GM}, C_{NPC}$)* | **Controlling Characteristic ("Control")** | $1\text{–}20$ *(Avg `10`)* | The governing attribute score (or composite $\lfloor(C_1 + C_2)/2\rfloor$) for the task. Higher $C$ lowers the Target Number by `1` per point. |
| $\Delta_{\text{tool}}$ | **Known Tool & Loadout Modifier** | $-4$ to $+8$ | Transparent equipment or training modifier known to the player before rolling (`-4` 2033 AI smart-rig, `-2` masterwork kit, `0` standard issue, `+4` to `+8` improvised tools or untrained default). Shifts $\text{TN}_P$. |
| $\Delta_{\text{ext}}$ | **Hidden External Task Difficulty** | $-10$ to $+16$ | Objective complexity of the obstacle or environment (`-8` Trivial, `-4` Easy, `0` Standard Benchmark, `+4` Challenging, `+8` Hard, `+12` Extreme, `+16` Heroic). Shifts the GM's hidden target $\text{TN}_{GM}$. |
| $\Delta_{\text{int}}$ | **Internal Character Penalty Modifier** | $-2$ to $+10$ | Internal physiological/psychological strain from Fatigue, Wound Shock, Pain, Dehydration, or Cognitive Dissonance (`-2` Focused/Conviction Surge, `0` Fresh, `+2` to `+10` Winded to Exhausted). Increases $\delta_{\text{under}}$. |
| $\text{TN}_0$ | **Universal Baseline Midpoint Target** | $21$ | The baseline `2d20` target number for an average human ($C = 10$) under standard conditions ($\Delta = 0$): $\text{TN}_0 = K + 1 = 21$. |
| $\text{TN}_P$ | **Target Number of the Player** | $9\text{–}35$ *(Base `21`)* | The **known personal Roll-Over target** the player must beat on each pair: $\text{TN}_P = (1.5K + 1) - C_P + \Delta_{\text{tool}} = 31 - C_P + \Delta_{\text{tool}}$. |
| $\text{TN}_{GM}$ | **Target Number of the Game Master** | $5\text{–}31$ *(Base `21`)* | The **hidden resistance target** the GM must beat on their opposing `2d20` roll: $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$ (equivalent to $C_{GM} = 10 + \Delta_{\text{ext}}$). |
| $\text{TN}_{\text{eff}}$ | **Effective Uncontested Target Number** | $5\text{–}43$ *(Base `21`)* | Combined single-side benchmark target used when analyzing uncontested probability curves: $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$. |
| $\delta_{\text{over}}$ | **Positive Success Carry Bonus** | $+6$ *(or $+8$)* | Momentum bonus passed to the next pair after a **successful** pair ($Y_n > \text{TN}_P$). Equal to $0.3K = +6$ (boosted to `+8` with `3+` successes on a complementary `KNO` setup roll). |
| $\delta_{\text{base}}$ | **Baseline Failure Carry Penalty** | $6$ | Base stumble penalty ($0.3K = 6$) before internal strain $\Delta_{\text{int}}$ is added. |
| $\delta_{\text{under}}$ | **Negative Failure Carry Penalty** | $4\text{–}16$ *(Base `6`)* | Total stumble penalty magnitude subtracted from the next pair after a **failed** pair ($Y_n \le \text{TN}_P$): $\delta_{\text{under}} = \delta_{\text{base}} + \Delta_{\text{int}} = 6 + \Delta_{\text{int}}$. |

### 2.2 Contested Outcomes, Margins & Statistical Moments

| Symbol | Full Name | Scale / Range | Definition & Formula |
| :--- | :--- | :---: | :--- |
| $S_P$ | **Player Success Count** | $0\dots N_P$ | Total number of pairs where the player beat their personal target ($Y_n > \text{TN}_P$). |
| $B_P$ | **Player Floor-Breach Count** | $0\dots N_P$ | Number of failed steps where negative carry pushed the player's modified total below the physical `2d20` minimum of snake-eyes ($Y_n < 2$, `raw_botches`). |
| $B_{\text{eff}}$ | **Player Effective Closest-Miss Botch Tier** | $0\dots 5$ | When $S_P = 0$, the player's closest attempt $Y_{\text{best}}$ binned into `4`-point ($K/5 = 4$) tiers below $\text{TN}_P$: $B_{\text{eff}} = \min(5, \lfloor(\text{TN}_P - Y_{\text{best}})/4\rfloor)$. |
| $M_P$ | **Player Signed Margin** | $-5\dots +N_P$ | Unified signed outcome of the player's cascade: $M_P = +S_P$ if $S_P \ge 1$; $M_P = 0$ on a Clean Miss ($S_P = 0, B_{\text{eff}} = 0$); or $M_P = -B_{\text{eff}}$ on a Botch. |
| $S_{GM}$ | **GM Resistance Success Count** | $0\dots 5+$ | Number of opposing successes scored by the GM. Under the **1-Pair Tiered GM Rule (`4`-pt over-tiers)**, when $X_{GM} > \text{TN}_{GM}$: $S_{GM} = 1 + \lfloor(X_{GM} - \text{TN}_{GM} - 1)/4\rfloor$. |
| $B_{GM}$ | **GM Environmental / Obstacle Botch Tier** | $0\dots 5$ | Under the **1-Pair Tiered GM Rule (`4`-pt under-tiers)**, when $X_{GM} \le \text{TN}_{GM}$: $B_{GM} = \min(5, \lfloor(\text{TN}_{GM} - X_{GM})/4\rfloor)$. |
| $M_{GM}$ | **GM Signed Margin** | $-5\dots +5+$ | Unified signed outcome of the GM's hidden roll: $M_{GM} = +S_{GM}$ if $X_{GM} > \text{TN}_{GM}$, or $M_{GM} = -B_{GM}$ if $X_{GM} \le \text{TN}_{GM}$. |
| $\Delta M$ | **Net Contested Signed Margin** | $\mathbb{Z}$ | Net margin between the Player and the GM: $\Delta M = M_P - M_{GM}$. Determines `CleanSuccess` ($1 \le \Delta M \le 2$) vs. `ExtraordinarySuccess` ($\Delta M \ge +3$). |
| $\text{PMF}$ | **Probability Mass Function** | $P(X = x) \in [0, 1]$ | Function giving the exact probability that a discrete random variable equals each possible integer outcome $x$. |
| $\text{E}[X]$ *(or $\mu$)* | **Expected Value (Mean)** | $\mathbb{R}$ | Probability-weighted average outcome: $\text{E}[X] = \mu = \sum_x x \cdot P(X = x)$ (e.g., $\text{E}[\text{Margin}]$ or $\text{E}[\Delta M]$). |
| $\text{Var}(X)$ *(or $\sigma^2$)* | **Variance** | $\mathbb{R}_{\ge 0}$ | Expected squared deviation from the mean: $\sigma^2 = \sum_x (x - \mu)^2 P(X = x)$ (where $\sigma = \sqrt{\sigma^2}$ is the standard deviation). |
| $\mu_3$ & $\gamma_1$ | **Third Central Moment & Skewness** | $\mathbb{R}$ | Measures distribution asymmetry: $\mu_3 = \sum_x (x - \mu)^3 P(X = x)$ and $\gamma_1 = \mu_3 / \sigma^3$ ($\gamma_1 > 0$ = right-skewed tail; $\gamma_1 < 0$ = left-skewed tail). |
| $\kappa_{\text{excess}}$ | **Excess Kurtosis** | $\mathbb{R}$ | Measures tail heaviness / peak flatness relative to a normal bell curve: $\kappa_{\text{excess}} = (\mu_4 / \sigma^4) - 3$. |
| $\text{BC}$ | **Sarle's Bimodality Coefficient** | $0.0\text{–}1.0$ | Diagnostic metric for whether a distribution is unimodal or bimodal: $\text{BC} = (\gamma_1^2 + 1) / (\kappa_{\text{excess}} + 3)$. Values **$\text{BC} > 0.555$ ($5/9$)** indicate a **bimodal ("watershed")** distribution. |

---

## 3. Character Characteristics, Senses, Psyche & Lifepath Variables

Used in [Characteristics, Senses, Psyche & Aptitudes](../03-character/characteristics_and_aptitudes.md), [Skill System & Hierarchical Skill Trees](../03-character/skills_and_trees.md), and [Lifepath Character Creation, Upkeep & Heat](../03-character/lifepaths_and_economy.md):

| Symbol / Abbr | Full Name | Scale / Range | Definition & Formula |
| :--- | :--- | :---: | :--- |
| `SPD`, `FOR`, `DEX`, `KIN`, `HLT`, `FIT` | **Physical Primary Characteristics** | `1–20` *(Avg `10`)* | **Speed** (`SPD`), **Force** (`FOR`), **Dexterity** (`DEX`), **Kinesthesis** (`KIN`), **Health** (`HLT`), and **Fitness** (`FIT`). Mapped from the `0.0–100.0` spectrum (`5.0` spectrum pts = `1` d20 step). |
| `MAS`, `HGT` | **Mass & Height** | `kg`, `m` | Character's physical body mass in kilograms (`MAS`, default `70 kg`) and stature in meters (`HGT`, default `1.50–1.75 m`). |
| `MEM`, `ANA`, `INS`, `CRE`, `DIS`, `FOC` | **Mental Primary Characteristics** | `1–20` *(Avg `10`)* | **Memory** (`MEM`), **Analysis** (`ANA`), **Insight** (`INS`), **Creativity** (`CRE`), **Discipline** (`DIS`), and **Focus** (`FOC`). |
| `COM`, `PRE`, `ELO` | **Social Primary Characteristics** | `1–20` *(Avg `10`)* | **Comeliness** (`COM`), **Presence** (`PRE`), and **Eloquence** (`ELO`). |
| `SIG`, `HEA`, `TAC`, `OLF`, `TAS`, `ITU` | **Sensory Characteristics** | `1–20` *(Avg `10`)* | **Sight** (`SIG`), **Hearing** (`HEA`), **Tactile** (`TAC`), **Olfactory** (`OLF`), **Taste** (`TAS`), and **Intuition** (`ITU`). |
| `EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG` | **Psyche Characteristics (The Invisible Traits)** | `1–20` *(Neutral `10`)* | **Ego** (`EGO`), **Selfishness** (`SLF`), **Openness** (`OPN`), **Belongingness** (`BLG`), **Extraversion** (`EXT`), **Conformity** (`CNF`), and **Aggressiveness** (`AGG`). |
| $\text{POW}$ | **Physical Power** | $\mathbb{R}$ (`10.0` Avg) | Rate of mechanical energy transfer: $\text{POW} = (\text{FOR} \times \text{SPD}) / 10$ (or raw product $\text{FOR} \times \text{SPD}$ on the `1–400` scale). |
| $\text{MMF}$ | **Melee Momentum Factor** | $\mathbb{R}$ | Mass-scaled kinetic impact multiplier for unarmed and melee strikes, derived from `MAS`, `FOR`, and `SPD`. |
| $A$ *(or $\text{APT}_{\text{domain}}$)* | **Domain Aptitude** | $-2$ to $+3$ | Innate neurological or somatic learning affinity across 9 domains (`QNT`, `MEC`, `CMP`, `BIO`, `LNG`, `KIN`, `FIN`, `AUD`, `SOC`). |
| $C_{\text{study}}$ | **Composite Study Characteristic** | `1–23` | Governing controller for Lifepath and Downtime educational advancement rolls: $C_{\text{study}} = \lfloor(\text{DIS} + C_{\text{mental}})/2\rfloor + \text{APT}_{\text{domain}}$. |
| $\text{SQM}$ | **School Quality Modifier / Socioeconomic Tier** | $-6$ to $+4$ *(or Tier `1–10`)* | In childhood Lifepaths (Stages 1–4), shifts educational difficulty (`+4` blighted school to `-6` elite prep). In adulthood, `SQM 1–10` tracks Socioeconomic Lifestyle Tier. |
| $\text{TN}_{\text{edu}}$ | **Educational Advancement Target Number** | $8\text{–}31$ | Target number for Lifepath schooling rolls: $\text{TN}_{\text{edu}} = 31 - C_{\text{study}} + \text{SQM}$. |

---

## 4. Action Economy, OODA Loop, Psyche & Social Combat Variables

Used in [Action Economy, OODA Loop & Stealth](../02-core-mechanics/action_economy_and_ooda.md) and [Psyche, Morale & Social Combat](../04-psyche-and-social/psyche_morale_and_social_combat.md):

| Symbol / Abbr | Full Name | Scale / Range | Definition & Formula |
| :--- | :--- | :---: | :--- |
| $\text{Tick}$ | **Action Clock Time Unit** | $1\text{ Tick} = 0.25\text{ s}$ | Fundamental combat time unit (`4 Ticks = 1.0 second`, `20 Ticks = 5.0s Tactical Window`, `240 Ticks = 1.0 minute`). |
| $D_{\text{cog}}$ | **Cognitive Decision Score ("Quick Thinking")** | `1–20` *(Avg `10`)* | Speed of mental orientation and tactical decision-making: $D_{\text{cog}} = \lfloor(\max(\text{ANA}, \text{CRE}) + \text{FOC}) / 2\rfloor$. |
| $\text{OODA}$ | **4-Pillar OODA Loop Score** | `1–20` *(Avg `10`)* | Observe–Orient–Decide–Act composite reflex score: $\text{OODA} = \lfloor(\text{ITU} + \text{INS} + D_{\text{cog}} + \text{SPD}) / 4\rfloor$. |
| $\text{TC}$ *(or $\text{TC}_{\text{base}}, \text{TC}_{\text{eff}}$)* | **Tick Cost (Action Duration)** | $\ge 1\text{ Tick}$ | Number of Ticks required to complete an action ($\text{TC} = \text{TC}_{\text{decide}} + \text{TC}_{\text{act}}$), modified by invested Tempo ($T - B$) and `action_time_factor`. |
| $\text{AWR}$ | **Perceptual Awareness Score** | `0–20` | Situational sensory alertness across Sight (`SIG`), Hearing (`HEA`), and Intuition (`ITU`). |
| $P$ | **Psyche Trait Score** | `1–20` *(Neutral `10`)* | Score in a specific Psyche trait (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`). |
| $\Delta_{\text{psyche}}$ | **Psyche Deviation** | $0\text{–}10$ | Distance of a Psyche trait from neutral (`10`): $\Delta_{\text{psyche}} = |P - 10|$. Sets hidden GM resistance $\text{TN}_{GM} = 21 - \Delta_{\text{psyche}}$ on a Psyche Friction Check. |
| $\Delta_{\text{mot}}$ | **Situational Motivation Modifier** | $-4$ to $+6$ | Shift to a combatant's Morale Controller based on what they are fighting for (e.g., protecting family/crew vs. mercenary paycheck). |
| $C_{\text{morale}}$ | **Morale Controlling Characteristic** | `1–24` | Effective controller for combat morale checks: $C_{\text{morale}} = \text{DIS} + \Delta_{\text{mot}}$ ($\text{TN}_P = 31 - C_{\text{morale}}$). |
| $\Delta_{\text{morale}}$ | **Battlefield Morale Penalty** | $0$ to $+12$ | Cumulative battlefield shock modifier (casualties, outnumbered, leader down), setting the GM's hidden resistance target $\text{TN}_{GM} = 21 - \Delta_{\text{morale}}$. |
| $\text{CMP}$ | **Social Composure Threshold** | $1\text{–}10$ *(Avg `5`)* | Mental stamina buffer in Social Combat: $\text{CMP} = \lfloor(\text{DIS} + \text{FOC}) / 4\rfloor$. |

---

## 5. Combat Tactical Triad ("Envelope & Fill"), Ballistics & Survival Variables

Used in [Tactical Triad & Combat Pools](../05-combat/tactical_triad_and_pools.md), [Melee Combat](../05-combat/melee_combat.md), [Ranged Combat & Armor](../05-combat/ranged_combat_and_armor.md), [Health & Trauma](../06-health-and-survival/health_and_trauma.md), and [Movement, Vehicles & Environment](../06-health-and-survival/movement_vehicles_and_environment.md):

| Symbol / Abbr | Full Name | Scale / Range | Definition & Formula |
| :--- | :--- | :---: | :--- |
| $C_{\text{combat}}$ | **Combat Readiness Controller** | `1–20` *(Avg `10`)* | Composite combat attribute combining the weapon/martial skill's controller $C_{\text{skill}}$ with `OODA`: $C_{\text{combat}} = \lfloor(C_{\text{skill}} + \text{OODA}) / 2\rfloor$. |
| $P_{\text{base}}$ | **Base Tactical Pool ("The Fill")** | $10\text{–}44$ | Total unmodified points available to allocate across the Tactical Triad: $P_{\text{base}} = C_{\text{combat}} + 3N$. |
| $P_{\text{eff}}$ | **Effective Tactical Pool** | $\ge 0$ | Tactical Pool remaining after subtracting `-2` per tier of Fatigue (`fatigue_level`), `-2` per tier of Wound Shock (`wound_shock_index`), and Morale modifier $\Delta P_{\text{morale}}$. |
| $B$ | **Neutral Triad Baseline** | $\lfloor P_{\text{eff}} / 3 \rfloor$ | Equal one-third split of the combatant's Effective Tactical Pool ($B = \lfloor P_{\text{eff}} / 3 \rfloor$). Investing Tempo $T > B$ speeds up actions; $T < B$ slows them down. |
| $T, C, F$ | **Allocated Tempo, Control & Force** | $T + C + F = P_{\text{eff}}$ | Points from $P_{\text{eff}}$ allocated into **Tempo** ($T$ — speed, initiative, evasion), **Control** ($C$ — precision, guard, recoil control), and **Force** ($F$ — strike power, recoil bracing, root). |
| $T_{\max}, C_{\max}, F_{\max}$ | **Aspect Ceilings ("The Envelope")** | `0–20` each | Maximum points that can be invested into each aspect on a single action, anchored by physical characteristics (`SPD` $\to T_{\max}$, `DEX`/`KIN`/`SIG` $\to C_{\max}$, `FOR` $\to F_{\max}$) and reduced by localized anatomical wounds. |
| $\text{Root}$ | **Physical Stability & Balance Rating** | $\mathbb{R}$ | Resistance to being knocked prone, swept, or thrown (`Kuzushi`), anchored by `FOR`, `KIN`, `MAS`, and allocated `Force + Control`. |
| $R$ | **Weapon Recoil Rating** | $1\text{–}12+$ | Physical recoil impulse of a firearm burst; compared against the shooter's allocated **Force ($F$)** and `FOR`. |
| $\text{PR}_{\text{Edged}}, \text{PR}_{\text{Ballistic}}$ | **Armor Protection Ratings** | $0\text{–}20+$ | Structural resistance of body armor against edged/piercing weapons ($\text{PR}_{\text{Edged}}$) and ballistic projectiles ($\text{PR}_{\text{Ballistic}}$). |
| $\text{BABT}$ | **Behind-Armor Blunt Trauma** | $\ge 0$ | Kinetic blunt shock transmitted into the wearer's ribs/organs even when armor stops projectile penetration. |
| $\text{Rel}$ | **Weapon Reliability Rating** | $0\text{–}5$ | Mechanical tolerance of a firearm (`0` fragile 3D-printed street gun to `4–5` rugged military service weapon). A malfunction/jam triggers when cascade floor breaches exceed reliability ($B_P > \text{Rel}$). |
| $\text{FT}$ | **Fatigue Threshold** | $\mathbb{Z}$ | Stamina buffer derived from `FIT` and `HLT` governing how many Exertion Marks a character can accumulate before gaining `+1` `fatigue_level`. |
| $\text{WSR}$ | **Wound Shock Resistance** | $6\text{–}30$ *(Avg `15`)* | Physiological pain and trauma buffer: $\text{WSR} = \text{HLT} + \lfloor\text{DIS} / 2\rfloor$. |
| $\text{SF}$ | **Stride Factor** | $\mathbb{R}$ (`1.00` at `1.75 m`) | Stature-based stride multiplier: $\text{SF} = \text{HGT} / 1.75\text{ m}$. |
| $V_{\text{sprint}}$ | **Maximum Sprint Velocity** | $\text{m/s}$ *(or $\text{m/Tick}$)* | Top sprint speed: $V_{\text{sprint}} = (4.0 + 0.35 \times \text{SPD}_{\text{eff}}) \times \text{SF}$ (`7.50 m/s` or `1.88 m/Tick` at average `SPD 10, HGT 1.75 m`). |
| $W_{\text{req}}$ | **Daily Water Requirement** | $3.0\text{–}10.0\text{ L/day}$ | Liters of water required per 24 hours based on ambient temperature and physical exertion. |
| $\text{CLO}$ | **Clothing Thermal Insulation Rating** | $\mathbb{R}$ | Thermal protection rating of clothing/suits against hypothermia ($\Delta_{\text{cold}}$) vs. heat retention ($\Delta_{\text{heat}}$). |
| $\text{Sv}$ | **Sievert (Ionizing Radiation Dose)** | $\ge 0\text{ Sv}$ | Unit of cumulative acute ionizing radiation exposure (`< 0.5 Sv` subclinical to `> 6.0 Sv` lethal acute radiation syndrome). |
