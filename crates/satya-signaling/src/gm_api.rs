//! Backend AI Game Master REST endpoints (`/api/sessions/:id/gm-config`, `/api/gm/sessions/:id/*`),
//! Co-Pilot vs. Autonomous trigger filters, and FastAPI LangGraph turn dispatch.

use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use rand::Rng;
use satya_engine::{
    ActionTimeFactorParams, CascadeTaskParams, CombatDomain, CombatTechniqueId,
    ContestedCascadeTaskParams, ContestedTieRule, GmResistanceMode, TacticalPoolEnvelope,
    TacticalPoolParams, TechniqueTriadEvalParams, TechniqueTriadResolution, Triad,
    compute_tactical_pool, evaluate_satya_cascade_task, evaluate_satya_contested_cascade_task,
    evaluate_technique_with_triad,
};
use satya_schema::{
    AiGmOperatingMode, AiGmProviderConfig, AiGmTrajectoryEvent, AiGmTurnStatus,
    AiGmTurnTriggerRequest, CanvasEventType, ChannelContext, ChatMessageEvent, CombatStance,
    GmChatRequest, GmMapUpdateRequest, GmRollRequest, LogLevel, McpToolInvocationRecord,
    MechanicEventType, MessageVisibility, Participant, ParticipantKind, ServerWsPacket,
    SessionLogEvent, SessionMode, SessionRole, TabletopCanvasEvent, TtrpgMechanicsEvent,
    UpdateCombatTrackerRequest, WgpuDiceSpec, subjects,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) fn verify_gm_service_header(
    state: &SignalingAppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let Some(auth_hdr) = headers.get("Authorization").and_then(|h| h.to_str().ok()) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing Authorization header for GM service" })),
        ));
    };
    let clean = auth_hdr.strip_prefix("Bearer ").unwrap_or(auth_hdr).trim();
    let expected = state.storage.config().ai_gm.service_token.trim();
    if clean.is_empty() || clean != expected {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid AI GM service_token" })),
        ));
    }
    Ok(())
}

/// Configures or updates ephemeral BYOK AI GM provider credentials at `POST /api/sessions/:session_id/gm-config`.
pub async fn update_session_gm_config_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<AiGmProviderConfig>,
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

    match state
        .auth
        .configure_session_ai_gm(&session_id, &user.user_id, req)
        .await
    {
        Ok((ai_participant, roster)) => {
            if let Ok(sync_msg) = serde_json::to_string(&ServerWsPacket::RosterSync {
                roster: roster.clone(),
            }) {
                state.broadcast_to_session(&session_id, &sync_msg).await;
            }
            (
                StatusCode::OK,
                Json(json!({
                    "status": "configured",
                    "session_id": session_id,
                    "ai_gm_participant": ai_participant,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Returns active session roster, operating mode, and tactical map state for the AI GM at `GET /api/gm/sessions/:session_id/context`.
pub async fn gm_session_context_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };
    (StatusCode::OK, Json(ctx)).into_response()
}

/// Emits a public narration or private whisper from the AI GM at `POST /api/gm/sessions/:session_id/chat`.
pub async fn gm_session_chat_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<GmChatRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };

    let default_cfg = &state.storage.config().ai_gm;
    let mut sender = ctx.ai_gm_participant.unwrap_or_else(|| {
        Participant::ai_gm(
            &session_id,
            &format!(
                "{}/{}",
                default_cfg.default_provider, default_cfg.reasoning_model
            ),
            &default_cfg.persona_id,
        )
    });
    if let Some(speaker) = req
        .speaker_override
        .as_ref()
        .filter(|s| !s.trim().is_empty())
    {
        sender.display_name = speaker.trim().to_string();
    }
    if let Some(mid) = req.model_id.as_ref().filter(|s| !s.trim().is_empty())
        && let Some(info) = sender.ai_agent_info.as_mut()
    {
        info.model_id = mid.trim().to_string();
    }

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
        &session_id,
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
        sender,
        visibility: req.visibility,
        recipients,
        channel_context: req.channel_context,
        reply_to_message_id: None,
        body: req.body.clone(),
        char_count: req.body.chars().count() as i32,
        word_count: req.body.split_whitespace().count() as i32,
        composition_time_ms: 50,
        is_edited: false,
    };

    let _ = state.storage.log_chat_message(&chat_event).await;
    state
        .auth
        .record_chat_event_to_db(&session_id, &chat_event)
        .await;
    let pkt_str = json!({
        "type": "CHAT_EVENT",
        "event": chat_event
    })
    .to_string();
    state
        .send_to_users(&session_id, &delivery_ids, &pkt_str)
        .await;

    (StatusCode::OK, Json(chat_event)).into_response()
}

/// Evaluates an authoritative Satya d20 check and spawns 3D `wgpu` dice at `POST /api/gm/sessions/:session_id/roll`.
pub async fn gm_session_roll_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<GmRollRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };

    let default_cfg = &state.storage.config().ai_gm;
    let mut actor = ctx.ai_gm_participant.unwrap_or_else(|| {
        Participant::ai_gm(
            &session_id,
            &format!(
                "{}/{}",
                default_cfg.default_provider, default_cfg.reasoning_model
            ),
            &default_cfg.persona_id,
        )
    });
    if !req.actor_display_name.trim().is_empty() {
        actor.display_name = req.actor_display_name.trim().to_string();
    }

    let target_participants: Vec<Participant> = ctx
        .roster
        .iter()
        .filter(|p| req.target_user_ids.contains(&p.user_id))
        .cloned()
        .collect();

    let n_pairs = req.pool_size.clamp(1, 10);
    let c_val = req.attribute_tn.clamp(1, 20);
    let int_mod = req.internal_mod.unwrap_or(req.fatigue_level).clamp(-2, 10);
    let tool_mod = req.known_tool_mod.clamp(-4, 12);
    let ext_mod = req.difficulty_rating.clamp(-10, 16);
    let parsed_tie_rule = match req.tie_rule.as_deref() {
        Some("strict_signed_margin") => ContestedTieRule::StrictSignedMargin,
        Some("active_player_wins_ties") => ContestedTieRule::ActivePlayerWinsTies,
        _ => ContestedTieRule::HybridSignedMargin,
    };
    let is_uncontested = matches!(
        req.gm_resistance_mode.as_deref(),
        Some("uncontested" | "none")
    );
    let mut contested_params = ContestedCascadeTaskParams::new_tiered_environment(
        n_pairs,
        c_val,
        int_mod,
        tool_mod,
        ext_mod,
        parsed_tie_rule,
    );
    if matches!(req.gm_resistance_mode.as_deref(), Some("full_cascade")) {
        contested_params.gm_mode = GmResistanceMode::FullCascade;
    }
    if let Some(d_over) = req.delta_over {
        contested_params.player.delta_over = d_over.clamp(2, 12);
    }
    if let Some(mode_str) = req.carry_mode.as_deref()
        && let Ok(parsed_mode) = mode_str.parse::<satya_engine::CarryMode>()
    {
        contested_params.player.carry_mode = parsed_mode;
    }
    if let Some(botch_str) = req.botch_rule.as_deref()
        && let Ok(parsed_botch) = botch_str.parse::<satya_engine::BotchRule>()
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

    let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TTRPG_MECHANICS,
        1,
        &session_id,
        ctx.session_mode,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to build EventMetadata" })),
        )
            .into_response();
    };

    let (mut mech_ev, base_note) = if is_uncontested {
        let mut params = CascadeTaskParams::new_k20(n_pairs, c_val, int_mod, tool_mod + ext_mod);
        params.carry_mode = contested_params.player.carry_mode;
        params.botch_rule = contested_params.player.botch_rule;
        params.delta_over = contested_params.player.delta_over;
        let (ev, outcome) = evaluate_satya_cascade_task(
            meta,
            actor.clone(),
            target_participants,
            req.skill_name.clone(),
            &params,
            &rolled_pairs,
            req.combat_stance,
            req.fatigue_level.max(0),
            req.wound_shock_index,
        );
        let note = format!(
            "cascade(mode={}, botch={}, tn_eff={}, signed_margin={:+}, {:?})",
            params.carry_mode,
            params.botch_rule,
            params.tn_eff(),
            outcome.signed_margin,
            outcome.outcome_kind
        );
        (ev, note)
    } else {
        let (ev, outcome) = evaluate_satya_contested_cascade_task(
            meta,
            actor.clone(),
            target_participants,
            req.skill_name.clone(),
            &contested_params,
            &rolled_pairs,
            &gm_pairs,
            req.combat_stance,
            req.fatigue_level.max(0),
            req.wound_shock_index,
        );
        let note = format!(
            "contested(kind={}, tn_p={}, tn_gm={}, s_p={}, s_gm={}, net_margin={:+}) | cascade(mode={}, botch={}, tn_eff={}, signed_margin={:+})",
            outcome.contested_kind,
            contested_params.player.tn_eff(),
            contested_params.gm.tn_eff(),
            outcome.player_outcome.total_successes,
            outcome.gm_outcome.total_successes,
            outcome.net_signed_margin,
            contested_params.player.carry_mode,
            contested_params.player.botch_rule,
            contested_params.player.tn_eff(),
            outcome.player_outcome.signed_margin
        );
        (ev, note)
    };
    mech_ev.mechanic_type = MechanicEventType::AiGmDecision;
    mech_ev.notes = match req
        .notes
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        Some(user_note) => Some(format!("{base_note} | {user_note}")),
        None => Some(base_note),
    };

    apply_combat_action_to_mechanics_and_tracker(
        &state,
        &session_id,
        &actor,
        req.combat_technique.as_deref(),
        req.tempo_pool,
        req.control_pool,
        req.force_pool,
        req.combat_stance,
        c_val,
        n_pairs,
        req.fatigue_level.max(0),
        req.wound_shock_index,
        &mut mech_ev,
    )
    .await;

    let _ = state.storage.log_ttrpg_mechanics(&mech_ev).await;
    state
        .auth
        .record_mechanics_event_to_db(&session_id, &mech_ev)
        .await;

    // Also emit Topic 7 (`tabletop-canvas`) WgpuDiceSpawned event & broadcast 3D dice seed
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
            terrain_notes: req.notes,
        };
        let _ = state.storage.log_tabletop_canvas(&canvas_ev).await;
    }

    if let Ok(wgpu_pkt) = serde_json::to_string(&ServerWsPacket::WgpuDiceRoll {
        actor,
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

    (StatusCode::OK, Json(mech_ev)).into_response()
}

/// Updates the active tactical map and token coordinates at `POST /api/gm/sessions/:session_id/map`.
pub async fn gm_session_map_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(mut req): Json<GmMapUpdateRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };

    if req.cell_size_cm <= 0 {
        req.cell_size_cm = 100;
    }
    if !req.tiles.is_empty() {
        satya_engine::tactical_map_geom::migrate_legacy_tiles_to_vector(
            &req.tiles,
            req.grid_width.max(1),
            req.grid_height.max(1),
            &mut req.wall_edges,
            &mut req.props,
        );
        req.tiles.clear();
    } else if req.wall_edges.is_empty() && req.props.is_empty() && req.grid_width >= 4 && req.grid_height >= 4 {
        let preset = req
            .theme_preset
            .as_deref()
            .or(Some(req.title.as_str()))
            .and_then(satya_engine::tactical_map::SectorPreset::from_code_str)
            .unwrap_or_default();
        let sec = satya_engine::tactical_map::generate_procedural_sector(
            preset,
            req.grid_width,
            req.grid_height,
            req.seed.unwrap_or(42),
        );
        req.wall_edges = sec.wall_edges;
        req.props = sec.props;
    }

    let _ = state
        .auth
        .set_session_active_map(&session_id, req.clone())
        .await;

    let default_cfg = &state.storage.config().ai_gm;
    let actor = ctx.ai_gm_participant.unwrap_or_else(|| {
        Participant::ai_gm(
            &session_id,
            &format!(
                "{}/{}",
                default_cfg.default_provider, default_cfg.reasoning_model
            ),
            &default_cfg.persona_id,
        )
    });

    let Ok(canvas_meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::TABLETOP_CANVAS,
        1,
        &session_id,
        ctx.session_mode,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to build EventMetadata" })),
        )
            .into_response();
    };

    let canvas_ev = TabletopCanvasEvent {
        metadata: canvas_meta,
        canvas_event_type: if ctx.active_map.is_some() {
            CanvasEventType::MapUpdated
        } else {
            CanvasEventType::MapGenerated
        },
        actor,
        map_id: Some(req.map_id.clone()),
        map_title: Some(req.title.clone()),
        image_url: Some(req.image_url.clone()),
        grid_width: Some(req.grid_width),
        grid_height: Some(req.grid_height),
        token_placements: req.tokens.clone(),
        wgpu_dice_roll: None,
        terrain_notes: req.notes.clone(),
    };
    let _ = state.storage.log_tabletop_canvas(&canvas_ev).await;

    crate::map_handlers::broadcast_role_filtered_map(&state, &session_id, &ctx.roster, &req).await;

    (StatusCode::OK, Json(canvas_ev)).into_response()
}

/// Single turn-scoped log entry piggybacked onto [`GmTrajectoryReportRequest`] for batch ingestion into `system-logs` (Topic 9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GmSessionLogEntry {
    /// Log severity level (`INFO`, `WARN`, `ERROR`, etc.).
    pub level: LogLevel,
    /// Optional 16-character hex W3C `span_id`.
    #[serde(default)]
    pub span_id: Option<String>,
    /// Optional 16-character hex caller `parent_span_id`.
    #[serde(default)]
    pub parent_span_id: Option<String>,
    /// Optional data artifact lineage ID produced or inspected at this step.
    #[serde(default)]
    pub lineage_id: Option<String>,
    /// Upstream data artifact lineage IDs consumed at this step.
    #[serde(default)]
    pub parent_lineage_ids: Vec<String>,
    /// Complete formatted plain-text log line prefixed with `[session=...] [trace=... span=...<-...] [lineage=...<-...]`.
    pub log_line: String,
}

/// Request payload for `POST /api/gm/sessions/:session_id/trajectory`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmTrajectoryReportRequest {
    /// Unique UUIDv7 identifier for the completed LangGraph turn.
    pub turn_id: String,
    /// 32-character hex W3C `trace_id`.
    pub trace_id: String,
    /// Operating mode (`AUTONOMOUS` or `CO_PILOT`).
    pub operating_mode: AiGmOperatingMode,
    /// Triggering participant.
    pub trigger_actor: Participant,
    /// `event_id` or `message_id` that triggered the turn.
    pub trigger_event_id: String,
    /// Summary of the triggering action.
    pub trigger_summary: String,
    /// LLM provider identifier.
    pub llm_provider: String,
    /// Reasoning model ID.
    pub reasoning_model_id: String,
    /// Optional vision model ID.
    #[serde(default)]
    pub vision_model_id: Option<String>,
    /// Ruleset engine ID (e.g. `"satya-d20"`).
    #[serde(default = "default_ruleset_id")]
    pub ruleset_engine_id: String,
    /// Ordered tool invocations executed during the turn.
    #[serde(default)]
    pub tool_invocations: Vec<McpToolInvocationRecord>,
    /// RAG/lore sources consulted.
    #[serde(default)]
    pub rag_sources_used: Vec<String>,
    /// Emitted event IDs.
    #[serde(default)]
    pub emitted_event_ids: Vec<String>,
    /// Prompt token count.
    #[serde(default)]
    pub prompt_tokens: i32,
    /// Completion token count.
    #[serde(default)]
    pub completion_tokens: i32,
    /// Cached token count.
    #[serde(default)]
    pub cached_tokens: i32,
    /// Total turn latency in milliseconds.
    #[serde(default)]
    pub total_turn_latency_ms: i64,
    /// Completion status of the turn.
    pub turn_status: AiGmTurnStatus,
    /// Bounded batch of turn-scoped `INFO`/`WARN`/`ERROR` log entries to persist to `system-logs` (Topic 9).
    #[serde(default)]
    pub turn_logs: Vec<GmSessionLogEntry>,
}

fn default_ruleset_id() -> String {
    "satya-d20".to_string()
}

/// Persists a completed LangGraph AI GM turn trajectory to `ai-gm-trajectories` at `POST /api/gm/sessions/:session_id/trajectory`.
pub async fn gm_session_trajectory_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<GmTrajectoryReportRequest>,
) -> impl IntoResponse {
    if let Err(err_resp) = verify_gm_service_header(&state, &headers) {
        return err_resp.into_response();
    }
    let Some((ctx, _vault)) = state.auth.get_session_gm_context(&session_id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Session '{session_id}' not found") })),
        )
            .into_response();
    };

    let default_cfg = &state.storage.config().ai_gm;
    let ai_participant = ctx.ai_gm_participant.unwrap_or_else(|| {
        Participant::ai_gm(
            &session_id,
            &format!("{}/{}", req.llm_provider, req.reasoning_model_id),
            &default_cfg.persona_id,
        )
    });

    let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::AI_GM_TRAJECTORY,
        1,
        &session_id,
        ctx.session_mode,
    ) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to build EventMetadata" })),
        )
            .into_response();
    };

    let trigger_actor = req.trigger_actor.clone();
    let trace_id = req.trace_id.clone();
    let turn_logs = req.turn_logs;

    let traj_ev = AiGmTrajectoryEvent {
        metadata: meta,
        turn_id: req.turn_id,
        trace_id: req.trace_id,
        operating_mode: req.operating_mode,
        ai_participant,
        trigger_actor: req.trigger_actor,
        trigger_event_id: req.trigger_event_id,
        trigger_summary: req.trigger_summary,
        llm_provider: req.llm_provider,
        reasoning_model_id: req.reasoning_model_id,
        vision_model_id: req.vision_model_id,
        ruleset_engine_id: req.ruleset_engine_id,
        tool_invocations: req.tool_invocations,
        rag_sources_used: req.rag_sources_used,
        emitted_event_ids: req.emitted_event_ids,
        prompt_tokens: req.prompt_tokens,
        completion_tokens: req.completion_tokens,
        cached_tokens: req.cached_tokens,
        total_turn_latency_ms: req.total_turn_latency_ms,
        turn_status: req.turn_status,
    };

    let _ = state.storage.log_ai_gm_trajectory(&traj_ev).await;

    // Persist bounded turn-scoped log lines into Topic 9 (`system-logs`)
    for entry in turn_logs.into_iter().take(64) {
        if let Ok(log_meta) = state.storage.registry().build_metadata(
            state.next_seq(),
            subjects::SESSION_LOG,
            1,
            &session_id,
            ctx.session_mode,
        ) {
            let log_ev = SessionLogEvent {
                metadata: log_meta,
                log_id: Uuid::now_v7().to_string(),
                level: entry.level,
                service_name: "satya-ml-ai-gm".to_string(),
                actor: Some(trigger_actor.clone()),
                trace_id: Some(trace_id.clone()),
                span_id: entry.span_id,
                parent_span_id: entry.parent_span_id,
                lineage_id: entry.lineage_id,
                parent_lineage_ids: entry.parent_lineage_ids,
                log_line: entry.log_line,
            };
            let _ = state.storage.log_session_log(&log_ev).await;
        }
    }

    (StatusCode::OK, Json(traj_ev)).into_response()
}

/// Evaluates a combat technique and Tactical Triad (`Tempo` / `Control` / `Force`) allocation
/// when a roll specifies a `combat_technique`, `tempo_pool` / `control_pool` / `force_pool`, or `combat_stance`.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_roll_combat_action_and_triad(
    combat_technique: Option<&str>,
    tempo_pool: Option<i32>,
    control_pool: Option<i32>,
    force_pool: Option<i32>,
    combat_stance: Option<CombatStance>,
    skill_c: i32,
    skill_n: usize,
    fatigue_level: i32,
    wound_shock_index: f32,
) -> Option<(TechniqueTriadResolution, TacticalPoolEnvelope)> {
    let raw_tech = combat_technique.map(str::trim).filter(|s| {
        !s.is_empty() && !s.eq_ignore_ascii_case("non_combat") && !s.eq_ignore_ascii_case("none")
    });
    let has_triad = tempo_pool.is_some() || control_pool.is_some() || force_pool.is_some();
    if raw_tech.is_none() && !has_triad && combat_stance.is_none() {
        return None;
    }

    let tech_id = raw_tech
        .and_then(CombatTechniqueId::from_code_str)
        .unwrap_or(match combat_stance {
            Some(CombatStance::Power) => CombatTechniqueId::Swing,
            Some(CombatStance::Maneuver) => CombatTechniqueId::Probe,
            Some(CombatStance::Stability) => CombatTechniqueId::Parry,
            Some(CombatStance::RangedAimed) => CombatTechniqueId::DeliberateAimedShot,
            Some(CombatStance::RangedSnap) => CombatTechniqueId::SnapShot,
            None => CombatTechniqueId::Punch,
        });
    let spec = tech_id.spec();
    let domain = if spec.allowed_ranged && !spec.allowed_unarmed {
        CombatDomain::Ranged
    } else if spec.allowed_armed_melee && !spec.allowed_unarmed {
        CombatDomain::ArmedMelee
    } else {
        CombatDomain::UnarmedMartialArts
    };

    let mut pool_params = TacticalPoolParams {
        skill_c: skill_c.clamp(1, 20),
        skill_n: skill_n.clamp(1, 10),
        is_ranged: domain == CombatDomain::Ranged,
        fatigue_level: fatigue_level.max(0),
        wound_shock_index: wound_shock_index.round() as i32,
        ..Default::default()
    };
    pool_params.chars.spd = skill_c.clamp(6, 18);
    pool_params.chars.for_stat = skill_c.clamp(6, 18);
    pool_params.chars.dex = skill_c.clamp(6, 18);
    pool_params.chars.ins = skill_c.clamp(6, 18);
    pool_params.chars.foc = skill_c.clamp(6, 18);
    let envelope = compute_tactical_pool(&pool_params);

    let b = envelope.neutral_baseline_b;
    let mut t = tempo_pool.unwrap_or(b).clamp(0, envelope.tempo_cap);
    let mut c = control_pool.unwrap_or(b).clamp(0, envelope.control_cap);
    let mut f = force_pool.unwrap_or(b).clamp(0, envelope.force_cap);
    if !has_triad {
        while t + c + f > envelope.p_eff && (t > 0 || c > 0 || f > 0) {
            if f >= c && f >= t && f > 0 {
                f -= 1;
            } else if c >= t && c > 0 {
                c -= 1;
            } else if t > 0 {
                t -= 1;
            }
        }
    }

    let eval_params = TechniqueTriadEvalParams {
        technique: tech_id,
        domain,
        envelope: envelope.clone(),
        alloc: Triad::new(t, c, f),
        skill_level_n: skill_n.clamp(1, 10),
        time_factor_params: ActionTimeFactorParams {
            fatigue_level: fatigue_level.max(0),
            wound_shock_index: f64::from(wound_shock_index.max(0.0)),
            ..Default::default()
        },
        weapon_recoil_r: if domain == CombatDomain::Ranged { 4 } else { 0 },
        ..Default::default()
    };
    let res = evaluate_technique_with_triad(&eval_params);
    Some((res, envelope))
}

/// Applies a resolved [`TechniqueTriadResolution`] to a [`TtrpgMechanicsEvent`], activates/advances
/// the session's Action Mode [`CombatTrackerState`], and synchronizes the Tactical Map stub.
#[allow(clippy::too_many_arguments)]
pub async fn apply_combat_action_to_mechanics_and_tracker(
    state: &Arc<SignalingAppState>,
    session_id: &str,
    actor: &Participant,
    combat_technique: Option<&str>,
    tempo_pool: Option<i32>,
    control_pool: Option<i32>,
    force_pool: Option<i32>,
    combat_stance: Option<CombatStance>,
    skill_c: i32,
    skill_n: usize,
    fatigue_level: i32,
    wound_shock_index: f32,
    mech_ev: &mut TtrpgMechanicsEvent,
) {
    let Some((res, env)) = evaluate_roll_combat_action_and_triad(
        combat_technique,
        tempo_pool,
        control_pool,
        force_pool,
        combat_stance,
        skill_c,
        skill_n,
        fatigue_level,
        wound_shock_index,
    ) else {
        return;
    };

    if mech_ev.mechanic_type == MechanicEventType::TaskResolution {
        mech_ev.mechanic_type = MechanicEventType::CombatAction;
    }
    mech_ev.action_time_cost = Some(res.action_cost.base_total_tc);
    mech_ev.action_count_cost = Some(res.effective_tc);

    let cur_tick = state
        .auth
        .get_session_combat_tracker(session_id)
        .await
        .map(|t| t.tick.max(1))
        .unwrap_or(1);
    let next_tick = cur_tick + (res.effective_tc.max(1) as u32);
    let stance_label = format!(
        "{} (T{}/C{}/F{})",
        res.technique.as_str().to_uppercase(),
        res.alloc.tempo,
        res.alloc.control,
        res.alloc.force
    );

    if let Ok(_tracker) = state
        .auth
        .update_session_combat_tracker(
            session_id,
            UpdateCombatTrackerRequest {
                tick_delta: None,
                combat_active: Some(true),
                entity_id: Some(actor.user_id.clone()),
                active_stance: Some(stance_label),
                next_action_tick: Some(next_tick),
                fatigue_level: Some(fatigue_level.max(0)),
                wound_shock: Some(wound_shock_index.round() as i32),
                grid_x: None,
                grid_y: None,
                zone_label: None,
            },
        )
        .await
        && let Some((ctx, _)) = state.auth.get_session_gm_context(session_id).await
        && let Some(map) = ctx.active_map
        && let Ok(map_pkt) =
            serde_json::to_string(&ServerWsPacket::MapUpdated { map: Box::new(map) })
    {
        state.broadcast_to_session(session_id, &map_pkt).await;
    }

    let triad_note = format!(
        "combat_action(tech={}, cat={}, triad=T:{}/C:{}/F:{} [P_eff={}/P_base={}, B={}], tc={}->{}_ticks, next_tick={}, guard={}, root={:.1}, impact={:.1}, tech_delta_ext={:+}, exertion=+{})",
        res.technique.as_str(),
        res.category.as_str(),
        res.alloc.tempo,
        res.alloc.control,
        res.alloc.force,
        env.p_eff,
        env.p_base,
        env.neutral_baseline_b,
        res.action_cost.base_total_tc,
        res.effective_tc,
        next_tick,
        res.effective_guard,
        res.effective_root,
        res.strike_impact,
        res.technique_delta_ext,
        res.exertion_marks_gained
    );
    mech_ev.notes = match mech_ev.notes.take() {
        Some(existing) if !existing.is_empty() => Some(format!("{existing} | {triad_note}")),
        _ => Some(triad_note),
    };
}

/// Evaluates whether a human chat message should trigger the AI GM according to the session's
/// operating topology (`TTRPG_GM_AI` Co-Pilot vs. `TTRPG_AI_GM` Autonomous):
/// - **Co-Pilot (`TTRPG_GM_AI`, human `SessionRole::Gm` present)**: Only triggers when directly addressed
///   (`@satya_ai_gm` / `@ai_gm`) or when a `GM_ONLY` or `PRIVATE` message targets the AI GM.
/// - **Autonomous (`TTRPG_AI_GM`, no human `SessionRole::Gm`)**: Triggers on all `IN_CHARACTER` and
///   `OUT_OF_CHARACTER` messages, `GM_ONLY` whispers, `PRIVATE` messages targeting the AI GM, or messages mentioning `gm`.
pub fn should_trigger_ai_gm_for_chat(
    roster: &[Participant],
    ai_gm: &Participant,
    visibility: MessageVisibility,
    recipient_user_ids: &[String],
    channel_context: ChannelContext,
    body: &str,
) -> bool {
    let has_human_gm = roster
        .iter()
        .any(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm);
    let lower = body.to_lowercase();
    let targets_ai_privately =
        visibility == MessageVisibility::Private && recipient_user_ids.contains(&ai_gm.user_id);
    let directly_mentions_ai = lower.contains("@satya_ai_gm") || lower.contains("@ai_gm");

    if has_human_gm {
        // Co-Pilot Mode (`TTRPG_GM_AI`): do not interrupt public player chat unless explicitly addressed
        visibility == MessageVisibility::GmOnly || targets_ai_privately || directly_mentions_ai
    } else {
        // Autonomous Solo AI GM Mode (`TTRPG_AI_GM`): reply to both IN_CHARACTER actions and OUT_OF_CHARACTER rules/table questions
        visibility == MessageVisibility::GmOnly
            || targets_ai_privately
            || channel_context == ChannelContext::InCharacter
            || channel_context == ChannelContext::OutOfCharacter
            || lower.contains("gm")
    }
}

/// Evaluates whether a `RollTtrpgTask` event should automatically trigger an AI GM narration turn.
/// In `TTRPG_AI_GM` (Autonomous mode, no human GM), the AI GM adjudicates every roll automatically;
/// in `TTRPG_GM_AI` (Co-Pilot mode, human GM present), the human GM adjudicates public player rolls.
pub fn should_trigger_ai_gm_for_roll(roster: &[Participant]) -> bool {
    !roster
        .iter()
        .any(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm)
}

/// Dispatches an [`AiGmTurnTriggerRequest`] over HTTP/1.1 to the Python FastAPI LangGraph service
/// at `{service_url}/internal/ai-gm/events` using default timeout settings (`3000` ms connect, `180` s response),
/// passing the ephemeral BYOK key in `X-Satya-Provider-Key` if present.
pub async fn post_ai_gm_event_to_fastapi(
    service_url: &str,
    service_token: &str,
    ephemeral_api_key: Option<&str>,
    payload: &AiGmTurnTriggerRequest,
) -> anyhow::Result<()> {
    post_ai_gm_event_to_fastapi_with_timeouts(
        service_url,
        service_token,
        ephemeral_api_key,
        payload,
        3_000,
        180,
    )
    .await
}

/// Dispatches an [`AiGmTurnTriggerRequest`] over HTTP/1.1 to the Python FastAPI LangGraph service
/// at `{service_url}/internal/ai-gm/events` with configurable TCP connection (`connect_timeout_ms`)
/// and response read (`request_timeout_secs`) timeouts.
///
/// Either timeout can also be overridden at runtime via the `SATYA_AI_GM_CONNECT_TIMEOUT_MS`
/// and `SATYA_AI_GM_REQUEST_TIMEOUT_SECS` environment variables.
pub async fn post_ai_gm_event_to_fastapi_with_timeouts(
    service_url: &str,
    service_token: &str,
    ephemeral_api_key: Option<&str>,
    payload: &AiGmTurnTriggerRequest,
    connect_timeout_ms: u64,
    request_timeout_secs: u64,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let effective_connect_ms = std::env::var("SATYA_AI_GM_CONNECT_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(connect_timeout_ms)
        .max(100);
    let effective_request_secs = std::env::var("SATYA_AI_GM_REQUEST_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(request_timeout_secs)
        .max(1);

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
    if let (Some(tid), Some(psid)) = (
        payload.trace_id.as_deref().filter(|s| !s.is_empty()),
        payload.parent_span_id.as_deref().filter(|s| !s.is_empty()),
    ) {
        extra_hdrs.push_str(&format!("traceparent: 00-{tid}-{psid}-01\r\n"));
    }
    if let Some(lid) = payload
        .lineage_id
        .as_deref()
        .or(Some(payload.trigger_event_id.as_str()))
        .filter(|s| !s.is_empty())
    {
        extra_hdrs.push_str(&format!("X-Satya-Lineage-Id: {lid}\r\n"));
    }

    let req_bytes = format!(
        "POST /internal/ai-gm/events HTTP/1.1\r\n\
         Host: {host_port}\r\n\
         Authorization: Bearer {service_token}\r\n\
         {extra_hdrs}\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body_bytes.len()
    );

    let mut stream = tokio::time::timeout(
        std::time::Duration::from_millis(effective_connect_ms),
        tokio::net::TcpStream::connect(&connect_addr),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Connection to AI GM service at {connect_addr} timed out"))??;

    stream.write_all(req_bytes.as_bytes()).await?;
    stream.write_all(&body_bytes).await?;
    stream.flush().await?;

    let mut resp_buf = vec![0u8; 512];
    let n = tokio::time::timeout(
        std::time::Duration::from_secs(effective_request_secs),
        stream.read(&mut resp_buf),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "Timed out after {effective_request_secs}s awaiting response from AI GM service at {connect_addr}"
        )
    })??;

    let status_line = String::from_utf8_lossy(&resp_buf[..n]);
    if !status_line.starts_with("HTTP/1.1 2") && !status_line.starts_with("HTTP/1.0 2") {
        anyhow::bail!(
            "AI GM service at {connect_addr} returned non-2xx response: {}",
            status_line.lines().next().unwrap_or("empty response")
        );
    }

    Ok(())
}

/// Triggers an AI Game Master turn for `session_id`:
/// 1. Attempts to dispatch the turn to the Python FastAPI LangGraph service (`POST /internal/ai-gm/events`).
/// 2. If the FastAPI service is offline:
///    - When `ai_gm.fail_fast_when_offline == true` (end-to-end runtime configuration in `config/ai.toml`),
///      logs an error to `system-logs` (`SessionLogEvent`) and emits a fail-fast `SYSTEM_NOTICE` error to the session.
///    - When `ai_gm.fail_fast_when_offline == false` (`StorageConfig::default()` used in unit/integration tests),
///      executes the deterministic in-process reply so tests run deterministically without an external daemon.
#[allow(clippy::too_many_arguments)]
pub async fn trigger_ai_gm_chat_reply(
    state: &Arc<SignalingAppState>,
    session_id: &str,
    session_mode: SessionMode,
    ai_gm: &Participant,
    triggering_player: &Participant,
    trigger_event_id: &str,
    trigger_kind: &str,
    visibility: MessageVisibility,
    channel_context: Option<ChannelContext>,
    context_text: &str,
    roster: &[Participant],
) {
    let ai_cfg = state.storage.config().ai_gm.clone();
    if !ai_cfg.enabled {
        return;
    }

    let (ctx_opt, vault_opt) = match state.auth.get_session_gm_context(session_id).await {
        Some((c, v)) => (Some(c), v),
        None => (None, None),
    };

    let human_gm_user_id = ctx_opt
        .as_ref()
        .and_then(|c| c.human_gm_user_id.clone())
        .or_else(|| {
            roster
                .iter()
                .find(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm)
                .map(|p| p.user_id.clone())
        });
    let operating_mode = if human_gm_user_id.is_some() {
        AiGmOperatingMode::CoPilot
    } else {
        AiGmOperatingMode::Autonomous
    };

    let provider = vault_opt
        .as_ref()
        .map(|v| v.provider.clone())
        .unwrap_or_else(|| ai_cfg.default_provider.clone());
    let model_id = vault_opt
        .as_ref()
        .map(|v| v.model_id.clone())
        .unwrap_or_else(|| ai_cfg.reasoning_model.clone());
    let vision_model_id = vault_opt
        .as_ref()
        .and_then(|v| v.vision_model_id.clone())
        .or_else(|| Some(ai_cfg.vision_model.clone()));
    let temperature = vault_opt
        .as_ref()
        .map(|v| v.temperature)
        .unwrap_or(ai_cfg.temperature);
    let persona_id = vault_opt
        .as_ref()
        .map(|v| v.persona_id.clone())
        .unwrap_or_else(|| ai_cfg.persona_id.clone());
    let base_url = vault_opt
        .as_ref()
        .and_then(|v| v.base_url.clone())
        .or_else(|| {
            if provider == "ollama" {
                Some(ai_cfg.ollama_base_url.clone())
            } else {
                None
            }
        });
    let ephemeral_key = vault_opt.as_ref().and_then(|v| v.api_key.clone());
    let trace_id = Uuid::now_v7().simple().to_string();
    let root_span_id = Uuid::now_v7().simple().to_string()[..16].to_string();

    let trigger_payload = AiGmTurnTriggerRequest {
        session_id: session_id.to_string(),
        session_mode,
        operating_mode,
        human_gm_user_id: human_gm_user_id.clone(),
        trigger_event_id: trigger_event_id.to_string(),
        trigger_kind: trigger_kind.to_string(),
        trigger_actor: triggering_player.clone(),
        visibility,
        channel_context,
        context_text: context_text.to_string(),
        roster: roster.to_vec(),
        provider,
        model_id,
        vision_model_id,
        temperature,
        persona_id,
        base_url,
        trace_id: Some(trace_id.clone()),
        parent_span_id: Some(root_span_id.clone()),
        lineage_id: Some(trigger_event_id.to_string()),
    };

    match post_ai_gm_event_to_fastapi_with_timeouts(
        &ai_cfg.service_url,
        &ai_cfg.service_token,
        ephemeral_key.as_deref(),
        &trigger_payload,
        ai_cfg.connect_timeout_ms,
        ai_cfg.request_timeout_secs,
    )
    .await
    {
        Ok(()) => return,
        Err(err) => {
            if ai_cfg.fail_fast_when_offline {
                let err_str = err.to_string();
                let is_response_timeout =
                    err_str.contains("Timed out") && err_str.contains("awaiting response");
                let err_msg = if is_response_timeout {
                    format!(
                        "[AI GM Turn Timeout] FastAPI AI GM service at {} is still generating a response ({err}). Increase `ai_gm.request_timeout_secs` in `config/ai.toml` (or set `SATYA_AI_GM_REQUEST_TIMEOUT_SECS`) for slower local models.",
                        ai_cfg.service_url
                    )
                } else {
                    format!(
                        "[AI GM Service Error] FastAPI AI GM service at {} is unreachable ({err}). Ensure `pixi run serve-ai-gm` is running.",
                        ai_cfg.service_url
                    )
                };
                tracing::error!(session_id = %session_id, trace_id = %trace_id, error = %err, "AI GM service dispatch failed (fail_fast_when_offline=true)");

                if let Ok(log_meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::SESSION_LOG,
                    1,
                    session_id,
                    session_mode,
                ) {
                    let log_ev = SessionLogEvent {
                        metadata: log_meta,
                        log_id: Uuid::now_v7().to_string(),
                        level: LogLevel::Error,
                        service_name: "satya-signaling".to_string(),
                        actor: Some(triggering_player.clone()),
                        trace_id: Some(trace_id.clone()),
                        span_id: Some(root_span_id.clone()),
                        parent_span_id: None,
                        lineage_id: Some(trigger_event_id.to_string()),
                        parent_lineage_ids: vec![],
                        log_line: format!(
                            "[ERROR] [session={session_id}] [trace={trace_id} span={root_span_id}] [lineage={trigger_event_id}] {err_msg}"
                        ),
                    };
                    let _ = state.storage.log_session_log(&log_ev).await;
                }

                // Emit fail-fast system notice to the triggering player / GM
                let req_ids = vec![triggering_player.user_id.clone()];
                let (recipients, delivery_ids) = SignalingAppState::resolve_message_targets(
                    ai_gm,
                    MessageVisibility::Private,
                    &req_ids,
                    roster,
                );
                if let Ok(meta) = state.storage.registry().build_metadata(
                    state.next_seq(),
                    subjects::CHAT_MESSAGE,
                    1,
                    session_id,
                    session_mode,
                ) {
                    let ev = ChatMessageEvent {
                        metadata: meta,
                        message_id: Uuid::now_v7().to_string(),
                        sender: ai_gm.clone(),
                        visibility: MessageVisibility::Private,
                        recipients,
                        channel_context: ChannelContext::SystemNotice,
                        reply_to_message_id: None,
                        body: err_msg.clone(),
                        char_count: err_msg.chars().count() as i32,
                        word_count: err_msg.split_whitespace().count() as i32,
                        composition_time_ms: 0,
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
                }
                return;
            }
        }
    }

    // Deterministic fallback for unit/integration tests (`fail_fast_when_offline == false`)
    let (reply_vis, target_uid) = if operating_mode == AiGmOperatingMode::CoPilot {
        (
            MessageVisibility::Private,
            human_gm_user_id.unwrap_or_else(|| triggering_player.user_id.clone()),
        )
    } else if visibility == MessageVisibility::GmOnly || visibility == MessageVisibility::Private {
        (
            MessageVisibility::Private,
            triggering_player.user_id.clone(),
        )
    } else {
        (MessageVisibility::Public, triggering_player.user_id.clone())
    };
    let req_ids = vec![target_uid];
    let (recipients, delivery_ids) =
        SignalingAppState::resolve_message_targets(ai_gm, reply_vis, &req_ids, roster);

    let response_body = if operating_mode == AiGmOperatingMode::CoPilot {
        format!(
            "[Co-Pilot Whisper to GM] Processed request from {}: {context_text}",
            triggering_player.display_name
        )
    } else if visibility == MessageVisibility::GmOnly {
        format!(
            "[Whisper to {}] Copy that. Environmental telemetry shows elevated heat signatures in Sector 4—proceed with caution.",
            triggering_player.display_name
        )
    } else {
        format!("AI GM Adjudication: {context_text}")
    };

    if let Ok(meta) = state.storage.registry().build_metadata(
        state.next_seq(),
        subjects::CHAT_MESSAGE,
        1,
        session_id,
        session_mode,
    ) {
        let ev = ChatMessageEvent {
            metadata: meta,
            message_id: Uuid::now_v7().to_string(),
            sender: ai_gm.clone(),
            visibility: reply_vis,
            recipients,
            channel_context: ChannelContext::GmNarration,
            reply_to_message_id: None,
            body: response_body.clone(),
            char_count: response_body.chars().count() as i32,
            word_count: response_body.split_whitespace().count() as i32,
            composition_time_ms: 45,
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
    }
}
