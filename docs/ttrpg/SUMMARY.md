# Satya TTRPG Rules Book — Summary & Table of Contents

This manifest defines the canonical chapter ordering, identifiers, categories, and cross-references for the **Satya Computer-Assisted RPG (CARPG) Rules Book**. It is consumed by:
- The **Satya Rust Engine & Web Frontend** (`satya-engine`, `satya-signaling` `/api/rules`, and the `satya-web` Left Tabbed Notebook **Rules Book** pane)
- The **Interactive Marimo Rules Book & Static HTML Generator** (`notebooks/satya_rules_book.py` & `python -m satya_ml.build_rules_book`)
- The **Satya AI Game Master** (`query_satya_rules` in `src/satya_ml/ai_gm/tools.py`)

---

## Table of Contents

### Part I: Foundation & Mathematical Calibration (`01-foundation/`)
1. [An RPG for a More Civilized Age: Design Philosophy](01-foundation/design_philosophy.md) (`id: design_philosophy`)
   - Origins of Satya, realism vs. escapism, and how Satya solves 17 structural flaws of traditional RPG systems.
2. [Probability & Calibration of the Cascading Dice Mechanic](01-foundation/probability_and_calibration.md) (`id: probability`)
   - Mathematical foundation, distribution topology (skewness, modality, tail behavior), why $K = 20$ outperforms $K = 10$ and $K = 100$, $C$ vs. $N$ sensitivity, and avoiding the "Skilled Botch Paradox."

### Part II: Core Mechanics, Task Resolution & Action Economy (`02-core-mechanics/`)
3. [Cascading `2d20` Engine & Task Resolution](02-core-mechanics/cascading_d20_and_tasks.md) (`id: tasks`)
   - Canonical Two-List Cascading (`2d20`) dice mechanic, $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$, carry modes (`markov_step`, `cumulative_snowball`), signed outcome tiers, 5-step Task Pipeline, Tool Tiers, Gate Classifications (`Class A / B / C`), Master Task Requirements Table, Extended Project Clocks, and Sarah's lockpicking walkthrough.
4. [Action Economy, OODA Loop, Time Scales & Stealth](02-core-mechanics/action_economy_and_ooda.md) (`id: action_economy`)
   - Temporal calibration (`1 Tick = 0.25s`, `4 Ticks = 1s`, `20 Ticks = 5s Tactical Window`), Full OODA Loop & Cognitive Decision Score ($D_{\text{cog}}$ — "Quick Thinking"), Count-Up Tick Clock mechanics, Base Time Cost (`Base TC`) reference tables, `action_time_factor`, `OODA` action pivots, and Multi-Sensory Stealth & Detection (`SIG`, `HEA`, `OLF`, `ITU`).

### Part III: Character Architecture, Skills, Lifepaths & Economy (`03-character/`)
5. [Characteristics, Senses, Psyche & Aptitudes](03-character/characteristics_and_aptitudes.md) (`id: characteristics`)
   - Physical (`SPD`, `FOR`, `DEX`, `KIN`, `HLT`, `FIT`, `MAS`, `HGT`), Mental (`MEM`, `ANA`, `INS`, `CRE`, `DIS`, `FOC`), Social (`COM`, `PRE`, `ELO`), Sensory (`SIG`, `HEA`, `TAC`, `OLF`, `TAS`, `ITU`), and Psyche (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`) attributes, derived metrics (`POW`, `MMF`, `OODA`, `D_cog`, `CMP`), and domain Aptitudes ($A \in [-2, +3]$).
6. [Skill System & Hierarchical Skill Trees](03-character/skills_and_trees.md) (`id: skills_and_trees`)
   - Talents (`TAL`) vs. Physical (`PHY`), Knowledge (`KNO`), and Hybrid (`PHY/KNO`) skills, Multi-Category taxonomy, `[Access]` restriction tiers, and the 8 Hierarchical Skill Trees (Mathematics, Physical Sciences/Trades, Computing/AI 2.0, Medicine/Biotech, Combat, Athletics/Vehicles, Social/PsyOps, and Literacy/Law/Tradecraft/Arts).
7. [Lifepath Character Creation, Socioeconomic Upkeep & Heat](03-character/lifepaths_and_economy.md) (`id: lifepaths_and_economy`)
   - Chronological Lifepath education & career progression (Ages 0–18+), School Quality (`SQM`), `DIS + Mental + APT` advancement rolls, Mateo Vance & Dr. Priya Nair walkthroughs, Socioeconomic Tier (`SQM 1–10`) lifestyle upkeep, Black-Market `[Access]` procurement gates, and Operational `Heat`.

### Part IV: Psyche, Morale & Social Combat (`04-psyche-and-social/`)
8. [Psyche, Morale & Social Combat](04-psyche-and-social/psyche_morale_and_social_combat.md) (`id: psyche_and_social`)
   - Acting against Psyche (`DIS` Friction Checks & Trait Drift), Purpose & Conviction Surges, Psychological Drives (`Harm:Other` vs. `Self:Protect`), Motivation ($\Delta_{\text{mot}}$), Aggression triggers, the Five Conditions of Morale (`High`, `Steady`, `Wavering`, `Broken`, `Panic`), and Social Combat ("Mental Wounds": Rhetorical Strain, Doubt, Ego Wounds, and Cognitive Dissonance).

### Part V: Combat Architecture, Close Quarters & Firefights (`05-combat/`)
9. [Combat Architecture, Tactical Triad & Pools](05-combat/tactical_triad_and_pools.md) (`id: tactical_triad`)
   - Combat design philosophy, anatomical trauma over Hit Points, Count-Up Action Clock overview, and the **"Envelope & Fill" Tactical Pool Architecture ($P_{\text{base}} = C_{\text{combat}} + 3N$)** across `Tempo`, `Control`, and `Force` ($T_{\max}, C_{\max}, F_{\max}$).
10. [Melee Combat, Close-Quarters Tactics & The Tactical Triad](05-combat/melee_combat.md) (`id: melee_combat`)
    - Close-quarters anatomy, `Tempo / Control / Force` in melee, `Guard` & Seizing the Initiative, `Root` vs. throws/sweeps, the Six Core Tactical Allocations (`TC`, `TF`, `CF`, `Pure T/C/F`), Melee Weapon Reach & Profiles, and the Viktor vs. Elena & Clinch/Throw walkthroughs.
11. [Ranged Combat, Suppression, Armor & Weapon Reliability](05-combat/ranged_combat_and_armor.md) (`id: ranged_combat_and_armor`)
    - Ranged fire modes & Recoil Ratings (`R`), Suppression vs. Morale, Armor Penetration (`PR_Edged`, `PR_Ballistic`) vs. Behind-Armor Blunt Trauma (`BABT`), Weapon Reliability (`Rel`) & Malfunctions on `SuccessWithComplication` / `BotchFailure`, and the Mateo vs. Sgt. Vance & Sniper Overwatch walkthroughs.

### Part VI: Health, Trauma, Movement, Vehicles & Survival (`06-health-and-survival/`)
12. [Health, Anatomical Trauma, Fatigue & Mental Conditions](06-health-and-survival/health_and_trauma.md) (`id: health_and_trauma`)
    - Anatomical `1d20` Hit Location & Called Shot tables, short-term Fatigue (`Levels 0–5`) & asphyxiation, long-term Structural Wounds (`Tiers 1–5`), Broken Bones & Dislocations, Hemorrhaging (`Grades 0–4` & `10.0-Unit` Blood Volume track), Systemic Wound Shock (`WSR`), acute sensory/physiological conditions (`Blinded`, `Stunned`, `Dizzy`, `Nausea`, `Unconscious`), Mental Trauma (`Acute Mental Shock`, `Panic`, `Dissociation`, `Catatonia`), and Medical Triage.
13. [Movement, Encumbrance, Vehicles & Environmental Survival](06-health-and-survival/movement_vehicles_and_environment.md) (`id: movement_and_environment`)
    - Combat movement gaits (`m/tick` and `m/s`), climbing/swimming/jumping/zero-G, overland travel (`km/h`), Encumbrance (`FOR` & `MAS` base capacity, ergonomic load multipliers, `Tiers 0–4`), Vehicles & High-Speed Chases in the Tactical Triad (plus Collisions & `G-LOC`), Hydration & Dehydration, Nutrition & Starvation, Temperature extremes (Hypothermia, Frostbite, Heatstroke), Sleep Deprivation, Altitude Hypoxia, Toxins, and Radiation.
