//! Canonical Satya TTRPG Rules Book catalog and multi-chapter search engine
//! (`docs/ttrpg/SUMMARY.md`, Parts I–VI, Chapters 1–17).

use satya_schema::{
    RulesBookCatalogResponse, RulesChapterDocument, RulesChapterSummary, RulesSearchHit,
};

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
        title: "Cascading 2d20 Engine & Task Resolution",
        relative_path: "02-core-mechanics/cascading_d20_and_tasks.md",
        description: "Canonical Two-List Cascading (2d20) dice mechanic, Player Personal Target TN_P = 31 - C_P + Delta_tool vs. Hidden GM Resistance Target TN_GM = 21 - Delta_ext (1-Pair Tiered GM Resistance), carry modes, 7-Way Contested Outcome Matrix, 5-step Task Pipeline, Tool Tiers, Gate Classifications, Extended Project Clocks, and Sarah's lockpicking walkthrough.",
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
        title: "Action Economy, OODA Loop, Time Scales & Stealth",
        relative_path: "02-core-mechanics/action_economy_and_ooda.md",
        description: "Temporal calibration (4 ticks = 1s, 240 ticks = 1 min), Full OODA Loop & Quick Thinking (D_cog), Count-Up Clock mechanics, Base TC tables, action_time_factor, and Multi-Sensory Stealth & Detection.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/02-core-mechanics/action_economy_and_ooda.md"
        ),
    },
    StaticChapterSpec {
        chapter_id: "characteristics",
        part: "Part III: Character Architecture, Skills, Lifepaths & Economy",
        title: "Characteristics, Senses, Psyche & Aptitudes",
        relative_path: "03-character/characteristics_and_aptitudes.md",
        description: "Physical, Mental, Social, Sensory, and Psyche (EGO, SLF, OPN, BLG, EXT, CNF, AGG) characteristics on the 1-20 Controlling Scale, derived metrics, and domain Aptitudes (A in [-2, +3]).",
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
        title: "Lifepath Character Creation, Socioeconomic Upkeep & Heat",
        relative_path: "03-character/lifepaths_and_economy.md",
        description: "Chronological Lifepath education & career progression (Ages 0-18+), DIS + Mental + APT advancement rolls, Socioeconomic Tier (SQM 1-10) upkeep, Black-Market [Access] gates, and Operational Heat.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/03-character/lifepaths_and_economy.md"
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
        chapter_id: "movement_and_environment",
        part: "Part VI: Health, Trauma, Movement, Vehicles & Survival",
        title: "Movement, Encumbrance, Vehicles & Environmental Survival",
        relative_path: "06-health-and-survival/movement_vehicles_and_environment.md",
        description: "Combat movement gaits (m/tick), overland travel, FOR & MAS Encumbrance tiers, Vehicles & High-Speed Chases, Hydration, Starvation, Temperature (Hypothermia/Heatstroke), Hypoxia, and Toxins.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/06-health-and-survival/movement_vehicles_and_environment.md"
        ),
    },
];

/// Reads a chapter's markdown from `docs/ttrpg/<relative_path>` on native targets if present on disk,
/// falling back to the compile-time embedded markdown.
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

/// Returns the ordered list of chapter metadata summaries for the Satya TTRPG Rules Book.
pub fn list_rules_chapters() -> Vec<RulesChapterSummary> {
    canonical_rules_chapters()
        .into_iter()
        .map(|doc| doc.summary)
        .collect()
}

/// Resolves legacy chapter IDs or filenames to their canonical reorganized `chapter_id`.
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
        other => other,
    }
}

/// Looks up a single chapter by its `chapter_id` slug (or relative filename stem, including legacy aliases).
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

/// Builds a [`RulesBookCatalogResponse`] containing all chapters and optional search hits.
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

pub use crate::combat::{
    MoraleState, TacticalPoolEnvelope, TacticalPoolEnvelopeSummary, TacticalPoolParams,
    TacticalTriadContestOutcome, TacticalTriadEvaluation, compute_tactical_pool,
    compute_tactical_pool_envelope, evaluate_tactical_triad, evaluate_tactical_triad_contest,
};
pub use crate::lifepath_economy::{LifepathSkillProgression, compute_lifepath_skill_progression};
pub use crate::psyche_social::{SocialDissonanceOutcome, compute_social_dissonance_check};

#[derive(Debug, Clone)]
struct MermaidNode {
    id: String,
    label_html: String,
    is_diamond: bool,
}

#[derive(Debug, Clone)]
struct MermaidEdge {
    from: String,
    to: String,
    label: Option<String>,
    dashed: bool,
}

#[derive(Debug, Clone)]
struct MermaidSubgraph {
    id: String,
    title: String,
    node_ids: Vec<String>,
}

/// Safely formats a Mermaid node or edge label into HTML, preserving `<b>`, `<i>`, and `<br/>`
/// tags while escaping raw `&`, `<`, `>` characters.
fn format_mermaid_label(raw: &str) -> String {
    let unquoted = raw.trim().trim_matches('"').trim();
    let tokenized = unquoted
        .replace("<b>", "\x01B_OPEN\x02")
        .replace("</b>", "\x01B_CLOSE\x02")
        .replace("<strong>", "\x01B_OPEN\x02")
        .replace("</strong>", "\x01B_CLOSE\x02")
        .replace("<i>", "\x01I_OPEN\x02")
        .replace("</i>", "\x01I_CLOSE\x02")
        .replace("<em>", "\x01I_OPEN\x02")
        .replace("</em>", "\x01I_CLOSE\x02")
        .replace("<br/>", "\x01BR\x02")
        .replace("<br />", "\x01BR\x02")
        .replace("<br>", "\x01BR\x02");

    escape_html_text(&tokenized)
        .replace("\x01B_OPEN\x02", "<strong>")
        .replace("\x01B_CLOSE\x02", "</strong>")
        .replace("\x01I_OPEN\x02", "<em>")
        .replace("\x01I_CLOSE\x02", "</em>")
        .replace("\x01BR\x02", "<br/>")
}

/// Parses a single Mermaid node token such as `S1["<b>1. Gate</b>"]`, `C1{"Y >= TN?"}`, or bare `S1`.
fn parse_mermaid_node_token(token: &str) -> Option<(String, Option<(String, bool)>)> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(open_idx) = t.find(['[', '{', '(']) {
        let id = t[..open_idx].trim().to_string();
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
        let open_ch = t[open_idx..].chars().next().unwrap_or('[');
        let close_ch = match open_ch {
            '{' => '}',
            '(' => ')',
            _ => ']',
        };
        let rest = &t[open_idx + open_ch.len_utf8()..];
        let inner = rest.strip_suffix(close_ch).unwrap_or(rest).trim();
        let is_diamond = open_ch == '{';
        return Some((id, Some((format_mermaid_label(inner), is_diamond))));
    }
    if t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some((t.to_string(), None));
    }
    None
}

type MermaidEdgeStep = (Option<String>, bool, String);

/// Splits a Mermaid edge statement line into a sequence of `(node_token, Vec<MermaidEdgeStep>)` steps.
fn split_mermaid_edge_chain(line: &str) -> Option<(String, Vec<MermaidEdgeStep>)> {
    let chars: Vec<char> = line.chars().collect();
    let mut parts: Vec<MermaidEdgeStep> = Vec::new();
    let mut first_token: Option<String> = None;
    let mut seg_start = 0usize;
    let mut pending_edge: Option<(Option<String>, bool)> = None;
    let mut in_quotes = false;
    let mut bracket_depth = 0usize;
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if !in_quotes {
            if matches!(c, '[' | '{' | '(') {
                bracket_depth += 1;
            } else if matches!(c, ']' | '}' | ')') {
                bracket_depth = bracket_depth.saturating_sub(1);
            }
        }

        if !in_quotes && bracket_depth == 0 {
            let rem: String = chars[i..].iter().collect();

            if let Some(is_dashed) = if rem.starts_with("-.->") {
                Some(true)
            } else if rem.starts_with("-->") {
                Some(false)
            } else {
                None
            } {
                let arrow_len = if is_dashed { 4 } else { 3 };
                let token: String = chars[seg_start..i].iter().collect();
                if first_token.is_none() {
                    first_token = Some(token.trim().to_string());
                } else if let Some((lbl, d)) = pending_edge.take() {
                    parts.push((lbl, d, token.trim().to_string()));
                }

                let mut after = i + arrow_len;
                while after < chars.len() && chars[after].is_whitespace() {
                    after += 1;
                }
                let mut edge_lbl = None;
                if after < chars.len()
                    && chars[after] == '|'
                    && let Some(close_pipe) = chars[after + 1..].iter().position(|&x| x == '|')
                {
                    let raw_lbl: String = chars[after + 1..after + 1 + close_pipe].iter().collect();
                    edge_lbl = Some(format_mermaid_label(&raw_lbl));
                    after = after + 1 + close_pipe + 1;
                }
                pending_edge = Some((edge_lbl, is_dashed));
                seg_start = after;
                i = after;
                continue;
            }

            if rem.starts_with("-- ")
                && let Some(arrow_pos) = rem[3..].find("-->")
            {
                let raw_lbl = rem[3..3 + arrow_pos].trim();
                let token: String = chars[seg_start..i].iter().collect();
                if first_token.is_none() {
                    first_token = Some(token.trim().to_string());
                } else if let Some((lbl, d)) = pending_edge.take() {
                    parts.push((lbl, d, token.trim().to_string()));
                }
                let advance = 3 + arrow_pos + 3;
                let advance_chars = rem[..advance].chars().count();
                pending_edge = Some((Some(format_mermaid_label(raw_lbl)), false));
                seg_start = i + advance_chars;
                i = seg_start;
                continue;
            }
        }
        i += 1;
    }

    if let Some((lbl, d)) = pending_edge.take() {
        let tail: String = chars[seg_start..].iter().collect();
        if !tail.trim().is_empty() {
            parts.push((lbl, d, tail.trim().to_string()));
        }
    }

    match first_token {
        Some(f) if !parts.is_empty() => Some((f, parts)),
        _ => None,
    }
}

/// Renders a single Mermaid node box and optional outgoing edge pills.
fn render_mermaid_node_html<F: Fn(&str) -> String>(
    html: &mut String,
    node: &MermaidNode,
    edges: &[MermaidEdge],
    include_out_edges: bool,
    lookup_title: &F,
) {
    let shape_cls = if node.is_diamond {
        "mermaid-node mermaid-node-diamond"
    } else {
        "mermaid-node"
    };
    html.push_str("<div class=\"");
    html.push_str(shape_cls);
    html.push_str("\"><div class=\"mermaid-node-body\">");
    html.push_str(&node.label_html);
    html.push_str("</div>");

    if include_out_edges {
        let out_edges: Vec<&MermaidEdge> = edges.iter().filter(|e| e.from == node.id).collect();
        if !out_edges.is_empty() {
            html.push_str("<div class=\"mermaid-node-links\">");
            for edge in out_edges {
                let arrow_sym = if edge.dashed { "⇢" } else { "→" };
                html.push_str("<span class=\"mermaid-link-pill\">");
                if let Some(ref lbl) = edge.label {
                    html.push_str("<span class=\"mermaid-edge-label\">");
                    html.push_str(lbl);
                    html.push_str("</span> ");
                }
                html.push_str(arrow_sym);
                html.push(' ');
                html.push_str(&lookup_title(&edge.to));
                html.push_str("</span>");
            }
            html.push_str("</div>");
        }
    }
    html.push_str("</div>");
}

/// Renders a Mermaid `flowchart LR` or `flowchart TD` diagram block into a responsive,
/// dark-mode HTML/SVG visual flowchart diagram.
pub fn render_mermaid_to_html(diagram_src: &str) -> String {
    let mut is_lr = false;
    let mut nodes: Vec<MermaidNode> = Vec::new();
    let mut node_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut edges: Vec<MermaidEdge> = Vec::new();
    let mut subgraphs: Vec<MermaidSubgraph> = Vec::new();
    let mut active_subgraph: Option<usize> = None;

    let upsert_node = |id: String,
                       label_opt: Option<(String, bool)>,
                       active_sg: Option<usize>,
                       nodes: &mut Vec<MermaidNode>,
                       node_index: &mut std::collections::HashMap<String, usize>,
                       subgraphs: &mut Vec<MermaidSubgraph>| {
        if let Some(&existing_idx) = node_index.get(&id) {
            if let Some((lbl, is_d)) = label_opt {
                nodes[existing_idx].label_html = lbl;
                nodes[existing_idx].is_diamond = is_d;
            }
        } else {
            let (lbl, is_d) = label_opt.unwrap_or_else(|| (escape_html_text(&id), false));
            let new_idx = nodes.len();
            nodes.push(MermaidNode {
                id: id.clone(),
                label_html: lbl,
                is_diamond: is_d,
            });
            node_index.insert(id.clone(), new_idx);
        }
        if let Some(sg_idx) = active_sg
            && !subgraphs[sg_idx].node_ids.contains(&id)
        {
            subgraphs[sg_idx].node_ids.push(id);
        }
    };

    for raw_line in diagram_src.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        if let Some(dir) = trimmed
            .strip_prefix("flowchart ")
            .or_else(|| trimmed.strip_prefix("graph "))
        {
            is_lr = dir.trim().eq_ignore_ascii_case("LR") || dir.trim().eq_ignore_ascii_case("RL");
            continue;
        }
        if let Some(sg_rest) = trimmed.strip_prefix("subgraph ") {
            let sg_trim = sg_rest.trim();
            let (sg_id, sg_title) = if let Some(open_br) = sg_trim.find('[') {
                let id_part = sg_trim[..open_br].trim().to_string();
                let title_raw = sg_trim[open_br + 1..].trim_end_matches(']').trim();
                (id_part, format_mermaid_label(title_raw))
            } else {
                (sg_trim.to_string(), format_mermaid_label(sg_trim))
            };
            let sg_idx = subgraphs.len();
            subgraphs.push(MermaidSubgraph {
                id: sg_id,
                title: sg_title,
                node_ids: Vec::new(),
            });
            active_subgraph = Some(sg_idx);
            continue;
        }
        if trimmed == "end" {
            active_subgraph = None;
            continue;
        }

        if let Some((first_tok, chain)) = split_mermaid_edge_chain(trimmed) {
            let mut prev_id = if let Some((id, lbl_opt)) = parse_mermaid_node_token(&first_tok) {
                upsert_node(
                    id.clone(),
                    lbl_opt,
                    active_subgraph,
                    &mut nodes,
                    &mut node_index,
                    &mut subgraphs,
                );
                id
            } else {
                first_tok
            };

            for (edge_lbl, is_dashed, target_tok) in chain {
                let target_id = if let Some((id, lbl_opt)) = parse_mermaid_node_token(&target_tok) {
                    upsert_node(
                        id.clone(),
                        lbl_opt,
                        active_subgraph,
                        &mut nodes,
                        &mut node_index,
                        &mut subgraphs,
                    );
                    id
                } else {
                    target_tok
                };
                edges.push(MermaidEdge {
                    from: prev_id,
                    to: target_id.clone(),
                    label: edge_lbl,
                    dashed: is_dashed,
                });
                prev_id = target_id;
            }
            continue;
        }

        if let Some((id, lbl_opt)) = parse_mermaid_node_token(trimmed) {
            upsert_node(
                id,
                lbl_opt,
                active_subgraph,
                &mut nodes,
                &mut node_index,
                &mut subgraphs,
            );
        }
    }

    if nodes.is_empty() && subgraphs.is_empty() {
        return format!(
            "<pre class=\"mermaid rules-mermaid\">{}</pre>",
            escape_html_text(diagram_src)
        );
    }

    let dir_class = if is_lr {
        "mermaid-dir-lr"
    } else {
        "mermaid-dir-td"
    };
    let mut html = format!(
        "<div class=\"mermaid rules-mermaid {dir_class}\" role=\"figure\" aria-label=\"Flowchart Diagram\">"
    );

    let subgraph_ids: std::collections::HashSet<&str> =
        subgraphs.iter().map(|s| s.id.as_str()).collect();
    let lookup_title = |id: &str| -> String {
        if let Some(&idx) = node_index.get(id) {
            let raw = &nodes[idx].label_html;
            let first_line = raw.split("<br/>").next().unwrap_or(raw);
            first_line
                .replace("<strong>", "")
                .replace("</strong>", "")
                .replace("<em>", "")
                .replace("</em>", "")
        } else if let Some(sg) = subgraphs.iter().find(|s| s.id == id) {
            sg.title.clone()
        } else {
            escape_html_text(id)
        }
    };

    if !subgraphs.is_empty() {
        let in_any_sg: std::collections::HashSet<&str> = subgraphs
            .iter()
            .flat_map(|sg| sg.node_ids.iter().map(String::as_str))
            .collect();
        let top_nodes: Vec<&MermaidNode> = nodes
            .iter()
            .filter(|n| !in_any_sg.contains(n.id.as_str()) && !subgraph_ids.contains(n.id.as_str()))
            .collect();

        if !top_nodes.is_empty() {
            html.push_str("<div class=\"mermaid-top-nodes\">");
            for n in top_nodes {
                render_mermaid_node_html(&mut html, n, &edges, false, &lookup_title);
            }
            html.push_str("</div>");
        }

        html.push_str("<div class=\"mermaid-subgraphs-grid\">");
        for sg in &subgraphs {
            html.push_str(
                "<section class=\"mermaid-subgraph\"><div class=\"mermaid-subgraph-title\">",
            );
            html.push_str(&sg.title);
            html.push_str("</div><div class=\"mermaid-subgraph-nodes\">");
            for nid in &sg.node_ids {
                if let Some(&nidx) = node_index.get(nid) {
                    render_mermaid_node_html(&mut html, &nodes[nidx], &edges, true, &lookup_title);
                }
            }
            html.push_str("</div></section>");
        }
        html.push_str("</div>");
    } else {
        let is_simple_chain = edges.len() + 1 == nodes.len()
            && edges
                .iter()
                .enumerate()
                .all(|(idx, e)| e.from == nodes[idx].id && e.to == nodes[idx + 1].id);

        html.push_str("<div class=\"mermaid-flow-track\">");
        for (idx, n) in nodes.iter().enumerate() {
            render_mermaid_node_html(&mut html, n, &edges, !is_simple_chain, &lookup_title);
            if is_simple_chain && idx + 1 < nodes.len() {
                let edge = &edges[idx];
                let arrow_sym = if is_lr { "→" } else { "↓" };
                html.push_str("<div class=\"mermaid-connector\" aria-hidden=\"true\">");
                if let Some(ref lbl) = edge.label {
                    html.push_str("<span class=\"mermaid-edge-label\">");
                    html.push_str(lbl);
                    html.push_str("</span>");
                }
                html.push_str("<span class=\"mermaid-arrow-glyph\">");
                html.push_str(arrow_sym);
                html.push_str("</span></div>");
            }
        }
        html.push_str("</div>");
    }

    html.push_str("</div>");
    html
}
