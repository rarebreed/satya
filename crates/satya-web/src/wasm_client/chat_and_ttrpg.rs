//! Targeted Session Chat composer, recipient selection, context toggles, message filtering,
//! UTC/local timestamp formatting, and Markdown/mechanics chat bubble rendering.
//!
//! Two-List Cascading (`2d20`) odds previews, searchable canonical skill combobox handlers, and
//! `satya_roll_task` live in [`crate::wasm_client::ttrpg_drawer_cascade`] and are re-exported here.

pub use crate::wasm_client::ttrpg_drawer_cascade::*;

use crate::wasm_client::dom::{
    document, escape_html, get_element, input_value, roll_browser_die_i64, set_class, set_display,
    set_input_checked, set_input_value,
};
use crate::wasm_client::notebook::render_wgpu_dice_roll;
use crate::wasm_client::options::parse_channel_context;
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

/// Sends a public or private chat message (or executes a `/msg`, `/w`, `/whisper`, `/roll`, `/r`, or `/help` command)
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
#[must_use]
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
#[must_use]
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
#[must_use]
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
#[must_use]
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
#[must_use]
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

pub(crate) fn render_mechanics_event_bubble_only(ev: &TtrpgMechanicsEvent) {
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
}

pub(crate) fn render_mechanics_event(ev: &TtrpgMechanicsEvent) {
    render_mechanics_event_bubble_only(ev);

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
