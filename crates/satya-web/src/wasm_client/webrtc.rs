//! WebSocket signaling client (`/ws/session`), P2P WebRTC Audio/Video + DataChannel mesh,
//! WebAudio RMS prosody telemetry, and Floating/Column Webcam frame controls.

use crate::wasm_client::auth_session::render_roster;
use crate::wasm_client::chat_and_ttrpg::{
    append_system_notice, render_chat_event, render_mechanics_event,
};
use crate::wasm_client::dom::{document, escape_html, get_element, set_class, set_text, window};
use crate::wasm_client::notebook::{
    render_gm_document_added, render_map_updated, render_wgpu_dice_roll,
};
use crate::wasm_client::state::{PeerEntry, with_state, with_state_mut};
use crate::wasm_client::telemetry::satya_refresh_metrics;
use js_sys::{Array, Float32Array, Reflect};
use satya_schema::{
    ClientWsPacket, InteractionTurnType, Participant, ParticipantKind, ServerWsPacket,
    WebRtcEventKind,
};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    AnalyserNode, AudioContext, Element, HtmlElement, HtmlVideoElement, MediaStream,
    MediaStreamConstraints, MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelEvent,
    RtcIceCandidate, RtcIceCandidateInit, RtcOfferOptions, RtcPeerConnection,
    RtcPeerConnectionIceEvent, RtcSdpType, RtcSessionDescriptionInit, RtcTrackEvent, WebSocket,
};

fn sync_drawer_rail_and_aria(
    collapsed: bool,
    rail_id: &str,
    toggle_btn_id: &str,
    expand_btn_id: &str,
    menu_btn_id: &str,
    title: &str,
) {
    if let Some(rail) = get_element(rail_id).and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        let _ = rail
            .style()
            .set_property("display", if collapsed { "flex" } else { "none" });
    }
    let expanded_str = if collapsed { "false" } else { "true" };
    for btn_id in [toggle_btn_id, expand_btn_id, menu_btn_id] {
        if let Some(btn) = get_element(btn_id) {
            let _ = btn.set_attribute("aria-expanded", expanded_str);
        }
    }
    let toggle_label = if collapsed {
        format!("◂ {title}")
    } else {
        "Collapse ▸".to_string()
    };
    set_text(toggle_btn_id, &toggle_label);
    let menu_label = if collapsed {
        format!("Show {title}")
    } else {
        format!("Hide {title}")
    };
    set_text(menu_btn_id, &menu_label);
}

fn sync_center_drawer_card(card_id: &str, base_class: &str, collapsed: bool) {
    if let Some(card) = get_element(card_id).and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        if collapsed {
            card.set_class_name(&format!("{base_class} is-collapsed"));
            let _ = card.set_attribute("inert", "");
            let _ = card.style().set_property("display", "none");
        } else {
            card.set_class_name(base_class);
            let _ = card.remove_attribute("inert");
            let _ = card.style().set_property("display", "flex");
        }
    }
}

pub(crate) fn sync_workspace_drawers_dom() {
    let (is_floating, webcam_collapsed, chat_collapsed, ttrpg_collapsed) = with_state(|st| {
        (
            st.webcam_floating,
            st.webcam_collapsed,
            st.chat_collapsed,
            st.ttrpg_collapsed,
        )
    });

    if let Some(card) = get_element("webrtc-card").and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        if webcam_collapsed {
            card.set_class_name("card webrtc-card is-collapsed");
            let _ = card.set_attribute("inert", "");
            let _ = card.style().set_property("display", "none");
        } else if is_floating {
            card.set_class_name("card webrtc-card is-floating");
            let _ = card.remove_attribute("inert");
            let _ = card.style().set_property("display", "flex");
            let _ = card.style().set_property("right", "24px");
            let _ = card.style().set_property("top", "78px");
            let _ = card.style().remove_property("left");
        } else {
            card.set_class_name("card webrtc-card");
            let _ = card.remove_attribute("inert");
            let _ = card.style().set_property("display", "flex");
            let _ = card.style().remove_property("left");
            let _ = card.style().remove_property("top");
            let _ = card.style().remove_property("right");
        }
    }

    let any_rail_collapsed = webcam_collapsed || chat_collapsed || ttrpg_collapsed;
    let both_center_collapsed = chat_collapsed && ttrpg_collapsed;

    if let Some(col) = get_element("webrtc-dock-column") {
        col.set_class_name(if webcam_collapsed || (is_floating && any_rail_collapsed) {
            "col-webcams is-collapsed"
        } else {
            "col-webcams"
        });
    }

    if let Some(rails_dock) =
        get_element("right-drawer-rails").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = rails_dock
            .style()
            .set_property("display", if any_rail_collapsed { "flex" } else { "none" });
    }

    sync_center_drawer_card("chat-card", "card chat-card", chat_collapsed);
    sync_center_drawer_card("ttrpg-drawer", "card ttrpg-card", ttrpg_collapsed);

    if let Some(center_col) =
        get_element("center-column").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        if both_center_collapsed {
            center_col.set_class_name("col-center is-collapsed");
            let _ = center_col.style().set_property("display", "none");
        } else {
            center_col.set_class_name("col-center");
            let _ = center_col.style().set_property("display", "flex");
        }
    }

    if let Some(ws_panel) = get_element("workspace-panel") {
        let mut classes = String::from("workspace-columns");
        if webcam_collapsed || (is_floating && any_rail_collapsed) {
            classes.push_str(" webcam-collapsed");
        } else if is_floating {
            classes.push_str(" webcam-undocked");
        }
        if both_center_collapsed {
            classes.push_str(" center-collapsed");
        } else if ttrpg_collapsed {
            classes.push_str(" ttrpg-collapsed");
        } else if chat_collapsed {
            classes.push_str(" chat-collapsed");
        }
        ws_panel.set_class_name(&classes);
    }

    sync_drawer_rail_and_aria(
        webcam_collapsed,
        "webcam-drawer-rail",
        "btn-toggle-webcam-drawer",
        "btn-expand-webcam-drawer",
        "btn-menu-toggle-webcam-pane",
        "Web Cams",
    );
    set_text(
        "btn-toggle-webcam-float",
        if is_floating { "Dock" } else { "Float" },
    );

    sync_drawer_rail_and_aria(
        chat_collapsed,
        "chat-drawer-rail",
        "btn-toggle-chat-drawer",
        "btn-expand-chat-drawer",
        "btn-menu-toggle-chat-pane",
        "Session Chat",
    );

    sync_drawer_rail_and_aria(
        ttrpg_collapsed,
        "ttrpg-drawer-rail",
        "btn-toggle-ttrpg-drawer",
        "btn-expand-ttrpg-drawer",
        "btn-menu-toggle-ttrpg-pane",
        "Task Roller",
    );
}

/// Toggles the Web Cams column (`#webrtc-dock-column`) between an expanded drawer and a
/// collapsed right-edge vertical rail tab (`#webcam-drawer-rail`) to reclaim workspace width.
#[wasm_bindgen]
pub fn satya_toggle_webcam_drawer() {
    with_state_mut(|st| {
        st.webcam_collapsed = !st.webcam_collapsed;
        st.webcam_drag_offset = None;
    });
    sync_workspace_drawers_dom();
}

/// Toggles the Session Chat container (`#chat-card`) in `#center-column` between expanded
/// and a collapsed drawer rail tab (`#chat-drawer-rail`) to reclaim vertical and horizontal room.
#[wasm_bindgen]
pub fn satya_toggle_chat_drawer() {
    with_state_mut(|st| {
        st.chat_collapsed = !st.chat_collapsed;
    });
    sync_workspace_drawers_dom();
}

/// Toggles the Task Resolution & Action Pool container (`#ttrpg-drawer`) in `#center-column`
/// between expanded and a collapsed drawer rail tab (`#ttrpg-drawer-rail`) to reclaim vertical and horizontal room.
#[wasm_bindgen]
pub fn satya_toggle_ttrpg_drawer() {
    with_state_mut(|st| {
        st.ttrpg_collapsed = !st.ttrpg_collapsed;
    });
    sync_workspace_drawers_dom();
}

/// Toggles the Web Cams container (`#webrtc-card`) between a docked right-column panel
/// and a floating, draggable overlay frame that can be re-attached back into the column.
#[wasm_bindgen]
pub fn satya_toggle_webcam_float() {
    with_state_mut(|st| {
        st.webcam_floating = !st.webcam_floating;
        st.webcam_collapsed = false;
        st.webcam_drag_offset = None;
    });
    sync_workspace_drawers_dom();
}

/// Toggles the vertical Web Cams grid (`#video-grid`) between 1 column (`cols-1`) and 2 columns (`cols-2`),
/// strictly capping the webcam layout at a maximum of 2 columns.
#[wasm_bindgen]
pub fn satya_toggle_webcam_columns() {
    let cols = with_state_mut(|st| {
        st.webcam_columns = if st.webcam_columns == 1 { 2 } else { 1 };
        st.webcam_columns
    });

    set_class(
        "video-grid",
        if cols == 2 {
            "video-grid cols-2"
        } else {
            "video-grid cols-1"
        },
    );
    set_text(
        "btn-toggle-webcam-cols",
        if cols == 2 { "1 Col" } else { "2 Cols" },
    );
}

/// Begins dragging the floating Web Cams frame (`#webrtc-card.is-floating`) when the user presses
/// on `#webrtc-drag-header`.
#[wasm_bindgen]
pub fn satya_start_webcam_drag(ev: web_sys::Event) {
    if !with_state(|st| st.webcam_floating) {
        return;
    }
    if let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) {
        let tag = target.tag_name().to_uppercase();
        if tag == "BUTTON" || target.closest("button").ok().flatten().is_some() {
            return;
        }
    }
    let Some(client_x) = Reflect::get(&ev, &JsValue::from_str("clientX"))
        .ok()
        .and_then(|v| v.as_f64())
    else {
        return;
    };
    let Some(client_y) = Reflect::get(&ev, &JsValue::from_str("clientY"))
        .ok()
        .and_then(|v| v.as_f64())
    else {
        return;
    };
    let Some(card) = get_element("webrtc-card").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };

    let left = card.offset_left() as f64;
    let top = card.offset_top() as f64;
    with_state_mut(|st| {
        st.webcam_drag_offset = Some((client_x - left, client_y - top));
    });
}

pub(crate) fn handle_webcam_drag_move(ev: &web_sys::Event) {
    let Some((off_x, off_y)) = with_state(|st| {
        if st.webcam_floating {
            st.webcam_drag_offset
        } else {
            None
        }
    }) else {
        return;
    };
    let Some(client_x) = Reflect::get(ev, &JsValue::from_str("clientX"))
        .ok()
        .and_then(|v| v.as_f64())
    else {
        return;
    };
    let Some(client_y) = Reflect::get(ev, &JsValue::from_str("clientY"))
        .ok()
        .and_then(|v| v.as_f64())
    else {
        return;
    };
    if let Some(card) = get_element("webrtc-card").and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        let new_left = (client_x - off_x).max(8.0);
        let new_top = (client_y - off_y).max(8.0);
        let _ = card.style().remove_property("right");
        let _ = card
            .style()
            .set_property("left", &format!("{new_left:.0}px"));
        let _ = card.style().set_property("top", &format!("{new_top:.0}px"));
    }
}

pub(crate) fn connect_signaling_websocket() {
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
        ServerWsPacket::MapUpdated { map } => {
            render_map_updated(&map);
            append_system_notice(&format!(
                "Tactical map updated: {} ({}x{}, {} tokens)",
                map.map_id,
                map.grid_width,
                map.grid_height,
                map.tokens.len()
            ));
            satya_refresh_metrics();
        }
        ServerWsPacket::WgpuDiceRoll { actor, spec } => {
            render_wgpu_dice_roll(&actor, &spec);
            append_system_notice(&format!(
                "{} triggered 3D dice roll ({} -> {:?})",
                actor.display_name, spec.dice_notation, spec.outcomes
            ));
            satya_refresh_metrics();
        }
        ServerWsPacket::GmDocumentAdded { document } => {
            render_gm_document_added(&document);
            append_system_notice(&format!(
                "GM_ONLY document '{}' ({:?}) submitted by {} ({} chars).",
                document.title,
                document.source_kind,
                document.uploader_display_name,
                document.char_count
            ));
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

        let mut stream_res = match media_devices.get_user_media_with_constraints(&constraints) {
            Ok(promise) => JsFuture::from(promise).await,
            Err(e) => Err(e),
        };

        if stream_res.is_err() {
            let video_only = MediaStreamConstraints::new();
            video_only.set_audio(&JsValue::FALSE);
            video_only.set_video(&JsValue::TRUE);
            if let Ok(promise) = media_devices.get_user_media_with_constraints(&video_only) {
                stream_res = JsFuture::from(promise).await;
            }
        }

        match stream_res {
            Ok(stream_val) => {
                if let Ok(stream) = stream_val.dyn_into::<MediaStream>() {
                    if let Some(video_el) = get_element("local-video")
                        .and_then(|e| e.dyn_into::<HtmlVideoElement>().ok())
                    {
                        video_el.set_src_object(Some(&stream));
                        let _ = video_el.play();
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

/// Determines whether the local participant (`self_user_id`) should act as the WebRTC SDP offer
/// initiator for `remote_user_id`.
///
/// During initial mesh formation (`renegotiate == false`), lexicographical ordering breaks ties so
/// only one side sends an offer. During renegotiation (`renegotiate == true`, such as when the
/// local user enables their camera/microphone), the local peer always initiates a new offer.
pub fn should_initiate_peer_connection(
    self_user_id: &str,
    remote_user_id: &str,
    renegotiate: bool,
) -> bool {
    if self_user_id == remote_user_id {
        return false;
    }
    renegotiate || (self_user_id < remote_user_id)
}

fn close_peer_connection(remote_user_id: &str) {
    with_state_mut(|st| {
        if let Some(old) = st.peers.remove(remote_user_id) {
            if let Some(dc) = old.dc {
                dc.close();
            }
            old.pc.set_onicecandidate(None);
            old.pc.set_ontrack(None);
            old.pc.set_ondatachannel(None);
            old.pc.close();
        }
    });
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
                close_peer_connection(&p.user_id);
            }
            if should_initiate_peer_connection(&self_part.user_id, &p.user_id, renegotiate) {
                let _ = ensure_peer_connection(&p.user_id, &p.display_name, true);
            }
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
            } else if let Some(fallback_stream) =
                vid.src_object().or_else(|| MediaStream::new().ok())
            {
                fallback_stream.add_track(&ev.track());
                vid.set_src_object(Some(&fallback_stream));
            }
            if let Ok(play_promise) = vid.play() {
                let vid_fallback = vid.clone();
                spawn_local(async move {
                    if JsFuture::from(play_promise).await.is_err() {
                        vid_fallback.set_muted(true);
                        let _ = vid_fallback.play();
                    }
                });
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
            let offer_opts = RtcOfferOptions::new();
            let _ = Reflect::set(
                &offer_opts,
                &JsValue::from_str("offerToReceiveAudio"),
                &JsValue::TRUE,
            );
            let _ = Reflect::set(
                &offer_opts,
                &JsValue::from_str("offerToReceiveVideo"),
                &JsValue::TRUE,
            );
            if let Ok(offer_val) =
                JsFuture::from(pc_offer.create_offer_with_rtc_offer_options(&offer_opts)).await
            {
                let sdp = Reflect::get(&offer_val, &JsValue::from_str("sdp"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
                desc.set_sdp(&sdp);
                let set_local_promise = pc_offer.set_local_description(&desc);
                if let Ok(json_str) = js_sys::JSON::stringify(&offer_val)
                    && let Some(payload) = json_str.as_string()
                {
                    send_ws_packet(&ClientWsPacket::WebrtcSignal {
                        target_user_id: uid_offer,
                        signal_kind: WebRtcEventKind::SignalingOffer,
                        payload,
                    });
                }
                let _ = JsFuture::from(set_local_promise).await;
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
                remote_desc_set: false,
                pending_ice_candidates: Vec::new(),
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

async fn flush_pending_ice_candidates(remote_user_id: &str, pc: &RtcPeerConnection) {
    let pending = with_state_mut(|st| {
        if let Some(entry) = st.peers.get_mut(remote_user_id) {
            entry.remote_desc_set = true;
            std::mem::take(&mut entry.pending_ice_candidates)
        } else {
            Vec::new()
        }
    });
    for cand_payload in pending {
        if let Ok(parsed_js) = js_sys::JSON::parse(&cand_payload) {
            let cand_init: RtcIceCandidateInit = parsed_js.unchecked_into();
            if let Ok(cand) = RtcIceCandidate::new(&cand_init) {
                let _ =
                    JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&cand)))
                        .await;
            }
        }
    }
}

async fn handle_webrtc_signal(
    from_participant: Participant,
    signal_kind: WebRtcEventKind,
    payload: String,
) {
    let remote_uid = from_participant.user_id.clone();
    let Ok(parsed_js) = js_sys::JSON::parse(&payload) else {
        return;
    };

    match signal_kind {
        WebRtcEventKind::SignalingOffer => {
            close_peer_connection(&remote_uid);
            let Some(pc) =
                ensure_peer_connection(&remote_uid, &from_participant.display_name, false)
            else {
                return;
            };

            let sdp = Reflect::get(&parsed_js, &JsValue::from_str("sdp"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
            remote_desc.set_sdp(&sdp);
            if JsFuture::from(pc.set_remote_description(&remote_desc))
                .await
                .is_err()
            {
                return;
            }
            flush_pending_ice_candidates(&remote_uid, &pc).await;

            if let Ok(ans_val) = JsFuture::from(pc.create_answer()).await {
                let ans_sdp = Reflect::get(&ans_val, &JsValue::from_str("sdp"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let local_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
                local_desc.set_sdp(&ans_sdp);
                let set_local_promise = pc.set_local_description(&local_desc);

                if let Ok(json_str) = js_sys::JSON::stringify(&ans_val)
                    && let Some(ans_payload) = json_str.as_string()
                {
                    send_ws_packet(&ClientWsPacket::WebrtcSignal {
                        target_user_id: remote_uid,
                        signal_kind: WebRtcEventKind::SignalingAnswer,
                        payload: ans_payload,
                    });
                }
                let _ = JsFuture::from(set_local_promise).await;
            }
        }
        WebRtcEventKind::SignalingAnswer => {
            let Some(pc) =
                ensure_peer_connection(&remote_uid, &from_participant.display_name, false)
            else {
                return;
            };
            let sdp = Reflect::get(&parsed_js, &JsValue::from_str("sdp"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            remote_desc.set_sdp(&sdp);
            if JsFuture::from(pc.set_remote_description(&remote_desc))
                .await
                .is_ok()
            {
                flush_pending_ice_candidates(&remote_uid, &pc).await;
            }
        }
        WebRtcEventKind::IceCandidate => {
            let (pc_opt, should_add_now) = with_state_mut(|st| {
                if let Some(entry) = st.peers.get_mut(&remote_uid) {
                    if entry.remote_desc_set {
                        (Some(entry.pc.clone()), true)
                    } else {
                        entry.pending_ice_candidates.push(payload.clone());
                        (None, false)
                    }
                } else {
                    (None, false)
                }
            });
            if should_add_now && let Some(pc) = pc_opt {
                let cand_init: RtcIceCandidateInit = parsed_js.unchecked_into();
                if let Ok(cand) = RtcIceCandidate::new(&cand_init) {
                    let _ = JsFuture::from(
                        pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&cand)),
                    )
                    .await;
                }
            }
        }
        _ => {}
    }
}
