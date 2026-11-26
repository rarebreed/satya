//! Persistent SQLx (SQLite WAL) + Argon2id user authentication, account-level RBAC
//! ([`AccountRole`]), and password-protected session roster management.
//!
//! Provides [`AuthAndSessionManager`] along with persistent user identity records
//! ([`UserAccount`]), active password-protected session state ([`ProtectedSession`]),
//! WebSocket join claims ([`JoinTokenClaims`]), and disconnect summaries
//! ([`SessionDisconnectSummary`]).

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::Utc;
pub use satya_schema::{AccountRole, AuthSuccessResponse};
use satya_schema::{
    AccountQuotaUsage, AiGmOperatingMode, AiGmProviderConfig, CharacterSheetRecord,
    CombatTrackerState, GmContextResponse, GmDocumentRecord, GmMapUpdateRequest,
    GmMemoryNoteRecord, GoogleAuthStatusResponse, Participant, ParticipantKind, SessionMode,
    SessionRole,
};
use satya_storage::{AuthConfig, QuotaConfig};
use serde::{Deserialize, Serialize};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};
use uuid::Uuid;

static ARGON2_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();
static DUMMY_ARGON2_HASH: OnceLock<String> = OnceLock::new();

fn argon2_semaphore() -> &'static Arc<Semaphore> {
    ARGON2_SEMAPHORE.get_or_init(|| {
        let permits = std::thread::available_parallelism()
            .map(|n| n.get().clamp(2, 4))
            .unwrap_or(2);
        Arc::new(Semaphore::new(permits))
    })
}

fn dummy_argon2_hash() -> &'static str {
    DUMMY_ARGON2_HASH.get_or_init(|| {
        AuthAndSessionManager::hash_secret("satya-dummy-timing-equalizer-secret")
            .unwrap_or_default()
    })
}

fn should_seed_demo_accounts(auth_config: &AuthConfig) -> bool {
    if let Some(explicit) = auth_config.seed_demo_accounts {
        return explicit;
    }
    if let Ok(raw) = std::env::var("SATYA_ENABLE_DEMO_ACCOUNTS") {
        let norm = raw.trim().to_ascii_lowercase();
        return matches!(norm.as_str(), "1" | "true" | "yes" | "on");
    }
    cfg!(test)
}

/// Persistent user account record stored in the SQLite `users` table (`data/auth/users.db`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    /// Persistent UUID (or preset demo ID) identifying this user across sessions.
    pub user_id: String,
    /// Normalized lowercase login username.
    pub username: String,
    /// Default display or character name for this user.
    pub display_name: String,
    /// Optional unique email address (kept private; used for directory lookup & offline GM alerts).
    #[serde(default)]
    pub email: Option<String>,
    /// Argon2id encoded password hash string.
    pub password_hash: String,
    /// Platform-level account role (`USER`, `GM`, or `ADMIN`).
    #[serde(default)]
    pub account_role: AccountRole,
    /// Account creation timestamp in UTC microseconds since Unix epoch.
    pub created_at_us: i64,
}

/// Ephemeral in-memory AI GM model and BYOK credential vault for an active session.
///
/// The `api_key` field is held strictly in memory for the duration of the session, is never
/// persisted to SQLite/Avro/PyLance or exposed in `ROSTER_SYNC` / `GmContextResponse`, and is
/// immediately scrubbed when the sponsoring user disconnects or the session ends.
#[derive(Debug, Clone)]
pub struct SessionAiGmVault {
    /// `user_id` of the authenticated participant who supplied the configuration/API key.
    pub sponsor_user_id: String,
    /// Model provider identifier (`"ollama"`, `"google_genai"`, `"openai"`, or `"anthropic"`).
    pub provider: String,
    /// Reasoning and tool-calling model identifier.
    pub model_id: String,
    /// Optional multimodal vision model identifier.
    pub vision_model_id: Option<String>,
    /// Sampling temperature.
    pub temperature: f32,
    /// Persona profile identifier.
    pub persona_id: String,
    /// Optional custom base URL.
    pub base_url: Option<String>,
    /// Ephemeral player-supplied API key (never serialized to logs or broadcast to peers).
    pub api_key: Option<String>,
}

/// Active password-protected session room and its current participant roster.
#[derive(Debug, Clone)]
pub struct ProtectedSession {
    /// Unique identifier for the session room.
    pub session_id: String,
    /// Argon2id encoded hash of the session password.
    pub password_hash: String,
    /// Business mode (`TTRPG` or `MEETING`) for this session.
    pub session_mode: SessionMode,
    /// Persistent `user_id` of the user who created/hosted the session.
    pub host_user_id: String,
    /// Persistent `user_id` of the owning GM (matches `host_user_id` for GM-owned sessions).
    pub owner_user_id: String,
    /// Optional parent campaign ID (`camp_<uuid>`).
    pub campaign_id: Option<String>,
    /// Optional human-friendly session title.
    pub title: Option<String>,
    /// Whether this session is discoverable via GM directory search.
    pub is_discoverable: bool,
    /// Whether players may submit join requests for this session.
    pub allow_join_requests: bool,
    /// Map of active participants in the session keyed by `user_id`.
    pub roster: HashMap<String, Participant>,
    /// Optional in-memory AI GM provider & ephemeral BYOK credential vault.
    pub ai_gm_vault: Option<SessionAiGmVault>,
    /// Optional currently active tactical map in the session.
    pub active_map: Option<GmMapUpdateRequest>,
    /// Sanitized `GM_ONLY` documents submitted to the GM in this session.
    pub gm_documents: Vec<GmDocumentRecord>,
    /// Campaign/session tag used to scope persistent GM documents and memories.
    pub campaign_tag: String,
    /// Persistent and session-scoped GM memory notes for this session/campaign.
    pub gm_memories: Vec<GmMemoryNoteRecord>,
    /// Authoritative Satya Count-Up Tick Combat Tracker state (`round`, `tick`, `combatants`).
    pub combat_tracker: CombatTrackerState,
    /// Session creation timestamp in UTC microseconds since Unix epoch.
    pub created_at_us: i64,
}

/// Verified claims bound to a WebSocket session join token (`join_<uuid>`).
#[derive(Debug, Clone)]
pub struct JoinTokenClaims {
    /// Session ID that the join token grants access to.
    pub session_id: String,
    /// Business mode (`TTRPG` or `MEETING`) of the session.
    pub session_mode: SessionMode,
    /// Authenticated [`Participant`] identity associated with the join token.
    pub participant: Participant,
    /// UTC microsecond expiration timestamp.
    pub expires_at_us: i64,
}

/// Stored bearer authentication token entry paired with its expiration timestamp.
#[derive(Debug, Clone)]
pub struct AuthTokenEntry {
    /// Authenticated user account snapshot.
    pub account: UserAccount,
    /// UTC microsecond expiration timestamp.
    pub expires_at_us: i64,
}

/// Summary of a participant's departure from an active session, used to emit
/// durable lifecycle and WebRTC disconnect events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDisconnectSummary {
    /// Session ID from which the participant disconnected.
    pub session_id: String,
    /// Business mode (`TTRPG` or `MEETING`) of the session.
    pub session_mode: SessionMode,
    /// Full [`Participant`] record of the departing user.
    pub departed_participant: Participant,
    /// Snapshot of all remaining participants still in the session roster.
    pub remaining_roster: Vec<Participant>,
    /// Count of human participants remaining in the session (`0` triggers `SESSION_ENDED`).
    pub remaining_human_count: usize,
    /// Whether active autonomous AI Players remain in the session (`SESSION_ENDED` suppressed while true).
    #[serde(default)]
    pub has_active_ai_players: bool,
}

/// Manages SQLx-backed Argon2id user registration/login, account-level RBAC (`USER` vs. `ADMIN`),
/// bearer authentication tokens, password-protected session creation/joining, and disconnect/logout flows.
pub struct AuthAndSessionManager {
    pub(crate) db_pool: SqlitePool,
    bootstrap_admins: HashSet<String>,
    bootstrap_gms: HashSet<String>,
    pub(crate) quotas: QuotaConfig,
    google_oauth_client_id: Option<String>,
    google_picker_api_key: Option<String>,
    pub(crate) users_by_username: RwLock<HashMap<String, UserAccount>>,
    pub(crate) auth_tokens: RwLock<HashMap<String, AuthTokenEntry>>,
    google_links: RwLock<HashMap<String, (String, Option<String>)>>,
    pub(crate) sessions: RwLock<HashMap<String, ProtectedSession>>,
    pub(crate) join_tokens: RwLock<HashMap<String, JoinTokenClaims>>,
}

/// Typed parameters for [`AuthAndSessionManager::create_session_with_params`].
#[derive(Debug, Clone)]
pub struct CreateSessionParams<'a> {
    /// Unique identifier for the session room to create or reopen.
    pub session_id: &'a str,
    /// Plaintext session password (hashed with Argon2id on creation).
    pub session_password: &'a str,
    /// Business operating mode (`TTRPG` or `MEETING`).
    pub session_mode: SessionMode,
    /// Role claimed by the session creator.
    pub role: SessionRole,
    /// Optional display name override for this session.
    pub display_name_override: Option<String>,
    /// Whether an autonomous AI Game Master is enabled.
    pub enable_ai_gm: bool,
    /// Optional parent campaign ID (`camp_<uuid>`).
    pub campaign_id: Option<&'a str>,
    /// Optional human-friendly session title.
    pub title: Option<String>,
    /// Whether this session is discoverable via GM directory search.
    pub is_discoverable: bool,
    /// Whether players may submit join requests for this session.
    pub allow_join_requests: bool,
}

/// Typed parameters for [`AuthAndSessionManager::provision_approved_campaign_session`].
#[derive(Debug, Clone, Copy)]
pub struct ProvisionCampaignSessionParams<'a> {
    /// Target parent campaign ID (`camp_<uuid>`).
    pub campaign_id: &'a str,
    /// Campaign slug (`campaign_tag`).
    pub campaign_slug: &'a str,
    /// Unique identifier for the newly provisioned session room.
    pub session_id: &'a str,
    /// Human-readable session title.
    pub title: &'a str,
    /// Operating mode (`TTRPG` or `MEETING`).
    pub session_mode: SessionMode,
    /// Whether an autonomous AI Game Master is enabled.
    pub enable_ai_gm: bool,
}

impl AuthAndSessionManager {
    /// Initializes the SQLx SQLite authentication and session manager with default [`QuotaConfig`].
    pub async fn new(base_dir: impl AsRef<Path>, auth_config: &AuthConfig) -> anyhow::Result<Self> {
        Self::new_with_quotas(base_dir, auth_config, QuotaConfig::default()).await
    }

    /// Initializes the SQLx SQLite authentication and session manager rooted at `base_dir`,
    /// running schema migrations, seeding default demo accounts if the `users` table is empty,
    /// and promoting configured `bootstrap_admins` and `bootstrap_gms`.
    pub async fn new_with_quotas(
        base_dir: impl AsRef<Path>,
        auth_config: &AuthConfig,
        quotas: QuotaConfig,
    ) -> anyhow::Result<Self> {
        let connect_opts =
            Self::resolve_sqlite_options(base_dir.as_ref(), &auth_config.database_url)?;
        let db_pool = SqlitePoolOptions::new()
            .max_connections(8)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(connect_opts)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                user_id       TEXT PRIMARY KEY NOT NULL,
                username      TEXT UNIQUE NOT NULL,
                display_name  TEXT NOT NULL,
                email         TEXT,
                password_hash TEXT NOT NULL,
                account_role  TEXT NOT NULL DEFAULT 'USER',
                created_at_us INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS characters (
                character_id        TEXT PRIMARY KEY NOT NULL,
                owner_user_id       TEXT NOT NULL,
                character_name      TEXT NOT NULL,
                version             INTEGER NOT NULL DEFAULT 1,
                verification_status TEXT NOT NULL DEFAULT 'VERIFIED_OFFICIAL',
                content_sha256      TEXT NOT NULL,
                signature_hex       TEXT,
                sheet_json          TEXT NOT NULL,
                created_at_us       INTEGER NOT NULL,
                updated_at_us       INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_characters_owner ON characters(owner_user_id);
            CREATE TABLE IF NOT EXISTS character_assets (
                asset_id      TEXT PRIMARY KEY NOT NULL,
                character_id  TEXT NOT NULL,
                owner_user_id TEXT NOT NULL,
                asset_kind    TEXT NOT NULL,
                filename      TEXT NOT NULL,
                mime_type     TEXT NOT NULL,
                sha256_hex    TEXT NOT NULL,
                size_bytes    INTEGER NOT NULL,
                blob_data     BLOB NOT NULL,
                created_at_us INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_character_assets_char ON character_assets(character_id);
            CREATE TABLE IF NOT EXISTS gm_documents (
                doc_id                TEXT PRIMARY KEY NOT NULL,
                session_id            TEXT NOT NULL,
                campaign_tag          TEXT NOT NULL,
                uploader_user_id      TEXT NOT NULL,
                uploader_display_name TEXT NOT NULL,
                title                 TEXT NOT NULL,
                source_kind           TEXT NOT NULL,
                google_file_id        TEXT,
                sanitized_content     TEXT NOT NULL,
                char_count            INTEGER NOT NULL,
                safety_flagged        INTEGER NOT NULL DEFAULT 0,
                created_at_us         INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_gm_docs_campaign ON gm_documents(campaign_tag, session_id);
            CREATE TABLE IF NOT EXISTS gm_memories (
                memory_id           TEXT PRIMARY KEY NOT NULL,
                session_id          TEXT NOT NULL,
                campaign_tag        TEXT NOT NULL,
                author_user_id      TEXT NOT NULL,
                author_display_name TEXT NOT NULL,
                category            TEXT NOT NULL,
                title               TEXT NOT NULL,
                content             TEXT NOT NULL,
                is_pinned           INTEGER NOT NULL DEFAULT 0,
                is_persistent       INTEGER NOT NULL DEFAULT 1,
                updated_at_us       INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_gm_memories_campaign ON gm_memories(campaign_tag, session_id);
            CREATE TABLE IF NOT EXISTS session_combat_state (
                session_id    TEXT PRIMARY KEY NOT NULL,
                state_json    TEXT NOT NULL,
                updated_at_us INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS auth_tokens (
                token         TEXT PRIMARY KEY NOT NULL,
                user_id       TEXT NOT NULL,
                created_at_us INTEGER NOT NULL,
                expires_at_us INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS campaigns (
                campaign_id     TEXT PRIMARY KEY NOT NULL,
                owner_user_id   TEXT NOT NULL,
                slug            TEXT NOT NULL,
                title           TEXT NOT NULL,
                description     TEXT NOT NULL DEFAULT '',
                setting_pack    TEXT NOT NULL DEFAULT 'awakened',
                is_discoverable INTEGER NOT NULL DEFAULT 1,
                is_archived     INTEGER NOT NULL DEFAULT 0,
                created_at_us   INTEGER NOT NULL,
                updated_at_us   INTEGER NOT NULL,
                UNIQUE(owner_user_id, slug COLLATE NOCASE)
            );
            CREATE INDEX IF NOT EXISTS idx_campaigns_owner ON campaigns(owner_user_id, updated_at_us DESC);
            CREATE TABLE IF NOT EXISTS saved_sessions (
                session_id          TEXT PRIMARY KEY NOT NULL,
                password_hash       TEXT NOT NULL,
                session_mode        TEXT NOT NULL,
                host_user_id        TEXT NOT NULL,
                owner_user_id       TEXT,
                campaign_id         TEXT,
                title               TEXT,
                is_discoverable     INTEGER NOT NULL DEFAULT 1,
                allow_join_requests INTEGER NOT NULL DEFAULT 1,
                campaign_tag        TEXT NOT NULL,
                enable_ai_gm        INTEGER NOT NULL DEFAULT 0,
                ai_provider_json    TEXT,
                active_map_json     TEXT,
                created_at_us       INTEGER NOT NULL,
                updated_at_us       INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS session_join_requests (
                request_id             TEXT PRIMARY KEY NOT NULL,
                request_kind           TEXT NOT NULL DEFAULT 'JOIN_SESSION',
                session_id             TEXT NOT NULL,
                proposed_session_title TEXT,
                proposed_session_mode  TEXT,
                proposed_enable_ai_gm  INTEGER NOT NULL DEFAULT 0,
                campaign_id            TEXT,
                owner_user_id          TEXT NOT NULL,
                requester_user_id      TEXT NOT NULL,
                requester_username     TEXT NOT NULL,
                requester_display_name TEXT NOT NULL,
                requested_role         TEXT NOT NULL,
                message                TEXT,
                status                 TEXT NOT NULL,
                approval_token         TEXT,
                decision_note          TEXT,
                emailed_offline        INTEGER NOT NULL DEFAULT 0,
                created_at_us          INTEGER NOT NULL,
                updated_at_us          INTEGER NOT NULL,
                expires_at_us          INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_join_req_owner
                ON session_join_requests(owner_user_id, status, updated_at_us DESC);
            CREATE INDEX IF NOT EXISTS idx_join_req_requester
                ON session_join_requests(requester_user_id, status, updated_at_us DESC);
            CREATE INDEX IF NOT EXISTS idx_join_req_session
                ON session_join_requests(session_id, requester_user_id);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_join_req_unique_pending
                ON session_join_requests(session_id, requester_user_id)
                WHERE status = 'PENDING';
            CREATE TABLE IF NOT EXISTS email_outbox (
                outbox_id     TEXT PRIMARY KEY NOT NULL,
                to_email      TEXT NOT NULL,
                subject       TEXT NOT NULL,
                body_text     TEXT NOT NULL,
                request_id    TEXT NOT NULL,
                status        TEXT NOT NULL,
                created_at_us INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS session_participants (
                session_id    TEXT NOT NULL,
                user_id       TEXT NOT NULL,
                username      TEXT NOT NULL,
                display_name  TEXT NOT NULL,
                role          TEXT NOT NULL,
                character_id  TEXT,
                updated_at_us INTEGER NOT NULL,
                PRIMARY KEY (session_id, user_id)
            );
            CREATE TABLE IF NOT EXISTS session_event_history (
                session_id     TEXT NOT NULL,
                sequence_num   INTEGER NOT NULL,
                event_kind     TEXT NOT NULL,
                timestamp_us   INTEGER NOT NULL,
                sender_user_id TEXT NOT NULL,
                visibility     TEXT NOT NULL,
                payload_json   TEXT NOT NULL,
                PRIMARY KEY (session_id, sequence_num)
            );
            CREATE INDEX IF NOT EXISTS idx_session_event_history_sid
                ON session_event_history(session_id, sequence_num);

            CREATE TABLE IF NOT EXISTS session_world_ledger (
                session_id    TEXT PRIMARY KEY,
                ledger_json   TEXT NOT NULL,
                updated_at_us INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS session_ai_players (
                session_id       TEXT NOT NULL,
                user_id          TEXT NOT NULL,
                display_name     TEXT NOT NULL,
                character_id     TEXT,
                persona_prompt   TEXT NOT NULL,
                provider         TEXT NOT NULL,
                model_id         TEXT NOT NULL,
                service_url      TEXT,
                auto_respond     INTEGER NOT NULL DEFAULT 1,
                max_rally_turns  INTEGER NOT NULL DEFAULT 2,
                updated_at_us    INTEGER NOT NULL,
                PRIMARY KEY (session_id, user_id)
            );

            CREATE TABLE IF NOT EXISTS saved_maps (
                map_id        TEXT PRIMARY KEY NOT NULL,
                owner_user_id TEXT NOT NULL,
                title         TEXT NOT NULL,
                theme_preset  TEXT,
                grid_width    INTEGER NOT NULL,
                grid_height   INTEGER NOT NULL,
                cell_size_cm  INTEGER NOT NULL DEFAULT 100,
                floor_count   INTEGER NOT NULL DEFAULT 1,
                prop_count    INTEGER NOT NULL DEFAULT 0,
                wall_count    INTEGER NOT NULL DEFAULT 0,
                surface_count INTEGER NOT NULL DEFAULT 0,
                map_json      TEXT NOT NULL,
                created_at_us INTEGER NOT NULL,
                updated_at_us INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_saved_maps_owner ON saved_maps(owner_user_id, updated_at_us DESC);
            "#,
        )
        .execute(&db_pool)
        .await?;

        // Idempotent column migrations for pre-existing SQLite databases
        for alter_sql in [
            "ALTER TABLE users ADD COLUMN email TEXT",
            "ALTER TABLE auth_tokens ADD COLUMN expires_at_us INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE saved_sessions ADD COLUMN owner_user_id TEXT",
            "ALTER TABLE saved_sessions ADD COLUMN campaign_id TEXT",
            "ALTER TABLE saved_sessions ADD COLUMN title TEXT",
            "ALTER TABLE saved_sessions ADD COLUMN is_discoverable INTEGER NOT NULL DEFAULT 1",
            "ALTER TABLE saved_sessions ADD COLUMN allow_join_requests INTEGER NOT NULL DEFAULT 1",
            "ALTER TABLE session_join_requests ADD COLUMN request_kind TEXT NOT NULL DEFAULT 'JOIN_SESSION'",
            "ALTER TABLE session_join_requests ADD COLUMN proposed_session_title TEXT",
            "ALTER TABLE session_join_requests ADD COLUMN proposed_session_mode TEXT",
            "ALTER TABLE session_join_requests ADD COLUMN proposed_enable_ai_gm INTEGER NOT NULL DEFAULT 0",
        ] {
            let _ = sqlx::query(alter_sql).execute(&db_pool).await;
        }
        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_nocase
                ON users(email COLLATE NOCASE) WHERE email IS NOT NULL AND email != '';
            UPDATE saved_sessions
                SET owner_user_id = host_user_id
                WHERE owner_user_id IS NULL OR owner_user_id = '';
            CREATE INDEX IF NOT EXISTS idx_saved_sessions_owner
                ON saved_sessions(owner_user_id, campaign_id, updated_at_us DESC);
            "#,
        )
        .execute(&db_pool)
        .await?;

        let bootstrap_admins: HashSet<String> = auth_config
            .bootstrap_admins
            .iter()
            .map(|u| u.trim().to_lowercase())
            .filter(|u| !u.is_empty())
            .collect();
        let bootstrap_gms: HashSet<String> = auth_config
            .bootstrap_gms
            .iter()
            .map(|u| u.trim().to_lowercase())
            .filter(|u| !u.is_empty())
            .collect();

        let existing_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&db_pool)
            .await?;

        if existing_count == 0 && should_seed_demo_accounts(auth_config) {
            let defaults = [
                ("u-demo-gm-sean", "gm_sean", "Sean (GM)", "gm_sean@satya.local"),
                ("u-demo-sara", "sara_valkyrie", "Sara (Valkyrie)", "sara@satya.local"),
                ("u-demo-marcus", "marcus_recon", "Marcus (Recon)", "marcus@satya.local"),
                (
                    "u-demo-host-elena",
                    "host_elena",
                    "Elena (Meeting Host)",
                    "elena@satya.local",
                ),
            ];
            for (uid, uname, dname, email) in defaults {
                let hash = Self::hash_secret("satya-truth-2026")?;
                let role = if bootstrap_admins.contains(uname) {
                    AccountRole::Admin
                } else if bootstrap_gms.contains(uname) {
                    AccountRole::Gm
                } else {
                    AccountRole::User
                };
                let now_us = Utc::now().timestamp_micros();
                sqlx::query(
                    r#"
                    INSERT INTO users (user_id, username, display_name, email, password_hash, account_role, created_at_us)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    "#,
                )
                .bind(uid)
                .bind(uname)
                .bind(dname)
                .bind(email)
                .bind(&hash)
                .bind(role.as_db_str())
                .bind(now_us)
                .execute(&db_pool)
                .await?;
            }
        } else {
            for admin_uname in &bootstrap_admins {
                sqlx::query("UPDATE users SET account_role = 'ADMIN' WHERE username = ?1")
                    .bind(admin_uname)
                    .execute(&db_pool)
                    .await?;
            }
            for gm_uname in &bootstrap_gms {
                sqlx::query(
                    "UPDATE users SET account_role = 'GM' WHERE username = ?1 AND account_role = 'USER'",
                )
                .bind(gm_uname)
                .execute(&db_pool)
                .await?;
            }
        }

        let rows = sqlx::query(
            "SELECT user_id, username, display_name, email, password_hash, account_role, created_at_us FROM users",
        )
        .fetch_all(&db_pool)
        .await?;

        let mut initial_users = HashMap::with_capacity(rows.len());
        let mut users_by_id = HashMap::with_capacity(rows.len());
        for row in rows {
            let uname: String = row.try_get("username")?;
            let role_str: String = row.try_get("account_role")?;
            let email: Option<String> = row
                .try_get::<Option<String>, _>("email")
                .ok()
                .flatten()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty());
            let acct = UserAccount {
                user_id: row.try_get("user_id")?,
                username: uname.clone(),
                display_name: row.try_get("display_name")?,
                email,
                password_hash: row.try_get("password_hash")?,
                account_role: AccountRole::from_db_str(&role_str),
                created_at_us: row.try_get("created_at_us")?,
            };
            users_by_id.insert(acct.user_id.clone(), acct.clone());
            initial_users.insert(uname, acct);
        }

        let now_us = Utc::now().timestamp_micros();
        let default_ttl_us = (quotas.auth_token_ttl_hours as i64).saturating_mul(3_600_000_000);
        let token_rows =
            sqlx::query("SELECT token, user_id, created_at_us, expires_at_us FROM auth_tokens")
                .fetch_all(&db_pool)
                .await
                .unwrap_or_default();
        let mut initial_tokens = HashMap::with_capacity(token_rows.len());
        for row in token_rows {
            if let (Ok(tok), Ok(uid)) = (
                row.try_get::<String, _>("token"),
                row.try_get::<String, _>("user_id"),
            ) {
                let created_at_us = row.try_get::<i64, _>("created_at_us").unwrap_or(now_us);
                let raw_exp = row.try_get::<i64, _>("expires_at_us").unwrap_or(0);
                let expires_at_us = if raw_exp > 0 {
                    raw_exp
                } else {
                    created_at_us.saturating_add(default_ttl_us)
                };
                if expires_at_us <= now_us {
                    let _ = sqlx::query("DELETE FROM auth_tokens WHERE token = ?1")
                        .bind(&tok)
                        .execute(&db_pool)
                        .await;
                    continue;
                }
                if let Some(acct) = users_by_id.get(&uid) {
                    initial_tokens.insert(
                        tok,
                        AuthTokenEntry {
                            account: acct.clone(),
                            expires_at_us,
                        },
                    );
                }
            }
        }

        let google_oauth_client_id = auth_config
            .google_oauth_client_id
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let google_picker_api_key = auth_config
            .google_picker_api_key
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Ok(Self {
            db_pool,
            bootstrap_admins,
            bootstrap_gms,
            quotas,
            google_oauth_client_id,
            google_picker_api_key,
            users_by_username: RwLock::new(initial_users),
            auth_tokens: RwLock::new(initial_tokens),
            google_links: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            join_tokens: RwLock::new(HashMap::new()),
        })
    }

    fn resolve_sqlite_options(
        base_dir: &Path,
        database_url: &str,
    ) -> anyhow::Result<SqliteConnectOptions> {
        let trimmed = database_url.trim();
        if trimmed.contains(":memory:") {
            return Ok(SqliteConnectOptions::from_str("sqlite::memory:")?
                .create_if_missing(true)
                .busy_timeout(Duration::from_secs(10)));
        }

        let path_part = trimmed
            .strip_prefix("sqlite://")
            .or_else(|| trimmed.strip_prefix("sqlite:"))
            .unwrap_or(trimmed);
        let file_rel = path_part.split('?').next().unwrap_or("data/auth/users.db");
        let candidate: PathBuf = Path::new(file_rel).components().collect();
        let raw_path = if candidate.is_absolute() {
            candidate
        } else {
            base_dir.join(candidate)
        };
        let full_path: PathBuf = raw_path.components().collect();

        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(SqliteConnectOptions::new()
            .filename(full_path)
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(10))
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal))
    }

    /// Hashes a plaintext secret (user password or session password) using Argon2id and a CSPRNG salt.
    pub fn hash_secret(secret: &str) -> anyhow::Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(secret.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Argon2 hash error: {e}"))?
            .to_string();
        Ok(hash)
    }

    /// Offloads Argon2id password hashing onto `tokio::task::spawn_blocking` gated by a bounded semaphore.
    pub async fn hash_secret_async(secret: String) -> anyhow::Result<String> {
        let _permit = argon2_semaphore()
            .acquire()
            .await
            .map_err(|_| anyhow::anyhow!("Argon2 semaphore closed"))?;
        tokio::task::spawn_blocking(move || Self::hash_secret(&secret))
            .await
            .map_err(|e| anyhow::anyhow!("Argon2 task panicked: {e}"))?
    }

    /// Verifies a plaintext secret against an Argon2id PHC string hash.
    pub fn verify_secret(secret: &str, encoded_hash: &str) -> bool {
        let Ok(parsed_hash) = PasswordHash::new(encoded_hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(secret.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Offloads Argon2id password verification onto `tokio::task::spawn_blocking` gated by a bounded semaphore.
    pub async fn verify_secret_async(secret: String, encoded_hash: String) -> bool {
        let Ok(_permit) = argon2_semaphore().acquire().await else {
            return false;
        };
        tokio::task::spawn_blocking(move || Self::verify_secret(&secret, &encoded_hash))
            .await
            .unwrap_or(false)
    }

    /// Validates and normalizes an optional email address (`None` when blank).
    pub fn normalize_and_validate_email(raw: Option<&str>) -> anyhow::Result<Option<String>> {
        let Some(trimmed) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        let lower = trimmed.to_lowercase();
        if lower.len() > 254 || lower.chars().any(|c| c.is_whitespace() || c.is_control()) {
            anyhow::bail!("Invalid email address format");
        }
        let Some((local, domain)) = lower.split_once('@') else {
            anyhow::bail!("Email address must contain '@'");
        };
        if local.is_empty()
            || domain.is_empty()
            || domain.contains('@')
            || !domain.contains('.')
            || domain.starts_with('.')
            || domain.ends_with('.')
        {
            anyhow::bail!("Invalid email address domain");
        }
        Ok(Some(lower))
    }

    /// Registers a new user account, persists it to the SQLx SQLite `users` table, and returns a bearer token response.
    /// Self-registered users receive [`AccountRole::User`] unless listed in `bootstrap_admins` or `bootstrap_gms`.
    pub async fn register_user(
        &self,
        username: &str,
        password: &str,
        display_name: &str,
    ) -> anyhow::Result<AuthSuccessResponse> {
        self.register_user_with_email(username, password, display_name, None)
            .await
    }

    /// Registers a new user account with an optional unique `email` address.
    pub async fn register_user_with_email(
        &self,
        username: &str,
        password: &str,
        display_name: &str,
        email: Option<&str>,
    ) -> anyhow::Result<AuthSuccessResponse> {
        let uname = crate::security::validate_and_normalize_username(username)
            .map_err(|e| anyhow::anyhow!(e))?;
        crate::security::validate_account_password(password)
            .map_err(|e| anyhow::anyhow!(e))?;
        let clean_display_name = if display_name.trim().is_empty() {
            uname.clone()
        } else {
            crate::security::validate_display_name(display_name)
                .map_err(|e| anyhow::anyhow!(e))?
        };
        let clean_email = Self::normalize_and_validate_email(email)?;

        {
            let guard = self.users_by_username.read().await;
            if guard.contains_key(&uname) {
                anyhow::bail!("Username '{uname}' is already registered");
            }
        }
        if let Some(ref em) = clean_email {
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM users WHERE email = ?1 COLLATE NOCASE",
            )
            .bind(em)
            .fetch_one(&self.db_pool)
            .await
            .unwrap_or(0);
            if exists > 0 {
                anyhow::bail!("Email '{em}' is already registered");
            }
        }

        let password_hash = Self::hash_secret_async(password.to_string()).await?;
        let account = {
            let mut guard = self.users_by_username.write().await;
            if guard.contains_key(&uname) {
                anyhow::bail!("Username '{uname}' is already registered");
            }
            if let Some(ref em) = clean_email {
                let exists: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM users WHERE email = ?1 COLLATE NOCASE",
                )
                .bind(em)
                .fetch_one(&self.db_pool)
                .await
                .unwrap_or(0);
                if exists > 0 {
                    anyhow::bail!("Email '{em}' is already registered");
                }
            }
            let account_role = if self.bootstrap_admins.contains(&uname) {
                AccountRole::Admin
            } else if self.bootstrap_gms.contains(&uname) {
                AccountRole::Gm
            } else {
                AccountRole::User
            };
            let acct = UserAccount {
                user_id: Uuid::now_v7().to_string(),
                username: uname.clone(),
                display_name: clean_display_name,
                email: clean_email.clone(),
                password_hash,
                account_role,
                created_at_us: Utc::now().timestamp_micros(),
            };

            sqlx::query(
                r#"
                INSERT INTO users (user_id, username, display_name, email, password_hash, account_role, created_at_us)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
            )
            .bind(&acct.user_id)
            .bind(&acct.username)
            .bind(&acct.display_name)
            .bind(&acct.email)
            .bind(&acct.password_hash)
            .bind(acct.account_role.as_db_str())
            .bind(acct.created_at_us)
            .execute(&self.db_pool)
            .await?;

            guard.insert(uname, acct.clone());
            acct
        };

        self.issue_auth_token(account).await
    }

    /// Updates the authenticated user's `display_name` and/or private `email` address.
    pub async fn update_user_profile(
        &self,
        user_id: &str,
        display_name: Option<&str>,
        email: Option<&str>,
    ) -> anyhow::Result<UserAccount> {
        let clean_dname = match display_name.map(str::trim).filter(|s| !s.is_empty()) {
            Some(d) => Some(
                crate::security::validate_display_name(d).map_err(|e| anyhow::anyhow!(e))?,
            ),
            None => None,
        };
        let next_email_opt = match email {
            Some(raw) => Some(Self::normalize_and_validate_email(Some(raw))?),
            None => None,
        };
        if let Some(Some(ref em)) = next_email_opt {
            let conflict: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM users WHERE email = ?1 COLLATE NOCASE AND user_id != ?2",
            )
            .bind(em)
            .bind(user_id)
            .fetch_one(&self.db_pool)
            .await
            .unwrap_or(0);
            if conflict > 0 {
                anyhow::bail!("Email '{em}' is already linked to another account");
            }
        }

        let updated = {
            let mut guard = self.users_by_username.write().await;
            let acct = guard
                .values_mut()
                .find(|u| u.user_id == user_id)
                .ok_or_else(|| anyhow::anyhow!("Unknown user_id '{user_id}'"))?;
            if let Some(dname) = clean_dname {
                acct.display_name = dname;
            }
            if let Some(em_val) = next_email_opt {
                acct.email = em_val;
            }
            sqlx::query("UPDATE users SET display_name = ?1, email = ?2 WHERE user_id = ?3")
                .bind(&acct.display_name)
                .bind(&acct.email)
                .bind(&acct.user_id)
                .execute(&self.db_pool)
                .await?;
            acct.clone()
        };

        let mut tokens = self.auth_tokens.write().await;
        for entry in tokens.values_mut() {
            if entry.account.user_id == user_id {
                entry.account.display_name = updated.display_name.clone();
                entry.account.email = updated.email.clone();
            }
        }

        Ok(updated)
    }

    /// Updates a user's [`AccountRole`] in both the SQLite `users` table and in-memory token/account caches.
    pub async fn set_user_account_role(
        &self,
        username: &str,
        role: AccountRole,
    ) -> anyhow::Result<UserAccount> {
        let uname = username.trim().to_lowercase();
        let updated = {
            let mut guard = self.users_by_username.write().await;
            let acct = guard
                .get_mut(&uname)
                .ok_or_else(|| anyhow::anyhow!("Unknown username '{uname}'"))?;
            sqlx::query("UPDATE users SET account_role = ?1 WHERE username = ?2")
                .bind(role.as_db_str())
                .bind(&uname)
                .execute(&self.db_pool)
                .await?;
            acct.account_role = role;
            acct.clone()
        };

        let mut tokens = self.auth_tokens.write().await;
        for entry in tokens.values_mut() {
            if entry.account.username == uname {
                entry.account.account_role = role;
            }
        }

        Ok(updated)
    }

    /// Authenticates an existing user with `username` and `password`, issuing a new bearer token.
    pub async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
    ) -> anyhow::Result<AuthSuccessResponse> {
        if crate::security::validate_password_upper_bound(password).is_err() {
            anyhow::bail!("Invalid username or password");
        }
        let uname = username.trim().to_lowercase();
        let account_opt = {
            let guard = self.users_by_username.read().await;
            guard.get(&uname).cloned()
        };

        let Some(account) = account_opt else {
            let _ = Self::verify_secret_async(
                password.to_string(),
                dummy_argon2_hash().to_string(),
            )
            .await;
            anyhow::bail!("Invalid username or password");
        };

        if !Self::verify_secret_async(password.to_string(), account.password_hash.clone()).await {
            anyhow::bail!("Invalid username or password");
        }

        self.issue_auth_token(account).await
    }

    pub(crate) async fn issue_auth_token(
        &self,
        account: UserAccount,
    ) -> anyhow::Result<AuthSuccessResponse> {
        let token = format!("satya_tok_{}", Uuid::new_v4());
        let now_us = Utc::now().timestamp_micros();
        let ttl_us = (self.quotas.auth_token_ttl_hours as i64).saturating_mul(3_600_000_000);
        let expires_at_us = now_us.saturating_add(ttl_us);
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO auth_tokens (token, user_id, created_at_us, expires_at_us) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&token)
        .bind(&account.user_id)
        .bind(now_us)
        .bind(expires_at_us)
        .execute(&self.db_pool)
        .await;

        self.auth_tokens.write().await.insert(
            token.clone(),
            AuthTokenEntry {
                account: account.clone(),
                expires_at_us,
            },
        );

        Ok(AuthSuccessResponse {
            auth_token: token,
            user_id: account.user_id,
            username: account.username,
            display_name: account.display_name,
            account_role: account.account_role,
            email: account.email,
        })
    }

    /// Verifies a bearer token (with or without a `"Bearer "` prefix) and returns the associated [`UserAccount`].
    pub async fn verify_auth_token(&self, token: &str) -> Option<UserAccount> {
        let clean = token.strip_prefix("Bearer ").unwrap_or(token).trim();
        let now_us = Utc::now().timestamp_micros();
        let entry = self.auth_tokens.read().await.get(clean).cloned()?;
        if entry.expires_at_us <= now_us {
            self.auth_tokens.write().await.remove(clean);
            let _ = sqlx::query("DELETE FROM auth_tokens WHERE token = ?1")
                .bind(clean)
                .execute(&self.db_pool)
                .await;
            return None;
        }
        Some(entry.account)
    }

    /// Computes live quota usage and effective limits for `user`.
    pub async fn compute_account_quota_usage(&self, user: &UserAccount) -> AccountQuotaUsage {
        let mult = if user.account_role == AccountRole::Admin {
            10
        } else {
            1
        };
        let campaigns_limit = if user.account_role.can_host_sessions() {
            self.quotas.max_campaigns_per_gm.saturating_mul(mult)
        } else {
            0
        };
        let sessions_limit = if user.account_role.can_host_sessions() {
            self.quotas.max_total_sessions_per_gm.saturating_mul(mult)
        } else {
            0
        };
        let campaigns_used: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM campaigns WHERE owner_user_id = ?1 AND is_archived = 0",
        )
        .bind(&user.user_id)
        .fetch_one(&self.db_pool)
        .await
        .unwrap_or(0);
        let sessions_used: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM saved_sessions WHERE COALESCE(NULLIF(owner_user_id, ''), host_user_id) = ?1",
        )
        .bind(&user.user_id)
        .fetch_one(&self.db_pool)
        .await
        .unwrap_or(0);
        let now_us = Utc::now().timestamp_micros();
        let pending_requests_used: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM session_join_requests WHERE requester_user_id = ?1 AND status = 'PENDING' AND expires_at_us > ?2",
        )
        .bind(&user.user_id)
        .bind(now_us)
        .fetch_one(&self.db_pool)
        .await
        .unwrap_or(0);

        AccountQuotaUsage {
            campaigns_used: campaigns_used.max(0) as usize,
            campaigns_limit,
            sessions_used: sessions_used.max(0) as usize,
            sessions_limit,
            pending_requests_used: pending_requests_used.max(0) as usize,
            pending_requests_limit: self.quotas.max_pending_join_requests_per_user,
        }
    }

    /// Looks up a persistent [`UserAccount`] by `user_id`.
    pub async fn get_user_by_id(&self, user_id: &str) -> Option<UserAccount> {
        let guard = self.users_by_username.read().await;
        guard.values().find(|u| u.user_id == user_id).cloned()
    }

    /// Provisions a newly approved campaign session in SQLite (`saved_sessions` + owner `session_participants`)
    /// without placing an offline owner into the active live WebSocket roster.
    pub async fn provision_approved_campaign_session(
        &self,
        owner: &UserAccount,
        params: ProvisionCampaignSessionParams<'_>,
    ) -> anyhow::Result<()> {
        let sid = params.session_id.trim();
        if sid.is_empty() {
            anyhow::bail!("session_id cannot be empty");
        }
        if self.sessions.read().await.contains_key(sid)
            || self.load_saved_session_meta_from_db(sid).await.is_some()
        {
            anyhow::bail!("Session '{sid}' already exists");
        }

        let mult = if owner.account_role == AccountRole::Admin {
            10
        } else {
            1
        };
        let total_limit = self.quotas.max_total_sessions_per_gm.saturating_mul(mult);
        let total_used: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM saved_sessions WHERE COALESCE(NULLIF(owner_user_id, ''), host_user_id) = ?1",
        )
        .bind(&owner.user_id)
        .fetch_one(&self.db_pool)
        .await
        .unwrap_or(0);
        if (total_used.max(0) as usize) >= total_limit {
            anyhow::bail!(
                "Quota exceeded: maximum {total_limit} sessions reached (delete old sessions to reclaim quota)"
            );
        }
        let per_camp_limit = self.quotas.max_sessions_per_campaign.saturating_mul(mult);
        let camp_used: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM saved_sessions WHERE campaign_id = ?1",
        )
        .bind(params.campaign_id)
        .fetch_one(&self.db_pool)
        .await
        .unwrap_or(0);
        if (camp_used.max(0) as usize) >= per_camp_limit {
            anyhow::bail!(
                "Quota exceeded: campaign '{}' reached the limit of {per_camp_limit} sessions",
                params.campaign_id
            );
        }

        let random_pw = format!("satya-aprv-{}", Uuid::new_v4());
        let password_hash = Self::hash_secret_async(random_pw).await?;
        let now_us = Utc::now().timestamp_micros();
        let owner_role = match params.session_mode {
            SessionMode::Ttrpg => SessionRole::Gm,
            SessionMode::Meeting => SessionRole::Host,
        };
        let owner_part = Participant::human(
            &owner.user_id,
            &owner.username,
            &owner.display_name,
            owner_role,
        );
        let clean_title = params.title.trim();
        let session_obj = ProtectedSession {
            session_id: sid.to_string(),
            password_hash,
            session_mode: params.session_mode,
            host_user_id: owner.user_id.clone(),
            owner_user_id: owner.user_id.clone(),
            campaign_id: Some(params.campaign_id.to_string()),
            title: if clean_title.is_empty() {
                None
            } else {
                Some(clean_title.to_string())
            },
            is_discoverable: true,
            allow_join_requests: true,
            roster: HashMap::new(),
            ai_gm_vault: None,
            active_map: None,
            gm_documents: Vec::new(),
            campaign_tag: params.campaign_slug.to_string(),
            gm_memories: Vec::new(),
            combat_tracker: CombatTrackerState::default(),
            created_at_us: now_us,
        };
        self.persist_session_metadata_to_db(&session_obj, params.enable_ai_gm)
            .await;
        self.persist_session_participant_to_db(sid, &owner_part)
            .await;
        Ok(())
    }

    /// Creates a password-protected session with `session_id` and `session_password` using default metadata.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_session(
        &self,
        user: &UserAccount,
        session_id: &str,
        session_password: &str,
        session_mode: SessionMode,
        role: SessionRole,
        display_name_override: Option<String>,
        enable_ai_gm: bool,
    ) -> anyhow::Result<(String, Participant, Vec<Participant>)> {
        self.create_session_with_params(
            user,
            CreateSessionParams {
                session_id,
                session_password,
                session_mode,
                role,
                display_name_override,
                enable_ai_gm,
                campaign_id: None,
                title: None,
                is_discoverable: true,
                allow_join_requests: true,
            },
        )
        .await
    }

    /// Creates or reopens a password-protected, GM-owned session with optional parent campaign binding
    /// and anti-bot quota enforcement.
    pub async fn create_session_with_params(
        &self,
        user: &UserAccount,
        params: CreateSessionParams<'_>,
    ) -> anyhow::Result<(String, Participant, Vec<Participant>)> {
        if !user.account_role.can_host_sessions() {
            anyhow::bail!(
                "Account role '{}' cannot host campaigns or sessions; GM or Admin privilege is required",
                user.account_role.as_db_str()
            );
        }

        let sid = crate::security::validate_session_slug(params.session_id)
            .map_err(|e| anyhow::anyhow!(e))?;
        if params.session_password.is_empty() {
            anyhow::bail!("session_password cannot be empty");
        }
        crate::security::validate_password_upper_bound(params.session_password)
            .map_err(|e| anyhow::anyhow!(e))?;
        let clean_display_override = match params
            .display_name_override
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(d) => Some(
                crate::security::validate_display_name(d).map_err(|e| anyhow::anyhow!(e))?,
            ),
            None => None,
        };
        let clean_title = match params
            .title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(t) => Some(
                crate::security::validate_title("Session title", t, 128)
                    .map_err(|e| anyhow::anyhow!(e))?,
            ),
            None => None,
        };
        let sid = sid.as_str();

        // Validate role for session creation
        match params.session_mode {
            SessionMode::Ttrpg => {
                if !matches!(
                    params.role,
                    SessionRole::Gm
                        | SessionRole::SoloGm
                        | SessionRole::Player
                        | SessionRole::Observer
                ) {
                    anyhow::bail!(
                        "TTRPG session creator must be GM, SOLO_GM, or PLAYER/OBSERVER with AI GM enabled"
                    );
                }
                if matches!(params.role, SessionRole::SoloGm | SessionRole::Observer)
                    && !params.enable_ai_gm
                {
                    anyhow::bail!(
                        "SOLO_GM and OBSERVER TTRPG session creators require AI GM enabled"
                    );
                }
            }
            SessionMode::Meeting => {
                if params.role != SessionRole::Host {
                    anyhow::bail!("Meeting creator must have the HOST role");
                }
            }
        }

        let live_meta = {
            let guard = self.sessions.read().await;
            guard.get(sid).map(|s| {
                let has_active_participants = s.roster.values().any(|p| {
                    p.participant_kind == ParticipantKind::Human
                        || (p.participant_kind == ParticipantKind::AiAgent
                            && p.role == SessionRole::Player)
                });
                (
                    s.host_user_id.clone(),
                    s.password_hash.clone(),
                    s.active_map.clone(),
                    s.created_at_us,
                    has_active_participants,
                )
            })
        };
        if let Some((_, _, _, _, true)) = live_meta {
            anyhow::bail!(
                "Session '{sid}' is already live; use /api/sessions/join or /api/sessions/resume"
            );
        }

        let db_meta = self.load_saved_session_meta_from_db(sid).await;
        let existing_meta = db_meta.or_else(|| live_meta.map(|(h, pw, am, cr, _)| (h, pw, am, cr)));

        let clean_campaign_id = params
            .campaign_id
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned);

        let campaign_slug = if let Some(ref cid) = clean_campaign_id {
            let row = sqlx::query(
                "SELECT owner_user_id, slug, is_archived FROM campaigns WHERE campaign_id = ?1",
            )
            .bind(cid)
            .fetch_optional(&self.db_pool)
            .await?;
            let Some(row) = row else {
                anyhow::bail!("Campaign '{cid}' does not exist");
            };
            let camp_owner: String = row.try_get("owner_user_id")?;
            let slug: String = row.try_get("slug")?;
            let is_archived: i64 = row.try_get("is_archived").unwrap_or(0);
            if camp_owner != user.user_id && user.account_role != AccountRole::Admin {
                anyhow::bail!("Campaign '{cid}' is owned by another GM");
            }
            if is_archived != 0 {
                anyhow::bail!("Cannot create a new session in archived campaign '{cid}'");
            }
            Some(slug)
        } else {
            None
        };

        let is_new_session = existing_meta.is_none();
        if is_new_session {
            let mult = if user.account_role == AccountRole::Admin {
                10
            } else {
                1
            };
            let total_limit = self.quotas.max_total_sessions_per_gm.saturating_mul(mult);
            let total_used: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM saved_sessions WHERE COALESCE(NULLIF(owner_user_id, ''), host_user_id) = ?1",
            )
            .bind(&user.user_id)
            .fetch_one(&self.db_pool)
            .await
            .unwrap_or(0);
            if (total_used.max(0) as usize) >= total_limit {
                anyhow::bail!(
                    "Quota exceeded: maximum {total_limit} sessions per GM reached (delete old sessions to reclaim quota)"
                );
            }
            if let Some(ref cid) = clean_campaign_id {
                let per_camp_limit = self.quotas.max_sessions_per_campaign.saturating_mul(mult);
                let camp_used: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM saved_sessions WHERE campaign_id = ?1",
                )
                .bind(cid)
                .fetch_one(&self.db_pool)
                .await
                .unwrap_or(0);
                if (camp_used.max(0) as usize) >= per_camp_limit {
                    anyhow::bail!(
                        "Quota exceeded: campaign '{cid}' reached the limit of {per_camp_limit} sessions"
                    );
                }
            }
        }

        let (password_hash, saved_active_map, created_at_us) =
            if let Some((saved_host, saved_pw_hash, saved_map, saved_created_at)) = existing_meta {
                let password_ok = Self::verify_secret_async(
                    params.session_password.to_string(),
                    saved_pw_hash.clone(),
                )
                .await;
                let saved_role = self.load_saved_participant_role(sid, &user.user_id).await;
                let role_ok = !params.role.is_human_gm_or_co_gm()
                    || saved_role.is_some_and(SessionRole::is_human_gm_or_co_gm)
                    || (params.role == SessionRole::SoloGm && params.enable_ai_gm);
                if saved_host != user.user_id || !password_ok || !role_ok {
                    anyhow::bail!(
                        "Session '{sid}' already exists; use /api/sessions/join or /api/sessions/resume"
                    );
                }
                (saved_pw_hash, saved_map, saved_created_at)
            } else {
                (
                    Self::hash_secret_async(params.session_password.to_string()).await?,
                    None,
                    Utc::now().timestamp_micros(),
                )
            };

        let mut self_participant = Participant::human(
            &user.user_id,
            &user.username,
            clean_display_override.unwrap_or_else(|| user.display_name.clone()),
            params.role,
        );
        if let Some(bound_cid) = self
            .load_saved_participant_character_id(sid, &user.user_id)
            .await
        {
            self_participant.character_id = Some(bound_cid);
        }

        let mut roster = HashMap::new();
        roster.insert(self_participant.user_id.clone(), self_participant.clone());

        if params.session_mode == SessionMode::Ttrpg && params.enable_ai_gm {
            let ai_gm = Participant::ai_gm(sid, "satya-gm-v1", "harsh-reality-arbiter");
            roster.insert(ai_gm.user_id.clone(), ai_gm);
        }

        let roster_vec: Vec<Participant> = roster.values().cloned().collect();
        let effective_campaign_tag = campaign_slug.unwrap_or_else(|| sid.to_string());
        let mut persisted_docs = self
            .load_persistent_gm_docs_from_db(sid, Some(&user.user_id))
            .await;
        if !effective_campaign_tag.eq_ignore_ascii_case(sid) {
            for doc in self
                .load_persistent_gm_docs_from_db(&effective_campaign_tag, Some(&user.user_id))
                .await
            {
                if !persisted_docs.iter().any(|d| d.doc_id == doc.doc_id) {
                    persisted_docs.push(doc);
                }
            }
        }
        let mut persisted_memories = self
            .load_persistent_gm_memories_from_db(sid, Some(&user.user_id))
            .await;
        if !effective_campaign_tag.eq_ignore_ascii_case(sid) {
            for mem in self
                .load_persistent_gm_memories_from_db(&effective_campaign_tag, Some(&user.user_id))
                .await
            {
                if !persisted_memories
                    .iter()
                    .any(|m| m.memory_id == mem.memory_id)
                {
                    persisted_memories.push(mem);
                }
            }
        }
        let combat_tracker = self.load_or_init_combat_tracker(sid, &roster_vec).await;

        let session_obj = ProtectedSession {
            session_id: sid.to_string(),
            password_hash,
            session_mode: params.session_mode,
            host_user_id: user.user_id.clone(),
            owner_user_id: user.user_id.clone(),
            campaign_id: clean_campaign_id,
            title: clean_title,
            is_discoverable: params.is_discoverable,
            allow_join_requests: params.allow_join_requests,
            roster,
            ai_gm_vault: None,
            active_map: saved_active_map,
            gm_documents: persisted_docs,
            campaign_tag: effective_campaign_tag,
            gm_memories: persisted_memories,
            combat_tracker,
            created_at_us,
        };

        self.persist_session_metadata_to_db(&session_obj, params.enable_ai_gm)
            .await;
        self.persist_session_participant_to_db(sid, &self_participant)
            .await;

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(sid.to_string(), session_obj);
        }

        let join_token = format!("join_{}", Uuid::new_v4());
        let join_expires_at_us = Utc::now().timestamp_micros()
            + (self.quotas.join_token_ttl_secs as i64).saturating_mul(1_000_000);
        self.join_tokens.write().await.insert(
            join_token.clone(),
            JoinTokenClaims {
                session_id: sid.to_string(),
                session_mode: params.session_mode,
                participant: self_participant.clone(),
                expires_at_us: join_expires_at_us,
            },
        );

        Ok((join_token, self_participant, roster_vec))
    }

    /// Verifies `session_id` + `session_password` and adds the authenticated user as a `Participant`.
    pub async fn join_session(
        &self,
        user: &UserAccount,
        session_id: &str,
        session_password: &str,
        role: SessionRole,
        display_name_override: Option<String>,
    ) -> anyhow::Result<(String, SessionMode, Participant, Vec<Participant>)> {
        let (join_token, mode, part, roster, _) = self
            .join_session_with_approval(
                user,
                session_id,
                session_password,
                None,
                role,
                display_name_override,
            )
            .await?;
        Ok((join_token, mode, part, roster))
    }

    /// Verifies `session_id` via either a user-bound `approval_token` OR `session_password`,
    /// returning `(join_token, session_mode, self_participant, roster, consumed_request_id)`.
    pub async fn join_session_with_approval(
        &self,
        user: &UserAccount,
        session_id: &str,
        session_password: &str,
        approval_token: Option<&str>,
        role: SessionRole,
        display_name_override: Option<String>,
    ) -> anyhow::Result<(
        String,
        SessionMode,
        Participant,
        Vec<Participant>,
        Option<String>,
    )> {
        if role == SessionRole::AiGm {
            anyhow::bail!("Human participants cannot join with the AI_GM role");
        }
        crate::security::validate_password_upper_bound(session_password)
            .map_err(|e| anyhow::anyhow!(e))?;
        let clean_display_override = match display_name_override
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(d) => Some(
                crate::security::validate_display_name(d).map_err(|e| anyhow::anyhow!(e))?,
            ),
            None => None,
        };
        let sid = session_id.trim();
        if !self.sessions.read().await.contains_key(sid) {
            let _ = self.rehydrate_session_from_db(sid).await;
        }
        let saved_cid = self
            .load_saved_participant_character_id(sid, &user.user_id)
            .await;
        let saved_role = self.load_saved_participant_role(sid, &user.user_id).await;

        let clean_approval = approval_token.map(str::trim).filter(|s| !s.is_empty());
        let mut consumed_request_id: Option<String> = None;

        if let Some(tok) = clean_approval {
            if matches!(
                role,
                SessionRole::Gm | SessionRole::Host | SessionRole::SoloGm | SessionRole::AiGm
            ) {
                anyhow::bail!("Approval tokens cannot grant privileged GM/HOST/SOLO_GM roles");
            }
            let now_us = Utc::now().timestamp_micros();
            let row = sqlx::query(
                r#"
                SELECT request_id, requested_role, status, expires_at_us
                FROM session_join_requests
                WHERE approval_token = ?1 AND session_id = ?2 AND requester_user_id = ?3
                "#,
            )
            .bind(tok)
            .bind(sid)
            .bind(&user.user_id)
            .fetch_optional(&self.db_pool)
            .await?;

            let Some(row) = row else {
                anyhow::bail!("Invalid or unauthorized approval token for session '{sid}'");
            };
            let req_id: String = row.try_get("request_id")?;
            let status: String = row.try_get("status")?;
            let expires_at_us: i64 = row.try_get("expires_at_us")?;
            let approved_role_str: String = row.try_get("requested_role")?;
            let approved_role = crate::session_persistence::session_role_from_db(&approved_role_str);

            if status != "APPROVED" {
                anyhow::bail!("Approval token has already been {status}");
            }
            if expires_at_us <= now_us {
                let _ = sqlx::query(
                    "UPDATE session_join_requests SET status = 'EXPIRED', updated_at_us = ?1 WHERE request_id = ?2",
                )
                .bind(now_us)
                .bind(&req_id)
                .execute(&self.db_pool)
                .await;
                anyhow::bail!("Approval token has expired; please submit a new join request");
            }
            if role != approved_role && role != SessionRole::Observer {
                anyhow::bail!(
                    "Approval token is scoped to role {:?}, not {:?}",
                    approved_role,
                    role
                );
            }

            let update_res = sqlx::query(
                r#"
                UPDATE session_join_requests
                SET status = 'CONSUMED', updated_at_us = ?1
                WHERE request_id = ?2 AND status = 'APPROVED'
                "#,
            )
            .bind(now_us)
            .bind(&req_id)
            .execute(&self.db_pool)
            .await?;
            if update_res.rows_affected() != 1 {
                anyhow::bail!("Approval token was already consumed");
            }
            consumed_request_id = Some(req_id);
        }

        let password_hash_opt = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;
            let is_session_owner = session.host_user_id == user.user_id;
            if consumed_request_id.is_none() && !is_session_owner {
                Some(session.password_hash.clone())
            } else {
                None
            }
        };
        if let Some(pw_hash) = password_hash_opt
            && !Self::verify_secret_async(session_password.to_string(), pw_hash).await
        {
            anyhow::bail!("Invalid session password for session '{sid}'");
        }

        let (mode, self_participant, roster_vec) = {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;

            let has_other_solo_gm = session.roster.values().any(|p| {
                p.user_id != user.user_id
                    && p.participant_kind == ParticipantKind::Human
                    && p.role == SessionRole::SoloGm
            });
            if has_other_solo_gm && role != SessionRole::Observer {
                anyhow::bail!(
                    "Solo Co-GM sessions allow only one human player (plus AI GM, AI Players, and optional Observers)"
                );
            }

            if role == SessionRole::SoloGm {
                let has_ai_gm = session.roster.values().any(|p| p.role == SessionRole::AiGm);
                let has_other_human_actor = session.roster.values().any(|p| {
                    p.user_id != user.user_id
                        && p.participant_kind == ParticipantKind::Human
                        && matches!(
                            p.role,
                            SessionRole::Gm | SessionRole::Player | SessionRole::SoloGm
                        )
                });
                if session.host_user_id != user.user_id || !has_ai_gm || has_other_human_actor {
                    anyhow::bail!(
                        "Cannot join as SOLO_GM unless you are the host of a solo AI GM session"
                    );
                }
            }

            if matches!(role, SessionRole::Gm | SessionRole::Host) {
                let existing_role = session
                    .roster
                    .get(&user.user_id)
                    .map(|p| p.role)
                    .or(saved_role);
                if session.host_user_id != user.user_id
                    || !matches!(existing_role, Some(SessionRole::Gm | SessionRole::Host))
                {
                    anyhow::bail!(
                        "Cannot join as GM/HOST without existing session GM/HOST authorization"
                    );
                }
            }

            let mut self_participant = Participant::human(
                &user.user_id,
                &user.username,
                clean_display_override.unwrap_or_else(|| user.display_name.clone()),
                role,
            );
            if let Some(cid) = saved_cid {
                self_participant.character_id = Some(cid);
            }

            session
                .roster
                .insert(self_participant.user_id.clone(), self_participant.clone());
            Self::sync_combatant_into_tracker(&mut session.combat_tracker, &self_participant);
            let roster_vec: Vec<Participant> = session.roster.values().cloned().collect();
            let mode = session.session_mode;
            (mode, self_participant, roster_vec)
        };

        self.persist_session_participant_to_db(sid, &self_participant)
            .await;

        let join_token = format!("join_{}", Uuid::new_v4());
        let join_expires_at_us = Utc::now().timestamp_micros()
            + (self.quotas.join_token_ttl_secs as i64).saturating_mul(1_000_000);
        self.join_tokens.write().await.insert(
            join_token.clone(),
            JoinTokenClaims {
                session_id: sid.to_string(),
                session_mode: mode,
                participant: self_participant.clone(),
                expires_at_us: join_expires_at_us,
            },
        );

        Ok((
            join_token,
            mode,
            self_participant,
            roster_vec,
            consumed_request_id,
        ))
    }

    /// Verifies a WebSocket `join_token` and returns its [`JoinTokenClaims`] if valid and unexpired.
    pub async fn verify_join_token(&self, join_token: &str) -> Option<JoinTokenClaims> {
        let now_us = Utc::now().timestamp_micros();
        let claims = self.join_tokens.read().await.get(join_token).cloned()?;
        if claims.expires_at_us <= now_us {
            self.join_tokens.write().await.remove(join_token);
            return None;
        }
        Some(claims)
    }

    /// Returns a snapshot of all current [`Participant`]s in `session_id`.
    pub async fn get_session_roster(&self, session_id: &str) -> Vec<Participant> {
        let guard = self.sessions.read().await;
        guard
            .get(session_id)
            .map(|s| s.roster.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Returns `true` if `user_id` is the recorded `host_user_id` for `session_id`.
    pub(crate) async fn is_session_host(&self, session_id: &str, user_id: &str) -> bool {
        let guard = self.sessions.read().await;
        guard
            .get(session_id.trim())
            .is_some_and(|s| s.host_user_id == user_id)
    }

    /// Configures or updates the in-memory AI GM provider, model, and ephemeral BYOK API key for `session_id`,
    /// updating the `Participant::ai_gm` entry in the session roster (without exposing `api_key`).
    pub async fn configure_session_ai_gm(
        &self,
        session_id: &str,
        sponsor_user_id: &str,
        cfg: AiGmProviderConfig,
    ) -> anyhow::Result<(Participant, Vec<Participant>)> {
        let sid = session_id.trim();
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(sid)
            .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;

        if session.session_mode != SessionMode::Ttrpg {
            anyhow::bail!("AI GM configuration is only supported for TTRPG sessions");
        }

        let clean_provider = if cfg.provider.trim().is_empty() {
            "ollama".to_string()
        } else {
            cfg.provider.trim().to_lowercase()
        };
        let clean_model = if cfg.model_id.trim().is_empty() {
            "gemma4:12b".to_string()
        } else {
            cfg.model_id.trim().to_string()
        };
        let clean_persona = if cfg.persona_id.trim().is_empty() {
            "harsh-reality-arbiter".to_string()
        } else {
            cfg.persona_id.trim().to_string()
        };
        let composite_model_id = if clean_model.contains('/') {
            clean_model.clone()
        } else {
            format!("{clean_provider}/{clean_model}")
        };

        let new_api_key = if let Some(explicit_key) = cfg
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let safe_key = crate::security::validate_api_key_header_safe(explicit_key)
                .map_err(|e| anyhow::anyhow!(e))?;
            Some(safe_key)
        } else if cfg.clear_api_key {
            None
        } else {
            session
                .ai_gm_vault
                .as_ref()
                .filter(|v| v.provider.eq_ignore_ascii_case(&clean_provider))
                .and_then(|v| v.api_key.clone())
        };

        session.ai_gm_vault = Some(SessionAiGmVault {
            sponsor_user_id: sponsor_user_id.to_string(),
            provider: clean_provider,
            model_id: clean_model,
            vision_model_id: cfg.vision_model_id.filter(|s| !s.trim().is_empty()),
            temperature: cfg.temperature,
            persona_id: clean_persona.clone(),
            base_url: cfg.base_url.filter(|s| !s.trim().is_empty()),
            api_key: new_api_key,
        });

        let ai_uid = format!("ai-gm-{sid}");
        let mut ai_participant = Participant::ai_gm(sid, &composite_model_id, &clean_persona);
        if let Some(info) = ai_participant.ai_agent_info.as_mut() {
            info.temperature = cfg.temperature;
        }
        session.roster.insert(ai_uid, ai_participant.clone());
        let roster_vec: Vec<Participant> = session.roster.values().cloned().collect();
        let vault_snap = session.ai_gm_vault.clone();
        drop(sessions);

        if let Some(v) = vault_snap.as_ref() {
            self.persist_session_ai_gm_meta_to_db(sid, v).await;
        }

        Ok((ai_participant, roster_vec))
    }

    /// Retrieves the authoritative [`GmContextResponse`] and in-memory [`SessionAiGmVault`] for `session_id`.
    pub async fn get_session_gm_context(
        &self,
        session_id: &str,
    ) -> Option<(GmContextResponse, Option<SessionAiGmVault>)> {
        let sid = session_id.trim();
        let (
            session_id_val,
            session_mode,
            roster,
            active_map,
            gm_documents,
            gm_memories,
            combat_tracker,
            campaign_tag,
            vault,
        ) = {
            let guard = self.sessions.read().await;
            let session = guard.get(sid)?;
            (
                session.session_id.clone(),
                session.session_mode,
                session
                    .roster
                    .values()
                    .cloned()
                    .collect::<Vec<Participant>>(),
                session.active_map.clone(),
                session.gm_documents.clone(),
                session.gm_memories.clone(),
                session.combat_tracker.clone(),
                session.campaign_tag.clone(),
                session.ai_gm_vault.clone(),
            )
        };

        let pure_human_gm_uid = roster
            .iter()
            .find(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm)
            .map(|p| p.user_id.clone());
        let solo_co_gm_uid = roster
            .iter()
            .find(|p| {
                p.participant_kind == ParticipantKind::Human && p.role == SessionRole::SoloGm
            })
            .map(|p| p.user_id.clone());
        let (operating_mode, human_gm_user_id) = if let Some(uid) = pure_human_gm_uid {
            (AiGmOperatingMode::CoPilot, Some(uid))
        } else if let Some(uid) = solo_co_gm_uid {
            (AiGmOperatingMode::SoloCoGm, Some(uid))
        } else {
            (AiGmOperatingMode::Autonomous, None)
        };
        let ai_gm_participant = roster.iter().find(|p| p.role == SessionRole::AiGm).cloned();
        let session_characters: Vec<CharacterSheetRecord> =
            self.load_session_characters_from_db(sid, &roster).await;

        let ctx = GmContextResponse {
            session_id: session_id_val,
            session_mode,
            operating_mode,
            ai_gm_participant,
            human_gm_user_id,
            roster,
            active_map,
            gm_documents,
            gm_memories,
            combat_tracker: Some(combat_tracker),
            session_characters,
            campaign_tag: Some(campaign_tag),
        };

        Some((ctx, vault))
    }

    /// Links a Google OIDC identity (`google_sub`) and optional ephemeral `drive.file` access token
    /// to an authenticated Satya `user_id` strictly in memory.
    pub async fn link_google_account(
        &self,
        user_id: &str,
        google_sub: String,
        access_token: Option<String>,
    ) -> GoogleAuthStatusResponse {
        let clean_sub = google_sub.trim().to_string();
        let clean_tok = access_token
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        let has_tok = clean_tok.is_some();
        self.google_links
            .write()
            .await
            .insert(user_id.to_string(), (clean_sub.clone(), clean_tok));
        GoogleAuthStatusResponse {
            user_id: user_id.to_string(),
            google_linked: true,
            google_sub: Some(clean_sub),
            has_ephemeral_drive_token: has_tok,
            enforced_scope: "https://www.googleapis.com/auth/drive.file".to_string(),
            google_oauth_client_id: self.google_oauth_client_id.clone(),
            google_picker_api_key: self.google_picker_api_key.clone(),
        }
    }

    /// Returns the Google OAuth link status for `user_id` without exposing any access token.
    pub async fn get_google_auth_status(&self, user_id: &str) -> GoogleAuthStatusResponse {
        let guard = self.google_links.read().await;
        if let Some((sub, tok)) = guard.get(user_id) {
            GoogleAuthStatusResponse {
                user_id: user_id.to_string(),
                google_linked: true,
                google_sub: Some(sub.clone()),
                has_ephemeral_drive_token: tok.is_some(),
                enforced_scope: "https://www.googleapis.com/auth/drive.file".to_string(),
                google_oauth_client_id: self.google_oauth_client_id.clone(),
                google_picker_api_key: self.google_picker_api_key.clone(),
            }
        } else {
            GoogleAuthStatusResponse {
                user_id: user_id.to_string(),
                google_linked: false,
                google_sub: None,
                has_ephemeral_drive_token: false,
                enforced_scope: "https://www.googleapis.com/auth/drive.file".to_string(),
                google_oauth_client_id: self.google_oauth_client_id.clone(),
                google_picker_api_key: self.google_picker_api_key.clone(),
            }
        }
    }

    /// Retrieves the ephemeral in-memory `drive.file` access token for `user_id`, if present.
    pub async fn get_ephemeral_google_token(&self, user_id: &str) -> Option<String> {
        self.google_links
            .read()
            .await
            .get(user_id)
            .and_then(|(_, tok)| tok.clone())
    }

    /// Appends a sanitized `GM_ONLY` [`GmDocumentRecord`] to `session_id` and returns both the stored
    /// record and the authorized recipient `user_id`s (strictly the session's Human GM/Host and the uploader).
    pub async fn add_session_gm_document(
        &self,
        session_id: &str,
        uploader_user_id: &str,
        mut doc: GmDocumentRecord,
    ) -> anyhow::Result<(GmDocumentRecord, Participant, Vec<String>)> {
        let sid = session_id.trim();
        let mut guard = self.sessions.write().await;
        let session = guard
            .get_mut(sid)
            .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;

        let uploader = session
            .roster
            .get(uploader_user_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Caller is not an active participant in '{sid}'"))?;

        let can_set_campaign_tag = uploader.role.is_gm_or_co_gm();
        doc.uploader_user_id = uploader.user_id.clone();
        doc.uploader_display_name = uploader.display_name.clone();
        if !can_set_campaign_tag
            || doc
                .campaign_tag
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            doc.campaign_tag = Some(session.campaign_tag.clone());
        }
        if let Some(existing) = session
            .gm_documents
            .iter_mut()
            .find(|d| d.doc_id == doc.doc_id)
        {
            *existing = doc.clone();
        } else {
            session.gm_documents.push(doc.clone());
        }

        let mut delivery_ids: Vec<String> = session
            .roster
            .values()
            .filter(|p| {
                p.participant_kind == ParticipantKind::Human
                    && (p.role.is_human_gm_or_co_gm() || p.user_id == uploader.user_id)
            })
            .map(|p| p.user_id.clone())
            .collect();
        delivery_ids.sort();
        delivery_ids.dedup();
        drop(guard);

        if doc.is_persistent {
            let _ = self.persist_gm_doc_to_db(&doc).await;
        }

        Ok((doc, uploader, delivery_ids))
    }

    /// Lists `GM_ONLY` documents in `session_id` visible to `caller_user_id`:
    /// - Human GM (`SessionRole::Gm`), Solo Co-GM (`SessionRole::SoloGm`), or Host (`SessionRole::Host`) receives all submitted documents.
    /// - Players/Participants receive only their own submitted documents (never another player's document).
    pub async fn list_session_gm_documents_for_user(
        &self,
        session_id: &str,
        caller_user_id: &str,
    ) -> anyhow::Result<Vec<GmDocumentRecord>> {
        let sid = session_id.trim();
        let guard = self.sessions.read().await;
        let session = guard
            .get(sid)
            .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;

        let caller = session
            .roster
            .get(caller_user_id)
            .ok_or_else(|| anyhow::anyhow!("Caller is not an active participant in '{sid}'"))?;

        if caller.role.is_human_gm_or_co_gm() {
            Ok(session.gm_documents.clone())
        } else {
            Ok(session
                .gm_documents
                .iter()
                .filter(|d| d.uploader_user_id == caller_user_id)
                .cloned()
                .collect())
        }
    }

    /// Updates the active tactical map stored in `session_id`.
    pub async fn set_session_active_map(
        &self,
        session_id: &str,
        map: GmMapUpdateRequest,
    ) -> anyhow::Result<()> {
        let sid = session_id.trim();
        {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;
            session.active_map = Some(map.clone());
        }
        self.persist_session_active_map_to_db(sid, &map).await;
        Ok(())
    }

    /// Removes a user from a session's active roster and invalidates their join token(s) for that session.
    /// Also immediately scrubs any ephemeral BYOK API key sponsored by `user_id` or when `remaining_human_count == 0`.
    /// Returns `Some(SessionDisconnectSummary)` if the user was present in the roster, or `None` if already removed.
    pub async fn leave_session(
        &self,
        session_id: &str,
        user_id: &str,
    ) -> Option<SessionDisconnectSummary> {
        self.leave_session_with_options(session_id, user_id, true, None)
            .await
    }

    /// Removes `user_id` from `session_id` with control over explicit leaves vs. transient WebSocket drops.
    ///
    /// On transient WS disconnects (`explicit_leave == false`) when autonomous AI Players remain in the
    /// session, the sponsor's BYOK key is retained for a short reload grace window (scrubbed by caller if
    /// the sponsor does not reconnect), and only `closing_join_token` is revoked rather than newer tabs' tokens.
    pub async fn leave_session_with_options(
        &self,
        session_id: &str,
        user_id: &str,
        explicit_leave: bool,
        closing_join_token: Option<&str>,
    ) -> Option<SessionDisconnectSummary> {
        let sid = session_id.trim();
        let summary = {
            let mut sessions = self.sessions.write().await;
            let session = sessions.get_mut(sid)?;
            let departed_participant = session.roster.remove(user_id)?;
            let remaining_roster: Vec<Participant> = session.roster.values().cloned().collect();
            let remaining_human_count = remaining_roster
                .iter()
                .filter(|p| p.participant_kind == ParticipantKind::Human)
                .count();
            let has_active_ai_players = remaining_roster.iter().any(|p| {
                p.participant_kind == ParticipantKind::AiAgent && p.role == SessionRole::Player
            });

            // Scrub sponsoring BYOK API keys immediately on explicit leave (or when no AI Players remain);
            // transient WS drops with active AI Players rely on the 30s reconnect grace scrubber.
            let defer_byok_scrub = !explicit_leave && has_active_ai_players;
            if !defer_byok_scrub
                && let Some(vault) = session.ai_gm_vault.as_mut()
                && vault.sponsor_user_id == user_id
            {
                vault.api_key = None;
            }
            if remaining_human_count == 0 && !has_active_ai_players {
                session.ai_gm_vault = None;
                session.gm_documents.retain(|d| d.is_persistent);
                session.gm_memories.retain(|m| m.is_persistent);
            }

            SessionDisconnectSummary {
                session_id: sid.to_string(),
                session_mode: session.session_mode,
                departed_participant,
                remaining_roster,
                remaining_human_count,
                has_active_ai_players,
            }
        };

        let mut jtok_guard = self.join_tokens.write().await;
        if explicit_leave {
            jtok_guard.retain(|_, claims| {
                !(claims.session_id == sid && claims.participant.user_id == user_id)
            });
        } else if let Some(tok) = closing_join_token {
            jtok_guard.remove(tok);
        }

        Some(summary)
    }

    /// Scrubs a departed sponsor's BYOK key if they have still not rejoined `session_id` after the reload grace period.
    pub async fn scrub_sponsor_byok_key_if_absent(&self, session_id: &str, sponsor_user_id: &str) {
        let mut sessions = self.sessions.write().await;
        let Some(session) = sessions.get_mut(session_id.trim()) else {
            return;
        };
        if session.roster.contains_key(sponsor_user_id) {
            return;
        }
        if let Some(vault) = session.ai_gm_vault.as_mut()
            && vault.sponsor_user_id == sponsor_user_id
        {
            vault.api_key = None;
        }
    }

    /// Revokes an authentication token and disconnects the user from any active sessions they are participating in.
    pub async fn logout_user(
        &self,
        token: &str,
    ) -> Option<(UserAccount, Vec<SessionDisconnectSummary>)> {
        let clean = token.strip_prefix("Bearer ").unwrap_or(token).trim();
        let entry = self.auth_tokens.write().await.remove(clean)?;
        let account = entry.account;
        let _ = sqlx::query("DELETE FROM auth_tokens WHERE token = ?1")
            .bind(clean)
            .execute(&self.db_pool)
            .await;
        if let Some(entry) = self.google_links.write().await.get_mut(&account.user_id) {
            entry.1 = None;
        }

        let session_ids: Vec<String> = {
            let sessions = self.sessions.read().await;
            sessions
                .values()
                .filter(|s| s.roster.contains_key(&account.user_id))
                .map(|s| s.session_id.clone())
                .collect()
        };

        let mut disconnected = Vec::new();
        for sid in session_ids {
            if let Some(summary) = self.leave_session(&sid, &account.user_id).await {
                disconnected.push(summary);
            }
        }

        Some((account, disconnected))
    }

    /// Revokes ALL active `auth_tokens` and `join_tokens` belonging to `user_id` and leaves all sessions.
    pub async fn revoke_all_user_tokens(&self, user_id: &str) -> Vec<SessionDisconnectSummary> {
        self.auth_tokens
            .write()
            .await
            .retain(|_, entry| entry.account.user_id != user_id);
        let _ = sqlx::query("DELETE FROM auth_tokens WHERE user_id = ?1")
            .bind(user_id)
            .execute(&self.db_pool)
            .await;
        self.join_tokens
            .write()
            .await
            .retain(|_, claims| claims.participant.user_id != user_id);
        if let Some(entry) = self.google_links.write().await.get_mut(user_id) {
            entry.1 = None;
        }

        let session_ids: Vec<String> = {
            let sessions = self.sessions.read().await;
            sessions
                .values()
                .filter(|s| s.roster.contains_key(user_id))
                .map(|s| s.session_id.clone())
                .collect()
        };
        let mut disconnected = Vec::new();
        for sid in session_ids {
            if let Some(summary) = self.leave_session(&sid, user_id).await {
                disconnected.push(summary);
            }
        }
        disconnected
    }

    /// Removes expired `auth_tokens` and `join_tokens` from memory and SQLite.
    pub async fn prune_expired_tokens(&self) {
        let now_us = Utc::now().timestamp_micros();
        self.auth_tokens
            .write()
            .await
            .retain(|_, entry| entry.expires_at_us > now_us);
        let _ = sqlx::query(
            "DELETE FROM auth_tokens WHERE expires_at_us > 0 AND expires_at_us <= ?1",
        )
        .bind(now_us)
        .execute(&self.db_pool)
        .await;
        self.join_tokens
            .write()
            .await
            .retain(|_, claims| claims.expires_at_us > now_us);
    }

    /// Drains all active sessions for graceful shutdown lifecycle logging.
    pub async fn drain_active_sessions(&self) -> Vec<ProtectedSession> {
        let mut sessions = self.sessions.write().await;
        self.join_tokens.write().await.clear();
        sessions.drain().map(|(_, s)| s).collect()
    }

    /// Returns a reference to the underlying SQLite connection pool.
    pub fn db_pool(&self) -> &SqlitePool {
        &self.db_pool
    }

    /// Binds `character_id` (and optionally updates `display_name`) on the caller's [`Participant`] record in `session_id`,
    /// returning `(updated_participant, updated_roster)`.
    pub async fn bind_character_to_session(
        &self,
        session_id: &str,
        user_id: &str,
        character_id: &str,
        character_name: &str,
    ) -> anyhow::Result<(Participant, Vec<Participant>)> {
        let sid = session_id.trim();
        let updated = {
            let mut guard = self.sessions.write().await;
            let session = guard
                .get_mut(sid)
                .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;
            let part = session
                .roster
                .get_mut(user_id)
                .ok_or_else(|| anyhow::anyhow!("Caller is not an active participant in '{sid}'"))?;
            if part.role == SessionRole::Observer {
                anyhow::bail!("Observers cannot bind character sheets to a session");
            }
            part.character_id = Some(character_id.to_string());
            if !character_name.trim().is_empty() {
                part.display_name = character_name.trim().to_string();
            }
            let updated_part = part.clone();
            let roster_vec: Vec<Participant> = session.roster.values().cloned().collect();
            (updated_part, roster_vec)
        };

        let mut join_guard = self.join_tokens.write().await;
        for claims in join_guard.values_mut() {
            if claims.session_id == sid && claims.participant.user_id == user_id {
                claims.participant = updated.0.clone();
            }
        }
        drop(join_guard);

        self.persist_session_participant_to_db(sid, &updated.0)
            .await;

        Ok(updated)
    }
}
