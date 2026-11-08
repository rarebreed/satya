//! Session Persistence, Resume, History Snapshot, Portable `.satyasession` Bundle,
//! and Chat/Mechanics Transcript Export DTOs (`GET /api/sessions/saved`,
//! `POST /api/sessions/resume`, `GET /api/sessions/:id/export`, `POST /api/sessions/import`,
//! `GET /api/sessions/:id/transcript`).

use crate::character::CharacterSheetRecord;
use crate::events::{ChatMessageEvent, MessageVisibility, TtrpgMechanicsEvent};
use crate::protocol::{
    CombatTrackerState, GmDocumentRecord, GmMapUpdateRequest, GmMemoryNoteRecord,
};
use crate::types::{Participant, SessionMode, SessionRole};
use serde::{Deserialize, Serialize};

/// Metadata summary of a saved session that an authenticated user has created or joined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionSummary {
    /// Unique identifier of the session room.
    pub session_id: String,
    /// Operating mode (`TTRPG` or `MEETING`).
    pub session_mode: SessionMode,
    /// Role used by the caller in this session (`GM`, `PLAYER`, `HOST`, etc.).
    pub role: SessionRole,
    /// Display or character name used by the caller in this session.
    pub display_name: String,
    /// Persistent `user_id` of the session creator/host.
    pub host_user_id: String,
    /// Campaign tag grouping persistent GM documents, memories, and NPCs.
    pub campaign_tag: String,
    /// Whether an autonomous or co-pilot AI Game Master is enabled in this session.
    #[serde(default)]
    pub has_ai_gm: bool,
    /// Bound `character_id` for the caller in this session, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_id: Option<String>,
    /// Total number of persisted chat messages in this session.
    #[serde(default)]
    pub chat_event_count: usize,
    /// Total number of persisted TTRPG mechanics/dice rolls in this session.
    #[serde(default)]
    pub mechanics_event_count: usize,
    /// Number of currently connected human participants in this session.
    #[serde(default)]
    pub active_human_count: usize,
    /// Creation timestamp in UTC microseconds since Unix epoch.
    #[serde(default)]
    pub created_at_us: i64,
    /// Timestamp of the most recent activity in UTC microseconds since Unix epoch.
    pub updated_at_us: i64,
}

/// HTTP response body for `GET /api/sessions/saved`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionsListResponse {
    /// Saved sessions ordered by most recently updated first.
    pub sessions: Vec<SavedSessionSummary>,
}

/// Visibility-filtered session state and chronological event history returned when
/// creating, joining, or resuming a session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionHistorySnapshot {
    /// Chronological `ChatMessageEvent` records visible to the caller (`PUBLIC`, caller's `PRIVATE`, and `GM_ONLY` for GM/Solo).
    #[serde(default)]
    pub chat_events: Vec<ChatMessageEvent>,
    /// Chronological `TtrpgMechanicsEvent` task resolution records in this session.
    #[serde(default)]
    pub mechanics_events: Vec<TtrpgMechanicsEvent>,
    /// Currently active tactical map and token placements, if any.
    #[serde(default)]
    pub active_map: Option<GmMapUpdateRequest>,
    /// Authoritative Count-Up Tick Combat Tracker state, if any.
    #[serde(default)]
    pub combat_tracker: Option<CombatTrackerState>,
    /// Character sheet bound to the caller in this session, if any.
    #[serde(default)]
    pub bound_character: Option<CharacterSheetRecord>,
}

/// HTTP request body for `POST /api/sessions/resume`.
///
/// Authorizes re-entry into a saved session via either:
/// - `join_token` (for seamless browser refresh recovery), OR
/// - `session_password` (when resuming after logging out or on another device), OR
/// - Being a previously verified participant or host of `session_id` in `session_participants`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeSessionRequest {
    /// Target `session_id` to resume.
    pub session_id: String,
    /// Optional plaintext session password (verified against the stored Argon2id hash when provided).
    #[serde(default)]
    pub session_password: Option<String>,
    /// Optional previous `join_token` from browser storage for zero-prompt refresh recovery.
    #[serde(default)]
    pub join_token: Option<String>,
    /// Optional role override (defaults to the caller's last recorded `SessionRole` in this session).
    #[serde(default)]
    pub role: Option<SessionRole>,
    /// Optional display name override (defaults to the caller's last recorded display name in this session).
    #[serde(default)]
    pub display_name: Option<String>,
}

fn default_snapshot_format_version() -> u16 {
    1
}

/// Portable `.satyasession` JSON bundle capturing a complete session's state, visibility-filtered
/// chat & mechanics history, tactical map, combat tracker, GM vault, and bound character sheets.
///
/// Ephemeral secrets (such as BYOK LLM API keys and Google OAuth tokens) are never included.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSnapshotBundle {
    /// Snapshot schema format version (`1`).
    #[serde(default = "default_snapshot_format_version")]
    pub format_version: u16,
    /// Export timestamp in UTC microseconds since Unix epoch.
    pub exported_at_us: i64,
    /// Unique identifier of the session.
    pub session_id: String,
    /// Operating mode (`TTRPG` or `MEETING`).
    pub session_mode: SessionMode,
    /// Campaign tag associated with the session.
    pub campaign_tag: String,
    /// Participant roster snapshot.
    #[serde(default)]
    pub roster: Vec<Participant>,
    /// Active tactical map state, if any.
    #[serde(default)]
    pub active_map: Option<GmMapUpdateRequest>,
    /// Authoritative Combat Tracker state, if any.
    #[serde(default)]
    pub combat_tracker: Option<CombatTrackerState>,
    /// Sanitized `GM_ONLY` documents visible to the exporting user.
    #[serde(default)]
    pub gm_documents: Vec<GmDocumentRecord>,
    /// GM memory notes visible to the exporting user.
    #[serde(default)]
    pub gm_memories: Vec<GmMemoryNoteRecord>,
    /// Character sheets bound to participants in this session.
    #[serde(default)]
    pub session_characters: Vec<CharacterSheetRecord>,
    /// Chronological chat messages visible to the exporting user.
    #[serde(default)]
    pub chat_events: Vec<ChatMessageEvent>,
    /// Chronological TTRPG mechanics/dice rolls.
    #[serde(default)]
    pub mechanics_events: Vec<TtrpgMechanicsEvent>,
}

/// HTTP request body for `POST /api/sessions/import` restoring a `.satyasession` snapshot bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportSessionSnapshotRequest {
    /// Portable session snapshot bundle to restore.
    pub bundle: SessionSnapshotBundle,
    /// Optional target `session_id` override (defaults to `bundle.session_id`).
    #[serde(default)]
    pub session_id_override: Option<String>,
    /// Session password to set if creating a new session record (or verifying an existing one).
    #[serde(default)]
    pub session_password: String,
    /// Optional role override for the importing user.
    #[serde(default)]
    pub role: Option<SessionRole>,
}

/// Discriminator for a chronological entry in a session transcript export (`CHAT` vs. `MECHANICS_ROLL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionTranscriptItemKind {
    /// Standard or AI GM chat message (`ChatMessageEvent`).
    Chat,
    /// Resolved `2d20` task or combat roll (`TtrpgMechanicsEvent`).
    MechanicsRoll,
}

/// Single chronological entry in an exported session chat/mechanics transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionTranscriptItem {
    /// Monotonic event sequence number ordering chat and mechanics events chronologically.
    pub sequence_num: i64,
    /// Event timestamp in UTC microseconds since Unix epoch.
    pub timestamp_us: i64,
    /// Entry kind (`CHAT` or `MECHANICS_ROLL`).
    pub kind: SessionTranscriptItemKind,
    /// Display name of the sender or rolling actor.
    pub actor_display_name: String,
    /// Session role of the sender or rolling actor.
    pub actor_role: SessionRole,
    /// Visibility tier of this entry (`PUBLIC`, `PRIVATE`, `GM_ONLY`, `HOST_ONLY`).
    pub visibility: MessageVisibility,
    /// Human-readable Markdown summary of the message or mechanics roll.
    pub summary_markdown: String,
    /// Populated when `kind` is [`SessionTranscriptItemKind::Chat`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_event: Option<ChatMessageEvent>,
    /// Populated when `kind` is [`SessionTranscriptItemKind::MechanicsRoll`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanics_event: Option<TtrpgMechanicsEvent>,
}

/// Structured JSON transcript response returned by `GET /api/sessions/:session_id/transcript?format=json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionTranscriptResponse {
    /// Session identifier.
    pub session_id: String,
    /// Operating mode (`TTRPG` or `MEETING`).
    pub session_mode: SessionMode,
    /// Export timestamp in UTC microseconds since Unix epoch.
    pub exported_at_us: i64,
    /// Persistent `user_id` of the caller who exported the transcript.
    pub exported_by_user_id: String,
    /// Role of the caller who exported the transcript.
    pub exported_by_role: SessionRole,
    /// Total number of chronological items included in `items`.
    pub total_items: usize,
    /// Chronologically ordered chat and mechanics events visible to the caller.
    pub items: Vec<SessionTranscriptItem>,
}
