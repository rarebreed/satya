use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::Utc;
use satya_schema::{Participant, SessionMode, SessionRole};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub created_at_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSuccessResponse {
    pub auth_token: String,
    pub user_id: String,
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct ProtectedSession {
    pub session_id: String,
    pub password_hash: String,
    pub session_mode: SessionMode,
    pub host_user_id: String,
    pub roster: HashMap<String, Participant>,
    pub created_at_us: i64,
}

#[derive(Debug, Clone)]
pub struct JoinTokenClaims {
    pub session_id: String,
    pub session_mode: SessionMode,
    pub participant: Participant,
}

pub struct AuthAndSessionManager {
    users_file: PathBuf,
    users_by_username: RwLock<HashMap<String, UserAccount>>,
    auth_tokens: RwLock<HashMap<String, UserAccount>>,
    sessions: RwLock<HashMap<String, ProtectedSession>>,
    join_tokens: RwLock<HashMap<String, JoinTokenClaims>>,
}

impl AuthAndSessionManager {
    pub fn new(base_dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let auth_dir = base_dir.as_ref().join("data").join("auth");
        std::fs::create_dir_all(&auth_dir)?;
        let users_file = auth_dir.join("users.json");

        let mut initial_users: HashMap<String, UserAccount> = if users_file.exists() {
            let content = std::fs::read_to_string(&users_file)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        if initial_users.is_empty() {
            let defaults = [
                ("u-demo-gm-sean", "gm_sean", "Sean (GM)"),
                ("u-demo-sara", "sara_valkyrie", "Sara (Valkyrie)"),
                ("u-demo-marcus", "marcus_recon", "Marcus (Recon)"),
                ("u-demo-host-elena", "host_elena", "Elena (Meeting Host)"),
            ];
            for (uid, uname, dname) in defaults {
                let hash = Self::hash_secret("satya-truth-2026")?;
                initial_users.insert(
                    uname.to_string(),
                    UserAccount {
                        user_id: uid.to_string(),
                        username: uname.to_string(),
                        display_name: dname.to_string(),
                        password_hash: hash,
                        created_at_us: Utc::now().timestamp_micros(),
                    },
                );
            }
            let serialized = serde_json::to_string_pretty(&initial_users)?;
            std::fs::write(&users_file, serialized)?;
        }

        Ok(Self {
            users_file,
            users_by_username: RwLock::new(initial_users),
            auth_tokens: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            join_tokens: RwLock::new(HashMap::new()),
        })
    }

    pub fn hash_secret(secret: &str) -> anyhow::Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(secret.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Argon2 hash error: {e}"))?
            .to_string();
        Ok(hash)
    }

    pub fn verify_secret(secret: &str, encoded_hash: &str) -> bool {
        let Ok(parsed_hash) = PasswordHash::new(encoded_hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(secret.as_bytes(), &parsed_hash)
            .is_ok()
    }

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
            let acct = UserAccount {
                user_id: Uuid::now_v7().to_string(),
                username: uname.clone(),
                display_name: if display_name.trim().is_empty() {
                    uname.clone()
                } else {
                    display_name.trim().to_string()
                },
                password_hash,
                created_at_us: Utc::now().timestamp_micros(),
            };
            guard.insert(uname, acct.clone());
            if let Ok(json) = serde_json::to_string_pretty(&*guard) {
                let _ = std::fs::write(&self.users_file, json);
            }
            acct
        };

        self.issue_auth_token(account).await
    }

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
        })
    }

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

    pub async fn verify_join_token(&self, join_token: &str) -> Option<JoinTokenClaims> {
        self.join_tokens.read().await.get(join_token).cloned()
    }

    pub async fn get_session_roster(&self, session_id: &str) -> Vec<Participant> {
        let guard = self.sessions.read().await;
        guard
            .get(session_id)
            .map(|s| s.roster.values().cloned().collect())
            .unwrap_or_default()
    }
}
