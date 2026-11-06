//! Thread-local client state (`SatyaWasmState`) and WebRTC peer entries (`PeerEntry`)
//! managed in Rust/WebAssembly across the session lifecycle.

use satya_schema::{
    AuthSuccessResponse, ChannelContext, GmDocumentRecord, Participant, SessionMode,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use web_sys::{MediaStream, RtcDataChannel, RtcPeerConnection, WebSocket};

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
    /// Up/Down arrow history buffer for the Session Chat input box (`#chat-input`).
    pub chat_history: crate::chat_cmd::ChatInputHistory,
    /// Selected private recipient `user_id`s in the vertical chat user list (empty = `BROADCAST` selected).
    pub selected_recipients: Vec<String>,
    /// Currently selected mutually exclusive [`ChannelContext`] toggle in the chat input bar.
    pub active_channel_context: ChannelContext,
    /// Currently active tab slug in the Left Tabbed Notebook (`"maps"`, `"dice"`, `"model"`, `"character"`, `"combat"`, `"iggy"`).
    pub active_notebook_tab: String,
    /// Whether the Web Cams pane (`#webrtc-card`) is currently detached as a floating frame.
    pub webcam_floating: bool,
    /// Whether the Web Cams column/drawer (`#webrtc-dock-column` / `#webrtc-card`) is collapsed to the right-edge rail.
    pub webcam_collapsed: bool,
    /// Whether the Session Chat drawer (`#chat-card`) in `#center-column` is collapsed.
    pub chat_collapsed: bool,
    /// Whether the Satya TTRPG Task Resolution drawer (`#ttrpg-drawer`) in `#center-column` is collapsed.
    pub ttrpg_collapsed: bool,
    /// Number of vertical columns in `#video-grid` (`1` or `2` — capped at most 2 columns).
    pub webcam_columns: u8,
    /// Active drag offset `(offset_x, offset_y)` while dragging the floating Web Cams frame.
    pub webcam_drag_offset: Option<(f64, f64)>,
    /// Current Satya combat round counter in the Combat Tracker tab.
    pub combat_round: u32,
    /// Current Satya Action-Count (Tick) clock in the Combat Tracker tab.
    pub combat_tick: u32,
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
    /// Interactive state for the Character Creation Wizard notebook tab (`#notebook-pane-wizard`).
    pub wizard: crate::wasm_client::wizard::CharacterWizardState,
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
            chat_history: crate::chat_cmd::ChatInputHistory::new(),
            selected_recipients: Vec::new(),
            active_channel_context: ChannelContext::InCharacter,
            active_notebook_tab: "maps".to_string(),
            webcam_floating: false,
            webcam_collapsed: false,
            chat_collapsed: false,
            ttrpg_collapsed: false,
            webcam_columns: 1,
            webcam_drag_offset: None,
            combat_round: 1,
            combat_tick: 0,
            google_linked: false,
            google_access_token: None,
            google_oauth_client_id: None,
            google_picker_api_key: None,
            gm_documents: Vec::new(),
            wizard: crate::wasm_client::wizard::CharacterWizardState::default(),
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
