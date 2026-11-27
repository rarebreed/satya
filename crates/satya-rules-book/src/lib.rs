//! Canonical Satya TTRPG Rules Book catalog and multi-chapter search engine
//! (`docs/ttrpg/SUMMARY.md`, Parts I–VII, Chapters 1–24).

use satya_schema::{
    RulesBookCatalogResponse, RulesChapterDocument, RulesChapterSummary, RulesSearchHit,
};

pub mod mermaid_svg;
pub mod rules_book_html;

pub use crate::mermaid_svg::render_mermaid_to_html;
pub use crate::rules_book_html::{
    escape_html_text, extract_markdown_headings, format_inline_markdown, render_latex_math_to_html,
    render_rules_markdown_to_html, slugify_heading,
};

/// Static compile-time definition of a canonical chapter in `docs/ttrpg/SUMMARY.md`.
struct StaticChapterSpec {
    chapter_id: &'static str,
    part: &'static str,
    title: &'static str,
    relative_path: &'static str,
    description: &'static str,
    embedded_markdown: &'static str,
}

const STATIC_CHAPTERS: &[StaticChapterSpec] = &[
    StaticChapterSpec {
        chapter_id: "design_philosophy",
        part: "Part I: Foundation & Mathematical Calibration",
        title: "An RPG for a More Civilized Age: Design Philosophy",
        relative_path: "01-foundation/design_philosophy.md",
        description: "Origins of Satya, realism vs. escapism, and how Satya solves 17 structural flaws of traditional RPG systems.",
        embedded_markdown: include_str!("../../../docs/ttrpg/01-foundation/design_philosophy.md"),
    },
    StaticChapterSpec {
        chapter_id: "notation_glossary",
        part: "Part I: Foundation & Mathematical Calibration",
        title: "Mathematical Notation, Variables & Formula Glossary",
        relative_path: "01-foundation/mathematical_notation_glossary.md",
        description: "Canonical reference for every mathematical variable, subscript (_P, _GM, _n, _eff), dice/cascade symbol (TN_P, TN_GM, delta_over, delta_under, Delta M), attribute abbreviation, OODA/Action Clock metric, and Tactical Triad formula.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/01-foundation/mathematical_notation_glossary.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "probability",
        part: "Part I: Foundation & Mathematical Calibration",
        title: "Probability & Calibration of the Cascading Dice Mechanic",
        relative_path: "01-foundation/probability_and_calibration.md",
        description: "Mathematical foundation, distribution topology (skewness, modality, tail behavior), scale invariance across K, avoiding the Skilled Botch Paradox, and the 1-Pair Tiered GM Resistance Rule.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/01-foundation/probability_and_calibration.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "tasks",
        part: "Part II: Core Mechanics, Task Resolution & Action Economy",
        title: "2d20xN Cascade Engine & Task Resolution",
        relative_path: "02-core-mechanics/cascading_d20_and_tasks.md",
        description: "Canonical 2d20xN cascade dice mechanic, Player Personal Target TN_P = 31 - C_P + Delta_tool vs. Hidden GM Resistance Target TN_GM = 21 - Delta_ext (1-Pair Tiered GM Resistance), carry modes, 7-Way Contested Outcome Matrix, 5-step Task Pipeline, Tool Tiers, Gate Classifications, Extended Project Clocks, and Sarah's lockpicking walkthrough.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "task_requirements_catalog",
        part: "Part II: Core Mechanics, Task Resolution & Action Economy",
        title: "Master Task Requirements Catalog (Section 8)",
        relative_path: "02-core-mechanics/task_requirements_catalog.md",
        description: "Canonical Section 8 reference table defining gate classes, primary/complementary skills (N_min), controlling characteristics (C_P), mandatory/situational tools (Delta_tool), and hidden GM resistance modifiers (Delta_ext) across all skill categories.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/02-core-mechanics/task_requirements_catalog.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "action_economy",
        part: "Part II: Core Mechanics, Task Resolution & Action Economy",
        title: "Action Economy, OODA Loop & Time Scales",
        relative_path: "02-core-mechanics/action_economy_and_ooda.md",
        description: "Temporal calibration (4 ticks = 1s, 240 ticks = 1 min), Full OODA Loop & Quick Thinking (D_cog), Count-Up Clock mechanics, Base TC tables, action_time_factor, and the Observe clock bridge.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/02-core-mechanics/action_economy_and_ooda.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "stealth_and_awareness",
        part: "Part II: Core Mechanics, Task Resolution & Action Economy",
        title: "Stealth, Passive Awareness, Observation & Investigation",
        relative_path: "02-core-mechanics/stealth_awareness_and_investigation.md",
        description: "Passive Awareness (0 Ticks, AWR) vs. Active Observation (6 Ticks, 6-sense isolation), Attention States & Watch Fatigue, 4-stage Alertness Ladder, Acoustic/Thermal/RF/Social Stealth, Tail Craft (SDR), and Forensic Trace Decay.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/02-core-mechanics/stealth_awareness_and_investigation.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "characteristics",
        part: "Part III: Character Architecture, Skills, Lifepaths & Economy",
        title: "Characteristics, Senses, Psyche & Aptitudes",
        relative_path: "03-character/characteristics_and_aptitudes.md",
        description: "Physical, Mental, Social, Sensory, and 9 Psyche (EGO, SLF, OPN, BLG, EXT, CNF, AGG, TRS, CPS) characteristics on the 1-20 Controlling Scale, derived metrics, and domain Aptitudes (A in [-2, +3]).",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/03-character/characteristics_and_aptitudes.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "skills_and_trees",
        part: "Part III: Character Architecture, Skills, Lifepaths & Economy",
        title: "Skill System & Hierarchical Skill Trees 1–4",
        relative_path: "03-character/skills_and_trees.md",
        description: "Talents (TAL) vs. Physical (PHY), Knowledge (KNO), and Hybrid (PHY/KNO) skills, Multi-Category taxonomy, [Access] restriction tiers, Prerequisite Rules, and Hierarchical Skill Trees 1–4 (Mathematics, Physical Sciences/Trades, Computing/AI 2.0, Biology/Medicine).",
        embedded_markdown: include_str!("../../../docs/ttrpg/03-character/skills_and_trees.md"),
    },
    StaticChapterSpec {
        chapter_id: "skills_trees_tactical_and_social",
        part: "Part III: Character Architecture, Skills, Lifepaths & Economy",
        title: "Hierarchical Skill Trees 5–8: Combat, Vehicles, Social & Tradecraft",
        relative_path: "03-character/skills_trees_tactical_and_social.md",
        description: "Hierarchical Skill Trees 5–8: Unarmed/Melee/Firearms/Heavy Ordnance (Tree 5), Athletics/Vehicles/Aviation/Drones (Tree 6), Social/Influence/Interrogation/PsyOps (Tree 7), and Literacy/Law/Intelligence/Tradecraft/Artistic (Tree 8).",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/03-character/skills_trees_tactical_and_social.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "lifepaths_and_economy",
        part: "Part III: Character Architecture, Skills, Lifepaths & Economy",
        title: "Lifepath Character Creation, Socioeconomic Upkeep, Contacts, Cover & Heat",
        relative_path: "03-character/lifepaths_and_economy.md",
        description: "Chronological Lifepath education & 14 career tracks (Ages 0-18+), DIS + Mental + APT advancement rolls, Socioeconomic Tier (SQM 1-10) upkeep, Black-Market [Access] gates, Operational Heat, Contacts & Favors, Cover Identities, and Safehouses.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/03-character/lifepaths_and_economy.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "lifepath_events_hobbies_and_factions",
        part: "Part III: Character Architecture, Skills, Lifepaths & Economy",
        title: "Lifepath Hobbies, Formative Life Events, Faction Templates & Walkthroughs",
        relative_path: "03-character/lifepath_events_hobbies_and_factions.md",
        description: "12 Personal Hobbies/Clubs (Experience SP), 2d20 Formative Life Events Table, 5 Awakened Faction Psyche & Lifepath Templates (TRS/CPS), and 4 Worked Walkthroughs.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/03-character/lifepath_events_hobbies_and_factions.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "psyche_and_social",
        part: "Part IV: Psyche, Morale & Social Combat",
        title: "Psyche, Morale & Social Combat",
        relative_path: "04-psyche-and-social/psyche_morale_and_social_combat.md",
        description: "Acting against Psyche (DIS Friction Checks & Trait Drift), Conviction Surges, Psychological Drives (Harm:Other vs. Self:Protect), the 5 Morale Conditions, and Social Combat (Cognitive Dissonance).",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/04-psyche-and-social/psyche_morale_and_social_combat.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "tactical_triad",
        part: "Part V: Combat Architecture, Close Quarters & Firefights",
        title: "Combat Architecture, Tactical Triad & Pools",
        relative_path: "05-combat/tactical_triad_and_pools.md",
        description: "Combat philosophy, anatomical trauma over Hit Points, Count-Up Action Clock overview, and the Envelope & Fill Tactical Pool Architecture (P_base = C_combat + 3N) across Tempo, Control, and Force.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/05-combat/tactical_triad_and_pools.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "melee_combat",
        part: "Part V: Combat Architecture, Close Quarters & Firefights",
        title: "Melee Combat, Close-Quarters Tactics & The Tactical Triad",
        relative_path: "05-combat/melee_combat.md",
        description: "Close-quarters anatomy, the Tactical Triad (Tempo, Control, Force), Guard & Seizing the Initiative, Root vs. throws, and Viktor vs. Elena walkthrough.",
        embedded_markdown: include_str!("../../../docs/ttrpg/05-combat/melee_combat.md"),
    },
    StaticChapterSpec {
        chapter_id: "firearms_and_ballistics",
        part: "Part V: Combat Architecture, Close Quarters & Firefights",
        title: "Granular Ballistics, Ammunition Physics & Firearm Engineering",
        relative_path: "05-combat/firearms_and_ballistics.md",
        description: "Four coupled firearm engineering chains, Ammunition & Internal/External Ballistics (m_proj, m_prop powder grains -> impact kinetic energy E_k, eta_bbl, eta_action), Master Ammunition Reference Table (9mm to .50 BMG), Feed Systems, Terminal Ballistics (I_KE + I_cal + W_yaw inverse-accuracy tumbling wound channels), Action Types, Weight, Center of Gravity (Delta_CoG), Overall Length (OAL), and Free-Floating vs. Clamped Barrel Specs.",
        embedded_markdown: include_str!("../../../docs/ttrpg/05-combat/firearms_and_ballistics.md"),
    },
    StaticChapterSpec {
        chapter_id: "ranged_combat_and_armor",
        part: "Part V: Combat Architecture, Close Quarters & Firefights",
        title: "Ranged Combat, Weapon Reliability, Master Catalogs & Armor",
        relative_path: "05-combat/ranged_combat_and_armor.md",
        description: "Ranged Fire Modes & Recoil Bracing (F >= R_eff) in the Tactical Triad, the Accuracy-Reliability Tolerance Trade-Off (tau_tol vs. Rel), Malfunctions, Clear Jam & Field-Strip times, Master Weapons & Armor Catalogs (PR vs. BABT), and worked firefights (Mateo vs. Sgt. Vance & AKM vs. 6.5mm Creedmoor).",
        embedded_markdown: include_str!("../../../docs/ttrpg/05-combat/ranged_combat_and_armor.md"),
    },
    StaticChapterSpec {
        chapter_id: "arsenal_and_explosives",
        part: "Part V: Combat Architecture, Close Quarters & Firefights",
        title: "Expanded Arsenal, Explosives, Breaching & Tactical Gear",
        relative_path: "05-combat/arsenal_explosives_and_tactical_gear.md",
        description: "Six extra calibers (.22 LR to .300 BLK), 10 expanded firearms (#13-#22 through .338 Lapua & .50 BMG), 5 specialty 12-Gauge shells, bows/crossbows & less-lethal systems, explosives/blast overpressure (1.5x indoor reflection), breaching vs. MaterialClass PR, and optics/gear.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/05-combat/arsenal_explosives_and_tactical_gear.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "health_and_trauma",
        part: "Part VI: Health, Trauma, Movement, Vehicles & Survival",
        title: "Health, Anatomical Trauma, Fatigue & Mental Conditions",
        relative_path: "06-health-and-survival/health_and_trauma.md",
        description: "1d20 Hit Locations & Called Shots, Fatigue, Structural Wounds, Broken Bones, Hemorrhaging & Blood Volume, Wound Shock, Status Effects, and Mental Trauma.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/06-health-and-survival/health_and_trauma.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "movement_and_vehicles",
        part: "Part VI: Health, Trauma, Movement, Vehicles & Survival",
        title: "Movement, Encumbrance & Vehicles",
        relative_path: "06-health-and-survival/movement_encumbrance_and_vehicles.md",
        description: "Combat movement gaits (m/tick), overland travel, FOR & MAS Encumbrance tiers, bulk/stowage slots, Vehicles & Chases, 2035 Vehicle Catalog, firing platforms, and component trauma.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/06-health-and-survival/movement_encumbrance_and_vehicles.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "environmental_hazards",
        part: "Part VI: Health, Trauma, Movement, Vehicles & Survival",
        title: "Environmental Hazards, Survival & Field Pharmacology",
        relative_path: "06-health-and-survival/environmental_hazards_and_survival.md",
        description: "Hydration, Starvation, Cold/Heat (CLO), Sleep, Hypoxia/Vacuum, Toxins, Radiation, Falling, Fire & Smoke, Electricity & EMP, Weather/Diving Pressure, Wound Sepsis, and Field Pharmacology.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/06-health-and-survival/environmental_hazards_and_survival.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "engineering_and_weapon_design",
        part: "Part VII: Engineering, Robotics, AI Architectures & Electronic Warfare",
        title: "Engineering, Workshops & Custom Weapon/Armor Design",
        relative_path: "07-engineering-and-tech/engineering_and_weapon_design.md",
        description: "4-tier Workshop Facilities (tau_max & Q_mfg ceilings), Project Clocks, Prototype Quirks, Bullet Construction & 17-caliber Custom Firearm Designer (wrapping ballistics.rs), Custom Melee/Shields, and Body Armor.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/07-engineering-and-tech/engineering_and_weapon_design.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "robotics_and_ai_architecture",
        part: "Part VII: Engineering, Robotics, AI Architectures & Electronic Warfare",
        title: "Robotics, Drones, AI 2.0 Architectures & Cyber-Prosthetics",
        relative_path: "07-engineering-and-tech/robotics_drones_and_ai_architecture.md",
        description: "Modular Drones/UGVs (6 chassis, 8 modules, battery endurance), Chassis Trauma & EMP, AI 2.0 Architecture Engineering & Verification Debt (bridging SyntheticMetabolicMode), and Clinical Cyber-Prosthetics & Myomer Augmentations.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/07-engineering-and-tech/robotics_drones_and_ai_architecture.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "electronic_warfare",
        part: "Part VII: Engineering, Robotics, AI Architectures & Electronic Warfare",
        title: "Electronic Warfare, 2035 Surveillance, ECM & ECCM",
        relative_path: "07-engineering-and-tech/electronic_warfare_and_surveillance.md",
        description: "7-node 2035 Surveillance Stack, 4-phase Network Intrusion & hard Air-Gap rule, 6 ECM & 5 ECCM methods, active RF DF Triangulation paradox, Optical/Fiber RF immunity, AI uplink severing, and Custom EW Hardware Designer.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/07-engineering-and-tech/electronic_warfare_and_surveillance.md"
        ),
    },
];

fn load_chapter_markdown(spec: &StaticChapterSpec) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let candidate = std::path::Path::new("docs")
            .join("ttrpg")
            .join(spec.relative_path);
        if let Ok(text) = std::fs::read_to_string(&candidate)
            && !text.trim().is_empty()
        {
            return text;
        }
    }
    spec.embedded_markdown.to_string()
}

/// Returns all canonical chapters in the Satya TTRPG Rules Book (`docs/ttrpg/`),
/// including metadata summaries, raw Markdown, and rendered HTML.
pub fn canonical_rules_chapters() -> Vec<RulesChapterDocument> {
    STATIC_CHAPTERS
        .iter()
        .map(|spec| {
            let markdown = load_chapter_markdown(spec);
            let headings = extract_markdown_headings(&markdown);
            let rendered_html = render_rules_markdown_to_html(&markdown);
            RulesChapterDocument {
                summary: RulesChapterSummary {
                    chapter_id: spec.chapter_id.to_string(),
                    part: spec.part.to_string(),
                    title: spec.title.to_string(),
                    relative_path: spec.relative_path.to_string(),
                    description: spec.description.to_string(),
                    headings,
                },
                markdown,
                rendered_html,
            }
        })
        .collect()
}

pub fn list_rules_chapters() -> Vec<RulesChapterSummary> {
    canonical_rules_chapters()
        .into_iter()
        .map(|doc| doc.summary)
        .collect()
}

fn resolve_chapter_alias(needle: &str) -> &str {
    match needle {
        "background" | "design-notes/background" => "design-philosophy",
        "satya-d20-rules" | "cascading-d20-and-tasks" => "tasks",
        "skill-design" | "skills-and-trees" => "skills-and-trees",
        "social-interactions" | "psyche-morale-and-social-combat" => "psyche-and-social",
        "combat" | "combat/combat" | "tactical-triad-and-pools" => "tactical-triad",
        "world-and-equipment-stubs" | "systems/world-and-equipment-stubs" => {
            "ranged-combat-and-armor"
        }
        "stealth-awareness-and-investigation" | "stealth-and-investigation" => {
            "stealth-and-awareness"
        }
        "arsenal-explosives-and-tactical-gear" | "explosives-and-arsenal" => {
            "arsenal-and-explosives"
        }
        "movement-and-environment" | "movement-vehicles-and-environment" => "movement-and-vehicles",
        "weapon-design" | "engineering-design" | "custom-weapons" => {
            "engineering-and-weapon-design"
        }
        "robotics-drones-and-ai-architecture"
        | "robotics-and-drones"
        | "cyber-prosthetics"
        | "ai-architecture" => "robotics-and-ai-architecture",
        "electronic-warfare-and-surveillance" | "ecm-eccm" | "ew" | "surveillance-and-ew" => {
            "electronic-warfare"
        }
        other => other,
    }
}

pub fn get_rules_chapter(chapter_id: &str) -> Option<RulesChapterDocument> {
    let raw_needle = chapter_id
        .trim()
        .trim_end_matches(".md")
        .to_lowercase()
        .replace('_', "-");
    let needle = resolve_chapter_alias(&raw_needle);
    canonical_rules_chapters().into_iter().find(|doc| {
        doc.summary.chapter_id.to_lowercase().replace('_', "-") == needle
            || doc
                .summary
                .relative_path
                .to_lowercase()
                .trim_end_matches(".md")
                .replace('_', "-")
                == needle
    })
}

/// Searches across all chapters of the Satya TTRPG Rules Book (`docs/ttrpg/`) and returns
/// the top matching sections ranked by keyword relevance.
pub fn search_rules_book(query: &str, max_results: usize) -> Vec<RulesSearchHit> {
    let limit = max_results.max(1);
    let q_lower = query.trim().to_lowercase();
    let tokens: Vec<String> = q_lower
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| t.len() >= 2)
        .map(str::to_string)
        .collect();

    let chapters = canonical_rules_chapters();
    if tokens.is_empty() {
        return chapters
            .into_iter()
            .take(limit)
            .map(|doc| {
                let snippet: String = doc.markdown.lines().take(12).collect::<Vec<_>>().join("\n");
                RulesSearchHit {
                    chapter_id: doc.summary.chapter_id,
                    chapter_title: doc.summary.title.clone(),
                    relative_path: doc.summary.relative_path,
                    section_heading: doc.summary.title,
                    snippet,
                    score: 1,
                }
            })
            .collect();
    }

    let mut hits = Vec::new();
    for doc in &chapters {
        let mut sections: Vec<(String, String)> = Vec::new();
        let mut current_heading = doc.summary.title.clone();
        let mut current_lines = Vec::new();

        for line in doc.markdown.lines() {
            if let Some(h2) = line.trim().strip_prefix("## ") {
                if !current_lines.is_empty() {
                    sections.push((current_heading.clone(), current_lines.join("\n")));
                    current_lines.clear();
                }
                current_heading = h2.trim().to_string();
                current_lines.push(line);
            } else {
                current_lines.push(line);
            }
        }
        if !current_lines.is_empty() {
            sections.push((current_heading, current_lines.join("\n")));
        }

        let title_lower = doc.summary.title.to_lowercase();
        let id_lower = doc.summary.chapter_id.to_lowercase();

        for (heading, body) in sections {
            let heading_lower = heading.to_lowercase();
            let body_lower = body.to_lowercase();

            let mut score = 0usize;
            for tok in &tokens {
                if id_lower.contains(tok) || title_lower.contains(tok) {
                    score += 4;
                }
                if heading_lower.contains(tok) {
                    score += 6;
                }
                let occurrences = body_lower.matches(tok.as_str()).count();
                score += occurrences.min(10);
            }

            if score > 0 {
                let snippet: String = body.lines().take(24).collect::<Vec<_>>().join("\n");
                hits.push(RulesSearchHit {
                    chapter_id: doc.summary.chapter_id.clone(),
                    chapter_title: doc.summary.title.clone(),
                    relative_path: doc.summary.relative_path.clone(),
                    section_heading: heading,
                    snippet,
                    score,
                });
            }
        }
    }

    hits.sort_by_key(|a| std::cmp::Reverse(a.score));
    hits.truncate(limit);
    hits
}

pub fn build_rules_catalog_response(
    query: Option<&str>,
    max_hits: usize,
) -> RulesBookCatalogResponse {
    let chapters = list_rules_chapters();
    let search_hits = match query.map(str::trim).filter(|q| !q.is_empty()) {
        Some(q) => search_rules_book(q, max_hits),
        None => Vec::new(),
    };
    RulesBookCatalogResponse {
        chapters,
        search_hits,
    }
}

#[cfg(test)]
mod tests;
