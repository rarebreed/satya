# Satya d20 TTRPG Core Ruleset Reference

This document defines the canonical **Satya d20** mechanics implemented by `TtrpgMechanicsEvent::evaluate_satya_task` in `crates/satya-schema/src/events.rs` and used by the Satya AI Game Master (`query_satya_rules`).

## 1. Roll-Under d20 Dice Pool & Target Numbers (TN)

- Every character and NPC has controlling **Attribute Target Numbers (`attribute_tn`)** on a `1..=20` scale (lower TN = harder attribute threshold unless boosted, or roll `<= attribute_tn` to score a success).
- When attempting a skill check or combat action:
  - Roll a pool of `N` twenty-sided dice (`Nd20`), where `pool_size` is determined by `1 + skill_rank` (modified by situational advantage/disadvantage).
  - Each die that rolls **less than or equal to `attribute_tn`** (`roll <= attribute_tn`) counts as **1 success**.
  - A natural **`1`** is an automatic critical die and counts as **2 successes** (`roll <= attribute_tn` + bonus natural-1 success).
  - A natural **`20`** is a complication die (`nat_20s += 1`).

## 2. Difficulty Rating & Degrees of Success (`OutcomeTier`)

Compare total `successes_counted` against the check's `difficulty_rating` (default `1` for standard tasks, `2` for hard tasks, `3+` for extreme tasks):

- **`EXTRAORDINARY_SUCCESS`**: `successes_counted >= difficulty_rating + 2` OR two or more natural `1`s (`nat_1s >= 2`).
- **`SUCCESS`**: `successes_counted >= difficulty_rating`.
- **`PARTIAL_SUCCESS`**: `successes_counted > 0` and `successes_counted + 1 == difficulty_rating` (succeeds at a cost or partial effect).
- **`FAILURE`**: `successes_counted < difficulty_rating` and not a partial success or critical failure.
- **`CRITICAL_FAILURE`**: `successes_counted == 0` AND at least one die rolled a natural `20` (`nat_20s > 0`).

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
