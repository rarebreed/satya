//! Pure Rust + WebAssembly (`wasm-bindgen` / `web-sys`) client runtime for the Satya
//! WebRTC Audio/Video, Chat, TTRPG Mechanics, and Apache Iggy Event Streaming platform.

use js_sys::{Array, Float32Array, Reflect};
use satya_schema::{
    AuthSuccessResponse, ChannelContext, ChatMessageEvent, ClientWsPacket, CombatStance,
    CreateSessionRequest, InteractionTurnType, JoinSessionRequest, LeaveSessionRequest,
    LoginRequest, LogoutRequest, MessageVisibility, Participant, ParticipantKind, RegisterRequest,
    ServerWsPacket, SessionEnterResponse, SessionMode, SessionRole, StorageMetricsViewResponse,
    TopicMetricsView, TtrpgMechanicsEvent, WebRtcEventKind,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    AnalyserNode, AudioContext, Document, Element, Headers, HtmlButtonElement, HtmlElement,
    HtmlInputElement, HtmlSelectElement, HtmlVideoElement, KeyboardEvent, MediaStream,
    MediaStreamConstraints, MessageEvent, Request, RequestInit, Response, RtcConfiguration,
    RtcDataChannel, RtcDataChannelEvent, RtcIceCandidate, RtcIceCandidateInit, RtcPeerConnection,
    RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit, RtcTrackEvent, WebSocket,
    Window,
};

/// Active WebRTC peer connection and optional `satya-chat-dc` DataChannel for a remote participant.
pub struct PeerEntry {
    /// Browser `RTCPeerConnection` instance for this remote peer.
    pub pc: RtcPeerConnection,
    /// Optional `RTCDataChannel` opened with this remote peer.
    pub dc: Option<RtcDataChannel>,
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

// ============================================================================
// Typed DOM Helpers (`web-sys`)
// ============================================================================

fn window() -> Option<Window> {
    web_sys::window()
}

fn document() -> Option<Document> {
    window()?.document()
}

fn get_element(id: &str) -> Option<Element> {
    document()?.get_element_by_id(id)
}

fn input_value(id: &str) -> String {
    get_element(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|inp| inp.value())
        .unwrap_or_default()
}

fn set_input_value(id: &str, val: &str) {
    if let Some(inp) = get_element(id).and_then(|el| el.dyn_into::<HtmlInputElement>().ok()) {
        inp.set_value(val);
    }
}

fn input_checked(id: &str) -> bool {
    get_element(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|inp| inp.checked())
        .unwrap_or(false)
}

fn select_value(id: &str) -> String {
    get_element(id)
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|sel| sel.value())
        .unwrap_or_default()
}

fn set_select_value(id: &str, val: &str) {
    if let Some(sel) = get_element(id).and_then(|el| el.dyn_into::<HtmlSelectElement>().ok()) {
        sel.set_value(val);
    }
}

fn set_text(id: &str, text: &str) {
    if let Some(el) = get_element(id) {
        el.set_text_content(Some(text));
    }
}

fn set_class(id: &str, class_name: &str) {
    if let Some(el) = get_element(id) {
        el.set_class_name(class_name);
    }
}

fn set_button_disabled(id: &str, disabled: bool) {
    if let Some(btn) = get_element(id).and_then(|el| el.dyn_into::<HtmlButtonElement>().ok()) {
        btn.set_disabled(disabled);
    }
}

fn set_display(id: &str, display: &str) {
    if let Some(el) = get_element(id).and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        let _ = el.style().set_property("display", display);
    }
}

/// Updates a status feedback element (`#auth-feedback` or `#session-feedback`) with styled text.
pub fn set_feedback(id: &str, text: &str, is_error: bool) {
    if let Some(el) = get_element(id).and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        el.set_text_content(Some(text));
        let color = if is_error { "#f43f5e" } else { "#38bdf8" };
        let _ = el.style().set_property("color", color);
    }
}

/// Escapes `&`, `<`, and `>` characters in `raw` for safe HTML text interpolation.
pub fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ============================================================================
// Typed HTTP Fetch Helpers (`web-sys::Request` / `Response`)
// ============================================================================

async fn fetch_json<Req: Serialize, Resp: DeserializeOwned>(
    method: &str,
    url: &str,
    auth_token: Option<&str>,
    body: Option<&Req>,
) -> Result<Resp, String> {
    let win = window().ok_or_else(|| "No browser Window available".to_string())?;
    let opts = RequestInit::new();
    opts.set_method(method);

    let headers = Headers::new().map_err(|_| "Failed to construct Headers".to_string())?;
    let _ = headers.set("Content-Type", "application/json");
    if let Some(tok) = auth_token {
        let _ = headers.set("Authorization", &format!("Bearer {tok}"));
    }
    opts.set_headers(&headers);

    if let Some(payload) = body {
        let json_str = serde_json::to_string(payload).map_err(|e| e.to_string())?;
        opts.set_body(&JsValue::from_str(&json_str));
    }

    let request =
        Request::new_with_str_and_init(url, &opts).map_err(|_| "Invalid Request".to_string())?;
    let resp_val = JsFuture::from(win.fetch_with_request(&request))
        .await
        .map_err(|_| "Network fetch failed".to_string())?;
    let resp: Response = resp_val
        .dyn_into()
        .map_err(|_| "Invalid Response cast".to_string())?;

    let text_promise = resp
        .text()
        .map_err(|_| "Failed to read response text".to_string())?;
    let text_val = JsFuture::from(text_promise)
        .await
        .map_err(|_| "Failed awaiting response text".to_string())?;
    let text = text_val.as_string().unwrap_or_default();

    if !resp.ok() {
        if let Ok(err_val) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(msg) = err_val.get("error").and_then(|v| v.as_str())
        {
            return Err(msg.to_string());
        }
        return Err(format!("HTTP {}: {}", resp.status(), text));
    }

    serde_json::from_str::<Resp>(&text).map_err(|e| format!("JSON decode error: {e}"))
}

// ============================================================================
// Pure Rust Role / Enum Parsing Helpers
// ============================================================================

/// Parses a `<select>` option string (`"TTRPG"` or `"MEETING"`) into a [`SessionMode`].
pub fn parse_session_mode(raw: &str) -> SessionMode {
    match raw {
        "MEETING" => SessionMode::Meeting,
        _ => SessionMode::Ttrpg,
    }
}

/// Parses a `<select>` option string into a [`SessionRole`].
pub fn parse_session_role(raw: &str) -> SessionRole {
    match raw {
        "GM" => SessionRole::Gm,
        "AI_GM" => SessionRole::AiGm,
        "HOST" => SessionRole::Host,
        "PARTICIPANT" => SessionRole::Participant,
        "OBSERVER" => SessionRole::Observer,
        _ => SessionRole::Player,
    }
}

/// Parses a `<select>` option string into a [`MessageVisibility`].
pub fn parse_visibility(raw: &str) -> MessageVisibility {
    match raw {
        "PRIVATE" => MessageVisibility::Private,
        "GM_ONLY" => MessageVisibility::GmOnly,
        "HOST_ONLY" => MessageVisibility::HostOnly,
        _ => MessageVisibility::Public,
    }
}

/// Parses a `<select>` option string into a [`ChannelContext`].
pub fn parse_channel_context(raw: &str) -> ChannelContext {
    match raw {
        "OUT_OF_CHARACTER" => ChannelContext::OutOfCharacter,
        "GM_NARRATION" => ChannelContext::GmNarration,
        "MEETING_PUBLIC" => ChannelContext::MeetingPublic,
        "MEETING_BACKCHANNEL" => ChannelContext::MeetingBackchannel,
        "SYSTEM_NOTICE" => ChannelContext::SystemNotice,
        _ => ChannelContext::InCharacter,
    }
}

/// Parses a `<select>` option string into an optional Satya TTRPG [`CombatStance`].
pub fn parse_combat_stance(raw: &str) -> Option<CombatStance> {
    match raw {
        "POWER" => Some(CombatStance::Power),
        "MANEUVER" => Some(CombatStance::Maneuver),
        "STABILITY" => Some(CombatStance::Stability),
        "RANGED_AIMED" => Some(CombatStance::RangedAimed),
        "RANGED_SNAP" => Some(CombatStance::RangedSnap),
        _ => None,
    }
}

// ============================================================================
// Exported Wasm Entrypoints & Event Handlers (`#[wasm_bindgen]`)
// ============================================================================

/// WebAssembly module entrypoint automatically invoked when `/pkg/satya_web.js` initializes.
#[wasm_bindgen(start)]
pub fn start_satya_wasm_client() {
    register_window_bindings();
    satya_visibility_changed();
    satya_refresh_metrics();
}

/// Binds the Rust Wasm handlers onto `window` so both SSR HTML attributes and CSR/hydrated
/// Leptos views invoke the compiled Rust WebAssembly functions directly.
pub fn register_window_bindings() {
    let Some(win) = window() else {
        return;
    };
    let bind = |name: &str, func: &JsValue| {
        let _ = Reflect::set(&win, &JsValue::from_str(name), func);
    };

    let c_fill = Closure::<dyn Fn(String, String, String)>::new(satya_fill_demo);
    bind("satyaFillDemo", c_fill.as_ref());
    c_fill.forget();

    let c_mode = Closure::<dyn Fn()>::new(satya_mode_changed);
    bind("satyaModeChanged", c_mode.as_ref());
    c_mode.forget();

    let c_reg = Closure::<dyn Fn()>::new(satya_register);
    bind("satyaRegister", c_reg.as_ref());
    c_reg.forget();

    let c_login = Closure::<dyn Fn()>::new(satya_login);
    bind("satyaLogin", c_login.as_ref());
    c_login.forget();

    let c_logout = Closure::<dyn Fn()>::new(satya_logout);
    bind("satyaLogout", c_logout.as_ref());
    c_logout.forget();

    let c_create = Closure::<dyn Fn()>::new(satya_create_session);
    bind("satyaCreateSession", c_create.as_ref());
    c_create.forget();

    let c_join = Closure::<dyn Fn()>::new(satya_join_session);
    bind("satyaJoinSession", c_join.as_ref());
    c_join.forget();

    let c_leave = Closure::<dyn Fn()>::new(satya_leave_session);
    bind("satyaLeaveSession", c_leave.as_ref());
    c_leave.forget();

    let c_cam = Closure::<dyn Fn()>::new(satya_start_camera);
    bind("satyaStartCamera", c_cam.as_ref());
    c_cam.forget();

    let c_aud = Closure::<dyn Fn()>::new(satya_toggle_audio);
    bind("satyaToggleAudio", c_aud.as_ref());
    c_aud.forget();

    let c_vid = Closure::<dyn Fn()>::new(satya_toggle_video);
    bind("satyaToggleVideo", c_vid.as_ref());
    c_vid.forget();

    let c_vad = Closure::<dyn Fn()>::new(satya_emit_prosody_sample);
    bind("satyaEmitProsodySample", c_vad.as_ref());
    c_vad.forget();

    let c_vis = Closure::<dyn Fn()>::new(satya_visibility_changed);
    bind("satyaVisibilityChanged", c_vis.as_ref());
    c_vis.forget();

    let c_key = Closure::<dyn Fn(KeyboardEvent)>::new(satya_chat_keydown);
    bind("satyaChatKeydown", c_key.as_ref());
    c_key.forget();

    let c_chat = Closure::<dyn Fn()>::new(satya_send_chat);
    bind("satyaSendChat", c_chat.as_ref());
    c_chat.forget();

    let c_roll = Closure::<dyn Fn()>::new(satya_roll_task);
    bind("satyaRollTask", c_roll.as_ref());
    c_roll.forget();

    let c_metrics = Closure::<dyn Fn()>::new(satya_refresh_metrics);
    bind("satyaRefreshMetrics", c_metrics.as_ref());
    c_metrics.forget();

    let c_flush = Closure::<dyn Fn()>::new(satya_flush_batches);
    bind("satyaFlushBatches", c_flush.as_ref());
    c_flush.forget();
}

/// Populates the login form inputs and default session mode/role for a preset demo account.
#[wasm_bindgen]
pub fn satya_fill_demo(username: String, password: String, display_name: String) {
    set_input_value("auth-username", &username);
    set_input_value("auth-password", &password);
    set_input_value("auth-display-name", &display_name);
    if username.starts_with("gm_") {
        set_select_value("session-mode-select", "TTRPG");
        set_select_value("session-role-select", "GM");
    } else if username.starts_with("host_") {
        set_select_value("session-mode-select", "MEETING");
        set_select_value("session-role-select", "HOST");
    } else {
        set_select_value("session-role-select", "PLAYER");
    }
}

/// Updates the default session role and toggles the TTRPG task resolution drawer when business mode changes.
#[wasm_bindgen]
pub fn satya_mode_changed() {
    let mode = select_value("session-mode-select");
    if mode == "MEETING" {
        set_select_value("session-role-select", "HOST");
        set_display("ttrpg-drawer", "none");
    } else {
        set_select_value("session-role-select", "GM");
        set_display("ttrpg-drawer", "block");
    }
}

/// Registers a new persistent user account via `POST /api/auth/register` and logs in automatically.
#[wasm_bindgen]
pub fn satya_register() {
    spawn_local(async move {
        let username = input_value("auth-username").trim().to_string();
        let password = input_value("auth-password");
        let raw_display = input_value("auth-display-name").trim().to_string();
        let display_name = if raw_display.is_empty() {
            username.clone()
        } else {
            raw_display
        };

        let req = RegisterRequest {
            username,
            password,
            display_name,
        };
        match fetch_json::<_, AuthSuccessResponse>("POST", "/api/auth/register", None, Some(&req))
            .await
        {
            Ok(data) => {
                set_feedback(
                    "auth-feedback",
                    &format!("Registered & authenticated user_id: {}", data.user_id),
                    false,
                );
                apply_auth_success(data);
            }
            Err(err) => set_feedback("auth-feedback", &err, true),
        }
    });
}

/// Authenticates the user with the credentials in `#auth-username` and `#auth-password` via `POST /api/auth/login`.
#[wasm_bindgen]
pub fn satya_login() {
    spawn_local(async move {
        let _ = perform_login().await;
    });
}

async fn perform_login() -> bool {
    let username = input_value("auth-username").trim().to_string();
    let password = input_value("auth-password");
    let req = LoginRequest { username, password };
    match fetch_json::<_, AuthSuccessResponse>("POST", "/api/auth/login", None, Some(&req)).await {
        Ok(data) => {
            set_feedback(
                "auth-feedback",
                &format!(
                    "Authenticated as {} (user_id: {})",
                    data.username, data.user_id
                ),
                false,
            );
            apply_auth_success(data);
            true
        }
        Err(err) => {
            set_feedback("auth-feedback", &err, true);
            false
        }
    }
}

fn apply_auth_success(data: AuthSuccessResponse) {
    let short_id: String = data.user_id.chars().take(8).collect();
    set_text(
        "auth-status-pill",
        &format!("User: {} ({short_id}...)", data.username),
    );
    set_class("auth-status-pill", "pill pill-active");
    set_button_disabled("btn-logout", false);

    with_state_mut(|st| {
        st.auth_token = Some(data.auth_token.clone());
        st.user = Some(data);
    });
}

fn set_session_buttons_enabled(in_session: bool) {
    set_button_disabled("btn-leave-session", !in_session);
    set_button_disabled("btn-workspace-disconnect", !in_session);
}

/// Creates a new password-protected session via `POST /api/sessions/create` and opens `/ws/session`.
#[wasm_bindgen]
pub fn satya_create_session() {
    spawn_local(async move {
        let has_token = with_state(|st| st.auth_token.is_some());
        if !has_token && !perform_login().await {
            return;
        }
        let Some(token) = with_state(|st| st.auth_token.clone()) else {
            return;
        };

        let session_id = input_value("session-id-input").trim().to_string();
        let session_password = input_value("session-password-input");
        let session_mode = parse_session_mode(&select_value("session-mode-select"));
        let role = parse_session_role(&select_value("session-role-select"));
        let enable_ai_gm = input_checked("enable-ai-gm");
        let display_raw = input_value("auth-display-name").trim().to_string();
        let display_name = if display_raw.is_empty() {
            None
        } else {
            Some(display_raw)
        };

        let req = CreateSessionRequest {
            session_id,
            session_password,
            session_mode,
            role,
            display_name,
            enable_ai_gm,
        };

        match fetch_json::<_, SessionEnterResponse>(
            "POST",
            "/api/sessions/create",
            Some(&token),
            Some(&req),
        )
        .await
        {
            Ok(data) => {
                set_feedback(
                    "session-feedback",
                    &format!(
                        "Created session '{}' ({:?})",
                        data.session_id, data.session_mode
                    ),
                    false,
                );
                enter_session(data);
            }
            Err(err) => set_feedback("session-feedback", &err, true),
        }
    });
}

/// Joins an existing password-protected session via `POST /api/sessions/join` and opens `/ws/session`.
#[wasm_bindgen]
pub fn satya_join_session() {
    spawn_local(async move {
        let has_token = with_state(|st| st.auth_token.is_some());
        if !has_token && !perform_login().await {
            return;
        }
        let Some(token) = with_state(|st| st.auth_token.clone()) else {
            return;
        };

        let session_id = input_value("session-id-input").trim().to_string();
        let session_password = input_value("session-password-input");
        let role = parse_session_role(&select_value("session-role-select"));
        let display_raw = input_value("auth-display-name").trim().to_string();
        let display_name = if display_raw.is_empty() {
            None
        } else {
            Some(display_raw)
        };

        let req = JoinSessionRequest {
            session_id,
            session_password,
            role,
            display_name,
        };

        match fetch_json::<_, SessionEnterResponse>(
            "POST",
            "/api/sessions/join",
            Some(&token),
            Some(&req),
        )
        .await
        {
            Ok(data) => {
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

fn enter_session(data: SessionEnterResponse) {
    set_text(
        "session-status-pill",
        &format!(
            "Session: {} [{:?} / {:?}]",
            data.session_id, data.session_mode, data.self_participant.role
        ),
    );
    set_class("session-status-pill", "pill pill-active");
    set_text(
        "local-participant-label",
        &format!(
            "{} ({:?})",
            data.self_participant.display_name, data.self_participant.role
        ),
    );

    with_state_mut(|st| {
        st.session_id = Some(data.session_id);
        st.session_mode = data.session_mode;
        st.join_token = Some(data.join_token);
        st.self_participant = Some(data.self_participant);
        st.roster = data.roster;
    });

    render_roster();
    set_session_buttons_enabled(true);
    connect_signaling_websocket();
    satya_refresh_metrics();
}

/// Closes all active WebRTC peer connections, DataChannels, local media tracks, and signaling WebSockets,
/// resetting the client's active session state and UI controls.
pub fn cleanup_client_session_state() {
    let remote_ids: Vec<String> = with_state_mut(|st| {
        let ids: Vec<String> = st.peers.keys().cloned().collect();
        for (_, peer) in st.peers.drain() {
            if let Some(dc) = peer.dc {
                dc.close();
            }
            peer.pc.close();
        }
        if let Some(stream) = st.local_stream.take() {
            let tracks = stream.get_tracks();
            for i in 0..tracks.length() {
                if let Ok(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
                    track.stop();
                }
            }
        }
        if let Some(ws) = st.ws.take() {
            let _ = ws.close();
        }
        st.session_id = None;
        st.join_token = None;
        st.self_participant = None;
        st.roster.clear();
        ids
    });

    for uid in remote_ids {
        if let Some(tile) = get_element(&format!("video-tile-{uid}")) {
            tile.remove();
        }
    }

    if let Some(video) =
        get_element("local-video").and_then(|e| e.dyn_into::<HtmlVideoElement>().ok())
    {
        video.set_src_object(None);
    }
    set_text("local-participant-label", "Local Preview");
    render_roster();
    set_session_buttons_enabled(false);
    set_text("session-status-pill", "No Active Session");
    set_class("session-status-pill", "pill pill-muted");
}

/// Disconnects the current user from the active session, emitting `LEAVE_SESSION` and calling `POST /api/sessions/leave`.
#[wasm_bindgen]
pub fn satya_leave_session() {
    spawn_local(async move {
        let (session_id, auth_token, peer_ids, audio_muted, video_muted) = with_state(|st| {
            (
                st.session_id.clone(),
                st.auth_token.clone(),
                st.peers.keys().cloned().collect::<Vec<_>>(),
                st.audio_muted,
                st.video_muted,
            )
        });

        let Some(departing_sid) = session_id else {
            set_feedback(
                "session-feedback",
                "Not currently connected to an active session",
                true,
            );
            return;
        };

        for remote_uid in peer_ids {
            send_ws_packet(&ClientWsPacket::WebrtcTelemetry {
                target_user_id: Some(remote_uid),
                event_kind: WebRtcEventKind::DataChannelClosed,
                audio_muted,
                video_muted,
            });
        }

        send_ws_packet(&ClientWsPacket::LeaveSession {
            reason: Some("User disconnected from session via Rust/Wasm UI".to_string()),
        });

        if let Some(tok) = auth_token {
            let req = LeaveSessionRequest {
                session_id: departing_sid.clone(),
                reason: Some("User disconnected from session via Rust/Wasm UI".to_string()),
            };
            let _ = fetch_json::<_, serde_json::Value>(
                "POST",
                "/api/sessions/leave",
                Some(&tok),
                Some(&req),
            )
            .await;
        }

        cleanup_client_session_state();
        set_feedback(
            "session-feedback",
            &format!("Disconnected from session '{departing_sid}'"),
            false,
        );
        append_system_notice(&format!(
            "Disconnected from session '{departing_sid}'. Logged PARTICIPANT_LEFT & PEER_DISCONNECTED events."
        ));
        satya_refresh_metrics();
    });
}

/// Logs out the currently authenticated user via `POST /api/auth/logout`, revoking their bearer token
/// and disconnecting them from any active session.
#[wasm_bindgen]
pub fn satya_logout() {
    spawn_local(async move {
        let (auth_token, username, active_sid, peer_ids, audio_muted, video_muted) =
            with_state(|st| {
                (
                    st.auth_token.clone(),
                    st.user
                        .as_ref()
                        .map(|u| u.username.clone())
                        .unwrap_or_else(|| "user".to_string()),
                    st.session_id.clone(),
                    st.peers.keys().cloned().collect::<Vec<_>>(),
                    st.audio_muted,
                    st.video_muted,
                )
            });

        let Some(tok) = auth_token else {
            set_feedback("auth-feedback", "No user is currently logged in", true);
            return;
        };

        for remote_uid in peer_ids {
            send_ws_packet(&ClientWsPacket::WebrtcTelemetry {
                target_user_id: Some(remote_uid),
                event_kind: WebRtcEventKind::DataChannelClosed,
                audio_muted,
                video_muted,
            });
        }

        let req = LogoutRequest {
            session_id: active_sid,
            reason: Some("User logged out of session via Rust/Wasm UI".to_string()),
        };
        let _ =
            fetch_json::<_, serde_json::Value>("POST", "/api/auth/logout", Some(&tok), Some(&req))
                .await;

        cleanup_client_session_state();
        with_state_mut(|st| {
            st.auth_token = None;
            st.user = None;
        });

        set_text("auth-status-pill", "Not Logged In");
        set_class("auth-status-pill", "pill pill-muted");
        set_button_disabled("btn-logout", true);

        set_feedback(
            "auth-feedback",
            &format!("Logged out user '{username}'"),
            false,
        );
        set_feedback("session-feedback", "", false);
        append_system_notice(&format!(
            "User '{username}' logged out. Logged USER_LOGGED_OUT lifecycle event."
        ));
        satya_refresh_metrics();
    });
}

fn render_roster() {
    let Some(doc) = document() else {
        return;
    };
    let (Some(list_el), Some(recip_el)) = (
        get_element("roster-list"),
        get_element("recipient-checkboxes"),
    ) else {
        return;
    };

    list_el.set_inner_html("");
    recip_el.set_inner_html("");

    let (roster, self_uid) = with_state(|st| {
        (
            st.roster.clone(),
            st.self_participant.as_ref().map(|p| p.user_id.clone()),
        )
    });

    for p in roster {
        if let Ok(chip) = doc.create_element("span") {
            let class_name = if p.participant_kind == ParticipantKind::AiAgent {
                "participant-chip ai"
            } else {
                "participant-chip"
            };
            chip.set_class_name(class_name);
            let short_id: String = p.user_id.chars().take(8).collect();
            chip.set_text_content(Some(&format!(
                "{} [{:?}] ({short_id})",
                p.display_name, p.role
            )));
            let _ = list_el.append_child(&chip);
        }

        if self_uid.as_deref() != Some(p.user_id.as_str())
            && let Ok(label) = doc.create_element("label")
        {
            label.set_class_name("recip-option");
            if let Ok(cb_el) = doc.create_element("input")
                && let Ok(cb) = cb_el.dyn_into::<HtmlInputElement>()
            {
                cb.set_type("checkbox");
                cb.set_value(&p.user_id);
                cb.set_checked(true);
                let _ = label.append_child(&cb);
            }
            if let Ok(span) = doc.create_element("span") {
                span.set_text_content(Some(&format!("{} ({:?})", p.display_name, p.role)));
                let _ = label.append_child(&span);
            }
            let _ = recip_el.append_child(&label);
        }
    }
}

// ============================================================================
// WebSocket Signaling & Server Packet Dispatch (`web-sys::WebSocket`)
// ============================================================================

fn connect_signaling_websocket() {
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
    let (Some(join_tok), session_id) = with_state(|st| {
        (
            st.join_token.clone(),
            st.session_id.clone().unwrap_or_default(),
        )
    }) else {
        return;
    };

    with_state_mut(|st| {
        if let Some(old_ws) = st.ws.take() {
            let _ = old_ws.close();
        }
    });

    let url = format!("{proto}//{host}/ws/session?join_token={join_tok}");
    let Ok(ws) = WebSocket::new(&url) else {
        return;
    };

    let sid_clone = session_id;
    let onopen = Closure::<dyn FnMut()>::new(move || {
        append_system_notice(&format!(
            "Connected to WebRTC Signaling & Apache Iggy Event Hub for '{sid_clone}' (Rust/Wasm client)."
        ));
        initiate_webrtc_mesh(false);
    });
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |ev: MessageEvent| {
        if let Some(text) = ev.data().as_string()
            && let Ok(pkt) = serde_json::from_str::<ServerWsPacket>(&text)
        {
            handle_server_packet(pkt);
        }
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    with_state_mut(|st| {
        st.ws = Some(ws);
    });
}

fn handle_server_packet(pkt: ServerWsPacket) {
    match pkt {
        ServerWsPacket::RosterSync { roster } => {
            with_state_mut(|st| {
                st.roster = roster;
            });
            render_roster();
            initiate_webrtc_mesh(false);
            satya_refresh_metrics();
        }
        ServerWsPacket::PeerDisconnected {
            participant,
            reason,
        } => {
            let uid = participant.user_id.clone();
            with_state_mut(|st| {
                if let Some(peer) = st.peers.remove(&uid) {
                    if let Some(dc) = peer.dc {
                        dc.close();
                    }
                    peer.pc.close();
                }
            });
            if let Some(tile) = get_element(&format!("video-tile-{uid}")) {
                tile.remove();
            }
            append_system_notice(&format!(
                "{} ({:?}) disconnected from session ({}).",
                participant.display_name,
                participant.role,
                reason.unwrap_or_else(|| "left".to_string())
            ));
            satya_refresh_metrics();
        }
        ServerWsPacket::ChatEvent { event } => {
            render_chat_event(&event);
            satya_refresh_metrics();
        }
        ServerWsPacket::TtrpgMechanicsEvent { event } => {
            render_mechanics_event(&event);
            satya_refresh_metrics();
        }
        ServerWsPacket::WebrtcSignal {
            from_participant,
            signal_kind,
            payload,
        } => {
            spawn_local(async move {
                handle_webrtc_signal(from_participant, signal_kind, payload).await;
            });
        }
    }
}

/// Serializes a [`ClientWsPacket`] to JSON and sends it over the active session `WebSocket`.
pub fn send_ws_packet(pkt: &ClientWsPacket) {
    let Ok(json_str) = serde_json::to_string(pkt) else {
        return;
    };
    with_state(|st| {
        if let Some(ws) = &st.ws
            && ws.ready_state() == WebSocket::OPEN
        {
            let _ = ws.send_with_str(&json_str);
        }
    });
}

// ============================================================================
// WebRTC P2P Audio/Video + DataChannel Mesh (`web-sys::RtcPeerConnection`)
// ============================================================================

/// Requests camera and microphone media tracks via `navigator.mediaDevices.getUserMedia`,
/// attaches the stream to `#local-video`, starts WebAudio RMS prosody monitoring, and renegotiates peers.
#[wasm_bindgen]
pub fn satya_start_camera() {
    spawn_local(async move {
        let Some(win) = window() else {
            return;
        };
        let Ok(media_devices) = win.navigator().media_devices() else {
            append_system_notice(
                "MediaDevices unavailable; continuing in DataChannel & signaling mode.",
            );
            return;
        };

        let constraints = MediaStreamConstraints::new();
        constraints.set_audio(&JsValue::TRUE);
        constraints.set_video(&JsValue::TRUE);

        let Ok(promise) = media_devices.get_user_media_with_constraints(&constraints) else {
            append_system_notice("Camera/Mic access unavailable; continuing in DataChannel mode.");
            return;
        };

        match JsFuture::from(promise).await {
            Ok(stream_val) => {
                if let Ok(stream) = stream_val.dyn_into::<MediaStream>() {
                    if let Some(video_el) = get_element("local-video")
                        .and_then(|e| e.dyn_into::<HtmlVideoElement>().ok())
                    {
                        video_el.set_src_object(Some(&stream));
                    }
                    start_audio_prosody_monitor(&stream);
                    with_state_mut(|st| {
                        st.local_stream = Some(stream);
                    });
                    initiate_webrtc_mesh(true);
                }
            }
            Err(_) => {
                append_system_notice(
                    "Camera/Mic permission declined; continuing in DataChannel & signaling mode.",
                );
            }
        }
    });
}

/// Toggles the local microphone audio track mute state and emits a `TRACK_MUTE_CHANGE` WebRTC telemetry event.
#[wasm_bindgen]
pub fn satya_toggle_audio() {
    let (audio_muted, video_muted) = with_state_mut(|st| {
        st.audio_muted = !st.audio_muted;
        if let Some(stream) = &st.local_stream {
            let tracks = stream.get_audio_tracks();
            for i in 0..tracks.length() {
                if let Ok(t) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
                    t.set_enabled(!st.audio_muted);
                }
            }
        }
        (st.audio_muted, st.video_muted)
    });

    set_text(
        "btn-mute-audio",
        if audio_muted {
            "Unmute Mic"
        } else {
            "Mute Mic"
        },
    );
    send_ws_packet(&ClientWsPacket::WebrtcTelemetry {
        target_user_id: None,
        event_kind: WebRtcEventKind::TrackMuteChange,
        audio_muted,
        video_muted,
    });
}

/// Toggles the local camera video track mute state and emits a `TRACK_MUTE_CHANGE` WebRTC telemetry event.
#[wasm_bindgen]
pub fn satya_toggle_video() {
    let (audio_muted, video_muted) = with_state_mut(|st| {
        st.video_muted = !st.video_muted;
        if let Some(stream) = &st.local_stream {
            let tracks = stream.get_video_tracks();
            for i in 0..tracks.length() {
                if let Ok(t) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
                    t.set_enabled(!st.video_muted);
                }
            }
        }
        (st.audio_muted, st.video_muted)
    });

    set_text(
        "btn-mute-video",
        if video_muted {
            "Unmute Video"
        } else {
            "Mute Video"
        },
    );
    send_ws_packet(&ClientWsPacket::WebrtcTelemetry {
        target_user_id: None,
        event_kind: WebRtcEventKind::TrackMuteChange,
        audio_muted,
        video_muted,
    });
}

fn start_audio_prosody_monitor(stream: &MediaStream) {
    let Ok(ctx) = AudioContext::new() else {
        return;
    };
    let Ok(source) = ctx.create_media_stream_source(stream) else {
        return;
    };
    let Ok(analyser) = ctx.create_analyser() else {
        return;
    };
    analyser.set_fft_size(512);
    let _ = source.connect_with_audio_node(&analyser);

    let Some(win) = window() else {
        return;
    };
    let analyser_rc = Rc::new(analyser);
    let tick = Closure::<dyn FnMut()>::new(move || {
        sample_analyser_rms(&analyser_rc);
    });
    let _ = win
        .set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), 250);
    tick.forget();
}

fn sample_analyser_rms(analyser: &AnalyserNode) {
    let len = analyser.fft_size();
    let buf = Float32Array::new_with_length(len);
    let mut vec_buf = vec![0.0f32; len as usize];
    analyser.get_float_time_domain_data(&mut vec_buf);
    buf.copy_from(&vec_buf);

    let sum_sq: f32 = vec_buf.iter().map(|v| v * v).sum();
    let rms = (sum_sq / (vec_buf.len().max(1) as f32)).sqrt();

    with_state_mut(|st| {
        st.rms_sum += rms;
        st.rms_samples += 1;
        if rms > st.rms_peak {
            st.rms_peak = rms;
        }
    });
    set_text("vad-energy-readout", &format!("RMS: {rms:.3}"));
}

/// Emits an `INTERACTION_DYNAMICS` prosody and voice-activity turn event to Apache Iggy using accumulated RMS energy metrics.
#[wasm_bindgen]
pub fn satya_emit_prosody_sample() {
    let (mean, peak) = with_state_mut(|st| {
        let m = if st.rms_samples > 0 {
            st.rms_sum / (st.rms_samples as f32)
        } else {
            0.18
        };
        let p = if st.rms_peak > 0.0 { st.rms_peak } else { 0.44 };
        st.rms_sum = 0.0;
        st.rms_samples = 0;
        st.rms_peak = 0.0;
        (m, p)
    });

    send_ws_packet(&ClientWsPacket::InteractionDynamics {
        turn_type: InteractionTurnType::SpeechEnd,
        duration_ms: 2450,
        rms_energy_mean: mean,
        rms_energy_peak: peak,
        pitch_hz_estimate: Some(148.0),
        preceding_silence_ms: 620,
    });
    append_system_notice(&format!(
        "Logged InteractionDynamicsEvent (RMS mean={mean:.2}, peak={peak:.2}) to Apache Iggy."
    ));
    satya_refresh_metrics();
}

fn initiate_webrtc_mesh(renegotiate: bool) {
    let (Some(self_part), roster) =
        with_state(|st| (st.self_participant.clone(), st.roster.clone()))
    else {
        return;
    };

    for p in roster {
        if p.user_id != self_part.user_id && p.participant_kind == ParticipantKind::Human {
            if renegotiate {
                with_state_mut(|st| {
                    if let Some(old) = st.peers.remove(&p.user_id) {
                        old.pc.close();
                    }
                });
            }
            let should_initiate = self_part.user_id < p.user_id;
            let _ = ensure_peer_connection(&p.user_id, &p.display_name, should_initiate);
        }
    }
}

fn ensure_peer_connection(
    remote_user_id: &str,
    remote_display_name: &str,
    is_initiator: bool,
) -> Option<RtcPeerConnection> {
    if let Some(existing) = with_state(|st| st.peers.get(remote_user_id).map(|e| e.pc.clone())) {
        return Some(existing);
    }

    let config = RtcConfiguration::new();
    let ice_servers = Array::new();
    let server_obj = js_sys::Object::new();
    let _ = Reflect::set(
        &server_obj,
        &JsValue::from_str("urls"),
        &JsValue::from_str("stun:stun.l.google.com:19302"),
    );
    ice_servers.push(&server_obj);
    config.set_ice_servers(&ice_servers);

    let pc = RtcPeerConnection::new_with_configuration(&config).ok()?;

    with_state(|st| {
        if let Some(stream) = &st.local_stream {
            let tracks = stream.get_tracks();
            for i in 0..tracks.length() {
                if let Ok(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
                    let _ = pc.add_track_0(&track, stream);
                }
            }
        }
    });

    let uid_ice = remote_user_id.to_string();
    let onice = Closure::<dyn FnMut(RtcPeerConnectionIceEvent)>::new(
        move |ev: RtcPeerConnectionIceEvent| {
            if let Some(cand) = ev.candidate()
                && let Ok(json_str) = js_sys::JSON::stringify(&cand.to_json())
                && let Some(payload) = json_str.as_string()
            {
                send_ws_packet(&ClientWsPacket::WebrtcSignal {
                    target_user_id: uid_ice.clone(),
                    signal_kind: WebRtcEventKind::IceCandidate,
                    payload,
                });
            }
        },
    );
    pc.set_onicecandidate(Some(onice.as_ref().unchecked_ref()));
    onice.forget();

    let uid_track = remote_user_id.to_string();
    let dname_track = remote_display_name.to_string();
    let ontrack = Closure::<dyn FnMut(RtcTrackEvent)>::new(move |ev: RtcTrackEvent| {
        let tile_id = format!("video-tile-{uid_track}");
        if get_element(&tile_id).is_none()
            && let (Some(doc), Some(grid)) = (document(), get_element("video-grid"))
            && let Ok(tile) = doc.create_element("div")
        {
            tile.set_id(&tile_id);
            tile.set_class_name("video-tile");
            tile.set_inner_html(&format!(
                r#"<video id="remote-video-{uid_track}" autoplay playsinline></video>
                <div class="tile-overlay">
                    <span class="role-tag">{}</span>
                    <span class="telemetry-mini">P2P WebRTC</span>
                </div>"#,
                escape_html(&dname_track)
            ));
            let _ = grid.append_child(&tile);
        }
        if let Some(vid) = get_element(&format!("remote-video-{uid_track}"))
            .and_then(|e| e.dyn_into::<HtmlVideoElement>().ok())
        {
            let streams = ev.streams();
            if streams.length() > 0
                && let Ok(stream) = streams.get(0).dyn_into::<MediaStream>()
            {
                vid.set_src_object(Some(&stream));
            }
        }
    });
    pc.set_ontrack(Some(ontrack.as_ref().unchecked_ref()));
    ontrack.forget();

    let mut dc_opt = None;
    if is_initiator {
        let dc = pc.create_data_channel("satya-chat-dc");
        setup_data_channel(&dc, remote_user_id);
        dc_opt = Some(dc);

        let pc_offer = pc.clone();
        let uid_offer = remote_user_id.to_string();
        spawn_local(async move {
            if let Ok(offer_val) = JsFuture::from(pc_offer.create_offer()).await {
                let sdp = Reflect::get(&offer_val, &JsValue::from_str("sdp"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
                desc.set_sdp(&sdp);
                let _ = JsFuture::from(pc_offer.set_local_description(&desc)).await;
                if let Ok(json_str) = js_sys::JSON::stringify(&offer_val)
                    && let Some(payload) = json_str.as_string()
                {
                    send_ws_packet(&ClientWsPacket::WebrtcSignal {
                        target_user_id: uid_offer,
                        signal_kind: WebRtcEventKind::SignalingOffer,
                        payload,
                    });
                }
            }
        });
    } else {
        let uid_dc = remote_user_id.to_string();
        let ondc =
            Closure::<dyn FnMut(RtcDataChannelEvent)>::new(move |ev: RtcDataChannelEvent| {
                let ch = ev.channel();
                setup_data_channel(&ch, &uid_dc);
            });
        pc.set_ondatachannel(Some(ondc.as_ref().unchecked_ref()));
        ondc.forget();
    }

    with_state_mut(|st| {
        st.peers.insert(
            remote_user_id.to_string(),
            PeerEntry {
                pc: pc.clone(),
                dc: dc_opt,
            },
        );
    });

    Some(pc)
}

fn setup_data_channel(dc: &RtcDataChannel, remote_user_id: &str) {
    let uid = remote_user_id.to_string();
    let onopen = Closure::<dyn FnMut()>::new(move || {
        let (audio_muted, video_muted) = with_state(|st| (st.audio_muted, st.video_muted));
        send_ws_packet(&ClientWsPacket::WebrtcTelemetry {
            target_user_id: Some(uid.clone()),
            event_kind: WebRtcEventKind::DataChannelOpen,
            audio_muted,
            video_muted,
        });
    });
    dc.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();
}

async fn handle_webrtc_signal(
    from_participant: Participant,
    signal_kind: WebRtcEventKind,
    payload: String,
) {
    let Some(pc) = ensure_peer_connection(
        &from_participant.user_id,
        &from_participant.display_name,
        false,
    ) else {
        return;
    };

    let Ok(parsed_js) = js_sys::JSON::parse(&payload) else {
        return;
    };

    match signal_kind {
        WebRtcEventKind::SignalingOffer => {
            let sdp = Reflect::get(&parsed_js, &JsValue::from_str("sdp"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
            remote_desc.set_sdp(&sdp);
            let _ = JsFuture::from(pc.set_remote_description(&remote_desc)).await;

            if let Ok(ans_val) = JsFuture::from(pc.create_answer()).await {
                let ans_sdp = Reflect::get(&ans_val, &JsValue::from_str("sdp"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let local_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
                local_desc.set_sdp(&ans_sdp);
                let _ = JsFuture::from(pc.set_local_description(&local_desc)).await;

                if let Ok(json_str) = js_sys::JSON::stringify(&ans_val)
                    && let Some(ans_payload) = json_str.as_string()
                {
                    send_ws_packet(&ClientWsPacket::WebrtcSignal {
                        target_user_id: from_participant.user_id,
                        signal_kind: WebRtcEventKind::SignalingAnswer,
                        payload: ans_payload,
                    });
                }
            }
        }
        WebRtcEventKind::SignalingAnswer => {
            let sdp = Reflect::get(&parsed_js, &JsValue::from_str("sdp"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            remote_desc.set_sdp(&sdp);
            let _ = JsFuture::from(pc.set_remote_description(&remote_desc)).await;
        }
        WebRtcEventKind::IceCandidate => {
            let cand_init: RtcIceCandidateInit = parsed_js.unchecked_into();
            if let Ok(cand) = RtcIceCandidate::new(&cand_init) {
                let _ =
                    JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&cand)))
                        .await;
            }
        }
        _ => {}
    }
}

// ============================================================================
// Targeted Chat & Satya TTRPG d20 Task Resolution
// ============================================================================

/// Shows or hides the private recipient checkbox picker based on `#chat-visibility`.
#[wasm_bindgen]
pub fn satya_visibility_changed() {
    let vis = select_value("chat-visibility");
    set_display(
        "private-recipient-picker",
        if vis == "PRIVATE" { "block" } else { "none" },
    );
}

/// Tracks chat composition start time on keydown and sends the message when `Enter` is pressed.
#[wasm_bindgen]
pub fn satya_chat_keydown(ev: KeyboardEvent) {
    with_state_mut(|st| {
        if st.typing_started_at_ms.is_none() {
            st.typing_started_at_ms = Some(js_sys::Date::now());
        }
    });
    if ev.key() == "Enter" {
        satya_send_chat();
    }
}

/// Sends a public, private, `GM_ONLY`, or `HOST_ONLY` chat message over the signaling WebSocket.
#[wasm_bindgen]
pub fn satya_send_chat() {
    let body = input_value("chat-input").trim().to_string();
    if body.is_empty() {
        return;
    }

    let visibility = parse_visibility(&select_value("chat-visibility"));
    let channel_context = parse_channel_context(&select_value("chat-context"));
    let composition_time_ms = with_state_mut(|st| {
        let elapsed = st
            .typing_started_at_ms
            .map(|t0| (js_sys::Date::now() - t0).max(1.0) as i64)
            .unwrap_or(450);
        st.typing_started_at_ms = None;
        elapsed
    });

    let mut recipient_user_ids = Vec::new();
    if visibility == MessageVisibility::Private
        && let Some(recip_box) = get_element("recipient-checkboxes")
    {
        let children = recip_box.children();
        for i in 0..children.length() {
            if let Some(label) = children.item(i)
                && let Some(first) = label.first_element_child()
                && let Ok(cb) = first.dyn_into::<HtmlInputElement>()
                && cb.checked()
            {
                recipient_user_ids.push(cb.value());
            }
        }
    }

    send_ws_packet(&ClientWsPacket::SendChat {
        visibility,
        recipient_user_ids,
        channel_context,
        body,
        composition_time_ms,
    });
    set_input_value("chat-input", "");
}

/// Evaluates and emits a Satya d20 roll-under TTRPG task resolution event (`ROLL_TTRPG_TASK`) over the WebSocket.
#[wasm_bindgen]
pub fn satya_roll_task() {
    let skill_raw = input_value("ttrpg-skill").trim().to_string();
    let skill_name = if skill_raw.is_empty() {
        "Athletics".to_string()
    } else {
        skill_raw
    };
    let attribute_tn = input_value("ttrpg-tn").parse::<i32>().unwrap_or(12);
    let pool_size = input_value("ttrpg-pool").parse::<usize>().unwrap_or(3);
    let difficulty_rating = input_value("ttrpg-diff").parse::<i32>().unwrap_or(2);
    let combat_stance = parse_combat_stance(&select_value("ttrpg-stance"));
    let fatigue_level = input_value("ttrpg-fatigue").parse::<i32>().unwrap_or(0);

    send_ws_packet(&ClientWsPacket::RollTtrpgTask {
        skill_name,
        attribute_tn,
        pool_size,
        difficulty_rating,
        combat_stance,
        fatigue_level,
    });
}

fn render_chat_event(ev: &ChatMessageEvent) {
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

    div.set_inner_html(&format!(
        r#"<div class="chat-meta">
            <span><strong>{}</strong> ({:?}) · {:?}</span>
            <span>{} · schema v{}</span>
        </div>
        <div>{}</div>"#,
        escape_html(&ev.sender.display_name),
        ev.sender.role,
        ev.channel_context,
        escape_html(&vis_label),
        ev.metadata.schema_version,
        escape_html(&ev.body)
    ));
    let _ = log.append_child(&div);
    log.set_scroll_top(log.scroll_height());
}

fn render_mechanics_event(ev: &TtrpgMechanicsEvent) {
    let (Some(doc), Some(log)) = (document(), get_element("chat-log")) else {
        return;
    };
    let Ok(div) = doc.create_element("div") else {
        return;
    };
    div.set_class_name("chat-msg mechanics");

    let rolls: Vec<String> = ev.dice_rolled_d20.iter().map(|r| r.to_string()).collect();
    let stance_info = match ev.combat_stance {
        Some(st) => format!(
            " · Stance: {st:?} (Action Cost: {})",
            ev.action_count_cost.unwrap_or(10)
        ),
        None => String::new(),
    };

    div.set_inner_html(&format!(
        r#"<div class="chat-meta">
            <span><strong>[SATYA MECHANICS] {}</strong> · {}</span>
            <span>Outcome: {:?}</span>
        </div>
        <div>Rolled d20s: [{}] vs TN {} (Diff {}) =&gt; <strong>{} successes</strong>{}</div>"#,
        escape_html(&ev.actor.display_name),
        escape_html(ev.skill_name.as_deref().unwrap_or("Action")),
        ev.outcome_tier,
        rolls.join(", "),
        ev.attribute_tn.unwrap_or(10),
        ev.difficulty_rating.unwrap_or(1),
        ev.successes_counted,
        stance_info
    ));
    let _ = log.append_child(&div);
    log.set_scroll_top(log.scroll_height());
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
    div.set_inner_html(&format!(
        r#"<div class="chat-meta"><span>SYSTEM</span><span>RUST/WASM</span></div><div>{}</div>"#,
        escape_html(text)
    ));
    let _ = log.append_child(&div);
    log.set_scroll_top(log.scroll_height());
}

// ============================================================================
// Apache Iggy Topic & Avro Schema Inspector
// ============================================================================

/// Fetches live Apache Iggy topic metrics from `GET /api/storage/metrics` and updates `#topic-metrics-table`.
#[wasm_bindgen]
pub fn satya_refresh_metrics() {
    spawn_local(async move {
        if let Ok(resp) =
            fetch_json::<(), StorageMetricsViewResponse>("GET", "/api/storage/metrics", None, None)
                .await
        {
            render_metrics_table(&resp.topics);
        }
    });
}

/// Triggers an immediate flush of all buffered topic events into `.avro` container batches via `POST /api/storage/flush`.
#[wasm_bindgen]
pub fn satya_flush_batches() {
    spawn_local(async move {
        if let Ok(val) =
            fetch_json::<(), serde_json::Value>("POST", "/api/storage/flush", None, None).await
        {
            let count = val
                .get("flushed_files")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            append_system_notice(&format!(
                "Flushed {count} offset-tracked Avro container batch(es) for PyLance ingestion."
            ));
            satya_refresh_metrics();
        }
    });
}

/// Renders an HTML `<table>` string summarizing per-topic Apache Iggy and Avro schema metrics.
pub fn format_metrics_table_html(topics: &[TopicMetricsView]) -> String {
    let rows: String = topics
        .iter()
        .map(|t| {
            let fp_short: String = t.active_schema_fingerprint_hex.chars().take(10).collect();
            let last_off = t
                .last_flushed_offset
                .map(|o| o.to_string())
                .unwrap_or_else(|| "-".to_string());
            format!(
                r#"<tr>
                    <td><strong>{}</strong></td>
                    <td><span class="mono">v{} ({fp_short}...)</span></td>
                    <td>{} (<span class="mono">{}</span>)</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{} (last offset: {last_off})</td>
                </tr>"#,
                escape_html(&t.topic),
                t.active_schema_version,
                escape_html(&t.storage_backend.to_uppercase()),
                escape_html(&t.destination_uri),
                t.total_messages_ingested,
                t.buffered_messages,
                t.flushed_batches_count,
            )
        })
        .collect();

    format!(
        r#"<table class="metrics-table">
            <thead>
                <tr>
                    <th>Apache Iggy Topic</th>
                    <th>Schema Ver &amp; CRC-64</th>
                    <th>Configurable Storage Sink</th>
                    <th>Ingested Events</th>
                    <th>Buffered</th>
                    <th>Flushed .avro Batches</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>"#
    )
}

fn render_metrics_table(topics: &[TopicMetricsView]) {
    if let Some(wrap) = get_element("topic-metrics-table") {
        wrap.set_inner_html(&format_metrics_table_html(topics));
    }
}
