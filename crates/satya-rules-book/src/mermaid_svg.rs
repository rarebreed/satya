//! Pure-Rust, zero-dependency layered SVG flowchart renderer for `mermaid` code blocks
//! across `docs/ttrpg/`, `docs/campaign/`, static HTML books, and `satya-web`.

use crate::escape_html_text;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

#[derive(Debug, Clone)]
struct StyledLine {
    text: String,
    is_bold: bool,
    is_italic: bool,
}

#[derive(Debug, Clone)]
struct MermaidNode {
    id: String,
    raw_label: String,
    lines: Vec<StyledLine>,
    is_diamond: bool,
    w: f64,
    h: f64,
}

#[derive(Debug, Clone)]
struct MermaidEdge {
    from: String,
    to: String,
    label: Option<Vec<String>>,
    dashed: bool,
}

#[derive(Debug, Clone)]
struct MermaidSubgraph {
    id: String,
    title_lines: Vec<String>,
    node_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn cx(self) -> f64 {
        self.x + self.w * 0.5
    }

    fn cy(self) -> f64 {
        self.y + self.h * 0.5
    }

    fn right(self) -> f64 {
        self.x + self.w
    }

    fn bottom(self) -> f64 {
        self.y + self.h
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

fn fnv1a_32(text: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in text.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn strip_inline_html_tags(input: &str) -> (String, bool, bool) {
    let is_bold = input.contains("<b>")
        || input.contains("<strong>")
        || input.contains("<B>")
        || input.contains("<STRONG>");
    let is_italic = input.contains("<i>")
        || input.contains("<em>")
        || input.contains("<I>")
        || input.contains("<EM>");
    let cleaned = input
        .replace("<b>", "")
        .replace("</b>", "")
        .replace("<strong>", "")
        .replace("</strong>", "")
        .replace("<i>", "")
        .replace("</i>", "")
        .replace("<em>", "")
        .replace("</em>", "")
        .replace("<B>", "")
        .replace("</B>", "")
        .replace("<STRONG>", "")
        .replace("</STRONG>", "")
        .replace("<I>", "")
        .replace("</I>", "")
        .replace("<EM>", "")
        .replace("</EM>", "");
    (cleaned, is_bold, is_italic)
}

fn wrap_plain_line(text: &str, max_chars: usize) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.chars().count() <= max_chars {
        return vec![trimmed.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in trimmed.split_whitespace() {
        let cur_len = cur.chars().count();
        let word_len = word.chars().count();
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur_len + 1 + word_len <= max_chars {
            cur.push(' ');
            cur.push_str(word);
        } else {
            out.push(cur);
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn normalize_newlines(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim()
        .replace("\\n", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n")
        .replace("<BR/>", "\n")
        .replace("<BR />", "\n")
        .replace("<BR>", "\n")
}

fn parse_node_lines(raw: &str, max_chars: usize) -> Vec<StyledLine> {
    let normalized = normalize_newlines(raw);
    let mut out = Vec::new();
    for part in normalized.split('\n') {
        let (plain, is_bold, is_italic) = strip_inline_html_tags(part);
        for wrapped in wrap_plain_line(&plain, max_chars) {
            out.push(StyledLine {
                text: wrapped,
                is_bold,
                is_italic,
            });
        }
    }
    if out.len() >= 2 && !out.iter().any(|l| l.is_bold) {
        out[0].is_bold = true;
    }
    if out.is_empty() {
        out.push(StyledLine {
            text: raw.trim().to_string(),
            is_bold: false,
            is_italic: false,
        });
    }
    out
}

fn parse_label_plain_lines(raw: &str, max_chars: usize) -> Vec<String> {
    let normalized = normalize_newlines(raw);
    let mut out = Vec::new();
    for part in normalized.split('\n') {
        let (plain, _, _) = strip_inline_html_tags(part);
        out.extend(wrap_plain_line(&plain, max_chars));
    }
    out
}

fn measure_node_box(lines: &[StyledLine], is_diamond: bool) -> (f64, f64) {
    let line_h = 14.5_f64;
    let pad_x = 13.0_f64;
    let pad_y = 9.0_f64;
    let max_line_px = lines
        .iter()
        .map(|l| {
            let cw = if l.is_bold { 6.5 } else { 6.0 };
            l.text.chars().count() as f64 * cw
        })
        .fold(0.0_f64, f64::max);
    let mut w = (max_line_px + 2.0 * pad_x).clamp(112.0, 268.0);
    let mut h = (lines.len() as f64 * line_h + 2.0 * pad_y).max(36.0);
    if is_diamond {
        w = (w * 1.32).max(134.0);
        h = (h * 1.36).max(52.0);
    }
    (w, h)
}

fn parse_mermaid_node_token(token: &str, max_chars: usize) -> Option<(String, Option<MermaidNode>)> {
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
        let lines = parse_node_lines(inner, max_chars);
        let (w, h) = measure_node_box(&lines, is_diamond);
        return Some((
            id.clone(),
            Some(MermaidNode {
                id,
                raw_label: inner.to_string(),
                lines,
                is_diamond,
                w,
                h,
            }),
        ));
    }
    if t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some((t.to_string(), None));
    }
    None
}

type MermaidEdgeStep = (Option<Vec<String>>, bool, String);

fn split_mermaid_edge_chain(line: &str) -> Option<(String, Vec<MermaidEdgeStep>)> {
    let chars: Vec<char> = line.chars().collect();
    let mut parts: Vec<MermaidEdgeStep> = Vec::new();
    let mut first_token: Option<String> = None;
    let mut seg_start = 0usize;
    let mut pending_edge: Option<(Option<Vec<String>>, bool)> = None;
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
                    let parsed = parse_label_plain_lines(&raw_lbl, 24);
                    if !parsed.is_empty() {
                        edge_lbl = Some(parsed);
                    }
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
                let parsed = parse_label_plain_lines(raw_lbl, 24);
                pending_edge = Some(((!parsed.is_empty()).then_some(parsed), false));
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

struct DagLayout {
    rects: Vec<Rect>,
    is_back: Vec<bool>,
    width: f64,
    height: f64,
}

fn dfs_mark_back_edges(
    u: usize,
    adj: &[Vec<(usize, usize)>],
    state: &mut [u8],
    is_back: &mut [bool],
) {
    state[u] = 1;
    for &(v, e_idx) in &adj[u] {
        if state[v] == 1 {
            is_back[e_idx] = true;
        } else if state[v] == 0 {
            dfs_mark_back_edges(v, adj, state, is_back);
        }
    }
    state[u] = 2;
}

fn layout_dag(
    sizes: &[(f64, f64)],
    edges: &[(usize, usize, bool)],
    is_lr: bool,
) -> DagLayout {
    let n = sizes.len();
    if n == 0 {
        return DagLayout {
            rects: Vec::new(),
            is_back: Vec::new(),
            width: 0.0,
            height: 0.0,
        };
    }

    // Special case: disconnected list of items (e.g. enumerations inside a subgraph).
    if edges.is_empty() && n > 1 {
        let gap = 10.0;
        let max_w = sizes.iter().map(|s| s.0).fold(0.0_f64, f64::max);
        let mut rects = Vec::with_capacity(n);
        let mut y = 0.0;
        for &(w, h) in sizes {
            rects.push(Rect {
                x: (max_w - w) * 0.5,
                y,
                w,
                h,
            });
            y += h + gap;
        }
        return DagLayout {
            rects,
            is_back: Vec::new(),
            width: max_w,
            height: (y - gap).max(0.0),
        };
    }

    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    let mut in_deg = vec![0usize; n];
    for (e_idx, &(u, v, _)) in edges.iter().enumerate() {
        if u < n && v < n {
            adj[u].push((v, e_idx));
            in_deg[v] += 1;
        }
    }

    let mut state = vec![0u8; n];
    let mut is_back = vec![false; edges.len()];
    for i in 0..n {
        if in_deg[i] == 0 && state[i] == 0 {
            dfs_mark_back_edges(i, &adj, &mut state, &mut is_back);
        }
    }
    for i in 0..n {
        if state[i] == 0 {
            dfs_mark_back_edges(i, &adj, &mut state, &mut is_back);
        }
    }

    let mut layer = vec![0usize; n];
    for _ in 0..n {
        let mut changed = false;
        for (e_idx, &(u, v, _)) in edges.iter().enumerate() {
            if !is_back[e_idx] && u < n && v < n && layer[v] <= layer[u] {
                layer[v] = layer[u] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let num_layers = layer.iter().copied().max().unwrap_or(0) + 1;
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); num_layers];
    for (idx, &l) in layer.iter().enumerate() {
        layers[l].push(idx);
    }

    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (e_idx, &(u, v, _)) in edges.iter().enumerate() {
        if !is_back[e_idx] && u < n && v < n {
            preds[v].push(u);
            succs[u].push(v);
        }
    }

    // 4-pass barycenter crossing reduction.
    for pass in 0..4 {
        let mut pos_in_layer = vec![0usize; n];
        for grp in &layers {
            for (pos, &nid) in grp.iter().enumerate() {
                pos_in_layer[nid] = pos;
            }
        }
        if pass % 2 == 0 {
            for (l, grp) in layers.iter_mut().enumerate().skip(1) {
                let mut keyed: Vec<(f64, usize, usize)> = grp
                    .iter()
                    .enumerate()
                    .map(|(cur_pos, &nid)| {
                        let matching: Vec<usize> = preds[nid]
                            .iter()
                            .copied()
                            .filter(|&p| layer[p] + 1 == l)
                            .map(|p| pos_in_layer[p])
                            .collect();
                        let key = if matching.is_empty() {
                            cur_pos as f64
                        } else {
                            matching.iter().sum::<usize>() as f64 / matching.len() as f64
                        };
                        (key, nid, nid)
                    })
                    .collect();
                keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
                *grp = keyed.into_iter().map(|t| t.2).collect();
            }
        } else {
            for l in (0..num_layers.saturating_sub(1)).rev() {
                let mut keyed: Vec<(f64, usize, usize)> = layers[l]
                    .iter()
                    .enumerate()
                    .map(|(cur_pos, &nid)| {
                        let matching: Vec<usize> = succs[nid]
                            .iter()
                            .copied()
                            .filter(|&s| layer[s] == l + 1)
                            .map(|s| pos_in_layer[s])
                            .collect();
                        let key = if matching.is_empty() {
                            cur_pos as f64
                        } else {
                            matching.iter().sum::<usize>() as f64 / matching.len() as f64
                        };
                        (key, nid, nid)
                    })
                    .collect();
                keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
                layers[l] = keyed.into_iter().map(|t| t.2).collect();
            }
        }
    }

    let mut layer_has_label = vec![false; num_layers];
    for (e_idx, &(u, _, has_lbl)) in edges.iter().enumerate() {
        if !is_back[e_idx] && has_lbl && u < n {
            layer_has_label[layer[u]] = true;
        }
    }

    let mut rects = vec![Rect::default(); n];
    if !is_lr {
        let col_gap = 22.0;
        let layer_widths: Vec<f64> = layers
            .iter()
            .map(|grp| {
                let sum_w: f64 = grp.iter().map(|&id| sizes[id].0).sum();
                sum_w + col_gap * grp.len().saturating_sub(1) as f64
            })
            .collect();
        let total_w = layer_widths.iter().copied().fold(0.0_f64, f64::max);
        let mut cur_y = 0.0;
        for (l, grp) in layers.iter().enumerate() {
            let max_h = grp.iter().map(|&id| sizes[id].1).fold(0.0_f64, f64::max);
            let mut cur_x = (total_w - layer_widths[l]) * 0.5;
            for &id in grp {
                let (w, h) = sizes[id];
                rects[id] = Rect {
                    x: cur_x,
                    y: cur_y + (max_h - h) * 0.5,
                    w,
                    h,
                };
                cur_x += w + col_gap;
            }
            let row_gap = if layer_has_label[l] { 54.0 } else { 40.0 };
            cur_y += max_h + if l + 1 < num_layers { row_gap } else { 0.0 };
        }
        DagLayout {
            rects,
            is_back,
            width: total_w,
            height: cur_y,
        }
    } else {
        let row_gap = 18.0;
        let layer_heights: Vec<f64> = layers
            .iter()
            .map(|grp| {
                let sum_h: f64 = grp.iter().map(|&id| sizes[id].1).sum();
                sum_h + row_gap * grp.len().saturating_sub(1) as f64
            })
            .collect();
        let total_h = layer_heights.iter().copied().fold(0.0_f64, f64::max);
        let mut cur_x = 0.0;
        for (l, grp) in layers.iter().enumerate() {
            let max_w = grp.iter().map(|&id| sizes[id].0).fold(0.0_f64, f64::max);
            let mut cur_y = (total_h - layer_heights[l]) * 0.5;
            for &id in grp {
                let (w, h) = sizes[id];
                rects[id] = Rect {
                    x: cur_x + (max_w - w) * 0.5,
                    y: cur_y,
                    w,
                    h,
                };
                cur_y += h + row_gap;
            }
            let col_gap = if layer_has_label[l] { 76.0 } else { 52.0 };
            cur_x += max_w + if l + 1 < num_layers { col_gap } else { 0.0 };
        }
        DagLayout {
            rects,
            is_back,
            width: cur_x,
            height: total_h,
        }
    }
}

fn port_point(rect: Rect, side: Side, frac: f64, is_diamond: bool) -> (f64, f64) {
    let f = frac.clamp(0.18, 0.82);
    if !is_diamond {
        return match side {
            Side::Top => (rect.x + rect.w * f, rect.y),
            Side::Bottom => (rect.x + rect.w * f, rect.bottom()),
            Side::Left => (rect.x, rect.y + rect.h * f),
            Side::Right => (rect.right(), rect.y + rect.h * f),
        };
    }
    let cx = rect.cx();
    let cy = rect.cy();
    let hw = rect.w * 0.5;
    let hh = rect.h * 0.5;
    match side {
        Side::Top => {
            let dx = (f - 0.5) * rect.w * 0.45;
            let dy = hh * (1.0 - (dx.abs() / hw).min(0.9));
            (cx + dx, cy - dy)
        }
        Side::Bottom => {
            let dx = (f - 0.5) * rect.w * 0.45;
            let dy = hh * (1.0 - (dx.abs() / hw).min(0.9));
            (cx + dx, cy + dy)
        }
        Side::Left => {
            let dy = (f - 0.5) * rect.h * 0.45;
            let dx = hw * (1.0 - (dy.abs() / hh).min(0.9));
            (cx - dx, cy + dy)
        }
        Side::Right => {
            let dy = (f - 0.5) * rect.h * 0.45;
            let dx = hw * (1.0 - (dy.abs() / hh).min(0.9));
            (cx + dx, cy + dy)
        }
    }
}

struct RoutedEdge {
    path_d: String,
    dashed: bool,
    is_back: bool,
    label_lines: Option<Vec<String>>,
    label_pos: (f64, f64),
    bounds: (f64, f64, f64, f64),
}

fn cubic_midpoint(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> (f64, f64) {
    (
        0.125 * p0.0 + 0.375 * p1.0 + 0.375 * p2.0 + 0.125 * p3.0,
        0.125 * p0.1 + 0.375 * p1.1 + 0.375 * p2.1 + 0.125 * p3.1,
    )
}

/// Renders a Mermaid `flowchart LR` or `flowchart TD` diagram block into a self-contained,
/// theme-aware inline SVG flowchart diagram.
pub fn render_mermaid_to_html(diagram_src: &str) -> String {
    let mut is_lr = false;
    for raw_line in diagram_src.lines() {
        let trimmed = raw_line.trim();
        if let Some(dir) = trimmed
            .strip_prefix("flowchart ")
            .or_else(|| trimmed.strip_prefix("graph "))
        {
            is_lr = dir.trim().eq_ignore_ascii_case("LR") || dir.trim().eq_ignore_ascii_case("RL");
            break;
        }
    }
    let wrap_chars = if is_lr { 30 } else { 36 };

    let mut nodes: Vec<MermaidNode> = Vec::new();
    let mut node_index: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<MermaidEdge> = Vec::new();
    let mut subgraphs: Vec<MermaidSubgraph> = Vec::new();
    let mut active_subgraph: Option<usize> = None;

    let upsert_node = |id: String,
                       node_opt: Option<MermaidNode>,
                       active_sg: Option<usize>,
                       nodes: &mut Vec<MermaidNode>,
                       node_index: &mut HashMap<String, usize>,
                       subgraphs: &mut [MermaidSubgraph]| {
        if let Some(&existing_idx) = node_index.get(&id) {
            if let Some(parsed) = node_opt {
                nodes[existing_idx] = parsed;
            }
        } else {
            let parsed = node_opt.unwrap_or_else(|| {
                let lines = parse_node_lines(&id, wrap_chars);
                let (w, h) = measure_node_box(&lines, false);
                MermaidNode {
                    id: id.clone(),
                    raw_label: id.clone(),
                    lines,
                    is_diamond: false,
                    w,
                    h,
                }
            });
            let new_idx = nodes.len();
            nodes.push(parsed);
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
        if trimmed.is_empty()
            || trimmed.starts_with("%%")
            || trimmed.starts_with("flowchart ")
            || trimmed.starts_with("graph ")
            || trimmed.starts_with("style ")
            || trimmed.starts_with("classDef ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("linkStyle ")
        {
            continue;
        }
        if let Some(sg_rest) = trimmed.strip_prefix("subgraph ") {
            let sg_trim = sg_rest.trim();
            let (sg_id, title_raw) = if let Some(open_br) = sg_trim.find('[') {
                let id_part = sg_trim[..open_br].trim().to_string();
                let title_part = sg_trim[open_br + 1..].trim_end_matches(']').trim();
                (id_part, title_part)
            } else {
                (sg_trim.to_string(), sg_trim)
            };
            let title_lines = parse_label_plain_lines(title_raw, 40);
            let sg_idx = subgraphs.len();
            subgraphs.push(MermaidSubgraph {
                id: sg_id,
                title_lines,
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
            let mut prev_id = if let Some((id, node_opt)) =
                parse_mermaid_node_token(&first_tok, wrap_chars)
            {
                let is_known_sg = node_opt.is_none() && subgraphs.iter().any(|sg| sg.id == id);
                if !is_known_sg {
                    upsert_node(
                        id.clone(),
                        node_opt,
                        active_subgraph,
                        &mut nodes,
                        &mut node_index,
                        &mut subgraphs,
                    );
                }
                id
            } else {
                first_tok
            };

            for (edge_lbl, is_dashed, target_tok) in chain {
                let target_id = if let Some((id, node_opt)) =
                    parse_mermaid_node_token(&target_tok, wrap_chars)
                {
                    let is_known_sg = node_opt.is_none() && subgraphs.iter().any(|sg| sg.id == id);
                    if !is_known_sg {
                        upsert_node(
                            id.clone(),
                            node_opt,
                            active_subgraph,
                            &mut nodes,
                            &mut node_index,
                            &mut subgraphs,
                        );
                    }
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

        if let Some((id, node_opt)) = parse_mermaid_node_token(trimmed, wrap_chars) {
            upsert_node(
                id,
                node_opt,
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

    // Build layout positions for every node (and every subgraph bounding box).
    let mut node_rects: HashMap<String, Rect> = HashMap::new();
    let mut sg_rects: Vec<Rect> = Vec::with_capacity(subgraphs.len());
    let mut sg_header_heights: Vec<f64> = Vec::with_capacity(subgraphs.len());
    let mut flow_lr = is_lr;
    let mut edge_is_back = vec![false; edges.len()];

    if subgraphs.is_empty() {
        let sizes: Vec<(f64, f64)> = nodes.iter().map(|n| (n.w, n.h)).collect();
        let edge_idx_pairs: Vec<(usize, usize, bool)> = edges
            .iter()
            .filter_map(|e| {
                Some((
                    *node_index.get(&e.from)?,
                    *node_index.get(&e.to)?,
                    e.label.is_some(),
                ))
            })
            .collect();
        let layout = layout_dag(&sizes, &edge_idx_pairs, is_lr);
        for (i, n) in nodes.iter().enumerate() {
            node_rects.insert(n.id.clone(), layout.rects[i]);
        }
        edge_is_back = layout.is_back;
    } else {
        let sg_id_to_idx: HashMap<&str, usize> = subgraphs
            .iter()
            .enumerate()
            .map(|(i, sg)| (sg.id.as_str(), i))
            .collect();
        let mut node_to_sg: HashMap<&str, usize> = HashMap::new();
        for (sg_idx, sg) in subgraphs.iter().enumerate() {
            for nid in &sg.node_ids {
                node_to_sg.insert(nid.as_str(), sg_idx);
            }
        }

        let sg_pad = 12.0;
        let mut sg_internal_layouts: Vec<DagLayout> = Vec::with_capacity(subgraphs.len());
        let mut sg_sizes: Vec<(f64, f64)> = Vec::with_capacity(subgraphs.len());

        for sg in &subgraphs {
            let local_map: HashMap<&str, usize> = sg
                .node_ids
                .iter()
                .enumerate()
                .map(|(i, id)| (id.as_str(), i))
                .collect();
            let local_sizes: Vec<(f64, f64)> = sg
                .node_ids
                .iter()
                .filter_map(|id| node_index.get(id).map(|&idx| (nodes[idx].w, nodes[idx].h)))
                .collect();
            let local_edges: Vec<(usize, usize, bool)> = edges
                .iter()
                .filter_map(|e| {
                    let u = *local_map.get(e.from.as_str())?;
                    let v = *local_map.get(e.to.as_str())?;
                    Some((u, v, e.label.is_some()))
                })
                .collect();
            let internal = layout_dag(&local_sizes, &local_edges, false);
            let title_lines_n = sg.title_lines.len().max(1);
            let header_h = 12.0 + title_lines_n as f64 * 14.0;
            let title_w = sg
                .title_lines
                .iter()
                .map(|l| l.chars().count() as f64 * 6.8)
                .fold(0.0_f64, f64::max)
                + 24.0;
            let box_w = internal.width.max(title_w) + 2.0 * sg_pad;
            let box_h = header_h + internal.height + 2.0 * sg_pad;
            sg_header_heights.push(header_h);
            sg_sizes.push((box_w, box_h));
            sg_internal_layouts.push(internal);
        }

        let standalone_nodes: Vec<usize> = (0..nodes.len())
            .filter(|&i| !node_to_sg.contains_key(nodes[i].id.as_str()))
            .collect();
        let standalone_super_map: HashMap<&str, usize> = standalone_nodes
            .iter()
            .enumerate()
            .map(|(k, &nidx)| (nodes[nidx].id.as_str(), subgraphs.len() + k))
            .collect();

        let mut super_sizes = sg_sizes.clone();
        for &nidx in &standalone_nodes {
            super_sizes.push((nodes[nidx].w, nodes[nidx].h));
        }

        let resolve_super = |id: &str| -> Option<usize> {
            sg_id_to_idx
                .get(id)
                .copied()
                .or_else(|| node_to_sg.get(id).copied())
                .or_else(|| standalone_super_map.get(id).copied())
        };

        let mut super_edges: Vec<(usize, usize, bool)> = Vec::new();
        let mut seen_super: HashSet<(usize, usize)> = HashSet::new();
        for e in &edges {
            if let (Some(su), Some(sv)) = (resolve_super(&e.from), resolve_super(&e.to))
                && su != sv
                && seen_super.insert((su, sv))
            {
                super_edges.push((su, sv, e.label.is_some()));
            }
        }

        let avg_sg_w =
            sg_sizes.iter().map(|s| s.0).sum::<f64>() / sg_sizes.len().max(1) as f64;
        let avg_sg_h =
            sg_sizes.iter().map(|s| s.1).sum::<f64>() / sg_sizes.len().max(1) as f64;
        flow_lr = is_lr || (subgraphs.len() >= 2 && avg_sg_h > avg_sg_w * 1.05);

        let outer = layout_dag(&super_sizes, &super_edges, flow_lr);
        for sg_idx in 0..subgraphs.len() {
            let sg_r = outer.rects[sg_idx];
            sg_rects.push(sg_r);
            let internal = &sg_internal_layouts[sg_idx];
            let inner_offset_x = sg_r.x + (sg_r.w - internal.width) * 0.5;
            let inner_offset_y = sg_r.y + sg_header_heights[sg_idx] + sg_pad;
            for (local_i, nid) in subgraphs[sg_idx].node_ids.iter().enumerate() {
                if let Some(r) = internal.rects.get(local_i) {
                    node_rects.insert(
                        nid.clone(),
                        Rect {
                            x: inner_offset_x + r.x,
                            y: inner_offset_y + r.y,
                            w: r.w,
                            h: r.h,
                        },
                    );
                }
            }
        }
        for (k, &nidx) in standalone_nodes.iter().enumerate() {
            node_rects.insert(nodes[nidx].id.clone(), outer.rects[subgraphs.len() + k]);
        }
        // Register subgraph bounding boxes for subgraph-ID edges.
        for (sg_idx, sg) in subgraphs.iter().enumerate() {
            node_rects.entry(sg.id.clone()).or_insert(sg_rects[sg_idx]);
        }

        // Determine back-edges on the full graph via actual resolved positions.
        for (idx, e) in edges.iter().enumerate() {
            if let (Some(sr), Some(dr)) = (node_rects.get(&e.from), node_rects.get(&e.to)) {
                let backwards = if flow_lr {
                    dr.cx() < sr.cx() - 4.0
                } else {
                    dr.cy() < sr.cy() - 4.0
                };
                edge_is_back[idx] = backwards;
            }
        }
    }

    let is_diamond_id = |id: &str| -> bool {
        node_index
            .get(id)
            .is_some_and(|&idx| nodes[idx].is_diamond)
    };

    // Determine edge exit/entry sides and distribute port slots evenly.
    #[derive(Clone, Copy)]
    struct EdgeSidePlan {
        src_side: Side,
        dst_side: Side,
        is_back: bool,
    }
    let mut plans: Vec<Option<EdgeSidePlan>> = vec![None; edges.len()];
    let mut out_groups: HashMap<(String, Side), Vec<(usize, f64)>> = HashMap::new();
    let mut in_groups: HashMap<(String, Side), Vec<(usize, f64)>> = HashMap::new();

    for (idx, e) in edges.iter().enumerate() {
        let (Some(&sr), Some(&dr)) = (node_rects.get(&e.from), node_rects.get(&e.to)) else {
            continue;
        };
        let is_back = edge_is_back[idx];
        let (src_side, dst_side) = if is_back {
            if flow_lr {
                (Side::Bottom, Side::Bottom)
            } else {
                (Side::Right, Side::Right)
            }
        } else if flow_lr {
            if dr.x >= sr.right() - 8.0 {
                (Side::Right, Side::Left)
            } else if dr.cy() >= sr.cy() {
                (Side::Bottom, Side::Top)
            } else {
                (Side::Top, Side::Bottom)
            }
        } else if dr.y >= sr.bottom() - 8.0 {
            (Side::Bottom, Side::Top)
        } else if dr.cx() >= sr.cx() {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        };

        plans[idx] = Some(EdgeSidePlan {
            src_side,
            dst_side,
            is_back,
        });
        let dst_orth = match src_side {
            Side::Top | Side::Bottom => dr.cx(),
            Side::Left | Side::Right => dr.cy(),
        };
        let src_orth = match dst_side {
            Side::Top | Side::Bottom => sr.cx(),
            Side::Left | Side::Right => sr.cy(),
        };
        out_groups
            .entry((e.from.clone(), src_side))
            .or_default()
            .push((idx, dst_orth));
        in_groups
            .entry((e.to.clone(), dst_side))
            .or_default()
            .push((idx, src_orth));
    }

    let mut out_frac = vec![0.5_f64; edges.len()];
    let mut in_frac = vec![0.5_f64; edges.len()];
    for group in out_groups.values_mut() {
        group.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        let m = group.len();
        for (k, &(idx, _)) in group.iter().enumerate() {
            out_frac[idx] = (k + 1) as f64 / (m + 1) as f64;
        }
    }
    for group in in_groups.values_mut() {
        group.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        let m = group.len();
        for (k, &(idx, _)) in group.iter().enumerate() {
            in_frac[idx] = (k + 1) as f64 / (m + 1) as f64;
        }
    }

    let mut content_max_x = 0.0_f64;
    let mut content_max_y = 0.0_f64;
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for r in node_rects.values().chain(sg_rects.iter()) {
        min_x = min_x.min(r.x);
        min_y = min_y.min(r.y);
        content_max_x = content_max_x.max(r.right());
        content_max_y = content_max_y.max(r.bottom());
    }
    if !min_x.is_finite() {
        min_x = 0.0;
        min_y = 0.0;
    }

    let mut routed_edges: Vec<RoutedEdge> = Vec::with_capacity(edges.len());
    let mut back_counter = 0usize;

    for (idx, e) in edges.iter().enumerate() {
        let Some(plan) = plans[idx] else {
            continue;
        };
        let sr = node_rects[&e.from];
        let dr = node_rects[&e.to];
        let (x1, y1) = port_point(sr, plan.src_side, out_frac[idx], is_diamond_id(&e.from));
        let (mut x2, mut y2) = port_point(dr, plan.dst_side, in_frac[idx], is_diamond_id(&e.to));

        // Pull endpoint back ~2px along approach vector so the arrowhead sits flush on the border.
        match plan.dst_side {
            Side::Top => y2 -= 2.0,
            Side::Bottom => y2 += 2.0,
            Side::Left => x2 -= 2.0,
            Side::Right => x2 += 2.0,
        }

        let (p1, p2, bounds) = if plan.is_back {
            let slot = back_counter;
            back_counter += 1;
            if flow_lr {
                let by = content_max_y + 28.0 + slot as f64 * 18.0;
                (
                    (x1, by),
                    (x2, by),
                    (x1.min(x2), y1.min(y2), x1.max(x2), by),
                )
            } else {
                let rx = content_max_x + 28.0 + slot as f64 * 18.0;
                (
                    (rx, y1),
                    (rx, y2),
                    (x1.min(x2), y1.min(y2), rx, y1.max(y2)),
                )
            }
        } else {
            match (plan.src_side, plan.dst_side) {
                (Side::Bottom, Side::Top) | (Side::Top, Side::Bottom) => {
                    let ym = (y1 + y2) * 0.5;
                    (
                        (x1, ym),
                        (x2, ym),
                        (x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2)),
                    )
                }
                _ => {
                    let xm = (x1 + x2) * 0.5;
                    (
                        (xm, y1),
                        (xm, y2),
                        (x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2)),
                    )
                }
            }
        };

        let path_d = format!(
            "M {:.1},{:.1} C {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
            x1, y1, p1.0, p1.1, p2.0, p2.1, x2, y2
        );
        let label_pos = cubic_midpoint((x1, y1), p1, p2, (x2, y2));
        let mut eb = bounds;
        if let Some(ref lbl) = e.label {
            let lw = lbl
                .iter()
                .map(|l| l.chars().count() as f64 * 5.6)
                .fold(0.0_f64, f64::max)
                + 14.0;
            let lh = lbl.len() as f64 * 12.0 + 6.0;
            eb.0 = eb.0.min(label_pos.0 - lw * 0.5);
            eb.1 = eb.1.min(label_pos.1 - lh * 0.5);
            eb.2 = eb.2.max(label_pos.0 + lw * 0.5);
            eb.3 = eb.3.max(label_pos.1 + lh * 0.5);
        }

        routed_edges.push(RoutedEdge {
            path_d,
            dashed: e.dashed,
            is_back: plan.is_back,
            label_lines: e.label.clone(),
            label_pos,
            bounds: eb,
        });
    }

    let mut max_x = content_max_x;
    let mut max_y = content_max_y;
    for re in &routed_edges {
        min_x = min_x.min(re.bounds.0);
        min_y = min_y.min(re.bounds.1);
        max_x = max_x.max(re.bounds.2);
        max_y = max_y.max(re.bounds.3);
    }

    let margin = 14.0;
    let shift_x = margin - min_x;
    let shift_y = margin - min_y;
    let svg_w = ((max_x - min_x) + 2.0 * margin).max(160.0);
    let svg_h = ((max_y - min_y) + 2.0 * margin).max(80.0);

    let hash = fnv1a_32(diagram_src);
    let arr_id = format!("m-arr-{hash:08x}");
    let darr_id = format!("m-darr-{hash:08x}");
    let dir_class = if flow_lr {
        "mermaid-dir-lr"
    } else {
        "mermaid-dir-td"
    };

    let mut html = String::with_capacity(4096);
    let _ = write!(
        html,
        "<div class=\"mermaid rules-mermaid {dir_class}\" role=\"figure\" aria-label=\"Flowchart Diagram\">\
         <svg class=\"mermaid-svg\" viewBox=\"0 0 {svg_w:.1} {svg_h:.1}\" width=\"{svg_w:.0}\" height=\"{svg_h:.0}\" \
         xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-label=\"Flowchart Diagram\">\
         <defs>\
         <marker id=\"{arr_id}\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" markerWidth=\"6.5\" markerHeight=\"6.5\" orient=\"auto-start-reverse\">\
         <path d=\"M 0 1.5 L 9 5 L 0 8.5 z\" class=\"mermaid-arrow-head\"/>\
         </marker>\
         <marker id=\"{darr_id}\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" markerWidth=\"6.5\" markerHeight=\"6.5\" orient=\"auto-start-reverse\">\
         <path d=\"M 0 1.5 L 9 5 L 0 8.5 z\" class=\"mermaid-arrow-head mermaid-arrow-head--dashed\"/>\
         </marker>\
         </defs>\
         <g transform=\"translate({shift_x:.1},{shift_y:.1})\">"
    );

    // Layer 1: Subgraph background boxes.
    for (sg_idx, sg) in subgraphs.iter().enumerate() {
        let r = sg_rects[sg_idx];
        let hh = sg_header_heights[sg_idx];
        let _ = write!(
            html,
            "<g class=\"mermaid-subgraph\">\
             <rect class=\"mermaid-subgraph-box\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"6\"/>\
             <rect class=\"mermaid-subgraph-header\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"6\"/>\
             <line class=\"mermaid-subgraph-sep\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\"/>",
            r.x,
            r.y,
            r.w,
            r.h,
            r.x,
            r.y,
            r.w,
            hh,
            r.x,
            r.y + hh,
            r.right(),
            r.y + hh,
        );
        let text_y = r.y + 16.0;
        let _ = write!(
            html,
            "<text class=\"mermaid-subgraph-label\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">",
            r.cx(),
            text_y
        );
        for (li, line) in sg.title_lines.iter().enumerate() {
            let dy = if li == 0 { 0.0 } else { 13.0 };
            let _ = write!(
                html,
                "<tspan x=\"{:.1}\" dy=\"{:.1}\">{}</tspan>",
                r.cx(),
                dy,
                escape_html_text(line)
            );
        }
        html.push_str("</text></g>");
    }

    // Layer 2: Edge paths.
    for re in &routed_edges {
        let mut cls = String::from("mermaid-edge");
        if re.dashed {
            cls.push_str(" mermaid-edge--dashed");
        }
        if re.is_back {
            cls.push_str(" mermaid-edge--back");
        }
        let marker = if re.dashed { &darr_id } else { &arr_id };
        let _ = write!(
            html,
            "<path class=\"{cls}\" d=\"{}\" fill=\"none\" marker-end=\"url(#{marker})\"/>",
            re.path_d
        );
    }

    // Layer 3: Nodes (boxes / diamonds + wrapped text).
    for node in &nodes {
        let Some(&r) = node_rects.get(&node.id) else {
            continue;
        };
        let cls = if node.is_diamond {
            "mermaid-node mermaid-node-diamond"
        } else {
            "mermaid-node"
        };
        let _ = write!(html, "<g class=\"{cls}\">");
        if node.is_diamond {
            let pts = format!(
                "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                r.cx(),
                r.y,
                r.right(),
                r.cy(),
                r.cx(),
                r.bottom(),
                r.x,
                r.cy()
            );
            let _ = write!(html, "<polygon class=\"mermaid-node-box\" points=\"{pts}\"/>");
        } else {
            let _ = write!(
                html,
                "<rect class=\"mermaid-node-box\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"5\"/>\
                 <rect class=\"mermaid-node-accent\" x=\"{:.1}\" y=\"{:.1}\" width=\"3.5\" height=\"{:.1}\" rx=\"2\"/>",
                r.x, r.y, r.w, r.h, r.x, r.y, r.h
            );
        }

        let line_h = 14.5_f64;
        let total_text_h = node.lines.len() as f64 * line_h;
        let start_y = r.cy() - total_text_h * 0.5 + 11.0;
        let _ = write!(
            html,
            "<text class=\"mermaid-node-text\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">",
            r.cx(),
            start_y
        );
        for (li, line) in node.lines.iter().enumerate() {
            let dy = if li == 0 { 0.0 } else { line_h };
            let tspan_cls = match (line.is_bold, line.is_italic) {
                (true, true) => " class=\"mermaid-node-title mermaid-node-italic\"",
                (true, false) => " class=\"mermaid-node-title\"",
                (false, true) => " class=\"mermaid-node-italic\"",
                (false, false) => "",
            };
            let _ = write!(
                html,
                "<tspan{tspan_cls} x=\"{:.1}\" dy=\"{:.1}\">{}</tspan>",
                r.cx(),
                dy,
                escape_html_text(&line.text)
            );
        }
        html.push_str("</text></g>");
    }

    // Layer 4: Edge label pills rendered above edges & nodes.
    for re in &routed_edges {
        let Some(ref lbl) = re.label_lines else {
            continue;
        };
        let line_h = 12.0_f64;
        let lw = lbl
            .iter()
            .map(|l| l.chars().count() as f64 * 5.6)
            .fold(0.0_f64, f64::max)
            + 12.0;
        let lh = lbl.len() as f64 * line_h + 6.0;
        let lx = re.label_pos.0 - lw * 0.5;
        let ly = re.label_pos.1 - lh * 0.5;
        let text_y = ly + 11.5;
        let _ = write!(
            html,
            "<g class=\"mermaid-edge-label-pill\">\
             <rect class=\"mermaid-edge-bg\" x=\"{lx:.1}\" y=\"{ly:.1}\" width=\"{lw:.1}\" height=\"{lh:.1}\" rx=\"4\"/>\
             <text class=\"mermaid-edge-text\" x=\"{:.1}\" y=\"{text_y:.1}\" text-anchor=\"middle\">",
            re.label_pos.0
        );
        for (li, line) in lbl.iter().enumerate() {
            let dy = if li == 0 { 0.0 } else { line_h };
            let _ = write!(
                html,
                "<tspan x=\"{:.1}\" dy=\"{dy:.1}\">{}</tspan>",
                re.label_pos.0,
                escape_html_text(line)
            );
        }
        html.push_str("</text></g>");
    }

    let _ = raw_label_unused_guard(&nodes);
    html.push_str("</g></svg></div>");
    html
}

fn raw_label_unused_guard(nodes: &[MermaidNode]) -> usize {
    nodes.iter().map(|n| n.raw_label.len()).sum()
}
