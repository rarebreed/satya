//! Canonical Satya TTRPG Rules Book catalog, Markdown-to-HTML renderer, multi-chapter search,
//! and Combat Tactical Pool ("Envelope & Fill" — Tempo, Control, Force) calculator.

use satya_schema::{
    RulesBookCatalogResponse, RulesChapterDocument, RulesChapterSummary, RulesSearchHit,
};
use serde::{Deserialize, Serialize};

/// Static compile-time definition of a canonical chapter in `docs/ttrpg/`.
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
        description: "Canonical Two-List Cascading (2d20) dice mechanic, TN_eff = 31 - C + Delta_ext, carry modes, signed outcome tiers, 5-step Task Pipeline, Tool Tiers, Gate Classifications, Master Task Table, and Extended Project Clocks.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/02-core-mechanics/cascading_d20_and_tasks.md"
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
        title: "Skill System & Hierarchical Skill Trees",
        relative_path: "03-character/skills_and_trees.md",
        description: "Talents (TAL) vs. Physical (PHY), Knowledge (KNO), and Hybrid (PHY/KNO) skills, Multi-Category taxonomy, [Access] restriction tiers, and the 8 Hierarchical Skill Trees.",
        embedded_markdown: include_str!("../../../docs/ttrpg/03-character/skills_and_trees.md"),
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
        chapter_id: "ranged_combat_and_armor",
        part: "Part V: Combat Architecture, Close Quarters & Firefights",
        title: "Ranged Combat, Suppression, Armor & Weapon Reliability",
        relative_path: "05-combat/ranged_combat_and_armor.md",
        description: "Ranged fire modes & Recoil Ratings (R), Suppression vs. Morale, Armor Penetration (PR) vs. Behind-Armor Blunt Trauma (BABT), Weapon Reliability (Rel) & Malfunctions, and firefight walkthroughs.",
        embedded_markdown: include_str!(
            "../../../docs/ttrpg/05-combat/ranged_combat_and_armor.md"
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

/// Extracts top-level `##` and `###` headings from a Markdown document.
pub fn extract_markdown_headings(markdown: &str) -> Vec<String> {
    let mut headings = Vec::new();
    let mut in_code_block = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            headings.push(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            headings.push(rest.trim().to_string());
        }
    }
    headings
}

/// Escapes HTML special characters (`&`, `<`, `>`, `"`, `'`).
pub fn escape_html_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Formats inline Markdown syntax (inline code `` `...` ``, bold `**...**`, italic `*...*`, and links `[label](href)`).
pub fn format_inline_markdown(line: &str) -> String {
    let escaped = escape_html_text(line);
    let mut result = String::with_capacity(escaped.len() + 32);
    let chars: Vec<char> = escaped.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Inline code `...`
        if chars[i] == '`'
            && let Some(end_offset) = chars[i + 1..].iter().position(|&c| c == '`')
        {
            let code_end = i + 1 + end_offset;
            let code_text: String = chars[i + 1..code_end].iter().collect();
            result.push_str("<code class=\"rules-inline-code\">");
            result.push_str(&code_text);
            result.push_str("</code>");
            i = code_end + 1;
            continue;
        }
        // Bold **...**
        if i + 1 < chars.len()
            && chars[i] == '*'
            && chars[i + 1] == '*'
            && let Some(end_offset) = chars[i + 2..]
                .windows(2)
                .position(|w| w[0] == '*' && w[1] == '*')
        {
            let bold_end = i + 2 + end_offset;
            let inner: String = chars[i + 2..bold_end].iter().collect();
            result.push_str("<strong>");
            result.push_str(&format_inline_markdown(&inner));
            result.push_str("</strong>");
            i = bold_end + 2;
            continue;
        }
        // Markdown link [label](url)
        if chars[i] == '['
            && let Some(close_bracket) = chars[i + 1..].iter().position(|&c| c == ']')
        {
            let bracket_idx = i + 1 + close_bracket;
            if bracket_idx + 1 < chars.len()
                && chars[bracket_idx + 1] == '('
                && let Some(close_paren) = chars[bracket_idx + 2..].iter().position(|&c| c == ')')
            {
                let paren_idx = bracket_idx + 2 + close_paren;
                let label: String = chars[i + 1..bracket_idx].iter().collect();
                let url: String = chars[bracket_idx + 2..paren_idx].iter().collect();
                result.push_str("<a class=\"rules-link\" href=\"");
                result.push_str(&url);
                result.push_str("\">");
                result.push_str(&label);
                result.push_str("</a>");
                i = paren_idx + 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Converts a heading title into a URL-safe anchor slug.
pub fn slugify_heading(title: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }
    slug.trim_end_matches('-').to_string()
}

/// Renders a canonical Satya TTRPG Markdown chapter into structured, safe HTML suitable for
/// the `satya-web` Rules Book pane and static book export.
pub fn render_rules_markdown_to_html(markdown: &str) -> String {
    let mut html = String::with_capacity(markdown.len() * 3 / 2);
    html.push_str("<article class=\"satya-rules-article\">");

    let lines: Vec<&str> = markdown.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" {
            html.push_str("<hr class=\"rules-divider\"/>");
            i += 1;
            continue;
        }

        // Fenced code block ```
        if let Some(lang) = trimmed.strip_prefix("```") {
            let lang_clean = lang.trim();
            i += 1;
            let mut block_lines = Vec::new();
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                block_lines.push(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1; // skip closing ```
            }
            let block_content = block_lines.join("\n");
            if lang_clean.eq_ignore_ascii_case("mermaid") {
                html.push_str("<pre class=\"mermaid rules-mermaid\">");
                html.push_str(&escape_html_text(&block_content));
                html.push_str("</pre>");
            } else {
                html.push_str("<pre class=\"rules-code-block\"><code data-lang=\"");
                html.push_str(&escape_html_text(lang_clean));
                html.push_str("\">");
                html.push_str(&escape_html_text(&block_content));
                html.push_str("</code></pre>");
            }
            continue;
        }

        // Display math block $$...$$
        if trimmed.starts_with("$$") {
            let mut math_lines = vec![trimmed];
            if !trimmed.ends_with("$$") || trimmed.len() == 2 {
                i += 1;
                while i < lines.len() {
                    math_lines.push(lines[i].trim());
                    if lines[i].trim().ends_with("$$") {
                        break;
                    }
                    i += 1;
                }
            }
            i += 1;
            html.push_str("<div class=\"rules-math-block\">");
            html.push_str(&escape_html_text(&math_lines.join("\n")));
            html.push_str("</div>");
            continue;
        }

        // Headings # .. ####
        if let Some(h4) = trimmed.strip_prefix("#### ") {
            let slug = slugify_heading(h4);
            html.push_str("<h4 id=\"");
            html.push_str(&slug);
            html.push_str("\">");
            html.push_str(&format_inline_markdown(h4));
            html.push_str("</h4>");
            i += 1;
            continue;
        }
        if let Some(h3) = trimmed.strip_prefix("### ") {
            let slug = slugify_heading(h3);
            html.push_str("<h3 id=\"");
            html.push_str(&slug);
            html.push_str("\">");
            html.push_str(&format_inline_markdown(h3));
            html.push_str("</h3>");
            i += 1;
            continue;
        }
        if let Some(h2) = trimmed.strip_prefix("## ") {
            let slug = slugify_heading(h2);
            html.push_str("<h2 id=\"");
            html.push_str(&slug);
            html.push_str("\">");
            html.push_str(&format_inline_markdown(h2));
            html.push_str("</h2>");
            i += 1;
            continue;
        }
        if let Some(h1) = trimmed.strip_prefix("# ") {
            let slug = slugify_heading(h1);
            html.push_str("<h1 id=\"");
            html.push_str(&slug);
            html.push_str("\">");
            html.push_str(&format_inline_markdown(h1));
            html.push_str("</h1>");
            i += 1;
            continue;
        }

        // Blockquote / GitHub Alert >
        if trimmed.starts_with('>') {
            let mut quote_lines = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('>') {
                let stripped = lines[i].trim().strip_prefix('>').unwrap_or("").trim_start();
                quote_lines.push(stripped);
                i += 1;
            }
            let first = quote_lines.first().copied().unwrap_or("");
            let (alert_class, alert_label, start_idx) = if first.starts_with("[!NOTE]") {
                ("rules-alert rules-alert-note", Some("NOTE"), 1)
            } else if first.starts_with("[!TIP]") {
                ("rules-alert rules-alert-tip", Some("TIP"), 1)
            } else if first.starts_with("[!IMPORTANT]") {
                ("rules-alert rules-alert-important", Some("IMPORTANT"), 1)
            } else if first.starts_with("[!WARNING]") {
                ("rules-alert rules-alert-warning", Some("WARNING"), 1)
            } else if first.starts_with("[!CAUTION]") {
                ("rules-alert rules-alert-caution", Some("CAUTION"), 1)
            } else {
                ("rules-blockquote", None, 0)
            };

            html.push_str("<blockquote class=\"");
            html.push_str(alert_class);
            html.push_str("\">");
            if let Some(label) = alert_label {
                html.push_str("<div class=\"rules-alert-title\">");
                html.push_str(label);
                html.push_str("</div>");
            }
            for qline in &quote_lines[start_idx..] {
                if !qline.is_empty() {
                    html.push_str("<p>");
                    html.push_str(&format_inline_markdown(qline));
                    html.push_str("</p>");
                }
            }
            html.push_str("</blockquote>");
            continue;
        }

        // GFM Table | ... |
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let mut table_rows = Vec::new();
            while i < lines.len() {
                let row_trim = lines[i].trim();
                if row_trim.starts_with('|') && row_trim.ends_with('|') {
                    table_rows.push(row_trim);
                    i += 1;
                } else {
                    break;
                }
            }
            html.push_str("<div class=\"rules-table-wrap\"><table class=\"rules-table\">");
            for (row_idx, row_str) in table_rows.iter().enumerate() {
                // Skip separator row like | :--- | :---: |
                let inner = row_str.trim_matches('|');
                let cells: Vec<&str> = inner.split('|').map(str::trim).collect();
                let is_sep = cells.iter().all(|c| {
                    !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
                });
                if is_sep {
                    continue;
                }
                if row_idx == 0 {
                    html.push_str("<thead><tr>");
                    for cell in cells {
                        html.push_str("<th>");
                        html.push_str(&format_inline_markdown(cell));
                        html.push_str("</th>");
                    }
                    html.push_str("</tr></thead><tbody>");
                } else {
                    html.push_str("<tr>");
                    for cell in cells {
                        html.push_str("<td>");
                        html.push_str(&format_inline_markdown(cell));
                        html.push_str("</td>");
                    }
                    html.push_str("</tr>");
                }
            }
            html.push_str("</tbody></table></div>");
            continue;
        }

        // Unordered list - or *
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            html.push_str("<ul class=\"rules-list\">");
            while i < lines.len() {
                let item_trim = lines[i].trim();
                if let Some(rest) = item_trim
                    .strip_prefix("- ")
                    .or_else(|| item_trim.strip_prefix("* "))
                {
                    html.push_str("<li>");
                    html.push_str(&format_inline_markdown(rest));
                    html.push_str("</li>");
                    i += 1;
                } else {
                    break;
                }
            }
            html.push_str("</ul>");
            continue;
        }

        // Ordered list 1. 2.
        let is_ordered = trimmed.split_once(". ").is_some_and(|(prefix, _)| {
            !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit())
        });
        if is_ordered {
            html.push_str("<ol class=\"rules-list rules-list-ordered\">");
            while i < lines.len() {
                let item_trim = lines[i].trim();
                if let Some((prefix, rest)) = item_trim.split_once(". ")
                    && !prefix.is_empty()
                    && prefix.chars().all(|c| c.is_ascii_digit())
                {
                    html.push_str("<li>");
                    html.push_str(&format_inline_markdown(rest));
                    html.push_str("</li>");
                    i += 1;
                } else {
                    break;
                }
            }
            html.push_str("</ol>");
            continue;
        }

        // Standard paragraph
        let mut para_lines = Vec::new();
        while i < lines.len() {
            let p_trim = lines[i].trim();
            if p_trim.is_empty()
                || p_trim.starts_with('#')
                || p_trim.starts_with("```")
                || p_trim.starts_with("$$")
                || p_trim.starts_with('>')
                || (p_trim.starts_with('|') && p_trim.ends_with('|'))
                || p_trim.starts_with("- ")
                || p_trim.starts_with("* ")
                || p_trim == "---"
            {
                break;
            }
            para_lines.push(p_trim);
            i += 1;
        }
        if !para_lines.is_empty() {
            html.push_str("<p>");
            html.push_str(&format_inline_markdown(&para_lines.join(" ")));
            html.push_str("</p>");
        }
    }

    html.push_str("</article>");
    html
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
        // Split chapter by `## ` sections
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

// ============================================================================
// Combat Tactical Pool ("Envelope & Fill" — Tempo, Control, Force) Calculator
// ============================================================================

/// Psychological & Morale condition governing a combatant's Tactical Pool size and allocation constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MoraleState {
    /// Eager and ready to attack (`+2` Pool; `+1` to Force or Tempo ceiling).
    High,
    /// Default composed condition (`0` Pool modifier; full allocation freedom up to ceilings).
    #[default]
    Steady,
    /// Adrenaline bloodlust surge (`+2` Pool; must allocate `Force >= P_eff / 2`, and `Control <= P_eff / 4`).
    Rage,
    /// Doubt creeping in (`-2` Pool; `Force` cannot exceed `Control`, `F <= C`).
    Wavering,
    /// Disengaging / withdrawing (`-4` Pool; active offensive `Force == 0`).
    Broken,
    /// Full-blown rout (`-4` Pool; `Control == 0` and `Force == 0`, `100%` locked into `Tempo`).
    Panic,
}

impl MoraleState {
    /// Parses a morale condition identifier string (`"high"`, `"steady"`, `"rage"`, `"wavering"`, `"broken"`, `"panic"`).
    pub fn from_str_name(name: &str) -> Result<Self, String> {
        match name.trim().to_ascii_lowercase().as_str() {
            "high" | "conviction" | "conviction_surge" => Ok(Self::High),
            "steady" | "normal" => Ok(Self::Steady),
            "rage" | "bloodlust" => Ok(Self::Rage),
            "wavering" | "shaken" => Ok(Self::Wavering),
            "broken" | "withdrawing" => Ok(Self::Broken),
            "panic" | "routed" => Ok(Self::Panic),
            other => Err(format!(
                "unknown MoraleState '{other}'; expected high, steady, rage, wavering, broken, or panic"
            )),
        }
    }

    /// Returns the canonical snake_case string representation of this morale condition.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Steady => "steady",
            Self::Rage => "rage",
            Self::Wavering => "wavering",
            Self::Broken => "broken",
            Self::Panic => "panic",
        }
    }

    /// Returns the Tactical Pool modifier ($\Delta P_{\text{morale}}$) for this morale state.
    pub fn pool_modifier(self) -> i32 {
        match self {
            Self::High | Self::Rage => 2,
            Self::Steady => 0,
            Self::Wavering => -2,
            Self::Broken | Self::Panic => -4,
        }
    }
}

/// Character attributes, skill level, and physical/psychological state inputs for the
/// Satya Combat Tactical Pool ("Envelope & Fill") calculator (`docs/ttrpg/combat/combat.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalPoolParams {
    /// Physical Speed (`SPD`, `1..=20`).
    pub spd: i32,
    /// Physical Force (`FOR`, `1..=20`).
    pub for_stat: i32,
    /// Physical Dexterity (`DEX`, `1..=20`).
    pub dex: i32,
    /// Physical Kinesthesis (`KIN`, `1..=20`).
    pub kin: i32,
    /// Sensory Sight (`SIG`, `1..=20`).
    pub sig: i32,
    /// Mental Insight (`INS`, `1..=20`).
    pub ins: i32,
    /// Sensory Intuition (`ITU`, `1..=20`).
    pub itu: i32,
    /// Mental Analysis (`ANA`, `1..=20`).
    pub ana: i32,
    /// Mental Creativity (`CRE`, `1..=20`).
    pub cre: i32,
    /// Mental Focus (`FOC`, `1..=20`).
    pub foc: i32,
    /// Mental Discipline (`DIS`, `1..=20`).
    pub dis: i32,
    /// Physical Fitness (`FIT`, `1..=20`).
    pub fit: i32,
    /// Character body mass in kilograms (default `70.0 kg`).
    pub mass_kg: f64,
    /// Controlling Characteristic $C_{\text{skill}}$ of the active combat skill (`1..=20`).
    pub skill_c: i32,
    /// Active Combat Skill Level $N \in [1, 10]$.
    pub skill_n: usize,
    /// Whether the action is a ranged weapon exchange (`true`) or melee/unarmed exchange (`false`).
    pub is_ranged: bool,
    /// Accumulated systemic Fatigue Level (`-2` pool per level, `+10%` `action_time_factor` per level).
    pub fatigue_level: i32,
    /// Accumulated systemic Wound Shock Tier (`-2` pool per tier, `+25%` `action_time_factor` per tier).
    pub wound_shock_index: i32,
    /// Localized structural wound penalty to Leg/Hip/Foot (slashes `Tempo Cap`).
    pub leg_wound_penalty: i32,
    /// Localized structural wound penalty to Arm/Wrist/Head/Eye (slashes `Control Cap`).
    pub arm_head_wound_penalty: i32,
    /// Localized structural wound penalty to Arm/Shoulder/Torso (slashes `Force Cap`).
    pub torso_shoulder_wound_penalty: i32,
    /// Active psychological Morale Condition.
    pub morale_state: MoraleState,
}

impl Default for TacticalPoolParams {
    fn default() -> Self {
        Self {
            spd: 10,
            for_stat: 10,
            dex: 10,
            kin: 10,
            sig: 10,
            ins: 10,
            itu: 10,
            ana: 10,
            cre: 10,
            foc: 10,
            dis: 10,
            fit: 10,
            mass_kg: 70.0,
            skill_c: 10,
            skill_n: 3,
            is_ranged: false,
            fatigue_level: 0,
            wound_shock_index: 0,
            leg_wound_penalty: 0,
            arm_head_wound_penalty: 0,
            torso_shoulder_wound_penalty: 0,
            morale_state: MoraleState::Steady,
        }
    }
}

/// Computed Tactical Pool ("Fill") and Characteristic Aspect Ceilings ("Envelope") for a combatant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalPoolEnvelope {
    /// Cognitive Decision Score ("Quick Thinking"): $D_{\text{cog}} = \lfloor(\max(\text{ANA}, \text{CRE}) + \text{FOC}) / 2\rfloor$.
    pub d_cog: i32,
    /// 4-Pillar OODA Initiative rating: $\lfloor(\text{ITU} + \text{INS} + D_{\text{cog}} + \text{SPD}) / 4\rfloor$.
    pub ooda_init: i32,
    /// Combat Readiness controller: $C_{\text{combat}} = \lfloor(C_{\text{skill}} + \text{OODA}) / 2\rfloor$.
    pub combat_readiness_c: i32,
    /// Base Tactical Pool before fatigue, shock, or morale: $P_{\text{base}} = C_{\text{combat}} + 3N$.
    pub p_base: i32,
    /// Effective Tactical Pool available to allocate: $P_{\text{eff}} = \max(0, P_{\text{base}} - 2\cdot\text{Fatigue} - 2\cdot\text{Shock} + \Delta P_{\text{morale}})$.
    pub p_eff: i32,
    /// Neutral per-aspect baseline: $B = \lfloor P_{\text{base}} / 3 \rfloor$.
    pub neutral_baseline_b: i32,
    /// Maximum points allocatable to **Tempo ($T_{\max}$)**.
    pub tempo_cap: i32,
    /// Maximum points allocatable to **Control ($C_{\max}$)**.
    pub control_cap: i32,
    /// Maximum points allocatable to **Force ($F_{\max}$)**.
    pub force_cap: i32,
    /// Mastery technique overflow bonus ($\lfloor(N - 3)/2\rfloor$ for $N \ge 5$) added to Control & Force caps.
    pub mastery_overflow: i32,
    /// Derived kinetic Power: $\text{POW} = (\text{FOR} \times \text{SPD}) / 10.0$.
    pub power_pow: f64,
    /// Derived Momentum & Mass Factor: $\text{MMF} = (\text{MAS} / 70.0) \times \text{SPD}$.
    pub momentum_mass_factor_mmf: f64,
    /// Derived Fatigue Threshold: $\text{FT} = \text{FIT} + \lfloor\text{DIS} / 2\rfloor$.
    pub fatigue_threshold_ft: i32,
    /// Action Time Factor multiplier: $1.0 + 0.10\cdot\text{Fatigue} + 0.25\cdot\text{Shock}$.
    pub action_time_factor: f64,
    /// Active psychological Morale Condition.
    pub morale_state: MoraleState,
    /// Minimum Force allocation required by Morale/Rage ($\lfloor P_{\text{eff}} / 2 \rfloor$ in `Rage`, else `0`).
    pub min_force_required: i32,
    /// Maximum Control allocation permitted by Morale/Rage + Ceiling.
    pub max_control_allowed: i32,
    /// Maximum Force allocation permitted by Morale/Rage + Ceiling.
    pub max_force_allowed: i32,
    /// Whether `Force` is constrained to not exceed `Control` ($F \le C$, active in `Wavering` morale).
    pub force_must_not_exceed_control: bool,
}

/// Evaluation of a specific `Tempo + Control + Force` Tactical Triad allocation against a [`TacticalPoolEnvelope`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalTriadEvaluation {
    /// Points allocated to **Tempo (`T`)**.
    pub tempo: i32,
    /// Points allocated to **Control (`C`)**.
    pub control: i32,
    /// Points allocated to **Force (`F`)**.
    pub force: i32,
    /// Whether the allocation satisfies all Pool, Ceiling, and Morale/Rage constraints.
    pub valid: bool,
    /// Human-readable descriptions of any violated constraints (empty when `valid == true`).
    pub violations: Vec<String>,
    /// Effective Count-Up Action Tick Cost (`TC`) after Tempo and `action_time_factor`.
    pub effective_tc: i32,
    /// Scaled melee Strike Impact ($\text{POW} \times (F / B)$).
    pub strike_impact: f64,
    /// Effective Rooted stability against throws, sweeps, and knockback ($F + \text{MMF}$).
    pub effective_root: f64,
    /// Exertion marks accumulated by this action (`0`, `1`, or `2`).
    pub exertion_marks_gained: i32,
    /// Canonical Tactical Archetype classification (`"TC (Probe / Counter)"`, `"TF (Blitz / Dive)"`, `"CF (Finisher / Anchor)"`, `"Pure T"`, `"Pure C"`, `"Pure F"`, or `"Balanced"`).
    pub archetype_label: String,
}

/// Computes the [`TacticalPoolEnvelope`] ("Envelope & Fill") for a combatant.
pub fn compute_tactical_pool(params: &TacticalPoolParams) -> TacticalPoolEnvelope {
    let d_cog = (params.ana.max(params.cre) + params.foc).div_euclid(2);
    let ooda_init = (params.itu + params.ins + d_cog + params.spd).div_euclid(4);
    let combat_readiness_c = (params.skill_c + ooda_init).div_euclid(2);
    let n_i32 = params.skill_n as i32;
    let p_base = combat_readiness_c + (3 * n_i32);
    let neutral_baseline_b = p_base.div_euclid(3).max(1);

    let morale_mod = params.morale_state.pool_modifier();
    let p_eff =
        (p_base - (2 * params.fatigue_level.max(0)) - (2 * params.wound_shock_index.max(0))
            + morale_mod)
            .max(0);

    let mastery_overflow = if n_i32 >= 5 {
        (n_i32 - 3).div_euclid(2)
    } else {
        0
    };
    let high_morale_bonus = i32::from(params.morale_state == MoraleState::High);

    let tempo_cap = (params.spd - params.leg_wound_penalty.max(0) + high_morale_bonus).max(0);
    let base_control_attr = if params.is_ranged {
        (params.dex + params.sig).div_euclid(2)
    } else {
        params.kin.max(params.dex)
    };
    let control_cap =
        (base_control_attr + mastery_overflow - params.arm_head_wound_penalty.max(0)).max(0);
    let force_cap = (params.for_stat + mastery_overflow + high_morale_bonus
        - params.torso_shoulder_wound_penalty.max(0))
    .max(0);

    let power_pow = (params.for_stat as f64 * params.spd as f64) / 10.0;
    let momentum_mass_factor_mmf = (params.mass_kg / 70.0) * (params.spd as f64);
    let fatigue_threshold_ft = params.fit + params.dis.div_euclid(2);
    let action_time_factor = 1.0
        + (0.10 * params.fatigue_level.max(0) as f64)
        + (0.25 * params.wound_shock_index.max(0) as f64);

    let (min_force_required, max_control_allowed, max_force_allowed, force_must_not_exceed_control) =
        match params.morale_state {
            MoraleState::High | MoraleState::Steady => (0, control_cap, force_cap, false),
            MoraleState::Rage => (
                p_eff.div_euclid(2),
                control_cap.min(p_eff.div_euclid(4)),
                force_cap,
                false,
            ),
            MoraleState::Wavering => (0, control_cap, force_cap, true),
            MoraleState::Broken => (0, control_cap, 0, false),
            MoraleState::Panic => (0, 0, 0, false),
        };

    TacticalPoolEnvelope {
        d_cog,
        ooda_init,
        combat_readiness_c,
        p_base,
        p_eff,
        neutral_baseline_b,
        tempo_cap,
        control_cap,
        force_cap,
        mastery_overflow,
        power_pow,
        momentum_mass_factor_mmf,
        fatigue_threshold_ft,
        action_time_factor,
        morale_state: params.morale_state,
        min_force_required,
        max_control_allowed,
        max_force_allowed,
        force_must_not_exceed_control,
    }
}

/// Evaluates a specific `(tempo, control, force)` allocation against a [`TacticalPoolEnvelope`]
/// and computes the resulting Count-Up Tick Cost (`TC`), Strike Impact, Root, Exertion Marks, and Archetype.
pub fn evaluate_tactical_triad(
    envelope: &TacticalPoolEnvelope,
    base_weapon_tc: i32,
    tempo: i32,
    control: i32,
    force: i32,
) -> TacticalTriadEvaluation {
    let mut violations = Vec::new();

    if tempo < 0 || control < 0 || force < 0 {
        violations
            .push("Aspect allocations (Tempo, Control, Force) cannot be negative.".to_string());
    }
    let sum = tempo + control + force;
    if sum > envelope.p_eff {
        violations.push(format!(
            "Total allocated points ({sum}) exceed Effective Tactical Pool P_eff ({}).",
            envelope.p_eff
        ));
    }
    if tempo > envelope.tempo_cap && envelope.morale_state != MoraleState::Panic {
        violations.push(format!(
            "Tempo ({tempo}) exceeds Tempo Ceiling T_max ({}).",
            envelope.tempo_cap
        ));
    }
    if control > envelope.max_control_allowed {
        violations.push(format!(
            "Control ({control}) exceeds maximum allowed Control ({}) under {:?} morale.",
            envelope.max_control_allowed, envelope.morale_state
        ));
    }
    if force > envelope.max_force_allowed {
        violations.push(format!(
            "Force ({force}) exceeds maximum allowed Force ({}) under {:?} morale.",
            envelope.max_force_allowed, envelope.morale_state
        ));
    }
    if force < envelope.min_force_required {
        violations.push(format!(
            "Force ({force}) is below minimum required Force ({}) under {:?} morale.",
            envelope.min_force_required, envelope.morale_state
        ));
    }
    if envelope.force_must_not_exceed_control && force > control {
        violations.push(format!(
            "Under Wavering morale, Force ({force}) cannot exceed Control ({control})."
        ));
    }

    let scaled_base_tc = (base_weapon_tc as f64 * envelope.action_time_factor).round() as i32;
    let effective_tc = (scaled_base_tc - (tempo - envelope.neutral_baseline_b)).max(2);

    let strike_impact = envelope.power_pow * (force as f64 / envelope.neutral_baseline_b as f64);
    let effective_root = force as f64 + envelope.momentum_mass_factor_mmf;

    let mut exertion_marks_gained = 0;
    if envelope.tempo_cap > 0 && tempo * 2 > envelope.tempo_cap {
        exertion_marks_gained += 1;
    }
    if envelope.force_cap > 0 && force * 2 > envelope.force_cap {
        exertion_marks_gained += 1;
    }

    let total_pos = sum.max(1) as f64;
    let t_share = tempo as f64 / total_pos;
    let c_share = control as f64 / total_pos;
    let f_share = force as f64 / total_pos;

    let archetype_label = if t_share >= 0.60 {
        "Pure T — All-Out Tempo (Sprint / Snap / Evade)".to_string()
    } else if c_share >= 0.60 {
        "Pure C — All-Out Control (Turtle Guard / Counter-Trap)".to_string()
    } else if f_share >= 0.60 {
        "Pure F — All-Out Force (Haymaker / Mag-Dump)".to_string()
    } else if t_share >= 0.35 && c_share >= 0.35 && f_share < 0.25 {
        "TC — Tempo + Control (The Probe / Counter)".to_string()
    } else if t_share >= 0.35 && f_share >= 0.35 && c_share < 0.25 {
        "TF — Tempo + Force (The Blitz / Dive)".to_string()
    } else if c_share >= 0.35 && f_share >= 0.35 && t_share < 0.25 {
        "CF — Control + Force (The Finisher / Anchor)".to_string()
    } else {
        "Balanced Triad Allocation (T / C / F)".to_string()
    };

    TacticalTriadEvaluation {
        tempo,
        control,
        force,
        valid: violations.is_empty(),
        violations,
        effective_tc,
        strike_impact,
        effective_root,
        exertion_marks_gained,
        archetype_label,
    }
}

// ============================================================================
// Interactive Rules Book Simulator Helpers (Tactical Pool, Triad, Lifepath, Social)
// ============================================================================

/// Summary of the Combat Tactical Pool ("Envelope & Fill") interactive lab calculation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalPoolEnvelopeSummary {
    /// Maximum capacity of the tactical envelope (`E_base`).
    pub base_envelope: i32,
    /// Uncapped tactical pool generated from cascade successes, skill, and reach (`P_raw`).
    pub raw_pool: i32,
    /// Effective pool available after clamping to `base_envelope` (`P_eff`).
    pub effective_pool: i32,
    /// Excess points above `base_envelope` lost to envelope overflow.
    pub overflow_wasted: i32,
    /// Net weapon reach modifier (`-2..=+3`).
    pub reach_modifier: i32,
    /// Maximum Tempo allocation cap from [`TacticalPoolEnvelope`].
    pub max_tempo: i32,
    /// Maximum Control allocation cap from [`TacticalPoolEnvelope`].
    pub max_control: i32,
    /// Maximum Force allocation cap from [`TacticalPoolEnvelope`].
    pub max_force: i32,
}

/// Computes the Satya Combat Tactical Pool ("Envelope & Fill") interactive lab summary.
pub fn compute_tactical_pool_envelope(
    control_c: i32,
    skill_level: i32,
    delta_int: i32,
    delta_ext: i32,
    num_successes: i32,
    weapon_reach: i32,
    opponent_reach: i32,
) -> TacticalPoolEnvelopeSummary {
    let c_clamped = control_c.clamp(1, 20);
    let n_clamped = skill_level.clamp(0, 10);
    let s_clamped = num_successes.max(0);

    let env = compute_tactical_pool(&TacticalPoolParams {
        skill_c: c_clamped,
        skill_n: n_clamped as usize,
        fatigue_level: (-delta_int).max(0),
        ..Default::default()
    });

    let base_envelope = (((c_clamped + 2 * n_clamped).div_euclid(3))
        + delta_int
        - delta_ext.div_euclid(2).max(0))
    .max(1);
    let reach_delta = weapon_reach - opponent_reach;
    let reach_modifier = reach_delta.clamp(-2, 3);
    let raw_pool = (s_clamped * 3 + n_clamped.div_euclid(2).max(0) + reach_modifier).max(0);
    let effective_pool = raw_pool.min(base_envelope);
    let overflow_wasted = (raw_pool - base_envelope).max(0);

    TacticalPoolEnvelopeSummary {
        base_envelope,
        raw_pool,
        effective_pool,
        overflow_wasted,
        reach_modifier,
        max_tempo: env.tempo_cap,
        max_control: env.control_cap,
        max_force: env.force_cap,
    }
}

/// Outcome of a Close-Quarters Tactical Triad (`Tempo`, `Control`, `Force`) contested exchange.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalTriadContestOutcome {
    /// Net Tempo differential (`attacker_tempo - defender_tempo`).
    pub net_tempo: i32,
    /// Net Control differential (`attacker_control - defender_control`).
    pub net_control: i32,
    /// Whether the attacker seizes the initiative (`net_tempo >= 2 && net_control >= 0`).
    pub seizes_initiative: bool,
    /// Whether the defender's guard is broken.
    pub guard_broken: bool,
    /// Whether a Kuzushi throw/sweep is triggered against `attacker_root`.
    pub throw_triggered: bool,
    /// Net penetrating force after armor DR and guard mitigation.
    pub penetrating_force: i32,
    /// Count-Up tick cost of the attacker's next exchange.
    pub next_tick_cost: i32,
    /// Canonical Tactical Archetype classification of the attacker's triad.
    pub archetype_label: String,
    /// Human-readable summary of the exchange outcome.
    pub outcome_summary: String,
}

/// Evaluates a Close-Quarters Tactical Triad (`Tempo`, `Control`, `Force`) exchange from Chapter 8.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_tactical_triad_contest(
    attacker_tempo: i32,
    attacker_control: i32,
    attacker_force: i32,
    defender_tempo: i32,
    defender_control: i32,
    defender_force: i32,
    attacker_root: i32,
    defender_guard: i32,
    armor_dr: i32,
) -> TacticalTriadContestOutcome {
    let env = compute_tactical_pool(&TacticalPoolParams {
        skill_c: 14,
        skill_n: 5,
        ..Default::default()
    });
    let eval_res =
        evaluate_tactical_triad(&env, 10, attacker_tempo, attacker_control, attacker_force);

    let atk_t = attacker_tempo.max(0);
    let atk_c = attacker_control.max(0);
    let atk_f = attacker_force.max(0);
    let def_t = defender_tempo.max(0);
    let def_c = defender_control.max(0);
    let def_f = defender_force.max(0);

    let net_tempo = atk_t - def_t;
    let net_control = atk_c - def_c;
    let seizes_initiative = net_tempo >= 2 && net_control >= 0;
    let guard_broken = (atk_c + (atk_f / 2).max(0)) > (def_c + defender_guard.max(0));
    let unbalance = ((atk_c + atk_f) - (def_c + def_f)).max(0);
    let throw_triggered = unbalance >= (attacker_root / 3).max(1);
    let raw_impact = atk_f + if guard_broken { net_control.max(0) } else { 0 };
    let penetrating_force =
        (raw_impact - armor_dr.max(0) - if guard_broken { 0 } else { def_c }).max(0);
    let next_tick_cost = (10 - atk_t.min(5) + (def_c / 2).max(0)).max(3);

    let outcome_summary = if throw_triggered && penetrating_force >= 3 {
        "Decisive Takedown & Armor Breach (Prone + Shock)".to_string()
    } else if throw_triggered {
        "Kuzushi Throw / Sweep (Target Off-Balance / Prone)".to_string()
    } else if guard_broken && penetrating_force > 0 {
        "Guard Broken & Penetrating Strike".to_string()
    } else if seizes_initiative {
        "Initiative Seized (Tempo Advantage on Next Exchange)".to_string()
    } else if penetrating_force > 0 {
        "Glancing Penetrating Blow".to_string()
    } else {
        "Deflected on Guard / Bind Equilibrium".to_string()
    };

    TacticalTriadContestOutcome {
        net_tempo,
        net_control,
        seizes_initiative,
        guard_broken,
        throw_triggered,
        penetrating_force,
        next_tick_cost,
        archetype_label: eval_res.archetype_label,
        outcome_summary,
    }
}

/// Summary of Lifepath exponential skill progression (`N in 0..=10`) from study hours.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifepathSkillProgression {
    /// Unmodified study hours invested.
    pub raw_hours: f64,
    /// Effective hours after instruction quality and aptitude multipliers.
    pub effective_hours: f64,
    /// Achieved skill rank $N \in [0, 10]$.
    pub skill_level_n: i32,
    /// Next skill rank target $\min(10, N + 1)$.
    pub next_level_n: i32,
    /// Remaining effective hours needed to reach `next_level_n`.
    pub hours_to_next_level: f64,
    /// Cumulative effective hour thresholds for each rank `1..=10`.
    pub cumulative_thresholds: std::collections::BTreeMap<i32, f64>,
}

fn round_1_dec(val: f64) -> f64 {
    (val * 10.0).round() / 10.0
}

/// Computes non-linear skill level $N \in [0, 10]$ from study hours using Chapter 6's exponential curve.
///
/// Each rank $N \ge 1$ requires $100 \times 2^{N - 1} \times \text{difficulty\_multiplier}$ effective hours.
pub fn compute_lifepath_skill_progression(
    hours_studied: f64,
    difficulty_multiplier: f64,
    instruction_quality: f64,
    aptitude_bonus: i32,
) -> LifepathSkillProgression {
    let eff_hours = hours_studied.max(0.0)
        * instruction_quality.max(0.25)
        * (1.0 + 0.08 * (aptitude_bonus as f64));
    let diff = difficulty_multiplier.max(0.5);
    let mut cumulative_thresholds = std::collections::BTreeMap::new();
    let mut cumulative = 0.0;
    let mut achieved_level = 0;

    for lvl in 1..=10 {
        let step_req = 100.0 * (2.0_f64.powi(lvl - 1)) * diff;
        cumulative += step_req;
        let rounded_cum = round_1_dec(cumulative);
        cumulative_thresholds.insert(lvl, rounded_cum);
        if eff_hours >= cumulative {
            achieved_level = lvl;
        }
    }

    let next_level = (achieved_level + 1).min(10);
    let next_threshold = if achieved_level < 10 {
        cumulative_thresholds
            .get(&next_level)
            .copied()
            .unwrap_or(0.0)
    } else {
        cumulative_thresholds.get(&10).copied().unwrap_or(0.0)
    };
    let hours_to_next = round_1_dec((next_threshold - eff_hours).max(0.0));

    LifepathSkillProgression {
        raw_hours: round_1_dec(hours_studied),
        effective_hours: round_1_dec(eff_hours),
        skill_level_n: achieved_level,
        next_level_n: next_level,
        hours_to_next_level: hours_to_next,
        cumulative_thresholds,
    }
}

/// Outcome of a Social Combat / Cognitive Dissonance check from Chapter 9.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialDissonanceOutcome {
    /// Total cognitive dissonance pressure generated by argument margin, evidence, and audience.
    pub dissonance_pressure: i32,
    /// Target's psychological resistance threshold derived from core conviction.
    pub resistance_threshold: i32,
    /// Net shift (`dissonance_pressure - resistance_threshold`).
    pub net_dissonance_shift: i32,
    /// Canonical narrative social outcome tier.
    pub social_outcome: String,
}

/// Computes Cognitive Dissonance pressure and belief-shift outcome from Chapter 9 (`psyche_morale_and_social_combat.md`).
pub fn compute_social_dissonance_check(
    argument_margin: i32,
    target_conviction: i32,
    evidence_tier: i32,
    shared_values_bonus: i32,
    public_audience_pressure: i32,
) -> SocialDissonanceOutcome {
    let conviction = target_conviction.clamp(1, 20);
    let evidence = evidence_tier.clamp(0, 5);
    let shared = shared_values_bonus.clamp(-3, 5);
    let audience = public_audience_pressure.clamp(-3, 5);

    let dissonance_pressure = (argument_margin * 2 + evidence * 3 + shared * 2 + audience).max(0);
    let resistance_threshold = conviction + (conviction - 10).div_euclid(2).max(0);
    let net_shift = dissonance_pressure - resistance_threshold;

    let social_outcome = if argument_margin <= 0 && net_shift < -4 {
        "Backfire / Entrenchment (Target hardens belief +2 Conviction)".to_string()
    } else if net_shift < 0 {
        "Rationalized Dismissal (Target absorbs tension without shifting)".to_string()
    } else if net_shift < 5 {
        "Seed of Doubt (+1 Dissonance Fatigue; opens follow-up concession)".to_string()
    } else if net_shift < 10 {
        "Tactical Concession (Target yields immediate behavioral compliance)".to_string()
    } else {
        "Paradigm Fracture / Core Belief Shift (Lasting alignment change)".to_string()
    };

    SocialDissonanceOutcome {
        dissonance_pressure,
        resistance_threshold,
        net_dissonance_shift: net_shift,
        social_outcome,
    }
}
