//! Client UI formatters and Wasm handlers for GM-owned Campaigns,
//! Privacy-Preserving GM Directory Search, Request-to-Join, Quotas, and `/ws/notifications`.

use crate::wasm_client::auth_session::enter_session;
use crate::wasm_client::dom::{
    escape_html, escape_js_single_quoted_attr, fetch_json, input_checked, input_value,
    select_value, set_class, set_display, set_feedback, set_html, set_input_value, set_text,
    window,
};
use crate::wasm_client::options::parse_session_role;
use crate::wasm_client::session_persistence::satya_refresh_saved_sessions;
use crate::wasm_client::state::{with_state, with_state_mut};
use satya_schema::{
    AccountQuotaUsage, AccountWsPacket, CampaignListResponse, CampaignSummary,
    CreateCampaignRequest, CreateJoinRequestBody, DecideJoinRequestBody,
    DirectoryMembershipStatus, DirectorySearchResponse, JoinRequestRecord, JoinRequestStatus,
    JoinRequestsListResponse, JoinSessionRequest, SessionEnterResponse, SessionRole,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{MessageEvent, WebSocket};

/// Formats `<option>` elements for `#session-campaign-select`.
#[must_use]
pub fn format_campaign_select_options_html(
    campaigns: &[CampaignSummary],
    selected_id: Option<&str>,
) -> String {
    let mut out = String::from(r#"<option value="">Standalone Session (No Campaign)</option>"#);
    for c in campaigns {
        if c.is_archived {
            continue;
        }
        let sel = if selected_id == Some(c.campaign_id.as_str()) {
            " selected"
        } else {
            ""
        };
        out.push_str(&format!(
            r#"<option value="{cid}"{sel}>{title} ({count} sessions)</option>"#,
            cid = escape_html(&c.campaign_id),
            title = escape_html(&c.title),
            count = c.session_count,
        ));
    }
    out
}

/// Formats the GM's owned campaign cards for `#gm-campaigns-list`.
#[must_use]
pub fn format_gm_campaigns_html(campaigns: &[CampaignSummary]) -> String {
    if campaigns.is_empty() {
        return r#"<div class="subtitle">No campaigns created yet. Create one above to group sessions.</div>"#
            .to_string();
    }
    let mut out = String::with_capacity(campaigns.len() * 260);
    for c in campaigns {
        let vis_badge = if c.is_discoverable {
            "Discoverable"
        } else {
            "Private"
        };
        let cid_js = escape_js_single_quoted_attr(&c.campaign_id);
        out.push_str(&format!(
            r#"<div class="dir-campaign-card">
                <div class="dir-campaign-head">
                    <strong>{title}</strong>
                    <span class="pill pill-muted">{vis_badge} · {count} sessions</span>
                    <button type="button" class="btn btn-sm btn-danger" onclick="window.satyaDeleteCampaign('{cid_js}')">Delete</button>
                </div>
                <div class="subtitle">{desc}</div>
            </div>"#,
            title = escape_html(&c.title),
            count = c.session_count,
            desc = if c.description.is_empty() {
                escape_html(&c.slug)
            } else {
                escape_html(&c.description)
            },
        ));
    }
    out
}

/// Formats search results returned by `GET /api/directory/search` for `#directory-results-list`.
#[must_use]
pub fn format_directory_results_html(resp: &DirectorySearchResponse) -> String {
    if resp.gms.is_empty() {
        if resp.query.trim().is_empty() {
            return String::new();
        }
        return format!(
            r#"<div class="subtitle">No GM found matching "{}".</div>"#,
            escape_html(&resp.query)
        );
    }

    let mut out = String::with_capacity(resp.gms.len() * 512);
    for gm in &resp.gms {
        let online_pill = if gm.is_online {
            r#"<span class="pill pill-active">GM Online</span>"#
        } else {
            r#"<span class="pill pill-muted">GM Offline</span>"#
        };
        out.push_str(&format!(
            r#"<div class="dir-gm-card">
                <div class="dir-gm-header">
                    <strong>{dname}</strong> <span class="mono">@{uname}</span> {online_pill}
                </div>"#,
            dname = escape_html(&gm.display_name),
            uname = escape_html(&gm.username),
        ));

        if gm.campaigns.is_empty() {
            out.push_str(r#"<div class="subtitle">No discoverable campaigns or sessions yet.</div>"#);
        } else {
            for camp in &gm.campaigns {
                out.push_str(&format!(
                    r#"<div class="dir-campaign-card">
                        <div class="dir-campaign-head"><strong>{ctitle}</strong> <span class="subtitle">{cdesc}</span></div>"#,
                    ctitle = escape_html(&camp.title),
                    cdesc = escape_html(&camp.description),
                ));
                if camp.sessions.is_empty() {
                    out.push_str(r#"<div class="subtitle">No sessions in this campaign yet.</div>"#);
                } else {
                    for s in &camp.sessions {
                        let sid_html = escape_html(&s.session_id);
                        let sid_js = escape_js_single_quoted_attr(&s.session_id);
                        let stitle = s.title.as_deref().unwrap_or(&s.session_id);
                        let live_badge = if s.is_live {
                            format!(
                                r#"<span class="pill pill-active">LIVE ({})</span>"#,
                                s.active_human_count
                            )
                        } else {
                            r#"<span class="pill pill-muted">Offline</span>"#.to_string()
                        };

                        let action_html = match s.caller_membership_status {
                            DirectoryMembershipStatus::Owner | DirectoryMembershipStatus::Member => {
                                r#"<span class="pill pill-active">Member — Use Saved List</span>"#
                                    .to_string()
                            }
                            DirectoryMembershipStatus::PendingRequest => {
                                if let Some(ref rid) = s.caller_request_id {
                                    let rid_js = escape_js_single_quoted_attr(rid);
                                    format!(
                                        r#"<span class="pill pill-muted">Request Pending</span> <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCancelJoinRequest('{rid_js}')">Cancel</button>"#
                                    )
                                } else {
                                    r#"<span class="pill pill-muted">Request Pending</span>"#
                                        .to_string()
                                }
                            }
                            DirectoryMembershipStatus::ApprovedReady => {
                                if let Some(ref tok) = s.approval_token {
                                    let tok_js = escape_js_single_quoted_attr(tok);
                                    format!(
                                        r#"<button type="button" class="btn btn-sm btn-accent" onclick="window.satyaJoinApprovedSession('{sid_js}', '{tok_js}')">Join Approved Session</button>"#
                                    )
                                } else {
                                    r#"<span class="pill pill-active">Approved</span>"#.to_string()
                                }
                            }
                            DirectoryMembershipStatus::Denied => {
                                r#"<span class="pill pill-muted">Denied</span>"#.to_string()
                            }
                            DirectoryMembershipStatus::None => {
                                if s.allow_join_requests {
                                    format!(
                                        r#"<button type="button" class="btn btn-sm btn-primary" onclick="window.satyaRequestJoinSession('{sid_js}')">Request to Join</button>"#
                                    )
                                } else {
                                    r#"<span class="pill pill-muted">Password Only</span>"#
                                        .to_string()
                                }
                            }
                        };

                        out.push_str(&format!(
                            r#"<div class="dir-session-row">
                                <div><strong>{title}</strong> <span class="mono">({sid_html})</span> {live_badge}</div>
                                <div class="dir-session-actions">{action_html}</div>
                            </div>"#,
                            title = escape_html(stitle),
                        ));
                    }
                }
                out.push_str("</div>");
            }
        }
        out.push_str("</div>");
    }
    out
}

/// Formats the GM's incoming join requests for `#gm-join-requests-list`.
#[must_use]
pub fn format_gm_join_requests_html(requests: &[JoinRequestRecord]) -> String {
    let pending: Vec<&JoinRequestRecord> = requests
        .iter()
        .filter(|r| r.status == JoinRequestStatus::Pending)
        .collect();
    if pending.is_empty() {
        return r#"<div class="subtitle">No pending incoming join requests.</div>"#.to_string();
    }
    let mut out = String::with_capacity(pending.len() * 280);
    for r in pending {
        let rid_js = escape_js_single_quoted_attr(&r.request_id);
        let msg_part = r
            .message
            .as_deref()
            .filter(|m| !m.is_empty())
            .map(|m| format!(r#" — "{}""#, escape_html(m)))
            .unwrap_or_default();
        out.push_str(&format!(
            r#"<div class="dir-session-row">
                <div>
                    <strong>{player}</strong> (<span class="mono">@{uname}</span>) wants to join <strong>{sid}</strong> as <span class="mono">{role:?}</span>{msg_part}
                </div>
                <div class="dir-session-actions">
                    <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaDecideJoinRequest('{rid_js}', true)">Approve</button>
                    <button type="button" class="btn btn-sm btn-danger" onclick="window.satyaDecideJoinRequest('{rid_js}', false)">Deny</button>
                </div>
            </div>"#,
            player = escape_html(&r.requester_display_name),
            uname = escape_html(&r.requester_username),
            sid = escape_html(&r.session_id),
            role = r.requested_role,
        ));
    }
    out
}

/// Formats the player's outbound join requests for `#my-join-requests-list`.
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
    let mut out = String::from(r#"<div class="menu-section-title">Your Join Requests</div>"#);
    for r in active {
        let sid_js = escape_js_single_quoted_attr(&r.session_id);
        let rid_js = escape_js_single_quoted_attr(&r.request_id);
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
                <div><strong>{sid}</strong> (GM: @{gm})</div>
                <div class="dir-session-actions">{action}</div>
            </div>"#,
            sid = escape_html(&r.session_id),
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
        html.push_str(&format!(
            r#"<div class="notif-item">
                <div><strong>@{uname}</strong> requested <strong>{sid}</strong></div>
                <div class="dir-session-actions">
                    <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaDecideJoinRequest('{rid_js}', true)">Approve</button>
                    <button type="button" class="btn btn-sm btn-danger" onclick="window.satyaDecideJoinRequest('{rid_js}', false)">Deny</button>
                </div>
            </div>"#,
            uname = escape_html(&r.requester_username),
            sid = escape_html(&r.session_id),
        ));
    }
    for r in ready_mine {
        let sid_js = escape_js_single_quoted_attr(&r.session_id);
        let tok_js = escape_js_single_quoted_attr(r.approval_token.as_deref().unwrap_or_default());
        html.push_str(&format!(
            r#"<div class="notif-item">
                <div>Approved for <strong>{sid}</strong>!</div>
                <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaJoinApprovedSession('{sid_js}', '{tok_js}')">Join Now</button>
            </div>"#,
            sid = escape_html(&r.session_id),
        ));
    }
    html
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
    set_text("notifications-badge", &total_badge.to_string());
    set_class(
        "notifications-badge",
        if total_badge > 0 {
            "pill pill-active"
        } else {
            "pill pill-muted"
        },
    );
    set_html("gm-join-requests-list", &format_gm_join_requests_html(&incoming));
    set_html("my-join-requests-list", &format_my_join_requests_html(&mine));
    set_html(
        "notifications-popover-list",
        &format_notifications_popover_html(&incoming, &mine),
    );
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

/// Refreshes the GM's owned campaigns list (`#gm-campaigns-list` + `#session-campaign-select`).
#[wasm_bindgen]
pub fn satya_refresh_my_campaigns() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        if let Ok(resp) =
            fetch_json::<(), CampaignListResponse>("GET", "/api/campaigns", Some(&tok), None).await
        {
            let cur_sel = select_value("session-campaign-select");
            set_html("gm-campaigns-list", &format_gm_campaigns_html(&resp.campaigns));
            set_html(
                "session-campaign-select",
                &format_campaign_select_options_html(&resp.campaigns, Some(cur_sel.trim())),
            );
            with_state_mut(|st| st.my_campaigns = resp.campaigns);
        }
    });
}

/// Creates a new GM-owned campaign via `POST /api/campaigns`.
#[wasm_bindgen]
pub fn satya_create_campaign() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        let title = input_value("campaign-title-input").trim().to_string();
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
            Ok(c) => {
                set_input_value("campaign-title-input", "");
                set_input_value("campaign-desc-input", "");
                set_feedback(
                    "session-feedback",
                    &format!("Created campaign '{}'", c.title),
                    false,
                );
                satya_refresh_my_campaigns();
                refresh_account_quotas();
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Deletes a GM-owned campaign (`DELETE /api/campaigns/:id`).
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

/// Deletes the currently selected saved session (`DELETE /api/sessions/:id`) to reclaim quota.
#[wasm_bindgen]
pub fn satya_delete_saved_session() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        let sid = select_value("saved-sessions-select").trim().to_string();
        let sid = if sid.is_empty() {
            input_value("session-id-input").trim().to_string()
        } else {
            sid
        };
        if sid.is_empty() {
            set_feedback("session-feedback", "Select a saved session to delete", true);
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
                set_feedback("session-feedback", &format!("Deleted session '{sid}'"), false);
                satya_refresh_saved_sessions();
                satya_refresh_my_campaigns();
                refresh_account_quotas();
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Searches the GM directory by email or `@username` via `GET /api/directory/search`.
#[wasm_bindgen]
pub fn satya_search_gm_directory() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            set_feedback("session-feedback", "Please log in first to search GMs", true);
            return;
        };
        let query = input_value("directory-gm-query").trim().to_string();
        if query.is_empty() {
            set_html("directory-results-list", "");
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
                set_html("directory-results-list", &format_directory_results_html(&resp));
                with_state_mut(|st| st.directory_results = Some(resp));
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
        let role = parse_session_role(&select_value("session-role-select"));
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
                    "GM offline — email notification sent"
                } else {
                    "GM notified"
                };
                set_feedback(
                    "session-feedback",
                    &format!("Requested to join '{}' ({channel})", jr.session_id),
                    false,
                );
                satya_refresh_join_requests();
                satya_search_gm_directory();
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
            satya_search_gm_directory();
        }
    });
}

/// Approves or denies an incoming join request as GM (`POST /api/join-requests/:id/decide`).
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
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Refreshes both incoming (GM) and outbound (Player) join requests.
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

/// Connects the account-level `/ws/notifications` WebSocket for live GM & player join alerts.
pub(crate) fn connect_notifications_websocket() {
    let Some(win) = window() else {
        return;
    };
    let Some(tok) = with_state(|st| st.auth_token.clone()) else {
        return;
    };
    let loc = win.location();
    let proto = if loc.protocol().unwrap_or_default() == "https:" {
        "wss:"
    } else {
        "ws:"
    };
    let host = loc.host().unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    disconnect_notifications_websocket();

    let url = format!("{proto}//{host}/ws/notifications?auth_token={tok}");
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
        st.notifications_ws = Some(ws);
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
            show_live_notification_toast(&format!(
                "Join request: @{} wants to join '{}'",
                request.requester_username, request.session_id
            ));
            satya_refresh_join_requests();
        }
        AccountWsPacket::JoinRequestResolved { request } => {
            let status_txt = if request.status == JoinRequestStatus::Approved {
                "approved! Click Join Approved Session."
            } else {
                "denied."
            };
            show_live_notification_toast(&format!(
                "Your request for '{}' was {status_txt}",
                request.session_id
            ));
            satya_refresh_join_requests();
            if with_state(|st| st.directory_results.is_some()) {
                satya_search_gm_directory();
            }
        }
    }
}
