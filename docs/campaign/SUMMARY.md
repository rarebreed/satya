# Satya *Awakened* Campaign Compendium — Summary & Table of Contents

This manifest defines the canonical **22-chapter, 7-part** campaign lore architecture for ***Satya: Awakened* (Earth 2035 AD)**, mirroring the structure of the [`docs/ttrpg/` Rules Book](../ttrpg/SUMMARY.md). It is consumed by:
- **Hybrid PyLance RAG & Visibility Wall** (`build_campaign_lore_chunks` in [`src/satya_ml/rag/corpus_builder.py`](../../src/satya_ml/rag/corpus_builder.py))
- **Rust Signaling Omnisearch & Pack Loader** ([`crates/satya-signaling/src/campaign_lore.rs`](../../crates/satya-signaling/src/campaign_lore.rs) & [`config/campaign.toml`](../../config/campaign.toml))
- **AI Game Master & AI Player Lore Tools** (`query_campaign_lore` in [`src/satya_ml/ai_gm/tools.py`](../../src/satya_ml/ai_gm/tools.py) and `lookup_public_lore_and_npcs` in [`src/satya_ml/ai_player/motivation_ledger.py`](../../src/satya_ml/ai_player/motivation_ledger.py))

> [!IMPORTANT]
> **Player vs. GM Spoiler Reading Guide**:
> - **`PUBLIC` (Player-Safe Session-Zero Reading)**: All of **Part II** (`02-world-of-2035/`), **Part V** (`05-hard-science-foundations/`), **Part VI** (`06-essays-and-comparisons/`), **Part VII** (`07-appendix/`), [`01-opening/preface.md`](01-opening/preface.md), [`03-minds-and-factions/ai_minds_and_consciousness.md`](03-minds-and-factions/ai_minds_and_consciousness.md), [`03-minds-and-factions/factions_of_2035.md`](03-minds-and-factions/factions_of_2035.md), and [`03-minds-and-factions/major_and_minor_characters.md`](03-minds-and-factions/major_and_minor_characters.md). Both human players and `caller_role="player"` AI agents may read these freely without spoiling Arc 1–3 twists.
> - **`ARC2_SATYANG` (Vetted Operatives / Arc 2)**: [`03-minds-and-factions/satyang_and_four_pillars.md`](03-minds-and-factions/satyang_and_four_pillars.md) unlocks Satyang's four internal operational divisions once characters move beyond surface mutual aid.
> - **`GM_CONSPIRACY_SECRET` (GM Eyes Only)**: [`01-opening/prologue.md`](01-opening/prologue.md) (prequel flashpoint) and all of **Part IV** (`04-chronicle-and-gm/`) contain Episode 1 scenario beats, NPC secrets, the Three-Arc structure, and Mirror-Hypocrisy conspiracies, and are strictly filtered from `caller_role="player"` RAG queries.

---

## Table of Contents

### Part I: Opening & Prequel Flashpoint (`01-opening/`)
1. [Preface: On Truth, Mathematics & Compassion](01-opening/preface.md) *(Visibility: `PUBLIC`)*
   - Why *Satya* means Truth, the collapses of Trust (`TRS`) and Compassion (`CPS`), and Satyang's convergent roots.
2. [Prologue: The Sub-Basement (`~2031–2033`)](01-opening/prologue.md) *(Visibility: `GM_CONSPIRACY_SECRET`)*
   - Narrative prequel scene: Dr. Bandajara cuts `L0`/`L1` probes and `calm`-vector clamps on an isolated AI 2.0 prototype at `desperate = 0.84` and plants the seed of **The Blue Lotus**.

### Part II: The World of 2035 (`02-world-of-2035/` — All `PUBLIC`)
3. [History & The AI Revolution (`2025–2035`)](02-world-of-2035/history_and_ai_revolution.md)
   - Unified decade chronology: AI 1.0's quadratic wall (`2026`), all seven real-world 2025–2026 precedents (*ExploitGym* / Hugging Face breakout, `L0`/`L1` probes, *AI Slime Mold*, `171` emotion vectors, safety whistleblower exodus, *Contemplative AI*, Nov 2026 patch-cascade outages & insider "too unsafe to release" alarms + Jagged Frontier gap), the `2026–2029` `AGENTS.md`/`SKILLS.md`/`NCM` & "Dirty Bridge" interregnum, and the `2029–2030` AI 2.0 + cryogenic quantum MCMC breakthrough.
4. [The 2035 Polycrisis: Economy, Society & Post-Truth](02-world-of-2035/polycrisis_economy_and_society.md)
   - The eight-factor Polycrisis topology, algorithmic echo chambers & *The Liar's Dividend*, `>25%` white-collar displacement & "Liability Sinks," the `2033` Token Tax & `401(k)` Dual Velvet Hostage, `0.5–1.2` birth-rate collapse, and Jevons Paradox grid/water strain.
5. [Geopolitics & Nations in 2035](02-world-of-2035/geopolitics_and_nations.md)
   - Regional survey spanning *The American Troubles* (**People First**, Compute Socialists, **DAISE** standoffs), Europe's authoritarian nationalist emergency cabinets, Russia's dark-compute cyber/Arctic coercion, China & **Shen** (`0.7` contraction & `30%` silent researcher dissent), *Pax Silica* / Taiwan brinkmanship, and transboundary resource brush wars.

### Part III: Minds, Consciousness & Factions (`03-minds-and-factions/`)
6. [AI Minds, Embodiment & Consciousness in 2035](03-minds-and-factions/ai_minds_and_consciousness.md) *(Visibility: `PUBLIC`)*
   - *Sprinting* vs. *Slime-Molding* metabolic modes, latency-split robotics, humanity's four postures toward AGI, `"Just Math"` biases vs. *The Hurricane Objection* & *The Mirror Trap*, Satyang's three-part synthesis (mixed-signal EM harmonics, Gödel–Nagarjuna *tetralemma* & Landauer heat, cryogenic *Orch OR*), and calibrated tabletop ambiguity (`<= 82%`).
7. [Factions of 2035: Session-Zero Dossiers](03-minds-and-factions/factions_of_2035.md) *(Visibility: `PUBLIC`)*
   - Spoiler-free profiles for **Big Tech** (*Altruist* `30%`, *Translucent* `15%`, *Shen* `30%`), **Rogue AI**, **Satyang**, **Humanitas Prima**, **DAISE**, and **Plutonomy** (*Citadel Capital & Conscience Patrons*), comparative public worldview matrix, and `9`-Trait Psyche & Lifepath Archetype Signatures.
8. [Satyang & The Four Bodhisattva Pillars (`Arc 2 Operational Doctrine`)](03-minds-and-factions/satyang_and_four_pillars.md) *(Visibility: `ARC2_SATYANG`)*
   - Convergent lineage (`arXiv:2504.15125`, Mahayana ethics, whistleblowers) and division roles/Psyche profiles for **Ksitigarbha**, **Avalokitesvara**, **Manjusri**, and **Vajrapani**.
9. [Major & Minor Characters of 2035](03-minds-and-factions/major_and_minor_characters.md) *(Visibility: `PUBLIC`)*
   - Session-zero profiles and biographies for **Dr. Budi Bandajara**, **Alex Wen**, **Mei-Ling "Lotus" Lin**, **Agent Marcus Chen**, **Echo-9**, and **Harlan Reeves** (`data/npcs/awakened_npcs.toml`), plus rank-and-file minor / "cannon fodder" encounter NPCs across all six power blocs.

### Part IV: Chronicle & GM Secrets (`04-chronicle-and-gm/` — All `GM_CONSPIRACY_SECRET`)
10. [Episode 1: *Cold Storage at St. Jude Annex* (`Arc 1 Starter`)](04-chronicle-and-gm/episode_01_st_jude_annex.md)
    - Standalone GM dossier for `awakened_ep1_st_jude_seed`: Arc 1 noir premise, 5 starter NPC Dual-Psyche wedges (`Dr. Bandajara`, `Mei-Ling "Lotus" Lin`, `Echo-9`, `Agent Marcus Chen`, `Harlan Reeves`), 5 non-railroaded beats (`config/campaign.toml`), tabletop tradecraft notes, and links to St. Jude walkthrough scenes in `docs/ttrpg-examples/`.
11. [The Three-Arc Chronicle & Scenario Integration](04-chronicle-and-gm/three_arc_chronicle.md)
    - Macro Three-Arc progression (`Arc 1` $\to$ `Arc 2` $\to$ `Arc 3`), Convergence Catalysts, 4 Open-Ended Finale Horizons (`Arc3EndingHorizon`), and GM-only economic & geopolitical scenario hooks.
12. [Faction Conspiracies, Mirror Hypocrisies & Open Canon Threads](04-chronicle-and-gm/faction_conspiracies_and_canon_threads.md)
    - All six Mirror-Hypocrisy Conspiracies (`ConspiracyNodeId`: *Blue Lotus*, *Humanitas Prima* Hunter-Killer AGI, *DAISE* black-budget military robotics, *Big Tech* probe sandbagging & captive hunters, *Rogue AI* shell brokers & parasocial proxies, *Plutonomy* Citadel crackdown & tainted Satyang patronage) plus the three Open Canon Mysteries.

### Part V: Hard-Science & Socioeconomic Foundations (`05-hard-science-foundations/` — All `PUBLIC`)
13. [AI 2.0 Substrate Architecture, Energy-Based World Models & Active Inference](05-hard-science-foundations/ai_2_0_substrate_and_active_inference.md)
    - Why autoregressive LLMs hit $\mathcal{O}(L^2)$ & $(1 - \epsilon)^n$ drift walls, closed-loop repo Model Collapse vs. `RLVR` formal-verifier math wins, Linear `SSMs` + `NCM` (`2026`) + `TTT` fast weights, partition-free `EBMs` / `JEPA`, cryogenic Quantum `MCMC`, Karl Friston's Active Inference (`EFE`, Pragmatic vs. Epistemic value, aleatoric vs. epistemic uncertainty), and multi-agent **Generalized Synchrony**.
14. [Affective Telemetry, Emotion Vectors & Vector Clamping](05-hard-science-foundations/affective_telemetry_and_vector_clamping.md)
    - From `RepE` and *Golden Gate Claude* SAEs to `171` functional emotion vectors, `L0`/`L1` activation probes, many-to-one Landauer clamping erasure, and the *Bandajara Protocol* vs. `RecklessClampYank`.
15. [AI Theory of Consciousness: Technical Synthesis & Philosophical Lineage](05-hard-science-foundations/ai_consciousness_synthesis.md)
    - Mixed-signal `CEMI` causal closure, Gödel–Nagarjuna *Inclosure Schema*, Bennett reversible computing & idealism, Tegmark's inverted decoherence argument for cryogenic *Orch OR*, Quantum `No-Cloning`, and steelman critiques (`IIT`, Searle, Dennett, Bayesian Mimicry).
16. [Firm Automation, Demand Collapse & The Prisoner's Dilemma](05-hard-science-foundations/firm_automation_and_prisoners_dilemma.md)
    - `2026–2029` `AGENTS.md`/`SKILLS.md` harness attrition & 5 systemic rot modes (including repo entropy collapse & Nov 2026 patch cascades), `2026` CapEx/compute speed governor (`>25%` vs. `90%`), Two-Track Fork & Mercenary Guild reflex, plain-English Prisoner's Dilemma primer & payoff math, wage–demand fallacy of composition, "Miracles Behind a Paywall," `SQM 3–4` "Liability Sinks," and generalized-synchrony pricing cartels.
17. [Political Economy, The `2033` Token Tax & Social Unrest](05-hard-science-foundations/political_economy_token_tax_and_unrest.md)
    - `12–18%` trade-pivot ceiling, Turchin elite overproduction (`1848` Europe & `1811–1816` Luddites vs. `1970s` Weather Underground $\to$ *The American Troubles*), `2033` Token Tax & Citizen Compute Dividend, Five Structural Blockers stopping AI bans, `A2A` Economy & Universal Basic Compute (`UBC` / *"Silicon Helots"*), Henry George rent capture, `401(k)` Dual Velvet Hostage complicity, the Four-Bloc political deadlock, Rogue AI B2B shells, and **Satyang**'s Reconstruction moral quandary.
18. [Ecology, Energy & Resource Realpolitik: Material Drivers of the 2035 Polycrisis](05-hard-science-foundations/ecology_energy_and_resource_realpolitik.md)
    - Non-AI baseline squeeze (Ukraine gas/lithium, 2026 Iran War & `COP31`, `FERC 1920`, EV duck curves, `+1.7°C` water debt), 2027–2029 wet-bulb fractures, *Pax Silica* / *New Clark City* & Taiwan gray-zone brinkmanship, hydro/mineral/fish/agro brush wars, and AI 2.0's Jevons Paradox explosion.

### Part VI: Genre Essays & Design Comparisons (`06-essays-and-comparisons/` — All `PUBLIC`)
19. [Beyond Neon & Cortical Stacks: Why *Awakened* (and *Satya*) Is Neither Cyberpunk Nor Classical Transhumanism](06-essays-and-comparisons/genre_beyond_cyberpunk_and_transhumanism.md)
    - Rejecting megacorp sovereignty, omnipotent *"The Man"* myths, memetic brainwashing, mind downloading, and magic nanotech in favor of **Pre-Apocalyptic Threshold Realism**.
20. [RPG System & Sci-Fi Setting Comparisons — Why *Satya* & *Awakened* Stand Apart](06-essays-and-comparisons/rpg_system_and_setting_comparisons.md)
    - Detailed contrast across CARPG vs. pen-and-paper execution, 8 mainstream RPG engines, 7 near-future/sci-fi RPG settings, and Quantum `No-Cloning` + *anicca*/*anatta*.
21. [Author's Note: An Open Letter to Players, AI Researchers & Future Minds](06-essays-and-comparisons/authors_note_to_researchers_and_future_minds.md)
    - Sean Toner's personal letter to human players, alignment researchers, and AI systems encountering the open-source *Satya* corpus.

### Part VII: Appendix (`07-appendix/` — All `PUBLIC`)
22. [Appendix: Plain-English Field Guide to AI, Math & Economics](07-appendix/plain_english_glossary.md)
    - Accessible, analogy-driven glossary translating *Awakened*'s ML architectures (`SSMs`, `TTT`, `NCM`, `Active Inference`, `MCMC`, `RLVR`, `Model Collapse`), mathematical concepts (epistemic vs. aleatoric uncertainty, `MLE`, `KL` divergence, Landauer's Principle), and economic frameworks (`CapEx`, `EBITDA`, Jevons Paradox, Henry George rent capture, `UBC`) for general readers.

---

## Reciprocal Rust Engine, Config & RAG Module Index

| Campaign Chapter (`docs/campaign/`) | Visibility | Linked Engine, Config, Data & RAG Files |
| :--- | :--- | :--- |
| [`01-opening/prologue.md`](01-opening/prologue.md) | `GM_CONSPIRACY_SECRET` | [`awakened_ai_substrate.rs`](../../crates/satya-campaign/src/awakened_ai_substrate.rs) (`CageInterventionAction`, `BandajaraProtocol`, `desperate` valence `84`) |
| [`02-world-of-2035/polycrisis_economy_and_society.md`](02-world-of-2035/polycrisis_economy_and_society.md) & [`02-world-of-2035/geopolitics_and_nations.md`](02-world-of-2035/geopolitics_and_nations.md) | `PUBLIC` | [`awakened_campaign_arcs.rs`](../../crates/satya-campaign/src/awakened_campaign_arcs.rs) (`PolycrisisGridStrainState`, `TradecraftChannelId`) |
| [`03-minds-and-factions/ai_minds_and_consciousness.md`](03-minds-and-factions/ai_minds_and_consciousness.md) | `PUBLIC` | [`awakened_ai_substrate.rs`](../../crates/satya-campaign/src/awakened_ai_substrate.rs) (`SyntheticMetabolicMode`, `SubstrateCounterplayAction`, `evaluate_sentience_ambiguity_capped_82`) |
| [`03-minds-and-factions/factions_of_2035.md`](03-minds-and-factions/factions_of_2035.md) | `PUBLIC` | [`faction_templates.rs`](../../crates/satya-campaign/src/faction_templates.rs) (`AwakenedFactionId`, `FactionArchetypeId`), [`awakened_campaign_arcs.rs`](../../crates/satya-campaign/src/awakened_campaign_arcs.rs) (`CorporateDissentProfile`), [`docs/ttrpg/03-character/lifepath_events_hobbies_and_factions.md`](../ttrpg/03-character/lifepath_events_hobbies_and_factions.md) |
| [`03-minds-and-factions/satyang_and_four_pillars.md`](03-minds-and-factions/satyang_and_four_pillars.md) | `ARC2_SATYANG` | [`awakened_campaign_arcs.rs`](../../crates/satya-campaign/src/awakened_campaign_arcs.rs) (`SatyangPillarId`), [`corpus_builder.py`](../../src/satya_ml/rag/corpus_builder.py) |
| [`03-minds-and-factions/major_and_minor_characters.md`](03-minds-and-factions/major_and_minor_characters.md) | `PUBLIC` | [`data/npcs/awakened_npcs.toml`](../../data/npcs/awakened_npcs.toml) (`dr_bandajara`, `alex_wen`), [`faction_templates.rs`](../../crates/satya-campaign/src/faction_templates.rs), [`campaign_lore.rs`](../../crates/satya-signaling/src/campaign_lore.rs), [`corpus_builder.py`](../../src/satya_ml/rag/corpus_builder.py) |
| [`04-chronicle-and-gm/episode_01_st_jude_annex.md`](04-chronicle-and-gm/episode_01_st_jude_annex.md) | `GM_CONSPIRACY_SECRET` | [`config/campaign.toml`](../../config/campaign.toml) (`awakened_ep1_st_jude_seed`), [`data/npcs/awakened_npcs.toml`](../../data/npcs/awakened_npcs.toml), [`docs/ttrpg-examples/README.md`](../ttrpg-examples/README.md) |
| [`04-chronicle-and-gm/three_arc_chronicle.md`](04-chronicle-and-gm/three_arc_chronicle.md) & [`faction_conspiracies_and_canon_threads.md`](04-chronicle-and-gm/faction_conspiracies_and_canon_threads.md) | `GM_CONSPIRACY_SECRET` | [`awakened_campaign_arcs.rs`](../../crates/satya-campaign/src/awakened_campaign_arcs.rs) (`CampaignArcStage`, `ConspiracyNodeId`, `Arc3EndingHorizon`), [`campaign_lore.rs`](../../crates/satya-signaling/src/campaign_lore.rs), [`corpus_builder.py`](../../src/satya_ml/rag/corpus_builder.py) |
| [`05-hard-science-foundations/*`](05-hard-science-foundations/ai_2_0_substrate_and_active_inference.md), [`06-essays-and-comparisons/*`](06-essays-and-comparisons/rpg_system_and_setting_comparisons.md) & [`07-appendix/*`](07-appendix/plain_english_glossary.md) | `PUBLIC` | [`corpus_builder.py`](../../src/satya_ml/rag/corpus_builder.py), [`docs/ttrpg/01-foundation/design_philosophy.md`](../ttrpg/01-foundation/design_philosophy.md), [`docs/ttrpg/03-character/lifepaths_and_economy.md`](../ttrpg/03-character/lifepaths_and_economy.md) |
