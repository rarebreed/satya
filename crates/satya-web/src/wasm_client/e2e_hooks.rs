//! Read-only grey-box E2E inspection hooks (`#[cfg(feature = "e2e-hooks")]`).
//!
//! Exposes `window.__satyaE2eSnapshot()` returning serialized non-secret [`E2eClientSnapshot`]
//! metadata from [`SatyaWasmState`] so Playwright E2E tests can synchronize on Wasm/WebSocket
//! state and verify invariants without mutating state or reading secret tokens.

use crate::wasm_client::state::{SatyaWasmState, with_state};
use js_sys::Reflect;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Non-secret, read-only projection of [`SatyaWasmState`] exposed to Playwright E2E tests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct E2eClientSnapshot {
    /// Whether the client currently holds an authenticated session token.
    pub logged_in: bool,
    /// Authenticated user's persistent `user_id`, if logged in.
    pub user_id: Option<String>,
    /// Authenticated user's handle (`username`), if logged in.
    pub username: Option<String>,
    /// Platform account role (`ADMIN`, `GM`, `USER`), if logged in.
    pub account_role: Option<String>,
    /// Current progressive-disclosure lobby wizard step.
    pub lobby_step: String,
    /// Current lobby scope toggle (`Mine` vs `Search`).
    pub lobby_scope: String,
    /// Session IDs present in the client's saved-session list.
    pub saved_session_ids: Vec<String>,
    /// Campaign IDs present in the client's owned-campaign list.
    pub my_campaign_ids: Vec<String>,
    /// Active session room ID, if connected/entered.
    pub session_id: Option<String>,
    /// Business mode of the active or selected session (`Ttrpg` / `Meeting`).
    pub session_mode: String,
    /// Local user's role in the active session, if entered.
    pub self_role: Option<String>,
    /// Browser `WebSocket.readyState` (`0=CONNECTING`, `1=OPEN`, `2=CLOSING`, `3=CLOSED`).
    pub ws_ready_state: Option<u16>,
    /// Consecutive automatic WebSocket reconnect attempts.
    pub ws_reconnect_attempts: u32,
    /// User IDs currently in the session roster.
    pub roster_user_ids: Vec<String>,
    /// Remote user IDs with active WebRTC peer entries.
    pub webrtc_peer_ids: Vec<String>,
    /// Currently selected Left Tabletop Notebook tab slug.
    pub active_notebook_tab: String,
    /// Current combat round counter.
    pub combat_round: u32,
    /// Current combat action-count tick.
    pub combat_tick: u32,
    /// IDs of `GM_ONLY` documents currently cached on the client.
    pub gm_doc_ids: Vec<String>,
    /// IDs of GM memory notes currently cached on the client.
    pub gm_memory_ids: Vec<String>,
    /// Character IDs of NPCs currently cached in the NPC roster.
    pub npc_ids: Vec<String>,
    /// Whether the viewer is permitted to inspect GM-only NPC secrets.
    pub viewer_can_see_gm_secrets: bool,
    /// Whether Solo Co-GM Spoiler Shield is currently active.
    pub solo_spoiler_shield: bool,
}

/// Projects [`SatyaWasmState`] into an [`E2eClientSnapshot`], strictly omitting all secret tokens.
#[must_use]
pub fn build_e2e_client_snapshot(st: &SatyaWasmState) -> E2eClientSnapshot {
    E2eClientSnapshot {
        logged_in: st.auth_token.is_some(),
        user_id: st.user.as_ref().map(|u| u.user_id.clone()),
        username: st.user.as_ref().map(|u| u.username.clone()),
        account_role: st
            .user
            .as_ref()
            .map(|u| u.account_role.as_db_str().to_string()),
        lobby_step: format!("{:?}", st.lobby_step),
        lobby_scope: format!("{:?}", st.lobby_scope),
        saved_session_ids: st
            .saved_sessions
            .iter()
            .map(|s| s.session_id.clone())
            .collect(),
        my_campaign_ids: st
            .my_campaigns
            .iter()
            .map(|c| c.campaign_id.clone())
            .collect(),
        session_id: st.session_id.clone(),
        session_mode: format!("{:?}", st.session_mode),
        self_role: st
            .self_participant
            .as_ref()
            .map(|p| format!("{:?}", p.role)),
        ws_ready_state: st.ws.as_ref().map(web_sys::WebSocket::ready_state),
        ws_reconnect_attempts: st.ws_reconnect_attempts,
        roster_user_ids: st.roster.iter().map(|p| p.user_id.clone()).collect(),
        webrtc_peer_ids: st.peers.keys().cloned().collect(),
        active_notebook_tab: st.active_notebook_tab.clone(),
        combat_round: st.combat_round,
        combat_tick: st.combat_tick,
        gm_doc_ids: st.gm_documents.iter().map(|d| d.doc_id.clone()).collect(),
        gm_memory_ids: st.gm_memories.iter().map(|m| m.memory_id.clone()).collect(),
        npc_ids: st
            .session_npcs
            .iter()
            .map(|n| n.character_id.clone())
            .collect(),
        viewer_can_see_gm_secrets: st.viewer_can_see_gm_secrets,
        solo_spoiler_shield: st.solo_spoiler_shield,
    }
}

/// Registers `window.__satyaE2eSnapshot()` on `win` when compiled with `e2e-hooks`.
pub fn register_e2e_window_hooks(win: &web_sys::Window) {
    let snapshot_fn = Closure::<dyn Fn() -> String>::new(|| {
        let snap = with_state(build_e2e_client_snapshot);
        serde_json::to_string(&snap).unwrap_or_else(|_| "{}".to_string())
    });
    let _ = Reflect::set(
        win,
        &JsValue::from_str("__satyaE2eSnapshot"),
        snapshot_fn.as_ref(),
    );
    snapshot_fn.forget();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::state::{LobbyScope, LobbyStep};
    use satya_schema::{Participant, SessionMode, SessionRole};

    #[test]
    fn test_e2e_client_snapshot_projects_state_and_omits_secrets() {
        let mut st = SatyaWasmState::default();
        st.auth_token = Some("satya_tok_secret_auth_123".to_string());
        st.join_token = Some("join_secret_token_456".to_string());
        st.google_access_token = Some("ya29.secret_google_oauth_789".to_string());
        st.session_id = Some("daise-awakened-v1".to_string());
        st.session_mode = SessionMode::Ttrpg;
        st.lobby_step = LobbyStep::TakeAction;
        st.lobby_scope = LobbyScope::Mine;
        st.self_participant = Some(Participant::human(
            "u-gm-1",
            "gm_sean",
            "Sean (GM)",
            SessionRole::Gm,
        ));
        st.roster = vec![st.self_participant.clone().unwrap()];
        st.combat_round = 3;
        st.combat_tick = 14;
        st.viewer_can_see_gm_secrets = true;

        let snap = build_e2e_client_snapshot(&st);
        assert!(snap.logged_in);
        assert_eq!(snap.session_id.as_deref(), Some("daise-awakened-v1"));
        assert_eq!(snap.lobby_step, "TakeAction");
        assert_eq!(snap.lobby_scope, "Mine");
        assert_eq!(snap.self_role.as_deref(), Some("Gm"));
        assert_eq!(snap.roster_user_ids, vec!["u-gm-1"]);
        assert_eq!(snap.combat_round, 3);
        assert_eq!(snap.combat_tick, 14);
        assert!(snap.viewer_can_see_gm_secrets);

        let json = serde_json::to_string(&snap).unwrap();
        assert!(!json.contains("satya_tok_secret_auth_123"));
        assert!(!json.contains("join_secret_token_456"));
        assert!(!json.contains("ya29.secret_google_oauth_789"));
        assert!(!json.contains("auth_token"));
        assert!(!json.contains("join_token"));
        assert!(!json.contains("google_access_token"));
    }
}
