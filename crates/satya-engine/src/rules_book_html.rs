//! Markdown, LaTeX Math, and Mermaid Flowchart to HTML rendering engine for the Satya TTRPG Rules Book.

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
        if let Some(rest) = trimmed
            .strip_prefix("## ")
            .or_else(|| trimmed.strip_prefix("### "))
        {
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
pub(crate) fn resolve_markdown_link_chapter_id(url: &str) -> Option<&'static str> {
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

const LATEX_SYMBOLS: &[(&str, &str)] = &[
    ("Delta", "Δ"),
    ("delta", "δ"),
    ("gamma", "γ"),
    ("Gamma", "Γ"),
    ("kappa", "κ"),
    ("mu", "μ"),
    ("sigma", "σ"),
    ("Sigma", "Σ"),
    ("alpha", "α"),
    ("beta", "β"),
    ("epsilon", "ε"),
    ("varepsilon", "ε"),
    ("zeta", "ζ"),
    ("eta", "η"),
    ("theta", "θ"),
    ("vartheta", "θ"),
    ("Theta", "Θ"),
    ("lambda", "λ"),
    ("Lambda", "Λ"),
    ("nu", "ν"),
    ("xi", "ξ"),
    ("pi", "π"),
    ("Pi", "Π"),
    ("rho", "ρ"),
    ("tau", "τ"),
    ("phi", "φ"),
    ("varphi", "φ"),
    ("Phi", "Φ"),
    ("chi", "χ"),
    ("psi", "ψ"),
    ("Psi", "Ψ"),
    ("omega", "ω"),
    ("Omega", "Ω"),
    ("times", " × "),
    ("cdot", " · "),
    ("circ", "∘"),
    ("propto", " ∝ "),
    ("pm", "±"),
    ("mp", "∓"),
    ("div", "÷"),
    ("le", " ≤ "),
    ("leq", " ≤ "),
    ("ge", " ≥ "),
    ("geq", " ≥ "),
    ("ne", " ≠ "),
    ("neq", " ≠ "),
    ("approx", " ≈ "),
    ("equiv", " ≡ "),
    ("sim", " ∼ "),
    ("ll", " ≪ "),
    ("gg", " ≫ "),
    ("implies", " ⟹ "),
    ("Rightarrow", " ⟹ "),
    ("iff", " ⟺ "),
    ("Leftrightarrow", " ⟺ "),
    ("to", " → "),
    ("rightarrow", " → "),
    ("leftarrow", " ← "),
    ("leftrightarrow", " ↔ "),
    ("mapsto", " ↦ "),
    ("in", " ∈ "),
    ("notin", " ∉ "),
    ("subset", " ⊂ "),
    ("subseteq", " ⊆ "),
    ("cup", " ∪ "),
    ("cap", " ∩ "),
    ("lfloor", "⌊"),
    ("rfloor", "⌋"),
    ("lceil", "⌈"),
    ("rceil", "⌉"),
    ("mid", " | "),
    ("vert", " | "),
    ("parallel", " ‖ "),
    ("Vert", " ‖ "),
    ("dots", "…"),
    ("ldots", "…"),
    ("cdots", "⋯"),
    ("infty", "∞"),
    ("sum", "∑"),
    ("prod", "∏"),
    ("int", "∫"),
    ("partial", "∂"),
    ("quad", "&emsp;"),
    ("qquad", "&emsp;&emsp;"),
];

/// Converts a raw LaTeX math expression into semantic HTML tokens (`<sub>`, `<sup>`, fractions,
/// cases, and Unicode mathematical operators/symbols).
fn render_latex_tokens(expr: &str) -> String {
    let chars: Vec<char> = expr.chars().collect();
    let mut out = String::with_capacity(expr.len() * 2 + 16);
    let mut i = 0;

    let skip_ws_group = |chars: &[char], mut idx: usize| -> Option<(String, usize)> {
        while idx < chars.len() && chars[idx].is_whitespace() {
            idx += 1;
        }
        consume_braced_group(chars, idx)
    };

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' {
            if i + 1 >= chars.len() {
                i += 1;
                continue;
            }
            let next = chars[i + 1];
            if !next.is_ascii_alphabetic() {
                match next {
                    '_' | '{' | '}' | '%' | '$' => out.push(next),
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

            if i + 1 < chars.len() && chars[i] == '\\' && chars[i + 1] == '!' {
                i += 2;
            }

            if let Some(&(_, sym)) = LATEX_SYMBOLS.iter().find(|&&(k, _)| k == cmd) {
                out.push_str(sym);
                continue;
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
                    let (num_raw, after_num) =
                        skip_ws_group(&chars, i).unwrap_or_else(|| (String::new(), i));
                    let (den_raw, after_den) = skip_ws_group(&chars, after_num)
                        .unwrap_or_else(|| (String::new(), after_num));
                    i = after_den;
                    out.push_str("<span class=\"math-frac\"><span class=\"math-num\">");
                    out.push_str(&render_latex_tokens(&num_raw));
                    out.push_str("</span><span class=\"math-den\">");
                    out.push_str(&render_latex_tokens(&den_raw));
                    out.push_str("</span></span>");
                }
                "sqrt" => {
                    if let Some((rad_raw, after_rad)) = skip_ws_group(&chars, i) {
                        i = after_rad;
                        out.push_str("<span class=\"math-sqrt\">√<span class=\"math-radicand\">");
                        out.push_str(&render_latex_tokens(&rad_raw));
                        out.push_str("</span></span>");
                    } else {
                        out.push('√');
                    }
                }
                "text" | "textrm" | "mathrm" | "operatorname" => {
                    if let Some((txt_raw, after_txt)) = skip_ws_group(&chars, i) {
                        i = after_txt;
                        out.push_str("<span class=\"math-roman\">");
                        out.push_str(&render_latex_text_argument(&txt_raw));
                        out.push_str("</span>");
                    }
                }
                "mathbf" | "boldsymbol" => {
                    if let Some((inner, after_grp)) = skip_ws_group(&chars, i) {
                        i = after_grp;
                        out.push_str("<strong>");
                        out.push_str(&render_latex_tokens(&inner));
                        out.push_str("</strong>");
                    }
                }
                "mathit" => {
                    if let Some((inner, after_grp)) = skip_ws_group(&chars, i) {
                        i = after_grp;
                        out.push_str("<em>");
                        out.push_str(&render_latex_tokens(&inner));
                        out.push_str("</em>");
                    }
                }
                "mathbb" | "mathcal" | "mathrel" | "underbrace" | "overbrace" => {
                    if let Some((inner, after_grp)) = skip_ws_group(&chars, i) {
                        i = after_grp;
                        out.push_str(&render_latex_tokens(&inner));
                    }
                }
                "begin" => {
                    let (env_name, after_env) =
                        skip_ws_group(&chars, i).unwrap_or_else(|| (String::new(), i));
                    i = after_env;
                    let end_marker = format!("\\end{{{env_name}}}");
                    let remaining: String = chars[i..].iter().collect();
                    if let Some(rel_end) = remaining.find(&end_marker) {
                        let body = &remaining[..rel_end];
                        i += body.chars().count() + end_marker.chars().count();
                        out.push_str(
                            "<span class=\"math-cases\"><span class=\"math-brace\">{</span><span class=\"math-cases-rows\">",
                        );
                        for row in body.split("\\\\").map(str::trim).filter(|r| !r.is_empty()) {
                            out.push_str("<span class=\"math-cases-row\">");
                            for (c_idx, col) in row.split('&').enumerate() {
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
                "max" | "min" | "log" | "ln" | "exp" | "sin" | "cos" | "tan" | "Pr" | "sup"
                | "inf" => {
                    out.push_str("<span class=\"math-fn\">");
                    out.push_str(&cmd);
                    out.push_str("</span>");
                }
                other => out.push_str(&escape_html_text(other)),
            }
            continue;
        }

        if ch == '_' || ch == '^' {
            let tag = if ch == '_' { "sub" } else { "sup" };
            let (script_html, next_i) = consume_latex_script_atom(&chars, i + 1);
            out.push('<');
            out.push_str(tag);
            out.push('>');
            out.push_str(&script_html);
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
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

        if chars[i] == '$'
            && let Some(math_end) = find_closing_inline_math_dollar(&chars, i)
        {
            let math_expr: String = chars[i + 1..math_end].iter().collect();
            result.push_str(&render_latex_math_to_html(&math_expr, false));
            i = math_end + 1;
            continue;
        }

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

        if chars[i] == '[' {
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

/// Shared list item body & sublist continuation parser used by both `<ul>` and `<ol>`.
fn consume_list_item_body<'a>(
    html: &mut String,
    lines: &[&'a str],
    mut i: usize,
    first_rest: &'a str,
) -> usize {
    html.push_str("<li>");
    let mut item_prose = vec![first_rest];
    while i < lines.len() {
        let next_raw = lines[i];
        let next_trim = next_raw.trim();
        let is_indented = next_raw.starts_with("  ") || next_raw.starts_with('\t');
        let is_top_bullet =
            !is_indented && (next_trim.starts_with("- ") || next_trim.starts_with("* "));
        let is_top_ordered = !is_indented
            && next_trim
                .split_once(". ")
                .is_some_and(|(p, _)| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
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
    i
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

        if trimmed == "---" || trimmed == "***" {
            html.push_str("<hr class=\"rules-divider\"/>");
            i += 1;
            continue;
        }

        if let Some(lang) = trimmed.strip_prefix("```") {
            let lang_clean = lang.trim();
            i += 1;
            let mut block_lines = Vec::new();
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                block_lines.push(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1;
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

        if let Some((tag, h_text)) = trimmed
            .strip_prefix("#### ")
            .map(|t| ("h4", t))
            .or_else(|| trimmed.strip_prefix("### ").map(|t| ("h3", t)))
            .or_else(|| trimmed.strip_prefix("## ").map(|t| ("h2", t)))
            .or_else(|| trimmed.strip_prefix("# ").map(|t| ("h1", t)))
        {
            let slug = slugify_heading(h_text);
            html.push('<');
            html.push_str(tag);
            html.push_str(" id=\"");
            html.push_str(&slug);
            html.push_str("\">");
            html.push_str(&format_inline_markdown(h_text));
            html.push_str("</");
            html.push_str(tag);
            html.push('>');
            i += 1;
            continue;
        }

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

        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            html.push_str("<ul class=\"rules-list\">");
            while i < lines.len() {
                let item_trim = lines[i].trim();
                if let Some(rest) = item_trim
                    .strip_prefix("- ")
                    .or_else(|| item_trim.strip_prefix("* "))
                {
                    i = consume_list_item_body(&mut html, &lines, i + 1, rest);
                } else {
                    break;
                }
            }
            html.push_str("</ul>");
            continue;
        }

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
                    i = consume_list_item_body(&mut html, &lines, i + 1, rest);
                } else {
                    break;
                }
            }
            html.push_str("</ol>");
            continue;
        }

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
