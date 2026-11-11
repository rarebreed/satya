//! Character Sheet Persistence Vault (`SQLite` + `~/Satya/characters/*.satyachar` + Apache Iggy
//! `character-sheets` topic -> PyLance), `.satyachar` Binary Export/Import (`showSaveFilePicker`),
//! Anti-Cheat Verification Badge, and Multimodal Asset Attachments (Portraits, Tokens, PDFs, 3D `.glb`).
//!
//! Pure record conversion and offline `.satyachar` Avro framing live in
//! [`crate::wasm_client::character_sheet_record`] and are re-exported here.

use crate::wasm_client::character_sheet::{CharacterSheetState, render_character_sheet};
pub use crate::wasm_client::character_sheet_record::*;
use crate::wasm_client::chat_and_ttrpg::append_system_notice;
pub use crate::wasm_client::dom::encode_base64_bytes;
pub(crate) use crate::wasm_client::dom::{
    prompt_save_bytes as prompt_save_satyachar_bytes, read_first_file_bytes,
};
use crate::wasm_client::dom::{escape_html, fetch_json, select_value, slugify_identifier, window};
use crate::wasm_client::state::{with_state, with_state_mut};
use js_sys::Uint8Array;
use satya_schema::{
    BindCharacterSessionRequest, CharacterAssetKind, CharacterListResponse, CharacterSheetRecord,
    CharacterVerificationStatus, ImportCharacterRequest, SaveCharacterRequest,
    SaveCharacterResponse, UploadCharacterAssetRequest,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};

/// Formats the Anti-Cheat Verification Badge HTML (`VERIFIED_OFFICIAL`, `LEGAL_CUSTOM`, `TAMPERED_OR_OVER_BUDGET`).
#[must_use]
pub fn format_verification_badge_html(sheet: &CharacterSheetState) -> String {
    format_verification_badge_with_id_html(sheet, Some("char-sheet-verification-badge"))
}

/// Formats the Anti-Cheat Verification Badge HTML with an optional `id` attribute so multiple
/// placements in the same DOM never collide on `id="char-sheet-verification-badge"`.
#[must_use]
pub fn format_verification_badge_with_id_html(
    sheet: &CharacterSheetState,
    elem_id: Option<&str>,
) -> String {
    let id_attr = match elem_id {
        Some(id) => format!(r#" id="{}""#, escape_html(id)),
        None => String::new(),
    };
    match sheet.verification_status {
        CharacterVerificationStatus::VerifiedOfficial => format!(
            r#"<span{id_attr} class="pill pill-active" title="Server HMAC-SHA256 verified and all Satya Engine budgets/DAG gates passed">Verified Official · Satya Signed</span>"#
        ),
        CharacterVerificationStatus::LegalCustom => format!(
            r#"<span{id_attr} class="pill pill-iggy" title="All Satya Engine Characteristic, Aptitude, SP Ledger, and Skill DAG gates passed">Legal Custom · Engine Audited</span>"#
        ),
        CharacterVerificationStatus::TamperedOrOverBudget => {
            let reason = if sheet.audit_violations.is_empty() {
                "Failed anti-cheat audit".to_string()
            } else {
                escape_html(&sheet.audit_violations.join(" | "))
            };
            format!(
                r#"<span{id_attr} class="pill pill-danger" title="{reason}">Tampered / Over-Budget</span>"#
            )
        }
    }
}

/// Formats the Character Persistence Vault, `.satyachar` Export/Import, and Multimodal Assets panel.
#[must_use]
pub fn format_character_vault_and_assets_html(sheet: &CharacterSheetState) -> String {
    let badge_html = format_verification_badge_html(sheet);
    let saved_opts: String = if sheet.saved_vault_list.is_empty() {
        r#"<option value="">-- Click 'Refresh Vault' or 'Save to Vault' --</option>"#.to_string()
    } else {
        sheet
            .saved_vault_list
            .iter()
            .map(|c| {
                let sel = if c.character_id == sheet.character_id {
                    " selected"
                } else {
                    ""
                };
                let v_tag = match c.verification_status {
                    CharacterVerificationStatus::VerifiedOfficial => "Verified",
                    CharacterVerificationStatus::LegalCustom => "Legal Custom",
                    CharacterVerificationStatus::TamperedOrOverBudget => "Tampered/Over-Budget",
                };
                format!(
                    r#"<option value="{id}"{sel}>{name} ({role} · {v_tag})</option>"#,
                    id = escape_html(&c.character_id),
                    name = escape_html(&c.character_name),
                    role = escape_html(&c.callsign_or_role),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    let violations_html = if sheet.audit_violations.is_empty() {
        String::new()
    } else {
        let items: String = sheet
            .audit_violations
            .iter()
            .map(|v| format!("<li>{}</li>", escape_html(v)))
            .collect::<Vec<_>>()
            .join("");
        format!(
            r#"<div class="feedback-box feedback-error"><strong>Anti-Cheat Audit Findings:</strong><ul>{items}</ul></div>"#
        )
    };

    let local_path_note = sheet
        .local_satya_file_path
        .as_deref()
        .map(|p| {
            format!(
                r#"<span class="pill pill-iggy" title="Saved to local home folder">Local File: {}</span>"#,
                escape_html(p)
            )
        })
        .unwrap_or_default();

    let feedback_html = if sheet.vault_feedback.trim().is_empty() {
        String::new()
    } else {
        format!(
            r#"<div id="char-sheet-vault-feedback" class="stat-note">{}</div>"#,
            escape_html(&sheet.vault_feedback)
        )
    };

    let assets_rows: String = if sheet.assets.is_empty() {
        r#"<tr><td colspan="5" class="subtitle">No multimodal assets attached yet. Upload a character portrait (PNG/WebP), tactical token, backstory PDF, or 3D miniature (`.glb`).</td></tr>"#.to_string()
    } else {
        sheet
            .assets
            .iter()
            .map(|a| {
                let kind_lbl = match a.asset_kind {
                    CharacterAssetKind::PortraitImage => "Portrait Image",
                    CharacterAssetKind::TacticalToken => "Tactical Token",
                    CharacterAssetKind::DossierPdf => "Backstory / Dossier PDF",
                    CharacterAssetKind::Model3dGlb => "3D Miniature (.glb)",
                };
                let kb = (a.size_bytes as f64) / 1024.0;
                let sha_short = a.sha256_hex.chars().take(12).collect::<String>();
                let url = format!(
                    "/api/characters/{}/assets/{}",
                    escape_html(&sheet.character_id),
                    escape_html(&a.asset_id)
                );
                format!(
                    r#"<tr>
                        <td><span class="pill pill-iggy">{kind_lbl}</span></td>
                        <td><strong>{fname}</strong></td>
                        <td class="mono">{mime} · {kb:.1} KB</td>
                        <td class="mono">{sha_short}…</td>
                        <td><a class="btn btn-sm btn-secondary" href="{url}" target="_blank" rel="noopener">Open / Download</a></td>
                    </tr>"#,
                    fname = escape_html(&a.filename),
                    mime = escape_html(&a.mime_type),
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    format!(
        r#"<section id="char-sheet-vault-bar" class="char-sheet-section" aria-label="Character Persistence Vault, .satyachar Export/Import &amp; Multimodal Assets">
            <div class="char-section-header">
                <div>
                    <h3>Character Vault</h3>
                </div>
                <div class="btn-row-compact">
                    {badge_html}
                    {local_path_note}
                </div>
            </div>

            {violations_html}

            <div class="char-tables-two-col">
                <div class="char-table-card">
                    <div class="char-table-title">
                        <span>Save, Export &amp; Import</span>
                        <span class="pill pill-muted">Schema ID 11 · CRC-64-AVRO + HMAC-SHA256</span>
                    </div>
                    <div class="model-controls-row">
                        <button type="button" id="char-sheet-save-vault-btn" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetSaveToVault()">
                            Save to Vault &amp; ~/Satya
                        </button>
                        <button type="button" id="char-sheet-export-btn" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetExportFile()">
                            Export .satyachar File…
                        </button>
                        <button type="button" id="char-sheet-bind-session-btn" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetBindSession()">
                            Bind to Active Session
                        </button>
                    </div>
                    <div class="model-controls-row">
                        <label class="inline-label" for="char-sheet-import-file-input">
                            <span>Import <code>.satyachar</code> Bundle</span>
                            <input
                                id="char-sheet-import-file-input"
                                type="file"
                                accept=".satyachar,.bin,application/octet-stream"
                                onchange="window.satyaCharSheetImportFile()"
                            />
                        </label>
                    </div>
                    <div class="model-controls-row">
                        <label class="inline-label" for="char-sheet-vault-select">
                            <span>Saved Vault Characters</span>
                            <select id="char-sheet-vault-select">{saved_opts}</select>
                        </label>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetLoadSaved('')">Load Selected</button>
                        <button type="button" class="btn btn-sm btn-secondary" onclick="window.satyaCharSheetRefreshVault()">Refresh Vault</button>
                    </div>
                    {feedback_html}
                </div>

                <div class="char-table-card">
                    <div class="char-table-title">
                        <span>Character Assets</span>
                        <span class="pill pill-muted">{asset_count} Attached</span>
                    </div>
                    <div class="model-controls-row">
                        <label class="inline-label" for="char-asset-kind-select">
                            <span>Asset Type</span>
                            <select id="char-asset-kind-select">
                                <option value="PORTRAIT_IMAGE">Portrait Image (PNG / WebP / JPEG)</option>
                                <option value="TACTICAL_TOKEN">Tactical Map Token (PNG / WebP)</option>
                                <option value="DOSSIER_PDF">Backstory / Character Dossier (PDF)</option>
                                <option value="MODEL_3D_GLB">3D Character Miniature (.glb)</option>
                            </select>
                        </label>
                        <input
                            id="char-asset-file-input"
                            type="file"
                            accept="image/*,application/pdf,.glb,.gltf"
                        />
                        <button type="button" class="btn btn-sm btn-accent" onclick="window.satyaCharSheetUploadAsset()">
                            + Upload Asset
                        </button>
                    </div>
                    <div class="char-table-scroll">
                        <table id="char-sheet-assets-table" class="char-data-table">
                            <thead>
                                <tr>
                                    <th>Type</th>
                                    <th>Filename</th>
                                    <th>Format &amp; Size</th>
                                    <th>SHA-256</th>
                                    <th>Action</th>
                                </tr>
                            </thead>
                            <tbody>{assets_rows}</tbody>
                        </table>
                    </div>
                </div>
            </div>
        </section>"#,
        asset_count = sheet.assets.len(),
    )
}

/// Saves the current Character Sheet to the SQLite Vault, writes `~/Satya/characters/<slug>.satyachar`
/// in local mode, and publishes a `CHARACTER_CREATED` / `CHARACTER_UPDATED` event to the Apache Iggy
/// `character-sheets` topic.
#[wasm_bindgen]
pub fn satya_char_sheet_save_to_vault() {
    spawn_local(async move {
        let (tok_opt, session_id, user_id, record): (
            Option<String>,
            Option<String>,
            String,
            CharacterSheetRecord,
        ) = with_state(|st| {
            let uid = st
                .user
                .as_ref()
                .map(|u| u.user_id.clone())
                .unwrap_or_else(|| "local-operator".to_string());
            (
                st.auth_token.clone(),
                st.session_id.clone(),
                uid.clone(),
                st.character_sheet.to_character_sheet_record(&uid),
            )
        });

        let Some(tok) = tok_opt else {
            with_state_mut(|st| {
                st.character_sheet.vault_feedback =
                    "Log in first (or click a Demo User in the top bar) to save to the Character Vault and ~/Satya/characters/."
                        .to_string();
            });
            render_character_sheet();
            return;
        };

        let req = SaveCharacterRequest {
            character: record,
            session_id: session_id.clone(),
            bind_to_session: session_id.is_some(),
        };
        match fetch_json::<_, SaveCharacterResponse>(
            "POST",
            "/api/characters",
            Some(&tok),
            Some(&req),
        )
        .await
        {
            Ok(resp) => {
                let name = resp.character.character_name.clone();
                let path_note = resp.local_file_path.clone();
                with_state_mut(|st| {
                    st.character_sheet
                        .apply_character_sheet_record(&resp.character);
                    st.character_sheet.local_satya_file_path = Some(resp.local_file_path);
                    st.character_sheet.vault_feedback = format!(
                        "Saved '{}' ({:?}) to Vault & {} · Emitted CharacterSheetEvent to Iggy topic `character-sheets`.",
                        name, resp.character.verification_status, path_note
                    );
                    if let Some(existing) = st
                        .character_sheet
                        .saved_vault_list
                        .iter_mut()
                        .find(|c| c.character_id == resp.character.character_id)
                    {
                        *existing = resp.character;
                    } else {
                        st.character_sheet
                            .saved_vault_list
                            .insert(0, resp.character);
                    }
                });
                append_system_notice(&format!(
                    "Saved character '{}' ({}) to Vault & {} (logged to Iggy `character-sheets`)",
                    name, user_id, path_note
                ));
            }
            Err(err) => {
                with_state_mut(|st| {
                    st.character_sheet.vault_feedback = format!("Save failed: {err}");
                });
            }
        }
        render_character_sheet();
    });
}

/// Exports the current character as a signed `.satyachar` Avro binary file, prompting the user where
/// to save via `window.showSaveFilePicker` (defaulting to `<slug>.satyachar`) and logging
/// `CHARACTER_EXPORTED` to the Apache Iggy `character-sheets` topic.
#[wasm_bindgen]
pub fn satya_char_sheet_export_file() {
    spawn_local(async move {
        let (tok_opt, session_id, record): (Option<String>, Option<String>, CharacterSheetRecord) =
            with_state(|st| {
                let uid = st
                    .user
                    .as_ref()
                    .map(|u| u.user_id.clone())
                    .unwrap_or_else(|| "local-operator".to_string());
                (
                    st.auth_token.clone(),
                    st.session_id.clone(),
                    st.character_sheet.to_character_sheet_record(&uid),
                )
            });

        let slug = slugify_identifier(&record.character_name, "operator");
        let filename = format!("{slug}.satyachar");

        let mut exported_bytes: Option<Vec<u8>> = None;
        if let Some(tok) = tok_opt.as_deref() {
            let save_req = SaveCharacterRequest {
                character: record.clone(),
                session_id: session_id.clone(),
                bind_to_session: false,
            };
            if let Ok(saved) = fetch_json::<_, SaveCharacterResponse>(
                "POST",
                "/api/characters",
                Some(tok),
                Some(&save_req),
            )
            .await
            {
                let cid = saved.character.character_id.clone();
                with_state_mut(|st| {
                    st.character_sheet
                        .apply_character_sheet_record(&saved.character);
                    st.character_sheet.local_satya_file_path = Some(saved.local_file_path);
                });
                if let Some(win) = window() {
                    let qs = session_id
                        .as_deref()
                        .map(|s| format!("?session_id={s}"))
                        .unwrap_or_default();
                    let url = format!("/api/characters/{cid}/export{qs}");
                    let init = web_sys::RequestInit::new();
                    init.set_method("GET");
                    if let Ok(headers) = web_sys::Headers::new() {
                        let _ = headers.set("Authorization", &format!("Bearer {tok}"));
                        init.set_headers(&headers);
                    }
                    if let Ok(req) = web_sys::Request::new_with_str_and_init(&url, &init)
                        && let Ok(resp_val) = JsFuture::from(win.fetch_with_request(&req)).await
                        && let Ok(resp) = resp_val.dyn_into::<web_sys::Response>()
                        && resp.ok()
                        && let Ok(ab_prom) = resp.array_buffer()
                        && let Ok(ab_val) = JsFuture::from(ab_prom).await
                    {
                        exported_bytes = Some(Uint8Array::new(&ab_val).to_vec());
                    }
                }
            }
        }

        let bytes = exported_bytes.unwrap_or_else(|| {
            encode_offline_satyachar_bytes(&record, session_id.as_deref()).unwrap_or_default()
        });

        let saved_ok = prompt_save_satyachar_bytes(&filename, &bytes).await;
        with_state_mut(|st| {
            st.character_sheet.vault_feedback = if saved_ok {
                format!(
                    "Exported '{}' ({} bytes) · Logged CHARACTER_EXPORTED to Iggy topic `character-sheets`.",
                    filename,
                    bytes.len()
                )
            } else {
                format!("Prepared '{}' ({} bytes).", filename, bytes.len())
            };
        });
        render_character_sheet();
    });
}

/// Imports a `.satyachar` binary file selected via `#char-sheet-import-file-input`, verifies its
/// server HMAC-SHA256 and `satya-engine` legality, loads it into the Character Sheet, and emits
/// `CHARACTER_IMPORTED` to the Apache Iggy `character-sheets` topic.
#[wasm_bindgen]
pub fn satya_char_sheet_import_file() {
    spawn_local(async move {
        let Some((fname, _mime, bytes)) =
            read_first_file_bytes("char-sheet-import-file-input").await
        else {
            return;
        };
        let (tok_opt, session_id): (Option<String>, Option<String>) =
            with_state(|st| (st.auth_token.clone(), st.session_id.clone()));

        if let Some(tok) = tok_opt {
            let req = ImportCharacterRequest {
                satyachar_base64: encode_base64_bytes(&bytes),
                session_id: session_id.clone(),
                bind_to_session: session_id.is_some(),
            };
            match fetch_json::<_, SaveCharacterResponse>(
                "POST",
                "/api/characters/import",
                Some(&tok),
                Some(&req),
            )
            .await
            {
                Ok(resp) => {
                    let name = resp.character.character_name.clone();
                    let status = resp.character.verification_status;
                    with_state_mut(|st| {
                        st.character_sheet
                            .apply_character_sheet_record(&resp.character);
                        st.character_sheet.local_satya_file_path = Some(resp.local_file_path);
                        st.character_sheet.vault_feedback = format!(
                            "Imported '{}' from {} ({:?}) · Logged CHARACTER_IMPORTED to Iggy topic `character-sheets`.",
                            name, fname, status
                        );
                    });
                    append_system_notice(&format!(
                        "Imported character '{}' from {} (Anti-Cheat: {:?})",
                        name, fname, status
                    ));
                    render_character_sheet();
                    return;
                }
                Err(err) => {
                    with_state_mut(|st| {
                        st.character_sheet.vault_feedback = format!("Import failed: {err}");
                    });
                }
            }
        }

        if let Ok(rec) = decode_offline_satyachar_bytes(&bytes) {
            with_state_mut(|st| {
                st.character_sheet.apply_character_sheet_record(&rec);
                st.character_sheet.vault_feedback = format!(
                    "Imported '{}' locally from {} (Anti-Cheat: {:?}). Log in to sync with Vault & Iggy.",
                    rec.character_name, fname, rec.verification_status
                );
            });
        }
        render_character_sheet();
    });
}

/// Fetches the logged-in player's saved characters from `GET /api/characters` (SQLite + `~/Satya/characters/`).
#[wasm_bindgen]
pub fn satya_char_sheet_refresh_vault() {
    spawn_local(async move {
        let tok_opt: Option<String> = with_state(|st| st.auth_token.clone());
        let Some(tok) = tok_opt else {
            with_state_mut(|st| {
                st.character_sheet.vault_feedback =
                    "Log in to list your saved characters from SQLite and ~/Satya/characters/."
                        .to_string();
            });
            render_character_sheet();
            return;
        };
        if let Ok(resp) =
            fetch_json::<(), CharacterListResponse>("GET", "/api/characters", Some(&tok), None)
                .await
        {
            let count = resp.characters.len();
            let dir_note = resp.local_directory;
            with_state_mut(|st| {
                st.character_sheet.saved_vault_list = resp.characters;
                st.character_sheet.vault_feedback =
                    format!("Loaded {count} saved character(s) from Vault & {dir_note}.");
            });
            render_character_sheet();
        }
    });
}

/// Loads a saved character from `saved_vault_list` into the active Character Sheet.
#[wasm_bindgen]
pub fn satya_char_sheet_load_saved(character_id: String) {
    let target = if character_id.trim().is_empty() {
        select_value("char-sheet-vault-select")
    } else {
        character_id.trim().to_string()
    };
    if target.is_empty() {
        return;
    }
    with_state_mut(|st| {
        if let Some(found) = st
            .character_sheet
            .saved_vault_list
            .iter()
            .find(|c| c.character_id == target)
            .cloned()
        {
            let name = found.character_name.clone();
            st.character_sheet.apply_character_sheet_record(&found);
            st.character_sheet.vault_feedback = format!("Loaded '{}' from Character Vault.", name);
        }
    });
    render_character_sheet();
}

/// Binds the current character sheet to the active session (`POST /api/characters/{id}/bind-session`)
/// and emits `CHARACTER_BOUND_TO_SESSION` to the Apache Iggy `character-sheets` topic.
#[wasm_bindgen]
pub fn satya_char_sheet_bind_session() {
    spawn_local(async move {
        let (tok_opt, session_opt, char_id, char_name): (
            Option<String>,
            Option<String>,
            String,
            String,
        ) = with_state(|st| {
            let cid = if st.character_sheet.character_id.trim().is_empty() {
                format!("char-{}", st.character_sheet.preset_id)
            } else {
                st.character_sheet.character_id.clone()
            };
            (
                st.auth_token.clone(),
                st.session_id.clone(),
                cid,
                st.character_sheet.character_name.clone(),
            )
        });
        let (Some(tok), Some(session_id)) = (tok_opt, session_opt) else {
            with_state_mut(|st| {
                st.character_sheet.vault_feedback =
                    "Log in and join/create a session first to bind this character to the session."
                        .to_string();
            });
            render_character_sheet();
            return;
        };

        let url = format!("/api/characters/{char_id}/bind-session");
        let req = BindCharacterSessionRequest {
            session_id: session_id.clone(),
        };
        match fetch_json::<_, SaveCharacterResponse>("POST", &url, Some(&tok), Some(&req)).await {
            Ok(resp) => {
                with_state_mut(|st| {
                    st.character_sheet
                        .apply_character_sheet_record(&resp.character);
                    st.character_sheet.vault_feedback = format!(
                        "Bound '{}' to active session '{}' · Emitted CHARACTER_BOUND_TO_SESSION to Iggy.",
                        char_name, session_id
                    );
                });
                append_system_notice(&format!(
                    "Operator '{}' bound to session '{}' (Anti-Cheat: {:?})",
                    char_name, session_id, resp.character.verification_status
                ));
            }
            Err(err) => {
                with_state_mut(|st| {
                    st.character_sheet.vault_feedback =
                        format!("Save the character to Vault first before binding ({err}).");
                });
            }
        }
        render_character_sheet();
    });
}

/// Uploads a multimodal asset (`PORTRAIT_IMAGE`, `TACTICAL_TOKEN`, `DOSSIER_PDF`, `MODEL_3D_GLB`)
/// to `POST /api/characters/{id}/assets`, registers `.glb` miniatures in the 3D viewer, and logs
/// `CHARACTER_ASSET_UPLOADED` to Apache Iggy.
#[wasm_bindgen]
pub fn satya_char_sheet_upload_asset() {
    spawn_local(async move {
        let Some((filename, mime_type, bytes)) =
            read_first_file_bytes("char-asset-file-input").await
        else {
            with_state_mut(|st| {
                st.character_sheet.vault_feedback =
                    "Choose an image, PDF, or .glb file first before clicking '+ Upload Asset'."
                        .to_string();
            });
            render_character_sheet();
            return;
        };
        let kind_str = select_value("char-asset-kind-select");
        let asset_kind = CharacterAssetKind::from_code_str(&kind_str);
        if asset_kind == CharacterAssetKind::Model3dGlb {
            let _ = crate::wasm_client::model_3d_wgpu::set_custom_glb_asset(&filename, bytes.clone());
        }

        let (tok_opt, session_id, record): (Option<String>, Option<String>, CharacterSheetRecord) =
            with_state(|st| {
                let uid = st
                    .user
                    .as_ref()
                    .map(|u| u.user_id.clone())
                    .unwrap_or_else(|| "local-operator".to_string());
                (
                    st.auth_token.clone(),
                    st.session_id.clone(),
                    st.character_sheet.to_character_sheet_record(&uid),
                )
            });

        let Some(tok) = tok_opt else {
            with_state_mut(|st| {
                st.character_sheet.vault_feedback =
                    "Log in first to upload multimodal character assets to your dedicated Vault."
                        .to_string();
            });
            render_character_sheet();
            return;
        };

        let save_req = SaveCharacterRequest {
            character: record.clone(),
            session_id: session_id.clone(),
            bind_to_session: false,
        };
        let _ = fetch_json::<_, SaveCharacterResponse>(
            "POST",
            "/api/characters",
            Some(&tok),
            Some(&save_req),
        )
        .await;

        let url = format!("/api/characters/{}/assets", record.character_id);
        let req = UploadCharacterAssetRequest {
            asset_kind,
            filename: filename.clone(),
            mime_type,
            data_base64: encode_base64_bytes(&bytes),
            session_id,
        };
        match fetch_json::<_, SaveCharacterResponse>("POST", &url, Some(&tok), Some(&req)).await {
            Ok(resp) => {
                with_state_mut(|st| {
                    st.character_sheet
                        .apply_character_sheet_record(&resp.character);
                    st.character_sheet.local_satya_file_path = Some(resp.local_file_path);
                    st.character_sheet.vault_feedback = format!(
                        "Uploaded asset '{}' ({} bytes) · Logged CHARACTER_ASSET_UPLOADED to Iggy `character-sheets`.",
                        filename,
                        bytes.len()
                    );
                });
                append_system_notice(&format!(
                    "Attached multimodal asset '{}' ({:?}) to character '{}'",
                    filename, asset_kind, resp.character.character_name
                ));
            }
            Err(err) => {
                with_state_mut(|st| {
                    st.character_sheet.vault_feedback = format!("Asset upload failed: {err}");
                });
            }
        }
        render_character_sheet();
    });
}
