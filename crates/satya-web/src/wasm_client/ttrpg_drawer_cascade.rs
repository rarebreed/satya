//! Satya Two-List Cascading ("Watershed") `2d20` Task Drawer: live exact Contested & Uncontested
//! combinatorial/DP probability previews, searchable canonical skill combobox (`#ttrpg-skill-dropdown`),
//! and `ROLL_TTRPG_TASK` WebSocket submission.

use crate::wasm_client::dom::{
    escape_html, escape_js_single_quoted_attr, get_element, input_value, select_value, set_display,
    set_html, set_input_value, set_text, toggle_element_display,
};
use crate::wasm_client::options::parse_combat_stance;
use crate::wasm_client::state::with_state;
use crate::wasm_client::webrtc::send_ws_packet;
use satya_schema::ClientWsPacket;
use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

/// Formats a concise summary pill for a [`satya_engine::CascadeTaskParams`] configuration,
/// displaying $\text{TN}_{\text{eff}}$, $\delta_{\text{under}}$, $P(\text{Succ} \ge 1)$, and $P(\text{Botch})$.
#[must_use]
pub fn format_cascade_odds_preview(params: &satya_engine::CascadeTaskParams) -> String {
    let odds = params.exact_odds();
    format!(
        "TN_eff {} (δ_under -{}) · P(Succ) {:.1}% · P(Botch) {:.1}%",
        params.tn_eff(),
        params.effective_delta_under(),
        odds.prob_any_success * 100.0,
        odds.prob_botch_failure * 100.0
    )
}

/// Computes and formats the live exact Contested (`1-Pair Tiered GM Resistance`) and Uncontested
/// Combinatorial & DP probability readout for the Satya Two-List Cascading ("Watershed") `2d20`
/// task drawer (`#ttrpg-odds-pill` and `#ttrpg-odds-readout`).
#[must_use]
pub fn format_contested_cascade_odds_summary(
    skill_level_n: usize,
    control_c: i32,
    delta_tool: i32,
    delta_ext: i32,
    delta_int: i32,
    carry_mode_raw: &str,
    botch_rule_raw: &str,
) -> (String, String) {
    use satya_engine::{
        BotchRule, CarryMode, CascadeTaskParams, ContestedCascadeTaskParams, ContestedTieRule,
        compute_stats,
    };
    use std::str::FromStr;

    let n = skill_level_n.clamp(1, 10);
    let c = control_c.clamp(1, 20);
    let d_tool = delta_tool.clamp(-4, 12);
    let d_ext = delta_ext.clamp(-10, 16);
    let d_int = delta_int.clamp(-2, 10);
    let carry_mode = CarryMode::from_str(carry_mode_raw).unwrap_or(CarryMode::MarkovStep);
    let botch_rule = BotchRule::from_str(botch_rule_raw).unwrap_or(BotchRule::ClosestMiss);

    let mut contested = ContestedCascadeTaskParams::new_tiered_environment(
        n,
        c,
        d_int,
        d_tool,
        d_ext,
        ContestedTieRule::HybridSignedMargin,
    );
    contested.player.carry_mode = carry_mode;
    contested.player.botch_rule = botch_rule;
    let c_odds = contested.exact_contested_odds();

    let mut uncontested = CascadeTaskParams::new_k20(n, c, d_int, d_tool + d_ext);
    uncontested.carry_mode = carry_mode;
    uncontested.botch_rule = botch_rule;
    let u_odds = uncontested.exact_odds();
    let u_stats = compute_stats(&u_odds.signed_margin_pmf);

    let tn_p = contested.player.tn_eff();
    let tn_gm = contested.gm.tn_eff();
    let tn_eff = uncontested.tn_eff();
    let p_win = c_odds.prob_total_win * 100.0;
    let p_succ = u_odds.prob_any_success * 100.0;

    let c_extra = c_odds.prob_extraordinary_success * 100.0;
    let c_clean = c_odds.prob_clean_success * 100.0;
    let c_graze = c_odds.prob_grazing_tie_partial * 100.0;
    let c_lucky = c_odds.prob_lucky_break_partial * 100.0;
    let c_breach = c_odds.prob_floor_breach_partial * 100.0;
    let c_partial = c_odds.prob_partial_success * 100.0;
    let c_fail = c_odds.prob_clean_failure * 100.0;
    let c_botch = c_odds.prob_botch_failure * 100.0;
    let c_mean = c_odds.net_margin_stats.mean;

    let u_comp = u_odds.prob_success_with_complication * 100.0;
    let u_clean_succ = (p_succ - u_comp).max(0.0);
    let u_fail = u_odds.prob_clean_failure * 100.0;
    let u_botch = u_odds.prob_botch_failure * 100.0;
    let u_mean = u_stats.mean;

    let pill_text = format!(
        "TN_P: {tn_p} vs TN_GM: {tn_gm} · P(Win): {p_win:.1}% · TN_eff: {tn_eff} · P(Succ≥1): {p_succ:.1}%"
    );

    let seg = |cls: &str, label: &str, pct: f64| -> String {
        if pct <= 0.05 {
            return String::new();
        }
        let txt = if pct >= 8.0 {
            format!("<span>{pct:.1}%</span>")
        } else {
            String::new()
        };
        format!(
            r#"<div class="ttrpg-odds-seg {cls}" style="width:{pct:.2}%" title="{label}: {pct:.2}%">{txt}</div>"#
        )
    };
    let bar_cell = |cls: &str, pct: f64| -> String {
        let w = pct.clamp(0.0, 100.0);
        format!(
            r#"<div class="ttrpg-odds-cell-bar"><span class="mono">{pct:.2}%</span><div class="ttrpg-odds-mini-track"><div class="ttrpg-odds-mini-fill {cls}" style="width:{w:.1}%"></div></div></div>"#
        )
    };

    let readout_html = format!(
        concat!(
            r#"<div class="ttrpg-odds-kpis">"#,
            r#"<div class="ttrpg-odds-kpi"><span class="kpi-label">Your Target (TN_P = {tn_p})</span><strong>Roll {tn_p_min}+ on 2d20</strong><span class="kpi-sub">31 - C {c} + Δ_tool {d_tool:+}</span></div>"#,
            r#"<div class="ttrpg-odds-kpi"><span class="kpi-label">GM / Hazard Target (TN_GM = {tn_gm})</span><strong>Roll {tn_gm_min}+ on 2d20</strong><span class="kpi-sub">21 - Δ_ext {d_ext:+} · Static TN_eff = {tn_eff}</span></div>"#,
            r#"<div class="ttrpg-odds-kpi"><span class="kpi-label">Cascade Carry ({n} × 2d20)</span><strong>+{d_over} Hit / -{d_under} Miss</strong><span class="kpi-sub">{carry_label} · {botch_label} ({carry_code}, {botch_code})</span></div>"#,
            r#"<div class="ttrpg-odds-kpi"><span class="kpi-label">Expected Net Margin</span><strong>E[ΔM]: {c_mean:+.2} vs GM</strong><span class="kpi-sub">Uncontested E[Margin]: {u_mean:+.2}</span></div>"#,
            r#"</div>"#,
            r#"<div class="ttrpg-odds-stacked-wrap">"#,
            r#"<div class="ttrpg-odds-stacked-header"><span><strong>Outcome Distribution Graph</strong> (Contested P(Win): {p_win:.2}%)</span><span class="kpi-sub">Extraordinary {c_extra:.1}% · Clean {c_clean:.1}% · Partial {c_partial:.1}% · Fail {c_fail:.1}% · Botch {c_botch:.1}%</span></div>"#,
            r#"<div class="ttrpg-odds-stacked-bar" role="img" aria-label="Outcome probability distribution bar">{seg_extra}{seg_clean}{seg_partial}{seg_fail}{seg_botch}</div>"#,
            r#"</div>"#,
            r#"<div class="metrics-table-wrap"><table class="metrics-table ttrpg-odds-table">"#,
            r#"<thead><tr><th>Outcome Tier</th><th>What It Means</th><th>Contested (vs. GM)</th><th>Uncontested (Static TN)</th></tr></thead>"#,
            r#"<tbody>"#,
            r#"<tr><td><span class="odds-dot seg-extraordinary"></span><strong>Extraordinary Win</strong></td><td class="kpi-sub">Beat GM by ≥3 net successes (-15% time, -4 next TN)</td><td>{cell_c_extra}</td><td><span class="kpi-sub">Included in Clean</span></td></tr>"#,
            r#"<tr><td><span class="odds-dot seg-clean"></span><strong>Clean Success</strong></td><td class="kpi-sub">Beat target cleanly with no complications</td><td>{cell_c_clean}</td><td>{cell_u_clean}</td></tr>"#,
            r#"<tr><td><span class="odds-dot seg-partial"></span><strong>Partial / Costly Win</strong></td><td class="kpi-sub">GrazingTie {c_graze:.1}% · LuckyBreak {c_lucky:.1}% · FloorBreach {c_breach:.1}% (P(Succ+Botch): {u_comp:.2}%)</td><td>{cell_c_partial}</td><td>{cell_u_comp}</td></tr>"#,
            r#"<tr><td><span class="odds-dot seg-fail"></span><strong>Clean Failure</strong></td><td class="kpi-sub">Miss target without triggering a botch (P(CleanFail): {u_fail:.2}%)</td><td>{cell_c_fail}</td><td>{cell_u_fail}</td></tr>"#,
            r#"<tr><td><span class="odds-dot seg-botch"></span><strong>Botch / Critical Fail</strong></td><td class="kpi-sub">Severe miss or &lt;2 floor breach (P(BotchFail): {u_botch:.2}%)</td><td>{cell_c_botch}</td><td>{cell_u_botch}</td></tr>"#,
            r#"</tbody>"#,
            r#"<tfoot><tr class="ttrpg-odds-total-row"><td><strong>Total Success Rate</strong></td><td class="kpi-sub">All winning outcomes combined (P(Win): {p_win:.2}% · P(Succ ≥ 1): {p_succ:.2}%)</td><td><strong>{p_win:.2}%</strong> <span class="kpi-sub">(E[ΔM]: {c_mean:+.2})</span></td><td><strong>{p_succ:.2}%</strong> <span class="kpi-sub">(E[Margin]: {u_mean:+.2})</span></td></tr></tfoot>"#,
            r#"</table></div>"#,
        ),
        tn_p = tn_p,
        tn_p_min = (tn_p + 1).clamp(2, 40),
        c = c,
        d_tool = d_tool,
        tn_gm = tn_gm,
        tn_gm_min = (tn_gm + 1).clamp(2, 40),
        d_ext = d_ext,
        tn_eff = tn_eff,
        n = n,
        d_over = uncontested.delta_over,
        d_under = uncontested.effective_delta_under(),
        carry_label = uncontested.carry_mode.label(),
        botch_label = uncontested.botch_rule.label(),
        carry_code = uncontested.carry_mode,
        botch_code = uncontested.botch_rule,
        c_mean = c_mean,
        u_mean = u_mean,
        p_win = p_win,
        p_succ = p_succ,
        c_extra = c_extra,
        c_clean = c_clean,
        c_partial = c_partial,
        c_graze = c_graze,
        c_lucky = c_lucky,
        c_breach = c_breach,
        c_fail = c_fail,
        c_botch = c_botch,
        u_comp = u_comp,
        u_fail = u_fail,
        u_botch = u_botch,
        seg_extra = seg("seg-extraordinary", "Extraordinary Win", c_extra),
        seg_clean = seg("seg-clean", "Clean Success", c_clean),
        seg_partial = seg("seg-partial", "Partial / Grazing Win", c_partial),
        seg_fail = seg("seg-fail", "Clean Failure", c_fail),
        seg_botch = seg("seg-botch", "Botch / Critical Failure", c_botch),
        cell_c_extra = bar_cell("seg-extraordinary", c_extra),
        cell_c_clean = bar_cell("seg-clean", c_clean),
        cell_u_clean = bar_cell("seg-clean", u_clean_succ),
        cell_c_partial = bar_cell("seg-partial", c_partial),
        cell_u_comp = bar_cell("seg-partial", u_comp),
        cell_c_fail = bar_cell("seg-fail", c_fail),
        cell_u_fail = bar_cell("seg-fail", u_fail),
        cell_c_botch = bar_cell("seg-botch", c_botch),
        cell_u_botch = bar_cell("seg-botch", u_botch),
    );

    (pill_text, readout_html)
}

/// Computes and formats the live exact Combinatorial & DP probability readout for the
/// Satya Two-List Cascading ("Watershed") `2d20` task drawer (`#ttrpg-odds-pill` and `#ttrpg-odds-readout`).
#[must_use]
pub fn format_cascade_odds_summary(
    skill_level_n: usize,
    control_c: i32,
    delta_ext: i32,
    delta_int: i32,
    carry_mode_raw: &str,
    botch_rule_raw: &str,
) -> (String, String) {
    let (_, readout) = format_contested_cascade_odds_summary(
        skill_level_n,
        control_c,
        0,
        delta_ext,
        delta_int,
        carry_mode_raw,
        botch_rule_raw,
    );
    let n = skill_level_n.clamp(1, 10);
    let c = control_c.clamp(1, 20);
    let d_ext = delta_ext.clamp(-10, 16);
    let d_int = delta_int.clamp(-2, 10);
    let mut params = satya_engine::CascadeTaskParams::new_k20(n, c, d_int, d_ext);
    if let Ok(m) = carry_mode_raw.parse::<satya_engine::CarryMode>() {
        params.carry_mode = m;
    }
    if let Ok(b) = botch_rule_raw.parse::<satya_engine::BotchRule>() {
        params.botch_rule = b;
    }
    let odds = params.exact_odds();
    let pill = format!(
        "TN_eff: {} · P(Succ≥1): {:.1}%",
        params.tn_eff(),
        odds.prob_any_success * 100.0
    );
    (pill, readout)
}

/// Reads the current Cascade inputs in `#ttrpg-drawer` and updates `#ttrpg-odds-pill`
/// and `#ttrpg-odds-readout` with live exact Contested & Uncontested probabilities from `satya-engine`.
#[wasm_bindgen]
pub fn satya_update_cascade_preview() {
    let skill_level = input_value("ttrpg-skill-level")
        .parse::<usize>()
        .unwrap_or(3)
        .clamp(1, 10);
    let control_c = input_value("ttrpg-control-c")
        .parse::<i32>()
        .unwrap_or(12)
        .clamp(1, 20);
    let internal_mod = input_value("ttrpg-internal-mod")
        .parse::<i32>()
        .unwrap_or(0)
        .clamp(-2, 10);
    let tool_mod = input_value("ttrpg-tool-mod")
        .parse::<i32>()
        .unwrap_or(0)
        .clamp(-4, 12);
    let external_mod = input_value("ttrpg-external-mod")
        .parse::<i32>()
        .unwrap_or(0)
        .clamp(-10, 16);
    let mode_raw = select_value("ttrpg-carry-mode");
    let botch_raw = select_value("ttrpg-botch-rule");
    let (pill_text, readout_text) = format_contested_cascade_odds_summary(
        skill_level,
        control_c,
        tool_mod,
        external_mod,
        internal_mod,
        &mode_raw,
        &botch_raw,
    );
    set_text("ttrpg-odds-pill", &pill_text);
    set_html("ttrpg-odds-readout", &readout_text);
    crate::wasm_client::ttrpg_drawer_triad::satya_update_drawer_triad_preview();
}

/// Alias for [`satya_update_cascade_preview`] updating both `#ttrpg-odds-pill` and `#ttrpg-odds-readout`.
#[wasm_bindgen]
pub fn satya_update_cascade_odds() {
    satya_update_cascade_preview();
}

/// Searches [`satya_engine::CANONICAL_SKILL_TREES`] by case-insensitive query tokens across skill name,
/// ID, tree title/code, categories, and governing characteristics, ordering trained skills and name
/// matches first.
#[must_use]
pub fn search_canonical_skills(
    query: &str,
    sheet: &crate::wasm_client::character_sheet::CharacterSheetState,
) -> Vec<&'static satya_engine::SkillNodeSpec> {
    use satya_engine::{CANONICAL_SKILL_TREES, SkillType};

    let tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();

    let mut matched: Vec<(u8, &'static satya_engine::SkillNodeSpec)> = CANONICAL_SKILL_TREES
        .iter()
        .filter_map(|spec| {
            let name_lc = spec.name.to_ascii_lowercase();
            let id_lc = spec.id.to_ascii_lowercase();
            let tree_lc = spec.tree.title().to_ascii_lowercase();
            let tree_code_lc = spec.tree.short_code().to_ascii_lowercase();
            let p_char_lc = spec.primary_char.as_str().to_ascii_lowercase();
            let s_char_lc = spec
                .secondary_char
                .map(|c| c.as_str().to_ascii_lowercase())
                .unwrap_or_default();

            if !tokens.is_empty() {
                let all_match = tokens.iter().all(|tok| {
                    name_lc.contains(tok)
                        || id_lc.contains(tok)
                        || tree_lc.contains(tok)
                        || tree_code_lc.contains(tok)
                        || p_char_lc.contains(tok)
                        || s_char_lc.contains(tok)
                        || spec
                            .categories
                            .iter()
                            .any(|cat| cat.label().to_ascii_lowercase().contains(tok))
                });
                if !all_match {
                    return None;
                }
            }

            let n = sheet.skills.get(spec.id).copied().unwrap_or(0);
            let is_trained = (spec.skill_type != SkillType::Tal && n >= 1)
                || (spec.skill_type == SkillType::Tal && n >= 2);
            let name_hit = !tokens.is_empty() && tokens.iter().all(|tok| name_lc.contains(tok));
            let prefix_hit = !tokens.is_empty() && name_lc.starts_with(&tokens[0]);

            let rank = match (prefix_hit, name_hit, is_trained) {
                (true, _, true) => 0,
                (true, _, false) => 1,
                (false, true, true) => 2,
                (false, true, false) => 3,
                (false, false, true) => 4,
                (false, false, false) => 5,
            };
            Some((rank, spec))
        })
        .collect();

    matched.sort_by_key(|&(rank, _)| rank);
    matched.into_iter().map(|(_, spec)| spec).collect()
}

/// Formats the inner HTML option rows for `#ttrpg-skill-dropdown` based on a search query and active
/// [`CharacterSheetState`](crate::wasm_client::character_sheet::CharacterSheetState).
#[must_use]
pub fn format_ttrpg_skill_dropdown_html(
    query: &str,
    sheet: &crate::wasm_client::character_sheet::CharacterSheetState,
) -> String {
    use satya_engine::SkillType;

    let matches = search_canonical_skills(query, sheet);
    if matches.is_empty() {
        return format!(
            r#"<div class="ttrpg-skill-empty">No canonical skills matched "{}"</div>"#,
            escape_html(query.trim())
        );
    }

    matches
        .into_iter()
        .map(|spec| {
            let n = sheet.skills.get(spec.id).copied().unwrap_or(0);
            let roll_n = n.max(1);
            let ctrl = sheet
                .characteristics
                .compute_controller(spec.primary_char, spec.secondary_char);
            let ctrl_formula = match spec.secondary_char {
                Some(sec) => format!("{}/{}", spec.primary_char.as_str(), sec.as_str()),
                None => spec.primary_char.as_str().to_string(),
            };
            let is_trained = (spec.skill_type != SkillType::Tal && n >= 1)
                || (spec.skill_type == SkillType::Tal && n >= 2);
            let trained_cls = if is_trained { " is-trained" } else { "" };
            let level_badge = if n >= 1 {
                format!(r#"<span class="pill pill-active">N={n} · C={ctrl}</span>"#)
            } else {
                format!(r#"<span class="pill pill-muted">N=0 (Roll {roll_n}d20) · C={ctrl}</span>"#)
            };
            let id_attr = escape_html(spec.id);
            let id_js = escape_js_single_quoted_attr(spec.id);

            format!(
                r#"<button type="button" class="ttrpg-skill-option{trained_cls}" role="option" data-skill-id="{id_attr}" onclick="window.satyaSelectTtrpgSkill('{id_js}')"><div class="ttrpg-skill-opt-main"><strong>{name}</strong><span class="stat-note">{tree_short} · T{tier} · {stype} ({ctrl_formula})</span></div>{level_badge}</button>"#,
                name = escape_html(spec.name),
                tree_short = spec.tree.short_code(),
                tier = spec.tier,
                stype = spec.skill_type.as_str(),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Returns the initial SSR HTML for `#ttrpg-skill-dropdown` using the default character sheet.
#[must_use]
pub fn default_ttrpg_skill_dropdown_html() -> String {
    format_ttrpg_skill_dropdown_html(
        "",
        &crate::wasm_client::character_sheet::CharacterSheetState::default(),
    )
}

fn set_ttrpg_skill_dropdown_open(open: bool, query: &str) {
    if open {
        let html = with_state(|st| format_ttrpg_skill_dropdown_html(query, &st.character_sheet));
        set_html("ttrpg-skill-dropdown", &html);
        set_display("ttrpg-skill-dropdown", "flex");
    } else {
        set_display("ttrpg-skill-dropdown", "none");
    }
    if let Some(inp) = get_element("ttrpg-skill") {
        let _ = inp.set_attribute("aria-expanded", if open { "true" } else { "false" });
    }
}

/// Filters `#ttrpg-skill-dropdown` live as the user types in `#ttrpg-skill`, and auto-syncs
/// `N` and `C` if the typed text exactly matches a canonical skill name or ID.
#[wasm_bindgen]
pub fn satya_filter_ttrpg_skill_dropdown() {
    let query = input_value("ttrpg-skill");
    set_ttrpg_skill_dropdown_open(true, &query);

    if let Some(spec) = satya_engine::lookup_skill_node(&query) {
        let (n, ctrl) = with_state(|st| {
            let n = st
                .character_sheet
                .skills
                .get(spec.id)
                .copied()
                .unwrap_or(0)
                .max(1);
            let ctrl = st
                .character_sheet
                .characteristics
                .compute_controller(spec.primary_char, spec.secondary_char);
            (n, ctrl)
        });
        set_input_value("ttrpg-skill-level", &n.clamp(1, 10).to_string());
        set_input_value("ttrpg-control-c", &ctrl.clamp(1, 20).to_string());
        satya_update_cascade_preview();
    }
}

/// Opens `#ttrpg-skill-dropdown` when `#ttrpg-skill` receives focus.
#[wasm_bindgen]
pub fn satya_open_ttrpg_skill_dropdown() {
    let query = input_value("ttrpg-skill");
    let effective_query = if satya_engine::lookup_skill_node(&query).is_some() {
        ""
    } else {
        query.as_str()
    };
    set_ttrpg_skill_dropdown_open(true, effective_query);
}

/// Toggles `#ttrpg-skill-dropdown` open or closed when `#btn-toggle-ttrpg-skill-dropdown` is clicked.
#[wasm_bindgen]
pub fn satya_toggle_ttrpg_skill_dropdown() {
    if let Some(opened) = toggle_element_display("ttrpg-skill-dropdown", "flex") {
        if opened {
            let html =
                with_state(|st| format_ttrpg_skill_dropdown_html("", &st.character_sheet));
            set_html("ttrpg-skill-dropdown", &html);
        }
        if let Some(inp) = get_element("ttrpg-skill") {
            let _ = inp.set_attribute("aria-expanded", if opened { "true" } else { "false" });
        }
    }
}

/// Selects a canonical skill from `#ttrpg-skill-dropdown` by its `skill_id`, populates `#ttrpg-skill`,
/// `#ttrpg-skill-level` (`N`), and `#ttrpg-control-c` (`C`) from the active Character Sheet, and closes the dropdown.
#[wasm_bindgen]
pub fn satya_select_ttrpg_skill(skill_id: String) {
    let Some(spec) = satya_engine::lookup_skill_node(&skill_id) else {
        set_ttrpg_skill_dropdown_open(false, "");
        return;
    };
    let (n, ctrl) = with_state(|st| {
        let n = st
            .character_sheet
            .skills
            .get(spec.id)
            .copied()
            .unwrap_or(0)
            .max(1);
        let ctrl = st
            .character_sheet
            .characteristics
            .compute_controller(spec.primary_char, spec.secondary_char);
        (n as i32, ctrl)
    });
    crate::wasm_client::notebook::satya_load_skill_preset(spec.name.to_string(), n, ctrl);
    set_ttrpg_skill_dropdown_open(false, "");
}

/// Handles keyboard shortcuts (`Escape` to close, `ArrowDown` to open, `Enter` to select top match)
/// inside `#ttrpg-skill`.
#[wasm_bindgen]
pub fn satya_ttrpg_skill_keydown(ev: KeyboardEvent) {
    let key = ev.key();
    if key == "Escape" {
        set_ttrpg_skill_dropdown_open(false, "");
    } else if key == "ArrowDown" {
        set_ttrpg_skill_dropdown_open(true, &input_value("ttrpg-skill"));
    } else if key == "Enter" {
        ev.prevent_default();
        let query = input_value("ttrpg-skill");
        let top_id = with_state(|st| {
            search_canonical_skills(&query, &st.character_sheet)
                .first()
                .map(|s| s.id.to_string())
        });
        if let Some(id) = top_id {
            satya_select_ttrpg_skill(id);
        } else {
            set_ttrpg_skill_dropdown_open(false, "");
        }
    }
}

/// Evaluates and emits a Satya Two-List Cascading (`2d20`) TTRPG task resolution event (`ROLL_TTRPG_TASK`) over the WebSocket.
#[wasm_bindgen]
pub fn satya_roll_task() {
    let skill_raw = input_value("ttrpg-skill").trim().to_string();
    let skill_name = if skill_raw.is_empty() {
        "Athletics".to_string()
    } else {
        skill_raw
    };
    let skill_level = input_value("ttrpg-skill-level")
        .parse::<usize>()
        .unwrap_or(3)
        .clamp(1, 10);
    let control_c = input_value("ttrpg-control-c")
        .parse::<i32>()
        .unwrap_or(12)
        .clamp(1, 20);
    let internal_mod = input_value("ttrpg-internal-mod")
        .parse::<i32>()
        .unwrap_or(0)
        .clamp(-2, 10);
    let known_tool_mod = input_value("ttrpg-tool-mod")
        .parse::<i32>()
        .unwrap_or(0)
        .clamp(-4, 12);
    let external_mod = input_value("ttrpg-external-mod")
        .parse::<i32>()
        .unwrap_or(0)
        .clamp(-10, 16);
    let carry_mode_raw = select_value("ttrpg-carry-mode");
    let carry_mode = if carry_mode_raw.trim().is_empty() {
        Some("markov_step".to_string())
    } else {
        Some(carry_mode_raw)
    };
    let botch_rule_raw = select_value("ttrpg-botch-rule");
    let botch_rule = if botch_rule_raw.trim().is_empty() {
        Some("closest_miss".to_string())
    } else {
        Some(botch_rule_raw)
    };
    let combat_stance = parse_combat_stance(&select_value("ttrpg-stance"));
    let fatigue_level = input_value("ttrpg-fatigue").parse::<i32>().unwrap_or(0);
    let wound_shock_index = input_value("ttrpg-wound-shock")
        .parse::<f32>()
        .unwrap_or(0.0);
    let tech_raw = select_value("ttrpg-action-technique");
    let combat_technique = if tech_raw.trim().is_empty()
        || tech_raw.eq_ignore_ascii_case("non_combat")
        || tech_raw.eq_ignore_ascii_case("none")
    {
        None
    } else {
        Some(tech_raw)
    };
    let (tempo_pool, control_pool, force_pool) = if combat_technique.is_some() {
        (
            input_value("ttrpg-pool-tempo").parse::<i32>().ok(),
            input_value("ttrpg-pool-control").parse::<i32>().ok(),
            input_value("ttrpg-pool-force").parse::<i32>().ok(),
        )
    } else {
        (None, None, None)
    };

    satya_update_cascade_preview();

    send_ws_packet(&ClientWsPacket::RollTtrpgTask {
        skill_name,
        skill_level,
        control_c,
        internal_mod,
        known_tool_mod,
        external_mod,
        delta_over: None,
        gm_resistance_mode: None,
        tie_rule: None,
        carry_mode,
        botch_rule,
        combat_stance,
        fatigue_level,
        wound_shock_index,
        combat_technique,
        tempo_pool,
        control_pool,
        force_pool,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::character_sheet::CharacterSheetState;

    #[test]
    fn test_search_canonical_skills_ranking_tokens_and_dropdown_html() {
        let mut sheet = CharacterSheetState::default();
        sheet.skills.clear();
        let target_spec = satya_engine::CANONICAL_SKILL_TREES
            .iter()
            .find(|s| s.skill_type != satya_engine::SkillType::Tal)
            .expect("canonical non-TAL skill exists");
        sheet.skills.insert(target_spec.id.to_string(), 4);

        // Empty query returns all canonical skills with trained skills ranked ahead of untrained
        let all = search_canonical_skills("", &sheet);
        assert_eq!(all.len(), satya_engine::CANONICAL_SKILL_TREES.len());
        assert_eq!(all[0].id, target_spec.id);

        // Prefix/name query prioritizes exact prefix + trained skill
        let prefix: String = target_spec.name.chars().take(4).collect();
        let filtered = search_canonical_skills(&prefix, &sheet);
        assert!(!filtered.is_empty());
        assert_eq!(filtered[0].id, target_spec.id);

        // Multi-token search across name + primary characteristic
        let multi_q = format!("{} {}", target_spec.name, target_spec.primary_char.as_str());
        let multi = search_canonical_skills(&multi_q, &sheet);
        assert!(multi.iter().any(|s| s.id == target_spec.id));

        // Non-matching query returns empty slice and empty-state HTML
        assert!(search_canonical_skills("zzzz_nonexistent_skill_999", &sheet).is_empty());
        let empty_html = format_ttrpg_skill_dropdown_html("zzzz <none>", &sheet);
        assert!(empty_html.contains("No canonical skills matched"));
        assert!(empty_html.contains("zzzz &lt;none&gt;"));

        // Matched dropdown HTML marks trained skill and wires satyaSelectTtrpgSkill
        let dd_html = format_ttrpg_skill_dropdown_html(target_spec.id, &sheet);
        assert!(dd_html.contains("is-trained"));
        assert!(dd_html.contains("N=4"));
        assert!(dd_html.contains(&format!(
            "window.satyaSelectTtrpgSkill('{}')",
            target_spec.id
        )));
    }

    #[test]
    fn test_format_contested_cascade_odds_summary_clamping_and_markup() {
        let (pill, html) = format_contested_cascade_odds_summary(
            3,
            12,
            -2,
            4,
            1,
            "markov_step",
            "closest_miss",
        );
        // TN_P = 31 - 12 + (-2) = 17; TN_GM = 21 - 4 = 17
        assert!(pill.contains("TN_P: 17 vs TN_GM: 17"));
        assert!(pill.contains("P(Win):"));
        assert!(pill.contains("P(Succ≥1):"));
        assert!(html.contains("Outcome Distribution Graph"));
        assert!(html.contains("Extraordinary Win"));
        assert!(html.contains("Total Success Rate"));

        // Out-of-bounds inputs clamp safely without panicking and fallback to defaults on unknown modes
        let (clamped_pill, clamped_html) =
            format_contested_cascade_odds_summary(99, 99, -99, 99, 99, "invalid", "invalid");
        assert!(clamped_pill.contains("TN_P:"));
        assert!(clamped_html.contains("10 × 2d20"));
    }
}
