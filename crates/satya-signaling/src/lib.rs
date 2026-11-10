//! Axum HTTP + WebSocket signaling server, session lifecycle orchestration,
//! targeted chat routing, durable session persistence/resume, and Apache Iggy Avro event ingestion for Satya.
//!
//! ## Module Structure
//! - [`auth`]: Persistent Argon2id SQLite user accounts, token verification, password-protected session state, and ephemeral BYOK vault.
//! - [`session_persistence`]: Durable SQLite session metadata/event history persistence, lazy session rehydration, visibility-filtered history replay, `.satyasession` snapshot export/import, and `.md`/`.json` transcript export.
//! - [`state`]: Shared [`SignalingAppState`], message target routing, WebSocket fanout, and session disconnect cleanup.
//! - [`http_handlers`]: Leptos SSR shell, `/api/auth/*` (register, login, logout), `/api/sessions/*` (create, join, leave), and `/api/storage/*` admin metrics/flush endpoints.
//! - [`gm_api`]: Backend AI Game Master REST endpoints (`/api/sessions/:id/gm-config`, `/api/gm/sessions/:id/*`), Co-Pilot vs. Autonomous trigger filters, and FastAPI LangGraph turn dispatch.
//! - [`gm_docs`]: `GM_ONLY` document sharing (`/api/sessions/:id/docs`), Google OAuth `drive.file` linking (`/api/auth/google`), HTML/image stripping, minor-safety filtering, and prompt-injection neutralization.
//! - [`ws`]: Full-duplex `/ws/session` WebSocket upgrade, real-time chat, WebRTC signaling relay, prosody telemetry, and TTRPG task resolution.

pub mod auth;
pub mod characters;
pub mod gm_api;
pub mod gm_docs;
pub mod gm_persistence;
pub mod http_handlers;
pub mod npcs;
pub mod player_ai_api;
pub mod session_export;
pub mod session_persistence;
pub mod state;
pub mod ws;

pub use characters::*;
pub use gm_api::*;
pub use gm_docs::*;
pub use gm_persistence::*;
pub use http_handlers::*;
pub use npcs::*;
pub use player_ai_api::*;
pub use session_export::*;
pub use session_persistence::*;
pub use state::*;
pub use ws::*;

use axum::{
    Router,
    routing::{delete, get, post},
};
use std::sync::Arc;

/// Constructs the Axum [`Router`] with all HTTP REST endpoints, static `/pkg` and `/maps` asset serving,
/// Backend GM REST endpoints (`/api/gm/sessions/:id/*`), and the `/ws/session` WebSocket upgrade route.
pub fn build_router(state: Arc<SignalingAppState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .nest_service("/pkg", tower_http::services::ServeDir::new("pkg"))
        .nest_service("/maps", tower_http::services::ServeDir::new("data/maps"))
        .nest_service("/book", tower_http::services::ServeDir::new("docs/book"))
        .route("/api/rules", get(list_rules_handler))
        .route("/api/rules/:chapter_id", get(get_rules_chapter_handler))
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/logout", post(logout_handler))
        .route("/api/auth/me", get(auth_me_handler))
        .route("/api/auth/google", post(google_auth_link_handler))
        .route("/api/auth/google/status", get(google_auth_status_handler))
        .route("/api/sessions/create", post(create_session_handler))
        .route("/api/sessions/join", post(join_session_handler))
        .route("/api/sessions/leave", post(leave_session_handler))
        .route("/api/sessions/saved", get(list_saved_sessions_handler))
        .route("/api/sessions/resume", post(resume_session_handler))
        .route(
            "/api/sessions/import",
            post(import_session_snapshot_handler),
        )
        .route(
            "/api/sessions/:session_id/export",
            get(export_session_snapshot_handler),
        )
        .route(
            "/api/sessions/:session_id/transcript",
            get(export_session_transcript_handler),
        )
        .route(
            "/api/sessions/:session_id/gm-config",
            post(update_session_gm_config_handler),
        )
        .route(
            "/api/sessions/:session_id/docs",
            get(list_session_gm_docs_handler).post(submit_session_gm_doc_handler),
        )
        .route(
            "/api/sessions/:session_id/docs/:doc_id/persist",
            post(toggle_session_gm_doc_persistence_handler),
        )
        .route(
            "/api/sessions/:session_id/memories",
            get(list_session_gm_memories_handler).post(upsert_session_gm_memory_handler),
        )
        .route(
            "/api/sessions/:session_id/memories/:memory_id",
            delete(delete_session_gm_memory_handler),
        )
        .route(
            "/api/sessions/:session_id/combat",
            get(get_session_combat_tracker_handler).post(update_session_combat_tracker_handler),
        )
        .route(
            "/api/sessions/:session_id/lookup",
            get(gm_omnisearch_lookup_handler),
        )
        .route(
            "/api/sessions/:session_id/npcs",
            get(list_session_npcs_handler),
        )
        .route(
            "/api/sessions/:session_id/npcs/generate",
            post(generate_session_npc_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/context",
            get(gm_session_context_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/memories",
            get(list_session_gm_memories_handler).post(upsert_session_gm_memory_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/combat",
            get(get_session_combat_tracker_handler).post(update_session_combat_tracker_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/lookup",
            get(gm_omnisearch_lookup_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/npcs",
            get(list_session_npcs_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/npcs/generate",
            post(generate_session_npc_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/chat",
            post(gm_session_chat_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/roll",
            post(gm_session_roll_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/map",
            post(gm_session_map_handler),
        )
        .route(
            "/api/gm/sessions/:session_id/trajectory",
            post(gm_session_trajectory_handler),
        )
        .route(
            "/api/sessions/:session_id/ai-players",
            get(list_session_ai_players_handler).post(upsert_session_ai_player_handler),
        )
        .route(
            "/api/sessions/:session_id/ai-players/step",
            post(step_session_ai_player_handler),
        )
        .route(
            "/api/sessions/:session_id/ai-players/:player_user_id",
            delete(remove_session_ai_player_handler),
        )
        .route(
            "/api/player-ai/sessions/:session_id/context",
            get(player_ai_session_context_handler),
        )
        .route(
            "/api/player-ai/sessions/:session_id/chat",
            post(player_ai_session_chat_handler),
        )
        .route(
            "/api/player-ai/sessions/:session_id/roll",
            post(player_ai_session_roll_handler),
        )
        .route("/api/storage/metrics", get(storage_metrics_handler))
        .route("/api/storage/flush", post(storage_flush_handler))
        .route(
            "/api/characters",
            get(list_characters_handler).post(save_character_handler),
        )
        .route("/api/characters/import", post(import_character_handler))
        .route(
            "/api/characters/:character_id/export",
            get(export_character_handler),
        )
        .route(
            "/api/characters/:character_id/bind-session",
            post(bind_character_session_handler),
        )
        .route(
            "/api/characters/:character_id/assets",
            post(upload_character_asset_handler),
        )
        .route(
            "/api/characters/:character_id/assets/:asset_id",
            get(get_character_asset_handler),
        )
        .route("/ws/session", get(ws_session_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests_core;
#[cfg(test)]
mod tests_persistence;
