//! WebSocket session handler (`/ws/session`), real-time chat routing, WebRTC signaling relay,
//! interaction dynamics ingestion, and live TTRPG task resolution.

use crate::{
    auth::JoinTokenClaims,
    gm_api::{
        should_trigger_ai_gm_for_chat, should_trigger_ai_gm_for_roll, trigger_ai_gm_chat_reply,
    },
    state::SignalingAppState,
};
use axum::{
    extract::{
        Query, State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use rand::Rng;
use satya_schema::{
    ChatMessageEvent, ClientWsPacket, IceCandidateType, InteractionDynamicsEvent,
    LifecycleEventType, MessageVisibility, ParticipantKind, SessionMode, SessionRole,
    TtrpgMechanicsEvent, WebRtcTelemetryEvent, subjects,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Query string parameters required when upgrading `/ws/session?join_token=...`.
#[derive(Debug, Deserialize)]
pub struct WsConnectQuery {
    /// Single-session join token returned by `/api/sessions/create` or `/api/sessions/join`.
    pub join_token: String,
}

/// Upgrades an authenticated HTTP request at `GET /ws/session?join_token=...` to a full-duplex WebSocket.
pub async fn ws_session_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsConnectQuery>,
    State(state): State<Arc<SignalingAppState>>,
) -> impl IntoResponse {
    let Some(claims) = state.auth.verify_join_token(&query.join_token).await else {
        return (StatusCode::UNAUTHORIZED, "Invalid join_token").into_response();
    };
    ws.on_upgrade(move |socket| handle_ws_client(socket, claims, state))
}

async fn handle_ws_client(
    socket: WebSocket,
    claims: JoinTokenClaims,
    state: Arc<SignalingAppState>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    let session_id = claims.session_id.clone();
    let session_mode = claims.session_mode;
    let self_participant = claims.participant.clone();
    let user_id = self_participant.user_id.clone();

    {
        let mut guard = state.active_sockets.write().await;
        guard
            .entry(session_id.clone())
            .or_default()
            .insert(user_id.clone(), tx);
    }

    // Broadcast updated roster to all connected peers in this session
    let roster = state.auth.get_session_roster(&session_id).await;
    let sync_msg = json!({
        "type": "ROSTER_SYNC",
        "roster": roster
    })
    .to_string();
    state.broadcast_to_session(&session_id, &sync_msg).await;

    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(WsMessage::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_rx.next().await {
        let WsMessage::Text(text) = msg else {
            continue;
        };
        let Ok(packet) = serde_json::from_str::<ClientWsPacket>(&text) else {
            continue;
        };

        let current_roster = state.auth.get_session_roster(&session_id).await;

        match packet {
            ClientWsPacket::SendChat {
                visibility,
                recipient_user_ids,
                channel_context,
                body,
                composition_time_ms,
            } => {
                let (recipients, delivery_ids) = SignalingAppState::resolve_message_targets(
                    &self_participant,
                    visibility,
                    &recipient_user_ids,
                    &current_roster,
                );

                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::CHAT_MESSAGE,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let word_count = body.split_whitespace().count() as i32;
                    let char_count = body.chars().count() as i32;
                    let chat_event = ChatMessageEvent {
                        metadata: meta,
                        message_id: Uuid::now_v7().to_string(),
                        sender: self_participant.clone(),
                        visibility,
                        recipients: recipients.clone(),
                        channel_context,
                        reply_to_message_id: None,
                        body: body.clone(),
                        char_count,
                        word_count,
                        composition_time_ms,
                        is_edited: false,
                    };

                    let _ = state.storage.log_chat_message(&chat_event).await;
                    let pkt_str = json!({
                        "type": "CHAT_EVENT",
                        "event": chat_event
                    })
                    .to_string();
                    state
                        .send_to_users(&session_id, &delivery_ids, &pkt_str)
                        .await;

                    if session_mode == SessionMode::Ttrpg
                        && self_participant.participant_kind == ParticipantKind::Human
                        && let Some(ai_gm) = current_roster
                            .iter()
                            .find(|p| p.role == SessionRole::AiGm)
                            .cloned()
                        && should_trigger_ai_gm_for_chat(
                            &current_roster,
                            &ai_gm,
                            visibility,
                            &recipient_user_ids,
                            channel_context,
                            &body,
                        )
                    {
                        trigger_ai_gm_chat_reply(
                            &state,
                            &session_id,
                            session_mode,
                            &ai_gm,
                            &self_participant,
                            &chat_event.message_id,
                            "CHAT",
                            visibility,
                            &body,
                            &current_roster,
                        )
                        .await;
                    }
                }
            }
            ClientWsPacket::WebrtcSignal {
                target_user_id,
                signal_kind,
                payload,
            } => {
                let remote_p = current_roster
                    .iter()
                    .find(|p| p.user_id == target_user_id)
                    .cloned();

                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::WEBRTC_TELEMETRY,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let tel_ev = WebRtcTelemetryEvent {
                        metadata: meta,
                        local_participant: self_participant.clone(),
                        remote_participant: remote_p,
                        event_kind: signal_kind,
                        ice_candidate_type: Some(IceCandidateType::Host),
                        rtt_ms: Some(4.2),
                        jitter_ms: Some(0.8),
                        packet_loss_rate: Some(0.0),
                        available_outgoing_bitrate_bps: Some(2_500_000),
                        audio_muted: false,
                        video_muted: false,
                    };
                    let _ = state.storage.log_webrtc_telemetry(&tel_ev).await;
                }

                let relay_json = json!({
                    "type": "WEBRTC_SIGNAL",
                    "from_participant": self_participant,
                    "signal_kind": signal_kind,
                    "payload": payload
                })
                .to_string();
                state
                    .send_to_users(&session_id, &[target_user_id], &relay_json)
                    .await;
            }
            ClientWsPacket::WebrtcTelemetry {
                target_user_id,
                event_kind,
                audio_muted,
                video_muted,
            } => {
                let remote_p = target_user_id
                    .and_then(|uid| current_roster.iter().find(|p| p.user_id == uid).cloned());
                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::WEBRTC_TELEMETRY,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let tel_ev = WebRtcTelemetryEvent {
                        metadata: meta,
                        local_participant: self_participant.clone(),
                        remote_participant: remote_p,
                        event_kind,
                        ice_candidate_type: Some(IceCandidateType::Host),
                        rtt_ms: Some(3.5),
                        jitter_ms: Some(0.5),
                        packet_loss_rate: Some(0.0),
                        available_outgoing_bitrate_bps: Some(3_000_000),
                        audio_muted,
                        video_muted,
                    };
                    let _ = state.storage.log_webrtc_telemetry(&tel_ev).await;
                }
            }
            ClientWsPacket::InteractionDynamics {
                turn_type,
                duration_ms,
                rms_energy_mean,
                rms_energy_peak,
                pitch_hz_estimate,
                preceding_silence_ms,
            } => {
                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::INTERACTION_DYNAMICS,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let dyn_ev = InteractionDynamicsEvent {
                        metadata: meta,
                        speaker: self_participant.clone(),
                        turn_type,
                        duration_ms,
                        rms_energy_mean,
                        rms_energy_peak,
                        pitch_hz_estimate,
                        overlapped_participants: Vec::new(),
                        preceding_silence_ms,
                        dominance_ratio: 1.0 / (current_roster.len().max(1) as f32),
                    };
                    let _ = state.storage.log_interaction_dynamics(&dyn_ev).await;
                }
            }
            ClientWsPacket::RollTtrpgTask {
                skill_name,
                attribute_tn,
                pool_size,
                difficulty_rating,
                combat_stance,
                fatigue_level,
            } => {
                let count = pool_size.clamp(1, 12);
                let rolls: Vec<i32> = {
                    let mut rng = rand::thread_rng();
                    (0..count).map(|_| rng.gen_range(1..=20)).collect()
                };

                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::TTRPG_MECHANICS,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let mech_ev = TtrpgMechanicsEvent::evaluate_satya_task(
                        meta,
                        self_participant.clone(),
                        Vec::new(),
                        skill_name.clone(),
                        attribute_tn,
                        difficulty_rating,
                        rolls,
                        combat_stance,
                        fatigue_level,
                        0.0,
                    );
                    let _ = state.storage.log_ttrpg_mechanics(&mech_ev).await;

                    let pkt_str = json!({
                        "type": "TTRPG_MECHANICS_EVENT",
                        "event": mech_ev
                    })
                    .to_string();
                    state.broadcast_to_session(&session_id, &pkt_str).await;

                    // Have AI GM narrate the task resolution outcome when in Autonomous mode (`TTRPG_AI_GM`)
                    if let Some(ai_gm) = current_roster
                        .iter()
                        .find(|p| p.role == SessionRole::AiGm)
                        .cloned()
                        && should_trigger_ai_gm_for_roll(&current_roster)
                    {
                        let narration = format!(
                            "[Satya Simulation] {} attempted '{}' (TN {}, Diff {}) -> {:?} ({} successes). Action Count Cost: {} ticks.",
                            self_participant.display_name,
                            skill_name,
                            attribute_tn,
                            difficulty_rating,
                            mech_ev.outcome_tier,
                            mech_ev.successes_counted,
                            mech_ev.action_count_cost.unwrap_or(10)
                        );
                        trigger_ai_gm_chat_reply(
                            &state,
                            &session_id,
                            session_mode,
                            &ai_gm,
                            &self_participant,
                            &mech_ev.metadata.event_id,
                            "MECHANICS",
                            MessageVisibility::Public,
                            &narration,
                            &current_roster,
                        )
                        .await;
                    }
                }
            }
            ClientWsPacket::LeaveSession { reason } => {
                let disc_reason =
                    reason.unwrap_or_else(|| "User disconnected from session".to_string());
                if let Some(summary) = state.auth.leave_session(&session_id, &user_id).await {
                    state
                        .process_session_disconnect(
                            &summary,
                            LifecycleEventType::ParticipantLeft,
                            &disc_reason,
                        )
                        .await;
                }
                break;
            }
        }
    }

    forward_task.abort();
    if let Some(summary) = state.auth.leave_session(&session_id, &user_id).await {
        state
            .process_session_disconnect(
                &summary,
                LifecycleEventType::ParticipantDisconnected,
                "WebSocket connection closed",
            )
            .await;
    } else {
        let mut guard = state.active_sockets.write().await;
        if let Some(room_map) = guard.get_mut(&session_id) {
            room_map.remove(&user_id);
        }
    }
}
