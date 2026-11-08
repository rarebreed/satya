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
use satya_engine::{
    BotchRule, CarryMode, CascadeTaskParams, ContestedCascadeTaskParams, ContestedTieRule,
    GmResistanceMode, evaluate_satya_cascade_task, evaluate_satya_contested_cascade_task,
};
use satya_schema::{
    CanvasEventType, ChatMessageEvent, ClientWsPacket, IceCandidateType, InteractionDynamicsEvent,
    LifecycleEventType, MessageVisibility, ParticipantKind, ServerWsPacket, SessionMode,
    SessionRole, TabletopCanvasEvent, WebRtcTelemetryEvent, WgpuDiceSpec, subjects,
};
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;
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
                        let state_clone = state.clone();
                        let session_id_clone = session_id.clone();
                        let self_part_clone = self_participant.clone();
                        let msg_id_clone = chat_event.message_id.clone();
                        let body_clone = body.clone();
                        let roster_clone = current_roster.clone();
                        tokio::spawn(async move {
                            trigger_ai_gm_chat_reply(
                                &state_clone,
                                &session_id_clone,
                                session_mode,
                                &ai_gm,
                                &self_part_clone,
                                &msg_id_clone,
                                "CHAT",
                                visibility,
                                &body_clone,
                                &roster_clone,
                            )
                            .await;
                        });
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
                skill_level,
                control_c,
                internal_mod,
                known_tool_mod,
                external_mod,
                delta_over,
                gm_resistance_mode,
                tie_rule,
                carry_mode,
                botch_rule,
                combat_stance,
                fatigue_level,
                wound_shock_index,
            } => {
                let n_pairs = skill_level.clamp(1, 10);
                let c_val = control_c.clamp(1, 20);
                let int_mod = internal_mod.clamp(-2, 10);
                let tool_mod = known_tool_mod.clamp(-4, 12);
                let ext_mod = external_mod.clamp(-10, 16);
                let parsed_tie_rule = match tie_rule.as_deref() {
                    Some("strict_signed_margin") => ContestedTieRule::StrictSignedMargin,
                    Some("active_player_wins_ties") => ContestedTieRule::ActivePlayerWinsTies,
                    _ => ContestedTieRule::HybridSignedMargin,
                };
                let is_uncontested =
                    matches!(gm_resistance_mode.as_deref(), Some("uncontested" | "none"));

                let mut contested_params = ContestedCascadeTaskParams::new_tiered_environment(
                    n_pairs,
                    c_val,
                    int_mod,
                    tool_mod,
                    ext_mod,
                    parsed_tie_rule,
                );
                if matches!(gm_resistance_mode.as_deref(), Some("full_cascade")) {
                    contested_params.gm_mode = GmResistanceMode::FullCascade;
                }
                if let Some(d_over) = delta_over {
                    contested_params.player.delta_over = d_over.clamp(2, 12);
                }
                if let Some(mode_str) = carry_mode.as_deref()
                    && let Ok(parsed_mode) = CarryMode::from_str(mode_str)
                {
                    contested_params.player.carry_mode = parsed_mode;
                }
                if let Some(botch_str) = botch_rule.as_deref()
                    && let Ok(parsed_botch) = BotchRule::from_str(botch_str)
                {
                    contested_params.player.botch_rule = parsed_botch;
                }

                let (rolled_pairs, gm_pairs, physics_seed) = {
                    let mut rng = rand::thread_rng();
                    let pairs: Vec<(i32, i32)> = (0..n_pairs)
                        .map(|_| (rng.gen_range(1..=20), rng.gen_range(1..=20)))
                        .collect();
                    let gm = vec![(rng.gen_range(1..=20), rng.gen_range(1..=20))];
                    let seed: i64 = rng.r#gen::<i64>().abs();
                    (pairs, gm, seed)
                };

                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::TTRPG_MECHANICS,
                    1,
                    &session_id,
                    session_mode,
                ) {
                    let mech_ev = if is_uncontested {
                        let mut params =
                            CascadeTaskParams::new_k20(n_pairs, c_val, int_mod, tool_mod + ext_mod);
                        params.carry_mode = contested_params.player.carry_mode;
                        params.botch_rule = contested_params.player.botch_rule;
                        params.delta_over = contested_params.player.delta_over;
                        let (mut ev, outcome) = evaluate_satya_cascade_task(
                            meta,
                            self_participant.clone(),
                            Vec::new(),
                            skill_name.clone(),
                            &params,
                            &rolled_pairs,
                            combat_stance,
                            fatigue_level.max(0),
                            wound_shock_index,
                        );
                        let odds = params.exact_odds();
                        let step_summary: Vec<String> = outcome
                            .steps
                            .iter()
                            .map(|s| {
                                let mark = if s.succeeded {
                                    "PASS"
                                } else if s.step_botch {
                                    "BOTCH"
                                } else {
                                    "MISS"
                                };
                                format!(
                                    "P{}: {}+{}{:+}={} ({})",
                                    s.pair_index, s.die_a, s.die_b, s.carry_in, s.effective_y, mark
                                )
                            })
                            .collect();
                        ev.notes = Some(format!(
                            "cascade(mode={}, botch={}, tn_eff={}, signed_margin={:+}) | Pairs: [{}] | C={}, Δext={:+} => TN_eff={} | Mode: {} (+{}/-{}) | Botch: {} | Signed Margin: {:+} ({:?}, P(Succ>=1)={:.1}%)",
                            params.carry_mode,
                            params.botch_rule,
                            params.tn_eff(),
                            outcome.signed_margin,
                            step_summary.join(", "),
                            c_val,
                            tool_mod + ext_mod,
                            params.tn_eff(),
                            params.carry_mode,
                            params.delta_over,
                            params.effective_delta_under(),
                            params.botch_rule,
                            outcome.signed_margin,
                            outcome.outcome_kind,
                            odds.prob_any_success * 100.0
                        ));
                        ev
                    } else {
                        let (mut ev, outcome) = evaluate_satya_contested_cascade_task(
                            meta,
                            self_participant.clone(),
                            Vec::new(),
                            skill_name.clone(),
                            &contested_params,
                            &rolled_pairs,
                            &gm_pairs,
                            combat_stance,
                            fatigue_level.max(0),
                            wound_shock_index,
                        );
                        let odds = contested_params.exact_contested_odds();
                        let step_summary: Vec<String> = outcome
                            .player_outcome
                            .steps
                            .iter()
                            .map(|s| {
                                let mark = if s.succeeded {
                                    "PASS"
                                } else if s.step_botch {
                                    "BOTCH"
                                } else {
                                    "MISS"
                                };
                                format!(
                                    "P{}: {}+{}{:+}={} ({})",
                                    s.pair_index, s.die_a, s.die_b, s.carry_in, s.effective_y, mark
                                )
                            })
                            .collect();
                        let gm_step = outcome.gm_outcome.steps.first();
                        let gm_summary = gm_step
                            .map(|g| {
                                format!(
                                    "GM: {}+{}={} vs TN_GM={} (S_GM={}, M_GM={:+})",
                                    g.die_a,
                                    g.die_b,
                                    g.raw_sum,
                                    g.tn_eff,
                                    outcome.gm_outcome.total_successes,
                                    outcome.gm_outcome.signed_margin
                                )
                            })
                            .unwrap_or_default();
                        ev.notes = Some(format!(
                            "contested(kind={}, tn_p={}, tn_gm={}, s_p={}, s_gm={}, net_margin={:+}) | cascade(mode={}, botch={}, tn_eff={}, signed_margin={:+}) | Pairs: [{}] | {} | C={}, Δtool={:+}, Δext={:+} => TN_P={}, TN_GM={} | Mode: {} (+{}/-{}) | Net Margin: {:+} ({}, P(Win)={:.1}%)",
                            outcome.contested_kind,
                            contested_params.player.tn_eff(),
                            contested_params.gm.tn_eff(),
                            outcome.player_outcome.total_successes,
                            outcome.gm_outcome.total_successes,
                            outcome.net_signed_margin,
                            contested_params.player.carry_mode,
                            contested_params.player.botch_rule,
                            contested_params.player.tn_eff(),
                            outcome.player_outcome.signed_margin,
                            step_summary.join(", "),
                            gm_summary,
                            c_val,
                            tool_mod,
                            ext_mod,
                            contested_params.player.tn_eff(),
                            contested_params.gm.tn_eff(),
                            contested_params.player.carry_mode,
                            contested_params.player.delta_over,
                            contested_params.player.effective_delta_under(),
                            outcome.net_signed_margin,
                            outcome.contested_kind,
                            odds.prob_total_win * 100.0
                        ));
                        ev
                    };

                    let _ = state.storage.log_ttrpg_mechanics(&mech_ev).await;

                    // Emit synchronized 3D `wgpu` physics dice roll for the `2d20` pairs
                    let dice_spec = WgpuDiceSpec {
                        dice_notation: format!("{n_pairs}x2d20"),
                        physics_seed,
                        outcomes: mech_ev.dice_rolled_d20.clone(),
                        visibility: MessageVisibility::Public,
                    };
                    if let Ok(canvas_meta) = state.storage.registry().build_metadata(
                        state.next_seq(),
                        subjects::TABLETOP_CANVAS,
                        1,
                        &session_id,
                        session_mode,
                    ) {
                        let canvas_ev = TabletopCanvasEvent {
                            metadata: canvas_meta,
                            canvas_event_type: CanvasEventType::WgpuDiceSpawned,
                            actor: self_participant.clone(),
                            map_id: None,
                            map_title: None,
                            image_url: None,
                            grid_width: None,
                            grid_height: None,
                            token_placements: Vec::new(),
                            wgpu_dice_roll: Some(dice_spec.clone()),
                            terrain_notes: mech_ev.notes.clone(),
                        };
                        let _ = state.storage.log_tabletop_canvas(&canvas_ev).await;
                    }
                    if let Ok(wgpu_pkt) = serde_json::to_string(&ServerWsPacket::WgpuDiceRoll {
                        actor: self_participant.clone(),
                        spec: dice_spec,
                    }) {
                        state.broadcast_to_session(&session_id, &wgpu_pkt).await;
                    }

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
                            "[Satya Simulation] {} attempted '{}' (Skill N={}, Control C={}, Δtool={:+}, Δext={:+}, TN_P={}, TN_GM={}, Δint={:+}, Carry={}) -> {:?} ({} successes). {} Action Count Cost: {} ticks.",
                            self_participant.display_name,
                            skill_name,
                            contested_params.player.skill_level,
                            contested_params.player.control_c,
                            tool_mod,
                            ext_mod,
                            contested_params.player.tn_eff(),
                            contested_params.gm.tn_eff(),
                            int_mod,
                            contested_params.player.carry_mode,
                            mech_ev.outcome_tier,
                            mech_ev.successes_counted,
                            mech_ev.notes.as_deref().unwrap_or(""),
                            mech_ev.action_count_cost.unwrap_or(10)
                        );
                        let state_clone = state.clone();
                        let session_id_clone = session_id.clone();
                        let self_part_clone = self_participant.clone();
                        let ev_id_clone = mech_ev.metadata.event_id.clone();
                        let roster_clone = current_roster.clone();
                        tokio::spawn(async move {
                            trigger_ai_gm_chat_reply(
                                &state_clone,
                                &session_id_clone,
                                session_mode,
                                &ai_gm,
                                &self_part_clone,
                                &ev_id_clone,
                                "MECHANICS",
                                MessageVisibility::Public,
                                &narration,
                                &roster_clone,
                            )
                            .await;
                        });
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
