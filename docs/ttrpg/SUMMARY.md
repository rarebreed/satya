# Satya TTRPG Rules Book — Summary & Table of Contents

This manifest defines the canonical chapter ordering, identifiers, categories, cross-references, and Rust engine module mappings for the **Satya Computer-Assisted RPG (CARPG) Rules Book**. It is consumed by:
- The **Satya Rust Engine & Web Frontend** (`satya-engine`, `satya-signaling` `/api/rules`, and the `satya-web` Left Tabbed Notebook **Rules Book** pane)
- The **Interactive Marimo Rules Book & Static HTML Generator** (`notebooks/satya_rules_book.py` & `python -m satya_ml.build_rules_book`)
- The **Satya AI Game Master** (`query_satya_rules` in `src/satya_ml/ai_gm/tools.py`)

---

## Table of Contents

### Part I: Foundation & Mathematical Calibration (`01-foundation/`)
1. [An RPG for a More Civilized Age: Design Philosophy](01-foundation/design_philosophy.md) (`id: design_philosophy`)
   - Origins of Satya, realism vs. escapism, and how Satya solves 17 structural flaws of traditional RPG systems.
2. [Mathematical Notation, Variables & Formula Glossary](01-foundation/mathematical_notation_glossary.md) (`id: notation_glossary`)
   - Canonical reference for every mathematical variable, subscript (`_P`, `_GM`, `_n`, `_eff`), dice/cascade symbol ($\text{TN}_P$, $\text{TN}_{GM}$, $\delta_{\text{over}}$, $\delta_{\text{under}}$, $\Delta M$), attribute abbreviation, OODA/Action Clock metric ($D_{\text{cog}}$, $\text{TC}$), and Tactical Triad formula ($P_{\text{base}}$, $P_{\text{eff}}$, $T_{\max}, C_{\max}, F_{\max}$) used across the rules book.
3. [Probability & Calibration of the Cascading Dice Mechanic](01-foundation/probability_and_calibration.md) (`id: probability`)
   - Mathematical foundation, distribution topology (skewness, modality, tail behavior), why $K = 20$ outperforms $K = 10$ and $K = 100$, $C$ vs. $N$ sensitivity, avoiding the "Skilled Botch Paradox," and Always-Contested Task Resolution via the **1-Pair Tiered GM Resistance Rule (`4`-pt over/under tiers)**.

### Part II: Core Mechanics, Task Resolution & Action Economy (`02-core-mechanics/`)
4. [Cascading `2d20` Engine & Task Resolution](02-core-mechanics/cascading_d20_and_tasks.md) (`id: tasks`)
   - Canonical Two-List Cascading (`2d20`) dice mechanic, Player Personal Target $\text{TN}_P = 31 - C_P + \Delta_{\text{tool}}$ vs. Hidden GM Resistance Target $\text{TN}_{GM} = 21 - \Delta_{\text{ext}}$ (`1-Pair Tiered GM Resistance`), carry modes (`markov_step`, `cumulative_snowball`), 7-Way Contested Outcome Matrix (`CleanSuccess`, `GrazingTieSuccess`, `LuckyBreakSuccess`, `SuccessWithFloorBreach`, `BotchFailure`), 5-step Task Pipeline, Tool Tiers, Gate Classifications (`Class A / B / C`), Extended Project Clocks, and Sarah's contested lockpicking walkthrough.
5. [Master Task Requirements Catalog](02-core-mechanics/task_requirements_catalog.md) (`id: task_requirements_catalog`)
   - Canonical Section 8 reference table defining gate classes, primary/complementary skills ($N_{\text{min}}$), controlling characteristics ($C_P$), mandatory/situational tools ($\Delta_{\text{tool}}$), and hidden GM resistance modifiers ($\Delta_{\text{ext}}$) across all skill categories.
6. [Action Economy, OODA Loop, Time Scales & Stealth](02-core-mechanics/action_economy_and_ooda.md) (`id: action_economy`)
   - Temporal calibration (`1 Tick = 0.25s`, `4 Ticks = 1s`, `20 Ticks = 5s Tactical Window`), Full OODA Loop & Cognitive Decision Score ($D_{\text{cog}}$ — "Quick Thinking"), Count-Up Tick Clock mechanics, Base Time Cost (`Base TC`) reference tables, `action_time_factor`, `OODA` action pivots, and Multi-Sensory Stealth & Detection (`SIG`, `HEA`, `OLF`, `ITU`).

### Part III: Character Architecture, Skills, Lifepaths & Economy (`03-character/`)
7. [Characteristics, Senses, Psyche & Aptitudes](03-character/characteristics_and_aptitudes.md) (`id: characteristics`)
   - Physical (`SPD`, `FOR`, `DEX`, `KIN`, `HLT`, `FIT`, `MAS`, `HGT`), Mental (`MEM`, `ANA`, `INS`, `CRE`, `DIS`, `FOC`), Social (`COM`, `PRE`, `ELO`), Sensory (`SIG`, `HEA`, `TAC`, `OLF`, `TAS`, `ITU`), and Psyche (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`) attributes, derived metrics (`POW`, `MMF`, `OODA`, `D_cog`, `CMP`), and domain Aptitudes ($A \in [-2, +3]$).
8. [Skill System & Hierarchical Skill Trees 1–4](03-character/skills_and_trees.md) (`id: skills_and_trees`)
   - Talents (`TAL`) vs. Physical (`PHY`), Knowledge (`KNO`), and Hybrid (`PHY/KNO`) skills, Multi-Category taxonomy, `[Access]` restriction tiers, Prerequisite Rules, and Hierarchical Skill Trees 1–4 (Mathematics, Physical Sciences/Trades, Computing/AI 2.0, and Biology/Medicine/Biotech).
9. [Hierarchical Skill Trees 5–8: Combat, Vehicles, Social & Tradecraft](03-character/skills_trees_tactical_and_social.md) (`id: skills_trees_tactical_and_social`)
   - Hierarchical Skill Trees 5–8: Unarmed/Melee/Firearms/Heavy Ordnance (`Tree 5`), Athletics/Vehicles/Aviation/Drones (`Tree 6`), Social/Influence/Interrogation/PsyOps (`Tree 7`), and Literacy/Law/Intelligence/Tradecraft/Artistic (`Tree 8`).
10. [Lifepath Character Creation, Socioeconomic Upkeep & Heat](03-character/lifepaths_and_economy.md) (`id: lifepaths_and_economy`)
    - Chronological Lifepath education & career progression (Ages 0–18+), School Quality (`SQM`), `DIS + Mental + APT` advancement rolls, Mateo Vance & Dr. Priya Nair walkthroughs, Socioeconomic Tier (`SQM 1–10`) lifestyle upkeep, Black-Market `[Access]` procurement gates, and Operational `Heat`.

### Part IV: Psyche, Morale & Social Combat (`04-psyche-and-social/`)
11. [Psyche, Morale & Social Combat](04-psyche-and-social/psyche_morale_and_social_combat.md) (`id: psyche_and_social`)
    - Acting against Psyche (`DIS` Friction Checks & Trait Drift), Purpose & Conviction Surges, Psychological Drives (`Harm:Other` vs. `Self:Protect`), Motivation ($\Delta_{\text{mot}}$), Aggression triggers, the Five Conditions of Morale (`High`, `Steady`, `Wavering`, `Broken`, `Panic`), and Social Combat ("Mental Wounds": Rhetorical Strain, Doubt, Ego Wounds, and Cognitive Dissonance).

### Part V: Combat Architecture, Close Quarters & Firefights (`05-combat/`)
12. [Combat Architecture, Tactical Triad & Pools](05-combat/tactical_triad_and_pools.md) (`id: tactical_triad`)
    - Combat design philosophy, anatomical trauma over Hit Points, Count-Up Action Clock overview, and the **"Envelope & Fill" Tactical Pool Architecture ($P_{\text{base}} = C_{\text{combat}} + 3N$)** across `Tempo`, `Control`, and `Force` ($T_{\max}, C_{\max}, F_{\max}$).
13. [Melee Combat, Close-Quarters Tactics & The Tactical Triad](05-combat/melee_combat.md) (`id: melee_combat`)
    - Close-quarters anatomy, `Tempo / Control / Force` in melee, `Guard` & Seizing the Initiative, `Root` vs. throws/sweeps, the Six Core Tactical Allocations (`TC`, `TF`, `CF`, `Pure T/C/F`), Melee Weapon Reach & Profiles, and the Viktor vs. Elena & Clinch/Throw walkthroughs.
14. [Granular Ballistics, Ammunition Physics & Firearm Engineering](05-combat/firearms_and_ballistics.md) (`id: firearms_and_ballistics`)
    - Four coupled firearm engineering chains, Ammunition & Internal/External Ballistics ($m_{\text{proj}}$, $m_{\text{prop}}$ powder grains $\to$ impact kinetic energy $E_k$, $\eta_{\text{bbl}}$, $\eta_{\text{action}}$), Master Ammunition Reference Table (`9mm` to `.50 BMG`), Feed Systems, Terminal Ballistics ($I_{\text{KE}} + I_{\text{cal}} + W_{\text{yaw}}$ inverse-accuracy tumbling wound channels), Action Types, Weight, Center of Gravity ($\Delta_{\text{CoG}}$), Overall Length (`OAL`), and Free-Floating vs. Clamped Barrel Specs.
15. [Ranged Combat, Weapon Reliability, Master Catalogs & Armor](05-combat/ranged_combat_and_armor.md) (`id: ranged_combat_and_armor`)
    - Ranged Fire Modes & Recoil Bracing ($F \ge R_{\text{eff}}$) in the Tactical Triad, the Accuracy–Reliability Tolerance Trade-Off ($\tau_{\text{tol}}$ vs. $\text{Rel}$), Malfunctions (`Stovepipe` vs. `Double-Feed`), Clear Jam & Field-Strip times, Master Granular Firearms Catalog (`12` benchmark weapons), Body Armor Engineering (`PR` vs. `BABT`), and worked firefights (Mateo vs. Sgt. Vance & AKM vs. `6.5mm Creedmoor` at 400m).

### Part VI: Health, Trauma, Movement, Vehicles & Survival (`06-health-and-survival/`)
16. [Health, Anatomical Trauma, Fatigue & Mental Conditions](06-health-and-survival/health_and_trauma.md) (`id: health_and_trauma`)
    - Anatomical `1d20` Hit Location & Called Shot tables, short-term Fatigue (`Levels 0–5`) & asphyxiation, long-term Structural Wounds (`Tiers 1–5`), Broken Bones & Dislocations, Hemorrhaging (`Grades 0–4` & `10.0-Unit` Blood Volume track), Systemic Wound Shock (`WSR`), acute sensory/physiological conditions (`Blinded`, `Stunned`, `Dizzy`, `Nausea`, `Unconscious`), Mental Trauma (`Acute Mental Shock`, `Panic`, `Dissociation`, `Catatonia`), and Medical Triage.
17. [Movement, Encumbrance, Vehicles & Environmental Survival](06-health-and-survival/movement_vehicles_and_environment.md) (`id: movement_and_environment`)
    - Combat movement gaits (`m/tick` and `m/s`), climbing/swimming/jumping/zero-G, overland travel (`km/h`), Encumbrance (`FOR` & `MAS` base capacity, ergonomic load multipliers, `Tiers 0–4`), Vehicles & High-Speed Chases in the Tactical Triad (plus Collisions & `G-LOC`), Hydration & Dehydration, Nutrition & Starvation, Temperature extremes (Hypothermia, Frostbite, Heatstroke), Sleep Deprivation, Altitude Hypoxia, Toxins, and Radiation.

---

## Reciprocal Rust Engine Module Index (`crates/satya-engine/src/`)

| TTRPG Chapter (`docs/ttrpg/`) | Canonical `chapter_id` | Rust Engine Implementation Modules (`crates/satya-engine/src/`) |
| :--- | :--- | :--- |
| [`01-foundation/design_philosophy.md`](01-foundation/design_philosophy.md) | `design_philosophy` | [`lib.rs`](../../crates/satya-engine/src/lib.rs), [`rules_book.rs`](../../crates/satya-engine/src/rules_book.rs), [`rules_book_html.rs`](../../crates/satya-engine/src/rules_book_html.rs) |
| [`01-foundation/mathematical_notation_glossary.md`](01-foundation/mathematical_notation_glossary.md) | `notation_glossary` | [`lib.rs`](../../crates/satya-engine/src/lib.rs), [`characteristics.rs`](../../crates/satya-engine/src/characteristics.rs), [`dice_probabilities.rs`](../../crates/satya-engine/src/dice_probabilities.rs), [`combat.rs`](../../crates/satya-engine/src/combat.rs) |
| [`01-foundation/probability_and_calibration.md`](01-foundation/probability_and_calibration.md) | `probability` | [`dice_probabilities.rs`](../../crates/satya-engine/src/dice_probabilities.rs), [`contested_cascade.rs`](../../crates/satya-engine/src/contested_cascade.rs) |
| [`02-core-mechanics/cascading_d20_and_tasks.md`](02-core-mechanics/cascading_d20_and_tasks.md) | `tasks` | [`dice_probabilities.rs`](../../crates/satya-engine/src/dice_probabilities.rs), [`contested_cascade.rs`](../../crates/satya-engine/src/contested_cascade.rs), [`task_resolution.rs`](../../crates/satya-engine/src/task_resolution.rs), [`walkthroughs.rs`](../../crates/satya-engine/src/walkthroughs.rs) |
| [`02-core-mechanics/task_requirements_catalog.md`](02-core-mechanics/task_requirements_catalog.md) | `task_requirements_catalog` | [`task_resolution.rs`](../../crates/satya-engine/src/task_resolution.rs) (`CANONICAL_BENCHMARK_TASKS`) |
| [`02-core-mechanics/action_economy_and_ooda.md`](02-core-mechanics/action_economy_and_ooda.md) | `action_economy` | [`action_economy.rs`](../../crates/satya-engine/src/action_economy.rs), [`action_catalog_stealth.rs`](../../crates/satya-engine/src/action_catalog_stealth.rs) |
| [`03-character/characteristics_and_aptitudes.md`](03-character/characteristics_and_aptitudes.md) | `characteristics` | [`characteristics.rs`](../../crates/satya-engine/src/characteristics.rs) |
| [`03-character/skills_and_trees.md`](03-character/skills_and_trees.md) | `skills_and_trees` | [`skills_trees.rs`](../../crates/satya-engine/src/skills_trees.rs), [`skills_catalog.rs`](../../crates/satya-engine/src/skills_catalog.rs) (Trees 1–4) |
| [`03-character/skills_trees_tactical_and_social.md`](03-character/skills_trees_tactical_and_social.md) | `skills_trees_tactical_and_social` | [`skills_trees.rs`](../../crates/satya-engine/src/skills_trees.rs), [`skills_catalog.rs`](../../crates/satya-engine/src/skills_catalog.rs) (Trees 5–8) |
| [`03-character/lifepaths_and_economy.md`](03-character/lifepaths_and_economy.md) | `lifepaths_and_economy` | [`lifepath_economy.rs`](../../crates/satya-engine/src/lifepath_economy.rs), [`lifepath_catalog.rs`](../../crates/satya-engine/src/lifepath_catalog.rs), [`walkthroughs.rs`](../../crates/satya-engine/src/walkthroughs.rs) |
| [`04-psyche-and-social/psyche_morale_and_social_combat.md`](04-psyche-and-social/psyche_morale_and_social_combat.md) | `psyche_and_social` | [`psyche_social.rs`](../../crates/satya-engine/src/psyche_social.rs), [`social_combat.rs`](../../crates/satya-engine/src/social_combat.rs) |
| [`05-combat/tactical_triad_and_pools.md`](05-combat/tactical_triad_and_pools.md) | `tactical_triad` | [`combat.rs`](../../crates/satya-engine/src/combat.rs), [`combat_techniques.rs`](../../crates/satya-engine/src/combat_techniques.rs) |
| [`05-combat/melee_combat.md`](05-combat/melee_combat.md) | `melee_combat` | [`combat.rs`](../../crates/satya-engine/src/combat.rs), [`combat_techniques.rs`](../../crates/satya-engine/src/combat_techniques.rs), [`walkthroughs.rs`](../../crates/satya-engine/src/walkthroughs.rs) |
| [`05-combat/firearms_and_ballistics.md`](05-combat/firearms_and_ballistics.md) | `firearms_and_ballistics` | [`ballistics.rs`](../../crates/satya-engine/src/ballistics.rs) |
| [`05-combat/ranged_combat_and_armor.md`](05-combat/ranged_combat_and_armor.md) | `ranged_combat_and_armor` | [`ranged_combat.rs`](../../crates/satya-engine/src/ranged_combat.rs), [`barrier_ballistics.rs`](../../crates/satya-engine/src/barrier_ballistics.rs), [`combat_techniques.rs`](../../crates/satya-engine/src/combat_techniques.rs), [`ballistics.rs`](../../crates/satya-engine/src/ballistics.rs), [`walkthroughs.rs`](../../crates/satya-engine/src/walkthroughs.rs) |
| [`06-health-and-survival/health_and_trauma.md`](06-health-and-survival/health_and_trauma.md) | `health_and_trauma` | [`health_survival.rs`](../../crates/satya-engine/src/health_survival.rs), [`status_and_triage.rs`](../../crates/satya-engine/src/status_and_triage.rs) |
| [`06-health-and-survival/movement_vehicles_and_environment.md`](06-health-and-survival/movement_vehicles_and_environment.md) | `movement_and_environment` | [`environment_vehicles.rs`](../../crates/satya-engine/src/environment_vehicles.rs), [`environmental_hazards.rs`](../../crates/satya-engine/src/environmental_hazards.rs) |
