# Satya TTRPG Skill System & Hierarchical Skill Trees

This chapter defines the canonical **Skill System, Skill Classifications, and the 8 Hierarchical Skill Trees** for **Satya** (set on near-future Earth, 5 years in the future, ca. 2031–2035). It integrates directly with:
- **[Characteristics, Senses, Psyche & Aptitudes](characteristics_and_aptitudes.md)** (for Controlling Characteristics $C \in [1, 20]$ and domain Aptitudes $A \in [-2, +3]$),
- **[Cascading `2d20` Engine & Task Resolution](../02-core-mechanics/cascading_d20_and_tasks.md)** (for the `2d20` Watershed Cascade, Tool Dependency Tiers, Task Prerequisite Gates Class A/B/C, and the Master Task Requirements Table), and
- **[Lifepath Character Creation, Economy & Heat](lifepaths_and_economy.md)** (for chronological skill acquisition from childhood through adult careers).

---

## 1. Core Design Philosophy

Satya's skill system is built on five foundational design pillars:

1. **Lifepath Grounding & Plausibility ("No Scientist with Random Full-Auto")**:
   A character's skills are the organic record of their upbringing, socioeconomic class, schooling quality, life events, and adult careers. A university researcher cannot arbitrarily purchase *Fully Automatic Weapons* or *Clandestine Narcotics Synthesis* at character creation unless their lifepath explicitly routed them through military service, law enforcement tactical training, or a criminal syndicate.
2. **Deep Prerequisite Hierarchies (The Skill Tree)**:
   Complex disciplines are gated behind foundational skills. A character who drops out of school at age 15 with only basic *Arithmetic* (or at best *Trigonometry* if exceptionally gifted and disciplined) lacks the *Calculus*, *Linear Algebra*, and *Differential Equations* required to learn advanced *Aerospace Engineering*, *Quantum Cryptography*, or *Machine Learning Architecture*. Conversely, street or military lifepaths open restricted and illicit branches inaccessible to sheltered academics.
3. **Nature of Proficiency (Talents vs. Physical Skills vs. Knowledge Skills)**:
   Every skill is classified by how humans acquire and execute it:
   - **Talents (`TAL`)**: Innate, universal human capacities (such as *Running*, *Singing*, *Observation*, or *Brawling*) in which every non-impaired person possesses a baseline proficiency (`N = 1`).
   - **Physical Skills (`PHY`)**: Trained motor, somatic, and kinesthetic abilities (`N = 0` untrained) governed by physical attributes and muscle memory.
   - **Knowledge Skills (`KNO`)**: Trained cognitive, academic, and analytical disciplines (`N = 0` untrained) governed by mental attributes, study discipline, and formal or street education.
   - **Hybrid Skills (`PHY/KNO`)**: High-complexity disciplines (such as *Trauma Surgery* or *Explosive Ordnance Disposal*) that require both deep theoretical mastery and precise physical execution.
4. **Multi-Category Taxonomy**:
   Skills belong to **one or more categories** (`Combat`, `Athletic`, `Technical`, `Science`, `Knowledge`, `Artistic`, `Social`, `Fieldcraft`, `Medical`, `Cyber`), allowing lifepaths, aptitudes, and equipment systems to reference overlapping domains cleanly.
5. **Explicit Tool & Task Dependencies**:
   Every skill specifies whether it requires tools (`[Tool: None]`, `[Tool: Situational]`, or `[Tool: Mandatory]`), and task resolution rules in **[Cascading `2d20` Engine & Task Resolution](../02-core-mechanics/cascading_d20_and_tasks.md)** explicitly define which skill(s), minimum skill levels, controlling characteristics, and tools are necessary to attempt any given action.

---

## 2. Skill Classifications, Categories, Access Gates, and Tool Tags

### 2.1 Skill Nature: Talent (`TAL`), Physical (`PHY`), Knowledge (`KNO`), and Hybrid (`PHY/KNO`)

| Skill Type | Untrained Baseline ($N$) | Defaulting When Untrained | Fatigue & Injury Impact ($\Delta_{\text{int}}$) | Decay & Study Profile |
| :--- | :---: | :--- | :--- | :--- |
| **Talent (`TAL`)** | **`N = 1`** *(Everyone starts at Level 1)* | Always usable at `N = 1` with $\Delta_{\text{ext}} = 0$ on basic tasks. | Standard $\Delta_{\text{int}}$ (`FOR`/`SPD`/`KIN`/`FIT` tasks) or half $\Delta_{\text{int}}$ (`SIG`/`HEA`/`ITU`/`PRE`/`ELO` tasks). | Never decays below `N = 1`; improves through natural use or dedicated training. |
| **Physical (`PHY`)** | **`N = 0`** *(Untrained)* | Can default to a root Talent or Parent Skill at `N = 1` with **$\Delta_{\text{ext}} = +4$ to $+8$** *(unless task is gated)*. | Full physical Fatigue and Wound Shock apply to both $\delta_{\text{under}}$ and Action Time Cost (`TC`). | Retained via muscle memory; requires physical practice or drills to advance. |
| **Knowledge (`KNO`)** | **`N = 0`** *(Untrained)* | **Cannot default** unless the character holds a directly related Parent/Sibling `KNO` skill (at **$\Delta_{\text{ext}} = +4$ to $+8$**). | Physical wounds impose half $\Delta_{\text{int}}$ unless in active combat; mental exhaustion/sleep deprivation degrades `FOC`/`ANA`/`MEM` and imposes full $\Delta_{\text{int}}$. | Requires literacy/math prerequisites and formal instruction, mentorship, or study materials. |
| **Hybrid (`PHY/KNO`)** | **`N = 0`** *(Untrained)* | Requires **both** the parent `KNO` prerequisite and `PHY` motor control; cannot be attempted untrained. | Full physical and cognitive $\Delta_{\text{int}}$ penalties apply. | Requires clinical, laboratory, or tactical field apprenticeship. |

---

### 2.2 Multi-Category System

Every skill is assigned **one or more** category tags. A single skill frequently bridges multiple domains:

- **`Athletic`**: Locomotion, gymnastics, swimming, climbing, physical conditioning, sports.
- **`Combat`**: Unarmed striking/grappling, melee weapons, firearms, heavy ordnance, tactical movement.
- **`Technical`**: Mechanical repair, electronics, fabrication, piloting, gunsmithing, structural/industrial engineering.
- **`Science`**: Mathematics, physics, chemistry, biology, earth/space sciences, formal research methodology.
- **`Knowledge`**: Literacy, humanities, history, law, finance, geopolitics, bureaucracy, intelligence analysis, street lore.
- **`Cyber`**: Programming, systems administration, AI/ML engineering, network exploitation, electronic warfare, cryptography.
- **`Medical`**: First aid, paramedicine, pharmacology, clinical diagnostics, trauma surgery, cyber-implant medicine.
- **`Artistic`**: Vocal/instrumental music, visual/digital arts, culinary arts, creative writing, acting/performance, synth-media.
- **`Social`**: Persuasion, deception, intimidation, negotiation, command, interrogation, etiquette, streetwise.
- **`Fieldcraft`**: Stealth, observation, tracking, urban/wilderness survival, lockpicking, surveillance, tradecraft.

> [!TIP]
> **Why Multi-Category Tags Matter**:
> Multi-category membership determines which **Aptitudes** ([`characteristics_and_aptitudes.md`](characteristics_and_aptitudes.md#10-aptitudes-a-in--2-3-human-baseline-0)), **Lifepath Career Tables** ([`lifepaths_and_economy.md`](lifepaths_and_economy.md)), and **Facility/Workshop Bonuses** apply to a skill. For example, *Explosive Ordnance Disposal (EOD)* carries the tags `[Combat, Technical, Science]`, while *Forensic Pathology* carries `[Science, Medical, Knowledge]`.

---

### 2.3 Lifepath Access & Restriction Tiers (`[Access]`)

To prevent immersion-breaking character builds, every skill is gated by an **Access Tier**:

| Access Tag | Who Can Acquire This Skill | Examples |
| :--- | :--- | :--- |
| **`[Access: Universal]`** | Any character via childhood upbringing, public schooling, hobbies, or daily life. | *Running*, *Singing*, *Reading*, *Writing*, *Arithmetic*, *Brawling*, *Ground Vehicle Driving*. |
| **`[Access: Academic]`** | Characters enrolled in secondary Honors/STEM tracks, Universities, or Research Institutes (plus math/literacy prerequisites). | *Calculus*, *Differential Equations*, *Organic Chemistry*, *Quantum Physics*, *Machine Learning Architecture*. |
| **`[Access: Vocational / Licensed]`** | Characters in accredited trade schools, apprenticeships, medical/legal residencies, or licensed corporate roles. | *Trauma Surgery*, *Commercial Aviation*, *CNC Fabrication*, *High-Voltage Electrical*, *Corporate Law*. |
| **`[Access: Restricted]`** | Strictly gated behind **Military, Police/SWAT, Federal Law Enforcement, Intelligence Agency, or licensed PMC** lifepaths. | *Fully Automatic Weapons*, *Heavy Weapons (ATGM/MANPADS)*, *Artillery/Mortars*, *CQB Breaching*, *CBRN Warfare*, *Counter-Interrogation*. |
| **`[Access: Illicit]`** | Gated behind **Street Gang, Organized Crime Syndicate, Cartel, Black-Hat Collective, Insurgent Cell, or Prison** lifepaths. | *Street Full-Auto / Sear Modification*, *Narcotics Synthesis*, *Hotwiring & Transponder Spoofing*, *Forgery & Identity Washing*, *Extortion*, *Improvised Munitions*. |

---

### 2.4 Tool Dependency Tags (`[Tool]`)

Every skill in the tables below carries a tool dependency tag (see **[Cascading `2d20` Engine & Task Resolution](../02-core-mechanics/cascading_d20_and_tasks.md#44-tool-dependency-classification-tool--2035-smart-tech-modifiers)** for the full Tool Tier $\Delta_{\text{ext}}$ table):
- **`[Tool: None]`**: Relies entirely on the body, voice, senses, or mind.
- **`[Tool: Situational]`**: Unassisted basic tasks work at $\Delta_{\text{ext}} = 0$, while complex tasks require tools or suffer an Improvised Penalty (`+4` to `+12`).
- **`[Tool: Mandatory]`**: Cannot be rolled at all without the required weapon, instrument, vehicle, lab, or computing rig.

---

## 3. The Hierarchical Skill Tree & Prerequisite Mechanics

### 3.1 Rules of the Skill Tree

Skills in Satya form a **Directed Acyclic Graph (DAG)**—a multi-tier skill tree where broad foundational roots unlock specialized branches:

1. **Parent Skill Thresholds ($N_{\text{parent}} \ge X$)**:
   To learn a Child Skill at Level `N = 1`, the character must already possess its Parent Skill(s) at or above the specified minimum rank (typically `N = 2` for intermediate skills, `N = 3` for professional specializations, and `N = 4` for advanced scientific/surgical disciplines).
2. **The Foundation Ceiling Rule ($N_{\text{child}} \le N_{\text{parent}} + 2$)**:
   A specialized child skill cannot exceed its primary foundational parent skill by more than **2 levels** (`N_child <= N_parent + 2`).
   - *Example*: If a character has *Calculus & Linear Algebra* at `N = 2`, their *Machine Learning & Neural Architecture* skill is capped at `N = 4` until they raise *Calculus & Linear Algebra* to `N = 3+`.
3. **Tree Defaulting Rules**:
   - **Down-Tree Defaulting (Parent $\to$ Child)**: If a character lacks a specific Child Skill (`N = 0`) but possesses its immediate Parent Skill at `N >= 1`, and the task is marked **Defaultable**, they may roll their Parent Skill level $N_{\text{parent}}$ with an External Difficulty Penalty of **$\Delta_{\text{ext}} = +4$ per tier step** down the tree.
   - **Sibling Defaulting (Branch $\leftrightarrow$ Branch)**: If two skills share an immediate Parent (e.g., *Pistols* and *Long Guns* under firearms fundamentals, or *Organic Chemistry* and *Analytical Chemistry*), a character may default from one sibling to another at **$\Delta_{\text{ext}} = +4$ to $+6$**, provided the target skill is not `[Tool/Training Locked]`.
   - **Hard-Gated Nodes (`[No Default]`)**: High-tier `KNO` and `PHY/KNO` nodes (such as *Neurosurgery*, *Differential Equations*, *Zero-Day Exploit Development*, or *Rotary-Wing Piloting*) **cannot be defaulted** from low-tier roots.

---

### 3.2 Tree 1: The Mathematics & Quantitative Spine (`Science`, `Knowledge`)

Mathematics is the master gatekeeper for all hard sciences, engineering, advanced computing/AI, ballistics, and quantitative finance. Rather than relying on a single generic intelligence score, foundational arithmetic combines **Analysis (`ANA`)** and **Memory (`MEM`)**, statistical and geometric modeling pairs **Analysis (`ANA`)** with **Insight (`INS`)**, multi-step calculus/differential equations require **Analysis (`ANA`)** and **Focus (`FOC`)**, and frontier cryptographic/tensor research demands **Analysis (`ANA`)** and **Creativity (`CRE`)**.

```mermaid
flowchart TD
    Arith["<b>Arithmetic</b><br/>(KNO | ANA/MEM | Tier 1)<br/>[Access: Universal]"]
    Alg["<b>Algebra & Geometry</b><br/>(KNO | ANA | Tier 2)<br/>Req: Arithmetic ≥ 2"]
    Trig["<b>Trigonometry & Pre-Calculus</b><br/>(KNO | ANA/INS, Min ANA ≥ 10 | Tier 3)<br/>Req: Algebra & Geometry ≥ 2"]
    Stats1["<b>Applied Statistics & Probability</b><br/>(KNO | ANA/INS, Min ANA ≥ 10 | Tier 3)<br/>Req: Algebra & Geometry ≥ 2"]
    Calc["<b>Calculus & Linear Algebra</b><br/>(KNO | ANA/FOC, Min ANA ≥ 11 | Tier 4)<br/>Req: Trigonometry ≥ 3"]
    Disc["<b>Discrete Math & Graph Theory</b><br/>(KNO | ANA/INS, Min ANA ≥ 11 | Tier 4)<br/>Req: Algebra & Geometry ≥ 3"]
    DiffEq["<b>Differential Equations & Numerical Methods</b><br/>(KNO | ANA/FOC, Min ANA ≥ 12 | Tier 5)<br/>Req: Calculus & Linear Algebra ≥ 3"]
    Stoch["<b>Stochastic Modeling & Econometrics</b><br/>(KNO | ANA/INS, Min ANA ≥ 12 | Tier 5)<br/>Req: Calc ≥ 2, Applied Stats ≥ 3"]
    CryptoMath["<b>Cryptographic Number Theory & Tensor Math</b><br/>(KNO | ANA/CRE, Min ANA ≥ 14 | Tier 6)<br/>Req: Discrete Math ≥ 3, Calc ≥ 3"]

    Arith --> Alg
    Alg --> Trig
    Alg --> Stats1
    Alg --> Disc
    Trig --> Calc
    Calc --> DiffEq
    Calc --> Stoch
    Stats1 --> Stoch
    Disc --> CryptoMath
    Calc --> CryptoMath
```

| Skill Name | Type | Categories | Control ($C$) | Min $C$ | Prerequisites | Access | Tools | What It Unlocks / Governs |
| :--- | :---: | :--- | :---: | :---: | :--- | :--- | :---: | :--- |
| **Arithmetic** | `KNO` | `Knowledge`, `Science` | `ANA/MEM` | — | None | `Universal` | `None` | Counting, fractions, percentages, street bookkeeping, basic budgeting; unlocks *Algebra & Geometry*. |
| **Algebra & Geometry** | `KNO` | `Science`, `Knowledge` | `ANA` | `ANA 8` | *Arithmetic* $\ge 2$ | `Universal` | `Situational` | Linear equations, spatial area/volume, trade drafting, basic carpentry/machining math; unlocks *Trigonometry*, *General Chemistry*, *Programming Fundamentals*. |
| **Trigonometry & Pre-Calculus** | `KNO` | `Science`, `Knowledge` | `ANA/INS` | `ANA 10` | *Algebra & Geometry* $\ge 2$ | `Universal` / `Academic` | `Situational` | Sine/cosine waves, vectors, triangulation, navigation, long-range ballistics (*Precision Rifle*), *Classical Physics*, *Surveying*. |
| **Applied Statistics & Probability** | `KNO` | `Science`, `Knowledge` | `ANA/INS` | `ANA 10` | *Algebra & Geometry* $\ge 2$ | `Academic` | `Situational` | Hypothesis testing, epidemiology, actuarial analysis, quality control, data science foundations. |
| **Discrete Math & Graph Theory** | `KNO` | `Science`, `Cyber` | `ANA/INS` | `ANA 11` | *Algebra & Geometry* $\ge 3$ | `Academic` | `Situational` | Boolean logic, combinatorics, formal algorithms, network topology, cryptography foundations. |
| **Calculus & Linear Algebra** | `KNO` | `Science` | `ANA/FOC` | `ANA 11, FOC 10` | *Trigonometry* $\ge 3$ | `Academic` | `Situational` | Derivatives, integrals, matrix operations, vector spaces; **mandatory prerequisite** for university *Engineering*, *Machine Learning*, and *Advanced Physics*. |
| **Differential Equations & Numerical Methods** | `KNO` | `Science` | `ANA/FOC` | `ANA 12, FOC 11` | *Calculus & Linear Algebra* $\ge 3$ | `Academic` | `Mandatory` *(Compute/Paper)* | Fluid dynamics, orbital mechanics, structural stress modeling, control theory, advanced simulation. |
| **Stochastic Modeling & Econometrics** | `KNO` | `Science`, `Knowledge` | `ANA/INS` | `ANA 12, INS 11` | *Calculus* $\ge 2$, *Statistics* $\ge 3$ | `Academic` / `Licensed` | `Mandatory` | Algorithmic trading, macroeconomic forecasting, complex risk modeling. |
| **Cryptographic Number Theory & Tensor Math** | `KNO` | `Science`, `Cyber` | `ANA/CRE` | `ANA 14, FOC 12` | *Discrete Math* $\ge 3$, *Calculus* $\ge 3$ | `Academic` / `Restricted` | `Mandatory` | Post-quantum cryptography, zero-knowledge proofs, theoretical physics, frontier AI 2.0 foundation models. |

---

### 3.3 Tree 2: Physical Sciences, Engineering & Practical Trades (`Science`, `Technical`)

Notice how this tree splits into **two parallel tracks**:
- The **Practical Trade Track** (accessible with *Arithmetic* or *Algebra* via vocational training, blue-collar work, or street gangs; controlled by **Dexterity (`DEX`)**, **Force (`FOR`)**, **Focus (`FOC`)**, **Analysis (`ANA`)**, and **Creativity (`CRE`)**).
- The **Formal Science & Engineering Track** (strictly gated behind *Trigonometry*, *Calculus*, and *Differential Equations*; controlled by **Analysis (`ANA`)**, **Memory (`MEM`)**, **Insight (`INS`)**, **Focus (`FOC`)**, and **Creativity (`CRE`)**).

```mermaid
flowchart TD
    subgraph MathGates["Mathematics Prerequisites"]
        Alg2["Algebra & Geometry ≥ 2"]
        Trig2["Trigonometry ≥ 2"]
        Calc3["Calculus & Linear Algebra ≥ 3"]
        Diff3["Differential Equations ≥ 3"]
    end

    subgraph TradeTrack["Practical Trades & Fabrication Track (Vocational / Street)"]
        Tinker["<b>Basic Tool Use & Tinkering</b><br/>(PHY/KNO | DEX/ANA | Tier 1)"]
        AutoMech["<b>Automotive & EV Mechanics</b><br/>(PHY/KNO | DEX/ANA | Tier 2)"]
        BasicElec["<b>Basic Electronics & Soldering</b><br/>(PHY/KNO | DEX/FOC | Tier 2)"]
        Metal["<b>Metalworking, Welding & Machining</b><br/>(PHY | DEX/FOR | Tier 2)"]
        Gunsmith["<b>Gunsmithing & Armorer</b><br/>(PHY/KNO | DEX/ANA | Tier 3)<br/>[Access: Vocational/Restricted/Illicit]"]
        ImprovMun["<b>Improvised Munitions & Street Mods</b><br/>(PHY/KNO | DEX/CRE | Tier 3)<br/>[Access: Illicit]"]
        CNC["<b>CNC, 3D Additive & Robotics Repair</b><br/>(PHY/KNO | ANA/DEX | Tier 3)"]
    end

    subgraph SciEngTrack["Formal Physical Science & Engineering Track (Academic)"]
        GenSci["<b>General Physical Science</b><br/>(KNO | ANA/MEM | Tier 1)"]
        Phys["<b>Classical & Applied Physics</b><br/>(KNO | ANA/INS, Min ANA ≥ 11 | Tier 2)"]
        Chem["<b>General Chemistry</b><br/>(KNO | ANA/MEM, Min ANA ≥ 10 | Tier 2)"]
        OrgChem["<b>Organic & Analytical Chemistry</b><br/>(KNO | ANA/MEM, Min ANA ≥ 12 | Tier 3)"]
        StreetChem["<b>Clandestine Narcotics Synthesis</b><br/>(PHY/KNO | MEM/DEX | Tier 3)<br/>[Access: Illicit]"]
        MechEng["<b>Mechanical & Structural Engineering</b><br/>(KNO | ANA/INS, Min ANA ≥ 12 | Tier 4)"]
        ElecEng["<b>Electrical & Microelectronic Engineering</b><br/>(KNO | ANA/FOC, Min ANA ≥ 12 | Tier 4)"]
        ChemEng["<b>Chemical & Energetics Engineering</b><br/>(KNO | ANA/FOC, Min ANA ≥ 13 | Tier 4)"]
        AeroEng["<b>Aerospace, Robotics & Mechatronics Eng.</b><br/>(KNO | ANA/CRE, Min ANA ≥ 13 | Tier 5)"]
        QuantumPhys["<b>Quantum & Nuclear Physics</b><br/>(KNO | ANA/INS, Min ANA ≥ 14 | Tier 5)"]
    end

    Tinker --> AutoMech
    Tinker --> BasicElec
    Tinker --> Metal
    Metal --> Gunsmith
    BasicElec --> ImprovMun
    Gunsmith --> ImprovMun
    BasicElec --> CNC
    Metal --> CNC

    GenSci --> Phys
    GenSci --> Chem
    Alg2 --> Chem
    Trig2 --> Phys
    Chem --> OrgChem
    Chem --> StreetChem
    OrgChem --> ChemEng
    Phys --> MechEng
    Phys --> ElecEng
    Calc3 --> MechEng
    Calc3 --> ElecEng
    MechEng --> AeroEng
    ElecEng --> AeroEng
    Diff3 --> AeroEng
    Diff3 --> QuantumPhys
    Phys --> QuantumPhys
```

| Skill Name | Type | Categories | Control ($C$) | Min $C$ | Prerequisites | Access | Tools |
| :--- | :---: | :--- | :---: | :---: | :--- | :--- | :---: |
| **Basic Tool Use & Tinkering** | `PHY/KNO` | `Technical` | `DEX/ANA` | — | None | `Universal` | `Mandatory` |
| **Automotive & EV Mechanics** | `PHY/KNO` | `Technical` | `DEX/ANA` | — | *Basic Tool Use* $\ge 1$, *Arithmetic* $\ge 1$ | `Universal` / `Vocational` | `Mandatory` |
| **Basic Electronics & Soldering** | `PHY/KNO` | `Technical` | `DEX/FOC` | `DEX 9, SIG 9` | *Basic Tool Use* $\ge 1$, *Arithmetic* $\ge 2$ | `Universal` / `Vocational` | `Mandatory` |
| **Metalworking, Welding & Machining** | `PHY` | `Technical` | `DEX/FOR` | `FOR 9, FOC 9` | *Basic Tool Use* $\ge 1$, *Algebra & Geometry* $\ge 1$ | `Vocational` | `Mandatory` |
| **Gunsmithing & Armorer** | `PHY/KNO` | `Technical`, `Combat` | `DEX/ANA` | `DEX 10, TAC 9` | *Metalworking* $\ge 2$ or *Basic Tool Use* $\ge 2$ | `Vocational` / `Restricted` / `Illicit` | `Mandatory` |
| **Improvised Munitions & Street Mods** | `PHY/KNO` | `Technical`, `Combat` | `DEX/CRE` | `DEX 10, CRE 10` | *Basic Electronics* $\ge 2$ or *Gunsmithing* $\ge 2$ | `Illicit` / `Restricted` | `Mandatory` |
| **CNC, 3D Additive & Robotics Repair** | `PHY/KNO` | `Technical`, `Cyber` | `ANA/DEX` | `ANA 10` | *Basic Electronics* $\ge 2$, *Metalworking* $\ge 2$, *Algebra* $\ge 2$ | `Vocational` | `Mandatory` |
| **General Physical Science** | `KNO` | `Science`, `Knowledge` | `ANA/MEM` | — | *Reading* $\ge 2$, *Arithmetic* $\ge 2$ | `Universal` | `Situational` |
| **General Chemistry** | `KNO` | `Science` | `ANA/MEM` | `ANA 10` | *General Physical Science* $\ge 2$, *Algebra & Geometry* $\ge 2$ | `Academic` | `Mandatory` *(Lab)* |
| **Classical & Applied Physics** | `KNO` | `Science` | `ANA/INS` | `ANA 11` | *General Physical Science* $\ge 2$, *Trigonometry* $\ge 2$ | `Academic` | `Situational` |
| **Clandestine Narcotics Synthesis** | `PHY/KNO` | `Science`, `Technical` | `MEM/DEX` | `MEM 9, OLF 9` | *General Chemistry* $\ge 1$ *(or gang cook apprenticeship)* | `Illicit` | `Mandatory` *(Lab)* |
| **Organic & Analytical Chemistry** | `KNO` | `Science` | `ANA/MEM` | `ANA 12, MEM 11` | *General Chemistry* $\ge 3$, *Algebra & Geometry* $\ge 3$ | `Academic` | `Mandatory` *(Lab)* |
| **Mechanical & Structural Engineering** | `KNO` | `Science`, `Technical` | `ANA/INS` | `ANA 12` | *Classical Physics* $\ge 3$, *Calculus & Linear Algebra* $\ge 3$ | `Academic` / `Licensed` | `Mandatory` |
| **Electrical & Microelectronic Engineering** | `KNO` | `Science`, `Technical`, `Cyber` | `ANA/FOC` | `ANA 12` | *Classical Physics* $\ge 3$, *Calculus & Linear Algebra* $\ge 3$ | `Academic` / `Licensed` | `Mandatory` |
| **Chemical & Energetics Engineering** | `KNO` | `Science`, `Technical` | `ANA/FOC` | `ANA 13` | *Organic Chemistry* $\ge 3$, *Calculus & Linear Algebra* $\ge 3$ | `Academic` / `Restricted` | `Mandatory` |
| **Aerospace, Robotics & Mechatronics Eng.** | `KNO` | `Science`, `Technical`, `Cyber` | `ANA/CRE` | `ANA 13, INS 11` | *Mechanical Eng.* $\ge 3$, *Electrical Eng.* $\ge 3$, *Diff. Equations* $\ge 3$ | `Academic` / `Restricted` | `Mandatory` |
| **Quantum & Nuclear Physics** | `KNO` | `Science` | `ANA/INS` | `ANA 14, CRE 12` | *Classical Physics* $\ge 4$, *Differential Equations* $\ge 3$ | `Academic` / `Restricted` | `Mandatory` |

---

### 3.4 Tree 3: Computing, AI 2.0 & Cyberwarfare (`Cyber`, `Technical`, `Science`)

In 2031–2035, computing spans basic user operation, script-kiddie/street hacking, enterprise software engineering, and frontier AI 2.0 neural architecture. Notice how **Programming (`ANA/FOC`)**, **Systems Architecture (`ANA/INS`)**, and **Exploit Development / AI Orchestration (`ANA/CRE`)** leverage distinct combinations of Mental Primary Characteristics.

```mermaid
flowchart TD
    DigLit["<b>Digital & AI Literacy</b><br/>(KNO | ANA/MEM | Tier 1)<br/>[Access: Universal]"]
    SysAdmin["<b>OS Administration & Hardware Tech</b><br/>(KNO/PHY | ANA/DEX | Tier 2)"]
    Prog["<b>Programming Fundamentals</b><br/>(KNO | ANA/FOC, Min ANA ≥ 10 | Tier 2)<br/>Req: Algebra & Geometry ≥ 2"]
    StreetHack["<b>Street Phishing, Skimming & Script Intrusion</b><br/>(KNO | CRE/INS | Tier 2)<br/>[Access: Illicit]"]
    NetEng["<b>Network Engineering & Cloud Infrastructure</b><br/>(KNO | ANA/MEM, Min ANA ≥ 11 | Tier 3)"]
    SoftEng["<b>Software Engineering & Systems Architecture</b><br/>(KNO | ANA/INS, Min ANA ≥ 11 | Tier 3)<br/>Req: Programming ≥ 3, Discrete Math ≥ 2"]
    PromptAI["<b>AI 2.0 Agent Orchestration & Synth-Media</b><br/>(KNO | CRE/ANA | Tier 2)"]
    NetExploit["<b>Network Exploitation & Penetration Testing</b><br/>(KNO | ANA/CRE, Min ANA ≥ 12 | Tier 4)<br/>[Access: Vocational/Restricted/Illicit]"]
    DigForensics["<b>Digital Forensics & Incident Response (DFIR)</b><br/>(KNO | ANA/INS, Min ANA ≥ 11 | Tier 4)"]
    MLArch["<b>Machine Learning & Neural Architecture</b><br/>(KNO | ANA/INS, Min ANA ≥ 13 | Tier 4)<br/>Req: SoftEng ≥ 3, Calculus ≥ 3, Stats ≥ 3"]
    ZeroDay["<b>Zero-Day Exploit Dev & Reverse Engineering</b><br/>(KNO | ANA/CRE, Min ANA ≥ 14 | Tier 5)<br/>[Access: Restricted/Illicit]"]
    EW["<b>Electronic Warfare, SIGINT & C-UAS Jamming</b><br/>(KNO/PHY | ANA/FOC, Min ANA ≥ 11 | Tier 4)<br/>[Access: Restricted]"]

    DigLit --> SysAdmin
    DigLit --> Prog
    DigLit --> StreetHack
    DigLit --> PromptAI
    SysAdmin --> NetEng
    Prog --> SoftEng
    NetEng --> NetExploit
    Prog --> NetExploit
    StreetHack --> NetExploit
    NetEng --> DigForensics
    SoftEng --> MLArch
    NetExploit --> ZeroDay
    SoftEng --> ZeroDay
    NetEng --> EW
```

| Skill Name | Type | Categories | Control ($C$) | Min $C$ | Prerequisites | Access | Tools |
| :--- | :---: | :--- | :---: | :---: | :--- | :--- | :---: |
| **Digital & AI Literacy** | `KNO` | `Cyber`, `Knowledge` | `ANA/MEM` | — | *Reading* $\ge 1$ | `Universal` | `Mandatory` |
| **AI 2.0 Agent Orchestration & Synth-Media** | `KNO` | `Cyber`, `Artistic`, `Knowledge` | `CRE/ANA` | `CRE 9, ANA 9` | *Digital & AI Literacy* $\ge 2$ | `Universal` | `Mandatory` |
| **OS Administration & Hardware Tech** | `KNO/PHY` | `Cyber`, `Technical` | `ANA/DEX` | `ANA 9` | *Digital & AI Literacy* $\ge 2$ | `Universal` / `Vocational` | `Mandatory` |
| **Street Phishing, Skimming & Script Intrusion** | `KNO` | `Cyber`, `Fieldcraft` | `CRE/INS` | `CRE 9` | *Digital & AI Literacy* $\ge 2$ | `Illicit` | `Mandatory` |
| **Programming Fundamentals** | `KNO` | `Cyber`, `Technical` | `ANA/FOC` | `ANA 10, FOC 9` | *Digital & AI Literacy* $\ge 2$, *Algebra & Geometry* $\ge 2$ | `Universal` / `Academic` | `Mandatory` |
| **Network Engineering & Cloud Infrastructure** | `KNO` | `Cyber`, `Technical` | `ANA/MEM` | `ANA 11` | *OS Administration* $\ge 2$, *Programming Fundamentals* $\ge 1$ | `Vocational` / `Academic` | `Mandatory` |
| **Software Engineering & Systems Architecture** | `KNO` | `Cyber`, `Science` | `ANA/INS` | `ANA 11, FOC 10` | *Programming Fundamentals* $\ge 3$, *Discrete Math* $\ge 2$ | `Academic` / `Vocational` | `Mandatory` |
| **Network Exploitation & Penetration Testing** | `KNO` | `Cyber`, `Fieldcraft` | `ANA/CRE` | `ANA 12, CRE 11` | *Network Eng.* $\ge 2$ & *Programming* $\ge 2$ *(or Street Intrusion $\ge 3$ + Programming $\ge 2$)* | `Licensed` / `Restricted` / `Illicit` | `Mandatory` |
| **Digital Forensics & Incident Response** | `KNO` | `Cyber`, `Knowledge` | `ANA/INS` | `ANA 11, INS 11, FOC 10` | *Network Engineering* $\ge 2$, *Programming* $\ge 2$ | `Licensed` / `Restricted` | `Mandatory` |
| **Electronic Warfare, SIGINT & C-UAS Jamming** | `KNO/PHY` | `Cyber`, `Combat`, `Technical` | `ANA/FOC` | `ANA 11, FOC 11` | *Network Engineering* $\ge 2$ or *Basic Electronics* $\ge 3$ | `Restricted` | `Mandatory` |
| **Machine Learning & Neural Architecture** | `KNO` | `Cyber`, `Science` | `ANA/INS` | `ANA 13, CRE 11` | *Software Eng.* $\ge 3$, *Calculus & Linear Algebra* $\ge 3$, *Statistics* $\ge 3$ | `Academic` | `Mandatory` |
| **Zero-Day Exploit Dev & Reverse Engineering** | `KNO` | `Cyber`, `Science` | `ANA/CRE` | `ANA 14, CRE 12, FOC 12` | *Network Exploitation* $\ge 3$, *Software Eng.* $\ge 3$, *Discrete Math* $\ge 3$ | `Restricted` / `Illicit` | `Mandatory` |

---

### 3.5 Tree 4: Biology, Medicine, Biotech & Forensics (`Medical`, `Science`)

Medicine showcases the distinction between **Memory (`MEM`)** (anatomical and pharmacological recall), **Insight (`INS`)** (clinical diagnostics and pathology pattern recognition), **Dexterity (`DEX`) + Focus (`FOC`) + Tactile (`TAC`)** (surgical execution), and **Discipline (`DIS`)** (tactical combat casualty care under fire).

```mermaid
flowchart TD
    FirstAid["<b>First Aid & Basic Life Support</b><br/>(PHY/KNO | DEX/DIS | Tier 1)<br/>[Access: Universal]"]
    GenBio["<b>General Biology</b><br/>(KNO | MEM/ANA | Tier 1)<br/>Req: Reading ≥ 2"]
    EMT["<b>Paramedicine & Tactical Combat Casualty Care (TCCC)</b><br/>(PHY/KNO | DEX/DIS ≥ 10 | Tier 2)<br/>[Access: Vocational/Restricted/Illicit]"]
    Anat["<b>Human Anatomy & Physiology</b><br/>(KNO | MEM/INS, Min MEM ≥ 10 | Tier 2)<br/>Req: General Biology ≥ 2"]
    MicroBio["<b>Microbiology, Genetics & Immunology</b><br/>(KNO | ANA/MEM, Min ANA ≥ 11 | Tier 3)<br/>Req: General Biology ≥ 3, Gen Chemistry ≥ 2"]
    Pharm["<b>Pharmacology & Toxicology</b><br/>(KNO | MEM/ANA, Min MEM ≥ 12 | Tier 3)<br/>Req: Anat ≥ 2, Organic Chemistry ≥ 2"]
    Diag["<b>Clinical Diagnostics & Internal Medicine</b><br/>(KNO | INS/ANA, Min INS ≥ 12 | Tier 4)<br/>Req: Anatomy ≥ 3, Pharmacology ≥ 2"]
    Psych["<b>Clinical Psychology & Psychiatry</b><br/>(KNO | INS/ELO, Min INS ≥ 11 | Tier 3)<br/>Req: Anatomy ≥ 2 or Social Science ≥ 2"]
    Forensic["<b>Forensic Science & Pathology</b><br/>(KNO/PHY | INS/ANA, Min INS ≥ 12 | Tier 4)<br/>Req: Anatomy ≥ 3, Org Chem ≥ 2"]
    TraumaSurg["<b>Trauma & General Surgery</b><br/>(PHY/KNO | DEX/FOC, Min DEX ≥ 13, DIS ≥ 12 | Tier 5)<br/>Req: Diagnostics ≥ 3, EMT ≥ 2"]
    NeuroCyber["<b>Neurosurgery & Cyber-Implant Integration</b><br/>(PHY/KNO | DEX/FOC, Min DEX ≥ 14, DIS ≥ 13 | Tier 6)<br/>Req: Trauma Surgery ≥ 3, Basic Electronics ≥ 2"]
    CRISPR["<b>Biotechnology & Synthetic Genomics</b><br/>(KNO | ANA/CRE, Min ANA ≥ 13 | Tier 5)<br/>Req: Microbiology ≥ 3, Org Chem ≥ 3, Stats ≥ 2"]

    FirstAid --> EMT
    GenBio --> Anat
    GenBio --> MicroBio
    Anat --> EMT
    Anat --> Pharm
    Anat --> Diag
    Anat --> Psych
    Anat --> Forensic
    Pharm --> Diag
    Diag --> TraumaSurg
    EMT --> TraumaSurg
    TraumaSurg --> NeuroCyber
    MicroBio --> CRISPR
```

| Skill Name | Type | Categories | Control ($C$) | Min $C$ | Prerequisites | Access | Tools |
| :--- | :---: | :--- | :---: | :---: | :--- | :--- | :---: |
| **First Aid & Basic Life Support** | `PHY/KNO` | `Medical` | `DEX/DIS` | — | None | `Universal` | `Situational` |
| **General Biology** | `KNO` | `Science` | `MEM/ANA` | `MEM 9` | *Reading* $\ge 2$ | `Universal` / `Academic` | `Situational` |
| **Paramedicine & TCCC (Combat Medic)** | `PHY/KNO` | `Medical`, `Combat` | `DEX/DIS` | `DEX 10, DIS 10, FOC 10` | *First Aid* $\ge 2$ | `Vocational` / `Restricted` / `Illicit` | `Mandatory` |
| **Human Anatomy & Physiology** | `KNO` | `Science`, `Medical` | `MEM/INS` | `MEM 10, ANA 10` | *General Biology* $\ge 2$ | `Academic` / `Vocational` | `Situational` |
| **Microbiology, Genetics & Immunology** | `KNO` | `Science`, `Medical` | `ANA/MEM` | `ANA 11, MEM 11` | *General Biology* $\ge 3$, *General Chemistry* $\ge 2$ | `Academic` | `Mandatory` *(Lab)* |
| **Pharmacology & Toxicology** | `KNO` | `Science`, `Medical` | `MEM/ANA` | `MEM 12, ANA 11` | *Human Anatomy* $\ge 2$, *Organic Chemistry* $\ge 2$ | `Academic` / `Licensed` | `Mandatory` |
| **Clinical Psychology & Psychiatry** | `KNO` | `Medical`, `Social`, `Knowledge` | `INS/ELO` | `INS 11, ELO 10` | *Human Anatomy* $\ge 2$, *Applied Statistics* $\ge 1$ | `Academic` / `Licensed` | `None` / `Situational` |
| **Clinical Diagnostics & Internal Medicine** | `KNO` | `Medical`, `Science` | `INS/ANA` | `INS 12, ANA 11, MEM 11` | *Human Anatomy* $\ge 3$, *Pharmacology* $\ge 2$ | `Licensed` *(Med School)* | `Situational` |
| **Forensic Science & Pathology** | `KNO/PHY` | `Science`, `Medical`, `Knowledge` | `INS/ANA` | `INS 12, ANA 11, FOC 11` | *Human Anatomy* $\ge 3$, *Organic Chemistry* $\ge 2$ | `Licensed` / `Restricted` | `Mandatory` *(Lab/Kit)* |
| **Trauma & General Surgery** | `PHY/KNO` | `Medical` | `DEX/FOC` | `DEX 13, FOC 12, DIS 12, TAC 11` | *Clinical Diagnostics* $\ge 3$, *Paramedicine* $\ge 2$ | `Licensed` / `Restricted` | `Mandatory` |
| **Neurosurgery & Cyber-Implant Integration** | `PHY/KNO` | `Medical`, `Cyber`, `Technical` | `DEX/FOC` | `DEX 14, FOC 13, DIS 13, TAC 12` | *Trauma Surgery* $\ge 3$, *Basic Electronics* $\ge 2$ | `Licensed` / `Illicit` *(Ripperdoc)* | `Mandatory` |
| **Biotechnology & Synthetic Genomics** | `KNO` | `Science`, `Medical` | `ANA/CRE` | `ANA 13, MEM 12` | *Microbiology* $\ge 3$, *Organic Chemistry* $\ge 3$, *Statistics* $\ge 2$ | `Academic` / `Restricted` | `Mandatory` *(Lab)* |

---

### 3.6 Tree 5: Combat — Unarmed, Melee, Firearms & Heavy Ordnance (`Combat`, `Athletic`, `Technical`)

This tree enforces the strict distinction between universal brawling, civilian/street weapon handling, and military/SWAT restricted ordnance, while leveraging Satya's physical and sensory characteristics:
- **Unarmed & Melee**: Uses **Force (`FOR`)** and **Speed (`SPD`)** ($\text{Power} = \text{Force} \times \text{Speed}$) for raw brawling, **Kinesthesis (`KIN`)** for martial arts footwork and grappling body control, and **Dexterity (`DEX`)** for precise blade work (see **[Melee Combat](../05-combat/melee_combat.md)**).
- **Ranged Marksmanship**: Pairs **Dexterity (`DEX`)** (trigger control and hand-eye coordination) with **Sight (`SIG`)** (visual target acquisition), **Speed (`SPD`)** (concealed fast-draw and CQB room entry), **Force (`FOR`)** (full-auto recoil control), or **Focus (`FOC`)** (long-range sniping; see **[Ranged Combat & Armor](../05-combat/ranged_combat_and_armor.md)**).

```mermaid
flowchart TD
    subgraph UnarmedMelee["Unarmed & Melee Combat Branch"]
        Brawl["<b>Brawling</b><br/>(TAL | FOR/SPD | Tier 0 — Starts N=1)<br/>[Access: Universal]"]
        StreetFight["<b>Street Fighting & Dirty Tricks</b><br/>(PHY | SPD/CRE | Tier 2)<br/>[Access: Illicit/Street]"]
        Striking["<b>Boxing, Kickboxing & Striking</b><br/>(PHY | SPD/KIN | Tier 2)"]
        Grapple["<b>Wrestling, Judo & BJJ Grappling</b><br/>(PHY | KIN/FOR | Tier 2)"]
        Martial["<b>Systematized Martial Arts</b><br/>(PHY | KIN/DIS ≥ 10 | Tier 3)"]
        CQC["<b>Lethal Close-Quarters Combat (CQC)</b><br/>(PHY | SPD/KIN, Min DIS ≥ 11 | Tier 3)<br/>[Access: Restricted]"]
        MeleeBasic["<b>Simple Melee Weapons (Club/Knife)</b><br/>(PHY | SPD/DEX | Tier 1)"]
        KnifeFight["<b>Knife Fighting & Close Blades</b><br/>(PHY | SPD/DEX ≥ 10 | Tier 2)<br/>[Access: Illicit/Restricted]"]
        Bludgeon["<b>Riot Batons & Impact Weapons</b><br/>(PHY | FOR/KIN | Tier 2)"]
        Sword["<b>Fencing & Long Blades</b><br/>(PHY | SPD/KIN ≥ 10 | Tier 2)"]
    end

    subgraph RangedOrdnance["Ranged Weapons & Heavy Ordnance Branch"]
        Throw["<b>Throwing</b><br/>(TAL | DEX/FOR | Tier 0 — Starts N=1)<br/>[Access: Universal]"]
        Grenade["<b>Grenades & Thrown Explosives</b><br/>(PHY | DEX/DIS | Tier 2)<br/>[Access: Restricted/Illicit]"]
        Pistol["<b>Pistols & Revolvers (Semi-Auto/Manual)</b><br/>(PHY | DEX/SIG | Tier 1)<br/>[Access: Universal/Licensed/Illicit]"]
        StreetShoot["<b>Concealed Draw & Street Shooting</b><br/>(PHY | SPD/DEX | Tier 2)<br/>[Access: Illicit/Licensed]"]
        CombatPistol["<b>Tactical Handgun & Point Shooting</b><br/>(PHY | DEX/FOC, Min SIG ≥ 10 | Tier 2)<br/>[Access: Restricted/Licensed]"]
        LongGun["<b>Long Guns (Rifles & Shotguns — Semi/Manual)</b><br/>(PHY | DEX/SIG | Tier 1)<br/>[Access: Universal/Licensed/Restricted]"]
        TacCarbine["<b>Tactical Carbine & CQB Room Entry</b><br/>(PHY | SPD/DEX ≥ 10 | Tier 2)<br/>[Access: Restricted/Illicit]"]
        FullAuto["<b>Fully Automatic Weapons & Machine Guns</b><br/>(PHY | DEX/FOR, Min FOR ≥ 10 | Tier 2)<br/><b>[Access: Restricted OR Illicit ONLY]</b>"]
        Sniper["<b>Precision Rifle & Long-Range Sniping</b><br/>(PHY/KNO | DEX/FOC, Min SIG ≥ 12 | Tier 3)<br/>Req: Long Guns ≥ 3, Trigonometry ≥ 2, DIS ≥ 11<br/><b>[Access: Restricted]</b>"]
        HeavyWep["<b>Heavy Weapons (ATGM, MANPADS, AGL)</b><br/>(PHY/KNO | DEX/ANA | Tier 3)<br/>Req: FullAuto ≥ 2 or LongGun ≥ 2<br/><b>[Access: Restricted]</b>"]
        Indirect["<b>Indirect Fire (Mortars & Artillery)</b><br/>(KNO/PHY | ANA/FOC | Tier 3)<br/>Req: Heavy Weapons ≥ 2, Trigonometry ≥ 2<br/><b>[Access: Restricted]</b>"]
        Demolitions["<b>Military Demolitions & Breaching Charges</b><br/>(PHY/KNO | DEX/ANA ≥ 10 | Tier 3)<br/><b>[Access: Restricted]</b>"]
        EOD["<b>Explosive Ordnance Disposal (EOD)</b><br/>(PHY/KNO | DEX/FOC ≥ 12, Min DIS ≥ 12 | Tier 4)<br/>Req: Demolitions ≥ 2, Basic Electronics ≥ 2<br/><b>[Access: Restricted]</b>"]
    end

    Brawl --> StreetFight
    Brawl --> Striking
    Brawl --> Grapple
    Striking --> Martial
    Grapple --> Martial
    Striking --> CQC
    Grapple --> CQC
    MeleeBasic --> KnifeFight
    MeleeBasic --> Bludgeon
    MeleeBasic --> Sword
    KnifeFight --> CQC

    Throw --> Grenade
    Pistol --> StreetShoot
    Pistol --> CombatPistol
    Pistol --> FullAuto
    LongGun --> TacCarbine
    LongGun --> FullAuto
    LongGun --> Sniper
    FullAuto --> HeavyWep
    LongGun --> HeavyWep
    HeavyWep --> Indirect
    Demolitions --> EOD
```

| Skill Name | Type | Categories | Control ($C$) | Min $C$ | Prerequisites | Access | Tools |
| :--- | :---: | :--- | :---: | :---: | :--- | :--- | :---: |
| **Brawling** | `TAL` | `Combat`, `Athletic` | `FOR/SPD` | — | None *(Starts at `N = 1`)* | `Universal` | `None` |
| **Street Fighting & Dirty Tricks** | `PHY` | `Combat`, `Fieldcraft` | `SPD/CRE` | — | *Brawling* $\ge 2$ | `Illicit` | `Situational` |
| **Boxing, Kickboxing & Striking** | `PHY` | `Combat`, `Athletic` | `SPD/KIN` | `SPD 9, KIN 9` | *Brawling* $\ge 1$ | `Universal` | `Situational` |
| **Wrestling, Judo & BJJ Grappling** | `PHY` | `Combat`, `Athletic` | `KIN/FOR` | `KIN 9, FOR 9` | *Brawling* $\ge 1$ | `Universal` | `None` |
| **Systematized Martial Arts** | `PHY` | `Combat`, `Athletic` | `KIN/DIS` | `KIN 10, DIS 10, SPD 10` | *Striking* $\ge 2$ or *Grappling* $\ge 2$ | `Universal` | `None` |
| **Lethal Close-Quarters Combat (CQC)** | `PHY` | `Combat` | `SPD/KIN` | `SPD 11, KIN 11, DIS 11` | *Striking* $\ge 2$, *Grappling* $\ge 2$, or *Knife Fighting* $\ge 2$ | `Restricted` | `Situational` |
| **Simple Melee Weapons (Club/Knife)** | `PHY` | `Combat` | `SPD/DEX` | — | *Brawling* $\ge 1$ | `Universal` | `Mandatory` |
| **Knife Fighting & Close Blades** | `PHY` | `Combat` | `SPD/DEX` | `DEX 10, SPD 10` | *Simple Melee Weapons* $\ge 2$ | `Illicit` / `Restricted` | `Mandatory` |
| **Riot Batons & Impact Weapons** | `PHY` | `Combat` | `FOR/KIN` | `FOR 9` | *Simple Melee Weapons* $\ge 1$ | `Restricted` / `Illicit` | `Mandatory` |
| **Fencing & Long Blades** | `PHY` | `Combat`, `Athletic` | `SPD/KIN` | `SPD 10, KIN 10, DEX 10` | *Simple Melee Weapons* $\ge 1$ | `Universal` | `Mandatory` |
| **Throwing** | `TAL` | `Combat`, `Athletic` | `DEX/FOR` | — | None *(Starts at `N = 1`)* | `Universal` | `Situational` |
| **Grenades & Thrown Explosives** | `PHY` | `Combat` | `DEX/DIS` | `DIS 9, FOR 8` | *Throwing* $\ge 2$ | `Restricted` / `Illicit` | `Mandatory` |
| **Pistols & Revolvers (Semi-Auto/Manual)** | `PHY` | `Combat` | `DEX/SIG` | — | None | `Universal` / `Licensed` / `Illicit` | `Mandatory` |
| **Concealed Draw & Street Shooting** | `PHY` | `Combat`, `Fieldcraft` | `SPD/DEX` | `SPD 10, DEX 10` | *Pistols & Revolvers* $\ge 2$, *Stealth/Concealment* $\ge 1$ | `Illicit` / `Licensed` / `Restricted` | `Mandatory` |
| **Tactical Handgun & Point Shooting** | `PHY` | `Combat` | `DEX/FOC` | `DEX 10, SIG 10, FOC 10` | *Pistols & Revolvers* $\ge 2$ | `Restricted` / `Licensed` | `Mandatory` |
| **Long Guns (Rifles/Shotguns — Semi/Manual)** | `PHY` | `Combat` | `DEX/SIG` | — | None | `Universal` / `Licensed` / `Restricted` | `Mandatory` |
| **Tactical Carbine & CQB Room Entry** | `PHY` | `Combat` | `SPD/DEX` | `SPD 10, DEX 10, FOC 10` | *Long Guns* $\ge 2$ | `Restricted` / `Illicit` | `Mandatory` |
| **Fully Automatic Weapons & Machine Guns** | `PHY` | `Combat` | `DEX/FOR` | **`FOR 10`** | *Long Guns* $\ge 2$ or *Pistols* $\ge 2$ | **`Restricted` or `Illicit` ONLY** | `Mandatory` |
| **Precision Rifle & Long-Range Sniping** | `PHY/KNO` | `Combat`, `Science`, `Fieldcraft` | `DEX/FOC` | `DEX 12, SIG 12, FOC 12, DIS 11` | *Long Guns* $\ge 3$, ***Trigonometry* $\ge 2$**, *Stealth/Camo* $\ge 2$ | **`Restricted` ONLY** | `Mandatory` |
| **Heavy Weapons (ATGM, MANPADS, AGL)** | `PHY/KNO` | `Combat`, `Technical` | `DEX/ANA` | `FOR 10, FOC 10, ANA 9` | *Fully Automatic Weapons* $\ge 2$ or *Long Guns* $\ge 2$ | **`Restricted` (or Cartel/Paramilitary)** | `Mandatory` |
| **Indirect Fire (Mortars & Artillery)** | `KNO/PHY` | `Combat`, `Science`, `Technical` | `ANA/FOC` | `ANA 10` | *Heavy Weapons* $\ge 2$, ***Trigonometry* $\ge 2$** | **`Restricted` ONLY** | `Mandatory` |
| **Military Demolitions & Breaching** | `PHY/KNO` | `Combat`, `Technical` | `DEX/ANA` | `ANA 10, DIS 10` | *Arithmetic* $\ge 2$ | **`Restricted` (or Insurgent)** | `Mandatory` |
| **Explosive Ordnance Disposal (EOD)** | `PHY/KNO` | `Combat`, `Technical`, `Science` | `DEX/FOC` | `DEX 12, FOC 12, DIS 12, TAC 11, ANA 11` | *Military Demolitions* $\ge 2$, *Basic Electronics* $\ge 2$ | **`Restricted` ONLY** | `Mandatory` |

> [!IMPORTANT]
> **Why *Precision Rifle / Sniping* Uses `DEX/FOC` (with `SIG >= 12, DIS >= 11` and *Trigonometry* $\ge 2$)**:
> Shooting a rifle at 50 meters uses *Long Guns* (`DEX/SIG`). Engaging a target at 1,200 meters requires **Sight (`SIG >= 12`)** to resolve the target, **Trigonometry & Pre-Calculus (`N >= 2`)** (`ANA/INS`) to compute milliradian holdovers, Coriolis drift, and high-angle cosine corrections, **Discipline (`DIS >= 11`)** to remain motionless for hours, and **Dexterity + Focus (`DEX/FOC`)** to break the trigger between heartbeats. Similarly, controlling an automatic rifle or sear-modified machine pistol requires **Force (`FOR >= 10`)** (`DEX/FOR`) and `[Access: Restricted or Illicit]`.

---

### 3.7 Tree 6: Athletics, Vehicles, Aviation & Drone Operations (`Athletic`, `Technical`, `Fieldcraft`)

Athletics and vehicle handling directly showcase **Fitness (`FIT`)**, **Speed (`SPD`)**, **Force (`FOR`)**, and **Kinesthesis (`KIN`)** (body awareness in 3D space—essential for parkour, gymnastics, motorcycles, and rotary-wing VTOL aircraft; see **[Movement, Vehicles & Environment](../06-health-and-survival/movement_vehicles_and_environment.md)**).

```mermaid
flowchart TD
    subgraph AthleticsBranch["Athletics & Somatic Mobility"]
        Run["<b>Running</b> (TAL | FIT/SPD | Starts N=1)"]
        Jump["<b>Jumping</b> (TAL | FOR/KIN | Starts N=1)"]
        Climb["<b>Climbing</b> (TAL | FOR/KIN | Starts N=1)"]
        Swim["<b>Swimming</b> (TAL | FIT/KIN | Starts N=1)"]
        Sprint["<b>Sprinting & Distance Conditioning</b> (PHY | FIT/SPD)"]
        Parkour["<b>Parkour & Urban Free-Running</b> (PHY | KIN/SPD, Min KIN ≥ 11)"]
        Mount["<b>Technical Mountaineering & Rappelling</b> (PHY | FIT/FOR, Min FOR ≥ 10)"]
        Scuba["<b>SCUBA & Combat Diving</b> (PHY/KNO | FIT/DIS ≥ 10)"]
        Acro["<b>Acrobatics, Gymnastics & Escape</b> (PHY | KIN/DEX, Min KIN ≥ 12)"]
    end

    subgraph VehiclesDrones["Vehicles, Aviation & Unmanned Systems (2031–2035)"]
        Drive["<b>Ground Vehicle (Civilian Auto)</b><br/>(PHY | DEX/SIG | Tier 1)"]
        TacDrive["<b>High-Speed Pursuit & Evasive Driving</b><br/>(PHY | SPD/DEX ≥ 11 | Tier 2)<br/>[Access: Restricted/Illicit]"]
        HeavyDrive["<b>Heavy Commercial & Armored Vehicles</b><br/>(PHY | DEX/FOC | Tier 2)"]
        Moto["<b>Motorcycle & Light Urban EV</b><br/>(PHY | KIN/SPD ≥ 10 | Tier 1)"]
        Boat["<b>Watercraft & Small Boat Handling</b><br/>(PHY | DEX/INS | Tier 1)"]
        DroneBasic["<b>Consumer UAV & FPV Drone Operation</b><br/>(PHY/KNO | DEX/SIG | Tier 1)"]
        TacDrone["<b>Tactical Recon, Swarm & Loitering Munition</b><br/>(PHY/KNO | FOC/ANA, Min FOC ≥ 11 | Tier 3)<br/>[Access: Restricted/Illicit]"]
        FixedWing["<b>Fixed-Wing Aviation</b><br/>(PHY/KNO | DEX/FOC ≥ 11 | Tier 3)<br/>Req: Trigonometry ≥ 2, Physics ≥ 1<br/>[Access: Licensed/Restricted]"]
        RotorWing["<b>Rotary-Wing & Tiltrotor VTOL Aviation</b><br/>(PHY/KNO | KIN/DEX ≥ 12, FOC ≥ 11 | Tier 4)<br/>Req: Trigonometry ≥ 2, Physics ≥ 1<br/>[Access: Licensed/Restricted]"]
    end

    Run --> Sprint
    Run --> Parkour
    Jump --> Parkour
    Climb --> Parkour
    Climb --> Mount
    Jump --> Acro
    Swim --> Scuba

    Drive --> TacDrive
    Drive --> HeavyDrive
    DroneBasic --> TacDrone
```

| Skill Name | Type | Categories | Control ($C$) | Min $C$ | Prerequisites | Access | Tools |
| :--- | :---: | :--- | :---: | :---: | :--- | :--- | :---: |
| **Running** | `TAL` | `Athletic` | `FIT/SPD` | — | None *(Starts at `N = 1`)* | `Universal` | `None` |
| **Jumping** | `TAL` | `Athletic` | `FOR/KIN` | — | None *(Starts at `N = 1`)* | `Universal` | `None` |
| **Climbing** | `TAL` | `Athletic`, `Fieldcraft` | `FOR/KIN` | — | None *(Starts at `N = 1`)* | `Universal` | `Situational` |
| **Swimming** | `TAL` | `Athletic`, `Fieldcraft` | `FIT/KIN` | — | None *(Starts at `N = 1`)* | `Universal` | `None` |
| **Sprinting & Distance Conditioning** | `PHY` | `Athletic` | `FIT/SPD` | `FIT 10` | *Running* $\ge 2$ | `Universal` | `None` |
| **Parkour & Urban Free-Running** | `PHY` | `Athletic`, `Fieldcraft` | `KIN/SPD` | `KIN 11, SPD 10, FOR 10` | *Running* $\ge 2$, *Jumping* $\ge 2$, *Climbing* $\ge 2$ | `Universal` / `Illicit` | `None` |
| **Technical Mountaineering & Rappelling** | `PHY` | `Athletic`, `Fieldcraft` | `FIT/FOR` | `FOR 10, FIT 10, KIN 10, DIS 10` | *Climbing* $\ge 2$ | `Universal` / `Restricted` | `Mandatory` |
| **SCUBA & Combat Diving** | `PHY/KNO` | `Athletic`, `Fieldcraft` | `FIT/DIS` | `FIT 10, DIS 10, KIN 9` | *Swimming* $\ge 2$, *General Physical Science* $\ge 1$ | `Licensed` / `Restricted` | `Mandatory` |
| **Acrobatics, Gymnastics & Escape** | `PHY` | `Athletic` | `KIN/DEX` | `KIN 12, SPD 10` | *Jumping* $\ge 2$ | `Universal` / `Illicit` | `None` |
| **Ground Vehicle (Civilian Auto)** | `PHY` | `Technical` | `DEX/SIG` | — | None | `Universal` | `Mandatory` |
| **High-Speed Pursuit & Evasive Driving** | `PHY` | `Technical`, `Combat` | `SPD/DEX` | `SPD 11, DEX 11, SIG 10, FOC 10` | *Ground Vehicle* $\ge 2$ | `Restricted` / `Illicit` | `Mandatory` |
| **Heavy Commercial & Armored Vehicles** | `PHY` | `Technical` | `DEX/FOC` | `DEX 9, SIG 9` | *Ground Vehicle* $\ge 2$ | `Vocational` / `Restricted` | `Mandatory` |
| **Motorcycle & Light Urban EV** | `PHY` | `Technical`, `Athletic` | `KIN/SPD` | `KIN 10, SPD 10` | None | `Universal` / `Illicit` | `Mandatory` |
| **Watercraft & Small Boat Handling** | `PHY` | `Technical`, `Fieldcraft` | `DEX/INS` | — | *Swimming* $\ge 1$ | `Universal` / `Vocational` | `Mandatory` |
| **Consumer UAV & FPV Drone Operation** | `PHY/KNO` | `Technical`, `Cyber` | `DEX/SIG` | `DEX 10` | *Digital & AI Literacy* $\ge 1$ | `Universal` | `Mandatory` |
| **Tactical Recon, Swarm & Loitering Munition** | `PHY/KNO` | `Combat`, `Technical`, `Cyber` | `FOC/ANA` | `FOC 11, ANA 10, DEX 10` | *Consumer UAV* $\ge 2$ | `Restricted` / `Illicit` | `Mandatory` |
| **Fixed-Wing Aviation** | `PHY/KNO` | `Technical` | `DEX/FOC` | `DEX 11, FOC 11, ANA 11, SIG 10` | *Trigonometry* $\ge 2$, *Classical Physics* $\ge 1$ | `Licensed` / `Restricted` | `Mandatory` |
| **Rotary-Wing & Tiltrotor VTOL Aviation** | `PHY/KNO` | `Technical` | `KIN/DEX` | `KIN 12, DEX 12, FOC 11, ANA 11` | *Trigonometry* $\ge 2$, *Classical Physics* $\ge 1$ | `Licensed` / `Restricted` | `Mandatory` |

---

### 3.8 Tree 7: Social, Influence, Interrogation & Psychological Warfare (`Social`, `Knowledge`, `Fieldcraft`)

Social skills leverage Satya's three distinct Social Primary Characteristics—**Comeliness (`COM`)** (physical beauty), **Presence (`PRE`)** (silent charisma and physical magnetism), and **Eloquence (`ELO`)** (how well one speaks and verbally influences others)—paired with **Insight (`INS`)**, **Intuition (`ITU`)**, **Analysis (`ANA`)**, **Creativity (`CRE`)**, **Memory (`MEM`)**, **Discipline (`DIS`)**, and **Focus (`FOC`)** (see **[Psyche, Morale & Social Combat](../04-psyche-and-social/psyche_morale_and_social_combat.md)**).

```mermaid
flowchart TD
    subgraph SocialTalents["Foundational Social Talents (Start at N=1)"]
        Charm["<b>Charm & Rapport</b><br/>(TAL | PRE/ELO | Tier 0 — Starts N=1)<br/>[Access: Universal]"]
        Persuade["<b>Persuasion & Rhetoric</b><br/>(TAL | ELO/ANA | Tier 0 — Starts N=1)<br/>[Access: Universal]"]
        Deceive["<b>Deception & Fast-Talk</b><br/>(TAL | ELO/CRE | Tier 0 — Starts N=1)<br/>[Access: Universal]"]
        Intim["<b>Intimidation & Coercion</b><br/>(TAL | PRE/DIS | Tier 0 — Starts N=1)<br/>[Access: Universal]"]
        Empathy["<b>Empathy & Read Person</b><br/>(TAL | INS/ITU | Tier 0 — Starts N=1)<br/>[Access: Universal]"]
    end

    subgraph InfluenceBranch["Influence, Etiquette & Deception Specializations"]
        Seduct["<b>Seduction & Honey-Trap Tradecraft</b><br/>(PHY/SOC | COM/PRE ≥ 10 | Tier 2)<br/>Req: Charm ≥ 2, Deception or Empathy ≥ 2"]
        Etiquette["<b>Etiquette, High Society & Corporate Protocol</b><br/>(KNO/SOC | PRE/MEM | Tier 2)<br/>Req: Charm ≥ 2 or Persuasion ≥ 2"]
        Streetwise["<b>Streetwise & Gang Protocol</b><br/>(KNO/SOC | INS/PRE | Tier 2)<br/>Req: Intimidation ≥ 1 or Deception ≥ 1<br/>[Access: Illicit/Restricted]"]
        ConArt["<b>Con Artistry, Social Engineering & Impersonation</b><br/>(KNO/SOC | ELO/CRE ≥ 10 | Tier 2)<br/>Req: Deception ≥ 2, Charm ≥ 2<br/>[Access: Illicit/Restricted]"]
        Negotiate["<b>Diplomacy, Mediation & High-Stakes Negotiation</b><br/>(KNO/SOC | ELO/INS ≥ 10 | Tier 2)<br/>Req: Persuasion ≥ 2, Empathy ≥ 2"]
        Oratory["<b>Public Speaking, Oratory & Media Spin</b><br/>(KNO/SOC | ELO/PRE ≥ 10 | Tier 2)<br/>Req: Persuasion ≥ 2, Writing ≥ 2"]
        Command["<b>Leadership, Command & Small-Unit Tactics</b><br/>(KNO/SOC | PRE/DIS ≥ 10, INS ≥ 10 | Tier 3)<br/>Req: Persuasion ≥ 2 or Intimidation ≥ 2<br/>[Access: Restricted/Illicit/Licensed]"]
    end

    subgraph InterrogationBranch["Elicitation, Interrogation & PsyOps Branch"]
        Elicit["<b>Investigative Interviewing & Subtle Elicitation</b><br/>(KNO/SOC | INS/ELO ≥ 11 | Tier 3)<br/>Req: Empathy ≥ 2, Charm or Persuasion ≥ 2<br/>[Access: Restricted/Licensed]"]
        Interrog["<b>Coercive Interrogation & Psychological Breaking</b><br/>(KNO/SOC | DIS/INS ≥ 11 | Tier 3)<br/>Req: Intimidation ≥ 2, Empathy ≥ 2<br/><b>[Access: Restricted OR Illicit ONLY]</b>"]
        SERE["<b>Counter-Interrogation & Resistance (SERE)</b><br/>(KNO/SOC | DIS/FOC ≥ 12 | Tier 3)<br/>Req: Deception ≥ 2<br/><b>[Access: Restricted OR Illicit ONLY]</b>"]
        PsyOps["<b>Psychological Operations (PsyOps) & Disinformation</b><br/>(KNO/SOC | CRE/INS ≥ 12 | Tier 4)<br/>Req: Oratory ≥ 2, AI Synth-Media ≥ 2<br/>[Access: Restricted/Illicit]"]
    end

    Charm --> Seduct
    Charm --> Etiquette
    Charm --> ConArt
    Charm --> Elicit
    Persuade --> Etiquette
    Persuade --> Negotiate
    Persuade --> Oratory
    Persuade --> Command
    Deceive --> Seduct
    Deceive --> Streetwise
    Deceive --> ConArt
    Deceive --> SERE
    Intim --> Streetwise
    Intim --> Interrog
    Intim --> Command
    Empathy --> Negotiate
    Empathy --> Elicit
    Empathy --> Interrog
    Oratory --> PsyOps
```

| Skill Name | Type | Categories | Control ($C$) | Min $C$ | Prerequisites | Access | Tools |
| :--- | :---: | :--- | :---: | :---: | :--- | :--- | :---: |
| **Charm & Rapport** | `TAL` | `Social` | `PRE/ELO` | — | None *(Starts at `N = 1`)* | `Universal` | `None` |
| **Persuasion & Rhetoric** | `TAL` | `Social` | `ELO/ANA` | — | None *(Starts at `N = 1`)* | `Universal` | `None` |
| **Deception & Fast-Talk** | `TAL` | `Social`, `Fieldcraft` | `ELO/CRE` | — | None *(Starts at `N = 1`)* | `Universal` | `None` |
| **Intimidation & Coercion** | `TAL` | `Social`, `Combat` | `PRE/DIS` | — | None *(Starts at `N = 1`)* | `Universal` | `Situational` |
| **Empathy & Read Person** | `TAL` | `Social` | `INS/ITU` | — | None *(Starts at `N = 1`)* | `Universal` | `None` |
| **Seduction & Honey-Trap Tradecraft** | `PHY/KNO` | `Social`, `Fieldcraft` | `COM/PRE` | `COM 10` *(or `PRE 10`)* | *Charm & Rapport* $\ge 2$, *Deception* $\ge 2$ or *Empathy* $\ge 2$ | `Universal` / `Restricted` / `Illicit` | `Situational` |
| **Etiquette, High Society & Corporate Protocol** | `KNO` | `Social`, `Knowledge` | `PRE/MEM` | — | *Charm & Rapport* $\ge 2$ or *Persuasion* $\ge 2$ | `Universal` / `Academic` / `Licensed` | `Situational` *(Wardrobe)* |
| **Streetwise & Gang Protocol** | `KNO` | `Social`, `Knowledge`, `Fieldcraft` | `INS/PRE` | — | *Deception* $\ge 1$ or *Intimidation* $\ge 1$ | `Illicit` / `Restricted` *(Vice/Gang Unit)* | `None` |
| **Con Artistry, Social Engineering & Impersonation** | `KNO` | `Social`, `Fieldcraft`, `Cyber` | `ELO/CRE` | `ELO 10, CRE 10, DIS 10` | *Deception* $\ge 2$, *Charm & Rapport* $\ge 2$ | `Illicit` / `Restricted` | `Situational` *(Props/Spoofer)* |
| **Diplomacy, Mediation & High-Stakes Negotiation** | `KNO` | `Social`, `Knowledge` | `ELO/INS` | `ELO 10, INS 10` | *Persuasion* $\ge 2$, *Empathy* $\ge 2$ | `Academic` / `Licensed` / `Restricted` | `None` |
| **Public Speaking, Oratory & Media Spin** | `KNO` | `Social`, `Artistic`, `Knowledge` | `ELO/PRE` | `ELO 10, PRE 10` | *Persuasion* $\ge 2$, *Writing* $\ge 2$ | `Universal` / `Academic` | `Situational` |
| **Leadership, Command & Small-Unit Tactics** | `KNO` | `Social`, `Combat`, `Knowledge` | `PRE/DIS` | `PRE 10, DIS 10, INS 10` | *Persuasion* $\ge 2$ or *Intimidation* $\ge 2$, *Observation* $\ge 2$ | `Restricted` / `Illicit` *(Shot-Caller)* / `Licensed` | `Situational` *(Comms)* |
| **Investigative Interviewing & Subtle Elicitation** | `KNO` | `Social`, `Fieldcraft`, `Knowledge` | `INS/ELO` | `INS 11, ELO 10` | *Empathy* $\ge 2$, *Charm* $\ge 2$ or *Persuasion* $\ge 2$ | `Restricted` / `Licensed` *(Detective/Reporter/Spy)* | `Situational` |
| **Coercive Interrogation & Psychological Breaking** | `KNO` | `Social`, `Fieldcraft` | `DIS/INS` | `DIS 11, INS 11, PRE 10` | *Intimidation* $\ge 2$, *Empathy* $\ge 2$ | **`Restricted` or `Illicit` ONLY** | `Situational` *(Biometric/Polygraph `-2`)* |
| **Counter-Interrogation & Resistance (SERE)** | `KNO` | `Social`, `Fieldcraft` | `DIS/FOC` | `DIS 12, FOC 11` | *Deception* $\ge 2$ | **`Restricted` or `Illicit` ONLY** | `None` |
| **Psychological Operations (PsyOps) & Disinformation** | `KNO` | `Social`, `Cyber`, `Knowledge` | `CRE/INS` | `CRE 11, INS 12, ELO 10` | *Public Speaking* $\ge 2$, *AI 2.0 Synth-Media* $\ge 2$ | `Restricted` / `Illicit` | `Mandatory` *(Media/Botnet)* |

> [!NOTE]
> **Elicitation, Coercive Interrogation & Social Combat (Psyche Resonance & Cognitive Dissonance)**:
> - **Investigative Interviewing & Subtle Elicitation** (`INS/ELO`) uses *Charm*, *Empathy*, pattern insight, and conversational framing to draw out secrets from a subject who does not even realize they are being interrogated (or via rapport-based detective questioning).
> - **Coercive Interrogation & Psychological Breaking** (`DIS/INS`) is used when a captive knows they are being interrogated and is actively resisting. It pits the interrogator's *Coercive Interrogation* (`DIS/INS`) against the subject's *Counter-Interrogation & Resistance* (`DIS/FOC`, or raw `DIS` at `N = 1` if untrained).
> - **Targeting Psyche in Social Combat**: As detailed in **[Psyche, Morale & Social Combat](../04-psyche-and-social/psyche_morale_and_social_combat.md)**, social skills interact directly with the target's seven **Psyche** traits (`EGO`, `SLF`, `OPN`, `BLG`, `EXT`, `CNF`, `AGG`).

---

### 3.9 Tree 8: Literacy, Law, Intelligence, Tradecraft & Artistic (`Knowledge`, `Fieldcraft`, `Artistic`)

This tree highlights the interplay of **Memory (`MEM`)** and **Eloquence (`ELO`)** in language and law, **Insight (`INS`)** and **Focus (`FOC`)** in intelligence and OSINT, **Tactile (`TAC`)** and **Dexterity (`DEX`)** in lockpicking and sleight of hand, and **Hearing (`HEA`)**, **Sight (`SIG`)**, **Taste (`TAS`)**, **Olfactory (`OLF`)**, and **Creativity (`CRE`)** across the arts.

```mermaid
flowchart TD
    subgraph LiteracyHumanities["Literacy, Law & Intelligence Branch"]
        Speech["<b>Native Language Speech</b> (TAL | ELO/MEM | Starts N=2)"]
        Read["<b>Reading</b> (KNO | MEM/FOC | Tier 1)"]
        Write["<b>Writing & Composition</b> (KNO | ELO/CRE | Tier 1)"]
        ForeignLang["<b>Foreign Language (Specific)</b> (KNO | MEM/ELO)"]
        Civics["<b>History, Civics & Geopolitics</b> (KNO | MEM/INS | Tier 2)"]
        Research["<b>Research, OSINT & Archival Analysis</b> (KNO | INS/FOC | Tier 2)"]
        LawCiv["<b>Civil, Corporate & Constitutional Law</b> (KNO | MEM/ANA ≥ 11 | Tier 3)"]
        LawCrim["<b>Criminal Law & Procedure</b> (KNO | MEM/ANA ≥ 10 | Tier 3)"]
        IntelAnal["<b>Strategic Intelligence & Threat Analysis</b> (KNO | INS/ANA ≥ 11 | Tier 3)<br/>[Access: Restricted]"]
    end

    subgraph TradecraftUnderworld["Physical Tradecraft & Underworld Branch"]
        Observe["<b>Observation & Awareness</b> (TAL | SIG/HEA | Starts N=1)"]
        Stealth["<b>Stealth & Sneaking</b> (TAL | KIN/FOC | Starts N=1)"]
        Sleight["<b>Sleight of Hand, Palming & Pickpocketing</b> (PHY | DEX/TAC ≥ 11 | Tier 2)<br/>[Access: Illicit/Artistic]"]
        Lockpick["<b>Lockpicking & Physical Security Bypass</b> (PHY/KNO | DEX/TAC ≥ 10 | Tier 2)<br/>[Access: Illicit/Restricted/Licensed]"]
        Hotwire["<b>Vehicle Hotwiring & Transponder Spoofing</b> (PHY/KNO | DEX/ANA | Tier 2)<br/>[Access: Illicit]"]
        Forgery["<b>Forgery, Counterfeiting & Identity Washing</b> (KNO/PHY | DEX/CRE ≥ 10 | Tier 3)<br/>[Access: Illicit/Restricted]"]
        Surveil["<b>Surveillance, Tail Craft & Counter-Surveillance</b> (PHY/KNO | FOC/SIG ≥ 10 | Tier 2)<br/>[Access: Restricted/Illicit]"]
        Fence["<b>Black Market Logistics, Smuggling & Fencing</b> (KNO/SOC | INS/ELO | Tier 2)<br/>[Access: Illicit]"]
    end

    subgraph ArtisticBranch["Artistic & Performance Branch"]
        Sing["<b>Singing & Vocal Control</b> (TAL | HEA/ELO | Starts N=1)"]
        Draw["<b>Drawing & Visual Sketching</b> (TAL | DEX/SIG | Starts N=1)"]
        VocalPerf["<b>Professional Vocal Performance & Mimicry</b> (PHY/ART | HEA/PRE)"]
        Instrument["<b>Musical Instrument (Specific)</b> (PHY/ART | DEX/HEA)"]
        VisualArt["<b>Graphic Design, Sculpture & 3D Art</b> (PHY/ART | CRE/DEX)"]
        Culinary["<b>Culinary Arts & Mixology</b> (PHY/KNO | TAS/OLF)"]
    end

    Speech --> Read
    Read --> Write
    Read --> Civics
    Write --> Civics
    Read --> Research
    Civics --> LawCiv
    Civics --> LawCrim
    Research --> IntelAnal
    Civics --> IntelAnal

    Observe --> Surveil
    Stealth --> Surveil
    Stealth --> Sleight
    Write --> Forgery

    Sing --> VocalPerf
    Draw --> VisualArt
```

| Skill Name | Type | Categories | Control ($C$) | Min $C$ | Prerequisites | Access | Tools |
| :--- | :---: | :--- | :---: | :---: | :--- | :--- | :---: |
| **Native Language Speech** | `TAL` | `Knowledge`, `Social` | `ELO/MEM` | — | None *(Starts at `N = 2` for native speakers)* | `Universal` | `None` |
| **Reading** | `KNO` | `Knowledge` | `MEM/FOC` | — | *Native Language Speech* $\ge 1$ | `Universal` | `None` |
| **Writing & Composition** | `KNO` | `Knowledge`, `Artistic` | `ELO/CRE` | — | *Reading* $\ge 1$ | `Universal` | `Mandatory` |
| **Foreign Language (Specific)** | `KNO` | `Knowledge`, `Social` | `MEM/ELO` | — | None *(Oral)*; *Reading* $\ge 1$ *(Literate)* | `Universal` | `None` |
| **History, Civics & Geopolitics** | `KNO` | `Knowledge` | `MEM/INS` | `MEM 9` | *Reading* $\ge 2$, *Writing* $\ge 2$ | `Universal` / `Academic` | `Situational` |
| **Research, OSINT & Archival Analysis** | `KNO` | `Knowledge`, `Cyber`, `Fieldcraft` | `INS/FOC` | `ANA 10, FOC 10` | *Reading* $\ge 2$, *Digital Literacy* $\ge 2$ | `Universal` / `Academic` | `Mandatory` |
| **Criminal Law & Procedure** | `KNO` | `Knowledge` | `MEM/ANA` | `MEM 10, ANA 10` | *History & Civics* $\ge 2$ *(or Streetwise $\ge 3$ for `N=1`)* | `Academic` / `Restricted` / `Illicit` | `Situational` |
| **Civil, Corporate & Constitutional Law** | `KNO` | `Knowledge` | `MEM/ANA` | `MEM 11, ANA 11, ELO 10` | *History & Civics* $\ge 3$, *Writing* $\ge 3$ | `Academic` / `Licensed` *(Law School)* | `Situational` |
| **Strategic Intelligence & Threat Analysis** | `KNO` | `Knowledge`, `Fieldcraft` | `INS/ANA` | `INS 11, ANA 11` | *Research & OSINT* $\ge 2$, *History & Civics* $\ge 2$ | `Restricted` | `Mandatory` |
| **Observation & Awareness** | `TAL` | `Fieldcraft` | `SIG/HEA` | — | None *(Starts at `N = 1`; swaps to `ITU`/`FOC` by context)* | `Universal` | `Situational` |
| **Stealth & Sneaking** | `TAL` | `Fieldcraft`, `Athletic` | `KIN/FOC` | — | None *(Starts at `N = 1`)* | `Universal` | `Situational` |
| **Sleight of Hand, Palming & Pickpocketing** | `PHY` | `Fieldcraft`, `Artistic` | `DEX/TAC` | `DEX 11, TAC 10` | *Stealth* $\ge 1$ | `Illicit` / `Artistic` | `None` |
| **Lockpicking & Physical Security Bypass** | `PHY/KNO` | `Fieldcraft`, `Technical` | `DEX/TAC` | `DEX 10, TAC 10, FOC 9` | *Basic Tool Use* $\ge 1$ or *Sleight of Hand* $\ge 1$ | `Illicit` / `Restricted` / `Licensed` | `Mandatory` |
| **Vehicle Hotwiring & Transponder Spoofing** | `PHY/KNO` | `Fieldcraft`, `Technical`, `Cyber` | `DEX/ANA` | `DEX 9` | *Streetwise* $\ge 1$ or *Basic Electronics* $\ge 1$ | `Illicit` / `Restricted` | `Mandatory` |
| **Forgery, Counterfeiting & Identity Washing** | `KNO/PHY` | `Fieldcraft`, `Cyber`, `Artistic` | `DEX/CRE` | `DEX 10, SIG 10, FOC 10` | *Writing* $\ge 2$, *Digital Literacy* $\ge 2$ | `Illicit` / `Restricted` | `Mandatory` |
| **Surveillance, Tail Craft & Counter-Surveillance** | `PHY/KNO` | `Fieldcraft` | `FOC/SIG` | `FOC 10, SIG 10, ITU 10` | *Observation* $\ge 2$, *Stealth* $\ge 2$ | `Restricted` / `Illicit` | `Situational` |
| **Black Market Logistics, Smuggling & Fencing** | `KNO` | `Social`, `Knowledge`, `Fieldcraft` | `INS/ELO` | `INS 9` | *Streetwise* $\ge 2$, *Arithmetic* $\ge 2$ | `Illicit` | `Situational` |
| **Singing & Vocal Control** | `TAL` | `Artistic`, `Social` | `HEA/ELO` | — | None *(Starts at `N = 1`; assisted by `FIT`)* | `Universal` | `None` |
| **Drawing & Visual Sketching** | `TAL` | `Artistic` | `DEX/SIG` | — | None *(Starts at `N = 1`; paired with `CRE`)* | `Universal` | `Mandatory` |
| **Professional Vocal Performance & Mimicry** | `PHY` | `Artistic`, `Fieldcraft` | `HEA/PRE` | `HEA 10, PRE 10` | *Singing* $\ge 2$ | `Universal` / `Artistic` | `Situational` |
| **Musical Instrument (Specific)** | `PHY` | `Artistic` | `DEX/HEA` | `DEX 9, HEA 9` | None *(paired with `CRE/DIS`)* | `Universal` / `Artistic` | `Mandatory` |
| **Graphic Design, Sculpture & 3D Synth-Art** | `PHY/KNO` | `Artistic`, `Cyber` | `CRE/DEX` | `CRE 9, DEX 9, SIG 9` | *Drawing* $\ge 2$ | `Universal` / `Artistic` | `Mandatory` |
| **Culinary Arts & Mixology** | `PHY/KNO` | `Artistic`, `Technical` | `TAS/OLF` | — | None *(paired with `CRE/DEX`)* | `Universal` / `Vocational` | `Mandatory` |
