//! Client UI formatters, pure host-testable 3-step progressive-disclosure modal state,
//! and Wasm handlers for `User -> N Campaigns -> M Sessions`, directory search,
//! Request-to-Join, Request-New-Session, Quotas, and `/ws/notifications`.

use crate::wasm_client::auth_session::{create_session_in_campaign, enter_session};
use crate::wasm_client::dom::{
    escape_html, escape_js_single_quoted_attr, fetch_json, input_checked, input_value,
    select_value, set_class, set_display, set_feedback, set_html, set_input_value, set_text,
    slugify_identifier, window,
};
use crate::wasm_client::options::{parse_session_mode, parse_session_role};
use crate::wasm_client::session_persistence::{
    resume_session_by_id, satya_refresh_saved_sessions,
};
use crate::wasm_client::state::{
    CampaignSelectionKey, LobbyScope, LobbyStep, SatyaWasmState, SessionSelectionKey, with_state,
    with_state_mut,
};
use satya_schema::{
    AccountQuotaUsage, AccountWsPacket, CampaignListResponse, CampaignSummary,
    CreateCampaignRequest, CreateCampaignSessionRequestBody, CreateJoinRequestBody,
    DecideJoinRequestBody, DirectoryCampaignEntry, DirectoryGmEntry, DirectoryMembershipStatus,
    DirectorySearchResponse, DirectorySessionEntry, JoinRequestKind, JoinRequestRecord,
    JoinRequestStatus, JoinRequestsListResponse, JoinSessionRequest, SavedSessionSummary,
    SessionEnterResponse, SessionMode, SessionRole,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{KeyboardEvent, MessageEvent, WebSocket};

/// Output of [`format_lobby_action_panel_html`] driving Step 3 (`#lobby-step-3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LobbyActionView {
    /// Dynamic HTML rendered into `#lobby-dynamic-action-body` (Rows 1, 2, or 4).
    pub dynamic_html: String,
    /// Whether the SSR-mounted `#lobby-new-session-panel` is visible (Row 3).
    pub show_new_session_panel: bool,
}

/// Shared immutable inputs for rendering the 3-step guided Campaign -> Session modal.
pub struct LobbyRenderContext<'a> {
    /// Active lobby scope (`Mine` vs `Search`).
    pub scope: LobbyScope,
    /// Current wizard step (`ChooseCampaign`, `ChooseSession`, `TakeAction`).
    pub lobby_step: LobbyStep,
    /// Authenticated caller's `user_id`.
    pub my_user_id: Option<&'a str>,
    /// Campaigns owned by the caller.
    pub my_campaigns: &'a [CampaignSummary],
    /// Saved sessions in which the caller owns or participates.
    pub saved_sessions: &'a [SavedSessionSummary],
    /// Most recent directory search response.
    pub directory_results: Option<&'a DirectorySearchResponse>,
    /// Selected user inside `directory_results`.
    pub selected_dir_user_id: Option<&'a str>,
    /// Active campaign selection key.
    pub selected_campaign: &'a CampaignSelectionKey,
    /// Active session selection key.
    pub selected_session: &'a SessionSelectionKey,
    /// Caller's outbound requests (used to show pending new-session requests).
    pub my_join_requests: &'a [JoinRequestRecord],
    /// Whether the optional invite-password input is expanded in Row 2.
    pub show_join_password_box: bool,
}

fn active_owned_campaigns(campaigns: &[CampaignSummary]) -> Vec<&CampaignSummary> {
    campaigns.iter().filter(|c| !c.is_archived).collect()
}

fn is_owned_session(s: &SavedSessionSummary, my_user_id: Option<&str>) -> bool {
    let owner = if s.owner_user_id.trim().is_empty() {
        s.host_user_id.as_str()
    } else {
        s.owner_user_id.as_str()
    };
    my_user_id.is_none_or(|uid| uid == owner)
}

fn has_standalone_owned(saved_sessions: &[SavedSessionSummary], my_user_id: Option<&str>) -> bool {
    saved_sessions
        .iter()
        .any(|s| s.campaign_id.is_none() && is_owned_session(s, my_user_id))
}

fn has_joined_other(saved_sessions: &[SavedSessionSummary], my_user_id: Option<&str>) -> bool {
    let Some(uid) = my_user_id else {
        return false;
    };
    saved_sessions.iter().any(|s| {
        let owner = if s.owner_user_id.trim().is_empty() {
            s.host_user_id.as_str()
        } else {
            s.owner_user_id.as_str()
        };
        owner != uid
    })
}

fn selected_directory_user<'a>(
    directory_results: Option<&'a DirectorySearchResponse>,
    selected_dir_uid: Option<&str>,
) -> Option<&'a DirectoryGmEntry> {
    let resp = directory_results?;
    if resp.gms.is_empty() {
        return None;
    }
    selected_dir_uid
        .and_then(|uid| resp.gms.iter().find(|g| g.user_id == uid))
        .or_else(|| resp.gms.first())
}

fn find_selected_dir_campaign<'a>(
    gm: &'a DirectoryGmEntry,
    camp_id: Option<&str>,
) -> Option<&'a DirectoryCampaignEntry> {
    gm.campaigns
        .iter()
        .find(|c| c.campaign_id.as_deref() == camp_id)
}

fn sessions_for_mine_campaign<'a>(
    key: &CampaignSelectionKey,
    my_user_id: Option<&str>,
    saved_sessions: &'a [SavedSessionSummary],
) -> Vec<&'a SavedSessionSummary> {
    match key {
        CampaignSelectionKey::OwnedCampaign(cid) => saved_sessions
            .iter()
            .filter(|s| s.campaign_id.as_deref() == Some(cid.as_str()))
            .collect(),
        CampaignSelectionKey::StandaloneOwned => saved_sessions
            .iter()
            .filter(|s| s.campaign_id.is_none() && is_owned_session(s, my_user_id))
            .collect(),
        CampaignSelectionKey::JoinedOther => saved_sessions
            .iter()
            .filter(|s| !is_owned_session(s, my_user_id))
            .collect(),
        _ => Vec::new(),
    }
}

/// Keeps `lobby_step`, `selected_campaign_key`, and `selected_session_key` consistent
/// without ever skipping ahead past the user's current step.
pub fn ensure_valid_lobby_selection(st: &mut SatyaWasmState) {
    let my_uid = st.user.as_ref().map(|u| u.user_id.clone());
    match st.lobby_scope {
        LobbyScope::Mine => {
            if st.lobby_step == LobbyStep::ChooseCampaign {
                return;
            }
            let camp_valid = match &st.selected_campaign_key {
                CampaignSelectionKey::NewCampaign => true,
                CampaignSelectionKey::OwnedCampaign(cid) => st
                    .my_campaigns
                    .iter()
                    .any(|c| !c.is_archived && c.campaign_id == *cid),
                CampaignSelectionKey::StandaloneOwned => {
                    has_standalone_owned(&st.saved_sessions, my_uid.as_deref())
                }
                CampaignSelectionKey::JoinedOther => {
                    has_joined_other(&st.saved_sessions, my_uid.as_deref())
                }
                CampaignSelectionKey::None | CampaignSelectionKey::OtherCampaign { .. } => false,
            };
            if !camp_valid {
                st.selected_campaign_key = CampaignSelectionKey::None;
                st.selected_session_key = SessionSelectionKey::None;
                st.lobby_step = LobbyStep::ChooseCampaign;
                return;
            }
            if st.lobby_step == LobbyStep::TakeAction {
                let matching = sessions_for_mine_campaign(
                    &st.selected_campaign_key,
                    my_uid.as_deref(),
                    &st.saved_sessions,
                );
                let sess_valid = match &st.selected_session_key {
                    SessionSelectionKey::Existing(sid) => {
                        matching.iter().any(|s| s.session_id == *sid)
                    }
                    SessionSelectionKey::NewSession => {
                        !matches!(st.selected_campaign_key, CampaignSelectionKey::JoinedOther)
                    }
                    SessionSelectionKey::None => false,
                };
                if !sess_valid {
                    st.selected_session_key = SessionSelectionKey::None;
                    st.lobby_step = LobbyStep::ChooseSession;
                }
            }
        }
        LobbyScope::Search => {
            let Some(gm) = selected_directory_user(
                st.directory_results.as_ref(),
                st.selected_directory_user_id.as_deref(),
            ) else {
                st.selected_directory_user_id = None;
                st.selected_campaign_key = CampaignSelectionKey::None;
                st.selected_session_key = SessionSelectionKey::None;
                st.lobby_step = LobbyStep::ChooseCampaign;
                return;
            };
            let gm_uid = gm.user_id.clone();
            let gm_camps = gm.campaigns.clone();
            st.selected_directory_user_id = Some(gm_uid.clone());

            if st.lobby_step == LobbyStep::ChooseCampaign {
                return;
            }
            let camp_valid = match &st.selected_campaign_key {
                CampaignSelectionKey::OtherCampaign {
                    owner_user_id,
                    campaign_id,
                } if *owner_user_id == gm_uid => gm_camps
                    .iter()
                    .any(|c| c.campaign_id.as_deref() == campaign_id.as_deref()),
                _ => false,
            };
            if !camp_valid {
                st.selected_campaign_key = CampaignSelectionKey::None;
                st.selected_session_key = SessionSelectionKey::None;
                st.lobby_step = LobbyStep::ChooseCampaign;
                return;
            }
            if st.lobby_step == LobbyStep::TakeAction {
                let selected_camp_id = match &st.selected_campaign_key {
                    CampaignSelectionKey::OtherCampaign { campaign_id, .. } => {
                        campaign_id.as_deref()
                    }
                    _ => None,
                };
                let Some(camp) = gm_camps
                    .iter()
                    .find(|c| c.campaign_id.as_deref() == selected_camp_id)
                else {
                    st.selected_session_key = SessionSelectionKey::None;
                    st.lobby_step = LobbyStep::ChooseSession;
                    return;
                };
                let can_request_new =
                    camp.campaign_id.is_some() && my_uid.as_deref() != Some(gm_uid.as_str());
                let sess_valid = match &st.selected_session_key {
                    SessionSelectionKey::Existing(sid) => {
                        camp.sessions.iter().any(|s| s.session_id == *sid)
                    }
                    SessionSelectionKey::NewSession => can_request_new,
                    SessionSelectionKey::None => false,
                };
                if !sess_valid {
                    st.selected_session_key = SessionSelectionKey::None;
                    st.lobby_step = LobbyStep::ChooseSession;
                }
            }
        }
    }
}

/// Formats clickable breadcrumb step pills (`#lobby-step-breadcrumb`).
#[must_use]
pub fn format_lobby_breadcrumb_html(step: LobbyStep, selected_campaign: &CampaignSelectionKey) -> String {
    let step1 = match step {
        LobbyStep::ChooseCampaign => r#"<span class="lobby-crumb is-active">1. Campaign</span>"#.to_string(),
        _ => r#"<button type="button" class="lobby-crumb is-done" onclick="window.satyaLobbyGoStep(1)">1. Campaign</button>"#.to_string(),
    };
    let step2_label = if matches!(selected_campaign, CampaignSelectionKey::NewCampaign) {
        "2. New Campaign"
    } else {
        "2. Session"
    };
    let step2 = match step {
        LobbyStep::ChooseCampaign => format!(r#"<span class="lobby-crumb">{step2_label}</span>"#),
        LobbyStep::ChooseSession => format!(r#"<span class="lobby-crumb is-active">{step2_label}</span>"#),
        LobbyStep::TakeAction => format!(
            r#"<button type="button" class="lobby-crumb is-done" onclick="window.satyaLobbyGoStep(2)">{step2_label}</button>"#
        ),
    };
    let step3 = if step == LobbyStep::TakeAction {
        r#"<span class="lobby-crumb is-active">3. Action</span>"#.to_string()
    } else {
        r#"<span class="lobby-crumb">3. Action</span>"#.to_string()
    };
    format!(
        r#"{step1} <span class="lobby-crumb-sep">›</span> {step2} <span class="lobby-crumb-sep">›</span> {step3}"#
    )
}

/// Formats clickable matched-user selector chips (`#directory-user-chips`).
#[must_use]
pub fn format_directory_user_chips_html(
    resp: &DirectorySearchResponse,
    selected_uid: Option<&str>,
) -> String {
    if resp.gms.is_empty() {
        if resp.query.trim().is_empty() {
            return String::new();
        }
        return format!(
            r#"<span class="subtitle">No users found matching &quot;{}&quot;.</span>"#,
            escape_html(&resp.query)
        );
    }
    let active_uid = selected_uid.unwrap_or(&resp.gms[0].user_id);
    let mut out = String::with_capacity(resp.gms.len() * 160);
    for gm in &resp.gms {
        let is_sel = gm.user_id == active_uid;
        let cls = if is_sel { "btn-chip active" } else { "btn-chip" };
        let status_dot = if gm.is_online { "●" } else { "○" };
        let uid_js = escape_js_single_quoted_attr(&gm.user_id);
        out.push_str(&format!(
            r#"<button type="button" class="{cls}" onclick="window.satyaSelectDirectoryUser('{uid_js}')">{status_dot} {dname} (@{uname})</button>"#,
            dname = escape_html(&gm.display_name),
            uname = escape_html(&gm.username),
        ));
    }
    out
}

/// Formats Step 1 campaign choices (`#campaign-list-pane`).
///
/// Row 6 invariant: `+ Create New Campaign` is **never** rendered in [`LobbyScope::Search`].
#[must_use]
pub fn format_lobby_campaign_list_html(ctx: &LobbyRenderContext<'_>) -> String {
    match ctx.scope {
        LobbyScope::Mine => {
            let active = active_owned_campaigns(ctx.my_campaigns);
            let mut out = String::with_capacity((active.len() + 3) * 240);

            for c in active {
                let cid_js = escape_js_single_quoted_attr(&c.campaign_id);
                let sess_count = ctx
                    .saved_sessions
                    .iter()
                    .filter(|s| s.campaign_id.as_deref() == Some(c.campaign_id.as_str()))
                    .count()
                    .max(c.session_count);
                let desc = if c.description.trim().is_empty() {
                    if c.is_discoverable {
                        "Discoverable campaign".to_string()
                    } else {
                        "Private campaign".to_string()
                    }
                } else {
                    escape_html(&c.description)
                };
                out.push_str(&format!(
                    r#"<div class="lobby-card-row">
                        <button type="button" class="lobby-card-select" onclick="window.satyaSelectLobbyCampaign('owned', '{cid_js}')">
                            <div class="lobby-card-main">
                                <strong>{title}</strong>
                                <span class="subtitle">{desc}</span>
                            </div>
                            <span class="lobby-card-meta">
                                <span class="pill pill-muted">{sess_count} sess</span>
                                <span class="lobby-chevron">›</span>
                            </span>
                        </button>
                        <button type="button" class="lobby-row-del" title="Delete campaign" aria-label="Delete campaign" onclick="window.satyaDeleteCampaign('{cid_js}')">×</button>
                    </div>"#,
                    title = escape_html(&c.title),
                ));
            }

            if has_standalone_owned(ctx.saved_sessions, ctx.my_user_id) {
                let count = ctx
                    .saved_sessions
                    .iter()
                    .filter(|s| s.campaign_id.is_none() && is_owned_session(s, ctx.my_user_id))
                    .count();
                out.push_str(&format!(
                    r#"<div class="lobby-card-row">
                        <button type="button" class="lobby-card-select" onclick="window.satyaSelectLobbyCampaign('standalone', '')">
                            <div class="lobby-card-main">
                                <strong>Standalone Sessions</strong>
                                <span class="subtitle">Sessions not grouped in a campaign</span>
                            </div>
                            <span class="lobby-card-meta">
                                <span class="pill pill-muted">{count} sess</span>
                                <span class="lobby-chevron">›</span>
                            </span>
                        </button>
                    </div>"#
                ));
            }

            if has_joined_other(ctx.saved_sessions, ctx.my_user_id) {
                let count = ctx
                    .saved_sessions
                    .iter()
                    .filter(|s| !is_owned_session(s, ctx.my_user_id))
                    .count();
                out.push_str(&format!(
                    r#"<div class="lobby-card-row">
                        <button type="button" class="lobby-card-select" onclick="window.satyaSelectLobbyCampaign('joined', '')">
                            <div class="lobby-card-main">
                                <strong>Joined Sessions</strong>
                                <span class="subtitle">Other users' sessions you belong to</span>
                            </div>
                            <span class="lobby-card-meta">
                                <span class="pill pill-muted">{count} sess</span>
                                <span class="lobby-chevron">›</span>
                            </span>
                        </button>
                    </div>"#
                ));
            }

            out.push_str(
                r#"<div class="lobby-card-row is-new-action">
                    <button type="button" class="lobby-card-select" onclick="window.satyaSelectLobbyCampaign('new', '')">
                        <div class="lobby-card-main">
                            <strong>+ Create New Campaign</strong>
                            <span class="subtitle">Start a new campaign and launch its first session</span>
                        </div>
                        <span class="lobby-chevron">›</span>
                    </button>
                </div>"#,
            );
            out
        }
        LobbyScope::Search => {
            let Some(gm) = selected_directory_user(ctx.directory_results, ctx.selected_dir_user_id)
            else {
                return r#"<div class="lobby-empty-hint">Search by email or @handle above to view another user's campaigns.</div>"#
                    .to_string();
            };
            if gm.campaigns.is_empty() {
                return format!(
                    r#"<div class="lobby-empty-hint">@{} has no discoverable campaigns yet.</div>"#,
                    escape_html(&gm.username)
                );
            }
            let mut out = String::with_capacity(gm.campaigns.len() * 220);
            for camp in &gm.campaigns {
                let cid_js =
                    escape_js_single_quoted_attr(camp.campaign_id.as_deref().unwrap_or(""));
                let desc = if camp.description.trim().is_empty() {
                    format!("Owned by @{}", escape_html(&gm.username))
                } else {
                    escape_html(&camp.description)
                };
                out.push_str(&format!(
                    r#"<div class="lobby-card-row">
                        <button type="button" class="lobby-card-select" onclick="window.satyaSelectLobbyCampaign('other', '{cid_js}')">
                            <div class="lobby-card-main">
                                <strong>{title}</strong>
                                <span class="subtitle">{desc}</span>
                            </div>
                            <span class="lobby-card-meta">
                                <span class="pill pill-muted">{count} sess</span>
                                <span class="lobby-chevron">›</span>
                            </span>
                        </button>
                    </div>"#,
                    title = escape_html(&camp.title),
                    count = camp.sessions.len(),
                ));
            }
            out
        }
    }
}

fn membership_badge_html(s: &DirectorySessionEntry) -> &'static str {
    match s.caller_membership_status {
        DirectoryMembershipStatus::Owner => r#"<span class="pill pill-active">Owner</span>"#,
        DirectoryMembershipStatus::Member => r#"<span class="pill pill-active">Member</span>"#,
        DirectoryMembershipStatus::ApprovedReady => r#"<span class="pill pill-active">Approved</span>"#,
        DirectoryMembershipStatus::PendingRequest => r#"<span class="pill pill-warn">Pending</span>"#,
        DirectoryMembershipStatus::Denied => r#"<span class="pill pill-muted">Denied</span>"#,
        DirectoryMembershipStatus::None => {
            if s.allow_join_requests {
                ""
            } else {
                r#"<span class="pill pill-muted">Password Only</span>"#
            }
        }
    }
}

/// Formats Step 2 session choices (`#session-list-pane`).
#[must_use]
pub fn format_lobby_session_list_html(ctx: &LobbyRenderContext<'_>) -> String {
    match ctx.selected_campaign {
        CampaignSelectionKey::OwnedCampaign(_)
        | CampaignSelectionKey::StandaloneOwned
        | CampaignSelectionKey::JoinedOther => {
            let sessions =
                sessions_for_mine_campaign(ctx.selected_campaign, ctx.my_user_id, ctx.saved_sessions);
            let mut out = String::with_capacity((sessions.len() + 1) * 220);
            for s in sessions {
                let sid_js = escape_js_single_quoted_attr(&s.session_id);
                let label = s.title.as_deref().unwrap_or(&s.session_id);
                let live_badge = if s.active_human_count > 0 {
                    format!(
                        r#"<span class="pill pill-active">Live ({})</span>"#,
                        s.active_human_count
                    )
                } else {
                    format!(r#"<span class="pill pill-muted">{:?}</span>"#, s.role)
                };
                out.push_str(&format!(
                    r#"<div class="lobby-card-row">
                        <button type="button" class="lobby-card-select" onclick="window.satyaSelectLobbySession('existing', '{sid_js}')">
                            <div class="lobby-card-main">
                                <strong>{label}</strong>
                                <span class="subtitle">{chats} msgs · {rolls} rolls</span>
                            </div>
                            <span class="lobby-card-meta">
                                {live_badge}
                                <span class="lobby-chevron">›</span>
                            </span>
                        </button>
                    </div>"#,
                    label = escape_html(label),
                    chats = s.chat_event_count,
                    rolls = s.mechanics_event_count,
                ));
            }
            if !matches!(ctx.selected_campaign, CampaignSelectionKey::JoinedOther) {
                out.push_str(
                    r#"<div class="lobby-card-row is-new-action">
                        <button type="button" class="lobby-card-select" onclick="window.satyaSelectLobbySession('new', '')">
                            <div class="lobby-card-main">
                                <strong>+ Create New Session</strong>
                                <span class="subtitle">Launch a new session inside this campaign</span>
                            </div>
                            <span class="lobby-chevron">›</span>
                        </button>
                    </div>"#,
                );
            }
            out
        }
        CampaignSelectionKey::OtherCampaign {
            owner_user_id,
            campaign_id,
        } => {
            let Some(gm) = selected_directory_user(ctx.directory_results, Some(owner_user_id))
            else {
                return r#"<div class="lobby-empty-hint">No user selected.</div>"#.to_string();
            };
            let Some(camp) = find_selected_dir_campaign(gm, campaign_id.as_deref()) else {
                return r#"<div class="lobby-empty-hint">Campaign not found.</div>"#.to_string();
            };
            let mut out = String::with_capacity((camp.sessions.len() + 2) * 240);
            for s in &camp.sessions {
                let sid_js = escape_js_single_quoted_attr(&s.session_id);
                let label = s.title.as_deref().unwrap_or(&s.session_id);
                let live_badge = if s.is_live {
                    format!(
                        r#"<span class="pill pill-active">Live ({})</span>"#,
                        s.active_human_count
                    )
                } else {
                    String::new()
                };
                let status_badge = membership_badge_html(s);
                out.push_str(&format!(
                    r#"<div class="lobby-card-row">
                        <button type="button" class="lobby-card-select" onclick="window.satyaSelectLobbySession('existing', '{sid_js}')">
                            <div class="lobby-card-main">
                                <strong>{label}</strong>
                                <span class="mono subtitle">{sid_html}</span>
                            </div>
                            <span class="lobby-card-meta">
                                {live_badge}{status_badge}
                                <span class="lobby-chevron">›</span>
                            </span>
                        </button>
                    </div>"#,
                    label = escape_html(label),
                    sid_html = escape_html(&s.session_id),
                ));
            }
            if let Some(cid) = camp.campaign_id.as_deref() {
                for r in ctx.my_join_requests.iter().filter(|r| {
                    r.request_kind == JoinRequestKind::CreateSession
                        && r.campaign_id.as_deref() == Some(cid)
                        && matches!(
                            r.status,
                            JoinRequestStatus::Pending | JoinRequestStatus::Approved
                        )
                        && !camp.sessions.iter().any(|s| s.session_id == r.session_id)
                }) {
                    let title = r.session_title.as_deref().unwrap_or(&r.session_id);
                    let badge = if r.status == JoinRequestStatus::Approved {
                        r#"<span class="pill pill-active">Ready</span>"#
                    } else {
                        r#"<span class="pill pill-warn">Requested</span>"#
                    };
                    out.push_str(&format!(
                        r#"<div class="lobby-card-row is-muted-row">
                            <div class="lobby-card-static">
                                <div class="lobby-card-main">
                                    <strong>{title}</strong>
                                    <span class="subtitle">Awaiting owner creation</span>
                                </div>
                                {badge}
                            </div>
                        </div>"#,
                        title = escape_html(title),
                    ));
                }
                if ctx.my_user_id != Some(owner_user_id.as_str()) {
                    out.push_str(
                        r#"<div class="lobby-card-row is-new-action">
                            <button type="button" class="lobby-card-select" onclick="window.satyaSelectLobbySession('new', '')">
                                <div class="lobby-card-main">
                                    <strong>+ Request New Session</strong>
                                    <span class="subtitle">Ask @{gm_uname} to start a new session here</span>
                                </div>
                                <span class="lobby-chevron">›</span>
                            </button>
                        </div>"#,
                    );
                }
            }
            out.replace("@{gm_uname}", &format!("@{}", escape_html(&gm.username)))
        }
        _ => String::new(),
    }
}

/// Computes Step 3 (`#lobby-step-3`) action view for the chosen Matrix row.
#[must_use]
pub fn format_lobby_action_panel_html(ctx: &LobbyRenderContext<'_>) -> LobbyActionView {
    match (ctx.selected_campaign, ctx.selected_session) {
        // Row 3: Existing | Self | New Session
        (
            CampaignSelectionKey::OwnedCampaign(_) | CampaignSelectionKey::StandaloneOwned,
            SessionSelectionKey::NewSession,
        ) => LobbyActionView {
            dynamic_html: String::new(),
            show_new_session_panel: true,
        },
        // Row 1 (and JoinedOther existing session): Existing | Self | Existing
        (
            CampaignSelectionKey::OwnedCampaign(_)
            | CampaignSelectionKey::StandaloneOwned
            | CampaignSelectionKey::JoinedOther,
            SessionSelectionKey::Existing(sid),
        ) => {
            let Some(s) = ctx.saved_sessions.iter().find(|x| &x.session_id == sid) else {
                return LobbyActionView {
                    dynamic_html: String::new(),
                    show_new_session_panel: false,
                };
            };
            let sid_js = escape_js_single_quoted_attr(&s.session_id);
            let sid_html = escape_html(&s.session_id);
            let mode_txt = match s.session_mode {
                SessionMode::Ttrpg => {
                    if s.has_ai_gm {
                        "TTRPG + AI GM"
                    } else {
                        "TTRPG"
                    }
                }
                SessionMode::Meeting => "Meeting",
            };
            let solo_btn = if s.has_ai_gm {
                format!(
                    r#"<button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaResumeSelectedSession('{sid_js}', true)">Resume as Solo Co-GM</button>"#
                )
            } else {
                String::new()
            };
            let del_btn = if is_owned_session(s, ctx.my_user_id) {
                format!(
                    r#"<button type="button" class="btn btn-sm btn-danger" onclick="window.satyaDeleteSelectedSession('{sid_js}')">Delete Session</button>"#
                )
            } else {
                String::new()
            };
            let dynamic_html = format!(
                r#"<div class="lobby-action-card">
                    <div class="subtitle">Room <span class="mono">{sid_html}</span> · {mode_txt} · {chats} msgs, {rolls} rolls</div>
                    <div class="btn-row" style="margin-top: 8px;">
                        <button type="button" class="btn btn-primary" onclick="window.satyaResumeSelectedSession('{sid_js}', false)">Join / Resume Session</button>
                        {solo_btn}
                    </div>
                    <div class="btn-row" style="margin-top: 8px;">
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaExportSessionSnapshot()">Export .satyasession</button>
                        {del_btn}
                    </div>
                </div>"#,
                chats = s.chat_event_count,
                rolls = s.mechanics_event_count,
            );
            LobbyActionView {
                dynamic_html,
                show_new_session_panel: false,
            }
        }
        // Row 2: Existing | Other | Existing Session
        (
            CampaignSelectionKey::OtherCampaign {
                owner_user_id,
                campaign_id,
            },
            SessionSelectionKey::Existing(sid),
        ) => {
            let session_opt = selected_directory_user(ctx.directory_results, Some(owner_user_id))
                .and_then(|gm| find_selected_dir_campaign(gm, campaign_id.as_deref()))
                .and_then(|c| c.sessions.iter().find(|s| &s.session_id == sid));
            let Some(s) = session_opt else {
                return LobbyActionView {
                    dynamic_html: String::new(),
                    show_new_session_panel: false,
                };
            };
            let sid_js = escape_js_single_quoted_attr(&s.session_id);
            let sid_html = escape_html(&s.session_id);
            let body = match s.caller_membership_status {
                DirectoryMembershipStatus::Owner | DirectoryMembershipStatus::Member => {
                    format!(
                        r#"<div class="btn-row"><button type="button" class="btn btn-primary" onclick="window.satyaResumeSelectedSession('{sid_js}', false)">Join / Resume Session</button></div>"#
                    )
                }
                DirectoryMembershipStatus::ApprovedReady => {
                    if let Some(ref tok) = s.approval_token {
                        let tok_js = escape_js_single_quoted_attr(tok);
                        format!(
                            r#"<div class="btn-row"><button type="button" class="btn btn-accent" onclick="window.satyaJoinApprovedSession('{sid_js}', '{tok_js}')">Join Approved Session</button></div>"#
                        )
                    } else {
                        r#"<span class="pill pill-active">Approved</span>"#.to_string()
                    }
                }
                DirectoryMembershipStatus::PendingRequest => {
                    let cancel_btn = s
                        .caller_request_id
                        .as_deref()
                        .map(|rid| {
                            let rid_js = escape_js_single_quoted_attr(rid);
                            format!(
                                r#"<button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCancelJoinRequest('{rid_js}')">Cancel Request</button>"#
                            )
                        })
                        .unwrap_or_default();
                    format!(
                        r#"<div class="btn-row" style="align-items: center;"><span class="pill pill-warn">Request Pending</span> {cancel_btn}</div>"#
                    )
                }
                DirectoryMembershipStatus::None | DirectoryMembershipStatus::Denied => {
                    let pw_block = format!(
                        r#"<div class="lobby-stack-form" style="margin-top: 8px;">
                            <label>
                                <span>Invite Password</span>
                                <input id="dir-join-password-input" type="password" placeholder="Enter session password..."/>
                            </label>
                            <div class="btn-row">
                                <button type="button" class="btn btn-secondary" onclick="window.satyaJoinDirectorySessionWithPassword('{sid_js}')">Join with Password</button>
                            </div>
                        </div>"#
                    );
                    if s.allow_join_requests {
                        let pw_section = if ctx.show_join_password_box {
                            pw_block
                        } else {
                            r#"<div style="margin-top: 8px;">
                                <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaToggleDirPasswordJoin()">Or join with an invite password</button>
                            </div>"#
                                .to_string()
                        };
                        format!(
                            r#"<div class="lobby-stack-form">
                                <label>
                                    <span>Requested Role</span>
                                    <select id="dir-join-role-select">
                                        <option value="PLAYER">Player</option>
                                        <option value="OBSERVER">Observer</option>
                                    </select>
                                </label>
                                <div class="btn-row">
                                    <button type="button" class="btn btn-primary" onclick="window.satyaRequestJoinSession('{sid_js}')">Request to Join</button>
                                </div>
                            </div>
                            {pw_section}"#
                        )
                    } else {
                        pw_block
                    }
                }
            };
            let dynamic_html = format!(
                r#"<div class="lobby-action-card">
                    <div class="subtitle" style="margin-bottom: 8px;">Room <span class="mono">{sid_html}</span> · {:?}</div>
                    {body}
                </div>"#,
                s.session_mode,
            );
            LobbyActionView {
                dynamic_html,
                show_new_session_panel: false,
            }
        }
        // Row 4: Existing | Other | New Session -> Request to create new session
        (
            CampaignSelectionKey::OtherCampaign {
                campaign_id: Some(cid),
                ..
            },
            SessionSelectionKey::NewSession,
        ) => {
            let cid_js = escape_js_single_quoted_attr(cid);
            let dynamic_html = format!(
                r#"<div class="lobby-stack-form">
                    <label>
                        <span>Proposed Session Title</span>
                        <input id="req-session-title-input" type="text" placeholder="e.g. Ep. 2: Cold Dock"/>
                    </label>
                    <div class="lobby-form-row">
                        <label>
                            <span>Mode</span>
                            <select id="req-session-mode-select">
                                <option value="TTRPG">TTRPG</option>
                                <option value="TTRPG_AI_GM">TTRPG + AI GM</option>
                                <option value="MEETING">Meeting</option>
                            </select>
                        </label>
                        <label>
                            <span>Requested Role</span>
                            <select id="req-session-role-select">
                                <option value="PLAYER">Player</option>
                                <option value="OBSERVER">Observer</option>
                            </select>
                        </label>
                    </div>
                    <label>
                        <span>Note to Owner (Optional)</span>
                        <input id="req-session-note-input" type="text" placeholder="Short pitch for the campaign owner"/>
                    </label>
                    <div class="btn-row" style="margin-top: 4px;">
                        <button type="button" class="btn btn-accent" onclick="window.satyaRequestCampaignSession('{cid_js}')">Send New Session Request</button>
                    </div>
                </div>"#
            );
            LobbyActionView {
                dynamic_html,
                show_new_session_panel: false,
            }
        }
        _ => LobbyActionView {
            dynamic_html: String::new(),
            show_new_session_panel: false,
        },
    }
}

/// Formats incoming join and new-session requests for `#gm-join-requests-list`.
#[must_use]
pub fn format_gm_join_requests_html(requests: &[JoinRequestRecord]) -> String {
    let pending: Vec<&JoinRequestRecord> = requests
        .iter()
        .filter(|r| r.status == JoinRequestStatus::Pending)
        .collect();
    if pending.is_empty() {
        return r#"<div class="subtitle">No pending incoming requests.</div>"#.to_string();
    }
    let mut out = String::with_capacity(pending.len() * 300);
    for r in pending {
        let rid_js = escape_js_single_quoted_attr(&r.request_id);
        let msg_part = r
            .message
            .as_deref()
            .filter(|m| !m.is_empty())
            .map(|m| format!(r#" — &quot;{}&quot;"#, escape_html(m)))
            .unwrap_or_default();
        let summary = match r.request_kind {
            JoinRequestKind::CreateSession => {
                let stitle = r.session_title.as_deref().unwrap_or(&r.session_id);
                let camp = r.campaign_title.as_deref().unwrap_or("campaign");
                format!(
                    r#"wants a new session <strong>{}</strong> in <strong>{}</strong> as <span class="mono">{:?}</span>"#,
                    escape_html(stitle),
                    escape_html(camp),
                    r.requested_role
                )
            }
            JoinRequestKind::JoinSession => format!(
                r#"wants to join <strong>{}</strong> as <span class="mono">{:?}</span>"#,
                escape_html(&r.session_id),
                r.requested_role
            ),
        };
        out.push_str(&format!(
            r#"<div class="dir-session-row">
                <div>
                    <strong>{player}</strong> (<span class="mono">@{uname}</span>) {summary}{msg_part}
                </div>
                <div class="dir-session-actions">
                    <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaDecideJoinRequest('{rid_js}', true)">Approve</button>
                    <button type="button" class="btn btn-sm btn-danger" onclick="window.satyaDecideJoinRequest('{rid_js}', false)">Deny</button>
                </div>
            </div>"#,
            player = escape_html(&r.requester_display_name),
            uname = escape_html(&r.requester_username),
        ));
    }
    out
}

/// Formats outbound join and new-session requests for `#my-join-requests-list`.
#[must_use]
pub fn format_my_join_requests_html(requests: &[JoinRequestRecord]) -> String {
    let active: Vec<&JoinRequestRecord> = requests
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                JoinRequestStatus::Pending | JoinRequestStatus::Approved
            )
        })
        .collect();
    if active.is_empty() {
        return String::new();
    }
    let mut out = String::from(r#"<div class="menu-section-title">Your Outbound Requests</div>"#);
    for r in active {
        let sid_js = escape_js_single_quoted_attr(&r.session_id);
        let rid_js = escape_js_single_quoted_attr(&r.request_id);
        let kind_label = if r.request_kind == JoinRequestKind::CreateSession {
            "New session: "
        } else {
            ""
        };
        let label = r.session_title.as_deref().unwrap_or(&r.session_id);
        let action = if r.status == JoinRequestStatus::Approved {
            if let Some(ref tok) = r.approval_token {
                let tok_js = escape_js_single_quoted_attr(tok);
                format!(
                    r#"<button type="button" class="btn btn-sm btn-accent" onclick="window.satyaJoinApprovedSession('{sid_js}', '{tok_js}')">Join Approved Session</button>"#
                )
            } else {
                r#"<span class="pill pill-active">Approved</span>"#.to_string()
            }
        } else {
            format!(
                r#"<span class="pill pill-muted">Pending</span> <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCancelJoinRequest('{rid_js}')">Cancel</button>"#
            )
        };
        out.push_str(&format!(
            r#"<div class="dir-session-row">
                <div>{kind_label}<strong>{label}</strong> (Owner: @{gm})</div>
                <div class="dir-session-actions">{action}</div>
            </div>"#,
            label = escape_html(label),
            gm = escape_html(&r.owner_username),
        ));
    }
    out
}

/// Formats the header notification popover items (`#notifications-popover-list`).
#[must_use]
pub fn format_notifications_popover_html(
    incoming: &[JoinRequestRecord],
    mine: &[JoinRequestRecord],
) -> String {
    let mut html = String::new();
    let pending_in: Vec<&JoinRequestRecord> = incoming
        .iter()
        .filter(|r| r.status == JoinRequestStatus::Pending)
        .collect();
    let ready_mine: Vec<&JoinRequestRecord> = mine
        .iter()
        .filter(|r| r.status == JoinRequestStatus::Approved && r.approval_token.is_some())
        .collect();

    if pending_in.is_empty() && ready_mine.is_empty() {
        return r#"<span class="subtitle">No pending alerts.</span>"#.to_string();
    }
    for r in pending_in {
        let rid_js = escape_js_single_quoted_attr(&r.request_id);
        let label = r.session_title.as_deref().unwrap_or(&r.session_id);
        let verb = if r.request_kind == JoinRequestKind::CreateSession {
            "requested new session"
        } else {
            "requested"
        };
        html.push_str(&format!(
            r#"<div class="notif-item">
                <div><strong>@{uname}</strong> {verb} <strong>{label}</strong></div>
                <div class="dir-session-actions">
                    <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaDecideJoinRequest('{rid_js}', true)">Approve</button>
                    <button type="button" class="btn btn-sm btn-danger" onclick="window.satyaDecideJoinRequest('{rid_js}', false)">Deny</button>
                </div>
            </div>"#,
            uname = escape_html(&r.requester_username),
            label = escape_html(label),
        ));
    }
    for r in ready_mine {
        let sid_js = escape_js_single_quoted_attr(&r.session_id);
        let tok_js = escape_js_single_quoted_attr(r.approval_token.as_deref().unwrap_or_default());
        let label = r.session_title.as_deref().unwrap_or(&r.session_id);
        html.push_str(&format!(
            r#"<div class="notif-item">
                <div>Approved for <strong>{label}</strong>!</div>
                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaJoinApprovedSession('{sid_js}', '{tok_js}')">Join Now</button>
            </div>"#,
            label = escape_html(label),
        ));
    }
    html
}

fn step2_context_html(st: &SatyaWasmState) -> String {
    match &st.selected_campaign_key {
        CampaignSelectionKey::NewCampaign => {
            r#"<h2>Create a New Campaign</h2><span class="subtitle">Group related sessions, maps, and notes under one campaign.</span>"#.to_string()
        }
        CampaignSelectionKey::OwnedCampaign(cid) => {
            let camp = st.my_campaigns.iter().find(|c| &c.campaign_id == cid);
            let title = camp.map(|c| c.title.as_str()).unwrap_or("Campaign");
            format!(
                r#"<h2>{}</h2><span class="subtitle">Choose an existing session or create a new one.</span>"#,
                escape_html(title)
            )
        }
        CampaignSelectionKey::StandaloneOwned => {
            r#"<h2>Standalone Sessions</h2><span class="subtitle">Choose an ungrouped session or create a new one.</span>"#.to_string()
        }
        CampaignSelectionKey::JoinedOther => {
            r#"<h2>Joined Sessions</h2><span class="subtitle">Select a session you belong to.</span>"#.to_string()
        }
        CampaignSelectionKey::OtherCampaign {
            owner_user_id,
            campaign_id,
        } => {
            let gm_opt =
                selected_directory_user(st.directory_results.as_ref(), Some(owner_user_id));
            let camp_title = gm_opt
                .and_then(|gm| find_selected_dir_campaign(gm, campaign_id.as_deref()))
                .map(|c| c.title.as_str())
                .unwrap_or("Campaign");
            let owner_uname = gm_opt.map(|g| g.username.as_str()).unwrap_or("owner");
            format!(
                r#"<h2>{}</h2><span class="subtitle">Hosted by @{} · Choose a session or request a new one.</span>"#,
                escape_html(camp_title),
                escape_html(owner_uname)
            )
        }
        CampaignSelectionKey::None => String::new(),
    }
}

fn step3_context_html(st: &SatyaWasmState) -> String {
    let camp_name = match &st.selected_campaign_key {
        CampaignSelectionKey::OwnedCampaign(cid) => st
            .my_campaigns
            .iter()
            .find(|c| &c.campaign_id == cid)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| "Campaign".to_string()),
        CampaignSelectionKey::StandaloneOwned => "Standalone Sessions".to_string(),
        CampaignSelectionKey::JoinedOther => "Joined Sessions".to_string(),
        CampaignSelectionKey::OtherCampaign {
            owner_user_id,
            campaign_id,
        } => selected_directory_user(st.directory_results.as_ref(), Some(owner_user_id))
            .and_then(|gm| find_selected_dir_campaign(gm, campaign_id.as_deref()))
            .map(|c| c.title.clone())
            .unwrap_or_else(|| "Campaign".to_string()),
        CampaignSelectionKey::NewCampaign | CampaignSelectionKey::None => "Campaign".to_string(),
    };
    let heading = match &st.selected_session_key {
        SessionSelectionKey::NewSession => match &st.selected_campaign_key {
            CampaignSelectionKey::OtherCampaign { .. } => "Request a New Session".to_string(),
            _ => "Create a New Session".to_string(),
        },
        SessionSelectionKey::Existing(sid) => st
            .saved_sessions
            .iter()
            .find(|s| &s.session_id == sid)
            .and_then(|s| s.title.clone())
            .or_else(|| {
                st.directory_results
                    .as_ref()
                    .and_then(|d| {
                        d.gms
                            .iter()
                            .flat_map(|g| &g.campaigns)
                            .flat_map(|c| &c.sessions)
                            .find(|s| &s.session_id == sid)
                            .and_then(|s| s.title.clone())
                    })
            })
            .unwrap_or_else(|| sid.clone()),
        SessionSelectionKey::None => "Session".to_string(),
    };
    format!(
        r#"<h2>{}</h2><span class="subtitle">in {}</span>"#,
        escape_html(&heading),
        escape_html(&camp_name),
    )
}

/// Synchronizes the 3-step progressive-disclosure modal DOM from `SatyaWasmState`.
pub(crate) fn render_session_lobby_ui() {
    let (
        scope,
        step,
        is_new_campaign,
        breadcrumb_html,
        user_chips_html,
        camp_html,
        step2_ctx,
        sess_html,
        step3_ctx,
        action_view,
        requests_open,
        has_requests,
    ) = with_state_mut(|st| {
        ensure_valid_lobby_selection(st);
        let my_uid = st.user.as_ref().map(|u| u.user_id.as_str());
        let ctx = LobbyRenderContext {
            scope: st.lobby_scope,
            lobby_step: st.lobby_step,
            my_user_id: my_uid,
            my_campaigns: &st.my_campaigns,
            saved_sessions: &st.saved_sessions,
            directory_results: st.directory_results.as_ref(),
            selected_dir_user_id: st.selected_directory_user_id.as_deref(),
            selected_campaign: &st.selected_campaign_key,
            selected_session: &st.selected_session_key,
            my_join_requests: &st.my_join_requests,
            show_join_password_box: st.show_join_password_box,
        };
        let chips = st
            .directory_results
            .as_ref()
            .map(|r| format_directory_user_chips_html(r, st.selected_directory_user_id.as_deref()))
            .unwrap_or_default();
        let pending_in = st
            .incoming_join_requests
            .iter()
            .filter(|r| r.status == JoinRequestStatus::Pending)
            .count();
        let active_mine = st
            .my_join_requests
            .iter()
            .filter(|r| matches!(r.status, JoinRequestStatus::Pending | JoinRequestStatus::Approved))
            .count();
        (
            st.lobby_scope,
            st.lobby_step,
            matches!(st.selected_campaign_key, CampaignSelectionKey::NewCampaign),
            format_lobby_breadcrumb_html(st.lobby_step, &st.selected_campaign_key),
            chips,
            format_lobby_campaign_list_html(&ctx),
            step2_context_html(st),
            format_lobby_session_list_html(&ctx),
            step3_context_html(st),
            format_lobby_action_panel_html(&ctx),
            st.requests_tray_open,
            (pending_in + active_mine) > 0,
        )
    });

    set_html("lobby-step-breadcrumb", &breadcrumb_html);
    set_display(
        "btn-lobby-back",
        if step == LobbyStep::ChooseCampaign {
            "none"
        } else {
            "inline-flex"
        },
    );
    set_display(
        "btn-toggle-requests-tray",
        if has_requests || requests_open {
            "inline-flex"
        } else {
            "none"
        },
    );
    set_display(
        "session-requests-tray",
        if requests_open { "block" } else { "none" },
    );

    set_display(
        "lobby-step-1",
        if step == LobbyStep::ChooseCampaign {
            "flex"
        } else {
            "none"
        },
    );
    set_display(
        "lobby-step-2",
        if step == LobbyStep::ChooseSession {
            "flex"
        } else {
            "none"
        },
    );
    set_display(
        "lobby-step-3",
        if step == LobbyStep::TakeAction {
            "flex"
        } else {
            "none"
        },
    );

    set_class(
        "btn-scope-my-campaigns",
        if scope == LobbyScope::Mine {
            "lobby-scope-btn active"
        } else {
            "lobby-scope-btn"
        },
    );
    set_class(
        "btn-scope-search-campaigns",
        if scope == LobbyScope::Search {
            "lobby-scope-btn active"
        } else {
            "lobby-scope-btn"
        },
    );
    set_display(
        "campaign-search-bar",
        if scope == LobbyScope::Search {
            "flex"
        } else {
            "none"
        },
    );
    set_html("directory-user-chips", &user_chips_html);
    set_html("campaign-list-pane", &camp_html);

    set_html("lobby-step2-context", &step2_ctx);
    set_display(
        "session-list-pane",
        if is_new_campaign { "none" } else { "flex" },
    );
    set_display(
        "lobby-new-campaign-fields",
        if is_new_campaign { "flex" } else { "none" },
    );
    set_html("session-list-pane", &sess_html);

    set_html("lobby-step3-context", &step3_ctx);
    set_html("lobby-dynamic-action-body", &action_view.dynamic_html);
    set_display(
        "lobby-new-session-panel",
        if action_view.show_new_session_panel {
            "flex"
        } else {
            "none"
        },
    );
}

pub(crate) fn show_live_notification_toast(text: &str) {
    set_text("live-notification-toast-text", text);
    set_display("live-notification-toast", "flex");
}

/// Dismisses the live notification toast banner.
#[wasm_bindgen]
pub fn satya_dismiss_notification_toast() {
    set_display("live-notification-toast", "none");
}

/// Toggles the header notifications popover open/closed.
#[wasm_bindgen]
pub fn satya_toggle_notifications_popover() {
    let open = with_state_mut(|st| {
        st.notifications_popover_open = !st.notifications_popover_open;
        st.notifications_popover_open
    });
    set_display(
        "notifications-popover",
        if open { "flex" } else { "none" },
    );
}

/// Switches between `My Campaigns` (`"mine"`) and `Find Another User` (`"search"`) on Step 1.
#[wasm_bindgen]
pub fn satya_set_lobby_scope(scope: String) {
    with_state_mut(|st| {
        st.lobby_scope = if scope.trim().eq_ignore_ascii_case("search") {
            LobbyScope::Search
        } else {
            LobbyScope::Mine
        };
        st.lobby_step = LobbyStep::ChooseCampaign;
        st.show_join_password_box = false;
    });
    render_session_lobby_ui();
}

/// Selects a matched directory user from `#directory-user-chips`.
#[wasm_bindgen]
pub fn satya_select_directory_user(user_id: String) {
    with_state_mut(|st| {
        st.selected_directory_user_id = Some(user_id);
        st.selected_campaign_key = CampaignSelectionKey::None;
        st.selected_session_key = SessionSelectionKey::None;
        st.lobby_step = LobbyStep::ChooseCampaign;
        st.show_join_password_box = false;
    });
    render_session_lobby_ui();
}

/// Selects a campaign on Step 1 and advances to Step 2 (`ChooseSession`).
#[wasm_bindgen]
pub fn satya_select_lobby_campaign(kind: String, id: String) {
    with_state_mut(|st| {
        st.selected_campaign_key = match kind.as_str() {
            "new" => CampaignSelectionKey::NewCampaign,
            "owned" => CampaignSelectionKey::OwnedCampaign(id),
            "standalone" => CampaignSelectionKey::StandaloneOwned,
            "joined" => CampaignSelectionKey::JoinedOther,
            "other" => {
                let owner_uid = selected_directory_user(
                    st.directory_results.as_ref(),
                    st.selected_directory_user_id.as_deref(),
                )
                .map(|g| g.user_id.clone())
                .unwrap_or_default();
                CampaignSelectionKey::OtherCampaign {
                    owner_user_id: owner_uid,
                    campaign_id: if id.trim().is_empty() { None } else { Some(id) },
                }
            }
            _ => CampaignSelectionKey::None,
        };
        st.selected_session_key = SessionSelectionKey::None;
        st.lobby_step = LobbyStep::ChooseSession;
        st.show_join_password_box = false;
    });
    set_feedback("session-feedback", "", false);
    render_session_lobby_ui();
}

/// Selects a session on Step 2 and advances to Step 3 (`TakeAction`).
#[wasm_bindgen]
pub fn satya_select_lobby_session(kind: String, id: String) {
    with_state_mut(|st| {
        st.selected_session_key = match kind.as_str() {
            "new" => SessionSelectionKey::NewSession,
            "existing" if !id.trim().is_empty() => SessionSelectionKey::Existing(id),
            _ => SessionSelectionKey::None,
        };
        st.lobby_step = LobbyStep::TakeAction;
        st.show_join_password_box = false;
    });
    set_feedback("session-feedback", "", false);
    render_session_lobby_ui();
}

/// Navigates back one step in the guided Campaign & Session modal.
#[wasm_bindgen]
pub fn satya_lobby_go_back() {
    with_state_mut(|st| {
        st.show_join_password_box = false;
        st.lobby_step = match st.lobby_step {
            LobbyStep::TakeAction => {
                st.selected_session_key = SessionSelectionKey::None;
                LobbyStep::ChooseSession
            }
            LobbyStep::ChooseSession | LobbyStep::ChooseCampaign => {
                st.selected_campaign_key = CampaignSelectionKey::None;
                st.selected_session_key = SessionSelectionKey::None;
                LobbyStep::ChooseCampaign
            }
        };
    });
    set_feedback("session-feedback", "", false);
    render_session_lobby_ui();
}

/// Jumps back to step `1` (`ChooseCampaign`) or `2` (`ChooseSession`) from the breadcrumb.
#[wasm_bindgen]
pub fn satya_lobby_go_step(step: u8) {
    with_state_mut(|st| {
        st.show_join_password_box = false;
        match step {
            1 => {
                st.lobby_step = LobbyStep::ChooseCampaign;
                st.selected_campaign_key = CampaignSelectionKey::None;
                st.selected_session_key = SessionSelectionKey::None;
            }
            2 if st.lobby_step == LobbyStep::TakeAction => {
                st.lobby_step = LobbyStep::ChooseSession;
                st.selected_session_key = SessionSelectionKey::None;
            }
            _ => {}
        }
    });
    set_feedback("session-feedback", "", false);
    render_session_lobby_ui();
}

/// Reveals the optional invite-password input on Step 3 (`Row 2`).
#[wasm_bindgen]
pub fn satya_toggle_dir_password_join() {
    with_state_mut(|st| {
        st.show_join_password_box = !st.show_join_password_box;
    });
    render_session_lobby_ui();
}

/// Toggles the collapsible `#session-requests-tray` open or closed.
#[wasm_bindgen]
pub fn satya_toggle_requests_tray() {
    let open = with_state_mut(|st| {
        st.requests_tray_open = !st.requests_tray_open;
        st.requests_tray_open
    });
    set_display("session-requests-tray", if open { "block" } else { "none" });
}

/// Triggers directory search when pressing `Enter` in `#directory-gm-query`.
#[wasm_bindgen]
pub fn satya_directory_search_keydown(ev: KeyboardEvent) {
    if ev.key() == "Enter" {
        ev.prevent_default();
        satya_search_gm_directory();
    }
}

/// Creates a new campaign on Step 2 (`Row 5`), then either advances to Step 3 (`New Session`)
/// when `and_start_session` is `true`, or shows the empty campaign's session list.
#[wasm_bindgen]
pub fn satya_create_campaign_step(and_start_session: bool) {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            set_feedback("session-feedback", "Please log in first.", true);
            return;
        };
        let title = input_value("campaign-title-input").trim().to_string();
        if title.is_empty() {
            set_feedback("session-feedback", "Enter a campaign title first.", true);
            return;
        }
        let description = input_value("campaign-desc-input").trim().to_string();
        let is_discoverable = input_checked("campaign-discoverable-checkbox");
        let req = CreateCampaignRequest {
            title,
            slug: None,
            description,
            setting_pack: None,
            is_discoverable,
        };
        match fetch_json::<_, CampaignSummary>("POST", "/api/campaigns", Some(&tok), Some(&req))
            .await
        {
            Ok(camp) => {
                set_input_value("campaign-title-input", "");
                set_input_value("campaign-desc-input", "");
                let cid = camp.campaign_id.clone();
                let ctitle = camp.title.clone();
                with_state_mut(|st| {
                    st.my_campaigns.insert(0, camp);
                    st.selected_campaign_key = CampaignSelectionKey::OwnedCampaign(cid);
                    if and_start_session {
                        st.selected_session_key = SessionSelectionKey::NewSession;
                        st.lobby_step = LobbyStep::TakeAction;
                    } else {
                        st.selected_session_key = SessionSelectionKey::None;
                        st.lobby_step = LobbyStep::ChooseSession;
                    }
                });
                refresh_account_quotas();
                set_feedback(
                    "session-feedback",
                    &format!("Created campaign '{ctitle}'"),
                    false,
                );
                render_session_lobby_ui();
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Submits Step 3 `Create & Enter Session` inside the selected owned/standalone campaign (`Row 3`).
#[wasm_bindgen]
pub fn satya_submit_lobby_create() {
    spawn_local(async move {
        let sel_camp = with_state(|st| st.selected_campaign_key.clone());
        match sel_camp {
            CampaignSelectionKey::OwnedCampaign(cid) => {
                let slug = with_state(|st| {
                    st.my_campaigns
                        .iter()
                        .find(|c| c.campaign_id == cid)
                        .map(|c| c.slug.clone())
                });
                let _ = create_session_in_campaign(Some(cid), slug).await;
            }
            CampaignSelectionKey::StandaloneOwned => {
                let _ = create_session_in_campaign(None, None).await;
            }
            _ => {
                set_feedback(
                    "session-feedback",
                    "Select one of your campaigns first.",
                    true,
                );
            }
        }
    });
}

/// Resumes an existing session selected in the lobby (`Row 1` or `Row 2 Member`).
#[wasm_bindgen]
pub fn satya_resume_selected_session(session_id: String, solo_gm: bool) {
    resume_session_by_id(
        session_id,
        if solo_gm {
            Some(SessionRole::SoloGm)
        } else {
            None
        },
    );
}

/// Deletes an owned session from Step 3 (`Row 1`), steps back to Step 2, and reclaims quota.
#[wasm_bindgen]
pub fn satya_delete_selected_session(session_id: String) {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        let sid = session_id.trim().to_string();
        if sid.is_empty() {
            return;
        }
        match fetch_json::<(), serde_json::Value>(
            "DELETE",
            &format!("/api/sessions/{sid}"),
            Some(&tok),
            None,
        )
        .await
        {
            Ok(_) => {
                let deleted_active = with_state(|st| st.session_id.as_deref() == Some(sid.as_str()));
                if deleted_active {
                    crate::wasm_client::auth_session::cleanup_client_session_state();
                }
                with_state_mut(|st| {
                    st.selected_session_key = SessionSelectionKey::None;
                    st.lobby_step = LobbyStep::ChooseSession;
                });
                set_feedback("session-feedback", &format!("Deleted session '{sid}'"), false);
                satya_refresh_saved_sessions();
                satya_refresh_my_campaigns();
                refresh_account_quotas();
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Joins another user's discoverable session using an invite password (`Row 2` password path).
#[wasm_bindgen]
pub fn satya_join_directory_session_with_password(session_id: String) {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            set_feedback("session-feedback", "Please log in first.", true);
            return;
        };
        let password = input_value("dir-join-password-input");
        if password.is_empty() {
            set_feedback("session-feedback", "Enter the session password first.", true);
            return;
        }
        let role_raw = select_value("dir-join-role-select");
        let role = if role_raw.trim().is_empty() {
            SessionRole::Player
        } else {
            parse_session_role(&role_raw)
        };
        let display_raw = input_value("auth-display-name").trim().to_string();
        let display_name = if display_raw.is_empty() {
            None
        } else {
            Some(display_raw)
        };
        let req = JoinSessionRequest {
            session_id,
            session_password: password,
            approval_token: None,
            role,
            display_name,
        };
        match fetch_json::<_, SessionEnterResponse>(
            "POST",
            "/api/sessions/join",
            Some(&tok),
            Some(&req),
        )
        .await
        {
            Ok(data) => {
                set_input_value("dir-join-password-input", "");
                set_feedback(
                    "session-feedback",
                    &format!(
                        "Joined session '{}' as {:?}",
                        data.session_id, data.self_participant.role
                    ),
                    false,
                );
                enter_session(data);
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Submits a Row 4 request to create a new session inside another user's campaign
/// via `POST /api/campaigns/:campaign_id/session-requests`.
#[wasm_bindgen]
pub fn satya_request_campaign_session(campaign_id: String) {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            set_feedback("session-feedback", "Please log in first.", true);
            return;
        };
        let session_title = input_value("req-session-title-input").trim().to_string();
        if session_title.is_empty() {
            set_feedback(
                "session-feedback",
                "Enter a proposed session title first.",
                true,
            );
            return;
        }
        let mode_raw = select_value("req-session-mode-select");
        let session_mode = parse_session_mode(&mode_raw);
        let enable_ai_gm = mode_raw == "TTRPG_AI_GM";
        let role_raw = select_value("req-session-role-select");
        let requested_role = if role_raw.trim().is_empty() {
            SessionRole::Player
        } else {
            parse_session_role(&role_raw)
        };
        let note_raw = input_value("req-session-note-input").trim().to_string();
        let message = if note_raw.is_empty() {
            None
        } else {
            Some(note_raw)
        };
        let req = CreateCampaignSessionRequestBody {
            session_title,
            session_id: None,
            session_mode,
            enable_ai_gm,
            requested_role,
            message,
        };
        match fetch_json::<_, JoinRequestRecord>(
            "POST",
            &format!("/api/campaigns/{campaign_id}/session-requests"),
            Some(&tok),
            Some(&req),
        )
        .await
        {
            Ok(jr) => {
                set_input_value("req-session-title-input", "");
                set_input_value("req-session-note-input", "");
                let channel = if jr.emailed_offline {
                    "owner offline — email sent"
                } else {
                    "owner notified"
                };
                with_state_mut(|st| {
                    st.selected_session_key = SessionSelectionKey::None;
                    st.lobby_step = LobbyStep::ChooseSession;
                });
                set_feedback(
                    "session-feedback",
                    &format!(
                        "Requested new session '{}' ({channel})",
                        jr.session_title.as_deref().unwrap_or(&jr.session_id)
                    ),
                    false,
                );
                satya_refresh_join_requests();
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Derives a fallback `session_id` slug from optional campaign slug and session title.
#[must_use]
pub fn derive_session_id_slug(campaign_slug: Option<&str>, session_title: &str) -> String {
    let title_slug = slugify_identifier(session_title, "session");
    match campaign_slug.map(str::trim).filter(|s| !s.is_empty()) {
        Some(cslug) => format!("{}-{}", slugify_identifier(cslug, "camp"), title_slug),
        None => title_slug,
    }
}

fn sync_notifications_ui() {
    let (incoming, mine) =
        with_state(|st| (st.incoming_join_requests.clone(), st.my_join_requests.clone()));
    let pending_count = incoming
        .iter()
        .filter(|r| r.status == JoinRequestStatus::Pending)
        .count();
    let approved_count = mine
        .iter()
        .filter(|r| r.status == JoinRequestStatus::Approved)
        .count();
    let total_badge = pending_count + approved_count;
    let badge_cls = if total_badge > 0 {
        "pill pill-active"
    } else {
        "pill pill-muted"
    };
    set_text("notifications-badge", &total_badge.to_string());
    set_class("notifications-badge", badge_cls);
    set_text("requests-tray-badge", &total_badge.to_string());
    set_class("requests-tray-badge", badge_cls);
    set_html("gm-join-requests-list", &format_gm_join_requests_html(&incoming));
    set_html("my-join-requests-list", &format_my_join_requests_html(&mine));
    set_html(
        "notifications-popover-list",
        &format_notifications_popover_html(&incoming, &mine),
    );
    render_session_lobby_ui();
}

/// Refreshes the caller's quota usage pill (`#account-quota-pill`).
pub(crate) fn refresh_account_quotas() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        if let Ok(q) =
            fetch_json::<(), AccountQuotaUsage>("GET", "/api/auth/quotas", Some(&tok), None).await
        {
            set_text(
                "account-quota-pill",
                &format!(
                    "Quota: {}/{} camps · {}/{} sess",
                    q.campaigns_used, q.campaigns_limit, q.sessions_used, q.sessions_limit
                ),
            );
            with_state_mut(|st| st.quota_usage = Some(q));
        }
    });
}

/// Refreshes the authenticated user's owned campaigns (`GET /api/campaigns`) and re-renders the modal.
#[wasm_bindgen]
pub fn satya_refresh_my_campaigns() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        if let Ok(resp) =
            fetch_json::<(), CampaignListResponse>("GET", "/api/campaigns", Some(&tok), None).await
        {
            with_state_mut(|st| st.my_campaigns = resp.campaigns);
            render_session_lobby_ui();
        }
    });
}

/// Deletes an owned campaign (`DELETE /api/campaigns/:id`).
#[wasm_bindgen]
pub fn satya_delete_campaign(campaign_id: String) {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        match fetch_json::<(), serde_json::Value>(
            "DELETE",
            &format!("/api/campaigns/{campaign_id}"),
            Some(&tok),
            None,
        )
        .await
        {
            Ok(_) => {
                set_feedback("session-feedback", "Deleted campaign", false);
                satya_refresh_my_campaigns();
                refresh_account_quotas();
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Searches the user/campaign directory by email or `@username` via `GET /api/directory/search`.
#[wasm_bindgen]
pub fn satya_search_gm_directory() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            set_feedback(
                "session-feedback",
                "Please log in first to search users",
                true,
            );
            return;
        };
        let query = input_value("directory-gm-query").trim().to_string();
        if query.is_empty() {
            with_state_mut(|st| {
                st.directory_results = None;
                st.selected_directory_user_id = None;
                if st.lobby_scope == LobbyScope::Search {
                    st.selected_campaign_key = CampaignSelectionKey::None;
                    st.selected_session_key = SessionSelectionKey::None;
                    st.lobby_step = LobbyStep::ChooseCampaign;
                }
            });
            render_session_lobby_ui();
            return;
        }
        let encoded = js_sys::encode_uri_component(&query)
            .as_string()
            .unwrap_or(query);
        match fetch_json::<(), DirectorySearchResponse>(
            "GET",
            &format!("/api/directory/search?q={encoded}"),
            Some(&tok),
            None,
        )
        .await
        {
            Ok(resp) => {
                with_state_mut(|st| {
                    st.lobby_scope = LobbyScope::Search;
                    st.lobby_step = LobbyStep::ChooseCampaign;
                    st.selected_directory_user_id =
                        resp.gms.first().map(|g| g.user_id.clone());
                    st.directory_results = Some(resp);
                    st.selected_campaign_key = CampaignSelectionKey::None;
                    st.selected_session_key = SessionSelectionKey::None;
                    st.show_join_password_box = false;
                });
                render_session_lobby_ui();
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Submits a Request-to-Join for `session_id` via `POST /api/sessions/:id/join-requests`.
#[wasm_bindgen]
pub fn satya_request_join_session(session_id: String) {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        let role_raw = select_value("dir-join-role-select");
        let role = if role_raw.trim().is_empty() {
            SessionRole::Player
        } else {
            parse_session_role(&role_raw)
        };
        let requested_role = if matches!(
            role,
            SessionRole::Player | SessionRole::Participant | SessionRole::Observer
        ) {
            role
        } else {
            SessionRole::Player
        };
        let req = CreateJoinRequestBody {
            requested_role,
            message: None,
        };
        match fetch_json::<_, JoinRequestRecord>(
            "POST",
            &format!("/api/sessions/{session_id}/join-requests"),
            Some(&tok),
            Some(&req),
        )
        .await
        {
            Ok(jr) => {
                let channel = if jr.emailed_offline {
                    "owner offline — email notification sent"
                } else {
                    "owner notified"
                };
                set_feedback(
                    "session-feedback",
                    &format!("Requested to join '{}' ({channel})", jr.session_id),
                    false,
                );
                satya_refresh_join_requests();
                if with_state(|st| st.directory_results.is_some()) {
                    satya_search_gm_directory();
                }
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Enters an approved session directly using the user-bound `approval_token` (no password needed).
#[wasm_bindgen]
pub fn satya_join_approved_session(session_id: String, approval_token: String) {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        let display_raw = input_value("auth-display-name").trim().to_string();
        let display_name = if display_raw.is_empty() {
            None
        } else {
            Some(display_raw)
        };
        let req = JoinSessionRequest {
            session_id: session_id.clone(),
            session_password: String::new(),
            approval_token: Some(approval_token),
            role: SessionRole::Player,
            display_name,
        };
        match fetch_json::<_, SessionEnterResponse>(
            "POST",
            "/api/sessions/join",
            Some(&tok),
            Some(&req),
        )
        .await
        {
            Ok(data) => {
                set_feedback(
                    "session-feedback",
                    &format!("Joined approved session '{}'", data.session_id),
                    false,
                );
                enter_session(data);
                satya_refresh_join_requests();
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Cancels an outbound `PENDING` join request (`DELETE /api/join-requests/:id`).
#[wasm_bindgen]
pub fn satya_cancel_join_request(request_id: String) {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        if fetch_json::<(), serde_json::Value>(
            "DELETE",
            &format!("/api/join-requests/{request_id}"),
            Some(&tok),
            None,
        )
        .await
        .is_ok()
        {
            satya_refresh_join_requests();
            if with_state(|st| st.directory_results.is_some()) {
                satya_search_gm_directory();
            }
        }
    });
}

/// Approves or denies an incoming join or new-session request (`POST /api/join-requests/:id/decide`).
#[wasm_bindgen]
pub fn satya_decide_join_request(request_id: String, approve: bool) {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        let req = DecideJoinRequestBody {
            approve,
            decision_note: None,
        };
        match fetch_json::<_, JoinRequestRecord>(
            "POST",
            &format!("/api/join-requests/{request_id}/decide"),
            Some(&tok),
            Some(&req),
        )
        .await
        {
            Ok(updated) => {
                let verb = if approve { "Approved" } else { "Denied" };
                set_feedback(
                    "session-feedback",
                    &format!("{verb} @{} for '{}'", updated.requester_username, updated.session_id),
                    false,
                );
                satya_refresh_join_requests();
                if approve && updated.request_kind == JoinRequestKind::CreateSession {
                    satya_refresh_saved_sessions();
                    satya_refresh_my_campaigns();
                    refresh_account_quotas();
                }
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Refreshes both incoming (owner) and outbound (requester) join/session requests.
#[wasm_bindgen]
pub fn satya_refresh_join_requests() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        if let Ok(mine) = fetch_json::<(), JoinRequestsListResponse>(
            "GET",
            "/api/join-requests/mine",
            Some(&tok),
            None,
        )
        .await
        {
            with_state_mut(|st| st.my_join_requests = mine.requests);
        }
        if let Ok(incoming) = fetch_json::<(), JoinRequestsListResponse>(
            "GET",
            "/api/join-requests/incoming",
            Some(&tok),
            None,
        )
        .await
        {
            with_state_mut(|st| st.incoming_join_requests = incoming.requests);
        }
        sync_notifications_ui();
    });
}

#[derive(serde::Serialize)]
struct WsNotifTicketReq {
    kind: &'static str,
}

#[derive(serde::Deserialize)]
struct WsNotifTicketResp {
    ticket: String,
}

/// Connects the account-level `/ws/notifications` WebSocket for live alerts via a short-lived ticket.
pub(crate) fn connect_notifications_websocket() {
    let Some(tok) = with_state(|st| st.auth_token.clone()) else {
        return;
    };

    disconnect_notifications_websocket();

    spawn_local(async move {
        let Ok(ticket_resp) = fetch_json::<_, WsNotifTicketResp>(
            "POST",
            "/api/ws/ticket",
            Some(&tok),
            Some(&WsNotifTicketReq {
                kind: "notifications",
            }),
        )
        .await
        else {
            return;
        };

        let still_logged_in = with_state(|st| st.auth_token.as_deref() == Some(tok.as_str()));
        if !still_logged_in {
            return;
        }
        let Some(win) = window() else {
            return;
        };
        let loc = win.location();
        let proto = if loc.protocol().unwrap_or_default() == "https:" {
            "wss:"
        } else {
            "ws:"
        };
        let host = loc.host().unwrap_or_else(|_| "127.0.0.1:3000".to_string());
        let url = format!("{proto}//{host}/ws/notifications?ticket={}", ticket_resp.ticket);
        let Ok(ws) = WebSocket::new(&url) else {
            return;
        };

        let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |ev: MessageEvent| {
            if let Some(text) = ev.data().as_string()
                && let Ok(pkt) = serde_json::from_str::<AccountWsPacket>(&text)
            {
                handle_account_ws_packet(pkt);
            }
        });
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        with_state_mut(|st| {
            if st.auth_token.as_deref() == Some(tok.as_str()) {
                st.notifications_ws = Some(ws);
            } else {
                let _ = ws.close();
            }
        });
    });
}

/// Closes any open `/ws/notifications` socket.
pub(crate) fn disconnect_notifications_websocket() {
    with_state_mut(|st| {
        if let Some(ws) = st.notifications_ws.take() {
            ws.set_onmessage(None);
            ws.set_onclose(None);
            ws.set_onerror(None);
            let _ = ws.close();
        }
    });
}

fn handle_account_ws_packet(pkt: AccountWsPacket) {
    match pkt {
        AccountWsPacket::Connected {
            pending_incoming_count,
            ..
        }
        | AccountWsPacket::PendingCountSync {
            pending_incoming_count,
        } => {
            if pending_incoming_count > 0 {
                satya_refresh_join_requests();
            } else {
                sync_notifications_ui();
            }
        }
        AccountWsPacket::JoinRequestReceived { request } => {
            let label = request
                .session_title
                .as_deref()
                .unwrap_or(&request.session_id);
            let msg = match request.request_kind {
                JoinRequestKind::CreateSession => format!(
                    "New session request: @{} wants '{}' in {}",
                    request.requester_username,
                    label,
                    request.campaign_title.as_deref().unwrap_or("campaign")
                ),
                JoinRequestKind::JoinSession => format!(
                    "Join request: @{} wants to join '{}'",
                    request.requester_username, label
                ),
            };
            show_live_notification_toast(&msg);
            satya_refresh_join_requests();
        }
        AccountWsPacket::JoinRequestResolved { request } => {
            let status_txt = if request.status == JoinRequestStatus::Approved {
                "approved! Click Join Approved Session."
            } else {
                "denied."
            };
            let label = request
                .session_title
                .as_deref()
                .unwrap_or(&request.session_id);
            show_live_notification_toast(&format!("Your request for '{label}' was {status_txt}"));
            satya_refresh_join_requests();
            if with_state(|st| st.directory_results.is_some()) {
                satya_search_gm_directory();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_camp(id: &str, title: &str) -> CampaignSummary {
        CampaignSummary {
            campaign_id: id.to_string(),
            owner_user_id: "u-self".to_string(),
            slug: slugify_identifier(title, "camp"),
            title: title.to_string(),
            description: "Test".to_string(),
            setting_pack: "awakened".to_string(),
            is_discoverable: true,
            is_archived: false,
            session_count: 1,
            created_at_us: 1,
            updated_at_us: 1,
        }
    }

    fn sample_saved(sid: &str, owner: &str, camp_id: Option<&str>) -> SavedSessionSummary {
        SavedSessionSummary {
            session_id: sid.to_string(),
            session_mode: SessionMode::Ttrpg,
            role: if owner == "u-self" {
                SessionRole::Gm
            } else {
                SessionRole::Player
            },
            display_name: "Tester".to_string(),
            host_user_id: owner.to_string(),
            owner_user_id: owner.to_string(),
            campaign_id: camp_id.map(ToOwned::to_owned),
            campaign_title: camp_id.map(|_| "Satyang".to_string()),
            title: Some(format!("Session {sid}")),
            is_discoverable: true,
            campaign_tag: "satyang".to_string(),
            has_ai_gm: false,
            character_id: None,
            chat_event_count: 2,
            mechanics_event_count: 1,
            active_human_count: 0,
            created_at_us: 1,
            updated_at_us: 1,
        }
    }

    fn sample_dir(status: DirectoryMembershipStatus) -> DirectorySearchResponse {
        DirectorySearchResponse {
            query: "@gm_sean".to_string(),
            gms: vec![DirectoryGmEntry {
                user_id: "u-other".to_string(),
                username: "gm_sean".to_string(),
                display_name: "Sean".to_string(),
                is_online: true,
                campaigns: vec![DirectoryCampaignEntry {
                    campaign_id: Some("camp-other".to_string()),
                    slug: "daise".to_string(),
                    title: "DAISE".to_string(),
                    description: "Ops".to_string(),
                    setting_pack: "awakened".to_string(),
                    sessions: vec![DirectorySessionEntry {
                        session_id: "daise-ep1".to_string(),
                        title: Some("Cold Dock".to_string()),
                        session_mode: SessionMode::Ttrpg,
                        has_ai_gm: false,
                        is_live: true,
                        active_human_count: 2,
                        allow_join_requests: true,
                        caller_membership_status: status,
                        caller_request_id: (status == DirectoryMembershipStatus::PendingRequest)
                            .then(|| "jreq_1".to_string()),
                        approval_token: (status == DirectoryMembershipStatus::ApprovedReady)
                            .then(|| "aprv_1".to_string()),
                        updated_at_us: 1,
                    }],
                }],
            }],
        }
    }

    #[test]
    fn test_guided_three_step_matrix_and_progressive_disclosure() {
        let camps = [sample_camp("camp-self", "Satyang")];
        let saved = [sample_saved("satyang-ep1", "u-self", Some("camp-self"))];
        let dir = sample_dir(DirectoryMembershipStatus::None);

        // Step 1 starts clean and never leaks + Create New Campaign into Search scope (Row 6)
        let mine_step1 = format_lobby_campaign_list_html(&LobbyRenderContext {
            scope: LobbyScope::Mine,
            lobby_step: LobbyStep::ChooseCampaign,
            my_user_id: Some("u-self"),
            my_campaigns: &camps,
            saved_sessions: &saved,
            directory_results: Some(&dir),
            selected_dir_user_id: Some("u-other"),
            selected_campaign: &CampaignSelectionKey::None,
            selected_session: &SessionSelectionKey::None,
            my_join_requests: &[],
            show_join_password_box: false,
        });
        let search_step1 = format_lobby_campaign_list_html(&LobbyRenderContext {
            scope: LobbyScope::Search,
            lobby_step: LobbyStep::ChooseCampaign,
            my_user_id: Some("u-self"),
            my_campaigns: &camps,
            saved_sessions: &saved,
            directory_results: Some(&dir),
            selected_dir_user_id: Some("u-other"),
            selected_campaign: &CampaignSelectionKey::None,
            selected_session: &SessionSelectionKey::None,
            my_join_requests: &[],
            show_join_password_box: false,
        });
        assert!(mine_step1.contains("+ Create New Campaign"));
        assert!(!search_step1.contains("+ Create New Campaign"));
        assert!(!search_step1.contains("satyaSelectLobbyCampaign('new'"));

        // Step 3 Row 1: Existing | Self | Existing -> Join / Resume Session
        let row1 = format_lobby_action_panel_html(&LobbyRenderContext {
            scope: LobbyScope::Mine,
            lobby_step: LobbyStep::TakeAction,
            my_user_id: Some("u-self"),
            my_campaigns: &camps,
            saved_sessions: &saved,
            directory_results: None,
            selected_dir_user_id: None,
            selected_campaign: &CampaignSelectionKey::OwnedCampaign("camp-self".to_string()),
            selected_session: &SessionSelectionKey::Existing("satyang-ep1".to_string()),
            my_join_requests: &[],
            show_join_password_box: false,
        });
        assert!(!row1.show_new_session_panel);
        assert!(row1.dynamic_html.contains("Join / Resume Session"));

        // Step 3 Row 2: Existing | Other | Existing -> password hidden until toggled
        let row2_collapsed = format_lobby_action_panel_html(&LobbyRenderContext {
            scope: LobbyScope::Search,
            lobby_step: LobbyStep::TakeAction,
            my_user_id: Some("u-self"),
            my_campaigns: &camps,
            saved_sessions: &saved,
            directory_results: Some(&dir),
            selected_dir_user_id: Some("u-other"),
            selected_campaign: &CampaignSelectionKey::OtherCampaign {
                owner_user_id: "u-other".to_string(),
                campaign_id: Some("camp-other".to_string()),
            },
            selected_session: &SessionSelectionKey::Existing("daise-ep1".to_string()),
            my_join_requests: &[],
            show_join_password_box: false,
        });
        assert!(row2_collapsed.dynamic_html.contains("Request to Join"));
        assert!(row2_collapsed.dynamic_html.contains("satyaToggleDirPasswordJoin"));
        assert!(!row2_collapsed.dynamic_html.contains("dir-join-password-input"));

        let row2_expanded = format_lobby_action_panel_html(&LobbyRenderContext {
            scope: LobbyScope::Search,
            lobby_step: LobbyStep::TakeAction,
            my_user_id: Some("u-self"),
            my_campaigns: &camps,
            saved_sessions: &saved,
            directory_results: Some(&dir),
            selected_dir_user_id: Some("u-other"),
            selected_campaign: &CampaignSelectionKey::OtherCampaign {
                owner_user_id: "u-other".to_string(),
                campaign_id: Some("camp-other".to_string()),
            },
            selected_session: &SessionSelectionKey::Existing("daise-ep1".to_string()),
            my_join_requests: &[],
            show_join_password_box: true,
        });
        assert!(row2_expanded.dynamic_html.contains("Join with Password"));
        assert!(row2_expanded.dynamic_html.contains("dir-join-password-input"));

        // Step 3 Row 3: Existing | Self | New -> SSR new-session form
        let row3 = format_lobby_action_panel_html(&LobbyRenderContext {
            scope: LobbyScope::Mine,
            lobby_step: LobbyStep::TakeAction,
            my_user_id: Some("u-self"),
            my_campaigns: &camps,
            saved_sessions: &saved,
            directory_results: None,
            selected_dir_user_id: None,
            selected_campaign: &CampaignSelectionKey::OwnedCampaign("camp-self".to_string()),
            selected_session: &SessionSelectionKey::NewSession,
            my_join_requests: &[],
            show_join_password_box: false,
        });
        assert!(row3.show_new_session_panel);

        // Step 3 Row 4: Existing | Other | New -> Send New Session Request
        let row4 = format_lobby_action_panel_html(&LobbyRenderContext {
            scope: LobbyScope::Search,
            lobby_step: LobbyStep::TakeAction,
            my_user_id: Some("u-self"),
            my_campaigns: &camps,
            saved_sessions: &saved,
            directory_results: Some(&dir),
            selected_dir_user_id: Some("u-other"),
            selected_campaign: &CampaignSelectionKey::OtherCampaign {
                owner_user_id: "u-other".to_string(),
                campaign_id: Some("camp-other".to_string()),
            },
            selected_session: &SessionSelectionKey::NewSession,
            my_join_requests: &[],
            show_join_password_box: false,
        });
        assert!(!row4.show_new_session_panel);
        assert!(row4.dynamic_html.contains("Send New Session Request"));
        assert!(row4.dynamic_html.contains("satyaRequestCampaignSession('camp-other')"));

        // Step validation steps back gracefully if selected session disappears
        let mut state = SatyaWasmState {
            my_campaigns: camps.to_vec(),
            saved_sessions: saved.to_vec(),
            lobby_scope: LobbyScope::Mine,
            lobby_step: LobbyStep::TakeAction,
            selected_campaign_key: CampaignSelectionKey::OwnedCampaign("camp-self".to_string()),
            selected_session_key: SessionSelectionKey::Existing("deleted-ep".to_string()),
            ..Default::default()
        };
        ensure_valid_lobby_selection(&mut state);
        assert_eq!(state.lobby_step, LobbyStep::ChooseSession);
        assert_eq!(state.selected_session_key, SessionSelectionKey::None);
    }
}
