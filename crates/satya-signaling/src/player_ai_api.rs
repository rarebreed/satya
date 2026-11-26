//! AI Player Agent (`satya_ml.ai_player`) session registration, player-scoped context isolation
//! (`GET /api/player-ai/sessions/:id/context`), turn stepping (`POST /api/sessions/:id/ai-players/step`),
//! player-attributed chat/roll execution, and AI-to-AI rally turn budget enforcement.

use crate::gm_api::{
    apply_combat_action_to_mechanics_and_tracker, evaluate_roll_combat_action_and_triad,
    should_trigger_ai_gm_for_chat, trigger_ai_gm_chat_reply, verify_gm_service_header,
};
use crate::state::{MAX_AUTONOMOUS_RALLY_ROUNDS, SessionRallyState, SignalingAppState};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use rand::Rng;
use satya_engine::{CascadeTaskParams, evaluate_satya_cascade_task};
use satya_schema::{
    AiPlayerConfig, CanvasEventType, ChannelContext, ChatMessageEvent, MessageVisibility,
    Participant, ParticipantKind, PlayerAiChatRequest, PlayerAiRollRequest, ServerWsPacket,
    SessionMode, SessionRole, StepAiPlayerRequest, TabletopCanvasEvent, UpdateAiRallyRequest,
    WgpuDiceSpec, subjects,
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

pub use crate::player_ai_spatial::*;

#[derive(Debug, Deserialize)]
pub struct PlayerAiContextQuery {
    #[serde(default = "default_ai_player_uid")]
    pub user_id: String,
}

pub(crate) fn default_ai_player_uid() -> String {
    "ai-player-1".to_string()
}

/// Validates an external or custom AI Player `service_url` against loopback and configured allowlists (C2 SSRF guard).
pub(crate) fn validate_ai_player_service_url(
    raw_url: &str,
    allowed_origins: &[String],
) -> anyhow::Result<String> {
    let trimmed = raw_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        anyhow::bail!("service_url cannot be empty");
    }
    let after_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .ok_or_else(|| anyhow::anyhow!("service_url must use http:// or https:// scheme"))?;
    let host_port = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    if host_port.is_empty() || host_port.contains('@') {
        anyhow::bail!("Invalid authority in AI Player service_url");
    }
    let host = if let Some(rest) = host_port.strip_prefix('[') {
        rest.split(']').next().unwrap_or("").to_ascii_lowercase()
    } else {
        host_port
            .split(':')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
    };
    let is_loopback = matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1");
    let scheme_and_authority = if trimmed.starts_with("https://") {
        format!("https://{host_port}")
    } else {
        format!("http://{host_port}")
    };
    let in_allowlist = allowed_origins.iter().any(|allowed| {
        let norm = allowed.trim().trim_end_matches('/');
        !norm.is_empty() && norm.eq_ignore_ascii_case(&scheme_and_authority)
    });
    if !is_loopback && !in_allowlist {
        anyhow::bail!(
            "Untrusted AI Player service_url host '{host}'; only loopback or configured allowlist origins are permitted"
        );
    }
    if is_loopback && !in_allowlist {
        let self_port: u16 = std::env::var("PORT")
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(3000);
        let req_port: u16 = if let Some(rest) = host_port.strip_prefix('[') {
            rest.split(']')
                .nth(1)
                .and_then(|s| s.strip_prefix(':'))
                .and_then(|p| p.parse().ok())
                .unwrap_or(if trimmed.starts_with("https://") { 443 } else { 80 })
        } else {
            host_port
                .split(':')
                .nth(1)
                .and_then(|p| p.parse().ok())
                .unwrap_or(if trimmed.starts_with("https://") { 443 } else { 80 })
        };
        if req_port == self_port {
            anyhow::bail!(
                "AI Player service_url cannot target the signaling server's own port ({self_port})"
            );
        }
    }
    Ok(trimmed.to_string())
}

impl SignalingAppState {
    /// Resets the consecutive AI-to-AI rally turn counter for `session_id` (and clears pause state).
    pub async fn reset_ai_rally_counter(&self, session_id: &str) {
        let mut guard = self.ai_rally_counters.write().await;
        let entry = guard.entry(session_id.trim().to_string()).or_default();
        entry.rounds_used = 0;
        entry.paused = false;
    }

    /// Returns `true` when autonomous AI Player rallying is currently paused for `session_id`.
    pub async fn is_ai_rally_paused(&self, session_id: &str) -> bool {
        self.ai_rally_counters
            .read()
            .await
            .get(session_id.trim())
            .is_some_and(|s| s.paused)
    }

    /// Updates pause state, max rounds (`1..=MAX_AUTONOMOUS_RALLY_ROUNDS`), or resets counter for `session_id`.
    pub async fn update_ai_rally_state(
        &self,
        session_id: &str,
        req: &UpdateAiRallyRequest,
    ) -> SessionRallyState {
        let mut guard = self.ai_rally_counters.write().await;
        let st = guard.entry(session_id.trim().to_string()).or_default();
        if let Some(p) = req.paused {
            st.paused = p;
        }
        if let Some(max_r) = req.max_rally_rounds {
            st.max_rounds = max_r.clamp(1, MAX_AUTONOMOUS_RALLY_ROUNDS);
        }
        if req.reset_counter == Some(true) {
            st.rounds_used = 0;
        }
        *st
    }

    /// Atomically checks and increments the AI-to-AI rally round counter against `max_rally_turns`.
    pub async fn try_consume_ai_rally_turn(&self, session_id: &str, max_rally_turns: u32) -> bool {
        let mut guard = self.ai_rally_counters.write().await;
        let st = guard.entry(session_id.trim().to_string()).or_default();
        if st.paused {
            return false;
        }
        let cap = if max_rally_turns > 0 {
            max_rally_turns.clamp(1, MAX_AUTONOMOUS_RALLY_ROUNDS)
        } else {
            st.max_rounds.clamp(1, MAX_AUTONOMOUS_RALLY_ROUNDS)
        };
        if st.rounds_used >= cap {
            return false;
        }
        st.rounds_used += 1;
        true
    }

    /// Hydrates `self.ai_players` and roster AI Player entries from SQLite `session_ai_players` if not yet in memory.
    pub async fn ensure_session_ai_players_loaded(&self, session_id: &str) {
        let sid = session_id.trim();
        let persisted = self.auth.load_session_ai_players_from_db(sid).await;
        if persisted.is_empty() {
            return;
        }
        let mut ai_guard = self.ai_players.write().await;
        let room_map = ai_guard.entry(sid.to_string()).or_default();
        let mut sessions_guard = self.auth.sessions.write().await;
        let mut session_opt = sessions_guard.get_mut(sid);
        let default_cfg = &self.storage.config().ai_gm;
        for cfg in persisted {
            let Some(uid) = cfg.user_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
                continue;
            };
            let display_name = cfg.display_name.as_deref().unwrap_or(uid).to_string();
            let provider = cfg
                .provider
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&default_cfg.default_provider)
                .to_string();
            let model_id = cfg
                .model_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&default_cfg.reasoning_model)
                .to_string();
            let participant = Participant::ai_player(
                uid,
                &display_name,
                cfg.character_id.clone(),
                &format!("{provider}/{model_id}"),
                "tactical-party-player",
            );
            if let Some(ref mut sess) = session_opt {
                sess.roster.entry(uid.to_string()).or_insert_with(|| participant.clone());
            }
            room_map.entry(uid.to_string()).or_insert_with(|| SessionAiPlayerEntry {
                participant,
                persona_prompt: cfg.persona_prompt.unwrap_or_default(),
                provider,
                model_id,
                service_url: cfg.service_url,
                auto_respond: cfg.auto_respond,
                max_rally_turns: cfg.max_rally_turns.clamp(1, MAX_AUTONOMOUS_RALLY_ROUNDS),
            });
        }
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
    let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };

    let sid = session_id.trim();
    if state
        .auth
        .resolve_authorized_member_role(sid, &user)
        .await
        .is_err()
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Caller is not an active participant in this session" })),
        )
            .into_response();
    }
    state.ensure_session_ai_players_loaded(sid).await;
    let players: Vec<SessionAiPlayerEntry> = state
        .ai_players
        .read()
        .await
        .get(sid)
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();
    let rally_state = state
        .ai_rally_counters
        .read()
        .await
        .get(sid)
        .copied()
        .unwrap_or_default();

    (
        StatusCode::OK,
        Json(json!({
            "session_id": sid,
            "ai_players": players,
            "rally_turns_used": rally_state.rounds_used,
            "max_rally_rounds": rally_state.max_rounds,
            "rally_paused": rally_state.paused,
        })),
    )
        .into_response()
}

/// Updates pause/resume or round cap for autonomous AI Player rallies at `POST /api/sessions/:session_id/ai-players/rally`.
pub async fn update_session_ai_rally_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<UpdateAiRallyRequest>,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header" })),
        )
            .into_response();
    };
    let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };
    let sid = session_id.trim();
    if !crate::npcs::can_caller_orchestrate_ai_session(&state, sid, &user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the session GM or host can control autonomous AI rallies" })),
        )
            .into_response();
    }
    let st = state.update_ai_rally_state(sid, &req).await;
    (
        StatusCode::OK,
        Json(json!({
            "session_id": sid,
            "rally_turns_used": st.rounds_used,
            "max_rally_rounds": st.max_rounds,
            "rally_paused": st.paused,
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
    let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };
    let sid = session_id.trim().to_string();
    if !crate::npcs::can_caller_orchestrate_ai_session(&state, &sid, &user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the session GM or host can configure AI Players" })),
        )
            .into_response();
    }

    let default_cfg = &state.storage.config().ai_gm;
    let pack_defaults =
        crate::campaign_lore::resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let uid = req
        .user_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("ai-player-1")
        .to_string();
    if !uid.starts_with("ai-player-") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "AI Player user_id must start with 'ai-player-'" })),
        )
            .into_response();
    }
    let validated_service_url = if let Some(raw_url) = req
        .service_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        match validate_ai_player_service_url(raw_url, &default_cfg.allowed_ai_player_service_urls)
        {
            Ok(u) => Some(u),
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": err.to_string() })),
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    let display_name = req
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&pack_defaults.default_ai_player_name)
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
    if persona_prompt.chars().count() > 4096 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "persona_prompt must not exceed 4096 characters" })),
        )
            .into_response();
    }
    let char_id = req
        .character_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let participant = Participant::ai_player(
        &uid,
        &display_name,
        char_id.clone(),
        &format!("{provider}/{model_id}"),
        "tactical-party-player",
    );

    let entry = SessionAiPlayerEntry {
        participant: participant.clone(),
        persona_prompt: persona_prompt.clone(),
        provider: provider.clone(),
        model_id: model_id.clone(),
        service_url: validated_service_url.clone(),
        auto_respond: req.auto_respond,
        max_rally_turns: req.max_rally_turns.clamp(1, MAX_AUTONOMOUS_RALLY_ROUNDS),
    };

    // Add to active session roster & combat tracker so `party_turn_gate_node` and clients see the AI Player
    let roster_res: Result<Vec<satya_schema::Participant>, (StatusCode, String)> = {
        let mut sessions_guard = state.auth.sessions.write().await;
        if let Some(session) = sessions_guard.get_mut(&sid) {
            let ai_player_count = session
                .roster
                .values()
                .filter(|p| {
                    p.participant_kind == satya_schema::ParticipantKind::AiAgent
                        && p.role == SessionRole::Player
                        && p.user_id != uid
                })
                .count();
            if ai_player_count >= 6 {
                Err((
                    StatusCode::BAD_REQUEST,
                    "Maximum 6 AI Players per session allowed".to_string(),
                ))
            } else if let Some(existing) = session.roster.get(&uid)
                && (existing.participant_kind == satya_schema::ParticipantKind::Human
                    || existing.role == SessionRole::AiGm)
            {
                Err((
                    StatusCode::CONFLICT,
                    format!("Roster slot '{uid}' is already occupied by a Human or AI GM participant"),
                ))
            } else {
                session.roster.insert(uid.clone(), participant.clone());
                crate::auth::AuthAndSessionManager::sync_combatant_into_tracker(
                    &mut session.combat_tracker,
                    &participant,
                );
                Ok(session.roster.values().cloned().collect::<Vec<_>>())
            }
        } else {
            Err((StatusCode::NOT_FOUND, format!("Session '{sid}' not found")))
        }
    };
    let roster = match roster_res {
        Ok(r) => r,
        Err((status, msg)) => return (status, Json(json!({ "error": msg }))).into_response(),
    };

    let persist_cfg = AiPlayerConfig {
        user_id: Some(uid.clone()),
        display_name: Some(display_name),
        character_id: char_id,
        persona_prompt: Some(persona_prompt),
        provider: Some(provider),
        model_id: Some(model_id),
        service_url: validated_service_url,
        auto_respond: req.auto_respond,
        max_rally_turns: req.max_rally_turns.clamp(1, MAX_AUTONOMOUS_RALLY_ROUNDS),
    };
    state
        .auth
        .upsert_session_ai_player_to_db(&sid, &persist_cfg)
        .await;

    state
        .ai_players
        .write()
        .await
        .entry(sid.clone())
        .or_default()
        .insert(uid.clone(), entry.clone());
    state.reset_ai_rally_counter(&sid).await;
    let _ = state
        .update_ai_rally_state(
            &sid,
            &UpdateAiRallyRequest {
                paused: Some(false),
                max_rally_rounds: Some(entry.max_rally_turns),
                reset_counter: Some(true),
            },
        )
        .await;

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
    let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };

    let sid = session_id.trim();
    if !crate::npcs::can_caller_orchestrate_ai_session(&state, sid, &user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the session GM or host can remove AI Players" })),
        )
            .into_response();
    }
    let uid = player_user_id.trim();
    if !uid.starts_with("ai-player-") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "player_user_id must refer to an AI Player ('ai-player-*')" })),
        )
            .into_response();
    }
    if let Some(session) = state.auth.sessions.read().await.get(sid)
        && let Some(existing) = session.roster.get(uid)
        && (existing.participant_kind != ParticipantKind::AiAgent
            || existing.role != SessionRole::Player)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Cannot remove a human participant or AI GM via ai-players endpoint" })),
        )
            .into_response();
    }
    if let Some(map) = state.ai_players.write().await.get_mut(sid) {
        map.remove(uid);
    }
    state.auth.delete_session_ai_player_from_db(sid, uid).await;

    let mut roster = Vec::new();
    if let Some(session) = state.auth.sessions.write().await.get_mut(sid) {
        session.roster.remove(uid);
        session
            .combat_tracker
            .combatants
            .retain(|c| c.entity_id != uid);
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
    let Some(user) = state.auth.verify_auth_token(auth_hdr).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired auth token" })),
        )
            .into_response();
    };

    let sid = session_id.trim().to_string();
    if !crate::npcs::can_caller_orchestrate_ai_session(&state, &sid, &user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Only the session GM or host can step AI Players" })),
        )
            .into_response();
    }
    state.ensure_session_ai_players_loaded(&sid).await;
    state.reset_ai_rally_counter(&sid).await;

    let mut entries: Vec<SessionAiPlayerEntry> = {
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
    entries.sort_by(|a, b| a.participant.user_id.cmp(&b.participant.user_id));

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

    let stepped_ids: Vec<String> = entries
        .iter()
        .map(|e| e.participant.user_id.clone())
        .collect();
    let fail_fast = state.storage.config().ai_gm.fail_fast_when_offline;
    if entries.len() > 1 && fail_fast {
        let state_clone = state.clone();
        let sid_clone = sid.clone();
        let prompt_clone = prompt.clone();
        let auto_gm = req.auto_trigger_gm;
        tokio::spawn(async move {
            for entry in entries {
                trigger_ai_player_turn(&state_clone, &sid_clone, &entry, &prompt_clone, auto_gm)
                    .await;
            }
        });
    } else {
        for entry in entries {
            trigger_ai_player_turn(&state, &sid, &entry, &prompt, req.auto_trigger_gm).await;
        }
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

    let filtered_active_map = gm_ctx
        .active_map
        .as_ref()
        .map(|m| crate::map_handlers::filter_map_for_role(m, SessionRole::Player));

    let mut seen_npc_ids = std::collections::BTreeSet::new();
    let mut public_npcs = Vec::new();
    for npc in crate::npcs::load_session_npcs_for_omnisearch(&state.auth, sid, false)
        .await
        .into_iter()
        .chain(crate::campaign_lore::load_pack_starter_npcs(
            Some(state.campaign_pack.as_str()),
            false,
        ))
    {
        if seen_npc_ids.insert(npc.character_id.clone()) {
            public_npcs.push(npc);
        }
    }

    let raw_ledger = crate::world_clocks::get_or_init_world_ledger(&state, sid).await;
    let filtered_world_ledger = satya_campaign::filter_world_ledger_for_viewer(&raw_ledger, false);
    let pack_defaults =
        crate::campaign_lore::resolve_pack_defaults(Some(state.campaign_pack.as_str()));
    let filtered_combat_tracker = gm_ctx.combat_tracker.as_ref().map(|t| {
        crate::auth::AuthAndSessionManager::filter_combat_tracker_for_viewer(
            t,
            gm_ctx.active_map.as_ref(),
            false,
        )
    });

    (
        StatusCode::OK,
        Json(json!({
            "session_id": sid,
            "player_user_id": player_uid,
            "player_display_name": ai_entry
                .as_ref()
                .map(|e| e.participant.display_name.clone())
                .unwrap_or_else(|| pack_defaults.default_ai_player_name.clone()),
            "persona_prompt": ai_entry
                .as_ref()
                .map(|e| e.persona_prompt.clone())
                .unwrap_or_default(),
            "roster": gm_ctx.roster,
            "character_sheet": bound_sheet,
            "combat_tracker": filtered_combat_tracker,
            "active_map": filtered_active_map,
            "public_npcs": public_npcs,
            "world_ledger": filtered_world_ledger,
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
    let pack_defaults =
        crate::campaign_lore::resolve_pack_defaults(Some(state.campaign_pack.as_str()));

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
                    .unwrap_or(&pack_defaults.default_ai_player_name),
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

    // Always allow AI GM to resolve an AI Player's message when auto_trigger_gm == true and rallies are not paused
    // (rally round limits are advanced once per complete AI GM narration round in gm_api.rs).
    if req.auto_trigger_gm
        && !state.is_ai_rally_paused(sid).await
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
    {
        let state_clone = state.clone();
        let sid_clone = sid.to_string();
        let session_mode = ctx.session_mode;
        let sender_clone = sender.clone();
        let msg_id_clone = chat_event.message_id.clone();
        let vis = req.visibility;
        let ch_ctx = req.channel_context;
        let body_clone = req.body.clone();
        let roster_clone = ctx.roster.clone();
        tokio::spawn(async move {
            trigger_ai_gm_chat_reply(
                &state_clone,
                &sid_clone,
                session_mode,
                &ai_gm,
                &sender_clone,
                &msg_id_clone,
                "CHAT",
                vis,
                Some(ch_ctx),
                &body_clone,
                &roster_clone,
            )
            .await;
        });
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

    let pack_defaults =
        crate::campaign_lore::resolve_pack_defaults(Some(state.campaign_pack.as_str()));
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
                    .unwrap_or(&pack_defaults.default_ai_player_name),
                None,
                "ollama/gemma4:12b",
                "tactical-party-player",
            )
        });

    let n_pairs = req.skill_rank.clamp(1, 10);
    let control_c = req.attribute_tn.clamp(1, 20);
    let int_mod = req.internal_mod.clamp(-2, 10);
    let tech_delta_ext = evaluate_roll_combat_action_and_triad(
        req.combat_technique.as_deref(),
        req.tempo_pool,
        req.control_pool,
        req.force_pool,
        None,
        control_c,
        n_pairs,
        int_mod.max(0),
        0.0,
    )
    .map(|(res, _)| res.technique_delta_ext)
    .unwrap_or(0);
    let ext_mod = (req.difficulty_mod + tech_delta_ext).clamp(-10, 16);
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
    let base_note = format!(
        "cascade(tn_eff={}, signed_margin={:+}, {:?})",
        params.tn_eff(),
        outcome.signed_margin,
        outcome.outcome_kind
    );
    mech_ev.notes = match req
        .notes
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        Some(note) => Some(format!("{base_note} | {note}")),
        None => Some(base_note),
    };

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
    if let Ok(canvas_meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TABLETOP_CANVAS,
        1,
        sid,
        ctx.session_mode,
    ) {
        let canvas_ev = TabletopCanvasEvent {
            metadata: canvas_meta,
            canvas_event_type: CanvasEventType::WgpuDiceSpawned,
            actor: actor.clone(),
            map_id: ctx.active_map.as_ref().map(|m| m.map_id.clone()),
            map_title: ctx.active_map.as_ref().map(|m| m.title.clone()),
            image_url: ctx.active_map.as_ref().map(|m| m.image_url.clone()),
            grid_width: ctx.active_map.as_ref().map(|m| m.grid_width),
            grid_height: ctx.active_map.as_ref().map(|m| m.grid_height),
            token_placements: Vec::new(),
            wgpu_dice_roll: Some(dice_spec.clone()),
            terrain_notes: req.notes.clone(),
        };
        let _ = state.storage.log_tabletop_canvas(&canvas_ev).await;
    }
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
    let raw_target_url = entry
        .service_url
        .clone()
        .or_else(|| std::env::var("SATYA_AI_PLAYER_URL").ok())
        .unwrap_or_else(|| ai_cfg.service_url.clone());
    let target_url =
        match validate_ai_player_service_url(&raw_target_url, &ai_cfg.allowed_ai_player_service_urls)
        {
            Ok(u) => u,
            Err(err) => {
                tracing::error!(
                    session_id = %session_id,
                    player_user_id = %entry.participant.user_id,
                    "Blocked AI Player dispatch to untrusted service URL: {err}"
                );
                if let Some((ctx, _)) = state.auth.get_session_gm_context(session_id).await {
                    let msg = format!(
                        "[AI Player Blocked] Refused dispatch for {} due to untrusted service_url: {err}",
                        entry.participant.display_name
                    );
                    state
                        .broadcast_system_notice_to_humans(
                            session_id,
                            ctx.session_mode,
                            &entry.participant,
                            &ctx.roster,
                            &msg,
                        )
                        .await;
                }
                return;
            }
        };

    let (ctx_opt, vault_opt) = match state.auth.get_session_gm_context(session_id).await {
        Some((c, v)) => (Some(c), v),
        None => (None, None),
    };
    let ephemeral_key = vault_opt
        .as_ref()
        .filter(|v| v.provider.eq_ignore_ascii_case(&entry.provider))
        .and_then(|v| v.api_key.clone());
    let filtered_active_map = ctx_opt
        .as_ref()
        .and_then(|c| c.active_map.as_ref())
        .map(|m| crate::map_handlers::filter_map_for_role(m, SessionRole::Player));
    let filtered_tracker = ctx_opt
        .as_ref()
        .and_then(|c| c.combat_tracker.as_ref())
        .map(|t| {
            crate::auth::AuthAndSessionManager::filter_combat_tracker_for_viewer(
                t,
                ctx_opt.as_ref().and_then(|c| c.active_map.as_ref()),
                false,
            )
        });
    let bound_sheet = ctx_opt.as_ref().and_then(|c| {
        let target_cid = entry.participant.character_id.as_deref();
        c.session_characters
            .iter()
            .find(|s| {
                target_cid.is_some_and(|cid| s.character_id == cid)
                    || s.owner_user_id == entry.participant.user_id
            })
            .cloned()
    });

    let payload = json!({
        "session_id": session_id,
        "player_user_id": entry.participant.user_id,
        "player_display_name": entry.participant.display_name,
        "persona_prompt": entry.persona_prompt,
        "roster": ctx_opt.as_ref().map(|c| c.roster.clone()).unwrap_or_default(),
        "character_sheet": bound_sheet,
        "combat_tracker": filtered_tracker,
        "active_map": filtered_active_map,
        "content": prompt_text,
        "auto_trigger_gm": auto_trigger_gm,
        "provider": entry.provider,
        "model_id": entry.model_id,
    });

    let service_token = crate::gm_api::resolve_effective_gm_service_token(state).unwrap_or_default();
    let dispatch_res = if service_token.is_empty() {
        Err(anyhow::anyhow!("AI Player service token is not configured"))
    } else {
        post_ai_player_event_to_fastapi(
            &target_url,
            &service_token,
            ephemeral_key.as_deref(),
            &payload,
            ai_cfg.connect_timeout_ms,
            ai_cfg.request_timeout_secs,
        )
        .await
    };
    match dispatch_res {
        Ok(()) => return,
        Err(err) => {
            if ai_cfg.fail_fast_when_offline {
                tracing::error!(
                    session_id = %session_id,
                    player_user_id = %entry.participant.user_id,
                    "AI Player service at {target_url} failed ({err}; fail_fast_when_offline=true)"
                );
                if let Some(ctx) = ctx_opt.as_ref() {
                    let msg = format!(
                        "[AI Player Service Error] {} turn dispatch failed against {target_url} ({err}).",
                        entry.participant.display_name
                    );
                    state
                        .broadcast_system_notice_to_humans(
                            session_id,
                            ctx.session_mode,
                            &entry.participant,
                            &ctx.roster,
                            &msg,
                        )
                        .await;
                }
                return;
            }
        }
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
    let base = service_url.trim().trim_end_matches('/');
    let endpoint = if base.starts_with("http://") || base.starts_with("https://") {
        format!("{base}/internal/ai-player/events")
    } else {
        format!("http://{base}/internal/ai-player/events")
    };

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_millis(connect_timeout_ms.max(100)))
        .timeout(std::time::Duration::from_secs(request_timeout_secs.max(1)))
        .build()?;

    let mut req_builder = client
        .post(&endpoint)
        .bearer_auth(service_token.trim())
        .json(payload);
    if let Some(k) = ephemeral_api_key.filter(|s| !s.trim().is_empty()) {
        req_builder = req_builder.header("X-Satya-Provider-Key", k.trim());
    }

    let resp = req_builder.send().await.map_err(|err| {
        if err.is_timeout() {
            anyhow::anyhow!("Timed out awaiting AI Player response from {endpoint}")
        } else {
            anyhow::anyhow!("Connection to AI Player service at {endpoint} failed: {err}")
        }
    })?;

    if !resp.status().is_success() {
        anyhow::bail!(
            "AI Player service at {endpoint} returned non-2xx response: {}",
            resp.status()
        );
    }
    Ok(())
}
