//! Thread-local client state (`SatyaWasmState`) and WebRTC peer entries (`PeerEntry`)
//! managed in Rust/WebAssembly across the session lifecycle.
//!
//! ## Complete Browser `thread_local!` Topology
//! 1. **`CLIENT_STATE` ([`SatyaWasmState`], this file)**: Auth/session tokens, roster, WebSocket,
//!    WebRTC peers, prosody audio sampler, chat history/recipients, drawer geometry, combat clock,
//!    Google Drive/GM Vault caches, NPCs, [`CharacterWizardState`](crate::wasm_client::wizard::CharacterWizardState),
//!    and [`CharacterSheetState`](crate::wasm_client::character_sheet::CharacterSheetState).
//! 2. **`MAP_UI_STATE` ([`crate::wasm_client::map_3d`])**: Tactical Map grid, tokens, gait reach,
//!    and LoS/Cover overlay state (`with_map_ui` / `with_map_ui_mut`).
//! 3. **`CUSTOM_GLB_MODEL` ([`crate::wasm_client::model_3d_wgpu`])**: User-imported or vault-loaded
//!    binary glTF 2.0 (`.glb`) mesh & raw bytes shared by the 3D Model tab and Tactical Map tokens.
//! 4. **WebGPU Canvas Controllers (`wasm32` only)**: `MAP_CTRL` (`map_3d_wgpu.rs`), `DICE_CONTROLLER`
//!    (`dice_3d_wgpu.rs`), and `MODEL_CTRL` (`model_3d_wgpu.rs`).

use crate::wasm_client::workspace_drawers::FloatingPaneKind;
use satya_schema::{
    AccountQuotaUsage, AuthSuccessResponse, CampaignLoreChapterSummary, CampaignSummary,
    ChannelContext, DirectorySearchResponse, GmDocumentRecord, GmLookupHit, GmMemoryNoteRecord,
    JoinRequestRecord, Participant, SavedSessionSummary, SessionMode, SessionNpcEntry,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use web_sys::{AudioContext, MediaStream, RtcDataChannel, RtcPeerConnection, WebSocket};

/// Scope toggle in the Campaign & Session lobby (`My Campaigns` vs `Find Another User`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LobbyScope {
    /// Caller's own campaigns, standalone sessions, and joined sessions.
    #[default]
    Mine,
    /// Directory search for another user's discoverable campaigns and sessions.
    Search,
}

/// Current step in the progressive-disclosure Campaign & Session modal wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LobbyStep {
    /// Step 1: Choose an owned campaign, '+ Create New Campaign', or search another user's campaigns.
    #[default]
    ChooseCampaign,
    /// Step 2: Choose a session in the selected campaign, '+ New Session' / '+ Request New Session',
    /// or configure the new campaign (Row 5).
    ChooseSession,
    /// Step 3: Focused action card or compact form for the chosen session (Rows 1-4).
    TakeAction,
}

/// Active campaign-column selection in the Campaign & Session lobby.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CampaignSelectionKey {
    /// Nothing explicitly selected yet (triggers auto-selection on refresh).
    #[default]
    None,
    /// `+ New Campaign` selected (Row 5: `New | Self | New`).
    NewCampaign,
    /// Existing owned campaign (`campaign_id`).
    OwnedCampaign(String),
    /// Legacy standalone sessions owned by the caller (`campaign_id == None`).
    StandaloneOwned,
    /// Sessions owned by other users that the caller has already joined.
    JoinedOther,
    /// Another user's discoverable campaign (`owner_user_id`, `Option<campaign_id>`).
    OtherCampaign {
        /// Owning user's persistent `user_id`.
        owner_user_id: String,
        /// Campaign ID (`None` for uncategorized standalone bucket).
        campaign_id: Option<String>,
    },
}

/// Active session-column selection in the Campaign & Session lobby.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SessionSelectionKey {
    /// Nothing selected in the session column.
    #[default]
    None,
    /// Existing session (`session_id`).
    Existing(String),
    /// `+ New Session` (Row 3) or `+ Request New Session` (Row 4).
    NewSession,
}

/// Canonical temporal granularity across the UI and engine (`1 Tick = 0.25s`, `20 Ticks = 5.0s`).
pub const SECONDS_PER_TICK: f64 = 0.25;

/// Snapshot of the player's currently planned Action Mode and Tactical Triad (`Tempo` / `Control` / `Force`)
/// shared between `#ttrpg-drawer` (`Task Resolution & Action Pool`) and `#notebook-pane-combat` (`Combat Tracker`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedCombatActionState {
    /// Canonical `CombatTechniqueId::as_str()` code when a combat technique is selected; `None` for Non-Combat.
    pub technique_code: Option<String>,
    /// Human-readable technique name when a combat technique is selected; `None` for Non-Combat.
    pub technique_name: Option<String>,
    /// Planned Tempo Pool (`T`) allocation.
    pub tempo: i32,
    /// Planned Control Pool (`C`) allocation.
    pub control: i32,
    /// Planned Force Pool (`F`) allocation.
    pub force: i32,
    /// Character's effective Tactical Pool (`P_eff`) for the active parameters.
    pub p_eff: i32,
    /// Effective Count-Up Tick cost (`effective_tc`) when a combat technique is active.
    pub effective_tc: Option<i32>,
    /// Whether the planned `T / C / F` split respects `P_eff` and all aspect caps.
    pub valid: bool,
    /// Tactical Triad archetype descriptor (e.g. `"Balanced"`, `"Blitz Tempo"`).
    pub archetype_label: Option<String>,
}

impl Default for PlannedCombatActionState {
    fn default() -> Self {
        Self {
            technique_code: None,
            technique_name: None,
            tempo: 7,
            control: 7,
            force: 7,
            p_eff: 0,
            effective_tc: None,
            valid: true,
            archetype_label: None,
        }
    }
}

/// Active WebRTC peer connection and optional `satya-chat-dc` DataChannel for a remote participant.
pub struct PeerEntry {
    /// Browser `RTCPeerConnection` instance for this remote peer.
    pub pc: RtcPeerConnection,
    /// Optional `RTCDataChannel` opened with this remote peer.
    pub dc: Option<RtcDataChannel>,
    /// Whether `setRemoteDescription` has completed on `pc`.
    pub remote_desc_set: bool,
    /// Remote ICE candidate JSON payloads received before `setRemoteDescription` completed.
    pub pending_ice_candidates: Vec<String>,
    /// Client timestamp (`Date::now()`) of the last accepted `SignalingOffer` from this peer.
    pub last_offer_at_ms: f64,
}

/// Client-side state managed in Rust/Wasm across the session lifecycle.
pub struct SatyaWasmState {
    /// Active bearer authentication token (`satya_tok_<uuid>`) if logged in.
    pub auth_token: Option<String>,
    /// Authenticated user profile returned by `/api/auth/login` or `/api/auth/register`.
    pub user: Option<AuthSuccessResponse>,
    /// Active session room ID if currently connected to a session.
    pub session_id: Option<String>,
    /// Business mode (`TTRPG` or `MEETING`) of the active or selected session.
    pub session_mode: SessionMode,
    /// Single-session WebSocket join token (`join_<uuid>`) for the active session.
    pub join_token: Option<String>,
    /// Full [`Participant`] record for the local user in the active session.
    pub self_participant: Option<Participant>,
    /// Current session participant roster synchronized via `ROSTER_SYNC`.
    pub roster: Vec<Participant>,
    /// Active browser `WebSocket` connection to `/ws/session`.
    pub ws: Option<WebSocket>,
    /// Local camera/microphone `MediaStream` captured via `getUserMedia`.
    pub local_stream: Option<MediaStream>,
    /// Whether the local microphone track is currently muted.
    pub audio_muted: bool,
    /// Whether the local camera video track is currently muted.
    pub video_muted: bool,
    /// Active WebRTC peer connections keyed by remote `user_id`.
    pub peers: HashMap<String, PeerEntry>,
    /// Timestamp (`Date::now()`) when the user started typing the current chat message.
    pub typing_started_at_ms: Option<f64>,
    /// Peak WebAudio RMS energy observed during the current speaking turn window.
    pub rms_peak: f32,
    /// Cumulative sum of WebAudio RMS energy samples during the current window.
    pub rms_sum: f32,
    /// Number of WebAudio RMS energy samples collected during the current window.
    pub rms_samples: u32,
    /// Active browser `AudioContext` driving the `250ms` WebAudio RMS prosody sampler.
    pub prosody_audio_ctx: Option<AudioContext>,
    /// Active `window.setInterval` handle (`i32`) for the `250ms` prosody sampler.
    pub prosody_interval_id: Option<i32>,
    /// Up/Down arrow history buffer for the Session Chat input box (`#chat-input`).
    pub chat_history: crate::chat_cmd::ChatInputHistory,
    /// Selected private recipient `user_id`s in the vertical chat user list (empty = `BROADCAST` selected).
    pub selected_recipients: Vec<String>,
    /// Currently selected mutually exclusive [`ChannelContext`] toggle in the chat input bar.
    pub active_channel_context: ChannelContext,
    /// Currently active tab slug in the 10-tab Left Tabletop Notebook (`"maps"`, `"dice"`, `"model"`, `"character"`, `"wizard"`, `"npcs"`, `"combat"`, `"rules"`, `"docs"`, `"iggy"`).
    pub active_notebook_tab: String,
    /// Whether the Web Cams pane (`#webrtc-card`) is currently detached as a floating frame.
    pub webcam_floating: bool,
    /// Whether the Session Chat pane (`#chat-card`) is currently detached as a floating frame.
    pub chat_floating: bool,
    /// Whether the Satya TTRPG Task Resolution pane (`#ttrpg-drawer`) is currently detached as a floating frame.
    pub ttrpg_floating: bool,
    /// Whether the Web Cams column/drawer (`#webrtc-dock-column` / `#webrtc-card`) is collapsed to the right-edge rail.
    pub webcam_collapsed: bool,
    /// Whether the Session Chat drawer (`#chat-card`) in `#center-column` is collapsed.
    pub chat_collapsed: bool,
    /// Whether the Satya TTRPG Task Resolution drawer (`#ttrpg-drawer`) in `#center-column` is collapsed.
    pub ttrpg_collapsed: bool,
    /// Number of vertical columns in `#video-grid` (`1` or `2` — capped at most 2 columns).
    pub webcam_columns: u8,
    /// Active pointer drag `(pane, offset_x_px, offset_y_px)` while dragging a floating pane header.
    pub active_pane_drag: Option<(FloatingPaneKind, f64, f64)>,
    /// Saved viewport-clamped `(left_px, top_px)` position while Session Chat is floating.
    pub chat_float_pos_px: Option<(f64, f64)>,
    /// Saved viewport-clamped `(left_px, top_px)` position while Task Resolution is floating.
    pub ttrpg_float_pos_px: Option<(f64, f64)>,
    /// Saved viewport-clamped `(left_px, top_px)` position while Web Cams is floating.
    pub webcam_float_pos_px: Option<(f64, f64)>,
    /// Stacking z-order of floating workspace panes (`index 2` = topmost, rendered at `z-index: 97`).
    pub floating_z_order: [FloatingPaneKind; 3],
    /// Whether the user is currently dragging `#workspace-col-splitter` to resize `#notebook-column` vs `#center-column`.
    pub column_resizing: bool,
    /// Current Satya combat round counter in the Combat Tracker tab.
    pub combat_round: u32,
    /// Current Satya Action-Count (Tick) clock in the Combat Tracker tab.
    pub combat_tick: u32,
    /// Authoritative server `CombatantTickEntry` rows received for the active session.
    pub combat_combatants: Vec<satya_schema::CombatantTickEntry>,
    /// Number of consecutive automatic WebSocket reconnect attempts for the active session.
    pub ws_reconnect_attempts: u32,
    /// Set to `true` on explicit leave/logout to suppress automatic WebSocket reconnection.
    pub ws_reconnect_suppressed: bool,
    /// Prevents scheduling duplicate overlapping WebSocket reconnect timers.
    pub ws_reconnect_scheduled: bool,
    /// Local UI mirror of whether autonomous AI Player rallies are currently paused.
    pub ai_rally_paused: bool,
    /// Local UI mirror of the per-round autonomous AI rally limit (`1..=10`).
    pub ai_rally_max_rounds: u32,
    /// Whether the current user has linked a Google account (`drive.file` scope).
    pub google_linked: bool,
    /// Ephemeral browser-side Google OAuth 2.0 `drive.file` access token (if authenticated via Google Identity Services).
    pub google_access_token: Option<String>,
    /// Optional Google Cloud OAuth 2.0 Web Client ID loaded from `/api/auth/google/status` or user input.
    pub google_oauth_client_id: Option<String>,
    /// Optional Google Cloud Picker API Key loaded from `/api/auth/google/status` or user input.
    pub google_picker_api_key: Option<String>,
    /// Cached `GM_ONLY` documents visible to the current user in the active session.
    pub gm_documents: Vec<GmDocumentRecord>,
    /// Cached persistent and session-scoped GM memory notes for the active session/campaign.
    pub gm_memories: Vec<GmMemoryNoteRecord>,
    /// Whether the user has executed a GM Archivist omnisearch (`#gm-omnisearch-results`) in the current view.
    pub gm_omnisearch_active: bool,
    /// Cached NPC entries for the `NPC List` tab (`#notebook-pane-npcs`).
    pub session_npcs: Vec<SessionNpcEntry>,
    /// Currently selected NPC `character_id` in the `NPC List` tab.
    pub selected_npc_id: Option<String>,
    /// Whether the current viewer is allowed to see GM-only NPC secrets (`true` for GM or in Solo/AI mode; `false` for non-GM players when a human GM runs the session).
    pub viewer_can_see_gm_secrets: bool,
    /// Interactive state for the Character Creation Wizard notebook tab (`#notebook-pane-wizard`).
    pub wizard: crate::wasm_client::wizard::CharacterWizardState,
    /// Interactive state for the Character Sheet / Status notebook tab (`#notebook-pane-character`).
    pub character_sheet: crate::wasm_client::character_sheet::CharacterSheetState,
    /// Whether the Outcome Distribution Graph breakdown table in `#ttrpg-odds-readout` is expanded.
    pub ttrpg_odds_table_expanded: bool,
    /// Planned Action Mode and `T / C / F` allocation synchronized between Task Resolution and Combat Tracker.
    pub planned_combat_action: PlannedCombatActionState,
    /// Solo Co-GM optional Spoiler Shield (`false` by default = full Co-GM access; `true` = player-safe view).
    pub solo_spoiler_shield: bool,
    /// Active campaign pack name reported by `/api/sessions/:id/npcs` (e.g. `"awakened"`).
    pub campaign_pack_name: String,
    /// Universal starter NPC catalog returned by `/api/sessions/:id/npcs`.
    pub campaign_npcs: Vec<SessionNpcEntry>,
    /// Campaign lore chapter catalog currently available to this viewer.
    pub campaign_lore_catalog: Vec<CampaignLoreChapterSummary>,
    /// Active source mode in the unified `Rules & Lore` reader (`"RULES"` or `"CAMPAIGN_LORE"`).
    pub reader_source_mode: String,
    /// Cached hits from the most recent GM Archivist omnisearch query.
    pub last_omnisearch_hits: Vec<GmLookupHit>,
    /// Item ID currently displayed inside `#gm-archivist-modal`, if open.
    pub archivist_modal_item_id: Option<String>,
    /// Campaigns owned by the authenticated caller (`GET /api/campaigns`).
    pub my_campaigns: Vec<CampaignSummary>,
    /// Saved sessions in which the caller has participated or hosted (`GET /api/sessions/saved`).
    pub saved_sessions: Vec<SavedSessionSummary>,
    /// Active lobby scope toggle (`Mine` vs `Search`).
    pub lobby_scope: LobbyScope,
    /// Current progressive-disclosure wizard step (`ChooseCampaign` -> `ChooseSession` -> `TakeAction`).
    pub lobby_step: LobbyStep,
    /// Selected user ID when viewing directory search results.
    pub selected_directory_user_id: Option<String>,
    /// Active campaign-column selection in the lobby.
    pub selected_campaign_key: CampaignSelectionKey,
    /// Active session-column selection in the lobby.
    pub selected_session_key: SessionSelectionKey,
    /// Whether the collapsible Requests tray in the lobby is currently open.
    pub requests_tray_open: bool,
    /// Whether the optional invite-password input is expanded in Row 2 (`Request to Join` view).
    pub show_join_password_box: bool,
    /// Most recent GM directory search response (`GET /api/directory/search`).
    pub directory_results: Option<DirectorySearchResponse>,
    /// Incoming join requests awaiting this GM's decision (`GET /api/join-requests/incoming`).
    pub incoming_join_requests: Vec<JoinRequestRecord>,
    /// Outbound join requests submitted by this user (`GET /api/join-requests/mine`).
    pub my_join_requests: Vec<JoinRequestRecord>,
    /// Per-user quota usage and limits (`GET /api/auth/quotas`).
    pub quota_usage: Option<AccountQuotaUsage>,
    /// Account-level notification WebSocket (`/ws/notifications`).
    pub notifications_ws: Option<WebSocket>,
    /// Whether the header notifications popover is currently expanded.
    pub notifications_popover_open: bool,
}

impl Default for SatyaWasmState {
    fn default() -> Self {
        Self {
            auth_token: None,
            user: None,
            session_id: None,
            session_mode: SessionMode::Ttrpg,
            join_token: None,
            self_participant: None,
            roster: Vec::new(),
            ws: None,
            local_stream: None,
            audio_muted: false,
            video_muted: false,
            peers: HashMap::new(),
            typing_started_at_ms: None,
            rms_peak: 0.0,
            rms_sum: 0.0,
            rms_samples: 0,
            prosody_audio_ctx: None,
            prosody_interval_id: None,
            chat_history: crate::chat_cmd::ChatInputHistory::new(),
            selected_recipients: Vec::new(),
            active_channel_context: ChannelContext::InCharacter,
            active_notebook_tab: "maps".to_string(),
            webcam_floating: false,
            chat_floating: false,
            ttrpg_floating: false,
            webcam_collapsed: false,
            chat_collapsed: false,
            ttrpg_collapsed: false,
            webcam_columns: 1,
            active_pane_drag: None,
            chat_float_pos_px: None,
            ttrpg_float_pos_px: None,
            webcam_float_pos_px: None,
            floating_z_order: [
                FloatingPaneKind::Chat,
                FloatingPaneKind::Ttrpg,
                FloatingPaneKind::Webcam,
            ],
            column_resizing: false,
            combat_round: 1,
            combat_tick: 0,
            combat_combatants: Vec::new(),
            ws_reconnect_attempts: 0,
            ws_reconnect_suppressed: false,
            ws_reconnect_scheduled: false,
            ai_rally_paused: false,
            ai_rally_max_rounds: 5,
            google_linked: false,
            google_access_token: None,
            google_oauth_client_id: None,
            google_picker_api_key: None,
            gm_documents: Vec::new(),
            gm_memories: Vec::new(),
            gm_omnisearch_active: false,
            session_npcs: Vec::new(),
            selected_npc_id: None,
            viewer_can_see_gm_secrets: false,
            wizard: crate::wasm_client::wizard::CharacterWizardState::default(),
            character_sheet: crate::wasm_client::character_sheet::CharacterSheetState::default(),
            ttrpg_odds_table_expanded: false,
            planned_combat_action: PlannedCombatActionState::default(),
            solo_spoiler_shield: false,
            campaign_pack_name: "awakened".to_string(),
            campaign_npcs: Vec::new(),
            campaign_lore_catalog: Vec::new(),
            reader_source_mode: "RULES".to_string(),
            last_omnisearch_hits: Vec::new(),
            archivist_modal_item_id: None,
            my_campaigns: Vec::new(),
            saved_sessions: Vec::new(),
            lobby_scope: LobbyScope::Mine,
            lobby_step: LobbyStep::ChooseCampaign,
            selected_directory_user_id: None,
            selected_campaign_key: CampaignSelectionKey::None,
            selected_session_key: SessionSelectionKey::None,
            requests_tray_open: false,
            show_join_password_box: false,
            directory_results: None,
            incoming_join_requests: Vec::new(),
            my_join_requests: Vec::new(),
            quota_usage: None,
            notifications_ws: None,
            notifications_popover_open: false,
        }
    }
}

thread_local! {
    static CLIENT_STATE: Rc<RefCell<SatyaWasmState>> = Rc::new(RefCell::new(SatyaWasmState::default()));
}

/// Executes a closure `f` with shared read-only access to the thread-local [`SatyaWasmState`].
pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&SatyaWasmState) -> R,
{
    CLIENT_STATE.with(|cell| f(&cell.borrow()))
}

/// Executes a closure `f` with exclusive mutable access to the thread-local [`SatyaWasmState`].
pub fn with_state_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut SatyaWasmState) -> R,
{
    CLIENT_STATE.with(|cell| f(&mut cell.borrow_mut()))
}
