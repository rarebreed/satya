# Satya TTRPG Rules Book — Summary & Table of Contents

This manifest defines the canonical chapter ordering, identifiers, categories, and cross-references for the **Satya Computer-Assisted RPG (CARPG) Rules Book**. It is consumed by:
- The **Satya Rust Engine & Web Frontend** (`satya-engine`, `satya-signaling` `/api/rules`, and the `satya-web` Left Tabbed Notebook **Rules Book** pane)
- The **Interactive Marimo Rules Book & Static HTML Generator** (`notebooks/satya_rules_book.py` & `python -m satya_ml.build_rules_book`)
- The **Satya AI Game Master** (`query_satya_rules` in `src/satya_ml/ai_gm/tools.py`)

---

## Table of Contents

### Part I: Philosophy & Design Vision
1. [An RPG for a More Civilized Age](design-notes/background.md) (`id: background`)
   - Origins of Satya, realism vs. escapism, and how Satya solves 17 structural flaws of traditional RPG systems.

### Part II: Core `2d20` Watershed Resolution & Probability
2. [Satya d20 Core Ruleset Reference](satya_d20_rules.md) (`id: satya_d20_rules`)
   - Canonical Two-List Cascading (`2d20`) dice mechanic, $\text{TN}_{\text{eff}} = 31 - C + \Delta_{\text{ext}}$, carry modes (`markov_step`, `cumulative_snowball`), signed outcome tiers, and the Count-Up Action Clock.
3. [Probability & The Cascading Dice Mechanic](probability.md) (`id: probability`)
   - Mathematical foundation, distribution topology (skewness, modality, tail behavior), scale invariance across $K$, and avoiding the "Skilled Botch Paradox."
4. [Task Resolution & Skill Check Mechanics](tasks.md) (`id: tasks`)
   - Step-by-step task resolution pipeline, Gate Classifications (`Class A / B / C`), Tool Tiers, Complementary `KNO` $\to$ `PHY` Skill Synergy, and complete worked examples (including Sarah's lockpicking cascade).

### Part III: Characters, Attributes & Lifepaths
5. [Character Characteristics & Attribute Architecture](character/characteristics.md) (`id: characteristics`)
   - Physical (`SPD`, `FOR`, `DEX`, `KIN`, `HLT`, `FIT`, `MAS`, `HGT`), Mental (`MEM`, `ANA`, `INS`, `CRE`, `DIS`, `FOC`), Social (`COM`, `PRE`, `ELO`), Sensory (`SIG`, `HEA`, `TAC`, `OLF`, `TAS`, `ITU`), and Psyche (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`) attributes plus derived metrics (`POW`, `MMF`, `OODA`, `CMP`).
6. [Skill System & Lifepath Architecture](skill-design.md) (`id: skill-design`)
   - Talents (`TAL`) vs. Physical (`PHY`) and Knowledge (`KNO`) skills, 8 hierarchical Skill Trees, Master Task Requirements Table, and chronological Lifepath creation (`SQM`, `DIS + Mental + APT` study rolls, and `[Access]` gates).

### Part IV: Psyche, Social Combat & Tactical Combat
7. [Social Interactions, Psyche Mechanics & Social Combat](social_interactions.md) (`id: social_interactions`)
   - Acting against Psyche (`DIS` Friction Checks & Trait Drift), Conviction Surges, Combat Morale modifiers, and Social Combat ("Mental Wounds": Rhetorical Strain, Doubt, Ego Wounds, and Cognitive Dissonance).
8. [Combat Architecture, Morale & Ranged Firefights](combat/combat.md) (`id: combat`)
   - Combat philosophy, anatomical trauma over Hit Points, Count-Up Action Clock (`TC` & `action_time_factor`), Psychological Drives & 5 Morale Conditions, the **"Envelope & Fill" Tactical Pool ($P_{\text{base}} = C_{\text{combat}} + 3N$)**, and Ranged Firefight mechanics (Worked Examples 2 & 3).
9. [Melee Combat, Close-Quarters Tactics & The Tactical Triad](combat/melee_combat.md) (`id: melee_combat`)
   - Close-quarters anatomy, the **Tactical Triad (`Tempo`, `Control`, `Force` — `TCF`)**, `Guard` & Seizing the Initiative, `Root` vs. throws/sweeps, the Six Core Tactical Allocations (`TC`, `TF`, `CF`, `Pure T/C/F`), Melee Weapon Reach & Profiles, and the Viktor vs. Elena tick-by-tick walkthrough.

### Part V: Action Economy, Health, Movement, Environment & World Systems
10. [Action Economy, Time Scales & The Count-Up Tick Clock](systems/action_economy.md) (`id: action_economy`)
    - Exact temporal calibration (`1 Tick = 0.25s`, `4 Ticks = 1s`, `20 Ticks = 5s Tactical Window`, `240 Ticks = 1 min`), Count-Up Tick Clock mechanics, Base Time Cost (`Base TC`) reference tables, how Characteristics (`SPD`, `DEX`, `ANA`, `INS`, `ITU`, `OODA`) and Health/Encumbrance (`action_time_factor`) affect action costs, and `OODA` action pivots.
11. [Health, Anatomical Trauma, Fatigue & Mental Conditions](systems/health_and_trauma.md) (`id: health_and_trauma`)
    - Anatomical `1d20` Hit Location & Called Shot tables, short-term Fatigue (`Levels 0–5`) & asphyxiation, long-term Structural Wounds (`Tiers 1–5`), Broken Bones & Dislocations, Hemorrhaging (`Grades 0–4` & `10.0-Unit` Blood Volume track), Systemic Wound Shock (`WSR`), acute sensory/physiological conditions (`Blinded`, `Stunned`, `Dizzy`, `Nausea`, `Unconscious`), and Mental Trauma (`Acute Mental Shock`, `Panic`, `Dissociation`, `Catatonia`).
12. [Movement, Encumbrance & Environmental Survival](systems/movement_and_environment.md) (`id: movement_and_environment`)
    - Combat movement gaits (`m/tick` and `m/s`), climbing/swimming/jumping/zero-G, overland travel (`km/h`), Encumbrance (`FOR` & `MAS` base capacity, ergonomic load multipliers, `Tiers 0–4`), Hydration & Dehydration, Nutrition & Starvation, Temperature extremes (Hypothermia, Frostbite, Heatstroke), Sleep Deprivation, Altitude Hypoxia, Toxins, and Radiation.
13. [Equipment, Armor, Stealth, Vehicles & World Systems (Reference Stubs)](systems/world_and_equipment_stubs.md) (`id: world_and_equipment_stubs`)
    - Armor Penetration vs. Behind-Armor Blunt Trauma (`BABT`), Weapon/Tool Reliability & Malfunctions, Multi-Sensory Stealth & Illumination/Acoustics, Vehicles & Chases, Economy (`SQM` upkeep, `[Access]` gates, `Heat`), and Downtime/Extended Project Clocks.
