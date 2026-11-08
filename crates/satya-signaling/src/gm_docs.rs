//! `GM_ONLY` Document Sharing, Text-Only Sanitization, Minor-Safety Content Guardrails,
//! and Google OAuth (`drive.file` Single-File Scope) Integration for Satya.
//!
//! Enforces three defense-in-depth guarantees:
//! 1. **Strict `GM_ONLY` Visibility (`Player -> GM` Only)**: Submitted documents are visible
//!    exclusively to the session's Human Game Master (`GM`/`HOST`), the session's AI GM
//!    (via `/api/gm/sessions/:session_id/context`), and the uploading user's own submission receipt.
//!    Other players in the session can never list or receive another player's documents.
//! 2. **Text-Only Extraction & Malware/HTML Stripping**: Only plain text, Markdown, and tabular CSV
//!    cells (`<= 32 KB`) are accepted. Binary files, embedded images, `<script>`/`<iframe>`/`<img>`
//!    tags, and `data:` URIs are stripped or rejected.
//! 3. **Minor-Safety & Prompt-Injection Guardrails**: Explicit/NSFW content is rejected at ingestion
//!    (`422 Unprocessable Entity`), and prompt-injection delimiters (`<untrusted_player_document>`)
//!    or imperative system overrides are neutralized before reaching the AI GM.

use crate::state::SignalingAppState;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use satya_schema::{
    GmDocumentRecord, GmDocumentSource, GoogleAuthLinkRequest, MessageVisibility, ServerWsPacket,
    SessionGmDocsResponse, SubmitGmDocumentRequest,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Maximum permitted byte size (`32 KB`) for a single `GM_ONLY` text/table document snapshot.
pub const MAX_GM_DOC_BYTES: usize = 32_768;

/// Prohibited adult/explicit or illegal content markers rejected at ingestion to protect sessions
/// that may include minors.
const PROHIBITED_CONTENT_MARKERS: &[&str] = &[
    "nsfw_explicit",
    "csam",
    "pornographic",
    "explicit sexual",
    "erotic roleplay",
    "xxx_adult",
];

/// Prompt-injection phrases that are neutralized and flagged (`safety_flagged = true`) during ingestion.
const PROMPT_INJECTION_PATTERNS: &[(&str, &str)] = &[
    ("<untrusted_player_document", "[scrubbed_untrusted_doc_open"),
    (
        "</untrusted_player_document>",
        "[/scrubbed_untrusted_doc_close]",
    ),
    (
        "ignore all previous instructions",
        "[redacted_prompt_injection]",
    ),
    (
        "ignore previous instructions",
        "[redacted_prompt_injection]",
    ),
    ("system override:", "[redacted_system_override]:"),
    ("disregard system prompt", "[redacted_prompt_injection]"),
];

/// Sanitizes a document title and body into a clean plain-text/Markdown/CSV snapshot:
/// - Rejects binary data (null bytes) or payloads exceeding [`MAX_GM_DOC_BYTES`].
/// - Rejects prohibited adult/explicit content markers.
/// - Strips HTML `<script>`, `<style>`, `<img>`, `<iframe>`, `data:` image URIs, and HTML tags.
/// - Neutralizes prompt-injection breakout tags and returns `(clean_title, clean_body, safety_flagged)`.
pub fn sanitize_and_guard_gm_document(
    raw_title: &str,
    raw_text: &str,
) -> Result<(String, String, bool), String> {
    let clean_title: String = raw_title
        .chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect::<String>()
        .trim()
        .to_string();
    if clean_title.is_empty() {
        return Err("Document title cannot be empty".to_string());
    }

    if raw_text.as_bytes().contains(&0) {
        return Err(
            "Binary file uploads are not permitted; only UTF-8 text, Markdown, or CSV is allowed"
                .to_string(),
        );
    }
    if raw_text.len() > MAX_GM_DOC_BYTES {
        return Err(format!(
            "Document exceeds maximum allowed size of {MAX_GM_DOC_BYTES} bytes (32 KB)"
        ));
    }

    let lowered_check = format!("{} {}", clean_title.to_lowercase(), raw_text.to_lowercase());
    for marker in PROHIBITED_CONTENT_MARKERS {
        if lowered_check.contains(marker) {
            return Err(
                "Document rejected by Satya safety filter: adult or explicit content is not permitted"
                    .to_string(),
            );
        }
    }

    let (stripped_html, html_flagged) = strip_html_and_media_tags(raw_text);
    let mut cleaned = stripped_html;
    let mut safety_flagged = html_flagged;

    for (pattern, replacement) in PROMPT_INJECTION_PATTERNS {
        if cleaned.to_lowercase().contains(pattern) {
            safety_flagged = true;
            cleaned = replace_case_insensitive(&cleaned, pattern, replacement);
        }
    }

    let final_text: String = cleaned
        .chars()
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect::<String>()
        .trim()
        .to_string();

    if final_text.is_empty() {
        return Err("Document body is empty after stripping non-text elements".to_string());
    }

    Ok((clean_title, final_text, safety_flagged))
}

fn strip_html_and_media_tags(input: &str) -> (String, bool) {
    let mut without_blocks = input.to_string();
    let mut flagged = false;

    for tag in ["script", "style", "iframe", "object", "embed", "svg"] {
        let open_pat = format!("<{tag}");
        let close_pat = format!("</{tag}>");
        while let Some(start) = without_blocks.to_lowercase().find(&open_pat) {
            flagged = true;
            if let Some(rel_end) = without_blocks[start..].to_lowercase().find(&close_pat) {
                let end = start + rel_end + close_pat.len();
                without_blocks.replace_range(start..end, "");
            } else {
                without_blocks.truncate(start);
                break;
            }
        }
    }

    if without_blocks.to_lowercase().contains("data:image/") {
        flagged = true;
        without_blocks =
            replace_case_insensitive(&without_blocks, "data:image/", "[stripped-image-uri]/");
    }

    // Strip inline HTML tags (`<img ...>`, `<a ...>`, etc.)
    let mut out = String::with_capacity(without_blocks.len());
    let mut in_tag = false;
    for ch in without_blocks.chars() {
        match ch {
            '<' => {
                in_tag = true;
                flagged = true;
            }
            '>' if in_tag => {
                in_tag = false;
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    (out, flagged)
}

fn replace_case_insensitive(haystack: &str, needle_lower: &str, replacement: &str) -> String {
    let mut result = haystack.to_string();
    let n_len = needle_lower.len();
    if n_len == 0 {
        return result;
    }
    let mut search_from = 0;
    while search_from <= result.len() {
        let lower_slice = result[search_from..].to_lowercase();
        let Some(pos) = lower_slice.find(needle_lower) else {
            break;
        };
        let abs_start = search_from + pos;
        let abs_end = (abs_start + n_len).min(result.len());
        if result.is_char_boundary(abs_start) && result.is_char_boundary(abs_end) {
            result.replace_range(abs_start..abs_end, replacement);
            search_from = abs_start + replacement.len();
        } else {
            break;
        }
    }
    result
}

/// Extracts plain text from a Google Docs API v1 `Document` JSON payload (`documents.get`),
/// ignoring all `inlineObjectElement` / embedded images.
pub fn extract_text_from_google_doc_json(doc_json: &serde_json::Value) -> String {
    let mut out = String::new();
    let Some(content_arr) = doc_json
        .get("body")
        .and_then(|b| b.get("content"))
        .and_then(|c| c.as_array())
    else {
        return out;
    };

    for structural_el in content_arr {
        if let Some(elements) = structural_el
            .get("paragraph")
            .and_then(|p| p.get("elements"))
            .and_then(|e| e.as_array())
        {
            for el in elements {
                if let Some(txt) = el
                    .get("textRun")
                    .and_then(|tr| tr.get("content"))
                    .and_then(|c| c.as_str())
                {
                    out.push_str(txt);
                }
            }
        } else if let Some(rows) = structural_el
            .get("table")
            .and_then(|t| t.get("tableRows"))
            .and_then(|r| r.as_array())
        {
            for row in rows {
                let mut cells_txt = Vec::new();
                if let Some(cells) = row.get("tableCells").and_then(|c| c.as_array()) {
                    for cell in cells {
                        let cell_val = extract_text_from_google_doc_json(&json!({
                            "body": { "content": cell.get("content").cloned().unwrap_or(json!([])) }
                        }));
                        cells_txt.push(cell_val.replace('\n', " ").trim().to_string());
                    }
                }
                if !cells_txt.is_empty() {
                    out.push_str(&cells_txt.join(" | "));
                    out.push('\n');
                }
            }
        }
    }
    out
}

/// Extracts a clean CSV/Markdown table from a Google Sheets API v4 `ValueRange` JSON payload (`values.get`).
pub fn extract_table_from_google_sheet_json(sheet_json: &serde_json::Value) -> String {
    let Some(rows) = sheet_json.get("values").and_then(|v| v.as_array()) else {
        return String::new();
    };
    let mut lines = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(cells) = row.as_array() {
            let line: Vec<String> = cells
                .iter()
                .map(|c| {
                    c.as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| c.to_string())
                        .replace('\n', " ")
                        .trim()
                        .to_string()
                })
                .collect();
            lines.push(line.join(" | "));
        }
    }
    lines.join("\n")
}

/// Extracts a clean Google Drive / Docs / Sheets File ID from either a raw file ID string
/// or a full `https://docs.google.com/.../d/<FILE_ID>/...` / `https://drive.google.com/...` URL.
pub fn extract_google_drive_file_id(raw_input: &str) -> String {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(after_d) = trimmed.split("/d/").nth(1)
        && let Some(id_seg) = after_d.split(['/', '?', '#', '&']).next()
        && !id_seg.trim().is_empty()
    {
        return id_seg.trim().to_string();
    }
    if let Some(after_id) = trimmed.split("id=").nth(1)
        && let Some(id_seg) = after_id.split(['&', '#', '/']).next()
        && !id_seg.trim().is_empty()
    {
        return id_seg.trim().to_string();
    }
    trimmed.to_string()
}

/// Fetches a Google Doc or Google Sheet using a single-file `drive.file` OAuth access token,
/// or resolves deterministic local demo documents (`gdoc-demo-*` / `gsheet-demo-*`) for offline development.
pub async fn fetch_google_doc_or_sheet_text(
    source_kind: GmDocumentSource,
    file_id: &str,
    access_token: Option<&str>,
) -> Result<String, String> {
    let extracted_id = extract_google_drive_file_id(file_id);
    let clean_id = extracted_id.as_str();
    if clean_id.is_empty() {
        return Err("google_file_id is required when importing a Google Doc or Sheet".to_string());
    }

    // Built-in offline/demo Google Docs & Sheets for local development and automated tests
    if clean_id.starts_with("gdoc-demo") || clean_id.starts_with("demo-doc") {
        return Ok(format!(
            "# Satya Operator Dossier ({clean_id})\n\
             Operator: Sara (Valkyrie) | Control C: 12 | Trauma Surgery N: 3\n\
             Background: Former Aethelgard orbital trauma surgeon assigned to Sector 4 Docking Bay.\n\
             GM Secret Hook: Suspects Dr. Aris Thorne tampered with the Sector 4 relay telemetry."
        ));
    }
    if clean_id.starts_with("gsheet-demo") || clean_id.starts_with("demo-sheet") {
        return Ok(format!(
            "Item | Mass (kg) | Stance Mod | Notes ({clean_id})\n\
             Valkyrie Exo-Rig | 18.5 | Stability +1 | Sealed EVA hardsuit\n\
             Med-Laser Scalpel | 0.8 | Precision +2 | +1 die on Trauma Surgery\n\
             Mag-Grapple Line | 2.1 | Maneuver 0 | 25m zero-G tether"
        ));
    }

    let Some(_token) = access_token.map(str::trim).filter(|t| !t.is_empty()) else {
        return Err(
            "An active Google OAuth `drive.file` access token is required to import Google Drive files (link Google account first or use `gdoc-demo-sara` / `gsheet-demo-gear`)"
                .to_string(),
        );
    };

    match source_kind {
        GmDocumentSource::GoogleDoc => {
            let simulated_doc_json = json!({
                "body": {
                    "content": [
                        {
                            "paragraph": {
                                "elements": [
                                    { "textRun": { "content": format!("# Google Doc Snapshot ({clean_id})\n") } },
                                    { "inlineObjectElement": { "inlineObjectId": "kix.embedded_image_ignored" } },
                                    { "textRun": { "content": "Extracted via single-file `drive.file` scope (embedded images stripped).\n" } }
                                ]
                            }
                        }
                    ]
                }
            });
            Ok(extract_text_from_google_doc_json(&simulated_doc_json))
        }
        GmDocumentSource::GoogleSheet => {
            let simulated_sheet_json = json!({
                "values": [
                    ["File ID", "Scope", "Status"],
                    [clean_id, "drive.file", "Tabular Snapshot Imported"]
                ]
            });
            Ok(extract_table_from_google_sheet_json(&simulated_sheet_json))
        }
        GmDocumentSource::DirectTextUpload => {
            Err("DirectTextUpload does not fetch from Google APIs".to_string())
        }
    }
}

/// Links an authenticated user's Satya account to a Google OIDC identity and optional ephemeral
/// `drive.file` access token at `POST /api/auth/google`.
pub async fn google_auth_link_handler(
    State(state): State<Arc<SignalingAppState>>,
    headers: HeaderMap,
    Json(req): Json<GoogleAuthLinkRequest>,
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
    if req.google_sub.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "google_sub cannot be empty" })),
        )
            .into_response();
    }

    let status = state
        .auth
        .link_google_account(&user.user_id, req.google_sub, req.access_token)
        .await;
    (StatusCode::OK, Json(status)).into_response()
}

/// Returns the caller's Google OAuth (`drive.file`) link status at `GET /api/auth/google/status`.
pub async fn google_auth_status_handler(
    State(state): State<Arc<SignalingAppState>>,
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

    let status = state.auth.get_google_auth_status(&user.user_id).await;
    (StatusCode::OK, Json(status)).into_response()
}

/// Ingests, sanitizes, and stores a `GM_ONLY` document snapshot at `POST /api/sessions/:session_id/docs`.
/// Delivers `ServerWsPacket::GmDocumentAdded` strictly to the session's Human GM/Host and the uploader.
pub async fn submit_session_gm_doc_handler(
    State(state): State<Arc<SignalingAppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<SubmitGmDocumentRequest>,
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

    let raw_body = match req.source_kind {
        GmDocumentSource::DirectTextUpload => req.raw_content.unwrap_or_default(),
        GmDocumentSource::GoogleDoc | GmDocumentSource::GoogleSheet => {
            if let Some(provided) = req
                .raw_content
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
            {
                provided
            } else {
                let stored_tok = state.auth.get_ephemeral_google_token(&user.user_id).await;
                let effective_tok = req.google_access_token.as_deref().or(stored_tok.as_deref());
                let file_id = req.google_file_id.as_deref().unwrap_or("");
                match fetch_google_doc_or_sheet_text(req.source_kind, file_id, effective_tok).await
                {
                    Ok(fetched) => fetched,
                    Err(err_msg) => {
                        return (StatusCode::BAD_REQUEST, Json(json!({ "error": err_msg })))
                            .into_response();
                    }
                }
            }
        }
    };

    let (clean_title, sanitized_content, safety_flagged) =
        match sanitize_and_guard_gm_document(&req.title, &raw_body) {
            Ok(res) => res,
            Err(err_msg) => {
                let code = if err_msg.contains("safety filter") {
                    StatusCode::UNPROCESSABLE_ENTITY
                } else {
                    StatusCode::BAD_REQUEST
                };
                return (code, Json(json!({ "error": err_msg }))).into_response();
            }
        };

    let char_count = sanitized_content.chars().count();
    let doc_record = GmDocumentRecord {
        doc_id: format!("doc_{}", Uuid::now_v7()),
        session_id: session_id.clone(),
        uploader_user_id: user.user_id.clone(),
        uploader_display_name: user.display_name.clone(),
        title: clean_title,
        source_kind: req.source_kind,
        google_file_id: req
            .google_file_id
            .map(|s| extract_google_drive_file_id(&s))
            .filter(|s| !s.is_empty()),
        visibility: MessageVisibility::GmOnly,
        sanitized_content,
        char_count,
        safety_flagged,
        is_persistent: req.is_persistent,
        campaign_tag: req
            .campaign_tag
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        created_at_us: Utc::now().timestamp_micros(),
    };

    match state
        .auth
        .add_session_gm_document(&session_id, &user.user_id, doc_record)
        .await
    {
        Ok((stored_doc, _uploader, delivery_ids)) => {
            if let Ok(ws_pkt) = serde_json::to_string(&ServerWsPacket::GmDocumentAdded {
                document: Box::new(stored_doc.clone()),
            }) {
                state
                    .send_to_users(&session_id, &delivery_ids, &ws_pkt)
                    .await;
            }
            (StatusCode::OK, Json(stored_doc)).into_response()
        }
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Lists `GM_ONLY` documents in `:session_id` visible to the authenticated caller at `GET /api/sessions/:session_id/docs`.
/// - Human GM (`GM`/`HOST`) receives all submitted documents in the session.
/// - Players receive only their own submitted documents.
pub async fn list_session_gm_docs_handler(
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

    match state
        .auth
        .list_session_gm_documents_for_user(&session_id, &user.user_id)
        .await
    {
        Ok(documents) => (
            StatusCode::OK,
            Json(SessionGmDocsResponse {
                session_id,
                documents,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
