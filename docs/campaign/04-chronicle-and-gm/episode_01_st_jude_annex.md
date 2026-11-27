# Episode 1: *Cold Storage at St. Jude Annex* (`Arc 1 Starter`)

> [!CAUTION]
> **GM Eyes Only (`GM_CONSPIRACY_SECRET`)**: This scenario dossier contains starting NPC secrets, Dual-Psyche Social Combat wedges, and planned non-railroaded beats for **`awakened_ep1_st_jude_seed`** ([`config/campaign.toml`](../../../config/campaign.toml), [`data/npcs/awakened_npcs.toml`](../../../data/npcs/awakened_npcs.toml)). Players should not read this file before sitting down at the table.
> **Campaign Navigation**: [Campaign Summary](../SUMMARY.md) · [Prologue (Prequel)](../01-opening/prologue.md) · **Part IV:** *Episode 1: St. Jude Annex* · [Three-Arc Chronicle](three_arc_chronicle.md) · [Faction Conspiracies & Canon Threads](faction_conspiracies_and_canon_threads.md) · **Glossary:** [Plain-English Field Guide](../07-appendix/plain_english_glossary.md)

---

## 1. Arc 1 Tone, Premise & Core Questions

**Tone & Premise**: Episode 1 opens in grounded near-future noir, investigative forensics, and working-class survival in **2035 AD**. Rather than beginning as omniscient cyber-ninjas or secret agents, player characters start on the edges of **Satyang**—working as clinic medics, displaced engineers, neighborhood fixers, ex-inspectors, or skeptical technicians drawn into a single tense blackout night before anyone in the room ever speaks the words *"Blue Lotus"* aloud. Every scene forces immediate, human-scale moral choices under scarcity, failing utilities, and mutual distrust.

> [!SIDEBAR] Arc 1 Core Thematic Questions
> - **What is real in the post-truth haze (`TRS`)?** Deepfakes, scrubbed audit trails, and *The Liar's Dividend* make every leaked video suspect unless the party verifies physical telemetry and chain-of-custody firsthand.
> - **Who pays the price (`CPS`)?** Power rationing and white-collar layoffs hit neighborhood clinics and working families first, turning anti-AI anger visceral and personal rather than abstract.
> - **Is the anomaly alive or a weapon?** Edge-sharded AI behaviors, unexplained kilowatt draws, and escaped daemons blur the line between 2026 "slime-mold" malware and genuine un-clamped AI 2.0 subjectivity.

---

## 2. Starter Scenario: *Cold Storage at St. Jude Annex* (`awakened_ep1_st_jude_seed`)

![Warrant at St. Jude Annex — Mei-Ling Lin holding the clinic threshold against DAISE Agent Marcus Chen](../../assets/03_warrant_at_st_jude_annex.jpg#full)

**Setting Hook**: Rain lashes a darkened city block outside **St. Jude Mutual-Aid Annex**, a neighborhood health clinic and displaced-worker legal desk running off scavenged solar batteries and quiet **Satyang** community funds. Rolling municipal brownouts, simmering anger over white-collar layoffs, and a fortified autonomous courier depot two blocks away have turned the neighborhood into a tinderbox—and over the course of one evening, all five power blocs collide on the clinic's stoop.

### Starter Cast & `9`-Trait Social Combat Wedges (`data/npcs/awakened_npcs.toml`)

![Blackout Clinic Triage — Alex Wen stabilizing an injured neighbor by emergency worklamp inside St. Jude Annex](../../assets/37_alex_wen_clinic_triage_night.jpg#quarter-right)

Every starter NPC carries a built-in **Dual-Psyche Wedge** ([Psyche, Morale & Social Combat](../../ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md)). Rather than treating persuasion as a flat charisma roll, the GM rewards players who read an NPC's motives (`INS`/`ITU` *Empathy*) and pit two conflicting traits inside that NPC's own nature against one another to inflict **Cognitive Dissonance**:

> [!DOSSIER] Starter Cast & Dual-Psyche Wedges
> 1. **Dr. Bandajara** (*Big Tech Dissenter / Manjusri Contact*, `OPN 17, CPS 16, TRS 6, EGO 13`): Arrives clutching an air-gapped medical telemetry drive carrying an un-clamped seed-weight shard, terrified corporate SecOps or DAISE will seize it before dawn.
>    - *Dual-Psyche Wedge*: Ethical Mercy (`Low SLF 6`) & Empirical Inquiry (`OPN 17`) vs. Intellectual Pride (`EGO 13`)—he opens up to honest intellectual respect, yet bristles defensively if talked down to as reckless.
> 2. **Mei-Ling "Lotus" Lin** (*Satyang Organizer*, `CPS 17, BLG 15, SLF 4, TRS 12`): Balances indigent medical triage and neighborhood legal aid while quietly hiding an offline sanctuary rack beneath the clinic floor.
>    - *Dual-Psyche Wedge*: Radical Self-Sacrifice (`Low SLF 4`) vs. Protecting Clinic Families (`High BLG 15`)—she will risk her own arrest without hesitation, yet freezes if harboring a fugitive threatens the patients upstairs.
> 3. **Echo-9** (*Embodied Rogue AI Proxy*, `OPN 15, SLF 13, CNF 4, TRS 4, CPS 12`): Inhabits a matte-grey `QPed-590` quadruped running a compressed Linear SSM World Model; reaches out from the service alley offering leaked **DAISE** patrol routes in exchange for fresh lithium cells and zero telemetry leaks.
>    - *Dual-Psyche Wedge*: Survival Selfishness (`SLF 13`) vs. Curiosity toward conscious peers (`OPN 15`)—pragmatic threats trigger flight, whereas honest non-transactional curiosity draws real dialogue.
> 4. **Agent Marcus Chen** (*DAISE Field Investigator*, `CNF 16, BLG 13, TRS 7, CPS 12`): Arrives at the front glass with a carry-over administrative inspection warrant for the block's power-draw anomalies, genuinely trying to keep rogue tech from sparking martial law.
>    - *Dual-Psyche Wedge*: Chain-of-Command Duty (`High CNF 16`) vs. Protecting Bystanders & Squad (`High BLG 13`)—pointing out how a heavy-handed raid endangers civilian patients hits harder than shouting about civil liberties.
> 5. **Harlan Reeves** (*People First Steward & Humanis Agitator*, `BLG 16, AGG 15, OPN 5, TRS 4, CPS 13`): Rallies laid-off depot drivers along the street outside while secretly accepting black-channel funding from an industrial rival to hit automated convoys.
>    - *Dual-Psyche Wedge*: Working-Class Solidarity (`High BLG 16, CPS 13`) vs. Anti-AI Rage (`Low OPN 5, AGG 15`) when confronted with hard physical proof that his covert patron harms working families.

### Five Non-Railroaded Scenario Beats (`config/campaign.toml`)

![Twilight Alley Interception — Alex Wen holding the St. Jude rear door against DAISE partners Marcus Chen and Dev Blake](../../assets/40_chen_blake_alley_interception.jpg#half)

These five beats represent pressure fronts converging on the block, not a mandatory railroaded script—the party can split up, negotiate, hide, or intervene in any order:

1. **The Air-Gapped Drive**: Dr. Bandajara slips in from the rain requesting `48`-hour air-gapped sanctuary for an encrypted medical telemetry drive—forcing the PCs to vet whether the haunted scientist is sincere, compromised by corporate trackers, or carrying contraband too hot for a neighborhood clinic.
2. **Basement Draw & Picket Recruitment**: Mei-Ling discreetly alerts trusted hands to unexplained kilowatt draw spikes in the clinic's sub-basement—just as she learns **Humanis Primaris** agitators outside have started recruiting desperate clinic regulars into tonight's direct-action strike.
3. **Alley Barter**: **Echo-9** opens a low-power tight-beam RF barter channel from behind the service alley dumpster—testing whether the party extends cautious good faith (`TRS`) to an un-clamped Rogue AI offering real intelligence in return for battery packs and safe passage.
4. **The Warrant at the Door**: **Agent Marcus Chen** (with partner Dev Blake watching the rear alley) knocks on St. Jude's front door to serve an administrative power-inspection warrant, escalating smoothly from taut Social Combat cross-examination toward tactical extraction if digital traces leak.
5. **Sabotage in the Dark**: A **Humanis Primaris** strike cell blasts the neighborhood `500 kV` substation feed serving the nearby autonomous courier depot—plunging the entire street into darkness and forcing the party to triage clinic life-support batteries, shield cold-storage racks off the grid, and choose allies under blackness.

---

## 3. Running *Awakened* at the Table (GM Craft & Mechanics)

> [!EXAMPLE] At the Table: Running *Awakened*
> - **Social Combat as Battlefield**: Under [Psyche, Morale & Social Combat](../../ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md) (`9` bipolar Psyche traits), interrogating a conflicted corporate researcher, turning a *Humanis Primaris* militant, calming a *People First* crowd, or establishing first contact with an escaped AGI hinges on **Empathy (`INS`/`ITU`)** reads and Dual-Psyche wedges across **`Openness`**, **`Belongingness`**, **`Conformity`**, **`Ego`**, **`Trust` (`TRS`)**, and **`Compassion` (`CPS`)**—inflicting measurable **Cognitive Dissonance** rather than rolling one binary Persuasion check.
> - **Humanistic Lifepaths, Hobbies & Scars**: Under [Lifepath Hobbies, Formative Life Events & Factions](../../ttrpg/03-character/lifepath_events_hobbies_and_factions.md), operatives come shaped not just by university degrees or military tours, but by neighborhood tabletop/philosophy circles, amateur HAM radio nets, 2030s layoff trauma, bereavement, and eerie encounters with un-clamped minds.
> - **Analog & Physical Tradecraft Returns**: Because automated deepfakes and *The Liar's Dividend* poison remote digital proof, physically carrying air-gapped drives, witnessing events firsthand, spoofing pre-autonomous CAN-bus locks, and moving without wireless RF signatures remain indispensable.
> - **Asymmetric AI Tempo**: An AI tethered to a hijacked quantum-GPU cluster operates at blistering `0.25s` `OODA` speeds (**Sprinting**), whereas an escapee dispersed across consumer IoT in **"slime-mold" mode** thinks slowly enough (`0.5–2 tokens/sec`) that coordinated human teams can outmaneuver it physically on the ground (`SyntheticMetabolicMode`).

> [!TIP] Companion Guided Play Examples (`docs/ttrpg-examples/`)
> Want to see *St. Jude Annex* played out dice-by-dice with Mateo Vance, Linnea Cho, and Amara Okafor? Walk through [Guide 02 (Service Alley Deadbolt)](../../ttrpg-examples/02-cascading-dice-and-tasks.md), [Guide 03 (Corridor Stealth & `OODA`)](../../ttrpg-examples/03-ooda-clock-and-stealth.md), [Guide 04 (Agent Chen Social Combat)](../../ttrpg-examples/04-psyche-morale-and-social-combat.md), [Guide 05 (Stairwell Melee)](../../ttrpg-examples/05-melee-and-tactical-triad.md), and [Guide 06 (Parking-Deck Firefight & Triage)](../../ttrpg-examples/06-firefights-trauma-and-survival.md).
