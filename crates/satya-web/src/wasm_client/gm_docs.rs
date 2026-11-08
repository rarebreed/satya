//! `GM_ONLY` Document Vault (`#notebook-pane-docs`) and Google Drive (`drive.file`) OAuth/Picker integration.

use crate::wasm_client::dom::{
    escape_html, fetch_json, get_element, input_checked, input_value, select_value, set_class,
    set_display, set_feedback, set_input_value, set_select_value, set_text, window,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use js_sys::{Function, Promise, Reflect};
use satya_schema::{
    GmDocumentRecord, GmDocumentSource, GmLookupHit, GmLookupSourceKind, GmMemoryCategory,
    GmMemoryNoteRecord, GmOmnisearchResponse, GoogleAuthLinkRequest, GoogleAuthStatusResponse,
    SessionGmDocsResponse, SessionGmMemoriesResponse, SubmitGmDocumentRequest,
    ToggleGmDocumentPersistenceRequest, UpsertGmMemoryRequest,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Headers, HtmlElement, Request, RequestInit, Response};

/// Parses a `GM_ONLY` document source select string into [`GmDocumentSource`].
pub fn parse_gm_doc_source(raw: &str) -> GmDocumentSource {
    match raw.trim().to_uppercase().as_str() {
        "GOOGLE_DOC" => GmDocumentSource::GoogleDoc,
        "GOOGLE_SHEET" => GmDocumentSource::GoogleSheet,
        _ => GmDocumentSource::DirectTextUpload,
    }
}

/// Formats the HTML list of sanitized `GM_ONLY` documents for `#gm-docs-list`.
pub fn format_gm_documents_list_html(docs: &[GmDocumentRecord]) -> String {
    if docs.is_empty() {
        return r#"<span class="pill pill-muted">No GM_ONLY documents submitted in this session yet.</span>"#.to_string();
    }
    docs.iter()
        .rev()
        .map(|d| {
            let src_label = match d.source_kind {
                GmDocumentSource::GoogleDoc => "Google Doc (drive.file)",
                GmDocumentSource::GoogleSheet => "Google Sheet (drive.file)",
                GmDocumentSource::DirectTextUpload => "Text/Markdown",
            };
            let flag_badge = if d.safety_flagged {
                r#" <span class="pill pill-muted" title="HTML/script or prompt-injection tokens were scrubbed">Sanitized</span>"#
            } else {
                ""
            };
            let tag_label = d
                .campaign_tag
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(&d.session_id);
            let persist_badge = if d.is_persistent {
                format!(
                    r#" <span class="pill pill-active" title="Saved in SQLite GM Vault across restarts and campaign sessions">Persistent ({})</span>"#,
                    escape_html(tag_label)
                )
            } else {
                r#" <span class="pill pill-muted" title="Session-only unless marked persistent">Session-Only</span>"#.to_string()
            };
            let next_persist = !d.is_persistent;
            let toggle_btn_label = if d.is_persistent {
                "Unmark Persistent"
            } else {
                "Mark Persistent"
            };
            format!(
                r#"<div class="gm-doc-card">
                    <div class="gm-doc-header">
                        <strong>{}</strong>
                        <div class="btn-row-compact">
                            <span class="pill pill-iggy">{src_label} · {} chars · GM_ONLY</span>{persist_badge}{flag_badge}
                            <button type="button" class="btn-chip" onclick="window.satyaToggleGmDocPersistence('{}', {next_persist})">{toggle_btn_label}</button>
                        </div>
                    </div>
                    <div class="subtitle">Submitted by <strong>{}</strong></div>
                    <pre class="gm-doc-body">{}</pre>
                </div>"#,
                escape_html(&d.title),
                d.char_count,
                escape_html(&d.doc_id),
                escape_html(&d.uploader_display_name),
                escape_html(&d.sanitized_content)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Appends or updates an incoming [`GmDocumentRecord`] in client state and re-renders `#gm-docs-list`.
pub fn render_gm_document_added(doc: &GmDocumentRecord) {
    let docs = with_state_mut(|st| {
        if let Some(existing) = st.gm_documents.iter_mut().find(|d| d.doc_id == doc.doc_id) {
            *existing = doc.clone();
        } else {
            st.gm_documents.push(doc.clone());
        }
        st.gm_documents.clone()
    });
    if let Some(list_el) = get_element("gm-docs-list") {
        list_el.set_inner_html(&format_gm_documents_list_html(&docs));
    }
    set_text("gm-docs-count-pill", &format!("{} GM Docs", docs.len()));
}

/// Extracts a clean Google Drive File ID (and optional inferred `GOOGLE_DOC` / `GOOGLE_SHEET` kind)
/// from either a raw File ID or a full `https://docs.google.com/.../d/<FILE_ID>/...` URL.
pub fn extract_google_drive_file_id_and_kind(raw_input: &str) -> (String, Option<&'static str>) {
    let trimmed = raw_input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    let inferred_kind = if trimmed.contains("docs.google.com/spreadsheets") {
        Some("GOOGLE_SHEET")
    } else if trimmed.contains("docs.google.com/document") {
        Some("GOOGLE_DOC")
    } else {
        None
    };

    if let Some(after_d) = trimmed.split("/d/").nth(1)
        && let Some(id_seg) = after_d.split(['/', '?', '#', '&']).next()
        && !id_seg.trim().is_empty()
    {
        return (id_seg.trim().to_string(), inferred_kind);
    }
    if let Some(after_id) = trimmed.split("id=").nth(1)
        && let Some(id_seg) = after_id.split(['&', '#', '/']).next()
        && !id_seg.trim().is_empty()
    {
        return (id_seg.trim().to_string(), inferred_kind);
    }
    (trimmed.to_string(), inferred_kind)
}

/// Synchronizes server-configured Google OAuth (`drive.file`) settings (`google_oauth_client_id`
/// and `google_picker_api_key`) from `GET /api/auth/google/status` after user authentication.
pub fn sync_google_auth_status_from_server() {
    spawn_local(async move {
        let Some(tok) = with_state(|st| st.auth_token.clone()) else {
            return;
        };
        if let Ok(status) = fetch_json::<(), GoogleAuthStatusResponse>(
            "GET",
            "/api/auth/google/status",
            Some(&tok),
            None,
        )
        .await
        {
            with_state_mut(|st| {
                st.google_linked = status.google_linked;
                if status.google_oauth_client_id.is_some() {
                    st.google_oauth_client_id = status.google_oauth_client_id.clone();
                }
                if status.google_picker_api_key.is_some() {
                    st.google_picker_api_key = status.google_picker_api_key.clone();
                }
            });
            if let Some(cid) = status.google_oauth_client_id.as_deref()
                && input_value("gm-google-client-id").trim().is_empty()
            {
                set_input_value("gm-google-client-id", cid);
            }
            if let Some(akey) = status.google_picker_api_key.as_deref()
                && input_value("gm-google-api-key").trim().is_empty()
            {
                set_input_value("gm-google-api-key", akey);
            }
            if status.google_linked {
                set_text(
                    "google-auth-status-pill",
                    "Google Linked (drive.file Scope)",
                );
                set_class("google-auth-status-pill", "pill pill-active");
            }
        }
    });
}

/// Links the authenticated user's Satya account to Google OAuth (`drive.file` single-file scope)
/// via `POST /api/auth/google`. If a Google Cloud OAuth Client ID is configured, launches Google Identity
/// Services (`google.accounts.oauth2.initTokenClient`) first; otherwise links in local development mode.
#[wasm_bindgen]
pub fn satya_link_google_account() {
    let client_id = resolved_google_client_id();
    let api_key = resolved_google_api_key();
    if !client_id.is_empty() {
        launch_browser_google_picker(&client_id, &api_key, false);
        return;
    }

    spawn_local(async move {
        let (auth_token, user) = with_state(|st| (st.auth_token.clone(), st.user.clone()));
        let (Some(tok), Some(u)) = (auth_token, user) else {
            set_feedback(
                "auth-feedback",
                "Please log in to your Satya account first before linking Google (`drive.file`).",
                true,
            );
            return;
        };

        let req = GoogleAuthLinkRequest {
            google_sub: format!("google-oidc-{}", u.username),
            google_display_hint: Some(u.display_name.clone()),
            access_token: Some(format!("ya29.drive-file-ephemeral-{}", u.user_id)),
        };

        match fetch_json::<_, GoogleAuthStatusResponse>(
            "POST",
            "/api/auth/google",
            Some(&tok),
            Some(&req),
        )
        .await
        {
            Ok(status) => {
                with_state_mut(|st| {
                    st.google_linked = status.google_linked;
                });
                set_text(
                    "google-auth-status-pill",
                    "Google Linked (drive.file Scope)",
                );
                set_class("google-auth-status-pill", "pill pill-active");
                set_feedback(
                    "auth-feedback",
                    "Linked Google identity (`drive.file` single-file scope). Personal email hidden from other players.",
                    false,
                );
            }
            Err(err) => set_feedback("auth-feedback", &err, true),
        }
    });
}

fn resolved_google_client_id() -> String {
    let ui_val = input_value("gm-google-client-id").trim().to_string();
    if !ui_val.is_empty() {
        return ui_val;
    }
    with_state(|st| st.google_oauth_client_id.clone().unwrap_or_default())
}

fn resolved_google_api_key() -> String {
    let ui_val = input_value("gm-google-api-key").trim().to_string();
    if !ui_val.is_empty() {
        return ui_val;
    }
    with_state(|st| st.google_picker_api_key.clone().unwrap_or_default())
}

/// Receives a live Google OAuth 2.0 `drive.file` access token from Google Identity Services in the browser
/// and links it to the caller's Satya account via `POST /api/auth/google`.
#[wasm_bindgen]
pub fn satya_receive_google_token(access_token: String) {
    let clean_tok = access_token.trim().to_string();
    if clean_tok.is_empty() {
        return;
    }
    with_state_mut(|st| {
        st.google_access_token = Some(clean_tok.clone());
        st.google_linked = true;
    });
    set_text(
        "google-auth-status-pill",
        "Google Linked (drive.file Scope)",
    );
    set_class("google-auth-status-pill", "pill pill-active");

    spawn_local(async move {
        let (auth_token, user) = with_state(|st| (st.auth_token.clone(), st.user.clone()));
        let (Some(tok), Some(u)) = (auth_token, user) else {
            return;
        };
        let req = GoogleAuthLinkRequest {
            google_sub: format!("google-oidc-{}", u.username),
            google_display_hint: Some(u.display_name),
            access_token: Some(clean_tok),
        };
        let _ = fetch_json::<_, GoogleAuthStatusResponse>(
            "POST",
            "/api/auth/google",
            Some(&tok),
            Some(&req),
        )
        .await;
    });
}

/// Receives a Google Doc or Google Sheet selected via the browser's `google.picker.PickerBuilder`
/// (`drive.file` single-file scope) along with its exported plain-text or CSV snapshot.
#[wasm_bindgen]
pub fn satya_receive_picked_google_doc(
    file_id: String,
    title: String,
    source_kind: String,
    content: String,
) {
    if !title.trim().is_empty() {
        set_input_value("gm-doc-title", title.trim());
    }
    let normalized_src = if source_kind == "GOOGLE_SHEET" {
        "GOOGLE_SHEET"
    } else {
        "GOOGLE_DOC"
    };
    set_select_value("gm-doc-source", normalized_src);
    set_input_value("gm-doc-google-id", file_id.trim());
    set_input_value("gm-doc-content", &content);
    satya_gm_doc_source_changed();
    set_feedback(
        "gm-doc-feedback",
        &format!(
            "Selected '{}' ({} chars) via Google Drive Picker (`drive.file`). Click 'Submit to GM Only' to share with the GM.",
            title.trim(),
            content.chars().count()
        ),
        false,
    );
}

/// Reads a local `.txt`, `.md`, or `.csv` file selected via `#gm-doc-file-input` (`File.text()`)
/// and populates `#gm-doc-title`, `#gm-doc-source`, and `#gm-doc-content`.
#[wasm_bindgen]
pub fn satya_gm_doc_file_selected() {
    spawn_local(async move {
        let Some(el) = get_element("gm-doc-file-input") else {
            return;
        };
        let Ok(files) = Reflect::get(&el, &JsValue::from_str("files")) else {
            return;
        };
        let Ok(file) = Reflect::get(&files, &JsValue::from_f64(0.0)) else {
            return;
        };
        if file.is_undefined() || file.is_null() {
            return;
        }
        let file_name = Reflect::get(&file, &JsValue::from_str("name"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "Uploaded Document".to_string());

        let Ok(text_fn_val) = Reflect::get(&file, &JsValue::from_str("text")) else {
            return;
        };
        let Ok(text_fn) = text_fn_val.dyn_into::<Function>() else {
            return;
        };
        let Ok(promise_val) = text_fn.call0(&file) else {
            return;
        };
        let Ok(promise) = promise_val.dyn_into::<Promise>() else {
            return;
        };
        let Ok(text_val) = JsFuture::from(promise).await else {
            set_feedback(
                "gm-doc-feedback",
                "Failed to read local file contents.",
                true,
            );
            return;
        };
        let mut content = text_val.as_string().unwrap_or_default();
        let mut truncated_note = "";
        if content.len() > 32_768 {
            content.truncate(32_768);
            truncated_note = " (truncated to 32 KB limit)";
        }

        let clean_title = file_name
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(&file_name)
            .replace(['_', '-'], " ");
        if !clean_title.trim().is_empty() {
            set_input_value("gm-doc-title", clean_title.trim());
        }
        set_select_value("gm-doc-source", "DIRECT_TEXT_UPLOAD");
        satya_gm_doc_source_changed();
        set_input_value("gm-doc-content", &content);
        set_feedback(
            "gm-doc-feedback",
            &format!(
                "Loaded '{}' ({} chars){truncated_note}. Click 'Submit to GM Only' to share with the GM.",
                file_name,
                content.chars().count()
            ),
            false,
        );
    });
}

/// Automatically extracts the Google Drive File ID (and switches `#gm-doc-source` between
/// `GOOGLE_DOC` and `GOOGLE_SHEET`) when the user pastes a full `https://docs.google.com/...` URL
/// into `#gm-doc-google-id`.
#[wasm_bindgen]
pub fn satya_gm_doc_url_changed() {
    let raw = input_value("gm-doc-google-id");
    let (clean_id, inferred_kind) = extract_google_drive_file_id_and_kind(&raw);
    if !clean_id.is_empty() && clean_id != raw {
        set_input_value("gm-doc-google-id", &clean_id);
        if let Some(kind) = inferred_kind {
            set_select_value("gm-doc-source", kind);
            satya_gm_doc_source_changed();
        }
        set_feedback(
            "gm-doc-feedback",
            &format!("Extracted Google Drive File ID: {clean_id}"),
            false,
        );
    }
}

/// Toggles visibility of `#gm-google-oauth-config-row` so the user can inspect or enter a
/// Google Cloud OAuth 2.0 Web Client ID and Google Picker API Key.
#[wasm_bindgen]
pub fn satya_toggle_google_oauth_config() {
    let Some(el) =
        get_element("gm-google-oauth-config-row").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let cur = el.style().get_property_value("display").unwrap_or_default();
    let next = if cur == "none" || cur.is_empty() {
        "grid"
    } else {
        "none"
    };
    let _ = el.style().set_property("display", next);
}

/// Opens Google's official `google.picker.PickerBuilder` modal (`drive.file` single-file scope)
/// so the user can click a Google Doc or Google Sheet in their Google Drive, grant single-file
/// read permission, and export its plain-text/CSV content into `#gm-doc-content`.
#[wasm_bindgen]
pub fn satya_open_google_drive_picker() {
    let client_id = resolved_google_client_id();
    let api_key = resolved_google_api_key();
    if client_id.is_empty() {
        set_display("gm-google-oauth-config-row", "grid");
        set_feedback(
            "gm-doc-feedback",
            "To open the live Google Drive Picker (`drive.file` single-file scope), enter your Google Cloud Web OAuth Client ID & Picker API Key above (or in `config/storage.toml`)—or download your Google Doc as .txt/.md (or Sheet as .csv) and use 'Choose Local File' above.",
            false,
        );
        return;
    }
    launch_browser_google_picker(&client_id, &api_key, true);
}

fn launch_browser_google_picker(client_id: &str, api_key: &str, open_picker: bool) {
    let js_code = r#"
        const loadScript = (src) => new Promise((resolve, reject) => {
            if (document.querySelector(`script[src="${src}"]`)) return resolve();
            const s = document.createElement('script');
            s.src = src;
            s.async = true;
            s.onload = () => resolve();
            s.onerror = () => reject(new Error('Failed to load ' + src));
            document.head.appendChild(s);
        });
        (async () => {
            try {
                await loadScript('https://accounts.google.com/gsi/client');
                if (openPicker) {
                    await loadScript('https://apis.google.com/js/api.js');
                }
                const runPicker = (token) => {
                    if (window.satyaReceiveGoogleToken) {
                        window.satyaReceiveGoogleToken(token);
                    }
                    if (!openPicker) return;
                    if (!window.gapi) return;
                    window.gapi.load('picker', () => {
                        const docsView = new window.google.picker.DocsView(window.google.picker.ViewId.DOCUMENTS);
                        const sheetsView = new window.google.picker.DocsView(window.google.picker.ViewId.SPREADSHEETS);
                        const builder = new window.google.picker.PickerBuilder()
                            .addView(docsView)
                            .addView(sheetsView)
                            .setOAuthToken(token)
                            .setCallback(async (data) => {
                                if (data.action === window.google.picker.Action.PICKED && data.docs && data.docs.length > 0) {
                                    const doc = data.docs[0];
                                    const isSheet = (doc.mimeType || '').includes('spreadsheet');
                                    const exportMime = isSheet ? 'text/csv' : 'text/plain';
                                    const srcKind = isSheet ? 'GOOGLE_SHEET' : 'GOOGLE_DOC';
                                    let exported = '';
                                    try {
                                        const resp = await fetch(
                                            `https://www.googleapis.com/drive/v3/files/${encodeURIComponent(doc.id)}/export?mimeType=${encodeURIComponent(exportMime)}`,
                                            { headers: { Authorization: `Bearer ${token}` } }
                                        );
                                        if (resp.ok) {
                                            exported = await resp.text();
                                        }
                                    } catch (_) {}
                                    if (window.satyaReceivePickedGoogleDoc) {
                                        window.satyaReceivePickedGoogleDoc(doc.id || '', doc.name || 'Google Drive Document', srcKind, exported);
                                    }
                                }
                            });
                        if (apiKey) {
                            builder.setDeveloperKey(apiKey);
                        }
                        builder.build().setVisible(true);
                    });
                };
                if (window._satyaGoogleAccessToken && openPicker) {
                    runPicker(window._satyaGoogleAccessToken);
                    return;
                }
                const tokenClient = window.google.accounts.oauth2.initTokenClient({
                    client_id: clientId,
                    scope: 'https://www.googleapis.com/auth/drive.file',
                    callback: (resp) => {
                        if (resp && resp.access_token) {
                            window._satyaGoogleAccessToken = resp.access_token;
                            runPicker(resp.access_token);
                        }
                    }
                });
                tokenClient.requestAccessToken({ prompt: '' });
            } catch (err) {
                console.error('Google Picker launch error:', err);
            }
        })();
    "#;
    let func = Function::new_with_args("clientId, apiKey, openPicker", js_code);
    let _ = func.call3(
        &JsValue::NULL,
        &JsValue::from_str(client_id),
        &JsValue::from_str(api_key),
        &JsValue::from_bool(open_picker),
    );
}

async fn try_export_google_drive_file_in_browser(
    source_kind: GmDocumentSource,
    file_id: &str,
    access_token: &str,
) -> Option<String> {
    let win = window()?;
    let export_mime = match source_kind {
        GmDocumentSource::GoogleSheet => "text/csv",
        _ => "text/plain",
    };
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}/export?mimeType={}",
        file_id.trim(),
        export_mime
    );
    let opts = RequestInit::new();
    opts.set_method("GET");
    let headers = Headers::new().ok()?;
    let _ = headers.set("Authorization", &format!("Bearer {access_token}"));
    opts.set_headers(&headers);
    let req = Request::new_with_str_and_init(&url, &opts).ok()?;
    let resp_val = JsFuture::from(win.fetch_with_request(&req)).await.ok()?;
    let resp: Response = resp_val.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let text_promise = resp.text().ok()?;
    let text_val = JsFuture::from(text_promise).await.ok()?;
    text_val.as_string().filter(|s| !s.trim().is_empty())
}

/// Toggles the Google File ID picker input when `#gm-doc-source` changes between
/// `DIRECT_TEXT_UPLOAD`, `GOOGLE_DOC`, and `GOOGLE_SHEET`.
#[wasm_bindgen]
pub fn satya_gm_doc_source_changed() {
    let src = parse_gm_doc_source(&select_value("gm-doc-source"));
    let is_google = matches!(
        src,
        GmDocumentSource::GoogleDoc | GmDocumentSource::GoogleSheet
    );
    set_display("gm-doc-google-row", if is_google { "grid" } else { "none" });
    if !is_google {
        set_display("gm-google-oauth-config-row", "none");
    }
}

/// Loads a built-in Google Doc (`gdoc-demo-sara`) or Google Sheet (`gsheet-demo-gear`) Picker preset
/// into the `GM Docs` form for single-click testing.
#[wasm_bindgen]
pub fn satya_load_gm_doc_preset(preset_kind: String) {
    if preset_kind == "sheet" {
        set_select_value("gm-doc-source", "GOOGLE_SHEET");
        set_input_value("gm-doc-title", "Valkyrie Squad Loadout Sheet");
        set_input_value("gm-doc-google-id", "gsheet-demo-gear");
        set_input_value("gm-doc-content", "");
    } else {
        set_select_value("gm-doc-source", "GOOGLE_DOC");
        set_input_value("gm-doc-title", "Sara Valkyrie Medical & Sector 4 Dossier");
        set_input_value("gm-doc-google-id", "gdoc-demo-sara");
        set_input_value("gm-doc-content", "");
    }
    satya_gm_doc_source_changed();
}

/// Submits a `GM_ONLY` document (direct text/Markdown/CSV or Google Doc/Sheet snapshot)
/// to `POST /api/sessions/:session_id/docs`.
#[wasm_bindgen]
pub fn satya_submit_gm_document() {
    spawn_local(async move {
        let (session_id, auth_token, google_tok) = with_state(|st| {
            (
                st.session_id.clone(),
                st.auth_token.clone(),
                st.google_access_token.clone(),
            )
        });
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            set_feedback(
                "gm-doc-feedback",
                "Join or create an active session first before submitting a GM_ONLY document.",
                true,
            );
            return;
        };

        let title = input_value("gm-doc-title").trim().to_string();
        let source_kind = parse_gm_doc_source(&select_value("gm-doc-source"));
        let mut raw_text = input_value("gm-doc-content");
        let (google_id, _) =
            extract_google_drive_file_id_and_kind(&input_value("gm-doc-google-id"));
        if !google_id.is_empty() {
            set_input_value("gm-doc-google-id", &google_id);
        }

        // If importing a non-demo Google Doc/Sheet with an active browser `drive.file` token and empty textarea,
        // export the plain-text/CSV content directly from the Google Drive API in the browser first.
        if matches!(
            source_kind,
            GmDocumentSource::GoogleDoc | GmDocumentSource::GoogleSheet
        ) && raw_text.trim().is_empty()
            && !google_id.is_empty()
            && !google_id.starts_with("gdoc-demo")
            && !google_id.starts_with("gsheet-demo")
            && let Some(ref gtok) = google_tok
            && !gtok.starts_with("ya29.drive-file-ephemeral-")
            && let Some(exported) =
                try_export_google_drive_file_in_browser(source_kind, &google_id, gtok).await
        {
            raw_text = exported;
        }

        let is_persistent = input_checked("gm-doc-persistent");
        let campaign_tag_raw = input_value("gm-doc-campaign-tag").trim().to_string();

        let req = SubmitGmDocumentRequest {
            title,
            source_kind,
            raw_content: if raw_text.trim().is_empty() {
                None
            } else {
                Some(raw_text)
            },
            google_file_id: if google_id.is_empty() {
                None
            } else {
                Some(google_id)
            },
            google_access_token: google_tok,
            is_persistent,
            campaign_tag: if campaign_tag_raw.is_empty() {
                None
            } else {
                Some(campaign_tag_raw)
            },
        };

        let url = format!("/api/sessions/{sid}/docs");
        match fetch_json::<_, GmDocumentRecord>("POST", &url, Some(&tok), Some(&req)).await {
            Ok(doc) => {
                render_gm_document_added(&doc);
                set_input_value("gm-doc-content", "");
                let persist_note = if doc.is_persistent {
                    format!(
                        " [Persisted under campaign/session '{}']",
                        doc.campaign_tag.as_deref().unwrap_or(&sid)
                    )
                } else {
                    String::new()
                };
                set_feedback(
                    "gm-doc-feedback",
                    &format!(
                        "Submitted '{}' ({} chars) strictly to GM & AI GM (`GM_ONLY`){persist_note}.",
                        doc.title, doc.char_count
                    ),
                    false,
                );
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Toggles `is_persistent` on an existing `GM_ONLY` document via `POST /api/sessions/:session_id/docs/:doc_id/persist`.
#[wasm_bindgen]
pub fn satya_toggle_gm_doc_persistence(doc_id: String, is_persistent: bool) {
    spawn_local(async move {
        let (session_id, auth_token) =
            with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            return;
        };
        let ctag = input_value("gm-doc-campaign-tag").trim().to_string();
        let req = ToggleGmDocumentPersistenceRequest {
            is_persistent,
            campaign_tag: if ctag.is_empty() { None } else { Some(ctag) },
        };
        let url = format!("/api/sessions/{sid}/docs/{doc_id}/persist");
        match fetch_json::<_, GmDocumentRecord>("POST", &url, Some(&tok), Some(&req)).await {
            Ok(doc) => {
                render_gm_document_added(&doc);
                set_feedback(
                    "gm-doc-feedback",
                    &format!(
                        "Updated '{}': {}",
                        doc.title,
                        if doc.is_persistent {
                            "Marked Persistent in GM Vault"
                        } else {
                            "Set to Session-Only"
                        }
                    ),
                    false,
                );
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Formats the HTML list of persistent and session GM memory notes for `#gm-memories-list`.
pub fn format_gm_memories_list_html(memories: &[GmMemoryNoteRecord]) -> String {
    if memories.is_empty() {
        return r#"<span class="pill pill-muted">No persistent GM memory notes saved for this campaign/session yet.</span>"#.to_string();
    }
    memories
        .iter()
        .map(|m| {
            let pin_badge = if m.is_pinned {
                r#" <span class="pill pill-active" title="Pinned into AI GM active context header">Pinned to AI GM</span>"#
            } else {
                ""
            };
            let persist_badge = if m.is_persistent {
                format!(
                    r#" <span class="pill pill-iggy">Persistent ({})</span>"#,
                    escape_html(&m.campaign_tag)
                )
            } else {
                r#" <span class="pill pill-muted">Session-Only</span>"#.to_string()
            };
            format!(
                r#"<div class="gm-doc-card">
                    <div class="gm-doc-header">
                        <strong>[{}] {}</strong>
                        <div class="btn-row-compact">
                            {pin_badge}{persist_badge}
                            <button type="button" class="btn-chip" onclick="window.satyaDeleteGmMemory('{}')">Delete</button>
                        </div>
                    </div>
                    <div class="subtitle">Recorded by <strong>{}</strong></div>
                    <pre class="gm-doc-body">{}</pre>
                </div>"#,
                escape_html(m.category.as_str()),
                escape_html(&m.title),
                escape_html(&m.memory_id),
                escape_html(&m.author_display_name),
                escape_html(&m.content)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Creates or updates a persistent GM memory note via `POST /api/sessions/:session_id/memories`.
#[wasm_bindgen]
pub fn satya_submit_gm_memory() {
    spawn_local(async move {
        let (session_id, auth_token) =
            with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            set_feedback(
                "gm-doc-feedback",
                "Join or create an active session first before saving a GM memory note.",
                true,
            );
            return;
        };
        let title = input_value("gm-memory-title").trim().to_string();
        let content = input_value("gm-memory-content").trim().to_string();
        let category = GmMemoryCategory::from_code_str(&select_value("gm-memory-category"));
        let is_pinned = input_checked("gm-memory-pinned");
        let is_persistent = input_checked("gm-memory-persistent");
        let ctag_raw = input_value("gm-doc-campaign-tag").trim().to_string();

        let req = UpsertGmMemoryRequest {
            memory_id: None,
            campaign_tag: if ctag_raw.is_empty() {
                None
            } else {
                Some(ctag_raw)
            },
            category,
            title,
            content,
            is_pinned,
            is_persistent,
        };
        let url = format!("/api/sessions/{sid}/memories");
        match fetch_json::<_, GmMemoryNoteRecord>("POST", &url, Some(&tok), Some(&req)).await {
            Ok(mem) => {
                set_input_value("gm-memory-content", "");
                set_feedback(
                    "gm-doc-feedback",
                    &format!("Saved GM memory note '{}' [{}].", mem.title, mem.category.as_str()),
                    false,
                );
                satya_refresh_gm_memories();
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Deletes a GM memory note via `DELETE /api/sessions/:session_id/memories/:memory_id`.
#[wasm_bindgen]
pub fn satya_delete_gm_memory(memory_id: String) {
    spawn_local(async move {
        let (session_id, auth_token) =
            with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            return;
        };
        let url = format!("/api/sessions/{sid}/memories/{memory_id}");
        if fetch_json::<(), serde_json::Value>("DELETE", &url, Some(&tok), None)
            .await
            .is_ok()
        {
            satya_refresh_gm_memories();
        }
    });
}

/// Refreshes the list of persistent & session GM memory notes from `GET /api/sessions/:session_id/memories`.
#[wasm_bindgen]
pub fn satya_refresh_gm_memories() {
    spawn_local(async move {
        let (session_id, auth_token) =
            with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            return;
        };
        let url = format!("/api/sessions/{sid}/memories");
        if let Ok(resp) =
            fetch_json::<(), SessionGmMemoriesResponse>("GET", &url, Some(&tok), None).await
        {
            with_state_mut(|st| {
                st.gm_memories = resp.memories.clone();
            });
            if let Some(el) = get_element("gm-memories-list") {
                el.set_inner_html(&format_gm_memories_list_html(&resp.memories));
            }
            set_text(
                "gm-memories-count-pill",
                &format!("{} GM Memories", resp.memories.len()),
            );
        }
    });
}

/// Formats ranked [`GmOmnisearchResponse`] hits for `#gm-omnisearch-results`.
pub fn format_gm_omnisearch_hits_html(resp: &GmOmnisearchResponse) -> String {
    if resp.hits.is_empty() {
        return format!(
            r#"<span class="pill pill-muted">No matches found for '{}' in scope {}.</span>"#,
            escape_html(&resp.query),
            escape_html(&resp.scope)
        );
    }
    resp.hits
        .iter()
        .map(|h| {
            let kind_badge = match h.source_kind {
                GmLookupSourceKind::RulesBook => "Rules Book",
                GmLookupSourceKind::GmDocument => "GM Doc",
                GmLookupSourceKind::GmMemory => "GM Memory",
                GmLookupSourceKind::CharacterSheet => "Character Sheet",
                GmLookupSourceKind::CombatTracker => "Combat Tracker",
                GmLookupSourceKind::TacticalMap => "Tactical Map",
            };
            let persist_pill = if h.is_persistent {
                r#" <span class="pill pill-active">Persistent</span>"#
            } else {
                ""
            };
            format!(
                r#"<div class="gm-doc-card">
                    <div class="gm-doc-header">
                        <strong>{}</strong>
                        <span class="pill pill-iggy">{kind_badge} · {}</span>{persist_pill}
                    </div>
                    <pre class="gm-doc-body">{}</pre>
                </div>"#,
                escape_html(&h.title),
                escape_html(&h.subtitle),
                escape_html(&h.snippet)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Searches across Rules Book chapters, Persistent GM Docs, GM Memories, Character Sheets,
/// Combat Tracker, and Tactical Map via `GET /api/sessions/:session_id/lookup` (with local fallback when offline).
#[wasm_bindgen]
pub fn satya_search_gm_omnisearch() {
    let query = input_value("gm-omnisearch-input").trim().to_string();
    let scope = select_value("gm-omnisearch-scope");
    let norm_scope = if scope.trim().is_empty() {
        "ALL".to_string()
    } else {
        scope.trim().to_uppercase()
    };

    let (session_id, auth_token, local_docs, local_memories) = with_state(|st| {
        (
            st.session_id.clone(),
            st.auth_token.clone(),
            st.gm_documents.clone(),
            st.gm_memories.clone(),
        )
    });

    if let (Some(sid), Some(tok)) = (session_id, auth_token) {
        spawn_local(async move {
            let encoded_q: String = js_sys::encode_uri_component(&query).into();
            let encoded_s: String = js_sys::encode_uri_component(&norm_scope).into();
            let url = format!("/api/sessions/{sid}/lookup?q={encoded_q}&scope={encoded_s}");
            if let Ok(resp) =
                fetch_json::<(), GmOmnisearchResponse>("GET", &url, Some(&tok), None).await
                && let Some(el) = get_element("gm-omnisearch-results")
            {
                set_display("gm-omnisearch-results", "flex");
                el.set_inner_html(&format_gm_omnisearch_hits_html(&resp));
            }
        });
        return;
    }

    // Immediate local fallback when searching before joining a session
    let mut hits = Vec::new();
    let q_for_rules = if query.is_empty() {
        "cascade"
    } else {
        query.as_str()
    };
    if matches!(norm_scope.as_str(), "ALL" | "RULES" | "RULES_BOOK") {
        for r in satya_engine::search_rules_book(q_for_rules, 5) {
            hits.push(GmLookupHit {
                source_kind: GmLookupSourceKind::RulesBook,
                item_id: r.chapter_id,
                title: format!("{} — {}", r.chapter_title, r.section_heading),
                subtitle: format!("Rules Book ({})", r.relative_path),
                snippet: r.snippet.chars().take(360).collect(),
                is_persistent: true,
                score: r.score,
            });
        }
    }
    for d in &local_docs {
        hits.push(GmLookupHit {
            source_kind: GmLookupSourceKind::GmDocument,
            item_id: d.doc_id.clone(),
            title: d.title.clone(),
            subtitle: format!("GM Doc · {} chars", d.char_count),
            snippet: d.sanitized_content.chars().take(360).collect(),
            is_persistent: d.is_persistent,
            score: 20,
        });
    }
    for m in &local_memories {
        hits.push(GmLookupHit {
            source_kind: GmLookupSourceKind::GmMemory,
            item_id: m.memory_id.clone(),
            title: m.title.clone(),
            subtitle: format!("GM Memory [{}]", m.category.as_str()),
            snippet: m.content.chars().take(360).collect(),
            is_persistent: m.is_persistent,
            score: 25,
        });
    }
    let resp = GmOmnisearchResponse {
        session_id: "local".to_string(),
        campaign_tag: "local".to_string(),
        query,
        scope: norm_scope,
        hits,
    };
    if let Some(el) = get_element("gm-omnisearch-results") {
        set_display("gm-omnisearch-results", "flex");
        el.set_inner_html(&format_gm_omnisearch_hits_html(&resp));
    }
}

/// Refreshes the list of `GM_ONLY` documents and persistent GM memories visible to the current user.
#[wasm_bindgen]
pub fn satya_refresh_gm_documents() {
    satya_refresh_gm_memories();
    spawn_local(async move {
        let (session_id, auth_token) =
            with_state(|st| (st.session_id.clone(), st.auth_token.clone()));
        let (Some(sid), Some(tok)) = (session_id, auth_token) else {
            return;
        };
        let url = format!("/api/sessions/{sid}/docs");
        if let Ok(resp) =
            fetch_json::<(), SessionGmDocsResponse>("GET", &url, Some(&tok), None).await
        {
            with_state_mut(|st| {
                st.gm_documents = resp.documents.clone();
            });
            if let Some(list_el) = get_element("gm-docs-list") {
                list_el.set_inner_html(&format_gm_documents_list_html(&resp.documents));
            }
            set_text(
                "gm-docs-count-pill",
                &format!("{} GM Docs", resp.documents.len()),
            );
        }
    });
}
