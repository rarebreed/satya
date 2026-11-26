//! Campaign ownership, GM directory discovery, passwordless join-request grants,
//! account quotas, and account-level `/ws/notifications` DTOs.

use crate::types::{SessionMode, SessionRole};
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_requested_player_role() -> SessionRole {
    SessionRole::Player
}

fn default_ttrpg_mode() -> SessionMode {
    SessionMode::Ttrpg
}

crate::define_wire_enum! {
    /// Distinguishes joining an existing session vs. requesting creation of a new session in another user's campaign.
    #[derive(Default)]
    pub enum JoinRequestKind [count = 2] {
        /// Request to join an existing session.
        #[default]
        JoinSession => ("JOIN_SESSION", "Join Session"),
        /// Request for the campaign owner to create a new session in their campaign.
        CreateSession => ("CREATE_SESSION", "New Session"),
    }
}

crate::define_wire_enum! {
    /// Lifecycle status of a player's request to join a GM-owned session.
    #[derive(Default)]
    pub enum JoinRequestStatus [count = 6] {
        /// Request submitted and awaiting GM approval or denial.
        #[default]
        Pending => ("PENDING", "Pending"),
        /// Approved by the GM; a single-use, user-bound `approval_token` is ready for entry.
        Approved => ("APPROVED", "Approved"),
        /// Denied by the GM.
        Denied => ("DENIED", "Denied"),
        /// Cancelled by the requesting player prior to decision.
        Cancelled => ("CANCELLED", "Cancelled"),
        /// Request or approval grant expired past its TTL.
        Expired => ("EXPIRED", "Expired"),
        /// Approval grant was consumed to join the session.
        Consumed => ("CONSUMED", "Consumed"),
    }
}

crate::define_wire_enum! {
    /// Caller's membership or join-request status relative to a directory session entry.
    #[derive(Default)]
    pub enum DirectoryMembershipStatus [count = 6] {
        /// Caller has no prior membership or active request for this session.
        #[default]
        None => ("NONE", "Not Joined"),
        /// Caller owns/hosts this session.
        Owner => ("OWNER", "Owner"),
        /// Caller is already a registered participant of this session and can resume directly.
        Member => ("MEMBER", "Member"),
        /// Caller has an open `PENDING` join request for this session.
        PendingRequest => ("PENDING_REQUEST", "Request Pending"),
        /// Caller has an `APPROVED` join grant ready to enter without a password.
        ApprovedReady => ("APPROVED_READY", "Approved (Ready to Join)"),
        /// Caller's most recent join request for this session was denied.
        Denied => ("DENIED", "Denied"),
    }
}

/// Persistent GM-owned campaign grouping one or more Satya sessions and persistent vault data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignSummary {
    /// Unique campaign identifier (`camp_<uuid>`).
    pub campaign_id: String,
    /// Persistent `user_id` of the owning GM.
    pub owner_user_id: String,
    /// Normalized unique-per-GM slug (also used as `campaign_tag` for persistent vault sharing).
    pub slug: String,
    /// Human-readable campaign title (e.g. `"Satyang"` or `"DAISE"`).
    pub title: String,
    /// Optional campaign summary/blurb shown in the directory.
    #[serde(default)]
    pub description: String,
    /// Campaign setting pack identifier (e.g. `"awakened"`).
    pub setting_pack: String,
    /// Whether this campaign appears in GM directory search results.
    #[serde(default = "default_true")]
    pub is_discoverable: bool,
    /// Whether this campaign has been archived by its owner.
    #[serde(default)]
    pub is_archived: bool,
    /// Number of non-deleted sessions belonging to this campaign.
    #[serde(default)]
    pub session_count: usize,
    /// Creation timestamp in UTC microseconds since Unix epoch.
    pub created_at_us: i64,
    /// Last modification timestamp in UTC microseconds since Unix epoch.
    pub updated_at_us: i64,
}

/// HTTP request body for `POST /api/campaigns`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateCampaignRequest {
    /// Human-readable campaign title (e.g. `"Satyang Operations"`).
    pub title: String,
    /// Optional URL-safe slug override (derived from `title` when omitted).
    #[serde(default)]
    pub slug: Option<String>,
    /// Optional campaign description shown to searching players.
    #[serde(default)]
    pub description: String,
    /// Optional setting pack identifier (defaults to server campaign pack).
    #[serde(default)]
    pub setting_pack: Option<String>,
    /// Whether this campaign is discoverable in directory search (defaults to `true`).
    #[serde(default = "default_true")]
    pub is_discoverable: bool,
}

/// HTTP request body for `PATCH /api/campaigns/:campaign_id`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCampaignRequest {
    /// Optional updated campaign title.
    #[serde(default)]
    pub title: Option<String>,
    /// Optional updated campaign description.
    #[serde(default)]
    pub description: Option<String>,
    /// Optional discoverability toggle.
    #[serde(default)]
    pub is_discoverable: Option<bool>,
    /// Optional archive toggle.
    #[serde(default)]
    pub is_archived: Option<bool>,
}

/// HTTP response body for `GET /api/campaigns`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignListResponse {
    /// Campaigns owned by the authenticated caller.
    pub campaigns: Vec<CampaignSummary>,
}

/// Public/discoverable summary of a session within a GM's directory listing (never exposes passwords).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectorySessionEntry {
    /// Unique identifier of the session room.
    pub session_id: String,
    /// Optional human-friendly session title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Operating mode (`TTRPG` or `MEETING`).
    pub session_mode: SessionMode,
    /// Whether an AI Game Master is enabled.
    #[serde(default)]
    pub has_ai_gm: bool,
    /// Whether this session currently has at least one connected human participant.
    #[serde(default)]
    pub is_live: bool,
    /// Number of currently connected human participants.
    #[serde(default)]
    pub active_human_count: usize,
    /// Whether players may submit join requests for this session.
    #[serde(default = "default_true")]
    pub allow_join_requests: bool,
    /// Caller's membership/request state for this session.
    #[serde(default)]
    pub caller_membership_status: DirectoryMembershipStatus,
    /// Caller's active request ID (`PENDING` or `APPROVED`), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller_request_id: Option<String>,
    /// Single-use, user-bound approval token populated ONLY when `caller_membership_status == ApprovedReady`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_token: Option<String>,
    /// Last activity timestamp in UTC microseconds since Unix epoch.
    pub updated_at_us: i64,
}

/// Campaign grouping inside a GM directory search result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryCampaignEntry {
    /// Campaign ID (`None` for the synthetic uncategorized bucket when legacy sessions have no campaign).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,
    /// Campaign slug.
    #[serde(default)]
    pub slug: String,
    /// Human-readable campaign title.
    pub title: String,
    /// Campaign summary/blurb.
    #[serde(default)]
    pub description: String,
    /// Setting pack identifier.
    #[serde(default)]
    pub setting_pack: String,
    /// Discoverable sessions in this campaign.
    #[serde(default)]
    pub sessions: Vec<DirectorySessionEntry>,
}

/// Privacy-preserving GM profile and campaigns returned by `GET /api/directory/search` (never exposes GM email).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryGmEntry {
    /// Persistent `user_id` of the owning GM.
    pub user_id: String,
    /// Unique `@username` handle of the owning GM.
    pub username: String,
    /// Display name of the owning GM.
    pub display_name: String,
    /// Whether the GM is currently online (via `/ws/notifications` or an active `/ws/session`).
    #[serde(default)]
    pub is_online: bool,
    /// Discoverable campaigns (and their discoverable sessions) owned by this GM.
    #[serde(default)]
    pub campaigns: Vec<DirectoryCampaignEntry>,
}

/// HTTP response body for `GET /api/directory/search?q=...`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectorySearchResponse {
    /// Normalized search query echoed back (without exposing unrequested emails).
    pub query: String,
    /// Matched GM directory entries.
    pub gms: Vec<DirectoryGmEntry>,
}

/// HTTP request body for `POST /api/sessions/:session_id/join-requests`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateJoinRequestBody {
    /// Role requested by the player (`PLAYER`, `PARTICIPANT`, or `OBSERVER`).
    pub requested_role: SessionRole,
    /// Optional short message to the GM.
    #[serde(default)]
    pub message: Option<String>,
}

/// HTTP request body for `POST /api/campaigns/:campaign_id/session-requests`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateCampaignSessionRequestBody {
    /// Proposed human-readable session title.
    pub session_title: String,
    /// Optional preferred session room ID slug (derived from campaign + title when omitted).
    #[serde(default)]
    pub session_id: Option<String>,
    /// Operating mode (`TTRPG` or `MEETING`).
    #[serde(default = "default_ttrpg_mode")]
    pub session_mode: SessionMode,
    /// Whether the requested session should enable an autonomous AI GM.
    #[serde(default)]
    pub enable_ai_gm: bool,
    /// Role requested by the requester (`PLAYER`, `PARTICIPANT`, or `OBSERVER`).
    #[serde(default = "default_requested_player_role")]
    pub requested_role: SessionRole,
    /// Optional short note to the campaign owner.
    #[serde(default)]
    pub message: Option<String>,
}

/// Persistent join-request or new-session request record visible to the requester and campaign owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinRequestRecord {
    /// Unique join request identifier (`jreq_<uuid>`).
    pub request_id: String,
    /// Distinguishes joining an existing session vs. creating a new session in a campaign.
    #[serde(default)]
    pub request_kind: JoinRequestKind,
    /// Target or proposed session ID.
    pub session_id: String,
    /// Optional target or proposed session title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_title: Option<String>,
    /// Proposed operating mode when `request_kind == CREATE_SESSION`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_session_mode: Option<SessionMode>,
    /// Whether AI GM is requested when `request_kind == CREATE_SESSION`.
    #[serde(default)]
    pub proposed_enable_ai_gm: bool,
    /// Optional parent campaign ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,
    /// Optional parent campaign title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_title: Option<String>,
    /// Owning GM's `user_id`.
    pub owner_user_id: String,
    /// Owning GM's `@username`.
    pub owner_username: String,
    /// Owning GM's display name.
    pub owner_display_name: String,
    /// Requesting player's `user_id`.
    pub requester_user_id: String,
    /// Requesting player's `@username`.
    pub requester_username: String,
    /// Requesting player's display name.
    pub requester_display_name: String,
    /// Requested session role (`PLAYER`, `PARTICIPANT`, or `OBSERVER`).
    pub requested_role: SessionRole,
    /// Optional player note to the GM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Current lifecycle status (`PENDING`, `APPROVED`, `DENIED`, `CANCELLED`, `EXPIRED`, `CONSUMED`).
    pub status: JoinRequestStatus,
    /// User-bound one-time approval token (populated ONLY for the requesting user when `status == Approved`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_token: Option<String>,
    /// Optional GM decision note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_note: Option<String>,
    /// Whether an offline email notification was dispatched to the GM.
    #[serde(default)]
    pub emailed_offline: bool,
    /// Creation timestamp in UTC microseconds since Unix epoch.
    pub created_at_us: i64,
    /// Last update timestamp in UTC microseconds since Unix epoch.
    pub updated_at_us: i64,
    /// Expiration timestamp in UTC microseconds since Unix epoch.
    pub expires_at_us: i64,
}

/// HTTP response body for `GET /api/join-requests/mine` and `GET /api/join-requests/incoming`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinRequestsListResponse {
    /// Matching join requests ordered by `updated_at_us DESC`.
    pub requests: Vec<JoinRequestRecord>,
}

/// HTTP request body for `POST /api/join-requests/:request_id/decide`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecideJoinRequestBody {
    /// `true` to approve and issue a passwordless grant; `false` to deny.
    pub approve: bool,
    /// Optional note shown to the requester.
    #[serde(default)]
    pub decision_note: Option<String>,
}

/// Per-user anti-bot resource quota usage and configured ceilings (`GET /api/auth/quotas`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountQuotaUsage {
    /// Number of active (non-archived) campaigns currently owned by the caller.
    pub campaigns_used: usize,
    /// Configured maximum active campaigns allowed for this account.
    pub campaigns_limit: usize,
    /// Number of saved sessions currently owned by the caller.
    pub sessions_used: usize,
    /// Configured maximum saved sessions allowed for this account.
    pub sessions_limit: usize,
    /// Number of currently `PENDING` outbound join requests submitted by the caller.
    pub pending_requests_used: usize,
    /// Configured maximum concurrent `PENDING` join requests allowed per user.
    pub pending_requests_limit: usize,
}

/// HTTP request body for `PATCH /api/auth/profile`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    /// Optional updated display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Optional updated email address (pass `Some("")` to clear).
    #[serde(default)]
    pub email: Option<String>,
}

/// HTTP request body for `POST /api/admin/users/role`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminSetUserRoleRequest {
    /// Target username whose platform role is being updated.
    pub username: String,
    /// Target [`crate::protocol::AccountRole`] (`USER`, `GM`, or `ADMIN`).
    pub account_role: crate::protocol::AccountRole,
}

/// Account-level real-time notification packet pushed over `/ws/notifications`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountWsPacket {
    /// Initial handshake confirmation with current pending incoming request count.
    Connected {
        /// Authenticated `user_id`.
        user_id: String,
        /// Number of pending join requests awaiting this GM's decision.
        pending_incoming_count: usize,
    },
    /// Delivered to an online GM when a player submits a new join request.
    JoinRequestReceived {
        /// Newly created [`JoinRequestRecord`].
        request: Box<JoinRequestRecord>,
    },
    /// Delivered to an online player when the GM approves or denies their join request.
    JoinRequestResolved {
        /// Updated [`JoinRequestRecord`] (includes `approval_token` when `Approved`).
        request: Box<JoinRequestRecord>,
    },
    /// Synchronizes the GM's pending incoming request count badge.
    PendingCountSync {
        /// Updated count of pending incoming join requests.
        pending_incoming_count: usize,
    },
}
