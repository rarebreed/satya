//! Browser `localStorage` session persistence, automatic session resume on refresh,
//! visibility-filtered chat & mechanics history replay, portable `.satyasession` export/import,
//! and Markdown (`.md`) / JSON (`.json`) chat transcript export.

use crate::wasm_client::auth_session::{apply_auth_success, enter_session, perform_login};
use crate::wasm_client::character_sheet::render_character_sheet;
use crate::wasm_client::chat_and_ttrpg::{
    append_system_notice, render_chat_event, render_mechanics_event_bubble_only,
};
use crate::wasm_client::dom::{
    escape_html, fetch_json, fetch_text, get_element, input_value, local_storage_get,
    local_storage_remove, prompt_save_bytes, read_first_file_bytes, select_value, set_feedback,
    set_html, set_input_value, set_text,
};
use crate::wasm_client::gm_docs::{satya_refresh_gm_documents, satya_refresh_gm_memories};
use crate::wasm_client::notebook::{render_combat_tracker, render_map_updated};
use crate::wasm_client::npcs::satya_refresh_npcs_tab;
use crate::wasm_client::options::parse_session_role;
use crate::wasm_client::state::{SECONDS_PER_TICK, with_state, with_state_mut};
use satya_schema::{
    AuthSuccessResponse, ChatMessageEvent, ImportSessionSnapshotRequest, ResumeSessionRequest,
    SavedSessionSummary, SavedSessionsListResponse, SessionEnterResponse, SessionHistorySnapshot,
    SessionMode, SessionRole, SessionSnapshotBundle, SessionTranscriptResponse,
    TtrpgMechanicsEvent,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// `localStorage` key storing the active user's bearer authentication token across page refreshes.
pub const AUTH_TOKEN_STORAGE_KEY: &str = "satya.auth.token";

/// `localStorage` key storing the active session reference for automatic reconnect on refresh.
pub const ACTIVE_SESSION_STORAGE_KEY: &str = "satya.session.active";

/// Lightweight reference to the user's active session persisted in `localStorage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedActiveSessionRef {
    /// Active session room identifier.
    pub session_id: String,
    /// Last issued WebSocket join token for this session.
    #[serde(default)]
    pub join_token: Option<String>,
    /// Last active role in the session.
    #[serde(default)]
    pub role: Option<SessionRole>,
}

/// Interleaved chronological item in a replayed [`SessionHistorySnapshot`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HistoryReplayItem<'a> {
    /// Persisted chat message (`ChatMessageEvent`).
    Chat(&'a ChatMessageEvent),
    /// Persisted TTRPG task/mechanics roll (`TtrpgMechanicsEvent`).
    Mechanics(&'a TtrpgMechanicsEvent),
}

impl<'a> HistoryReplayItem<'a> {
    /// Monotonic session event `sequence_num` used for deterministic replay ordering.
    #[must_use]
    pub fn sequence_num(&self) -> i64 {
        match self {
            Self::Chat(ev) => ev.metadata.sequence_num,
            Self::Mechanics(ev) => ev.metadata.sequence_num,
        }
    }
}

/// Merges a snapshot's `chat_events` and `mechanics_events` into a single vector sorted by
/// ascending `metadata.sequence_num`.
#[must_use]
pub fn merge_history_events_by_sequence(
    history: &SessionHistorySnapshot,
) -> Vec<HistoryReplayItem<'_>> {
    let mut merged: Vec<HistoryReplayItem<'_>> =
        Vec::with_capacity(history.chat_events.len() + history.mechanics_events.len());
    for ev in &history.chat_events {
        merged.push(HistoryReplayItem::Chat(ev));
    }
    for ev in &history.mechanics_events {
        merged.push(HistoryReplayItem::Mechanics(ev));
    }
    merged.sort_by_key(HistoryReplayItem::sequence_num);
    merged
}

/// Formats `<option>` elements for `#saved-sessions-select` from a slice of [`SavedSessionSummary`] records.
#[must_use]
pub fn format_saved_sessions_options_html(
    sessions: &[SavedSessionSummary],
    selected_id: Option<&str>,
) -> String {
    if sessions.is_empty() {
        return r#"<option value="">No saved sessions yet — Create or Join a session below</option>"#
            .to_string();
    }
    let mut out = String::with_capacity(sessions.len() * 180);
    for (idx, s) in sessions.iter().enumerate() {
        let is_selected = match selected_id.filter(|v| !v.is_empty()) {
            Some(sel) => sel == s.session_id,
            None => idx == 0,
        };
        let sel_attr = if is_selected { " selected" } else { "" };
        let mode_label = match s.session_mode {
            SessionMode::Ttrpg => {
                if s.has_ai_gm {
                    "TTRPG + AI GM"
                } else {
                    "TTRPG"
                }
            }
            SessionMode::Meeting => "Meeting",
        };
        out.push_str(&format!(
            r#"<option value="{sid}"{sel_attr}>{sid} [{mode_label} · {role:?}] — {chats} msgs, {rolls} rolls ({active} live)</option>"#,
            sid = escape_html(&s.session_id),
            role = s.role,
            chats = s.chat_event_count,
            rolls = s.mechanics_event_count,
            active = s.active_human_count,
        ));
    }
    out
}

/// Restores any saved bearer token and active session from `localStorage` when the Wasm client boots.
pub(crate) fn restore_saved_auth_and_session() {
    let Some(saved_token) = local_storage_get(AUTH_TOKEN_STORAGE_KEY)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    else {
        return;
    };

    spawn_local(async move {
        let Ok(auth_data) =
            fetch_json::<(), AuthSuccessResponse>("GET", "/api/auth/me", Some(&saved_token), None)
                .await
        else {
            local_storage_remove(AUTH_TOKEN_STORAGE_KEY);
            local_storage_remove(ACTIVE_SESSION_STORAGE_KEY);
            return;
        };

        apply_auth_success(auth_data);

        let Some(active_raw) = local_storage_get(ACTIVE_SESSION_STORAGE_KEY) else {
            return;
        };
        let Ok(active_ref) = serde_json::from_str::<PersistedActiveSessionRef>(&active_raw) else {
            local_storage_remove(ACTIVE_SESSION_STORAGE_KEY);
            return;
        };
        if active_ref.session_id.trim().is_empty() {
            return;
        }

        let req = ResumeSessionRequest {
            session_id: active_ref.session_id.clone(),
            session_password: None,
            join_token: active_ref.join_token,
            role: active_ref.role,
            display_name: None,
        };
        match fetch_json::<_, SessionEnterResponse>(
            "POST",
            "/api/sessions/resume",
            Some(&saved_token),
            Some(&req),
        )
        .await
        {
            Ok(enter_data) => {
                let sid = enter_data.session_id.clone();
                set_input_value("session-id-input", &sid);
                set_feedback(
                    "session-feedback",
                    &format!("Automatically resumed session '{sid}'"),
                    false,
                );
                enter_session(enter_data);
            }
            Err(_) => {
                local_storage_remove(ACTIVE_SESSION_STORAGE_KEY);
            }
        }
    });
}

/// Replays a [`SessionHistorySnapshot`] into `#chat-log`, the tactical map canvas, the combat tracker,
/// and the character sheet when entering or resuming a session.
pub(crate) fn replay_session_history_snapshot(history: Option<SessionHistorySnapshot>) {
    if let Some(hist) = history {
        set_html("chat-log", "");

        for item in merge_history_events_by_sequence(&hist) {
            match item {
                HistoryReplayItem::Chat(ev) => render_chat_event(ev),
                HistoryReplayItem::Mechanics(ev) => render_mechanics_event_bubble_only(ev),
            }
        }

        if let Some(map) = hist.active_map.as_ref() {
            render_map_updated(map);
        }
        if let Some(ct) = hist.combat_tracker {
            let tick = ct.tick;
            let round = ct.round.max(1);
            with_state_mut(|st| {
                st.combat_tick = tick;
                st.combat_round = round;
            });
            let elapsed_sec = f64::from(tick) * SECONDS_PER_TICK;
            set_text(
                "combat-round-pill",
                &format!("Window {round} · Tick {tick} ({elapsed_sec:.2}s)"),
            );
            render_combat_tracker();
        }
        if let Some(sheet) = hist.bound_character.as_ref() {
            with_state_mut(|st| {
                st.character_sheet.apply_character_sheet_record(sheet);
            });
            render_character_sheet();
        }

        if !hist.chat_events.is_empty() || !hist.mechanics_events.is_empty() {
            append_system_notice(&format!(
                "Replayed saved session history: **{}** chat messages and **{}** task checks restored.",
                hist.chat_events.len(),
                hist.mechanics_events.len()
            ));
        }
    }

    satya_refresh_gm_documents();
    satya_refresh_gm_memories();
    satya_refresh_npcs_tab();
}

/// Fetches the authenticated user's saved sessions from `GET /api/sessions/saved`
/// and populates `#saved-sessions-select`.
#[wasm_bindgen]
pub fn satya_refresh_saved_sessions() {
    spawn_local(async move {
        let Some(token) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        if let Ok(resp) = fetch_json::<(), SavedSessionsListResponse>(
            "GET",
            "/api/sessions/saved",
            Some(&token),
            None,
        )
        .await
        {
            let current_sel = select_value("saved-sessions-select");
            let html = format_saved_sessions_options_html(&resp.sessions, Some(current_sel.trim()));
            set_html("saved-sessions-select", &html);
        }
    });
}

/// Syncs `#session-id-input` when the user chooses a saved session in `#saved-sessions-select`.
#[wasm_bindgen]
pub fn satya_select_saved_session() {
    let selected_sid = select_value("saved-sessions-select");
    if !selected_sid.trim().is_empty() {
        set_input_value("session-id-input", selected_sid.trim());
    }
}

/// Resumes the selected saved session (from `#saved-sessions-select` or `#session-id-input`)
/// via `POST /api/sessions/resume`.
#[wasm_bindgen]
pub fn satya_resume_saved_session() {
    spawn_local(async move {
        let has_token = with_state(|st| st.auth_token.is_some());
        if !has_token && !perform_login().await {
            return;
        }
        let Some(token) = with_state(|st| st.auth_token.clone()) else {
            return;
        };

        let dropdown_sid = select_value("saved-sessions-select").trim().to_string();
        let input_sid = input_value("session-id-input").trim().to_string();
        let session_id = if !dropdown_sid.is_empty() {
            dropdown_sid
        } else {
            input_sid
        };
        if session_id.is_empty() {
            set_feedback(
                "session-feedback",
                "Select a saved session or enter a Session ID to resume.",
                true,
            );
            return;
        }

        let pwd_raw = input_value("session-password-input");
        let session_password = if pwd_raw.trim().is_empty() {
            None
        } else {
            Some(pwd_raw)
        };
        let display_raw = input_value("auth-display-name").trim().to_string();
        let display_name = if display_raw.is_empty() {
            None
        } else {
            Some(display_raw)
        };

        let req = ResumeSessionRequest {
            session_id: session_id.clone(),
            session_password,
            join_token: None,
            role: None,
            display_name,
        };

        match fetch_json::<_, SessionEnterResponse>(
            "POST",
            "/api/sessions/resume",
            Some(&token),
            Some(&req),
        )
        .await
        {
            Ok(data) => {
                let chat_count = data
                    .history
                    .as_ref()
                    .map(|h| h.chat_events.len())
                    .unwrap_or(0);
                let mech_count = data
                    .history
                    .as_ref()
                    .map(|h| h.mechanics_events.len())
                    .unwrap_or(0);
                set_input_value("session-id-input", &data.session_id);
                set_feedback(
                    "session-feedback",
                    &format!(
                        "Resumed session '{}' ({chat_count} chat msgs, {mech_count} rolls restored)",
                        data.session_id
                    ),
                    false,
                );
                enter_session(data);
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Exports the active or selected session as a portable `.satyasession` JSON bundle via `GET /api/sessions/:id/export`.
#[wasm_bindgen]
pub fn satya_export_session_snapshot() {
    spawn_local(async move {
        let Some(token) = with_state(|st| st.auth_token.clone()) else {
            set_feedback(
                "session-feedback",
                "Log in first to export a session.",
                true,
            );
            return;
        };
        let active_sid = with_state(|st| st.session_id.clone()).unwrap_or_default();
        let dropdown_sid = select_value("saved-sessions-select").trim().to_string();
        let input_sid = input_value("session-id-input").trim().to_string();
        let session_id = if !active_sid.is_empty() {
            active_sid
        } else if !dropdown_sid.is_empty() {
            dropdown_sid
        } else {
            input_sid
        };
        if session_id.is_empty() {
            set_feedback(
                "session-feedback",
                "Select or enter a Session ID to export.",
                true,
            );
            return;
        }

        let url = format!("/api/sessions/{session_id}/export");
        match fetch_json::<(), SessionSnapshotBundle>("GET", &url, Some(&token), None).await {
            Ok(bundle) => {
                if let Ok(pretty_bytes) = serde_json::to_vec_pretty(&bundle) {
                    let filename = format!("{session_id}.satyasession");
                    let _ = prompt_save_bytes(&filename, &pretty_bytes).await;
                    set_feedback(
                        "session-feedback",
                        &format!("Exported session snapshot '{filename}'"),
                        false,
                    );
                }
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Imports a `.satyasession` JSON bundle from `#session-import-file-input` via `POST /api/sessions/import`.
#[wasm_bindgen]
pub fn satya_import_session_snapshot() {
    spawn_local(async move {
        let has_token = with_state(|st| st.auth_token.is_some());
        if !has_token && !perform_login().await {
            return;
        }
        let Some(token) = with_state(|st| st.auth_token.clone()) else {
            return;
        };

        let Some((file_name, _mime, bytes)) =
            read_first_file_bytes("session-import-file-input").await
        else {
            set_feedback(
                "session-feedback",
                "Select a `.satyasession` file first.",
                true,
            );
            return;
        };

        let Ok(bundle) = serde_json::from_slice::<SessionSnapshotBundle>(&bytes) else {
            set_feedback(
                "session-feedback",
                &format!("'{file_name}' is not a valid .satyasession JSON bundle."),
                true,
            );
            return;
        };

        let sid_input = input_value("session-id-input").trim().to_string();
        let session_id_override = if sid_input.is_empty() {
            None
        } else {
            Some(sid_input)
        };
        let pwd_raw = input_value("session-password-input");
        let session_password = if pwd_raw.trim().is_empty() {
            "satya-room-2026".to_string()
        } else {
            pwd_raw
        };
        let role = Some(parse_session_role(&select_value("session-role-select")));

        let req = ImportSessionSnapshotRequest {
            bundle,
            session_id_override,
            session_password,
            role,
        };

        match fetch_json::<_, SessionEnterResponse>(
            "POST",
            "/api/sessions/import",
            Some(&token),
            Some(&req),
        )
        .await
        {
            Ok(data) => {
                set_input_value("session-id-input", &data.session_id);
                set_feedback(
                    "session-feedback",
                    &format!(
                        "Imported & resumed session '{}' from '{file_name}'",
                        data.session_id
                    ),
                    false,
                );
                enter_session(data);
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Exports the current session's chat & mechanics transcript in Markdown (`"md"`) or JSON (`"json"`).
#[wasm_bindgen]
pub fn satya_export_chat_transcript(format: String) {
    spawn_local(async move {
        let fmt = if format.trim().eq_ignore_ascii_case("md")
            || format.trim().eq_ignore_ascii_case("markdown")
        {
            "md"
        } else {
            "json"
        };

        let (auth_token, active_sid) =
            with_state(|st| (st.auth_token.clone(), st.session_id.clone()));
        let session_id = active_sid
            .or_else(|| {
                let s = select_value("saved-sessions-select").trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            })
            .or_else(|| {
                let s = input_value("session-id-input").trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            })
            .unwrap_or_else(|| "satya-session".to_string());

        if let Some(tok) = auth_token {
            let url = format!("/api/sessions/{session_id}/transcript?format={fmt}");
            if fmt == "md" {
                if let Ok(md_text) = fetch_text("GET", &url, Some(&tok)).await {
                    let filename = format!("{session_id}-transcript.md");
                    let _ = prompt_save_bytes(&filename, md_text.as_bytes()).await;
                    append_system_notice(&format!(
                        "Exported Markdown session transcript `{filename}`."
                    ));
                    return;
                }
            } else if let Ok(tx_resp) =
                fetch_json::<(), SessionTranscriptResponse>("GET", &url, Some(&tok), None).await
                && let Ok(pretty_bytes) = serde_json::to_vec_pretty(&tx_resp)
            {
                let filename = format!("{session_id}-transcript.json");
                let _ = prompt_save_bytes(&filename, &pretty_bytes).await;
                append_system_notice(&format!(
                    "Exported JSON session transcript `{filename}` ({} items).",
                    tx_resp.total_items
                ));
                return;
            }
        }

        // Offline/DOM fallback if not connected to a server session
        let dom_text = get_element("chat-log")
            .and_then(|el| el.text_content())
            .unwrap_or_default();
        if fmt == "md" {
            let md = format!(
                "# Satya Chat Transcript (`{session_id}`)\n\n{}\n",
                dom_text.trim()
            );
            let filename = format!("{session_id}-transcript.md");
            let _ = prompt_save_bytes(&filename, md.as_bytes()).await;
        } else {
            let fallback_json = serde_json::json!({
                "session_id": session_id,
                "raw_dom_transcript": dom_text.trim(),
            });
            if let Ok(bytes) = serde_json::to_vec_pretty(&fallback_json) {
                let filename = format!("{session_id}-transcript.json");
                let _ = prompt_save_bytes(&filename, &bytes).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::{ChannelContext, EventMetadata, MessageVisibility, Participant};

    fn dummy_meta(seq: i64) -> EventMetadata {
        EventMetadata {
            event_id: format!("ev-{seq}"),
            sequence_num: seq,
            client_timestamp_us: 1_793_644_935_000_000 + seq * 1_000,
            ingest_timestamp_us: 1_793_644_935_000_000 + seq * 1_000,
            timezone_offset_min: 0,
            schema_subject: "io.satya.events.v1.ChatMessageEvent".to_string(),
            schema_version: 1,
            schema_fingerprint_crc64: vec![0; 8],
            session_id: "sector-4".to_string(),
            session_mode: SessionMode::Ttrpg,
        }
    }

    #[test]
    fn test_merge_history_events_by_sequence_interleaves_chat_and_mechanics() {
        let sara = Participant::human("u-sara", "sara", "Sara", SessionRole::Player);
        let chat_1 = ChatMessageEvent {
            metadata: dummy_meta(1),
            message_id: "m-1".to_string(),
            sender: sara.clone(),
            visibility: MessageVisibility::Public,
            recipients: vec![],
            channel_context: ChannelContext::InCharacter,
            reply_to_message_id: None,
            body: "Slicing the airlock keypad...".to_string(),
            char_count: 29,
            word_count: 4,
            composition_time_ms: 400,
            is_edited: false,
        };
        let chat_4 = ChatMessageEvent {
            metadata: dummy_meta(4),
            message_id: "m-4".to_string(),
            body: "We're through!".to_string(),
            ..chat_1.clone()
        };
        let mech_2 = TtrpgMechanicsEvent::evaluate_satya_task(
            dummy_meta(2),
            sara.clone(),
            vec![],
            "Intrusion".to_string(),
            13,
            2,
            vec![8, 12],
            None,
            0,
            0.0,
        );
        let mech_3 = TtrpgMechanicsEvent::evaluate_satya_task(
            dummy_meta(3),
            sara,
            vec![],
            "Athletics".to_string(),
            12,
            2,
            vec![15, 9],
            None,
            0,
            0.0,
        );

        let snap = SessionHistorySnapshot {
            chat_events: vec![chat_4, chat_1],
            mechanics_events: vec![mech_3, mech_2],
            ..Default::default()
        };

        let merged = merge_history_events_by_sequence(&snap);
        let seqs: Vec<(i64, bool)> = merged
            .iter()
            .map(|item| {
                (
                    item.sequence_num(),
                    matches!(item, HistoryReplayItem::Chat(_)),
                )
            })
            .collect();
        assert_eq!(seqs, vec![(1, true), (2, false), (3, false), (4, true)]);
    }
}
