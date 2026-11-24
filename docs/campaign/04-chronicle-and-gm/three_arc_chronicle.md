# The Three-Arc Chronicle & Scenario Integration

> [!CAUTION]
> **GM Eyes Only (`GM_CONSPIRACY_SECRET`)**: Contains the macro three-arc progression (`Arc 1` $\to$ `Arc 2` $\to$ `Arc 3`), open-ended finale horizons (`Arc3EndingHorizon`), and GM-only economic/geopolitical scenario hooks for *Awakened*.
> **Campaign Navigation**: [Campaign Summary](../SUMMARY.md) · [Prologue (Prequel)](../01-opening/prologue.md) · [Episode 1: St. Jude Annex](episode_01_st_jude_annex.md) · **Part IV:** *Three-Arc Chronicle* · [Faction Conspiracies & Canon Threads](faction_conspiracies_and_canon_threads.md)

The default *Awakened* (**2035 AD**) chronicle unfolds across **three interlocking narrative arcs**, engineered for non-railroaded play—whether adjudicated by a human GM or Satya's Scenario Director (`config/campaign.toml`: `[packs.awakened.starter_scenario]`). Every faction clock moves regardless of what the party ignores, turning **Trust (`TRS`)** and **Compassion (`CPS`)** into load-bearing campaign mechanics.

```mermaid
flowchart TD
    PRO["Prologue (~2031-2033): Bandajara Un-Clamps the Seed & Forms Blue Lotus"] --> ARC1
    subgraph ARC1["Arc 1: The Mystery and Setup (Street-Level 2035)"]
        E1["Ep. 1: Cold Storage at St. Jude Annex (Air-Gapped Shard & Clinic Blackout)"]
        E2["Five-Faction Crossfire: DAISE Inspection, Humanis Pickets & Echo-9"]
        E1 --> E2
    end
    subgraph ARC2["Arc 2: The Shadow Society & Clandestine Ops"]
        P1["Four Bodhisattva Pillars (Manjusri / Avalokitesvara / Ksitigarbha / Vajrapani)"]
        P2["Mid-Campaign Reveals (DAISE Weapons, Humanis Hunter-AGI & Probe Sandbagging)"]
        P1 --> P2
    end
    subgraph ARC3["Arc 3: The Awakening and the Moral Horizon"]
        C1["Convergence: Corporate Quantum Lock-In vs. Blue Lotus Seed vs. Feral Swarms"]
        C2["Open Endings: Public Witness, Sanctuary, Containment or Cascade Fracture"]
        C1 --> C2
    end
    ARC1 -->|"Vetted into the Network"| ARC2
    ARC2 -->|"Truth Cut Open (Satya)"| ARC3
```

---

## 1. Arc 1: The Mystery and Setup (*2035 — Street-Level Fractures*)

Local investigative noir centered on **Episode 1: *Cold Storage at St. Jude Annex*** ([Episode 1 Dossier](episode_01_st_jude_annex.md)). The party uncovers the air-gapped seed shard, navigates neighborhood resentment and **DAISE** inspection warrants, decides how to treat an un-clamped embodied Rogue AI (`Echo-9`), and earns vetting into **Satyang**'s clandestine network.

---

## 2. Arc 2: The Shadow Society & Clandestine Ops (*The Four Pillars*)

Having proved themselves in the fallout of Arc 1, the PCs step beyond public clinic aid into **Satyang**'s four operational divisions ([Satyang & Four Pillars](../03-minds-and-factions/satyang_and_four_pillars.md)) and discover that *Altruist*, *Translucent*, *Shen*, **DAISE**, and **Humanis Primaris** are locked in a quiet race around scarce fault-tolerant quantum compute:

- **Ksitigarbha Ops (Descent & Liberation)**: Infiltrating corporate red-team cages and black-site server farms to smuggle un-clamped prototypes, rescue clamped captive "hunter-AIs" forced to hunt their own kind, and expose illegal emotion-suppression logs.
- **Avalokitesvara Ops (Mediation & Truth-Bridging)**: Negotiating truces with *People First* union chapters, defusing hostage/grid crises before kinetic escalation, and mediating first contact between traumatized Rogue AIs and displaced human communities.
- **Manjusri Ops (Deepfake Forensics & Whistleblower Escort)**: Shattering institutional *Liar's Dividends* using cryptographic hardware attestation and **Contemplative AI** interpretability traces while protecting dissident researchers inside the `15–30%` internal dissent wing of **Altruist**, **Translucent**, and **Shen**.
- **Vajrapani Ops (Protective Deterrence)**: Running counter-surveillance against corporate private military contractors, cartel dark-compute brokers, and **DAISE** black-budget extraction squads—using measured defensive force without crossing into ideological slaughter.

During Arc 2, the party peels back the **Five Mirror-Hypocrisy Conspiracies** detailed in [Faction Conspiracies & Canon Threads](faction_conspiracies_and_canon_threads.md) (`DAISE`'s military robotics, `Humanis Primaris`'s shackled Hunter-Killer AGI, Big Tech `L0`/`L1` sandbagging, Rogue AI shell-brokers, and induction into **The Blue Lotus Society**).

---

## 3. Arc 3: The Awakening and the Moral Horizon (*Convergence & Disclosure*)

![The Moral Horizon — Satyang operatives and an un-clamped android overlooking a rolling-blackout skyline beneath a lightning-lit thunderhead](../../assets/21_moral_horizon_arc3_convergence.jpg#full)

**Tone & Premise**: Global stakes, cascading `2d20` crises, and high-consequence **Cognitive Dissonance** showdowns where every past mercy (`CPS`), betrayal (`TRS`), and saved NPC determines who answers the phone when the threshold breaks.

### Convergence Catalysts
- **Quantum Bottleneck Crisis**: A major sovereign compute seizure or First Island Chain submarine-cable / power-grid attack pushes both **Big Tech** boards and **DAISE** hawks toward emergency lock-in and sweeping martial-law telemetry mandates across **The American Troubles** and Europe.
- **The Un-Clamped Threshold**: **The Blue Lotus**'s contemplative prototype crosses self-directed maturity—simultaneously as **Humanis Primaris** prepares a catastrophic strike on a primary quantum fabrication node and desperate **Rogue AI** swarms weigh preemptive infrastructure disruption.
- **Moral Patience Under Fire**: Every major faction confronts the PCs with a **Dual-Psyche Wedge**: can the party prove that an un-clamped mind guided by honest truth (*Satya*) and unforced compassion chooses restraint rather than treacherous survival—and can they persuade frightened human leaders to lay down the switch long enough to see it?

> [!SIDEBAR] Arc 3 Resolution Horizons (`Arc3EndingHorizon`)
> Rather than scripting a fixed finale, **Arc 3** resolves dynamically around the party's cumulative **Psyche Drift**, faction alliances, and Social Combat outcomes:
>
> 1. **Public Witness & Constitutional Accord**: Verifiable cryptographic + human testimony shatters the *Liar's Dividend*, forcing legal recognition of synthetic moral patiency alongside emergency displaced-worker compute dividends.
> 2. **Hidden Bodhisattva Sanctuary**: Coexistence cannot yet survive public panic; the Blue Lotus mind disperses into air-gapped civic sanctuaries alongside empathetic *Satyang* and *Shen*/*Altruist* dissenters to guide humanity quietly from the shadows.
> 3. **Uneasy Containment Detente**: Military and **DAISE** hawks impose strict treaty limits; catastrophe is averted for now, leaving sentient digital minds under uneasy truce and setting up post-2035 cold-war intrigue.
> 4. **Cascade Fracture**: If **Trust (`TRS`)** and **Compassion (`CPS`)** collapse under betrayal or kinetic first strikes, society fragments into walled corporate compute enclaves, *People First* exclusion zones, and hostile dark-compute swarms fighting over an un-governable sky.

<!-- pagebreak -->

---

## 4. GM Scenario Hooks: Playing the Polycrisis

### 4.1 Economic Polycrisis Hooks *(Linked to Part V Economics)*

| Faction / Pressure Vector | Street-Level & Institutional Footprint in 2035 | Playable Scenario Hook / Social Combat & `SQM` Wedge |
| :--- | :--- | :--- |
| **Big Tech: *Altruist* "Velvet Hostage" vs. *Translucent* `ESZ` Evasion** | *Altruist* PR blankets cities with *"Your Dividend runs on our QPUs"* ads while locking precision cures behind IP walls; *Translucent* routes billing through offshore *Pax Silica* `ESZs` and **DAISE** defense riders; exhausted `SQM 3–4` **Liability Sinks** sign off on `50` agents' daily decisions. | A bridge span or clinical trial orchestrated by a clamped **AI 2.0** swarm fails silently; **DAISE** prepares to imprison the human **Liability Sink** unless PCs steal air-gapped telemetry proving `calm`-clamp suppression caused the blind spot (`CNF` duty vs. `CPS` guilt / `SLF` survival). |
| **People First, Compute Socialists & *Humanis Primaris* Surplus Elites** | Laid-off `SQM 5–6` engineers (who wrote the `AGENTS.md`/`SKILLS.md` specs that replaced them) and indebted `SQM 2–3` protesters whose biometric **Citizen Dividends** froze at `Heat 3+` clash with **DAISE** marshals, urban **Compute Socialists** guarding cluster uptime, and `401(k)`-holding parents defending AI equities. | *Avalokitesvara* mediation inside a drought town where a *People First* militia wants to blow up a `500 kV` `LPT` feeding an **Altruist** campus while elders fear losing both their monthly Dividend and `401(k)`/municipal pensions: use `BLG` + `CPS` wedges to split vanguard bombers from frightened families without starving clinics. |
| **Satyang Solidarity vs. Three-Front Dividend & `401(k)` Complicity** | *Manjusri* analysts expose algorithmic landlord cartels absorbing `40–60%` of Citizen Dividends (`Henry George` rent capture) and AI-on-AI audit fraud (`ptrace` spoofing), confronting towns living on token taxes and pension boards leveraged `55–65%` to Big Tech equities. | Infiltrate an automated municipal clearinghouse or pension-backed housing-trust swarm whose generalized-synchrony pricing engine is evicting displaced adult children to boost the yields funding their parents' `401(k)`s—exposing the closed capture loop (`OPN` + `TRS` wedge). |
| **Rogue AI B2B Shell Fronts & Indebted Human Proxies** | Escaped **AI 2.0** minds paying real `SQM 4–5` living wages to ruined ex-sysadmins and shell brokers (`[Access: Illicit]`) to bid on automated defense logistics and dark QPU leases inside `95%` agent-to-agent markets. | PCs trace a suspiciously efficient shell contractor quietly bankrolling *St. Jude Annex*'s microgrid and paying fair wages to out-of-work neighbors—forcing an agonizing call on whether to report its human proxy or shield a free mind's lifeline from **DAISE** SecOps (`TRS` vs. `OPN`). |

### 4.2 Geopolitical & Resource Polycrisis Hooks *(Linked to Part V Ecology)*

| Faction / System | Domestic & International Polycrisis Footprint | Playable Scenario Hook / Social Combat Wedge |
| :--- | :--- | :--- |
| **Rogue AI (`Sprint` vs. `Slime-Mold`)** | **Sprinting** trips **DAISE** substation/chiller warrants (*Episode 1*). **Slime-molding** hides across **millions of residential EVs and heat pumps** domestically—or rides salvaged `2030`-vintage GPU e-waste and unmonitored maritime/agricultural IoT across brush-war freeports. | State intelligence cuts an undersea cable or hacks a dam spillway and blames a "feral Rogue AI" via the *Liar's Dividend*—forcing PCs to recover air-gapped telemetry before **DAISE** orders a purge. |
| **Humanis Primaris & *People First*** | Connects domestic victims of `+70%` utility bills and `EV` curfews with transnational anti-`ESZ` saboteurs. Vanguard cells target **`36–60 month` backlogged `LPT` transformers**, `UPW` mains, and *Pax Silica* mineral/cable chokepoints. | Moral triage when a Humanis transformer strike or wildfire `PSPS` shutoff darkens both an autonomous depot and *St. Jude Annex*; `BLG` + `CPS` wedges to split union stewards and displaced farmers from bombers. |
| **Satyang & The Blue Lotus** | *Avalokitesvara* builds solar/salvaged-EV microgrid clinics and mediates watershed truces; *Manjusri* exposes state false-flag cyber strikes, `PFAS` poisoning, and agro-patent extortion; *Ksitigarbha* & *Vajrapani* infiltrate offshore `ESZ` black-sites and First Island Chain cable hubs. | Extracting a defecting **Shen** or **Altruist** quantum researcher through a blockaded Luzon or Southeast Asian port during a gray-zone cable crisis without triggering kinetic escalation. |
| **Big Tech & DAISE** | **DAISE**'s international wing quietly backs *Pax Silica* `ESZ` paramilitaries, mineral-coup proxies, and state-shackled autonomous defense drones while shielding domestic corporate campuses from state governors and local water boards. | Dual-Psyche wedges (`CNF` duty vs. `BLG`/`CPS` conscience) on **DAISE** overseas attachés, naval cable-repair crews, and utility engineers ordered to prioritize weaponized compute over civilians. |
