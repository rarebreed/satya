//! Targeted Session Chat composer, recipient selection, context toggles, message filtering,
//! timestamp formatting, and Satya Two-List Cascading (`2d20`) task resolution.

use crate::wasm_client::dom::{
    document, escape_html, get_element, input_value, roll_browser_die_i64, select_value, set_class,
    set_display, set_html, set_input_checked, set_input_value, set_text,
};
use crate::wasm_client::notebook::render_wgpu_dice_roll;
use crate::wasm_client::options::{parse_channel_context, parse_combat_stance};
use crate::wasm_client::state::{with_state, with_state_mut};
use crate::wasm_client::webrtc::send_ws_packet;
use satya_schema::{
    ChannelContext, ChatMessageEvent, ClientWsPacket, MessageVisibility, ParticipantKind,
    TtrpgMechanicsEvent, WgpuDiceSpec,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement, HtmlInputElement, KeyboardEvent};

/// Legacy no-op handler retained for compatibility; recipient selection is now managed via
/// [`satya_select_broadcast`] and [`satya_toggle_recipient`].
#[wasm_bindgen]
pub fn satya_visibility_changed() {}

/// Selects `BROADCAST` in the vertical chat recipient list and deselects all individual users.
#[wasm_bindgen]
pub fn satya_select_broadcast() {
    with_state_mut(|st| {
        st.selected_recipients.clear();
    });
    set_input_checked("recipient-cb-broadcast", true);
    set_class(
        "recipient-row-broadcast",
        "recip-row broadcast-row selected",
    );

    if let Some(recip_box) = get_element("recipient-checkboxes") {
        let children = recip_box.children();
        for i in 0..children.length() {
            if let Some(label) = children.item(i) {
                if label.id() == "recipient-row-broadcast" {
                    continue;
                }
                label.set_class_name("recip-row");
                if let Some(first) = label.first_element_child()
                    && let Ok(cb) = first.dyn_into::<HtmlInputElement>()
                {
                    cb.set_checked(false);
                }
            }
        }
    }
}

/// Toggles selection of a specific user in the vertical chat recipient list.
///
/// Selecting one or more users automatically deselects `BROADCAST`; deselecting all users
/// automatically re-selects `BROADCAST`.
#[wasm_bindgen]
pub fn satya_toggle_recipient(user_id: String) {
    let mut checked_ids = Vec::new();
    if let Some(recip_box) = get_element("recipient-checkboxes") {
        let children = recip_box.children();
        for i in 0..children.length() {
            if let Some(label) = children.item(i) {
                if label.id() == "recipient-row-broadcast" {
                    continue;
                }
                if let Some(first) = label.first_element_child()
                    && let Ok(cb) = first.dyn_into::<HtmlInputElement>()
                {
                    if cb.checked() {
                        checked_ids.push(cb.value());
                        label.set_class_name("recip-row selected");
                    } else {
                        label.set_class_name("recip-row");
                    }
                }
            }
        }
    } else {
        with_state_mut(|st| {
            if let Some(pos) = st.selected_recipients.iter().position(|id| id == &user_id) {
                st.selected_recipients.remove(pos);
            } else if !user_id.is_empty() {
                st.selected_recipients.push(user_id);
            }
            checked_ids = st.selected_recipients.clone();
        });
    }

    let broadcast_active = checked_ids.is_empty();
    with_state_mut(|st| {
        st.selected_recipients = checked_ids;
    });
    set_input_checked("recipient-cb-broadcast", broadcast_active);
    set_class(
        "recipient-row-broadcast",
        if broadcast_active {
            "recip-row broadcast-row selected"
        } else {
            "recip-row broadcast-row"
        },
    );
}

/// Selects one of the mutually exclusive [`ChannelContext`] toggle buttons inside the chat input field
/// (`"IN_CHARACTER"`, `"OUT_OF_CHARACTER"`, or `"GM_NARRATION"`).
#[wasm_bindgen]
pub fn satya_select_chat_context(ctx: String) {
    let parsed = parse_channel_context(&ctx);
    with_state_mut(|st| {
        st.active_channel_context = parsed;
    });

    let buttons = [
        ("btn-ctx-ic", ChannelContext::InCharacter),
        ("btn-ctx-ooc", ChannelContext::OutOfCharacter),
        ("btn-ctx-gm", ChannelContext::GmNarration),
    ];
    for (btn_id, target_ctx) in buttons {
        let is_active = parsed == target_ctx;
        set_class(
            btn_id,
            if is_active {
                "btn-ctx-toggle active"
            } else {
                "btn-ctx-toggle"
            },
        );
        if let Some(btn) = get_element(btn_id) {
            let _ = btn.set_attribute("aria-pressed", if is_active { "true" } else { "false" });
        }
    }
}

pub(crate) fn sync_gm_context_button_visibility(is_gm: bool) {
    set_display("btn-ctx-gm", if is_gm { "inline-flex" } else { "none" });
    if !is_gm {
        let was_gm = with_state(|st| st.active_channel_context == ChannelContext::GmNarration);
        if was_gm {
            satya_select_chat_context("IN_CHARACTER".to_string());
        }
    }
}

/// Tracks chat composition start time on keydown, navigates input history on `ArrowUp` / `ArrowDown`
/// (when the draft is single-line), and sends the Markdown message when `Enter` (without `Shift`) is pressed.
/// Pressing `Shift+Enter` inserts a newline in the Markdown composer.
#[wasm_bindgen]
pub fn satya_chat_keydown(ev: KeyboardEvent) {
    let key = ev.key();
    let current = input_value("chat-input");
    if key == "ArrowUp" && !ev.shift_key() && !current.contains('\n') {
        ev.prevent_default();
        if let Some(prev) = with_state_mut(|st| st.chat_history.navigate_up(&current)) {
            set_input_value("chat-input", &prev);
        }
        return;
    }
    if key == "ArrowDown" && !ev.shift_key() && !current.contains('\n') {
        ev.prevent_default();
        if let Some(next) = with_state_mut(|st| st.chat_history.navigate_down()) {
            set_input_value("chat-input", &next);
        }
        return;
    }
    with_state_mut(|st| {
        if st.typing_started_at_ms.is_none() {
            st.typing_started_at_ms = Some(js_sys::Date::now());
        }
    });
    if key == "Enter" && !ev.shift_key() {
        ev.prevent_default();
        satya_send_chat();
    }
}

/// Sends a public or private chat message (or executes a `/msg`, `/roll`, or `--help` command)
/// over the signaling WebSocket based on the selected recipient(s) and active context icon toggle.
#[wasm_bindgen]
pub fn satya_send_chat() {
    let body = input_value("chat-input").trim().to_string();
    if body.is_empty() {
        return;
    }

    with_state_mut(|st| st.chat_history.record_sent(&body));

    let (mut recipient_user_ids, channel_context, composition_time_ms) = with_state_mut(|st| {
        let elapsed = st
            .typing_started_at_ms
            .map(|t0| (js_sys::Date::now() - t0).max(1.0) as i64)
            .unwrap_or(450);
        st.typing_started_at_ms = None;
        (
            st.selected_recipients.clone(),
            st.active_channel_context,
            elapsed,
        )
    });

    if recipient_user_ids.is_empty()
        && let Some(recip_box) = get_element("recipient-checkboxes")
    {
        let children = recip_box.children();
        for i in 0..children.length() {
            if let Some(label) = children.item(i) {
                if label.id() == "recipient-row-broadcast" {
                    continue;
                }
                if let Some(first) = label.first_element_child()
                    && let Ok(cb) = first.dyn_into::<HtmlInputElement>()
                    && cb.checked()
                {
                    recipient_user_ids.push(cb.value());
                }
            }
        }
    }

    let mut visibility = if recipient_user_ids.is_empty() {
        MessageVisibility::Public
    } else {
        MessageVisibility::Private
    };

    let final_body = match crate::chat_cmd::parse_chat_command(&body) {
        Err(err) => {
            append_system_notice(&format!("Command error: {err}"));
            return;
        }
        Ok(Some(crate::chat_cmd::ChatCommand::Help { help_text, .. })) => {
            append_system_notice(&help_text);
            set_input_value("chat-input", "");
            return;
        }
        Ok(Some(crate::chat_cmd::ChatCommand::MsgBroadcast { body: msg_body })) => {
            visibility = MessageVisibility::Public;
            recipient_user_ids.clear();
            msg_body
        }
        Ok(Some(crate::chat_cmd::ChatCommand::MsgPrivate {
            targets,
            body: msg_body,
        })) => {
            let (roster, self_uid) = with_state(|st| {
                (
                    st.roster.clone(),
                    st.self_participant.as_ref().map(|p| p.user_id.clone()),
                )
            });
            let (matched_ids, unresolved) =
                crate::chat_cmd::resolve_msg_recipients(&targets, &roster, self_uid.as_deref());
            if !unresolved.is_empty() {
                let names: Vec<String> = unresolved.iter().map(|u| format!("@{u}")).collect();
                append_system_notice(&format!(
                    "Unresolved recipient(s): {}. Check active session roster.",
                    names.join(", ")
                ));
            }
            if matched_ids.is_empty() {
                return;
            }
            visibility = MessageVisibility::Private;
            recipient_user_ids = matched_ids;
            msg_body
        }
        Ok(Some(crate::chat_cmd::ChatCommand::Roll { expr_raw, pipeline })) => {
            let outcome = match pipeline.evaluate_with_roller(&expr_raw, roll_browser_die_i64) {
                Ok(out) => out,
                Err(err) => {
                    append_system_notice(&format!("Roll error: {err}"));
                    return;
                }
            };
            let has_ws = with_state(|st| st.ws.is_some());
            if !has_ws {
                append_system_notice(&outcome.summary);
                set_input_value("chat-input", "");
                return;
            }
            outcome.summary
        }
        Ok(None) => body,
    };

    send_ws_packet(&ClientWsPacket::SendChat {
        visibility,
        recipient_user_ids,
        channel_context,
        body: final_body,
        composition_time_ms,
    });
    set_input_value("chat-input", "");
}

/// Formats a concise summary pill for a [`satya_engine::CascadeTaskParams`] configuration,
/// displaying $\text{TN}_{\text{eff}}$, $\delta_{\text{under}}$, $P(\text{Succ} \ge 1)$, and $P(\text{Botch})$.
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

            format!(
                r#"<button type="button" class="ttrpg-skill-option{trained_cls}" role="option" data-skill-id="{id}" onclick="window.satyaSelectTtrpgSkill('{id}')"><div class="ttrpg-skill-opt-main"><strong>{name}</strong><span class="stat-note">{tree_short} · T{tier} · {stype} ({ctrl_formula})</span></div>{level_badge}</button>"#,
                id = spec.id,
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
        if let Some(dd) = get_element("ttrpg-skill-dropdown") {
            dd.set_inner_html(&html);
        }
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
    let is_open = get_element("ttrpg-skill-dropdown")
        .and_then(|el| el.dyn_into::<HtmlElement>().ok())
        .map(|el| el.style().get_property_value("display").unwrap_or_default() != "none")
        .unwrap_or(false);
    if is_open {
        set_ttrpg_skill_dropdown_open(false, "");
    } else {
        set_ttrpg_skill_dropdown_open(true, "");
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
    });
}

fn is_msg_kind_enabled(kind: &str) -> bool {
    let cb_id = match kind {
        "PUBLIC" => "filter-msg-public",
        "PRIVATE" => "filter-msg-private",
        "AI_GM" => "filter-msg-ai-gm",
        "MECHANICS" => "filter-msg-mechanics",
        "SYSTEM" => "filter-msg-system",
        _ => return true,
    };
    get_element(cb_id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|cb| cb.checked())
        .unwrap_or(true)
}

fn apply_msg_filter_to_element(el: &Element, kind: &str) {
    if let Ok(html_el) = el.clone().dyn_into::<HtmlElement>() {
        let display = if is_msg_kind_enabled(kind) {
            ""
        } else {
            "none"
        };
        let _ = html_el.style().set_property("display", display);
    }
}

/// Applies the active message-type filter checkboxes (`#filter-msg-*`) to all messages in `#chat-log`.
#[wasm_bindgen]
pub fn satya_filter_chat() {
    let Some(log) = get_element("chat-log") else {
        return;
    };
    let children = log.children();
    for i in 0..children.length() {
        if let Some(child) = children.item(i) {
            let kind = child.get_attribute("data-msg-kind").unwrap_or_default();
            apply_msg_filter_to_element(&child, &kind);
        }
    }
}

/// Converts a Unix epoch timestamp in microseconds (`timestamp-micros`) into a deterministic UTC
/// `(short_hh_mm_ss, full_iso_8601)` pair (`("HH:MM:SS", "YYYY-MM-DDTHH:MM:SS.mmmZ")`).
pub fn format_timestamp_us_utc(timestamp_us: i64) -> (String, String) {
    let clamped_us = timestamp_us.max(0);
    let total_secs = clamped_us / 1_000_000;
    let millis = (clamped_us % 1_000_000) / 1_000;
    let secs_of_day = total_secs.rem_euclid(86_400);
    let hours = secs_of_day / 3_600;
    let mins = (secs_of_day % 3_600) / 60;
    let secs = secs_of_day % 60;

    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm)
    let z = total_secs.div_euclid(86_400) + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    let short = format!("{hours:02}:{mins:02}:{secs:02}");
    let full = format!("{year:04}-{month:02}-{day:02}T{short}.{millis:03}Z");
    (short, full)
}

/// Formats a Unix epoch microsecond timestamp (`client_timestamp_us` or `ingest_timestamp_us`)
/// into a displayable `(short_hh_mm_ss, full_timestamp_title)` pair.
///
/// On `wasm32`, formats `HH:MM:SS` in the player's local browser timezone with the full local date
/// and ISO-8601 UTC string in the hover tooltip. On non-`wasm32` targets, formats UTC.
pub fn format_timestamp_us(timestamp_us: i64) -> (String, String) {
    #[cfg(target_arch = "wasm32")]
    {
        let ms = if timestamp_us > 0 {
            (timestamp_us as f64) / 1_000.0
        } else {
            js_sys::Date::now()
        };
        let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
        let hours = date.get_hours();
        let mins = date.get_minutes();
        let secs = date.get_seconds();
        let year = date.get_full_year();
        let month = date.get_month() + 1;
        let day = date.get_date();
        let short = format!("{hours:02}:{mins:02}:{secs:02}");
        let iso_utc: String = date.to_iso_string().into();
        let full = format!("{year:04}-{month:02}-{day:02} {short} ({iso_utc})");
        (short, full)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        format_timestamp_us_utc(timestamp_us)
    }
}

/// Renders a Markdown chat message body into sanitized, XSS-safe HTML via
/// [`satya_engine::render_rules_markdown_to_html`], supporting bold, italic, inline/fenced code,
/// bullet/numbered lists, blockquotes/callouts (`> [!NOTE]`), tables, links, LaTeX math (`$...$`),
/// and Mermaid flowcharts while escaping raw HTML tags.
pub fn format_chat_markdown_html(markdown: &str) -> String {
    let trimmed = markdown.trim();
    if trimmed.is_empty() {
        return r#"<div class="chat-markdown-body"></div>"#.to_string();
    }
    format!(
        r#"<div class="chat-markdown-body">{}</div>"#,
        satya_engine::render_rules_markdown_to_html(trimmed)
    )
}

/// Formats the inner HTML for a [`ChatMessageEvent`] card in `#chat-log`, including sender,
/// role, channel context, `<time class="chat-timestamp">`, visibility/recipients, schema version,
/// and rendered Markdown body.
pub fn format_chat_event_html(ev: &ChatMessageEvent) -> String {
    let recip_names: Vec<String> = ev
        .recipients
        .iter()
        .map(|r| r.display_name.clone())
        .collect();
    let vis_label = if ev.visibility == MessageVisibility::Public {
        "PUBLIC".to_string()
    } else {
        format!("{:?} -> [{}]", ev.visibility, recip_names.join(", "))
    };
    let ts_us = if ev.metadata.ingest_timestamp_us > 0 {
        ev.metadata.ingest_timestamp_us
    } else {
        ev.metadata.client_timestamp_us
    };
    let (ts_short, ts_title) = format_timestamp_us(ts_us);

    format!(
        r#"<div class="chat-meta">
            <span><strong>{}</strong> ({:?}) · {:?}</span>
            <span><time class="chat-timestamp" datetime="{}" title="{}">{}</time> · {} · schema v{}</span>
        </div>
        {}"#,
        escape_html(&ev.sender.display_name),
        ev.sender.role,
        ev.channel_context,
        escape_html(&ts_title),
        escape_html(&ts_title),
        escape_html(&ts_short),
        escape_html(&vis_label),
        ev.metadata.schema_version,
        format_chat_markdown_html(&ev.body)
    )
}

pub(crate) fn render_chat_event(ev: &ChatMessageEvent) {
    let (Some(doc), Some(log)) = (document(), get_element("chat-log")) else {
        return;
    };
    let Ok(div) = doc.create_element("div") else {
        return;
    };

    let mut cls = String::from("chat-msg");
    if ev.visibility != MessageVisibility::Public {
        cls.push_str(" private");
    }
    if ev.sender.participant_kind == ParticipantKind::AiAgent {
        cls.push_str(" ai-gm");
    }
    div.set_class_name(&cls);

    let msg_kind = if ev.channel_context == ChannelContext::SystemNotice {
        "SYSTEM"
    } else if ev.sender.participant_kind == ParticipantKind::AiAgent {
        "AI_GM"
    } else if ev.visibility != MessageVisibility::Public {
        "PRIVATE"
    } else {
        "PUBLIC"
    };
    let _ = div.set_attribute("data-msg-kind", msg_kind);
    apply_msg_filter_to_element(&div, msg_kind);

    div.set_inner_html(&format_chat_event_html(ev));
    let _ = log.append_child(&div);
    log.set_scroll_top(log.scroll_height());
}

/// Formats the inner HTML for a [`TtrpgMechanicsEvent`] bubble in `#chat-log`,
/// displaying `2d20` pairs, `Control C`, `TN_eff`, `successes_counted`, cascade trace notes, and timestamp.
pub fn format_mechanics_event_html(ev: &TtrpgMechanicsEvent) -> String {
    let rolls_display =
        if ev.dice_rolled_d20.len() >= 2 && ev.dice_rolled_d20.len().is_multiple_of(2) {
            let pairs: Vec<String> = ev
                .dice_rolled_d20
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[a, b]| format!("({a}+{b}={})", a + b))
                .collect();
            format!("2d20 Pairs: [{}]", pairs.join(", "))
        } else {
            let rolls: Vec<String> = ev.dice_rolled_d20.iter().map(|r| r.to_string()).collect();
            format!("Rolled d20s: [{}]", rolls.join(", "))
        };
    let stance_info = match ev.combat_stance {
        Some(st) => format!(
            " · Stance: {st:?} (Action Cost: {} ticks)",
            ev.action_count_cost.unwrap_or(10)
        ),
        None => String::new(),
    };
    let notes_info = match ev.notes.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(n) => format!(r#" · <span class="mono">{}</span>"#, escape_html(n)),
        None => String::new(),
    };
    let ts_us = if ev.metadata.ingest_timestamp_us > 0 {
        ev.metadata.ingest_timestamp_us
    } else {
        ev.metadata.client_timestamp_us
    };
    let (ts_short, ts_title) = format_timestamp_us(ts_us);

    format!(
        r#"<div class="chat-meta">
            <span><strong>[SATYA MECHANICS] {}</strong> · {}</span>
            <span><time class="chat-timestamp" datetime="{}" title="{}">{}</time> · Outcome: {:?}</span>
        </div>
        <div>{} · Control C: {} · TN_eff: {} =&gt; <strong>{} successes</strong>{}{}</div>"#,
        escape_html(&ev.actor.display_name),
        escape_html(ev.skill_name.as_deref().unwrap_or("Action")),
        escape_html(&ts_title),
        escape_html(&ts_title),
        escape_html(&ts_short),
        ev.outcome_tier,
        rolls_display,
        ev.attribute_tn.unwrap_or(10),
        ev.difficulty_rating.unwrap_or(21),
        ev.successes_counted,
        stance_info,
        notes_info
    )
}

pub(crate) fn render_mechanics_event(ev: &TtrpgMechanicsEvent) {
    let (Some(doc), Some(log)) = (document(), get_element("chat-log")) else {
        return;
    };
    let Ok(div) = doc.create_element("div") else {
        return;
    };
    div.set_class_name("chat-msg mechanics");
    let _ = div.set_attribute("data-msg-kind", "MECHANICS");
    apply_msg_filter_to_element(&div, "MECHANICS");

    div.set_inner_html(&format_mechanics_event_html(ev));
    let _ = log.append_child(&div);
    log.set_scroll_top(log.scroll_height());

    if !ev.dice_rolled_d20.is_empty() {
        let spec = WgpuDiceSpec {
            dice_notation: format!("{}d20", ev.dice_rolled_d20.len()),
            physics_seed: ev.metadata.sequence_num,
            outcomes: ev.dice_rolled_d20.clone(),
            visibility: MessageVisibility::Public,
        };
        render_wgpu_dice_roll(&ev.actor, &spec);
    }
}

/// Appends a styled `SYSTEM` / `RUST/WASM` notice message to `#chat-log`.
pub fn append_system_notice(text: &str) {
    let (Some(doc), Some(log)) = (document(), get_element("chat-log")) else {
        return;
    };
    let Ok(div) = doc.create_element("div") else {
        return;
    };
    div.set_class_name("chat-msg");
    let _ = div.set_attribute("data-msg-kind", "SYSTEM");
    apply_msg_filter_to_element(&div, "SYSTEM");
    let (ts_short, ts_title) = format_timestamp_us(0);
    div.set_inner_html(&format!(
        r#"<div class="chat-meta"><span>SYSTEM</span><span><time class="chat-timestamp" datetime="{}" title="{}">{}</time> · RUST/WASM</span></div>{}"#,
        escape_html(&ts_title),
        escape_html(&ts_title),
        escape_html(&ts_short),
        format_chat_markdown_html(text)
    ));
    let _ = log.append_child(&div);
    log.set_scroll_top(log.scroll_height());
}
