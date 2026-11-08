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
    AiGmOperatingMode, AiGmProviderConfig, GmContextResponse, GmDocumentRecord, GmMapUpdateRequest,
    GoogleAuthStatusResponse, Participant, ParticipantKind, SessionMode, SessionRole,
};
use satya_storage::AuthConfig;
use serde::{Deserialize, Serialize};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Persistent user account record stored in the SQLite `users` table (`data/auth/users.db`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    /// Persistent UUID (or preset demo ID) identifying this user across sessions.
    pub user_id: String,
    /// Normalized lowercase login username.
    pub username: String,
    /// Default display or character name for this user.
    pub display_name: String,
    /// Argon2id encoded password hash string.
    pub password_hash: String,
    /// Platform-level account role (`USER` or `ADMIN`).
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
    /// Map of active participants in the session keyed by `user_id`.
    pub roster: HashMap<String, Participant>,
    /// Optional in-memory AI GM provider & ephemeral BYOK credential vault.
    pub ai_gm_vault: Option<SessionAiGmVault>,
    /// Optional currently active tactical map in the session.
    pub active_map: Option<GmMapUpdateRequest>,
    /// Sanitized `GM_ONLY` documents submitted to the GM in this session.
    pub gm_documents: Vec<GmDocumentRecord>,
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
}

/// Manages SQLx-backed Argon2id user registration/login, account-level RBAC (`USER` vs. `ADMIN`),
/// bearer authentication tokens, password-protected session creation/joining, and disconnect/logout flows.
pub struct AuthAndSessionManager {
    db_pool: SqlitePool,
    bootstrap_admins: HashSet<String>,
    google_oauth_client_id: Option<String>,
    google_picker_api_key: Option<String>,
    users_by_username: RwLock<HashMap<String, UserAccount>>,
    auth_tokens: RwLock<HashMap<String, UserAccount>>,
    google_links: RwLock<HashMap<String, (String, Option<String>)>>,
    sessions: RwLock<HashMap<String, ProtectedSession>>,
    join_tokens: RwLock<HashMap<String, JoinTokenClaims>>,
}

impl AuthAndSessionManager {
    /// Initializes the SQLx SQLite authentication and session manager rooted at `base_dir`,
    /// running schema migrations, seeding default demo accounts if the `users` table is empty,
    /// and promoting any configured `bootstrap_admins` to [`AccountRole::Admin`].
    pub async fn new(base_dir: impl AsRef<Path>, auth_config: &AuthConfig) -> anyhow::Result<Self> {
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

        let existing_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&db_pool)
            .await?;

        if existing_count == 0 {
            let defaults = [
                ("u-demo-gm-sean", "gm_sean", "Sean (GM)"),
                ("u-demo-sara", "sara_valkyrie", "Sara (Valkyrie)"),
                ("u-demo-marcus", "marcus_recon", "Marcus (Recon)"),
                ("u-demo-host-elena", "host_elena", "Elena (Meeting Host)"),
            ];
            for (uid, uname, dname) in defaults {
                let hash = Self::hash_secret("satya-truth-2026")?;
                let role = if bootstrap_admins.contains(uname) {
                    AccountRole::Admin
                } else {
                    AccountRole::User
                };
                let now_us = Utc::now().timestamp_micros();
                sqlx::query(
                    r#"
                    INSERT INTO users (user_id, username, display_name, password_hash, account_role, created_at_us)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                    "#,
                )
                .bind(uid)
                .bind(uname)
                .bind(dname)
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
        }

        let rows = sqlx::query(
            "SELECT user_id, username, display_name, password_hash, account_role, created_at_us FROM users",
        )
        .fetch_all(&db_pool)
        .await?;

        let mut initial_users = HashMap::with_capacity(rows.len());
        for row in rows {
            let uname: String = row.try_get("username")?;
            let role_str: String = row.try_get("account_role")?;
            initial_users.insert(
                uname.clone(),
                UserAccount {
                    user_id: row.try_get("user_id")?,
                    username: uname,
                    display_name: row.try_get("display_name")?,
                    password_hash: row.try_get("password_hash")?,
                    account_role: AccountRole::from_db_str(&role_str),
                    created_at_us: row.try_get("created_at_us")?,
                },
            );
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
            google_oauth_client_id,
            google_picker_api_key,
            users_by_username: RwLock::new(initial_users),
            auth_tokens: RwLock::new(HashMap::new()),
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

    /// Verifies a plaintext secret against an Argon2id PHC string hash.
    pub fn verify_secret(secret: &str, encoded_hash: &str) -> bool {
        let Ok(parsed_hash) = PasswordHash::new(encoded_hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(secret.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Registers a new user account, persists it to the SQLx SQLite `users` table, and returns a bearer token response.
    /// Self-registered users receive [`AccountRole::User`] unless their username is listed in `bootstrap_admins`.
    pub async fn register_user(
        &self,
        username: &str,
        password: &str,
        display_name: &str,
    ) -> anyhow::Result<AuthSuccessResponse> {
        let uname = username.trim().to_lowercase();
        if uname.is_empty() || password.len() < 4 {
            anyhow::bail!("Username cannot be empty and password must be at least 4 characters");
        }

        let account = {
            let mut guard = self.users_by_username.write().await;
            if guard.contains_key(&uname) {
                anyhow::bail!("Username '{uname}' is already registered");
            }
            let password_hash = Self::hash_secret(password)?;
            let account_role = if self.bootstrap_admins.contains(&uname) {
                AccountRole::Admin
            } else {
                AccountRole::User
            };
            let acct = UserAccount {
                user_id: Uuid::now_v7().to_string(),
                username: uname.clone(),
                display_name: if display_name.trim().is_empty() {
                    uname.clone()
                } else {
                    display_name.trim().to_string()
                },
                password_hash,
                account_role,
                created_at_us: Utc::now().timestamp_micros(),
            };

            sqlx::query(
                r#"
                INSERT INTO users (user_id, username, display_name, password_hash, account_role, created_at_us)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            )
            .bind(&acct.user_id)
            .bind(&acct.username)
            .bind(&acct.display_name)
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
        for acct in tokens.values_mut() {
            if acct.username == uname {
                acct.account_role = role;
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
        let uname = username.trim().to_lowercase();
        let account = {
            let guard = self.users_by_username.read().await;
            guard
                .get(&uname)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Unknown username '{uname}'"))?
        };

        if !Self::verify_secret(password, &account.password_hash) {
            anyhow::bail!("Invalid password for user '{uname}'");
        }

        self.issue_auth_token(account).await
    }

    async fn issue_auth_token(&self, account: UserAccount) -> anyhow::Result<AuthSuccessResponse> {
        let token = format!("satya_tok_{}", Uuid::new_v4());
        self.auth_tokens
            .write()
            .await
            .insert(token.clone(), account.clone());

        Ok(AuthSuccessResponse {
            auth_token: token,
            user_id: account.user_id,
            username: account.username,
            display_name: account.display_name,
            account_role: account.account_role,
        })
    }

    /// Verifies a bearer token (with or without a `"Bearer "` prefix) and returns the associated [`UserAccount`].
    pub async fn verify_auth_token(&self, token: &str) -> Option<UserAccount> {
        let clean = token.strip_prefix("Bearer ").unwrap_or(token).trim();
        self.auth_tokens.read().await.get(clean).cloned()
    }

    /// Creates a password-protected session with `session_id` and `session_password`.
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
        let sid = session_id.trim();
        if sid.is_empty() {
            anyhow::bail!("session_id cannot be empty");
        }
        if session_password.is_empty() {
            anyhow::bail!("session_password cannot be empty");
        }

        // Validate role for session creation
        match session_mode {
            SessionMode::Ttrpg => {
                if !matches!(role, SessionRole::Gm | SessionRole::Player) {
                    anyhow::bail!("TTRPG session creator must be GM or PLAYER (with AI GM)");
                }
            }
            SessionMode::Meeting => {
                if role != SessionRole::Host {
                    anyhow::bail!("Meeting creator must have the HOST role");
                }
            }
        }

        let password_hash = Self::hash_secret(session_password)?;
        let self_participant = Participant::human(
            &user.user_id,
            &user.username,
            display_name_override.unwrap_or_else(|| user.display_name.clone()),
            role,
        );

        let mut roster = HashMap::new();
        roster.insert(self_participant.user_id.clone(), self_participant.clone());

        if session_mode == SessionMode::Ttrpg && enable_ai_gm {
            let ai_gm = Participant::ai_gm(sid, "satya-gm-v1", "harsh-reality-arbiter");
            roster.insert(ai_gm.user_id.clone(), ai_gm);
        }

        let roster_vec: Vec<Participant> = roster.values().cloned().collect();

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(
                sid.to_string(),
                ProtectedSession {
                    session_id: sid.to_string(),
                    password_hash,
                    session_mode,
                    host_user_id: user.user_id.clone(),
                    roster,
                    ai_gm_vault: None,
                    active_map: None,
                    gm_documents: Vec::new(),
                    created_at_us: Utc::now().timestamp_micros(),
                },
            );
        }

        let join_token = format!("join_{}", Uuid::new_v4());
        self.join_tokens.write().await.insert(
            join_token.clone(),
            JoinTokenClaims {
                session_id: sid.to_string(),
                session_mode,
                participant: self_participant.clone(),
            },
        );

        Ok((join_token, self_participant, roster_vec))
    }

    /// Verifies `session_id` and `session_password` and adds the authenticated user as a `Participant`.
    pub async fn join_session(
        &self,
        user: &UserAccount,
        session_id: &str,
        session_password: &str,
        role: SessionRole,
        display_name_override: Option<String>,
    ) -> anyhow::Result<(String, SessionMode, Participant, Vec<Participant>)> {
        let sid = session_id.trim();
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(sid)
            .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;

        if !Self::verify_secret(session_password, &session.password_hash) {
            anyhow::bail!("Invalid session password for session '{sid}'");
        }

        let self_participant = Participant::human(
            &user.user_id,
            &user.username,
            display_name_override.unwrap_or_else(|| user.display_name.clone()),
            role,
        );

        session
            .roster
            .insert(self_participant.user_id.clone(), self_participant.clone());
        let roster_vec: Vec<Participant> = session.roster.values().cloned().collect();
        let mode = session.session_mode;

        drop(sessions);

        let join_token = format!("join_{}", Uuid::new_v4());
        self.join_tokens.write().await.insert(
            join_token.clone(),
            JoinTokenClaims {
                session_id: sid.to_string(),
                session_mode: mode,
                participant: self_participant.clone(),
            },
        );

        Ok((join_token, mode, self_participant, roster_vec))
    }

    /// Verifies a WebSocket `join_token` and returns its [`JoinTokenClaims`] if valid.
    pub async fn verify_join_token(&self, join_token: &str) -> Option<JoinTokenClaims> {
        self.join_tokens.read().await.get(join_token).cloned()
    }

    /// Returns a snapshot of all current [`Participant`]s in `session_id`.
    pub async fn get_session_roster(&self, session_id: &str) -> Vec<Participant> {
        let guard = self.sessions.read().await;
        guard
            .get(session_id)
            .map(|s| s.roster.values().cloned().collect())
            .unwrap_or_default()
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
            "qwen2.5:14b".to_string()
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

        session.ai_gm_vault = Some(SessionAiGmVault {
            sponsor_user_id: sponsor_user_id.to_string(),
            provider: clean_provider,
            model_id: clean_model,
            vision_model_id: cfg.vision_model_id.filter(|s| !s.trim().is_empty()),
            temperature: cfg.temperature,
            persona_id: clean_persona.clone(),
            base_url: cfg.base_url.filter(|s| !s.trim().is_empty()),
            api_key: cfg.api_key.filter(|s| !s.trim().is_empty()),
        });

        let ai_uid = format!("ai-gm-{sid}");
        let mut ai_participant = Participant::ai_gm(sid, &composite_model_id, &clean_persona);
        if let Some(info) = ai_participant.ai_agent_info.as_mut() {
            info.temperature = cfg.temperature;
        }
        session.roster.insert(ai_uid, ai_participant.clone());
        let roster_vec: Vec<Participant> = session.roster.values().cloned().collect();

        Ok((ai_participant, roster_vec))
    }

    /// Retrieves the authoritative [`GmContextResponse`] and in-memory [`SessionAiGmVault`] for `session_id`.
    pub async fn get_session_gm_context(
        &self,
        session_id: &str,
    ) -> Option<(GmContextResponse, Option<SessionAiGmVault>)> {
        let sid = session_id.trim();
        let guard = self.sessions.read().await;
        let session = guard.get(sid)?;

        let roster: Vec<Participant> = session.roster.values().cloned().collect();
        let human_gm_user_id = roster
            .iter()
            .find(|p| p.participant_kind == ParticipantKind::Human && p.role == SessionRole::Gm)
            .map(|p| p.user_id.clone());
        let operating_mode = if human_gm_user_id.is_some() {
            AiGmOperatingMode::CoPilot
        } else {
            AiGmOperatingMode::Autonomous
        };
        let ai_gm_participant = roster.iter().find(|p| p.role == SessionRole::AiGm).cloned();

        let ctx = GmContextResponse {
            session_id: session.session_id.clone(),
            session_mode: session.session_mode,
            operating_mode,
            ai_gm_participant,
            human_gm_user_id,
            roster,
            active_map: session.active_map.clone(),
            gm_documents: session.gm_documents.clone(),
        };

        Some((ctx, session.ai_gm_vault.clone()))
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

        doc.uploader_user_id = uploader.user_id.clone();
        doc.uploader_display_name = uploader.display_name.clone();
        session.gm_documents.push(doc.clone());

        let mut delivery_ids: Vec<String> = session
            .roster
            .values()
            .filter(|p| {
                p.participant_kind == ParticipantKind::Human
                    && (matches!(p.role, SessionRole::Gm | SessionRole::Host)
                        || p.user_id == uploader.user_id)
            })
            .map(|p| p.user_id.clone())
            .collect();
        delivery_ids.sort();
        delivery_ids.dedup();

        Ok((doc, uploader, delivery_ids))
    }

    /// Lists `GM_ONLY` documents in `session_id` visible to `caller_user_id`:
    /// - Human GM (`SessionRole::Gm`) or Host (`SessionRole::Host`) receives all submitted documents.
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

        if matches!(caller.role, SessionRole::Gm | SessionRole::Host) {
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
        let mut guard = self.sessions.write().await;
        let session = guard
            .get_mut(sid)
            .ok_or_else(|| anyhow::anyhow!("Session '{sid}' does not exist"))?;
        session.active_map = Some(map);
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

            // Scrub ephemeral BYOK credentials when the sponsoring user leaves or no humans remain
            if remaining_human_count == 0 {
                session.ai_gm_vault = None;
                session.gm_documents.clear();
            } else if let Some(vault) = session.ai_gm_vault.as_mut()
                && vault.sponsor_user_id == user_id
            {
                vault.api_key = None;
            }

            SessionDisconnectSummary {
                session_id: sid.to_string(),
                session_mode: session.session_mode,
                departed_participant,
                remaining_roster,
                remaining_human_count,
            }
        };

        self.join_tokens.write().await.retain(|_, claims| {
            !(claims.session_id == sid && claims.participant.user_id == user_id)
        });

        Some(summary)
    }

    /// Revokes an authentication token and disconnects the user from any active sessions they are participating in.
    pub async fn logout_user(
        &self,
        token: &str,
    ) -> Option<(UserAccount, Vec<SessionDisconnectSummary>)> {
        let clean = token.strip_prefix("Bearer ").unwrap_or(token).trim();
        let account = self.auth_tokens.write().await.remove(clean)?;
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

        Ok(updated)
    }
}
