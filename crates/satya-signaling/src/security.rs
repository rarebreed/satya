//! Shared security validation helpers, SSRF guards, WebSocket Origin checks,
//! authenticated `/maps/*` asset handler, and HTTP security headers middleware.

use crate::rate_limit::trusted_proxy_hops;
use crate::state::SignalingAppState;
use axum::{
    Json,
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

/// Maximum length of a user account or session password (bytes / chars).
pub const MAX_PASSWORD_CHARS: usize = 128;
/// Maximum byte length of a password accepted prior to Argon2 hashing.
pub const MAX_PASSWORD_BYTES: usize = 512;
/// Minimum length of a newly registered user account password.
pub const MIN_ACCOUNT_PASSWORD_CHARS: usize = 8;
/// Maximum byte length for a live WebSocket chat message body (16 KiB).
pub const MAX_WS_CHAT_BODY_BYTES: usize = 16 * 1024;
/// Maximum byte length for a WebRTC SDP/ICE relay payload (64 KiB).
pub const MAX_WS_WEBRTC_PAYLOAD_BYTES: usize = 64 * 1024;

/// Validates a normalized lowercase username (`3..=32` chars, `[a-z0-9_.-]`).
pub fn validate_and_normalize_username(raw: &str) -> Result<String, String> {
    let uname = raw.trim().to_lowercase();
    let char_len = uname.chars().count();
    if !(3..=32).contains(&char_len) {
        return Err("Username must be between 3 and 32 characters".to_string());
    }
    if !uname
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-'))
    {
        return Err(
            "Username may only contain letters, digits, underscores, dots, and hyphens".to_string(),
        );
    }
    Ok(uname)
}

/// Validates a user account password for registration (`8..=128` chars, `<= 512` bytes).
pub fn validate_account_password(pw: &str) -> Result<(), String> {
    let char_len = pw.chars().count();
    if char_len < MIN_ACCOUNT_PASSWORD_CHARS {
        return Err(format!(
            "Password must be at least {MIN_ACCOUNT_PASSWORD_CHARS} characters"
        ));
    }
    validate_password_upper_bound(pw)
}

/// Enforces the upper bound on a password before Argon2 hashing/verification to prevent CPU DoS.
pub fn validate_password_upper_bound(pw: &str) -> Result<(), String> {
    if pw.len() > MAX_PASSWORD_BYTES || pw.chars().count() > MAX_PASSWORD_CHARS {
        return Err(format!(
            "Password must not exceed {MAX_PASSWORD_CHARS} characters"
        ));
    }
    Ok(())
}

/// Validates a human display name (`1..=64` chars, no control characters).
pub fn validate_display_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Display name cannot be empty".to_string());
    }
    if trimmed.chars().count() > 64 {
        return Err("Display name must not exceed 64 characters".to_string());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("Display name contains invalid control characters".to_string());
    }
    Ok(trimmed.to_string())
}

/// Validates a strict slug identifier (`[A-Za-z0-9_-]{1,64}`) such as `session_id`.
pub fn validate_session_slug(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err("session_id must be between 1 and 64 characters".to_string());
    }
    if !trimmed
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return Err(
            "session_id may only contain ASCII letters, digits, '_' and '-'".to_string(),
        );
    }
    Ok(trimmed.to_string())
}

/// Validates a filesystem-safe resource identifier (`[A-Za-z0-9_.-]{1,64}`, never `..`, `/`, or `\`).
pub fn validate_safe_resource_id(field: &str, raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err(format!("{field} must be between 1 and 64 characters"));
    }
    if trimmed.contains("..")
        || trimmed.starts_with('.')
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        return Err(format!("{field} contains disallowed characters"));
    }
    Ok(trimmed.to_string())
}

/// Validates a title string (`1..=max_chars`, no control characters).
pub fn validate_title(field: &str, raw: &str, max_chars: usize) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} cannot be empty"));
    }
    if trimmed.chars().count() > max_chars {
        return Err(format!("{field} must not exceed {max_chars} characters"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(format!("{field} contains invalid control characters"));
    }
    Ok(trimmed.to_string())
}

/// Validates a description string (`<= max_chars`, strips disallowed control chars except newline/tab).
pub fn validate_description(field: &str, raw: &str, max_chars: usize) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > max_chars {
        return Err(format!("{field} must not exceed {max_chars} characters"));
    }
    if trimmed
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(format!("{field} contains invalid control characters"));
    }
    Ok(trimmed.to_string())
}

/// Validates an ephemeral BYOK API key does not contain CRLF or control characters.
pub fn validate_api_key_header_safe(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.len() > 512 {
        return Err("API key exceeds maximum allowed length".to_string());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("API key contains invalid control characters".to_string());
    }
    Ok(trimmed.to_string())
}

fn is_disallowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_unspecified()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.octets()[0] == 169 && v4.octets()[1] == 254
        }
        IpAddr::V6(v6) => {
            v6.is_unspecified()
                || v6.is_multicast()
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

fn is_private_or_loopback_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || is_disallowed_ip(ip),
        IpAddr::V6(v6) => {
            v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00 || is_disallowed_ip(ip)
        }
    }
}

/// Extracts `(scheme, host_lower, scheme_and_authority)` from an `http://` or `https://` URL.
pub fn parse_http_origin(raw_url: &str) -> Result<(String, String, String), String> {
    let trimmed = raw_url.trim().trim_end_matches('/');
    if trimmed.is_empty() || trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("URL cannot be empty or contain whitespace/control characters".to_string());
    }
    let (scheme, after_scheme) = if let Some(rest) = trimmed.strip_prefix("https://") {
        ("https".to_string(), rest)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        ("http".to_string(), rest)
    } else {
        return Err("URL must use http:// or https:// scheme".to_string());
    };

    let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') {
        return Err("Invalid URL authority".to_string());
    }
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split(']').next().unwrap_or("").to_ascii_lowercase()
    } else {
        authority
            .split(':')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
    };
    if host.is_empty() {
        return Err("URL host cannot be empty".to_string());
    }
    let origin = format!("{scheme}://{authority}");
    Ok((scheme, host, origin))
}

/// Validates a BYOK AI GM provider `base_url` against SSRF rules and `allowed_provider_base_urls`.
pub fn validate_provider_base_url(
    provider: &str,
    raw_url: &str,
    default_ollama_url: &str,
    allowed_provider_base_urls: &[String],
) -> Result<String, String> {
    let (scheme, host, origin) = parse_http_origin(raw_url)?;
    if let Ok(ip) = host.parse::<IpAddr>()
        && is_disallowed_ip(ip)
    {
        return Err(format!(
            "Untrusted provider base_url host '{host}'; link-local/metadata addresses are forbidden"
        ));
    }

    let in_allowlist = allowed_provider_base_urls.iter().any(|allowed| {
        let norm = allowed.trim().trim_end_matches('/');
        !norm.is_empty() && norm.eq_ignore_ascii_case(&origin)
    });
    if in_allowlist {
        return Ok(raw_url.trim().trim_end_matches('/').to_string());
    }

    let norm_provider = provider.trim().to_ascii_lowercase();
    if norm_provider == "ollama" {
        let is_loopback = matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1");
        let matches_default = parse_http_origin(default_ollama_url)
            .is_ok_and(|(_, _, def_origin)| def_origin.eq_ignore_ascii_case(&origin));
        if is_loopback || matches_default {
            return Ok(raw_url.trim().trim_end_matches('/').to_string());
        }
        return Err(format!(
            "Untrusted Ollama base_url host '{host}'; only loopback, configured ollama_base_url, or allowed_provider_base_urls are permitted"
        ));
    }

    let official_host = match norm_provider.as_str() {
        "openai" => Some("api.openai.com"),
        "anthropic" => Some("api.anthropic.com"),
        "google_genai" => Some("generativelanguage.googleapis.com"),
        _ => None,
    };
    if let Some(expected_host) = official_host
        && scheme == "https"
        && host == expected_host
    {
        return Ok(raw_url.trim().trim_end_matches('/').to_string());
    }
    if let Ok(ip) = host.parse::<IpAddr>()
        && is_private_or_loopback_ip(ip)
    {
        return Err(format!(
            "Untrusted provider base_url host '{host}'; private/loopback cloud endpoints require explicit allowlisting"
        ));
    }

    Err(format!(
        "Untrusted provider base_url '{origin}' for provider '{norm_provider}'; add it to allowed_provider_base_urls to permit"
    ))
}

/// Extracts `host[:port]` authority (lowercased) from an HTTP `Origin` header value.
fn origin_authority(origin_hdr: &str) -> Option<String> {
    let trimmed = origin_hdr.trim().trim_end_matches('/');
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))?;
    let auth = rest.split('/').next()?;
    if auth.is_empty() || auth.contains('@') {
        None
    } else {
        Some(auth.to_ascii_lowercase())
    }
}

/// Validates the `Origin` header on WebSocket upgrades against the request `Host` header
/// and optional `SATYA_ALLOWED_ORIGINS` allowlist.
pub fn verify_ws_origin(headers: &HeaderMap) -> Result<(), (StatusCode, &'static str)> {
    let Some(raw_origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        // Non-browser clients (CLI/tests) typically omit `Origin`; when present, it must match.
        return Ok(());
    };
    let origin_clean = raw_origin.trim().trim_end_matches('/');
    if origin_clean.eq_ignore_ascii_case("null") {
        return Err((StatusCode::FORBIDDEN, "WebSocket Origin 'null' is forbidden"));
    }

    if let Ok(allowed_env) = std::env::var("SATYA_ALLOWED_ORIGINS") {
        let allowed_match = allowed_env
            .split(',')
            .map(|s| s.trim().trim_end_matches('/'))
            .filter(|s| !s.is_empty())
            .any(|allowed| allowed.eq_ignore_ascii_case(origin_clean));
        if allowed_match {
            return Ok(());
        }
    }

    let Some(origin_auth) = origin_authority(origin_clean) else {
        return Err((StatusCode::FORBIDDEN, "Invalid WebSocket Origin header"));
    };
    let host_auth = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|h| h.trim().to_ascii_lowercase())
        .unwrap_or_default();

    if !host_auth.is_empty() && origin_auth == host_auth {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "Cross-origin WebSocket upgrade rejected"))
    }
}

/// Route-level Axum middleware rejecting cross-origin WebSocket upgrades before `WebSocketUpgrade` extraction.
pub async fn ws_origin_guard_middleware(req: Request, next: Next) -> Response {
    if let Err(err_resp) = verify_ws_origin(req.headers()) {
        return err_resp.into_response();
    }
    next.run(req).await
}

/// Axum middleware applying defense-in-depth HTTP security headers (CSP, nosniff, X-Frame-Options,
/// Referrer-Policy, Permissions-Policy, COOP, and HSTS when TLS is active).
pub async fn security_headers_middleware(
    State(state): State<Arc<SignalingAppState>>,
    req: Request,
    next: Next,
) -> Response {
    let forwarded_https = trusted_proxy_hops() > 0
        && req
            .headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("https"));
    let is_https = state.tls_enabled || forwarded_https;

    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(
            "camera=(self), microphone=(self), geolocation=(), payment=(), usb=()",
        ),
    );
    headers.insert(
        header::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin-allow-popups"),
    );
    headers.insert(
        header::HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval' 'unsafe-eval' https://accounts.google.com https://apis.google.com; \
             style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
             font-src 'self' https://fonts.gstatic.com; \
             img-src 'self' data: blob: https:; \
             media-src 'self' mediastream: blob:; \
             connect-src 'self' ws: wss: https://www.googleapis.com https://accounts.google.com; \
             frame-src https://accounts.google.com https://docs.google.com; \
             frame-ancestors 'none'; \
             object-src 'none'; \
             base-uri 'self'; \
             form-action 'self'",
        ),
    );
    if is_https {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    resp
}

/// Authenticated, path-traversal-safe static file handler for `GET /maps/*file`.
///
/// Replaces public `ServeDir::new("data/maps")` so map assets require a valid bearer token
/// and SVGs are never served inline without `nosniff`.
pub async fn get_data_map_asset_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Path(rel_path): Path<String>,
) -> impl IntoResponse {
    let Some(auth_hdr) = headers.get(header::AUTHORIZATION).and_then(|h| h.to_str().ok()) else {
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

    let trimmed = rel_path.trim().trim_start_matches('/');
    if trimmed.is_empty()
        || trimmed.contains('\0')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid map asset path" })),
        )
            .into_response();
    }

    let base_dir = PathBuf::from("data").join("maps");
    let Ok(canon_base) = std::fs::canonicalize(&base_dir) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Map asset not found" })),
        )
            .into_response();
    };
    let candidate = canon_base.join(trimmed);
    let Ok(canon_target) = std::fs::canonicalize(&candidate) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Map asset not found" })),
        )
            .into_response();
    };
    if !canon_target.starts_with(&canon_base) || !canon_target.is_file() {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Access denied" })),
        )
            .into_response();
    }

    let Ok(bytes) = tokio::fs::read(&canon_target).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Map asset not found" })),
        )
            .into_response();
    };

    let ext = canon_target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (content_type, disposition) = match ext.as_str() {
        "png" => ("image/png", "inline"),
        "jpg" | "jpeg" => ("image/jpeg", "inline"),
        "webp" => ("image/webp", "inline"),
        "json" => ("application/json", "attachment"),
        "svg" => ("image/svg+xml", "attachment"),
        _ => ("application/octet-stream", "attachment"),
    };

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, disposition),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        Body::from(bytes),
    )
        .into_response()
}
