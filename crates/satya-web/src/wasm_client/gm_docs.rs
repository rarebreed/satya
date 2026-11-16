//! `GM_ONLY` Document Vault (`#notebook-pane-docs`) and Google Drive (`drive.file`) OAuth/Picker integration.

use crate::wasm_client::dom::{
    active_session_and_token_or_feedback, escape_html, escape_js_single_quoted_attr, fetch_json,
    input_checked, input_value, read_first_file_bytes, select_value, set_display, set_feedback,
    set_html, set_input_value, set_select_value, set_text,
};
pub use crate::wasm_client::gm_docs_google::*;
pub use crate::wasm_client::gm_memories_and_director::*;
use crate::wasm_client::state::{with_state, with_state_mut};
use satya_schema::{
    GmDocumentRecord, GmDocumentSource, SessionGmDocsResponse, SubmitGmDocumentRequest,
    ToggleGmDocumentPersistenceRequest,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

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
            let js_id = escape_js_single_quoted_attr(&d.doc_id);
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
                            <button type="button" class="btn-chip" onclick="window.satyaToggleGmDocPersistence('{js_id}', {next_persist})">{toggle_btn_label}</button>
                            <button type="button" class="btn-chip" onclick="window.satyaDeleteGmDocument('{js_id}')">Delete</button>
                        </div>
                    </div>
                    <div class="subtitle">Submitted by <strong>{}</strong></div>
                    <pre class="gm-doc-body">{}</pre>
                </div>"#,
                escape_html(&d.title),
                d.char_count,
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
    set_html("gm-docs-list", &format_gm_documents_list_html(&docs));
    set_text("gm-docs-count-pill", &format!("{} GM Docs", docs.len()));
}

/// Reads a local `.txt`, `.md`, or `.csv` file selected via `#gm-doc-file-input`
/// and populates `#gm-doc-title`, `#gm-doc-source`, and `#gm-doc-content`.
#[wasm_bindgen]
pub fn satya_gm_doc_file_selected() {
    spawn_local(async move {
        let Some((file_name, _mime, bytes)) = read_first_file_bytes("gm-doc-file-input").await
        else {
            return;
        };
        let mut content = String::from_utf8_lossy(&bytes).into_owned();
        let mut truncated_note = "";
        if content.len() > 32_768 {
            let mut boundary = 32_768;
            while !content.is_char_boundary(boundary) {
                boundary -= 1;
            }
            content.truncate(boundary);
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
        let Some((sid, tok)) = active_session_and_token_or_feedback(
            "gm-doc-feedback",
            "Join or create an active session first before submitting a GM_ONLY document.",
        ) else {
            return;
        };
        let google_tok = with_state(|st| st.google_access_token.clone());

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
                refresh_gm_omnisearch_if_active();
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Toggles `is_persistent` on an existing `GM_ONLY` document via `POST /api/sessions/:session_id/docs/:doc_id/persist`.
#[wasm_bindgen]
pub fn satya_toggle_gm_doc_persistence(doc_id: String, is_persistent: bool) {
    spawn_local(async move {
        let Some((sid, tok)) = active_session_and_token_or_feedback("", "") else {
            let updated = with_state_mut(|st| {
                let mut found = None;
                if let Some(d) = st.gm_documents.iter_mut().find(|d| d.doc_id == doc_id) {
                    d.is_persistent = is_persistent;
                    found = Some(d.clone());
                }
                found
            });
            if let Some(doc) = updated {
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
            refresh_gm_omnisearch_if_active();
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
                refresh_gm_omnisearch_if_active();
            }
            Err(err) => set_feedback("gm-doc-feedback", &err, true),
        }
    });
}

/// Refreshes the list of `GM_ONLY` documents and persistent GM memories visible to the current user.
#[wasm_bindgen]
pub fn satya_refresh_gm_documents() {
    satya_refresh_gm_memories();
    spawn_local(async move {
        let Some((sid, tok)) = active_session_and_token_or_feedback("", "") else {
            return;
        };
        let url = format!("/api/sessions/{sid}/docs");
        if let Ok(resp) =
            fetch_json::<(), SessionGmDocsResponse>("GET", &url, Some(&tok), None).await
        {
            with_state_mut(|st| {
                st.gm_documents = resp.documents.clone();
            });
            set_html(
                "gm-docs-list",
                &format_gm_documents_list_html(&resp.documents),
            );
            set_text(
                "gm-docs-count-pill",
                &format!("{} GM Docs", resp.documents.len()),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::MessageVisibility;

    #[test]
    fn test_parse_gm_doc_source() {
        assert_eq!(
            parse_gm_doc_source(" google_doc "),
            GmDocumentSource::GoogleDoc
        );
        assert_eq!(
            parse_gm_doc_source("GOOGLE_SHEET"),
            GmDocumentSource::GoogleSheet
        );
        assert_eq!(
            parse_gm_doc_source("DIRECT_TEXT_UPLOAD"),
            GmDocumentSource::DirectTextUpload
        );
        assert_eq!(
            parse_gm_doc_source("unknown"),
            GmDocumentSource::DirectTextUpload
        );
    }

    #[test]
    fn test_format_gm_documents_list_html_empty_and_sanitized_entries() {
        let empty = format_gm_documents_list_html(&[]);
        assert!(empty.contains("No GM_ONLY documents submitted"));

        let docs = vec![GmDocumentRecord {
            doc_id: "doc_it's_1".to_string(),
            session_id: "sector-4".to_string(),
            uploader_user_id: "u-sara".to_string(),
            uploader_display_name: "Sara <Valkyrie>".to_string(),
            title: "Sector 4 Backstory & Medical".to_string(),
            source_kind: GmDocumentSource::GoogleDoc,
            google_file_id: Some("gdoc-1".to_string()),
            visibility: MessageVisibility::GmOnly,
            sanitized_content: "Classified EVA trauma dossier.".to_string(),
            char_count: 30,
            safety_flagged: true,
            is_persistent: true,
            campaign_tag: Some("orbital-arc".to_string()),
            created_at_us: 1_793_644_935_000_000,
        }];
        let html = format_gm_documents_list_html(&docs);
        assert!(html.contains("Sector 4 Backstory &amp; Medical"));
        assert!(html.contains("Google Doc (drive.file) · 30 chars · GM_ONLY"));
        assert!(html.contains("Persistent (orbital-arc)"));
        assert!(html.contains("Sanitized"));
        assert!(html.contains("Sara &lt;Valkyrie&gt;"));
        assert!(html.contains("window.satyaToggleGmDocPersistence('doc_it\\'s_1', false)"));
        assert!(html.contains("window.satyaDeleteGmDocument('doc_it\\'s_1')"));
    }
}
