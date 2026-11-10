//! AI Player Agent (`satya_ml.ai_player`) session registration, player-scoped context isolation
//! (`GET /api/player-ai/sessions/:id/context`), turn stepping (`POST /api/sessions/:id/ai-players/step`),
//! player-attributed chat/roll execution, and AI-to-AI rally turn budget enforcement.

use crate::gm_api::{
    apply_combat_action_to_mechanics_and_tracker, should_trigger_ai_gm_for_chat,
    trigger_ai_gm_chat_reply, verify_gm_service_header,
};
use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use rand::Rng;
use satya_engine::{CascadeTaskParams, evaluate_satya_cascade_task};
use satya_schema::{
    AiPlayerConfig, ChannelContext, ChatMessageEvent, MessageVisibility, Participant,
    PlayerAiChatRequest, PlayerAiRollRequest, ServerWsPacket, SessionMode, SessionRole,
    StepAiPlayerRequest, WgpuDiceSpec, subjects,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Active AI Player configuration and participant record stored per session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAiPlayerEntry {
    pub participant: Participant,
    pub persona_prompt: String,
    pub provider: String,
    pub model_id: String,
    pub service_url: Option<String>,
    pub auto_respond: bool,
    pub max_rally_turns: u32,
}

#[derive(Debug, Deserialize)]
pub struct PlayerAiContextQuery {
    #[serde(default = "default_ai_player_uid")]
    pub user_id: String,
}

fn default_ai_player_uid() -> String {
    "ai-player-1".to_string()
}

impl SignalingAppState {
    /// Resets the consecutive AI-to-AI rally turn counter for `session_id` (called when a human acts or steps a turn).
    pub async fn reset_ai_rally_counter(&self, session_id: &str) {
        self.ai_rally_counters
            .write()
            .await
            .insert(session_id.trim().to_string(), 0);
    }

    /// Atomically checks and increments the AI-to-AI rally counter for `session_id` against `max_rally_turns`.
    pub async fn try_consume_ai_rally_turn(&self, session_id: &str, max_rally_turns: u32) -> bool {
        let cap = max_rally_turns.clamp(1, 10);
        let mut guard = self.ai_rally_counters.write().await;
        let current = guard.entry(session_id.trim().to_string()).or_insert(0);
        if *current >= cap {
            return false;
        }
        *current += 1;
        true
    }
}

/// Lists all configured AI Player agents in `session_id` at `GET /api/sessions/:session_id/ai-players`.
pub async fn list_session_ai_players_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    if state.auth.verify_auth_token(auth_hdr).await.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    }

    let sid = session_id.trim();
    let players: Vec<SessionAiPlayerEntry> = state
        .ai_players
        .read()
        .await
        .get(sid)
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();
    let rally_count = state
        .ai_rally_counters
        .read()
        .await
        .get(sid)
        .copied()
        .unwrap_or(0);

    (
        StatusCode::OK,
        Json(json!({
            "session_id": sid,
            "ai_players": players,
            "rally_turns_used": rally_count,
        })),
    )
        .into_response()
}

/// Spawns or updates an AI Player in `session_id` at `POST /api/sessions/:session_id/ai-players`.
pub async fn upsert_session_ai_player_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<AiPlayerConfig>,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    if state.auth.verify_auth_token(auth_hdr).await.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    }

    let sid = session_id.trim().to_string();
    let default_cfg = &state.storage.config().ai_gm;
    let uid = req
        .user_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("ai-player-1")
        .to_string();
    let display_name = req
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Kaelen Voss (AI Player)")
        .to_string();
    let provider = req
        .provider
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&default_cfg.default_provider)
        .to_string();
    let model_id = req
        .model_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&default_cfg.reasoning_model)
        .to_string();
    let persona_prompt = req
        .persona_prompt
        .as_deref()
        .map(str::trim)
        .unwrap_or("Tactical recon scout; favors disciplined Tempo/Control fire and teamwork.")
        .to_string();
    let char_id = req
        .character_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let participant = Participant::ai_player(
        &uid,
        &display_name,
        char_id,
        &format!("{provider}/{model_id}"),
        "tactical-party-player",
    );

    let entry = SessionAiPlayerEntry {
        participant: participant.clone(),
        persona_prompt,
        provider,
        model_id,
        service_url: req
            .service_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        auto_respond: req.auto_respond,
        max_rally_turns: req.max_rally_turns.clamp(1, 10),
    };

    // Add to active session roster so `party_turn_gate_node` and clients see the AI Player
    let roster_opt = {
        let mut sessions_guard = state.auth.sessions.write().await;
        if let Some(session) = sessions_guard.get_mut(&sid) {
            session.roster.insert(uid.clone(), participant.clone());
            Some(session.roster.values().cloned().collect::<Vec<_>>())
        } else {
            None
        }
    };
    let Some(roster) = roster_opt else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{sid}' not found") })),
        )
            .into_response();
    };

    state
        .ai_players
        .write()
        .await
        .entry(sid.clone())
        .or_default()
        .insert(uid.clone(), entry.clone());
    state.reset_ai_rally_counter(&sid).await;

    if let Ok(sync_msg) = serde_json::to_string(&ServerWsPacket::RosterSync {
        roster: roster.clone(),
    }) {
        state.broadcast_to_session(&sid, &sync_msg).await;
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "configured",
            "session_id": sid,
            "ai_player": entry,
            "roster": roster,
        })),
    )
        .into_response()
}

/// Removes an AI Player from `session_id` at `DELETE /api/sessions/:session_id/ai-players/:player_user_id`.
pub async fn remove_session_ai_player_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path((session_id, player_user_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    if state.auth.verify_auth_token(auth_hdr).await.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    }

    let sid = session_id.trim();
    let uid = player_user_id.trim();
    if let Some(map) = state.ai_players.write().await.get_mut(sid) {
        map.remove(uid);
    }

    let mut roster = Vec::new();
    if let Some(session) = state.auth.sessions.write().await.get_mut(sid) {
        session.roster.remove(uid);
        roster = session.roster.values().cloned().collect();
    }
    if !roster.is_empty()
        && let Ok(sync_msg) = serde_json::to_string(&ServerWsPacket::RosterSync {
            roster: roster.clone(),
        })
    {
        state.broadcast_to_session(sid, &sync_msg).await;
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "removed",
            "session_id": sid,
            "player_user_id": uid,
            "roster": roster,
        })),
    )
        .into_response()
}

/// Manually steps one or all AI Players in `session_id` at `POST /api/sessions/:session_id/ai-players/step`.
pub async fn step_session_ai_player_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<StepAiPlayerRequest>,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    if state.auth.verify_auth_token(auth_hdr).await.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    }

    let sid = session_id.trim().to_string();
    state.reset_ai_rally_counter(&sid).await;

    let entries: Vec<SessionAiPlayerEntry> = {
        let guard = state.ai_players.read().await;
        let Some(room_players) = guard.get(&sid) else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("No AI Players configured in session '{sid}'") })),
            )
                .into_response();
        };
        let target_uid = req.user_id.as_deref().map(str::trim).unwrap_or("");
        if target_uid.is_empty() || target_uid.eq_ignore_ascii_case("all") {
            room_players.values().cloned().collect()
        } else if let Some(found) = room_players.get(target_uid) {
            vec![found.clone()]
        } else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": format!("AI Player '{target_uid}' not found in session '{sid}'") })),
            )
                .into_response();
        }
    };

    if entries.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("No AI Players active in session '{sid}'") })),
        )
            .into_response();
    }

    let prompt = req
        .prompt
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Take your character's turn in the current scene or combat tick.")
        .to_string();

    let mut stepped_ids = Vec::new();
    for entry in entries {
        stepped_ids.push(entry.participant.user_id.clone());
        trigger_ai_player_turn(&state, &sid, &entry, &prompt, req.auto_trigger_gm).await;
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "stepped",
            "session_id": sid,
            "stepped_player_user_ids": stepped_ids,
        })),
    )
        .into_response()
}

/// Returns strictly player-visible context for an AI Player at `GET /api/player-ai/sessions/:session_id/context`.
///
/// Never exposes `GM_ONLY` documents, persistent GM memory notes, NPC secret dossiers, or other players' private whispers.
pub async fn player_ai_session_context_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    Query(query): Query<PlayerAiContextQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let sid = session_id.trim();
    let player_uid = query.user_id.trim();

    let Some((gm_ctx, _)) = state.auth.get_session_gm_context(sid).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{sid}' not found") })),
        )
            .into_response();
    };

    let ai_entry = state
        .ai_players
        .read()
        .await
        .get(sid)
        .and_then(|m| m.get(player_uid))
        .cloned();

    // Load strictly player-visible chat history (`SessionRole::Player`, `has_human_gm = true` so GM_ONLY messages from others are excluded)
    let (visible_chats, _) = state
        .auth
        .load_visible_events_from_db(sid, player_uid, SessionRole::Player, true)
        .await;
    let recent_messages: Vec<ChatMessageEvent> = visible_chats
        .into_iter()
        .rev()
        .take(16)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    // Match this AI Player's bound character sheet if present
    let target_char_id = ai_entry
        .as_ref()
        .and_then(|e| e.participant.character_id.clone());
    let bound_sheet = gm_ctx
        .session_characters
        .iter()
        .find(|c| {
            target_char_id
                .as_deref()
                .is_some_and(|cid| c.character_id == cid)
                || c.owner_user_id == player_uid
        })
        .cloned();

    (
        StatusCode::OK,
        Json(json!({
            "session_id": sid,
            "player_user_id": player_uid,
            "player_display_name": ai_entry
                .as_ref()
                .map(|e| e.participant.display_name.clone())
                .unwrap_or_else(|| "Kaelen Voss (AI Player)".to_string()),
            "persona_prompt": ai_entry
                .as_ref()
                .map(|e| e.persona_prompt.clone())
                .unwrap_or_default(),
            "roster": gm_ctx.roster,
            "character_sheet": bound_sheet,
            "combat_tracker": gm_ctx.combat_tracker,
            "active_map": gm_ctx.active_map,
            "recent_messages": recent_messages,
        })),
    )
        .into_response()
}

/// Emits a chat message from an AI Player at `POST /api/player-ai/sessions/:session_id/chat`
/// and triggers the 7-node AI GM (`party_turn_gate_node`) if within the session's `max_rally_turns` budget.
pub async fn player_ai_session_chat_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<PlayerAiChatRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let sid = session_id.trim();
    let Some((ctx, _)) = state.auth.get_session_gm_context(sid).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{sid}' not found") })),
        )
            .into_response();
    };

    let ai_entry = state
        .ai_players
        .read()
        .await
        .get(sid)
        .and_then(|m| m.get(req.user_id.trim()))
        .cloned();

    let sender = ai_entry
        .as_ref()
        .map(|e| e.participant.clone())
        .or_else(|| {
            ctx.roster
                .iter()
                .find(|p| p.user_id == req.user_id.trim())
                .cloned()
        })
        .unwrap_or_else(|| {
            Participant::ai_player(
                req.user_id.trim(),
                req.display_name
                    .as_deref()
                    .unwrap_or("Kaelen Voss (AI Player)"),
                None,
                "ollama/gemma4:12b",
                "tactical-party-player",
            )
        });

    let (recipients, delivery_ids) = SignalingAppState::resolve_message_targets(
        &sender,
        req.visibility,
        &req.recipient_user_ids,
        &ctx.roster,
    );

    let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::CHAT_MESSAGE,
        1,
        sid,
        ctx.session_mode,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to build EventMetadata" })),
        )
            .into_response();
    };

    let chat_event = ChatMessageEvent {
        metadata: meta,
        message_id: Uuid::now_v7().to_string(),
        sender: sender.clone(),
        visibility: req.visibility,
        recipients,
        channel_context: req.channel_context,
        reply_to_message_id: None,
        body: req.body.clone(),
        char_count: req.body.chars().count() as i32,
        word_count: req.body.split_whitespace().count() as i32,
        composition_time_ms: 40,
        is_edited: false,
    };

    let _ = state.storage.log_chat_message(&chat_event).await;
    state.auth.record_chat_event_to_db(sid, &chat_event).await;
    let pkt_str = json!({
        "type": "CHAT_EVENT",
        "event": chat_event
    })
    .to_string();
    state.send_to_users(sid, &delivery_ids, &pkt_str).await;

    let max_rally = ai_entry.as_ref().map(|e| e.max_rally_turns).unwrap_or(2);
    if req.auto_trigger_gm
        && ctx.session_mode == SessionMode::Ttrpg
        && let Some(ai_gm) = ctx
            .roster
            .iter()
            .find(|p| p.role == SessionRole::AiGm)
            .cloned()
        && should_trigger_ai_gm_for_chat(
            &ctx.roster,
            &ai_gm,
            req.visibility,
            &req.recipient_user_ids,
            req.channel_context,
            &req.body,
        )
        && state.try_consume_ai_rally_turn(sid, max_rally).await
    {
        trigger_ai_gm_chat_reply(
            &state,
            sid,
            ctx.session_mode,
            &ai_gm,
            &sender,
            &chat_event.message_id,
            "CHAT",
            req.visibility,
            Some(req.channel_context),
            &req.body,
            &ctx.roster,
        )
        .await;
    }

    (StatusCode::OK, Json(chat_event)).into_response()
}

/// Evaluates an authoritative Satya `2d20` check + Combat Technique `T/C/F` triad for an AI Player
/// at `POST /api/player-ai/sessions/:session_id/roll`.
pub async fn player_ai_session_roll_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<PlayerAiRollRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let sid = session_id.trim();
    let Some((ctx, _)) = state.auth.get_session_gm_context(sid).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{sid}' not found") })),
        )
            .into_response();
    };

    let actor = state
        .ai_players
        .read()
        .await
        .get(sid)
        .and_then(|m| m.get(req.user_id.trim()))
        .map(|e| e.participant.clone())
        .or_else(|| {
            ctx.roster
                .iter()
                .find(|p| p.user_id == req.user_id.trim())
                .cloned()
        })
        .unwrap_or_else(|| {
            Participant::ai_player(
                req.user_id.trim(),
                req.actor_display_name
                    .as_deref()
                    .unwrap_or("Kaelen Voss (AI Player)"),
                None,
                "ollama/gemma4:12b",
                "tactical-party-player",
            )
        });

    let n_pairs = req.skill_rank.clamp(1, 10);
    let control_c = req.attribute_tn.clamp(1, 20);
    let int_mod = req.internal_mod.clamp(-2, 10);
    let ext_mod = req.difficulty_mod.clamp(-10, 16);
    let (rolled_pairs, physics_seed) = {
        let mut rng = rand::thread_rng();
        let pairs: Vec<(i32, i32)> = (0..n_pairs)
            .map(|_| (rng.gen_range(1..=20), rng.gen_range(1..=20)))
            .collect();
        let seed: i64 = rng.r#gen::<i64>().abs();
        (pairs, seed)
    };

    let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TTRPG_MECHANICS,
        1,
        sid,
        ctx.session_mode,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to build EventMetadata" })),
        )
            .into_response();
    };

    let params = CascadeTaskParams::new_k20(n_pairs, control_c, int_mod, ext_mod);
    let (mut mech_ev, outcome) = evaluate_satya_cascade_task(
        meta,
        actor.clone(),
        Vec::new(),
        req.check_category.clone(),
        &params,
        &rolled_pairs,
        None,
        int_mod.max(0),
        0.0,
    );
    if let Some(note) = req
        .notes
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        mech_ev.notes = Some(note.to_string());
    }

    apply_combat_action_to_mechanics_and_tracker(
        &state,
        sid,
        &actor,
        req.combat_technique.as_deref(),
        req.tempo_pool,
        req.control_pool,
        req.force_pool,
        None,
        control_c,
        n_pairs,
        int_mod.max(0),
        0.0,
        &mut mech_ev,
    )
    .await;

    let _ = state.storage.log_ttrpg_mechanics(&mech_ev).await;
    state.auth.record_mechanics_event_to_db(sid, &mech_ev).await;

    if let Ok(pkt_str) = serde_json::to_string(&ServerWsPacket::TtrpgMechanicsEvent {
        event: Box::new(mech_ev.clone()),
    }) {
        state.broadcast_to_session(sid, &pkt_str).await;
    }

    let dice_spec = WgpuDiceSpec {
        dice_notation: format!("{n_pairs}x2d20"),
        physics_seed,
        outcomes: mech_ev.dice_rolled_d20.clone(),
        visibility: MessageVisibility::Public,
    };
    if let Ok(dice_pkt) = serde_json::to_string(&ServerWsPacket::WgpuDiceRoll {
        actor: actor.clone(),
        spec: dice_spec,
    }) {
        state.broadcast_to_session(sid, &dice_pkt).await;
    }

    let next_tick = state
        .auth
        .get_session_combat_tracker(sid)
        .await
        .and_then(|t| {
            t.combatants
                .into_iter()
                .find(|c| c.entity_id == actor.user_id)
                .map(|c| c.next_action_tick)
        });

    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "event_id": mech_ev.metadata.event_id,
            "session_id": sid,
            "player_user_id": actor.user_id,
            "actor_display_name": actor.display_name,
            "check_category": req.check_category,
            "combat_technique": req.combat_technique,
            "effective_tn": params.tn_eff(),
            "rolled_pairs": rolled_pairs,
            "successes_counted": mech_ev.successes_counted,
            "signed_margin": outcome.signed_margin,
            "degree_of_success": format!("{:?}", mech_ev.outcome_tier),
            "effective_tc": mech_ev.action_count_cost,
            "next_action_tick": next_tick,
            "notes": mech_ev.notes,
        })),
    )
        .into_response()
}

/// Dispatches an AI Player turn to `POST /internal/ai-player/events` (or executes a deterministic
/// in-process player turn when the FastAPI service is offline during unit tests).
pub async fn trigger_ai_player_turn(
    state: &Arc<SignalingAppState>,
    session_id: &str,
    entry: &SessionAiPlayerEntry,
    prompt_text: &str,
    auto_trigger_gm: bool,
) {
    let ai_cfg = state.storage.config().ai_gm.clone();
    let target_url = entry
        .service_url
        .clone()
        .or_else(|| std::env::var("SATYA_AI_PLAYER_URL").ok())
        .unwrap_or_else(|| ai_cfg.service_url.clone());

    let (ctx_opt, vault_opt) = match state.auth.get_session_gm_context(session_id).await {
        Some((c, v)) => (Some(c), v),
        None => (None, None),
    };
    let ephemeral_key = vault_opt.as_ref().and_then(|v| v.api_key.clone());

    let payload = json!({
        "session_id": session_id,
        "player_user_id": entry.participant.user_id,
        "player_display_name": entry.participant.display_name,
        "persona_prompt": entry.persona_prompt,
        "roster": ctx_opt.as_ref().map(|c| c.roster.clone()).unwrap_or_default(),
        "combat_tracker": ctx_opt.as_ref().map(|c| c.combat_tracker.clone()),
        "active_map": ctx_opt.as_ref().and_then(|c| c.active_map.clone()),
        "content": prompt_text,
        "auto_trigger_gm": auto_trigger_gm,
        "provider": entry.provider,
        "model_id": entry.model_id,
    });

    if post_ai_player_event_to_fastapi(
        &target_url,
        &ai_cfg.service_token,
        ephemeral_key.as_deref(),
        &payload,
        ai_cfg.connect_timeout_ms,
        ai_cfg.request_timeout_secs,
    )
    .await
    .is_ok()
    {
        return;
    }

    if ai_cfg.fail_fast_when_offline {
        tracing::error!(
            session_id = %session_id,
            player_user_id = %entry.participant.user_id,
            "AI Player service at {target_url} unreachable (fail_fast_when_offline=true)"
        );
        return;
    }

    // Deterministic offline fallback for unit/integration tests (`fail_fast_when_offline == false`)
    let Some(ctx) = ctx_opt else {
        return;
    };
    let combat_active = ctx.combat_tracker.as_ref().is_some_and(|t| t.combat_active);
    let body = if combat_active {
        format!(
            "**{}** advances to cover and fires a `snap_shot` (`T=4/C=4/F=3`): *{prompt_text}*",
            entry.participant.display_name
        )
    } else {
        format!(
            "**{}** responds in-character: *\"Copy that — scanning the sector perimeter now.\" ({prompt_text})*",
            entry.participant.display_name
        )
    };

    let (recipients, delivery_ids) = SignalingAppState::resolve_message_targets(
        &entry.participant,
        MessageVisibility::Public,
        &[],
        &ctx.roster,
    );
    if let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::CHAT_MESSAGE,
        1,
        session_id,
        ctx.session_mode,
    ) {
        let ev = ChatMessageEvent {
            metadata: meta,
            message_id: Uuid::now_v7().to_string(),
            sender: entry.participant.clone(),
            visibility: MessageVisibility::Public,
            recipients,
            channel_context: ChannelContext::InCharacter,
            reply_to_message_id: None,
            body: body.clone(),
            char_count: body.chars().count() as i32,
            word_count: body.split_whitespace().count() as i32,
            composition_time_ms: 35,
            is_edited: false,
        };
        let _ = state.storage.log_chat_message(&ev).await;
        state.auth.record_chat_event_to_db(session_id, &ev).await;
        let pkt_str = json!({
            "type": "CHAT_EVENT",
            "event": ev
        })
        .to_string();
        state
            .send_to_users(session_id, &delivery_ids, &pkt_str)
            .await;

        if auto_trigger_gm
            && state
                .try_consume_ai_rally_turn(session_id, entry.max_rally_turns)
                .await
            && let Some(ai_gm) = ctx
                .roster
                .iter()
                .find(|p| p.role == SessionRole::AiGm)
                .cloned()
        {
            trigger_ai_gm_chat_reply(
                state,
                session_id,
                ctx.session_mode,
                &ai_gm,
                &entry.participant,
                &ev.message_id,
                "CHAT",
                MessageVisibility::Public,
                Some(ChannelContext::InCharacter),
                &body,
                &ctx.roster,
            )
            .await;
        }
    }
}

/// Automatically triggers any `auto_respond = true` AI Players in `session_id` after a human player
/// submits a public `IN_CHARACTER` chat declaration or `RollTtrpgTask`.
pub async fn trigger_auto_ai_players_for_human_turn(
    state: &Arc<SignalingAppState>,
    session_id: &str,
    human_actor: &Participant,
    human_action_summary: &str,
) {
    let entries: Vec<SessionAiPlayerEntry> = state
        .ai_players
        .read()
        .await
        .get(session_id.trim())
        .map(|m| m.values().filter(|e| e.auto_respond).cloned().collect())
        .unwrap_or_default();

    if entries.is_empty() {
        return;
    }

    let prompt = format!(
        "Fellow party member {} just declared: \"{}\". Declare your character's complementary action or roll for this party turn.",
        human_actor.display_name, human_action_summary
    );
    for entry in entries {
        trigger_ai_player_turn(state, session_id, &entry, &prompt, true).await;
    }
}

async fn post_ai_player_event_to_fastapi(
    service_url: &str,
    service_token: &str,
    ephemeral_api_key: Option<&str>,
    payload: &serde_json::Value,
    connect_timeout_ms: u64,
    request_timeout_secs: u64,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let trimmed = service_url
        .trim()
        .strip_prefix("http://")
        .or_else(|| service_url.trim().strip_prefix("https://"))
        .unwrap_or(service_url.trim());
    let host_port = trimmed.split('/').next().unwrap_or("127.0.0.1:8090");
    let connect_addr = if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{host_port}:80")
    };

    let body_bytes = serde_json::to_vec(payload)?;
    let mut extra_hdrs = String::new();
    if let Some(k) = ephemeral_api_key.filter(|s| !s.trim().is_empty()) {
        extra_hdrs.push_str(&format!("X-Satya-Provider-Key: {}\r\n", k.trim()));
    }

    let req_bytes = format!(
        "POST /internal/ai-player/events HTTP/1.1\r\n\
         Host: {host_port}\r\n\
         Authorization: Bearer {service_token}\r\n\
         {extra_hdrs}\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body_bytes.len()
    );

    let mut stream = tokio::time::timeout(
        std::time::Duration::from_millis(connect_timeout_ms.max(100)),
        tokio::net::TcpStream::connect(&connect_addr),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!("Connection to AI Player service at {connect_addr} timed out")
    })??;

    stream.write_all(req_bytes.as_bytes()).await?;
    stream.write_all(&body_bytes).await?;
    stream.flush().await?;

    let mut resp_buf = vec![0u8; 512];
    let n = tokio::time::timeout(
        std::time::Duration::from_secs(request_timeout_secs.max(1)),
        stream.read(&mut resp_buf),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Timed out awaiting AI Player response from {connect_addr}"))??;

    let status_line = String::from_utf8_lossy(&resp_buf[..n]);
    if !status_line.starts_with("HTTP/1.1 2") && !status_line.starts_with("HTTP/1.0 2") {
        anyhow::bail!(
            "AI Player service at {connect_addr} returned non-2xx response: {}",
            status_line.lines().next().unwrap_or("empty response")
        );
    }
    Ok(())
}
