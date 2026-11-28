//! Pluggable outbound email notification transport for offline GM alerts and account recovery.
//!
//! Supports zero-network [`LocalOutboxEmailNotifier`] (persisting into SQLite `email_outbox`
//! and structured logs) as well as GCP-ready HTTP API dispatch (`Resend`, `SendGrid`, `Mailgun`,
//! or generic `Webhook`) via `EmailConfig`.

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::Utc;
use satya_storage::{EmailConfig, EmailProviderKind};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// Outbound email payload constructed for a Satya user (join requests or account recovery).
#[derive(Debug, Clone)]
pub struct OutboundEmail {
    /// Destination email address of the recipient.
    pub to_email: String,
    /// Email subject line.
    pub subject: String,
    /// Plain-text email body.
    pub body_text: String,
    /// Associated request or recovery correlation identifier (`jreq_<uuid>` or `rec_<uuid>`).
    pub request_id: String,
}

/// Delivery receipt returned by an [`EmailNotifier`].
#[derive(Debug, Clone)]
pub struct EmailDeliveryResult {
    /// Logical delivery channel recorded in `SessionAccessEvent` (`"EMAIL"` or `"LOCAL_OUTBOX"`).
    pub channel: String,
    /// Provider name used to dispatch the message.
    pub provider: String,
    /// Whether an external HTTP provider accepted the email (always `true` for local outbox).
    pub delivered: bool,
}

/// Pluggable async email delivery interface.
#[async_trait]
pub trait EmailNotifier: Send + Sync {
    /// Delivers (or records in local outbox) an outbound notification email.
    async fn send_email(&self, msg: &OutboundEmail) -> anyhow::Result<EmailDeliveryResult>;
}

/// Offline / dev-mode email notifier that writes outbound messages to SQLite `email_outbox`
/// and emits structured `tracing` logs without making external network calls.
#[derive(Debug, Clone)]
pub struct LocalOutboxEmailNotifier {
    db_pool: SqlitePool,
}

impl LocalOutboxEmailNotifier {
    /// Creates a new [`LocalOutboxEmailNotifier`] backed by `db_pool`.
    #[must_use]
    pub fn new(db_pool: SqlitePool) -> Self {
        Self { db_pool }
    }
}

/// Sanitizes an email subject to prevent CRLF header or log injection.
fn sanitize_email_subject(raw: &str) -> String {
    raw.chars()
        .map(|c| if c == '\r' || c == '\n' || c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .chars()
        .take(240)
        .collect()
}

#[async_trait]
impl EmailNotifier for LocalOutboxEmailNotifier {
    async fn send_email(&self, msg: &OutboundEmail) -> anyhow::Result<EmailDeliveryResult> {
        let safe_subject = sanitize_email_subject(&msg.subject);
        let outbox_id = format!("mail_{}", Uuid::now_v7());
        let now_us = Utc::now().timestamp_micros();
        sqlx::query(
            r#"
            INSERT INTO email_outbox (outbox_id, to_email, subject, body_text, request_id, status, created_at_us)
            VALUES (?1, ?2, ?3, ?4, ?5, 'QUEUED_LOCAL', ?6)
            "#,
        )
        .bind(&outbox_id)
        .bind(&msg.to_email)
        .bind(&safe_subject)
        .bind(&msg.body_text)
        .bind(&msg.request_id)
        .bind(now_us)
        .execute(&self.db_pool)
        .await?;

        info!(
            outbox_id = %outbox_id,
            request_id = %msg.request_id,
            subject = %safe_subject,
            "Recorded outbound email in local email_outbox"
        );

        Ok(EmailDeliveryResult {
            channel: "EMAIL".to_string(),
            provider: "local_outbox".to_string(),
            delivered: true,
        })
    }
}

/// Production HTTP API email notifier (`Resend`, `SendGrid`, `Mailgun`, or custom `Webhook`)
/// reading its credential from the environment variable named in [`EmailConfig::api_key_env`].
pub struct HttpEmailNotifier {
    config: EmailConfig,
    client: reqwest::Client,
    fallback: LocalOutboxEmailNotifier,
}

impl HttpEmailNotifier {
    /// Constructs a new [`HttpEmailNotifier`] with a 10-second HTTP timeout.
    #[must_use]
    pub fn new(config: EmailConfig, db_pool: SqlitePool) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            client,
            fallback: LocalOutboxEmailNotifier::new(db_pool),
        }
    }

    async fn record_delivery_status(&self, msg: &OutboundEmail, status: &str) {
        let outbox_id = format!("mail_{}", Uuid::now_v7());
        let now_us = Utc::now().timestamp_micros();
        let _ = sqlx::query(
            r#"
            INSERT INTO email_outbox (outbox_id, to_email, subject, body_text, request_id, status, created_at_us)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(&outbox_id)
        .bind(&msg.to_email)
        .bind(&msg.subject)
        .bind(&msg.body_text)
        .bind(&msg.request_id)
        .bind(status)
        .bind(now_us)
        .execute(&self.fallback.db_pool)
        .await;
    }
}

#[async_trait]
impl EmailNotifier for HttpEmailNotifier {
    async fn send_email(&self, msg: &OutboundEmail) -> anyhow::Result<EmailDeliveryResult> {
        let mut sanitized_msg = msg.clone();
        sanitized_msg.subject = sanitize_email_subject(&msg.subject);
        let msg = &sanitized_msg;

        if !self.config.enabled || self.config.provider == EmailProviderKind::LocalOutbox {
            return self.fallback.send_email(msg).await;
        }

        let api_key = std::env::var(&self.config.api_key_env)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let Some(key) = api_key else {
            warn!(
                env_var = %self.config.api_key_env,
                "Email API key env var missing/empty; falling back to local email_outbox"
            );
            return self.fallback.send_email(msg).await;
        };

        let resp = match self.config.provider {
            EmailProviderKind::LocalOutbox => return self.fallback.send_email(msg).await,
            EmailProviderKind::Resend => {
                let url = self
                    .config
                    .api_base_url
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("https://api.resend.com/emails");
                self.client
                    .post(url)
                    .bearer_auth(&key)
                    .json(&json!({
                        "from": self.config.from_address,
                        "to": [msg.to_email],
                        "subject": msg.subject,
                        "text": msg.body_text,
                    }))
                    .send()
                    .await
            }
            EmailProviderKind::SendGrid => {
                let url = self
                    .config
                    .api_base_url
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("https://api.sendgrid.com/v3/mail/send");
                self.client
                    .post(url)
                    .bearer_auth(&key)
                    .json(&json!({
                        "personalizations": [{ "to": [{ "email": msg.to_email }] }],
                        "from": { "email": self.config.from_address },
                        "subject": msg.subject,
                        "content": [{ "type": "text/plain", "value": msg.body_text }],
                    }))
                    .send()
                    .await
            }
            EmailProviderKind::Mailgun => {
                let base = self
                    .config
                    .api_base_url
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("Mailgun provider requires email.api_base_url"))?;
                let url = format!("{}/messages", base.trim_end_matches('/'));
                let auth_b64 = BASE64_STANDARD.encode(format!("api:{key}"));
                self.client
                    .post(&url)
                    .header("Authorization", format!("Basic {auth_b64}"))
                    .form(&[
                        ("from", self.config.from_address.as_str()),
                        ("to", msg.to_email.as_str()),
                        ("subject", msg.subject.as_str()),
                        ("text", msg.body_text.as_str()),
                    ])
                    .send()
                    .await
            }
            EmailProviderKind::Webhook => {
                let url = self
                    .config
                    .api_base_url
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("Webhook provider requires email.api_base_url"))?;
                self.client
                    .post(url)
                    .bearer_auth(&key)
                    .json(&json!({
                        "from": self.config.from_address,
                        "to": msg.to_email,
                        "subject": msg.subject,
                        "text": msg.body_text,
                        "request_id": msg.request_id,
                    }))
                    .send()
                    .await
            }
        };

        let provider_name = format!("{:?}", self.config.provider).to_lowercase();
        match resp {
            Ok(r) if r.status().is_success() => {
                self.record_delivery_status(msg, "SENT_HTTP").await;
                Ok(EmailDeliveryResult {
                    channel: "EMAIL".to_string(),
                    provider: provider_name,
                    delivered: true,
                })
            }
            Ok(r) => {
                let code = r.status();
                warn!(status = %code, provider = %provider_name, "Email API returned non-success; recording in local outbox");
                self.record_delivery_status(msg, &format!("FALLBACK_HTTP_{code}"))
                    .await;
                Ok(EmailDeliveryResult {
                    channel: "EMAIL".to_string(),
                    provider: provider_name,
                    delivered: false,
                })
            }
            Err(err) => {
                warn!(error = %err, provider = %provider_name, "Email API request failed; recording in local outbox");
                self.record_delivery_status(msg, "FALLBACK_NETWORK_ERR")
                    .await;
                Ok(EmailDeliveryResult {
                    channel: "EMAIL".to_string(),
                    provider: provider_name,
                    delivered: false,
                })
            }
        }
    }
}

/// Builds the active [`EmailNotifier`] implementation from [`EmailConfig`].
#[must_use]
pub fn build_email_notifier(config: &EmailConfig, db_pool: SqlitePool) -> Arc<dyn EmailNotifier> {
    if !config.enabled || config.provider == EmailProviderKind::LocalOutbox {
        Arc::new(LocalOutboxEmailNotifier::new(db_pool))
    } else {
        Arc::new(HttpEmailNotifier::new(config.clone(), db_pool))
    }
}
