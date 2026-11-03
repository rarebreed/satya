# Satya d20 TTRPG Core Ruleset Reference

This document defines the canonical **Satya d20** mechanics implemented by `CascadeTaskParams`, `evaluate_cascade_roll`, and `evaluate_satya_cascade_task` in [`crates/satya-engine/src/dice_probabilities.rs`](file:///c:/Users/hapah/Projects/satya/crates/satya-engine/src/dice_probabilities.rs) (calibrated in [`docs/ttrpg/probability.md`](file:///c:/Users/hapah/Projects/satya/docs/ttrpg/probability.md)) and used by the Satya AI Game Master (`query_satya_rules`).

## 1. Two-List Cascading ("Watershed") `2d20` Dice Mechanic & Target Numbers ($\text{TN}_{\text{eff}}$)

Satya uses the **Two-List Cascading ("Watershed") Dice Mechanic** with **`K = 20` (`d20`)** dice so that the shape (skewness, modality, and tail behavior) of the probability distribution dynamically reflects skill, natural control, fatigue/injury, and environmental difficulty.

### 1.1 The Dice Pool: Sequential Pairs of `2d20` Determined by Skill Level ($N$)
- When attempting a skill check or combat action, a character rolls **$N$ sequential pairs of 20-sided dice (`2d20`)**, where **$N$ is the character's Skill Level**:
  $$\text{List A} = (A_1, A_2, \dots, A_N), \qquad \text{List B} = (B_1, B_2, \dots, B_N)$$
- **Skill Level ($N$) Scale**:
  - **`N = 1`**: Untrained / Novice (or raw Controlling Characteristic check; rolls a single `2d20` pair).
  - **`N = 2`**: Apprentice / Dabbler (`2` sequential `2d20` pairs).
  - **`N = 3`**: **Competent / Journeyman (Trained Average)** (`3` sequential `2d20` pairs; `70.10%` success chance at $C = 10, \Delta = 0$).
  - **`N = 4–5`**: Professional / Veteran Specialist.
  - **`N = 6–7`**: Expert / Master.
  - **`N = 8`**: **Grandmaster (Standard Maximum)** (`92.37%` success chance at $C = 10, \Delta = 0$).
  - **`N = 10`**: Mythic / Transcendent Cap.
- At each step $n \in \{1, \dots, N\}$, the unmodified pair sum $X_n = A_n + B_n$ ranges from **$2$ to $40$**, forming a triangular distribution centered at its exact midpoint of **$21$**.

### 1.2 Roll-Over Target Number ($\text{TN}_{\text{eff}}$) & Controlling Characteristic ($C$)
- Satya is a **Roll-Over** system where **higher numbers are always better**:
  - A higher pair sum ($A_n + B_n$) is always better.
  - A higher **Controlling Characteristic ($C$)** (such as Strength, Dexterity/Agility, Intelligence, or Willpower) is always better.
- **Controlling Characteristic ($C$) Scale**: Theoretical range `1–20`, playable human range **`4–18`**, with the **Human Average at `C = 10`**.
- The **Midpoint-Anchored Target Number ($\text{TN}_{\text{eff}}$)** is calibrated so an average human ($C = 10$) attempting a standard benchmark task ($\Delta_{\text{ext}} = 0$) needs the exact `2d20` midpoint of **$21$**:
  $$\text{TN}_{\text{eff}} = (1.5K + 1) - C + \Delta_{\text{ext}} = 31 - C + \Delta_{\text{ext}}$$
  - Each **+1 to Controlling Characteristic ($C$)** *lowers* $\text{TN}_{\text{eff}}$ by `1` on every pair (making the check easier).
  - Each **+1 to External Task Difficulty ($\Delta_{\text{ext}}$)** *raises* $\text{TN}_{\text{eff}}$ by `1` on every pair (making the check harder).

### 1.3 Sequential Watershed Cascade: Carry-Over Momentum ($\delta_{\text{over}}$ & $\delta_{\text{under}}$)
The $N$ pairs are resolved sequentially from $n = 1$ to $N$ using the effective pair total $Y_n = A_n + B_n + \text{Carry}_n$:
1. **Pair 1 ($n = 1$)**: Starts with zero carry-over ($\text{Carry}_1 = 0$, so $Y_1 = A_1 + B_1$).
   - If $Y_1 > \text{TN}_{\text{eff}}$ (or $\ge \text{TN}_{\text{eff}}$): **+1 Success!** The character gains momentum, passing a positive carry bonus $+\delta_{\text{over}}$ (**`+6`** baseline) to Pair 2.
   - If $Y_1 < \text{TN}_{\text{eff}}$: **0 Successes on Pair 1.** The character stumbles, passing a negative carry penalty $-\delta_{\text{under}}$ to Pair 2, where:
     $$\delta_{\text{under}} = \delta_{\text{base}} + \Delta_{\text{int}} = 6 + \Delta_{\text{int}}$$
2. **Subsequent Pairs ($n = 2 \dots N$)**: Each pair computes $Y_n = A_n + B_n + \text{Carry}_n$ and compares $Y_n$ against $\text{TN}_{\text{eff}}$.
   - **Standard Carry Mode (`markov_step` / `CarryMode::MarkovStep`)**: Pair $n$ receives only a single $+\delta_{\text{over}}$ (`+6`) if Pair $n-1$ succeeded, or a single $-\delta_{\text{under}}$ (`-4` to `-16`) if Pair $n-1$ failed. A skilled character who misses Pair 1 can fight through the penalty on Pair 2 or 3 and flip the cascade back into positive $+\delta_{\text{over}}$ momentum.
   - **High-Stakes Carry Mode (`cumulative_snowball` / `CarryMode::CumulativeSnowball`)**: Carry-over compounds across steps ($\text{Carry}_{n+1} = \text{Carry}_n \pm \delta$), creating a stark bimodal ("watershed") distribution where early momentum snowballs.
   - **Linear Margin Carry Mode (`linear_margin` / `CarryMode::LinearMargin`)**: Exact difference carry ($\text{Carry}_{n+1} = Y_n - \text{TN}_{\text{eff}}$).

### 1.4 The Four Task Factors
Every task in Satya is parameterized by four orthogonal factors (`CascadeTaskParams`):
1. **Skill Level ($N \in [1, 8]$)**: Number of `2d20` pairs rolled; expands the right tail (`+1` to `+N` successes), raises expected margin `E[Margin]`, and provides extra chances to recover from an early miss.
2. **Controlling Characteristic ($C \in [1, 20]$)**: Shifts $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$ across all pairs simultaneously (including Pair 1).
3. **Internal Character Modifiers ($\Delta_{\text{int}} \in [-2, +10]$)**: Models Fatigue, Injury, and Stress by increasing the failure carry penalty $\delta_{\text{under}} = 6 + \Delta_{\text{int}}$ (`-4` when Focused/Rested at $\Delta_{\text{int}} = -2$, `-6` Fresh baseline at $\Delta_{\text{int}} = 0$, `-10` Winded at $+4$, `-12` Battered at $+6$, up to `-16` Exhausted/Agonized at $+10$). Crucially, $\Delta_{\text{int}}$ imposes **zero penalty on Pair 1**—if a fatigued master succeeds on Pair 1, fatigue never slows them down, but if they miss Pair 1, recovery is much harder.
4. **External Task Modifiers ($\Delta_{\text{ext}} \in [-10, +16]$)**: Shifts $\text{TN}_{\text{eff}}$ for environmental conditions and task complexity (`-10` Trivial, `-4` Easy, `0` Standard Benchmark, `+4` Challenging, `+8` Hard, `+12` Extreme, `+16` Nearly Impossible).

---

## 2. Signed Outcome Spectrum (`-N` to `+N`), Botch Rules, & `OutcomeTier`

A single cascading roll resolves both **how well the character succeeds** and **how badly they fail** on a unified signed margin spectrum (`signed_margin` in `-N..=+N`):

### 2.1 Positive Outcomes (`+1` to `+N` Successes)
- **`+1` Success (Bare Minimum)**: Accomplishes the task with minimal margin.
- **`+2` Successes (Solid Execution)**: Clean, reliable professional execution.
- **`+3` to `+4` Successes (Extraordinary / Critical)**: Faster, quieter, higher yield, or secondary tactical advantage (`OutcomeTier::ExtraordinarySuccess`).
- **`+5+` Successes (Masterwork / Legendary)**: Achievable only by highly trained specialists ($N \ge 5$).

### 2.2 Botch Severity (`-1` to `-N`) & Avoiding the "Skilled Botch Paradox"
Satya prevents the classic dice-pool paradox (where rolling more dice for higher skill accidentally increases critical failure rates) through two botch evaluation rules (`BotchRule`):
1. **Worst-Case Recovery (`closest_miss` / `BotchRule::ClosestMiss` — Default)**:
   - A botch failure only occurs when **`total_successes == 0`**.
   - Severity is determined by the character's **closest attempt across all $N$ pairs**, $Y_{\text{best}} = \max(Y_1, \dots, Y_N)$, binned in 4-point ($K / 5 = 4$) tiers below $\text{TN}_{\text{eff}}$ (`min_miss_tier` in `0..=5`):
     - If $\text{TN}_{\text{eff}} - 4 \le Y_{\text{best}} \le \text{TN}_{\text{eff}}$ (`min_miss_tier == 0`): **`0` (Clean Failure / Near Miss)**.
     - Each additional 4-point tier below $\text{TN}_{\text{eff}}$ adds `-1` botch severity (`-1` to `-5`).
   - Because adding another skill pair $Y_{N+1}$ can only increase $\max(Y_1, \dots, Y_N)$, higher skill $N$ strictly reduces both failure probability and botch severity.
2. **Impossible Floor Breach (`ceiling_breach` & "Success with a Complication")**:
   - Because two `d20` dice have a minimum sum of `2`, an effective pair total $Y_n = A_n + B_n - \delta_{\text{under}}$ can only drop **below `2` ($Y_n < 2$)** after a prior failed pair applies $-\delta_{\text{under}}$ and the current pair rolls near snake-eyes (`raw_botches += 1`).
   - If a character achieves **`total_successes >= 1` AND `raw_botches >= 1`** in the same cascade, the result is **`CascadeOutcomeKind::SuccessWithComplication`** (`OutcomeTier::PartialSuccess` — succeeding at a cost, noise, tool damage, or strain).

### 2.3 Canonical `CascadeOutcomeKind` & `OutcomeTier` Mapping
In `evaluate_cascade_roll` and `evaluate_satya_cascade_task` (`crates/satya-engine/src/dice_probabilities.rs`):
- **`EXTRAORDINARY_SUCCESS`** (`OutcomeTier::ExtraordinarySuccess`): `CascadeOutcomeKind::Success` with `total_successes >= 3` (`raw_botches == 0`).
- **`SUCCESS`** (`OutcomeTier::Success`): `CascadeOutcomeKind::Success` with `1 <= total_successes <= 2` (`raw_botches == 0`).
- **`PARTIAL_SUCCESS`** (`OutcomeTier::PartialSuccess`): `CascadeOutcomeKind::SuccessWithComplication` (`total_successes >= 1` and `raw_botches >= 1`, succeeding with a complication).
- **`FAILURE`** (`OutcomeTier::Failure`): `CascadeOutcomeKind::CleanFailure` (`total_successes == 0` and `effective_botches == 0`).
- **`CRITICAL_FAILURE`** (`OutcomeTier::CriticalFailure`): `CascadeOutcomeKind::BotchFailure` (`total_successes == 0` and `effective_botches >= 1`, `signed_margin < 0`).

---

## 3. Count-Up Action Clock & Combat Stances (`CombatStance`)

Satya combat uses a continuous **Count-Up Tick Clock** rather than fixed cyclic rounds. The combatant with the lowest tick count acts next, and their action advances their clock by `action_count_cost`.

### Base Time Cost (`action_time_cost` / `TC`) by Stance
- **`POWER`** (`TC = 14`): Heavy, high-impact melee or ordnance strike; slower recovery.
- **`RANGED_AIMED`** (`TC = 12`): Deliberate sighted fire; grants accuracy at higher tick cost.
- **`MANEUVER`** (`TC = 11`): Mobile tactical engagement, flanking, or repositioning during an exchange.
- **Unstanced / Standard Action** (`TC = 10`): Default skill check or interaction.
- **`STABILITY`** (`TC = 9`): Braced, defensive posture prioritizing guard and low tick recovery.
- **`RANGED_SNAP`** (`TC = 7`): Fast reactive snapshot at reduced time cost.

## 4. Fatigue & Wound Shock Time Penalty (`action_time_factor`)

Accumulated **Fatigue (`fatigue_level`)** and **Wound Shock (`wound_shock_index`)** degrade a character's speed on the tick clock:

$$\text{action\_time\_factor} = 1.0 + (0.10 \times \text{fatigue\_level}) + (0.25 \times \text{wound\_shock\_index})$$

$$\text{action\_count\_cost} = \text{round}(\text{base\_tc} \times \text{action\_time\_factor})$$
