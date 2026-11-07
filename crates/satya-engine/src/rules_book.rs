//! Canonical Satya TTRPG Rules Book catalog, Markdown-to-HTML renderer, multi-chapter search,
//! and Combat Tactical Pool ("Envelope & Fill" — Tempo, Control, Force) calculator.

use satya_schema::{
    RulesBookCatalogResponse, RulesChapterDocument, RulesChapterSummary, RulesSearchHit,
};

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

/// Maps a relative Markdown link URL (such as `../01-foundation/mathematical_notation_glossary.md`)
/// to its canonical `chapter_id` if it points to one of the Satya Rules Book chapters.
fn resolve_markdown_link_chapter_id(url: &str) -> Option<&'static str> {
    let path_part = url.split(['#', '?']).next().unwrap_or(url).trim();
    if !path_part.ends_with(".md") {
        return None;
    }
    let file_name = path_part.rsplit('/').next().unwrap_or(path_part);
    match file_name {
        "design_philosophy.md" | "background.md" => Some("design_philosophy"),
        "mathematical_notation_glossary.md" | "notation_glossary.md" => Some("notation_glossary"),
        "probability_and_calibration.md" | "probability.md" => Some("probability"),
        "cascading_d20_and_tasks.md" | "satya_d20_rules.md" | "tasks.md" => Some("tasks"),
        "action_economy_and_ooda.md" | "action_economy.md" => Some("action_economy"),
        "characteristics_and_aptitudes.md" | "characteristics.md" => Some("characteristics"),
        "skills_and_trees.md" | "skill-design.md" => Some("skills_and_trees"),
        "lifepaths_and_economy.md" => Some("lifepaths_and_economy"),
        "psyche_morale_and_social_combat.md"
        | "social_interactions.md"
        | "psyche_and_social.md" => Some("psyche_and_social"),
        "tactical_triad_and_pools.md" | "combat.md" | "tactical_triad.md" => Some("tactical_triad"),
        "melee_combat.md" => Some("melee_combat"),
        "ranged_combat_and_armor.md" | "world_and_equipment_stubs.md" => {
            Some("ranged_combat_and_armor")
        }
        "health_and_trauma.md" => Some("health_and_trauma"),
        "movement_vehicles_and_environment.md" | "movement_and_environment.md" => {
            Some("movement_and_environment")
        }
        _ => None,
    }
}

/// Extracts a brace-delimited `{...}` group starting at `chars[pos]` (where `chars[pos] == '{'`),
/// returning `(inner_string, next_index_after_closing_brace)`.
fn consume_braced_group(chars: &[char], pos: usize) -> Option<(String, usize)> {
    if pos >= chars.len() || chars[pos] != '{' {
        return None;
    }
    let mut depth = 0usize;
    let mut i = pos;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && matches!(chars[i + 1], '{' | '}') {
            i += 2;
            continue;
        }
        if chars[i] == '{' {
            depth += 1;
        } else if chars[i] == '}' {
            depth -= 1;
            if depth == 0 {
                let inner: String = chars[pos + 1..i].iter().collect();
                return Some((inner, i + 1));
            }
        }
        i += 1;
    }
    let inner: String = chars[pos + 1..].iter().collect();
    Some((inner, chars.len()))
}

/// Consumes a single LaTeX script argument (either `{...}`, a `\command`, or a single character)
/// starting at `pos` for `_` or `^`.
fn consume_latex_script_atom(chars: &[char], pos: usize) -> (String, usize) {
    let mut i = pos;
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    if i >= chars.len() {
        return (String::new(), i);
    }
    if chars[i] == '{'
        && let Some((inner, next_i)) = consume_braced_group(chars, i)
    {
        return (render_latex_tokens(&inner), next_i);
    }
    if chars[i] == '\\' {
        let mut end = i + 1;
        while end < chars.len() && chars[end].is_ascii_alphabetic() {
            end += 1;
        }
        if end < chars.len()
            && chars[end] == '{'
            && let Some((_, after_brace)) = consume_braced_group(chars, end)
        {
            end = after_brace;
        }
        let slice: String = chars[i..end].iter().collect();
        return (render_latex_tokens(&slice), end);
    }
    let ch = chars[i];
    (escape_html_text(&ch.to_string()), i + 1)
}

/// Renders plain text inside `\text{...}` or `\mathrm{...}`, unescaping `\_` and escaping HTML.
fn render_latex_text_argument(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 8);
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                '_' | '{' | '}' | '%' | '$' | '#' => {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                '&' => {
                    out.push_str("&amp;");
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        match chars[i] {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(chars[i]),
        }
        i += 1;
    }
    out
}

/// Converts a raw LaTeX math expression into semantic HTML tokens (`<sub>`, `<sup>`, fractions,
/// cases, and Unicode mathematical operators/symbols).
fn render_latex_tokens(expr: &str) -> String {
    let chars: Vec<char> = expr.chars().collect();
    let mut out = String::with_capacity(expr.len() * 2 + 16);
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' {
            if i + 1 >= chars.len() {
                i += 1;
                continue;
            }
            let next = chars[i + 1];
            // Single-character escaped symbol or spacing command
            if !next.is_ascii_alphabetic() {
                match next {
                    '_' => out.push('_'),
                    '{' => out.push('{'),
                    '}' => out.push('}'),
                    '%' => out.push('%'),
                    '$' => out.push('$'),
                    '&' => out.push_str("&amp;"),
                    ',' => out.push_str("&#8201;"),
                    ';' | ':' => out.push_str("&ensp;"),
                    '!' => {}
                    ' ' => out.push_str("&nbsp;"),
                    '\\' => out.push_str("<br/>"),
                    '|' => out.push('‖'),
                    _ => out.push_str(&escape_html_text(&next.to_string())),
                }
                i += 2;
                continue;
            }

            let mut cmd_end = i + 1;
            while cmd_end < chars.len() && chars[cmd_end].is_ascii_alphabetic() {
                cmd_end += 1;
            }
            let cmd: String = chars[i + 1..cmd_end].iter().collect();
            i = cmd_end;

            // Skip optional `\!` right after `\min`, `\max`, etc.
            if i + 1 < chars.len() && chars[i] == '\\' && chars[i + 1] == '!' {
                i += 2;
            }

            match cmd.as_str() {
                "left" | "right" | "big" | "Big" | "bigg" | "Bigg" | "bigl" | "bigr" | "Bigl"
                | "Bigr" => {
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if i < chars.len() && chars[i] == '.' {
                        i += 1;
                    }
                }
                "frac" | "dfrac" | "tfrac" => {
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    let (num_raw, after_num) =
                        consume_braced_group(&chars, i).unwrap_or_else(|| (String::new(), i));
                    i = after_num;
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    let (den_raw, after_den) =
                        consume_braced_group(&chars, i).unwrap_or_else(|| (String::new(), i));
                    i = after_den;
                    out.push_str("<span class=\"math-frac\"><span class=\"math-num\">");
                    out.push_str(&render_latex_tokens(&num_raw));
                    out.push_str("</span><span class=\"math-den\">");
                    out.push_str(&render_latex_tokens(&den_raw));
                    out.push_str("</span></span>");
                }
                "sqrt" => {
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if let Some((rad_raw, after_rad)) = consume_braced_group(&chars, i) {
                        i = after_rad;
                        out.push_str("<span class=\"math-sqrt\">√<span class=\"math-radicand\">");
                        out.push_str(&render_latex_tokens(&rad_raw));
                        out.push_str("</span></span>");
                    } else {
                        out.push('√');
                    }
                }
                "text" | "textrm" | "mathrm" | "operatorname" => {
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if let Some((txt_raw, after_txt)) = consume_braced_group(&chars, i) {
                        i = after_txt;
                        out.push_str("<span class=\"math-roman\">");
                        out.push_str(&render_latex_text_argument(&txt_raw));
                        out.push_str("</span>");
                    }
                }
                "mathbf" | "boldsymbol" => {
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if let Some((inner, after_grp)) = consume_braced_group(&chars, i) {
                        i = after_grp;
                        out.push_str("<strong>");
                        out.push_str(&render_latex_tokens(&inner));
                        out.push_str("</strong>");
                    }
                }
                "mathit" => {
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if let Some((inner, after_grp)) = consume_braced_group(&chars, i) {
                        i = after_grp;
                        out.push_str("<em>");
                        out.push_str(&render_latex_tokens(&inner));
                        out.push_str("</em>");
                    }
                }
                "mathbb" | "mathcal" | "mathrel" | "underbrace" | "overbrace" => {
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    if let Some((inner, after_grp)) = consume_braced_group(&chars, i) {
                        i = after_grp;
                        out.push_str(&render_latex_tokens(&inner));
                    }
                }
                "begin" => {
                    while i < chars.len() && chars[i].is_whitespace() {
                        i += 1;
                    }
                    let (env_name, after_env) =
                        consume_braced_group(&chars, i).unwrap_or_else(|| (String::new(), i));
                    i = after_env;
                    let end_marker = format!("\\end{{{env_name}}}");
                    let remaining: String = chars[i..].iter().collect();
                    if let Some(rel_end) = remaining.find(&end_marker) {
                        let body = &remaining[..rel_end];
                        let body_chars = body.chars().count();
                        let marker_chars = end_marker.chars().count();
                        i += body_chars + marker_chars;
                        out.push_str(
                            "<span class=\"math-cases\"><span class=\"math-brace\">{</span><span class=\"math-cases-rows\">",
                        );
                        for row in body.split("\\\\") {
                            let row_trim = row.trim();
                            if row_trim.is_empty() {
                                continue;
                            }
                            out.push_str("<span class=\"math-cases-row\">");
                            let cols: Vec<&str> = row_trim.split('&').collect();
                            for (c_idx, col) in cols.iter().enumerate() {
                                if c_idx > 0 {
                                    out.push_str("<span class=\"math-cases-sep\">&ensp;</span>");
                                }
                                out.push_str(&render_latex_tokens(col.trim()));
                            }
                            out.push_str("</span>");
                        }
                        out.push_str("</span></span>");
                    }
                }
                "Delta" => out.push('Δ'),
                "delta" => out.push('δ'),
                "gamma" => out.push('γ'),
                "Gamma" => out.push('Γ'),
                "kappa" => out.push('κ'),
                "mu" => out.push('μ'),
                "sigma" => out.push('σ'),
                "Sigma" => out.push('Σ'),
                "alpha" => out.push('α'),
                "beta" => out.push('β'),
                "epsilon" | "varepsilon" => out.push('ε'),
                "theta" | "vartheta" => out.push('θ'),
                "Theta" => out.push('Θ'),
                "lambda" => out.push('λ'),
                "Lambda" => out.push('Λ'),
                "pi" => out.push('π'),
                "Pi" => out.push('Π'),
                "rho" => out.push('ρ'),
                "tau" => out.push('τ'),
                "phi" | "varphi" => out.push('φ'),
                "Phi" => out.push('Φ'),
                "omega" => out.push('ω'),
                "Omega" => out.push('Ω'),
                "times" => out.push_str(" × "),
                "cdot" => out.push_str(" · "),
                "pm" => out.push('±'),
                "mp" => out.push('∓'),
                "div" => out.push('÷'),
                "le" | "leq" => out.push_str(" ≤ "),
                "ge" | "geq" => out.push_str(" ≥ "),
                "ne" | "neq" => out.push_str(" ≠ "),
                "approx" => out.push_str(" ≈ "),
                "equiv" => out.push_str(" ≡ "),
                "sim" => out.push_str(" ∼ "),
                "ll" => out.push_str(" ≪ "),
                "gg" => out.push_str(" ≫ "),
                "implies" | "Rightarrow" => out.push_str(" ⟹ "),
                "iff" | "Leftrightarrow" => out.push_str(" ⟺ "),
                "to" | "rightarrow" => out.push_str(" → "),
                "leftarrow" => out.push_str(" ← "),
                "leftrightarrow" => out.push_str(" ↔ "),
                "mapsto" => out.push_str(" ↦ "),
                "in" => out.push_str(" ∈ "),
                "notin" => out.push_str(" ∉ "),
                "subset" => out.push_str(" ⊂ "),
                "subseteq" => out.push_str(" ⊆ "),
                "cup" => out.push_str(" ∪ "),
                "cap" => out.push_str(" ∩ "),
                "lfloor" => out.push('⌊'),
                "rfloor" => out.push('⌋'),
                "lceil" => out.push('⌈'),
                "rceil" => out.push('⌉'),
                "mid" | "vert" => out.push_str(" | "),
                "parallel" | "Vert" => out.push_str(" ‖ "),
                "dots" | "ldots" => out.push('…'),
                "cdots" => out.push('⋯'),
                "infty" => out.push('∞'),
                "sum" => out.push('∑'),
                "prod" => out.push('∏'),
                "int" => out.push('∫'),
                "partial" => out.push('∂'),
                "quad" => out.push_str("&emsp;"),
                "qquad" => out.push_str("&emsp;&emsp;"),
                "max" | "min" | "log" | "ln" | "exp" | "sin" | "cos" | "tan" | "Pr" | "sup"
                | "inf" => {
                    out.push_str("<span class=\"math-fn\">");
                    out.push_str(&cmd);
                    out.push_str("</span>");
                }
                other => {
                    out.push_str(&escape_html_text(other));
                }
            }
            continue;
        }

        if ch == '_' {
            let (sub_html, next_i) = consume_latex_script_atom(&chars, i + 1);
            out.push_str("<sub>");
            out.push_str(&sub_html);
            out.push_str("</sub>");
            i = next_i;
            continue;
        }

        if ch == '^' {
            let (sup_html, next_i) = consume_latex_script_atom(&chars, i + 1);
            out.push_str("<sup>");
            out.push_str(&sup_html);
            out.push_str("</sup>");
            i = next_i;
            continue;
        }

        if ch == '{'
            && let Some((inner, next_i)) = consume_braced_group(&chars, i)
        {
            out.push_str(&render_latex_tokens(&inner));
            i = next_i;
            continue;
        }

        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '-' => out.push('−'),
            _ => out.push(ch),
        }
        i += 1;
    }

    out
}

/// Renders a LaTeX math expression (`$...$` when `display == false`, `$$...$$` when `display == true`)
/// into structured, styled HTML.
pub fn render_latex_math_to_html(expr: &str, display: bool) -> String {
    let trimmed = expr
        .trim()
        .trim_start_matches("$$")
        .trim_end_matches("$$")
        .trim_start_matches('$')
        .trim_end_matches('$')
        .trim();
    let inner = render_latex_tokens(trimmed);
    if display {
        format!("<div class=\"rules-math-block\">{inner}</div>")
    } else {
        format!("<span class=\"rules-math-inline\">{inner}</span>")
    }
}

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

/// A single parsed Mermaid edge hop: `(edge_label_html, is_dashed, target_node_token)`.
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

            // Pattern 1: `-.->|"label"|` or `-->|"label"|`
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

            // Pattern 2: `-- "label" -->`
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
        let idx = if let Some(&existing_idx) = node_index.get(&id) {
            if let Some((lbl, is_d)) = label_opt {
                nodes[existing_idx].label_html = lbl;
                nodes[existing_idx].is_diamond = is_d;
            }
            existing_idx
        } else {
            let (lbl, is_d) = label_opt.unwrap_or_else(|| (escape_html_text(&id), false));
            let new_idx = nodes.len();
            nodes.push(MermaidNode {
                id: id.clone(),
                label_html: lbl,
                is_diamond: is_d,
            });
            node_index.insert(id.clone(), new_idx);
            new_idx
        };
        let _ = idx;
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

    // Helper closure to format outgoing edges from a node ID as compact connection badges
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
        // Render any top-level nodes that do not belong to any subgraph
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
                let shape_cls = if n.is_diamond {
                    "mermaid-node mermaid-node-diamond"
                } else {
                    "mermaid-node"
                };
                html.push_str("<div class=\"");
                html.push_str(shape_cls);
                html.push_str("\"><div class=\"mermaid-node-body\">");
                html.push_str(&n.label_html);
                html.push_str("</div></div>");
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
            for (pos, nid) in sg.node_ids.iter().enumerate() {
                if let Some(&nidx) = node_index.get(nid) {
                    let n = &nodes[nidx];
                    let shape_cls = if n.is_diamond {
                        "mermaid-node mermaid-node-diamond"
                    } else {
                        "mermaid-node"
                    };
                    html.push_str("<div class=\"");
                    html.push_str(shape_cls);
                    html.push_str("\"><div class=\"mermaid-node-body\">");
                    html.push_str(&n.label_html);
                    html.push_str("</div>");

                    // Outgoing edges from this node to other nodes
                    let out_edges: Vec<&MermaidEdge> =
                        edges.iter().filter(|e| e.from == *nid).collect();
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
                    html.push_str("</div>");
                    let _ = pos;
                }
            }
            html.push_str("</div></section>");
        }
        html.push_str("</div>");
    } else {
        // Check if this is a simple linear chain `N0 -> N1 -> N2 ...`
        let is_simple_chain = edges.len() + 1 == nodes.len()
            && edges
                .iter()
                .enumerate()
                .all(|(idx, e)| e.from == nodes[idx].id && e.to == nodes[idx + 1].id);

        html.push_str("<div class=\"mermaid-flow-track\">");
        for (idx, n) in nodes.iter().enumerate() {
            let shape_cls = if n.is_diamond {
                "mermaid-node mermaid-node-diamond"
            } else {
                "mermaid-node"
            };
            html.push_str("<div class=\"");
            html.push_str(shape_cls);
            html.push_str("\"><div class=\"mermaid-node-body\">");
            html.push_str(&n.label_html);
            html.push_str("</div>");

            if !is_simple_chain {
                let out_edges: Vec<&MermaidEdge> =
                    edges.iter().filter(|e| e.from == n.id).collect();
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

/// Checks if a `$` at `start` begins a valid inline LaTeX math span `$...$`, returning
/// the index of the closing `$` if valid.
fn find_closing_inline_math_dollar(chars: &[char], start: usize) -> Option<usize> {
    if start + 2 > chars.len() || chars[start] != '$' {
        return None;
    }
    let first_inner = chars[start + 1];
    if first_inner == '$' || first_inner.is_whitespace() {
        return None;
    }
    let mut j = start + 1;
    while j < chars.len() {
        if chars[j] == '\\' {
            j += 2;
            continue;
        }
        if chars[j] == '`' {
            // Never cross an inline code backtick
            return None;
        }
        if chars[j] == '$' {
            let prev = chars[j - 1];
            let next_is_digit = j + 1 < chars.len() && chars[j + 1].is_ascii_digit();
            if !prev.is_whitespace() && !next_is_digit {
                return Some(j);
            }
            return None;
        }
        j += 1;
    }
    None
}

/// Formats inline Markdown syntax (inline code `` `...` ``, display math `$$...$$`, inline math `$...$`,
/// bold `**...**`, italic `*...*`, and links `[label](href)`).
pub fn format_inline_markdown(line: &str) -> String {
    let mut result = String::with_capacity(line.len() + 48);
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // 1. Inline code `...`
        if chars[i] == '`'
            && let Some(end_offset) = chars[i + 1..].iter().position(|&c| c == '`')
        {
            let code_end = i + 1 + end_offset;
            let code_text: String = chars[i + 1..code_end].iter().collect();
            result.push_str("<code class=\"rules-inline-code\">");
            result.push_str(&escape_html_text(&code_text));
            result.push_str("</code>");
            i = code_end + 1;
            continue;
        }

        // 2. Inline display math $$...$$
        if i + 1 < chars.len()
            && chars[i] == '$'
            && chars[i + 1] == '$'
            && let Some(end_offset) = chars[i + 2..]
                .windows(2)
                .position(|w| w[0] == '$' && w[1] == '$')
        {
            let math_end = i + 2 + end_offset;
            let math_expr: String = chars[i + 2..math_end].iter().collect();
            result.push_str(&render_latex_math_to_html(&math_expr, true));
            i = math_end + 2;
            continue;
        }

        // 3. Inline math $...$
        if chars[i] == '$'
            && let Some(math_end) = find_closing_inline_math_dollar(&chars, i)
        {
            let math_expr: String = chars[i + 1..math_end].iter().collect();
            result.push_str(&render_latex_math_to_html(&math_expr, false));
            i = math_end + 1;
            continue;
        }

        // 4. Bold + Italic ***...***
        if i + 2 < chars.len()
            && chars[i] == '*'
            && chars[i + 1] == '*'
            && chars[i + 2] == '*'
            && let Some(end_offset) = chars[i + 3..]
                .windows(3)
                .position(|w| w[0] == '*' && w[1] == '*' && w[2] == '*')
        {
            let end_idx = i + 3 + end_offset;
            let inner: String = chars[i + 3..end_idx].iter().collect();
            result.push_str("<strong><em>");
            result.push_str(&format_inline_markdown(&inner));
            result.push_str("</em></strong>");
            i = end_idx + 3;
            continue;
        }

        // 5. Bold **...**
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

        // 6. Italic *...*
        if chars[i] == '*'
            && i + 1 < chars.len()
            && !chars[i + 1].is_whitespace()
            && let Some(end_offset) = chars[i + 1..].iter().position(|&c| c == '*')
        {
            let em_end = i + 1 + end_offset;
            if !chars[em_end - 1].is_whitespace() {
                let inner: String = chars[i + 1..em_end].iter().collect();
                result.push_str("<em>");
                result.push_str(&format_inline_markdown(&inner));
                result.push_str("</em>");
                i = em_end + 1;
                continue;
            }
        }

        // 7. Markdown link [label](url)
        if chars[i] == '[' {
            // Find matching `]` allowing nested brackets if any
            let mut depth = 0usize;
            let mut bracket_idx = None;
            for (offset, &ch) in chars[i..].iter().enumerate() {
                if ch == '[' {
                    depth += 1;
                } else if ch == ']' {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        bracket_idx = Some(i + offset);
                        break;
                    }
                }
            }
            if let Some(b_idx) = bracket_idx
                && b_idx + 1 < chars.len()
                && chars[b_idx + 1] == '('
                && let Some(close_paren) = chars[b_idx + 2..].iter().position(|&c| c == ')')
            {
                let paren_idx = b_idx + 2 + close_paren;
                let label: String = chars[i + 1..b_idx].iter().collect();
                let url: String = chars[b_idx + 2..paren_idx].iter().collect();
                result.push_str("<a class=\"rules-link\" href=\"");
                result.push_str(&escape_html_text(&url));
                result.push('"');
                if let Some(target_chapter) = resolve_markdown_link_chapter_id(&url) {
                    result.push_str(" data-rules-chapter=\"");
                    result.push_str(target_chapter);
                    result.push_str("\" onclick=\"if(window.satyaSelectRulesChapter){window.satyaSelectRulesChapter('");
                    result.push_str(target_chapter);
                    result.push_str("');return false;}\"");
                }
                result.push('>');
                result.push_str(&format_inline_markdown(&label));
                result.push_str("</a>");
                i = paren_idx + 1;
                continue;
            }
        }

        // Plain character — escape HTML entities once at leaf level
        match chars[i] {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            ch => result.push(ch),
        }
        i += 1;
    }
    result
}

/// Splits a GFM table row (`| col1 | col2 |`) into trimmed cell strings without splitting on
/// `|` inside inline code (`` `...` ``), inline math (`$...$`), or escaped pipes (`\|`).
fn split_markdown_table_cells(row_str: &str) -> Vec<String> {
    let inner = row_str.trim().trim_matches('|');
    let chars: Vec<char> = inner.chars().collect();
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_code = false;
    let mut in_math = false;
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && i + 1 < chars.len() && chars[i + 1] == '|' {
            current.push('|');
            i += 2;
            continue;
        }
        if ch == '`' {
            in_code = !in_code;
            current.push(ch);
            i += 1;
            continue;
        }
        if ch == '$' && !in_code {
            if in_math {
                in_math = false;
            } else if find_closing_inline_math_dollar(&chars, i).is_some() {
                in_math = true;
            }
            current.push(ch);
            i += 1;
            continue;
        }
        if ch == '|' && !in_code && !in_math {
            cells.push(current.trim().to_string());
            current.clear();
            i += 1;
            continue;
        }
        current.push(ch);
        i += 1;
    }
    cells.push(current.trim().to_string());
    cells
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
                html.push_str(&render_mermaid_to_html(&block_content));
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
            html.push_str(&render_latex_math_to_html(&math_lines.join("\n"), true));
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
            let mut q_para = Vec::new();
            for &qline in &quote_lines[start_idx..] {
                if qline.is_empty() {
                    if !q_para.is_empty() {
                        html.push_str("<p>");
                        html.push_str(&format_inline_markdown(&q_para.join(" ")));
                        html.push_str("</p>");
                        q_para.clear();
                    }
                } else {
                    q_para.push(qline);
                }
            }
            if !q_para.is_empty() {
                html.push_str("<p>");
                html.push_str(&format_inline_markdown(&q_para.join(" ")));
                html.push_str("</p>");
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
                let cells = split_markdown_table_cells(row_str);
                let is_sep = cells.iter().all(|c| {
                    !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
                });
                if is_sep {
                    continue;
                }
                if row_idx == 0 {
                    html.push_str("<thead><tr>");
                    for cell in &cells {
                        html.push_str("<th>");
                        html.push_str(&format_inline_markdown(cell));
                        html.push_str("</th>");
                    }
                    html.push_str("</tr></thead><tbody>");
                } else {
                    html.push_str("<tr>");
                    for cell in &cells {
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
                    let mut item_prose = vec![rest];
                    i += 1;
                    // Consume any continuation lines (indented or lazy-wrapped non-empty lines)
                    while i < lines.len() {
                        let next_raw = lines[i];
                        let next_trim = next_raw.trim();
                        let is_indented = next_raw.starts_with("  ") || next_raw.starts_with('\t');
                        let is_top_bullet = !is_indented
                            && (next_trim.starts_with("- ") || next_trim.starts_with("* "));
                        let is_top_ordered = !is_indented
                            && next_trim.split_once(". ").is_some_and(|(p, _)| {
                                !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())
                            });
                        let is_new_block = next_trim.is_empty()
                            || next_trim == "---"
                            || next_trim == "***"
                            || next_trim.starts_with('#')
                            || next_trim.starts_with("```")
                            || next_trim.starts_with('>')
                            || (next_trim.starts_with('|') && next_trim.ends_with('|'))
                            || (!is_indented && next_trim.starts_with("$$"));

                        if is_new_block || is_top_bullet || is_top_ordered {
                            break;
                        }
                        if is_indented && next_trim.starts_with("$$") {
                            if !item_prose.is_empty() {
                                html.push_str(&format_inline_markdown(&item_prose.join(" ")));
                                item_prose.clear();
                            }
                            html.push_str(&render_latex_math_to_html(next_trim, true));
                            i += 1;
                        } else if is_indented
                            && let Some(sub_rest) = next_trim
                                .strip_prefix("- ")
                                .or_else(|| next_trim.strip_prefix("* "))
                        {
                            if !item_prose.is_empty() {
                                html.push_str(&format_inline_markdown(&item_prose.join(" ")));
                                item_prose.clear();
                            }
                            let mut sub_prose = vec![sub_rest];
                            i += 1;
                            while i < lines.len() {
                                let sub_raw = lines[i];
                                let sub_trim = sub_raw.trim();
                                if !sub_trim.is_empty()
                                    && (sub_raw.starts_with("    ") || sub_raw.starts_with('\t'))
                                    && !sub_trim.starts_with("- ")
                                    && !sub_trim.starts_with("* ")
                                    && !sub_trim.starts_with("$$")
                                {
                                    sub_prose.push(sub_trim);
                                    i += 1;
                                } else {
                                    break;
                                }
                            }
                            html.push_str("<div class=\"rules-sublist-item\">• ");
                            html.push_str(&format_inline_markdown(&sub_prose.join(" ")));
                            html.push_str("</div>");
                        } else {
                            item_prose.push(next_trim);
                            i += 1;
                        }
                    }
                    if !item_prose.is_empty() {
                        html.push_str(&format_inline_markdown(&item_prose.join(" ")));
                    }
                    html.push_str("</li>");
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
                    let mut item_prose = vec![rest];
                    i += 1;
                    while i < lines.len() {
                        let next_raw = lines[i];
                        let next_trim = next_raw.trim();
                        let is_indented = next_raw.starts_with("  ") || next_raw.starts_with('\t');
                        let is_top_bullet = !is_indented
                            && (next_trim.starts_with("- ") || next_trim.starts_with("* "));
                        let is_top_ordered = !is_indented
                            && next_trim.split_once(". ").is_some_and(|(p, _)| {
                                !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())
                            });
                        let is_new_block = next_trim.is_empty()
                            || next_trim == "---"
                            || next_trim == "***"
                            || next_trim.starts_with('#')
                            || next_trim.starts_with("```")
                            || next_trim.starts_with('>')
                            || (next_trim.starts_with('|') && next_trim.ends_with('|'))
                            || (!is_indented && next_trim.starts_with("$$"));

                        if is_new_block || is_top_bullet || is_top_ordered {
                            break;
                        }
                        if is_indented && next_trim.starts_with("$$") {
                            if !item_prose.is_empty() {
                                html.push_str(&format_inline_markdown(&item_prose.join(" ")));
                                item_prose.clear();
                            }
                            html.push_str(&render_latex_math_to_html(next_trim, true));
                            i += 1;
                        } else if is_indented
                            && let Some(sub_rest) = next_trim
                                .strip_prefix("- ")
                                .or_else(|| next_trim.strip_prefix("* "))
                        {
                            if !item_prose.is_empty() {
                                html.push_str(&format_inline_markdown(&item_prose.join(" ")));
                                item_prose.clear();
                            }
                            let mut sub_prose = vec![sub_rest];
                            i += 1;
                            while i < lines.len() {
                                let sub_raw = lines[i];
                                let sub_trim = sub_raw.trim();
                                if !sub_trim.is_empty()
                                    && (sub_raw.starts_with("    ") || sub_raw.starts_with('\t'))
                                    && !sub_trim.starts_with("- ")
                                    && !sub_trim.starts_with("* ")
                                    && !sub_trim.starts_with("$$")
                                {
                                    sub_prose.push(sub_trim);
                                    i += 1;
                                } else {
                                    break;
                                }
                            }
                            html.push_str("<div class=\"rules-sublist-item\">• ");
                            html.push_str(&format_inline_markdown(&sub_prose.join(" ")));
                            html.push_str("</div>");
                        } else {
                            item_prose.push(next_trim);
                            i += 1;
                        }
                    }
                    if !item_prose.is_empty() {
                        html.push_str(&format_inline_markdown(&item_prose.join(" ")));
                    }
                    html.push_str("</li>");
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
// Re-exported Combat, Lifepath & Social Calculators (relocated to domain modules)
// ============================================================================

pub use crate::combat::{
    MoraleState, TacticalPoolEnvelope, TacticalPoolEnvelopeSummary, TacticalPoolParams,
    TacticalTriadContestOutcome, TacticalTriadEvaluation, compute_tactical_pool,
    compute_tactical_pool_envelope, evaluate_tactical_triad, evaluate_tactical_triad_contest,
};
pub use crate::lifepath_economy::{LifepathSkillProgression, compute_lifepath_skill_progression};
pub use crate::psyche_social::{SocialDissonanceOutcome, compute_social_dissonance_check};

