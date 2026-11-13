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
    AuthSuccessResponse, ChannelContext, GmDocumentRecord, GmMemoryNoteRecord, Participant,
    SessionMode, SessionNpcEntry,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use web_sys::{AudioContext, MediaStream, RtcDataChannel, RtcPeerConnection, WebSocket};

/// Canonical temporal granularity across the UI and engine (`1 Tick = 0.25s`, `20 Ticks = 5.0s`).
pub const SECONDS_PER_TICK: f64 = 0.25;

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
            google_linked: false,
            google_access_token: None,
            google_oauth_client_id: None,
            google_picker_api_key: None,
            gm_documents: Vec::new(),
            gm_memories: Vec::new(),
            session_npcs: Vec::new(),
            selected_npc_id: None,
            viewer_can_see_gm_secrets: true,
            wizard: crate::wasm_client::wizard::CharacterWizardState::default(),
            character_sheet: crate::wasm_client::character_sheet::CharacterSheetState::default(),
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
